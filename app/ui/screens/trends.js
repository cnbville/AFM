/* Trends, Compare, Career comps and the Stat finder (lane NB10, the Data
 * overhaul's items 29-32; boards 52, 54, 55, 56 and 58). Trends draws the
 * league's years with the middle half of teams, a team's last ten seasons
 * against its league and a player against the usual curve at his spot.
 * Compare puts two to four players or teams side by side; a player's
 * Trends tab carries his curve and the careers whose seasons looked like
 * his. The Stat finder searches seasons, careers, games and streaks, and
 * keeps saved searches that check again once a game day has passed. Every
 * class is tr-; every stat name explains itself on hover (data-h8). */

Object.assign(I, {
  trends: '<path d="M4 20h16"/><path d="M4 16l5-6 4 3 7-8"/>',
  dcompare: '<path d="M8 4v16M16 4v16"/><path d="M4 9h8M12 15h8"/>',
  statfind: '<circle cx="10" cy="10" r="6"/><path d="M15 15l5 5M7.5 10h5"/>',
});
navAdd({ go: "trends", label: "Trends", icon: "trends", sec: "data", after: "teamlead" });
navAdd({ go: "dcompare", label: "Compare", icon: "dcompare", sec: "data", after: "trends" });
navAdd({ go: "statfind", label: "Stat finder", icon: "statfind", sec: "data", after: "dcompare" });
Object.assign(CRUMBS, { trends: "Trends", dcompare: "Compare", statfind: "Stat finder" });

const trH8 = (k) => (k ? ` data-h8="${esc(k)}" tabindex="0"` : "");
const trWho = (r) => (r.linked ? playerLink(r.id, r.name) : esc(r.name));

// ---------- the line chart ----------
// One measure, one axis: a chart per number. Lines 2px, markers 8px, the
// middle half a quiet band, a crosshair and a tooltip on every season.
const TR_TIPS = {};
let trSeq = 0;
function trChart({ xs, series, band, fmt, xfmt = String, tips, label, h = 200, w = 560 }) {
  const vals = [...series.flatMap((s) => s.pts.map((p) => p[1])), ...(band || []).flatMap((b) => [b[1], b[2]])].filter((v) => v != null && Number.isFinite(v));
  if (!vals.length || !xs.length) return '<div class="empty small">Nothing counted yet.</div>';
  const W = w, H = h, L = 58, R = 70, T = 10, B = 26;
  let lo = Math.min(...vals), hi = Math.max(...vals);
  const pad = (hi - lo) * 0.1 || Math.abs(hi) * 0.05 || 1;
  lo -= pad; hi += pad;
  const x0 = Math.min(...xs), x1 = Math.max(...xs);
  const X = (x) => (x1 === x0 ? (L + W - R) / 2 : L + ((x - x0) / (x1 - x0)) * (W - L - R));
  const Y = (v) => T + (1 - (v - lo) / (hi - lo)) * (H - T - B);
  const grid = [0, 1, 2, 3, 4].map((k) => {
    const v = lo + ((hi - lo) * k) / 4;
    return `<line class="tr-grid" x1="${L}" x2="${W - R}" y1="${Y(v).toFixed(1)}" y2="${Y(v).toFixed(1)}"/><text class="tr-yl" x="${L - 6}" y="${(Y(v) + 3.5).toFixed(1)}">${esc(fmt(v))}</text>`;
  }).join("");
  const step = Math.max(1, Math.ceil(xs.length / 9));
  const xl = xs.map((x, i) => (i % step === 0 || i === xs.length - 1 ? `<text class="tr-xl" x="${X(x).toFixed(1)}" y="${H - 6}">${esc(xfmt(x))}</text>` : "")).join("");
  let area = "";
  if (band && band.length) {
    const b = band.filter((x) => x[1] != null && x[2] != null);
    if (b.length > 1) area = `<path class="tr-band" d="M${b.map((x) => `${X(x[0]).toFixed(1)},${Y(x[2]).toFixed(1)}`).join("L")}L${b.slice().reverse().map((x) => `${X(x[0]).toFixed(1)},${Y(x[1]).toFixed(1)}`).join("L")}Z"/>`;
    else if (b.length === 1) area = `<line class="tr-bandv" x1="${X(b[0][0])}" x2="${X(b[0][0])}" y1="${Y(b[0][2])}" y2="${Y(b[0][1])}"/>`;
  }
  const lines = series.map((s) => {
    // Break the line where a season is missing.
    const segs = [];
    let cur = [];
    for (const x of xs) {
      const p = s.pts.find((q) => q[0] === x);
      if (p && p[1] != null && Number.isFinite(p[1])) cur.push(p);
      else if (cur.length) { segs.push(cur); cur = []; }
    }
    if (cur.length) segs.push(cur);
    const path = segs.map((g) => `<polyline class="tr-line ${s.cls}${s.dash ? " dash" : ""}" points="${g.map((p) => `${X(p[0]).toFixed(1)},${Y(p[1]).toFixed(1)}`).join(" ")}"/>`).join("");
    const dots = s.dots === false ? "" : s.pts.filter((p) => p[1] != null && Number.isFinite(p[1])).map((p) => `<circle class="tr-dot ${s.cls}" cx="${X(p[0]).toFixed(1)}" cy="${Y(p[1]).toFixed(1)}" r="4"/>`).join("");
    const last = s.pts.filter((p) => p[1] != null && Number.isFinite(p[1])).pop();
    const tag = series.length > 1 && last && s.name ? `<text class="tr-tag" x="${(X(last[0]) + 8).toFixed(1)}" y="${(Y(last[1]) + 3.5).toFixed(1)}">${esc(s.name)}</text>` : "";
    return path + dots + tag;
  }).join("");
  const id = "trc" + ++trSeq;
  TR_TIPS[id] = tips || [];
  const hits = xs.map((x, i) => {
    const a = i === 0 ? L : (X(xs[i - 1]) + X(x)) / 2, b = i === xs.length - 1 ? W - R : (X(x) + X(xs[i + 1])) / 2;
    return `<rect class="tr-hit" data-c="${id}" data-i="${i}" data-x="${X(x).toFixed(1)}" x="${a.toFixed(1)}" y="${T}" width="${Math.max(1, b - a).toFixed(1)}" height="${H - T - B}"/>`;
  }).join("");
  return `<div class="tr-chart${w > 600 ? " wide" : ""}" id="${id}"><svg viewBox="0 0 ${W} ${H}" role="img" aria-label="${esc(label || "")}">${grid}${area}${xl}<line class="tr-cross" x1="0" x2="0" y1="${T}" y2="${H - B}" visibility="hidden"/>${lines}${hits}</svg><div class="tr-tip" hidden></div></div>`;
}
function trLegend(items) {
  return `<div class="tr-legend">${items.map((i) => `<span${trH8(i.h8)}><i class="tr-sw ${i.cls}"></i>${esc(i.name)}</span>`).join("")}</div>`;
}
document.addEventListener("mouseover", (ev) => {
  const t = ev.target;
  if (!t.classList || !t.classList.contains("tr-hit")) return;
  const box = document.getElementById(t.dataset.c);
  if (!box) return;
  const tip = box.querySelector(".tr-tip"), cross = box.querySelector(".tr-cross");
  const html = (TR_TIPS[t.dataset.c] || [])[Number(t.dataset.i)];
  if (!html) { tip.hidden = true; return; }
  const svg = box.querySelector("svg");
  const vb = svg.viewBox.baseVal.width || 560;
  const x = Number(t.dataset.x);
  cross.setAttribute("x1", x); cross.setAttribute("x2", x); cross.setAttribute("visibility", "visible");
  tip.innerHTML = html;
  tip.hidden = false;
  const left = (x / vb) * box.clientWidth;
  tip.style.left = `${Math.min(Math.max(8, left + 12), box.clientWidth - tip.offsetWidth - 8)}px`;
});
document.addEventListener("mouseout", (ev) => {
  const t = ev.target;
  if (!t.classList || !t.classList.contains("tr-hit")) return;
  const box = document.getElementById(t.dataset.c);
  if (!box || (ev.relatedTarget && ev.relatedTarget.classList && ev.relatedTarget.classList.contains("tr-hit"))) return;
  box.querySelector(".tr-tip").hidden = true;
  box.querySelector(".tr-cross").setAttribute("visibility", "hidden");
});
// The y labels read the number's own text: the server's format, rebuilt
// from the row's text for its unit.
function trFmtLike(sample) {
  const s = String(sample || "");
  if (s.endsWith("%")) return (v) => `${(100 * v).toFixed(0)}%`;
  if (s.endsWith(" s")) return (v) => `${v.toFixed(2)} s`;
  if (s.endsWith(" lb")) return (v) => `${v.toFixed(0)} lb`;
  if (/^\d+:\d\d$/.test(s)) return (v) => `${Math.floor(Math.round(v) / 60)}:${String(Math.round(v) % 60).padStart(2, "0")}`;
  if (/'\d/.test(s)) return (v) => `${Math.floor(v / 12)}'${(v - 12 * Math.floor(v / 12)).toFixed(1)}"`;
  if (/^[+-]/.test(s)) return (v) => `${v >= 0 ? "+" : ""}${v.toFixed(2)}`;
  if (/\.\d\d$/.test(s)) return (v) => v.toFixed(2);
  if (/\.\d$/.test(s)) return (v) => v.toFixed(1);
  return (v) => (Math.abs(v) >= 100 ? Math.round(v).toLocaleString() : v.toFixed(1));
}

// ---------- the number picker (up to three) ----------
function trPicker(act, menu, stats) {
  const opts = (cur) => menu.map((g) => `<optgroup label="${esc(g.group)}">${g.items.map((n) => `<option value="${esc(n.key)}" ${n.key === cur ? "selected" : ""}>${esc(n.label)}</option>`).join("")}</optgroup>`).join("");
  const chips = stats.map((s, i) => `<span class="tr-pick"><select class="ssel" data-act="${act}" data-i="${i}">${opts(s.key)}</select>${stats.length > 1 ? `<button class="tbtn" data-act="${act}x" data-i="${i}" title="Take this number off">×</button>` : ""}</span>`).join("");
  const add = stats.length < 3 ? `<select class="ssel tr-add" data-act="${act}" data-i="${stats.length}"><option value="">Add a number</option>${opts(null)}</select>` : "";
  return `<div class="tbar tr-picks">${chips}${add}</div>`;
}

// ---------- Trends (item 29, board 58) ----------
const TR_TABS = ["League", "Team", "Player"];
screens.trends = async (a) => {
  const K = keep("trends", {});
  for (const k of ["level", "division", "state"]) if (a[k] != null) { K[k] = a[k]; delete a[k]; }
  if (a.kind && a.id != null) { K.team = { kind: a.kind, id: Number(a.id) }; delete a.kind; a.tab = "Team"; }
  if (a.pid != null) { K.pid = Number(a.pid); delete a.pid; a.tab = "Player"; }
  const tab = tabOf("trtab", TR_TABS, a);
  title("Trends", tab);
  let body = "";
  if (tab === "League") body = await trLeague(K);
  else if (tab === "Team") body = await trTeam(K);
  else body = await trPlayerTab(K);
  const sub = { League: "How the game has changed: each number's average team by season, with the middle half of teams shaded.", Team: "A team's last ten seasons against its league, each number ranked.", Player: "A player's seasons against the usual path at his spot, age by age." }[tab];
  return `<div class="pad sl dt tr">${pageHead({ kicker: "Data", title: "Trends", sub, goto: gotoLine([["Compare", "dcompare"], ["Stat finder", "statfind"], ["Team leaderboards", "teamlead"]]) })}${tabs("trtab", TR_TABS, tab)}<div class="tr-body">${body}</div></div>`;
};
GOTO.trends = () => [["Compare", "dcompare"], ["Stat finder", "statfind"], ["Team leaderboards", "teamlead"]];

async function trLeague(K) {
  const x = await api("trends", { tab: "league", level: dtLevel(K), division: K.division || undefined, state: K.state || undefined, stats: K.stats || undefined });
  K.stats = x.stats.map((s) => s.key);
  const where = x.level === "college" ? `<select class="ssel" data-act="trsel" data-k="division">${x.divisions.map((d) => `<option ${d === x.division ? "selected" : ""}>${esc(d)}</option>`).join("")}</select>` : x.level === "hs" ? `<select class="ssel" data-act="trsel" data-k="state">${Object.keys(STATE_NAMES).map((c) => `<option value="${c}" ${c === x.state ? "selected" : ""}>${esc(STATE_NAMES[c])}</option>`).join("")}</select>` : "";
  const bar = `<div class="tbar">${dtSeg("trlevel", DT_LEVELS, x.level)}${where}<span class="tspace"></span><span class="muted small">${x.rows.length} seasons · ${esc(x.scope)}</span></div>`;
  if (!x.rows.length) return `${bar}<div class="empty">No season counted at this level yet.</div>`;
  const ys = x.rows.map((r) => r.year);
  const charts = x.stats.map((s, k) => {
    const pts = x.rows.map((r) => [r.year, r.cells[k] ? r.cells[k].v : null]);
    const band = x.rows.map((r) => [r.year, r.cells[k] ? r.cells[k].lo : null, r.cells[k] ? r.cells[k].hi : null]);
    const sample = (x.rows.find((r) => r.cells[k]) || { cells: [] }).cells[k];
    const tips = x.rows.map((r) => (r.cells[k] ? `<b>${r.year}</b><span>Average team <b>${esc(r.cells[k].text)}</b></span><span>Middle half ${esc(r.cells[k].lo_text)} to ${esc(r.cells[k].hi_text)}</span><small>${r.cells[k].n} teams</small>` : ""));
    const first = x.rows.find((r) => r.cells[k]), last = x.rows.slice().reverse().find((r) => r.cells[k]);
    const change = first && last && first !== last ? `<small class="muted">${first.year} ${esc(first.cells[k].text)} → ${last.year} ${esc(last.cells[k].text)}</small>` : "";
    return `<section class="panel dt-panel tr-panel"><header><h2${trH8(s.h8)}>${esc(s.label)}</h2>${change}</header>${trChart({ xs: ys, series: [{ name: "Average team", cls: "tr-s1", pts }], band, fmt: trFmtLike(sample && sample.text), tips, label: `${s.label} by season` })}</section>`;
  }).join("");
  const legend = trLegend([{ name: "Average team", cls: "tr-s1" }, { name: "Middle half of teams", cls: "tr-bandsw", h8: "TrBand" }]);
  const heads = ["Season", ...x.stats.map((s) => s.label)];
  const rows = x.rows.slice().reverse().map((r) => [r.year, ...r.cells.map((c) => (c ? `${esc(c.text)} <small class="muted">${esc(c.lo_text)}–${esc(c.hi_text)}</small>` : "–"))]);
  const table = `<div class="tbl"><table><thead><tr>${heads.map((h, i) => `<th class="${i ? "" : "l"}"${trH8(i ? x.stats[i - 1].h8 : "")}>${esc(h)}</th>`).join("")}</tr></thead><tbody>${rows.map((r) => `<tr>${r.map((v, i) => `<td class="${i ? "" : "l"}">${v}</td>`).join("")}</tr>`).join("")}</tbody></table></div>`;
  const src = x.depth < 2 ? '<p class="dt-src">Formation, personnel and coverage numbers need film charting at this level; your seat has ' + (x.depth ? "play by play" : "box scores") + " here.</p>" : "";
  return `${bar}${trPicker("trpick", x.menu, x.stats)}${legend}<div class="tr-grid3">${charts}</div><section class="panel dt-panel"><h2>Season by season</h2>${table}</section>${src}`;
}

async function trTeam(K) {
  const t = K.team || (S.status && S.status.mine ? S.status.mine.ref : null);
  const finder = `<div class="tbar tr-find"><input class="ssel" type="search" placeholder="Find a team" data-act="trfind" data-kind="teams" data-into="trteam"><div class="tr-hits" id="trteam-hits"></div></div>`;
  if (!t) return `${finder}<div class="empty">Pick a team to see its seasons.</div>`;
  const x = await api("trends", { tab: "team", kind: t.kind, id: t.id, stats: K.tstats || undefined, all: K.all || undefined });
  if (x.error) return `${finder}<div class="empty">${esc(x.error)}</div>`;
  K.tstats = x.stats.map((s) => s.key);
  const ys = x.rows.map((r) => r.year);
  const charts = x.stats.map((s, k) => {
    const pts = x.rows.map((r) => [r.year, r.cells[k].v]);
    const mean = x.rows.map((r) => [r.year, r.cells[k].group ? r.cells[k].group.v : null]);
    const band = x.rows.map((r) => [r.year, r.cells[k].group ? r.cells[k].group.lo : null, r.cells[k].group ? r.cells[k].group.hi : null]);
    const sample = (x.rows.find((r) => r.cells[k].text) || { cells: [] }).cells[k];
    const tips = x.rows.map((r) => {
      const c = r.cells[k];
      return `<b>${r.year}${r.record ? ` · ${esc(r.record)}` : ""}</b><span>${esc(x.team.name)} <b>${esc(c.text || "–")}</b>${c.rank ? ` · ${c.rank} of ${c.of}` : ""}</span>${c.group ? `<span>${esc(r.group)} average ${esc(c.group.text)}</span><small>Middle half ${esc(c.group.lo_text)} to ${esc(c.group.hi_text)}</small>` : ""}`;
    });
    return `<section class="panel dt-panel tr-panel"><header><h2${trH8(s.h8)}>${esc(s.label)}</h2><small class="muted">${s.better === "high" ? "Higher is better" : s.better === "low" ? "Lower is better" : ""}</small></header>${trChart({ xs: ys, series: [{ name: x.team.name, cls: "tr-s1", pts }, { name: "League", cls: "tr-ref", pts: mean, dash: true, dots: false }], band, fmt: trFmtLike(sample && sample.text), tips, label: `${x.team.name}: ${s.label} by season` })}</section>`;
  }).join("");
  const legend = trLegend([{ name: x.team.name, cls: "tr-s1" }, { name: "League average", cls: "tr-ref" }, { name: "Middle half of its league", cls: "tr-bandsw", h8: "TrBand" }]);
  const heads = ["Season", "League", "Record", ...x.stats.map((s) => s.label)];
  const rows = x.rows.slice().reverse().map((r) => [r.year, esc(r.group || ""), esc(r.record || ""), ...r.cells.map((c) => `${esc(c.text || "–")}${c.rank ? ` <small class="dt-rk2">${c.rank}</small>` : ""}`)]);
  const table = `<div class="tbl"><table><thead><tr>${heads.map((h, i) => `<th class="${i < 3 ? "l" : ""}"${trH8(i > 2 ? x.stats[i - 3].h8 : "")}>${esc(h)}</th>`).join("")}</tr></thead><tbody>${rows.map((r) => `<tr>${r.map((v, i) => `<td class="${i < 3 ? "l" : ""}">${v}</td>`).join("")}</tr>`).join("")}</tbody></table></div>`;
  const head = `<div class="tbar">${teamChip(x.team, 22)}<span class="tspace"></span>${dtSeg("trall", [["", "Last 10 seasons"], ["1", "Every season"]], K.all ? "1" : "")}${linkTo("teamdata", { kind: x.team.ref.kind, id: x.team.ref.id }, "Team data", "gochip")}</div>`;
  return `${finder}${head}${trPicker("trtpick", x.menu, x.stats)}${legend}<div class="tr-grid3">${charts}</div><section class="panel dt-panel"><h2>Season by season</h2>${table}<p class="muted small">The small number is the rank in its league that season.</p></section>`;
}

async function trPlayerTab(K) {
  const finder = `<div class="tbar tr-find"><input class="ssel" type="search" placeholder="Find a player" data-act="trfind" data-kind="players" data-into="trplayer"><div class="tr-hits" id="trplayer-hits"></div></div>`;
  if (K.pid == null) return `${finder}<div class="empty">Pick a player to lay his seasons over the usual curve at his spot.</div>`;
  return `${finder}<div class="tbar">${playerLink(K.pid, "Player page")}<span class="tspace"></span></div>${await trCurve(K.pid, keep("trp:" + K.pid, {}))}`;
}

// A player's seasons against the usual curve, one number at a time, and
// the careers whose seasons looked like his (items 29 and 31).
async function trCurve(id, P) {
  let x;
  try { x = await api("player_trend", { id, level: P.level || undefined, group: P.group || undefined }); } catch (e) { return `<div class="empty">${esc(e.message || "No numbers.")}</div>`; }
  if (x.error) return `<div class="empty">${esc(x.error)}</div>`;
  const k = Math.min(P.metric || 0, x.metrics.length - 1);
  const m = x.metrics[k];
  const ages = [...new Set([...x.seasons.map((s) => s.age), ...x.curve.map((c) => c.age)].filter((a) => a != null))].sort((a, b) => a - b);
  const mine = x.seasons.filter((s) => s.age != null).map((s) => [s.age, s.v[k]]);
  const usual = x.curve.map((c) => [c.age, c.v[k]]);
  const fmt = trFmtLike((x.seasons.find((s) => s.text[k]) || { text: [] }).text[k]);
  const tips = ages.map((age) => {
    const s = x.seasons.filter((q) => q.age === age);
    const c = x.curve.find((q) => q.age === age);
    return `<b>Age ${age}</b>${s.map((q) => `<span>${q.year}${q.class ? ` · ${esc(q.class)}` : ""} <b>${esc(q.text[k] || "–")}</b>${q.pct[k] != null ? ` · ${q.pct[k]}th pct` : ""}</span>`).join("")}${c && c.text[k] ? `<span>Usual ${esc(c.text[k])}</span><small>${c.n} seasons</small>` : ""}`;
  });
  const metricSel = `<select class="ssel" data-act="trmetric" data-pid="${id}">${x.metrics.map((q, i) => `<option value="${i}" ${i === k ? "selected" : ""}>${esc(q.label)}</option>`).join("")}</select>`;
  const chart = trChart({ xs: ages, series: [{ name: x.name, cls: "tr-s1", pts: mine }, { name: "Usual", cls: "tr-ref", pts: usual, dash: true, dots: false }], fmt, xfmt: (a) => `${a}`, tips, label: `${x.name}: ${m.label} by age against the usual curve`, w: 900, h: 250 });
  const legend = trLegend([{ name: x.name, cls: "tr-s1" }, { name: `Usual ${x.group} at each age`, cls: "tr-ref", h8: "TrCurve" }]);
  const heads = ["Season", "Age", "Class", "Team", x.volume, ...x.metrics.map((q) => q.label)];
  const rows = x.seasons.slice().reverse().map((s) => `<tr><td class="l">${s.year}</td><td>${s.age ?? ""}</td><td class="l">${esc(s.class || "")}</td><td class="l">${s.team ? teamA(s.team) : ""}</td><td>${s.vol}${s.qualified ? "" : ' <small class="muted" title="Below the minimum that season">·</small>'}</td>${s.text.map((t, i) => `<td>${esc(t || "–")}${s.pct[i] != null ? ` <small class="dt-rk2" title="Percentile among ${s.field} qualifiers">${s.pct[i]}</small>` : ""}</td>`).join("")}</tr>`).join("");
  const table = `<div class="tbl"><table><thead><tr>${heads.map((h, i) => `<th class="${i === 0 || i === 2 || i === 3 ? "l" : ""}"${trH8(i > 4 ? x.metrics[i - 5].key : i === 4 ? "Vol" : "")}>${esc(h)}</th>`).join("")}</tr></thead><tbody>${rows}</tbody></table></div>`;
  const comps = await trComps(id, P, x);
  return `<section class="panel dt-panel tr-panel"><header><div><div class="st-label">${esc(x.group)} card · ${x.charted ? "charted" : "box score"}${x.since ? ` · seasons since ${x.since}` : ""}</div><h2${trH8(m.key)}>${esc(m.label)} by age</h2></div>${metricSel}</header>${legend}${chart}<p class="muted small">The usual curve is the average qualifier at his position at each age (an age shows once ${x.curve_min} seasons count). ${esc(x.outcome.text)}.</p></section>
    <section class="panel dt-panel"><h2>Season by season</h2>${table}<p class="muted small">The small number is his percentile among that season's qualifiers.</p></section>${comps}`;
}

async function trComps(id, P, t) {
  let c;
  try { c = await api("career_comps", { id, level: P.level || undefined, group: P.group || undefined, seasons: P.seasons || undefined, mode: P.mode || undefined, drafted: P.drafted || undefined, n: 8 }); } catch (e) { return ""; }
  if (c.error) return `<section class="panel dt-panel"><h2>Career comps</h2><p class="muted">${esc(c.error)}</p></section>`;
  const nSel = c.most > 1 ? dtSeg("trcompn", Array.from({ length: c.most }, (_, i) => [i + 1, `${i + 1} season${i ? "s" : ""}`]), c.seasons_n).replace(/data-act="trcompn"/g, `data-act="trcompn" data-pid="${id}"`) : "";
  const mode = dtSeg("trcompm", [["first", "His first seasons"], ["last", "His latest seasons"]], c.mode).replace(/data-act="trcompm"/g, `data-act="trcompm" data-pid="${id}"`);
  const drafted = `<label class="tr-chk"><input type="checkbox" data-act="trcompd" data-pid="${id}" ${P.drafted ? "checked" : ""}> Drafted only</label>`;
  const line = (s) => `${s.year}${s.class ? ` ${esc(s.class)}` : ""}${s.age != null ? `, age ${s.age}` : ""}`;
  const mine = `<li class="tr-me"><span class="tr-cw"><b>${esc(c.name)}</b><small>${c.mine.map(line).join(" · ")}</small></span>${c.mine.map((s) => `<span class="tr-cs">${s.text.map((v, i) => `<em${trH8(c.metrics[i].key)}>${esc(c.metrics[i].label)} <b>${esc(v || "–")}</b></em>`).slice(0, 4).join("")}</span>`).join("")}</li>`;
  const rows = c.comps.map((o) => `<li><span class="tr-cw">${trWho(o)}<small>${o.team ? teamA(o.team) : ""} · ${o.seasons.map(line).join(" · ")}</small></span>${o.seasons.map((s) => `<span class="tr-cs">${s.text.map((v, i) => `<em>${esc(c.metrics[i].label)} <b>${esc(v || "–")}</b></em>`).slice(0, 4).join("")}</span>`).join("")}<span class="tr-next"><small>What came next</small>${esc(o.outcome.text)}</span><b class="tr-match" data-h8="CompMatch" tabindex="0">Match ${o.match.toFixed(2).replace(/^0/, "")}</b></li>`).join("");
  const cmp = c.comps.length ? linkTo("dcompare", { ids: [id, ...c.comps.slice(0, 3).map((o) => o.id)].join(",") }, "Compare with the top three", "gochip") : "";
  return `<section class="panel dt-panel tr-comps"><header><div><h2>Career comps</h2><small class="muted">Players whose ${c.mode === "first" ? "first" : "latest"} ${c.seasons_n === 1 ? "season" : `${c.seasons_n} seasons`} at the level read like his, by percentile in every ${esc(c.group)} number and playing time, and what became of them.</small></div>${cmp}</header><div class="tbar">${mode}${nSel}${drafted}</div><ol class="tr-clist">${mine}${rows || '<li class="muted">No finished career reads close yet.</li>'}</ol></section>`;
}

// The player page's Trends tab: his curve and his comps.
PLAYER_TABS.push({ name: "Trends", after: "Data", draw: async (p) => `<div class="sl dt tr">${await trCurve(p.id, keep("trp:" + p.id, {}))}</div>` });

// ---------- Compare (item 30, board 54) ----------
const CMP_TABS = ["Players", "Teams"];
const CMP_LINES = [["class", "By class"], ["age", "By age"], ["year", "One season"], ["career", "Career"]];
screens.dcompare = async (a) => {
  const K = keep("dcompare", { ids: [], teams: [] });
  if (a.ids) { K.ids = String(a.ids).split(",").map(Number).filter((n) => !Number.isNaN(n)).slice(0, 4); delete a.ids; a.tab = "Players"; }
  if (a.id != null) { if (!K.ids.includes(Number(a.id))) K.ids = [...K.ids, Number(a.id)].slice(-4); delete a.id; a.tab = "Players"; }
  if (a.kind && a.tid != null) { const t = { kind: a.kind, id: Number(a.tid) }; if (!K.teams.some((x) => x.kind === t.kind && x.id === t.id)) K.teams = [...K.teams, t].slice(-4); delete a.kind; delete a.tid; a.tab = "Teams"; }
  const tab = tabOf("cmptab", CMP_TABS, a);
  title("Compare", tab);
  const body = tab === "Players" ? await cmpPlayers(K) : await cmpTeams(K);
  return `<div class="pad sl dt tr">${pageHead({ kicker: "Data", title: "Compare", sub: "Two to four players or teams side by side: percentiles in their own league that season, seasons lined up by class or age, careers and what came next.", goto: gotoLine([["Trends", "trends"], ["Stat finder", "statfind"], ["Player analytics", "analytics"]]) })}${tabs("cmptab", CMP_TABS, tab)}<div class="tr-body">${body}</div></div>`;
};
GOTO.dcompare = () => [["Trends", "trends"], ["Stat finder", "statfind"], ["Player analytics", "analytics"]];
const cmpColor = (i) => `tr-s${i + 1}`;

async function cmpPlayers(K) {
  const add = K.ids.length < 4 ? `<div class="tr-find"><input class="ssel" type="search" placeholder="Add a player" data-act="trfind" data-kind="players" data-into="cmpp"><div class="tr-hits" id="cmpp-hits"></div></div>` : "";
  const x = K.ids.length ? await api("data_compare", { kind: "players", ids: K.ids, line: K.line || "class", at: K.at != null ? Number(K.at) : undefined }) : { players: [], empty: "Add two to four players." };
  const chips = (x.players || []).map((p, i) => `<span class="tr-chip"><i class="tr-sw ${cmpColor(i)}"></i>${trWho(p)}<button class="tbtn" data-act="cmpx" data-i="${i}" title="Take him off">×</button></span>`).join("");
  const bar = `<div class="tbar">${chips}${add}</div>`;
  if (x.error || x.empty || !x.players.length) return `${bar}<div class="empty">${esc(x.error || x.empty || "Add two to four players.")}</div>`;
  const P = x.players;
  // Which season to line up: a class, an age or a year everyone shares.
  let atSel = "";
  if (x.line === "age") {
    const ages = [...new Set(P.flatMap((p) => p.seasons.map((s) => s.age)).filter((v) => v != null))].sort((a, b) => a - b);
    atSel = `<select class="ssel" data-act="cmpat">${ages.map((v) => `<option value="${v}" ${v === (x.at ?? P[0].season?.age) ? "selected" : ""}>Age ${v}</option>`).join("")}</select>`;
  } else if (x.line === "year") {
    const ys = [...new Set(P.flatMap((p) => p.seasons.map((s) => s.year)))].sort((a, b) => b - a);
    atSel = `<select class="ssel" data-act="cmpat">${ys.map((v) => `<option value="${v}" ${v === (x.at ?? P[0].season?.year) ? "selected" : ""}>${v}</option>`).join("")}</select>`;
  } else if (x.line === "class") {
    const n = Math.max(1, ...P.flatMap((p) => p.seasons.map((s) => s.class_n || 1)));
    atSel = `<select class="ssel" data-act="cmpat">${Array.from({ length: n }, (_, i) => i + 1).map((v) => `<option value="${v}" ${v === (x.at ?? 1) ? "selected" : ""}>${esc(trYearWord(v, x.level))}</option>`).join("")}</select>`;
  }
  const tools = `<div class="tbar">${dtSeg("cmpline", CMP_LINES, x.line)}${atSel}<span class="tspace"></span><span class="muted small">${esc(x.group)} card${x.charted ? ", charted" : ", box score"}</span></div>`;
  const legend = trLegend(P.map((p, i) => ({ name: `${p.name}${p.label ? ` (${p.label})` : ""}`, cls: cmpColor(i) })));
  const rows = x.metrics.map((m, k) => `<li><span class="dt-ml"${trH8(m.key)}>${esc(m.label)}</span><span class="tr-bars">${P.map((p, i) => {
    const c = p.cells[k] || {};
    const w = c.pct != null ? c.pct : 0;
    return `<span class="tr-bar"><i class="${cmpColor(i)}" style="width:${w}%"></i><b>${esc(c.text || "–")}</b><small>${c.pct != null ? c.pct : ""}</small></span>`;
  }).join("")}</span></li>`).join("");
  const seasonLine = P.map((p, i) => `<span class="tr-chip"><i class="tr-sw ${cmpColor(i)}"></i>${esc(p.name)}: ${p.season ? `${p.season.year}${p.season.class ? ` ${esc(p.season.class)}` : ""}${p.season.age != null ? `, age ${p.season.age}` : ""}, ${p.season.games} game${p.season.games === 1 ? "" : "s"}${p.season.team ? ` · ${teamA(p.season.team)}` : ""}` : x.line === "career" ? esc(p.label || "No seasons") : "no season there"}</span>`).join("");
  const barsPanel = `<section class="panel dt-panel"><h2>${x.line === "career" ? "Careers at the level" : "Side by side"}</h2><div class="tr-line2">${seasonLine}</div>${legend}<ul class="tr-cmp">${rows}</ul><p class="muted small">${x.line === "career" ? "Career numbers at the level; percentiles are for single seasons, so a career shows its numbers only." : "Bars are percentiles among that season's qualifiers in each player's own division."}</p></section>`;
  // Every season lined up by class or age.
  const by = x.line === "age" ? "age" : "class";
  const keys = [...new Set(P.flatMap((p) => p.seasons.map((s) => (by === "age" ? s.age : s.class_n))).filter((v) => v != null))].sort((a, b) => a - b);
  const first = x.metrics.slice(0, 3);
  const lined = keys.map((v) => `<tr><td class="l">${by === "age" ? `Age ${v}` : esc(trYearWord(v, x.level))}</td>${P.map((p) => {
    const s = p.seasons.find((q) => (by === "age" ? q.age : q.class_n) === v);
    return s ? `<td class="l"><b>${s.year}</b> ${first.map((m, i) => `<span class="tr-mini">${esc(m.label)} ${esc(s.text[i] || "–")}${s.pct[i] != null ? ` <small>${s.pct[i]}</small>` : ""}</span>`).join("")}</td>` : '<td class="l muted">–</td>';
  }).join("")}</tr>`).join("");
  const linedPanel = `<section class="panel dt-panel"><h2>Every season, lined up by ${by}</h2><div class="tbl"><table><thead><tr><th class="l">${by === "age" ? "Age" : "Year"}</th>${P.map((p, i) => `<th class="l"><i class="tr-sw ${cmpColor(i)}"></i> ${esc(p.name)}</th>`).join("")}</tr></thead><tbody>${lined}</tbody></table></div></section>`;
  const next = `<section class="panel dt-panel"><h2>What came next</h2><ul class="tr-next2">${P.map((p, i) => `<li><i class="tr-sw ${cmpColor(i)}"></i>${trWho(p)}<span>${esc(p.outcome.text)}</span></li>`).join("")}</ul></section>`;
  return `${bar}${tools}${barsPanel}${linedPanel}${next}`;
}
function trYearWord(n, level) {
  if (level === "nfl") return n === 1 ? "Rookie year" : `Year ${n}`;
  return ["", "Freshman year", "Sophomore year", "Junior year", "Senior year", "Fifth year"][n] || `Year ${n}`;
}

async function cmpTeams(K) {
  const add = K.teams.length < 4 ? `<div class="tr-find"><input class="ssel" type="search" placeholder="Add a team" data-act="trfind" data-kind="teams" data-into="cmpt"><div class="tr-hits" id="cmpt-hits"></div></div>` : "";
  if (!K.teams.length && S.status && S.status.mine) K.teams = [S.status.mine.ref];
  const x = K.teams.length ? await api("data_compare", { kind: "teams", teams: K.teams, year: K.tyear || undefined }) : { teams: [], rows: [] };
  const chips = x.teams.map((t, i) => `<span class="tr-chip"><i class="tr-sw ${cmpColor(i)}"></i>${teamChip(t.team, 18)}<button class="tbtn" data-act="cmptx" data-i="${i}" title="Take it off">×</button></span>`).join("");
  const bar = `<div class="tbar">${chips}${add}</div>`;
  if (!x.teams.length) return `${bar}<div class="empty">Add two to four teams.</div>`;
  const years = [...new Set(x.teams.flatMap((t) => t.years || []))].sort((a, b) => b - a);
  const ysel = years.length ? `<select class="ssel" data-act="cmptyear"><option value="">Each team's latest season</option>${years.map((y) => `<option value="${y}" ${y === K.tyear ? "selected" : ""}>${y}</option>`).join("")}</select>` : "";
  const heads = `<tr><th class="l">Number</th>${x.teams.map((t, i) => `<th class="l"><i class="tr-sw ${cmpColor(i)}"></i> ${esc(t.team.name)}<small class="muted"> ${t.year || ""}${t.record ? ` · ${esc(t.record)}` : ""}${t.group ? ` · ${esc(t.group)}` : ""}</small></th>`).join("")}</tr>`;
  let unit = "";
  const body = x.rows.map((r) => {
    const cap = r.unit !== unit ? `<tr class="tr-unit"><td colspan="${x.teams.length + 1}">${esc(r.unit)}</td></tr>` : "";
    unit = r.unit;
    return `${cap}<tr><td class="l"${trH8(r.h8)}>${esc(r.label)}</td>${x.teams.map((t, i) => {
      if (t.empty) return '<td class="l muted">–</td>';
      const c = (t.cells || []).find((q) => q.key === r.key) || {};
      return `<td class="l"><span class="tr-bar sm"><i class="${cmpColor(i)}" style="width:${c.pct ?? 0}%"></i><b>${esc(c.text || "–")}</b><small>${c.rank ? `${c.rank} of ${t.of}` : ""}</small></span></td>`;
    }).join("")}</tr>`;
  }).join("");
  return `${bar}<div class="tbar">${ysel}<span class="tspace"></span><span class="muted small">Ranks and bars are within each team's own league that season.</span></div><section class="panel dt-panel"><div class="tbl"><table class="tr-ttbl"><thead>${heads}</thead><tbody>${body}</tbody></table></div></section>`;
}

// ---------- Stat finder (item 32, board 55) ----------
const SF_FINDS = [["seasons", "Seasons"], ["careers", "Careers"], ["games", "Games"], ["streaks", "Streaks"]];
const SF_POS = ["QB", "RB", "WR", "TE", "OL", "OT", "IOL", "DL", "DT", "EDGE", "LB", "DB", "CB", "S", "K", "P"];
let SF_STATS = null;
screens.statfind = async (a) => {
  const K = keep("statfind", { find: "seasons", conds: [] });
  if (a.q) { try { Object.assign(K, JSON.parse(a.q)); } catch (e) { /* a bad link keeps the last search */ } delete a.q; }
  if (!SF_STATS) SF_STATS = (await api("stat_finder_stats")).stats;
  title("Stat finder", SF_FINDS.find((f) => f[0] === K.find)[1]);
  const q = sfQuery(K);
  const x = await api("stat_finder", q);
  const level = x.level;
  const opt = (list, cur, all) => `${all ? `<option value="">${esc(all)}</option>` : ""}${list.map((v) => { const [k, t] = Array.isArray(v) ? v : [v, v]; return `<option value="${esc(k)}" ${String(cur ?? "") === String(k) ? "selected" : ""}>${esc(t)}</option>`; }).join("")}`;
  const statOpts = (cur) => {
    const groups = [...new Set(SF_STATS.map((s) => s.group))];
    return groups.map((g) => `<optgroup label="${esc(g)}">${SF_STATS.filter((s) => s.group === g).map((s) => `<option value="${esc(s.key)}" ${s.key === cur ? "selected" : ""}>${esc(s.name)}</option>`).join("")}</optgroup>`).join("");
  };
  const now = (S.status && S.status.season) || x.to;
  const first = x.since || now - 40;
  const years = [];
  for (let y = now; y >= first; y--) years.push(y);
  const where = level === "college" ? `<select class="ssel" data-act="sfset" data-k="division">${opt([["all", "Every division"], ...x.divisions.map((d) => [d, d])], x.division)}</select>` : level === "hs" ? `<select class="ssel" data-act="sfset" data-k="state">${opt(Object.keys(STATE_NAMES).map((c) => [c, STATE_NAMES[c]]), x.state)}</select>` : "";
  const cls = level !== "nfl" ? `<select class="ssel" data-act="sfset" data-k="class">${opt([["Fr", level === "hs" ? "Freshmen" : "Freshmen"], ["So", "Sophomores"], ["Jr", "Juniors"], ["Sr", "Seniors"], ...(level === "college" ? [["5", "Fifth years"]] : [])], K.class, "Every class")}</select>${level === "college" ? `<select class="ssel" data-act="sfset" data-k="rs">${opt([["yes", "Redshirted"], ["no", "Not redshirted"]], K.rs, "Redshirt or not")}</select>` : ""}` : "";
  const ages = Array.from({ length: 27 }, (_, i) => i + 14);
  const filters = `<div class="tbar sf-bar">${dtSeg("sflevel", DT_LEVELS, level)}${where}<select class="ssel" data-act="sfset" data-k="from" title="From">${opt(years.slice().reverse(), x.from)}</select><span class="muted small">to</span><select class="ssel" data-act="sfset" data-k="to" title="To">${opt(years, x.to)}</select><select class="ssel" data-act="sfset" data-k="pos">${opt(SF_POS, K.pos, "Every position")}</select>${cls}<select class="ssel" data-act="sfset" data-k="age_min" title="Youngest age">${opt(ages, K.age_min, "Any age")}</select><select class="ssel" data-act="sfset" data-k="age_max" title="Oldest age">${opt(ages, K.age_max, "to any age")}</select></div>`;
  const conds = K.conds.map((c, i) => `<span class="sf-cond"><select class="ssel" data-act="sfcond" data-i="${i}" data-k="stat">${statOpts(c.stat)}</select><select class="ssel" data-act="sfcond" data-i="${i}" data-k="op"><option value=">=" ${c.op !== "<=" ? "selected" : ""}>at least</option><option value="<=" ${c.op === "<=" ? "selected" : ""}>at most</option></select><input class="ssel sf-v" type="number" step="any" value="${esc(c.v ?? "")}" data-act="sfcondv" data-i="${i}"><button class="tbtn" data-act="sfdel" data-i="${i}" title="Take this condition off">×</button></span>`).join("");
  const streak = K.find === "streaks" ? `<label class="small muted">Games in a row, at least <input class="ssel sf-n" type="number" min="2" value="${esc(K.min_len || 3)}" data-act="sfminlen"></label>` : "";
  const condBar = `<div class="tbar sf-conds">${conds}${K.conds.length < 8 ? '<button class="tbtn" data-act="sfadd">+ Condition</button>' : ""}${streak}<span class="tspace"></span><button class="st-btn" data-act="sfrun">Search</button><button class="tbtn" data-act="sfsave">Save search</button></div>`;
  const saved = sfSavedPanel();
  let body;
  if (x.empty) body = `<div class="empty">${esc(x.empty)}</div>`;
  else if (!x.rows.length) body = '<div class="empty">Nothing meets every condition.</div>';
  else body = sfTable(x, K.find);
  const say = sfSay(K, x);
  // Saved searches are checked again once a game day has passed.
  setTimeout(() => sfCheckSaved(), 0);
  return `<div class="pad sl dt tr sf">${pageHead({ kicker: "Data", title: "Stat finder", sub: "Every season, career, game or streak that meets your conditions, at any level and any year the game has kept.", goto: gotoLine([["Trends", "trends"], ["Compare", "dcompare"], ["Stat history", "stathist"]]) })}${dtSeg("sffind", SF_FINDS, K.find)}${filters}${condBar}<p class="sf-say">${esc(say)} <span class="muted">${x.total.toLocaleString()} found${x.total > x.rows.length ? `, the top ${x.rows.length} shown` : ""}.</span></p>${body}${saved}</div>`;
};
GOTO.statfind = () => [["Trends", "trends"], ["Compare", "dcompare"], ["Stat history", "stathist"]];

function sfQuery(K) {
  const q = { level: dtLevel(K), find: K.find };
  for (const k of ["division", "state", "from", "to", "pos", "class", "rs", "age_min", "age_max", "sort", "min_len"]) if (K[k] != null && K[k] !== "") q[k] = ["from", "to", "age_min", "age_max", "min_len"].includes(k) ? Number(K[k]) : K[k];
  q.conds = K.conds.filter((c) => c.stat && c.v !== "" && c.v != null && !Number.isNaN(Number(c.v))).map((c) => ({ stat: c.stat, op: c.op || ">=", v: Number(c.v) }));
  if (K.dir != null) q.dir = K.dir;
  return q;
}
// The search in a sentence: "FBS freshmen with 1,000 rushing yards since 2016".
function sfSay(K, x) {
  const name = (k) => (SF_STATS.find((s) => s.key === k) || { name: k }).name.toLowerCase();
  const who = [x.level === "college" ? (x.division === "all" ? "College" : x.division) : x.level === "nfl" ? "NFL" : `${STATE_NAMES[x.state] || "High school"} high school`];
  if (K.class) who.push({ Fr: "freshmen", So: "sophomores", Jr: "juniors", Sr: "seniors", 5: "fifth years" }[K.class] || "");
  if (K.pos) who.push(K.pos);
  if (!K.class && !K.pos) who.push(K.find === "careers" ? "careers" : K.find === "games" ? "games" : K.find === "streaks" ? "streaks" : "seasons");
  const conds = sfQuery(K).conds.map((c) => `${c.op === "<=" ? "at most " : ""}${Number(c.v).toLocaleString()} ${name(c.stat)}`);
  const span = x.from === x.to ? `in ${x.from}` : `from ${x.from} to ${x.to}`;
  return `${who.join(" ")}${conds.length ? ` with ${conds.join(" and ")}` : ""}${K.find === "streaks" ? ` in ${K.min_len || 3}+ games in a row` : K.find === "games" && conds.length ? " in a game" : ""} ${span}.`;
}
function sfTable(x, find) {
  const vcols = x.cols.map((c, i) => ({ k: "c" + i, t: c.name, h8: c.key, v: (r) => (r.v ? r.v[i] : null), f: (r) => esc(r.vals ? r.vals[i] ?? "–" : "–") }));
  const who = { k: "name", t: "Player", l: true, f: (r) => trWho(r), v: (r) => r.name };
  const pos = { k: "pos", t: "Pos", l: true };
  const team = { k: "team", t: "Team", l: true, f: (r) => (r.team ? teamA(r.team) : ""), v: (r) => (r.team ? r.team.name : "") };
  let cols;
  if (find === "careers") cols = [who, pos, { k: "yrs", t: "Years", l: true, v: (r) => (r.extra ? `${r.extra.first}-${r.extra.last}` : ""), f: (r) => (r.extra ? (r.extra.first === r.extra.last ? `${r.extra.first}` : `${r.extra.first}-${r.extra.last}`) : "") }, { k: "n", t: "Seasons", v: (r) => (r.extra ? r.extra.seasons : null) }, team, { k: "games", t: "G", h8: "Games" }, ...vcols];
  else if (find === "games") cols = [{ k: "date", t: "Date", l: true, v: (r) => (r.extra ? r.extra.date : "") }, who, pos, team, { k: "opp", t: "Opponent", l: true, v: (r) => (r.extra && r.extra.opp ? r.extra.opp.name : ""), f: (r) => (r.extra ? `${r.extra.at ? r.extra.at + " " : ""}${r.extra.opp ? teamA(r.extra.opp) : "?"}` : "") }, { k: "res", t: "Result", l: true, v: (r) => (r.extra ? r.extra.result : "") }, ...vcols];
  else if (find === "streaks") cols = [who, pos, team, { k: "len", t: "Games", tip: "Games in a row that met every condition" }, { k: "from", t: "From", l: true }, { k: "to", t: "To", l: true, f: (r) => `${esc(r.to)}${r.running ? ' <span class="tag soft" title="Still going">running</span>' : ""}` }, { k: "seasons", t: "Seasons", l: true }];
  else cols = [{ k: "year", t: "Year", l: true }, who, pos, team, { k: "class", t: "Class", l: true }, { k: "age", t: "Age" }, { k: "games", t: "G", h8: "Games" }, ...vcols];
  return table("finder:" + find, cols, x.rows.map((r, i) => ({ ...r, _i: i })), { sort: null });
}

// Saved searches: kept in this app's settings, each with the matches seen
// at the last check; a search is checked again once a game day has passed
// since (proposed, not confirmed).
const sfRowKey = (r) => `${r.id}|${r.year ?? ""}|${r.extra && r.extra.date ? r.extra.date : ""}|${r.to ?? ""}|${r.len ?? ""}`;
function sfSavedPanel() {
  const list = prefs.statSearches || [];
  if (!list.length) return "";
  return `<section class="panel dt-panel sf-saved"><h2>Saved searches</h2><ul>${list.map((s, i) => `<li><button class="link" data-act="sfload" data-i="${i}">${esc(s.name)}</button><small class="muted">${s.total != null ? `${s.total.toLocaleString()} found` : ""}${s.checked ? ` · checked ${esc(s.checked)}` : ""}</small>${s.fresh ? `<span class="tag" title="New matches since you last opened it">${s.fresh} new</span>` : ""}<button class="tbtn" data-act="sfforget" data-i="${i}" title="Forget this search">Forget</button></li>`).join("")}</ul><p class="muted small">Each saved search is checked again once a game day has passed; new matches since you last opened it are marked.</p></section>`;
}
let sfChecking = false;
async function sfCheckSaved() {
  const list = prefs.statSearches || [];
  const today = S.status && S.status.today;
  if (sfChecking || !today || !list.some((s) => s.checked !== today)) return;
  sfChecking = true;
  try {
    for (const s of list) {
      if (s.checked === today) continue;
      let x;
      try { x = await api("stat_finder", { ...s.q, limit: 500 }); } catch (e) { continue; }
      const keys = (x.rows || []).map(sfRowKey);
      const seen = new Set(s.seen || []);
      s.fresh = (s.fresh || 0) + keys.filter((k) => !seen.has(k)).length;
      s.seen = keys;
      s.total = x.total;
      s.checked = today;
    }
    savePrefs();
    const box = document.querySelector(".sf-saved");
    if (box) box.outerHTML = sfSavedPanel();
  } finally {
    sfChecking = false;
  }
}

// ---------- finding a player or a team to add ----------
let trFindTimer = 0;
document.addEventListener("input", (ev) => {
  const t = ev.target;
  if (!t.dataset || t.dataset.act !== "trfind") return;
  clearTimeout(trFindTimer);
  const box = document.getElementById(t.dataset.into + "-hits");
  const q = t.value.trim();
  if (q.length < 2) { if (box) box.innerHTML = ""; return; }
  trFindTimer = setTimeout(async () => {
    const x = await api("compare_find", { q, kind: t.dataset.kind });
    if (!box) return;
    box.innerHTML = x.items.slice(0, 12).map((h) => `<button class="tr-hit2" data-act="trpickhit" data-into="${t.dataset.into}" data-v='${esc(JSON.stringify(h.ref || { id: h.id }))}'><b>${esc(h.name)}</b><small>${esc(h.sub || "")}</small></button>`).join("") || '<span class="muted small">No one by that name.</span>';
  }, 220);
});

Object.assign(CORE_MORE, {
  trlevel: (el) => { const K = keep("trends", {}); K.level = el.dataset.v; K.division = null; K.state = null; K.stats = null; refresh(el); },
  trall: (el) => { keep("trends", {}).all = el.dataset.v === "1"; refresh(el); },
  trpickx: (el) => { const K = keep("trends", {}); K.stats.splice(Number(el.dataset.i), 1); refresh(el); },
  trtpickx: (el) => { const K = keep("trends", {}); K.tstats.splice(Number(el.dataset.i), 1); refresh(el); },
  trcompn: (el) => { keep("trp:" + el.dataset.pid, {}).seasons = Number(el.dataset.v); refresh(el); },
  trcompm: (el) => { keep("trp:" + el.dataset.pid, {}).mode = el.dataset.v; refresh(el); },
  trpickhit: (el) => {
    const v = JSON.parse(el.dataset.v);
    const into = el.dataset.into;
    if (into === "trteam") keep("trends", {}).team = v;
    else if (into === "trplayer") keep("trends", {}).pid = v.id;
    else if (into === "cmpp") { const K = keep("dcompare", { ids: [], teams: [] }); if (!K.ids.includes(v.id)) K.ids = [...K.ids, v.id].slice(0, 4); }
    else if (into === "cmpt") { const K = keep("dcompare", { ids: [], teams: [] }); if (!K.teams.some((t) => t.kind === v.kind && t.id === v.id)) K.teams = [...K.teams, v].slice(0, 4); }
    refresh(el);
  },
  cmpx: (el) => { const K = keep("dcompare", { ids: [], teams: [] }); K.ids.splice(Number(el.dataset.i), 1); refresh(el); },
  cmptx: (el) => { const K = keep("dcompare", { ids: [], teams: [] }); K.teams.splice(Number(el.dataset.i), 1); refresh(el); },
  cmpline: (el) => { const K = keep("dcompare", { ids: [], teams: [] }); K.line = el.dataset.v; K.at = null; refresh(el); },
  sffind: (el) => { keep("statfind", { find: "seasons", conds: [] }).find = el.dataset.v; refresh(el); },
  sflevel: (el) => { const K = keep("statfind", { find: "seasons", conds: [] }); K.level = el.dataset.v; K.division = null; K.state = null; K.class = null; K.rs = null; K.from = null; K.to = null; refresh(el); },
  sfadd: (el) => { const K = keep("statfind", { find: "seasons", conds: [] }); K.conds.push({ stat: (K.pos === "QB" ? "PassYds" : K.pos === "WR" || K.pos === "TE" ? "RecYds" : "RushYds"), op: ">=", v: 1000 }); refresh(el); },
  sfdel: (el) => { const K = keep("statfind", { find: "seasons", conds: [] }); K.conds.splice(Number(el.dataset.i), 1); refresh(el); },
  sfrun: (el) => refresh(el),
  sfsave: async (el) => {
    const K = keep("statfind", { find: "seasons", conds: [] });
    const name = (await ask("Name this search", { value: sfSay(K, { level: dtLevel(K), division: K.division || "FBS", state: K.state, from: K.from ?? "", to: K.to ?? "" }).replace(/\.$/, ""), ok: "Save" })) || "";
    if (!name.trim()) return;
    const q = { ...sfQuery(K), limit: 500 };
    let x = { rows: [], total: 0 };
    try { x = await api("stat_finder", q); } catch (e) { /* saved all the same */ }
    const s = { name: name.trim(), q, keep: { ...K, conds: K.conds.map((c) => ({ ...c })) }, seen: x.rows.map(sfRowKey), total: x.total, checked: S.status && S.status.today, fresh: 0 };
    prefs.statSearches = [...(prefs.statSearches || []).filter((v) => v.name !== s.name), s].slice(-20);
    savePrefs();
    refresh(el);
  },
  sfload: (el) => {
    const s = (prefs.statSearches || [])[Number(el.dataset.i)];
    if (!s) return;
    const K = keep("statfind", { find: "seasons", conds: [] });
    for (const k of Object.keys(K)) delete K[k];
    Object.assign(K, JSON.parse(JSON.stringify(s.keep)));
    s.fresh = 0;
    savePrefs();
    refresh(el);
  },
  sfforget: (el) => { (prefs.statSearches || []).splice(Number(el.dataset.i), 1); savePrefs(); refresh(el); },
});
document.addEventListener("change", (ev) => {
  const t = ev.target;
  if (!t.dataset) return;
  const a = t.dataset.act;
  if (a === "trpick" || a === "trtpick") {
    const K = keep("trends", {});
    const k = a === "trpick" ? "stats" : "tstats";
    if (!t.value) return;
    K[k] = (K[k] || []).slice();
    K[k][Number(t.dataset.i)] = t.value;
    K[k] = [...new Set(K[k].filter(Boolean))].slice(0, 3);
    refresh(t);
  } else if (a === "trsel") { keep("trends", {})[t.dataset.k] = t.value || null; refresh(t); }
  else if (a === "trmetric") { keep("trp:" + t.dataset.pid, {}).metric = Number(t.value); refresh(t); }
  else if (a === "trcompd") { keep("trp:" + t.dataset.pid, {}).drafted = t.checked || null; refresh(t); }
  else if (a === "cmpat") { keep("dcompare", { ids: [], teams: [] }).at = Number(t.value); refresh(t); }
  else if (a === "cmptyear") { keep("dcompare", { ids: [], teams: [] }).tyear = t.value ? Number(t.value) : null; refresh(t); }
  else if (a === "sfset") { const K = keep("statfind", { find: "seasons", conds: [] }); K[t.dataset.k] = t.value || null; refresh(t); }
  else if (a === "sfcond") { const K = keep("statfind", { find: "seasons", conds: [] }); K.conds[Number(t.dataset.i)][t.dataset.k] = t.value; refresh(t); }
  else if (a === "sfcondv") { const K = keep("statfind", { find: "seasons", conds: [] }); K.conds[Number(t.dataset.i)].v = t.value; }
  else if (a === "sfminlen") { keep("statfind", { find: "seasons", conds: [] }).min_len = Number(t.value) || 3; }
});
// Enter in a condition's value runs the search.
document.addEventListener("keydown", (ev) => {
  const t = ev.target;
  if (ev.key !== "Enter" || !t.dataset || (t.dataset.act !== "sfcondv" && t.dataset.act !== "sfminlen")) return;
  const K = keep("statfind", { find: "seasons", conds: [] });
  if (t.dataset.act === "sfcondv") K.conds[Number(t.dataset.i)].v = t.value;
  else K.min_len = Number(t.value) || 3;
  refresh(t);
});
