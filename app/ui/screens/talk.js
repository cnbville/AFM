// Game day talks (next update, lane 14: dialogue D4-D6). The pregame room
// before kickoff and the halftime room at the break: your pick (move, tone,
// subject), your line roughly, your staff's read with the risk, how each
// unit and the key players will take it, a captain to hand it to; then what
// happened. No faces (Cain's rule for group talks). And the sideline card
// in the live game: a word with a player after a costly mistake. Classes
// are tk-; the page carries .sl for lane 1's Press Box tokens.
"use strict";

const TK = { pick: null, moment: null };

screens.talk = async () => {
  let x;
  try {
    x = await api("talk_room", TK.pick || {});
  } catch (e) {
    return `<div class="pad sl tk"><div class="h1">The locker room</div><p class="sub">${esc(e.message)}</p><div><button class="st-btn glass" data-go="gameday">Game day</button></div></div>`;
  }
  if (x.status === "played") {
    TK.pick = null;
    return `<div class="pad sl tk"><div class="h1">The team is on the field</div><p class="sub">${x.pregame ? `Your talk before kickoff: ${esc(x.pregame.move_name)}, ${esc(x.pregame.tone_name.toLowerCase())}.` : ""} The halftime talk opens at the break.</p><div><button class="st-btn primary" data-go="live">Back to the game</button></div></div>`;
  }
  if (TK.moment !== x.moment) TK.pick = null;
  TK.moment = x.moment;
  const said = x.status === "said";
  const top = tkTop(x);
  const body = said ? (x.pick.move === "captain" ? tkCaptainSaid(x) : tkSaid(x)) : tkChoose(x);
  return `<div class="sl tk"><div class="tk-room"></div><div class="pad tk-pad">${top}${body}</div></div>`;
};
CRUMBS.talk = "Team talk";
GOTO.talk = () => [["Game day", "gameday"], ["Depth chart", "depth"], ["Squad", "roster"]];

// ---------- the parts ----------
function tkTop(x) {
  const half = x.moment === "halftime";
  const at = x.home ? "vs" : "at";
  const rec = x.records || ["", ""];
  const c = x.clock;
  const clock = half
    ? `<div class="tk-clock"><b>${x.score[0]}-${x.score[1]}</b><span>The break is ${x.break} minutes. Your coordinators work the board while you talk.</span></div>`
    : c
      ? `<div class="tk-clock"><b>${esc(c.in)}</b><div class="tk-line"><i style="left:0"></i><i style="left:66%"></i><i style="left:100%"></i></div><span><small>In ${esc(c.in)}</small><small>Out ${esc(c.out)}</small><small>Kick ${esc(c.kick)}</small></span></div>`
      : "";
  const tag = [x.level, x.name, ...(x.stakes ? x.stakes.why.slice(0, 1) : [])].filter(Boolean).map(esc).join(" · ");
  return `<div class="tk-top"><span class="tk-pill">${half ? "Halftime talk" : "Pregame talk"}</span>
    <span class="tk-teams">${teamLink(x.me)} <small>${esc(rec[0])}</small> <em>${at}</em> ${teamLink(x.opp)} <small>${esc(rec[1])}</small></span>
    <span class="tk-tag">${tag}</span>${x.date ? `<span class="tk-tag">${esc(x.date)}</span>` : ""}${clock}</div>`;
}

const tkBar = (now, tick, worry) =>
  `<span class="tk-bar ${worry ? "worry" : ""}"><i style="width:${(100 * now).toFixed(0)}%"></i><b style="left:${(100 * tick).toFixed(0)}%"></b></span>`;

function tkUnits(units, title, note, afterNow) {
  const rows = units.map((u) => {
    const [bar, tick] = afterNow ? [u.after, u.before] : [u.after, u.before];
    return `<tr><td>${esc(u.unit)}</td><td class="${u.worry ? "tk-warn" : ""}">${esc(u.words)}</td><td>${tkBar(bar.poise, tick.poise, u.worry && bar.poise < tick.poise)}</td><td>${tkBar(bar.focus, tick.focus, bar.focus < tick.focus - 0.04)}</td></tr>`;
  }).join("");
  return `<section class="tk-panel"><div class="tk-ph"><b>${title}</b><span>${note}</span></div>
    <p class="tk-note">Likely, not certain: your staff's read, from each player's temper, composure, ego and years.</p>
    <table class="tk-units"><thead><tr><th>Unit</th><th>Likely response</th><th>Composure</th><th>Focus</th></tr></thead><tbody>${rows}</tbody></table></section>`;
}

function tkKey(key, title) {
  const rows = key.map((p) => `<li><div class="tk-kp"><div>${playerLink(p.id, p.name)} <small>${esc(p.pos)}${p.class ? " · " + esc(p.class) : ""}${p.captain ? " · captain" : ""}</small><div class="tk-dim">${esc(p.tag)}</div></div>
    <div><b class="${p.worry ? "tk-warn" : ""}">${esc(p.why)}</b><div class="tk-dim">${esc(p.detail)}</div></div></div></li>`).join("");
  return `<section class="tk-panel"><div class="tk-ph"><b>${title}</b><span>why each one</span></div><ul class="tk-list">${rows || '<li class="tk-dim">Nobody moves much.</li>'}</ul></section>`;
}

function tkStaff(x) {
  const st = (x.staff || []).map((s) => `<p class="tk-q">${coachLink(s.coach, s.name)}, ${esc(s.role)}: “${esc(s.text)}”</p>`).join("");
  const r = x.risk;
  return `<section class="tk-sub"><div class="tk-ph"><b>Your staff's read</b><span>${(x.staff || []).length} coordinator${(x.staff || []).length === 1 ? "" : "s"}</span></div>${st}
    ${r ? `<div class="tk-risk ${r.level}"><b>Risk · ${esc(r.level)}</b><span>${esc(r.text)}</span></div>` : ""}</section>`;
}

function tkChips(x) {
  const p = x.pick;
  const chip = (k, v, name, on, extra = "") => `<button class="tk-chip ${on ? "on" : ""}" data-act="tk-pick" data-k="${k}" data-v="${esc(v)}" ${extra}>${esc(name)}</button>`;
  const o = x.options;
  const moves = o.moves.map((m) => chip("move", m.key, m.name, p.move === m.key)).join("");
  const tones = o.tones.map((t) => chip("tone", t.key, t.name + (t.worn >= 0.6 ? " · worn" : ""), p.tone === t.key, t.worn >= 0.6 ? 'title="You have used this tone a lot lately: it reaches them less"' : "")).join("");
  const subs = p.move === "captain" ? "" : o.subjects.map((s) => chip("subject", s.key, s.name, p.subject === s.key)).join("");
  const one = p.subject === "one" && p.move !== "captain"
    ? `<select class="tk-sel" data-act="tk-who">${(x.ones || []).map((o) => `<option value="${o.id}" ${o.id === p.who ? "selected" : ""}>${esc(o.name)} · ${esc(o.note)}</option>`).join("")}</select>` : "";
  const caps = p.move === "captain" ? `<select class="tk-sel" data-act="tk-who">${(x.captains.voices || []).filter((v) => !v.you).map((v) => `<option value="${v.id}" ${v.id === p.who ? "selected" : ""}>${esc(v.name)} · ${esc(v.pos)}</option>`).join("")}</select>` : "";
  return `<div class="tk-picks"><span>Move</span><div>${moves}</div><span>Tone</span><div>${tones}</div>${subs ? `<span>Subject</span><div>${subs}${one}</div>` : `<span>Captain</span><div>${caps}</div>`}</div>`;
}

function tkChoose(x) {
  const half = x.moment === "halftime";
  const head = half
    ? `<div class="tk-head"><div class="tk-label">Team talk · the whole room · ${x.margin > 0 ? `up ${x.margin}` : x.margin < 0 ? `down ${-x.margin}` : "level"} at the half</div><h1>${x.margin < 0 ? "Thirty minutes to fix it." : x.margin > 0 ? "Thirty minutes to finish it." : "It's a new game."}</h1></div>`
    : `<div class="tk-head"><div class="tk-label">Team talk · the whole room · ${x.dressed} dressed</div><h1>${esc(x.stakes.head)}</h1><p>${esc((x.stakes.why || []).join(" "))}</p></div>`;
  const left = half ? tkHalfLeft(x) : tkLeft(x);
  const others = (x.others || []).map((o) => `<tr><td>${esc(o.tone)}</td><td>${esc(o.text)}</td></tr>`).join("");
  const cap = (x.captains.voices || []).find((v) => !v.you);
  const mid = `<section class="tk-panel tk-mid"><div class="tk-ph"><b>Your talk</b><span>one of each</span></div>${tkChips(x)}
    <div class="tk-sub"><div class="tk-ph"><b>Your line, roughly</b><span>from your picks, in your voice</span></div>
      <p class="tk-line-q">${x.pick.move === "nothing" ? "You say nothing. They get dressed and go." : `“${esc(x.line)}”`}</p></div>
    ${tkStaff(x)}
    ${others ? `<div class="tk-sub"><div class="tk-ph"><b>Other tones, roughly</b><span>same move and subject</span></div><table class="tk-others"><tbody>${others}</tbody></table></div>` : ""}
    <div class="tk-actions"><button class="st-btn primary" data-act="tk-say">Say it</button>
      ${cap && x.pick.move !== "captain" ? `<button class="st-btn glass" data-act="tk-captain" data-id="${cap.id}">Hand it to ${esc(cap.name.split(" ").slice(-1)[0])}</button>` : ""}
      <button class="st-btn glass" data-act="tk-nothing">Say nothing</button>
      <span class="tk-dim">${half ? `${x.break} minute break` : x.clock ? `About ${x.clock.minutes} minutes in the room` : ""}</span></div></section>`;
  const right = `<div class="tk-col">${tkUnits(x.units, "How the room will take it", "bar: after your line · tick: now")}${tkKey(x.key, "Key players")}</div>`;
  return `${head}<div class="tk-grid">${left}${mid}${right}</div>`;
}

function tkLeft(x) {
  const q = x.quote;
  const rival = q
    ? `<section class="tk-sub"><div class="tk-ph"><b>Their man in the paper</b><span>${teamLink(x.opp, "Scouting report")}</span></div><blockquote class="tk-quote">“${esc(q.text)}”</blockquote>
      <p class="tk-dim">${playerLink(q.id, q.name)}, ${esc(q.pos)}${q.class ? ", " + esc(q.class) : ""}, about our ${esc(q.unit.toLowerCase())}. Pinned up in your weight room.</p></section>` : "";
  const wk = x.week;
  const worn = (wk.worn || []).map((t) => `${esc(t.tone)} ${t.worn >= 0.6 ? "worn" : "used"}`).join(", ");
  const hurt = (x.hurt || []).map((p) => `<tr><td>${playerLink(p.id, p.name)} <small>${esc(p.pos)}</small></td><td class="tk-dim">${p.room ? "Out · the room plays for him" : "Out"}</td></tr>`).join("");
  return `<div class="tk-col"><section class="tk-panel">
      <div class="tk-sub"><div class="tk-ph"><b>The stakes</b><span>${x.stakes.level >= 0.6 ? "big" : x.stakes.level >= 0.4 ? "more than most" : "a regular week"}</span></div>
        <ul class="tk-why">${(x.stakes.why || []).map((w) => `<li>${esc(w)}</li>`).join("") || "<li>No more than any other week.</li>"}</ul></div>
      ${rival}
      <div class="tk-sub"><div class="tk-ph"><b>This week</b></div><div class="tk-kv"><span>Last week</span><span>${esc(wk.last)}</span><span>Practice</span><span>${esc(wk.practice)}</span>
        <span>Your talks</span><span>${worn ? `Lately: ${worn}.` : "Fresh: no tone worn."}${wk.handed ? ` Handed to a captain ${wk.handed} of the last 4.` : ""}</span></div></div>
      <div class="tk-sub"><div class="tk-ph"><b>Who is hurt</b><span>starters</span></div>${hurt ? `<table class="tk-hurt"><tbody>${hurt}</tbody></table>` : '<p class="tk-dim">Every starter dresses.</p>'}</div>
    </section></div>`;
}

function tkHalfLeft(x) {
  const wk = x.week;
  return `<div class="tk-col"><section class="tk-panel"><div class="tk-sub"><div class="tk-ph"><b>The half</b></div>
    <p>${x.margin > 0 ? `You lead by ${x.margin}.` : x.margin < 0 ? `You trail by ${-x.margin}.` : "Level at the half."} Your pregame talk has mostly worn off by now: what you say here replaces what is left of it.</p></div>
    <div class="tk-sub"><div class="tk-ph"><b>Your talks</b></div><p class="tk-dim">${(wk.worn || []).length ? `Lately: ${wk.worn.map((t) => esc(t.tone)).join(", ")}.` : "No tone worn."}</p></div>
    <div class="tk-sub"><div class="tk-ph"><b>Your coordinators</b></div><p class="tk-dim">Their adjustments are on the halftime board, back in the game. They work it while you talk.</p></div></section></div>`;
}

// What the first quarter looks like from the room (from the read, not a bonus).
function tkFirst(x) {
  const u = Object.fromEntries((x.units || []).map((v) => [v.unit, v]));
  const d = (n, k) => (u[n] ? u[n].after[k] - u[n].before[k] : 0);
  const front = d("Defensive front", "fire") + d("Linebackers", "fire");
  const line = d("Offensive line", "focus") + d("Defensive front", "focus");
  const late = /late hit/.test(x.risk ? x.risk.text : "");
  const pois = (x.units || []).reduce((a, v) => a + v.after.poise - v.before.poise, 0) / Math.max(1, (x.units || []).length);
  const row = (k, b, t, warn) => `<div class="tk-fq"><span>${k}</span><b class="${warn ? "tk-warn" : ""}">${b}</b><p>${t}</p></div>`;
  return `<section class="tk-panel"><div class="tk-ph"><b>The first quarter</b><span>from the players, not a bonus</span></div>
    ${row("Early downs", front > 0.06 ? "Edge on defense." : front < -0.04 ? "Flat up front." : "As they came in.", front > 0.06 ? "The front is fired up: faster get-offs on first and second down." : "Nothing much changes on the early downs.")}
    ${row("Pre-snap penalties", line < -0.04 ? "Up a little." : line > 0.04 ? "Down a little." : "About as usual.", line < -0.04 ? "The lines are loose: a long count could pull someone early." : "The lines know the count.", line < -0.04)}
    ${row("Late hits", late ? "A bit more likely." : "As usual.", late ? "Someone wants a piece of somebody." : "Nobody is looking for a fight.", late)}
    ${row("Mistakes", pois < -0.03 ? "Bite harder." : pois > 0.03 ? "Bite less." : "As usual.", pois < -0.03 ? "Composure is down: a turnover could snowball." : "Composure holds.", pois < -0.03)}
    ${row("How long it lasts", "Most of the first quarter.", "Young players settle after the first hits; the veterans hold it longer.")}</section>`;
}

function tkSaid(x) {
  const p = x.pick;
  const half = x.moment === "halftime";
  const head = `<div class="tk-head"><div class="tk-chips">${p.move === "nothing" ? '<span class="tk-chip on">Say nothing</span>' : `<span class="tk-chip on">${esc(p.move_name)}</span><span class="tk-chip on">${esc(p.tone_name)}</span><span class="tk-chip on">${esc(p.subject_name)}</span>`}<span class="tk-label">Said · the whole room</span></div>
    <h1 class="tk-said">${p.move === "nothing" ? "You said nothing. They taped up and went." : `“${esc(x.line)}”`}</h1></div>`;
  const moved = (x.units || []).filter((u) => u.words !== "Unmoved.");
  const what = `<section class="tk-panel"><div class="tk-ph"><b>What happened</b><span>in the room</span></div>
    <p>${moved.length ? moved.slice(0, 3).map((u) => `${esc(u.unit)}: ${esc(u.words.toLowerCase())}`).join(" ") : "The room barely moved. They were where they were going to be."}</p></section>`;
  const right = `<div class="tk-col">${tkFirst(x)}<section class="tk-panel">${tkStaff(x)}
    <div class="tk-actions">${half ? `<button class="st-btn primary" data-go="live">Back out for the second half</button>` : `<button class="st-btn primary" data-act="tk-run">Run out</button><button class="st-btn glass" data-act="tk-undo">Change it</button>`}
    <span class="tk-dim">${x.clock ? `Out at ${esc(x.clock.out)}` : ""}</span></div></section></div>`;
  return `${head}<div class="tk-grid"><div class="tk-col">${what}${tkUnits(x.units, "By unit", "bar: now · tick: before your line", true)}</div><div class="tk-col">${tkKey(x.key, "Who stood out")}</div>${right}</div>`;
}

function tkCaptainSaid(x) {
  const c = x.captains;
  const cap = (c.voices || []).find((v) => v.id === x.pick.who) || (c.voices || []).find((v) => !v.you);
  const voices = (c.voices || []).map((v) => `<li><div class="tk-kp"><div>${v.you ? "<b>You</b>" : playerLink(v.id, v.name)} <small>${v.you ? "head coach" : `${esc(v.pos)} · ${v.years} yr${v.years === 1 ? "" : "s"}`}</small></div>
    <div><span class="tk-dim">${esc(v.reach_words)}</span>${tkBar(v.reach, v.reach, v.you && v.reach < 0.55)}</div></div></li>`).join("");
  const head = `<div class="tk-head"><div class="tk-chips"><span class="tk-chip on">Hand it to a captain</span><span class="tk-chip on">${esc(x.pick.tone_name)}</span>${cap ? `<span class="tk-chip on">${esc(cap.name)}</span>` : ""}<span class="tk-label">Your part · 20 seconds</span></div>
    <h1 class="tk-said">“${esc(x.line)}”</h1></div>`;
  const left = `<div class="tk-col"><section class="tk-panel"><div class="tk-ph"><b>Who the room listens to</b><span>how far each voice carries</span></div><ul class="tk-list">${voices}</ul>
    <p class="tk-note">How far a voice carries comes from leadership, years, how he plays and which side of the ball he is on.</p></section>
    ${cap ? `<section class="tk-panel"><div class="tk-ph"><b>Why ${esc(cap.name.split(" ").slice(-1)[0])}</b></div><div class="tk-kv"><span>Leadership</span><span>${esc(cap.leadership)}</span><span>Years</span><span>${cap.years}</span></div></section>` : ""}
    <section class="tk-panel"><div class="tk-ph"><b>How often you do this</b></div><p class="tk-dim">${c.handed ? `${c.handed + 1} of your last 5 talks. Do it every week and they start to ask who runs the team.` : "First time lately. The room still sees it as your call."}</p></section></div>`;
  const says = cap && cap.says && cap.says.length ? cap.says.map((l) => `<p>${esc(l)}</p>`).join("") : "<p>He keeps it short.</p>";
  const mid = `<div class="tk-col"><section class="tk-panel tk-speech"><div class="tk-ph"><b>What ${cap ? esc(cap.name.split(" ").slice(-1)[0]) : "he"} says</b><span>in his own words</span></div><blockquote>${says}</blockquote></section></div>`;
  const right = `<div class="tk-col">${tkUnits(x.units, "The room read", "bar: now · tick: before", true)}${tkKey(x.key.slice(0, 3), "Who stood out")}
    <section class="tk-panel">${tkStaff(x)}<div class="tk-actions">${x.moment === "halftime" ? `<button class="st-btn primary" data-go="live">Back out for the second half</button>` : `<button class="st-btn primary" data-act="tk-run">Take them out</button><button class="st-btn glass" data-act="tk-undo">Change it</button>`}</div></section></div>`;
  return `${head}<div class="tk-grid">${left}${mid}${right}</div>`;
}

// ---------- the live game's parts ----------
// The halftime board's talk block (gameday.js halftimePanel).
function tkHalfBlock(st) {
  const h = st.halftime || {};
  const r = (st.talks && st.talks.rooms || []).filter((x) => x.halftime);
  if (h.talked) {
    const p = r.length ? r[r.length - 1].pick : null;
    return `<div><div class="st-label">The halftime talk</div><div class="lv-advice">${p ? `You talked to them: ${esc(p.move_name)}, ${esc(p.tone_name.toLowerCase())}.` : "You talked to them."} ${r.length ? esc(r[r.length - 1].first) : ""}</div></div>`;
  }
  return `<div><div class="st-label">The halftime talk</div><div class="lv-row"><button class="st-btn glass" data-go="talk">Talk to the room</button><span class="tk-dim">Or start the half and your staff reads the room for you.</span></div></div>`;
}

// The sideline card: a player waiting for a word (gameday.js decision).
function tkWordCard(st) {
  const w = (st.words || [])[0];
  if (!w) return "";
  const opts = w.options.map((o, i) => `<button class="tk-word ${o.staff ? "staff" : ""}" data-act="tk-word" data-id="${w.id}" data-i="${i}"><b>${esc(o.label)}</b><span>“${esc(o.line)}”</span><small class="${/rattle/.test(o.read) ? "tk-warn" : ""}">${esc(o.read)}${o.staff ? " · your position coach's word" : ""}</small></button>`).join("");
  return `<details class="tk-card" open><summary><span class="st-label">A word on the sideline</span> ${playerLink(w.id, w.name)} <small>${esc(w.pos)} · ${esc(w.mistake)}</small></summary>
    <p class="tk-dim">${esc(w.tag)}. He's coming off now. Say nothing and his position coach has the word before the next snap.</p><div class="tk-words">${opts}</div></details>`;
}

// ---------- actions ----------
Object.assign(CORE_MORE, {
  "tk-pick": (el) => {
    TK.pick = { ...(TK.pick || {}), [el.dataset.k]: el.dataset.v };
    if (el.dataset.k === "move" && el.dataset.v !== "captain") delete TK.pick.who;
    render($(".view") ? $(".view").scrollTop : 0);
  },
  "tk-captain": (el) => {
    TK.pick = { ...(TK.pick || {}), move: "captain", tone: "calm", who: Number(el.dataset.id) };
    render($(".view") ? $(".view").scrollTop : 0);
  },
  "tk-say": async () => tkSay(TK.pick || {}),
  "tk-nothing": async () => tkSay({ nothing: true }),
  "tk-undo": async () => {
    try { await api("talk_say", { undo: true }); } catch (e) { toast(e.message); }
    render();
  },
  "tk-run": async () => {
    showBusy("Out of the tunnel…");
    try {
      await api("gameday_kickoff");
      TK.pick = null;
      hideBusy();
      go("live");
    } catch (e) {
      hideBusy();
      toast(e.message);
    }
  },
});
document.addEventListener("change", (ev) => {
  if (ev.target.dataset && ev.target.dataset.act === "tk-who") {
    TK.pick = { ...(TK.pick || {}), who: Number(ev.target.value) };
    render($(".view") ? $(".view").scrollTop : 0);
  }
});

async function tkSay(pick) {
  try {
    const x = await api("talk_room", TK.pick || {});
    const p = pick.nothing ? { nothing: true } : { move: x.pick.move, tone: x.pick.tone, subject: x.pick.subject, who: x.pick.who };
    if (x.moment === "halftime") {
      await api("live_answer", { kind: "talk", ...p });
    } else {
      await api("talk_say", p);
    }
    TK.pick = null;
    render(0);
  } catch (e) {
    toast(e.message);
  }
}

// The Handbook's page (lane 8's hbAdd), from docs/reference/how-it-works/people.md, "Game day talks".
if (typeof hbAdd === "function") {
  hbAdd({
    id: "talks", group: "Running the team", title: "Game day talks",
    lede: "Before kickoff and at halftime you talk to the whole room, or hand it to a captain; after a costly mistake you can have a word with the player. Every team's coach does the same, and what it does comes from each player: his temper, composure, ego, years and how much he believes in you.",
    how: [
      ["Pick a move, a tone and a subject", "Tell them straight, challenge, praise, warn and the rest; fiery to calm; the stakes, them, us, the plan, the seniors or one player. Your line comes out in your own voice, and your staff reads how it will land."],
      ["Each player has his own pitch", "A guard plays best fired up, a quarterback and a kicker calmer. A fiery talk lifts the front and can tip a nervy young lineman over the top: an early false start, a late hit. The same tone every week wears out."],
      ["It fades through the first quarter", "What a talk does is on the field from the first snap and fades as the game goes, faster for young players. At halftime your talk takes over from what is left; the break is 13 minutes in the NFL, 20 in college and high school."],
      ["A word after a mistake", "A turnover, a flag that kept their drive alive or killed yours, a blown assignment on their score, or a third small mistake: he gets a word when he comes off. One that lands takes the sting out; your own words also move his trust in you."],
    ],
    lv: {
      hs: "Teenagers swing furthest: a big Friday fires them up, and a hard word can shrink a sophomore. Seniors' night and the crosstown rival raise the stakes.",
      college: "Stakes come from the game: a rivalry, two ranked teams, a playoff place, a bowl, a coach back at the school that fired him.",
      nfl: "Veterans have heard every speech: talks reach them less, and a captain's voice often carries further than yours, above all after a loss.",
    },
    where: [["Game day", "gameday"], ["Team talk", "talk"], ["The live game", "live"]],
    rel: ["people", "gameplan", "tendencies"],
  });
  hbScreen("talk", "talks", [
    ["Your talk", "The move, the tone and the subject; one player or a captain.", "Your talk"],
    ["Your staff's read", "Your coordinators' words and the risk they see.", "Your staff's read"],
    ["How the room will take it", "Each unit's composure and focus: the bar after your line, the tick now.", "How the room will take it"],
    ["Key players", "The men it moves most, and why.", "Key players"],
  ]);
}
