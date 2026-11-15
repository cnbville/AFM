// Hand-off projections (Cain 2026-11-15): before you hand a meeting to your
// staff, both sides of the choice, from the same code that does it
// (`world/folk/handoff.rs`). The button names what happens; the sheet shows
// what you give up and what you get, a man's result as the range of your
// read of him, and a fixed standard or a skip in plain words. Also the
// holiday preview (the meetings that fall inside, each with its line, and
// the ones you pick to stop for) and the standing duties' next two weeks.
"use strict";

const HO_CLS = { man: "man", standard: "std", skip: "skip" };
const hoUp = (x) => String(x || "").replace(/^./, (c) => c.toUpperCase());

// One bar: solid is the likely move, faint how far it can go. A centered
// bar runs from the tick (no change) left for down, right for up.
function hoBar(b) {
  const seg = (from, to, cls) => {
    const l = Math.min(from, to), r = Math.max(from, to);
    return r - l < 0.4 ? "" : `<i class="${cls}" style="left:${l}%;width:${r - l}%"></i>`;
  };
  if (b.centered) {
    const at = (x) => 50 + Math.max(-50, Math.min(50, (x / b.top) * 50));
    const s = at(b.solid), f = at(b.faint);
    const sign = (x) => (x < 0 ? "neg" : "pos");
    const dir = b.faint === 0 && b.solid === 0 ? "none" : sign(b.faint || b.solid);
    return `<div class="ho-bar"><span>${esc(b.name)}</span><span class="ho-track">${seg(s, f, `faint ${dir}`)}${seg(50, s, `solid ${dir}`)}<b class="tick"></b></span></div>`;
  }
  const at = (x) => Math.max(0, Math.min(100, (x / b.top) * 100));
  const s = at(b.solid), f = at(b.faint);
  return `<div class="ho-bar"><span>${esc(b.name)}</span><span class="ho-track">${seg(s, f, "faint pos")}${seg(0, s, "solid pos")}</span></div>`;
}

function hoRead(r) {
  const pos = (x) => ((x - 1) / 19) * 100;
  const val = r.yours || r.lo === r.hi ? `${r.lo}` : `${r.lo}–${r.hi}`;
  return `<div class="ho-read"><span>${esc(r.name)}</span><b>${val}</b><span class="ho-rbar"><i class="${r.yours ? "yours" : ""}" style="left:${pos(r.lo)}%;width:${Math.max(2.5, pos(r.hi) - pos(r.lo))}%"></i></span></div>`;
}

function hoSide(side, cls, tone, primary) {
  const reads = side.reads.length ? `<div class="ho-reads">${side.reads.map(hoRead).join("")}</div>` : "";
  const bars = side.bars.length ? `<div class="ho-bars">${side.bars.map(hoBar).join("")}</div>` : "";
  const list = side.list.length ? `<div class="ho-list"><span class="st-label ${cls === "hand" ? "give" : "get"}">${esc(side.list_head)}</span>${side.list.map((x) => `<p>${rich(x)}</p>`).join("")}</div>` : "";
  return `<section class="ho-side ho-${cls} ${cls === "hand" ? "tone-" + HO_CLS[tone] : ""}">
    <div class="ho-top"><b>${rich(side.title)}</b>${side.aside ? `<span class="muted small">${esc(side.aside)}</span>` : ""}</div>
    ${reads}
    <div class="ho-say"><span class="st-label">${esc(side.head)}</span><div class="ho-word">${esc(side.word)}</div>${side.text ? `<p>${rich(side.text)}</p>` : ""}</div>
    ${bars}${list}
    <button type="button" class="st-btn ${primary ? "primary" : "glass"} ${cls === "hand" && tone === "skip" ? "danger" : ""}" data-ho="${cls}">${esc(side.button)}</button>
  </section>`;
}

// The sheet. Resolves { pick: "hand" | "hold", label } or null (not now).
function handSheet(id) {
  return new Promise(async (done) => {
    let h;
    try { h = await api("people_handoff", { id }); } catch (e) { toast(e.message); return done(null); }
    const o = document.createElement("div");
    o.className = "overlay ho-overlay";
    o.innerHTML = `<div class="box ho" role="dialog" aria-modal="true" aria-label="${esc(h.label)}">
      <header><span class="st-label">${esc(h.tone === "man" ? "A man on your staff" : h.tone === "standard" ? "A fixed standard" : "Nobody goes")}</span><h2>${rich(h.title)}</h2></header>
      <div class="ho-grid">${hoSide(h.hand, "hand", h.tone, false)}${hoSide(h.hold, "hold", h.tone, true)}</div>
      ${h.note ? `<p class="ho-note">${esc(h.note)}</p>` : ""}
      <div class="ho-foot"><button type="button" class="st-btn glass" data-ho="no">Not now</button></div></div>`;
    document.body.appendChild(o);
    const keys = (ev) => { if (ev.key === "Escape") { ev.preventDefault(); end(null); } };
    const end = (v) => { o.remove(); document.removeEventListener("keydown", keys, true); done(v); };
    document.addEventListener("keydown", keys, true);
    o.addEventListener("mousedown", (ev) => { if (ev.target === o) end(null); });
    o.addEventListener("click", (ev) => {
      const b = ev.target.closest("[data-ho]");
      if (!b) return;
      end(b.dataset.ho === "no" ? null : { pick: b.dataset.ho, label: h.label });
    });
    const first = o.querySelector('[data-ho="hold"]');
    if (first) first.focus();
  });
}

// Hands the meeting on, or goes in to hold it, after the sheet.
async function handOff(el, ev) {
  if (ev) ev.stopPropagation();
  const id = Number(el.dataset.id);
  const r = await handSheet(id);
  if (!r) return;
  if (r.pick === "hold") {
    if (S.route.screen === "people_scene" && S.route.args && S.route.args.id === id) refresh(el);
    else go("people_scene", { id });
    return;
  }
  try {
    await api("people_hand", { id });
    S.status = await api("status");
    toast(r.label.replace(/^Hand to /, "Handed to ").replace(/^Let /, "Let ") + ".");
    if (S.route.screen === "people_scene") refresh(); else refresh(el);
  } catch (e) { toast(e.message); }
}

// ---------- the holiday preview ----------
const HO_DOT = { man: "var(--good)", standard: "var(--gold)", skip: "var(--live)" };
function hoHolidayHtml(p, picked) {
  const by = { man: 0, standard: 0, skip: 0 };
  p.rows.forEach((r) => { by[r.tone] += 1; });
  const bits = [];
  if (by.man) bits.push(`${by.man} go${by.man === 1 ? "es" : ""} to your coaches`);
  if (by.standard) bits.push(`${by.standard} ${by.standard === 1 ? "is" : "are"} ruled by your staff's standard`);
  if (by.skip) bits.push(`${by.skip} ${by.skip === 1 ? "is" : "are"} skipped`);
  const head = p.rows.length ? `${p.rows.length} meeting${p.rows.length === 1 ? "" : "s"} fall${p.rows.length === 1 ? "s" : ""} inside, until ${esc(p.untilText)}: ${bits.join(", ")}.` : `No meeting is set before ${esc(p.untilText)}.`;
  const rows = p.rows.map((r) => `<div class="ho-hrow"><span class="when">${esc(r.dateText)}</span><span class="ho-ttl"><i class="ho-dot" style="background:${HO_DOT[r.tone]}"></i>${rich(r.title)}</span><span class="ho-ln">${rich(r.line)}</span><button type="button" class="st-btn small glass" data-stop="${r.id}" aria-pressed="${picked.has(r.id)}" title="Come back the morning it is held">Stop for it</button></div>`).join("");
  const list = (t, xs) => (xs.length ? `<div class="ho-sm"><span class="st-label">${t}</span>${xs.map((x) => `<p>${rich(x)}</p>`).join("")}</div>` : "");
  return `<div class="ho-holiday"><p class="ho-hhead">${head}</p>${rows ? `<div class="ho-hrows">${rows}</div>` : ""}
    <div class="ho-two">${list("What can't be known yet", p.unknown)}${list("Duties your staff holds", p.duties)}</div></div>`;
}

// ---------- standing duties, read ahead ----------
async function dutySheet(key) {
  let p;
  try { p = await api("people_duty_preview", { duty: key }); } catch (e) { toast(e.message); return; }
  const o = document.createElement("div");
  o.className = "overlay ho-overlay";
  const reads = p.reads.length ? `<div class="ho-reads">${p.reads.map(hoRead).join("")}</div>` : "";
  const plan = p.plan.map((r) => `<div class="ho-prow"><b>${esc(r.dateText || r.label)}</b><span>${rich(r.text)}</span></div>`).join("");
  o.innerHTML = `<div class="box ho" role="dialog" aria-modal="true" aria-label="${esc(p.name)}">
    <header><span class="st-label">Standing duty · ${p.staff ? "your staff has it" : "you have it"}</span><h2>${esc(p.name)}</h2></header>
    <div class="ho-grid">
      <section class="ho-side"><div class="ho-top"><b>Who runs it</b></div><div class="ho-say"><div class="ho-word">${esc(p.who)}</div><p>${esc(p.does)}</p></div>${reads}
        <div class="ho-list"><span class="st-label get">You keep</span>${p.keep.map((x) => `<p>${rich(x)}</p>`).join("")}</div></section>
      <section class="ho-side"><div class="ho-top"><b>Their next two weeks</b><span class="muted small">as they'd run it</span></div><div class="ho-plan">${plan}</div>
        <div class="ho-list"><span class="st-label give">You give up</span>${p.give.map((x) => `<p>${rich(x)}</p>`).join("")}</div></section>
    </div>
    ${p.unknown ? `<p class="ho-note">${esc(p.unknown)}</p>` : ""}
    <div class="ho-foot"><button type="button" class="st-btn primary" data-ho="no">Close</button></div></div>`;
  document.body.appendChild(o);
  const keys = (ev) => { if (ev.key === "Escape") { ev.preventDefault(); end(); } };
  const end = () => { o.remove(); document.removeEventListener("keydown", keys, true); };
  document.addEventListener("keydown", keys, true);
  o.addEventListener("mousedown", (ev) => { if (ev.target === o) end(); });
  o.addEventListener("click", (ev) => { if (ev.target.closest("[data-ho]")) end(); });
}
Object.assign(CORE_MORE, { hoduty: (el, ev) => { if (ev) ev.stopPropagation(); dutySheet(el.dataset.duty); } });
const hoDutyLink = (key) => `<a class="link small ho-dutylink" data-act="hoduty" data-duty="${esc(key)}">What it comes to</a>`;
