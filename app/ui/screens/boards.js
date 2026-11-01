// Two recruiting boards that belong with Find recruits (update 2, lane 7),
// in Press Box:
// - The recruiting map (board 7): your board at each prospect's hometown
//   on the real map, rings 250 and 500 miles from campus, every prospect
//   at your level by metro area, your conference rivals' commits and your
//   assistants' areas; the state you pick and its top names; your class
//   so far. A high school coach sees the colleges calling about his
//   players around his school.
// - The race (board 8): a recruit's interest in each school month by
//   month as your staff reads it, your school against the leader with
//   your staff's projection to his decision, the moments that moved him,
//   and the tug of war of his reasons against any school after him.
// Every rating stays your staff's read: the reasons' weights are ranges,
// the interest is what he lets your staff see.
"use strict";

Object.assign(CRUMBS, { recruit_race: "The race" });
// (A high school coach's race leads back to his own side.)
GOTO.recruit_race = (a, d) => (d && d.seat === "hs"
  ? [["His player page", "player", { id: a.id }], ["Colleges calling", "hsside"], ["My players", "myplayers"]]
  : [["His file", "recruit_file", { id: a.id }], ["His player page", "player", { id: a.id }], ["Recruiting board", "recruit_board"], ["Recruiting map", "recruit_board", { tab: "map" }], ["Find recruits", "recruit_find"]]);

// ---------- the map's pieces ----------
const FRM_SIDE = ["VT", "NH", "MA", "RI", "CT", "NJ", "DE", "MD", "DC"];
const FRM_LAND = US_STATES.map((s) => s[5]).join("");
const frmSign = (x, dp = 0) => (x > 0 ? "+" : x < 0 ? "−" : "±") + Math.abs(x).toFixed(dp);
const frmPct = (a, b) => (b ? Math.round((a / b) * 100) + "%" : "0%");
const frmOrd = (n) => {
  const s = ["th", "st", "nd", "rd"], v = n % 100;
  return n + (s[(v - 20) % 10] || s[v] || s[0]);
};
function frmShade(n, max, a) {
  if (!n) return "";
  const pct = (3.5 + Math.sqrt(n / Math.max(1, max)) * a * 100).toFixed(1);
  return `fill:color-mix(in srgb, var(--fr-fg) ${pct}%, var(--frm-land))`;
}

// The map itself: the states (shaded by `fill`), rings round the home
// place, the marks, the state labels (beside the coast for the small ones)
// and the home place in gold.
function frmSvg(o) {
  const states = US_STATES.map(([id, , , , , d]) => `<path class="frm-st${o.sel === id ? " on" : ""}" d="${d}" style="${o.fill(id) || ""}" data-act="${o.act || "fr-mapst"}" data-v="${id}"><title>${esc(STATE_NAMES[id] || id)}${o.tip ? esc(o.tip(id)) : ""}</title></path>`).join("");
  const sel = US_STATES.find((s) => s[0] === o.sel);
  // Rings round a place in an inset stay on its own land.
  const inset = o.home.st === "AK" || o.home.st === "HI" ? US_STATES.find((s) => s[0] === o.home.st) : null;
  const [hx, hy] = usProject(o.home.at[0], o.home.at[1], o.home.st);
  const rings = o.rings.map((mi) => `<path class="frm-ring" d="${usRing(o.home.at[0], o.home.at[1], mi, o.home.st)}"/>`).join("");
  // Each ring's label goes at its top, or the nearest point round the ring
  // clear of the state labels, the home name and the frame's edge.
  const hw = 7 * String(o.home.name || "").length;
  const meets = (a, q) => a[0] < q[2] && q[0] < a[2] && a[1] < q[3] && q[1] < a[3];
  const labBox = (x, y, id) => [x - 13, y - 10, x + 13, y + (o.count && o.count(id) !== "" ? 17 : 3)];
  // The home state's own label steps below the home mark when they meet.
  const labAt = {};
  const own = US_STATES.find((s) => s[0] === o.home.st && FRM_SIDE.indexOf(s[0]) < 0);
  if (own && meets(labBox(own[3], own[4], own[0]), [hx - 10, hy - 19, hx + 10 + hw, hy + 8])) labAt[own[0]] = [hx, hy + 24];
  const busy = US_STATES.filter((s) => FRM_SIDE.indexOf(s[0]) < 0).map((s) => labBox(...(labAt[s[0]] || [s[3], s[4]]), s[0]));
  const hit = (box) => busy.some((q) => meets(box, q));
  // The home name sits to the right of its mark, or to the left when a
  // state label is there.
  const homeLeft = hit([hx + 9, hy - 17, hx + 9 + hw, hy - 3]) && !hit([hx - 9 - hw, hy - 17, hx - 9, hy - 3]);
  busy.push(homeLeft ? [hx - 9 - hw, hy - 19, hx + 6, hy + 6] : [hx - 6, hy - 19, hx + 9 + hw, hy + 6]);
  const ringLabels = o.rings.map((mi) => {
    for (const b of [0, 340, 20, 320, 40, 300, 60, 280, 80, 260, 100, 240, 120, 220, 140, 200, 160, 180]) {
      const [x, y] = usAt(o.home.at[0], o.home.at[1], mi, b, o.home.st);
      const box = [x - 22, y - 7, x + 22, y + 5];
      if (y < 10 || y > US_MAP.h - 6 || x < 24 || x > US_MAP.w - 24) continue;
      if (hit(box)) continue;
      busy.push(box);
      return `<text class="frm-rl" x="${x.toFixed(1)}" y="${(y + 3).toFixed(1)}">${mi} MI</text>`;
    }
    return "";
  }).join("");
  const labels = [], leaders = [];
  for (const [id, cx, cy, lx0, ly0] of US_STATES) {
    const [lx, ly] = (labAt[id] || [lx0, ly0]).map((v) => Math.round(v * 10) / 10);
    const n = o.count ? o.count(id) : "";
    const on = id === o.sel;
    const side = FRM_SIDE.indexOf(id);
    const cls = `frm-lab${on ? " on" : ""}${n || o.marked(id) ? "" : " none"}`;
    if (side >= 0) {
      const y = 112 + side * 22;
      leaders.push(`<line class="frm-lead${on ? " on" : ""}" x1="808" y1="${y}" x2="${cx}" y2="${cy}"/>`);
      labels.push(`<text class="${cls}" x="812" y="${y + 4}" data-act="${o.act || "fr-mapst"}" data-v="${id}">${id}${n !== "" ? `<tspan class="frm-n" dx="5">${n}</tspan>` : ""}</text>`);
    } else {
      labels.push(`<text class="${cls}" x="${lx}" y="${ly}" text-anchor="middle" data-act="${o.act || "fr-mapst"}" data-v="${id}">${id}${n !== "" ? `<tspan class="frm-n" x="${lx}" dy="13">${n}</tspan>` : ""}</text>`);
    }
  }
  return `<svg class="frm-svg" viewBox="0 0 ${US_MAP.w} ${US_MAP.h}" role="img" aria-label="${esc(o.aria)}">
    <defs><clipPath id="frm-land-${o.key}"><path d="${inset ? inset[5] : FRM_LAND}"/></clipPath></defs>
    ${states}
    <g clip-path="url(#frm-land-${o.key})">${rings}</g>
    ${sel ? `<path class="frm-sel" d="${sel[5]}"/>` : ""}
    ${leaders.join("")}
    ${o.marks}
    ${ringLabels}
    <rect class="frm-home" x="${(hx - 4.5).toFixed(1)}" y="${(hy - 4.5).toFixed(1)}" width="9" height="9" transform="rotate(45 ${hx.toFixed(1)} ${hy.toFixed(1)})"/>
    <text class="frm-homet" x="${(homeLeft ? hx - 9 : hx + 9).toFixed(1)}" y="${(hy - 7).toFixed(1)}"${homeLeft ? ' text-anchor="end"' : ""}>${esc(o.home.name)}</text>
    ${o.hi || ""}
    ${labels.join("")}
  </svg>`;
}
const frmCells = (cells) => `<div class="frm-cells">${cells.map(([k, v, d]) => `<div><span class="fr-k">${esc(k)}</span><b class="fr-num">${v}</b><span class="fr-mute">${d}</span></div>`).join("")}</div>`;

// ---------- the college's map (the Map tab of the recruiting board) ----------
const FRM_DOT = { c: [4, "c"], g: [3.6, "g"], b: [3.6, "b"], n: [2.6, "n"] };
const FRM_ORDER = { n: 0, b: 1, g: 2, c: 3 };
async function frMap() {
  const m = await api("recruit_map");
  const k = keep("frmap", { mode: "board", st: m.home, pp: null });
  const by = Object.fromEntries(m.states.map((s) => [s.code, s]));
  if (!by[k.st]) k.st = m.home;
  const areaIx = Object.fromEntries(m.staff.map((c, i) => [c.id, i]));
  const level = m.level === "FBS" ? "FBS" : m.level === "FCS" ? "FCS" : m.level;
  const allWord = m.floor >= 2 ? `every ${level}-level prospect: ${m.floor} stars and up` : `every prospect with a star or an offer`;
  const MODES = {
    board: ["Your board", "every prospect your staff recruits, at his hometown", "on the board", (s) => s.board, 0.17],
    all: ["All talent", `${allWord}, class of ${m.year}`, "prospects", (s) => s.all, 0.3],
    rivals: [`${m.conference} rivals' commits`, `where the other ${m.rivals_n} are getting commitments`, "commits", (s) => s.rivals, 0.3],
    areas: ["Staff areas", "the assistant who recruits each state; pick a state to change it", "on the board", (s) => s.board, 0],
  };
  const [title, sub, unit, val, alpha] = MODES[k.mode] || MODES.board;
  const max = Math.max(1, ...m.states.map(val));
  const sum = m.states.reduce((a, s) => a + val(s), 0);
  const fill = (id) => {
    const s = by[id];
    if (!s) return "";
    if (k.mode === "areas") {
      const ix = s.coach ? areaIx[s.coach.id] : undefined;
      return ix == null ? "" : `fill:color-mix(in srgb, ${FC_AREA[ix % FC_AREA.length]} ${s.board ? 34 : 16}%, var(--frm-land))`;
    }
    return frmShade(val(s), max, alpha);
  };
  // The marks for the mode.
  let marks = "";
  const P = (at, st) => usProject(at[0], at[1], st);
  if (k.mode === "board" || k.mode === "areas") {
    marks = m.dots.slice().sort((a, b) => FRM_ORDER[a.k] - FRM_ORDER[b.k]).map((d) => {
      const [x, y] = P(d.at, d.st);
      const [r, c] = FRM_DOT[d.k] || FRM_DOT.n;
      return `<circle class="frm-dot ${c}" cx="${x.toFixed(1)}" cy="${y.toFixed(1)}" r="${r}" data-act="fr-mapdot" data-id="${d.id}" data-st="${d.st}"><title>${esc(d.name)} · ${Math.round(d.mi)} miles</title></circle>`;
    }).join("");
  } else if (k.mode === "all") {
    const mv = Math.max(1, ...m.metros.map((x) => x.v));
    marks = m.metros.map((x) => { const [cx, cy] = P(x.at); return `<circle class="frm-metro" cx="${cx.toFixed(1)}" cy="${cy.toFixed(1)}" r="${(2 + Math.sqrt(x.v / mv) * 16).toFixed(1)}"><title>${esc(x.name)}: ${frN(x.v)}</title></circle>`; }).join("");
  } else if (k.mode === "rivals") {
    marks = m.campuses.map((c) => { const [x, y] = P(c.at); return `<rect class="frm-camp" x="${(x - 3.5).toFixed(1)}" y="${(y - 3.5).toFixed(1)}" width="7" height="7" rx="1.5"><title>${esc(c.name)}</title></rect>`; }).join("");
  }
  const pd = k.pp != null ? m.dots.find((d) => d.id === k.pp) : null;
  let hi = "";
  if (pd) {
    const [x, y] = P(pd.at, pd.st);
    const right = x > 700;
    hi = `<circle class="frm-hi" cx="${x.toFixed(1)}" cy="${y.toFixed(1)}" r="8.5"/><text class="frm-hit" x="${(right ? x - 13 : x + 13).toFixed(1)}" y="${(y + 4).toFixed(1)}" text-anchor="${right ? "end" : "start"}">${esc(pd.name)}</text>`;
  }
  const svg = frmSvg({
    key: "c", sel: k.st, fill, marks, hi, rings: [250, 500],
    home: { at: m.campus.at, st: m.home, name: m.campus.city },
    count: k.mode === "board" || k.mode === "areas" ? null : (id) => (by[id] && val(by[id])) || "",
    marked: (id) => by[id] && (by[id].board || val(by[id])),
    tip: (id) => (by[id] ? `: ${val(by[id])} ${unit}` : ""),
    aria: `Map of the United States: ${title}`,
  });
  // The legend under the map.
  const keys = {
    board: [["c", "Committed"], ["g", "Offered"], ["b", "Leaning away"], ["n", "Not offered yet"]].map(([c, t]) => `<span><svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true"><circle class="frm-dot ${c}" cx="6" cy="6" r="${FRM_DOT[c][0]}"/></svg>${t}</span>`).join(""),
    all: '<span><svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true"><circle class="frm-metro" cx="7" cy="7" r="5.5"/></svg>Metro areas, sized by prospects</span>',
    rivals: `<span><svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true"><rect class="frm-camp" x="2.5" y="2.5" width="7" height="7" rx="1.5"/></svg>Other ${esc(m.conference)} campuses</span>`,
    areas: m.staff.map((c, i) => `<span><em class="frm-sw" style="background:${FC_AREA[i % FC_AREA.length]}"></em>${esc(trLast(c.name))} <small class="fr-mute">${c.states.length}</small></span>`).join(""),
  }[k.mode];
  const ramp = k.mode === "areas" ? "" : `<span class="frm-ramp"><b class="fr-num">0</b><i></i><b class="fr-num">${max}</b><span>per state</span></span>`;
  const legend = `<div class="frm-key">${ramp}${keys}<span class="frm-ringkey"><i></i>250 and 500 miles from ${esc(m.campus.city)}</span></div>`;
  // The strip under the map.
  let cells;
  const D = m.dots;
  if (k.mode === "board" || k.mode === "areas") {
    const c = D.filter((d) => d.k === "c");
    const far = c.slice().sort((a, b) => b.mi - a.mi)[0];
    const n250 = D.filter((d) => d.mi <= 250).length, n500 = D.filter((d) => d.mi <= 500).length;
    const st = new Set(D.map((d) => d.st)).size;
    cells = [
      ["On the board", frN(D.length), `in ${frPlural(st, "state")}`],
      ["Within 250 miles", frN(n250), `${frmPct(n250, D.length)} of the board`],
      ["Within 500 miles", frN(n500), `${frmPct(n500, D.length)} of the board`],
      ["Commits within 500", `${c.filter((d) => d.mi <= 500).length} of ${c.length}`, far ? `farthest ${frN(Math.round(far.mi))} miles` : "no commits yet"],
    ];
  } else if (k.mode === "all") {
    const top3 = m.states.slice().sort((a, b) => b.all - a.all).slice(0, 3);
    const t3 = top3.reduce((a, s) => a + s.all, 0);
    const big = m.metros[0];
    cells = [
      ["Prospects", frN(sum), `${m.floor >= 2 ? `${level}, ${m.floor} stars and up` : "a star or an offer"}`],
      ["Within 500 miles", frN(m.all_near[1]), `${frmPct(m.all_near[1], sum)} of the class`],
      [top3.map((s) => STATE_NAMES[s.code] || s.code).join(", "), frN(t3), `${frmPct(t3, sum)} of the class`],
      ["Biggest metro", big ? frN(big.v) : "0", big ? esc(big.name) : "none"],
    ];
  } else {
    const topSt = m.states.slice().sort((a, b) => b.rivals - a.rivals)[0];
    const home = by[m.home];
    cells = [
      ["Rival commits", frN(sum), `the other ${frPlural(m.rivals_n, "school")}`],
      [topSt && topSt.rivals ? `From ${STATE_NAMES[topSt.code] || topSt.code}` : "Biggest state", topSt ? frN(topSt.rivals) : "0", topSt ? `${frmPct(topSt.rivals, sum)} of their classes` : ""],
      ["Within your 500 miles", frN(m.rivals_near[1]), `${frmPct(m.rivals_near[1], sum)} of their classes`],
      [`From ${STATE_NAMES[m.home] || m.home}`, frN(home ? home.rivals : 0), `you have ${frPlural(home ? home.commits : 0, "commit")} there`],
    ];
  }
  // Your class so far.
  const c = m.class;
  const parts = c.split.map((s) => s.v).concat([c.open]);
  const tot = Math.max(1, c.slots, c.commits);
  const shades = ["var(--fr-fg)", "var(--fr-fg2)", "var(--fr-fg3)", "var(--fr-hair)"];
  const bar = `<div class="frm-split">${parts.map((v, i) => (v ? `<i style="width:${((v / tot) * 100).toFixed(1)}%;background:${shades[i]}"></i>` : "")).join("")}</div>`;
  const splitCells = c.split.map((s) => [s.k, s.v]).concat([["Open", c.open]]);
  const classCard = `<div class="frm-card"><div class="fr-ph"><span class="fr-k">Class of ${m.year}</span><span class="fr-mute">${c.signing ? `${esc(c.signing.name.toLowerCase())} ${esc(c.signing.date)}` : ""}</span></div>
    <div class="frm-hero"><b class="fr-num">${c.commits}</b><div><span>${c.commits === 1 ? "commit" : "commits"} of ${frPlural(c.slots, "spot")}</span><span class="fr-mute">${c.open ? `${frPlural(c.open, "spot")} still open` : "the class is full"}</span></div></div>
    ${bar}<div class="frm-quad">${splitCells.map(([kk, v]) => `<div><span class="fr-mute">${esc(kk)}</span><b class="fr-num">${v}</b></div>`).join("")}</div>
    <div class="frm-tri">
      <div><span class="fr-k">National</span><b class="fr-num">${c.rank ? frmOrd(c.rank) : "—"}</b><span class="fr-mute">${c.rank ? `of ${c.rank_of} ${esc(level)}` : "not ranked yet"}</span></div>
      <div><span class="fr-k">${esc(m.conference)}</span><b class="fr-num">${c.conf_rank ? frmOrd(c.conf_rank) : "—"}</b><span class="fr-mute">of ${c.conf_of}</span></div>
      <div><span class="fr-k">Average</span><b class="fr-num">${c.average != null ? c.average.toFixed(1) : "—"}</b><span class="fr-mute">${c.average != null ? "composite" : "no rated commits"}</span></div>
    </div></div>`;
  // The state you picked.
  const s = by[k.st] || { code: k.st, board: 0, commits: 0, offers: 0, leaning: 0, all: 0, rivals: 0, top: [] };
  const name = STATE_NAMES[s.code] || s.code;
  const pick = m.staff.length
    ? `<select class="fr-sel frm-area" data-fcarea="${s.code}" aria-label="Who recruits ${esc(name)}"><option value="">The staff's own split</option>${m.staff.map((cc) => `<option value="${cc.id}" ${s.set && s.coach && s.coach.id === cc.id ? "selected" : ""}>${esc(cc.name)}, ${esc(cc.job.toLowerCase())} (${cc.states.length})</option>`).join("")}</select>`
    : "";
  const kv = [
    ["On the board", s.board],
    ["Committed to you", s.commits, s.commits ? "gold" : ""],
    ["Offers out", s.offers],
    ["Leaning elsewhere", s.leaning, s.leaning ? "amber" : ""],
    [m.floor >= 2 ? `${level}-level prospects` : "Prospects with a star or offer", s.all],
    [`${m.conference} rivals' commits`, s.rivals],
    [`From ${m.campus.city}`, s.main ? `${esc(s.main.town)}, ${frN(s.main.mi)} miles` : "—"],
  ].map(([t, v, tone]) => `<div class="frm-kv"><span class="fr-mute">${esc(t)}</span><b class="${tone ? "frm-" + tone : ""}">${v}</b></div>`).join("");
  const stateCard = `<div class="frm-state"><div class="fr-ph"><span class="fr-k">${esc(name)}${s.code === m.home ? " · home state" : ""}</span><span class="fr-mute">${s.coach ? `covered by ${coachLink(s.coach.id, s.coach.name)}` : "nobody covers it"}</span></div>
    ${kv}<div class="frm-kv"><span class="fr-mute">Recruited by</span>${pick || '<span class="fr-mute">no assistants</span>'}</div></div>`;
  const dotWord = { c: "", g: "", b: "amber", n: "mute" };
  const tops = s.top.length
    ? s.top.map((p) => `<div class="frm-prow${k.pp === p.id ? " on" : ""}" data-act="fr-mappp" data-id="${p.id}"><span class="frm-pn">${playerLink(p.id, p.name)}<small class="fr-mute">${esc(p.pos)} · ${esc(p.town)}</small></span>${frStars(p.stars, "—")}<span class="frm-pw ${dotWord[p.k] || ""}">${linkTo("recruit_race", { id: p.id }, esc(p.words))}</span></div>`).join("")
    : `<div class="fr-empty">${s.board ? `${s.board} on your board, none in this class.` : `Nobody from ${esc(name)} is on your board.`} ${linkTo("recruit_find", { state: s.code }, "Find recruits there")}</div>`;
  const modes = frSeg("fr-mapmode", Object.entries(MODES).map(([v, d]) => [v, d[0].replace(`${m.conference} rivals' commits`, "Rivals' commits")]), k.mode, "Map");
  return `<div class="fr frm">
    <section class="fr-panel frm-main">
      <div class="frm-head"><div><span class="frm-title">${esc(title)}</span><span class="fr-mute">${esc(sub)}</span></div>${modes}</div>
      <div class="frm-total fr-mute">${frN(sum)} ${esc(unit)}${k.mode === "board" ? ` · ${frN(D.filter((d) => d.k === "c").length)} committed` : ""}</div>
      <div class="frm-wrap">${svg}</div>
      ${legend}
      ${frmCells(cells)}
    </section>
    <aside class="frm-side">
      <div class="fr-panel">${classCard}</div>
      <div class="fr-panel">${stateCard}
        <div class="frm-tops"><div class="frm-prow frm-th"><span class="fr-k">Top prospects</span><span class="fr-k">Stars</span><span class="fr-k">Where he stands</span></div>${tops}</div></div>
    </aside></div>`;
}

// ---------- the high school's map: the colleges calling around you ----------
const FRM_STAGE = { commit: "c", offer: "g", visit: "v", camp: "n", interest: "n" };
function frHsMap(x, lines) {
  const k = keep("fr-hs", {});
  const home = x.home;
  const st = (code) => lines.filter((l) => l.st === code);
  const count = {};
  for (const l of lines) count[l.st] = (count[l.st] || 0) + 1;
  const max = Math.max(1, ...Object.values(count));
  const order = { n: 0, v: 1, g: 2, c: 3 };
  const marks = lines.slice().sort((a, b) => order[FRM_STAGE[a.stage_key] || "n"] - order[FRM_STAGE[b.stage_key] || "n"]).map((l) => {
    const [cx, cy] = usProject(l.at[0], l.at[1], l.st);
    const c = FRM_STAGE[l.stage_key] || "n";
    const r = c === "c" ? 5 : c === "g" ? 4.2 : 3;
    return `<rect class="frm-cc ${c}" x="${(cx - r).toFixed(1)}" y="${(cy - r).toFixed(1)}" width="${2 * r}" height="${2 * r}" rx="1.5" data-act="fr-hsst" data-v="${l.st}"><title>${esc(l.team.name)} · ${esc(l.stage)} · ${frN(l.mi)} miles</title></rect>`;
  }).join("");
  const svg = frmSvg({
    key: "h", act: "fr-hsst", sel: k.mapst || null, rings: [250, 500], marks,
    home: { at: home.at, st: home.st, name: home.town },
    fill: (id) => frmShade(count[id] || 0, max, 0.25),
    count: (id) => count[id] || "",
    marked: (id) => count[id],
    tip: (id) => (count[id] ? `: ${count[id]} calling` : ""),
    aria: "Map of the United States: the colleges calling about your players",
  });
  const n250 = lines.filter((l) => l.mi <= 250).length, n500 = lines.filter((l) => l.mi <= 500).length;
  const far = lines.slice().sort((a, b) => b.mi - a.mi)[0];
  const offers = lines.filter((l) => /^(offer|commit)$/.test(l.stage_key));
  const cells = [
    ["Colleges calling", frN(lines.length), `from ${frPlural(Object.keys(count).length, "state")}`],
    ["Within 250 miles", frN(n250), `${frmPct(n250, lines.length)} of them`],
    ["Within 500 miles", frN(n500), `${frmPct(n500, lines.length)} of them`],
    ["Farthest", far ? `${frN(far.mi)} mi` : "—", far ? esc(far.team.name) : "nobody yet"],
  ];
  const pickSt = k.mapst && count[k.mapst] ? k.mapst : null;
  const list = (pickSt ? st(pickSt) : offers.length ? offers : lines).slice().sort((a, b) => a.mi - b.mi).slice(0, 14);
  const side = `<div class="fr-panel"><div class="fr-ph"><span class="fr-k">${pickSt ? esc(STATE_NAMES[pickSt] || pickSt) : offers.length ? "The offers, nearest first" : "Nearest first"}</span>${pickSt ? '<button class="fr-chipbtn" data-act="fr-hsst" data-v="">Every state</button>' : ""}</div>
    ${list.length ? `<div class="frm-tops">${list.map((l) => `<div class="frm-prow frm-cl"><span class="frm-pn">${teamLink(l.team)}<small class="fr-mute">${esc(l.level)} · ${l.about.map((p) => esc(p.name)).join(", ")}</small></span><span class="fr-num fr-mute">${frN(l.mi)} mi</span><span class="frm-pw">${/^(offer|commit)$/.test(l.stage_key) ? `<b>${esc(l.stage)}</b>` : esc(l.stage)}</span></div>`).join("")}</div>` : '<div class="fr-empty">No college calls from there.</div>'}</div>`;
  const legend = `<div class="frm-key">${[["c", "Committed or signed"], ["g", "Offered"], ["v", "Visit"], ["n", "Camp invite or interest"]].map(([c, t]) => `<span><svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true"><rect class="frm-cc ${c}" x="2" y="2" width="8" height="8" rx="1.5"/></svg>${t}</span>`).join("")}<span class="frm-ringkey"><i></i>250 and 500 miles from ${esc(home.town)}</span></div>`;
  return `<div class="frm frm-hs"><section class="fr-panel frm-main"><div class="frm-wrap">${svg}</div>${legend}${frmCells(cells)}</section><aside class="frm-side">${side}</aside></div>`;
}

// ---------- the race (board 8) ----------
// A school's interest at day t: the weekly line between its points, none
// before it joined his race.
function frrAt(pts, t) {
  if (!pts.length || t < pts[0][0]) return null;
  for (let i = 1; i < pts.length; i++) {
    if (pts[i][0] >= t) {
      const [t0, v0] = pts[i - 1], [t1, v1] = pts[i];
      return t1 === t0 ? v1 : v0 + ((v1 - v0) * (t - t0)) / (t1 - t0);
    }
  }
  return pts[pts.length - 1][0] >= t - 10 ? pts[pts.length - 1][1] : null;
}
screens.recruit_race = async (a) => {
  const x = await api("recruit_race", { id: a.id });
  if (x.team && x.team.colors) setColors(x.team.colors);
  S.title = `${x.name}: the race`;
  FR.race = x.id;
  const k = keep("frrace:" + x.id, { snap: -1, rival: null, fac: 0, ev: null });
  const hsSeat = x.seat === "hs";
  // The side your staff reads from: your school, or the leader.
  const schools = x.schools;
  const mine = schools.find((s) => s.me && s.now != null);
  const span = x.span;
  // Snapshots: the first of each month, and today.
  // (The last three months' firsts at most, so the row stays one line
  // beside the title; the chart below shows the whole race.)
  const snaps = x.months.filter((mo) => mo.t > 0).slice(-3).map((mo) => [mo.label, mo.t]);
  snaps.unshift([x.since, 0]);
  snaps.push(["Today", span]);
  const snapIx = k.snap < 0 || k.snap >= snaps.length ? snaps.length - 1 : k.snap;
  const tSnap = snaps[snapIx][1], tPrev = snaps[Math.max(0, snapIx - 1)][1];
  const lanesData = schools.map((s) => ({ s, v: frrAt(s.pts, tSnap), p: frrAt(s.pts, tPrev) }))
    .filter((l) => l.v != null || l.s.me)
    .sort((p, q) => (q.v ?? -1) - (p.v ?? -1));
  // Today's standings (the card and the tug of war read these, whatever
  // snapshot the lanes show).
  const vNow = (s) => (s ? frrAt(s.pts, span) : null);
  const nowOrder = schools.map((s) => ({ s, v: vNow(s) })).filter((l) => l.v != null).sort((p, q) => q.v - p.v);
  const ref = mine || (nowOrder[0] && nowOrder[0].s) || (lanesData[0] && lanesData[0].s);
  const others = x.compare.filter((j) => (!ref || j !== ref.id) && schools.some((s) => s.id === j));
  const rivalId = k.rival != null && others.includes(k.rival) ? k.rival : others[0];
  const rival = schools.find((s) => s.id === rivalId);
  const shortOf = (j) => { const s = schools.find((q) => q.id === j); return s ? s.short : ""; };
  const lanes = lanesData.slice(0, 7).map((l, r) => {
    const me = l.s.me, riv = l.s.id === rivalId;
    const v = l.v == null ? null : Math.round(l.v), p = l.p == null ? null : Math.round(l.p);
    const ch = v == null || p == null || snapIx === 0 ? "" : frmSign(v - p);
    const lo = Math.min(l.v ?? 0, l.p ?? l.v ?? 0), hi = Math.max(l.v ?? 0, l.p ?? l.v ?? 0);
    return `<div class="frr-lane${me ? " me" : ""}${riv ? " riv" : ""}"><span class="fr-num frr-rank">${r + 1}</span><span class="frr-name">${teamLink(l.s.team, l.s.name)}${l.s.signed ? ' <small class="fr-mute">signed</small>' : l.s.committed ? ' <small class="fr-mute">committed</small>' : ""}</span>
      <span class="frr-track"><i class="frr-line"></i><i class="frr-tick" style="left:25%"></i><i class="frr-tick" style="left:50%"></i><i class="frr-tick" style="left:75%"></i>
      ${l.v != null && l.p != null && snapIx > 0 ? `<i class="frr-trail" style="left:${lo.toFixed(1)}%;width:${(hi - lo).toFixed(1)}%"></i><i class="frr-ghost" style="left:${l.p.toFixed(1)}%"></i>` : ""}
      ${l.v != null ? `<i class="frr-dot" style="left:${l.v.toFixed(1)}%"></i>` : ""}</span>
      <b class="fr-num frr-val">${v == null ? "—" : v}</b><span class="fr-num frr-ch${me && ch.startsWith("+") ? " frr-up" : me && ch.startsWith("−") ? " frr-dn" : ""}">${ch}</span></div>`;
  }).join("");
  const snapSeg = frSeg("fr-racesnap", snaps.map(([n], i) => [String(i), n]), String(snapIx), "Snapshot");
  // The gap: your school against the best other (or the leader's margin).
  const gapAt = (t) => {
    const vals = schools.map((s) => [s, frrAt(s.pts, t)]).filter(([, v]) => v != null);
    if (!vals.length) return null;
    if (ref && mine) {
      const mv = frrAt(mine.pts, t);
      if (mv == null) return null;
      const best = Math.max(...vals.filter(([s]) => s.id !== mine.id).map(([, v]) => v), 0);
      return mv - best;
    }
    const sorted = vals.map(([, v]) => v).sort((p, q) => q - p);
    return sorted.length > 1 ? sorted[0] - sorted[1] : sorted[0];
  };
  const days = [];
  for (const s of schools) for (const [t] of s.pts) if (!days.includes(t)) days.push(t);
  days.sort((p, q) => p - q);
  const gpts = days.map((t) => [t, gapAt(t)]).filter(([, g]) => g != null);
  // Your staff projects only the last six months before he decides.
  const decideT = x.decides && !x.decides.past && x.decides.t - span <= 183 ? x.decides.t : null;
  const endT = Math.max(span + 14, decideT || 0);
  const gNow = gpts.length ? gpts[gpts.length - 1][1] : 0;
  const back = gpts.filter(([t]) => t >= span - 35);
  const slope = back.length > 1 ? (back[back.length - 1][1] - back[0][1]) / Math.max(7, back[back.length - 1][0] - back[0][0]) : 0;
  const left = decideT != null ? decideT - span : 0;
  const pc = gNow + slope * left * 0.5, pw = 3 + 0.08 * left;
  const PU = pc + pw, PL = pc - pw;
  const allG = gpts.map(([, g]) => g).concat(decideT != null ? [PU, PL] : []).concat([0]);
  const gmax = Math.ceil((Math.max(...allG) + 2) / 10) * 10, gmin = Math.floor((Math.min(...allG) - 2) / 10) * 10;
  const W = 880, X0 = 40, X1 = 840, Y0 = 30, Y1 = 98;
  const GX = (t) => X0 + (t / Math.max(1, endT)) * (X1 - X0);
  const GY = (g) => Y0 + ((gmax - g) / Math.max(1, gmax - gmin)) * (Y1 - Y0);
  const line = gpts.map(([t, g]) => `${GX(t).toFixed(1)},${GY(g).toFixed(1)}`).join(" ");
  const area = (f) => (gpts.length ? `M${GX(gpts[0][0]).toFixed(1)},${GY(0).toFixed(1)}${gpts.map(([t, g]) => `L${GX(t).toFixed(1)},${GY(f(g)).toFixed(1)}`).join("")}L${GX(gpts[gpts.length - 1][0]).toFixed(1)},${GY(0).toFixed(1)}Z` : "");
  const band = decideT != null && gpts.length ? `M${GX(span).toFixed(1)},${GY(gNow).toFixed(1)}L${GX(decideT).toFixed(1)},${GY(PU).toFixed(1)}L${GX(decideT).toFixed(1)},${GY(PL).toFixed(1)}Z` : "";
  const every = Math.max(1, Math.ceil(x.months.length / 12));
  const mon = x.months.filter((mo, i) => (x.months.length - 1 - i) % every === 0).map((mo) => `<line class="frr-mon" x1="${GX(mo.t).toFixed(1)}" x2="${GX(mo.t).toFixed(1)}" y1="${Y0}" y2="${Y1}"/><text class="frr-mt" x="${GX(mo.t).toFixed(1)}" y="112" text-anchor="middle">${esc(mo.label.split(" ")[1] || mo.label).toUpperCase()}</text>`).join("");
  // The axis marks, with an end mark left out when it would sit on the zero's.
  const gl = [gmax, 0, gmin].filter((g, i, arr) => arr.indexOf(g) === i && (g === 0 || Math.abs(GY(g) - GY(0)) >= 11)).map((g) => `<text class="frr-gl" x="34" y="${(GY(g) + 3.5).toFixed(1)}" text-anchor="end">${g === 0 ? "0" : frmSign(g)}</text>`).join("");
  const evIx = k.ev;
  // Each moment's numbered mark over its day; marks a few days apart step
  // right so each stays readable.
  const placed = [];
  const marks = x.moments.map((e, i) => {
    const t = Math.max(0, Math.min(e.t, endT));
    const y = e.planned ? GY(gNow + (pc - gNow) * Math.min(1, (t - span) / Math.max(1, left))) : GY(gapAt(Math.min(t, span)) ?? 0);
    let mx = GX(t);
    while (placed.some(([px, py]) => Math.abs(px - mx) < 19 && Math.abs(py - y) < 19)) mx += 19;
    placed.push([mx, y]);
    return `<g class="frr-mk${i === evIx ? " on" : ""}${e.planned ? " plan" : ""}" data-act="fr-raceev" data-v="${i}"><circle cx="${mx.toFixed(1)}" cy="${(y - 14).toFixed(1)}" r="9"/><text x="${mx.toFixed(1)}" y="${(y - 10.5).toFixed(1)}" text-anchor="middle">${i + 1}</text></g>`;
  }).join("");
  const gapTitle = mine ? `${esc(mine.name.toUpperCase())} AGAINST THE BEST OTHER` : "THE LEADER'S MARGIN";
  const chart = gpts.length > 1
    ? `<svg class="frr-gap" viewBox="0 0 ${W} 118" role="img" aria-label="${mine ? "Your school against the leader over time" : "The leader's margin over time"}">
      <text class="frr-gt" x="0" y="12">${gapTitle}</text>${mon}${gl}
      <path class="frr-below" d="${area((g) => Math.min(g, 0))}"/><path class="frr-above" d="${area((g) => Math.max(g, 0))}"/>
      <line class="frr-zero" x1="${X0}" x2="${X1}" y1="${GY(0).toFixed(1)}" y2="${GY(0).toFixed(1)}"/>
      ${band ? `<path class="frr-band" d="${band}"/><text class="frr-pt" x="${GX(decideT).toFixed(1)}" y="${Math.max(24, GY(PU) - 6).toFixed(1)}" text-anchor="end">staff projection to ${esc(x.decides.what)}</text>` : ""}
      <polyline class="frr-gline" points="${line}"/>
      <line class="frr-snap" x1="${GX(tSnap).toFixed(1)}" x2="${GX(tSnap).toFixed(1)}" y1="28" y2="${Y1}"/>
      ${marks}</svg>`
    : `<div class="fr-empty">Your staff logs his race each Monday${x.followed ? "" : " once he is on your board"}; the chart fills as the weeks come.</div>`;
  // The tug of war of his reasons.
  const refId = ref ? ref.id : null;
  const at = (r, j) => r.by.find((b) => b.id === j);
  const mids = x.reasons.map((r) => (r.weight ? (r.weight[0] + r.weight[1]) / 2 : 3));
  const Wsum = mids.reduce((p, q) => p + q, 0) || 1;
  const PX = 50 / 8;
  const pts = (r, i, j) => { const me = at(r, refId), o = at(r, j); return me && o ? (mids[i] * (me.score - o.score)) / Wsum : 0; };
  const tug = x.reasons.map((r, i) => {
    const v = rival ? pts(r, i, rival.id) : 0, len = Math.min(Math.abs(v) * PX, 50), on = i === k.fac;
    const [lo, hi] = r.weight || [1, 5];
    const sq = [1, 2, 3, 4, 5].map((q) => `<i class="${q <= lo ? "f" : q <= hi ? "o" : ""}"></i>`).join("");
    return `<div class="frr-tug${on ? " on" : ""}" data-act="fr-racefac" data-v="${i}"><span class="frr-rn">${esc(r.name)}</span><span class="frr-w" title="Your staff's read: ${lo} to ${hi} of 5">${sq}</span>
      <span class="frr-bar"><i class="frr-mid"></i><i class="frr-fill${v >= 0 ? " us" : ""}" style="left:${(v >= 0 ? 50 : 50 - len).toFixed(1)}%;width:${len.toFixed(1)}%"></i></span><b class="fr-num${v >= 0 ? " us" : ""}">${frmSign(v, 1)}</b></div>`;
  }).join("");
  const net = rival ? x.reasons.reduce((p, r, i) => p + pts(r, i, rival.id), 0) : 0;
  const nlen = Math.min(Math.abs(net) * PX, 50);
  const refShort = ref ? ref.short : "";
  const rivSeg = others.length ? frSeg("fr-racerival", others.slice(0, 5).map((j) => [String(j), shortOf(j)]), String(rivalId), "Against") : "";
  const weighs = `<div class="fr-panel frr-weighs"><div class="fr-ph"><span><b class="frr-h3">What he weighs</b> <span class="fr-mute">his ${x.reasons.length} reasons, how much each counts to your staff's read, and who wins it</span></span><span class="frr-against"><span class="fr-mute">Against</span>${rivSeg}</span></div>
    <div class="frr-tug frr-th"><span class="fr-k">His reason</span><span class="fr-k">Weight</span><span class="fr-k frr-ends"><span>← ${esc(rival ? rival.name : "")} better</span><span>${esc(ref ? ref.name : "")} better →</span></span><span class="fr-k">Points</span></div>
    ${rival ? tug : '<div class="fr-empty">No other school is after him yet.</div>'}
    ${rival ? `<div class="frr-tug frr-net"><span class="frr-rn"><b>His reasons alone</b></span><span></span><span class="frr-bar"><i class="frr-mid"></i><i class="frr-fill${net >= 0 ? " us" : ""}" style="left:${(net >= 0 ? 50 : 50 - nlen).toFixed(1)}%;width:${nlen.toFixed(1)}%"></i></span><b class="fr-num${net >= 0 ? " us" : ""}">${frmSign(net, 1)}</b></div>` : ""}</div>`;
  // Where it stands, today.
  const leaderOther = nowOrder.find((l) => !mine || l.s.id !== mine.id);
  let gapBig = "—", t1 = "", t2 = "", t3 = "";
  if (x.signed || x.committed) {
    gapBig = x.walk_on ? "Walk-on" : x.signed ? "Signed" : "Committed";
    t1 = `${x.walk_on ? (x.signed ? "walking on at" : "committed to walk on at") : x.signed ? "signed with" : "committed to"} ${x.signed || x.committed}`;
  }
  const done = x.signed || x.committed;
  const doneMine = !!(done && mine && done === mine.name);
  if (gpts.length) {
    if (!done) gapBig = frmSign(Math.round(gNow));
    const worst = gpts.reduce((p, q) => (q[1] < p[1] ? q : p));
    const best = gpts.reduce((p, q) => (q[1] > p[1] ? q : p));
    const dayName = (t) => { const mo = x.months.filter((m2) => m2.t <= t).pop(); return mo ? `${t - mo.t + 1} ${mo.label.split(" ")[1]}` : x.since; };
    if (mine && leaderOther) {
      if (x.committed && !x.signed) t1 = `${x.walk_on ? "committed to walk on at" : "committed to"} ${x.committed} · you ${frmSign(Math.round(gNow))} against ${doneMine ? "the best other" : "them"}`;
      else if (!done) t1 = gNow < 0 ? `behind ${leaderOther.s.name}, ${Math.round(vNow(leaderOther.s))} to ${Math.round(vNow(mine))}` : `ahead of ${leaderOther.s.name}, ${Math.round(vNow(mine))} to ${Math.round(vNow(leaderOther.s))}`;
      t2 = gNow >= worst[1] + 1 && worst[1] < 0 ? `closed from ${frmSign(Math.round(worst[1]))} on ${dayName(worst[0])}` : `widest ${best[1] >= 0 ? "lead" : "gap"} ${frmSign(Math.round(best[1]))} on ${dayName(best[0])}`;
      const alone = x.reasons.reduce((p, r, i) => p + pts(r, i, leaderOther.s.id), 0);
      t3 = `his reasons alone: ${frmSign(alone, 1)} against ${leaderOther.s.short}`;
    } else if (nowOrder[0]) {
      if (!x.signed && !x.committed) t1 = `${nowOrder[0].s.name} leads${nowOrder[1] ? ` ${nowOrder[1].s.name}` : ""}`;
      t2 = `widest margin ${frmSign(Math.round(best[1]))} on ${dayName(best[0])}`;
    }
  }
  const nowCells = [
    [x.decides ? esc(x.decides.label) : "He decides", x.decides ? esc(x.decides.date) : "Not yet known", x.decides ? esc(x.decides.what) : "his own time"],
    ["Next visit", x.next_visit ? esc(x.next_visit.date) : "None set", x.next_visit ? esc(x.next_visit.where) : ""],
    ["Listens to", esc(x.listens), hsSeat ? "and you" : ""],
  ];
  const standCard = `<div class="frm-card"><div class="fr-ph"><span class="fr-k">Where it stands</span><span class="fr-mute">${x.reader ? `read by ${coachLink(x.reader.id, x.reader.name)}${x.reader.state ? ` (${esc(x.reader.state)})` : ""}` : hsSeat ? "your read as his coach" : ""}</span></div>
    <div class="frm-hero${/^[A-Za-z]/.test(gapBig) ? " word" : ""}"><b class="fr-num ${mine && (done ? doneMine : gNow >= 0) ? "frm-gold" : ""}">${esc(gapBig)}</b><div><span>${esc(t1)}</span><span class="fr-mute">${esc(t2)}</span><span class="fr-mute">${esc(t3)}</span></div></div>
    <div class="frm-tri">${nowCells.map(([kk, v, d]) => `<div><span class="fr-k">${kk}</span><b>${v}</b><span class="fr-mute">${d}</span></div>`).join("")}</div></div>`;
  // The reason you picked.
  const fr = x.reasons[Math.min(k.fac, x.reasons.length - 1)];
  let reasonCard = "";
  if (fr) {
    const me = at(fr, refId), o = rival ? at(fr, rival.id) : null;
    const v = rival ? pts(fr, Math.min(k.fac, x.reasons.length - 1), rival.id) : 0;
    const [lo, hi] = fr.weight || [1, 5];
    reasonCard = `<div class="frm-card"><div class="fr-ph"><span class="fr-k">His reason ${fr.k + 1} of ${x.reasons.length}</span><span class="fr-mute">staff read ${lo === hi ? `${lo}` : `${lo} to ${hi}`} of 5</span></div>
      <b class="frr-fname">${esc(fr.name)}</b>
      <div class="frm-tri"><div><span class="fr-k">${esc(refShort)}</span><b class="fr-num frm-gold">${me ? me.score : "—"}</b><span class="fr-mute">${me ? esc(me.fact) : ""}</span></div><div><span class="fr-k">${esc(rival ? rival.short : "")}</span><b class="fr-num">${o ? o.score : "—"}</b><span class="fr-mute">${o ? esc(o.fact) : ""}</span></div><div><span class="fr-k">Points to ${esc(refShort)}</span><b class="fr-num ${v >= 0 ? "frm-gold" : "frm-amber"}">${frmSign(v, 1)}</b><span class="fr-mute">at the read's middle</span></div></div>
      <div class="frr-src">${fr.source.map((s) => `<span class="frr-tag">${esc(s)}</span>`).join("")}</div>
      <div class="frm-kv"><span class="fr-mute">What could move it</span><b>${esc(fr.move)}</b></div></div>`;
  }
  const moments = x.moments.length
    ? x.moments.map((e, i) => `<div class="frr-mrow${i === evIx ? " on" : ""}" data-act="fr-raceev" data-v="${i}"><span class="frr-mn">${i + 1}</span><span class="frr-mt2"><small>${esc(e.date)}</small><span>${esc(e.text)}</span></span><b class="fr-num ${e.planned ? "fr-mute" : e.school && e.school.me ? "frm-gold" : ""}">${e.planned ? "planned" : e.delta != null && e.school ? `${frmSign(e.delta)} ${esc(e.school.short)}` : ""}</b></div>`).join("")
    : '<div class="fr-empty">Nothing has moved him since your staff began following him.</div>';
  const kicker = `Class of ${x.class_of} · ${esc(x.pos)} · ${x.stars ? `${x.stars} stars` : "unranked"} · ${esc(x.town)}`;
  return `<div class="pad fr frr">
    <div class="frr-headrow"><div><div class="fr-k">${kicker}</div><h1 class="fr-h1">${playerLink(x.id, x.name)}</h1></div></div>
    ${gotoFor("recruit_race", { id: x.id }, x)}
    <div class="frr-grid">
      <div class="frr-left">
        <div class="fr-panel"><div class="fr-ph"><span><b class="frr-h3">The race</b> <span class="fr-mute">his interest, 0 to 100, as your staff reads it · ${snapIx === 0 ? "where it started" : `trail since ${esc(snaps[snapIx - 1][0])}`}</span></span>${snapSeg}</div>
          <div class="frr-lanes">${lanes || '<div class="fr-empty">No school is after him yet.</div>'}</div>${chart}</div>
        ${weighs}
      </div>
      <aside class="fr-panel frr-side">${standCard}${reasonCard}<div><div class="fr-k frr-mh">Moments that moved him</div>${moments}</div></aside>
    </div></div>`;
};
