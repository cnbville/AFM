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
Object.assign(CRUMBS, { stathist: "Stat history", analytics: "Player analytics" });

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
  else if (key === "team") main = x.team_book ? dtTeamBook(x.team_book, x.stats, K) : `<div class="empty">Pick a team from its team page, or take a seat at this level, to see a team's own book.</div>`;
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
  return `<section class="dt-book panel"><header><h2>${esc(l.name)}</h2>${more}</header>${l.min ? `<p class="muted small">${esc(l.min)}</p>` : ""}<ol>${real}${l.rows.map(dtRow).join("") || '<li class="muted">Nobody yet.</li>'}</ol></section>`;
}

function dtLeaders(x) {
  const L = x.leaders;
  const chips = `<div class="seg wrap">${x.stats.map((n, i) => (x.level === "hs" && i === 12 ? "" : `<button data-act="dtstat" data-v="${i}" aria-pressed="${L.stat === i}">${esc(n)}</button>`)).join("")}</div>`;
  const cell = (e) => (e ? `<div class="dt-lead"><b class="dt-v">${esc(e.value)}</b> ${dtWho(e)} <small>${e.team ? teamA(e.team) : ""}${e.pos ? " · " + esc(e.pos) : ""}</small>${dtEra(e.era)}</div>` : "");
  const rows = L.rows.map((r) => `<tr><td>${r.year}</td><td class="l">${cell(r.first)}</td><td class="l">${cell(r.second)}</td></tr>`).join("");
  return `${chips}<div class="tbl"><table><thead><tr><th>Season</th><th class="l">${esc(L.name)}: the leader</th><th class="l">Runner-up</th></tr></thead><tbody>${rows || '<tr><td colspan="3" class="muted">No seasons in the books yet.</td></tr>'}</tbody></table></div>`;
}

function dtTeamBook(b, stats, K) {
  const groups = [["career", "Career"], ["season", "Season"], ["game", "Single game"]];
  const chips = `<div class="seg wrap"><button data-act="dtstat" data-v="" aria-pressed="${K.stat == null}">Every stat</button>${stats.map((n, i) => `<button data-act="dtstat" data-v="${i}" aria-pressed="${String(K.stat) === String(i)}">${esc(n)}</button>`).join("")}</div>`;
  const body = groups.map(([k, n]) => {
    const lists = b.lists.filter((l) => l.kind === k && l.rows.length);
    if (!lists.length) return "";
    return `<h3 class="dt-h3">${n}</h3><div class="dt-books">${lists.map((l) => `<section class="dt-book panel"><header><h2>${esc(l.name.charAt(0).toUpperCase() + l.name.slice(1))}</h2></header><ol>${l.rows.map(dtRow).join("")}</ol></section>`).join("")}</div>`;
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
    return c ? `<li><span class="dt-who"><small>${esc(r.name)}</small>${dtWho(c)}</span><b class="dt-v">${esc(c.value)}</b></li>` : "";
  }).join("");
  return `<section class="panel dt-panel"><h2>${teamA(o.team)}'s records</h2>${rows ? `<ol class="dt-plain">${rows}</ol>` : '<p class="muted small">Nothing in the team\'s book yet.</p>'}<p>${linkTo("stathist", { tab: "team", kind: o.team.ref.kind, id: o.team.ref.id }, "The whole book", "gochip")}</p></section>`;
}

// ---------- Player analytics (items 7 and 8, board 46) ----------
screens.analytics = async (a) => {
  const K = keep("analytics", {});
  for (const k of ["level", "division", "state", "group", "year"]) if (a[k] != null) { K[k] = a[k]; delete a[k]; }
  if (a.card != null) { K.card = Number(a.card); delete a.card; }
  const level = dtLevel(K);
  const args = { level, division: K.division || undefined, state: K.state || undefined, group: K.group || undefined, year: K.year ? Number(K.year) : undefined, min: K.min !== "" && K.min != null ? Number(K.min) : undefined, sort: K.sort != null ? Number(K.sort) : undefined, dir: K.dir || -1 };
  const x = await api("analytics", args);
  title("Player analytics", `${x.group} · ${x.year}`);
  const bar = `<div class="tbar">${dtSeg("dalevel", DT_LEVELS, x.level)}
    ${x.level === "college" ? dtSel("a:division", x.divisions, x.division) : ""}
    ${x.level === "hs" ? dtSel("a:state", Object.keys(STATE_NAMES).map((c) => [c, STATE_NAMES[c]]), x.state) : ""}
    ${dtSel("a:year", x.years, x.year)}
    <label class="dt-min">At least <input type="number" min="0" step="1" value="${x.min}" data-act="damin"> ${esc(x.volume)}</label>
    <span class="tspace"></span><span class="tag ${x.charted ? "gold" : "soft"}">${x.charted ? "Charted" : "Box score"}</span></div>
    <div class="tbar">${dtSeg("dagroup", x.groups.map((g) => [g, g]), x.group)}</div>`;
  const head = `<tr><th>#</th><th class="l">Player</th><th class="l">Pos</th><th class="l">Team</th><th title="Games">G</th><th title="${esc(x.volume)}">Vol</th>${x.columns.map((c, i) => `<th><button class="dt-sort ${x.sort === i ? "on" : ""}" data-act="dasort" data-v="${i}" title="${esc(c.long)} · ${c.better === "high" ? "higher is better" : "lower is better"}">${esc(c.label)}</button></th>`).join("")}<th></th></tr>`;
  const body = x.rows.map((r) => `<tr class="${K.card === r.id ? "on" : ""}"><td>${r.rank}</td><td class="l">${dtWho(r)}</td><td class="l">${esc(r.pos)}</td><td class="l">${r.team ? teamChip(r.team, 16) : ""}</td><td>${r.games}</td><td>${r.vol}</td>${r.cells.map((v, i) => `<td class="dt-pc" style="--p:${r.pct[i] ?? 0}%" title="${r.pct[i] != null ? r.pct[i] + "th percentile" : ""}">${v == null ? "" : esc(v)}</td>`).join("")}<td><button class="tbtn" data-act="dacard" data-id="${r.id}">Card</button></td></tr>`).join("");
  const table = `<div class="tbl dt-at"><table><thead>${head}</thead><tbody>${body || `<tr><td colspan="${7 + x.columns.length}" class="muted">Nobody qualifies yet. Lower the minimum to see more.</td></tr>`}</tbody></table></div>`;
  const card = K.card != null ? await dtCard(K.card, x.level) : "";
  const sub = `${x.total} qualify of ${x.pool} who played. Each number's shade is his percentile among the qualifiers; sorting goes by percentile, best first.`;
  return `<div class="pad sl dt">${pageHead({ kicker: `Data · ${esc(x.division || (x.state ? STATE_NAMES[x.state] || x.state : x.level === "nfl" ? "NFL" : ""))} · ${x.year}`, title: "Player analytics", sub, goto: gotoLine([["Stat history", "stathist"], ["Leaderboards", "leaders"]]) })}${bar}<div class="dt-main ${card ? "" : "full"}"><div class="dt-body">${table}</div>${card ? `<aside class="dt-side">${card}</aside>` : ""}</div>${dtSource(x.source)}</div>`;
};
GOTO.analytics = () => [["Stat history", "stathist"], ["Leaderboards", "leaders"]];

async function dtCard(id, level) {
  let c;
  try { c = await api("analytics_card", { id, level }); } catch (e) { return ""; }
  if (c.error) return `<section class="panel dt-panel"><p class="muted">${esc(c.error)}</p></section>`;
  if (c.empty) return `<section class="panel dt-panel"><h2>${playerLink(c.id, c.name)}</h2><p class="muted">${esc(c.empty)}</p></section>`;
  const bars = c.metrics.map((m) => `<li><span class="dt-ml" title="${esc(m.long)}">${esc(m.label)}</span><i class="dt-bar"><i style="width:${m.pct ?? 0}%"></i></i><b>${m.value == null ? "–" : esc(m.value)}</b><small>${m.pct == null ? "" : m.pct}</small></li>`).join("");
  const like = c.like.map((l) => `<li><span class="dt-who">${dtWho(l)}<small>${l.year} · ${l.team ? teamA(l.team) : ""} · ${esc(l.now)}</small></span><b class="dt-v" title="How close the two seasons read, 100 the same">${l.close}</b></li>`).join("");
  return `<section class="panel dt-panel dt-card"><header><div><div class="st-label">${esc(c.group)} card · ${c.year}${c.charted ? " · charted" : " · box score"}</div><h2>${playerLink(c.id, c.name)}</h2><small>${c.team ? teamA(c.team) : ""} · ${c.games} games · ${c.vol} ${esc(c.volume)}${c.qualified ? "" : " · below the minimum"}</small></div><button class="tbtn" data-act="dacard" data-id="">Close</button></header>
    <ul class="dt-bars">${bars}</ul><p class="muted small">Percentile among ${c.field} qualifiers that season.</p>
    <h3 class="dt-h3">Seasons like his</h3>${like ? `<ol class="dt-plain">${like}</ol>` : '<p class="muted small">No past season reads close yet.</p>'}</section>`;
}

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
  if (a.tab === "records" || a.tab === "nfl_records" || a.tab === "Record book") {
    const lv = a.tab === "nfl_records" ? "nfl" : "college";
    if (a.division) keep("stathist", {}).division = a.division;
    keep("stathist", {}).level = lv;
    return screens.stathist({});
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
  dalevel: (el) => { const K = keep("analytics", {}); K.level = el.dataset.v; K.year = null; K.min = null; K.sort = null; K.card = null; refresh(el); },
  dagroup: (el) => { const K = keep("analytics", {}); K.group = el.dataset.v; K.min = null; K.sort = null; K.card = null; refresh(el); },
  dasort: (el) => { const K = keep("analytics", {}); const v = Number(el.dataset.v); if (K.sort === v) K.dir = (K.dir || -1) * -1; else { K.sort = v; K.dir = -1; } refresh(el); },
  dacard: (el) => { const K = keep("analytics", {}); K.card = el.dataset.id === "" || K.card === Number(el.dataset.id) ? null : Number(el.dataset.id); refresh(el); },
  damode: (el) => { keep("history", {}).amode = el.dataset.v; refresh(el); },
});
document.addEventListener("change", (ev) => {
  const t = ev.target;
  if (!t.dataset) return;
  if (t.dataset.act === "damin") { keep("analytics", {}).min = t.value; refresh(t); return; }
  if (t.dataset.act !== "dsel") return;
  const k = t.dataset.k;
  if (k.startsWith("a:")) {
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
