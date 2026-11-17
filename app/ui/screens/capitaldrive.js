/* Capital: the drive (lane CS3; docs/design/capital.md, CP12, CP37, CP39,
 * board 3). Your program's capital campaign for its building: the goal and
 * what it means, how much is in (paid, pledged, slipping, still to raise,
 * all as shares of the goal, never dollars), the four stages, every pledge
 * with the moment that brought it, the lead donors still waiting for you,
 * your AD's read and the calls you can make today (the goal's size, going
 * public, working the lead donors yourself, pushing the AD to borrow).
 *
 *   capitaldrive   no arguments.
 *
 * It reads `capital_drive` and calls `capital_goal`, `capital_public`,
 * `capital_hands` and `capital_borrow` (lane_capital_drive.rs). The
 * building itself, an overrun and a gift's catch are the project screen's.
 * Every class is cdr- so the app's global names never restyle it. */
"use strict";

I.capdrive = '<path d="M4 20.5h16"/><path d="M6 20.5V9l6-4.5L18 9v11.5"/><path d="M9.5 20.5v-5h5v5"/><path d="M12 11v1"/>';
// The Capital section is shared by the six screens: whichever loads first
// adds it, the rest find it there.
if (!NAV_SECS.some((s) => s[0] === "capital")) NAV_SECS.splice(Math.max(1, NAV_SECS.findIndex((s) => s[0] === "team")), 0, ["capital", "Capital"]);
navAdd({ go: "capitaldrive", label: "The drive", icon: "capdrive", sec: "capital", seats: ["college"] });
CRUMBS.capitaldrive = "The drive";

const cdrDate = (iso, opt) => (iso ? new Date(iso + "T12:00:00").toLocaleDateString("en-US", opt || { month: "short", year: "numeric" }) : "");
const CDR_NAV = [["capital", "Campus"], ["capitaldonors", "Donors"], ["capitaldrive", "The drive"], ["capitalproject", "Project"], ["capitalpecking", "Pecking order"], ["capitalseat", "Your seat"]];

function cdrNav() {
  const here = CDR_NAV.filter((n) => n[0] === "capitaldrive" || screens[n[0]]);
  return `<nav class="cdr-nav" aria-label="Capital screens">${here.map((n) => `<button class="${n[0] === "capitaldrive" ? "on" : ""}" data-go="${n[0]}">${n[1]}</button>`).join("")}</nav>`;
}

// The bar: paid in, pledged, then what is left to raise, with the pledges
// donors pulled marked inside that space (they were never raised).
function cdrBar(d) {
  const p = d.pct;
  const seg = (v, cls) => (v > 0 ? `<i class="${cls}" style="width:${Math.min(v, 100)}%"></i>` : "");
  const yrs = d.pledgeYears.min == null ? "" : d.pledgeYears.min === d.pledgeYears.max ? `paid over ${d.pledgeYears.max} years` : `paid over ${d.pledgeYears.min} to ${d.pledgeYears.max} years`;
  const legend = [
    `<span><b class="in">${p.paid}% paid in</b></span>`,
    `<span><b class="gold">${p.pledged}% pledged</b>${yrs ? `, ${yrs}` : ""}</span>`,
    p.pulled > 0 ? `<span><b class="amb">${p.pulled}% pulled</b>: a donor took the rest back</span>` : "",
    d.stage === "closed" ? "" : `<span>${p.toRaise}% still to raise</span>`,
  ].filter(Boolean).join("");
  return `<div class="cdr-bar" role="img" aria-label="${p.paid}% paid in, ${p.pledged}% pledged, ${p.toRaise}% still to raise">${seg(p.paid, "in")}${seg(p.pledged, "gold")}${seg(Math.min(p.pulled, p.toRaise), "amb")}</div><div class="cdr-leg">${legend}</div>`;
}

function cdrStages(d) {
  return `<div class="cdr-stages">${d.stages.map((s) => {
    const when = s.date ? (s.state === "next" ? `by ${cdrDate(s.date)}` : cdrDate(s.date)) : "";
    return `<div class="cdr-st ${s.state}"><span class="ln"></span><b>${esc(s.label)}</b><span class="cdr-dim">${esc(s.note)}${when ? ` · ${esc(when)}` : ""}</span></div>`;
  }).join("")}</div>`;
}

function cdrGifts(d) {
  if (!d.gifts.length) return `<p class="cdr-empty">No pledge yet. ${d.stage === "study" ? "The study comes first." : "The lead gifts need you in the room."}</p>`;
  const head = `<div class="cdr-pl cdr-hd"><span>When</span><span>Who</span><span>For</span><span>What brought it</span></div>`;
  const rows = d.gifts.map((g) => {
    const bits = [esc(g.size), g.share ? `${g.share}% of the goal` : "under 1% of the goal"];
    if (g.tied) bits.push("tied to you");
    if (g.state === "pulled") bits.push('<b class="amb">pulled</b>');
    return `<div class="cdr-pl${g.state === "pulled" ? " pulled" : ""}"><span class="cdr-dim">${esc(cdrDate(g.when))}</span><span class="who"><b>${esc(g.who)}</b><small>${bits.join(" · ")}</small></span><span>${esc(g.what)}</span><span class="mo">${esc(g.moment)}</span></div>`;
  }).join("");
  return head + rows;
}

function cdrWaiting(d) {
  const lead = d.waiting.map((w) => `<div class="cdr-ask"><b>${esc(w.name)}</b>, ${esc(w.business)}.<small>${esc(w.read)} ${w.rooms.length ? `He'd give to the ${esc(w.rooms.join(" or the "))}. ` : ""}${esc(cdrCap(w.withYou))}${w.met ? `, met ${w.met} time${w.met > 1 ? "s" : ""}` : ", never met"}.</small></div>`).join("");
  const none = !d.waiting.length ? `<p class="cdr-dim sm">${d.stage === "closed" ? "The drive is closed." : d.stage === "study" ? "The study names the lead donors." : "Every lead donor has answered."}</p>` : "";
  const office = d.stage === "closed" || !d.office ? "" : `<p class="cdr-dim sm">${d.handsOn ? "The development office works" : "The development office works"} ${d.office} other donors for you. ${d.handsOn ? "Lead gifts need you in the room." : "You've left the lead gifts to the office: they give it only a courtesy gift."}</p>`;
  const link = screens.capitaldonors ? "capitaldonors" : "groundweek";
  return `<section class="cdr-card"><div class="cdr-k">Asks still out</div>${lead}${none}${office}${d.stage === "closed" ? "" : `<button class="cdr-link" data-go="${link}">Plan a donor slot</button>`}</section>`;
}
const cdrCap = (s) => (s ? s.charAt(0).toUpperCase() + s.slice(1) : s);

function cdrCalls(d) {
  const c = d.calls;
  const out = [];
  if (c.sizes.length) {
    out.push(`<div class="cdr-sub">The goal, before it goes public</div>${c.sizes.map((s) => `<div class="cdr-opt${s.current ? " cur" : ""}"><span><b>${esc(cdrCap(s.words))}</b><small>${s.advised ? "What the study advises" : s.bold ? "Bolder than the study says the donors are there for" : "Safer than the study's"}</small></span>${s.current ? '<small class="cdr-dim">Your goal</small>' : `<button class="st-btn glass" data-act="cdrgoal" data-size="${s.key}">Set it</button>`}</div>`).join("")}`);
  }
  if (c.public) {
    out.push(`<div class="cdr-opt"><span><b>Take the drive public</b><small>${c.publicEarly ? "Less than half is in. A losing season after it stalls the drive in the papers." : "Half the goal is in: this is when real campaigns go public."}</small></span><button class="st-btn glass${c.publicEarly ? "" : " gold"}" data-act="cdrpublic">${c.publicEarly ? "Go early" : "Go public"}</button></div>`);
  }
  if (c.hands) {
    out.push(`<div class="cdr-opt"><span><b>${c.handsOn ? "You work the lead donors" : "The office runs the drive"}</b><small>${c.handsOn ? "Each lead gift needs you in the room." : "Your lead donors get a courtesy gift from the office, not the big one."}</small></span><button class="st-btn glass" data-act="cdrhands" data-on="${c.handsOn ? 0 : 1}">${c.handsOn ? "Hand it to the office" : "Work them yourself"}</button></div>`);
  }
  if (c.borrow) {
    out.push(`<div class="cdr-sub">Push your AD to borrow</div><div class="cdr-opt"><span><b>Break ground now</b><small>The bonds come out of football's staff pool for years. One ask a year.</small></span><button class="st-btn glass" data-act="cdrborrow" data-bigger="0">Ask</button></div>${c.borrowBigger ? `<div class="cdr-opt"><span><b>Build ${esc(c.borrowBiggerWords)}</b><small>A bigger goal and the same bonds, bigger.</small></span><button class="st-btn glass" data-act="cdrborrow" data-bigger="1">Ask</button></div>` : ""}`);
  }
  if (!out.length) return "";
  return `<section class="cdr-card"><div class="cdr-k">Your calls</div>${out.join("")}</section>`;
}

screens.capitaldrive = async () => {
  let x;
  try { x = await api("capital_drive", {}); } catch (e) {
    title("The drive", "");
    return `<div class="pad sl cdr"><section class="cdr-card"><div class="cdr-k">Capital</div><p class="cdr-empty">${esc(e.message || String(e))}</p></section></div>`;
  }
  title("The drive", x.program);
  const top = `<div class="cdr-top"><div class="cdr-t"><div class="cdr-k">Capital · ${esc(x.program)}</div><h1>The drive</h1></div>${cdrNav()}<small class="cdr-dim">${esc(cdrDate(x.today, { weekday: "short", month: "long", day: "numeric", year: "numeric" }))}</small></div>`;
  const d = x.drive;
  if (!d) {
    const msg = x.ready
      ? "No drive under way. Your AD opens a study each July when the department's money and fans would keep your facilities above where they stand, or the fever is up. You can also ask him for a facility in a meeting."
      : "Drives begin with the first season of your career. Until then the buildings come from the department's old decisions.";
    return `<div class="pad sl cdr">${top}<section class="cdr-card"><div class="cdr-k">The drive</div><p class="cdr-empty">${esc(msg)}</p></section></div>`;
  }
  const closed = d.stage === "closed";
  const label = closed ? (d.shelved ? "A drive shelved" : d.hit ? "A drive closed on its goal" : "A drive closed short") : d.setByYou ? "Your goal" : "The study's goal";
  const when = closed ? (d.closed ? `, ${cdrDate(d.closed)}` : "") : d.publicOn ? `, announced ${cdrDate(d.publicOn, { month: "long", day: "numeric", year: "numeric" })}` : "";
  const close = d.stages.find((s) => s.key === "close");
  const big = d.shelved ? "Shelved" : `${d.pct.total}%`;
  const sub = d.shelved ? "its pledges wait for the next drive" : closed ? "of the goal raised" : `pledged${close && close.date ? ` · close by ${cdrDate(close.date)}` : ""}`;
  const building = d.groundBroke
    ? `<p class="cdr-dim sm">Ground broke ${esc(cdrDate(d.groundBroke, { month: "long", day: "numeric", year: "numeric" }))}${d.opens ? `; it opens in ${d.opens}` : ""}.${d.overrun ? ' <b class="amb">The cost has run over: it needs your answer.</b>' : ""} ${screens.capitalproject ? '<button class="cdr-link" data-go="capitalproject">Open the project</button>' : ""}</p>` : "";
  const hero = `<section class="cdr-card cdr-hero">
    <div class="cdr-h"><div class="cdr-hl"><div class="cdr-k">${label}${esc(when)}</div><div class="cdr-bt">${esc(d.title)}</div><p class="cdr-dim">${esc(d.story)}</p></div>
      <div class="cdr-hr"><div class="cdr-big${d.shelved ? " small" : ""}">${esc(big)}</div><div class="cdr-dim sm">${esc(sub)}</div></div></div>
    ${d.shelved ? "" : cdrBar(d)}${cdrStages(d)}${building}</section>`;
  const ad = `<section class="cdr-card"><div class="cdr-k">Your AD${d.ad.name ? `, ${esc(d.ad.name)}` : ""} · ${esc(d.ad.trust)}</div><p class="cdr-q">${esc(d.ad.says)}</p><p class="cdr-dim sm">${esc(d.ad.fine)}</p></section>`;
  return `<div class="pad sl cdr">${top}${hero}<div class="cdr-cols"><section class="cdr-card cdr-gifts"><div class="cdr-k">Every pledge, and what brought it</div>${cdrGifts(d)}</section><div class="cdr-side">${ad}${cdrWaiting(d)}${cdrCalls(d)}</div></div></div>`;
};

// ---- the buttons ----
const cdrDone = async (el, p) => {
  try {
    const r = await p;
    if (r && r.said) toast(r.said);
  } catch (e) { toast(e.message || String(e)); }
  refresh(el);
};
Object.assign(CORE_MORE, {
  cdrgoal: (el) => cdrDone(el, api("capital_goal", { size: el.dataset.size })),
  cdrpublic: (el) => cdrDone(el, api("capital_public", {})),
  cdrhands: (el) => cdrDone(el, api("capital_hands", { on: el.dataset.on === "1" })),
  cdrborrow: (el) => cdrDone(el, api("capital_borrow", { bigger: el.dataset.bigger === "1" })),
});
