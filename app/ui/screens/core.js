// Lane screens: core (docs/final-build/plan.md). Registers screens.<name>.
"use strict";
/* Lane 9 · the screens' core (final build): the shared components every
 * screen uses, and the core screens (Home, Inbox, the player page, News,
 * Calendar, Standings, the team, coach and career pages, My players,
 * Leaderboards and the player finder, History, Settings, Compare).
 *
 * Components (other lanes call these; all are globals):
 *
 *   rich(text)                  The game's link markup to HTML. [[p:12|Name]]
 *                               a player, [[c:7|Name]] a coach,
 *                               [[t:college:45|Iowa]] a team (nfl, college,
 *                               hs), [[g:3|text]] one of your kept games,
 *                               [[d:2026-03-14|text]] a calendar day,
 *                               [[s:screen?k=v|text]] any screen. Everything
 *                               else is escaped. Use it for every line the
 *                               engine wrote (L1). api("linked", {texts})
 *                               links plain lines the same way.
 *   linkTo(screen, args, html)  One link by hand (data-go with arguments).
 *   teamA(team) / teamChip(team) A team link / a team link with its mark.
 *   pageHead({kicker, title, sub, actions, goto})
 *                               A screen's title block; goto is the "Go to"
 *                               line from gotoFor(screen, args) (L2).
 *   GOTO[screen](args, data)    The topic 17 map of where each screen leads;
 *                               other lanes add theirs: GOTO.myscreen = ...
 *   tabs(key, list, current)    A tab bar; tabOf(key, list, args) reads the
 *                               current one (kept in the route, so back
 *                               returns to it, L4).
 *   table(key, cols, rows, opts) The sortable table (S8): click a column to
 *                               sort, shift-click for a second sort, the
 *                               Columns menu, saved views, the filter
 *                               builder, chips ("On my list", "Has my
 *                               note"), compare up to five, CSV, arrow keys
 *                               and Enter. cols: {k, t, l (left), f (cell
 *                               HTML), v (sort value), tip, hide (off by
 *                               default), stat (a leaderboard key: the cell
 *                               opens that leaderboard)}. opts: {sort,
 *                               groupBy, empty, groups, select(row) -> id
 *                               (compare), tools: false, mineChip}.
 *   bigTable(cfg)               The engine-paged table for tens of thousands
 *                               of rows (leaders, the finder): it asks the
 *                               game for the rows it draws as you scroll.
 *   hover cards                 Any data-go link to a player, coach, team or
 *                               game shows its card after a short hover (L5).
 *   openBeside(screen, args)    The page beside the screen (shift-click, L6);
 *   openWindow(screen, args)    its own window (⌘-click).
 *   ctxMenu(ev, items)          A right-click menu; names get one by
 *                               themselves (S10).
 *   openFind(q)                 The search palette (⌘K, S6).
 *   showLetter(id)              The inbox's full-screen letter (10c).
 *   spark(values) / bars(rows)  Small charts.
 *   posterScene(kind, colors)   The drawn scene on a storyline poster (9c).
 *   face(p, size)               The face slot: Lane 12 sets window.faceFor;
 *                               until then an initials silhouette.
 *   showReveals(n)              The big moments (topic 9), from the game.
 *   PLAYER_TABS                 The player page's tabs; other lanes push
 *                               {name, after, show(p), draw(p)} (Lane 11's
 *                               Development tab has its slot after Stats).
 *   noteBox(about, page)        "Add a note" anywhere: about {kind, id, name}.
 */

// ---------- links (L1) ----------
function linkTo(screen, args, html, cls = "link") {
  return `<a class="${cls}" data-go="${esc(screen)}" data-args='${esc(JSON.stringify(args || {}))}'>${html}</a>`;
}
function rich(text) {
  const s = String(text ?? "");
  let out = "";
  let i = 0;
  while (i < s.length) {
    const a = s.indexOf("[[", i);
    if (a < 0) { out += esc(s.slice(i)); break; }
    const b = s.indexOf("]]", a);
    const bar = s.indexOf("|", a);
    if (b < 0 || bar < 0 || bar > b) { out += esc(s.slice(i, a + 2)); i = a + 2; continue; }
    out += esc(s.slice(i, a));
    const target = s.slice(a + 2, bar);
    const label = esc(s.slice(bar + 1, b));
    const k = target.slice(0, target.indexOf(":"));
    const v = target.slice(target.indexOf(":") + 1);
    switch (k) {
      case "p": out += `<a class="link name" data-go="player" data-id="${esc(v)}">${label}</a>`; break;
      case "c": out += `<a class="link name" data-go="coach" data-id="${esc(v)}">${label}</a>`; break;
      case "t": { const [kind, id] = v.split(":"); out += `<a class="link team" data-go="team" data-kind="${esc(kind)}" data-id="${esc(id)}">${label}</a>`; break; }
      case "g": out += `<a class="link" data-go="game" data-index="${esc(v)}">${label}</a>`; break;
      case "d": out += `<a class="link" data-go="calendar" data-args='${esc(JSON.stringify({ day: v }))}'>${label}</a>`; break;
      case "s": {
        const [scr, q] = v.split("?");
        const args = {};
        for (const kv of (q || "").split("&").filter(Boolean)) { const [x, y] = kv.split("="); args[x] = /^-?\d+$/.test(y) ? Number(y) : decodeURIComponent(y || ""); }
        out += linkTo(scr, args, label);
        break;
      }
      default: out += label;
    }
    i = b + 2;
  }
  return out;
}
const plainText = (t) => String(t ?? "").replace(/\[\[[^|\]]*\|([^\]]*)\]\]/g, "$1");
function teamA(t, text) {
  if (!t || !t.ref) return esc(text ?? (t && t.name) ?? "");
  return `<a class="link team" data-go="team" data-kind="${t.ref.kind}" data-id="${t.ref.id}">${esc(text ?? t.name)}</a>`;
}
function teamChip(t, size = 20) {
  if (!t) return "";
  return `<span class="teamcell">${mark(t, size)} ${teamA(t)}</span>`;
}
const stars = (n) => (n ? `<span class="stars" title="${n}-star recruit">${"★".repeat(n)}<i>${"★".repeat(Math.max(0, 5 - n))}</i></span>` : "");

// ---------- page heads and the "Go to" line (L2) ----------
const GOTO = {};
function gotoLine(items) {
  const list = (items || []).filter(Boolean);
  if (!list.length) return "";
  return `<nav class="goto" aria-label="Go to"><span class="st-label">Go to</span>${list.map(([label, scr, args]) => linkTo(scr, args || {}, esc(label), "gochip")).join("")}</nav>`;
}
function gotoFor(screen, args, data) {
  try { return GOTO[screen] ? gotoLine(GOTO[screen](args || {}, data || {})) : ""; } catch (e) { return ""; }
}
function pageHead({ kicker = "", title: t = "", sub = "", actions = "", goto = "" }) {
  return `<header class="phead"><div class="phead-copy">${kicker ? `<div class="st-label">${kicker}</div>` : ""}<div class="h1">${t}</div>${sub ? `<p class="sub">${sub}</p>` : ""}</div>${actions ? `<div class="phead-act">${actions}</div>` : ""}</header>${goto}`;
}
const teamArgs = (a) => (a && a.kind ? { kind: a.kind, id: a.id } : {});
GOTO.roster = (a) => [["Depth chart", "depth"], ["Training", "training"], ["Schedule", "schedule", teamArgs(a)], ["Staff", "staff", teamArgs(a)], ["Team page", "team", teamArgs(a)], ["Leaderboards", "leaders"]];
GOTO.depth = () => [["Squad", "roster"], ["Training", "training"], ["Schedule", "schedule"]];
GOTO.training = () => [["Squad", "roster"], ["Depth chart", "depth"], ["Staff", "staff"]];
GOTO.recruiting = () => [["My players", "myplayers"], ["Player finder", "finder"], ["Recruiting mail", "inbox", { key: "Recruiting" }]];
GOTO.contracts = () => [["Squad", "roster"], ["Money mail", "inbox", { key: "Money" }]];
// The schedule page's own line (update 2, lane 6, B4): Standings, Calendar
// and the next opponent (your game day hub, or that team's page).
GOTO.schedule = (a, x) => {
  const opp = x && x.next_opp;
  const next = opp ? (x.mine ? [`Next opponent: ${opp.name}`, "gameday"] : [`Next opponent: ${opp.name}`, "team", { kind: opp.ref.kind, id: opp.ref.id }]) : null;
  return [["Standings", "standings"], ["Calendar", "calendar"], next, x && x.mine && x.level !== undefined ? ["The year", "year"] : null];
};
GOTO.games = () => [["Schedule", "schedule"], ["Standings", "standings"], ["Leaderboards", "leaders"]];
GOTO.staff = (a) => [["Team page", "team", teamArgs(a)], ["Your profile", "you"]];
GOTO.game = () => [["Game pages", "games"], ["Schedule", "schedule"], ["Standings", "standings"]];
// The first playable's team screens draw their own heads; their Go-to line
// is added under the first heading after they draw.
const GOTO_AFTER = ["roster", "depth", "training", "recruiting", "contracts", "schedule", "games", "staff", "game"];

// ---------- tabs (L4) ----------
function tabs(key, list, cur) {
  return `<div class="tabsline" role="tablist">${list.map((t) => {
    const name = typeof t === "string" ? t : t.name;
    const badge = typeof t === "string" || !t.badge ? "" : ` <small>${esc(t.badge)}</small>`;
    return `<button role="tab" data-act="tab" data-key="${esc(key)}" data-t="${esc(name)}" aria-selected="${name === cur}">${esc(name)}${badge}</button>`;
  }).join("")}</div>`;
}
function tabOf(key, list, args) {
  const k = keep(key, { t: list[0] });
  if (args && args.tab && list.includes(args.tab)) k.t = args.tab;
  if (!list.includes(k.t)) k.t = list[0];
  return k.t;
}

// ---------- the table (S8) ----------
// Per-table settings live in S.keep (this session) and saved views in
// prefs (this browser): columns shown, filters, chips, picked rows.
const TBL = {};
function tableState(key, cols) {
  const t = keep("tbl:" + key, { hidden: null, filters: [], chips: {}, picked: [] });
  if (!t.hidden) t.hidden = cols.filter((c) => c.hide).map((c) => c.k);
  return t;
}
const cellValue = (c, r) => (c.v || ((x) => x[c.k]))(r);
function applyFilters(rows, cols, st) {
  let list = rows;
  for (const f of st.filters) {
    const c = cols.find((c) => c.k === f.k);
    if (!c) continue;
    list = list.filter((r) => {
      const v = cellValue(c, r);
      if (f.op === "has") return plainText(typeof v === "object" && v ? v.name : v ?? "").toLowerCase().includes(String(f.v).toLowerCase());
      const n = Number(v), m = Number(f.v);
      if (Number.isNaN(n) || Number.isNaN(m)) return String(v) === String(f.v);
      return f.op === ">=" ? n >= m : f.op === "<=" ? n <= m : n === m;
    });
  }
  if (st.chips.listed) list = list.filter((r) => r.listed);
  if (st.chips.noted) list = list.filter((r) => r.noted);
  if (st.chips.mine) list = list.filter((r) => r.mine);
  return list;
}
function sortRows(list, cols, s) {
  const cmp = (k, dir) => {
    const c = cols.find((c) => c.k === k);
    if (!c) return () => 0;
    return (a, b) => {
      const x = cellValue(c, a), y = cellValue(c, b);
      if (typeof x === "string" || typeof y === "string") return String(x ?? "").localeCompare(String(y ?? "")) * -dir;
      return ((x ?? -1e9) - (y ?? -1e9)) * dir;
    };
  };
  if (!s.k) return list;
  const one = cmp(s.k, s.dir), two = s.k2 ? cmp(s.k2, s.dir2 || -1) : () => 0;
  return list.slice().sort((a, b) => one(a, b) || two(a, b));
}
function table(key, cols, rows, opts = {}) {
  TBL[key] = { cols, rows, opts };
  const s = keep("sort:" + key, { k: opts.sort || null, dir: -1 });
  const st = tableState(key, cols);
  const shown = cols.filter((c) => !st.hidden.includes(c.k));
  const list = sortRows(applyFilters(rows, cols, st), cols, s);
  const span = shown.length + (opts.select ? 1 : 0);
  // A group spans the columns it names that are showing (hidden ones and
  // all-hidden groups drop out, so the heads stay over their columns).
  let at = 0;
  const groups = opts.groups
    ? `<tr class="groups">${opts.select ? "<th></th>" : ""}${opts.groups.map((g) => {
        const n = cols.slice(at, at + g.span).filter((c) => !st.hidden.includes(c.k)).length;
        at += g.span;
        return n ? `<th colspan="${n}" class="${g.div ? "div" : ""}">${esc(g.t)}${g.who ? ` <span>· ${esc(g.who)}</span>` : ""}</th>` : "";
      }).join("")}</tr>`
    : "";
  const text = (c) => typeof cellValue(c, rows[0] || {}) === "string";
  const arrow = (c) => (s.k === c.k ? ((text(c) ? s.dir > 0 : s.dir < 0) ? "sorted-desc" : "sorted-asc") : s.k2 === c.k ? "sorted-2" : "");
  const pick = opts.select ? `<th class="pickcol" title="Pick up to five to compare"></th>` : "";
  const head = pick + shown.map((c) => `<th data-sort="${key}|${c.k}" class="${c.l ? "l" : ""} ${c.div ? "div" : ""} ${arrow(c)}" title="${esc((c.tip || c.t) + " · click to sort, shift-click for a second sort")}">${esc(c.t)}</th>`).join("");
  let body = "";
  let lastGroup = null;
  for (const r of list) {
    if (opts.groupBy && !s.k) {
      const g = opts.groupBy(r);
      if (g !== lastGroup) {
        body += `<tr class="grouprow"><td class="l" colspan="${span}">${esc(g)}</td></tr>`;
        lastGroup = g;
      }
    }
    const pid = opts.select ? opts.select(r) : null;
    const pickCell = opts.select ? `<td class="pickcol"><input type="checkbox" data-act="pick" data-key="${esc(key)}" data-id="${pid}" ${st.picked.includes(pid) ? "checked" : ""} aria-label="Pick to compare"></td>` : "";
    const chips = (r.listed ? '<i class="chipdot listed" title="On my list"></i>' : "") + (r.noted ? '<i class="chipdot noted" title="Has my note"></i>' : "");
    body += `<tr class="${r.mine ? "mine" : ""} ${r.cls || ""}">${pickCell}${shown.map((c, i) => {
      const inner = c.f ? c.f(r) : esc(num(r[c.k]));
      const stat = c.stat ? ` data-stat="${esc(c.stat)}"` : "";
      return `<td class="${c.l ? "l" : ""} ${c.div ? "div" : ""} ${c.wrap ? "wrap" : ""} ${c.stat ? "statcell" : ""}"${stat}>${inner}${i === 0 && chips ? " " + chips : ""}</td>`;
    }).join("")}</tr>`;
  }
  if (!list.length) body = `<tr><td class="l" colspan="${span}"><div class="empty">${esc(rows.length ? "Nothing matches these filters." : opts.empty || "Nothing here yet.")}</div></td></tr>`;
  const tools = opts.tools === false ? "" : tableTools(key, cols, st, rows, opts);
  return `${tools}<div class="tbl warm box" data-table="${esc(key)}"><table><thead>${groups}<tr>${head}</tr></thead><tbody>${body}</tbody></table></div>`;
}
// The bar over a table: chips, filters, columns, saved views, compare, CSV.
function tableTools(key, cols, st, rows, opts) {
  const has = (k) => rows.some((r) => r[k]);
  const chip = (k, label) => `<button class="tchip" data-act="tchip" data-key="${esc(key)}" data-k="${k}" aria-pressed="${!!st.chips[k]}">${label}</button>`;
  const chips = [has("listed") || st.chips.listed ? chip("listed", '<i class="chipdot listed"></i>On my list') : "", has("noted") || st.chips.noted ? chip("noted", '<i class="chipdot noted"></i>Has my note') : "", opts.mineChip ? chip("mine", "Mine") : ""].join("");
  const filters = st.filters.map((f, i) => `<span class="tfilter">${esc((cols.find((c) => c.k === f.k) || {}).t || f.k)} ${esc(f.op === "has" ? "has" : f.op)} <b>${esc(f.v)}</b><button data-act="tfdel" data-key="${esc(key)}" data-i="${i}" aria-label="Remove filter">×</button></span>`).join("");
  const views = (prefs.views && prefs.views[key]) || [];
  const compare = opts.select && st.picked.length >= 2 ? `<button class="st-btn small primary" data-act="compare" data-key="${esc(key)}">Compare ${st.picked.length}</button>` : opts.select && st.picked.length ? `<span class="muted small">${st.picked.length} picked · up to five</span>` : "";
  return `<div class="ttools">${chips}${filters}<button class="tbtn" data-act="tfilter" data-key="${esc(key)}">+ Filter</button><span class="tspace"></span>${compare}${views.length ? `<select class="tview" data-act="tview" data-key="${esc(key)}"><option value="">Saved views</option>${views.map((v, i) => `<option value="${i}">${esc(v.name)}</option>`).join("")}</select>` : ""}<button class="tbtn" data-act="tsave" data-key="${esc(key)}" title="Keep these columns, filters and sort as a view">Save view</button><button class="tbtn" data-act="tcols" data-key="${esc(key)}">Columns</button>${opts.csv !== false ? `<button class="tbtn" data-act="tcsv" data-key="${esc(key)}" title="Save the table as a CSV file">CSV</button>` : ""}</div>`;
}

function menuAt(el, html, onClick) {
  closeMenu();
  const r = el.getBoundingClientRect();
  const m = document.createElement("div");
  m.className = "popmenu";
  m.innerHTML = html;
  document.body.appendChild(m);
  m.style.left = Math.max(12, Math.min(window.innerWidth - m.offsetWidth - 12, r.left)) + "px";
  m.style.top = Math.max(12, Math.min(window.innerHeight - m.offsetHeight - 12, r.bottom + 6)) + "px";
  m.addEventListener("click", (ev) => { ev.stopPropagation(); if (onClick) onClick(ev); });
  setTimeout(() => document.addEventListener("click", (e) => { if (!m.contains(e.target)) closeMenu(); }, { once: true }), 0);
  return m;
}
function closeMenu() { document.querySelectorAll(".popmenu").forEach((m) => m.remove()); }

async function tableAct(act, el) {
  const key = el.dataset.key;
  const T = TBL[key];
  if (!T) return false;
  const st = tableState(key, T.cols);
  switch (act) {
    case "tchip": st.chips[el.dataset.k] = !st.chips[el.dataset.k]; refresh(el); return true;
    case "tfdel": st.filters.splice(Number(el.dataset.i), 1); refresh(el); return true;
    case "tcols": {
      menuAt(el, `<div class="st-label">Columns</div><div class="mcols">${T.cols.map((c) => `<label class="mrow"><input type="checkbox" data-k="${esc(c.k)}" ${st.hidden.includes(c.k) ? "" : "checked"}> ${esc(c.tip || c.t)}</label>`).join("")}</div>`, (ev) => {
        const cb = ev.target.closest("input[data-k]");
        if (!cb) return;
        const k = cb.dataset.k;
        st.hidden = cb.checked ? st.hidden.filter((x) => x !== k) : [...st.hidden, k];
        const box = document.querySelector(`[data-table="${key}"]`);
        if (box) box.outerHTML = table(key, T.cols, T.rows, { ...T.opts, tools: false });
      });
      return true;
    }
    case "tfilter": {
      const m = menuAt(el, `<div class="st-label">Add a filter</div><div class="fbuild"><select id="fk">${T.cols.map((c) => `<option value="${esc(c.k)}">${esc(c.tip || c.t)}</option>`).join("")}</select><select id="fop"><option value=">=">at least</option><option value="<=">at most</option><option value="=">is</option><option value="has">contains</option></select><input id="fv" placeholder="Value"><button class="st-btn small primary" data-x="add">Add</button></div>`, (ev) => {
        if (!ev.target.closest('[data-x="add"]')) return;
        const v = m.querySelector("#fv").value.trim();
        if (!v) return;
        st.filters.push({ k: m.querySelector("#fk").value, op: m.querySelector("#fop").value, v });
        closeMenu();
        refresh();
      });
      const inp = m.querySelector("#fv");
      inp.focus();
      inp.addEventListener("keydown", (e) => { if (e.key === "Enter") m.querySelector('[data-x="add"]').click(); });
      return true;
    }
    case "tsave": {
      const name = await ask("Save this view", { placeholder: "A name, like \"Senior corners\"", ok: "Save view", note: "Keeps the columns, filters and sort of this table, in this browser." });
      if (!name) return true;
      prefs.views = prefs.views || {};
      (prefs.views[key] = prefs.views[key] || []).push({ name, hidden: st.hidden, filters: st.filters, chips: st.chips, sort: S.keep["sort:" + key] });
      savePrefs();
      toast(`View saved: ${name}`);
      refresh();
      return true;
    }
    case "tcsv": {
      const cols = T.cols.filter((c) => !st.hidden.includes(c.k));
      const list = sortRows(applyFilters(T.rows, T.cols, st), T.cols, keep("sort:" + key, { k: null, dir: -1 }));
      const cell = (c, r) => {
        const v = c.csv ? c.csv(r) : r[c.k];
        const s = plainText(typeof v === "object" && v ? v.name ?? (Array.isArray(v) ? v.join("-") : "") : v ?? "");
        return /[",\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
      };
      const text = [cols.map((c) => c.tip || c.t).join(","), ...list.map((r) => cols.map((c) => cell(c, r)).join(","))].join("\n");
      try { const r = await api("export_text", { name: key, text }); toast("Saved " + r.path); } catch (e) { toast(e.message); }
      return true;
    }
    case "pick": {
      const id = Number(el.dataset.id);
      st.picked = el.checked ? [...st.picked.filter((x) => x !== id), id].slice(-5) : st.picked.filter((x) => x !== id);
      refresh(el);
      return true;
    }
    case "compare": go("compare", { ids: st.picked.slice(0, 5) }); return true;
  }
  return false;
}
document.addEventListener("change", (ev) => {
  const t = ev.target;
  if (!t.dataset) return;
  if (t.dataset.act === "tview" && t.value !== "") {
    const key = t.dataset.key;
    const v = prefs.views[key][Number(t.value)];
    const st = tableState(key, TBL[key].cols);
    Object.assign(st, { hidden: v.hidden || [], filters: v.filters || [], chips: v.chips || {} });
    if (v.sort) S.keep["sort:" + key] = { ...v.sort };
    refresh(t);
  }
});
// Arrow keys move through a table's rows; Enter opens the row (S8).
document.addEventListener("keydown", (ev) => {
  if (!["ArrowDown", "ArrowUp", "Enter"].includes(ev.key) || ev.metaKey || ev.altKey) return;
  if (ev.target.closest && ev.target.closest("input, textarea, select, button")) return;
  if (document.querySelector(".letter-room, .find-room, .overlay, .st-moment.fixed")) return;
  const rows = [...document.querySelectorAll(".view .tbl tbody tr:not(.grouprow):not(.spacer)")];
  if (!rows.length) return;
  let i = rows.findIndex((r) => r.classList.contains("kbd"));
  if (ev.key === "Enter") {
    if (i < 0) return;
    const a = rows[i].querySelector("[data-go]");
    if (a) { ev.preventDefault(); a.click(); }
    return;
  }
  ev.preventDefault();
  if (i >= 0) rows[i].classList.remove("kbd");
  i = ev.key === "ArrowDown" ? Math.min(rows.length - 1, i + 1) : Math.max(0, i < 0 ? 0 : i - 1);
  rows[i].classList.add("kbd");
  rows[i].scrollIntoView({ block: "nearest" });
});

// ---------- the engine-paged table (S8, S9) ----------
// cfg: {key, cmd, args, cols, sort, empty, onMeta}. The game sorts and
// filters; the page draws only the rows in view and asks for more as you
// scroll.
const BIG = {};
function bigTable(cfg) {
  BIG[cfg.key] = { ...cfg, cache: new Map(), total: null, pending: new Set() };
  const s = keep("sort:" + cfg.key, { k: cfg.sort || null, dir: -1 });
  const head = cfg.cols.map((c) => `<th ${c.nosort ? "" : `data-sort="${cfg.key}|${c.k}"`} class="${c.l ? "l" : ""} ${s.k === c.k ? (s.dir < 0 ? "sorted-desc" : "sorted-asc") : s.k2 === c.k ? "sorted-2" : ""}" title="${esc(c.tip || c.t)}">${esc(c.t)}</th>`).join("");
  return `<div class="tbl warm box big" data-big="${esc(cfg.key)}"><table><thead><tr>${head}</tr></thead><tbody><tr><td class="l" colspan="${cfg.cols.length}"><div class="empty">Loading…</div></td></tr></tbody></table></div><div class="bigfoot muted small" data-bigfoot="${esc(cfg.key)}"></div>`;
}
async function bigFill(key) {
  const B = BIG[key];
  const box = document.querySelector(`[data-big="${key}"]`);
  if (!B || !box) return;
  const rowH = 34, page = 100;
  const from = Math.max(0, Math.floor(box.scrollTop / rowH) - 20);
  const to = from + Math.ceil((box.clientHeight || 600) / rowH) + 40;
  const s = S.keep["sort:" + key] || {};
  for (let p = Math.floor(from / page); p <= Math.floor(to / page); p++) {
    if (B.cache.has(p) || B.pending.has(p) || (B.total != null && p * page >= B.total)) continue;
    B.pending.add(p);
    try {
      const r = await api(B.cmd, { ...B.args, offset: p * page, limit: page, sort: s.k || undefined, dir: s.dir, sort2: s.k2 || undefined, dir2: s.dir2 });
      B.total = r.total;
      B.cache.set(p, r.rows);
      if (B.onMeta) B.onMeta(r);
    } catch (e) { toast(e.message); }
    B.pending.delete(p);
  }
  if (!box.isConnected) return;
  const total = B.total ?? 0;
  const cols = B.cols;
  const end = Math.min(total, to);
  let html = from ? `<tr class="spacer" style="height:${from * rowH}px"><td colspan="${cols.length}"></td></tr>` : "";
  for (let i = from; i < end; i++) {
    const r = (B.cache.get(Math.floor(i / page)) || [])[i % page];
    if (!r) { html += `<tr class="spacer" style="height:${rowH}px"><td colspan="${cols.length}"></td></tr>`; continue; }
    const chips = (r.listed ? '<i class="chipdot listed" title="On my list"></i>' : "") + (r.noted ? '<i class="chipdot noted" title="Has my note"></i>' : "");
    html += `<tr class="${r.mine ? "mine" : ""}" style="height:${rowH}px">${cols.map((c) => `<td class="${c.l ? "l" : ""} ${c.stat ? "statcell" : ""}"${c.stat ? ` data-stat="${esc(c.stat)}"` : ""}>${c.f ? c.f(r) : esc(num(r[c.k]))}${c.chips && chips ? " " + chips : ""}</td>`).join("")}</tr>`;
  }
  if (total > end) html += `<tr class="spacer" style="height:${(total - end) * rowH}px"><td colspan="${cols.length}"></td></tr>`;
  box.querySelector("tbody").innerHTML = total ? html : `<tr><td class="l" colspan="${cols.length}"><div class="empty">${esc(B.empty || "Nothing matches.")}</div></td></tr>`;
  const foot = document.querySelector(`[data-bigfoot="${key}"]`);
  if (foot) foot.textContent = total ? `${total.toLocaleString()} row${total === 1 ? "" : "s"} · the game sorts them all` : "";
}
function bigWire() {
  for (const box of document.querySelectorAll("[data-big]")) {
    if (box._wired) continue;
    box._wired = true;
    let t = 0;
    box.addEventListener("scroll", () => { cancelAnimationFrame(t); t = requestAnimationFrame(() => bigFill(box.dataset.big)); }, { passive: true });
    bigFill(box.dataset.big);
  }
}

// ---------- small charts ----------
function spark(values, { w = 160, h = 36, color = "var(--accent)" } = {}) {
  const v = values.filter((x) => x != null);
  if (v.length < 2) return "";
  const lo = Math.min(...v), hi = Math.max(...v), span = hi - lo || 1;
  const pts = values.map((x, i) => `${((i / (values.length - 1)) * w).toFixed(1)},${(h - 3 - ((x - lo) / span) * (h - 6)).toFixed(1)}`).join(" ");
  return `<svg class="spark" viewBox="0 0 ${w} ${h}" width="${w}" height="${h}" aria-hidden="true"><polyline points="${pts}" fill="none" stroke="${color}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>`;
}
function bars(rows, { max } = {}) {
  const m = max || Math.max(1, ...rows.map((r) => r.value));
  return `<div class="bars">${rows.map((r) => `<div class="barrow"><span>${r.label}</span><i style="--w:${Math.round((r.value / m) * 100)}%"></i><b>${esc(r.text ?? r.value)}</b></div>`).join("")}</div>`;
}

// ---------- the face slot (Lane 12 fills it) ----------
function face(p, size = 96) {
  if (typeof window.faceFor === "function") {
    const html = window.faceFor(p, size);
    if (html) return html;
  }
  const c = p.colors || (p.team && p.team.colors) || ["#3a3f4a", "#1c1f27"];
  return `<span class="face sil" style="--s:${size}px;--c1:${c[0]};--c2:${c[1] || c[0]}" aria-hidden="true"><svg viewBox="0 0 100 100"><circle cx="50" cy="38" r="19"/><path d="M12 100c4-25 19-37 38-37s34 12 38 37z"/></svg><b>${esc(p.initials || initials(p.name))}</b></span>`;
}

// ---------- drawn scenes for story posters (9c) ----------
function posterScene(kind, colors) {
  const c1 = (colors && colors[0]) || "#a71930", c2 = (colors && colors[1]) || "#322f2b";
  const id = "g" + Math.random().toString(36).slice(2, 8);
  const ground = `<rect width="320" height="180" fill="#101216"/><rect width="320" height="180" fill="url(#${id})"/>`;
  const field = `<g opacity=".18" stroke="#fff" stroke-width="1.2">${Array.from({ length: 12 }, (_, i) => `<line x1="${i * 34}" y1="0" x2="${i * 34 - 50}" y2="180"/>`).join("")}</g>`;
  const fig = (x, y, s = 1, flip = 1) => `<g transform="translate(${x} ${y}) scale(${s * flip} ${s})" fill="#fff" opacity=".92"><circle cx="0" cy="-48" r="9.5"/><path d="M-11 -37h22l7 27-8 2-5-14-2 22 11 31h-9l-10-27-10 27h-9l10-31 2-22-4 14-8-2z"/></g>`;
  const S2 = {
    game: field + `<ellipse cx="238" cy="66" rx="27" ry="15" fill="#fff" transform="rotate(-30 238 66)"/><path d="M227 71l22-10" stroke="${c1}" stroke-width="2.4"/>` + fig(110, 152, 1.1),
    result: field + `<text x="20" y="128" font-size="86" font-weight="900" fill="#fff" opacity=".14" font-family="Archivo">FINAL</text>` + fig(252, 156, 1.2, -1),
    recruit: `<path d="M150 74c0-23 20-35 46-35s46 12 46 35v6h-92z" fill="${c1}" stroke="#fff" stroke-width="3"/><rect x="146" y="78" width="100" height="10" rx="5" fill="#fff"/><rect x="40" y="132" width="240" height="8" rx="4" fill="#fff" opacity=".3"/><circle cx="196" cy="58" r="5" fill="#fff"/>`,
    portal: `<circle cx="160" cy="92" r="70" fill="none" stroke="#fff" stroke-width="3" opacity=".75"/><circle cx="160" cy="92" r="48" fill="none" stroke="#fff" stroke-width="2" opacity=".45"/><circle cx="160" cy="92" r="26" fill="#fff" opacity=".2"/>` + fig(160, 142, 0.8),
    coach: `<rect x="70" y="40" width="180" height="110" rx="6" fill="#fff" opacity=".1" stroke="#fff" stroke-opacity=".4"/><path d="M96 122l26-32 25 18 30-42 25 30" stroke="#fff" stroke-width="3" fill="none"/><circle cx="96" cy="122" r="6" fill="none" stroke="#fff" stroke-width="3"/><path d="M216 70l18 18M234 70l-18 18" stroke="#fff" stroke-width="3"/>`,
    award: `<path d="M140 40h40v30a20 20 0 0 1-40 0z" fill="#ffd60a"/><path d="M140 48h-14a14 14 0 0 0 14 18M180 48h14a14 14 0 0 1-14 18" stroke="#ffd60a" stroke-width="4" fill="none"/><rect x="154" y="90" width="12" height="22" fill="#ffd60a"/><rect x="138" y="112" width="44" height="10" rx="3" fill="#ffd60a"/>`,
    draft: `<rect x="100" y="34" width="120" height="112" rx="8" fill="#fff"/><text x="160" y="110" text-anchor="middle" font-size="58" font-weight="900" fill="${c1}" font-family="Archivo">1</text><rect x="118" y="50" width="84" height="8" rx="4" fill="${c1}" opacity=".5"/>`,
    weather: `<g stroke="#cde" stroke-width="2" opacity=".55">${Array.from({ length: 28 }, (_, i) => `<line x1="${(i * 37) % 320}" y1="${(i * 23) % 140}" x2="${((i * 37) % 320) - 8}" y2="${((i * 23) % 140) + 22}"/>`).join("")}</g>` + fig(160, 160, 1),
    poll: [70, 110, 52, 130, 90].map((h, i) => `<rect x="${60 + i * 44}" y="${160 - h}" width="28" height="${h}" rx="4" fill="#fff" opacity="${0.45 + i * 0.1}"/>`).join(""),
    office: `<path d="M80 150V72l80-36 80 36v78z" fill="#fff" opacity=".85"/><g fill="#101216">${[100, 130, 160, 190, 220].map((x) => `<rect x="${x - 6}" y="86" width="12" height="54"/>`).join("")}</g>`,
    rise: field + `<path d="M40 150l70-40 50 20 110-90" stroke="#fff" stroke-width="5" fill="none" stroke-linecap="round"/><path d="M250 40h22v22" stroke="#fff" stroke-width="5" fill="none" stroke-linecap="round"/>`,
    practice: field + `<g fill="#ff9f0a">${[70, 120, 170, 220, 270].map((x, i) => `<path d="M${x} ${150 - (i % 2) * 20}l10-26 10 26z"/>`).join("")}</g>` + fig(160, 118, 0.9),
    injury: `<rect x="140" y="44" width="40" height="104" rx="6" fill="#fff"/><rect x="108" y="76" width="104" height="40" rx="6" fill="#fff"/>`,
    record: `<text x="160" y="122" text-anchor="middle" font-size="80" font-weight="900" fill="#fff" font-family="Archivo">#1</text>`,
    fans: Array.from({ length: 15 }, (_, i) => `<circle cx="${14 + i * 21}" cy="${152 - (i % 3) * 9}" r="9" fill="#fff" opacity=".65"/><rect x="${6 + i * 21}" y="${160 - (i % 3) * 9}" width="16" height="30" rx="7" fill="#fff" opacity=".45"/>`).join(""),
    news: `<rect x="70" y="34" width="180" height="118" rx="4" fill="#fff" opacity=".93"/><rect x="84" y="48" width="150" height="15" fill="${c1}"/>${[76, 88, 100, 112, 124, 136].map((y, i) => `<rect x="84" y="${y}" width="${i % 3 === 2 ? 100 : 150}" height="5" rx="2" fill="#8a8f99"/>`).join("")}`,
  };
  return `<svg class="scene" viewBox="0 0 320 180" preserveAspectRatio="xMidYMid slice" aria-hidden="true"><defs><radialGradient id="${id}" cx=".75" cy=".15" r=".95"><stop offset="0" stop-color="${c1}" stop-opacity=".95"/><stop offset=".55" stop-color="${c2}" stop-opacity=".55"/><stop offset="1" stop-color="#101216" stop-opacity="0"/></radialGradient></defs>${ground}${S2[kind] || S2.news}</svg>`;
}

// ---------- hover cards (L5) ----------
const PEEK = { cache: new Map(), timer: 0, hideTimer: 0, el: null, on: null };
function peekForget() { PEEK.cache.clear(); }
function peekHide() {
  clearTimeout(PEEK.timer);
  if (PEEK.el) PEEK.el.remove();
  PEEK.el = null;
  PEEK.on = null;
}
function peekKey(a) {
  const d = a.dataset;
  if (d.go === "player" && d.id) return { kind: "player", id: d.id };
  if (d.go === "coach" && d.id) return { kind: "coach", id: d.id };
  if (d.go === "team" && d.kind) return { kind: "team", id: d.id, tkind: d.kind };
  if (d.go === "game" && d.index != null) return { kind: "game", id: d.index };
  return null;
}
function peekHtml(x) {
  if (x.kind === "player") {
    return `<div class="pk-head">${face({ name: x.name, initials: x.initials, team: x.team }, 52)}<div><b>${esc(x.name)}</b><small>${esc(x.pos)}${x.jersey != null ? ` · #${x.jersey}` : ""} · ${x.team ? esc(x.team.name) : esc(x.where)}</small><small>${esc(x["class"])} · age ${x.age} · ${esc(x.ht)}, ${x.wt} lb ${stars(x.stars)}</small></div></div>
      <div class="pk-body">${x.read ? `<div class="kvline"><span>Your staff's read</span>${range(x.read)}</div>` : ""}${x.yours ? `<div class="kvline"><span>Your own read</span>${yourRange(x.yours)}</div>` : ""}${x.level_rating ? `<div class="kvline"><span>Level</span><b title="${esc(lvlTip(x))}">${esc(x.level_rating)}</b></div>` : ""}${x.last ? `<div class="kvline"><span>Last season</span><b>${esc(x.last)}</b></div>` : ""}${x.status ? `<div class="kvline"><span>Status</span><b>${esc(x.status)}</b></div>` : ""}</div>
      <div class="pk-foot">${x.listed ? '<span class="st-pill">On my list</span>' : ""}${x.noted ? '<span class="st-pill">Has my note</span>' : ""}<span class="muted small">Click to open · shift-click beside · right-click for more</span></div>`;
  }
  if (x.kind === "coach") {
    const j = x.job;
    return `<div class="pk-head">${face({ name: x.name }, 52)}<div><b>${esc(x.name)}</b><small>${j ? `${esc(j.role)}, ${esc(j.name)} since ${j.since}` : "Out of coaching"}</small><small>Age ${x.age} · ${esc(x.reputation)}</small></div></div>
      <div class="pk-body">${j && j.record ? `<div class="kvline"><span>Record</span><b>${esc(j.record)}</b></div>` : ""}<div class="kvline"><span>Came up as</span><b>${esc(x.came_up)}</b></div>${x.titles ? `<div class="kvline"><span>Titles</span><b>${x.titles}</b></div>` : ""}</div>`;
  }
  if (x.kind === "team") {
    const n = x.next;
    return `<div class="pk-head">${mark(x, 44)}<div><b>${esc(x.name)}</b><small>${esc(x.line || "")}</small><small>${esc(x.record || "")}</small></div></div>
      <div class="pk-body">${x.coach ? `<div class="kvline"><span>Head coach</span><b>${esc(x.coach)}</b></div>` : ""}${n && n.opp ? `<div class="kvline"><span>Next</span><b>${n.home ? "vs" : "at"} ${esc(n.opp.name)}${n.date ? ` · ${esc(n.date)}` : ""}</b></div>` : ""}</div>`;
  }
  if (x.kind === "game") return `<div class="pk-head"><div><b>${esc(x.names[1])} ${x.score[1]}, ${esc(x.names[0])} ${x.score[0]}</b><small>${esc(x.date)}</small></div></div>`;
  return "";
}
function peekShow(a, x) {
  if (PEEK.on !== a || !a.isConnected) return;
  if (PEEK.el) PEEK.el.remove();
  const el = document.createElement("div");
  el.className = "peek";
  el.innerHTML = peekHtml(x);
  document.body.appendChild(el);
  const r = a.getBoundingClientRect();
  const w = el.offsetWidth, h = el.offsetHeight;
  let left = r.left, top = r.bottom + 8;
  if (left + w > window.innerWidth - 12) left = window.innerWidth - w - 12;
  if (top + h > window.innerHeight - 12) top = r.top - h - 8;
  el.style.left = Math.max(12, left) + "px";
  el.style.top = Math.max(12, top) + "px";
  el.addEventListener("mouseenter", () => clearTimeout(PEEK.hideTimer));
  el.addEventListener("mouseleave", () => { PEEK.hideTimer = setTimeout(peekHide, 180); });
  PEEK.el = el;
}
document.addEventListener("mouseover", (ev) => {
  if (prefs.hover === "off") return;
  const a = ev.target.closest && ev.target.closest("a[data-go]");
  if (!a || a === PEEK.on || a.closest(".peek, .find-room, .side")) return;
  const k = peekKey(a);
  if (!k) return;
  clearTimeout(PEEK.timer);
  PEEK.on = a;
  PEEK.timer = setTimeout(async () => {
    const ck = JSON.stringify(k);
    let x = PEEK.cache.get(ck);
    if (!x) {
      try {
        x = await api("peek", k.kind === "team" ? { kind: k.tkind, id: Number(k.id) } : { kind: k.kind, id: Number(k.id) });
        if (k.kind === "team") x.kind = "team";
        PEEK.cache.set(ck, x);
      } catch (e) { return; }
    }
    peekShow(a, x);
  }, Number(prefs.hoverDelay) || 450);
});
document.addEventListener("mouseout", (ev) => {
  const a = ev.target.closest && ev.target.closest("a[data-go]");
  if (!a || a !== PEEK.on) return;
  if (ev.relatedTarget && a.contains(ev.relatedTarget)) return;
  clearTimeout(PEEK.timer);
  PEEK.hideTimer = setTimeout(() => { if (PEEK.on === a) peekHide(); }, 180);
});

// ---------- the page beside, and its own window (L6) ----------
function openBeside(screen, args) {
  peekHide();
  S.beside = { screen, args: args || {} };
  drawBeside();
}
function closeBeside() {
  S.beside = null;
  document.querySelectorAll(".beside").forEach((b) => b.remove());
  document.documentElement.classList.remove("has-beside");
}
async function drawBeside(fresh) {
  const b = S.beside;
  if (!b || !screens[b.screen]) return;
  let el = document.querySelector(".beside");
  if (!el) {
    el = document.createElement("aside");
    el.className = "beside";
    el.innerHTML = `<div class="beside-bar"><b class="beside-title"></b><span class="tspace"></span><button class="st-btn small glass" data-act="besidefull">Open full</button><button class="st-icon-btn" data-act="besideclose" aria-label="Close">×</button></div><div class="beside-body"><div class="empty">Loading…</div></div>`;
    document.body.appendChild(el);
    document.documentElement.classList.add("has-beside");
  }
  const body = el.querySelector(".beside-body");
  const top = fresh ? 0 : body.scrollTop;
  const keepTitle = S.title;
  S.besideDrawing = true;
  try {
    const html = await screens[b.screen](b.args);
    if (S.beside !== b) return;
    el.querySelector(".beside-title").textContent = S.title || CRUMBS[b.screen] || b.screen;
    body.innerHTML = html;
    body.scrollTop = top;
    bigWire();
  } catch (e) { body.innerHTML = `<div class="empty">${esc(e.message || e)}</div>`; }
  finally { S.besideDrawing = false; S.title = keepTitle; }
}
function openWindow(screen, args) {
  // The Mac app has one window: ⌘-click opens the page wide beside the
  // screen there (Claude's pick, logged in docs/game.md); a browser opens a
  // real window on the page's own address.
  if (window.__TAURI__) { openBeside(screen, args); document.querySelector(".beside")?.classList.add("wide"); return; }
  const q = Object.entries(args || {}).filter(([, v]) => v != null && typeof v !== "object").map(([k, v]) => `${k}=${encodeURIComponent(v)}`).join("&");
  window.open(`${location.pathname}#${screen}${q ? "?" + q : ""}`, "_blank", "width=1180,height=860");
}

// ---------- right-click on names (S10) ----------
function ctxMenu(ev, items) {
  ev.preventDefault();
  closeMenu();
  const m = document.createElement("div");
  m.className = "popmenu ctx";
  m.innerHTML = items.map((it, i) => (it === "-" ? '<hr>' : `<button data-i="${i}">${esc(it.label)}${it.key ? `<kbd>${esc(it.key)}</kbd>` : ""}</button>`)).join("");
  document.body.appendChild(m);
  m.style.left = Math.min(window.innerWidth - m.offsetWidth - 8, ev.clientX) + "px";
  m.style.top = Math.min(window.innerHeight - m.offsetHeight - 8, ev.clientY) + "px";
  m.addEventListener("click", (e) => { const b = e.target.closest("button[data-i]"); if (!b) return; closeMenu(); items[Number(b.dataset.i)].run(); });
  setTimeout(() => document.addEventListener("click", closeMenu, { once: true }), 0);
}
document.addEventListener("contextmenu", (ev) => {
  const a = ev.target.closest && ev.target.closest("a[data-go]");
  if (!a || !S.status || !S.status.mine) return;
  const d = a.dataset;
  const args = linkArgs(d);
  const name = a.textContent.trim();
  const items = [
    { label: "Open", run: () => go(d.go, args) },
    { label: "Open beside", key: "⇧ click", run: () => openBeside(d.go, args) },
    { label: "Open in a new window", key: "⌘ click", run: () => openWindow(d.go, args) },
  ];
  if (d.go === "player") {
    items.push("-",
      { label: "Add to my players", run: async () => { await api("list_add", { id: args.id }); toast(`${name} is on your list.`); peekForget(); } },
      { label: "Take off my players", run: async () => { await api("list_remove", { id: args.id }); toast(`${name} is off your list.`); peekForget(); } },
      { label: "Write a note…", run: () => noteBox({ kind: "player", id: args.id, name }, `${S.title || "Page"}`) },
      { label: "Compare…", run: () => { const t = keep("cmp", { ids: [] }); t.ids = [...t.ids.filter((x) => x !== args.id), args.id].slice(-5); toast(t.ids.length > 1 ? `${t.ids.length} players to compare. Open Compare from any of them.` : `${name} picked to compare. Pick another.`); if (t.ids.length > 1) go("compare", { ids: t.ids }); } });
  } else if (d.go === "coach") {
    items.push("-", { label: "Write a note…", run: () => noteBox({ kind: "coach", id: args.id, name }, S.title || "Page") });
  } else if (d.go === "team") {
    items.push("-", { label: "Roster", run: () => go("roster", args) }, { label: "Schedule", run: () => go("schedule", args) }, { label: "Write a note…", run: () => noteBox({ kind: "team", id: teamCode(args), name }, S.title || "Page") });
  }
  items.push("-", { label: "Copy name", run: () => navigator.clipboard && navigator.clipboard.writeText(name) });
  peekHide();
  ctxMenu(ev, items);
});
const teamCode = (a) => (a.kind === "nfl" ? a.id : a.kind === "college" ? 100000 + a.id : 1000000 + a.id);

// ---------- search everything (S6, ⌘K) ----------
function openFind(q = "") {
  peekHide();
  if (document.querySelector(".find-room")) { document.querySelector(".find-room input").focus(); return; }
  const room = document.createElement("div");
  room.className = "find-room";
  room.innerHTML = `<div class="find-box"><div class="find-in">${icon("search")}<input placeholder="Players, coaches, teams, towns, games, records, screens, notes, mail" autocomplete="off" spellcheck="false"><kbd>esc</kbd></div><div class="find-out"><div class="find-hint">Type two letters. ↑ ↓ to move, return to open, shift-return to open beside.</div></div></div>`;
  document.body.appendChild(room);
  const inp = room.querySelector("input"), out = room.querySelector(".find-out");
  let seq = 0, items = [], at = 0;
  const close = () => room.remove();
  const open = (it, beside) => {
    close();
    const target = it.kind === "player" ? ["player", { id: it.id }] : it.kind === "coach" ? ["coach", { id: it.id }] : it.kind === "team" ? ["team", it.ref] : it.kind === "game" ? ["game", { index: it.index }] : it.kind === "screen" ? [it.go, {}] : it.kind === "history" ? ["history", it.args] : it.kind === "finder" ? ["finder", it.args] : it.kind === "mail" ? ["inbox", { open: it.id }] : it.kind === "note" ? ["notes", {}] : null;
    if (!target) return;
    if (target[0] === "notes") { openNotes(); return; }
    if (beside) openBeside(target[0], target[1]); else go(target[0], target[1]);
  };
  const draw = (x) => {
    items = [];
    out.innerHTML = x.groups.length ? x.groups.map((g) => `<div class="find-group"><div class="st-label">${esc(g.name)}${g.more ? ` <small>and ${g.more} more</small>` : ""}</div>${g.items.map((it) => { items.push(it); const i = items.length - 1; return `<button class="find-item" data-i="${i}">${it.colors ? mark({ name: it.name, colors: it.colors }, 22) : ""}<span><b>${esc(plainText(it.name))}</b><small>${esc(it.sub || "")}</small></span></button>`; }).join("")}</div>`).join("") : `<div class="find-hint">Nothing matches "${esc(inp.value)}".</div>`;
    at = 0;
    mark_();
  };
  const mark_ = () => out.querySelectorAll(".find-item").forEach((b) => b.classList.toggle("on", Number(b.dataset.i) === at));
  inp.addEventListener("input", () => {
    const n = ++seq;
    clearTimeout(inp._t);
    inp._t = setTimeout(async () => {
      if (inp.value.trim().length < 2) { out.innerHTML = '<div class="find-hint">Type two letters.</div>'; items = []; return; }
      try { const x = await api("find", { q: inp.value }); if (n === seq) draw(x); } catch (e) { /* the world is busy */ }
    }, 110);
  });
  inp.addEventListener("keydown", (e) => {
    if (e.key === "Escape") close();
    else if (e.key === "ArrowDown") { e.preventDefault(); at = Math.min(items.length - 1, at + 1); mark_(); out.querySelector(".find-item.on")?.scrollIntoView({ block: "nearest" }); }
    else if (e.key === "ArrowUp") { e.preventDefault(); at = Math.max(0, at - 1); mark_(); out.querySelector(".find-item.on")?.scrollIntoView({ block: "nearest" }); }
    else if (e.key === "Enter" && items[at]) open(items[at], e.shiftKey);
  });
  out.addEventListener("click", (e) => { const b = e.target.closest(".find-item"); if (b) open(items[Number(b.dataset.i)], e.shiftKey); });
  room.addEventListener("mousedown", (e) => { if (e.target === room) close(); });
  inp.value = q;
  inp.focus();
  if (q) inp.dispatchEvent(new Event("input"));
}
document.addEventListener("focusin", (ev) => {
  if (ev.target.id === "q") { const v = ev.target.value; ev.target.blur(); ev.target.value = ""; openFind(v); }
});

// ---------- notes on anything (10g) ----------
function noteBox(about, page) {
  const pageName = page || S.title || "Page";
  dialog(`<b>A note${about && about.name ? ` on ${esc(about.name)}` : ""}</b><textarea id="nb" placeholder="Only you see your notes." style="min-height:120px"></textarea><small class="sub">Written on: ${esc(pageName)}</small>`, "Save note", async (box) => {
    const text = box.querySelector("#nb").value.trim();
    if (!text) return;
    try {
      await api("note_write", { text, page: pageName, about: about && about.id != null ? { kind: about.kind, id: Number(about.id) } : null, about_name: about ? about.name : "" });
      toast("Note saved.");
      peekForget();
      if (document.querySelector(".notes-sheet")) openNotes();
    } catch (e) { toast(e.message); }
  });
  setTimeout(() => { const t = document.querySelector("#nb"); if (t) t.focus(); }, 30);
}
async function openNotes() {
  let el = document.querySelector(".notes-sheet");
  if (!el) {
    el = document.createElement("aside");
    el.className = "notes-sheet";
    document.body.appendChild(el);
  }
  const x = await api("notes_all");
  const ab = (n) => {
    const a = n.about;
    if (!a || !a.name) return "";
    if (a.kind === "player") return playerLink(a.id, a.name);
    if (a.kind === "coach") return coachLink(a.id, a.name);
    if (a.kind === "team" && a.ref) return `<a class="link" data-go="team" data-kind="${a.ref.kind}" data-id="${a.ref.id}">${esc(a.name)}</a>`;
    return esc(a.name);
  };
  el.innerHTML = `<div class="ns-head"><div><div class="st-label">Your notes</div><b>${x.notes.length} note${x.notes.length === 1 ? "" : "s"}</b></div><span class="tspace"></span><button class="st-btn small primary" data-act="noteadd">Add a note</button><button class="st-icon-btn" data-act="notesclose" aria-label="Close">×</button></div>
    <div class="ns-list">${x.notes.map((n) => `<div class="ns-note" data-src="${n.src}" data-id="${n.id ?? ""}" data-k="${n.k}"><div class="ns-meta"><span>${esc(n.date)}</span><span class="ns-page">${esc(n.page || "")}</span></div>${ab(n) ? `<div class="ns-about">${ab(n)}</div>` : ""}<div class="ns-text" contenteditable="true" spellcheck="true">${esc(n.text)}</div><div class="ns-act"><a class="link" data-act="notesave">Save edit</a><a class="link" data-act="notedel">Delete</a></div></div>`).join("") || '<div class="empty">No notes yet. Write one from any page: right-click a name, or "Add a note" in a letter.</div>'}</div>`;
}

// ---------- the inbox's letter (10a, 10c) ----------
const MAIL = { list: [], at: -1 };
function letterHead(m) {
  const c = m.colors || (S.status && S.status.mine && S.status.mine.colors) || ["#333", "#999"];
  const when = `${esc(m.pretty)}`;
  switch (m.head) {
    case "school":
    case "club": {
      // The sender's own logo when a pack has it (screens/logos.js).
      const lg = typeof logoFor === "function" ? logoFor({ name: m.org }) : null;
      return `<div class="lh lh-school" style="--c1:${c[0]};--c2:${c[1]}">${lg ? `<span class="lh-crest lgo-lh">${logoImg(lg)}</span>` : `<span class="lh-crest">${esc(initials(m.org))}</span>`}<div><b>${esc(m.org)}</b><small>${esc(m.org_line)}</small></div></div><div class="lh-date">${when}</div>`;
    }
    case "conference":
    case "league":
      return `<div class="lh lh-office">${(() => { const lg = typeof compLogo === "function" ? compLogo(m.org) : null; return lg ? `<div class="lh-seal lgo-seal">${logoImg(lg)}</div>` : `<div class="lh-seal">${esc(initials(m.org))}</div>`; })()}<b>${esc(m.org)}</b><small>${esc(m.org_line)}</small></div><div class="lh-date">${when}</div>`;
    case "paper":
      return `<div class="lh lh-paper"><div class="mast">${esc(m.org)}</div><small>${esc(m.org_line)}</small></div>`;
    case "email":
      return `<div class="lh lh-email"><div class="em-row"><span>From</span><b>${esc(m.from)}</b>${m.role ? ` <small>· ${esc(m.role)}</small>` : ""}</div><div class="em-row"><span>To</span>${esc(m.to)}</div><div class="em-row"><span>Date</span>${when}, ${esc(m.time)}</div><div class="em-row"><span>Subject</span><b>${esc(m.subject)}</b></div></div>`;
    default:
      return `<div class="lh lh-memo" style="--c1:${c[0]}"><div class="memo-word">Memorandum</div><small>${esc(m.org)} · ${esc(m.org_line)}</small><div class="memo-grid"><span>To</span><b>${esc(m.to)}</b><span>From</span><b>${esc(m.from)}, ${esc(m.role)}</b><span>Date</span><b>${when}</b><span>Re</span><b>${esc(m.subject)}</b></div></div>`;
  }
}
function letterHtml(m) {
  const paper = m.head === "paper";
  const tableHtml = m.table && m.table.length > 1
    ? `<div class="lt-table"><table><thead><tr>${m.table[0].map((h) => `<th>${esc(h)}</th>`).join("")}</tr></thead><tbody>${m.table.slice(1).map((r) => `<tr>${r.map((c) => `<td>${rich(c)}</td>`).join("")}</tr>`).join("")}</tbody></table></div>`
    : "";
  const formal = !["email", "memo", "paper"].includes(m.head);
  const body = paper
    ? `<div class="clips">${m.body.map((l, i) => (i === 0 ? `<p class="clip-intro">${rich(l)}</p>` : `<article class="clip"><p>${rich(l)}</p></article>`)).join("")}</div>`
    : m.body.map((l) => `<p>${rich(l)}</p>`).join("");
  const sign = m.head === "paper" ? `<p class="lt-sign small muted">${esc(m.sign || "")}</p>` : `<div class="lt-sign">${m.sign ? `<p>${esc(m.sign)}</p>` : ""}${m.head === "email" ? "" : `<p class="sig">${esc(m.from)}</p><small>${esc(m.role)}</small>`}</div>`;
  return `<div class="lt-paper head-${m.head}">${letterHead(m)}${formal ? `<div class="lt-to"><small>To</small> ${esc(m.to)}</div><div class="lt-subject">${esc(m.subject)}</div>` : m.head === "paper" ? `<div class="lt-subject paper">${esc(m.subject)}</div>` : ""}<div class="lt-body">${body}${tableHtml}</div>${sign}${m.attach && m.attach.length ? `<div class="lt-attach">${m.attach.map((a) => `<span class="att">${icon("note")}${esc(a)}</span>`).join("")}</div>` : ""}</div>`;
}
function letterSide(m) {
  const about = m.about;
  const aboutBtn = about ? (about.kind === "player" ? `<button class="st-btn glass" data-act="letterabout" data-go2="player" data-id="${about.id}">Open ${esc(about.name)}</button>` : about.kind === "team" ? `<button class="st-btn glass" data-act="letterabout" data-go2="team" data-args='${esc(JSON.stringify(about.ref))}'>Open ${esc(about.name)}</button>` : about.kind === "game" ? `<button class="st-btn glass" data-act="letterabout" data-go2="game" data-args='${esc(JSON.stringify({ index: about.index }))}'>Open the game page</button>` : about.kind === "coach" ? `<button class="st-btn glass" data-act="letterabout" data-go2="coach" data-id="${about.id}">Open ${esc(about.name)}</button>` : "") : "";
  const answers = m.answers.length
    ? `<div class="lt-answers"><div class="st-label">${m.answered != null ? "Your answer" : "Answer"}${m.answer_by && m.answered == null ? ` · by ${esc(m.answer_by)}` : ""}</div>${m.answers.map((a, i) => `<button class="ans ${m.answered === i ? "picked" : ""}" data-act="answer" data-i="${i}" ${m.answered != null ? "disabled" : ""}><b>${esc(a.label)}</b><small>${esc(a.does)}</small></button>`).join("")}</div>`
    : "";
  return `<div class="lt-side">
    <div class="lt-tools"><button class="st-icon-btn ${m.saved ? "on" : ""}" data-act="letterstar" title="${m.saved ? "Saved: stays until you delete it" : "Save: keeps it past 30 days"}">${icon("star")}</button><button class="st-btn small glass" data-act="letterunread">Mark unread</button><button class="st-btn small glass" data-act="letterdel">Delete</button></div>
    ${m.clears_in != null ? `<div class="muted small">Clears in ${m.clears_in} d unless saved.</div>` : ""}
    ${answers}
    ${aboutBtn}
    <button class="st-btn glass" data-act="letternote">Add a note</button>
    ${m.notes && m.notes.length ? `<div class="lt-notes">${m.notes.map((n) => `<div class="ns-note"><div class="ns-meta"><span>${esc(n.date)}</span></div>${esc(n.text)}</div>`).join("")}</div>` : ""}
    <div class="lt-nav"><button class="st-btn small glass" data-act="letterprev" ${MAIL.at > 0 ? "" : "disabled"}>‹ Newer</button><span class="muted small">${MAIL.at + 1} of ${MAIL.list.length}</span><button class="st-btn small glass" data-act="letternext" ${MAIL.at < MAIL.list.length - 1 ? "" : "disabled"}>Older ›</button></div>
    <div class="muted small">← → to move, esc to close</div>
  </div>`;
}
async function showLetter(id, list) {
  if (list) MAIL.list = list;
  MAIL.at = MAIL.list.indexOf(String(id));
  peekHide();
  // The letter is read before the room opens: an empty room showed its
  // blank paper stack over the inbox until the letter came (minor bug 1).
  const ask = (MAIL.ask = String(id));
  let m;
  try { m = await api("letter", { id: String(id) }); } catch (e) { toast(e.message); closeLetter(); return; }
  if (MAIL.ask !== ask) return;
  MAIL.cur = m;
  let room = document.querySelector(".letter-room");
  const fresh = !room;
  if (!room) {
    room = document.createElement("div");
    room.className = "letter-room";
    room.innerHTML = `<div class="lt-stage"><div class="lt-stack"><i></i><i></i><i></i></div><div class="lt-sheet"></div></div><div class="lt-aside"></div><button class="lt-close st-icon-btn" data-act="letterclose" aria-label="Close">×</button>`;
  }
  const sheet = room.querySelector(".lt-sheet");
  sheet.classList.remove("rise", "turn");
  void sheet.offsetWidth;
  sheet.innerHTML = letterHtml(m);
  sheet.classList.add(fresh ? "rise" : "turn");
  room.querySelector(".lt-aside").innerHTML = letterSide(m);
  if (fresh) {
    document.body.appendChild(room);
    document.documentElement.classList.add("reading");
  }
  room.querySelector(".lt-stage").scrollTop = 0;
  if (S.status) { S.status.unread = m.unread; badge(); }
  const row = document.querySelector(`.mrow[data-id="${id}"]`);
  if (row) row.classList.remove("unread");
}
function closeLetter() {
  document.querySelectorAll(".letter-room").forEach((r) => r.remove());
  document.documentElement.classList.remove("reading");
  if (S.route.screen === "inbox") refresh();
}
function badge() {
  const b = document.querySelector('.nav[data-nav="inbox"] .badge');
  const n = (S.status && S.status.unread) || 0;
  if (b) { if (n) b.textContent = n > 99 ? "99+" : n; else b.remove(); }
  else if (n) { const nav = document.querySelector('.nav[data-nav="inbox"]'); if (nav) nav.insertAdjacentHTML("beforeend", `<em class="badge">${n}</em>`); }
}
document.addEventListener("keydown", (ev) => {
  if (!document.querySelector(".letter-room") || (ev.target.closest && ev.target.closest("input, textarea"))) return;
  if (ev.key === "Escape") { ev.preventDefault(); closeLetter(); }
  if (ev.key === "ArrowRight" || ev.key === "j") { ev.preventDefault(); if (MAIL.at < MAIL.list.length - 1) showLetter(MAIL.list[MAIL.at + 1]); }
  if (ev.key === "ArrowLeft" || ev.key === "k") { ev.preventDefault(); if (MAIL.at > 0) showLetter(MAIL.list[MAIL.at - 1]); }
});

// ---------- the big moments (screens topic 9) ----------
const REVEALS = [];
async function showReveals(n) {
  if (!n) return;
  try { REVEALS.push(...(await api("reveals"))); } catch (e) { return; }
  if (!document.querySelector(".reveal-room")) nextReveal();
}
function nextReveal() {
  const r = REVEALS.shift();
  if (!r) return;
  const c = r.colors || ["#a71930", "#322f2b"];
  const room = document.createElement("div");
  room.className = `reveal-room kind-${r.kind}`;
  room.style.setProperty("--c1", c[0]);
  room.style.setProperty("--c2", c[1] || c[0]);
  const scene = r.kind === "commit" || r.kind === "signing" ? posterScene("recruit", c) : r.kind === "title" ? posterScene("award", c) : r.kind === "pick" ? posterScene("draft", c) : posterScene("coach", c);
  room.innerHTML = `<div class="rv-art">${scene}</div><div class="rv-copy"><div class="st-label">${esc(r.kicker)}</div>${r.number ? `<div class="rv-num">${esc(r.number)}</div>` : ""}<div class="rv-name">${esc(r.name)}</div><div class="rv-sub">${esc(r.sub)}</div>${r.verdict ? `<p class="rv-verdict">${esc(r.verdict)}</p>` : ""}<div class="st-actions">${r.about && r.about.kind === "player" ? `<button class="st-btn primary" data-act="revealopen" data-go2="player" data-id="${r.about.id}">Open his page</button>` : r.about && r.about.kind === "team" && r.about.ref ? `<button class="st-btn primary" data-act="revealopen" data-go2="team" data-args='${esc(JSON.stringify(r.about.ref))}'>Open the team</button>` : ""}<button class="st-btn glass" data-act="revealnext">${REVEALS.length ? "Next" : "Continue"}</button></div><small class="muted">${esc(r.date)}</small></div>`;
  document.body.appendChild(room);
}

// ---------- holiday mode on screen ----------
async function holiday() {
  const { f: box } = dialog(`<b>Go on holiday</b><p class="sub">Your staff runs the team while you're away: they answer what can wait, and the game stops for anything only you can decide.</p>
    <div class="seg hseg">${[[7, "A week"], [14, "Two weeks"], [30, "A month"], [90, "Three months"]].map(([d, t], i) => `<button type="button" data-days="${d}" aria-pressed="${i === 0}">${t}</button>`).join("")}</div>`, "Go on holiday", async (b) => {
    const pressed = b.querySelector('.hseg [aria-pressed="true"]');
    const days = Number(pressed ? pressed.dataset.days : 7);
    const room = document.createElement("div");
    room.className = "holiday-room";
    room.innerHTML = `<div class="hol-sun"></div><div class="hol-copy"><div class="st-label">On holiday</div><div class="h1">Your staff has the keys</div><p class="sub">Back in ${days} days, or sooner if something needs you.</p><div class="st-bar"><i></i></div></div>`;
    document.body.appendChild(room);
    try {
      const r = await api("holiday", { days });
      S.status = r.status;
      toast(`Back from holiday: ${r.days} day${r.days === 1 ? "" : "s"}, now ${r.today}.${r.why && r.why.length ? " " + r.why.map((w) => w.text).slice(0, 2).join(" · ") : ""}`);
      go("home", {}, { root: true });
      showReveals(r.status && r.status.reveals);
    } catch (e) { toast(e.message); }
    room.remove();
  });
  if (box) box.addEventListener("click", (e) => { const t = e.target.closest("[data-days]"); if (!t) return; box.querySelectorAll("[data-days]").forEach((x) => x.setAttribute("aria-pressed", String(x === t))); });
}

// ---------- clicks this lane answers ----------
async function coreAct(act, el, ev) {
  if (!act) return false;
  if (["tchip", "tfdel", "tcols", "tfilter", "tsave", "tcsv", "pick", "compare"].includes(act)) return tableAct(act, el);
  if (CORE_MORE[act]) { CORE_MORE[act](el, ev); return true; }
  const d = el.dataset;
  switch (act) {
    case "tab": {
      const k = keep(d.key, { t: d.t });
      k.t = d.t;
      if (!el.closest(".beside")) S.route.args = { ...S.route.args, tab: d.t };
      refresh(el);
      return true;
    }
    case "holiday": holiday(); return true;
    case "notes": openNotes(); return true;
    case "notesclose": document.querySelector(".notes-sheet")?.remove(); return true;
    case "noteadd": noteBox(null, S.title || "Page"); return true;
    case "notesave":
    case "notedel": {
      const n = el.closest(".ns-note");
      const args = { src: n.dataset.src, k: Number(n.dataset.k) };
      if (n.dataset.id) args.id = Number(n.dataset.id);
      if (act === "notedel") { if (!(await sure("Delete this note?", "It can't be brought back."))) return true; args.delete = true; }
      else args.text = n.querySelector(".ns-text").innerText.trim();
      try { await api("note_edit", args); toast(act === "notedel" ? "Note deleted." : "Note saved."); openNotes(); peekForget(); if (S.route.screen === "player") refresh(); } catch (e) { toast(e.message); }
      return true;
    }
    case "openmail": go("inbox", { open: d.id }); return true;
    case "mshow": keep("mail", {}).show = d.v; refresh(el); return true;
    case "msaved": { const m = keep("mail", {}); m.saved = !m.saved; refresh(el); return true; }
    case "mallread": await api("mail_set", { all: true, read: true }); S.status.unread = 0; refresh(el); return true;
    case "mrow": showLetter(d.id, [...document.querySelectorAll(".mrow[data-id]")].map((r) => r.dataset.id)); return true;
    case "mstar": {
      ev.stopPropagation();
      const on = !el.classList.contains("on");
      await api("mail_set", { id: d.id, saved: on });
      el.classList.toggle("on", on);
      el.closest(".mrow")?.classList.toggle("saved", on);
      return true;
    }
    case "letterclose": closeLetter(); return true;
    case "letterprev": if (MAIL.at > 0) showLetter(MAIL.list[MAIL.at - 1]); return true;
    case "letternext": if (MAIL.at < MAIL.list.length - 1) showLetter(MAIL.list[MAIL.at + 1]); return true;
    case "letterstar": { const m = MAIL.cur; await api("mail_set", { id: m.id, saved: !m.saved }); showLetter(m.id); return true; }
    case "letterunread": { const m = MAIL.cur; const r = await api("mail_set", { id: m.id, read: false }); S.status.unread = r.unread; badge(); closeLetter(); return true; }
    case "letterdel": {
      const m = MAIL.cur;
      const r = await api("mail_set", { id: m.id, delete: true });
      S.status.unread = r.unread;
      badge();
      MAIL.list = MAIL.list.filter((x) => x !== m.id);
      if (MAIL.list.length) showLetter(MAIL.list[Math.min(MAIL.at, MAIL.list.length - 1)]); else closeLetter();
      toast("Deleted.");
      return true;
    }
    case "letternote": noteBox(MAIL.cur.about && MAIL.cur.about.kind !== "game" ? { kind: MAIL.cur.about.kind, id: MAIL.cur.about.kind === "team" ? teamCode(MAIL.cur.about.ref) : MAIL.cur.about.id, name: MAIL.cur.about.name } : null, `Inbox · ${MAIL.cur.subject}`); return true;
    case "letterabout": {
      const args = d.args ? JSON.parse(d.args) : { id: Number(d.id) };
      openBeside(d.go2, args);
      return true;
    }
    case "answer": {
      const m = MAIL.cur;
      try {
        await api("mail_answer", { id: m.id, pick: Number(d.i) });
        toast(`Answered: ${m.answers[Number(d.i)].label}.`);
        S.status = await api("status");
        showLetter(m.id);
        peekForget();
      } catch (e) { toast(e.message); }
      return true;
    }
    case "revealnext": el.closest(".reveal-room").remove(); nextReveal(); return true;
    case "revealopen": {
      el.closest(".reveal-room").remove();
      REVEALS.length = 0;
      go(d.go2, d.args ? JSON.parse(d.args) : { id: Number(d.id) });
      return true;
    }
    case "besideclose": closeBeside(); return true;
    case "besidefull": { const b = S.beside; closeBeside(); go(b.screen, b.args); return true; }
    case "l7pitch": {
      if (!(await sure(`Talk to ${d.name}'s family about your school?`, "Most state associations bar recruiting. In Texas an athletic transfer sits out a year. If it comes out, your school forfeits the games he played and you are suspended for a year.", "Make the pitch"))) return true;
      try { const r = await api("hs_pitch", { id: Number(d.id) }); toast(r.text); refresh(el); } catch (e) { toast(e.message); }
      return true;
    }
    case "l7stake": {
      const id = Number(d.id);
      let pub = await sure(`Say it in public: ${d.name} is your guy?`, "The press hears it. If he hits, the credit is bigger; if he busts, it costs you more. Three years will tell.", "Say it in public");
      if (!pub && !(await sure(`Keep your call on ${d.name} between you and your staff?`, "Your staff knows. A hit still earns their respect; a bust costs a little.", "Keep it private"))) return true;
      try { const r = await api("stake_name", { id, public: pub }); toast(r.text); refresh(el); } catch (e) { toast(e.message); }
      return true;
    }
    case "listadd":
    case "listremove": {
      const id = Number(d.id);
      try { await api(act === "listadd" ? "list_add" : "list_remove", { id }); toast(act === "listadd" ? "Added to my players." : "Taken off my players."); peekForget(); refresh(el); } catch (e) { toast(e.message); }
      return true;
    }
    case "pnote": noteBox({ kind: "player", id: Number(d.id), name: d.name }, `Player page · ${d.name}`); return true;
    case "pcut": {
      if (!(await sure(`Cut ${d.name}?`, d.words, "Cut him"))) return true;
      try { const r = await api("college_cut", { id: Number(d.id) }); toast(r.text); peekForget(); refresh(el); } catch (e) { toast(e.message); }
      return true;
    }
    case "pnotedel": {
      if (!(await sure("Delete this note?", "It can't be brought back."))) return true;
      await api("note_edit", { src: "player", id: Number(d.id), k: Number(d.k), delete: true });
      refresh(el);
      return true;
    }
    case "pnoteedit": {
      const n = el.closest(".pnote");
      const text = n.querySelector(".ns-text").innerText.trim();
      await api("note_edit", { src: "player", id: Number(d.id), k: Number(d.k), text });
      toast("Note saved.");
      return true;
    }
    case "cnote": noteBox({ kind: "coach", id: Number(d.id), name: d.name }, `Coach page · ${d.name}`); return true;
    case "pcmp": { const t = keep("cmp", { ids: [] }); const id = Number(d.id); t.ids = [...t.ids.filter((x) => x !== id), id].slice(-5); if (t.ids.length > 1) go("compare", { ids: t.ids }); else toast("Picked to compare. Pick another player the same way, or tick rows in any table."); return true; }
    case "set": {
      const k = d.k;
      prefs[k] = d.v;
      savePrefs();
      applyPrefs();
      refresh(el);
      return true;
    }
    case "lscope": case "lview": case "lper": {
      const L = keep("leaders", {});
      L[act.slice(1)] = d.v;
      if (act !== "lper") { L.columns = null; S.keep["sort:leaders"] = { k: null, dir: -1 }; }
      refresh(el);
      return true;
    }
    case "lcols": {
      const L = keep("leaders", {});
      const all = (S.leadersMeta && S.leadersMeta.all_columns) || [];
      const cur = new Set((S.leadersMeta && S.leadersMeta.columns.map((c) => c.key)) || []);
      menuAt(el, `<div class="st-label">Columns</div><div class="mcols">${all.map((c) => `<label class="mrow"><input type="checkbox" data-k="${esc(c.key)}" ${cur.has(c.key) ? "checked" : ""}> ${esc(c.long)}</label>`).join("")}</div><button class="st-btn small primary" data-x="ok">Show</button>`, (e) => {
        if (!e.target.closest('[data-x="ok"]')) return;
        L.columns = [...document.querySelectorAll(".popmenu input[data-k]:checked")].map((x) => x.dataset.k);
        closeMenu();
        refresh();
      });
      return true;
    }
    case "lcsv": {
      const L = S.leadersArgs || {};
      const s = S.keep["sort:leaders"] || {};
      try { const r = await api("table_csv", { ...L, table: "leaders", sort: s.k || undefined, dir: s.dir }); toast("Saved " + r.path); } catch (e) { toast(e.message); }
      return true;
    }
    case "fadd": {
      const F = keep("finder", { filters: [] });
      const m = menuAt(el, `<div class="st-label">Add a filter</div><div class="fbuild"><select id="ff">${FINDER_FIELDS.map(([k, t]) => `<option value="${k}">${esc(t)}</option>`).join("")}</select><select id="fo"><option value="=">is</option><option value=">=">at least</option><option value="<=">at most</option></select><input id="fv2" placeholder="Value"><button class="st-btn small primary" data-x="add">Add</button></div>`, (e) => {
        if (!e.target.closest('[data-x="add"]')) return;
        const field = m.querySelector("#ff").value;
        const flag = ["listed", "noted", "mine"].includes(field);
        const v = m.querySelector("#fv2").value.trim();
        if (!v && !flag) return;
        F.filters.push({ field, op: m.querySelector("#fo").value, value: flag ? true : v });
        closeMenu();
        refresh();
      });
      m.querySelector("#fv2").focus();
      return true;
    }
    case "fdel": { const F = keep("finder", { filters: [] }); F.filters.splice(Number(d.i), 1); refresh(el); return true; }
    case "fwatch": {
      const F = keep("finder", { filters: [] });
      const name = await ask("Save this search as a watch", { placeholder: "A name, like \"Texas 4-star corners\"", ok: "Watch", note: "Every Monday your personnel office tells you about players it finds that it hadn't before." });
      if (!name) return true;
      await api("watch_set", { name, query: { filters: F.filters } });
      toast(`Watching: ${name}`);
      return true;
    }
    case "unwatch": await api("watch_set", { remove: Number(d.k) }); refresh(el); return true;
    case "fcsv": {
      const F = keep("finder", { filters: [] });
      try { const r = await api("table_csv", { table: "finder", filters: F.filters }); toast("Saved " + r.path); } catch (e) { toast(e.message); }
      return true;
    }
    case "hslide": { const H = keep("home", { i: 0 }); H.i = Number(d.i); homeSlide(); return true; }
    case "hprev": case "hnext": { const H = keep("home", { i: 0 }); H.i += act === "hnext" ? 1 : -1; homeSlide(); return true; }
    case "newskind": { const N = keep("newsfeed", {}); N.kind = d.v; N.chain = null; S.route.args = { kind: d.v }; refresh(el); return true; }
    case "hist": { const H = keep("history", {}); H[d.k] = d.v; refresh(el); return true; }
    case "ptab": { keep("ptab", {}).t = d.t; refresh(el); return true; }
  }
  return false;
}
// A stat cell opens that stat's leaderboard (S8).
document.addEventListener("click", (ev) => {
  const td = ev.target.closest && ev.target.closest("td.statcell[data-stat]");
  if (!td || ev.target.closest("a, button, input")) return;
  ev.stopPropagation();
  if (S.route.screen === "leaders") { S.keep["sort:leaders"] = { k: td.dataset.stat, dir: -1 }; refresh(); return; }
  go("leaders", { sort: td.dataset.stat });
}, true);
document.addEventListener("change", (ev) => {
  const t = ev.target;
  if (!t.dataset) return;
  if (t.dataset.act === "lsel") {
    const L = keep("leaders", {});
    L[t.dataset.k] = t.value;
    if (t.dataset.k === "scope") { L.year = null; L.conf = null; L.state = null; }
    refresh(t);
  }
  if (t.dataset.act === "hsel") { const H = keep("history", {}); H[t.dataset.k] = t.value; refresh(t); }
  if (t.dataset.act === "mkey") { keep("mail", {}).key = t.value; refresh(t); }
  if (t.dataset.act === "setsel") { prefs[t.dataset.k] = t.value; savePrefs(); applyPrefs(); refresh(t); }
  if (t.dataset.act === "attrgroup") { keep("pattr", {}).g = t.value; refresh(t); }
  if (t.dataset.act === "bdiv") { keep("brackets", {}).division = t.value; refresh(t); }
});
document.addEventListener("input", (ev) => {
  const t = ev.target;
  if (t.id === "mq") {
    clearTimeout(t._t);
    t._t = setTimeout(() => { keep("mail", {}).q = t.value; refresh().then(() => { const n = document.querySelector("#mq"); if (n) { n.focus(); n.setSelectionRange(n.value.length, n.value.length); } }); }, 220);
  }
  if (t.dataset && t.dataset.act === "textsize") { prefs.textSize = t.value; savePrefs(); applyPrefs(); }
});

// After each screen draws: the Go-to line on the first playable's team
// screens, the engine-paged tables, the hero carousel, a letter asked for.
function afterRender() {
  const r = S.route;
  if (GOTO_AFTER.includes(r.screen) && !document.querySelector(".view .goto")) {
    const h = document.querySelector(".view .h1, .view .hero");
    const line = gotoFor(r.screen, r.args);
    if (h && line) (h.classList.contains("hero") ? h : h.parentElement).insertAdjacentHTML("afterend", `<div class="pad goto-pad">${line}</div>`);
  }
  bigWire();
  // Big moments waiting from before this page loaded (a save, a reload).
  if (S.status && S.status.reveals > 0 && !document.querySelector(".reveal-room")) { const n = S.status.reveals; S.status.reveals = 0; showReveals(n); }
  if (r.screen === "home") homeWire();
  if (r.screen === "training" && typeof twAfter === "function") twAfter();
  if (r.screen === "inbox" && r.args.open) { const id = r.args.open; delete r.args.open; showLetter(id, [...document.querySelectorAll(".mrow[data-id]")].map((x) => x.dataset.id)); }
}

// ---------- Home (9a-9e) ----------
let homeTimer = 0;
function homeSlide() {
  const H = keep("home", { i: 0 });
  const slides = [...document.querySelectorAll(".hcar .hs")];
  if (!slides.length) return;
  H.i = ((H.i % slides.length) + slides.length) % slides.length;
  slides.forEach((s, i) => s.classList.toggle("on", i === H.i));
  document.querySelectorAll(".hcar .hdots button").forEach((b, i) => b.setAttribute("aria-current", String(i === H.i)));
}
function homeWire() {
  clearInterval(homeTimer);
  const car = document.querySelector(".hcar");
  if (!car) return;
  homeSlide();
  let hold = false;
  car.addEventListener("mouseenter", () => (hold = true));
  car.addEventListener("mouseleave", () => (hold = false));
  homeTimer = setInterval(() => {
    if (!car.isConnected) return clearInterval(homeTimer);
    if (hold || document.hidden || document.documentElement.classList.contains("calm")) return;
    keep("home", { i: 0 }).i++;
    homeSlide();
  }, 8000);
}
function mailRow(m) {
  const tag = m.answer_by ? `<span class="tag due">Answer by ${esc(m.answer_by)}</span>` : m.answers && m.answered == null ? '<span class="tag">Needs an answer</span>' : "";
  const clears = m.clears_in != null ? `<span class="tag soft">Clears in ${m.clears_in} d</span>` : "";
  return `<div class="mrow ${m.unread ? "unread" : ""} ${m.saved ? "saved" : ""} head-${m.head}" data-act="mrow" data-id="${m.id}" role="button" tabindex="0">
    <span class="mdot"></span><button class="mstar ${m.saved ? "on" : ""}" data-act="mstar" data-id="${m.id}" title="${m.saved ? "Saved" : "Save"}">${icon("star")}</button>
    <span class="mfrom">${esc(m.from)}<small>${esc(m.org)}</small></span>
    <span class="msubj"><b>${esc(m.subject)}</b><small>${esc(m.preview)}</small></span>
    <span class="mtags">${tag}${clears}${(m.keys || []).slice(0, 2).map((k) => `<span class="tag key">${esc(k)}</span>`).join("")}</span>
    <span class="mtime">${esc(m.time)}</span></div>`;
}
// What a hero slide's button opens, in words.
const SLIDE_GO = { game: "Game page", games: "Game pages", schedule: "Schedule", player: "Player page", team: "Team page", coach: "Coach page", standings: "Standings", inbox: "Read it", news: "News", calendar: "Calendar", roster: "Squad", recruiting: "Recruiting", history: "History", leaders: "Leaderboards" };
screens.home = async () => {
  const x = await api("dashboard");
  S.status = x.status;
  // Out of work, home is the job market (lane 10, screens/fo-school.js).
  if (!x.team) return S.status && S.status.you && screens.market ? screens.market() : `<div class="pad">${pageHead({ title: "Home" })}</div>`;
  setColors(x.team.colors);
  title("Home");
  const slides = x.slides.map((s, i) => {
    const c = s.colors || x.team.colors;
    const goBtn = s.go ? linkTo(s.go.screen, s.go.args || {}, esc(s.go.label || SLIDE_GO[s.go.screen] || "Open"), "st-btn glass small") : "";
    const long = s.stat != null && String(s.stat).length > 6 ? " long" : "";
    return `<div class="hs ${i === 0 ? "on" : ""}${long}" style="--c1:${c[0]};--c2:${c[1] || c[0]}"><div class="hs-art">${posterScene(s.kind === "team" ? "game" : s.kind, c)}</div><div class="hs-copy"><div class="st-label">${esc(s.kicker)}</div><h2>${rich(s.title)}</h2><div class="hs-sub">${rich(s.sub || "")}</div><div class="st-actions">${goBtn}</div></div>${s.stat != null ? `<div class="hs-stat"><b>${esc(s.stat)}</b><small>${esc(s.stat_label || "")}</small></div>` : ""}</div>`;
  }).join("");
  const car = `<section class="hcar" aria-roledescription="carousel">${slides}<button class="hnav l" data-act="hprev" aria-label="Previous">${icon("back")}</button><button class="hnav r" data-act="hnext" aria-label="Next">${icon("fwd")}</button><div class="hdots">${x.slides.map((_, i) => `<button data-act="hslide" data-i="${i}" aria-label="Slide ${i + 1}"></button>`).join("")}</div></section>`;
  const stories = x.stories.length
    ? `<div class="posters">${x.stories.map((s) => `<article class="poster ${s.mine ? "mine" : ""}">${posterScene(s.scene, x.team.colors)}<div class="poster-cap"><div class="st-label">${esc(s.kicker)} · ${esc(s.date)}</div><p>${rich(s.text)}</p></div></article>`).join("")}</div>`
    : `<div class="empty">No stories about your team yet. They come with the season.</div>`;
  const week = x.week.length ? `<div class="wk">${x.week.map((e) => `<div class="wk-row ${e.mine ? "mine" : ""}"><span class="wk-when"><b>${e.in === 0 ? "Today" : e.in === 1 ? "Tomorrow" : "In " + e.in + " d"}</b><small>${esc(e.date)}</small></span><span class="wk-kind">${esc(e.kind)}</span><span>${rich(e.text)}</span></div>`).join("")}</div>` : '<div class="empty">A quiet week.</div>';
  const tracked = x.tracked.length
    ? `<div class="trk">${x.tracked.slice(0, 7).map((p) => {
      const then = p.then && p.then.read, now = p.read;
      const d = then && now ? Math.round(mid(now) - mid(then)) : null;
      return `<div class="trk-row"><span>${playerLink(p.id, p.name)}<small>${esc(p.pos)} · ${esc(p.where)}</small></span>${now ? range(now, null, { p: p.id, k: "ovr" }) : `<small class="muted">${esc(p.level)}</small>`}${d != null && d !== 0 ? `<span class="delta ${d > 0 ? "up" : "down"}">${d > 0 ? "▲" : "▼"} ${Math.abs(d)}</span>` : "<span></span>"}</div>`;
    }).join("")}</div>`
    : `<div class="empty">Nobody on your list yet. Right-click any name, or use "Add to my players" on a player's page.</div>`;
  const boss = x.boss;
  const bossCard = `<div class="boss">${face({ name: boss.name }, 44)}<div><b>${esc(boss.name)}</b><small>${esc(boss.role)}${boss.date ? " · " + esc(boss.date) : ""}</small>${boss.line ? `<p>"${esc(boss.line)}"</p>` : '<p class="muted">Nothing from him lately.</p>'}${boss.mail ? `<a class="link small" data-act="openmail" data-id="${boss.mail}">Read the letter</a>` : ""}</div></div>`;
  const mail = x.mail.length ? `<div class="mlist compact">${x.mail.map(mailRow).join("")}</div>` : '<div class="empty">Nothing in the inbox.</div>';
  return `${car}<div class="pad home">
    ${gotoLine([["Inbox", "inbox"], ["Calendar", "calendar"], ["Squad", "roster"], ["Standings", "standings"], ["News", "news"], ["My players", "myplayers"]])}
    <div class="home-grid">
      <div class="col">
        <div class="panel"><h2>Storylines <small>your team, your rivals, your people</small></h2>${stories}</div>
        <div class="panel"><h2>Inbox <small>${S.status.unread || 0} unread · <a class="link" data-nav="inbox">open the inbox</a></small></h2>${mail}</div>
        <div class="panel"><h2>Around football <small>what concerns you</small></h2>${x.around.length ? `<div class="feed">${x.around.map((e) => `<div class="feed-row"><span class="feed-when">${esc(e.date)}</span><span class="feed-kind">${esc(e.kind)}</span><span>${rich(e.text)}</span></div>`).join("")}</div>` : '<div class="empty">Quiet around the league.</div>'}</div>
      </div>
      <div class="col">
        <div class="panel"><h2>The next seven days</h2>${week}</div>
        <div class="panel"><h2>${esc(boss.role)}</h2>${bossCard}</div>
        <div class="panel"><h2>${esc(x.team.name)} in words</h2><div class="words">${x.words.map((w) => `<p>${rich(w)}</p>`).join("")}</div></div>
        <div class="panel"><h2>Tracked players <small><a class="link" data-nav="myplayers">my players</a></small></h2>${tracked}</div>
      </div>
    </div></div>`;
};

// ---------- Inbox (10a-10i) ----------
screens.inbox = async (a) => {
  const M = keep("mail", { show: "all", key: "", q: "", saved: false });
  if (a.key) { M.key = a.key; delete a.key; }
  const x = await api("mail", { show: M.show, key: M.key, q: M.q, saved: M.saved });
  if (S.status) S.status.unread = x.unread;
  title("Inbox");
  let day = null;
  const rows = x.rows.map((m) => {
    const h = m.day !== day ? `<div class="mday">${esc(m.day)}</div>` : "";
    day = m.day;
    return h + mailRow(m);
  }).join("");
  const seg = [["all", "All", x.total], ["unread", "Unread", x.unread], ["read", "Read", x.total - x.unread]].map(([v, t, n]) => `<button data-act="mshow" data-v="${v}" aria-pressed="${M.show === v}">${t}<small>${n}</small></button>`).join("");
  const keys = `<select class="mkey" data-act="mkey"><option value="">Every keyword</option>${x.keywords.map((k) => `<option value="${esc(k.name)}" ${M.key === k.name ? "selected" : ""}>${esc(k.name)} (${k.count})</option>`).join("")}</select>`;
  return `<div class="pad inbox">
    <header class="phead"><div class="phead-copy"><div class="st-label">${x.unread} unread · kept 30 days</div><div class="h1">Inbox</div></div><div class="phead-act"><div class="msearch">${icon("search")}<input id="mq" value="${esc(M.q)}" placeholder="Search subject, sender and letters" autocomplete="off"></div></div></header>
    <div class="mbar"><div class="seg">${seg}</div>${keys}<span class="tspace"></span><button class="tbtn ${M.saved ? "on" : ""}" data-act="msaved">${icon("star")} Saved mails <small>${x.saved}</small></button><button class="tbtn" data-act="notes">${icon("note")} Notes <small>${x.notes}</small></button><button class="tbtn" data-act="mallread">Mark all read</button></div>
    <div class="mlist">${rows || `<div class="empty">${M.q || M.key || M.saved || M.show !== "all" ? "No mail matches." : "No mail yet. It arrives each morning as the days play."}</div>`}</div></div>`;
};

// ---------- the player page (S1-S3) ----------
// Other lanes add tabs: PLAYER_TABS.push({name, after, show(p), draw(p)});
// draw may return a promise of HTML. Lane 11's Development tab sits after
// Stats, Lane 12's face fills the hero through window.faceFor.
const PLAYER_TABS = [];
const ROLE_SHORT = { "High school": "HS", College: "College", NFL: "NFL" };
function seasonsTable(p) {
  if (!p.seasons.length) return '<div class="empty">No season lines yet.</div>';
  return table("pseasons", [
    { k: "year", t: "Year" },
    { k: "level", t: "Level", l: true },
    { k: "team", t: "Team", l: true, f: (r) => (r.team ? teamChip(r.team, 18) : '<span class="muted">Free agent</span>'), v: (r) => (r.team ? r.team.name : "") },
    { k: "pos", t: "Pos", l: true },
    { k: "games", t: "G" },
    { k: "starts", t: "GS", hide: !p.seasons.some((s) => s.starts != null), f: (r) => (r.starts == null ? "" : r.starts) },
    { k: "line", t: "Season", l: true, wrap: true, f: (r) => esc(r.line) },
    { k: "honors", t: "Honors", l: true, v: (r) => [r.all_america && "All-American", r.pro_bowl && "Pro Bowl", r.all_pro && "All-Pro", ...(r.awards || [])].filter(Boolean).join(", "), f: (r) => [r.all_america && '<span class="tag gold">All-American</span>', r.pro_bowl && '<span class="tag gold">Pro Bowl</span>', r.all_pro && '<span class="tag gold">All-Pro</span>', ...(r.awards || []).map((a) => `<span class="tag gold">${esc(a)}</span>`)].filter(Boolean).join(" ") },
  ], p.seasons, { sort: null, tools: false });
}
function measurablesHtml(p) {
  if (!p.measurables.length) return "";
  return `<div class="meas">${p.measurables.map((m) => `<div><b>${esc(m.value)}</b><small>${esc(m.label)}</small><em>${esc(m.at || "")}</em></div>`).join("")}</div>`;
}
// Dynamic Links: the physical gifts your staff sees, strongest first, each
// named by its measured numbers (or a film guess) and where it helps.
function linksHtml(p) {
  if (!p.links || !p.links.length) return "";
  const src = p.links.some((l) => !l.measured) ? "a read from film and his frame until he tests" : "measured";
  return `<div class="panel"><h2>Dynamic Links <small>physical gifts your staff sees · ${esc(src)}</small></h2><div class="items">${p.links.map((l) => `<div class="item" title="${esc(l.edge)}"><span class="when"><span class="tag ${l.strength === "full" ? "gold" : "soft"}">${esc(l.strength)}</span></span><span><b>${esc(l.name)}</b> <small class="muted">${esc(l.numbers)}</small><br><small class="sub">${esc(l.edge)}</small></span></div>`).join("")}</div></div>`;
}
function readCard(p) {
  const pot = p.potential;
  const who = p.mine ? "Your position coach" : "Your scouts";
  return `<div class="readcard"><div class="rc-top"><div><div class="st-label">${esc(who)}'s read</div><div class="rc-tier">${pot ? esc(pot.tier) : p.ovr ? "Rated, not projected" : "Not scouted"}</div></div>${p.level_rating ? `<div class="lvl" title="${esc(lvlTip(p))}"><b>${esc(lvlParts(p.level_rating).n)}</b><small>${esc(lvlParts(p.level_rating).words)}</small></div>` : ""}</div>
    ${pot ? `<p class="sub">Staff ${esc(words(pot.sure))}.${pot.hints && pot.hints.length ? " " + pot.hints.map(esc).join(". ") + "." : ""}</p>` : `<p class="sub">${p.ovr ? "Your staff rates what he is now; nobody has projected what he could become." : "Nobody on your staff has seen him. Only his measurables and public profile are known."}</p>`}
    <div class="kv"><dt>Rating now</dt><dd>${range(p.ovr, who.toLowerCase(), { p: p.id, k: "ovr" })}</dd><dt>Your own read</dt><dd>${yourRange(p.yours, p.yours_looks)}</dd><dt>Mood</dt><dd>${esc(p.mood)}</dd><dt>Status</dt><dd>${esc(p.status || "Available")}</dd>${p.born ? `<dt>Born</dt><dd>${esc(p.born)} <small class="sub">(${p.age})</small></dd>` : ""}${p.on_camera ? `<dt>On camera</dt><dd>${esc(p.on_camera)}</dd>` : ""}${p.followers ? `<dt>Followers</dt><dd>${fans(p.followers)}</dd>` : ""}${p.brand ? `<dt>Ads and deals</dt><dd>${money(p.brand)} <small class="sub">a year, reported</small></dd>` : ""}</div></div>`;
}
const fans = (n) => (n >= 1e6 ? (n / 1e6).toFixed(1) + "M" : n >= 1e4 ? Math.round(n / 1e3) + "k" : n.toLocaleString());
const PT = {
  Overview: (p) => {
    const last = p.seasons[p.seasons.length - 1];
    const fit = p.fit && p.fit.length > 1
      ? `<div class="panel"><h2>Where else he could play <small>your staff's read at each spot</small></h2>${p.fit.map((f) => `<div class="attr"><span>${esc(f.pos)}${f.now ? ' <span class="tag soft">Now</span>' : ""}</span>${range(f.read, null, { p: p.id, k: "at:" + f.pos })}</div>`).join("")}</div>`
      : "";
    return `<div class="grid2"><div class="panel">${readCard(p)}</div>
      <div class="panel"><h2>Five that matter at ${esc(p.pos)} <small>the mental side</small></h2>${p.key_mental.map((a) => `<div class="attr"><span>${esc(a.label)}</span>${range(a.read, null, { p: p.id, k: a.key })}</div>`).join("") || '<div class="empty">Not seen.</div>'}</div></div>
      ${p.measurables.length ? `<div class="panel"><h2>Measurables <small>where each was taken</small></h2>${measurablesHtml(p)}</div>` : ""}
      ${linksHtml(p)}
      <div class="grid2">${last ? `<div class="panel"><h2>Latest season <small>${esc(last.year)} · ${esc(last.level)}</small></h2><p class="bigline">${esc(last.line)}</p>${last.team ? `<p class="sub">${teamChip(last.team, 18)} · ${last.games} game${last.games === 1 ? "" : "s"}</p>` : ""}</div>` : ""}${fit}</div>
      ${p.stories.length ? `<div class="panel"><h2>What happened</h2><div class="items">${p.stories.slice(0, 8).map((s) => `<div class="item story"><span>${rich(s)}</span></div>`).join("")}</div></div>` : ""}`;
  },
  Attributes: (p) => {
    const g = keep("pattr", { g: "" }).g;
    const groups = p.groups.filter((x) => !g || x.name === g);
    const sel = `<select data-act="attrgroup" class="ssel"><option value="">Every group</option>${p.groups.map((x) => `<option ${x.name === g ? "selected" : ""}>${esc(x.name)}</option>`).join("")}</select>`;
    return `<div class="tbar">${sel}<span class="muted small">Ranges are your staff's reads: a narrow bar is a sure read. The outlined bar under one is your own read. Nobody sees a true number.</span></div><div class="attrgrid">${groups.map((x) => `<div class="panel"><h2>${esc(x.name)} <small>${esc(p.mine ? "your coaches" : x.name === "Physical" ? "public measurables" : "your scouts")}</small></h2><div class="attrs">${x.attrs.map((a) => `<div class="attr"><span>${esc(a.label)}</span><span class="l7-pair">${range(a.read, null, { p: p.id, k: a.key })}${a.yours ? yourRange(a.yours, p.yours_looks) : ""}</span></div>`).join("")}</div></div>`).join("")}</div>`;
  },
  Reports: (p) => `<div class="grid2"><div class="panel">${readCard(p)}</div><div class="panel"><h2>Reports on file</h2>${(p.reports || []).length ? `<div class="items">${p.reports.map((r) => `<div class="item"><span class="when">${esc(r.date)}</span><span><b>${esc(r.by || "")}</b> ${rich(r.text)}</span></div>`).join("")}</div>` : `<div class="empty">${p.mine ? "Your position coach grades him every week; his read is the bar on each attribute." : "No written reports yet. Put him on your list and your scouts will keep looking."}</div>`}</div></div>`,
  Stats: (p) => {
    const v = keep("pstats", { v: "Seasons" });
    const seg = ["Seasons", "Game logs", "Splits"].map((t) => `<button data-act="pstatv" data-v="${t}" aria-pressed="${v.v === t}">${t}</button>`).join("");
    let body = "";
    if (v.v === "Seasons") body = seasonsTable(p);
    else if (v.v === "Game logs") body = p.game_logs.length ? table("plogs", [{ k: "date", t: "Date", l: true }, { k: "opp", t: "Opponent", l: true }, { k: "result", t: "Result", l: true, f: (r) => `<a class="link" data-go="game" data-index="${r.index}">${esc(r.result)}</a>` }, { k: "line", t: "His line", l: true, wrap: true }], p.game_logs, { tools: false }) : '<div class="empty">Game logs come from the games you play and watch; he hasn\'t played in one yet.</div>';
    else body = p.splits.length ? table("psplits", [{ k: "name", t: "Split", l: true }, { k: "games", t: "G" }, { k: "yards", t: "Yards a game", f: (r) => r.yards.toFixed(1) }, { k: "td", t: "TD" }, { k: "tackles", t: "Tackles a game", f: (r) => r.tackles.toFixed(1) }], p.splits, { tools: false }) : '<div class="empty">No splits until he plays in your games.</div>';
    return `<div class="tbar"><div class="seg">${seg}</div></div>${body}`;
  },
  Career: (p) => {
    const hon = p.honors;
    const honors = [...(hon ? hon.awards.map((a) => `${a.year} ${a.award}`) : []), ...(hon ? hon.all_america.map((a) => `${a.year} All-American (${a.division})`) : []), ...p.nfl_awards.map((a) => `${a.year} ${a.award}`), ...(hon && hon.hall ? [`College Football Hall of Fame, ${hon.hall}`] : [])];
    const pos = p.positions_detail && p.positions_detail.length > 1 ? `<div class="panel"><h2>Positions</h2><div class="poshist">${p.positions_detail.map((x) => `<span><b>${esc(x.pos)}</b><small>from age ${x.age}</small></span>`).join('<i>→</i>')}</div></div>` : "";
    return `${honors.length ? `<div class="panel"><h2>Honors</h2><div class="honors">${honors.map((h) => `<span class="tag gold">${esc(h)}</span>`).join("")}</div></div>` : ""}${pos}
      <div class="panel"><h2>Year by year</h2>${table("pcareer", [
        { k: "age", t: "Age" }, { k: "level", t: "Level", l: true }, { k: "pos", t: "Pos", l: true }, { k: "role", t: "Role", l: true },
        { k: "team", t: "Team", l: true, f: (r) => (r.team ? logoIn({ name: r.team }) + esc(r.team) : "") }, { k: "missed", t: "Missed" }, { k: "events", t: "What happened", l: true, wrap: true, f: (r) => esc((r.events || []).map(words).join(", ")) },
      ], p.history, { tools: false })}</div>
      ${p.career_story.length ? `<div class="panel"><h2>His story</h2><div class="items">${p.career_story.map((s) => `<div class="item story"><span>${rich(s)}</span></div>`).join("")}</div></div>` : ""}`;
  },
  "History with you": (p) => {
    const w = p.with_you || { seasons: [], contacts: [] };
    return `<div class="grid2"><div class="panel"><h2>Seasons with you</h2>${w.seasons.length ? `<div class="items">${w.seasons.map((s) => `<div class="item"><span class="when">${s.year}</span><span>${teamChip(s.team, 18)}</span></div>`).join("")}</div>` : '<div class="empty">He hasn\'t played for you.</div>'}</div>
      <div class="panel"><h2>What you've done <small>calls, notes, letters</small></h2>${w.contacts.length ? `<div class="items">${w.contacts.map((c) => `<div class="item"><span class="when">${esc(c.date)}</span><span>${esc(c.what)}</span></div>`).join("")}</div>` : '<div class="empty">No contact logged yet. Answers to your mail land here.</div>'}</div></div>`;
  },
  Tendencies: (p) => `<div class="panel"><h2>Tendencies <small>how often he tries things, not how well</small></h2><div class="attrs">${p.tendencies.map((t) => `<div class="attr"><span>${esc(t.label)}</span>${range(t.read)}</div>`).join("")}</div></div>`,
  Recruiting: (p) => {
    const r = p.recruiting;
    const line = (x) => `<div class="item"><span class="when">${esc(x.date || "")}</span><span>${teamChip(x.team, 18)}${x.kind ? ` <small class="muted">${esc(x.kind)}</small>` : ""}</span></div>`;
    return `<div class="st-tiles rtiles"><div><b>${r.stars ? stars(r.stars) : "Unrated"}</b><small>${r.class_of ? "Class of " + r.class_of : "Recruiting stars"}</small></div>${r.camp_stars ? `<div><b>${stars(r.camp_stars)}</b><small>at camps</small></div>` : ""}<div><b>${r.offers.length}</b><small>offers</small></div><div><b>${r.visits.length}</b><small>visits</small></div>${r.all_state ? `<div><b>${esc(r.all_state)}</b><small>honors</small></div>` : ""}</div>
      ${r.commit || r.signed ? `<div class="panel"><h2>Where he's going</h2><div class="items">${r.signed ? `<div class="item"><span class="when">${esc(r.signed.date)}</span><span>Signed with ${teamChip(r.signed.team, 18)}${r.signed.early ? " · early enrollee" : ""}</span></div>` : ""}${r.commit ? `<div class="item"><span class="when">${esc(r.commit.date)}</span><span>Committed to ${teamChip(r.commit.team, 18)}${r.commit.walk_on ? " as a walk-on" : ""}</span></div>` : ""}</div></div>` : ""}
      <div class="grid2"><div class="panel"><h2>Offers</h2>${r.offers.length ? `<div class="items">${r.offers.map(line).join("")}</div>` : '<div class="empty">No offers.</div>'}</div><div class="panel"><h2>Visits</h2>${r.visits.length ? `<div class="items">${r.visits.map(line).join("")}</div>` : '<div class="empty">No visits.</div>'}</div></div>`;
  },
  Contract: (p) => {
    const c = p.pro.contract;
    return `<div class="panel"><h2>Contract</h2>${c ? `<div class="st-tiles"><div><b>${money(c.apy)}</b><small>a year</small></div><div><b>${money(c.total)}</b><small>${c.years} year${c.years === 1 ? "" : "s"} total</small></div><div><b>${money(c.guaranteed)}</b><small>guaranteed</small></div><div><b>${money(c.cap_hit)}</b><small>cap hit this year</small></div><div><b>${c.through}</b><small>through</small></div></div>` : '<div class="empty">No contract.</div>'}
      <div class="kv"><dt>Drafted</dt><dd>${esc(p.pro.draft || "Undrafted")}</dd><dt>Accrued seasons</dt><dd>${p.pro.accrued}</dd><dt>Career earnings</dt><dd>${money(p.pro.earnings)}</dd></div></div>`;
  },
  Notes: (p) => `<div class="panel"><h2>Your notes on ${esc(p.name)} <small>only you see them; they stay with him for his whole career</small></h2>
    <div class="pnotes">${p.notes.map((n) => `<div class="pnote"><div class="ns-meta"><span>${esc(n.date)}</span><span class="tspace"></span><a class="link" data-act="pnoteedit" data-id="${p.id}" data-k="${n.k}">Save edit</a><a class="link" data-act="pnotedel" data-id="${p.id}" data-k="${n.k}">Delete</a></div><div class="ns-text" contenteditable="true" spellcheck="true">${esc(n.text)}</div></div>`).join("") || '<div class="empty">No notes yet.</div>'}</div>
    <button class="st-btn small primary" data-act="pnote" data-id="${p.id}" data-name="${esc(p.name)}">Add a note</button></div>`,
};
function playerTabs(p) {
  const list = ["Overview", "Attributes", "Reports", "Stats", "Career", "History with you"];
  if (p.tendencies.length) list.push("Tendencies");
  if (p.recruiting && (p.recruiting.stars || p.recruiting.offers.length || p.stage === "HighSchool")) list.push("Recruiting");
  if (p.pro) list.push("Contract");
  list.push({ name: "Notes", badge: p.notes.length || "" });
  for (const t of PLAYER_TABS) {
    if (t.show && !t.show(p)) continue;
    const at = list.findIndex((x) => (typeof x === "string" ? x : x.name) === (t.after || "Stats"));
    list.splice(at >= 0 ? at + 1 : list.length, 0, { name: t.name, badge: t.badge ? t.badge(p) : "" });
  }
  return list;
}
// A school named for its town reads as the town on its own ("Prattville,
// AL · Prattville"): say it's the high school.
const hsName = (n) => (/\b(High|Academy|School|Prep|Institute|Catholic|Christian|Military)\b/i.test(n) ? n : `${n} High`);
screens.player = async (a) => {
  const p = await api("profile", { id: a.id });
  const c = p.colors || ["#2a2f3a", "#111"];
  setColors(c);
  title(p.name, p.pos);
  const list = playerTabs(p);
  const names = list.map((t) => (typeof t === "string" ? t : t.name));
  const tab = tabOf("ptab", names, a);
  const extra = PLAYER_TABS.find((t) => t.name === tab);
  const body = extra ? await extra.draw(p) : await PT[tab](p);
  const posHist = (p.positions || []).map(([age, pos]) => `${pos} from age ${age}`).join(" · ");
  const lr = p.level_rating ? `<span class="lvlpill" title="${esc(lvlTip(p))}">${esc(p.level_rating)}</span>` : "";
  const listBtn = p.listed
    ? `<button class="st-btn glass" data-act="listremove" data-id="${p.id}">${icon("star")} On my players</button>`
    : `<button class="st-btn glass" data-act="listadd" data-id="${p.id}">${icon("star")} Add to my players</button>`;
  return `<div class="hero phero" style="--c1:${c[0]};--c2:${c[1] || c[0]}"><div class="art"></div><div class="jersey" aria-hidden="true">${esc(p.jersey != null ? p.jersey : p.pos)}</div>
    <div class="phero-in">${face({ ...p, colors: c }, 132)}<div class="copy">
      <div class="st-label"><span class="pos" title="${esc(posHist || p.pos)}">${esc(p.pos)}</span> · ${esc(p.class)} · ${p.team ? logoIn(p.team) + teamA(p.team) : esc(p.where)}</div>
      <h1>${esc(p.name)}${p.nick ? ` <span class="pnick" title="The name he goes by">“${esc(p.nick)}”</span>` : ""}</h1>
      <div class="meta"><span>Age <b>${p.age}</b></span><span><b>${esc(p.ht)}</b>, <b>${p.wt}</b> lb</span><span>${esc(p.home)}</span>${p.high_school ? `<span>${p.high_school_id != null ? `<a class="link team" data-go="team" data-kind="hs" data-id="${p.high_school_id}">${esc(hsName(p.high_school))}</a>` : esc(hsName(p.high_school))}</span>` : ""}${p.national ? `<span>${esc(p.national)} <span class="muted">U Sports, Canada</span></span>` : ""}${p.stars ? `<span>${stars(p.stars)}</span>` : ""}${p.status ? `<span class="st-pill live">${esc(p.status)}</span>` : ""}${lr}</div>
      <div class="st-actions">${listBtn}<button class="st-btn glass" data-act="pnote" data-id="${p.id}" data-name="${esc(p.name)}">${icon("note")} Add a note</button><button class="st-btn glass" data-act="pcmp" data-id="${p.id}">Compare</button>${l7Stake(p)}${p.college_cut ? `<button class="st-btn glass" data-act="pcut" data-id="${p.id}" data-name="${esc(p.name)}" data-words="${esc(p.college_cut)}" title="Cut him from the team">Cut him</button>` : ""}</div></div></div></div>
    <div class="pad">${gotoFor("player", a, p)}${tabs("ptab", list, tab)}<div class="tabbody">${body}</div></div>`;
};
// His path at your place (lane 7, SK64): when he'd play, who's ahead of
// him the year he arrives, his fit and who teaches him.
PLAYER_TABS.push({
  name: "At our place",
  after: "Overview",
  show: (p) => p.stake && p.stake.can,
  draw: async (p) => {
    let x = null;
    try { x = await api("l7_path", { id: p.id }); } catch (e) { x = null; }
    if (!x) return '<div class="empty">Your staff hasn\'t seen enough of him to project his path here.</div>';
    const ahead = x.ahead.length ? table("l7ahead", [{ k: "name", t: "Ahead of him the year he arrives", l: true, f: (r) => `${playerLink(r.id, r.name)} <small class="sub">${esc(r.class)}</small>` }, { k: "read", t: "Staff read", v: (r) => mid(r.read), f: (r) => range(r.read, null, { p: r.id, k: "ovr" }) }, { k: "leaves", t: "Through" }], x.ahead, { tools: false }) : `<div class="empty">Nobody ahead of him at ${esc(p.pos)} the year he arrives.</div>`;
    return `<div class="grid2"><div class="panel"><h2>His path here <small>arrives ${x.arrives} · ${x.starters} starter${x.starters === 1 ? "" : "s"} at ${esc(p.pos)}</small></h2>
      <p class="bigline">${esc(x.when)}${x.start ? ` <small class="sub">(${x.start})</small>` : ""}</p>${x.transfer_risk ? '<p class="sub">A kid this rated who sits two years often transfers.</p>' : ""}
      <div class="kv"><dt>Fit</dt><dd>${esc(x.fit_words)} <small class="sub">${esc(words(x.scheme))}</small></dd><dt>Who teaches him</dt><dd>${x.teacher ? coachLink(x.teacher.id, x.teacher.name) : "Nobody yet"}${x.takes ? ` · he ${esc(x.takes)}` : ""}</dd><dt>Best for us</dt><dd><b>${Math.round(x.for_us)}</b> <small class="sub">his read, less the years he waits, plus his fit</small></dd></div></div>
      <div class="panel">${ahead}</div></div>`;
  },
});

// Your name on him (lane 7, SK40): the button, or the call you made. At a
// high school, the pitch to his family instead (SK42).
function l7Stake(p) {
  const hp = p.pitch;
  if (hp && hp.on) return `<span class="st-pill ${hp.on.moved ? "good" : "gold"}" title="You talked to his family on ${esc(hp.on.date)}">${hp.on.moved ? "Came to your school" : hp.on.done ? "Stayed put" : "Family deciding by June"}</span>`;
  if (hp && hp.can) return `<button class="st-btn glass" data-act="l7pitch" data-id="${p.id}" data-name="${esc(p.name)}" title="Talk his family into a transfer to your school: most states bar it">Recruit him to your school</button>`;
  const s = p.stake;
  if (!s) return "";
  if (s.on) {
    const how = s.on.done ? (s.on.hit ? `came true: ${s.on.turned}` : `didn't come through: ${s.on.turned}`) : "three years will tell";
    return `<span class="st-pill ${s.on.done ? (s.on.hit ? "good" : "bad") : "gold"}" title="You staked your name on him ${s.on.public ? "in public" : "privately"} on ${esc(s.on.date)}; ${esc(how)}">Your guy${s.on.public ? ", on record" : ""}</span>`;
  }
  return s.can ? `<button class="st-btn glass" data-act="l7stake" data-id="${p.id}" data-name="${esc(p.name)}" title="Mark him as your guy: credit if he hits, a cost if he busts">Stake your name</button>` : "";
}
GOTO.player = (a, p) => [p.team ? ["Squad", "roster", p.team.ref ? { kind: p.team.ref.kind, id: p.team.ref.id } : {}] : null, p.team ? ["Schedule", "schedule", p.team.ref ? { kind: p.team.ref.kind, id: p.team.ref.id } : {}] : null, ["Leaderboards", "leaders"], ["My players", "myplayers"]];

// ---------- compare (S8) ----------
screens.compare = async (a) => {
  const ids = (a.ids || keep("cmp", { ids: [] }).ids || []).slice(0, 5);
  if (ids.length < 2) return `<div class="pad">${pageHead({ title: "Compare", sub: "Pick two to five players: tick rows in any table, or press Compare on a player's page." })}</div>`;
  const x = await api("compare", { ids });
  title("Compare", `${x.players.length} players`);
  const ps = x.players;
  const attrs = new Map();
  ps.forEach((p, i) => p.groups.forEach((g) => g.attrs.forEach((t) => { const k = g.name + "|" + t.label; if (!attrs.has(k)) attrs.set(k, { g: g.name, label: t.label, key: t.key, reads: [] }); attrs.get(k).reads[i] = t.read; })));
  const head = `<tr><th class="l"></th>${ps.map((p) => `<th class="l cmp-h">${face(p, 44)}<div>${playerLink(p.id, p.name)}<small>${esc(p.pos)} · ${esc(p.class)}${p.team ? " · " + esc(p.team.name) : ""}</small></div><button class="st-icon-btn" data-act="cmpdrop" data-id="${p.id}" aria-label="Take out">×</button></th>`).join("")}</tr>`;
  const row = (label, cells) => `<tr><td class="l">${label}</td>${cells.map((c) => `<td class="l">${c}</td>`).join("")}</tr>`;
  let body = row("Rating now", ps.map((p) => range(p.ovr, null, { p: p.id, k: "ovr" }))) + row("Your own read", ps.map((p) => yourRange(p.yours))) + row("Level", ps.map((p) => esc(p.level_rating || ""))) + row("Size", ps.map((p) => `${esc(p.ht)}, ${p.wt} lb`)) + row("Age", ps.map((p) => p.age)) + row("Stars", ps.map((p) => stars(p.stars))) + row("Last season", ps.map((p) => esc(p.last || "")));
  let g = null;
  for (const v of attrs.values()) {
    if (v.g !== g) { body += `<tr class="grouprow"><td class="l" colspan="${ps.length + 1}">${esc(v.g)}</td></tr>`; g = v.g; }
    const best = Math.max(...v.reads.map((r) => (r ? mid(r) : -1)));
    body += `<tr><td class="l">${esc(v.label)}</td>${ps.map((_, i) => `<td class="l ${v.reads[i] && mid(v.reads[i]) === best && best > 0 ? "best" : ""}">${range(v.reads[i], null, { p: ps[i].id, k: v.key })}</td>`).join("")}</tr>`;
  }
  return `<div class="pad">${pageHead({ kicker: "Side by side · your staff's reads", title: "Compare" })}<div class="tbl warm box cmp"><table><thead>${head}</thead><tbody>${body}</tbody></table></div></div>`;
};

// ---------- the coach page and your own (section 6 on screen) ----------
const cap = (t) => String(t || "").replace(/^./, (c) => c.toUpperCase());
const jobLine = (j) => (j ? `${esc(cap(j.role))} · ${j.team ? teamA({ ref: j.team, name: j.name }) : esc(j.name)} since ${j.since}${j.record ? ` · ${esc(j.record)}` : ""}` : "Out of work");
function treeHtml(list, empty) {
  return list.length ? `<div class="tree">${list.map((t) => `<div class="tree-node">${face({ name: t.name }, 34)}<span>${coachLink(t.id, t.name)}<small>${t.job ? esc(`${t.job.role}, ${t.job.name}`) : "Out of work"}${t.reputation ? " · " + esc(t.reputation) : ""}</small></span></div>`).join("")}</div>` : `<div class="empty">${empty}</div>`;
}
screens.coach = async (a) => {
  const c = await api("coach_page", { id: a.id });
  title(c.name, c.job ? c.job.role : "Coach");
  // Your read of him (update 2, board 18) comes with having him on your staff.
  const names = ["Profile", ...(c.your_staff ? ["Ratings"] : []), ...(c.record ? ["Recruiting record"] : []), "Jobs", "Coaching tree", "Players he developed", "Notes"];
  const tab = tabOf("ctab", names, a);
  let body = "";
  if (tab === "Ratings") {
    body = await rrCoachTab(a.id);
  } else if (tab === "Profile") {
    body = `<div class="grid2"><div class="panel"><h2>Who he is</h2><div class="kv"><dt>Age</dt><dd>${c.age}</dd><dt>Standing</dt><dd>${esc(c.reputation)}</dd>${c.renown ? `<dt>Name</dt><dd>${esc(c.renown)}</dd>` : ""}<dt>Came up as</dt><dd>${esc(c.came_up)}</dd><dt>As a player</dt><dd>${c.played ? esc(c.played.replace(/^Played: /, "")) : c.ex_pro ? "Played pro" : "Didn't play college football"}</dd><dt>Titles</dt><dd>${c.titles}</dd>${c.ats ? `<dt>Against the spread</dt><dd>${esc(c.ats)}</dd>` : ""}${c.mentor_line ? `<dt>Learned under</dt><dd>${rich(c.mentor_line)}</dd>` : ""}</div></div>
      <div class="panel"><h2>Contract <small>as reported</small></h2>${c.contract ? `<div class="st-tiles"><div><b>${money(c.contract.salary)}</b><small>a year</small></div><div><b>${c.contract.through}</b><small>through</small></div><div><b>${c.contract.buyout > 0 ? money(c.contract.buyout) : "None"}</b><small>buyout now</small></div></div>` : '<div class="empty">No contract reported.</div>'}</div></div>`;
  } else if (tab === "Recruiting record") {
    // His calls on recruits for you (lane 7, SK30): whom he wanted, whom he
    // passed on, and how they turned out three years on.
    const r = c.record;
    body = `<div class="panel"><h2>His word on recruits <small>${r.wanted} wanted · ${r.passed} passed · you overruled him ${r.overruled} time${r.overruled === 1 ? "" : "s"}${r.settled ? ` · right on ${r.right} of ${r.settled} settled` : ""}</small></h2>${r.heart ? '<p class="sub">Overruled again and again lately: he works his area with less heart and listens to other offers.</p>' : ""}
      ${table("crec", [{ k: "date", t: "When", l: true }, { k: "name", t: "Recruit", l: true, f: (x) => `${playerLink(x.id, x.name)} <small class="sub">${esc(x.pos)}</small>` }, { k: "call", t: "His call", l: true }, { k: "you", t: "You", l: true, f: (x) => (x.you === "Overruled" ? '<span class="st-pill warn">Overruled</span>' : "Agreed") }, { k: "turned", t: "Three years on", l: true, wrap: true, f: (x) => (x.turned ? `${esc(x.turned)}${x.right === true ? ' <span class="st-pill good">He was right</span>' : x.right === false ? ' <span class="st-pill bad">He was wrong</span>' : ""}` : '<span class="muted">Too soon</span>') }], r.rows, { tools: false })}</div>`;
  } else if (tab === "Jobs") {
    body = table("cjobs", [{ k: "where", t: "Where", l: true }, { k: "role", t: "Role", l: true }, { k: "from", t: "From" }, { k: "to", t: "To" }, { k: "record", t: "Record", l: true }, { k: "ended", t: "How it ended", l: true }], c.history, { tools: false });
  } else if (tab === "Coaching tree") {
    body = `<div class="panel"><h2>Coaches who learned under him <small>${c.tree.length}</small></h2>${treeHtml(c.tree, "Nobody has come up under him yet.")}</div>`;
  } else if (tab === "Players he developed") {
    body = c.developed.length ? table("cdev", [{ k: "name", t: "Player", l: true, f: (r) => playerLink(r.id, r.name) }, { k: "pos", t: "Pos", l: true }, { k: "year", t: "Drafted" }, { k: "round", t: "Rd" }, { k: "pick", t: "Pick" }, { k: "team", t: "By", l: true, f: (r) => teamChip(r.team, 18), v: (r) => r.team.name }], c.developed, { tools: false }) : '<div class="empty">No player he coached has been drafted yet.</div>';
  } else {
    body = `<div class="panel"><h2>Your notes on ${esc(c.name)}</h2>${(c.notes || []).length ? `<div class="items">${c.notes.map((n) => `<div class="item"><span class="when">${esc(n.date)}</span><span>${esc(n.text)}</span></div>`).join("")}</div>` : '<div class="empty">No notes yet.</div>'}<button class="st-btn small primary" data-act="cnote" data-id="${c.id ?? a.id}" data-name="${esc(c.name)}">Add a note</button></div>`;
  }
  return `<div class="pad">${pageHead({ kicker: jobLine(c.job), title: `${esc(c.name)}${c.you ? ' <span class="st-pill gold">You</span>' : ""}`, actions: `<button class="st-btn small glass" data-act="cnote" data-id="${a.id}" data-name="${esc(c.name)}">${icon("note")} Add a note</button>` })}${gotoFor("coach", a, c)}${tabs("ctab", tab === "Notes" ? names : [...names.slice(0, -1), { name: "Notes", badge: (c.notes || []).length || "" }], tab)}<div class="tabbody">${body}</div></div>`;
};
GOTO.coach = (a, c) => [c.job && c.job.team ? ["His team", "team", c.job.team] : null, c.job && c.job.team ? ["His staff", "staff", c.job.team] : null, ["Coaching carousel", "history", { tab: "Coaching carousel" }]];

const STYLE = [["risk", "Risk", "Conservative", "Aggressive"], ["freedom", "Structure or freedom", "Do your job", "Freelance"], ["physical", "Physical", "Protective", "Physical"], ["ball_hunting", "Ball hunting", "Sound", "Ball hunting"], ["mould", "Mould or adapt", "Adapt to him", "Mould him"]];
screens.you = async (a) => {
  const y = await api("career_page");
  if (!y) return `<div class="pad">${pageHead({ title: "No career yet" })}</div>`;
  title("Your profile");
  const row = (t, v) => `<div class="attr"><span>${esc(t)}</span><span class="rg"><span class="t"><i style="left:0;width:${(v / 20) * 100}%;background:${rateColor(v * 5)}"></i></span><b>${v.toFixed(1)}</b></span></div>`;
  const x = y.extra;
  const p = y.profile || {};
  const tab = tabOf("ytab", ["Profile", "Contract", "Coaching tree", "Job market", "Jobs"], a);
  let body = "";
  if (tab === "Profile") {
    const from = [p.hometown, y.home_state].filter(Boolean).join(", ");
    const school = [p.alma_mater, p.college_years].filter(Boolean).join(", ") + (p.degree ? ` (${p.degree})` : "");
    const yr = Number(String(S.status && S.status.today).slice(-4));
    const kids = (p.children || []).map(([n, b]) => `${n}, ${Math.max(0, yr - b)}`).join("; ");
    const family = [p.status, p.status === "Married" && p.spouse ? `to ${p.spouse}` : ""].filter(Boolean).join(" ") + (kids ? `; children: ${kids}` : "");
    body = `<div class="grid2"><div class="panel"><h2>You</h2><div class="kv"><dt>Age</dt><dd>${y.age}</dd>${y.road ? `<dt>Came up as</dt><dd>${esc(y.road)}</dd>` : ""}<dt>Reputation</dt><dd>${esc(y.reputation)}${y.stock && y.stock !== y.reputation ? ` · <span class="muted">${esc(y.stock)}</span>` : ""}</dd><dt>Record</dt><dd>${y.record === "0-0" && !(S.status && S.status.seat && S.status.seat.head) ? '<span class="sub">No games as a head coach yet</span>' : esc(y.record)}</dd><dt>Titles</dt><dd>${y.titles}</dd>${x && x.calls ? `<dt>Calls the plays</dt><dd>${esc(x.calls)}</dd>` : ""}${from ? `<dt>From</dt><dd>${esc(from)}${p.high_school ? ` · ${esc(p.high_school)}` : ""}</dd>` : ""}${school.trim() ? `<dt>College</dt><dd>${esc(school)}</dd>` : ""}${family.trim() ? `<dt>Family</dt><dd>${esc(family)}</dd>` : ""}${y.mentor ? `<dt>Learned under</dt><dd>${coachLink(y.mentor.id, y.mentor.name)}${y.mentor.job ? ` <small class="muted">${esc(y.mentor.job.role)}, ${esc(y.mentor.job.name)}</small>` : ""}</dd>` : ""}</div></div>
      <div class="panel"><h2>Teaching <small>your own ratings, 1-20; nobody else sees them</small></h2>${y.teaching.map((t) => row(cap(t.group), t.value)).join("")}</div></div>
      <div class="grid2"><div class="panel"><h2>The rest</h2>${[["Scheme", y.scheme], ["Game management", y.game_management], ...(x ? [["Play calling", x.play_calling], ["In-game adaptation", x.adaptation]] : []), ["Motivation", y.motivation], ["Discipline", y.discipline], ["Relationships", y.relationships], ["Recruiting", y.recruiting], ...(x ? [["Evaluation", x.evaluation]] : []), ["Network", y.network], ...(x ? [["Negotiating", x.negotiating]] : [])].filter((r) => r[1] != null).map(([t, v]) => row(t, v)).join("")}</div>
      ${y.style ? `<div class="panel"><h2>Your coaching style <small>1-20</small></h2>${STYLE.map(([k, t, lo, hi]) => `<div class="attr"><span>${esc(t)} <small class="sub">${esc(lo)} to ${esc(hi)}</small></span><span class="rg"><span class="t"><i style="left:0;width:${(y.style[k] / 20) * 100}%;background:${rateColor(y.style[k] * 5)}"></i></span><b>${Math.round(y.style[k])}</b></span></div>`).join("")}</div>` : ""}</div>${typeof youLife === "function" ? youLife(y) : ""}`;
  } else if (tab === "Contract") {
    const c = y.contract;
    body = c ? `<div class="panel"><h2>Your contract</h2><div class="st-tiles"><div><b>${money(c.salary)}</b><small>a year</small></div><div><b>${c.through}</b><small>through</small></div><div><b>${money(c.buyout)}</b><small>what firing you costs now</small></div><div><b>${Math.round((c.buyout_share || 0) * 100)}%</b><small>of what's left is guaranteed</small></div></div><p class="sub">Your boss reads your contract with your results: a big buyout buys patience, a small one doesn't.</p></div>` : '<div class="empty">No written contract: high school coaches work year to year.</div>';
  } else if (tab === "Coaching tree") {
    body = `<div class="panel"><h2>Coaches who learned under you</h2>${treeHtml(y.tree, "Nobody has come up under you yet. Assistants who work for you a few years join your tree when they move on.")}</div>`;
  } else if (tab === "Job market") {
    body = `${screens.market ? `<div class="sub" style="margin-bottom:12px">${linkTo("market", {}, "Open the job market", "st-btn small primary")} to apply, put your name in and answer offers.</div>` : ""}<div class="panel"><h2>Head jobs that opened <small>this year and last · your stock: ${esc(y.stock)}</small></h2>${y.market.length ? table("ymarket", [{ k: "team", t: "Job", l: true, f: (r) => teamChip(r.team, 18), v: (r) => r.team.name }, { k: "line", t: "Where", l: true }, { k: "prestige", t: "Program", l: true }, { k: "date", t: "Opened", l: true }, { k: "filled", t: "Filled by", l: true, f: (r) => (r.filled ? coachLink(r.filled.id, r.filled.name) : '<span class="tag due">Open</span>'), v: (r) => (r.filled ? r.filled.name : "") }], y.market, { tools: false }) : '<div class="empty">No head jobs have opened yet. Black Monday and the end of the college season move the carousel.</div>'}</div>`;
  } else {
    body = y.history && y.history.length ? table("yjobs", [{ k: "where", t: "Where", l: true }, { k: "role", t: "Role", l: true }, { k: "from", t: "From" }, { k: "to", t: "To" }, { k: "record", t: "Record", l: true }], y.history, { tools: false }) : '<div class="empty">This is your first job.</div>';
  }
  return `<div class="pad">${pageHead({ kicker: esc(y.title || `Head coach since ${y.started}`), title: esc(y.name), sub: esc(y.record) })}${gotoLine([["Your staff", "staff"], ["Your boss's letters", "inbox", { key: "Your job" }], ["Coaching carousel", "history", { tab: "Coaching carousel" }]])}${tabs("ytab", ["Profile", "Contract", "Coaching tree", "Job market", "Jobs"], tab)}<div class="tabbody">${body}</div></div>`;
};

// ---------- the team page ----------
screens.team = async (a) => {
  const t = await api("team_page", a.kind ? { kind: a.kind, id: a.id } : {});
  setColors(t.colors);
  title(t.name, t.level);
  const pro = t.ref.kind === "nfl";
  const ta = { kind: t.ref.kind, id: t.ref.id };
  const seasons = t.seasons && t.seasons.length
    ? pro
      ? table("tseasons", [{ k: "year", t: "Season", f: (s) => linkTo("schedule", { ...ta, season: s.year }, String(s.year)) }, { k: "record", t: "Record", l: true }, { k: "coach", t: "Head coach", l: true }, { k: "note", t: "Playoffs", l: true }], t.seasons, { tools: false })
      : table("tseasons", [{ k: "year", t: "Season", f: (s) => linkTo("schedule", { ...ta, season: s.year }, String(s.year)) }, { k: "record", t: "Record", l: true }, { k: "coach", t: "Coach", l: true }, { k: "pts", t: "Points", l: true, f: (s) => (s.g ? `${s.pf}-${s.pa}` : "") }, { k: "titles", t: "Titles", l: true, wrap: true, f: (s) => [s.conf ? "Conference" : "", s.nat ? (t.ref.kind === "hs" ? "State" : "National") : ""].filter(Boolean).join(", ") }], t.seasons, { tools: false })
    : `<div class="empty">${pro ? "The league's book of seasons starts with this world's next season." : "No seasons kept."}</div>`;
  const leaders = (t.leaders || []).length ? `<div class="panel"><h2>Last season's leaders</h2><div class="items">${t.leaders.map((l) => `<div class="item"><span class="when">${esc(l.label)}</span><span>${playerLink(l.id, l.name)} <small class="muted">${esc(l.pos)}</small><br><small>${esc(l.line)}</small></span></div>`).join("")}</div></div>` : "";
  const news = t.news.length ? `<div class="items">${t.news.map((n) => `<div class="item"><span class="when">${esc(n.date)}</span><span><small class="muted">${esc(n.kind)}</small> ${rich(n.text)}</span></div>`).join("")}</div>` : '<div class="empty">Nothing in the last two months.</div>';
  return `<div class="hero" style="--c1:${t.colors[0]};--c2:${t.colors[1] || t.colors[0]}"><div class="art"></div>${(typeof logoHero === "function" && logoHero(t)) || `<div class="st-outline">${esc(initials(t.name))}</div>`}<div class="copy"><div class="st-label">${esc(t.level)}</div><h1>${esc(t.name)}</h1>
    <div class="meta"><span><b>${esc(t.record)}</b> ${esc(t.record_note || "this season")}</span>${t.place ? `<span>${esc(t.place)}</span>` : ""}${t.prestige ? `<span>${esc(t.prestige)}</span>` : ""}${t.stadium ? `<span>${Number(t.stadium).toLocaleString()} seats</span>` : ""}${t.coach ? `<span>Head coach ${coachLink(t.coach.id, t.coach.name)}</span>` : ""}</div>
    <div class="st-actions">${linkTo("roster", ta, "Roster", "st-btn glass")}${linkTo("schedule", ta, "Schedule", "st-btn glass")}${linkTo("staff", ta, "Staff", "st-btn glass")}${logoChange(t)}</div></div></div>
    <div class="pad">${gotoFor("team", a, t)}<div class="grid2"><div class="col"><div class="panel"><h2>Lately</h2>${news}</div>${leaders}</div>
    <div class="col"><div class="panel"><h2>Seasons</h2>${seasons}</div>${t.rivals.length ? `<div class="panel"><h2>${pro ? "Division" : t.ref.kind === "hs" ? "Its group" : "Rivals"}</h2><div class="chips">${t.rivals.map((r) => teamChip(r, 20)).join("")}</div></div>` : ""}</div></div></div>`;
};
GOTO.team = (a, t) => [["Standings", "standings"], t.ref && t.ref.kind === "college" ? ["Leaderboards", "leaders"] : null, ["History", "history"]];

// ---------- news feeds (topic 9 on screen) ----------
const FEEDS = [["beat", "Beat writers"], ["national", "National"], ["recruits", "Recruiting"], ["level", "Your level"], ["fans", "Fans"]];
screens.news = async (a) => {
  const N = keep("newsfeed", { kind: "beat", chain: null });
  if (a.kind) N.kind = a.kind;
  let x = await api("feed", { kind: N.kind, chain: N.chain || undefined, limit: 120 });
  // A quiet feed you didn't pick yourself: open the first one with stories.
  if (!x.rows.length && !a.kind && !N.chain) {
    for (const [k] of FEEDS.filter(([k]) => k !== N.kind)) {
      const y = await api("feed", { kind: k, limit: 120 });
      if (y.rows.length) { N.kind = k; x = y; break; }
    }
  }
  title("News");
  const seg = FEEDS.map(([k, t]) => `<button data-act="newskind" data-v="${k}" aria-pressed="${N.kind === k}">${t}</button>`).join("");
  const rows = x.rows.map((r) => `<article class="story ${r.mine ? "mine" : ""} loud-${esc(r.loud || "small")}"><div class="story-art">${posterScene(r.scene || "news", (S.status && S.status.mine && S.status.mine.colors) || ["#3b4252", "#1c1f27"])}</div><div class="story-copy"><div class="story-meta"><span class="st-label">${esc(r.kind)}</span><span class="cert cert-${esc((r.certainty || "").toLowerCase())}" title="How sure the source is">${esc(r.certainty || "")}</span><span class="muted">${esc(r.source || "")} · ${esc(r.date)}</span></div><p>${rich(r.text)}</p>${r.chain ? `<a class="link small" data-act="newschain" data-k="${esc(r.chain.key)}">${r.chain.count} more in this story</a>` : ""}</div></article>`).join("");
  const chain = N.chain ? `<div class="chainbar">Following one story <a class="link" data-act="newschain" data-k="">Show everything</a></div>` : "";
  return `<div class="pad">${pageHead({ kicker: "Rumours are marked as rumours", title: "News", actions: `<div class="seg">${seg}</div>` })}${chain}<div class="stories">${rows || `<div class="empty">${N.kind === "beat" ? "Your beat writers haven't filed anything yet." : "Nothing in this feed yet."}</div>`}</div></div>`;
};
GOTO.news = () => [["Inbox", "inbox"], ["Calendar", "calendar"], ["Standings", "standings"]];

// ---------- the calendar (section 4 on screen) ----------
screens.calendar = async (a) => {
  const C = keep("calmonth", {});
  if (a.day) { C.day = a.day; C.month = null; }
  const x = await api("calendar_month", C.day ? { day: C.day } : C.month ? { month: C.month } : {});
  title("Calendar", x.title);
  const head = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"].map((d) => `<div class="cal-dh">${d}</div>`).join("");
  const cell = (d) => {
    const g = d.game;
    const res = g && g.score ? (g.score[0] > g.score[1] ? "W" : g.score[0] < g.score[1] ? "L" : "T") : "";
    const game = g ? `<div class="cal-game ${res ? "r" + res : ""}" title="${esc(g.at + " " + g.opp.name)}">${g.score ? `<b>${res} ${g.score[0]}-${g.score[1]}</b>` : ""}<span>${mark(g.opp, 14)} ${esc(g.at)} ${esc(g.opp.name)}</span></div>` : "";
    return `<div class="cal-d ${d.in_month ? "" : "out"} ${d.today ? "today" : ""} ${d.past ? "past" : ""} ${C.day === d.iso ? "picked" : ""}" data-act="calday" data-iso="${d.iso}"><span class="cal-n">${d.day}</span>${game}${d.items.map((e) => `<div class="cal-e ${e.mine ? "mine" : ""} loud-${e.loud.toLowerCase()}" title="${esc(plainText(e.text))}">${esc(plainText(e.text))}</div>`).join("")}${d.more ? `<div class="cal-more">+${d.more} more</div>` : ""}</div>`;
  };
  const grid = `<div class="cal-grid">${head}${x.weeks.flat().map(cell).join("")}</div>`;
  const d = x.day;
  const side = d
    ? `<div class="panel"><h2>${esc(d.pretty)} <small>${d.in === 0 ? "today" : d.in > 0 ? "in " + d.in + " days" : -d.in + " days ago"}</small></h2>${d.game ? `<p class="bigline">${esc(d.game.at)} ${teamA(d.game.opp)}${d.game.score ? ` · ${d.game.score[0]}-${d.game.score[1]}` : ""}${d.game.page != null ? ` · <a class="link" data-go="game" data-index="${d.game.page}">game page</a>` : ""}</p>` : ""}${d.items.length ? `<div class="items">${d.items.map((e) => `<div class="item"><span class="when">${esc(e.kind)}</span><span>${e.mine ? "<b>" : ""}${rich(e.text)}${e.mine ? "</b>" : ""}</span></div>`).join("")}</div>` : `<div class="empty">${d.in > 0 ? "Nothing posted for this day yet." : "Nothing kept for this day."}</div>`}</div>`
    : "";
  const ahead = (x.ahead || []).length ? `<div class="panel"><h2>Coming up</h2><div class="items">${x.ahead.map((e) => `<div class="item cal"><span class="when">${esc(e.date)}<br><small>${e.in === 0 ? "today" : "in " + e.in + " d"}</small></span><span>${e.mine ? "<b>" + esc(e.text) + "</b>" : esc(e.text)}</span></div>`).join("")}</div></div>` : "";
  return `<div class="pad">${pageHead({ kicker: `Today is ${esc(S.status ? S.status.today : x.today)}`, title: esc(x.title), actions: `<button class="st-icon-btn" data-act="calmove" data-v="${x.prev}" aria-label="Previous month">${icon("back")}</button><button class="st-btn small glass" data-act="calmove" data-v="">Today</button><button class="st-icon-btn" data-act="calmove" data-v="${x.next}" aria-label="Next month">${icon("fwd")}</button>` })}
    <div class="cal-wrap"><div>${grid}<p class="muted small">Your games and the dates that concern you are in bold; big moments for everyone show too. Click a day for all of it.</p></div><div class="col">${side}${ahead}</div></div></div>`;
};
GOTO.calendar = () => [["Schedule", "schedule"], ["Inbox", "inbox"], ["News", "news"]];

// ---------- standings and playoff brackets ----------
function bracketHtml(b) {
  if (!b.rounds || !b.rounds.length) return `<div class="empty">${b.pending ? "The bracket fills in once the playoffs are set." : "No playoffs to show yet."}</div>`;
  const side = (t, s, won) => `<div class="bk-t ${won ? "won" : ""}">${t ? `${mark(t, 18)}${teamA(t)}` : '<span class="muted">To be decided</span>'}<b>${s ?? ""}</b></div>`;
  return `<div class="bracket">${b.rounds.map((r) => `<div class="bk-round"><div class="st-label">${esc(r.name)}</div>${r.games.map((g) => {
    const s = g.score;
    return `<div class="bk-game">${side(g.away, s ? s[1] : null, s && s[1] > s[0])}${side(g.home, s ? s[0] : null, s && s[0] > s[1])}${g.name || g.date ? `<small class="muted">${esc(g.name || "")}${g.name && g.date ? " · " : ""}${esc(g.date || "")}${g.page != null ? ` · <a class="link" data-go="game" data-index="${g.page}">game page</a>` : ""}</small>` : ""}</div>`;
  }).join("") || '<div class="bk-game empty">Not played yet</div>'}</div>`).join("")}</div>`;
}
screens.standings = async (a) => {
  const tab = tabOf("stab", ["Standings", "Playoffs"], a);
  title("Standings");
  let body = "";
  if (tab === "Standings") {
    const x = await api("standings", a.conference ? { conference: a.conference } : {});
    const groups = x.groups.map((g) => `<div class="panel"><h2>${compIn(g.name)}${esc(g.name)}</h2>${table("st:" + g.name, [
      { k: "team", t: "Team", l: true, f: (t) => teamChip(t.team, 20), v: (t) => t.team.name },
      ...(g.teams[0] && g.teams[0].cw != null ? [{ k: "conf", t: "Conf", f: (t) => `${t.cw}-${t.cl}`, v: (t) => t.cw - t.cl + t.cw / 100 }] : []),
      { k: "all", t: "Overall", f: (t) => `<b>${t.w}-${t.l}${t.t ? "-" + t.t : ""}</b>`, v: (t) => t.w - t.l + t.w / 100 },
    ], g.teams.map((t) => ({ ...t, mine: t.team.mine })), { tools: false })}</div>`).join("");
    const confs = x.conferences ? `<select class="ssel" data-act="conf">${x.conferences.map((c) => `<option ${x.groups[0] && c === x.groups[0].name ? "selected" : ""}>${esc(c)}</option>`).join("")}</select>` : "";
    body = `${confs ? `<div class="tbar">${confs}</div>` : ""}<div class="${x.groups.length > 1 ? "grid2" : ""}">${groups || '<div class="empty">No standings until the season starts.</div>'}</div>`;
  } else {
    const B = keep("brackets", {});
    const b = await api("brackets", B.division ? { level: "college", division: B.division } : {});
    const divs = b.level === "college" ? `<select class="ssel" data-act="bdiv">${["FBS", "FCS", "D2", "D3", "NAIA", "JUCO"].map((d) => `<option ${b.division === d ? "selected" : ""}>${d}</option>`).join("")}</select>` : "";
    body = `<div class="tbar">${divs}<b>${esc(b.title || "")}</b></div>${bracketHtml(b)}`;
  }
  return `<div class="pad">${pageHead({ title: "Standings" })}${gotoLine([["Schedule", "schedule"], ["Leaderboards", "leaders"], ["History", "history"]])}${tabs("stab", ["Standings", "Playoffs"], tab)}<div class="tabbody">${body}</div></div>`;
};

// ---------- my players (S5) ----------
screens.myplayers = async () => {
  const x = await api("my_players");
  title("My players", `${x.rows.length}`);
  const rows = x.rows.map((r) => ({ ...r, listed: false, noted: r.notes > 0 }));
  const delta = (r) => (r.then && r.then.read && r.read ? Math.round(mid(r.read) - mid(r.then.read)) : null);
  const t = table("myplayers", [
    { k: "name", t: "Player", l: true, f: (r) => playerLink(r.id, r.name) },
    { k: "pos", t: "Pos", l: true },
    { k: "age", t: "Age" },
    { k: "level", t: "Level", l: true },
    { k: "class", t: "Class", l: true, v: (r) => classRank(r.class) ?? r.class },
    { k: "team", t: "Team", l: true, f: (r) => (r.team ? teamChip(r.team, 18) : esc(r.where)), v: (r) => (r.team ? r.team.name : r.where) },
    { k: "stars", t: "Stars", f: (r) => stars(r.stars) },
    { k: "read", t: "Your read now", l: true, f: (r) => range(r.read, null, { p: r.id, k: "ovr" }), v: (r) => mid(r.read) },
    { k: "change", t: "Since added", f: (r) => { const d = delta(r); return d == null || d === 0 ? '<span class="muted">·</span>' : `<span class="delta ${d > 0 ? "up" : "down"}">${d > 0 ? "▲" : "▼"} ${Math.abs(d)}</span>`; }, v: (r) => delta(r) ?? 0, tip: "How your staff's read moved since you added him" },
    { k: "added", t: "Added", l: true, f: (r) => esc(r.then ? r.then.date : ""), v: (r) => (r.then ? r.then.date : ""), hide: true },
    { k: "status", t: "Status", l: true, f: (r) => esc(r.status || "") },
    { k: "note", t: "Your last note", l: true, wrap: true, f: (r) => (r.note ? `<span class="muted">${esc(r.note)}</span>` : "") },
    { k: "x", t: "", f: (r) => `<button class="st-icon-btn small" data-act="listremove" data-id="${r.id}" title="Take off my players">×</button>` },
  ], rows, { empty: "Nobody yet. Right-click any name, or press \"Add to my players\" on a player's page.", select: (r) => r.id, groupBy: (r) => r.level });
  const watches = x.watches.length
    ? `<div class="items">${x.watches.map((w) => `<div class="item"><span class="when">${esc(w.since)}</span><span><b>${esc(w.name)}</b> <small class="muted">${w.seen} found so far</small></span>${linkTo("finder", w.query || {}, "Open", "st-btn small glass")}<button class="st-btn small" data-act="unwatch" data-k="${w.k}">Stop watching</button></div>`).join("")}</div>`
    : '<div class="empty">No saved searches. Build one in the player finder and press "Watch"; every Monday your personnel office reports who\'s new.</div>';
  return `<div class="pad">${pageHead({ kicker: "Your list, your notes, your watches", title: "My players", actions: linkTo("finder", {}, `${icon("search")} Player finder`, "st-btn small primary") })}${gotoLine([["Player finder", "finder"], ["Recruiting", "recruiting"], ["Leaderboards", "leaders"]])}${t}<div class="panel"><h2>Saved searches <small>watched every Monday</small></h2>${watches}</div></div>`;
};

// ---------- leaderboards (S8) ----------
const SCOPES = [["fbs", "FBS"], ["fcs", "FCS"], ["d2", "Division II"], ["d3", "Division III"], ["naia", "NAIA"], ["juco", "Junior college"], ["college", "All college"], ["nfl", "NFL"], ["hs", "High school"]];
const POS_GROUPS = ["QB", "RB", "WR", "TE", "OL", "DL", "LB", "DB", "ST"];
const fmtCell = (v, d) => (v == null ? "" : typeof v === "number" ? (d ? v.toFixed(d) : Math.round(v).toLocaleString()) : esc(v));
screens.leaders = async (a) => {
  const L = keep("leaders", { per: "total" });
  if (a.sort) { S.keep["sort:leaders"] = { k: a.sort, dir: -1 }; delete a.sort; }
  for (const k of ["scope", "view", "year"]) if (a[k] != null) { L[k] = a[k]; delete a[k]; }
  const s = keep("sort:leaders", { k: null, dir: -1 });
  const filters = [...(L.pos ? [{ field: "pos", value: L.pos }] : []), ...(L.conf ? [{ field: "conf", value: L.conf }] : []), ...(L.listed ? [{ field: "listed", value: true }] : [])];
  const args = { scope: L.scope, year: L.year ? Number(L.year) : undefined, view: L.view, columns: L.columns || undefined, per: L.per, filters, state: L.state || undefined };
  const x = await api("leaders", { ...args, offset: 0, limit: 100, sort: s.k || undefined, dir: s.dir, sort2: s.k2 || undefined, dir2: s.dir2 });
  if (!x.years.length) x.years = [x.year];
  S.leadersMeta = x;
  S.leadersArgs = args;
  L.scope = x.scope;
  title("Leaderboards", `${x.year}`);
  const view = L.view || (x.views[0] && x.views[0].name);
  const cols = [
    { k: "rank", t: "#", nosort: true },
    { k: "name", t: "Player", l: true, chips: true, f: (r) => playerLink(r.id, r.name) },
    { k: "pos", t: "Pos", l: true },
    { k: "team", t: "Team", l: true, f: (r) => (r.team ? teamChip(r.team, 18) : "") },
    ...x.columns.map((c) => ({ k: c.key, t: c.label, tip: c.long + " · click a number to rank by it", stat: c.key, f: (r) => fmtCell(r.cells[c.key], c.fmt) })),
  ];
  const html = bigTable({ key: "leaders", cmd: "leaders", args, cols, empty: "Nobody qualifies yet." });
  BIG.leaders.cache.set(0, x.rows);
  BIG.leaders.total = x.total;
  const sel = (k, list, cur, all) => `<select class="ssel" data-act="lsel" data-k="${k}">${all ? `<option value="">${all}</option>` : ""}${list.map(([v, t]) => `<option value="${esc(v)}" ${String(cur ?? "") === String(v) ? "selected" : ""}>${esc(t)}</option>`).join("")}</select>`;
  const college = !["nfl", "hs"].includes(x.scope);
  const bar = `<div class="tbar">${sel("scope", SCOPES, x.scope)}${sel("year", x.years.map((y) => [y, y]), x.year)}${sel("pos", POS_GROUPS.map((p) => [p, p]), L.pos, "Every position")}${college && x.conferences.length ? sel("conf", x.conferences.map((c) => [c, c]), L.conf, "Every conference") : ""}${x.scope === "hs" ? sel("state", Object.entries(STATE_NAMES), L.state || (S.status && S.status.mine && S.status.mine.state) || "", "Your state") : ""}<span class="tspace"></span><div class="seg">${["total", "game", "snap"].map((p) => `<button data-act="lper" data-v="${p}" aria-pressed="${L.per === p}">${p === "total" ? "Totals" : p === "game" ? "Per game" : "Per 100 snaps"}</button>`).join("")}</div></div>
    <div class="tbar"><div class="seg wrap">${x.views.map((v) => `<button data-act="lview" data-v="${esc(v.name)}" aria-pressed="${view === v.name && !L.columns}">${esc(v.name)}</button>`).join("")}</div><span class="tspace"></span><button class="tchip" data-act="lmine" aria-pressed="${!!L.listed}"><i class="chipdot listed"></i>On my list</button><button class="tbtn" data-act="lcols">Columns</button><button class="tbtn" data-act="lcsv">CSV</button></div>`;
  return `<div class="pad">${pageHead({ kicker: `${(SCOPES.find((z) => z[0] === x.scope) || ["", x.scope])[1]} · ${x.year} · ${L.per === "total" ? "season totals" : L.per === "game" ? "per game" : "per 100 snaps"}`, title: "Leaderboards", sub: "Rate stats need the usual minimums. Click any number to rank by it; shift-click a heading for a second sort." })}${bar}${html}</div>`;
};
GOTO.leaders = () => [["Player finder", "finder"], ["Record books", "history", { tab: "Record book" }], ["Standings", "standings"]];

// ---------- the player finder (S6, S8) ----------
const FINDER_FIELDS = [["pos", "Position (QB, CB, DL…)"], ["level", "Level (High school, College, NFL, Draft pool)"], ["state", "Home state (TX)"], ["town", "Hometown (Austin, TX)"], ["stars", "Stars"], ["class", "Class of"], ["age", "Age"], ["ht", "Height in inches"], ["wt", "Weight"], ["team", "Team name has"], ["name", "Name has"], ["listed", "On my list"], ["noted", "Has my note"], ["mine", "On my team"]];
screens.finder = async (a) => {
  const F = keep("finder", { filters: [] });
  if (a.filters) { F.filters = a.filters; delete a.filters; }
  title("Player finder");
  const label = (f) => { const t = (FINDER_FIELDS.find((x) => x[0] === f.field) || [f.field, f.field])[1].replace(/ \(.*\)$/, ""); return f.value === true ? esc(t) : `${esc(t)} ${f.op === ">=" ? "at least" : f.op === "<=" ? "at most" : "is"} <b>${esc(f.value)}</b>`; };
  const chips = F.filters.map((f, i) => `<span class="tfilter">${label(f)}<button data-act="fdel" data-i="${i}" aria-label="Remove">×</button></span>`).join("");
  const cols = [
    { k: "name", t: "Player", l: true, chips: true, f: (r) => playerLink(r.id, r.name) },
    { k: "pos", t: "Pos", l: true },
    { k: "age", t: "Age" },
    { k: "stars", t: "Stars", f: (r) => stars(r.stars) },
    { k: "class", t: "Class of" },
    { k: "level", t: "Level", l: true },
    { k: "ht", t: "Ht", f: (r) => esc(r.ht || "") },
    { k: "wt", t: "Wt" },
    { k: "state", t: "From", l: true },
    { k: "team", t: "Where", l: true, nosort: true, f: (r) => (r.team ? teamChip(r.team, 18) : esc(r.where || "")) },
  ];
  const html = bigTable({ key: "finder", cmd: "finder", args: { filters: F.filters }, cols, sort: "stars", empty: "Nobody matches these filters." });
  return `<div class="pad">${pageHead({ kicker: "Everyone in football, sorted by the game", title: "Player finder", sub: "Build a search from filters. Save it as a watch and your personnel office tells you every Monday who's new." })}
    <div class="tbar">${chips || '<span class="muted small">No filters: everyone.</span>'}<button class="tbtn" data-act="fadd">+ Filter</button><span class="tspace"></span><button class="tbtn" data-act="fwatch" ${F.filters.length ? "" : "disabled"}>${icon("star")} Watch this search</button><button class="tbtn" data-act="fcsv">CSV</button></div>${html}</div>`;
};
GOTO.finder = () => [["My players", "myplayers"], ["Leaderboards", "leaders"], ["Recruiting", "recruiting"]];

// ---------- history: champions, record books, halls, awards, the carousel, drafts ----------
const HIST = [["Champions", "champions"], ["Record book", "records"], ["NFL records", "nfl_records"], ["Hall of Fame", "hall"], ["Awards", "awards"], ["Coaching carousel", "carousel"], ["Drafts", "drafts"]];
screens.history = async (a) => {
  if (a.tab) { const m = HIST.find((h) => h[1] === a.tab); if (m) a.tab = m[0]; }
  const H = keep("history", {});
  if (a.division) { H.division = a.division; delete a.division; }
  const tab = tabOf("htab", HIST.map((h) => h[0]), a);
  const key = HIST.find((h) => h[0] === tab)[1];
  const x = await api("history", { tab: key, division: H.division || undefined, year: H["y:" + key] ? Number(H["y:" + key]) : undefined });
  title("History", tab);
  const b = x.body;
  const yearSel = (cur) => `<select class="ssel" data-act="hsel" data-k="y:${key}">${x.years.map((y) => `<option ${y === cur ? "selected" : ""}>${y}</option>`).join("")}</select>`;
  let body = "";
  if (key === "champions") {
    body = table("hchamps", [{ k: "year", t: "Year" }, { k: "level", t: "Level", l: true }, { k: "team", t: "Champion", l: true, f: (r) => teamChip(r.team, 20), v: (r) => r.team.name }, { k: "record", t: "Record", l: true }, { k: "coach", t: "Coach", l: true, f: (r) => (r.coach ? coachLink(r.coach.id, r.coach.name) : ""), v: (r) => (r.coach ? r.coach.name : "") }], b, { tools: false, empty: "No champions kept yet." });
  } else if (key === "records") {
    body = `<div class="tbar"><select class="ssel" data-act="hsel" data-k="division">${["FBS", "FCS", "D2", "D3", "NAIA", "JUCO"].map((d) => `<option ${x.division === d ? "selected" : ""}>${d}</option>`).join("")}</select><span class="muted small">Season, career and streak records; a live record is still being added to.</span></div>
      <div class="books">${b.map((bk) => `<div class="panel book"><h2>${esc(bk.name)}</h2><ol>${bk.rows.map((r) => `<li class="${r.live ? "live" : ""}"><b>${esc(r.value)}</b><span>${r.id ? playerLink(r.id, r.name) : esc(r.name)}<small>${r.team ? esc(r.team.name) + " · " : ""}${esc(r.year)}${r.live ? " · active" : ""}</small></span></li>`).join("")}</ol></div>`).join("") || '<div class="empty">No records yet.</div>'}</div>`;
  } else if (key === "nfl_records") {
    body = table("hnfl", [{ k: "stat", t: "Record", l: true }, { k: "career", t: "Kind", l: true, f: (r) => (r.career ? "Career" : "Season") }, { k: "value", t: "Mark", l: true }, { k: "holder", t: "Held by", l: true }, { k: "year", t: "Year" }, { k: "world", t: "", l: true, f: (r) => (r.world ? '<span class="tag gold">Set in this world</span>' : '<span class="muted">Real record</span>') }], b, { tools: false });
  } else if (key === "hall") {
    body = b.length ? `<div class="hof">${b.map((h) => `<div class="hof-card">${face({ name: h.name }, 56)}<div><div class="st-label">${esc(h.level)} · ${h.year}</div><b>${h.id ? playerLink(h.id, h.name) : esc(h.name)}</b>${h.pos ? ` <small class="muted">${esc(h.pos)}</small>` : ""}${h.coach ? ' <span class="tag soft">Coach</span>' : ""}<p class="sub">${esc(h.why || "")}</p>${(h.teams || []).length ? `<div class="chips">${h.teams.map((t) => teamChip(t, 16)).join("")}</div>` : ""}</div></div>`).join("")}</div>` : '<div class="empty">Nobody inducted yet.</div>';
  } else if (key === "awards") {
    body = `<div class="tbar">${yearSel(b.year)}</div>${table("hawards", [{ k: "award", t: "Award", l: true }, { k: "level", t: "Level", l: true }, { k: "name", t: "Winner", l: true, f: (r) => playerLink(r.id, r.name) }, { k: "pos", t: "Pos", l: true, f: (r) => esc(r.pos || "") }, { k: "team", t: "Team", l: true, f: (r) => (r.team ? teamChip(r.team, 18) : ""), v: (r) => (r.team ? r.team.name : "") }], b.rows, { tools: false, empty: "No awards that year." })}`;
  } else if (key === "carousel") {
    body = `<div class="tbar">${yearSel(b.year)}</div>${b.rows.length ? `<div class="items">${b.rows.map((r) => `<div class="item"><span class="when">${esc(r.date || "")}</span><span><span class="tag ${r.kind === "Fired" ? "due" : "soft"}">${esc(r.kind)}</span> ${rich(r.text)}</span></div>`).join("")}</div>` : '<div class="empty">No coaching moves that year yet.</div>'}`;
  } else {
    body = `<div class="tbar">${yearSel(b.year)}</div>${table("hdraft", [{ k: "round", t: "Rd" }, { k: "pick", t: "Pick" }, { k: "team", t: "Team", l: true, f: (r) => teamChip(r.team, 18), v: (r) => r.team.name }, { k: "name", t: "Player", l: true, f: (r) => playerLink(r.id, r.name) }, { k: "pos", t: "Pos", l: true }, { k: "college", t: "College", l: true, f: (r) => (r.college ? teamA(r.college) : ""), v: (r) => (r.college ? r.college.name : "") }], b.rows, { tools: false, sort: null, empty: "No draft that year yet." })}`;
  }
  return `<div class="pad">${pageHead({ kicker: "The world's memory", title: "History" })}${tabs("htab", HIST.map((h) => h[0]), tab)}<div class="tabbody">${body}</div></div>`;
};

// ---------- settings ----------
screens.settings = async () => {
  title("Settings");
  const seg = (k, list, cur) => `<div class="seg">${list.map(([v, t]) => `<button data-act="set" data-k="${k}" data-v="${v}" aria-pressed="${String(cur) === String(v)}">${t}</button>`).join("")}</div>`;
  const row = (t, sub, ctl) => `<div class="setrow"><div><b>${t}</b><small>${sub}</small></div>${ctl}</div>`;
  const keys = [["Space", "Continue"], ["⌘K", "Search anything"], ["⌘[ and ⌘]", "Back and forward"], ["Shift-click", "Open beside this screen"], ["⌘-click", "Open in its own window"], ["Right-click a name", "Add to my players, add a note, compare"], ["↑ ↓ and return", "Move through a table and open the row"], ["← →", "Older and newer letter"], ["Esc", "Close the letter or panel"]];
  return `<div class="pad">${pageHead({ title: "Settings" })}<div class="grid2"><div class="panel"><h2>Look</h2>
    ${row("Theme", "Showtime dark, or light for daytime", seg("theme", [["", "Dark"], ["light", "Light"]], prefs.theme || ""))}
    ${row("Text size", `${Math.round((Number(prefs.textSize) || 1) * 100)}%`, `<input type="range" min="0.9" max="1.3" step="0.05" value="${Number(prefs.textSize) || 1}" data-act="textsize" aria-label="Text size">`)}
    ${row("Motion", "Reduced motion stops the hero carousel and the letter's rise", seg("motion", [["", "Like the system"], ["full", "Full"], ["reduce", "Reduced"]], prefs.motion || ""))}</div>
    <div class="panel"><h2>Reading</h2>
    ${row("Hover cards", "A card when you rest on a name", seg("hover", [["", "On"], ["off", "Off"]], prefs.hover || ""))}
    ${row("Hover delay", "How long to rest first", seg("hoverDelay", [["250", "Quick"], ["", "Normal"], ["900", "Slow"]], prefs.hoverDelay || ""))}</div></div>
    <div class="panel"><h2>Logos</h2>${row("Logo packs", "AFM has no logos of its own: install a pack, the way Football Manager's work", linkTo("logos", {}, "Logos", "st-btn small"))}</div>
    <div class="panel"><h2>Keys and clicks</h2><div class="keys">${keys.map(([k, t]) => `<div><kbd>${esc(k)}</kbd><span>${esc(t)}</span></div>`).join("")}</div></div></div>`;
};

// ---------- search as a page (the palette is ⌘K) ----------
screens.search = async (a) => {
  const x = await api("find", { q: a.q || "" });
  title("Search", a.q);
  const target = (it) => (it.kind === "player" ? ["player", { id: it.id }] : it.kind === "coach" ? ["coach", { id: it.id }] : it.kind === "team" ? ["team", it.ref] : it.kind === "game" ? ["game", { index: it.index }] : it.kind === "screen" ? [it.go, {}] : it.kind === "history" ? ["history", it.args] : it.kind === "finder" ? ["finder", it.args] : it.kind === "mail" ? ["inbox", { open: it.id }] : null);
  return `<div class="pad">${pageHead({ kicker: "Search", title: `"${esc(a.q || "")}"` })}<div class="sgrid">${x.groups.map((g) => `<div class="panel"><h2>${esc(g.name)}${g.more ? ` <small>and ${g.more} more</small>` : ""}</h2><div class="items">${g.items.map((it) => { const t = target(it); return `<div class="item">${it.colors ? mark({ name: it.name, colors: it.colors }, 20) : ""}<span>${t ? linkTo(t[0], t[1], esc(plainText(it.name))) : esc(plainText(it.name))}<br><small class="muted">${esc(it.sub || "")}</small></span></div>`; }).join("")}</div></div>`).join("") || `<div class="empty">Nothing matches.</div>`}</div></div>`;
};

// ---------- the live rooms' shared presentation ----------
// liveRoom({kicker, title, sub, colors, clock, body, side, actions}) draws
// the full-bleed room the draft, signing day, the deadline and the war
// room share (Lane 7 owns what happens in them).
function liveRoom(o) {
  const c = o.colors || (S.status && S.status.mine && S.status.mine.colors) || ["#1d2330", "#0c0f15"];
  return `<div class="liveroom" style="--c1:${c[0]};--c2:${c[1] || c[0]}"><header class="lr-head"><span class="st-pill live">Live</span><div><div class="st-label">${esc(o.kicker || "")}</div><h1>${esc(o.title || "")}</h1>${o.sub ? `<p>${rich(o.sub)}</p>` : ""}</div>${o.clock ? `<div class="lr-clock">${esc(o.clock)}</div>` : ""}</header><div class="lr-body"><div class="lr-main">${o.body || ""}</div>${o.side ? `<aside class="lr-side">${o.side}</aside>` : ""}</div>${o.actions ? `<footer class="lr-foot">${o.actions}</footer>` : ""}</div>`;
}
if (!screens.studio) screens.studio = async (a) => liveRoom({ kicker: a.kicker || "Live", title: a.title || "The room", sub: a.sub || "", body: (a.lines || []).map((l) => `<p>${rich(l)}</p>`).join("") || '<div class="empty">Nothing live right now.</div>' });

// The actions the screens above add.
const CORE_MORE = {
  pstatv: (el) => { keep("pstats", {}).v = el.dataset.v; refresh(el); },
  cmpdrop: (el) => { const t = keep("cmp", { ids: [] }); const id = Number(el.dataset.id); t.ids = (S.route.args.ids || t.ids).filter((x) => x !== id); go("compare", { ids: t.ids }, false); },
  newschain: (el) => { keep("newsfeed", {}).chain = el.dataset.k || null; refresh(el); },
  calday: (el) => { const C = keep("calmonth", {}); C.day = el.dataset.iso; C.month = null; refresh(el); },
  calmove: (el) => { const C = keep("calmonth", {}); C.day = null; C.month = el.dataset.v || null; S.route.args = {}; refresh(el); },
  lmine: (el) => { const L = keep("leaders", {}); L.listed = !L.listed; refresh(el); },
};
Object.assign(CRUMBS, { finder: "Player finder", studio: "Live" });
