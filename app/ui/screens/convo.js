// Live talks v3, lane N6: the conversation page (docs/design/live-talks-persona.md
// section 16.3; the six boards Cain agreed on 2026-11-15, afm/live-talks/v3-mockups/).
// One page for any talk with a person across from you, in the section 10 Talk
// look: a room in your team's colours behind three glass columns. Him on the
// left (or who is in the room), the talk in the middle (chat bubbles, his tells,
// "Heard as" under your lines, a gold card for each promise logged, what the
// game checked), where it stands and what you can do on the right.
//
// What the columns hold comes from the scene's reply. The page draws today's
// `people_scene` and `people_player` as they are (his mood and reasons, your
// promises, the trust bar, the moves and subject chips with no key) and
// draws more when the reply carries `talk`: a block the game fills from the
// persona lanes as they land (docs/reference/how-it-works/live-talks.md,
// "Live talks v3, lane N6: the screens"). Every block is optional; nothing
// is drawn for a block the game did not send.
//
//   talk.persona   true when the talk plays on the ear and the voice (N4): no moves list, ideas instead
//   talk.channel   "In person" | "Phone call" | "Text" | ... (the header)
//   talk.minutes   {left, of} minutes of this talk (outlined number), else the week's hours
//   talk.left      sections down the left column, after his head card
//   talk.room      who is in the room (replaces the head card): [{id, name, ini, tag, tagKind,
//                  line, tone}], talk.roomActs [{text, cmd, args}]
//   talk.right     sections down the right column, before the move list
//   talk.after     {lineIndex: [cards]}: cards under line N of the talk (logged, checked,
//                  read, built, staff)
//   talk.ideas     [text]: a click puts the words in the box, to change
//   talk.moves     {tabs: [{k, name, note}], builder: {...}}: the pick lists
//   talk.over      true when the talk ended on a meeting that still holds a decision: no box,
//                  the meeting's own picks as "Still yours to decide" (talk_wrap ends a talk)
//   talk.confirm   [{text, cmd, args}]: what waits on your say-so (N4's talk_confirm; with talk.persona
//                  the game lists it on every draw, so a reload keeps it)
// A section is {k, label, note, ...} with k one of: text, entries, tiles, chips,
// bars, split, clips, word, stand, asks, learned, rank, facts, deal, ideas.
"use strict";

const CV_EYE = '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M2 12s3.5-6 10-6 10 6 10 6-3.5 6-10 6S2 12 2 12z"/><circle cx="12" cy="12" r="2.5"/></svg>';
const CV_SPK = '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M11 5 6 9H3v6h3l5 4z"/><path d="M15.5 8.5a5 5 0 0 1 0 7"/></svg>';
const cvIni = (n) => String(n || "").split(/\s+/).filter(Boolean).slice(0, 2).map((w) => w[0]).join("").toUpperCase();
const CV_CONFIRM = {};
// The game's words with their links, and **bold** for the part that matters.
const cvT = (t) => rich(t).replace(/\*\*(.+?)\*\*/g, "<b>$1</b>");
const cvPct = (v) => Math.round(100 * Math.max(0, Math.min(1, v)));

// ---------- the page ----------
// Which talks draw here: a person across from you, a talk that has opened.
function cvWants(s, lead) {
  if (!lead || s.future) return false;
  return lead.kind === "player" || lead.kind === "cast" || lead.kind === "coach" || !!(s.talk && s.talk.room);
}

// His drawn head, with the expression and the turn of the tell your read caught
// (`s.face`, as the face-to-face page drew it), else his still, else a silhouette.
async function cvHead(lead, cue) {
  const f = await api("face", { id: lead.id }).catch(() => null);
  const mine = f && f.mine && f.mine.mode !== "team" && f.mine.data;
  if (mine) return `<img class="cv-head photo" src="${mine}" alt="">`;
  if (f && f.drawn_on && f.spec && cue && window.AFMFaces && AFMFaces.liveHead) {
    const key = `${lead.id}|${cue.expression}|${cue.strength}|${cue.yaw}|${cue.down}`;
    const spec = { ...f.spec, expression: cue.expression, strength: cue.strength, view: { yaw: cue.yaw, down: cue.down } };
    AFMFaces.liveHead(key, spec).then((url) => {
      const el = document.querySelector(`[data-cvhead="${CSS.escape(key)}"]`);
      if (url && el) el.src = url;
    });
    return `<img class="cv-head" data-cvhead="${esc(key)}" ${f.drawn ? `src="${f.drawn}"` : ""} alt="">`;
  }
  if (f && f.drawn) return `<img class="cv-head" src="${f.drawn}" alt="">`;
  return "";
}

async function cvScene(s, lead, pp, cast, st) {
  const colors = (S.status && S.status.mine && S.status.mine.colors) || ["#2b3140", "#11141b"];
  const T = s.talk || {};
  const key = "scene:" + s.id;
  const box = aiBoxPeek(s, key);
  // A talk the game plays on the ear and the voice (`talk.persona`, set by the game
  // with a key): no answer buttons, ideas take their place (question 7).
  // When the typed line fails for good (the box says "off"), the moves come back.
  const v3 = !!box && !!T.persona && !(aiState.boxes[key] && aiState.boxes[key].phase === "off");
  // The talk ended on a meeting that still holds a decision (`talk.over`): no box,
  // the meeting's own picks as one decision.
  const over = !!T.over && !s.done;
  const peek = lead && lead.kind === "player" ? await api("peek", { kind: "player", id: lead.id }).catch(() => null) : null;
  const head = lead && lead.kind === "player" ? await cvHead(lead, s.face) : "";
  const moves = over ? cvDecide(s) : v3 ? "" : await cvMoves(s, st, T);
  const left = cvLeft(s, lead, pp, cast, peek, head, T);
  const mid = cvCenter(s, lead, T, key, over ? null : box);
  const right = cvRight(s, pp, T, moves, v3 && !over, key, box);
  return `<div class="cv st-room" style="--accent:${esc(colors[0])};--accent-2:${esc(colors[1] || colors[0])}"><i></i><i></i><i></i>
    <div class="cv-grid ${T.room ? "cv-grid-room" : ""} ${(T.right || []).some((x) => x.k === "deal") ? "cv-grid-deal" : ""}"><section class="cv-col st-glass cv-left" data-keep="cvL">${left}</section>
    <section class="cv-col st-glass cv-mid">${mid}</section>
    <aside class="cv-col st-glass cv-right" data-keep="cvR">${right}</aside></div></div>`;
}

// The box's state, if the scene shows one (a key is usable or the demo is on).
function aiBoxPeek(s, key) {
  return typeof aiFlag === "function" ? aiFlag(s, "say", { cmd: "people_say", args: { id: s.id }, limit: 400 }) : null;
}

// ---------- the left column ----------
function cvLeft(s, lead, pp, cast, peek, head, T) {
  const tabs = ["Now", "File", "With you", "In his ear"];
  const t = keep("cvside", { t: "Now" });
  if (!tabs.includes(t.t)) t.t = "Now";
  const num = peek && peek.jersey != null ? `<span class="cv-num st-outline">${esc(peek.jersey)}</span>` : "";
  const prime = T.room && T.room.find((p) => p.lead);
  const tagText = peek ? [peek.jersey != null ? `#${peek.jersey}` : "", peek.pos].filter(Boolean).map(esc).join(" · ") : esc(lead.pos || "");
  const bio = peek
    ? `<p class="cv-bio">${[peek["class"], peek.ht && `${peek.ht}, ${peek.wt}`, peek.where].filter(Boolean).map(esc).join(" · ")}</p>`
    : lead.pos ? `<p class="cv-bio">${esc(lead.pos)}</p>` : "";
  // A room (a team meeting, a position room) shows your team's mark, not a
  // face: nobody in it is the one man across.
  const pic = T.team && T.room
    ? `<div class="cv-photo cv-photo-sil">${mark(T.team, 120)}</div>`
    : head
    ? `<div class="cv-photo">${num}${head}<span class="cv-tags"><b>${tagText}</b>${prime && prime.stars ? `<b>${esc(prime.stars)}</b>` : ""}</span></div>`
    : lead.kind === "cast" ? "" : `<div class="cv-photo cv-photo-sil">${face({ name: plainText(lead.link), id: lead.id }, 120)}</div>`;
  const nameRow = `<div class="cv-namerow"><h2 class="cv-name">${cvT(lead.link)}</h2>${prime && prime.tag ? `<em class="cv-tag ${esc(prime.tagKind || "")}">${esc(prime.tag)}</em>` : ""}</div>`;
  if (T.room) return cvRoom(s, T, pic, nameRow, bio, prime);
  // A compact head for talks that are about numbers on the left (the film room).
  const small = T.compact ? `<div class="cv-minihead"><span class="cv-minipic">${head || face({ name: plainText(lead.link), id: lead.id }, 56)}</span><div><h2 class="cv-name sm">${cvT(lead.link)}</h2><p class="cv-bio">${esc(lead.pos || "")}${peek && peek.jersey != null ? ` · #${esc(peek.jersey)}` : ""}${peek && peek["class"] ? ` · ${esc(peek["class"].toLowerCase())}` : ""}</p></div></div>` : "";
  const tabsHtml = pp ? `<div class="cv-seg" role="tablist">${tabs.map((x) => `<button type="button" role="tab" data-act="cvside" data-v="${x}" aria-pressed="${t.t === x}">${x}</button>`).join("")}</div>` : "";
  let body = "";
  if (pp) body = t.t === "Now" ? cvNow(pp, T) : cvSideTab(t.t, pp, lead);
  else if (cast) body = `<div class="cv-sec"><div class="cv-h">Where you stand</div><p>${cast.trust ? esc(cast.trust.words) : "New to you"}</p></div>${(cast.reputation || []).length ? `<div class="cv-sec"><div class="cv-h">What people say</div>${cast.reputation.map((r) => `<p class="cv-small">${esc(r)}</p>`).join("")}</div>` : ""}`;
  const more = (T.left || []).map(cvSection).join("");
  const others = (s.who || []).slice(1);
  const room = others.length ? `<div class="cv-sec"><div class="cv-h">Also in the room</div>${others.map((o) => `<div class="cv-person">${face({ name: plainText(o.link), id: o.kind === "player" ? o.id : undefined }, 30)}<span>${cvT(o.link)}${o.pos ? ` <small>${esc(o.pos)}</small>` : o.role ? ` <small>${esc(ppCap(o.role))}</small>` : ""}</span></div>`).join("")}</div>` : "";
  if (T.compact) return `${small}${more}${room}`;
  return `${tabsHtml}${pic}${nameRow}${bio}${body}${t.t === "Now" || !pp ? more : ""}${room}`;
}

// "Now": his read, in dated lines, from what your people have seen.
function cvNow(pp, T) {
  const mood = `<div class="cv-sec"><div class="cv-h">Your read<span>${pp.mine ? "your own" : "from outside"}</span></div>
    <p class="cv-big">${esc(ppCap(pp.mood.words))}.</p>
    ${pp.staffSays ? `<p class="cv-small">${esc(pp.staffSays)}</p>` : ""}
    ${(pp.reasons || []).slice(0, 4).map((r) => `<p class="cv-entry"><b>${esc(r.date)}</b> ${esc(ppCap(r.why))}</p>`).join("")}
    ${pp.out ? `<p class="cv-entry"><b>Sitting</b> ${esc(pp.out)}</p>` : ""}${pp.life ? `<p class="cv-entry"><b>At home</b> ${esc(ppCap(pp.life))}</p>` : ""}</div>`;
  const word = (pp.promises || []).length
    ? `<div class="cv-word"><b>Your word to him</b>${pp.promises.map((p) => `<span>${esc(ppCap(p.what))} ${p.kept === true ? "<i class=\"ok\">Kept</i>" : p.kept === false ? "<i class=\"bad\">Broken</i>" : `<i>Due ${esc(p.due)}</i>`}</span>`).join("")}</div>`
    : "";
  return mood + word;
}
function cvSideTab(tab, pp, lead) {
  if (tab === "File") {
    return `<div class="cv-sec">${pp.trust ? `<p class="cv-kv"><span>Trust</span><b>${esc(ppCap(pp.trust.words))}</b></p>` : ""}${pp.mentor ? `<p class="cv-kv"><span>Mentor</span><b>${cvT(pp.mentor)}</b></p>` : ""}${pp.case ? `<p class="cv-kv"><span>A case</span><b>${esc(pp.case)}</b></p>` : ""}${pp.line ? `<p class="cv-kv"><span>Your line</span><b>${esc(ppCap(pp.line))}</b></p>` : ""}<p><a class="link" data-go="player" data-id="${lead.id}">His full page</a></p></div>`;
  }
  if (tab === "With you") {
    return `<div class="cv-sec">${(pp.promises || []).length ? pp.promises.map((p) => `<p class="cv-entry"><b>${esc(p.due)}</b> ${esc(ppCap(p.what))} ${p.kept === true ? "<i class=\"ok\">Kept</i>" : p.kept === false ? "<i class=\"bad\">Broken</i>" : ""}</p>`).join("") : '<p class="cv-small">No promises between you.</p>'}</div>`;
  }
  return `<div class="cv-sec">${(pp.ear || []).length ? pp.ear.map((e) => `<p class="cv-entry"><b>${esc(e.sure)}</b> ${cvT(e.who)}<br><span class="cv-small">${esc(e.what)}</span></p>`).join("") : '<p class="cv-small">Nobody your staff knows of.</p>'}</div>`;
}

// Who is in the room (a home visit, a group): everyone hears you.
function cvRoom(s, T, pic, nameRow, bio, prime) {
  const others = T.room.filter((p) => !p.lead);
  // The men in the room as the talk counts them (a room's own list leads with
  // the room itself).
  const n = T.roomN != null ? T.roomN : T.room.length;
  const people = others.map((p) => `<div class="cv-who"><span class="cv-ini ${esc(p.tone || "")}">${esc(p.ini || cvIni(p.name))}</span><div><div class="cv-whoh"><b>${esc(p.name)}</b>${p.tag ? `<em class="cv-tag ${esc(p.tagKind || "")}">${esc(p.tag)}</em>` : ""}</div><p>${esc(p.line || "")}</p></div></div>`).join("");
  const acts = (T.roomActs || []).length
    ? `<div class="cv-sec"><div class="cv-h">Change who's in the room</div>${T.roomActs.map((a) => `<button type="button" class="cv-pill" data-act="cvdo" data-cmd="${esc(a.cmd)}" data-args='${esc(JSON.stringify(a.args || {}))}'>${esc(a.text)}</button>`).join("")}</div>` : "";
  return `<div class="cv-h">In the room<span>${n} ${n === 1 ? "person" : "people"} · they all hear you</span></div>${pic}${nameRow}${bio}${prime && prime.line ? `<p class="cv-small">${esc(prime.line)}</p>` : ""}${people ? `<div class="cv-sec">${people}</div>` : ""}${(T.left || []).map(cvSection).join("")}${acts}`;
}

// ---------- the middle ----------
function cvCenter(s, lead, T, key, box) {
  const left = T.minutes || s.clock;
  const big = left ? (left.left != null ? left.left : left.min) : s.hours != null ? Math.round(s.hours * 10) / 10 : null;
  const unit = left ? "min" : "h";
  const sub = s.hours != null ? `${Math.round(s.hours * 10) / 10}${T.week ? ` of ${T.week}` : ""} h left this week` : "";
  const parts = [s.kindName, T.channel, s.place].filter(Boolean);
  const place = parts.filter((x, i) => !parts.slice(0, i).some((y) => y.toLowerCase().includes(x.toLowerCase()) || x.toLowerCase().includes(y.toLowerCase()))).map(esc).join(" · ");
  const ay = aiYours(key);
  const after = T.after || {};
  const me = (S.status && S.status.mine && S.status.mine.name) || "";
  const bits = [];
  s.lines.forEach((l, i) => {
    if (l.by === 0) bits.push(`<p class="cv-narr">${CV_EYE}<span>${cvT(l.text)}</span></p>`);
    else if (l.by === 1) {
      const heard = ay(l);
      const picked = l.picked ? `<div class="ai-ha"><span class="ai-lb">You picked</span><span>${esc(l.picked)}</span></div>` : "";
      bits.push(`<div class="cv-you"><p>${cvT(l.text)}</p>${heard || picked ? `<div class="cv-heard">${heard}${picked}</div>` : ""}</div>`);
    } else {
      const who = l.name || "";
      const first = bitsPrevBy(s.lines, i) !== l.by || bitsPrevName(s.lines, i) !== who;
      bits.push(`<div class="cv-him">${first ? `<span class="cv-av">${cvAv(lead, who)}</span>` : `<span class="cv-av gap"></span>`}<div class="cv-b">${first && who && s.who && s.who.length > 1 ? `<span class="cv-who-n">${esc(who)}</span>` : ""}<p>${cvT(l.text)}</p>${l.tell ? `<span class="cv-tell">${CV_EYE}<i>${esc(l.tell)}</i></span>` : ""}</div></div>`);
    }
    for (const c of after[i] || []) bits.push(cvCard(c));
  });
  if (!s.lines.length) bits.push(`<p class="cv-narr cv-quiet">${esc(s.done ? "Nothing was said." : "It's quiet. You speak first.")}</p>`);
  const b = aiState.boxes[key];
  const confirm = cvConfirm(s, T, b);
  const docs = (s.docs || []).map((d) => `<section class="cv-doc"><div class="cv-h">${esc(d.title)}</div>${d.note ? `<p class="cv-small">${cvT(d.note)}</p>` : ""}<table><thead><tr>${d.cols.map((c) => `<th>${esc(c)}</th>`).join("")}</tr></thead><tbody>${d.rows.map((r) => `<tr>${r.cells.map((c) => `<td>${cvT(c)}</td>`).join("")}</tr>`).join("")}</tbody></table></section>`).join("");
  const ended = s.done
    ? `<section class="cv-end"><div class="cv-h">${s.handed ? `Handed to ${esc(s.handed)}` : "How it went"}</div>${(s.outcome.length ? s.outcome : ["It's done."]).map((o) => `<p>${cvT(o)}</p>`).join("")}<div class="cv-acts">${s.next ? `<button type="button" class="st-btn small primary" data-act="ppopen" data-id="${s.next.id}">Next: ${esc(plainText(s.next.title))}</button>` : ""}<button type="button" class="st-btn small glass" data-nav="people">Back to People</button></div></section>`
    : "";
  const to = T.to && T.to.list ? keep("cvto:" + s.id, {}) : null;
  if (to && s.say && !to.who) to.who = T.to.list[0];
  if (to && s.say) s.say = { ...s.say, args: { ...(s.say.args || {}), to: to.who } };
  const toBar = to ? `<div class="cv-to"><span>${esc(T.to.label || "Say something")}</span>${T.to.list.map((n) => `<button type="button" class="cv-chip ${n === to.who ? "on" : ""}" data-act="cvto" data-id="${s.id}" data-v="${esc(n)}">${esc(n)}</button>`).join("")}</div>` : "";
  const composer = !s.done && box
    ? `<div class="cv-comp">${toBar}${aiSayBox(s, key, { preset: { cmd: "people_say", args: { id: s.id }, limit: 400 }, log: key, words: { ph: s.kind === "OneOnOne" ? "Say it the way you would say it to him." : "Type what you'd say. The moves still work.", title: "Say it your way" } })}</div>`
    : "";
  return `<header class="cv-top"><div><div class="st-label">${place}</div><h2 class="cv-title">${cvT(s.title)}</h2>${T.lede ? `<p class="cv-lede">${cvT(T.lede)}</p>` : ""}</div>${big != null ? `<div class="cv-time"><span class="st-label">Talk time</span><span class="cv-n"><b class="st-outline">${esc(big)}</b><span class="cv-unit">${unit}</span></span><small>${sub}</small></div>` : ""}</header>
    <div class="cv-log"><div>${bits.join("")}${docs}${confirm}${ended}</div></div>${composer}`;
}
const bitsPrevBy = (lines, i) => (i > 0 ? lines[i - 1].by : -1);
const bitsPrevName = (lines, i) => (i > 0 ? lines[i - 1].name : "");
function cvAv(lead, who) {
  if (lead && lead.kind === "player" && (!who || plainText(lead.link) === who)) return face({ name: plainText(lead.link), id: lead.id }, 34);
  return `<span class="cv-ini">${esc(cvIni(who))}</span>`;
}

// One card under a line of the talk.
function cvCard(c) {
  switch (c.k) {
    case "logged": {
      const facts = (c.facts || []).map((f) => `<div><span>${esc(f.l)}</span><span>${cvT(f.t)}</span></div>`).join("");
      return `<div class="cv-prom"><span class="cv-badge">${esc(c.badge || "Logged · a promise")}</span>${c.counts ? `<small class="cv-counts">${esc(c.counts)}</small>` : ""}<b>${cvT(c.title)}</b>${c.chart ? cvChart(c.chart) : ""}${facts ? `<div class="cv-facts">${facts}</div>` : ""}${c.staff ? `<div class="cv-off"><span class="cv-ini">${esc(c.staff.ini || cvIni(c.staff.name))}</span><p><b>${esc(c.staff.name)}</b>${c.staff.role ? `, ${esc(c.staff.role)}` : ""}: ${cvT(c.staff.text)}</p></div>` : ""}</div>`;
    }
    case "check":
      return `<div class="cv-check ${c.ok === true ? "ok" : c.ok === false ? "bad" : ""}"><span class="cv-badge">${esc(c.badge || (c.ok ? "Checked: true" : "Checked: not true"))}</span><p>${cvT(c.text)}</p></div>`;
    case "read":
      return `<p class="cv-read">${CV_EYE}<span>${cvT(c.text)}</span></p>`;
    case "built":
      return `<div class="cv-prom cv-built"><span class="cv-badge">${esc(c.badge || "Built · an instruction")}</span><b>${cvT(c.title)}</b>${(c.facts || []).length ? `<div class="cv-facts">${c.facts.map((f) => `<div><span>${esc(f.l)}</span><span>${cvT(f.t)}</span></div>`).join("")}</div>` : ""}</div>`;
    case "staff":
      return `<div class="cv-off"><span class="cv-ini">${esc(c.ini || cvIni(c.name))}</span><p><b>${esc(c.name)}</b>${c.role ? `, ${esc(c.role)}` : ""}: ${cvT(c.text)}</p></div>`;
    default:
      return "";
  }
}

// A small chart under a promise or a deal.
function cvChart(c) {
  if (c.type === "stack") {
    const max = Math.max(1, ...c.bars.map((b) => (b.a || 0) + (b.b || 0)));
    return `<div class="cv-chart">${c.label ? `<div class="cv-h"><em>${esc(c.label)}</em>${c.legend ? `<span class="cv-legend">${c.legend.map((l, i) => `<i class="k${i}"></i>${esc(l)}`).join("")}</span>` : ""}</div>` : ""}<div class="cv-stack">${c.bars.map((b) => `<div class="cv-stackc"><small>${esc(b.top || "")}</small><span style="height:${Math.round(70 * ((b.a || 0) + (b.b || 0)) / max)}px"><i class="a" style="height:${Math.round(100 * (b.a || 0) / Math.max(1, (b.a || 0) + (b.b || 0)))}%"></i><i class="b"></i></span><em>${esc(b.l)}</em></div>`).join("")}</div>${c.note ? `<p class="cv-small">${esc(c.note)}</p>` : ""}</div>`;
  }
  const max = Math.max(0.01, ...c.bars.map((b) => b.v), c.line || 0) * 1.25;
  return `<div class="cv-chart">${c.label ? `<div class="cv-h cv-gold"><em>${esc(c.label)}</em></div>` : ""}<div class="cv-bars">${c.line != null ? `<s style="bottom:${Math.round(54 * c.line / max)}px"></s>` : ""}${c.bars.map((b) => `<div class="cv-bar ${b.now ? "now" : ""}"><small>${b.v != null ? Math.round(b.v * 100) + "%" : ""}</small><span style="height:${b.v != null ? Math.max(3, Math.round(54 * b.v / max)) : 54}px"></span><em>${esc(b.l)}</em></div>`).join("")}</div></div>`;
}

// What waits on your say-so (an offer, a request, a decision): N4's
// `talk_confirm`. Nothing here is done until you press the button.
function cvConfirm(s, T, b) {
  // On a talk the game plays (`talk.persona`) the game lists what waits on you each
  // time the page is drawn, so a reload keeps it; the box's own copy is for the rest.
  const list = T.persona ? T.confirm || [] : [...(T.confirm || []), ...((b && b.confirm) || [])];
  CV_CONFIRM[s.id] = list;
  if (!list.length) return "";
  return list.map((c, i) => `<div class="cv-confirm"><p>${cvT(c.text)}</p><div class="cv-acts"><button type="button" class="st-btn small glass" data-act="cvconfirm" data-i="${i}" data-id="${s.id}" data-yes="0">Not that</button><button type="button" class="st-btn small primary" data-act="cvconfirm" data-i="${i}" data-id="${s.id}" data-yes="1">Do it</button></div></div>`).join("");
}

// ---------- the right column ----------
function cvRight(s, pp, T, moves, v3, key, box) {
  const out = [];
  if (moves) out.push(moves);
  if (T.right && T.right.length) out.push(T.right.map((x) => cvSection({ ...x, key })).join(""));
  else if (pp) out.push(cvStand(s, pp, T));
  const ideas = T.ideas && T.ideas.length ? cvSection({ k: "ideas", label: "Ideas", note: "A click puts the words in your box to change", items: T.ideas, key }) : "";
  if (box && ideas) out.push(ideas);
  // While you talk (a v3 talk you've said a line in) the way out is a goodbye, typed
  // or this button: it ends the meeting with no cost, and what you didn't argue stands.
  // Before your first line you can still hand the meeting on.
  const talking = v3 && (s.lines || []).some((l) => l.by === 1);
  const end = cvEnd(s);
  const wrap = v3 && !end ? `<button type="button" class="st-btn small glass" data-act="cvdo" data-cmd="talk_wrap" data-args='${esc(JSON.stringify({ id: s.id }))}' title="Say your goodbyes. What you didn't argue stands; nothing is held against you for leaving.">Wrap it up</button>` : "";
  const hand = talking ? "" : `<button type="button" class="st-btn small glass ${ppHandCls(s)}" data-act="pphand" data-id="${s.id}" title="See what you give up and what you get first">${esc(ppHandLabel(s))}</button>`;
  if (!s.done) out.push(`<div class="cv-foot">${hand}${end}${wrap}</div>`);
  return out.join("");
}
// The decision a talk didn't settle: the meeting's own picks, in full, one click each.
function cvDecide(s) {
  const picks = s.picks.filter((p) => p.code !== 900 && p.code !== 969);
  if (!picks.length) return "";
  return `<div class="cv-sec cv-move"><div class="cv-h"><em>Still yours to decide</em><span>The talk is over</span></div>${picks.map((p) => `<button type="button" class="cv-opt cv-row" data-act="ppsay" data-id="${s.id}" data-code="${p.code}" title="${esc(p.hint || "")}"><span>${cvT(p.label)}</span></button>`).join("")}</div>`;
}
const cvEnd = (s) => {
  const end = s.picks.find((p) => p.code === 900 || p.code === 969);
  return end ? `<button type="button" class="st-btn small glass" data-act="ppsay" data-id="${s.id}" data-code="${end.code}">${esc(end.mv === "End" ? "End the talk" : end.mv)}</button>` : "";
};
// Where this talk stands, from the bars today's talk keeps.
function cvStand(s, pp, T) {
  const bm = s.bars && s.bars.mood, bt = s.bars && s.bars.trust;
  const mood = bm ? bm.words : pp.mood.words;
  const trust = bt ? bt.words : pp.trust ? pp.trust.words : null;
  const was = bm && Math.round((bm.now - bm.from) * 100) ? ppCap(talksMoodWords(bm.from)) : "";
  const feel = [trust ? ["Trust", ppCap(trust)] : null].filter(Boolean);
  return `<div class="cv-sec"><div class="cv-h">Where this talk stands</div><p class="cv-mood"><b>${esc(ppCap(mood))}</b>${was ? ` <small>was ${esc(was)}</small>` : ""}</p>${feel.length ? `<div class="cv-tiles cv-tiles3">${feel.map(([l, v]) => `<div><span>${l}</span><b>${esc(v)}</b></div>`).join("")}</div>` : ""}${pp.staffSays ? `<p class="cv-read">${CV_EYE}<span>Your read: ${esc(pp.staffSays)}</span></p>` : ""}</div>`;
}
// The mood word for a number, as the game says it (only the words the bar sends).
const talksMoodWords = (v) => (v > 0.5 ? "happy" : v > 0.15 ? "content" : v > -0.15 ? "level" : v > -0.5 ? "frustrated" : "angry");

// ---------- sections (both side columns) ----------
function cvSection(x) {
  const head = x.label ? `<div class="cv-h"><em>${esc(x.label)}</em>${x.note ? `<span>${esc(x.note)}</span>` : ""}</div>` : "";
  const sub = x.sub ? `<p class="cv-small cv-sub">${esc(x.sub)}</p>` : "";
  const wrap = (inner, cls = "") => `<div class="cv-sec ${cls}">${head}${sub}${inner}</div>`;
  switch (x.k) {
    case "text":
      return wrap(`${x.head ? `<p class="cv-big">${cvT(x.head)}</p>` : ""}${(x.paras || [x.text]).filter(Boolean).map((p) => `<p>${cvT(p)}</p>`).join("")}${x.small ? `<p class="cv-small">${cvT(x.small)}</p>` : ""}${x.pair ? `<div class="cv-tiles cv-tiles2">${x.pair.map((p) => `<div><span>${esc(p.l)}</span><b>${esc(p.t)}</b></div>`).join("")}</div>` : ""}`);
    case "entries":
      return wrap(`${x.head ? `<p class="cv-big">${cvT(x.head)}</p>` : ""}${(x.items || []).map((e) => `<p class="cv-entry"><b>${esc(e.date || "")}</b> ${cvT(e.text)}</p>`).join("")}`);
    case "tiles":
      return wrap(`<div class="cv-tiles">${(x.items || []).map((t) => `<div><b>${esc(t.n)}</b><span>${esc(t.l)}</span></div>`).join("")}</div>${x.text ? `<p class="cv-small">${cvT(x.text)}</p>` : ""}`);
    case "chips":
      return wrap(`<div class="cv-chips">${(x.items || []).map((c) => `<span class="cv-mind ${c.hot ? "hot" : ""}">${c.lead ? `<b>${cvT(c.lead)}</b> ` : ""}${cvT(c.rest || "")}</span>`).join("")}</div>`);
    case "bars":
      return wrap(`<div class="cv-hbars">${(x.items || []).map((b) => `<div><span>${esc(b.l)}</span><i><u style="width:${cvPct(b.pct)}%"></u></i><b>${esc(b.n ?? "")}</b></div>`).join("")}</div>`);
    case "split":
      return wrap(`<div class="cv-split"><span style="width:${cvPct(x.pct)}%">${esc(x.a)} ${Math.round(100 * x.pct)}%</span><span class="r">${esc(x.b)} ${Math.round(100 * (1 - x.pct))}%</span></div>`);
    case "clips":
      return wrap(`${(x.items || []).map((c) => `<button type="button" class="cv-clip" ${c.go ? `data-go="${esc(c.go)}" data-id="${esc(c.id ?? "")}"` : ""}><span class="cv-mini"></span><span><b>${esc(c.title)}</b><small>${esc(c.sub || "")}</small></span><i>▶</i></button>`).join("")}`);
    case "word":
      return `<div class="cv-word">${(x.items || []).map((w) => `<span>${cvT(w)}</span>`).join("")}</div>`;
    case "stand":
      return wrap(`<p class="cv-mood"><b>${esc(x.mood)}</b>${x.was ? ` <small>was ${esc(x.was)}</small>` : ""}</p><div class="cv-tiles cv-tiles3">${(x.feel || []).map((f) => `<div><span>${esc(f.l)}</span><b>${esc(f.t)}</b></div>`).join("")}</div>${x.read ? `<p class="cv-read">${CV_EYE}<span>${cvT(x.read)}</span></p>` : ""}`);
    case "asks":
      return wrap(`${(x.items || []).map((a) => `<div class="cv-ask"><div class="m"><b>${cvT(a.text)}</b>${a.pct != null ? `<i class="t"><u style="width:${cvPct(a.pct)}%"></u></i><em class="n">${Math.round(100 * a.pct)}%</em>` : ""}</div>${a.sub ? `<small>${esc(a.sub)}</small>` : ""}</div>`).join("")}`);
    case "learned":
      return wrap(`${(x.items || []).map((l) => `<div class="cv-learn"><span class="cv-ini">${esc(l.ini || cvIni(l.name))}</span><div><b>${cvT(l.text)}</b><small>${cvT(l.sub || "")}</small>${l.act ? `<button type="button" class="cv-pill" data-act="cvdo" data-cmd="${esc(l.act.cmd)}" data-args='${esc(JSON.stringify(l.act.args || {}))}'>${esc(l.act.text)}</button>` : ""}</div></div>`).join("")}`);
    case "rank":
      return wrap(`${(x.items || []).map((r, i) => `<div class="cv-rank ${r.on ? "on" : ""}"><b class="n">${i + 1}</b><span><b>${cvT(r.name)}</b><small>${esc(r.sub || "")}</small></span>${r.tag ? `<em class="cv-tag">${esc(r.tag)}</em>` : ""}</div>`).join("")}${x.text ? `<p class="cv-small">${cvT(x.text)}</p>` : ""}`);
    case "facts":
      return wrap(`${(x.items || []).map((f) => `<p>${cvT(f)}</p>`).join("")}`);
    case "ideas":
      return wrap(`${(x.items || []).map((t) => `<button type="button" class="cv-idea" data-act="cvidea" data-key="${esc(x.key || "")}" data-text="${esc(t)}">${esc(t)}</button>`).join("")}`);
    case "deal":
      return cvDeal(x);
    case "promises":
      return cvPromises(x);
    default:
      return "";
  }
}

// What you can promise (board 5): every position has its own list, judged on
// the play log. The game sends the positions and, for the one picked, its cards.
function cvPromises(x) {
  const st = keep("cvp", {});
  const pick = (x.tabs || []).includes(st.t) ? st.t : x.pick;
  const cards = (x.cards || {})[pick] || x.cards_default || [];
  const chips = (x.tabs || []).map((t) => `<button type="button" class="cv-chip ${t === pick ? "on" : ""}" data-act="cvpos" data-v="${esc(t)}">${esc(t)}</button>`).join("");
  const cs = cards.map((c) => `<div class="cv-pcard"><div class="cv-whoh"><b>${esc(c.title)}</b>${c.tag ? `<em class="cv-tag gold">${esc(c.tag)}</em>` : ""}</div><div class="cv-facts">${(c.facts || []).map((f) => `<div><span>${esc(f.l)}</span><span>${cvT(f.t)}</span></div>`).join("")}</div></div>`).join("");
  return `<div class="cv-sec"><div class="cv-h cv-bigh"><em>${esc(x.label || "What you can promise")}</em></div>${x.sub ? `<p class="cv-small">${esc(x.sub)}</p>` : ""}<div class="cv-chipbar">${chips}</div>${cs}${x.every ? `<div class="cv-h"><em>Every position, too</em></div><p class="cv-small">${esc(x.every)}</p>` : ""}${x.talkonly ? `<div class="cv-off"><em class="cv-tag">${esc(x.talkonly.badge || "Talk only")}</em><p>${cvT(x.talkonly.text)}</p></div>` : ""}</div>`;
}

// The deal on the table (board 3): his counter beside your offer, row by row,
// built by the game from what you said; the cap chart; what sending it does.
function cvDeal(x) {
  const rows = (x.rows || []).map((r) => `<tr class="${r.hot ? "hot" : ""}"><td>${esc(r.l)}</td><td>${esc(r.a ?? "")}</td><td>${esc(r.b ?? "")}</td></tr>`).join("");
  const send = x.send ? `<div class="cv-confirm"><p>${cvT(x.send.text)}</p><div class="cv-acts">${x.send.change ? `<button type="button" class="st-btn small glass" data-act="cvidea" data-key="${esc(x.key || "")}" data-text="${esc(x.send.change)}">Change it</button>` : ""}<button type="button" class="st-btn small primary" data-act="cvdo" data-cmd="${esc(x.send.cmd)}" data-args='${esc(JSON.stringify(x.send.args || {}))}'>${esc(x.send.button || "Send the offer")}</button></div></div>` : "";
  return `<div class="cv-sec cv-deal"><div class="cv-h cv-dealh"><b>${esc(x.title || "The deal on the table")}</b><span>${esc(x.note || "Built from your words")}</span></div>
    <table class="cv-dt"><thead><tr><th></th><th>${esc(x.cols ? x.cols[0] : "Your offer")}</th><th>${esc(x.cols ? x.cols[1] : "His counter")}</th></tr></thead><tbody>${rows}</tbody></table>
    ${x.text ? `<p class="cv-small">${cvT(x.text)}</p>` : ""}${x.chart ? cvChart(x.chart) : ""}${send}</div>`;
}

// ---------- your move (no key, or a talk that is not played yet) ----------
// Moves and tones and the subject you turn the talk to, as the first
// playable has them; the builder below them comes from `talk.moves`.
async function cvMoves(s, st, T) {
  if (s.done) return "";
  const talk = s.kind === "OneOnOne";
  const isTalk = (c) => talk && c >= 100 && c < 190;
  let turn = talk && st.turn && st.turn !== s.subject ? st.turn : null;
  let look = null;
  if (turn) {
    look = await api("people_subject_look", { id: s.id, subject: turn }).catch(() => null);
    if (!look) { turn = null; st.turn = null; }
  }
  const tpicks = look ? look.picks : s.picks;
  if (look && st.pick != null && !tpicks.some((p) => p.code === st.pick || (isTalk(p.code) && isTalk(st.pick) && Math.floor((p.code - 100) / 10) === Math.floor((st.pick - 100) / 10)))) st.pick = null;
  const chosen = st.pick != null ? tpicks.find((p) => p.code === st.pick) || (look ? tpicks.find((p) => isTalk(p.code) && isTalk(st.pick) && Math.floor((p.code - 100) / 10) === Math.floor((st.pick - 100) / 10)) : null) : null;
  const offered = tpicks.filter((p) => p.code !== 900 && p.code !== 969);
  const twice = (mv) => offered.filter((p) => p.mv === mv).length > 1;
  const chipText = (p) => {
    if (!twice(p.mv)) return p.mv;
    const w = plainText(p.label).replace(/^["“]|["”]$/g, "").split(/\s+/);
    return w.length > 4 ? `${p.mv} · ${w.slice(0, 4).join(" ").replace(/[.,!?;:]+$/, "")}…` : `${p.mv} · ${w.join(" ")}`;
  };
  const chips = offered.map((p) => `<button type="button" class="cv-opt ${chosen && chosen.code === p.code ? "on" : ""}" data-act="pppick" data-id="${s.id}" data-code="${p.code}" data-tone="${esc(p.tone)}" title="${esc(plainText(p.label))}"><span>${esc(chipText(p))}</span></button>`).join("");
  const tone = st.tone || (chosen && chosen.tone);
  const tones = talk ? `<div class="cv-pick"><span>Tone</span><div class="cv-segw">${PP_TONES.map(([k, t]) => `<button type="button" data-act="pptone" data-id="${s.id}" data-v="${k}" aria-pressed="${tone === k}">${t}</button>`).join("")}</div></div>` : "";
  const subjects = talk && s.subjects && s.subjects.length ? `<div class="cv-sec cv-about"><div class="cv-h">Ask about</div>${s.subjects.map((x) => `<button type="button" class="cv-opt ${x.key === (turn || s.subject) ? "on" : ""}" data-act="tksubject" data-id="${s.id}" data-s="${x.key}"><span>${esc(x.name)}</span></button>`).join("")}</div>` : "";
  let say = "";
  if (chosen) {
    const code = isTalk(chosen.code) ? chosen.code - (chosen.code % 10) + PP_TONE_CODE[tone || chosen.tone] : chosen.code;
    const expect = talk && !turn ? await api("people_expect", { id: s.id, code }).catch(() => null) : null;
    say = `<div class="cv-say"><p class="cv-q">${cvT(chosen.label)}</p>${expect ? `<p class="cv-small">${cvT(expect.who)} · ${esc(expect.sure)}: ${esc(expect.what)}</p>` : chosen.hint ? `<p class="cv-small">${esc(chosen.hint)}</p>` : ""}
      <button type="button" class="st-btn primary cv-go" data-act="${turn ? "tkturn" : "ppsay"}" data-id="${s.id}" data-s="${esc(turn || "")}" data-code="${code}">${chosen.code === 900 ? "End it" : "Say it"}</button></div>`;
  } else if (turn) {
    const name = (s.subjects.find((x) => x.key === turn) || {}).name || "that";
    say = `<div class="cv-say"><p class="cv-small">${look.asks_first ? "Nothing is said yet. He would ask you something first." : "Nothing is said yet. Pick a move and a tone for it."}</p>
      <button type="button" class="st-btn glass cv-go" data-act="tkturn" data-id="${s.id}" data-s="${esc(turn)}" data-code="">${look.asks_first ? `Turn the talk to ${esc(name.toLowerCase())}` : "Just bring it up"}</button></div>`;
  }
  const builder = T.moves ? cvBuilder(s, T.moves) : "";
  const nokey = aiBoxPeek(s) ? "" : '<span class="cv-off-note">Live talks off</span>';
  const tabs = cvMoveTabs(s, T);
  if (!chips && !subjects && !builder && !say && !tabs) return "";
  return `<div class="cv-sec cv-move"><div class="cv-h"><em>Your move</em>${nokey}</div>${tabs}${chips || !tabs ? `<div class="cv-opts">${chips || '<p class="cv-small">Waiting on him.</p>'}</div>` : ""}${chips ? tones : ""}${say}${builder}</div>${subjects}`;
}

// The pick lists (board 6): Ask, Tell, Offer or promise, Ask him to. The game
// sends each list's rows (an "Ask about" row with its tag) or a builder.
function cvMoveTabs(s, T) {
  const m = T.moves;
  if (!m || !m.tabs || !m.tabs.length) return "";
  const st = keep("cvm:" + s.id, {});
  const k = m.tabs.some((x) => x.k === st.k) ? st.k : m.tabs[0].k;
  const tab = m.tabs.find((x) => x.k === k);
  const bar = `<div class="cv-seg" role="tablist">${m.tabs.map((x) => `<button type="button" role="tab" data-act="cvtab" data-id="${s.id}" data-v="${esc(x.k)}" aria-pressed="${x.k === k}">${esc(x.name)}</button>`).join("")}</div>`;
  const rows = (tab.rows || []).map((r) => `<button type="button" class="cv-opt cv-row" data-act="cvdo" data-cmd="${esc(r.cmd)}" data-args='${esc(JSON.stringify(r.args || {}))}'><span>${esc(r.text)}</span>${r.tag ? `<em class="cv-tag">${esc(r.tag)}</em>` : ""}</button>`).join("");
  return `${bar}${tab.builder ? cvBuilder(s, { builder: tab.builder }) : ""}${rows ? `<div class="cv-sec"><div class="cv-h"><em>${esc(tab.label || "Ask about")}</em></div>${rows}</div>` : ""}`;
}

// The pick lists' builder (a promise, an offer, an ask): fields with their
// choices, what you would say, what it is judged on, one button.
function cvBuilder(s, m) {
  const b = m.builder;
  if (!b) return "";
  const st = keep("cvb:" + s.id, {});
  const fields = (b.fields || []).map((f) => `<label class="cv-f ${f.half ? "half" : ""}"><span>${esc(f.l)}</span><select data-act="cvfield" data-id="${s.id}" data-f="${esc(f.k)}">${f.opts.map((o) => `<option value="${esc(o.v)}" ${String(st[f.k] ?? f.pick ?? f.opts[0].v) === String(o.v) ? "selected" : ""}>${esc(o.t)}</option>`).join("")}</select></label>`).join("");
  return `<div class="cv-build"><div class="cv-h">${esc(b.title || "Build it")}</div><div class="cv-fields">${fields}</div>${b.say ? `<div class="cv-h">You'd say</div><p class="cv-q">${cvT(b.say)}</p>` : ""}${b.judged ? `<div class="cv-h">Judged on · plan setting</div><p class="cv-small">${cvT(b.judged)}</p>` : ""}${b.go ? `<button type="button" class="st-btn primary cv-go" data-act="cvbuild" data-id="${s.id}" data-cmd="${esc(b.go.cmd)}" data-args='${esc(JSON.stringify(b.go.args || {}))}'>${esc(b.go.text || "Say it")}</button>` : ""}</div>`;
}

// ---------- what its buttons do ----------
Object.assign(CORE_MORE, {
  cvto: (el) => { keep("cvto:" + el.dataset.id, {}).who = el.dataset.v; refresh(el); },
  cvpos: (el) => { keep("cvp", {}).t = el.dataset.v; refresh(el); },
  cvtab: (el) => { keep("cvm:" + el.dataset.id, {}).k = el.dataset.v; refresh(el); },
  cvside: (el) => { keep("cvside", { t: "Now" }).t = el.dataset.v; refresh(el); },
  // A button the game gave a command for: run it, then draw the page again.
  cvdo: async (el) => {
    el.disabled = true;
    try {
      const r = await api(el.dataset.cmd, JSON.parse(el.dataset.args || "{}"));
      if (r && r.done) S.status = await api("status");
      refresh(el);
    } catch (e) { toast(e.message); el.disabled = false; }
  },
  // A click on an idea puts the words in the box, to change.
  cvidea: (el) => {
    const b = aiState.boxes[el.dataset.key];
    const f = b && document.querySelector(`[data-ai-in][data-ai="${CSS.escape(el.dataset.key)}"]`);
    if (!b || !f) return;
    b.text = el.dataset.text;
    f.value = b.text;
    f.dispatchEvent(new Event("input", { bubbles: true }));
    f.focus();
  },
  cvfield: (el) => { keep("cvb:" + el.dataset.id, {})[el.dataset.f] = el.value; },
  cvbuild: async (el) => {
    el.disabled = true;
    try {
      const args = { ...JSON.parse(el.dataset.args || "{}"), ...keep("cvb:" + el.dataset.id, {}) };
      const r = await api(el.dataset.cmd, args);
      if (r && r.done) S.status = await api("status");
      refresh(el);
    } catch (e) { toast(e.message); el.disabled = false; }
  },
  // What waits on your say-so: N4's talk_confirm (add yes:false to drop it).
  cvconfirm: async (el) => {
    const id = Number(el.dataset.id);
    const b = aiState.boxes["scene:" + id];
    const c = (CV_CONFIRM[id] || [])[Number(el.dataset.i)];
    if (!c) return;
    el.disabled = true;
    try {
      const args = { ...c.args, yes: el.dataset.yes === "1" };
      await api(c.cmd, args);
      if (b && b.confirm) b.confirm = b.confirm.filter((x) => x !== c);
      if (c.keep) c.keep.dropped = true;
      refresh(el);
    } catch (e) { toast(e.message); el.disabled = false; }
  },
});


// A pick list's choice is kept as you change it (selects fire "change", not a click).
document.addEventListener("change", (ev) => {
  const f = ev.target.closest && ev.target.closest('[data-act="cvfield"]');
  if (f) keep("cvb:" + f.dataset.id, {})[f.dataset.f] = f.value;
});

// ---------- inbox: people asking for you (board 4) ----------
// Calls and texts that come to you (lane N1's "coming"): the list, one
// person's thread, and what the urgent one needs from you. The game sends
//   talk_asking {open?} -> {
//     items:  [{id, ini, name, sub, text, when, tag, urgent}],
//     open:   {id, ini, name, sub, quiet:{tag, text}, days:[{day, lines:[{by, text, time, note, heard,
//                 tell, kind}]}], reply:{label, ph, note, send:{cmd,args}, call:{cmd,args}}},
//     stops:  {tag, kind, name, ini, sub, text, know:[{head?, text}], options:[{text, primary, cmd, args}],
//                 warn, label, why} }
// A line's `kind` is "grapevine" for what reached him from someone else.
screens.asking = async (a) => {
  const colors = (S.status && S.status.mine && S.status.mine.colors) || ["#2b3140", "#11141b"];
  setColors(colors);
  let x;
  try {
    x = await api("talk_asking", { open: a.open ?? null });
  } catch (e) {
    return `<div class="pad"><div class="h1">Asking for you</div><p class="sub">${esc(e.message)}</p><button class="st-btn glass" data-nav="inbox">Back to Inbox</button></div>`;
  }
  title("Asking for you");
  const list = (x.items || []).map((i) => `<button type="button" class="cv-ask-row ${i.urgent ? "hot" : ""} ${x.open && x.open.id === i.id ? "on" : ""}" data-act="cvopen" data-id="${esc(i.id)}"><span class="cv-ini">${esc(i.ini || cvIni(i.name))}</span><span><b>${esc(i.name)}</b><small>${esc(i.sub || "")}</small><span class="cv-ask-t">${cvT(i.text)}</span></span><em class="cv-ask-w">${i.tag ? `<i class="cv-tag ${i.urgent ? "hot" : ""}">${esc(i.tag)}</i>` : ""}<small>${esc(i.when || "")}</small></em></button>`).join("");
  const left = `<div class="cv-h cv-bigh"><em>Asking for you</em></div><p class="cv-small">Urgent ones stop Continue. The rest wait, and remember how long you took.</p><div class="cv-asklist">${list || '<p class="cv-small">Nobody is waiting on you.</p>'}</div>`;
  const o = x.open;
  let mid = `<p class="cv-narr cv-quiet">Pick someone to answer.</p>`;
  if (o) {
    const days = (o.days || []).map((d) => `<div class="cv-day">${esc(d.day)}</div>${d.lines.map((l) => {
      if (l.kind === "grapevine") return `<div class="cv-vine"><em class="cv-tag gold">Grapevine</em><p>${cvT(l.text)}</p></div>`;
      if (l.by === 1) return `<div class="cv-you"><p>${cvT(l.text)}</p><div class="cv-heard"><span>${esc(l.time || "")}</span>${l.heard ? `<div class="ai-ha"><span class="ai-lb">Heard as</span><span>${esc(l.heard)}</span></div>` : ""}</div></div>`;
      return `<div class="cv-him"><span class="cv-av"><span class="cv-ini">${esc(o.ini || cvIni(o.name))}</span></span><div class="cv-b"><p>${cvT(l.text)}</p><small class="cv-time-s">${esc(l.time || "")}</small>${l.tell ? `<span class="cv-tell">${CV_EYE}<i>${esc(l.tell)}</i></span>` : ""}${l.note ? `<p class="cv-read">${CV_EYE}<span>${cvT(l.note)}</span></p>` : ""}</div></div>`;
    }).join("")}`).join("");
    const r = o.reply || {};
    const quiet = o.quiet ? `<div class="cv-quiet-tag"><em class="cv-tag">${esc(o.quiet.tag)}</em><small>${esc(o.quiet.text)}</small></div>` : "";
    const key = "ask:" + o.id;
    const box = aiFlag({ say: r.send ? { cmd: r.send.cmd, args: r.send.args, limit: 400 } : null }, "say", r.send ? null : { cmd: "people_say", args: {}, limit: 400 });
    mid = `<header class="cv-top"><div class="cv-ask-head"><span class="cv-ini big">${esc(o.ini || cvIni(o.name))}</span><div><h2 class="cv-title">${esc(o.name)}</h2><p class="cv-lede">${esc(o.sub || "")}</p></div></div>${quiet}</header>
      <div class="cv-log"><div>${days}</div></div>
      <div class="cv-comp">${box ? aiSayBox({ say: box }, key, { log: key, words: { title: r.label || "Text " + o.name.split(" ")[0], ph: r.ph || "Write it the way you'd text a recruit you want.", btn: "Send" } }) : ""}${r.note ? `<div class="cv-b3"><small>${esc(r.note)}</small>${r.call ? `<button type="button" class="st-btn small glass" data-act="cvdo" data-cmd="${esc(r.call.cmd)}" data-args='${esc(JSON.stringify(r.call.args || {}))}'>Call him instead</button>` : ""}</div>` : ""}</div>`;
  }
  const st = x.stops;
  const right = st
    ? `<div class="cv-stopline"><em class="cv-tag hot">${esc(st.tag || "Stops Continue")}</em><small>${esc(st.kind || "")}</small></div>
       <div class="cv-who big"><span class="cv-ini big">${esc(st.ini || cvIni(st.name))}</span><div><h2 class="cv-title">${esc(st.name)}</h2><p class="cv-small">${esc(st.sub || "")}</p></div></div>
       <p class="cv-bigq">${cvT(st.text)}</p>
       ${(st.know || []).length ? `<div class="cv-sec"><div class="cv-h"><em>What you know</em></div>${st.know.map((k) => `<p>${cvT(k.text)}</p>`).join("")}</div>` : ""}
       ${(st.options || []).map((op) => `<button type="button" class="cv-bigbtn ${op.primary ? "pri" : ""}" data-act="cvdo" data-cmd="${esc(op.cmd)}" data-args='${esc(JSON.stringify(op.args || {}))}'>${esc(op.text)}</button>`).join("")}
       ${st.warn ? `<p class="cv-warnbox">${cvT(st.warn)}</p>` : ""}
       ${st.why ? `<div class="cv-sec"><div class="cv-h"><em>What stops Continue</em></div><p class="cv-small">${esc(st.why)}</p></div>` : ""}`
    : `<div class="cv-h"><em>What stops Continue</em></div><p class="cv-small">A family crisis, a trade request, an agent's deadline, your boss asking for a meeting. Everything else waits in this list.</p>`;
  return `<div class="cv st-room" style="--accent:${esc(colors[0])};--accent-2:${esc(colors[1] || colors[0])}"><i></i><i></i><i></i>
    <div class="cv-grid cv-grid-ask"><section class="cv-col st-glass cv-left">${left}</section><section class="cv-col st-glass cv-mid">${mid}</section><aside class="cv-col st-glass cv-right">${right}</aside></div></div>`;
};
CRUMBS.asking = "Asking for you";
GOTO.asking = () => [["Inbox", "inbox"], ["People", "people"], ["Calendar", "calendar"]];
Object.assign(CORE_MORE, {
  cvopen: (el) => { go("asking", { open: Number(el.dataset.id) || el.dataset.id }); },
});

// The page fills the window under whatever sits above it (the bar of meetings
// that stop Continue), so each column scrolls by itself.
function cvFit() {
  for (const el of document.querySelectorAll(".cv.st-room")) {
    const top = el.getBoundingClientRect().top;
    el.style.height = window.innerWidth > 1100 ? `${Math.max(560, window.innerHeight - top)}px` : "";
  }
}
window.addEventListener("resize", () => { if (typeof cvFit === "function") cvFit(); });
