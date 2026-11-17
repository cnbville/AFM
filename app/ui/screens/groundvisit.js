/* Ground's school visit (lane S3; docs/design/ground.md part F, board 3,
 * GR72, GR79). One visit to a high school, played on your eye, in the
 * school's colors: its stops in order down the left (the sign-in sheet with
 * who came before you, the coach's office, the weight room, the counselor,
 * practice in season) with where the school's kids went, the talk with the
 * head coach in the middle (his lines, your chips, your own words when you
 * have a key), and on the right the new lead filling its blanks as the stops
 * go, who heard of him first and your ground with the coach.
 *
 * It is the page the game opens for a school visit scene: `people_scene`
 * hands it the scene (`gvScene`, below). It reads `people_scene` (the
 * lines, the picks, the sign-in sheet; its `ground` part: the stops so far,
 * the kids with the new names marked, the hours left, where the talk is),
 * `ground_school` (its colors, where its kids went), `ground_person` (the
 * head coach: his face, his standing with every staff) and `ground_lead`
 * (the new lead's blanks and who heard first), and it writes
 * `people_answer` (a stop, a pick in the talk), `people_hand` (skip with its
 * result) and a typed line (live talks, `people_say`). Nothing here shows a
 * hidden number: your eye is a word, a coach's nature a word from his
 * record. Every class is gv- (the glass and rows reuse the school page's
 * gd- ones) so the app's global names never restyle it. */
"use strict";

// The stops of a visit, with the hours each takes (`scenes/visit.rs`).
const GV_STOPS = [
  { key: "office", code: 1, name: "The coach's office", min: 60, open: "His list, his word on the kids, and his ties" },
  { key: "practice", code: 4, name: "Practice", min: 60, open: "A live look at the kids here", optional: true },
  { key: "weight", code: 2, name: "The weight room", min: 30, open: "What your eye catches" },
  { key: "counselor", code: 3, name: "The counselor", min: 30, open: "Grades and core courses, with the family's okay" },
];
const GV_LEAVE = 900;
const GV_TALK = { ask: "asked about his kids", straight: "told him straight about his best kid", clinic: "asked him to your clinic", sell: "sold him on your program" };
const gvCap = (s) => (s ? s.charAt(0).toUpperCase() + s.slice(1) : s);
const gvClock = (m) => {
  const h = Math.floor(m / 60) % 24;
  return `${h % 12 || 12}:${String(m % 60).padStart(2, "0")}`;
};
const gvHours = (min) => {
  const h = Math.floor(min / 60), m = min % 60;
  return !min ? "None left" : h && m ? `${h} h ${m} min` : h ? `${h} h` : `${m} min`;
};
const GV = { face: null };

async function gvScene(s) {
  const g = s.ground;
  const lead = (s.who || []).find((x) => x.kind === "coach");
  const [school, coach] = await Promise.all([
    g.school ? api("ground_school", { id: g.school.id }).catch(() => null) : null,
    lead ? api("ground_person", { kind: "coach", id: lead.id }).catch(() => null) : null,
  ]);
  const newKids = (g.kids || []).filter((k) => k.new);
  const st = keep("gvlead:" + s.id, { kid: null });
  if (!newKids.some((k) => k.id === st.kid)) st.kid = newKids.length ? newKids[newKids.length - 1].id : null;
  const card = st.kid != null ? await api("ground_lead", { id: st.kid }).catch(() => null) : null;

  const colors = (school && school.school.colors) || ["#1d7a55", "#0a2d20"];
  title(plainText(s.title), "A school visit");
  GV.face = coach ? coach.face : null;
  const ay = aiYours("scene:" + s.id);
  const key = "scene:" + s.id;
  const box = typeof aiBoxPeek === "function" ? aiBoxPeek(s, key) : null;
  const open = (code) => s.picks.find((p) => p.code === code);
  const inOffice = g.at === "office";

  // ---- the topbar ----
  const when = `${s.dateText}${g.time ? ` · ${g.time}` : ""}`;
  const here = school ? school.school.name : g.school ? g.school.name : "";
  const man = g.man && !g.manIsYou ? `<span class="gd-chip gv-man">You're ${esc(g.man)} here</span>` : "";
  const hand = !s.done ? `<button class="st-btn glass ${typeof ppHandCls === "function" ? ppHandCls(s) : ""}" data-act="pphand" data-id="${s.id}" title="See what you give up and what you get first">${esc(typeof ppHandLabel === "function" ? ppHandLabel(s) : "Skip with its result")}</button>` : "";
  const lv = coach ? coach.level : g.who && g.who.level;
  const top = `<div class="gd-top"><div><h1>${esc(when)}</h1><small>${esc(here)} · ${esc(g.man && !g.manIsYou ? `${g.man}'s visit` : "Your visit")}, played on ${g.manIsYou ? "your" : "his"} eye</small></div><div class="gd-sp"></div>${man}
    <div class="gp-stat"><span>${g.manIsYou ? "Your eye" : "His eye"}</span><b>${esc(gvCap(g.eye ? g.eye.words : "fair"))}${g.eye && g.eye.tired ? " · tired" : ""}</b></div>
    <div class="gp-stat"><span>With him</span><b>${esc(lv || "Stranger")}</b></div>
    <div class="gp-stat"><span>Time here</span><b>${esc(s.done ? "Done" : gvHours(g.minutesLeft))}</b></div>${hand}</div>`;

  // ---- the left column: the stops in order, where the school's kids went ----
  const left = `<div class="gv-col l">${gvStops(s, g, open)}${gvWent(school)}</div>`;

  // ---- the middle: the coach and the talk ----
  const mid = `<div class="gv-col m"><section class="gd-glass gv-talkbox">${gvHead(lead, coach, g, school)}${gvTalk(s, coach, ay)}${gvEnd(s)}${gvMoves(s, g, open, inOffice)}${!s.done && inOffice && box ? `<div class="gv-comp">${aiSayBox(s, key, { preset: { cmd: "people_say", args: { id: s.id }, limit: 400 }, log: key, words: { ph: "Or say it your way…", title: "Say it your way" } })}</div>` : ""}</section></div>`;

  // ---- the right: the new lead, who heard first, your ground with him ----
  const right = `<div class="gv-col r">${gvLead(s, g, newKids, card, open)}${card ? gvOrder(card) : ""}${gvGround(g, coach)}</div>`;
  return `<div class="sl gd"><div class="gd-room gv-room" style="--c1:${esc(colors[0])};--c2:${esc(colors[1] || colors[0])}">${top}<div class="gv-body">${left}${mid}${right}</div></div></div>`;
}

// The stops: done ones with what they came to, the one you're in, the open
// ones with the hour they would start. The sign-in sheet is always first.
function gvStops(s, g, open) {
  const start = g.start != null ? g.start : 9 * 60;
  const total = g.minutesTotal || 240;
  const left = g.minutesLeft != null ? g.minutesLeft : total;
  const doneKeys = (g.stops || []).map((x) => x.key).filter((k) => GV_STOPS.some((q) => q.key === k));
  const talked = (g.stops || []).filter((x) => GV_TALK[x.key]).map((x) => GV_TALK[x.key]);
  const sheet = (s.docs || []).find((d) => d.title === "The sign-in sheet");
  const sheetHtml = sheet && sheet.rows.length
    ? `<div class="gv-sheet" aria-label="The sign-in sheet">${sheet.rows.map((r) => `<div><span>${esc(r.cells[1])}</span><span>${esc(r.cells[0])}<i>${esc(r.cells[2])}</i></span></div>`).join("")}</div>`
    : '<div class="gv-sheet blank">No college names on it since spring. Nobody has been in.</div>';
  const rows = [`<div class="gv-stop"><span class="t">${gvClock(start)}</span><span class="n done"></span><div><b>Front office, the sign-in sheet</b><small>Who came before you</small>${sheetHtml}</div></div>`];
  // The ones you've done, in the order you did them, then the ones still open.
  const order = [...doneKeys.map((k) => GV_STOPS.find((q) => q.key === k)), ...GV_STOPS.filter((q) => !doneKeys.includes(q.key))];
  let cursor = start;
  let count = 1;
  for (const k of order) {
    const d = (g.stops || []).find((x) => x.key === k.key);
    const pick = open(k.code);
    if (d) {
      const here = g.at === k.key;
      const text = k.key === "office" ? (talked.length ? `You ${talked.join(", ")}.` : "") : d.text;
      rows.push(`<div class="gv-stop"><span class="t">${gvClock(cursor)}</span><span class="n ${here ? "now" : "done"}"></span><div><b>${esc(k.name)}</b>${text || here ? `<small>${esc(text || "")}${here ? " Now." : ""}</small>` : ""}</div></div>`);
      cursor += k.min;
      count++;
    } else if (!s.done && (pick || (g.at && !k.optional && left >= k.min))) {
      const play = pick && !g.at ? ` data-act="gvpick" data-id="${s.id}" data-code="${k.code}" role="button" tabindex="0"` : "";
      rows.push(`<div class="gv-stop open"${play}><span class="t">${gvClock(cursor)}</span><span class="n"></span><div><b>${esc(k.name)}</b><small>${esc((pick && pick.hint) || k.open)}</small></div></div>`);
      cursor += k.min;
      count++;
    } else if (!k.optional) {
      rows.push(`<div class="gv-stop gone"><span class="t">—</span><span class="n"></span><div><b>${esc(k.name)}</b><small>${s.done ? "Not this time." : "No time left for it."}</small></div></div>`);
      count++;
    }
  }
  return `<section class="gd-glass gv-stops"><div class="gd-lab2">The visit · ${count} stops</div>${rows.join("")}</section>`;
}

// Where the school's kids went: its colleges, with your own marked.
function gvWent(x) {
  if (!x) return "";
  const rows = x.went.slice(0, 6);
  const maxk = Math.max(1, ...rows.map((w) => w.kids));
  const list = rows.length
    ? rows.map((w) => `<div class="gv-hb"><span style="${w.yours ? "color:#8fb0ff" : ""}">${esc(w.name)}</span><span class="tr"><i style="width:${Math.max(6, Math.round((100 * w.kids) / maxk))}%;background:${w.yours ? "#2f6fff" : gdTone(w.color, 0.9)}"></i></span><b>${w.kids}</b></div>`).join("")
    : '<div class="gd-empty">Nobody from here has gone on in this world\'s years.</div>';
  return `<section class="gd-glass"><div class="gd-lab2">Where ${esc(x.school.name)}'s kids went${x.first_season ? ` · since ${x.first_season}` : ""}</div><div class="gv-went">${list}</div><small class="gv-note">${x.kids} on to college, ${x.drafted} drafted.${x.went.length > 6 ? ` ${x.went.length - 6} more colleges took one or two.` : ""}</small></section>`;
}

// The coach: his head, who he is, where you stand, who is Trusted here.
function gvHead(lead, coach, g, school) {
  if (!lead) return `<div class="gv-hd"><div class="gv-pic"><span class="gp-mono" style="--c1:#444;--c2:#222">?</span></div><div class="gv-who"><div class="gd-lab2">Head job open</div><h1>Nobody's in the office</h1></div></div>`;
  const f = coach && coach.face;
  const [c1, c2] = f ? f.colors : ["#444", "#222"];
  const pic = f && f.drawn ? `<img class="gp-face-img" src="${f.drawn}" alt="${esc(g.who ? g.who.name : "")}">` : `<span class="gp-mono" style="--c1:${esc(c1)};--c2:${esc(c2)}">${esc(f ? f.initials : "?")}</span>`;
  const sr = coach && coach.staff_read;
  const since = school && school.head_since;
  const lab = [coach ? gvCap((coach.role || "Head coach").split(" at ")[0]) + (since ? ` since ${since}` : "") : "Head coach", sr && sr.nature ? sr.nature : ""].filter(Boolean).join(" · ");
  const others = coach ? coach.staffs.filter((x) => !x.you && x.level.n > 0).sort((a, b) => b.level.n - a.level.n)[0] : null;
  const name = coach ? coach.name : g.who.name;
  const lv = coach ? coach.lv : null;
  return `<div class="gv-hd"><div class="gv-pic" data-gpface="${f ? f.id : ""}">${pic}</div><div class="gv-who"><div class="gd-lab2">${esc(lab)}</div><h1><a data-go="groundperson" data-kind="coach" data-id="${lead.id}">${esc(name)}</a></h1>
    <div class="gv-lv">${lv ? gdLv(lv, lv.n ? "you" : "") : ""}${others ? `<small>${esc(others.name)} is ${esc(others.level.name)} here</small>` : ""}</div></div></div>`;
}

// The talk: what you noticed in italics, your lines, his, with how a typed
// line was heard under yours.
function gvTalk(s, coach, ay) {
  const f = coach && coach.face;
  const av = f && f.drawn ? `<img class="gv-av" src="${f.drawn}" alt="">` : `<span class="gv-av ini">${esc(f ? f.initials : "?")}</span>`;
  const bits = [];
  let prevBy = -1;
  s.lines.forEach((l) => {
    if (l.by === 0) bits.push(`<p class="gv-tell">${rich(l.text)}</p>`);
    else if (l.by === 1) {
      const heard = ay(l);
      bits.push(`<div class="gv-you"><p>${rich(l.text)}</p>${heard ? `<div class="ai-heard">${heard}</div>` : ""}</div>`);
    } else {
      bits.push(`<div class="gv-him">${prevBy === 2 ? '<span class="gv-av gap"></span>' : av}<div class="b">${prevBy !== 2 && l.name ? `<span class="who">${esc(l.name)}</span>` : ""}<p>${rich(l.text)}</p>${l.tell ? `<em>${esc(l.tell)}</em>` : ""}</div></div>`);
    }
    prevBy = l.by;
  });
  if (!bits.length) bits.push(`<p class="gv-tell">${esc(s.done ? "Nothing was said." : "You sign in at the front office.")}</p>`);
  return `<div class="gv-talk" data-keep="gvtalk">${bits.join("")}</div>`;
}

// What you walk away with, and where to next.
function gvEnd(s) {
  if (!s.done) return "";
  const lines = (s.outcome.length ? s.outcome : ["It's done."]).map((o) => `<p>${rich(o)}</p>`).join("");
  return `<div class="gv-end"><div class="gd-lab2">${s.handed ? `Handed to ${esc(s.handed)}` : "What you walk away with"}</div>${lines}<div class="gv-acts">${s.next ? `<button class="st-btn small primary" data-act="ppopen" data-id="${s.next.id}">Next: ${esc(plainText(s.next.title))}</button>` : ""}<button class="st-btn small glass" data-go="groundweek">Back to the week</button></div></div>`;
}

// Your picks as chips under the talk: in his office the talk's own, between
// stops the stops themselves and heading out.
function gvMoves(s, g, open, inOffice) {
  if (s.done) return "";
  if (!s.picks.length) return `<div class="gv-next">Waiting on him.</div>`;
  const hint = inOffice ? "" : `<div class="gv-next">Where to next? Pick a stop, or head out.</div>`;
  return `${hint}<div class="gv-ideas">${s.picks.map((p) => `<button class="gv-idea${p.code === 109 || p.code === GV_LEAVE ? " end" : ""}" data-act="gvpick" data-id="${s.id}" data-code="${p.code}" title="${esc(p.hint || "")}">${esc(p.label)}${inOffice || p.code === GV_LEAVE ? "" : `<small>${esc(p.hint || "")}</small>`}</button>`).join("")}</div>`;
}

// A new lead, the five blanks, filling as the stops go.
function gvLead(s, g, newKids, card, open) {
  if (!newKids.length || !card) {
    return `<section class="gd-glass"><div class="gd-lab2">A new lead</div><div class="gd-empty">No new name yet. His list in the office and your eye in the weight room are where they come from.</div></section>`;
  }
  const k = card.kid;
  const l = card.lead;
  const tabs = newKids.length > 1 ? `<div class="gv-tabs">${newKids.map((x) => `<button class="gd-chip${x.id === k.id ? " gold" : ""}" data-act="gvkid" data-s="${s.id}" data-id="${x.id}">${esc(x.name)}</button>`).join("")}</div>` : "";
  const pub = k.stars > 0 ? `${k.stars}-star` : "Not rated";
  const offers = k.offers.length ? `${k.offers.length} offer${k.offers.length === 1 ? "" : "s"}` : "No offers";
  const said = l && (l.said_height || l.claim) ? [l.said_height, l.said_forty ? `${l.said_forty} forty` : ""].filter(Boolean).join(", ") || gvCap(l.claim) : "Nothing yet";
  const filled = card.blanks.filter((b) => b.filled).length;
  const next = (b) => (b.key === "live" && (open(2) || open(4)) && !b.filled) || (b.key === "character" && false);
  const blanks = card.blanks.map((b, i) => `<div class="gv-blank${b.filled ? " done" : ""}${next(b) ? " now" : ""}"><span class="n">${b.filled ? "✓" : i + 1}</span><div><b>${esc(b.name)}</b><small>${b.filled ? `Filled ${esc(b.filled.date)}, ${esc(b.filled.by)}` : esc(b.fills)}</small></div><span class="d">${b.filled ? "" : esc(b.time)}</span></div>`).join("");
  const goto = typeof screens.groundlead === "function" ? "groundlead" : "player";
  return `<section class="gd-glass gv-lead"><div class="gv-lh">${gdIni(k.name, 40)}<div><div class="gd-lab2">A new lead</div><div class="gv-ln"><a data-go="${goto}" data-id="${k.id}">${esc(k.name)}</a></div><small>${esc(k.pos)} · class of ${k.class_of}</small></div></div>${tabs}
    <div class="gd-kv c2"><div><span>Public</span><b>${esc(pub)}</b><small>${esc(offers)}</small></div><div><span>The coach says</span><b>${esc(said)}</b><small>${l && l.said ? esc(l.said) : ""}</small></div></div>
    <div class="gd-lab2">Five blanks · ${filled ? `${filled} filled` : "none filled yet"}</div><div class="gv-blanks">${blanks}</div></section>`;
}

// Who heard first: the order the news went, with the level each held.
function gvOrder(card) {
  const n = card.news && card.news[0];
  if (!n) return "";
  const rows = n.order.map((h) => `<div class="gv-ord"><span class="t">${esc(h.day)}</span><span class="dot${h.you ? " you" : ""}"></span><div><b>${esc(h.program)}${h.you ? " (you)" : ""}</b><small>${esc(h.level)} here${h.why ? ` · ${esc(String(h.why).replace(/_/g, " "))}` : ""}</small></div></div>`).join("");
  const found = card.find;
  return `<section class="gd-glass"><div class="gd-lab2">Who heard first</div>${rows || '<div class="gd-empty">Nobody on record yet.</div>'}<small class="gv-note">${esc(n.order_words || "")} Hearing first doesn't make him a find. Offering first does, and ${found ? "you have" : "nobody has"}.</small></section>`;
}

// Your ground with the coach, and what this visit moved.
function gvGround(g, coach) {
  if (!coach) return "";
  const moved = (g.levels || []).filter((x) => x.who === `coach:${coach.key.id}` && x.from !== x.to);
  const word = coach.moments && coach.moments[0] ? `“${esc(coach.moments[0].words)}”` : "";
  return `<section class="gd-glass"><div class="gd-lab2">Your ground with him</div><div class="gv-gl">${gdLv(coach.lv, coach.lv.n ? "you" : "")}<small>${coach.met ? `since ${esc(coach.met)}` : "you've just met"}</small></div>
    ${moved.map((m) => `<div class="gv-moved">${esc(m.from)} → <b>${esc(m.to)}</b> today</div>`).join("")}
    <small class="gv-note">${esc(coach.why || "")}</small>${word ? `<small class="gv-q">${word}</small>` : ""}<small class="gv-note">${esc(coach.gets.next ? `${coach.gets.next.level}: ${coach.gets.next.words}` : "There is nothing above Home. Keep it.")}</small></section>`;
}

Object.assign(CORE_MORE, {
  // A stop or a pick in the talk: the scene plays it and the page draws again.
  gvpick: async (el) => {
    if (el.disabled) return;
    el.disabled = true;
    try {
      const r = await api("people_answer", { id: Number(el.dataset.id), code: Number(el.dataset.code) });
      if (r && r.done) S.status = await api("status");
      refresh(el);
    } catch (e) { toast(e.message); el.disabled = false; }
  },
  gvkid: (el) => { keep("gvlead:" + el.dataset.s, {}).kid = Number(el.dataset.id); refresh(el); },
});

// After the page draws: the talk shows its last line, and the coach's head is
// drawn from his recipe (kept in the save's folder for next time).
(() => {
  const after0 = afterRender;
  afterRender = function () {
    after0();
    const talk = document.querySelector(".gv-talk");
    if (talk) talk.scrollTop = talk.scrollHeight;
    const f = GV.face;
    const box = f && document.querySelector(`.gv-pic[data-gpface="${f.id}"]`);
    if (!box || box.querySelector("img") || !f.drawn_on || !f.spec || !window.AFMFaces || !AFMFaces.liveHead) return;
    AFMFaces.liveHead(`gp${f.id}-${f.key}`, JSON.parse(JSON.stringify(f.spec))).then((url) => {
      if (!url) return;
      const b = document.querySelector(`.gv-pic[data-gpface="${f.id}"]`);
      if (b && !b.querySelector("img")) b.innerHTML = `<img class="gp-face-img" src="${url}" alt="">`;
      api("face_drawn_store", { id: f.id, key: f.key, data: url }).catch(() => {});
    });
  };
})();
