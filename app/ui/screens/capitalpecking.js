/* Capital: the Pecking order (lane CS5; docs/design/capital.md CP29-CP31,
 * CP37, CP39, board 5). Where the press puts your conference's facilities
 * each July: every program ranked with its move since last July, its
 * newest building and the drive you know of (public, building, a rumor, or
 * none public), your own rank over the years, and what your donors hear of
 * rivals' quiet phases (CP30). The press's score and every dollar stay in
 * the engine; the page reads in ranks, shares and words.
 *
 *   capitalpecking   no arguments.
 *
 * It reads `capital_pecking` (lane_capital_pecking.rs) and changes nothing.
 * Every class is cpk- so the app's global names never restyle it. */
"use strict";

I.cappecking = '<path d="M5 20.5V12M12 20.5V4.5M19 20.5V9"/><path d="M3.5 20.5h17"/>';
navAdd({ go: "capitalpecking", label: "Pecking order", icon: "cappecking", sec: "capital", seats: ["college"] });
CRUMBS.capitalpecking = "Pecking order";

const cpkOrd = (n) => `${n}${["th", "st", "nd", "rd"][(n % 100 >= 11 && n % 100 <= 13) || n % 10 > 3 ? 0 : n % 10]}`;

screens.capitalpecking = async () => {
  let x;
  try { x = await api("capital_pecking", {}); } catch (e) {
    title("Pecking order", "");
    return `<div class="pad sl cpk"><section class="cpk-card"><div class="cpk-k">Capital</div><p class="cpk-empty">${esc(e.message || String(e))}</p></section></div>`;
  }
  title("Pecking order", x.program);
  const head = `<div class="cpk-top"><div class="cpk-t"><div class="cpk-k">Capital · ${esc(x.program)}</div><h1>Pecking order</h1></div>${capTabs("capitalpecking")}<small class="cpk-dim">${esc(capDate(x.today, { weekday: "short", month: "long", day: "numeric", year: "numeric" }))}</small></div>`;
  const p = x.pecking;
  if (!p) {
    return `<div class="pad sl cpk">${head}<section class="cpk-card"><div class="cpk-k">Nothing ranked yet</div><p class="cpk-empty">The press ranks every conference's facilities each July, and recruiting sites quote it. The first order comes out this July.</p></section>${cpkRumors(x)}</div>`;
  }
  return `<div class="pad sl cpk">${head}<div class="cpk-cols"><section class="cpk-card cpk-table">${cpkTable(p)}</section><div class="cpk-side">${cpkYears(x)}${cpkRumors(x)}</div></div></div>`;
};

function cpkMove(m) {
  if (m == null) return '<span class="mv flat" title="Not ranked in this conference last July">new</span>';
  if (m === 0) return '<span class="mv flat" aria-label="No change">=</span>';
  return `<span class="mv ${m > 0 ? "up" : "down"}" aria-label="${m > 0 ? "Up" : "Down"} ${Math.abs(m)}">${m > 0 ? "+" : "-"}${Math.abs(m)}</span>`;
}

function cpkTable(p) {
  const head = '<div class="cpk-tr cpk-hd"><span>#</span><span>School</span><span>Move</span><span>Newest building</span><span>Drive</span></div>';
  const rows = p.rows.map((t) => {
    const d = t.drive;
    return `<div class="cpk-tr${t.me ? " me" : ""}"><span class="n">${t.n}</span><span class="sc"><b>${esc(t.program)}${t.me ? " (you)" : ""}</b><small>${esc(capCap(t.because))}</small></span>${cpkMove(t.move)}<span class="nb">${t.newest ? esc(t.newest) : '<span class="cpk-dim">None yet</span>'}</span><span class="dr ${d.kind}">${esc(d.words)}</span></div>`;
  }).join("");
  return `<div class="cpk-th"><div class="cpk-h">${esc(p.conference)} facilities, July ${p.year}</div><span class="cpk-dim">The press ranks every July. Recruiting sites quote it.</span></div>${head}${rows}`;
}

// Your rank over the years: a line on a 1-to-N axis, the last July in gold.
function cpkYears(x) {
  const ys = x.years;
  const p = x.pecking;
  const say = p.rank == null ? "" : p.before == null
    ? `${cpkOrd(p.rank)} of ${p.of} this July.`
    : p.rank === p.before ? `${cpkOrd(p.rank)} of ${p.of}, as last July.` : `${cpkOrd(p.rank)} of ${p.of} this July, ${p.rank < p.before ? "up from" : "down from"} ${cpkOrd(p.before)}.`;
  const why = p.why ? ` The press goes by ${esc(p.why)}.` : "";
  if (ys.length < 2) {
    return `<section class="cpk-card"><div class="cpk-k">Your rank over the years</div><p>${esc(say)}${why}</p><p class="cpk-dim sm">A line will draw once the press has ranked you in two Julys.</p></section>`;
  }
  const W = 340, H = 150, L = 22, R = 14, T = 20, B = 130;
  const top = Math.max(...ys.map((y) => y.of));
  const x0 = ys[0].year, x1 = ys[ys.length - 1].year;
  const px = (y) => L + ((y - x0) / Math.max(x1 - x0, 1)) * (W - L - R);
  const py = (r) => T + ((r - 1) / Math.max(top - 1, 1)) * (B - T);
  const pts = ys.map((y) => `${px(y.year).toFixed(1)},${py(y.rank).toFixed(1)}`);
  const last = ys[ys.length - 1];
  const alt = ys.map((y) => `${y.year}: ${cpkOrd(y.rank)}`).join(", ");
  return `<section class="cpk-card"><div class="cpk-k">Your rank over the years</div>
    <svg class="cpk-ch" role="img" aria-label="Your facilities rank by July, best at the top. ${esc(alt)}" viewBox="0 0 ${W} ${H}">
      <path d="M${L} ${T}H${W - R}M${L} ${(T + B) / 2}H${W - R}M${L} ${B}H${W - R}" class="gl"/>
      <polyline points="${pts.join(" ")}" class="ln"/>
      <circle cx="${px(last.year).toFixed(1)}" cy="${py(last.rank).toFixed(1)}" r="5" class="dot"/>
      <text x="0" y="${T + 4}" class="tx">1</text><text x="0" y="${B + 4}" class="tx">${top}</text>
      <text x="${L}" y="${H - 4}" class="tx">${x0}</text><text x="${W - R}" y="${H - 4}" class="tx" text-anchor="end">${x1}</text>
    </svg><p>${esc(say)}${why}</p></section>`;
}

function cpkRumors(x) {
  const r = x.rumors;
  const body = r.length
    ? r.map((m) => `<div class="cpk-rum"><b>${esc(m.title)}</b><span>${esc(m.source)}. ${esc(m.detail)}</span></div>`).join("")
    : '<p class="cpk-dim">Nothing yet. A rival\'s study and quiet phase are never in the news: you hear of them only from your own donors, and the warmer they are to you, the more they tell.</p>';
  return `<section class="cpk-card"><div class="cpk-k">What your donors hear</div>${body}</section>`;
}
