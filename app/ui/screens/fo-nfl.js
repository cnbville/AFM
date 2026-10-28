// Front office screens: the NFL desk (lane 10). Your front office's home,
// the cap sheet by year, expiring deals with tags and tenders, the
// negotiation room, free agency, the trade desk and roster moves (the cut
// to 53, the practice squad, the waiver wire). Every number is the
// league's or your staff's read; a player's hidden wants never show.
"use strict";

foNavHide("contracts", "nfl");
foNavAdd("nfl", [
  ["fo_office", "Front office", "office"],
  ["fo_cap", "Cap sheet", "cap"],
  ["fo_expiring", "Deals and tags", "tag"],
  ["fo_market", "Free agency", "fa"],
  ["fo_trades", "Trades", "trade"],
  ["fo_moves", "Roster moves", "cut"],
]);
Object.assign(CRUMBS, {
  fo_office: "Front office", fo_cap: "Cap sheet", fo_expiring: "Deals and tags", fo_talk: "Negotiation",
  fo_market: "Free agency", fo_trades: "Trades", fo_moves: "Roster moves",
});

// ---------- small pieces ----------
const foM = (v) => (v == null ? "-" : Math.abs(v) < 500 ? "$0" : money(v));
const foPct = (v) => Math.round((v || 0) * 100) + "%";
// Where each desk screen leads (lane 9's "Go to" line under the head).
GOTO.fo_office = () => [["Cap sheet", "fo_cap"], ["Deals and tags", "fo_expiring"], ["Free agency", "fo_market"], ["Trades", "fo_trades"], ["Roster moves", "fo_moves"], ["Staff", "staff"], ["Money mail", "inbox", { key: "Money" }]];
GOTO.fo_cap = () => [["Squad", "roster"], ["Free agency", "fo_market"], ["Trades", "fo_trades"], ["Draft board", "draftboard"]];
GOTO.fo_expiring = () => [["Front office", "fo_office"], ["Cap sheet", "fo_cap"], ["Free agency", "fo_market"], ["Squad", "roster"]];
GOTO.fo_talk = (a) => [["Player page", "player", { id: a.id }], ["Deals and tags", "fo_expiring"], ["Free agency", "fo_market"], ["Cap sheet", "fo_cap"]];
GOTO.fo_market = () => [["Front office", "fo_office"], ["Cap sheet", "fo_cap"], ["Deals and tags", "fo_expiring"], ["Squad", "roster"], ["Depth chart", "depth"]];
GOTO.fo_trades = () => {
  const tr = foTrade();
  return [["Front office", "fo_office"], ["Cap sheet", "fo_cap"], tr.team != null ? ["Their club", "team", { kind: "nfl", id: tr.team }] : null, ["Squad", "roster"]];
};
GOTO.fo_moves = () => [["Front office", "fo_office"], ["Depth chart", "depth"], ["Squad", "roster"], ["Cap sheet", "fo_cap"]];
function foHead(team, title, sub, extra = "") {
  if (team) setColors(team.colors);
  return `<div class="fo-head">${team ? mark(team, 48) : ""}<div><div class="st-label">${team ? esc(team.name) : ""}</div><div class="h1">${esc(title)}</div>${sub ? `<div class="sub">${sub}</div>` : ""}</div>${extra}</div>${gotoFor(S.route.screen, S.route.args)}`;
}
function foBtn(fo, text, data = {}, cls = "glass") {
  return `<button class="st-btn small ${cls}" data-act="fo" data-fo="${fo}"${Object.entries(data).map(([k, v]) => ` data-${k}="${esc(v)}"`).join("")}>${esc(text)}</button>`;
}
function foOwner(key, owner) {
  return `<div class="seg fo-owner" title="Who decides this for your club"><button data-act="fo" data-fo="owner" data-duty="${key}" data-owner="you" aria-pressed="${owner === "you"}">You</button><button data-act="fo" data-fo="owner" data-duty="${key}" data-owner="staff" aria-pressed="${owner === "staff"}">Your staff</button></div>`;
}
function foPick(p) {
  return `<span class="fo-pick ${p.comp ? "comp" : ""}" title="${esc(p.text)} · ${p.points} point${p.points === 1 ? "" : "s"} on your chart">${mark(p.from, 18)}<b>${p.year} R${p.round}</b>${p.own ? "" : `<small>${esc(p.from.name.split(" ").pop())}</small>`}${p.comp ? "<small>comp</small>" : ""}</span>`;
}
const foKindWords = { ReSign: "Re-signing", Extension: "Extension", FreeAgent: "Free agent", OfferSheet: "Offer sheet" };
const foStateWords = { Open: "Talking", Standing: "Offer on the table", Agreed: "Agreed", Signed: "Signed", Broken: "Talks broken off", Gone: "Gone" };
function foTalkChip(t) {
  if (!t) return '<span class="muted">-</span>';
  const cls = { Agreed: "good", Signed: "good", Standing: "gold", Broken: "live" }[t.state] || "";
  return `<span class="st-pill ${cls}">${esc(foStateWords[t.state] || t.state)}</span>`;
}

// ---------- Front office home ----------
screens.fo_office = async () => {
  const x = await api("fo_office");
  const c = x.cap || {};
  const r = x.roster || {};
  const alerts = [
    x.expiring ? [`${x.expiring} expiring deal${x.expiring === 1 ? "" : "s"}`, "Re-sign, tag, tender or let them walk", "fo_expiring"] : null,
    x.calls ? [`${x.calls} trade call${x.calls === 1 ? "" : "s"} waiting`, "Other clubs want an answer", "fo_trades"] : null,
    x.talks ? [`${x.talks} open talk${x.talks === 1 ? "" : "s"}`, "Agents waiting on your next offer", "fo_market"] : null,
  ].filter(Boolean);
  const duties = x.duties.map((d) => `<tr><td class="l"><b>${esc(d.name)}</b></td><td class="l wrap fo-staffdoes">${esc(d.staff)}</td><td>${foOwner(d.key, d.owner)}</td></tr>`).join("");
  const ahead = x.ahead.map((e) => `<div class="item cal"><span class="when">${esc(e.date)}<br><small>${e.in === 0 ? "today" : "in " + e.in + " d"}</small></span><span><a class="link" data-go="${e.screen}">${esc(e.text)}</a></span></div>`).join("") || '<div class="empty">No dates left this league year.</div>';
  const log = x.log.map((m) => `<div class="item"><span class="when">${esc(m.date)}</span><span>${m.id ? `<a class="link" data-go="player" data-id="${m.id}">${esc(m.text)}</a>` : esc(m.text)}</span></div>`).join("") || '<div class="empty">Nothing yet. Every move you make is written here.</div>';
  return `<div class="pad fo">
    ${foHead(x.team, "Front office", "You are the head coach and general manager. Each duty is yours or your staff's: whatever you hand over, the person hired for it decides by his own judgment, and whatever you keep waits for you.")}
    <div class="st-tiles fo-tiles">
      <div><b class="${c.room < 0 ? "neg" : ""}">${foM(c.room)}</b><small>cap room this year${c.reserve ? ` · your cap manager would keep ${foM(c.reserve)} of it for the season` : ""}</small></div>
      <div><b>${foM(c.cap)}</b><small>salary cap${c.carryover ? ` + ${foM(c.carryover)} carried over` : ""}</small></div>
      <div><b>${foM(c.dead)}</b><small>dead money${c.voids ? ` · <span class="neg">${foM(c.voids)} more at the league year</span> as void years void` : ""}</small></div>
      <div><b>${foM(c.next_room)}</b><small>room next year so far</small></div>
      <div><b>${r.on ?? "-"}<span class="fo-of">/${r.limit ?? ""}</span></b><small>on the roster</small></div>
      <div><b>${r.squad ?? "-"}<span class="fo-of">/${r.squad_limit ?? ""}</span></b><small>practice squad · ${r.reserve ?? 0} on reserve</small></div>
    </div>
    ${alerts.length ? `<div class="fo-alerts">${alerts.map(([t, s, g]) => `<button class="st-banner" data-go="${g}"><span class="st-mark" style="background:var(--gold);color:#000">!</span><span><b>${esc(t)}</b><small>${esc(s)}</small></span></button>`).join("")}</div>` : ""}
    <div class="grid2">
      <div class="panel"><h2>Who handles what <small title="Your own rating: it reads agents' claims and sets how far you can push">${linkTo("you", {}, `Your negotiating ${x.gm ? x.gm.negotiating : "-"}/20`)}</small></h2>
        <div class="tbl"><table><thead><tr><th class="l">Duty</th><th class="l">What your staff does with it</th><th>Decides</th></tr></thead><tbody>${duties}</tbody></table></div></div>
      <div style="display:grid;gap:18px">
        <div class="panel"><h2>The league year ahead</h2><div class="items">${ahead}</div></div>
        <div class="panel"><h2>Your moves</h2><div class="items fo-log">${log}</div></div>
      </div>
    </div>
  </div>`;
};

// ---------- Cap sheet ----------
screens.fo_cap = async (a) => {
  const x = await api("fo_cap");
  const yrs = x.years;
  // A link from his contract, the Squad or the cap board marks his row (the
  // arrow keys go on from it) and brings it into view.
  const him = a && a.id != null ? x.rows.find((r) => r.id === a.id) : null;
  if (him) {
    him.cls = "kbd";
    setTimeout(() => { const tr = document.querySelector('.tbl[data-table="fo_cap"] tr.kbd'); if (tr) tr.scrollIntoView({ block: "nearest" }); }, 30);
  }
  const book = x.books.map((b) => {
    const used = Math.min(1, (b.committed + b.dead) / (b.cap + (b.carryover || 0)));
    const dead = b.dead / (b.cap + (b.carryover || 0));
    return `<div class="fo-book"><div class="st-label">${b.year}</div><b class="${b.room < 0 ? "neg" : ""}">${foM(b.room)}</b><small>room</small>
      <div class="fo-bar"><i style="width:${Math.max(0, (used - dead) * 100)}%"></i><i class="dead" style="width:${dead * 100}%"></i></div>
      <small>${foM(b.committed)} committed · ${b.contracts} deals${b.dead ? ` · ${foM(b.dead)} dead` : ""}</small><small class="muted">cap ${foM(b.cap)}${b.carryover ? ` + ${foM(b.carryover)} carried over` : ""}${b.reserve ? ` · ${foM(b.reserve)} kept for the season` : ""}</small>${b.voids ? `<small class="neg" title="Deals that ran out still spread bonus over void years: when the new league year voids them, what's left lands on this year's cap">${foM(b.voids)} more lands at the league year (void years)</small>` : ""}</div>`;
  }).join("");
  const hit = (h) => (h == null ? '<span class="muted">-</span>' : h.void ? `<span class="fo-void" title="A void year: only the bonus spread lands here">${foM(h.hit)}</span>` : `<span title="Base ${foM(h.base)} · bonus ${foM(h.bonus)} · roster and workout ${foM(h.roster)}${h.gtd ? " · guaranteed " + foM(h.gtd) : ""}" class="${h.gtd >= h.base && h.base > 0 ? "fo-gtd" : ""}">${foM(h.hit)}</span>`);
  const cols = [
    { k: "name", t: "Player", l: true, f: (r) => `${playerLink(r.id, r.name, "Contract")}${r.no_trade ? ' <span class="st-pill" title="No-trade clause">NTC</span>' : ""}` },
    { k: "pos", t: "Pos", l: true },
    { k: "age", t: "Age" },
    { k: "ovr", t: "Rating", v: (r) => mid(r.ovr), f: (r) => range(r.ovr, null, { p: r.id, k: "ovr" }) },
    ...yrs.map((y, k) => ({ k: "y" + y, t: String(y), div: k === 0, v: (r) => (r.hits[k] ? r.hits[k].hit : -1), f: (r) => hit(r.hits[k]) })),
    { k: "kind", t: "Deal", l: true, div: true, f: (r) => `${esc(r.kind)}${r.ending ? ' <span class="st-pill gold">Ends</span>' : ""}` },
    { k: "apy", t: "A year", f: (r) => `<span title="${r.years_left} more years · ${foM(r.gtd_left)} still guaranteed">${foM(r.apy)}</span>` },
    { k: "gtd_left", t: "Gtd left", tip: "Pay still guaranteed from this league year on", f: (r) => (r.gtd_left ? foM(r.gtd_left) : '<span class="muted">-</span>') },
    { k: "dead", t: "Dead if cut", div: true, v: (r) => r.cut.dead, f: (r) => foM(r.cut.dead) },
    { k: "saves", t: "Saves", tip: "Cap room a release frees this year", v: (r) => r.cut.saves, f: (r) => `<span class="${r.cut.saves < 0 ? "neg" : "pos"}">${foM(r.cut.saves)}</span>` },
    { k: "act", t: "", f: (r) => (r.where === "Practice squad" ? `<span class="muted">Squad</span>` : foBtn("capmove", "Moves", { id: r.id, name: r.name, voids: x.voids_ok ? 1 : 0, re: r.can_restructure ? 1 : 0, dead: r.cut.dead, saves: r.cut.saves, dj: r.cut.dead_june1, dj2: r.cut.dead_june1_next, sj: r.cut.saves_june1, june1: x.after_june1 ? 0 : 2 - x.june1_used })) },
  ];
  const dead = x.dead.map((d) => `<tr><td class="l">${d.year}</td><td class="l">${playerLink(d.id, d.name)}</td><td class="l">${esc(d.why)}</td><td>${foM(d.amount)}</td></tr>`).join("") || `<tr><td class="l" colspan="4"><div class="empty">No dead money on the books.</div></td></tr>`;
  const picks = x.picks.map(foPick).join("") || '<span class="muted">No picks.</span>';
  return `<div class="film"><div class="pad fo">
    ${foHead(x.team, "Cap sheet", `Every contract's cap number for five league years, with what a release costs and saves today. ${x.top51 ? "<b>The rule of 51 is on:</b> until the first game only the 51 biggest cap numbers count. " : ""}${x.after_june1 ? "After June 1 a release splits its dead money over two years by itself." : `Post-June 1 designations used: ${x.june1_used} of 2.`}`)}
    <div class="fo-books">${book}</div>
    <div class="fo-legend"><span><i class="fo-gtd">Underlined</i> fully guaranteed</span><span><i class="fo-void">In italics</i> a void year: only the bonus spread</span><span><i class="neg">In red</i> a release costs room</span><span>Hover a cap number for its parts.</span></div>
    ${table("fo_cap", cols, x.rows, { sort: "y" + yrs[0], groups: [{ t: "", span: 4 }, { t: "Cap number by league year", span: yrs.length, div: true }, { t: "The deal", span: 3, div: true }, { t: `Released today${x.after_june1 ? " (split)" : ""}`, span: 3, div: true }] })}
    <div class="grid2">
      <div class="panel"><h2>Dead money <small>charge by charge</small></h2><div class="tbl"><table><thead><tr><th class="l">Year</th><th class="l">Player</th><th class="l">Why</th><th>Amount</th></tr></thead><tbody>${dead}</tbody></table></div></div>
      <div class="panel"><h2>Your draft picks <small>the next three drafts</small></h2><div class="fo-picks">${picks}</div></div>
    </div>
  </div></div>`;
};

// ---------- Deals and tags ----------
screens.fo_expiring = async () => {
  const x = await api("fo_expiring");
  const act = (r) => {
    const b = [];
    if (r.talk && r.talk.state !== "Signed") b.push(foBtn("talk", "Talks", { id: r.id }, "primary"));
    else if (r.status_fa === "Unrestricted" || r.extend || r.status_fa === "Rights held") b.push(foBtn("talk", r.extend ? "Extend" : "Re-sign", { id: r.id }, "primary"));
    if (r.tags && x.window) b.push(foBtn("tag", "Tag", { id: r.id, name: r.name, f: r.tags.franchise, x: r.tags.exclusive, t: r.tags.transition }));
    if (r.tenders && x.before_league_year) b.push(foBtn("tender", "Tender", { id: r.id, name: r.name, a: r.tenders.first, b: r.tenders.second, c: r.tenders.original, drafted: r.tenders.drafted || 0 }));
    if (r.erfa != null && x.before_league_year) b.push(foBtn("erfa", "Tender (ERFA)", { id: r.id, name: r.name, p: r.erfa }));
    if (r.tagged) b.push(foBtn("rescind", x.before_league_year ? "Withdraw" : "Take back", { id: r.id, name: r.name, before: x.before_league_year ? 1 : 0, what: /tag/i.test(r.tagged) ? "tag" : "tender" }));
    if (r.option) b.push(`<div class="seg fo-mini"><button data-act="fo" data-fo="option" data-id="${r.id}" data-take="1" aria-pressed="${r.option_choice === true}">Pick up</button><button data-act="fo" data-fo="option" data-id="${r.id}" data-take="0" aria-pressed="${r.option_choice === false}">Decline</button></div>`);
    if (r.sheet) b.push(`<div class="seg fo-mini"><button data-act="fo" data-fo="sheet" data-id="${r.id}" data-m="1" aria-pressed="${r.sheet.choice === true}">Match</button><button data-act="fo" data-fo="sheet" data-id="${r.id}" data-m="0" aria-pressed="${r.sheet.choice === false}">Let him go</button></div>`);
    return b.join(" ");
  };
  const what = (r) => {
    if (r.sheet) return `<b>Offer sheet</b> from ${teamLink(r.sheet.team)}: ${r.sheet.years} ${r.sheet.years === 1 ? "yr" : "yrs"} at ${foM(r.sheet.apy)} · until ${esc(r.sheet.until)}`;
    if (r.tagged) return `<b>${esc(r.tagged)}</b>`;
    if (r.option) return `Fifth-year option: ${foM(r.option_price)}${r.option_choice == null ? " · <span class=\"gold\">undecided</span>" : r.option_choice ? " · picking it up" : " · declining"}`;
    if (r.extend) return `Through ${r.through}`;
    if (r.tags) return `<span title="Exclusive ${foM(r.tags.exclusive)} · transition ${foM(r.tags.transition)}">Tag ${foM(r.tags.franchise)}</span>`;
    if (r.tenders) return `<span title="Second round ${foM(r.tenders.second)} · ${r.tenders.drafted ? "original round" : "right of first refusal"} ${foM(r.tenders.original)}">Tenders from ${foM(r.tenders.original)}</span>`;
    if (r.erfa != null) return `Exclusive rights: a minimum tender ${foM(r.erfa)} keeps him`;
    return "";
  };
  const cols = [
    { k: "name", t: "Player", l: true, f: (r) => playerLink(r.id, r.name) },
    { k: "pos", t: "Pos", l: true },
    { k: "age", t: "Age" },
    { k: "accrued", t: "Seasons", tip: "Accrued seasons, last season's counted" },
    { k: "ovr", t: "Rating", v: (r) => mid(r.ovr), f: (r) => range(r.ovr, null, { p: r.id, k: "ovr" }) },
    { k: "market", t: "His market", div: true, tip: "Your cap manager's read of what he'd get a year", v: (r) => r.market[0], f: (r) => `${foM(r.market[0])}-${foM(r.market[1])}` },
    { k: "apy", t: "Now a year", f: (r) => foM(r.apy) },
    { k: "what", t: "Where it stands", l: true, div: true, wrap: true, v: (r) => r.order, f: (r) => `${what(r)} ${r.talk ? foTalkChip(r.talk) : ""}` },
    { k: "act", t: "", l: true, f: act },
  ];
  return `<div class="film"><div class="pad fo">
    ${foHead(x.team, "Deals and tags", `Players whose deals end at the new league year (${esc(x.league_year)}), rookies with a fifth-year option, and stars due an extension. ${x.window ? `<b>The tag window is open until ${esc(x.deadline)}.</b>` : `The tag window runs ${esc(x.window_open)} to ${esc(x.deadline)}.`} ${x.one_tag ? "One tag a club a year." : "A franchise and a transition tag a club a year."} Undecided options are declined on ${esc(x.option_deadline)}. Cap room ${foM(x.room)}.`)}
    ${table("fo_exp", cols, x.rows, { groupBy: (r) => r.group, empty: "Nobody's deal is running out, and no option or extension is due." })}
  </div></div>`;
};

// ---------- The negotiation room ----------
function foDraft(t) {
  // The offer you're writing: kept per player until you send it.
  return keep("fodraft:" + t.id, (() => {
    const a = (t.talk && (t.talk.offer || t.talk.ask)) || null;
    const base = a ? { ...a } : { years: 3, apy: Math.max(t.minimum, t.market[0]), gtd: 0.3, bonus: 0.15, void_years: 0, incentives: 0, no_trade: false, start: false, ramp: 0.06, low_first: false };
    return { years: base.years, apy: base.apy, gtd: base.gtd, bonus: base.bonus, void_years: base.void_years || 0, incentives: base.incentives || 0, no_trade: !!base.no_trade, start: !!base.start, ramp: base.ramp ?? 0.06, low_first: !!base.low_first };
  })());
}
screens.fo_talk = async (a) => {
  const t = await api("fo_talk", { id: a.id });
  setColors(t.team ? t.team.colors : null);
  const d = foDraft(t);
  let pv = null;
  try { pv = await api("fo_preview", { id: t.id, ...d }); } catch (e) { pv = null; }
  const ag = t.agent;
  const ask = t.talk && t.talk.ask;
  const lines = (t.lines || []).map((l) => `<div class="fo-line ${l.ours ? "ours" : "theirs"}"><small>${esc(l.date)} · ${l.ours ? "You" : ag ? esc(ag.name) : "His agent"}</small><div>${esc(l.text)}</div></div>`).join("") || `<div class="empty">No talks yet. Your first offer opens them.</div>`;
  const yearsRows = pv ? pv.years.map((y) => `<tr class="${y.void ? "fo-voidrow" : ""}"><td class="l">${y.year}${y.void ? ' <span class="st-pill">void</span>' : ""}</td><td>${foM(y.base)}</td><td>${foM(y.bonus)}</td><td>${foM(y.other)}</td><td><b>${foM(y.hit)}</b></td><td class="muted">${foPct(y.share)}</td><td>${foM(y.gtd + y.injury)}</td><td class="${y.room != null && y.room < 0 ? "neg" : "muted"}">${y.room != null ? foM(y.room) : ""}</td></tr>`).join("") : "";
  const opt = (n, v) => `<option value="${n}" ${n === v ? "selected" : ""}>${n}</option>`;
  const live = t.kind != null;
  const answer = S.keep.foanswer && S.keep.foanswer.id === t.id ? S.keep.foanswer.a : null;
  return `<div class="hero small"><div class="art"></div><div class="st-outline">${esc(t.pos)}</div><div class="copy">
    <div class="st-label" style="color:rgba(255,255,255,.7)">${esc(foKindWords[t.kind] || "No talks possible")} · ${t.team ? teamLink(t.team) : "Free agent"}</div>
    <h1>${playerLink(t.id, t.name)}</h1>
    <div class="meta"><span>Age <b>${t.age}</b></span><span>${t.accrued} accrued seasons</span><span>Rating ${range(t.ovr, null, { p: t.id, k: "ovr" })}</span>${t.contract ? `<span>${esc(t.contract.kind)}, ${foM(t.contract.apy)} a year through ${t.contract.through}</span>` : ""}</div></div></div>
    <div class="pad fo"><p class="sub">${esc(t.kind_words)}</p>
    ${answer ? `<div class="fo-answer ${answer.verdict}"><div class="st-label">${esc({ Signed: "Deal", Agreed: "Agreed", Standing: "On the table", Counter: "Counter", Broken: "Talks ended" }[answer.verdict] || answer.verdict)} · ${esc(answer.gap)}</div><b>${esc(answer.text)}</b>${answer.claim ? `<div>${esc(answer.claim)}</div><small>${esc(answer.read || "")}</small>` : ""}</div>` : ""}
    <div class="grid2">
      <div class="panel"><h2>Your offer <small>${t.voids_ok ? "void years allowed in this era" : "no void years in this era"}</small></h2>
        <div class="fo-form">
          <label>Years<select data-fo-d="years">${[1, 2, 3, 4, 5, 6, 7].map((n) => opt(n, d.years)).join("")}</select></label>
          <label>New money a year ($M)<input type="number" step="0.25" min="0" data-fo-d="apy" value="${(d.apy / 1e6).toFixed(2)}"></label>
          <label>Guaranteed (of the total)<input type="range" min="0" max="100" step="5" data-fo-d="gtd" value="${Math.round(d.gtd * 100)}"><b>${foPct(d.gtd)}</b></label>
          <label>Signing bonus (part of it)<input type="range" min="0" max="${Math.round(d.gtd * 100)}" step="5" data-fo-d="bonus" value="${Math.round(Math.min(d.bonus, d.gtd) * 100)}"><b>${foPct(Math.min(d.bonus, d.gtd))}</b></label>
          ${t.voids_ok ? `<label>Void years<select data-fo-d="void_years">${[0, 1, 2, 3, 4].map((n) => opt(n, d.void_years)).join("")}</select></label>` : ""}
          <label>Shape<select data-fo-d="ramp">${[[0, "Even"], [0.06, "Rising (6% a year)"], [0.15, "Back-loaded (15%)"], [0.25, "Steeply back-loaded (25%)"]].map(([v, n]) => `<option value="${v}" ${Math.abs(v - d.ramp) < 0.001 ? "selected" : ""}>${n}</option>`).join("")}</select></label>
          <label>Incentives a year ($M)<input type="number" step="0.25" min="0" data-fo-d="incentives" value="${(d.incentives / 1e6).toFixed(2)}"></label>
          <label class="fo-check" title="Real deals keep year one low: a big bonus and a base near the minimum"><input type="checkbox" data-fo-d="low_first" ${d.low_first ? "checked" : ""}> Year one's base at the minimum</label>
          <label class="fo-check"><input type="checkbox" data-fo-d="no_trade" ${d.no_trade ? "checked" : ""}> No-trade clause</label>
          <label class="fo-check"><input type="checkbox" data-fo-d="start" ${d.start ? "checked" : ""}> Promise him a starting job</label>
        </div>
        ${pv ? `<div class="st-tiles"><div><b>${foM(pv.total)}</b><small>total</small></div><div><b>${foM(pv.guaranteed)}</b><small>guaranteed</small></div><div><b>${foM(pv.bonus)}</b><small>signing bonus</small></div><div><b>${foM(pv.first_cash)}</b><small>cash in year one</small></div></div>
        <div class="tbl"><table><thead><tr><th class="l">Year</th><th>Base</th><th>Bonus</th><th>Roster etc.</th><th>Cap number</th><th>Of cap</th><th>Guaranteed</th><th title="Your cap room once this deal is on the books">Room after</th></tr></thead><tbody>${yearsRows}</tbody></table></div>
        ${pv.over ? `<div class="fo-warn">Year one is ${foM(pv.over)} over your cap room. A bigger signing bonus, a back-loaded shape${t.voids_ok ? " or void years" : ""} lower it; so does clearing room on the cap sheet.</div>` : ""}` : ""}
        <div class="st-actions">${live ? foBtn("offer", "Make the offer", { id: t.id }, "primary") : ""} ${ask ? foBtn("meet", "Write in his ask", { id: t.id }) : ""} ${t.can_visit ? foBtn("visit", "Bring him in for a visit", { id: t.id }) : ""}</div>
        <small class="sub">Minimum for his seasons ${foM(t.minimum)} · cap ${foM(t.cap)} · your room ${foM(t.room)} this year, ${foM(t.room_next)} next.</small>
      </div>
      <div style="display:grid;gap:18px">
        <div class="panel"><h2>What you know <small title="Your own rating: it reads agents' claims and sets how far you can push">${linkTo("you", {}, `Your negotiating ${t.negotiating}/20`)}</small></h2>
          <div class="kv"><dt>His market</dt><dd>${foM(t.market[0])} to ${foM(t.market[1])} a year <small class="sub">· your cap manager's read</small></dd>
          ${ask ? `<dt>His agent asks</dt><dd><b>${ask.years} ${ask.years === 1 ? "yr" : "yrs"}, ${foM(ask.apy)} a year</b>, ${foPct(ask.gtd)} guaranteed${ask.start ? ", a starting job" : ""}</dd>` : ""}
          ${t.talk ? `<dt>The talks</dt><dd>${foTalkChip(t.talk)} <span class="fo-patience" title="How much patience his side has left"><i style="width:${Math.round(t.talk.patience * 100)}%"></i></span></dd>` : ""}
          <dt>What he wants</dt><dd>${t.wants ? esc(t.wants) : '<span class="muted">A visit, or a few rounds of talks, would tell you.</span>'}</dd></div></div>
        <div class="panel"><h2>His agent</h2>${ag ? `<div class="kv"><dt>Agent</dt><dd><b>${esc(ag.name)}</b>, ${esc(ag.agency)}</dd><dt>Agency</dt><dd>${esc(ag.size)}, ${ag.clients} client${ag.clients === 1 ? "" : "s"}</dd><dt>With your club</dt><dd>${esc(ag.history)}</dd></div>` : '<div class="sub">He represents himself.</div>'}</div>
        <div class="panel"><h2>The talks</h2><div class="fo-lines">${lines}</div></div>
      </div>
    </div>
  </div>`;
};

// ---------- Free agency ----------
screens.fo_market = async () => {
  const f = keep("fomarket", { pos: "", street: false, mine: false });
  const x = await api("fo_free_agents", { pos: f.pos || undefined, street: f.street });
  let rows = x.rows;
  if (f.mine) rows = rows.filter((r) => r.talk);
  const status = (r) => {
    if (r.agreed) {
      const a = r.agreed;
      const tip = a.verbal ? `A verbal agreement from the negotiating window: he signs when the league year opens unless a clearly better offer sways him.` : `He likes this offer and decides by ${a.until}. A clearly better one can still sway him.`;
      const who = a.mine ? "you" : a.team.name;
      return `<span class="st-pill ${a.mine ? "good" : ""}" title="${esc(tip)}">${a.verbal ? `Verbal: ${esc(who)}` : `Leaning to ${esc(who)}`}</span>${a.verbal ? "" : ` <small class="muted">by ${esc(a.until.replace(/^\w+ /, "").replace(/ \d{4}$/, ""))}</small>`}`;
    }
    if (r.kind === "OfferSheet") return `<span class="st-pill">Restricted (${esc(r.rfa || "")})</span>`;
    if (!r.kind) return `<span class="muted" title="Released: he is free now, and his market opens with every other club's on ${esc(x.league_year)}">Released · talks from ${esc(x.league_year.replace(/^\w+ /, "").replace(/ \d{4}$/, ""))}</span>`;
    if (r.kind === "ReSign") return '<span class="muted">Yours until the league year</span>';
    return '<span class="muted">No deal yet</span>';
  };
  const cols = [
    { k: "name", t: "Player", l: true, f: (r) => `${playerLink(r.id, r.name)}${r.visited ? ' <span class="st-pill" title="He visited your building">Visited</span>' : ""}` },
    { k: "pos", t: "Pos", l: true },
    { k: "age", t: "Age" },
    { k: "accrued", t: "Seasons" },
    { k: "ovr", t: "Rating", tip: "The league's public read", v: (r) => mid(r.ovr), f: (r) => range(r.ovr, null, { p: r.id, k: "ovr" }) },
    { k: "former", t: "Last club", l: true, div: true, v: (r) => (r.former ? r.former.name : ""), f: (r) => (r.former ? teamLink(r.former) : '<span class="muted">-</span>') },
    { k: "market", t: "His market", v: (r) => r.market[0], f: (r) => `${foM(r.market[0])}-${foM(r.market[1])}` },
    { k: "status", t: "Market", l: true, v: (r) => (r.agreed ? 1 : 0), f: status },
    { k: "talk", t: "Your talks", l: true, div: true, v: (r) => (r.talk ? 1 : 0), f: (r) => foTalkChip(r.talk) },
    { k: "act", t: "", f: (r) => (r.kind ? foBtn("talk", r.talk ? "Talks" : "Make an offer", { id: r.id }, r.talk ? "glass" : "primary") : "") },
  ];
  const posSeg = `<div class="seg"><button data-act="fo" data-fo="mpos" data-v="" aria-pressed="${!f.pos}">All</button>${x.positions.map((p) => `<button data-act="fo" data-fo="mpos" data-v="${p}" aria-pressed="${f.pos === p}">${p}</button>`).join("")}</div>`;
  return `<div class="film"><div class="pad fo">
    ${foHead(x.team, "Free agency", `<b>${esc(x.phase)}.</b> The negotiating window opens ${esc(x.opens)}; the league year ${esc(x.league_year)}. Your offers sit beside every other club's, and each player takes the one that fits what he wants. Room ${foM(x.room)} · roster ${x.roster} of ${x.limit}.`, foOwner("free_agency", x.owner))}
    <div class="filters">${posSeg}<div class="seg"><button data-act="fo" data-fo="mstreet" aria-pressed="${f.street}">Include the street</button><button data-act="fo" data-fo="mmine" aria-pressed="${f.mine}">Your talks only</button></div></div>
    ${table("fo_market", cols, rows, { sort: "market", empty: "Nobody on the market fits." })}
  </div></div>`;
};

// ---------- Trades ----------
function foTrade() {
  return keep("fotrade", { team: null, give: [], givePicks: [], get: [], getPicks: [], cond: null, answer: null, shop: null });
}
const foPickKey = (p) => `${p.year}:${p.round}:${p.orig}:${p.comp ? 1 : 0}`;
screens.fo_trades = async (a = {}) => {
  const x = await api("fo_trades");
  const tr = foTrade();
  // Opened on a club or a player (minor bug 71): "Trade for him", a club's
  // menu, the cap board's "Trade now". Read once, so later picks stand.
  if (a.team != null || a.get || a.give) {
    const t = a.team != null ? Number(a.team) : tr.team;
    if (t !== tr.team) Object.assign(tr, { get: [], getPicks: [], cond: null, answer: null });
    tr.team = t;
    for (const id of a.get || []) if (!tr.get.includes(Number(id))) tr.get.push(Number(id));
    for (const id of a.give || []) if (!tr.give.includes(Number(id))) tr.give.push(Number(id));
    delete a.team; delete a.get; delete a.give;
  }
  if (tr.team == null && x.teams.length) tr.team = x.teams[0].ref.id;
  const other = tr.team != null ? await api("fo_trade_team", { team: tr.team }) : null;
  const pts = (list, sel, key) => list.filter((p) => sel.includes(key(p))).reduce((s, p) => s + (p.points || 0), 0);
  const giveP = x.roster.filter((r) => tr.give.includes(r.id));
  const getP = other ? other.rows.filter((r) => tr.get.includes(r.id)) : [];
  const givePts = giveP.reduce((s, r) => s + r.points, 0) + pts(x.picks, tr.givePicks, foPickKey);
  const getPts = getP.reduce((s, r) => s + r.points, 0) + (other ? pts(other.picks, tr.getPicks, foPickKey) : 0);
  const pickBox = (list, sel, side) => list.map((p) => `<label class="fo-pickbox ${sel.includes(foPickKey(p)) ? "on" : ""}"><input type="checkbox" data-fo-pick="${side}" data-k="${foPickKey(p)}" ${sel.includes(foPickKey(p)) ? "checked" : ""}>${foPick(p)}<small>${p.points}</small></label>`).join("") || '<span class="muted">No picks to trade.</span>';
  const plist = (rows, sel, side) => `<div class="tbl fo-tradelist"><table><thead><tr><th></th><th class="l">Player</th><th class="l">Pos</th><th>Age</th><th>Rating</th><th>Cap hit</th><th title="His value over his contract, on your chart">Value</th></tr></thead><tbody>${rows.slice().sort((a, b) => sel.includes(b.id) - sel.includes(a.id) || mid(b.ovr) - mid(a.ovr)).map((r) => `<tr class="${sel.includes(r.id) ? "on" : ""}"><td><input type="checkbox" data-fo-pl="${side}" data-id="${r.id}" ${sel.includes(r.id) ? "checked" : ""} ${r.no_trade ? 'title="No-trade clause"' : ""}></td><td class="l">${playerLink(r.id, r.name)}${r.no_trade ? ' <span class="st-pill">NTC</span>' : ""}${r.block ? ' <span class="st-pill gold">Block</span>' : ""}</td><td class="l">${esc(r.pos)}</td><td>${r.age}</td><td>${range(r.ovr, null, { p: r.id, k: "ovr" })}</td><td>${foM(r.hit)}</td><td class="${r.points < 0 ? "neg" : ""}">${r.points}</td></tr>`).join("")}</tbody></table></div>`;
  const calls = x.calls.filter((c) => !c.answered).map((c) => `<div class="fo-call">${mark(c.team, 40)}<div><b>${teamLink(c.team)} call about ${c.player ? playerLink(c.player.id, c.player.name) : "a player"}</b><small>They offer ${c.picks.map(foPick).join(" ")} · answer by ${esc(c.until)}</small></div><div class="st-actions">${foBtn("call", "Accept", { k: c.k, yes: 1 }, "primary")} ${foBtn("call", "Turn down", { k: c.k, yes: 0 })}</div></div>`).join("");
  const ans = tr.answer;
  const condOpts = tr.givePicks.length && tr.get.length ? `<div class="fo-cond"><label class="fo-check"><input type="checkbox" data-fo-cond="on" ${tr.cond ? "checked" : ""}> Make one of your picks conditional</label>${tr.cond ? `<select data-fo-cond="pick">${x.picks.filter((p) => tr.givePicks.includes(foPickKey(p)) && p.round > 1).map((p) => `<option value="${foPickKey(p)}" ${tr.cond.pick === foPickKey(p) ? "selected" : ""}>${esc(p.text)}</option>`).join("")}</select> moves up a round <select data-fo-cond="kind"><option value="games" ${tr.cond.kind === "games" ? "selected" : ""}>if he plays in</option><option value="starts" ${tr.cond.kind === "starts" ? "selected" : ""}>if he starts</option><option value="pro_bowl" ${tr.cond.kind === "pro_bowl" ? "selected" : ""}>if he makes the Pro Bowl</option></select>${tr.cond.kind !== "pro_bowl" ? `<input type="number" min="1" max="17" data-fo-cond="n" value="${tr.cond.n || 8}"> games` : ""} · <select data-fo-cond="player">${getP.map((r) => `<option value="${r.id}" ${tr.cond.player === r.id ? "selected" : ""}>${esc(r.name)}</option>`).join("")}</select>` : ""}</div>` : "";
  const shop = tr.shop ? `<div class="panel"><h2>Shopping ${esc(tr.shop.name)} <small>the best package of picks each club would give</small></h2>${tr.shop.offers.length ? tr.shop.offers.map((o) => `<div class="fo-call">${mark(o.team, 32)}<div><b>${teamLink(o.team)}</b><small>${o.picks.map(foPick).join(" ")} · ${o.points} point${o.points === 1 ? "" : "s"} on your chart</small></div>${foBtn("shoptake", "Build this trade", { team: o.team.ref.id, id: tr.shop.id, picks: JSON.stringify(o.picks.map((p) => foPickKey(p))) })}</div>`).join("") : '<div class="empty">No club would give picks for him today.</div>'}</div>` : "";
  return `<div class="film"><div class="pad fo">
    ${foHead(x.team, "Trades", `${x.open ? `Trades are open until the deadline, ${esc(x.deadline)}.` : `Trades open with the league year (${esc(x.league_year)}) and close at the deadline (${esc(x.deadline)}).`} The other GM answers by his own read of the players, his chart and whether his club is chasing a title. Your chart: ${x.classic ? "the classic one" : "a surplus-value chart"}.`, foOwner("trades", x.owner))}
    ${calls ? `<div class="panel"><h2>Calls waiting</h2>${calls}</div>` : ""}
    <div class="panel fo-builder"><h2>Build a trade <small>with <select data-fo-team>${x.teams.map((t) => `<option value="${t.ref.id}" ${t.ref.id === tr.team ? "selected" : ""}>${esc(t.name)} · ${esc(t.mode)}</option>`).join("")}</select></small></h2>
      ${other ? `<div class="sub">${mark(other.team, 20)} ${teamLink(other.team)} · GM ${esc(other.team.gm)} · ${esc(other.team.mode)} · cap room ${foM(other.room)}</div>` : ""}
      <div class="fo-sides">
        <div><div class="st-label">You give · ${Math.round(givePts)} point${Math.round(givePts) === 1 ? "" : "s"}</div>${plist(x.roster, tr.give, "give")}<div class="fo-picks">${pickBox(x.picks, tr.givePicks, "give")}</div></div>
        <div><div class="st-label">You get · ${Math.round(getPts)} point${Math.round(getPts) === 1 ? "" : "s"}</div>${other ? plist(other.rows, tr.get, "get") + `<div class="fo-picks">${pickBox(other.picks, tr.getPicks, "get")}</div>` : ""}</div>
      </div>
      ${condOpts}
      <div class="st-actions">${foBtn("teval", "Ask their GM", {}, "glass")} ${foBtn("tpropose", "Make the trade", {}, "primary")} ${foBtn("tclear", "Clear", {})}</div>
      ${ans ? `<div class="fo-answer ${ans.accept ? "Signed" : "Counter"}"><b>${esc(ans.text)}</b>${ans.counter ? `<div>${ans.counter.map(foPick).join(" ")} ${foBtn("taddcounter", "Add it", {})}</div>` : ""}<small>Your chart: you give ${ans.points[0]}, you get ${ans.points[1]}.</small></div>` : ""}
    </div>
    ${shop}
    <div class="grid2">
      <div class="panel"><h2>Your trading block <small>buyers in each window treat them as available</small></h2>${x.block.length ? x.block.map((r) => `<div class="item"><span class="when">${esc(r.pos)}</span><span>${playerLink(r.id, r.name)} ${foBtn("block", "Take off", { id: r.id, on: 0 })} ${foBtn("shop", "Shop him", { id: r.id, name: r.name })}</span></div>`).join("") : '<div class="empty">Nobody on the block. Pick a player below.</div>'}
        <select data-fo-blockadd><option value="">Put a player on the block…</option>${x.roster.filter((r) => !r.block).map((r) => `<option value="${r.id}">${esc(r.pos)} ${esc(r.name)}</option>`).join("")}</select></div>
      <div class="panel"><h2>On the books</h2>${x.conditionals.map((c) => `<div class="item"><span class="when">${c.from_me ? "Yours" : "Theirs"}</span><span>${foPick(c.pick)} ${esc(c.cond)} (${esc(c.player || "")})</span></div>`).join("")}${x.made.map((m) => `<div class="item"><span class="when">${esc(m.date)}</span><span>${esc(m.text)}</span></div>`).join("") || (x.conditionals.length ? "" : '<div class="empty">No trades yet.</div>')}</div>
    </div>
  </div></div>`;
};

// ---------- Roster moves: the cut, the squad, the wire ----------
screens.fo_moves = async () => {
  const tab = keep("fomoves", { t: "Cut to 53" });
  const tabs = ["Cut to 53", "Practice squad", "Waiver wire"];
  let body = "";
  let team = null;
  if (tab.t === "Cut to 53") {
    const x = await api("fo_cutdown");
    team = x.team;
    const final = (r) => r.plan || r.staff;
    const n = (k) => x.rows.filter((r) => final(r) === k).length;
    const seg = (r) => `<div class="seg fo-mini">${["keep", "squad", "release"].map((k) => `<button data-act="fo" data-fo="plan" data-id="${r.id}" data-cut="${k}" aria-pressed="${r.plan === k}" ${k === "squad" && !r.squad_ok ? "disabled title=\"Too many seasons for the squad\"" : ""}>${{ keep: "53", squad: "Squad", release: "Cut" }[k]}</button>`).join("")}${r.plan ? `<button data-act="fo" data-fo="plan" data-id="${r.id}" data-cut="" title="Back to the staff's call">Staff</button>` : ""}</div>`;
    const cols = [
      { k: "rank", t: "#", tip: "Your staff's ranking from camp and the preseason tape", v: (r) => -r.rank, f: (r) => r.rank },
      { k: "name", t: "Player", l: true, f: (r) => playerLink(r.id, r.name) },
      { k: "pos", t: "Pos", l: true },
      { k: "age", t: "Age" },
      { k: "class", t: "Exp", l: true, v: (r) => classRank(r.class) },
      { k: "round", t: "Drafted", v: (r) => r.round || 99, f: (r) => (r.round ? esc(r.draft || "R" + r.round) : '<span class="muted">Undrafted</span>') },
      { k: "ovr", t: "Rating", v: (r) => mid(r.ovr), f: (r) => range(r.ovr, null, { p: r.id, k: "ovr" }) },
      { k: "hit", t: "Cap hit", div: true, f: (r) => foM(r.hit) },
      { k: "dead", t: "Dead if cut", f: (r) => foM(r.dead) },
      { k: "staff", t: "Staff says", l: true, div: true, f: (r) => `<span class="st-pill ${r.staff === "keep" ? "good" : r.staff === "squad" ? "gold" : ""}">${{ keep: "53", squad: "Squad", release: "Cut" }[r.staff]}</span>` },
      { k: "plan", t: "Your call", l: true, v: (r) => r.plan || "", f: seg },
    ];
    body = `<p class="sub">Cutdown day is ${esc(x.date)}${x.in >= 0 ? ` (in ${x.in} day${x.in === 1 ? "" : "s"})` : ""}. Your call stands; every player you leave to the staff goes where their ranking puts him. <b>${n("keep")} of ${x.limit}</b> on the 53 · <b>${n("squad")} of ${x.squad_limit}</b> on the squad · ${n("release")} cut.</p>
      <div class="filters">${foOwner("cutdown", x.owner)}</div>
      ${table("fo_cut", cols, x.rows, { groupBy: (r) => r.pos, empty: "Nobody under contract for the season yet." })}`;
  } else if (tab.t === "Practice squad") {
    const x = await api("fo_squad");
    team = x.team;
    const sq = x.squad.map((r) => `<tr><td class="l">${playerLink(r.id, r.name)}</td><td class="l">${esc(r.pos)}</td><td>${r.age}</td><td>${range(r.ovr, null, { p: r.id, k: "ovr" })}</td><td>${r.elevations}</td><td>${x.protect ? `<label class="fo-check"><input type="checkbox" data-fo-protect="${r.id}" ${r.protected ? "checked" : ""}> Protect</label>` : ""}</td><td>${foBtn("signup", "Sign to the 53", { id: r.id })}</td></tr>`).join("") || `<tr><td class="l" colspan="7"><div class="empty">Nobody on the squad.</div></td></tr>`;
    const st = x.street.map((r) => `<tr><td class="l">${playerLink(r.id, r.name)}</td><td class="l">${esc(r.pos)}</td><td>${r.age}</td><td>${range(r.ovr, null, { p: r.id, k: "ovr" })}</td><td class="l">${r.team ? teamLink(r.team) : '<span class="muted">Unsigned</span>'}</td><td>${x.open ? foBtn("signsquad", "Sign to the squad", { id: r.id }) : ""}</td></tr>`).join("");
    body = `<p class="sub">${x.squad.length} of ${x.limit} on the squad · roster ${x.roster} of ${x.roster_limit}. ${x.protect ? `Protect up to ${x.protect} a week: other clubs can sign the rest to their 53.` : ""} ${x.open ? "" : "Squads are formed at cutdown."}</p>
      <div class="filters">${foOwner("practice_squad", x.owner)}</div>
      <div class="grid2"><div class="panel"><h2>Your squad</h2><div class="tbl"><table><thead><tr><th class="l">Player</th><th class="l">Pos</th><th>Age</th><th>Rating</th><th title="Game-day elevations this season">Elev.</th><th></th><th></th></tr></thead><tbody>${sq}</tbody></table></div></div>
      <div class="panel"><h2>Eligible on the street <small>the league's read</small></h2><div class="tbl"><table><thead><tr><th class="l">Player</th><th class="l">Pos</th><th>Age</th><th>Rating</th><th class="l"></th><th></th></tr></thead><tbody>${st}</tbody></table></div></div></div>`;
  } else {
    const x = await api("fo_wire");
    team = x.team;
    const cols = [
      { k: "name", t: "Player", l: true, f: (r) => playerLink(r.id, r.name) },
      { k: "pos", t: "Pos", l: true },
      { k: "age", t: "Age" },
      { k: "ovr", t: "Rating", v: (r) => mid(r.ovr), f: (r) => range(r.ovr, null, { p: r.id, k: "ovr" }) },
      { k: "from", t: "Waived by", l: true, div: true, v: (r) => (r.from ? r.from.name : ""), f: (r) => (r.from ? teamLink(r.from) : "") },
      { k: "hit", t: "Cap hit", f: (r) => foM(r.hit) },
      { k: "through", t: "Through" },
      { k: "act", t: "", f: (r) => foBtn("claim", r.claimed ? "Withdraw claim" : "Claim him", { id: r.id, on: r.claimed ? 0 : 1 }, r.claimed ? "glass" : "primary") },
    ];
    body = `<p class="sub">Players stay on the wire until 4 p.m. the next day. Claims go in the league's order (worst record first): you are <b>${x.place ? "#" + x.place : "-"}</b>. A claim takes over his contract. Roster ${x.roster} of ${x.limit} · room ${foM(x.room)}.</p>
      <div class="filters">${foOwner("waivers", x.owner)}</div>${table("fo_wire", cols, x.rows, { sort: "ovr", empty: "Nobody is on waivers today." })}`;
  }
  return `<div class="film"><div class="pad fo">${foHead(team, "Roster moves", "")}
    <div class="tabsline">${tabs.map((t) => `<button data-act="fo" data-fo="mtab" data-t="${t}" aria-selected="${t === tab.t}">${t}</button>`).join("")}</div>
    ${body}</div></div>`;
};

// ---------- actions ----------
// Restructure or release one of your players, from the cap sheet, his page
// or a menu (minor bug 62): `cap` is the game's numbers for him today.
async function foCapMove(kind, id, name, cap) {
  const c = cap || {};
  if (kind === "restructure") {
    const v = await foChoose(`Restructure ${name}`, "His base salary above the minimum becomes a signing bonus spread over the years left: room now, more on later caps.", [
      { v: "0", t: "Over the years left", s: "No void years." },
      ...(c.voids ? [{ v: "2", t: "Add two void years", s: "Spreads it further; the leftover lands on the cap when the deal voids." }] : []),
    ]);
    if (v != null) await foDo("fo_restructure", { id, voids: Number(v) });
    return;
  }
  const split = c.june1 > 0 && (c.dj2 > 0 || c.dj !== c.dead);
  const v = await foChoose(`Release ${name}`, "Every later year's bonus spread lands on the cap, with any guaranteed pay he's owed.", [
    { v: "now", t: `Release now · ${foM(c.dead)} dead, ${c.saves >= 0 ? "saves" : "costs"} ${foM(Math.abs(c.saves || 0))}` },
    ...(split ? [{ v: "june1", t: `Post-June 1 designation · ${foM(c.dj)} dead now, ${foM(c.dj2)} next year`, s: `Saves ${foM(c.sj)} this year once June 1 passes. ${c.june1} of 2 left this year.` }] : []),
  ]);
  if (v) await foDo("fo_release", { id, june1: v === "june1" });
}
function foRedraw() { render($(".view") && $(".view").scrollTop); }
async function foDo(cmd, args, ok) {
  try {
    const r = await api(cmd, args);
    if (r && r.text) toast(r.text);
    else if (ok) toast(ok);
    S.status = null;
    foRedraw();
    return r;
  } catch (e) {
    toast(e.message || String(e));
    return null;
  }
}
function foChoose(title, note, options) {
  const { f, answer } = dialog(`<b>${esc(title)}</b>${note ? `<small class="sub">${esc(note)}</small>` : ""}<div class="fo-choices">${options.map((o, k) => `<label class="fo-choice"><input type="radio" name="c" value="${esc(o.v)}" ${k === 0 ? "checked" : ""}><span><b>${esc(o.t)}</b>${o.s ? `<small>${esc(o.s)}</small>` : ""}</span></label>`).join("")}</div>`, "Confirm", (f) => f.elements.c.value);
  f.querySelector("[type=submit]").focus();
  return answer;
}
document.addEventListener("click", async (ev) => {
  const el = ev.target.closest("[data-act=fo]");
  if (!el) return;
  const d = el.dataset;
  const id = d.id != null ? Number(d.id) : undefined;
  switch (d.fo) {
    case "owner": await foDo("fo_set_owner", { duty: d.duty, owner: d.owner }, d.owner === "you" ? "You decide this now." : "Your staff has it now."); break;
    case "talk": go("fo_talk", { id }); break;
    case "tag": {
      const v = await foChoose(`Tag ${d.name}`, "The tag is a one-year deal, fully guaranteed, and counts on your cap the day it's placed.", [
        { v: "franchise", t: `Franchise tag · ${foM(Number(d.f))}`, s: "Other clubs may sign him to an offer sheet; you can match or take two first-round picks." },
        { v: "exclusive", t: `Exclusive franchise tag · ${foM(Number(d.x))}`, s: "Nobody else may talk to him. Priced from this year's top five at his position." },
        { v: "transition", t: `Transition tag · ${foM(Number(d.t))}`, s: "You keep the right to match any offer, with no picks if you don't." },
      ]);
      if (v) await foDo("fo_tag", { id, kind: v });
      break;
    }
    case "tender": {
      const v = await foChoose(`Tender ${d.name}`, "A restricted free agent's tender: a one-year offer. Another club may sign him to an offer sheet; you match or take the pick it names.", [
        { v: "first", t: `First-round tender · ${foM(Number(d.a))}` },
        { v: "second", t: `Second-round tender · ${foM(Number(d.b))}` },
        { v: Number(d.drafted) ? "original" : "rofr", t: `${Number(d.drafted) ? `Original round (round ${d.drafted})` : "Right of first refusal"} · ${foM(Number(d.c))}` },
      ]);
      if (v) await foDo("fo_tender", { id, level: v });
      break;
    }
    case "erfa": if (await sure(`Tender ${d.name}?`, `An exclusive-rights tender at ${foM(Number(d.p))}: he can't talk to other clubs.`, "Tender him")) await foDo("fo_tender", { id, level: "erfa" }); break;
    case "rescind":
      if (d.before === "1") await foDo("fo_rescind", { id });
      else if (await sure(`Take back ${d.name}'s ${d.what}?`, "He becomes a free agent at once, and you owe nothing.", "Take it back")) await foDo("fo_rescind", { id });
      break;
    case "option": await foDo("fo_option", { id, take: d.take === "1" }, d.take === "1" ? "You'll pick up his option." : "You'll decline his option."); break;
    case "sheet": await foDo("fo_sheet", { id, match: d.m === "1" }, d.m === "1" ? "You'll match." : "You'll let him go."); break;
    case "capmove": {
      const opts = [];
      if (Number(d.re)) opts.push({ v: "re0", t: "Restructure over the years left", s: "His base above the minimum becomes a signing bonus spread over his deal: room now, more on later caps." });
      if (Number(d.re) && Number(d.voids)) opts.push({ v: "re2", t: "Restructure with two void years", s: "Spreads it further; what's left lands on the cap the year the deal voids." });
      opts.push({ v: "now", t: `Release him now · ${foM(Number(d.dead))} dead, ${Number(d.saves) >= 0 ? "saves" : "costs"} ${foM(Math.abs(Number(d.saves)))}`, s: "Every later year's bonus spread lands on this year's cap, with any guaranteed pay he's owed." });
      if (Number(d.june1) > 0 && (Number(d.dj2) > 0 || Number(d.dj) !== Number(d.dead))) opts.push({ v: "june1", t: `Release with a post-June 1 designation · ${foM(Number(d.dj))} dead now, ${foM(Number(d.dj2))} next year`, s: `Saves ${foM(Number(d.sj))} once June 1 passes. ${d.june1} of 2 left this year.` });
      const v = await foChoose(d.name, "", opts);
      if (v === "re0" || v === "re2") await foDo("fo_restructure", { id, voids: v === "re2" ? 2 : 0 });
      else if (v === "now" || v === "june1") await foDo("fo_release", { id, june1: v === "june1" });
      break;
    }
    case "restructure": {
      const v = await foChoose(`Restructure ${d.name}`, "His base salary above the minimum becomes a signing bonus spread over the years left: room now, more on later caps.", [
        { v: "0", t: "Over the years left", s: "No void years." },
        ...(Number(d.voids) ? [{ v: "2", t: "Add two void years", s: "Spreads it further; the leftover lands on the cap when the deal voids." }] : []),
      ]);
      if (v != null) await foDo("fo_restructure", { id, voids: Number(v) });
      break;
    }
    case "release": {
      const after = Number(d.dj2) > 0 || Number(d.dj) !== Number(d.dead);
      const v = await foChoose(`Release ${d.name}`, "Every later year's bonus spread lands on the cap, with any guaranteed pay he's owed.", [
        { v: "now", t: `Release now · ${foM(Number(d.dead))} dead, saves ${foM(Number(d.saves))}` },
        ...(after ? [{ v: "june1", t: `Post-June 1 designation · ${foM(Number(d.dj))} dead now, ${foM(Number(d.dj2))} next year`, s: `Saves ${foM(Number(d.sj))} this year once June 1 passes. Two a year.` }] : []),
      ]);
      if (v) await foDo("fo_release", { id, june1: v === "june1" });
      break;
    }
    case "offer": {
      const dr = keep("fodraft:" + id, {});
      try {
        const t = await api("fo_offer", { id, ...dr });
        S.keep.foanswer = { id, a: t.answer };
        S.status = null;
        foRedraw();
      } catch (e) { toast(e.message || String(e)); }
      break;
    }
    case "meet": {
      const t = await api("fo_talk", { id });
      if (t.talk && t.talk.ask) Object.assign(keep("fodraft:" + id, {}), { ...t.talk.ask });
      foRedraw();
      break;
    }
    case "visit": await foDo("fo_visit", { id }); break;
    case "mpos": keep("fomarket", {}).pos = d.v; foRedraw(); break;
    case "mstreet": { const f = keep("fomarket", {}); f.street = !f.street; foRedraw(); break; }
    case "mmine": { const f = keep("fomarket", {}); f.mine = !f.mine; foRedraw(); break; }
    case "mtab": keep("fomoves", {}).t = d.t; foRedraw(); break;
    case "plan": await foDo("fo_plan", { id, cut: d.cut || null }); break;
    case "signup": await foDo("fo_sign_up", { id }); break;
    case "signsquad": await foDo("fo_sign_squad", { id }); break;
    case "claim": await foDo("fo_claim", { id, on: d.on === "1" }, d.on === "1" ? "Claim in." : "Claim withdrawn."); break;
    case "call": await foDo("fo_call", { k: Number(d.k), yes: d.yes === "1" }); break;
    case "block": await foDo("fo_block", { id, on: d.on === "1" }, "Off the block."); break;
    case "shop": {
      try {
        const r = await api("fo_shop", { id });
        foTrade().shop = { id, name: d.name, offers: r.offers };
        foRedraw();
      } catch (e) { toast(e.message || String(e)); }
      break;
    }
    case "shoptake": {
      const tr = foTrade();
      Object.assign(tr, { team: Number(d.team), give: [id], givePicks: [], get: [], getPicks: JSON.parse(d.picks), cond: null, answer: null, shop: null });
      foRedraw();
      break;
    }
    case "tclear": Object.assign(foTrade(), { give: [], givePicks: [], get: [], getPicks: [], cond: null, answer: null }); foRedraw(); break;
    case "teval":
    case "tpropose": {
      const tr = foTrade();
      const pk = (k) => { const [year, round, orig, comp] = k.split(":").map(Number); return { year, round, orig, comp: comp === 1 }; };
      const args = {
        team: tr.team, give_players: tr.give, get_players: tr.get,
        give_picks: tr.givePicks.map(pk), get_picks: tr.getPicks.map(pk),
        conditional: tr.cond && tr.cond.pick && tr.cond.player ? { ...pk(tr.cond.pick), kind: tr.cond.kind, n: tr.cond.n || 8, player: tr.cond.player } : null,
      };
      try {
        const a = await api(d.fo === "teval" ? "fo_trade_eval" : "fo_trade_propose", args);
        tr.answer = a;
        if (d.fo === "tpropose" && a.accept) {
          toast("Trade made.");
          Object.assign(tr, { give: [], givePicks: [], get: [], getPicks: [], cond: null });
          S.status = null;
        }
        foRedraw();
      } catch (e) { toast(e.message || String(e)); }
      break;
    }
    case "taddcounter": {
      const tr = foTrade();
      for (const p of (tr.answer && tr.answer.counter) || []) if (!tr.givePicks.includes(foPickKey(p))) tr.givePicks.push(foPickKey(p));
      tr.answer = null;
      foRedraw();
      break;
    }
  }
});
document.addEventListener("change", async (ev) => {
  const t = ev.target;
  const dd = t.dataset;
  if (dd.foD) {
    const id = S.route.args.id;
    const dr = keep("fodraft:" + id, {});
    const v = t.type === "checkbox" ? t.checked : Number(t.value);
    dr[dd.foD] = dd.foD === "apy" || dd.foD === "incentives" ? v * 1e6 : dd.foD === "gtd" || dd.foD === "bonus" ? v / 100 : v;
    if (dr.bonus > dr.gtd) dr.bonus = dr.gtd;
    foRedraw();
  } else if (dd.foPl) {
    const tr = foTrade();
    const list = dd.foPl === "give" ? tr.give : tr.get;
    const id = Number(dd.id);
    const k = list.indexOf(id);
    if (t.checked && k < 0) list.push(id);
    if (!t.checked && k >= 0) list.splice(k, 1);
    tr.answer = null;
    foRedraw();
  } else if (dd.foPick) {
    const tr = foTrade();
    const list = dd.foPick === "give" ? tr.givePicks : tr.getPicks;
    const k = list.indexOf(dd.k);
    if (t.checked && k < 0) list.push(dd.k);
    if (!t.checked && k >= 0) list.splice(k, 1);
    tr.answer = null;
    foRedraw();
  } else if (dd.foTeam !== undefined) {
    Object.assign(foTrade(), { team: Number(t.value), get: [], getPicks: [], cond: null, answer: null });
    foRedraw();
  } else if (dd.foCond) {
    const tr = foTrade();
    if (dd.foCond === "on") tr.cond = t.checked ? { kind: "games", n: 8, pick: null, player: tr.get[0] || null } : null;
    else if (tr.cond) tr.cond[dd.foCond] = dd.foCond === "n" || dd.foCond === "player" ? Number(t.value) : t.value;
    if (tr.cond && !tr.cond.pick) {
      const s = document.querySelector("[data-fo-cond=pick]");
      tr.cond.pick = s ? s.value : tr.givePicks[0];
    }
    tr.answer = null;
    foRedraw();
  } else if (dd.foBlockadd !== undefined && t.value) {
    await foDo("fo_block", { id: Number(t.value), on: true }, "On the block: buyers will call.");
  } else if (dd.foProtect) {
    await foDo("fo_protect", { id: Number(dd.foProtect), on: t.checked }, t.checked ? "Protected this week." : "No longer protected.");
  }
});
