#!/usr/bin/env node
// The UX pass's live crawl (docs/reference/ux-checklist.md, "How to run a pass").
//
// It drives the real screens in Chromium against a running `afm_play` with a
// career started, the way the Mac app would see them:
//   - window.__TAURI__.core.invoke is stubbed so api() takes the Mac path, and
//     window.prompt / confirm / alert / open do what the Mac webview does
//     (nothing, silently) but are counted, so a feature that depends on one shows;
//   - every sidebar screen of the seat is opened, then every screen they link to
//     (the first link of each kind it meets), and each one is screenshotted;
//   - on every screen each visible control is clicked on a freshly drawn page and
//     the page is compared before and after. It reports: controls that changed
//     nothing, controls the mouse can't reach, page errors, calls the game
//     refused (and whether anybody was told), dialogs Esc doesn't close, text
//     that reads "undefined" / "NaN" / "[object Object]", a page that scrolls
//     sideways, and prompt / confirm / alert / window.open calls.
//
// Usage (the server is `afm_play 7700 app/ui <home>`; see README.md):
//   node crawl.js setup --seat=hs                  start a career for a seat (a world must exist: --build builds one)
//   node crawl.js run --out=/tmp/ux --max=25       crawl the career that is open
//   options: --base=http://127.0.0.1:7700  --only=roster,depth  --width=1280 --height=800
"use strict";
const { chromium } = require("playwright");
const fs = require("fs");
const path = require("path");

const A = Object.fromEntries(process.argv.slice(3).map((a) => { const [k, ...v] = a.replace(/^--/, "").split("="); return [k, v.length ? v.join("=") : "1"]; }));
const MODE = process.argv[2];
const BASE = A.base || "http://127.0.0.1:7700";
const OUT = A.out || "./ux-crawl-out";
const MAX = +(A.max || 25);
const W = +(A.width || 1280), H = +(A.height || 800);
const ONLY = A.only ? A.only.split(",") : null;
const SETTLE = +(A.settle || 350);

async function call(cmd, args = {}) {
  const r = await fetch(`${BASE}/api/${cmd}`, { method: "POST", body: JSON.stringify(args) });
  const j = await r.json();
  if (j.err !== undefined) throw new Error(`${cmd}: ${j.err}`);
  return j.ok;
}

// ---------- setup: a career for a seat ----------
const SEATS = {
  hs: { level: "hs", background: "hs_coach" },
  college: { level: "college", background: "college_assistant" },
  nfl: { level: "nfl", background: "nfl_assistant" },
  nflhead: { level: "nfl", background: "nfl_star" },
};
async function setup() {
  const seat = SEATS[A.seat || "hs"];
  if (!seat) throw new Error("--seat is one of " + Object.keys(SEATS).join(", "));
  if (A.build) {
    console.log("building a world (about 5 minutes)...");
    await call("build_world", { seed: +(A.seed || 3), scale: 0.03, history_years: 2, engine_from: 2026, names: "fictional" });
  }
  const jobs = (await call("jobs", { level: seat.level })).jobs;
  const job = (A.team ? jobs.find((j) => j.name.includes(A.team)) : null) || jobs[+(A.job || 0)];
  console.log("job:", job.name);
  const st = await call("start_career", { name: "Alex Tester", age: 42, background: seat.background, job: job.ref, seed: 5 });
  console.log("started:", st.mine && st.mine.name, st.clock || st.today);
}

// ---------- the page ----------
const INIT = () => {
  window.__ux = { prompts: [], confirms: [], alerts: [], opens: [] };
  window.prompt = (...a) => { window.__ux.prompts.push(String(a[0])); return null; };
  window.confirm = (...a) => { window.__ux.confirms.push(String(a[0])); return false; };
  window.alert = (...a) => { window.__ux.alerts.push(String(a[0])); };
  window.open = (...a) => { window.__ux.opens.push(String(a[0])); return null; };
  // The Mac app's api(): invoke("call") that rejects with the game's message as a plain string.
  window.__TAURI__ = {
    core: {
      invoke: async (_name, { cmd, args }) => {
        const r = await fetch("/api/" + cmd, { method: "POST", body: JSON.stringify(args || {}) });
        const j = await r.json();
        if (j.err !== undefined) throw String(j.err);
        return j.ok;
      },
    },
  };
};

const SKIP_CONTROL = /main menu|new career|quit|close the career|delete (save|world)|^load|open a world|reveal|restart|^continue$|^holiday|^save( game)?$|play to|sim to|skip to|advance|^day$|next day|simulate|jump to|let the staff coach/i;
const SEL = [".view button", ".view [data-act]", ".view select", ".view input[type=checkbox]", ".view input[type=radio]", ".view summary", ".view [role=tab]", ".view [role=button]"].join(", ");

function slug(s) { return String(s).replace(/[^a-z0-9]+/gi, "_").replace(/^_|_$/g, "").slice(0, 70); }

async function run() {
  fs.mkdirSync(path.join(OUT, "shots"), { recursive: true });
  const browser = await chromium.launch();
  const ctx = await browser.newContext({ viewport: { width: W, height: H }, deviceScaleFactor: 1 });
  await ctx.addInitScript(INIT);
  const page = await ctx.newPage();
  page.setDefaultTimeout(5000);
  let reqs = 0, cur = null;
  const findings = [];
  const note = (kind, msg, extra = {}) => { findings.push({ kind, route: cur && cur.route, from: cur && cur.from, control: cur && cur.control, msg, ...extra }); if (A.verbose) console.log("   !", kind, "|", cur && cur.control, "|", String(msg).slice(0, 160)); };
  page.on("request", (r) => { if (r.url().includes("/api/")) reqs++; });
  page.on("pageerror", (e) => note("page-error", String(e.message || e).slice(0, 300)));
  page.on("console", (m) => { if (m.type() === "error" && !/favicon|Failed to load resource/.test(m.text())) note("console-error", m.text().slice(0, 300)); });
  page.on("response", async (r) => {
    if (!r.url().includes("/api/")) return;
    try { const j = await r.json(); if (j && j.err !== undefined) note("api-refused", `${r.url().split("/api/")[1]}: ${String(j.err).slice(0, 200)}`, { cmd: r.url().split("/api/")[1] }); } catch (e) { /* not json */ }
  });

  await page.goto(BASE);
  await page.waitForFunction(() => typeof go === "function" && typeof S !== "undefined", null, { timeout: 30000 });
  const enter = async () => {
    await page.evaluate(async () => { S.status = await api("status"); S.status.reveals = 0; });
    await page.evaluate(() => { document.querySelectorAll('[class*="reveal"], .overlay').forEach((e) => e.remove()); });
  };
  await enter();
  const seat = await page.evaluate(() => seatKind());
  console.log("seat:", seat, "team:", await page.evaluate(() => S.status && S.status.mine && S.status.mine.name));

  // Open a route and wait until the page stops changing.
  const open = async (screen, args = {}) => {
    // The big moments (reveal rooms) cover the page until answered; a person answers them, the crawl skips them.
    await page.evaluate(([s, a]) => { if (S.status) S.status.reveals = 0; if (typeof peekHide === "function") peekHide(); document.querySelectorAll('[class*="reveal"], .overlay, .beside, .st-toast, .popmenu, .peek, .notes-sheet, .letter-room').forEach((e) => e.remove()); document.documentElement.classList.remove("reading"); return go(s, a, { push: false, root: true }); }, [screen, args]);
    let last = "", still = 0;
    for (let i = 0; i < 40 && still < 3; i++) {
      await page.waitForTimeout(150);
      const t = await page.evaluate(() => { const v = document.querySelector(".view"); return v ? v.innerHTML.length + ":" + document.querySelectorAll(".view *").length : "none"; });
      if (t === last) still++; else { still = 0; last = t; }
    }
  };
  const snap = (el) => page.evaluate((el) => {
    const hash = (s) => { let h = 0; for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) | 0; return h; };
    const v = document.querySelector(".view"), b = document.querySelector(".beside");
    // The page's text and, for every element, its tag, classes and aria state (not style: animations move that).
    const shape = (root) => { let h = 0; for (const e of root.querySelectorAll("*")) { h = (h * 31 + hash(e.tagName + "." + (typeof e.className === "string" ? e.className : "") + [...e.attributes].filter((a) => /^aria-|^hidden$|^open$/.test(a.name)).map((a) => a.name + a.value).join(""))) | 0; } return h; };
    const o = { route: JSON.stringify(S.route), view: v ? hash(v.innerText) + ":" + v.querySelectorAll("*").length + ":" + shape(v) : "", beside: b ? hash(b.innerText) : "", dialogs: document.querySelectorAll(".overlay").length, toasts: document.querySelectorAll(".st-toast").length, scroll: v ? v.scrollTop : 0, ux: JSON.stringify(window.__ux) };
    // Pop-ups, menus and cards are added to the body itself.
    o.body = hash([...document.body.children].map((c) => c.tagName + "." + c.className + ":" + (c.id || "")).join("|")) + ":" + document.body.children.length;
    const side = document.querySelector(".side"), top = document.querySelector(".topbar");
    o.frame = (side ? hash(side.innerText) : 0) + ":" + (top ? hash(top.innerText) : 0);
    if (el && el.isConnected) o.el = [el.className, el.getAttribute("aria-pressed"), el.getAttribute("aria-selected"), el.getAttribute("aria-expanded"), el.getAttribute("aria-current"), el.checked, el.value, el.disabled].join("|");
    else o.el = "gone";
    o.focus = document.activeElement ? document.activeElement.tagName + "." + document.activeElement.className : "";
    return o;
  }, el);

  // The routes: the seat's sidebar first, then every screen they link to.
  const navs = await page.evaluate(() => NAV.filter((n) => (!n.seats || !seatKind() || n.seats.includes(seatKind())) && (!n.show || n.show()) && screens[n.go]).map((n) => ({ go: n.go, label: String(n.label) })));
  console.log("sidebar screens:", navs.length);
  const seen = new Set(), queue = navs.map((n) => ({ screen: n.go, args: {}, label: n.label, from: "sidebar" }));
  const report = [];
  const keyOf = (r) => r.screen + ":" + JSON.stringify(r.args);

  while (queue.length) {
    const r = queue.shift();
    const k = keyOf(r);
    if (seen.has(k)) continue;
    seen.add(k);
    if (ONLY && !ONLY.includes(r.screen)) continue;
    cur = { route: k, control: "(draw)", from: r.from };
    const before = findings.length;
    await page.evaluate(() => { window.__ux = { prompts: [], confirms: [], alerts: [], opens: [] }; });
    await open(r.screen, r.args);
    const info = await page.evaluate(() => {
      const v = document.querySelector(".view");
      if (!v) return null;
      const txt = v.innerText || "";
      const bad = [...txt.matchAll(/.{0,30}(\bundefined\b|\bNaN\b|\[object Object\]|\bnull\b).{0,20}/g)].slice(0, 4).map((m) => m[0].replace(/\s+/g, " "));
      const tables = [...v.querySelectorAll("table")];
      const links = [];
      const kinds = new Set();
      for (const a of v.querySelectorAll("[data-go]")) {
        const g = a.dataset.go;
        if (kinds.has(g)) continue;
        kinds.add(g);
        const args = {};
        if (a.dataset.id !== undefined && a.dataset.id !== "") args.id = Number(a.dataset.id);
        if (a.dataset.kind) args.kind = a.dataset.kind;
        if (a.dataset.tab) args.tab = a.dataset.tab;
        if (a.dataset.args) { try { Object.assign(args, JSON.parse(a.dataset.args)); } catch (e) { /* plain */ } }
        links.push({ screen: typeof screenFor === "function" ? screenFor(g) : g, args });
      }
      // The tab bar of the screen (the first one): each tab is a screen of its own to crawl.
      const tabEls = [...v.querySelectorAll('[data-act="tab"][data-t]')];
      const key0 = tabEls.length ? tabEls[0].dataset.key : null;
      const tabs = tabEls.filter((b) => b.dataset.key === key0).map((b) => b.dataset.t);
      return {
        tabs,
        chars: txt.length, h: v.scrollHeight, overflowX: v.scrollWidth > v.clientWidth + (/hidden|clip/.test(getComputedStyle(v).overflowX) ? 24 : 2), bodyOverflowX: document.documentElement.scrollWidth > window.innerWidth + 2,
        tables: tables.length, maxCols: Math.max(0, ...tables.map((t) => (t.querySelector("tr") || { children: [] }).children.length)),
        panels: v.querySelectorAll(".panel").length, buttons: v.querySelectorAll("button").length, empties: [...v.querySelectorAll(".empty")].map((e) => e.innerText.slice(0, 80)),
        bad, links,
      };
    });
    if (!info) { note("no-view", "the screen drew no .view"); report.push({ ...r, drew: false }); continue; }
    for (const l of info.links) if (!seen.has(keyOf(l))) queue.push({ ...l, label: l.screen, from: r.screen });
    // Every tab of the screen, once (the tab you are on is this route).
    if (!r.args.tab) for (const t of info.tabs.slice(1)) queue.push({ screen: r.screen, args: { ...r.args, tab: t }, label: r.screen + " / " + t, from: r.screen });
    if (info.bad.length) note("bad-text", info.bad.join(" | "));
    if (info.overflowX || info.bodyOverflowX) note("sideways-scroll", `view scrolls sideways at ${W}px (${info.overflowX ? "view" : "page"})`);
    if (info.chars < 40) note("blank", `almost no text (${info.chars} chars)`);
    const shot = path.join("shots", slug(k) + ".png");
    await page.screenshot({ path: path.join(OUT, shot) });
    const ux0 = await page.evaluate(() => window.__ux);
    if (ux0.prompts.length || ux0.confirms.length || ux0.alerts.length || ux0.opens.length) note("mac-dialog", `on draw: ${JSON.stringify(ux0)}`);

    // ----- the controls -----
    // Listed in the page in one go (cheap), each looked up again after a fresh draw by its place in the list.
    const list = () => page.evaluate((SEL) => {
      const out = [];
      for (const el of document.querySelectorAll(SEL)) {
        const rc = el.getBoundingClientRect();
        if (rc.width < 1 || rc.height < 1) continue;
        const cs = getComputedStyle(el);
        if (cs.visibility === "hidden" || cs.display === "none") continue;
        let unseen = false;
        for (let e = el; e && e !== document.body; e = e.parentElement) { const c = getComputedStyle(e); if (c.opacity === "0" || c.visibility === "hidden" || c.display === "none") { unseen = true; break; } }
        out.push({ i: out.length, tag: el.tagName, text: (el.innerText || el.value || el.getAttribute("aria-label") || el.title || "").replace(/\s+/g, " ").trim().slice(0, 60), act: el.dataset.act || "", disabled: !!el.disabled, nav: !!el.dataset.nav, unseen, el });
      }
      document.querySelectorAll("[data-ux-i]").forEach((e) => e.removeAttribute("data-ux-i"));
      out.forEach((o) => o.el.setAttribute("data-ux-i", String(o.i)));
      return out.map(({ el, ...o }) => o);
    }, SEL);
    const all = await list();
    const count = all.length;
    const idx = [];
    const cap = r.args.tab ? Math.min(MAX, 12) : MAX;
    // Controls that can be tried: not disabled, not off the page's own frame, not hidden slides.
    const triable = all.filter((d) => !d.disabled && !d.nav && !d.unseen && !SKIP_CONTROL.test(d.text) && !SKIP_CONTROL.test(d.act));
    if (A.verbose) console.log(`   controls ${count}: disabled ${all.filter((d) => d.disabled).length}, frame ${all.filter((d) => d.nav).length}, hidden ${all.filter((d) => d.unseen).length}, skipped ${all.filter((d) => SKIP_CONTROL.test(d.text) || SKIP_CONTROL.test(d.act)).length}, to try ${triable.length}`);
    if (triable.length <= cap) idx.push(...triable.map((d) => d.i));
    else for (let n = 0; n < cap; n++) idx.push(triable[Math.floor((n * triable.length) / cap)].i);
    const dead = [], tested = [];
    // Each control is tried on the page as it first drew: the sort, tab and filters it kept are put back.
    const kept = await page.evaluate(() => JSON.stringify(S.keep));
    for (const i of idx) {
      await page.evaluate((j) => { S.keep = JSON.parse(j); }, kept);
      await open(r.screen, r.args);
      const now = await list();
      const d = now[i];
      const want = all[i];
      if (!d || d.text !== want.text || d.act !== want.act) { if (A.verbose) console.log(`   (skipped #${i}: drew differently: "${want.text}" -> "${d && d.text}")`); continue; } // the page drew differently this time
      const label = `${d.tag.toLowerCase()}${d.act ? "[" + d.act + "]" : ""} "${d.text}"`;
      cur = { route: k, control: label, from: r.from };
      const handle = await page.evaluateHandle((i) => document.querySelector(`[data-ux-i="${i}"]`), i);
      const b = await snap(handle);
      const r0 = reqs, f0 = findings.length;
      // Like a person's mouse: scroll to it, make sure it is what sits under its centre, press there.
      const aim = await page.evaluate((i) => {
        const el = document.querySelector(`[data-ux-i="${i}"]`);
        if (!el) return { err: "gone" };
        if (el.tagName === "SELECT") {
          if (el.options.length < 2) return { skip: true };
          const to = el.selectedIndex === 0 ? 1 : 0;
          el.selectedIndex = to;
          el.dispatchEvent(new Event("input", { bubbles: true }));
          el.dispatchEvent(new Event("change", { bubbles: true }));
          return { select: true };
        }
        el.scrollIntoView({ block: "center", inline: "center" });
        const rc = el.getBoundingClientRect();
        const x = rc.x + rc.width / 2, y = rc.y + rc.height / 2;
        const top = document.elementFromPoint(x, y);
        if (!top) return { err: "off screen" };
        if (!(el.contains(top) || top.contains(el))) { const nm = (e) => e.tagName.toLowerCase() + "." + String(e.className && e.className.baseVal !== undefined ? e.className.baseVal : e.className).slice(0, 30); const chain = []; for (let e = top; e && e !== document.body && chain.length < 4; e = e.parentElement) chain.push(nm(e)); return { err: "covered by " + chain.join(" < ") + " :: " + (top.innerText || "").slice(0, 40) }; }
        return { x, y };
      }, i);
      if (aim.skip) continue;
      if (aim.err) { note("unclickable", `${label}: ${aim.err}`); continue; }
      if (!aim.select) await page.mouse.click(aim.x, aim.y);
      await page.waitForTimeout(SETTLE);
      const a = await snap(handle);
      tested.push(label);
      const sameRoute = a.route === b.route;
      const changed = a.route !== b.route || a.view !== b.view || a.beside !== b.beside || a.dialogs !== b.dialogs || a.toasts !== b.toasts || a.el !== b.el || a.ux !== b.ux || a.frame !== b.frame || a.body !== b.body || reqs > r0;
      if (!changed) { dead.push(label); note("no-effect", label); }
      const refused = findings.slice(f0).filter((f) => f.kind === "api-refused");
      if (refused.length && a.toasts === b.toasts && a.dialogs === b.dialogs && a.view === b.view) note("silent-refusal", `${label} -> ${refused.map((x) => x.msg).join("; ")}`);
      if (a.ux !== b.ux) { const u = JSON.parse(a.ux); note("mac-dialog", `${label} called ${JSON.stringify(u)}`); }
      if (a.dialogs > b.dialogs) {
        await page.keyboard.press("Escape");
        await page.waitForTimeout(150);
        const n = await page.evaluate(() => document.querySelectorAll(".overlay").length);
        if (n > b.dialogs) { note("esc-no-close", `${label}: Esc leaves the dialog open`); await page.evaluate(() => document.querySelectorAll(".overlay").forEach((e) => e.remove())); }
      }
    }
    report.push({ ...r, drew: true, chars: info.chars, h: info.h, tables: info.tables, maxCols: info.maxCols, panels: info.panels, buttons: info.buttons, controls: count, tested: tested.length, dead: dead.length, empties: info.empties, findings: findings.length - before });
    console.log(`${k.padEnd(48).slice(0, 48)} ${String(count).padStart(3)} controls, ${tested.length} tried, ${dead.length} dead, ${findings.length - before} findings`);
  }

  // The frame: the sidebar, top bar and the keys.
  cur = { route: "frame", control: "" };
  await open("home", {});
  for (const key of ["Escape", "Meta+k", "Escape", "Meta+[", "Meta+]"]) { await page.keyboard.press(key); await page.waitForTimeout(150); }
  fs.writeFileSync(path.join(OUT, "findings.json"), JSON.stringify({ seat, base: BASE, findings, report }, null, 1));
  await browser.close();

  // A short text report.
  const by = {};
  for (const f of findings) (by[f.kind] ||= []).push(f);
  let md = `# UX crawl, ${seat} seat\n\n${report.length} screens, ${findings.length} findings.\n\n`;
  for (const [kind, list] of Object.entries(by)) {
    md += `## ${kind} (${list.length})\n`;
    const grouped = {};
    for (const f of list) (grouped[f.route] ||= []).push(f);
    for (const [route, fs2] of Object.entries(grouped)) md += `- **${route}**\n` + fs2.slice(0, 12).map((f) => `  - ${f.control && f.control !== "(draw)" ? f.control + ": " : ""}${f.msg === f.control ? "" : f.msg}`).join("\n") + "\n";
    md += "\n";
  }
  md += "## Screens\n\n| screen | controls | tried | dead | tables | widest | panels | chars | height |\n|---|---|---|---|---|---|---|---|---|\n";
  for (const r of report) if (r.drew) md += `| ${r.screen} ${Object.keys(r.args).length ? JSON.stringify(r.args) : ""} | ${r.controls} | ${r.tested} | ${r.dead} | ${r.tables} | ${r.maxCols} | ${r.panels} | ${r.chars} | ${r.h} |\n`;
  fs.writeFileSync(path.join(OUT, "report.md"), md);
  console.log("wrote", path.join(OUT, "report.md"));
}

// ---------- map: which script draws which page (static, no server needed) ----------
function pageMap() {
  const ui = path.resolve(__dirname, "../../../app/ui");
  const html = fs.readFileSync(path.join(ui, "index.html"), "utf8");
  const order = [...html.matchAll(/<script src="([^"]+)"/g)].map((m) => m[1]);
  const reg = {}, nav = {}, crumbs = {}, tabs = [];
  for (const f of order) {
    const t = fs.readFileSync(path.join(ui, f), "utf8");
    t.split("\n").forEach((line, i) => {
      let m;
      const re = /(?:^|[^\w.])screens(?:\.([a-z_0-9]+)|\["([a-z_0-9]+)"\])\s*=(?!=)/g;
      while ((m = re.exec(line))) { const n = m[1] || m[2]; (reg[n] ||= []).push(`${f}:${i + 1}`); }
      const nv = line.match(/go:\s*"([a-z_0-9]+)"(?:,\s*get label\(\)\s*\{[^}]*\}|,\s*label:\s*"([^"]+)")?(?:[^}]*?sec:\s*"([a-z]+)")?(?:[^}]*?seats:\s*\[([^\]]*)\])?/);
      if (nv && /navAdd\(|^\s*\{ go:/.test(line) && (nv[2] || nv[3])) nav[nv[1]] = { ...(nav[nv[1]] || {}), label: nv[2] || (nav[nv[1]] || {}).label, sec: nv[3] || (nav[nv[1]] || {}).sec, seats: nv[4] ? nv[4].replace(/"/g, "").replace(/\s+/g, "") : (nav[nv[1]] || {}).seats };
      const cr = line.match(/^\s*CRUMBS\.([a-z_0-9]+)\s*=\s*"([^"]+)"/);
      if (cr) crumbs[cr[1]] = cr[2];
      const pt = line.match(/PLAYER_TABS\.push\(\{\s*name:\s*"([^"]+)"/);
      if (pt) tabs.push([pt[1], `${f}:${i + 1}`]);
    });
  }
  const sec = { top: "top", team: "Your team", fo: "Front office", scout: "Scouting", data: "Data", world: "World" };
  let md = "# Page map: which script draws which page\n\nGenerated by `node docs/reference/ux-pass/crawl.js map` from `app/ui`. Scripts are listed in load order: a later one wraps an earlier one (it calls the earlier draw and adds to it), so read them all, the last first. `file:line` is where the page is assigned. The route name is what `--only=` and `go()` take.\n\n";
  md += "| Route | Sidebar | Seats | Scripts (load order) |\n|---|---|---|---|\n";
  const names = Object.keys(reg).sort((a, b) => ((nav[a] ? 0 : 1) - (nav[b] ? 0 : 1)) || a.localeCompare(b));
  for (const n of names) {
    const v = nav[n];
    md += `| \`${n}\`${crumbs[n] ? ` (${crumbs[n]})` : ""} | ${v ? `${sec[v.sec] || v.sec} › ${v.label || n}` : "linked from other pages"} | ${v && v.seats ? v.seats : "all"} | ${reg[n].map((x) => "`" + x.replace("screens/", "") + "`").join(" → ")} |\n`;
  }
  md += "\n## Player page tabs\n\nThe `player` page draws `PT.<Tab>` in `core.js` for Overview, Attributes, Reports, Stats, Career, History with you, Tendencies, Recruiting, Contract and Notes. Other scripts add tabs with `PLAYER_TABS.push`:\n\n";
  for (const [n, w] of tabs) md += `- ${n}: \`${w.replace("screens/", "")}\`\n`;
  return md;
}

// ---------- clock: play the world on to a date a page needs (game day, the draft, signing day) ----------
async function clock() {
  const days = +(A.days || 7);
  const start = (await call("status")).today;
  console.log("from", start);
  let t = start, stalls = 0;
  const target = (await call("status")).today;
  for (let n = 0, left = days; left > 0 && n < 200; n++) {
    const r = await call("holiday", { days: Math.min(20, left) });
    left -= r.days || 0;
    const now = r.status ? r.status.today : (await call("status")).today;
    if (now === t) { stalls++; await call("day", {}); } else stalls = 0;
    t = now;
    console.log(" ", now, "left", left);
    if (stalls > 5) break;
  }
  console.log("now", (await call("status")).today, "(was", target + ")");
}

(MODE === "setup" ? setup() : MODE === "run" ? run() : MODE === "clock" ? clock() : MODE === "map" ? Promise.resolve(console.log(pageMap())) : Promise.reject(new Error("node crawl.js setup|run|clock|map"))).catch((e) => { console.error(e); process.exit(1); });
