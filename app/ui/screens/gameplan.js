// The deeper game plan (update 2, lane 5: G1-G23), boards 19-27 in Press
// Box. The Plans page holds the program's decisions: the identity (your
// guidelines for the coordinators, with the standing game rules and the
// call names), the self-scout, the call sheet your coordinators draft each
// week and the installs. The depth chart gets a Packages tab (three deep
// and the line's rotation). Game day's Game plan tab holds the week against
// one opponent and its Report tab the teams like us; hiring, the recruiting
// board and the draft board read fit against your guidelines.
//
// Every rating is your staff's read (a range or words), never a true
// number. Clicks go through CORE_MORE (data-act "dp…"); selects and sliders
// through one change listener (data-dpch). A redraw that only moves the
// picked row reuses the last answers (DP.reuse) instead of asking again.
"use strict";

const DP = { cache: {}, reuse: false, pend: {}, sel: {}, notes: { offense: [], defense: [] }, id: null, rules: null, sheet: null };
const DP_LVL = { hs: "high school", college: "college", nfl: "NFL" };
const DP_FIRM_WORDS = { firm: "firm", lean: "a lean", theirs: "his call" };

async function dpGet(cmd, args = {}) {
  const key = cmd + "|" + JSON.stringify(args);
  if (DP.reuse && DP.cache[key]) return DP.cache[key];
  const x = await api(cmd, args);
  DP.cache[key] = x;
  return x;
}
// Redraw with the answers in hand (a row picked, a slider moved).
function dpLocal(el) {
  DP.reuse = true;
  refresh(el);
}
async function dpDo(cmd, args, el, said) {
  try {
    const r = await api(cmd, args);
    DP.cache = {};
    if (said) toast(said);
    refresh(el);
    return r;
  } catch (e) {
    toast(e.message || String(e));
    return null;
  }
}
const dpPlural = (n, w, ws) => `${n} ${n === 1 ? w : ws || w + "s"}`;
const dpCap = (s) => (s ? String(s)[0].toUpperCase() + String(s).slice(1) : "");
const dpNum = (v) => (typeof v === "number" && isFinite(v) ? v : 0);
function dpSeg(act, items, cur, data = {}, cls = "", dis = false) {
  const da = Object.entries(data).map(([k, v]) => ` data-${k}="${esc(v)}"`).join("");
  return `<span class="dp-seg ${cls}">${items.map(([v, label, own]) => `<button data-act="${act}"${da} data-v="${esc(v)}" aria-pressed="${String(v) === String(cur)}"${dis ? " disabled" : ""}>${esc(label)}${own ? '<i class="own"></i>' : ""}</button>`).join("")}</span>`;
}
function dpHead(team, kicker, name, tabsHtml, actions) {
  return `<div class="dp-head"><span class="dp-crest">${mark(team, 44)}</span><div class="dp-head-copy"><span class="dp-lbl">${kicker}</span><h1 class="dp-h1">${esc(name)}</h1></div>${tabsHtml || ""}<div class="dp-head-act">${actions || ""}</div></div>`;
}
// Bars game by game with a dashed line or two (the identity, the
// self-scout): values in percent.
function dpBars(games, lines, opts = {}) {
  const H = 84;
  const vals = games.map((g) => dpNum(g.v));
  const top = Math.min(100, Math.max(10, ...vals, ...lines.map((l) => dpNum(l.v))) * 1.1);
  const h = (v) => Math.round((dpNum(v) / top) * H);
  const cols = games.map((g) => `<div title="${esc(g.title)}: ${g.v}%"><b>${g.v}</b><i class="${opts.hi && opts.hi(g.v) ? "hi" : ""}" style="height:${Math.max(2, h(g.v))}px"></i></div>`).join("");
  const ls = lines.filter((l) => l.v != null).map((l) => `<span class="tl ${l.own ? "own" : ""}" style="top:${112 - h(l.v)}px"></span><span class="tt ${l.own ? "own" : ""}" style="top:${Math.max(0, 112 - h(l.v) - 14)}px">${esc(l.label)}</span>`).join("");
  return `<div class="dp-bars">${ls}<div class="cols">${cols}</div></div><div class="dp-barx">${games.map((g) => `<span title="${esc(g.title)}">${esc(g.o)}</span>`).join("")}</div>`;
}

// ======================================================================
// Plans: Identity, Self-scout, Call sheet, Playbook, Installs
// ======================================================================
const DP_PLAN_TABS = ["Identity", "Self-scout", "Call sheet", "Playbook", "Installs"];
screens.plans = async (a) => {
  try {
    return await dpPlans(a || {});
  } finally {
    DP.reuse = false;
  }
};
async function dpPlans(a) {
  const tab = tabOf("dpplan", DP_PLAN_TABS, a);
  title("Plans", tab);
  const st = S.status || {};
  const team = st.mine || null;
  if (team) setColors(team.colors);
  let out = { body: "", actions: "", kick: "" };
  if (tab === "Identity") out = await dpIdentityTab();
  else if (tab === "Self-scout") out = await dpScoutTab();
  else if (tab === "Call sheet") out = await dpSheetTab();
  else if (tab === "Installs") out = await dpInstallsTab();
  else {
    const x = await api("plans");
    out = { body: `<div class="dp-book">${bookTabHtml(x)}</div>` };
  }
  const kicker = [team ? esc(team.name) : "", st.season ? esc(st.season) : "", st.today ? esc(st.today) : "", out.kick || ""].filter(Boolean).join(" · ");
  const goto = gotoLine([["Game day", "gameday"], ["Depth chart", "depth"], ["Training", "training"], ["Play designer", "designer"], ["Staff", "people_staff"]]);
  return `<div class="film"><div class="pad dp">${dpHead(team, kicker, "Plans", tabs("dpplan", DP_PLAN_TABS, tab), out.actions)}${goto}<div class="tabbody">${out.body}</div></div></div>`;
}

// ---------- Identity (board 19) ----------
function dpEff(r) {
  const p = DP.pend[r.key] || {};
  const to = "to" in p ? p.to : r.value;
  return { to, firm: p.firm || r.firm || "lean", set: to != null, pend: !!DP.pend[r.key] };
}
function dpExpect(x, r, e) {
  if (r.kind !== "track") return null;
  if (!e.pend) return r.expect;
  if (!e.set) return r.own;
  const k = x.pulls && x.pulls[e.firm] != null ? x.pulls[e.firm] : 0.55;
  return r.own + k * (e.to - r.own);
}
function dpIdRow(x, r, on) {
  const e = dpEff(r);
  const dis = x.head ? "" : " disabled";
  let ctl;
  if (r.kind === "options") {
    const cur = e.set ? e.to : r.own;
    ctl = `<span class="dp-seg ${e.set ? "gold" : ""}">${r.options.map((o) => `<button data-act="dpopt" data-key="${esc(r.key)}" data-v="${esc(o.v)}" aria-pressed="${o.v === cur}" title="${esc(o.fit ? `${o.label}: your staff reads the roster as a ${o.fit.toLowerCase()} fit${o.own ? ". His own." : ""}` : o.own ? `${o.label}: his own` : o.label)}"${dis}>${esc(o.label)}${o.own ? '<i class="own"></i>' : ""}</button>`).join("")}</span>`;
  } else {
    const span = Math.max(1, r.max - r.min);
    const P = (v) => Math.max(0, Math.min(100, ((dpNum(v) - r.min) / span) * 100)).toFixed(1);
    const ex = dpExpect(x, r, e);
    const lo = Math.min(r.own, ex), hi = Math.max(r.own, ex);
    const cur = e.set ? e.to : r.own;
    ctl = `<span class="dp-trkw"><button class="dp-nud" data-act="dpnud" data-key="${esc(r.key)}" data-d="${-r.step}" aria-label="Lower"${dis}>−</button>
      <span class="dp-trk" title="○ his own ${Math.round(r.own)}%${e.set ? ` · ◆ yours ${Math.round(e.to)}%` : ""}${r.season != null ? ` · │ this season ${r.season}%` : ""}"><span class="ln"></span><span class="bd" style="left:${P(lo)}%;width:${(P(hi) - P(lo)).toFixed(1)}%"></span>${r.season != null ? `<span class="sx" style="left:${P(r.season)}%"></span>` : ""}<span class="cx" style="left:${P(r.own)}%"></span>${e.set ? `<span class="tx" style="left:${P(e.to)}%"></span>` : ""}<span class="lo">${esc(r.label_lo)}</span><span class="hi">${esc(r.label_hi)}</span>${x.head ? `<input type="range" min="${r.min}" max="${r.max}" step="${r.step}" value="${Math.round(cur)}" data-dpch="dptrk" data-key="${esc(r.key)}" aria-label="${esc(r.name)}">` : ""}</span>
      <button class="dp-nud" data-act="dpnud" data-key="${esc(r.key)}" data-d="${r.step}" aria-label="Higher"${dis}>+</button></span>`;
  }
  const fit = r.fit ? `<span class="dp-fit"><b class="f${r.fit.n - 1}">${esc(r.fit.word)}</b><span class="dp-fitbar">${[1, 2, 3, 4].map((j) => `<i class="${j <= r.fit.n ? "on" : ""}"></i>`).join("")}</span></span>` : `<span class="dp-fit"><b class="fn">Not a roster call</b></span>`;
  const firm = `<span class="dp-seg small ${e.set ? "gold" : ""}">${["firm", "lean", "theirs"].map((f, i) => `<button data-act="dpfirm" data-key="${esc(r.key)}" data-v="${f}" aria-pressed="${e.set ? e.firm === f : f === "theirs"}"${dis}>${esc(r.firm_labels[i])}</button>`).join("")}</span>`;
  const ex = dpExpCell(x, r, e);
  return `<div class="dp-grow ${on ? "on" : ""} ${e.pend ? "pend" : ""}"><button class="dp-nm" data-act="dpsel" data-k="idrow" data-v="${esc(r.key)}"><b>${esc(r.name)}</b><small>${esc(r.sub)}</small></button>${ctl}${fit}${firm}<span class="dp-exp"><b>${esc(ex.v)}</b><small>${esc(ex.u)}</small></span></div>`;
}
function dpExpCell(x, r, e) {
  if (r.kind === "track") return { v: `${Math.round(dpExpect(x, r, e))}%`, u: String(r.unit).replace(/^%\s*/, "") };
  if (e.pend) return { v: "new", u: "after you save" };
  if (typeof r.expect === "number") return { v: `${Math.round(r.expect)}%`, u: r.unit };
  return { v: String(r.expect), u: r.unit };
}
function dpOptLabel(r, v) {
  const o = (r.options || []).find((q) => q.v === v);
  return o ? o.label : String(v);
}
function dpIdDetail(x, r) {
  const e = dpEff(r);
  const who = r.who === "Both play callers" ? "the play callers" : `the ${r.who.toLowerCase()}`;
  const unit = String(r.unit).replace(/^%\s*/, "");
  let set = r.set_text, firm = r.firm_text;
  if (e.pend) {
    const yours = r.kind === "options" ? dpOptLabel(r, e.to) : `${Math.round(e.to)}% ${unit}`;
    const own = r.kind === "options" ? dpOptLabel(r, r.own) : `${Math.round(r.own)}%`;
    set = e.set ? `Not saved yet: you'd ask for ${yours}, ${DP_FIRM_WORDS[e.firm]}.` : `Not saved yet: you'd leave it to ${who}.`;
    const k = x.pulls ? x.pulls[e.firm] : null;
    firm = e.set && k != null ? `${dpCap(DP_FIRM_WORDS[e.firm])}: he moves about ${Math.round(k * 100)}% of the way from his own ${own} toward ${yours}. Save the identity to make it the plan.` : "Save the identity to make it the plan.";
  }
  const ex = dpExpCell(x, r, e);
  const expLine = ex.v === "new" ? "" : r.kind === "track" ? ` Expect about ${ex.v} ${unit}.` : typeof r.expect === "number" ? ` Expect about ${ex.v} of snaps ${r.unit}.` : "";
  const games = r.games || [];
  let chart;
  if (r.kind === "options" && r.key === "system") chart = `<div class="dp-foot">${esc(dpCap(r.measure))}.</div>`;
  else if (!games.length) chart = `<div class="empty">No games ${esc(x.season)} yet: the bars fill in game by game.</div>`;
  else {
    const lines = [];
    const tgt = r.kind === "track" ? (e.set ? e.to : null) : typeof r.target === "number" ? r.target : null;
    if (tgt != null) lines.push({ v: tgt, label: r.kind === "track" ? "your target" : "expected" });
    if (r.own_line != null && (tgt == null || Math.abs(r.own_line - tgt) > 3)) lines.push({ v: r.own_line, label: r.who === "Both play callers" ? "their habit" : "his own", own: true });
    chart = dpBars(games, lines);
  }
  const kv = (r.kv || []).map((k) => `<div class="dp-kv"><span>${esc(k.k === "Your roster · fit" && r.fit ? `Your roster · ${r.fit.word.toLowerCase()} fit` : k.k)}</span><span>${esc(k.v)}</span></div>`).join("");
  const clear = x.head && e.set ? `<div class="dp-btns"><button class="st-btn small glass" data-act="dpclear" data-key="${esc(r.key)}">Leave it to ${esc(who)}</button></div>` : "";
  return `<div class="panel"><div class="dp-card"><span class="dp-lbl">${esc(r.group)} · ${esc(r.who)}</span><b>${esc(r.name)}</b><span>${esc(set)}</span><small>${esc(firm)}${esc(expLine)}</small>${clear}</div>
    <div><div class="dp-sec"><span class="dp-lbl">${esc(dpCap(x.season))}, game by game</span><small>${r.season != null ? `average ${r.season}% · ` : ""}${esc(r.measure)}</small></div>${chart}</div>
    <div>${kv}</div>
    <div class="dp-foot">Guidelines lean the coordinators, never bind them: a 4-3 team still goes to nickel against three receivers, and a third and long is a throw whatever the run share. Game day's plan starts from here each week.</div></div>`;
}
function dpCoord(c) {
  return `<div class="dp-coord"><div><b>${rich(c.name)}</b><span class="${c.warn ? "warn" : ""}">${esc(c.mood)}</span></div><span>${esc(c.role)} · ${c.of ? `agrees with ${c.agree} of your ${dpPlural(c.of, "guideline")} on his side` : "no guidelines set on his side"}</span><span class="k">${esc(c.knows)}</span><span>${esc(c.line)}</span></div>`;
}
async function dpIdentityTab() {
  const seg = keep("dpidseg", { v: "guide" });
  const segHtml = dpSeg("dpkeep", [["guide", "Guidelines"], ["rules", "Game rules and call names"]], seg.v, { key: "dpidseg", f: "v" });
  if (seg.v === "rules") return dpRulesTab(segHtml);
  const x = await dpGet("dp_identity");
  DP.id = x;
  setColors(x.team.colors);
  // Pending edits from an older screen are dropped once saved.
  for (const k of Object.keys(DP.pend)) if (!x.rows.some((r) => r.key === k)) delete DP.pend[k];
  const sel = x.rows.find((r) => r.key === DP.sel.idrow) || x.rows[0];
  const groups = ["Defense", "Offense", "Program"].map((g) => {
    const rows = x.rows.filter((r) => r.group === g);
    return rows.length ? `<div class="dp-ghd">${g}</div>${rows.map((r) => dpIdRow(x, r, r === sel)).join("")}` : "";
  }).join("");
  const n = Object.keys(DP.pend).length;
  const pending = n ? `<div class="dp-pending"><span><b>${dpPlural(n, "change")}</b> not saved yet: your staff plays to the saved identity until you save.</span><span class="dp-btns" style="margin-left:auto"><button class="st-btn small glass" data-act="dpdrop">Undo them</button><button class="st-btn small primary" data-act="dpsave">Save the identity</button></span></div>` : "";
  const coords = x.coords.length ? `<div class="dp-coords">${x.coords.map(dpCoord).join("")}</div>` : "";
  const left = `<div class="panel"><div class="dp-ph"><span><span class="dp-pt">How we play</span><span class="dp-ps">every week's plan starts from these</span></span>${segHtml}</div>${pending}
    <div class="dp-grow th"><span>Guideline</span><span>Your call · <span class="gold">◆ yours</span> · ○ his own · │ ${esc(x.season)}</span><span>Roster</span><span>How firm</span><span style="text-align:right">Expect</span></div>${groups}${coords}</div>`;
  const actions = x.head
    ? `<button class="st-btn glass" data-act="dptalk">Talk it through with staff</button><button class="st-btn primary" data-act="dpsave"${n ? "" : " disabled"}>Save the identity</button>`
    : `<span class="dp-ps">The head coach's identity: you coach to it.</span>`;
  return { body: `<div class="dp-cols">${left}${dpIdDetail(x, sel)}</div>`, actions, kick: x.set ? `set ${esc(x.set)}` : "nothing set yet" };
}

// ---------- Game rules and call names (board 20) ----------
const DP_FOURTH = { staff: "your staff's own lean", numbers: "by the numbers", aggressive: "aggressive", conservative: "conservative" };
const DP_TWO = { staff: "your staff's call", chart: "by the chart", kick: "always kick" };
const DP_TO = { staff: "timeouts as the staff likes", keep_two: "keep two timeouts", keep_one: "keep one timeout", free: "use timeouts freely" };
const DP_FM = { staff: "the staff's call", run: "run it out", attack: "keep attacking" };
const DP_TEMPO = { normal: "normal tempo", hurry: "no-huddle", milk: "milk the clock" };
async function dpRulesTab(segHtml) {
  const x = await dpGet("dp_rules");
  DP.rules = x;
  const r = x.rules;
  const dis = !x.head;
  const own = x.grid.reduce((n, row) => n + row.cells.filter((c) => c.own).length, 0);
  const sel = DP.sel.cell;
  const head = x.cols.map((c) => { const [a, ...b] = String(c).split(" "); return `<span class="h">${esc(a)}<small>${esc(b.join(" "))}</small></span>`; }).join("");
  const grid = x.grid.map((row, ri) => `<span class="r">${esc(row.name)}</span>${row.cells.map((c, ci) => `<button class="dp-cell ${String(c.v).toLowerCase()}${c.own ? " own" : ""}${sel && sel[0] === ri && sel[1] === ci ? " sel" : ""}" data-act="dpcell" data-r="${ri}" data-c="${ci}" title="${esc(`${row.name}, ball on ${x.cols[ci]}: ${c.why}`)}">${esc(c.v)}</button>`).join("")}`).join("");
  let info = `<span>Click a square for the numbers behind it${x.head ? " and to make the call your own" : ""}.</span>`;
  if (sel && x.grid[sel[0]] && x.grid[sel[0]].cells[sel[1]]) {
    const c = x.grid[sel[0]].cells[sel[1]];
    const opts = [["go", "Go"], ["fg", "Field goal"], ["punt", "Punt"]];
    info = `<span><b>${esc(x.grid[sel[0]].name)}, ball on ${esc(x.cols[sel[1]])}: ${esc(c.v === "FG" ? "field goal" : c.v.toLowerCase())}</b>${c.own ? " (yours)" : ` (the preset)`}. ${esc(c.why)} They convert about ${c.conv}% from here; win chance going ${c.go}%, punting ${c.punt}%${c.fg != null ? `, kicking ${c.fg}%` : ""}.</span>${x.head ? `<span class="dp-btns">${opts.map(([v, t]) => `<button class="dp-chipbtn ${c.own && c.v.toLowerCase() === v ? "on" : ""}" data-act="dpcellset" data-r="${sel[0]}" data-c="${sel[1]}" data-v="${v}">${t}</button>`).join("")}${c.own ? `<button class="dp-chipbtn" data-act="dpcellset" data-r="${sel[0]}" data-c="${sel[1]}" data-v="">Back to the preset (${esc(c.base)})</button>` : ""}</span>` : ""}`;
  }
  const kicker = x.kicker ? `, with ${playerLink(x.kicker.id, x.kicker.name)}'s leg` : "";
  const fourth = `<div class="dp-blk"><div class="dp-bt"><span><b>4th downs</b><small>your analytics staff's chart${kicker}</small></span>${dpSeg("dprule", [["staff", "Staff"], ["numbers", "The numbers"], ["aggressive", "Aggressive"], ["conservative", "Conservative"]], r.fourth, { k: "fourth" }, "", dis)}</div>
    <div class="dp-chart"><span class="h" style="text-align:left">To go · ball on</span>${head}${grid}</div>
    <div class="dp-cellinfo">${info}<small>${x.tally.go} go · ${x.tally.fg} field goal · ${x.tally.punt} punt${own ? ` · ${dpPlural(own, "square")} yours${x.head ? ` · <a class="link" data-act="dpcellclear">clear them</a>` : ""}` : ""}</small></div></div>`;
  const chips = (list) => list.map((m) => `<i class="${m.go ? "on" : ""}" title="${m.go ? "Go for two" : "Kick"}">${m.m}</i>`).join("");
  const twoTxt = { staff: "Your staff decides by its own chart and its trust in the kicker.", chart: "In the 4th quarter, go for two when the margin after the touchdown is in gold; kick otherwise.", kick: "Always kick the extra point." }[r.two];
  const two = `<div class="dp-blk"><div class="dp-bt"><span><b>Two-point tries</b><small>in the 4th quarter, by the score after the touchdown</small></span>${dpSeg("dprule", [["staff", "Staff"], ["chart", "The chart"], ["kick", "Always kick"]], r.two, { k: "two" }, "", dis)}</div>
    <div class="dp-margins"><b>Behind by</b><span>${chips(x.behind)}</span><b>Ahead by</b><span>${chips(x.ahead)}</span></div><span class="dp-foot">${esc(twoTxt)}</span></div>`;
  const toTxt = { staff: "Your staff spends them as the game goes.", keep_two: "Keep two for the last two minutes of each half: one for a challenge, one for the last drive.", keep_one: "Keep one for the end of each half.", free: "Use them when the snap needs it: a bad look, a play clock running out." }[r.timeouts];
  const fmTxt = { staff: "Your staff decides with the lead and the clock.", run: "Ahead by one score with four minutes left: run, stay in bounds, make them spend their timeouts.", attack: "Keep calling the game: a first down ends it as surely as the clock." }[r.four];
  const twoCols = `<div class="dp-two-cols"><div class="dp-blk"><div class="dp-bt"><span><b>Timeouts</b></span></div>${dpSeg("dprule", [["staff", "Staff"], ["keep_two", "Keep two"], ["keep_one", "Keep one"], ["free", "Freely"]], r.timeouts, { k: "timeouts" }, "", dis)}<span>${esc(toTxt)}</span></div>
    <div class="dp-blk"><div class="dp-bt"><span><b>Four-minute plan</b></span></div>${dpSeg("dprule", [["staff", "Staff"], ["run", "Run it out"], ["attack", "Keep attacking"]], r.four, { k: "four" }, "", dis)}<span>${esc(fmTxt)}</span></div></div>`;
  const week = x.week && x.week.changed ? `<div class="dp-rulesnote"><span>For ${rich(x.week.opp)} on ${esc(x.week.date)} game day changed the rules for that game only: 4th downs ${esc(DP_FOURTH[x.week.rules.fourth])}, two-point ${esc(DP_TWO[x.week.rules.two])}.</span><a class="link" data-go="gameday" data-tab="Game plan">See game day</a></div>` : "";
  const log = `<div class="dp-blk"><span class="dp-lbl">${esc(dpCap(x.season_words))} against the rules</span><div class="dp-tiles">${x.season.map((s) => `<div class="dp-tile"><span>${esc(s.k)}</span><b>${esc(s.v)}</b><small>${esc(s.sub)}</small></div>`).join("")}</div></div>`;
  const left = `<div class="panel"><div class="dp-ph"><span><span class="dp-pt">Standing game rules</span><span class="dp-ps">game day starts from these each week</span></span>${segHtml}</div>${fourth}${two}${twoCols}${week}${log}</div>`;
  const sides = x.words.map((s) => `<div class="dp-card"><span class="dp-lbl">${esc(s.name)}'s call names</span><b class="num">since ${esc(s.since)}</b><small>${s.changed ? `changed ${esc(s.changed)}${s.fresh ? ": your players learn them this week" : ""}` : `${dpPlural(s.knowers, "man", "men")} who left know them`}${s.leak != null ? ` · ${s.leak > 0 ? `your next opponent reads about ${s.leak}% of calls` : "your next opponent reads none"}` : ""}</small></div>`).join("");
  const holders = x.holders.length
    ? x.holders.map((h) => `<div class="dp-holder${h.when ? " warn" : ""}"><span><b>${rich(h.name)} <small>· ${esc(h.what)}</small></b><span>${h.side ? "Defense" : "Offense"}'s words · now ${rich(h.now)}</span>${h.when ? `<span class="warn">You play them ${esc(h.when)}</span>` : ""}</span><span><b>${h.share}%</b><small>of calls</small></span></div>`).join("")
    : '<div class="empty">Nobody who left your staff or roster knows the words you use now.</div>';
  const change = x.head
    ? `<div class="dp-card"><b>New call names</b><span>New words cost a little: about one extra bust in 70 snaps for the side that changed, the week they go in. Everyone who left with the old words stops reading your calls.</span><div class="dp-btns">${x.words.map((s) => s.fresh ? `<button class="st-btn small glass" data-act="dpwords" data-side="${s.side}" data-undo="1">Undo the ${esc(s.name.toLowerCase())}'s change</button>` : `<button class="st-btn small" data-act="dpwords" data-side="${s.side}">New ${esc(s.name.toLowerCase())} words</button>`).join("")}</div></div>`
    : "";
  const right = `<div class="panel"><div class="dp-ph"><span><span class="dp-pt">Call names</span><span class="dp-ps">who could read us</span></span></div><div class="dp-cols even">${sides}</div><span class="dp-lbl">Who knows our calls</span><div class="dp-holders">${holders}</div>${change}<div class="dp-foot">Wristband numbers and sideline boards change every game by themselves and cost nothing. The words players learned are what a player takes along.</div></div>`;
  return { body: `<div class="dp-cols">${left}${right}</div>`, actions: x.head ? "" : `<span class="dp-ps">The head coach's rules: you coach to them.</span>`, kick: "standing rules" };
}

// ---------- Self-scout (board 21) ----------
async function dpScoutTab() {
  const k = keep("dpscout", { side: "offense" });
  const x = await dpGet("dp_scout", { side: k.side });
  const def = x.side === "defense";
  const lvl = DP_LVL[x.level] || "level";
  const seg = dpSeg("dpkeep", [["offense", "Our offense"], ["defense", "Our defense"]], k.side, { key: "dpscout", f: "side" });
  const downs = x.downs.map((d) => `<div class="dp-drow"><span><b>${esc(d.label)}</b><small>${dpPlural(d.n, def ? "throw" : "snap")}</small></span><span><span class="dp-bar"><i class="${d.flag ? "bad" : ""}" style="width:${dpNum(d.v)}%"></i><u style="left:${dpNum(d.avg)}%" title="The ${esc(lvl)} usual: ${d.avg}%"></u></span><small class="n${d.flag ? " bad" : ""}">${esc(d.note)}</small></span><b class="${d.flag ? "bad" : ""}">${d.v != null ? d.v + "%" : "–"}</b></div>`).join("");
  const g = x.guess;
  const guess = g.games.length ? dpBars(g.games, [{ v: g.line, label: `${lvl} ${g.line}%`, own: true }], { hi: (v) => v > g.line + 5 }) : `<div class="empty">No game with ten snaps yet ${esc(x.season)}.</div>`;
  const tables = x.tables.map((t) => `<div><div class="dp-sec"><b>${esc(t.title)}</b></div><div class="dp-trow th">${t.cols.map((c) => `<span>${esc(c)}</span>`).join("")}</div>${t.rows.length ? t.rows.map((r) => `<div class="dp-trow">${r.cells.map((c, i) => `<span${r.flag && i >= 2 ? ' class="flag"' : ""}>${esc(c)}</span>`).join("")}</div>`).join("") : '<div class="empty">No snaps yet.</div>'}${t.note ? `<div class="dp-foot" style="padding-top:6px">${esc(t.note)}</div>` : ""}</div>`).join("");
  const left = `<div class="panel"><div class="dp-ph"><span><span class="dp-pt">How we look on film</span><span class="dp-ps">${dpPlural(x.games, "game")}, ${dpPlural(x.snaps, "snap")} · ${esc(x.season)}</span></span>${seg}</div>
    <div class="dp-cols"><div><div class="dp-sec"><b>${def ? "Man coverage by down" : "Runs by down"}</b><small>│ the ${esc(lvl)} usual</small></div>${downs}
      <div style="padding-top:16px"><div class="dp-sec"><b>${def ? "A man-or-zone guess" : "A run-or-pass guess"}, game by game</b><small>${g.overall != null ? `right on ${g.overall}% overall` : "no games yet"}</small></div>${guess}</div>
      <div class="dp-foot" style="padding-top:12px">${esc(x.read)}</div></div>
    <div style="display:flex;flex-direction:column;gap:16px">${tables}</div></div></div>`;
  const tsel = Math.min(Number(DP.sel.tell) || 0, Math.max(0, x.tells.length - 1));
  const tells = x.tells.length
    ? x.tells.map((t, i) => `<button class="dp-tell ${i === tsel ? "on" : ""}" data-act="dpsel" data-k="tell" data-v="${i}"><span><b>${esc(t.name)}</b><small>${esc(t.what)}</small><small class="f">${t.seen != null ? `seen in ${t.seen} of ${dpPlural(t.of, "game")}` : `over ${dpPlural(t.of, "game")}`}</small></span><span><b class="${t.strength}">${t.share}%</b><small class="${t.strength}">${esc(t.strength)}</small></span></button>`).join("")
    : `<div class="empty">No tell a ${def ? "play caller" : "defense"} could trust yet: nothing passes ${esc(x.edge)}.</div>`;
  const t = x.tells[tsel];
  let card = "";
  if (t) {
    const sees = def
      ? `${dpCap(t.what)}: an offense that studies your tape ${t.name.startsWith("Man") ? "calls its man-beaters there" : "keeps a back in and throws hot"}.`
      : `From ${t.name.toLowerCase()} you ${t.what}: a defense that studies your tape ${t.kind === "run" ? "loads the box" : "plays the pass"} when it sees it.`;
    const br = (t.breakers || []).length
      ? t.breakers.map((b) => `<div class="dp-opt"><span><span class="gold">↺</span> ${esc(dpCap(b))}</span><small>on this week's sheet</small></div>`).join("")
      : `<span class="dp-foot">${def ? "Mix the coverage on this down and group: send the sheet back with \"More zone\" or \"More man\"." : "No tendency breaker for this look on this week's sheet. Send it back with \"Break our tendencies\"."} <a class="link" data-act="tab" data-key="dpplan" data-t="Call sheet">Call sheet</a></span>`;
    card = `<div class="dp-card"><span class="dp-lbl">What a ${def ? "play caller" : "defense"} sees</span><span>${esc(sees)}</span><span class="dp-lbl">Tendency breakers</span>${br}</div>`;
  }
  let study = "";
  if (x.studied) study = `<div class="dp-foot">Your staff ran the full study ${esc(x.study)}: every snap since spring, read with a sharper eye (a tell from ${esc(x.edge)}).</div>`;
  else if (x.head && x.bye) study = `<div class="dp-card"><b>The full study</b><span>No game for more than a week: your staff has time to go through every snap since spring. Tells show from 6 snaps and 70% instead of 8 and 75%.</span><div class="dp-btns"><button class="st-btn small primary" data-act="dpstudy" data-side="${esc(k.side)}">Run the full study</button></div></div>`;
  else study = `<div class="dp-foot">${x.next_in != null ? `Your next game is in ${dpPlural(x.next_in, "day")}: the full study waits for a bye week.` : "The full study comes with a bye week."} ${esc(x.cut)}</div>`;
  const right = `<div class="panel"><div class="dp-ph"><span><span class="dp-pt">Tells</span><span class="dp-ps">what gives the play away</span></span></div><div style="display:flex;flex-direction:column;gap:2px">${tells}</div>${card}${study}<div class="dp-foot">The read updates every week from the last game's tape. ${x.breakers ? `${dpPlural(x.breakers, "tendency breaker")} on this week's sheet.` : ""}</div></div>`;
  return { body: `<div class="dp-cols">${left}${right}</div>`, kick: def ? "our defense on film" : "our offense on film" };
}

// ---------- Call sheet (board 22) ----------
function dpAllPlays(x) {
  return [...x.script.plays, ...x.extras.flatMap((b) => b.plays), ...x.grid.flatMap((b) => b.plays)];
}
function dpBoxOf(x, i) {
  if (x.script.plays.some((p) => p.i === i)) return x.script.name;
  const b = [...x.extras, ...x.grid].find((q) => q.plays.some((p) => p.i === i));
  return b ? b.name : "";
}
function dpPl(p, n, script) {
  const on = DP.sel.play === p.i;
  const marks = `${p.fav ? '<span class="m" title="The quarterback\'s favourite">★</span>' : ""}${p.breaker ? '<span class="m" title="Breaks a tendency">↺</span>' : ""}${p.cut ? '<span class="m bad" title="Your coordinator would cut it">✂</span>' : ""}${p.yours ? '<span class="m" title="Yours">●</span>' : ""}`;
  const tail = script && p.tests ? p.tests : p.look;
  return `<button class="dp-pl${on ? " on" : ""}${p.cut ? " cut" : ""}" data-act="dpsel" data-k="play" data-v="${p.i}" title="${esc(`${dpCap(p.name)} · ${p.look}${p.why ? " · " + p.why : ""}`)}"><span class="i">${n}</span><span class="n">${esc(dpCap(p.name))}</span>${marks}<span class="t">${esc(tail)}</span></button>`;
}
function dpBox(b, split) {
  let inner;
  if (split) {
    const col = (h, list) => `<div><span class="ch">${h}</span>${list.map((p, j) => dpPl(p, j + 1)).join("") || '<span class="ch">none</span>'}</div>`;
    inner = `<div class="hc">${col("Left hash", b.plays.filter((p) => p.hash !== 2))}${col("Right hash", b.plays.filter((p) => p.hash === 2))}</div>`;
  } else inner = b.plays.map((p, j) => dpPl(p, j + 1)).join("");
  return `<div class="dp-box"><div><b>${esc(b.name)}</b><small>${dpPlural(b.plays.length, "call")}</small></div>${inner || '<span class="ch">nothing in it</span>'}</div>`;
}
async function dpSheetTab() {
  const k = keep("dpsheet", { side: "offense", wet: false });
  const x = await dpGet("dp_sheet", { side: k.side, wet: k.wet });
  if (!x.has) {
    return { body: `<div class="panel"><div class="dp-ph"><span><span class="dp-pt">The call sheet</span><span class="dp-ps">drafted by your coordinators the week of each game</span></span></div><p class="sub">${esc(x.why)}</p><div class="dp-btns"><button class="st-btn small glass" data-act="tab" data-key="dpplan" data-t="Playbook">The playbook</button><button class="st-btn small glass" data-go="schedule">Schedule</button></div></div>`, kick: "no game ahead" };
  }
  DP.sheet = x;
  const def = x.side === "defense";
  const sideSeg = dpSeg("dpkeep", [["offense", "Offense"], ["defense", "Defense"]], k.side, { key: "dpsheet", f: "side" });
  const wetSeg = dpSeg("dpkeep", [["false", "Dry"], ["true", `Wet${x.wet_n ? ` · ${x.wet_n}` : ""}`]], String(!!k.wet), { key: "dpsheet", f: "wet" });
  const legend = `${def ? "" : "★ the quarterback's favourite · "}↺ breaks a tendency · ✂ your coordinator would cut · ● yours`;
  const waiting = x.parts.filter((p) => !p.approved).length;
  const banner = x.all_approved ? `<span class="gold">Approved: it goes on the card as it stands</span>` : `<span>${dpPlural(waiting, "part")} waiting for your look</span>`;
  // The script's lines show what each play tests.
  const scriptBox = `<div class="dp-box"><div><b>${esc(x.script.name)}</b><small>what each play tests</small></div>${x.script.plays.map((p, j) => dpPl(p, j + 1, true)).join("") || '<span class="ch">nothing in it</span>'}</div>`;
  const left = `<div class="panel"><div class="dp-ph"><span><span class="dp-pt">${def ? "Defensive call sheet" : "Offensive call sheet"}</span><span class="dp-ps">for ${rich(x.opp)} · ${esc(x.game)}</span></span><span class="dp-btns">${sideSeg}${wetSeg}</span></div>
    <div class="dp-legend"><span>${esc(legend)}</span>${banner}</div>
    <div class="dp-sheetw"><div>${scriptBox}${x.extras.map((b) => dpBox(b, false)).join("")}</div>
      <div class="dp-boxes">${x.grid.map((b) => dpBox(b, b.split)).join("")}</div></div></div>`;
  const by = x.by ? (plainText(x.by) === "you" ? "You" : rich(x.by)) : "Your coordinator";
  const doneN = x.parts.filter((p) => p.approved).length;
  const psel = DP.sel.part != null ? Number(DP.sel.part) : null;
  const parts = x.parts.map((p) => `<button class="dp-prt" data-act="dpsel" data-k="part" data-v="${p.i}" style="${psel === p.i ? "background:var(--dp-sel);box-shadow:inset 2px 0 0 var(--dp-gold)" : ""}"><span>${esc(p.name)}</span><span>${p.n}</span><span class="${p.state === "Approved" ? "ok" : p.state.startsWith("You edited") ? "ed" : ""}">${esc(p.state)}</span></button>`).join("");
  const play = dpAllPlays(x).find((p) => p.i === DP.sel.play);
  let card;
  if (play) {
    const marks = [play.fav && "the quarterback's favourite", play.breaker && "breaks a tendency", play.cut && "your coordinator would cut it", play.yours && "yours", play.wet && "for wet weather"].filter(Boolean).join(" · ");
    const hash = play.hash === 1 ? "left hash" : play.hash === 2 ? "right hash" : "either hash";
    card = `<div class="dp-card"><span class="dp-lbl">${esc(dpBoxOf(x, play.i))} · ${hash}</span><b>${esc(dpCap(play.name))}</b>${marks ? `<small class="${play.cut ? "bad" : "gold"}">${esc(marks)}</small>` : ""}<small>${esc(play.look)}</small><span class="dp-lbl">Why ${play.yours ? "it's there" : !x.by ? "your coordinator has it" : plainText(x.by) === "you" ? "you have it" : `${plainText(x.by)} has it`}</span><span>${esc(play.why || "your call")}</span>${play.tests ? `<div class="dp-kv2"><span>Tests</span><span>${esc(play.tests)}</span></div>` : ""}
      ${x.head ? `<div class="dp-btns">${play.cut ? `<button class="st-btn small glass" data-act="dpedit" data-i="${play.i}" data-a="keep">Keep it</button>` : ""}<button class="st-btn small glass" data-act="dpedit" data-i="${play.i}" data-a="up">Move up</button><button class="st-btn small glass" data-act="dpedit" data-i="${play.i}" data-a="cut">Cut it</button></div>` : ""}</div>`;
  } else {
    const favs = !def && x.favourites.length ? `<span class="dp-lbl">The quarterback's favourites</span>${x.favourites.slice(0, 4).map((f) => `<div class="dp-opt"><span>★ ${esc(dpCap(f.name))}</span><small>${dpPlural(f.n, "throw")} · ${f.epa >= 0 ? "+" : ""}${f.epa} EPA a throw</small></div>`).join("")}` : "";
    card = `<div class="dp-card"><span class="dp-lbl">Drafted ${esc(x.drafted)}${x.round ? ` · redraft ${x.round}` : ""}</span><b>${def ? "The defense's card" : "The offense's card"}</b><span>${by} drafted it from the film and your identity. Pick a play to see why it's there; your play callers call from the card on about ${x.stick}% of the snaps where a card play fits.</span>${favs}</div>`;
  }
  // Adding a play: beside the picked one, in its box.
  let add = "";
  if (x.head) {
    if (!def) {
      const a = keep("dpadd", { personnel: "11", backfield: "shotgun", set: "doubles", kind: "pass", k: 0 });
      const list = a.kind === "run" ? x.add.runs : x.add.passes;
      if (!list.some((p) => p.i === Number(a.k))) a.k = list[0] ? list[0].i : 0;
      const sel = (f, opts, cur) => `<select class="dp-wsel" data-dpch="dpadd" data-f="${f}">${opts.map(([v, t]) => `<option value="${esc(v)}" ${String(v) === String(cur) ? "selected" : ""}>${esc(t)}</option>`).join("")}</select>`;
      add = `<div class="dp-notes">${sel("personnel", x.add.personnels.map((p) => [p, p + " personnel"]), a.personnel)}${sel("backfield", x.add.backfields.map((b) => [b, dpCap(b)]), a.backfield)}${sel("set", x.add.sets.map((s) => [s, dpCap(s)]), a.set)}${sel("kind", [["pass", "Pass"], ["run", "Run"]], a.kind)}${sel("k", list.map((p) => [p.i, dpCap(p.name)]), a.k)}</div>`;
    } else {
      const a = keep("dpaddd", { coverage: 0, pressure: 0 });
      const sel = (f, opts, cur) => `<select class="dp-wsel" data-dpch="dpaddd" data-f="${f}">${opts.map((o) => `<option value="${o.i}" ${String(o.i) === String(cur) ? "selected" : ""}>${esc(o.name)}</option>`).join("")}</select>`;
      add = `<div class="dp-notes">${sel("coverage", x.add.coverages, a.coverage)}${sel("pressure", x.add.pressures, a.pressure)}</div>`;
    }
    add = `<div class="dp-card"><span class="dp-lbl">Add a call of your own</span>${add}<div class="dp-btns"><button class="st-btn small glass" data-act="dpaddplay"${play ? "" : " disabled"}>${play ? `Add to ${esc(dpBoxOf(x, play.i))}` : "Pick a play in a box first"}</button></div></div>`;
  }
  const notesSel = DP.notes[x.side] || [];
  const notes = x.notes.map((n) => `<button class="dp-chipbtn ${notesSel.includes(n.i) ? "on" : ""}" data-act="dpnote" data-v="${n.i}" title="${n.on ? "In your last note" : ""}"${x.head ? "" : " disabled"}>${esc(n.text)}${n.on ? " ·" : ""}</button>`).join("");
  const part = psel != null ? x.parts.find((p) => p.i === psel) : null;
  const review = x.head
    ? `<div class="dp-card"><span class="dp-lbl">Your note if you send it back${part ? ` · ${esc(part.name)}` : " · the whole sheet"}</span><div class="dp-notes">${notes}</div><div class="dp-btns"><button class="st-btn small primary" data-act="dpapprove"${part ? ` data-part="${part.i}"` : ""}>${part ? "Approve this part" : "Approve the whole sheet"}</button><button class="st-btn small glass" data-act="dpsendback"${part ? ` data-part="${part.i}"` : ""}${notesSel.length ? "" : " disabled"}>Send it back</button>${part ? `<button class="st-btn small glass" data-act="dpsel" data-k="part" data-v="">The whole sheet</button>` : ""}</div></div>`
    : `<div class="dp-foot">The head coach approves the sheet. You coach your room to it.</div>`;
  const right = `<div class="panel"><div><div class="dp-sec"><span class="dp-lbl">The draft, part by part</span><small>${doneN} of ${x.parts.length} approved</small></div>${parts}</div>${card}${add}${review}</div>`;
  return { body: `<div class="dp-cols">${left}${right}</div>`, kick: `drafted by ${x.by ? plainText(x.by) : "your coordinator"}` };
}

// ---------- Installs (board 23) ----------
const dpG = (v) => Math.max(0, Math.min(100, ((dpNum(v) - 40) / 60) * 100)).toFixed(1);
async function dpInstallsTab() {
  const k = keep("dpinst", { unit: "offense" });
  const x = await dpGet("dp_installs", { unit: k.unit });
  const seg = dpSeg("dpkeep", [["offense", "Offense"], ["defense", "Defense"], ["kicking", "Kicking game"]], k.unit, { key: "dpinst", f: "unit" });
  const sel = x.rows.find((r) => r.id === DP.sel.inst) || x.rows.find((r) => r.flag) || x.rows[0];
  const rows = x.rows.map((r) => `<button class="dp-irow ${sel && sel.id === r.id ? "on" : ""}" data-act="dpsel" data-k="inst" data-v="${r.id}"><span><i>${esc(r.pos)}</i><b>${esc(r.name)}</b><small>${esc(r.class)} · ${esc(r.role)}</small></span><span><span class="dp-gb ${r.flag ? "low" : ""}"><i class="f" style="width:${dpG(r.friday)}%"></i><i class="n" style="width:${dpG(r.now)}%"></i><u style="left:${dpG(x.ready)}%"></u></span><span class="dp-gv">${x.week ? `<span>${r.now} →</span> <b class="${r.flag ? "bad" : ""}">${r.friday}</b>` : `<b>${r.now}</b>`}</span></span><span class="r">${r.calls}</span><span class="r">${esc(r.film)}</span><span class="r${r.busts >= 3 ? " bad" : r.busts >= 1 ? " mid" : ""}">${x.week ? r.busts : "–"}</span></button>`).join("");
  const opp = x.opp ? rich(x.opp) : "";
  const left = `<div class="panel"><div class="dp-ph"><span><span class="dp-pt">Who knows the plan</span><span class="dp-ps">${!x.has_game ? "no game ahead: the plan goes in the week of each game" : x.week ? `the ${opp} plan, from walkthroughs, reps and film` : `the ${opp} plan goes in the week of the game`}</span></span>${seg}</div>
    <div class="dp-irow th"><span>Player</span><span>${x.week ? "Grasp from 40% · now → by game day" : "Grasp from 40% · now"} · │ game ready ${x.ready}%</span><span class="r">New</span><span class="r">Film h</span><span class="r">Busts</span></div>
    ${rows || '<div class="empty">Nobody in this unit\'s two-deep yet.</div>'}
    <div class="dp-opt"><span>${!x.rows.length ? "" : x.week ? `The unit is at ${x.total.now}% now and ${x.total.friday}% by game day: ${esc(x.total.words)} a game expected (${x.total.busts}).` : `The unit knows ${x.total.now}% of the sheet now. The week of the game puts in the rest.`}</span><small>${esc(x.film_hours)} h of film a man · ${esc(x.load)}</small></div></div>`;
  const fams = x.families.length
    ? x.families.map((f) => `<div class="dp-item"><span><b>${esc(dpCap(f.name))}</b><small>${dpPlural(f.calls, "call")} on the sheet${f.new ? " · new to them" : ""}</small></span><span><span class="dp-gv">${x.week ? `<span>${f.now} →</span> <b class="${f.friday < x.ready ? "bad" : ""}">${f.friday}</b>` : `<b>${f.now}</b>`}</span><span class="dp-gb ${x.week && f.friday < x.ready ? "low" : ""}" style="width:96px;flex:none"><i class="f" style="width:${dpG(f.friday)}%"></i><i class="n" style="width:${dpG(f.now)}%"></i><u style="left:${dpG(x.ready)}%"></u></span></span></div>`).join("")
    : `<div class="empty">${x.sheet ? "Nothing new for this unit on this week's sheet." : "No sheet drafted yet: the families go in once your coordinators draft it."}</div>`;
  const H = 110;
  const hh = (v) => Math.round((Math.max(0, dpNum(v) - 40) / 60) * H);
  const week = x.chart.length
    ? `<div class="dp-week"><span class="rl" style="top:${140 - hh(x.ready)}px"></span><span class="rt" style="top:${Math.max(0, 140 - hh(x.ready) - 14)}px">game ready</span><div class="cols">${x.chart.map((c, i) => `<div><b class="${c.proj ? "p" : ""}">${c.v}</b><i class="${c.proj ? "p" : ""}${!c.proj && (i === x.chart.length - 1 || x.chart[i + 1].proj) ? " now" : ""}" style="height:${Math.max(2, hh(c.v))}px"></i></div>`).join("")}</div></div><div class="dp-weekx">${x.chart.map((c, i) => `<span class="${!c.proj && (i === x.chart.length - 1 || x.chart[i + 1].proj) ? "now" : ""}">${esc(c.d)}</span>`).join("")}</div>`
    : `<div class="empty">${x.has_game && !x.week ? "The chart fills in the week of the game." : "The week's chart fills in from Monday."}</div>`;
  let card = "";
  if (sel) {
    const opt = (xk, label, on, note) => `<div class="dp-opt"><span>${label}</span><span class="dp-btns" style="align-items:center"><small>${note}</small><button class="dp-chipbtn ${on ? "on" : ""}" data-act="dpextra" data-id="${sel.id}" data-x="${xk}" data-on="${on ? 0 : 1}"${x.week ? "" : " disabled"}>${on ? "On" : "Add"}</button></span></div>`;
    card = `<div class="dp-card"><span class="dp-lbl">${esc(sel.pos)} · ${esc(sel.class)} · ${esc(sel.role)} string</span><div class="dp-sec"><b>${playerLink(sel.id, sel.name)}</b><b class="num ${sel.flag ? "bad" : ""}">${x.week ? `${sel.now}% → ${sel.friday}%` : `${sel.now}%`}</b></div><span>Your staff's read: he ${esc(sel.iq_words)}. ${x.week ? `${dpPlural(sel.calls, "new call")} this week, ${esc(sel.film)} h of film. ${sel.flag ? "At this pace he isn't game ready by game day: expect busts on the new calls." : "Game ready by game day at this pace."}` : `${dpPlural(sel.calls, "call")} on the sheet he doesn't know yet. Extra film, walkthroughs and base downs open in the week of the game.`}</span>
      ${opt("film", "Extra film tonight", sel.extra.film, "+1.5 h")}${opt("walk", "An extra walkthrough", sel.extra.walk, "the new calls, at walking pace")}${opt("base", "Base downs only this week", sel.extra.base, "the new calls go to his backup")}</div>`;
  }
  let designs = "";
  try {
    const p = await dpGet("plans");
    if (p.designs && p.designs.length && k.unit === "offense") {
      designs = `<div><span class="dp-lbl">Your own plays going in</span>${p.designs.map((d) => `<label class="dp-opt" style="cursor:pointer"><span><input type="checkbox" data-act="tr-inst" data-k="${d.k}" ${d.installing ? "checked" : ""}> ${esc(d.design.name)}</span>${typeof trInstall === "function" ? trInstall(d.install, d.words) : `<small>${esc(d.words)}</small>`}</label>`).join("")}</div>`;
    }
  } catch (e) {
    designs = "";
  }
  const right = `<div class="panel"><div class="dp-ph"><span><span class="dp-pt">${x.week || !x.has_game ? "New this week" : "On the sheet"}</span><span class="dp-ps">${x.week ? "the unit's grasp, now → game day" : "the unit's grasp now"}</span></span></div><div>${fams}</div>
    <div><div class="dp-sec"><span class="dp-lbl">The unit's grasp through the week</span><small>${!x.has_game ? "no game this week" : x.week ? `${dpPlural(x.days, "practice day")} left before ${esc(x.game)}` : `${esc(x.game)}, ${dpPlural(x.away, "day")} away`}</small></div>${week}</div>${card}${designs}</div>`;
  return { body: `<div class="dp-cols">${left}${right}</div>`, kick: x.has_game ? `${esc(x.game)}` : "" };
}

// ======================================================================
// The depth chart's Packages tab (board 24)
// ======================================================================
const dpDepthBase = screens.depth;
screens.depth = async (a) => {
  try {
    a = a || {};
    if (a.kind && typeof trMine === "function" && !trMine(a)) return await dpDepthBase(a);
    const k = keep("trboard", { t: null });
    if (a.tab === "Packages") k.t = "Packages";
    if (k.t === "Packages" && (!a.tab || a.tab === "Packages")) return await dpPackagesPage(a);
    const html = await dpDepthBase(a);
    return html.replace('<button role="tab" data-act="tab" data-key="trboard" data-t="By position"', '<button role="tab" data-act="tab" data-key="trboard" data-t="Packages" aria-selected="false">Packages</button><button role="tab" data-act="tab" data-key="trboard" data-t="By position"');
  } finally {
    DP.reuse = false;
  }
};
const DP_ROLES = [[0, "Staff's call"], [1, "Whole series"], [2, "Every other"], [3, "Rush downs"]];
async function dpPackagesPage(a) {
  const bx = await api("boards", {});
  setColors(bx.team.colors);
  title("Depth chart", "Packages");
  const names = [...bx.boards.map((b) => b.label), "Packages", "By position"];
  const head = pageHead({ kicker: `${esc(typeof trCap === "function" ? trCap(bx.tier) : bx.tier)} · game day`, title: "Depth chart", sub: "Who plays in each package, three deep, and how the defensive line rotates. Your position coaches fill what you leave.", goto: gotoLine([["Squad", "roster"], ["Training", "training"], ["Plans", "plans"], ["Game day", "gameday"], ["Schedule", "schedule"]]) });
  const tabsHtml = tabs("trboard", names.map((n) => { const b = bx.boards.find((q) => q.label === n); return b && b.yours ? { name: n, badge: "●" } : n; }), "Packages");
  const want = keep("dppk", { board: "Nickel" });
  const x = await dpGet("dp_packages", { board: want.board });
  const dis = x.head ? "" : " disabled";
  const groups = ["Defense", "Offense"].map((g) => {
    const list = x.packages.filter((p) => p.group === g);
    return list.length ? `<span class="dp-lbl" style="padding:8px 12px 4px">${g}</span>${list.map((p) => `<button class="dp-pk ${p.key === x.open ? "on" : ""}" data-act="dpkeep" data-key="dppk" data-f="board" data-v="${esc(p.key)}"><span><b>${esc(p.label)}${p.yours ? ' <span class="gold">●</span>' : ""}</b><i>${p.share != null ? p.share + "%" : "–"}</i></span><small class="${p.thin ? "bad" : ""}">${esc(p.ready)}</small></button>`).join("")}` : "";
  }).join("");
  const pkg = x.packages.find((p) => p.key === x.open) || {};
  const pool = (pos) => ((x.pool.find((q) => q.pos === pos) || {}).players || []);
  const cell = (s, pk, i) => {
    const p = pk.p;
    if (i === 0) return `<span class="dp-dc first"><span>${p ? playerLink(p.id, p.name) : '<span class="muted">Open</span>'}</span>${p ? range(p.read, null, { p: p.id, k: "ovr" }) : ""}</span>`;
    const opts = pool(s.pos).filter((q) => !s.picks[0].p || q.id !== s.picks[0].p.id);
    const cur = pk.yours && p ? p.id : 0;
    const out = p && p.practice === "out";
    return `<span class="dp-dc ${pk.yours ? "yours" : ""} ${i === 1 && s.thin ? "thin" : ""}" title="${p ? esc(`${p.name}: ${p.class}${p.practice !== "full" ? `, ${p.practice}` : ""}${p.note ? `, ${p.note}` : ""}`) : "Nobody behind him"}"><select data-dpch="dpdepth" data-board="${esc(x.open)}" data-slot="${s.k}" data-n="${i + 1}"${dis}><option value="0">${p && !pk.yours ? `${esc(p.name)} (staff)` : "Your staff's pick"}</option>${opts.map((q) => `<option value="${q.id}" ${q.id === cur ? "selected" : ""}>${esc(q.name)} · ${esc(q.class)}${q.practice !== "full" ? ` · ${esc(q.practice)}` : ""}</option>`).join("")}</select>${out ? '<span class="out">out</span>' : p ? range(p.read, null, { p: p.id, k: "ovr" }) : ""}</span>`;
  };
  const spots = x.spots.map((s) => `<div class="dp-drw"><span>${esc(s.slot)}</span>${s.picks.map((pk, i) => cell(s, pk, i)).join("")}</div>`).join("");
  const when = pkg.share != null ? `On ${pkg.share}% of ${pkg.group === "Defense" ? "the snaps your defense played" : "your offense's snaps"} this season.` : "Not enough snaps this season to count yet (30 or more).";
  const left = `<div class="panel"><div class="dp-ph"><span><span class="dp-pt">Packages</span><span class="dp-ps">who plays in each, three deep</span></span></div>
    <div class="dp-pkw"><div style="display:flex;flex-direction:column;gap:2px">${groups}</div><div style="min-width:0"><div class="dp-sec"><b>${esc(x.label)}</b><small>${esc(when)}</small></div>
    <div class="dp-drw th"><span>Spot</span><span>1st · <a class="link" data-act="tab" data-key="trboard" data-t="${esc(x.label)}">set on its board</a></span><span>2nd</span><span>3rd</span></div>${spots || '<div class="empty">No spots on this package.</div>'}
    <div class="dp-foot" style="padding-top:10px">The 1st string is the package's board. Behind it, a spot you leave goes to your position coach's next man; a thin spot is one whose 2nd string is out or missing.</div></div></div></div>`;
  const series = Array.from({ length: x.series }, (_, i) => `<span>${i + 1}</span>`).join("");
  const line = x.line.length
    ? x.line.map((p) => `<div class="dp-rg"><span><b>${playerLink(p.id, p.name)}</b><small class="dp-rot">${esc(p.pos)} · ${esc(p.string)} · <select data-dpch="dprot" data-id="${p.id}"${dis}>${DP_ROLES.map(([v, t]) => `<option value="${v}" ${v === p.role ? "selected" : ""}>${t}</option>`).join("")}</select></small></span>${p.cells.map((c) => `<i class="${c}"></i>`).join("")}<b>${p.snaps}</b></div>`).join("")
    : '<div class="empty">No defensive linemen on the depth chart.</div>';
  const rules = ["Whole series", "Every other", "Rush downs", "Staff's call"].map((t, i) => `<div class="dp-kv2"><span>${t}</span><span>${esc(x.rules[i] || "")}</span></div>`).join("");
  const right = `<div class="panel"><div class="dp-ph"><span><span class="dp-pt">Line rotation</span><span class="dp-ps">by series, a normal game</span></span></div>
    <div class="dp-key"><span><i class="whole"></i>whole series</span><span><i class="staff"></i>the staff's spell</span><span><i class="rush"></i>rush downs only</span></div>
    <div class="dp-rg th"><span>Series</span>${series}<span>Snaps</span></div>${line}<div>${rules}</div>
    <div class="dp-tiles"><div class="dp-tile"><span>Line snaps a game</span><b>${x.line_snaps}</b><small>the top four at each spot, about ${x.series} series</small></div></div></div>`;
  return `<div class="film"><div class="pad tr-pad">${head}${tabsHtml}</div><div class="pad dp" style="padding-top:0"><div class="dp-cols">${left}${right}</div></div></div>`;
}

// ======================================================================
// Game day: the week's plan (board 25) and teams like us (board 26)
// ======================================================================
const dpGdBase = screens.gameday;
screens.gameday = async (a) => {
  try {
    return await dpGdBase(a);
  } finally {
    DP.reuse = false;
  }
};
const DP_ST = [
  ["kick", "Kickoffs", ["Staff", "Deep", "Squib", "Pooch"]],
  ["punt", "Punts", ["Staff", "Directional", "Coffin corner"]],
  ["ret", "Kick returns", ["Staff", "Bring it out", "Touchback"]],
  ["pret", "Punt returns", ["Staff", "Set up a return", "Fair catch"]],
  ["fake", "Fake punt", ["Staff", "If the look shows", "First chance"]],
  ["onside", "Onside kick", ["Staff", "If the look shows", "First chance"]],
];
window.dpWeekTab = async (h, parts) => {
  let x;
  try {
    x = await dpGet("dp_week");
  } catch (e) {
    return parts.fallback || "";
  }
  if (!x.has) return parts.fallback || "";
  const dis = x.head ? "" : " disabled";
  const wk = (args) => `data-act="dpweek" data-a="${esc(JSON.stringify(args))}"`;
  const pkgWord = { staff: "staff", base: "base", nickel: "nickel", dime: "dime" };
  const answers = x.answers.map((a) => {
    const pkg = a.pkg === "staff" ? a.staff.pkg : a.pkg;
    const man = a.man != null ? a.man : a.staff.man;
    const rush = a.rush != null ? a.rush : a.staff.rush;
    const mans = [0, 20, 40, 60, 80, 100];
    const rushes = [10, 20, 30, 40, 50, 60, 70];
    if (a.man != null && !mans.includes(a.man)) mans.push(a.man);
    if (a.rush != null && !rushes.includes(a.rush)) rushes.push(a.rush);
    const sel = (f, opts, cur) => `<select data-dpch="dpans" data-k="${a.k}" data-f="${f}"${dis}>${opts.map(([v, t]) => `<option value="${v}" ${String(v) === String(cur) ? "selected" : ""}>${esc(t)}</option>`).join("")}</select>`;
    return `<div class="dp-prow"><span><b>${esc(a.name)}</b><small>${a.share}%</small></span><span><small>They ${esc(a.does)}${a.snaps ? ` · ${a.snaps} snaps` : ""}</small><b class="us">${esc(dpCap(pkg))} · ${man}% man · ${rush}% five or more</b><span class="ctl">${sel("pkg", [["staff", `Staff: ${a.staff.pkg}`], ["base", "Base"], ["nickel", "Nickel"], ["dime", "Dime"]], a.pkg)}${sel("man", [["staff", `Staff: ${a.staff.man}% man`], ...mans.sort((p, q) => p - q).map((v) => [v, `${v}% man`])], a.man != null ? a.man : "staff")}${sel("rush", [["staff", `Staff: ${a.staff.rush}% 5+`], ...rushes.sort((p, q) => p - q).map((v) => [v, `${v}% five or more`])], a.rush != null ? a.rush : "staff")}</span></span></div>`;
  }).join("");
  const limOpts = [0, 15, 20, 25, 30, 40, 50];
  const limits = x.limits.map((l) => `<div class="dp-lrow"><span><b>${playerLink(l.id, l.name)} <small>· ${esc(l.pos)} · ${esc(l.practice)}${l.note ? ` · ${esc(l.note)}` : ""}</small></b><span class="ctl"><select class="dp-wsel" data-dpch="dplimit" data-id="${l.id}"${dis}>${[...new Set([...limOpts, l.limit || 0])].sort((p, q) => p - q).map((v) => `<option value="${v}" ${v === (l.limit || 0) ? "selected" : ""}>${v ? `${v} snaps` : "No limit"}</option>`).join("")}</select></span></span><span><b>${l.limit || "Full"}</b><small>${l.limit ? "snap limit" : "no limit"}</small></span></div>`).join("");
  const addable = x.two_deep.filter((p) => !x.limits.some((l) => l.id === p.id));
  const limitAdd = x.head && addable.length ? `<select class="dp-wsel" data-dpch="dplimitadd"><option value="">A limit for someone else…</option>${addable.map((p) => `<option value="${p.id}">${esc(p.name)} (${esc(p.pos)})</option>`).join("")}</select>` : "";
  const p1 = `<div class="panel"><div><div class="dp-sec"><b>Their personnel, our answer</b><small>${esc(x.opp_name)}${x.film_snaps ? ` · ${x.film_snaps} snaps of film` : " · no film yet"}</small></div>${answers}</div>
    <div><div class="dp-sec"><b>Snap limits</b><small>back from injury or banged up</small></div>${limits || '<div class="dp-foot">Everyone in the two-deep practised in full: no limit needed.</div>'}${limitAdd ? `<div style="padding-top:8px">${limitAdd}</div>` : ""}</div>
    <div><div class="dp-sec"><b>Matchups</b><small>the week's plan against ${esc(x.opp_name)}</small></div>${parts.matchups || ""}</div></div>`;
  const ifs = x.ifs.map((f) => `<div class="dp-if"><span><span><i>IF </i>${esc(f.when)}</span><b><i>THEN </i>${esc(f.then)}</b>${f.staff ? "<small>your staff's pick from their film</small>" : ""}</span><button class="dp-chipbtn ${f.on ? "on" : ""}" ${wk({ if: { k: f.k, on: !f.on } })}${dis}>${f.on ? "On" : "Off"}</button></div>`).join("");
  const orders = x.orders.map((o) => {
    const given = o.given.map((g) => `<span class="dp-tag gold">${esc(g.name)}${x.head ? ` <button ${wk({ order: { id: g.id, k: o.k, on: false } })} aria-label="Take it back">×</button>` : ""}</span>`).join(" ");
    const left = o.players.filter((p) => !o.given.some((g) => g.id === p.id));
    return `<div class="dp-st"><span>${esc(o.name)}<br><small>${esc(o.for)}</small></span><span>${given ? `<span class="dp-notes">${given}</span>` : '<small class="muted">nobody this week</small>'}${x.head && left.length ? `<select class="dp-wsel" data-dpch="dporder" data-k="${o.k}"><option value="">Give it to…</option>${left.map((p) => `<option value="${p.id}">${esc(p.name)} (${esc(p.pos)})</option>`).join("")}</select>` : ""}</span></div>`;
  }).join("");
  const checks = x.checks.map((c) => `<div class="dp-if"><span><b>${esc(c.text)}</b><small>${c.staff ? "on unless you take it off" : "off unless you put it on"}</small></span><button class="dp-chipbtn ${c.on ? "on" : ""}" ${wk({ check: { k: c.k, on: !c.on } })}${dis}>${c.on ? "On" : "Off"}</button></div>`).join("");
  const p2 = `<div class="panel"><div><div class="dp-sec"><b>If they do this, we go to</b><small>the play callers switch on the look</small></div>${ifs}</div>
    <div><div class="dp-sec"><b>Player instructions</b><small>for ${esc(x.opp_name)} only</small></div>${orders}</div>
    <div><div class="dp-sec"><b>The quarterback's checks</b><small>at the line, from what he sees</small></div>${checks}</div></div>`;
  const teams = DP_ST.map(([key, label, opts]) => `<div class="dp-st"><span>${label}</span><span><span class="dp-seg small">${opts.map((t, v) => `<button ${wk({ teams: { [key]: v } })} aria-pressed="${x.teams[key] === v}"${dis}>${t}</button>`).join("")}</span></span></div>`).join("");
  const keys = x.keys.length ? x.keys.map((k, i) => `<div class="dp-lk"><span>${esc(k.text)}</span><span>${i + 1}</span></div>`).join("") : `<div class="dp-foot">Your staff writes the keys on plan day, ${esc(x.keys_day)}.</div>`;
  const last = x.last_keys ? `<div><div class="dp-sec"><b>Last game's keys · ${esc(x.last_keys.opp)}</b><small>${x.last_keys.met} of ${x.last_keys.of} met</small></div>${x.last_keys.keys.map((k) => `<div class="dp-lk" title="${esc(k.what)}"><span>${esc(k.text)}<br><small class="muted">${esc(k.what)}</small></span><span class="${k.met ? "" : "miss"}">${k.met ? "✓" : "✗"}</span></div>`).join("")}</div>` : "";
  const r = x.week_rules;
  const rseg = (k, items) => `<span class="dp-seg small">${items.map(([v, t]) => `<button data-act="gd-rule" data-k="${k}" data-v="${v}" aria-pressed="${r[k] === v}">${t}</button>`).join("")}</span>`;
  const rules = `<div class="dp-rulesnote"><span class="dp-lbl">Game rules · ${x.rules_changed ? "changed for this game" : "from the identity"}</span><span>4th downs ${esc(DP_FOURTH[r.fourth])} · two-point ${esc(DP_TWO[r.two])} · ${esc(DP_TO[r.timeouts])} · four-minute: ${esc(DP_FM[r.four])} · ${esc(DP_TEMPO[r.tempo])}. Change any of them for this game only.</span>
    <div class="row"><small>4th downs</small>${rseg("fourth", [["staff", "Staff"], ["numbers", "Numbers"], ["aggressive", "Aggressive"], ["conservative", "Conservative"]])}</div>
    <div class="row"><small>Two-point</small>${rseg("two", [["staff", "Staff"], ["chart", "Chart"], ["kick", "Kick"]])}</div>
    <div class="row"><small>Tempo</small>${rseg("tempo", [["normal", "Normal"], ["hurry", "No-huddle"], ["milk", "Milk it"]])}</div>
    <div class="row"><small>Timeouts</small>${rseg("timeouts", [["staff", "Staff"], ["keep_two", "Keep two"], ["keep_one", "Keep one"], ["free", "Freely"]])}</div>
    <div class="row"><small>Four-minute</small>${rseg("four", [["staff", "Staff"], ["run", "Run it out"], ["attack", "Attack"]])}</div>
    <div class="row"><small>${x.rules_changed ? "Only this game: the standing rules come back after it." : "The standing rules and the 4th-down chart are on Plans."}</small><span class="dp-btns">${x.rules_changed ? '<button class="dp-chipbtn" data-act="gd-rulereset">Back to the standing rules</button>' : ""}<a class="link" data-go="plans" data-tab="Identity">Plans</a></span></div></div>`;
  const p3 = `<div class="panel"><div><div class="dp-sec"><b>Special teams</b><small>${x.teams.fg_range ? `field goal range ${esc(x.teams.fg_range)}` : "no kicker on the roster"}</small></div>${teams}<div class="dp-foot" style="padding-top:6px">Returners and the cover teams are on the depth chart's <a class="link" data-go="depth" data-tab="Kickoff return">return boards</a>.</div></div>
    <div><div class="dp-sec"><b>Keys to the game</b><small>checked after the final whistle</small></div>${keys}</div>${last}${rules}</div>`;
  const bar = `<div class="dp-ph" style="margin-bottom:12px"><span><span class="dp-pt">The week's plan</span><span class="dp-ps">against ${rich(x.opp)} · ${esc(x.game)}${x.locked ? " · locked" : ""}</span></span><span class="dp-btns"><button class="st-btn small glass" data-go="plans" data-tab="Call sheet">The call sheet${x.sheet.waiting ? ` · ${dpPlural(x.sheet.waiting, "part")} waiting` : x.sheet.drafted ? " · approved" : ""}</button>${x.head ? `<button class="st-btn small ${x.locked ? "glass" : "primary"}" ${wk({ lock: !x.locked })}>${x.locked ? "Unlock the plan" : "Lock the plan"}</button>` : ""}</span></div>`;
  return `<div class="dp">${bar}<div class="dp-cols three">${p1}${p2}${p3}</div>${x.head ? "" : '<div class="dp-foot" style="padding-top:10px">The head coach\'s plan: you see it and coach your room to it.</div>'}</div>`;
};
window.dpLikeUs = async (h) => {
  const k = keep("dplike", { side: "offense" });
  let x;
  try {
    x = await dpGet("dp_like_us", { side: k.side });
  } catch (e) {
    return "";
  }
  if (!x.has) return "";
  const def = x.side === "defense";
  const seg = dpSeg("dpkeep", [["offense", "Their defense"], ["defense", "Their offense"]], k.side, { key: "dplike", f: "side" });
  const head = `<div class="dp-ph"><span><span class="dp-pt">Teams like us</span><span class="dp-ps">${def ? "their offense against defenses like ours" : "their defense against offenses like ours"} · ${dpPlural(x.tapes, "tape")}, ${x.like_n} like us</span></span>${seg}</div>`;
  if (!x.ours || !x.games.length) {
    return `<div class="dp" style="margin-top:16px"><div class="panel">${head}<div class="empty">${!x.ours ? "Your own film is too thin yet to say which teams play like you: it fills in after your first games." : `No film of ${esc(x.opp_name)} yet this season.`}</div></div></div>`;
  }
  const games = x.games.map((g) => `<div class="dp-gm" style="${g.like ? "box-shadow:inset 0 0 0 1px var(--dp-gold-ring)" : ""}"><span><b>${rich(g.vs)}</b><i class="${g.won ? "" : "lost"}">${g.won ? "W" : "L"} ${esc(g.score)}</i></span><small>${esc(g.date)} · ${g.ypp} a play${g.man != null ? ` · ${g.man}% man` : ""}${g.rush != null ? ` · ${g.rush}% 5+` : ""}</small><span class="sim"><u><i style="width:${dpNum(g.sim)}%"></i></u><small>${g.sim}% like us</small></span></div>`).join("");
  const pos = (s) => { const v = parseFloat(s); return String(s).endsWith("%") ? Math.max(0, Math.min(100, v)) : Math.max(0, Math.min(100, (v / 12) * 100)); };
  const cmp = x.rows.length
    ? `<div class="dp-cmp th"><span>What they do</span><b>Usual</b><span></span><b>Like us</b></div>${x.rows.map((c) => { const a = pos(c.usual), b = pos(c.like); return `<div class="dp-cmp"><span>${esc(c.k)}<small>${esc(c.unit)}</small></span><b>${esc(c.usual)}</b><span class="tk"><u></u><s class="${c.big ? "big" : ""}" style="left:${Math.min(a, b)}%;width:${Math.abs(b - a)}%"></s><i class="a" style="left:${a}%"></i><i class="b ${c.big ? "big" : ""}" style="left:${b}%"></i></span><b class="${c.big ? "big" : ""}">${esc(c.like)}</b></div>`; }).join("")}`
    : `<div class="empty">Not enough snaps in the games like us to compare yet.</div>`;
  const worked = x.worked.length ? x.worked.map((w) => `<div class="dp-wl"><span>${esc(dpCap(w.name))}<small>${esc(w.look)} · ${dpPlural(w.n, "snap")}${w.on_sheet ? " · on your sheet" : ""}</small></span><b>${w.ypp} yds</b></div>`).join("") : '<div class="empty">Nothing stood out yet.</div>';
  const sug = x.suggest ? `<div class="dp-card"><span class="dp-lbl">${!x.by || plainText(x.by) === "you" ? "Your staff" : rich(x.by)} suggests</span><span>${esc(x.suggest.text)}</span>${x.head ? `<div class="dp-btns"><button class="st-btn small primary" data-act="dplike" data-name="${esc(x.suggest.name)}" data-look="${esc(x.suggest.look)}">Put it on the sheet</button></div>` : ""}</div>` : "";
  const half = x.half.length ? `<div><span class="dp-lbl">After halftime in those games</span>${x.half.map((t) => `<div class="dp-foot" style="padding-top:4px">${esc(t)}</div>`).join("")}</div>` : "";
  return `<div class="dp" style="margin-top:16px"><div class="panel">${head}<div class="dp-gms">${games}</div><div class="dp-cols"><div>${cmp}</div><div><div class="dp-sec"><b>${def ? "What held them" : "What worked for teams like us"}</b><small>yards a play</small></div>${worked}${sug}${half}</div></div></div></div>`;
};

// ======================================================================
// Fit with how we play (board 27): hiring, recruiting, the draft board
// ======================================================================
function dpFitStaffHtml(x) {
  if (!x.rows.length) return "";
  const cols = (x.rows[0].ticks || []).map((t) => t.k).slice(0, 3);
  const pad = (arr) => arr.concat(Array(Math.max(0, 3 - arr.length)).fill("<span></span>")).join("");
  const sym = { yes: "✓", part: "~", no: "✕" };
  const word = { yes: "fits", part: "in part", no: "against" };
  const rows = x.rows.slice(0, 15).map((r) => `<div class="dp-hrow"><span>${rich(r.link)}<small>${esc(r.calls)} · ${plainText(r.now)} · ${esc(r.rep)}</small></span>${pad((r.ticks || []).slice(0, 3).map((t) => `<span class="fx" title="${esc(t.why)}"><b class="${t.t}">${sym[t.t]}</b><small>${word[t.t]}</small></span>`))}<b class="${esc(r.fit)}">${esc(r.fit === "No guidelines" ? "–" : r.fit)}</b></div>`).join("");
  const against = x.against.length ? `Your guidelines: ${x.against.map((a) => `${a.k} ${a.v} (${DP_FIRM_WORDS[a.firm] || a.firm})`).join(" · ")}.` : "No guidelines on this side yet: set them on Plans, Identity, and every candidate is ticked against them.";
  return `<div class="dp dp-fitp"><div class="panel"><div class="dp-ph"><span><span class="dp-pt small">Fit with how we play</span><span class="dp-ps">${esc(against)}</span></span><a class="link" data-go="plans" data-tab="Identity">Identity</a></div>
    ${cols.length ? `<div class="dp-hrow th"><span>Candidate · what he calls</span>${pad(cols.map((c) => `<span style="text-align:center">${esc(c)}</span>`))}<b>Fit</b></div>${rows}` : ""}</div></div>`;
}
const dpPsBase = screens.people_staff;
if (dpPsBase) {
  screens.people_staff = async (a) => {
    const html = await dpPsBase(a);
    try {
      const t = S.keep.pstab && S.keep.pstab.t;
      const role = S.keep.pshire && S.keep.pshire.role;
      if (t !== "Hiring" || !["oc", "dc"].includes(role)) return html;
      const x = await api("dp_fit_staff", { role });
      const add = dpFitStaffHtml(x);
      return add ? html.replace(/<\/div>\s*$/, add + "</div>") : html;
    } catch (e) {
      return html;
    }
  };
}
async function dpFitRecruits() {
  const k = keep("dpfitr", { pos: "CB" });
  const x = await api("dp_fit_recruits", { pos: k.pos });
  if (x.kind === "none") return "";
  const seg = dpSeg("dpkeep", [["CB", "Corners"], ["S", "Safeties"]], k.pos, { key: "dpfitr", f: "pos" });
  const rows = x.rows.slice(0, 20).map((r) => `<div class="dp-rrow"><span>${playerLink(r.id, r.name)}<small>${esc(r.class)}${r.interest ? ` · ${esc(r.interest)}` : ""}</small></span>${range(r.read, null, { p: r.id, k: "ovr" })}${range(r.man, "press-man")}${range(r.zone, "zone")}<span><span class="dp-tag ${x.guideline && ((x.manish && /man/i.test(r.tag)) || (!x.manish && /zone/i.test(r.tag))) ? "gold" : ""}">${esc(r.tag)}</span></span><span>#${r.fit_rank}<small class="muted"> · talent #${r.talent_rank}</small></span></div>`).join("");
  const where = x.kind === "draft" ? "the draft board" : "your recruiting board";
  return `<div class="dp dp-fitp"><div class="panel"><div class="dp-ph"><span><span class="dp-pt small">Fit with how we play</span><span class="dp-ps">${x.guideline ? `your coverage guideline: ${esc(x.guideline)} · ${where} in its order` : esc(x.why)}</span></span>${seg}</div>
    ${rows ? `<div class="dp-rrow th"><span>Player</span><span>Overall</span><span>Press-man</span><span>Zone</span><span>Best in</span><span>Order</span></div>${rows}` : `<div class="empty">No ${k.pos === "S" ? "safeties" : "corners"} on ${where} yet.</div>`}
    <div class="dp-foot">${esc(x.why)} "Order" is the board with your guideline's weight; "talent" the order on ability alone.</div></div></div>`;
}
for (const name of ["recruit_board", "draftboard"]) {
  const base = screens[name];
  if (!base) continue;
  screens[name] = async (a) => {
    const html = await base(a);
    try {
      return html + (await dpFitRecruits());
    } catch (e) {
      return html;
    }
  };
}

// ======================================================================
// Clicks and changes
// ======================================================================
function dpRow(key) {
  return DP.id && DP.id.rows.find((r) => r.key === key);
}
// A pending identity change: kept until saved, dropped when it matches.
function dpPendSet(key, patch) {
  const r = dpRow(key);
  if (!r) return;
  const p = { ...(DP.pend[key] || {}), ...patch };
  const to = "to" in p ? p.to : r.value;
  const firm = p.firm || r.firm;
  if (to === r.value && (to == null || firm === r.firm)) delete DP.pend[key];
  else DP.pend[key] = p;
  DP.sel.idrow = key;
}
function dpSnap(r, v) {
  const s = r.step || 5;
  return Math.max(r.min, Math.min(r.max, Math.round(v / s) * s));
}
Object.assign(CORE_MORE, {
  dpkeep: (el) => {
    const d = el.dataset;
    const v = d.v === "true" ? true : d.v === "false" ? false : d.v;
    keep(d.key, {})[d.f] = v;
    DP.sel.play = null;
    DP.sel.tell = 0;
    refresh(el);
  },
  dpsel: (el) => {
    const d = el.dataset;
    const v = d.v === "" ? null : /^-?\d+$/.test(d.v) ? Number(d.v) : d.v;
    DP.sel[d.k] = DP.sel[d.k] === v && d.k === "play" ? null : v;
    dpLocal(el);
  },
  dpopt: (el) => {
    const r = dpRow(el.dataset.key);
    if (!r) return;
    dpPendSet(r.key, { to: el.dataset.v });
    dpLocal(el);
  },
  dpnud: (el) => {
    const r = dpRow(el.dataset.key);
    if (!r) return;
    const e = dpEff(r);
    dpPendSet(r.key, { to: dpSnap(r, (e.set ? e.to : r.own) + Number(el.dataset.d)) });
    dpLocal(el);
  },
  dpfirm: (el) => {
    const r = dpRow(el.dataset.key);
    if (!r) return;
    const e = dpEff(r);
    const f = el.dataset.v;
    if (!e.set) {
      // Nothing said yet: a firm or a lean starts from his own.
      if (f !== "theirs") dpPendSet(r.key, { to: r.kind === "track" ? dpSnap(r, r.own) : r.own, firm: f });
    } else dpPendSet(r.key, { firm: f });
    dpLocal(el);
  },
  dpclear: (el) => {
    dpPendSet(el.dataset.key, { to: null });
    dpLocal(el);
  },
  dpdrop: (el) => {
    DP.pend = {};
    dpLocal(el);
  },
  dpsave: async (el) => {
    const changes = Object.entries(DP.pend).map(([key, p]) => ({ key, ...("to" in p ? { to: p.to } : {}), ...(p.firm ? { firm: p.firm } : {}) }));
    if (!changes.length) return;
    const r = await dpDo("dp_guide", { changes }, el, "The identity is saved: your coordinators plan to it from this week.");
    if (r) {
      DP.pend = {};
      refresh(el);
    }
  },
  dptalk: (el) => dpDo("dp_talk", {}, el, "You talked the identity through with your coordinators: for a month a firm guideline against their views costs little trust."),
  dprule: async (el) => {
    const d = el.dataset;
    const own = DP.rules ? DP.rules.grid.reduce((n, row) => n + row.cells.filter((c) => c.own).length, 0) : 0;
    if (d.k === "fourth" && own && !(await sure("Start the chart over from the preset?", `Your own ${dpPlural(own, "square")} go: the preset fills the whole chart.`, "Start over"))) return;
    dpDo("dp_rules_set", { [d.k]: d.v }, el, "The standing rules are saved: game day starts from them.");
  },
  dpcell: (el) => {
    const r = Number(el.dataset.r), c = Number(el.dataset.c);
    DP.sel.cell = DP.sel.cell && DP.sel.cell[0] === r && DP.sel.cell[1] === c ? null : [r, c];
    dpLocal(el);
  },
  dpcellset: (el) => {
    const d = el.dataset;
    dpDo("dp_rules_set", { cell: [Number(d.r), Number(d.c), d.v || 0] }, el);
  },
  dpcellclear: async (el) => {
    if (!(await sure("Clear your own squares?", "The chart goes back to the preset everywhere.", "Clear them"))) return;
    dpDo("dp_rules_set", { clear_cells: true }, el);
  },
  dpwords: async (el) => {
    const d = el.dataset;
    const side = Number(d.side);
    if (d.undo) return dpDo("dp_words", { side, act: "undo" }, el, "The old words are back.");
    const name = side ? "defense" : "offense";
    if (!(await sure(`New call names for the ${name}?`, "Your players learn new words this week: about one extra bust in 70 snaps. Everyone who left with the old words stops reading your calls.", "Change them"))) return;
    dpDo("dp_words", { side, act: "change" }, el, `New ${name} words: the players learn them this week.`);
  },
  dpstudy: (el) => dpDo("dp_study", { side: el.dataset.side }, el, "Your staff ran the full study: every snap since spring."),
  dpedit: (el) => {
    const k = keep("dpsheet", { side: "offense", wet: false });
    const act = el.dataset.a;
    if (act === "cut") DP.sel.play = null;
    dpDo("dp_sheet_edit", { side: k.side, i: Number(el.dataset.i), act }, el, act === "cut" ? "Cut from the sheet." : act === "keep" ? "It stays on the sheet." : "");
  },
  dpnote: (el) => {
    const k = keep("dpsheet", { side: "offense", wet: false });
    const v = Number(el.dataset.v);
    const list = DP.notes[k.side] || (DP.notes[k.side] = []);
    const i = list.indexOf(v);
    if (i >= 0) list.splice(i, 1);
    else list.push(v);
    dpLocal(el);
  },
  dpapprove: (el) => {
    const k = keep("dpsheet", { side: "offense", wet: false });
    const part = el.dataset.part != null ? Number(el.dataset.part) : undefined;
    dpDo("dp_sheet_review", { side: k.side, approve: true, ...(part != null ? { part } : {}) }, el, part != null ? "Approved." : "The sheet is approved: it goes on the card as it stands.");
  },
  dpsendback: async (el) => {
    const k = keep("dpsheet", { side: "offense", wet: false });
    const notes = DP.notes[k.side] || [];
    if (!notes.length) return toast("Pick a note for your coordinator first.");
    const part = el.dataset.part != null ? Number(el.dataset.part) : undefined;
    const r = await dpDo("dp_sheet_review", { side: k.side, approve: false, notes, ...(part != null ? { part } : {}) }, el, "Sent back: your coordinator redrafts it with your notes.");
    if (r) {
      DP.notes[k.side] = [];
      DP.sel.play = null;
    }
  },
  dpaddplay: (el) => {
    const k = keep("dpsheet", { side: "offense", wet: false });
    const i = DP.sel.play;
    if (i == null) return toast("Pick a play in the box you want it in.");
    let play;
    if (k.side === "defense") {
      const a = keep("dpaddd", { coverage: 0, pressure: 0 });
      play = { coverage: Number(a.coverage), pressure: Number(a.pressure) };
    } else {
      const a = keep("dpadd", { personnel: "11", backfield: "shotgun", set: "doubles", kind: "pass", k: 0 });
      play = { kind: a.kind, k: Number(a.k), personnel: a.personnel, backfield: a.backfield, set: a.set };
    }
    dpDo("dp_sheet_edit", { side: k.side, i, act: "add", play }, el, "Added to the sheet.");
  },
  dpextra: (el) => {
    const d = el.dataset;
    const k = keep("dpinst", { unit: "offense" });
    dpDo("dp_install_extra", { id: Number(d.id), x: d.x, on: d.on === "1", unit: k.unit }, el);
  },
  dpweek: (el) => {
    let args;
    try {
      args = JSON.parse(el.dataset.a);
    } catch (e) {
      return;
    }
    dpDo("dp_week_set", args, el, args.lock === true ? "The plan is locked: your staff's checks and switches go in as they stand." : "");
  },
  dplike: (el) => dpDo("dp_like_us_add", { name: el.dataset.name, look: el.dataset.look }, el, "It's on the first-down list of the call sheet."),
});
document.addEventListener("change", (ev) => {
  const el = ev.target.closest && ev.target.closest("[data-dpch]");
  if (!el) return;
  const d = el.dataset;
  const v = el.value;
  switch (d.dpch) {
    case "dptrk": {
      const r = dpRow(d.key);
      if (!r) return;
      dpPendSet(r.key, { to: dpSnap(r, Number(v)) });
      dpLocal(el);
      break;
    }
    case "dpadd":
      keep("dpadd", {})[d.f] = v;
      dpLocal(el);
      break;
    case "dpaddd":
      keep("dpaddd", {})[d.f] = v;
      dpLocal(el);
      break;
    case "dpans":
      dpDo("dp_week_set", { answer: { k: Number(d.k), [d.f]: d.f === "pkg" ? v : v === "staff" ? null : Number(v) } }, el);
      break;
    case "dplimit":
      dpDo("dp_week_set", { limit: { id: Number(d.id), n: Number(v) } }, el);
      break;
    case "dplimitadd":
      if (v) dpDo("dp_week_set", { limit: { id: Number(v), n: 30 } }, el, "A 30-snap limit: change it on his line.");
      break;
    case "dporder":
      if (v) dpDo("dp_week_set", { order: { id: Number(v), k: Number(d.k), on: true } }, el);
      break;
    case "dpdepth":
      dpDo("dp_depth_set", { board: d.board, slot: Number(d.slot), n: Number(d.n), id: Number(v) }, el, "Depth chart saved.");
      break;
    case "dprot":
      dpDo("dp_rotation_set", { id: Number(d.id), role: Number(v) }, el, "Rotation saved.");
      break;
  }
});
Object.assign(CRUMBS, { plans: "Plans" });
