// Lane screens: the scouting fresh look (update 2, lane 8). Registers
// screens.evaluate and the scouting file's "Your look" tab.
"use strict";
/* Your evaluation desk in Press Box (SK7-SK22):
 *
 *   every seat    your days out and who saw you there (SK12, SK13), your
 *                 eye's record (SK14), board meetings and who was right
 *                 (SK16), your hard minimums (SK17).
 *   an NFL club   the May review of your scouts and the scout market (SK7-
 *                 SK11), your service (SK18), the analytics staff's record
 *                 (SK6).
 *   a college     your personnel staff and its market (SK7), the NFL on
 *                 your campus and your word (SK19), your pro day (SK20),
 *                 the advisory list and your stay-or-go talks (SK21).
 *   a high school the athletes in your building (SK22).
 *
 * The file's tab: what his tape holds (SK1-SK5), the days you could go,
 * the interview rooms (SK15), your service's grade, your word.
 * Only your staff's reads show: ranges, words and counts.
 */

I.eye = '<path d="M2 12s3.6-7 10-7 10 7 10 7-3.6 7-10 7S2 12 2 12z"/><circle cx="12" cy="12" r="3"/>';
navAdd({ go: "evaluate", label: "Evaluation", icon: "eye", sec: "scout", after: "scouting", seats: ["nfl", "college", "hs"] });
CRUMBS.evaluate = "Evaluation";
GOTO.evaluate = () => (scSeat() === "nfl"
  ? [["Department", "scouting"], ["Draft board", "draftboard"], ["Pre-draft", "predraft"], ["Staff", "staff"]]
  : scSeat() === "college"
    ? [["Department", "scouting"], ["Recruit board", "recruitboard"], ["Recruiting", "recruiting"], ["Squad", "roster"]]
    : [["College calls", "hsside"], ["Squad", "roster"], ["Recruiting", "recruiting"]]);

const evLk = (act, data, html, cls = "") => `<button class="ev-btn ${cls}" data-act="${act}" ${Object.entries(data).map(([k, v]) => `data-${k}="${esc(String(v))}"`).join(" ")}>${html}</button>`;
const evCard = (label, body, cls = "") => `<section class="ev-card ${cls}"><div class="st-label">${esc(label)}</div>${body}</section>`;
const evEmpty = (t) => `<p class="ev-empty">${esc(t)}</p>`;
const evFileLink = (p) => (p ? `<a class="link name" data-go="scout" data-id="${p.id}">${esc(p.name)}</a>` : "");
const evRec = (r) => {
  if (!r || !r.n) return `<span class="ev-dim">No look-backs yet</span>`;
  const miss = r.missed != null ? [`${r.missed} missed`] : [r.high ? `${r.high} too high` : "", r.low ? `${r.low} too low` : ""];
  return `<span class="ev-rec"><b>${r.held}</b> of ${r.n} held${miss.filter(Boolean).map((m) => ` · ${m}`).join("")}</span>`;
};
// A staff range as a soft band on the 40-99 scale (or another).
const evBand = (lo, hi, cls = "", min = 40, max = 99) => {
  const x = (v) => Math.max(0, Math.min(100, ((v - min) / (max - min)) * 100));
  return `<span class="ev-band ${cls}" title="${lo}-${hi}"><i style="left:${x(lo)}%;width:${Math.max(2, x(hi) - x(lo))}%"></i><b>${lo}-${hi}</b></span>`;
};

screens.evaluate = async (a) => {
  const x = await api("eval_home");
  if (!x.seat) return `<div class="pad"><p class="sub">Start a career first.</p></div>`;
  const list = x.seat === "nfl"
    ? ["Scouts", "Days out", "Meetings", "Minimums", "Service"]
    : x.seat === "college"
      ? ["Personnel", "Days out", "NFL on campus", "Pro day", "Advisory", "Meetings", "Minimums"]
      : ["Athletes", "Days out"];
  const tab = tabOf("evtab", list, a);
  title("Evaluation", String(x.year));
  const body = {
    Scouts: evScouts, "Days out": evTrips, Meetings: evMeetings, Minimums: evMinimums, Service: evService,
    Personnel: evPersonnel, "NFL on campus": evCampus, "Pro day": evProDay, Advisory: evAdvisory, Athletes: evAthletes,
  }[tab](x);
  const sub = x.seat === "nfl" ? "Your scouts, your own days out, the room and the rules you hold to." : x.seat === "college" ? "Your personnel staff, your days out, the NFL on your campus and your juniors' calls." : "Your days out and the athletes in your building who don't play yet.";
  return `<div class="pad sl ev ev-${x.seat}">
    <header class="ev-head"><div><div class="st-label">Scouting · ${esc(x.today)}</div><h1 class="ev-h1">Evaluation</h1><p class="ev-sub">${esc(sub)}</p></div>${evTops(x)}</header>
    <div class="ev-tabs">${tabs("evtab", list, tab)}</div>
    ${body}
    <div class="ev-foot">${gotoFor("evaluate", a, x)}</div>
  </div>`;
};

function evTops(x) {
  const cells = [
    `<div class="ev-top"><span class="st-label">Your eye</span><b>${x.eye.credited}</b><small>${esc(x.eye.words)}</small></div>`,
    `<div class="ev-top"><span class="st-label">Days out</span><b>${x.trips.length}</b><small>${x.trips.filter((t) => t.seen).length} seen by the press</small></div>`,
  ];
  if (x.nfl && x.nfl.analytics.has) cells.push(`<div class="ev-top"><span class="st-label">Analytics</span><b>${x.nfl.analytics.record.held}</b><small>${esc(x.nfl.analytics.record.words)}</small></div>`);
  if (x.college && x.college.pro_day) cells.push(`<div class="ev-top"><span class="st-label">Pro day ${x.college.pro_day.year}</span><b>${x.college.pro_day.clubs}</b><small>clubs, ${x.college.pro_day.heads} with a head man</small></div>`);
  return `<div class="ev-tops">${cells.join("")}</div>`;
}

// ---------- NFL: the May review and the market (SK7-SK11) ----------
function evScoutRow(s, extra) {
  return `<tr><td class="ev-name"><b>${esc(s.name)}</b><small>${esc(s.job)}${s.area ? ` · ${esc(s.area)}` : ""}</small></td>
    <td class="num">${s.age}</td><td>${s.contacts ? esc(s.contacts) : '<span class="ev-dim">—</span>'}</td>
    <td>${evRec(s.record)}${s.before ? `<small class="ev-dim ev-line">${esc(s.before)}</small>` : ""}</td><td class="num">${esc(s.pay)}${s.underpaid ? ' <span class="ev-amber" title="The market pays him more">under market</span>' : ""}</td>
    <td class="num ev-dim">${esc(s.market)}</td>${extra || ""}</tr>`;
}
function evScouts(x) {
  const n = x.nfl;
  const head = `<thead><tr><th>Scout</th><th class="num">Age</th><th>Contacts</th><th>Look-backs</th><th class="num">Pay</th><th class="num">Market</th>`;
  let review = "";
  if (n.review) {
    const rows = n.review.scouts.map((s) => evScoutRow(s, `<td><div class="ev-seg">${n.review.calls.map((c) => evLk("evcall", { place: s.place, call: c.key }, esc(c.words), s.call === c.key ? "on" : "")).join("")}</div></td>`)).join("");
    const hires = n.review.hires.length ? `<div class="ev-sub2">Signed for June 1</div><table class="tbl ev-tbl">${head}<th>Post</th></tr></thead><tbody>${n.review.hires.map((s) => evScoutRow(s, `<td>${esc(s.post)}</td>`)).join("")}</tbody></table>` : "";
    review = evCard(`The May review · contracts end ${n.review.due}`, `<p class="ev-dim">Every call you leave undecided is a renewal. A sharp man you don't raise may take another club's offer.</p>
      <table class="tbl ev-tbl">${head}<th>Your call</th></tr></thead><tbody>${rows}</tbody></table>${hires}
      <div class="ev-actions">${evLk("evclose", {}, "Close the review now", "pri")}</div>`, "wide");
  } else {
    review = evCard("Your scouts", `<p class="ev-dim">The May review opens after the draft, when scouting contracts end.</p><table class="tbl ev-tbl">${head}</tr></thead><tbody>${n.staff.map((s) => evScoutRow(s)).join("")}</tbody></table>`, "wide");
  }
  const market = n.market.length
    ? `<table class="tbl ev-tbl">${head}<th>Clubs</th><th></th></tr></thead><tbody>${n.market.map((s) => evScoutRow(s, `<td class="ev-dim">${(s.clubs || []).slice(-2).map((t) => teamLink(t)).join(", ") || "—"}</td><td>${n.review ? evLk("evsign", { k: s.k }, "Sign") : ""}</td>`)).join("")}</tbody></table>`
    : evEmpty("Nobody is out of work just now.");
  return `${review}${evCard(`The scout market · ${n.market.length} out of work`, market, "wide")}`;
}

function evService(x) {
  const s = x.nfl.service;
  const a = x.nfl.analytics;
  const opts = s.options.map((o) => evLk("evservice", { service: o.key }, esc(o.words), (s.next ? s.next === o.words : s.words === o.words) ? "on" : "")).join("");
  return `<div class="ev-grid">
    ${evCard("Your scouting service", `<h2 class="ev-h2">${esc(s.words)}</h2><p class="ev-dim">About two-thirds of clubs belong to National or BLESTO: ${esc(s.fee)} and a scout lent for part of the fall. The service grades every rising senior in the spring (1.0 to 9.0; 7.0 and up is about round one) and your fall calls start from its list. Outside the services your scouts travel more on the money saved.</p>
      <div class="ev-seg">${opts}</div>${s.next ? `<p class="ev-note">From after the draft: ${esc(s.next)}.</p>` : ""}`)}
    ${evCard("Your analytics staff", a.has ? `<h2 class="ev-h2">${esc(a.name)}</h2><p class="ev-dim">Reads his numbers (production, age, testing, against his level), not his tape. Its own order of each class is kept for its look-back.</p><div class="ev-big">${evRec(a.record)}</div>` : evEmpty("No analytics department reads college numbers yet (clubs from 2003)."))}
  </div>`;
}

// ---------- every seat: days out, meetings, minimums ----------
function evTrips(x) {
  if (!x.trips.length) return evCard("Your days out", evEmpty("No trips yet. Open a player's scouting file and pick a day under \"Your look\"."), "wide");
  const rows = x.trips.map((t) => `<tr><td class="ev-date">${esc(t.date)}</td><td>${evFileLink(t.player)} <small class="ev-dim">${esc(t.player.pos)} · ${esc(t.player.class)}</small></td>
    <td>${esc(t.kind)}</td><td>${t.sent ? `Sent ${esc(t.sent)}` : "You"}</td>
    <td>${t.done ? (t.seen ? '<span class="ev-amber">Seen: the press reported it</span>' : '<span class="ev-dim">Not noticed</span>') : '<span class="ev-gold">Planned</span>'}</td></tr>`).join("");
  return evCard("Your days out", `<p class="ev-dim">A head coach or GM turning up is news, so the interest leaks; a man you send isn't.</p><table class="tbl ev-tbl"><thead><tr><th>Date</th><th>Player</th><th>Trip</th><th>Who went</th><th>Seen</th></tr></thead><tbody>${rows}</tbody></table>`, "wide");
}

function evMeetings(x) {
  // Reads on the file's own scale: prospects sit on 40-99, recruits lower.
  const dom = x.seat === "nfl" ? [40, 99] : [1, 99];
  const rec = x.meeting_record.length
    ? `<table class="tbl ev-tbl"><thead><tr><th>Who</th><th class="num">Right</th><th class="num">Wrong</th></tr></thead><tbody>${x.meeting_record.map((r) => `<tr><td>${esc(r.name)}</td><td class="num">${r.right}</td><td class="num">${r.wrong}</td></tr>`).join("")}</tbody></table>`
    : evEmpty("Each meeting's calls are judged once the class has its look-back.");
  const ms = x.meetings.length
    ? x.meetings.map((m) => evCard(`${m.kind} · ${m.date}`, m.items.length ? `<table class="tbl ev-tbl ev-meet"><thead><tr><th>Player</th><th>Presented by</th><th>High man</th><th>Low man</th><th>Your call</th></tr></thead><tbody>${m.items.map((it) => `<tr>
        <td><a class="link name" data-go="scout" data-id="${it.id}">${esc(it.name)}</a> <small class="ev-dim">${esc(it.pos)}</small></td>
        <td>${esc(it.by)}${it.manner ? ` <small class="ev-dim">${esc(it.manner)}</small>` : ""}</td>
        <td><small>${esc(it.high.name)}</small>${evBand(it.high.read[0], it.high.read[1], "hi", ...dom)}</td>
        <td><small>${esc(it.low.name)}</small>${evBand(it.low.read[0], it.low.read[1], "lo", ...dom)}</td>
        <td><div class="ev-seg">${[[1, "High"], [3, "Between"], [2, "Low"]].map(([v, t]) => evLk("evsettle", { meeting: m.index, id: it.id, side: v }, t, it.settled === v ? "on" : "")).join("")}</div></td></tr>`).join("")}</tbody></table>` : evEmpty("Your room agreed on everyone."), "wide")).join("")
    : evCard("Board meetings", evEmpty(x.seat === "nfl" ? "The room meets after the all-star games in February and for April's final stack." : "Your staff meets June 1, August 1 and December 1."), "wide");
  return `${ms}${evCard("Who was right in the room", rec)}`;
}

function evMinimums(x) {
  const rows = x.minimums.map((m) => `<tr><td>${esc(m.pos)}</td><td>${esc(m.words)}</td><td class="num">${m.value}</td><td>${m.drop ? "Off the board" : "Flagged"}</td><td>${evLk("evminoff", { pos: m.pos, measure: m.measure }, "Remove")}</td></tr>`).join("");
  const form = `<div class="ev-form">
    <select data-ev="pos" aria-label="Position">${x.positions.map((p) => `<option>${esc(p)}</option>`).join("")}</select>
    <select data-ev="measure" aria-label="Measure">${x.measures.map((m) => `<option value="${m.key}">${esc(m.words)}</option>`).join("")}</select>
    <input data-ev="value" type="number" step="0.01" placeholder="Line" aria-label="Line">
    <label class="ev-check"><input data-ev="drop" type="checkbox"> Take him off the board</label>
    ${evLk("evmin", {}, "Set", "pri")}</div>`;
  return evCard("Your hard minimums", `<p class="ev-dim">Lines a player must clear at a spot (Seattle's 32-inch arms at corner). Under a line he is flagged on your board, or taken off it. Many GMs and head coaches hold one or two of their own.</p>${rows ? `<table class="tbl ev-tbl"><thead><tr><th>Spot</th><th>Measure</th><th class="num">Line</th><th>Under it</th><th></th></tr></thead><tbody>${rows}</tbody></table>` : evEmpty("No minimums set.")}${form}`, "wide");
}

// ---------- college ----------
function evPersonnel(x) {
  const c = x.college;
  const staff = c.personnel.length
    ? `<table class="tbl ev-tbl"><thead><tr><th>Name</th><th>Job</th><th class="num">Since</th><th></th></tr></thead><tbody>${c.personnel.map((p) => `<tr><td><b>${esc(p.name)}</b></td><td>${esc(p.job)}</td><td class="num">${p.since}</td><td>${evLk("evletgo", { post: p.post }, "Let go")}</td></tr>`).join("")}</tbody></table>`
    : evEmpty("No personnel staff yet.");
  const market = c.personnel_market.length
    ? `<table class="tbl ev-tbl"><thead><tr><th>Name</th><th>Job</th><th class="num">Age</th><th>Programs</th><th>Look-backs</th><th></th></tr></thead><tbody>${c.personnel_market.map((h) => `<tr><td><b>${esc(h.name)}</b></td><td>${esc(h.job)}</td><td class="num">${h.age}</td><td class="ev-dim">${h.past.map((t) => teamLink(t)).join(", ") || "—"}</td><td>${evRec(h.record)}</td><td>${evLk("evhire", { key: h.key }, "Hire")}</td></tr>`).join("")}</tbody></table>`
    : evEmpty("Nobody is looking this year.");
  return `${evCard("Your personnel staff", `<p class="ev-dim">They evaluate film and work on campus; only the on-field coaches go out.</p>${staff}`, "wide")}${evCard("Personnel people looking for work", market, "wide")}`;
}

function evCampus(x) {
  const c = x.college;
  const open = [[0, "Closed"], [1, "Open practice"], [2, "Practice and the film room"]].map(([v, t]) => evLk("evopen", { level: v }, t, c.openness === v ? "on" : "")).join("");
  const rows = c.campus.map((p) => `<tr><td>${evFileLink(p)} <small class="ev-dim">${esc(p.pos)} · ${esc(p.class)}</small></td><td class="num">${p.clubs}</td><td class="num">${p.heads}</td>
    <td>${p.words.length ? p.words.map((w) => `${esc(w.club)}: ${["worth a look", "can play", "a sure thing"][w.strength]}${w.held === true ? " ✓" : w.held === false ? ' <span class="ev-amber">✗</span>' : ""}`).join("; ") : '<span class="ev-dim">—</span>'}</td></tr>`).join("");
  const clubs = c.clubs.slice().sort((a, b) => b.cred - a.cred).map((k) => `<tr><td>${teamLink({ name: k.name, ref: { kind: "nfl", id: k.club } })}</td><td><span class="ev-meter"><i style="width:${k.cred}%"></i></span></td><td class="${k.cred < 25 ? "ev-amber" : ""}">${esc(k.words)}</td></tr>`).join("");
  return `<div class="ev-grid">
    ${evCard("NFL scouts on your campus this year", `<div class="ev-seg">${open}</div><p class="ev-dim">Closed practice hides more from them; the film room shows more. Their traffic shows your players' stock.</p>${rows ? `<table class="tbl ev-tbl"><thead><tr><th>Player</th><th class="num">Clubs</th><th class="num">Head men</th><th>Your word</th></tr></thead><tbody>${rows}</tbody></table>` : evEmpty("No NFL club has been through this fall yet.")}`, "wide")}
    ${evCard("Each club's credit for your word", `<p class="ev-dim">Your word on a player is settled after his draft: it builds your credit with that club or burns it.</p><table class="tbl ev-tbl"><tbody>${clubs}</tbody></table>`)}
  </div>`;
}

function evProDay(x) {
  const d = x.college.pro_day;
  if (!d) return evCard("Your pro day", evEmpty("Your pro day is held in March or April once your seniors and early entrants are declared."), "wide");
  return evCard(`Your pro day · ${d.year}`, `<div class="ev-stats"><div><b>${d.clubs}</b><small>clubs sent a scout</small></div><div><b>${d.heads}</b><small>sent a head coach, coordinator or GM</small></div><div><b>${d.tested}</b><small>players worked out</small></div></div>
    ${d.head_clubs.length ? `<p><span class="st-label">Head men from</span> ${d.head_clubs.map((t) => teamLink(t)).join(", ")}</p>` : ""}
    ${d.players.length ? `<table class="tbl ev-tbl"><thead><tr><th>Worked out</th><th>Spot</th><th>Now</th></tr></thead><tbody>${d.players.map((p) => `<tr><td>${evFileLink(p)}</td><td>${esc(p.pos)}</td><td class="ev-dim">${esc(p.class)}</td></tr>`).join("")}</tbody></table>` : ""}
    <p class="ev-dim">A pro day the league's head men come to sells the path to the pros in recruiting.</p>`, "wide");
}

function evAdvisory(x) {
  const a = x.college.advisory;
  const rule = !a.committee ? "The committee doesn't exist yet (it began in 1994): your juniors go on their own and their agents' word."
    : a.limit ? `Five names a school since 2014 (${a.sent} sent), more only by exception for a likely first-rounder. Three answers: first round, second round or return to school.`
      : "Any underclassman can ask; the committee answers with a round.";
  const say = [[0, "Stay a year"], [1, "Your call"], [2, "You're ready"]];
  const rows = a.juniors.map((j) => `<tr><td>${evFileLink(j)} <small class="ev-dim">${esc(j.pos)} · ${esc(j.class)}</small></td><td>${esc(j.nfl_word)}</td>
    <td>${a.committee ? evLk("evadv", { id: j.id, on: j.sent ? 0 : 1 }, j.sent ? "Sent" : "Send", j.sent ? "on" : "") : ""}</td><td>${j.grade ? esc(j.grade) : '<span class="ev-dim">—</span>'}</td>
    <td><div class="ev-seg">${say.map(([v, t]) => evLk("evtalk", { id: j.id, say: v }, t, j.talk === v ? "on" : "")).join("")}</div></td></tr>`).join("");
  return evCard("The NFL's advisory grades and your talks", `<p class="ev-dim">${esc(rule)} Requests close December 10.</p>${rows ? `<table class="tbl ev-tbl"><thead><tr><th>Player</th><th>The NFL's word</th><th>Committee</th><th>Grade</th><th>Your talk</th></tr></thead><tbody>${rows}</tbody></table>` : evEmpty("None of your underclassmen has an NFL read this year.")}`, "wide");
}

// ---------- high school ----------
function evAthletes(x) {
  const h = x.hs;
  const rows = h.athletes.map((a) => `<tr><td><b>${esc(a.name)}</b></td><td>${esc(a.sport)}</td><td class="num">${a.class_of}</td><td>${esc(a.pos)}</td><td>${evBand(a.tools[0], a.tools[1], "", 1, 20)}</td><td class="ev-dim">${esc(a.spotted_by)}</td><td>${evLk("evath", { key: a.key }, "Talk to him", "pri")}</td></tr>`).join("");
  const talks = h.talks.length ? `<table class="tbl ev-tbl"><tbody>${h.talks.map((t) => `<tr><td class="ev-date">${esc(t.date)}</td><td>${t.player ? evFileLink(t.player) : `<b>${esc(t.name || "—")}</b>`}</td><td class="ev-dim">${esc(t.sport || "")}</td><td>${t.yes ? '<span class="ev-gold">Came out</span>' : '<span class="ev-dim">Stayed with his sport</span>'}</td></tr>`).join("")}</tbody></table>` : "";
  return `${evCard("Athletes in your building who don't play football", `<p class="ev-dim">Your staff's read of the athlete he is, on the 1-20 tools scale. A talk asks him to come out; seniors rarely do.</p>${rows ? `<table class="tbl ev-tbl"><thead><tr><th>Name</th><th>Sport</th><th class="num">Class</th><th>Spot</th><th>Tools</th><th>Spotted by</th><th></th></tr></thead><tbody>${rows}</tbody></table>` : evEmpty("Nobody new caught your staff's eye this year.")}`, "wide")}${talks ? evCard("Your talks", talks, "wide") : ""}`;
}

// ---------- the scouting file's "Your look" tab ----------
async function evFileTab(id) {
  let f;
  try { f = await api("eval_file", { id }); } catch (e) { return evEmpty(e.message || String(e)); }
  const tape = f.tape.length ? `<ul class="ev-list">${f.tape.map((t) => `<li>${esc(t)}</li>`).join("")}</ul>` : evEmpty("Nothing out of the ordinary on his tape, or no tape yet.");
  const send = f.send.length ? `<select data-ev="send" aria-label="Who goes"><option value="">You go</option>${f.send.map((s) => `<option value="${s.code}">${esc(s.name)} · ${esc(s.job)}</option>`).join("")}</select>` : "";
  const trips = f.trips.length ? `<div class="ev-trips">${f.trips.map((t) => `<div class="ev-trip ${t.date ? "" : "off"}"><b>${esc(t.words)}</b>${t.date ? `<small>${esc(t.date)}</small>${evLk("evtrip", { id: f.id, kind: t.kind }, "Go", "pri")}` : `<small>${esc(t.why)}</small>`}</div>`).join("")}</div>${send && f.trips.some((t) => t.date) ? `<div class="ev-form">${send}</div>` : ""}` : evEmpty("No trips for your seat.");
  const planned = f.planned.length ? `<table class="tbl ev-tbl"><tbody>${f.planned.map((t) => `<tr><td class="ev-date">${esc(t.date)}</td><td>${esc(t.kind)}</td><td>${t.sent ? `Sent ${esc(t.sent)}` : "You"}</td><td>${t.done ? (t.seen ? '<span class="ev-amber">Seen</span>' : "Done") : '<span class="ev-gold">Planned</span>'}</td></tr>`).join("")}</tbody></table>` : "";
  const rooms = f.rooms.map((r) => `<div class="ev-room" data-room="${esc(r.room)}"><div class="ev-ph"><b>${esc(r.room)}</b><small>${r.minutes} minutes</small></div>
    <div class="ev-topics">${f.topics.map((t) => `<label><span>${esc(t.words)}</span><input type="number" min="0" max="${r.minutes}" step="1" value="0" data-topic="${t.key}"></label>`).join("")}</div>
    ${evLk("evinterview", { id: f.id, room: r.room }, "Run the interview", "pri")}</div>`).join("");
  const done = f.interviews.map((iv) => `<div class="ev-iv"><div class="ev-ph"><b>${esc(iv.room)}</b><small>${esc(iv.date)} · ${iv.minutes} minutes</small></div>${iv.answers.map((a) => `<p><span class="st-label">${esc(a.topic)}</span> ${esc(a.line)}</p>`).join("")}${iv.quiz ? `<p class="ev-note">${esc(iv.quiz)}</p>` : ""}</div>`).join("");
  const word = f.word ? evCard("Your word to NFL clubs", `<div class="ev-form"><select data-ev="club" aria-label="Club">${f.word.clubs.map((c) => `<option value="${c.club}" ${c.given != null ? "disabled" : ""}>${esc(c.name)} · ${esc(c.words)}${c.given != null ? " (told)" : ""}</option>`).join("")}</select>
    <select data-ev="strength" aria-label="What you tell them">${f.word.strengths.map((s, k) => `<option value="${k}">${esc(s)}</option>`).join("")}</select>${evLk("evword", { id: f.id }, "Tell them", "pri")}</div><p class="ev-dim">Settled after his draft: a sure thing holds if he goes in two rounds.</p>`) : "";
  const svc = f.service ? `<p><span class="st-label">${esc(f.service.name)}</span> <b class="ev-grade">${f.service.grade.toFixed(1)}</b></p>` : "";
  return `<div class="sl ev ev-file">
    ${f.under ? `<p class="ev-amber ev-flag">${esc(f.under)}</p>` : ""}
    <div class="ev-grid">
      ${evCard("What his tape holds", `${tape}${svc}`)}
      ${evCard("Go yourself, or send someone", `${trips}${planned}`)}
      ${rooms || done ? evCard("The interview room", `${rooms}${done}`, "wide") : ""}
      ${word}
    </div></div>`;
}

// ---------- the moves ----------
async function evDo(el, cmd, args) {
  try {
    const r = await api(cmd, args);
    if (r && r.said) toast(r.said);
    refresh(el);
  } catch (e) {
    toast(e.message || String(e));
  }
}
const evVal = (el, k) => {
  const box = el.closest(".ev-card, .ev-room, .ev-form") || document;
  const f = box.querySelector(`[data-ev="${k}"]`) || (el.closest(".ev-card") || document).querySelector(`[data-ev="${k}"]`);
  return f ? (f.type === "checkbox" ? f.checked : f.value) : "";
};
Object.assign(CORE_MORE, {
  evcall: (el) => evDo(el, "eval_review_call", { place: Number(el.dataset.place), call: el.dataset.call }),
  evclose: (el) => evDo(el, "eval_review_close", {}),
  evsign: (el) => evDo(el, "eval_sign", { k: Number(el.dataset.k) }),
  evhire: (el) => evDo(el, "eval_hire", { key: Number(el.dataset.key) }),
  evletgo: (el) => evDo(el, "eval_let_go", { post: Number(el.dataset.post) }),
  evtrip: (el) => {
    const s = evVal(el, "send");
    evDo(el, "eval_trip", { id: Number(el.dataset.id), kind: el.dataset.kind, ...(s ? { send: Number(s) } : {}) });
  },
  evinterview: (el) => {
    const room = el.closest(".ev-room");
    const spent = {};
    room.querySelectorAll("[data-topic]").forEach((i) => { spent[i.dataset.topic] = Number(i.value || 0); });
    evDo(el, "eval_interview", { id: Number(el.dataset.id), room: el.dataset.room, spent });
  },
  evsettle: (el) => evDo(el, "eval_settle", { meeting: Number(el.dataset.meeting), id: Number(el.dataset.id), side: Number(el.dataset.side) }),
  evmin: (el) => {
    const v = Number(evVal(el, "value"));
    if (!v) { toast("Set a line first."); return; }
    evDo(el, "eval_minimum", { pos: evVal(el, "pos"), measure: evVal(el, "measure"), value: v, drop: !!evVal(el, "drop") });
  },
  evminoff: (el) => evDo(el, "eval_minimum", { pos: el.dataset.pos, measure: el.dataset.measure }),
  evservice: (el) => evDo(el, "eval_service", { service: el.dataset.service }),
  evword: (el) => evDo(el, "eval_word", { id: Number(el.dataset.id), club: Number(evVal(el, "club")), strength: Number(evVal(el, "strength")) }),
  evopen: (el) => evDo(el, "eval_openness", { level: Number(el.dataset.level) }),
  evadv: (el) => evDo(el, "eval_advisory", { id: Number(el.dataset.id), on: el.dataset.on === "1" }),
  evtalk: (el) => evDo(el, "eval_talk", { id: Number(el.dataset.id), say: Number(el.dataset.say) }),
  evath: (el) => evDo(el, "eval_athlete", { key: Number(el.dataset.key) }),
});
