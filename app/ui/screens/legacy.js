// Lane screens: systems to chase (update 2, lane 2). Registers screens.<name>.
"use strict";
/* The chases' screens, boards 34-36, in Press Box:
 *
 *   legacy            Your legacy (34): your ambitions, your coaching tree,
 *                     the Hall of Fame race with the real greats, your
 *                     scheme's spread, your guys, your level's five.
 *   legacy_ambitions  What do you want here? (35): the ambitions that fit
 *                     you on taking a job, up to three, private or out loud.
 *   legacy_chases     Your level's five chases (36), each with its chart.
 *   Strips            each chase's own screen shows its line on top
 *                     ("lives on"), and Home shows your chases.
 */

I.laurel = '<path d="M12 21v-6"/><path d="M12 15c-3.5 0-6.5-2.6-7-7 2.6.2 4.6 1.3 5.8 3.2"/><path d="M12 15c3.5 0 6.5-2.6 7-7-2.6.2-4.6 1.3-5.8 3.2"/><path d="M12 12c-1.6-1.4-2.3-3.4-2-6 1.6.9 2.6 2.4 2.9 4.3"/><path d="M12 12c1.6-1.4 2.3-3.4 2-6-1.6.9-2.6 2.4-2.9 4.3"/><path d="M8.5 21h7"/>';

navAdd({ go: "legacy", label: "Your legacy", icon: "laurel", sec: "world", after: "chapters" });
Object.assign(CRUMBS, { legacy: "Your legacy", legacy_ambitions: "What do you want here?", legacy_chases: "Your chases" });

// ---------- small pieces ----------
const LG_TONE = ["lgy-good", "lgy-plain", "lgy-bad"];
const lgTone = (t) => LG_TONE[t] || "lgy-plain";
const lgEmpty = (t) => `<div class="empty">${esc(t)}</div>`;
const lgCrest = (t) => (t ? `<span class="lgy-crest">${mark(t, 40)}</span>` : "");
function lgHead(x, h1, act) {
  return `<header class="lgy-head">${lgCrest(x.team)}<div class="lgy-head-copy"><span class="st-label">${esc(x.ctx || "")}</span><h1 class="lgy-h1">${esc(h1)}</h1></div><div class="lgy-head-act">${act || ""}</div></header>`;
}
// A milestone ladder: five steps, the reached ones in gold.
function lgLadder(steps, pays) {
  return `<div class="lgy-lad">${steps.map((s, i) => `<div class="${s.done ? "done" : ""}"><i></i><span>${esc(s.n)}</span>${pays ? `<b>${esc(pays[i])}</b>` : ""}</div>`).join("")}</div>`;
}
const lgOther = (o) => `<div class="lgy-ot"><span class="st-label">${esc(o.who)}</span><span>${esc(o.text)}</span><span class="${o.warn ? "lgy-bad" : "lgy-dim"}">${esc(o.status)}</span></div>`;

// ---------- Your legacy (34) ----------
function lgTree(t) {
  if (!t) return lgEmpty("Your tree starts with the coaches who taught you and grows with every assistant you send on.");
  const W = 480, NW = 150;
  // Rows close up when a row is empty (no mentors in the world's books, no
  // second branch yet), so the tree sits at the top of its panel.
  const used = [t.mentors.length > 0, true, t.gen1.length > 0, t.gen2.length > 0];
  const pickRow = (list, n) => {
    const v = list.slice().sort((a, b) => (b.head - a.head) || (b.rival - a.rival) || (b.yours - a.yours));
    return { shown: v.slice(0, n), more: Math.max(0, v.length - n) };
  };
  const xs = (n) => (n === 1 ? [240] : n === 2 ? [150, 330] : [85, 240, 395]);
  const rows = [pickRow(t.mentors, 3), { shown: [t.you], more: 0 }, pickRow(t.gen1, 3), pickRow(t.gen2, 3)];
  // A row with one of your assistants carries his "Grow" choice, so it
  // takes more room before the next row (or the line under the tree).
  const tall = (row) => row >= 2 && rows[row].shown.some((n) => n.yours);
  let next = 0;
  const RY = used.map((u, row) => { const y = next; if (u) next += 78 + (tall(row) ? 26 : 0); return y; });
  const last = used.lastIndexOf(true);
  const H = RY[last] + 56 + (tall(last) ? 26 : 0);
  const at = new Map();
  const nodes = [];
  rows.forEach((r, row) => {
    const X = xs(r.shown.length);
    r.shown.forEach((n, k) => {
      const kind = row === 0 ? "mentor" : row === 1 ? "you" : n.rival ? "rival" : row === 2 ? "branch" : "leaf";
      const pos = { x: X[k], y: RY[row], row };
      if (n.coach != null) at.set(n.coach, pos);
      nodes.push({ n, kind, pos });
    });
  });
  const youPos = { x: 240, y: RY[1] };
  const links = [];
  nodes.forEach(({ n, kind, pos }) => {
    let from = null;
    if (pos.row === 1) {
      nodes.filter((m) => m.pos.row === 0).forEach((m) => links.push({ a: m.pos, b: pos, mentor: true }));
      return;
    }
    if (pos.row === 2) from = youPos;
    if (pos.row === 3) from = n.parent != null && at.get(n.parent) ? at.get(n.parent) : null;
    if (from) links.push({ a: from, b: pos, mentor: false });
    void kind;
  });
  const path = (l) => {
    const y0 = l.a.y + 48, y1 = l.b.y, ym = (y0 + y1) / 2;
    return `<path class="${l.mentor ? "m" : ""}" d="M${l.a.x} ${y0} V${ym} H${l.b.x} V${y1}"></path>`;
  };
  const box = ({ n, kind, pos }) => {
    const left = ((pos.x - NW / 2) / W) * 100;
    const nm = n.coach != null ? coachLink(n.coach, n.name) : esc(n.name);
    const grow = n.yours && kind !== "you" && kind !== "mentor"
      ? `<select class="lgy-grow" data-lgchange="lggrow" data-coach="${n.coach}" title="How you grow him"><option value="">Grow: his own way</option>${[["calls", "calls his side's plays"], ["recruiting", "a recruiting area"], ["clinics", "clinics and visits"]].map(([v, l]) => `<option value="${v}" ${n.grow === v ? "selected" : ""}>${l}</option>`).join("")}</select>`
      : "";
    return `<div class="lgy-tn ${kind}" style="left:${left.toFixed(2)}%;top:${pos.y}px"><b>${nm}</b><span>${esc(n.line)}</span>${grow}</div>`;
  };
  const more = [rows[2].more ? `${rows[2].more} more who learned under you` : "", rows[3].more ? `${rows[3].more} more in the next branch` : ""].filter(Boolean);
  return `<div class="lgy-tree" style="height:${H}px"><svg viewBox="0 0 ${W} ${H}" preserveAspectRatio="none" role="img" aria-label="Your coaching tree: who taught you, and the coaches you grew">${links.map(path).join("")}</svg>${nodes.map(box).join("")}</div>
    ${more.length ? `<span class="sub small">${esc(more.join(" · "))}</span>` : ""}
    <div class="lgy-tiles">${t.tiles.map(([v, k]) => `<div class="lgy-tile"><b>${esc(v)}</b><span>${esc(k)}</span></div>`).join("")}</div>`;
}

function lgHall(h) {
  if (!h) return lgEmpty("The Hall of Fame race starts with your first season as a head coach.");
  const bars = h.bars.map((b) => {
    // A count the hall's line doesn't ask for (a hall of honor that only
    // counts years) shows its number without a bar to fill.
    if (!(b.need > 0)) return `<div class="lgy-hb"><span>${esc(b.k)}</span><span class="lgy-dim small">not on this hall's line</span><b>${esc(b.text)}</b></div>`;
    const max = Math.max(b.need, b.have) * 1.15 || 1;
    const w = Math.min(100, (b.have / max) * 100), line = Math.min(100, (b.need / max) * 100);
    return `<div class="lgy-hb"><span>${esc(b.k)}</span><div class="lgy-hbar"><i class="${b.have >= b.need ? "met" : ""}" style="width:${w.toFixed(1)}%"></i><em style="left:${line.toFixed(1)}%"></em></div><b>${esc(b.text)}</b></div>`;
  }).join("");
  return `<div class="lgy-hall"><span class="st-label">${esc(h.hall)}</span>${h.note ? `<span class="sub small">${esc(h.note)}</span>` : ""}</div>
    <div class="lgy-hbs">${bars}</div>
    <span class="lgy-gold small">${esc(h.moves)}</span>
    ${h.listK ? `<div class="lgy-kv"><span>${esc(h.listK)}</span><b>${esc(h.listV)}</b></div>` : ""}
    <span class="st-label lgy-pad">Real greats, for reference</span>
    <div class="lgy-greats">${h.greats.map(([n, v]) => `<div><span>${esc(n)}</span><span class="lgy-dim">${esc(v)}</span></div>`).join("")}</div>`;
}

function lgSchemeBars(years) {
  if (!years.length) return "";
  const W = 408, H = 132, base = 112, n = years.length, cw = 400 / n, bw = Math.min(34, cw * 0.7);
  const mx = Math.max(1, ...years.map(([, a, b, c]) => a + b + c)), sc = 92 / mx;
  const g = years.map(([yr, a, b, c], i) => {
    const x = 8 + i * cw + (cw - bw) / 2, h0 = a * sc, h1 = b * sc, h2 = c * sc, tot = a + b + c;
    return `<g><rect class="lgy-shs" x="${x.toFixed(1)}" y="${(base - h0).toFixed(1)}" width="${bw}" height="${h0.toFixed(1)}" rx="3"></rect><rect class="lgy-scol" x="${x.toFixed(1)}" y="${(base - h0 - h1).toFixed(1)}" width="${bw}" height="${h1.toFixed(1)}" rx="3"></rect><rect class="lgy-snfl" x="${x.toFixed(1)}" y="${(base - h0 - h1 - h2).toFixed(1)}" width="${bw}" height="${h2.toFixed(1)}" rx="3"></rect><text x="${(x + bw / 2).toFixed(1)}" y="${(base - h0 - h1 - h2 - 5).toFixed(1)}" text-anchor="middle" class="t">${tot || "·"}</text><text x="${(x + bw / 2).toFixed(1)}" y="128" text-anchor="middle" class="y">${yr}</text></g>`;
  }).join("");
  return `<svg class="lgy-sbars" viewBox="0 0 ${W} ${H}" role="img" aria-label="Teams running your system each year, by level">${g}</svg>
    <div class="lgy-legend"><span><i class="lgy-shs"></i>High school</span><span><i class="lgy-scol"></i>College</span><span><i class="lgy-snfl"></i>NFL</span></div>`;
}

function lgScheme(s) {
  return `<div class="lgy-sname"><b>${esc(s.name)}</b><span class="sub small">${esc(s.named ? (s.system || "") + " · " + s.who : s.who)}</span></div>
    ${s.named ? lgSchemeBars(s.years) : `<span class="sub small">${s.now ? `You run ${esc(s.now)} now.` : "Your offense and defense show here once your team plays."}</span>`}
    ${s.via.length ? `<div class="lgy-chips">${s.via.map((v) => `<span class="lgy-chip">${esc(v)}</span>`).join("")}</div>` : ""}
    ${s.film ? `<span class="sub small lgy-foot">${esc(s.film)}</span>` : ""}`;
}

function lgGuys(x) {
  const rows = x.guys.length
    ? x.guys.map((g) => `<div class="lgy-gy"><span class="lgy-gn"><b>${playerLink(g.id, g.name)}</b><small>${esc(g.line)}</small></span><span>${esc(g.now)}</span><span class="lgy-dim">${esc(g.next)}</span><span class="${lgTone(g.tone)} lgy-right"><b>${esc(g.bond)}</b></span></div>`).join("")
    : lgEmpty("Every player you coach stays one of your guys: where they go, what they become, how they feel about you.");
  return `<div class="lgy-tiles">${x.gStats.map(([v, k]) => `<div class="lgy-tile"><b>${esc(v)}</b><span>${esc(k)}</span></div>`).join("")}</div>
    <div class="lgy-gy lgy-gh"><span class="st-label">Player</span><span class="st-label">Now</span><span class="st-label">Next</span><span class="st-label lgy-right">Bond</span></div>${rows}`;
}

function lgFive(x) {
  if (!x.five.length) return lgEmpty("Your level's chases come with a team.");
  return x.five.map((f) => `<div class="lgy-l5" data-go="legacy_chases" data-args='${esc(JSON.stringify({ code: f.code }))}' role="link" tabindex="0"><span class="lgy-code">${esc(f.code)}</span><span class="lgy-l5n"><b>${esc(f.name)}</b><small>${esc(f.line)}</small></span><span class="${lgTone(f.tone)} lgy-right"><b>${esc(f.word)}</b></span></div>`).join("");
}

screens.legacy = async () => {
  const x = await api("legacy_page");
  title("Your legacy");
  if (!x.team) return `<div class="pad sl lg">${lgHead(x, "Your legacy")}<div class="empty">${esc(x.ctx)}</div></div>`;
  const ams = x.ambitions.length
    ? x.ambitions.map((a) => `<div class="lgy-am"><div class="lgy-amt"><b>${esc(a.title)}</b><span class="lgy-tags"><span class="lgy-chip">${esc(a.group)}</span><span class="lgy-chip ${a.loud ? "lgy-gold" : ""}">${a.loud ? "Said out loud" : "Private"}</span></span></div>${lgLadder(a.steps)}<span class="lgy-gold small">${esc(a.next)}</span></div>`).join("")
    : lgEmpty(x.picker ? "Your new job's ambitions wait for you." : "No ambitions of your own right now. New ones come with a new job or when your life changes.");
  const pick = x.picker ? linkTo("legacy_ambitions", {}, "Pick your ambitions", "st-btn primary small") : "";
  const tally = [x.reached ? `${x.reached} reached` : "", x.closed ? `${x.closed} closed` : ""].filter(Boolean).join(", ");
  const act = `${pick}${linkTo("legacy_chases", {}, "Your chases", "st-btn glass small")}`;
  return `<div class="pad sl lg">
    ${lgHead(x, "Your legacy", act)}${gotoFor("legacy")}
    <div class="lgy-grid">
      <section class="panel lgy-p"><div class="lgy-ph"><b>Ambitions</b><span class="sub">${x.ambitions.length} of 3 of your own${tally ? " · " + esc(tally) : ""}</span></div>
        ${ams}
        <div class="lgy-others">${x.others.map(lgOther).join("")}</div>
      </section>
      <section class="panel lgy-p"><div class="lgy-ph"><b>Your coaching tree</b><span class="sub">who taught you, and who you grew</span></div>${lgTree(x.tree)}</section>
      <section class="panel lgy-p"><div class="lgy-ph"><b>Hall of Fame race</b>${x.hall ? `<span class="lgy-word">${esc(x.hall.word)}</span>` : ""}</div>${lgHall(x.hall)}</section>
      <section class="panel lgy-p"><div class="lgy-ph"><b>Your scheme</b><span class="sub">${esc(x.scheme.since)}</span></div>${lgScheme(x.scheme)}</section>
      <section class="panel lgy-p"><div class="lgy-ph"><b>Your guys</b><span class="sub">${esc(x.guysSub)}</span></div>${lgGuys(x)}</section>
      <section class="panel lgy-p"><div class="lgy-ph"><b>${esc(x.fiveT || "Your chases")}</b>${linkTo("legacy_chases", {}, "each has its own panel", "link sub")}</div>${lgFive(x)}</section>
    </div>
    ${x.log.length ? `<section class="panel lgy-p lgy-log"><div class="lgy-ph"><b>What your chases paid</b><span class="sub">lesson points go into the rating each one used; reputation is of 20</span></div>${x.log.map((p) => `<div class="lgy-lg"><span class="lgy-dim">${esc(p.date)}</span><span class="lgy-code">${esc(p.sys)}</span><span>${esc(p.what)}</span><span class="${p.lesson > 0 || p.rep > 0 ? "lgy-good" : "lgy-bad"} lgy-right">${p.lesson > 0 ? `+${p.lesson.toFixed(2)} ${esc(p.into)}` : ""}${p.rep ? ` ${p.rep > 0 ? "+" : "−"}${Math.abs(p.rep).toFixed(2)} rep` : ""}</span></div>`).join("")}</section>` : ""}
  </div>`;
};
GOTO.legacy = () => [["Your chases", "legacy_chases"], ["Career story", "story"], ["Season chapters", "chapters"], ["Your profile", "you"], ["Staff", "staff"]];

// ---------- What do you want here? (35) ----------
screens.legacy_ambitions = async () => {
  const x = await api("legacy_ambitions");
  title("What do you want here?");
  if (!x.open) {
    const held = x.held.length
      ? x.held.map((a) => `<div class="lgy-am"><div class="lgy-amt"><b>${esc(a.title)}</b><span class="lgy-tags"><span class="lgy-chip">${esc(a.group)}</span><span class="lgy-chip ${a.loud ? "lgy-gold" : ""}">${a.loud ? "Said out loud" : "Private"}</span></span></div>${lgLadder(a.steps)}<span class="lgy-gold small">${esc(a.next)}</span></div>`).join("")
      : lgEmpty("No ambitions of your own right now.");
    return `<div class="pad sl lg">${lgHead({ ctx: "Your ambitions" }, "What do you want here?", linkTo("legacy", {}, "Your legacy", "st-btn glass small"))}
      <div class="lgy-pick"><section class="panel lgy-p"><div class="lgy-ph"><b>Yours</b><span class="sub">new ones come with a new job, or when your life changes</span></div>${held}</section>
      <aside class="panel lgy-p"><span class="st-label">On your desk too</span><div class="lgy-others">${x.others.map(lgOther).join("")}</div><span class="sub small">These come to you. They don't use your three.</span></aside></div></div>`;
  }
  const room = x.max - x.held.length;
  const picked = x.offers.filter((o) => o.picked);
  const full = picked.length >= room;
  const offers = x.offers.map((o) => {
    const pays = o.steps.map((s) => "+" + (o.loud ? s.pl : s.p).toFixed(2));
    const btn = o.picked ? `<button class="lgy-pk on" data-act="lgpick" data-id="${o.id}" data-on="0">Picked ✓</button>` : `<button class="lgy-pk" data-act="lgpick" data-id="${o.id}" data-on="1" ${full ? "disabled" : ""}>${full ? "Three picked" : "Pick"}</button>`;
    return `<article class="lgy-ac ${o.picked ? "on" : ""}"><div class="lgy-act"><span class="lgy-acn"><span class="st-label">${esc(o.group)}</span><b>${esc(o.title)}</b></span>${btn}</div><span class="sub small">${esc(o.why)}</span>${lgLadder(o.steps, pays)}<span class="lgy-faint small">${esc(o.closes)} · pays into ${esc(o.into)}</span></article>`;
  }).join("");
  const slots = [];
  x.held.forEach((a) => slots.push(`<div class="lgy-sl held"><div class="lgy-amt"><b>${esc(a.title)}</b><span class="lgy-chip">${a.loud ? "Said out loud" : "Private"}</span></div><span class="sub small">Already yours: it counts toward your three.</span></div>`));
  picked.forEach((o) => slots.push(`<div class="lgy-sl"><div class="lgy-amt"><b>${esc(o.title)}</b><div class="seg lgy-seg"><button data-act="lgloud" data-id="${o.id}" data-loud="0" aria-pressed="${!o.loud}">Private</button><button data-act="lgloud" data-id="${o.id}" data-loud="1" aria-pressed="${o.loud}">Out loud</button></div></div><span class="small ${o.loud ? "lgy-gold" : "lgy-dim"}">${o.loud ? "Out loud: every step pays more, the press and fans hold you to it, and it becomes a story. If it closes, it costs reputation." : "Private: safer and pays less. If it closes, it just ends."}</span></div>`));
  while (slots.length < x.max) slots.push('<div class="lgy-sl open"><b>An open slot</b><span class="sub small">Pick one on the left, or leave it open: new ones come when your life changes.</span></div>');
  const quotes = picked.filter((o) => o.loud).map((o) => `<span class="lgy-q">"${esc(o.quote)}"</span>`).join("");
  return `<div class="pad sl lg">
    ${lgHead(x, "What do you want here?")}
    <div class="lgy-pick">
      <section class="panel lgy-p"><div class="lgy-ph"><b>Ambitions that fit you</b><span class="sub">${esc(x.start)}</span></div><div class="lgy-offers">${offers}</div></section>
      <aside class="panel lgy-p">
        <div class="lgy-ph"><b>Your three</b><span class="sub">${picked.length + x.held.length} of ${x.max} picked</span></div>
        ${slots.join("")}
        <span class="st-label lgy-pad">On your desk too</span>
        <div class="lgy-others">${x.others.map(lgOther).join("")}</div>
        <span class="sub small">These come to you. They don't use your three.</span>
        <div class="lgy-card"><span class="st-label">How ambitions pay</span><span class="small">Each step pays 0.25 to 1.00 lesson points by how hard it is, into the ratings it used, plus reputation. A harder start pays more, and so does saying it out loud.</span><span class="small">Some close for good: a rival retires, a job goes to someone else. New ones come when your life changes.</span></div>
        <div class="lgy-card"><span class="st-label lgy-gold">${esc(x.introT)}</span>${quotes || '<span class="sub small">Nothing said out loud. Your goals stay between you and your staff.</span>'}<span class="sub small">${esc(x.introW)}</span></div>
        <div class="lgy-btns"><button class="st-btn primary" data-act="lgkeep" data-later="0">Keep these</button><button class="st-btn glass" data-act="lgkeep" data-later="1" ${x.later ? `disabled title="Put off until ${esc(x.later)}"` : ""}>Decide later</button></div>
      </aside>
    </div></div>`;
};
GOTO.legacy_ambitions = () => [["Your legacy", "legacy"], ["Your profile", "you"]];

// ---------- the five chases (36) ----------
function lgBars(v, W, H) {
  const n = v.data.length;
  if (!n) return "";
  const cw = W / n, bw = Math.min(40, cw * 0.62);
  const mx = Math.max(1e-9, ...v.data.map(([, vals]) => vals.reduce((a, b) => a + b, 0)));
  const cls = ["s0", "s1", "s2", "s3", "s4"];
  let rects = "", tops = "", xl = "";
  v.data.forEach(([lab, vals], k) => {
    const x = k * cw + (cw - bw) / 2;
    let y = H - 4;
    vals.forEach((val, s) => {
      const h = (val / mx) * (H - 28);
      if (h > 0) rects += `<rect class="${cls[s] || "s4"}" x="${x.toFixed(1)}" y="${(y - h).toFixed(1)}" width="${bw.toFixed(1)}" height="${Math.max(h - 1, 1).toFixed(1)}" rx="2"></rect>`;
      y -= h;
    });
    const tot = vals.reduce((a, b) => a + b, 0);
    const top = v.tops ? v.tops[k] : String(Math.round(tot * 10) / 10);
    tops += `<text class="t" x="${(x + bw / 2).toFixed(1)}" y="${(y - 6).toFixed(1)}" text-anchor="middle">${esc(top)}</text>`;
    xl += `<text class="x" x="${(x + bw / 2).toFixed(1)}" y="${H + 12}" text-anchor="middle">${esc(lab)}</text>`;
  });
  const legend = v.names.length > 1 ? `<div class="lgy-legend">${v.names.map((nm, s) => `<span><i class="${cls[s] || "s4"}"></i>${esc(nm)}</span>`).join("")}</div>` : "";
  return `<svg class="lgy-viz" viewBox="0 -4 ${W} ${H + 18}" role="img" aria-label="${esc(v.names.join(", "))}">${rects}${tops}${xl}</svg>${legend}`;
}
function lgRows(v) {
  return `<div class="lgy-rows">${v.rows.map(([n, val, t, tone]) => `<div class="lgy-rw"><span class="${tone === 0 && v.max < 20 ? "lgy-gold" : ""}">${esc(n)}</span><div class="lgy-rbar"><i class="${lgTone(tone)}" style="width:${Math.max((val / (v.max || 1)) * 100, val > 0 ? 3 : 0).toFixed(1)}%"></i></div><span class="${tone === 2 ? "lgy-bad" : "lgy-dim"}">${esc(t)}</span></div>`).join("")}</div>`;
}
function lgLine(v, W, H) {
  const n = v.pts.length;
  if (!n) return "";
  const X = (k) => (n === 1 ? W / 2 : 24 + (k * (W - 48)) / (n - 1));
  const Y = (val) => { const f = (val - v.lo) / ((v.hi - v.lo) || 1); return 14 + (v.invert ? f : 1 - f) * (H - 40); };
  const xy = v.pts.map(([, val], k) => [X(k), Y(val)]);
  const line = xy.map(([x, y]) => `${x.toFixed(1)},${y.toFixed(1)}`).join(" ");
  const area = `${X(0).toFixed(1)},${H - 4} ${line} ${X(n - 1).toFixed(1)},${H - 4}`;
  const grid = [0.25, 0.5, 0.75].map((f) => `<line class="g" x1="0" x2="${W}" y1="${(14 + f * (H - 40)).toFixed(1)}" y2="${(14 + f * (H - 40)).toFixed(1)}"></line>`).join("");
  const dots = xy.map(([x, y]) => `<circle cx="${x.toFixed(1)}" cy="${y.toFixed(1)}" r="3.5"></circle>`).join("");
  const vals = xy.map(([x, y], k) => `<text class="t" x="${x.toFixed(1)}" y="${(y < 30 ? y + 16 : y - 8).toFixed(1)}" text-anchor="middle">${esc(String(v.pts[k][1]))}</text>`).join("");
  const marks = v.marks.map(([k, t]) => xy[k] ? `<text class="m" x="${Math.min(Math.max(xy[k][0], 50), W - 50).toFixed(1)}" y="${(xy[k][1] < H / 2 ? xy[k][1] + 30 : xy[k][1] - 22).toFixed(1)}" text-anchor="middle">${esc(t)}</text>` : "").join("");
  const xl = v.pts.map(([t], k) => `<text class="x" x="${X(k).toFixed(1)}" y="${H + 12}" text-anchor="middle">${esc(t)}</text>`).join("");
  return `<svg class="lgy-viz" viewBox="0 0 ${W} ${H + 16}" role="img" aria-label="${esc(v.note)}">${grid}<polygon class="a" points="${area}"></polygon><polyline points="${line}"></polyline>${dots}${vals}${marks}${xl}</svg><span class="sub small">${esc(v.note)}</span>`;
}
function lgViz(v, wide) {
  const W = wide ? 644 : 412, H = wide ? 170 : 140;
  if (v.kind === "bars") return lgBars(v, W, H);
  if (v.kind === "rows") return lgRows(v);
  if (v.kind === "line") return lgLine(v, W, H);
  return lgEmpty(v.text || "Nothing to draw yet.");
}
// The controls a chase has: the window's call, the collective's split, the feeders' help.
function lgControls(p, x) {
  const e = x.extra || {};
  if (p.code === "L1" && e.window != null) {
    return `<div class="lgy-ctl"><span class="st-label">This league year</span><div class="seg lgy-seg">${["Hold", "Push in", "Reload"].map((t, k) => `<button data-act="lgwindow" data-v="${k}" aria-pressed="${e.window === k}">${t}</button>`).join("")}</div></div>`;
  }
  if (p.code === "L9" && e.split) {
    const tot = e.split.reduce((a, b) => a + b, 0) || 1;
    const now = e.split.map((s) => Math.round((s / tot) * 100));
    const presets = [["Keep your stars", [0.6, 0.2, 0.2]], ["Balanced", [0.4, 0.3, 0.3]], ["Transfers", [0.3, 0.5, 0.2]], ["The new class", [0.3, 0.2, 0.5]]];
    const on = (s) => s.every((v, i) => Math.abs(v * 100 - now[i]) < 2);
    return `<div class="lgy-ctl"><span class="st-label">How the collective spends</span><div class="seg lgy-seg">${presets.map(([t, s]) => `<button data-act="lgsplit" data-s="${s.join(",")}" aria-pressed="${on(s)}">${t}</button>`).join("")}</div></div>`;
  }
  if (p.code === "L14" && e.feeders && e.feeders.length) {
    const opts = [["playbook", "Your playbook"], ["clinic", "A clinic night"], ["assistant", "An assistant on its staff"], ["equipment", "Equipment ($2,500)"]];
    return `<div class="lgy-ctl lgy-feed">${e.feeders.map((f) => {
      const left = opts.filter(([k]) => !f[k]);
      return `<div class="lgy-fd"><span><b>${esc(f.name)}</b><small>${esc(f.coach)}</small></span>${left.length ? `<select data-lgchange="lgfeeder" data-name="${esc(f.name)}"><option value="">Give…</option>${left.map(([k, t]) => `<option value="${k}">${esc(t)}</option>`).join("")}</select>` : '<span class="lgy-good small">All of it</span>'}</div>`;
    }).join("")}<span class="sub small">The booster club has ${esc(Math.round(e.boosters || 0).toLocaleString("en-US", { style: "currency", currency: "USD", maximumFractionDigits: 0 }))}.</span></div>`;
  }
  return "";
}
function lgPanel(p, k, x) {
  const wide = k >= 3;
  return `<section class="panel lgy-cp ${wide ? "wide" : ""}" id="lgy-${esc(p.code)}">
    <div class="lgy-cph"><span class="lgy-cpn"><span class="lgy-code">${esc(p.code)}</span><b>${esc(p.name)}</b></span><span class="${lgTone(p.tone)} lgy-word">${esc(p.word)}</span></div>
    <span class="sub small lgy-home">Lives on ${linkTo(p.screen, {}, esc(p.home))}</span>
    ${lgViz(p.vis, wide)}
    <div class="lgy-kvs">${p.kv.map((r) => `<div class="lgy-kv2"><span class="st-label">${esc(r.k)}</span><span class="${r.tone === 2 ? "lgy-bad" : r.tone === 0 ? "" : "lgy-dim"}">${esc(r.v)}</span></div>`).join("")}</div>
    ${lgControls(p, x)}
  </section>`;
}
screens.legacy_chases = async (a) => {
  const x = await api("legacy_chases");
  title(x.title || "Your chases");
  if (!x.panels.length) return `<div class="pad sl lg">${lgHead(x, x.title || "Your chases")}<div class="empty">${esc(x.ctx)}</div></div>`;
  const html = `<div class="pad sl lg">
    ${lgHead({ team: x.team, ctx: x.ctx + " · each chase also lives on its own screen" }, x.title, linkTo("legacy", {}, "Your legacy", "st-btn glass small"))}${gotoFor("legacy_chases")}
    <div class="lgy-five">${x.panels.map((p, k) => lgPanel(p, k, x)).join("")}</div></div>`;
  if (a && a.code) setTimeout(() => { const el = document.getElementById("lgy-" + a.code); if (el) el.scrollIntoView({ block: "center" }); }, 30);
  return html;
};
GOTO.legacy_chases = () => [["Your legacy", "legacy"], ["Career story", "story"], ["Season chapters", "chapters"]];

// ---------- each chase on its own screen, and Home ----------
const LG_HOMES = { fo_cap: ["L1"], depth: ["L2"], draftrecord: ["L3"], fo_office: ["L4"], standings: ["L5"], recruit_board: ["L6"], team: ["L7"], traditions: ["L8"], nil_desk: ["L9"], schedule: ["L10"], school: ["L11"], history: ["L12"], hsside: ["L13"], area: ["L14"], market: ["L15"] };
function lgStrip(rows) {
  return `<div class="sl lg lgy-strip">${rows.map((r) => `<div class="lgy-sr" data-go="legacy_chases" data-args='${esc(JSON.stringify({ code: r.code }))}' role="link" tabindex="0"><span class="lgy-code">${esc(r.code)}</span><b>${esc(r.name)}</b><span class="${lgTone(r.tone)}"><b>${esc(r.word)}</b></span><span class="lgy-dim">${esc(r.line)}</span><span class="lgy-go">Your chases ›</span></div>`).join("")}</div>`;
}
const lgInsert = (html, piece) => {
  const m = /<div class="pad[^"]*">/.exec(html);
  return m ? html.slice(0, m.index + m[0].length) + piece + html.slice(m.index + m[0].length) : piece + html;
};
for (const [scr, codes] of Object.entries(LG_HOMES)) {
  const base = screens[scr];
  if (!base) continue;
  screens[scr] = async (a) => {
    const html = await base(a);
    // Another team's page shows its own: the strip is for your team's screens.
    if (a && a.kind && S.status && S.status.mine && S.status.mine.ref && (a.kind !== S.status.mine.ref.kind || Number(a.id) !== Number(S.status.mine.ref.id))) return html;
    if (typeof html !== "string") return html;
    let rows = [];
    try { rows = (await api("legacy_strip", { codes })).rows; } catch (e) { return html; }
    return rows.length ? lgInsert(html, lgStrip(rows)) : html;
  };
}
{
  const base = screens.home;
  screens.home = async (a) => {
    const html = await base(a);
    if (typeof html !== "string" || html.indexOf('class="home-grid"') < 0) return html;
    let x;
    try { x = await api("legacy_strip", {}); } catch (e) { return html; }
    if (!x.rows.length && !x.ambitions.length) return html;
    const ams = x.ambitions.length
      ? x.ambitions.map((m) => `<div class="lgy-ha"><b>${esc(m.title)}</b><span class="lgy-dim">${m.done} of ${m.of} · next: ${esc(m.next)}</span></div>`).join("")
      : `<span class="sub small">${x.picker ? linkTo("legacy_ambitions", {}, "Pick your ambitions for this job") : "No ambitions of your own right now."}</span>`;
    const five = x.rows.map((r) => `<div class="lgy-hf" data-go="legacy_chases" data-args='${esc(JSON.stringify({ code: r.code }))}' role="link" tabindex="0"><span class="lgy-code">${esc(r.code)}</span><span>${esc(r.name)}</span><span class="${lgTone(r.tone)} lgy-right"><b>${esc(r.word)}</b></span></div>`).join("");
    const panel = `<div class="panel sl lg lgy-home"><h2>Your chases <small>${linkTo("legacy", {}, "your legacy")}</small></h2>${ams}<div class="lgy-hfs">${five}</div></div>`;
    const at = html.indexOf('<div class="col">', html.indexOf('<div class="col">') + 1);
    if (at < 0) return html;
    const after = at + '<div class="col">'.length;
    return html.slice(0, after) + panel + html.slice(after);
  };
}

// ---------- clicks ----------
Object.assign(CORE_MORE, {
  lgpick: async (el) => {
    try { const r = await api("legacy_pick", { id: Number(el.dataset.id), on: el.dataset.on === "1" }); toast(r.said); refresh(el); } catch (e) { toast(e.message); }
  },
  lgloud: async (el) => {
    try { const r = await api("legacy_loud", { id: Number(el.dataset.id), loud: el.dataset.loud === "1" }); toast(r.said); refresh(el); } catch (e) { toast(e.message); }
  },
  lgkeep: async (el) => {
    const later = el.dataset.later === "1";
    if (!later) {
      const ok = await sure("Keep these ambitions?", "Once they're yours, the words are out: an ambition said out loud can't be taken back.", "Keep them");
      if (!ok) return;
    }
    try { const r = await api("legacy_keep", { later }); toast(r.said); go("legacy", {}); } catch (e) { toast(e.message); }
  },
  lgwindow: async (el) => {
    try { await api("legacy_window", { choice: Number(el.dataset.v) }); refresh(el); } catch (e) { toast(e.message); }
  },
  lgsplit: async (el) => {
    const [stars, transfers, cls] = el.dataset.s.split(",").map(Number);
    try { await api("legacy_split", { stars, transfers, class: cls }); refresh(el); } catch (e) { toast(e.message); }
  },
});
// The selects act on change, not on the click that opens them.
const LG_CHANGE = {
  lggrow: async (el) => {
    try { await api("legacy_grow", { coach: Number(el.dataset.coach), grow: el.value }); toast(el.value ? "His growth plan is set." : "He grows his own way."); refresh(el); } catch (e) { toast(e.message); }
  },
  lgfeeder: async (el) => {
    if (!el.value) return;
    try { const r = await api("legacy_feeder", { name: el.dataset.name, what: el.value }); toast(r.said); refresh(el); } catch (e) { toast(e.message); }
  },
};
document.addEventListener("change", (ev) => {
  const el = ev.target;
  const f = el && el.dataset && LG_CHANGE[el.dataset.lgchange];
  if (f) f(el);
});
