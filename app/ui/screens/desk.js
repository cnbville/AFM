/* Lane NB11 (Data overhaul items 33-41; boards 45, 48, 50, 56 and 57 in
 * Press Box): Your numbers, the next opponent's report, prospects and
 * their cards, draft, recruiting and portal data, coaches judged by their
 * units, and money against results. Public numbers and your staff's reads
 * only, never a hidden rating. Every stat's name explains itself on hover
 * (data-h8). Classes are dk-. */

Object.assign(I, {
  yournum: '<path d="M4 20V10M10 20V4M16 20v-7M22 20H2"/>',
  nextopp: '<circle cx="8" cy="12" r="4"/><circle cx="17" cy="12" r="4"/><path d="M12 12h1"/>',
  prospects: '<circle cx="12" cy="8" r="4"/><path d="M5 21a7 7 0 0 1 14 0M18 4l2-2"/>',
  draftdata: '<path d="M4 4h16v16H4zM4 9h16M9 9v11"/>',
  coachdata: '<path d="M12 3l3 6 6 .9-4.5 4.4 1 6.2L12 17.6 6.5 20.5l1-6.2L3 9.9 9 9z"/>',
  moneydata: '<circle cx="12" cy="12" r="9"/><path d="M15 8.5c-.6-1-1.7-1.5-3-1.5-1.7 0-3 .9-3 2.3 0 3.2 6 1.7 6 4.9 0 1.4-1.3 2.3-3 2.3-1.4 0-2.6-.6-3.2-1.6M12 5v2M12 17v2"/>',
});
{
  const i = NAV.findIndex((n) => n.go === "stathist");
  const items = [
    { go: "yournum", label: "Your numbers", icon: "yournum", sec: "data" },
    { go: "nextopp", label: "Next opponent", icon: "nextopp", sec: "data" },
  ];
  if (i >= 0) NAV.splice(i, 0, ...items); else items.forEach(navAdd);
}
navAdd({ go: "prospects", label: "Prospects", icon: "prospects", sec: "data", after: "history" });
navAdd({ go: "draftdata", label: "Draft and recruiting", icon: "draftdata", sec: "data", after: "prospects" });
navAdd({ go: "coachdata", label: "Coaches", icon: "coachdata", sec: "data", after: "draftdata" });
navAdd({ go: "moneydata", label: "Money", icon: "moneydata", sec: "data", after: "coachdata" });
Object.assign(CRUMBS, { yournum: "Your numbers", nextopp: "Next opponent", prospects: "Prospects", prospect: "Prospect", draftdata: "Draft and recruiting", coachdata: "Coaches", moneydata: "Money" });

const dkN = (v, d = 1) => (v == null || Number.isNaN(Number(v)) ? "–" : Number(v).toFixed(d));
const dkP = (v, d = 0) => (v == null ? "–" : Number(v).toFixed(d) + "%");
const dkH = (key, text) => (key ? `<span data-h8="${esc(key)}" tabindex="0">${esc(text)}</span>` : esc(text));
const dkWho = (p) => (!p ? "" : p.linked !== false && p.id != null ? playerLink(p.id, p.name) : esc(p.name || ""));
const dkTeam = (t) => (t ? teamChip(t, 16) : "–");
const dkTone = (pct) => (pct == null ? "" : pct >= 75 ? "hi" : pct <= 25 ? "lo" : "");
const dkBar = (pct) => (pct == null ? "" : `<span class="dk-bar ${dkTone(pct)}"><i style="width:${Math.max(2, Math.min(100, pct))}%"></i></span>`);
const dkOrd = (n) => (n == null ? "–" : n + (n % 100 > 10 && n % 100 < 14 ? "th" : ["th", "st", "nd", "rd"][n % 10] || "th"));
const dkPick = (r) => `<div class="tbar">${r}<span class="tspace"></span></div>`;
const dkSel = (act, k, list, cur, store) => `<select class="ssel" data-act="${act}" data-k="${k}"${store ? ` data-store="${store}"` : ""}>${list.map(([v, t]) => `<option value="${esc(v)}" ${String(v) === String(cur ?? "") ? "selected" : ""}>${esc(t)}</option>`).join("")}</select>`;
// The data tables: names, teams and words read from the left, numbers
// from the right (the app's tables right-align every cell).
const DK_LEFT = new Set(["Number", "Player", "Group", "Opponent", "School", "State", "Committed", "Team", "Test", "Event", "Where", "Program", "Conference", "Club", "Move", "Why", "From", "To", "Role before", "Draft", "Went", "Outcome", "Best numbers", "How he turned out", "Line", "Coach", "Now", "Where they went", "Money in", "Money out", "Round", "Stars", "Season", "Date", "Tendency", "Contract", "Coach"]);
function dkTable(heads, rows, keys = []) {
  const left = heads.map((h, i) => i === 0 || DK_LEFT.has(String(h)));
  return `<div class="tbl dk-tbl"><table><thead><tr>${heads.map((h, i) => `<th class="${left[i] ? "l" : ""}"${tdH8(keys[i])}>${esc(h)}</th>`).join("")}</tr></thead><tbody>${rows.map((r) => `<tr>${r.map((v, i) => `<td class="${left[i] ? "l" : ""}">${v ?? ""}</td>`).join("")}</tr>`).join("")}</tbody></table></div>`;
}
const DK_POS = ["QB", "RB", "WR", "TE", "OT", "IOL", "EDGE", "DT", "LB", "CB", "S", "K", "P"];
const dkLevel = () => ({ nfl: "nfl", other: "nfl", hs: "hs" }[seatKind()] || "college");

// ---------- Your numbers (item 41, board 45) ----------
screens.yournum = async () => {
  const x = await api("your_numbers", {});
  if (x.error) return `<div class="pad sl dt">${pageHead({ kicker: "Data", title: "Your numbers" })}<div class="empty">${esc(x.error)}</div></div>`;
  title("Your numbers", `${x.team.name} · ${x.year}`);
  setColors(x.team.colors);
  const five = (vals) => `<span class="dk-five">${vals.map((v) => `<i class="${dkTone(v)}" style="height:${v == null ? 2 : Math.max(4, v)}%" title="${v == null ? "no season" : dkOrd(v) + " percentile"}"></i>`).join("")}</span>`;
  const tiles = x.tiles.map((t) => `<div class="dk-tile ${dkTone(t.pct)}"><small>${dkH(t.key, t.label)}</small>
      <b>${esc(t.text || "–")}</b><span class="dk-rk">${t.rank ? `${dkOrd(t.rank)} of ${t.n}` : ""}${t.avg ? ` · middle ${esc(t.avg)}` : ""}</span>
      <span class="dk-track"><i style="left:${t.pct ?? 50}%"></i></span><span class="dk-ends"><em>worst</em>${five(t.five)}<em>best</em></span></div>`).join("");
  const F = x.five;
  const fiveChart = F.years.length ? `<section class="panel dt-panel"><h2>Five seasons <small>percentile among ${esc(x.group)} · ${esc(F.off_label)} and ${esc(F.def_label)}</small></h2>
    <div class="dk-years">${F.years.map((y, i) => `<div><span class="dk-yb"><i class="off" style="height:${F.off[i] ?? 0}%"></i><i class="def" style="height:${F.def[i] ?? 0}%"></i></span><small>${y}</small><em>${F.off[i] ?? "–"} · ${F.def[i] ?? "–"}</em></div>`).join("")}</div>
    <p class="muted small"><span class="dk-key off"></span> Offense <span class="dk-key def"></span> Defense</p></section>` : "";
  const goDriver = (d) => {
    if (d.go === "situations") return linkTo("situations", {}, "Situations ›");
    if (d.go === "decisions") return linkTo("situations", { tab: "4th downs and 2-point" }, "Decisions ›");
    return linkTo("teamdata", { kind: x.team.ref.kind, id: x.team.ref.id }, "Team data ›");
  };
  const drivers = x.drivers.length ? `<section class="panel dt-panel"><h2>What's driving it <small>each line opens its page</small></h2>
    ${x.drivers.map((d) => `<div class="dk-drv ${d.good ? "up" : "down"}"><b>${dkH(d.key, d.h)}</b><span>${esc(d.t)}</span>${goDriver(d)}</div>`).join("")}</section>` : "";
  const movers = x.movers.length ? `<section class="panel dt-panel"><h2>${dkH("Movers", "This week's biggest movers")} <small>your rank among ${esc(x.group)} before the last seven days' games, and now</small></h2>
    ${dkTable(["Number", "Was", "Now", "Rank"], x.movers.map((m) => [dkH(m.key, m.label), esc(m.was), `<b>${esc(m.now)}</b>`, `<span class="${m.up ? "td-up" : "td-dn"}">${dkOrd(m.from)} → ${dkOrd(m.to)}</span>`]))}</section>` : "";
  const alerts = x.alerts.length ? `<section class="panel dt-panel"><h2>Your players <small>who stands out and who worries, by the percentile among the level's qualifiers</small></h2>
    ${dkTable(["Player", "Group", "Number", "Value", "Percentile"], x.alerts.map((a) => [dkWho(a), esc(a.group), dkH(a.cell.key, a.cell.label || ""), esc(String(a.cell.value ?? "–")), `<span class="dk-alert ${a.up ? "up" : "down"}">${dkOrd(a.cell.pct)}</span> ${dkBar(a.cell.pct)}`]))}</section>` : "";
  const goto = gotoLine([["Next opponent", "nextopp"], ["Team data", "teamdata", x.team.ref], ["Situations", "situations"], ["Player analytics", "analytics"]]);
  return `<div class="pad sl dt dk">${pageHead({ kicker: `Data · ${esc(x.group)} · the ${x.year} season`, title: "Your numbers", sub: `Ranks among the ${x.n} teams in ${esc(x.group)}.`, goto })}
    <section class="panel warm dk-verdict"><p>${esc(x.verdict)}</p></section>
    <div class="dk-tiles">${tiles}</div>${fiveChart}<div class="dk-grid">${drivers}${movers}</div>${alerts}${dtSource(x.source)}</div>`;
};
GOTO.yournum = () => [["Next opponent", "nextopp"], ["Situations", "situations"], ["Player analytics", "analytics"]];

// ---------- Next opponent (item 33) ----------
screens.nextopp = async (a) => {
  const args = a && a.kind ? { kind: a.kind, id: Number(a.id) } : {};
  const x = await api("next_opponent", args);
  if (x.error && !x.opp) return `<div class="pad sl dt">${pageHead({ kicker: "Data", title: "Next opponent" })}<div class="empty">${esc(x.error)}</div></div>`;
  const o = x.opp;
  title("Next opponent", o.name);
  setColors(o.colors);
  const g = x.game;
  const when = g ? `${g.home ? "vs" : "at"} · ${esc(g.date || "")}${g.in != null ? (g.in === 0 ? " · today" : ` · in ${g.in} day${g.in === 1 ? "" : "s"}`) : ""}${g.week ? ` · week ${g.week}` : ""}` : "";
  const cell = (c) => (c ? `<b>${esc(c.text)}</b> <small class="muted">${c.rank ? dkOrd(c.rank) : ""}</small> ${dkBar(c.pct)}` : "–");
  const side = (s) => tdPanel(s.name, dkTable(["Number", s.heads[0], s.heads[1], ""], s.rows.map((r) => [dkH(r.key, r.label), cell(r.off), cell(r.def), r.edge === "off" ? `<span class="dk-edge up">${esc(s.heads[0])}</span>` : r.edge === "def" ? `<span class="dk-edge down">${esc(s.heads[1])}</span>` : ""])));
  const sides = (x.sides || []).map(side).join("");
  const team = (x.team_side || []).length ? tdPanel("Their whole team", dkTable(["Number", "Value", "Rank"], x.team_side.map((r) => [dkH(r.key, r.label), `<b>${esc(r.v.text)}</b>`, r.v.rank ? `${dkOrd(r.v.rank)} of ${r.v.n}` : "–"]))) : "";
  const tilt = (x.matchups || []).length ? `<section class="panel dt-panel"><h2>Matchups that tilt <small>each group's mean percentile on its numbers, yours against theirs</small></h2>
    ${x.matchups.map((m) => `<div class="dk-tilt ${m.way}"><b>${esc(m.label)}</b><span class="dk-vs"><span>${m.a}</span>${dkBar(m.a)}<em>vs</em>${dkBar(m.b)}<span>${m.b}</span></span><small>${esc(m.text)}</small></div>`).join("")}</section>` : "";
  const best = (x.players || []).length ? `<section class="panel dt-panel"><h2>Their best men by the numbers <small>two a group, by mean percentile</small></h2>
    ${dkTable(["Player", "Group", "Mean", "Best numbers"], x.players.flatMap((gp) => gp.rows.map((r) => [dkWho(r), esc(gp.group), `${dkOrd(r.mean)}`, (r.best || []).map((b) => `${dkH(b.key, b.label || "")} ${esc(String(b.value ?? ""))} <small class="muted">${dkOrd(b.pct)}</small>`).join(" · ")])))}</section>` : "";
  const tend = x.tendencies ? tdTendencies({ tendencies: x.tendencies, depth: x.depth, level: x.level }) : "";
  const last = (x.last || []).length ? tdPanel("Their last games", dkTable(["Date", "Opponent", "Score"], x.last.map((r) => [esc(r.date || `week ${r.week}`), `${r.home ? "vs" : "at"} ${dkTeam(r.opp)}`, `<b class="${r.score[0] > r.score[1] ? "td-up" : "td-dn"}">${r.score[0] > r.score[1] ? "W" : r.score[0] < r.score[1] ? "L" : "T"} ${r.score[0]}-${r.score[1]}</b>`]))) : "";
  const goto = gotoLine([["Game day", "gameday"], ["Their team data", "teamdata", o.ref], ["Your numbers", "yournum"], ["Situations", "situations"]]);
  return `<div class="pad sl dt dk">${pageHead({ kicker: `Data · advance scouting${x.group ? " · " + esc(x.group) : ""}`, title: `${mark(o, 30)} ${esc(o.name)}`, sub: `${when}${when ? " · " : ""}their ${x.year} numbers beside yours`, goto })}
    ${x.error ? `<div class="empty">${esc(x.error)}</div>` : ""}${sides}<div class="dk-grid">${tilt}${team}</div>${best}${tend ? `<h3 class="dt-h3">Their calls</h3>${tend}` : ""}${last}${dtSource(x.source)}</div>`;
};
GOTO.nextopp = () => [["Game day", "gameday"], ["Your numbers", "yournum"]];
// The game day hub links its opponent's numbers (section 9's advance scouting).
{
  const base = screens.gameday;
  if (base) {
    screens.gameday = async (a) => {
      const html = await base(a);
      const line = `<div class="dk-gdlink">${gotoLine([["Opponent numbers", "nextopp"], ["Your numbers", "yournum"]])}</div>`;
      return typeof html === "string" && html.includes('<div class="gd-hero">') ? html.replace('<div class="gd-hero">', `<div class="gd-hero">${line}`) : html;
    };
  }
}

// ---------- Prospects and the prospect card (items 34 and 35, board 56) ----------
const PR_TABS = ["Draft prospects", "High school class"];
screens.prospects = async (a) => {
  const K = keep("prospects", {});
  const tab = tabOf("prtab", PR_TABS, a);
  const hs = tab === PR_TABS[1];
  const x = await api("prospects", { level: hs ? "hs" : "college", pos: K.pos || undefined, class: hs && K.class ? Number(K.class) : undefined, state: hs && K.state ? K.state : undefined });
  title("Prospects", hs ? `Class of ${x.class}` : `${x.draft} draft`);
  const pos = dkSel("prsel", "pos", [["", "Every position"], ...DK_POS.map((p) => [p, p])], K.pos || "");
  let bar, body;
  if (hs) {
    bar = dkPick(`${dkSel("prsel", "class", x.classes.map((c) => [c, `Class of ${c}`]), x.class)}${pos}<input class="ssel dk-state" data-act="prstate" placeholder="State, e.g. TX" value="${esc(K.state || "")}" maxlength="2">`);
    body = x.rows.length ? dkTable(["Rank", "Player", "Pos", "School", "State", "Stars", "Rating", "Pos rank", "State rank", "Offers", "Committed", "Games"],
      x.rows.map((r) => [r.rank.rank, linkTo("prospect", { id: r.id }, esc(r.name)), esc(r.pos), dkTeam(r.school), esc(r.state || ""), stars(r.rank.stars), esc(r.rank.rating), r.rank.pos_rank, r.rank.state_rank, r.offers, r.commit ? dkTeam(r.commit) : "", r.games]),
      ["", "", "", "", "", "", "ClassScore", "", "", "", "", "Games"]) : `<div class="empty">The services haven't ranked this class yet.</div>`;
  } else {
    bar = dkPick(pos);
    body = x.rows.length ? dkTable(["Public", "Ours", "Player", "Pos", "School", "Class", "Age on draft day", "Games", "Dominator", "Best", "Breakout age"],
      x.rows.map((r) => [r.public ?? "–", r.ours ?? "–", `${linkTo("prospect", { id: r.id }, esc(r.name))}${r.declared ? ' <span class="tag soft">Declared</span>' : ""}`, esc(r.pos), dkTeam(r.team), esc(r.class), esc(r.age), r.games, esc(r.dominator || "–"), esc(r.best || "–"), esc(r.breakout || "–")]),
      ["PublicRank", "", "", "", "", "", "", "Games", "Dominator", "Dominator", "BreakoutAge"]) : `<div class="empty">No draft prospects on the public board yet.</div>`;
  }
  const goto = gotoLine([["Draft and recruiting", "draftdata"], ["Draft board", "draftboard"], ["Player analytics", "analytics"]]);
  return `<div class="pad sl dt dk">${pageHead({ kicker: "Data · prospects", title: "Prospects", sub: hs ? "A high school class in the services' order, with the numbers recruiting analysts keep." : `The ${x.draft} draft's prospects by the public board, with their production markers.`, goto })}${tabs("prtab", PR_TABS, tab)}${bar}<section class="panel dt-panel">${body}</section>
    ${dtSource(hs ? "Rankings as the services publish them; game lines from the box scores schools report." : "Production from the stat book; shares against each team's own numbers that season.")}</div>`;
};
GOTO.prospects = () => [["Draft and recruiting", "draftdata"], ["Player analytics", "analytics"]];
document.addEventListener("change", (ev) => {
  const t = ev.target;
  if (!t.dataset) return;
  if (t.dataset.act === "prsel") { keep("prospects", {})[t.dataset.k] = t.value || null; refresh(t); }
  if (t.dataset.act === "prstate") { keep("prospects", {}).state = t.value.trim().toUpperCase() || null; refresh(t); }
  if (t.dataset.act === "dksel") { keep(t.dataset.store, {})[t.dataset.k] = t.value === "" ? null : t.value; refresh(t); }
});

screens.prospect = async (a) => {
  const x = await api("prospect_card", { id: Number(a.id) });
  if (x.error) return `<div class="pad"><div class="empty">${esc(x.error)}</div></div>`;
  title(x.name, `${x.pos} · ${x.class}`);
  if (x.team && x.team.colors) setColors(x.team.colors);
  const read = x.read && x.read.now ? `<div><small data-h8="StaffRange">Your staff's read</small><b>${x.read.now[0]}–${x.read.now[1]}</b><small>${esc(x.read.role || "")}${x.read.looks ? ` · ${x.read.looks} looks` : ""}</small></div>` : `<div><small data-h8="StaffRange">Your staff's read</small><b>–</b><small>${esc((x.read && x.read.role) || "Not seen")}</small></div>`;
  const head = pageHead({
    kicker: `Data · ${x.kind === "hs" ? "high school prospect" : "draft prospect"}`,
    title: `${playerLink(x.id, x.name)}`,
    sub: `${esc(x.pos)} · ${x.team ? teamA(x.team) : ""} · ${esc(x.class)} · ${esc(x.hometown || "")} · listed ${esc(x.listed.height)}, ${x.listed.weight} lb${x.stars ? " · " + stars(x.stars) : ""}`,
    goto: gotoLine([["Player page", "player", { id: x.id }], ["Prospects", "prospects"], ["Scouting file", "scout", { id: x.id }]]),
  });
  const body = x.kind === "hs" ? prHs(x, read) : await prCollege(x, read);
  return `<div class="pad sl dt dk">${head}${body}</div>`;
};

async function prCollege(x, read) {
  const D = x.draft;
  const tiles = `<div class="st-tiles dk-st">${read}
    <div><small data-h8="PublicRank">Public board</small><b>${D.public ?? "–"}</b><small>${D.declared ? "declared" : "not declared"} for ${D.year}</small></div>
    <div><small>Your board</small><b>${D.ours ?? "–"}</b><small>age ${esc(D.age)} on draft day</small></div>
    <div><small data-h8="BreakoutAge">Breakout age</small><b>${x.breakout ? esc(x.breakout.age) : "–"}</b><small>${x.breakout ? `in ${x.breakout.year}` : "no breakout season yet"}</small></div>
    <div><small data-h8="AgeAdjDom">Best age-adjusted dominator</small><b>${esc(x.best_age_adjusted || "–")}</b><small>&nbsp;</small></div></div>`;
  const heads = (x.seasons[0] && x.seasons[0].line.map((c) => c[0])) || [];
  const seasons = x.seasons.length ? tdPanel("Production by season", dkTable(["Season", "Team", "Age", ...heads.map(String), "Dominator", "Age-adjusted"],
    x.seasons.map((s) => [s.year, dkTeam(s.team), esc(s.age), ...s.line.map((c) => esc(String(c[1]))), esc(s.dominator || "–"), esc(s.age_adjusted || "–")]),
    ["", "", "", ...heads.map(() => ""), "Dominator", "AgeAdjDom"])) : "";
  const lastM = x.seasons.length ? x.seasons[x.seasons.length - 1].markers : [];
  const markers = lastM.length ? tdPanel("College-to-NFL markers at his spot", `<ul class="dt-plain">${lastM.map((m) => `<li>${dkH(m.key, m.label)}<b>${esc(m.value)}</b></li>`).join("")}</ul>`) : "";
  const metrics = Array.isArray(x.metrics) && x.metrics.length ? tdPanel(`His analytics, ${x.metrics_year}`, dkTable(["Number", "Value", "Middle", "Percentile"], x.metrics.map((m) => [dkH(m.key, m.label), `<b>${esc(m.value || "–")}</b>`, esc(m.mid || ""), m.pct == null ? "–" : `${dkOrd(m.pct)} ${dkBar(m.pct)}`]))) : "";
  const tests = x.tests.map((t) => tdPanel(`${t.where}, ${t.date}`, dkTable(["Test", "Result", "Percentile at his spot"], t.rows.map((r) => [dkH("CombinePct", r.label), `<b>${esc(String(r.value))}</b>`, r.pct == null ? "–" : `${dkOrd(r.pct)} ${dkBar(r.pct)}`])))).join("");
  const S = x.size;
  const size = tdPanel(`Size ${S.measured ? "as measured" : "as listed"}`, `<ul class="dt-plain"><li>${dkH("SizePct", "Height")}<b>${esc(S.height)} <small class="muted">${dkOrd(S.height_pct)}</small></b></li><li>${dkH("SizePct", "Weight")}<b>${S.weight} lb <small class="muted">${dkOrd(S.weight_pct)}</small></b></li></ul><p class="muted small">Against real combine invitees at his position, 2000-2025.</p>`);
  // The seasons most like his: the career comps when the game has them,
  // else his card's closest seasons.
  let comps = "";
  try {
    const c = await api("career_comps", { id: x.id, mode: "last", seasons: 2, drafted: true, n: 8 });
    if (c && Array.isArray(c.comps) && c.comps.length) {
      comps = tdPanel("Careers like his", dkTable(["Player", "Seasons", "Match", "How he turned out"], c.comps.map((p) => [dkWho(p), esc(p.years || ""), dkP(100 * (p.match || 0)), esc((p.outcome && p.outcome.text) || "")])));
    }
  } catch (e) { /* not in this build */ }
  if (!comps && (x.like || []).length) {
    comps = tdPanel("Seasons like his", dkTable(["Player", "Season", "Team", "Closeness", "Drafted", "NFL games", "Now"], x.like.map((l) => {
      const o = l.outcome || {};
      const d = o.drafted ? `${o.drafted.year} · round ${o.drafted.round}, pick ${o.drafted.pick}` : "undrafted";
      return [dkWho(l), l.year, dkTeam(l.team), `${l.close}%`, esc(d), o.nfl_games ?? "–", esc(o.now || "")];
    })));
  }
  // His high school years, kept on the card (an early enrollee has no
  // college season yet).
  const hs = x.high_school ? `<h3 class="dt-h3">High school</h3>${prHs({ ...x.high_school, pos: x.pos }, "")}` : "";
  return `${tiles}${seasons}<div class="dk-grid">${markers}${size}</div>${metrics}${tests}${comps}${hs}`;
}

function prHs(x, read) {
  const last = x.ranks.length ? x.ranks[x.ranks.length - 1] : null;
  const tiles = `<div class="st-tiles dk-st">${read}
    <div><small>National rank</small><b>${last ? last.rank : "–"}</b><small>${last ? `${esc(x.pos)} ${last.pos_rank} · state ${last.state_rank}` : "not ranked"}</small></div>
    <div><small data-h8="ClassScore">Composite</small><b>${last ? esc(last.rating) : "–"}</b><small>${last ? stars(last.stars) : ""}</small></div>
    <div><small>Offers</small><b>${x.offers.length}</b><small>${x.commit ? `committed to ${esc(x.commit.team.name)}` : "uncommitted"}</small></div></div>`;
  const ranks = x.ranks.length ? tdPanel("Stars over time", `<div class="dk-ranks">${x.ranks.map((r) => `<div><small>${esc(r.date)}</small>${stars(r.stars)}<b>${r.rank}</b><em>${esc(r.rating)}</em></div>`).join("")}</div>`) : "";
  const keys = (x.lines[0] && x.lines[0].cells.map((c) => c.label)) || [];
  const lines = x.lines.length ? tdPanel("Season lines", dkTable(["Season", "School", "Games", ...keys], x.lines.map((l) => [l.year, dkTeam(l.school), l.games, ...l.cells.map((c) => `<b>${dkN(c.value, Number.isInteger(c.value) ? 0 : 1)}</b> <small class="muted" title="state · class">${c.state_pct == null ? "" : dkOrd(c.state_pct)} · ${c.class_pct == null ? "" : dkOrd(c.class_pct)}</small>`)])) + `<p class="muted small">${dkH("StateClassPct", "Percentiles")}: among his position in his state that year, then among his class nationwide.</p>`) : "";
  const games = x.games.length ? tdPanel("Game log", dkTable(["Date", "Opponent", "Result", "Line"], x.games.map((g) => [esc(g.date), `${esc(g.site)} ${dkTeam(g.opp)}`, `<b class="${g.won ? "td-up" : "td-dn"}">${g.won ? "W" : "L"} ${g.score[0]}-${g.score[1]}</b>`, g.played ? g.line.map((c) => `${esc(c[0])} ${esc(String(c[1]))}`).join(" · ") : '<span class="muted">did not play</span>']))) : "";
  const times = x.times.length ? tdPanel("Testing", dkTable(["Event", "Time", "Timed", "Where", "Date", "Weight", "Percentile at his spot"], x.times.map((t) => [dkH("CampTime", t.event), `<b>${esc(t.secs)}</b>`, esc(t.timed), esc(t.at), esc(t.date), t.lb ? `${t.lb} lb` : "", t.pct == null ? "–" : `${dkOrd(t.pct)} ${dkBar(t.pct)}`]))) : "";
  const offers = x.offers.length ? tdPanel("Offers", `<ul class="dt-plain">${x.offers.map((o) => `<li>${dkTeam(o.team)}<b>${esc(o.date || "")}</b></li>`).join("")}</ul>`) : "";
  const visits = x.visits.length ? tdPanel("Visits and contacts", `<ul class="dt-plain">${x.visits.map((v) => `<li>${dkTeam(v.team)}<b>${esc(v.kind)} · ${esc(v.date)}</b></li>`).join("")}</ul>`) : "";
  return `${tiles}${ranks}${lines}${games}${times}<div class="dk-grid">${offers}${visits}</div>`;
}

// ---------- Draft, recruiting and the portal (items 36-38, boards 48 and 57) ----------
const DD_TABS = ["Draft", "Recruiting", "Portal"];
const DD_DRAFT = [["rounds", "By round"], ["positions", "By position"], ["conferences", "By conference"], ["schools", "By school"], ["clubs", "By club"]];
const DD_REC = [["classes", "Class rankings"], ["stars", "Stars and outcomes"], ["develop", "Who develops"], ["pipelines", "State pipelines"]];
screens.draftdata = async (a) => {
  const K = keep("draftdata", {});
  const tab = tabOf("ddtab", DD_TABS, a);
  let x, body, bar = "";
  if (tab === "Draft") {
    x = await api("draft_data", { tab: K.dtab || "rounds", from: K.from ? Number(K.from) : undefined, to: K.to ? Number(K.to) : undefined });
    if (x.empty) body = `<div class="empty">${esc(x.empty)}</div>`;
    else {
      bar = dkPick(dtSeg("dddtab", DD_DRAFT, x.tab));
      body = ddDraft(x);
    }
  } else if (tab === "Recruiting") {
    x = await api("recruit_data", { tab: K.rtab || "classes", division: K.division || undefined, year: K.year ? Number(K.year) : undefined });
    bar = dkPick(`${dtSeg("ddrtab", DD_REC, x.tab)}${dkSel("dksel", "division", (x.divisions || []).map((d) => [d, d]), x.division, "draftdata")}${(x.years || []).length ? dkSel("dksel", "year", x.years.slice().reverse().map((y) => [y, `${y} class`]), x.year, "draftdata") : ""}`);
    body = x.empty ? `<div class="empty">${esc(x.empty)}</div>` : ddRecruit(x);
  } else {
    x = await api("portal_data", { division: K.division || undefined, year: K.pyear ? Number(K.pyear) : undefined });
    bar = dkPick(dkSel("dksel", "division", (x.divisions || []).map((d) => [d, d]), x.division, "draftdata"));
    body = x.empty ? `<div class="empty">${esc(x.empty)}</div>` : ddPortal(x);
  }
  title("Draft and recruiting", tab);
  const goto = gotoLine([["Prospects", "prospects"], ["Coaches", "coachdata"], ["Draft board", "draftboard"]]);
  return `<div class="pad sl dt dk">${pageHead({ kicker: "Data · draft, recruiting and the portal", title: "Draft and recruiting", sub: "How picks, classes and transfers turned out, from the classes file the world keeps.", goto })}${tabs("ddtab", DD_TABS, tab)}${bar}${body}</div>`;
};
GOTO.draftdata = () => [["Prospects", "prospects"], ["Coaches", "coachdata"]];
Object.assign(CORE_MORE, {
  dddtab: (el) => { keep("draftdata", {}).dtab = el.dataset.v; refresh(el); },
  ddrtab: (el) => { keep("draftdata", {}).rtab = el.dataset.v; refresh(el); },
  cdrole: (el) => { keep("coachdata", {}).role = el.dataset.v; refresh(el); },
  cdwin: (el) => { keep("coachdata", {}).window = el.dataset.v; refresh(el); },
  cdlevel: (el) => { keep("coachdata", {}).level = el.dataset.v; refresh(el); },
  mdlevel: (el) => { const K = keep("moneydata", {}); K.kind = null; K.id = null; K.year = null; refresh(el); },
});

function ddDraft(x) {
  const label = (r) => {
    if (x.tab === "rounds") return `Round ${r.label}`;
    if (r.label && typeof r.label === "object") return dkTeam(r.label);
    return esc(String(r.label));
  };
  const head = ["", "Picks", "Hit rate", "4+ seasons", "Pro Bowl", "Games", "Starts a pick", "Value over slot a pick"];
  const keys = ["", "", "HitRate", "", "ProBowlRate", "Games", "StartsPerSeason", "ValueOverSlot"];
  const rows = x.rows.map((r) => [label(r), r.picks, dkP(r.hit), dkP(r.four), dkP(r.pro_bowl), dkN(r.games, 0), dkN(r.starts, 1), `<b class="${(r.over_pick || 0) > 0.005 ? "td-up" : (r.over_pick || 0) < -0.005 ? "td-dn" : ""}">${dkN(r.over_pick, 2)}</b>`]);
  const real = x.tab === "rounds" ? tdPanel("The real NFL, 1980-2015 drafts", dkTable(["Round", "4+ seasons", "Pro Bowl", "Games", "Seasons started"], x.rows.filter((r) => r.real).map((r) => [`Round ${r.label}`, dkP(r.real.four), dkP(r.real.pro_bowl), dkN(r.real.games, 0), dkN(r.real.seasons_started, 1)]))) : "";
  const yours = x.yours ? tdPanel(`${x.mine ? esc(x.mine.name) : "Your club"} by round`, dkTable(["Round", "Picks", "Hit rate", "Starts a pick", "Value over slot a pick"], x.yours.map((r) => [`Round ${r.label}`, r.picks, dkP(r.hit), dkN(r.starts, 1), dkN(r.over_pick, 2)]))) : "";
  const pk = (list) => !list.length ? `<div class="empty">None yet: a class needs ${x.mature} seasons behind it.</div>` : dkTable(["Player", "Draft", "Pos", "Club", "School", "Games", "Starts", "Pro Bowls", "Over slot"], list.map((p) => [dkWho(p.player), `${p.year} · ${p.round}-${p.pick}`, esc(p.pos || ""), dkTeam(p.club), dkTeam(p.school), p.games, p.starts, p.pro_bowls, dkN(p.over, 1)]));
  const B = x.board;
  const board = B ? tdPanel("Your board against the draft", `<p class="muted small">${dkH("BoardRecord", "Board record")}: yours ${B.ours == null ? "–" : dkN(B.ours, 2)}, the public's ${B.public == null ? "–" : dkN(B.public, 2)}, over ${B.judged} graded players.</p>${(B.above || []).length ? dkTable(["Player", "Draft", "Pos", "Your rank", "Went", "Outcome"], B.above.map((r) => [dkWho(r.player), r.year, esc(r.pos), r.ours, r.pick ?? "–", r.outcome ?? "–"])) : ""}`) : "";
  const slots = tdPanel("Starts a season by pick", dkTable(["Picks", "Starts a season"], x.slots.map((s) => [esc(s.label), dkN(s.rate, 2)])));
  return `${tdPanel(`Drafts ${x.from}-${x.to}`, dkTable(head, rows, keys) + `<p class="muted small">Counted through ${x.through}. A hit started ${x.hit_starts} games or more, for drafts ${x.mature} years old or more; Pro Bowls and games for drafts ${x.long} years old or more.</p>`)}
    <div class="dk-grid">${real}${yours}${slots}</div>${board}<div class="dk-grid">${tdPanel("Steals", pk(x.steals))}${tdPanel("Busts", pk(x.busts))}</div>`;
}

function ddRecruit(x) {
  if (x.tab === "stars") {
    const y = (x.yours || []).length ? tdPanel("Your program", dkTable(["Stars", "Signees", "Drafted"], x.yours.map((r) => [stars(r.stars) || "Unrated", r.signees, dkP(r.drafted)]))) : "";
    return `${tdPanel("Stars against how they turned out", dkTable(["Stars", "Signees", "Drafted", "1st round", "Started 12+", "Transferred", "Moved down"], x.stars.map((r) => [stars(r.stars) || "Unrated", r.signees, dkP(r.drafted, 1), dkP(r.first_round, 1), dkP(r.started), dkP(r.transferred), dkP(r.down)])) + `<p class="muted small">Classes that have finished college, counted through ${x.through}.</p>`)}${y}`;
  }
  if (x.tab === "develop") {
    const row = (r) => [dkTeam(r.team), esc(r.conference || ""), r.signees, dkN(r.picks, 0), dkN(r.expected, 1), `<b class="${r.over >= 0 ? "td-up" : "td-dn"}">${dkN(r.over, 1)}</b>`];
    const head = ["Program", "Conference", "Signees", "Picks", "Expected", "Over"];
    const keys = ["", "", "", "", "", "DevelopOver"];
    const y = x.yours ? `<p class="muted small">Your program: ${dkOrd(x.yours.rank)} of ${x.yours.of}.</p>` : "";
    return `<div class="dk-grid">${tdPanel("Who develops", dkTable(head, x.develop.map(row), keys) + y)}${tdPanel("Who doesn't", dkTable(head, x.least.map(row), keys))}</div>`;
  }
  if (x.tab === "pipelines") {
    return tdPanel(`State pipelines, ${x.span[0] + 1}-${x.span[1]} classes`, dkTable(["State", "Signees", "Stayed home", "Where they went", "To you"], x.pipelines.map((r) => [esc(r.state), r.signees, dkP(r.stay), r.top.map((t) => `${dkTeam(t.team)} <small class="muted">${t.n}</small>`).join(" · "), r.yours ?? ""]), ["", "", "PipelineStay", "", ""]));
  }
  const members = (x.members || []).length ? tdPanel("Your class", dkTable(["Player", "Pos", "State", "Stars", "Rating"], x.members.map((m) => [dkWho(m.player), esc(m.pos || ""), esc(m.state || ""), stars(m.stars), m.rating == null ? "–" : dkN(m.rating, 4)]))) : "";
  return `${tdPanel(`Class rankings, ${x.year}`, dkTable(["", "Program", "Conference", "Signees", "5★", "4★", "3★", "Avg rating", "Score", "Blue-chip share"], x.classes.slice(0, 100).map((r) => [r.rank, dkTeam(r.team), esc(r.conference || ""), r.signees, r.five, r.four, r.three, dkN(r.avg, 4), `<b>${dkN(r.score, 1)}</b>`, dkP(r.blue)]), ["", "", "", "", "", "", "", "", "ClassScore", "BlueChipShare"]))}${members}`;
}

function ddPortal(x) {
  const progs = tdPanel(`The portal by program, ${x.division}`, dkTable(["Program", "In", "Out", "Net", "Played after", "Started 6+"], x.programs.slice(0, 80).map((r) => [dkTeam(r.team), r.in, r.out, `<b class="${r.net >= 0 ? "td-up" : "td-dn"}">${r.net > 0 ? "+" : ""}${r.net}</b>`, r.played, r.started]), ["", "", "", "PortalNet", "PortalPlayed", ""]) + `<p class="muted small">${x.entries} entries from ${esc(x.division)}, counted through ${x.through}.</p>`);
  const dirs = tdPanel("How transfers did", dkTable(["Move", "Players", "Games the season before", "Played after", "Games a season", "Starts a season", "Starters"], x.dirs.map((d) => [esc(d.dir), d.n, dkN(d.before_games, 1), d.played, dkN(d.games, 1), dkN(d.starts, 1), dkP(d.starters)])));
  const whys = tdPanel("Why they went", dkTable(["Why", "Players", "Found a school", "Started 6+ after"], x.whys.map((w) => [esc(w.why), w.n, dkP(w.landed), dkP(w.started)])));
  const yours = (x.yours || []).length ? tdPanel("Your program's moves", dkTable(["Player", "Pos", "Stars", "", "From", "To", "Why", "Role before", "Games before", "Since: games · starts"], x.yours.map((m) => [dkWho(m.player), esc(m.pos || ""), stars(m.stars), m.in ? '<span class="td-up">In</span>' : '<span class="td-dn">Out</span>', dkTeam(m.from), m.to ? dkTeam(m.to) : "–", esc(m.why), esc(m.role), m.before_games, `${m.after.games} · ${m.after.starts}`]))) : "";
  return `${progs}<div class="dk-grid">${dirs}${whys}</div>${yours}`;
}

// ---------- Coaches (item 39, board 48) ----------
screens.coachdata = async (a) => {
  const K = keep("coachdata", {});
  const level = K.level || dkLevel();
  const x = await api("coach_data", { level, role: K.role || "oc", window: K.window != null ? Number(K.window) : 3, division: K.division || undefined, state: K.state || undefined });
  title("Coaches", `${x.role_name || ""} · ${x.group || ""}`);
  const win = Number(x.window);
  const bar = dkPick(`${dtSeg("cdlevel", [["college", "College"], ["nfl", "NFL"], ["hs", "High school"]], level)}${dtSeg("cdrole", [["oc", "Offensive coordinators"], ["dc", "Defensive coordinators"], ["hc", "Head coaches"]], x.role)}${dtSeg("cdwin", [["1", "This season"], ["3", "3 seasons"], ["0", "Every season"]], String(win))}`);
  const money = (v) => (v == null ? "–" : v >= 1e6 ? `$${(v / 1e6).toFixed(1)}M` : `$${Math.round(v / 1e3)}K`);
  const rows = (x.rows || []).map((r) => [
    `${r.coach != null ? coachLink(r.coach, r.name) : esc(r.name || "")}${r.mine ? ' <span class="tag soft">You</span>' : ""}`,
    dkTeam(r.team), r.since, `<b>${r.this}</b>`, `<b>${r.three}</b>`, `<b>${r.all}</b>`, r.talent ?? "–", r.rank ?? "–",
    r.over == null ? "–" : `<span class="${r.over >= 0 ? "td-up" : "td-dn"}">${r.over > 0 ? "+" : ""}${r.over}</span>`,
    esc(r.tendency || ""), money(r.pay), r.through ?? "",
    `<span class="dk-spark">${(r.by_year || []).slice(-6).map(([y, p]) => `<i class="${dkTone(p)}" style="height:${Math.max(4, p)}%" title="${y}: ${p}"></i>`).join("")}</span>`,
  ]);
  const head = ["Coach", "Team", "Since", "This season", "3 seasons", "Career", "Talent rank", "Unit rank", "Over talent", "Tendency", "Pay", "Through", "By season"];
  const keys = ["", "", "", "CoachPct", "CoachPct", "CoachPct", "TalentRank", "", "OverTalent", "", "", "", ""];
  const body = rows.length ? dkTable(head, rows, keys) : `<div class="empty">${esc(x.note || "No unit seasons counted yet.")}</div>`;
  const goto = gotoLine([["Draft and recruiting", "draftdata"], ["Situations", "situations", { tab: "4th downs and 2-point" }], ["Staff", "staff"]]);
  return `<div class="pad sl dt dk">${pageHead({ kicker: `Data · judging coaches · ${esc(x.group || "")}`, title: "Coaches", sub: `${esc(x.role_name || "")}s ranked by ${esc(x.measure || "")}, as a percentile at the level, through ${x.year ?? ""}.`, goto })}${bar}<section class="panel dt-panel">${body}</section>
    ${dtSource(`Units by season from the people tally; talent is ${x.talent_word || "not kept at this level"}; pay where a contract is public.`)}</div>`;
};
GOTO.coachdata = () => [["Draft and recruiting", "draftdata"], ["Situations", "situations"]];

// ---------- Money (item 40, board 50) ----------
screens.moneydata = async (a) => {
  const K = keep("moneydata", {});
  if (a && a.kind && a.id != null) { K.kind = a.kind; K.id = Number(a.id); }
  const args = { year: K.year ? Number(K.year) : undefined };
  if (K.kind && K.id != null) Object.assign(args, { kind: K.kind, id: K.id });
  const x = await api("money_data", args);
  if (x.error && !x.team) return `<div class="pad"><div class="empty">${esc(x.error)}</div></div>`;
  title("Money", `${x.team.name} · ${x.year}`);
  setColors(x.team.colors);
  const years = (x.years || []).length ? dkSel("dksel", "year", x.years.map((y) => [y, String(y)]), x.year, "moneydata") : "";
  const bar = dkPick(`${years}${K.kind ? `<button class="st-btn glass" data-act="mdlevel" data-v="mine">Back to your team</button>` : ""}`);
  let body = "";
  if (x.error) body = `<div class="empty">${esc(x.error)}</div>`;
  else if (x.level === "hs") body = mdHs(x);
  else body = mdTeam(x);
  const goto = gotoLine([["Contracts", "contracts"], ["Team data", "teamdata", x.team.ref], ["Your numbers", "yournum"]]);
  const sub = x.level === "nfl" ? "The cap by position group against what each group gave back." : x.level === "college" ? "NIL and the school's revenue share by position group, against results." : "The football budget line by line, against the books so far.";
  return `<div class="pad sl dt dk">${pageHead({ kicker: `Data · money · ${esc(x.among || "")}`, title: `${mark(x.team, 30)} ${esc(x.team.name)}`, sub, goto })}${bar}${body}
    ${dtSource(x.level === "nfl" ? "Cap figures from the league office." : x.level === "college" ? "Your collective's books; other programs' money is what their reported deals add up to, estimated." : "From the school's books and the district board's minutes.")}</div>`;
};
GOTO.moneydata = () => [["Contracts", "contracts"], ["Your numbers", "yournum"]];
// A team page links its money and, for another team, its numbers as an opponent.
{
  const core = GOTO.team;
  GOTO.team = (a, t) => {
    const list = core(a, t);
    if (t && t.ref && t.ref.kind !== "other") list.push(["Money", "moneydata", { kind: t.ref.kind, id: t.ref.id }]);
    return list;
  };
}

function mdTeam(x) {
  const tiles = `<div class="st-tiles dk-st">${x.tiles.map((t) => `<div><small>${esc(t.label)}</small><b>${esc(t.value == null ? "–" : String(t.value))}</b><small>${esc(t.sub == null ? "" : String(t.sub))}</small></div>`).join("")}</div>`;
  const college = x.level === "college";
  const head = ["Group", college ? "Paid" : "Cap hit", "By money", ...(college ? ["School share", "Collective"] : []), "Result", "By result", college ? "Biggest deal" : "Biggest cap hit"];
  const keys = ["", college ? "" : "CapHit", "MoneyRank", ...(college ? ["RevenueShare", "CollectiveNil"] : []), "", "ResultRank", ""];
  const rows = x.groups.map((g) => [esc(g.group), `<b>${esc(g.money)}</b>`, g.money_rank ?? "–", ...(college ? [esc(g.pots ? g.pots.school : ""), esc(g.pots ? g.pots.collective : "")] : []),
    g.result ? dkH(g.result_key, g.result) : "–", g.result_rank ?? "–", g.biggest ? `${dkWho(g.biggest)} <small class="muted">${esc(g.biggest.money)}</small>` : ""]);
  const groups = `<section class="panel dt-panel"><h2>${college ? "NIL and revenue share" : "Cap"} by position group <small>ranks of ${x.of} in ${esc(x.among)} · total ${esc(x.total)}</small></h2>${dkTable(head, rows, keys)}</section>`;
  const S = x.scatter;
  let scatter = "";
  if (S && S.dots.length) {
    const xs = S.dots.map((d) => d.money);
    const lo = Math.min(...xs), hi = Math.max(...xs);
    const W = 520, H = 220, pad = 28;
    const px = (v) => pad + ((v - lo) / Math.max(1, hi - lo)) * (W - 2 * pad);
    const py = (s) => H - pad - s * (H - 2 * pad);
    const dots = S.dots.map((d) => `<circle cx="${px(d.money).toFixed(1)}" cy="${py(d.share).toFixed(1)}" r="${d.mine ? 6 : 4}" class="${d.mine ? "mine" : ""}"><title>${esc(d.team ? d.team.name : "")}: ${esc((d.money / 1e6).toFixed(1))}M, ${(100 * d.share).toFixed(0)}% wins</title></circle>`).join("");
    const fm = (v) => `$${(v / 1e6).toFixed(v >= 1e7 ? 0 : 1)}M`;
    scatter = tdPanel(`${college ? "Collective money" : "Player cash"} against wins, ${S.years[0]}-${S.years[S.years.length - 1]}`, `<svg class="dk-scatter" viewBox="0 0 ${W} ${H}" role="img"><line x1="${pad}" y1="${H - pad}" x2="${W - pad}" y2="${H - pad}"/><line x1="${pad}" y1="${pad}" x2="${pad}" y2="${H - pad}"/><text x="${pad}" y="${H - 8}">${fm(lo)}</text><text x="${W - pad}" y="${H - 8}" text-anchor="end">${fm(hi)}</text><text x="4" y="${pad}">100%</text><text x="4" y="${H - pad}">0%</text>${dots}</svg><p class="muted small">Each dot a team: its money a year against its share of games won${S.r != null ? `; correlation ${S.r}` : ""}.</p>`);
  }
  const V = x.value;
  const vt = (list) => dkTable(["Player", "Pos", "Money", "Games", "Starts", "Snaps", V.per], list.map((p) => [dkWho(p), esc(p.pos || ""), esc(p.money), p.games, p.starts, p.snaps, `<b>${esc(p.per)}</b>`]), ["", "", "", "", "", "", "SnapsPerMoney"]);
  const value = V && (V.best.length || V.worst.length) ? `<div class="dk-grid">${tdPanel("Best value", vt(V.best))}${tdPanel("Least back for the money", vt(V.worst))}</div>` : "";
  return `${tiles}${groups}${scatter}${value}`;
}

function mdHs(x) {
  const tiles = `<div class="st-tiles dk-st">${x.tiles.map((t) => `<div><small>${esc(t.label)}</small><b>${esc(String(t.value))}</b><small>${esc(t.sub)}</small></div>`).join("")}</div>`;
  const usd = (v) => (v ? `$${Math.round(v).toLocaleString("en-US")}` : "–");
  const part = (inc) => dkTable([inc ? "Money in" : "Money out", "Budget", "So far", `Rank of ${x.of}`], x.lines.filter((l) => l.income === inc).map((l) => [esc(l.name), usd(l.budget), `<b>${usd(l.books)}</b>`, l.rank ?? "–"]));
  return `${tiles}<div class="dk-grid">${tdPanel("Money in", part(true))}${tdPanel("Money out", part(false))}</div><p class="muted small">Ranked among ${esc(x.among)}'s schools by budget.</p>`;
}
