/* Ground's rival (lane S6; docs/design/ground.md part J, board 6, GR75, GR79).
 * The other staffs' men on your ground, in the glass look with his program's
 * colors: his face and style read from what he does, his ground, the race for
 * a kid of yours in signs (dated and numbered, each with where it came from,
 * never his numbers), what the kid's coach tells you, his claim and the day
 * it settles, your answers with their costs, where he works on the map, his
 * history with you and your men against him.
 *
 *   groundrival   with a man ({id}): his page. Without one: the rival men on
 *                 your ground, your commits in play and the claims out there.
 *
 * It reads `ground_rival_page`, `ground_rivals`, `ground_race` and
 * `ground_map`, and writes `ground_fight` and `ground_claim` (lane G9) and
 * `face_drawn_store`. No hidden number is ever on it: a rival is a style in
 * words, a level in words and signs in words. Every class is rv- (the glass
 * and rows reuse the map's gd- ones) so the app's global names never restyle
 * it. */
"use strict";

I.gdrival = '<circle cx="8.5" cy="8" r="3.2"/><path d="M2.5 20c0-3.3 2.7-6 6-6s6 2.7 6 6"/><path d="M15 6l5 5m0-5l-5 5"/>';
navAdd({ go: "groundrival", label: "Rivals", icon: "gdrival", sec: "ground", after: "groundperson", seats: ["college"] });
CRUMBS.groundrival = "Rivals";

const RV = { face: null };
const rvCap = (s) => (s ? s.charAt(0).toUpperCase() + s.slice(1) : s);
const rvPlural = (n, one, many) => `${n} ${n === 1 ? one : many || one + "s"}`;
const rvLink = (man, inner, cls = "") => `<a class="${cls}" data-go="groundrival" data-id="${man}">${inner}</a>`;
// A level word as the map draws it: the word and its three ticks.
// The brighter of his program's two colors, toned so it reads on the glass.
const rvHi = (c1, c2) => {
  const lum = (h) => { const m = /^#?([0-9a-f]{6})$/i.exec(h || ""); if (!m) return 0; const n = parseInt(m[1], 16); return (0.2126 * ((n >> 16) & 255) + 0.7152 * ((n >> 8) & 255) + 0.0722 * (n & 255)) / 255; };
  return gdTone(lum(c2) > lum(c1) ? c2 : c1, 1);
};
const rvLv = (name, cls = "") => gdLv({ name, n: ["Stranger", "Known", "Trusted", "Home"].indexOf(name) }, cls);

screens.groundrival = async (args) => {
  if (!args || args.id == null) return rvList();
  const x = await api("ground_rival_page", { man: Number(args.id) });
  title(x.name, `${x.school} · rival`);
  RV.face = x.face;
  const [c1, c2] = x.colors;
  const t = x.totals;
  const hue = rvHi(c1, c2);

  // ---- the topbar ----
  const top = `<div class="gd-top"><div><h1>${esc(x.today)}</h1><small>Rivals · ${esc(x.name)}, ${esc(x.school)}</small></div><div class="gd-sp"></div>
    <div class="gp-stat"><span>On your ground</span><b>${rvPlural(t.men, "rival man", "rival men")}, ${rvPlural(t.staffs, "staff")}</b></div>
    <div class="gp-stat"><span>Races with him</span><b>${rvPlural(x.races.length, "kid")}, ${rvPlural(x.claims_his.length + x.claims_yours.length, "claim")}</b></div>
    <button class="st-btn primary" data-go="groundstaff">Plan an answer</button></div>`;

  // ---- the left column: his face and what you know of him ----
  const f = x.face;
  const pic = f.drawn
    ? `<img class="rv-face-img" src="${f.drawn}" alt="${esc(x.name)}">`
    : `<span class="rv-mono" style="--c1:${esc(c1)};--c2:${esc(c2)}">${esc(f.initials)}</span>`;
  const line = (l, h) => `<div class="rv-line"><span>${l}</span><div>${h}</div></div>`;
  const areaWords = x.areas.length ? x.areas.map((a) => esc(a[1])).join(" and ") : "yours";
  const homeLine = x.home.length
    ? `Home with ${rvPlural(x.home.length, "coach", "coaches")}: ${esc(x.home.slice(0, 3).join(", "))}${x.home.length > 3 ? ` and ${x.home.length - 3} more` : ""}.`
    : "Nobody on your ground has made him a Home man yet.";
  const levels = x.areas.filter((a) => a[2] !== "Stranger").map((a) => `${esc(a[2])} in ${esc(a[1])}`).join(", ");
  const left = `<section class="gd-glass rv-hero"><div class="rv-face" data-rvface="${f.id}">${pic}</div>
      <div><div class="rv-kicker" style="color:${hue}">${esc(x.school)} · ${esc(x.role)}</div><h1 class="rv-name">${esc(x.name)}</h1><div class="gd-sub">His areas: ${areaWords}</div></div>
      <div class="rv-lines">${line("His style", `<b>${esc(rvCap(x.style))},</b> from what he does: ${esc(x.style_line)}`)}
        ${line("His ground", `${homeLine}${levels ? ` ${esc(rvCap(levels))}.` : ""}`)}
        ${x.career.length ? line("His career", `${esc(x.career.join(". "))}.`) : ""}
        ${x.seen.length ? line("Seen lately", `${esc(x.seen.join("; "))}.`) : ""}</div></section>`;

  // ---- the middle: the race in signs, his claim, your answers ----
  const races = x.races.length
    ? x.races.map((r) => `<section class="gd-glass rv-race"><div class="rv-head"><div class="gd-lab2">The race for <a data-go="player" data-id="${r.id}">${esc(r.name)}</a> · in signs</div><span class="dim">never his numbers</span></div>
        <div class="rv-signs">${r.signs.map((s) => `<div class="rv-sg"><span class="k">${s.n}</span><span class="d">${esc(s.date)}</span><div><b>${esc(s.text)}</b></div><span class="src">${esc(s.source)}</span></div>`).join("")}</div>
        ${r.told.length ? `<div class="rv-told">${r.told.map((l) => `<p>${esc(l)}</p>`).join("")}</div>` : ""}
        <small class="dim">${esc(r.position)} at <a data-go="groundschool" data-id="${r.school_id}">${esc(r.school)}</a>${r.said ? ` · you called the family: ${esc(r.said)}` : ""}</small></section>`).join("")
    : `<section class="gd-glass"><div class="gd-lab2">The race, in signs</div><div class="gd-empty">${esc(x.name)} isn't working any commit of yours right now. ${x.seen.length ? `Your people have seen him at ${esc(x.seen.slice(0, 3).join(", "))}.` : "Your people haven't seen him at a school lately."}</div></section>`;

  const open = x.claims_his.filter((c) => c.held == null);
  const settled = [...x.claims_his, ...x.claims_yours].filter((c) => c.held != null);
  const claimBox = (c, yours) => `<div class="rv-claim${yours ? " yours" : ""}"><div class="gd-lab2"${yours ? "" : ' style="color:var(--gd-amber)"'}>${yours ? "Your claim about him, and when it settles" : "His claim, and when it settles"}</div>
      <div class="rv-q">${esc(c.words)}</div><small>To ${esc(c.name)}'s family, ${esc(c.said)}. Settles ${esc(c.settles)}. ${esc(c.note)}</small></div>`;
  const claims = [...open.map((c) => claimBox(c, false)), ...x.claims_yours.filter((c) => c.held == null).map((c) => claimBox(c, true))].join("");
  const settledRows = settled.length
    ? `<div class="gd-lab2">Settled</div>${settled.map((c) => `<div class="rv-ans done"><div><b>${esc(c.words)}</b><small>${esc(c.name)}'s family · ${esc(c.said)}</small></div><span class="gd-chip${c.held ? "" : " gold"}">${c.held ? "It held up" : "It wasn't true"}</span></div>`).join("")}`
    : "";

  const answers = [];
  for (const r of x.races) {
    if (r.open) answers.push(`<button class="rv-ans" data-act="rvfight" data-id="${r.id}"><div><b>Call ${esc(r.name)}'s family</b><small>Costs a call to his family and his coach. How it lands follows where they stand with you: ${esc(r.family.name)}.</small></div><span class="gd-chip">a call</span></button>`);
    for (const c of r.your_claims) {
      if (c.made) continue;
      answers.push(`<button class="rv-ans" data-act="rvclaim" data-id="${r.id}" data-about="${x.program}" data-topic="${c.topic}"><div><b>${esc(c.title)}</b><small>It settles on the facts, in ${c.topic === "job" ? "December" : "about two months"}. If it isn't true, it costs you with the family and his coach.</small></div><span class="gd-chip amb">a risk</span></button>`);
    }
  }
  if (open.length) answers.unshift(`<div class="rv-ans calm"><div><b>Let ${esc(open[0].settles)} answer it</b><small>Costs nothing now. If it isn't true, the family remembers who said it, and so does his coach.</small></div><span class="gd-chip gold">free</span></div>`);
  const answer = `<section class="gd-glass rv-answer">${claims}<div class="gd-lab2">Your answer</div>${answers.length ? answers.join("") : '<div class="gd-empty">Nothing to answer. He has no claim out and none of your commits is in his race.</div>'}${settledRows}</section>`;
  const mid = `<div class="rv-col m">${races}${answer}</div>`;

  // ---- the right column: where he works, history, your men against him ----
  const hist = x.history.length
    ? x.history.map((h) => `<div class="rv-hist"><span class="rv-dot"></span><div>${esc(h)}</div></div>`).join("")
    : '<div class="gd-empty">Nothing between you yet. You haven\'t coached together or raced for a kid.</div>';
  const against = x.against.length
    ? x.against.map((a) => `<div class="rv-vs"><div><b>${esc(a.man || "Nobody")}</b><small>${esc(a.nickname)}${a.man ? "" : " · no man of yours is on it"}</small></div><div class="rv-lvs"><span>You</span>${gdLv(a.you, a.you.n ? "you" : "")}<span>Him</span>${rvLv(a.his)}</div></div>`).join("")
    : '<div class="gd-empty">He isn\'t working an area you cover.</div>';
  const right = `<div class="rv-col r"><section class="gd-glass"><div class="gd-lab2">Where he works</div><div id="rv-map" class="rv-map" data-map='${esc(JSON.stringify({ map: x.map, colors: [hue, hue], races: x.races.map((r, n) => ({ n: n + 1, name: r.name, school: r.school, state: r.state, at: r.at })) }))}'><div class="gd-empty">Drawing the map.</div></div><small class="dim">His program's color is his ground. The dashed lines are its pipelines to campus.</small></section>
    <section class="gd-glass"><div class="gd-lab2">His history with you</div>${hist}</section>
    <section class="gd-glass"><div class="gd-lab2">Your men against him</div>${against}</section></div>`;

  return `<div class="sl gd"><div class="gd-room rv-room" style="--c1:${esc(c1)};--c2:${esc(c2)};--hi:${esc(hue)}">${top}<div class="rv-body"><div class="rv-col l">${left}</div>${mid}${right}</div></div></div>`;
};

// Where the Rivals page starts: his men, your commits in play, the claims.
async function rvList() {
  const [a, b] = await Promise.all([api("ground_rivals", {}), api("ground_race", {})]);
  const staffs = new Set(a.rivals.map((r) => r.program)).size;
  title("Rivals", `${a.rivals.length} on your ground`);
  const rows = a.rivals.map((r) => rvLink(r.man, `<span class="rv-row"><span class="rv-sw" style="background:${gdTone(r.colors[0], 0.95)}"></span><div><b>${esc(r.name)}</b><small>${esc(r.school)} · ${esc(r.role)} · ${esc(r.style)}</small></div><span class="rv-areas">${r.areas.slice(0, 2).map((z) => `<span>${esc(z[1])} ${rvLv(z[2])}</span>`).join("")}</span></span>`, "rv-rowlink")).join("");
  const wavers = b.wavers.length
    ? b.wavers.slice(0, 8).map((x) => `<div class="rv-wv"><div><b>${esc(x.name)}</b><small>${esc(x.rival_school || "A rival")} is working him · ${esc(x.date_words)} · ${rvPlural(x.signs.length, "sign")}</small>${x.line ? `<small class="said">${esc(x.line)}</small>` : ""}</div>${x.fought ? '<span class="gd-chip">You called</span>' : `<button class="st-btn small" data-act="rvfight" data-id="${x.id}">Call the family</button>`}</div>`).join("")
    : '<div class="gd-empty">None of your commits is wavering.</div>';
  const claims = b.claims.length
    ? b.claims.slice(0, 8).map((c) => `<div class="rv-wv"><div><b>${esc(c.yours ? "You" : c.by || "A rival")}: “${esc(c.words)}”</b><small>${esc(c.name)}'s family · settles ${esc(c.settles_words)}</small></div><span class="gd-chip${c.held === false ? " gold" : ""}">${c.held == null ? "Open" : c.held ? "It held up" : "It wasn't true"}</span></div>`).join("")
    : '<div class="gd-empty">No claims either way.</div>';
  return `<div class="sl gd"><div class="gd-room rv-room" style="--c1:#c25b0c;--c2:#2a1204"><div class="gd-top"><div><h1>Rivals</h1><small>${esc(a.today)} · The other staffs' men on your ground</small></div><div class="gd-sp"></div>
    <div class="gp-stat"><span>On your ground</span><b>${rvPlural(a.rivals.length, "rival man", "rival men")}, ${rvPlural(staffs, "staff")}</b></div></div>
    <div class="rv-listbody"><section class="gd-glass rv-list"><div class="gd-lab2">The men working your areas, the most ground first</div>${rows || '<div class="gd-empty">No rival has been seen on your ground yet. Visit a school and read the sign-in sheet.</div>'}</section>
    <div class="rv-col"><section class="gd-glass"><div class="gd-lab2">Your commits in play</div>${wavers}</section><section class="gd-glass"><div class="gd-lab2">Claims out there</div>${claims}</section></div></div></div></div>`;
}

Object.assign(CORE_MORE, {
  rvfight: async (el) => {
    try {
      const r = await api("ground_fight", { id: Number(el.dataset.id) });
      if (r && r.said) toast(r.said);
    } catch (e) { toast(e.message); }
    refresh(el);
  },
  rvclaim: async (el) => {
    try {
      const r = await api("ground_claim", { id: Number(el.dataset.id), about: Number(el.dataset.about), topic: el.dataset.topic });
      if (r && r.said) toast(r.said);
    } catch (e) { toast(e.message); }
    refresh(el);
  },
  rvstate: (el) => {
    S.route.args = { ...S.route.args, state: el.dataset.state };
    refresh(el);
  },
});

// His ground on the map: his program's color on the areas he works, dashed
// lines from every area in the state to his campus where his pipelines run,
// and your commits he is in the race for, numbered.
async function rvDrawMap(box) {
  const d = JSON.parse(box.dataset.map);
  const states = d.map.states;
  const picked = states.includes(S.route.args && S.route.args.state) ? S.route.args.state : (d.races.find((r) => states.includes(r.state)) || {}).state || states[0];
  if (!picked) { box.innerHTML = '<div class="gd-empty">No map for him yet.</div>'; return; }
  const [m, art] = await Promise.all([api("ground_map", { state: picked }), gdArt(picked)]);
  if (!art) { box.innerHTML = `<div class="gd-empty">No map is drawn for ${esc(m.state.name)} yet.</div>`; return; }
  if (!box.isConnected) return;
  const W = art.projection.w, H = art.projection.h;
  const mine = new Set(d.map.areas);
    const paths = Object.entries(art.areas).map(([k, v]) => `<path d="${v.d}" style="${mine.has(k) ? `fill:${d.colors[0]};fill-opacity:.5` : "fill:var(--gd-blank)"}"/>`).join("");
  const lines = [];
  for (const a of m.areas) {
    const p = a.pipelines.find((q) => q.program === d.map.program && q.strength >= 0.12);
    if (!p) continue;
    const [lx, ly] = gdPt(art, a.label[0], a.label[1]);
    const [tx, ty] = gdPt(art, p.at[0], p.at[1]);
    const t = Math.min(1, ...[tx < 0 ? (0 - lx) / (tx - lx) : 1, tx > W ? (W - lx) / (tx - lx) : 1, ty < 0 ? (0 - ly) / (ty - ly) : 1, ty > H ? (H - ly) / (ty - ly) : 1]);
    lines.push(`<line x1="${lx.toFixed(1)}" y1="${ly.toFixed(1)}" x2="${(lx + (tx - lx) * t).toFixed(1)}" y2="${(ly + (ty - ly) * t).toFixed(1)}" stroke="${esc(d.colors[0])}" class="rv-pipe"/>`);
  }
  const k = (state) => d.races.filter((r) => r.state === state);
  const pins = k(picked).map((r) => { const [px, py] = gdPt(art, r.at[0], r.at[1]); return `<g class="rv-pin" transform="translate(${px.toFixed(1)} ${py.toFixed(1)})"><circle r="9"/><text y="3.5">${r.n}</text></g><text class="rv-pl" x="${(px + 13).toFixed(1)}" y="${(py + 4).toFixed(1)}">${esc(r.school)}</text>`; }).join("");
  const names = m.areas.filter((a) => mine.has(a.code)).map((a) => { const [lx, ly] = gdPt(art, a.label[0], a.label[1]); return `<text class="rv-al" x="${lx.toFixed(1)}" y="${ly.toFixed(1)}">${esc(a.nickname)}</text>`; }).join("");
  box.innerHTML = `${states.length > 1 ? `<div class="gd-layers">${states.map((s) => `<button aria-pressed="${s === picked}" data-act="rvstate" data-state="${s}">${s}</button>`).join("")}</div>` : ""}
    <svg viewBox="0 0 ${W} ${H}" class="rv-svg" role="img" aria-label="${esc(m.state.name)} by area, with his ground shaded">${paths}<path class="gd-bord" d="${art.borders}"/><path class="gd-edge" d="${art.edge}"/>${lines.join("")}${names}${pins}</svg>`;
  const svg = box.querySelector("svg");
  if (svg && svg.clientWidth) {
    const s = W / svg.clientWidth;
    svg.querySelectorAll(".rv-pipe").forEach((e) => { e.setAttribute("stroke-width", 1.8 * s); e.setAttribute("stroke-dasharray", `${2 * s} ${5 * s}`); });
    svg.querySelectorAll(".rv-pin circle").forEach((e) => e.setAttribute("r", 8 * s));
    svg.querySelectorAll(".rv-pin text").forEach((e) => { e.style.fontSize = `${10 * s}px`; e.setAttribute("y", 3.5 * s); });
    svg.querySelectorAll(".rv-pl, .rv-al").forEach((e) => { e.style.fontSize = `${11 * s}px`; e.style.strokeWidth = `${3 * s}px`; });
    svg.querySelector(".gd-bord")?.setAttribute("stroke-width", 1.1 * s);
    svg.querySelector(".gd-edge")?.setAttribute("stroke-width", 1.4 * s);
  }
}

// After the page draws: his map, and his head from his recipe (kept in the
// save's folder for next time).
(() => {
  const after0 = afterRender;
  afterRender = function () {
    after0();
    const box = document.getElementById("rv-map");
    if (box) rvDrawMap(box);
    const f = RV.face;
    const host = f && document.querySelector(`.rv-face[data-rvface="${f.id}"]`);
    if (!host || host.querySelector("img") || !f.drawn_on || !f.spec || !window.AFMFaces || !AFMFaces.liveHead) return;
    AFMFaces.liveHead(`rv${f.id}-${f.key}`, JSON.parse(JSON.stringify(f.spec))).then((url) => {
      if (!url) return;
      const b = document.querySelector(`.rv-face[data-rvface="${f.id}"]`);
      if (b && !b.querySelector("img")) b.innerHTML = `<img class="rv-face-img" src="${url}" alt="">`;
      api("face_drawn_store", { id: f.id, key: f.key, data: url }).catch(() => {});
    });
  };
})();
