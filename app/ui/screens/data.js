/* The Data section (update 3, lane L20; boards 43 and 46) in Press Box: its
 * own place in the sidebar between Your team and World, at every seat, with
 * Stat history, Player analytics, Leaderboards and History in it. Stat
 * history keeps every level's careers, seasons, single games, rate records
 * and each year's leader, with the era column, the real NFL record on top,
 * who is chasing a record now, what fell this season and the team's own
 * book. Analytics ranks a position group by percentile and opens a card
 * with the seasons most like his. Every number follows your seat's real
 * sources (the line at each page's foot). Every class is dt- so the app's
 * global names never restyle it; the page carries .sl for lane 1's tokens. */

Object.assign(I, {
  stathist: '<path d="M4 20V5M4 20h16"/><path d="M8 16l3-4 3 2 5-7"/><circle cx="19" cy="7" r="1"/>',
  analytics: '<circle cx="12" cy="12" r="8"/><path d="M12 4v8l6 4"/>',
  glossary: '<path d="M5 4h11a3 3 0 0 1 3 3v13H8a3 3 0 0 1-3-3z"/><path d="M5 17a3 3 0 0 1 3-3h11M9 8h6"/>',
});
if (!NAV_SECS.some(([k]) => k === "data")) NAV_SECS.splice(NAV_SECS.findIndex(([k]) => k === "world"), 0, ["data", "Data"]);
navAdd({ go: "stathist", label: "Stat history", icon: "stathist", sec: "data" });
navAdd({ go: "analytics", label: "Player analytics", icon: "analytics", sec: "data", after: "stathist" });
// Leaderboards and History move in from World (item 2).
{
  for (const go of ["leaders", "history"]) {
    const i = NAV.findIndex((n) => n.go === go);
    if (i >= 0) NAV.splice(i, 1);
  }
  const at = NAV.findIndex((n) => n.go === "analytics");
  NAV.splice(at + 1, 0, { go: "leaders", label: "Leaderboards", icon: "leader", sec: "data" }, { go: "history", label: "History", icon: "history", sec: "data" });
}
navAdd({ go: "glossary", label: "Glossary", icon: "glossary", sec: "data", after: "history" });
Object.assign(CRUMBS, { stathist: "Stat history", analytics: "Player analytics", glossary: "Glossary" });

const DT_LEVELS = [["college", "College"], ["nfl", "NFL"], ["hs", "High school"]];
const dtSeg = (act, list, cur) => `<div class="seg">${list.map(([v, t]) => `<button data-act="${act}" data-v="${esc(v)}" aria-pressed="${String(cur) === String(v)}">${esc(t)}</button>`).join("")}</div>`;
const dtSel = (k, list, cur, all) => `<select class="ssel" data-act="dsel" data-k="${k}">${all != null ? `<option value="">${esc(all)}</option>` : ""}${list.map((v) => { const [val, t] = Array.isArray(v) ? v : [v, v]; return `<option value="${esc(val)}" ${String(cur ?? "") === String(val) ? "selected" : ""}>${esc(t)}</option>`; }).join("")}</select>`;
const dtWho = (r) => (r.linked ? playerLink(r.id, r.name) : esc(r.name));
const dtEra = (e) => (e == null ? "" : `<span class="dt-era ${e >= 130 ? "hi" : e < 90 ? "lo" : ""}" title="Era: 100 is that season's average for a regular at his position">${e}</span>`);
const dtSource = (line) => (line ? `<p class="dt-src">${esc(line)}</p>` : "");
// The level the page shows: the one picked, else your seat's.
const dtLevel = (K) => K.level || ({ nfl: "nfl", other: "nfl", hs: "hs" }[seatKind()] || "college");

// ---------- Stat history (items 15-22, board 43) ----------
const DT_TABS = [["Careers", "career"], ["Single seasons", "season"], ["Single games", "game"], ["Rates", "rates"], ["Each year's leader", "leaders"], ["Team book", "team"]];

screens.stathist = async (a) => {
  const K = keep("stathist", {});
  for (const k of ["level", "division", "conference", "state"]) if (a[k] != null) { K[k] = a[k]; delete a[k]; }
  if (a.kind != null && a.id != null) { K.team = { kind: a.kind, id: Number(a.id) }; K.level = a.kind === "hs" ? "hs" : a.kind; delete a.kind; delete a.id; a.tab = "Team book"; }
  if (a.tab) { const m = DT_TABS.find((t) => t[1] === a.tab); if (m) a.tab = m[0]; }
  const tab = tabOf("dttab", DT_TABS.map((t) => t[0]), a);
  const key = DT_TABS.find((t) => t[0] === tab)[1];
  const level = dtLevel(K);
  // A return touchdown pick from another level has no high school book.
  if (level === "hs" && Number(K.stat) === 12) K.stat = null;
  const x = await api("stat_history", {
    level, tab: key, division: K.division || undefined, conference: K.conference || undefined, state: K.state || undefined,
    stat: K.stat != null ? Number(K.stat) : undefined, team: key === "team" && K.team && K.team.kind === level ? K.team : undefined,
  });
  title("Stat history", `${x.scope}`);
  const bar = `<div class="tbar">${dtSeg("dtlevel", DT_LEVELS, x.level)}
    ${x.level === "college" ? dtSel("division", x.divisions, x.division) + dtSel("conference", x.conferences, x.conference, "Whole division") : ""}
    ${x.level === "hs" ? dtSel("state", x.states.map((c) => [c, STATE_NAMES[c] || c]), x.state) : ""}
    <span class="tspace"></span>${x.since ? `<span class="muted small">Books since ${x.since}</span>` : ""}</div>`;
  let main = "";
  if (key === "leaders") main = dtLeaders(x);
  else if (key === "team") main = x.team_book ? dtTeamBook(x.team_book, x.stats, K, x.level, x.stat_keys) :`<div class="empty">Pick a team from its team page, or take a seat at this level, to see a team's own book.</div>`;
  else main = `<div class="dt-books">${x.lists.map((l) => dtList(l, K)).join("") || '<div class="empty">Nothing in these books yet.</div>'}</div>`;
  const side = `<aside class="dt-side">${dtChasing(x.chasing)}${dtFell(x.fell)}${x.own ? dtOwn(x.own) : ""}</aside>`;
  const goto = gotoLine([["Leaderboards", "leaders"], ["Player analytics", "analytics"], ["History", "history"]]);
  return `<div class="pad sl dt">${pageHead({ kicker: `Data · ${esc(x.scope)}`, title: "Stat history", sub: esc(x.note), goto })}${bar}${tabs("dttab", DT_TABS.map((t) => t[0]), tab)}<div class="dt-main"><div class="dt-body">${main}</div>${side}</div>${dtSource(x.source)}</div>`;
};
GOTO.stathist = () => [["Leaderboards", "leaders"], ["Player analytics", "analytics"], ["History", "history"]];

function dtGame(g) {
  if (!g) return "";
  return `<small class="dt-game">${g.home ? "vs" : "at"} ${g.opp ? teamA(g.opp) : "?"} · <b class="${g.won ? "w" : "l"}">${g.won ? "W" : "L"} ${esc(g.score)}</b> · ${esc(g.date)}</small>`;
}

function dtRow(r) {
  const bits = [r.team ? teamA(r.team) : "", r.pos ? esc(r.pos) : "", esc(r.year), r.games ? `${r.games} g` : "", r.live ? '<span class="tag soft">active</span>' : ""].filter(Boolean).join(" · ");
  return `<li class="${r.live ? "live" : ""}"><span class="dt-rk">${r.rank}</span><b class="dt-v">${esc(r.value)}</b><span class="dt-who">${dtWho(r)}<small>${bits}</small>${dtGame(r.game)}</span>${dtEra(r.era)}</li>`;
}

function dtList(l, K) {
  const real = l.real ? `<li class="dt-real"><span class="dt-rk">★</span><b class="dt-v">${esc(l.real.value)}</b><span class="dt-who">${esc(l.real.holder)}<small>${esc(l.real.year)} · <span class="tag gold">Real NFL record</span></small></span></li>` : "";
  const more = l.stat != null && l.kind !== "streak" && l.kind.indexOf("rate") < 0 ? `<button class="dt-more" data-act="dtstat" data-v="${String(K.stat) === String(l.stat) ? "" : l.stat}">${String(K.stat) === String(l.stat) ? "Top 10" : "Top 25"}</button>` : "";
  return `<section class="dt-book panel"><header><h2${l.h8 ? ` data-h8="${esc(l.h8)}"` : ""}>${esc(l.name)}</h2>${more}</header>${l.min ? `<p class="muted small">${esc(l.min)}</p>` : ""}<ol>${real}${l.rows.map(dtRow).join("") || '<li class="muted">Nobody yet.</li>'}</ol></section>`;
}

function dtLeaders(x) {
  const L = x.leaders;
  const chips = `<div class="seg wrap">${x.stats.map((n, i) => (x.level === "hs" && i === 12 ? "" : `<button data-act="dtstat" data-v="${i}" aria-pressed="${L.stat === i}" data-h8="${esc((x.stat_keys || [])[i] || "")}">${esc(n)}</button>`)).join("")}</div>`;
  const cell = (e) => (e ? `<div class="dt-lead"><b class="dt-v">${esc(e.value)}</b> ${dtWho(e)} <small>${e.team ? teamA(e.team) : ""}${e.pos ? " · " + esc(e.pos) : ""}</small>${dtEra(e.era)}</div>` : "");
  const rows = L.rows.map((r) => `<tr><td>${r.year}</td><td class="l">${cell(r.first)}</td><td class="l">${cell(r.second)}</td></tr>`).join("");
  return `${chips}<div class="tbl"><table><thead><tr><th>Season</th><th class="l"${L.h8 ? ` data-h8="${esc(L.h8)}"` : ""}>${esc(L.name)}: the leader</th><th class="l">Runner-up</th></tr></thead><tbody>${rows || '<tr><td colspan="3" class="muted">No seasons in the books yet.</td></tr>'}</tbody></table></div>`;
}

function dtTeamBook(b, stats, K, level, keys = []) {
  const groups = [["career", "Career"], ["season", "Season"], ["game", "Single game"]];
  // High school books keep no return touchdowns (stat 12), as on Each year's leader.
  const chips = `<div class="seg wrap"><button data-act="dtstat" data-v="" aria-pressed="${K.stat == null}">Every stat</button>${stats.map((n, i) => (level === "hs" && i === 12 ? "" : `<button data-act="dtstat" data-v="${i}" aria-pressed="${String(K.stat) === String(i)}" data-h8="${esc(keys[i] || "")}">${esc(n)}</button>`)).join("")}</div>`;
  const body = groups.map(([k, n]) => {
    const lists = b.lists.filter((l) => l.kind === k && l.rows.length);
    if (!lists.length) return "";
    return `<h3 class="dt-h3">${n}</h3><div class="dt-books">${lists.map((l) => `<section class="dt-book panel"><header><h2${l.h8 ? ` data-h8="${esc(l.h8)}"` : ""}>${esc(l.name.charAt(0).toUpperCase() + l.name.slice(1))}</h2></header><ol>${l.rows.map(dtRow).join("")}</ol></section>`).join("")}</div>`;
  }).join("");
  return `<div class="dt-teamhead">${mark(b.team, 40)}<div><div class="st-label">Team book · the best ${b.top} in each${b.since ? ` · since ${b.since}` : ""}</div><b>${teamA(b.team)}</b></div></div>${chips}${body || '<div class="empty">Nothing in this team\'s book yet.</div>'}`;
}

function dtChasing(list) {
  const rows = (list || []).map((c) => `<li><span class="dt-who">${dtWho(c)}<small>${c.team ? teamA(c.team) + " · " : ""}${esc(c.book)}</small></span><span class="dt-ch"><b>${esc(c.have)}</b>${c.pace ? ` <small>on pace for ${esc(c.pace)}</small>` : ` <small>needs ${esc(c.needs)}</small>`}<i class="dt-bar"><i style="width:${Math.min(100, c.pct)}%"></i></i><small>Record ${esc(c.record)}, ${esc(c.holder)} (${c.holder_year})</small></span></li>`).join("");
  return `<section class="panel dt-panel"><h2>Chasing now</h2>${rows ? `<ol class="dt-plain">${rows}</ol>` : '<p class="muted small">Nobody is close to a record right now.</p>'}</section>`;
}

function dtFell(list) {
  const rows = (list || []).map((f) => `<li><span class="when">${esc(f.date)}</span><span>${dtWho(f)} ${f.team ? `<small>${teamA(f.team)}</small>` : ""} set the ${f.where ? `${teamA(f.where)} ` : ""}${esc(f.book.toLowerCase())} record: <b>${esc(f.value)}</b><small>Old mark ${esc(f.old.value)}, ${f.old.linked ? playerLink(f.old.id, f.old.name) : esc(f.old.name)} (${f.old.year})</small></span></li>`).join("");
  return `<section class="panel dt-panel"><h2>Fell this season</h2>${rows ? `<ol class="dt-plain dt-fell">${rows}</ol>` : '<p class="muted small">No record has fallen this season or last.</p>'}</section>`;
}

function dtOwn(o) {
  const rows = o.rows.slice(0, 13).map((r) => {
    const c = r.career || r.season;
    return c ? `<li><span class="dt-who"><small${r.h8 ? ` data-h8="${esc(r.h8)}"` : ""}>${esc(r.name)}</small>${dtWho(c)}</span><b class="dt-v">${esc(c.value)}</b></li>` : "";
  }).join("");
  return `<section class="panel dt-panel"><h2>${teamA(o.team)}'s records</h2>${rows ? `<ol class="dt-plain">${rows}</ol>` : '<p class="muted small">Nothing in the team\'s book yet.</p>'}<p>${linkTo("stathist", { tab: "team", kind: o.team.ref.kind, id: o.team.ref.id }, "The whole book", "gochip")}</p></section>`;
}

// ---------- Player analytics (items 7 and 8, board 46) ----------
screens.analytics = async (a) => {
  const K = keep("analytics", {});
  for (const k of ["level", "division", "state", "group", "year"]) if (a[k] != null) { K[k] = a[k]; delete a[k]; }
  if (a.card != null) { K.card = Number(a.card); delete a.card; }
  const level = dtLevel(K);
  const args = { level, division: K.division || undefined, state: K.state || undefined, group: K.group || undefined, year: K.year ? Number(K.year) : undefined, min: K.min !== "" && K.min != null ? Number(K.min) : undefined, sort: K.sort != null ? Number(K.sort) : undefined, dir: K.dir || -1, class: K.class || undefined, rs: K.rs || undefined, age_min: K.age_min ? Number(K.age_min) : undefined, age_max: K.age_max ? Number(K.age_max) : undefined,
    view: K.view ? Number(K.view) : undefined, mode: K.mode || undefined, team: K.team != null && K.team !== "" ? Number(K.team) : undefined, conference: K.conference || undefined, spot: K.spot != null && K.spot !== "" ? Number(K.spot) : undefined, eligible: K.eligible || undefined, transfer: K.transfer || undefined, stars: K.stars ? Number(K.stars) : undefined, home: K.home || undefined };
  const x = await api("analytics", args);
  title("Player analytics", `${x.group} · ${x.year}`);
  const bar = `<div class="tbar">${dtSeg("dalevel", DT_LEVELS, x.level)}
    ${x.level === "college" ? dtSel("a:division", x.divisions, x.division) : ""}
    ${x.level === "hs" ? dtSel("a:state", Object.keys(STATE_NAMES).map((c) => [c, STATE_NAMES[c]]), x.state) : ""}
    ${dtSel("a:year", x.years, x.year)}
    <label class="dt-min">At least <input type="number" min="0" step="1" value="${x.min}" data-act="damin"> ${esc(x.volume)}</label>
    <span class="tspace"></span><span class="tag ${x.charted ? "gold" : "soft"}">${x.charted ? "Charted" : "Box score"}</span></div>
    <div class="tbar">${dtSeg("dagroup", x.groups.map((g) => [g, g]), x.group)}</div>
    <div class="tbar dt-filter">${dtSel("f:class", x.classes, x.class, x.level === "nfl" ? "Any year in the league" : "Every class")}
    ${x.level === "college" ? dtSel("f:rs", [["yes", "Redshirted"], ["no", "Never redshirted"]], x.rs, "Redshirt or not") : ""}
    <label class="dt-min">Age <input type="number" min="13" max="45" step="1" value="${x.age_min ?? ""}" placeholder="any" data-act="daage" data-k="age_min"> to <input type="number" min="13" max="45" step="1" value="${x.age_max ?? ""}" placeholder="any" data-act="daage" data-k="age_max"></label>
    ${x.teams.length ? dtSel("f:team", x.teams, x.team, x.level === "nfl" ? "Every club" : "Every team") : ""}
    ${x.conferences.length ? dtSel("f:conference", x.conferences, x.conference, "Every conference") : ""}
    ${x.spots.length ? dtSel("f:spot", x.spots, x.spot, "Every spot") : ""}
    ${x.level === "college" ? dtSel("f:eligible", [["yes", "Draft eligible"], ["no", "Not yet eligible"]], x.eligible, "Eligible or not") + dtSel("f:transfer", [["yes", "Transfers"], ["no", "Never transferred"]], x.transfer, "Transfer or not") + dtSel("f:stars", [5, 4, 3, 2, 1, 0].map((n) => [n, n ? `${n} star${n > 1 ? "s" : ""}` : "Unrated"]), x.stars, "Any stars") : ""}
    ${x.level !== "hs" ? dtSel("f:home", Object.keys(STATE_NAMES).map((c) => [c, "From " + STATE_NAMES[c]]), x.home, "Any home state") : ""}
    ${x.filtered ? `<button class="tbtn" data-act="daclear">Clear</button>` : ""}</div>
    ${dtViewsBar(x)}`;
  // Each stat's name explains itself on hover (H8, help.js), with the
  // middle value among every qualifier this season.
  const h8 = (c, n) => `data-h8="${esc(c.key)}"${c.avg != null ? ` data-h8avg="${esc(c.avg)}"` : ""}${c.mid != null ? ` data-h8mid="${esc(c.mid)}" data-h8n="${n}"` : ""}`;
  // Era + and opponent-adjusted columns carry a mark; their cells keep the
  // raw number beside.
  const marked = (c) => (x.mode === "era" && c.era) || (x.mode === "adj" && c.adj);
  const head = `<tr><th>#</th><th class="l">Player</th><th class="l">Pos</th><th class="l">Team</th><th data-h8="Games">G</th><th data-h8="Vol">Vol</th>${x.columns.map((c, i) => `<th ${h8(c, x.qualified)}><button class="dt-sort ${x.sort === i ? "on" : ""}" data-act="dasort" data-v="${i}">${esc(c.label)}${marked(c) ? `<sup class="dt-mk" data-h8="${x.mode === "era" ? "EraIndex" : "OppAdj"}">${x.mode === "era" ? "+" : "adj"}</sup>` : ""}</button></th>`).join("")}<th></th></tr>`;
  const cellOf = (r, i) => {
    const v = r.cells[i];
    if (v == null) return "";
    const raw = r.raw && marked(x.columns[i]) && r.raw[i] != null ? `<small class="dt-raw">${esc(r.raw[i])}</small>` : "";
    return esc(v) + raw;
  };
  const body = x.rows.map((r) => `<tr class="${K.card === r.id ? "on" : ""}"><td>${r.rank}</td><td class="l">${dtWho(r)}</td><td class="l">${esc(r.pos)}</td><td class="l">${r.team ? teamChip(r.team, 16) : ""}</td><td>${r.games}</td><td>${r.vol}</td>${r.cells.map((v, i) => `<td class="dt-pc" style="--p:${r.pct[i] ?? 0}%" title="${r.pct[i] != null ? r.pct[i] + "th percentile" : ""}">${cellOf(r, i)}</td>`).join("")}<td><button class="tbtn" data-act="dacard" data-id="${r.id}">Card</button></td></tr>`).join("");
  const none = x.filtered && x.qualified ? "Nobody who qualifies matches the filter." : "Nobody qualifies yet. Lower the minimum to see more.";
  const table = `<div class="tbl dt-at"><table><thead>${head}</thead><tbody>${body || `<tr><td colspan="${7 + x.columns.length}" class="muted">${none}</td></tr>`}</tbody></table></div>`;
  // The card reads the table's own season and division.
  const card = K.card != null ? await dtCard(K.card, x.level, x.year, x.division, x.state, x.group === "RET" ? "RET" : undefined, x.view) : "";
  const sub = x.filtered
    ? `${x.total} of the ${x.qualified} who qualify match the filter (${x.pool} played). Each number's shade is still his percentile among every qualifier; sorting goes by percentile, best first.`
    : `${x.total} qualify of ${x.pool} who played. Each number's shade is his percentile among the qualifiers; sorting goes by percentile, best first.`;
  const deepNote = x.deep_from != null && x.year < x.deep_from && x.columns.length && x.view > 0 ? `<p class="muted small">The depth numbers on this card begin with the ${x.deep_from} season.</p>` : "";
  return `<div class="pad sl dt">${pageHead({ kicker: `Data · ${esc(x.division || (x.state ? STATE_NAMES[x.state] || x.state : x.level === "nfl" ? "NFL" : ""))} · ${x.year}`, title: "Player analytics", sub, goto: gotoLine([["Stat history", "stathist"], ["Leaderboards", "leaders"]]) })}${bar}<div class="dt-main ${card ? "" : "full"}"><div class="dt-body">${deepNote}${table}</div>${card ? `<aside class="dt-side">${card}</aside>` : ""}</div>${dtSource(x.source)}</div>`;
};
// The card's views (items 4-11 and 28), the reading (Raw, Era +,
// Opponent-adjusted: items 12 and 13) and your saved views (item 14),
// which come back each visit.
function dtViewsBar(x) {
  const views = x.views.length > 1 ? dtSeg("daview", x.views.map((v, i) => [i, v]), x.view) : "";
  const modes = [["raw", "Raw"], ["era", "Era +"]].concat(x.level !== "hs" ? [["adj", "Opponent-adjusted"]] : []);
  const saved = prefs.dataViews || [];
  const pick = saved.length ? `<select class="ssel" data-act="dsel" data-k="v:load"><option value="">Saved views</option>${saved.map((v, i) => `<option value="${i}">${esc(v.name)}</option>`).join("")}</select><button class="tbtn" data-act="daviewdel" title="Forget the saved view picked last">Forget</button>` : "";
  return `<div class="tbar dt-views">${views}<span class="tspace"></span>${dtSeg("dareading", modes, x.mode)}${pick}<button class="tbtn" data-act="daviewsave">Save view</button></div>`;
}
const DT_VIEW_KEYS = ["level", "division", "state", "group", "view", "mode", "min", "class", "rs", "age_min", "age_max", "team", "conference", "spot", "eligible", "transfer", "stars", "home"];
GOTO.analytics = () => [["Stat history", "stathist"], ["Leaderboards", "leaders"]];

async function dtCard(id, level, year, division, state, group, view) {
  let c;
  try { c = await api("analytics_card", { id, level, year: year != null ? Number(year) : undefined, division: division || undefined, state: level === "hs" ? state || undefined : undefined, group, view: view || undefined }); } catch (e) { return ""; }
  if (c.error) return `<section class="panel dt-panel"><p class="muted">${esc(c.error)}</p></section>`;
  if (c.empty) return `<section class="panel dt-panel"><h2>${playerLink(c.id, c.name)}</h2><p class="muted">${esc(c.empty)}</p></section>`;
  const bars = c.metrics.map((m) => `<li><span class="dt-ml" tabindex="0" data-h8="${esc(m.key)}"${m.mid != null ? ` data-h8mid="${esc(m.mid)}" data-h8n="${c.field}"` : ""}>${esc(m.label)}</span><i class="dt-bar"><i style="width:${m.pct ?? 0}%"></i></i><b>${m.value == null ? "–" : esc(m.value)}</b><small>${m.pct == null ? "" : m.pct}</small></li>`).join("");
  const like = c.like.map((l) => `<li><span class="dt-who">${dtWho(l)}<small>${l.year} · ${l.team ? teamA(l.team) : ""} · ${esc(l.now)}</small></span><b class="dt-v" title="How close the two seasons read, 100 the same">${l.close}</b></li>`).join("");
  return `<section class="panel dt-panel dt-card"><header><div><div class="st-label">${esc(c.group)} card · ${c.year}${c.charted ? " · charted" : " · box score"}</div><h2>${playerLink(c.id, c.name)}</h2><small>${c.team ? teamA(c.team) : ""} · ${c.games} games · ${c.vol} ${esc(c.volume)}${c.qualified ? "" : " · below the minimum"}</small></div><button class="tbtn" data-act="dacard" data-id="">Close</button></header>
    <ul class="dt-bars">${bars}</ul><p class="muted small">Percentile among ${c.field} qualifiers that season.</p>
    ${c.view ? "" : `<h3 class="dt-h3">Seasons like his</h3>${like ? `<ol class="dt-plain">${like}</ol>` : '<p class="muted small">No past season reads close yet.</p>'}`}
    <p>${linkTo("player", { id: c.id, tab: "Data" }, "His Data tab", "gochip")}</p></section>`;
}

// ---------- Glossary (item 3): every stat in plain words ----------
screens.glossary = async () => {
  title("Glossary", "Data");
  const H = await hpStats();
  const terms = [...H.terms.values()].sort((a, b) => a.name.localeCompare(b.name));
  const way = (b) => (b > 0 ? "Higher is better" : b < 0 ? "Lower is better" : "Neither way is better by itself");
  const rows = terms.map((t) => `<li class="dt-term" data-q="${esc((t.name + " " + t.plain + " " + t.key).toLowerCase())}"><b>${esc(t.name)}</b><p>${esc(t.plain)}</p>${t.how ? `<p class="muted small">${esc(t.how)}</p>` : ""}<small>${way(t.better)}</small></li>`).join("");
  const sub = `${terms.length} stats, each in plain words with how the game works it out. Hover a stat's name anywhere in the game for the same words.`;
  return `<div class="pad sl dt">${pageHead({ kicker: "Data", title: "Glossary", sub, goto: gotoLine([["Player analytics", "analytics"], ["Stat history", "stathist"]]) })}<div class="tbar"><input class="ssel dt-gq" type="search" placeholder="Find a stat" data-act="dtgq"></div><ol class="dt-gloss">${rows}</ol></div>`;
};
GOTO.glossary = () => [["Player analytics", "analytics"], ["Stat history", "stathist"]];
document.addEventListener("input", (ev) => {
  const t = ev.target;
  if (!t.dataset || t.dataset.act !== "dtgq") return;
  const q = t.value.trim().toLowerCase();
  for (const li of document.querySelectorAll(".dt-gloss .dt-term")) li.hidden = !!q && !li.dataset.q.includes(q);
});

// ---------- History: the new tabs (items 23-25) ----------
// Record books and NFL records live on Stat history now; Champions gains
// the conference and state champions, Awards a view of one award's winners.
{
  for (const k of ["records", "nfl_records"]) {
    const i = HIST.findIndex((h) => h[1] === k);
    if (i >= 0) HIST.splice(i, 1);
  }
  const at = HIST.findIndex((h) => h[1] === "champions") + 1;
  HIST.splice(at, 0, ["Conference champions", "dconf"], ["State champions", "dstate"]);
}
const dtCoreHistory = screens.history;
screens.history = async (a) => {
  // An old Record book link (search, an older chip) becomes Stat history:
  // the route is rewritten in place, so the sidebar, the tabs and the level
  // and division controls are Stat history's own on every redraw.
  if (a.tab === "records" || a.tab === "nfl_records" || a.tab === "Record book") {
    const args = { level: a.tab === "nfl_records" ? "nfl" : "college" };
    if (a.division) { args.division = a.division; keep("stathist", {}).conference = null; }
    const r = S.beside && S.beside.args === a ? S.beside : S.route.args === a ? S.route : null;
    if (r) { r.screen = "stathist"; r.args = args; }
    return screens.stathist(args);
  }
  if (a.tab) { const m = HIST.find((h) => h[1] === a.tab); if (m) a.tab = m[0]; }
  const names = HIST.map((h) => h[0]);
  const tab = tabOf("htab", names, a);
  const key = HIST.find((h) => h[0] === tab)[1];
  const H = keep("history", {});
  if (key !== "dconf" && key !== "dstate" && !(key === "awards" && H.amode === "award")) {
    const html = await dtCoreHistory(a);
    if (key !== "awards") return html;
    // The Awards tab's two views.
    return html.replace('<div class="tabbody"><div class="tbar">', `<div class="tabbody"><div class="tbar">${dtSeg("damode", [["year", "By year"], ["award", "By award"]], "year")}`);
  }
  title("History", tab);
  let body = "";
  const coach = (r) => (r.coach ? coachLink(r.coach.id, r.coach.name) : "");
  if (key === "dconf") {
    const x = await api("champions_more", { tab: "conferences", division: H.cdiv || undefined });
    const rows = x.rows.filter((r) => !H.cconf || r.conference === H.cconf);
    body = `<div class="tbar">${dtSel("h:cdiv", x.divisions, x.division)}${dtSel("h:cconf", x.conferences, H.cconf, "Every conference")}</div>${table("dtconf", [{ k: "year", t: "Year" }, { k: "conference", t: "Conference", l: true }, { k: "team", t: "Champion", l: true, f: (r) => teamChip(r.team, 20), v: (r) => r.team.name }, { k: "record", t: "Record", l: true, f: (r) => esc(r.record) + (r.national ? ' <span class="tag gold">National champion</span>' : "") }, { k: "coach", t: "Coach", l: true, f: coach, v: (r) => (r.coach ? r.coach.name : "") }], rows, { tools: false, empty: "No conference champions kept yet." })}`;
  } else if (key === "dstate") {
    const x = await api("champions_more", { tab: "states", state: H.cstate || undefined });
    const rows = x.rows.filter((r) => !H.cclass || r.class === H.cclass);
    body = `<div class="tbar">${dtSel("h:cstate", x.states.map((c) => [c, STATE_NAMES[c] || c]), x.state)}${dtSel("h:cclass", x.classes, H.cclass, "Every class")}</div>${table("dtstate", [{ k: "year", t: "Year" }, { k: "class", t: "Class", l: true }, { k: "team", t: "Champion", l: true, f: (r) => teamChip(r.team, 20), v: (r) => r.team.name }, { k: "record", t: "Record", l: true }, { k: "coach", t: "Coach", l: true, f: coach, v: (r) => (r.coach ? r.coach.name : "") }], rows, { tools: false, empty: "No state champions kept yet." })}`;
  } else {
    const x = await api("champions_more", { tab: "award", award: H.aname || undefined, award_level: H.alevel || undefined });
    const pick = `<select class="ssel" data-act="dsel" data-k="h:award">${x.awards.map((w) => `<option value="${esc(w.level + "|" + w.name)}" ${w.name === x.award && w.level === x.award_level ? "selected" : ""}>${esc(w.level)} · ${esc(w.name)}</option>`).join("")}</select>`;
    body = `<div class="tbar">${dtSeg("damode", [["year", "By year"], ["award", "By award"]], "award")}${pick}</div>${table("dtaward", [{ k: "year", t: "Year" }, { k: "name", t: "Winner", l: true, f: (r) => dtWho(r), v: (r) => r.name }, { k: "pos", t: "Pos", l: true, f: (r) => esc(r.pos || "") }, { k: "team", t: "Team", l: true, f: (r) => (r.team ? teamChip(r.team, 18) : ""), v: (r) => (r.team ? r.team.name : "") }], x.rows, { tools: false, empty: "Nobody has won it yet." })}`;
  }
  return `<div class="pad">${pageHead({ kicker: "The world's memory", title: "History", goto: gotoLine([["Stat history", "stathist"], ["Leaderboards", "leaders"]]) })}${tabs("htab", names, tab)}<div class="tabbody">${body}</div></div>`;
};

// ---------- actions ----------
Object.assign(CORE_MORE, {
  dtlevel: (el) => { const K = keep("stathist", {}); K.level = el.dataset.v; K.conference = null; K.stat = null; refresh(el); },
  dtstat: (el) => { const K = keep("stathist", {}); K.stat = el.dataset.v === "" ? null : Number(el.dataset.v); refresh(el); },
  // A new level or group starts on the default sort, best first.
  dalevel: (el) => { const K = keep("analytics", {}); K.level = el.dataset.v; K.year = null; K.min = null; K.sort = null; K.dir = -1; K.card = null; K.class = null; K.rs = null; refresh(el); },
  daclear: (el) => { const K = keep("analytics", {}); for (const k of ["class", "rs", "age_min", "age_max", "team", "conference", "spot", "eligible", "transfer", "stars", "home"]) K[k] = null; refresh(el); },
  dagroup: (el) => { const K = keep("analytics", {}); K.group = el.dataset.v; K.min = null; K.sort = null; K.dir = -1; K.card = null; K.view = null; K.spot = null; refresh(el); },
  daview: (el) => { const K = keep("analytics", {}); K.view = Number(el.dataset.v) || null; K.sort = null; K.dir = -1; refresh(el); },
  dareading: (el) => { keep("analytics", {}).mode = el.dataset.v === "raw" ? null : el.dataset.v; refresh(el); },
  // The Mac app's webview has no window.prompt (it answers nothing), so the
  // name is asked in the page's own dialog.
  daviewsave: async (el) => {
    const K = keep("analytics", {});
    const name = await ask("Name this view", { value: [K.group || "QB", K.level || ""].filter(Boolean).join(" · "), ok: "Save view" });
    if (!name) return;
    const v = { name };
    for (const k of DT_VIEW_KEYS) if (K[k] != null && K[k] !== "") v[k] = K[k];
    prefs.dataViews = [...(prefs.dataViews || []).filter((x) => x.name !== name), v].slice(-20);
    savePrefs();
    refresh(el);
  },
  daviewdel: (el) => {
    const K = keep("analytics", {});
    const list = prefs.dataViews || [];
    const i = list.findIndex((v) => v.name === K.loaded);
    if (i < 0) return;
    list.splice(i, 1);
    prefs.dataViews = list;
    K.loaded = null;
    savePrefs();
    refresh(el);
  },
  dasort: (el) => { const K = keep("analytics", {}); const v = Number(el.dataset.v); if (K.sort === v) K.dir = (K.dir || -1) * -1; else { K.sort = v; K.dir = -1; } refresh(el); },
  dacard: (el) => { const K = keep("analytics", {}); K.card = el.dataset.id === "" || K.card === Number(el.dataset.id) ? null : Number(el.dataset.id); refresh(el); },
  damode: (el) => { keep("history", {}).amode = el.dataset.v; refresh(el); },
});
document.addEventListener("change", (ev) => {
  const t = ev.target;
  if (!t.dataset) return;
  if (t.dataset.act === "damin") { keep("analytics", {}).min = t.value; refresh(t); return; }
  if (t.dataset.act === "daage") { keep("analytics", {})[t.dataset.k] = t.value || null; refresh(t); return; }
  if (t.dataset.act !== "dsel") return;
  const k = t.dataset.k;
  if (k === "v:load") {
    // A saved view: its level, group, card view, reading and filters.
    const v = (prefs.dataViews || [])[Number(t.value)];
    if (!v) return;
    const K = keep("analytics", {});
    for (const key of DT_VIEW_KEYS) K[key] = v[key] ?? null;
    K.loaded = v.name; K.year = null; K.sort = null; K.dir = -1; K.card = null;
  } else if (k.startsWith("f:")) {
    // The class filter keeps the season, the minimum and the open card.
    keep("analytics", {})[k.slice(2)] = t.value || null;
  } else if (k.startsWith("a:")) {
    const K = keep("analytics", {});
    K[k.slice(2)] = t.value || null;
    if (k !== "a:year") { K.year = null; K.card = null; }
    K.min = null;
  } else if (k.startsWith("h:")) {
    const H = keep("history", {});
    if (k === "h:award") { const [lv, ...n] = t.value.split("|"); H.alevel = lv; H.aname = n.join("|"); }
    else {
      H[k.slice(2)] = t.value || null;
      if (k === "h:cdiv") H.cconf = null;
      if (k === "h:cstate") H.cclass = null;
    }
  } else {
    const K = keep("stathist", {});
    K[k] = t.value || null;
    if (k === "division") K.conference = null;
  }
  refresh(t);
});

// ---------- The player page's Data tab (items 11, 15, 28; minor bug 44) ----------
// Facet grades, where he lined up, the throw map by zone, the route map,
// run paths, the pressure map, his splits and his games, one season at a
// time, as deep as your charting goes at his level.
PLAYER_TABS.push({
  name: "Data",
  after: "Stats",
  show: (p) => p.stage !== "HighSchool",
  draw: async (p) => {
    const P = keep("pdata:" + p.id, {});
    let d;
    try { d = await api("player_data", { id: p.id, level: P.level || undefined, year: P.year ? Number(P.year) : undefined }); } catch (e) { return `<div class="empty">${esc(e.message || "No data.")}</div>`; }
    if (d.error) return `<div class="empty">${esc(d.error)}</div>`;
    const lv = (k) => ({ college: "College", nfl: "NFL" }[k] || k);
    const levels = d.other ? dtSeg("pdlevel", [[d.level, lv(d.level)], [d.other, lv(d.other)]].sort((a, b) => (a[0] === "college" ? -1 : 1)), d.level) : "";
    const years = d.years.length > 1 ? `<select class="ssel" data-act="pdyear" data-pid="${p.id}">${d.years.map((y) => `<option value="${y}" ${y === d.year ? "selected" : ""}>${y}</option>`).join("")}</select>` : "";
    const bar = `<div class="tbar" data-pid="${p.id}">${levels}${years}<span class="tspace"></span>${d.team ? teamChip(d.team, 18) : ""}${d.games != null ? `<span class="muted small">${d.games} games · ${d.starts} starts</span>` : ""}</div>`;
    if (d.empty) return `<div class="dt pdt">${bar}<div class="empty">${esc(d.empty)}</div></div>`;
    const panel = (h, body, h8) => (body ? `<section class="panel dt-panel"><h2${h8 ? ` data-h8="${esc(h8)}"` : ""}>${esc(h)}</h2>${body}</section>` : "");
    const facets = (d.facets || []).length ? `<ul class="dt-bars pdt-bars">${d.facets.map((f) => `<li><span class="dt-ml">${esc(f.facet[0].toUpperCase() + f.facet.slice(1))}</span><i class="dt-bar"><i style="width:${Math.max(0, Math.min(100, f.value))}%"></i></i><b>${esc(f.grade)}</b><small>${f.plays}</small></li>`).join("")}</ul><p class="muted small">Grades 0-100 from the charting of every play; the small number is the plays behind each.</p>` : "";
    const snaps = (d.alignment || []).reduce((a, s) => a + s.snaps, 0);
    const align = snaps ? `<ul class="dt-bars pdt-bars">${d.alignment.map((s) => `<li><span class="dt-ml">${esc(s.spot)}</span><i class="dt-bar"><i style="width:${(100 * s.snaps) / snaps}%"></i></i><b>${s.snaps}</b><small>${Math.round((100 * s.snaps) / snaps)}%</small></li>`).join("")}</ul>` : "";
    const throws = d.throws ? pdtThrows(d.throws) : "";
    const routes = (d.routes || []).length ? pdtTable(["Route", "Tgt", "Rec", "Catch%", "Yds", "Y/T", "EPA/T", "Sep"], d.routes.map((r) => [r.route, r.tgt, r.rec, r.catch, r.yds, r.ypt, r.epa, r.sep])) : "";
    const runs = d.runs ? pdtRuns(d.runs) : "";
    const prs = (d.pressure || []).some((s) => s.prs) ? pdtPressure(d.pressure) : "";
    const splits = (d.splits || []).map((t) => panel(`Splits · ${t.title}`, pdtTable(["Split", ...t.heads], t.rows.map((r) => [r.split, ...r.cells])))).join("");
    const log = d.log && d.log.rows.length ? pdtTable(["Date", "Opponent", "Result", ...d.log.heads], d.log.rows.map((g) => [g.date, `${g.at ? g.at + " " : ""}${g.opp ? teamA(g.opp) : "?"}${g.kind && g.kind !== "regular" ? ` <span class="tag soft">${esc(g.kind)}</span>` : ""}`, g.result, ...g.cells]), [1]) : "";
    const ch = d.charting ? `<p class="dt-src">${esc(d.charting.kind[0].toUpperCase() + d.charting.kind.slice(1))}${d.charting.days > 1 ? `, ${d.charting.days} days behind` : ""}.</p>` : `<p class="dt-src">Box scores only at this level: no third party charts it snap by snap, so the charted maps don't show.</p>`;
    return `<div class="dt pdt">${bar}<div class="pdt-grid">${panel("Facet grades", facets, "GrPass")}${panel("Where he lined up", align)}${panel(`Throw map · as the ${d.throws ? d.throws.role : ""}`, throws)}${panel("Run paths", runs)}${panel("Route map", routes)}${panel("Pressure map", prs)}</div>${splits}${panel("Game by game", log)}${ch}</div>`;
  },
});

// A plain table of rows (cells already strings, the `html` columns as HTML).
function pdtTable(heads, rows, html = []) {
  const cell = (v, i) => (v == null ? "" : html.includes(i) ? v : esc(String(v)));
  return `<div class="tbl"><table><thead><tr>${heads.map((h, i) => `<th class="${i === 0 || html.includes(i) ? "l" : ""}">${esc(h)}</th>`).join("")}</tr></thead><tbody>${rows.map((r) => `<tr>${r.map((v, i) => `<td class="${i === 0 || html.includes(i) ? "l" : ""}">${cell(v, i)}</td>`).join("")}</tr>`).join("")}</tbody></table></div>`;
}

// The throw map: depth rows (deep at the top) by thirds, shaded by EPA a throw.
function pdtThrows(t) {
  const rows = [...t.rows].reverse().map((r) => `<div class="pdt-zrow"><span class="pdt-zl">${esc(r.depth)}</span>${r.cells.map((c) => {
    const e = Math.max(-0.6, Math.min(0.6, c.epa_v || 0));
    const tone = c.att ? (e >= 0 ? `--z:${Math.round((e / 0.6) * 60)}%` : `--n:${Math.round((-e / 0.6) * 60)}%`) : "";
    return `<div class="pdt-z ${c.att ? "" : "none"} ${e >= 0 ? "up" : "dn"}" style="${tone}"><b>${c.att ? esc(c.cmp || "") : "–"}</b><small>${c.att} att · ${c.yds} yds</small><small>${c.epa ? esc(c.epa) + " EPA" : ""}</small></div>`;
  }).join("")}</div>`).join("");
  return `<div class="pdt-zone"><div class="pdt-zrow pdt-zh"><span></span>${t.thirds.map((x) => `<span>${esc(x)}</span>`).join("")}</div>${rows}<div class="pdt-los">Line of scrimmage</div></div>`;
}

// Run paths: his carries by gap, left to right from the offense's side.
function pdtRuns(rows) {
  const most = Math.max(1, ...rows.map((r) => r.car));
  return `<div class="pdt-gaps">${rows.map((r) => `<div class="pdt-gap"><i style="height:${Math.round((70 * r.car) / most) + 4}px"></i><b>${r.car}</b><small>${esc(r.gap)}</small><small>${r.ypc ? esc(r.ypc) + " Y/C" : ""}</small><small>${r.epa ? esc(r.epa) + " EPA" : ""}</small><small>${r.succ ? esc(r.succ) + " succ" : ""}</small></div>`).join("")}</div>`;
}

// The pressure map: his pressures by the blocker he beat, across the line.
function pdtPressure(rows) {
  const most = Math.max(1, ...rows.map((r) => r.prs));
  return `<div class="pdt-gaps pdt-prs">${rows.map((r) => `<div class="pdt-gap"><i style="height:${Math.round((70 * r.prs) / most) + 4}px"></i><b>${r.prs}</b><small>${esc(r.spot)}</small><small>${r.sacks !== "0" ? esc(r.sacks) + " sacks" : ""}</small><small>${r.ttp ? esc(r.ttp) + " s" : ""}</small></div>`).join("")}</div>`;
}

Object.assign(CORE_MORE, {
  pdlevel: (el) => {
    const id = (el.closest("[data-pid]") || {}).dataset?.pid || (S.route && S.route.args && S.route.args.id);
    const P = keep("pdata:" + id, {});
    P.level = el.dataset.v; P.year = null;
    refresh(el);
  },
});
document.addEventListener("change", (ev) => {
  const t = ev.target;
  if (!t.dataset || t.dataset.act !== "pdyear") return;
  keep("pdata:" + t.dataset.pid, {}).year = t.value;
  refresh(t);
});

// ---------- Team data (lane NB7, items 16-23; boards 44 and 53) ----------
// A team's Data page (its analytics with ranks, tendencies, games with the
// unit grades and snap shares, what came back, and its power rating and
// odds), its stats by season, and every team ranked, the tiers chart and
// the power ratings. Every number names itself on hover (data-h8).
Object.assign(I, {
  teamdata: '<path d="M4 20V9M10 20V4M16 20v-7M22 20H2"/>',
  teamlead: '<path d="M7 20V10M12 20V4M17 20v-6"/><circle cx="12" cy="3" r="1"/>',
});
navAdd({ go: "teamlead", label: "Team leaderboards", icon: "teamlead", sec: "data", after: "leaders" });
navAdd({ go: "teamdata", label: "Team data", icon: "teamdata", sec: "data", after: "analytics" });
Object.assign(CRUMBS, { teamdata: "Team data", teamseasons: "Stats by season", teamlead: "Team leaderboards" });
{
  const core = GOTO.team;
  GOTO.team = (a, t) => {
    const list = core(a, t);
    const ta = t.ref ? { kind: t.ref.kind, id: t.ref.id } : {};
    if (t.ref && t.ref.kind !== "other") list.splice(2, 0, ["Team data", "teamdata", ta], ["Stats by season", "teamseasons", ta]);
    return list;
  };
}
const TD_TABS = ["Analytics", "Tendencies", "Games", "Roster", "Power"];
const tdH8 = (k) => (k ? ` data-h8="${esc(k)}"` : "");
const tdRank = (c, n) => (c && c.rank ? `<small class="dt-rk2 ${c.rank <= Math.max(1, Math.round(n / 10)) ? "top" : c.rank > n - Math.max(1, Math.round(n / 10)) ? "low" : ""}">${c.rank}</small>` : "");
const tdArgs = (a) => (a && a.kind ? { kind: a.kind, id: Number(a.id) } : {});
function tdTable(heads, rows, keys = []) {
  return `<div class="tbl"><table><thead><tr>${heads.map((h, i) => `<th class="${i === 0 ? "l" : ""}"${tdH8(keys[i])}>${esc(h)}</th>`).join("")}</tr></thead><tbody>${rows.map((r) => `<tr>${r.map((v, i) => `<td class="${i === 0 ? "l" : ""}">${v ?? ""}</td>`).join("")}</tr>`).join("")}</tbody></table></div>`;
}
const tdPanel = (h, body, h8) => (body ? `<section class="panel dt-panel"><h2${tdH8(h8)}>${esc(h)}</h2>${body}</section>` : "");

screens.teamdata = async (a) => {
  const K = keep("teamdata:" + (a.kind || "mine") + ":" + (a.id ?? ""), {});
  if (a.year) { K.year = Number(a.year); delete a.year; }
  const tab = tabOf("tdtab", TD_TABS, a);
  const x = await api("team_data", { ...tdArgs(a), year: K.year || undefined, tab: tab.toLowerCase() });
  if (x.error && !x.team) return `<div class="pad"><div class="empty">${esc(x.error)}</div></div>`;
  title("Team data", x.team.name);
  setColors(x.team.colors);
  const ta = x.team.ref;
  const years = x.years.length > 1 ? `<select class="ssel" data-act="tdyear" data-k="teamdata:${esc((a.kind || "mine") + ":" + (a.id ?? ""))}">${x.years.map((y) => `<option value="${y}" ${y === x.year ? "selected" : ""}>${y}</option>`).join("")}</select>` : "";
  const bar = `<div class="tbar">${years}<span class="tspace"></span><span class="muted small">${x.record ? `${esc(x.record)} · ` : ""}ranked among ${esc(x.group)} (${x.n})</span></div>`;
  let body = "";
  if (x.error) body = `<div class="empty">${esc(x.error)}</div>`;
  else if (tab === "Analytics") body = tdAnalytics(x);
  else if (tab === "Tendencies") body = tdTendencies(x);
  else if (tab === "Games") body = tdGames(x);
  else if (tab === "Roster") body = tdRoster(x);
  else body = tdPower(x);
  const src = x.depth == null
    ? "Box scores only for your seat at this level: the charted numbers don't show."
    : `${x.charting[0].toUpperCase() + x.charting.slice(1)} at this level.${x.depth < 2 ? " Film charting adds pressure, coverage, the calls and the grades." : ""}`;
  const goto = gotoLine([["Team page", "team", ta], ["Stats by season", "teamseasons", ta], ["Team leaderboards", "teamlead", { level: x.level }]]);
  return `<div class="pad sl dt td">${pageHead({ kicker: `Data · ${x.mine ? "self-scout" : esc(x.group)}`, title: `${mark(x.team, 30)} ${esc(x.team.name)}`, sub: `The ${x.year} season`, goto })}${bar}${tabs("tdtab", TD_TABS, tab)}<div class="td-body">${body}</div>${dtSource(src)}</div>`;
};
GOTO.teamdata = (a) => [["Team page", "team", tdArgs(a)], ["Stats by season", "teamseasons", tdArgs(a)], ["Team leaderboards", "teamlead"]];

function tdAnalytics(x) {
  if (!x.sides) return `<div class="empty">No games counted for this team that season.</div>`;
  return x.sides.map((s) => {
    if (!s.rows.length) return "";
    const two = s.heads.length === 2;
    const heads = two ? [s.name, "Offense", "", "Defense", "", "Level"] : [s.name, "Team", "", "Level"];
    const rows = s.rows.map((r) => {
      const cells = r.cells.map((c) => (c ? [`<b>${esc(c.text)}</b>`, tdRank(c, x.n)] : ["–", ""])).flat();
      const avg = r.cells.map((c) => (c && c.avg ? esc(c.avg) : "")).join(two ? " / " : "");
      return [`<span${tdH8(r.key)} tabindex="0">${esc(r.label)}</span>`, ...cells, `<small class="muted">${avg}</small>`];
    });
    return tdPanel(s.name, tdTable(heads, rows));
  }).join("") + `<p class="muted small">The small number is the rank among ${esc(x.group)}; a defense ranks by what it kept the other side to. Level is the group's average${x.sides[0].heads.length === 2 ? ", offense / defense" : ""}.</p>`;
}

function tdTendencies(x) {
  const t = x.tendencies;
  if (!t) return `<div class="empty">${x.level === "hs" ? "High school is box scores: no play-by-play to read the calls from." : "Your seat doesn't chart this level: the calls by situation don't show."}</div>`;
  const calls = tdTable(["Situation", "Snaps", "Pass", "Expected", "Over", "Level"], t.calls.map((r) => [esc(r.label), r.snaps, esc(r.pass), esc(r.expected), `<b class="${parseFloat(r.over) > 0 ? "td-up" : "td-dn"}">${esc(r.over)}</b>`, esc(r.level || "")]), ["", "TendSnaps", "TendPass", "TendXPass", "TendOver", "TendLevel"]);
  let html = tdPanel("Run or pass by situation", calls, "TmProe");
  if (t.offense) {
    const o = t.offense;
    const share = (rows, first, key) => tdTable([first, "Snaps", "Share", "Pass", "EPA a play"], rows.map((r) => [esc(r.label), r.snaps, esc(r.share), esc(r.pass), esc(r.epa)]), [key, "PkgSnaps", "Share", "PkgPass", "EpaPlay"]);
    const rates = `<ul class="dt-plain">${o.rates.map((r) => `<li><span${tdH8(r.key)} tabindex="0">${esc(r.label)}</span><b>${esc(r.v || "–")}</b></li>`).join("")}</ul>`;
    html += `<div class="td-grid">${tdPanel("Personnel", share(o.personnel, "Personnel", "Personnel"), "Personnel")}${tdPanel("Backfield", share(o.backfield, "Backfield", "Backfield"), "Backfield")}${tdPanel("How it gets there", rates)}</div>`;
  }
  if (t.defense) {
    const d = t.defense;
    const list = (rows, k) => `<ul class="dt-plain">${rows.map((r) => `<li><span>${esc(r.label)}</span><b>${esc(r[k] || "–")}</b></li>`).join("")}</ul>`;
    const pk = tdTable(["Package", "Snaps", "Share", "Pass", "EPA a play"], d.packages.map((r) => [esc(r.label), r.snaps, esc(r.share), esc(r.pass), esc(r.epa)]), ["Package", "PkgSnaps", "Share", "PkgPass", "EpaPlay"]);
    const bx = tdTable(["Box", "Share", "Runs", "Y/C"], d.boxes.map((r) => [esc(r.label), esc(r.share), r.runs, esc(r.ypc || "")]), ["BoxCount", "Share", "PkgRun", "Ypc"]);
    const rates = `<ul class="dt-plain"><li><span data-h8="ManRate" tabindex="0">Man coverage</span><b>${esc(d.man || "–")}</b></li>${d.rates.map((r) => `<li><span${tdH8(r.key)} tabindex="0">${esc(r.label)}</span><b>${esc(r.v || "–")}</b></li>`).join("")}</ul>`;
    html += `<h3 class="dt-h3">Its defense</h3><div class="td-grid">${tdPanel("Coverage shells", list(d.shells, "share"), "Shell")}${tdPanel("Coverages", list(d.coverages.slice(0, 8), "share"), "CoverCall")}${tdPanel("Blitz by down", list(d.blitz, "v"), "TeamBlitz")}${tdPanel("Its calls", rates)}${tdPanel("Packages", pk, "Package")}${tdPanel("Box counts", bx, "BoxCount")}</div>`;
  } else if (x.depth != null && x.depth < 2) {
    html += `<p class="muted small">Film charting adds personnel, the backfield, motion, play action and the defense's coverages, blitzes and boxes.</p>`;
  }
  return html;
}

function tdGames(x) {
  const g = x.games_list;
  if (!g || !g.rows.length) return `<div class="empty">No games counted for this team that season.</div>`;
  const heads = ["Date", "Opponent", "Result", "Yards", ...(g.rows[0].epa ? ["EPA a play"] : []), ...(g.graded ? ["Offense grade", "Defense grade"] : [])];
  const keys = ["", "", "", "TmYdsG", ...(g.rows[0].epa ? ["TmEpa"] : []), ...(g.graded ? ["UnitGrade", "UnitGrade"] : [])];
  const rows = g.rows.map((r) => [esc(r.date), `${r.home ? "" : r.neutral ? "vs " : "at "}${r.opp ? teamA(r.opp) : "?"}${r.kind && r.kind !== "regular" ? ` <span class="tag soft">${esc(r.kind)}</span>` : ""}`, esc(r.result), `${r.yards[0]}-${r.yards[1]}`, ...(r.epa ? [`${esc(r.epa[0])} / ${esc(r.epa[1])}`] : []), ...(g.graded ? [esc(r.grades[0] || ""), esc(r.grades[1] || "")] : [])]);
  let html = tdPanel("Game by game", tdTable(heads, rows, keys));
  if (g.snaps) {
    const n = g.rows.length;
    const unit = (list) => tdTable(["Player", "Pos", "Snaps", ...g.rows.map((_, i) => String(i + 1))], list.map((m) => [m.id != null ? playerLink(m.id, m.name) : esc(m.name), esc(m.pos || ""), m.snaps, ...m.games.map((v) => (v == null ? '<span class="muted">–</span>' : `<span class="td-snap" style="--p:${v}%">${v}</span>`))]), ["", "", "Snaps", ...Array(n).fill("SnapShare")]);
    html += `<div class="td-grid2">${tdPanel("Snap shares · offense", unit(g.snaps.offense), "SnapShare")}${tdPanel("Snap shares · defense", unit(g.snaps.defense), "SnapShare")}</div><p class="muted small">Each column is a game in the order above: the share of the unit's snaps he played.</p>`;
  }
  return html;
}

function tdRoster(x) {
  const b = x.back || {};
  let html = "";
  if (b.production) {
    const p = b.production;
    html += tdPanel("Returning production", `<p class="bigline"><b>${esc(p.overall)}</b> overall · offense ${esc(p.offense)} · defense ${esc(p.defense)}</p><ul class="dt-plain">${p.stats.map((s) => `<li><span>${esc(s.label)}</span><b>${esc(s.share)}</b></li>`).join("")}</ul>`, "RetProd");
  }
  if (b.starters) {
    const s = b.starters;
    html += tdPanel("Starters back", `<ul class="dt-plain"><li><span>Offense</span><b>${s.offense[0]} of ${s.offense[1]}</b></li><li><span>Defense</span><b>${s.defense[0]} of ${s.defense[1]}</b></li><li><span data-h8="StartsBack" tabindex="0">Starts back</span><b>${esc(s.starts[0])} / ${esc(s.starts[1])}</b></li></ul>`, "StartersBack");
  }
  if (b.snaps) html += tdPanel("Snaps back", `<ul class="dt-plain"><li><span>Offense</span><b>${esc(b.snaps[0])}</b></li><li><span>Defense</span><b>${esc(b.snaps[1])}</b></li></ul>`, "SnapsBack");
  const t = b.talent;
  if (t && t.stars != null) {
    html += tdPanel("Roster talent", `<p class="bigline"><b>${esc(t.stars)}★</b> average${t.rank ? ` · ${t.rank[0]} of ${t.rank[1]}` : ""}</p><ul class="dt-plain"><li><span>Offense</span><b>${esc(t.offense)}★</b></li><li><span>Defense</span><b>${esc(t.defense)}★</b></li><li><span data-h8="BlueChips" tabindex="0">Blue-chip share</span><b>${esc(t.blue_chips || "–")}</b></li><li><span>Men</span><b>${t.men}</b></li></ul>`, "TeamStars");
  } else if (t && t.seniors) {
    html += tdPanel("Its regulars", `<ul class="dt-plain"><li><span>Offense</span><b>${t.starters[0]} starters, ${t.seniors[0]} seniors</b></li><li><span>Defense</span><b>${t.starters[1]} starters, ${t.seniors[1]} seniors</b></li></ul>`, "Seniors");
  } else if (b.seniors_now != null) {
    html += tdPanel("Seniors", `<p class="bigline"><b>${b.seniors_now}</b> seniors on the roster</p>`, "Seniors");
  }
  return html ? `<div class="td-grid">${html}</div>` : `<div class="empty">No season before this one is counted yet.</div>`;
}

function tdPower(x) {
  const p = x.power;
  if (!p) return `<div class="empty">No rating for this team that season.</div>`;
  if (p.closing) return tdPanel("Closing power rating", `<p class="bigline"><b>${esc(p.closing.overall)}</b> · ${p.rank[0]} of ${p.rank[1]}</p><ul class="dt-plain"><li><span data-h8="PowerOff" tabindex="0">Offense</span><b>${esc(p.closing.off)}</b></li><li><span data-h8="PowerDef" tabindex="0">Defense</span><b>${esc(p.closing.def)}</b></li></ul>`, "PowerRtg");
  const o = p.odds;
  const odds = o ? `<ul class="dt-plain"><li><span data-h8="ExpWins" tabindex="0">Expected wins</span><b>${esc(o.wins)}-${esc(o.losses)}</b></li>${x.level === "college" ? `<li><span data-h8="BowlOdds" tabindex="0">Bowl eligible</span><b>${esc(o.bowl)}</b></li>` : ""}${x.level !== "hs" ? `<li><span data-h8="LeagueOdds" tabindex="0">Wins its ${x.level === "nfl" ? "division" : "conference"}</span><b>${esc(o.league)}</b></li><li><span data-h8="PlayoffOdds" tabindex="0">Playoff</span><b>${esc(o.playoff)}</b></li><li><span data-h8="ByeOdds" tabindex="0">First-round bye</span><b>${esc(o.bye)}</b></li>` : `<li><span data-h8="WinningOdds" tabindex="0">Winning record</span><b>${esc(o.winning)}</b></li>`}</ul>` : "";
  const now = `<p class="bigline"><b>${esc(p.now.overall)}</b> · ${p.rank[0]} of ${p.rank[1]} · from ${p.games} game${p.games === 1 ? "" : "s"}</p><ul class="dt-plain"><li><span data-h8="PowerOff" tabindex="0">Offense</span><b>${esc(p.now.off)}</b></li><li><span data-h8="PowerDef" tabindex="0">Defense</span><b>${esc(p.now.def)}</b></li>${p.preseason ? `<li><span data-h8="Projection" tabindex="0">Preseason projection</span><b>${esc(p.preseason.overall)}</b></li>` : ""}<li><span>Home field</span><b>${esc(p.home)} points</b></li></ul>`;
  const coming = p.coming.length ? tdTable(["Date", "Opponent", "Margin", "Win chance"], p.coming.map((g) => [esc(g.date), `${g.home ? "" : g.neutral ? "vs " : "at "}${g.opp ? teamA(g.opp) : "?"}`, esc(g.margin), `<b>${esc(g.chance)}</b>`]), ["", "", "", "WinChance"]) : `<div class="empty">No games left to play.</div>`;
  return `<div class="td-grid">${tdPanel("Power rating", now, "PowerRtg")}${tdPanel("Odds", odds, "ExpWins")}</div>${tdPanel("Coming games", coming, "WinChance")}<p class="muted small">Ratings from scores alone, the schedule and home field taken out; results land within ${esc(p.sigma)} points of the ratings about two times in three.</p>`;
}

// Stats by season (item 18, board 44).
screens.teamseasons = async (a) => {
  const K = keep("teamseasons", { unit: 1, per: "game" });
  const x = await api("team_seasons", { ...tdArgs(a), unit: K.unit });
  if (x.error) return `<div class="pad"><div class="empty">${esc(x.error)}</div></div>`;
  title("Stats by season", x.team.name);
  setColors(x.team.colors);
  const ta = x.team.ref;
  const now = x.rows[0] && x.rows[0].coordinator;
  const cols = [
    { k: "year", t: "Year", f: (r) => linkTo("teamdata", { ...ta, year: r.year }, String(r.year)) },
    { k: "record", t: "Record", l: true },
    { k: "head", t: "Head coach", l: true, f: (r) => (r.head ? coachLink(r.head.id, r.head.name) : ""), v: (r) => (r.head ? r.head.name : "") },
    ...(x.role ? [{ k: "coord", t: x.role[0].toUpperCase() + x.role.slice(1), l: true, f: (r) => (r.coordinator ? coachLink(r.coordinator.id, r.coordinator.name) : ""), v: (r) => (r.coordinator ? r.coordinator.name : "") }] : []),
    ...x.cols.map((c, i) => ({
      k: "c" + i, t: c.label, h8: c.key,
      v: (r) => (r.cells[i] ? parseFloat(r.cells[i].text) : null),
      f: (r) => { const c2 = r.cells[i]; if (!c2) return "–"; const v = K.per === "total" && c2.total ? c2.total : c2.text; return `${esc(v)} ${c2.rank ? `<small class="dt-rk2" title="Rank among ${esc(r.group)} (${r.n})">${c2.rank}</small>` : ""}`; },
      csv: (r) => (r.cells[i] ? (K.per === "total" && r.cells[i].total) || r.cells[i].text : ""),
    })),
    ...(x.talent ? [{ k: "talent", t: x.talent, h8: x.level === "college" ? "TeamStars" : "Seniors", f: (r) => esc(r.talent ?? "") }] : []),
    { k: "back", t: "Starters back", h8: "StartersBack", f: (r) => (r.back ? `${r.back[0]} of ${r.back[1]}` : ""), v: (r) => (r.back ? r.back[0] : null) },
  ];
  const rows = x.rows.map((r) => ({ ...r, cls: now && r.coordinator && r.coordinator.id === now.id ? "td-hl" : "" }));
  const bar = `<div class="tbar">${dtSeg("tsunit", x.units.map((u, i) => [i, u]), x.unit)}${dtSeg("tsper", [["game", "Per game"], ["total", "Totals"]], K.per)}<span class="tspace"></span>${now ? `<span class="muted small">Highlighted: ${esc(now.name)}'s seasons</span>` : ""}</div>`;
  const goto = gotoLine([["Team data", "teamdata", ta], ["Team page", "team", ta], ["Team leaderboards", "teamlead", { level: x.level }]]);
  return `<div class="pad sl dt td">${pageHead({ kicker: "Data", title: `${mark(x.team, 30)} ${esc(x.team.name)} › Stats by season`, sub: "Each number with its rank at the level that season. A year opens that season's team data.", goto })}${bar}${x.rows.length ? table("teamseasons", cols, rows, { sort: null }) : '<div class="empty">No seasons counted yet.</div>'}</div>`;
};
GOTO.teamseasons = (a) => [["Team data", "teamdata", tdArgs(a)], ["Team page", "team", tdArgs(a)], ["Team leaderboards", "teamlead"]];

// Team leaderboards, the tiers chart and the power ratings (items 19, 20, 23).
const TL_TABS = ["Leaderboard", "Tiers", "Power"];
screens.teamlead = async (a) => {
  const K = keep("teamlead", {});
  for (const k of ["level", "division", "state"]) if (a[k] != null) { K[k] = a[k]; delete a[k]; }
  const tab = tabOf("tltab", TL_TABS, a);
  const view = { Leaderboard: "table", Tiers: "tiers", Power: "power" }[tab];
  const x = await api("team_leaders", { level: dtLevel(K), view, year: K.year || undefined, division: K.division || undefined, state: K.state || undefined, stat: K.stat || undefined, side: K.side || 0 });
  title("Team leaderboards", tab);
  const years = `<select class="ssel" data-act="tlsel" data-k="year">${x.years.map((y) => `<option value="${y}" ${y === x.year ? "selected" : ""}>${y}</option>`).join("")}</select>`;
  const where = x.level === "college" ? `<select class="ssel" data-act="tlsel" data-k="division">${x.divisions.map((d) => `<option ${d === x.division ? "selected" : ""}>${esc(d)}</option>`).join("")}</select>` : x.level === "hs" ? `<select class="ssel" data-act="tlsel" data-k="state">${x.states.map((c) => `<option value="${esc(c)}" ${c === x.state ? "selected" : ""}>${esc(STATE_NAMES[c] || c)}</option>`).join("")}</select>` : "";
  let tools = "";
  let body = "";
  if (view === "table") {
    const st = x.stats.find((s) => s.key === x.stat) || x.stats[0];
    tools = `<select class="ssel" data-act="tlsel" data-k="stat">${x.stats.map((s) => `<option value="${esc(s.key)}" ${s.key === x.stat ? "selected" : ""}>${esc(s.label)}</option>`).join("")}</select>${st && st.sides ? dtSeg("tlside", [[0, "Offense"], [1, "Defense"]], x.side) : ""}`;
    const cols = [
      { k: "rank", t: "#" },
      { k: "team", t: "Team", l: true, f: (r) => teamChip(r.team, 18), v: (r) => (r.team ? r.team.name : "") },
      { k: "record", t: "Record", l: true },
      { k: "value", t: st ? st.label : "", h8: x.stat, v: (r) => parseFloat(r.value), f: (r) => `<b>${esc(r.value)}</b>` },
      ...x.cols.map((c, i) => ({ k: "c" + i, t: c.label, h8: c.key, v: (r) => parseFloat(r.cells[i]), f: (r) => esc(r.cells[i] ?? "–") })),
    ];
    body = x.rows.length ? table("teamlead", cols, x.rows.map((r) => ({ ...r, mine: r.team && r.team.mine })), { sort: null }) : '<div class="empty">No teams counted that season.</div>';
  } else if (view === "tiers") {
    body = tlTiers(x);
  } else {
    body = tlPower(x);
  }
  const bar = `<div class="tbar">${dtSeg("tllevel", DT_LEVELS, x.level)}${where}${years}${tools}<span class="tspace"></span><span class="muted small">${x.n} teams</span></div>`;
  return `<div class="pad sl dt td">${pageHead({ kicker: "Data", title: "Team leaderboards", sub: "Every team at a level, any season. A team opens its page; Team data on its page has the rest.", goto: gotoLine([["Leaderboards", "leaders"], ["Player analytics", "analytics"], ["Team data", "teamdata"]]) })}${bar}${tabs("tltab", TL_TABS, tab)}<div class="td-body">${body}</div></div>`;
};
GOTO.teamlead = () => [["Leaderboards", "leaders"], ["Player analytics", "analytics"], ["Team data", "teamdata"]];

// The tiers chart: offense across, defense up (allowing less is higher),
// each team its own logo.
function tlTiers(x) {
  const pts = x.points || [];
  if (pts.length < 2) return '<div class="empty">Too few teams counted that season.</div>';
  const os = pts.map((p) => p.o), ds = pts.map((p) => p.d);
  const pad = (lo, hi) => { const m = (hi - lo) * 0.08 || 0.05; return [lo - m, hi + m]; };
  const [ox0, ox1] = pad(Math.min(...os), Math.max(...os));
  const [dy0, dy1] = pad(Math.min(...ds), Math.max(...ds));
  const mo = os.reduce((a, b) => a + b, 0) / os.length, md = ds.reduce((a, b) => a + b, 0) / ds.length;
  const X = (v) => (100 * (v - ox0)) / (ox1 - ox0);
  const Y = (v) => (100 * (v - dy0)) / (dy1 - dy0);
  const f = (v) => (x.axis.fmt === "signed" ? (v >= 0 ? "+" : "") + v.toFixed(2) : v.toFixed(1));
  const size = pts.length > 80 ? 16 : 22;
  const dots = pts.map((p) => `<span class="tl-dot ${p.team && p.team.mine ? "mine" : ""}" style="left:${X(p.o)}%;top:${Y(p.d)}%" title="${esc((p.team ? p.team.name : "") + " · " + p.record + " · offense " + f(p.o) + ", defense " + f(p.d))}">${p.team ? `<a data-go="teamdata" data-kind="${p.team.ref.kind}" data-id="${p.team.ref.id}">${mark(p.team, size)}</a>` : ""}</span>`).join("");
  const lab = x.axis.label;
  return `<div class="panel tl-wrap"><div class="tl-chart" role="img" aria-label="${esc(lab)}: offense against defense"><i class="tl-mx" style="left:${X(mo)}%"></i><i class="tl-my" style="top:${Y(md)}%"></i><span class="tl-q q1">Good offense, good defense</span><span class="tl-q q2">Good defense</span><span class="tl-q q3">Good offense</span><span class="tl-q q4">Struggling</span>${dots}</div><div class="tl-axes"><span data-h8="${esc(x.axis.key)}" tabindex="0">Offense: ${esc(lab)} →</span><span>Defense: ${esc(lab)} allowed, less is higher ↑</span></div></div>`;
}

function tlPower(x) {
  const p = x.power;
  if (!p || !p.rows.length) return '<div class="empty">No ratings yet.</div>';
  if (p.closing) return table("tlpower", [{ k: "rank", t: "#" }, { k: "team", t: "Team", l: true, f: (r) => teamChip(r.team, 18), v: (r) => (r.team ? r.team.name : "") }, { k: "now", t: "Power", h8: "PowerRtg", v: (r) => r.now.v, f: (r) => `<b>${esc(r.now.overall)}</b>` }, { k: "off", t: "Off", h8: "PowerOff", v: (r) => parseFloat(r.now.off), f: (r) => esc(r.now.off) }, { k: "def", t: "Def", h8: "PowerDef", v: (r) => parseFloat(r.now.def), f: (r) => esc(r.now.def) }], p.rows, { sort: null }) + '<p class="muted small">Closing ratings: the season\'s scores alone.</p>';
  const pc = (k) => (r) => parseFloat((r.odds || {})[k]);
  const odd = (k, t, h8) => ({ k, t, h8, v: pc(k), f: (r) => esc(r.odds ? r.odds[k] : "") });
  const cols = [
    { k: "rank", t: "#" },
    { k: "team", t: "Team", l: true, f: (r) => teamChip(r.team, 18), v: (r) => (r.team ? r.team.name : "") },
    { k: "record", t: "Record", l: true, f: (r) => esc(r.record || "0-0") },
    { k: "now", t: "Power", h8: "PowerRtg", v: (r) => r.now.v, f: (r) => `<b>${esc(r.now.overall)}</b>` },
    { k: "off", t: "Off", h8: "PowerOff", v: (r) => parseFloat(r.now.off), f: (r) => esc(r.now.off) },
    { k: "def", t: "Def", h8: "PowerDef", v: (r) => parseFloat(r.now.def), f: (r) => esc(r.now.def) },
    { k: "pre", t: "Preseason", h8: "Projection", v: (r) => (r.preseason ? r.preseason.v : null), f: (r) => esc(r.preseason ? r.preseason.overall : "") },
    { k: "wins", t: "Exp. wins", h8: "ExpWins", v: pc("wins"), f: (r) => esc(r.odds ? `${r.odds.wins}-${r.odds.losses}` : "") },
    ...(x.level === "college" ? [odd("bowl", "Bowl", "BowlOdds")] : []),
    ...(x.level !== "hs" ? [odd("league", x.level === "nfl" ? "Division" : "League", "LeagueOdds"), odd("playoff", "Playoff", "PlayoffOdds"), odd("bye", "Bye", "ByeOdds")] : [odd("winning", "Winning", "WinningOdds")]),
  ];
  return table("tlpower", cols, p.rows.map((r) => ({ ...r, mine: r.team && r.team.mine })), { sort: null }) + `<p class="muted small">Ratings from scores, the schedule and home field (${esc(p.home)} points); early in a season the preseason projection counts as three games. The odds play the rest of the season out 2,000 times.</p>`;
}

Object.assign(CORE_MORE, {
  tsunit: (el) => { keep("teamseasons", {}).unit = Number(el.dataset.v); refresh(el); },
  tsper: (el) => { keep("teamseasons", {}).per = el.dataset.v; refresh(el); },
  tllevel: (el) => { const K = keep("teamlead", {}); K.level = el.dataset.v; K.year = null; K.stat = null; refresh(el); },
  tlside: (el) => { keep("teamlead", {}).side = Number(el.dataset.v); refresh(el); },
});
document.addEventListener("change", (ev) => {
  const t = ev.target;
  if (!t.dataset) return;
  if (t.dataset.act === "tdyear") { keep(t.dataset.k, {}).year = Number(t.value); refresh(t); }
  if (t.dataset.act === "tlsel") { const K = keep("teamlead", {}); K[t.dataset.k] = t.dataset.k === "year" ? Number(t.value) : t.value; if (t.dataset.k === "stat") K.side = 0; refresh(t); }
});
