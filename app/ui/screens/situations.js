/* Lane NB8 (Data overhaul items 24-27; boards 47 and 49): Situations in
 * the Data section, with its tabs (down and distance, field zones, the red
 * zone, the two-minute drill, the score and garbage time, 4th downs and
 * 2-point tries against the analysts' model, concepts against coverages),
 * and the game page for any game in the books (`tgame`). Every number
 * follows your seat's charting (the line at each page's foot), and every
 * stat's name explains itself on hover (data-h8). Classes are sx-. */

Object.assign(I, {
  situations: '<path d="M4 18h16M4 18V6M8 14l3-3 3 2 5-6"/><path d="M15 7h4v4"/>',
});
navAdd({ go: "situations", label: "Situations", icon: "situations", sec: "data", after: "analytics" });
Object.assign(CRUMBS, { situations: "Situations", tgame: "Game" });

const SX_TABS = ["Downs and distance", "Field zones", "Red zone", "Two-minute", "Score and garbage time", "4th downs and 2-point", "Concepts against coverages"];
const SX_LEVELS = [["college", "College"], ["nfl", "NFL"], ["hs", "High school"]];
const sxNum = (v, d = 1) => (v == null || Number.isNaN(v) ? "–" : Number(v).toFixed(d));
const sxSign = (v, d = 2) => (v == null ? "–" : (v > 0 ? "+" : v < 0 ? "−" : "") + Math.abs(v).toFixed(d));
const sxPct = (v, d = 0) => (v == null ? "–" : Number(v).toFixed(d) + "%");
const sxH = (key, text) => `<span data-h8="${esc(key)}">${esc(text)}</span>`;
const sxSrc = (line) => (line ? `<p class="dt-src">${esc(line)}</p>` : "");

// A game's link: your kept game's full page, else the books' game page.
function sxGame(link, html) {
  if (!link) return html;
  if (link.seat != null) return `<a class="link" data-go="game" data-index="${link.seat}">${html}</a>`;
  return linkTo("tgame", { year: link.year, key: link.key }, html);
}

screens.situations = async (a) => {
  const K = keep("situ", {});
  for (const k of ["level", "division", "year", "team", "side", "view", "measure"]) if (a[k] != null) { K[k] = a[k]; delete a[k]; }
  if (a.kind && a.id != null) { K.level = a.kind === "hs" ? "hs" : a.kind; K.team = Number(a.id); delete a.kind; delete a.id; }
  const tab = tabOf("sxtab", SX_TABS, a);
  const level = K.level || ({ nfl: "nfl", other: "nfl", hs: "hs" }[seatKind()] || "college");
  const args = { level, division: K.division || undefined, year: K.year ? Number(K.year) : undefined, team: K.team === "" ? "none" : K.team != null ? Number(K.team) : undefined };
  let x, body;
  if (tab === "4th downs and 2-point") {
    x = await api("decisions", args);
    body = x.blocked ? `<div class="empty">${esc(x.blocked)}</div>` : sxDecisions(x);
  } else if (tab === "Concepts against coverages") {
    x = await api("concepts", { ...args, view: K.view || undefined, measure: K.measure || undefined });
    body = x.blocked ? `<div class="empty">${esc(x.blocked)}</div>` : sxConcepts(x);
  } else {
    x = await api("situations", { ...args, side: K.side || "o" });
    body = x.blocked ? `<div class="empty">${esc(x.blocked)}</div>` : sxSituations(x, tab);
  }
  const who = x.team ? `${x.team.name}` : "the league";
  title("Situations", `${who} · ${x.year}`);
  const teamSel = `<select class="ssel" data-act="sxsel" data-k="team"><option value="">${x.level === "hs" ? "Your team" : "The league only"}</option>${(x.pick || []).map(([id, n]) => `<option value="${id}" ${x.team_id === id ? "selected" : ""}>${esc(n)}</option>`).join("")}</select>`;
  const side = tab.startsWith("4th") || tab.startsWith("Concepts") ? "" : `<div class="seg"><button data-act="sxside" data-v="o" aria-pressed="${x.side !== "d"}">With the ball</button><button data-act="sxside" data-v="d" aria-pressed="${x.side === "d"}">On defense</button></div>`;
  const bar = `<div class="tbar">${dtSeg("sxlevel", SX_LEVELS, x.level)}
    ${x.level === "college" ? `<select class="ssel" data-act="sxsel" data-k="division">${x.divisions.map((d) => `<option ${d === x.division ? "selected" : ""}>${esc(d)}</option>`).join("")}</select>` : ""}
    ${(x.years || []).length ? `<select class="ssel" data-act="sxsel" data-k="year">${x.years.map((y) => `<option value="${y}" ${y === x.year ? "selected" : ""}>${y}</option>`).join("")}</select>` : ""}
    ${x.level === "hs" ? "" : teamSel}${side}<span class="tspace"></span>${x.charted_to ? `<span class="muted small">Charted through ${esc(x.charted_to)}</span>` : ""}</div>`;
  const goto = gotoLine([["Player analytics", "analytics"], ["Leaderboards", "leaders"], ["Stat history", "stathist"]]);
  const sub = x.team ? `${x.mine ? "Your team" : esc(x.team.name)} against the ${esc(x.level === "college" ? x.division : x.level === "nfl" ? "NFL" : "level")}'s average team, ${x.year}.` : `Every team in the ${esc(x.level === "college" ? x.division : x.level === "nfl" ? "NFL" : "level")}, ${x.year}.`;
  return `<div class="pad sl dt sx">${pageHead({ kicker: "Data · situations and decisions", title: "Situations", sub, goto })}${bar}${tabs("sxtab", SX_TABS, tab)}<div class="sx-body">${body}</div>${sxSrc(x.source)}</div>`;
};
GOTO.situations = () => [["Player analytics", "analytics"], ["Leaderboards", "leaders"], ["Stat history", "stathist"]];

// ---------- Situations (item 25) ----------
const SX_COLS = [["Plays", "n", (v) => Math.round(v), ""], ["Pass rate", "pass", (v) => sxPct(v), "PassRate"], ["Yards a play", "ypp", (v) => sxNum(v), "Ypp"], ["EPA a play", "epa", (v) => sxSign(v), "TmEpa"], ["Success", "succ", (v) => sxPct(v), "TmSucc"], ["1st down or TD", "conv", (v) => sxPct(v), "ConvRate"]];

function sxTable(rows, x, first = "Situation") {
  const d = x.side === "d";
  const tone = (k, t, l) => {
    if (!t || !l || k === "n" || k === "pass" || l[k] == null) return "";
    const better = t[k] > l[k];
    const close = Math.abs(t[k] - l[k]) < (k === "epa" ? 0.02 : k === "ypp" ? 0.15 : 1.5);
    if (close) return "";
    return better !== d ? "up" : "down";
  };
  const head = `<tr><th class="l">${esc(first)}</th>${SX_COLS.map(([t, , , h]) => `<th ${h ? `data-h8="${h}"` : ""}>${t}</th>`).join("")}</tr>`;
  const body = rows.map((r) => {
    const t = r.team, l = r.level;
    if (!t && !l) return "";
    return `<tr><td class="l">${esc(r.label)}</td>${SX_COLS.map(([, k, f]) => {
      if (!x.team) return `<td>${l ? f(l[k]) : "–"}</td>`;
      return `<td class="sx-cell ${tone(k, t, l)}"><b>${t ? f(t[k]) : "–"}</b><small>${l ? f(l[k]) : ""}</small></td>`;
    }).join("")}</tr>`;
  }).join("");
  return `<div class="tbl sx-tbl"><table><thead>${head}</thead><tbody>${body}</tbody></table></div>`;
}

function sxSituations(x, tab) {
  const key = x.team ? `<p class="muted small">${x.mine ? "Your" : esc(x.team.name) + "'s"} numbers in bold; the level's average team below each, in grey. ${x.side === "d" ? "On defense, lower is better, so green marks numbers held under the level's." : "Green marks better than the level's, orange worse."}</p>` : `<p class="muted small">The level's average team. Pick a team to set its numbers beside.</p>`;
  if (tab === "Downs and distance") return `<section class="panel dt-panel"><h2>Down and distance <small>runs and dropbacks</small></h2>${key}${sxTable([x.all, ...x.downs], x, "Down and yards to go")}</section>`;
  if (tab === "Field zones") return `<section class="panel dt-panel"><h2>Field zones <small>where the snap began</small></h2>${key}${sxTable(x.zones, x, "Spot")}</section>`;
  if (tab === "Red zone") {
    const R = x.red_trips;
    const tile = (k, t, f, h) => `<div><small data-h8="${h}">${t}</small><b>${R.team ? f(R.team[k]) : f(R.level[k])}</b>${R.team ? `<small>Level ${f(R.level[k])}</small>` : ""}</div>`;
    const lvTrips = R.level, tm = R.team;
    const trips = tm ? `${tm.trips}` : `${lvTrips.trips}`;
    return `<div class="st-tiles sx-tiles"><div><small data-h8="RzTrips">Red zone trips</small><b>${trips}</b><small>${tm ? "this season" : "every team's"}</small></div>${tile("td", "Touchdowns a trip", (v) => sxPct(v), "RzTrips")}${tile("fg", "Field goals a trip", (v) => sxPct(v), "RzTrips")}${tile("pts", "Points a trip", (v) => sxNum(v, 2), "RzPts")}${tile("pts40", "Points a trip inside the 40", (v) => sxNum(v, 2), "PtsTrip")}${tile("three_outs", "Three-and-outs", (v) => sxPct(v), "ThreeOuts")}</div>
      <section class="panel dt-panel"><h2>Inside the 20</h2>${key}${sxTable(x.red_zone, x, "Snaps")}</section>`;
  }
  if (tab === "Two-minute") return `<section class="panel dt-panel"><h2>The two-minute drill <small>the last two minutes of each half, against the rest</small></h2>${key}${sxTable(x.two_minute, x, "When")}</section>`;
  return `<section class="panel dt-panel"><h2>The score and <span data-h8="Garbage">garbage time</span></h2>${key}${sxTable(x.score, x, "Score at the snap")}<p class="muted small">${esc(x.garbage_note)}</p></section>`;
}

// ---------- Decisions (item 26, board 47) ----------
function sxDecisions(x) {
  const S = x.sums, L = x.level_sums, M = x.median || {};
  const lvName = x.level === "college" ? x.division : x.level === "nfl" ? "NFL" : "level";
  const tiles = S ? `<div class="st-tiles sx-tiles">
      <div><small data-h8="GoRate">Went for it</small><b>${S.went}</b><small>${S.went_said_go} of ${S.said_go} times the model said go</small></div>
      <div><small data-h8="Agree">Followed the model</small><b>${sxPct(S.agreed)}</b><small>of ${S.weighed} 4th downs · ${esc(lvName)} median ${sxPct(M.agreed)}</small></div>
      <div><small data-h8="WpLost">Win chance left</small><b>${sxNum(S.lost * 100, 0)}</b><small>points, about ${sxNum(S.lost, 1)} wins · median ${sxNum((M.lost || 0) * 100, 0)}</small></div>
      <div><small data-h8="TwoPt">Two-point tries</small><b>${S.twos_made} of ${S.twos}</b><small>the model wanted ${S.said_two}</small></div>
    </div>` : "";
  const word = { go: "Go", punt: "Punt", kick: "FG" };
  const wpc = (v) => (v == null || v < 0 ? "–" : sxNum(v, 0));
  const fourths = (x.fourths || []).slice().reverse().map((f) => `<tr class="${f.agreed === false ? "sx-off" : ""}">
      <td class="l">${sxGame(f.link, `${esc(f.at)} ${f.opp ? esc(f.opp.name) : "?"}`)}<small class="muted"> ${esc(f.date)}</small></td><td class="l">${esc(f.clock)}</td>
      <td>${f.lead > 0 ? "+" + f.lead : f.lead}</td><td class="l">${esc(f.dd)}, ${esc(f.spot)}</td>
      <td class="l"><b>${esc(f.did)}</b></td><td class="l">${f.best ? `<span class="sx-call ${f.best}">${word[f.best]}</span>` : "–"}</td>
      <td class="sx-wp">${wpc(f.wp[0])} · ${wpc(f.wp[1])} · ${wpc(f.wp[2])}</td><td class="${f.lost > 1 ? "down" : ""}">${f.lost == null ? "–" : f.lost < 0.05 ? "0" : "−" + sxNum(f.lost, 1)}</td>
      <td class="l">${esc(f.result)}${f.who ? ` · ${playerLink(f.who.id, f.who.name)}` : ""}</td></tr>`).join("");
  const list = x.team ? `<section class="panel dt-panel"><h2>${x.mine ? "Your" : esc(x.team.name) + "'s"} 4th downs <small>every one this season, newest first, against the model</small></h2>${fourths ? `<div class="tbl"><table><thead><tr><th class="l">Game</th><th class="l">Clock</th><th>Score</th><th class="l">Down and spot</th><th class="l">${x.mine ? "You" : "They"}</th><th class="l">Model</th><th data-h8="WinChance">Win chance go · punt · kick</th><th data-h8="WpLost">Cost</th><th class="l">What happened</th></tr></thead><tbody>${fourths}</tbody></table></div>` : '<div class="empty">No 4th downs charted yet.</div>'}</section>` : "";
  // The model's grid, with the level's (and the team's) go rate in each cell.
  const grid = `<section class="panel dt-panel"><h2>The model's call on 4th down <small>${esc(lvName)}, a tied game, 10:00 in the 2nd quarter · the small number is how often the ${esc(lvName)} went for it there${x.team ? `, then ${x.mine ? "you" : "this team"}` : ""}</small></h2>
    <div class="sx-grid" style="--cols:${x.zones.length}"><span></span>${x.zones.map((z) => `<span class="sx-gz">${esc(z)}</span>`).join("")}
    ${x.rows.map((r, i) => `<span class="sx-gr">${esc(r)}</span>${x.model[i].map((c, j) => {
      const lc = x.level_grid[i][j], tc = x.team_grid ? x.team_grid[i][j] : null;
      const rate = (g) => (g && g.n ? `${Math.round((100 * g.went) / g.n)}%<i>${g.n}</i>` : "–");
      return `<span class="sx-gc ${c.call}" title="Go ${sxNum(c.go, 1)}% · punt ${c.punt == null ? "–" : sxNum(c.punt, 1)}% · kick ${c.kick == null ? "–" : sxNum(c.kick, 1)}% · converts ${sxNum(c.conv, 0)}%"><b>${c.call === "kick" ? "FG" : c.call.toUpperCase()}</b><small>${c.by >= 99 ? "only choice" : "+" + sxNum(c.by, 1)}</small><em>${rate(lc)}${tc ? ` · ${rate(tc)}` : ""}</em></span>`;
    }).join("")}`).join("")}</div></section>`;
  const coaches = (x.coaches || []).map((c) => {
    const s = c.sums;
    return `<tr class="${c.mine ? "sx-mine" : ""}"><td class="l">${c.coach ? coachLink(c.coach.id, c.coach.name) : "–"}${c.mine ? ' <span class="tag soft">You</span>' : ""}</td><td class="l">${teamChip(c.team, 16)}</td><td>${s.fourths}</td><td>${s.went}</td><td>${s.said_go ? `${s.went_said_go}/${s.said_go}` : "–"}</td><td>${sxPct(s.agreed)}</td><td>${sxNum(s.lost_game, 1)}</td><td>${s.twos ? `${s.twos_made}/${s.twos}` : "0"}</td><td>${s.challenges ? `${s.won}/${s.challenges}` : "0"}</td></tr>`;
  }).join("");
  const med = `<tr class="sx-med"><td class="l">Median</td><td></td><td></td><td></td><td>${sxPct(M.go_rate)}</td><td>${sxPct(M.agreed)}</td><td>${sxNum(M.lost_game, 1)}</td><td></td><td></td></tr>`;
  const coachT = coaches ? `<section class="panel dt-panel"><h2>Head coaches on 4th down this season <small>most often with the model first</small></h2><div class="tbl"><table><thead><tr><th class="l">Coach</th><th class="l">Team</th><th>4th downs</th><th>Went</th><th data-h8="GoRate">Went when "go"</th><th data-h8="Agree">Agrees</th><th data-h8="WpLost">Win chance lost a game</th><th data-h8="TwoPt">Two-point</th><th data-h8="ChalWon">Challenges won</th></tr></thead><tbody>${med}${coaches}</tbody></table></div></section>` : "";
  const tries = (x.tries || []).slice().reverse().map((t) => `<tr class="${t.agreed === false ? "sx-off" : ""}"><td class="l">${sxGame(t.link, `${esc(t.at)} ${t.opp ? esc(t.opp.name) : "?"}`)}<small class="muted"> ${esc(t.date)}</small></td><td class="l">${esc(t.clock)}</td><td>${t.lead > 0 ? "+" + t.lead : t.lead}</td><td class="l"><b>${esc(t.did)}</b> ${t.good ? "good" : "no good"}</td><td>${wpc(t.wp[0])} · ${wpc(t.wp[1])}</td><td class="l">${t.best === "two" ? "Go for two" : t.best ? "Kick" : "–"}</td><td>${t.lost == null ? "–" : t.lost < 0.05 ? "0" : "−" + sxNum(t.lost, 1)}</td></tr>`).join("");
  const triesT = x.team && tries ? `<section class="panel dt-panel"><h2>Tries after touchdowns <small>kick or go for two, against the model</small></h2><div class="tbl"><table><thead><tr><th class="l">Game</th><th class="l">Clock</th><th>Score</th><th class="l">Did</th><th data-h8="WinChance">Win chance kick · two</th><th class="l">Model</th><th data-h8="WpLost">Cost</th></tr></thead><tbody>${tries}</tbody></table></div></section>` : "";
  const clock = S ? `<section class="panel dt-panel"><h2>Timeouts and challenges</h2><div class="st-tiles sx-tiles small">
      <div><small>Timeouts used</small><b>${S.timeouts}</b><small>${sxNum(S.timeouts / Math.max(1, S.games), 1)} a game · median ${sxNum(M.timeouts, 1)} · ${S.to_late} in the last 2 minutes of a half</small></div>
      <div><small data-h8="TimeoutsLeft">Left in close losses</small><b>${S.to_left}</b><small>in ${S.close_losses} losses by 8 or less</small></div>
      <div><small data-h8="ChalWon">Challenges won</small><b>${S.won} of ${S.challenges}</b><small>median thrown ${sxNum(M.challenges, 0)}, won ${sxPct(M.won)}</small></div>
      <div><small data-h8="MissedChal">Wrong calls not challenged</small><b>${S.missed}</b><small>rulings against ${x.mine ? "you" : "them"} the replay would have overturned</small></div>
    </div>${(x.challenges || []).length ? `<div class="tbl"><table><thead><tr><th class="l">Game</th><th class="l">Clock</th><th class="l">The ruling</th><th class="l">Result</th></tr></thead><tbody>${x.challenges.map((c) => `<tr><td class="l">${sxGame(c.link, c.opp ? esc(c.opp.name) : "?")}<small class="muted"> ${esc(c.date)}</small></td><td class="l">${esc(c.clock)}</td><td class="l">${esc(c.call)}</td><td class="l ${c.won ? "up" : "down"}">${c.won ? "Overturned" : "Stood"}</td></tr>`).join("")}</tbody></table></div>` : ""}</section>` : "";
  const lvLine = `<p class="muted small">${esc(lvName)}: ${L.fourths} 4th downs, went for it on ${L.went} (${sxPct(L.went ? (100 * L.went) / L.fourths : 0)}), converted ${sxPct(L.conv_rate)}; went on ${sxPct(L.go_rate)} of the times the model said go. The model calls a choice best when it is worth ${sxNum(x.clear, 0)} point of win chance more than the next; within that, either choice agrees.</p>`;
  return `${tiles}${list}<div class="sx-two">${grid}${coachT}</div>${lvLine}${triesT}${clock}`;
}

// ---------- Concepts against coverages (item 27, board 49) ----------
const SX_MEASURES = [["success", "Success rate"], ["epa", "EPA a play"], ["ypp", "Yards a play"]];
function sxConcepts(x) {
  const f = x.measure === "epa" ? (v) => sxSign(v) : x.measure === "ypp" ? (v) => sxNum(v) : (v) => sxPct(v);
  const views = [["league", "The league"], ...(x.team ? [["offense", x.mine ? "Your offense" : "Their offense"], ["defense", x.mine ? "Your defense" : "Their defense"]] : [])];
  const bar = `<div class="tbar">${dtSeg("sxmeasure", SX_MEASURES, x.measure)}${dtSeg("sxview", views, x.view)}</div>`;
  const all = x.col_all.map((c) => c.v);
  const mean = all.length ? x.grid.reduce((a, r) => a + r.all.v * r.all.n, 0) / Math.max(1, x.grid.reduce((a, r) => a + r.all.n, 0)) : 0;
  const spread = x.measure === "epa" ? 0.3 : x.measure === "ypp" ? 3 : 15;
  const shade = (v, n) => {
    if (v == null || n < 20) return "";
    const d = Math.max(-1, Math.min(1, (v - mean) / spread));
    const flip = x.view === "defense" ? -1 : 1;
    const z = d * flip;
    return z >= 0 ? `--g:${Math.round(z * 55)}%` : `--b:${Math.round(-z * 55)}%`;
  };
  const cell = (c) => (c ? `<td class="sx-cc ${c.n < 20 ? "thin" : ""}" style="${shade(c.v, c.n)}"><b>${f(c.v)}</b><small>${Math.round(c.n)}</small></td>` : `<td class="sx-cc none">–</td>`);
  let lastPass = null;
  const rows = x.grid.map((r) => {
    const sep = lastPass !== r.pass ? `<tr class="grouprow"><td class="l" colspan="${x.coverages.length + 2}">${r.pass ? "Pass concepts" : "Run concepts"}</td></tr>` : "";
    lastPass = r.pass;
    return `${sep}<tr><td class="l">${esc(r.concept)}</td>${r.cells.map(cell).join("")}${cell(r.all)}</tr>`;
  }).join("");
  const seen = `<tr class="sx-seen"><td class="l" data-h8="CovSeen">Seen</td>${x.seen.map((s) => `<td>${sxPct(s)}</td>`).join("")}<td>100%</td></tr>`;
  const grid = `<section class="panel dt-panel"><h2><span data-h8="Concept">Concepts</span> against <span data-h8="Coverage">coverages</span> <small>${x.view === "league" ? "every team's offense" : x.view === "offense" ? "the plays it ran against what it saw" : "the plays run at it against the coverage it played"} · ${Math.round(x.snaps)} snaps · the small number is snaps, faint under 20</small></h2>
    <div class="tbl sx-cctbl"><table><thead><tr><th class="l">Concept</th>${x.coverages.map((c) => `<th>${esc(c)}</th>`).join("")}<th>All</th></tr></thead><tbody>${seen}${rows}</tbody></table></div></section>`;
  const T = x.tiles || {};
  const tiles = x.team ? `<div class="st-tiles sx-tiles">
      <div><small>Most seen against ${x.mine ? "you" : "them"}</small><b class="sx-tw">${T.most_seen ? esc(T.most_seen.coverage) : "–"}</b><small>${T.most_seen ? sxPct(T.most_seen.share) + " of snaps" : ""}</small></div>
      <div><small>${x.mine ? "Your" : "Their"} best answer to it</small><b class="sx-tw">${T.best ? esc(T.best.concept) : "–"}</b><small>${T.best ? `${f(T.best.v)} on ${Math.round(T.best.n)} snaps` : ""}</small></div>
      <div><small>Costing ${x.mine ? "you" : "them"} most</small><b class="sx-tw">${T.costly ? esc(T.costly.concept) : "–"}</b><small>${T.costly ? `vs ${esc(T.costly.coverage)}: ${sxSign(T.costly.epa, 1)} EPA on ${Math.round(T.costly.n)} snaps` : ""}</small></div>
      <div><small>${esc(x.level === "college" ? x.division : x.level === "nfl" ? "The NFL" : "The level")} is copying</small><b class="sx-tw">${x.copying ? esc(x.copying.concept) : "–"}</b><small>${x.copying ? `up ${sxNum(x.copying.up, 1)} points of snaps on last season` : ""}</small></div>
    </div>` : x.copying ? `<p class="muted small">${esc(x.level === "nfl" ? "The NFL" : "The level")} is copying <b>${esc(x.copying.concept)}</b>: up ${sxNum(x.copying.up, 1)} points of snaps on last season.</p>` : "";
  const next = x.next ? `<section class="panel dt-panel"><h2>Next: ${teamA(x.next.team)} <small>${esc(x.next.date)} · the coverages its defense plays, and the level's best answer to each</small></h2><div class="tbl"><table><thead><tr><th class="l" data-h8="Coverage">Coverage</th><th>All downs</th><th>3rd down</th><th class="l">Best answer</th></tr></thead><tbody>${x.next.rows.map((r) => `<tr><td class="l">${esc(r.coverage)}</td><td>${sxPct(r.all)}</td><td>${sxPct(r.third)}</td><td class="l">${r.best ? `${esc(r.best.concept)} <small class="muted">${f(r.best.v)}</small>` : "–"}</td></tr>`).join("")}</tbody></table></div></section>` : "";
  const yours = (x.yours || []).length ? `<section class="panel dt-panel"><h2>${x.mine ? "Your" : "Their"} calls against what ${x.mine ? "you" : "they"} saw <small>the concept called most against each coverage, against the level's numbers</small></h2><div class="tbl"><table><thead><tr><th class="l">Coverage</th><th>Snaps</th><th class="l">Called most</th><th>${x.mine ? "Yours" : "Theirs"}</th><th>Level's</th><th class="l">Best answer</th></tr></thead><tbody>${x.yours.map((r) => `<tr><td class="l">${esc(r.coverage)}</td><td>${Math.round(r.seen)}</td><td class="l">${esc(r.called)} <small class="muted">×${Math.round(r.called_n)}</small></td><td>${f(r.yours)}</td><td>${r.level == null ? "–" : f(r.level)}</td><td class="l">${r.best ? `${esc(r.best.concept)} <small class="muted">${f(r.best.v)}</small>` : "–"}</td></tr>`).join("")}</tbody></table></div></section>` : "";
  return `${bar}${tiles}${grid}<div class="sx-two">${next}${yours}</div>`;
}

Object.assign(CORE_MORE, {
  sxlevel: (el) => { const K = keep("situ", {}); K.level = el.dataset.v; K.team = null; K.year = null; K.division = null; refresh(el); },
  sxside: (el) => { keep("situ", {}).side = el.dataset.v; refresh(el); },
  sxview: (el) => { keep("situ", {}).view = el.dataset.v; refresh(el); },
  sxmeasure: (el) => { keep("situ", {}).measure = el.dataset.v; refresh(el); },
});
document.addEventListener("change", (ev) => {
  const t = ev.target;
  if (!t.dataset || t.dataset.act !== "sxsel") return;
  const K = keep("situ", {});
  K[t.dataset.k] = t.value === "" ? (t.dataset.k === "team" ? "" : null) : t.value;
  if (t.dataset.k === "division") { K.team = null; }
  refresh(t);
});

// ---------- The five factors on a game page (item 24) ----------
// Away first, as the box score reads.
function nb8Five(F, pg, awayName, homeName) {
  if (!F) return "";
  const r = (t, h, f) => `<tr><td class="l" data-h8="${h}">${t}</td><td>${f(F[1], F[0])}</td><td>${f(F[0], F[1])}</td></tr>`;
  const pct = (k) => (a) => sxPct(a[k]);
  const pgw = pg ? `<tr class="sx-pg"><td class="l" data-h8="Pgwe">Post-game win expectancy</td><td><b>${sxPct(pg[1])}</b></td><td><b>${sxPct(pg[0])}</b></td></tr>` : "";
  return `<div class="panel warm"><h2>The five factors <small>success, explosiveness, field position, finishing drives and turnovers, and what the box says the result should have been</small></h2><div class="tbl"><table><thead><tr><th class="l"></th><th>${esc(awayName)}</th><th>${esc(homeName)}</th></tr></thead><tbody>
    ${r("Success rate (EPA above zero)", "TmSucc", pct("success"))}${r("Explosiveness", "Explosiveness", (a) => sxNum(a.explosiveness, 2))}${r("Explosive plays", "ExplosivePlays", (a) => `${a.explosive} <small class="muted">${sxPct(a.explosive_rate)}</small>`)}
    ${r("Field position", "FieldPos", (a) => `own ${Math.round(a.start)}`)}${r("Points a trip inside the 40", "PtsTrip", (a) => `${sxNum(a.pts40, 2)} <small class="muted">${a.trips40} trips</small>`)}${r("Red zone TDs / trips", "RzTrips", (a) => a.rz)}
    ${r("Turnovers", "Turnovers", (a) => a.turnovers)}${r("Havoc rate", "Havoc", pct("havoc"))}${r("Line yards a carry", "LineYds", (a) => sxNum(a.line_yards, 2))}${r("Stuff rate", "Stuffed", pct("stuffed"))}${r("Three-and-outs", "ThreeOuts", (a) => a.three_outs)}
    ${pgw}</tbody></table></div></div>`;
}

// ---------- Any game's page from the books (item 24) ----------
function sxWpChart(wp, mineHome = true) {
  if (!wp || wp.length < 2) return "";
  const W = 600, H = 120, n = wp.length;
  const pts = wp.map((v, i) => `${((i / (n - 1)) * W).toFixed(1)},${(H - ((mineHome ? v : 100 - v) / 100) * H).toFixed(1)}`).join(" ");
  return `<svg class="sx-wpc" viewBox="0 0 ${W} ${H}" preserveAspectRatio="none" role="img" aria-label="Win chance through the game"><line x1="0" y1="${H / 2}" x2="${W}" y2="${H / 2}" class="mid"/><polyline points="${pts}" class="ln"/></svg>`;
}

screens.tgame = async (a) => {
  const g = await api("tally_game", { year: a.year, key: a.key });
  if (g.error) return `<div class="pad"><div class="empty">${esc(g.error)}</div></div>`;
  const [h, w] = [g.box[0], g.box[1]];
  const won = (k) => g.score[k] > g.score[1 - k];
  title("Game", `${g.away ? g.away.name : "?"} at ${g.home ? g.home.name : "?"}`);
  const qs = (g.quarters || []).map((q, i) => `<th>${i < 4 ? "Q" + (i + 1) : "OT"}</th>`).join("");
  const ql = (k) => (g.quarters || []).map((q) => `<td>${q[k]}</td>`).join("");
  const kindWord = g.kind && g.kind !== "regular" ? ` · ${esc(g.kind)}` : "";
  const hero = `<div class="sx-hero"><div class="st-label">${esc(g.date)} · ${esc(g.group)}${kindWord}${g.neutral ? " · neutral site" : ""}${g.overtime ? " · overtime" : ""}</div>
    <div class="sx-score"><span class="${won(1) ? "won" : ""}">${g.away ? teamChip(g.away, 36) : "?"}<b>${g.score[1]}</b></span><span class="muted">at</span><span class="${won(0) ? "won" : ""}"><b>${g.score[0]}</b>${g.home ? teamChip(g.home, 36) : "?"}</span></div>
    ${qs ? `<div class="tbl sx-qs"><table><thead><tr><th class="l"></th>${qs}<th>T</th></tr></thead><tbody><tr><td class="l">${g.away ? esc(g.away.name) : ""}</td>${ql(1)}<td><b>${g.score[1]}</b></td></tr><tr><td class="l">${g.home ? esc(g.home.name) : ""}</td>${ql(0)}<td><b>${g.score[0]}</b></td></tr></tbody></table></div>` : ""}
    ${g.seat != null ? `<p><a class="link" data-go="game" data-index="${g.seat}">Your game's full page: every snap, the drive chart and the film ›</a></p>` : ""}</div>`;
  const stat = (t, f, h8) => `<tr><td class="l" ${h8 ? `data-h8="${h8}"` : ""}>${t}</td><td>${f(w)}</td><td>${f(h)}</td></tr>`;
  const teamT = `<section class="panel dt-panel"><h2>Team stats</h2><div class="tbl"><table><thead><tr><th class="l"></th><th>${g.away ? esc(g.away.name) : ""}</th><th>${g.home ? esc(g.home.name) : ""}</th></tr></thead><tbody>
    ${stat("Total yards", (t) => t.yards)}${stat("Passing", (t) => t.pass)}${stat("Completions", (t) => t.cmp)}${stat("Rushing", (t) => `${t.rush} <small class="muted">${t.carries} car</small>`)}${stat("First downs", (t) => t.first_downs)}${stat("Third downs", (t) => t.third, "ThirdDowns")}${stat("Fourth downs", (t) => t.fourth, "FourthDowns")}${stat("Sacks by", (t) => t.sacks)}${stat("Turnovers", (t) => t.turnovers, "Turnovers")}${stat("Penalties", (t) => t.penalties)}${stat("Possession", (t) => t.top)}${stat("Plays", (t) => t.plays)}
    </tbody></table></div></section>`;
  const pl = (k, name) => {
    const P = g.players[k];
    const t = (head, cols, list) => (list && list.length ? tbl([head, ...cols], list.map((r) => [r.who.id != null ? playerLink(r.who.id, r.who.name) : esc(r.who.name), ...r.cells])) : "");
    const body = t("Passing", ["Cmp", "Att", "Yds", "TD", "Int", "Sk"], P.passing) + t("Rushing", ["Car", "Yds", "TD"], P.rushing) + t("Receiving", ["Tgt", "Rec", "Yds", "TD"], P.receiving) + t("Defense", ["Tkl", "TFL", "Sk", "Int", "PBU"], P.defense) + t("Kicking", ["FG", "FGA", "XP", "Long"], P.kicking);
    return `<section class="panel dt-panel"><h2>${esc(name)}</h2>${body || '<div class="empty">No player lines kept at this level.</div>'}</section>`;
  };
  let adv = "";
  if (g.charted) {
    const chart = g.wp && g.wp.length > 1 ? `<section class="panel dt-panel"><h2 data-h8="WinChance">${g.home ? esc(g.home.name) : "Home"} win chance <small>before every snap</small></h2>${sxWpChart(g.wp)}</section>` : "";
    const word = ["go", "punt", "kick"];
    const fr = (g.fourths || []).map((f) => {
      const side = f.side === 0 ? g.home : g.away;
      const did = ["Went", "Punted", "Kicked"][f.did];
      const best = f.best == null ? "–" : `<span class="sx-call ${word[f.best]}">${word[f.best] === "kick" ? "FG" : word[f.best][0].toUpperCase() + word[f.best].slice(1)}</span>`;
      const yl = f.yl, own = 100 - yl;
      const spot = own < 50 ? `own ${own}` : own > 50 ? `opp ${yl}` : "50";
      const wp = (v) => (v == null || v < 0 ? "–" : sxNum(v, 0));
      return `<tr><td class="l">${esc(f.clock)}</td><td class="l">${side ? esc(side.name) : ""}</td><td class="l">4th & ${f.togo >= yl && yl <= 10 ? "goal" : Math.max(1, Math.ceil(f.togo))}, ${spot}</td><td class="l"><b>${did}</b></td><td class="l">${best}</td><td>${wp(f.wp[0])} · ${wp(f.wp[1])} · ${wp(f.wp[2])}</td><td class="${f.lost > 1 ? "down" : ""}">${f.lost == null ? "–" : f.lost < 0.05 ? "0" : "−" + sxNum(f.lost, 1)}</td></tr>`;
    }).join("");
    const tr = (g.tries || []).map((t) => {
      const side = t.side === 0 ? g.home : g.away;
      return `<tr><td class="l">${esc(t.clock)}</td><td class="l">${side ? esc(side.name) : ""}</td><td class="l"><b>${t.two ? "Went for two" : "Kicked"}</b> ${t.good ? "good" : "no good"}</td><td>${t.wp[0] < 0 ? "–" : sxNum(t.wp[0], 0)} · ${t.wp[1] < 0 ? "–" : sxNum(t.wp[1], 0)}</td><td class="l">${t.best_two == null ? "–" : t.best_two ? "Go for two" : "Kick"}</td><td>${t.lost == null ? "–" : t.lost < 0.05 ? "0" : "−" + sxNum(t.lost, 1)}</td></tr>`;
    }).join("");
    adv = `<div class="sx-two">${nb8Five(g.advanced, g.pgwe, g.away ? g.away.name : "Away", g.home ? g.home.name : "Home")}${chart}</div>
      <section class="panel dt-panel"><h2>Every 4th down against the model <small>win chance going, punting and kicking at the snap</small></h2>${fr ? `<div class="tbl"><table><thead><tr><th class="l">When</th><th class="l">Team</th><th class="l">Down and spot</th><th class="l">Did</th><th class="l">Model</th><th data-h8="WinChance">Go · punt · kick</th><th data-h8="WpLost">Cost</th></tr></thead><tbody>${fr}</tbody></table></div>` : '<div class="empty">No 4th-down decisions in this game.</div>'}</section>
      ${tr ? `<section class="panel dt-panel"><h2>Tries after touchdowns</h2><div class="tbl"><table><thead><tr><th class="l">When</th><th class="l">Team</th><th class="l">Did</th><th data-h8="WinChance">Kick · two</th><th class="l">Model</th><th data-h8="WpLost">Cost</th></tr></thead><tbody>${tr}</tbody></table></div></section>` : ""}`;
  }
  const goto = gotoLine([["Situations", "situations"], ["Leaderboards", "leaders"], ["Stat history", "stathist"]]);
  return `<div class="pad sl dt sx">${goto}${hero}<div class="sx-two">${teamT}<div>${pl(1, g.away ? g.away.name : "Away")}${pl(0, g.home ? g.home.name : "Home")}</div></div>${adv}${sxSrc(g.source)}</div>`;
};
GOTO.tgame = () => [["Situations", "situations"], ["Leaderboards", "leaders"], ["Stat history", "stathist"]];
