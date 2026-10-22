// Lane screens: people (docs/final-build/plan.md). Registers screens.<name>.
"use strict";
/* Lane 6 · people (final build): section 6 on your side.
 *
 *   people         The people hub: what waits for you (meetings, calls,
 *                  press), your week's hours, the room, your boss, the
 *                  takeover file, your rules, your network, league
 *                  discipline and the log of everything said.
 *   people_scene   A meeting, talk, press conference or call as a scene in
 *                  a calm room in your team's colours (screens topic 8): who
 *                  is in the room on the left, the conversation in the
 *                  middle, your moves on the right. A move takes two steps:
 *                  pick it (and its tone, in a one-on-one), read what your
 *                  staff expects and how sure they are, then say it.
 *   people_cast    A person around a team (a boss, a booster, a reporter, a
 *                  trainer): who they are, what people say about them, your
 *                  standing with them, your meetings.
 *   people_staff   Your staff office: deals, raises, letting a coach go,
 *                  the openings and the candidates, interviews and offers.
 *   Player page    A "People" tab: his mood and its reasons, his trust in
 *                  you, who's in his ear, his ties, your line to his
 *                  position coach, and Talk.
 */

I.people = '<circle cx="8" cy="8" r="3"/><circle cx="16.5" cy="9" r="2.5"/><path d="M2.5 19c.6-3.6 2.8-5.5 5.5-5.5s4.9 1.9 5.5 5.5M14 14.2c.8-.4 1.6-.6 2.5-.6 2.3 0 4.2 1.6 4.8 4.9"/>';
I.talk = '<path d="M4 5h16v10H9l-5 4z"/>';
navAdd({ go: "people", label: "People", icon: "people", sec: "team", after: "staff" });
navAdd({ go: "people_staff", label: "Staff office", icon: "staff", sec: "team", after: "people", seats: ["college", "nfl", "hs"] });
Object.assign(CRUMBS, { people: "People", people_scene: "Meeting", people_cast: "Person", people_staff: "Staff office" });

const PP_TONES = [["calm", "Calm"], ["warm", "Warm"], ["firm", "Firm"], ["fiery", "Fiery"]];
const PP_TONE_CODE = { calm: 0, warm: 1, firm: 2, fiery: 3 };
const ppCap = (t) => String(t || "").replace(/^./, (c) => c.toUpperCase());
const ppHours = (h) => (h == null ? "" : `${Math.round(h * 10) / 10} h`);
function ppMoodDot(v) {
  const c = v > 0.35 ? "var(--good)" : v > 0.1 ? "color-mix(in oklab, var(--good) 55%, var(--fg3))" : v > -0.15 ? "var(--fg3)" : v > -0.4 ? "color-mix(in oklab, var(--live) 55%, var(--fg3))" : "var(--live)";
  return `<i class="pp-dot" style="background:${c}"></i>`;
}
function ppWho(list) {
  return (list || []).filter((x) => x.link).map((x) => `<span class="pp-who">${rich(x.link)}${x.pos ? ` <small>${esc(x.pos)}</small>` : x.role ? ` <small>${esc(ppCap(x.role))}</small>` : ""}</span>`).join("");
}
function ppSceneCard(s, opts = {}) {
  const when = s.today ? '<span class="tag due">Today</span>' : s.late ? '<span class="tag due">Waiting</span>' : `<span class="tag soft">${esc(s.dateText)}</span>`;
  const state = s.done ? (s.handed ? `<span class="tag soft">Handed to ${esc(s.handed)}</span>` : '<span class="tag gold">Held</span>') : when;
  const out = s.done && s.outcome && s.outcome.length ? `<div class="pp-out">${s.outcome.slice(0, 2).map((o) => `<p>${rich(o)}</p>`).join("")}</div>` : "";
  const acts = s.done || opts.noActs ? "" : `<div class="pp-acts"><button class="st-btn small primary" data-act="ppopen" data-id="${s.id}">${s.opened ? "Go back in" : s.stops ? "Hold it" : "Take it"}</button><button class="st-btn small glass" data-act="pphand" data-id="${s.id}" title="Your staff plays it the way they would">Hand to staff</button></div>`;
  return `<article class="pp-card ${s.today ? "now" : ""} ${s.done ? "done" : ""}" data-act="ppopen" data-id="${s.id}">
    <div class="pp-card-top"><span class="st-label">${esc(s.kindName)}</span>${state}</div>
    <h3>${rich(s.title)}</h3>
    <div class="pp-card-meta">${s.place ? `<span>${esc(s.place)}</span>` : ""}${!s.today && !s.done ? "" : `<span>${esc(s.dateText)}</span>`}</div>
    ${s.who && s.who.length ? `<div class="pp-whos">${ppWho(s.who)}</div>` : ""}${out}${acts}</article>`;
}

// ---------- the hub ----------
const PP_TABS_ALL = ["Agenda", "Takeover", "Room", "Rules", "Boss", "Network", "League", "Log"];
screens.people = async (a) => {
  const x = await api("people_agenda");
  if (x.none) return `<div class="pad">${pageHead({ title: "People" })}<div class="empty">You need a job first.</div></div>`;
  const list = PP_TABS_ALL.filter((t) => t !== "Takeover" || x.takeover);
  const tab = tabOf("pptab", list, a);
  title("People", tab);
  const waiting = x.waiting.length;
  const tabList = list.map((t) => (t === "Agenda" && waiting ? { name: t, badge: waiting } : t));
  let body = "";
  if (tab === "Agenda") body = ppAgenda(x);
  else if (tab === "Takeover") body = await ppTakeover();
  else if (tab === "Room") body = await ppRoom();
  else if (tab === "Rules") body = await ppRules();
  else if (tab === "Boss") body = await ppBoss();
  else if (tab === "Network") body = await ppNetwork();
  else if (tab === "League") body = await ppLeague();
  else body = await ppLog();
  const hrs = x.hours;
  const pct = hrs.week ? Math.max(0, Math.min(100, (hrs.left / hrs.week) * 100)) : 0;
  const sub = `${esc(x.date)} · the room is <b>${esc(x.mood.words.toLowerCase())}</b> · ${rich(x.boss.link)}: ${esc(x.boss.words.toLowerCase())}`;
  const actions = `<div class="pp-week" title="Hours for people this week, beyond film and the game plan. They refill on Monday."><span class="st-label">Your week</span><b>${ppHours(hrs.left)}</b><small>of ${ppHours(hrs.week)} left</small><span class="pp-bar"><i style="width:${pct}%"></i></span></div>
    <button class="st-btn small glass" data-act="ppteamtalk" title="Talk to the whole team (an hour)">${icon("talk")} Team talk</button>`;
  return `<div class="pad">${pageHead({ kicker: rich(x.team.link), title: "People", sub, actions, goto: gotoFor("people", a, x) })}${tabs("pptab", tabList, tab)}<div class="tabbody">${body}</div></div>`;
};
GOTO.people = () => [["Staff office", "people_staff"], ["Squad", "roster"], ["Inbox", "inbox", { key: "Your people" }], ["Calendar", "calendar"], ["Your profile", "you"]];

function ppAgenda(x) {
  const waiting = x.waiting.length
    ? `<div class="pp-cards">${x.waiting.map((s) => ppSceneCard(s)).join("")}</div>`
    : '<div class="empty">Nothing is waiting for you. Your door is open: talk to a player from his page, or call a team talk.</div>';
  const recent = x.recent.length ? `<div class="pp-cards slim">${x.recent.map((s) => ppSceneCard(s, { noActs: true })).join("")}</div>` : '<div class="empty">No meetings yet.</div>';
  const goals = x.goals.length
    ? `<div class="items">${x.goals.map((g) => `<div class="item"><span class="when">${g.season}</span><span>${esc(g.words)} ${g.met === true ? '<span class="tag gold">Met</span>' : g.met === false ? '<span class="tag due">Missed</span>' : ""}</span></div>`).join("")}</div>`
    : '<div class="empty">No goals set yet: your boss sets them before the season.</div>';
  const promises = x.promises.length
    ? `<div class="items">${x.promises.map((p) => `<div class="item"><span class="when">${esc(p.due)}</span><span>${rich(p.who)}: ${esc(p.what)}${p.public ? ' <span class="tag key">In public</span>' : ""} ${p.kept === true ? '<span class="tag gold">Kept</span>' : p.kept === false ? '<span class="tag due">Broken</span>' : '<span class="tag soft">Open</span>'}</span></div>`).join("")}</div>`
    : '<div class="empty">You haven\'t promised anyone anything. Every promise you make lands here, with its date.</div>';
  const due = x.due.length ? `<div class="items">${x.due.map((d) => `<div class="item"><span class="when">${esc(d.date)}</span><span>${rich(d.text)}</span></div>`).join("")}</div>` : '<div class="empty">Nothing dated ahead.</div>';
  const log = x.log.length ? `<div class="items">${x.log.map((e) => `<div class="item"><span class="when">${esc(e.date || "")}</span><span><b>${rich(e.source)}</b> ${rich(e.text)}</span></div>`).join("")}</div>` : '<div class="empty">Nothing said yet.</div>';
  return `<div class="grid2">
    <div class="col"><section class="panel"><h2>Waiting for you <small>${x.waiting.length}</small></h2>${waiting}</section>
      <section class="panel"><h2>Lately <small>what came of it</small></h2>${recent}</section></div>
    <div class="col">
      <section class="panel"><h2>Your boss <small>${esc(x.boss.title)}</small></h2><p class="bigline">${rich(x.boss.link)}</p><p class="muted">${esc(x.boss.words)}</p>${goals}<div class="pp-acts"><button class="st-btn small glass" data-act="ppbossmeet">Ask for a meeting</button><a class="link small" data-act="pptabgo" data-t="Boss">The full file</a></div></section>
      <section class="panel"><h2>Promises <small>the ledger</small></h2>${promises}</section>
      <section class="panel"><h2>Coming up</h2>${due}</section>
      <section class="panel"><h2>What was said <small><a class="link" data-act="pptabgo" data-t="Log">All of it</a></small></h2>${log}</section>
    </div></div>`;
}

async function ppTakeover() {
  const t = await api("people_takeover");
  if (t.none) return '<div class="empty">No takeover on file.</div>';
  const meet = t.meetings.map((s) => ppSceneCard(s)).join("");
  const risks = t.risks.length
    ? table("pp-risks", [{ k: "link", t: "Player", l: true, f: (r) => rich(r.link), v: (r) => plainText(r.link) }, { k: "pos", t: "Pos" }, { k: "why", t: "Why your staff worries", l: true, wrap: true }], t.risks, { tools: false })
    : '<div class="empty">Your staff hasn\'t flagged anyone yet.</div>';
  const claims = t.claims.length
    ? `<div class="items">${t.claims.map((c) => `<div class="item"><span class="when">${esc(c.by)}</span><span>${rich(c.link)} ${t.checked ? `<span class="tag ${c.dir === "oversold" ? "due" : "gold"}">${esc(c.dir)}</span>` : '<span class="tag soft">to check after spring</span>'}</span></div>`).join("")}</div>`
    : '<div class="empty">No claims to check.</div>';
  const staff = `<div class="kv"><dt>Kept</dt><dd>${t.kept.map(rich).join(", ") || "—"}</dd><dt>Let go</dt><dd>${t.letGo.map(rich).join(", ") || "—"}</dd></div>`;
  return `<div class="grid2"><div class="col"><section class="panel"><h2>Your first two weeks <small>since ${esc(t.started || "")}</small></h2><div class="pp-cards">${meet}</div></section></div>
    <div class="col"><section class="panel"><h2>The job</h2><div class="kv">${t.team ? `<dt>Team</dt><dd>${rich(t.team.link)}</dd>` : ""}${t.old ? `<dt>You replaced</dt><dd>${rich(t.old)}</dd>` : ""}<dt>How the hire read</dt><dd>${esc(ppCap(t.fit))}</dd>${t.window ? `<dt>Transfer window</dt><dd>Your players may enter the portal ${esc(t.window.open)} to ${esc(t.window.close)}</dd>` : ""}</div></section>
      <section class="panel"><h2>Who might leave <small>your staff's read</small></h2>${risks}</section>
      <section class="panel"><h2>The old staff</h2>${staff}</section>
      <section class="panel"><h2>Claims to check <small>what the holdovers told you</small></h2>${claims}</section></div></div>`;
}

async function ppRoom() {
  const r = await api("people_room");
  const cols = [
    { k: "link", t: "Player", l: true, f: (x) => `${rich(x.link)}${x.captain ? ' <span class="tag gold" title="Captain">C</span>' : ""}${x.council ? ' <span class="tag key" title="Leadership council">Council</span>' : ""}${x.out ? ' <span class="tag due">Sitting</span>' : ""}`, v: (x) => plainText(x.link) },
    { k: "pos", t: "Pos" },
    { k: "age", t: "Age" },
    { k: "mood", t: "Mood", l: true, f: (x) => `${ppMoodDot(x.mood)} ${esc(x.moodWords)}`, v: (x) => x.mood },
    { k: "why", t: "What your staff hears", l: true, wrap: true },
    { k: "trust", t: "Trust in you", l: true, f: (x) => (x.trust == null ? '<span class="muted">Not yet</span>' : esc(ppTrust(x.trust))), v: (x) => x.trust ?? -1 },
    { k: "friends", t: "Close friends", tip: "Close friends on the roster" },
    { k: "line", t: "Your line to his coach", l: true, f: (x) => esc(x.line ? ppCap(x.line) : "") },
    { k: "talk", t: "", f: (x) => `<button class="st-btn small glass" data-act="pptalkmenu" data-id="${x.id}">Talk</button>`, v: () => 0 },
  ];
  return `<section class="panel"><h2>The room <small>${esc(r.mood.words)} · ${ppHours(r.hours)} left this week</small></h2>${table("pp-room", cols, r.rows, { sort: "mood", empty: "No players." })}</section>`;
}
function ppTrust(t) {
  return t >= 0.8 ? "Would run through a wall for you" : t >= 0.65 ? "Trusts you" : t >= 0.5 ? "Buying in" : t >= 0.38 ? "Waiting to see" : t >= 0.25 ? "Doubts you" : "Has stopped listening";
}

async function ppRules() {
  const r = await api("people_rules");
  if (r.none) return "";
  const levels = [[0, "Off"], [1, "Lenient"], [2, "Firm"], [3, "Strict"]];
  const cards = r.kinds.map((k) => `<div class="pp-rule ${k.strict ? "on" : ""}"><div><b>${esc(k.name)}</b><p class="muted small">${esc(k.about)}</p></div><div class="seg">${levels.map(([v, t]) => `<button data-act="pprule" data-k="${k.key}" data-v="${v}" aria-pressed="${k.strict === v}">${t}</button>`).join("")}</div></div>`).join("");
  const rit = r.ritualsOn ? `<section class="panel"><h2>Rookie rituals</h2><p class="muted small">Carrying pads, the haircut, the rookie dinner. Within limits keeps it a rite of passage; the veterans' call risks a bill or a story.</p><div class="seg">${[[0, "Banned"], [1, "Within limits"], [2, "The veterans' call"]].map(([v, t]) => `<button data-act="pprituals" data-v="${v}" aria-pressed="${r.rituals === v}">${t}</button>`).join("")}</div></section>` : "";
  const council = r.council.length ? `<div class="items">${r.council.map((c) => `<div class="item"><span class="when">Council</span><span>${rich(c.link)} <a class="link small" data-act="ppcouncil" data-id="${c.id}" data-off="1">Remove</a></span></div>`).join("")}</div>` : '<div class="empty">No council. Name players from the Room tab or in a one-on-one; the council can decide cases you hand it.</div>';
  const out = r.out.length ? `<div class="items">${r.out.map((o) => `<div class="item"><span class="when">to ${esc(o.through)}</span><span>${rich(o.link)}: ${esc(o.why)}</span></div>`).join("")}</div>` : '<div class="empty">Nobody is sitting.</div>';
  const rulings = r.rulings.length
    ? table("pp-rulings", [{ k: "date", t: "Date", l: true }, { k: "who", t: "Player", l: true, f: (x) => rich(x.who), v: (x) => plainText(x.who) }, { k: "rule", t: "Rule", l: true }, { k: "response", t: "What you did", l: true }, { k: "star", t: "Starter", f: (x) => (x.star ? "Yes" : "") }], r.rulings, { tools: false })
    : '<div class="empty">No rulings yet. The room watches the first few closely: a star treated softer than a walk-on is noticed.</div>';
  const t = r.trouble;
  const tw = t < 0.92 ? "Your book keeps trouble down" : t > 1.08 ? "A loose book: more trouble than usual" : "About the usual trouble";
  return `<div class="grid2"><div class="col"><section class="panel"><h2>Your rules <small>${esc(tw)}</small></h2><div class="pp-rules">${cards}</div></section></div>
    <div class="col">${rit}<section class="panel"><h2>Leadership council</h2>${council}</section><section class="panel"><h2>Sitting now</h2>${out}</section><section class="panel"><h2>Your rulings <small>for consistency</small></h2>${rulings}</section></div></div>`;
}

async function ppBoss() {
  const b = await api("people_boss");
  if (b.none) return "";
  const goals = b.goals.length
    ? b.goals.map((g) => `<h3 class="pp-h3">${g.season}</h3><div class="items">${g.rows.map((r) => `<div class="item"><span class="when">${r.deal > 0 ? "Raised" : r.deal < 0 ? "Asked down" : "As set"}</span><span>${esc(r.words)} ${r.met === true ? '<span class="tag gold">Met</span>' : r.met === false ? '<span class="tag due">Missed</span>' : r.now ? '<span class="tag soft">On track</span>' : '<span class="tag soft">Not yet</span>'}</span></div>`).join("")}</div>`).join("")
    : '<div class="empty">No goals yet.</div>';
  const asks = b.asks.length ? `<div class="items">${b.asks.map((a) => `<div class="item"><span class="when">${esc(a.date)}</span><span><b>${esc(a.what)}</b> ${a.granted ? '<span class="tag gold">Yes</span>' : '<span class="tag due">No</span>'}<br><span class="muted">${rich(a.words)}</span></span></div>`).join("")}</div>` : '<div class="empty">You haven\'t asked for anything.</div>';
  const meet = b.meetings.length ? `<div class="pp-cards slim">${b.meetings.map((s) => ppSceneCard(s, { noActs: s.done })).join("")}</div>` : '<div class="empty">No meetings yet.</div>';
  const others = b.others.length ? `<div class="items">${b.others.map((o) => `<div class="item"><span class="when">${esc(o.title)}</span><span>${rich(o.link)}${o.trust ? ` · <span class="muted">${esc(o.trust)}</span>` : ""}</span></div>`).join("")}</div>` : "";
  return `<div class="grid2"><div class="col">${ppCastCard(b.boss, `<div class="pp-acts"><button class="st-btn small primary" data-act="ppbossmeet" ${b.canRequest ? "" : 'disabled title="You saw him less than a month ago"'}>Ask for a meeting</button></div>`)}
      <section class="panel"><h2>Goals</h2>${goals}</section></div>
    <div class="col"><section class="panel"><h2>What you've asked for</h2>${asks}</section><section class="panel"><h2>Meetings</h2>${meet}</section>${others ? `<section class="panel"><h2>Above and around him</h2>${others}</section>` : ""}</div></div>`;
}
function ppCastCard(c, extra = "") {
  const rep = (c.reputation || []).map((r) => `<li>${esc(r)}</li>`).join("");
  return `<section class="panel pp-person"><div class="pp-person-top">${face({ name: c.name, colors: null }, 64)}<div><div class="st-label">${esc(c.title)}</div><p class="bigline">${rich(c.link)}</p><p class="muted small">${c.age} · in the job since ${c.since}${c.team ? ` · ${rich(c.team.link)}` : ""}</p></div></div>
    <div class="kv"><dt>Where you stand</dt><dd>${c.trust ? esc(c.trust.words) : "You haven't met"}</dd></div>
    <div><div class="st-label">What people say</div><ul class="pp-rep">${rep}</ul></div>${extra}</section>`;
}

async function ppNetwork() {
  const n = await api("people_contacts");
  const cols = [
    { k: "link", t: "Coach", l: true, f: (x) => rich(x.link), v: (x) => plainText(x.link) },
    { k: "now", t: "Now", l: true, f: (x) => rich(x.now), v: (x) => plainText(x.now) },
    { k: "how", t: "How you know him", l: true, wrap: true },
    { k: "warmth", t: "Tie", l: true, f: (x) => esc(x.warmthWords), v: (x) => x.warmth },
    { k: "last", t: "Last talked", l: true, f: (x) => esc(x.last || "—"), v: (x) => x.last || "" },
    { k: "balance", t: "Favours", tip: "Plus: he owes you. Minus: you owe him.", f: (x) => (x.balance > 0 ? `<span class="delta up">+${x.balance}</span>` : x.balance < 0 ? `<span class="delta down">${x.balance}</span>` : "") },
    { k: "reach", t: "", f: (x) => `<button class="st-btn small glass" data-act="ppreach" data-id="${x.coach}" ${x.recent >= 3 ? 'title="You\'ve called him a lot this month"' : ""}>Reach out</button>`, v: () => 0 },
  ];
  const ledger = n.ledger.length ? `<div class="items">${n.ledger.map((f) => `<div class="item"><span class="when">${esc(f.date)}</span><span>${f.asked ? "You asked" : "He asked"}: ${rich(f.words)} · ${rich(f.who)}</span></div>`).join("")}</div>` : '<div class="empty">No favours either way yet.</div>';
  const interest = n.interest.length ? `<div class="items">${n.interest.map((i) => `<div class="item"><span class="when">${i.year}</span><span>${rich(i.team)}: ${rich(i.words)}</span></div>`).join("")}</div>` : '<div class="empty">Nobody has asked about you. Win, and they will.</div>';
  const road = `<div class="kv"><dt>Clinic talks</dt><dd>${n.clinics}</dd><dt>All-star games</dt><dd>${n.allstar.map((a) => `${a.year} ${esc(a.game)}`).join(", ") || "—"}</dd></div>`;
  return `<div class="grid2"><div class="col"><section class="panel"><h2>Your network <small>${n.rows.length} people · ${ppHours(n.hours)} left this week</small></h2>${table("pp-net", cols, n.rows, { sort: "warmth", empty: "Your network is empty." })}</section></div>
    <div class="col"><section class="panel"><h2>The favour ledger</h2>${ledger}</section><section class="panel"><h2>Who has asked about you</h2>${interest}</section><section class="panel"><h2>On the road</h2>${road}</section></div></div>`;
}

async function ppLeague() {
  const l = await api("people_league");
  const years = l.years.length
    ? table("pp-lyears", [{ k: "year", t: "Year", l: true }, { k: "nflFines", t: "NFL fines" }, { k: "nflSuspensions", t: "Suspensions for hits" }, { k: "targeting", t: "Targeting ejections" }, { k: "hsEjections", t: "High school ejections" }], l.years, { tools: false })
    : '<div class="empty">No season on record yet.</div>';
  const recent = l.recent.length
    ? table("pp-lfines", [{ k: "date", t: "Date", l: true }, { k: "who", t: "Player", l: true, f: (x) => rich(x.who), v: (x) => plainText(x.who) }, { k: "level", t: "Level" }, { k: "what", t: "For", l: true, wrap: true }, { k: "amount", t: "Fine", f: (x) => esc(x.amount || "") }, { k: "games", t: "Games", f: (x) => (x.games ? x.games : "") }, { k: "appealed", t: "Appeal", l: true, f: (x) => (x.appealed ? (x.reduced ? "Reduced" : "Upheld") : "") }], l.recent, {})
    : '<div class="empty">No fines or ejections yet.</div>';
  const cases = l.cases.length
    ? table("pp-lcases", [{ k: "who", t: "Player", l: true, f: (x) => rich(x.who), v: (x) => plainText(x.who) }, { k: "what", t: "Case", l: true, wrap: true }, { k: "opened", t: "Opened", l: true }, { k: "step", t: "Where it stands", l: true }, { k: "outcome", t: "Outcome", l: true, wrap: true, f: (x) => esc(x.outcome || "") }], l.cases, {})
    : '<div class="empty">No cases in the courts.</div>';
  return `<section class="panel"><h2>League discipline <small>fines, ejections and suspensions on the real scale</small></h2>${years}</section><section class="panel"><h2>This season's rulings</h2>${recent}</section><section class="panel"><h2>In the courts and appeals</h2>${cases}</section>`;
}

async function ppLog() {
  const k = keep("pplog", { topic: "" });
  const l = await api("people_log", { topic: k.topic });
  const seg = `<div class="seg wrap"><button data-act="pplogtopic" data-v="" aria-pressed="${!k.topic}">Everything</button>${l.topics.map((t) => `<button data-act="pplogtopic" data-v="${esc(t)}" aria-pressed="${k.topic === t}">${esc(ppCap(t))}</button>`).join("")}</div>`;
  const rows = l.rows.length ? `<div class="items">${l.rows.map((e) => `<div class="item"><span class="when">${esc(e.date || "")}</span><span><b>${rich(e.source)}</b> ${rich(e.text)}${e.angle ? `<br><span class="muted small">${rich(e.angle)}</span>` : ""}${e.check ? `<br><span class="tag key">${esc(e.check)}</span>` : ""}</span></div>`).join("")}</div>` : '<div class="empty">Nothing yet.</div>';
  return `<section class="panel"><h2>Everything said <small>the people log</small></h2>${seg}${rows}</section>`;
}

// ---------- the scene ----------
screens.people_scene = async (a) => {
  const s = await api("people_scene", { id: a.id });
  const lead = (s.who || [])[0];
  const pp = lead && lead.kind === "player" ? await api("people_player", { id: lead.id }).catch(() => null) : null;
  const cast = lead && lead.kind === "cast" && lead.tok ? await api("people_cast", { tok: lead.tok }).catch(() => null) : null;
  title(plainText(s.title), s.kindName);
  const st = keep("ppscene:" + s.id, { pick: null, tone: null });
  if (st.pick != null && !s.picks.some((p) => p.code === st.pick || (s.kind === "OneOnOne" && Math.floor((p.code - 100) / 10) === Math.floor((st.pick - 100) / 10) && p.code >= 100 && p.code < 190))) st.pick = null;
  const colors = (S.status && S.status.mine && S.status.mine.colors) || ["#2b3140", "#11141b"];
  setColors(colors);
  // The conversation.
  const lines = s.lines.map((l) => {
    if (l.by === 0) return `<p class="pp-n">${rich(l.text)}</p>`;
    const cls = l.by === 1 ? "you" : l.by === 2 ? "them" : "other";
    return `<div class="pp-say ${cls}"><b>${esc(l.name)}</b><p>${rich(l.text)}</p>${l.tell ? `<em class="pp-tell">${esc(l.tell)}</em>` : ""}</div>`;
  }).join("");
  const docs = s.docs.map((d) => `<section class="pp-doc"><h3>${esc(d.title)}</h3>${d.note ? `<p class="muted small">${rich(d.note)}</p>` : ""}<div class="tbl warm"><table><thead><tr>${d.cols.map((c, i) => `<th class="${i === 0 || i === d.cols.length - 1 ? "l" : "l"}">${esc(c)}</th>`).join("")}</tr></thead><tbody>${d.rows.map((r) => `<tr>${r.cells.map((c) => `<td class="l wrap">${rich(c)}</td>`).join("")}</tr>`).join("")}</tbody></table></div></section>`).join("");
  const outcome = s.done
    ? `<section class="pp-end"><div class="st-label">${s.handed ? `Handed to ${esc(s.handed)}` : "How it went"}</div>${(s.outcome.length ? s.outcome : ["It's done."]).map((o) => `<p>${rich(o)}</p>`).join("")}<div class="pp-acts">${s.next ? `<button class="st-btn small primary" data-act="ppopen" data-id="${s.next.id}">Next: ${esc(plainText(s.next.title))}</button>` : ""}<button class="st-btn small glass" data-nav="people">Back to People</button></div></section>`
    : "";
  // Your moves.
  let moves = "";
  if (!s.done) {
    const talk = s.kind === "OneOnOne";
    const chosen = st.pick != null ? s.picks.find((p) => p.code === st.pick) || s.picks.find((p) => talk && p.code >= 100 && p.code < 190 && Math.floor((p.code - 100) / 10) === Math.floor((st.pick - 100) / 10)) : null;
    const list = s.picks.map((p) => {
      const on = chosen && chosen.code === p.code;
      return `<button class="pp-move ${on ? "on" : ""} ${p.code === 900 ? "end" : ""}" data-act="pppick" data-id="${s.id}" data-code="${p.code}" data-tone="${esc(p.tone)}"><span class="pp-mv">${esc(p.mv)}${p.code === 900 ? "" : ` <i class="pp-tone ${esc(p.tone)}">${esc(p.tone)}</i>`}</span><span class="pp-lab">${rich(p.label)}</span></button>`;
    }).join("");
    let confirm = "";
    if (chosen) {
      const code = talk && chosen.code >= 100 && chosen.code < 190 ? chosen.code - (chosen.code % 10) + PP_TONE_CODE[st.tone || chosen.tone] : chosen.code;
      let expect = null;
      if (talk && chosen.code !== 900) expect = await api("people_expect", { id: s.id, code }).catch(() => null);
      const tones = talk && chosen.code >= 100 && chosen.code < 190 ? `<div class="pp-tones"><span class="st-label">Tone</span><div class="seg">${PP_TONES.map(([k, t]) => `<button data-act="pptone" data-id="${s.id}" data-v="${k}" aria-pressed="${(st.tone || chosen.tone) === k}">${t}</button>`).join("")}</div></div>` : "";
      const read = expect ? `<div class="pp-read"><div class="st-label">${rich(expect.who)} · ${esc(expect.sure)}</div><p>${esc(expect.what)}</p></div>` : chosen.hint ? `<div class="pp-read"><div class="st-label">What it tends to do</div><p>${esc(chosen.hint)}</p></div>` : "";
      confirm = `<div class="pp-confirm"><p class="pp-quote">${rich(chosen.label)}</p>${tones}${read}<div class="pp-acts"><button class="st-btn small primary" data-act="ppsay" data-id="${s.id}" data-code="${code}">${chosen.code === 900 ? "End it" : "Say it"}</button><button class="st-btn small glass" data-act="ppunpick" data-id="${s.id}">Something else</button></div></div>`;
    }
    moves = `<aside class="pp-moves"><div class="st-label">Your move</div>${confirm || `<div class="pp-movelist">${list || '<div class="empty">Waiting on them.</div>'}</div>`}
      <div class="pp-movefoot"><button class="st-btn small glass" data-act="pphand" data-id="${s.id}" title="Your staff plays it the way they would">Hand it to your staff</button><span class="muted small">${ppHours(s.hours)} left this week</span></div></aside>`;
  }
  // Who's in the room.
  const side = ppSide(s, lead, pp, cast);
  return `<div class="pp-room" style="--c1:${colors[0]};--c2:${colors[1] || colors[0]}">
    <header class="pp-roomhead"><div class="st-label">${esc(s.kindName)} · ${esc(s.dateText)}${s.place ? ` · ${esc(s.place)}` : ""}</div><h1>${rich(s.title)}</h1>${s.team ? `<div class="pp-roomteam">${rich(s.team.link)}</div>` : ""}</header>
    <div class="pp-stage ${s.done ? "done" : ""}">${side}<main class="pp-talk">${lines}${docs}${outcome}</main>${moves}</div></div>`;
};
GOTO.people_scene = () => [["People", "people"], ["Inbox", "inbox"], ["Calendar", "calendar"]];

function ppSide(s, lead, pp, cast) {
  const others = (s.who || []).slice(1);
  const room = others.length ? `<div class="pp-sidesec"><div class="st-label">Also in the room</div>${others.map((o) => `<div class="pp-person-row">${face({ name: plainText(o.link), id: o.kind === "player" ? o.id : undefined }, 30)}<span>${rich(o.link)}${o.pos ? ` <small>${esc(o.pos)}</small>` : o.role ? `<small> ${esc(ppCap(o.role))}</small>` : ""}</span></div>`).join("")}</div>` : "";
  if (!lead) return `<aside class="pp-side">${room || '<div class="muted small">The room is yours.</div>'}</aside>`;
  const name = plainText(lead.link);
  let head = `<div class="pp-sidehead">${face({ name, id: lead.kind === "player" ? lead.id : undefined }, 84)}<div><p class="bigline">${rich(lead.link)}</p><p class="muted small">${esc(lead.pos || ppCap(lead.role || ""))}</p></div></div>`;
  let body = "";
  if (pp) {
    const t = keep("ppside", { t: "Now" });
    const list = ["Now", "File", "With you", "In his ear"];
    if (!list.includes(t.t)) t.t = "Now";
    const tabsHtml = `<div class="seg pp-sidetabs">${list.map((x) => `<button data-act="ppsidetab" data-v="${x}" aria-pressed="${t.t === x}">${x}</button>`).join("")}</div>`;
    let inner = "";
    if (t.t === "Now") {
      inner = `<p class="pp-mood">${ppMoodDot(pp.mood.value)} ${esc(pp.mood.words)}</p>${pp.staffSays ? `<p class="small"><span class="st-label">Your staff thinks</span><br>${esc(pp.staffSays)}</p>` : ""}${pp.reasons.length ? `<div class="items">${pp.reasons.slice(0, 6).map((r) => `<div class="item"><span class="when">${esc(r.date)}</span><span><span class="delta ${r.up ? "up" : "down"}">${r.up ? "▲" : "▼"}</span> ${esc(ppCap(r.why))}</span></div>`).join("")}</div>` : ""}${pp.out ? `<p class="small"><span class="tag due">Sitting</span> ${esc(pp.out)}</p>` : ""}${pp.life ? `<p class="small"><span class="st-label">At home</span><br>${esc(ppCap(pp.life))}</p>` : ""}`;
    } else if (t.t === "File") {
      inner = `<div class="kv">${pp.trust ? `<dt>Trust</dt><dd>${esc(pp.trust.words)}</dd>` : ""}${pp.mentor ? `<dt>Mentor</dt><dd>${rich(pp.mentor)}</dd>` : ""}${pp.case ? `<dt>A case</dt><dd>${esc(pp.case)}</dd>` : ""}${pp.line ? `<dt>Your line</dt><dd>${esc(ppCap(pp.line))}</dd>` : ""}</div><p><a class="link" data-go="player" data-id="${lead.id}">His full page</a></p>`;
    } else if (t.t === "With you") {
      inner = pp.promises.length ? `<div class="items">${pp.promises.map((p) => `<div class="item"><span class="when">${esc(p.due)}</span><span>${esc(ppCap(p.what))} ${p.kept === true ? '<span class="tag gold">Kept</span>' : p.kept === false ? '<span class="tag due">Broken</span>' : '<span class="tag soft">Open</span>'}</span></div>`).join("")}</div>` : '<div class="muted small">No promises between you.</div>';
    } else {
      inner = pp.ear.length ? `<div class="items">${pp.ear.map((e) => `<div class="item"><span class="when">${esc(e.sure)}</span><span>${rich(e.who)}<br><span class="muted small">${esc(e.what)}</span></span></div>`).join("")}</div>` : '<div class="muted small">Nobody your staff knows of.</div>';
    }
    body = tabsHtml + `<div class="pp-sidebody">${inner}</div>`;
  } else if (cast) {
    body = `<div class="kv"><dt>Where you stand</dt><dd>${cast.trust ? esc(cast.trust.words) : "New to you"}</dd></div><div><div class="st-label">What people say</div><ul class="pp-rep">${(cast.reputation || []).map((r) => `<li>${esc(r)}</li>`).join("")}</ul></div>${cast.booster ? `<p class="small muted">${esc(cast.booster.business)}${cast.booster.gift ? `. ${esc(cast.booster.gift)}` : ""}</p>` : ""}${cast.reporter ? `<p class="small muted">${esc(cast.reporter.outlet)} · ${esc(cast.reporter.beat)} · ${esc(cast.reporter.lean)}</p>` : ""}`;
  }
  return `<aside class="pp-side">${head}${body}${room}</aside>`;
}

// ---------- a person around a team ----------
screens.people_cast = async (a) => {
  const c = await api("people_cast", { tok: a.tok });
  title(c.name, c.title);
  const b = c.booster
    ? `<section class="panel"><h2>The booster</h2><div class="kv"><dt>Business</dt><dd>${esc(c.booster.business)}</dd>${c.booster.gift ? `<dt>Has given</dt><dd>${esc(c.booster.gift)}</dd>` : ""}${c.booster.yearly ? `<dt>A year now</dt><dd>${esc(c.booster.yearly)}</dd>` : ""}<dt>Close to</dt><dd>${esc(c.booster.closeTo)}</dd><dt>What he wants</dt><dd>${c.booster.wants.map(esc).join("; ") || "Nothing he's said"}</dd>${c.booster.loyalOld ? "<dt>His loyalty</dt><dd>Close to the coach you replaced</dd>" : ""}</div></section>`
    : "";
  const r = c.reporter
    ? `<section class="panel"><h2>The reporter</h2><div class="kv"><dt>Outlet</dt><dd>${esc(c.reporter.outlet)}${c.reporter.national ? " (national)" : ""}</dd><dt>Beat</dt><dd>${esc(ppCap(c.reporter.beat))}</dd><dt>Lean</dt><dd>${esc(c.reporter.lean)}</dd><dt>His reports</dt><dd>${esc(c.reporter.accuracy)}</dd><dt>Who talks to him</dt><dd>${c.reporter.sources.map(esc).join(", ") || "—"}</dd>${c.reporter.leaks ? "<dt>Careful</dt><dd>What's said in the building reaches him</dd>" : ""}</div></section>`
    : "";
  const scenes = c.scenes.length ? `<div class="pp-cards slim">${c.scenes.map((s) => ppSceneCard(s, { noActs: s.done })).join("")}</div>` : '<div class="empty">You haven\'t met yet.</div>';
  const log = c.log.length ? `<div class="items">${c.log.map((e) => `<div class="item"><span class="when">${esc(e.date || "")}</span><span>${rich(e.text)}</span></div>`).join("")}</div>` : "";
  return `<div class="pad">${pageHead({ kicker: `${esc(c.title)} · ${rich(c.team.link)}`, title: esc(c.name), sub: `${c.age} · ${c.years ? `${c.years} year${c.years === 1 ? "" : "s"} in the job` : "New in the job"}`, goto: gotoFor("people_cast", a, c) })}
    <div class="grid2"><div class="col">${ppCastCard(c)}${b}${r}</div><div class="col"><section class="panel"><h2>Your meetings</h2>${scenes}</section>${log ? `<section class="panel"><h2>What's on file</h2>${log}</section>` : ""}</div></div></div>`;
};
GOTO.people_cast = (a, c) => [["People", "people"], c && c.team ? ["Their team", "team", { kind: c.team.kind, id: c.team.id }] : null];

// ---------- the staff office ----------
screens.people_staff = async (a) => {
  const x = await api("people_staff");
  if (x.none) return `<div class="pad">${pageHead({ title: "Staff office" })}<div class="empty">You need a job first.</div></div>`;
  const list = ["Your staff", "Hiring", "Support staff"];
  const tab = tabOf("pstab", list, a);
  title("Staff office", tab);
  let body = "";
  if (tab === "Your staff") {
    const cols = [
      { k: "link", t: "Coach", l: true, f: (r) => `${rich(r.link)}${r.climber ? ' <span class="tag key" title="Wants a bigger job">Climber</span>' : ""}${r.leave ? ` <span class="tag due" title="${esc(r.leave)}">On leave</span>` : ""}`, v: (r) => plainText(r.link) },
      { k: "role", t: "Role", l: true },
      { k: "age", t: "Age" },
      { k: "cameUp", t: "Came up", l: true },
      { k: "salary", t: "Salary", f: (r) => esc(r.salary || "—") },
      { k: "through", t: "Through" },
      { k: "buyout", t: "Buyout", tip: "The share of the rest of his deal owed if you let him go" },
      { k: "owed", t: "Owed if fired", f: (r) => esc(r.owed || "—") },
      { k: "trust", t: "With you", l: true, f: (r) => esc(r.trust || "New") },
      { k: "family", t: "At home", l: true, wrap: true, hide: true },
      { k: "act", t: "", f: (r) => `<button class="st-btn small glass" data-act="ppraise" data-id="${r.coach}">Raise</button> <button class="st-btn small glass" data-act="ppfire" data-id="${r.coach}" data-name="${esc(plainText(r.link))}">Let go</button>`, v: () => 0 },
    ];
    body = `<section class="panel"><h2>Your assistants <small>${x.rows.length}${x.owed ? ` · still owed to coaches you let go: ${esc(x.owed)}` : ""}</small></h2>${table("ps-staff", cols, x.rows, { empty: x.level === "hs" ? "Your high school staff is on the Support staff tab." : "No assistants." })}</section>`;
  } else if (tab === "Hiring") {
    const roles = [...x.openings.map((o) => [o.key, o.name, true]), ["oc", "Offensive coordinator"], ["dc", "Defensive coordinator"], ["qb", "Quarterbacks coach"], ["rb", "Running backs coach"], ["wr", "Wide receivers coach"], ["ol", "Offensive line coach"], ["dl", "Defensive line coach"], ["lb", "Linebackers coach"], ["db", "Defensive backs coach"], ["st", "Special teams coordinator"]].filter((r, i, arr) => arr.findIndex((y) => y[0] === r[0]) === i);
    const k = keep("pshire", { role: x.openings[0] ? x.openings[0].key : "oc" });
    const seg = `<div class="seg wrap">${roles.map(([v, t, open]) => `<button data-act="pprole" data-v="${v}" aria-pressed="${k.role === v}">${esc(t)}${open ? " <small>open</small>" : ""}</button>`).join("")}</div>`;
    const c = await api("people_candidates", { role: k.role });
    const cols = [
      { k: "link", t: "Coach", l: true, f: (r) => `${rich(r.link)}${r.interviewed ? ' <span class="tag gold">Interviewed</span>' : ""}${r.blocked ? ' <span class="tag due" title="His team can block a lateral move">Can be blocked</span>' : ""}`, v: (r) => plainText(r.link) },
      { k: "age", t: "Age" },
      { k: "now", t: "Now", l: true, f: (r) => rich(r.now), v: (r) => plainText(r.now) },
      { k: "rep", t: "Reputation", l: true },
      { k: "cameUp", t: "Came up", l: true },
      { k: "known", t: "Known to you", l: true },
      { k: "read", t: "Teaching (your read)", f: (r) => range([r.read[0] * 5, r.read[1] * 5].map(Math.round), "your read of his teaching"), v: (r) => r.read[0] + r.read[1] },
      { k: "asks", t: "Asks", v: (r) => r.asksValue },
      { k: "act", t: "", f: (r) => `<button class="st-btn small glass" data-act="ppinterview" data-id="${r.coach}" data-role="${c.role}">Interview</button> <button class="st-btn small primary" data-act="ppoffer" data-id="${r.coach}" data-role="${c.role}" data-asks="${r.asksValue}" data-name="${esc(plainText(r.link))}">Offer</button>`, v: () => 0 },
    ];
    body = `<section class="panel"><h2>${esc(c.roleName)} <small>your network first, then coaches out of work, then assistants a rung down</small></h2>${seg}${table("ps-cand", cols, c.rows, { empty: "Nobody fits this job right now." })}</section>`;
  } else {
    body = `<section class="panel"><h2>Support staff and the rest of the building</h2>${table("ps-support", [{ k: "title", t: "Job", l: true }, { k: "name", t: "Name", l: true }, { k: "age", t: "Age" }, { k: "since", t: "Since" }, { k: "subject", t: "Teaches", l: true, f: (r) => esc(r.subject || "") }, { k: "pay", t: "Pay" }], x.support, { tools: false, empty: "Nobody on file." })}</section>`;
  }
  return `<div class="pad">${pageHead({ kicker: rich(x.team.link), title: "Staff office", sub: "Deals, raises, letting a coach go, and who you could hire.", goto: gotoFor("people_staff", a, x) })}${tabs("pstab", list, tab)}<div class="tabbody">${body}</div></div>`;
};
GOTO.people_staff = () => [["Staff", "staff"], ["People", "people"], ["Network", "people", { tab: "Network" }], ["Your profile", "you"]];
{
  const was = GOTO.staff;
  GOTO.staff = (a, d) => [...(was ? was(a, d) : []), ["Staff office", "people_staff"], ["People", "people"]];
}

// ---------- the player page's People tab ----------
PLAYER_TABS.push({
  name: "People",
  after: "Career",
  draw: async (p) => {
    const x = await api("people_player", { id: p.id });
    const reasons = x.reasons.length ? `<div class="items">${x.reasons.map((r) => `<div class="item"><span class="when">${esc(r.date)}</span><span><span class="delta ${r.up ? "up" : "down"}">${r.up ? "▲" : "▼"}</span> ${esc(ppCap(r.why))}</span></div>`).join("")}</div>` : '<div class="empty">Nothing dated: his mood is his role, the team and his life.</div>';
    const ties = x.ties.length ? `<div class="items">${x.ties.map((t) => `<div class="item"><span class="when">${esc(ppCap(t.words))}</span><span>${rich(t.link)}</span></div>`).join("")}</div>` : '<div class="empty">No close ties on file.</div>';
    const ear = x.ear.length ? `<div class="items">${x.ear.map((e) => `<div class="item"><span class="when">${esc(e.sure)}</span><span>${rich(e.who)}: <span class="muted">${esc(e.what)}</span></span></div>`).join("")}</div>` : '<div class="empty">Nobody your staff knows of.</div>';
    const lines = [["", "No line"], ["level", "Keep him level"], ["push", "Push him"], ["watch", "Tell me if he's slipping"]];
    const cur = x.line === "keep him level" ? "level" : x.line === "push him" ? "push" : x.line === "tell me if he's slipping" ? "watch" : "";
    const mine = x.mine
      ? `<section class="panel"><h2>Talk to him <small>an hour · ${ppHours(x.hours)} left this week</small></h2><div class="pp-subjects">${x.subjects.map((s) => `<button class="st-btn small glass" data-act="pptalk" data-id="${p.id}" data-s="${s.key}">${esc(s.name)}</button>`).join("")}</div>
        <div><div class="st-label">Your line to his position coach</div><div class="seg wrap">${lines.map(([v, t]) => `<button data-act="ppline" data-id="${p.id}" data-v="${v}" aria-pressed="${cur === v}">${esc(t)}</button>`).join("")}</div></div></section>`
      : "";
    const promises = x.promises.length ? `<div class="items">${x.promises.map((q) => `<div class="item"><span class="when">${esc(q.due)}</span><span>${esc(ppCap(q.what))} ${q.kept === true ? '<span class="tag gold">Kept</span>' : q.kept === false ? '<span class="tag due">Broken</span>' : '<span class="tag soft">Open</span>'}</span></div>`).join("")}</div>` : "";
    return `<div class="grid2"><div class="col"><section class="panel"><h2>How he is</h2><p class="pp-mood">${ppMoodDot(x.mood.value)} ${esc(x.mood.words)}${x.trust ? ` · <span class="muted">${esc(x.trust.words)}</span>` : ""}</p>${x.staffSays ? `<p class="small"><span class="st-label">Your staff thinks</span><br>${esc(x.staffSays)}</p>` : ""}${reasons}${x.out ? `<p><span class="tag due">Sitting</span> ${esc(x.out)}</p>` : ""}${x.life ? `<p><span class="st-label">At home</span><br>${esc(ppCap(x.life))}</p>` : ""}${x.case ? `<p><span class="st-label">A case</span><br>${esc(x.case)}</p>` : ""}</section>${mine}</div>
      <div class="col"><section class="panel"><h2>In his ear</h2>${ear}</section><section class="panel"><h2>Close ties</h2>${x.mentor ? `<p class="small">Mentor: ${rich(x.mentor)}</p>` : ""}${ties}</section>${promises ? `<section class="panel"><h2>Promises to him</h2>${promises}</section>` : ""}</div></div>`;
  },
});

// ---------- actions ----------
async function ppGoScene(p) {
  try {
    const r = await p;
    if (r && r.scene != null) go("people_scene", { id: r.scene });
  } catch (e) { toast(e.message); }
}
Object.assign(CORE_MORE, {
  ppopen: (el, ev) => { if (ev && ev.target.closest("[data-act]") !== el) return; go("people_scene", { id: Number(el.dataset.id) }); },
  pphand: async (el, ev) => {
    if (ev) ev.stopPropagation();
    try { await api("people_hand", { id: Number(el.dataset.id) }); toast("Handed to your staff."); if (S.route.screen === "people_scene") refresh(); else refresh(el); } catch (e) { toast(e.message); }
  },
  pppick: (el) => { const st = keep("ppscene:" + el.dataset.id, {}); st.pick = Number(el.dataset.code); st.tone = el.dataset.tone; refresh(el); },
  ppunpick: (el) => { const st = keep("ppscene:" + el.dataset.id, {}); st.pick = null; st.tone = null; refresh(el); },
  pptone: (el) => { const st = keep("ppscene:" + el.dataset.id, {}); st.tone = el.dataset.v; refresh(el); },
  ppsay: async (el) => {
    const id = Number(el.dataset.id);
    el.disabled = true;
    try { await api("people_answer", { id, code: Number(el.dataset.code) }); const st = keep("ppscene:" + id, {}); st.pick = null; st.tone = null; refresh(el); } catch (e) { toast(e.message); el.disabled = false; }
  },
  ppsidetab: (el) => { keep("ppside", { t: "Now" }).t = el.dataset.v; refresh(el); },
  pptabgo: (el) => { keep("pptab", {}).t = el.dataset.t; go("people", { tab: el.dataset.t }, false); },
  ppteamtalk: () => ppGoScene(api("people_team_talk")),
  ppbossmeet: () => ppGoScene(api("people_boss_meeting")),
  pptalk: (el) => ppGoScene(api("people_talk", { id: Number(el.dataset.id), subject: el.dataset.s })),
  pptalkmenu: (el, ev) => {
    if (ev) ev.stopPropagation();
    const subs = [["role", "His role"], ["usage", "His touches"], ["money", "His money"], ["behavior", "His behavior"], ["life", "Life away from football"], ["team", "The team"], ["future", "His future"]];
    menuAt(el, `<div class="st-label">Talk to him about</div>${subs.map(([k, t]) => `<div class="mrow" data-s="${k}">${esc(t)}</div>`).join("")}`, (e) => {
      const r = e.target.closest("[data-s]");
      if (!r) return;
      closeMenu();
      ppGoScene(api("people_talk", { id: Number(el.dataset.id), subject: r.dataset.s }));
    });
  },
  ppline: async (el) => { try { await api("people_line", { id: Number(el.dataset.id), line: el.dataset.v }); refresh(el); } catch (e) { toast(e.message); } },
  pprule: async (el) => { try { await api("people_rules_set", { key: el.dataset.k, strict: Number(el.dataset.v) }); refresh(el); } catch (e) { toast(e.message); } },
  pprituals: async (el) => { try { await api("people_rules_set", { rituals: Number(el.dataset.v) }); refresh(el); } catch (e) { toast(e.message); } },
  ppcouncil: async (el) => {
    try {
      const r = await api("people_rules");
      const ids = r.council.map((c) => c.id).filter((id) => id !== Number(el.dataset.id));
      if (!el.dataset.off) ids.push(Number(el.dataset.id));
      await api("people_council", { ids });
      refresh(el);
    } catch (e) { toast(e.message); }
  },
  pplogtopic: (el) => { keep("pplog", {}).topic = el.dataset.v; refresh(el); },
  ppreach: (el, ev) => {
    if (ev) ev.stopPropagation();
    const how = [["text", "Text him", "a few minutes"], ["call", "Call him", "half an hour"], ["lunch", "Take him to lunch", "two and a half hours, nearby only"]];
    const why = [["catchup", "Just to catch up"], ["name", "Ask for a name for your staff"], ["grapevine", "The grapevine: hot seats and jobs"]];
    menuAt(el, `<div class="st-label">How</div>${how.map(([k, t, n]) => `<label class="mrow"><input type="radio" name="pphow" value="${k}" ${k === "call" ? "checked" : ""}> ${esc(t)} <small class="muted">${esc(n)}</small></label>`).join("")}<div class="st-label">Why</div>${why.map(([k, t]) => `<div class="mrow" data-why="${k}">${esc(t)}</div>`).join("")}`, async (e) => {
      const r = e.target.closest("[data-why]");
      if (!r) return;
      const m = e.target.closest(".popmenu");
      const h = m && m.querySelector("input[name=pphow]:checked");
      closeMenu();
      try { const x = await api("people_reach", { coach: Number(el.dataset.id), how: h ? h.value : "call", why: r.dataset.why }); toast(plainText(x.text)); refresh(el); } catch (err) { toast(err.message); }
    });
  },
  ppraise: (el, ev) => {
    if (ev) ev.stopPropagation();
    menuAt(el, `<div class="st-label">A raise</div>${[[0.05, "5%"], [0.1, "10%"], [0.15, "15%"], [0.25, "25%: keep him from leaving"]].map(([v, t]) => `<div class="mrow" data-v="${v}">${esc(t)}</div>`).join("")}`, async (e) => {
      const r = e.target.closest("[data-v]");
      if (!r) return;
      closeMenu();
      try { const x = await api("people_raise", { coach: Number(el.dataset.id), share: Number(r.dataset.v) }); toast(plainText(x.text)); refresh(el); } catch (err) { toast(err.message); }
    });
  },
  ppfire: async (el) => {
    if (!(await sure(`Let ${el.dataset.name} go?`, "You tell him yourself. What's left of his deal is owed, and the room and his recruits will hear about it.", "Call him in"))) return;
    ppGoScene(api("people_fire", { coach: Number(el.dataset.id) }));
  },
  pprole: (el) => { keep("pshire", {}).role = el.dataset.v; refresh(el); },
  ppinterview: (el) => ppGoScene(api("people_interview", { coach: Number(el.dataset.id), role: el.dataset.role })),
  ppoffer: async (el) => {
    const asks = Number(el.dataset.asks) || 0;
    const v = await ask(`Offer ${el.dataset.name} the job`, { value: String(Math.round(asks / 1000) * 1000), ok: "Make the offer", note: `Salary a year in dollars. He asks about ${money(asks)}; under it he may say no, well over it eats your staff pool. Two years.` });
    if (v == null || v === "") return;
    const salary = Number(String(v).replace(/[^0-9.]/g, ""));
    if (!salary) { toast("That isn't a salary."); return; }
    try { const x = await api("people_offer", { coach: Number(el.dataset.id), role: el.dataset.role, salary, years: 2 }); toast(plainText(x.text)); refresh(el); } catch (e) { toast(e.message); }
  },
});
