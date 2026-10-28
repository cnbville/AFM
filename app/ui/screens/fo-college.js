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
  nil: '<circle cx="12" cy="12" r="8.5"/><path d="M14.8 9.2c-.6-.9-1.6-1.3-2.8-1.3-1.6 0-2.8.8-2.8 2.1 0 3 5.8 1.5 5.8 4.3 0 1.3-1.3 2.2-3 2.2-1.3 0-2.4-.5-3-1.4M12 6.5v11"/>',
});
foNavAdd("college", [
  ["recruit_board", "Recruiting", "board"],
  ["recruit_find", "Find recruits", "find"],
  ["visits_desk", "Visits", "visit"],
  ["portal_desk", "Transfer portal", "portal"],
  ["nil_desk", "NIL", "nil"],
]);
Object.assign(CRUMBS, {
  recruit_board: "Recruiting board",
  recruit_find: "Find recruits",
  recruit_file: "Recruit's file",
  visits_desk: "Visits",
  portal_desk: "Transfer portal",
  nil_desk: "NIL",
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
function fcDuty(duties, key) {
  const d = duties.list.find((x) => x.key === key);
  if (!d) return "";
  const what = { board: "the recruiting board", visits: "your visits", portal: "the transfer portal", nil: "NIL" }[key];
  return `<div class="fc-duty ${d.yours ? "yours" : ""}"><span>${d.yours ? `<b>You run ${what}.</b>` : `<b>Your staff runs ${what}.</b> ${esc(d.staff)}`}</span>
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
screens.recruit_board = async (a) => {
  const x = await api("recruit_board");
  setColors(x.team.colors);
  const k = keep("fcb", { tab: "targets", sel: null, order: "mine", cls: "next", pos: "ALL" });
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
  const openPos = c.by_pos.filter((p) => p.open > 0);
  const heroLine = `${esc(fcCap(x.calendar.period))} · ${esc(x.calendar.rule)}`;
  const sub = `${commits ? `${fcCap(fcCount(commits))} in${c.signed ? ` (${c.signed} signed)` : ""}` : "No commitments yet"}, ${fcPlural(left, "place")} to fill${live.length ? `, ${fcPlural(live.length, "offer")} out` : ""}.${homes ? ` Home weekends left: ${homes}.` : ""}`;
  const panel = `<h2>The class so far <small>${esc(x.division)} · ${c.whole_roster ? `${c.limit}-man roster` : `${c.limit} signees a year`}${sg ? ` · ${esc(sg.name.toLowerCase())} ${esc(sg.date)}` : ""}</small></h2>
    ${fcTiles([
      [String(commits), commits === 1 ? "Commit" : "Commits"],
      [c.rank ? ordinal(c.rank) : "-", c.rank ? `Public rank of ${c.rank_of}` : "Not ranked yet"],
      [String(left), "Places left"],
      [sg ? String(sg.in) : "-", sg ? "Days to signing" : "No signing date"],
    ])}
    <div class="fc-open"><span class="st-label">Open spots</span>${openPos.length ? openPos.map((p) => `<button class="st-pill" data-fc="planpos" data-v="${p.pos}" title="See the ${p.pos}s on your board">${p.pos} ${p.open}</button>`).join("") : '<span class="sub">Every spot your staff planned is filled.</span>'}</div>`;
  const tabs = [["targets", "Top targets"], ["board", "Full board", x.rows.length], ["map", "Map"], ["plan", "Class plan and staff"]];
  let body = "";
  if (k.tab === "targets") body = await fcTargets(x, k);
  else if (k.tab === "board") body = fcBoardTable(x, k, yours);
  else if (k.tab === "map") body = await fcMap();
  else body = fcPlan(x);
  return `<div class="film">${fcHero({ label: `${teamLink(x.team)} · ${esc(x.conference)} · Recruiting`, title: `Class of ${year}`, line: heroLine, sub, big: String(year).slice(2), panel })}
  <div class="pad fc-pad">
    <div class="fc-bar">${fcTabs("fcb", tabs, k.tab)}${fcGoto([["recruit_find", "Find recruits"], ["visits_desk", "Visits"], ["scouting", "Scouting"], ["calendar", "Calendar"], ["roster", "Squad"]])}</div>
    ${yours ? "" : fcDuty(x.duties, "board")}
    ${body}
  </div></div>`;
};

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
function fcFile(f, ctx, compact) {
  const [c1, c2] = f.hs_colors || ["#444", "#222"];
  const schools = f.schools.length
    ? f.schools
        .map((s) => `<div class="fc-school ${s.yours ? "yours" : ""}"><span class="fc-n">${s.n}</span>${mark(s.team, 28)}<span><b>${teamLink(s.team)}</b><small>${esc(s.status)}</small></span>${s.pill ? `<span class="st-pill ${s.pill === "Leads" ? "good" : "gold"}">${esc(s.pill)}</span>` : ""}</div>`)
        .join("")
    : '<div class="empty">No offers yet. The first one is remembered.</div>';
  const read = `<div class="fc-read"><div>${range(f.read, null, { p: f.id, k: "ovr" })}</div><div><b>${fcPot(f.pot)}</b>${f.pot ? `<small class="sub">How sure: ${esc(words(f.pot.sure))}</small>` : ""}</div></div>
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
        <div class="fc-interest ${fcTone(f.tone)}">${fcDot(fcTone(f.tone))}<b>${esc(f.interest)}</b></div><p class="sub">${esc(f.reason)}</p></div>
      <div class="fc-block"><div class="fc-h"><span class="st-label">His top schools</span><small class="sub">${f.schools_n > 1 ? `${f.schools_n} schools after him · ` : ""}as your staff hears it</small></div><div class="fc-schools">${schools}</div></div>
      <div class="fc-next"><span class="st-label">Next contact</span><span>${nextLine}</span></div>
    </div>`;
  const right = `<div class="fc-right">${actions}${visits}</div>`;
  const family = `<div class="panel fc-family"><h2>Family and transcript <small>what your staff has learned</small></h2>
      <dl class="kv"><dt>Who decides</dt><dd>${esc(f.decides)}</dd><dt>Home</dt><dd>${esc(f.town)}, ${Math.round(f.miles)} miles from campus</dd><dt>Transcript</dt><dd>${esc(f.gpa)} core GPA${f.test ? ` · test ${esc(f.test)}` : ""}</dd><dt>NCAA</dt><dd>${esc(f.qualifies)}</dd><dt>Visits taken</dt><dd>${fcPlural(f.officials_used, "official visit")} of five</dd></dl></div>`;
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

// The buttons for a recruit: board, offer, your own time, his NIL number.
function fcActions(f) {
  const b = [];
  const closed = ["signed", "enrolled", "signed_elsewhere", "gone"].includes(f.standing);
  if (!f.board_yours) {
    b.push(`<p class="sub">Your coordinator runs the board: his offers and order stand until you take it back.</p>`);
  } else if (!closed) {
    if (f.offered) b.push(`<button class="st-btn small glass" data-fc="pull" data-id="${f.id}">Pull the offer</button>`);
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
    // Your own number for him, whoever runs NIL (minor bug 59).
    if (f.board_yours) {
      nil = `<div class="fc-nil"><span class="st-label">NIL promise</span>
        <div class="fc-nilrow"><label class="fc-moneyin">$<input type="number" min="0" step="5" value="${f.nil != null ? Math.round(f.nil / 1000) : ""}" placeholder="${Math.round(f.usual / 1000)}" data-fcin="promise" data-id="${f.id}">thousand a year</label><button class="st-btn small primary" data-fc="promise" data-id="${f.id}">Promise</button></div>
        <small class="sub">The collective's usual offer: ${money(f.usual)}. The going rate for his position tops out near ${money(f.nil_max)}, but only your budget limits you: ${money(f.budget.left)} left this year.</small>
        ${f.nil != null ? `<button class="st-btn small glass" data-fc="promiseclear" data-id="${f.id}">Back to the usual offer</button>` : ""}</div>`;
    } else {
      nil = `<div class="fc-nil"><span class="st-label">NIL</span><small class="sub">Your collective offers about ${money(f.usual)} a year.</small></div>`;
    }
  }
  return `<div class="fc-block"><div class="fc-h"><span class="st-label">Your moves</span></div><div class="fc-acts">${b.join("")}</div>${nil}</div>`;
}

// Visits the rules allow from today, each kind with its next days or why not.
function fcVisitPlanner(f) {
  if (!f.options) return "";
  if (!f.visits_yours) return `<div class="fc-block"><div class="fc-h"><span class="st-label">Visits</span></div><p class="sub">Your staff sets the visits: the board's top names come in on home weekends.</p></div>`;
  const rows = f.options
    .map((o) => {
      if (o.set) return `<div class="fc-vopt"><b>${esc(o.name)}</b><span class="fc-vset"><span class="fc-vdone">${fcDot("good")}Set for ${esc(o.set.date)}</span><button class="st-btn small glass" data-fc="vcancel" data-id="${f.id}" data-kind="${o.key}">Call off</button></span></div>`;
      if (!o.days.length) return `<div class="fc-vopt"><b>${esc(o.name)}</b><small class="sub">${esc(o.why || "Not possible this year.")}</small></div>`;
      return `<div class="fc-vopt"><b>${esc(o.name)}</b><span class="fc-vset"><select data-fcsel="v-${f.id}-${o.key}">${o.days.map((d) => `<option value="${d.d}">${esc(d.date)}${d.word ? " · " + esc(d.word.replace(/ period$/, "").replace(/^home game against /, "vs ")) : ""}</option>`).join("")}</select><button class="st-btn small glass" data-fc="visit" data-id="${f.id}" data-kind="${o.key}">Invite</button></span></div>`;
    })
    .join("");
  return `<div class="fc-block"><div class="fc-h"><span class="st-label">Plan a visit</span><small class="sub">on the NCAA's calendar</small></div><div class="fc-vopts">${rows}</div></div>`;
}

// The full board: everyone, your order or by position, every column a coach reads.
function fcBoardTable(x, k, yours) {
  const year = x.class.year;
  const classes = [["next", `Class of ${year}`], ["later", "Younger classes"], ["all", "Everyone"]];
  let rows = x.rows.filter((r) => (k.cls === "next" ? r.class_of === year || r.juco : k.cls === "later" ? r.class_of > year && !r.juco : true));
  if (k.pos !== "ALL") rows = rows.filter((r) => r.pos === k.pos);
  const posOpts = [["ALL", "All"], ...x.class.by_pos.map((p) => [p.pos, p.pos])].filter(([v]) => v === "ALL" || x.rows.some((r) => r.pos === v));
  const cols = [
    { k: "place", t: "#", tip: "Your order: commitments first, then this order decides who can commit when the class fills", f: (r) => fcLink("recruit_file", String(r.place), { id: r.id }) },
    { k: "name", t: "Recruit", l: true, f: (r) => `${playerLink(r.id, r.name, "Recruiting")}${r.priority ? ' <span class="fc-yourtime" title="You give him your own time">●</span>' : ""}` },
    { k: "pos", t: "Pos", l: true },
    { k: "cls", t: "Class", l: true, v: (r) => r.class_of },
    { k: "stars", t: "★", tip: "The services' stars", f: (r) => fcStars(r.stars) },
    { k: "nat", t: "Public", l: true, tip: "The services' rank at his position and in his state", v: (r) => -(r.nat || 9999), f: fcPublic },
    { k: "school", t: "School · Hometown", l: true, div: true, v: (r) => (r.hs ? r.hs.name : ""), f: (r) => `${fcSchool(r)} <small class="sub">· ${esc(r.town)}</small>` },
    { k: "read", t: "Read", div: true, tip: "Your position coach's read of his play now", v: (r) => mid(r.read), f: (r) => range(r.read, null, { p: r.id, k: "ovr" }) },
    { k: "pot", t: "Ceiling", l: true, tip: "Your staff's words on how far he can go", v: (r) => (r.pot ? r.pot.tier : ""), f: (r) => fcPot(r.pot) },
    { k: "interest", t: "His interest", l: true, div: true, f: (r) => `<span title="${esc(r.reason)}">${fcDot(fcTone(r.tone))}${esc(r.interest)}</span>` },
    { k: "leader", t: "Leader", l: true, v: (r) => (r.leader ? r.leader.name : ""), f: (r) => (r.leader ? teamLink(r.leader) : '<span class="muted">-</span>') },
    { k: "offers", t: "Offers" },
    { k: "offered", t: "Your offer", l: true, div: true, v: (r) => (r.offered ? 1 : 0), f: fcOfferCell },
    { k: "next", t: "Next", l: true, v: (r) => (r.next ? r.next.date : ""), f: (r) => (r.next ? `${esc(r.next.date)} <small class="sub">${esc(r.next.what)}</small>` : '<span class="muted">-</span>') },
    ...(yours
      ? [{ k: "act", t: "", l: true, v: () => 0, f: (r) => {
          const closed = ["signed", "enrolled", "signed_elsewhere", "gone"].includes(r.standing);
          return `<span class="fc-rowacts"><button class="fc-ib" data-fc="move" data-id="${r.id}" data-to="${Math.max(0, r.place - 2)}" title="Up one place">▲</button><button class="fc-ib" data-fc="move" data-id="${r.id}" data-to="${r.place}" title="Down one place">▼</button><button class="fc-ib ${r.priority ? "on" : ""}" data-fc="prio" data-id="${r.id}" data-on="${r.priority ? 0 : 1}" title="${r.priority ? "Leave him to his position coach" : "Give him your own time"}">●</button>${closed ? "" : r.offered ? `<button class="st-btn small glass" data-fc="pull" data-id="${r.id}">Pull</button>` : `<button class="st-btn small primary" data-fc="offer" data-id="${r.id}">Offer</button>`}</span>`;
        } }]
      : []),
  ];
  const note = `${rows.length} of ${x.rows.length} shown. Rest on a row for his staff's reason; a row opens his file.`;
  return `<div class="filters">${fcSeg("fcb", "cls", classes, k.cls)}${fcSeg("fcb", "pos", posOpts, k.pos)}${fcSeg("fcb", "order", [["mine", "Your order"], ["pos", "By position"]], k.order)}<span class="sub fc-note">${esc(note)}</span></div>
    ${table("fcboard", cols, rows, { groupBy: k.order === "pos" ? (r) => r.pos : null, empty: "Nobody on the board in this view.", groups: [{ t: "Recruit", span: 7 }, { t: "Your staff", who: "position coaches", span: 2, div: true }, { t: "His recruitment", span: 3, div: true }, { t: "Yours", span: yours ? 3 : 2, div: true }] })}`;
}

// The class plan: the spots your staff would fill, the NCAA's limits, who
// runs what, and the dates ahead.
function fcPlan(x) {
  const c = x.class;
  const rows = c.by_pos.map((p) => `<tr><td class="l"><a class="link" data-fc="planpos" data-v="${p.pos}">${p.pos}</a></td><td>${p.need}</td><td>${p.committed}</td><td>${p.signed}</td><td>${p.offers}</td><td><b>${p.open ? p.open : '<span class="muted">0</span>'}</b></td></tr>`).join("");
  const limit = c.whole_roster
    ? `From 2025 the House settlement caps the whole roster at ${c.limit}, with no signee limit: your roster has room for ${c.room} more.`
    : `${c.signees} of the NCAA's ${c.limit} signees a year counted so far; your 85 scholarships leave room for ${c.room}.`;
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

// Finding recruits is its own screen now: screens/find.js (update 2, lane 7).

// ---------- visits ----------
screens.visits_desk = async () => {
  const x = await api("visits_desk");
  setColors(x.team.colors);
  const yours = x.duties.list.find((d) => d.key === "visits").yours;
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
        { k: "act", t: "", l: true, f: (r) => (yours ? `<button class="st-btn small glass" data-fc="vcancel" data-id="${r.id}" data-kind="${r.key}">Call off</button>` : "") },
      ], x.ahead, { empty: "Nothing set." })
    : `<div class="panel"><div class="empty">No visits set. Open a recruit's file to invite him: ${fcLink("recruit_board", "your board")}.</div></div>`;
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
  const rules = `<dl class="kv fc-rules"><dt>Official visits</dt><dd>Paid by you, 48 hours on campus. Five per recruit and one per school; seniors from September 1${x.junior_officials ? ", juniors April 1 to June 30" : ""}. Never in a dead period.</dd>
    <dt>Unofficial visits</dt><dd>Paid by the family, any number, outside dead periods.</dd>
    <dt>Home visits</dt><dd>The head coach in his family's living room: seniors only, in a contact period, once a cycle.</dd>
    <dt>Drop-ins</dt><dd>Watching him practise at his school in an evaluation period: no talking off campus.</dd></dl>`;
  const set = x.ahead.filter((v) => v.key === "official").length;
  const panel = `<h2>This cycle <small>Class of ${x.class}</small></h2>${fcTiles([[String(x.ahead.length), "Visits set"], [String(set), "Officials set"], [String(x.officials), "Officials taken"], [String(x.taken), "Visits taken"]])}`;
  return `<div class="film">${fcHero({ label: `${teamLink(x.team)} · Recruiting`, title: "Visits", line: `${esc(fcCap(x.calendar.period))} · ${esc(x.calendar.rule)}`, panel })}
  <div class="pad fc-pad">
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
screens.portal_desk = async () => {
  const k = keep("fcp", { tab: "entrants", pos: "ALL", div: "ALL" });
  const x = await api("portal_desk", { pos: k.pos, division: k.div });
  setColors(x.team.colors);
  const open = x.windows.find((w) => w.open);
  const next = x.windows.find((w) => !w.open && w.in > 0);
  const line = open ? `<span><b>${esc(open.name)} is open</b> · to ${esc(open.closes)}</span>` : next ? `<span>The ${esc(next.name.toLowerCase())} opens ${esc(next.opens)} (${fcIn(next.in)})</span>` : "<span>No window open</span>";
  const leaving = x.roster.filter((r) => r.rank <= 1).length;
  const panel = `<h2>The portal <small>where it stands today</small></h2>${fcTiles([[String(x.in_portal), "In the portal"], [String(x.targets.filter((t) => !t.settled).length), "Your targets"], [String(leaving), "Yours at risk"], [String(x.talks), "Talks held"]])}`;
  const tabs = [["entrants", "Entrants", x.shown], ["targets", "Your targets", x.targets.length], ["roster", "Your roster", x.roster.length], ["whispers", "Quiet words", x.whispers.length]];
  const nilIn = (r) => (x.nil_era && x.nil_yours ? `<label class="fc-moneyin small">$<input type="number" min="0" step="5" value="${r.nil != null ? Math.round(r.nil / 1000) : ""}" placeholder="${Math.round(r.usual / 1000)}" data-fcin="pnil" data-id="${r.id}">k</label>` : "");
  let body = "";
  if (k.tab === "entrants") {
    const POS = ["QB", "RB", "WR", "TE", "OT", "IOL", "EDGE", "DT", "LB", "CB", "S", "K", "P", "LS"];
    const cols = [
      { k: "name", t: "Player", l: true, f: (r) => `${playerLink(r.id, r.name, "Recruiting")}${r.contacted ? ' <span class="st-pill" title="You reached out before he entered">Talked</span>' : ""}` },
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
      { k: "offered", t: "Your offer", l: true, div: true, v: (r) => (r.offered ? 1 : 0), f: (r) => (x.yours ? (r.offered ? `<span class="fc-rowacts"><span class="st-pill good">Offered</span>${nilIn(r)}<button class="st-btn small glass" data-fc="ppull" data-id="${r.id}">Pull</button></span>` : `<span class="fc-rowacts">${nilIn(r)}<button class="st-btn small primary" data-fc="poffer" data-id="${r.id}">Offer</button></span>`) : r.offered ? "Offered" : "") },
    ];
    const posSel = `<select class="fc-select" data-fcf2="pos"><option value="ALL">Every position</option>${POS.map((p) => `<option ${k.pos === p ? "selected" : ""}>${p}</option>`).join("")}</select>`;
    const divs = [["ALL", "Every division"], ["FBS", "FBS"], ["FCS", "FCS"], ["D2", "D2"], ["NAIA", "NAIA"], ["D3", "D3"], ["JUCO", "JUCO"]];
    body = `<div class="filters">${posSel}${fcSeg("fcp", "div", divs, k.div)}<span class="sub fc-note">${esc(x.sit)}</span></div>
      ${table("fcportal", cols, x.entrants, { sort: null, empty: open ? "Nobody in the portal matches." : "The portal is closed: entrants come when a window opens.", groups: [{ t: "Player", span: 7 }, { t: "Your staff", span: 2, div: true }, { t: "His move", span: 3, div: true }, { t: "Yours", span: 1, div: true }] })}`;
  } else if (k.tab === "targets") {
    body = table("fcptargets", [
      { k: "name", t: "Player", l: true, f: (r) => playerLink(r.id, r.name, "Recruiting") },
      { k: "pos", t: "Pos", l: true },
      { k: "from", t: "From", l: true, v: (r) => r.from.name, f: (r) => teamLink(r.from) },
      { k: "status", t: "Where it stands", l: true, div: true },
      { k: "offered", t: "Offered", l: true, f: (r) => esc(r.offered || "-") },
      { k: "nil", t: "NIL", f: (r) => (r.nil != null ? money(r.nil) : '<span class="muted">usual</span>') },
      { k: "decide", t: "Decides", l: true, f: (r) => esc(r.decide || "-") },
      { k: "act", t: "", l: true, f: (r) => (r.settled ? "" : `<button class="st-btn small glass" data-fc="pforget" data-id="${r.id}">Forget him</button>`) },
    ], x.targets, { empty: "No targets yet. Offer an entrant, or reach out quietly to a player with whispers about him." });
  } else if (k.tab === "roster") {
    body = `<p class="sub">Your players by how likely they are to leave, in your staff's words. A talk sways a player the way your relationships with players do: a good one keeps him, a poor one can push him.</p>
    ${table("fcproster", [
      { k: "rank", t: "", v: (r) => -r.rank, f: (r) => fcDot(fcTone(r.tone)) },
      { k: "name", t: "Player", l: true, f: (r) => playerLink(r.id, r.name, "People") },
      { k: "pos", t: "Pos", l: true },
      { k: "cls", t: "Class", l: true, v: (r) => classRank(r.cls) },
      { k: "role", t: "Role", l: true, v: (r) => r.role || "", f: (r) => esc(r.role || "-") },
      { k: "read", t: "Read", div: true, v: (r) => mid(r.read), f: (r) => range(r.read, null, { p: r.id, k: "ovr" }) },
      { k: "risk", t: "Mood about staying", l: true, div: true, v: (r) => -r.rank },
      { k: "why", t: "What your staff hears", l: true, wrap: true, tip: "The strongest pull away this year, and the strongest reason he has to stay", f: (r) => `${esc(r.why ? r.why[0].toUpperCase() + r.why.slice(1) : "-")}${r.hold ? `<br><small class="sub">Keeping him: ${esc(r.hold)}</small>` : ""}` },
      { k: "nil", t: "NIL now", f: (r) => (r.nil ? money(r.nil) : '<span class="muted">-</span>') },
      { k: "talk", t: "Your talk", l: true, div: true, v: (r) => (r.talk ? 1 : 0), f: (r) => (r.talk ? `<small>${esc(r.talk.date)}: ${esc(r.talk.answer)}</small>` : r.in_portal || !x.yours ? "" : `<button class="st-btn small glass" data-fc="talk" data-id="${r.id}">Talk to him</button>`) },
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
      { k: "contacted", t: "Your word", l: true, div: true, v: (r) => (r.contacted ? 1 : 0), f: (r) => (r.contacted ? `${esc(r.contacted)}${r.leaked ? ' <span class="st-pill gold">Word got out</span>' : ""}` : x.yours ? `<button class="st-btn small glass" data-fc="pcontact" data-id="${r.id}" data-name="${esc(r.name)}">Reach out</button>` : "") },
    ], x.whispers, { empty: "No whispers around the country right now." })}`;
  }
  return `<div class="film">${fcHero({ label: `${teamLink(x.team)} · Roster`, title: "Transfer portal", line, sub: esc(x.rule), panel })}
  <div class="pad fc-pad">
    <div class="fc-bar">${fcTabs("fcp", tabs, k.tab)}${fcGoto([["roster", "Squad"], ["depth", "Depth chart"], ["nil_desk", "NIL"], ["recruit_board", "Recruiting board"]])}</div>
    ${x.yours ? "" : fcDuty(x.duties, "portal")}
    ${body}
  </div></div>`;
};

// ---------- NIL ----------
screens.nil_desk = async () => {
  const x = await api("nil_desk");
  setColors(x.team.colors);
  const b = x.budget;
  if (!x.era) {
    return `<div class="film">${fcHero({ label: `${teamLink(x.team)} · Money`, title: "NIL", line: "<span>Before July 1, 2021 a college player can't be paid for his name, image or likeness.</span>" })}
  <div class="pad fc-pad"><div class="fc-bar"><span></span>${fcGoto([["roster", "Squad"], ["portal_desk", "Transfer portal"], ["recruit_board", "Recruiting board"]])}</div>
    <div class="panel"><div class="empty">The NCAA's amateur rules hold: scholarships, cost of attendance and nothing more. NIL opens on July 1, 2021.</div></div></div></div>`;
  }
  const deals = fcSum(x.rows.map((r) => ({ v: r.deal != null ? r.deal : 0 })), "v");
  const m = (v) => (v === 0 ? "$0" : money(v));
  const ledger = `<div class="st-ledger fc-ledger"><div><span class="st-label">Money in</span>
      <div class="item"><span>Collective</span><b>${m(b.collective)}</b></div>
      ${x.house ? `<div class="item"><span>Revenue share (House settlement)</span><b>${m(b.revenue_share)}</b></div>` : ""}
      <div class="item tot"><span>This year</span><b>${m(b.total)}</b></div></div>
    <div><span class="st-label">Money out</span>
      <div class="item"><span>Roster deals</span><b>${m(b.roster)}</b></div>
      <div class="item"><span>Promised to recruits</span><b>${m(b.recruits)}</b></div>
      <div class="item"><span>Offered in the portal</span><b>${m(b.portal)}</b></div>
      <div class="item tot"><span>Left</span><b class="${b.left < 0 ? "fc-over" : ""}">${m(b.left)}</b></div></div></div>`;
  const panel = `<h2>Money year ${x.year}-${String(x.year + 1).slice(2)} <small>paid ${esc(x.pay_day)}</small></h2>${fcTiles([[m(b.total), "Budget"], [m(b.left), "Left"], [String(x.rows.filter((r) => r.deal != null).length), "Deals set"], [x.trend, "Boosters"]])}`;
  const cols = [
    { k: "name", t: "Player", l: true, f: (r) => playerLink(r.id, r.name) },
    { k: "pos", t: "Pos", l: true },
    { k: "cls", t: "Class", l: true, v: (r) => classRank(r.cls) },
    { k: "left", t: "Yrs left" },
    { k: "role", t: "Role", l: true, v: (r) => r.role || "", f: (r) => esc(r.role || "-") },
    { k: "read", t: "Read", div: true, v: (r) => mid(r.read), f: (r) => range(r.read, null, { p: r.id, k: "ovr" }) },
    { k: "now", t: "Paid last year", div: true, f: (r) => (r.now ? money(r.now) : '<span class="muted">-</span>') },
    { k: "brand", t: "His own deals", tip: "Brand and ad money he earns himself, apart from the collective (a public estimate for the year)", v: (r) => r.brand || 0, f: (r) => (r.brand ? money(r.brand) : '<span class="muted">-</span>') },
    { k: "followers", t: "Followers", tip: "Social media followers, every platform", v: (r) => r.followers || 0, f: (r) => (r.followers ? fcFollow(r.followers) : '<span class="muted">-</span>') },
    { k: "camera", t: "On camera", l: true, hide: true, tip: "His position coach's word for how he comes across in front of a camera", f: (r) => `<span class="sub">${esc(r.camera || "")}</span>` },
    { k: "usual", t: "Collective's split", div: true, f: (r) => (r.usual ? money(r.usual) : '<span class="muted">-</span>') },
    { k: "deal", t: "Your deal", div: true, v: (r) => (r.deal != null ? r.deal : -1), f: (r) => (x.yours ? `<label class="fc-moneyin small">$<input type="number" min="0" step="5" value="${r.deal != null ? Math.round(r.deal / 1000) : ""}" placeholder="${Math.round(r.usual / 1000)}" data-fcin="deal" data-id="${r.id}">k</label>` : r.usual ? money(r.usual) : "-") },
  ];
  const promised = x.promised.length
    ? x.promised.map((p) => `<div class="item"><span class="when">${money(p.nil)}</span><span>${playerLink(p.id, p.name, "Recruiting")} <small class="sub">· ${esc(p.pos)} · ${p.stars ? p.stars + "★" : "unrated"}${p.standing === "committed" ? " · committed" : p.standing === "signed" ? " · signed" : ""}</small></span></div>`).join("")
    : '<div class="empty">No promises to recruits. Set one in a recruit\'s file.</div>';
  const portal = x.portal.length
    ? x.portal.map((p) => `<div class="item"><span class="when">${money(p.nil)}</span><span>${playerLink(p.id, p.name, "Recruiting")} <small class="sub">· ${esc(p.pos)} from ${teamLink(p.from)}</small></span></div>`).join("")
    : '<div class="empty">No portal offers carry money.</div>';
  const hist = x.history.length
    ? `<div class="tbl warm"><table><thead><tr><th class="l">Season</th><th>NIL budget</th><th>From boosters</th></tr></thead><tbody>${x.history.map((h) => `<tr><td class="l">${h.season}</td><td>${m(h.budget)}</td><td>${m(h.boosters)}</td></tr>`).join("")}</tbody></table></div>`
    : '<div class="empty">No seasons yet.</div>';
  const buttons = x.yours
    ? `<div class="fc-acts"><button class="st-btn small glass" data-fc="nilusual">Use the collective's split</button><button class="st-btn small glass" data-fc="nilclear">Clear every deal</button></div>`
    : "";
  const deal = x.yours
    ? `<p class="sub">Set each player's deal for the money year in thousands. A player left without one gets nothing from the collective; one paid well under his worth hears about it, and the portal is a phone call away. ${money(deals)} set so far.</p>`
    : "";
  return `<div class="film">${fcHero({ label: `${teamLink(x.team)} · Money`, title: "NIL", line: `<span>${x.house ? "Revenue sharing and the collective" : "The collective"}${x.big_booster ? " · a big booster behind it" : ""}</span>`, sub: esc(x.trend_line), panel })}
  <div class="pad fc-pad">
    <div class="fc-bar"><div class="st-label">The roster's deals</div>${fcGoto([["roster", "Squad"], ["portal_desk", "Transfer portal"], ["recruit_board", "Recruiting board"], ["depth", "Depth chart"]])}</div>
    ${x.yours ? "" : fcDuty(x.duties, "nil")}
    <div class="fc-nilgrid"><div class="panel"><h2>Where the money goes <small>this money year</small></h2>${ledger}</div>
      <div class="panel"><h2>Promised to recruits <small>paid once they enroll</small></h2><div class="items">${promised}</div></div>
      <div class="panel"><h2>Portal offers <small>paid if he comes</small></h2><div class="items">${portal}</div></div>
      <div class="panel"><h2>The collective by year</h2>${hist}</div></div>
    <div class="fc-bar"><div class="st-label">Every player's deal</div>${buttons}</div>
    ${deal}
    ${table("fcnil", cols, x.rows, { sort: "now", groupBy: null })}
  </div></div>`;
};

// ---------- the desk's clicks and edits ----------
// Captured before the frame's own handler, so a button in a row acts and
// never opens the row's first link.
async function fcDo(cmd, args, msg) {
  try {
    const r = await api(cmd, args);
    toast(msg || (r && r.text) || "Done.");
    render($(".view") && $(".view").scrollTop);
    return r;
  } catch (e) {
    toast(e.message || String(e));
  }
}
document.addEventListener(
  "click",
  async (ev) => {
    const el = ev.target.closest("[data-fc]");
    if (!el) return;
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
      case "remove": fcDo("recruit_remove", { id }); break;
      case "offer": fcDo("recruit_offer", { id }); break;
      case "pull": if (await sure("Pull the offer?", "A committed recruit's commitment goes with it, and his high school coach remembers.", "Pull it")) fcDo("recruit_pull", { id }); break;
      case "prio": fcDo("recruit_priority", { id, on: d.on === "1" }); break;
      case "move": fcDo("recruit_move", { id, to: Number(d.to) }); break;
      case "promise": {
        const inp = el.closest(".fc-nilrow").querySelector('[data-fcin="promise"]');
        fcDo("recruit_promise", inp.value === "" ? { id } : { id, nil: Math.max(0, Number(inp.value)) * 1000 });
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
      case "poffer": {
        const inp = document.querySelector(`[data-fcin="pnil"][data-id="${id}"]`);
        const v = inp && inp.value !== "" ? Number(inp.value) * 1000 : null;
        fcDo("portal_offer", v != null ? { id, nil: v } : { id });
        break;
      }
      case "ppull": fcDo("portal_pull", { id }); break;
      case "pforget": fcDo("portal_forget", { id }); break;
      case "pcontact":
        if (await sure(`Reach out to ${d.name}?`, "He isn't in the portal: a call now is tampering. It may sway him your way; about one call in eight gets out, and his coach and the press hear about it.", "Make the call")) fcDo("portal_contact", { id });
        break;
      case "talk": fcDo("keep_talk", { id }); break;
      case "nilusual": fcDo("nil_usual", {}); break;
      case "nilclear": if (await sure("Clear every deal?", "Every player on the roster goes back to no deal until you set one.", "Clear them")) fcDo("nil_clear", {}); break;
    }
  },
  true,
);
// Enter in a recruit's NIL box presses its Promise button.
document.addEventListener("keydown", (ev) => {
  if (ev.key === "Enter" && ev.target.dataset && ev.target.dataset.fcin === "promise") ev.target.closest(".fc-nilrow").querySelector('[data-fc="promise"]').click();
});
// Rows of the full board open the recruit's file; the desk's selects and
// the other money boxes save on change.
document.addEventListener("change", (ev) => {
  const t = ev.target;
  if (!t.dataset) return;
  if (t.dataset.fcf2) {
    S.keep.fcp[t.dataset.fcf2] = t.value;
    render();
    return;
  }
  if (t.dataset.fcarea) {
    fcDo("recruit_area", t.value === "" ? { state: t.dataset.fcarea } : { state: t.dataset.fcarea, coach: Number(t.value) }, t.value === "" ? "Back to the staff's own split." : "Area set.");
    return;
  }
  const id = Number(t.dataset.id);
  const v = t.value === "" ? null : Math.max(0, Number(t.value)) * 1000;
  if (t.dataset.fcin === "deal" && v != null) fcDo("nil_set", { id, amount: v });
  if (t.dataset.fcin === "pnil") {
    // A portal offer's money: re-sent with the offer, or saved on it if out.
    const off = t.closest("td") && t.closest("td").querySelector('[data-fc="ppull"]');
    if (off) fcDo("portal_offer", v == null ? { id } : { id, nil: v });
  }
});
