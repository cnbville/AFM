// AFM World Reader: the area panels (lane R6) and the views they need: a
// histogram against real, world-against-real bars, a year line with the
// real band shaded, a heat grid, a class funnel, a state map and a dated
// timeline. The book works each panel out when it is built (`panels`
// table); this file draws one from its columns. A `grp` column becomes a
// row of buttons to pick the group; a `year` column follows the year slider.
"use strict";

const C_WORLD = "#f0607c", C_REAL = "#b1b6c0", C_FAINT = "rgba(255,255,255,.08)";

// The US as a tile grid: [state, column, row].
const US_TILES = [
  ["AK", 0, 0], ["ME", 11, 0], ["VT", 10, 1], ["NH", 11, 1], ["WA", 1, 2], ["ID", 2, 2], ["MT", 3, 2], ["ND", 4, 2], ["MN", 5, 2],
  ["IL", 6, 2], ["WI", 7, 2], ["MI", 8, 2], ["NY", 9, 2], ["RI", 10, 2], ["MA", 11, 2], ["OR", 1, 3], ["NV", 2, 3], ["WY", 3, 3],
  ["SD", 4, 3], ["IA", 5, 3], ["IN", 6, 3], ["OH", 7, 3], ["PA", 8, 3], ["NJ", 9, 3], ["CT", 10, 3], ["CA", 1, 4], ["UT", 2, 4],
  ["CO", 3, 4], ["NE", 4, 4], ["MO", 5, 4], ["KY", 6, 4], ["WV", 7, 4], ["VA", 8, 4], ["MD", 9, 4], ["DE", 10, 4], ["AZ", 2, 5],
  ["NM", 3, 5], ["KS", 4, 5], ["AR", 5, 5], ["TN", 6, 5], ["NC", 7, 5], ["SC", 8, 5], ["DC", 9, 5], ["OK", 4, 6], ["LA", 5, 6],
  ["MS", 6, 6], ["AL", 7, 6], ["GA", 8, 6], ["HI", 0, 7], ["TX", 4, 7], ["FL", 9, 7],
];

const colIx = (b, name) => (b.cols || []).indexOf(name);
const cellOf = (b, r, name) => { const i = colIx(b, name); return i < 0 ? undefined : r[i]; };
const numOf = (v) => (typeof v === "number" ? v : v == null || v === "" ? NaN : Number(v));

// A row's page from the panel's `open` template: `e/people/{id}`.
function openOf(b, r) {
  if (!b.open) return "";
  let ok = true;
  const link = b.open.replace(/\{(\w+)\}/g, (_, c) => { const v = cellOf(b, r, c); if (v == null || v === "") ok = false; return encodeURIComponent(v); });
  return ok ? link : "";
}

// The rows on screen: the picked group and the slider's year.
function panelRows(b) {
  let rows = b.rows || [];
  const gi = colIx(b, "grp");
  if (gi >= 0) {
    const groups = [...new Set(rows.map((r) => r[gi]))];
    if (b.g == null || !groups.includes(b.g)) b.g = groups[0];
    rows = rows.filter((r) => r[gi] === b.g);
  }
  const yi = colIx(b, "year");
  if (yi >= 0 && b.pkind !== "line" && b.pkind !== "table") {
    const years = [...new Set(rows.map((r) => r[yi]))].filter((y) => y != null).sort((a, z) => a - z);
    const want = typeof F !== "undefined" && F.year ? +F.year : null;
    b.shownYear = want != null && years.includes(want) ? want : years[years.length - 1];
    if (b.shownYear != null) rows = rows.filter((r) => r[yi] === b.shownYear);
  }
  return rows;
}

function groupBar(b, id) {
  const gi = colIx(b, "grp");
  if (gi < 0) return "";
  const groups = [...new Set((b.rows || []).map((r) => r[gi]))];
  if (groups.length < 2) return "";
  return `<div class="seg" role="group" aria-label="Group">${groups.map((g) => `<button type="button" class="${g === b.g ? "on" : ""}" aria-pressed="${g === b.g}" data-grp="${id}" data-g="${esc(g)}">${esc(g)}</button>`).join("")}</div>`;
}

const pctText = (v, unit) => (v == null || Number.isNaN(v) ? "–" : unit === "%" ? `${fmt(v)}%` : unit ? `${fmt(v)} ${unit}` : fmt(v));

// World against real per row, as two bars each.
function barsHtml(b, rows) {
  const li = colIx(b, "label"), wi = colIx(b, "world"), ri = colIx(b, "real"), ni = colIx(b, "n");
  const extra = (b.cols || []).map((c, i) => i).filter((i) => ![li, wi, ri, ni, colIx(b, "grp"), colIx(b, "year")].includes(i) && !/^_/.test(b.cols[i]));
  const max = Math.max(1e-9, ...rows.flatMap((r) => [numOf(r[wi]), ri >= 0 ? numOf(r[ri]) : 0].filter((x) => !Number.isNaN(x))));
  const bar = (v, cls) => Number.isNaN(numOf(v)) ? "" : `<div class="bar ${cls}"><span style="width:${Math.max(0, Math.min(100, (numOf(v) / max) * 100)).toFixed(1)}%"></span></div>`;
  const head = `<th>${esc(b.label || "")}</th><th class="n">World</th><th></th>${ri >= 0 ? `<th class="n">Real</th><th></th><th>Mark</th>` : ""}${ni >= 0 ? `<th class="n">n</th>` : ""}${extra.map((i) => `<th>${esc(label(b.cols[i]))}</th>`).join("")}`;
  const body = rows.map((r) => {
    const link = openOf(b, r);
    const w = numOf(r[wi]), re = ri >= 0 ? numOf(r[ri]) : NaN;
    const m = ri >= 0 && !Number.isNaN(re) && !Number.isNaN(w) ? markOf(w, re) : "";
    return `<tr class="${link ? "click" : ""}" ${link ? `data-go="${esc(link)}"` : ""}><td><b>${esc(r[li])}</b></td><td class="n">${esc(pctText(w, b.unit))}</td><td>${bar(w, "")}</td>` +
      (ri >= 0 ? `<td class="n" style="color:var(--sub)">${esc(pctText(re, b.unit))}</td><td>${bar(re, "r")}</td><td>${mark(m)}</td>` : "") +
      (ni >= 0 ? `<td class="n">${esc(fmt(r[ni]))}</td>` : "") +
      extra.map((i) => `<td>${esc(fmt(r[i]))}</td>`).join("") + "</tr>";
  }).join("");
  return `<div class="tbl"><table class="bars"><thead><tr>${head}</tr></thead><tbody>${body}</tbody></table></div>`;
}

// Near within 5% of real (or 0.5 points on a share), off within 15%.
function markOf(w, re) {
  if (re === 0 && w === 0) return "near";
  const gap = Math.abs(w - re) / Math.max(Math.abs(re), 1e-9);
  const pts = Math.abs(w - re);
  if (gap <= 0.05 || pts <= 0.5) return "near";
  if (gap <= 0.15 || pts <= 1.5) return "off";
  return "far";
}

// Shared SVG frame: x and y scales, grid and labels.
function frame(xs, ys, opt) {
  const W = 1000, Hh = opt.h || 260, L = 52, R = 14, T = 14, B = 34;
  const x0 = Math.min(...xs), x1 = Math.max(...xs);
  let y0 = opt.zero ? Math.min(0, ...ys) : Math.min(...ys), y1 = Math.max(...ys);
  if (y1 === y0) y1 = y0 + 1;
  const pad = (y1 - y0) * 0.06;
  if (!opt.zero) y0 -= pad;
  y1 += pad;
  const X = (x) => L + ((x - x0) / Math.max(1e-9, x1 - x0)) * (W - L - R);
  const Y = (y) => T + (1 - (y - y0) / (y1 - y0)) * (Hh - T - B);
  const step = nice((y1 - y0) / 5);
  let g = "";
  for (let y = Math.ceil(y0 / step) * step; y <= y1; y += step) g += `<line class="ax" x1="${L}" x2="${W - R}" y1="${Y(y)}" y2="${Y(y)}"/><text x="${L - 8}" y="${Y(y) + 4}" text-anchor="end">${fmt(+y.toFixed(6))}</text>`;
  return { W, Hh, L, R, T, B, X, Y, g, x0, x1, y0, y1 };
}
function nice(x) {
  const e = Math.pow(10, Math.floor(Math.log10(Math.max(x, 1e-9))));
  const f = x / e;
  return (f < 1.5 ? 1 : f < 3 ? 2 : f < 7 ? 5 : 10) * e;
}

// A histogram: world bars, the real spread as a grey step line over them.
function histHtml(b, rows) {
  const xi = colIx(b, "x"), wi = colIx(b, "world"), ri = colIx(b, "real");
  const pts = rows.filter((r) => !Number.isNaN(numOf(r[wi])));
  if (pts.length < 2) return `<p class="sub">Too few values for a histogram.</p>`;
  const numeric = pts.every((r) => !Number.isNaN(numOf(r[xi])));
  const n = pts.length;
  const ys = pts.flatMap((r) => [numOf(r[wi]), ri >= 0 ? numOf(r[ri]) : 0]).filter((y) => !Number.isNaN(y));
  const fr = frame([0, n], ys, { zero: true, h: 240 });
  const bw = (fr.W - fr.L - fr.R) / n;
  let g = fr.g;
  pts.forEach((r, i) => {
    const w = numOf(r[wi]);
    const x = fr.L + i * bw;
    g += `<rect x="${(x + 1).toFixed(1)}" y="${fr.Y(w).toFixed(1)}" width="${Math.max(1, bw - 2).toFixed(1)}" height="${(fr.Y(0) - fr.Y(w)).toFixed(1)}" fill="${C_WORLD}" opacity=".85"><title>${esc(r[xi])}: ${esc(pctText(w, b.unit))}${ri >= 0 ? ` · real ${esc(pctText(numOf(r[ri]), b.unit))}` : ""}</title></rect>`;
  });
  if (ri >= 0) {
    let d = "";
    pts.forEach((r, i) => {
      const y = numOf(r[ri]);
      if (Number.isNaN(y)) return;
      const x = fr.L + i * bw;
      d += `${d ? "L" : "M"}${x.toFixed(1)},${fr.Y(y).toFixed(1)} L${(x + bw).toFixed(1)},${fr.Y(y).toFixed(1)} `;
    });
    g += `<path d="${d}" fill="none" stroke="${C_REAL}" stroke-width="2.5" stroke-dasharray="6 4"/>`;
  }
  const every = Math.max(1, Math.ceil(n / 14));
  pts.forEach((r, i) => { if (i % every === 0) g += `<text x="${(fr.L + (i + 0.5) * bw).toFixed(1)}" y="${fr.Hh - 10}" text-anchor="middle">${esc(numeric ? fmt(numOf(r[xi])) : r[xi])}</text>`; });
  const legend = `<span class="key"><i style="background:${C_WORLD}"></i>World</span>${ri >= 0 ? `<span class="key"><i style="background:${C_REAL}"></i>Real</span>` : ""}`;
  return `<div class="legend">${legend}</div><div class="chart"><svg viewBox="0 0 ${fr.W} ${fr.Hh}" role="img" aria-label="${esc(b.title)}">${g}</svg></div>`;
}

// Year lines; the real band shaded and the real line dashed.
function lineHtml(b, rows) {
  const yi = colIx(b, "year");
  const skip = new Set(["year", "grp", "real_lo", "real_hi", "real", "n"]);
  const series = (b.cols || []).map((c, i) => ({ c, i })).filter((s) => !skip.has(s.c) && !/^_/.test(s.c) && rows.some((r) => !Number.isNaN(numOf(r[s.i]))));
  const lo = colIx(b, "real_lo"), hi = colIx(b, "real_hi"), re = colIx(b, "real");
  const pts = rows.filter((r) => !Number.isNaN(numOf(r[yi]))).sort((a, z) => a[yi] - z[yi]);
  if (pts.length < 2) return `<p class="sub">Too few years for a line.</p>`;
  const xs = pts.map((r) => numOf(r[yi]));
  const ys = pts.flatMap((r) => [...series.map((s) => numOf(r[s.i])), lo >= 0 ? numOf(r[lo]) : NaN, hi >= 0 ? numOf(r[hi]) : NaN, re >= 0 ? numOf(r[re]) : NaN]).filter((y) => !Number.isNaN(y));
  const fr = frame(xs, ys, { zero: !!b.zero, h: 260 });
  let g = fr.g;
  if (lo >= 0 && hi >= 0) {
    const band = pts.filter((r) => !Number.isNaN(numOf(r[lo])) && !Number.isNaN(numOf(r[hi])));
    if (band.length) {
      const top = band.map((r) => `${fr.X(numOf(r[yi])).toFixed(1)},${fr.Y(numOf(r[hi])).toFixed(1)}`);
      const bot = band.slice().reverse().map((r) => `${fr.X(numOf(r[yi])).toFixed(1)},${fr.Y(numOf(r[lo])).toFixed(1)}`);
      const one = band.length === 1 ? ` ${(fr.X(numOf(band[0][yi])) + 6).toFixed(1)},${fr.Y(numOf(band[0][hi])).toFixed(1)}` : "";
      g += `<polygon points="${top.join(" ")}${one} ${bot.join(" ")}" fill="rgba(177,182,192,.16)" stroke="none"/>`;
    }
  }
  if (re >= 0) {
    const p = pts.filter((r) => !Number.isNaN(numOf(r[re]))).map((r) => `${fr.X(numOf(r[yi])).toFixed(1)},${fr.Y(numOf(r[re])).toFixed(1)}`);
    if (p.length > 1) g += `<polyline fill="none" stroke="${C_REAL}" stroke-width="2" stroke-dasharray="6 5" points="${p.join(" ")}"/>`;
  }
  const colors = [C_WORLD, "#7fb2ff", "#ffd60a", "#30d158", "#bf5af2", "#ff9f0a"];
  series.forEach((s, k) => {
    const p = pts.filter((r) => !Number.isNaN(numOf(r[s.i]))).map((r) => `${fr.X(numOf(r[yi])).toFixed(1)},${fr.Y(numOf(r[s.i])).toFixed(1)}`);
    g += `<polyline fill="none" stroke="${colors[k % colors.length]}" stroke-width="${k ? 2 : 3}" points="${p.join(" ")}"/>`;
    pts.forEach((r) => { const v = numOf(r[s.i]); if (!Number.isNaN(v)) g += `<circle cx="${fr.X(numOf(r[yi])).toFixed(1)}" cy="${fr.Y(v).toFixed(1)}" r="${k ? 2.5 : 3.5}" fill="${colors[k % colors.length]}"><title>${r[yi]} · ${esc(label(s.c))}: ${esc(pctText(v, b.unit))}</title></circle>`; });
  });
  const step = Math.max(1, Math.round((fr.x1 - fr.x0) / 12));
  for (let x = fr.x0; x <= fr.x1; x += step) g += `<text x="${fr.X(x)}" y="${fr.Hh - 10}" text-anchor="middle">${x}</text>`;
  if (typeof F !== "undefined" && F.year && F.year >= fr.x0 && F.year <= fr.x1) g += `<line x1="${fr.X(F.year)}" x2="${fr.X(F.year)}" y1="${fr.T}" y2="${fr.Hh - fr.B}" stroke="#ffd60a" stroke-dasharray="3 4"/>`;
  const legend = series.map((s, k) => `<span class="key"><i style="background:${colors[k % colors.length]}"></i>${esc(label(s.c))}</span>`).join("") +
    (lo >= 0 ? `<span class="key"><i style="background:rgba(177,182,192,.4)"></i>Real range</span>` : "") + (re >= 0 ? `<span class="key"><i style="background:${C_REAL}"></i>Real</span>` : "");
  return `<div class="legend">${legend}</div><div class="chart"><svg viewBox="0 0 ${fr.W} ${fr.Hh}" role="img" aria-label="${esc(b.title)}">${g}</svg></div>`;
}

// A heat grid: rows by columns, each cell shaded by its value.
function matrixHtml(b, rows) {
  const ri = colIx(b, "row"), ci = colIx(b, "col"), vi = colIx(b, "value"), reali = colIx(b, "real");
  const rs = [...new Set(rows.map((r) => r[ri]))];
  const cs = [...new Set(rows.map((r) => r[ci]))];
  const at = new Map(rows.map((r) => [r[ri] + "\u0001" + r[ci], r]));
  const max = Math.max(1e-9, ...rows.map((r) => Math.abs(numOf(r[vi]))).filter((x) => !Number.isNaN(x)));
  const head = `<th>${esc(b.rowLabel || "")}</th>` + cs.map((c) => `<th class="n">${esc(c)}</th>`).join("");
  const body = rs.map((rv) => `<tr><td><b>${esc(rv)}</b></td>` + cs.map((cv) => {
    const r = at.get(rv + "\u0001" + cv);
    if (!r) return `<td class="n" style="color:var(--faint)">–</td>`;
    const v = numOf(r[vi]);
    const a = Number.isNaN(v) ? 0 : Math.min(1, Math.abs(v) / max);
    const real = reali >= 0 && r[reali] != null ? `<span class="sub"> · ${esc(pctText(numOf(r[reali]), b.unit))}</span>` : "";
    const link = openOf(b, r);
    return `<td class="n heat ${link ? "click" : ""}" ${link ? `data-go="${esc(link)}"` : ""} style="background:rgba(240,96,124,${(0.06 + a * 0.55).toFixed(2)})" title="${esc(rv)} · ${esc(cv)}">${esc(pctText(v, b.unit))}${real}</td>`;
  }).join("") + "</tr>").join("");
  return `<div class="tbl"><table class="matrix"><thead><tr>${head}</tr></thead><tbody>${body}</tbody></table></div>`;
}

// A class funnel: each stage's bar against the start, the real share as a tick.
function funnelHtml(b, rows) {
  const si = colIx(b, "stage"), ni = colIx(b, "n"), shi = colIx(b, "share"), rei = colIx(b, "real");
  const n0 = Math.max(1, numOf(rows[0] && rows[0][ni]) || 1);
  const body = rows.map((r, i) => {
    const n = numOf(r[ni]);
    const share = shi >= 0 && !Number.isNaN(numOf(r[shi])) ? numOf(r[shi]) : (n / n0) * 100;
    const w = Math.max(0.4, Math.min(100, Math.sqrt(Math.max(0, n / n0)) * 100));
    const real = rei >= 0 ? numOf(r[rei]) : NaN;
    const tick = Number.isNaN(real) ? "" : `<i class="tick" style="left:${Math.min(100, Math.sqrt(real / 100) * 100).toFixed(1)}%" title="Real ${fmt(real)}%"></i>`;
    const prev = i ? numOf(rows[i - 1][ni]) : NaN;
    const kept = i && prev > 0 ? ` · ${fmt((n / prev) * 100)}% of the stage above` : "";
    const link = openOf(b, r);
    return `<div class="fstage ${link ? "click" : ""}" ${link ? `data-go="${esc(link)}"` : ""}><div class="fl"><b>${esc(r[si])}</b><span class="sub">${fmt(n)} · ${fmt(share)}% of the class${kept}${Number.isNaN(real) ? "" : ` · real ${fmt(real)}%`}</span></div><div class="fb"><span style="width:${w.toFixed(1)}%"></span>${tick}</div></div>`;
  }).join("");
  return `<div class="funnel">${body}</div><p class="sub" style="margin:6px 0 0">Bar widths are on a square-root scale so the small stages show; the grey tick is the real share.</p>`;
}

// The states as tiles, each shaded by the world's share against real.
function mapHtml(b, rows) {
  const si = colIx(b, "state"), wi = colIx(b, "world"), ri = colIx(b, "real"), ni = colIx(b, "n");
  const by = new Map(rows.map((r) => [String(r[si]).toUpperCase(), r]));
  const max = Math.max(1e-9, ...rows.map((r) => numOf(r[wi])).filter((x) => !Number.isNaN(x)));
  const tile = 76, gap = 6;
  let g = "";
  US_TILES.forEach(([st, c, rr]) => {
    const r = by.get(st);
    const x = c * (tile + gap), y = rr * (tile + gap);
    let fill = "rgba(255,255,255,.04)", line2 = "–", tip = st + ": no players";
    if (r) {
      const w = numOf(r[wi]);
      const re = ri >= 0 ? numOf(r[ri]) : NaN;
      if (!Number.isNaN(re) && re > 0) {
        // Over real: orange; under: blue; the stronger, the further off.
        const ratio = Math.log2(Math.max(w, 1e-6) / re);
        const a = Math.min(1, Math.abs(ratio) / 1.5);
        fill = ratio >= 0 ? `rgba(255,159,10,${(0.12 + 0.7 * a).toFixed(2)})` : `rgba(10,132,255,${(0.12 + 0.7 * a).toFixed(2)})`;
      } else fill = `rgba(240,96,124,${(0.1 + 0.75 * Math.min(1, w / max)).toFixed(2)})`;
      line2 = pctText(w, b.unit);
      tip = `${st}: world ${pctText(w, b.unit)}${Number.isNaN(re) ? "" : ` · real ${pctText(re, b.unit)}`}${ni >= 0 ? ` · ${fmt(r[ni])} players` : ""}`;
    }
    const link = r ? openOf(b, r) : b.open && b.open.includes("{state}") ? b.open.replace("{state}", st) : "";
    g += `<g ${link ? `class="click" data-go="${esc(link)}"` : ""}><rect x="${x}" y="${y}" width="${tile}" height="${tile}" rx="10" fill="${fill}" stroke="rgba(255,255,255,.08)"><title>${esc(tip)}</title></rect>` +
      `<text x="${x + 10}" y="${y + 24}" font-size="15" font-weight="800" fill="#f4f5f7">${st}</text><text x="${x + 10}" y="${y + 58}" font-size="15" fill="#d6d9df">${esc(line2)}</text></g>`;
  });
  const W = 12 * (tile + gap), Hh = 8 * (tile + gap);
  const legend = ri >= 0
    ? `<span class="key"><i style="background:rgba(255,159,10,.75)"></i>Above real</span><span class="key"><i style="background:rgba(10,132,255,.75)"></i>Below real</span><span class="key"><i style="background:rgba(255,255,255,.15)"></i>Near: pale</span>`
    : `<span class="key"><i style="background:${C_WORLD}"></i>Darker: more</span>`;
  return `<div class="legend">${legend}</div><div class="chart map"><svg viewBox="0 0 ${W} ${Hh}" role="img" aria-label="${esc(b.title)}">${g}</svg></div>`;
}

function panelTableHtml(b, rows) {
  const hide = new Set(["grp", "_link"]);
  const cols = (b.cols || []).map((c, i) => ({ c, i })).filter((x) => !hide.has(x.c));
  const head = cols.map(({ c }) => `<th class="${/^(n|world|real|share|pct|value)$|_n$|^n_/.test(c) ? "n" : ""}">${esc(label(c))}</th>`).join("");
  const body = rows.map((r) => {
    const link = openOf(b, r);
    return `<tr class="${link ? "click" : ""}" ${link ? `data-go="${esc(link)}"` : ""}>` + cols.map(({ c, i }) => {
      const v = r[i];
      const yearish = /(^|_)(year|season|class)$/.test(c) && typeof v === "number";
      return `<td class="${typeof v === "number" ? "n" : ""}">${esc(yearish ? String(v) : fmt(v))}</td>`;
    }).join("") + "</tr>";
  }).join("");
  return `<div class="tbl" style="max-height:520px;overflow:auto"><table><thead><tr>${head}</tr></thead><tbody>${body || `<tr><td class="sub">No rows.</td></tr>`}</tbody></table></div>`;
}

function label(c) {
  const s = String(c).replace(/^x_/, "").replace(/_/g, " ");
  return s.charAt(0).toUpperCase() + s.slice(1);
}

function panelHtml(b, id) {
  const rows = panelRows(b);
  const yearNote = b.shownYear != null ? ` · ${b.shownYear}` : "";
  const head = `<div class="head"><h2>${esc(b.title)}${yearNote}</h2>${b.code ? `<span class="mk code">${esc(b.code)}</span>` : ""}</div>`;
  let body;
  if (b.status && b.status !== "ok" && !(b.rows || []).length) body = `<p class="sub">${esc(b.status === "no rows" ? "No rows in this book." : b.status)}</p>`;
  else if (!rows.length) body = `<p class="sub">No rows under these filters.</p>`;
  else body = ({ bars: barsHtml, hist: histHtml, line: lineHtml, matrix: matrixHtml, funnel: funnelHtml, map: mapHtml })[b.pkind]?.(b, rows) || panelTableHtml(b, rows);
  const says = b.says ? `<p class="says">${esc(b.says)}</p>` : "";
  const more = b.more ? `<p class="sub" style="margin:0">${fmt(b.more)} more rows than shown.</p>` : "";
  const note = b.note ? `<p class="sub" style="margin:0" data-h="${hv({ what: b.title, how: b.note, sql: b.sql, needs: b.needs })}">${esc(b.note)}</p>` : "";
  return `<section class="panel sec pnl" id="blk-${id}">${head}${says}${groupBar(b, id)}${body}${more}${note}</section>`;
}

// An area's header: code, name, its coverage and its verdict in words.
function areaHtml(b) {
  const st = b.status ? `<span class="mk ${b.status === "checked" ? "near" : b.status === "not stored" ? "far" : "off"}">${esc(b.status)}</span>` : "";
  const m = b.marks || {};
  const chips = [m.far ? `<span class="mk far">${m.far} far off</span>` : "", m.off ? `<span class="mk off">${m.off} off</span>` : "", m.near ? `<span class="mk near">${m.near} near</span>` : ""].join("");
  const shows = b.shows ? `<p class="sub" style="margin:0;max-width:110ch">${esc(b.shows)}</p>` : "";
  const missing = (b.missing || []).length ? `<details class="sub"><summary>Not stored yet (${b.missing.length})</summary><ul>${b.missing.map((x) => `<li>${esc(x)}</li>`).join("")}</ul></details>` : "";
  return `<section class="area" id="area-${esc(b.code)}"><div class="head"><span class="mk code">${esc(b.code)}</span><h2>${b.link ? `<a href="#/${esc(b.link)}">${esc(b.title)}</a>` : esc(b.title)}</h2>${st}${chips}</div><p class="verdict">${esc(b.verdict || "")}</p>${shows}${missing}</section>`;
}

// A dated timeline: one card per line, grouped by year.
function timelineHtml(b) {
  const items = b.items || [];
  if (!items.length) return `<p class="sub">${esc(b.empty || "Nothing dated yet.")}</p>`;
  return `<div class="tline">${items.map((i) => `<div class="tl-it"><b>${esc(i.when)}</b><p>${i.link ? `<a href="#/${esc(i.link)}">${esc(i.text)}</a>` : esc(i.text)}${i.why ? `<span class="why">${esc(i.why)}</span>` : ""}</p>${i.mark ? mark(i.mark) : ""}</div>`).join("")}</div>`;
}

// Named meters: attributes, tendencies or traits, with an optional ghost.
function metersHtml(b) {
  return `<div class="meters">${(b.groups || []).map((g) => `<div class="mg"><span class="lbl">${esc(g.name)}</span>${g.items.map((m) => {
    const v = numOf(m.v), max = m.max || 99;
    const ghost = m.ghost != null ? `<i class="ghost" style="left:${Math.min(100, (numOf(m.ghost) / max) * 100).toFixed(1)}%" title="${esc(m.ghost_label || "")} ${fmt(m.ghost)}"></i>` : "";
    return `<div class="at" ${m.h ? `data-h="${hv(m.h)}"` : ""}><span>${esc(m.k)}${m.debug ? ' <span class="mk dbg">debug</span>' : ""}</span><div class="meter"><span style="width:${Math.max(0, Math.min(100, (v / max) * 100)).toFixed(1)}%"></span>${ghost}</div><b>${esc(fmt(v))}</b>${m.sub ? `<em>${esc(m.sub)}</em>` : ""}</div>`;
  }).join("")}</div>`).join("")}</div>`;
}

window.extraBlock = function (b, id) {
  const title = b.title ? `<div class="head"><h2>${esc(b.title)}</h2>${b.note ? `<span class="sub">${esc(b.note)}</span>` : ""}</div>` : "";
  switch (b.kind) {
    case "panel": return panelHtml(b, id);
    case "area": return areaHtml(b);
    case "timeline": return `<section class="panel sec">${title}${timelineHtml(b)}</section>`;
    case "meters": return `<section class="panel sec">${title}${metersHtml(b)}</section>`;
    default: return "";
  }
};

document.addEventListener("click", (e) => {
  const t = e.target.closest("[data-grp]");
  if (!t) return;
  const b = PAGE.blocks[t.dataset.grp];
  if (!b) return;
  b.g = t.dataset.g;
  // A number-looking group compares as a number.
  const gi = colIx(b, "grp");
  const sample = (b.rows || []).find((r) => String(r[gi]) === t.dataset.g);
  if (sample) b.g = sample[gi];
  redrawBlock(t.dataset.grp);
});
