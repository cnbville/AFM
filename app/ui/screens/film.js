// Lane screens: the film room and being there (update 3, lane 9). Registers
// screens.film: your week of film, your staff's queue, a cut-up watched
// snap by snap, your next opponent's tape, the days you are there (his
// game from the stands, a visit, your camp, signing day, combine week).
"use strict";
/* The film room in Press Box (SK32-SK39, SK43-SK63), to the agreed mock-up
 * without its tape-quality chips:
 *
 *   home      your week (how hard you work, your ethic, film left, fresh
 *             eyes, wear, home life), today's days out, your staff's queue,
 *             your next opponent's tape, your reports, your film year.
 *   brief     the question to settle, the cut-ups, who sits in.
 *   session   three stills a snap from the engine's tracking (never the 2D
 *             view), the call, the run-back, your mark, the room's talk,
 *             your read narrowing beside your staff's, the grade sheet.
 *   verdict   your answer and the report saved to his file.
 *   there     a visit's hours in drawn places, your camp, the signing room,
 *             combine week.
 * Only reads show: ranges, words and counts.
 */

I.film = '<rect x="3" y="5" width="18" height="14" rx="2"/><path d="M3 9h18M3 15h18M7 5v4M12 5v4M17 5v4M7 15v4M12 15v4M17 15v4"/>';
navAdd({ go: "film", label: "Film room", icon: "film", sec: "scout", after: "evaluate", seats: ["nfl", "college", "hs"] });
CRUMBS.film = "Film room";
GOTO.film = () => (scSeat() === "nfl"
  ? [["Draft board", "draftboard"], ["Evaluation", "evaluate"], ["Game day", "gameday"]]
  : scSeat() === "college"
    ? [["Recruit board", "recruitboard"], ["Evaluation", "evaluate"], ["Game day", "gameday"]]
    : [["Evaluation", "evaluate"], ["Squad", "roster"], ["Game day", "gameday"]]);

const frBtn = (act, data, html, cls = "") => `<button class="st-btn small ${cls || "glass"}" data-act="${act}" ${Object.entries(data).map(([k, v]) => `data-${k}="${esc(String(v))}"`).join(" ")}>${html}</button>`;
const frCard = (label, body, cls = "") => `<section class="fr-card ${cls}"><div class="st-label">${label}</div>${body}</section>`;
const frCell = (label, big, cap) => `<div class="fr-cell"><div class="st-label">${esc(label)}</div><b>${esc(big)}</b>${cap ? `<small>${esc(cap)}</small>` : ""}</div>`;
const frBar = (pct, cls = "") => `<div class="fr-bar"><i class="${cls}" style="width:${Math.max(0, Math.min(100, pct))}%"></i></div>`;
const frH = (h) => (h >= 10 ? `${Math.round(h)} h` : `${(Math.round(h * 10) / 10).toFixed(1)} h`);
const frWho = (id, name) => (id != null ? `<a class="link name" data-go="scout" data-id="${id}">${esc(name)}</a>` : esc(name));
let frAngle = "side";

screens.film = async (a = {}) => {
  if (a.visit) return frVisit();
  if (a.camp) return frCamp();
  if (a.sign) return frSigning();
  if (a.combine) return frCombine();
  if (a.report != null) return frReport(await api("film_report", a.report === "last" ? {} : { k: Number(a.report) }));
  const s = await api("film_session");
  if (s.open && (a.id == null || (s.target.player === Number(a.id)))) return frSession(s);
  if (a.id != null) {
    // A cut-up from your staff's queue opens on the cut they sent.
    const k = keep("frq:" + Number(a.id), {});
    if (a.cut && k.qcut !== a.cut) { k.cut = a.cut; k.qcut = a.cut; }
    return frBrief(await api("film_brief", { id: Number(a.id), ask: k.q }));
  }
  return frHome(await api("film_home"));
};

function frHead(x, sub, extra = "") {
  const pct = x.budget > 0 ? (x.left / x.budget) * 100 : 0;
  return `<header class="fr-head"><div><div class="st-label">Film room · ${esc(x.date || "")}</div><h1 class="fr-h1">${esc(sub.title)}</h1><p class="fr-sub">${esc(sub.line)}</p></div>
    <div class="fr-tops">
      <div class="fr-top"><span class="st-label">Film time left this week</span><b>${frH(x.left)}</b>${frBar(pct)}<small>of ${frH(x.budget)}${x.today ? ` · ${frH(x.today)} today` : ""}</small></div>
      <div class="fr-top"><span class="st-label">Your eye today</span><b>${esc(x.eye_word)}</b>${frBar(x.eye_pct, x.eye_pct < 60 ? "warn" : "")}<small>${x.today >= 3 ? "a long day: things slip by" : "fresh for the first hours"}</small></div>
      ${extra}
    </div></header>`;
}

// ---------- the home ----------
function frHome(x) {
  title("Film room");
  if (!x.seat) return `<div class="pad"><p class="sub">Start a career first.</p></div>`;
  const why = x.prep ? `${frH(x.prep)} game prep, ${frH(x.recruit)} for ${x.seat === "nfl" ? "prospects" : "recruits"} at normal hours` : `${frH(x.recruit)} at normal hours`;
  const works = x.works.map((o) => frBtn("frwork", { level: o.key }, `${esc(o.label)} <small>${o.hours}${o.key === "junkie" ? "+" : ""} h</small>`, o.key === x.work ? "on" : "glass")).join("");
  const ethic = [6, 9, 12, 15, 18].map((v) => frBtn("frethic", { v }, `${v}`, Math.round(x.ethic) === v ? "on" : "glass")).join("");
  const week = frCard("Your week", `
    <div class="fr-row"><span class="fr-q">How hard you work this week</span><div class="fr-seg">${works}</div></div>
    <div class="fr-row"><span class="fr-q">Your work ethic, from your coach profile${x.ethic_set ? "" : " (set it once; it moves with how you really work)"}</span><div class="fr-seg">${ethic}<span class="fr-dim">now ${x.ethic.toFixed(1)}</span></div></div>
    ${x.rested ? `<p class="fr-warn">You're home resting until ${esc(x.rest_until)}: no film.</p>` : ""}
    <div class="fr-cells">
      ${frCell("Film this week", `${frH(x.used)} of ${frH(x.budget)}`, why)}
      ${frCell("Fresh eyes for", frH(x.fresh), "a day, then things slip by")}
      ${frCell("Wear on you", x.wear_word, "health and burnout")}
      ${frCell("Home life", x.home_word, "your family")}
    </div>`);
  const today = [];
  for (const t of x.trips) {
    const act = t.how === "stands" ? frBtn("frstands", { id: t.id }, "Watch from the stands", "primary") : frBtn("frvisit", { id: t.id }, "Go in", "primary");
    today.push(`<div class="fr-item"><div><b>${esc(t.what)}: ${frWho(t.id, t.name)}</b>${t.watchers.length ? `<small>Also there: ${esc(t.watchers.slice(0, 5).join(", "))}${t.watchers.length > 5 ? ` and ${t.watchers.length - 5} more` : ""}</small>` : "<small>Nobody else from a staff or club.</small>"}</div>${act}</div>`);
  }
  if (x.visit_open) today.push(`<div class="fr-item"><div><b>Your visit is under way</b></div><button class="st-btn small primary" data-go="film" data-args='{"visit":1}'>Back to it</button></div>`);
  if (x.camp_day) today.push(`<div class="fr-item"><div><b>Your one-day camp is today</b><small>Coach a group yourself; offer on the spot.</small></div><button class="st-btn small primary" data-go="film" data-args='{"camp":1}'>Run your camp</button></div>`);
  if (x.signing_day) today.push(`<div class="fr-item"><div><b>Signing day</b><small>${x.seat === "hs" ? "Your players sign in your gym." : "The papers come in all day."}</small></div><button class="st-btn small primary" data-go="film" data-args='{"sign":1}'>${x.seat === "hs" ? "The ceremony" : "The signing room"}</button></div>`);
  if (x.combine != null) today.push(`<div class="fr-item"><div><b>Combine week</b><small>${x.combine ? "Today's workout group is on the field." : "Interviews and medicals; the workouts start Thursday."}</small></div><button class="st-btn small primary" data-go="film" data-args='{"combine":1}'>Combine week</button></div>`);
  if (x.team_camp && x.team_camp.length) today.push(`<div class="fr-item"><div><b>Take your team to a college's team camp</b><small>Your boys get seen by that staff.</small></div><div class="fr-seg"><select data-fr="tcamp">${x.team_camp.map((c) => `<option value="${c.program}">${esc(c.name)}</option>`).join("")}</select>${frBtn("frtcamp", {}, "Go", "primary")}</div></div>`);
  if (x.open) today.unshift(`<div class="fr-item"><div><b>Your cut-up of ${esc(x.open.name)}</b><small>Snap ${x.open.at + 1} of ${x.open.n}</small></div><button class="st-btn small primary" data-go="film">Back to the tape</button></div>`);
  const todayCard = today.length ? frCard("Today", `<div class="fr-items">${today.join("")}</div>`) : "";
  const q = x.queue.length ? x.queue.map((qi, n) => {
    const st = qi.state;
    const acts = st === 0
      ? `<div class="fr-seg"><button class="st-btn small primary" data-go="film" data-id="${qi.id}" data-args='{"cut":"${esc(qi.cut || "")}"}'>Open</button>${frBtn("frskip", { k: qi.k }, "Skip it")}${frBtn("frqmove", { k: qi.k, up: 1 }, "Up")}${frBtn("frqmove", { k: qi.k, up: 0 }, "Down")}</div>`
      : `<small class="${st === 2 ? "fr-amber" : "fr-dim"}">${st === 1 ? "Watched." : st === 2 ? `Skipped. ${esc(qi.from.split(" ").pop())} sees that you skipped it.` : "Ran past its date."}</small>`;
    return `<div class="fr-q-row ${st ? "done" : ""}"><div class="fr-q-top"><b>${frWho(qi.id, qi.name)}</b><span class="fr-chips"><span class="st-pill">${esc(qi.cut_words)}</span><span class="st-pill">By ${esc(qi.by)}</span></span></div>
      <small class="fr-dim">${esc(qi.meta)}</small><div class="fr-say">${esc(qi.from)}, ${esc(qi.job)}: “${esc(qi.why)}”</div>${acts}</div>`;
  }).join("") : `<p class="fr-dim">${x.seat === "hs" ? "A high school coach cuts his own tape: open any of your players from the squad or his file." : "Nothing sent this week. Your staff send cut-ups on Mondays."}</p>`;
  const queue = frCard(`Your film queue <span class="fr-r">Sent by your staff</span>`, q);
  const opp = x.opp.length ? frCard("Your next opponent's tape", `<div class="fr-items">${x.opp.map((t) => `<div class="fr-item"><div><b>${esc(t.label)}</b><small>${esc(t.date_text)} · ${esc(t.score)}${t.studied ? " · studied" : ""}</small></div><div class="fr-seg">${frBtn("fropp", { date: t.date, home: t.home, away: t.away, mode: "full" }, "Watch with your coordinator", t.studied ? "glass" : "primary")}${frBtn("fropp", { date: t.date, home: t.home, away: t.away, mode: "quick" }, "Watch it quick")}</div></div>`).join("")}</div><p class="fr-dim">What you catch goes into the game plan as its hints, with their counts. Each tape is a small sample.</p>`) : "";
  const reps = x.reports.length ? `<table class="tbl fr-tbl"><thead><tr><th>Player</th><th>Date</th><th>The question</th><th>Verdict</th><th class="num">Calls</th><th class="num">Caught</th><th></th></tr></thead><tbody>${x.reports.map((r) => `<tr><td>${frWho(r.id, r.name)}</td><td class="fr-dim">${esc(r.date)}</td><td>${esc(r.ask || (r.mode === "quick" ? "Watched quick" : "—"))}</td><td>${r.verdict ? esc(r.verdict) : '<span class="fr-dim">—</span>'}${r.settled != null ? ` <span class="${r.settled ? "fr-gold" : "fr-amber"}">${r.settled ? "held up" : "missed"}</span>` : ""}</td><td class="num">${r.calls[1] ? `${r.calls[0]} of ${r.calls[1]}` : "—"}</td><td class="num">${r.caught}</td><td><a class="link" data-go="film" data-args='{"report":${r.k}}'>${r.stills ? "The look-back" : "Report"}</a></td></tr>`).join("")}</tbody></table>` : `<p class="fr-dim">Each session becomes your report in his file.</p>`;
  const year = frCard("Your film year", `<div class="fr-year"><b>${Math.round(x.year_hours)}</b><small>hours of film this year</small></div>
    <div class="st-label fr-mt">Your eye by position · hours watched</div>
    ${x.by_pos.length ? x.by_pos.slice(0, 8).map((p) => `<div class="fr-pos"><div><span>${esc(p.pos)}</span><span class="fr-dim">${p.hours} h · ${esc(p.word)}</span></div>${frBar(p.pct)}</div>`).join("") : '<p class="fr-dim">Your eye grows fastest at the positions you watch most.</p>'}
    <div class="fr-cells two">${frCell("Finds off the tape", `${x.finds_n}`, x.finds[0] ? `${x.finds[0].name}, off ${x.finds[0].on_name}'s tape` : "kids nobody recruits")}${frCell("Your staff's film work", x.staff_line.split(":")[0], x.staff_line.split(":")[1] || "")}</div>
    ${x.word ? `<p class="fr-note">${esc(x.word)}</p>` : ""}
    <p class="fr-dim">Each a small edge, none decisive. The costs of long hours (health, burnout, family) are real.</p>`);
  const weeks = x.weeks.length ? frCard("Your weeks", `<table class="tbl fr-tbl"><thead><tr><th>Week of</th><th>Work</th><th class="num">Film</th></tr></thead><tbody>${x.weeks.map((w) => `<tr><td>${esc(w.monday)}</td><td>${esc(w.work)}</td><td class="num">${frH(w.film)} of ${frH(w.budget)}</td></tr>`).join("")}</tbody></table>`) : "";
  return `<div class="pad sl fr">
    ${frHead(x, { title: "Film room", line: "All film is cut-ups, on your own hours. What your own eye catches moves your own read." })}
    <div class="fr-cols"><div class="fr-main">${todayCard}${week}${queue}${opp}${frCard("Your reports", reps)}</div><div class="fr-side">${year}${weeks}</div></div>
    <div class="fr-foot">${gotoFor("film", {}, x)}</div></div>`;
}

// ---------- the brief ----------
function frBrief(b) {
  title(b.name, "Film room");
  const head = { date: "", left: b.left, budget: b.budget, today: 0, eye_word: "", eye_pct: 100 };
  if (!b.film) {
    return `<div class="pad sl fr"><header class="fr-head"><div><div class="st-label">Film room</div><h1 class="fr-h1">${esc(b.name)}</h1><p class="fr-sub">${esc(b.meta)}</p></div></header>${frCard("No tape", `<p class="fr-dim">${esc(b.why_not || "There's no film of him.")}</p>`)}</div>`;
  }
  const pickQ = keep("frq:" + b.id, {});
  if (!pickQ.q && b.questions[0]) pickQ.q = b.questions[0].key;
  if (!pickQ.cut || !b.cuts.some((c) => c.key === pickQ.cut)) pickQ.cut = b.cuts[0].key;
  const q = b.questions.find((x) => x.key === pickQ.q) || b.questions[0];
  const qs = b.questions.map((x) => `<button class="st-btn small ${x.key === (q && q.key) ? "on" : "glass"}" data-act="frpickq" data-id="${b.id}" data-q="${esc(x.key)}">${esc(x.text)}</button>`).join("");
  const cuts = b.cuts.map((c) => `<button class="st-btn small ${c.key === pickQ.cut ? "on" : "glass"}" data-act="frpickcut" data-id="${b.id}" data-cut="${c.key}">${esc(c.label)}</button>`).join("");
  const mins = 6 * b.snap_min;
  return `<div class="pad sl fr">
    <header class="fr-head"><div><div class="st-label">Film room · his tape</div><h1 class="fr-h1">${esc(b.name)}</h1><p class="fr-sub">${esc(b.meta)}</p></div>
      <div class="fr-tops"><div class="fr-top"><span class="st-label">Film time left this week</span><b>${frH(b.left)}</b>${frBar(b.budget ? (b.left / b.budget) * 100 : 0)}<small>of ${frH(b.budget)}</small></div></div></header>
    <div class="fr-cols"><div class="fr-main">
      ${frCard("The question to settle", `<div class="fr-big">${esc(q ? q.text : "What does his tape say?")}</div><p class="fr-dim">${esc(q ? q.why : "")}</p><div class="fr-seg wrap">${qs}</div>`)}
      ${frCard(`The cut-up · about ${mins} min`, `<div class="fr-seg wrap">${cuts}</div>
        <p class="fr-dim">Six of his snaps, played on the engine from his real team against real opponents. Each is three stills: the snap, the moment that decides it, the end.</p>
        <div class="fr-seg">${frBtn("fropen", { id: b.id, mode: "full" }, "Watch it snap by snap", "primary")}${frBtn("fropen", { id: b.id, mode: "quick" }, `Watch it quick · ${b.quick_min} min`)}<button class="st-btn small glass" data-go="film">Back to the queue</button></div>`)}
    </div><div class="fr-side">${frReads(b.reads, null)}</div></div></div>`;
}

function frReads(r, speed) {
  if (!r) return "";
  const x = (v) => Math.max(0, Math.min(100, ((v - 20) / 79) * 100));
  const band = (rg, cls) => (rg ? `<i class="${cls}" style="left:${x(rg[0])}%;width:${Math.max(1.5, x(rg[1]) - x(rg[0]))}%"></i>` : "");
  const row = (name, s, y, note) => `<div class="fr-read"><div><b>${esc(name)}</b><span class="fr-dim">Staff ${s ? `${s[0]}-${s[1]}` : "—"} · <span class="fr-gold">You ${y ? (y[0] === y[1] ? `${y[0]}` : `${y[0]}-${y[1]}`) : "not seen"}</span></span></div><div class="fr-track">${band(s, "s")}${band(y, "y")}</div>${note ? `<small class="fr-dim">${esc(note)}</small>` : ""}</div>`;
  const rows = [row("Overall now", r.staff_now, r.you_now)].concat(r.rows.map((a) => row(a.name, a.staff, a.you)));
  return frCard(`Two reads <span class="fr-r"><i class="fr-key s"></i>Your staff <i class="fr-key y"></i>You</span>`, `${rows.join("")}${speed ? `<div class="fr-cell"><div class="st-label">Game speed</div><span>${esc(speed)}</span></div>` : ""}<p class="fr-dim">Your read starts as a first impression. It narrows only with what your own eye catches.</p>`);
}

// ---------- a still ----------
function frProj(x, y) {
  if (frAngle !== "end") return [x, y];
  // From the end zone: behind the offense, the far side narrower.
  const k = 0.55 + 0.45 * (y / 100);
  return [50 + (x - 50) * k, 8 + y * 0.84];
}
function frStill(st, label) {
  if (!st || st.veiled) return `<div class="fr-still"><div class="st-label">${esc(label)}</div><div class="fr-field veiled"><span>Not played yet</span></div><small>&nbsp;</small></div>`;
  const yl = [10, 22, 34, 46, 58, 70, 82, 94].map((v) => { const [, y] = frProj(50, v); return `<line x1="0" y1="${y}" x2="100" y2="${y}" class="yl"/>`; }).join("");
  const [, los] = frProj(50, st.los);
  const lines = st.lines.map((l) => `<polyline class="ln k${l.k}" points="${l.pts.map((p) => frProj(p[0], p[1]).join(",")).join(" ")}"/>`).join("");
  const dots = st.dots.map((d) => { const [x, y] = frProj(d[0], d[1]); return `<g class="dot k${d[2]}"><circle cx="${x}" cy="${y}" r="${d[2] === 2 ? 2.4 : 1.8}"/>${d[3] ? `<text x="${x}" y="${y - 3}">${esc(d[3])}</text>` : ""}</g>`; }).join("");
  const ring = st.ring ? (() => { const [x, y] = frProj(st.ring[0], st.ring[1]); return `<circle class="ring" cx="${x}" cy="${y}" r="4.6"/>`; })() : "";
  return `<div class="fr-still"><div class="st-label">${esc(label)}</div><svg class="fr-field" viewBox="0 0 100 100" preserveAspectRatio="none">${yl}<line x1="0" y1="${los}" x2="100" y2="${los}" class="los"/>${lines}${ring}${dots}</svg><small>${esc(st.caption)}</small></div>`;
}

// ---------- the session ----------
function frSession(s) {
  title(s.name, "Film room");
  const sn = s.snap;
  const pips = s.pips.map((p) => `<i class="pip p${p}"></i>`).join("");
  const stills = sn.stills.map((st, k) => frStill(st, ["The snap", "The moment", "The end"][k])).join("");
  let action = "";
  if (s.step === 0) {
    action = `<div class="fr-seg">${frBtn("frplay", {}, `▶ Play the snap${s.mode === "full" ? ` · ${s.snap_min} min` : ""}`, "primary")}<span class="fr-dim">${s.mode === "live" ? "From the stands: no replays tonight." : "The tape stops before the moment on about one snap in three."}</span></div>`;
  } else if (s.step === 1 && sn.call) {
    action = `<div class="fr-call"><div class="st-label">The tape stops. Call it.</div><div class="fr-big">${esc(sn.call.q)}</div><div class="fr-seg wrap">${sn.call.opts.map((o, k) => frBtn("frcall", { pick: k }, esc(o))).join("")}</div></div>`;
  } else if (s.step === 2) {
    const parts = [];
    if (sn.call_result) parts.push(`<div class="fr-result ${sn.call_result.right ? "right" : "wrong"}">${esc(sn.call_result.text)}</div>`);
    if (sn.surprise) parts.push(`<div class="fr-jump">Jumps out: ${esc(sn.surprise)}</div>`);
    parts.push(`<div class="fr-tile"><div class="st-label">Charted</div>${esc(sn.chart)}</div>`);
    if (sn.caught.length) parts.push(`<div class="fr-tile gold"><div class="st-label">Your eye caught</div>${sn.caught.map((c) => `<div>${esc(c)}</div>`).join("")}</div>`);
    if (sn.after) parts.push(`<p class="fr-cap"><b>After the whistle.</b> ${esc(sn.after)}</p>`);
    if (sn.half) parts.push(`<div class="fr-half"><i>${esc(sn.half)}</i><div class="fr-seg">${frBtn("frrunback", {}, `Run it back · ${s.runback_min} min`)}${frBtn("frmoveon", {}, "Move on")}</div></div>`);
    if (sn.replayed) parts.push(`<div class="fr-tile gold"><div class="st-label">Caught on the replay</div>${esc(sn.replayed)}</div>`);
    if (sn.moved) parts.push(`<p class="fr-dim">You moved on. Whatever it was, you didn't see it.</p>`);
    if (sn.tired && sn.slipped) parts.push(`<p class="fr-amber">Tired eyes: ${frH(s.today)} of film today. Something on this snap may have slipped by.</p>`);
    parts.push(`<div class="st-label">Your mark</div><div class="fr-seg wrap">${s.tags.map((t, k) => frBtn("frmark", { tag: k }, esc(t), sn.mark === k ? "on" : "glass")).join("")}</div>`);
    parts.push(`<div class="fr-seg">${frBtn("frnext", {}, s.at + 1 < s.n ? "Next snap" : "Your verdict", "primary")}</div>`);
    action = parts.join("");
  } else if (s.step === 3) {
    action = `<div class="fr-call"><div class="st-label">Your verdict</div><div class="fr-big">${esc(s.ask ? s.ask.text : "")}</div><div class="fr-seg">${["Yes", "No", "Not sure"].map((t, k) => frBtn("frverdict", { answer: k }, t, k === 0 ? "primary" : "glass")).join("")}</div></div>`;
  }
  const sheet = s.sheet.length ? `<table class="tbl fr-tbl"><thead><tr><th>#</th><th>Game</th><th>Down</th><th>His job</th><th>Result</th><th>You</th>${s.coach ? `<th>${esc(s.coach.name.split(" ").pop())}</th>` : ""}</tr></thead><tbody>${s.sheet.map((r) => `<tr><td>${r.no}</td><td class="fr-dim">${esc(r.game)}</td><td>${esc(r.sit)}</td><td>${esc(r.job)}</td><td>${esc(r.result)}</td><td class="${r.you === "Plus" ? "fr-gold" : r.you === "Minus" ? "fr-amber" : ""}">${esc(r.you || "")}</td>${s.coach ? `<td class="fr-dim">${r.his ? `${r.his} noted` : ""}</td>` : ""}</tr>`).join("")}</tbody></table>` : `<p class="fr-dim">Fills in as you watch.</p>`;
  const room = s.coach || s.analyst ? frCard("In the room", `<p class="fr-dim">${[s.coach ? `${esc(s.coach.name)}, ${esc(s.coach.job.toLowerCase())}` : "", s.analyst ? `${esc(s.analyst)}, your analyst` : ""].filter(Boolean).join(" · ")}</p>${s.talk.length ? s.talk.map((t) => `<div class="fr-talk"><small>Snap ${t.snap}</small> ${esc(t.text)}</div>`).join("") : '<p class="fr-dim">Quiet so far.</p>'}${s.numbers ? `<div class="fr-tile"><div class="st-label">The analyst's numbers</div>${esc(s.numbers)}</div>` : ""}`) : "";
  const hints = s.hints.length ? frCard("Into the game plan", s.hints.map((h) => `<div class="fr-tile ${h.found ? "gold" : ""}"><div class="st-label">${h.found ? "Caught" : "On this tape"} · ${h.snaps} snap${h.snaps === 1 ? "" : "s"}</div>${esc(h.text)}</div>`).join("")) : "";
  const head = { date: s.games[0] || "", left: s.left, budget: s.budget, today: s.today, eye_word: s.eye_word, eye_pct: s.eye_pct };
  const name = s.target.player != null ? frWho(s.target.player, s.name) : esc(s.name);
  return `<div class="pad sl fr">
    ${frHead(head, { title: s.name, line: s.ask ? s.ask.text : "" })}
    <div class="fr-cols"><div class="fr-main">
      <section class="fr-card fr-tape">
        <div class="fr-tape-top"><div><div class="st-label">${esc(s.cut)} · snap ${s.at + 1} of ${s.n}${s.mode === "live" ? " · from the stands" : ""}</div><b class="fr-sit">${name} · ${esc(sn.game)} · ${esc(sn.sit)}</b>${sn.job ? `<small class="fr-dim">His job: ${esc(sn.job)}</small>` : ""}</div>
          <div class="fr-tape-r"><div class="fr-pips">${pips}</div><div class="fr-seg">${frBtn("frangle", { a: "side" }, "Sideline", frAngle === "side" ? "on" : "glass")}${frBtn("frangle", { a: "end" }, "End zone", frAngle === "end" ? "on" : "glass")}</div></div></div>
        <div class="fr-stills">${stills}</div>
        <div class="fr-act">${action}</div>
        <div class="fr-seg end">${frBtn("frclose", {}, "Leave the film room")}<span class="fr-dim">What you watched still counts.</span></div>
      </section>
      ${frCard("Your grade sheet", sheet)}
    </div><div class="fr-side">${frReads(s.reads, s.speed)}${room}${hints}</div></div></div>`;
}

// ---------- the report ----------
function frReport(x) {
  const r = x.report;
  if (!r) return frHome({});
  title(r.name, "Film report");
  const kept = r.snaps.length ? frCard("The look-back · the snaps it rests on", r.snaps.map((sn) => `<div class="fr-kept"><b>${esc(sn.sit)}</b> <span class="fr-dim">${esc(sn.job)} · ${esc(sn.result)}</span><div class="fr-stills small">${sn.stills.map((st, k) => frStill(st, ["The snap", "The moment", "The end"][k])).join("")}</div></div>`).join("")) : "";
  return `<div class="pad sl fr">
    <header class="fr-head"><div><div class="st-label">Film report · ${esc(r.date)}</div><h1 class="fr-h1">${r.id != null ? frWho(r.id, r.name) : esc(r.name)}</h1><p class="fr-sub">${esc(r.ask || "")}</p></div></header>
    ${x.said ? `<p class="fr-note">${esc(x.said)}</p>` : ""}
    <div class="fr-cells">
      ${frCell("On the record", r.verdict || "No verdict", r.settled != null ? (r.settled ? "held up three years on" : "missed, three years on") : "")}
      ${r.coach ? frCell(`${r.coach.split(" ").pop()}'s answer`, r.coach_answer || "—", r.coach) : ""}
      ${frCell("Your calls", r.calls[1] ? `${r.calls[0]} of ${r.calls[1]}` : "None", "with your coach in the room")}
      ${frCell("Saved to his file", `${r.watched} snaps`, `${r.minutes} min, under your name`)}
      ${frCell("Carried forward", r.carried.length ? r.carried.join(", ") : "Nothing", "tags for the interview room and the board")}
      ${r.analyst ? frCell("The analyst's view", r.analyst, r.numbers) : ""}
    </div>
    ${r.caught.length ? frCard("What your eye caught", r.caught.map((c) => `<div class="fr-tile">${esc(c)}</div>`).join("")) : ""}
    ${r.hints.length ? frCard("Into the game plan", `<p class="fr-dim">${r.hints.length} hint${r.hints.length === 1 ? "" : "s"} went into your plan. See game day.</p>`) : ""}
    ${kept}
    <div class="fr-seg"><button class="st-btn small glass" data-go="film">Back to the film room</button>${r.id != null ? `<button class="st-btn small glass" data-go="scout" data-id="${r.id}">His file</button>` : ""}</div></div>`;
}

// ---------- being there ----------
const FR_PLACES = {
  field: { name: "The practice field", svg: '<rect x="0" y="30" width="100" height="70" class="grass"/><path d="M0 45h100M0 60h100M0 75h100M0 90h100" class="chalk"/><rect x="6" y="8" width="18" height="22" class="build"/><path d="M80 30V10M76 10h8" class="post"/><circle cx="40" cy="55" r="2" class="pl"/><circle cx="48" cy="58" r="2" class="pl"/><circle cx="55" cy="52" r="2" class="pl"/><circle cx="62" cy="66" r="2" class="pl me"/>' },
  weights: { name: "The weight room", svg: '<rect x="0" y="70" width="100" height="30" class="floor"/><path d="M14 60h22M14 56v8M36 56v8M60 60h24M60 55v10M84 55v10" class="iron"/><rect x="8" y="20" width="84" height="26" class="mirror"/><circle cx="72" cy="66" r="2.4" class="pl me"/>' },
  training: { name: "The training room", svg: '<rect x="0" y="72" width="100" height="28" class="floor"/><rect x="12" y="58" width="30" height="8" class="table"/><rect x="56" y="58" width="30" height="8" class="table"/><rect x="40" y="18" width="20" height="20" class="cross"/><circle cx="70" cy="54" r="2.4" class="pl"/>' },
  hallway: { name: "The hallway", svg: '<path d="M0 100L35 40h30l35 60z" class="floor"/><rect x="35" y="10" width="30" height="30" class="door"/><rect x="4" y="30" width="10" height="50" class="locker"/><rect x="86" y="30" width="10" height="50" class="locker"/><circle cx="50" cy="70" r="2.4" class="pl"/>' },
  indoor: { name: "The indoor facility", svg: '<path d="M0 30Q50 0 100 30V100H0z" class="roof"/><rect x="0" y="55" width="100" height="45" class="grass"/><path d="M10 70h80M10 85h80" class="chalk"/><circle cx="30" cy="78" r="2" class="pl"/><circle cx="62" cy="76" r="2.4" class="pl me"/><path d="M75 60v12" class="post"/>' },
};
async function frVisit() {
  const x = await api("there_view");
  const v = x.visit;
  if (!v) return `<div class="pad sl fr">${frCard("No visit", '<p class="fr-dim">Book a day at his school from his file (Evaluation, Days out).</p>')}</div>`;
  title(v.name, "Visit");
  const pl = FR_PLACES[v.place] || FR_PLACES.field;
  const opts = v.opts.map((o) => `<button class="st-btn small ${o.done ? "on" : "glass"}" data-act="frchoose" data-key="${o.key}" ${!v.open || o.done || !o.fits ? "disabled" : ""}>${esc(o.label)} <small>${o.minutes} min</small></button>`).join("");
  const notes = v.done.length ? v.done.map((d) => `<div class="fr-tile"><div class="st-label">${esc((FR_PLACES[d.place] || {}).name || d.place)}</div>${esc(d.line)}</div>`).join("") : '<p class="fr-dim">Each choice shows something different, and the time runs out.</p>';
  return `<div class="pad sl fr">
    <header class="fr-head"><div><div class="st-label">${esc(v.what)} · ${esc(v.date)}</div><h1 class="fr-h1">${frWho(v.id, v.name)}</h1><p class="fr-sub">${v.open ? "Four hours at his school. How you spend them is yours." : "The visit is over: your notes are in his file."}</p></div>
      <div class="fr-tops"><div class="fr-top"><span class="st-label">Time left</span><b>${Math.floor(v.left / 60)} h ${v.left % 60 ? `${v.left % 60} min` : ""}</b>${frBar((v.left / 240) * 100)}</div></div></header>
    <div class="fr-cols"><div class="fr-main">
      <section class="fr-card fr-scene"><div class="st-label">${esc(pl.name)}</div><svg viewBox="0 0 100 100" preserveAspectRatio="none" class="fr-place">${pl.svg}</svg></section>
      ${frCard("How you spend the hours", `<div class="fr-seg wrap">${opts}</div>${v.open ? `<div class="fr-seg">${frBtn("frleave", {}, "Leave")}</div>` : ""}`)}
      ${frCard("Your notes", notes)}
    </div><div class="fr-side">${frCard("Who else is here", v.watchers.length ? v.watchers.map((w) => `<div class="fr-tile">${esc(w)}</div>`).join("") : '<p class="fr-dim">Nobody else from a staff or club today.</p>')}</div></div></div>`;
}

async function frCamp() {
  const c = await api("camp");
  title("Your camp");
  if (!c.open) {
    return `<div class="pad sl fr"><header class="fr-head"><div><div class="st-label">Film room · camp</div><h1 class="fr-h1">Your one-day camp</h1><p class="fr-sub">${c.camp_day ? "The campers are checking in." : "Your camps are the second and third Saturdays in June."}</p></div></header>${c.camp_day ? frCard("Open the gates", frBtn("frcampopen", {}, "Start camp", "primary")) : ""}</div>`;
  }
  const groups = c.groups.map((g, k) => frBtn("frcampcoach", { g: k }, esc(g), c.group === k ? "on" : "glass")).join("");
  const campers = `<table class="tbl fr-tbl"><thead><tr><th>Camper</th><th>School</th><th></th></tr></thead><tbody>${c.campers.map((p) => `<tr><td>${frWho(p.id, p.name)} <span class="fr-dim">${esc(p.pos)}</span></td><td class="fr-dim">${esc(p.meta)}</td><td>${p.offered || c.offered.includes(p.id) ? '<span class="fr-gold">Offered</span>' : frBtn("frcampoffer", { id: p.id }, "Offer on the spot")}</td></tr>`).join("")}</tbody></table>`;
  return `<div class="pad sl fr">
    <header class="fr-head"><div><div class="st-label">Film room · camp</div><h1 class="fr-h1">Your one-day camp</h1><p class="fr-sub">${c.campers.length} of your board's recruits came. Coach a group yourself and see them up close.</p></div></header>
    <div class="fr-cols"><div class="fr-main">
      ${frCard("Coach a group yourself", `<div class="fr-seg wrap">${groups}</div>${c.group == null ? '<p class="fr-dim">One group a day: the rest you see from a distance.</p>' : ""}`)}
      ${c.standouts.length ? frCard("Who stood out", c.standouts.map((s) => `<div class="fr-tile gold"><b>${frWho(s.id, s.name)}</b> ${esc(s.line)}</div>`).join("")) : ""}
      ${c.one_on_ones.length ? frCard("One-on-ones", c.one_on_ones.map((o) => `<div class="fr-tile">${esc(o.line)}</div>`).join("")) : ""}
      ${frCard("The campers", campers)}
      <div class="fr-seg">${frBtn("frcampclose", {}, "Close camp")}</div>
    </div></div></div>`;
}

async function frSigning() {
  const x = await api("signing_room");
  title(x.seat_hs ? "Signing ceremony" : "Signing room");
  const rows = x.rows.length ? x.rows.map((r) => `<div class="fr-sign ${r.ours ? "ours" : ""}"><span class="fr-time">${esc(r.time)}</span><div><b>${frWho(r.id, r.name)}</b> <span class="fr-dim">signs with</span> <b>${esc(r.school)}</b>${r.flip ? ' <span class="fr-amber">a flip</span>' : ""}${r.ours ? ' <span class="fr-gold">The papers are in</span>' : ""}<small class="fr-dim">${esc(r.line)}</small></div></div>`).join("") : `<p class="fr-dim">${x.today ? "No one you follow signs today." : "Signing days are the early period in December and the first Wednesday in February."}</p>`;
  return `<div class="pad sl fr"><header class="fr-head"><div><div class="st-label">Film room · signing day</div><h1 class="fr-h1">${x.seat_hs ? "Your gym" : "The signing room"}</h1><p class="fr-sub">${x.seat_hs ? "Your players at the table, the hats in front of them." : "Your staff work the phones; the papers come in through the day."}</p></div></header>${frCard("Through the day", rows)}</div>`;
}

async function frCombine() {
  const x = await api("combine_week");
  title("Combine week");
  if (!x.on) return `<div class="pad sl fr">${frCard("Combine week", '<p class="fr-dim">The combine is in late February and early March.</p>')}</div>`;
  const days = ["Arrivals, medicals and interviews", "Defensive line and linebackers", "Defensive backs and tight ends", "Quarterbacks, receivers and running backs", "Offensive line and specialists"];
  const rows = x.rows.length ? `<table class="tbl fr-tbl"><thead><tr><th>Player</th><th class="num">Public</th><th class="num">Your board</th><th class="num">40</th><th class="num">Vert</th><th class="num">Broad</th><th class="num">Bench</th><th class="num">3-cone</th><th class="num">Shuttle</th></tr></thead><tbody>${x.rows.map((r) => `<tr><td>${frWho(r.id, r.name)} <span class="fr-dim">${esc(r.pos)}</span></td><td class="num">${r.public}</td><td class="num">${r.ours || "—"}</td><td class="num">${r.forty.toFixed(2)}</td><td class="num">${r.vertical}</td><td class="num">${r.broad}</td><td class="num">${r.bench}</td><td class="num">${r.cone.toFixed(2)}</td><td class="num">${r.shuttle.toFixed(2)}</td></tr>`).join("")}</tbody></table>` : '<p class="fr-dim">No workouts on the field today: your interview slots and the medical rechecks fill the day (Evaluation, the interview room).</p>';
  return `<div class="pad sl fr"><header class="fr-head"><div><div class="st-label">Film room · combine week</div><h1 class="fr-h1">${esc(days[x.day] || "Combine week")}</h1><p class="fr-sub">The numbers land group by group; your board moves as they do.</p></div></header>${frCard("On the field today", rows)}<div class="fr-seg"><button class="st-btn small glass" data-go="draftboard">Your draft board</button><button class="st-btn small glass" data-go="evaluate">The interview room</button></div></div>`;
}

// ---------- the moves ----------
async function frDo(el, cmd, args, after) {
  try {
    const r = await api(cmd, args);
    if (r && r.said) toast(r.said);
    if (after) after(r); else refresh(el);
  } catch (e) {
    toast(e.message || String(e));
  }
}
Object.assign(CORE_MORE, {
  frwork: (el) => frDo(el, "film_work", { level: el.dataset.level }),
  frethic: (el) => frDo(el, "film_ethic", { value: Number(el.dataset.v) }),
  frskip: (el) => frDo(el, "film_queue_skip", { k: Number(el.dataset.k) }),
  frqmove: (el) => frDo(el, "film_queue_move", { k: Number(el.dataset.k), up: el.dataset.up === "1" }),
  frpickq: (el) => { keep("frq:" + el.dataset.id, {}).q = el.dataset.q; refresh(el); },
  frpickcut: (el) => { keep("frq:" + el.dataset.id, {}).cut = el.dataset.cut; refresh(el); },
  fropen: (el) => {
    const id = Number(el.dataset.id);
    const k = keep("frq:" + id, {});
    frDo(el, "film_open", { id, mode: el.dataset.mode, cut: k.cut, ask: k.q }, () => go("film", el.dataset.mode === "quick" ? { report: "last" } : { id }));
  },
  fropp: (el) => frDo(el, "film_opp", { date: Number(el.dataset.date), home: Number(el.dataset.home), away: Number(el.dataset.away), mode: el.dataset.mode }, (r) => go("film", r.open ? {} : { report: "last" })),
  frplay: (el) => frDo(el, "film_snap", {}),
  frcall: (el) => frDo(el, "film_call", { pick: Number(el.dataset.pick) }),
  frrunback: (el) => frDo(el, "film_runback", {}),
  frmoveon: (el) => frDo(el, "film_moveon", {}),
  frmark: (el) => frDo(el, "film_mark", { tag: Number(el.dataset.tag) }),
  frnext: (el) => frDo(el, "film_next", {}),
  frverdict: (el) => frDo(el, "film_verdict", { answer: Number(el.dataset.answer) }, () => go("film", { report: "last" }, false)),
  frclose: (el) => frDo(el, "film_leave", {}, (r) => go("film", r && r.saved ? { report: "last" } : {}, false)),
  frangle: (el) => { frAngle = el.dataset.a; refresh(el); },
  frstands: (el) => frDo(el, "there_stands", { id: Number(el.dataset.id) }, () => go("film", {})),
  frvisit: (el) => frDo(el, "there_visit", { id: Number(el.dataset.id) }, () => go("film", { visit: 1 })),
  frchoose: (el) => frDo(el, "there_choose", { key: el.dataset.key }),
  frleave: (el) => frDo(el, "there_leave", {}),
  frcampopen: (el) => frDo(el, "camp_open", {}),
  frcampcoach: (el) => frDo(el, "camp_coach", { group: Number(el.dataset.g) }),
  frcampoffer: (el) => frDo(el, "camp_offer", { id: Number(el.dataset.id) }),
  frcampclose: (el) => frDo(el, "camp_close", {}, () => go("film", {})),
  frtcamp: (el) => {
    const s = (el.closest(".fr-item") || document).querySelector('[data-fr="tcamp"]');
    frDo(el, "team_camp", { program: Number(s && s.value) });
  },
});
