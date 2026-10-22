// AFM's shell (section 8 topic 5, section 10 topics 1-5 and 17): one page,
// a frosted sidebar, a top bar with the clickable path you took, search,
// the date and Continue, the pinned must-answer items under it, and the
// screen. Back and forward work like a browser (buttons, ⌘[ ⌘], the
// mouse's back button and a two-finger swipe); each screen keeps its tab,
// sort, filters and scroll when you come back to it.
//
// The shared components every screen uses (tables, rich text with links,
// hover cards, the page beside, right-click actions, page heads with their
// "Go to" line, tabs, the letter, charts, reveals) are in screens/core.js,
// documented at its top. This file is the frame and the router.
"use strict";

// ---------- talking to the game ----------
async function api(cmd, args = {}) {
  if (window.__TAURI__ && window.__TAURI__.core) {
    // The Mac app rejects with the game's message as a plain string.
    try { return await window.__TAURI__.core.invoke("call", { cmd, args }); } catch (e) { throw new Error(plain(e && e.message ? e.message : String(e))); }
  }
  const r = await fetch("/api/" + cmd, { method: "POST", body: JSON.stringify(args) });
  const j = await r.json();
  if (j.err !== undefined) throw new Error(plain(j.err));
  return j.ok;
}
// The system's file errors in plain words.
function plain(m) {
  return String(m)
    .replace(/No such file or directory \(os error 2\)/, "there is no file or folder at that path")
    .replace(/Permission denied \(os error 13\)/, "AFM isn't allowed to read it (check the file's permissions)")
    .replace(/No space left on device \(os error 28\)/, "the disk is full");
}

// ---------- small helpers ----------
const $ = (s, el = document) => el.querySelector(s);
const esc = (s) => String(s ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
const money = (v) => (v == null ? "" : (v < 0 ? "-" : "") + (Math.abs(v) >= 1e6 ? "$" + (Math.abs(v) / 1e6).toFixed(1) + "M" : "$" + Math.round(Math.abs(v) / 1e3) + "k"));
function toast(text) {
  const t = document.createElement("div");
  t.className = "st-toast";
  // A name ending "Jr." closes its own sentence.
  t.textContent = String(text).replace(/\b(Jr|Sr)\.\./g, "$1.");
  $("#toasts").appendChild(t);
  setTimeout(() => t.remove(), 4200);
}
function setColors(c) {
  if (!c) return;
  // A page drawn in the panel beside the screen never re-lights the screen.
  if (S.besideDrawing) return;
  document.documentElement.style.setProperty("--accent", c[0]);
  document.documentElement.style.setProperty("--accent-2", c[1] || c[0]);
}
function initials(name) {
  return String(name || "").split(/\s+/).filter((w) => /^[A-Z]/.test(w)).slice(0, 3).map((w) => w[0]).join("");
}
function mark(team, size) {
  const c = team && team.colors ? team.colors : ["#444", "#222"];
  return `<span class="st-mark" style="background:${c[0]};${size ? `--s:${size}px;` : ""}box-shadow:inset 0 0 0 2px ${c[1]}55">${esc(initials(team && team.name))}</span>`;
}
function teamLink(t, text) {
  if (!t) return "";
  return `<a class="link" data-go="team" data-kind="${t.ref.kind}" data-id="${t.ref.id}">${esc(text ?? t.name)}</a>`;
}
function playerLink(id, name, tab) {
  return `<a class="link name" data-go="player" data-id="${id}"${tab ? ` data-tab="${esc(tab)}"` : ""}>${esc(name)}</a>`;
}
function coachLink(id, name) {
  return id == null ? esc(name) : `<a class="link name" data-go="coach" data-id="${id}">${esc(name)}</a>`;
}
// The rating colour: white low, red high (screens topic 4).
function rateColor(v) {
  const t = Math.max(0, Math.min(1, (v - 35) / 55));
  const a = [244, 239, 233], b = [200, 16, 46];
  const c = a.map((x, i) => Math.round(x + (b[i] - x) * t));
  return `rgb(${c[0]},${c[1]},${c[2]})`;
}
// A staff read as a bar spanning low to high, with the numbers (topic 4).
// `who` names whose read it is for the hover.
function range(r, who) {
  if (!r) return `<span class="rg ns" title="Nobody on your staff has seen this"><span class="t"></span><b>Not seen</b></span>`;
  const [lo, hi] = r;
  const m = (lo + hi) / 2;
  const tip = `Your staff's read: ${lo} to ${hi}${who ? ` · ${who}` : ""}. A narrow bar is a sure read; a wide one says your staff isn't sure.`;
  return `<span class="rg" title="${esc(tip)}"><span class="t"><i style="left:${lo}%;width:${Math.max(2, hi - lo)}%;background:${rateColor(m)}"></i></span><b>${lo}-${hi}</b></span>`;
}
// "minorInjury" -> "minor injury"
const words = (s) => String(s).replace(/([a-z])([A-Z])/g, "$1 $2").toLowerCase();
const mid = (r) => (r ? (r[0] + r[1]) / 2 : -1);
// 6'4 5/8" -> 76.625, for sorting by height.
function inches(t) {
  const m = String(t || "").match(/(\d+)'\s*(\d+)?(?:\s+(\d+)\/(\d+))?/);
  return m ? Number(m[1]) * 12 + Number(m[2] || 0) + (m[3] ? Number(m[3]) / Number(m[4]) : 0) : null;
}
// Fr < So < Jr < Sr (a redshirt half a year on); Rookie < 1 yrs < 2 yrs.
function classRank(c) {
  const t = String(c || "");
  const k = { Fr: 1, So: 2, Jr: 3, Sr: 4 }[t.replace("RS ", "")];
  if (k) return k + (t.startsWith("RS ") ? 0.5 : 0);
  if (t === "Rookie") return 0;
  const y = t.match(/^(\d+) yrs?$/);
  return y ? Number(y[1]) : null;
}
// A number for a table cell: whole numbers as they are, shares of a tackle
// or a sack to one decimal.
const num = (v) => (typeof v === "number" && !Number.isInteger(v) ? v.toFixed(1) : v);

// ---------- per-viewer settings (topic 15): kept in this browser ----------
const prefs = (() => {
  let p = {};
  try { p = JSON.parse(localStorage.getItem("afm.prefs") || "{}") || {}; } catch (e) { p = {}; }
  return p;
})();
function savePrefs() {
  try { localStorage.setItem("afm.prefs", JSON.stringify(prefs)); } catch (e) { /* private window: kept for this session */ }
}
function applyPrefs() {
  const h = document.documentElement;
  h.dataset.theme = prefs.theme === "light" ? "light" : "";
  const z = Math.max(0.9, Math.min(1.3, Number(prefs.textSize) || 1));
  h.style.setProperty("--zoom", String(z));
  document.body && (document.body.style.zoom = z === 1 ? "" : String(z));
  const reduce = prefs.motion === "reduce" || (prefs.motion !== "full" && window.matchMedia && matchMedia("(prefers-reduced-motion: reduce)").matches);
  h.classList.toggle("calm", !!reduce);
}

// ---------- state ----------
const S = {
  status: null,
  route: { screen: "start", args: {} },
  back: [],
  fwd: [],
  keep: {}, // per-screen sort, filters, tabs
  busy: false,
  notesOpen: false,
  // The game day of your last save or load this session (null: never saved).
  savedOn: null,
  // Each render's number: a slow screen that finishes after a newer one
  // never draws over it.
  drawn: 0,
  // What the screen being drawn calls itself, for the path (L7).
  title: null,
  // The page shown beside the screen (L6): {screen, args} or null.
  beside: null,
  besideDrawing: false,
};
const keep = (k, d) => (S.keep[k] ??= d);

// The opening, load and new-career pages sit outside a career: back and forward move
// between the career's screens only, so they never lead out of it.
const outside = (r) => r.screen === "start" || r.screen === "setup" || r.screen === "load";
// go(screen, args, push) keeps its old third argument (false: don't add to
// history); an object can say {root: true} for a sidebar click, which
// starts a new path in the breadcrumb.
function go(screen, args = {}, opt = true) {
  const o = typeof opt === "object" ? opt : { push: opt };
  const push = o.push !== false;
  if (push && !(outside(S.route) && !outside({ screen }))) {
    S.back.push({ ...S.route, scroll: $(".view") ? $(".view").scrollTop : 0 });
    if (S.back.length > 120) S.back.shift();
    S.fwd = [];
  }
  S.route = { screen, args, root: !!o.root };
  render();
}
// Into a career (taken, loaded or resumed): its history starts here.
function enterCareer() {
  S.back = [];
  S.fwd = [];
  go("home", {}, { push: false, root: true });
}
function goBack() {
  if (!S.back.length) return;
  S.fwd.push({ ...S.route, scroll: $(".view") ? $(".view").scrollTop : 0 });
  S.route = S.back.pop();
  render(S.route.scroll);
}
function goFwd() {
  if (!S.fwd.length) return;
  S.back.push({ ...S.route, scroll: $(".view") ? $(".view").scrollTop : 0 });
  S.route = S.fwd.pop();
  render(S.route.scroll);
}
// Back several steps at once (a breadcrumb click).
function goBackTo(n) {
  if (n <= 0) return;
  let r = S.route;
  for (let i = 0; i < n && S.back.length; i++) {
    S.fwd.push({ ...r, scroll: i === 0 && $(".view") ? $(".view").scrollTop : r.scroll });
    r = S.back.pop();
  }
  S.route = r;
  render(r.scroll);
}
// A screen names itself for the path: title("Jalen Carter", "Development").
function title(t, sub) {
  S.title = sub ? `${t} · ${sub}` : t;
}

// ---------- icons ----------
const I = {
  home: '<path d="M3 11l9-8 9 8v9a1 1 0 0 1-1 1h-5v-6h-6v6H4a1 1 0 0 1-1-1z"/>',
  inbox: '<path d="M3 13l3-8h12l3 8v6a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1z M3 13h5l1 3h6l1-3h5"/>',
  cal: '<rect x="3" y="5" width="18" height="16" rx="2"/><path d="M3 10h18M8 3v4M16 3v4"/>',
  squad: '<circle cx="9" cy="8" r="3.5"/><path d="M2.5 20c.8-4 3.4-6 6.5-6s5.7 2 6.5 6"/><circle cx="17" cy="9" r="2.6"/><path d="M16 14c2.8 0 4.8 1.6 5.5 5"/>',
  depth: '<path d="M4 6h16M4 12h11M4 18h7"/>',
  sched: '<path d="M4 4h16v16H4z M4 9h16 M9 4v16"/>',
  table: '<path d="M4 5h16M4 10h16M4 15h16M4 20h16"/>',
  staff: '<circle cx="12" cy="7" r="3.5"/><path d="M5 21c.8-4.5 3.6-7 7-7s6.2 2.5 7 7"/>',
  game: '<ellipse cx="12" cy="12" rx="9" ry="5.5" transform="rotate(-35 12 12)"/><path d="M9.5 14.5l5-5M10.5 11l2.5 2.5"/>',
  news: '<path d="M4 5h13v14H5a1 1 0 0 1-1-1z M17 8h3v10a1 1 0 0 1-1 1h-2 M7 9h7M7 13h7M7 16h4"/>',
  you: '<circle cx="12" cy="8" r="4"/><path d="M4 21c1-5 4.2-7.5 8-7.5s7 2.5 8 7.5"/>',
  save: '<path d="M5 3h11l3 3v15H5z M8 3v6h8V3 M8 21v-7h8v7"/>',
  bug: '<rect x="7" y="7" width="10" height="13" rx="5"/><path d="M12 7V4M9 4.5l1.5 2.5M15 4.5L13.5 7M4 12h3M17 12h3M4 18l3-1.5M20 18l-3-1.5"/>',
  back: '<path d="M15 5l-7 7 7 7"/>',
  fwd: '<path d="M9 5l7 7-7 7"/>',
  team: '<path d="M12 3l8 4v5c0 5-3.5 8-8 9-4.5-1-8-4-8-9V7z"/>',
  recruit: '<path d="M4 20V6l8-3 8 3v14M9 20v-6h6v6M8 9h.01M16 9h.01"/>',
  money: '<rect x="3" y="6" width="18" height="12" rx="2"/><circle cx="12" cy="12" r="2.5"/><path d="M6 9v6M18 9v6"/>',
  train: '<path d="M3 10v4M6 7v10M18 7v10M21 10v4M6 12h12"/>',
  menu: '<path d="M4 6h16M4 12h16M4 18h16"/>',
  theme: '<circle cx="12" cy="12" r="8"/><path d="M12 4a8 8 0 0 0 0 16z" fill="currentColor"/>',
  star: '<path d="M12 3.5l2.6 5.3 5.9.9-4.3 4.1 1 5.8-5.2-2.7-5.2 2.7 1-5.8-4.3-4.1 5.9-.9z"/>',
  list: '<path d="M8 6h12M8 12h12M8 18h12"/><circle cx="4" cy="6" r="1"/><circle cx="4" cy="12" r="1"/><circle cx="4" cy="18" r="1"/>',
  trophy: '<path d="M8 4h8v5a4 4 0 0 1-8 0z M8 6H5a3 3 0 0 0 3 4 M16 6h3a3 3 0 0 1-3 4 M12 13v4 M8 20h8 M9 17h6"/>',
  history: '<path d="M4 12a8 8 0 1 0 2.4-5.7"/><path d="M4 4v4h4M12 8v4l3 2"/>',
  gear: '<circle cx="12" cy="12" r="3"/><path d="M12 2.5v3M12 18.5v3M4.2 5.6l2.1 2.1M17.7 16.3l2.1 2.1M2.5 12h3M18.5 12h3M4.2 18.4l2.1-2.1M17.7 7.7l2.1-2.1"/>',
  sun: '<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M2 12h2M20 12h2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/>',
  search: '<circle cx="11" cy="11" r="7"/><path d="M20 20l-4-4"/>',
  note: '<path d="M5 3h10l4 4v14H5z M15 3v4h4 M8 11h8M8 15h8M8 19h5"/>',
  leader: '<path d="M4 20V10M10 20V4M16 20v-8M22 20H2"/>',
  bracket: '<path d="M3 5h5v4h4 M3 13h5V9 M3 15h5v4h4 M3 23h5v-4 M12 7v14 M12 14h9"/>',
};
const icon = (k) => `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">${I[k] || ""}</svg>`;

// ---------- the sidebar (topic 1): every area of your job, by seat ----------
// Other lanes add their screens with navAdd({go, label, icon, sec, seats,
// after}); seats lists "college", "nfl", "hs" (all when left out).
const NAV = [
  { go: "home", label: "Home", icon: "home", sec: "top" },
  { go: "inbox", label: "Inbox", icon: "inbox", sec: "top", badge: () => (S.status && S.status.unread) || 0 },
  { go: "calendar", label: "Calendar", icon: "cal", sec: "top" },
  { go: "myplayers", label: "My players", icon: "star", sec: "top" },
  { go: "roster", label: "Squad", icon: "squad", sec: "team" },
  { go: "depth", label: "Depth chart", icon: "depth", sec: "team" },
  { go: "training", label: "Training", icon: "train", sec: "team" },
  { go: "recruiting", label: "Recruiting", icon: "recruit", sec: "team", seats: ["college", "hs"] },
  { go: "contracts", label: "Contracts", icon: "money", sec: "team", seats: ["nfl"] },
  { go: "schedule", label: "Schedule", icon: "sched", sec: "team" },
  { go: "games", label: "Game pages", icon: "game", sec: "team" },
  { go: "staff", label: "Staff", icon: "staff", sec: "team" },
  { go: "news", label: "News", icon: "news", sec: "world" },
  { go: "standings", label: "Standings", icon: "table", sec: "world" },
  { go: "leaders", label: "Leaderboards", icon: "leader", sec: "world" },
  { go: "history", label: "History", icon: "history", sec: "world" },
  { go: "team", label: "Team history", icon: "team", sec: "world" },
  { go: "you", label: "Your profile", icon: "you", sec: "world" },
];
const NAV_SECS = [["top", ""], ["team", "Your team"], ["world", "World"]];
function navAdd(item) {
  const i = NAV.findIndex((n) => n.go === item.go);
  if (i >= 0) { NAV[i] = { ...NAV[i], ...item }; return; }
  const at = item.after ? NAV.findIndex((n) => n.go === item.after) : -1;
  if (at >= 0) NAV.splice(at + 1, 0, item); else NAV.push(item);
}
function seatKind() {
  const me = S.status && S.status.mine;
  return me ? me.ref.kind : null;
}

// ---------- the frame ----------
function frame(content, crumb) {
  const st = S.status || {};
  const me = st.mine;
  const seat = seatKind();
  const navHtml = NAV_SECS.map(([sec, name]) => {
    const items = NAV.filter((n) => n.sec === sec && (!n.seats || !seat || n.seats.includes(seat)) && (!n.show || n.show()) && screens[n.go]);
    if (!items.length) return "";
    return (name ? `<div class="sec st-label">${name}</div>` : "") + items.map((n) => {
      const b = n.badge ? n.badge() : 0;
      return `<button class="nav" data-nav="${n.go}" aria-current="${S.route.screen === n.go}">${icon(n.icon)}<span>${esc(n.label)}</span>${b ? `<em class="badge">${b > 99 ? "99+" : b}</em>` : ""}</button>`;
    }).join("");
  }).join("");
  const next = st.next && st.next[0];
  return `<div class="frame">
    <aside class="side">
      <div class="brand">${me ? mark(me) : '<span class="st-mark">AFM</span>'}<div><b>${esc(me ? me.name : "AFM")}</b><small>${esc(st.you ? "Coach " + st.you : "American Football Manager")}</small></div></div>
      <nav class="navs">${navHtml}</nav>
      <div class="foot">
        <button class="nav save" data-act="save" title="Save game (⌘S)">${icon("save")}<span>Save</span></button>
        <button class="nav ico" data-act="holiday" title="Holiday" aria-label="Holiday">${icon("sun")}</button>
        <button class="nav ico" data-nav="settings" title="Settings" aria-label="Settings" aria-current="${S.route.screen === "settings"}">${icon("gear")}</button>
        <button class="nav ico" data-act="bug" title="Report a bug" aria-label="Report a bug">${icon("bug")}</button>
        <button class="nav ico" data-act="menu" title="Main menu" aria-label="Main menu">${icon("menu")}</button>
      </div>
    </aside>
    <section class="main">
      <header class="topbar">
        <button class="st-icon-btn" data-act="back" title="Back (⌘[)" ${S.back.length ? "" : "disabled"}>${icon("back")}</button>
        <button class="st-icon-btn" data-act="fwd" title="Forward (⌘])" ${S.fwd.length ? "" : "disabled"}>${icon("fwd")}</button>
        <nav class="crumb" aria-label="Where you are">${crumb || ""}</nav>
        <div class="search"><span class="sicon">${icon("search")}</span><input id="q" placeholder="Search anything" autocomplete="off" spellcheck="false"><kbd>⌘K</kbd></div>
        <div class="datebox"><b>${esc(st.today || "")}</b><small>${next ? esc(`${next.text.slice(0, 60)}${next.text.length > 60 ? "…" : ""} · ${next.in === 0 ? "today" : next.in === 1 ? "tomorrow" : "in " + next.in + " d"}`) : ""}</small></div>
        <button class="st-btn primary continue ${S.busy ? "busy" : ""}" data-act="continue" title="Continue (space)">${S.busy ? "Playing…" : "Continue"}</button>
        ${S.busy ? '<div class="progress"><i></i></div>' : ""}
      </header>
      ${pinnedHtml(st)}
      <div class="view">${content}</div>
    </section>
  </div>`;
}

// Things that must be answered sit under Continue with their deadlines
// (topic 1); each opens its decision.
function pinnedHtml(st) {
  const list = (st.pinned || []).slice(0, 4);
  if (!list.length) return "";
  return `<div class="pinned">${list.map((p) => {
    const open = p.mail != null ? `data-act="openmail" data-id="${p.mail}"` : p.go ? `data-go="${esc(p.go)}"${p.id != null ? ` data-id="${p.id}"` : ""}` : "";
    return `<button class="pin" ${open}><span class="pdot"></span><span class="ptxt"><b>${esc(p.text)}</b><small>${esc(p.rule || "Needs your answer")}</small></span><span class="when">${esc(p.by ? "Answer by " + p.by : p.date || "")}</span></button>`;
  }).join("")}</div>`;
}

// The path you took since your last sidebar click, each step a link (L7).
function crumbHtml() {
  const trail = [];
  for (let i = S.back.length - 1; i >= 0 && trail.length < 5; i--) {
    const r = S.back[i];
    if (outside(r)) break;
    trail.unshift({ r, n: S.back.length - i });
    if (r.root) break;
  }
  const label = (r) => esc(r.title || CRUMBS[r.screen] || r.screen);
  // A page opened from itself, or twice in a row, shows once.
  const key = (r) => r.screen + JSON.stringify(r.args || {});
  for (let i = trail.length - 1; i >= 0; i--) {
    if (key(trail[i].r) === key(S.route) || (i > 0 && key(trail[i].r) === key(trail[i - 1].r))) trail.splice(i, 1);
  }
  const steps = trail.map((t) => `<a class="link" data-act="crumb" data-n="${t.n}">${label(t.r)}</a>`);
  steps.push(`<b>${label({ ...S.route, title: S.title })}</b>`);
  return steps.join('<span class="sep">›</span>');
}

// ---------- screens ----------
const screens = {};
const CRUMBS = { home: "Home", inbox: "Inbox", news: "News", calendar: "Calendar", roster: "Squad", depth: "Depth chart", training: "Training", recruiting: "Recruiting", contracts: "Contracts", player: "Player", schedule: "Schedule", games: "Game pages", game: "Game", standings: "Standings", staff: "Staff", coach: "Coach", team: "Team", search: "Search", you: "Your profile", myplayers: "My players", leaders: "Leaderboards", history: "History", settings: "Settings", compare: "Compare", studio: "Live" };

// ---------- your team's screens (section 8; Lanes 10, 11 and 2 deepen them) ----------
const CLASSES = ["Fr", "So", "Jr", "Sr"];
const STATE_NAMES = { AL: "Alabama", AK: "Alaska", AZ: "Arizona", AR: "Arkansas", CA: "California", CO: "Colorado", CT: "Connecticut", DE: "Delaware", DC: "District of Columbia", FL: "Florida", GA: "Georgia", HI: "Hawaii", ID: "Idaho", IL: "Illinois", IN: "Indiana", IA: "Iowa", KS: "Kansas", KY: "Kentucky", LA: "Louisiana", ME: "Maine", MD: "Maryland", MA: "Massachusetts", MI: "Michigan", MN: "Minnesota", MS: "Mississippi", MO: "Missouri", MT: "Montana", NE: "Nebraska", NV: "Nevada", NH: "New Hampshire", NJ: "New Jersey", NM: "New Mexico", NY: "New York", NC: "North Carolina", ND: "North Dakota", OH: "Ohio", OK: "Oklahoma", OR: "Oregon", PA: "Pennsylvania", RI: "Rhode Island", SC: "South Carolina", SD: "South Dakota", TN: "Tennessee", TX: "Texas", UT: "Utah", VT: "Vermont", VA: "Virginia", WA: "Washington", WV: "West Virginia", WI: "Wisconsin", WY: "Wyoming" };
screens.roster = async (a) => {
  const x = await api("roster", a.kind ? { kind: a.kind, id: a.id } : {});
  setColors(x.team.colors);
  const f = keep("roster:" + (a.kind ? a.kind + a.id : "mine"), { unit: "ALL", cls: "ALL", injured: false });
  const count = (fn) => x.rows.filter(fn).length;
  const pro = x.team.ref.kind === "nfl";
  // NFL players by years in the league: rookies, young, prime, veterans.
  const band = (r) => (pro ? ((y) => (y === 0 ? "R" : y <= 3 ? "1-3" : y <= 6 ? "4-6" : "7+"))(classRank(r.class) || 0) : r.class.replace("RS ", ""));
  const bands = pro ? [["R", "Rookies"], ["1-3", "1-3 yrs"], ["4-6", "4-6 yrs"], ["7+", "7+ yrs"]] : CLASSES.map((c) => [c, c]);
  let rows = x.rows.filter((r) => (f.unit === "ALL" || r.group === f.unit) && (f.cls === "ALL" || band(r) === f.cls) && (!f.injured || /Injur/.test(r.status)));
  const cols = [
    { k: "name", t: "Name", l: true, f: (r) => `${playerLink(r.id, r.name)}${r.notes ? ' <span class="st-pill" title="Your notes">✎ ' + r.notes + "</span>" : ""}` },
    { k: "pos", t: "Pos", l: true },
    { k: "class", t: "Class", l: true, v: (r) => classRank(r.class) },
    { k: "age", t: "Age" },
    { k: "ht", t: "Ht", div: true, v: (r) => inches(r.ht) },
    { k: "wt", t: "Wt" },
    { k: "home", t: "Hometown", l: true, wrap: true },
    { k: "stars", t: "★", tip: "Recruiting stars", f: (r) => (r.stars ? "★".repeat(r.stars) : '<span class="muted">-</span>') },
    { k: "ovr", t: "Rating", div: true, tip: "Your staff's read of his play now, as a range", v: (r) => mid(r.ovr), f: (r) => range(r.ovr) },
    { k: "mood", t: "Mood", l: true },
    { k: "status", t: "Status", l: true, f: (r) => (r.status ? `<span class="dot ${/Injur|Susp|ineligible/.test(r.status) ? "bad" : "warn"}"></span>${esc(r.status)}` : '<span class="muted">Available</span>') },
  ];
  const seg = (k, opts) => `<div class="seg">${opts.map(([v, t, n]) => `<button data-act="rf" data-f="${k}" data-v="${v}" aria-pressed="${f[k] === v}">${t}${n != null ? `<small>${n}</small>` : ""}</button>`).join("")}</div>`;
  return `<div class="film"><div class="pad">
    <div style="display:flex;align-items:center;gap:14px">${mark(x.team, 48)}<div><div class="h1">${esc(x.team.name)}</div><div class="sub">${x.rows.length} players · ${x.mine ? "your staff reads them every day" : "only what is public: measurables and stars"}</div></div></div>
    <div class="filters">
      ${seg("unit", [["ALL", "All", x.rows.length], ["OFF", "Offense", count((r) => r.group === "OFF")], ["DEF", "Defense", count((r) => r.group === "DEF")], ["ST", "Special teams", count((r) => r.group === "ST")]])}
      ${seg("cls", [["ALL", pro ? "Everyone" : "All classes"], ...bands.map(([v, t]) => [v, t, count((r) => band(r) === v)])])}
      <div class="seg"><button data-act="rinj" aria-pressed="${f.injured}">Injured only</button></div>
    </div>
    ${table("roster", cols, rows, {
      groupBy: (r) => r.pos,
      groups: [{ t: "Player", span: 4 }, { t: "Body · measured", span: 4, div: true }, { t: "Opinion", who: x.mine ? "your position coaches" : "public", span: 3, div: true }],
    })}
  </div></div>`;
};

screens.training = async () => {
  const x = await api("training");
  setColors(x.team.colors);
  const set = x.rows.filter((r) => r.focus).length;
  const cols = [
    { k: "name", t: "Name", l: true, f: (r) => playerLink(r.id, r.name) },
    { k: "pos", t: "Pos", l: true },
    { k: "class", t: "Class", l: true, v: (r) => classRank(r.class) },
    { k: "age", t: "Age" },
    { k: "ovr", t: "Rating", div: true, v: (r) => mid(r.ovr), f: (r) => range(r.ovr) },
    { k: "focus", t: "Main focus", l: true, div: true, v: (r) => r.focus || "", f: (r) => `<select class="focus" data-act="focus" data-id="${r.id}" ${r.may_plan === false ? `disabled title="${esc(S.status && S.status.seat && S.status.seat.boss ? S.status.seat.boss.name : "The head coach")} plans him: your job covers ${esc(S.status && S.status.seat ? S.status.seat.controls : "")}"` : ""}><option value="">Balanced (the staff's plan)</option>${r.options.map((o) => `<option value="${o.key}" ${o.key === r.focus ? "selected" : ""}>${esc(o.label)}</option>`).join("")}</select>` },
  ];
  return `<div class="film"><div class="pad"><div class="h1">Training</div>
    <p class="sub">Each player's plan for the year. A main focus gets the bigger share of his individual periods: it grows up to half as fast again, and his other skills of the same kind a little slower. How much sticks depends on how coachable he is. Growth lands when his year's development does. <span id="focuscount">${set} of ${x.rows.length} players have a focus.</span></p>
    ${table("training", cols, x.rows, { groupBy: (r) => r.pos })}</div></div>`;
};

screens.recruiting = async () => {
  const x = await api("recruiting");
  setColors(x.team.colors);
  const college = x.level === "college";
  const cols = [
    { k: "name", t: "Name", l: true, f: (r) => playerLink(r.id, r.name) },
    { k: "pos", t: "Pos", l: true },
    { k: "class", t: "Class", l: true, v: (r) => classRank(r.class) },
    { k: "stars", t: "★", f: (r) => (r.stars ? "★".repeat(r.stars) : "-") },
    { k: "ht", t: "Ht", div: true, v: (r) => inches(r.ht) },
    { k: "wt", t: "Wt" },
    { k: college ? "school" : "home", t: college ? "High school" : "Hometown", l: true, f: (r) => esc(college ? `${r.school} · ${r.home}` : r.home) },
    { k: "rank", t: "Where it stands", l: true, div: true, v: (r) => -r.rank, f: (r) => (r.rank <= 1 ? `<b>${esc(r.recruit)}</b>` : esc(r.recruit)) },
    { k: "offers", t: "Offers" },
    ...(college ? [{ k: "elsewhere", t: "Committed elsewhere", l: true, f: (r) => esc(r.elsewhere || "") }] : [{ k: "offer_list", t: "Who offered", l: true, wrap: true, f: (r) => esc((r.offer_list || []).join(", ")) }]),
  ];
  const signed = x.rows.filter((r) => r.rank <= 1).length;
  const sub = college
    ? `Every high schooler your program has offered, visited, or who committed to you. Your recruiting staff runs the board for now; ${signed} committed or signed.`
    : `Your players with college interest: who offered, and where they stand. ${signed} committed or signed.`;
  return `<div class="film"><div class="pad"><div class="h1">Recruiting</div><p class="sub">${sub}</p>${table("recruiting", cols, x.rows, { sort: "rank", empty: "Nobody yet." })}</div></div>`;
};

screens.contracts = async () => {
  const x = await api("contracts");
  setColors(x.team.colors);
  const cols = [
    { k: "name", t: "Name", l: true, f: (r) => playerLink(r.id, r.name) },
    { k: "pos", t: "Pos", l: true },
    { k: "age", t: "Age" },
    { k: "ovr", t: "Rating", v: (r) => mid(r.ovr), f: (r) => range(r.ovr) },
    { k: "cap_hit", t: `${x.year} cap hit`, div: true, f: (r) => money(r.cap_hit) },
    { k: "apy", t: "A year", f: (r) => money(r.apy) },
    { k: "total", t: "Total", f: (r) => money(r.total) },
    { k: "guaranteed", t: "Guaranteed", f: (r) => money(r.guaranteed) },
    { k: "through", t: "Through", f: (r) => (r.ending ? `${r.through} <span class="st-pill gold" title="His deal ran out with last season: a free agent when the new league year opens, unless he signs again">Free agent soon</span>` : esc(r.through)) },
  ];
  const counted = x.top51 ? `the top 51 ${money(x.committed)} (all ${x.rows.length}: ${money(x.all)})` : money(x.committed);
  return `<div class="film"><div class="pad"><div class="h1">Contracts</div>
    <p class="sub">Your cap manager runs the books for now. Cap ${money(x.cap)}${x.carryover ? ` + ${money(x.carryover)} carried over` : ""} · contracts ${counted} · dead money ${money(x.dead)} · <b>room ${money(x.room)}</b>${x.top51 ? ". Until the first game only a team's 51 biggest cap hits count." : ""}</p>
    ${table("contracts", cols, x.rows, { sort: "cap_hit" })}</div></div>`;
};

screens.depth = async () => {
  const x = await api("depth");
  setColors(x.team.colors);
  const pos = keep("depthpos", { p: "QB" });
  const cur = x.positions.find((p) => p.pos === pos.p) || x.positions[0];
  // An assistant sets only his own job's positions (lane 10); the rest are the head coach's.
  const ctl = cur.controls !== false;
  const seat = S.status && S.status.seat;
  const boss = seat && seat.boss ? seat.boss.name : "the head coach";
  const rows = cur.players.map((r, i) => {
    const n = i < cur.starters ? `<b>${i + 1}</b>` : `<span class="muted">${i + 1}</span>`;
    const where = r.here ? esc(r.pos) : `<span class="st-pill gold" title="Listed at ${esc(r.pos)}: the staff plays him here too">${esc(r.pos)}</span>`;
    const deep = i >= cur.listed ? ' class="deep"' : "";
    return `<tr${deep}><td class="l">${n}</td><td class="l">${playerLink(r.id, r.name)}</td><td class="l">${where}</td><td class="l">${esc(r.class)}</td><td>${range(r.ovr)}</td><td class="l">${esc(r.status)}</td>
      <td><button class="st-btn small glass" data-act="dmove" data-i="${i}" data-d="-1" ${i === 0 || !ctl ? "disabled" : ""}>Up</button> <button class="st-btn small glass" data-act="dmove" data-i="${i}" data-d="1" ${i === cur.players.length - 1 || !ctl ? "disabled" : ""}>Down</button></td></tr>`;
  }).join("");
  return `<div class="film"><div class="pad">
    <div class="h1">Depth chart</div><p class="sub">The order your team plays on game day, as the staff will dress it: a player listed at another spot shows where they would use him too, as small schools play their best athletes both ways. Where you set an order, the engine plays yours. Players in grey after the line aren't in the game-day list.</p>
    <div class="tabsline">${x.positions.map((p) => `<button data-act="dpos" data-p="${p.pos}" aria-selected="${p.pos === cur.pos}"${p.controls === false ? ` style="opacity:.5" title="${esc(boss)}'s call"` : ""}>${p.pos}${p.set_by_you ? " ●" : ""}</button>`).join("")}</div>
    <div class="panel warm"><h2>${esc(cur.pos)} <small>${cur.starters} start · ${cur.set_by_you ? "your order" : !ctl ? `${esc(boss)}'s call: your job covers ${esc(seat.controls)}` : seat && !seat.head ? `the staff's order · you propose it, ${esc(boss)} has the last word` : "the staff's order"}</small></h2>
    <div class="tbl"><table><thead><tr><th class="l">#</th><th class="l">Name</th><th class="l">Listed at</th><th class="l">Class</th><th>Rating</th><th class="l">Status</th><th>Move</th></tr></thead><tbody>
    ${rows || `<tr><td class="l" colspan="7"><div class="empty">Nobody on the roster plays ${esc(cur.pos)}. On game day the staff gives the job to the best fit they have.</div></td></tr>`}
    </tbody></table></div>
    <div style="display:flex;gap:10px">${cur.set_by_you ? '<button class="st-btn small glass" data-act="dreset">Give it back to the staff</button>' : ""}</div></div>
  </div></div>`;
};

screens.schedule = async (a) => {
  const x = await api("schedule", a.kind ? { kind: a.kind, id: a.id } : {});
  setColors(x.team.colors);
  const games = x.games.map((g) => `<tr><td class="l">${esc(g.week ? "Wk " + g.week : "")} ${esc(g.date || "")}</td><td class="l">${esc(g.at)} ${teamLink(g.opp)}${g.name ? ` <span class="st-pill gold">${esc(g.name)}</span>` : ""}${g.conf ? ' <span class="muted">conf</span>' : ""}</td><td class="l muted">${esc((g.why || []).join(", "))}</td><td>${g.result ? `<b>${esc(g.result)}</b>` : '<span class="muted">' + esc(g.kick || "") + "</span>"}</td></tr>`).join("");
  const kept = x.box_scores.map((g) => `<div class="item"><span class="when">${esc(g.date)}</span><span><a class="link" data-go="game" data-index="${g.index}">${esc(g.away)} ${g.score[1]} at ${esc(g.home)} ${g.score[0]}</a></span></div>`).join("");
  return `<div class="film"><div class="pad"><div class="h1">${esc(x.team.name)} schedule</div>
    <div class="tbl warm"><table><thead><tr><th class="l">Date</th><th class="l">Opponent</th><th class="l">Why it matters</th><th>Result</th></tr></thead><tbody>${games || '<tr><td class="l" colspan="4"><div class="empty">The schedule comes out with the season.</div></td></tr>'}</tbody></table></div>
    ${kept ? `<div class="panel"><h2>Game pages <small>box score and every play</small></h2><div class="items">${kept}</div></div>` : ""}</div></div>`;
};

screens.games = async () => {
  const x = await api("games");
  return `<div class="pad"><div class="h1">Game pages</div><div class="panel"><div class="items">${x.games.map((g) => `<div class="item"><span class="when">${esc(g.date)}</span><span><a class="link" data-go="game" data-index="${g.index}">${esc(g.away)} ${g.score[1]} at ${esc(g.home)} ${g.score[0]}</a></span></div>`).join("") || '<div class="empty">Your games show here once the season starts.</div>'}</div></div></div>`;
};

screens.game = async (a) => {
  const g = await api("game", { index: a.index });
  const tab = keep("gtab", { t: "Box score" });
  const [h, w] = g.sides;
  const qs = (g.quarters || []).map((q, i) => `<th>${i < 4 ? "Q" + (i + 1) : "OT"}</th>`).join("");
  const line = (k) => (g.quarters || []).map((q) => `<td>${q[k]}</td>`).join("");
  const stat = (t, f) => `<tr><td class="l">${t}</td><td>${f(w.totals)}</td><td>${f(h.totals)}</td></tr>`;
  let body;
  if (tab.t === "Box score") {
    const side = (s) => `<div class="panel warm"><h2>${esc(s.name)}</h2>
      ${tbl(["Passing", "C/A", "Yds", "TD", "Int", "Sk"], s.passing.map((r) => [playerLink(r.id, r.name), `${r.cmp}/${r.att}`, r.yds, r.td, r.int, r.sk]))}
      ${tbl(["Rushing", "Car", "Yds", "TD", "Long"], s.rushing.map((r) => [playerLink(r.id, r.name), r.car, r.yds, r.td, r.long]))}
      ${tbl(["Receiving", "Rec", "Tgt", "Yds", "TD"], s.receiving.map((r) => [playerLink(r.id, r.name), r.rec, r.tgt, r.yds, r.td]))}
      ${tbl(["Defense", "Tkl", "TFL", "Sk", "Int", "PD"], s.defense.map((r) => [playerLink(r.id, r.name), r.tkl, r.tfl, r.sk, r.int, r.pd]))}</div>`;
    body = `<div class="panel warm"><div class="tbl"><table><thead><tr><th class="l">Team</th><th>${esc(w.name)}</th><th>${esc(h.name)}</th></tr></thead><tbody>
      ${stat("Total yards", (t) => t.yards)}${stat("Passing", (t) => t.pass)}${stat("Rushing", (t) => t.rush)}${stat("First downs", (t) => t.first_downs)}${stat("Third downs", (t) => t.third)}${stat("Turnovers", (t) => t.turnovers)}${stat("Penalties", (t) => `${t.penalties}-${t.pen_yds}`)}${stat("Possession", (t) => t.top)}${stat("Plays", (t) => t.plays)}
      </tbody></table></div></div><div class="grid2">${side(w)}${side(h)}</div>`;
  } else {
    let lastQ = 0;
    body = `<div class="panel warm"><div class="tbl pbp"><table><thead><tr><th class="l">Q</th><th class="l">Clock</th><th class="l">Ball</th><th class="l">Down</th><th class="l">Play</th><th class="l">The call and why</th></tr></thead><tbody>${g.plays
      .map((p) => {
        const sep = p.q !== lastQ ? `<tr class="grouprow"><td class="l" colspan="6">${p.q <= 4 ? "Quarter " + p.q : "Overtime"} · ${esc(w.name)} ${p.score[1]}, ${esc(h.name)} ${p.score[0]}</td></tr>` : "";
        lastQ = p.q;
        return `${sep}<tr><td class="l q">${p.q}</td><td class="l q">${esc(p.clock)}</td><td class="l q">${esc(p.off)}</td><td class="l q">${p.down ? `${p.down}&amp;${p.togo} ${esc(p.spot)}` : ""}</td><td class="l">${esc(p.text)}</td><td class="l why">${esc(p.call)}${p.why ? " · " + esc(p.why) : ""}</td></tr>`;
      })
      .join("")}</tbody></table></div></div>`;
  }
  return `<div class="st-room" style="padding:30px 26px;border-radius:0 0 28px 28px"><i></i><i></i><i></i>
    <div class="st-label">${esc(g.date)}${g.overtime ? " · overtime" : ""}</div>
    <div class="score" style="margin-top:10px"><span class="side-name">${esc(w.name)}</span><span class="pts">${w.score}</span><span class="sub">at</span><span class="pts">${h.score}</span><span class="side-name">${esc(h.name)}</span></div>
    <div class="tbl" style="max-width:520px;margin-top:14px"><table><thead><tr><th class="l"></th>${qs}<th>T</th></tr></thead><tbody><tr><td class="l">${esc(w.name)}</td>${line(1)}<td><b>${w.score}</b></td></tr><tr><td class="l">${esc(h.name)}</td>${line(0)}<td><b>${h.score}</b></td></tr></tbody></table></div></div>
    <div class="pad"><div class="tabsline">${["Box score", "Play by play"].map((t) => `<button data-act="gtab" data-t="${t}" aria-selected="${t === tab.t}">${t}</button>`).join("")}</div>${body}</div>`;
};
function tbl(head, rows) {
  if (!rows.length) return "";
  return `<div class="tbl"><table><thead><tr>${head.map((h, i) => `<th class="${i ? "" : "l"}">${esc(h)}</th>`).join("")}</tr></thead><tbody>${rows.map((r) => `<tr>${r.map((c, i) => `<td class="${i ? "" : "l"}">${i ? esc(num(c)) : c}</td>`).join("")}</tr>`).join("")}</tbody></table></div>`;
}

screens.staff = async (a) => {
  const x = await api("staff", a.kind ? { kind: a.kind, id: a.id } : {});
  setColors(x.team.colors);
  const cols = [
    { k: "role", t: "Role", l: true },
    { k: "name", t: "Name", l: true, f: (r) => (r.id == null ? `<b>${esc(r.name)}</b>` : `<a class="link name" data-go="coach" data-id="${r.id}">${esc(r.name)}</a>`) + (r.you ? ' <span class="st-pill gold">You</span>' : "") },
    { k: "age", t: "Age" },
    { k: "reputation", t: "Reputation", l: true },
    { k: "came_up", t: x.rows.some((r) => r.pay) ? "Came up as, or day job" : "Came up as", l: true },
    ...(x.rows.some((r) => r.pay) ? [{ k: "pay", t: "Stipend", f: (r) => (r.pay ? money(r.pay) : "") }] : []),
    { k: "record", t: "Record here", l: true },
    { k: "since", t: "Since" },
  ];
  return `<div class="film"><div class="pad"><div class="h1">${esc(x.team.name)} staff</div><p class="sub">Their ability isn't shown: you see what their work and their name say.</p>${table("staff", cols, x.rows)}</div></div>`;
};

// ---------- render ----------
async function render(scroll) {
  const r = S.route;
  const app = $("#app");
  const n = ++S.drawn;
  try {
    if (outside(r)) {
      // These pages lay themselves out to the window (pregame.css) and
      // scroll inside; drawn again (a row picked), each box keeps its place.
      const html = await screens[r.screen](r.args);
      if (n !== S.drawn) return;
      const page = r.screen + (r.screen === "setup" && S.keep.setup ? S.keep.setup.step : "");
      const kept = app.dataset.page === page ? [...app.querySelectorAll("[data-keep]")].map((el) => [el.dataset.keep, el.scrollTop]) : [];
      app.innerHTML = html;
      app.dataset.page = page;
      for (const [k, t] of kept) { const el = app.querySelector(`[data-keep="${k}"]`); if (el) el.scrollTop = t; }
      return;
    }
    if (!S.status) S.status = await api("status");
    if (!screens[r.screen]) { S.route = { screen: "home", args: {} }; return render(); }
    S.title = null;
    const html = await screens[r.screen](r.args);
    // A newer screen was asked for while this one loaded: it draws instead.
    if (n !== S.drawn) return;
    r.title = S.title || CRUMBS[r.screen] || r.screen;
    app.dataset.page = "";
    const keepY = scroll ?? null;
    app.innerHTML = frame(html, crumbHtml());
    if (S.beside) drawBeside(true);
    if (typeof afterRender === "function") afterRender();
    fitBoxes();
    if (keepY) $(".view").scrollTop = keepY;
  } catch (e) {
    console.error(e);
    toast(e.message || String(e));
    if (!app.innerHTML) app.innerHTML = frame(`<div class="pad"><div class="h1">Something went wrong</div><p class="sub">${esc(e.message || e)}</p></div>`);
  }
}
// Draws the screen again where it is (a tab, a filter): a control inside
// the page beside redraws that page only.
function refresh(el) {
  if (el && el.closest && el.closest(".beside")) return drawBeside();
  return render($(".view") ? $(".view").scrollTop : 0);
}

// A long table scrolls in its own box that ends at the window's bottom
// edge, so its column names stay on top and a table wider than the window
// shows its sideways scrollbar without scrolling the page first.
function fitBoxes() {
  const v = $(".view");
  if (!v) return;
  const vr = v.getBoundingClientRect();
  for (const b of document.querySelectorAll(".view .tbl.box, .view .tbl.pbp")) {
    const top = b.getBoundingClientRect().top - vr.top + v.scrollTop;
    const h = top < v.clientHeight - 300 ? v.clientHeight - top - 20 : v.clientHeight - 40;
    b.style.maxHeight = `${Math.max(260, Math.round(h))}px`;
  }
}
window.addEventListener("resize", fitBoxes);

// ---------- Continue ----------
async function cont() {
  if (S.busy) return;
  S.busy = true;
  // Only the button and the bar change while the days play: drawing the
  // whole screen again would fetch it again and could land after the next one.
  const b = $(".continue");
  if (b) { b.classList.add("busy"); b.textContent = "Playing…"; }
  const top = $(".topbar");
  if (top && !top.querySelector(".progress")) top.insertAdjacentHTML("beforeend", '<div class="progress"><i></i></div>');
  document.documentElement.classList.add("running");
  const from = S.route;
  try {
    const r = await api("continue", { max_days: 400 });
    S.status = r.status;
    S.busy = false;
    document.documentElement.classList.remove("running");
    if (typeof peekForget === "function") peekForget();
    // A screen picked while the days played: you stay there.
    const moved = S.route !== from;
    const whys = r.why.map((w) => w.text);
    const why = whys.slice(0, 3).join(" · ") + (whys.length > 3 ? ` · and ${whys.length - 3} more in your inbox` : "");
    if (r.game) {
      toast(`Final: ${r.game.text}`);
      if (moved) render();
      else go("game", { index: r.game.index });
    } else {
      // A morning stop is today, before it plays; any other stop is the
      // day just played, and you are on the morning after it.
      toast(r.morning ? `${r.today}, morning${why ? ": " + why : ""}` : `${r.days} day${r.days === 1 ? "" : "s"} played, to ${r.today}. On ${r.stopped_on}${why ? ": " + why : ""}`);
      // A live day (signing day, the draft, cutdown day) opens its room.
      if (r.studio && screens.studio && !moved) go("studio", r.studio);
      // A meeting this morning opens its room (people).
      else if (r.meeting != null && screens.people_scene && !moved) go("people_scene", { id: r.meeting });
      else if (moved || S.route.screen === "home") render();
      else go("home", {}, { root: true });
    }
    if (typeof showReveals === "function") showReveals(r.status && r.status.reveals);
  } catch (e) {
    S.busy = false;
    document.documentElement.classList.remove("running");
    toast(e.message);
    render();
  }
}

// ---------- events ----------
// A link's arguments: data-kind, data-id, data-index, data-tab, and any
// data-args JSON.
function linkArgs(d) {
  const args = {};
  if (d.args) { try { Object.assign(args, JSON.parse(d.args)); } catch (e) { /* plain link */ } }
  if (d.kind) args.kind = d.kind;
  if (d.id !== undefined && d.id !== "") args.id = Number(d.id);
  if (d.index !== undefined) args.index = Number(d.index);
  if (d.tab) args.tab = d.tab;
  return args;
}
document.addEventListener("click", async (ev) => {
  let el = ev.target.closest("[data-go],[data-act],[data-sort],[data-nav],input,select,textarea,label");
  // A click anywhere on a table row opens its first link, as in FM.
  if (!el) { const tr = ev.target.closest(".tbl tbody tr"); el = tr && !tr.classList.contains("grouprow") && tr.querySelector("[data-go]"); }
  if (!el || !el.dataset || (!el.dataset.go && !el.dataset.act && !el.dataset.sort && !el.dataset.nav)) return;
  const d = el.dataset;
  if (d.nav) {
    if (typeof closeBeside === "function") closeBeside();
    go(d.nav, {}, { root: true });
    return;
  }
  if (d.sort) {
    const [key, k] = d.sort.split("|");
    const s = keep("sort:" + key, { k: null, dir: -1 });
    if (ev.shiftKey && s.k && s.k !== k) {
      // Shift-click: a second sort, inside the first.
      if (s.k2 !== k) { s.k2 = k; s.dir2 = -1; } else if (s.dir2 < 0) s.dir2 = 1; else s.k2 = null;
    } else if (s.k !== k) { s.k = k; s.dir = -1; s.k2 = null; } else if (s.dir < 0) s.dir = 1; else { s.k = null; s.k2 = null; }
    refresh(el);
    return;
  }
  if (d.go) {
    ev.preventDefault();
    const args = linkArgs(d);
    if (d.go === "player") S.notesOpen = false;
    // Shift-click: the page beside this one (L6); ⌘-click: its own window.
    if (ev.shiftKey && typeof openBeside === "function") { openBeside(d.go, args); return; }
    if ((ev.metaKey || ev.ctrlKey) && typeof openWindow === "function") { openWindow(d.go, args); return; }
    if (el.closest(".beside") && typeof closeBeside === "function") closeBeside();
    if (typeof peekHide === "function") peekHide();
    go(d.go, args);
    return;
  }
  if (typeof coreAct === "function" && (await coreAct(d.act, el, ev))) return;
  switch (d.act) {
    case "back": goBack(); break;
    case "fwd": goFwd(); break;
    case "crumb": goBackTo(Number(d.n)); break;
    case "continue": cont(); break;
    case "theme": prefs.theme = prefs.theme === "light" ? "" : "light"; savePrefs(); applyPrefs(); break;
    case "save": {
      if (S.busy) break;
      const name = await ask("Save your career", { value: S.lastSaveName || `${S.status && S.status.mine ? S.status.mine.name : (S.status && S.status.you) || "career"} ${S.status ? S.status.season : ""}`, ok: "Save", note: "A save with the same name is replaced." });
      if (!name) break;
      showBusy("Saving your career… a full world takes about a minute.");
      try {
        const r = await api("save", { name });
        S.savedOn = S.status && S.status.today;
        S.lastSaveName = name;
        hideBusy();
        toast("Saved: " + r.saved);
      } catch (e) { hideBusy(); toast(e.message); }
      break;
    }
    case "menu": go("start"); break;
    case "resume": enterCareer(); break;
    case "bug": {
      const note = await ask("What looks wrong?", { placeholder: "One line is enough: what you did and what you saw.", ok: "Save the bug report", long: true, note: "The report keeps this screen and the game as it is now, so it can be played back." });
      if (!note) break;
      showBusy("Saving the bug report…");
      try { const r = await api("bug", { note, screen: S.route }); hideBusy(); toast("Bug report saved in " + r.folder); } catch (e) { hideBusy(); toast(e.message); }
      break;
    }
    case "rf": { const f = S.keep["roster:" + routeKey()]; f[d.f] = d.v; refresh(el); break; }
    case "rinj": { const f = S.keep["roster:" + routeKey()]; f.injured = !f.injured; refresh(el); break; }
    case "gtab": S.keep.gtab.t = d.t; S.route.args.tab = d.t; refresh(el); break;
    case "newsmine": S.keep.news.mine = d.v === "1"; refresh(el); break;
    case "dpos": S.keep.depthpos.p = d.p; S.route.args.pos = d.p; refresh(el); break;
    case "dmove":
    case "dreset": {
      const x = await api("depth");
      const cur = x.positions.find((p) => p.pos === S.keep.depthpos.p) || x.positions[0];
      let ids = cur.players.map((p) => p.id);
      if (d.act === "dmove") {
        const i = Number(d.i), j = i + Number(d.d);
        [ids[i], ids[j]] = [ids[j], ids[i]];
      } else ids = [];
      // An assistant's order is a proposal: the head coach's answer, if any.
      const r = await api("set_depth", { pos: cur.pos, ids });
      if (r && r.note) toast(r.note);
      refresh(el);
      break;
    }
  }
});
function routeKey() {
  const a = S.route.args;
  return a.kind ? a.kind + a.id : "mine";
}
document.addEventListener("change", (ev) => {
  if (ev.target.dataset.act === "conf") go("standings", { ...S.route.args, conference: ev.target.value }, false);
  if (ev.target.dataset.act === "focus") {
    api("set_focus", { id: Number(ev.target.dataset.id), key: ev.target.value })
      .then(() => {
        toast("Plan saved.");
        const n = document.querySelectorAll("select.focus").length;
        const set = [...document.querySelectorAll("select.focus")].filter((x) => x.value).length;
        const c = $("#focuscount");
        if (c) c.textContent = `${set} of ${n} players have a focus.`;
      })
      .catch((e) => toast(e.message));
  }
});
document.addEventListener("keydown", (ev) => {
  const typing = ev.target.closest && ev.target.closest("input, textarea, select, [contenteditable]");
  // A career with no job yet still runs its days (lane 10).
  const inCareer = S.status && (S.status.mine || S.status.you) && !outside(S.route);
  if ((ev.metaKey || ev.ctrlKey) && ev.key.toLowerCase() === "k") { ev.preventDefault(); if (typeof openFind === "function") openFind(); else { const q = $("#q"); if (q) q.focus(); } return; }
  // ⌘[ and ⌘] go back and forward (topic 15).
  if ((ev.metaKey || ev.ctrlKey) && ev.key === "[" && inCareer) { ev.preventDefault(); goBack(); return; }
  if ((ev.metaKey || ev.ctrlKey) && ev.key === "]" && inCareer) { ev.preventDefault(); goFwd(); return; }
  // ⌘S saves, as the sidebar's Save game does.
  if ((ev.metaKey || ev.ctrlKey) && ev.key.toLowerCase() === "s" && inCareer) { ev.preventDefault(); const b = $('[data-act="save"]'); if (b) b.click(); return; }
  if (ev.key === " " && ev.target === document.body && inCareer && !document.querySelector(".overlay, .letter-room, .st-moment.fixed")) { ev.preventDefault(); cont(); return; }
  // Option+arrows move by a word inside a text field on a Mac.
  if (ev.altKey && !typing && ev.key === "ArrowLeft" && inCareer) goBack();
  if (ev.altKey && !typing && ev.key === "ArrowRight" && inCareer) goFwd();
});
// The mouse's back and forward buttons.
window.addEventListener("mouseup", (ev) => {
  if (!(S.status && (S.status.mine || S.status.you)) || outside(S.route)) return;
  if (ev.button === 3) { ev.preventDefault(); goBack(); }
  if (ev.button === 4) { ev.preventDefault(); goFwd(); }
});
// A two-finger swipe left or right on the trackpad goes back or forward,
// as in Safari, unless what is under the pointer scrolls sideways.
(() => {
  let acc = 0, t0 = 0, fired = 0;
  const scrollsX = (el) => {
    for (let e = el; e && e !== document.body; e = e.parentElement) {
      if (e.scrollWidth > e.clientWidth + 2) {
        const o = getComputedStyle(e).overflowX;
        if (o === "auto" || o === "scroll") return true;
      }
    }
    return false;
  };
  window.addEventListener("wheel", (ev) => {
    if (!(S.status && (S.status.mine || S.status.you)) || outside(S.route)) return;
    if (Math.abs(ev.deltaX) < Math.abs(ev.deltaY) * 1.6 || ev.ctrlKey) return;
    if (scrollsX(ev.target)) return;
    const now = performance.now();
    if (now - t0 > 260) acc = 0;
    t0 = now;
    acc += ev.deltaX;
    if (now - fired < 700) return;
    const hint = $("#swipehint") || document.body.appendChild(Object.assign(document.createElement("div"), { id: "swipehint", className: "swipehint" }));
    hint.className = "swipehint " + (acc < 0 ? "l" : "r");
    hint.style.opacity = String(Math.min(1, Math.abs(acc) / 160));
    clearTimeout(hint._t);
    hint._t = setTimeout(() => (hint.style.opacity = "0"), 220);
    if (Math.abs(acc) > 160) {
      fired = now;
      const dir = acc;
      acc = 0;
      hint.style.opacity = "0";
      if (dir < 0) goBack(); else goFwd();
    }
  }, { passive: true });
})();

// One busy box at a time: a second call changes its words. It counts the
// seconds, and after five minutes says what to do if nothing comes back.
let busyTimer = null;
function showBusy(text) {
  if (document.activeElement && document.activeElement !== document.body) document.activeElement.blur();
  let o = $("#busy");
  if (!o) {
    o = document.createElement("div");
    o.className = "overlay";
    o.id = "busy";
    document.body.appendChild(o);
  }
  const t0 = Date.now();
  o.innerHTML = `<div class="box"><b>${esc(text)}</b><div class="st-bar" style="width:100%"><i style="width:30%;animation:st-scan 1.1s ease-in-out infinite alternate"></i></div><small class="busy-time"></small></div>`;
  clearInterval(busyTimer);
  busyTimer = setInterval(() => {
    const el = o.querySelector(".busy-time");
    if (!el || !o.isConnected) return clearInterval(busyTimer);
    const s = Math.round((Date.now() - t0) / 1000);
    el.textContent = `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}` + (s > 300 ? " · This is taking much longer than it should. If nothing changes in a few minutes, quit AFM (⌘Q), open it again and report the bug." : "");
  }, 1000);
}
function hideBusy() { clearInterval(busyTimer); document.querySelectorAll("#busy").forEach((o) => o.remove()); }

// The game's own question boxes. The Mac app's window has no browser
// pop-ups (prompt and confirm return nothing there), so every question is
// asked in the page. `ask` gives the text typed, or null; `sure` true or false.
function dialog(html, ok, onOk) {
  const o = document.createElement("div");
  o.className = "overlay" + (outside(S.route) ? " pg-dlg" : "");
  o.innerHTML = `<form class="box ask">${html}<div class="ask-actions"><button type="button" class="st-btn glass" data-dlg="no">Cancel</button><button type="submit" class="st-btn primary">${esc(ok)}</button></div></form>`;
  document.body.appendChild(o);
  const f = o.querySelector("form");
  const answer = new Promise((done) => {
    const keys = (ev) => { if (ev.key === "Escape") { ev.preventDefault(); end(null); } };
    const end = (v) => { o.remove(); document.removeEventListener("keydown", keys, true); done(v); };
    document.addEventListener("keydown", keys, true);
    f.addEventListener("submit", (ev) => { ev.preventDefault(); end(onOk(f)); });
    o.querySelector("[data-dlg=no]").addEventListener("click", () => end(null));
    o.addEventListener("mousedown", (ev) => { if (ev.target === o) end(null); });
  });
  return { f, answer };
}
function ask(title, { value = "", placeholder = "", ok = "OK", note = "", long = false } = {}) {
  const field = long
    ? `<textarea name="v" rows="4" placeholder="${esc(placeholder)}">${esc(value)}</textarea>`
    : `<input name="v" value="${esc(value)}" placeholder="${esc(placeholder)}" autocomplete="off">`;
  const { f, answer } = dialog(`<b>${esc(title)}</b>${note ? `<small class="sub">${esc(note)}</small>` : ""}${field}`, ok, (f) => f.elements.v.value.trim() || null);
  f.elements.v.focus();
  f.elements.v.select();
  return answer;
}
function sure(title, note, ok = "Go on") {
  const { f, answer } = dialog(`<b>${esc(title)}</b>${note ? `<small class="sub">${esc(note)}</small>` : ""}`, ok, () => true);
  f.querySelector("[type=submit]").focus();
  return answer.then((v) => v === true);
}
// Before anything that closes the career you have open.
function leaveCareer() {
  if (!(S.status && S.status.you)) return Promise.resolve(true);
  const since = S.savedOn ? (S.savedOn === S.status.today ? "You saved today." : `Your last save was on ${S.savedOn}; the days since then are lost.`) : "You haven't saved this career yet: all of it is lost.";
  return sure("Close the career you have open?", `${since} Save first from the sidebar if you want to keep it.`, "Close it");
}

// ---------- updates ----------
// The desktop app updates itself: at start, and every few hours while it runs,
// it asks the public downloads page for a newer build, downloads it in the
// background and installs it. With no career open it restarts on its own;
// with one open it asks first, so nothing unsaved is lost.
let updating = false;
function updateBar(html) {
  let b = $("#updatebar");
  if (!b) {
    b = document.createElement("div");
    b.id = "updatebar";
    b.className = "updatebar";
    document.body.appendChild(b);
  }
  b.innerHTML = html;
}
async function checkUpdate() {
  const T = window.__TAURI__;
  if (!T || !T.updater || updating) return;
  let up = null;
  try { up = await T.updater.check(); } catch (e) { console.warn("update check", e); return false; }
  if (!up) return null;
  updating = true;
  let got = 0, total = 0;
  updateBar(`<b>Updating AFM to version ${esc(up.version)}…</b><small id="upmb"></small>`);
  try {
    await up.downloadAndInstall((ev) => {
      if (ev.event === "Started") total = ev.data.contentLength || 0;
      if (ev.event === "Progress") got += ev.data.chunkLength || 0;
      const el = $("#upmb");
      if (el && total) el.textContent = `${Math.round(got / 1e6)} of ${Math.round(total / 1e6)} MB`;
    });
  } catch (e) {
    updating = false;
    updateBar(`<b>The update didn't install.</b><small>${esc(e.message || e)}. AFM tries again next time it starts.</small><button class="st-btn small glass" data-act="updclose">Close</button>`);
    return;
  }
  // A career open, or a world being built or downloaded: never cut off.
  const working = await api("job").then((j) => !!(j && !j.finished), () => false);
  const busy = (S.status && S.status.you) || working;
  if (!busy) return T.process.relaunch();
  updateBar(`<b>Version ${esc(up.version)} is installed.</b><small>${working ? "Restart once the world you're building or downloading is ready." : "Save your career, then restart to use it."}</small><button class="st-btn small primary" data-act="updrestart">Restart now</button>`);
}
document.addEventListener("click", (ev) => {
  const a = ev.target.closest("[data-act=updrestart],[data-act=updclose]");
  if (!a) return;
  if (a.dataset.act === "updrestart") window.__TAURI__.process.relaunch();
  else { const b = $("#updatebar"); if (b) b.remove(); }
});
setTimeout(checkUpdate, 3000);
setInterval(checkUpdate, 3 * 3600 * 1000);

// ---------- start ----------
// A page opened in its own window (⌘-click) carries its route in the
// address: #player?id=12&tab=Stats.
function routeFromHash() {
  const h = decodeURIComponent(location.hash.slice(1));
  if (!h) return null;
  const [screen, q] = h.split("?");
  const args = {};
  for (const kv of (q || "").split("&").filter(Boolean)) {
    const [k, v] = kv.split("=");
    args[k] = /^-?\d+$/.test(v) ? Number(v) : v;
  }
  return { screen, args };
}
(async () => {
  applyPrefs();
  try {
    const h = await api("hello");
    if (h.loaded) {
      S.status = await api("status");
      const r = routeFromHash();
      const inCareer = S.status.mine || S.status.you;
      if (inCareer && r && r.screen) { S.route = { screen: r.screen, args: r.args, root: true }; render(); return; }
      go(inCareer ? "home" : "start", {}, { push: false, root: true });
    } else go("start", {}, false);
  } catch (e) {
    $("#app").innerHTML = `<div class="setup"><div class="h1">AFM can't reach the game</div><p class="sub">${esc(e.message || e)}</p></div>`;
  }
})();
