// Lane screens: rating ranges and your staff's quality (update 2, lane 3).
"use strict";
/* RR1-RR4 and Q1-Q5, in Press Box:
 *
 *   The hover card  every range that knows whose it is (`range(r, who,
 *                   ctx)` in app.js, `rrBar` here) explains itself: a
 *                   player's against the starters and backups at his level
 *                   and position this season, with the combine for a
 *                   physical rating (RR1, RR4); a staff range against the
 *                   same job at the same level (RR2).
 *   Your staff      people_staff's Quality tab (board 17): every member by
 *                   department with two key reads, faint when they started
 *                   and solid now, and the one you pick on the right; Jobs
 *                   and hand-offs says who runs what for you.
 *   Ratings         the coach page's tab for your own staff (board 18), and
 *                   staff_person, the same page for support staff (Q4).
 */

Object.assign(CRUMBS, { staff_person: "Staff" });

// ---------- small pieces ----------
const RR_WORDS = ["Poor", "Weak", "Average", "Good", "Very good", "Elite"];
// A read in a word, as the game says it: "Very good", or "Weak to good".
function rrWords(lo, hi) {
  const band = (v) => (v <= 4 ? 0 : v <= 8 ? 1 : v <= 11 ? 2 : v <= 14 ? 3 : v <= 17 ? 4 : 5);
  if (hi - lo >= 5) return `${RR_WORDS[band(lo)]} to ${RR_WORDS[band(hi)].toLowerCase()}`;
  return RR_WORDS[band(Math.floor((lo + hi) / 2))];
}
const rrLower = (s) => String(s || "").replace(/^./, (c) => c.toLowerCase());
// Growth in rating points with its sign: "+1.2", "-0.4", and "0.0", never "-0.0".
function rrSigned(v) {
  const r = Math.round(v * 10) / 10;
  return r === 0 ? "0.0" : (r > 0 ? "+" : "") + r.toFixed(1);
}
// A staff member's name: a coach opens his page on Ratings, anyone else
// his own staff page.
function rrName(r, cls = "link name") {
  return r.coach != null ? `<a class="${cls}" data-go="coach" data-id="${r.coach}" data-tab="Ratings">${esc(r.name)}</a>` : linkTo("staff_person", { who: r.who }, esc(r.name), cls);
}
// A read on the hidden 1-20 scale: the read when he started faint, the
// shown one solid (amber when it is a weak spot, gold when picked), and
// the numbers with an arrow when it fell this season.
function rrBar(f, who, opts = {}) {
  const now = opts.at !== undefined ? opts.at : f.now;
  if (!now) return `<span class="rr-b ns"><span class="rr-t"></span><b>—</b></span>`;
  const p = (v) => ((v - 0.5) / 20) * 100;
  const seg = (r, cls) => (r ? `<i class="${cls}" style="left:${p(r[0])}%;width:${p(r[1] + 1) - p(r[0])}%"></i>` : "");
  const weak = now[1] <= 9;
  const cls = weak ? "weak" : opts.on ? "gold" : "now";
  const start = opts.start !== undefined ? opts.start : f.start;
  const dn = !opts.noArrow && f.moved < 0 ? '<em title="Your read fell this season">↓</em>' : "";
  return `<span class="rr-b"${rrAttrs({ s: who, k: f.key }, now[0], now[1])}><span class="rr-t"><u style="left:${p(10)}%"></u>${seg(start, "st")}${seg(now, cls)}</span><b>${now[0]}-${now[1]}${dn}</b></span>`;
}

// ---------- the hover card (RR1-RR4) ----------
const RR = { cache: new Map(), timer: 0, hideTimer: 0, watch: 0, el: null, on: null };
function rrHide() {
  clearTimeout(RR.timer);
  clearInterval(RR.watch);
  if (RR.el) RR.el.remove();
  RR.el = null;
  RR.on = null;
}
// The scale with its marks as ticks and the range laid on it.
function rrScale(range, ticks, lo0, hi0, gold) {
  const pct = (v) => Math.max(0, Math.min(100, ((v - lo0) / (hi0 - lo0)) * 100));
  const a = pct(range[0] - 0.5), b = pct(range[1] + 0.5);
  return `<div class="rr-scale"><span class="${gold ? "gold" : ""}" style="left:${a}%;width:${Math.max(1.5, b - a)}%"></span>${ticks.map((t) => `<i class="${t.cls || ""}" style="left:${pct(t.v)}%"></i>`).join("")}</div>`;
}
function rrPlayerHtml(x) {
  const m = x.marks;
  const lo0 = Math.max(1, Math.min(x.range[0], m.backup) - 8);
  const hi0 = Math.min(99, Math.max(x.range[1], m.elite) + 6);
  return `<div class="rr-top"><span class="st-label">${esc(x.title)} · your staff's read</span><b>${x.range[0]}-${x.range[1]}</b></div>
    <div class="rr-word">${esc(x.band)}</div>
    ${rrScale(x.range, [{ v: m.backup }, { v: m.starter }, { v: m.elite, cls: "e" }], lo0, hi0, true)}
    <div class="rr-marks"><span><small>Typical backup</small><b>${m.backup}</b></span><span><small>Typical starter</small><b>${m.starter}</b></span><span><small>Elite from</small><b>${m.elite}</b></span></div>
    <p>${esc(x.line)}</p>
    ${x.combine ? `<p class="rr-comb"><span class="st-label">At the combine</span>${esc(x.combine)}</p>` : ""}
    <p class="rr-pool">${esc(x.pool)}, ${esc(x.count)}.${x.widened ? ` ${esc(x.widened)}` : ""}</p>
    ${x.change ? rrChangeHtml(x.change, x.dev ? x.pid : null) : ""}`;
}
// H2 (lane 6): how the read moved since the last season's end, and why.
function rrChangeHtml(c, pid) {
  const cls = /^Up/.test(c.word) ? "up" : /^Down/.test(c.word) ? "down" : "";
  return `<div class="hp-chg"><div class="rr-top"><span class="st-label">Since the end of ${esc(c.season)}</span><b class="${cls}">${esc(c.word)}</b></div>
    <p>${esc(c.line)}</p>
    ${c.why && c.why.length ? `<ul>${c.why.map((t) => `<li>${rich(t)}</li>`).join("")}</ul>` : ""}
    ${pid != null ? playerLink(pid, "His Development tab", "Development") : ""}</div>`;
}
function rrStaffHtml(x) {
  const m = x.marks;
  return `<div class="rr-top"><span class="st-label">${esc(x.title)} · your read</span><b>${x.range[0]}-${x.range[1]}</b></div>
    <div class="rr-word">${esc(x.word)} <small>${esc(rrLower(x.sure))}</small></div>
    ${rrScale(x.range, m ? [{ v: m.q25 }, { v: m.q50 }, { v: m.q90, cls: "e" }] : [], 0.5, 20.5, true)}
    ${m ? `<div class="rr-marks"><span><small>Bottom quarter under</small><b>${m.q25}</b></span><span><small>Typical</small><b>${m.q50}</b></span><span><small>Best tenth from</small><b>${m.q90}</b></span></div><p>${esc(x.band)}</p>` : ""}
    <p class="rr-rest"><span class="st-label">What it rests on</span>${esc(x.rests)}</p>
    <p class="rr-pool">${esc(x.pool)}, on the hidden 1 to 20 scale. Nobody sees a true number.</p>`;
}
function rrShow(t, x) {
  if (RR.on !== t || !t.isConnected) return;
  if (RR.el) RR.el.remove();
  const el = document.createElement("div");
  el.className = "rr-card sl";
  el.setAttribute("role", "tooltip");
  const lo = t.dataset.lo, hi = t.dataset.hi;
  el.innerHTML = x.error
    ? `<div class="rr-top"><span class="st-label">Your read</span><b>${esc(lo)}-${esc(hi)}</b></div><p>${esc(x.error)}</p>`
    : t.dataset.rr.startsWith("s:") ? rrStaffHtml(x) : rrPlayerHtml({ ...x, pid: Number(t.dataset.rr.slice(2)) });
  document.body.appendChild(el);
  // A range laid out in its table's columns has no box of its own: its bar does.
  let r = t.getBoundingClientRect();
  if (!r.width && t.firstElementChild) r = t.firstElementChild.getBoundingClientRect();
  const w = el.offsetWidth, h = el.offsetHeight;
  let left = r.left - 12, top = r.bottom + 8;
  if (left + w > window.innerWidth - 12) left = window.innerWidth - w - 12;
  if (top + h > window.innerHeight - 12) top = r.top - h - 8;
  el.style.left = Math.max(12, left) + "px";
  el.style.top = Math.max(12, top) + "px";
  el.addEventListener("mouseenter", () => clearTimeout(RR.hideTimer));
  el.addEventListener("mouseleave", () => { RR.hideTimer = setTimeout(rrHide, 160); });
  RR.el = el;
  // A page drawn again under a still mouse takes the range away without a
  // mouseout: the card goes with it.
  clearInterval(RR.watch);
  RR.watch = setInterval(() => { if (!RR.on || !RR.on.isConnected) rrHide(); }, 250);
}
async function rrOpen(t, delay) {
  if (prefs.hover === "off") return;
  clearTimeout(RR.timer);
  clearTimeout(RR.hideTimer);
  if (RR.el && RR.on !== t) { RR.el.remove(); RR.el = null; }
  RR.on = t;
  RR.timer = setTimeout(async () => {
    const d = t.dataset;
    const [kind, whose] = [d.rr.slice(0, 1), d.rr.slice(2)];
    const ck = `${S.status ? S.status.today : ""}|${d.rr}|${d.rrk}|${d.lo}|${d.hi}`;
    let x = RR.cache.get(ck);
    if (!x) {
      const args = { k: d.rrk, lo: Number(d.lo), hi: Number(d.hi) };
      try {
        x = kind === "s" ? await api("rr_staff", { ...args, who: whose }) : await api("rr_card", { ...args, p: Number(whose) });
      } catch (e) {
        x = { error: e.message || String(e) };
      }
      if (RR.cache.size > 400) RR.cache.clear();
      RR.cache.set(ck, x);
    }
    rrShow(t, x);
  }, delay);
}
document.addEventListener("mouseover", (ev) => {
  const t = ev.target.closest && ev.target.closest("[data-rr]");
  if (!t || t === RR.on) return;
  rrOpen(t, Math.min(450, Number(prefs.hoverDelay) || 350));
});
document.addEventListener("mouseout", (ev) => {
  const t = ev.target.closest && ev.target.closest("[data-rr]");
  if (!t || t !== RR.on) return;
  if (ev.relatedTarget && t.contains(ev.relatedTarget)) return;
  clearTimeout(RR.timer);
  RR.hideTimer = setTimeout(() => { if (RR.on === t) rrHide(); }, 160);
});
// The keyboard reaches a range too: tab onto it and the card opens.
document.addEventListener("focusin", (ev) => {
  const t = ev.target.closest && ev.target.closest("[data-rr]");
  if (t) rrOpen(t, 120);
});
document.addEventListener("focusout", (ev) => {
  const t = ev.target.closest && ev.target.closest("[data-rr]");
  if (t && t === RR.on) rrHide();
});
document.addEventListener("scroll", () => { if (RR.el) rrHide(); }, true);
document.addEventListener("mousedown", (ev) => { if (RR.el && !RR.el.contains(ev.target)) rrHide(); });

// ---------- your staff, Quality (board 17) ----------
function rrRow(r, on) {
  const cells = [0, 1].map((i) => {
    const f = r.reads[i];
    return f ? `<div class="rr-cell"><span class="rr-lab">${esc(f.label)}</span>${f.now ? rrBar(f, r.who, { on }) : `<span class="rr-ns" title="${esc(f.why || "")}">Not seen</span>`}</div>` : '<div class="rr-cell"></div>';
  }).join("");
  return `<div class="rr-row ${on ? "on" : ""}" data-act="rrpick" data-who="${r.who}"><div class="rr-who">${rrName(r)}<small>${esc(r.title)}</small></div><div class="rr-with"><b>${esc(r.with)}</b><small>${esc(r.since)}</small></div>${cells}<div class="rr-sure">${esc(r.sure || "—")}</div><div class="rr-pay"><b>${esc(r.c1)}</b><small>${esc(r.c2)}</small></div></div>`;
}
function rrDetail(d) {
  const last = d.history.length - 1;
  const hist = d.history.map((h, i) => `<div class="rr-hrow ${i === last ? "now" : ""}"><span>${esc(h.label)}</span>${rrBar({ key: "", moved: 0 }, null, { at: [h.lo, h.hi], start: null, on: i === last, noArrow: true })}</div>`).join("");
  const all = d.all.map((f) => `<div class="rr-arow"><span>${esc(f.label)}</span>${f.now ? rrBar(f, d.who) : `<span class="rr-why">not seen: ${esc(rrLower(f.why))}</span>`}</div>`).join("");
  const ev = d.evidence.map((e) => `<div class="rr-ev"><span class="st-label">${esc(e.k)}</span><span>${esc(e.t)}</span></div>`).join("");
  return `<div class="rr-dh"><div><b class="rr-dn">${rrName(d, "link")}</b><small>${esc(d.role)}</small></div><div class="rr-dc"><b>${esc(d.contract)}</b><small>${esc(d.hired)}</small></div></div>
    ${d.word ? `<div class="rr-big"><span class="st-label">Your read · ${esc(d.key)}</span><b class="rr-bw ${d.weak ? "weak" : ""}">${esc(d.word)}</b><small class="sub">${esc(d.sub)}</small>${d.history.length > 1 ? `<span class="st-label rr-mt">How your read narrowed</span>${hist}` : ""}</div>` : ""}
    <span class="st-label rr-mt">Every rating, your read</span><div class="rr-all">${all}</div>
    <span class="st-label rr-mt">Where your read comes from</span><div class="rr-evs">${ev}</div>
    <div class="rr-open">${d.coach != null ? `<a class="st-btn small glass" data-go="coach" data-id="${d.coach}" data-tab="Ratings">Open his ratings</a>` : linkTo("staff_person", { who: d.who }, "Open his page", "st-btn small glass")}</div>`;
}
async function rrQuality() {
  const x = await api("staff_quality");
  if (x.none) return '<div class="empty">You need a job first.</div>';
  const k = keep("rrq", { dept: null, who: null });
  const dept = x.depts.find((d) => d.name === k.dept) || x.depts[0];
  if (!dept.rows.some((r) => r.who === k.who)) k.who = dept.rows.length ? dept.rows[0].who : null;
  const seg = `<div class="seg rr-depts">${x.depts.map((d) => `<button data-act="rrdept" data-v="${esc(d.name)}" aria-pressed="${d === dept}">${esc(d.name)}${d.rows.length ? ` <small>${d.rows.length}</small>` : ""}</button>`).join("")}</div>`;
  const rows = dept.rows.length ? dept.rows.map((r) => rrRow(r, r.who === k.who)).join("") : `<div class="empty">${esc(dept.empty)}</div>`;
  let side = `<div class="empty">${esc(dept.empty)}</div>`;
  if (k.who) {
    try { side = rrDetail(await api("staff_member", { who: k.who })); } catch (e) { side = `<div class="empty">${esc(e.message)}</div>`; }
  }
  return `<div class="rr-q">
    <section class="panel rr-p"><div class="rr-ph"><div><b>Your read of them</b><span class="sub">from their work, on the hidden 1 to 20 scale</span></div>${seg}</div>
      <div class="rr-list"><div class="rr-lh"><span>Staff</span><span>With you</span><span>Main read</span><span>Second read</span><span>How sure</span><span>Contract</span></div>${rows}</div>
      <div class="rr-legend"><span><i class="st"></i>your read when they started</span><span><i class="now"></i>your read now</span><span><i class="weak"></i>a weak spot</span><span><em>↓</em> moved down this season</span><span class="rr-foot">${esc(dept.foot)}</span></div></section>
    <aside class="panel rr-p rr-side">${side}</aside></div>`;
}

// ---------- your staff, Jobs and hand-offs ----------
async function rrJobs() {
  const x = await api("staff_quality");
  if (x.none) return '<div class="empty">You need a job first.</div>';
  return `<div class="rr-jobs">${x.depts.map((d) => `<section class="panel rr-p"><div class="rr-ph"><div><b>${esc(d.name)}</b><span class="sub">${esc(d.foot)}</span></div></div>
    ${d.rows.length ? d.rows.map((r) => `<div class="rr-job"><div class="rr-who">${rrName(r)}<small>${esc(r.title)}${r.age ? ` · ${r.age}` : ""}</small></div><span>${esc(r.runs)}</span><small class="sub">${esc(r.hired)}</small></div>`).join("") : `<div class="empty">${esc(d.empty)}</div>`}</section>`).join("")}</div>`;
}
// The office's deals with your read beside them (Q5: your own assistants
// get a read, as the coaches you could hire do).
async function rrReads() {
  try {
    const x = await api("staff_quality");
    const m = new Map();
    for (const d of x.depts || []) for (const r of d.rows) if (r.coach != null) m.set(r.coach, r);
    return m;
  } catch (e) { return new Map(); }
}

// ---------- the Ratings page (board 18) ----------
// The record chart: each season's player residuals as dots, its mean as a
// line, players like them at 0, and the picked player in gold. The labels
// sit on a halo of the panel's colour so dots under them can't hide them.
function rrChart(rec, pick) {
  const S0 = rec.seasons;
  if (!S0.length) return "";
  const W = 400, H = 170, L = 26, B = 22;
  const vals = S0.flatMap((s) => s.players.map((p) => p.r)).concat(S0.map((s) => s.mean), [2, -2]);
  const top = Math.ceil(Math.max(...vals) / 2) * 2, bot = Math.floor(Math.min(...vals) / 2) * 2;
  const y = (v) => 8 + ((top - v) / (top - bot || 1)) * (H - B - 8);
  const bw = (W - L) / S0.length;
  const grid = [];
  for (let v = bot; v <= top; v += 2) grid.push(`<line x1="${L}" x2="${W}" y1="${y(v)}" y2="${y(v)}" class="${v === 0 ? "zero" : ""}"/><text x="${L - 6}" y="${y(v) + 3}" text-anchor="end">${v > 0 ? "+" + v : v}</text>`);
  const gold = [];
  const parts = S0.map((s, i) => {
    const x0 = L + i * bw + 10, x1 = L + (i + 1) * bw - 10;
    const dots = s.players.map((p, j) => {
      const cx = x0 + ((j * 37) % 100) / 100 * (x1 - x0);
      if (p.id === pick) gold.push([cx, y(p.r)]);
      const c = `<circle cx="${cx.toFixed(1)}" cy="${y(p.r).toFixed(1)}" r="3.2" class="${p.id === pick ? "pick" : p.r < -0.5 ? "low" : ""}"><title>${esc(p.name || "")}: ${rrSigned(p.r)}</title></circle>`;
      return p.id != null ? `<a data-go="player" data-id="${p.id}">${c}</a>` : c;
    }).join("");
    return `${dots}<line x1="${x0}" x2="${x1}" y1="${y(s.mean)}" y2="${y(s.mean)}" class="mean"/><text x="${x1}" y="${y(s.mean) - 5}" text-anchor="end" class="mv">${rrSigned(s.mean)}</text><text x="${(x0 + x1) / 2}" y="${H - 4}" text-anchor="middle" class="yr">${s.year}</text>`;
  }).join("");
  const path = gold.length > 1 ? `<polyline points="${gold.map((g) => g.map((v) => v.toFixed(1)).join(",")).join(" ")}" class="pick"/>` : "";
  return `<svg class="rr-chart" viewBox="0 0 ${W} ${H}" role="img" aria-label="Growth against players like them, by season">${grid.join("")}${path}${parts}</svg>`;
}
function rrRatingsHtml(x) {
  const k = keep("rrstep:" + x.who, { i: null, pick: null });
  const n = x.steps.length;
  const step = k.i == null || k.i >= n ? n - 1 : k.i;
  const stepper = `<div class="seg rr-steps">${x.steps.map((s, i) => `<button data-act="rrstep" data-who="${x.who}" data-i="${i}" aria-pressed="${i === step}">${esc(s)}</button>`).join("")}</div>`;
  const groups = [];
  for (const f of x.facets) {
    let g = groups.find((y) => y.name === f.group);
    if (!g) groups.push((g = { name: f.group, rows: [] }));
    g.rows.push(f);
  }
  const rows = groups.map((g) => `<div class="rr-grp st-label">${esc(g.name)}</div>${g.rows.map((f) => {
    const at = f.at[step];
    if (!at) return `<div class="rr-rrow ns"><span>${esc(f.name)}</span><span class="rr-b ns"><span class="rr-t"></span></span><b>—</b><span>Not seen</span><span class="rr-rest">${esc(f.why || f.rests)}</span></div>`;
    const bar = rrBar({ key: f.key, moved: step === n - 1 ? f.moved : 0 }, x.who, { at, start: step > 0 ? f.at[0] : null, on: true });
    return `<div class="rr-rrow"><span>${esc(f.name)}</span>${bar}<span class="rr-wd">${esc(rrWords(at[0], at[1]))}</span><span class="rr-rest">${esc(f.rests)}</span></div>`;
  }).join("")}`).join("");
  const rec = x.record;
  const pick = k.pick != null ? k.pick : x.moved.length ? x.moved[0].id : null;
  const moved = x.moved.length
    ? x.moved.map((m) => `<div class="rr-mv ${m.id === pick ? "on" : ""}" data-act="rrmv" data-who="${x.who}" data-id="${m.id}"><span>${playerLink(m.id, m.name)}<small>${esc(m.pos)} · ${esc(rrLower(m.class))}</small></span><span class="rr-mvr">${m.from != null ? `${m.from} → <b>${m.to ?? "—"}</b>` : ""}</span><span class="sub">${esc(m.then)} → ${esc(m.now)}</span></div>`).join("")
    : '<div class="empty">Nobody has moved up the depth chart under him yet.</div>';
  const record = rec.seasons.length
    ? `${rrChart(rec, pick)}<div class="rr-mean"><b>${rrSigned(rec.mean)}</b><span>${esc(rec.line)}</span></div>`
    : `<div class="rr-work">${x.facets.filter((f) => !f.why).map((f) => `<div class="rr-ev"><span class="st-label">${esc(f.name)}</span><span>${esc(f.rests)}</span></div>`).join("") || '<div class="empty">His first season with you: nothing measured yet.</div>'}</div>`;
  const o = x.outside;
  const tiles = [
    o ? [`Reputation`, o.reputation, "the word around the league"] : null,
    o ? [`Market`, o.market, o.market_sub] : null,
    o ? [`Wants next`, o.wants, o.wants_sub] : null,
    x.contract ? ["Contract", x.contract.line, x.contract.sub || ""] : null,
  ].filter(Boolean);
  const outside = `<section class="panel rr-p rr-out"><div class="rr-ph"><div><b>Outside your office</b><span class="sub">${o ? `what the rest of football sees, and what ${esc(x.name.split(" ")[0])} wants` : "who pays him, and his deal"}</span></div></div>
    <div class="rr-tiles">${tiles.map(([a, b, c]) => `<div><span class="st-label">${esc(a)}</span><b>${esc(b)}</b><small class="sub">${esc(c)}</small></div>`).join("")}</div>${x.summary ? `<p class="rr-sum">${esc(x.summary)}</p>` : ""}</section>`;
  return `<div class="rr-rt">
    <div class="rr-col"><section class="panel rr-p"><div class="rr-ph"><div><b>Your read</b><span class="sub">${esc(x.evidence_years)}, on the hidden 1 to 20 scale</span></div>${stepper}</div>
      <div class="rr-rh"><span>Rating</span><span class="rr-sc"><i>1</i><i>10</i><i>20</i></span><span>Range</span><span>In words</span><span>What it rests on</span></div>${rows}</section>
      ${outside}</div>
    <aside class="panel rr-p rr-side"><span class="st-label">${rec.seasons.length ? "Record with you" : "Work record"}</span>${rec.seasons.length ? `<p class="sub rr-cap">How much more his players grew than players like them (the line at 0), in rating points a year</p>` : `<p class="sub rr-cap">What his work has shown you so far</p>`}${record}
      ${x.coach != null ? `<span class="st-label rr-mt">Players moved up</span><div class="rr-mvs">${moved}</div>` : ""}
      ${x.before.length ? `<span class="st-label rr-mt">${esc(x.before_title)}</span><div class="rr-before">${x.before.map((b) => `<div><b>${esc(b.years)}</b><span>${esc(b.what)}</span></div>`).join("")}</div>` : ""}</aside></div>`;
}
// The coach page's Ratings tab, for your own staff.
async function rrCoachTab(id) {
  try {
    return `<div class="sl rr">${rrRatingsHtml(await api("coach_ratings", { id }))}</div>`;
  } catch (e) {
    return `<div class="empty">${esc(e.message)}</div>`;
  }
}
// A support staff member's page (Q4): the same reads and his work record.
screens.staff_person = async (a) => {
  const x = await api("coach_ratings", { who: a.who });
  title(x.name, "Staff");
  return `<div class="pad sl rr">${pageHead({ kicker: esc(x.kicker), title: esc(x.name), actions: linkTo("people_staff", { tab: "Quality" }, "Your staff", "st-btn small glass") })}${gotoFor("staff_person", a, x)}${rrRatingsHtml(x)}</div>`;
};
GOTO.staff_person = () => [["Your staff", "people_staff", { tab: "Quality" }], ["Jobs and hand-offs", "people_staff", { tab: "Jobs and hand-offs" }], ["People", "people"]];

Object.assign(CORE_MORE, {
  rrpick: (el) => { keep("rrq", {}).who = el.dataset.who; refresh(el); },
  rrdept: (el) => { const k = keep("rrq", {}); k.dept = el.dataset.v; k.who = null; refresh(el); },
  rrstep: (el) => { keep("rrstep:" + el.dataset.who, {}).i = Number(el.dataset.i); refresh(el); },
  rrmv: (el, ev) => { if (ev && ev.target.closest("[data-go]")) return; keep("rrstep:" + el.dataset.who, {}).pick = Number(el.dataset.id); refresh(el); },
});
