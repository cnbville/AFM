// Lane screens: the game boards (update 2, lane 9). Registers screens.<name>.
"use strict";
/* Six Press Box boards against the game's own data:
 *
 *   Game flow     V10, a game page tab: the win chance after every snap,
 *                 the swing plays, the drives on one field, the final.
 *   Line battles  V12, a game page tab: every rep up front, snap by snap,
 *                 the protection and the rush, each matchup.
 *   Passing       V11, a quarterback's player page tab: every throw your
 *                 film has, where it went, to whom, off what pocket clock.
 *   capboard      V14, where the money goes: the NFL cap by player and
 *                 year, college's NIL budget and scholarships.
 *   locker        V15, the locker room: who's close to whom, the moods,
 *                 the leaders, the friction, the units' chemistry.
 *   polls         V16, the race: the college polls, the NFL's playoff
 *                 picture, your state's rankings, week by week.
 *
 * Ratings are your staff's ranges. Classes carry the bd- prefix.
 */

I.blocks = '<rect x="3" y="4" width="10" height="9" rx="1.5"/><rect x="15" y="4" width="6" height="5" rx="1.5"/><rect x="15" y="11" width="6" height="9" rx="1.5"/><rect x="3" y="15" width="10" height="5" rx="1.5"/>';
I.locker = '<circle cx="6" cy="7" r="2.2"/><circle cx="18" cy="7" r="2.2"/><circle cx="12" cy="17" r="2.2"/><path d="M8 8l2.6 7M16 8l-2.6 7M8.2 7h7.6"/>';
I.race = '<path d="M3 18l5-6 4 3 4-7 5 4"/><circle cx="21" cy="12" r="1.2"/><path d="M3 21h18"/>';

navAdd({
  go: "capboard", icon: "blocks", sec: "team", after: "contracts", seats: ["nfl", "college"],
  show: () => !!(S.status && S.status.mine),
  get label() { return seatKind() === "college" ? "NIL budget" : "Cap"; },
});
navAdd({ go: "locker", label: "Locker room", icon: "locker", sec: "team", after: "people", seats: ["college", "nfl", "hs"], show: () => !!(S.status && S.status.mine) });
navAdd({
  go: "polls", icon: "race", sec: "world", after: "standings", seats: ["college", "nfl", "hs"],
  show: () => !!(S.status && S.status.mine),
  get label() { const k = seatKind(); return k === "nfl" ? "Playoff picture" : k === "hs" ? "State rankings" : "Polls"; },
});
Object.assign(CRUMBS, { capboard: "Where the money goes", locker: "Locker room", polls: "The race" });

// ---------- shared ----------
const bdNum = (v, d = 1) => (v == null || Number.isNaN(Number(v)) ? "—" : Number(v).toFixed(d));
const bdPct = (a, b) => (b ? Math.round((100 * a) / b) : 0);
function bdMoney(v) {
  if (v == null) return "—";
  const a = Math.abs(v), s = v < 0 ? "−" : "";
  if (a >= 1e6) return `${s}$${(a / 1e6).toFixed(1)}M`;
  if (a >= 1e3) return `${s}$${Math.round(a / 1e3)}K`;
  return `${s}$${Math.round(a)}`;
}
// A surname, past any Jr. or III.
const bdSur = (n) => {
  const w = String(n || "").split(" ").filter(Boolean);
  while (w.length > 1 && /^(jr|sr|ii|iii|iv|v)\.?$/i.test(w[w.length - 1])) w.pop();
  return w.pop() || "";
};
// A segmented switch: each button sets keep(key)[field] and redraws.
function bdSegs(key, field, list, cur, cls = "", extra = "") {
  return `<div class="bd-segs ${cls}" role="tablist">${list.map(([v, n]) => `<button class="bd-seg" role="tab" data-act="bd-pick" data-k="${esc(key)}" data-f="${esc(field)}" data-v="${esc(v)}" aria-selected="${String(v) === String(cur)}"${extra}>${n}</button>`).join("")}</div>`;
}
// Equal label-over-value cells, the hero row's numbers.
function bdCells(list) {
  return `<div class="bd-cells">${list.map((c) => `<div class="bd-cell${c.bad || c.warn ? " bad" : ""}"${c.tip ? ` title="${esc(c.tip)}"` : ""}><span class="st-label">${esc(c.k)}</span><b class="bd-cond">${c.html != null ? c.html : esc(c.v)}</b>${c.sub ? `<small>${esc(c.sub)}</small>` : ""}</div>`).join("")}</div>`;
}
const bdEmpty = (t) => `<div class="bd-empty">${esc(t)}</div>`;
const bdTeam = (t, text) => (t && t.ref ? teamLink(t, text) : esc(text || (t && t.name) || ""));
Object.assign(CORE_MORE, {
  "bd-pick": (el) => {
    const st = keep(el.dataset.k, {});
    const val = (v) => (v === "" || v == null ? null : /^-?\d+$/.test(v) ? Number(v) : v);
    st[el.dataset.f] = val(el.dataset.v);
    if (el.dataset.f2) st[el.dataset.f2] = val(el.dataset.v2);
    if (el.dataset.f3) st[el.dataset.f3] = val(el.dataset.v3);
    refresh(el);
  },
});

// ======================================================================
// V10, game flow: a game page tab, first and the default.
// ======================================================================
window.gamePageTabs = window.gamePageTabs || {};
window.gamePageFirst = window.gamePageFirst || [];
window.gamePageOwnWp = window.gamePageOwnWp || [];
window.gamePageFirst.unshift("Game flow");
window.gamePageOwnWp.push("Game flow");

function bdFlowChart(x, sel) {
  const W = 940, H = 214, L = 30, R = 930, T = 30, B = 176;
  const end = Math.max(x.end, x.length);
  const X = (t) => L + (Math.min(t, end) / end) * (R - L);
  const Y = (p) => T + (1 - p) * (B - T);
  const pts = x.wp.map(([, t, p]) => `${X(t).toFixed(1)},${Y(p).toFixed(1)}`).join(" ");
  const first = x.wp[0], last = x.wp[x.wp.length - 1];
  const area = first ? `${X(first[1]).toFixed(1)},${Y(0.5)} ${pts} ${X(last[1]).toFixed(1)},${Y(0.5)}` : "";
  const qs = [];
  const div = (a, b, label, line) => `${line ? `<line class="bd-q" x1="${X(a).toFixed(1)}" x2="${X(a).toFixed(1)}" y1="${T}" y2="${B}"></line>` : ""}<text class="bd-axis" x="${X((a + b) / 2).toFixed(1)}" y="${H - 6}" text-anchor="middle">${label}</text>`;
  for (let q = 0; q < 4; q++) qs.push(div(q * x.qlen, Math.min(end, (q + 1) * x.qlen), "Q" + (q + 1), q > 0));
  // Overtime: each period (or, with no clock, each round) from its start.
  const ot = x.ot || [];
  ot.forEach(([a, label], k) => qs.push(div(a, k + 1 < ot.length ? ot[k + 1][0] : end, label, true)));
  const us = x.names[x.us].toUpperCase(), them = x.names[1 - x.us].toUpperCase();
  // Each swing's letter on the line after its snap; two close together
  // step apart, above or below, so both stay readable.
  const placed = [];
  const marks = x.swings.map((s, j) => {
    const on = j === sel;
    const px = X(s.t), py0 = Y(s.after / 100);
    let py = py0;
    for (const dy of [0, -26, 26, -52, 52]) {
      const y = Math.max(T, Math.min(B, py0 + dy));
      if (!placed.some(([a, b]) => Math.abs(a - px) < 28 && Math.abs(b - y) < 26)) { py = y; break; }
    }
    placed.push([px, py]);
    return `<button class="bd-mk${on ? " on" : ""}${s.delta < 0 ? " down" : ""}" style="left:${((px / W) * 100).toFixed(2)}%;top:${((py / H) * 100).toFixed(2)}%" data-act="bd-pick" data-k="bd:flow" data-f="s" data-v="${j}" aria-label="Swing play ${s.letter}">${s.letter}</button>`;
  }).join("");
  return `<div class="bd-wp"><svg viewBox="0 0 ${W} ${H}" role="img" aria-label="Our chance to win after every snap">
    <defs><linearGradient id="bdWpArea" x1="0" y1="${T}" x2="0" y2="${B}" gradientUnits="userSpaceOnUse"><stop offset="0" class="bd-st-gold"></stop><stop offset=".5" class="bd-st-gold0"></stop><stop offset=".5" class="bd-st-ink0"></stop><stop offset="1" class="bd-st-ink"></stop></linearGradient></defs>
    ${qs.join("")}<line class="bd-mid" x1="${L}" x2="${R}" y1="${Y(0.5)}" y2="${Y(0.5)}"></line>
    <text class="bd-axis strong" x="${L}" y="${T - 12}">${esc(us)} WINS</text><text class="bd-axis" x="${L}" y="${B + 18}">${esc(them)} WINS</text>
    <polygon class="bd-wp-area" points="${area}"></polygon><polyline class="bd-wp-line" points="${pts}"></polyline>
  </svg>${marks}</div>`;
}

window.gamePageTabs["Game flow"] = async (g, a) => {
  const x = await api("board_flow", { index: a.index });
  const st = keep("bd:flow", { s: 0 });
  const sel = Math.min(st.s ?? 0, Math.max(0, x.swings.length - 1));
  const us = x.names[x.us], them = x.names[1 - x.us];
  const lost = !x.final.won && !x.final.tied;
  const pt = lost ? x.high : x.low;
  const low = pt ? `${lost ? "Highest" : "Lowest"} <b class="bd-cond">${pt.p}%</b> ${pt.qn > 4 ? "in overtime" : `with ${esc(pt.clock)} left in the ${["first", "second", "third", "fourth"][pt.qn - 1] || "game"}`}` : "";
  const spread = x.spread ? `${x.spread > 0 ? "favoured by" : "underdog by"} ${Math.abs(x.spread)} before kickoff` : "even before kickoff";
  const drives = x.drives.map((d) => {
    const lo = Math.min(d.from, d.to), hi = Math.max(d.from, d.to);
    const on = d.swing && x.swings[sel] && x.swings[sel].letter === d.swing;
    return `<div class="bd-drow${d.ours ? " ours" : ""}${on ? " on" : ""}" data-act="gp-drive" data-i="${d.i}" role="link" tabindex="0" title="This drive in the play by play">
      <span class="bd-dq">${esc(d.q)} · ${esc(d.clock)}</span>
      <span class="bd-dbar"><i class="bd-d50"></i><i class="bd-dseg" style="left:${lo}%;width:${Math.max(0.6, hi - lo)}%"></i><i class="bd-ddot" style="left:${d.to}%"></i></span>
      <span class="bd-dres${d.bad ? " bad" : ""}${d.points > 0 ? " pts" : ""}">${esc(d.result)}${d.swing ? ` <em class="bd-let">${esc(d.swing)}</em>` : ""}</span>
      <span class="bd-cond bd-dim">${d.plays} · ${d.yards} ${d.yards === 1 ? "yd" : "yds"} · ${esc(d.time)}</span></div>`;
  }).join("");
  const f = x.final;
  const qrow = (k, name, mine) => `<div class="bd-qg${mine ? "" : " them"}"><span>${esc(name)}</span>${f.quarters.map((q) => `<span class="bd-cond">${q[k]}</span>`).join("")}<span class="bd-cond tot">${f.score[k]}</span></div>`;
  const stats = x.stats.map((s) => `<div class="bd-srow"><span class="bd-cond">${esc(s.a)}</span><span class="bd-smid"><small>${esc(s.k)}</small><span class="bd-split"><i style="width:${s.share}%"></i><i></i></span></span><span class="bd-cond bd-r">${esc(s.b)}</span></div>`).join("");
  const swings = x.swings.length
    ? x.swings.map((s, j) => `<div class="bd-wrow${j === sel ? " on" : ""}" data-act="bd-pick" data-k="bd:flow" data-f="s" data-v="${j}" role="button" tabindex="0">
        <span class="bd-letter${j === sel ? " on" : ""}">${s.letter}</span>
        <span class="bd-wtext">${esc(s.text)}${s.who.length ? `<small>${s.who.map((p) => playerLink(p.id, p.name)).join(", ")}</small>` : ""}
          <span class="bd-wlinks"><a class="link" data-vwgo="${s.n}">Watch ›</a><a class="link" data-act="gp-play" data-n="${s.n}">Snap ›</a></span></span>
        <span class="bd-wright"><span class="bd-cond bd-dim">${esc(s.q)} ${esc(s.clock)}</span>${((d) => `<b class="bd-cond${d < 0 ? " bad" : ""}">${d > 0 ? "+" : d < 0 ? "−" : "±"}${Math.abs(d)}%</b>`)(Math.round(s.after) - Math.round(s.before))}<small class="bd-dim">${Math.round(s.before)}% to ${Math.round(s.after)}%</small></span></div>`).join("")
    : bdEmpty("No snap swung the game by four points or more.");
  const how = f.how ? `<span>${esc(f.word)} on ${esc(f.how.what)}</span><span class="bd-dim">${esc(f.how.when)} · <a class="link" data-vwgo="${f.how.n}">Watch ›</a></span>` : `<span>${esc(f.word)}</span>`;
  return `<div class="bd"><div class="bd-flow">
    <div class="bd-col">
      <section class="bd-panel">
        <div class="bd-ph"><span><b class="bd-pt">Win chance</b> <span class="bd-ps">${esc(us)}'s chance to win after every snap · ${esc(spread)}</span></span><span class="bd-ps">${low}</span></div>
        ${bdFlowChart(x, sel)}
      </section>
      <section class="bd-panel bd-drives">
        <div class="bd-ph"><b>Drives</b><span class="bd-ps">gold: ${esc(us)} · grey: ${esc(them)} · the left end zone is ${esc(us)}'s · click a drive for its plays</span></div>
        <div class="bd-drow bd-thd"><span>Drive</span><span class="bd-dends"><span>${esc(initials(us))}<span class="bd-gl"> goal line</span></span><span>50</span><span>${esc(initials(them))}<span class="bd-gl"> goal line</span></span></span><span>Result</span><span class="bd-r">Plays · yards · time</span></div>
        ${drives || bdEmpty("No drives kept for this game.")}
      </section>
    </div>
    <aside class="bd-panel bd-side">
      <div class="bd-card">
        <div class="bd-line2"><span class="st-label">Final${x.overtime ? " · overtime" : ""}</span><span class="bd-dim">${f.record ? `record ${esc(f.record)}` : esc(x.level)}</span></div>
        <div class="bd-hero"><b class="bd-cond bd-big${f.won ? "" : f.tied ? "" : " lost"}">${f.score[0]}-${f.score[1]}</b><span class="bd-how">${how}</span></div>
        <div class="bd-qgrid"><div class="bd-qg bd-qh"><span></span>${f.quarters.map((q, i) => `<span>${i < 4 ? i + 1 : "OT"}</span>`).join("")}<span>T</span></div>${qrow(0, us, true)}${qrow(1, them, false)}</div>
      </div>
      <div class="bd-block"><div class="bd-line2"><span class="st-label" title="${esc(us)}">${esc(initials(us))}</span><span class="st-label">Team stats</span><span class="st-label" title="${esc(them)}">${esc(initials(them))}</span></div>${stats}</div>
      <div class="bd-block"><span class="st-label">Swing plays</span>${swings}</div>
    </aside>
  </div></div>`;
};

// ======================================================================
// V12, line battles: a game page tab.
// ======================================================================
const BD_OUT = ["won", "even", "lost"];
window.gamePageTabs["Line battles"] = async (g, a) => {
  const x = await api("board_battles", { index: a.index });
  const bk = "bd:bat:" + a.index;
  const st = keep(bk, { mode: "pass", row: 0, snap: null });
  const modes = [["pass", "Pass protection"], ["run", "Run blocking"], ["rush", "Pass rush"]];
  const mk = x.modes[st.mode] ? st.mode : "pass";
  const m = x.modes[mk];
  const us = x.names[x.us], them = x.names[1 - x.us];
  const head = `<div class="bd-ph"><span><b class="bd-pt">Line battles</b> <span class="bd-ps">${mk === "rush" ? `our rushers against ${esc(them)}'s blockers` : `our blockers against ${esc(them)}'s front`} · every rep, snap by snap</span></span>${bdSegs(bk, "mode", modes, mk, "", ' data-f2="snap" data-v2="" data-f3="row" data-v3="0"')}</div>`;
  if (!m.snaps.length) return `<div class="bd bd-bat"><section class="bd-panel">${head}${bdEmpty(mk === "run" ? "No runs in this game." : "No dropbacks in this game.")}</section></div>`;
  const rows = m.rows;
  const ri = Math.min(st.row ?? 0, Math.max(0, rows.length - 1));
  const sn = st.snap != null && st.snap < m.snaps.length ? st.snap : null;
  // The rep grid: a row a man, a column a snap.
  const n = m.snaps.length;
  const qt = m.quarters.map((k, q) => (k == null ? "" : `<span class="bd-gq" style="left:${((k / n) * 100).toFixed(2)}%">${q < 4 ? "Q" + (q + 1) : "OT"}</span>`)).join("");
  const grid = rows.map((r, i) => {
    const cells = r.cells.map((c, k) => {
      if (!c) return `<i class="bd-gc none${k === sn ? " col" : ""}"></i>`;
      const tip = `${m.snaps[k].q} ${m.snaps[k].clock}: ${BD_OUT[c.o]}${c.mv ? " · " + c.mv : ""}${c.home ? " · got home" : ""}`;
      return `<i class="bd-gc o${c.o}${c.home ? " home" : ""}${k === sn ? " col" : ""}" data-act="bd-pick" data-k="${bk}" data-f="snap" data-v="${k}" title="${esc(tip)}"></i>`;
    }).join("");
    const bad = mk === "rush" ? r.pct < 20 : r.lostPct >= 30;
    return `<div class="bd-grow${i === ri ? " on" : ""}">
      <span class="bd-gname" data-act="bd-pick" data-k="${bk}" data-f="row" data-v="${i}" data-f2="snap" data-v2="" role="button" tabindex="0"><b>${esc(r.spot || r.pos)}</b> ${esc(bdSur(r.name))}${r.hurt ? ` <em class="bd-hurt" title="${esc(r.hurt)}">hurt</em>` : ""}</span>
      <span class="bd-gcells" style="--n:${n}">${cells}</span>
      <span class="bd-gwel bd-cond${bad ? " bad" : ""}">${r.won} ${r.even} ${r.lost} <b>${Math.round(r.pct)}%</b></span></div>`;
  }).join("");
  // The front: our man and the man he faced most.
  const fr = m.front;
  const gapNames = mk === "run" ? ["Left end", "Left tackle", "Middle", "Right tackle", "Right end"] : ["Left edge", "Left inside", "Middle", "Right inside", "Right edge"];
  const gmax = Math.max(1, ...fr.gaps.map((q) => q.n));
  const gapTitle = mk === "run" ? "Where the runs went: yards a run by hole" : mk === "pass" ? "Where the pressure came through" : "Where our pressure got home";
  // The front across the line: their men above ours, each pair joined, and
  // an arrow through each gap, wider the more came through it (runs go up
  // from our side; pressure comes down at ours, or up at theirs when we
  // rush).
  const fn = Math.max(1, fr.links.length);
  const gx = [0, Math.min(2, fn) / fn, 0.5, 1 - Math.min(2, fn) / fn, 1].map((f) => 3 + f * 94);
  const up = mk !== "pass";
  const chips = (side) => fr.links.map((l, i) => {
    const name = side === "them" ? (l.bName ? esc(bdSur(l.bName)) : "—") : esc(bdSur(l.aName));
    const pos = side === "them" ? esc(l.bPos || "") : esc(rows[i] ? rows[i].spot || l.aPos : l.aPos);
    return `<span class="bd-chip ${side}${i === ri ? " on" : ""}" data-act="bd-pick" data-k="${bk}" data-f="row" data-v="${i}" data-f2="snap" data-v2="" role="button" tabindex="0">${name}<small>${pos}</small></span>`;
  }).join("");
  const cons = fr.links.map((l, i) => {
    const bad = mk === "rush" ? l.pct < 20 : l.lost >= 30;
    return `<span class="bd-fcon${bad ? " bad" : ""}${i === ri ? " on" : ""}" style="left:${(((i + 0.5) / fn) * 100).toFixed(2)}%"><b class="bd-cond">${Math.round(l.pct)}%</b><small>${l.reps}</small></span>`;
  }).join("");
  const arrows = fr.gaps.map((q, k) => {
    const v = mk === "run" ? (q.n ? bdNum(q.v) + " yds" : "—") : String(q.n);
    const bad = mk === "pass" ? q.n >= 3 : mk === "run" ? q.n >= 3 && q.v < 3 : false;
    const sw = q.n ? 2 + (7 * q.n) / gmax : 1;
    return `<span class="bd-farrow${bad ? " bad" : ""}${q.n ? "" : " none"}" style="left:${gx[k].toFixed(2)}%" title="${esc(gapNames[k])}: ${esc(v)}${mk === "run" ? ` over ${q.n} run${q.n === 1 ? "" : "s"}` : ""}">
      <svg viewBox="0 0 28 72" aria-hidden="true"${up ? ' class="up"' : ""}><line x1="14" y1="4" x2="14" y2="54" stroke-width="${sw.toFixed(1)}"></line><path d="M${(14 - 4 - sw).toFixed(1)} 52 L14 68 L${(14 + 4 + sw).toFixed(1)} 52 Z"></path></svg>
      <small class="bd-cond">${esc(v)}</small></span>`;
  }).join("");
  const front = `<div class="bd-front2" style="--n:${fn}">
      <div class="bd-fside"><span class="st-label">${esc(them)}</span></div>
      <div class="bd-frow">${chips("them")}</div>
      <div class="bd-fband">${cons}${arrows}</div>
      <div class="bd-frow">${chips("ours")}</div>
      <div class="bd-fside"><span class="st-label">${esc(us)}</span></div>
      <div class="bd-fgaps">${gapNames.map((n, k) => `<span style="left:${gx[k].toFixed(2)}%">${esc(n)}</span>`).join("")}</div>
    </div>`;
  // The side: a snap's card, or the picked man's matchups.
  let side;
  if (sn != null) {
    const s = m.snaps[sn];
    side = `<div class="bd-card">
      <div class="bd-line2"><span class="st-label">${esc(s.q)} ${esc(s.clock)} · ${esc(s.sit)}</span><a class="link" data-act="bd-pick" data-k="${bk}" data-f="snap" data-v="">Close</a></div>
      <span class="bd-ptext">${esc(s.text)}</span>
      ${bdCells(s.facts.map(([k, v]) => ({ k, v })))}
      <div class="bd-reps">${s.chips.map((c) => `<div class="bd-rep o${c.o}"><span>${playerLink(c.id, bdSur(c.name))} <small>${esc(c.pos)}</small></span><span class="bd-dim">vs ${playerLink(c.vs, bdSur(c.vsName))}</span><b>${esc(c.out)}</b><small>${c.mv ? esc(c.mv) + " · " : ""}${bdNum(c.t)} s${c.double ? " · double team" : ""}</small></div>`).join("") || '<div class="bd-dim">No reps up front on this snap.</div>'}</div>
      <div class="bd-links"><a class="link" data-vwgo="${s.n}">Watch it ›</a><a class="link" data-act="gp-play" data-n="${s.n}">In the play by play ›</a></div></div>`;
  } else if (rows[ri]) {
    const r = rows[ri];
    const ps = m.pairs.filter((p) => p.a === r.id);
    const pk = keep("bd:batp", {});
    const p = ps.find((q) => q.b === pk[r.id]) || ps[0];
    const rr = (list, who) => list && list.length ? list.map((q) => `<div class="bd-attr"><span>${esc(q.k)}</span>${range(q.r, who)}</div>`).join("") : '<div class="bd-dim">Not seen</div>';
    side = p ? `<div class="bd-card">
      <div class="bd-line2"><span class="st-label">${esc(r.spot || r.pos)} · ${esc(r.cls || "")}</span><span class="bd-dim">${r.won + r.even + r.lost} rep${r.won + r.even + r.lost === 1 ? "" : "s"}</span></div>
      <span class="bd-mtitle">${playerLink(p.a, p.aName)} <span class="bd-dim">vs</span> ${playerLink(p.b, p.bName)} <small class="bd-dim">${esc(p.bPos)}</small></span>
      ${ps.length > 1 ? `<div class="bd-segs small">${ps.map((q) => `<button class="bd-seg" data-act="bd-pick" data-k="bd:batp" data-f="${r.id}" data-v="${q.b}" aria-selected="${q.b === p.b}">${esc(bdSur(q.bName))} <small>${q.reps}</small></button>`).join("")}</div>` : ""}
      <div class="bd-hero"><b class="bd-cond bd-big">${Math.round(p.pct)}%</b><span class="bd-how"><span>${mk === "rush" ? "rush wins" : "reps won"}, ${p.won} of ${p.reps}</span><span class="bd-dim">${p.even} even · ${p.lost} lost${p.avgThrow != null ? ` · ball out in ${bdNum(p.avgThrow)} s` : ""}</span></span></div>
      <div class="bd-wel"><i class="o0" style="width:${p.pct}%"></i><i class="o1" style="width:${p.evenPct}%"></i><i class="o2" style="width:${p.lostPct}%"></i></div>
      <div class="bd-two"><div><span class="st-label">${esc(bdSur(p.aName))} · ${esc(p.fromA || "your staff")}</span>${rr(p.ra, "your staff")}</div><div><span class="st-label">${esc(bdSur(p.bName))} · ${esc(p.fromB || "film")}</span>${rr(p.rb, p.fromB || "film")}</div></div>
      ${p.moves.length ? `<div class="bd-kvs"><span class="st-label">Moves he faced</span>${p.moves.map((q) => `<div class="bd-kv"><span>${esc(q.k)}</span><b class="bd-cond${q.lost ? " bad" : ""}">lost ${q.lost} of ${q.faced}</b></div>`).join("")}</div>` : ""}
      ${p.byOut.length ? `<div class="bd-kvs"><span class="st-label">Yards a run by how his rep went</span>${p.byOut.map((q) => `<div class="bd-kv"><span>${esc(q.k)} · ${q.n}</span><b class="bd-cond">${q.v == null ? "—" : bdNum(q.v) + " yds"}</b></div>`).join("")}</div>` : ""}
      <div class="bd-kvs"><span class="st-label">${mk === "rush" ? "Wins" : "Reps won"} by situation</span>${p.splits.map((q) => `<div class="bd-kv"><span>${esc(q.k)}</span><b class="bd-cond">${q.v == null ? "—" : Math.round(q.v) + "%"}</b></div>`).join("")}</div>
      ${p.lostAt.length ? `<div class="bd-kvs"><span class="st-label">The lost reps</span><div class="bd-chips">${p.lostAt.map((q) => `<a class="bd-tag${q.home ? " bad" : ""}" data-vwgo="${q.n}" title="Watch it">${bdNum(q.t)} s${q.home ? " · got home" : ""}</a>`).join("")}</div></div>` : ""}
      ${p.quote && p.quote.text ? `<div class="bd-quote">“${esc(p.quote.text)}”<small>${coachLink(p.quote.id, p.quote.name)}, ${esc(p.quote.role)}</small></div>` : ""}
    </div>` : bdEmpty("No matchups for him in this game.");
  }
  const calls = m.calls;
  const callTbl = `<div class="bd-card"><div class="bd-line2"><span class="st-label">${esc(calls.title)}</span></div>
    <div class="bd-tbl"><div class="bd-tr bd-thd">${calls.cols.map((c, i) => `<span${i ? ' class="bd-r"' : ""}>${esc(c)}</span>`).join("")}</div>${calls.rows.map((r) => `<div class="bd-tr">${r.map((c, i) => `<span${i ? ' class="bd-r bd-cond"' : ""}>${esc(c)}</span>`).join("")}</div>`).join("")}</div>
    ${calls.note ? `<small class="bd-dim">${esc(calls.note)}</small>` : ""}</div>`;
  const notCharted = !x.charted ? `<div class="bd-note">This game was played before the line was charted snap by snap: the reps come from the blocks the engine kept (won or lost and how long each held), without protections, rush gaps, even reps or where each man lined up.</div>` : "";
  return `<div class="bd bd-bat">
    <section class="bd-panel">${head}${notCharted}${bdCells(m.cells.map((c) => ({ k: c.k, v: c.v, sub: c.sub, bad: c.bad })))}</section>
    <div class="bd-cols">
      <div class="bd-col">
        <section class="bd-panel">
          <div class="bd-ph"><b>Every rep</b><span class="bd-ps">a row a man, a column a snap · light: won · dim: even · amber: lost · dot: got home · click a square for the snap, a name for his matchups</span></div>
          <div class="bd-grid"><div class="bd-grow bd-thd"><span>${mk === "rush" ? "Our rusher" : "Our blocker"}</span><span class="bd-gcells bd-gqs">${qt}</span><span class="bd-r">W E L · won</span></div>${grid}</div>
        </section>
        <section class="bd-panel">
          <div class="bd-ph"><b>The front</b><span class="bd-ps">${mk === "rush" ? "each rusher and the blocker he faced most" : "each blocker and the man he faced most"} · ${mk === "rush" ? "rush" : "rep"} wins</span></div>
          ${front}
          <div class="bd-ph"><b>${gapTitle}</b><span class="bd-ps">${mk === "run" ? "the arrows: where each run crossed the line, yards a run" : x.charted ? "the arrows: the first man home on each pressured snap" : "not charted for this game"}</span></div>
        </section>
      </div>
      <aside class="bd-panel bd-side">${side || ""}${callTbl}</aside>
    </div>
  </div>`;
};

// ======================================================================
// V11, the pass map: a quarterback's Passing tab.
// ======================================================================
const BD_ZONES = ["Deep left", "Deep middle", "Deep right", "Middle left", "Middle", "Middle right", "Short left", "Short middle", "Short right", "Behind left", "Behind middle", "Behind right"];
const BD_DEPTH = ["20 yards or more", "10 to 19 yards", "0 to 9 yards", "behind the line"];
const BD_BINS = ["behind", "0-4", "5-9", "10-14", "15-19", "20-29", "30+"];
const BD_CLOCK = [["under 2.0 s", 0, 2], ["2.0 to 2.5", 2, 2.5], ["2.5 to 3.0", 2.5, 3], ["3.0 to 3.5", 3, 3.5], ["3.5 s or more", 3.5, 99]];
const bdOrd = (n) => { const s = ["th", "st", "nd", "rd"], v = n % 100; return n + (s[(v - 20) % 10] || s[v] || s[0]); };
// A throw's zone: thirds of the field across (from the passer's view, his
// left on the left) by four depths.
function bdZone(t) {
  const row = t.air >= 20 ? 0 : t.air >= 10 ? 1 : t.air >= 0 ? 2 : 3;
  const col = t.y1 > 35.55 ? 0 : t.y1 >= 17.78 ? 1 : 2;
  return row * 3 + col;
}
const bdSigned = (e, d = 2) => (e > 0 ? "+" : e < 0 ? "−" : "±") + Math.abs(e).toFixed(d);
// What the staff says about a set of throws, from their numbers.
function bdThrowNote(a, kind) {
  if (!a.length) return kind === "zone" ? "He hasn't thrown here this season." : "No throws to him on film.";
  const c = a.filter((t) => t.ok).length, epa = a.reduce((s, t) => s + t.epa, 0) / a.length;
  const ints = a.filter((t) => t.int).length, tds = a.filter((t) => t.td).length;
  const xp = a.reduce((s, t) => s + t.x, 0) / a.length, rate = c / a.length;
  const here = kind === "zone" ? "here" : "his way";
  if (a.length < 4) return `Only ${a.length} ${a.length === 1 ? "throw" : "throws"} ${here}: too few to say.`;
  if (ints >= 2 && epa < 0) return `${ints} picks ${kind === "zone" ? "here" : "throwing to him"}. It costs us points: he has to see the safety before he lets it go.`;
  if (epa >= 0.3) return `${kind === "zone" ? "His best money" : "A read he trusts, and it pays"}: ${c} of ${a.length}${tds ? `, ${tds} TD` : ""}, ${bdSigned(epa)} points a throw.`;
  if (rate < xp - 0.06) return `Catchable balls aren't being caught: ${c} of ${a.length} against about ${Math.round(xp * a.length)} expected.`;
  if (epa < -0.15) return `This costs us: ${bdSigned(epa)} points a throw on ${a.length} throws.`;
  return `Solid, not special: ${c} of ${a.length}, ${bdSigned(epa)} points a throw.`;
}

PLAYER_TABS.push({
  name: "Passing",
  after: "Stats",
  show: (p) => p.pos === "QB",
  draw: async (p) => {
    const x = await api("board_passmap", { id: p.id });
    if (x.source === "none") return `<div class="bd">${bdEmpty("Your staff has no film of his throws. Your own games and the film of your coming opponents are kept; nobody else's throws are.")}</div>`;
    const key = "bd:pm:" + p.id;
    const st = keep(key, { f: "all", v: "paths", z: 4, r: null });
    const keepT = (t) => st.f === "all" || (st.f === "clean" && !t.pr) || (st.f === "pressure" && t.pr) || (st.f === "third" && t.d === 3);
    const T = x.throws.map((t) => ({ ...t, z: bdZone(t) })).filter(keepT);
    const sum = (a, f) => a.reduce((s, t) => s + f(t), 0);
    const xpOf = (a) => (a.length ? sum(a, (t) => t.x) / a.length : 0);
    const level = x.base && !x.base.none ? x.base : null;
    const lvlName = x.base ? x.base.name : "his level";
    // The field: 11.25 px a yard, 50 yards past the line at the top.
    // A ball aimed past the sideline sits on it.
    const PX = 600 / 53.33, X = (y) => (53.33 - Math.max(0.3, Math.min(53, y))) * PX, Y = (d) => (50 - Math.max(-8, Math.min(49.4, d))) * PX;
    const rEff = st.r != null && T.some((t) => t.tgt === st.r) ? st.r : null;
    const focus = (t) => (rEff != null ? t.tgt === rEff : t.z === st.z);
    let lines = "", nums = "";
    for (let d = -5; d <= 45; d += 5) {
      const y = Y(d).toFixed(1);
      lines += `<line class="bd-yl${d === 0 ? " los" : d % 10 === 0 ? " ten" : ""}" x1="0" x2="600" y1="${y}" y2="${y}"></line>`;
      if (d % 10 === 0 || d === -5) {
        const t = d === 0 ? "LOS" : d > 0 ? "+" + d : "−" + -d;
        nums += `<text class="bd-axis" x="8" y="${(Y(d) - 5).toFixed(1)}">${t}</text><text class="bd-axis" x="592" y="${(Y(d) - 5).toFixed(1)}" text-anchor="end">${t}</text>`;
      }
    }
    let hash = "";
    for (let d = -8; d <= 49; d++) {
      if (d % 5 === 0) continue;
      const y = Y(d).toFixed(1);
      hash += `M222 ${y}H228M372 ${y}H378M0 ${y}H5M595 ${y}H600`;
    }
    const zoneEpa = {};
    for (let i = 0; i < 12; i++) { const q = T.filter((t) => t.z === i); zoneEpa[i] = q.length ? sum(q, (t) => t.epa) / q.length : 0; }
    let marks = "";
    for (const t of T) {
      const on = focus(t), x2 = X(t.y1), y2 = Y(t.air), qx = X(t.y0), qy = Y(-6);
      const o = on ? 1 : 0.24;
      if (st.v === "heat") {
        const e = zoneEpa[t.z];
        marks += `<circle class="bd-heat${e < 0 ? " bad" : ""}" cx="${x2.toFixed(1)}" cy="${y2.toFixed(1)}" r="22" opacity="${(rEff != null && !on ? 0.12 : Math.min(0.85, 0.22 + Math.abs(e))).toFixed(2)}"></circle>`;
        continue;
      }
      if (st.v === "paths") {
        const dx = x2 - qx, dy = y2 - qy, len = Math.hypot(dx, dy) || 1;
        let px = -dy / len, py = dx / len;
        if ((dx >= 0 ? 1 : -1) * px < 0) { px = -px; py = -py; }
        const cx = (qx + x2) / 2 + px * len * 0.07, cy = (qy + y2) / 2 + py * len * 0.07;
        marks += `<path class="bd-path${t.int ? " int" : t.ok ? " ok" : ""}" d="M${qx.toFixed(1)} ${qy.toFixed(1)}Q${cx.toFixed(1)} ${cy.toFixed(1)} ${x2.toFixed(1)} ${y2.toFixed(1)}" opacity="${o}"></path>`;
        if (t.ok && t.yac > 0) marks += `<line class="bd-yac" x1="${x2.toFixed(1)}" y1="${y2.toFixed(1)}" x2="${x2.toFixed(1)}" y2="${Y(t.air + t.yac).toFixed(1)}" opacity="${o}"></line>`;
      }
      const tip = `<title>${esc(`${t.date} · ${t.opp} · ${t.q} ${t.clock} · ${bdNum(t.air, 0)} air yards${t.ok ? `, ${bdNum(t.yac, 0)} after the catch` : ""}`)}</title>`;
      if (t.int) marks += `<path class="bd-x" d="M${(x2 - 4.5).toFixed(1)} ${(y2 - 4.5).toFixed(1)}l9 9m0 -9l-9 9" opacity="${on ? 1 : 0.4}">${tip}</path>`;
      else marks += `<circle class="bd-dot${t.ok ? " ok" : ""}" cx="${x2.toFixed(1)}" cy="${y2.toFixed(1)}" r="3" opacity="${on ? 1 : 0.4}">${tip}</circle>`;
      if (t.td) marks += `<circle class="bd-ring" cx="${x2.toFixed(1)}" cy="${y2.toFixed(1)}" r="6.2" opacity="${on ? 1 : 0.4}"></circle>`;
    }
    const rowsY = [[50, 20], [20, 10], [10, 0], [0, -8.3]];
    const zd = BD_ZONES.map((name, i) => {
      const row = Math.floor(i / 3), col = i % 3, a = T.filter((t) => t.z === i), c = a.filter((t) => t.ok);
      const y = Y(rowsY[row][0]), h = Y(rowsY[row][1]) - y;
      return { i, name, row, a, c, x: col * 200, y, h, e: zoneEpa[i] };
    });
    const zones = zd.map((z) => `<button class="bd-zb${st.v === "heat" ? " h" : ""}${rEff == null && z.i === st.z ? " on" : ""}" style="left:${(z.x / 6).toFixed(2)}%;top:${((z.y / 656) * 100).toFixed(2)}%;height:${((z.h / 656) * 100).toFixed(2)}%" data-act="bd-pick" data-k="${key}" data-f="z" data-v="${z.i}" data-f2="r" data-v2="" aria-label="${esc(z.name)}">${
      st.v === "zones" ? `<span class="bd-cond">${z.c.length}/${z.a.length}</span><small class="bd-cond">${z.a.length ? bdNum(sum(z.a, (t) => t.yds) / z.a.length) : "0.0"} a throw</small>` : ""}${
      st.v === "heat" ? `<span class="st-label">${esc(z.name)}</span><b class="bd-cond${z.e < 0 ? " bad" : ""}">${z.a.length ? bdSigned(z.e) : "—"}</b><small class="bd-cond">${z.c.length}/${z.a.length} caught</small>` : ""}</button>`).join("");
    const legend = st.v === "heat"
      ? '<span><i class="bd-lg ink"></i>adds points</span><span><i class="bd-lg bad"></i>costs points</span><span class="bd-dim">brighter where it adds or costs more · numbers: points added a throw</span>'
      : `${st.v === "paths" ? '<span><i class="bd-lg line"></i>ball in the air</span><span><i class="bd-lg yac"></i>after the catch</span>' : ""}<span>● caught</span><span>○ incomplete</span><span>◎ touchdown</span><span class="bad">✕ picked off</span>`;
    // Totals.
    const C = T.filter((t) => t.ok), yds = sum(T, (t) => t.yds);
    const tot = bdCells([
      { k: "Caught", v: `${C.length}/${T.length}` },
      { k: "Rate", v: T.length ? bdPct(C.length, T.length) + "%" : "—" },
      { k: "TD-INT", v: `${T.filter((t) => t.td).length}-${T.filter((t) => t.int).length}` },
    ]);
    // Who he threw to.
    const per = x.receivers
      .map((r) => { const a = T.filter((t) => t.tgt === r.id), c = a.filter((t) => t.ok); return { ...r, a, c, y: sum(a, (t) => t.yds), air: sum(c, (t) => t.air), yac: sum(c, (t) => t.yac) }; })
      .filter((r) => r.a.length)
      .sort((a, b) => b.a.length - a.a.length);
    const MX = 125 / 53.33, MY = (d) => (42 - Math.max(-7, Math.min(41, d))) * (66 / 50);
    const rcv = per.slice(0, 8).map((r) => {
      const on = r.id === st.r;
      const dots = r.a.map((t) => `<circle class="bd-mdot${t.int ? " int" : t.ok ? " ok" : ""}" cx="${((53.33 - Math.max(0.5, Math.min(52.8, t.y1))) * MX).toFixed(1)}" cy="${MY(t.air).toFixed(1)}" r="2.1"></circle>`).join("");
      const grid = [0, 10, 20, 30].map((d) => `<line class="bd-yl${d === 0 ? " los" : ""}" x1="0" x2="125" y1="${MY(d).toFixed(1)}" y2="${MY(d).toFixed(1)}"></line>`).join("");
      return `<button class="bd-rc${on ? " on" : ""}" data-act="bd-pick" data-k="${key}" data-f="r" data-v="${on ? "" : r.id}">
        <span class="bd-line2"><b class="${bdSur(r.name).length > 9 ? "bd-cond" : ""}" title="${esc(r.name)}">${esc(bdSur(r.name))}</b><small class="bd-dim">${esc(r.pos)}</small></span>
        <svg viewBox="0 0 125 66" aria-hidden="true">${grid}${dots}</svg>
        <span class="bd-line2 bd-cond"><b>${r.c.length}/${r.a.length}</b><span>${r.a.filter((t) => t.td).length} TD</span></span><span class="bd-cond">${Math.round(r.y).toLocaleString()} ${Math.round(r.y) === 1 ? "yd" : "yds"}</span>
        <small class="bd-cond bd-dim">air ${Math.round(r.air)} · after the catch ${Math.round(r.yac)}</small></button>`;
    }).join("");
    // The detail: a receiver when one is picked, else the zone.
    const rsel = rEff != null ? per.find((r) => r.id === rEff) : null;
    const zs = zd[st.z] || zd[4];
    const a = rsel ? rsel.a : zs.a;
    const c = a.filter((t) => t.ok), rate = a.length ? c.length / a.length : 0, xp = xpOf(a), over = rate - xp;
    const thrown = zd.filter((q) => q.a.length);
    const rank = thrown.slice().sort((p1, p2) => p2.e - p1.e).findIndex((q) => q.i === zs.i) + 1;
    const cells = rsel
      ? bdCells([{ k: "Yards", v: String(Math.round(rsel.y)) }, { k: "TD", v: String(a.filter((t) => t.td).length) }, { k: "Air · after", v: `${Math.round(rsel.air)} · ${Math.round(rsel.yac)}` }])
      : bdCells([
        { k: "A throw", v: (a.length ? bdNum(sum(a, (t) => t.yds) / a.length) : "0.0") + " yds" },
        { k: "TD-INT", v: `${a.filter((t) => t.td).length}-${a.filter((t) => t.int).length}` },
        { k: "Points added", v: a.length ? bdSigned(zs.e) : "—", sub: a.length ? `${bdOrd(rank)} of ${thrown.length}` : "", bad: a.length && zs.e < 0 },
      ]);
    const splitRows = rsel
      ? ["Deep", "Middle", "Short", "Behind"].map((k, row) => { const q = a.filter((t) => Math.floor(t.z / 3) === row); return [k, q.filter((t) => t.ok).length, q.length]; })
      : per.map((r) => [bdSur(r.name), r.a.filter((t) => t.z === zs.i && t.ok).length, r.a.filter((t) => t.z === zs.i).length]).filter((q) => q[2]).sort((q1, q2) => q2[2] - q1[2]).slice(0, 4);
    const split = splitRows.map(([k, cc, aa]) => `<div class="bd-sp"><span>${esc(k)}</span><span class="bd-bar"><i style="width:${aa ? (cc / aa) * 100 : 0}%"></i></span><b class="bd-cond">${cc}/${aa}</b></div>`).join("");
    const who = x.staff ? `${coachLink(x.staff.id, x.staff.name)}, ${esc(x.staff.role)}` : "your staff";
    const result = (t) => (t.int ? "Picked off" : t.td ? "Touchdown" : t.ok ? "Caught" : t.drop ? "Dropped" : "Incomplete");
    const list = a.slice(0, 12).map((t) => `<div class="bd-tr bd-trow"${t.g != null ? ` data-go="game" data-index="${t.g}" role="link" tabindex="0" title="The game page"` : ""}><span class="bd-tgm" title="${esc(`${t.date} · ${t.opp} · ${t.q} ${t.clock}`)}"><span>${esc(t.date.split(" ").slice(1, 3).join(" "))} <small class="bd-dim">${esc(t.opp)}</small></span><small class="bd-cond bd-dim">${esc(t.q)} ${esc(t.clock)}</small></span><span class="bd-cond">${["", "1st", "2nd", "3rd", "4th"][t.d] || ""} & ${t.togo}</span><span class="bd-cond bd-r">${bdNum(t.air, 0)}${t.ok ? ` + ${bdNum(t.yac, 0)}` : ""}</span><span class="bd-r${t.int ? " bad" : ""}">${result(t)}</span></div>`).join("");
    const more = a.length > 12 ? `<small class="bd-dim">and ${a.length - 12} more</small>` : "";
    const detail = `<div class="bd-card">
      <div class="bd-line2"><span class="st-label">${esc(rsel ? rsel.name : zs.name)}</span><span class="bd-dim">${esc(rsel ? rsel.pos : BD_DEPTH[zs.row])}</span></div>
      <div class="bd-hero"><b class="bd-cond bd-big">${c.length}/${a.length}</b><span class="bd-how"><span>caught, <b class="bd-cond">${a.length ? Math.round(rate * 100) + "%" : "—"}</b></span><span class="bd-dim">${bdPct(a.length, T.length)}% of his throws went ${rsel ? "to him" : "here"}</span></span></div>
      <div class="bd-xp"><div class="bd-line2"><span>Caught against expected</span><b class="bd-cond${a.length && over < -0.04 ? " bad" : ""}">${a.length ? bdSigned(Math.round(over * 100), 0) + "%" : "—"}</b></div><span class="bd-bar"><i style="width:${(rate * 100).toFixed(1)}%"></i><em style="left:${(xp * 100).toFixed(1)}%"></em></span><small class="bd-dim">bar: caught · tick: expected for those throws (${Math.round(xp * 100)}%), from the depth, the separation and the pressure</small></div>
      ${cells}<div class="bd-sps">${split}</div>
      <div class="bd-quote">“${esc(bdThrowNote(a, rsel ? "receiver" : "zone"))}”<small>${who}</small></div>
      ${list ? `<div class="bd-tbl bd-throws"><div class="bd-tr bd-thd"><span>Game</span><span>Down</span><span class="bd-r">Air + after</span><span class="bd-r">Result</span></div>${list}</div>${more}` : ""}</div>`;
    // How deep he throws, against his level.
    const binRows = BD_BINS.map((n, i) => {
      const q = T.filter((t) => t.bin === i), cq = q.filter((t) => t.ok), share = T.length ? (100 * q.length) / T.length : 0;
      const lv = level ? level.depth[i] : null, r = q.length ? cq.length / q.length : 0, low = q.length && r < xpOf(q) - 0.04;
      return `<div class="bd-bin"><b class="bd-cond">${Math.round(share)}%</b><span class="bd-binbar"><i style="height:${Math.min(100, share * 2.6).toFixed(1)}%"></i>${lv != null ? `<em style="bottom:${Math.min(100, lv * 2.6).toFixed(1)}%" title="${esc(lvlName)}: ${lv}%"></em>` : ""}</span><small>${n}</small><span class="bd-cond${low ? " bad" : ""}">${q.length ? Math.round(r * 100) + "%" : "–"}</span></div>`;
    }).join("");
    const adot = T.length ? sum(T, (t) => t.air) / T.length : 0;
    const ck = BD_CLOCK.map(([n, lo, hi]) => ({ n, a: T.filter((t) => t.t >= lo && t.t < hi) }));
    const cmax = Math.max(1, ...ck.map((q) => q.a.length));
    const clock = ck.map(({ n, a: q }) => {
      const cq = q.filter((t) => t.ok), r = q.length ? cq.length / q.length : 0, low = q.length && r < xpOf(q) - 0.04;
      return `<div class="bd-cl"><span>${n}</span><span class="bd-clbar"><i style="width:${((q.filter((t) => !t.pr).length / cmax) * 100).toFixed(1)}%"></i><i class="pr" style="width:${((q.filter((t) => t.pr).length / cmax) * 100).toFixed(1)}%"></i></span><span class="bd-cond">${cq.length}/${q.length}</span><b class="bd-cond${low ? " bad" : ""}">${q.length ? Math.round(r * 100) + "%" : "–"}</b></div>`;
    }).join("");
    const ttt = T.length ? sum(T, (t) => t.t) / T.length : 0;
    const pressured = T.filter((t) => t.pr).length;
    const lvlNote = level ? `${esc(lvlName)} numbers from ${level.last ? `last season (${level.season})` : `the ${level.season} season`}` : `no ${esc(lvlName)} numbers yet`;
    const filmTxt = `${x.filmGames} ${x.filmGames === 1 ? "game" : "games"} of film your staff keeps on a coming opponent`;
    const seatTxt = `${x.seatGames} of your kept ${x.seatGames === 1 ? "game" : "games"}`;
    const src = x.source === "film" ? `from ${filmTxt}` : x.source === "both" ? `from ${seatTxt} and ${filmTxt}` : `from ${seatTxt}`;
    const fname = { all: "all throws", clean: "clean pocket", pressure: "under pressure", third: "third down" }[st.f];
    const filters = bdSegs(key, "f", [["all", "All throws"], ["clean", "Clean pocket"], ["pressure", "Under pressure"], ["third", "Third down"]], st.f);
    const views = bdSegs(key, "v", [["paths", "Paths"], ["heat", "Heat"], ["zones", "Zones"]], st.v);
    return `<div class="bd bd-pm">
      <div class="bd-line2 bd-pmhead"><span class="bd-ps">${x.season} season · ${T.length} ${T.length === 1 ? "throw" : "throws"} ${src}${x.throwaways ? ` · ${x.throwaways} thrown away, left out` : ""}</span>${filters}</div>
      <div class="bd-cols3">
        <section class="bd-panel bd-pml">
          <div class="bd-card"><div class="bd-line2"><span class="st-label">Passing</span><span class="bd-dim">${esc(fname)}</span></div>
            <div class="bd-hero"><b class="bd-cond bd-big">${T.length ? bdNum(yds / T.length) : "0.0"}</b><span class="bd-how"><span>yards a throw</span><span class="bd-dim"><b class="bd-cond">${Math.round(yds).toLocaleString()}</b> yards in all</span></span></div>${tot}</div>
          <div class="bd-line2"><span class="st-label">Who he threw to</span><span class="bd-dim">pick one for his throws</span></div>
          <div class="bd-rcs">${rcv || bdEmpty("No targets on film.")}</div>
        </section>
        <section class="bd-panel bd-pmc">
          <div class="bd-ph"><span><b class="bd-pt">Where the ball went</b> <span class="bd-ps">${esc(fname)}, ${T.length} ${T.length === 1 ? "throw" : "throws"}</span></span>${views}</div>
          <div class="bd-field"><svg viewBox="0 0 600 656" role="img" aria-label="Every throw from where ${esc(p.name)} threw it"><path class="bd-hash" d="${hash}"></path>${lines}<line class="bd-third" x1="200" x2="200" y1="0" y2="656"></line><line class="bd-third" x1="400" x2="400" y1="0" y2="656"></line>${nums}${marks}</svg>${zones}</div>
          <div class="bd-legend">${legend}</div>
        </section>
        <aside class="bd-panel bd-side">${detail}
          <div class="bd-card"><div class="bd-line2"><span class="st-label">How deep he throws</span><span class="bd-dim"><i class="bd-lg ink"></i>him <i class="bd-lg tick"></i>${esc(lvlName)}</span></div>
            <span class="bd-dim">Average target <b class="bd-cond">${bdNum(adot)}</b> yards past the line${level ? ` · ${esc(lvlName)} ${bdNum(level.adot)}` : ""}</span>
            <div class="bd-bins">${binRows}</div><small class="bd-dim">top: share of his throws · bottom: how many were caught · ${lvlNote}</small></div>
          <div class="bd-card"><div class="bd-line2"><span class="st-label">Pocket clock</span><span class="bd-dim"><i class="bd-lg ink"></i>clean <i class="bd-lg bad"></i>pressured</span></div>
            <span class="bd-dim"><b class="bd-cond">${bdNum(ttt, 2)}</b> seconds to throw on average${level ? ` · ${esc(lvlName)} ${bdNum(level.ttt, 2)}` : ""} · pressured on ${bdPct(pressured, T.length)}% of these throws${level ? ` (${esc(lvlName)}: ${bdNum(level.pressure)}% of dropbacks)` : ""}</span>
            <div class="bd-cls">${clock}</div></div>
        </aside>
      </div></div>`;
  },
});

// ======================================================================
// V14, where the money goes: the cap as blocks (NFL), the NIL budget and
// the scholarships (college).
// ======================================================================
// A squarified treemap: items {v} into the rectangle, each with x, y, w, h.
function bdSquarify(items, X0, Y0, W, H) {
  const list = items.filter((it) => it.v > 0).sort((a, b) => b.v - a.v);
  const total = list.reduce((s, it) => s + it.v, 0);
  if (!total || W <= 0 || H <= 0) return [];
  const k = (W * H) / total, out = [];
  let rest = list, x = X0, y = Y0, w = W, h = H;
  const worst = (row, side) => {
    const s = row.reduce((a, it) => a + it.v * k, 0), mx = row[0].v * k, mn = row[row.length - 1].v * k;
    return Math.max((side * side * mx) / (s * s), (s * s) / (side * side * mn));
  };
  while (rest.length) {
    const side = Math.min(w, h);
    let n = 1;
    while (n < rest.length && worst(rest.slice(0, n + 1), side) <= worst(rest.slice(0, n), side)) n++;
    const row = rest.slice(0, n), thick = row.reduce((a, it) => a + it.v * k, 0) / side;
    let off = 0;
    for (const it of row) {
      const len = (it.v * k) / thick;
      out.push(w >= h ? { ...it, x, y: y + off, w: thick, h: len } : { ...it, x: x + off, y, w: len, h: thick });
      off += len;
    }
    if (w >= h) { x += thick; w -= thick; } else { y += thick; h -= thick; }
    rest = rest.slice(n);
  }
  return out;
}
const BD_GROUP_SHORT = { "Offensive line": "OL", "EDGE and DT": "DL", "RB and TE": "RB/TE", Kicking: "K/P/LS" };
// The blocks: position groups, each with its men (the small ones lumped),
// then the extra blocks (dead money, open space, promises).
function bdTreemap(groups, blocks, extra, sel, cap) {
  const W = 872, H = 640;
  // The map draws about 540 px wide at the default 1280 px window (wider
  // at bigger ones); a label needs its pixels, so each test is in units of
  // that drawing.
  const K = W / 540;
  const fits = (r, w, h) => r.w >= w * K && r.h >= h * K;
  const by = groups.map((g, i) => {
    const mine = blocks.filter((b) => b.group === i).sort((a, b) => b.hit - a.hit);
    const big = mine.filter((b, k) => k < 6 && b.hit >= cap * 0.006), small = mine.filter((b) => !big.includes(b));
    const items = big.map((b) => ({ b, v: b.hit }));
    if (small.length) items.push({ lump: small, v: small.reduce((s, b) => s + b.hit, 0) });
    return { g, i, items, v: mine.reduce((s, b) => s + b.hit, 0) };
  });
  const top = [...by.filter((x) => x.v > 0), ...extra.filter((e) => e.v > 0)];
  let html = "";
  const pct = (v, d) => ((v / d) * 100).toFixed(2);
  const box = (r) => `left:${pct(r.x, W)}%;top:${pct(r.y, H)}%;width:${pct(r.w, W)}%;height:${pct(r.h, H)}%`;
  for (const t of bdSquarify(top, 0, 0, W, H)) {
    if (t.kind) {
      const r = { x: t.x + 2, y: t.y + 2, w: t.w - 4, h: t.h - 4 };
      const show = fits(r, 70, 46);
      html += `<div class="bd-tb bd-tx ${t.kind}" style="${box(r)}" title="${esc(t.title)}: ${bdMoney(t.v)}">${show ? `<span class="bd-nm">${esc(t.title)}</span><b class="bd-cond">${bdMoney(t.v)}</b>${fits(r, 120, 70) ? `<small>${esc(t.sub)}</small>` : ""}` : ""}</div>`;
      continue;
    }
    const gname = t.g;
    const hw = t.w - 4;
    // A group too small for its heading and its blocks is one block: the
    // group and its money, its men on hover.
    if (t.h < 56 || t.w < 44) {
      const r = { x: t.x + 2, y: t.y + 2, w: t.w - 4, h: t.h - 4 };
      const short = BD_GROUP_SHORT[gname] || gname;
      const txt = r.h >= 15 * K ? [`${short} · ${bdMoney(t.v)}`, short].find((s) => s.length * 6.6 * K <= r.w - 12) || "" : "";
      const men = blocks.filter((b) => b.group === t.i).sort((a, b) => b.hit - a.hit).map((q) => `${q.name} ${bdMoney(q.hit)}`);
      html += `<div class="bd-tb lump bd-tgs" style="${box(r)}" title="${esc(`${gname}: ${men.join(", ")}`)}">${txt ? `<span class="bd-cond">${esc(txt)}</span>` : ""}</div>`;
      continue;
    }
    const label = [`${gname} · ${bdMoney(t.v)}`, `${BD_GROUP_SHORT[gname] || gname} · ${bdMoney(t.v)}`, BD_GROUP_SHORT[gname] || gname].find((s) => s.length * 7.2 * K <= hw - 18) || "";
    html += `<div class="bd-tg" style="${box({ x: t.x + 2, y: t.y + 2, w: t.w - 4, h: t.h - 4 })}"><span class="st-label">${esc(label)}</span></div>`;
    for (const b of bdSquarify(t.items, t.x + 5, t.y + 24, t.w - 10, t.h - 29)) {
      const r = { x: b.x + 1.5, y: b.y + 1.5, w: b.w - 3, h: b.h - 3 };
      if (r.w <= 1 || r.h <= 1) continue;
      if (b.lump) {
        const n = b.lump.length;
        // "N more" needs its own width (about 7 a letter and the padding).
        const both = fits(r, Math.max(54, `${n} more`.length * 7 + 16), 46), sum = !both && fits(r, 48, 26);
        html += `<div class="bd-tb lump" style="${box(r)}" title="${esc(b.lump.map((q) => `${q.name} ${bdMoney(q.hit)}`).join(", "))}">${both ? `<span class="bd-nm">${n} more</span>` : ""}${both || sum ? `<b class="bd-cond">${bdMoney(b.v)}</b>` : ""}</div>`;
        continue;
      }
      const p = b.b, on = p.id === sel, left = p.left;
      const room = r.w / K - 18;
      const name = p.name.length * 6.6 <= room ? p.name : bdSur(p.name).length * 6.6 <= room ? bdSur(p.name) : "";
      // Name and money with room for both, else the money alone.
      const show = fits(r, 0, 46) && name, sum = !show && fits(r, 48, 26);
      const sub = fits(r, 120, 80) ? `${p.pos} · ${p.through ? "through " + p.through : p.cls || ""}${p.usual ? " · usual split" : ""}` : "";
      html += `<button class="bd-tb y${Math.min(3, Math.max(1, left))}${on ? " on" : ""}${p.usual ? " usual" : ""}${p.counts === false ? " out" : ""}" style="${box(r)}" data-act="bd-pick" data-k="bd:cap" data-f="sel" data-v="${p.id}" aria-label="${esc(p.name)} ${bdMoney(p.hit)}" title="${esc(`${p.name}, ${p.pos}: ${bdMoney(p.hit)}${p.counts === false ? ", outside the top 51" : ""}`)}">${show ? `<span class="bd-nm">${esc(name)}</span>` : ""}${show || sum ? `<b class="bd-cond">${bdMoney(p.hit)}</b>` : ""}${show && sub ? `<small>${esc(sub)}</small>` : ""}</button>`;
    }
  }
  return `<div class="bd-tree">${html}</div>`;
}

screens.capboard = async () => {
  const st = keep("bd:cap", { year: null, sel: null });
  const x = await api("board_cap", st.year ? { year: st.year } : {});
  if (x.level === "none") {
    title("Where the money goes");
    return `<div class="pad bd">${bdEmpty(x.why)}</div>`;
  }
  return x.level === "college" ? bdCapCollege(x, st) : bdCapNfl(x, st);
};

function bdCapNfl(x, st) {
  title("Where the money goes", String(x.year));
  const sel = x.blocks.find((b) => b.id === st.sel) || x.blocks.slice().sort((a, b) => b.hit - a.hit)[0];
  const deadSum = x.dead.reduce((s, d) => s + d.amount, 0);
  const extra = [
    { kind: "dead", title: "Dead money", sub: "from players gone", v: deadSum },
    { kind: "space", title: "Open space", sub: x.rookies > 0 ? "before the rookie pool" : "left under the cap", v: Math.max(0, x.space) },
  ];
  const years = bdSegs("bd:cap", "year", x.years.map((y) => [y.year, `${y.year} <span class="bd-cond bd-dim">${y.pct}%</span>`]), x.year, "small");
  const clocks = bdCells([
    { k: "Cap", v: bdMoney(x.cap), sub: x.carry ? `${bdMoney(x.leagueCap)} ${x.carry > 0 ? "+" : ""}${bdMoney(x.carry)} carried over` : "" },
    { k: "Committed", v: bdMoney(x.committed) },
    { k: "Space", v: bdMoney(x.space), bad: x.space < 0 },
    ...(x.rookies > 0 ? [{ k: "Rookie pool", v: bdMoney(x.rookies), sub: "your picks at their slots" }, { k: "After rookies", v: bdMoney(x.afterRookies), bad: x.afterRookies < 0 }] : []),
  ]);
  let card = bdEmpty(`Nobody is under contract for ${x.year} yet.`);
  if (sel) {
    const topBar = sel.topAtPos > 0 ? Math.min(100, (100 * sel.hit) / sel.topAtPos) : 0;
    const moves = sel.moves.map((m) => `<div class="bd-mv"><span>${esc(m.k)}${m.noTrade ? ' <small class="bad">no-trade clause</small>' : ""}</span><span class="bd-cond${m.saves < 0 ? " bad" : ""}">${bdMoney(m.saves)}</span><span class="bd-cond${m.now > 0 ? " bad" : ""}">${bdMoney(m.now)}</span><span class="bd-cond">${bdMoney(m.later)}</span></div>`).join("");
    card = `<div class="bd-card">
      <div class="bd-line2"><span class="st-label">${esc(sel.pos)} · age ${sel.age}${sel.squad ? " · practice squad" : ""}</span><span class="bd-dim">signed through ${sel.through}</span></div>
      <span class="bd-mtitle">${playerLink(sel.id, sel.name)}</span>
      <div class="bd-hero"><b class="bd-cond bd-big">${bdMoney(sel.hit)}</b><span class="bd-how"><span>cap hit in ${x.year}</span><span class="bd-dim">${sel.pct}% of the cap</span></span></div>
      <div class="bd-xp"><span class="bd-bar"><i style="width:${topBar.toFixed(1)}%"></i></span><small class="bd-dim">${Math.round(topBar)}% of the biggest ${esc(sel.pos)} cap hit in the league (${bdMoney(sel.topAtPos)})</small></div>
      ${bdCells([{ k: "Salary", v: bdMoney(sel.salary) }, { k: "Bonus", v: bdMoney(sel.bonus), sub: `${sel.bonusShare}% of the hit` }, { k: "Guaranteed", v: bdMoney(sel.gtd), sub: "still to pay" }])}
      <div class="bd-kv"><span>Your staff's read</span>${range(sel.read, "your staff", { p: sel.id, k: "ovr" })}</div>
      <div class="bd-mvs"><div class="bd-mv bd-thd"><span>If you move him in ${x.year}</span><span>Saves</span><span>Dead now</span><span>Dead later</span></div>${moves}</div>
      <div class="bd-links"><a class="link" data-go="player" data-id="${sel.id}" data-tab="Contract">His contract ›</a><a class="link" data-go="contracts">Contracts ›</a></div></div>`;
  }
  const e = x.ending;
  const ending = `<div class="bd-card"><div class="bd-line2"><span class="st-label">Contracts ending after ${e.year}</span><span class="bd-dim">${e.count} ${e.count === 1 ? "player" : "players"}</span></div>
    <div class="bd-hero"><b class="bd-cond bd-big">${bdMoney(e.sum)}</b><span class="bd-how"><span>comes off the books for ${e.year + 1}</span></span></div>
    <div class="bd-ens">${e.top.map((p) => `<div class="bd-en"><span>${playerLink(p.id, p.name)} <small class="bd-dim">${esc(p.pos)}, ${p.age}</small></span>${range(p.read, "your staff", { p: p.id, k: "ovr" })}<span class="bd-cond bd-r">${bdMoney(p.hit)}</span></div>`).join("") || '<div class="bd-dim">Nobody.</div>'}</div></div>`;
  // The twelve biggest charges by name, the rest summed in one line.
  const deadTop = x.dead.slice().sort((a, b) => b.amount - a.amount), deadRest = deadTop.splice(12);
  const dead = x.dead.length ? `<div class="bd-deads"><span class="st-label">Dead money in ${x.year}</span>${deadTop.map((d) => `<span>${d.id != null ? playerLink(d.id, d.name) : esc(d.name)}${d.void ? ' <small class="bd-dim">void years</small>' : d.incentive ? ' <small class="bd-dim">incentive</small>' : ""} <b class="bd-cond${d.credit ? "" : " bad"}">${bdMoney(d.amount)}</b></span>`).join("")}${deadRest.length ? `<span class="bd-dim">and ${deadRest.length} more, <b class="bd-cond">${bdMoney(deadRest.reduce((s, d) => s + d.amount, 0))}</b> in all</span>` : ""}</div>` : "";
  const over = x.space < 0 ? `<div class="bd-note bad">Over the cap by ${bdMoney(-x.space)}: a team must be under it by the start of the league year.</div>` : "";
  return `<div class="pad bd bd-cap">
    <header class="bd-head"><span class="bd-crest">${mark(x.team, 40)}</span><div class="bd-head-copy"><span class="st-label">${esc(x.team.name)} · ${x.year} league year${x.capped ? "" : " · no cap this year"}</span><h1 class="bd-h1">Where the money goes</h1></div>${clocks}</header>
    ${over}
    <div class="bd-cols">
      <section class="bd-panel">
        <div class="bd-ph"><span><b class="bd-pt">Cap by position</b> <span class="bd-ps">each block a player's cap hit · shade: years left on his deal</span></span><span class="bd-yrs"><span class="bd-dim">Cap already signed</span>${years}</span></div>
        ${bdTreemap(x.groups, x.blocks, extra, sel && sel.id, x.cap)}
        <div class="bd-legend"><span>Signed for</span><span><i class="bd-lg y1"></i>this year only</span><span><i class="bd-lg y2"></i>two years</span><span><i class="bd-lg y3"></i>three or more</span><span><i class="bd-lg dead"></i>dead money</span><span><i class="bd-lg space"></i>open space</span>${x.top51 ? '<span><i class="bd-lg out"></i>outside the top 51</span>' : ""}</div>
        ${x.top51 ? `<small class="bd-dim">Until the first game only the 51 biggest cap hits count: ${bdMoney(x.outside)} sits outside them.</small>` : ""}
        ${dead}
      </section>
      <aside class="bd-panel bd-side">${card}${ending}</aside>
    </div></div>`;
}

function bdCapCollege(x, st) {
  title("NIL budget", String(x.moneyYear));
  const b = x.budget, sch = x.scholarships;
  const sel = x.blocks.find((q) => q.id === st.sel) || x.blocks.slice().sort((p, q) => q.hit - p.hit)[0];
  const extra = [
    { kind: "space", title: "Promised to recruits", sub: "the signing class's deals", v: b.recruits },
    { kind: "space", title: "Portal targets", sub: "set aside for transfers", v: b.portal },
    { kind: b.left < 0 ? "dead" : "space", title: b.left < 0 ? "Over the budget" : "Left", sub: b.left < 0 ? "more promised than there is" : "not yet promised", v: Math.abs(b.left) },
  ];
  const clocks = bdCells([
    { k: "Budget", v: bdMoney(b.total) },
    b.usualRoster != null
      ? { k: "To the roster", v: bdMoney(b.usualRoster), sub: x.budget.yours ? "the usual split, no deals set" : "the collective's usual split" }
      : { k: "To the roster", v: bdMoney(b.roster) },
    { k: "Promised", v: bdMoney(b.recruits + b.portal), sub: "recruits and the portal" },
    b.leftUsual != null
      ? { k: "Left", v: bdMoney(b.left), sub: `${bdMoney(b.leftUsual)} with the usual split`, bad: b.left < 0 }
      : { k: "Left", v: bdMoney(b.left), bad: b.left < 0 },
  ]);
  // Under the House settlement the limit counts the whole roster but its
  // designated walk-ons.
  const over = sch.whole && sch.counted > sch.cap;
  const rosterLine = sch.whole
    ? `<span class="bd-dim${over ? " bad" : ""}">${sch.counted} of ${sch.cap} roster places used${sch.roster > sch.counted ? ` · ${sch.roster - sch.counted} designated walk-ons outside the limit` : ""}</span>`
    : `<span class="bd-dim">${sch.walkOns} walk-ons · ${sch.roster} on the roster</span>`;
  const aidHow = sch.aid === "none" ? "no athletic aid in Division III" : sch.aid === "equivalency" ? `on aid · ${sch.equiv} equivalencies, shared out` : `on aid${sch.limit ? "" : " · no scholarship limit"}`;
  const schCard = `<div class="bd-card"><div class="bd-line2"><span class="st-label">Scholarships, ${sch.season}</span><span class="bd-dim">${esc(x.level_name)}</span></div>
    <div class="bd-hero"><b class="bd-cond bd-big${(sch.limit && sch.onAid > sch.limit) || over ? " bad" : ""}">${sch.aid === "none" ? "—" : `${sch.onAid}${sch.limit ? `/${sch.limit}` : ""}`}</b><span class="bd-how"><span>${aidHow}</span>${rosterLine}</span></div>
    ${sch.limit ? `<div class="bd-xp"><span class="bd-bar"><i style="width:${Math.min(100, (100 * sch.onAid) / sch.limit).toFixed(1)}%"></i></span></div>` : ""}
    ${bdCells([{ k: "Signees allowed", v: sch.signees ? String(sch.signees) : "no limit" }, { k: "Open now", v: sch.limit ? String(Math.max(0, sch.limit - sch.onAid)) : "—", bad: sch.limit && sch.onAid > sch.limit }])}
    <div class="bd-links"><a class="link" data-go="recruiting">Recruiting ›</a><a class="link" data-go="roster">Squad ›</a></div></div>`;
  if (!x.era) {
    return `<div class="pad bd bd-cap"><header class="bd-head"><span class="bd-crest">${mark(x.team, 40)}</span><div class="bd-head-copy"><span class="st-label">${esc(x.team.name)} · ${x.moneyYear}</span><h1 class="bd-h1">Scholarships</h1></div></header>
      <div class="bd-cols"><section class="bd-panel">${bdEmpty("Players can't be paid for their name, image and likeness yet: that starts on 1 July 2021. Until then the money is the scholarship.")}</section><aside class="bd-panel bd-side">${schCard}</aside></div></div>`;
  }
  let card = bdEmpty("Nobody on the roster has a deal yet.");
  if (sel) {
    card = `<div class="bd-card">
      <div class="bd-line2"><span class="st-label">${esc(sel.pos)} · ${esc(sel.cls)}${sel.walkOn ? " · walk-on" : ""}</span><span class="bd-dim">${sel.left} ${sel.left === 1 ? "season" : "seasons"} of eligibility left</span></div>
      <span class="bd-mtitle">${playerLink(sel.id, sel.name)}</span>
      <div class="bd-hero"><b class="bd-cond bd-big">${bdMoney(sel.hit)}</b><span class="bd-how"><span>his deal for ${x.moneyYear}</span><span class="bd-dim">${sel.pct}% of the budget${sel.usual ? " · the collective's usual split, no deal set yet" : ""}</span></span></div>
      ${bdCells([{ k: "NIL income", v: bdMoney(sel.income), sub: "all his deals so far" }, { k: "Eligibility", v: `${sel.left} left` }, { k: "Your read", html: range(sel.read, "your staff", { p: sel.id, k: "ovr" }) }])}
      <div class="bd-links"><a class="link" data-go="player" data-id="${sel.id}">His page ›</a><a class="link" data-go="nil_desk">NIL deals ›</a><a class="link" data-go="money">Finances ›</a></div></div>`;
  }
  const budget = `<div class="bd-card"><div class="bd-line2"><span class="st-label">Where the budget comes from</span><span class="bd-dim">${x.house ? "revenue sharing era" : "collective era"}</span></div>
    ${[["Revenue share", b.revenue], ["The collective", b.collective]].map(([k, v]) => `<div class="bd-kv"><span>${k}</span><b class="bd-cond">${bdMoney(v)}</b></div>`).join("")}
    <span class="st-label">Where it goes</span>
    ${[["The roster", b.usualRoster ?? b.roster], ["Recruits", b.recruits], ["Portal", b.portal], ["Left", b.leftUsual ?? b.left]].map(([k, v]) => `<div class="bd-kv"><span>${k}</span><span class="bd-kvbar"><i style="width:${b.total > 0 ? Math.max(0, Math.min(100, (100 * v) / b.total)).toFixed(1) : 0}%"></i></span><b class="bd-cond${v < 0 ? " bad" : ""}">${bdMoney(v)}</b></div>`).join("")}
    ${b.usual ? `<small class="bd-dim">No deals set yet for ${x.moneyYear}: the blocks show the collective's usual split. On 1 July only the deals you set are paid; <a class="link" data-go="nil_desk">set them</a> or hand NIL to your staff.</small>` : b.yours ? "" : '<small class="bd-dim">Your staff hands out the NIL money: the collective\'s usual split.</small>'}</div>`;
  return `<div class="pad bd bd-cap">
    <header class="bd-head"><span class="bd-crest">${mark(x.team, 40)}</span><div class="bd-head-copy"><span class="st-label">${esc(x.team.name)} · NIL money year ${x.moneyYear}</span><h1 class="bd-h1">Where the money goes</h1></div>${clocks}</header>
    <div class="bd-cols">
      <section class="bd-panel">
        <div class="bd-ph"><span><b class="bd-pt">NIL by position</b> <span class="bd-ps">each block a player's deal · shade: seasons of eligibility left</span></span></div>
        ${bdTreemap(x.groups, x.blocks, extra, sel && sel.id, b.total || 1)}
        <div class="bd-legend"><span>Eligibility left</span><span><i class="bd-lg y1"></i>one season</span><span><i class="bd-lg y2"></i>two</span><span><i class="bd-lg y3"></i>three or more</span><span><i class="bd-lg space"></i>promised and left</span><span><i class="bd-lg dead"></i>over the budget</span></div>
      </section>
      <aside class="bd-panel bd-side">${card}${budget}${schCard}</aside>
    </div></div>`;
}

// ======================================================================
// V15, the locker room: who's close to whom, moods, leaders, friction.
// ======================================================================
const BD_LOCK_TABS = ["Links", "Groups", "Morale", "Leaders"];
const BD_LOCK_VIEWS = [["all", "All links"], ["f", "Friends"], ["m", "Mentors"], ["t", "Friction"]];
const BD_LOCK_KIND = { all: "cfmt", f: "cf", m: "m", t: "t" };
const bdShortName = (n) => (n.length > 14 ? `${n[0]}. ${bdSur(n)}` : n);
const bdLockSub = (n) => `${n.pos} · ${String(n.cls || "").replace("redshirt ", "r-")}${n.cap ? " · captain" : ""}`;

// A steady layout on six columns: the line and kicking on the left, the
// backs and receivers and the defense on the right, each group on its own
// rows under its name, so no two men ever cover each other. Inside a
// group, each next man is the one with most links to those already
// placed, so linked men sit side by side. The map draws about 540 px
// wide at the default window; the dots keep their pixel size.
function bdLockLayout(nodes, edges, W, H) {
  const k0 = W / 540, LBL = 24, CW = W / 6;
  const tie = (i, j) => edges.some((e) => (e.a === i && e.b === j) || (e.a === j && e.b === i));
  const by = [0, 1, 2, 3].map((g) => {
    const left = nodes.map((n, i) => (n.g === g ? i : -1)).filter((i) => i >= 0), out = [];
    while (left.length) {
      let best = 0, most = -1;
      left.forEach((i, k) => { const v = out.filter((j) => tie(i, j)).length; if (v > most) { most = v; best = k; } });
      out.push(left.splice(best, 1)[0]);
    }
    return out;
  });
  const rows = (n, c) => (n ? Math.ceil(n / c) : 0);
  // Split the six columns so neither side runs out of rows.
  let cl = 2, worst = 1e9;
  for (let c = 1; c <= 5; c++) {
    const a = rows(by[0].length, c) + rows(by[3].length, c), b = rows(by[1].length, 6 - c) + rows(by[2].length, 6 - c);
    const m = Math.max(a, b) * 10 + Math.abs(c - 2);
    if (m < worst) { worst = m; cl = c; }
  }
  const P = new Array(nodes.length);
  const side = (gs, c0, nc) => {
    const live = gs.filter((g) => by[g].length);
    const total = live.reduce((s, g) => s + rows(by[g].length, nc), 0);
    const ch = Math.min(104, (H - LBL * live.length - 8) / Math.max(1, total));
    const gap = (H - LBL * live.length - total * ch) / (live.length + 1);
    let y = gap;
    for (const g of live) {
      y += LBL;
      by[g].forEach((i, k) => {
        const row = Math.floor(k / nc), col = k % nc, inRow = Math.min(nc, by[g].length - row * nc);
        const s = 22 + nodes[i].say * 4;
        P[i] = { x: (c0 + (nc - inRow) / 2 + col + 0.5) * CW, y: y + row * ch + (s * k0) / 2 + 2, w: CW - 10, h: ch, s: s * k0, d: s, g };
      });
      y += rows(by[g].length, nc) * ch + gap;
    }
  };
  side([0, 3], 0, cl);
  side([1, 2], cl, 6 - cl);
  return P;
}

function bdLockMap(x, st, sel) {
  const W = 872, H = 600;
  const P = bdLockLayout(x.nodes, x.edges, W, H);
  const kinds = BD_LOCK_KIND[st.f] || BD_LOCK_KIND.all;
  const near = new Set([sel]);
  for (const e of x.edges) { if (e.a === sel) near.add(e.b); if (e.b === sel) near.add(e.a); }
  const lines = x.edges.filter((e) => kinds.includes(e.k)).map((e) => {
    const a = P[e.a], b = P[e.b], mine = e.a === sel || e.b === sel;
    return `<line class="bd-le ${e.k}${mine ? " on" : ""}" x1="${a.x.toFixed(1)}" y1="${a.y.toFixed(1)}" x2="${b.x.toFixed(1)}" y2="${b.y.toFixed(1)}"><title>${esc(`${x.nodes[e.a].name} and ${x.nodes[e.b].name}: ${e.why}`)}</title></line>`;
  }).join("");
  // Group names above each cluster.
  const box = x.groups.map(() => null);
  P.forEach((p) => {
    const b = box[p.g] || { x0: 1e9, y0: 1e9, x1: -1e9 };
    b.x0 = Math.min(b.x0, p.x - p.w / 2); b.y0 = Math.min(b.y0, p.y - p.s / 2); b.x1 = Math.max(b.x1, p.x + p.w / 2);
    box[p.g] = b;
  });
  const gl = box.map((b, g) => (b ? `<span class="bd-lg-name st-label" style="left:${((Math.max(0, b.x0) / W) * 100).toFixed(2)}%;top:${((Math.max(0, b.y0 - 22) / H) * 100).toFixed(2)}%">${esc(x.groups[g])}</span>` : "")).join("");
  const dots = x.nodes.map((n, i) => {
    const p = P[i], on = i === sel;
    const ini = n.name.split(" ").filter(Boolean).map((w) => w[0]).slice(0, 2).join("");
    const sub = bdLockSub(n);
    return `<button class="bd-nd${on ? " on" : ""}${near.has(i) ? "" : " far"}" style="left:${((p.x / W) * 100).toFixed(2)}%;top:${(((p.y - p.s / 2) / H) * 100).toFixed(2)}%" data-act="bd-pick" data-k="bd:lock" data-f="sel" data-v="${n.id}" aria-label="${esc(`${n.name}, ${sub}, ${x.moods[n.m - 1]}`)}">
      <span class="bd-dot m${n.m}" style="width:${p.d}px;height:${p.d}px;font-size:${Math.round(p.d / 3.2)}px">${esc(ini)}</span>
      <span class="bd-ndn">${esc(bdShortName(n.name))}</span><span class="bd-nds">${esc(sub)}</span></button>`;
  }).join("");
  return `<div class="bd-map"><svg viewBox="0 0 ${W} ${H}" preserveAspectRatio="none" aria-hidden="true">${lines}</svg>${gl}${dots}</div>
    <div class="bd-legend"><span><i class="bd-lk c"></i>close friends</span><span><i class="bd-lk f"></i>friends</span><span><i class="bd-lk m"></i>mentor</span><span><i class="bd-lk t"></i>friction</span><span class="bd-dim">Brighter dot = happier · amber edge = unhappy · bigger = more say in the room</span></div>`;
}

function bdMoodSteps(m) {
  return `<span class="bd-steps${m <= 2 ? " bad" : ""}">${[1, 2, 3, 4, 5].map((k) => `<i${k <= m ? ' class="on"' : ""}></i>`).join("")}</span>`;
}

function bdLockCard(x, n) {
  if (!n) return bdEmpty("Pick a man on the map.");
  const links = n.links.map((l) => `<div class="bd-lkrow"><span class="bd-ell">${playerLink(l.id, l.name)} <span class="bd-dim">${esc(l.why)}</span></span><span class="bd-lkk ${l.k}">${esc(l.k)}</span></div>`).join("") || '<div class="bd-dim">No close ties on the team.</div>';
  const why = n.why.map((w) => `<div class="bd-why"><b class="${w.s === "+" ? "" : "bad"}">${esc(w.s)}</b><span>${esc(w.t)}</span></div>`).join("") || '<div class="bd-dim">Nothing stands out.</div>';
  return `<div class="bd-card">
    <div class="bd-line2"><span class="st-label">${esc(n.pos)} · ${esc(n.cls)}${n.cap ? " · captain" : ""}${n.starter ? " · starter" : ""}</span><span class="bd-dim">${esc(n.home || "")}</span></div>
    <span class="bd-mtitle">${playerLink(n.id, n.name)}</span>
    <div class="bd-hero"><b class="bd-big${n.m <= 2 ? " bad" : ""}">${esc(n.mood)}</b><span class="bd-how"><span>morale, ${esc(n.trend)}</span>${bdMoodSteps(n.m)}</span></div>
    ${bdCells(n.cells)}
    <div class="bd-list"><span class="st-label">Why</span>${why}</div>
    <div class="bd-list"><span class="st-label">His links</span>${links}</div>
    <div class="bd-links"><a class="link" data-go="player" data-id="${n.id}">His page ›</a><a class="link" data-act="pptalkmenu" data-id="${n.id}">Talk to him ›</a></div></div>`;
}

function bdLockNotes(x) {
  if (!x.notes.length) return "";
  return `<div class="bd-card"><span class="st-label">Worth knowing</span>${x.notes.map((w) => {
    const sel = w.sel != null && x.nodes[w.sel] ? x.nodes[w.sel].id : null;
    const act = w.act.k === "talk"
      ? `<button class="bd-act" data-act="pptalk" data-id="${w.act.id}" data-s="${esc(w.act.s)}">${esc(w.act.label)}</button>`
      : `<a class="bd-act" data-go="player" data-id="${w.act.id}">${esc(w.act.label)}</a>`;
    return `<div class="bd-wk"${sel != null ? ` data-act="bd-pick" data-k="bd:lock" data-f="sel" data-v="${sel}" role="button" tabindex="0"` : ""}>
      <span class="bd-wkc"><span><span class="st-label${w.warn ? " bad" : ""}">${esc(w.tag)}</span> <b>${esc(w.t)}</b></span><span class="bd-dim">${esc(w.s)}</span></span>${act}</div>`;
  }).join("")}</div>`;
}

function bdLockRows(x, rows, st) {
  const head = `<div class="bd-mr bd-thd"><span>Player</span><span>Pos</span><span>Class</span><span>Mood</span><span>Snaps</span><span>Friends</span><span>Say</span><span>Why</span></div>`;
  return head + rows.map((r) => `<div class="bd-mr${r.id === st.sel ? " on" : ""}"${r.shown != null ? ` data-act="bd-pick" data-k="bd:lock" data-f="sel" data-v="${r.id}" role="button" tabindex="0"` : ""}>
    <span class="bd-ell">${playerLink(r.id, r.name)}${r.cap ? ' <small class="bd-dim">C</small>' : ""}</span><span>${esc(r.pos)}</span><span class="bd-dim bd-ell">${esc(r.cls)}</span>
    <span class="bd-mm${r.m <= 2 ? " bad" : ""}">${bdMoodSteps(r.m)} ${esc(x.moods[r.m - 1])}</span><span class="bd-cond bd-snp">${esc(r.snaps)}${r.snapsSub ? `<small class="bd-dim">${esc(r.snapsSub)}</small>` : ""}</span><span class="bd-cond">${r.friends}</span><span>${esc(x.say[r.say - 1])}</span><span class="bd-dim bd-clamp" title="${esc(r.why)}">${esc(r.why)}</span></div>`).join("");
}

screens.locker = async (a) => {
  const x = await api("board_locker");
  title("Locker room");
  if (x.none) return `<div class="pad bd">${bdEmpty("Nobody is on the roster yet.")}</div>`;
  const tab = tabOf("bd:lockt", BD_LOCK_TABS, a);
  const st = keep("bd:lock", { sel: null, f: "all", room: null });
  let si = x.nodes.findIndex((n) => n.id === st.sel);
  if (si < 0) {
    const f = x.notes.find((w) => w.sel != null);
    si = f ? f.sel : 0;
  }
  const sel = x.nodes[si];
  const chem = !x.chem.length ? "" : bdCells(x.chem.map((c) => ({ k: c.n, v: c.v == null ? "—" : String(c.v), warn: c.v != null && c.v < 60, tip: `How well ${c.of} know each other, 0-100` })));
  const after = x.after ? `, after ${esc(x.after)}` : "";
  let main;
  if (tab === "Links") {
    main = `<div class="bd-ph"><span><b class="bd-pt">Links</b> <span class="bd-ps">${x.shown} of ${x.roster} shown: captains, friction, anyone with trouble, then the starters</span></span>${bdSegs("bd:lock", "f", BD_LOCK_VIEWS, st.f, "small")}</div>${bdLockMap(x, st, si)}`;
  } else if (tab === "Groups") {
    const cards = x.groups.map((g, gi) => {
      const rooms = x.rooms.filter((r) => r.g === gi);
      if (!rooms.length) return "";
      return `<div class="bd-rooms"><span class="st-label">${esc(g)}</span>${rooms.map((r) => {
        const men = x.rows.filter((m) => m.pos === r.pos);
        return `<div class="bd-room${r.low ? " low" : ""}">
          <div class="bd-line2"><b>${esc(r.pos)}</b><span class="bd-dim">${r.n} ${r.n === 1 ? "man" : "men"}</span></div>
          ${bdCells([{ k: "Mood", v: r.mood, warn: r.low }, { k: "Unhappy", v: String(r.unhappy), warn: r.unhappy > 0 }, { k: "Ties inside", v: String(r.links) }, { k: "Starters' chemistry", v: r.chem == null ? "—" : String(r.chem), warn: r.chem != null && r.chem < 60 }])}
          <div class="bd-men">${men.map((m) => `<span class="bd-man${m.m <= 2 ? " bad" : ""}">${playerLink(m.id, m.name)}${m.starter ? ' <small class="bd-dim">starter</small>' : ""}</span>`).join("")}</div></div>`;
      }).join("")}</div>`;
    }).join("");
    main = `<div class="bd-ph"><span><b class="bd-pt">Groups</b> <span class="bd-ps">each position room: its mood, the ties inside it and how well its starters know each other</span></span></div><div class="bd-roomgrid">${cards}</div>`;
  } else if (tab === "Morale") {
    const rows = x.rows.slice().sort((p, q) => p.m - q.m || q.say - p.say);
    const counts = x.moods.map((m, k) => `${x.rows.filter((r) => r.m === k + 1).length} ${m.toLowerCase()}`).join(" · ");
    main = `<div class="bd-ph"><span><b class="bd-pt">Morale</b> <span class="bd-ps">${esc(counts)}</span></span></div><div class="bd-mtable">${bdLockRows(x, rows, st)}</div>`;
  } else {
    const caps = x.rows.filter((r) => r.cap), council = x.rows.filter((r) => r.council && !r.cap);
    const voices = x.rows.filter((r) => !r.cap && !r.council).sort((p, q) => q.say - p.say).slice(0, 12);
    const card = (r) => `<div class="bd-lead${r.m <= 2 ? " low" : ""}"><span class="st-label">${esc(r.pos)} · ${esc(r.cls)}</span><b>${playerLink(r.id, r.name)}</b><span class="bd-dim">${esc(x.moods[r.m - 1])} · ${esc(x.say[r.say - 1])} say</span></div>`;
    main = `<div class="bd-ph"><span><b class="bd-pt">Leaders</b> <span class="bd-ps">who the room listens to</span></span><a class="link" data-go="people" data-tab="Rules">Team rules and the council ›</a></div>
      <span class="st-label">Captains</span><div class="bd-leads">${caps.map(card).join("") || '<span class="bd-dim">No captains named yet.</span>'}</div>
      <span class="st-label">Leadership council</span><div class="bd-leads">${council.map(card).join("") || '<span class="bd-dim">No one else on the council.</span>'}</div>
      <span class="st-label">The loudest other voices</span><div class="bd-mtable">${bdLockRows(x, voices, st)}</div>`;
  }
  return `<div class="pad bd bd-lock">
    <header class="bd-head"><span class="bd-crest">${mark(x.team, 40)}</span><div class="bd-head-copy"><span class="st-label">${esc(x.team.name)} · the locker room · ${esc(x.date)}${after}</span><h1 class="bd-h1">Who's close to whom</h1></div>
      ${chem ? `<div class="bd-headr"><span class="st-label">Chemistry</span>${chem}</div>` : ""}</header>
    <div class="bd-tabrow">${tabs("bd:lockt", BD_LOCK_TABS, tab)}<span class="bd-dim">The room is ${esc(x.mood.words)} · <a class="link" data-go="people" data-tab="Room">People ›</a></span></div>
    <div class="bd-cols">
      <section class="bd-panel">${main}</section>
      <aside class="bd-panel bd-side">${bdLockCard(x, sel)}${bdLockNotes(x)}</aside>
    </div></div>`;
};

// ======================================================================
// V16, the race: college polls, the NFL's playoff picture, state ranks.
// ======================================================================
const BD_POLL_H1 = { college: "The poll race", nfl: "The playoff picture", hs: "State rankings" };

function bdTable(t) {
  if (!t || !t.rows) return "";
  if (!t.rows.length) return '<div class="bd-dim bd-pad">No games yet this season: the table fills in from the first one.</div>';
  return `<div class="bd-std"><div class="bd-sr bd-thd" style="--n:${t.cols.length}"><span></span><span>Team</span>${t.cols.map((c) => `<span class="bd-r">${esc(c)}</span>`).join("")}</div>
    ${t.rows.map((r) => `<div class="bd-sr${r.me ? " me" : ""}" style="--n:${t.cols.length}"><span class="bd-cond bd-dim">${r.place}</span><span class="bd-ell">${bdTeam(r.t)}</span>${r.cells.map((c) => `<span class="bd-cond bd-r">${esc(c)}</span>`).join("")}</div>`).join("")}</div>`;
}

function bdPollChart(x, st, sel) {
  const c = x.chart, n = c.cols.length, out = c.out != null;
  const rows = c.depth + (out ? 1 : 0);
  const rh = Math.min(34, 600 / rows);
  const W = 900, L = 44, R = n > 1 ? 716 : 380, T = 26;
  const X = (i) => (n > 1 ? L + (i * (R - L)) / (n - 1) : L + 40);
  const NR = T + c.depth * rh + 6;
  const Y = (r) => (r > 0 ? T + (r - 1) * rh : NR);
  const H = Math.round((out ? NR : T + (c.depth - 1) * rh) + 30);
  const hl = st.hl ?? 0;
  const lastOf = (l) => l.r[l.r.length - 1];
  const inHl = (l) => hl === 2 || (hl === 0 ? l.hl : x.level === "nfl" ? lastOf(l) > 0 && lastOf(l) <= c.cut + 3 : lastOf(l) > 0 && lastOf(l) <= 10);
  let grid = "";
  for (let r = 1; r <= c.depth; r++) {
    if (rh >= 20 || r === 1 || r % 5 === 0) grid += `<text class="bd-axis" x="${L - 14}" y="${(Y(r) + 4).toFixed(1)}" text-anchor="end">${r}</text>`;
  }
  if (out) grid += `<text class="bd-axis" x="${L - 14}" y="${(NR + 4).toFixed(1)}" text-anchor="end">${esc(c.out)}</text><line class="bd-q" x1="${L}" x2="${R}" y1="${(NR - rh / 2 - 2).toFixed(1)}" y2="${(NR - rh / 2 - 2).toFixed(1)}"></line>`;
  // A long season: "WK" on the first column only.
  c.cols.forEach((t, i) => { grid += `<text class="bd-axis" x="${X(i).toFixed(1)}" y="12" text-anchor="middle">${esc(i > 0 && n > 10 ? String(t).replace(/^WK\s*/i, "") : t)}</text>`; });
  if (c.cut) {
    const y = (Y(c.cut) + Y(c.cut + 1)) / 2;
    grid += `<line class="bd-cut" x1="${L - 6}" x2="${R + 8}" y1="${y.toFixed(1)}" y2="${y.toFixed(1)}"></line><text class="bd-axis" x="${R + 12}" y="${(y + 4).toFixed(1)}">playoff line</text>`;
  }
  let lines = "", hits = "", labels = "", mine = "";
  const order = c.lines.slice().sort((a, b) => (inHl(a) ? 1 : 0) - (inHl(b) ? 1 : 0));
  // Each placed name's span across (chart units, about 9.5 a letter at the
  // default window) and its row: a name is left off where it would cover one.
  const used = [];
  const span = (s) => s.length * 9.5 + 12;
  // The letters on your line keep their spots too.
  const mine0 = c.lines.find((l) => l.me);
  for (const m of x.moves || []) used.push([X(m.col) - 16, X(m.col) + 16, (mine0 ? Y(mine0.r[m.col]) : NR) - 16]);
  for (const l of order) {
    const on = l.id === sel, h = inHl(l);
    const pts = l.r.map((v, i) => [i, v]).filter((q) => q[1] > 0);
    if (l.me) {
      for (let i = 0; i + 1 < n; i++) {
        if (!(l.r[i] > 0 || l.r[i + 1] > 0)) continue;
        mine += `<line class="bd-pl me${l.r[i] > 0 && l.r[i + 1] > 0 ? "" : " dash"}" x1="${X(i).toFixed(1)}" y1="${Y(l.r[i]).toFixed(1)}" x2="${X(i + 1).toFixed(1)}" y2="${Y(l.r[i + 1]).toFixed(1)}"></line>`;
      }
      l.r.forEach((v, i) => { mine += `<circle class="bd-pd me${v > 0 ? "" : " nr"}" cx="${X(i).toFixed(1)}" cy="${Y(v).toFixed(1)}" r="4"></circle>`; });
    } else if (pts.length) {
      const p = pts.map(([i, v]) => `${X(i).toFixed(1)},${Y(v).toFixed(1)}`).join(" ");
      const cls = on ? "on" : h ? "hl" : "";
      lines += `<polyline class="bd-pl ${cls}" points="${p}"></polyline>${pts.map(([i, v]) => `<circle class="bd-pd ${cls}" cx="${X(i).toFixed(1)}" cy="${Y(v).toFixed(1)}" r="${on ? 3.5 : h ? 2.6 : 2}"></circle>`).join("")}`;
      hits += `<polyline class="bd-hit" points="${p}" data-act="bd-pick" data-k="bd:poll" data-f="sel" data-v="${l.id}"><title>${esc(l.name)}</title></polyline>`;
    }
    // The name at its last point, when there's room.
    const lp = l.me && !pts.length ? [n - 1, 0] : pts[pts.length - 1];
    if (!lp) continue;
    const gone = lp[0] < n - 1;
    if (!l.me && !on && (gone ? !h : false)) continue;
    const lx = X(lp[0]) + (l.me ? 12 : 8), ly = Y(lp[1]);
    const name = `${l.name}${gone && lp[1] > 0 ? " · out" : ""}`;
    const lx1 = Math.min(W, lx + span(name));
    if (!l.me && !on && used.some((u) => lx < u[1] && u[0] < lx1 && Math.abs(u[2] - ly) < rh * 0.9)) continue;
    used.push([lx, lx1, ly]);
    labels += `<button class="bd-pn${l.me ? " me" : on ? " on" : h ? " hl" : ""}" style="left:${((lx / W) * 100).toFixed(2)}%;top:${((ly / H) * 100).toFixed(2)}%;max-width:${(((W - lx) / W) * 100).toFixed(2)}%" title="${esc(name)}" data-act="bd-pick" data-k="bd:poll" data-f="sel" data-v="${l.id}">${esc(name)}</button>`;
  }
  const me = c.lines.find((l) => l.me);
  const marks = (x.moves || []).map((m, k) => {
    const on = k === st.wk;
    const y = me ? Y(me.r[m.col]) : NR;
    return `<button class="bd-mk${on ? " on" : ""}" style="left:${((X(m.col) / W) * 100).toFixed(2)}%;top:${(((y - 16) / H) * 100).toFixed(2)}%" data-act="bd-pick" data-k="bd:poll" data-f="wk" data-v="${k}" aria-label="Move ${m.k}">${m.k}</button>`;
  }).join("");
  // The names' size follows the rows' drawn height (--fs, in the chart's
  // own width), so neighbours never cover each other.
  return `<div class="bd-race" style="aspect-ratio:${W}/${H};--fs:${((rh / W) * 78).toFixed(3)}cqw"><svg viewBox="0 0 ${W} ${H}" role="img" aria-label="Rank by week">${grid}${lines}${mine}${hits}</svg>${labels}${marks}</div>`;
}

function bdPollCard(x, l) {
  if (!l) return "";
  const c = x.chart, n = l.r.length;
  const now = l.r[n - 1], prev = n > 1 ? l.r[n - 2] : null;
  const head = `<div class="bd-line2"><span class="st-label">${esc(c.poll)} · ${esc(c.date || "")}</span><span class="bd-dim">${esc(l.rec)}${l.grp ? ` · ${esc(l.grp)}` : ""}</span></div><span class="bd-mtitle${l.me ? " me" : ""}">${bdTeam(l.t, l.name)}</span>`;
  if (x.level === "nfl") {
    const seed = l.seed ? `${bdOrd(l.seed)} seed` : `${bdOrd(now)}`;
    const mv = prev == null ? "" : now < prev ? `up ${prev - now} from last week` : now > prev ? `down ${now - prev} from last week` : "same as last week";
    const left = l.left.map((g) => `<div class="bd-kv"><span class="bd-ell"><span class="bd-cond bd-dim">WK ${g.w}</span> ${esc(g.o)}</span><b class="bd-cond">${esc(g.r)}</b></div>`).join("");
    return `<div class="bd-card">${head}
      <div class="bd-hero"><b class="bd-cond bd-big${l.me ? " me" : ""}${!l.seed ? " dim" : ""}">${esc(seed)}</b><span class="bd-how"><span>${l.lead ? "leads the division" : l.seed ? "a wild card" : `${bdOrd(now)} in the conference`}</span><span class="bd-dim">${esc(mv)}</span></span></div>
      ${bdCells([{ k: "Record", v: l.rec }, { k: "Games back", v: l.back > 0 && !l.seed ? String(l.back) : "—", sub: "of the last seed" }, { k: "Games left", v: String(l.left.length) }])}
      ${left ? `<div class="bd-list"><span class="st-label">Still to play</span>${left}</div>` : ""}</div>`;
  }
  const mv = now === 0 ? (prev > 0 ? `dropped out from No. ${prev}` : "not ranked") : prev == null ? "the first poll" : prev === 0 ? "new in the poll" : now < prev ? `up ${prev - now} from last time` : now > prev ? `down ${now - prev} from last time` : "same as last time";
  let ballots = "";
  if (l.b) {
    const mx = Math.max(1, ...l.b);
    const voters = l.b.reduce((s, v) => s + v, 0);
    const hi = l.b.findIndex((v) => v > 0), lo = 25 - l.b.slice().reverse().findIndex((v) => v > 0);
    const bars = l.b.map((v, k) => `<i class="${k + 1 === now ? (l.me ? "me" : "on") : k === 25 ? "nr" : ""}" style="height:${v ? Math.max(2, (v / mx) * 46).toFixed(1) : 0}px" title="${k < 25 ? `No. ${k + 1}` : "Not on the ballot"}: ${v}"></i>`).join("");
    const ticks = [[1, "No. 1"], [5, "5"], [10, "10"], [15, "15"], [20, "20"], [25, "25"], [26, "NR"]].map(([k, t]) => `<span style="left:${(((k - 0.5) / 26) * 100).toFixed(1)}%">${t}</span>`).join("");
    ballots = `<span class="bd-dim">where the ${voters} voters put them</span><div class="bd-ballot">${bars}</div><div class="bd-ticks">${ticks}</div>
      ${bdCells([{ k: "Points", v: l.pts != null ? String(l.pts) : "—" }, { k: "Highest", v: hi < 0 || hi === 25 ? "NR" : `No. ${hi + 1}` }, { k: "Lowest", v: lo >= 25 ? "NR" : `No. ${lo + 1}` }, { k: "1st place", v: String(l.firsts ?? l.b[0]) }])}`;
  } else if (x.level === "hs") {
    ballots = bdCells([{ k: "Record", v: l.rec || "—" }, { k: "Class", v: l.grp }, { k: "Ranked", v: l.true ? bdOrd(l.true) : "NR", sub: l.true > 25 ? "outside the top 25" : "" }]);
  }
  return `<div class="bd-card">${head}
    <div class="bd-hero"><b class="bd-cond bd-big${l.me ? " me" : ""}">${now > 0 ? now : esc(c.out || "NR")}</b><span class="bd-how"><span>${esc(mv)}</span></span></div>${ballots}</div>`;
}

function bdPollMoves(x, st) {
  if (!x.moves || !x.moves.length) return "";
  const me = x.chart.lines.find((l) => l.me);
  return `<div class="bd-card"><span class="st-label">Why ${esc(me ? me.name : "you")} moved</span>${x.moves.map((m, k) => {
    const on = k === st.wk;
    const res = m.game != null ? `<a class="link" data-go="game" data-index="${m.game}">${esc(m.r)}</a>` : esc(m.r);
    const bad = /^(out|down)/.test(m.ch);
    return `<div class="bd-wm${on ? " on" : ""}" data-act="bd-pick" data-k="bd:poll" data-f="wk" data-v="${k}" role="button" tabindex="0">
      <span class="bd-mk static${on ? " on" : ""}">${m.k}</span><span class="bd-cond bd-dim">${esc(x.chart.cols[m.col] || "")}</span><span class="bd-ell">${res}</span><b class="bd-cond">${esc(m.rank)}</b><span class="bd-cond${bad ? " bad" : m.ch === "same" ? " bd-dim" : ""}">${esc(m.ch)}</span>
      <span class="bd-wy">${esc(m.why)}</span></div>`;
  }).join("")}</div>`;
}

function bdPollNext(x) {
  const t = x.level === "nfl" ? "Games left" : "Chances to climb";
  const rows = (x.next || []).map((g) => `<div class="bd-nx"><span class="bd-cond bd-dim">${esc(g.d)}</span><span class="bd-ell">${g.t && g.t.ref ? teamLink(g.t, g.o) : esc(g.o)}</span><b class="bd-cond">${esc(g.r)}</b><span class="bd-dim bd-ell" title="${esc(g.s)}">${esc(g.s)}</span></div>`).join("");
  const note = x.note ? `<div class="bd-note${x.note.done ? "" : " soft"}"><span>${esc(x.note.t)}</span><b class="bd-cond">${esc(x.note.d || "")}</b>${x.note.done && x.note.d ? '<span class="bd-dim">out</span>' : ""}</div>` : "";
  return `<div class="bd-card"><span class="st-label">${t}</span>${rows || '<div class="bd-dim">No games left this season.</div>'}${note}</div>`;
}

screens.polls = async () => {
  const st = keep("bd:poll", { tab: null, sel: null, wk: null, hl: 0 });
  const x = await api("board_polls", st.tab ? { tab: st.tab } : {});
  if (x.level === "none" || x.none) {
    title("The race");
    return `<div class="pad bd">${bdEmpty(x.why || "No race to show yet.")}</div>`;
  }
  title(BD_POLL_H1[x.level]);
  const tabRow = `<div class="bd-tabrow"><div class="tabsline" role="tablist">${x.tabs.map((t) => `<button role="tab" data-act="bd-pick" data-k="bd:poll" data-f="tab" data-v="${esc(t.k)}" data-f2="sel" data-v2="" aria-selected="${t.k === x.tab}">${esc(t.n)}</button>`).join("")}</div>
    <span class="bd-dim"><a class="link" data-go="standings">Standings ›</a> · <a class="link" data-go="schedule">Schedule ›</a></span></div>`;
  const hero = bdCells(x.hero.map((h) => ({ k: h.k, v: h.v, warn: h.warn })));
  const head = `<header class="bd-head"><span class="bd-crest">${mark(x.team, 40)}</span><div class="bd-head-copy"><span class="st-label">${esc(x.head)}</span><h1 class="bd-h1">${BD_POLL_H1[x.level]}</h1></div>${hero}</header>`;
  const fbs = x.fbsOnly ? `<div class="bd-note soft">${esc(x.fbsOnly)}</div>` : "";
  if (!x.chart) {
    return `<div class="pad bd bd-poll">${head}${tabRow}${fbs}
      <div class="bd-cols"><section class="bd-panel">${x.table ? `<div class="bd-ph"><b class="bd-pt">${esc(x.table.title)}</b></div>${bdTable(x.table)}` : bdEmpty("No league table yet.")}</section>
      <aside class="bd-panel bd-side">${bdPollNext(x)}</aside></div></div>`;
  }
  const c = x.chart;
  if (st.wk == null || st.wk >= (x.moves || []).length) st.wk = Math.max(0, (x.moves || []).length - 1);
  const pick = c.lines.find((l) => l.id === st.sel) || c.lines.find((l) => l.me);
  const hls = bdSegs("bd:poll", "hl", c.hls.map((h, k) => [k, esc(h)]), st.hl ?? 0, "small");
  return `<div class="pad bd bd-poll">${head}${tabRow}${fbs}
    <div class="bd-cols">
      <section class="bd-panel">
        <div class="bd-ph"><span><b class="bd-pt">${x.level === "nfl" ? "Place by week" : "Rank by week"}</b> <span class="bd-ps">${c.lines.length} teams shown · click a name or a letter</span></span>${hls}</div>
        ${bdPollChart(x, st, pick && pick.id)}
        ${bdPollMoves(x, st)}
      </section>
      <aside class="bd-panel bd-side">${bdPollCard(x, pick)}${bdPollNext(x)}${x.table ? `<div class="bd-card"><span class="st-label">${esc(x.table.title)}</span>${bdTable(x.table)}</div>` : ""}</aside>
    </div></div>`;
};
