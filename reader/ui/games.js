// AFM World Reader: the game, club and draft views (lane R8). A game's win
// chance snap by snap, its drives on the field, the play by play with
// every why and its names linked, player lines in tabs; a club's seasons
// as a strip; a draft class as a board. The app works each one out
// (`games.rs`); this file only draws it.
"use strict";

const G_HOME = "#f0607c", G_AWAY = "#64a8ff";

// "#123" in a why as the player's name, linked to his page.
function namesIn(s, names) {
  return esc(s).replace(/#(\d+)/g, (m, id) => {
    const n = names && names[id];
    if (!n) return m;
    return n[1] ? `<a href="#/${esc(n[1])}">${esc(n[0])}</a>` : esc(n[0]);
  });
}

function segHtml(id, key, opts, cur) {
  return `<div class="seg" role="group">${opts.map(([v, l]) => `<button type="button" class="${v === cur ? "on" : ""}" aria-pressed="${v === cur}" data-gseg="${id}" data-k="${key}" data-v="${esc(v)}">${esc(l)}</button>`).join("")}</div>`;
}

function headHtml(b, extra) {
  return b.title ? `<div class="head"><h2>${esc(b.title)}</h2>${b.note ? `<span class="sub">${esc(b.note)}</span>` : ""}${extra || ""}</div>` : "";
}

// The home side's chance to win after every snap, the 50% line between.
function winlineHtml(b) {
  const pts = b.points || [];
  if (pts.length < 2) return `<p class="sub">Too few snaps for a chart.</p>`;
  const W = 1000, Hh = 280, L = 54, R = 16, T = 18, B = 30;
  const qn = Math.max(b.quarters || 4, 4), qm = b.qmin || 15;
  const x1 = Math.max(qn * qm, ...pts.map((p) => p[0]));
  const X = (x) => L + (x / x1) * (W - L - R);
  const Y = (y) => T + (1 - y / 100) * (Hh - T - B);
  let g = "";
  for (let q = 0; q < qn; q++) {
    const a = X(q * qm), z = X(Math.min(x1, (q + 1) * qm));
    if (q) g += `<line class="ax" x1="${a}" x2="${a}" y1="${T}" y2="${Hh - B}"/>`;
    g += `<text x="${(a + z) / 2}" y="${Hh - 8}" text-anchor="middle">${q < 4 ? "Q" + (q + 1) : "OT"}</text>`;
  }
  for (const y of [0, 25, 50, 75, 100]) g += `<line class="ax" x1="${L}" x2="${W - R}" y1="${Y(y)}" y2="${Y(y)}" ${y === 50 ? 'stroke-dasharray="4 4"' : ""}/><text x="${L - 8}" y="${Y(y) + 4}" text-anchor="end">${y === 50 ? "50%" : y === 100 ? esc(b.home) : y === 0 ? esc(b.away) : y + "%"}</text>`;
  const line = pts.map((p) => `${X(p[0]).toFixed(1)},${Y(p[1]).toFixed(1)}`).join(" ");
  const area = `${X(pts[0][0]).toFixed(1)},${Y(50)} ${line} ${X(pts[pts.length - 1][0]).toFixed(1)},${Y(50)}`;
  g += `<defs><clipPath id="wl-top"><rect x="0" y="0" width="${W}" height="${Y(50)}"/></clipPath><clipPath id="wl-bot"><rect x="0" y="${Y(50)}" width="${W}" height="${Hh}"/></clipPath></defs>`;
  g += `<polygon points="${area}" fill="${G_HOME}" fill-opacity=".22" clip-path="url(#wl-top)"/><polygon points="${area}" fill="${G_AWAY}" fill-opacity=".22" clip-path="url(#wl-bot)"/>`;
  g += `<polyline fill="none" stroke="#f4f5f7" stroke-width="2" points="${line}"/>`;
  const at = (t) => { let best = pts[0]; for (const p of pts) if (p[0] <= t + 0.001) best = p; return best[1]; };
  (b.marks || []).forEach((m) => {
    const c = m.side ? G_AWAY : G_HOME;
    g += `<circle cx="${X(m.t)}" cy="${Y(at(m.t))}" r="6" fill="${c}" stroke="#07080b" stroke-width="2"><title>${esc(m.text)} (${esc(m.score)})</title></circle>`;
  });
  const legend = `<span class="key"><i style="background:${G_HOME}"></i>${esc(b.home)} ahead in the chance</span><span class="key"><i style="background:${G_AWAY}"></i>${esc(b.away)} ahead</span><span class="key"><i style="background:#f4f5f7"></i>Dots: scores (hover)</span>`;
  return `<div class="legend">${legend}</div><div class="chart"><svg viewBox="0 0 ${W} ${Hh}" role="img" aria-label="${esc(b.title)}">${g}</svg></div>`;
}

const DRIVE_TONE = { Touchdown: "near", "Field goal": "off", Interception: "far", Fumble: "far", Downs: "far", Safety: "far", "Missed field goal": "far", "Blocked kick": "far" };

// Every drive on the field, from the offense's own goal line (left) to the
// one it is going for (right).
function drivesHtml(b, id) {
  const side = b.side == null ? "all" : b.side;
  const ds = (b.drives || []).filter((d) => side === "all" || String(d.side) === side);
  const opts = [["all", "Both"], ["1", b.sides[1]], ["0", b.sides[0]]];
  const rows = ds.map((d) => {
    const a = Math.max(0, Math.min(100, d.start)), z = Math.max(0, Math.min(100, d.end));
    const lo = Math.min(a, z), hi = Math.max(a, z);
    const tone = DRIVE_TONE[d.result] || "dim";
    const yds = Math.round(d.end - d.start), mm = Math.floor(d.secs / 60), ss = String(d.secs % 60).padStart(2, "0");
    return `<div class="drv"><span class="drv-t" style="border-color:${d.side ? G_AWAY : G_HOME}">${esc(b.sides[d.side])}</span><span class="sub">${esc(d.clock)}</span>
      <div class="drv-f" aria-label="From the ${Math.round(d.start)} to the ${Math.round(d.end)}"><i class="ez"></i><i class="ez r"></i><b class="seg-${tone}" style="left:${lo}%;width:${Math.max(0.6, hi - lo)}%"></b></div>
      <span class="num">${d.plays} plays, ${yds} yds, ${mm}:${ss}</span><span class="drv-r t-${tone}">${esc(d.result)}${d.points ? ` +${d.points}` : ""}</span></div>`;
  }).join("");
  return `<section class="panel sec" id="blk-${id}">${headHtml(b)}${segHtml(id, "side", opts, side)}<div class="drvs"><div class="drv drv-h"><span></span><span></span><div class="sub drv-ax"><span>Own goal</span><span>50</span><span>Their goal</span></div><span></span><span></span></div>${rows || `<p class="sub">No drives.</p>`}</div></section>`;
}

const PBP_KEEP = { all: () => true, score: (p) => p.flags.includes("score"), turnover: (p) => p.flags.includes("turnover"), big: (p) => p.flags.includes("big"), fourth: (p) => p.flags.includes("fourth"), flag: (p) => p.flags.includes("flag") };

function pbpHtml(b, id) {
  const show = b.show || "all", q = b.q || "all";
  const qs = [...new Set((b.plays || []).map((p) => p.q))];
  const plays = (b.plays || []).filter((p) => PBP_KEEP[show](p) && (q === "all" || String(p.q) === q));
  let lastQ = null;
  const rows = plays.map((p) => {
    let head = "";
    if (p.q !== lastQ) { lastQ = p.q; head = `<tr class="pbp-q"><td colspan="7">${p.q <= 4 ? "Quarter " + p.q : "Overtime"}</td></tr>`; }
    const call = [p.call ? esc(p.call) : "", p.call_why ? `<i>${esc(p.call_why)}</i>` : "", p.defense ? `vs ${esc(p.defense)}` : ""].filter(Boolean).join(" · ");
    const cls = p.flags.includes("score") ? "pbp-sc" : p.flags.includes("turnover") ? "pbp-to" : "";
    const epa = p.epa == null ? "" : `<span class="${p.epa >= 0 ? "t-near" : "t-far"}">${p.epa > 0 ? "+" : ""}${p.epa.toFixed(2)}</span>`;
    return `${head}<tr class="${cls}"><td class="sub">${esc(p.clock)}</td><td><span class="drv-t" style="border-color:${p.side ? G_AWAY : G_HOME}">${esc(b.sides[p.side])}</span></td>
      <td class="sub">${p.down ? esc(p.down) + "<br>" : ""}${esc(p.spot)}</td><td style="min-width:360px">${namesIn(p.why, b.names)}${call ? `<span class="why">${call}</span>` : ""}</td>
      <td class="n">${epa}</td><td class="n">${p.wp == null ? "" : p.wp.toFixed(0) + "%"}</td><td class="n">${esc(p.score)}</td></tr>`;
  }).join("");
  const opts = [["all", "Every snap"], ["score", "Scores"], ["turnover", "Turnovers"], ["big", "Big plays"], ["fourth", "Fourth downs"], ["flag", "Flags"]];
  const qopts = [["all", "All quarters"]].concat(qs.map((x) => [String(x), x <= 4 ? "Q" + x : "OT"]));
  return `<section class="panel sec" id="blk-${id}">${headHtml(b)}<div class="tags">${segHtml(id, "show", opts, show)}${segHtml(id, "q", qopts, q)}</div>
    <div class="tbl" style="max-height:760px;overflow:auto"><table class="pbp"><thead><tr><th>When</th><th>Ball</th><th>Down</th><th>What happened, and why</th><th class="n">EPA</th><th class="n">${esc(b.sides[0])} chance</th><th class="n">Score</th></tr></thead>
    <tbody>${rows || `<tr><td colspan="7" class="sub">No snap of this kind.</td></tr>`}</tbody></table></div><span class="sub">${plays.length} of ${(b.plays || []).length} snaps</span></section>`;
}

function tabtablesHtml(b, id) {
  const tabs = b.tabs || [];
  if (!tabs.length) return "";
  const k = Math.min(b.t || 0, tabs.length - 1);
  const t = tabs[k];
  const opts = tabs.map((x, i) => [String(i), `${x.name} (${x.rows.length})`]);
  return `<section class="panel sec" id="blk-${id}">${headHtml(b)}${segHtml(id, "t", opts, String(k))}<span class="sub">${esc(t.note || "")}</span>${tableHtml(t)}</section>`;
}

// A club's seasons: colour by win share, a ring for the playoffs, gold for
// a title.
function stripHtml(b) {
  const items = (b.items || []).map((s) => {
    const g = s.w + s.l + s.t, pct = g ? (s.w + s.t / 2) / g : 0.5;
    const bg = pct >= 0.5 ? `rgba(48,209,88,${(0.08 + (pct - 0.5) * 1.1).toFixed(2)})` : `rgba(255,55,95,${(0.08 + (0.5 - pct) * 1.1).toFixed(2)})`;
    const ring = s.round >= 5 ? "strip-gold" : s.round > 0 ? "strip-po" : "";
    return `<a class="strip-it ${ring}" href="#/${esc(s.link)}" style="background:${bg}" title="${esc(s.text)}"><b>${s.year}</b><span>${s.w}-${s.l}${s.t ? "-" + s.t : ""}</span></a>`;
  }).join("");
  return `<div class="strip">${items}</div>`;
}

function picksHtml(b) {
  const rounds = new Map();
  (b.picks || []).forEach((p) => { if (!rounds.has(p.round)) rounds.set(p.round, []); rounds.get(p.round).push(p); });
  return [...rounds.entries()].map(([r, ps]) => `<div class="pk-r"><span class="lbl">Round ${r} · ${ps.length} picks</span><div class="pk-g">${ps.map((p) =>
    `<a class="pk" href="#/${esc(p.link)}"><span class="pk-n num">${r}.${String(p.pick).padStart(2, "0")}</span><span class="sub">${esc(p.club)}</span><b>${esc(p.name)}</b><span>${esc(p.pos)}${p.college ? " · " + esc(p.college) : ""}</span><span class="sub">${esc(p.status || "")}${p.now ? " · " + esc(p.now) : ""}${p.ovr != null ? " · overall " + esc(p.ovr) : ""}</span></a>`).join("")}</div></div>`).join("");
}

(function () {
  const base = window.extraBlock;
  window.extraBlock = function (b, id) {
    switch (b.kind) {
      case "winline": return `<section class="panel sec">${headHtml(b)}${winlineHtml(b)}</section>`;
      case "drives": return drivesHtml(b, id);
      case "pbp": return pbpHtml(b, id);
      case "tabtables": return tabtablesHtml(b, id);
      case "strip": return `<section class="panel sec">${headHtml(b)}${stripHtml(b)}</section>`;
      case "picks": return `<section class="panel sec">${headHtml(b)}${picksHtml(b)}</section>`;
      default: return base ? base(b, id) : "";
    }
  };
})();

document.addEventListener("click", (e) => {
  const t = e.target.closest("[data-gseg]");
  if (!t) return;
  const b = PAGE.blocks[t.dataset.gseg];
  if (!b) return;
  b[t.dataset.k] = t.dataset.k === "t" ? +t.dataset.v : t.dataset.v;
  redrawBlock(t.dataset.gseg);
});
