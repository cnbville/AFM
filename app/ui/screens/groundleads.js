/* Ground's lead card (lane S4; docs/design/ground.md part N, board 4b, GR73,
 * GR79). A lead is a kid somebody told you about. The card answers four
 * things at once: who he is and how he's growing, which of the five blanks
 * are filled (and by whom), who else is on him against the day the services
 * rate his class, and what you do about it.
 *
 *   groundleads   your open leads as a strip, and the picked lead's card:
 *                 his face and size, what the teller's record says, the ring
 *                 of five blanks with who filled each and when, who's in his
 *                 corner, the race in signs on a timeline against the
 *                 services' next update, your word (offer now, fill the last
 *                 blank first, let him go) with what each risks, and the
 *                 bloom steps with what each pays.
 *
 * It reads `ground_leads` and `ground_lead` and writes `ground_fill`,
 * `ground_lead_drop` and `recruit_offer` (lanes G4 and the recruiting desk).
 * Press Box look (GR79): dark panels, ivory text, one gold accent for what
 * needs your call, amber for a rival's marks and what slips. Every class is
 * gl- so the app's global names never restyle it. */
"use strict";

I.gdlead = '<circle cx="11" cy="11" r="6.5"/><path d="M16 16l4.5 4.5M11 8v6M8 11h6"/>';
navAdd({ go: "groundleads", label: "Leads", icon: "gdlead", sec: "ground", after: "groundweek", seats: ["college"], badge: () => GL.n });
CRUMBS.groundleads = "Leads";

const GL = { id: null, ask: null, n: 0 };
const GL_MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const glDay = (iso) => new Date(iso + "T12:00:00").getTime() / 864e5;
const glShort = (iso) => { const d = new Date(iso + "T12:00:00"); return `${GL_MONTHS[d.getMonth()]} ${d.getDate()}`; };
const glLong = (iso) => new Date(iso + "T12:00:00").toLocaleDateString("en-US", { weekday: "long", month: "long", day: "numeric", year: "numeric" });
const glIni = (name, size = 28) => `<span class="gl-ini" style="--s:${size}px">${esc((name || "?").split(/[\s,]+/).filter(Boolean).map((x) => x[0]).slice(0, 2).join("").toUpperCase())}</span>`;
const glLast = (n) => { const w = (n || "").split(" ").filter(Boolean); while (w.length > 1 && /^(jr\.?|sr\.?|ii|iii|iv|v)$/i.test(w[w.length - 1])) w.pop(); return w[w.length - 1] || ""; };
// A line he said, in quotes once (the engine's own line may carry them already).
const glQuote = (t) => `\u201C${String(t || "").replace(/^["\u201C]+|["\u201D"]+$/g, "")}\u201D`;
const glWord = (n) => ["no", "one", "two", "three", "four", "five", "six"][n] ?? String(n);

// "rising junior, class of 2028": a class's year in school from the date.
function glYear(classOf, iso) {
  const d = new Date(iso + "T12:00:00");
  const end = d.getFullYear() + (d.getMonth() >= 7 ? 1 : 0);
  const left = classOf - end;
  const names = ["senior", "junior", "sophomore", "freshman"];
  const summer = d.getMonth() === 5 || d.getMonth() === 6;
  if (left < 0 || left > 4) return `class of ${classOf}`;
  const k = summer ? left - 1 : left;
  if (k < 0) return `class of ${classOf}`;
  return `${summer ? "rising " : ""}${names[Math.min(k, 3)]}, class of ${classOf}`;
}

screens.groundleads = async (args) => {
  const list = await api("ground_leads", {});
  const rows = list.leads || [];
  GL.n = rows.length;
  // A link carries the lead once; picking another afterwards must stick.
  if (args && args.id) { GL.id = +args.id; delete args.id; }
  if (!rows.some((r) => r.kid.id === GL.id)) GL.id = rows.length ? (rows.find((r) => r.ready) || rows[0]).kid.id : null;
  title("Leads", rows.length ? `${rows.length} open` : "None open");
  const ready = rows.filter((r) => r.ready).length;
  const top = (x) => {
    const sv = x ? x.services : null;
    const nextStat = sv && !sv.rated ? `<div class="gl-stat"><span>Services' next update</span><b>${esc(glShort(sv.next.date))} · ${esc(sv.next.season)}</b></div>` : "";
    return `<div class="gl-top"><div><h1>${esc(glLong(list.today))}</h1><small>${esc(list.finder || "")}</small></div><div class="gl-sp"></div>
      <div class="gl-stat"><span>Leads</span><b>${rows.length} open${ready ? ` · ${ready} with every blank filled` : ""}</b></div>${nextStat}
      ${rows.length > 1 ? `<button class="st-btn primary" data-act="glnext">Next lead</button>` : ""}</div>`;
  };
  if (!rows.length) {
    return `<div class="pad sl gl">${top(null)}<section class="gl-pb gl-none"><div class="gl-lab">No leads yet</div>
      <b class="gl-h">Nobody has told you about a kid yet.</b>
      <p>A lead starts when a person on your ground gives you a name: a head coach's list, a call that carries news, a tip from an alum, a kid who walks up at your camp. Your staff has to be at the schools for any of it to happen.</p>
      <div><button class="st-btn primary" data-go="groundweek">Open this week</button> <button class="st-btn glass" data-go="ground">Open the map</button></div></section></div>`;
  }
  const x = await api("ground_lead", { id: GL.id });
  const CAP = 14;
  let shown = rows.slice(0, CAP);
  if (!shown.some((r) => r.kid.id === GL.id)) shown = [...shown.slice(0, CAP - 1), rows.find((r) => r.kid.id === GL.id)];
  const strip = rows.length > 1 ? `<div class="gl-strip" role="tablist" aria-label="Your leads">${shown.map((r) => `<button class="gl-chip${r.kid.id === GL.id ? " on" : ""}" role="tab" aria-selected="${r.kid.id === GL.id}" data-act="glpick" data-id="${r.kid.id}"><span class="gl-face sm" data-face="${r.kid.id}">${glIni(r.kid.name, 26)}</span><span class="tx"><b>${esc(r.kid.name)}</b><small>${esc(r.kid.pos)} · ${r.filled}/5</small></span></button>`).join("")}${rows.length > shown.length ? `<span class="gl-more">${rows.length - shown.length} more. Next lead walks through them all.</span>` : ""}</div>` : "";
  setTimeout(glFaces, 30);
  return `<div class="pad sl gl">${top(x)}${strip}<div class="gl-grid">${glLeft(x)}${glMid(x)}${glRight(x)}</div></div>`;
};

function glFaces() {
  const slots = [...document.querySelectorAll(".gl-face[data-face]")];
  if (!slots.length || !window.AFMFaces || !AFMFaces.pics) return;
  const ids = [...new Set(slots.map((s) => +s.dataset.face))];
  AFMFaces.pics(ids, (id, src) => document.querySelectorAll(`.gl-face[data-face="${id}"]`).forEach((s) => {
    if (!s.querySelector("img")) s.insertAdjacentHTML("afterbegin", `<img src="${src}" alt="">`);
    s.classList.add("has");
  }));
}

// ---- the left column: him ----
function glLeft(x) {
  const k = x.kid, l = x.lead, sv = x.services;
  const teller = l && l.from ? l.from : null;
  const chips = [teller ? `${glLast(teller.name)}'s word` : l && l.tipper ? `${l.tipper}'s tip` : "", l && l.man ? `${glLast(l.man.name)}'s area` : ""].filter(Boolean).map((c) => `<span class="gl-tag">${esc(c)}</span>`).join("");
  const all = x.offers_all || k.offers;
  const offers = all.length ? `${all.length} ${all.length === 1 ? "offer" : "offers"}` : "None";
  const offerLine = all.length ? all.slice(0, 3).join(", ") + (all.length > 3 ? ` and ${all.length - 3} more` : "") : "public";
  const face = `<section class="gl-pb gl-id"><div class="gl-face big" data-face="${k.id}">${glIni(k.name, 96)}<div class="gl-over"><div class="gl-lab">Lead${l ? ` · since ${esc(glShort(l.since_iso))}` : ""}</div><div class="gl-name">${esc(k.name)}</div></div></div>
    <div class="gl-dim">${esc(k.pos)} · ${esc(k.school)} · ${esc(glYear(k.class_of, x.today))}</div>
    ${chips ? `<div class="gl-tags">${chips}</div>` : ""}
    <div class="gl-kv"><div><span>The services</span><b>${sv.rated ? `${sv.stars} ${sv.stars === 1 ? "star" : "stars"}` : "Not rated"}</b><small>${sv.rated ? "rated" : `the ${esc(sv.next.season)} update is ${esc(glShort(sv.next.date))}`}</small></div>
      <div><span>Offers</span><b>${esc(offers)}</b><small>${esc(offerLine)}</small></div></div></section>`;
  return `<div class="gl-col a">${face}${glGrowth(x)}${glRecord(x)}</div>`;
}

function glGrowth(x) {
  const g = x.growth, l = x.lead;
  const forties = (g.times || []).filter((t) => t.event === "Forty").sort((a, b) => (a.timed === "Laser" ? -1 : 1) - (b.timed === "Laser" ? -1 : 1));
  const f = forties[0];
  const forty = f ? f.secs : null;
  const m = g.measured, t = g.then;
  const track = (a, b, hi) => `<span class="tr"><i class="a" style="width:${Math.max(2, Math.round(100 * a / hi))}%"></i><i class="b" style="left:${Math.round(100 * a / hi)}%;width:${Math.max(2, Math.round(100 * (b - a) / hi))}%"></i></span>`;
  let body;
  if (m && t) {
    const dw = m.weight - t.weight, dh = m.height_in - t.height_in;
    const sgn = (n, u) => `${n > 0 ? "+" : n < 0 ? "−" : ""}${Math.abs(n)} ${u}`;
    body = `<div class="gl-gr" role="img" aria-label="Weight ${t.weight} to ${m.weight} pounds, height ${t.height} to ${m.height}${forty ? `, forty ${forty.toFixed(2)}` : ""}">
      <div><span>Weight</span>${dw >= 0 ? track(t.weight, m.weight, m.weight * 1.12) : track(m.weight, t.weight, t.weight * 1.12)}<b>${sgn(dw, "lb")}</b></div>
      <div><span>Height</span>${dh >= 0 ? track(t.height_in, m.height_in, m.height_in * 1.1) : track(m.height_in, t.height_in, t.height_in * 1.1)}<b>${sgn(dh, "in")}</b></div>
      ${forty ? `<div><span>The forty</span><span class="tr"><i class="b" style="width:${Math.max(8, Math.min(100, Math.round(100 * (5.6 - forty) / 1.2)))}%"></i></span><b>${forty.toFixed(2)}</b></div>` : ""}</div>
      <small class="gl-dim">${t.weight} pounds and ${esc(t.height)} a year ago, ${m.weight} and ${esc(m.height)} now${forty ? `, ${forty.toFixed(2)} ${f.timed === "Laser" ? "on the laser" : f.timed === "Fat" ? "fully timed" : "by hand"}` : ""}.</small>`;
  } else {
    const said = l && (l.said_height || l.said_forty) ? `His word: ${[l.said_height, l.said_forty ? l.said_forty.toFixed(2) + " in the forty" : ""].filter(Boolean).join(", ")}. ` : "";
    body = `<p class="gl-note">${esc(said)}Nobody of yours has measured him yet.${forty ? ` Timed ${forty.toFixed(2)} ${f.timed === "Hand" ? "by hand" : f.timed === "Laser" ? "on the laser" : "fully timed"}.` : ""} Your camp or a showcase does it.</p>`;
  }
  const oversold = l && l.word === false ? `<p class="gl-warn">He isn't what he said he was.</p>` : l && l.word === true ? `<p class="gl-ok">He was what he said he was.</p>` : "";
  return `<section class="gl-pb"><div class="gl-lab">He's growing · a year ago to now</div>${body}${oversold}</section>`;
}

function glRecord(x) {
  const l = x.lead;
  if (!l) return "";
  const t = l.from;
  const man = l.man;
  let head, segs = "";
  if (t) {
    const held = t.held, of = t.of;
    segs = of ? `<div class="gl-seg" role="img" aria-label="${held} of ${of} held up">${Array.from({ length: Math.min(of, 24) }, (_, i) => `<i class="${i < held ? "h" : "m"}"></i>`).join("")}</div>` : "";
    head = `<div class="gl-row"><div class="gl-lab">${esc(glLast(t.name))}'s record</div><span class="gl-dim sm">kids he told you about</span></div>${segs}
      <div class="gl-row sm"><span>${of ? `<b>${held} of ${of}</b> held up` : "No kid of his judged yet"}</span><span class="gl-dim">${esc(t.role)}</span></div>`;
  } else {
    head = `<div class="gl-lab">${esc(l.tipper || "A tipper")}'s tip</div><p class="gl-note">A tipper's record is kept by his tips, not here. ${esc(l.how_words || "")}</p>`;
  }
  const manLine = man ? `<div class="gl-row sm"><span>${esc(man.name)} <span class="gl-dim">heard it for you</span></span><span><b>${man.finds} ${man.finds === 1 ? "find" : "finds"}</b>, ${man.blooms} bloomed</span></div>` : "";
  return `<section class="gl-pb">${head}${manLine}</section>`;
}

// ---- the middle column: the five blanks ----
function glMid(x) {
  const bl = x.blanks, l = x.lead, k = x.kid;
  const n = bl.filter((b) => b.filled).length;
  const open = bl.filter((b) => !b.filled);
  const C = 2 * Math.PI * 38;
  const ring = `<svg width="92" height="92" viewBox="0 0 92 92" role="img" aria-label="${n} of 5 blanks filled"><circle cx="46" cy="46" r="38" class="gl-rb"/><circle cx="46" cy="46" r="38" class="gl-rf" stroke-dasharray="${(C * n / 5).toFixed(1)} ${C.toFixed(1)}" transform="rotate(-90 46 46)"/><text x="46" y="53" text-anchor="middle" class="gl-rt">${n}/5</text></svg>`;
  const head = open.length === 0 ? "Every blank is filled" : open.length === 1 ? `One blank left: ${open[0].name.toLowerCase()}` : `${glWord(open.length).replace(/^./, (c) => c.toUpperCase())} blanks to fill`;
  const sub = open.length ? `${esc(open[0].fills)}, ${esc(open[0].time)}.${open.some((b) => b.asked) ? " You've asked for one already." : ""}` : "Everything your staff can learn about him is in the file.";
  const hero = `<section class="gl-pb gl-hero">${ring}<div><div class="gl-lab">The lead card · five blanks</div><div class="gl-big">${esc(head)}</div><div class="gl-dim">${sub}</div></div></section>`;

  const forties = (x.growth.times || []).filter((t) => t.event === "Forty").sort((a, b) => (a.timed === "Laser" ? -1 : 1) - (b.timed === "Laser" ? -1 : 1));
  const found = (b) => {
    const m = x.growth.measured;
    const how = b.filled ? (b.filled.how || "A look") : "";
    const read = k.staff ? ` Your staff reads him ${k.staff[0]}-${k.staff[1]}.` : "";
    if (b.key === "size" && m) return `${m.height}, ${m.weight} pounds.${forties[0] ? "" : ""}`;
    if (b.key === "speed" && forties[0]) return `Ran ${forties[0].secs.toFixed(2)} ${forties[0].timed === "Laser" ? "on the laser" : forties[0].timed === "Fat" ? "fully timed" : "by hand"}.`;
    if (b.key === "tape" || b.key === "live") return `${how}.${read}`;
    return `${how}.`;
  };
  const rowsHtml = bl.map((b, i) => {
    if (b.filled) {
      return `<div class="gl-ev"><span class="n">${i + 1}</span><div><b>${esc(b.name)}</b><p>${esc(found(b))}</p></div><div class="wh">${glIni(b.filled.by, 28)}<div><b>${esc(b.filled.by)}</b>${esc(b.filled.how || "")} · ${esc(glShort(b.filled.iso))}</div></div></div>`;
    }
    const first = b === open[0];
    const note = b.key === "character" && l ? `${esc(l.from ? glLast(l.from.name) : "He")} says ${esc(glQuote(l.said))} That's what he tells you at ${esc((l.from && l.from.level) || "his level")}.` : "";
    const btn = b.asked
      ? `<span class="gl-wait">Asked. It waits for its day.</span>`
      : `<button class="st-btn ${first ? "primary" : "glass"}" data-act="glfill" data-id="${k.id}" data-blank="${esc(b.key)}">${esc(glFillWord(b))}</button><small class="gl-dim">${esc(b.time)}</small>`;
    return `<div class="gl-ev open"><span class="n">${i + 1}</span><div><b class="gl-gold">${esc(b.name)} · open</b><p>${note || esc(b.fills) + "."}</p></div><div class="act">${btn}</div></div>`;
  }).join("");
  const blanks = `<section class="gl-pb gl-blanks">${rowsHtml}</section>`;

  // Who's in his corner: the teller's word, and what your staff made of him.
  const voices = [];
  if (l && (l.from || l.tipper)) {
    const nm = l.from ? l.from.name : l.tipper;
    voices.push(`<div class="gl-vc">${glIni(nm, 34)}<div><b>${esc(nm)} told you</b>${l.claim ? `<span class="gl-tag sm">${esc(l.claim)}</span>` : ""}<p>${esc(glQuote(l.said))}</p><small>${l.from ? `You're ${esc(l.from.level)} with him. ${esc(l.from.why || "")}` : "A tip, not a person on your ground."}</small></div></div>`);
  }
  const tape = bl.find((b) => b.key === "tape" && b.filled) || bl.find((b) => b.key === "live" && b.filled);
  const who = tape ? tape.filled.by : l && l.man ? l.man.name : "Your staff";
  const pot = k.potential && (k.potential.staff || k.potential.mine || k.potential);
  const tier = pot && pot.tier ? `${pot.tier}${pot.sure ? `, ${pot.sure}` : ""}` : "";
  voices.push(`<div class="gl-vc">${glIni(who, 34)}<div><b>${esc(who)} on the file</b><p>${k.staff ? `"Reads him ${k.staff[0]} to ${k.staff[1]}${tier ? `. ${esc(tier)}` : ""}."` : "Nobody has seen him enough to read him yet."}</p>${pot && pot.hints && pot.hints.length ? `<small>${esc(pot.hints[0])}</small>` : ""}</div></div>`);
  const corner = `<section class="gl-pb"><div class="gl-lab">Who's in his corner</div><div class="gl-two2">${voices.join("")}</div></section>`;
  return `<div class="gl-col b">${hero}${blanks}${corner}</div>`;
}
const glFillWord = (b) => ({ size: "Measure him", speed: "Time him", tape: "Get his tape", live: "See him live", character: "Make the calls" }[b.key] || "Fill it");

// ---- the right column: the race and your word ----
function glRight(x) {
  return `<div class="gl-col c">${glRace(x)}${glWordBox(x)}${glBloom(x)}</div>`;
}

function glRace(x) {
  const R = x.race, sv = x.services;
  const today = glDay(x.today);
  const marks = [...R.you.map((m) => m.iso), ...R.rivals.flatMap((r) => r.marks.map((m) => m.iso).filter(Boolean))];
  const winEnd = !sv.rated ? glDay(sv.next.date) : null;
  const t0 = Math.min(today - 10, ...marks.map(glDay)) - 4;
  const t1 = Math.max(today + 14, winEnd ? winEnd : 0) + 4;
  const W = 340, X0 = 8, X1 = 332, ROW = 56, TOP = 34;
  const px = (t) => X0 + ((t - t0) / (t1 - t0)) * (X1 - X0);
  const rows = [...R.rivals.map((r) => ({ name: r.program, you: false, marks: r.marks })), { name: "You", you: true, marks: R.you.map((m) => ({ iso: m.iso, what: m.what })) }];
  const H = TOP + rows.length * ROW + 30;
  let svg = "";
  if (winEnd) {
    svg += `<rect class="gl-win" x="${px(today).toFixed(1)}" y="${TOP - 6}" width="${(px(winEnd) - px(today)).toFixed(1)}" height="${rows.length * ROW + 4}" rx="8"/>`;
    svg += `<line class="gl-end" x1="${px(winEnd).toFixed(1)}" x2="${px(winEnd).toFixed(1)}" y1="${TOP - 10}" y2="${TOP + rows.length * ROW}"/>`;
    const wk = Math.max(1, Math.round((winEnd - today) / 7));
    svg += `<text class="gl-wl" x="${((px(today) + px(winEnd)) / 2).toFixed(1)}" y="${TOP + 12}" text-anchor="middle">YOUR WINDOW</text><text class="gl-ws" x="${((px(today) + px(winEnd)) / 2).toFixed(1)}" y="${TOP + 26}" text-anchor="middle">${wk} ${wk === 1 ? "week" : "weeks"}</text>`;
    svg += `<text class="gl-el" x="${W - 4}" y="12" text-anchor="end">${esc(glShort(sv.next.date)).toUpperCase()} · SERVICES' ${esc(sv.next.season).toUpperCase()} UPDATE</text>`;
  }
  svg += `<line class="gl-now" x1="${px(today).toFixed(1)}" x2="${px(today).toFixed(1)}" y1="${TOP - 10}" y2="${TOP + rows.length * ROW}"/><text class="gl-nl" x="${(px(today) - 4).toFixed(1)}" y="${TOP - 14}" text-anchor="end">NOW · ${esc(glShort(x.today)).toUpperCase()}</text>`;
  rows.forEach((r, ri) => {
    const y = TOP + ri * ROW + 16;
    svg += `<text class="gl-rn${r.you ? " you" : ""}" x="${X0}" y="${y - 10}">${esc(r.name.toUpperCase())}</text><line class="gl-rl" x1="${X0}" x2="${X1}" y1="${y + 4}" y2="${y + 4}"/>`;
    const dated = r.marks.filter((m) => m.iso).sort((a, b) => glDay(a.iso) - glDay(b.iso));
    const undated = r.marks.filter((m) => !m.iso);
    if (dated.length > 1) svg += `<line class="gl-seg2${r.you ? " you" : ""}" x1="${px(glDay(dated[0].iso)).toFixed(1)}" x2="${px(glDay(dated[dated.length - 1].iso)).toFixed(1)}" y1="${y + 4}" y2="${y + 4}"/>`;
    // Labels sit under the line in two lanes; one that would overlap its lane's last is left to its title.
    const lane = [-1e9, -1e9];
    for (const m of dated) {
      const cx = px(glDay(m.iso));
      const off = /offered/.test(m.what);
      svg += `<circle class="gl-dot${r.you ? " you" : ""}${off ? " off" : ""}" cx="${cx.toFixed(1)}" cy="${y + 4}" r="5.2"><title>${esc(glShort(m.iso))}: ${esc(m.what)}</title></circle>`;
      const txt = `${glShort(m.iso)} · ${m.what}`;
      const w = txt.length * 5.4;
      let left = Math.min(Math.max(cx - w / 2, X0), X1 - w);
      const k = lane[0] + 6 <= left ? 0 : lane[1] + 6 <= left ? 1 : -1;
      if (k >= 0) {
        lane[k] = left + w;
        svg += `<text class="gl-ml" x="${left.toFixed(1)}" y="${y + 20 + k * 12}">${esc(txt)}</text>`;
      }
    }
    for (const m of undated) {
      svg += `<circle class="gl-dot off" cx="${(X1 - 6).toFixed(1)}" cy="${y + 4}" r="5.2"><title>${esc(m.what)}, date not known</title></circle><text class="gl-ml" x="${X1}" y="${y - 6}" text-anchor="end">${esc(m.what)}</text>`;
    }
    if (!dated.length && !undated.length) svg += `<text class="gl-ml" x="${X0 + 6}" y="${y + 20}">nothing yet</text>`;
  });
  // Month ticks along the bottom.
  const ticks = [];
  const d0 = new Date(t0 * 864e5 + 12 * 36e5);
  for (let d = new Date(d0.getFullYear(), d0.getMonth() + 1, 1, 12); d.getTime() / 864e5 < t1; d = new Date(d.getFullYear(), d.getMonth() + 1, 1, 12)) {
    const cx = px(d.getTime() / 864e5);
    ticks.push(`<text class="gl-tk" x="${cx.toFixed(1)}" y="${H - 4}">${GL_MONTHS[d.getMonth()].toUpperCase()}</text>`);
  }
  svg += `<line class="gl-axis" x1="${X0}" x2="${X1}" y1="${H - 16}" y2="${H - 16}"/>${ticks.join("")}`;
  const aria = `The race: ${R.rivals.length ? R.rivals.map((r) => `${r.program} ${r.marks.map((m) => `${m.what}${m.iso ? " " + glShort(m.iso) : ""}`).join(", ")}`).join("; ") + ". " : "Nobody else shows on him. "}You: ${R.you.map((m) => `${m.what} ${glShort(m.iso)}`).join(", ")}.`;
  const note = sv.rated ? `The services have him at ${sv.stars} ${sv.stars === 1 ? "star" : "stars"}, so everybody can see him now.` : R.rivals.length ? "Only what you can see: who heard when, and who has offered." : "No other staff shows on him yet. That can change any day.";
  return `<section class="gl-pb"><div class="gl-row"><div class="gl-lab">The race · who heard, and when</div><span class="gl-dim sm">in signs</span></div>
    <svg viewBox="0 0 ${W} ${H}" width="100%" role="img" aria-label="${esc(aria)}">${svg}</svg><small class="gl-dim">${esc(note)}</small></section>`;
}

function glWordBox(x) {
  const w = x.word, k = x.kid, f = x.find;
  if (f) {
    const bloomed = f.blooms.map((b) => `<li><b>${esc(b.date)}</b> ${esc(b.words)}</li>`).join("");
    return `<section class="gl-pb gl-find"><div class="gl-row"><div class="gl-lab">He's your find</div><span class="gl-dim sm">you offered ${esc(f.date)}</span></div>
      <p class="gl-q">The world had him at ${f.stars ? `${f.stars} ${f.stars === 1 ? "star" : "stars"}` : "no stars"} with ${f.offers ? f.offers : "no"} other ${f.offers === 1 ? "offer" : "offers"}. ${esc(f.pays)}</p>
      ${bloomed ? `<ul class="gl-bl">${bloomed}</ul>` : `<small class="gl-dim">He hasn't bloomed yet. When the world catches up, it's on your name.</small>`}
      ${f.lost ? `<p class="gl-warn">He signed with ${esc(f.lost.program)}. He's still your find.</p>` : ""}${f.setback ? `<p class="gl-warn">${esc(f.setback.words)}</p>` : ""}
      <div class="gl-two"><button class="st-btn glass" data-act="gldrop" data-id="${k.id}">${GL.ask === "drop" ? "Drop the lead for good?" : "Drop the lead"}</button></div></section>`;
  }
  if (!w) return `<section class="gl-pb"><div class="gl-lab">Your word</div><p class="gl-note">He's past high school, so there's nothing to decide here.</p></section>`;
  const acts = w.options.map((o, i) => {
    if (o.key === "offer") {
      const sure = GL.ask === "offer";
      return `<button class="gl-act go${sure ? " sure" : ""}" data-act="gloffer" data-id="${k.id}"><b>${sure ? `Yes, offer ${esc(glLast(k.name))} a scholarship` : "Offer him now"}</b><small>${esc(o.risk)}${sure ? " It goes out today. Pulling it back is on the recruiting desk." : ""}</small></button>`;
    }
    if (o.key === "fill") return `<button class="gl-act" data-act="glfill" data-id="${k.id}" data-blank="${esc(o.blank)}"><b>${esc(o.label)}</b><small>${esc(o.risk)}</small></button>`;
    return `<div class="gl-act wait"><b>${esc(o.label)}</b><small>${esc(o.risk)}</small></div>`;
  }).join("");
  const offered = w.offered ? `<p class="gl-ok">You've offered him. ${w.find_if_offered ? "" : "The world already rated him, so he isn't a find."}</p>` : "";
  const drop = `<button class="gl-act" data-act="gldrop" data-id="${k.id}"><b>${GL.ask === "drop" ? "Yes, let him go" : "Let him go"}</b><small>${GL.ask === "drop" ? "He leaves your leads. News about him can bring him back." : "Takes him off your leads. Nothing is said to him."}</small></button>`;
  return `<section class="gl-pb"><div class="gl-row"><div class="gl-lab">Your word</div><span class="gl-dim sm">${(x.offers_all || k.offers).length ? `${(x.offers_all || k.offers).length} ${(x.offers_all || k.offers).length === 1 ? "offer" : "offers"} out` : "Nobody has offered"}</span></div>${offered}${acts}${drop}</section>`;
}

function glBloom(x) {
  const w = x.word, f = x.find;
  if (!w || !w.steps) return "";
  const reached = f ? f.steps : 0;
  const bar = ["33%", "66%", "100%"];
  const rows = w.steps.map((s, i) => `<div class="gl-st${i < reached ? " got" : ""}"><span class="gl-tick"><i style="width:${bar[i]}"></i></span><div><b>${esc(s.what)}</b>${s.pays ? `<small>${esc(s.pays)}</small>` : ""}</div></div>`).join("");
  const chip = w.find_if_offered ? `<span class="gl-tag sm">${esc(w.pays || "")}</span>` : "";
  return `<section class="gl-pb"><div class="gl-row"><div class="gl-lab">${f ? "What his bloom pays" : "If you offer and he blooms"}</div></div>${chip}<div class="gl-steps">${rows}</div>
    <small class="gl-dim">${w.find_if_offered || f ? "Each step pays the teller, your man and your name as a finder. A find you lose to a rival stays yours." : "The world rates him already, so a bloom pays nothing here."}</small></section>`;
}

// ---- actions ----
const glSay = (r) => { if (r && r.text) toast(r.text); };
Object.assign(CORE_MORE, {
  glpick: (el) => { GL.id = +el.dataset.id; GL.ask = null; refresh(el); },
  glnext: async (el) => {
    try {
      const rows = (await api("ground_leads", {})).leads;
      const i = rows.findIndex((r) => r.kid.id === GL.id);
      if (rows.length) GL.id = rows[(i + 1) % rows.length].kid.id;
    } catch (e) { toast(e.message); }
    GL.ask = null;
    refresh(el);
  },
  glfill: async (el) => {
    try { glSay(await api("ground_fill", { id: +el.dataset.id, blank: el.dataset.blank })); } catch (e) { toast(e.message); }
    GL.ask = null;
    refresh(el);
  },
  gloffer: async (el) => {
    if (GL.ask !== "offer") { GL.ask = "offer"; return refresh(el); }
    GL.ask = null;
    try {
      const r = await api("recruit_offer", { id: +el.dataset.id });
      toast(typeof r === "string" ? r : r.text || r.said || "Offer sent.");
    } catch (e) { toast(e.message); }
    refresh(el);
  },
  gldrop: async (el) => {
    if (GL.ask !== "drop") { GL.ask = "drop"; return refresh(el); }
    GL.ask = null;
    try { glSay(await api("ground_lead_drop", { id: +el.dataset.id })); } catch (e) { toast(e.message); }
    refresh(el);
  },
});
