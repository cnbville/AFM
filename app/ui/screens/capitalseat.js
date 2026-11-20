/* Capital: Your seat (lane CS6; docs/design/capital.md CP32-CP37, CP39,
 * board 6). The rooms of the seats that have no donors.
 *
 *   High school: bond night (where the district's vote stands, what moved
 *   it, your work in town, the ask to the board), the booster club's builds
 *   and your school's five rooms.
 *   NFL: the players' report card by room, your leverage with the owner and
 *   the rooms he may be asked for (he is asked in the owner meeting).
 *
 *   capitalseat   no arguments.
 *
 * It reads `capital_seat` and answers with `capital_bond_ask` and
 * `capital_town_work` {work} (lane_capital_seat.rs). No dollar figures
 * (CP39): a vote reads in percent and points, a room in levels, a lean in
 * words. Every class is cps- so the app's global names never restyle it. */
"use strict";

I.capseat = '<path d="M4 20.5h16"/><path d="M6 20.5V9l6-4.5L18 9v11.5"/><path d="M9.5 20.5v-6h5v6"/>';
navAdd({ go: "capitalseat", label: "Your seat", icon: "capseat", sec: "capital", seats: ["hs", "nfl"] });
CRUMBS.capitalseat = "Your seat";

const CPS = { said: "", ask: false };
const CPS_GRADES = ["F-", "F", "D-", "D", "D+", "C-", "C", "C+", "B-", "B", "B+", "A-", "A+"];

const cpsPts = (p) => `${p > 0 ? "+" : p < 0 ? "-" : ""}${Math.abs(p) % 1 ? Math.abs(p).toFixed(1) : Math.abs(p)}`;
const cpsTone = (p) => (p > 0 ? "up" : p < 0 ? "down" : "");

screens.capitalseat = async () => {
  let x;
  try { x = await api("capital_seat", {}); } catch (e) {
    title("Your seat", "");
    return `<div class="pad sl cps"><section class="cps-card"><div class="cps-k">Capital</div><p class="cps-empty">${esc(e.message || String(e))}</p></section></div>`;
  }
  const hs = x.seat === "hs";
  title("Your seat", hs ? x.school : x.club);
  const said = CPS.said ? `<div class="cps-said"><b>${esc(CPS.said)}</b></div>` : "";
  CPS.said = "";
  const head = `<div class="cps-top"><div class="cps-t"><div class="cps-k">Capital · ${esc(hs ? `${x.school} · ${x.town}` : x.club)}</div><h1>Your seat</h1></div>${capTabs("capitalseat")}<small class="cps-dim">${esc(capDate(x.today, { weekday: "short", month: "long", day: "numeric", year: "numeric" }))}</small></div>`;
  return `<div class="pad sl cps">${said}${head}${hs ? cpsHigh(x) : cpsPro(x)}</div>`;
};

// ---------------------------------------------------------------- high school

function cpsHigh(x) {
  const main = x.bond ? cpsBond(x) : cpsNoBond(x);
  return `<div class="cps-cols"><div class="cps-main">${main}${cpsRooms(x)}</div><div class="cps-side">${cpsBoosters(x)}${cpsIf(x)}</div></div>`;
}

function cpsBond(x) {
  const b = x.bond;
  const day = capDate(b.day_iso, { weekday: "long", month: "long", day: "numeric", year: "numeric" });
  const what = b.package === "new"
    ? `A new stadium for ${b.seats.toLocaleString("en-US")}, with a turf field and a field house under the stands.`
    : `New stands, lights and a new turf field, for about ${b.seats.toLocaleString("en-US")}.`;
  const shared = b.shared_with.length ? ` Both schools would play there: you and ${cpsList(b.shared_with)}.` : "";
  const top = `<div><div class="cps-k">${b.yes != null ? "The vote" : "Bond night"} · ${esc(day)} · ${esc(x.town)}</div><h2>${esc(capCap(b.name))}</h2><p class="cps-dim">${esc(what)}${esc(shared)}</p></div>`;
  if (b.yes != null) {
    const opens = b.opens ? `It opens in ${b.opens}.` : "";
    return `<section class="cps-card cps-main-card">${top}<p class="cps-result"><b class="up">Passed</b> with ${b.yes}% of the vote. ${esc(opens)}</p>${cpsWork(x)}</section>`;
  }
  const need = b.needs > 50 ? `It needs ${b.needs}% to pass.` : "It needs more than half.";
  const last = x.last ? ` The last vote here, in ${x.last.year}, ${x.last.passed ? "passed" : "failed"}${x.last.yes != null ? ` with ${x.last.yes}%` : ""}.` : "";
  const bar = `<div class="cps-poll"><div class="cps-row"><span>Yes today, in the district's poll</span><b class="gold">${b.poll}%</b></div>
    <div class="cps-bar" role="img" aria-label="Yes ${b.poll} percent in the poll, ${b.needs} percent needed"><i style="width:${Math.min(Math.max(b.poll, 0), 100)}%"></i><u style="left:${b.needs}%"></u></div>
    <small class="cps-dim">${esc(b.outlook)} ${esc(need)}${esc(last)} Election night adds its own luck.</small></div>`;
  const rows = b.moved.length
    ? b.moved.map((m) => `<div class="cps-ev"><span class="cps-dim">${esc(m.when || "This year")}</span><span>${esc(m.what)}</span><b class="${cpsTone(m.points)}">${cpsPts(m.points)}</b></div>`).join("")
    : '<p class="cps-dim">Nothing has moved it yet. Every town starts a little over half for it.</p>';
  return `<section class="cps-card cps-main-card">${top}${bar}<div class="cps-k">What moved the vote</div><div class="cps-evs">${rows}</div><small class="cps-dim">Points of the vote. Every town starts a little over half for a stadium.</small>${cpsWork(x)}</section>`;
}

function cpsWork(x) {
  const w = x.work;
  if (!w) return "";
  const list = w.list.map((k) => {
    const btn = `<button type="button" class="st-btn glass${k.can ? " gold" : ""}" data-act="cpswork" data-work="${esc(k.key)}"${k.can ? "" : " disabled"}${k.why ? ` title="${esc(k.why)}"` : ""}>Do it</button>`;
    return `<div class="cps-ev cps-wk"><span><b>${esc(capCap(k.words))}</b><small class="cps-dim">${k.hours} hours of your week · ${cpsPts(k.points)} point${k.points === 1 ? "" : "s"} · done ${k.done} of ${k.times}${k.why ? ` · ${esc(k.why)}` : ""}</small></span>${btn}</div>`;
  }).join("");
  const note = w.open
    ? "One slot a week. Each kind helps only so often."
    : `The campaign opens ten weeks before election day, on ${capDate(w.opens, { month: "long", day: "numeric" })}.`;
  return `<div class="cps-k">Your work in town</div><div class="cps-evs">${list}</div><small class="cps-dim">${esc(note)}</small>`;
}

function cpsNoBond(x) {
  const last = x.last ? `The last vote here, in ${x.last.year}, ${x.last.passed ? "passed" : "failed"}${x.last.yes != null ? ` with ${x.last.yes}%` : ""}.` : "No stadium bond has come to a vote here yet.";
  const mates = x.district.filter((n) => n !== x.school);
  const dist = mates.length ? `Your district's schools share a stadium: you and ${cpsList(mates)}.` : "Yours is the only public school in its district.";
  let act;
  if (x.can_ask) {
    act = CPS.ask
      ? `<div class="cps-go"><button type="button" class="st-btn glass cps-confirm" data-act="cpsaskgo">Confirm: ask the board</button><button type="button" class="st-btn glass" data-act="cpsask">Not now</button><small class="cps-dim">${esc(x.odds)} You can ask once a year.</small></div>`
      : `<div class="cps-go"><button type="button" class="st-btn glass gold" data-act="cpsask">Ask the board for a stadium bond</button><small class="cps-dim">${esc(x.odds)}</small></div>`;
  } else {
    act = `<p class="cps-dim">${esc(x.why_not || "")}</p>`;
  }
  return `<section class="cps-card cps-main-card"><div><div class="cps-k">Bond night · ${esc(x.town)}</div><h2>No bond is on the ballot</h2><p class="cps-dim">${esc(dist)} ${esc(last)}</p></div><p>When the board calls one, it goes to the town on a real election day, and the vote reads your seasons, the crowds and the work you do in town.</p>${act}</section>`;
}

function cpsRooms(x) {
  const rows = x.rooms.map((r) => `<div class="cps-rm"><span class="nm"><b>${esc(capCap(r.name))}</b><small class="cps-dim">${r.since ? `Built or renovated ${r.since}` : r.level ? "Standing" : "None yet"}</small></span>${capPips(r.level)}<span class="gv">${esc(r.unlocks)}</span></div>`).join("");
  return `<section class="cps-card"><div class="cps-k">Your school's rooms</div>${rows}</section>`;
}

function cpsBoosters(x) {
  const b = x.boosters;
  if (!b) return `<section class="cps-card"><div class="cps-k">The booster club's builds</div><p class="cps-dim">The booster club's list comes with your school's desk.</p></section>`;
  const built = b.built.length
    ? b.built.map((p) => `<p><b>${p.year}</b> · ${esc(p.name)}</p>`).join("")
    : '<p class="cps-dim">The club has not paid for a room yet.</p>';
  const next = b.next
    ? `<p class="cps-dim">Next on its list: ${esc(b.next.name.toLowerCase())}. The club ${esc(b.next.has)}.</p>`
    : '<p class="cps-dim">Nothing on its wish list is a room. Add one at the school desk.</p>';
  return `<section class="cps-card"><div class="cps-k">The booster club's builds</div>${built}${next}</section>`;
}

function cpsIf(x) {
  const b = x.bond;
  if (!b) {
    return `<section class="cps-card"><div class="cps-k">If a bond passes</div><p>A new stadium takes your stadium room to the top; a renovation lifts it a level and a half. The field is turf either way.</p><p class="cps-dim">Your crosstown rivals in the district get the same stadium.</p></section>`;
  }
  const open = b.opens || b.would_open;
  const lines = [
    b.package === "new" ? "Your stadium room goes to the top level, and the new field house lifts your locker room." : "Your stadium room rises a level and a half, and the field is turf.",
    `${b.yes != null ? "It opens" : "It would open"} in the fall of ${open}.`,
  ];
  const mates = b.shared_with.length ? `<p class="cps-amb">${esc(cpsList(b.shared_with))} ${b.shared_with.length > 1 ? "get" : "gets"} the same stadium.</p>` : "";
  return `<section class="cps-card"><div class="cps-k">${b.yes != null ? "What it brings" : "If it passes"}</div>${lines.map((l) => `<p>${esc(l)}</p>`).join("")}${mates}</section>`;
}

const cpsList = (a) => (a.length <= 1 ? a.join("") : `${a.slice(0, -1).join(", ")} and ${a[a.length - 1]}`);

// ------------------------------------------------------------------------ NFL

function cpsPro(x) {
  return `<div class="cps-cols"><section class="cps-card cps-main-card">${cpsCard(x)}</section><div class="cps-side">${cpsLeverage(x)}${cpsAsk(x)}${cpsBuild(x)}</div></div>`;
}

function cpsCard(x) {
  const head = '<div class="cps-gr cps-hd"><span>Room</span><span>Grade</span><span>What players said</span></div>';
  const rows = x.rooms.map((r) => {
    const g = r.grade ? CPS_GRADES.indexOf(r.grade) : -1;
    const tone = g < 0 ? "" : g <= 4 ? "down" : g >= 9 ? "up" : "";
    const tail = r.said ? esc(r.said) : `<span class="cps-dim">${esc(r.unlocks)}</span>`;
    return `<div class="cps-gr"><span class="nm"><b>${esc(capCap(r.name))}</b>${capPips(r.level)}</span><span class="g ${tone}" title="${esc(r.card_line || "Not on the card")}">${r.grade ? esc(r.grade) : "-"}<small>${r.card_line ? esc(r.card_line) : "Not on the card"}</small></span><span class="sd">${tail}</span></div>`;
  }).join("");
  return `<div><div class="cps-k">The players' report card, by room · ${esc(x.club)}, ${x.year}</div><p class="cps-dim">Only your players and the agents see these grades. Free agents' agents read them when they pick a club.</p></div>${head}${rows}`;
}

function cpsLeverage(x) {
  const w = x.why || {};
  const bits = [];
  if (w.record) bits.push(`${w.record}${w.note ? `, ${w.note}` : ""}${w.year ? ` (${w.year})` : ""}.`);
  bits.push(w.wanted ? "Another club has wanted you this past year." : "No other club has asked for you this past year.");
  return `<section class="cps-card"><div class="cps-k">Your leverage with the owner</div><div class="cps-lev">${esc(x.leverage)}</div><div class="cps-bar" role="img" aria-label="${esc(x.leverage)}"><i style="width:${Math.round(x.share * 100)}%"></i></div><p class="cps-dim">${esc(bits.join(" "))}</p></section>`;
}

function cpsAsk(x) {
  let body;
  if (x.asked) {
    body = `<p>You asked for the ${esc(x.asked.room)} this league year, and he said ${x.asked.yes ? "yes" : "no"}.</p>${x.asked.yes ? "" : '<p class="cps-dim">You can ask again next league year.</p>'}`;
  } else if (x.build) {
    body = '<p class="cps-dim">A renovation is already under way. The owner will not take another ask until it opens.</p>';
  } else if (x.offers.length) {
    body = x.offers.map((o) => `<div class="cps-of"><span><b>${esc(capCap(o.name))}</b><small class="cps-dim">${esc(o.unlocks)}</small></span>${capPips(o.level)}<span class="cps-lean">${esc(o.chance)}</span></div>`).join("")
      + '<p class="cps-dim sm">You raise it at your owner meeting. He hears one room a league year, and a better leverage makes a yes easier.</p>';
  } else {
    body = '<p class="cps-dim">Every room is as good as it gets, or the rooms are not live yet. There is nothing to ask for.</p>';
  }
  const quote = x.owner ? `<p class="cps-dim sm">${esc(x.owner)}</p>` : "";
  return `<section class="cps-card"><div class="cps-k">Your ask</div>${body}${quote}</section>`;
}

function cpsBuild(x) {
  const b = x.build;
  if (!b) return "";
  return `<section class="cps-card"><div class="cps-k">Under way</div><p>The ${esc(cpsList(b.rooms))} ${b.rooms.length > 1 ? "are" : "is"} being redone. ${b.asked ? "You asked for it. " : ""}It opens ${esc(b.opens)}.</p></section>`;
}

// ---------------------------------------------------------------- the buttons

const cpsDone = async (el, p) => {
  try {
    const r = await p;
    if (r && r.said) CPS.said = r.said;
  } catch (e) { toast(e.message || String(e)); }
  CPS.ask = false;
  refresh(el);
};
Object.assign(CORE_MORE, {
  cpsask: (el) => { CPS.ask = !CPS.ask; refresh(el); },
  cpsaskgo: (el) => cpsDone(el, api("capital_bond_ask", {})),
  cpswork: (el) => cpsDone(el, api("capital_town_work", { work: el.dataset.work })),
});
