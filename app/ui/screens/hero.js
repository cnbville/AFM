// Lane 16 (update 3, HH1-HH5): the Home hero's three big slides.
// - A, under the lights (Broadcast): your stadium at night from a high
//   end-zone camera, seats filled with the real crowd share, the captains at
//   midfield, the road from here and what's asked of you.
// - B, the program's story (Press Box): wins in every season by head coach,
//   a dashed bar for this season, what you could be the first to do, and the
//   names on the wall.
// - C, the room: last season's starters in a team photo with an empty spot
//   for you, and the room's facts.
// The game builds the list (dashboard().slides, A first); core.js's
// screens.home puts each big slide's frame in, and heroDraw() fills the
// scenes once the carousel has its width, again on every resize.
"use strict";

const HERO = {
  data: [],
  kinds: new Set(["stadium", "story", "room"]),
};

// ---------- small helpers ----------
const hf1 = (v) => v.toFixed(1);
const hfmt = (n) => Number(n).toLocaleString("en-US");
function hshade(hex, k) {
  const m = /^#?([0-9a-f]{6})$/i.exec(String(hex || "").trim());
  if (!m) return "#3a3f48";
  const n = parseInt(m[1], 16);
  const ch = (s) => Math.round(((n >> s) & 255) * (1 - k));
  return `#${[16, 8, 0].map((s) => ch(s).toString(16).padStart(2, "0")).join("")}`;
}
function hface(p, cls, color) {
  const ini = initials(p.name || p.full || "");
  return `<span class="${cls} hr-face" data-face="${p.id}" style="--fc:${color}"><b>${esc(ini)}</b></span>`;
}
const heroPlayer = (p, inner, cls = "", style = "") => (p.id != null ? `<button class="${cls}" data-go="player" data-id="${p.id}"${style ? ` style="${style}"` : ""}>${inner}</button>` : `<span class="${cls}"${style ? ` style="${style}"` : ""}>${inner}</span>`);

// ---------- the frame each big slide starts with ----------
function heroFrame(s, i, on, label) {
  return `<div class="hs hr hr-${s.kind}${on ? " on" : ""}" data-hero="${i}" data-kind="${s.kind}" data-name="${esc(label)}" style="--c1:${s.colors[0]};--c2:${s.colors[1] || s.colors[0]}"></div>`;
}

// ---------- A: under the lights ----------
// The camera (same as the mock-up's): world x across the field (yards from
// the middle), z down it (0 the near goal line), e height in yards.
const CAM = { CX: 696, HY: 40, ZC: 41.6, K: 24232, KX: 1258.6, W: 1392, H: 600 };
const hP = (x, z, e) => {
  const zp = z + 10 + CAM.ZC;
  return [CAM.CX + (CAM.KX * x) / zp, CAM.HY + (CAM.K - CAM.KX * (e || 0)) / zp, zp];
};
const hpoly = (pts) => "M" + pts.map((p) => hf1(p[0]) + " " + hf1(p[1])).join(" L") + " Z";

function stadiumScene(h, c1) {
  let seed = 7;
  const rnd = () => { seed = (seed * 16807) % 2147483647; return seed / 2147483647; };
  const G = { empty: [], c1: [], c2: [], c3: [], struct: [], struct2: [], windows: [], lights: [], poles: [], trees: "", board: "" };
  const fill = Math.max(0, Math.min(1, h.fill || 0));
  const seatSq = (x, z, e, sz) => {
    const p = hP(x, z, e);
    if (p[0] < -8 || p[0] > CAM.W + 8 || p[1] < -8 || p[1] > 608) return;
    const d = Math.max(0.5, (sz * CAM.KX) / p[2] / 2);
    const s = "M" + hf1(p[0] - d) + " " + hf1(p[1] - d) + "h" + hf1(2 * d) + "v" + hf1(2 * d) + "h-" + hf1(2 * d) + "z";
    if (rnd() > fill) G.empty.push(s);
    else { const r = rnd(); (r < 0.62 ? G.c1 : r < 0.74 ? G.c2 : G.c3).push(s); }
  };
  const SZ = 0.36;
  const tier = (l0, dl, e0, de, rows, zA, zB, ends, step) => {
    const lT = l0 + dl * rows, eT = e0 + de * rows;
    for (const s of [-1, 1]) G.struct.push(hpoly([hP(s * l0, zA, e0 - 0.8), hP(s * l0, zB, e0 - 0.8), hP(s * lT, zB, eT + 0.6), hP(s * lT, zA, eT + 0.6)]));
    if (ends) G.struct.push(hpoly([hP(-l0, 110 + (l0 - 26.65), e0 - 0.8), hP(l0, 110 + (l0 - 26.65), e0 - 0.8), hP(lT, 110 + (lT - 26.65), eT + 0.6), hP(-lT, 110 + (lT - 26.65), eT + 0.6)]));
    for (let r = 0; r < rows; r++) {
      const l = l0 + dl * r, e = e0 + de * r;
      for (let z = zA; z <= zB; z += step) { seatSq(l, z, e, SZ); seatSq(-l, z, e, SZ); }
      if (ends) for (let x = -l; x <= l; x += step) seatSq(x, 110 + (l - 26.65), e, SZ);
    }
    return [lT, eT];
  };
  let board = null;
  if (h.level === "hs") {
    // Home bleachers with the press box, small visitor bleachers, six light
    // poles, the scoreboard and the tree line.
    const t = [];
    for (let x = -150; x <= 150; x += 3) t.push(hP(x, 150, 9 + 3.5 * Math.sin(x * 0.21) + 2.2 * Math.sin(x * 0.53 + 1) + rnd() * 1.5));
    const a = hP(-150, 150, 0), b = hP(150, 150, 0);
    G.trees = `M${hf1(a[0])} ${hf1(a[1])} L` + t.map((p) => hf1(p[0]) + " " + hf1(p[1])).join(" L") + ` L${hf1(b[0])} ${hf1(b[1])} Z`;
    const zh = [10, 90], zv = [28, 72];
    const lt = 31 + 0.95 * 14, et = 0.7 + 0.5 * 14;
    G.struct.push(hpoly([hP(-31, zh[0], 0), hP(-31, zh[1], 0), hP(-lt, zh[1], et), hP(-lt, zh[0], et)]));
    for (let r = 0; r < 14; r++) for (let z = zh[0]; z <= zh[1]; z += 1.1) seatSq(-(31 + 0.95 * r), z, 0.7 + 0.5 * r, 0.4);
    G.struct.push(hpoly([hP(31, zv[0], 0), hP(31, zv[1], 0), hP(31 + 0.95 * 7, zv[1], 0.7 + 0.5 * 7), hP(31 + 0.95 * 7, zv[0], 0.7 + 0.5 * 7)]));
    for (let r = 0; r < 7; r++) for (let z = zv[0]; z <= zv[1]; z += 1.1) seatSq(31 + 0.95 * r, z, 0.7 + 0.5 * r, 0.4);
    G.struct2.push(hpoly([hP(-lt - 0.5, 36, et + 0.5), hP(-lt - 0.5, 64, et + 0.5), hP(-lt - 0.5, 64, et + 4.5), hP(-lt - 0.5, 36, et + 4.5)]));
    G.windows.push(hpoly([hP(-lt - 0.6, 37, et + 2.2), hP(-lt - 0.6, 63, et + 2.2), hP(-lt - 0.6, 63, et + 3.8), hP(-lt - 0.6, 37, et + 3.8)]));
    for (const s of [-1, 1]) for (const z of [12, 50, 88]) {
      const b0 = hP(s * 42, z, 0), tp = hP(s * 42, z, 23);
      G.poles.push(`M${hf1(b0[0])} ${hf1(b0[1])} L${hf1(tp[0])} ${hf1(tp[1])}`);
      G.lights.push(hpoly([hP(s * 42, z - 2, 22.6), hP(s * 42, z + 2, 22.6), hP(s * 42, z + 2, 24.6), hP(s * 42, z - 2, 24.6)]));
    }
    board = [-11, 11, 140, 8, 15];
    for (const x of [-7, 7]) { const b0 = hP(x, 140, 0), tp = hP(x, 140, 8); G.poles.push(`M${hf1(b0[0])} ${hf1(b0[1])} L${hf1(tp[0])} ${hf1(tp[1])}`); }
  } else {
    // A bowl: lower deck, the suites band, an upper deck on the sides (all
    // the way round in the NFL), the lights on the rim, a board at college.
    const pro = h.level === "nfl" || h.level === "other";
    const lower = tier(34, 0.85, 1.2, 0.5, pro ? 30 : 28, -14, 114, true, 1.3);
    const lb = lower[0] + 1, eb = lower[1] + 0.8;
    for (const s of [-1, 1]) {
      G.struct2.push(hpoly([hP(s * lb, -14, eb), hP(s * lb, 114, eb), hP(s * lb, 114, eb + 3), hP(s * lb, -14, eb + 3)]));
      for (let z = -12; z < 112; z += 2.2) G.windows.push(hpoly([hP(s * (lb - 0.05), z, eb + 0.9), hP(s * (lb - 0.05), z + 1.6, eb + 0.9), hP(s * (lb - 0.05), z + 1.6, eb + 2.2), hP(s * (lb - 0.05), z, eb + 2.2)]));
    }
    const upper = tier(lb + 1.5, 0.8, eb + 3.6, 0.62, pro ? 20 : 16, -10, 110, pro, 1.5);
    for (const s of [-1, 1]) for (const z of [-6, 20, 46, 72, 98]) G.lights.push(hpoly([hP(s * (upper[0] + 0.5), z, upper[1] + 2), hP(s * (upper[0] + 0.5), z + 9, upper[1] + 2), hP(s * (upper[0] + 0.5), z + 9, upper[1] + 3.4), hP(s * (upper[0] + 0.5), z, upper[1] + 3.4)]));
    for (const s of [-1, 1]) G.struct2.push(hpoly([hP(s * (upper[0] + 0.5), -10, upper[1] + 0.6), hP(s * (upper[0] + 0.5), 110, upper[1] + 0.6), hP(s * (upper[0] + 0.5), 110, upper[1] + 2), hP(s * (upper[0] + 0.5), -10, upper[1] + 2)]));
    if (!pro) board = [-16, 16, 150, 13, 22];
  }
  let B = null;
  if (board && h.board) {
    const [xa, xb, zb, ea, eb2] = board, tl = hP(xa, zb, eb2), br = hP(xb, zb, ea);
    G.board = hpoly([tl, [br[0], tl[1]], br, [tl[0], br[1]]]);
    B = { x: tl[0], y: tl[1], w: br[0] - tl[0], h: br[1] - tl[1] };
  }
  const geo = {};
  for (const k of Object.keys(G)) geo[k] = Array.isArray(G[k]) ? G[k].join("") : G[k];
  return { geo, board: B, crowd: hshade(c1, 0.35) };
}

// The field, drawn once per slide with the team's end zones and crest.
function heroField(h, c1) {
  const hw = 26.65, o = [];
  o.push(`<path d="${hpoly([hP(-hw - 9, -14), hP(hw + 9, -14), hP(hw + 9, 114), hP(-hw - 9, 114)])}" fill="#0c1a12"/>`);
  o.push(`<path d="${hpoly([hP(-hw, -10), hP(hw, -10), hP(hw, 110), hP(-hw, 110)])}" fill="url(#hr-turf)"/>`);
  for (let z = 0; z < 100; z += 10) o.push(`<path d="${hpoly([hP(-hw, z), hP(hw, z), hP(hw, z + 5), hP(-hw, z + 5)])}" fill="rgba(255,255,255,.035)"/>`);
  o.push(`<path d="${hpoly([hP(-hw, -10), hP(hw, -10), hP(hw, 0), hP(-hw, 0)])}" fill="${c1}" opacity=".88"/>`);
  o.push(`<path d="${hpoly([hP(-hw, 100), hP(hw, 100), hP(hw, 110), hP(-hw, 110)])}" fill="${c1}" opacity=".88"/>`);
  for (let z = 0; z <= 100; z += 5) {
    const a = hP(-hw, z), b = hP(hw, z), edge = z === 0 || z === 100;
    o.push(`<path d="M${hf1(a[0])} ${hf1(a[1])} L${hf1(b[0])} ${hf1(b[1])}" stroke="#f4f5f7" stroke-opacity="${edge ? 0.8 : 0.42}" stroke-width="${(((edge ? 2.6 : 1.5) * 52) / a[2]).toFixed(2)}"/>`);
  }
  for (const s of [-1, 1]) {
    const a = hP(s * hw, -10), b = hP(s * hw, 110);
    o.push(`<path d="M${hf1(a[0])} ${hf1(a[1])} L${hf1(b[0])} ${hf1(b[1])}" stroke="#f4f5f7" stroke-opacity=".7" stroke-width="2.2"/>`);
  }
  const d = [];
  for (let z = 1; z < 100; z++) {
    if (z % 5 === 0) continue;
    for (const x of [-hw + 0.3, -3.1, 3.1, hw - 0.3]) { const a = hP(x - 0.35, z), b = hP(x + 0.35, z); d.push(`M${hf1(a[0])} ${hf1(a[1])}L${hf1(b[0])} ${hf1(b[1])}`); }
  }
  o.push(`<path d="${d.join("")}" stroke="#f4f5f7" stroke-opacity=".3" stroke-width=".8"/>`);
  for (let z = 10; z < 100; z += 10) {
    const n = 50 - Math.abs(50 - z);
    for (const s of [-1, 1]) {
      const [x, y, zp] = hP(s * (hw - 9), z);
      o.push(`<text transform="translate(${hf1(x)} ${hf1(y)}) scale(1 ${((19.25 / zp) * 1.7).toFixed(3)})" text-anchor="middle" dominant-baseline="middle" fill="#f4f5f7" fill-opacity=".34" font-weight="800" font-size="${hf1((2 * CAM.KX) / zp)}">${n}</text>`);
    }
  }
  const [mx, my, mz] = hP(0, 50);
  o.push(`<ellipse cx="${hf1(mx)}" cy="${hf1(my)}" rx="${hf1((6.5 * CAM.KX) / mz)}" ry="${hf1((6.5 * CAM.K) / mz / mz)}" fill="${c1}" fill-opacity=".9" stroke="#f4f5f7" stroke-opacity=".5"/>`);
  o.push(`<text transform="translate(${hf1(mx)} ${hf1(my + 1)}) scale(1 ${((19.25 / mz) * 1.6).toFixed(3)})" text-anchor="middle" dominant-baseline="middle" fill="#fff" font-weight="900" font-size="${hf1((4.2 * CAM.KX) / mz)}" style="font-stretch:112%">${esc(h.crest)}</text>`);
  const [nx, ny] = hP(0, -4.6);
  o.push(`<text transform="translate(${hf1(nx)} ${hf1(ny)}) scale(1 .44)" text-anchor="middle" dominant-baseline="middle" fill="#fff" fill-opacity=".94" font-weight="900" font-size="110" letter-spacing="6" style="font-stretch:118%" textLength="420" lengthAdjust="spacingAndGlyphs">${esc(h.ez[0])}</text>`);
  const [fx, fy, fz] = hP(0, 105);
  o.push(`<text transform="translate(${hf1(fx)} ${hf1(fy)}) scale(1 .30)" text-anchor="middle" dominant-baseline="middle" fill="#fff" fill-opacity=".85" font-weight="900" font-size="${hf1((4.4 * CAM.KX) / fz)}" letter-spacing="2" style="font-stretch:118%" textLength="${Math.round((32 * CAM.KX) / fz)}" lengthAdjust="spacingAndGlyphs">${esc(h.ez[1])}</text>`);
  return o.join("");
}

function heroCrest(h, c) {
  const t = String(h.crest || "");
  const fs = t.length > 2 ? 12 : 15;
  return `<svg class="hr-crest" width="40" height="46" viewBox="0 0 46 52" aria-hidden="true"><path d="M23 1 L44 8 L44 26 C44 39 34 47 23 51 C12 47 2 39 2 26 L2 8 Z" fill="${c[0]}" stroke="${c[1] || "#f4f5f7"}" stroke-width="1.5"/><text x="23" y="${fs < 15 ? 31 : 32}" text-anchor="middle" fill="#fff" font-size="${fs}" font-weight="900" style="font-stretch:112%">${esc(t)}</text></svg>`;
}

function slideA(s, W) {
  const h = s.hero, c = s.colors, c1 = c[0];
  const sc = Math.max(W / CAM.W, 1), ox = (W - CAM.W * sc) / 2, oy = (CAM.H - CAM.H * sc) / 2;
  const toScreen = (p) => [p[0] * sc + ox, p[1] * sc + oy];
  const { geo, board, crowd } = stadiumScene(h, c1);
  // The captains stand in a row below midfield, a stem up to each one's spot.
  const caps = h.captains || [];
  const n = caps.length, pc = toScreen(hP(0, 46));
  const pw = Math.max(132, Math.min(174, (W - 48) / Math.max(n, 1) - 12)), sp = pw + 12;
  const plates = caps.map((p, i) => {
    const x = W / 2 + (i - (n - 1) / 2) * sp, y = pc[1] + 40;
    return { p, x, y };
  });
  const dots = plates.map((q) => `<ellipse cx="${hf1((q.x - ox) / sc)}" cy="${hf1((pc[1] - oy) / sc)}" rx="6" ry="2.2" fill="#f4f5f7" fill-opacity=".85"/>`).join("");
  const T = h.team, A = h.asked;
  const road = (h.road || []).map((r) => `<div class="hr-cd ${r.in === "now" ? "now" : ""}"><span>${esc(r.date)}</span><span>${esc(r.text)}</span><span>${esc(r.in)}</span></div>`).join("");
  const plateHtml = plates.map((q) => `<span class="hr-stem" style="left:${hf1(q.x)}px;top:${hf1(pc[1] + 3)}px;height:${hf1(q.y - pc[1] - 3)}px"></span>` +
    heroPlayer(q.p, `${hface(q.p, "hr-pf", c1)}<span class="hr-pt"><b>${esc(q.p.name)}</b><small>${esc(q.p.pos)} · ${esc(q.p.cls)} · captain</small><em>${esc(q.p.line)}</em></span>`, "hr-plate", `left:${hf1(q.x - pw / 2)}px;top:${hf1(q.y)}px;width:${hf1(pw)}px`)).join("");
  let boardHtml = "";
  if (board && h.board) {
    const [bx, by] = toScreen([board.x, board.y]);
    const bw = board.w * sc, bh = board.h * sc;
    boardHtml = `<div class="hr-board" style="left:${hf1(bx)}px;top:${hf1(by)}px;width:${hf1(bw)}px;height:${hf1(bh)}px;font-size:${hf1(Math.min(bh * 0.19, bw / 17))}px"><span>${esc(h.board[0])}</span><span>${esc(h.board[1])}</span></div>`;
  }
  const avg = T.avg != null ? hfmt(T.avg) + (h.level === "nfl" ? "" : ` · ${T.fill}%`) : "-";
  const rival = A.rival ? `<div class="hr-rival">${linkTo("team", A.rival.ref, `<span class="hr-rc" style="background:${A.rival.colors[0]};box-shadow:inset -7px 0 0 ${A.rival.colors[1] || A.rival.colors[0]}"></span>`, "")}<span>${esc(A.rival.text)}</span></div>` : "";
  return `<svg class="hr-scene" viewBox="0 0 ${CAM.W} ${CAM.H}" preserveAspectRatio="xMidYMid slice" aria-hidden="true">
      <defs>
        <radialGradient id="hr-turf" cx="0.5" cy="0.55" r="0.75"><stop offset="0" stop-color="#2f7445"/><stop offset="0.55" stop-color="#215a35"/><stop offset="1" stop-color="#123320"/></radialGradient>
        <linearGradient id="hr-sky" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#05070b"/><stop offset="1" stop-color="#0b1018"/></linearGradient>
        <radialGradient id="hr-vig" cx="0.5" cy="0.5" r="0.72"><stop offset="0" stop-color="#030407" stop-opacity="0"/><stop offset="0.5" stop-color="#030407" stop-opacity="0.1"/><stop offset="1" stop-color="#030407" stop-opacity="0.82"/></radialGradient>
        <radialGradient id="hr-scrim" cx="0" cy="0" r="1"><stop offset="0" stop-color="#04060a" stop-opacity="0.92"/><stop offset="0.6" stop-color="#04060a" stop-opacity="0.7"/><stop offset="1" stop-color="#04060a" stop-opacity="0"/></radialGradient>
        <linearGradient id="hr-fade" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#04060a" stop-opacity="0"/><stop offset="1" stop-color="#04060a" stop-opacity=".75"/></linearGradient>
      </defs>
      <rect width="${CAM.W}" height="${CAM.H}" fill="url(#hr-sky)"/>
      <path d="${geo.trees}" fill="#070b09"/><path d="${geo.struct}" fill="#12151b"/><path d="${geo.struct2}" fill="#181b22"/>
      <path d="${geo.empty}" fill="#1c1f26"/><path d="${geo.c1}" fill="${crowd}"/><path d="${geo.c2}" fill="#80848c"/><path d="${geo.c3}" fill="#3a3f48"/>
      <path d="${geo.windows}" fill="#e9dfc4" fill-opacity=".34"/><path d="${geo.poles}" stroke="#3c414b" stroke-width="2" fill="none"/><path d="${geo.lights}" fill="#efeadb"/>
      <path d="${geo.board}" fill="#0b0d11" stroke="#2c313b"/>
      ${heroField(h, c1)}
      <rect width="${CAM.W}" height="${CAM.H}" fill="url(#hr-vig)"/>
      <rect width="${Math.round(900 / sc)}" height="${Math.round(330 / sc)}" x="${hf1(-ox / sc)}" fill="url(#hr-scrim)"/>
      ${dots}
      <rect y="400" width="${CAM.W}" height="200" fill="url(#hr-fade)"/>
    </svg>
    ${boardHtml}
    ${n ? `<span class="st-label hr-caplbl" style="left:${hf1(W / 2)}px;top:${hf1(pc[1] - 26)}px">${n > 3 ? `Your ${n} captains` : "Your captains"}</span>` : ""}
    ${plateHtml}
    <div class="hr-head"><span class="st-label">${esc(s.kicker)}</span><h2>${rich(s.title)}</h2><p>${rich(s.sub || "")}</p></div>
    <div class="hr-glass hr-road"><div class="hr-gh"><b>The road from here</b>${linkTo("calendar", {}, "Calendar", "hr-lk")}</div>${road || '<div class="hr-cd"><span></span><span class="muted">Nothing dated yet.</span><span></span></div>'}</div>
    <div class="hr-glass hr-lower">
      <div class="hr-lt">${heroCrest(h, c)}<div class="hr-ltn">${linkTo("team", s.go.args, esc(T.name), "")}<small>${esc([T.stadium, T.place].filter(Boolean).join(" · "))}</small></div><span class="hr-rec"><b>${esc(T.record)}</b><small>${esc(T.rec_label)}</small></span></div>
      <div class="hr-cells">
        <div><b>${T.seats ? hfmt(T.seats) : "-"}</b><small>seats</small></div>
        <div><b>${avg}</b><small>${esc(T.avg_label || "no home crowd yet")}</small><i class="hr-bar"><i style="width:${Math.min(100, T.fill || 0)}%;background:${(T.fill || 0) < 80 ? "#e0a73a" : "#f4f5f7"}"></i></i></div>
        <div><span>${esc(T.last_home || T.line2)}</span><small>${esc(T.last_home ? T.line2 : T.line3)}</small></div>
      </div>
    </div>
    <div class="hr-glass hr-asked">
      <span class="st-label">What's asked of you</span>
      ${A.wants ? `<q>${esc(A.wants)}</q>` : ""}
      <span class="hr-bl">${A.boss ? `<a class="hr-lk" data-go="people" data-args='{"tab":"Boss"}'>${esc(A.boss)}</a>, ${esc(A.line)}` : ""}</span>
      ${rival}
    </div>`;
}

// ---------- B: the program's story ----------
function storyChart(h, c, W, H) {
  const bars = h.bars || [];
  const slots = bars.length + 1;
  const X0 = 8, slot = Math.max(10, (W - X0 - 30) / slots), bw = Math.max(6, Math.min(18, slot * 0.66));
  const maxW = Math.max(12, ...bars.map((b) => b.w), h.line ? h.line.wins : 0, h.now ? h.now.w : 0) + 1;
  const base = H - 64, top = 20, hpx = (w) => ((base - top) * w) / maxW;
  const shades = [hshade(c[0], 0), hshade(c[0], 0.4)];
  const out = [];
  out.push(`<path d="${[0, 4, 8, 12, 16].filter((w) => w < maxW).map((w) => `M0 ${hf1(base - hpx(w))} H${hf1(X0 + slots * slot)}`).join(" ")}" stroke="rgba(255,255,255,.06)" fill="none"/>`);
  const label = slot >= 15;
  bars.forEach((b, i) => {
    const hh = Math.max(2, hpx(b.w)), x = X0 + i * slot;
    out.push(`<g class="hr-bg" data-go="schedule" data-args='{"season":${b.y}}'><title>${b.y}: ${esc(b.rec)}</title><rect x="${hf1(x)}" y="${hf1(base - hh)}" width="${hf1(bw)}" height="${hf1(hh)}" rx="3" fill="${b.you ? "#f4f5f7" : shades[b.era % 2]}" ${b.you ? 'fill-opacity=".85"' : ""}/>${b.cap ? `<rect x="${hf1(x)}" y="${hf1(base - hh)}" width="${hf1(bw)}" height="3" rx="1.5" fill="#F1BE48"/>` : ""}${label ? `<text x="${hf1(x + bw / 2)}" y="${hf1(base - hh - 5)}" text-anchor="middle" class="hr-bn">${b.w}</text>` : ""}</g>`);
  });
  const yx = X0 + bars.length * slot;
  if (h.now) {
    const target = h.line ? h.line.wins : Math.max(...bars.map((b) => b.w), 6);
    const yh = hpx(target), wh = hpx(h.now.w);
    out.push(`<rect x="${hf1(yx)}" y="${hf1(base - yh)}" width="${hf1(bw)}" height="${hf1(yh)}" rx="3" fill="rgba(255,255,255,.04)" stroke="#f4f5f7" stroke-opacity=".7" stroke-dasharray="3 3"/>`);
    if (h.now.played) out.push(`<rect x="${hf1(yx)}" y="${hf1(base - Math.max(2, wh))}" width="${hf1(bw)}" height="${hf1(Math.max(2, wh))}" rx="3" fill="#f4f5f7"/>`);
    out.push(`<text x="${hf1(yx + bw / 2)}" y="${hf1(base + 14)}" text-anchor="middle" class="hr-bt">${h.now.y}</text><text x="${hf1(yx + bw / 2)}" y="${hf1(base - yh - 6)}" text-anchor="middle" class="hr-bt">${h.now.played ? esc(h.now.rec) : "you"}</text>`);
  }
  if (h.line) {
    const ly = base - hpx(h.line.wins);
    out.push(`<path d="M0 ${hf1(ly)} H${hf1(yx + bw + 4)}" stroke="#f4f5f7" stroke-opacity=".5" stroke-dasharray="2 4"/>`);
    // The words end short of this season's bar, which carries its own label.
    out.push(`<text x="${hf1(Math.min(h.now ? yx - 8 : yx + bw, W - 4))}" y="${hf1(ly - 6)}" text-anchor="end" class="hr-lt2">${esc(h.line.text)}</text>`);
  }
  (h.eras || []).forEach((e) => {
    const i0 = bars.findIndex((b) => b.y === e.from), i1 = bars.findIndex((b) => b.y === e.to);
    if (i0 < 0 || i1 < 0) return;
    const xa = X0 + i0 * slot - (slot - bw) / 2, xb = X0 + i1 * slot + bw + (slot - bw) / 2, span = xb - xa;
    out.push(`<path d="M${hf1(xa)} ${hf1(base + 8)} V${hf1(base + 44)}" stroke="rgba(255,255,255,.18)"/>`);
    const nm = span >= e.coach.length * 6.2 + 6 ? e.coach : "";
    const rec = span >= e.rec.length * 6 + 6 && e.to - e.from >= 2 ? e.rec : "";
    if (nm) out.push(`<text x="${hf1((xa + xb) / 2)}" y="${hf1(base + 26)}" text-anchor="middle" class="hr-en ${e.you ? "you" : ""}"${e.id != null ? ` data-go="coach" data-id="${e.id}"` : ""}><title>${esc(e.full)} · ${e.from}-${e.to} · ${esc(e.rec)}</title>${esc(nm)}</text>`);
    if (rec) out.push(`<text x="${hf1((xa + xb) / 2)}" y="${hf1(base + 40)}" text-anchor="middle" class="hr-er">${esc(rec)}</text>`);
  });
  const caps = bars.some((b) => b.cap);
  out.push(`<text x="0" y="${hf1(H - 4)}" class="hr-foot">Wins in each season since ${bars.length ? bars[0].y : ""}, one shade per head coach${caps ? `; gold top: ${esc(h.title_word)}` : ""}.</text>`);
  return `<svg width="${W}" height="${H}" viewBox="0 0 ${W} ${H}" role="img" aria-label="Wins in every season, by head coach">${out.join("")}</svg>`;
}

function slideB(s) {
  const h = s.hero, c = s.colors;
  const marks = (h.marks || []).map((m, i) => `<div class="hr-mk ${m.state}"><span class="st-cond ${i === 0 ? "first" : ""}">${esc(m.n)}</span><span><b>${esc(m.t)}</b><small>${esc(m.l)}</small></span></div>`).join("");
  const wall = (h.wall.rows || []).map((l) => {
    const pic = l.face ? hface(l, "hr-lgf", c[0]) : `<span class="hr-lgi">${esc(initials(l.full || l.name))}</span>`;
    const body = `${pic}<b>${esc(l.name)}</b><small>${esc(l.what)}${l.nick && l.full ? ` (${esc(l.full)})` : ""}</small>${l.more ? `<small>${esc(l.more)}</small>` : ""}`;
    return l.linked ? heroPlayer(l, body, "hr-lg") : `<span class="hr-lg">${body}</span>`;
  }).join("");
  return `<div class="hr-b">
    <div class="hr-bl-col">
      <div class="hr-kick">${heroCrest(h, c)}<span><span class="st-label">${esc(s.kicker)}</span><small>${esc(h.kicker2)}</small></span></div>
      <h2>${rich(s.title)}</h2>
      <p>${rich(s.sub || "")}</p>
      <div class="hr-stings">${(h.stings || []).map((x) => `<div><b class="st-cond">${esc(x.n)}</b><small>${esc(x.l)}</small></div>`).join("")}</div>
      <div class="hr-chart"></div>
    </div>
    <div class="hr-br-col">
      <div class="hr-ph"><b>What you could be the first to do</b><small>from ${(h.bars || []).length} seasons of records</small></div>
      <div class="hr-marks">${marks || '<div class="muted">The record book is still empty.</div>'}</div>
      <div class="hr-ph"><b>${esc(h.wall.title)}</b><small>${esc(h.wall.sub)}</small></div>
      <div class="hr-wall">${wall || '<span class="muted">No names yet.</span>'}</div>
      <small class="hr-wn">${esc(h.wall.note || "")}</small>
    </div>
  </div>`;
}

// ---------- C: the room ----------
function slideC(s) {
  const h = s.hero, c = s.colors;
  const tone = (t) => (t === 2 ? "amber" : t === 1 ? "hurt" : "");
  return `<span class="hr-word st-wide">${esc(h.word)}</span>
    <div class="hr-photo"></div>
    <div class="hr-cl">
      <div class="hr-kick">${heroCrest(h, c)}<span class="st-label">${esc(s.kicker)}</span></div>
      <h2>${rich(s.title)}</h2>
      <p>${rich(s.sub || "")}</p>
      <div class="hr-facts">${(h.facts || []).map((f, i) => `<div class="${tone(f.tone)} ${i === 0 ? "first" : ""}"><b class="st-cond">${esc(f.n)}</b><span>${esc(f.t)}</span></div>`).join("")}</div>
    </div>
    <div class="hr-caption"></div>`;
}

// Last season's starters on three risers: offense at the back, defense in
// the middle, specialists and captains in front with your empty spot.
function roomPhoto(s, box, cap) {
  const h = s.hero, c1 = s.colors[0];
  const W = box.clientWidth, H = box.clientHeight;
  const back = h.back || [], mid = h.mid || [], caps = (h.caps || []).map((p) => ({ ...p, cap: true })), st = h.st || [];
  const half = Math.ceil(caps.length / 2), sl = Math.ceil(st.length / 2);
  const front = [...st.slice(0, sl), ...caps.slice(0, half), { you: true }, ...caps.slice(half), ...st.slice(sl)];
  const rows = [back, mid, front], scale = [0.9, 0.96, 1.04];
  const maxN = Math.max(...rows.map((r) => r.length), 1);
  const slot = Math.max(30, Math.min(84, (W - 40) / maxN, H / 5.2));
  const k = slot / 84;
  // The group sits in the middle of the frame, risers and all.
  const tall = 222 * k + slot * scale[2] * 1.36 * 1.2 + 16;
  const oy = Math.max(8, (H - tall) / 2);
  const tops = [oy, oy + 106 * k, oy + 222 * k];
  let html = "";
  rows.forEach((row, ri) => {
    const sp = slot * scale[ri], w = sp * 1.36, hh = w * 1.2, x0 = W / 2 - (row.length * sp) / 2;
    html += `<span class="hr-riser" style="left:${hf1(x0 - 24)}px;width:${hf1(row.length * sp + 48)}px;top:${hf1(tops[ri] + hh * 0.8)}px"></span>`;
    row.forEach((p, i) => {
      const x = x0 + i * sp + sp / 2 - w / 2, y = tops[ri];
      if (p.you) {
        html += `<span class="hr-you" title="You, ${esc(h.you_role || "head coach")}" style="left:${hf1(x0 + i * sp + 4)}px;top:${hf1(y + 14 * k)}px;width:${hf1(sp - 8)}px;height:${hf1(hh * 0.72)}px"><b>You</b>${sp >= 64 ? `<small>${esc(h.you_role || "head coach")}</small>` : ""}</span>`;
        return;
      }
      html += `<button class="hr-hd" data-go="player" data-id="${p.id}" title="${esc(p.name)} · ${esc(p.pos)} · ${esc(p.cls)}${p.cap ? " · captain" : ""}" style="left:${hf1(x)}px;top:${hf1(y)}px;width:${hf1(w)}px;height:${hf1(hh)}px;z-index:${ri + 1}">${hface(p, "hr-hdf", c1)}</button>`;
    });
  });
  box.innerHTML = html;
  const fr = front.map((p) => (p.you ? "you" : `${esc(p.last)} (${esc(p.pos)}${p.cap ? ", captain" : ""})`)).join(", ");
  cap.innerHTML = `<span class="st-label">${esc(h.cap_head)}</span>
    <span><b>Front row, from left:</b> ${fr}</span>
    <span><b>Middle row:</b> ${mid.map((p) => esc(p.last)).join(", ") || "-"}</span>
    <span><b>Back row:</b> ${back.map((p) => esc(p.last)).join(", ") || "-"}</span>`;
}

// ---------- drawing and faces ----------
function heroDraw() {
  const car = document.querySelector(".hcar");
  if (!car) return;
  const stage = car.querySelector(".hstage");
  const W = stage ? stage.clientWidth : car.clientWidth;
  if (!W) return;
  car.dataset.w = W < 1120 ? "s" : W < 1300 ? "m" : "l";
  for (const el of car.querySelectorAll(".hs.hr")) {
    const s = HERO.data[Number(el.dataset.hero)];
    if (!s) continue;
    if (s.kind === "stadium") {
      el.innerHTML = slideA(s, W);
      heroFitPlates(el);
    }
    else if (s.kind === "story") {
      if (!el.firstChild) el.innerHTML = slideB(s);
      const box = el.querySelector(".hr-chart");
      if (box) box.innerHTML = storyChart(s.hero, s.colors, box.clientWidth, Math.max(150, box.clientHeight));
    } else if (s.kind === "room") {
      if (!el.firstChild) el.innerHTML = slideC(s);
      roomPhoto(s, el.querySelector(".hr-photo"), el.querySelector(".hr-caption"));
    }
  }
  heroFaces(car);
}
// The captains' row clears the cards below it: the stems shorten first,
// then the plates drop their stat line.
function heroFitPlates(el) {
  const plates = [...el.querySelectorAll(".hr-plate")];
  if (!plates.length) return;
  const cards = [...el.querySelectorAll(".hr-asked, .hr-lower")].map((c) => c.getBoundingClientRect());
  const over = () => Math.max(0, ...plates.map((p) => {
    const r = p.getBoundingClientRect();
    return Math.max(0, ...cards.filter((c) => r.right > c.left && r.left < c.right).map((c) => r.bottom + 10 - c.top));
  }));
  let need = over();
  if (!need) return;
  const lift = Math.min(need, 28);
  for (const p of plates) p.style.top = `${parseFloat(p.style.top) - lift}px`;
  for (const st of el.querySelectorAll(".hr-stem")) st.style.height = `${Math.max(6, parseFloat(st.style.height) - lift)}px`;
  if (over()) el.classList.add("hr-tight");
}
function heroFaces(root) {
  const slots = [...root.querySelectorAll(".hr-face[data-face]")];
  if (!slots.length) return;
  const put = (id, src) => root.querySelectorAll(`.hr-face[data-face="${id}"]`).forEach((s) => {
    if (!s.querySelector("img")) s.insertAdjacentHTML("beforeend", `<img src="${src}" alt="">`);
  });
  const ids = [...new Set(slots.map((s) => Number(s.dataset.face)).filter((x) => x > 0))];
  if (window.AFMFaces && window.AFMFaces.pics) window.AFMFaces.pics(ids, put);
}
let heroResize = 0;
window.addEventListener("resize", () => {
  clearTimeout(heroResize);
  heroResize = setTimeout(() => { if (document.querySelector(".hcar .hs.hr")) heroDraw(); }, 120);
});

// People to know first (the frame under the hero).
function heroKnow(list, colors) {
  if (!list || !list.length) return '<div class="empty">Nobody stands out yet.</div>';
  return `<div class="hr-know">${list.map((k) => `<div class="hr-kn ${k.tone === 2 ? "amber" : k.tone === 1 ? "hurt" : ""}">${hface(k, "hr-kf", colors[0])}<span>${playerLink(k.id, k.name)} <small>${esc(k.pos)}</small></span><span class="hr-kw">${esc(k.why)}</span></div>`).join("")}</div>`;
}

// The Handbook's page for Home (lane 8's hbAdd). hero.js loads before the
// Handbook's scripts, so the page is added once every script has run.
window.addEventListener("DOMContentLoaded", () => {
  if (typeof hbAdd !== "function") return;
  hbAdd({
    id: "home", group: "Running the team", title: "Home",
    lede: "Home opens on three big slides about your team: your stadium on a game night, the program's story and the room you coach. Your next game, the last result and the season's news follow them.",
    how: [
      ["Under the lights", "Your stadium at night, filled to last season's real home crowd: grey seats are seats nobody bought. Your captains stand at midfield, and the cards show the road ahead, your boss's words and your rival."],
      ["The program's story", "Every season on record as a bar, shaded by head coach, with titles in gold. The marks list what nobody here has done for a long time; each turns done or missed as your seasons go."],
      ["The room", "Last season's starters in a team photo, this season's once the games begin, with an empty spot for you. Beside it: who is unhappy, who is hurt and the room's mood."],
      ["Switching", "The arrows and dots under the slides move between them, and Home opens on the one you picked last. Left alone they turn every eight seconds, and stop while your pointer is over them."],
    ],
    lv: {
      hs: "The board at the far end shows your record. The wall lists the seniors leaving for college in June, and the rival line is the school you play for a trophy.",
      college: "The rival line is your biggest rivalry series. The wall holds the program's award winners and, when there are few, its highest draft picks.",
      nfl: "No board at the far end. The rival line is your division's standings, and the wall holds the club's award winners.",
    },
    where: [["Home", "home"], ["History", "history"], ["Squad", "roster"], ["People, the room", "people", { tab: "Room" }]],
    rel: ["calendar", "people", "draft"],
  });
  hbScreen("home", "home", [
    ["Under the lights", "Your stadium, your captains, the road from here and what's asked of you."],
    ["The program's story", "Your seasons on record and the marks you could be the first to reach."],
    ["The room", "The starters' photo and how the room feels."],
    ["People to know first", "Three people to meet first, and why: the unhappiest player, a captain, the most hurt.", "People to know first"],
    ["Continue", "Plays days until something needs you."],
  ]);
});
