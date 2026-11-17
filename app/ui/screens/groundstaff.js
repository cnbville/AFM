/* Your staff's ground (lane S7; docs/design/ground.md part K, board 7b, GR68,
 * GR76, GR79). The head coach's page for his men: when another school's search
 * reaches one of them, the warning with his face, the days to the early signing
 * day, what pulls him each way and your read of him, his line in the hallway,
 * your counter in his office (a raise, a title or a promise, each with how far
 * it reaches and what it costs you), who takes his area if he goes, what leaves
 * with him on the map and the commits who would waver. Every day else, the same
 * page is the staff's ground: each man's areas and people, his mood and
 * why, his record with you, the areas nobody works and the coordinator's
 * letter, the ride-alongs, the men who left. A rail of all your men runs along
 * the bottom.
 *
 *   groundstaff   with {man} (a link's {id}): that man (his warning if he is courted, else
 *                 his ground). Without: the first man courted, else the first.
 *
 * It reads `ground_staff`, `ground_carousel` and `ground_staff_man` (lane
 * G10) and writes with `ground_counter`, `ground_let_go`, `ground_waver_call`,
 * `ground_staff_areas`, `ground_staff_ride` and `ground_staff_ride_end`. Never
 * a hidden number: moods are words with their reasons, reach is words. Every
 * class is gs- so the app's global names never restyle it. */
"use strict";

I.gdstaff = '<circle cx="9" cy="8" r="3"/><path d="M3.5 19c.5-3.5 2.7-5.5 5.5-5.5s5 2 5.5 5.5"/><circle cx="17" cy="9.5" r="2.3"/><path d="M16 14c2.4 0 4 1.5 4.5 4.5"/>';
navAdd({ go: "groundstaff", label: "Your men", icon: "gdstaff", sec: "ground", after: "groundreport", seats: ["college"], show: () => typeof foIsHead !== "function" || foIsHead(), badge: () => GS.n });
CRUMBS.groundstaff = "Your men";

const GS = { faces: [], said: "", n: 0 };
const gsCap = (s) => (s ? s.charAt(0).toUpperCase() + s.slice(1) : s);
const gsDate = (iso, opt) => (iso ? new Date(iso + "T12:00:00").toLocaleDateString("en-US", opt) : "");
const gsIni = (name, size = 30, tone = "") => `<span class="gs-ini ${tone}" style="--s:${size}px">${esc((name || "?").split(/[\s,]+/).filter(Boolean).map((x) => x[0]).slice(0, 2).join("").toUpperCase())}</span>`;
const gsPips = (n, of = 4, cls = "") => `<span class="gs-pips ${cls}" aria-hidden="true">${Array.from({ length: of }, (_, i) => `<i class="${i < n ? "on" : ""}"></i>`).join("")}</span>`;
const GS_LV = { Stranger: 0, Known: 1, Trusted: 2, Home: 3 };
const gsLv = (name) => `<span class="gs-lv">${esc(name)} ${gsPips(GS_LV[name] || 0, 3)}</span>`;
const gsMood = (w) => ({ settled: "good", content: "good", restless: "amb", "listening to other jobs": "amb", unhappy: "amb" }[w] || "");

// A man's head: his drawn picture when the game has one, else his initials
// (the drawn head is made after the page draws and kept in the save's folder).
function gsFace(f, name, size, cls = "") {
  if (!f) return gsIni(name, size, cls);
  if (f.id != null) GS.faces.push(f);
  const pic = f.drawn ? `<img src="${f.drawn}" alt="${esc(name)}">` : "";
  return `<span class="gs-face ${cls}" style="--s:${size}px" data-gsface="${f.id}">${pic}${f.drawn ? "" : gsIni(name, size)}</span>`;
}

const GS_COUNTER = {
  raise: ["Money", "A raise from your staff money"],
  title: ["A title", "Assistant head coach"],
  promise: ["A promise", "The next coordinator job"],
};
const gsReachPips = (words) => ({ "likely to keep him": 4, "better than even": 3, "a coin flip": 2, "a long shot": 1 }[words] || 0);

screens.groundstaff = async (args) => {
  let st;
  try { st = await api("ground_staff", {}); } catch (e) {
    title("Your men", "");
    return `<div class="pad sl gs"><section class="gs-pb"><div class="gs-lab">Your men</div><p class="gs-empty">${esc(e.message || String(e))}</p></section></div>`;
  }
  GS.faces = [];
  GS.n = st.courted.length;
  // A link carries the man as `id` (the app's link arguments), a call as `man`.
  const want = args && (args.man != null || args.id != null) ? Number(args.man != null ? args.man : args.id) : null;
  const car = await api("ground_carousel", want != null ? { man: want } : {}).catch(() => null);
  const warn = car && car.courted && car.courted.state === "waiting" ? car : null;
  const sel = want != null ? want : warn ? warn.courted.man : st.men.length ? st.men[0].man : null;
  const showWarn = warn && warn.courted.man === sel;
  const page = !showWarn && sel != null ? await api("ground_staff_man", { man: sel }).catch(() => null) : null;
  title("Your men", showWarn ? `${warn.courted.name} may go` : "Your staff's ground");

  const said = GS.said ? `<div class="gs-said"><b>${esc(GS.said)}</b></div>` : "";
  GS.said = "";
  const empty = st.empty.length;
  const top = `<div class="gs-top"><div><h1>${esc(gsDate(st.today, { weekday: "long", month: "long", day: "numeric", year: "numeric" }))}</h1><small>${showWarn ? "The carousel" : "Your staff's ground"} · the February meeting is ${esc(gsDate(st.february_meeting, { month: "long", day: "numeric" }))}</small></div><div class="gs-sp"></div>
    <div class="gs-stat"><span>Your men</span><b>${st.men.length} on the road</b></div>
    <div class="gs-stat"><span>Warnings</span><b class="${st.courted.length ? "amb" : ""}">${st.courted.length ? (st.courted.length === 1 ? "1 man may go" : `${st.courted.length} men may go`) : "None"}</b></div>
    <div class="gs-stat"><span>Areas with nobody</span><b class="${empty ? "amb" : ""}">${empty || "None"}</b></div>
    <button class="st-btn primary" data-go="staff">Open the staff room</button></div>`;

  const body = showWarn ? gsWarning(warn) : page ? gsMan(page, st) : '<section class="gs-pb"><p class="gs-empty">You have no men on the road yet. Hire a position coach and the coordinator gives him his ground.</p></section>';
  return `<div class="pad sl gs">${said}${top}${body}${gsRail(st, sel)}</div>`;
};

// ---- the warning (GR76) ----
function gsWarning(x) {
  const c = x.courted;
  const first = c.name.split(" ").slice(-1)[0];
  const days = c.days_to_early_signing;
  const es = days != null ? new Date(new Date(x.today + "T12:00:00").getTime() + days * 864e5).toISOString().slice(0, 10) : null;
  const step = (cls, nm, sub) => `<div class="${cls}"><i></i><b>${esc(nm)}</b><span>${esc(sub)}</span></div>`;
  const line = `<section class="gs-pb gs-warn"><div class="gs-live">A warning · ${esc(gsDate(c.made, { month: "short", day: "numeric" }))}</div>
    <div class="gs-big">${gsFace(c.face, c.name, 180, "big")}<div class="gs-cap"><span class="gs-lab">${esc(gsCap(c.role))}</span><b>${esc(c.name)}</b></div></div>
    <h2>${esc(first)} may go</h2>
    <p class="gs-body">${esc(c.to)} wants him as ${esc(c.job)}.${c.why ? ` ${esc(gsCap(c.why))}.` : ""}</p>
    <div class="gs-cd">${step("now", "Offer", gsDate(c.made, { month: "short", day: "numeric" }))}${step("", "Your counter", `by ${gsDate(c.until, { month: "short", day: "numeric" })}`)}${es ? step("end", "Early signing day", gsDate(es, { month: "short", day: "numeric" })) : ""}</div></section>`;

  // what is pulling him each way, and your read of him
  const stay = c.pulls_stay || [], away = c.pulls_away || [];
  const share = Math.max(8, Math.min(92, Math.round((100 * (stay.length + 0.5)) / (stay.length + away.length + 1))));
  const pulls = `<section class="gs-pb"><div class="gs-row"><div class="gs-lab">What's pulling him</div><span class="gs-tag ${gsMood(c.mood)}">${esc(gsCap(c.mood))}</span></div>
    <div class="gs-beam" role="img" aria-label="${stay.length} reasons to stay, ${away.length} to go"><span style="width:${share}%"></span><i style="left:${share}%"></i></div>
    <div class="gs-ends"><span>Stays</span><span>Goes</span></div>
    <div class="gs-pull"><div>${stay.map((t) => `<p><em>+</em>${esc(gsCap(t))}</p>`).join("") || '<p class="gs-dim">Nothing holds him that you can name.</p>'}</div><div>${away.map((t) => `<p><em class="amb">&minus;</em>${esc(gsCap(t))}</p>`).join("")}</div></div>
    ${c.read ? `<div class="gs-read"><span class="gs-chip">Your read</span><p>${esc(c.read)}</p></div>` : ""}</section>`;

  // his office: the line and your counters
  const counters = c.counters.map((k) => {
    const [tag, name] = GS_COUNTER[k.kind];
    const chosen = c.counter === k.kind;
    const pips = k.reach ? gsReachPips(k.reach) : 0;
    const barred = !k.reach || /already/.test(k.side_effect || "");
    return `<div class="gs-cn${chosen ? " pick" : ""}${barred ? " bar" : ""}"><div class="gs-cnh"><span class="gs-chip">${tag}</span><b>${name}</b></div><p>${esc(gsCap(k.side_effect || ""))}.</p>
      <div class="gs-cnf"><span class="gs-ef">${gsPips(pips, 4, pips >= 3 ? "g" : "")}<small>${k.reach ? esc(gsCap(k.reach)) : "Not for him"}</small></span>
      <button class="st-btn glass" data-act="gscounter" data-man="${c.man}" data-kind="${k.kind}"${barred || c.counter ? " disabled" : ""}>${chosen ? "Offered" : "Counter"}</button></div></div>`;
  }).join("");
  const hall = `<section class="gs-pb"><div class="gs-row"><div class="gs-lab">His office</div><span class="gs-dim sm">one counter, his answer at once</span></div>
    <div class="gs-hall">${gsFace(c.face, c.name, 38)}<div><small>${esc(first)}, in the hallway</small><p class="gs-q">${esc(c.line)}</p></div></div>${counters}
    <div class="gs-row gs-end"><span class="gs-dim sm">No counter by ${esc(gsDate(c.until, { month: "long", day: "numeric" }))} and the search hires him.</span><button class="st-btn glass" data-act="gsletgo" data-man="${c.man}">Let him go now</button></div></section>`;

  // who takes his area
  const takers = x.takers.length ? x.takers.map((t) => {
    const teach = t.teaching ? t.teaching : null;
    return `<div class="gs-tk${t.barred ? " barred" : ""}">${gsIni(t.name, 34)}<div><b>${esc(t.name)}${t.barred ? ' <span class="gs-chip amb">Barred</span>' : ""}</b><small>${esc(t.now || "")}${t.barred ? ` ${esc(t.barred)}` : ""}</small>
      <div class="gs-tkm"><span>Ground</span><b>${esc(t.ground)}</b>${t.tag === "teaching" ? '<span class="gs-chip">Best teacher</span>' : ""}${t.tag === "ground" ? '<span class="gs-chip">Knows the ground</span>' : ""}</div>${(t.signs || []).slice(0, 2).map((s) => `<small class="gs-dim">${esc(gsCap(s))}</small>`).join("")}</div></div>`;
  }).join("") : '<p class="gs-empty">Nobody on the market fits the job.</p>';
  const tk = `<section class="gs-pb"><div class="gs-lab">If he goes · who takes his area</div><span class="gs-dim sm">The best man rarely has the ground. Teaching, ground and the rules in signs.</span>${takers}</section>`;

  // the days
  const day = days != null
    ? `<section class="gs-pb gs-days"><div class="gs-num">${days}</div><div><div class="gs-lab">Days to the early signing day</div><b>${x.wavers.length ? `${x.wavers.length === 1 ? "A commit of his signs" : `${x.wavers.length} of his commits sign`} that day, or doesn't` : "Nothing of his waits on it"}</b></div></section>`
    : "";

  // what leaves with him
  const lv = x.leaves.map((a) => `<div class="gs-lad"><div><span class="gs-lab">${esc(a.nickname)}</span>${gsLv(a.now)}</div><span class="gs-arrow">${a.now === a.if_he_goes ? "=" : "&rarr;"}</span><div class="${a.now === a.if_he_goes ? "" : "down"}"><span class="gs-lab">If he goes</span>${gsLv(a.if_he_goes)}</div></div>`).join("");
  const wavers = x.wavers.map((k) => {
    const sh = x.shaken.find((s) => s.id === k.id);
    return `<div class="gs-wv">${gsIni(k.name, 34)}<div><b>${esc(k.name)} · ${esc(k.pos)}</b><small>${k.stars}-star${sh ? `. ${esc(sh.line || "A warning, and a scene.")}` : ". He would waver."}</small></div>${sh && !sh.called ? `<button class="st-btn glass" data-act="gswaver" data-id="${k.id}">Call him</button>` : sh && sh.called ? '<small class="gs-dim">Called</small>' : ""}</div>`;
  }).join("");
  const leaves = `<section class="gs-pb"><div class="gs-lab">What leaves with him</div>${lv || '<p class="gs-empty">Nothing of yours sits on his ground.</p>'}
    ${x.wavers.length ? `<div class="gs-lab gs-sub">${x.wavers.length} commit${x.wavers.length === 1 ? "" : "s"} could waver</div>${wavers}` : ""}</section>`;

  return `<div class="gs-grid"><div class="gs-col">${line}${pulls}</div><div class="gs-col">${hall}${tk}</div><div class="gs-col">${day}${leaves}</div></div>`;
}

// ---- a man's ground, when nobody is after him ----
function gsMan(m, st) {
  const mood = m.mood;
  const first = m.name.split(" ").slice(-1)[0];
  const rid = m.ride ? `<p class="gs-note">${m.ride[1] ? `Riding along with ${esc(m.ride[0])}` : `${esc(m.ride[0])} rides along with him`}. <button class="st-btn glass sm" data-act="gsrideend" data-man="${m.man}">End the ride-along</button></p>` : "";
  const who = `<section class="gs-pb gs-hero"><div class="gs-big">${gsFace(m.face, m.name, 180, "big")}<div class="gs-cap"><span class="gs-lab">${esc(gsCap(m.role))}${m.home ? ` · ${esc(m.home)}` : ""}</span><b>${esc(m.name)}</b></div></div>
    <div class="gs-row"><span class="gs-tag ${gsMood(mood.words)}">${esc(gsCap(mood.words))}</span><span class="gs-dim sm">${m.age ? `${m.age}` : ""}${m.age ? " · " : ""}${esc(gsCap(m.style))}</span></div>
    <p class="gs-body">${esc(gsCap(m.style_why || ""))}</p>
    ${mood.reasons.length || mood.lifts.length ? `<div class="gs-pull"><div>${mood.lifts.map((t) => `<p><em>+</em>${esc(gsCap(t))}</p>`).join("")}</div><div>${mood.reasons.map((t) => `<p><em class="amb">&minus;</em>${esc(gsCap(t))}</p>`).join("")}</div></div>` : '<p class="gs-dim sm">Nothing is weighing on him and nothing is lifting him.</p>'}
    ${m.courted ? `<p class="gs-warn-line">${esc(m.courted)}</p>` : ""}${rid}</section>`;

  const r = m.record;
  const rec = `<section class="gs-pb"><div class="gs-lab">His record with you</div>
    <div class="gs-tiles"><div><span>Kids he wanted</span><b>${r[0]}</b><small>offers he asked for</small></div><div><span>Kids he passed on</span><b>${r[1]}</b><small>he'd not offer</small></div><div><span>He was right</span><b>${r[2]}</b><small>the world caught up</small></div><div><span>Overruled</span><b class="${r[3] > 2 ? "amb" : ""}">${r[3]}</b><small>by you or the board</small></div></div>
    <div class="gs-row sm"><span>Road days</span><b>${m.days[0]} this spring · ${m.days[1]} this fall · ${m.days[2]} this week</b></div>
    ${m.credited.length ? `<div class="gs-lab gs-sub">Kids credited to him</div>${m.credited.slice(0, 5).map((k) => `<div class="gs-row sm"><span>${esc(k.name)} <small class="gs-dim">${k.stars}-star · ${k.year}</small></span><b>${esc(gsCap(k.now))}</b></div>`).join("")}` : '<p class="gs-empty">No signee has been credited to him yet.</p>'}</section>`;

  // his areas, moved on any Monday
  const open = st.empty;
  const mine = m.areas;
  const areas = mine.map((a) => `<div class="gs-ar"><div><b>${esc(a.nickname)}</b><small>${esc(a.why)}${a.with.length ? ` · with ${esc(a.with.join(", "))}` : ""}</small></div>${gsLv(a.level)}<small class="gs-dim">${a.heads} head coaches${a.known + a.trusted + a.home ? `, ${a.known + a.trusted + a.home} of them Known or better` : ""}</small><button class="st-btn glass sm" data-act="gsdrop" data-man="${m.man}" data-area="${esc(a.code)}" title="Take ${esc(a.nickname)} off his list">Take off</button></div>`).join("");
  const adds = open.length ? `<div class="gs-lab gs-sub">Nobody works these</div><div class="gs-opens">${open.map((a) => `<button class="st-btn glass sm" data-act="gsadd" data-man="${m.man}" data-area="${esc(a.code)}">Give ${esc(first)} ${esc(a.nickname)}</button>`).join("")}</div>` : "";
  const ar = `<section class="gs-pb"><div class="gs-row"><div class="gs-lab">His areas</div><span class="gs-dim sm">changes take hold the next Monday</span></div>${areas || '<p class="gs-empty">He has no area. The coordinator gives him one on Monday.</p>'}${adds}</section>`;
  const sign = m.signs && m.signs.length ? `<section class="gs-pb"><div class="gs-lab">What the people around him say</div>${m.signs.map((s) => `<p class="gs-note">${esc(gsCap(s))}</p>`).join("")}</section>` : "";
  const ties = m.people && m.people.length ? `<section class="gs-pb"><div class="gs-lab">His people · ${m.levels[2]} Home, ${m.levels[1]} Trusted, ${m.levels[0]} Known</div>${m.people.slice(0, 6).map((p) => `<a class="gs-ps" data-go="groundperson" data-kind="${esc(p.key.kind)}" data-id="${p.key.id}">${gsIni(p.name, 28)}<div><b>${esc(p.name)}</b><small>${esc(gsCap(p.role))}</small></div>${gsLv(p.level)}</a>`).join("")}</section>` : "";

  // the staff's own business: the coordinator's letter, rides, who left, misses
  const letter = st.proposal.letter && st.proposal.letter.length ? `<section class="gs-pb gs-letter"><div class="gs-lab">The coordinator's letter${st.proposal.date ? ` · ${esc(gsDate(st.proposal.date, { month: "short", day: "numeric" }))}` : ""}</div>${st.proposal.letter.map((l) => `<p class="gs-note">${esc(l)}</p>`).join("")}</section>` : "";
  const left = st.left.length ? `<section class="gs-pb"><div class="gs-lab">Men who left</div>${st.left.map((l) => `<div class="gs-row sm"><span><b>${esc(l.name || "A coach")}</b> <small class="gs-dim">${esc(gsDate(l.date, { month: "short", year: "numeric" }))}</small><br><small class="gs-dim">${esc(l.why || "")}</small></span><span class="gs-tag ${l.terms === "angry" ? "amb" : ""}">${esc(gsCap(l.terms))}</span></div>`).join("")}</section>` : "";
  const miss = st.misses.length ? `<section class="gs-pb"><div class="gs-lab">Kids from areas nobody of yours works</div>${st.misses.map((k) => `<div class="gs-row sm"><span><b>${esc(k.name)}</b> <small class="gs-dim">${k.stars}-star · ${esc(k.area)}</small></span><b>Signed with ${esc(k.to)}</b></div>`).join("")}</section>` : "";
  return `<div class="gs-grid"><div class="gs-col">${who}</div><div class="gs-col">${ar}${rec}</div><div class="gs-col">${sign}${ties}${letter}${left}${miss}</div></div>`;
}

// ---- the rail of your men, along the bottom ----
function gsRail(st, sel) {
  const waiting = new Set(st.courted);
  const row = (m) => {
    const hot = waiting.has(m.man);
    const best = m.areas.length ? m.areas.reduce((a, b) => ((GS_LV[b.level] || 0) > (GS_LV[a.level] || 0) ? b : a)) : null;
    const r = m.record;
    return `<button class="gs-mn${m.man === sel ? " on" : ""}${hot ? " hot" : ""}" data-go="groundstaff" data-id="${m.man}"><span class="gs-mh">${gsFace(m.face, m.name, 30)}<span><b>${esc(m.name.split(" ").slice(-1)[0])}</b><small>${esc(m.role)}</small></span></span>
      <small class="gs-ar1">${best ? `${esc(best.nickname)} · ${esc(best.level)}` : "No area"}${m.areas.length > 1 ? ` +${m.areas.length - 1}` : ""}</small>${best ? gsPips(GS_LV[best.level] || 0, 3) : ""}
      <span class="gs-mm ${hot ? "amb" : gsMood(m.mood)}"><i></i>${hot ? "May go" : esc(gsCap(m.mood).replace("Listening to other jobs", "Listening"))}</span><small class="gs-dim">${r.right} of ${r.offered + r.passed} right${m.ride ? " · riding" : ""}</small></button>`;
  };
  return `<div class="gs-rail">${st.men.map(row).join("")}</div>`;
}

// ---- the buttons ----
const gsDone = async (el, p) => {
  try {
    const r = await p;
    if (r && r.said) GS.said = Array.isArray(r.said) ? r.said.join(" ") : r.said;
  } catch (e) { toast(e.message || String(e)); }
  refresh(el);
};
// An area change sends his whole list again, so read it first.
const gsAreas = (el, change) => {
  const m = Number(el.dataset.man);
  return gsDone(el, api("ground_staff_man", { man: m }).then((x) => api("ground_staff_areas", { man: m, areas: change(x.areas.map((a) => a.code)) })));
};
Object.assign(CORE_MORE, {
  gscounter: (el) => gsDone(el, api("ground_counter", { man: Number(el.dataset.man), kind: el.dataset.kind })),
  gsletgo: (el) => gsDone(el, api("ground_let_go", { man: Number(el.dataset.man) })),
  gswaver: (el) => gsDone(el, api("ground_waver_call", { id: Number(el.dataset.id) })),
  gsrideend: (el) => gsDone(el, api("ground_staff_ride_end", { young: Number(el.dataset.man) })),
  gsadd: (el) => gsAreas(el, (codes) => codes.concat(el.dataset.area)),
  gsdrop: (el) => gsAreas(el, (codes) => codes.filter((c) => c !== el.dataset.area)),
});

// After the page draws: each man's head, drawn from his recipe, kept in the
// save's folder for next time (the initials stay when drawn heads are off).
(() => {
  const after0 = afterRender;
  afterRender = function () {
    after0();
    const seen = new Set();
    for (const f of GS.faces) {
      if (!f || seen.has(f.id) || !f.drawn_on || !f.spec || !window.AFMFaces || !AFMFaces.liveHead) continue;
      seen.add(f.id);
      const boxes = () => [...document.querySelectorAll(`.gs-face[data-gsface="${f.id}"]`)].filter((b) => !b.querySelector("img"));
      if (!boxes().length) continue;
      AFMFaces.liveHead(`gs${f.id}-${f.key}`, JSON.parse(JSON.stringify(f.spec))).then((url) => {
        if (!url) return;
        boxes().forEach((b) => { b.innerHTML = `<img src="${url}" alt="">`; });
        api("face_drawn_store", { id: f.id, key: f.key, data: url }).catch(() => {});
      });
    }
  };
})();
