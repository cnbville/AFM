// Lane screens: the schedule page (update 2, lane 6). Registers screens.schedule.
"use strict";
/* A team's season in Press Box (SC1-SC15 and the minor bugs B1-B6):
 *
 *   the picker      every season in the team's book; the held ones open game
 *                   by game, the rest show their line (B5); the next season's
 *                   dates beside it.
 *   the shapes      the strip (SC9), the totals (SC10) and the strength of
 *                   schedule (SC11).
 *   Schedule        kickoff and network (SC1), both ranks (SC2), the
 *                   opponent's record going in and at the end (SC3), your
 *                   running record (SC4), byes and rest (SC5), the site and
 *                   weather (SC6), the line, the crowd against the seats
 *                   (SC7, B6), the result and its game page (B1), leaders and
 *                   your staff's player of the game (SC8), level tags (SC12),
 *                   the guarantee money (SC13), the scouting report (B2) and
 *                   the rivalry's series (B3).
 *   JV and freshman a high school's Thursday games (SC15).
 *   Future          the seasons ahead (SC14).
 *   Go to           Standings, Calendar, Next opponent (B4).
 *
 * A side panel opens on a row (the game), the network, the stadium, the
 * contract, the scouting report and the series.
 */

// Each drawn page's data, by the key on its root: a schedule or year board
// open beside the screen keeps its own (R27). The last few pages only.
const SCH_PAGES = new Map();
let SCH_SEQ = 0;
function schKeep(x) {
  const k = String(++SCH_SEQ);
  SCH_PAGES.set(k, x);
  while (SCH_PAGES.size > 8) SCH_PAGES.delete(SCH_PAGES.keys().next().value);
  return k;
}
const schOf = (el) => {
  const root = el && el.closest ? el.closest("[data-pk]") : null;
  return root ? SCH_PAGES.get(root.dataset.pk) || null : null;
};
const schRoot = (el) => (el && el.closest ? el.closest(".sch") : null);
const schNum = (n) => Number(n || 0).toLocaleString("en-US");
const schMoney = (n) => (n >= 1e6 ? `$${(Math.round(n / 1e4) / 100).toString()}M` : `$${Math.round(n / 1e3)}K`);
// The opponent's record: going in and how he finished, or, while the season
// runs, his record now (a game still to play has only that).
const schOppRec = (r) => {
  const now = r.opp_final !== "0-0" ? `now ${esc(r.opp_final)}` : "";
  if (!r.played && r.opp_now) return now;
  return `going in ${esc(r.opp_in)}${r.opp_now ? (now ? ` · ${now}` : "") : ` · finished ${esc(r.opp_final)}`}`;
};
const schCell = (k, v) => `<div class="sch-cell"><span class="st-label">${esc(k)}</span><span class="sch-v">${v}</span></div>`;
const schLk = (act, data, html) => `<a class="link" data-act="${act}" ${Object.entries(data).map(([k, v]) => `data-${k}="${esc(String(v))}"`).join(" ")} role="button" tabindex="0">${html}</a>`;

screens.schedule = async (a) => {
  const args = teamArgs(a);
  if (a.season) args.season = Number(a.season);
  const x = await api("schedule", args);
  const pk = schKeep(x);
  setColors(x.team.colors);
  title(`${x.team.name} schedule`, x.season ? String(x.season) : "");
  if (x.other) return schOther(x, a, pk);
  const lv = x.level;
  const tabList = ["Schedule", ...(x.jv ? ["JV and freshman"] : []), "Future schedules"];
  const tab = tabOf("schtab", tabList, a);
  const body = tab === "Schedule" ? schTable(x) : tab === "JV and freshman" ? schJv(x) : schFuture(x);
  return `<div class="pad sl sch sch-${lv}" data-pk="${pk}">
    ${schHead(x, a)}
    ${x.rows.length ? schShapes(x) : schSummary(x)}
    <div class="sch-tabs">${tabs("schtab", tabList, tab)}</div>
    ${body}
    <div class="sch-foot">${gotoFor("schedule", a, x)}</div>
    <aside class="sch-side" aria-label="Details" hidden><button class="sch-x" data-act="schclose" aria-label="Close">×</button><div class="sch-sbody"></div></aside>
  </div>`;
};

function schHead(x, a) {
  const targs = { kind: x.team.ref.kind, id: x.team.ref.id };
  const recent = x.seasons.slice(0, 6).reverse();
  const older = x.seasons.slice(6);
  const btn = (s) => linkTo("schedule", { ...targs, season: s.year }, `${s.year}${s.record ? ` <small>${esc(s.record)}</small>` : ""}`, `sch-yr${s.year === x.season ? " on" : ""}${s.held ? "" : " thin"}`);
  const pick = older.length
    ? `<select class="sch-older" data-act="schseason" data-kind="${esc(String(x.team.ref.kind))}" data-id="${x.team.ref.id}" aria-label="Earlier seasons"><option value="">Earlier</option>${older.map((s) => `<option value="${s.year}" ${s.year === x.season ? "selected" : ""}>${s.year}${s.record ? ` · ${esc(s.record)}` : ""}</option>`).join("")}</select>`
    : "";
  return `<header class="sch-head">
    <div class="sch-head-copy">
      <div class="st-label">${mark(x.team, 18)} ${teamA(x.team)}</div>
      <h1 class="sch-h1">${teamA(x.team)} schedule</h1>
      ${x.sub ? `<p class="sch-sub">${esc(x.sub)}</p>` : ""}
    </div>
    <div class="sch-seasons" role="group" aria-label="Season"><span class="st-label">Season</span>${recent.map(btn).join("")}${pick}${x.mine ? linkTo("year", {}, "The year", "sch-yr sch-yearlink") : ""}${x.next ? `<span class="sch-next">${esc(x.next)}</span>` : ""}</div>
  </header>`;
}

// A season the world keeps only as its line in the book.
function schSummary(x) {
  const s = x.summary;
  if (!s) return `<div class="sch-panel sch-empty">${esc(x.season ? `No games on record for ${x.season} yet. The schedule comes out before the season.` : "The schedule comes out with the season.")}</div>`;
  return `<div class="sch-panel sch-sumonly">
    <div class="sch-ph"><span class="st-label">${x.season} in the book</span><span class="st-label">Game by game is kept for this season and last, and for your own seasons</span></div>
    <div class="sch-hero"><b>${esc(s.record)}</b><span>${s.coach ? `under ${esc(s.coach)}` : ""}</span></div>
    <div class="sch-cells">${schCell("Points for", s.pf != null ? schNum(s.pf) : "-")}${schCell("Points against", s.pa != null ? schNum(s.pa) : "-")}${schCell("Title", s.title ? (x.level === "hs" ? "State champions" : x.level === "nfl" ? "Won the Super Bowl" : "National champions") : s.conf_title ? "Conference champions" : "-")}</div>
  </div>`;
}

// The strip, the totals and the strength of schedule (SC9-SC11).
function schShapes(x) {
  const games = x.strip.filter((s) => !s.bye);
  const maxM = Math.max(1, ...games.map((s) => Math.abs(s.margin)));
  const strip = x.strip.map((s) => {
    if (s.bye) return `<div class="sch-tile bye" title="Bye · ${esc(s.date)}"><i></i></div>`;
    const h = Math.max(6, (Math.abs(s.margin) / maxM) * 46);
    const cls = s.margin > 0 ? "w" : s.margin < 0 ? "l" : "t";
    return `<button class="sch-tile ${cls}" data-act="schrow" data-i="${s.i}" title="${esc(s.title)}"><i style="height:${h}%"></i><span>${s.margin > 0 ? "+" : ""}${s.margin}</span></button>`;
  }).join("");
  const t = x.totals;
  const sos = x.sos;
  const leagueWord = x.level === "nfl" ? "division" : x.level === "hs" ? "district" : x.league_name || "conference";
  return `<div class="sch-sum">
    <div class="sch-panel"><div class="sch-ph"><span class="st-label">Season strip</span><span class="st-label">${games.length ? `${games.length} game${games.length === 1 ? "" : "s"} with a score · biggest margin ${maxM}` : "No scores yet"}</span></div>
      <div class="sch-strip">${games.length ? `<div class="sch-base"></div>${strip}` : '<div class="sch-wait">The tiles fill in as the games are played.</div>'}</div>
      <div class="sch-legend"><span><i class="w"></i>Win, height = margin</span><span><i class="l"></i>Loss</span><span><i class="b"></i>Bye</span><span>Click a tile for the game</span></div></div>
    <div class="sch-panel"><div class="sch-ph"><span class="st-label">Season totals</span><span class="st-label">${esc(x.finish || "")}</span></div>
      <div class="sch-hero"><b>${esc(t.overall)}</b><span>overall${t.league !== "0-0" ? ` · ${esc(t.league)} ${esc(leagueWord)}` : ""}</span></div>
      <div class="sch-cells">${t.cells.map(([k, v]) => schCell(k, v)).join("")}</div></div>
    <div class="sch-panel sch-sos"><div class="sch-ph"><span class="st-label">Strength of schedule</span></div>
      ${sos ? `<div class="sch-hero"><b>${esc(sos.rank_text)}</b><span>hardest of ${schNum(sos.of)} · ${esc(sos.pool)}</span></div>
      <div class="sch-sosline">Opponents ${esc(sos.record)} (${esc(sos.pct)})</div>
      <div class="sch-bar"><i style="width:${sos.bar.toFixed(1)}%"></i></div><div class="sch-ticks"><span>Easiest</span><span>Hardest</span></div>
      <div class="sch-note">Opponents' records leave out their games against you.${x.season && x.rows.some((r) => r.kind === "game" && !r.played) ? " It moves as the season is played." : ""}</div>` : '<div class="sch-wait">Ranked once games are played.</div>'}</div>
  </div>`;
}

function schRank(r) {
  return r ? `<span class="sch-rk">No. ${r}</span>` : "";
}

function schTags(r, x) {
  const out = [];
  for (const t of r.tags || []) {
    if (t === "Playoffs") out.push(`<span class="sch-tag gold">Playoffs</span>`);
    else out.push(`<span class="sch-tag">${esc(t)}</span>`);
  }
  if (r.name) {
    if (r.rivalry != null || (x.level === "hs" && !r.playoff && r.name)) out.push(schLk("schseries", { kind: r.opp.ref.kind, id: r.opp.ref.id }, `<span class="sch-tag gold">${esc(r.name.charAt(0).toUpperCase() + r.name.slice(1))}</span>`) + (r.trophy ? ` <span class="sch-trophy">${esc(r.trophy)}</span>` : ""));
    else out.push(`${compIn(r.name)}<span class="sch-tag gold">${esc(r.name)}</span>`);
  }
  return out.join("");
}

function schTable(x) {
  if (!x.rows.length) return `<div class="sch-panel sch-empty">${x.summary ? "This season is kept as its line in the book: the record above, its games no longer." : "The schedule comes out with the season."}</div>`;
  const hs = x.level === "hs";
  const nfl = x.level === "nfl";
  const cols = hs ? 6 : 7;
  const leagueWord = nfl ? "division" : hs ? "district" : "conf";
  const rows = x.rows.map((r) => {
    if (r.kind === "bye") return `<tr class="sch-bye"><td colspan="${cols}">Bye · ${esc(r.date)}</td></tr>`;
    const res = r.result || {};
    const scoreHtml = res.us != null ? `<span class="sch-res ${res.wl === "W" ? "w" : ""}"><span class="sch-wl">${esc(res.wl)}</span>${res.us}-${res.them}</span>` : res.wl ? `<span class="sch-res ${res.wl === "W" ? "w" : ""}"><span class="sch-wl">${esc(res.wl)}</span></span>` : "";
    const result = res.text
      ? `<span class="sch-dim">${esc(res.text)}</span>`
      : scoreHtml
        ? r.game != null
          ? `<a class="link" data-go="game" data-index="${r.game}">${scoreHtml}</a><div class="sch-l2">${linkTo("game", { index: r.game }, "Game page")}</div>`
          : `${scoreHtml}<div class="sch-l2">${res.us != null ? "Score in the books" : "Score not reported"}</div>`
        : `<span class="sch-dim">${r.kick ? esc(r.kick) : "Time TBA"}</span>`;
    const ours = r.ranks && r.ranks[0] ? ` <span class="sch-rk" title="Your rank at kickoff">you No. ${r.ranks[0]}</span>` : "";
    const scout = x.can_scout && !r.opp.mine ? schLk("schscout", { kind: r.opp.ref.kind, id: r.opp.ref.id }, "Scouting report") : "";
    const opp = `<div class="sch-l1">${esc(r.at)} ${schRank(r.ranks && r.ranks[1])}${logoIn(r.opp)}${teamA(r.opp)}${schTags(r, x)}</div><div class="sch-l2">${[schOppRec(r) + ours, scout].filter(Boolean).join(" · ")}</div>`;
    const where = `<div class="sch-l1">${schLk("schvenue", { i: r.i }, esc(r.stadium))}</div><div class="sch-l2">${esc(r.place)}${r.weather ? ` · ${esc(r.weather)}` : ""}</div>`;
    const ln = r.line;
    const line = hs ? "" : `<td>${ln ? `<div class="sch-l1 num">${esc(ln.spread)} <span class="sch-dim">O/U ${ln.total}</span></div>${ln.against ? `<div class="sch-l2">${esc(ln.against)}</div>` : ""}` : '<span class="sch-dim">–</span>'}</td>`;
    const pct = r.crowd && r.seats ? r.crowd / r.seats : null;
    const crowd = r.crowd
      ? `<div class="sch-l1 num">${schNum(r.crowd)}${pct != null ? ` <span class="sch-dim">· ${Math.round(pct * 100)}%</span>` : ""}</div>${pct != null ? `<div class="sch-bar thin ${pct > 1 ? "over" : ""}"><i style="width:${Math.min(100, pct * 100).toFixed(1)}%"></i></div><div class="sch-l2 num">of ${schNum(r.seats)}</div>` : `<div class="sch-l2">neutral site</div>`}`
      : `<span class="sch-dim">${r.seats ? `${schNum(r.seats)} seats` : "–"}</span>`;
    const record = r.record ? `<div class="sch-l1 num">${esc(r.record)}</div><div class="sch-l2 num">${r.league_record ? `${esc(r.league_record)} ${esc(leagueWord)}` : "&nbsp;"}</div>` : "";
    const L = r.leaders || {};
    const lead = [];
    if (L.pass) lead.push(`<span>Pass ${playerLink(L.pass.id, L.pass.name)} ${esc(L.pass.line)}</span>`);
    if (L.rush) lead.push(`<span>Rush ${playerLink(L.rush.id, L.rush.name)} ${esc(L.rush.line)}</span>`);
    if (L.rec) lead.push(`<span>Rec ${playerLink(L.rec.id, L.rec.name)} ${esc(L.rec.line)}</span>`);
    if (L.opp) lead.push(`<span>${esc(r.opp.name)}: ${playerLink(L.opp.id, L.opp.name)} ${esc(L.opp.line)}</span>`);
    if (L.pog) lead.push(`<span class="sch-pog"><b>Player of the game</b>${playerLink(L.pog.id, L.pog.name)} · ${esc(L.pog.line)}</span>`);
    if (r.fee) lead.push(`<span class="sch-fee">${schLk("schfee", { i: r.i }, esc(r.fee.text))}</span>`);
    if (r.why && r.why.length) lead.push(`<span class="sch-why">Why it ${r.played ? "mattered" : "matters"}: ${esc(r.why.join(", "))}</span>`);
    const rest = r.rest ? `<span class="sch-rest ${r.rest.short ? "short" : ""}">${esc(r.rest.text)}</span>` : "";
    const when = r.date ? `<div class="sch-l1">${esc(r.date)}</div><div class="sch-l2">${r.kick ? esc(r.kick) : "Time TBA"} · ${r.tv ? schLk("schtv", { i: r.i }, esc(r.tv)) : "TV TBA"}</div>${rest}` : `<div class="sch-l1">Date TBA</div><div class="sch-l2">Out ${esc(x.release)}</div>`;
    return `<tr class="sch-g" data-act="schrow" data-i="${r.i}">
      <td>${when}</td><td>${opp}</td><td>${where}</td>${line}<td class="sch-crowd">${crowd}</td><td>${result}</td><td>${record}</td></tr>
      ${lead.length ? `<tr class="sch-subrow" data-i="${r.i}"><td colspan="${cols}"><div class="sch-subgrid">${lead.join("")}</div></td></tr>` : ""}`;
  }).join("");
  const head = `<tr><th>Date, kickoff, TV</th><th>Opponent</th><th>Where, weather</th>${hs ? "" : `<th title="Your side of the closing line, the total, and how the game went against it">Line</th>`}<th>Crowd</th><th>Result</th><th>Record</th></tr>`;
  const note = x.dates_hidden ? `<div class="sch-note">The league releases the dates on ${esc(x.release)}. Until then the season is its opponents, home and away.</div>` : x.level === "college" ? `<div class="sch-note">Ranks are ${esc(x.rank_label || "AP")} at kickoff. Kickoffs and networks are set 6 to 12 days ahead, the first weeks in the summer.</div>` : x.level === "hs" ? `<div class="sch-note">Ranks are the state media poll in ${esc(x.rank_label || "the class")} at kickoff.</div>` : "";
  return `<div class="sch-tbl"><table>${head}${rows}</table></div>${note}`;
}

function schJv(x) {
  const rows = (x.jv || []).map((r) => {
    const res = r.result;
    return `<tr><td><div class="sch-l1">${esc(r.date)}</div><div class="sch-l2">${esc(r.kick)} · the night before the varsity</div></td><td><span class="sch-tag">${esc(r.squad)}</span></td><td><div class="sch-l1">${esc(r.at)} ${teamA(r.opp)}</div>${r.note ? `<div class="sch-l2">${esc(r.note)}</div>` : ""}</td><td>${res ? `<span class="sch-res ${res.wl === "W" ? "w" : ""}"><span class="sch-wl">${esc(res.wl)}</span>${res.us}-${res.them}</span>` : `<span class="sch-dim">${r.note ? "No game" : "To play"}</span>`}</td></tr>`;
  }).join("");
  return `${rows ? `<div class="sch-tbl"><table class="narrow"><tr><th>Date</th><th>Team</th><th>Opponent</th><th>Result</th></tr>${rows}</table></div>` : '<div class="sch-panel sch-empty">No JV or freshman games this season.</div>'}
    <div class="sch-note">JV and freshmen play Thursday against Friday's opponent, at the other school's field. Moving a player up or down shows here first (${linkTo("school", {}, "Your school")}).</div>`;
}

function schFuture(x) {
  if (!x.future.length) return `<div class="sch-panel sch-empty">Nothing on the books yet.</div>`;
  return `<div class="sch-fut">${x.future.map((f) => `<div class="sch-panel"><h3>${esc(f.title)}</h3><ul>${f.lines.map((l) => `<li><span>${l.opp ? teamA(l.opp, l.what) : esc(l.what)}</span><span>${esc(l.note)}</span></li>`).join("")}</ul></div>`).join("")}</div>
    <div class="sch-note">${esc(x.future_note)}</div>`;
}

// Another league's season: the plain list, your own games' pages under it.
function schOther(x, a, pk) {
  const games = x.games.map((g) => `<tr><td>${esc(g.week ? "Wk " + g.week : "")} ${esc(g.date || "")}</td><td>${esc(g.at)} ${teamA(g.opp)}${g.name ? ` <span class="sch-tag gold">${esc(g.name)}</span>` : ""}</td><td>${g.result ? `<b>${esc(g.result)}</b>` : '<span class="sch-dim">To play</span>'}</td></tr>`).join("");
  const kept = (x.box_scores || []).map((g) => `<li><span><a class="link" data-go="game" data-index="${g.index}">${esc(g.away)} ${g.score[1]} at ${esc(g.home)} ${g.score[0]}</a></span><span>${esc(g.date)}</span></li>`).join("");
  return `<div class="pad sl sch" data-pk="${pk}"><header class="sch-head"><div class="sch-head-copy"><h1 class="sch-h1">${teamA(x.team)} schedule</h1></div></header>
    <div class="sch-tbl"><table class="narrow"><tr><th>Date</th><th>Opponent</th><th>Result</th></tr>${games || '<tr><td colspan="3"><div class="empty">The schedule comes out with the season.</div></td></tr>'}</table></div>
    ${kept ? `<div class="sch-fut"><div class="sch-panel"><h3>Game pages</h3><ul>${kept}</ul></div></div>` : ""}
    <div class="sch-foot">${gotoFor("schedule", a, x)}</div></div>`;
}

// ---------- the side panel ----------
function schOpen(root, html, i) {
  const side = root && root.querySelector(".sch-side");
  if (!side) return;
  side.querySelector(".sch-sbody").innerHTML = html;
  side.hidden = false;
  requestAnimationFrame(() => side.classList.add("open"));
  root.querySelectorAll(".sch-g.on, .sch-subrow.on, .sch-tile.on").forEach((n) => n.classList.remove("on"));
  if (i != null) root.querySelectorAll(`.sch-g[data-i="${i}"], .sch-subrow[data-i="${i}"], .sch-tile[data-i="${i}"]`).forEach((n) => n.classList.add("on"));
}
// One page's panel, or every open one (Escape).
function schClose(root) {
  for (const r of root ? [root] : document.querySelectorAll(".sch")) {
    const side = r.querySelector(".sch-side");
    if (side) { side.classList.remove("open"); side.hidden = true; }
    r.querySelectorAll(".sch-g.on, .sch-subrow.on, .sch-tile.on").forEach((n) => n.classList.remove("on"));
  }
}
const schRow = (x, i) => x && x.rows && x.rows.find((r) => r.kind === "game" && r.i === Number(i));

function schGamePanel(r, x) {
  const res = r.result || {};
  const head = res.us != null ? `${res.wl} ${res.us}-${res.them} ${esc(r.at)} ${esc(r.opp.name)}` : res.wl ? `${res.wl} ${esc(r.at)} ${esc(r.opp.name)}` : `${esc(r.at)} ${esc(r.opp.name)}`;
  const L = r.leaders || {};
  const leaders = [L.pass && `Pass: ${playerLink(L.pass.id, L.pass.name)} ${esc(L.pass.line)}`, L.rush && `Rush: ${playerLink(L.rush.id, L.rush.name)} ${esc(L.rush.line)}`, L.rec && `Rec: ${playerLink(L.rec.id, L.rec.name)} ${esc(L.rec.line)}`, L.pog && `<b class="sch-pogk">Player of the game</b> ${playerLink(L.pog.id, L.pog.name)}, ${esc(L.pog.line)}`].filter(Boolean);
  const pct = r.crowd && r.seats ? ` · ${Math.round((r.crowd / r.seats) * 100)}%` : "";
  // A game called off is over without a score.
  const called = !r.played && r.result && r.result.text ? r.result.text : "";
  const over = r.played || !!called;
  return `<div class="st-label">${r.played ? "The game" : called ? esc(called) : "Coming up"}</div><h2>${head}</h2>
    <div class="sch-lede">${esc(r.date || "Date TBA")} · ${esc(r.stadium)}, ${esc(r.place)}</div>
    <div class="sch-cells two">${schCell("Kickoff", `${esc(r.kick || "TBA")} · ${esc(r.tv || "TV TBA")}`)}${schCell("Weather", esc(r.weather || (over ? "–" : "At kickoff")))}${schCell("Crowd", r.crowd ? `${schNum(r.crowd)}${pct}` : "–")}${schCell("Rest", esc(r.rest ? r.rest.text : "–"))}${r.line ? schCell("Line", `${esc(r.line.spread)}, O/U ${r.line.total}`) : ""}${schCell(r.ranks && r.ranks[1] ? "Their rank" : "Their record", r.ranks && r.ranks[1] ? `No. ${r.ranks[1]}` : `${esc(r.opp_in)} going in`)}</div>
    ${leaders.length ? `<div class="sch-sec"><span class="st-label">Leaders</span>${leaders.map((l) => `<p>${l}</p>`).join("")}</div>` : r.played ? `<div class="sch-sec"><p class="sch-dim">${r.mine ? "The box score is no longer kept for this game." : "Box scores are kept for your own games."}</p></div>` : ""}
    ${r.why && r.why.length ? `<div class="sch-sec"><span class="st-label">Why it ${over ? "mattered" : "matters"}</span><p>${esc(r.why.join(", "))}</p></div>` : ""}
    <div class="sch-acts">${r.game != null ? linkTo("game", { index: r.game }, "Game page", "st-btn small") : ""}${linkTo("team", { kind: r.opp.ref.kind, id: r.opp.ref.id }, "Team page", "st-btn small glass")}${x.can_scout && !r.opp.mine ? `<button class="st-btn small glass" data-act="schscout" data-kind="${r.opp.ref.kind}" data-id="${r.opp.ref.id}">Scouting report</button>` : ""}</div>`;
}

async function schScoutPanel(root, kind, id) {
  schOpen(root, '<div class="sch-dim">Pulling your staff\'s report…</div>');
  let s;
  try { s = await api("schedule_scout", { kind, id: Number(id) }); } catch (e) { schOpen(root, `<div class="st-label">Your scouting report</div><p>${esc(e.message || e)}</p>`); return; }
  const rep = s.report || {};
  const secs = (rep.sections || []).slice(0, 5).map((sec) => `<div class="sch-sec"><span class="st-label">${esc(sec.unit)} · ${esc(sec.title)}</span>${(sec.lines || []).slice(0, 3).map((l) => `<p>${esc(l.text)}${l.sure ? ` <span class="sch-dim">(${esc(l.sure)})</span>` : ""}</p>`).join("")}</div>`).join("");
  const none = !rep.games;
  schOpen(root, `<div class="st-label">Your scouting report</div><h2>${esc(s.team.name)}</h2>
    <div class="sch-lede">${none ? "No film yet: your staff has no tape of them this season." : `From ${rep.games} tape${rep.games === 1 ? "" : "s"}, ${schNum(rep.snaps)} snaps${rep.thin ? " · a thin sample, so the reads are loose" : ""}`}</div>
    ${rep.staff ? `<p class="sch-dim">${esc(rep.staff)}</p>` : ""}${secs}
    <div class="sch-acts">${s.next ? linkTo("gameday", {}, "Full report in game week", "st-btn small") : ""}${linkTo("team", { kind: s.team.ref.kind, id: s.team.ref.id }, "Team page", "st-btn small glass")}</div>`);
}

async function schSeriesPanel(root, x, kind, id) {
  schOpen(root, '<div class="sch-dim">Opening the series…</div>');
  let s;
  try { s = await api("schedule_series", { kind, id: Number(id), of: x ? x.team.ref.id : undefined }); } catch (e) { schOpen(root, `<div class="st-label">Series history</div><p>${esc(e.message || e)}</p>`); return; }
  const lead = s.wins === s.losses ? `Tied ${s.wins}-${s.losses}${s.ties ? `-${s.ties}` : ""}` : s.wins > s.losses ? `${esc(s.me.name)} leads ${s.wins}-${s.losses}${s.ties ? `-${s.ties}` : ""}` : `${esc(s.rival.name)} leads ${s.losses}-${s.wins}${s.ties ? `-${s.ties}` : ""}`;
  const game = s.game.charAt(0).toUpperCase() + s.game.slice(1);
  schOpen(root, `<div class="st-label">Series history</div><h2>${esc(game)}</h2>
    <div class="sch-lede">${s.trophy ? `${esc(s.trophy)} · ` : ""}${s.real ? "the all-time record" : s.since ? `on record since ${s.since}` : ""}</div>
    <div class="sch-cells two">${schCell("Series", lead)}${schCell(s.trophy ? "Trophy holder" : "Last won by", s.holder ? teamA(s.holder) : "–")}</div>
    <div class="sch-sec"><span class="st-label">Meetings on record</span>${s.meetings.length ? s.meetings.map((m) => `<p>${m.year} · ${esc(m.text)} at ${esc(m.at)}</p>`).join("") : '<p class="sch-dim">The meetings before last season live in the series record above.</p>'}</div>
    <div class="sch-acts">${linkTo("traditions", { tab: "Rivalries" }, "Rivalries", "st-btn small glass")}${teamA(s.rival) ? linkTo("team", { kind: s.rival.ref.kind, id: s.rival.ref.id }, "Their team page", "st-btn small glass") : ""}</div>`);
}

function schFeePanel(r, x) {
  const f = r.fee;
  const how = f.how === "paid" ? `${esc(x.team.name)} paid ${esc(r.opp.name)}` : f.how === "received" ? `${esc(r.opp.name)} paid ${esc(x.team.name)}` : "The game's organisers paid each side";
  const kind = f.how === "neutral" ? "A neutral-site kickoff game." : "A one-off game: no return trip.";
  return `<div class="st-label">Game contract</div><h2>${esc(r.opp.name)} · ${schMoney(f.amount)}</h2><div class="sch-lede">${kind}</div>
    <div class="sch-cells two">${schCell("Paid", how)}${schCell("Signed", f.signed ? String(f.signed) : "Booked late, at the going rate")}</div>
    <p class="sch-dim">Real one-off games pay about $300K-700K for an FCS visitor and $1-2M for a Group of 5 visitor; a Group of 5 host pays less. A neutral site pays each side a few million.</p>
    <div class="sch-acts">${linkTo("money", x.mine ? {} : { kind: x.team.ref.kind, id: x.team.ref.id }, "The books", "st-btn small glass")}</div>`;
}

function schVenuePanel(r) {
  return `<div class="st-label">Stadium</div><h2>${esc(r.stadium)}</h2><div class="sch-lede">${esc(r.place)}</div>
    <div class="sch-cells two">${schCell("Seats", r.seats ? schNum(r.seats) : "–")}${schCell("Roof", r.indoor ? "Indoors" : "Open air")}${schCell("Crowd", r.crowd ? schNum(r.crowd) : "–")}${schCell("Weather", esc(r.weather || "–"))}</div>`;
}

function schTvPanel(r, x) {
  const lv = x.level;
  const why = lv === "hs" ? "The school's games on the town's station; the NFHS Network streams most games now." : lv === "nfl" ? "Set with the schedule in May; late-season games can be flexed into prime time." : "Most kickoffs and networks are picked 6 or 12 days ahead; the biggest game of a window goes to its biggest network.";
  return `<div class="st-label">Broadcast</div><h2>${esc(r.tv)}</h2><div class="sch-lede">${esc(r.date || "")} · ${esc(r.kick || "")}</div><p class="sch-dim">${esc(why)}</p>`;
}

Object.assign(CORE_MORE, {
  schrow: (el, ev) => {
    if (ev && ev.target && ev.target.closest("a, button") && ev.target.closest("a, button") !== el) return;
    const x = schOf(el);
    const r = schRow(x, el.dataset.i);
    if (r) schOpen(schRoot(el), schGamePanel(r, x), r.i);
  },
  schtv: (el) => { const x = schOf(el); const r = schRow(x, el.dataset.i); if (r && r.tv) schOpen(schRoot(el), schTvPanel(r, x), r.i); },
  schvenue: (el) => { const r = schRow(schOf(el), el.dataset.i); if (r) schOpen(schRoot(el), schVenuePanel(r), r.i); },
  schfee: (el) => { const x = schOf(el); const r = schRow(x, el.dataset.i); if (r && r.fee) schOpen(schRoot(el), schFeePanel(r, x), r.i); },
  schscout: (el) => schScoutPanel(schRoot(el), el.dataset.kind, el.dataset.id),
  schseries: (el) => schSeriesPanel(schRoot(el), schOf(el), el.dataset.kind, el.dataset.id),
  schclose: (el) => schClose(schRoot(el)),
});

// The earlier seasons' list opens one (the team on the list itself);
// Escape closes the side panels.
document.addEventListener("change", (ev) => {
  const t = ev.target;
  if (!t.matches || !t.matches(".sch-older") || !t.value) return;
  go("schedule", { kind: t.dataset.kind, id: Number(t.dataset.id), season: Number(t.value) });
});
document.addEventListener("keydown", (ev) => {
  if (ev.key === "Escape" && document.querySelector(".sch-side.open")) schClose();
});

// ---------- the year (V13, board 13 v2) ----------
/* Your football year as a ribbon, from the day after one Super Bowl to the
 * next: your games, the training blocks your level's rules set, the
 * roster's dates, recruiting (scouting at the NFL) and the rules lane, with
 * today marked. Below: this week's practice plan against the hours allowed,
 * the picked block's dates and rules, and what's coming up. */

const YR_KINDS = [["prac", "Practice"], ["film", "Film"], ["meet", "Meetings"], ["lift", "Strength"], ["game", "Game"]];
CRUMBS.year = "The year";
GOTO.year = () => [["Schedule", "schedule"], ["Calendar", "calendar"], ["Training", "training"]];

// The year is Training's "The year" tab (SC1), drawn under Training's own
// head; its old key opens it there.
screens.year = (a) => {
  const args = { ...(a || {}), tab: "The year" };
  if (!S.besideDrawing && S.route.screen === "year") { S.route.screen = "training"; S.route.args = args; }
  return screens.training(args);
};
// The year board on its own (the ribbon, the week and the phase card), for
// the top of Training's "The year" tab.
async function yrBoard(a = {}) {
  const x = await api("year_board", {});
  const pk = schKeep(x);
  const sel = Math.min(Math.max(0, a && a.pick != null ? Number(a.pick) : x.cur), x.phases.length - 1);
  requestAnimationFrame(yrFit);
  return `<div class="sl yr yr-in yr-${x.level}" data-pk="${pk}">
    ${yrRibbon(x, sel)}
    <div class="yr-low">
      ${yrWeek(x.week)}
      <aside class="yr-panel yr-side">
        <div class="yr-card">${yrCard(x.phases[sel])}</div>
        <div class="yr-next"><span class="st-label">Coming up</span>${x.ahead.map((n) => `<div class="yr-nx"><span class="yr-nd"><b>${n.d}</b><span class="st-label">${esc(n.m)}</span></span><span class="yr-nt"><b>${esc(n.t)}</b><span>${esc(n.s)}${n.in == null ? "" : n.in === 0 ? " · today" : ` · in ${n.in} day${n.in === 1 ? "" : "s"}`}</span></span></div>`).join("") || `<p class="sch-dim">Nothing more this year. The next football year starts ${esc(x.next_year)}.</p>`}</div>
      </aside>
    </div>
  </div>`;
}

function yrRibbon(x, sel) {
  const pc = (f) => `${(f * 100).toFixed(2)}%`;
  const months = x.months.map((m) => `<span class="yr-mline" style="left:${pc(m.x)}"></span><span class="st-label yr-mlab" style="left:calc(${pc(m.x)} + 6px)">${esc(m.n)}</span>`).join("");
  const lanes = x.lanes.map((n, i) => `<span class="st-label yr-lane" style="top:${[30, 72, 114, 156, 194][i]}px">${esc(n)}</span>`).join("");
  const note = x.games_note || (x.games.length ? "" : "No games on the books yet");
  const games = (note ? `<span class="yr-gnote">${esc(note)}</span>` : "") + x.games.map((g) => `<a class="yr-g ${g.r === "W" ? "w" : g.r === "L" ? "l" : g.r === "T" ? "t" : ""}${g.next ? " next" : ""}${g.post ? " post" : ""}" data-go="schedule" data-args="{}" data-x="${g.x}" style="left:${pc(g.x)}" title="${esc(g.title)}">${esc(g.o)}</a>`).join("");
  const phases = x.phases.map((p, k) => `<button class="yr-ph${p.cur ? " cur" : ""}${k === sel ? " on" : ""}${p.past ? " past" : ""}" data-act="yrpick" data-k="${k}" style="left:${pc(p.x)};top:${64 + p.lane * 42}px;width:${pc(p.w)}" title="${esc(p.n)} · ${esc(p.d)}" aria-label="${esc(p.n)}"><span class="yr-mk"></span><span class="yr-pl" data-n="${esc(p.n)}" data-s="${esc(p.s)}">${esc(p.n)}</span></button>`).join("");
  const words = Object.fromEntries(x.legend.map(([k, n]) => [k, n]));
  const rules = x.rules.map((r) => `<span class="yr-r p-${r.k}" style="left:${pc(r.x)};width:calc(${pc(r.w)} - 2px)" title="${esc(words[r.k] || "")} · ${esc(r.d)}"></span>`).join("");
  const legend = x.legend.map(([k, n]) => `<span><i class="yr-r p-${k}"></i>${esc(n)}</span>`).join("");
  return `<section class="yr-panel">
    <div class="yr-ph-head"><div><b class="yr-pt">The year at a glance</b><span class="sch-dim">click any block for its dates and rules</span></div>
      <div class="yr-legend"><span>${x.level === "nfl" ? "The CBA:" : "Recruiting rules:"}</span>${legend}</div></div>
    <div class="yr-rib">${lanes}<div class="yr-track">${months}${games}${phases}${rules}
      <span class="yr-today" style="left:${pc(x.today_x)}"></span><span class="st-label yr-todayl" style="${x.today_x > 0.92 ? `right:calc(${pc(1 - x.today_x)} + 6px)` : `left:calc(${pc(x.today_x)} + 6px)`}">Today</span></div></div>
  </section>`;
}

function yrWeek(wk) {
  const PX = 0.62;
  const cols = wk.days.map((d) => {
    const blocks = d.blocks.map((b) => `<span class="yr-b k-${b.k}" style="min-height:${Math.max(20, Math.round(b.min * PX))}px" title="${esc(b.n)} · ${b.min} min">${esc(b.n)}</span>`).join("");
    return `<div class="yr-day${d.today ? " today" : ""}"><span class="yr-dn">${esc(d.n)}</span>${blocks || '<span class="yr-off">Off</span>'}</div>`;
  }).join("");
  const hrs = wk.hours ? `Player hours <b>${wk.hours.used}</b> of ${wk.hours.allowed} allowed` : `On the field <b>${esc(wk.field)}</b>`;
  return `<section class="yr-panel yr-week">
    <div class="yr-ph-head"><div><b>This week</b><span class="sch-dim">${esc(wk.title)}</span></div><span class="sch-dim">${hrs}</span></div>
    <div class="yr-days">${cols}</div>
    <div class="yr-kinds">${YR_KINDS.map(([k, n]) => `<span><i class="yr-b k-${k}"></i>${n}</span>`).join("")}<span class="sch-dim">${esc(wk.block)} · ${wk.yours ? "your plan" : "your staff's plan"} · ${linkTo("training", {}, "Training")}</span></div>
  </section>`;
}

function yrCard(p) {
  if (!p) return '<p class="sch-dim">Nothing on the year yet.</p>';
  return `<div class="yr-ch"><span class="st-label">${esc(p.lane_name)}</span><span class="sch-dim">${esc(p.d)}</span></div>
    <b class="yr-cn">${esc(p.n)}</b>
    <div class="yr-kv">${p.kv.map((r) => `<div><span>${esc(r.k)}</span><b>${esc(r.v)}</b></div>`).join("")}</div>`;
}

// Fit the ribbon to its width: game tiles as wide as a week allows; each
// bar's name inside it, else its short label, else just after the bar while
// that clears the next one, else none (the card and the tooltip name it);
// the Today label hides the month name it would sit on. Every board drawn
// (one may be open beside the screen).
function yrFit() {
  document.querySelectorAll(".yr-track").forEach(yrFitOne);
}
function yrFitOne(tr) {
  const W = tr.clientWidth;
  const xs = [...tr.querySelectorAll(".yr-g")].map((g) => Number(g.dataset.x)).sort((a, b) => a - b);
  const gaps = xs.slice(1).map((x, i) => (x - xs[i]) * W).filter((g) => g > 0).sort((a, b) => a - b);
  const gw = gaps.length ? Math.max(14, Math.min(22, Math.floor(gaps[Math.floor(gaps.length * 0.2)] - 2))) : 22;
  tr.style.setProperty("--yr-gw", `${gw}px`);
  tr.classList.toggle("tight", gw < 20);
  const lanes = {};
  tr.querySelectorAll(".yr-ph").forEach((b) => (lanes[b.style.top] = lanes[b.style.top] || []).push(b));
  for (const bars of Object.values(lanes)) {
    bars.sort((a, b) => a.offsetLeft - b.offsetLeft);
    bars.forEach((b, i) => {
      const l = b.querySelector(".yr-pl");
      const end = i + 1 < bars.length ? bars[i + 1].offsetLeft : W;
      const room = { in: b.offsetWidth - 16, out: end - (b.offsetLeft + b.offsetWidth) - 14 };
      b.classList.remove("out", "bare");
      let put = "bare";
      for (const [t, where] of [[l.dataset.n, "in"], [l.dataset.s, "in"], [l.dataset.n, "out"], [l.dataset.s, "out"]]) {
        l.textContent = t;
        if (l.getBoundingClientRect().width <= room[where]) { put = where; break; }
      }
      if (put !== "in") b.classList.add(put);
    });
  }
  const t = tr.querySelector(".yr-todayl");
  if (t) {
    const r = t.getBoundingClientRect();
    tr.querySelectorAll(".yr-mlab").forEach((m) => {
      m.classList.remove("bare");
      const q = m.getBoundingClientRect();
      m.classList.toggle("bare", q.right > r.left - 4 && q.left < r.right + 4);
    });
  }
}

Object.assign(CORE_MORE, {
  yrpick: (el) => {
    const k = Number(el.dataset.k);
    const x = schOf(el);
    const root = el.closest(".yr");
    if (!x || !x.phases[k] || !root) return;
    root.querySelectorAll(".yr-ph.on").forEach((n) => n.classList.remove("on"));
    el.classList.add("on");
    const c = root.querySelector(".yr-card");
    if (c) c.innerHTML = yrCard(x.phases[k]);
    // The pick stays when Training redraws its "The year" tab.
    if (!el.closest(".beside") && S.route && S.route.screen === "training") S.route.args = { ...S.route.args, pick: k };
  },
});
window.addEventListener("resize", () => { if (document.querySelector(".yr-rib")) yrFit(); });
