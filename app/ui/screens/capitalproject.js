/* Capital's project screen (lane CS4; docs/design/capital.md CP24, CP26,
 * CP37, CP39, board 4). The one building going up for your program: the
 * answer a cost overrun needs (cut rooms, wait, or borrow, each shown before
 * you confirm), the rooms it holds and what each gives, where it stands
 * against the day it opens, the temporary space the team works in meanwhile,
 * how it is paid and the strings on it (names on the building, each donor's
 * gift and the call he may make about it).
 *
 *   capitalproject   no arguments.
 *
 * It reads `capital_project` and answers with `capital_overrun` {fix} and
 * `capital_catch` {id, yes}. No dollar figures (CP39): the cost reads in
 * shares and words. Every class is cpj- so the app's global names never
 * restyle it. */
"use strict";

I.cpjproject = '<path d="M3.5 20.5h17"/><path d="M6 20.5V10h12v10.5"/><path d="M9 10V6.5h6V10"/><path d="M10 14h4M10 17h4"/>';
navAdd({ go: "capitalproject", label: "Project", icon: "cpjproject", sec: "capital", seats: ["college"] });
CRUMBS.capitalproject = "Project";

const CPJ = { pick: -1, said: "" };

const cpjDay = (iso, opt) => capDate(iso, opt || { month: "short", year: "numeric" });

screens.capitalproject = async () => {
  let x;
  try { x = await api("capital_project", {}); } catch (e) {
    title("Project", "");
    return `<div class="pad sl cpj"><section class="cpj-card"><div class="cpj-k">Capital</div><p class="cpj-empty">${esc(e.message || String(e))}</p></section></div>`;
  }
  title("Project", x.program);
  const b = x.building;
  const said = CPJ.said ? `<div class="cpj-said"><b>${esc(CPJ.said)}</b></div>` : "";
  CPJ.said = "";
  const head = (name) => `<div class="cpj-top"><div class="cpj-t"><div class="cpj-k">Capital · ${esc(x.program)}</div><h1>${esc(name)}</h1></div>${capTabs("capitalproject")}<small class="cpj-dim">${esc(capDate(x.today, { weekday: "short", month: "long", day: "numeric", year: "numeric" }))}</small></div>`;
  if (!b) {
    CPJ.pick = -1;
    const msg = x.ready
      ? "No building is going up. A drive breaks ground when its pledges reach four fifths of the goal, or half if your AD borrows. The building's rooms, its timeline and any cost overrun come here."
      : "Buildings begin with the first season of your career. Until then they come from the department's old decisions.";
    const drive = screens.capitaldrive ? '<button type="button" class="cpj-link" data-go="capitaldrive">Open the drive</button>' : "";
    return `<div class="pad sl cpj">${said}${head("Project")}<section class="cpj-card"><div class="cpj-k">Nothing under way</div><p class="cpj-empty">${esc(msg)}</p>${drive}</section></div>`;
  }
  const d = b.decision;
  if (!d || CPJ.pick >= d.options.length || (CPJ.pick >= 0 && !d.options[CPJ.pick].can)) CPJ.pick = -1;
  const pick = d ? d.options[CPJ.pick] : null;
  return `<div class="pad sl cpj">${said}${head(b.title)}${cpjDecision(b)}${cpjAnswered(b)}<div class="cpj-cols"><section class="cpj-card cpj-rooms">${cpjRooms(b, pick)}</section><div class="cpj-side">${cpjTimeline(b, pick, x.today)}${cpjWithout(b)}${cpjPaid(b)}${cpjStrings(b, x.head)}</div></div></div>`;
};

// ---- the answer an overrun needs ----
function cpjDecision(b) {
  const d = b.decision;
  if (!d) return "";
  const opts = d.options.map((o, i) => `<button type="button" class="cpj-opt${i === CPJ.pick ? " sel" : ""}" data-act="cpjpick" data-i="${i}" aria-pressed="${i === CPJ.pick}"${o.can && d.canAnswer ? "" : " disabled"}><b>${esc(o.title)}</b><span>${esc(o.what)}</span><small>${esc(o.cost)}</small></button>`).join("");
  const o = CPJ.pick >= 0 ? d.options[CPJ.pick] : null;
  const act = !d.canAnswer
    ? '<p class="cpj-dim sm">The head coach answers the builder.</p>'
    : o ? `<div class="cpj-go"><button type="button" class="st-btn glass cpj-confirm" data-act="cpjfix" data-fix="${o.key}">Confirm: ${esc(o.title.toLowerCase())}</button><small class="cpj-dim">Nothing changes until you confirm.</small></div>`
      : '<p class="cpj-dim sm">Pick an answer to see what it does to the rooms and the date.</p>';
  return `<section class="cpj-card cpj-dec"><div class="cpj-dh"><div class="cpj-k amb">Decision · costs ran over</div><span class="cpj-dim">The builder found it ran ${d.share}% over the cost, on ${esc(cpjDay(d.found, { month: "long", day: "numeric" }))}. Your AD takes the quiet answer, a year's wait, if you leave it past ${esc(cpjDay(d.by, { month: "long", day: "numeric" }))}.</span></div><div class="cpj-opts">${opts}</div>${act}</section>`;
}

function cpjAnswered(b) {
  const a = b.answered;
  if (!a) return "";
  return `<section class="cpj-card cpj-ans"><div class="cpj-k">Costs ran ${a.share}% over</div><p>${esc(a.words)}</p></section>`;
}

// ---- the rooms ----
function cpjRooms(b, pick) {
  const drops = pick && pick.key === "cut" ? pick.drops : [];
  const head = '<div class="cpj-rm cpj-hd"><span>Room</span><span>Today to opening</span><span>What it gives you</span></div>';
  const rows = b.rooms.map((r) => {
    const gone = r.cut || drops.includes(r.room);
    const note = r.cut ? "cut, moved to the open plot" : gone ? "would be cut, moved to the open plot" : r.new ? "new" : r.from ? `moves from the ${r.from}` : "moves in";
    return `<div class="cpj-rm${gone ? " gone" : ""}"><span class="nm"><b>${esc(capCap(r.name))}</b><small>${esc(capCap(note))}${r.temporary && !gone ? " · the team works without it meanwhile" : ""}</small></span>${capPips(r.level, gone ? r.level : r.target)}<span class="gv">${esc(capCap(r.unlocks))}</span></div>`;
  }).join("");
  return `<div class="cpj-k">The rooms it holds</div>${head}${rows}`;
}

// ---- where it stands ----
function cpjTimeline(b, pick, todayIso) {
  const t0 = Date.parse(b.ground + "T12:00:00");
  const wait = pick && pick.key === "wait";
  const t1 = Date.parse(b.ready + "T12:00:00");
  const t2 = wait ? t1 + 365 * 86400000 : t1;
  const span = Math.max(t2 - t0, 1);
  const at = (t) => Math.max(0, Math.min(100, ((t - t0) / span) * 100));
  const today = at(Date.parse(todayIso + "T12:00:00"));
  const found = b.found ? at(Date.parse(b.found + "T12:00:00")) : null;
  const endPct = at(t1);
  const left = (p) => `left:${p}%`;
  const label = `${wait ? "Opens for the " + (b.opens + 1) : "Opens for the " + b.opens} season`;
  return `<section class="cpj-card"><div class="cpj-k">Timeline</div>
    <div class="cpj-tl" role="img" aria-label="Ground broke ${esc(cpjDay(b.ground))}; ${esc(label)}">
      <span class="lb" style="left:0">Ground broke</span><span class="lb t" style="${left(Math.min(Math.max(today, 14), 82))}">Today</span><span class="lb e" style="right:0">Opens</span>
      <span class="bar done" style="left:0;width:${today}%"></span><span class="bar todo" style="${left(today)};width:${Math.max(endPct - today, 0)}%"></span>${wait ? `<span class="bar late" style="${left(endPct)};width:${100 - endPct}%"></span>` : ""}
      ${found != null ? `<span class="pin" style="${left(found)}" title="Costs ran over"></span>` : ""}
      <span class="tk" style="left:0">${esc(cpjDay(b.ground))}</span><span class="tk" style="right:0">${esc(cpjDay(wait ? new Date(t2).toISOString().slice(0, 10) : b.ready))}</span></div>
    <p class="cpj-dim sm">${esc(label)}.${b.paid.setBack ? " Pledges were pulled, and it opens a year later." : ""}${wait ? " A year later while the drive raises the overrun." : ""}</p></section>`;
}

function cpjWithout(b) {
  const rooms = b.without;
  if (!rooms.length) {
    return `<section class="cpj-card"><div class="cpj-k">While it goes up</div><p>No room closes for it. The team keeps its facilities all the way.</p></section>`;
  }
  return `<section class="cpj-card"><div class="cpj-k">While it goes up</div><p>The team works without the ${esc(rooms.join(", the "))} until it opens.</p><p class="amb sm">Those rooms read a level lower for their tools, and the team's mood is a little down.</p></section>`;
}

function cpjPaid(b) {
  const p = b.paid;
  return `<section class="cpj-card"><div class="cpj-k">How it is paid</div><p>Donors' pledges cover ${p.pledged}% of the cost${p.bonds ? "; the athletic department's bonds cover part of the rest, and football's staff pool is smaller for years" : "; the department and the university carry the rest"}.</p></section>`;
}

// ---- the strings ----
function cpjStrings(b, head) {
  const names = b.names.map((n) => n.down
    ? `<p class="cpj-dim"><b>${esc(capCap(n.words))}</b> came down ${esc(cpjDay(n.down))}.</p>`
    : `<p><b>${esc(capCap(n.words))}</b> has his name up${n.donor ? `, ${esc(n.donor)}'s` : ""}.</p>`).join("");
  const gifts = b.gifts.map((g) => cpjGift(g, head)).join("");
  const more = b.moreGifts > 0 ? `<p class="cpj-dim sm">And ${b.moreGifts} smaller gift${b.moreGifts === 1 ? "" : "s"}.</p>` : "";
  if (!names && !gifts) return `<section class="cpj-card"><div class="cpj-k">Strings on this building</div><p class="cpj-dim">No donor has put a name or a condition on it.</p></section>`;
  return `<section class="cpj-card"><div class="cpj-k">Strings on this building</div>${names}${gifts}${more}</section>`;
}

function cpjGift(g, head) {
  const tied = g.tied ? " · tied to you" : "";
  const who = `<b>${esc(g.name)}</b>${g.room ? ` · ${esc(g.room)}` : ""}<small>${g.share ? g.share + "% of the goal" : "under 1% of the goal"}${tied}</small>`;
  if (g.state === "call") {
    const c = g.call;
    const risk = c.mayPull ? " He likes to collect: a no may cost you the rest of his pledge." : " A no cools him toward the staff.";
    const wince = c.adWinces ? " Your AD won't like a donor picking coordinators." : "";
    const btns = head ? `<div class="cpj-cb"><button type="button" class="st-btn glass gold" data-act="cpjcatch" data-id="${c.id}" data-yes="1">Grant it</button><button type="button" class="st-btn glass" data-act="cpjcatch" data-id="${c.id}" data-yes="0">Turn it down</button></div>` : "";
    return `<div class="cpj-gift call"><div>${who}</div><p><b class="amb">He's called:</b> he'd like ${esc(c.ask)}. Answer by ${esc(cpjDay(c.by, { month: "long", day: "numeric" }))}, or your AD answers for you.<small>A yes warms him.${esc(risk)}${esc(wince)}</small></p>${btns}</div>`;
  }
  const line = {
    pulled: '<b class="amb">Pulled</b> the rest of his pledge.',
    granted: `You granted him ${esc(g.call && g.call.ask)}.`,
    refused: `You turned him down on ${esc(g.call && g.call.ask)}.`,
    quiet: g.read === "Likes to collect." ? "Gave without strings so far. Your development director expects a call within a year." : g.read === "Likes to be thanked." ? "Gave without strings. He'll want to be thanked." : "Gave without strings, and lets go.",
  }[g.state];
  return `<div class="cpj-gift${g.state === "pulled" ? " pulled" : ""}"><div>${who}</div><p class="cpj-dim">${line}</p></div>`;
}

// ---- the buttons ----
const cpjDone = async (el, p) => {
  try {
    const r = await p;
    if (r && r.said) CPJ.said = r.said;
  } catch (e) { toast(e.message || String(e)); }
  CPJ.pick = -1;
  refresh(el);
};
Object.assign(CORE_MORE, {
  cpjpick: (el) => { const i = Number(el.dataset.i); CPJ.pick = CPJ.pick === i ? -1 : i; refresh(el); },
  cpjfix: (el) => cpjDone(el, api("capital_overrun", { fix: el.dataset.fix })),
  cpjcatch: (el) => cpjDone(el, api("capital_catch", { id: Number(el.dataset.id), yes: el.dataset.yes === "1" })),
});
