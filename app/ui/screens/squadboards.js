// The player and roster boards (update 2, lane 9A): the depth chart and
// the needs map in the stadium (V1, V2), the roster at a calm glance (V3),
// a player's shape (V4), his career arc (V5) and his body and testing (V6),
// at every seat that has a squad. Every rating is your staff's range (with
// the rating card on hover), never a true number; potential stays words.
// Clicks go through CORE_MORE (data-act "sb…"); a redraw that only moves
// the picked spot or row reuses the last answers (SB.reuse).
"use strict";

const SB = { cache: {}, reuse: false };
async function sbGet(cmd, args = {}) {
  const key = cmd + "|" + JSON.stringify(args);
  if (SB.reuse && SB.cache[key]) return SB.cache[key];
  const x = await api(cmd, args);
  SB.cache[key] = x;
  return x;
}
function sbLocal(el) {
  SB.reuse = true;
  Promise.resolve(refresh(el)).finally(() => { SB.reuse = false; });
}
const sbNum = (v) => (typeof v === "number" && isFinite(v) ? v : null);
const sbMid = (r) => (r ? Math.round((r[0] + r[1]) / 2) : null);
const sbPlural = (n, w, ws) => `${n} ${n === 1 ? w : ws || w + "s"}`;
const sbOrd = (n) => { const s = ["th", "st", "nd", "rd"], v = n % 100; return n + (s[(v - 20) % 10] || s[v] || s[0]); };
// A board's scale runs from its floor to 99. The floor is 40, where college
// and NFL reads sit; a board whose reads sit lower (high school, the small
// colleges) drops it by tens, from the lowest tenth of its numbers.
function sbFloor(...vs) {
  const v = vs.flat(2).filter((n) => typeof n === "number" && isFinite(n)).sort((a, b) => a - b);
  if (!v.length) return 40;
  const low = v[Math.floor((v.length - 1) * 0.1)];
  return Math.max(0, Math.min(40, Math.floor((low - 5) / 10) * 10));
}
// The bar of a range on the floor-99 scale (soft band, the middle marked).
const sbPct = (v, f = 40) => Math.max(0, Math.min(100, ((v - f) / (99 - f)) * 100));
function sbBand(r, tick, f = 40) {
  if (!r) return `<span class="sb-bar"><span class="sb-ns">Not seen</span></span>`;
  const l = sbPct(r[0], f), w = Math.max(1.5, sbPct(r[1], f) - l);
  return `<span class="sb-bar"><i class="sb-bd" style="left:${l}%;width:${w}%"></i><i class="sb-md" style="left:${sbPct((r[0] + r[1]) / 2, f)}%"></i>${tick != null ? `<i class="sb-tk" style="left:${sbPct(tick, f)}%"></i>` : ""}</span>`;
}

// ---------- the stadium (V1, V2) ----------
// The broadcast camera behind the unit: field x across 0-900, depth dy
// from the near edge (0) up the field (negative); a perspective stage of
// 980 x 640, drawn in SVG so it scales with the page.
const SB_ST = { W: 980, H: 640, PV: 1200, CO: Math.cos(Math.PI / 6), SI: Math.sin(Math.PI / 6), OX: 490, OY: 128, BY: 640 };
function sbProj(fx, dy) {
  const s = SB_ST, dx = fx - 450, Y = s.BY + dy * s.CO, Z = dy * s.SI, k = s.PV / (s.PV - Z);
  return [s.OX + dx * k, s.OY + (Y - s.OY) * k, k];
}
// Across the field in yards (from the middle) to x: the line spread wide
// enough for a plate at every spot, the wide-outs near the sidelines.
const SB_ACROSS = [[0, 0], [2.2, 112], [4.4, 224], [8, 318], [13, 388], [21, 436], [27, 452]];
function sbAcross(y) {
  const a = Math.abs(y);
  for (let i = 1; i < SB_ACROSS.length; i++) {
    const [y0, x0] = SB_ACROSS[i - 1], [y1, x1] = SB_ACROSS[i];
    if (a <= y1) return 450 + Math.sign(y) * (x0 + ((a - y0) / (y1 - y0)) * (x1 - x0));
  }
  return 450 + Math.sign(y) * 452;
}
// Hash marks by level (yards in from each sideline): high school 17.8,
// college 20, the NFL 23.6.
function sbHashes(level) {
  const lv = String(level || "");
  const h = lv === "NFL" ? 23.58 : /FBS|FCS|college|Division|NAIA|JUCO/i.test(lv) ? 20 : 17.78;
  return [h, 53.33 - h].map((y) => (y / 53.33) * 900);
}
function sbField(level, los, yd, firstDown) {
  const P = (x, d) => sbProj(x, d).slice(0, 2).map((n) => n.toFixed(1)).join(",");
  const grass = `<polygon class="sb-grass" points="${P(-60, 0)} ${P(960, 0)} ${P(960, -760)} ${P(-60, -760)}"/>`;
  const field = `<polygon class="sb-turf" points="${P(0, 0)} ${P(900, 0)} ${P(900, -760)} ${P(0, -760)}"/>`;
  const stripes = [];
  const lines = [];
  const nums = [];
  for (let n = -30; n <= 30; n++) {
    const d = los + n * 5 * yd;
    if (d > 0 || d < -760) continue;
    if (n % 2 === 0) {
      const d2 = d - 5 * yd;
      if (d2 >= -760) stripes.push(`<polygon class="sb-stripe" points="${P(0, d)} ${P(900, d)} ${P(900, d2)} ${P(0, d2)}"/>`);
    }
    lines.push(`<line class="sb-yl" x1="${sbProj(0, d)[0].toFixed(1)}" y1="${sbProj(0, d)[1].toFixed(1)}" x2="${sbProj(900, d)[0].toFixed(1)}" y2="${sbProj(900, d)[1].toFixed(1)}"/>`);
    // The ball on its own 35: numbers every ten yards.
    const yardline = 35 + n * 5;
    const shown = yardline > 50 ? 100 - yardline : yardline;
    if (shown > 0 && shown % 10 === 0) {
      for (const fx of [70, 830]) {
        const [x, y, k] = sbProj(fx, d);
        nums.push(`<text class="sb-num" x="${x.toFixed(1)}" y="${(y + 9 * k).toFixed(1)}" font-size="${(26 * k).toFixed(1)}">${String(shown).split("").join(" ")}</text>`);
      }
    }
  }
  const hashes = [];
  for (const hx of [...sbHashes(level), 4, 896]) {
    for (let n = -60; n <= 30; n++) {
      const d = los + n * yd;
      if (d > 0 || d < -760 || n % 5 === 0) continue;
      const [a, b] = [sbProj(hx - 7, d), sbProj(hx + 7, d)];
      hashes.push(`<line class="sb-hash" x1="${a[0].toFixed(1)}" y1="${a[1].toFixed(1)}" x2="${b[0].toFixed(1)}" y2="${b[1].toFixed(1)}"/>`);
    }
  }
  const lineAt = (d, cls) => `<line class="${cls}" x1="${sbProj(0, d)[0].toFixed(1)}" y1="${sbProj(0, d)[1].toFixed(1)}" x2="${sbProj(900, d)[0].toFixed(1)}" y2="${sbProj(900, d)[1].toFixed(1)}"/>`;
  return `<svg class="sb-field" viewBox="0 0 ${SB_ST.W} ${SB_ST.H}" preserveAspectRatio="xMidYMid slice" aria-hidden="true">
    <defs><linearGradient id="sbsky" x1="0" y1="0" x2="0" y2="1"><stop offset="0" class="sb-sky0"/><stop offset="1" class="sb-sky1"/></linearGradient><filter id="sbblur" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="16"/></filter></defs>
    <rect width="${SB_ST.W}" height="${SB_ST.H}" fill="url(#sbsky)"/>${grass}${field}${stripes.join("")}${lines.join("")}${hashes.join("")}${nums.join("")}
    ${lineAt(los, "sb-los")}${firstDown != null ? lineAt(firstDown, "sb-fd") : ""}</svg>`;
}
// Lay the plates out so none covers another. Each item stands on its
// spot (x, y is the bottom middle, k its scale); w and h are a plate's
// size in stage units at scale 1 and vgap the room its stem needs below.
// Plates at about the same depth, on the same side of the ball, form a row,
// packed across in their order (the men a step back move a row back, the
// rest shrink a little, two rows when even that won't fit); then each row
// steps toward the camera until it clears the plates above it.
function sbSpread(items, w, h, vgap) {
  const gap = 6, L = 6, R = SB_ST.W - 6;
  const wide = (a) => w * a.k;
  const rows = [];
  for (const a of [...items].sort((p, q) => p.y - q.y)) {
    const r = rows[rows.length - 1];
    if (r && a.y - r[0].y < h * 0.6 && a.side === r[0].side) r.push(a);
    else rows.push([a]);
  }
  const pack = (r) => {
    r.sort((p, q) => p.x - q.x);
    let edge = L;
    for (const a of r) { a.x = Math.max(a.x, edge + wide(a) / 2); edge = a.x + wide(a) / 2 + gap; }
    edge = R;
    for (const a of [...r].reverse()) { a.x = Math.min(a.x, edge - wide(a) / 2); edge = a.x - wide(a) / 2 - gap; }
  };
  const fits = (r) => r.reduce((s, a) => s + wide(a) + gap, -gap) <= R - L;
  for (let n = 0; n < rows.length; n++) {
    const r = rows[n];
    if (!fits(r)) {
      // The men a step off the line go back a row first (a flanker, a
      // slot), then the rest shrink to fit.
      const scale = (row) => (R - L - gap * (row.length - 1)) / row.reduce((s, a) => s + wide(a), 0);
      r.sort((p, q) => p.y - q.y);
      const back = [];
      while (r.length > 1 && scale(r) < 0.9 && r[r.length - 1].y > r[0].y + 4) back.unshift(r.pop());
      if (back.length) rows.splice(n + 1, 0, back);
      const f = scale(r);
      if (f < 1) for (const a of r) a.k *= Math.max(0.72, f);
    }
    if (!fits(r)) {
      // Every other plate steps back a row.
      r.sort((p, q) => p.x - q.x);
      const back = r.filter((_, i) => i % 2 === 1);
      for (const a of back) a.y += h * a.k + vgap;
      rows.splice(n + 1, 0, back);
      rows[n] = r.filter((_, i) => i % 2 === 0);
    }
    pack(rows[n]);
  }
  const placed = [];
  for (const r of rows) {
    let shift = 0;
    for (const a of r) {
      for (const b of placed) {
        if (Math.abs(a.x - b.x) >= (wide(a) + wide(b)) / 2 + gap) continue;
        const clear = b.y + vgap - (a.y - h * a.k);
        if (clear > shift) shift = clear;
      }
    }
    for (const a of r) a.y += shift;
    placed.push(...r);
  }
  // Keep the whole formation inside the stage.
  const low = Math.max(...items.map((a) => a.y + vgap / 2));
  if (low > SB_ST.H - 2) for (const a of items) a.y -= low - (SB_ST.H - 2);
  return items;
}
const sbAt = (x, y) => `left:${((x / SB_ST.W) * 100).toFixed(2)}%;top:${((y / SB_ST.H) * 100).toFixed(2)}%`;

// ---------- V1: the depth chart in the stadium ----------
const SB_DC_TABS = ["Offense", "Defense", "Special teams", "Returners"];
const SB_DC_ALL = [...SB_DC_TABS, "Packages", "By position"];
const SB_TAB_KEY = { Offense: "offense", Defense: "defense", "Special teams": "special", Returners: "returns" };
// A flat board's label to its stadium tab and formation.
const SB_FORM_OF = {
  Offense: ["Offense", "offense"], Jumbo: ["Offense", "jumbo"], "Goal line offense": ["Offense", "goal_line_o"],
  Defense: ["Defense", "defense"], Nickel: ["Defense", "nickel"], Dime: ["Defense", "dime"], "Goal line defense": ["Defense", "goal_line_d"],
  Specialists: ["Special teams", "specialists"], "Field goal and extra point": ["Special teams", "field_goal"], "Kickoff coverage": ["Special teams", "kick_cover"], "Punt coverage": ["Special teams", "punt_cover"],
  "Kickoff return": ["Returners", "kick_return"], "Punt return": ["Returners", "punt_return"],
};
const SB_FORM_LABEL = Object.fromEntries(Object.entries(SB_FORM_OF).map(([label, [, f]]) => [f, label]));
const SB_DEF_FORMS = ["defense", "nickel", "dime", "goal_line_d", "punt_return"];
function sbDcTabs(cur, x) {
  const t = tabs("trboard", SB_DC_ALL, cur);
  const view = keep("sbdcview", { v: "Broadcast" }).v;
  let extra = "";
  if (SB_DC_TABS.includes(cur) && x) {
    const tab = x.tabs.find((q) => q.label === cur);
    const forms = tab ? tab.forms : [];
    extra = `<span class="sb-tabx">${forms.length > 1 ? `<span class="sb-seg" aria-label="Formation">${forms.map((f) => `<button data-act="sbform" data-tab="${esc(cur)}" data-v="${esc(f.key)}" aria-pressed="${f.key === x.form}">${esc(f.label)}</button>`).join("")}</span>` : ""}<span class="sb-seg" aria-label="View">${["Broadcast", "Overhead"].map((v) => `<button data-act="sbdcview" data-v="${v}" aria-pressed="${v === view}">${v}</button>`).join("")}</span></span>`;
  }
  return `<div class="sb-tabrow">${t}${extra}</div>`;
}
const sbDepthBase = screens.depth;
screens.depth = async (a) => {
  a = a || {};
  if (a.kind && typeof trMine === "function" && !trMine(a)) return sbDepthBase(a);
  const k = keep("trboard", { t: "Offense" });
  // A flat board asked for by name (another page's link, an old keep)
  // opens on its stadium tab with its formation.
  for (const v of [a.tab, k.t]) {
    if (v && SB_FORM_OF[v] && !SB_DC_TABS.includes(v)) {
      const [tab, form] = SB_FORM_OF[v];
      keep("sbform", {})[tab] = form;
      k.t = tab;
      a = { ...a, tab };
      break;
    }
  }
  if (a.tab && SB_DC_ALL.includes(a.tab)) k.t = a.tab;
  if (!SB_DC_ALL.includes(k.t)) k.t = "Offense";
  const cur = k.t;
  if (cur === "Packages" || cur === "By position") {
    const html = await sbDepthBase({ ...a, tab: cur });
    k.t = cur;
    return html.replace(/<div class="tabsline" role="tablist">[\s\S]*?<\/div>/, sbDcTabs(cur, null));
  }
  const form = keep("sbform", {})[cur] || SB_TAB_KEY[cur];
  let x;
  try {
    x = await sbGet("sb_depth", { tab: SB_TAB_KEY[cur], form });
  } catch (e) {
    const html = await sbDepthBase({ ...a, tab: "By position" });
    k.t = cur;
    return html;
  }
  if (keep("sbdcview", { v: "Broadcast" }).v === "Overhead") {
    // The flat drag-and-drop board of this formation, under the stadium's tabs.
    const label = SB_FORM_LABEL[x.form] || "Offense";
    k.t = label;
    const html = await sbDepthBase({ ...a, tab: label });
    k.t = cur;
    return html.replace(/<div class="tabsline" role="tablist">[\s\S]*?<\/div>/, sbDcTabs(cur, x));
  }
  return sbDepthPage(x, cur);
};
function sbPlate(s, k, sel, pos) {
  const p = s.picks[0];
  const rd = (m) => (m && m.read ? `<b class="sb-prr"${rrAttrs({ p: m.id, k: "ovr" }, m.read[0], m.read[1])}>${m.read[0]}-${m.read[1]}</b>` : "");
  const back = s.picks.slice(1).map((b, i) => `<span class="sb-pb"><i>${i + 2}</i><span>${b ? esc(b.last) : '<span class="sb-dim">Nobody</span>'}</span>${rd(b)}</span>`).join("");
  const tag = p && p.health === "out" ? '<span class="sb-tag bad">OUT</span>' : p && p.health === "limited" ? '<span class="sb-tag warn">Q</span>' : "";
  return `<button class="sb-plate ${sel ? "on" : ""} ${p ? "" : "empty"}" style="${sbAt(pos.x, pos.y)};--k:${pos.k.toFixed(3)}" data-act="sbspot" data-k="${k}" aria-label="${esc(s.pos_full)}${p ? ", " + esc(p.name) : ""}">
    <span class="sb-pt"><span class="sb-pn">${p && p.num != null ? p.num : "–"}</span><span class="sb-ps">${esc(s.slot)}</span>${rd(p).replace('class="sb-prr"', 'class="sb-pr"')}</span>
    <span class="sb-pl"><span class="sb-plt">${p ? esc(p.last) : "Open"}</span>${tag}</span>${back}${p && s.by ? `<span class="hp-by hp-pby ${s.by.toLowerCase()}" aria-hidden="true">${s.by}</span>` : ""}</button>`;
}
function sbUnits(units) {
  return `<div class="sb-units">${units.map((u, i) => `<div class="sb-unit ${i === units.length - 1 ? "all" : ""}"><div class="sb-unit-h"><span class="sb-lbl">${esc(u.name)}</span><small>${u.rank ? `${esc(u.rank_words)} ${esc(u.among)}` : esc(u.among || "")}</small></div><div class="sb-unit-v"><span class="sb-grade">${esc(u.grade)}</span><b>${u.value != null ? u.value : "–"}</b></div><span class="sb-ubar"><i style="width:${u.pct != null ? u.pct : 0}%"></i></span></div>`).join("")}</div>`;
}
function sbRing(r, size = 84) {
  const m = r ? (r[0] + r[1]) / 2 : 0;
  const R = size / 2 - 6, C = 2 * Math.PI * R;
  return `<span class="sb-ring" style="width:${size}px;height:${size}px"><svg viewBox="0 0 ${size} ${size}" aria-hidden="true"><circle cx="${size / 2}" cy="${size / 2}" r="${R}" class="sb-ring-t"/><circle cx="${size / 2}" cy="${size / 2}" r="${R}" class="sb-ring-v" stroke-dasharray="${((m / 99) * C).toFixed(1)} 999" transform="rotate(-90 ${size / 2} ${size / 2})"/></svg><b>${r ? Math.round(m) : "–"}</b></span>`;
}
function sbDepthPage(x, cur) {
  setColors(x.team.colors);
  title("Depth chart", cur);
  const selKey = "sbdc:" + x.form;
  const sel = Math.min(keep(selKey, { k: 0 }).k, Math.max(0, x.slots.length - 1));
  const def = SB_DEF_FORMS.includes(x.form);
  const LOS = -440, YD = 27;
  const items = x.slots.map((s) => {
    const behind = Math.max(-2, Math.min(16, def ? s.x : -s.x));
    const [px, py] = sbProj(sbAcross(s.y), LOS + behind * YD);
    // Every plate keeps one size, as on a broadcast, so every name stays legible.
    return { x: px, y: py, k: 1 };
  });
  // A plate is 10 by 8.25 em on a base of 1.25% of the stage (squadboards.css).
  sbSpread(items, 123, 101, 11);
  const plates = x.slots.map((s, i) => sbPlate(s, i, i === sel, items[i])).join("");
  const s = x.slots[sel];
  const head = pageHead({
    kicker: `${esc(x.level)} · ${esc(x.form_label)}`,
    title: `<span class="sb-crest">${mark(x.team, 40)}</span>Depth Chart`,
    sub: x.head ? "Your staff's order where you leave it, yours where you set it. Click a spot for the man and the depth behind him." : "The head coach sets what isn't yours. Click a spot for the man and the depth behind him.",
    goto: gotoLine([["Squad", "roster"], ["Needs map", "needs"], ["Training", "training"], ["Game day", "gameday"], ["Schedule", "schedule"]]),
  });
  const next = x.next ? `<span><span class="sb-lbl">Next game</span> ${x.next.home ? "vs" : "at"} ${teamChip(x.next.opp, 18)} <small>${esc(x.next.date || "")}${x.next.in != null ? ` · ${sbPlural(x.next.in, "day")}` : ""}</small></span>` : `<span class="sb-dim">No game ahead this season.</span>`;
  const last = x.last_game ? `<span><span class="sb-lbl">Last game</span> ${esc(x.last_game.opp)} <small>${x.last_game.score[0]}-${x.last_game.score[1]} · ${esc(x.last_game.date)}</small></span>` : "";
  const actions = x.yours ? `<button class="st-btn small glass" data-act="sbreset" data-form="${esc(x.form)}">Give it back to the staff</button>` : "";
  const stage = `<div class="sb-stage" role="group" aria-label="${esc(x.form_label)} on the field">${sbField(x.level, LOS, YD, def ? null : LOS - 10 * YD)}${plates}</div>`;
  return `<div class="film sb"><div class="pad sb-pad">${head}${sbDcTabs(cur, x)}
    <div class="sb-dc"><div class="sb-dc-main"><div class="sb-next">${next}${last}${actions ? `<span class="sb-act">${actions}</span>` : ""}</div>${stage}${hpDcFoot(x)}${sbUnits(x.units)}</div>
    <aside class="sb-side">${s ? sbSpotPanel(x, s, sel) : '<div class="sb-empty">No spots on this formation.</div>'}</aside></div></div></div>`;
}
function sbSpotPanel(x, s, k) {
  const p = s.picks[0];
  const d = s.detail;
  if (!p) return `<div class="sb-ph"><span class="sb-lbl gold">${esc(s.pos_full)}</span><h2 class="sb-name">Open</h2><p class="sb-dim">Nobody is listed here. The engine sends the next man left in the room.</p></div>`;
  const keys = d ? d.keys.map((q) => `<div class="sb-kr"><span>${esc(q.label)}</span>${range(q.read, null, { p: p.id, k: q.key })}</div>`).join("") : "";
  const snaps = d && d.snaps != null && d.team_snaps ? `${d.snaps} of ${d.team_snaps}` : "–";
  const depth = s.picks.map((m, i) => {
    if (!m) return `<div class="sb-dr ${i === 0 ? "first" : ""}"><span class="sb-di">${i + 1}</span><span class="sb-dim">Nobody</span></div>`;
    const up = i > 0 && s.controls && x.slots[k] ? `<button class="sb-up" data-act="sbup" data-k="${k}" data-n="${i + 1}" title="Move ${esc(m.name)} up" aria-label="Move ${esc(m.name)} up">↑</button>` : "";
    const hurt = m.health !== "full" ? ` <span class="sb-tag ${m.health === "out" ? "bad" : "warn"}" title="${esc(m.health_why || "")}">${m.health === "out" ? "Out" : "Limited"}</span>` : "";
    const ready = i === 0 ? "Starter" : m.ready;
    return `<div class="sb-dr ${i === 0 ? "first" : ""}"><span class="sb-di">${i + 1}</span><span class="sb-dn">${playerLink(m.id, m.name)}${hurt}<small>${esc(m.class)} · <span class="${/Needs|Out|Redshirt/.test(ready) ? "sb-warn" : ""}">${esc(ready)}</span>${m.pos !== s.pos ? ` · plays ${esc(m.pos)}` : ""}</small></span>${range(m.read, null, { p: m.id, k: "ovr" })}${up}</div>`;
  }).join("");
  const coach = d && d.coach ? `<div class="sb-coach"><span class="sb-ci">${esc(String(d.coach.name || "?").split(" ").map((w) => w[0]).slice(0, 2).join(""))}</span><div><div class="sb-cn">${d.coach.you ? "You" : coachLink(d.coach.id, d.coach.name || "")}, ${esc(d.coach.title)}</div><p>${esc(d.note)}</p></div></div>` : "";
  return `<span class="sb-bignum" aria-hidden="true">${p.num != null ? p.num : ""}</span>
    <div class="sb-ph"><span class="sb-lbl gold">${esc(s.pos_full)} · Starter</span><span class="sb-first">${esc(p.first)}</span><span class="sb-last">${esc(p.last)}</span><span class="sb-meta">${p.num != null ? "#" + p.num + " · " : ""}${esc(p.class_full)} · ${esc(p.size)} · ${esc(p.home)}</span></div>
    <div class="sb-hero">${sbRing(p.read)}<div><div class="sb-lbl">Staff read</div>${range(p.read, "his position coach", { p: p.id, k: "ovr" })}${d && d.band ? `<div class="sb-bw">${esc(d.band.word)}</div><small class="sb-dim">${esc(d.band.line)}</small>` : ""}</div></div>
    <div class="sb-cells"><div><span class="sb-lbl">Snaps last game</span><b>${snaps}</b></div><div><span class="sb-lbl">Grade last game</span><b>${d && d.grade != null ? d.grade : "–"}</b></div></div>
    ${hpWhyHere(x, s, k)}
    <div class="sb-sec"><span class="sb-lbl">What matters at ${esc(s.pos)}</span>${keys}</div>
    <div class="sb-sec"><span class="sb-lbl">Depth at ${esc(s.slot)}</span>${depth}</div>
    ${coach}
    <div class="st-actions">${linkTo("player", { id: p.id }, "Player page", "st-btn small")}<button class="st-btn small glass" data-act="pcmp" data-id="${p.id}">Compare</button></div>`;
}

// ---------- H4 (lane 6): who put him there, and why (board 37) ----------
const HP_BY = { Y: "You", C: "Coordinator", A: "Auto chart" };
function hpDcFoot(x) {
  const legend = `<div class="hp-legend">${Object.entries(HP_BY).map(([k, t]) => `<span><span class="hp-by ${k.toLowerCase()}">${k}</span>${t}</span>`).join("")}</div>`;
  const ch = (x.changes || []).map((c) => `<div class="hp-change"><span class="hp-by ${esc(String(c.who).toLowerCase())}">${esc(c.who)}</span><span>${rich(c.text)}</span><small>${esc(c.date)}</small></div>`).join("");
  return `<div class="hp-dcfoot"><div class="hp-dcl"><span class="sb-lbl">Who set each spot</span>${legend}</div><div class="hp-dcl"><span class="sb-lbl">Changes this week</span>${ch ? `<div class="hp-changes">${ch}</div>` : '<span class="sb-dim">No changes this week.</span>'}</div></div>`;
}
function hpWhyHere(x, s, k) {
  const w = s.why;
  if (!w) return "";
  const why = (w.why || []).map((r) => `<li><span><b>${esc(r.t)}</b>${rich(r.s)}</span></li>`).join("");
  const alts = (w.alts || []).map((m) => `<div class="hp-alt"><div>${playerLink(m.id, m.name)}<small>${esc(m.pos)} · ${esc(m.class)}</small><small class="${m.hurt ? "hurt" : ""}">${esc(m.note)}</small></div>${range(m.read, null, { p: m.id, k: "ovr" })}</div>`).join("");
  const reps = (w.practice || []).filter((r) => r.grade).map((r) => `<div class="hp-rep"><span>${playerLink(r.id, r.name)}<small>${esc(r.words)}</small></span><i><u style="width:${Math.max(0, Math.min(100, r.grade))}%"></u></i></div>`).join("");
  const second = s.picks[1];
  const calls = (s.controls && second ? `<button class="st-btn small glass" data-act="sbup" data-k="${k}" data-n="2">Move ${esc(second.last)} up</button>` : "")
    + (w.by === "Y" ? `<button class="st-btn small glass" data-act="sbreset" data-form="${esc(x.form)}">Give it back to the staff</button>` : "");
  return `<div class="sb-sec hp-wh"><span class="sb-lbl">Why is he here?</span>
    <div class="hp-wh-by"><span class="hp-by ${esc(String(w.by).toLowerCase())}">${esc(w.by)}</span><span><b>${esc(w.by_name)}</b><small>${esc(w.when)}</small></span></div>
    ${why ? `<ol>${why}</ol>` : ""}
    ${alts ? `<span class="sb-lbl">${esc(w.else_title)}</span><div class="hp-alts">${alts}</div>` : ""}
    ${reps ? `<span class="sb-lbl">Practice grades</span><div class="hp-reps">${reps}</div>` : ""}
    ${calls ? `<div class="hp-calls">${calls}</div>` : ""}</div>`;
}

// ---------- V2: the needs map in the stadium ----------
const SB_VERDICTS = ["Hole", "Thin", "Average", "Solid", "Strength"];
const sbVc = (v) => "v-" + String(v || "Hole").toLowerCase();
screens.needs = async (a) => {
  a = a || {};
  const year = Math.max(0, Math.min(2, keep("sbneedy", { y: 0 }).y));
  const x = await sbGet("sb_needs", { year });
  setColors(x.team.colors);
  const y = year;
  title("Needs map", x.years[y].label);
  const sel = keep("sbneeds", { k: "QB" });
  const s = x.spots.find((q) => q.key === sel.k) || x.spots[0];
  const LOS = -330, YD = 24;
  const items = x.spots.map((p) => {
    // Offense below the line (toward the camera), defense above it.
    const [px, py] = sbProj(sbAcross(p.y), LOS - p.x * YD);
    return { x: px, y: py, k: 1, side: p.x > 0 ? "d" : "o" };
  });
  // A chip is 6.6 by 4.6 em on a base of 1.15% of the stage (squadboards.css).
  sbSpread(items, 75, 52, 9);
  const pools = x.spots.map((p, i) => {
    const [cx, cy] = [items[i].x, items[i].y - 14 * items[i].k];
    return `<ellipse class="sb-pool ${sbVc(p.verdicts[y])}" cx="${cx.toFixed(1)}" cy="${cy.toFixed(1)}" rx="${(52 * items[i].k).toFixed(1)}" ry="${(22 * items[i].k).toFixed(1)}" filter="url(#sbblur)"/>`;
  }).join("");
  const chips = x.spots.map((p, i) => {
    const v = p.vals[y], v0 = p.vals[0];
    const d = y > 0 && v != null && v0 != null && v !== v0 ? `<i class="${v > v0 ? "up" : "down"}">${v > v0 ? "▲" : "▼"}${Math.abs(v - v0)}</i>` : "";
    return `<button class="sb-chip ${sbVc(p.verdicts[y])} ${p.key === s.key ? "on" : ""}" style="${sbAt(items[i].x, items[i].y)};--k:${items[i].k.toFixed(3)}" data-act="sbneed" data-k="${esc(p.key)}" aria-label="${esc(p.full)}: ${esc(p.verdicts[y])}"><span>${esc(p.key.replace(/\d$/, ""))}${d}</span><b>${v != null ? v : "–"}</b></button>`;
  }).join("");
  const field = sbField(x.level, LOS, YD, null).replace("</svg>", `${pools}</svg>`);
  const legend = `<div class="sb-legend">${SB_VERDICTS.map((v) => `<span><i class="${sbVc(v)}"></i>${v}</span>`).join("")}</div>`;
  const yearCards = x.years.map((q, i) => `<button class="sb-year ${i === y ? "on" : ""}" data-act="sbneedy" data-v="${i}"><span class="sb-lbl">${esc(q.label)} · ${q.fall}</span><small>${esc(x.notes[i])}</small><b>${q.avg != null ? q.avg : "–"}</b><small class="${q.holes ? "sb-warn" : ""}">${sbPlural(q.holes, "hole")} · ${q.thin} thin</small></button>`).join("");
  const leaving = `<div class="sb-leave"><span class="sb-lbl">After this season</span>${x.leaving.map((l) => `<div class="sb-lv" title="${esc((l.names || []).join(", "))}"><b>${l.n}</b><span>${esc(l.label)}</span></div>`).join("")}</div>`;
  const nf = sbFloor(x.spots.map((q) => q.vals));
  const bars = s.vals.map((v, i) => `<button class="sb-ybar ${i === y ? "on" : ""}" data-act="sbneedy" data-v="${i}"><span class="sb-yb"><i class="${sbVc(s.verdicts[i])}" style="height:${v != null ? sbPct(v, nf) : 0}%"></i></span><b>${v != null ? v : "–"}</b><small>${x.falls[i]}</small></button>`).join("");
  const rows = s.names.map((n, i) => `<div class="sb-sr ${i === y ? "on" : ""}"><span>${x.falls[i]}</span><span>${n ? `${playerLink(n.id, n.name)} <small>${esc(n.class)}${n.commit ? " · commit" : ""}</small>` : '<span class="sb-warn">Nobody</span>'}</span><b class="${sbVc(s.verdicts[i])}-t">${s.vals[i] != null ? s.vals[i] : "–"}</b></div>`).join("");
  const fills = x.kind === "college" ? [["Recruiting board", "recruit_board"], ["Portal", "portal_desk"], ["NIL", "nil_desk"]] : x.kind === "nfl" ? [["Cap", "fo_cap"], ["Expiring deals", "fo_expiring"], ["Draft board", "draftboard"]] : [["Squad", "roster"], ["Training", "training"]];
  const panel = `<div class="sb-ph"><span class="sb-lbl gold">${esc(s.full)} · fall ${x.falls[y]}</span><span class="sb-bignum2 ${sbVc(s.verdicts[y])}-t">${s.vals[y] != null ? s.vals[y] : "–"}</span><span class="sb-verdict ${sbVc(s.verdicts[y])}-t">${esc(s.verdicts[y])}</span><span class="sb-meta">${s.names[y] ? `${esc(s.names[y].name)} starts` : "Nobody to start"}</span></div>
    <div class="sb-sec"><span class="sb-lbl">By year</span><div class="sb-ybars">${bars}</div></div>
    <div class="sb-sec"><span class="sb-lbl">Starter each year</span>${rows}</div>
    <div class="sb-sec"><span class="sb-lbl">Why it changes</span><p>${esc(s.why)}</p></div>
    <div class="sb-sec"><span class="sb-lbl">How to fill it</span><p>${esc(s.fix)}</p></div>
    <div class="st-actions">${fills.map(([l, g]) => linkTo(g, {}, l, "st-btn small glass")).join("")}</div>`;
  const head = pageHead({
    kicker: `${esc(x.level)} · roster planning`,
    title: `<span class="sb-crest">${mark(x.team, 40)}</span>Where the Holes Open`,
    sub: `Each starting spot now and the next two falls: who stays, who leaves, who arrives, each man's read carried forward by how players his age grow. ${esc(x.roster_line)}.`,
    goto: gotoLine([["Depth chart", "depth"], ["Squad", "roster"], ...fills.slice(0, 2)]),
  });
  return `<div class="film sb"><div class="pad sb-pad">${head}
    <div class="sb-dc"><div class="sb-dc-main"><div class="sb-next"><span class="sb-lbl">${esc(x.years[y].label)} · fall ${x.falls[y]}</span><small class="sb-dim">${esc(x.notes[y])}</small>${legend}</div>
      <div class="sb-stage needs" role="group" aria-label="Needs on the field">${field}<span class="sb-side-l off">Offense</span><span class="sb-side-l def">Defense</span>${chips}</div>
      <div class="sb-years">${yearCards}${leaving}</div><p class="sb-foot">${esc(x.draft_note)}</p></div>
    <aside class="sb-side">${panel}</aside></div></div></div>`;
};
Object.assign(I, { needs: '<path d="M4 20V10M10 20V4M16 20v-7M22 20H2"/><circle cx="16" cy="9" r="2"/>' });
navAdd({ go: "needs", label: "Needs map", icon: "needs", sec: "team", seats: ["college", "nfl", "hs"], after: "depth" });
GOTO.needs = () => [["Depth chart", "depth"], ["Squad", "roster"]];

// ---------- V3: the roster at a calm glance ----------
// The Squad's first view; the six working views stay beside it.
const sbRosterBase = screens.roster;
screens.roster = async (a) => {
  a = a || {};
  const mine = typeof trMine === "function" ? trMine(a) : !a.kind;
  const k = keep("sqview", { v: "Overview" });
  if (a.view) k.v = a.view;
  if (!mine || k.v !== "Overview") {
    const html = await sbRosterBase(a);
    if (!mine) return html;
    // The Overview button joins the six views.
    return html.replace('<div class="seg tr-views">', `<div class="seg tr-views"><button data-act="tr-sqview" data-v="Overview" aria-pressed="false">Overview</button>`);
  }
  let x;
  try {
    x = await sbGet("sb_roster");
  } catch (e) {
    k.v = "General";
    return sbRosterBase(a);
  }
  setColors(x.team.colors);
  title("Squad", "Overview");
  const rail = keep("sbrail", { g: "ALL" });
  const groups = x.groups.filter((g) => g.count > 0);
  const vseg = `<div class="seg tr-views"><button data-act="tr-sqview" data-v="Overview" aria-pressed="true">Overview</button>${["General", "Ratings", "Development", "Fitness", "Depth", "Contract"].map((v) => `<button data-act="tr-sqview" data-v="${v}" aria-pressed="false">${v === "Contract" && x.level !== "nfl" ? "Eligibility" : v}</button>`).join("")}</div>`;
  const railHtml = `<nav class="sb-rail" aria-label="Position groups"><button data-act="sbrail" data-v="ALL" aria-pressed="${rail.g === "ALL"}"><span>All</span><small>${x.rows.length}</small></button>${groups.map((g) => `<button data-act="sbrail" data-v="${g.key}" aria-pressed="${rail.g === g.key}"><span>${esc(g.key)}</span><small>${g.count}</small></button>`).join("")}</nav>`;
  const shown = groups.filter((g) => rail.g === "ALL" || g.key === rail.g);
  const selId = keep("sbrow", { id: null });
  const all = x.rows;
  const pick = all.find((r) => r.id === selId.id) || all.find((r) => shown.some((g) => g.key === r.rail)) || all[0];
  const money = x.level === "nfl" ? "Cap" : x.level === "college" ? "NIL" : "Offers";
  const head = `<div class="sb-rh"><span>#</span><span>Player</span><span>Year</span><span>Staff rating</span><span title="His grades in your last six games; a ring is a game he didn't play">Last 6</span><span title="His share of his unit's snaps in ${esc(x.season_words)}">Snaps</span><span>Morale</span><span>${money}</span></div>`;
  const body = shown.map((g) => {
    const rows = all.filter((r) => r.rail === g.key).sort((p, q) => (p.depth || 99) - (q.depth || 99) || sbMid(q.ovr) - sbMid(p.ovr));
    const u = g.unit;
    const mix = g.mix.map((m) => `${m.n} ${esc(m.class)}`).join(" · ");
    return `<div class="sb-band"><span class="sb-bn">${esc(g.name)}</span><small>${sbPlural(g.count, "player")} · starters ${u.value != null ? u.value : "–"}${u.rank ? ` · ${esc(u.rank_words)} ${esc(u.among)}` : ""}</small><small class="sb-mix">${mix}</small></div>${rows.map((r) => sbRosterRow(r, pick && r.id === pick.id)).join("")}`;
  }).join("");
  const pageH = pageHead({
    kicker: `${esc(x.level_name)} · ${sbPlural(x.rows.length, "player")}`,
    title: `<span class="sb-crest">${mark(x.team, 40)}</span>${esc(x.team.name)}`,
    sub: `Your staff reads them every day. Snaps and grades from ${esc(x.season_words)}'s games.`,
    goto: gotoLine([["Depth chart", "depth"], ["Needs map", "needs"], ["Training", "training"], ["Staff", "staff"]]),
  });
  return `<div class="film sb"><div class="pad sb-pad">${pageH}<div class="filters">${vseg}</div>
    <div class="sb-roster">${railHtml}<div class="sb-rt" role="table">${head}${body}</div><aside class="sb-side">${pick ? sbRosterPanel(pick, x) : ""}</aside></div></div></div>`;
};
function sbPips(y) {
  if (!y || !y.n) return `<span class="sb-dim">${esc((y && y.words) || "")}</span>`;
  const pips = Array.from({ length: y.n }, (_, i) => `<i class="${i < y.done ? "on" : ""}"></i>`).join("");
  return `<span class="sb-pips" title="${esc(y.left || "")}">${pips}</span><small>${esc(y.words)}</small>`;
}
function sbSpark(last6) {
  const W = 92, H = 24;
  const n = last6.length;
  if (!n) return '<span class="sb-dim">No games</span>';
  const xs = (i) => 6 + (i * (W - 12)) / 5;
  const ys = (g) => H - 3 - ((Math.max(30, Math.min(95, g)) - 30) / 65) * (H - 6);
  const pts = last6.map((g, i) => (g != null ? [xs(i), ys(g)] : null));
  let path = "";
  pts.forEach((p, i) => { if (p) path += `${path && pts[i - 1] ? "L" : "M"}${p[0].toFixed(1)},${p[1].toFixed(1)}`; });
  const dots = pts.map((p, i) => (p ? `<circle cx="${p[0].toFixed(1)}" cy="${p[1].toFixed(1)}" r="2" class="sb-sd"/>` : `<circle cx="${xs(i).toFixed(1)}" cy="${H / 2}" r="2.6" class="sb-dnp"/>`)).join("");
  const tip = last6.map((g) => (g == null ? "didn't play" : g)).join(", ");
  return `<svg class="sb-spark" viewBox="0 0 ${W} ${H}" width="${W}" height="${H}" role="img" aria-label="Last six grades: ${tip}"><title>${tip}</title><line x1="0" x2="${W}" y1="${ys(60).toFixed(1)}" y2="${ys(60).toFixed(1)}" class="sb-avg"/><path d="${path}" class="sb-sl"/>${dots}</svg>`;
}
function sbRosterRow(r, on) {
  const hurt = r.health !== "full" ? ` <span class="sb-tag ${r.health === "out" ? "bad" : "warn"}" title="${esc(r.health_why || r.status || "")}">${r.health === "out" ? "Out" : "Limited"}</span>` : "";
  const m = r.money || {};
  return `<div class="sb-rr ${on ? "on" : ""}" data-act="sbrow" data-id="${r.id}" role="row">
    <span class="sb-num">${r.num != null ? r.num : ""}</span>
    <span class="sb-who">${playerLink(r.id, r.name)}${hurt}<small>${esc(r.pos)} · ${esc(r.ht)}, ${r.wt} · ${esc(r.home)}</small></span>
    <span class="sb-yr">${sbPips(r.years)}</span>
    <span class="sb-rd"><b>${r.ovr ? sbMid(r.ovr) : "–"}</b>${range(r.ovr, null, { p: r.id, k: "ovr" })}</span>
    <span>${sbSpark(r.last6 || [])}</span>
    <span class="sb-sn">${r.snaps_pct != null ? `<span class="sb-sb"><i style="width:${r.snaps_pct}%"></i></span><b>${r.snaps_pct}%</b>` : '<span class="sb-dim">–</span>'}</span>
    <span class="${r.unhappy ? "sb-warn" : ""}">${esc(r.mood)}</span>
    <span title="${esc(m.note || "")}">${m.value ? esc(m.value) : '<span class="sb-dim">–</span>'}</span></div>`;
}
function sbRosterPanel(r, x) {
  const [first, ...rest] = String(r.name).split(" ");
  const m = r.money || {};
  const keys = (r.keys || []).map((q, i) => `<div class="sb-kr"><span>${esc(q.label)}</span>${range(q.read, null, { p: r.id, k: r["k" + i + "_key"] })}</div>`).join("");
  const line = r.line ? `<div class="sb-sec"><span class="sb-lbl">${r.line.year === x.season ? "This season" : `${r.line.year} season`}</span><p class="sb-big">${esc(r.line.line)}</p><small class="sb-dim">${r.line.games != null ? sbPlural(r.line.games, "game") : ""} · ${esc(r.line.level || "")}</small></div>` : `<div class="sb-sec"><span class="sb-lbl">This season</span><p class="sb-dim">No games yet.</p></div>`;
  return `<span class="sb-bignum" aria-hidden="true">${r.num != null ? r.num : ""}</span>
    <div class="sb-ph"><span class="sb-lbl gold">${esc(r.pos)} · ${esc(r.class_full || r.class)}${r.num != null ? " · #" + r.num : ""}</span><span class="sb-first">${esc(first)}</span><span class="sb-last">${esc(rest.join(" "))}</span><span class="sb-meta">${esc(r.ht)}, ${r.wt} lb · ${esc(r.home)}</span></div>
    <div class="sb-card"><span class="sb-lbl">Staff read</span><div class="sb-hero2"><b>${r.ovr ? sbMid(r.ovr) : "–"}</b>${range(r.ovr, "his position coach", { p: r.id, k: "ovr" })}</div>${sbBand(r.ovr, null, sbFloor(x.rows.map((q) => (q.ovr ? q.ovr[0] : null))))}</div>
    <div class="sb-sec"><span class="sb-lbl">What matters at ${esc(r.pos)}</span>${keys}</div>
    <div class="sb-cells"><div><span class="sb-lbl">Snaps</span><b>${r.snaps_pct != null ? r.snaps_pct + "%" : "–"}</b></div><div><span class="sb-lbl">${esc(m.label || "Money")}</span><b>${esc(m.value || "–")}</b></div><div><span class="sb-lbl">Health</span><b class="${r.health !== "full" ? "sb-warn" : ""}">${r.health === "out" ? "Out" : r.health === "limited" ? "Limited" : "Full"}</b></div><div><span class="sb-lbl">Morale</span><b class="${r.unhappy ? "sb-warn" : ""}">${esc(r.mood)}</b></div></div>
    ${line}
    <div class="st-actions">${linkTo("player", { id: r.id }, "Player page", "st-btn small")}${linkTo("player", { id: r.id, tab: "People" }, "Talk", "st-btn small glass")}</div>`;
}
