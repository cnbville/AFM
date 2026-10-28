// The pages before a career (Cain, 2026-10-21, first-play notes A and B):
// the opening screen, Load game and the six steps of a new career (World,
// You, Background, Ratings, Style, Job), in one FM-style look of their own
// (pregame.css). Showtime starts once you take the job. Buttons here carry
// `data-pg`; app.js's helpers (api, go, render, dialogs) do the rest.
"use strict";

// What these pages fetched, kept while you move between them.
// `loaded`: the library world open in the game with no career yet.
const PG = { loaded: null, lib: null, hello: null, online: null, offline: "", onlineAsked: false, opts: null, jobs: {}, towns: {}, similar: {}, jobPoll: null };

// ---------- small helpers ----------
function pgMark(team, big) {
  const lg = typeof logoFor === "function" ? logoFor(team) : null;
  if (lg) return `<span class="pg-mk lgo-pg ${big ? "big" : ""}">${logoImg(lg)}</span>`;
  const hex = (x) => typeof x === "string" && /^#[0-9a-f]{3,8}$/i.test(x);
  const c = team && team.colors && hex(team.colors[0]) && hex(team.colors[1]) ? team.colors : ["#2a2f37", "#eceef1"];
  return `<span class="pg-mk ${big ? "big" : ""}" style="background:${esc(c[0])};color:${esc(c[1])}">${esc(initials(team && team.name) || "?")}</span>`;
}
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const size = (mb) => (mb >= 1000 ? (mb / 1000).toFixed(2) + " GB" : Math.max(1, Math.round(mb)) + " MB");
const yrs = (n) => `${n} year${n === 1 ? "" : "s"}`;
// When a file was saved, in your own time: "Today, 11:42", "19 Oct, 16:05".
function savedText(secs) {
  if (!secs) return "";
  const d = new Date(secs * 1000), now = new Date();
  const hm = `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
  const day = (x) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
  const diff = Math.round((day(now) - day(d)) / 864e5);
  if (diff === 0) return `Today, ${hm}`;
  if (diff === 1) return `Yesterday, ${hm}`;
  return `${d.getDate()} ${MONTHS[d.getMonth()]}${d.getFullYear() !== now.getFullYear() ? " " + d.getFullYear() : ""}, ${hm}`;
}
// "2026-10-21 14:03 UTC" -> "21 Oct 2026".
function builtText(written) {
  const m = String(written || "").match(/^(\d{4})-(\d{2})-(\d{2})/);
  return m ? `${Number(m[3])} ${MONTHS[Number(m[2]) - 1]} ${m[1]}` : "";
}
const money0 = (v) => (v == null ? "" : v >= 1e6 ? "$" + (v / 1e6).toFixed(1) + "M" : v >= 1e3 ? "$" + Math.round(v / 1e3) + "k" : "$" + Math.round(v));
const thousands = (v) => (v == null ? "" : Number(v).toLocaleString("en-US"));
const isMac = () => !!(window.__TAURI__ && window.__TAURI__.core);
function worldTitle(x) {
  const m = x && x.meta;
  return m ? `World ${m.start_year}` : (x && x.name) || "A world";
}
function bar(share) {
  return `<span class="pg-bar"><i style="width:${Math.round(Math.max(0, Math.min(1, share)) * 100)}%"></i></span>`;
}

// The top bar and the stadium band every page but the opening shares.
function pgPage({ steps = "", right = "", kicker, title, lede = "", body, foot }) {
  return `<div class="pg">
    <header class="pg-top"><button class="pg-logo" data-pg="home" title="Back to the opening screen">AFM</button>
      ${steps || '<span style="flex:1"></span>'}<span class="pg-date">${esc(right)}</span></header>
    <div class="pg-main" data-keep="main">
      <section class="pg-band"><div class="pg-kicker">${esc(kicker)}</div><h1>${esc(title)}</h1>${lede ? `<p class="pg-lede">${esc(lede)}</p>` : ""}</section>
      ${body}
    </div>
    <footer class="pg-foot">${foot}</footer>
  </div>`;
}

// The version a person sees: on the Mac, the app's own number (0.2.<build>,
// the one updates compare against), since the game library's stays 0.2.0.
async function appHello() {
  const h = await api("hello");
  try { if (isMac() && window.__TAURI__.app) h.version = await window.__TAURI__.app.getVersion(); } catch (e) { /* keep the game's number */ }
  return h;
}

// ---------- the opening screen ----------
screens.start = async () => {
  const [lib, hello] = await Promise.all([api("library"), PG.hello ? Promise.resolve(PG.hello) : appHello()]);
  PG.lib = lib;
  PG.hello = hello;
  // A career is open with or without a team (out of work is a career too).
  const open = S.status && S.status.you;
  const last = lib.saves[0];
  // The card the menu shows: the career you have open, else your newest save.
  let card = null, cont = "", savedLine = "";
  if (open) {
    card = await api("card").catch(() => null);
    savedLine = S.savedOn ? (S.savedOn === S.status.today ? "Saved today" : `Last saved ${S.savedOn}`) : "Not saved yet";
  } else if (last) {
    card = last.card;
    savedLine = "Saved " + savedText(last.modified).replace(/^(Today|Yesterday)/, (x) => x.toLowerCase());
  }
  const seat = card && card.seat;
  if (open || last) {
    const free = open && !S.status.mine;
    const name = seat ? seat.team.name : free ? S.status.you : open ? S.status.mine.name : (last.meta && last.meta.career) || last.name;
    const role = seat ? `${seat.role}` : free ? "Out of coaching" : "Head coach";
    const sub = seat ? [seat.line, seat.record, seat.next ? "next: " + seat.next : ""].filter(Boolean).join(" · ") : free ? "Looking for a job" : (last && last.meta ? last.meta.career || "" : "");
    const team = seat ? seat.team : free ? { name, colors: null } : open ? S.status.mine : { name, colors: null };
    cont = `<button class="pg-mi on" data-pg="continue">Continue</button>
      <button class="pg-save" data-pg="continue">
        <div class="t">${pgMark(team)}<div><b>${esc(name)} · ${esc(role)}</b><small>${esc(sub)}</small></div></div>
        <div class="w"><span>${esc(seat ? seat.date : open ? S.status.today : last.meta ? last.meta.world_date : "")}</span><span>${esc(savedLine)}</span></div>
      </button>`;
  }
  // Around football: the headlines of that card's world, else the library's.
  const src = card || (lib.worlds[0] && lib.worlds[0].card);
  const news = src && src.headlines && src.headlines.length
    ? `<div class="pg-news"><div class="h">Around football · ${esc(src.month || "")}</div>${src.headlines.map((h) => `<div class="i">${esc(h)}</div>`).join("")}</div>`
    : "";
  const wsrc = src && src.world;
  const info = wsrc
    ? `World ${wsrc.first + wsrc.history_years} · seed ${wsrc.seed} · ${yrs(wsrc.history_years)} of history`
    : lib.worlds.length ? `${lib.worlds.length} world${lib.worlds.length === 1 ? "" : "s"} in your library` : "No world in your library yet";
  return `<div class="pg-stage">
    <div class="pg-wordmark"><b>AFM</b><span>American Football Manager</span></div>
    <nav class="pg-menu" aria-label="Main menu">
      ${cont}
      <button class="pg-mi ${cont ? "" : "on"}" data-pg="new">New career</button>
      <button class="pg-mi" data-pg="load">Load game</button>
      <div style="height:10px"></div>
      <button class="pg-mi quiet" data-pg="settings">Settings</button>
      ${isMac() && window.__TAURI__.process ? '<button class="pg-mi quiet" data-pg="quit">Quit</button>' : ""}
    </nav>
    <div class="pg-bottom">
      <div class="pg-info"><span>${esc(info)}</span><span>Version ${esc(hello.version)}${isMac() ? " · Mac" : ""}</span></div>
      ${news}
    </div>
  </div>`;
};

// ---------- Load game ----------
screens.load = async () => {
  const lib = await api("library");
  PG.lib = lib;
  const st = keep("load", { sel: null });
  const saves = lib.saves;
  if (!saves.find((x) => x.path === st.sel)) st.sel = saves[0] ? saves[0].path : null;
  const cur = saves.find((x) => x.path === st.sel);
  const seatOf = (x) => (x.card && x.card.seat) || null;
  const worldOf = (x) => {
    const w = x.card && x.card.world;
    return w ? `World ${w.first + w.history_years} · ${w.names} names` : x.meta ? `World ${x.meta.start_year} · seed ${x.meta.seed}` : "";
  };
  const rows = saves.map((x) => {
    const s = seatOf(x);
    return `<tr data-pg="pick-save" data-path="${esc(x.path)}" class="${x.path === st.sel ? "sel" : ""}">
      <td style="font-weight:600">${esc(x.name)}</td>
      <td>${s ? pgMark(s.team) + esc(s.team.name) : esc(x.meta && x.meta.career ? x.meta.career.split(", ").slice(1).join(", ") : "")}</td>
      <td>${esc(s ? s.date : x.meta ? x.meta.world_date : "")}</td>
      <td class="n">${esc(s ? s.record : "")}</td>
      <td class="pg-dim">${esc(worldOf(x))}</td>
      <td class="n pg-dim">${esc(savedText(x.modified))}</td></tr>`;
  }).join("");
  const s = cur && seatOf(cur);
  const side = cur
    ? `<aside class="pg-aside"><div class="pg-panel">
        <div class="pg-filehead">${pgMark(s ? s.team : { name: cur.name }, true)}<div><b>${esc(s ? s.team.name : cur.name)}</b><small>${esc(s ? `${s.role} · ${s.line}` : cur.meta ? cur.meta.career || "" : "")}</small></div></div>
        <div class="pg-kv">
          <span>Coach</span><span>${esc(s ? s.coach : cur.meta && cur.meta.career ? cur.meta.career.split(", ")[0] : "")}</span>
          <span>In the game</span><span>${esc(s ? s.date : cur.meta ? cur.meta.world_date : "")}</span>
          ${s ? `<span>This season</span><span>${esc(s.record)}</span><span>Next</span><span>${esc(s.next || "-")}</span>` : ""}
          <span>World</span><span>${esc(worldOf(cur))}</span>
          <span>Saved</span><span>${esc(savedText(cur.modified))}</span>
          <span>Size</span><span>${esc(size(cur.mb))}</span>
        </div>
        <div class="pg-acts-row"><button class="pg-btn s" data-pg="rename-save">Rename</button><button class="pg-btn s" data-pg="delete-save">Delete…</button></div>
      </div></aside>`
    : "";
  const body = `<div class="pg-body"><div class="pg-col"><div class="pg-panel">
      <div class="pg-ph"><span>Saves <small>· ${saves.length}</small></span><span class="pg-acts"><button class="pg-btn s" data-pg="open-save-file">Open a save file…</button>${isMac() ? '<button class="pg-btn s" data-pg="reveal-saves">Show in Finder</button>' : ""}</span></div>
      ${saves.length ? `<div class="pg-tbl" data-keep="saves"><table><thead><tr><th>Career</th><th>Team</th><th>In the game</th><th class="n">Season</th><th>World</th><th class="n">Saved</th></tr></thead><tbody>${rows}</tbody></table></div>`
        : `<div class="pg-empty">No saves yet. During a career, Save in the sidebar keeps it here (${esc(lib.home)}/saves).</div>`}
    </div></div>${side}</div>`;
  return pgPage({
    right: "Your saves",
    kicker: "Load game",
    title: "Pick up where you left off",
    body,
    foot: `<button class="pg-btn" data-pg="home">Back</button><span class="pg-sum">${cur ? esc(`${cur.name} · ${s ? s.date : cur.meta ? cur.meta.world_date : ""}`) : ""}</span><button class="pg-btn p" data-pg="load-save" ${cur ? "" : "disabled"}>Load</button>`,
  });
};

async function openSave(path) {
  if ($("#busy") || !(await leaveCareer())) return;
  showBusy("Loading your save…");
  try {
    S.status = await api("load", { path });
    PG.loaded = null;
    hideBusy();
    S.savedOn = S.status.today;
    S.lastSaveName = path.split(/[\\/]/).pop().replace(/\.afmsave$/i, "");
    S.keep = {};
    if (S.status.you) enterCareer();
    else {
      // A save with no career yet: a new career starts from it as it is.
      S.back = []; S.fwd = [];
      PG.loaded = path;
      S.keep.setup = newSetup();
      go("setup", {}, false);
    }
  } catch (e) { hideBusy(); toast(e.message); }
}

// ---------- a new career ----------
const STEPS = ["World", "You", "Background", "Ratings", "Style", "Job"];
function newSetup() {
  const y = new Date().getFullYear();
  // A world already open with no career goes on from it without reopening.
  const open = S.status && !S.status.you && PG.loaded ? PG.loaded : null;
  return {
    step: 1, reached: 1, world: open, loaded: open,
    first: "", last: "", born: `${y - 42}-06-15`, eth: "white",
    hometown: "", state: "TX", high_school: "", alma_mater: "", college_years: "", degree: "",
    status: "Married", spouse: "", roots: "", children: [],
    height: "6 ft 1", build: "Athletic", hair: "Short, dark", facial_hair: "Clean-shaven", photo: null,
    bg: "college_assistant", ratings: null, ratingsFor: null,
    style: { risk: 2, freedom: 2, physical: 2, ball_hunting: 2, mould: 2 },
    level: "college", job: null, q: "",
    // Lane 10: your last stops, and the seat you start in (head coach, an
    // assistant's role, or out of work at a level).
    stops: [], mode: "head", role: null, freeLevel: "college",
    f: { college: { div: "FBS", conf: null }, nfl: { conf: null, div: null }, hs: { state: null, cls: null }, other: { league: null } },
    sort: { k: null, desc: true },
  };
}
const setupState = () => (S.keep.setup ??= newSetup());

function stepsBar(st) {
  return `<nav class="pg-steps" aria-label="New career steps">${STEPS.map((n, i) => {
    const k = i + 1;
    const cls = k === st.step ? "on" : k < st.step || k <= st.reached ? "done" : "";
    return `<button class="pg-step ${cls}" data-pg="step" data-k="${k}" ${k <= st.reached ? "" : "disabled"}><i>${k}</i>${n}</button>`;
  }).join("")}</nav>`;
}
function setupPage(st, { title, lede, body, sum, next, nextOk = true, extra = "" }) {
  const back = st.step === 1 ? `<button class="pg-btn" data-pg="home">Back</button>` : `<button class="pg-btn" data-pg="step" data-k="${st.step - 1}">Back</button>`;
  return pgPage({
    steps: stepsBar(st),
    right: (PG.opts && PG.opts.today) || "",
    kicker: `New career · Step ${st.step} of 6`,
    title, lede, body,
    foot: `${back}${extra}<span class="pg-sum">${sum}</span><button class="pg-btn p" data-pg="${st.step === 6 ? "take" : "next"}" ${nextOk ? "" : "disabled"}>${esc(next)}</button>`,
  });
}

screens.setup = async () => {
  const st = setupState();
  // Every step after the first needs its world open, with no career yet.
  const ready = st.loaded && S.status && !S.status.you;
  if (st.step > 1 && !ready) st.step = 1;
  if (st.step > 1 && !PG.opts) PG.opts = await api("setup");
  return [null, stepWorld, stepYou, stepBackground, stepRatings, stepStyle, stepJob][st.step](st);
};

// Step 1: the world.
async function stepWorld(st) {
  const lib = await api("library");
  PG.lib = lib;
  const job = await api("job").catch(() => null);
  if (job && !job.finished) watchBuild();
  // The worlds page, once a session; again a minute after it couldn't be reached.
  if (!PG.onlineAsked || (!PG.online && PG.offline && Date.now() - (PG.offlineAt || 0) > 60000)) {
    PG.onlineAsked = true;
    api("online_worlds").then((o) => { PG.online = o; PG.offline = ""; }, (e) => { PG.offline = e.message; PG.offlineAt = Date.now(); })
      .finally(() => { if (S.route.screen === "setup" && setupState().step === 1) render(); });
  }
  const worlds = lib.worlds.slice();
  const test = st.loaded === "test";
  // A world open from outside the library (a save with no career yet).
  const extra = st.loaded && st.loaded !== "test" && !worlds.find((x) => x.path === st.loaded) ? st.loaded : null;
  if (!st.world || (st.world !== "test" && st.world !== extra && !worlds.find((x) => x.path === st.world))) st.world = test ? "test" : extra || (worlds[0] ? worlds[0].path : null);
  const champ = (list) => (list && list.length ? `${esc(list[0][0])} <span class="pg-dim">${esc(list[0][1])}</span>` : '<span class="pg-dim">-</span>');
  const rows = worlds.map((x) => {
    const w = x.card && x.card.world, m = x.meta || {};
    const years = w ? `${yrs(w.history_years)}, ${w.first}-${w.last}` : m.history_years ? yrs(m.history_years) : "";
    return `<tr data-pg="pick-world" data-path="${esc(x.path)}" class="${x.path === st.world ? "sel" : ""}">
      <td><b>${esc(worldTitle(x))}</b> <span class="pg-dim">· seed ${esc(m.seed)}</span></td><td>${esc(years)}</td><td>${esc(w ? w.starts : m.world_date || "")}</td>
      <td>${w ? champ(w.sb) : '<span class="pg-dim">read when it first opens</span>'}</td><td>${w ? champ(w.nc) : ""}</td>
      <td class="n">${esc(size(x.mb))}</td><td class="n">${esc(builtText(m.written))}</td></tr>`;
  });
  if (extra) rows.push(`<tr data-pg="pick-world" data-path="${esc(extra)}" class="${st.world === extra ? "sel" : ""}"><td><b>${esc(extra.split(/[\\/]/).pop())}</b> <span class="pg-dim">· open now</span></td><td colspan="6" class="pg-dim">A save with no career yet: a new career starts from it as it is.</td></tr>`);
  if (test) rows.push(`<tr data-pg="pick-world" data-path="test" class="${st.world === "test" ? "sel" : ""}"><td><b>Test world</b> <span class="pg-dim">· small, not kept</span></td><td>2 years</td><td>${esc(S.status ? S.status.today : "")}</td><td class="pg-dim">-</td><td class="pg-dim">-</td><td class="n pg-dim">in memory</td><td class="n"></td></tr>`);
  // A newer build of a world you have shows too: downloading it replaces yours.
  const online = PG.online ? PG.online.worlds.filter((o) => !o.have && (o.newer || !worlds.some((x) => x.name === o.name))) : [];
  for (const o of online) {
    const what = o.newer ? "A newer build of a world in your library, made by the current game: downloading it replaces yours (your saves keep their own copy)" : "Ready-built on the worlds page: download it into your library";
    rows.push(`<tr><td><b>${esc(o.title || o.name)}</b> <span class="pg-tag">${o.newer ? "NEWER BUILD" : "ONLINE"}</span></td><td colspan="4" class="pg-dim">${what}</td><td class="n">${esc(size(o.mb))}</td><td class="n"><button class="pg-btn s" data-pg="download" data-tag="${esc(o.tag)}">${o.newer ? "Update" : "Download"}</button></td></tr>`);
  }
  const sel = worlds.find((x) => x.path === st.world);
  const wc = sel && sel.card && sel.card.world;
  const story = st.world === "test"
    ? `<div class="pg-empty">A small test world for trying the game: two years of history at a fraction of the size. It isn't kept in the library.</div>`
    : wc ? `<div class="pg-story">${wc.story.map(([k, v]) => `<div><span>${esc(k)}</span><span>${esc(v)}</span></div>`).join("")}</div>`
    : sel ? `<div class="pg-empty">This world's story is read the first time it opens, and kept from then on.</div>` : "";
  const progress = job ? buildPanel(job) : "";
  const offline = !PG.online && PG.offline ? `<div class="pg-empty" title="${esc(PG.offline)}">The worlds to download can't be listed right now, so only the worlds on this computer show.</div>` : "";
  const names = wc ? wc.names : st.world === "test" && S.status && S.status.names ? String(S.status.names).toLowerCase() : "real";
  const body = `<div class="pg-body"><div class="pg-col">
      <div class="pg-panel"><div class="pg-ph"><span>World library <small>· ${worlds.length} world${worlds.length === 1 ? "" : "s"}</small></span>
        <span class="pg-acts">${isMac() ? '<button class="pg-btn s" data-pg="import">Import a world folder…</button><button class="pg-btn s" data-pg="import" data-file="1">Import a world file…</button>' : '<button class="pg-btn s" data-pg="import">Import from a path…</button>'}<button class="pg-btn s" data-pg="build">Build a new world</button></span></div>
        ${rows.length ? `<div class="pg-tbl" data-keep="worlds"><table><thead><tr><th>World</th><th>History</th><th>Starts</th><th>Super Bowl champions</th><th>National champions</th><th class="n">Size</th><th class="n">Built</th></tr></thead><tbody>${rows.join("")}</tbody></table></div>`
          : `<div class="pg-empty">Your library is empty. Download a ready-built world below, import a world folder, or build one.</div>`}
        ${progress}${offline}
      </div>
      ${story ? `<div class="pg-panel"><div class="pg-ph"><span>The world's story ${wc ? `<small>· what ${esc(yrs(wc.history_years))} left behind</small>` : ""}</span></div>${story}</div>` : ""}
    </div>
    <aside class="pg-aside">
      <div class="pg-panel"><div class="pg-ph"><span>Names</span><small>fixed for the whole career</small></div>
        <div class="pg-pad"><div class="pg-seg" role="group" aria-label="Names"><button class="${names === "real" ? "on" : ""}" ${names === "real" ? "" : "disabled"}>Real names</button><button class="${names === "fictional" ? "on" : ""}" ${names === "fictional" ? "" : "disabled"}>Fictional names</button></div>
        <p class="pg-note">${names === "real"
          ? "The real NFL clubs, real colleges and their real conferences. Towns are real; high schools, players and coaches are generated either way. A world's names are set when it is built: Build a new world offers fictional names."
          : "Every college, conference and NFL club carries a made-up name. The towns, the cities the clubs play in and the bowls stay real; high schools, players and coaches are generated either way."}</p></div></div>
      <div class="pg-panel"><div class="pg-ph"><span>Your new career</span></div>
        <div class="pg-kv"><span>World</span><span>${esc(st.world === "test" ? "Test world" : sel ? worldTitle(sel) : "-")}</span><span>Names</span><span>${names === "real" ? "Real names" : "Fictional names"}</span>
        <span>You</span><span class="pg-dim">step 2</span><span>Background</span><span class="pg-dim">step 3</span><span>Job</span><span class="pg-dim">step 6</span></div></div>
    </aside></div>`;
  return setupPage(st, {
    title: "Choose your world",
    lede: "Each world is a whole football country with its history behind it, stopped on the morning after the Super Bowl. Every career you start from it plays out fresh from that morning.",
    body,
    sum: esc(st.world === "test" ? "Test world" : sel ? `${worldTitle(sel)} · ${names === "real" ? "Real names" : "Fictional names"}` : "Pick a world"),
    next: "Next: You",
    nextOk: !!st.world,
  });
}

function buildPanel(j) {
  const pct = Math.round((j.done || 0) * 100);
  const min = Math.round((j.seconds || 0) / 60);
  const what = j.finished ? (j.error ? "Stopped" : "Ready") : j.kind === "build" ? "Building" : "Downloading";
  const stop = j.finished || j.stopping ? "" : `<button class="pg-btn s" data-pg="jobstop">Stop</button>`;
  return `<div class="pg-pad" id="pg-job" style="border-top:1px solid var(--pg-line)"><div style="display:flex;justify-content:space-between;gap:12px;align-items:center"><b>${esc(what)} · ${esc(j.name)} <span class="pg-dim">${pct}% · ${min} min</span></b>${stop}</div>
    <div class="pg-meter"><i style="width:${pct}%"></i></div><p class="pg-note" ${j.error ? 'style="color:#ff9c90"' : ""}>${esc(j.error || j.text)}</p></div>`;
}
function watchBuild() {
  clearInterval(PG.jobPoll);
  PG.jobPoll = setInterval(async () => {
    if (S.route.screen !== "setup" || setupState().step !== 1) return clearInterval(PG.jobPoll);
    let j = null;
    try { j = await api("job"); } catch (e) { return; }
    const el = $("#pg-job");
    if (el && j) el.outerHTML = buildPanel(j);
    if (!j || j.finished) { clearInterval(PG.jobPoll); PG.online = null; PG.onlineAsked = false; render(); }
  }, 2000);
}

// Step 2: you.
const HEIGHTS = Array.from({ length: 15 }, (_, i) => `${Math.floor((66 + i) / 12)} ft ${(66 + i) % 12}`);
const BUILDS = ["Lean", "Athletic", "Average", "Stocky", "Heavy", "Former lineman, still big", "Former linebacker, a little heavier", "Former receiver, still lean"];
const HAIR = ["Short, dark", "Short, greying at the sides", "Grey", "White", "Thinning", "Shaved", "Bald", "Longer, swept back", "Close-cropped", "Curly"];
const FACIAL = ["Clean-shaven", "Stubble", "Moustache", "Goatee", "Trimmed beard", "Full beard", "Grey beard"];
const HERITAGE = [["white", "White"], ["black", "Black"], ["hispanic", "Hispanic"], ["polynesian", "Polynesian"], ["east_asian", "East Asian"], ["southeast_asian", "Southeast Asian"], ["south_asian", "South Asian"]];
function ageOn(born, today) {
  const m = String(born || "").match(/^(\d{4})-(\d{2})-(\d{2})$/);
  if (!m) return null;
  const t = String(today || "").match(/(\d+) (\w{3}) (\d{4})$/);
  const ty = t ? Number(t[3]) : new Date().getFullYear(), tm = t ? MONTHS.indexOf(t[2]) + 1 : 1, td = t ? Number(t[1]) : 1;
  return ty - Number(m[1]) - ((tm < Number(m[2]) || (tm === Number(m[2]) && td < Number(m[3]))) ? 1 : 0);
}
// A child's school year from his age (US schools, a February date).
function schoolYear(age) {
  if (age < 5) return "Not in school yet";
  if (age === 5) return "Kindergarten";
  const g = age - 5;
  if (g <= 8) return `${g}${g === 1 ? "st" : g === 2 ? "nd" : g === 3 ? "rd" : "th"} grade${g === 8 ? ", starts high school in the fall" : ""}`;
  if (g <= 12) return ["Freshman", "Sophomore", "Junior", "Senior"][g - 9] + " in high school" + (g === 12 ? ": a move this year is hardest" : "");
  return age <= 22 ? "College age" : "Grown up";
}
const fullName = (st) => `${st.first.trim()} ${st.last.trim()}`.trim();
function youSummary(st) {
  const age = ageOn(st.born, PG.opts && PG.opts.today);
  const state = PG.opts ? (PG.opts.states.find((s) => s.code === st.state) || {}).name : st.state;
  return [fullName(st) || "Your name", age != null ? String(age) : "", st.hometown ? `${st.hometown}, ${state}` : state, st.alma_mater].filter(Boolean).map(esc).join(" · ");
}
// Your hometown in the world, with its schools.
async function hometownOf(st) {
  const towns = (PG.towns[st.state] ??= await api("towns", { state: st.state }).catch(() => ({ towns: [] })));
  return towns.towns.find((t) => t.town.toLowerCase() === st.hometown.trim().toLowerCase()) || null;
}
const schoolsHint = (town) => town ? `One of the world's ${town.schools.length} school${town.schools.length === 1 ? "" : "s"} in ${esc(town.town)}` : "Pick your hometown to see its schools";

// Who in the world knows you (lane 10): the world's own history finds the
// coaches who coached you in high school and college and the staffs you
// were on, from what you write on the You and Background steps.
const stateName = (code) => ((PG.opts && PG.opts.states.find((s) => s.code === code)) || {}).name || code;
function stopArgs(st) {
  return st.stops.filter((s) => s.id != null).map((s) => ({ employer: { kind: s.kind, id: s.id }, role: s.role === "played" ? "pc0" : s.role, from: Number(s.from), to: Number(s.to), playing: s.role === "played" }));
}
function knownArgs(st) {
  return {
    age: ageOn(st.born, PG.opts && PG.opts.today) || 42, home_state: st.state, background: st.bg,
    profile: { born: st.born, high_school: st.high_school, alma_mater: st.alma_mater, college_years: st.college_years },
    stops: stopArgs(st),
  };
}
async function knownHtml(st, n) {
  const args = knownArgs(st);
  const key = JSON.stringify(args);
  if (!PG.known || PG.known.key !== key) PG.known = { key, data: await api("people_you_know", args).catch(() => null) };
  const x = PG.known.data;
  if (!x) return `<div class="pg-ph"><span>People who know you</span></div><div class="pg-empty">The world can't be read right now.</div>`;
  const f = x.found;
  const where = [
    f.high_school ? `<span>High school</span><span>${esc(f.high_school.name)}, ${esc(f.high_school.years)}</span>`
      : st.high_school.trim() ? `<span>High school</span><span class="pg-warn">No ${esc(st.high_school.trim())} in the world's ${esc(stateName(st.state))}: pick one from the list</span>` : "",
    f.college ? `<span>College</span><span>${esc(f.college.name)}, ${esc(f.college.years)} · ${f.played ? "played" : "a student"}</span>`
      : st.alma_mater.trim() ? `<span>College</span><span class="pg-warn">${esc(st.alma_mater.trim())} isn't a college in the world: pick one from the list</span>` : "",
    st.stops.length ? `<span>Stops</span><span>${st.stops.filter((s) => s.id != null).length} of ${st.stops.length} found</span>` : "",
  ].join("");
  const rows = x.known.slice(0, n).map((k) => `<div class="pg-known"><div><b>${esc(k.name)}</b><span class="pg-tag">${esc(k.close)}</span></div><small>${esc(k.how)}, ${esc(k.years)}</small><small class="pg-dim">Now: ${esc(k.now)}</small></div>`).join("");
  const more = x.count > n ? `<div class="pg-empty" style="padding-top:8px">And ${x.count - n} more.</div>` : "";
  return `<div class="pg-ph"><span>People who know you</span><small>${x.count ? `${x.count} in the world` : "nobody yet"}</small></div>
    ${where ? `<div class="pg-kv">${where}</div>` : ""}
    ${rows ? `<div class="pg-knownlist">${rows}</div>${more}` : `<div class="pg-empty">Name your high school and college as the world knows them, and your last stops on the Background step: the coaches who were there then know you, and a search calls them about you.</div>`}`;
}
async function refreshKnown() {
  const el = $("#pg-known");
  if (!el) return;
  const html = await knownHtml(setupState(), Number(el.dataset.n) || 6);
  const now = $("#pg-known");
  if (now) now.innerHTML = html;
}
// Your coaches (update 3, SA1): the setup page's record of each coach you
// made, kept by the game outside any save when his career starts. A new
// career can take him again as he is (straight to the job) or with
// something changed (the steps open filled in with him).
const COACH_KEEP = ["first", "last", "born", "eth", "hometown", "state", "high_school", "alma_mater", "college_years", "degree", "status", "spouse", "roots", "children", "height", "build", "hair", "facial_hair", "photo", "bg", "pos", "ratings", "ratingsFor", "style", "calls", "stops", "rdLevel"];
function coachRecord(st) {
  const r = {};
  for (const k of COACH_KEEP) if (st[k] !== undefined && st[k] !== null) r[k] = st[k];
  return JSON.parse(JSON.stringify(r));
}
async function coachUse(st, file, asHe) {
  const c = ((PG.coaches && PG.coaches.coaches) || []).find((x) => x.file === file);
  if (!c) return;
  Object.assign(st, JSON.parse(JSON.stringify(c.coach)), { from: file });
  // His stops are looked up again in this world by their names.
  for (const s of st.stops || []) { s.id = null; await findStop(s); }
  PG.known = null;
  if (asHe) { st.reached = 6; toStep(st, 6); } else { st.reached = Math.max(st.reached, 5); toStep(st, 2); }
}
async function coachesPanel(st) {
  const x = (PG.coaches ??= await api("coach_profiles").catch(() => ({ coaches: [] })));
  if (!x.coaches.length) return "";
  const rows = x.coaches.map((c) => `<div class="pg-known pg-coach ${st.from === c.file ? "on" : ""}"><div><b>${esc(c.name)}</b><span class="pg-acts"><button class="pg-btn s p" data-pg="coach-use" data-file="${esc(c.file)}">Use him as he is</button><button class="pg-btn s" data-pg="coach-edit" data-file="${esc(c.file)}">Change something</button><button class="pg-btn s" data-pg="coach-forget" data-file="${esc(c.file)}" aria-label="Forget ${esc(c.name)}">Forget</button></span></div><small>${esc(c.road)} · kept ${esc(savedText(c.saved))}</small></div>`).join("");
  return `<div class="pg-panel"><div class="pg-ph"><span>Your coaches</span><small>kept by the game outside any save · or make a new one below</small></div><div class="pg-knownlist">${rows}</div></div>`;
}
async function stepYou(st) {
  const o = PG.opts;
  const town = await hometownOf(st);
  const towns = PG.towns[st.state] || { towns: [] };
  const age = ageOn(st.born, o.today);
  const f = (id, label, input, cls = "", small = "") => `<div class="pg-f ${cls}"><label for="pg-${id}">${label}</label>${input}${small ? `<small>${small}</small>` : ""}</div>`;
  const inp = (k, ph = "", list = "") => `<input id="pg-${k}" data-f="${k}" value="${esc(st[k])}" placeholder="${esc(ph)}" autocomplete="off" ${list ? `list="${list}"` : ""}>`;
  const sel = (k, opts) => `<select id="pg-${k}" data-f="${k}">${opts.map((x) => { const [v, t] = Array.isArray(x) ? x : [x, x]; return `<option value="${esc(v)}" ${v === st[k] ? "selected" : ""}>${esc(t)}</option>`; }).join("")}</select>`;
  const kids = st.children.map((c, i) => {
    const a = Math.max(0, o.year - c.born);
    return `<div><input data-kid="${i}" data-kf="name" value="${esc(c.name)}" placeholder="Name"><input data-kid="${i}" data-kf="age" type="number" min="0" max="40" value="${a}"><span class="pg-dim">${esc(schoolYear(a))}</span><button class="x" data-pg="kid-del" data-i="${i}" aria-label="Remove">×</button></div>`;
  }).join("");
  const kept = await coachesPanel(st);
  const body = `<div class="pg-body"><div class="pg-col">${kept}
      <div class="pg-panel"><div class="pg-ph"><span>Who you are</span></div><div class="pg-grid">
        ${f("first", "First name", inp("first", "First name"), !st.first.trim() && st.tried ? "bad" : "")}
        ${f("last", "Last name", inp("last", "Last name"), !st.last.trim() && st.tried ? "bad" : "")}
        ${f("born", "Date of birth", `<input id="pg-born" data-f="born" type="date" value="${esc(st.born)}" min="1945-01-01" max="2001-12-31">`, "", age != null ? `<span id="pg-age">${age} years old</span>` : '<span id="pg-age"></span>')}
        ${f("eth", "Heritage", sel("eth", HERITAGE))}
      </div></div>
      <div class="pg-panel"><div class="pg-ph"><span>Where you're from</span><small>people from your home area and your school know your name</small></div><div class="pg-grid">
        ${f("hometown", "Hometown", inp("hometown", "Your hometown", "pg-towns"))}
        ${f("state", "State", sel("state", o.states.map((s) => [s.code, s.name])))}
        ${f("high_school", "High school", inp("high_school", "Your high school", "pg-schools"), "w2", `<span id="pg-hshint">${schoolsHint(town)}</span>`)}
        ${f("alma_mater", "Alma mater", inp("alma_mater", "Where you went to college", "pg-programs"), "w2", "Where you went to college, played or not")}
        ${f("college_years", "Years there", inp("college_years", "2001-2004"))}
        ${f("degree", "Degree", inp("degree", "Kinesiology"))}
      </div></div>
      <div class="pg-panel"><div class="pg-ph"><span>Family</span><small>a move is easier on a family with young children than with a high school senior</small></div><div class="pg-grid">
        ${f("status", "Status", sel("status", ["Married", "Single", "Divorced", "Widowed"]))}
        ${f("spouse", "Spouse", `<input id="pg-spouse" data-f="spouse" value="${esc(st.spouse)}" placeholder="${st.status === "Married" ? "Your spouse's name" : ""}" ${st.status === "Married" ? "" : "disabled"}>`)}
        ${f("roots", "Where your roots are", inp("roots", "Both sets of parents in your hometown"), "w2")}
      </div>
      <div class="pg-kids" style="border-top:1px solid var(--pg-line)"><div class="h"><span>Children</span><span>Age</span><span>In school</span><span></span></div>${kids || '<div><span class="pg-dim">No children</span></div>'}</div>
      <div class="pg-acts-row" style="padding-top:12px"><button class="pg-btn s" data-pg="kid-add">Add a child</button></div></div>
    </div>
    <aside class="pg-aside">
      <div class="pg-panel"><div class="pg-ph"><span>Your photo</span><small>optional</small></div><div class="pg-pad">
        <div style="display:flex;gap:14px;align-items:center">
          <div class="pg-photo ${st.photo ? "has" : ""}" style="${st.photo ? `background-image:url(${st.photo})` : ""}">${st.photo ? "" : '<svg width="44" height="44" viewBox="0 0 24 24" fill="none" stroke="#5F6670" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="8" r="4"/><path d="M4 21c1.5-4 4.5-6 8-6s6.5 2 8 6"/></svg>'}</div>
          <div style="display:grid;gap:8px"><button class="pg-btn s" data-pg="photo">${st.photo ? "Change the photo…" : "Add a photo…"}</button>${st.photo ? '<button class="pg-btn s" data-pg="photo-del">Remove it</button>' : ""}<button class="pg-btn s" data-pg="prompt">Copy AI prompt</button></div>
        </div>
        <input type="file" id="pg-photo-file" accept="image/*" hidden>
        <p class="pg-note">Copy AI prompt writes a portrait prompt from your looks below, for any image tool. Your photo stays with you through every media day.</p>
      </div></div>
      <div class="pg-panel"><div class="pg-ph"><span>Looks</span></div><div class="pg-grid two">
        ${f("height", "Height", sel("height", HEIGHTS))}${f("build", "Build", sel("build", BUILDS))}
        ${f("hair", "Hair", sel("hair", HAIR))}${f("facial_hair", "Facial hair", sel("facial_hair", FACIAL))}
      </div><div class="pg-pad" style="padding-top:0"><p class="pg-note">Looks are for your photo and the press. What a room feels when you walk in is your presence: the world reads it from your road and how you deal with people, and nobody shows it to you as a number.</p></div></div>
      <div class="pg-panel" id="pg-known" data-n="6">${await knownHtml(st, 6)}</div>
    </aside></div>
    <datalist id="pg-towns">${towns.towns.map((t) => `<option value="${esc(t.town)}">`).join("")}</datalist>
    <datalist id="pg-schools">${(town ? town.schools : []).map((s) => `<option value="${esc(s)}">`).join("")}</datalist>
    <datalist id="pg-programs">${o.programs.map((p) => `<option value="${esc(p)}">`).join("")}</datalist>`;
  return setupPage(st, {
    title: "You",
    lede: "The man the players, boosters and press will meet. Where you grew up and where you played decide who already knows you, and your family moves with you, or doesn't, every time you take a new job.",
    body, sum: `<span id="pg-yousum">${youSummary(st)}</span>`, next: "Next: Background",
  });
}

// Step 3: the road into coaching. The words for each road (people topic 15).
const ROADS = {
  nfl_star: { road: "A decade in the league with All-Pro years, then straight into coaching.", like: "Deion Sanders, Mike Vrabel", knows: "Old teammates and coaches across the NFL, agents, and every recruit's parents.", greet: "Boosters and recruits are thrilled from day one. The media asks whether you can coach.", coached: "A short apprenticeship, then a head job." },
  nfl_pro_bowl: { road: "A long NFL career with Pro Bowls, then a few years as an assistant.", like: "Mike Singletary", knows: "NFL teammates and coaches, a good number of agents.", greet: "Respected for what you did on the field. Boosters want quick results.", coached: "A few years as an NFL position coach." },
  nfl_starter: { road: "Several years starting in the NFL, then the assistant ladder.", like: "Dan Campbell", knows: "Teammates and coaches from your NFL clubs.", greet: "Players listen because you played. Big-program boosters need convincing.", coached: "Position coach, then coordinator." },
  nfl_journeyman: { road: "Seven clubs in nine years, learning every room you sat in.", like: "Doug Pederson", knows: "Coaches from many NFL staffs, a wide spread of people.", greet: "Little known. A big program's boosters and press start skeptical.", coached: "Quality control, then position coach." },
  college_player: { road: "Played in college, then worked up from graduate assistant.", like: "Kirby Smart, Sean McVay", knows: "Your college teammates and the staff that coached you.", greet: "Unproven. A blue blood's boosters, media and recruits start skeptical.", coached: "Graduate assistant, then a small-college position coach." },
  never_played: { road: "No playing career past high school; you got in through the X's and O's.", like: "Mike Leach", knows: "Few people. The coaches you learned from.", greet: "Unproven, and you never played. The most skeptical start in the game.", coached: "Lower-division coordinator, a long climb." },
  hs_coach: { road: "Years running a high school program, then the jump to college.", like: "Gus Malzahn", knows: "High school coaches in your state and the college recruiters who visited.", greet: "Unproven at this level. High school coaches in your state open their doors.", coached: "Head coach of a high school program." },
  college_assistant: { road: "The usual road: position coach, then coordinator, recruiting your areas.", like: "Kirby Smart, Dabo Swinney", knows: "College staffs you worked on, high school coaches in your recruiting areas.", greet: "Little known to fans. Recruits in your areas already know you.", coached: "Position coach, then coordinator, on college staffs." },
  nfl_assistant: { road: "Years on NFL staffs, ending as a coordinator.", like: "Sean McDermott", knows: "NFL coaches, GMs and scouts from the staffs you were on.", greet: "Respected for your scheme. College boosters doubt your recruiting.", coached: "NFL position coach, then coordinator." },
  scout: { road: "Years on the road grading players, then into the front office.", like: "John Schneider (most GMs start here)", knows: "Scouts, GMs, agents and the college staffs you visited.", greet: "Unproven as a coach. Agents and personnel people take your calls.", coached: "Area scout, then director of college scouting." },
  analytics: { road: "From the numbers side: game-day decisions, roster value, the data room.", like: "Paul DePodesta (Browns, 2016)", knows: "Front-office people around the league.", greet: "Unproven, and old-school boosters and media won't trust it.", coached: "Director of football analytics." },
};
const repWord = (r) => (r >= 16 ? "One of the best in the game" : r >= 13 ? "Highly regarded" : r >= 10 ? "Respected" : r >= 7 ? "Little known" : "Unproven");
// Your last stops before this job (lane 10): the staffs you were on and,
// on a playing road, the NFL clubs you played for. The coaches who were
// there with you know you; a coaching stop goes on your record.
const STOP_KINDS = [["college", "College"], ["nfl", "NFL"], ["hs", "High school"]];
const STOP_ROLES = [["hc", "Head coach"], ["oc", "Offensive coordinator"], ["dc", "Defensive coordinator"], ["pc0", "Quarterbacks coach"], ["pc1", "Running backs coach"], ["pc2", "Wide receivers coach"], ["pc3", "Offensive line coach"], ["pc4", "Defensive line coach"], ["pc5", "Linebackers coach"], ["pc6", "Defensive backs coach"], ["pc7", "Special teams coordinator"]];
const MAX_STOPS = 6;
async function stopTeams(kind) {
  return (PG.jobs[kind] ??= await api("jobs", { level: kind })).jobs;
}
async function findStop(s) {
  const name = s.name.trim().toLowerCase();
  if (!name) { s.id = null; return; }
  const hit = (await stopTeams(s.kind)).find((j) => j.name.toLowerCase() === name);
  s.id = hit ? hit.ref.id : null;
  if (hit) s.name = hit.name;
}
function newStop(st) {
  const bg = PG.opts.backgrounds.find((b) => b.key === st.bg) || {};
  const last = st.stops[st.stops.length - 1];
  const to = last ? Math.max(1970, Number(last.from) - 1) : PG.opts.year - 1;
  const [kind, role] = bg.ex_pro && !st.stops.some((s) => s.role === "played") ? ["nfl", "played"] : bg.key === "nfl_assistant" || bg.key === "nfl_star" || bg.key === "nfl_pro_bowl" ? ["nfl", "pc0"] : bg.key === "hs_coach" ? ["hs", "hc"] : ["college", "pc0"];
  return { kind, name: "", id: null, role, from: to - 3, to };
}
async function stopsPanel(st) {
  const bg = PG.opts.backgrounds.find((b) => b.key === st.bg) || {};
  const kinds = new Set(st.stops.map((s) => s.kind));
  const lists = {};
  for (const k of kinds) {
    let l = await stopTeams(k);
    // A high school stop lists your home state's schools (any other by name).
    if (k === "hs") l = l.filter((j) => j.state === st.state);
    lists[k] = l;
  }
  const opt = (v, t, cur) => `<option value="${esc(v)}" ${v === cur ? "selected" : ""}>${esc(t)}</option>`;
  const rows = st.stops.map((s, i) => {
    const roles = [...(s.kind === "nfl" && bg.ex_pro ? [["played", "Played there"]] : []), ...STOP_ROLES];
    const bad = s.name.trim() && s.id == null;
    return `<div class="pg-stop ${bad ? "bad" : ""}">
      <select data-stop="${i}" data-sf="kind" aria-label="Level">${STOP_KINDS.map(([v, t]) => opt(v, t, s.kind)).join("")}</select>
      <input data-stop="${i}" data-sf="name" list="pg-stop-${s.kind}" value="${esc(s.name)}" placeholder="${s.kind === "nfl" ? "The club" : s.kind === "hs" ? "The school" : "The college"}" autocomplete="off" aria-label="Team">
      <select data-stop="${i}" data-sf="role" aria-label="Job">${roles.map(([v, t]) => opt(v, t, s.role)).join("")}</select>
      <input data-stop="${i}" data-sf="from" type="number" min="1970" max="${PG.opts.year}" value="${esc(s.from)}" aria-label="From">
      <input data-stop="${i}" data-sf="to" type="number" min="1970" max="${PG.opts.year}" value="${esc(s.to)}" aria-label="To">
      <button class="x" data-pg="stop-del" data-i="${i}" aria-label="Remove">×</button></div>`;
  }).join("");
  const lists_html = Object.entries(lists).map(([k, l]) => `<datalist id="pg-stop-${k}">${l.map((j) => `<option value="${esc(j.name)}">`).join("")}</datalist>`).join("");
  return `<div class="pg-panel"><div class="pg-ph"><span>Where you've been</span><small>your last stops before this job, newest first · optional</small></div>
    ${rows ? `<div class="pg-stops"><div class="pg-stop h"><span>Level</span><span>Team</span><span>Job</span><span>From</span><span>To</span><span></span></div>${rows}</div>` : `<div class="pg-empty">No stops: the world knows you from your road, your high school and your college. Add the staffs you were on${bg.ex_pro ? " and the NFL clubs you played for" : ""}, and the coaches who were there with you know you.</div>`}
    <div class="pg-acts-row" style="padding-top:12px"><button class="pg-btn s" data-pg="stop-add" ${st.stops.length >= MAX_STOPS ? "disabled" : ""}>Add a stop</button>${st.stops.some((s) => s.name.trim() && s.id == null) ? '<span class="pg-note" style="align-self:center;color:var(--pg-amber)">A team not in the world is left out: pick one from the list.</span>' : ""}</div>${lists_html}</div>`;
}
// Step 3: your road (update 3, lane 2, coaching C3-C7, board 40). Each road
// locks its strengths at elite values outside the pool (C3); every other
// rating starts at the road's typical level, and those starts are the pool
// (C4). A playing road picks the position you played and an assistant's road
// the group you came up coaching (C5). How you compare is read from the
// world's own coaches at the level of your first job (C7).
const RD_GROUPS = [["qb", "QB", "Quarterbacks"], ["rb", "RB", "Running backs"], ["wr", "WR/TE", "Receivers"], ["ol", "OL", "Offensive line"], ["dl", "DL", "Defensive line"], ["lb", "LB", "Linebackers"], ["db", "DB", "Defensive backs"], ["st", "ST", "Special teams"]];
const RD_OTHER = ["scheme", "play_calling", "game_management", "adaptation", "motivation", "discipline", "recruiting", "network", "relationships", "evaluation", "negotiating"];
const RD_SCALE = 300;
const RD_NOPOS = {
  never_played: "No position: you never played past high school. Your strengths are the system and the calls.",
  hs_coach: "Every teaching group starts at 14: a high school head coach teaches the whole roster.",
  scout: "No position or group: your strengths are judging players and the deals around them.",
  analytics: "No position or group: your strengths are judging players and game-day decisions.",
};
const RD_TIES = { hs_coach: "About 30 programs in your state", college_assistant: "About 25 programs in your recruiting area", never_played: "None yet", nfl_assistant: "None yet", scout: "None yet", analytics: "None yet" };
const RD_NAMEFX = {
  nfl_star: ["Your name draws transfers to your school", "Your name adds about 3 recruiting points for two seasons, fading by the fourth unless you win", "A small pull on free agents, who follow money and winning"],
  nfl_pro_bowl: ["Your name draws a few transfers", "Your name adds about 1.5 recruiting points for two seasons", "A small pull on free agents, who follow money and winning"],
};
const RD_NAMEFX_NONE = ["Your name doesn't move them yet", "Your name doesn't move recruits yet", "Your name doesn't move them yet"];
const RD_LEVELS = [["hs", "High school"], ["college", "College"], ["nfl", "NFL"]];
const rdLevel = (st) => st.rdLevel || (["hs", "college", "nfl"].includes(st.level) ? st.level : "college");
const rdPick = (st, bg) => (bg.picks ? (st.pos && st.pos[bg.key]) || bg.group : null);
// A road's starting ratings with the position or group picked, as the
// game's own setup reads them (setup.rs, `starting`).
function rdValues(bg, pick) {
  const keys = PG.opts.ratings.map((r) => r.key);
  const v = keys.map(() => bg.base);
  const lock = new Set();
  const g = bg.picks ? keys.indexOf(pick || bg.group) : -1;
  for (const [k, x] of bg.locks) {
    const at = k === "groups" ? [0, 1, 2, 3, 4, 5, 6, 7] : k === "pos" || k === "grp" ? (g >= 0 ? [g] : []) : [keys.indexOf(k)];
    for (const i of at) if (i >= 0) { v[i] = x; lock.add(i); }
  }
  const locked = [...lock].reduce((a, i) => a + v[i], 0);
  const free = keys.length - lock.size;
  return { v, lock, locked, free, pool: free * bg.base, tot: locked + free * bg.base, g, keys };
}
function rdLocksText(bg, x) {
  const name = (k) => (PG.opts.ratings.find((r) => r.key === k) || { name: k }).name;
  return bg.locks.map((l, i) => [l, i]).sort((a, b) => b[0][1] - a[0][1] || a[1] - b[1]).map(([[k, v]]) => {
    if (k === "groups") return `All 8 groups <b>${v}</b>`;
    if (k === "pos" || k === "grp") return `${k === "pos" ? "His position" : "His group"} ${esc(x.g >= 0 ? RD_GROUPS[x.g][1] : "")} <b>${v}</b>`;
    return `${esc(name(k))} <b>${v}</b>`;
  }).join(" · ");
}
function rdBlocks(x, gap) {
  const order = [...x.v.keys()];
  return [...order.filter((i) => x.lock.has(i)), ...order.filter((i) => !x.lock.has(i))].map((i) => `<i class="${x.lock.has(i) ? "lk" : ""}" style="width:calc(${((x.v[i] / RD_SCALE) * 100).toFixed(2)}% - ${gap}px)"></i>`).join("");
}
function rdCompare(st, bg, x) {
  const m = PG.opts.marks;
  const lv = rdLevel(st);
  const at = (k) => x.v[x.keys.indexOf(k)];
  const bestG = x.g >= 0 ? x.g : [0, 1, 2, 3, 4, 5, 6, 7].reduce((b, i) => (x.v[i] > x.v[b] ? i : b), 0);
  const best = x.v[bestG];
  const f = (n) => (Math.round(n * 10) / 10).toString();
  let title, rows;
  if (lv === "hs") {
    title = "Against high school head coaches";
    rows = [["Your best teaching group", best, `Median head coach's best group ${f(m.hs.best)}`, [m.hs.best]], ["Everything else", bg.base, `Top tenth of head coaches, about ${f(m.hs.other_top)} overall`, [m.hs.other_top]]];
  } else if (lv === "nfl") {
    title = "Against NFL staffs";
    rows = [["Scheme", at("scheme"), `Coordinators ${f(m.nfl.scheme[0])}, head coaches ${f(m.nfl.scheme[1])}`, m.nfl.scheme], ["Play calling", at("play_calling"), `Coordinators ${f(m.nfl.calling[0])}, head coaches ${f(m.nfl.calling[1])}`, m.nfl.calling], ["Evaluation", at("evaluation"), `Head coaches ${f(m.nfl.eval[0])}; the best scouting director ${f(m.nfl.eval[1])}`, m.nfl.eval]];
  } else {
    title = "Against top-68 FBS staffs";
    const [med, top] = m.college.groups[bestG];
    const gn = RD_GROUPS[bestG];
    rows = [[x.g >= 0 ? `Teaching your position, ${gn[1]}` : `Your best teaching group, ${gn[1]}`, best, `${gn[2]} coaches: median ${f(med)}, best tenth ${f(top)}`, [med, top]], ["Scheme", at("scheme"), `Top-68 FBS coordinators, about ${f(m.college.coord_scheme)}`, [m.college.coord_scheme]], ["Everything else", bg.base, `Coordinators about ${f(m.college.coord_other[0])}; FCS ${f(m.college.coord_other[1])}, small colleges ${f(m.college.coord_other[2])}`, [m.college.coord_other[0]]]];
  }
  const pct = (v) => (((Math.max(1, Math.min(20, v)) - 1) / 19) * 100).toFixed(1);
  const axis = [1, 5, 10, 15, 20].map((v) => `<span style="left:${pct(v)}%">${v}</span>`).join("");
  const body = rows.map(([t, y, cap, ticks]) => `<div class="pg-rd-cr"><div class="pg-rd-crh"><b>${esc(t)}</b><em class="${y >= ticks[0] ? "up" : ""}">${y}</em></div>
    <div class="pg-rd-sx"><i class="tr"></i>${ticks.map((v) => `<i class="tk" style="left:${pct(v)}%"><small>${f(v)}</small></i>`).join("")}<i class="you ${y >= ticks[0] ? "up" : ""}" style="left:${pct(y)}%"></i></div><small>${esc(cap)}</small></div>`).join("");
  return `<div class="pg-rd-panel"><div class="pg-rd-h"><span>${esc(title)}</span><small><i class="you"></i>you <i class="them"></i>them</small></div><div class="pg-rd-ax">${axis}</div>${body}</div>`;
}
async function stepBackground(st) {
  const o = PG.opts;
  const stops = await stopsPanel(st);
  const bg = o.backgrounds.find((b) => b.key === st.bg) || o.backgrounds[0];
  const r = ROADS[bg.key] || {};
  const lv = rdLevel(st);
  const all = o.backgrounds.map((b) => rdValues(b, rdPick(st, b)));
  const k = o.backgrounds.indexOf(bg);
  const x = all[k];
  const tots = all.map((a) => a.tot);
  const mn = Math.min(...tots), mx = Math.max(...tots);
  const rank = 1 + tots.filter((t) => t > x.tot).length;
  const ord = (n) => n + (n % 10 === 1 && n !== 11 ? "st" : n % 10 === 2 && n !== 12 ? "nd" : n % 10 === 3 && n !== 13 ? "rd" : "th");
  const guide = ((x.tot / RD_SCALE) * 100).toFixed(2);
  const rows = o.backgrounds.map((b, j) => {
    const a = all[j];
    const marks = [...a.lock].map((i) => "m-" + a.keys[i]).join(" ");
    return `<button class="pg-rd-rw r-${j} ${j === k ? "on" : ""}" data-pg="pick-bg" data-k="${b.key}"><i class="pg-rd-mk ${marks}"></i>
      <span class="pg-rd-rh"><b>${esc(b.name)}</b><em>${a.tot}</em></span>
      <span class="pg-rd-rb"><span class="pg-rd-tk">${rdBlocks(a, 1)}<i class="gd" style="left:${guide}%"></i></span><small><b>${a.locked}</b> + ${a.free} × ${b.base}</small></span>
      <span class="pg-rd-rs">${rdLocksText(b, a)}</span></button>`;
  }).join("");
  const sorted = bg.locks.map((l, i) => [l, i]).sort((a, b) => b[0][1] - a[0][1] || a[1] - b[1]).map(([l]) => l);
  const sum = sorted.map(([kk, v]) => (kk === "groups" ? `8 × ${v}` : String(v))).join(" + ");
  const pickT = bg.picks === "pos" ? "The position you played" : "The group you came up coaching";
  const pos = bg.picks
    ? `<div class="pg-rd-pos"><span>${pickT}</span><div>${RD_GROUPS.map(([gk, s], i) => `<button class="${i === x.g ? "on" : ""}" data-pg="rd-pos" data-k="${gk}">${s}</button>`).join("")}</div></div>`
    : `<p class="pg-rd-nopos">${esc(RD_NOPOS[bg.key] || "")}</p>`;
  const cell = (i, n) => {
    const lk = x.lock.has(i), v = x.v[i];
    return `<div class="pg-rd-cl c-${x.keys[i]} ${lk ? "lk" : ""}" tabindex="0"><span class="n">${esc(n)}</span><b>${v}${lk ? '<svg viewBox="0 0 10 12" aria-label="locked"><rect x="1" y="5" width="8" height="6.5" rx="1.2"/><path d="M3 5V3.6a2 2 0 0 1 4 0V5" fill="none" stroke-width="1.6"/></svg>' : ""}</b><i><s style="width:${((v / 20) * 100).toFixed(1)}%"></s></i></div>`;
  };
  const teach = RD_GROUPS.map(([, s, n], i) => cell(i, i === 2 ? "WR and TE" : s)).join("");
  const other = RD_OTHER.map((key) => { const i = x.keys.indexOf(key); return cell(i, o.ratings[i].name.replace("In-game adaptation", "Adaptation")); }).join("");
  const nLock = (ids) => ids.filter((i) => x.lock.has(i)).length;
  const legend = `<div class="pg-rd-cl pg-rd-leg"><span><i class="lk"></i>Locked at the road's value</span><span><i></i>From the pool, starts at ${bg.base}</span></div>`;
  const pct = (v) => (((v - 1) / 19) * 100).toFixed(1);
  const rep = [["name_rep", "Name", "Fans, recruits and boosters"], ["standing", "Standing", "Owners, GMs, big-program ADs, other coaches"]].map(([key, n, cap]) => `<div class="pg-rd-rep"><div class="pg-rd-crh"><span><b>${n}</b><small>${cap}</small></span><em>${bg[key]}</em></div>
    <div class="pg-rd-sx">${[5, 10, 15].map((v) => `<i class="mn" style="left:${pct(v)}%"></i>`).join("")}<i class="tr"></i>${o.backgrounds.map((b, j) => (j === k ? "" : `<i class="rt t-${j}" style="left:${pct(b[key])}%"></i>`)).join("")}<i class="you up" style="left:${pct(bg[key])}%"></i></div></div>`).join("");
  const played = bg.ex_pro
    ? `${st.alma_mater || "College"}${st.college_years ? ", " + st.college_years : ""}, then the NFL`
    : bg.key === "never_played" ? `High school only${st.high_school ? ", at " + st.high_school : ""}` : `${st.alma_mater || "College"}${st.college_years ? ", " + st.college_years : ""}`;
  const lvI = ["hs", "college", "nfl"].indexOf(lv);
  const namefx = (RD_NAMEFX[bg.key] || RD_NAMEFX_NONE)[lvI];
  const ties = RD_TIES[bg.key] || "Your own high school";
  const also = [[["Transfers", "Recruits", "Free agents"][lvI], namefx], ["High school ties", ties], ["Knows you", r.knows]].map(([a, b]) => `<div class="pg-rd-al"><small>${esc(a)}</small><span>${esc(b)}</span></div>`).join("");
  const hover = o.ratings.map((rr) => `.pg-rd:has(.c-${rr.key}:hover) .m-${rr.key},.pg-rd:has(.c-${rr.key}:focus) .m-${rr.key}`).join(",") + "{opacity:1}" +
    o.backgrounds.map((_, j) => `.pg-rd:has(.r-${j}:hover) .t-${j}`).join(",") + "{background:#f4f5f7;top:0;height:14px}";
  const body = `<style>${hover}</style><div class="pg-rd">
    <div class="pg-rd-top"><span>First job at</span><div class="pg-seg">${RD_LEVELS.map(([kk, n]) => `<button class="${kk === lv ? "on" : ""}" data-pg="rd-level" data-k="${kk}">${n}</button>`).join("")}</div></div>
    <section class="pg-rd-panel pg-rd-list"><div class="pg-rd-h big"><span>Eleven roads</span><small><i class="lk"></i>locked <i></i>pool <i class="gd"></i>your total</small></div>${rows}
      <p class="pg-rd-foot">Bars share one scale, 0 to 300. Totals were 174 to 239 when the road was one pool; now ${mn} to ${mx}. Point at a rating to mark the roads that lock it.</p></section>
    <section class="pg-rd-mid">
      <div class="pg-rd-panel pg-rd-hero"><div class="pg-rd-hh"><div><small>Your road</small><h2>${esc(bg.name)}</h2><p>${esc(r.road || "")}</p></div><span>Like ${esc(r.like || "")}</span></div>
        <div class="pg-rd-eq"><div><small>Locked strengths</small><b class="gold">${x.locked}</b><span>${x.lock.size} ${x.lock.size === 1 ? "rating" : "ratings"}: ${sum}</span></div><i>+</i>
          <div><small>Pool to spread</small><b>${x.pool}</b><span>${x.free} ratings × ${bg.base} each</span></div><i>=</i>
          <div><small>Total</small><b>${x.tot}</b><span>${rank === 1 ? "Highest of the 11 roads" : `${ord(rank)} highest of the 11 roads`}</span></div></div>
        <div class="pg-rd-hb">${rdBlocks(x, 2)}</div>
        <div class="pg-rd-hf"><span>One block per rating, gold if locked</span><span class="pg-rd-range"><span>All 11 roads</span><span class="bar"><i style="left:${((mn / RD_SCALE) * 100).toFixed(2)}%;width:${(((mx - mn) / RD_SCALE) * 100).toFixed(2)}%"></i><b style="left:${guide}%"></b></span><span>${mn}–${mx}</span></span></div>
        <p class="pg-rd-note">Locked strengths can't be spent below the road's value; pool points can still raise them to 20. You spread the pool in the next step.</p></div>
      ${pos}
      <div class="pg-rd-grids"><div><div class="pg-rd-gh"><span>Teaching groups · 8</span><small>${nLock([0, 1, 2, 3, 4, 5, 6, 7])} locked</small></div><div class="pg-rd-gb two">${teach}</div></div>
        <div><div class="pg-rd-gh"><span>The other 11 ratings</span><small>${nLock(RD_OTHER.map((kk) => x.keys.indexOf(kk)))} locked</small></div><div class="pg-rd-gb three">${other}${legend}</div></div></div>
    </section>
    <aside class="pg-rd-side">
      <div class="pg-rd-panel"><div class="pg-rd-h"><span>Reputation</span><small>1 to 20 · <i class="them"></i>the other ten roads</small></div>${rep}<p class="pg-rd-foot">Starting values proposed, not confirmed.</p></div>
      <div class="pg-rd-panel"><div class="pg-rd-h"><span>Also from the road</span></div>${also}</div>
      ${rdCompare(st, bg, x)}
    </aside></div>
    <div class="pg-body pg-rd-below"><div class="pg-col">${stops}</div><aside class="pg-aside">
      <div class="pg-panel"><div class="pg-ph"><span>Where the world places you</span><small>from your profile</small></div><div class="pg-kv">
        <span>Played</span><span>${esc(played)}</span><span>Coached</span><span>${esc(r.coached || "")}</span><span>At a big job</span><span>${esc(r.greet || "")}</span></div></div>
      <div class="pg-panel" id="pg-known" data-n="10">${await knownHtml(st, 10)}</div></aside></div>`;
  return setupPage(st, {
    title: "Your road into coaching",
    lede: "The road locks the strengths you came up with, sets the pool you spread over the rest, and decides who in the world already knows you. Real head coaches came up every one of these ways.",
    body, sum: esc(`${bg.name} · ${x.locked} locked + ${x.pool} pool = ${x.tot}`), next: "Next: Ratings",
  });
}

// Step 4: the ratings. What each drives and how it grows (people topic 15).
const TEACH = (pos) => [`How fast your ${pos} grow, in practice and in film`, "Coaching the group yourself, a strong position coach beside you"];
const RATING_TEXT = {
  qb: TEACH("quarterbacks"), rb: TEACH("running backs"), wr: TEACH("receivers and tight ends"), ol: TEACH("offensive linemen"),
  dl: TEACH("defensive linemen"), lb: TEACH("linebackers"), db: TEACH("corners and safeties"), st: TEACH("specialists and coverage units"),
  scheme: ["How well your system fits together and wins its battles", "Years running it, clinics, a sharp coordinator"],
  game_management: ["Clock, timeouts, fourth downs, two-point calls", "Head coaching games"],
  play_calling: ["Picking the right call for the down, distance and defense, when you call the plays", "Calling plays yourself"],
  adaptation: ["Seeing what the opponent does and answering it by halftime", "Calling plays yourself"],
  motivation: ["Effort, belief and how a team plays when it is behind", "Talks that land, winning"],
  discipline: ["Penalties, missed assignments, trouble off the field", "Holding to your rules"],
  relationships: ["Trust from players, staff and bosses", "Keeping your word, time with people"],
  recruiting: ["Visits, home visits, closing on signing day", "Recruiting yourself; fades in the NFL"],
  evaluation: ["Judging what a player is now and what he can become: how sure your reads are", "Watching film and players in person"],
  network: ["Who returns your call: coaches, GMs, agents, high school coaches", "Every job and every staff you work on"],
  negotiating: ["Reading agents' bluffs and floors in contract talks (the NFL for now)", "Doing deals"],
};
const vcls = (v) => "pg-v " + (v <= 5 ? "c1" : v <= 10 ? "c2" : v <= 15 ? "c3" : "c4");
// The road's starting ratings with your position or group (C5), and each
// one's floor: a locked strength can't go below the road's value (C3).
function bgRatings(st) {
  const bg = PG.opts.backgrounds.find((b) => b.key === st.bg) || PG.opts.backgrounds[0];
  const pick = rdPick(st, bg);
  const x = rdValues(bg, pick);
  const key = `${bg.key}|${pick || ""}`;
  if (!st.ratings || st.ratingsFor !== key) { st.ratings = x.v.slice(); st.ratingsFor = key; }
  st.floor = x.v.map((v, i) => (x.lock.has(i) ? v : 1));
  return bg;
}
async function stepRatings(st) {
  const o = PG.opts;
  const bg = bgRatings(st);
  const spent = st.ratings.reduce((a, b) => a + b, 0);
  const left = bg.pool - spent;
  let group = "", body = "";
  o.ratings.forEach((r, i) => {
    if (r.group !== group) { group = r.group; body += `<tr class="g"><td colspan="5">${esc(group)}</td></tr>`; }
    const v = st.ratings[i];
    const [drives, grows] = RATING_TEXT[r.key] || ["", ""];
    body += `<tr><td style="font-weight:600">${esc(r.name)}${st.floor[i] > 1 ? ` <span class="pg-tag" title="Locked at ${st.floor[i]}: it can't go lower">locked</span>` : ""}</td><td><span class="${vcls(v)}" id="pg-v${i}">${v}</span></td>
      <td><span class="pg-pm"><button data-pg="rate" data-i="${i}" data-d="-1" aria-label="Lower ${esc(r.name)}" ${v <= st.floor[i] ? `disabled title="Locked: your road's value is the floor"` : ""}>−</button><button data-pg="rate" data-i="${i}" data-d="1" aria-label="Raise ${esc(r.name)}" ${v >= 20 || left <= 0 ? "disabled" : ""}>+</button></span></td>
      <td class="d">${esc(drives)}</td><td class="d">${esc(grows)}</td></tr>`;
  });
  const top = o.ratings.map((r, i) => [r.name, st.ratings[i]]).sort((a, b) => b[1] - a[1]).slice(0, 3).map((x) => `${x[0]} ${x[1]}`).join(" · ");
  const at = (k) => st.ratings[o.ratings.findIndex((r) => r.key === k)];
  const teach = st.ratings.slice(0, 8).reduce((a, b) => a + b, 0) / 8;
  const read = at("recruiting") >= 15 ? "High school coaches in your areas already call you a closer."
    : at("play_calling") >= 15 ? "Staffs around the game study your play calling."
    : teach >= 14 ? "Known on staffs as a teacher: players get better in your rooms."
    : at("scheme") >= 15 ? "Your scheme is known: other staffs copy your playbook."
    : at("evaluation") >= 15 ? "Personnel people trust your eye for a player."
    : at("motivation") >= 15 ? "Players say they'd run through a wall for you."
    : "A coach few fans have heard of. Results will write your name.";
  const key = `${bg.key}|${st.ratings.slice(0, 8).join(",")}`;
  const like = (PG.similar[key] ??= await api("similar", { background: bg.key, ratings: st.ratings }).catch(() => ({ coaches: [] })));
  const pageBody = `<div class="pg-body"><div class="pg-col"><div class="pg-panel">
      <div class="pg-ph"><span>${esc(bg.name)} <small>· locked strengths, and the pool where the road starts it</small></span><button class="pg-btn s" data-pg="rate-reset">Reset to the road's starts</button></div>
      <div class="pg-tbl pg-ratings"><table><thead><tr><th>Rating</th><th>Value</th><th></th><th>What it drives</th><th>Grows by</th></tr></thead><tbody>${body}</tbody></table></div></div></div>
    <aside class="pg-aside">
      <div class="pg-panel"><div class="pg-ph"><span>Points</span><small>${bg.locked} locked + ${bg.pool - bg.locked} pool = ${bg.pool}</small></div><div class="pg-pad">
        <div style="display:flex;align-items:baseline;gap:10px"><span class="pg-big" style="color:${left === 0 ? "var(--pg-fg)" : left < 0 ? "#ff9c90" : "var(--pg-amber)"}">${left}</span><span style="font-size:13px;color:var(--pg-fg2)">points left</span></div>
        <div class="pg-meter"><i style="width:${Math.min(100, Math.round((spent / bg.pool) * 100))}%"></i></div>
        <p class="pg-note">A locked strength can't go below your road's value; points from the pool can still raise it to 20. Every rating is 1-20 on the same scale as every coach in the world. Once you're in the job they grow by doing it, and each point costs more the higher it gets; at the start every point costs the same.</p></div></div>
      <div class="pg-panel"><div class="pg-ph"><span>How the world reads you</span></div><div class="pg-pad" style="gap:6px">
        <span style="font-size:16px;font-weight:700">${esc(repWord(bg.reputation))}</span><span style="color:var(--pg-fg2);font-size:13px;line-height:1.5">${esc(read)}</span>
        <span class="pg-note">Nobody sees these numbers but you: the world reads your reputation and what you do.</span></div></div>
      <div class="pg-panel"><div class="pg-ph"><span>Coaches in the world on your road</span></div><div class="pg-like">
        ${like.coaches.length ? like.coaches.map((c) => `<div><span><b>${esc(c.name)}</b>, ${c.age}</span><small>${esc(c.job)} · ${esc(c.came_up)} · ${esc(c.reputation.toLowerCase())}</small></div>`).join("") : '<div><small>Nobody in this world came up quite your way.</small></div>'}</div></div>
    </aside></div>`;
  return setupPage(st, {
    title: "Your ratings",
    lede: "Spread your road's pool over the ratings every coach in the world has; your locked strengths stay at least where the road put them. You see your own numbers; everyone else sees only your reputation.",
    body: pageBody, sum: esc(`Strongest: ${top}`), next: "Next: Style",
    nextOk: left >= 0,
  });
}

// Step 5: your coaching style (tendencies, "Your coaching style").
const STYLE_SETS = [
  { key: "risk", name: "Risk", poles: "Jim Tressel punting from midfield against Bruce Arians's \"no risk it, no biscuit\"",
    labels: ["Very safe", "Conservative", "Balanced", "Aggressive", "Very aggressive"],
    low: "Punts on fourth and two at midfield, plays for field position, returners take a knee.", mid: "Goes for it when the down, distance and score say so.",
    high: "Goes for it on fourth and short in his own half, takes deep shots, returners bring kicks out." },
  { key: "freedom", name: "Structure or freedom", poles: "Bill Belichick's \"do your job\" against the Ravens letting Ed Reed freelance",
    labels: ["Do your job", "Strict", "Earned", "Loose", "Freelance"],
    low: "Every player in his gap and his route. Instructions are followed more often.", mid: "Veterans earn some freedom; young players stay on script.",
    high: "Safeties jump routes, backs bounce runs, quarterbacks scramble. Bigger plays both ways." },
  { key: "physical", name: "Physical or protective", poles: "Pete Carroll's rugby-style shoulder tackling against Jim Harbaugh's physical culture",
    labels: ["Protective", "Careful", "Normal", "Tough", "Physical"],
    low: "Less live contact in practice, sure tackles over big hits, fewer practice injuries.", mid: "The usual contact for your level.",
    high: "Live hitting in practice, players look for the big hit. Tougher teams, more injuries." },
  { key: "ball_hunting", name: "Ball hunting or sound", poles: "A wrap-up defense that keeps everything in front against Lovie Smith's punch-out Bears",
    labels: ["Sound", "Wrap up", "Balanced", "Opportunist", "Ball hunting"],
    low: "Wrap up and keep the play in front. Fewer takeaways, fewer missed tackles.", mid: "Punch at the ball when it is safe to.",
    high: "Strips, jumped routes, hands up at the line. More takeaways and more missed tackles." },
  { key: "mould", name: "Mould or adapt", poles: "A system staff that coaches every habit against Andy Reid building around Patrick Mahomes",
    labels: ["Adapt to him", "Flexible", "Halfway", "System", "Mould him"],
    low: "Builds around what each player already does. Fit counts less on your boards; habits change slowly.", mid: "Meets players halfway.",
    high: "Coaches every habit toward your system, twice as fast. Fit counts more on your boards." },
];
// The five steps of each scale on the 1-20 style scale every coach has.
const STYLE_VALUES = [2, 6, 10, 14, 18];
function pressLabel(p) {
  return p.risk >= 3 && p.ball_hunting >= 3 ? "A gambler on fourth down and on defense"
    : p.freedom <= 1 && p.mould >= 3 ? "A system coach: everyone does his job"
    : p.physical >= 3 ? "A tough, physical program"
    : p.risk <= 1 ? "Plays the percentages, wins the field position game"
    : p.freedom >= 3 ? "A players' coach who lets them play"
    : "Balanced: no label yet";
}
// Who calls your plays (plays and tactics topic 6.1): a head coach calls
// one side himself or hands both to his coordinators. The default is the
// side you came up teaching.
const CALLS = [
  ["none", "Your coordinators", "Your coordinators call both sides. You run the game: the clock, timeouts and fourth downs."],
  ["offense", "The offense", "You call your offense, like Sean McVay or Kyle Shanahan. Your defensive coordinator calls the defense."],
  ["defense", "The defense", "You call your defense, like Mike Macdonald or Sean McDermott. Your offensive coordinator calls the offense."],
  ["both", "Both sides", "You call every snap on both sides."],
];
function callsOf(st) {
  if (st.calls) return st.calls;
  const r = st.ratings || [];
  let top = 0;
  for (let i = 1; i < 8; i++) if ((r[i] || 0) > (r[top] || 0)) top = i;
  return top < 4 ? "offense" : top < 7 ? "defense" : "none";
}
function stepStyle(st) {
  const p = st.style;
  const calls = callsOf(st);
  const callSet = `<div class="pg-set"><div><h3>Who calls the plays</h3><div class="poles">Your play calling and in-game adaptation count on the side you call. At a high school you call both sides.</div></div>
    <div class="pg-scale four" role="group" aria-label="Who calls the plays">${CALLS.map(([k, t]) => `<button class="${calls === k ? "on" : ""}" data-pg="calls" data-k="${k}">${esc(t)}</button>`).join("")}</div>
    <div class="pg-eff">${esc(CALLS.find((c) => c[0] === calls)[2])}</div></div>`;
  const sets = STYLE_SETS.map((s) => `<div class="pg-set"><div><h3>${esc(s.name)}</h3><div class="poles">${esc(s.poles)}</div></div>
    <div class="pg-scale" role="group" aria-label="${esc(s.name)}">${s.labels.map((l, j) => `<button class="${p[s.key] === j ? "on" : ""}" data-pg="style" data-k="${s.key}" data-j="${j}">${esc(l)}</button>`).join("")}</div>
    <div class="pg-eff">${esc(p[s.key] <= 1 ? s.low : p[s.key] >= 3 ? s.high : s.mid)}</div></div>`).join("");
  const body = `<div class="pg-body"><div class="pg-col"><div class="pg-panel">${sets}${callSet}</div></div>
    <aside class="pg-aside">
      <div class="pg-panel"><div class="pg-ph"><span>What the press will call you</span></div><div class="pg-pad" style="gap:6px">
        <span style="font-size:16px;font-weight:700">${esc(pressLabel(p))}</span><span class="pg-note">The media's name for you comes from what you do, not from these settings. This is where your choices point.</span></div></div>
      <div class="pg-panel"><div class="pg-ph"><span>Also shaping your team</span></div><div class="pg-kv">
        <span>Your schemes</span><span>Set in your playbook once you have the job. A timing offense or an attacking front pulls habits as much as one of these settings.</span>
        <span>Coordinators</span><span>Each brings his own style to his side of the ball. The unit lands two-thirds of the way toward yours.</span>
        <span>Tempo, run or pass</span><span>Game plan settings, chosen week by week.</span></div></div>
    </aside></div>`;
  return setupPage(st, {
    title: "Your coaching style",
    lede: "How your team plays and which habits you coach into players. Every setting is a real philosophy with winners at both ends; none is better. Players drift toward your style over a season, faster with a coach who moulds.",
    body, sum: esc([...STYLE_SETS.map((s) => s.labels[p[s.key]]), "calls: " + CALLS.find((c) => c[0] === calls)[1].toLowerCase()].join(" · ")), next: "Next: Job",
  });
}

// Step 6: the job.
const LEVELS = [["nfl", "NFL"], ["college", "College"], ["hs", "High school"], ["other", "Other leagues"]];
const DIVS = ["FBS", "FCS", "D2", "D3", "NAIA", "JUCO"];
const DIV_NAMES = { D2: "Division II", D3: "Division III" };
function jobFilters(st, all) {
  const lv = st.level, f = st.f[lv];
  const count = (pred) => all.filter(pred).length;
  const btn = (key, val, label, n) => `<button class="${f[key] === val ? "on" : ""}" data-pg="jf" data-key="${key}" data-v="${esc(val ?? "")}"><span>${esc(label)}</span>${n != null ? `<small>${n}</small>` : ""}</button>`;
  let html = "";
  if (lv === "college") {
    const divs = DIVS.filter((d) => all.some((j) => j.division === d));
    const inDiv = all.filter((j) => j.division === f.div);
    const confs = [...new Set(inDiv.map((j) => j.conference))].sort();
    html = `<h4>Division</h4>${divs.map((d) => btn("div", d, DIV_NAMES[d] || d, count((j) => j.division === d))).join("")}
      <h4>Conference</h4>${btn("conf", null, "All", inDiv.length)}${confs.map((c) => btn("conf", c, c, inDiv.filter((j) => j.conference === c).length)).join("")}`;
  } else if (lv === "other") {
    // The CFL, arena football and the spring leagues playing this year or next.
    const leagues = [...new Set(all.map((j) => j.league))];
    html = `<h4>League</h4>${btn("league", null, "Every league", all.length)}${leagues.map((l) => btn("league", l, all.find((j) => j.league === l).league_name, count((j) => j.league === l))).join("")}`;
  } else if (lv === "nfl") {
    const divs = [...new Set(all.map((j) => j.division).filter(Boolean))].sort();
    html = `<h4>Conference</h4>${btn("conf", null, "Both", all.length)}${["AFC", "NFC"].map((c) => btn("conf", c, c, count((j) => j.conference === c))).join("")}
      <h4>Division</h4>${btn("div", null, "All", null)}${divs.filter((d) => !f.conf || d.startsWith(f.conf)).map((d) => btn("div", d, d, count((j) => j.division === d))).join("")}`;
  } else {
    const states = [...new Set(all.map((j) => j.state))].sort((a, b) => STATE_NAMES[a] < STATE_NAMES[b] ? -1 : 1);
    const inState = all.filter((j) => !f.state || j.state === f.state);
    // Biggest class first, in each state's own names (6A Division I, Class AA, Group 5).
    const rank = {};
    for (const j of inState) rank[j.class] = Math.min(rank[j.class] ?? 99, j.class_rank ?? 99);
    const classes = Object.keys(rank).sort((a, b) => rank[a] - rank[b] || a.localeCompare(b));
    html = `<h4>State</h4>${btn("state", null, "Every state", all.length)}${states.map((s) => btn("state", s, STATE_NAMES[s] || s, count((j) => j.state === s))).join("")}
      <h4>Class</h4>${btn("cls", null, "All", inState.length)}${classes.map((c) => btn("cls", c, c, inState.filter((j) => j.class === c).length)).join("")}`;
  }
  // An assistant's seat is on a college or NFL staff (lane 10).
  const levels = st.mode === "assistant" ? LEVELS.filter(([k]) => k !== "hs") : LEVELS;
  return `<nav class="pg-panel pg-filters" aria-label="Filters">
    <div class="pg-seg ${levels.length === 2 ? "two" : levels.length === 4 ? "four" : ""}">${levels.map(([k, t]) => `<button class="${lv === k ? "on" : ""}" data-pg="level" data-k="${k}">${t}</button>`).join("")}</div>
    <input class="pg-search" id="pg-q" value="${esc(st.q)}" placeholder="Find a team, town or state" aria-label="Find a team" autocomplete="off">
    <div class="pg-flt" data-keep="flt">${html}</div></nav>`;
}
const JOB_COLS = {
  college: [["name", "Program"], ["record", "2025", "n"], ["prestige", "Prestige"], ["stadium", "Stadium", "n"], ["nil", "NIL budget", "n"], ["coach", "Head coach now"]],
  nfl: [["name", "Club"], ["record", "2025", "n"], ["playoffs", "Playoffs"], ["titles", "Super Bowls", "n"], ["cap_room", "Cap room", "n"], ["coach", "Head coach now"]],
  hs: [["name", "School"], ["record", "2025", "n"], ["prestige", "Prestige"], ["class", "Class"], ["enrollment", "Enrollment", "n"], ["coach", "Head coach now"]],
  other: [["name", "Club"], ["league", "League"], ["record", "Last season", "n"], ["titles", "Titles", "n"], ["pay", "Head coach pay", "n"], ["coach", "Head coach now"]],
};
const winPct = (r) => { const m = String(r || "").match(/^(\d+)-(\d+)/); return m ? Number(m[1]) / Math.max(1, Number(m[1]) + Number(m[2])) : -1; };
function jobSortValue(j, k) {
  if (k === "record") return winPct(j.record);
  if (k === "coach") return j.coach ? j.coach.name : "";
  if (k === "name" || k === "class" || k === "playoffs" || k === "league") return String(j[k] || "");
  return Number(j[k] || 0);
}
function jobCell(j, k) {
  switch (k) {
    case "name": return `<td style="font-weight:600">${pgMark(j)}${esc(j.name)}</td>`;
    case "record": return `<td class="n">${esc(j.record)}</td>`;
    case "prestige": return `<td>${bar(j.prestige)}<span class="pg-dim">${esc(j.prestige_word)}</span></td>`;
    case "stadium": return `<td class="n">${thousands(j.stadium)}</td>`;
    case "nil": return `<td class="n">${j.nil ? money0(j.nil) : '<span class="pg-dim">-</span>'}</td>`;
    case "playoffs": return `<td class="pg-dim">${esc(j.playoffs || "Missed the playoffs")}</td>`;
    case "titles": return `<td class="n">${j.titles || '<span class="pg-dim">-</span>'}</td>`;
    case "cap_room": return `<td class="n">${money(j.cap_room)}</td>`;
    case "league": return `<td>${esc(j.league)}</td>`;
    case "pay": return `<td class="n">${money0(j.pay)}</td>`;
    case "class": return `<td>${esc(j.class)}</td>`;
    case "enrollment": return `<td class="n">${thousands(j.enrollment)}</td>`;
    case "coach": return `<td class="pg-dim">${j.coach ? esc(j.coach.name) : "Open"}</td>`;
  }
  return "<td></td>";
}
function jobFile(st, j) {
  if (!j) return `<aside class="pg-panel pg-jobfile"><div class="pg-empty">Pick a job to read its file: last season, the coach you'd replace, your boss and what he expects.</div></aside>`;
  const c = j.coach;
  const replace = c ? `${c.name}, ${c.record} in ${c.seasons} season${c.seasons === 1 ? "" : "s"}. He goes to the coaching market.` : "Nobody: the job is open.";
  let kv = `<span>2025</span><span>${esc(j.record)}${j.playoffs ? " · " + esc(j.playoffs) : ""}</span>`;
  if (st.level === "college") {
    kv += `<span>Stadium</span><span>${thousands(j.stadium)} seats</span><span>NIL budget</span><span>${j.nil ? money0(j.nil) + " a year" : "-"}</span>
      <span>Titles</span><span>${j.titles ? `${j.titles} national` : "No national titles"}</span>
      <span>You replace</span><span>${esc(replace)}</span><span>Your boss</span><span>${j.ad ? `${esc(j.ad.name)}, athletic director since ${j.ad.since}` : "The athletic director's job is open"}</span>`;
  } else if (st.level === "nfl") {
    kv += `<span>Super Bowls</span><span>${j.titles ? `${j.titles}, the last in ${j.last_title}` : "None yet"}</span><span>Cap room</span><span>${money(j.cap_room)} for ${esc(PG.opts.year)}</span>
      <span>You replace</span><span>${esc(replace)}</span><span>Your boss</span><span>The owner. In the NFL you are head coach and GM; ${esc(j.gm.name)} has run the personnel side since ${j.gm.since}.</span>`;
  } else if (st.level === "other") {
    kv += `<span>League</span><span>${esc(j.league_name)}</span><span>Titles</span><span>${j.titles ? `${j.titles} ${esc(j.trophy || "")}${j.titles === 1 ? "" : "s"}, the last in ${j.last_title}` : "None yet"}</span>
      <span>Pay</span><span>About ${money0(j.pay)} a year for the head coach</span>
      <span>You replace</span><span>${esc(replace)}</span><span>Your boss</span><span>The owner and his general manager. The league office sets the roster rules and the salary cap; your players sign one-year and two-year deals.</span>`;
  } else {
    kv += `<span>Class</span><span>${esc(j.class)} · ${thousands(j.enrollment)} students${j.private ? " · private" : ""}</span><span>State titles</span><span>${j.titles || "None"}</span>
      <span>You replace</span><span>${esc(replace)}</span><span>Your boss</span><span>The principal and the school board; most high school head coaches also teach.</span>`;
  }
  kv += `<span>Expectation</span><span>${esc(j.expectation)}</span><span>The calendar</span><span>${esc(j.calendar)}</span>`;
  const grades = j.grades ? `<div class="pg-grades-h">What recruits see · program grades</div><div class="pg-grades">${j.grades.map(([n, v]) => `<div><span>${esc(n[0].toUpperCase() + n.slice(1))}</span><b>${v}</b></div>`).join("")}</div>` : "";
  return `<aside class="pg-panel pg-jobfile"><div class="pg-filehead">${pgMark(j, true)}<div><b>${esc(j.name)}</b><small>${esc(j.city || j.town ? `${j.city || j.town + ", " + j.state} · ${j.line}` : j.line)}</small></div></div>
    <div class="pg-kv">${kv}</div>${grades}</aside>`;
}
// The three ways in (lane 10): a head job, an assistant's job on a staff,
// or no job yet, hired through the market like any other coach.
const MODES = [
  ["head", "Head coach", "Any head job in the world. The coach you replace goes to the market."],
  ["assistant", "Assistant", "A coordinator or position coach on a college or NFL staff, under a head coach who has the last word."],
  ["free", "No job yet", "Start out of work. Searches call you when your name fits, and teams judge you like any candidate."],
];
function modesBar(st) {
  return `<div class="pg-modes" role="group" aria-label="How you start">${MODES.map(([k, t, d]) => `<button class="pg-mode ${st.mode === k ? "on" : ""}" data-pg="mode" data-k="${k}"><b>${esc(t)}</b><small>${esc(d)}</small></button>`).join("")}</div>`;
}
// The assistant's file: the staff's jobs, who holds each, and your boss.
function assistantFile(st, j) {
  if (!j) return `<aside class="pg-panel pg-jobfile"><div class="pg-empty">Pick a staff to read its jobs: who holds each now, and the head coach you would work for.</div></aside>`;
  const c = j.coach;
  const cur = j.roles.find((r) => r.key === st.role);
  const ctl = !cur ? "" : cur.key === "oc" ? "the offense" : cur.key === "dc" ? "the defense" : cur.title.replace(/ coach$/, "").replace(/ coordinator$/, "").toLowerCase();
  const roles = j.roles.map((r) => `<button class="pg-role ${r.key === st.role ? "on" : ""}" data-pg="role" data-k="${r.key}"><b>${esc(r.title)}</b><small>${r.holder ? esc(r.holder) + " now" : "Open"}</small></button>`).join("");
  return `<aside class="pg-panel pg-jobfile"><div class="pg-filehead">${pgMark(j, true)}<div><b>${esc(j.name)}</b><small>${esc(j.line)}</small></div></div>
    <div class="pg-kv"><span>${PG.opts.year - 1}</span><span>${esc(j.record)}</span><span>Head coach</span><span>${c ? `${esc(c.name)}, ${esc(c.record)} in ${c.seasons} season${c.seasons === 1 ? "" : "s"}` : "The job is open: a search fills it, and the new man decides whether you stay"}</span>
    ${cur ? `<span>You run</span><span>${esc(ctl[0].toUpperCase() + ctl.slice(1))}: the depth chart and the players' plans. ${c ? esc(c.name.split(" ").slice(-1)[0]) : "The head coach"} reads your order against his own and can overrule it.</span><span>You replace</span><span>${cur.holder ? `${esc(cur.holder)}. He goes to the coaching market.` : "Nobody: the job is open."}</span>` : ""}</div>
    <div class="pg-grades-h">The staff's jobs · pick yours</div><div class="pg-roles">${roles}</div></aside>`;
}
// No job yet: the level you want, and how the market works there.
const FREE_TEXT = {
  nfl: ["NFL staffs are set by February. Openings come when coaches leave in the spring and summer; the big round is Black Monday, after the last regular-season Sunday, with head coaches hired in January and their coordinators within weeks.", "An NFL search wants coordinators and head coaches with records. Without one, a college or high school job first is the usual road."],
  college: ["Head coaches are fired from late October; searches run through the bowls into January, and new head coaches hire their staffs from December to February. You start in February, after the big moves: openings come back through the spring as assistants move up or leave.", "A position coach's job is the first rung; a coordinator's needs a record or someone on the staff who knows you. Your agent shops you to FBS and FCS staffs; the Job market screen widens or narrows that any day."],
  hs: ["Principals and boards decide on their coaches after the season, and most openings are posted and filled from January to April, so February is the busy time. You look in your home state first: change that on the Job market screen any day.", "High school jobs pay a teacher's salary plus a stipend; most head coaches teach a subject at the school."],
  other: ["The CFL fires after the Grey Cup in November and hires through the winter; the spring leagues hire in the fall for a season from March. Any seat still empty is filled about three weeks before camp.", "A head coach or coordinator's job in the CFL, arena football or a spring league. NFL and college coaches take them and come back: a good season there puts your name back on NFL lists."],
};
function stepFree(st) {
  const lv = st.freeLevel || "college";
  const [when, rung] = FREE_TEXT[lv];
  const body = `${modesBar(st)}<div class="pg-body"><div class="pg-col">
      <div class="pg-panel"><div class="pg-ph"><span>Where you want to coach</span><small>the jobs you let it be known you want</small></div>
        <div class="pg-pad"><div class="pg-seg" style="justify-self:start">${LEVELS.map(([k, t]) => `<button class="${lv === k ? "on" : ""}" data-pg="free-level" data-k="${k}">${t}</button>`).join("")}</div>
        <p class="pg-note">${esc(when)}</p><p class="pg-note">${esc(rung)}</p></div></div>
      <div class="pg-panel"><div class="pg-ph"><span>How the market works</span></div><div class="pg-story">
        <div><span>Who calls</span><span>A search reads every name: the coaches who put theirs in and the ones known to want that kind of job. It judges you as it judges them, by your record, your reputation, your ratings as people around football see them, and who on the hiring side knows you.</span></div>
        <div><span>Your name in</span><span>On the Job market screen you put your name in for any opening and say which jobs you'd take and where. Your agent and your friends spread the word.</span></div>
        <div><span>Offers</span><span>An offer waits a few days for your answer, a high school one a little longer. Turn it down and the job goes to the next name. While you hold three offers, searches pass you by.</span></div>
        <div><span>Your family</span><span>A job in another state moves your family, or doesn't. A move is easier with young children than with a high school senior.</span></div>
        <div><span>The days</span><span>Continue runs the world on while you wait: every game, signing and hiring goes on without you.</span></div>
      </div></div></div>
    <aside class="pg-aside"><div class="pg-panel" id="pg-known" data-n="10">${PG.knownHtml || ""}</div></aside></div>`;
  return body;
}
async function stepJob(st) {
  if (st.mode === "free") {
    PG.knownHtml = await knownHtml(st, 10);
    return setupPage(st, {
      title: "Your job",
      lede: "Start without a team, the way most careers start between jobs: your name goes on the coaching market, and the searches decide whether to call.",
      body: stepFree(st),
      sum: esc(`No job yet · looking for ${{ nfl: "an NFL job", college: "a college job", hs: "a high school head job", other: "a job in the other leagues" }[st.freeLevel || "college"]}`),
      next: "Start without a job",
    });
  }
  if (st.mode === "assistant" && st.level === "hs") st.level = "college";
  const lv = st.level;
  const assistant = st.mode === "assistant";
  const data = assistant ? (PG.jobs["a:" + lv] ??= await api("assistant_jobs", { level: lv })) : (PG.jobs[lv] ??= await api("jobs", { level: lv }));
  const all = data.jobs;
  const f = st.f[lv];
  if (lv === "hs" && f.state === null && !f.touched) f.state = st.state;
  const q = st.q.trim().toLowerCase();
  const hay = (j) => `${j.name} ${j.sub} ${STATE_NAMES[j.state] || ""} ${j.city || ""} ${j.town || ""}`.toLowerCase();
  let list = all.filter((j) => {
    if (q) return hay(j).includes(q);
    if (lv === "college") return j.division === f.div && (!f.conf || j.conference === f.conf);
    if (lv === "nfl") return (!f.conf || j.conference === f.conf) && (!f.div || j.division === f.div);
    if (lv === "other") return !f.league || j.league === f.league;
    return (!f.state || j.state === f.state) && (!f.cls || j.class === f.cls);
  });
  if (st.sort.k) {
    const k = st.sort.k, dir = st.sort.desc ? -1 : 1;
    list = list.slice().sort((a, b) => { const x = jobSortValue(a, k), y = jobSortValue(b, k); return (x < y ? -1 : x > y ? 1 : 0) * dir; });
  }
  const shown = list.slice(0, 400);
  const cols = JOB_COLS[lv];
  const cur = st.job && st.job.kind === lv ? all.find((j) => j.ref.id === st.job.id) : null;
  const what = q ? `${list.length} match "${esc(st.q.trim())}"` : lv === "college" ? `${esc(f.conf || DIV_NAMES[f.div] || f.div)}` : lv === "nfl" ? esc(f.div || f.conf || "The NFL") : lv === "other" ? esc(f.league ? all.find((j) => j.league === f.league).league_name : "The other leagues") : esc(f.state ? `${STATE_NAMES[f.state] || f.state}${f.cls ? " " + f.cls : ""}` : "Every state");
  const sortedBy = st.sort.k ? (cols.find((c) => c[0] === st.sort.k) || [, ""])[1].toLowerCase() : "prestige";
  const kind = assistant ? `staff${list.length === 1 ? "" : "s"}` : `head coaching job${list.length === 1 ? "" : "s"}`;
  const head = (k, t) => (assistant && k === "coach" ? "Head coach" : t);
  const table = `<div class="pg-panel pg-jobtable"><div class="pg-ph"><span>${what} <small>· ${list.length} ${kind} · sorted by ${esc(sortedBy)}${list.length > shown.length ? ` · showing the first ${shown.length}: narrow it with the filters or a search` : ""}</small></span></div>
    ${shown.length ? `<div class="pg-tbl dense" data-keep="jobs"><table><thead><tr>${cols.map(([k, t, n]) => `<th class="${n || ""} ${st.sort.k === k ? "sorted" : ""}" data-pg="jsort" data-k="${k}">${head(k, t)}${st.sort.k === k ? (st.sort.desc ? " ▾" : " ▴") : ""}</th>`).join("")}</tr></thead>
    <tbody>${shown.map((j) => `<tr data-pg="pick-job" data-kind="${j.ref.kind}" data-id="${j.ref.id}" class="${cur && cur.ref.id === j.ref.id ? "sel" : ""}">${cols.map(([k]) => jobCell(j, k)).join("")}</tr>`).join("")}</tbody></table></div>`
      : `<div class="pg-empty">No job matches. Try a team, a town, a state or its code (Texas or TX).</div>`}</div>`;
  const role = assistant && cur ? cur.roles.find((r) => r.key === st.role) : null;
  const body = `${modesBar(st)}<div class="pg-jobs">${jobFilters(st, all)}${table}${assistant ? assistantFile(st, cur) : jobFile(st, cur)}</div>`;
  return setupPage(st, {
    title: "Your job",
    lede: assistant
      ? "Every coordinator's and position coach's job on a college or NFL staff is yours to take. You run what your job runs; the head coach runs the rest and has the last word."
      : "Every head coaching job in the world is open. The coach you replace goes to the coaching market, and his staff stays under contract until you decide.",
    body,
    sum: assistant
      ? cur ? esc(role ? `${role.title} · ${cur.name}${cur.coach ? " · under " + cur.coach.name : ""}` : `${cur.name} · pick a job on the staff`) : "Pick a staff, then your job on it."
      : cur ? esc(`${cur.name} · head coach · ${cur.expectation}`) : "Pick a job to take it.",
    next: "Take the job", nextOk: assistant ? !!role : !!cur,
  });
}

// ---------- actions ----------
async function openWorldFor(st) {
  if (st.world === "test") return true;
  if (st.loaded === st.world && S.status && !S.status.you) return true;
  if ($("#busy") || !(await leaveCareer())) return false;
  showBusy("Opening the world… a full world takes about a minute.");
  try {
    S.status = await api("open_world", { path: st.world });
    // The team picker shows an installed pack's logos (screens/logos.js).
    if (typeof logosLoad === "function") await logosLoad();
    hideBusy();
    S.savedOn = null;
    st.loaded = st.world;
    PG.loaded = st.world;
    PG.opts = null; PG.jobs = {}; PG.towns = {}; PG.similar = {}; PG.known = null;
    PG.lib = null;
    // The closed career's screens are gone: Back can't lead to them.
    S.back = []; S.fwd = [];
    return true;
  } catch (e) { hideBusy(); toast(e.message); return false; }
}
function toStep(st, k) {
  st.step = k;
  st.reached = Math.max(st.reached, k);
  render();
}
function readPhoto(file) {
  return new Promise((done, fail) => {
    const r = new FileReader();
    r.onerror = () => fail(new Error("Could not read that picture."));
    r.onload = () => {
      const img = new Image();
      img.onerror = () => fail(new Error("That file isn't a picture AFM can read."));
      img.onload = () => {
        // A 4:5 portrait, cropped from the middle, about 25 KB.
        const W = 320, H = 400, c = document.createElement("canvas");
        c.width = W; c.height = H;
        const k = Math.max(W / img.width, H / img.height);
        const w = img.width * k, h = img.height * k;
        c.getContext("2d").drawImage(img, (W - w) / 2, (H - h) / 3, w, h);
        done(c.toDataURL("image/jpeg", 0.85));
      };
      img.src = r.result;
    };
    r.readAsDataURL(file);
  });
}
function aiPrompt(st) {
  const age = ageOn(st.born, PG.opts && PG.opts.today) || 42;
  const her = (HERITAGE.find((h) => h[0] === st.eth) || [, ""])[1].toLowerCase();
  return `Professional head-and-shoulders portrait photograph of a ${age}-year-old ${her} American football head coach, ${st.height.replace(" ft ", "'")}" tall, ${st.build.toLowerCase()} build, ${st.hair.toLowerCase()} hair, ${st.facial_hair.toLowerCase()}. He wears a plain team quarter-zip and stands on a practice field on media day. Soft natural daylight, 85 mm lens, f/2.8, realistic skin texture, a calm, confident expression with a slight smile. No logos, no text, no watermark. Portrait 4:5.`;
}
async function copyText(text) {
  try { await navigator.clipboard.writeText(text); return true; } catch (e) { /* the old way below */ }
  const t = document.createElement("textarea");
  t.value = text; document.body.appendChild(t); t.select();
  let ok = false;
  try { ok = document.execCommand("copy"); } catch (e) { ok = false; }
  t.remove();
  return ok;
}
function startArgs(st) {
  const o = PG.opts;
  const style = {};
  for (const s of STYLE_SETS) style[s.key] = STYLE_VALUES[st.style[s.key]];
  return {
    name: fullName(st), age: ageOn(st.born, o.today) || 42, home_state: st.state, ethnicity: st.eth, background: st.bg,
    position: rdPick(st, o.backgrounds.find((b) => b.key === st.bg) || o.backgrounds[0]),
    ratings: st.ratings, style, job: st.job, calls: callsOf(st), coach: coachRecord(st),
    profile: {
      born: st.born, hometown: st.hometown, high_school: st.high_school, alma_mater: st.alma_mater, college_years: st.college_years, degree: st.degree,
      status: st.status, spouse: st.status === "Married" ? st.spouse : "", roots: st.roots, children: st.children.filter((c) => c.name.trim()),
      height: st.height, build: st.build, hair: st.hair, facial_hair: st.facial_hair, photo: st.photo,
    },
  };
}

document.addEventListener("click", async (ev) => {
  const a = ev.target.closest("[data-pg]");
  if (!a || a.disabled) return;
  const d = a.dataset;
  const st = S.route.screen === "setup" ? setupState() : null;
  switch (d.pg) {
    case "home": go("start", {}, false); break;
    case "continue": {
      if (S.status && S.status.you) { enterCareer(); break; }
      const last = PG.lib && PG.lib.saves[0];
      if (last) openSave(last.path);
      break;
    }
    case "new": {
      if (S.status && S.status.you && !(await leaveCareer())) break;
      const was = S.keep.setup;
      const same = was && !(S.status && S.status.you) && (was.loaded ? was.loaded === PG.loaded : !PG.loaded);
      if (!same) S.keep.setup = newSetup();
      go("setup", {}, false);
      break;
    }
    case "load": go("load", {}, false); break;
    case "settings": {
      const h = PG.hello || {};
      const lib = PG.lib || (await api("library"));
      const { f, answer } = dialog(`<b>Settings</b><div class="pg-kv" style="padding:0"><span>Version</span><span>${esc(h.version || "")}</span><span>Game folder</span><span style="word-break:break-all">${esc(lib.home)}</span><span>Worlds</span><span>${lib.worlds.length}</span><span>Saves</span><span>${lib.saves.length}</span></div>
        <div style="display:flex;gap:8px;flex-wrap:wrap">${isMac() ? '<button type="button" class="pg-btn s" data-pg="reveal-home">Show the game folder</button>' : ""}${isMac() && window.__TAURI__.updater ? '<button type="button" class="pg-btn s" data-pg="update-now">Check for updates</button>' : ""}</div>`, "Close", () => true);
      f.querySelector("[data-dlg=no]").remove();
      await answer;
      break;
    }
    case "reveal-home": api("reveal", { path: (PG.lib || (await api("library"))).home }).catch((e) => toast(e.message)); break;
    case "update-now": toast("Looking for a newer version…"); checkUpdate().then((r) => { if (r === false) toast("The update page can't be reached right now."); else if (!updating) toast("AFM is up to date."); }); break;
    case "quit": {
      if (!(await leaveCareer())) break;
      const j = await api("job").catch(() => null);
      if (j && !j.finished && !(await sure(`Quit while ${j.kind === "build" ? "building" : "downloading"} ${j.name}?`, "Quitting stops it, and it starts again from the beginning next time.", "Quit anyway"))) break;
      try { await window.__TAURI__.process.exit(0); } catch (e) { toast("Quit with ⌘Q."); }
      break;
    }
    // Load game.
    case "pick-save": S.keep.load.sel = d.path; render(); break;
    case "load-save": if (S.keep.load && S.keep.load.sel) openSave(S.keep.load.sel); break;
    case "open-save-file": {
      const path = isMac() && window.__TAURI__.dialog
        ? await window.__TAURI__.dialog.open({ multiple: false, filters: [{ name: "AFM save", extensions: ["afmsave"] }] })
        : await ask("Open a save file", { placeholder: "/Users/you/Downloads/my career.afmsave", ok: "Open" });
      if (path) openSave(path);
      break;
    }
    case "reveal-saves": api("reveal", { path: S.keep.load && S.keep.load.sel }).catch((e) => toast(e.message)); break;
    case "rename-save": {
      const sel = S.keep.load && S.keep.load.sel;
      if (!sel) break;
      const old = sel.split(/[\\/]/).pop().replace(/\.afmsave$/i, "");
      const name = await ask("Rename the save", { value: old, ok: "Rename" });
      if (!name || name === old) break;
      try { const r = await api("rename_save", { path: sel, name }); S.keep.load.sel = r.path; render(); } catch (e) { toast(e.message); }
      break;
    }
    case "delete-save": {
      const sel = S.keep.load && S.keep.load.sel;
      if (!sel) break;
      const ok = await sure(`Delete ${sel.split(/[\\/]/).pop()}?`, "The save file is removed from your saves folder. This can't be undone.", "Delete it");
      if (!ok) break;
      try { await api("delete_save", { path: sel }); S.keep.load.sel = null; render(); } catch (e) { toast(e.message); }
      break;
    }
    // The new career.
    case "step": if (st) { const k = Number(d.k); if (k > 1 && !(await openWorldFor(st))) break; toStep(st, k); } break;
    case "next": {
      if (!st) break;
      if (st.step === 1) { if (!st.world || !(await openWorldFor(st))) break; }
      if (st.step === 2 && (!st.first.trim() || !st.last.trim())) { st.tried = true; toast("Give your first and last name first."); render(); break; }
      toStep(st, st.step + 1);
      break;
    }
    case "pick-world": st.world = d.path; render(); break;
    case "import": {
      const path = isMac() && window.__TAURI__.dialog
        ? await window.__TAURI__.dialog.open(d.file ? { multiple: false, filters: [{ name: "AFM world", extensions: ["afmworld", "001"] }] } : { directory: true, multiple: false })
        : await ask("Import a world", { placeholder: "/Users/you/Downloads/world-2026-seed2026", ok: "Import", note: "A world folder (whole or in parts) or an .afmworld file. It is copied into your library." });
      if (!path) break;
      showBusy("Adding the world to your library…");
      try { const r = await api("import_world", { path }); hideBusy(); st.world = r.path; toast("Added to your world library."); render(); } catch (e) { hideBusy(); toast(e.message); }
      break;
    }
    case "build": {
      const seed = Math.floor(Math.random() * 90000) + 10000;
      const { f, answer } = dialog(`<b>Build a new world</b><small class="sub">A full world is a whole football country with its history, kept in your library: it takes 2-3 hours and up to about 6.5 GB of memory at its peak, and you can keep using AFM. A test world is small and quick, for trying the game, and isn't kept.</small>
        <div class="pg-grid two" style="padding:0"><div class="pg-f"><label>Kind</label><select name="kind"><option value="full">A full world</option><option value="test">A test world (about 2 minutes)</option></select></div>
        <div class="pg-f"><label>Seed</label><input name="seed" value="${seed}"></div><div class="pg-f"><label>Years of history (full)</label><input name="years" value="30"></div>
        <div class="pg-f"><label>Size (test)</label><select name="scale"><option value="0.03">3%</option><option value="0.1">10%</option></select></div>
        <div class="pg-f" style="grid-column:1/-1"><label>Names</label><select name="names"><option value="real">Real names: the real clubs, colleges and conferences</option><option value="fictional">Fictional names: made-up colleges, conferences and NFL clubs in the real cities</option></select></div></div>`, "Build", (f) => ({ kind: f.elements.kind.value, seed: Number(f.elements.seed.value) || seed, years: Number(f.elements.years.value) || 30, scale: Number(f.elements.scale.value) || 0.03, names: f.elements.names.value }));
      f.elements.kind.focus();
      const v = await answer;
      if (!v) break;
      if (v.kind === "full") {
        try { await api("build_world_file", { seed: v.seed, history_years: Math.max(1, Math.min(60, v.years)), names: v.names }); render(); } catch (e) { toast(e.message); }
        break;
      }
      if ($("#busy") || !(await leaveCareer())) break;
      showBusy("Building a test world… a small one takes a couple of minutes.");
      try {
        await api("build_world", { seed: v.seed, scale: v.scale, history_years: 2, engine_from: 2026, names: v.names });
        S.status = await api("status");
        hideBusy();
        S.savedOn = null;
        st.world = "test"; st.loaded = "test"; PG.loaded = "test";
        S.back = []; S.fwd = [];
        PG.opts = null; PG.jobs = {}; PG.towns = {}; PG.similar = {}; PG.known = null;
        render();
      } catch (e) { hideBusy(); toast(e.message); }
      break;
    }
    case "download": try { await api("download_world", { tag: d.tag }); render(); } catch (e) { toast(e.message); } break;
    case "jobstop": try { const j = await api("job_stop"); const el = $("#pg-job"); if (el && j) el.outerHTML = buildPanel(j); } catch (e) { toast(e.message); } break;
    case "kid-add": st.children.push({ name: "", born: PG.opts.year - 8 }); render(); break;
    case "kid-del": st.children.splice(Number(d.i), 1); render(); break;
    case "photo": $("#pg-photo-file").click(); break;
    case "photo-del": st.photo = null; render(); break;
    case "prompt": {
      const text = aiPrompt(st);
      if (await copyText(text)) toast("Copied the portrait prompt: paste it into any image tool.");
      else await ask("Your portrait prompt", { value: text, long: true, ok: "Done", note: "Copy it from here." });
      break;
    }
    case "coach-use": case "coach-edit": if (st) await coachUse(st, d.file, d.pg === "coach-use"); break;
    case "coach-forget": {
      if (!(await sure("Forget this coach?", "His file goes from the game folder. Careers that started with him keep him.", "Forget him"))) break;
      try { PG.coaches = await api("coach_profile_forget", { file: d.file }); render(); } catch (e) { toast(e.message); }
      break;
    }
    case "pick-bg": st.bg = d.k; render(); break;
    case "rd-pos": (st.pos ??= {})[st.bg] = d.k; render(); break;
    case "rd-level": st.rdLevel = d.k; if (!st.job) st.level = d.k; render(); break;
    case "rate": {
      const bg = bgRatings(st);
      const i = Number(d.i), step = Number(d.d);
      const left = bg.pool - st.ratings.reduce((x, y) => x + y, 0);
      const v = st.ratings[i] + step;
      if (v < st.floor[i] || v > 20 || (step > 0 && left <= 0)) break;
      st.ratings[i] = v;
      render();
      break;
    }
    case "rate-reset": st.ratingsFor = null; render(); break;
    case "style": st.style[d.k] = Number(d.j); render(); break;
    case "level": st.level = d.k; st.sort = { k: null, desc: true }; render(); break;
    case "jf": {
      const f = st.f[st.level];
      f[d.key] = d.v || null;
      f.touched = true;
      if (d.key === "div" && st.level === "college") f.conf = null;
      if (d.key === "conf" && st.level === "nfl") f.div = null;
      if (d.key === "state") f.cls = null;
      st.q = "";
      render();
      break;
    }
    case "jsort": {
      const k = d.k;
      st.sort = st.sort.k === k ? { k, desc: !st.sort.desc } : { k, desc: !["name", "coach", "class"].includes(k) };
      render();
      break;
    }
    case "pick-job": st.job = { kind: d.kind, id: Number(d.id) }; render(); break;
    case "calls": st.calls = d.k; render(); break;
    // Lane 10: the seat you start in, and your last stops.
    case "mode": st.mode = d.k; st.sort = { k: null, desc: true }; render(); break;
    case "role": st.role = d.k; render(); break;
    case "free-level": st.freeLevel = d.k; render(); break;
    case "stop-add": if (st.stops.length < MAX_STOPS) { st.stops.push(newStop(st)); render(); } break;
    case "stop-del": st.stops.splice(Number(d.i), 1); render(); break;
    case "take": {
      if (st.mode !== "free" && !st.job) break;
      if (st.mode === "assistant" && !st.role) break;
      const bg = bgRatings(st);
      const spent = st.ratings ? st.ratings.reduce((x, y) => x + y, 0) : 0;
      if (spent > bg.pool) { toast(`${spent - bg.pool} rating points over: take some back first.`); toStep(st, 4); break; }
      if ($("#busy")) break;
      showBusy(st.mode === "free" ? "Putting your name about…" : "Taking the job…");
      try {
        // Every seat starts through the front office's setup (lane 10):
        // the people who know you and your last stops come with you.
        const job = st.mode === "free" ? {} : { job: st.job };
        S.status = await api("start_career_seat", { ...startArgs(st), ...job, start: st.mode, role: st.role, level: st.freeLevel, stops: stopArgs(st) });
        hideBusy();
        S.savedOn = null;
        S.lastSaveName = null;
        S.keep = {};
        PG.opts = null; PG.jobs = {}; PG.loaded = null; PG.known = null; PG.coaches = null;
        enterCareer();
      } catch (e) { hideBusy(); toast(e.message); }
      break;
    }
  }
});

// Typing on the You step keeps what you type without drawing the page again.
document.addEventListener("input", (ev) => {
  const el = ev.target;
  if (!outside(S.route) || S.route.screen !== "setup") return;
  const st = setupState();
  if (el.dataset.f) {
    st[el.dataset.f] = el.value;
    if (el.dataset.f === "born") { const a = ageOn(el.value, PG.opts && PG.opts.today); const t = $("#pg-age"); if (t) t.textContent = a != null ? `${a} years old` : ""; }
    const s = $("#pg-yousum");
    if (s) s.innerHTML = youSummary(st);
  } else if (el.dataset.kid !== undefined) {
    const c = st.children[Number(el.dataset.kid)];
    if (!c) return;
    if (el.dataset.kf === "name") c.name = el.value;
    else { const a = Math.max(0, Math.min(40, Number(el.value) || 0)); c.born = PG.opts.year - a; el.nextElementSibling.textContent = schoolYear(a); }
  } else if (el.id === "pg-q") {
    st.q = el.value;
    clearTimeout(PG.qt);
    PG.qt = setTimeout(async () => { await render(); const i = $("#pg-q"); if (i) { i.focus(); i.setSelectionRange(i.value.length, i.value.length); } }, 250);
  } else if (el.dataset.stop !== undefined && ["name", "from", "to"].includes(el.dataset.sf)) {
    // A stop keeps what you type; it is looked up when you leave the box.
    const s = st.stops[Number(el.dataset.stop)];
    if (s) s[el.dataset.sf] = el.value;
  }
});
document.addEventListener("change", async (ev) => {
  const el = ev.target;
  if (!outside(S.route) || S.route.screen !== "setup") return;
  const st = setupState();
  if (el.dataset.f === "hometown") {
    const town = await hometownOf(st);
    const list = $("#pg-schools"), hint = $("#pg-hshint");
    if (list) list.innerHTML = (town ? town.schools : []).map((s) => `<option value="${esc(s)}">`).join("");
    if (hint) hint.innerHTML = schoolsHint(town);
  }
  if (["state", "status"].includes(el.dataset.f)) {
    if (el.dataset.f === "state") { st.high_school = ""; st.f.hs.state = null; st.f.hs.touched = false; }
    render();
  }
  // Your schools and years decide who knows you (lane 10).
  if (["high_school", "alma_mater", "college_years", "born"].includes(el.dataset.f)) refreshKnown();
  if (el.dataset.stop !== undefined) {
    const s = st.stops[Number(el.dataset.stop)];
    if (!s) return;
    const k = el.dataset.sf;
    s[k] = el.value;
    if (k === "kind") {
      s.name = ""; s.id = null;
      if (s.role === "played" && s.kind !== "nfl") s.role = "pc0";
    }
    if (k === "from" || k === "to") {
      const y = PG.opts.year, v = Math.max(1970, Math.min(y, Math.round(Number(s[k]) || y - 1)));
      s[k] = v;
      if (Number(s.from) > Number(s.to)) [s.from, s.to] = [s.to, s.from];
    }
    if (k === "name") await findStop(s);
    render();
  }
  if (el.id === "pg-photo-file" && el.files && el.files[0]) {
    try { st.photo = await readPhoto(el.files[0]); render(); } catch (e) { toast(e.message); }
  }
});
