// AFM's screens (section 8 topic 5, section 10 topics 1-5). One page: a
// frosted sidebar, a top bar with the date and Continue, and the screen.
// Every name is a link; back and forward work like a browser and each
// screen keeps its sort and filters when you come back.
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
  t.textContent = text;
  $("#toasts").appendChild(t);
  setTimeout(() => t.remove(), 4200);
}
function setColors(c) {
  if (!c) return;
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
function teamLink(t) {
  if (!t) return "";
  return `<a class="link" data-go="team" data-kind="${t.ref.kind}" data-id="${t.ref.id}">${esc(t.name)}</a>`;
}
function playerLink(id, name) {
  return `<a class="link name" data-go="player" data-id="${id}">${esc(name)}</a>`;
}
// The rating colour: white low, red high (screens topic 4).
function rateColor(v) {
  const t = Math.max(0, Math.min(1, (v - 35) / 55));
  const a = [244, 239, 233], b = [200, 16, 46];
  const c = a.map((x, i) => Math.round(x + (b[i] - x) * t));
  return `rgb(${c[0]},${c[1]},${c[2]})`;
}
function range(r) {
  if (!r) return `<span class="rg ns"><span class="t"></span><b>Not seen</b></span>`;
  const [lo, hi] = r;
  const mid = (lo + hi) / 2;
  return `<span class="rg" title="Your staff's read: ${lo} to ${hi}"><span class="t"><i style="left:${lo}%;width:${Math.max(2, hi - lo)}%;background:${rateColor(mid)}"></i></span><b>${lo}-${hi}</b></span>`;
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
};
const keep = (k, d) => (S.keep[k] ??= d);

// The opening, load and new-career pages sit outside a career: back and forward move
// between the career's screens only, so they never lead out of it.
const outside = (r) => r.screen === "start" || r.screen === "setup" || r.screen === "load";
function go(screen, args = {}, push = true) {
  if (push && !(outside(S.route) && !outside({ screen }))) {
    S.back.push({ ...S.route, scroll: $(".view") ? $(".view").scrollTop : 0 });
    S.fwd = [];
  }
  S.route = { screen, args };
  render();
}
// Into a career (taken, loaded or resumed): its history starts here.
function enterCareer() {
  S.back = [];
  S.fwd = [];
  go("home", {}, false);
}
function goBack() {
  if (!S.back.length) return;
  S.fwd.push(S.route);
  S.route = S.back.pop();
  render(S.route.scroll);
}
function goFwd() {
  if (!S.fwd.length) return;
  S.back.push(S.route);
  S.route = S.fwd.pop();
  render();
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
};
const icon = (k) => `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">${I[k] || ""}</svg>`;

// ---------- the frame ----------
function frame(content, crumb) {
  const st = S.status || {};
  const me = st.mine;
  const nav = [
    ["home", "Home", "home"],
    ["inbox", "Inbox", "inbox"],
    ["calendar", "Calendar", "cal"],
    ["sec", "Your team"],
    ["roster", "Squad", "squad"],
    ["depth", "Depth chart", "depth"],
    ["training", "Training", "train"],
    ...(me && me.ref.kind !== "nfl" ? [["recruiting", "Recruiting", "recruit"]] : []),
    ...(me && me.ref.kind === "nfl" ? [["contracts", "Contracts", "money"]] : []),
    ["schedule", "Schedule", "sched"],
    ["games", "Game pages", "game"],
    ["standings", "Standings", "table"],
    ["staff", "Staff", "staff"],
    ["sec", "World"],
    ["news", "News", "news"],
    ["team", me ? "Team history" : "Team", "team"],
    ["you", "Your profile", "you"],
  ];
  const navHtml = nav
    .map((n) =>
      n[0] === "sec"
        ? `<div class="sec st-label">${n[1]}</div>`
        : `<button class="nav" data-go="${n[0]}" aria-current="${S.route.screen === n[0]}">${icon(n[2])}${n[1]}</button>`,
    )
    .join("");
  const next = st.next && st.next[0];
  const pinned = (st.pinned || [])
    .map((p) => `<button class="st-banner"><span class="st-mark" style="background:var(--live)">!</span><span><b>${esc(p.text)}</b><small>${esc(p.rule || "Needs your answer")}</small></span><span class="when">${esc(p.date)}</span></button>`)
    .join("");
  return `<div class="frame">
    <aside class="side">
      <div class="brand">${me ? mark(me) : '<span class="st-mark">AFM</span>'}<div><b>${esc(me ? me.name : "AFM")}</b><small>${esc(st.you ? "Coach " + st.you : "American Football Manager")}</small></div></div>
      <nav class="navs">${navHtml}</nav>
      <div class="foot">
        <button class="nav" data-act="save">${icon("save")}Save game</button>
        <button class="nav" data-act="menu">${icon("menu")}Main menu</button>
        <button class="nav" data-act="bug">${icon("bug")}Report a bug</button>
        <button class="nav" data-act="theme">${icon("theme")}Light or dark</button>
      </div>
    </aside>
    <section class="main">
      <header class="topbar">
        <button class="st-icon-btn" data-act="back" title="Back">${icon("back")}</button>
        <button class="st-icon-btn" data-act="fwd" title="Forward">${icon("fwd")}</button>
        <div class="crumb">${crumb || ""}</div>
        <div class="search"><input id="q" placeholder="Search players, coaches, teams (⌘K)" autocomplete="off"></div>
        <div class="datebox"><b>${esc(st.today || "")}</b><small>${next ? esc(`${next.text.slice(0, 60)}${next.text.length > 60 ? "…" : ""} · ${next.in === 0 ? "today" : next.in === 1 ? "tomorrow" : "in " + next.in + " d"}`) : ""}</small></div>
        <button class="st-btn primary continue ${S.busy ? "busy" : ""}" data-act="continue">${S.busy ? "Playing…" : "Continue"}</button>
        ${S.busy ? '<div class="progress"><i></i></div>' : ""}
      </header>
      ${pinned ? `<div class="pinned">${pinned}</div>` : ""}
      <div class="view">${content}</div>
    </section>
  </div>`;
}

// ---------- the table component (screens topic 3) ----------
// cols: [{k, t, l (left), div (group start), v: row => sort value, f: row => html}]
function table(key, cols, rows, opts = {}) {
  const s = keep("sort:" + key, { k: opts.sort || null, dir: -1 });
  let list = rows.slice();
  if (s.k) {
    const c = cols.find((c) => c.k === s.k);
    if (c) {
      const v = c.v || ((r) => r[c.k]);
      list.sort((a, b) => {
        const x = v(a), y = v(b);
        if (typeof x === "string" || typeof y === "string") return String(x).localeCompare(String(y)) * -s.dir;
        return ((x ?? -1e9) - (y ?? -1e9)) * s.dir;
      });
    }
  }
  const groups = opts.groups
    ? `<tr class="groups">${opts.groups.map((g) => `<th colspan="${g.span}" class="${g.div ? "div" : ""}">${esc(g.t)}${g.who ? ` <span>· ${esc(g.who)}</span>` : ""}</th>`).join("")}</tr>`
    : "";
  // Words sort A to Z on the first click, numbers highest first; the arrow
  // shows the order on screen.
  const text = (c) => typeof (c.v || ((r) => r[c.k]))(rows[0] || {}) === "string";
  const arrow = (c) => (s.k !== c.k ? "" : (text(c) ? s.dir > 0 : s.dir < 0) ? "sorted-desc" : "sorted-asc");
  const head = cols
    .map((c) => `<th data-sort="${key}|${c.k}" class="${c.l ? "l" : ""} ${c.div ? "div" : ""} ${arrow(c)}" title="${esc(c.tip || c.t)}">${esc(c.t)}</th>`)
    .join("");
  let body = "";
  let lastGroup = null;
  for (const r of list) {
    if (opts.groupBy && !s.k) {
      const g = opts.groupBy(r);
      if (g !== lastGroup) {
        body += `<tr class="grouprow"><td class="l" colspan="${cols.length}">${esc(g)}</td></tr>`;
        lastGroup = g;
      }
    }
    body += `<tr>${cols.map((c) => `<td class="${c.l ? "l" : ""} ${c.div ? "div" : ""} ${c.wrap ? "wrap" : ""}">${c.f ? c.f(r) : esc(num(r[c.k]))}</td>`).join("")}</tr>`;
  }
  if (!list.length) body = `<tr><td class="l" colspan="${cols.length}"><div class="empty">${esc(opts.empty || "Nothing here yet.")}</div></td></tr>`;
  return `<div class="tbl warm box"><table><thead>${groups}<tr>${head}</tr></thead><tbody>${body}</tbody></table></div>`;
}

// ---------- screens ----------
const screens = {};

// The opening screen, Load game and the new career's six steps are in
// pregame.js, in their own look.

screens.home = async () => {
  const h = await api("home");
  if (!h.team) return `<div class="pad"><div class="h1">No team yet</div></div>`;
  setColors(h.team.colors);
  const ng = h.next_game;
  const inbox = (h.inbox || []).slice(0, 14).map(itemRow).join("") || '<div class="empty">Quiet so far.</div>';
  const news = (h.news || []).slice(0, 14).map(itemRow).join("");
  const followed = (h.followed || []).map((p) => `<div class="item"><span class="when">${esc(p.pos)}</span><span>${playerLink(p.id, p.name)} <span class="sub">· ${esc(p.where)}</span><br><small class="sub">${esc(p.note || "")}</small></span></div>`).join("") || '<div class="empty">Write a note on any player and he shows up here.</div>';
  return `<div class="hero"><div class="art"></div><div class="st-outline">${esc(h.status.season)}</div>
    <div class="copy"><div class="st-label" style="color:rgba(255,255,255,.7)">${esc(h.team.level)}</div>
    <h1>${esc(h.team.name)}</h1>
    <div class="meta"><span><b>${esc(h.team.record)}</b> ${esc(h.team.record_note || "this season")}</span>${ng ? `<span>Next: <b>${ng.home ? "vs" : "at"} ${esc(ng.opp.name)}</b> · ${esc(ng.date || "")}${ng.in != null ? ` (in ${ng.in} days)` : ""}</span>` : ""}</div>
    <div class="st-actions"><button class="st-btn glass" data-go="roster">Squad</button><button class="st-btn glass" data-go="depth">Depth chart</button><button class="st-btn glass" data-go="schedule">Schedule</button></div></div></div>
    <div class="pad"><div class="grid2">
      <div class="panel"><h2>Inbox <small>what needs you and what concerns your team</small></h2><div class="items">${inbox}</div></div>
      <div style="display:grid;gap:18px"><div class="panel"><h2>My players <small>players you keep notes on</small></h2><div class="items">${followed}</div></div>
      <div class="panel"><h2>Around football</h2><div class="items">${news}</div></div></div>
    </div></div>`;
};

// A line's text with every name it links (L2): the engine sends `parts`,
// plain `{t}` or a link `{t, go, kind, id, index}` that opens on click.
function lineHtml(e) {
  // A name ending in Jr. at the end of a line: one period.
  const fix = (t) => String(t || "").replace(/\b(Jr|Sr)\.\./g, "$1.");
  if (!Array.isArray(e.parts) || !e.parts.length) return esc(fix(e.text));
  const attr = (k, v) => (v == null ? "" : ` data-${k}="${esc(v)}"`);
  return e.parts.map((p, i) => {
    const t = i === e.parts.length - 1 ? fix(p.t) : p.t;
    if (!p.go) return esc(t);
    const cls = p.go === "player" ? "link name" : "link";
    return `<a class="${cls}" data-go="${p.go}"${attr("kind", p.kind)}${attr("id", p.id)}${attr("index", p.index)}>${esc(t)}</a>`;
  }).join("");
}

function itemRow(e) {
  const cls = `${(e.loud || "").toLowerCase()} ${e.decision ? "decision" : ""}`;
  const text = lineHtml(e);
  return `<div class="item ${cls}"><span class="when">${esc(e.date || "")}</span><span>${e.loud === "Big" || e.loud === "Huge" ? "<b>" + text + "</b>" : text}${e.rule ? ` <small class="sub">· ${esc(e.rule)}</small>` : ""}</span></div>`;
}

screens.inbox = async () => {
  const x = await api("inbox", { days: 120 });
  return `<div class="pad"><div class="h1">Inbox</div><div class="panel"><div class="items">${x.items.map(itemRow).join("") || '<div class="empty">Nothing yet.</div>'}</div></div></div>`;
};
screens.news = async () => {
  const k = keep("news", { mine: false });
  const x = await api("news", { limit: 300, mine: k.mine });
  return `<div class="pad"><div class="h1">News</div><div class="seg"><button data-act="newsmine" data-v="0" aria-pressed="${!k.mine}">Everything</button><button data-act="newsmine" data-v="1" aria-pressed="${k.mine}">Your team and level</button></div>
    <div class="panel"><div class="items">${x.items.map(itemRow).join("")}</div></div></div>`;
};
screens.calendar = async () => {
  const x = await api("calendar");
  const tag = { game: "Your game", date: "Date", huge: "Huge", big: "Big" };
  return `<div class="pad"><div class="h1">Calendar</div><p class="sub">Your games, the year's fixed dates at your level and the big moments already set. Yours are in bold.</p><div class="panel"><div class="items">${x.items.map((e) => `<div class="item cal"><span class="when">${esc(e.date)}<br><small>${e.in === 0 ? "today" : "in " + e.in + " d"}</small></span><span>${e.mine ? "<b>" + lineHtml(e) + "</b>" : lineHtml(e)}</span><span class="st-chip ${e.loud}">${tag[e.loud] || ""}</span></div>`).join("") || '<div class="empty">Nothing set yet.</div>'}</div></div></div>`;
};

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
    { k: "focus", t: "Main focus", l: true, div: true, v: (r) => r.focus || "", f: (r) => `<select class="focus" data-act="focus" data-id="${r.id}"><option value="">Balanced (the staff's plan)</option>${r.options.map((o) => `<option value="${o.key}" ${o.key === r.focus ? "selected" : ""}>${esc(o.label)}</option>`).join("")}</select>` },
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
  const rows = cur.players.map((r, i) => {
    const n = i < cur.starters ? `<b>${i + 1}</b>` : `<span class="muted">${i + 1}</span>`;
    const where = r.here ? esc(r.pos) : `<span class="st-pill gold" title="Listed at ${esc(r.pos)}: the staff plays him here too">${esc(r.pos)}</span>`;
    const deep = i >= cur.listed ? ' class="deep"' : "";
    return `<tr${deep}><td class="l">${n}</td><td class="l">${playerLink(r.id, r.name)}</td><td class="l">${where}</td><td class="l">${esc(r.class)}</td><td>${range(r.ovr)}</td><td class="l">${esc(r.status)}</td>
      <td><button class="st-btn small glass" data-act="dmove" data-i="${i}" data-d="-1" ${i === 0 ? "disabled" : ""}>Up</button> <button class="st-btn small glass" data-act="dmove" data-i="${i}" data-d="1" ${i === cur.players.length - 1 ? "disabled" : ""}>Down</button></td></tr>`;
  }).join("");
  return `<div class="film"><div class="pad">
    <div class="h1">Depth chart</div><p class="sub">The order your team plays on game day, as the staff will dress it: a player listed at another spot shows where they would use him too, as small schools play their best athletes both ways. Where you set an order, the engine plays yours. Players in grey after the line aren't in the game-day list.</p>
    <div class="tabsline">${x.positions.map((p) => `<button data-act="dpos" data-p="${p.pos}" aria-selected="${p.pos === cur.pos}">${p.pos}${p.set_by_you ? " ●" : ""}</button>`).join("")}</div>
    <div class="panel warm"><h2>${esc(cur.pos)} <small>${cur.starters} start · ${cur.set_by_you ? "your order" : "the staff's order"}</small></h2>
    <div class="tbl"><table><thead><tr><th class="l">#</th><th class="l">Name</th><th class="l">Listed at</th><th class="l">Class</th><th>Rating</th><th class="l">Status</th><th>Move</th></tr></thead><tbody>
    ${rows || `<tr><td class="l" colspan="7"><div class="empty">Nobody on the roster plays ${esc(cur.pos)}. On game day the staff gives the job to the best fit they have.</div></td></tr>`}
    </tbody></table></div>
    <div style="display:flex;gap:10px">${cur.set_by_you ? '<button class="st-btn small glass" data-act="dreset">Give it back to the staff</button>' : ""}</div></div>
  </div></div>`;
};

screens.player = async (a) => {
  const p = await api("player", { id: a.id });
  if (p.team) setColors(p.team.colors);
  const tab = keep("ptab", { t: "Overview" });
  const tabs = ["Overview", "Attributes", "Career", ...(p.tendencies.length ? ["Tendencies"] : []), ...(p.pro ? ["Contract"] : []), ...(p.offers.length ? ["Recruiting"] : [])];
  if (!tabs.includes(tab.t)) tab.t = "Overview";
  const who = p.mine ? "your position coach" : "public measurables only";
  let body = "";
  if (tab.t === "Overview") {
    const phys = p.groups.find((g) => g.name === "Physical");
    body = `<div class="grid2"><div class="panel"><h2>Your staff's read</h2>
      ${p.potential ? `<div style="font-size:20px;font-weight:800">${esc(p.potential.tier)}</div><div class="sub">Staff ${esc(p.potential.sure.replace(/([A-Z])/g, " $1").trim().toLowerCase())}.${p.potential.hints.length ? " " + p.potential.hints.map(esc).join(". ") + "." : ""}</div>` : `<div class="sub">Your staff hasn't scouted him. Only his measurables and his public profile are known.</div>`}
      <div class="kv"><dt>Rating now</dt><dd>${range(p.ovr)}</dd><dt>Mood</dt><dd>${esc(p.mood)}</dd><dt>Status</dt><dd>${esc(p.status || "Available")}</dd>${p.born ? `<dt>Born</dt><dd>${esc(p.born)} <small class="sub">(${p.age})</small></dd>` : ""}<dt>On camera</dt><dd>${esc(p.on_camera)}</dd>${p.followers ? `<dt>Followers</dt><dd>${p.followers >= 1e6 ? (p.followers / 1e6).toFixed(1) + "M" : p.followers >= 1e4 ? Math.round(p.followers / 1e3) + "k" : p.followers.toLocaleString()}</dd>` : ""}${p.brand ? `<dt>Ads and deals</dt><dd>${money(p.brand)} <small class="sub">a year, reported</small></dd>` : ""}</div></div>
      <div class="panel"><h2>Physical <small>Opinion: ${esc(who)}</small></h2>${phys.attrs.map((x) => `<div class="attr"><span>${esc(x.label)}</span>${range(x.read)}</div>`).join("")}</div></div>
      ${p.stories.length ? `<div class="panel"><h2>What happened</h2><div class="items">${p.stories.map((s) => `<div class="item story"><span>${esc(s)}</span></div>`).join("")}</div></div>` : ""}`;
  } else if (tab.t === "Attributes") {
    body = p.groups.map((g) => `<div class="panel"><h2>${esc(g.name)}</h2><div class="opinion">Opinion: ${esc(p.mine ? "your coaches" : g.name === "Physical" ? "public measurables" : "not seen by your staff")}</div><div class="attrs">${g.attrs.map((x) => `<div class="attr"><span>${esc(x.label)}</span>${range(x.read)}</div>`).join("")}</div></div>`).join("");
  } else if (tab.t === "Career") {
    body = table("pcareer", [
      { k: "age", t: "Age" }, { k: "level", t: "Level", l: true }, { k: "pos", t: "Pos", l: true }, { k: "role", t: "Role", l: true },
      { k: "team", t: "Team", l: true }, { k: "missed", t: "Missed" }, { k: "events", t: "What happened", l: true, f: (r) => esc((r.events || []).map(words).join(", ")) },
    ], p.history);
  } else if (tab.t === "Tendencies") {
    body = `<div class="panel"><h2>Tendencies <small>how often he tries things, not how well</small></h2><div class="attrs">${p.tendencies.map((t) => `<div class="attr"><span>${esc(t.label)}</span>${range(t.read)}</div>`).join("")}</div></div>`;
  } else if (tab.t === "Contract") {
    const c = p.pro.contract;
    body = `<div class="panel"><h2>Contract</h2>${c ? `<div class="st-tiles"><div><b>${money(c.apy)}</b><small>a year</small></div><div><b>${money(c.total)}</b><small>${c.years} years total</small></div><div><b>${money(c.guaranteed)}</b><small>guaranteed</small></div><div><b>${money(c.cap_hit)}</b><small>cap hit this year</small></div><div><b>${c.through}</b><small>through</small></div></div>` : '<div class="empty">No contract.</div>'}
      <div class="kv"><dt>Drafted</dt><dd>${esc(p.pro.draft || "Undrafted")}</dd><dt>Accrued seasons</dt><dd>${p.pro.accrued}</dd><dt>Career earnings</dt><dd>${money(p.pro.earnings)}</dd></div></div>`;
  } else if (tab.t === "Recruiting") {
    body = `<div class="panel"><h2>Offers <small>${p.offers.length}</small></h2><div class="items">${p.offers.map((o) => `<div class="item"><span class="when"></span><span>${esc(o)}</span></div>`).join("")}</div></div>`;
  }
  const notes = S.notesOpen
    ? `<aside class="notes"><div style="display:flex;justify-content:space-between;align-items:center"><b>Your notes on ${esc(p.name)}</b><button class="st-btn small glass" data-act="notes">Close</button></div>
      <div style="display:grid;gap:8px"><textarea id="note" placeholder="Only you see your notes. They stay with him for his whole career."></textarea><button class="st-btn small primary" data-act="addnote" data-id="${p.id}">Save note</button></div>
      <div class="list">${p.notes.map((n) => `<div class="note"><small><span>${esc(n.date)}</span><a class="link" data-act="delnote" data-id="${p.id}" data-k="${n.k}">Delete</a></small>${esc(n.text)}</div>`).join("") || '<div class="empty">No notes yet.</div>'}</div></aside>`
    : "";
  return `<div class="hero small"><div class="art"></div><div class="st-outline">${esc(p.pos)}</div><div class="copy">
    <div class="st-label" style="color:rgba(255,255,255,.7)">${esc(p.pos)} · ${esc(p.class)} · ${p.team ? teamLink(p.team) : esc(p.where)}</div>
    <h1>${esc(p.name)}</h1>
    <div class="meta"><span>Age <b>${p.age}</b></span><span><b>${esc(p.ht)}</b>, <b>${p.wt}</b> lb</span><span>${esc(p.home)}</span>${p.high_school ? `<span>${esc(p.high_school)}</span>` : ""}${p.stars ? `<span>${"★".repeat(p.stars)}</span>` : ""}${p.status ? `<span class="st-pill live">${esc(p.status)}</span>` : ""}</div>
    <div class="st-actions"><button class="st-btn glass" data-act="notes">Notes${p.notes.length ? ` (${p.notes.length})` : ""}</button><button class="st-btn glass" title="A placeholder for now" disabled>Dynamic Links</button></div></div></div>
    <div class="pad"><div class="tabsline">${tabs.map((t) => `<button data-act="ptab" data-t="${t}" aria-selected="${t === tab.t}">${t}</button>`).join("")}</div>${body}</div>${notes}`;
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

screens.standings = async (a) => {
  const x = await api("standings", a.conference ? { conference: a.conference } : {});
  const groups = x.groups.map((g) => `<div class="panel warm"><h2>${esc(g.name)}</h2><div class="tbl"><table><thead><tr><th class="l">Team</th>${g.teams[0] && g.teams[0].cw != null ? "<th>Conf</th>" : ""}<th>Overall</th></tr></thead><tbody>${g.teams
    .map((t) => `<tr class="${t.team.mine ? "mine" : ""}"><td class="l"><span class="teamcell">${mark(t.team, 22)} ${teamLink(t.team)}</span></td>${t.cw != null ? `<td>${t.cw}-${t.cl}</td>` : ""}<td><b>${t.w}-${t.l}${t.t ? "-" + t.t : ""}</b></td></tr>`)
    .join("")}</tbody></table></div></div>`).join("");
  const confs = x.conferences ? `<div class="field" style="max-width:320px"><label>Conference</label><select data-act="conf">${x.conferences.map((c) => `<option ${x.groups[0] && c === x.groups[0].name ? "selected" : ""}>${esc(c)}</option>`).join("")}</select></div>` : "";
  return `<div class="film"><div class="pad"><div class="h1">Standings</div>${confs}<div class="${x.groups.length > 1 ? "grid2" : ""}">${groups || '<div class="empty">No standings until the season starts.</div>'}</div></div></div>`;
};

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

screens.coach = async (a) => {
  const c = await api("coach", { id: a.id });
  return `<div class="pad"><div class="st-label">${c.job ? `${esc(c.job.role)} · ${esc(c.job.name)} since ${c.job.since}${c.job.record ? ` · ${esc(c.job.record)}` : ""}` : "Out of work"}</div><div class="h1">${esc(c.name)}${c.you ? ' <span class="st-pill gold">You</span>' : ""}</div>
    <div class="kv"><dt>Age</dt><dd>${c.age}</dd><dt>Reputation</dt><dd>${esc(c.reputation)}</dd><dt>Came up as</dt><dd>${esc(c.came_up)}</dd><dt>Played pro</dt><dd>${c.ex_pro ? "Yes" : "No"}</dd><dt>Titles</dt><dd>${c.titles}</dd>${c.mentor ? `<dt>Learned under</dt><dd><a class="link" data-go="coach" data-id="${c.mentor.id}">${esc(c.mentor.name)}</a></dd>` : ""}</div>
    ${tbl(["Where", "Role", "Years", "Record", "How it ended"], c.history.map((h) => [esc(h.where), h.role, `${h.from}-${h.to}`, h.record, h.ended]))}</div>`;
};

screens.team = async (a) => {
  const t = await api("team", a.kind ? { kind: a.kind, id: a.id } : {});
  setColors(t.colors);
  const pro = t.ref.kind === "nfl";
  const seasons = t.seasons && t.seasons.length
    ? pro
      ? tbl(["Season", "Record", "Head coach", "Playoffs"], t.seasons.map((s) => [String(s.year), s.record, s.coach || "", s.note]))
      : tbl(["Season", "Record", "Coach", "Points", "Titles"], t.seasons.map((s) => [String(s.year), s.record, s.coach || "", s.g ? `${s.pf}-${s.pa}` : "", [s.conf ? "Conference" : "", s.nat ? (t.ref.kind === "hs" ? "State" : "National") : ""].filter(Boolean).join(", ")]))
    : "";
  const noSeasons = pro ? "The league's book of seasons starts with this world's next season." : "No seasons kept.";
  const q = `data-kind="${t.ref.kind}" data-id="${t.ref.id}"`;
  return `<div class="hero"><div class="art"></div><div class="st-outline">${esc(initials(t.name))}</div><div class="copy"><div class="st-label" style="color:rgba(255,255,255,.7)">${esc(t.level)}</div><h1>${esc(t.name)}</h1>
    <div class="meta"><span><b>${esc(t.record)}</b> ${esc(t.record_note || "this season")}</span>${t.place ? `<span>${esc(t.place)}</span>` : ""}${t.prestige ? `<span>${esc(t.prestige)}</span>` : ""}${t.stadium ? `<span>${Number(t.stadium).toLocaleString()} seats</span>` : ""}</div>
    <div class="st-actions"><button class="st-btn glass" data-go="roster" ${q}>Roster</button><button class="st-btn glass" data-go="schedule" ${q}>Schedule</button><button class="st-btn glass" data-go="staff" ${q}>Staff</button></div></div></div>
    <div class="pad"><div class="panel"><h2>Seasons</h2>${seasons || `<div class="empty">${noSeasons}</div>`}</div></div>`;
};

screens.search = async (a) => {
  const x = await api("search", { q: a.q });
  const none = (what) => `<div class="empty">No ${what} match "${esc(a.q)}".</div>`;
  return `<div class="pad"><div class="h1">Search: ${esc(a.q)}</div><div class="grid2">
    <div class="panel"><h2>Players <small>${x.players.length === 60 ? "the first 60" : x.players.length}</small></h2>${x.players.length ? tbl(["Name", "Pos", "Age", "Level", "Where"], x.players.map((p) => [playerLink(p.id, p.name), p.pos, p.age, p.level, p.where])) : none("players")}</div>
    <div style="display:grid;gap:18px"><div class="panel"><h2>Teams</h2><div class="items">${x.teams.map((t) => `<div class="item"><span class="when">${mark(t, 22)}</span><span>${teamLink(t)}<br><small class="sub">${esc(t.sub || "")}</small></span></div>`).join("") || none("teams")}</div></div>
    <div class="panel"><h2>Coaches</h2><div class="items">${x.coaches.map((c) => `<div class="item"><span class="when">${esc(c.role || "Out of work")}</span><span><a class="link" data-go="coach" data-id="${c.id}">${esc(c.name)}</a>${c.at ? ` <small class="sub">· ${esc(c.at)}</small>` : ""}</span></div>`).join("") || none("coaches")}</div></div></div></div></div>`;
};

const STYLE = [["risk", "Risk", "Conservative", "Aggressive"], ["freedom", "Structure or freedom", "Do your job", "Freelance"], ["physical", "Physical", "Protective", "Physical"], ["ball_hunting", "Ball hunting", "Sound", "Ball hunting"], ["mould", "Mould or adapt", "Adapt to him", "Mould him"]];
screens.you = async () => {
  const y = await api("you");
  if (!y) return `<div class="pad"><div class="h1">No career yet</div></div>`;
  const row = (t, v) => `<div class="attr"><span>${esc(t)}</span><span class="rg"><span class="t"><i style="left:0;width:${(v / 20) * 100}%;background:${rateColor(v * 5)}"></i></span><b>${v.toFixed(1)}</b></span></div>`;
  const x = y.extra;
  const p = y.profile || {};
  const from = [p.hometown, y.home_state].filter(Boolean).join(", ");
  const school = [p.alma_mater, p.college_years].filter(Boolean).join(", ") + (p.degree ? ` (${p.degree})` : "");
  const kids = (p.children || []).map(([n, b]) => `${n}, ${Math.max(0, Number(String(S.status && S.status.today).slice(-4)) - b)}`).join("; ");
  const family = [p.status, p.status === "Married" && p.spouse ? `to ${p.spouse}` : ""].filter(Boolean).join(" ") + (kids ? `; children: ${kids}` : "");
  const looks = [p.height, p.build, p.hair && p.hair + " hair", p.facial_hair].filter(Boolean).join(", ").toLowerCase();
  return `<div class="pad"><div style="display:flex;gap:18px;align-items:center">${p.photo ? `<img src="${esc(p.photo)}" alt="" style="width:88px;height:110px;object-fit:cover;border-radius:14px">` : ""}<div><div class="st-label">Head coach since ${esc(y.started)}</div><div class="h1">${esc(y.name)}</div></div></div>
    <div class="kv"><dt>Age</dt><dd>${y.age}</dd>${y.road ? `<dt>Came up as</dt><dd>${esc(y.road)}</dd>` : ""}<dt>Reputation</dt><dd>${esc(y.reputation)}</dd><dt>Record</dt><dd>${esc(y.record)}</dd><dt>Titles</dt><dd>${y.titles}</dd>${x && x.calls ? `<dt>Calls the plays</dt><dd>${esc(x.calls)}</dd>` : ""}
    ${from ? `<dt>From</dt><dd>${esc(from)}${p.high_school ? ` · ${esc(p.high_school)}` : ""}</dd>` : ""}${school.trim() ? `<dt>College</dt><dd>${esc(school)}</dd>` : ""}${family.trim() ? `<dt>Family</dt><dd>${esc(family)}${p.roots ? ` · roots: ${esc(p.roots)}` : ""}</dd>` : ""}${looks ? `<dt>Looks</dt><dd>${esc(looks)}</dd>` : ""}</div>
    <div class="grid2"><div class="panel"><h2>Teaching <small>your own ratings, 1-20; nobody else sees them</small></h2>${y.teaching.map((t) => row(t.group, t.value)).join("")}</div>
    <div class="panel"><h2>The rest</h2>${[["Scheme", y.scheme], ["Game management", y.game_management], ...(x ? [["Play calling", x.play_calling], ["In-game adaptation", x.adaptation]] : []), ["Motivation", y.motivation], ["Discipline", y.discipline], ["Relationships", y.relationships], ["Recruiting", y.recruiting], ...(x ? [["Evaluation", x.evaluation]] : []), ["Network", y.network], ...(x ? [["Negotiating", x.negotiating]] : [])].map(([t, v]) => row(t, v)).join("")}</div></div>
    ${y.style ? `<div class="panel"><h2>Your coaching style <small>how your team plays and how you coach habits, 1-20</small></h2>${STYLE.map(([k, t, lo, hi]) => `<div class="attr"><span>${esc(t)} <small class="sub">${esc(lo)} to ${esc(hi)}</small></span><span class="rg"><span class="t"><i style="left:0;width:${(y.style[k] / 20) * 100}%;background:${rateColor(y.style[k] * 5)}"></i></span><b>${Math.round(y.style[k])}</b></span></div>`).join("")}</div>` : ""}
    ${y.history && y.history.length ? `<div class="panel"><h2>Your jobs</h2>${tbl(["Where", "Role", "Years", "Record"], y.history.map((h) => [esc(h.where), h.role, `${h.from}-${h.to}`, h.record]))}</div>` : ""}</div>`;
};

// ---------- render ----------
const CRUMBS = { home: "Home", inbox: "Inbox", news: "News", calendar: "Calendar", roster: "Squad", depth: "Depth chart", training: "Training", recruiting: "Recruiting", contracts: "Contracts", player: "Player", schedule: "Schedule", games: "Game pages", game: "Game", standings: "Standings", staff: "Staff", coach: "Coach", team: "Team", search: "Search", you: "Your profile" };

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
    const html = await screens[r.screen](r.args);
    // A newer screen was asked for while this one loaded: it draws instead.
    if (n !== S.drawn) return;
    app.dataset.page = "";
    const me = S.status && S.status.mine;
    app.innerHTML = frame(html, `${me ? esc(me.name) + " · " : ""}<b>${CRUMBS[r.screen] || ""}</b>`);
    fitBoxes();
    if (scroll) $(".view").scrollTop = scroll;
  } catch (e) {
    console.error(e);
    toast(e.message || String(e));
    if (!app.innerHTML) app.innerHTML = frame(`<div class="pad"><div class="h1">Something went wrong</div><p class="sub">${esc(e.message || e)}</p></div>`);
  }
}

// A long table scrolls in its own box that ends at the window's bottom
// edge, so its column names stay on top and a table wider than the window
// shows its sideways scrollbar without scrolling the page first.
function fitBoxes() {
  const v = $(".view");
  if (!v) return;
  const vr = v.getBoundingClientRect();
  for (const b of document.querySelectorAll(".tbl.box, .tbl.pbp")) {
    const top = b.getBoundingClientRect().top - vr.top + v.scrollTop;
    const h = top < v.clientHeight - 300 ? v.clientHeight - top - 20 : v.clientHeight - 40;
    b.style.maxHeight = `${Math.max(260, Math.round(h))}px`;
  }
}
window.addEventListener("resize", fitBoxes);

async function cont() {
  if (S.busy) return;
  S.busy = true;
  // Only the button and the bar change while the days play: drawing the
  // whole screen again would fetch it again and could land after the next one.
  const b = $(".continue");
  if (b) { b.classList.add("busy"); b.textContent = "Playing…"; }
  const top = $(".topbar");
  if (top && !top.querySelector(".progress")) top.insertAdjacentHTML("beforeend", '<div class="progress"><i></i></div>');
  const from = S.route;
  try {
    const r = await api("continue", { max_days: 400 });
    S.status = r.status;
    S.busy = false;
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
      if (moved || S.route.screen === "home") render();
      else go("home");
    }
  } catch (e) {
    S.busy = false;
    toast(e.message);
    render();
  }
}

// ---------- events ----------
document.addEventListener("click", async (ev) => {
  let el = ev.target.closest("[data-go],[data-act],[data-sort],input,select,textarea");
  // A click anywhere on a table row opens its first link, as in FM.
  if (!el) { const tr = ev.target.closest(".tbl tbody tr"); el = tr && tr.querySelector("[data-go]"); }
  if (!el || !el.dataset || (!el.dataset.go && !el.dataset.act && !el.dataset.sort)) return;
  const d = el.dataset;
  if (d.sort) {
    const [key, k] = d.sort.split("|");
    const s = keep("sort:" + key, { k: null, dir: -1 });
    // Highest first, then lowest first, then back to the table's own order.
    if (s.k !== k) { s.k = k; s.dir = -1; } else if (s.dir < 0) s.dir = 1; else s.k = null;
    render($(".view") && $(".view").scrollTop);
    return;
  }
  if (d.go) {
    const args = {};
    if (d.kind) args.kind = d.kind;
    if (d.id !== undefined) args.id = Number(d.id);
    if (d.index !== undefined) args.index = Number(d.index);
    if (d.go === "player") S.notesOpen = false;
    go(d.go, args);
    return;
  }
  switch (d.act) {
    case "back": goBack(); break;
    case "fwd": goFwd(); break;
    case "continue": cont(); break;
    case "theme": {
      const h = document.documentElement;
      h.dataset.theme = h.dataset.theme === "light" ? "" : "light";
      break;
    }
    case "save": {
      if (S.busy) break;
      const name = await ask("Save your career", { value: S.lastSaveName || `${S.status && S.status.mine ? S.status.mine.name : "career"} ${S.status ? S.status.season : ""}`, ok: "Save", note: "A save with the same name is replaced." });
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
    case "rf": { const f = S.keep["roster:" + routeKey()]; f[d.f] = d.v; render($(".view").scrollTop); break; }
    case "rinj": { const f = S.keep["roster:" + routeKey()]; f.injured = !f.injured; render(); break; }
    case "ptab": S.keep.ptab.t = d.t; render(); break;
    case "gtab": S.keep.gtab.t = d.t; render(); break;
    case "notes": S.notesOpen = !S.notesOpen; render($(".view").scrollTop); break;
    case "addnote": {
      const text = $("#note").value.trim();
      if (!text) break;
      await api("note_add", { id: Number(d.id), text });
      render($(".view").scrollTop);
      break;
    }
    case "delnote": await api("note_delete", { id: Number(d.id), note: Number(d.k) }); render($(".view").scrollTop); break;
    case "newsmine": S.keep.news.mine = d.v === "1"; render(); break;
    case "dpos": S.keep.depthpos.p = d.p; render(); break;
    case "dmove":
    case "dreset": {
      const x = await api("depth");
      const cur = x.positions.find((p) => p.pos === S.keep.depthpos.p) || x.positions[0];
      let ids = cur.players.map((p) => p.id);
      if (d.act === "dmove") {
        const i = Number(d.i), j = i + Number(d.d);
        [ids[i], ids[j]] = [ids[j], ids[i]];
      } else ids = [];
      await api("set_depth", { pos: cur.pos, ids });
      render($(".view").scrollTop);
      break;
    }
  }
});
function routeKey() {
  const a = S.route.args;
  return a.kind ? a.kind + a.id : "mine";
}
document.addEventListener("change", (ev) => {
  if (ev.target.dataset.act === "conf") go("standings", { conference: ev.target.value });
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
  if ((ev.metaKey || ev.ctrlKey) && ev.key === "k") { ev.preventDefault(); const q = $("#q"); if (q) q.focus(); }
  if (ev.target.id === "q" && ev.key === "Enter" && ev.target.value.trim().length > 1) go("search", { q: ev.target.value.trim() });
  if (ev.key === " " && ev.target === document.body && S.status && S.status.mine && !outside(S.route)) { ev.preventDefault(); cont(); }
  // Option+arrows move by a word inside a text field on a Mac.
  const typing = ev.target.closest && ev.target.closest("input, textarea, select, [contenteditable]");
  if (ev.altKey && !typing && ev.key === "ArrowLeft") goBack();
  if (ev.altKey && !typing && ev.key === "ArrowRight") goFwd();
});
window.addEventListener("mouseup", (ev) => { if (ev.button === 3) goBack(); if (ev.button === 4) goFwd(); });

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
  if (!(S.status && S.status.mine)) return Promise.resolve(true);
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
  const busy = (S.status && S.status.mine) || working;
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
(async () => {
  try {
    const h = await api("hello");
    if (h.loaded) {
      S.status = await api("status");
      go(S.status.mine ? "home" : "start", {}, false);
    } else go("start", {}, false);
  } catch (e) {
    $("#app").innerHTML = `<div class="setup"><div class="h1">AFM can't reach the game</div><p class="sub">${esc(e.message || e)}</p></div>`;
  }
})();
