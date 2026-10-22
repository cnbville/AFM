// Lane screens: money (docs/final-build/plan.md). Registers screens.<name>.
"use strict";
/* Lane 4 · money (final build): later extras topic 1, as each seat sees it.
 *
 *   money         Your club's or school's money. NFL: the club's public
 *                 numbers (Forbes-style revenue and value), the stadium and
 *                 the gate, the owner, the NFLPA report card, and the
 *                 football budgets you spend. College: football's lines of
 *                 the athletic department's budget, the department's
 *                 published report, tickets and donations, projects and the
 *                 ask for a building. High school: the school's football
 *                 budget in full, the equipment budget you spend, the
 *                 fundraisers you run, the district.
 *                 With {kind, id}: another team's published numbers only.
 *   money_league  The league's published money: NFL club values, revenue,
 *                 crowds and report cards; college departments, the
 *                 conferences' media deals and every conference move.
 */

I.ledger = '<path d="M5 3h11l3 3v15H5z"/><path d="M8 9h8M8 13h8M8 17h5"/><path d="M15 3v4h4"/>';
navAdd({ go: "money", label: "Finances", icon: "ledger", sec: "team", after: "games" });
Object.assign(CRUMBS, { money: "Finances", money_league: "League money" });

// ---------- formatting ----------
function mn$(v, { sign = false } = {}) {
  if (v == null || Number.isNaN(v)) return "";
  const a = Math.abs(v), s = v < 0 ? "-" : sign && v > 0 ? "+" : "";
  if (a >= 1e9) return `${s}$${(a / 1e9).toFixed(2)}B`;
  if (a >= 1e7) return `${s}$${Math.round(a / 1e6)}M`;
  if (a >= 1e6) return `${s}$${(a / 1e6).toFixed(1)}M`;
  if (a >= 1e4) return `${s}$${Math.round(a / 1e3)}K`;
  return `${s}$${Math.round(a).toLocaleString()}`;
}
const mnN = (v) => (v == null ? "" : Number(v).toLocaleString());
const mnPct = (v) => (v == null ? "" : `${Math.round(v * 100)}%`);
const mnOrd = (n) => { const s = ["th", "st", "nd", "rd"], v = n % 100; return n + (s[(v - 20) % 10] || s[v] || s[0]); };

// ---------- charts ----------
// A line chart over seasons: each series {name, values:[...], color}; the
// x axis is the years. Hover a point for its value.
function mnLine(years, series, { h = 150, fmt = mn$, zero = false } = {}) {
  const w = 560, padL = 54, padR = 12, padT = 12, padB = 22;
  const all = series.flatMap((s) => s.values).filter((v) => v != null);
  if (years.length < 2 || !all.length) return '<div class="empty">The books open their first year this season: the chart fills as the years close.</div>';
  let lo = zero ? 0 : Math.min(...all), hi = Math.max(...all);
  if (hi === lo) { hi += 1; lo -= zero ? 0 : 1; }
  const pad = (hi - lo) * 0.08; hi += pad; if (!zero) lo -= pad;
  const x = (i) => padL + (i / (years.length - 1)) * (w - padL - padR);
  const y = (v) => padT + (1 - (v - lo) / (hi - lo)) * (h - padT - padB);
  const ticks = [0, 0.5, 1].map((f) => lo + f * (hi - lo));
  const grid = ticks.map((t) => `<line x1="${padL}" x2="${w - padR}" y1="${y(t)}" y2="${y(t)}" class="mn-grid"/><text x="${padL - 6}" y="${y(t) + 4}" class="mn-ax" text-anchor="end">${esc(fmt(t))}</text>`).join("");
  const step = Math.max(1, Math.ceil(years.length / 8));
  const xl = years.map((yr, i) => (i % step === 0 || i === years.length - 1 ? `<text x="${x(i)}" y="${h - 6}" class="mn-ax" text-anchor="middle">${yr}</text>` : "")).join("");
  const lines = series.map((s) => {
    const pts = s.values.map((v, i) => (v == null ? null : `${x(i).toFixed(1)},${y(v).toFixed(1)}`)).filter(Boolean).join(" ");
    const dots = s.values.map((v, i) => (v == null ? "" : `<circle cx="${x(i).toFixed(1)}" cy="${y(v).toFixed(1)}" r="3" fill="${s.color}"><title>${years[i]} · ${esc(s.name)}: ${esc(fmt(v))}</title></circle>`)).join("");
    return `<polyline points="${pts}" fill="none" stroke="${s.color}" stroke-width="2.2" stroke-linejoin="round" stroke-linecap="round"/>${dots}`;
  }).join("");
  const key = series.length > 1 ? `<div class="mn-key">${series.map((s) => `<span><i style="background:${s.color}"></i>${esc(s.name)}</span>`).join("")}</div>` : "";
  return `<svg class="mn-chart" viewBox="0 0 ${w} ${h}" preserveAspectRatio="none" role="img">${grid}${xl}${lines}</svg>${key}`;
}
// Budget against spent so far, one meter a line.
function mnMeters(rows, { note = "spent so far" } = {}) {
  if (!rows || !rows.length) return "";
  return `<div class="mn-meters">${rows.map((r) => {
    const pct = r.budget > 0 ? Math.min(140, (r.spent / r.budget) * 100) : 0;
    const over = r.budget > 0 && r.spent > r.budget * 1.001;
    return `<div class="mn-meter ${over ? "over" : ""}"><span class="mn-mname">${esc(r.name)}</span><span class="mn-bar"><i style="width:${Math.min(100, pct)}%"></i></span><span class="mn-mnum"><b>${mn$(r.spent)}</b> <small>of ${mn$(r.budget)}</small></span></div>`;
  }).join("")}<p class="mn-foot">Budget for the money year, ${esc(note)}.</p></div>`;
}
// A stacked share bar: parts {name, value, color}.
function mnShare(parts) {
  const tot = parts.reduce((a, p) => a + Math.max(0, p.value), 0) || 1;
  return `<div class="mn-share">${parts.filter((p) => p.value > 0).map((p) => `<i style="flex:${p.value / tot};background:${p.color}" title="${esc(p.name)}: ${mn$(p.value)} (${Math.round((p.value / tot) * 100)}%)"></i>`).join("")}</div><div class="mn-key">${parts.filter((p) => p.value > 0).map((p) => `<span><i style="background:${p.color}"></i>${esc(p.name)} <b>${mn$(p.value)}</b></span>`).join("")}</div>`;
}
const MN_C = ["var(--accent)", "var(--gold)", "var(--good)", "#5e9cff", "#bf5af2", "#ff9f0a", "#64d2ff", "var(--fg3)"];
const mnTile = (v, label, title = "") => `<div${title ? ` title="${esc(title)}"` : ""}><b>${v}</b><small>${esc(label)}</small></div>`;

// ---------- the screen ----------
screens.money = async (a) => {
  const x = await api("money_books", a.kind ? { kind: a.kind, id: a.id } : {});
  setColors(x.team.colors);
  const b = x.books;
  if (!b) return `<div class="pad">${pageHead({ title: "Finances" })}<div class="empty">No books for this team yet.</div></div>`;
  const list = x.seat === "nfl" ? (x.own ? ["Overview", "Football budget", "Gate", "Report card"] : ["Overview", "Gate", "Report card"])
    : x.seat === "college" ? (x.own ? ["Overview", "Football budget", "Department", "Gate"] : ["Overview", "Department", "Gate"])
    : x.own ? ["Overview", "Equipment", "Fundraisers", "Ledger"] : ["Overview"];
  const tab = tabOf("mntab", list, a);
  title(x.own ? "Finances" : `${x.team.name}: finances`, tab);
  let body = "";
  if (x.seat === "nfl") body = mnNfl(x, b, tab);
  else if (x.seat === "college") body = mnCollege(x, b, tab);
  else body = mnHs(x, b, tab);
  const sub = x.seat === "nfl" ? (x.own ? "Your club's published numbers, and the football budgets you spend. The owner keeps the rest of the books." : "What is published about the club: estimates, the stadium and the gate.")
    : x.seat === "college" ? (x.own ? "Football's lines of the athletic department's budget, and the department's report as it is published." : b.private ? "A private school: only the federal totals (EADA) are published." : "The athletic department's published report.")
    : x.own ? "Your program's money in full: the district's budget, the gate, the equipment budget you spend and the fundraisers you run." : "A high school's books aren't published.";
  const actions = x.seat === "college" && x.own
    ? `<button class="st-btn small glass" data-act="mnfacility" ${b.asked || b.under_way ? "disabled" : ""} title="${b.under_way ? "A project is under way" : b.asked ? "You asked this year" : "Ask the athletic director for a new football building"}">${icon("ledger")} Ask for a building</button>`
    : "";
  const kicker = teamA(x.team) + (x.own ? ` · ${esc(x.today)}` : "");
  return `<div class="pad mn">${pageHead({ kicker, title: x.own ? "Finances" : `${esc(x.team.name)} finances`, sub, actions, goto: gotoFor("money", a, x) })}${list.length > 1 ? tabs("mntab", list, tab) : ""}<div class="tabbody">${body}</div></div>`;
};
GOTO.money = (a, x) => [
  ["League money", "money_league", { level: x.seat === "college" ? "college" : "nfl" }],
  ["Team page", "team", a.kind ? { kind: a.kind, id: a.id } : {}],
  x.seat === "nfl" && x.own ? ["Contracts", "contracts"] : null,
  ["Schedule", "schedule", a.kind ? { kind: a.kind, id: a.id } : {}],
  ["Money mail", "inbox", { key: "Money" }],
];

// ---------- NFL ----------
function mnNfl(x, b, tab) {
  const p = b.public, st = b.stadium, h = b.history || [];
  const years = h.map((r) => r.year);
  if (tab === "Overview") {
    const tiles = `<div class="st-tiles">${mnTile(mn$(p.value), `club value · ${mnOrd(p.value_rank)} in the league`, "An estimate in the manner of Forbes: revenue times what clubs sell for")}${mnTile(mn$(p.revenue), `revenue · ${mnOrd(p.revenue_rank)}`, "Last year's, estimated")}${mnTile(mn$(p.income, { sign: true }), "operating income")}${mnTile(mnN(p.season_tickets), "season tickets")}${mnTile(mnPct(p.renewal), "renewed")}${mnTile(p.waitlist ? mnN(p.waitlist) : "None", "waiting list")}</div>`;
    const chart = mnLine(years, [{ name: "Value", values: h.map((r) => r.value), color: MN_C[1] }, { name: "Revenue", values: h.map((r) => r.revenue), color: MN_C[0] }], { zero: true });
    const att = mnLine(years, [{ name: "Crowd a game", values: h.map((r) => r.attendance), color: MN_C[2] }, { name: "Seats", values: h.map((r) => r.capacity), color: MN_C[7] }], { fmt: (v) => mnN(Math.round(v)), h: 120 });
    const owner = `<div class="kv"><dt>Owner</dt><dd><b>${esc(b.owner.name)}</b>${b.owner.since ? ` <span class="muted">since ${b.owner.since}</span>` : ""}</dd>${b.owner.words ? `<dt>On money</dt><dd>${esc(b.owner.words)}</dd>` : ""}<dt>Market</dt><dd>${esc(p.city)}${p.state ? ", " + esc(p.state) : ""}</dd>${p.moving ? `<dt>Moving</dt><dd><span class="tag due">To ${esc(p.moving.city)}, ${esc(p.moving.state)} in ${p.moving.year}</span></dd>` : ""}${p.league_revenue ? `<dt>League revenue</dt><dd>${mn$(p.league_revenue)} <span class="muted">shared nationally, plus each club's own</span></dd>` : ""}</div>`;
    const stadium = `<div class="kv"><dt>Stadium</dt><dd><b>${esc(st.name)}</b>${st.dome ? ' <span class="tag soft">Roof</span>' : ""}</dd><dt>Seats</dt><dd>${mnN(st.capacity)}</dd><dt>Opened</dt><dd>${st.opened}${st.renovated > st.opened ? ` · renovated ${st.renovated}` : ""} <span class="muted">(${st.age} years)</span></dd>${st.cost ? `<dt>Built for</dt><dd>${mn$(st.cost)}${st.public ? ` · ${mn$(st.public)} public money` : ""}</dd>` : ""}<dt>Ticket</dt><dd>${mn$(p.price)} on average</dd></div>`;
    const proj = mnProjects(b.projects);
    return `${tiles}<div class="grid2" style="margin-top:18px"><div class="col"><section class="panel"><h2>Value and revenue <small>estimates, by season</small></h2>${chart}</section><section class="panel"><h2>The crowd <small>regular-season average</small></h2>${att}</section></div>
      <div class="col"><section class="panel"><h2>The owner</h2>${owner}</section><section class="panel"><h2>Home</h2>${stadium}</section>${proj}</div></div>`;
  }
  if (tab === "Football budget") {
    const capNote = `The cap this year is ${mn$(b.cap)}; player costs count cash paid, bonuses included.`;
    const spend = b.spend != null ? `<p class="sub">How far the owner goes for football: <b>${b.spend > 0.8 ? "as far as it takes" : b.spend > 0.62 ? "well past the league's middle" : b.spend > 0.45 ? "about the league's middle" : "carefully"}</b>. ${esc(capNote)}</p>` : "";
    return `<div class="grid2"><section class="panel"><h2>The football budget <small>${x.year}</small></h2>${spend}${mnMeters(b.football)}</section>
      <section class="panel"><h2>What it buys</h2><div class="kv"><dt>Players</dt><dd>Salaries, bonuses and the per-game roster bonuses</dd><dt>Benefits</dt><dd>Pension, health, the annuity and the CBA's funds</dd><dt>Coaches</dt><dd>Your staff's pay and any buyouts owed</dd><dt>Scouting</dt><dd>The scouts, their travel, the combine and pro days</dd><dt>Medical</dt><dd>Trainers, doctors, rehab</dd><dt>Travel</dt><dd>Charters and hotels</dd><dt>Amenities</dt><dd>Food, the family room, the locker room: the report card reads them</dd></div></section></div>`;
  }
  if (tab === "Gate") return mnGates(b.gates, h, years);
  return mnCard(b.card, x.own);
}

function mnGates(gates, h, years) {
  const rows = (gates || []).map((g) => [esc(g.date), `${teamChip(g.opp, 18)}`, esc(g.kind), mnN(g.attendance), g.sold_out ? '<span class="tag gold">Sold out</span>' : "", g.blackout ? '<span class="tag due" title="Not sold out 72 hours before kickoff: off local TV (the rule ran 1973-2014)">Blacked out</span>' : ""]);
  const games = rows.length ? `<div class="tbl"><table><thead><tr><th class="l">Date</th><th class="l">Visitor</th><th class="l">Game</th><th>Crowd</th><th></th><th></th></tr></thead><tbody>${rows.map((r) => `<tr>${r.map((c, i) => `<td class="${i < 3 ? "l" : ""}">${c}</td>`).join("")}</tr>`).join("")}</tbody></table></div>` : '<div class="empty">No home games yet this season.</div>';
  const past = h.length ? `<div class="tbl"><table><thead><tr><th class="l">Season</th><th>Crowd a game</th><th>Seats</th><th>Sellouts</th><th>Blackouts</th><th>Ticket</th><th>Renewed</th></tr></thead><tbody>${h.slice().reverse().map((r) => `<tr><td class="l">${r.year}</td><td>${mnN(r.attendance)}</td><td>${mnN(r.capacity)}</td><td>${r.sellouts ?? ""}${r.home_games ? ` of ${r.home_games}` : ""}</td><td>${r.blackouts || ""}</td><td>${mn$(r.price)}</td><td>${mnPct(r.renewal)}</td></tr>`).join("")}</tbody></table></div>` : "";
  return `<div class="grid2"><section class="panel"><h2>This season at home</h2>${games}</section><section class="panel"><h2>Seasons past</h2>${past || '<div class="empty">The first season closes after the Super Bowl.</div>'}</section></div>`;
}

function mnCard(c, own) {
  if (!c) return '<div class="empty">The NFLPA surveys players from 2023: the first report cards come out in late February.</div>';
  if (c.hidden) return `<div class="empty">The ${c.year} report cards stayed inside the union: an arbitrator ruled in 2025 they can't be published. Players and agents still talk.</div>`;
  const tone = (s) => (s >= 10 ? "var(--good)" : s >= 7 ? "var(--fg2)" : s >= 4 ? "var(--gold)" : "var(--live)");
  const rows = c.grades.map((g) => `<div class="mn-grade"><span>${esc(g.part)}</span><b style="color:${tone(g.score)}">${esc(g.grade)}</b></div>`).join("");
  return `<section class="panel"><h2>NFLPA team report card <small>${c.year} · ${mnOrd(c.rank)} of ${c.of}${c.public ? "" : " · the union's own, shared with you"}</small></h2><div class="mn-grades">${rows}</div><p class="sub">Players grade their club every year. Free agents and their agents read it; a bad grade on the owner or the food is the kind of thing an owner hears about.</p></section>`;
}

function mnProjects(list) {
  if (!list || !list.length) return "";
  return `<section class="panel"><h2>Projects</h2><div class="items">${list.map((p) => `<div class="item"><span class="when">${p.done ? "Opened " + p.opens : "Opens " + p.opens}</span><span><b>${esc(p.name)}</b> · ${mn$(p.cost)}${p.public ? ` (${mn$(p.public)} public)` : ""}${p.pledged ? ` · donors ${mn$(p.pledged)}` : ""}${p.capacity ? ` · ${mnN(p.capacity)} seats` : ""}${p.asked ? ' <span class="tag key">You asked</span>' : ""}</span></div>`).join("")}</div></section>`;
}

// ---------- college ----------
function mnCollege(x, b, tab) {
  const h = b.history || [];
  const years = h.map((r) => r.year);
  const t = b.tickets;
  const r = b.report;
  if (tab === "Overview") {
    const total = r || b.eada;
    const tiles = `<div class="st-tiles">${mnTile(total ? mn$(total.revenue) : "-", `department revenue${total ? " · " + total.year : ""}`)}${mnTile(total ? mn$(total.expenses) : "-", "department expenses")}${r ? mnTile(mn$(r.support), "from the university and students", "Institutional support and student fees: the subsidy a department leans on") : ""}${mnTile(mnN(t.attendance), "crowd a home game")}${mnTile(mnN(t.season_tickets), "season tickets")}${mnTile(mnPct(t.renewal), "renewed")}</div>`;
    const chart = mnLine(years, [{ name: "Revenue", values: h.map((y) => y.revenue), color: MN_C[0] }, { name: "Expenses", values: h.map((y) => y.expenses), color: MN_C[7] }, ...(h.some((y) => y.support != null) ? [{ name: "University support", values: h.map((y) => y.support ?? null), color: MN_C[1] }] : [])], { zero: true });
    const conf = `<div class="kv"><dt>Conference</dt><dd><b>${esc(b.conference)}</b> <span class="muted">${esc(b.tier)}</span></dd>${b.deal ? `<dt>Media deal</dt><dd>${mn$(b.deal.per_school)} a school a year, to ${b.deal.until}</dd>` : ""}<dt>What TV pays for you</dt><dd>${mn$(b.media_value)} a year <span class="muted">your brand on a network's books</span></dd><dt>Stadium</dt><dd>${mnN(b.stadium.seats)} seats${b.stadium.crowd_seats < b.stadium.seats ? ` <span class="muted">(crowds draw like ${mnN(b.stadium.crowd_seats)})</span>` : ""}</dd><dt>Ticket</dt><dd>${mn$(t.price)} a game · priority donation ${mn$(t.priority)} a seat</dd>${t.home_games ? `<dt>This season</dt><dd>${t.home_games} home games · ${t.sellouts} sold out</dd>` : ""}</div>`;
    return `${tiles}<div class="grid2" style="margin-top:18px"><div class="col"><section class="panel"><h2>The department <small>by money year (July to June)</small></h2>${chart}</section></div><div class="col"><section class="panel"><h2>Where it stands</h2>${conf}</section>${mnProjects(b.projects)}</div></div>`;
  }
  if (tab === "Football budget") {
    const lvl = b.level > 0.75 ? "among the best funded in the country" : b.level > 0.45 ? "well funded" : b.level > 0.2 ? "about the middle" : "lean";
    return `<div class="grid2"><section class="panel"><h2>Football's budget <small>${x.year}-${String(x.year + 1).slice(2)}</small></h2><p class="sub">Your staff, recruiting and support buy what this pays for: <b>${lvl}</b>.</p>${mnMeters(b.football)}</section>
      <section class="panel"><h2>${esc(b.ad || "Your athletic director")}</h2><p class="sub">${b.under_way ? "A building project is under way: it is on the list on the Overview." : b.asked ? "You asked for a building this year. The answer stands until next year." : "The athletic director decides what gets built, with the donors. You can ask once a year: winning, the department's reserves and the boosters' money make a yes likelier."}</p>
      <div class="kv"><dt>Coaches</dt><dd>Your pay and your staff's</dd><dt>Support staff</dt><dd>Analysts, strength, nutrition, video, recruiting staff</dd><dt>Recruiting</dt><dd>Travel, visits, mail and the off-campus calendar</dd><dt>Scholarships</dt><dd>Tuition, room and board, cost of attendance</dd><dt>Revenue sharing</dt><dd>Payments to players from 2025 (the House settlement)</dd><dt>Guarantees</dt><dd>What you pay visitors to come and play</dd></div></section></div>`;
  }
  if (tab === "Department") {
    if (!r) return `<section class="panel"><h2>${esc(x.team.name)} athletics</h2><div class="empty">${b.private ? `A private school: the department publishes no report. The federal totals (EADA) for ${b.eada ? b.eada.year : "last year"}: revenue ${b.eada ? mn$(b.eada.revenue) : "-"}, expenses ${b.eada ? mn$(b.eada.expenses) : "-"}.` : "The first report is filed when the money year closes in June."}</div></section>`;
    const inc = [["Conference distributions", r.conference], ["Tickets", r.tickets], ["Donations", r.donations], ["NCAA distributions", r.ncaa], ["Media rights and licensing", r.media_rights], ["Guarantees received", r.guarantees_in], ["University and students", r.support]];
    const out = [["Coaches", r.coaches], ["Scholarships", r.scholarships], ["Revenue sharing", r.rev_share], ["Debt on facilities", r.debt]];
    return `<div class="grid2"><section class="panel"><h2>Money in <small>${r.year}-${String(r.year + 1).slice(2)} · ${mn$(r.revenue)}</small></h2>${mnShare(inc.map(([n, v], i) => ({ name: n, value: v, color: MN_C[i] })))}</section>
      <section class="panel"><h2>Money out <small>${mn$(r.expenses)}${r.expenses > r.revenue ? ' · <span style="color:var(--live)">in the red</span>' : ""}</small></h2>${mnShare(out.map(([n, v], i) => ({ name: n, value: v, color: MN_C[i] })).concat([{ name: "Everything else", value: Math.max(0, r.expenses - out.reduce((a, o) => a + o[1], 0)), color: MN_C[7] }]))}</section></div>`;
  }
  // Gate
  const past = h.length ? `<div class="tbl"><table><thead><tr><th class="l">Season</th><th>Crowd a game</th><th>Home games</th><th>Sellouts</th><th>Ticket</th><th>Season tickets</th><th>Renewed</th></tr></thead><tbody>${h.slice().reverse().map((y) => `<tr><td class="l">${y.year}</td><td>${mnN(y.attendance)}</td><td>${y.home_games}</td><td>${y.sellouts}</td><td>${mn$(y.price)}</td><td>${mnN(y.season_tickets)}</td><td>${mnPct(y.renewal)}</td></tr>`).join("")}</tbody></table></div>` : '<div class="empty">The first season closes in June.</div>';
  const att = mnLine(years, [{ name: "Crowd a game", values: h.map((y) => y.attendance), color: MN_C[2] }], { fmt: (v) => mnN(Math.round(v)), h: 120 });
  return `<div class="grid2"><section class="panel"><h2>Seasons past</h2>${past}</section><section class="panel"><h2>The crowd</h2>${att}<p class="sub">Season tickets renew in the spring: the price, last season and the fans' mood decide how many come back. A sellout sets the price up; empty seats hold it.</p></section></div>`;
}

// ---------- high school ----------
// The booster club's account (kept on the school page's desk).
function mnBoosters(k) {
  if (!k) return "";
  return `<section class="panel"><h2>The booster club's account <small>${mnN(k.members)} members</small></h2>
    <div class="st-tiles">${mnTile(mn$(k.balance), "in the bank")}${mnTile(mn$(k.raised), "raised this year")}${mnTile(mn$(k.spent), "spent this year")}</div>
    <p class="sub">${k.fundraised ? `${mn$(k.fundraised)} of it from your team's fundraisers.` : "Your team's fundraisers pay into this account."} The club spends it on meals, helmets, buses and the projects on your wish list.</p></section>`;
}
function mnHs(x, b, tab) {
  if (!x.own) return `<section class="panel"><h2>${esc(x.team.name)}</h2><div class="kv"><dt>Students</dt><dd>${mnN(b.enrollment)}</dd><dt>A Friday crowd</dt><dd>about ${mnN(b.crowd)}</dd><dt>Pay to play</dt><dd>${b.fee ? mn$(b.fee) + " a player" : "No fee"}</dd></div><p class="sub">A school's books aren't published: this is what the town knows.</p></section>`;
  const s = b.seat || {};
  if (tab === "Overview") {
    const inc = b.budget.filter((l) => l.income), out = b.budget.filter((l) => !l.income);
    const sum = (l, k) => l.reduce((a, r) => a + r[k], 0);
    const tiles = `<div class="st-tiles">${mnTile(mn$(sum(inc, "budget")), "football's money this year")}${mnTile(mn$(s.left), "equipment budget left to spend")}${mnTile(mnN(b.crowd), "a Friday crowd")}${mnTile(b.fee ? mn$(b.fee) : "None", "pay to play, a player")}</div>`;
    const last = b.last ? `<section class="panel"><h2>Last year <small>${b.last.year}-${String(b.last.year + 1).slice(2)}</small></h2>${mnShare(b.last.lines.filter((l) => l.income).map((l, i) => ({ name: l.name, value: l.amount, color: MN_C[i] })))}</section>` : "";
    return `${tiles}<div class="grid2" style="margin-top:18px"><div class="col"><section class="panel"><h2>Money in <small>budget and so far</small></h2>${mnMeters(inc, { note: "in so far" })}</section>${last}</div>
      <div class="col"><section class="panel"><h2>Money out</h2>${mnMeters(out)}</section>${mnBoosters(b.boosters)}<section class="panel"><h2>Who pays for what</h2><p class="sub">The district pays the coaches' stipends, buses, officials and the field. The gate goes to the athletic fund and the state association splits playoff gates. The booster club pays for meals, helmets and the projects on your wish list.</p>${linkTo("school", {}, "The booster club and the people you answer to", "gochip")}</section></div></div>`;
  }
  if (tab === "Equipment") {
    const pct = s.equipment ? Math.min(100, ((s.basics + s.bought) / s.equipment) * 100) : 0;
    const items = b.purchases.map((p) => {
      const done = (s.done || []).includes(p.key);
      const can = !done && s.left >= p.price;
      return `<div class="mn-buy ${done ? "done" : ""}"><div><b>${esc(p.name)}</b><small>${esc(p.what)}</small></div><span class="mn-price">${mn$(p.price)}</span>${done ? '<span class="tag gold">Bought</span>' : `<button class="st-btn small ${can ? "primary" : "glass"}" data-act="mnhs" data-what="buy" data-key="${p.key}" ${can ? "" : "disabled"}>Buy</button>`}</div>`;
    }).join("");
    const ask = s.asked ? '<span class="tag soft">Asked this year</span>' : '<button class="st-btn small glass" data-act="mnhs" data-what="district">Ask the district for more</button>';
    return `<div class="grid2"><section class="panel"><h2>The equipment budget <small>${mn$(s.equipment)} from the district</small></h2>
      <div class="mn-meter"><span class="mn-mname">The year's basics</span><span class="mn-bar"><i style="width:${pct}%"></i></span><span class="mn-mnum"><b>${mn$(s.left)}</b> <small>left to spend</small></span></div>
      <p class="sub">About two thirds goes to the basics: pads and cleats replaced, balls, tape, the trainer's kit. The rest is yours to spend on what makes practice better.</p><div class="mn-buys">${items}</div><div class="pp-acts">${ask}</div></section>
      <section class="panel"><h2>Helmets</h2><p class="sub">Helmets are reconditioned and recertified every spring, and none is worn past ten years: the booster club pays for both, and for new helmets each summer. It shows on the club's ledger.</p>${linkTo("school", {}, "The booster club", "gochip")}</section></div>`;
  }
  if (tab === "Fundraisers") {
    const run = (s.running || []).map((r) => `<div class="item"><span class="when">${esc(r.ends)}</span><span><b>${esc(r.name)}</b> under way · counted that day</span></div>`).join("");
    const cards = b.fundraisers.map((f) => {
      const done = (s.done || []).includes(f.key);
      const going = (s.running || []).some((r) => r.key === f.key);
      return `<div class="mn-buy ${done ? "done" : ""}"><div><b>${esc(f.name)}</b><small>${esc(f.what)} · ${f.weeks} week${f.weeks > 1 ? "s" : ""}</small></div><span class="mn-price">${f.take ? `${mn$(f.take[0])}-${mn$(f.take[1])}` : ""}</span>${going ? '<span class="tag key">Running</span>' : done ? '<span class="tag soft">Done this year</span>' : `<button class="st-btn small primary" data-act="mnhs" data-what="fundraiser" data-key="${f.key}" ${(s.running || []).length >= 2 ? "disabled" : ""}>Run it</button>`}</div>`;
    }).join("");
    return `<div class="grid2"><section class="panel"><h2>Fundraisers you run <small>the money goes to the booster club</small></h2><p class="sub">What each brings depends on the town's money and the program's name. Two at a time: the town has only so many weekends.</p><div class="mn-buys">${cards}</div></section>
      <section class="panel"><h2>Under way</h2>${run ? `<div class="items">${run}</div>` : '<div class="empty">Nothing running.</div>'}</section></div>`;
  }
  const led = (s.ledger || []).map((e) => `<div class="item"><span class="when">${esc(e.date)}</span><span>${esc(e.what)} <b style="color:${e.amount < 0 ? "var(--live)" : "var(--good)"}">${mn$(e.amount, { sign: true })}</b></span></div>`).join("");
  return `<section class="panel"><h2>What you ran, bought and asked for</h2>${led ? `<div class="items">${led}</div>` : '<div class="empty">Nothing yet.</div>'}</section>`;
}

// ---------- the league's money ----------
screens.money_league = async (a) => {
  const x = await api("money_league", a.level ? { level: a.level } : {});
  const lvl = x.level;
  title("League money", lvl === "college" ? "College" : "NFL");
  const sw = `<div class="tabsline">${[["nfl", "NFL"], ["college", "College"]].map(([k, t]) => `<button data-go="money_league" data-args='${esc(JSON.stringify({ level: k }))}' aria-selected="${lvl === k}">${t}</button>`).join("")}</div>`;
  let body;
  if (lvl === "college") {
    const cols = [
      { k: "team", t: "School", l: true, f: (r) => teamChip(r.team, 18), v: (r) => r.team.name },
      { k: "conference", t: "Conference", l: true },
      { k: "division", t: "Level", l: true },
      { k: "revenue", t: "Revenue", f: (r) => mn$(r.revenue) },
      { k: "expenses", t: "Expenses", f: (r) => mn$(r.expenses) },
      { k: "support", t: "University support", f: (r) => (r.support == null ? '<span class="muted" title="Private: not published">-</span>' : mn$(r.support)) },
      { k: "attendance", t: "Crowd", f: (r) => mnN(r.attendance) },
    ];
    const deals = x.deals.length ? `<div class="items">${x.deals.map((d) => `<div class="item"><span class="when">to ${d.until}</span><span><b>${esc(d.conference)}</b> · ${mn$(d.per_school)} a school a year</span></div>`).join("")}</div>` : '<div class="empty">The real deals run through 2026; the world negotiates its own after.</div>';
    const moves = x.moves.length ? `<div class="items">${x.moves.map((m) => `<div class="item"><span class="when">${esc(m.date)}</span><span>${esc(m.text)}</span></div>`).join("")}</div>` : '<div class="empty">No conference moves this year.</div>';
    body = `<div class="grid2"><section class="panel"><h2>Athletic departments <small>FBS and FCS, last money year</small></h2>${table("mnleaguecol", cols, x.rows, { sort: "revenue" })}</section><div class="col"><section class="panel"><h2>Media deals</h2>${deals}</section><section class="panel"><h2>Conference moves <small>this year</small></h2>${moves}</section></div></div>`;
  } else {
    const cols = [
      { k: "team", t: "Club", l: true, f: (r) => teamChip(r.team, 18), v: (r) => r.team.name },
      { k: "value", t: "Value", f: (r) => mn$(r.value) },
      { k: "revenue", t: "Revenue", f: (r) => mn$(r.revenue) },
      { k: "income", t: "Operating income", f: (r) => mn$(r.income) },
      { k: "attendance", t: "Crowd", f: (r) => mnN(r.attendance) },
      { k: "capacity", t: "Seats", f: (r) => mnN(r.capacity) },
      { k: "stadium", t: "Stadium", l: true, hide: true },
      { k: "opened", t: "Opened", hide: true },
      { k: "owner", t: "Owner", l: true },
      { k: "card", t: "Report card", f: (r) => (r.card ? mnOrd(r.card) : "") },
      { k: "blackouts", t: "Blackouts", hide: true },
    ];
    const lr = x.league || [];
    const chart = mnLine(lr.map((r) => r.year), [{ name: "League revenue", values: lr.map((r) => r.revenue), color: MN_C[0] }], { zero: true, h: 120 });
    body = `<section class="panel"><h2>The clubs <small>estimates in the manner of Forbes${x.card_year ? ` · report card ranks from ${x.card_year}` : ""}</small></h2>${table("mnleaguenfl", cols, x.rows, { sort: "value" })}${x.cards_internal ? '<p class="sub">From 2025 the report cards stay inside the union.</p>' : ""}</section><section class="panel" style="margin-top:18px"><h2>The league's revenue</h2>${chart}</section>`;
  }
  return `<div class="pad mn">${pageHead({ title: "League money", sub: lvl === "college" ? "Published numbers only: private schools don't publish theirs." : "What is published about the clubs: values, revenue, crowds and the players' report cards." })}${sw}<div class="tabbody">${body}</div></div>`;
};

Object.assign(CORE_MORE, {
  mnfacility: async (el) => {
    try { const r = await api("money_ask_facility"); toast(r.text); refresh(el); } catch (e) { toast(e.message); }
  },
  mnhs: async (el) => {
    try { const r = await api("money_hs_act", { what: el.dataset.what, key: el.dataset.key || "" }); toast(r.text); refresh(el); } catch (e) { toast(e.message); }
  },
});
