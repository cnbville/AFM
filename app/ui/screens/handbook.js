/* The Handbook and your first week (update 3, lane 8: H8-H15, boards 38
 * and 39) in Press Box. The Handbook screen: the page list with its search,
 * one page with its example from your save, each level with the one you
 * coach first, where you see it, related pages and what's coming up. The
 * "?" in every screen's header opens that screen's page beside it, with
 * what the screen itself shows marked on it. The Getting started card on
 * Home, the Help question at a new career and the Help setting. The pages'
 * words are in handbook-pages.js; the game side is lane_handbook.rs.
 * Every class is hb8- so the app's global names never restyle it. */
"use strict";

I.book = '<path d="M4 5.5A2.5 2.5 0 0 1 6.5 3H20v15H6.5A2.5 2.5 0 0 0 4 20.5z"/><path d="M4 20.5A2.5 2.5 0 0 0 6.5 23H20v-5"/><path d="M9 7.5h7M9 11h5"/>';
navAdd({ go: "handbook", label: "Handbook", icon: "book", sec: "world" });
CRUMBS.handbook = "Handbook";
const HB8 = { week: null, q: "" };
const HB8_LEVELS = { hs: "High school", college: "College", nfl: "NFL" };
const hb8Rel = (id) => { const p = hbPage(id); return p ? `<button class="hb8-chip" data-go="handbook" data-args='${esc(JSON.stringify({ page: id }))}'>${esc(p.title)}</button>` : ""; };
const hb8Where = (w) => `<a class="hb8-where" data-go="${esc(w[1])}" data-args='${esc(JSON.stringify(w[2] || {}))}'><span>${esc(w[0])}</span><i>›</i></a>`;
// The levels, the one you coach first.
const hb8Order = (x) => (x && x.order) || ["college", "hs", "nfl"];

// ---------- the Handbook screen (H8-H10) ----------
screens.handbook = async (a) => {
  const id = hbPage(a.page) ? a.page : (keep("hb8", { page: "body" }).page || "body");
  keep("hb8", {}).page = id;
  if (S.besideDrawing) return hb8Panel(a, id);
  const p = hbPage(id);
  const x = await api("l8_page", { page: id });
  title("Handbook", p.title);
  const pages = HB.pages;
  const k = pages.indexOf(p);
  const q = HB8.q.trim().toLowerCase();
  const hit = (pg) => !q || [pg.title, pg.lede, ...pg.how.flat(), ...Object.values(pg.lv)].join(" ").toLowerCase().includes(q);
  const nav = HB.groups.map((g) => {
    const list = pages.filter((pg) => pg.group === g && hit(pg));
    if (!list.length) return "";
    return `<div class="hb8-grp"><div class="st-label">${esc(g)}</div>${list.map((pg) => {
      const due = (x.due || []).includes(pg.id);
      const read = !due && (x.read || []).includes(pg.id);
      return `<button class="hb8-nav" data-go="handbook" data-args='${esc(JSON.stringify({ page: pg.id }))}' aria-current="${pg.id === id}"><span>${esc(pg.title)}</span>${due ? '<i class="hb8-due" title="Comes up this month"></i>' : read ? '<i class="hb8-read" title="Read">✓</i>' : ""}</button>`;
    }).join("")}</div>`;
  }).join("") || '<p class="hb8-dim">No page has that word.</p>';
  const helpWord = { full: "Full", hints: "Hints only", off: "Off" }[x.help] || "Full";
  const left = `<aside class="hb8-panel hb8-left">
    <input class="hb8-search" data-hb8q placeholder="Search the Handbook" value="${esc(HB8.q)}" aria-label="Search the Handbook" spellcheck="false">
    <nav class="hb8-navs">${nav}</nav>
    <div class="hb8-legend"><span><i class="hb8-due"></i>comes up this month</span><span><i class="hb8-read">✓</i>read</span></div>
    <div class="hb8-helpline">Help: <b>${esc(helpWord)}</b> · ${linkTo("settings", {}, "change it in Settings")}</div>
  </aside>`;
  const L = x.live || null;
  const how = p.how.map(([t, s], i) => `<div class="hb8-how"><i>${i + 1}</i><div><b>${esc(t)}</b><p>${L && L.how && L.how.k === i ? rich(L.how.s) : esc(s)}</p></div></div>`).join("");
  const order = hb8Order(x);
  const levels = order.map((lv, i) => `<div class="hb8-lv ${i === 0 ? "you" : ""}"><div class="hb8-lvh"><b>${HB8_LEVELS[lv]}</b>${i === 0 ? "<small>you coach here</small>" : ""}</div><p>${esc(p.lv[lv] || "")}</p></div>`).join("");
  const center = `<article class="hb8-panel hb8-page">
    <div class="st-label hb8-kick">${esc(p.group)} · page ${k + 1} of ${pages.length}</div>
    <h2 class="hb8-title">${esc(p.title)}</h2>
    <p class="hb8-lede">${esc(p.lede)}</p>
    <div class="st-label">How it works</div>
    <div class="hb8-hows">${how}</div>
    ${L ? hb8Live(L) : ""}
    <div class="st-label">At each level, the one you coach first</div>
    <div class="hb8-lvs">${levels}</div>
  </article>`;
  const side = L && L.side ? `<section class="hb8-sec"><div class="hb8-sh"><span class="st-label">${esc(L.side.title)}</span><small>${esc(L.side.sub || "")}</small></div>
    ${L.side.rows.map((r, i) => `<div class="hb8-fast"><span class="nm">${rich(r[0])}</span><span class="ps">${esc(r[1])}</span><b class="${i === 0 ? "gold" : ""}">${esc(r[2])}</b><small>${esc(r[3])}</small></div>`).join("")}
    ${L.side.note ? `<p class="hb8-dim">${esc(L.side.note)}</p>` : ""}</section>` : "";
  const soon = (x.soon || []).length ? `<section class="hb8-sec"><div class="st-label">Coming up</div>${x.soon.map((u) => `<div class="hb8-soon"><b>${esc(u.d)}</b><div><span>${esc(u.t)}</span><small>${esc(u.s)}</small></div></div>`).join("")}</section>` : "";
  const right = `<aside class="hb8-panel hb8-right">
    ${side}
    <section class="hb8-sec"><div class="st-label">Where you see it</div>${p.where.map(hb8Where).join("")}</section>
    <section class="hb8-sec"><div class="st-label">Related pages</div><div class="hb8-chips">${p.rel.map(hb8Rel).join("")}</div></section>
    ${soon}
    <div class="hb8-qnote"><i class="hb8-qb">?</i><span>The ? in any screen's header opens that screen's page in the side panel, so you never leave the screen.</span></div>
  </aside>`;
  const head = `<div class="hb8-head">${x.head ? mark(x.head.team, 40) : ""}<div><div class="st-label">${esc(x.head ? x.head.label : "")} · ${pages.length} pages</div><h1>Handbook</h1></div></div>`;
  return `<div class="pad sl hb8">${head}<div class="hb8-grid">${left}${center}${right}</div></div>`;
};
GOTO.handbook = () => [["Settings", "settings"], ["Calendar", "calendar"], ["Inbox", "inbox"]];

// "From your save": the example (H10), with the field at each level when
// the page has one (body and speed).
function hb8Live(L) {
  const strips = L.strips && L.strips.length ? hb8Strips(L) : "";
  return `<section class="hb8-save">
    <div class="hb8-saveh"><span class="st-label">From your save</span><small>${esc(L.when || "")}</small></div>
    ${L.name ? `<div class="hb8-who"><div><b>${rich(L.name)}</b>${L.pos ? `<span>${esc(L.pos)}</span>` : ""}${L.line ? `<span class="ln">${esc(L.line)}</span>` : ""}</div>${L.big ? `<em>${esc(L.big)}</em>` : ""}</div>` : L.big ? `<div class="hb8-who"><div></div><em>${esc(L.big)}</em></div>` : ""}
    ${strips}
    ${(L.lines || []).map((t) => `<p>${rich(t)}</p>`).join("")}
    ${L.note ? `<p class="hb8-note">${rich(L.note)}</p>` : ""}
  </section>`;
}
// Each level's forties at his position, his time as a gold line.
function hb8Strips(L) {
  // Percent across, so the plot stretches with its column and each row
  // stays level with its label; the dots are short ticks that don't stretch.
  const you = Number(L.big);
  const all = L.strips.flatMap((s) => s.dots).concat([you]);
  const X0 = Math.floor((Math.min(...all) - 0.03) * 10) / 10, X1 = Math.ceil((Math.max(...all) + 0.03) * 10) / 10;
  const step = X1 - X0 > 0.8 ? 0.2 : 0.1;
  const rowH = 42, H = L.strips.length * rowH;
  const pc = (t) => (((Math.max(X0, Math.min(X1, t)) - X0) / (X1 - X0)) * 100).toFixed(2);
  const rows = L.strips.map((s, k) => {
    const cy = rowH / 2 + k * rowH;
    const dots = s.dots.map((t, i) => { const y = cy + ((i * 37) % 22) - 11; return `<line x1="${pc(t)}" x2="${pc(t)}" y1="${y - 2.5}" y2="${y + 2.5}"/>`; }).join("");
    return `<g class="${s.you ? "on" : ""}">${dots}<line class="md" x1="${pc(s.med)}" x2="${pc(s.med)}" y1="${cy - 14}" y2="${cy + 14}"/></g>`;
  }).join("");
  const marks = [];
  for (let t = Math.ceil(X0 / step - 1e-9) * step; t <= X1 + 1e-9; t += step) marks.push(t);
  const ticks = marks.map((t) => `<span style="left:${pc(t)}%">${t.toFixed(1)}</span>`).join("");
  const labels = L.strips.map((s) => `<div class="hb8-stl"><b>${esc(s.n)}</b><small>${esc(s.sub)}</small></div>`).join("");
  const pct = L.strips.map((s) => `<div class="hb8-stp ${s.you ? "on" : ""}"><b>${s.pct}%</b><small>ran slower</small></div>`).join("");
  return `<div class="hb8-strips"><div class="hb8-stls">${labels}</div>
    <div class="hb8-stplot"><span class="hb8-youl" style="left:calc(10px + (100% - 20px) * ${pc(you) / 100})">${esc(`${L.short || ""} ${L.big}`)}</span>
      <svg viewBox="0 0 100 ${H}" preserveAspectRatio="none" class="hb8-stsvg" style="height:${H}px" role="img" aria-label="${esc(`${L.short || ""} ${L.big} against each level`)}">${rows}<line class="you" x1="${pc(you)}" x2="${pc(you)}" y1="0" y2="${H}"/></svg>
      <div class="hb8-ticks">${ticks}</div></div>
    <div class="hb8-stps">${pct}</div></div><div class="hb8-axis">← faster · white tick: median</div>`;
}

// ---------- the side panel from a screen's "?" (H11) ----------
async function hb8Panel(a, id) {
  const p = hbPage(id);
  const from = a.from || "";
  const sc = HB.screens[from] || {};
  const x = await api("l8_page", { page: id, panel: true });
  S.title = `Handbook · ${p.title}`;
  const on = (sc.on || p.how.slice(0, 3)).map((o, i) => `<div class="hb8-on"><i>${i + 1}</i><div><b>${esc(o[0])}</b><p>${esc(o[1])}</p></div></div>`).join("");
  const L = x.live;
  const live = L && (L.lines || []).length ? `<div class="hb8-save sm"><span class="st-label">From your save</span><p>${rich(L.lines[0])}</p></div>` : "";
  const order = hb8Order(x);
  const lv = order.map((l, i) => `<div class="hb8-plv ${i === 0 ? "you" : ""}"><b>${HB8_LEVELS[l]}</b><p>${esc(p.lv[l] || "")}</p></div>`).join("");
  return `<div class="pad sl hb8 hb8-pn">
    <div class="hb8-pk"><span class="st-label">Handbook · ${esc(p.group)}</span><button class="hb8-x" data-act="besideclose" aria-label="Close">×</button></div>
    <h2 class="hb8-title sm">${esc(p.title)}</h2>
    <p class="hb8-lede">${esc(p.lede)}</p>
    <div class="st-label">${sc.on ? "On this screen" : "How it works"}</div>
    <div class="hb8-ons">${on}</div>
    ${live}
    <div class="st-label">At each level</div>
    <div class="hb8-plvs">${lv}</div>
    <div class="hb8-prel"><span class="st-label">Related</span>${p.rel.map(hb8Rel).join("")}</div>
    <p class="hb8-dim">Close the panel and you're back where you were on the screen.</p>
    <div class="hb8-pbtns"><button class="st-btn primary" data-act="hb8full">Open the full page</button><button class="st-btn glass" data-act="besideclose">Close</button></div>
  </div>`;
}

// The "?" goes beside the first heading of every screen; a screen with no
// heading gets it at the end of the path.
function hb8Q() {
  const r = S.route;
  if (r.screen === "handbook" || document.querySelector(".view .hb8-q")) return;
  const btn = `<button class="hb8-q" data-act="hb8q" title="How this screen works" aria-label="How this screen works">?</button>`;
  const h = document.querySelector(".view h1, .view .h1");
  if (h) h.insertAdjacentHTML("beforeend", btn);
  else document.querySelector(".crumb")?.insertAdjacentHTML("beforeend", btn);
}
// The panel's numbers on the screen itself, found by their words.
function hb8Pins() {
  document.querySelectorAll(".hb8-pin").forEach((x) => x.remove());
  document.querySelectorAll(".hb8-pinned").forEach((x) => x.classList.remove("hb8-pinned"));
  const b = S.beside;
  if (!b || b.screen !== "handbook" || b.args.from !== S.route.screen) return;
  const sc = HB.screens[b.args.from];
  if (!sc || !sc.on) return;
  sc.on.forEach((o, i) => {
    if (!o[2]) return;
    const want = o[2].toLowerCase();
    const el = [...document.querySelectorAll(".view th, .view [role=tab], .view .st-label, .view h2, .view h3")].find((e) => e.textContent.trim().toLowerCase().startsWith(want));
    if (!el) return;
    el.classList.add("hb8-pinned");
    el.insertAdjacentHTML("afterbegin", `<i class="hb8-pin">${i + 1}</i>`);
  });
}
// A look-over task ticks when its screen opens (H13).
function hb8Seen() {
  if (!HB8.week || !S.status || !S.status.mine) return;
  api("l8_seen", { screen: S.route.screen }).then((r) => { if (r.ticked) HB8.week = null; }).catch(() => {});
}
async function hb8Week() {
  if (HB8.week !== null) return;
  try { HB8.week = (await api("l8_help")).week; } catch (e) { HB8.week = false; }
}

{
  const after0 = afterRender;
  afterRender = function () {
    after0();
    if (!S.status || !S.status.mine) return;
    hb8Q();
    hb8Pins();
    if (HB8.week === null) hb8Week().then(hb8Seen); else hb8Seen();
  };
  const beside0 = drawBeside;
  drawBeside = async function (fresh) {
    await beside0(fresh);
    const el = document.querySelector(".beside");
    if (el) el.classList.toggle("hb8-side", !!S.beside && S.beside.screen === "handbook");
    hb8Pins();
  };
  const close0 = closeBeside;
  closeBeside = function () { close0(); hb8Pins(); };
  // Another career (taken or loaded) has its own week.
  const enter0 = enterCareer;
  enterCareer = function () { HB8.week = null; enter0(); };
}

// ---------- the Getting started card on Home (H13) ----------
function hb8Card(c) {
  const bar = c.tasks.map((t) => `<i class="${t.done ? "on" : ""}"></i>`).join("");
  const rows = c.tasks.map((t, i) => {
    const mark = t.done ? '<span class="hb8-tk done">✓</span>' : `<span class="hb8-tk ${t.today ? "now" : ""}">${i + 1}</span>`;
    const act = t.done
      ? `<span class="hb8-tdone">Done · <a class="link" data-act="hb8task" data-key="${esc(t.key)}" data-done="0">undo</a></span>`
      : `<span class="hb8-tbtn"><button class="st-btn ${t.today ? "primary" : "glass"} small" data-go="${esc(t.go)}">${esc(t.button)}</button>${t.act ? `<a class="link hb8-mark" data-act="hb8task" data-key="${esc(t.key)}" data-done="1">mark done</a>` : ""}</span>`;
    return `<div class="hb8-task ${t.done ? "done" : ""} ${t.today ? "today" : ""}">${mark}<span class="hb8-tday">${esc(t.day.replace(/ \d{4}$/, ""))}</span><div class="hb8-tt"><b>${esc(t.title)}${t.today && !t.done ? '<small class="hb8-today">Today</small>' : ""}</b><span>${esc(t.text)}</span></div>${act}</div>`;
  }).join("");
  return `<section class="sl hb8 hb8-card" aria-label="Getting started">
    <div class="hb8-ch"><div><div class="st-label">Getting started · your first week</div><h2>${esc(c.title)}</h2><p>${esc(c.from)}</p></div>
      <div class="hb8-cnt"><b><em>${c.done}</em> of ${c.of} done</b><div class="hb8-bar">${bar}</div></div></div>
    <div class="hb8-tasks">${rows}</div>
    <div class="hb8-cf"><span>Do them in any order. Each one ticks itself off when you've done it, and the card goes away when all ${c.of} are done.</span><span>Help: <b>${esc(c.help)}</b> · ${linkTo("settings", {}, "Settings")}</span></div>
  </section>`;
}
{
  const home0 = screens.home;
  screens.home = async (a) => {
    const html = await home0(a);
    if (typeof html !== "string") return html;
    let c;
    try { c = await api("l8_started"); } catch (e) { return html; }
    HB8.week = !!c.show;
    if (!c.show) return html;
    // On top of Home for the first week, the hero below it.
    return `<div class="pad hb8-cardpad">${hb8Card(c)}</div>${html}`;
  };
}

// ---------- the Help setting (H15) ----------
const HB8_HELP = [["full", "Full", "Your guided first week, first-time notes and your staff's warnings"], ["hints", "Hints only", "First-time notes and warnings, no guided week"], ["off", "Off", "Nothing from your staff about how the game works"]];
{
  const settings0 = screens.settings;
  screens.settings = async (a) => {
    const html = await settings0(a);
    if (typeof html !== "string" || !S.status || !S.status.mine) return html;
    let h;
    try { h = await api("l8_help"); } catch (e) { return html; }
    const seg = `<div class="seg">${HB8_HELP.map(([v, t]) => `<button data-act="hb8help" data-v="${v}" aria-pressed="${h.level === v}">${t}</button>`).join("")}</div>`;
    const cur = HB8_HELP.find((x) => x[0] === h.level) || HB8_HELP[0];
    const panel = `<div class="panel"><h2>Help</h2><div class="setrow"><div><b>How much the game guides you</b><small>${esc(cur[2])}. Turned to Full in your first week at a job, the guided week starts that day.</small></div>${seg}</div><div class="setrow"><div><b>The Handbook</b><small>One page per system, with examples from your save</small></div>${linkTo("handbook", {}, "Open the Handbook", "st-btn small")}</div></div>`;
    const at = html.indexOf('<div class="panel"><h2>Logos</h2>');
    return at < 0 ? html : html.slice(0, at) + panel + html.slice(at);
  };
}

// The question at a new career (H15): Full on your first career on this
// machine, Hints only after that; it goes with the start.
const hb8Default = () => (prefs.careers ? "hints" : "full");
{
  const page0 = setupPage;
  setupPage = function (st, o) {
    if (st.step !== 6) return page0(st, o);
    const cur = st.help || hb8Default();
    const seg = `<span class="hb8-setup"><span>Help</span>${HB8_HELP.map(([v, t, s]) => `<button class="pg-btn ${cur === v ? "on" : ""}" data-hb8setup="${v}" title="${esc(s)}" aria-pressed="${cur === v}">${t}</button>`).join("")}</span>`;
    return page0(st, { ...o, extra: (o.extra || "") + seg });
  };
  const args0 = startArgs;
  startArgs = function (st) {
    prefs.careers = (prefs.careers || 0) + 1;
    savePrefs();
    HB8.week = null;
    return { ...args0(st), help: st.help || hb8Default() };
  };
}

// ---------- clicks ----------
Object.assign(CORE_MORE, {
  hb8q: () => {
    const scr = S.route.screen;
    const sc = HB.screens[scr];
    openBeside("handbook", { page: sc ? sc.page : "calendar", from: scr });
  },
  hb8full: () => { const b = S.beside; closeBeside(); go("handbook", { page: b && b.args.page }); },
  hb8task: async (el) => {
    try { await api("l8_task", { key: el.dataset.key, done: el.dataset.done === "1" }); refresh(el); } catch (e) { toast(e.message); }
  },
  hb8help: async (el) => {
    try { const h = await api("l8_help_set", { level: el.dataset.v }); HB8.week = h.week; toast(`Help: ${HB8_HELP.find((x) => x[0] === h.level)[1]}.`); refresh(el); } catch (e) { toast(e.message); }
  },
});
document.addEventListener("click", (ev) => {
  const b = ev.target.closest && ev.target.closest("[data-hb8setup]");
  if (!b) return;
  setupState().help = b.dataset.hb8setup;
  render();
});
let hb8Timer = 0;
document.addEventListener("input", (ev) => {
  if (!ev.target.matches || !ev.target.matches("[data-hb8q]")) return;
  HB8.q = ev.target.value;
  clearTimeout(hb8Timer);
  hb8Timer = setTimeout(async () => {
    const el = document.querySelector("[data-hb8q]");
    const pos = el ? el.selectionStart : 0;
    await render();
    const n = document.querySelector("[data-hb8q]");
    if (n) { n.focus(); n.setSelectionRange(pos, pos); }
  }, 180);
});
