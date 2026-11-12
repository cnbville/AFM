// Front office screens: the college desk (lane 10). Your program's
// recruiting board, a recruit's file, finding prospects, visits, the
// transfer portal and NIL, on the NCAA's rules of the year. Every rating is
// your staff's read as a range, potential only in words, and a recruit's
// interest in your staff's words.
"use strict";

// ---------- sidebar, crumbs and icons ----------
Object.assign(I, {
  board: '<path d="M5 4h14v17H5z M9 4V2.5h6V4 M8.5 9h7M8.5 13h7M8.5 17h4"/>',
  find: '<circle cx="11" cy="11" r="6.5"/><path d="M20 20l-4.2-4.2"/>',
  visit: '<path d="M3 21h18M5 21V10l7-5 7 5v11M10 21v-5h4v5"/>',
  portal: '<path d="M4 12h12M12 7l5 5-5 5M20 4v16"/>',
  walkon: '<circle cx="12" cy="7" r="3.2"/><path d="M6 21v-3.5c0-2.8 2.7-4.8 6-4.8s6 2 6 4.8V21M16.5 4.5l3 3M19.5 4.5l-3 3"/>',
  nil: '<circle cx="12" cy="12" r="8.5"/><path d="M14.8 9.2c-.6-.9-1.6-1.3-2.8-1.3-1.6 0-2.8.8-2.8 2.1 0 3 5.8 1.5 5.8 4.3 0 1.3-1.3 2.2-3 2.2-1.3 0-2.4-.5-3-1.4M12 6.5v11"/>',
});
// The board holds the plain Recruiting table's job, and the watch, portal
// and JUCO lists as tabs (SC1).
foNavHide("recruiting", "college");
foNavAdd("college", [
  ["recruit_board", "Recruiting board", "board"],
  ["recruit_find", "Find recruits", "find"],
  ["visits_desk", "Visits", "visit"],
  ["portal_desk", "Transfer portal", "portal"],
  ["nil_desk", "NIL", "nil"],
  ["walkons_desk", "Walk-ons", "walkon"],
]);
Object.assign(CRUMBS, {
  recruit_board: "Recruiting board",
  recruit_find: "Find recruits",
  recruit_file: "Recruit's file",
  portal_file: "Transfer's file",
  visits_desk: "Visits",
  portal_desk: "Transfer portal",
  nil_desk: "NIL",
  walkons_desk: "Walk-ons",
});

// The old one-table Recruiting screen stays for the high school seat; a
// college head coach lands on the board.
{
  const plain = screens.recruiting;
  screens.recruiting = async (a) => {
    const me = S.status && S.status.mine;
    if (me && me.ref.kind === "college") {
      S.route = { screen: "recruit_board", args: a || {} };
      return screens.recruit_board(a || {});
    }
    return plain(a);
  };
}

// A recruit's player page carries your offer too (minor bug 58): it is
// among his moves (core.js, minor bugs 62-63).

// ---------- small pieces ----------
const fcStars = (n) => (n ? `<span class="fc-stars" title="${n}-star recruit (the services)">${"★".repeat(n)}<i>${"★".repeat(Math.max(0, 5 - n))}</i></span>` : '<span class="muted">-</span>');
const fcDot = (tone) => `<span class="dot ${tone || ""}"></span>`;
const fcSum = (list, k) => list.reduce((a, r) => a + (r[k] || 0), 0);
const fcIn = (n) => (n == null ? "" : n <= 0 ? "today" : n === 1 ? "tomorrow" : `in ${n} days`);
const fcPlural = (n, one, many) => `${n} ${n === 1 ? one : many || one + "s"}`;
const fcWords = ["no", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven", "twelve"];
const fcCount = (n) => (n < fcWords.length ? fcWords[n] : String(n));
const fcFollow = (n) => (n >= 1e6 ? `${(n / 1e6).toFixed(n >= 1e7 ? 0 : 1)}M` : n >= 1e3 ? `${Math.round(n / 1e3)}K` : String(n));
const fcCap = (s) => String(s || "").replace(/^./, (c) => c.toUpperCase());
// A link to one of the desk's own screens (or any built screen), carrying its subject.
function fcLink(screen, text, args = {}) {
  if (!screens[screen]) return esc(text);
  const a = Object.entries(args).map(([k, v]) => ` data-${k}="${esc(v)}"`).join("");
  return `<a class="link" data-go="${screen}"${a}>${esc(text)}</a>`;
}
// The "Go to" line: the three to five screens a coach turns to next, only
// those built, never the one you are on (screens topic 17, rule 5).
function fcGoto(list) {
  const here = S.route.screen;
  const items = list.filter(([s]) => screens[s] && s !== here).slice(0, 5);
  if (!items.length) return "";
  return gotoLine(items.map(([s, t, a]) => [t, s, a]));
}
function fcTabs(key, tabs, cur) {
  return `<div class="tabsline fc-tabs">${tabs.map(([k, t, n]) => `<button data-fc="tab" data-key="${key}" data-v="${k}" aria-selected="${cur === k}">${esc(t)}${n != null ? ` <small>${n}</small>` : ""}</button>`).join("")}</div>`;
}
function fcSeg(key, field, opts, cur) {
  return `<div class="seg">${opts.map(([v, t, n]) => `<button data-fc="seg" data-key="${key}" data-f="${field}" data-v="${esc(v)}" aria-pressed="${String(cur) === String(v)}">${esc(t)}${n != null ? `<small>${n}</small>` : ""}</button>`).join("")}</div>`;
}
// The hero over every desk screen: your program's light, the subject's big
// number behind the title, and a glass panel of the numbers that matter.
function fcHero({ label, title, line, sub, big, panel }) {
  return `<div class="hero small fc-hero"><div class="art"></div>${big != null ? `<div class="st-outline">${esc(big)}</div>` : ""}
    <div class="copy"><div class="st-label" style="color:rgba(255,255,255,.78)">${label}</div><h1>${esc(title)}</h1>${line ? `<div class="meta">${line}</div>` : ""}${sub ? `<p class="fc-heroline">${sub}</p>` : ""}</div>
    ${panel ? `<div class="st-glass st-panel fc-heropanel">${panel}</div>` : ""}</div>`;
}
function fcTiles(list) {
  return `<div class="st-tiles fc-tiles">${list.map(([v, t, go]) => `<div${go ? ` class="fc-tile" data-go="${go[0]}"${Object.entries(go[1] || {}).map(([k, x]) => ` data-${k}="${esc(x)}"`).join("")}` : ""}><b>${esc(v)}</b><small>${esc(t)}</small></div>`).join("")}</div>`;
}
// Who runs a duty: you, or your staff (with what they do), and the switch.
// A portal offer's money from its row: both boxes, a blank one no money.
function fcPortalMoney(id) {
  const val = (key) => { const inp = document.querySelector(`[data-fcin="${key}"][data-id="${id}"]`); return inp && inp.value !== "" ? Math.max(0, Number(inp.value)) * 1000 : null; };
  const a = { id };
  const nil = val("pnil"), coll = val("pcollp");
  if (nil != null) a.nil = nil;
  if (coll != null) a.coll = coll;
  return a;
}

function fcDuty(duties, key) {
  const d = duties.list.find((x) => x.key === key);
  if (!d) return "";
  const what = { board: "the recruiting board", visits: "your visits", portal: "the transfer portal", nil: "NIL" }[key];
  return `<div class="fc-duty ${d.yours ? "yours" : ""}"><span>${d.yours ? `<b>You run ${what}.</b>` : `<b>Your staff runs ${what}.</b> ${esc(d.staff)} A move you make yourself stands.`}</span>
    <button class="st-btn small glass" data-fc="duty" data-duty="${d.key}" data-owner="${d.yours ? "staff" : "you"}">${d.yours ? "Hand it to your staff" : "Take it back"}</button></div>`;
}
function fcPot(p) {
  if (!p) return '<span class="muted">Not worked yet</span>';
  return `<span title="How sure: ${esc(words(p.sure))}${p.hints && p.hints.length ? " · " + esc(p.hints.join(" · ")) : ""}">${esc(fcCap(p.tier))}</span>`;
}
const fcPublic = (r) => `<small>${esc(r.rank)}${r.camp ? ` · <span title="The kicking and snapping camps' own stars">${r.camp}★ camps</span>` : ""}</small>`;
const fcTone = (t) => (t === "good" ? "good" : t === "bad" ? "bad" : t === "warn" ? "warn" : "");
// A recruit's school: his high school's page, or his junior college's.
const fcSchool = (r) => (r.hs ? teamLink(r.hs) : esc(r.high_school || ""));
function fcStanding(r) {
  return {
    committed: '<span class="st-pill good">Committed to you</span>',
    signed: '<span class="st-pill good">Signed with you</span>',
    enrolled: '<span class="st-pill good">On campus</span>',
    committed_elsewhere: '<span class="st-pill">Committed elsewhere</span>',
    signed_elsewhere: '<span class="st-pill">Signed elsewhere</span>',
    gone: '<span class="st-pill">Off the market</span>',
  }[r.standing] || "";
}
// Your offer's state on the board: when it went out and whether the class
// has room to take his yes (commitments first, then your order).
function fcOfferCell(r) {
  if (r.standing === "signed" || r.standing === "enrolled") return '<span class="muted">Signed</span>';
  if (r.offered) {
    const who = r.by && r.by !== "You" ? ` <small class="sub" title="${esc(r.why || "")}">· ${esc(r.by)}</small>` : "";
    const room = r.committable === false ? ' <span class="st-pill gold" title="More live offers are ahead of him than your class has places, in the class or at his position: his yes waits until a place opens.">Past the class</span>' : "";
    return `<span>Offered ${esc(r.offered_on || "")}</span>${who}${room}`;
  }
  if (r.pulled) return `<span class="muted">Pulled ${esc(r.pulled)}</span>`;
  return '<span class="muted">No offer</span>';
}

// ---------- the recruiting board ----------
// The board's rows by id, for the move bar's counts (its ticks are the table's).
let FC_ROWS = new Map();
let FC_FRESH = null;
// Whether the board is yours to offer and pull on (else your coordinator's).
let FC_YOURS = true;
screens.recruit_board = async (a) => {
  const x = await api("recruit_board");
  setColors(x.team.colors);
  dvOwn("board", x);
  FC_ROWS = new Map(x.rows.map((r) => [r.id, r]));
  FC_YOURS = x.duties.list.find((d) => d.key === "board").yours;
  FC_FRESH = FC_YOURS ? x.fresh : null;
  const k = keep("fcb", { tab: "targets", sel: null, order: "mine", cls: "next", pos: "ALL", offers: "all" });
  k.offers ??= "all";
  // One position pick across the board, Find recruits and the portal (depthview.js).
  k.pos = dvSync("board", k.pos);
  if (a && a.id) { k.sel = a.id; k.tab = "targets"; }
  if (a && a.tab) { k.tab = a.tab; a.tab = null; }
  const c = x.class;
  const yours = x.duties.list.find((d) => d.key === "board").yours;
  const year = c.year;
  const cur = x.rows.filter((r) => r.class_of === year);
  const live = cur.filter((r) => r.offered && !["signed", "enrolled", "signed_elsewhere", "gone"].includes(r.standing));
  const commits = c.commits + c.signed;
  const left = Math.max(0, c.slots - c.commits);
  const sg = x.calendar.signing;
  const homes = x.homes.slice(0, 3).map((h) => esc(h.date)).join(", ");
  const heroLine = `${esc(fcCap(x.calendar.period))} · ${esc(x.calendar.rule)}`;
  const sub = `${commits ? `${fcCap(fcCount(commits))} in${c.signed ? ` (${c.signed} signed)` : ""}` : "No commitments yet"}, ${fcPlural(left, "place")} to fill${live.length ? `, ${fcPlural(live.length, "offer")} out` : ""}.${homes ? ` Home weekends left: ${homes}.` : ""}`;
  const panel = `<h2>The class so far <small>${esc(x.division)} · ${c.whole_roster ? `${c.limit}-man roster` : c.yearly ? `${c.yearly} signees a year` : "no signing limit"}${sg ? ` · ${esc(sg.name.toLowerCase())} ${esc(sg.date)}` : ""}</small></h2>
    ${fcTiles([
      [String(commits), commits === 1 ? "Commit" : "Commits"],
      [c.rank ? ordinal(c.rank) : "-", c.rank ? `Public rank of ${c.rank_of}` : "Not ranked yet"],
      [String(left), "Places left"],
      [sg ? String(sg.in) : "-", sg ? "Days to signing" : "No signing date"],
    ])}`;
  const ch = await api("recruit_chain").catch(() => null);
  const tabs = [["targets", "Top targets"], ["board", "Full board", x.rows.length], ["word", "Your staff's word", ch ? ch.waiting.length : null], ["passed", "Passed on", ch ? ch.passed.length : null], ["map", "Map"], ["plan", "Class plan and staff"], ["watch", "Watch list"], ["portal", "Portal"], ["juco", "JUCO"]];
  let body = "";
  if (k.tab === "targets") body = await fcTargets(x, k);
  else if (k.tab === "board") body = fcBoardTable(x, k, yours);
  else if (k.tab === "word") body = l7Word(ch);
  else if (k.tab === "passed") body = l7Passed(ch);
  else if (k.tab === "map") body = await fcMap();
  else if (k.tab === "plan") body = fcPlan(x);
  // Your staff's watch list, the portal and JUCO (screens/scouting.js).
  else body = await scRecruitList(k.tab, x);
  return `<div class="film">${fcHero({ label: `${teamLink(x.team)} · ${esc(x.conference)} · Recruiting`, title: `Class of ${year}`, line: heroLine, sub, big: String(year).slice(2), panel })}
  <div class="pad fc-pad">
    <div class="fc-bar">${fcTabs("fcb", tabs, k.tab)}${fcGoto([["recruit_find", "Find recruits"], ["visits_desk", "Visits"], ["scouting", "Scouting"], ["calendar", "Calendar"], ["roster", "Squad"]])}</div>
    ${dvBlock("board")}
    ${yours ? fcFresh(x) : fcDuty(x.duties, "board")}
    ${body}
  </div></div>`;
};

// Your fresh start at a new job: the old staff's offers still out, and a
// new staff's reset of the board, free until its last day.
function fcFresh(x) {
  const f = x.fresh;
  if (!f) return "";
  const open = f.offers - f.commits;
  const btns = [`<button class="st-btn small primary" data-fc="reset" data-keep="0">Pull all ${f.offers}</button>`];
  if (f.commits && open) btns.push(`<button class="st-btn small glass" data-fc="reset" data-keep="1">Pull the ${open} not committed</button>`);
  btns.push(`<button class="st-btn small glass" data-fc="oldpick">Pick them on the full board</button>`);
  return `<div class="fc-fresh"><div><span class="st-label">Your fresh start · until ${esc(f.until)} · ${fcPlural(f.days, "day")} left</span>
    <p><b>The old staff left ${fcPlural(f.offers, "offer")} out${f.commits ? `, ${fcPlural(f.commits, "commitment")} among them` : ""}.</b> A new staff goes through the board it inherits and decides whom it re-offers. Until ${esc(f.until)}, pulling any of the old staff's offers costs you nothing with the recruit, his family or his high school coach. A commitment goes with its offer; players who signed stay signed.</p></div>
    <div class="fc-acts">${btns.join("")}</div></div>`;
}
// The board's own buttons on its move bar: pull the ticked recruits'
// offers, or take the ones without an offer off the board.
MOVEBAR_EXTRA.fcboard = (ids) => {
  const rows = ids.map((id) => FC_ROWS.get(id)).filter(Boolean);
  // With your coordinator on the board, his offers and pulls wait on your staff's word.
  const live = FC_YOURS ? rows.filter((r) => r.offered && !["signed", "enrolled"].includes(r.standing)) : [];
  const off = rows.filter((r) => r.on_board && !r.offered);
  return (live.length ? `<button class="st-btn small primary" data-fc="mbpull">Pull ${live.length === 1 ? "the offer" : `${live.length} offers`}</button>` : "")
    + (off.length ? `<button class="st-btn small glass" data-fc="mbunboard">Take ${off.length} off the board</button>` : "");
};

// ---------- the offer chain (lane 7, SK26-SK31) ----------
// Each Monday your position coaches bring the recruits they want offered,
// after their coordinator's film; nothing goes out until you sign. With
// the board handed to your staff, its offers and pulls wait here too.
function l7Word(ch) {
  if (!ch) return '<div class="empty">Your staff has nothing for you yet.</div>';
  const who = (r) => `${coachLink(r.coach.id, r.coach.name)}${r.film ? ` <small class="sub">· film: ${coachLink(r.film.id, r.film.name)}</small>` : ""}`;
  const wait = ch.waiting.length
    ? table("l7w", [
        { k: "name", t: "Recruit", l: true, f: (r) => `${playerLink(r.id, r.name)} <small class="sub">${esc(r.pos)} · ${esc(r.cls)}</small>${r.gem ? ' <span class="st-pill gold" title="Your staff reads him far above his stars">Under the radar</span>' : ""}${r.pull ? ' <span class="st-pill bad">Pull</span>' : ""}` },
        { k: "stars", t: "★", tip: "The services' stars", f: (r) => fcStars(r.stars) },
        { k: "school", t: "School", l: true, v: (r) => (r.hs ? r.hs.name : ""), f: (r) => (r.hs ? teamLink(r.hs) : "") },
        { k: "coach", t: "His coach", l: true, div: true, v: (r) => r.coach.name, f: who },
        { k: "read", t: "His read", tip: "Your staff's read the day he brought him, against the position's line", v: (r) => mid(r.read), f: (r) => `${range(r.read, null, { p: r.id, k: "ovr" })} <small class="sub">line ${r.line}</small>` },
        { k: "yours", t: "Yours", tip: "Your own read: your looks only, or a first impression", v: (r) => mid(r.yours), f: (r) => yourRange(r.yours) },
        { k: "conviction", t: "How hard", l: true, tip: "How hard he argues for it", v: (r) => r.conviction, f: (r) => `<span title="${r.conviction} of 20">${esc(r.argue)}</span>` },
        { k: "why", t: "His word", l: true, wrap: true, f: (r) => `<small>${esc(r.why)}</small>` },
        { k: "act", t: "", l: true, f: (r) => `<span class="fc-rowacts"><button class="st-btn small primary" data-fc="l7sign" data-id="${r.id}">${r.pull ? "Pull it" : "Offer"}</button><button class="st-btn small glass" data-fc="l7decline" data-id="${r.id}">Decline</button></span>` },
      ], ch.waiting, { sort: null })
    : `<div class="empty">Nothing waits for your word. Your position coaches bring their recruits on Mondays.</div>`;
  const done = ch.answered.length
    ? `<div class="panel"><h2>Answered <small>the last season of your word</small></h2><div class="items">${ch.answered.slice(0, 14).map((r) => `<div class="item"><span class="when">${esc(r.answered || r.date)}</span><span>${playerLink(r.id, r.name)} <small class="sub">${esc(r.pos)}</small> · ${coachLink(r.coach.id, r.coach.name)} ${r.pull ? "wanted the offer back" : "wanted him"} · <b>${esc({ Signed: "you signed it", Declined: "you overruled him", Lapsed: "he went elsewhere first" }[r.answer] || r.answer)}</b></span></div>`).join("")}</div></div>`
    : "";
  return `<div class="panel"><h2>Waiting for your word <small>${ch.board_yours ? "your position coaches' recommendations, each after the coordinator's film" : "your staff runs the board: its offers and pulls wait for your signature"}</small></h2>
    ${ch.waiting.length > 1 ? `<div class="fc-acts"><button class="st-btn small primary" data-fc="l7all">Sign the whole list</button><span class="muted small">Declining overrules the coach: it's kept on his record, and the look-back three years on says who was right.</span></div>` : ""}
    ${wait}</div>${done}`;
}
function l7Passed(ch) {
  if (!ch || !ch.passed.length) return '<div class="empty">Your staff hasn\'t passed on anyone the services rate.</div>';
  return `<div class="panel"><h2>Passed on <small>the recruits your staff passed on, with who, when and why${ch.flags ? ` · ${ch.flags} flagged where the public and your staff disagree most` : ""}</small></h2>
    ${table("l7p", [
      { k: "name", t: "Recruit", l: true, f: (r) => `${playerLink(r.id, r.name)} <small class="sub">${esc(r.pos)} · ${esc(r.cls)}</small>${r.flag ? ' <span class="st-pill warn" title="The services rate him far above your staff">Public and staff disagree</span>' : ""}${r.back ? ' <span class="st-pill">Back on your board</span>' : ""}` },
      { k: "stars", t: "★", tip: "The services' stars", f: (r) => fcStars(r.stars) },
      { k: "where", t: "Now", l: true, f: (r) => esc(r.where) },
      { k: "coach", t: "Who passed", l: true, div: true, v: (r) => r.coach.name, f: (r) => `${coachLink(r.coach.id, r.coach.name)} <small class="sub">${esc(r.date)}</small>` },
      { k: "now", t: "Staff read", tip: "Your staff's read today", v: (r) => mid(r.now), f: (r) => range(r.now, null, { p: r.id, k: "ovr" }) },
      { k: "yours", t: "Yours", tip: "Your own read", v: (r) => mid(r.yours), f: (r) => yourRange(r.yours) },
      { k: "why", t: "Why", l: true, wrap: true, f: (r) => `<small>${esc(r.why)}</small>` },
      { k: "act", t: "", l: true, f: (r) => `<span class="fc-rowacts">${r.back ? "" : `<button class="st-btn small glass" data-fc="l7back" data-id="${r.id}">Back on the board</button>`}<button class="st-btn small glass" data-fc="l7see" data-id="${r.id}">See him</button></span>` },
    ], ch.passed, { sort: null })}</div>`;
}

// ---------- the map (screens topic 7) ----------
// The real map of the country (board 7, update 2 lane 7): screens/boards.js
// draws it. Each assistant's area keeps its own quiet tint there.
const FC_AREA = ["#8fb3d9", "#d9a86c", "#9ccf9a", "#d98f8f", "#b89ad9", "#d9cf7a", "#7fcfc4", "#d99ac2", "#a8b0bd"];
async function fcMap() {
  return frMap();
}

function ordinal(n) {
  const s = ["th", "st", "nd", "rd"], v = n % 100;
  return n + (s[(v - 20) % 10] || s[v] || s[0]);
}

// Top targets: your order as a poster shelf, and the one you pick, in full.
async function fcTargets(x, k) {
  const year = x.class.year;
  // The names still in play: a signed recruit is in the class, not a target.
  const list = x.rows.filter((r) => !["signed", "signed_elsewhere", "gone", "enrolled"].includes(r.standing));
  const shelf = list.slice(0, 12);
  if (!shelf.length) {
    const signed = x.rows.filter((r) => r.standing === "signed").length;
    return `<div class="panel"><div class="empty">${signed ? `Every name on your board has decided: ${fcPlural(signed, "recruit")} signed with you. ` : "Your board is empty. "}${fcLink("recruit_find", "Find recruits")} in your footprint and add them, or let your coordinator build it.</div></div>`;
  }
  if (!k.sel || !x.rows.some((r) => r.id === k.sel)) k.sel = shelf[0].id;
  const posters = shelf
    .map((r) => {
      const [c1, c2] = r.hs_colors || ["#444", "#222"];
      const pick = r.id === k.sel;
      return `<div class="st-ranked fc-ranked"><span class="st-outline" aria-hidden="true">${r.place}</span>
        <button type="button" class="st-poster fc-poster" data-fc="pick" data-id="${r.id}" aria-pressed="${pick}" style="background:linear-gradient(165deg, ${c1}, color-mix(in oklab, ${c1} 28%, #000) 78%);--hs2:${c2}">
          <span class="st-pill">${esc(r.pos)}</span>
          <span class="st-outline fc-hsmark" aria-hidden="true">${esc(initials(r.hs ? r.hs.name : "") || r.pos)}</span>
          ${r.priority ? '<span class="fc-yourtime" title="You give him your own time">●</span>' : ""}
          <span class="st-poster-cap"><b>${esc(r.name)}</b><small>${esc(r.hs ? r.hs.name : "")}</small><small>${esc(r.town)}${r.class_of !== year ? ` · ${r.class_of}` : ""}</small>
          <small class="fc-cap-int ${fcTone(r.tone)}">${esc(r.interest)}</small><small class="fc-cap-stars">${r.stars ? "★".repeat(r.stars) + " public" : "Unranked"}</small></span>
        </button></div>`;
    })
    .join("");
  const f = await api("recruit_file", { id: k.sel });
  return `<section class="st-shelf fc-shelf"><div class="st-shelf-head"><h2>Top targets</h2><span>Your staff's order, not the services'. Stars are the public's rating; the range is your position coach's read.</span></div>
    <div class="st-rail">${posters}</div></section>
    ${fcFile(f, x, true)}`;
}

// One recruit's file: who he is, your staff's read, where his recruitment
// stands, and every move the rules allow you today.
// A player's first answer to your money (N1): his mood in a word, then
// what he or his family said. His decision still comes in its own time.
const FC_MOOD_TONE = { thrilled: "good", pleased: "good", fine: "", wants_more: "gold", insulted: "fc-bad" };
// How close your offer is to what he needs to decide now (the satisfaction
// bar), as your staff reads it: a bar and words, never his number.
function fcBarMeter(b) {
  if (!b) return "";
  return `<div class="fc-barm" title="Your staff's read of how far your offer is from what he needs to commit now. Clear it and he says yes on the spot."><div class="st-meter thin"><span>What he needs</span><span class="t"><i style="width:${Math.round(b.m * 100)}%"></i></span><b></b></div><small class="sub">${fcDot(fcTone(b.tone))}${esc(b.words)}</small></div>`;
}

function fcAnswer(a) {
  if (!a) return "";
  return `<div class="fc-answer"><span class="st-pill ${FC_MOOD_TONE[a.mood] || ""}">${esc(a.word)}</span><span>${esc(a.text)}</span><small>${esc(a.date)}</small></div>`;
}
function fcFile(f, ctx, compact) {
  const [c1, c2] = f.hs_colors || ["#444", "#222"];
  const schools = f.schools.length
    ? f.schools
        .map((s) => `<div class="fc-school ${s.yours ? "yours" : ""}"><span class="fc-n">${s.n}</span>${mark(s.team, 28)}<span><b>${teamLink(s.team)}</b><small>${esc(s.status)}</small></span>${s.pill ? `<span class="st-pill ${s.pill === "Leads" ? "good" : "gold"}">${esc(s.pill)}</span>` : ""}</div>`)
        .join("")
    : '<div class="empty">No offers yet. The first one is remembered.</div>';
  const read = `<div class="fc-read"><div><span class="l7-pair">${range(f.read, null, { p: f.id, k: "ovr" })}${yourRange(f.yours)}</span></div><div><b>${fcPot(f.pot)}</b>${f.pot ? `<small class="sub">How sure: ${esc(words(f.pot.sure))}</small>` : ""}</div></div>
    ${f.pot && f.pot.hints && f.pot.hints.length ? `<div class="fc-hints">${f.pot.hints.map((h) => `<span class="st-fact">${esc(fcCap(h))}</span>`).join("")}</div>` : ""}
    <small class="sub">Read by ${esc(f.reader)}${f.reader_role ? `, your ${esc(f.reader_role)}` : ""}.${f.on_board ? "" : " Add him to your board and the read narrows as your staff works him."}</small>`;
  const nextLine = f.next ? `<b>${esc(f.next.date)}</b> · ${esc(f.next.what)}` : "Nothing set";
  const actions = fcActions(f);
  const visits = fcVisitPlanner(f);
  const left = `<div class="st-room fc-idroom" style="--accent:${c1};--accent-2:${c2}"><i></i><i></i><i></i>
      ${f.place ? `<span class="st-outline" aria-hidden="true">${f.place}</span>` : ""}
      <div class="fc-id">
        <div class="st-label">${esc(f.rank)}</div>
        <h2>${playerLink(f.id, f.name, "Recruiting")}</h2>
        <div><b>${fcSchool(f)}</b><div class="fc-town">${esc(f.town)} · ${f.juco ? "Junior college" : `Class of ${f.class_of}`} · ${esc(f.cls)}</div></div>
        <div class="fc-pills"><span class="st-pill">${esc(f.pos)}</span><span class="st-pill">${f.stars ? `${f.stars}★ public` : "Unranked"}</span>${f.priority ? '<span class="st-pill gold">Your own time</span>' : ""}${fcStanding(f)}</div>
        <dl class="fc-idkv"><dt>Decides</dt><dd>${esc(f.decides)}</dd><dt>Transcript</dt><dd>${esc(f.gpa)} core GPA · ${esc(f.qualifies)}</dd><dt>From campus</dt><dd>${Math.round(f.miles)} miles</dd>${f.camera ? `<dt>On camera</dt><dd>${esc(f.camera)}${f.followers ? ` · ${fcFollow(f.followers)} followers` : ""}</dd>` : ""}</dl>
        <div class="fc-meas"><b class="st-cond">${esc(f.ht)}</b><span>Height</span><b class="st-cond">${esc(f.wt)}</b><span>Weight, lbs</span><b class="st-cond">${f.offers}</b><span>${f.offers === 1 ? "Offer" : "Offers"}</span></div>
      </div></div>`;
  const mid = `<div class="fc-mid">
      <div class="fc-block"><div class="fc-h"><span class="st-label">Your staff's read</span>${compact ? fcLink("recruit_file", "Open his full file", { id: f.id }) : playerLink(f.id, "His player page")}</div>${read}</div>
      <div class="fc-block"><div class="fc-h"><span class="st-label">His interest in ${esc(ctx.team.name)}</span>${fcLink("recruit_race", "The race, month by month", { id: f.id })}</div>
        <div class="fc-interest ${fcTone(f.tone)}">${fcDot(fcTone(f.tone))}<b>${esc(f.interest)}</b></div><p class="sub">${esc(f.reason)}</p>${fcBarMeter(f.bar)}</div>
      <div class="fc-block"><div class="fc-h"><span class="st-label">His top schools</span><small class="sub">${f.schools_n > 1 ? `${f.schools_n} schools after him · ` : ""}as your staff hears it</small></div><div class="fc-schools">${schools}</div></div>
      <div class="fc-next"><span class="st-label">Next contact</span><span>${nextLine}</span></div>
    </div>`;
  const right = `<div class="fc-right">${actions}${visits}</div>`;
  const family = `<div class="panel fc-family"><h2>Family and transcript <small>what your staff has learned</small></h2>
      <dl class="kv"><dt>Who decides</dt><dd>${esc(f.decides)}</dd><dt>Home</dt><dd>${esc(f.town)}, ${Math.round(f.miles)} miles from campus</dd><dt>Transcript</dt><dd>${esc(f.gpa)} core GPA${f.test ? ` · test ${esc(f.test)}` : ""}</dd><dt>Eligibility</dt><dd>${esc(f.qualifies)}</dd><dt>Visits taken</dt><dd>${fcPlural(f.officials_used, "official visit")}${f.official_cap ? ` of ${f.official_cap}` : ""}</dd></dl></div>`;
  const early = f.early && f.early.length
    ? `<div class="panel"><h2>His offers in order <small>the first ones stay with a recruit</small></h2><div class="items">${f.early.map((o) => `<div class="item"><span class="when">${esc(o.date)}</span><span>${teamLink(o.team)}</span></div>`).join("")}</div></div>`
    : "";
  const history = f.visits && f.visits.length
    ? `<div class="panel"><h2>Visits and contacts <small>with your program</small></h2><div class="items">${f.visits.map((v) => `<div class="item fc-vitem"><span class="when">${esc(v.date)}</span><span><b>${esc(v.kind)}</b> · ${esc(v.state)}${v.note ? `<br><small class="sub">${esc(v.note)}</small>` : ""}</span>${v.state === "Set" && v.key ? `<button class="st-btn small glass" data-fc="vcancel" data-id="${f.id}" data-kind="${v.key}">Call off</button>` : ""}</div>`).join("")}</div></div>`
    : "";
  if (compact) {
    return `<section class="fc-file warm">${left}${mid}${right}</section>`;
  }
  return `<section class="fc-file warm">${left}${mid}${right}</section><div class="grid3">${family}${history || '<div class="panel"><h2>Visits and contacts</h2><div class="empty">He hasn\'t been on your campus yet.</div></div>'}${early || '<div class="panel"><h2>His offers in order</h2><div class="empty">No offers yet.</div></div>'}</div>`;
}

// Your own film session on him (bug 82): today, on your own film hours;
// when you can't, the button says why. A transfer's tape is his last
// college team's games, a redshirt's his high school's (minor bug 353).
function fcFilmBtn(f) {
  const fn = f.film_now;
  const why = fn && fn.blocked ? (/today/.test(fn.blocked) ? "watched today" : /resting/.test(fn.blocked) ? "resting" : /open/.test(fn.blocked) ? "cut-up open" : "hours spent") : fn ? `${fn.left.toFixed(1)} h left` : "";
  const title = fn && fn.blocked ? fn.blocked : `Watch his film yourself, today, in the film room, on your own film hours: read by your own eye${fn && fn.source ? `. The tape is ${fn.source}.` : ""}`;
  return `<button class="st-btn small glass l7-film" data-go="film" data-id="${f.id}" title="${esc(title)}">Watch his film now${why ? ` <small>${why}</small>` : ""}</button>`;
}

// The buttons for a recruit: board, offer, your own time, his NIL number.
function fcActions(f) {
  const b = [];
  const closed = ["signed", "enrolled", "signed_elsewhere", "gone"].includes(f.standing);
  if (!["signed", "enrolled", "gone"].includes(f.standing)) b.push(fcFilmBtn(f));
  if (!f.board_yours) {
    b.push(`<p class="sub">Your coordinator runs the board: his offers and order stand until you take it back.</p>`);
  } else if (!closed) {
    if (f.offered) b.push(f.free_pull ? `<button class="st-btn small glass" data-fc="pull" data-id="${f.id}" data-free="1" title="Your fresh start is on: a new staff resets the board, and it costs you nothing">Pull the old staff's offer</button>` : `<button class="st-btn small glass" data-fc="pull" data-id="${f.id}">Pull the offer</button>`);
    else b.push(`<button class="st-btn small primary" data-fc="offer" data-id="${f.id}">Offer a scholarship</button>`);
    if (f.on_board) {
      b.push(`<button class="st-btn small glass" data-fc="prio" data-id="${f.id}" data-on="${f.priority ? 0 : 1}">${f.priority ? "Leave him to his position coach" : "Give him your own time"}</button>`);
      if (f.place > 1) b.push(`<button class="st-btn small glass" data-fc="move" data-id="${f.id}" data-to="${f.place - 2}" title="Up one place">Up one</button>`);
      b.push(`<button class="st-btn small glass" data-fc="move" data-id="${f.id}" data-to="${f.place}" title="Down one place">Down one</button>`);
      if (!f.offered) b.push(`<button class="st-btn small glass" data-fc="remove" data-id="${f.id}">Take off the board</button>`);
    } else b.push(`<button class="st-btn small glass" data-fc="add" data-id="${f.id}">Add to your board</button>`);
  }
  let nil = "";
  if (f.nil_era && !closed) {
    // Every NIL dollar is yours (Cain, 2026-11-07): your number for him,
    // whoever runs the board. From July 2025 two boxes, the school's
    // revenue share and the collective's deal, which NIL Go reviews (B18).
    // NIL is optional on an offer (Cain, 2026-11-06): with no number your
    // offer is the scholarship. The going rate reads only his public stars
    // and your money, never what he is really worth.
    const kind = `${f.stars ? `a ${f.stars}★` : "an unranked"} ${f.pos}`;
    const two = f.two_pots;
    const k = (x) => (x != null ? Math.round(x / 1000) : "");
    const promised = f.nil != null || f.nil_coll != null;
    // A blank box is no money, whoever made the offer: your coordinator
    // never names money (Cain, 2026-11-08). The going rate is only a guide.
    const box = (label, key, val) => `<label class="fc-moneyin">${two ? `<span class="fc-pot">${label}</span>` : ""}$<input type="number" min="0" step="5" value="${k(val)}" placeholder="none" data-fcin="${key}" data-id="${f.id}">thousand a year</label>`;
    const state = promised
      ? two ? `You promised ${money(f.nil || 0)} of revenue share and a ${money(f.nil_coll || 0)} collective deal a year.` : `You promised ${money(f.nil)} a year.`
      : "No money promised: your offer is the scholarship alone.";
    const market = two
      ? `The going rate for ${kind} at a school like yours: about ${money(f.usual)} of revenue share and a ${money(f.usual_coll)} collective deal, which NIL Go clears up to his range.`
      : `Schools like yours usually pay ${kind} about ${money(f.usual)}.`;
    const left = two ? `${money(f.budget.left)} left in the revenue share and ${money(f.budget.coll_left)} in the collective` : `${money(f.budget.left)} left this year`;
    nil = `<div class="fc-nil"><span class="st-label">NIL promise <small>optional</small></span>
        <div class="fc-nilrow">${box("Revenue share", "promise", f.nil)}${two ? box("Collective deal", "pcoll", f.nil_coll) : ""}<button class="st-btn small primary" data-fc="promise" data-id="${f.id}">Promise</button></div>
        <small class="sub">${state} ${market} The going rate for his position tops out near ${money(f.nil_max)}, but only your money limits you: ${left}.</small>
        ${fcAnswer(f.answer)}
        ${promised ? `<button class="st-btn small glass" data-fc="promiseclear" data-id="${f.id}">Take the promise back</button>` : ""}</div>`;
  }
  return `<div class="fc-block"><div class="fc-h"><span class="st-label">Your moves</span></div><div class="fc-acts">${b.join("")}</div>${nil}</div>`;
}

// Visits the rules allow from today, each kind with its next days or why not.
function fcVisitPlanner(f) {
  if (!f.options) return "";
  // With your staff on visits, a visit you set still stands (SC6).
  const staffLine = f.visits_yours ? "" : `<p class="sub">Your staff sets the visits: the board's top names come in on home weekends. A visit you set yourself stands.</p>`;
  const rows = f.options
    .map((o) => {
      if (o.set) return `<div class="fc-vopt"><b>${esc(o.name)}</b><span class="fc-vset"><span class="fc-vdone">${fcDot("good")}Set for ${esc(o.set.date)}</span><button class="st-btn small glass" data-fc="vcancel" data-id="${f.id}" data-kind="${o.key}">Call off</button></span></div>`;
      if (!o.days.length) return `<div class="fc-vopt"><b>${esc(o.name)}</b><small class="sub">${esc(o.why || "Not possible this year.")}</small></div>`;
      return `<div class="fc-vopt"><b>${esc(o.name)}</b><span class="fc-vset"><select data-fcsel="v-${f.id}-${o.key}">${o.days.map((d) => `<option value="${d.d}">${esc(d.date)}${d.word ? " · " + esc(d.word.replace(/ period$/, "").replace(/^home game against /, "vs ")) : ""}</option>`).join("")}</select><button class="st-btn small glass" data-fc="visit" data-id="${f.id}" data-kind="${o.key}">Invite</button></span></div>`;
    })
    .join("");
  return `<div class="fc-block"><div class="fc-h"><span class="st-label">Plan a visit</span><small class="sub">on the NCAA's calendar</small></div>${staffLine}<div class="fc-vopts">${rows}</div></div>`;
}

// The full board: everyone, your order or by position, every column a coach reads.
function fcBoardTable(x, k, yours) {
  const year = x.class.year;
  const classes = [["next", `Class of ${year}`], ["later", "Younger classes"], ["all", "Everyone"]];
  let rows = x.rows.filter((r) => (k.cls === "next" ? r.class_of === year || r.juco : k.cls === "later" ? r.class_of > year && !r.juco : true));
  rows = rows.filter((r) => dvPosHas(k.pos, r.pos));
  // Whose offers: yours out, the old staff's still out, or the ones pulled.
  const live = (r) => r.offered && !["signed", "enrolled", "signed_elsewhere", "gone"].includes(r.standing);
  const offerOpts = [["all", "Everyone"], ["out", "Offers out"], ...(x.rows.some((r) => r.inherited && live(r)) || k.offers === "old" ? [["old", "The old staff's"]] : []), ...(x.rows.some((r) => r.pulled && !r.offered) || k.offers === "pulled" ? [["pulled", "Pulled"]] : [])];
  if (k.offers === "out") rows = rows.filter(live);
  else if (k.offers === "old") rows = rows.filter((r) => r.inherited && live(r));
  else if (k.offers === "pulled") rows = rows.filter((r) => r.pulled && !r.offered);
  const posOpts = [["ALL", "All"], ...x.class.by_pos.map((p) => [p.pos, p.pos])].filter(([v]) => v === "ALL" || x.rows.some((r) => r.pos === v));
  const cols = [
    { k: "place", t: "#", tip: "Your order: commitments first, then this order decides who can commit when the class fills", f: (r) => fcLink("recruit_file", String(r.place), { id: r.id }) },
    { k: "name", t: "Recruit", l: true, f: (r) => `${playerLink(r.id, r.name, "Recruiting")}${r.priority ? ' <span class="fc-yourtime" title="You give him your own time">●</span>' : ""}` },
    { k: "pos", t: "Pos", l: true },
    { k: "cls", t: "Class", l: true, v: (r) => r.class_of },
    { k: "stars", t: "★", tip: "The services' stars", f: (r) => fcStars(r.stars) },
    { k: "nat", t: "Public", l: true, tip: "The services' rank at his position and in his state", v: (r) => -(r.nat || 9999), f: fcPublic },
    { k: "school", t: "School · Hometown", l: true, div: true, v: (r) => (r.hs ? r.hs.name : ""), f: (r) => `${fcSchool(r)} <small class="sub">· ${esc(r.town)}</small>` },
    { k: "read", t: "Read", div: true, tip: "Your staff's read of his play now", v: (r) => mid(r.read), f: (r) => range(r.read, null, { p: r.id, k: "ovr" }) },
    { k: "yours", t: "Yours", tip: "Your own read: what you have seen of him yourself, or your first impression", v: (r) => mid(r.yours), f: (r) => yourRange(r.yours) },
    { k: "for_us", t: "For us", tip: "Best for us: his read with when he'd play here and how he fits what you run (his path on his page)", v: (r) => (r.for_us == null ? -99 : r.for_us), f: (r) => (r.for_us == null ? '<span class="muted">-</span>' : `<b>${Math.round(r.for_us)}</b>`) },
    { k: "pot", t: "Ceiling", l: true, tip: "Your staff's words on how far he can go", v: (r) => (r.pot ? r.pot.tier : ""), f: (r) => fcPot(r.pot) },
    { k: "interest", t: "His interest", l: true, div: true, f: (r) => `<span title="${esc(r.reason)}">${fcDot(fcTone(r.tone))}${esc(r.interest)}</span>` },
    { k: "leader", t: "Leader", l: true, v: (r) => (r.leader ? r.leader.name : ""), f: (r) => (r.leader ? teamLink(r.leader) : '<span class="muted">-</span>') },
    { k: "offers", t: "Offers" },
    { k: "offered", t: "Your offer", l: true, div: true, v: (r) => (r.offered ? 1 : 0), f: fcOfferCell },
    { k: "next", t: "Next", l: true, v: (r) => (r.next ? r.next.date : ""), f: (r) => (r.next ? `${esc(r.next.date)} <small class="sub">${esc(r.next.what)}</small>` : '<span class="muted">-</span>') },
    ...(yours
      ? [{ k: "act", t: "", l: true, v: () => 0, f: (r) => {
          const closed = ["signed", "enrolled", "signed_elsewhere", "gone"].includes(r.standing);
          return `<span class="fc-rowacts"><button class="fc-ib" data-fc="move" data-id="${r.id}" data-to="${Math.max(0, r.place - 2)}" title="Up one place">▲</button><button class="fc-ib" data-fc="move" data-id="${r.id}" data-to="${r.place}" title="Down one place">▼</button><button class="fc-ib ${r.priority ? "on" : ""}" data-fc="prio" data-id="${r.id}" data-on="${r.priority ? 0 : 1}" title="${r.priority ? "Leave him to his position coach" : "Give him your own time"}">●</button>${closed ? "" : r.offered ? `<button class="st-btn small glass" data-fc="pull" data-id="${r.id}"${r.free_pull ? ' data-free="1" title="The old staff\'s offer: free to pull in your fresh start"' : ""}>Pull</button>` : `<button class="st-btn small primary" data-fc="offer" data-id="${r.id}">Offer</button>`}</span>`;
        } }]
      : []),
  ];
  const note = `${rows.length} of ${x.rows.length} shown. Rest on a row for his staff's reason; a row opens his file.${yours ? " Tick rows to pull their offers together." : ""}`;
  return `<div class="filters">${fcSeg("fcb", "cls", classes, k.cls)}${fcSeg("fcb", "pos", posOpts, k.pos)}${k.pos.startsWith("g:") ? dvPickPill() : ""}${fcSeg("fcb", "offers", offerOpts, k.offers)}${fcSeg("fcb", "order", [["mine", "Your order"], ["pos", "By position"]], k.order)}<span class="sub fc-note">${esc(note)}</span></div>
    ${table("fcboard", cols, rows, { groupBy: k.order === "pos" ? (r) => r.pos : null, empty: "Nobody on the board in this view.", groups: [{ t: "Recruit", span: 7 }, { t: "Your staff", who: "position coaches", span: 4, div: true }, { t: "His recruitment", span: 3, div: true }, { t: "Yours", span: yours ? 3 : 2, div: true }] })}`;
}

// The class plan: the spots your staff would fill, the NCAA's limits, who
// runs what, and the dates ahead.
function fcPlan(x) {
  const c = x.class;
  const rows = c.by_pos.map((p) => `<tr><td class="l"><a class="link" data-fc="planpos" data-v="${p.pos}">${p.pos}</a></td><td>${p.need}</td><td>${p.committed}</td><td>${p.signed}</td><td>${p.offers}</td><td><b>${p.open ? p.open : '<span class="muted">0</span>'}</b></td></tr>`).join("");
  const limit = c.whole_roster
    ? `From 2025 the House settlement caps the whole roster at ${c.limit}, with no signee limit: your roster has room for ${c.room} more.`
    : `${c.yearly ? `The ${c.body} allows ${c.yearly} signees a year; ${c.signed} signed so far. ` : `The ${c.body} sets no yearly signing limit. `}${
        c.aid === "none" ? `Division III gives no athletic aid; your roster has room for ${c.room} more.`
        : c.aid === "shared" ? `${c.equiv} scholarships are shared out${c.aid_max ? ` among at most ${c.aid_max} players on aid` : ""}; your roster has room for ${c.room} more.`
        : `Your ${c.limit} scholarships leave room for ${c.room}.`}`;
  const duties = x.duties.list
    .map((d) => `<div class="fc-dutyrow"><div><b>${esc(d.name)}</b><small class="sub">${d.yours ? "You run it. Your staff steps in only when you hand it over." : esc(d.staff)}</small></div>
      <div class="seg"><button data-fc="duty" data-duty="${d.key}" data-owner="you" aria-pressed="${d.yours}">You</button><button data-fc="duty" data-duty="${d.key}" data-owner="staff" aria-pressed="${!d.yours}">Your staff</button></div></div>`)
    .join("");
  const fill = `<div class="fc-dutyrow"><div><b>Your coordinator fills the gaps</b><small class="sub">When your board leaves a spot open on a signing day, ${esc(x.duties.coordinator || "your coordinator")} offers the players your staff rates next. Each of his offers carries his reason.</small></div>
    <div class="seg"><button data-fc="fill" data-on="1" aria-pressed="${x.duties.fill}">On</button><button data-fc="fill" data-on="0" aria-pressed="${!x.duties.fill}">Off</button></div></div>`;
  const dates = x.calendar.dates.map((d) => `<div class="item"><span class="when">${esc(d.date)}<br><small>${fcIn(d.in)}</small></span><span>${esc(d.text)}</span></div>`).join("");
  const homes = x.homes.length
    ? x.homes.map((h) => `<div class="item"><span class="when">${esc(h.date)}</span><span><span class="teamcell">${mark(h.opp, 20)}<span>vs ${teamLink(h.opp)}</span></span>${h.officials ? ` <small class="sub">· ${fcPlural(h.officials, "official visit")} set</small>` : ""}</span></div>`).join("")
    : '<div class="empty">No home games left this season.</div>';
  return `<div class="grid2">
    <div style="display:grid;gap:18px">
      <div class="panel"><h2>Class of ${c.year} by position <small>your staff's plan: how many it would chase at each spot</small></h2>
        <div class="tbl warm"><table><thead><tr><th class="l">Pos</th><th title="Recruits your staff would chase at the spot; the class itself takes only its places">Chase</th><th>Committed</th><th>Signed</th><th title="Live offers out">Offers</th><th>Open</th></tr></thead><tbody>${rows}</tbody></table></div>
        <p class="sub">${esc(limit)} The class can take ${fcPlural(c.slots, "more commitment")}${c.slots ? "" : ": new yeses wait for a place"}.</p></div>
      <div class="panel"><h2>Who runs what <small>hand any of it to your staff, take it back any day</small></h2>${duties}${fill}</div>
    </div>
    <div style="display:grid;gap:18px">
      <div class="panel"><h2>The recruiting calendar <small>${esc(x.calendar.today)} · ${esc(x.calendar.period)}</small></h2><div class="items">${dates || '<div class="empty">Nothing ahead this year.</div>'}</div></div>
      <div class="panel"><h2>Home weekends <small>official visits in the fall come on these</small></h2><div class="items">${homes}</div></div>
    </div></div>`;
}

// ---------- one recruit's file, as its own page ----------
screens.recruit_file = async (a) => {
  const [f, b] = await Promise.all([api("recruit_file", { id: a.id }), api("recruit_board")]);
  setColors(b.team.colors);
  const go = fcGoto([["recruit_board", "Recruiting board"], ["recruit_race", "The race", { id: f.id }], ["visits_desk", "Visits"], ["player", "His player page", { id: f.id }], ["recruit_find", "Find recruits"]]);
  return `<div class="film"><div class="pad fc-pad">
    <div class="fc-bar"><div><div class="st-label">${teamLink(b.team)} · Recruiting · Class of ${f.class_of}</div><div class="h1">${esc(f.name)}</div></div>${go}</div>
    ${fcFile(f, b, false)}
  </div></div>`;
};

// ---------- a transfer's file ----------
// The recruit's file for a portal entrant (portal recruiting, Cain
// 2026-11-07): the same layout and moves a high school recruit has, on the
// portal's clock. Call him, offer, promise a role, host him on campus or
// see him at home; he says yes the day your offer clears what he needs.
function fcPortalMoves(f) {
  const b = [];
  // His tape (minor bug 353): a transfer's last college team's games.
  if (f.film_now) b.push(fcFilmBtn(f));
  // A blank box is no money; the tip gives a typical transfer's share. In
  // two pots each box names its pot (Cain, 2026-11-08: "which is which?").
  const box = (key, val, usual, what) => `<label class="fc-moneyin" title="${esc(`${what}. A typical transfer's share here: about ${money(usual)} a year`)}">${f.two_pots ? `<span class="fc-pot">${key === "pcollp" ? "Collective deal" : "Revenue share"}</span>` : ""}$<input type="number" min="0" step="5" value="${val != null ? Math.round(val / 1000) : ""}" placeholder="none" data-fcin="${key}" data-id="${f.id}">thousand a year</label>`;
  const nilIn = f.nil_era ? box("pnil", f.nil, f.usual, f.two_pots ? "Revenue share" : "NIL") + (f.two_pots ? box("pcollp", f.coll, f.coll_usual, "Collective deal, reviewed by NIL Go") : "") : "";
  if (f.committed) {
    b.push(`<p class="sub">${fcDot("good")}He told you yes on ${esc(f.committed)}. He signs tonight.</p>`);
  } else if (f.shopping) {
    b.push(`<button class="st-btn small primary" data-fc="pcall" data-id="${f.id}" ${f.called_today ? 'disabled title="You called him today"' : 'title="You or his position coach on the phone: what you sell him, and a promise if you make one"'}>${f.called_today ? "Called today" : "Call him"}</button>`);
    if (f.offered) b.push(`<button class="st-btn small glass" data-fc="ppull" data-id="${f.id}">Pull the offer</button>`);
  }
  let pay = "";
  if (f.shopping && !f.committed && f.nil_era) {
    const what = f.two_pots ? "revenue share" : "NIL";
    pay = `<div class="fc-nil"><span class="st-label">Your offer <small>${what} a year, optional</small></span>
      <div class="fc-nilrow">${nilIn}<button class="st-btn small ${f.offered ? "glass" : "primary"}" data-fc="poffer" data-id="${f.id}">${f.offered ? "Change it" : "Offer"}</button></div>
      <small class="sub">${f.nil != null || f.coll != null ? (f.two_pots ? `You offered ${money(f.nil || 0)} of revenue share and a ${money(f.coll || 0)} collective deal a year.` : `You offered ${money(f.nil)} a year.`) : f.offered ? "Your offer is the place alone: no money." : "A blank box is no money."} About ${money(f.usual)} is usual from a budget like yours, only a guide; ${money(f.budget.left)} left this year.</small>${fcAnswer(f.answer)}</div>`;
  } else if (f.shopping && !f.committed) {
    b.push(`<button class="st-btn small ${f.offered ? "glass" : "primary"}" data-fc="poffer" data-id="${f.id}">${f.offered ? "Offer stands" : "Offer a place"}</button>`);
  }
  const promise = f.promise
    ? `<p class="sub">You promised ${esc(f.promise.words.toLowerCase())} on ${esc(f.promise.date)}. Once he's yours it's on your word: kept, he trusts you; broken, he and his friends remember.</p>`
    : f.shopping ? `<p class="sub">A promise of a role is made in a call or a sit-down.</p>` : "";
  return `<div class="fc-block"><div class="fc-h"><span class="st-label">Your moves</span></div><div class="fc-acts">${b.join("")}</div>${pay}${promise}</div>`;
}
function fcPortalVisits(f) {
  if (!f.shopping || f.committed) return "";
  const rows = f.options
    .map((o) => {
      if (o.set) return `<div class="fc-vopt"><b>${esc(o.name)}</b><span class="fc-vset"><span class="fc-vdone">${fcDot("good")}Set for ${esc(o.set.date)}</span><button class="st-btn small glass" data-fc="pvcancel" data-id="${f.id}" data-kind="${o.key}">Call off</button></span></div>`;
      if (!o.days.length) return `<div class="fc-vopt"><b>${esc(o.name)}</b><small class="sub">${esc(o.why || "Not while he shops.")}</small></div>`;
      return `<div class="fc-vopt"><b>${esc(o.name)}</b><span class="fc-vset"><select data-fcsel="pv-${f.id}-${o.key}">${o.days.map((d) => `<option value="${d.d}">${esc(d.date)}${d.word ? " · " + esc(d.word.replace(/ period$/, "")) : ""}</option>`).join("")}</select><button class="st-btn small glass" data-fc="pvisit" data-id="${f.id}" data-kind="${o.key}">Invite</button></span></div>`;
    })
    .join("");
  return `<div class="fc-block"><div class="fc-h"><span class="st-label">Plan a visit</span><small class="sub">24 to 48 hours, before he decides</small></div><div class="fc-vopts">${rows}</div></div>`;
}
screens.portal_file = async (a) => {
  const f = await api("portal_file", { id: a.id });
  const [c1, c2] = f.colors || ["#444", "#222"];
  const schools = f.schools.length
    ? f.schools.map((s) => `<div class="fc-school ${s.yours ? "yours" : ""}"><span class="fc-n">${s.n}</span>${mark(s.team, 28)}<span><b>${teamLink(s.team)}</b><small>${esc(s.status)}</small></span>${s.pill ? `<span class="st-pill good">${esc(s.pill)}</span>` : ""}</div>`).join("")
    : `<div class="empty">${f.shopping ? "Nobody has a place for him yet." : "He isn't shopping."}</div>`;
  const read = `<div class="fc-read"><div><span class="l7-pair">${range(f.read, null, { p: f.id, k: "ovr" })}${yourRange(f.yours)}</span></div><div><b>${fcPot(f.pot)}</b>${f.pot ? `<small class="sub">How sure: ${esc(words(f.pot.sure))}</small>` : ""}</div></div>
    <small class="sub">Read by ${esc(f.reader)}${f.reader_role ? `, your ${esc(f.reader_role)}` : ""}, from his college tape.</small>`;
  const next = (f.visits || []).find((v) => v.state === "Set");
  const left = `<div class="st-room fc-idroom" style="--accent:${c1};--accent-2:${c2}"><i></i><i></i><i></i>
      <div class="fc-id">
        <div class="st-label">${esc(f.status)}</div>
        <h2>${playerLink(f.id, f.name, "Recruiting")}</h2>
        <div><b>${f.from ? teamLink(f.from) : ""}</b>${f.division ? ` <small>${esc(f.division)}</small>` : ""}<div class="fc-town">${esc(f.town)} · ${esc(f.cls)}</div></div>
        <div class="fc-pills"><span class="st-pill">${esc(f.pos)}</span><span class="st-pill">${f.stars ? `${f.stars}★ out of high school` : "Unranked"}</span>${f.role ? `<span class="st-pill">${esc(f.role)}</span>` : ""}${f.sits ? '<span class="st-pill gold">Sits a year with you</span>' : ""}</div>
        <dl class="fc-idkv">${f.why ? `<dt>Why he left</dt><dd>${esc(fcCap(f.why))}</dd>` : ""}${f.entered ? `<dt>Entered</dt><dd>${esc(f.entered)}</dd>` : ""}${f.decide ? `<dt>Decides</dt><dd>${esc(f.decide)}${f.days != null ? ` · ${esc(fcIn(f.days))}` : ""}</dd>` : ""}<dt>From campus</dt><dd>${Math.round(f.miles)} miles from home</dd></dl>
        <div class="fc-meas"><b class="st-cond">${esc(f.ht)}</b><span>Height</span><b class="st-cond">${esc(f.wt)}</b><span>Weight, lbs</span><b class="st-cond">${f.left}</b><span>${f.left === 1 ? "Season left" : "Seasons left"}</span></div>
      </div></div>`;
  const mid = `<div class="fc-mid">
      <div class="fc-block"><div class="fc-h"><span class="st-label">Your staff's read</span>${playerLink(f.id, "His player page")}</div>${read}</div>
      ${f.interest ? `<div class="fc-block"><div class="fc-h"><span class="st-label">His interest in your program</span></div><div class="fc-interest ${fcTone(f.tone)}">${fcDot(fcTone(f.tone))}<b>${esc(f.interest)}</b></div><p class="sub">${esc(f.reason)}</p>${fcBarMeter(f.bar)}</div>` : ""}
      <div class="fc-block"><div class="fc-h"><span class="st-label">His top schools</span><small class="sub">${f.schools_n > 1 ? `${f.schools_n} schools after him · ` : ""}as your staff hears it</small></div><div class="fc-schools">${schools}</div></div>
      <div class="fc-next"><span class="st-label">Next contact</span><span>${next ? `<b>${esc(next.date)}</b> · ${esc(next.kind)}` : f.shopping ? "Nothing set: a call is open any day" : "Nothing set"}</span></div>
    </div>`;
  const right = `<div class="fc-right">${fcPortalMoves(f)}${fcPortalVisits(f)}</div>`;
  const talks = f.talks.length
    ? `<div class="items">${f.talks.map((t) => `<div class="item"><span class="when">${esc(t.date)}</span><span><b>${esc(t.place)}</b> · sold ${esc(t.pitch.toLowerCase())}<br><small class="sub">${esc(t.how)}</small></span></div>`).join("")}</div>`
    : '<div class="empty">You haven\'t talked to him yet. Every staff with a place for him has called.</div>';
  const visits = f.visits.length
    ? `<div class="items">${f.visits.map((v) => `<div class="item"><span class="when">${esc(v.date)}</span><span><b>${esc(v.kind)}</b> · ${esc(v.state)}${v.note ? `<br><small class="sub">${esc(v.note)}</small>` : ""}</span></div>`).join("")}</div>`
    : '<div class="empty">He hasn\'t been on your campus.</div>';
  const go = fcGoto([["portal_desk", "Transfer portal"], ["player", "His player page", { id: f.id }], ["nil_desk", "NIL"], ["depth", "Depth chart"]]);
  return `<div class="film"><div class="pad fc-pad">
    <div class="fc-bar"><div><div class="st-label">Transfer portal${f.from ? ` · from ${esc(f.from.name)}` : ""}</div><div class="h1">${esc(f.name)}</div></div>${go}</div>
    <section class="fc-file warm">${left}${mid}${right}</section>
    <div class="grid3"><div class="panel"><h2>Your talks <small>the latest at each place counts</small></h2>${talks}</div><div class="panel"><h2>Visits <small>with your program</small></h2>${visits}</div><div class="panel"><h2>How a transfer decides</h2><p class="sub">He weighs the same things a recruit's family does, for himself: playing time against who's ahead of him, the money, the school, home, the staff, his shot at the league and winning. Every staff with a place for him calls; his finalists host him. When one school clearly gives him what he needs, he stops shopping and says yes, often days before his date.</p></div></div>
  </div></div>`;
};

// Finding recruits is its own screen now: screens/find.js (update 2, lane 7).

// ---------- visits ----------
// The desk's own invite (SC7): a recruit from your board, the kind of
// visit, one of his open dates on the NCAA calendar (the same moves as his
// file's Plan a visit).
async function fcInvite(x) {
  if (!x.targets.length) return `<div class="panel"><h2>Invite a recruit</h2><div class="empty">Nobody on your board to invite: ${fcLink("recruit_board", "add recruits to your board")}.</div></div>`;
  const k = keep("fcvinv", { id: null, kind: null });
  if (!x.targets.some((t) => t.id === k.id)) k.id = x.targets[0].id;
  let o;
  try { o = await api("visit_options", { id: k.id }); } catch (e) { o = { options: [], officials_used: 0 }; }
  const open = o.options.filter((v) => !v.set && v.days.length);
  if (!open.some((v) => v.key === k.kind)) k.kind = open.length ? open[0].key : null;
  const kind = open.find((v) => v.key === k.kind);
  const who = `<select class="fc-select" data-fcinv="id" aria-label="Who">${x.targets.map((t) => `<option value="${t.id}" ${t.id === k.id ? "selected" : ""}>${t.rank}. ${esc(t.name)} · ${esc(t.pos)}${t.stars ? " · " + "★".repeat(t.stars) : ""}${t.offered ? " · offered" : ""}</option>`).join("")}</select>`;
  const kinds = `<select class="fc-select" data-fcinv="kind" aria-label="Kind of visit">${o.options.map((v) => {
    const ok = !v.set && v.days.length;
    const left = v.key === "official" && o.official_cap ? ` (${Math.max(0, o.official_cap - o.officials_used)} of ${o.official_cap} left)` : "";
    return `<option value="${v.key}" ${v.key === k.kind ? "selected" : ""} ${ok ? "" : "disabled"}>${esc(v.name + left)}${v.set ? ` · set for ${esc(v.set.date)}` : ok ? "" : ` · ${esc(v.why || "not now")}`}</option>`;
  }).join("")}</select>`;
  const when = kind
    ? `<select class="fc-select" data-fcsel="v-${k.id}-${kind.key}" aria-label="When">${kind.days.map((d) => `<option value="${d.d}">${esc(d.date)}${d.word ? " · " + esc(d.word.replace(/ period$/, "").replace(/^home game against /, "vs ")) : ""}</option>`).join("")}</select>`
    : '<select class="fc-select" disabled aria-label="When"><option>No open date</option></select>';
  return `<div class="panel"><h2>Invite a recruit <small>only his open dates inside the NCAA calendar, each with your home game or the period</small></h2>
    <div class="fc-inv"><label><span class="st-label">Who</span>${who}</label><label><span class="st-label">Kind</span>${kinds}</label><label><span class="st-label">When</span>${when}</label>${kind ? `<button class="st-btn small primary" data-fc="visit" data-id="${k.id}" data-kind="${kind.key}">Invite</button>` : ""}</div></div>`;
}
screens.visits_desk = async () => {
  const x = await api("visits_desk");
  setColors(x.team.colors);
  const yours = x.duties.list.find((d) => d.key === "visits").yours;
  // A visit you set stands while your staff runs the rest (SC6).
  const invite = (yours ? "" : '<p class="sub">Your staff sets the visits: the board\'s top names come in on home weekends. A visit you set yourself stands.</p>') + (await fcInvite(x));
  const ahead = x.ahead.length
    ? table("fcvahead", [
        { k: "d", t: "Date", l: true, f: (r) => `<b>${esc(r.date)}</b> <small class="sub">${fcIn(r.in)}</small>` },
        { k: "kind", t: "Visit", l: true },
        { k: "name", t: "Recruit", l: true, div: true, f: (r) => playerLink(r.id, r.name, "Recruiting") },
        { k: "pos", t: "Pos", l: true },
        { k: "stars", t: "★", f: (r) => fcStars(r.stars) },
        { k: "town", t: "Hometown", l: true },
        { k: "interest", t: "His interest", l: true, div: true },
        { k: "note", t: "His answer", l: true, wrap: true },
        { k: "act", t: "", l: true, f: (r) => `<button class="st-btn small glass" data-fc="vcancel" data-id="${r.id}" data-kind="${r.key}">Call off</button>` },
      ], x.ahead, { empty: "Nothing set." })
    : `<div class="panel"><div class="empty">No visits set. Invite a recruit above, or from his file on ${fcLink("recruit_board", "your board")}.</div></div>`;
  const past = table("fcvpast", [
    { k: "d", t: "Date", l: true, f: (r) => esc(r.date) },
    { k: "kind", t: "Visit", l: true },
    { k: "name", t: "Recruit", l: true, div: true, f: (r) => playerLink(r.id, r.name, "Recruiting") },
    { k: "pos", t: "Pos", l: true },
    { k: "stars", t: "★", f: (r) => fcStars(r.stars) },
    { k: "state", t: "", l: true, f: (r) => `${fcDot(r.state === "Done" ? "good" : r.state === "Declined" ? "bad" : "")}${esc(r.state)}` },
    { k: "note", t: "How it went", l: true, wrap: true },
    { k: "interest", t: "His interest now", l: true, div: true },
  ], x.past, { empty: "No visits yet this year." });
  const legend = [["contact", "Contact"], ["evaluation", "Evaluation"], ["quiet", "Quiet"], ["dead", "Dead"]];
  const cal = `<div class="fc-ncaa">${x.weeks.map((w) => `<div class="fc-week"><small>${esc(w.start)}</small><div>${w.days.map((d) => `<i class="p-${d.period.split(" ")[0]}${d.past ? " past" : ""}${d.today ? " today" : ""}" title="${esc(d.wd)} ${d.day}: ${esc(d.period)}">${d.day}</i>`).join("")}</div></div>`).join("")}</div>
    <div class="fc-legend">${legend.map(([k, t]) => `<span><i class="p-${k}"></i>${t}</span>`).join("")}</div>`;
  const homes = x.homes.length
    ? x.homes.map((h) => `<div class="item"><span class="when">${esc(h.date)}</span><span><span class="teamcell">${mark(h.opp, 20)}<span>vs ${teamLink(h.opp)}</span></span>${h.officials ? ` <small class="sub">· ${fcPlural(h.officials, "official visit")} set</small>` : ' <small class="sub">· no officials yet</small>'}</span></div>`).join("")
    : '<div class="empty">No home games left this season.</div>';
  const rules = `<dl class="kv fc-rules"><dt>Official visits</dt><dd>Paid by you, 48 hours on campus. ${x.official_cap ? "Five per recruit and one per school" : "One per school, with no limit on how many a recruit takes (since July 1, 2023)"}; seniors from September 1${x.junior_officials ? ", juniors April 1 to June 30" : ""}. Never in a dead period.</dd>
    <dt>Unofficial visits</dt><dd>Paid by the family, any number, outside dead periods.</dd>
    <dt>Home visits</dt><dd>The head coach in his family's living room: seniors only, in a contact period, once a cycle.</dd>
    <dt>Drop-ins</dt><dd>Watching him practise at his school in an evaluation period: no talking off campus.</dd></dl>`;
  const set = x.ahead.filter((v) => v.key === "official").length;
  const panel = `<h2>This cycle <small>Class of ${x.class}</small></h2>${fcTiles([[String(x.ahead.length), "Visits set"], [String(set), "Officials set"], [String(x.officials), "Officials taken"], [String(x.taken), "Visits taken"]])}`;
  return `<div class="film">${fcHero({ label: `${teamLink(x.team)} · Recruiting`, title: "Visits", line: `${esc(fcCap(x.calendar.period))} · ${esc(x.calendar.rule)}`, panel })}
  <div class="pad fc-pad">
    ${invite}
    <div class="fc-bar"><div class="st-label">${x.ahead.length ? `Coming up · ${fcPlural(x.ahead.length, "visit")}` : "Coming up"}</div>${fcGoto([["recruit_board", "Recruiting board"], ["calendar", "Calendar"], ["schedule", "Schedule"], ["recruit_find", "Find recruits"]])}</div>
    ${yours ? "" : fcDuty(x.duties, "visits")}
    ${ahead}
    <div class="grid2"><div class="panel"><h2>The NCAA calendar <small>the next sixteen weeks, day by day</small></h2>${cal}${rules}</div>
      <div class="panel"><h2>Home weekends <small>your home games: the best day for an official</small></h2><div class="items">${homes}</div></div></div>
    <div class="st-label">Answers and visits this year</div>
    ${past}
  </div></div>`;
};

// ---------- the transfer portal ----------
// "If he stayed" for the next two falls: for a player who has left, the role
// the Needs map says he would have had; for one still here, the same map's
// word for where he stands. No calculation of its own (sb_needs: coming,
// portal, men, spots).
function fcStayCols(nd) {
  if (!nd || !Array.isArray(nd.portal) || !Array.isArray(nd.men) || nd.coming == null) return [];
  const gone = new Map(nd.portal.map((q) => [q.id, q]));
  const men = new Map(nd.men.map((m) => [m.id, m]));
  const spotWord = new Map(nd.spots.map((q) => [q.key, String(q.full).toLowerCase()]));
  const cell = (n) => (r) => {
    const q = gone.get(r.id);
    if (q) {
      const z = q.stayed[n];
      return z ? esc(z.words) : `<span class="muted" title="${esc(q.stay_gone || "")}">Gone by then</span>`;
    }
    const m = men.get(r.id);
    if (!m) return '<span class="muted">-</span>';
    if (m.spots[n]) return `Starter at ${esc(spotWord.get(m.spots[n]) || m.spots[n])}`;
    return m.vals[n] != null ? '<span class="muted">Not a starter</span>' : '<span class="muted">Gone by then</span>';
  };
  return [0, 1].map((o) => ({ k: "stay" + o, t: `If he stayed ’${String(nd.falls[nd.coming + o]).slice(2)}`, l: true, tip: `The role he would have in the fall of ${nd.falls[nd.coming + o]}, by the Needs map`, div: o === 0, f: cell(nd.coming + o) }));
}
screens.portal_desk = async () => {
  const k = keep("fcp", { tab: "entrants", pos: "ALL", div: "ALL" });
  // One position pick across the board, Find recruits and the portal (depthview.js).
  k.pos = dvSync("portal", k.pos);
  const x = await api("portal_desk", { pos: k.pos, division: k.div });
  setColors(x.team.colors);
  dvOwn("portal", x);
  DV.portalPage = { pos: k.pos, div: k.div, shown: x.shown };
  // When any of your players have left, the portal opens on them: the first thing to see is who is gone.
  const left = x.roster.filter((r) => r.in_portal).length;
  if (!k.seen) { k.seen = true; if (left) k.tab = "roster"; }
  const open = x.windows.find((w) => w.open);
  const next = x.windows.find((w) => !w.open && w.in > 0);
  const line = open ? `<span><b>${esc(open.name)} is open</b> · to ${esc(open.closes)}</span>` : next ? `<span>The ${esc(next.name.toLowerCase())} opens ${esc(next.opens)} (${fcIn(next.in)})</span>` : "<span>No window open</span>";
  const leaving = x.roster.filter((r) => r.rank <= 1).length;
  const panel = `<h2>The portal <small>where it stands today</small></h2>${fcTiles([[String(x.in_portal), "In the portal"], [String(x.targets.filter((t) => !t.settled).length), "Your targets"], [String(leaving), "Yours at risk"], [String(x.talks), "Talks held"]])}`;
  const tabs = [["entrants", "Entrants", x.shown], ["targets", "Your targets", x.targets.length], ["roster", "Your roster", x.roster.length], ["whispers", "Quiet words", x.whispers.length]];
  if (left) tabs.unshift(tabs.splice(2, 1)[0]);
  // Every NIL dollar is yours (Cain, 2026-11-07): a blank box is no money,
  // and from July 2025 the collective's deal is a second box (B18). The
  // going rate is in the box's tip.
  const k1 = (v) => (v != null ? Math.round(v / 1000) : "");
  // In two pots each box names its pot (Cain, 2026-11-08: "which is which?").
  const nilBox = (r, key, val, usual, what) => `<label class="fc-moneyin small" title="${esc(`${what}. A typical transfer's share here: about ${money(usual)} a year`)}">${x.house ? `<span class="fc-pot">${key === "pcollp" ? "Collective" : "Share"}</span>` : ""}$<input type="number" min="0" step="5" value="${k1(val)}" data-was="${k1(val)}" placeholder="none" data-fcin="${key}" data-id="${r.id}">k</label>`;
  const nilIn = (r) => (x.nil_era ? nilBox(r, "pnil", r.nil, r.usual, x.house ? "Revenue share" : "NIL") + (x.house ? nilBox(r, "pcollp", r.coll, r.coll_usual, "Collective deal, reviewed by NIL Go") : "") : "");
  let body = "";
  if (k.tab === "entrants") {
    const POS = ["QB", "RB", "WR", "TE", "OT", "IOL", "EDGE", "DT", "LB", "CB", "S", "K", "P", "LS"];
    const cols = [
      { k: "name", t: "Player", l: true, f: (r) => `${fcLink("portal_file", r.name, { id: r.id })}${r.contacted ? ' <span class="st-pill" title="You reached out before he entered">Talked</span>' : ""}` },
      { k: "pos", t: "Pos", l: true },
      { k: "from", t: "From", l: true, v: (r) => r.from.name, f: (r) => `${teamLink(r.from)} <small class="sub">${esc(r.division)}</small>` },
      { k: "division", t: "Div", l: true, hide: true },
      { k: "cls", t: "Class", l: true, hide: true },
      { k: "left", t: "Yrs", tip: "Seasons he could play from next season on", f: (r) => `<span title="${esc(r.cls)}">${r.left}</span>` },
      { k: "stars", t: "★", tip: "His stars out of high school", f: (r) => fcStars(r.stars) },
      { k: "role", t: "Last role", l: true, div: true, v: (r) => r.role || "", f: (r) => esc(r.role || "-") },
      { k: "read", t: "Read", tip: "Your staff's read from his tape", v: (r) => mid(r.read), f: (r) => range(r.read, null, { p: r.id, k: "ovr" }) },
      { k: "why", t: "Why he left", l: true, wrap: true, div: true },
      { k: "days", t: "Decides", tip: "When he means to choose", f: (r) => `<span title="${esc(fcIn(r.days))}">${esc(String(r.decide || "").replace(/ \d{4}$/, ""))}</span>` },
      { k: "sits", t: "", l: true, v: (r) => (r.sits ? 1 : 0), f: (r) => (r.sits ? '<span class="st-pill gold" title="He would sit out a season with you">Sits a year</span>' : "") },
      { k: "offered", t: "Your offer", l: true, div: true, v: (r) => (r.offered ? 1 : 0), f: (r) => (r.committed ? '<span class="st-pill good">Told you yes</span>' : r.offered ? `<span class="fc-rowacts"><span class="st-pill good">Offered</span>${nilIn(r)}<button class="st-btn small primary" data-fc="poffer" data-id="${r.id}" data-fcsave hidden>Change</button><button class="st-btn small glass" data-fc="ppull" data-id="${r.id}">Pull</button></span>` : `<span class="fc-rowacts">${nilIn(r)}<button class="st-btn small primary" data-fc="poffer" data-id="${r.id}">Offer</button></span>`) },
    ];
    // The groups are the strip's: OL, DL and DB, and the specialists (the game takes them as one choice).
    const posSel = `<select class="fc-select" data-fcf2="pos"><option value="ALL">Every position</option><optgroup label="Positions">${POS.map((p) => `<option ${k.pos === p ? "selected" : ""}>${p}</option>`).join("")}</optgroup><optgroup label="Groups">${["OL", "DL", "DB", "Specialists"].map((p) => `<option ${k.pos === p ? "selected" : ""}>${p}</option>`).join("")}</optgroup></select>`;
    const divs = [["ALL", "Every division"], ["FBS", "FBS"], ["FCS", "FCS"], ["D2", "D2"], ["NAIA", "NAIA"], ["D3", "D3"], ["JUCO", "JUCO"]];
    body = `<div class="filters">${posSel}${fcSeg("fcp", "div", divs, k.div)}<span class="sub fc-note">${esc(x.sit)}</span></div>
      ${table("fcportal", cols, x.entrants, { sort: null, empty: open ? "Nobody in the portal matches." : "The portal is closed: entrants come when a window opens.", groups: [{ t: "Player", span: 7 }, { t: "Your staff", span: 2, div: true }, { t: "His move", span: 3, div: true }, { t: "Yours", span: 1, div: true }] })}`;
  } else if (k.tab === "targets") {
    body = table("fcptargets", [
      { k: "name", t: "Player", l: true, f: (r) => fcLink("portal_file", r.name, { id: r.id }) },
      { k: "pos", t: "Pos", l: true },
      { k: "from", t: "From", l: true, v: (r) => r.from.name, f: (r) => teamLink(r.from) },
      { k: "status", t: "Where it stands", l: true, div: true, f: (r) => `${esc(r.committed ? `Committed to you, ${r.committed}` : r.status)}${r.promise ? `<br><small class="sub">Promised: ${esc(r.promise.toLowerCase())}</small>` : ""}` },
      { k: "interest", t: "His interest", l: true, v: (r) => (r.interest ? r.interest.words : ""), f: (r) => (r.interest ? `${fcDot(fcTone(r.interest.tone))}${esc(r.interest.words)}` : '<span class="muted">-</span>') },
      { k: "next", t: "Next contact", l: true, f: (r) => esc(r.next || "-") },
      { k: "offered", t: "Offered", l: true, f: (r) => esc(r.offered || "-") },
      { k: "nil", t: "NIL", tip: "Your offer's money a year: revenue share, and the collective's deal from July 2025", v: (r) => (r.nil || 0) + (r.coll || 0), f: (r) => (r.nil != null || r.coll != null ? money((r.nil || 0) + (r.coll || 0)) + (r.coll ? ` <small class="sub">${money(r.coll)} collective</small>` : "") : '<span class="muted">none</span>') },
      { k: "answer", t: "His answer", l: true, wrap: true, v: (r) => (r.answer ? r.answer.word : ""), f: (r) => (r.answer ? `<span class="st-pill ${FC_MOOD_TONE[r.answer.mood] || ""}">${esc(r.answer.word)}</span> <small class="sub">${esc(r.answer.text)}</small>` : '<span class="muted">-</span>') },
      { k: "decide", t: "Decides", l: true, f: (r) => esc(r.decide || "-") },
      { k: "act", t: "", l: true, f: (r) => (r.settled || r.committed ? "" : `<button class="st-btn small glass" data-fc="pforget" data-id="${r.id}">Forget him</button>`) },
    ], x.targets, { empty: "No targets yet. Offer an entrant, or reach out quietly to a player with whispers about him." });
  } else if (k.tab === "roster") {
    // The role each would have had if he stayed, in the Needs map's words (its
    // list of men and of your players in the portal); the columns wait for the map.
    await dvFetch(["needs"]);
    const stayCols = fcStayCols(DV.data.needs);
    body = `<p class="sub">Your players by how likely they are to leave, in your staff's words. A talk sways a player the way your relationships with players do: a good one keeps him, a poor one can push him.</p>
    ${table("fcproster", [
      { k: "rank", t: "", v: (r) => -r.rank, f: (r) => fcDot(fcTone(r.tone)) },
      { k: "name", t: "Player", l: true, f: (r) => playerLink(r.id, r.name, "People") },
      { k: "pos", t: "Pos", l: true },
      { k: "cls", t: "Class", l: true, v: (r) => classRank(r.cls) },
      { k: "role", t: "Last season", l: true, tip: "The role he had last season", v: (r) => r.role || "", f: (r) => esc(r.role || "-") },
      ...stayCols,
      { k: "read", t: "Read", div: true, v: (r) => mid(r.read), f: (r) => range(r.read, null, { p: r.id, k: "ovr" }) },
      { k: "risk", t: "Mood about staying", l: true, div: true, v: (r) => -r.rank },
      { k: "why", t: "What your staff hears", l: true, wrap: true, tip: "The strongest pull away this year, and the strongest reason he has to stay", f: (r) => `${esc(r.why ? r.why[0].toUpperCase() + r.why.slice(1) : "-")}${r.hold ? `<br><small class="sub">Keeping him: ${esc(r.hold)}</small>` : ""}` },
      { k: "nil", t: "NIL now", f: (r) => (r.nil ? money(r.nil) : '<span class="muted">-</span>') },
      { k: "talk", t: "Your talk", l: true, div: true, v: (r) => (r.talk ? 1 : 0), f: (r) => (r.talk ? `<small>${esc(r.talk.date)}: ${esc(r.talk.answer)}</small>` : r.in_portal ? "" : `<button class="st-btn small glass" data-fc="talk" data-id="${r.id}">Talk to him</button>`) },
    ], x.roster, { sort: "rank", groupBy: null })}`;
  } else {
    body = `<p class="sub">Players elsewhere with whispers that they may leave. Calling one before he enters is tampering: the NCAA rarely proves it, but word gets around about one call in eight.</p>
    ${table("fcpwhisper", [
      { k: "name", t: "Player", l: true, f: (r) => playerLink(r.id, r.name) },
      { k: "pos", t: "Pos", l: true },
      { k: "from", t: "School", l: true, v: (r) => r.from.name, f: (r) => teamLink(r.from) },
      { k: "division", t: "Div", l: true },
      { k: "cls", t: "Class", l: true },
      { k: "stars", t: "★", f: (r) => fcStars(r.stars) },
      { k: "role", t: "Role", l: true, div: true, v: (r) => r.role || "", f: (r) => esc(r.role || "-") },
      { k: "read", t: "Read", v: (r) => mid(r.read), f: (r) => range(r.read, null, { p: r.id, k: "ovr" }) },
      { k: "contacted", t: "Your word", l: true, div: true, v: (r) => (r.contacted ? 1 : 0), f: (r) => (r.contacted ? `${esc(r.contacted)}${r.leaked ? ' <span class="st-pill gold">Word got out</span>' : ""}` : `<button class="st-btn small glass" data-fc="pcontact" data-id="${r.id}" data-name="${esc(r.name)}">Reach out</button>`) },
    ], x.whispers, { empty: "No whispers around the country right now." })}`;
  }
  return `<div class="film">${fcHero({ label: `${teamLink(x.team)} · Roster`, title: "Transfer portal", line, sub: esc(x.rule), panel })}
  <div class="pad fc-pad">
    <div class="fc-bar">${fcTabs("fcp", tabs, k.tab)}${fcGoto([["roster", "Squad"], ["depth", "Depth chart"], ["nil_desk", "NIL"], ["recruit_board", "Recruiting board"]])}</div>
    ${dvBlock("portal")}
    ${x.yours ? "" : fcDuty(x.duties, "portal")}
    ${body}
  </div></div>`;
};

// ---------- NIL ----------
// The scholarships and the roster limit (the budget map's numbers), for the
// desk's header (SC1).
function fcAidTiles(sch) {
  if (!sch) return [];
  const aid = sch.aid === "none" ? ["None", "Athletic aid (Division III)"] : [`${sch.onAid}${sch.limit ? "/" + sch.limit : ""}`, sch.aid === "equivalency" ? `On aid · ${sch.equiv} equivalencies` : "On scholarship"];
  return [aid, sch.whole ? [`${sch.counted}/${sch.cap}`, "Roster places used"] : [String(sch.roster), `On the roster · ${sch.walkOns} walk-ons`]];
}
screens.nil_desk = async () => {
  const [x, cap] = await Promise.all([api("nil_desk"), api("board_cap").catch(() => null)]);
  setColors(x.team.colors);
  const b = x.budget;
  const aid = fcAidTiles(cap && cap.scholarships);
  if (!x.era) {
    return `<div class="film">${fcHero({ label: `${teamLink(x.team)} · Money`, title: "NIL", line: "<span>Before July 1, 2021 a college player can't be paid for his name, image or likeness.</span>", panel: aid.length ? `<h2>Scholarships <small>${esc(cap.level_name || "")}</small></h2>${fcTiles(aid)}` : "" })}
  <div class="pad fc-pad"><div class="fc-bar"><span></span>${fcGoto([["roster", "Squad"], ["portal_desk", "Transfer portal"], ["recruit_board", "Recruiting board"]])}</div>
    <div class="panel"><div class="empty">The NCAA's amateur rules hold: scholarships, cost of attendance and nothing more. NIL opens on July 1, 2021.</div></div></div></div>`;
  }
  // Every NIL dollar is yours (Cain, 2026-11-07): you set each player's
  // revenue share and, from July 2025, his collective deal too, which NIL Go
  // reviews (B18). Each deal stands until you change it; a blank box is $0
  // and pays nothing.
  const two = x.house;
  const m = (v) => (v === 0 ? "$0" : money(v));
  const pays = (r) => (r.deal || 0) + (two ? r.coll || 0 : 0);
  const blanks = x.rows.filter((r) => pays(r) <= 0).length;
  const set = fcSum(x.rows.map((r) => ({ v: pays(r) })), "v");
  const out = (roster, recruits, portal, left, label) => `<div class="item"><span>${label}</span><b>${m(roster)}</b></div>
      <div class="item"><span>Promised to recruits</span><b>${m(recruits)}</b></div>
      <div class="item"><span>Offered in the portal</span><b>${m(portal)}</b></div>
      <div class="item tot"><span>Left</span><b class="${left < 0 ? "fc-over" : ""}">${m(left)}</b></div>`;
  const ledger = two
    ? `<div class="st-ledger fc-ledger"><div><span class="st-label">Revenue share · you set it</span>
      <div class="item tot"><span>This year</span><b>${m(b.revenue_share)}</b></div>${out(b.roster, b.recruits, b.portal, b.left, "Your roster deals")}</div>
    <div><span class="st-label">Collective deal · you set it</span>
      <div class="item tot"><span>From boosters this year</span><b>${m(b.collective)}</b></div>${out(b.coll_roster, b.coll_recruits, b.coll_portal, b.coll_left, "Your roster deals")}
      <div class="item"><span>NIL Go clears of your roster deals</span><b>${m(b.collective_roster)}</b></div>
      <div class="item"><span>NIL Go holds back</span><b>${m(b.nil_go_held)}</b></div></div></div>`
    : `<div class="st-ledger fc-ledger"><div><span class="st-label">Money in</span>
      <div class="item"><span>Collective</span><b>${m(b.collective)}</b></div>
      <div class="item tot"><span>This year</span><b>${m(b.total)}</b></div></div>
    <div><span class="st-label">Money out · you set it</span>${out(b.roster, b.recruits, b.portal, b.left, "Your roster deals")}</div></div>`;
  const tiles = two
    ? [[m(b.revenue_share), "Revenue share"], [m(b.left), "Revenue share left"], [m(b.collective), "Collective, from boosters"], [m(b.coll_left), "Collective left"], [String(blanks), "Blank"]]
    : [[m(b.yours), "Budget"], [m(b.left), "Left"], [String(blanks), "Blank"]];
  const panel = `<h2>Money year ${x.year}-${String(x.year + 1).slice(2)} <small>paid ${esc(x.pay_day)} · you set every player's money</small></h2>${fcTiles([...tiles, [x.trend, "Boosters"], ...aid])}`;
  const k1 = (v) => (v ? Math.round(v / 1000) : "");
  const box = (r, key, v) => `<label class="fc-moneyin small">$<input type="number" min="0" step="5" value="${k1(v)}" data-was="${k1(v)}" placeholder="none" data-fcin="${key}" data-id="${r.id}">k</label>`;
  // Nothing is sent while you type (Cain, 2026-11-08): Set, or Enter, sends the row.
  const setBtn = (r) => `<button class="st-btn small primary" data-fc="dealset" data-id="${r.id}" data-fcsave hidden>Set</button>`;
  const going = (v) => (v ? `<span class="muted">${money(v)}</span>` : '<span class="muted">-</span>');
  const cols = [
    { k: "name", t: "Player", l: true, f: (r) => playerLink(r.id, r.name) },
    { k: "pos", t: "Pos", l: true },
    { k: "cls", t: "Class", l: true, v: (r) => classRank(r.cls) },
    { k: "left", t: "Yrs left" },
    { k: "role", t: "Role", l: true, v: (r) => r.role || "", f: (r) => esc(r.role || "-") },
    { k: "read", t: "Read", v: (r) => mid(r.read), f: (r) => range(r.read, null, { p: r.id, k: "ovr" }) },
    { k: "now", t: "Paid last year", div: true, f: (r) => (r.now ? money(r.now) : '<span class="muted">-</span>') },
    { k: "brand", t: "His own deals", tip: "Brand and ad money he earns himself, apart from yours (a public estimate for the year)", v: (r) => r.brand || 0, f: (r) => (r.brand ? money(r.brand) : '<span class="muted">-</span>') },
    { k: "followers", t: "Followers", tip: "Social media followers, every platform", v: (r) => r.followers || 0, f: (r) => (r.followers ? fcFollow(r.followers) : '<span class="muted">-</span>') },
    { k: "camera", t: "On camera", l: true, hide: true, tip: "His position coach's word for how he comes across in front of a camera", f: (r) => `<span class="sub">${esc(r.camera || "")}</span>` },
    { k: "usual", t: "Going rate", div: true, tip: "What players in his role usually get from the school's revenue share: only a guide", f: (r) => going(r.usual) },
    { k: "deal", t: two ? "Your share" : "Yours", v: (r) => r.deal || 0, f: (r) => box(r, "deal", r.deal) + setBtn(r) },
    ...(two
      ? [
          { k: "coll_usual", t: "Going rate", div: true, tip: "What players in his role usually get from the collective, as NIL Go clears it: only a guide", f: (r) => going(r.coll_usual) },
          { k: "coll", t: "Your collective", v: (r) => r.coll || 0, f: (r) => box(r, "dcoll", r.coll) + setBtn(r) },
          { k: "cleared", t: "NIL Go", l: true, tip: "What NIL Go clears of your collective deal: past the top of his range it holds the rest back, and it isn't paid this year", v: (r) => r.cleared || 0, f: (r) => (!r.coll ? '<span class="muted">-</span>' : r.cleared + 1 < r.coll ? `<span class="st-pill gold" title="${esc(`NIL Go clears ${money(r.cleared)} of your ${money(r.coll)}; ${money(r.coll - r.cleared)} isn't paid`)}">${money(r.cleared)}</span>` : '<span class="st-pill good">Cleared</span>') },
        ]
      : []),
    { k: "answer", t: "His answer", l: true, div: true, tip: "His first word on the numbers you set", v: (r) => (r.answer ? r.answer.word : ""), f: (r) => (r.answer ? `<span class="st-pill ${FC_MOOD_TONE[r.answer.mood] || ""}" title="${esc(r.answer.text)}">${esc(r.answer.word)}</span>` : '<span class="muted">-</span>') },
  ];
  const groups = [
    { t: "Him", span: 6 },
    { t: "For reference", span: 4, div: true },
    { t: two ? "Revenue share" : "Your deal", who: two ? "you set it, the school pays" : "you set it, the collective pays", span: 2, div: true },
    ...(two ? [{ t: "Collective deal", who: "you set it, boosters pay, NIL Go reviews", span: 3, div: true }] : []),
    { t: "His answer", span: 1, div: true },
  ];
  const parts = (p) => (two && p.coll ? ` <small class="sub">${money(p.school || 0)} + ${money(p.coll)} collective</small>` : "");
  const promised = x.promised.length
    ? x.promised.map((p) => `<div class="item"><span class="when">${money(p.nil)}</span><span>${playerLink(p.id, p.name, "Recruiting")} <small class="sub">· ${esc(p.pos)} · ${p.stars ? p.stars + "★" : "unrated"}${p.standing === "committed" ? " · committed" : p.standing === "signed" ? " · signed" : ""}</small>${parts(p)}</span></div>`).join("")
    : '<div class="empty">No promises to recruits. Set one in a recruit\'s file.</div>';
  const portal = x.portal.length
    ? x.portal.map((p) => `<div class="item"><span class="when">${money(p.nil)}</span><span>${playerLink(p.id, p.name, "Recruiting")} <small class="sub">· ${esc(p.pos)} from ${teamLink(p.from)}</small>${parts(p)}</span></div>`).join("")
    : '<div class="empty">No portal offers carry money.</div>';
  const hist = x.history.length
    ? `<div class="tbl warm"><table><thead><tr><th class="l">Season</th><th>NIL budget</th><th>From boosters</th></tr></thead><tbody>${x.history.map((h) => `<tr><td class="l">${h.season}</td><td>${m(h.budget)}</td><td>${m(h.boosters)}</td></tr>`).join("")}</tbody></table></div>`
    : '<div class="empty">No seasons yet.</div>';
  const cards = two
    ? `<div class="fc-nilcards"><div class="fc-nilcard"><b>Revenue share · you set it</b><span>The school pays it, under the House cap. Your number is his number.</span></div>
      <div class="fc-nilcard"><b>Collective deal · you set it</b><span>The boosters pay it. NIL Go reviews every deal and holds back whatever is past the range for a player like him.</span></div>
      <div class="fc-nilcard"><b>His own deals · for reference</b><span>Brand and ad money he earns himself. Not money you pay.</span></div></div>`
    : `<div class="fc-nilcards"><div class="fc-nilcard"><b>Your deal · you set it</b><span>The collective pays it. Your number is his number.</span></div>
      <div class="fc-nilcard"><b>His own deals · for reference</b><span>Brand and ad money he earns himself. Not money you pay.</span></div></div>`;
  const buttons = `<div class="fc-acts"><button class="st-btn small glass" data-fc="nilusual">Fill blanks with the going rate</button><button class="st-btn small glass" data-fc="nilclear">Clear every deal</button></div>`;
  const deal = `<p class="sub">A blank box pays nothing, and nothing is ever filled in for you. The going rate beside each box is only a guide: what players in his role usually get. Type a number and press Set or Enter. Every deal stands from one money year to the next until you change it, in thousands a year. A player paid well under his worth hears about it, and the portal is a phone call away; each answers when you set his number. ${m(set)} set in all.</p>`;
  return `<div class="film">${fcHero({ label: `${teamLink(x.team)} · Money`, title: "NIL", line: `<span>${two ? "Revenue sharing and the collective" : "The collective"}${x.big_booster ? " · a big booster behind it" : ""}</span>`, sub: esc(x.trend_line), panel })}
  <div class="pad fc-pad">
    <div class="fc-bar"><div class="st-label">The roster's deals</div>${fcGoto([["roster", "Squad"], ["portal_desk", "Transfer portal"], ["recruit_board", "Recruiting board"], ["depth", "Depth chart"]])}</div>
    ${cards}
    <div class="fc-nilgrid"><div class="panel"><h2>Where the money goes <small>this money year</small></h2>${ledger}</div>
      <div class="panel"><h2>Promised to recruits <small>paid once they enroll</small></h2><div class="items">${promised}</div></div>
      <div class="panel"><h2>Portal offers <small>paid if he comes</small></h2><div class="items">${portal}</div></div>
      <div class="panel"><h2>The collective by year</h2>${hist}</div></div>
    <div class="fc-bar"><div class="st-label">Every player's deal</div>${buttons}</div>
    ${deal}
    ${table("fcnil", cols, x.rows, { sort: "now", groupBy: null, groups })}
  </div></div>`;
};

// ---------- walk-ons: scholarships and the open tryout ----------
// Your walk-ons, the scholarship call on each, and the open tryout for the
// students on campus between summer enrollment and the first weeks of fall
// camp (lane B of the Jan 1 start). The ranges are your staff's read from
// one day of drills.
screens.walkons_desk = async () => {
  const x = await api("walkons_desk");
  setColors(x.team.colors);
  const t = x.tryout;
  const aid = x.aid.none
    ? ["None", x.aid.none]
    : x.aid.whole
      ? [`${x.squad.n}/${x.squad.cap}`, "Roster places · any can carry aid"]
      : [`${x.aid.on}/${x.aid.limit}`, "On scholarship"];
  const earned = x.rows.filter((r) => r.earned).length;
  const panel = `<h2>Season ${x.season} <small>walk-ons and aid</small></h2>${fcTiles([aid, [String(x.rows.length), "Walk-ons"], [String(earned), "In the two-deep"], [x.squad.whole ? `${x.squad.n}/${x.squad.cap}` : `${x.squad.n}`, x.squad.whole ? "House roster limit" : `On the squad · room to ${x.squad.cap}`]])}`;
  const cols = [
    { k: "name", t: "Player", l: true, f: (r) => playerLink(r.id, r.name) },
    { k: "pos", t: "Pos", l: true },
    { k: "cls", t: "Class", l: true, v: (r) => classRank(r.cls) },
    { k: "role", t: "Role", l: true, v: (r) => (r.earned ? 1 : 0), f: (r) => `${r.earned ? fcDot("good") : ""}${esc(r.role || "-")}` },
    { k: "read", t: "Read", div: true, v: (r) => mid(r.read), f: (r) => range(r.read, null, { p: r.id, k: "ovr" }) },
    { k: "words", t: "Scholarship", l: true, wrap: true, f: (r) => `<span class="sub">${esc(r.words)}</span>` },
    { k: "act", t: "", l: true, f: (r) => (r.can ? `<button class="st-btn small primary" data-fc="woaward" data-id="${r.id}" data-name="${esc(r.name)}" data-words="${esc(r.words)}">Put him on scholarship</button>` : "") },
  ];
  const mine = x.rows.length
    ? table("fcwo", cols, x.rows, { sort: "role", groupBy: null })
    : `<div class="panel"><div class="empty">No walk-ons on the roster.${t.can ? " The open tryout below is the way to add one." : ""}</div></div>`;
  const came = t.came.length
    ? table("fcwot", [
        { k: "name", t: "Student", l: true, f: (r) => playerLink(r.id, r.name, "Recruiting") },
        { k: "pos", t: "Pos", l: true },
        { k: "from", t: "From", l: true, f: (r) => `${esc(r.from || "-")}${r.juco ? ' <span class="st-pill">JUCO</span>' : ""}` },
        { k: "state", t: "Home", l: true },
        { k: "ht", t: "Ht", l: true },
        { k: "wt", t: "Wt" },
        { k: "forty", t: "40", tip: "Hand-timed on the day", f: (r) => (r.forty ? r.forty.toFixed(2) : "-") },
        { k: "grade", t: "Read", div: true, tip: "Your staff's read from one day of drills: wide, and wider with a weaker eye", v: (r) => (r.low + r.high) / 2, f: (r) => `<b>${r.low}-${r.high}</b> <small class="sub">${esc(r.reader)}</small>` },
        { k: "act", t: "", l: true, f: (r) => (r.kept ? `${fcDot("good")}Kept` : r.gone ? '<span class="muted">Gone</span>' : `<button class="st-btn small glass" data-fc="wokeep" data-id="${r.id}">Keep him</button>`) },
      ], t.came, { sort: "grade", groupBy: null })
    : "";
  const tryout = t.held
    ? `<div class="panel"><h2>The open tryout <small>held ${esc(t.held)} · ${fcPlural(t.came.length, "student")} came</small></h2><p class="sub">Keep any you want as walk-ons: no aid, a place on the squad, and the chance to earn a scholarship later. Each one kept takes a roster place.</p>${came || '<div class="empty">Nobody came out.</div>'}</div>`
    : `<div class="panel"><h2>The open tryout <small>${esc(t.from)} to ${esc(t.to)}</small></h2>
      <p class="sub">Once a season, students already on campus try out for your staff: a forty, the drills, a day's look. Up to ${t.looks} come out. Your position coaches read them; nobody has a file.</p>
      ${t.can ? '<div class="fc-acts"><button class="st-btn small primary" data-fc="wohold">Hold the tryout</button></div>' : `<div class="empty">${esc(t.why || "")}</div>`}</div>`;
  return `<div class="film">${fcHero({ label: `${teamLink(x.team)} · Roster`, title: "Walk-ons", line: "<span>Scholarships for the walk-ons who earn one, and the open tryout</span>", panel })}
  <div class="pad fc-pad">
    <div class="fc-bar"><div class="st-label">Your walk-ons</div>${fcGoto([["roster", "Squad"], ["depth", "Depth chart"], ["nil_desk", "NIL"], ["recruit_find", "Find recruits"]])}</div>
    ${mine}
    ${tryout}
  </div></div>`;
};

// ---------- the desk's clicks and edits ----------
// Captured before the frame's own handler, so a button in a row acts and
// never opens the row's first link.
async function fcDo(cmd, args, msg) {
  // Money boxes typed in but not sent yet, in other rows, survive the redraw.
  const sent = args && args.id != null ? String(args.id) : null;
  const dirty = [...document.querySelectorAll("input[data-was]")]
    .filter((i) => i.value !== i.dataset.was && i.dataset.id !== sent)
    .map((i) => [i.dataset.fcin, i.dataset.id, i.value]);
  try {
    const r = await api(cmd, args);
    toast(msg || (r && r.text) || "Done.");
    await render($(".view") && $(".view").scrollTop);
    for (const [k, id, v] of dirty) {
      const i = document.querySelector(`input[data-fcin="${k}"][data-id="${id}"]`);
      if (i) {
        i.value = v;
        i.dispatchEvent(new Event("input", { bubbles: true }));
      }
    }
    return r;
  } catch (e) {
    toast(e.message || String(e));
  }
}
document.addEventListener(
  "click",
  async (ev) => {
    const el = ev.target.closest("[data-fc]");
    // A link to a page is app.js's to open, whatever else marks it.
    if (!el || el.matches("a[data-go]")) return;
    ev.stopPropagation();
    ev.preventDefault();
    const d = el.dataset;
    const id = d.id ? Number(d.id) : undefined;
    switch (d.fc) {
      case "tab": S.keep[d.key].tab = d.v; render(); break;
      case "seg": {
        const k = S.keep[d.key];
        k[d.f] = d.v;
        render($(".view").scrollTop);
        break;
      }
      case "pick": S.keep.fcb.sel = id; render($(".view").scrollTop); break;
      case "mapst": S.keep.fcm.st = d.v; render($(".view").scrollTop); break;
      case "planpos": { const k = keep("fcb", {}); k.tab = "board"; k.pos = d.v; k.cls = "next"; render(); break; }
      case "add": fcDo("recruit_add", { id }); break;
      case "l7sign": fcDo("chain_sign", { id }); break;
      case "l7decline": fcDo("chain_decline", { id }); break;
      case "l7all": fcDo("chain_sign_all", {}); break;
      case "l7back": fcDo("chain_back", { id }); break;
      case "l7see": go("recruit_file", { id }); break;
      case "remove": fcDo("recruit_remove", { id }); break;
      case "offer": fcDo("recruit_offer", { id }); break;
      case "l7film": api("scout_ask", { id, kind: "Watch his film now" }).then((r) => { toast(r.said); render($(".view") && $(".view").scrollTop); }).catch((e) => toast(e.message || String(e))); break;
      case "pull": if (await sure(...pullWords(d.free === "1"))) fcDo("recruit_pull", { id }); break;
      // The fresh start's reset: every old offer, or all but the commitments.
      case "reset": {
        const f = FC_FRESH;
        if (!f) break;
        const keepC = d.keep === "1";
        const n = keepC ? f.offers - f.commits : f.offers;
        if (!(await sure(`Pull ${n === 1 ? "the old staff's offer" : `the old staff's ${n} offers`}?`, `${keepC ? "The commitments stay. " : f.commits ? `${fcCap(fcPlural(f.commits, "commitment"))} ${f.commits === 1 ? "goes" : "go"} with ${f.commits === 1 ? "its offer" : "their offers"}. ` : ""}Your fresh start is on: a new staff resets the board, so it costs you nothing. They leave your board; Find recruits still has every one of them.`, "Pull them"))) break;
        // The ones that leave the board leave the ticks too.
        const gone = [...FC_ROWS.values()].filter((r) => r.inherited && r.offered && (!keepC || r.standing !== "committed")).map((r) => r.id);
        if (await fcDo("recruit_reset", { keep_commits: keepC })) {
          const st = S.keep["tbl:fcboard"];
          if (st) st.picked = st.picked.filter((x) => !gone.includes(x));
        }
        break;
      }
      case "oldpick": { const k = keep("fcb", {}); Object.assign(k, { tab: "board", offers: "old", cls: "all", pos: "ALL" }); render(); break; }
      // The move bar's own buttons on the full board (its ticks are the table's).
      case "mbpull": case "mbunboard": {
        const st = S.keep["tbl:fcboard"];
        if (!st) break;
        const rows = st.picked.map((x) => FC_ROWS.get(x)).filter(Boolean);
        if (d.fc === "mbunboard") {
          const ids = rows.filter((r) => r.on_board && !r.offered).map((r) => r.id);
          if (!ids.length) break;
          const r = await fcDo("recruit_remove_many", { ids });
          if (r) st.picked = st.picked.filter((x) => !ids.includes(x));
          break;
        }
        const ids = rows.filter((r) => r.offered && !["signed", "enrolled"].includes(r.standing)).map((r) => r.id);
        if (!ids.length) break;
        // The pulled stay ticked, in view: Take off the board is the next click.
        if (!(await sure(...pullWords(rows.filter((r) => ids.includes(r.id)).every((r) => r.free_pull), ids.length)))) break;
        try {
          const r = await api("recruit_pull_many", { ids });
          toast(r.text);
          const k = keep("fcb", {});
          if (k.offers === "old" || k.offers === "out") k.offers = "pulled";
          render($(".view") && $(".view").scrollTop);
        } catch (e) {
          toast(e.message || String(e));
        }
        break;
      }
      case "prio": fcDo("recruit_priority", { id, on: d.on === "1" }); break;
      case "move": fcDo("recruit_move", { id, to: Number(d.to) }); break;
      case "promise": {
        // Both boxes go together: a blank one is no money in that pot.
        const row = el.closest(".fc-nilrow");
        const inp = (key) => row.querySelector(`[data-fcin="${key}"]`);
        const val = (key) => { const i = inp(key); return i && i.value !== "" ? Math.max(0, Number(i.value)) * 1000 : null; };
        const nil = val("promise"), coll = val("pcoll");
        const a = { id };
        if (nil != null) a.nil = nil;
        if (coll != null) a.coll = coll;
        fcDo("recruit_promise", a);
        break;
      }
      case "promiseclear": fcDo("recruit_promise", { id }); break;
      case "visit": {
        const sel = document.querySelector(`[data-fcsel="v-${id}-${d.kind}"]`);
        if (!sel) break;
        const r = await fcDo("visit_set", { id, kind: d.kind, date: Number(sel.value) });
        if (r && r.declined) toast(r.text);
        break;
      }
      case "vcancel": fcDo("visit_cancel", { id, kind: d.kind }); break;
      case "duty": fcDo("college_duty", { duty: d.duty, owner: d.owner }); break;
      case "fill": fcDo("college_fill", { on: d.on === "1" }); break;
      case "poffer": fcDo("portal_offer", fcPortalMoney(id)); break;
      case "ppull": fcDo("portal_pull", { id }); break;
      case "pcall":
        try {
          const r = await api("portal_call", { id });
          go("people_scene", { id: r.scene });
        } catch (e) {
          toast(e.message || String(e));
        }
        break;
      case "pvisit": {
        const sel = document.querySelector(`[data-fcsel="pv-${id}-${d.kind}"]`);
        if (!sel) break;
        const r = await fcDo("portal_visit_set", { id, kind: d.kind, date: Number(sel.value) });
        if (r && r.declined) toast(r.text);
        break;
      }
      case "pvcancel": fcDo("portal_visit_cancel", { id, kind: d.kind }); break;
      case "pforget": fcDo("portal_forget", { id }); break;
      case "pcontact":
        if (await sure(`Reach out to ${d.name}?`, "He isn't in the portal: a call now is tampering. It may sway him your way; about one call in eight gets out, and his coach and the press hear about it.", "Make the call")) fcDo("portal_contact", { id });
        break;
      case "talk": fcDo("keep_talk", { id }); break;
      case "dealset": {
        // The row's changed boxes, a blank one a deal of nothing.
        const row = el.closest("tr");
        if (!row) break;
        const a = { id };
        for (const [key, arg] of [["deal", "amount"], ["dcoll", "coll"]]) {
          const i = row.querySelector(`[data-fcin="${key}"]`);
          if (i && i.value !== i.dataset.was) a[arg] = i.value === "" ? 0 : Math.max(0, Number(i.value)) * 1000;
        }
        if (a.amount != null || a.coll != null) fcDo("nil_set", a);
        break;
      }
      case "nilusual": fcDo("nil_usual", {}); break;
      case "woaward": if (await sure(`Put ${d.name} on scholarship?`, d.words, "Put him on scholarship")) fcDo("college_scholarship", { id }); break;
      case "wohold": fcDo("tryout_hold", {}); break;
      case "wokeep": fcDo("tryout_keep", { id }); break;
      case "nilclear": if (await sure("Clear every deal?", "Every box on the roster goes to $0. A blank box pays nothing on July 1 until you set it.", "Clear them")) fcDo("nil_clear", {}); break;
    }
  },
  true,
);
// Enter in a money box presses its button: Promise, Set or the portal offer.
document.addEventListener("keydown", (ev) => {
  const key = ev.key === "Enter" && ev.target.dataset && ev.target.dataset.fcin;
  if (!key) return;
  const near = (sel) => { const r = ev.target.closest(".fc-nilrow, tr, .fc-rowacts"); return r && r.querySelector(sel); };
  const btn = key === "promise" || key === "pcoll" ? near('[data-fc="promise"]') : key === "deal" || key === "dcoll" ? near('[data-fc="dealset"]') : key === "pnil" || key === "pcollp" ? near('[data-fc="poffer"]') : null;
  if (btn) { ev.preventDefault(); btn.click(); }
});
// A money box you change shows its row's Set or Change button; nothing is
// sent until you press it (Cain, 2026-11-08: clearing a box sent $0 mid-typing).
document.addEventListener("input", (ev) => {
  const t = ev.target;
  if (!t.dataset || t.dataset.was == null) return;
  // The button beside the box: on the NIL desk each pot's cell has its own
  // (either sends the whole row), on the portal the row's Change.
  const cell = t.closest(".fc-rowacts, td");
  const btn = cell && cell.querySelector("[data-fcsave]");
  if (!btn) return;
  btn.hidden = ![...cell.querySelectorAll("input[data-was]")].some((i) => i.value !== i.dataset.was);
});
// The desk's selects save on change.
document.addEventListener("change", (ev) => {
  const t = ev.target;
  if (!t.dataset) return;
  if (t.dataset.fcf2) {
    S.keep.fcp[t.dataset.fcf2] = t.value;
    render();
    return;
  }
  if (t.dataset.fcinv) {
    const k = keep("fcvinv", { id: null, kind: null });
    if (t.dataset.fcinv === "id") Object.assign(k, { id: Number(t.value), kind: null });
    else k.kind = t.value;
    render($(".view") && $(".view").scrollTop);
    return;
  }
  if (t.dataset.fcarea) {
    fcDo("recruit_area", t.value === "" ? { state: t.dataset.fcarea } : { state: t.dataset.fcarea, coach: Number(t.value) }, t.value === "" ? "Back to the staff's own split." : "Area set.");
    return;
  }
  // The money boxes send nothing on their own: Set, Change or Enter does.
});
