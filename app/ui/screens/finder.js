// The trade finder (TF1-TF19): shop one of your players over days. Six
// tabs on one line: Shop him (set up the shop and read the market), Offers
// (what came in, each with its expiry), Talks (counter with their picks,
// name a rival), Closing (owners, his new deal, the physical), Draft night
// (the war room's calls) and Move a contract (what it costs to move a
// deal nobody wants). The look is the approved v3 boards: monochrome, club
// colours only on club marks, gold the one accent.
"use strict";

CRUMBS.fo_finder = "Trade finder";
GOTO.fo_finder = () => [["Trades", "fo_trades"], ["Cap sheet", "fo_cap"], ["Front office", "fo_office"], ["Squad", "roster"]];

const tfState = () => keep("tf", { tab: "shop", open: false, ask: "best", paydown: 0, agent: true, clubs: null, club: null, picks: [], rival: null, bluff: false, dump: null });
const tfBtn = (k, text, data = {}, cls = "glass", dis = false) =>
  `<button class="st-btn small ${cls}" data-act="tf" data-tf="${k}"${Object.entries(data).map(([a, v]) => ` data-${a}="${esc(v)}"`).join("")}${dis ? " disabled" : ""}>${esc(text)}</button>`;
const tfBar = (v, max, o = {}) => `<div class="p3-bar ${o.cls || ""}"><i class="f ${o.f || ""}" style="width:${Math.max(0, Math.min(100, (v / Math.max(1, max)) * 100))}%"></i>${o.tick != null ? `<i class="${o.tk || "t"}" style="left:${Math.max(0, Math.min(100, (o.tick / Math.max(1, max)) * 100))}%"></i>` : ""}</div>`;
const tfSeg = (n, of) => `<span class="p3-seg">${Array.from({ length: of }, (_, i) => `<i class="${i < n ? "on" : ""}"></i>`).join("")}</span>`;
const tfPick = (p) => `<span class="p3-pk" title="${esc(p.text)} · ${p.points} on your chart">${mark(p.from, 18)}<b>${p.year} R${p.round}</b><span>${p.own === false || !p.own ? esc(p.from.name.split(" ").pop()) : "yours"}${p.comp ? " · comp" : ""}</span></span>`;
const tfClub = (t, sub) => `<div class="p3-club">${mark(t, 28)}<div><b>${teamLink(t)}</b>${sub ? `<small>${esc(sub)}</small>` : ""}</div></div>`;
const tfPkg = (pk) => {
  if (!pk) return "";
  const chips = pk.picks.map((p) => tfPick(p) + (p.cond ? `<span class="p3-tag gold" title="${esc(p.cond)}">Conditional</span>` : "")).join("");
  const swap = pk.swap ? `<span class="p3-tag">Swap of ${pk.swap.get.year} R${pk.swap.get.round}s</span>` : "";
  const back = pk.back.map((r) => `<div class="t2" style="margin-top:6px">With ${playerLink(r.id, r.name)} · ${esc(r.pos)} · ${r.age} · ${range(r.ovr, null, { p: r.id, k: "ovr" })}</div>`).join("");
  const conds = pk.picks.filter((p) => p.cond).map((p) => `<div class="t3" style="margin-top:6px">${p.year} R${p.round} moves up a round ${esc(p.cond)}.</div>`).join("");
  return `<div class="p3-pks">${chips}${swap}</div>${conds}${back}`;
};

screens.fo_finder = async (a = {}) => {
  const st = tfState();
  if (a.id != null) { st.id = Number(a.id); delete a.id; }
  if (a.tab) { st.tab = a.tab; delete a.tab; }
  let x;
  try {
    x = await api("tf_view", { id: st.id, open: st.open, ask: st.ask, paydown: st.paydown, agent: st.agent, club: st.club, move: st.tab === "move", night: st.tab === "draft" });
  } catch (e) {
    return tfPicker(String(e.message || e));
  }
  // Opened with no player: the shop you have open, else pick one.
  if (st.id == null && (!x.shop || x.shop.closed || a.pick)) { delete a.pick; return tfPicker("Pick the player you want to shop."); }
  st.id = x.player.id;
  const shop = x.shop;
  // He's gone: the shop's record and draft night stay, the rest needs a player of yours.
  if (x.player.mine === false && !["offers", "closing", "talks", "draft"].includes(st.tab)) st.tab = "closing";
  const tabs = [["shop", "Shop him"], ["offers", "Offers", shop ? shop.count.offers : 0], ["talks", "Talks"], ["closing", "Closing"], ["draft", "Draft night"], ["move", "Move a contract"]];
  const tabline = `<div class="tabsline" role="tablist">${tabs.map(([k, t, n]) => `<button data-act="tf" data-tf="tab" data-t="${k}" aria-selected="${k === st.tab}">${t}${n ? `<small>${n}</small>` : ""}</button>`).join("")}</div>`;
  const p = x.player;
  const status = p.mine === false ? `<span class="gold">Traded to the ${esc(p.gone_to || "")}</span>` : p.wants_out ? '<span class="gold">Trade request</span>' : p.leverage.length ? `<span class="gold">${esc(p.leverage[0])}</span>` : "Under contract";
  const strip = `<div class="p3-strip"><div>${mark(p.team || x.team, 40)}<b class="nm">${playerLink(p.id, p.name)}</b><span class="sub"><span>${esc(p.pos)} · ${p.age}</span>${range(p.ovr, null, { p: p.id, k: "ovr" })}</span></div>
    <div><span class="lb">Cap hit this year</span><b>${foM(p.hit)}</b></div>
    <div><span class="lb">Signed through</span><b>${p.through ?? "-"}</b></div>
    <div><span class="lb">Worth to you</span><b class="${p.points < 0 ? "gold" : ""}">${p.points == null ? "-" : p.points + " pts"}</b></div>
    <div><span class="lb">Status</span><b>${status}</b></div></div>`;
  const body = { shop: tfShop, offers: tfOffers, talks: tfTalks, closing: tfClosing, draft: tfDraft, move: tfMove }[st.tab](x, st);
  const team = x.team;
  const sub = x.open
    ? `Shop one of your players and gather offers over the coming days. Each club answers by its own read of him, its cap and its season. Trades close at the deadline, ${esc(x.deadline)}. Your chart: ${x.classic ? "the classic one" : "a surplus-value chart"}.`
    : `Trades are closed until the new league year. Your chart: ${x.classic ? "the classic one" : "a surplus-value chart"}.`;
  return `<div class="film"><div class="pad fo p3">${foHead(team, "Trade finder", sub)}${tabline}${st.tab === "draft" ? "" : strip}${x.shopping_other ? `<div class="p3-note">You're shopping <b>${esc(x.shopping_other)}</b> now. Close that shop before you open another.</div>` : ""}${body}</div></div>`;
};

async function tfPicker(msg) {
  const x = await api("fo_trades");
  const rows = x.roster.slice().sort((a, b) => b.points - a.points).map((r) => `<tr><td>${playerLink(r.id, r.name)}</td><td>${esc(r.pos)}</td><td>${r.age}</td><td>${range(r.ovr, null, { p: r.id, k: "ovr" })}</td><td class="r">${foM(r.hit)}</td><td class="r">${r.points}</td><td>${tfBtn("pick", "Shop him", { id: r.id }, "primary")}</td></tr>`).join("");
  return `<div class="film"><div class="pad fo p3">${foHead(x.team, "Trade finder", "Pick the player you want to shop.")}
    <div class="panel"><h2>Your players <small>${esc(msg)}</small></h2><table class="p3-t"><thead><tr><th>Player</th><th>Pos</th><th>Age</th><th>Rating</th><th class="r">Cap hit</th><th class="r">Worth to you</th><th></th></tr></thead><tbody>${rows}</tbody></table></div></div></div>`;
}

const tfAct = (left, note, right) => `<div class="p3-act">${left}<span class="grow">${note || ""}</span>${right}</div>`;

function tfShop(x, st) {
  const live = x.shop && !x.shop.closed;
  const reads = x.reads;
  if (st.clubs == null) st.clubs = reads.filter((r) => !r.pass).slice(0, 5).map((r) => r.club);
  const max = Math.max(x.ask_points * 1.4, ...reads.map((r) => r.high), 1);
  const rows = reads.map((r) => {
    const on = st.open ? !r.pass : st.clubs.includes(r.club);
    const pass = r.pass ? `<small>${esc(r.pass)}</small>` : "";
    const band = r.pass === "Won't buy" ? "" : `<div class="p3-bar band" style="margin:2px 0 8px"><i class="f ${r.pass ? "dim" : ""}" style="left:${(100 * r.low) / max}%;width:${Math.max(2, (100 * (r.high - r.low)) / max)}%"></i>${x.ask_points ? `<i class="t" style="left:${(100 * x.ask_points) / max}%"></i>` : ""}</div>`;
    return `<tr class="${r.pass ? "dim" : ""}"><td>${st.open ? `<span class="p3-cb ${on ? "on" : ""} dis"></span>` : `<span class="p3-cb ${on ? "on" : ""} ${r.pass ? "dis" : ""}" data-act="tf" data-tf="club" data-club="${r.club}" style="cursor:pointer"></span>`}</td>
      <td>${tfClub(r.team, "GM " + r.gm)}</td><td>${esc(r.mode)}${r.record ? `<small>${esc(r.record)} · playoff odds ${r.race}%</small>` : `<small>playoff odds ${r.race}%</small>`}</td><td>${esc(r.need)}</td><td class="r">${foM(r.room)}</td>
      <td>${r.fits ? "Yes" : '<span class="p3-tag gold">No room</span>'}</td>
      <td style="min-width:240px">${band}<b>${r.pass === "Won't buy" ? "Won't buy" : `${r.low} to ${r.high} pts`}</b>${pass}</td></tr>`;
  }).join("");
  const askSeg = [["best", "Best offers"], ["second", "A second"], ["first", "A first"], ["starter", "A starter back and a pick"]].map(([k, t]) => `<button data-act="tf" data-tf="ask" data-v="${k}" aria-pressed="${st.ask === k}">${t}</button>`).join("");
  const pdSeg = x.paydowns.map((d) => `<button data-act="tf" data-tf="paydown" data-v="${d.share}" aria-pressed="${Math.abs(st.paydown - d.share) < 0.01}">${d.share === 0 ? "No" : d.share === 0.25 ? "A quarter" : "Half"}${d.share ? ` <small>${foM(d.amount)}</small>` : ""}</button>`).join("");
  const reach = st.open ? reads.length : st.clubs.length;
  const lev = x.player.leverage;
  const levNote = lev.length ? `<div class="p3-note"><b>Clubs know your hand.</b> ${esc(lev.join(". "))}. Every GM has read it: expect offers below his worth to you.</div>` : "";
  const setup = `<div class="p3-grid p3-g21">
      <div class="panel"><h2>Set up the shop <small>offers come in over the next days, each with its own expiry</small></h2>
        <div class="body">
          <div class="p3-field"><span class="lb">How you shop</span>
            <div class="p3-opt ${st.open ? "" : "on"}" data-act="tf" data-tf="mode" data-v="0" style="cursor:pointer"><span class="p3-rad"></span><div><b>Quietly</b><span class="t2">Only the clubs you tick hear from you. Each call can still leak to the press.</span></div></div>
            <div class="p3-opt ${st.open ? "on" : ""}" data-act="tf" data-tf="mode" data-v="1" style="cursor:pointer"><span class="p3-rad"></span><div><b>Openly</b><span class="t2">The whole league hears. More clubs bid and each keeps less for himself, the media runs it, and he hears it too: it hurts him more.</span></div></div>
          </div>
          <div class="p3-field"><span class="lb">Your asking price</span><div class="seg">${askSeg}</div>
            <span class="t3">A club whose own price is far below it won't call back. The gold tick in the market below is your ask${x.ask_points ? ` (${x.ask_points} pts)` : ""}.</span></div>
          <div class="p3-field"><span class="lb">Pay part of his salary down</span><div class="seg">${pdSeg}</div>
            <span class="t3">His salary becomes a signing bonus before the trade, so he fits a buyer's cap. The bonus stays on your cap as dead money.</span></div>
          <div class="p3-tog" data-act="tf" data-tf="agent" style="cursor:pointer"><span class="sw" style="${st.agent ? "" : "background:var(--fill-2)"}"></span><div><b>Buyers may talk to his agent about a new deal</b><span class="t3">An offer can then hang on an extension: no deal, no trade.</span></div></div>
        </div></div>
      <div style="display:grid;gap:18px">
        <div class="panel"><h2>What this does</h2><div class="p3-cells" style="grid-auto-flow:row">
          <div><span class="lb">Reach</span><b>${st.open ? `All ${reads.length} clubs` : `${reach} club${reach === 1 ? "" : "s"} you ticked`}</b><span class="t3">${st.open ? "Every club that wants him calls." : `Open would reach all ${reads.length}.`}</span></div>
          <div><span class="lb">Leaks</span><b>${st.open ? "It's news the day you open" : "Each call can reach the press"}</b></div>
          <div><span class="lb">He hears it</span><b>${st.open ? "Yes, and it stings more" : "Not unless it leaks"}</b><span class="t3">${esc(x.mood.words)}</span></div></div></div>
        ${levNote}
      </div></div>`;
  const market = `<div class="panel"><h2>The market for him <small>your pro scouts' read of each club${st.open ? "" : "; tick the clubs you call"}</small></h2>
      <table class="p3-t"><thead><tr><th></th><th>Club</th><th>Their season</th><th>Need at ${esc(x.player.pos)}</th><th class="r">Cap room</th><th>Fits his cap</th><th>What he'd fetch <span style="text-transform:none;letter-spacing:0;font-weight:500">· <span style="color:var(--gold)">| your ask</span> · on your chart</span></th></tr></thead><tbody>${rows}</tbody></table>
      <div class="body t3" style="border-top:1px solid var(--hair)">What he'd fetch is your staff's read of where each club's offers land. Its top price stays hidden, as it does in real life. A club in the race near the deadline pays by its playoff odds.</div></div>`;
  const note = `${st.open ? "Open shop" : `Quiet shop · ${reach} club${reach === 1 ? "" : "s"}`} · ${{ best: "best offers", second: "asking a second", first: "asking a first", starter: "asking a starter back and a pick" }[st.ask]}${st.paydown ? ` · paying ${st.paydown === 0.5 ? "half" : "a quarter"} down` : ""}`;
  const act = live
    ? tfAct(tfBtn("go_offers", "See the offers"), "The shop is open: the market above is today's read.", tfBtn("close", "Close the shop", {}, "primary"))
    : tfAct(foBtn("block", "Put him on the block instead", { id: x.player.id, on: 1 }), note, tfBtn("start", "Start shopping", {}, "primary", !x.open));
  return setup + market + act;
}

function tfOffers(x, st) {
  const s = x.shop;
  if (!s) return `<div class="panel"><div class="body"><div class="empty">No shop open. Set one up on Shop him.</div></div></div>`;
  const offers = s.bids.filter((b) => b.state === "offer" || b.state === "won").sort((a, b) => (b.package?.worth || 0) - (a.package?.worth || 0));
  const worth = Math.max(1, x.player.points);
  const max = Math.max(worth * 1.3, ...offers.map((o) => o.package?.worth || 0));
  const rows = offers.map((o, k) => `<tr class="${k === 0 ? "on" : ""}"><td style="width:34px;color:var(--fg3)">${k + 1}</td>
      <td>${tfClub(o.team, "GM " + o.gm)}</td>
      <td>${tfPkg(o.package)}${o.ext ? `<div style="margin-top:6px"><span class="p3-tag gold">Hangs on a new deal</span> <span class="t3">${o.ext.years} years at ${foM(o.ext.apy)} a year: no deal, no trade.</span></div>` : ""}</td>
      <td style="min-width:190px">${tfBar(o.package?.worth || 0, max, { tick: worth, tk: "k", f: k === 0 ? "" : "dim" })}<div class="t2" style="margin-top:7px"><b style="color:var(--fg)">${o.package?.worth ?? 0}</b> of his ${x.player.points}</div></td>
      <td><b>${o.state === "won" ? "Accepted" : esc(o.until)}</b><small>${o.state === "won" ? "closing" : "expires"}</small></td>
      <td>${o.state === "offer" ? `<div class="p3-row-act">${tfBtn("talk", "Counter", { club: o.club })}${tfBtn("accept", "Accept", { club: o.club }, "primary")}</div><div class="p3-row-act" style="margin-top:6px"><a class="p3-link" data-act="tf" data-tf="lapse" data-club="${o.club}">Let it lapse</a></div>` : ""}</td></tr>`).join("");
  const waiting = s.bids.filter((b) => b.state === "waiting");
  const passed = s.bids.filter((b) => ["passed", "dropped", "ended", "lapsed"].includes(b.state));
  const others = passed.map((b) => `<tr class="dim"><td></td><td>${tfClub(b.team)}</td><td colspan="4">${esc(b.words)}</td></tr>`).join("");
  const log = s.log.slice().reverse().map((l) => `<div><span class="d">${esc(l.date)}</span><span style="${l.you ? "color:var(--fg)" : ""}">${esc(l.text)}</span></div>`).join("");
  const days = s.opened;
  const best = offers.find((o) => o.state === "offer");
  return `<div class="p3-sumbar"><div class="p3-cells">
      <div><span class="lb">The shop</span><b>${s.closed ? "Closed" : s.open ? "Open" : "Quiet"}</b><span class="t3">since ${esc(days)}</span></div>
      <div><span class="lb">Offers</span><b>${s.count.offers}</b><span class="t3">each with its expiry</span></div>
      <div><span class="lb">Still thinking</span><b>${waiting.length} club${waiting.length === 1 ? "" : "s"}</b><span class="t3">${waiting.length ? `next answer ${esc(waiting[0].due)}` : "all answered"}</span></div>
      <div><span class="lb">Passed</span><b>${passed.length} club${passed.length === 1 ? "" : "s"}</b><span class="t3">of ${s.count.called} called</span></div>
      <div><span class="lb">How he's taking it</span><b>${x.mood.knows ? (x.mood.trust >= 60 ? "Fine" : x.mood.trust >= 35 ? "Unsettled" : "Angry") : "Unaware"}</b><span class="t3">${esc(x.mood.words)}</span></div>
    </div></div>
    <div class="panel"><h2>Offers for ${esc(x.player.name)} <small>best first, on your chart · the tick is his worth to you</small></h2>
      <table class="p3-t"><thead><tr><th></th><th>Club</th><th>Offer</th><th>Against his worth</th><th>Expires</th><th></th></tr></thead><tbody>${rows || `<tr><td colspan="6"><div class="empty">No offers yet. Clubs answer in a day or two.</div></td></tr>`}${others}</tbody></table></div>
    <div class="panel"><h2>The talks <small>day by day</small></h2><div class="p3-log">${log}</div></div>
    ${tfAct(s.closed ? "" : tfBtn("close", "Close the shop"), "Accepting starts the closing: owners, his new deal, the physical", best ? tfBtn("talk", "Counter the best offer", { club: best.club }, "primary") : "")}`;
}

function tfTalks(x, st) {
  const s = x.shop, t = x.talk;
  if (!s || !t) return `<div class="panel"><div class="body"><div class="empty">No offer to talk over. Offers land on the Offers tab.</div></div></div>`;
  st.club = t.club;
  const bid = s.bids.find((b) => b.club === t.club);
  const give = x.player.points, get = bid && bid.package ? bid.package.worth : 0;
  const key = (p) => `${p.year}:${p.round}:${p.orig}:${p.comp ? 1 : 0}`;
  const picks = t.picks.map((p) => { const on = st.picks.includes(key(p)); return `<tr class="${on ? "on" : ""}" data-act="tf" data-tf="tpick" data-k="${key(p)}" style="cursor:pointer"><td style="width:34px"><span class="p3-cb ${on ? "on" : ""}"></span></td><td>${tfPick(p)}</td><td>${p.orig === t.club && !p.comp ? "their own" : p.comp ? "compensatory" : "from the " + esc(p.from.name)}</td><td class="r">${p.points}</td></tr>`; }).join("");
  const asked = t.picks.filter((p) => st.picks.includes(key(p))).reduce((a, p) => a + p.points, 0);
  const th = t.lines.map((l) => `<div class="${l.you ? "me" : ""}"><span class="who">${l.you ? "You" : esc(t.gm)}</span><span class="txt">${esc(l.text)}</span></div>`).join("") || `<div><span class="who">${esc(t.gm)}</span><span class="txt">${bid && bid.package ? esc("We'd give " + bid.package.text + ".") : ""}</span></div>`;
  const rivals = t.rivals.filter((r) => r.true || r.believe > 0).slice(0, 4).map((r) => {
    const on = st.rival === r.club;
    const lvl = r.believe;
    const words = r.true ? "he'll believe it" : lvl >= 50 ? "he might believe it" : lvl >= 25 ? "he probably won't believe it" : "he won't believe it";
    return `<div class="p3-opt ${on ? "on" : ""}" data-act="tf" data-tf="rival" data-club="${r.club}" data-bluff="${r.true ? 0 : 1}" style="cursor:pointer"><span class="p3-rad"></span><div><b>${r.true ? `${esc(r.team.name)} offer more` : `Say ${esc(r.team.name)} offer more`}</b><span class="t2">${r.true ? `True: ${esc(r.package ? r.package.text : "")}. He raises toward his top, or drops out if it's past it.` : "A bluff. He judges it on their need, their cap and their season. Caught, it costs his trust in you for a year."}</span><div style="display:grid;grid-template-columns:auto 120px;gap:12px;align-items:center;margin-top:8px"><span class="t3">Your staff: ${words}</span>${tfBar(lvl, 100)}</div></div><span class="p3-tag ${r.true ? "" : "gold"}">${r.true ? "True" : "Bluff"}</span></div>`;
  }).join("");
  const offerOn = bid && bid.state === "offer";
  const others = s.bids.filter((b) => b.state === "offer" && b.club !== t.club).map((b) => tfBtn("talk", b.team.name, { club: b.club })).join(" ");
  return `<div class="p3-grid p3-g11">
      <div style="display:grid;gap:18px">
        <div class="panel"><h2><span class="p3-club">${mark(t.team, 22)} Talks with ${teamLink(t.team)}</span> <small>GM ${esc(t.gm)}</small></h2>
          <div class="p3-cells" style="border-bottom:1px solid var(--hair)"><div><span class="lb">You give</span><b>${give} pts</b><span class="t3">${esc(x.player.name)}</span></div><div><span class="lb">You get</span><b>${get} pts</b><span class="t3">${bid && bid.package ? esc(bid.package.text) : "-"}</span></div><div><span class="lb">${get >= give ? "Over" : "Short"}</span><b style="color:var(--gold)">${Math.abs(give - get)} pts</b><span class="t3">on your chart</span></div></div>
          <div class="body" style="padding-bottom:6px"><span class="lb">Their picks · tick what you ask for</span></div>
          <table class="p3-t"><thead><tr><th></th><th>Pick</th><th>Slot</th><th class="r">Points</th></tr></thead><tbody>${picks}</tbody></table>
          <div class="body" style="border-top:1px solid var(--hair)"><div style="display:grid;grid-template-columns:auto 1fr;gap:16px;align-items:center"><span class="lb">Their patience</span><div style="display:flex;gap:14px;align-items:center">${tfSeg(Math.round((bid ? bid.patience : 0) / 10), 10)}<span class="t3">your staff's read; every counter can end the talks</span></div></div>${others ? `<div class="t3">Other offers: ${others}</div>` : ""}</div></div>
      </div>
      <div style="display:grid;gap:18px">
        <div class="panel"><h2>The talks <small>he opened below his top price</small></h2><div class="p3-th">${th}</div></div>
        <div class="panel"><h2>Tell him a rival offers more</h2><div class="body">${rivals || '<span class="t3">No club to name.</span>'}
          <div>${tfBtn("tell", "Tell him", {}, "glass", !offerOn || st.rival == null)}</div></div></div>
      </div></div>
    ${tfAct(offerOn ? tfBtn("walk", "Walk away", { club: t.club }) : "", offerOn ? `Their offer stands until ${esc(bid.until)}${st.picks.length ? ` · you ask ${asked} pts` : ""}` : esc(bid ? bid.words : ""), offerOn ? tfBtn("accept", "Accept his offer", { club: t.club }) + " " + tfBtn("counter", st.picks.length ? "Send this counter" : "Ask for more", { club: t.club }, "primary") : "")}`;
}

function tfClosing(x) {
  const s = x.shop, c = s && s.closing;
  if (!c) return `<div class="panel"><div class="body"><div class="empty">Nothing is closing. Accept an offer first.</div></div></div>`;
  const steps = [...c.owners, ...(c.deal ? [c.deal] : []), c.physical];
  const done = steps.filter((x) => x.done === true).length;
  const now = steps.findIndex((x) => x.done == null);
  const step = (k, st) => {
    const cls = st.done === true ? "done" : st.done === false ? "" : k === now ? "now" : "";
    const tag = st.done === true ? '<span class="p3-tag">Done</span>' : st.done === false ? '<span class="p3-tag gold">Failed</span>' : `<span class="p3-tag gold">${esc(st.due)}</span>`;
    const what = st.what === "The physical" ? "Their doctors read his record and examine him. A new finding can cut the price or end the deal." : st.what === "His new deal with them" ? `${c.ext ? `${c.ext.years} years at ${foM(c.ext.apy)} a year. ` : ""}The offer hangs on it: no deal, no trade.` : "A big move, and their owner has the final say on those.";
    return `<div class="${cls}"><span class="p3-n ${st.done === true ? "done" : ""}">${st.done === true ? "✓" : k + 1}</span><div><b>${esc(st.what)}</b><span class="t2">${what}</span></div>${tag}</div>`;
  };
  const cap = c.cap;
  const deadPct = cap.hit > 0 ? Math.min(100, (100 * cap.dead) / cap.hit) : 0;
  const revised = c.revised ? `<div class="p3-note"><b>Their doctors found something.</b> They now offer ${esc(c.revised.text)}. ${tfBtn("revised", "Take the new terms", { yes: 1 }, "primary")} ${tfBtn("revised", "Walk away", { yes: 0 })}</div>` : "";
  const result = c.result ? `<div class="p3-note"><b>${c.result === "Done" ? "The trade is done." : esc(c.result)}</b></div>` : "";
  return `${revised}${result}<div class="p3-grid p3-g21">
      <div class="panel"><h2><span class="p3-club">${mark(c.team, 22)} Agreed with ${teamLink(c.team)}</span> <small>${steps.length} step${steps.length === 1 ? "" : "s"} before it counts</small></h2>
        <div class="p3-cells" style="border-bottom:1px solid var(--hair)"><div><span class="lb">You give</span><b>${esc(x.player.name)}</b><span class="t3">${esc(x.player.pos)} · ${x.player.age} · ${foM(x.player.hit)}</span></div><div><span class="lb">You get</span><b>${c.package.worth} pts</b><span class="t3">${esc(c.package.text)}</span></div><div><span class="lb">Progress</span><b>${done} of ${steps.length} done</b><div class="p3-bar" style="margin-top:4px"><i class="f" style="width:${(100 * done) / steps.length}%"></i></div></div></div>
        <div class="p3-steps">${steps.map((st, k) => step(k, st)).join("")}</div></div>
      <div style="display:grid;gap:18px">
        <div class="panel"><h2>What it does to your cap</h2>
          <div class="p3-cells" style="grid-auto-flow:row;grid-template-columns:1fr 1fr"><div><span class="lb">Cap hit gone</span><b>${foM(cap.hit)}</b><span class="t3">this year</span></div><div><span class="lb">Dead money</span><b>${foM(cap.dead)}</b><span class="t3">his bonus${cap.paydown ? " and the pay-down" : ""}</span></div><div style="border-top:1px solid var(--hair)"><span class="lb">Pay-down</span><b>${cap.paydown ? foM(cap.paydown) : "None"}</b><span class="t3">${cap.paydown ? "stays on your cap" : "he fit their cap"}</span></div><div style="border-top:1px solid var(--hair)"><span class="lb">Coming back</span><b>${foM(cap.back)}</b><span class="t3">players in the deal</span></div></div>
          <div class="body" style="border-top:1px solid var(--hair)"><div class="p3-bar tall"><i class="f hatch" style="width:${deadPct}%"></i><i class="f" style="left:${deadPct}%;width:${100 - deadPct}%"></i></div><span class="t3">Hatched: dead money still on your cap. Solid: cap freed (${foM(cap.saved)} net this year).</span></div></div>
        <div class="panel"><h2>How he's taking it</h2><div class="body"><b class="tx">${x.mood.trust >= 60 ? "Ready to go" : x.mood.trust >= 35 ? "Unsettled" : "Angry"}</b><span class="t2">${esc(x.mood.words)}</span></div></div>
      </div></div>
    ${c.result ? tfAct(tfBtn("go_offers", "Back to the offers"), c.result === "Done" ? "The trade is on the books" : "The deal fell through", tfBtn("another", "Shop another player", {}, "primary")) : tfAct(tfBtn("go_offers", "Back to the offers"), "A trade counts only after the physical", tfBtn("noop", "Complete after the physical", {}, "primary", true))}`;
}

function tfDraft(x, st) {
  const d = x.draft;
  if (!d) return `<div class="panel"><div class="body"><div class="empty">The finder works in the war room on draft night. The draft isn't live.</div></div></div>`;
  const rail = d.strip.map((r) => `<div class="${r.now ? "now" : ""} ${r.mine ? "you" : ""}"><span class="lbl">${r.now ? "On the clock" : r.mine ? "You" : "&nbsp;"}</span><b>${r.pick}</b>${mark(r.team, 24)}<span>${esc(r.team.name.split(" ").pop())}</span></div>`).join("");
  const offers = d.offers.map((o, k) => `<tr class="${k === 0 ? "on" : ""}"><td>${tfClub(o.from)}</td><td>${o.gets.map(esc).join(" + ")}</td><td>${o.after ? `they want ${playerLink(o.after.id, o.after.name)}` : ""}</td><td><div class="p3-row-act">${tfBtn("daccept", "Accept", { k }, "primary")}</div></td></tr>`).join("");
  const nights = d.nights.map((n, k) => `<tr class="${k === 0 ? "on" : ""}"><td>${tfClub(n.team)}</td><td>${n.slots.map((s) => `pick ${s}`).join(" + ")}</td><td class="r">${n.worth}</td><td><div class="p3-row-act">${tfBtn("night", "Accept", { club: n.club }, "primary")}</div></td></tr>`).join("");
  const q = d.quotes[0];
  return `<div class="p3-sumbar"><div class="p3-cells">
      <div><span class="lb">On the clock</span><b>Pick ${d.pick}</b><span class="t3">${d.on_clock ? "you're on the clock" : d.my_next ? `you pick at ${d.my_next}` : "no more picks tonight"}</span></div>
      <div><span class="lb">Your pick</span><b>${d.my_next ? `Pick ${d.my_next}` : "-"}</b><span class="t3">round ${d.round ?? "-"}</span></div>
      <div><span class="lb">Player in play</span><b>${esc(x.player.name)} · ${esc(x.player.pos)} · ${x.player.age}</b><span class="t3">${x.player.points} pts to you</span></div>
    </div><div class="p3-rail">${rail}</div></div>
    ${d.on_clock ? `<div class="panel"><h2>Shopping your pick <small>every call holds only while the clock runs</small></h2><table class="p3-t"><thead><tr><th>Club</th><th>They give</th><th></th><th></th></tr></thead><tbody>${offers || `<tr><td colspan="4"><div class="empty">No calls for this pick.</div></td></tr>`}</tbody></table></div>` : ""}
    <div class="p3-grid p3-g11">
      <div class="panel"><h2>Shop a player tonight <small>for picks in this draft</small></h2><table class="p3-t"><thead><tr><th>Club</th><th>They give</th><th class="r">On your chart</th><th></th></tr></thead><tbody>${nights || `<tr><td colspan="4"><div class="empty">No club gives tonight's picks for ${esc(x.player.name)}.</div></td></tr>`}</tbody></table></div>
      <div class="panel"><h2>Moving up</h2>${d.quotes.length ? `<table class="p3-t"><tbody>${d.quotes.map((q) => `<tr><td><b>Pick ${q.pick}</b><small>${teamLink(q.from)}</small></td><td>for ${q.gets.map(esc).join(" + ")}</td><td>${tfBtn("up", "Move up", { slot: q.slot })}</td></tr>`).join("")}</tbody></table>` : '<div class="body"><span class="t3">Nobody between here and your pick will move down for what you have.</span></div>'}</div>
    </div>
    ${tfAct(q ? tfBtn("up", `Move up to ${q.pick}`, { slot: q.slot }) : "", "Clubs answer within minutes tonight", '<a class="st-btn small glass" data-go="warroom">Back to the war room</a>')}`;
}

function tfMove(x, st) {
  const m = x.move;
  if (!m) return "";
  const opts = m.options.slice(0, 8);
  if (st.dump == null && opts.length) st.dump = opts[0].club;
  const hit = Math.max(1, x.player.hit);
  const what = (o) => {
    const parts = [];
    if (o.paydown) parts.push(`<span class="p3-pk"><b>Pay ${o.paydown === 0.5 ? "half" : "a quarter"} down</b></span>`);
    if (o.back) parts.push(`<span class="p3-pk"><b>Their deal back</b><span>${esc(o.back.name)} · ${esc(o.back.pos)} · ${o.back.age}</span></span>`);
    parts.push(...o.picks.map(tfPick));
    if (!parts.length) parts.push('<span class="p3-pk"><b>Nothing</b><span>they take him as he is</span></span>');
    return `<div class="p3-pks">${parts.join("")}</div>`;
  };
  const rows = opts.map((o) => {
    const on = st.dump === o.club;
    const dp = Math.min(100, (100 * o.dead) / hit), sp = Math.max(0, Math.min(100 - dp, (100 * o.saved) / hit));
    return `<tr class="${on ? "on" : ""}" data-act="tf" data-tf="dump" data-club="${o.club}" style="cursor:pointer"><td style="width:34px"><span class="p3-rad" style="display:block;${on ? "box-shadow:inset 0 0 0 5px var(--fg)" : ""}"></span></td>
      <td>${tfClub(o.team)}</td><td>${what(o)}<div class="t3" style="margin-top:6px">${o.cost > 0 ? `${o.cost} pts on your chart` : "costs you no picks"}</div></td>
      <td class="r">${foM(o.dead)}</td><td class="r">${foM(o.saved)}</td>
      <td style="min-width:170px"><div class="p3-bar tall"><i class="f hatch" style="width:${dp}%"></i><i class="f" style="left:${dp}%;width:${sp}%"></i></div><div class="t3" style="margin-top:6px">${o.back ? "Their deal comes back to you" : `of his ${foM(x.player.hit)}`}</div></td></tr>`;
  }).join("");
  const sel = opts.find((o) => o.club === st.dump);
  const rel = m.release;
  return `${m.worth < 0 ? `<div class="p3-note"><b>Nobody takes his deal as it is.</b> Every club's read has him worth less than his pay (${m.worth} pts to you). To move him you pay with a pick, take a worse deal back or pay part of his salary down.</div>` : `<div class="p3-note"><b>His deal still has value</b> (${m.worth} pts to you). Shop him for offers instead; these are the prices to move him for nothing back.</div>`}
    <div class="panel"><h2>What it costs, club by club <small>cheapest first · hatched is your dead money, solid is cap saved this year</small></h2>
      <table class="p3-t"><thead><tr><th></th><th>Club</th><th>What it takes</th><th class="r">Your dead money</th><th class="r">Cap saved</th><th></th></tr></thead><tbody>${rows || `<tr><td colspan="6"><div class="empty">No club takes him on any terms.</div></td></tr>`}</tbody></table>
      ${rel ? `<div class="body" style="border-top:1px solid var(--hair)"><span class="t2"><b style="color:var(--fg)">Or release him.</b> No pick leaves: ${foM(rel.dead)} lands on your cap now${rel.later ? ` and ${foM(rel.later)} next year` : ""}, saving ${foM(rel.saved)} this year.</span><span class="t3">Real example: in 2017 Houston gave Cleveland a second-round pick to take Brock Osweiler's contract.</span></div>` : ""}</div>
    ${tfAct(`<a class="st-btn small glass" data-go="fo_cap">Open the cap sheet</a>`, sel ? `${esc(sel.team.name)} · ${sel.cost > 0 ? sel.cost + " pts" : "no picks"}` : "", tfBtn("dumpgo", "Make this offer", {}, "primary", !sel || !x.open))}`;
}

document.addEventListener("click", async (ev) => {
  const el = ev.target.closest("[data-act=tf]");
  if (!el || el.disabled) return;
  const d = el.dataset;
  const st = tfState();
  const club = d.club != null ? Number(d.club) : undefined;
  const run = async (cmd, args, after) => {
    try {
      const r = await api(cmd, args);
      if (r && r.text) toast(r.text);
      if (after) after(r);
      S.status = null;
    } catch (e) { toast(e.message || String(e)); }
    foRedraw();
  };
  switch (d.tf) {
    case "tab": st.tab = d.t; foRedraw(); break;
    case "pick": st.id = Number(d.id); st.clubs = null; st.tab = "shop"; foRedraw(); break;
    case "mode": st.open = d.v === "1"; foRedraw(); break;
    case "ask": st.ask = d.v; foRedraw(); break;
    case "paydown": st.paydown = Number(d.v); foRedraw(); break;
    case "agent": st.agent = !st.agent; foRedraw(); break;
    case "club": { const i = st.clubs.indexOf(club); if (i >= 0) st.clubs.splice(i, 1); else st.clubs.push(club); foRedraw(); break; }
    case "start": await run("tf_start", { id: st.id, open: st.open, ask: st.ask, paydown: st.paydown, agent: st.agent, clubs: st.clubs }, () => { st.tab = "offers"; toast(st.open ? "The word is out. Offers come in over the next days." : "Your calls are out. Answers come in a day or two."); }); break;
    case "go_offers": st.tab = "offers"; foRedraw(); break;
    case "another": Object.assign(st, { id: null, clubs: null, tab: "shop", club: null }); go("fo_finder", {}); break;
    case "talk": st.club = club; st.picks = []; st.rival = null; st.tab = "talks"; foRedraw(); break;
    case "tpick": { const i = st.picks.indexOf(d.k); if (i >= 0) st.picks.splice(i, 1); else st.picks.push(d.k); foRedraw(); break; }
    case "counter": {
      const picks = st.picks.map((k) => { const [year, round, orig, comp] = k.split(":").map(Number); return { year, round, orig, comp: comp === 1 }; });
      await run("tf_counter", { club, picks }, () => { st.picks = []; });
      break;
    }
    case "rival": st.rival = club; st.bluff = d.bluff === "1"; foRedraw(); break;
    case "tell": await run("tf_rival", { club: st.club, rival: st.rival, bluff: st.bluff }, () => { st.rival = null; }); break;
    case "accept": await run("tf_accept", { club }, () => { st.tab = "closing"; }); break;
    case "lapse": await run("tf_lapse", { club, walk: false }); break;
    case "walk": await run("tf_lapse", { club, walk: true }, () => { st.tab = "offers"; }); break;
    case "close": await run("tf_close", {}); break;
    case "revised": await run("tf_revised", { yes: d.yes === "1" }); break;
    case "dump": st.dump = club; foRedraw(); break;
    case "dumpgo": await run("tf_dump", { id: st.id, club: st.dump }, () => { st.dump = null; st.id = null; st.tab = "shop"; }); break;
    case "night": await run("tf_night", { id: st.id, club }); break;
    case "daccept": await run("draft_accept", { offer: Number(d.k) }); break;
    case "up": await run("draft_trade_up", { slot: Number(d.slot) }); break;
  }
});
