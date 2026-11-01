// The player boards of lane 9A (update 2): a player's shape (V4, its own
// tab), his career arc (V5, the Career tab) and his body and testing (V6,
// its own tab). Loaded after squadboards.js, whose helpers (sbGet, sbBand,
// sbPct and the rest) they share. Every rating is your staff's range.
"use strict";

const sbCap = (s) => (s ? String(s)[0].toUpperCase() + String(s).slice(1) : "");
const sbDiff = (d) => (d == null ? "–" : d > 0 ? "+" + d : d < 0 ? "−" + -d : "±0");

// ---------- V4: a player's shape ----------
const SB_RADAR = { C: [260, 235], R: 150 };
function sbRadarPt(i, v, f = 40, t = 99) {
  const a = -Math.PI / 2 + (i * Math.PI) / 4;
  const r = Math.max(0, Math.min(1, (v - f) / (t - f))) * SB_RADAR.R;
  return [SB_RADAR.C[0] + Math.cos(a) * r, SB_RADAR.C[1] + Math.sin(a) * r];
}
function sbShapeCmp(x) {
  const c = keep("sbshcmp", { v: "level" });
  if (c.v === "last" && !x.compare.last) c.v = "level";
  if (c.v.startsWith("m") && !x.compare.mates[Number(c.v.slice(1))]) c.v = "level";
  return c.v;
}
function sbCmpVal(f, cmp) {
  if (cmp === "level") return sbNum(f.level);
  if (cmp === "last") return sbNum(f.last);
  if (cmp.startsWith("m")) return sbNum((f.mates || [])[Number(cmp.slice(1))]);
  return null;
}
async function sbShapeTab(p) {
  const x = await sbGet("sb_shape", { id: p.id });
  const cmp = sbShapeCmp(x);
  const ax = Math.min(7, Math.max(0, keep("sbshax:" + x.pos, { i: 0 }).i));
  const F = x.families;
  const fm = (f) => (f.lo != null ? (f.lo + f.hi) / 2 : null);
  // One floor for the level: his families, the level's starters and his read.
  const fl = sbFloor(F.map((f) => [f.lo, f.level]), x.read, x.band ? x.band.starter : null);
  // Its edge: 99, or nearer where the level's starters and his best family
  // sit well below it (high school), so the shape fills the circle.
  const top = Math.min(99, Math.max(fl + 50, ...F.map((f) => Math.ceil(((sbNum(f.level) || 0) + 25) / 10) * 10), ...F.map((f) => Math.ceil(((f.hi || 0) + 8) / 10) * 10)));
  const rings = [10, 20, 30, 40, 50, 60, 70, 80, 90].filter((v) => v > fl && v < top).map((v) => {
    const r = ((v - fl) / (top - fl)) * SB_RADAR.R;
    const a = (-3 * Math.PI) / 8;
    return `<circle cx="${SB_RADAR.C[0]}" cy="${SB_RADAR.C[1]}" r="${r.toFixed(1)}" class="sb-rg"/><text x="${(SB_RADAR.C[0] + Math.cos(a) * r + 4).toFixed(1)}" y="${(SB_RADAR.C[1] + Math.sin(a) * r + 3).toFixed(1)}" class="sb-rgt">${v}</text>`;
  }).join("");
  const spokes = F.map((_, i) => { const [x2, y2] = sbRadarPt(i, top, fl, top); return `<line x1="${SB_RADAR.C[0]}" y1="${SB_RADAR.C[1]}" x2="${x2.toFixed(1)}" y2="${y2.toFixed(1)}" class="sb-sp"/>`; }).join("");
  const pts = (vals) => vals.map((v, i) => sbRadarPt(i, v == null ? fl : v, fl, top).map((n) => n.toFixed(1)).join(","));
  const seen = F.every((f) => f.lo != null);
  const band = seen ? `<path d="M${pts(F.map((f) => f.hi)).join(" L")} Z M${pts(F.map((f) => f.lo)).join(" L")} Z" class="sb-rb" fill-rule="evenodd"/>` : "";
  const cv = F.map((f) => sbCmpVal(f, cmp));
  const cmpPoly = cv.every((v) => v != null) ? `<polygon points="${pts(cv).join(" ")}" class="sb-rc"/>` : "";
  const midPoly = F.some((f) => f.lo != null) ? `<polygon points="${pts(F.map(fm)).join(" ")}" class="sb-rm"/>` : "";
  const axes = F.map((f, i) => {
    const a = -Math.PI / 2 + (i * Math.PI) / 4;
    const lx = SB_RADAR.C[0] + Math.cos(a) * (SB_RADAR.R + 54), ly = SB_RADAR.C[1] + Math.sin(a) * (SB_RADAR.R + 34);
    const m = fm(f);
    const [mx, my] = sbRadarPt(i, m == null ? fl : m, fl, top);
    return `${m != null ? `<circle cx="${mx.toFixed(1)}" cy="${my.toFixed(1)}" r="${i === ax ? 5 : 2.6}" class="sb-rdot ${i === ax ? "on" : ""}"/>` : ""}<foreignObject x="${(lx - 62).toFixed(1)}" y="${(ly - 18).toFixed(1)}" width="124" height="38"><button xmlns="http://www.w3.org/1999/xhtml" class="sb-ax ${i === ax ? "on" : ""}" data-act="sbax" data-pos="${esc(x.pos)}" data-i="${i}"><b>${esc(f.name.toUpperCase())}</b><small>${f.lo != null ? `${f.lo}-${f.hi}` : "Not seen"}</small></button></foreignObject>`;
  }).join("");
  const cmpOpts = [["level", x.compare.level_short], ...(x.compare.last ? [["last", x.compare.last_short]] : []), ...x.compare.mates.slice(0, 3).map((m, i) => ["m" + i, m.name.split(" ").slice(-1)[0]])];
  const cmpName = cmp === "level" ? x.compare.level_short : cmp === "last" ? x.compare.last_short : (x.compare.mates[Number(cmp.slice(1))] || {}).name;
  const f = F[ax];
  const c = sbCmpVal(f, cmp), m = fm(f);
  const diff = m != null && c != null ? Math.round(m - c) : null;
  const famCard = `<div class="sb-card"><span class="sb-lbl">${esc(f.name)}</span><div class="sb-cells three"><div><span class="sb-lbl">Range</span><b>${f.lo != null ? `${f.lo}-${f.hi}` : "Not seen"}</b></div><div><span class="sb-lbl">${esc(cmpName || "")}</span><b>${c != null ? Math.round(c) : "–"}</b></div><div><span class="sb-lbl">Difference</span><b class="${diff != null && diff <= -5 ? "sb-warn" : ""}">${sbDiff(diff)}</b></div></div>
    <div class="sb-sec"><span class="sb-lbl">Built from</span><div class="sb-chips">${f.from.map((a) => `<span class="sb-fchip"><span>${esc(a.label)}</span>${range(a.read, null, { p: x.id, k: a.key })}</span>`).join("")}</div></div><p class="sb-note">${esc(f.note)}</p></div>`;
  const listRows = x.groups.map((g) => `<div class="sb-lg"><span class="sb-lbl">${esc(g.name)}</span>${g.rows.map((r) => {
    const cvv = cmp === "level" ? r.level : cmp === "last" ? (r.last ? (r.last[0] + r.last[1]) / 2 : null) : (r.mates || [])[Number(cmp.slice(1))];
    const d = r.read && cvv != null ? Math.round((r.read[0] + r.read[1]) / 2 - cvv) : null;
    return `<div class="sb-lr ${r.fams.includes(ax) ? "on" : ""}"><span>${esc(r.label)}</span>${range(r.read, null, { p: x.id, k: r.key })}${sbBand(r.read, cvv, fl)}<b class="${d != null && d <= -5 ? "sb-warn" : d != null && d >= 3 ? "" : "sb-dim"}">${sbDiff(d)}</b></div>`;
  }).join("")}</div>`).join("");
  const b = x.band;
  const scale = x.read ? `<span class="sb-scale"><i class="sb-sc-b" style="left:${sbPct(x.read[0], fl)}%;width:${Math.max(1.5, sbPct(x.read[1], fl) - sbPct(x.read[0], fl))}%"></i>${b ? `<i class="sb-sc-t" style="left:${sbPct(b.starter, fl)}%" title="A typical starter: ${b.starter}"></i>` : ""}</span><span class="sb-scl"><small>${fl}</small><small>${b ? `typical starter ${b.starter}` : ""}</small><small>99</small></span>` : "";
  const since = x.since
    ? `<div class="sb-card"><span class="sb-lbl">Since the end of ${x.since.season}</span><div class="sb-hero2"><b>${sbDiff(x.since.change)}</b><small class="sb-dim">overall read${x.since.then ? `, from ${x.since.then[0]}-${x.since.then[1]}` : ""}</small></div>${x.since.gains.map((g) => `<div class="sb-kv"><span>${esc(g.name)}</span><b>${sbDiff(g.d)}</b></div>`).join("")}</div>`
    : `<div class="sb-card"><span class="sb-lbl">Since the last kept read</span><p class="sb-dim">${esc(x.kept_note || "")}</p></div>`;
  const left = `<div class="sb-card"><span class="sb-lbl">Staff read</span><div class="sb-hero2"><b>${x.read ? sbMid(x.read) : "–"}</b>${range(x.read, x.own ? "his position coach" : "your scouts", { p: x.id, k: "ovr" })}</div>${scale}
      ${b ? `<div class="sb-kv"><span>Level</span><b>${esc(b.word)}</b></div><small class="sb-dim">${esc(b.line)}</small>` : ""}
      <div class="sb-kv"><span>Outlook</span><b>${esc(sbCap(x.outlook.tier))}</b></div><small class="sb-dim">The staff's word, not a promise. ${esc(x.tape)}.</small></div>
    <div class="sb-card"><span class="sb-lbl">Player facts</span>${x.facts.map((q) => `<div class="sb-kv"><span>${esc(q.k)}</span><b>${esc(q.v)}</b></div>`).join("")}</div>${since}`;
  const radar = `<svg class="sb-radar" viewBox="0 0 520 470" role="img" aria-label="${esc(x.name)}: eight skill families against ${esc(cmpName || "")}">${rings}${spokes}${band}${cmpPoly}${midPoly}${axes}</svg>`;
  const legend = `<div class="sb-rleg"><span><i class="me"></i>Your staff's read, the band its range</span><span><i class="cmp"></i>${esc(cmpName || "")}</span></div>`;
  const nav = x.nav ? `<span class="sb-pn2">${linkTo("player", { id: x.nav.prev, tab: "Shape" }, "‹ Previous", "st-btn small glass")}${linkTo("player", { id: x.nav.next, tab: "Shape" }, "Next ›", "st-btn small glass")}</span>` : "";
  return `<div class="sb"><div class="sb-top"><span class="sb-lbl">Compare with</span><span class="sb-seg">${cmpOpts.map(([v, l]) => `<button data-act="sbshcmp" data-v="${v}" aria-pressed="${v === cmp}">${esc(l)}</button>`).join("")}</span>${nav}</div>
    <div class="sb-shape"><div class="sb-col">${left}</div><div class="sb-col mid"><div class="sb-card">${radar}${legend}</div>${famCard}</div><div class="sb-col"><div class="sb-card"><span class="sb-lbl">Ratings <small class="sb-dim">against ${esc(cmpName || "")}</small></span>${listRows}</div></div></div></div>`;
}
// A tab that can't be drawn says why instead of breaking the page.
const sbSafe = (f) => async (p) => {
  try {
    return await f(p);
  } catch (e) {
    return `<div class="empty">${esc(e.message || String(e))}</div>`;
  }
};
PLAYER_TABS.push({ name: "Shape", after: "Overview", draw: sbSafe(sbShapeTab) });

// ---------- V5: the career arc (the Career tab) ----------
const SB_ARC = { W: 960, H: 432, L: 46, R: 18, T: 22, B: 40, lo: 40, hi: 99 };
// The arc's floor: his reads, the road's half paths and the marks drawn.
function sbArcFloor(x) {
  return sbFloor(x.points.map((p) => (p.read ? p.read[0] : null)), x.table.map((r) => (r.read ? r.read[0] : null)), x.path.map((p) => p.q[1]), x.refs.map((r) => r.v));
}
function sbArcChart(x, metric, ghostsOn, selSeason) {
  const A = { ...SB_ARC, lo: sbArcFloor(x) };
  const first = x.points.length ? Math.min(...x.table.map((r) => r.season), ...x.points.map((p) => p.season)) : x.table.length ? x.table[0].season : x.points[0].season;
  const all = [...x.table.map((r) => r.season), ...x.points.map((p) => p.season), ...(metric === "ovr" ? x.path.map((p) => p.season) : [])];
  const s0 = Math.min(first, ...all), s1 = Math.max(...all, s0 + 1);
  const X = (s) => A.L + ((s - s0) / (s1 - s0)) * (A.W - A.L - A.R);
  const Y = (v) => A.T + (1 - (Math.max(A.lo, Math.min(A.hi, v)) - A.lo) / (A.hi - A.lo)) * (A.H - A.T - A.B);
  // The eras behind: each season's level.
  const lv = new Map();
  x.table.forEach((r) => lv.set(r.season, r.level));
  if (metric === "ovr") x.path.forEach((p) => lv.set(p.season, p.level));
  const eras = [];
  for (let s = s0; s <= s1; s++) {
    const l = lv.get(s) || (eras.length ? eras[eras.length - 1].l : "");
    if (eras.length && eras[eras.length - 1].l === l) eras[eras.length - 1].b = s;
    else eras.push({ l, a: s, b: s });
  }
  const eraHtml = eras.map((e, i) => `<rect x="${(X(e.a) - (i ? (X(e.a) - X(e.a - 1)) / 2 : 0)).toFixed(1)}" y="${A.T}" width="${(X(e.b) - X(e.a) + (X(s0 + 1) - X(s0))).toFixed(1)}" height="${A.H - A.T - A.B}" class="sb-era e${i % 2}"/><text x="${(X(e.a) + 6).toFixed(1)}" y="${A.T + 14}" class="sb-erat">${esc(e.l.toUpperCase())}${e.a > x.points[x.points.length - 1].season ? " · PROJECTED" : ""}</text>`).join("");
  const grid = [10, 20, 30, 40, 50, 60, 70, 80, 90].filter((v) => v > A.lo).map((v) => `<line x1="${A.L}" x2="${A.W - A.R}" y1="${Y(v).toFixed(1)}" y2="${Y(v).toFixed(1)}" class="sb-ag"/><text x="${A.L - 8}" y="${(Y(v) + 4).toFixed(1)}" class="sb-agt">${v}</text>`).join("");
  const years = [];
  for (let s = s0; s <= s1; s++) years.push(`<text x="${X(s).toFixed(1)}" y="${A.H - A.B + 18}" class="sb-ayt ${s === selSeason ? "on" : ""}">${s}</text>`);
  const refs = metric === "ovr" ? x.refs.map((r) => `<line x1="${A.L}" x2="${A.W - A.R}" y1="${Y(r.v).toFixed(1)}" y2="${Y(r.v).toFixed(1)}" class="sb-ref"/><text x="${A.W - A.R - 4}" y="${(Y(r.v) - 5).toFixed(1)}" class="sb-reft">${esc(r.label)} ${r.v}</text>`).join("") : "";
  const now = x.points[x.points.length - 1];
  const today = `<line x1="${X(now.season).toFixed(1)}" x2="${X(now.season).toFixed(1)}" y1="${A.T}" y2="${A.H - A.B}" class="sb-today"/><text x="${(X(now.season) + 5).toFixed(1)}" y="${A.H - A.B - 6}" class="sb-todayt">TODAY</text>`;
  const ki = metric === "ovr" ? -1 : x.metrics.findIndex((m) => m.key === metric) - 1;
  const rd = (p) => (ki < 0 ? p.read : p.keys ? p.keys[ki] : null);
  let proj = "", ghosts = "";
  if (metric === "ovr" && x.path.length && now.read) {
    const m0 = (now.read[0] + now.read[1]) / 2;
    const P = [{ season: now.season, q: [now.read[0], now.read[0], m0, now.read[1], now.read[1]] }, ...x.path];
    const area = (lo, hi) => `${P.map((p) => `${X(p.season).toFixed(1)},${Y(p.q[hi]).toFixed(1)}`).join(" ")} ${P.slice().reverse().map((p) => `${X(p.season).toFixed(1)},${Y(p.q[lo]).toFixed(1)}`).join(" ")}`;
    proj = `<polygon points="${area(0, 4)}" class="sb-po"/><polygon points="${area(1, 3)}" class="sb-pi"/><polyline points="${P.map((p) => `${X(p.season).toFixed(1)},${Y(p.q[2]).toFixed(1)}`).join(" ")}" class="sb-pm"/>`;
    if (ghostsOn) ghosts = x.ghosts.map((g) => {
      const line = `<polyline points="${g.d.map((d, i) => `${X(now.season + i).toFixed(1)},${Y(m0 + d).toFixed(1)}`).join(" ")}" class="sb-gh"><title>${esc(g.name)}</title></polyline>`;
      return g.id != null ? `<a data-go="player" data-id="${g.id}">${line}</a>` : line;
    }).join("");
  }
  const pts = x.points.filter((p) => rd(p));
  const line = pts.length > 1 ? `<polyline points="${pts.map((p) => `${X(p.season).toFixed(1)},${Y((rd(p)[0] + rd(p)[1]) / 2).toFixed(1)}`).join(" ")}" class="sb-al"/>` : "";
  const dots = pts.map((p) => { const r = rd(p); return `<line x1="${X(p.season).toFixed(1)}" x2="${X(p.season).toFixed(1)}" y1="${Y(r[0]).toFixed(1)}" y2="${Y(r[1]).toFixed(1)}" class="sb-aw"/><circle cx="${X(p.season).toFixed(1)}" cy="${Y((r[0] + r[1]) / 2).toFixed(1)}" r="${p.season === selSeason ? 6 : 4}" class="sb-ad ${p.now ? "now" : ""}"/>`; }).join("");
  // A click on a season picks it.
  const hits = [];
  for (let s = s0; s <= s1; s++) hits.push(`<rect x="${(X(s) - (X(s0 + 1) - X(s0)) / 2).toFixed(1)}" y="${A.T}" width="${(X(s0 + 1) - X(s0)).toFixed(1)}" height="${A.H - A.T - A.B}" class="sb-hit" data-act="sbarcy" data-id="${x.id}" data-v="${s}"><title>${s}</title></rect>`);
  return `<svg class="sb-arc" viewBox="0 0 ${A.W} ${A.H}" role="img" aria-label="${esc(x.name)}'s career by your staff's reads">${eraHtml}${grid}${refs}${proj}${ghosts}${today}${line}${dots}${years.join("")}${hits.join("")}</svg>`;
}
async function sbArcTab(p) {
  const x = await sbGet("sb_arc", { id: p.id });
  const metric = keep("sbarcm", { v: "ovr" }).v;
  const mt = x.metrics.slice(0, 4).some((m) => m.key === metric) ? metric : "ovr";
  const ghostsOn = keep("sbarcg", { v: false }).v;
  const now = x.points[x.points.length - 1];
  const sel = keep("sbarcy:" + x.id, { v: now.season }).v;
  const chart = sbArcChart(x, mt, ghostsOn, sel);
  const ki = mt === "ovr" ? -1 : x.metrics.findIndex((m) => m.key === mt) - 1;
  const readOf = (r) => (ki < 0 ? r.read : r.keys ? r.keys[ki] : null);
  const af = sbArcFloor(x);
  const rows = x.table.slice().reverse().map((r) => {
    const rr = readOf(r);
    return `<div class="sb-ar ${r.season === sel ? "on" : ""} ${r.now ? "now" : ""}" data-act="sbarcy" data-id="${x.id}" data-v="${r.season}"><span>${r.season}<small>age ${r.age}</small></span><span>${esc(r.team || r.level)}<small>${esc(r.level)} · ${esc(r.pos)}</small></span><span class="sb-arr">${rr ? `<b>${sbMid(rr)}</b>${sbBand(rr, null, af)}<small>${rr[0]}-${rr[1]}</small>` : '<span class="sb-dim" title="Your staff kept no read of him that season">Not kept</span>'}</span><span>${r.games != null ? r.games : "–"}</span><span class="sb-wrap">${esc(r.line || "")}</span><span class="sb-wrap">${esc(r.what || "")}</span></div>`;
  }).join("");
  const pk = x.peak;
  const proj = `<div class="sb-card"><span class="sb-lbl">Projection</span>${pk ? `<div class="sb-hero2"><b>${pk.v}</b><small class="sb-dim">likely peak · ${pk.lo}-${pk.hi}</small></div><span class="sb-scale"><i class="sb-sc-b" style="left:${sbPct(pk.lo, af)}%;width:${Math.max(1.5, sbPct(pk.hi, af) - sbPct(pk.lo, af))}%"></i>${x.nfl_starter != null ? `<i class="sb-sc-t" style="left:${sbPct(x.nfl_starter, af)}%" title="A typical NFL starter: ${x.nfl_starter}"></i>` : ""}</span><span class="sb-scl"><small>${af}</small><small>${x.nfl_starter != null ? `typical NFL starter ${x.nfl_starter}` : ""}</small><small>99</small></span>` : '<p class="sb-dim">Nobody on your staff has read him yet.</p>'}
    <div class="sb-cells three"><div><span class="sb-lbl">Peak age</span><b>${pk ? pk.age : "–"}</b></div><div><span class="sb-lbl">Draft</span><b>${esc(x.draft.value)}</b><small class="sb-dim">${esc(x.draft.note)}</small></div><div><span class="sb-lbl">Starter</span><b>${esc(x.starter.value)}</b><small class="sb-dim">${esc(x.starter.note)}</small></div></div><p class="sb-note">${esc(x.footnote)}</p></div>`;
  const row = x.table.find((r) => r.season === sel);
  const pr = x.path.find((q) => q.season === sel);
  const yearCard = row
    ? `<div class="sb-card"><span class="sb-lbl">${sel}${row.now ? " · this season" : ""}</span><div class="sb-kv"><span>Team</span><b>${esc(row.team || row.level)}</b></div><div class="sb-kv"><span>Staff read</span><b>${row.read ? `${row.read[0]}-${row.read[1]}` : "No read kept"}</b></div>${row.level_rating ? `<div class="sb-kv"><span>Level rating</span><b>${row.level_rating}</b></div>` : ""}<div class="sb-kv"><span>Games</span><b>${row.games != null ? row.games : "–"}</b></div><p class="sb-note">${esc(row.line || "")}${row.line ? ". " : ""}${esc(row.what || "")}</p></div>`
    : pr
      ? `<div class="sb-card"><span class="sb-lbl">${sel} · projected</span><div class="sb-kv"><span>Likely level</span><b>${esc(pr.level)}</b></div><div class="sb-kv"><span>Age</span><b>${pr.age}</b></div><div class="sb-kv"><span>Likely read</span><b>${Math.round(pr.q[2])}</b></div><div class="sb-kv"><span>Half the paths</span><b>${Math.round(pr.q[1])}-${Math.round(pr.q[3])}</b></div><div class="sb-kv"><span>Most paths</span><b>${Math.round(pr.q[0])}-${Math.round(pr.q[4])}</b></div></div>`
      : "";
  const alikes = x.alikes.length ? `<div class="sb-card"><span class="sb-lbl">Shapes like his <small class="sb-dim">the pros and blue-chips rated like him at his age</small></span>${x.alikes.map((a) => `<div class="sb-al2"><span>${playerLink(a.id, a.name)}<small>${esc(a.outcome)}</small></span><b>${esc(a.grew)}</b></div>`).join("")}</div>` : "";
  const seg = `<span class="sb-seg">${x.metrics.slice(0, 4).map((m) => `<button data-act="sbarcm" data-v="${esc(m.key)}" aria-pressed="${m.key === mt}">${esc(m.label)}</button>`).join("")}</span><label class="sb-check"><input type="checkbox" data-act="sbarcg" ${ghostsOn ? "checked" : ""} ${mt !== "ovr" ? "disabled" : ""}> Look-alikes</label>`;
  return `<div class="sb"><div class="sb-top">${seg}${x.kept_note ? `<small class="sb-dim">${esc(x.kept_note)}</small>` : ""}</div>
    <div class="sb-arcw"><div class="sb-col"><div class="sb-card">${chart}<div class="sb-rleg"><span><i class="me"></i>Your staff's kept reads</span>${mt === "ovr" ? '<span><i class="pm"></i>Likely road</span><span><i class="pi"></i>Half the paths</span><span><i class="po"></i>Most paths</span>' : ""}</div></div>
      <div class="sb-card"><div class="sb-ah"><span>Year</span><span>Team</span><span>Staff read</span><span>G</span><span>Line</span><span>What happened</span></div>${rows}</div></div>
    <div class="sb-col">${proj}${yearCard}${alikes}</div></div></div>`;
}
const sbCareerBase = PT.Career;
PT.Career = async (p) => {
  let arc = "";
  try {
    arc = await sbArcTab(p);
  } catch (e) {
    arc = "";
  }
  return arc + sbCareerBase(p);
};

// ---------- V6: body and testing ----------
async function sbBodyTab(p) {
  const x = await sbGet("sb_body", { id: p.id });
  const ci = Math.max(0, x.compares.findIndex((c) => c.key === keep("sbbcmp", { v: "level" }).v));
  const cmp = x.compares[ci];
  const sc = x.scores[ci];
  const seg = `<span class="sb-seg">${x.compares.map((c, i) => `<button data-act="sbbcmp" data-v="${esc(c.key)}" aria-pressed="${i === ci}">${esc(c.label)}</button>`).join("")}</span>`;
  const score = `<div class="sb-card"><span class="sb-lbl">Athletic score</span><div class="sb-hero2"><b>${sc.score != null ? sc.score.toFixed(1) : "–"}</b><small class="sb-dim">out of 10, against ${esc(cmp.long)}</small></div>${sc.better != null ? `<small class="sb-dim">Better than ${sc.better}% of them</small>` : '<small class="sb-dim">Too few of his numbers are known to score him.</small>'}
    <span class="sb-scale"><i class="sb-sc-b" style="left:0;width:${sc.score != null ? sc.score * 10 : 0}%"></i><i class="sb-sc-t" style="left:50%"></i></span><span class="sb-scl"><small>0</small><small>5</small><small>10</small></span>
    <div class="sb-cells four">${sc.subs.map((s) => `<div><span class="sb-lbl">${esc(s.k)}</span><b class="${s.v != null && s.v < 2.5 ? "sb-warn" : ""}">${s.v != null ? s.v.toFixed(1) : "–"}</b><small class="sb-dim">of 10</small></div>`).join("")}</div></div>`;
  const ti = Math.max(0, x.rows.findIndex((r) => r.key === keep("sbbt", { v: "forty" }).v));
  const rows = x.rows.map((r, i) => {
    const pc = r.pct ? r.pct[ci] : null;
    return `<div class="sb-mr ${i === ti ? "on" : ""}" data-act="sbbt" data-v="${esc(r.key)}"><span>${esc(r.label)}</span><span>${r.text != null ? `<b>${esc(r.text)}</b><small class="${r.verified === false ? "sb-warn" : "sb-dim"}" title="${esc(r.source || "")}">${r.verified ? "✓ " : ""}${esc(r.source || "")}</small>` : '<span class="sb-dim">Not tested</span>'}</span><span class="sb-pb2">${pc != null ? `<span class="sb-pbar"><i style="width:${pc}%" class="${pc < 25 ? "low" : ""}"></i><i class="mid"></i></span>` : ""}</span><b class="${pc != null && pc < 25 ? "sb-warn" : ""}">${pc != null ? pc : "–"}</b></div>`;
  }).join("");
  const lifts = x.lifts.length ? `<div class="sb-sec"><span class="sb-lbl">The weight room <small class="sb-dim">your strength staff</small></span>${x.lifts.map((l) => `<div class="sb-kv"><span>${esc(l.label)}</span><b>${esc(l.text)}</b></div>`).join("")}</div>` : "";
  const t = x.rows[ti];
  const plot = sbBodyPlot(x, ti, ci);
  const alikes = x.alikes.length ? `<div class="sb-card"><span class="sb-lbl">Built like him <small class="sb-dim">drafted ${esc(x.pos)}s closest in size and testing</small></span><div class="sb-bh"><span>Player</span><span>Drafted</span><span>NFL career</span><span>Match</span></div>${x.alikes.map((a) => `<div class="sb-br"><span>${playerLink(a.id, a.name)}<small>${esc(a.line)}</small></span><span>${esc(a.drafted)}</span><span>${esc(a.career)}</span><b>${a.match}%</b></div>`).join("")}</div>` : `<div class="sb-card"><span class="sb-lbl">Built like him</span><p class="sb-dim">Too few of his numbers are known to match him.</p></div>`;
  const ws = x.weights;
  const wline = ws.length > 1 ? (() => {
    const W = 640, H = 150, lo = Math.min(...ws.map((w) => w.wt)) - 4, hi = Math.max(...ws.map((w) => w.wt)) + 4;
    const X = (i) => 40 + (i * (W - 80)) / Math.max(1, ws.length - 1), Y = (v) => H - 34 - ((v - lo) / Math.max(1, hi - lo)) * (H - 66);
    // Every point carries its weight; the dates under at most six of them.
    const every = Math.ceil(ws.length / 6);
    const named = (i) => i === 0 || i === ws.length - 1 || (i % every === 0 && ws.length - 1 - i >= every / 2);
    const day = (s) => String(s || "").replace(/^[A-Z][a-z]{2} \d{1,2} /, "");
    return `<svg class="sb-wl" viewBox="0 0 ${W} ${H}" role="img" aria-label="His weight over time"><polyline points="${ws.map((w, i) => `${X(i).toFixed(1)},${Y(w.wt).toFixed(1)}`).join(" ")}" class="sb-al"/>${ws.map((w, i) => `<circle cx="${X(i).toFixed(1)}" cy="${Y(w.wt).toFixed(1)}" r="4" class="sb-ad"><title>${esc(w.label)}: ${w.wt} lb (${esc(w.source || "")})</title></circle>${named(i) ? `<text x="${X(i).toFixed(1)}" y="${(Y(w.wt) - 10).toFixed(1)}" class="sb-wlv">${w.wt}</text><text x="${X(i).toFixed(1)}" y="${H - 10}" class="sb-wlt">${esc(day(w.label))}</text>` : ""}`).join("")}</svg>`;
  })() : '<p class="sb-dim">Your staff keeps his weight from each testing day and each season\'s end.</p>';
  const body = x.body;
  const over = `<div class="sb-card"><span class="sb-lbl">Body over time</span>${wline}<div class="sb-kv"><span>Playing weight</span><b>${body.now} lb</b></div>${body.gained != null && ws.length > 1 ? `<div class="sb-kv"><span>Since ${esc(ws[0].label)}</span><b>${sbDiff(Math.round(body.gained))} lb</b></div>` : ""}<div class="sb-kv"><span>Staff target</span><b>${body.target ? body.target + " lb" : "None set"}</b></div><div class="sb-kv"><span>Games missed hurt</span><b>${body.missed}</b></div><div class="sb-kv"><span>Seasons with an injury</span><b>${body.injuries}</b></div></div>`;
  return `<div class="sb"><div class="sb-top"><span class="sb-lbl">Compare with</span>${seg}</div>
    <div class="sb-body"><div class="sb-col">${score}<div class="sb-card"><span class="sb-lbl">Measurements <small class="sb-dim">✓ verified · the rest self-reported</small></span><div class="sb-mh"><span>Test</span><span>Result</span><span>Percentile</span><span>Pct</span></div>${rows}${lifts}</div></div>
    <div class="sb-col"><div class="sb-card"><span class="sb-lbl">${esc(t.label)} against weight <small class="sb-dim">${esc(cmp.long)}</small></span>${plot}</div>${alikes}${over}</div></div></div>`;
}
function sbBodyPlot(x, ti, ci) {
  const W = 900, H = 392, L = 52, R = 14, T = 16, B = 40;
  const t = x.rows[ti];
  const me = t.value, mw = x.rows[1].value;
  const pool = ci === 0 ? x.plot.level : ci === 1 ? x.plot.drafted : [];
  const pts = pool.map((m) => [m.v[ti], m.v[1], m.f, m]).filter((q) => q[0] != null && q[1] != null);
  const real = x.plot.real;
  let xs = pts.map((q) => q[0]), ys = pts.map((q) => q[1]);
  if (ci === 2 && real[ti] && real[1]) {
    xs = [real[ti][0] - 2.5 * real[ti][1], real[ti][0] + 2.5 * real[ti][1]];
    ys = [real[1][0] - 2.5 * real[1][1], real[1][0] + 2.5 * real[1][1]];
  }
  if (me != null) xs.push(me);
  if (mw != null) ys.push(mw);
  if (!xs.length || !ys.length) return '<p class="sb-dim">Nothing to plot for this test.</p>';
  const pad = (a, b) => { const d = (b - a) * 0.06 || 1; return [a - d, b + d]; };
  const [x0, x1] = pad(Math.min(...xs), Math.max(...xs)), [y0, y1] = pad(Math.min(...ys), Math.max(...ys));
  // Faster to the left on the clock.
  const flip = t.lower;
  const X = (v) => L + ((flip ? x1 - v : v - x0) / (x1 - x0)) * (W - L - R);
  const Y = (v) => H - B - ((v - y0) / (y1 - y0)) * (H - T - B);
  const tick = (a, b, n) => Array.from({ length: n + 1 }, (_, i) => a + ((b - a) * i) / n);
  const dec = Math.abs(x1 - x0) < 3 ? 2 : Math.abs(x1 - x0) < 20 ? 1 : 0;
  const gx = tick(x0, x1, 6).map((v) => `<line x1="${X(v).toFixed(1)}" x2="${X(v).toFixed(1)}" y1="${T}" y2="${H - B}" class="sb-ag"/><text x="${X(v).toFixed(1)}" y="${H - B + 16}" class="sb-ayt">${v.toFixed(dec)}</text>`).join("");
  const gy = tick(y0, y1, 4).map((v) => `<line x1="${L}" x2="${W - R}" y1="${Y(v).toFixed(1)}" y2="${Y(v).toFixed(1)}" class="sb-ag"/><text x="${L - 6}" y="${(Y(v) + 4).toFixed(1)}" class="sb-agt">${Math.round(v)}</text>`).join("");
  // Every dot is a man (minor bug 30): his name, school and both numbers on
  // hover, his page on Body and testing on a click; a retired man the world
  // keeps no page for shows his name only.
  const fmt = (k, v) => (k === 1 ? `${Math.round(v)} lb` : k === 0 ? `${Math.floor(v / 12)}'${Math.round(v % 12)}"` : t.lower ? v.toFixed(2) : v.toFixed(1));
  const dots = pts.map((q) => {
    const m = q[3], cx = X(q[0]).toFixed(1), cy = Y(q[1]).toFixed(1);
    const l = `${m.n || ""}${m.s ? " · " + m.s : ""} · ${fmt(ti, q[0])} · ${fmt(1, q[1])}`;
    const c = `<circle cx="${cx}" cy="${cy}" r="${q[2] ? 3.6 : 2.6}" class="sb-dot ${q[2] ? "first" : ""}"/>`;
    return m.id != null
      ? `<a class="sb-pt" data-go="player" data-id="${m.id}" data-tab="Body and testing" data-l="${esc(l)}" data-x="${cx}" data-y="${cy}">${c}</a>`
      : `<g class="sb-pt" data-l="${esc(l)}" data-x="${cx}" data-y="${cy}">${c}</g>`;
  }).join("");
  let ell = "";
  if (ci === 2 && real[ti] && real[1]) {
    const [mx, sx] = real[ti], [my, sy] = real[1];
    const rx = Math.abs(X(mx + sx) - X(mx)), ry = Math.abs(Y(my + sy) - Y(my));
    ell = `<ellipse cx="${X(mx).toFixed(1)}" cy="${Y(my).toFixed(1)}" rx="${(2 * rx).toFixed(1)}" ry="${(2 * ry).toFixed(1)}" class="sb-e2"/><ellipse cx="${X(mx).toFixed(1)}" cy="${Y(my).toFixed(1)}" rx="${rx.toFixed(1)}" ry="${ry.toFixed(1)}" class="sb-e1"/><text x="${(X(mx) + rx + 6).toFixed(1)}" y="${Y(my).toFixed(1)}" class="sb-reft">the middle two-thirds</text>`;
  }
  const meDot = me != null && mw != null ? (() => {
    const cx = X(me), cy = Y(mw);
    const label = `${x.last} · ${t.text} · ${x.rows[1].text}`;
    const bw = label.length * 6.6 + 18, bx = cx + bw + 14 > W ? cx - bw - 12 : cx + 12;
    return `<circle cx="${cx.toFixed(1)}" cy="${cy.toFixed(1)}" r="6" class="sb-me"/><rect x="${bx.toFixed(1)}" y="${(cy - 30).toFixed(1)}" width="${bw.toFixed(1)}" height="22" rx="6" class="sb-mel"/><text x="${(bx + 9).toFixed(1)}" y="${(cy - 15).toFixed(1)}" class="sb-melt">${esc(label)}</text>`;
  })() : "";
  const legend = ci === 1 ? '<div class="sb-rleg"><span><i class="dot"></i>Drafted</span><span><i class="ring"></i>First round</span><span><i class="me"></i>Him</span></div>' : ci === 2 ? '<div class="sb-rleg"><span><i class="po"></i>The real combine since 2000, one and two spreads</span><span><i class="me"></i>Him</span></div>' : '<div class="sb-rleg"><span><i class="dot"></i>His level at his position</span><span><i class="me"></i>Him</span></div>';
  return `<svg class="sb-plot" viewBox="0 0 ${W} ${H}" data-w="${W}" role="img" aria-label="${esc(t.label)} against weight">${gx}${gy}${ell}${dots}${meDot}<g class="sb-hov"></g><text x="${W - R}" y="${H - 4}" class="sb-axl">${esc(t.label.toUpperCase())}${flip ? " · FASTER TO THE RIGHT" : ""}</text><text x="${L}" y="${T - 4}" class="sb-axl l">WEIGHT (LB)</text></svg>${legend}`;
}
PLAYER_TABS.push({ name: "Body and testing", after: "Shape", draw: sbSafe(sbBodyTab) });
// A dot's label on hover (minor bug 30), drawn as the plot's own label.
document.addEventListener("mouseover", (ev) => {
  const a = ev.target.closest && ev.target.closest(".sb-plot .sb-pt");
  if (!a) return;
  const svg = a.closest("svg"), g = svg && svg.querySelector(".sb-hov");
  if (!g) return;
  const cx = Number(a.dataset.x), cy = Number(a.dataset.y), W = Number(svg.dataset.w) || 900, label = a.dataset.l || "";
  const bw = label.length * 6.6 + 18, bx = cx + bw + 14 > W ? cx - bw - 12 : cx + 12;
  g.innerHTML = `<circle cx="${cx}" cy="${cy}" r="5" class="sb-hdot"/><rect x="${bx.toFixed(1)}" y="${(cy - 30).toFixed(1)}" width="${bw.toFixed(1)}" height="22" rx="6" class="sb-hlab"/><text x="${(bx + 9).toFixed(1)}" y="${(cy - 15).toFixed(1)}" class="sb-hlabt">${esc(label)}</text>`;
});
document.addEventListener("mouseout", (ev) => {
  const a = ev.target.closest && ev.target.closest(".sb-plot .sb-pt");
  if (!a || (ev.relatedTarget && a.contains(ev.relatedTarget))) return;
  const g = a.closest("svg") && a.closest("svg").querySelector(".sb-hov");
  if (g) g.innerHTML = "";
});

// ---------- clicks ----------
Object.assign(CORE_MORE, {
  sbspot: (el) => { keep("sbdc:" + sbCurForm(), { k: 0 }).k = Number(el.dataset.k); sbLocal(el); },
  sbform: (el) => { keep("sbform", {})[el.dataset.tab] = el.dataset.v; refresh(el); },
  sbdcview: (el) => { keep("sbdcview", { v: "Broadcast" }).v = el.dataset.v; refresh(el); },
  sbup: async (el) => {
    const tab = keep("trboard", { t: "Offense" }).t;
    const form = sbCurForm();
    try {
      const r = await api("sb_depth_up", { tab: SB_TAB_KEY[tab], form, slot: Number(el.dataset.k), n: Number(el.dataset.n) });
      SB.cache = {};
      toast(r && r.note ? r.note : "Depth chart saved.");
    } catch (e) {
      toast(e.message || String(e));
    }
    refresh(el);
  },
  sbreset: async (el) => {
    try {
      await api("sb_depth_reset", { tab: SB_TAB_KEY[keep("trboard", { t: "Offense" }).t], form: el.dataset.form });
      SB.cache = {};
      toast("Your staff has the depth chart back.");
    } catch (e) {
      toast(e.message || String(e));
    }
    refresh(el);
  },
  sbneed: (el) => { keep("sbneeds", { k: "QB" }).k = el.dataset.k; sbLocal(el); },
  sbneedy: (el) => { keep("sbneedy", { y: 0 }).y = Number(el.dataset.v); refresh(el); },
  sbrail: (el) => { keep("sbrail", { g: "ALL" }).g = el.dataset.v; sbLocal(el); },
  sbrow: (el, ev) => { if (ev && ev.target.closest("a")) return; keep("sbrow", { id: null }).id = Number(el.dataset.id); sbLocal(el); },
  sbshcmp: (el) => { keep("sbshcmp", { v: "level" }).v = el.dataset.v; sbLocal(el); },
  sbax: (el) => { keep("sbshax:" + el.dataset.pos, { i: 0 }).i = Number(el.dataset.i); sbLocal(el); },
  sbarcm: (el) => { keep("sbarcm", { v: "ovr" }).v = el.dataset.v; sbLocal(el); },
  sbarcg: (el) => { const k = keep("sbarcg", { v: false }); k.v = !k.v; sbLocal(el); },
  sbarcy: (el) => { keep("sbarcy:" + el.dataset.id, { v: 0 }).v = Number(el.dataset.v); sbLocal(el); },
  sbbcmp: (el) => { keep("sbbcmp", { v: "level" }).v = el.dataset.v; sbLocal(el); },
  sbbt: (el) => { keep("sbbt", { v: "forty" }).v = el.dataset.v; sbLocal(el); },
});
// The formation on the stadium's open tab.
function sbCurForm() {
  const tab = keep("trboard", { t: "Offense" }).t;
  return keep("sbform", {})[tab] || SB_TAB_KEY[tab] || "offense";
}
