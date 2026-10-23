// Lane screens: training (docs/final-build/plan.md). Registers screens.<name>.
// Lane 11, training 6b and the plans (docs/design/training.md, screens
// topics 6 and 8, plays and tactics topics 3-8): the practice week and its
// rules, the year's blocks, players' plans, practice reports, injuries and
// rule cases; the Development tab on your players' pages; the plans (the
// season's lean, the call sheet, the playbook, installs) and the play
// designer; the squad in six views; the depth chart as formation boards
// with your position coaches' proposals and the packages. Your staff's
// reads are ranges or words, never a true number or a ceiling.
"use strict";

// ---------- shared pieces ----------
const TR = { week: null, tpl: {}, sel: null, drag: null, des: null, desDraw: null, pick: null };
const trScroll = () => ($(".view") ? $(".view").scrollTop : 0);
const trRedraw = () => render(trScroll());
const trCap = (s) => String(s || "").replace(/^./, (c) => c.toUpperCase());
const trMine = (a) => !a || !a.kind || (S.status && S.status.mine && S.status.mine.ref.kind === a.kind && S.status.mine.ref.id === Number(a.id));
const trLast = (n) => { const w = String(n || "").split(" ").filter((x) => !/^(Jr\.?|Sr\.?|II|III|IV|V)$/.test(x)); return w[w.length - 1] || n; };
const trPlural = (n, one, many) => `${n} ${n === 1 ? one : many || one + "s"}`;
async function trDo(cmd, args, said) {
  try {
    const r = await api(cmd, args);
    if (said) toast(typeof said === "function" ? said(r) : said);
    return r === undefined ? true : r;
  } catch (e) {
    toast(e.message || String(e));
    return null;
  }
}
function trGoto(list) {
  return gotoLine(list.filter((x) => x && x[1] !== S.route.screen).slice(0, 5));
}
const trSel = (act, opts, cur, attrs = "") => `<select class="tr-sel" data-act="${act}" ${attrs}>${opts.map(([v, t]) => `<option value="${esc(v)}" ${String(v) === String(cur) ? "selected" : ""}>${esc(t)}</option>`).join("")}</select>`;
const trDot = (s) => `<span class="dot ${s === "out" ? "bad" : s === "limited" ? "warn" : "good"}"></span>`;
function trStatus(r) {
  if (!r.practice || r.practice === "full") return `${trDot("full")}<span class="muted">Full</span>`;
  return `${trDot(r.practice)}${esc(trCap(r.practice))}${r.practice_why ? ` <small class="muted">· ${esc(r.practice_why)}</small>` : ""}`;
}
const trLoad = (r) => `<span class="tr-load ${r.spike ? "spike" : ""}" title="${esc(r.load_trend || "")}">${esc(r.load || "")}</span>${r.spike ? ' <span class="st-pill gold" title="His load jumped: the trainers are watching him">Watch</span>' : ""}`;
const trGrowth = (g) => (g ? `<span class="tr-grow g-${g.replace(/\s+/g, "-")}">${esc(g)}</span>` : '<span class="muted">No read yet</span>');

Object.assign(I, {
  plans: '<path d="M5 3h10l4 4v14H5z M15 3v4h4"/><circle cx="9" cy="12" r="1.6"/><path d="M13 11l3 3M16 11l-3 3M9 17c2 0 4-1 6-3"/>',
  design: '<path d="M4 20l4-1 11-11-3-3L5 16z M14 6l3 3"/>',
});
navAdd({ go: "plans", label: "Plans", icon: "plans", sec: "team", after: "training" });
navAdd({ go: "designer", label: "Play designer", icon: "design", sec: "team", after: "plans" });
Object.assign(CRUMBS, { plans: "Plans", designer: "Play designer", roster: "Squad", depth: "Depth chart", training: "Training" });
GOTO.training = () => [["Squad", "roster", { view: "Development" }], ["Depth chart", "depth"], ["Plans", "plans"], ["Staff", "staff"], ["Schedule", "schedule"]];
GOTO.plans = () => [["Game day", "gameday"], ["Play designer", "designer"], ["Training", "training"], ["Depth chart", "depth"], ["Schedule", "schedule"]];
GOTO.designer = () => [["Plans", "plans", { tab: "Playbook" }], ["Call sheet", "plans", { tab: "Call sheet" }], ["Training", "training"], ["Depth chart", "depth"]];
GOTO.depth = () => [["Squad", "roster"], ["Training", "training"], ["Plans", "plans"], ["Game day", "gameday"], ["Schedule", "schedule"]];
GOTO.roster = (a) => (trMine(a) ? [["Depth chart", "depth"], ["Training", "training"], ["Plans", "plans"], ["Staff", "staff"], ["Leaderboards", "leaders"]] : [["Schedule", "schedule", teamArgs(a)], ["Staff", "staff", teamArgs(a)], ["Team page", "team", teamArgs(a)], ["Leaderboards", "leaders"]]);
// These screens draw their own Go-to lines.
for (const s of ["training", "depth", "roster"]) {
  const i = GOTO_AFTER.indexOf(s);
  if (i >= 0) GOTO_AFTER.splice(i, 1);
}

// ======================================================================
// Training: the week, players, templates, the year, reports
// ======================================================================
const DAY_SHORT = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const GROUP_CLS = { field: "f", team: "t", body: "b", mind: "m", rest: "r" };
const trClone = (days) => JSON.parse(JSON.stringify(days.map((d) => ({ periods: d.periods, game: d.game, second: d.second }))));
// A day's sum, worked out in the page as you edit it.
function trDayFacts(d) {
  const min = d.periods.reduce((s, p) => s + Number(p.min || 0), 0);
  const field = d.periods.filter((p) => p.field !== false && !["weight_room", "meeting", "film", "recovery"].includes(p.kind)).reduce((s, p) => s + Number(p.min || 0), 0);
  const pads = d.periods.some((p) => p.gear === "pads");
  const full = d.periods.filter((p) => p.contact === "thud" || p.contact === "live").reduce((s, p) => s + Number(p.min || 0), 0);
  return { min, field, pads, full };
}
function periodChip(p, k, i, edit, x) {
  const kind = x.options.kinds.find((o) => o.kind === p.kind) || { label: p.label || p.kind, group: p.group };
  const gear = (x.options.gears.find((g) => g.v === p.gear) || {}).t || p.gear;
  const contact = (x.options.contacts.find((c) => c.v === p.contact) || {}).t || p.contact;
  const sit = p.kind === "situations" ? x.options.situations[p.what || 0] : "";
  const sel = TR.sel && TR.sel.d === k && TR.sel.i === i;
  return `<button class="tr-per g-${GROUP_CLS[kind.group] || "f"} ${sel ? "on" : ""}" ${edit ? `data-act="tr-per" data-d="${k}" data-i="${i}"` : "disabled"} title="${esc(kind.grows || "")}">
    <b>${esc(kind.label)}</b><span>${p.min} min${p.kind === "weight_room" || p.kind === "meeting" || p.kind === "film" || p.kind === "recovery" || p.kind === "walkthrough" ? "" : ` · ${esc(gear)} · ${esc(contact)}`}${sit ? ` · ${esc(sit)}` : ""}${p.who && p.who !== "all" ? ` · ${p.who === "ones" ? "two-deep" : "young"}` : ""}</span></button>`;
}
function periodEditor(p, k, i, x, allowed) {
  const o = x.options;
  const row = (label, html, note) => `<label>${label}${note ? `<small>${esc(note)}</small>` : ""}</label>${html}`;
  const kinds = o.kinds.map((q) => [q.kind, q.label]);
  return `<div class="panel warm tr-edit"><h2>${esc(DAY_SHORT[k])} · period ${i + 1} <small>${esc((o.kinds.find((q) => q.kind === p.kind) || {}).grows || "")}</small></h2>
    <div class="gd-form">
      ${row("What", trSel("tr-pf", kinds, p.kind, 'data-f="kind"'))}
      ${row("Minutes", `<input class="tr-num" type="number" min="5" max="180" step="5" value="${p.min}" data-act="tr-pf" data-f="min">`)}
      ${row("Gear", trSel("tr-pf", o.gears.map((g) => [g.v, g.t]), p.gear, 'data-f="gear"'), allowed ? `today's rule: up to ${allowed.gear}` : "")}
      ${row("Contact", trSel("tr-pf", o.contacts.map((c) => [c.v, c.t]), p.contact, 'data-f="contact"'), allowed ? `today's rule: up to ${allowed.contact}` : "")}
      ${row("Tempo", trSel("tr-pf", o.tempos, p.tempo || "normal", 'data-f="tempo"'))}
      ${row("Who", trSel("tr-pf", o.whos, p.who || "all", 'data-f="who"'))}
      ${p.kind === "situations" ? row("Situation", trSel("tr-pf", o.situations.map((s, n) => [n, s]), p.what || 0, 'data-f="what"')) : ""}
    </div>
    <div class="st-actions"><button class="st-btn small glass" data-act="tr-pmove" data-dir="-1" ${i === 0 ? "disabled" : ""}>Earlier</button><button class="st-btn small glass" data-act="tr-pmove" data-dir="1">Later</button><button class="st-btn small glass" data-act="tr-pdup">Copy it</button><button class="st-btn small glass" data-act="tr-pdel">Remove</button><button class="st-btn small" data-act="tr-pdone">Done</button></div></div>`;
}
// The seven days side by side. `mode` is "week" or a template slot.
function weekBoard(days, x, mode, info) {
  const edit = !!info.edit;
  const cols = days.map((d, k) => {
    const f = trDayFacts(d);
    const meta = info.days ? info.days[k] : null;
    const rule = meta && meta.rule ? `<div class="tr-rule" title="${esc(meta.rule)}">${esc(meta.rule)}</div>` : "";
    const head = `<div class="tr-dh ${meta && meta.today ? "today" : ""} ${meta && meta.past ? "past" : ""}"><b>${esc(meta ? meta.weekday : DAY_SHORT[k])}</b><small>${esc(meta ? meta.date : "")}</small></div>`;
    if (d.game) return `<div class="tr-day game">${head}<div class="tr-game">Game day</div>${rule}</div>`;
    const list = d.periods.map((p, i) => periodChip(p, k, i, edit, x)).join("");
    const empty = !d.periods.length ? `<div class="tr-off">${meta && meta.allowed && !meta.allowed.coaches ? "No coaches allowed" : "Day off"}</div>` : "";
    const sum = d.periods.length ? `<div class="tr-sum"><span>${f.min} min</span>${f.field ? `<span>${f.field} on the field</span>` : ""}${f.pads ? '<span class="pads">pads</span>' : ""}${f.full ? `<span class="hit">${f.full} full contact</span>` : ""}</div>` : "";
    const add = edit ? `<button class="tr-add" data-act="tr-padd" data-d="${k}">+ Period</button>` : "";
    return `<div class="tr-day ${TR.sel && TR.sel.d === k ? "sel" : ""}">${head}${list}${empty}${add}${sum}${rule}</div>`;
  }).join("");
  return `<div class="tr-week">${cols}</div>`;
}
function trDraft(x) {
  // The week you're editing stays in the page until you save or leave the week.
  if (!TR.week || TR.week.monday !== x.monday) TR.week = { monday: x.monday, days: trClone(x.days), dirty: false };
  return TR.week;
}
function workBars(x) {
  const max = Math.max(1, ...x.work.map((w) => Math.max(w.mine, w.staff)));
  return x.work.map((w) => `<div class="tr-wk"><span>${esc(w.family)}</span><div class="tr-wbar"><i class="mine" style="width:${(100 * w.mine) / max}%"></i><i class="staff" style="left:${(100 * w.staff) / max}%"></i></div><em class="${w.lean}">${w.lean === "more" ? "more than the staff's" : w.lean === "less" ? "less" : w.lean === "none" ? "none this week" : "as the staff"}</em></div>`).join("");
}
function weekTab(x) {
  const draft = trDraft(x);
  const own = x.own;
  const edit = x.head && !own;
  const sel = TR.sel && draft.days[TR.sel.d] && draft.days[TR.sel.d].periods[TR.sel.i] ? TR.sel : null;
  const tiles = `<div class="st-tiles tr-tiles">
    <div><b>${esc(x.block_label)}</b><small>${esc(x.slot_label)}</small></div>
    ${x.counts.map((c) => `<div><b>${esc(c.split(" ")[0])}<i> / ${esc(c.split(" ")[2])}</i></b><small>${esc(c.split(" ").slice(3).join(" "))} this block</small></div>`).join("")}
    <div><b>${x.load.full}<i> / ${x.load.staff_full}</i></b><small>full-contact minutes, yours / the staff's</small></div>
    <div><b>${x.prep.mine}<i> / ${x.prep.staff}</i></b><small>minutes on the plan</small></div>
    ${x.fresh != null ? `<div><b>${x.fresh}%</b><small>fresh legs on game day</small></div>` : ""}</div>`;
  const breaches = x.breaches.length ? `<div class="panel tr-breach"><h2>Outside the rules <small>${x.breaches.length}</small></h2><div class="items">${x.breaches.map((b) => `<div class="item"><span class="when">${esc(b.day || "Block")}</span><span>${esc(b.text)}</span></div>`).join("")}</div></div>` : "";
  const status = own
    ? `<p class="sub">${esc(x.block_label)}: your staff can't run practice now. Players work on their own; the strength staff's numbers and the facility log say who is doing it.</p>`
    : `<p class="sub">${x.yours ? (x.pushed ? "Your week, saved outside the rules: a staff member or a player could report it." : "Your week. ") : "Your staff's week for this block, from their style. Change any period; the staff keeps the rest. "}${x.fitted ? `Your staff trimmed ${trPlural(x.fitted, "thing")} to keep it inside the rules. ` : ""}${x.head ? "" : "The head coach runs practice; you see his week."}</p>`;
  const actions = edit
    ? `<div class="st-actions"><button class="st-btn primary small" data-act="tr-wsave" ${draft.dirty ? "" : "disabled"}>Save this week</button>${draft.dirty ? '<button class="st-btn small glass" data-act="tr-wundo">Undo changes</button>' : ""}${x.yours ? '<button class="st-btn small glass" data-act="tr-wreset">Give it back to the staff</button>' : ""}<button class="st-btn small glass" data-act="tr-wtpl" title="Keep this week as the template for every ${esc(x.slot_label.toLowerCase())}">Keep as the ${esc(x.slot_label.toLowerCase())} template</button></div>`
    : "";
  const editor = sel ? periodEditor(draft.days[sel.d].periods[sel.i], sel.d, sel.i, x, x.days[sel.d].allowed) : "";
  return `${tiles}${status}${actions}${weekBoard(draft.days, x, "week", { edit, days: x.days })}${editor}${breaches}
    <div class="grid2"><div class="panel warm"><h2>The week's work <small>by family, against your staff's own week</small></h2>${workBars(x)}<p class="sub">Growth leans the way the minutes go, a little: the most a plan moves a family is about a seventh, and nobody grows past what his body allows.</p></div>
    <div class="panel warm"><h2>Why your staff plans it this way <small>from your style</small></h2><div class="items">${x.reasons.map((r) => `<div class="item story"><span>${esc(r)}</span></div>`).join("") || '<div class="empty">No notes.</div>'}</div></div></div>`;
}
function playersTab(x) {
  const cols = [
    { k: "name", t: "Name", l: true, f: (r) => playerLink(r.id, r.name, "Development") },
    { k: "pos", t: "Pos", l: true },
    { k: "class", t: "Class", l: true, v: (r) => classRank(r.class) },
    { k: "role", t: "Role", l: true },
    { k: "ovr", t: "Rating", div: true, v: (r) => mid(r.ovr), f: (r) => range(r.ovr) },
    { k: "growth", t: "Growth", l: true, tip: "Your position coach's read of how he's coming along, in words", f: (r) => trGrowth(r.growth) },
    { k: "grade_words", t: "Practice", l: true, tip: "His practice grades against the room", v: (r) => r.grade },
    { k: "practice", t: "Status", l: true, f: trStatus },
    { k: "load", t: "Load", l: true, div: true, f: trLoad, v: (r) => ["light", "moderate", "heavy", "very heavy"].indexOf(r.load) },
    { k: "load_trend", t: "Trend", l: true, hide: true },
    { k: "focus", t: "Main focus", l: true, div: true, v: (r) => r.plan.focus_label || "", f: (r) => `<select class="tr-sel" data-act="tr-focus" data-id="${r.id}" ${r.may_plan === false ? 'disabled title="The head coach plans him"' : ""}><option value="">${esc(r.plan.staff_focus ? "Staff: " + r.plan.staff_focus : "Balanced")}</option>${r.plan.options.map((o) => `<option value="${o.key}" ${o.key === r.plan.focus ? "selected" : ""}>${esc(o.label)}</option>`).join("")}</select>` },
    { k: "habit", t: "Habit", l: true, v: (r) => (r.plan.habit ? r.plan.habit.label : ""), f: (r) => (r.plan.habit ? esc(r.plan.habit.label) : '<span class="muted">-</span>') },
    { k: "body", t: "Body", l: true, v: (r) => (r.plan.body ? r.plan.body.target : 0), f: (r) => (r.plan.body ? `${r.wt} → ${r.plan.body.target} lb` : '<span class="muted">-</span>') },
    { k: "moving", t: "Learning", l: true, v: (r) => (r.plan.moving ? r.plan.moving.to : ""), f: (r) => (r.plan.moving ? `${esc(r.plan.moving.to)} <small class="muted">${r.plan.moving.progress}%</small>` : '<span class="muted">-</span>') },
    { k: "extra", t: "Extra", l: true, hide: true, v: (r) => Object.entries(r.plan.extra).filter(([, v]) => v).map(([k]) => k).join(", ") },
    { k: "rest", t: "Rest days", hide: true, v: (r) => r.plan.rest },
    { k: "sport", t: "Other sport", l: true, hide: x.tier !== "high school", v: (r) => r.sport || "" },
  ];
  const set = x.players.filter((r) => r.plan.focus).length;
  return `<p class="sub">Every player's plan. A main focus takes the bigger share of his individual periods; how much sticks depends on his coachability and who teaches him. Open a player's Development tab for his habit, body, a new position, extra work and rest. ${set} of ${x.players.length} have a focus you set.</p>
    ${table("trplayers", cols, x.players, { groupBy: (r) => r.pos })}`;
}
function templatesTab(x) {
  const k = keep("trtpl", { slot: x.slot });
  const t = x.templates.find((s) => s.slot === k.slot) || x.templates[0];
  if (!t) return '<div class="empty">No templates at this level.</div>';
  if (!TR.tpl[t.slot]) TR.tpl[t.slot] = { days: trClone(t.days), dirty: false };
  const draft = TR.tpl[t.slot];
  const edit = x.head;
  const sel = TR.sel && TR.sel.tpl === t.slot && draft.days[TR.sel.d] && draft.days[TR.sel.d].periods[TR.sel.i] ? TR.sel : null;
  return `<p class="sub">A template is the week your staff runs whenever that kind of week comes round. The rules of the day still apply: your staff trims anything a day doesn't allow.</p>
    <div class="seg tr-slots">${x.templates.map((s) => `<button data-act="tr-tslot" data-v="${s.slot}" aria-pressed="${s.slot === t.slot}">${esc(s.label)}${s.yours ? " ●" : ""}</button>`).join("")}</div>
    ${edit ? `<div class="st-actions"><button class="st-btn primary small" data-act="tr-tsave" data-slot="${t.slot}" ${draft.dirty ? "" : "disabled"}>Save the template</button>${draft.dirty ? `<button class="st-btn small glass" data-act="tr-tundo" data-slot="${t.slot}">Undo changes</button>` : ""}${t.yours ? `<button class="st-btn small glass" data-act="tr-treset" data-slot="${t.slot}">Use the staff's</button>` : ""}</div>` : ""}
    ${weekBoard(draft.days, x, t.slot, { edit })}${sel ? periodEditor(draft.days[sel.d].periods[sel.i], sel.d, sel.i, x, null) : ""}`;
}
function yearTab(x) {
  return `<div class="panel warm"><h2>The year's blocks <small>what the rules allow in each, at your level and in this era</small></h2><div class="tr-blocks">${x.blocks.map((b) => `<div class="tr-blk ${b.now ? "now" : ""} ${b.own ? "own" : ""}"><b>${esc(b.label)}</b><small>${esc(b.from)} – ${esc(b.to)}</small>${b.now ? '<span class="st-pill live">Now</span>' : ""}${b.own ? '<em>on their own</em>' : ""}</div>`).join("")}</div></div>
    <div class="grid2"><div class="panel warm"><h2>This block's limits</h2><div class="kv">${Object.entries(x.limits).filter(([, v]) => v != null).map(([k, v]) => `<dt>${esc(trCap(k))}</dt><dd>${v}</dd>`).join("") || "<dt>Limits</dt><dd>No count limits in this block</dd>"}</div><div class="items">${x.counts.map((c) => `<div class="item story"><span>${esc(c)}</span></div>`).join("")}</div></div>
    <div class="panel warm"><h2>Each day's rule this week</h2><div class="items">${x.days.map((d) => `<div class="item"><span class="when">${esc(d.weekday.slice(0, 3))}</span><span>${esc(d.rule || "No rule")}${d.allowed.coaches ? ` <small class="muted">· up to ${esc(d.allowed.gear)}, ${esc(d.allowed.contact)}${d.allowed.day_min ? `, ${d.allowed.day_min} min` : ""}</small>` : ""}</span></div>`).join("")}</div></div></div>`;
}
function reportsTab(x) {
  const log = x.log.map((l) => `<div class="panel warm"><h2>Week of ${esc(l.monday)} <small>${esc(l.block || "")} · ${l.practices} practices · ${l.minutes} min on the field · ${l.snaps} team snaps${l.injuries ? ` · ${l.injuries} hurt` : ""}</small></h2>
    <div class="grid2"><div><div class="st-label">Best practice grades</div><div class="items">${l.best.map((b) => `<div class="item"><span class="when">${b.grade}</span><span>${rich(b.link)}</span></div>`).join("") || '<div class="empty">No grades.</div>'}</div></div>
    <div><div class="st-label">Battles</div><div class="items">${l.battles.map((b) => `<div class="item story"><span>${rich(b.a)} <b>${b.aw}-${b.bw}</b> ${rich(b.b)}</span></div>`).join("") || '<div class="empty">No battles this week.</div>'}</div></div></div></div>`).join("");
  const reports = x.reports.map((r) => `<div class="panel warm"><h2>${esc(r.block)} report <small>${esc(r.date)}</small></h2><div class="items">${r.lines.map(([a, b]) => `<div class="item"><span class="when">${esc(a)}</span><span>${rich(b)}</span></div>`).join("")}</div></div>`).join("");
  const hurt = x.hurt.length ? `<div class="panel warm"><h2>Hurt in practice and off the field <small>${x.hurt.length}</small></h2><div class="items">${x.hurt.map((h) => `<div class="item"><span class="when">${esc(h.from)}</span><span>${rich(h.link)}: ${esc(h.what)}${h.how && h.how !== h.what ? ` <small class="muted">(${esc(h.how)})</small>` : ""} · back about ${esc(h.until)}</span></div>`).join("")}</div></div>` : "";
  const cases = x.cases.length ? `<div class="panel tr-breach"><h2>Rule cases <small>practice outside the rules that came out</small></h2><div class="items">${x.cases.map((c) => `<div class="item"><span class="when">${esc(c.date)}</span><span><b>${esc(c.what)}</b> ${esc(c.how)}. ${esc(c.penalty)}</span></div>`).join("")}</div></div>` : "";
  const notes = `<div class="panel warm"><h2>Staff notes <small>the trainers, the strength staff and your position coaches</small></h2>${x.notes.length ? `<div class="items">${x.notes.map((n) => `<div class="item"><span class="when">${esc(n.date)}</span><span><b>${esc(n.dept)}</b> ${rich(n.text)}</span></div>`).join("")}</div>` : '<div class="empty">No notes yet. They come in after each week.</div>'}</div>`;
  return `${cases}${hurt}${notes}${reports}${log || '<div class="empty">No practice weeks logged yet this year.</div>'}`;
}
screens.training = async (a) => {
  const x = await api("practice");
  setColors(x.team.colors);
  title("Training", x.block_label);
  const list = ["This week", "Players", "Templates", "The year", "Reports"];
  const tab = tabOf("trtab", list, a);
  const body = tab === "This week" ? weekTab(x) : tab === "Players" ? playersTab(x) : tab === "Templates" ? templatesTab(x) : tab === "The year" ? yearTab(x) : reportsTab(x);
  const head = pageHead({
    kicker: `${esc(trCap(x.tier))} · ${esc(x.block_label)} · week of ${esc(x.monday)}`,
    title: "Training",
    sub: x.own ? "The players are on their own: the rules keep your staff away." : `${esc(x.slot_label)}. ${x.yours ? "You set this week." : "Your staff's week."}`,
    goto: trGoto([["Squad", "roster", { view: "Development" }], ["Depth chart", "depth"], ["Plans", "plans"], ["Staff", "staff"], ["Schedule", "schedule"]]),
  });
  return `<div class="film"><div class="pad tr-pad">${head}${tabs("trtab", list.map((t) => (t === "Reports" && x.cases.length ? { name: t, badge: x.cases.length } : t)), tab)}<div class="tabbody">${body}</div></div></div>`;
};

// ======================================================================
// The Development tab on your players' pages (L8)
// ======================================================================
PLAYER_TABS.push({
  name: "Development",
  after: "Stats",
  show: (p) => !!p.mine,
  draw: async (p) => {
    const d = await api("player_dev", { id: p.id });
    if (!d.mine) return '<div class="empty">Only your own players have a development plan.</div>';
    const pl = d.plan;
    const dis = d.may_plan ? "" : 'disabled title="The head coach plans him"';
    const t = d.teacher;
    const teacher = t.name ? `${t.you ? "You" : coachLink(t.id, t.name)}, ${esc(t.title)}${t.you ? "" : ` · ${esc(t.words)}`}` : "Nobody on the staff";
    const key = d.key.map((k) => `<div class="attr ${k.focus ? "tr-focus" : ""}"><span>${esc(k.label)} <small class="muted">${esc(k.family)}</small>${k.focus ? ' <span class="tag soft">Focus</span>' : ""}</span>${range(k.read, "your position coach")}</div>`).join("");
    const habit = d.habit ? `<div class="attr"><span>${esc(d.habit.label)} <small class="muted">now</small></span>${range(d.habit.read)}</div><div class="sub">Coaching it toward ${d.habit.target_read} of 99: a habit moves slowly, faster with a good teacher and a coachable player.</div>` : "";
    const ex = pl.extra;
    const plan = `<div class="gd-form">
      <label>Main focus<small>${pl.focus_by === "you" ? "your pick" : pl.staff_focus ? `staff's pick: ${esc(pl.staff_focus)}` : "balanced"}</small></label>
      <select class="tr-sel" data-act="tr-dev" data-id="${d.id}" data-f="focus" ${dis}><option value="">${esc(pl.staff_focus ? "Leave it to the staff (" + pl.staff_focus + ")" : "Balanced")}</option>${pl.options.map((o) => `<option value="${o.key}" ${o.key === pl.focus ? "selected" : ""}>${esc(o.label)}</option>`).join("")}</select>
      <label>A habit to coach<small>how often he tries something, not how well</small></label>
      <div class="tr-row"><select class="tr-sel" data-act="tr-dev" data-id="${d.id}" data-f="habit" ${dis}><option value="">None</option>${d.habits.map((h) => `<option value="${h.k}" ${pl.habit && pl.habit.k === h.k ? "selected" : ""}>${esc(h.label)}</option>`).join("")}</select>
      ${pl.habit ? `<select class="tr-sel" data-act="tr-dev" data-id="${d.id}" data-f="habit_target" ${dis}>${[[4, "Much less often"], [7, "Less often"], [13, "More often"], [16, "Much more often"]].map(([v, l]) => `<option value="${v}" ${pl.habit.target === v ? "selected" : ""}>${l}</option>`).join("")}</select>` : ""}</div>
      <label>Body<small>${d.height}, ${d.weight} lb now</small></label>
      <div class="tr-row"><input class="tr-num" type="number" min="150" max="380" step="1" placeholder="Target lb" value="${pl.body ? pl.body.target : ""}" data-act="tr-dev" data-id="${d.id}" data-f="body" ${dis}>
      <select class="tr-sel" data-act="tr-dev" data-id="${d.id}" data-f="body_emph" ${dis}>${[["strength", "Strength first"], ["speed", "Keep his speed"], ["lean", "Lean mass"]].map(([v, l]) => `<option value="${v}" ${pl.body && pl.body.emphasis === v ? "selected" : ""}>${l}</option>`).join("")}</select></div>
      <label>Learn a new position<small>${pl.moving ? `${esc(pl.moving.to)}: ${pl.moving.progress}% of the way` : "reps at another spot in individual periods"}</small></label>
      <select class="tr-sel" data-act="tr-dev" data-id="${d.id}" data-f="moving" ${dis}><option value="">Stay at ${esc(d.pos)}</option>${d.positions.filter((x) => x !== d.pos).map((x) => `<option ${pl.moving && pl.moving.to === x ? "selected" : ""}>${esc(x)}</option>`).join("")}</select>
      <label>Extra work<small>on top of the team's week; it adds load</small></label>
      <div class="tr-checks">${[["film", 1, "Extra film"], ["work", 2, "Extra technique"], ["lift", 4, "Extra lifting"]].map(([k, bit, l]) => `<label class="gd-check"><input type="checkbox" data-act="tr-dev" data-id="${d.id}" data-f="extra" data-bit="${bit}" ${ex[k] ? "checked" : ""} ${dis}> ${l}</label>`).join("")}</div>
      <label>Rest<small>days a week he sits out practice</small></label>
      <select class="tr-sel" data-act="tr-dev" data-id="${d.id}" data-f="rest" ${dis}>${[[0, "Practises every day"], [1, "One rest day"], [2, "Two rest days"], [3, "Three rest days"]].map(([v, l]) => `<option value="${v}" ${pl.rest === v ? "selected" : ""}>${l}</option>`).join("")}</select>
    </div>`;
    const notes = d.notes.length ? `<div class="items">${d.notes.map((n) => `<div class="item"><span class="when">${esc(n.date)}</span><span><b>${esc(n.dept)}</b> ${rich(n.text)}</span></div>`).join("")}</div>` : '<div class="empty">No staff notes on him yet.</div>';
    const log = d.practice_log.length ? `<div class="items">${d.practice_log.map((l) => `<div class="item"><span class="when">${esc(l.monday)}</span><span>${l.grade != null ? `Practice grade <b>${l.grade}</b>. ` : ""}${l.battles.map(rich).join(" · ")}</span></div>`).join("")}</div>` : '<div class="empty">No practice reps logged yet.</div>';
    return `<div class="st-tiles tr-tiles"><div><b>${trGrowth(d.growth)}</b><small>your position coach's read this week</small></div><div><b class="tr-w">${esc(d.year)}</b><small>the year's development</small></div><div><b class="tr-w">${esc(trCap(d.grade.words))}</b><small>practice${d.grade.value ? ` · grade ${d.grade.value}` : ""}</small></div><div><b class="tr-w">${esc(trCap(d.load.level))}</b><small>load · ${esc(d.load.trend)}</small></div><div><b class="tr-w">${esc(d.role)}</b><small>in practice</small></div></div>
      <div class="grid2"><div class="panel warm"><h2>His plan <small>${d.may_plan ? "yours to set" : "the head coach's"}</small></h2>${plan}</div>
      <div class="sc-col"><div class="panel warm"><h2>Who teaches him</h2><p class="bigline tr-teach">${teacher}</p><div class="kv"><dt>Practice</dt><dd>${trDot(d.practice.status)}${esc(trCap(d.practice.status))}${d.practice.why ? ` · ${esc(d.practice.why)}` : ""}</dd><dt>Mood</dt><dd>${esc(d.mood)}</dd>${d.sport ? `<dt>Other sport</dt><dd>${esc(trCap(d.sport.name))}${d.sport.now ? " · in season now" : ""}</dd>` : ""}</div></div>
      <div class="panel warm"><h2>What matters at ${esc(d.pos)} <small>your position coach's read, as ranges</small></h2>${key}${habit}<p class="sub">Your staff reads where he is, not where he could end up: nobody sees a ceiling.</p></div></div></div>
      <div class="grid2"><div class="panel warm"><h2>Staff notes</h2>${notes}</div><div class="panel warm"><h2>Practice <small>grades and one-on-ones</small></h2>${log}</div></div>`;
  },
});

// ======================================================================
// Plans: the game plan, the call sheet, the playbook, installs
// ======================================================================
const trInstall = (v, words) => `<span class="tr-inst" title="${esc(words)}"><span class="t"><i style="width:${Math.max(2, v)}%"></i></span><b>${esc(words)}</b></span>`;
function planTabHtml(x) {
  const lean = Math.round(x.lean * 100);
  const stick = Math.round(x.stick * 100);
  return `<div class="grid2"><div class="panel warm"><h2>The season's lean <small>what your play caller reaches for</small></h2>
      <div class="gd-form stack"><label>Run or pass<small>${lean === 0 ? "As the play caller likes" : lean > 0 ? `${lean} points more passing` : `${-lean} points more running`}</small></label>
      <input type="range" class="tr-range" min="-20" max="20" step="1" value="${lean}" data-act="tr-lean" ${x.head ? "" : "disabled"}>
      <label>Stick to the call sheet<small>${stick}% of snaps from the card when a play fits the situation; the rest your play caller's own read</small></label>
      <input type="range" class="tr-range" min="0" max="100" step="5" value="${stick}" data-act="tr-stick"></div></div>
    <div class="panel warm"><h2>${x.next ? "The next game" : "No game ahead"} <small>${esc(x.next_date || "")}</small></h2>
      ${x.next ? `<p class="bigline">${rich(typeof x.next === "string" ? x.next : x.next.text || "")}</p>` : '<p class="sub">The plan carries over to the next game on the schedule.</p>'}
      <p class="sub">The week's matchups (whose corner to throw at, bracketing, the spy) and your game rules are on the game day screen. The call sheet below is what your play caller has on his card.</p>
      <div class="st-actions"><button class="st-btn small" data-go="gameday" data-tab="Game plan">The week's game plan</button><button class="st-btn small glass" data-go="designer">Draw a play</button></div></div></div>`;
}
function sheetTabHtml(x) {
  const k = keep("trsheet", { sit: 0 });
  const sit = x.sheet[k.sit] || x.sheet[0];
  const add = keep("tradd", { personnel: "11", backfield: "shotgun", set: "doubles", kind: "pass", play: x.passes[0].name });
  const playOpts = add.kind === "run" ? x.runs.map((r) => [r.name, `${trCap(r.name)} · ${r.family}`]) : add.kind === "design" ? x.designs.map((d) => [d.k, d.design.name]) : x.passes.map((p) => [p.name, `${trCap(p.name)} · ${p.family}`]);
  const list = sit.plays.map((p, i) => `<tr><td class="l">${i + 1}</td><td class="l"><b>${esc(trCap(p.name))}</b> <small class="muted">${esc(p.kind)} · ${esc(p.family)}</small></td><td class="l">${esc(p.personnel)} · ${esc(p.backfield)} · ${esc(p.set)}</td><td>${trInstall(p.install, p.words)}</td><td><button class="sc-mini" data-act="tr-sup" data-i="${i}" ${i === 0 ? "disabled" : ""}>↑</button> <button class="sc-mini" data-act="tr-sdel" data-i="${i}">×</button></td></tr>`).join("");
  return `<div class="tr-sheet"><div class="tr-sits">${x.sheet.map((s) => `<button class="tr-sit" data-act="tr-sit" data-k="${s.k}" aria-pressed="${s.k === sit.k}"><b>${esc(s.name)}</b><small>${trPlural(s.plays.length, "play")}</small></button>`).join("")}</div>
    <div><div class="panel warm"><h2>${esc(sit.name)} <small>${sit.plays.length ? "your play caller reaches for these first, top to bottom" : "empty: your play caller calls his own"}</small></h2>
      <div class="tbl"><table><thead><tr><th class="l">#</th><th class="l">Play</th><th class="l">Personnel · backfield · set</th><th>Install</th><th></th></tr></thead><tbody>${list || '<tr><td class="l" colspan="5"><div class="empty">No plays on the card for this situation.</div></td></tr>'}</tbody></table></div>
      <div class="tr-addrow">${trSel("tr-add", x.personnels.map((p) => [p, p + " personnel"]), add.personnel, 'data-f="personnel"')}${trSel("tr-add", x.backfields.map((b) => [b, trCap(b)]), add.backfield, 'data-f="backfield"')}${trSel("tr-add", x.sets.map((s) => [s, trCap(s)]), add.set, 'data-f="set"')}${trSel("tr-add", [["pass", "Pass"], ["run", "Run"], ...(x.designs.length ? [["design", "Your design"]] : [])], add.kind, 'data-f="kind"')}${trSel("tr-add", playOpts, add.play, 'data-f="play"')}<button class="st-btn small primary" data-act="tr-sadd" ${sit.plays.length >= 12 ? "disabled" : ""}>Add to the card</button></div>
      <p class="sub">${x.on_sheet} plays on the card. Plays from the playbook are known; reps on the card sharpen them. A play you drew stays off the card in games until your players know it about a third of the way.</p></div></div></div>`;
}
function bookTabHtml(x) {
  const fam = x.families.map((f) => `<div class="panel warm tr-fam"><h2>${esc(trCap(f.family))} <small>${f.reps} reps</small></h2>${trInstall(f.install, f.words)}
    ${f.runs.length ? `<div class="tr-plays"><span class="st-label">Runs</span>${f.runs.map((r) => `<span class="tag soft">${esc(trCap(r))}</span>`).join("")}</div>` : ""}
    ${f.passes.length ? `<div class="tr-plays"><span class="st-label">Passes</span>${f.passes.map((r) => `<span class="tag soft">${esc(trCap(r))}</span>`).join("")}</div>` : ""}</div>`).join("");
  const def = x.defense.map((f) => `<div class="attr"><span>${esc(trCap(f.family))} <small class="muted">${f.reps} reps</small></span>${trInstall(f.install, f.words)}</div>`).join("");
  const des = x.designs.length ? x.designs.map((d) => `<div class="item"><span class="when">${esc(d.saved)}</span><span><a class="link" data-go="designer" data-args='${esc(JSON.stringify({ k: d.k }))}'>${esc(d.design.name)}</a> <small class="muted">${esc(d.design.personnel)} · ${esc(d.design.set)}${d.on_sheet ? " · on the card" : ""}</small> ${trInstall(d.install, d.words)}</span></div>`).join("") : `<div class="empty">No plays of your own yet. <a class="link" data-go="designer">Draw one</a>.</div>`;
  return `<p class="sub">Your playbook by family. Your players know the base playbook; plays on the call sheet get the team periods' reps this year, and plays in a family share their rules, so reps on one sharpen the others.</p>
    <div class="tr-fams">${fam}</div><div class="grid2"><div class="panel warm"><h2>Defense <small>fronts and coverages</small></h2>${def}</div><div class="panel warm"><h2>Your designs</h2><div class="items">${des}</div></div></div>`;
}
function installTabHtml(x) {
  const p = x.pace;
  const des = x.designs.map((d) => `<label class="gd-check tr-instrow"><input type="checkbox" data-act="tr-inst" data-k="${d.k}" ${d.installing ? "checked" : ""}> <b>${esc(d.design.name)}</b> ${trInstall(d.install, d.words)}</label>`).join("");
  return `<div class="st-tiles tr-tiles"><div><b>${p.team_minutes}</b><small>team and situation minutes this week</small></div><div><b>${p.keys}</b><small>families and designs being installed</small></div><div><b>${p.reps_each}</b><small>reps a week for each</small></div><div><b>${p.to_know || "-"}</b><small>weeks until they know a new one</small></div></div>
    <div class="grid2"><div class="panel warm"><h2>What goes in <small>every family on the card, and the designs you tick</small></h2>${des || '<div class="empty">No designs to install. The card\'s families go in by themselves.</div>'}<p class="sub">The card's families and the designs you tick share the week's team reps: fewer things in, more reps each. A design goes into games once your players know it about a third of the way.</p></div>
    <div class="panel warm"><h2>How it goes in</h2><div class="items"><div class="item story"><span>Team periods, seven-on-seven, situations and walkthroughs teach the plan. Your week's minutes are on Training.</span></div><div class="item story"><span>A family the players know well rusts slowly when it's off the card.</span></div><div class="item story"><span>A design your players don't know yet stays off the card on game day; your play caller calls from the playbook instead.</span></div></div>
    <div class="st-actions"><button class="st-btn small glass" data-go="training">The practice week</button></div></div></div>`;
}
screens.plans = async (a) => {
  const x = await api("plans");
  setColors(x.team.colors);
  const list = ["Game plan", "Call sheet", "Playbook", "Installs"];
  const tab = tabOf("trplan", list, a);
  title("Plans", tab);
  TR.plans = x;
  const body = tab === "Game plan" ? planTabHtml(x) : tab === "Call sheet" ? sheetTabHtml(x) : tab === "Playbook" ? bookTabHtml(x) : installTabHtml(x);
  return `<div class="film"><div class="pad tr-pad">${pageHead({ kicker: `${esc(trCap(x.tier))} · ${esc(x.coordinator ? "play caller " + x.coordinator : "your staff")}`, title: "Plans", sub: "The season's lean, the call sheet, the playbook and what goes in each week.", goto: trGoto([["Game day", "gameday"], ["Play designer", "designer"], ["Training", "training"], ["Depth chart", "depth"]]) })}${tabs("trplan", list, tab)}<div class="tabbody">${body}</div></div></div>`;
};

// ======================================================================
// The play designer (plays and tactics topic 8)
// ======================================================================
const DES_W = 560, DES_H = 420, DES_Y = 30, DES_BACK = 12, DES_DEEP = 25;
// The middle of the field is drawn wider (2 to 1 inside six yards) so the
// line and the backs have room; outside it the field is close to scale.
const bdSpread = (u, at, k, out) => Math.sign(u) * (Math.abs(u) <= at ? Math.abs(u) * k : at * k + (Math.abs(u) - at) * out);
const desX = (y) => ((bdSpread(y, 6, 2, 0.75) + DES_Y) / (2 * DES_Y)) * DES_W;
const desY = (x) => ((DES_DEEP - x) / (DES_DEEP + DES_BACK)) * DES_H;
function fieldSvg(geo, opts = {}) {
  const lines = [];
  for (let yd = -10; yd <= DES_DEEP; yd += 5) lines.push(`<line x1="0" x2="${DES_W}" y1="${desY(yd)}" y2="${desY(yd)}" class="${yd === 0 ? "los" : "yd"}"/>${yd > 0 ? `<text x="8" y="${desY(yd) - 3}" class="ydt">${yd}</text>` : ""}`);
  const hash = [-3.08, 3.08].map((h) => `<line x1="${desX(h)}" x2="${desX(h)}" y1="0" y2="${DES_H}" class="hash"/>`).join("");
  const paths = geo.players.filter((p) => p.path && p.path.length > 1).map((p) => {
    const pts = p.path.map(([x, y]) => `${desX(y).toFixed(1)},${desY(x).toFixed(1)}`).join(" ");
    const last = p.path[p.path.length - 1];
    return `<polyline points="${pts}" class="route ${p.read ? "r" + Math.min(p.read, 3) : ""}"/><circle cx="${desX(last[1]).toFixed(1)}" cy="${desY(last[0]).toFixed(1)}" r="3" class="tip ${p.read ? "r" + Math.min(p.read, 3) : ""}"/>`;
  }).join("");
  const men = geo.players.map((p) => `<g class="man ${p.line ? "ol" : ""} ${opts.sel === p.role ? "sel" : ""}" ${opts.click ? `data-act="tr-drole" data-role="${p.role}"` : ""} transform="translate(${desX(p.y).toFixed(1)},${desY(p.x).toFixed(1)})"><circle r="${opts.small ? 6 : 10}"/>${opts.small ? "" : `<text dy="3.5">${esc(p.label)}</text>`}${p.read && !opts.small ? `<text class="rd" dx="12" dy="-8">${p.read}</text>` : ""}</g>`).join("");
  return `<svg class="tr-field ${opts.small ? "small" : ""}" viewBox="0 0 ${DES_W} ${DES_H}" role="img" aria-label="The play on the field">${lines.join("")}${hash}${paths}${men}</svg>`;
}
function newDesign() {
  return { name: "New play", personnel: "11", backfield: "shotgun", set: "doubles", strength: "left", motion: "none", routes: [], reads: [], protection: "standard", drop: "medium", play_action: false };
}
screens.designer = async (a) => {
  const x = await api("designer", a.k != null ? { k: a.k } : {});
  setColors(x.team.colors);
  title("Play designer");
  if (!TR.des || (a.k != null && TR.des.k !== a.k)) {
    const open = a.k != null ? x.designs.find((d) => d.k === a.k) : null;
    TR.des = { k: open ? open.k : null, d: open ? JSON.parse(JSON.stringify(open.design)) : newDesign(), role: null };
  }
  const des = TR.des;
  const g = await api("design_draw", { design: des.d });
  const d = des.d;
  const per = x.personnels.find((p) => p.v === d.personnel) || x.personnels[0];
  const roleRows = per.roles.map((r) => {
    const rt = d.routes.find((q) => q.role === r.role) || { route: "", depth: 0 };
    const def = x.routes.find((q) => q.name === rt.route);
    const read = d.reads.indexOf(r.role);
    return `<tr class="${des.role === r.role ? "on" : ""}"><td class="l"><a class="link" data-act="tr-drole" data-role="${r.role}">${esc(r.label)}</a></td>
      <td class="l">${trSel("tr-droute", [["", "Default for the protection"], ...x.routes.map((q) => [q.name, q.label])], rt.route, `data-role="${r.role}"`)}</td>
      <td class="l">${def && def.depths.length ? trSel("tr-ddepth", def.depths.map((v) => [v, v + " yd"]), Math.round(rt.depth || def.depths[0]), `data-role="${r.role}"`) : '<span class="muted">-</span>'}</td>
      <td>${trSel("tr-dread", [[0, "-"], [1, "1st"], [2, "2nd"], [3, "3rd"], [4, "4th"]], read >= 0 ? read + 1 : 0, `data-role="${r.role}"`)}</td></tr>`;
  }).join("");
  const sel = (k, opts, cur) => trSel("tr-dset", opts, cur, `data-f="${k}"`);
  const issues = g.issues.length ? `<div class="panel tr-breach"><h2>Not legal at your level</h2><div class="items">${g.issues.map((i) => `<div class="item story"><span>${esc(i)}</span></div>`).join("")}</div></div>` : `<p class="tr-ok">Legal at your level.</p>`;
  const saved = des.k != null ? x.designs.find((q) => q.k === des.k) : null;
  const review = saved && saved.review.length ? `<div class="panel warm"><h2>${esc(x.coordinator || "Your coordinator")}'s review</h2><div class="items">${saved.review.map((r) => `<div class="item story"><span>${esc(r)}</span></div>`).join("")}</div>${saved.read && saved.read.snaps ? `<p class="sub">Called ${saved.read.snaps} times: ${saved.read.yards} yards a play, ${saved.read.completions} completions.</p>` : ""}</div>` : "";
  const list = x.designs.map((q) => `<button class="tr-dl ${des.k === q.k ? "on" : ""}" data-act="tr-dopen" data-k="${q.k}">${fieldSvg(q.geometry, { small: true })}<span><b>${esc(q.design.name)}</b><small>${esc(q.design.personnel)} · ${esc(q.design.set)} · ${esc(q.words)}</small></span></button>`).join("");
  return `<div class="film"><div class="pad tr-pad">${pageHead({ kicker: `${esc(trCap(x.tier))} · plays and tactics`, title: "Play designer", sub: "Draw a pass play from a formation and routes. It goes into your playbook, your players learn it in team periods, and your play caller can call it once it's on the card.", goto: trGoto([["Plans", "plans", { tab: "Playbook" }], ["Call sheet", "plans", { tab: "Call sheet" }], ["Training", "training"]]) })}
    <div class="tr-des"><aside class="tr-dlist"><button class="st-btn small primary" data-act="tr-dnew">New play</button>${list || '<div class="empty">No plays yet.</div>'}</aside>
    <div class="tr-dboard"><div class="panel warm tr-dfield">${fieldSvg(g.geometry, { click: true, sel: des.role })}<p class="sub">Click a player to pick his row. Numbers are the quarterback's reads in order.</p></div>${issues}
    <div class="panel warm"><h2>Routes and reads <small>a player with no route runs his job in the protection</small></h2><div class="tbl"><table><thead><tr><th class="l">Player</th><th class="l">Route</th><th class="l">Depth</th><th>Read</th></tr></thead><tbody>${roleRows}</tbody></table></div></div>${review}</div>
    <div class="tr-dform"><div class="panel warm"><h2>The play</h2><div class="gd-form">
      <label>Name</label><input class="tr-text" data-act="tr-dname" value="${esc(d.name)}" maxlength="40">
      <label>Personnel</label>${sel("personnel", x.personnels.map((p) => [p.v, p.v + " personnel"]), d.personnel)}
      <label>Backfield</label>${sel("backfield", x.backfields.map((b) => [b, trCap(b)]), d.backfield)}
      <label>Set</label>${sel("set", x.sets.map((s) => [s, trCap(s)]), d.set)}
      <label>Strength</label>${sel("strength", [["left", "Left"], ["right", "Right"]], d.strength)}
      <label>Motion</label>${sel("motion", x.motions, d.motion)}
      <label>Protection</label>${sel("protection", x.protections, d.protection)}
      <label>Drop</label>${sel("drop", x.drops, d.drop)}
      <label>Play action</label><div><label class="gd-check"><input type="checkbox" data-act="tr-dpa" ${d.play_action ? "checked" : ""}> A run fake first</label></div>
    </div></div>
    <div class="st-actions"><button class="st-btn primary" data-act="tr-dsave" ${g.issues.length ? "disabled" : ""}>${des.k != null ? "Save changes" : "Save to the playbook"}</button>${des.k != null ? '<button class="st-btn glass" data-act="tr-ddel">Delete</button>' : ""}</div></div></div></div></div>`;
};

// ======================================================================
// The squad: six views (screens topic 6)
// ======================================================================
const rosterBase = screens.roster;
const SQUAD_VIEWS = ["General", "Ratings", "Development", "Fitness", "Depth", "Contract"];
function squadCols(view, x) {
  const name = { k: "name", t: "Name", l: true, f: (r) => `${playerLink(r.id, r.name, view === "Development" ? "Development" : "")}${r.notes ? ` <span class="st-pill" title="Your notes">✎ ${r.notes}</span>` : ""}` };
  const pos = { k: "pos", t: "Pos", l: true };
  const cls = { k: "class", t: "Class", l: true, v: (r) => classRank(r.class) };
  const ovr = { k: "ovr", t: "Rating", tip: "Your staff's read of his play now", v: (r) => mid(r.ovr), f: (r) => range(r.ovr) };
  const key = (n) => ({ k: "k" + n, t: `Key ${n + 1}`, tip: "The skills that matter most at his position, your position coach's read", v: (r) => mid(r["k" + n]), f: (r) => `<span class="tr-key"><small>${esc(r["k" + n + "_label"] || "")}</small>${range(r["k" + n])}</span>` });
  const pro = x.tier === "nfl";
  const views = {
    General: [name, pos, cls, { k: "age", t: "Age" }, { k: "ht", t: "Ht", v: (r) => inches(r.ht) }, { k: "wt", t: "Wt" }, { k: "home", t: "Hometown", l: true, wrap: true }, { k: "stars", t: "★", tip: "Recruiting stars", f: (r) => (r.stars ? "★".repeat(r.stars) : '<span class="muted">-</span>') }, ovr, { k: "mood", t: "Mood", l: true }, { k: "status", t: "Status", l: true, f: (r) => (r.status ? `<span class="dot ${/Injur|Susp|ineligible/.test(r.status) ? "bad" : "warn"}"></span>${esc(r.status)}` : '<span class="muted">Available</span>') }, { k: "born", t: "Born", l: true, hide: true }],
    Ratings: [name, pos, cls, ovr, key(0), key(1), key(2), key(3)],
    Development: [name, pos, cls, { k: "age", t: "Age" }, ovr, { k: "growth", t: "Growth", l: true, f: (r) => trGrowth(r.growth) }, { k: "grade_words", t: "Practice", l: true, v: (r) => r.grade }, { k: "focus", t: "Main focus", l: true, f: (r) => (r.focus ? `${esc(r.focus)}${r.focus_by === "staff" ? ' <small class="muted">staff</small>' : ""}` : '<span class="muted">Balanced</span>') }, { k: "teacher", t: "Taught by", l: true, v: (r) => r.teacher.name || "", f: (r) => (r.teacher.name ? `${r.teacher.you ? "You" : coachLink(r.teacher.id, r.teacher.name)} <small class="muted">${esc(r.teacher.words)}</small>` : '<span class="muted">-</span>') }, { k: "habit", t: "Habit", l: true, hide: true }, { k: "moving", t: "Learning", l: true, hide: true }, { k: "body_target", t: "Body target", hide: true }],
    Fitness: [name, pos, cls, { k: "practice", t: "Practice", l: true, f: trStatus }, { k: "load", t: "Load", l: true, f: trLoad, v: (r) => ["light", "moderate", "heavy", "very heavy"].indexOf(r.load) }, { k: "load_trend", t: "Trend", l: true }, { k: "missed", t: "Games missed", tip: "Last season" }, { k: "status", t: "Status", l: true }, { k: "wt", t: "Wt" }, { k: "sport", t: "Other sport", l: true, hide: x.tier !== "high school" }],
    Depth: [name, pos, cls, { k: "depth", t: "Depth", tip: "His place at his position on game day", v: (r) => (r.depth == null ? 99 : -r.depth), f: (r) => (r.depth ? (r.depth === 1 ? "<b>1st</b>" : r.depth === 2 ? "2nd" : r.depth === 3 ? "3rd" : r.depth + "th") : '<span class="muted">-</span>') }, { k: "role", t: "Practice role", l: true }, { k: "last_role", t: "Last season", l: true, f: (r) => esc(trCap(r.last_role || "")) }, ovr, { k: "grade_words", t: "Practice grades", l: true, v: (r) => r.grade }],
    Contract: pro
      ? [name, pos, { k: "age", t: "Age" }, { k: "class", t: "Experience", l: true, v: (r) => classRank(r.class) }, { k: "apy", t: "A year", f: (r) => money(r.apy) }, { k: "through", t: "Through" }, { k: "accrued", t: "Accrued seasons" }, ovr]
      : [name, pos, cls, { k: "age", t: "Age" }, { k: "years_left", t: x.tier === "high school" ? "Years left" : "Seasons left", tip: x.tier === "high school" ? "School years before he graduates" : "Seasons of eligibility left to play" }, { k: "redshirt", t: "Redshirt", l: true, hide: x.tier === "high school", f: (r) => (r.redshirt ? "Used" : '<span class="muted">Available</span>'), v: (r) => (r.redshirt ? 1 : 0) }, { k: "offers", t: "College offers", hide: x.tier !== "high school" }, { k: "stars", t: "★", f: (r) => (r.stars ? "★".repeat(r.stars) : '<span class="muted">-</span>') }],
  };
  return views[view];
}
screens.roster = async (a) => {
  if (!trMine(a)) return rosterBase(a);
  const x = await api("squad");
  setColors(x.team.colors);
  const viewNames = SQUAD_VIEWS.map((v) => (v === "Contract" && x.tier !== "nfl" ? "Eligibility" : v));
  const k = keep("sqview", { v: "General" });
  if (a.view && SQUAD_VIEWS.includes(a.view)) k.v = a.view;
  const view = SQUAD_VIEWS.includes(k.v) ? k.v : "General";
  title("Squad", viewNames[SQUAD_VIEWS.indexOf(view)]);
  const f = keep("sqf", { unit: "ALL" });
  const rows = x.rows.filter((r) => f.unit === "ALL" || r.group === f.unit);
  const count = (u) => x.rows.filter((r) => r.group === u).length;
  const seg = `<div class="seg">${[["ALL", "All", x.rows.length], ["OFF", "Offense", count("OFF")], ["DEF", "Defense", count("DEF")], ["ST", "Special teams", count("ST")]].map(([v, t, n]) => `<button data-act="tr-squnit" data-v="${v}" aria-pressed="${f.unit === v}">${t}<small>${n}</small></button>`).join("")}</div>`;
  const vseg = `<div class="seg tr-views">${SQUAD_VIEWS.map((v, i) => `<button data-act="tr-sqview" data-v="${v}" aria-pressed="${v === view}">${viewNames[i]}</button>`).join("")}</div>`;
  const out = x.rows.filter((r) => r.practice === "out").length;
  const hurt = x.rows.filter((r) => r.spike).length;
  return `<div class="film"><div class="pad tr-pad">${pageHead({ kicker: `${esc(trCap(x.tier))} · ${x.rows.length} players`, title: `<span class="tr-tmark">${mark(x.team, 40)}</span>${esc(x.team.name)}`, sub: `Your staff reads them every day.${out ? ` ${trPlural(out, "player")} out of practice.` : ""}${hurt ? ` ${hurt} on load watch.` : ""}`, goto: trGoto([["Depth chart", "depth"], ["Training", "training"], ["Plans", "plans"], ["Staff", "staff"], ["Leaderboards", "leaders"]]) })}
    <div class="filters">${vseg}${seg}</div>
    ${table("squad:" + view, squadCols(view, x), rows, { groupBy: (r) => r.pos })}</div></div>`;
};

// ======================================================================
// The depth chart as formation boards (screens topic 8)
// ======================================================================
const depthBase = screens.depth;
const BOARD_W = 640, BOARD_H = 380;
// The board spreads the middle of the field (the line, the box) as the
// designer does, so every spot's card has room.
const bdX = (y) => ((bdSpread(y, 6, 2.2, 0.7) + 27) / 54) * 100;
const bdY = (x) => ((21 - bdSpread(x, 8, 1.8, 0.9)) / 45) * 100;
function slotCard(s, k, b, sel) {
  const p = s.player;
  const prop = s.proposal && (!p || s.proposal.id !== p.id) ? `<em title="Your position coach would put ${esc(s.proposal.name)} here">Coach: ${esc(trLast(s.proposal.name))}</em>` : "";
  const drag = s.controls ? 'draggable="true"' : "";
  return `<div class="tr-slot ${sel ? "sel" : ""} ${p ? "" : "empty"} ${s.controls ? "" : "locked"} ${p && p.practice !== "full" ? "p-" + p.practice : ""}" style="left:${bdX(s.y)}%;top:${bdY(s.x)}%" data-act="tr-slot" data-k="${k}" data-board="${esc(b.key)}" ${drag} ${p ? `data-pid="${p.id}"` : ""}>
    <small>${esc(s.slot)}</small><b>${p ? esc(trLast(p.name)) : "Open"}</b>${p ? `<span class="tr-sr">${p.ovr ? `${p.ovr[0]}-${p.ovr[1]}` : "?"}${p.pos !== s.pos ? ` · ${esc(p.pos)}` : ""}</span>` : ""}${prop}</div>`;
}
screens.depth = async (a) => {
  if (a.kind && !trMine(a)) return depthBase(a);
  const x = await api("boards", {});
  setColors(x.team.colors);
  const names = [...x.boards.map((b) => b.label), "By position"];
  const tab = tabOf("trboard", names, a);
  title("Depth chart", tab);
  const head = pageHead({ kicker: `${esc(trCap(x.tier))} · game day`, title: "Depth chart", sub: "Drag a player onto a spot, or click a spot and then a player. Your position coaches' picks show under each spot; where you set an order, the engine plays yours.", goto: trGoto([["Squad", "roster"], ["Training", "training"], ["Plans", "plans"], ["Game day", "gameday"], ["Schedule", "schedule"]]) });
  const tabsHtml = tabs("trboard", names.map((n) => { const b = x.boards.find((q) => q.label === n); return b && b.yours ? { name: n, badge: "●" } : n; }), tab);
  if (tab === "By position") {
    const inner = await depthBase(a);
    return `<div class="film"><div class="pad tr-pad">${head}${tabsHtml}</div>${inner.replace('<div class="film">', "<div>")}</div>`;
  }
  const b = x.boards.find((q) => q.label === tab) || x.boards[0];
  const sel = TR.pick && TR.pick.board === b.key ? TR.pick.k : null;
  const want = sel != null ? b.slots[sel] : null;
  const onBoard = new Set(b.slots.filter((s) => s.player).map((s) => s.player.id));
  // The pool: everyone, the slot's position first when a slot is picked.
  const pool = x.players.slice().sort((p, q) => (want ? (q.pos === want.pos) - (p.pos === want.pos) : 0) || mid(q.ovr) - mid(p.ovr));
  const poolHtml = pool.filter((p) => !want || p.pos === want.pos || keep("trpoolall", { v: false }).v).map((p) => `<div class="tr-pp ${onBoard.has(p.id) ? "on" : ""} ${p.practice !== "full" ? "p-" + p.practice : ""}" draggable="true" data-act="tr-pp" data-pid="${p.id}"><span class="pos">${esc(p.pos)}</span><b>${playerLink(p.id, p.name)}</b><small>${esc(p.class)} · ${esc(p.role || "")}${p.practice !== "full" ? ` · ${esc(p.practice)}` : ""}</small>${range(p.ovr)}</div>`).join("");
  const props = b.slots.some((s) => s.proposal && (!s.player || s.proposal.id !== s.player.id));
  const locked = b.slots.some((s) => !s.controls);
  const changes = b.slots.map((s) => (s.proposal && s.player && s.proposal.id !== s.player.id ? `<div class="item"><span class="when">${esc(s.slot)}</span><span>${playerLink(s.proposal.id, s.proposal.name)} over ${playerLink(s.player.id, s.player.name)} <small class="muted">${esc(s.proposal.practice === "full" ? s.proposal.class : s.proposal.practice)}</small></span></div>` : "")).join("");
  return `<div class="film"><div class="pad tr-pad">${head}${tabsHtml}
    <div class="tr-boardwrap"><div class="panel warm tr-boardp"><h2>${esc(b.label)} <small>${b.yours ? "your order" : b.kind === "package" ? "your position coaches' picks: set it to make it yours" : "your staff's order"}${locked ? " · some spots are the head coach's" : ""}</small></h2>
      <div class="tr-board ${b.kind}" data-board="${esc(b.key)}"><div class="tr-los"></div>${b.slots.map((s, k) => slotCard(s, k, b, sel === k)).join("")}</div>
      <div class="st-actions">${props ? `<button class="st-btn small" data-act="tr-bprop" data-board="${esc(b.key)}">Use the coaches' picks</button>` : ""}${b.yours ? `<button class="st-btn small glass" data-act="tr-breset" data-board="${esc(b.key)}">Give it back to the staff</button>` : ""}${sel != null ? `<button class="st-btn small glass" data-act="tr-bclear" data-board="${esc(b.key)}" data-k="${sel}">Clear ${esc(want.slot)}</button>` : ""}</div>
      ${changes ? `<div class="tr-props"><div class="st-label">Your position coaches would change</div><div class="items">${changes}</div></div>` : ""}</div>
    <aside class="panel warm tr-pool"><h2>${want ? `For ${esc(want.slot)} <small>${esc(want.pos)}s first</small>` : "Players <small>drag onto a spot</small>"}</h2>${want ? `<label class="gd-check"><input type="checkbox" data-act="tr-poolall" ${keep("trpoolall", { v: false }).v ? "checked" : ""}> Everyone, not only ${esc(want.pos)}s</label>` : ""}<div class="tr-pplist">${poolHtml || '<div class="empty">Nobody at that position.</div>'}</div></aside></div></div></div>`;
};

// ---------- actions ----------
function trBoardIds(b) {
  return b.slots.map((s) => (s.player ? s.player.id : null));
}
async function trPlace(boardKey, k, pid) {
  const x = await api("boards", {});
  const b = x.boards.find((q) => q.key === boardKey);
  if (!b) return;
  const ids = trBoardIds(b);
  const from = ids.indexOf(pid);
  // Two spots at one position swap; on a base board a man moved to another
  // position's spot leaves his old one to the next man in that order.
  if (from >= 0 && from !== k) ids[from] = b.kind === "base" && b.slots[from].pos !== b.slots[k].pos ? null : ids[k];
  ids[k] = pid;
  const r = await trDo("board_set", { board: boardKey, ids });
  if (r && typeof r === "object" && r.note) toast(r.note);
  else if (r) toast("Depth chart saved.");
  TR.pick = null;
  trRedraw();
}
function trTab() {
  return keep("trtab", { t: "This week" }).t;
}
function trDays() {
  if (trTab() === "Templates") {
    const slot = keep("trtpl", {}).slot;
    const t = Object.keys(TR.tpl).includes(slot) ? slot : Object.keys(TR.tpl)[0];
    return { draft: TR.tpl[t], tpl: t };
  }
  return { draft: TR.week, tpl: null };
}
async function trSaveSheet(k, plays) {
  const r = await trDo("plans_sheet", { sit: k, plays: plays.map((p) => ({ personnel: p.personnel, backfield: p.backfield, set: p.set, kind: p.kind, play: p.play })) });
  if (r) trRedraw();
}
const trDesignRedraw = trRedraw;
document.addEventListener("click", async (ev) => {
  const el = ev.target.closest("[data-act^='tr-']");
  if (!el || el.tagName === "SELECT" || (el.tagName === "INPUT" && el.type !== "checkbox")) return;
  const d = el.dataset;
  switch (d.act) {
    case "tr-per": {
      const { tpl } = trDays();
      TR.sel = TR.sel && TR.sel.d === Number(d.d) && TR.sel.i === Number(d.i) && TR.sel.tpl === tpl ? null : { d: Number(d.d), i: Number(d.i), tpl };
      trRedraw();
      break;
    }
    case "tr-pdone": TR.sel = null; trRedraw(); break;
    case "tr-padd": {
      const { draft, tpl } = trDays();
      const day = draft.days[Number(d.d)];
      day.periods.push({ kind: "individual", min: 20, gear: "helmets", contact: "air", tempo: "normal", who: "all", what: 0 });
      draft.dirty = true;
      TR.sel = { d: Number(d.d), i: day.periods.length - 1, tpl };
      trRedraw();
      break;
    }
    case "tr-pdel":
    case "tr-pdup":
    case "tr-pmove": {
      const { draft } = trDays();
      if (!TR.sel) break;
      const ps = draft.days[TR.sel.d].periods;
      const i = TR.sel.i;
      if (d.act === "tr-pdel") { ps.splice(i, 1); TR.sel = null; }
      else if (d.act === "tr-pdup") { ps.splice(i + 1, 0, JSON.parse(JSON.stringify(ps[i]))); TR.sel.i = i + 1; }
      else {
        const j = i + Number(d.dir);
        if (j < 0 || j >= ps.length) break;
        [ps[i], ps[j]] = [ps[j], ps[i]];
        TR.sel.i = j;
      }
      draft.dirty = true;
      trRedraw();
      break;
    }
    case "tr-wsave": {
      try {
        await api("practice_week", { days: TR.week.days });
        toast("This week saved.");
        TR.week = null;
        TR.sel = null;
        trRedraw();
      } catch (e) {
        const m = e.message || String(e);
        if (/Outside the rules/.test(m) && (await sure("Outside the rules", `${m} Run it anyway? A staff member or a player could report it, and a case costs practices or more.`, "Run it anyway"))) {
          if (await trDo("practice_week", { days: TR.week.days, push: true }, "Saved outside the rules.")) { TR.week = null; TR.sel = null; trRedraw(); }
        } else toast(m);
      }
      break;
    }
    case "tr-wundo": TR.week = null; TR.sel = null; trRedraw(); break;
    case "tr-wreset": if (await trDo("practice_week_reset", {}, "Your staff runs this week again.")) { TR.week = null; TR.sel = null; trRedraw(); } break;
    case "tr-wtpl": {
      const x = await api("practice");
      if (await trDo("practice_template", { slot: x.slot, days: TR.week.days }, `Kept as the ${x.slot_label.toLowerCase()} template.`)) { delete TR.tpl[x.slot]; trRedraw(); }
      break;
    }
    case "tr-slot": {
      if (el.classList.contains("tr-slot") && d.board) {
        // A spot on a board: pick it, or put the picked player there.
        const k = Number(d.k);
        if (el.classList.contains("locked")) { toast("That spot is the head coach's call."); break; }
        TR.pick = TR.pick && TR.pick.board === d.board && TR.pick.k === k ? null : { board: d.board, k };
        trRedraw();
        break;
      }
      break;
    }
    case "tr-tslot": keep("trtpl", {}).slot = d.v; TR.sel = null; trRedraw(); break;
    case "tr-tsave": {
      const t = TR.tpl[d.slot];
      try {
        await api("practice_template", { slot: d.slot, days: t.days });
        toast("Template saved.");
        delete TR.tpl[d.slot];
        TR.sel = null;
        TR.week = null;
        trRedraw();
      } catch (e) { toast(e.message); }
      break;
    }
    case "tr-tundo": delete TR.tpl[d.slot]; TR.sel = null; trRedraw(); break;
    case "tr-treset": if (await trDo("practice_template_reset", { slot: d.slot }, "Back to your staff's template.")) { delete TR.tpl[d.slot]; TR.week = null; trRedraw(); } break;
    case "tr-sit": keep("trsheet", {}).sit = Number(d.k); trRedraw(); break;
    case "tr-sadd": {
      const x = TR.plans;
      const k = keep("trsheet", { sit: 0 }).sit;
      const add = keep("tradd", {});
      const sit = x.sheet[k];
      const play = add.kind === "design" ? Number(add.play) : add.play;
      await trSaveSheet(k, [...sit.plays, { ...add, play }]);
      break;
    }
    case "tr-sdel":
    case "tr-sup": {
      const x = TR.plans;
      const k = keep("trsheet", { sit: 0 }).sit;
      const plays = x.sheet[k].plays.slice();
      const i = Number(d.i);
      if (d.act === "tr-sdel") plays.splice(i, 1);
      else [plays[i - 1], plays[i]] = [plays[i], plays[i - 1]];
      await trSaveSheet(k, plays);
      break;
    }
    case "tr-inst": await trDo("plans_install", { design: Number(d.k), on: el.checked }, el.checked ? "It goes in this week." : "Taken out of the install."); break;
    case "tr-dnew": TR.des = { k: null, d: newDesign(), role: null }; trRedraw(); break;
    case "tr-dopen": TR.des = null; go("designer", { k: Number(d.k) }, false); break;
    case "tr-drole": TR.des.role = TR.des.role === d.role ? null : d.role; trRedraw(); break;
    case "tr-dpa": TR.des.d.play_action = el.checked; trRedraw(); break;
    case "tr-dsave": {
      const r = await trDo("design_save", { design: TR.des.d, k: TR.des.k }, () => `Saved to your playbook. ${TR.des.k == null ? "It goes into this week's install." : "Your players learn it again."}`);
      if (r) { TR.des.k = r.open != null ? r.open : TR.des.k; S.route.args = { k: TR.des.k }; trRedraw(); }
      break;
    }
    case "tr-ddel": {
      if (!(await sure("Delete this play?", "It comes off the card and out of the install.", "Delete"))) break;
      if (await trDo("design_delete", { k: TR.des.k }, "Deleted.")) { TR.des = { k: null, d: newDesign(), role: null }; S.route.args = {}; trRedraw(); }
      break;
    }
    case "tr-squnit": keep("sqf", {}).unit = d.v; trRedraw(); break;
    case "tr-sqview": keep("sqview", {}).v = d.v; S.route.args = { ...S.route.args, view: d.v }; trRedraw(); break;
    case "tr-pp": {
      if (ev.target.closest("[data-go]")) break;
      if (!TR.pick) { toast("Pick a spot on the board first, or drag him onto one."); break; }
      await trPlace(TR.pick.board, TR.pick.k, Number(d.pid));
      break;
    }
    case "tr-bprop": {
      const x = await api("boards", {});
      const b = x.boards.find((q) => q.key === d.board);
      const ids = b.slots.map((s) => (s.proposal ? s.proposal.id : s.player ? s.player.id : null));
      const r = await trDo("board_set", { board: d.board, ids });
      if (r) { toast(typeof r === "object" && r.note ? r.note : "Your coaches' picks are in."); trRedraw(); }
      break;
    }
    case "tr-breset": if (await trDo("board_set", { board: d.board, reset: true }, "Your staff sets it again.")) trRedraw(); break;
    case "tr-bclear": {
      const x = await api("boards", {});
      const b = x.boards.find((q) => q.key === d.board);
      const ids = trBoardIds(b);
      ids[Number(d.k)] = null;
      if (await trDo("board_set", { board: d.board, ids }, "Cleared.")) { TR.pick = null; trRedraw(); }
      break;
    }
    case "tr-poolall": keep("trpoolall", { v: false }).v = el.checked; trRedraw(); break;
  }
});
document.addEventListener("change", async (ev) => {
  const el = ev.target.closest("[data-act^='tr-']");
  if (!el) return;
  const d = el.dataset;
  switch (d.act) {
    case "tr-pf": {
      const { draft } = trDays();
      if (!TR.sel || !draft) break;
      const p = draft.days[TR.sel.d].periods[TR.sel.i];
      p[d.f] = d.f === "min" || d.f === "what" ? Number(el.value) : el.value;
      if (d.f === "kind") p.field = !["weight_room", "meeting", "film", "recovery"].includes(el.value);
      draft.dirty = true;
      trRedraw();
      break;
    }
    case "tr-focus": await trDo("practice_plan", { id: Number(d.id), focus: el.value }, "Plan saved."); break;
    case "tr-dev": {
      const id = Number(d.id);
      const dv = await api("player_dev", { id });
      const pl = dv.plan;
      const args = { id };
      if (d.f === "focus") args.focus = el.value;
      else if (d.f === "habit") args.habit = el.value === "" ? null : [Number(el.value), 13];
      else if (d.f === "habit_target") args.habit = [pl.habit.k, Number(el.value)];
      else if (d.f === "body") args.body = el.value === "" ? null : [Number(el.value), ["strength", "speed", "lean"].indexOf(pl.body ? pl.body.emphasis : "strength")];
      else if (d.f === "body_emph") args.body = [pl.body ? pl.body.target : Math.round(dv.weight), ["strength", "speed", "lean"].indexOf(el.value)];
      else if (d.f === "moving") args.moving = el.value;
      else if (d.f === "rest") args.rest = Number(el.value);
      else if (d.f === "extra") {
        const bits = (pl.extra.film ? 1 : 0) | (pl.extra.work ? 2 : 0) | (pl.extra.lift ? 4 : 0);
        args.extra = el.checked ? bits | Number(d.bit) : bits & ~Number(d.bit);
      }
      if (await trDo("practice_plan", args, "Plan saved.")) trRedraw();
      break;
    }
    case "tr-lean": await trDo("plans_set", { lean: Number(el.value) / 100 }, "The lean is set."); trRedraw(); break;
    case "tr-stick": await trDo("plans_set", { stick: Number(el.value) / 100 }, "Saved."); trRedraw(); break;
    case "tr-add": {
      const add = keep("tradd", {});
      add[d.f] = el.value;
      if (d.f === "kind") {
        const x = TR.plans;
        add.play = el.value === "run" ? x.runs[0].name : el.value === "design" ? (x.designs[0] ? x.designs[0].k : "") : x.passes[0].name;
        trRedraw();
      }
      break;
    }
    case "tr-dset": TR.des.d[d.f] = el.value; trDesignRedraw(); break;
    case "tr-dname": TR.des.d.name = el.value; break;
    case "tr-droute": {
      const des = TR.des.d;
      des.routes = des.routes.filter((r) => r.role !== d.role);
      if (el.value) {
        const def = (await api("designer", {})).routes.find((q) => q.name === el.value);
        des.routes.push({ role: d.role, route: el.value, depth: def && def.depths.length ? def.depths[0] : 0 });
      }
      TR.des.role = d.role;
      trDesignRedraw();
      break;
    }
    case "tr-ddepth": {
      const r = TR.des.d.routes.find((q) => q.role === d.role);
      if (r) r.depth = Number(el.value);
      trDesignRedraw();
      break;
    }
    case "tr-dread": {
      const des = TR.des.d;
      des.reads = des.reads.filter((x) => x !== d.role);
      const n = Number(el.value);
      if (n > 0) des.reads.splice(Math.min(n - 1, des.reads.length), 0, d.role);
      trDesignRedraw();
      break;
    }
  }
});
document.addEventListener("input", (ev) => {
  const el = ev.target;
  if (el.dataset && el.dataset.act === "tr-dname" && TR.des) TR.des.d.name = el.value;
});
// Drag a player onto a spot (or one spot onto another to swap them).
document.addEventListener("dragstart", (ev) => {
  const el = ev.target.closest && ev.target.closest(".tr-pp,.tr-slot[data-pid]");
  if (!el) return;
  TR.drag = { pid: Number(el.dataset.pid) };
  ev.dataTransfer.effectAllowed = "move";
  try { ev.dataTransfer.setData("text/plain", el.dataset.pid); } catch (e) { /* some browsers */ }
  el.classList.add("dragging");
});
document.addEventListener("dragend", (ev) => {
  const el = ev.target.closest && ev.target.closest(".dragging");
  if (el) el.classList.remove("dragging");
  document.querySelectorAll(".tr-slot.over").forEach((x) => x.classList.remove("over"));
});
document.addEventListener("dragover", (ev) => {
  const el = ev.target.closest && ev.target.closest(".tr-slot");
  if (!el || !TR.drag || el.classList.contains("locked")) return;
  ev.preventDefault();
  document.querySelectorAll(".tr-slot.over").forEach((x) => x !== el && x.classList.remove("over"));
  el.classList.add("over");
});
document.addEventListener("drop", async (ev) => {
  const el = ev.target.closest && ev.target.closest(".tr-slot");
  if (!el || !TR.drag) return;
  ev.preventDefault();
  const pid = TR.drag.pid;
  TR.drag = null;
  await trPlace(el.dataset.board, Number(el.dataset.k), pid);
});
