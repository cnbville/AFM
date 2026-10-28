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
// Draws again from the data already in the page (a pick, the side panel,
// the draft week): nothing changed on the server.
const trLocal = () => { TR.reuse = true; return render(trScroll()); };
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
navAdd({ go: "plans", label: "Plans", icon: "plans", sec: "team", after: "training", seats: ["college", "nfl", "hs"] });
navAdd({ go: "designer", label: "Play designer", icon: "design", sec: "team", after: "plans", seats: ["college", "nfl", "hs"] });
Object.assign(CRUMBS, { plans: "Plans", designer: "Play designer", roster: "Squad", depth: "Depth chart", training: "Training" });
GOTO.training = () => [["Squad", "roster", { view: "Development" }], ["Depth chart", "depth"], ["Plans", "plans"], ["Staff", "staff"], ["Schedule", "schedule"]];
GOTO.plans = () => [["Game day", "gameday"], ["Play designer", "designer"], ["Training", "training"], ["Depth chart", "depth"], ["Schedule", "schedule"]];
GOTO.designer = () => [["Plans", "plans", { tab: "Playbook" }], ["Call sheet", "plans", { tab: "Call sheet" }], ["Training", "training"], ["Depth chart", "depth"]];
GOTO.depth = () => [["Squad", "roster"], ["Training", "training"], ["Plans", "plans"], ["Game day", "gameday"], ["Schedule", "schedule"]];
// Your Squad's one line in every view (screens topic 17): the Fitness view
// is its Health, and the money desk is the cap sheet at the NFL, the
// recruiting board in college. Another league's club has no training.
GOTO.roster = (a) => {
  if (!trMine(a)) return [["Schedule", "schedule", teamArgs(a)], ["Staff", "staff", teamArgs(a)], ["Team page", "team", teamArgs(a)], ["Leaderboards", "leaders"]];
  const seat = seatKind();
  const onHealth = S.route.screen === "roster" && (S.keep.sqview || {}).v === "Fitness";
  return [["Depth chart", "depth"], seat !== "other" ? ["Training", "training"] : null, onHealth ? null : ["Health", "roster", { view: "Fitness" }], seat === "nfl" ? ["Cap sheet", "fo_cap"] : seat === "college" ? ["Recruiting board", "recruit_board"] : null, ["Compare", "compare"]];
};
// These screens draw their own Go-to lines.
for (const s of ["training", "depth", "roster"]) {
  const i = GOTO_AFTER.indexOf(s);
  if (i >= 0) GOTO_AFTER.splice(i, 1);
}

// ======================================================================
// Training: the week, players, templates, the year, reports
// ======================================================================
const DAY_SHORT = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const GROUP_CLS = { technique: "f", team: "t", situations: "t", special: "t", body: "b", classroom: "m", people: "r" };
const trClone = (days) => JSON.parse(JSON.stringify(days.map((d) => ({ periods: d.periods, game: d.game, second: d.second }))));
// A day's sum, worked out in the page as you edit it.
function trDayFacts(d, x) {
  const min = d.periods.reduce((s, p) => s + Number(p.min || 0), 0);
  const field = d.periods.filter((p) => (x ? twKind(x, p.kind).field && !twOwn(p) : p.field !== false)).reduce((s, p) => s + Number(p.min || 0), 0);
  const pads = d.periods.some((p) => p.gear === "pads");
  const full = d.periods.filter((p) => p.contact === "thud" || p.contact === "live").reduce((s, p) => s + Number(p.min || 0), 0);
  return { min, field, pads, full };
}
function periodChip(p, k, i, edit, x) {
  const kind = x.options.kinds.find((o) => o.kind === p.kind) || { label: p.label || p.kind, group: p.group };
  const gear = (x.options.gears.find((g) => g.v === p.gear) || {}).t || p.gear;
  const contact = (x.options.contacts.find((c) => c.v === p.contact) || {}).t || p.contact;
  const sit = (p.kind === "situations" || p.kind === "team") && kind.whats && kind.whats.length ? kind.whats[Math.min(kind.whats.length - 1, Number(p.what) || 0)] : "";
  const sel = TR.sel && TR.sel.d === k && TR.sel.i === i;
  return `<button class="tr-per g-${GROUP_CLS[kind.group] || "f"} ${sel ? "on" : ""}" ${edit ? `data-act="tr-per" data-d="${k}" data-i="${i}"` : "disabled"} title="${esc(kind.grows || "")}">
    <b>${esc(twName(x, p))}</b><span>${p.min} min${!kind.field || twOwn(p) || p.kind === "walkthrough" ? "" : ` · ${esc(gear)} · ${esc(contact)}`}${sit ? ` · ${esc(sit)}` : ""}${p.who && p.who !== "all" ? ` · ${p.who === "ones" ? "two-deep" : "young"}` : ""}</span></button>`;
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
      ${(o.kinds.find((q) => q.kind === p.kind) || { whats: [] }).whats.length ? row(p.kind === "team" || p.kind === "situations" ? "Situation" : "Which", trSel("tr-pf", (o.kinds.find((q) => q.kind === p.kind) || { whats: [] }).whats.map((s, n) => [n, s]), p.what || 0, 'data-f="what"')) : ""}
    </div>
    <div class="st-actions"><button class="st-btn small glass" data-act="tr-pmove" data-dir="-1" ${i === 0 ? "disabled" : ""}>Earlier</button><button class="st-btn small glass" data-act="tr-pmove" data-dir="1">Later</button><button class="st-btn small glass" data-act="tr-pdup">Copy it</button><button class="st-btn small glass" data-act="tr-pdel">Remove</button><button class="st-btn small" data-act="tr-pdone">Done</button></div></div>`;
}
// The seven days side by side. `mode` is "week" or a template slot.
function weekBoard(days, x, mode, info) {
  const edit = !!info.edit;
  // The rule most days share is said once above the week; a day shows its
  // own only when it differs (a game day, a dead day, a camp day).
  const ruleCount = {};
  for (const m of info.days || []) if (m && m.rule) ruleCount[m.rule] = (ruleCount[m.rule] || 0) + 1;
  const common = Object.keys(ruleCount).sort((a, b) => ruleCount[b] - ruleCount[a])[0] || "";
  const cols = days.map((d, k) => {
    const f = trDayFacts(d, x);
    const meta = info.days ? info.days[k] : null;
    const rule = meta && meta.rule && meta.rule !== common ? `<div class="tr-rule" title="${esc(meta.rule)}">${esc(meta.rule)}</div>` : "";
    const head = `<div class="tr-dh ${meta && meta.today ? "today" : ""} ${meta && meta.past ? "past" : ""}"><b>${esc(meta ? meta.weekday : DAY_SHORT[k])}</b><small>${esc(meta ? meta.date : "")}</small></div>`;
    if (d.game) return `<div class="tr-day game ${meta && meta.today ? "today" : ""}">${head}<div class="tr-game">Game day</div>${rule}</div>`;
    const list = d.periods.map((p, i) => periodChip(p, k, i, edit, x)).join("");
    const empty = !d.periods.length ? `<div class="tr-off">${meta && meta.allowed && !meta.allowed.coaches ? "No coaches allowed" : "Day off"}</div>` : "";
    const sum = d.periods.length ? `<div class="tr-sum"><span>${f.min} min</span>${f.field ? `<span>${f.field} on the field</span>` : ""}${f.pads ? '<span class="pads">pads</span>' : ""}${f.full ? `<span class="hit">${f.full} full contact</span>` : ""}</div>` : "";
    const add = edit ? `<button class="tr-add" data-act="tr-padd" data-d="${k}">+ Period</button>` : "";
    return `<div class="tr-day ${TR.sel && TR.sel.d === k ? "sel" : ""} ${meta && meta.today ? "today" : ""}">${head}${list}${empty}${add}${sum}${rule}</div>`;
  }).join("");
  return `${common ? `<p class="tr-rules"><b>The rules this week</b> ${esc(common)}</p>` : ""}<div class="tr-week">${cols}</div>`;
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
// ---------- This week (update 2, lane 4, the agreed mock-up) ----------
// The rules against the hours, contact, game prep and effort; each day's
// clock with who runs each period and your read of him as a teacher; each
// player's prep (TR36-TR43); your staff's reasons (T1); the work by
// family; lift groups and body targets; the strength coach's numbers
// (T3); who's where. Every period, coach and player opens the side panel.
// Prep words are your staff's reads, never a true number.
const TW = { side: null };
const TW_WORDS = ["None", "Unprepared", "Underprepared", "Patchy", "Prepared", "Sharp"];
const TW_SUBS = new Set(["meeting", "film", "walkthrough", "install", "team_building", "recovery"]);
function twHm(m) {
  m = Math.max(0, Math.round(Number(m) || 0));
  const h = Math.floor(m / 60), r = m % 60;
  return h ? `${h} h${r ? " " + r : ""}` : `${r} min`;
}
function twClock(m) {
  const h = Math.floor(m / 60) % 24, mi = Math.round(m % 60);
  return `${h % 12 === 0 ? 12 : h % 12}:${String(mi).padStart(2, "0")} ${h < 12 ? "am" : "pm"}`;
}
const twMin = (p) => Number(p.min) || 0;
const twKind = (x, k) => x.options.kinds.find((o) => o.kind === k) || { kind: k, label: trCap(String(k).replace(/_/g, " ")), field: false, counted: true, film: false, strength: false, slot: "day", whats: [], by: [], family: null, family_full: null, grows: "" };
const twFull = (p) => p.contact === "thud" || p.contact === "live";
const twOwn = (p) => p.kind === "on_air" && Number(p.what) === 1;
const twCounted = (x, p) => twKind(x, p.kind).counted && !twOwn(p);
const twFamily = (x, p) => (twFull(p) ? twKind(x, p.kind).family_full : twKind(x, p.kind).family);
function twName(x, p) {
  const k = twKind(x, p.kind);
  if (twOwn(p)) return "Throwing on air (players' own)";
  if (TW_SUBS.has(p.kind) && k.whats.length) return k.whats[Math.min(k.whats.length - 1, Number(p.what) || 0)];
  if (p.kind === "weight_room" && Number(p.what) === 1) return "Weight room (in season)";
  return k.label;
}
// The period's detail line: a team period's situation, a field period's
// gear and contact, crowd noise.
function twDetail(x, p) {
  const k = twKind(x, p.kind);
  const bits = [];
  if ((p.kind === "team" || p.kind === "situations") && k.whats.length) bits.push(k.whats[Math.min(k.whats.length - 1, Number(p.what) || 0)]);
  if (k.field && !twOwn(p) && p.kind !== "walkthrough") {
    const g = (x.options.gears.find((o) => o.v === p.gear) || {}).t;
    if (g && p.gear !== "none") bits.push(g);
    if (p.contact !== "air") bits.push((x.options.contacts.find((o) => o.v === p.contact) || {}).t || p.contact);
  }
  if (p.noise) bits.push("crowd noise");
  if (p.who === "ones" && k.field) bits.push("two-deep");
  if (p.who === "young") bits.push("young players");
  return bits.join(" · ");
}
// Who runs it (TR3): the staff member for this kind of period.
function twRunner(x, p) {
  const by = twKind(x, p.kind).by || [];
  const key = by[Math.min(by.length - 1, Number(p.what) || 0)];
  return key && x.staff[key] ? { key, ...x.staff[key] } : null;
}
const twWord = (w) => `<span class="tw-word w${w}">${esc(TW_WORDS[w] || "None")}</span>`;
const twTrend = (five) => `<span class="tw-tr" title="The last five weeks, oldest first">${(five || []).map((v, i) => `<i class="${v && v <= 2 ? "a" : ""} ${i === 4 ? "now" : ""} ${v ? "" : "none"}" style="height:${v ? v * 4 : 2}px"></i>`).join("")}</span>`;
// A day laid out on the clock (TR2): the strength staff's work early, then
// classes, the dietitian at lunch, the day's work after classes, study
// hall and outings in the evening; a game day's periods lead up to the
// kickoff. Each item is a period (its index and start) or classes.
function twLayout(x, day, meta) {
  const c = (meta && meta.clock) || { early: null, classes: null, day: 540, evening: 1140, kick: null };
  const ps = day.periods;
  const items = [];
  const gap = (a, b) => (twKind(x, a.kind).field !== twKind(x, b.kind).field ? 15 : 5);
  if (day.game) {
    const kick = c.kick || 900;
    let t = kick - 120;
    for (let i = ps.length - 1; i >= 0; i--) {
      t -= twMin(ps[i]);
      items.unshift({ i, t });
      t -= 15;
    }
    items.push({ game: true, t: kick });
    return items;
  }
  const field = ps.some((p) => twKind(x, p.kind).field && !twOwn(p));
  const slot = (p) => {
    const s = twKind(x, p.kind).slot;
    if (s === "early" && c.early != null && (!field || p.kind === "weight_room")) return "early";
    if (s === "lunch" && c.classes) return "lunch";
    // Community service and a team outing are daytime; the dinners are in
    // the evening.
    if (s === "evening" && !(p.kind === "team_building" && (p.what === 1 || p.what === 3))) return "evening";
    return "day";
  };
  const by = { early: [], lunch: [], day: [], evening: [] };
  ps.forEach((p, i) => by[slot(p)].push(i));
  let t = c.early != null ? c.early : c.day;
  let last = null;
  for (const i of by.early) {
    if (last) t += gap(last, ps[i]);
    items.push({ i, t });
    t += twMin(ps[i]);
    last = ps[i];
  }
  const earlyEnd = by.early.length ? t : 0;
  if (c.classes) items.push({ cls: c.classes, t: c.classes.from });
  for (const i of by.lunch) items.push({ i, t: 735 });
  t = Math.max(c.day, c.classes ? c.classes.to + 25 : 0, by.early.length ? earlyEnd + 15 : 0);
  last = null;
  for (const i of by.day) {
    if (last) t += gap(last, ps[i]);
    items.push({ i, t });
    t += twMin(ps[i]);
    last = ps[i];
  }
  let ev = Math.max(c.evening, by.day.length ? t + 30 : 0);
  last = null;
  for (const i of by.evening) {
    if (ps[i].kind === "home_study") { items.push({ i, t: null, eve: true }); continue; }
    if (last) ev += 10;
    items.push({ i, t: ev });
    ev += twMin(ps[i]);
    last = ps[i];
  }
  return items.sort((a, b) => (a.t == null ? 1e9 : a.t) - (b.t == null ? 1e9 : b.t));
}
// The week's sums from the draft, so they move as you edit.
function twSums(x, days) {
  const s = { used: 0, film: 0, strength: 0, longest: 0, full: 0, off: 0, daily: [] };
  for (const d of days) {
    let dm = 0;
    if (d.game) dm = 180;
    else {
      for (const p of d.periods) {
        if (twFull(p)) s.full += twMin(p);
        if (!twCounted(x, p)) continue;
        dm += twMin(p);
        const k = twKind(x, p.kind);
        if (k.film) s.film += twMin(p);
        if (k.strength || p.kind === "weight_room") s.strength += twMin(p);
      }
      if (d.periods.every((p) => !twCounted(x, p))) s.off++;
    }
    s.used += dm;
    s.longest = Math.max(s.longest, dm);
    s.daily.push(dm);
  }
  return s;
}
const twCell = (k, v, cls = "") => `<div class="tw-cell"><span class="tw-k">${k}</span><span class="tw-v ${cls}">${v}</span></div>`;
const twPh = (a, b) => `<div class="tw-ph"><span class="tw-k">${a}</span>${b ? `<span class="tw-k">${b}</span>` : ""}</div>`;
// "12 of 15 practices this block" → ["Practices this block", "12 of 15"].
const twCount = (c) => { const w = String(c).split(" "); return [trCap(w.slice(3).join(" ")), w.slice(0, 3).join(" ")]; };
function twHoursCard(x, s) {
  const h = x.cards.hours;
  const counts = (h.counts || []).map(twCount);
  if (h.cap) {
    const over = s.used > h.cap;
    const film = h.game ? twCell("Game day", "3 h <small>counted flat</small>") : twCell("Film", `${twHm(s.film)}${h.film_cap ? ` <small>of ${twHm(h.film_cap)}</small>` : ""}`);
    return `${twPh("Hours against the rule", esc(h.label))}
      <div class="tw-hero"><b>${twHm(s.used)}</b><small>of ${twHm(h.cap)}</small></div>
      <div class="tw-bar"><i class="${over ? "a" : ""}" style="width:${Math.min(100, (100 * s.used) / h.cap)}%"></i></div>
      <div class="tw-cells">${twCell("Strength, speed, conditioning", twHm(s.strength))}${film}${twCell("Longest day", `${twHm(s.longest)}${h.day_cap_text ? ` <small>of ${esc(h.day_cap_text)}</small>` : ""}`)}${twCell("Days off", `${h.off_need} required <small>· ${s.off} kept</small>`, s.off < h.off_need ? "amber" : "")}</div>
      <div class="tw-mini ${over ? "amber" : ""}">${over ? `${twHm(s.used - h.cap)} over the rule.` : `${twHm(h.cap - s.used)} left.`}${counts.length ? ` This block: ${counts.map(([k, v]) => `${v} ${k.toLowerCase().replace(/ this block$/, "")}`).join(", ")}.` : ""}</div>`;
  }
  const cells = [twCell("Longest day", twHm(s.longest)), twCell("Days off", `${h.off_need} required <small>· ${s.off} kept</small>`, s.off < h.off_need ? "amber" : "")].concat(counts.slice(0, 4).map(([k, v]) => twCell(esc(k), esc(v))));
  return `${twPh("Time against the rule", esc(h.label))}
    <div class="tw-hero"><b>${twHm(s.used)}</b><small>of team work this week</small></div>
    <div class="tw-bar"><i style="width:${Math.min(100, (100 * s.used) / (x.tier === "hs" ? 600 : 1500))}%"></i></div>
    <div class="tw-cells">${cells.join("")}</div>
    <div class="tw-mini">${x.tier === "nfl" ? "Phases, OTA days and padded practices are the CBA's limits." : x.tier === "college" ? "No hours to count: the NCAA keeps the staff from requiring anything this block." : "The state counts practices and contact, not hours."}</div>`;
}
function twContactCard(x, days) {
  const c = x.cards.contact;
  if (!c.allowed) {
    return `${twPh("Contact", "this block")}<div class="tw-hero"><b>None</b></div><div class="tw-lede">No football on the field this block</div>
      <div class="tw-cells">${twCell("Full contact", "0 · not allowed")}${twCell("Pads", "Not allowed")}${twCell("Footballs", x.own ? "Players' own only" : "Not allowed")}${twCell("On the field", x.own ? "No staff" : "Strength staff only")}</div>`;
  }
  let full = 0;
  const per = [];
  days.forEach((d, k) => {
    if (d.game || !d.periods.some((p) => twKind(x, p.kind).field && !twOwn(p))) return;
    const m = d.periods.filter(twFull).reduce((a, p) => a + twMin(p), 0);
    full += m;
    const what = [...new Set(d.periods.filter(twFull).map((p) => twKind(x, p.kind).label.toLowerCase()))];
    const gears = ["none", "helmets", "shells", "pads"];
    const gear = d.periods.reduce((g, p) => Math.max(g, gears.indexOf(p.gear)), 0);
    per.push([DAY_SHORT[k], m ? `${m} min <small>(${esc(what.join(", "))})</small>` : `0 <small>· ${esc(((x.options.gears.find((o) => o.v === gears[gear]) || {}).t || "").toLowerCase())}</small>`]);
  });
  const over = c.week_cap != null && full > c.week_cap;
  return `${twPh("Contact", "this week")}<div class="tw-hero"><b>${full} min</b><span class="tw-w">full contact</span></div><div class="tw-lede">Your staff's week: ${c.staff} min · up to ${esc(c.pads.toLowerCase())}, ${esc(c.max_contact.toLowerCase())}</div>
    <div class="tw-cells">${per.slice(0, 4).map(([k, v]) => twCell(k, v)).join("") || twCell("Field days", "None this week")}</div>
    ${over ? `<div class="tw-mini amber">Over the week's full-contact limit of ${c.week_cap} min.</div>` : c.week_cap != null ? `<div class="tw-mini">The rule: up to ${c.week_cap} min of full contact a week.</div>` : ""}`;
}
function twPrepCard(x) {
  const g = x.cards.gp;
  if (g.game) {
    return `${twPh("Game prep", "this week")}<div class="tw-hero"><b>${g.mine}</b><span class="tw-w">of ${g.staff} game plan minutes, your staff's</span></div><div class="tw-lede">Installs, the opponent's film, walkthroughs and situations, weighted by how much each readies the plan</div>
      <div class="tw-cells">${twCell("Install", `about ${g.calls} new calls`)}${twCell("Opponent film", esc(g.opp_film_text))}${twCell("All film", esc(g.film_text))}${twCell("Saved week", "Updates when you save")}</div>`;
  }
  const season = x.block === "season" || x.block === "postseason";
  return `${twPh("Game prep", "film is not game prep")}<div class="tw-hero"><b class="tw-sm">${season ? "No game this week" : "No game ahead"}</b></div><div class="tw-lede">${esc(g.opener ? trCap(g.opener) : "No game on the schedule yet")}</div>
    <div class="tw-cells">${twCell("The system on film", esc(g.system_text))}${twCell("All film", esc(g.film_text))}</div>
    <div class="tw-mini">Film this week is learning the system, not game prep: it counts against the film hours, never as game plan minutes.</div>`;
}
function twEffortCard(x) {
  const e = x.effort;
  if (e.none) {
    return `${twPh("Prep and effort", "no read")}<div class="tw-hero"><b class="tw-sm">The players' own time</b></div><div class="tw-lede">The rules keep your staff away, so nobody reads the week. The words start again when the staff may work with them.</div>`;
  }
  const max = Math.max(1, ...[1, 2, 3, 4, 5].map((w) => e.words[w]));
  return `${twPh("Prep and effort", e.attend != null ? `${e.attend}% of required sessions` : "")}
    <div class="tw-words">${[5, 4, 3, 2, 1].map((w) => `<div><i class="${w <= 2 ? "a" : ""}" style="height:${(e.words[w] / max) * 26 + 2}px"></i></div>`).join("")}</div>
    <div class="tw-wl">${[5, 4, 3, 2, 1].map((w) => `<span><b class="${w <= 2 && e.words[w] ? "amber" : ""}">${e.words[w]}</b>${w === 2 ? "Under&shy;prepared" : TW_WORDS[w]}</span>`).join("")}</div>
    <div class="tw-cells">${twCell("Missed sessions", `${e.missed}${e.absent ? ` <small>· ${e.absent} absent</small>` : ""}`)}${twCell("Limited or rested", `${e.limited}${e.out ? ` <small>· ${e.out} in rehab</small>` : ""}`)}${twCell("New this week", `${e.new}${e.away ? ` <small>· ${e.away} away part-time</small>` : ""}`)}${twCell("Extra work", `${e.extra} player${e.extra === 1 ? "" : "s"}`)}</div>
    <div class="tw-mini">Effort from the ${x.tier === "hs" ? "coaches" : "strength staff"}: <b>${esc(e.word || "no read")}</b>.${e.voluntary ? " The sessions are voluntary: a missed one breaks no rule, but the staff notices." : ""}</div>`;
}
function twYear(x) {
  const y = x.year;
  if (!y || !y.phases.length) return "";
  return `<div class="tw-next"><div class="tw-ph"><span class="tw-k">Where the year is</span><span class="tw-k">${esc(S.status ? S.status.today : "")}</span></div>
    <div class="tw-line">${y.phases.map((p) => `<div class="tw-seg ${p.done ? "done" : ""} ${p.now ? "now" : ""}" style="flex:${p.weeks};--p:${p.p}%" title="${esc(`${p.long}: ${p.from} – ${p.to}`)}"></div>`).join("")}</div>
    <div class="tw-labels">${y.phases.map((p) => `<span style="flex:${p.weeks}" class="${p.now ? "on" : ""}" title="${esc(p.long)}">${esc(p.label)}</span>`).join("")}</div>
    <div class="tw-say">${esc(y.say)}</div></div>`;
}
// One day's column: the required day off, the periods on the clock, the
// add button and the day's counted minutes.
function twDay(x, d, k, meta, edit) {
  const items = twLayout(x, d, meta);
  const filmCap = x.cards.hours.film_cap;
  let counted = 0, field = 0;
  const body = items.map((it) => {
    if (it.cls) return `<div class="tw-per class"><div class="t"><span>${twClock(it.cls.from)}</span></div><div class="n">Classes</div><div class="m">${esc(it.cls.name)} · until ${esc(twClock(it.cls.to))}</div></div>`;
    if (it.game) return `<div class="tw-per game"><div class="t"><span>${twClock(it.t)}</span><span class="cnt">Kickoff</span></div><div class="n">${esc(meta && meta.game_text ? meta.game_text : "Game day")}</div><div class="m">The game day counts 3 hours</div></div>`;
    const p = d.periods[it.i];
    const kd = twKind(x, p.kind);
    const cnt = twCounted(x, p);
    if (cnt && !d.game) counted += twMin(p);
    if (kd.field && !twOwn(p)) field += twMin(p);
    const label = d.game ? "In the game day" : !cnt ? "Not counted" : kd.film && filmCap ? "Counted as film" : "Counted";
    const wx = meta && meta.weather;
    const place = kd.field || p.kind === "mat_drills" || p.kind === "testing" ? (kd.field && !(wx && wx.place === "inside") ? " · Outside" : " · Indoors") : "";
    const fam = twFamily(x, p);
    const who = twRunner(x, p);
    const on = TR.sel && TR.sel.tpl == null && TR.sel.d === k && TR.sel.i === it.i;
    const detail = twDetail(x, p);
    return `<button class="tw-per ${twFull(p) ? "contact" : ""} ${cnt || d.game ? "" : "free"} ${on ? "on" : ""}" data-act="tr-wper" data-d="${k}" data-i="${it.i}" title="${esc(kd.grows || "")}">
      <div class="t"><span>${it.eve ? "Evening" : twClock(it.t)}</span><span class="cnt">${label}</span></div>
      <div class="n">${esc(twName(x, p))}</div>
      <div class="m">${twMin(p)} min${place}${fam ? ` · ${esc(fam)}` : ""}${detail ? ` · ${esc(detail)}` : ""}</div>
      ${who ? `<div class="who">${esc(who.name)}${who.words ? ` · ${esc(who.words)}` : ""}</div>` : ""}</button>`;
  }).join("");
  const off = meta && meta.off ? `<div class="tw-off ${meta.off === "req" ? "req" : ""}"><b>${meta.off === "req" ? "Required day off" : "Day off"}</b><span>${esc(meta.off === "req" ? meta.off_why : "Your staff's choice")}</span></div>` : "";
  const none = !d.periods.length && !d.game && !off ? `<div class="tw-off"><b>${meta && meta.allowed && !meta.allowed.coaches ? "No coaches allowed" : "Nothing planned"}</b></div>` : "";
  const wx = meta && meta.weather && field ? meta.weather : null;
  const wxText = wx ? `${wx.temp}°${wx.why ? ` · ${wx.why}` : ""}${wx.place === "inside" ? " · moved indoors" : wx.place === "lost" ? " · field work lost" : wx.place === "delayed" ? " · lightning hold" : ""}` : "";
  const add = edit && !d.game ? `<button class="tw-addp" data-act="tr-padd" data-d="${k}">+ Period</button>` : "";
  const foot = d.game ? "3 h counted" : `${twHm(counted)} counted${field ? ` · ${twHm(field)} on the field` : ""}`;
  return `<div class="tw-day ${meta && meta.today ? "today" : ""} ${meta && meta.past ? "past" : ""}"><div class="tw-dh"><b>${esc(meta ? meta.weekday : DAY_SHORT[k])}</b><span>${esc(meta ? String(meta.date).split(" ").slice(1, 3).join(" ") : "")}</span></div>${wxText ? `<div class="tw-wx ${wx.place !== "outside" ? "moved" : ""}">${esc(wxText)}</div>` : ""}${off}${none}${body}${add}<div class="tw-df"><span>${foot}</span></div></div>`;
}
// What a kind of period is, for the side panel (by sub-kind where it has
// them).
const TW_IS = {
  warmup: "The team warm-up and stretch, by unit.",
  individual: "Each position coach with his room: footwork, hands and the position's technique.",
  inside_run: "The run game inside the tackles, offense against defense.",
  seven_on_seven: "The passing game without linemen: routes against coverage.",
  one_on_ones: "One against one: receivers and backs against defensive backs, linemen against linemen.",
  team: "Eleven on eleven: base plays, or this week's install against the scout team's look.",
  scrimmage: "A live, game-like scrimmage with officials and the chains.",
  situations: "Game situations against a look: third down, red zone, two-minute and the rest.",
  special_teams: "Coverage, returns, punts and field goals with the units.",
  walkthrough: ["The plan at a jog: who goes where, without wear.", "A full rehearsal of the game day, from the first snap to the last.", "The night before a road game, in a hotel ballroom."],
  conditioning: "Running for fitness: 110s, gassers and tempo runs.",
  speed: "Starts, change of direction and the shuttle with the strength staff.",
  weight_room: ["Lifting in groups at set times with the strength staff.", "In-season lifting: shorter, to hold strength through the season."],
  meeting: ["The whole team with the head coach.", "Offense, defense and special teams apart, with the coordinators.", "Each room with its position coach."],
  film: ["The next opponent's tape, cut by your staff.", "Your own tendencies, read the way an opponent would read them.", "Yesterday's practice, graded.", "Third downs, red zone and two-minute from your games and the league's.", "The system you bring in, on cut-ups.", "The last game, every snap graded."],
  young: "The young players' own period: more reps while the two-deep rests.",
  recovery: ["Treatment, cold tubs, massage and sleep plans with the trainers.", "The injured with the trainers, step by step back to practice."],
  mat_drills: "Dawn stations by unit, a winner each morning. Past two mornings a week it costs mood.",
  testing: "The strength staff's testing: bench, squat, clean, the 40, the shuttle and the vertical.",
  install: ["New calls on the board before they're repped: base downs.", "New calls on the board before they're repped: third down.", "New calls on the board before they're repped: red zone and goal line.", "New calls on the board before they're repped: two-minute and situations.", "The system you bring in, piece by piece."],
  study_hall: "Flagged players with the academic staff.",
  on_air: ["Quarterbacks and receivers running routes without a defense.", "The players' own throwing session, no staff allowed: the quarterback's timing with his receivers."],
  ball_security: "The gauntlet, the JUGS machine and ball-carrying drills.",
  kick_stations: "Snappers, holders and kickers at their stations; punts in the wind.",
  mobility: "Hips, ankles and the back with the strength staff.",
  vision: "Light boards and strobe glasses: reaction and tracking.",
  vr_reps: "Headset reps of the protections and reads: mental reps with no wear.",
  mental_skills: "Composure and routines with the sports psychologist; the players opt in.",
  rules_clinic: "Officials walk the team through this year's points of emphasis.",
  plan_test: "A quiz on the game plan: your staff sees who knows it.",
  nutrition: "The body plan with the dietitian, by position group.",
  council: "Elected players with the head coach: the team's voice, bonds and captains.",
  team_building: ["A team dinner.", "Community service in town.", "The pasta dinner before game day.", "A team outing."],
  signals: "The calls, checks and hand signals, in noise.",
  home_study: "The plan on his own time: tablet, Hudl or the playbook. The hard workers take more from it.",
};
const twIs = (p) => { const v = TW_IS[p.kind]; return Array.isArray(v) ? v[Math.min(v.length - 1, Number(p.what) || 0)] : v || ""; };
function twCountedText(x, p, game) {
  if (game) return "Inside the game day's 3 hours";
  if (!twCounted(x, p)) return twOwn(p) ? "Not counted: the players' own, no staff" : "Not counted: medical, academic, voluntary or his own";
  return twKind(x, p.kind).film && x.cards.hours.film_cap ? "Counted against the rule, as film" : "Counted against the rule";
}
// The side panel: a period (with the editor when you run practice), a
// coach, a player's prep or the rules.
function twSide(x, draft, edit) {
  let h = "";
  const pr = (k) => `<div class="tw-k">${k}</div>`;
  if (TR.sel && TR.sel.tpl == null && draft.days[TR.sel.d] && draft.days[TR.sel.d].periods[TR.sel.i]) {
    const { d: dk, i } = TR.sel;
    const day = draft.days[dk];
    const meta = x.days[dk];
    const p = day.periods[i];
    const kd = twKind(x, p.kind);
    const it = twLayout(x, day, meta).find((q) => q.i === i);
    const who = twRunner(x, p);
    const fam = twFamily(x, p);
    const famGrows = fam ? (x.options.grows.find((g) => g[0] === fam) || [])[1] : "";
    const whoCell = who ? (who.group ? `<a class="link" data-act="tr-wcoach" data-key="${who.key}">${esc(who.name)}</a>` : who.you ? "You" : who.id != null ? `<a class="link" data-act="tr-wcoach" data-key="${who.key}">${esc(who.name)}</a>` : esc(who.name)) : "—";
    h = `${pr(`Period · ${esc(meta.weekday)} ${esc(String(meta.date).split(" ").slice(1, 3).join(" "))}`)}<h2>${esc(twName(x, p))}</h2>
      <div class="tw-lede">${it && it.t != null ? twClock(it.t) : "Evening"} · ${twMin(p)} min${twDetail(x, p) ? ` · ${esc(twDetail(x, p))}` : ""}</div>
      <div class="tw-cells">${twCell("Runs it", whoCell)}${twCell(who && who.teach === false ? "Your read of his work" : "Teaching", who && who.words ? `${esc(who.words)}${who.sure ? ` <small>· ${esc(who.sure.toLowerCase())}</small>` : ""}` : "—")}${twCell("Family", esc(fam || "None"))}${twCell("The rule", esc(twCountedText(x, p, day.game)))}</div>
      <div class="tw-sec"><span class="tw-k">What it is</span><p>${esc(twIs(p))}</p></div>
      <div class="tw-sec"><span class="tw-k">What it grows</span><p>${esc(trCap(kd.grows || ""))}${famGrows ? `<br><span class="muted">${esc(fam)}: ${esc(famGrows)}</span>` : ""}</p></div>`;
    if (edit && !day.game) h += twEditor(x, p, i, meta.allowed);
    else if (!edit) h += `<p class="muted tw-note">${x.own ? "The players are on their own: nothing here is your staff's." : "The head coach runs practice: you see his week."}</p>`;
  } else if (TW.side && TW.side.kind === "coach") {
    const base = x.staff[TW.side.key] || {};
    const m = TW.side.n != null && base.group ? base.group[TW.side.n] : base;
    if (m && m.group) {
      h = `${pr("Staff")}<h2>${esc(m.name)}</h2><div class="tw-lede">${esc(m.title || "")}</div><div class="tw-sec">${m.group.map((g, n) => `<p><a class="link" data-act="tr-wcoach" data-key="${TW.side.key}" data-n="${n}">${esc(g.name)}</a> <span class="muted">· ${esc(g.title || "")}${g.words ? ` · ${esc(g.words)}` : ""}</span></p>`).join("") || "<p>Nobody in the job.</p>"}</div>`;
    } else if (m) {
      const runs = [];
      draft.days.forEach((day, dk) => twLayout(x, day, x.days[dk]).forEach((it) => {
        if (it.i == null) return;
        const p = day.periods[it.i];
        const w = twRunner(x, p);
        const same = w && (w.key === TW.side.key || w.name === m.name || (w.group && w.group.some((g) => g.name === m.name)));
        if (same) runs.push(`<p>${DAY_SHORT[dk]} ${it.t != null ? twClock(it.t) : "evening"} · ${esc(twName(x, p))}</p>`);
      }));
      const rg = m.range ? `<div class="tw-range"><i style="left:${((m.range[0] - 1) / 19) * 100}%;width:${Math.max(3, ((m.range[1] - m.range[0]) / 19) * 100)}%"></i></div><div class="tw-l2">${m.teach ? "Teaching" : "His job"}, your staff's read on 1-20: ${m.range[0]}-${m.range[1]}</div>` : "";
      h = `${pr("Staff")}<h2>${esc(m.name)}</h2><div class="tw-lede">${esc(m.title || "")}</div>
        ${m.you ? "<p>You run it.</p>" : `<div class="tw-cells">${twCell(m.teach ? "Your read as a teacher" : "Your read of his work", esc(m.words || "Not seen yet"))}${twCell("How sure", esc(m.sure || "—"))}</div>${rg}`}
        <div class="tw-sec"><span class="tw-k">This week</span>${runs.join("") || "<p>No periods this week.</p>"}</div>
        ${m.id != null && !m.you ? `<div class="tw-acts"><a class="st-btn small glass" data-go="coach" data-id="${m.id}">Coach page</a><a class="st-btn small glass" data-go="coach" data-id="${m.id}" data-tab="Ratings">Ratings</a></div>` : ""}`;
    }
  } else if (TW.side && TW.side.kind === "player") {
    const r = x.players.find((q) => q.id === TW.side.id);
    if (r && r.prep) {
      const q = r.prep;
      const off = !x.cards.gp.game;
      const where = off ? "in practice" : "in this week's game";
      const means =
        q.word === 5
          ? `Sharp: a little cleaner than usual ${where}, about 7% fewer mental errors.`
          : q.word === 4
            ? `Prepared: his normal level ${where}, no extra mental errors.`
            : q.word === 3
              ? `Patchy: a few more busts on the new calls ${where}, nothing on what he knows.`
              : q.word
                ? `Poor prep: more busts, wrong gaps and late reads ${where}, through the battles. Up to about a third more mental errors. A poorly prepared player can still have a good day.`
                : "No read this week.";
      const rust = q.rust ? "After a run of poor weeks the dip is in his ratings: up to 3 points off his reading and technique. Two or three good weeks win it back." : "No rust: his five-week run is fine.";
      const ex = r.plan && r.plan.extra ? r.plan.extra : {};
      h = `${pr("Prep this week")}<h2>${esc(r.name)}</h2><div class="tw-lede">${esc(r.pos)} · ${esc(r.class || "")} · ${esc(r.role || "")}</div>
        <div class="tw-prow">${twWord(q.word)}${twTrend(q.five)}<span class="${q.rust ? "amber" : "muted"}">${q.rust ? `Rusty: ${esc(q.rust)}` : "No rust"}</span></div>
        <div class="tw-cells">${twCell(off ? "Work" : "Reps on the plan", esc(q.work))}${twCell(off ? "Meetings" : "Installs", esc(q.meet))}${twCell("Film", esc(q.film))}${twCell("Walkthroughs", esc(q.walks))}${twCell("Own study", esc(q.own))}${twCell("Practice status", esc(q.status_text), q.limited ? "amber" : "")}</div>
        ${q.says ? `<div class="tw-sec"><span class="tw-k">Your staff says</span><p class="${q.warn ? "amber" : ""}">${esc(q.says)}</p></div>` : ""}
        <div class="tw-sec"><span class="tw-k">What it means</span><p>${esc(means)}</p><p>${esc(rust)}</p></div>
        <div class="tw-acts"><a class="st-btn small glass" data-go="player" data-id="${r.id}">Player page</a><a class="st-btn small glass" data-go="player" data-id="${r.id}" data-tab="Development">Individual plan</a><button class="st-btn small glass" data-act="pptalkmenu" data-id="${r.id}">Talk to him</button>${r.may_plan !== false && !ex.film ? `<button class="st-btn small glass" data-act="tr-wextra" data-id="${r.id}">Extra film</button>` : ""}</div>`;
    }
  } else if (TW.side && TW.side.kind === "rule") {
    const org = x.tier === "college" ? "NCAA" : x.tier === "hs" ? "The state association" : "The NFL's CBA";
    const rules = [...new Set(x.days.map((d) => d.rule).filter(Boolean))];
    h = `${pr("The rules this week")}<h2>${esc(org)}</h2><p>${esc(x.rule || "")}</p>
      ${rules.length > 1 ? `<div class="tw-sec"><span class="tw-k">By day</span>${x.days.map((d) => `<p><b>${esc(d.weekday.slice(0, 3))}</b> ${esc(d.rule || "No rule")}</p>`).join("")}</div>` : ""}
      <div class="tw-sec"><p>A required day off shows with a ring; a day off your staff chose has none.</p><p>Break a rule and the penalty is real: lost practices, fines and the news.</p></div>`;
  }
  if (!h) return "";
  return `<aside class="tw-side" aria-label="Details"><button class="tw-x" data-act="tr-wclose" aria-label="Close">×</button>${h}</aside>`;
}
// The period's editor inside the side panel.
function twEditor(x, p, i, allowed) {
  const o = x.options;
  const kd = twKind(x, p.kind);
  const row = (label, html, note) => `<label>${label}${note ? `<small>${esc(note)}</small>` : ""}</label>${html}`;
  const groups = {};
  for (const q of o.kinds) (groups[q.family || "Other"] = groups[q.family || "Other"] || []).push(q);
  const kindSel = `<select class="tr-sel" data-act="tr-pf" data-f="kind">${Object.entries(groups).map(([g, list]) => `<optgroup label="${esc(g)}">${list.map((q) => `<option value="${q.kind}" ${q.kind === p.kind ? "selected" : ""}>${esc(q.label)}</option>`).join("")}</optgroup>`).join("")}</select>`;
  const field = kd.field && !twOwn(p);
  return `<div class="tw-sec tw-edit"><span class="tw-k">Change it</span><div class="gd-form">
    ${row("What", kindSel)}
    ${kd.whats.length ? row(p.kind === "team" || p.kind === "situations" ? "Situation" : "Which", trSel("tr-pf", kd.whats.map((w, n) => [n, w]), Number(p.what) || 0, 'data-f="what"')) : ""}
    ${row("Minutes", `<input class="tr-num" type="number" min="5" max="180" step="5" value="${twMin(p)}" data-act="tr-pf" data-f="min">`)}
    ${field ? row("Gear", trSel("tr-pf", o.gears.map((g) => [g.v, g.t]), p.gear, 'data-f="gear"'), allowed ? `the rule: up to ${allowed.gear}` : "") : ""}
    ${field ? row("Contact", trSel("tr-pf", o.contacts.map((c) => [c.v, c.t]), p.contact, 'data-f="contact"'), allowed ? `the rule: up to ${allowed.contact}` : "") : ""}
    ${field ? row("Tempo", trSel("tr-pf", o.tempos, p.tempo || "normal", 'data-f="tempo"')) : ""}
    ${field ? row("Who", trSel("tr-pf", o.whos, p.who || "all", 'data-f="who"')) : ""}
    ${field ? row("Crowd noise", `<label class="gd-check"><input type="checkbox" data-act="tr-pf" data-f="noise" ${p.noise ? "checked" : ""}> Speakers on, a silent count</label>`) : ""}
    </div>
    <div class="tw-acts"><button class="st-btn small glass" data-act="tr-pmove" data-dir="-1">Earlier</button><button class="st-btn small glass" data-act="tr-pmove" data-dir="1">Later</button><button class="st-btn small glass" data-act="tr-pdup">Copy it</button><button class="st-btn small glass" data-act="tr-pdel">Remove</button></div></div>`;
}
// The prep table (TR36, TR43): the two-deep by default.
function twPrepTable(x) {
  const off = !x.cards.gp.game;
  const k = keep("twprep", { v: "two" });
  const rows = x.players.filter((r) => r.prep && (k.v === "all" || (k.v === "poor" ? (r.prep.word > 0 && r.prep.word <= 2) || !!r.prep.rust : r.role !== "Scout team and young")));
  const cols = [
    { k: "name", t: "Player", l: true, v: (r) => r.name, f: (r) => `<a class="link name" data-act="tr-wplayer" data-id="${r.id}">${esc(r.name)}</a> <span class="muted">${esc(r.pos)}</span><div class="tw-l2">${esc(r.class || "")} · ${esc(r.role || "")}</div>` },
    { k: "status", t: "Status", l: true, v: (r) => r.prep.status_text, f: (r) => `<span class="tw-stat ${r.prep.limited ? "lim" : ""}">${esc(r.prep.status_text)}</span>` },
    { k: "work", t: off ? "Work" : "Reps on the plan", l: true, div: true, wrap: true, v: (r) => r.prep.work, f: (r) => esc(r.prep.work || "") },
    { k: "meet", t: off ? "Meetings" : "Installs", l: true, v: (r) => r.prep.meet, f: (r) => esc(r.prep.meet || "") },
    { k: "film", t: "Film", l: true, wrap: true, v: (r) => r.prep.film, f: (r) => esc(r.prep.film || "") },
    { k: "walks", t: "Walk\u00ADthroughs", l: true, v: (r) => r.prep.walks, f: (r) => esc(r.prep.walks || "") },
    { k: "own", t: "Own study", l: true, wrap: true, v: (r) => r.prep.own, f: (r) => esc(r.prep.own || "") },
    { k: "word", t: "Prep this week", l: true, div: true, tip: "Your staff's word for his week, from what he really did", v: (r) => r.prep.word, f: (r) => twWord(r.prep.word) },
    { k: "five", t: "5 weeks", l: true, tip: "The last five weeks, oldest first", v: (r) => r.prep.five.reduce((a, b) => a + b, 0), f: (r) => twTrend(r.prep.five) },
    { k: "rust", t: "Rust", l: true, v: (r) => r.prep.rust, f: (r) => (r.prep.rust ? `<span class="amber tw-rustc">${esc(r.prep.rust)}</span>` : '<span class="muted">None</span>') },
    { k: "says", t: "Your staff says", l: true, wrap: true, v: (r) => r.prep.says, f: (r) => (r.prep.says ? `<span class="tw-says ${r.prep.warn ? "amber" : ""}">${esc(r.prep.says)}</span>` : '<span class="muted">—</span>') },
  ];
  const seg = `<div class="seg tw-seg2">${[["two", "The two-deep"], ["poor", "Poorly prepared or rusty"], ["all", "Everyone"]].map(([v, t]) => `<button data-act="tr-wprep" data-v="${v}" aria-pressed="${k.v === v}">${t}</button>`).join("")}</div>`;
  return `${seg}${table("trprep", cols, rows, { groupBy: (r) => r.pos, empty: k.v === "poor" ? "Nobody poorly prepared or rusty this week." : "No players." })}`;
}
// The week's work by family, yours against your staff's own week.
function twFamilies(x, days) {
  const mine = {};
  for (const d of days) for (const p of d.periods) { const f = twFamily(x, p); if (f) mine[f] = (mine[f] || 0) + twMin(p); }
  const max = Math.max(1, ...x.families.map((f) => Math.max(mine[f.family] || 0, f.staff)));
  return `${twPh("The week's work", "by family, minutes")}${x.families.map((f) => {
    const m = mine[f.family] || 0;
    const cmp = !m && !f.staff ? "none this week" : m > f.staff * 1.1 + 5 ? "more than the staff's" : m < f.staff * 0.9 - 5 ? "less than the staff's" : "as the staff";
    return `<div class="tw-fam" title="${esc(f.grows)}"><span>${esc(f.family)}</span><div class="tw-bar"><i style="width:${(100 * m) / max}%"></i>${f.staff ? `<u style="left:${(100 * f.staff) / max}%"></u>` : ""}</div><span class="v">${m ? `${twHm(m)} · ` : ""}${cmp}</span></div>`;
  }).join("")}<div class="tw-mini">The tick is your staff's own week. Growth leans the way the minutes go, a little: the most a plan moves a family is about a seventh, and nobody grows past what his body allows.</div>`;
}
// Lift groups (TR4) with their times from the week's first lifting day.
function twGroups(x, days) {
  let at = null;
  const lift = [];
  days.forEach((d, k) => {
    if (!d.game && d.periods.some((p) => p.kind === "weight_room")) lift.push(DAY_SHORT[k]);
    if (at || d.game) return;
    const i = d.periods.findIndex((p) => p.kind === "weight_room");
    if (i < 0) return;
    const lay = twLayout(x, d, x.days[k]);
    const it = lay.find((q) => q.i === i);
    const c = x.days[k].clock || {};
    // The afternoon is free from the end of classes to the day's first
    // period after them (or the evening).
    const after = c.classes ? lay.filter((q) => q.i != null && q.t != null && q.t >= c.classes.to).map((q) => q.t) : [];
    const pm = after.length ? Math.min(...after) : c.evening || 1140;
    if (it && it.t != null) at = { t: it.t, m: twMin(d.periods[i]), cls: c.classes || null, day: c.day, pm };
  });
  // The groups take the morning's slots before classes (or the day's
  // start), then the afternoon's between classes and the day's start; a
  // group with no slot lifts after practice, rehab with the trainers.
  const slots = [];
  if (at) {
    const step = at.m + 10;
    const end = at.cls && at.cls.from > at.t ? at.cls.from - 10 : at.day != null ? at.day - 5 : 24 * 60;
    for (let t = at.t; t + at.m <= end && slots.length < 4; t += step) slots.push(t);
    if (at.cls) for (let t = at.cls.to + 10; t + at.m <= at.pm - 5 && slots.length < 4; t += step) slots.push(t);
  }
  const shown = x.groups.filter((g) => g.count > 0);
  const order = shown.filter((g) => g.at >= 0);
  const time = (g) => {
    if (g.at === -1) return "with the trainers";
    if (g.at === -2) return "after practice";
    if (!at) return "no lifting this week";
    const k = order.indexOf(g);
    if (slots[k] != null) return twClock(slots[k]);
    return /rehab/i.test(g.name) ? "with the trainers, around classes" : "after practice";
  };
  return `${twPh("Lift groups", lift.length ? esc(lift.join(" ")) : "")}<ul class="tw-list">${shown.map((g) => {
    const s = x.staff[g.by];
    return `<li><span><b>${esc(g.name)}</b><span class="muted"> · ${g.count}</span></span><span>${esc(time(g))}${s ? ` · <a class="link" data-act="tr-wcoach" data-key="${g.by}">${esc(s.name)}</a>` : ""}</span></li>`;
  }).join("")}</ul>`;
}
function twBody(x) {
  return `${twPh("Body targets by group", "from the dietitian and strength staff")}<table class="tw-small"><tr><th>Group</th><th>Now</th><th>Target</th><th></th></tr>${x.body.map((b) => `<tr><td>${esc(b.group)} <span class="muted">· ${b.count}</span></td><td>${esc(b.now)}</td><td class="muted">${esc(b.target)}</td><td class="${b.warn ? "amber" : "muted"}">${esc(b.status)}</td></tr>`).join("")}</table>
    <div class="tw-mini">${trPlural(x.body.reduce((a, b) => a + b.planned, 0), "player")} ${x.body.reduce((a, b) => a + b.planned, 0) === 1 ? "has" : "have"} a body target you set; the rest grow into their frames.</div>`;
}
// The strength coach's numbers (TR5, T3).
function twNumbers(x) {
  const n = x.numbers;
  const coach = x.staff.str ? x.staff.str.name : "the strength staff";
  if (!n.rows.length) return `<div class="panel tw-panel"><div class="empty">No testing yet this year.${n.next ? ` The next testing day is ${esc(n.next)}.` : ""}</div></div>`;
  const cols = [
    { k: "name", t: "Player", l: true, f: (r) => playerLink(r.id, r.name, "Development") },
    { k: "pos", t: "Pos", l: true },
    { k: "bench", t: n.pro ? "225 bench" : "Bench", v: (r) => parseFloat(r.bench) || 0, f: (r) => esc(r.bench) },
    { k: "squat", t: "Squat", hide: n.pro, v: (r) => parseFloat(r.squat) || 0, f: (r) => esc(r.squat) },
    { k: "clean", t: "Clean", hide: n.pro, v: (r) => parseFloat(r.clean) || 0, f: (r) => esc(r.clean) },
    { k: "speed", t: n.pro ? "10-yd split" : "40", v: (r) => -(parseFloat(r.speed) || 99), f: (r) => esc(r.speed) },
    { k: "shuttle", t: "Shuttle", v: (r) => -(parseFloat(r.shuttle) || 99), f: (r) => esc(r.shuttle) },
    { k: "vertical", t: "Vertical", v: (r) => parseFloat(r.vertical) || 0, f: (r) => esc(r.vertical) },
    { k: "weight", t: "Weight", div: true, f: (r) => `${r.weight} lb` },
    { k: "target", t: "Target", f: (r) => (r.target ? `<span class="muted">${r.target} lb</span>` : '<span class="muted">—</span>') },
    { k: "diff", t: "", l: true, v: (r) => (r.diff == null ? 0 : Math.abs(r.diff)), f: (r) => (r.diff == null ? "" : `<span class="${Math.abs(r.diff) >= 4 ? "amber" : "muted"}">${r.diff === 0 ? "On target" : `${r.diff > 0 ? "+" : ""}${Math.round(r.diff)} lb`}</span>`) },
  ];
  return table("trnums", cols, n.rows, { groupBy: (r) => r.pos });
}
// Who's where (TR7, TR8, TR9, TR14).
function twWhere(x) {
  const w = x.where;
  const pl = (p) => `${playerLink(p.id, p.name, "Development")} <span class="muted">${esc(p.pos)}</span>`;
  const box = (t, sub, rows, empty) => `<div class="panel tw-panel">${twPh(t, sub)}${rows.length ? `<table class="tw-small">${rows.map((r) => `<tr>${r.map((c, i) => `<td class="${i ? "muted" : ""}">${c}</td>`).join("")}</tr>`).join("")}</table>` : `<div class="tw-mini">${esc(empty)}</div>`}</div>`;
  // Many at other sports (a high school's spring): one row a sport, as a
  // coach would say it ("24 on the baseball team").
  const sports = {};
  for (const a of w.away) { const k = a.what.split(":")[0]; (sports[k] = sports[k] || []).push(a); }
  const many = w.away.length > 8;
  const names = (xs) => `${xs.slice(0, 5).map((a) => pl(a.p)).join(", ")}${xs.length > 5 ? ` <span class="muted">and ${xs.length - 5} more</span>` : ""}`;
  const away = (many
    ? Object.entries(sports).map(([k, xs]) => [`<b>${xs.length} ${esc({ basketball: "on the basketball team", baseball: "on the baseball team", track: "at track", wrestling: "wrestling" }[k] || `at ${k}`)}</b><div class="tw-l2">${names(xs)}</div>`, esc(xs[0].what.split(":").slice(1).join(":").trim() || xs[0].what)])
    : w.away.map((a) => [pl(a.p), esc(a.what)])
  ).concat(x.players.filter((r) => r.prep && (r.prep.status === "away" || r.prep.status === "absent") && !w.away.some((a) => a.p.id === r.id)).map((r) => [`${playerLink(r.id, r.name, "Development")} <span class="muted">${esc(r.pos)}</span>`, esc(r.prep.status_text)]));
  return `<div class="grid2 tw-g2">${box("Rehab group, with the trainers' step", `${w.rehab.length}`, w.rehab.map((r) => [pl(r.p), `${esc(r.what)}${r.since ? ` · since ${esc(r.since)}` : ""}`, esc(r.step), esc(r.back)]), "Nobody in rehab this week.")}
    ${box("New arrivals", "the last four weeks", w.arrivals.map((a) => [pl(a.p), esc(a.how), esc(a.at)]), "Nobody new in the last four weeks.")}
    ${box("At another sport, or away", w.away.length ? `${w.away.length} at other sports` : "", away, w.tier === "nfl" ? "Nobody away: NFL players play no other sport." : "Nobody away this week.")}
    ${box("Study hall", w.tier === "nfl" ? "" : `${w.flagged} flagged · ${esc(w.line)}`, w.study.map((s) => [pl(s.p), `<span class="${s.risk ? "amber" : ""}">${esc(s.why)}</span>`]), w.tier === "nfl" ? "No study hall at this level." : "Nobody flagged.")}</div>`;
}
function twHow() {
  return `${twPh("How prep plays out", "")}
    <p><b>This week.</b> Poor prep shows only through the battles: more busts, wrong gaps, late reads, timing off with his quarterback, up to about a third more mental errors. A poorly prepared player can still have a good day.</p>
    <p><b>Mental reps count.</b> A rested veteran who does the walkthrough and the film stays prepared. One who misses everything doesn't.</p>
    <p><b>Five weeks.</b> A run of poor weeks makes him rusty: up to about 3 points off his reading and technique ratings, won back over two or three good weeks.</p>
    <p><b>The season.</b> Sustained poor prep slows his growth and shows in practice grades and the depth chart battle.</p>
    <p><b>Neutral by default.</b> An average staff's normal week costs nothing. Good prep avoids the losses and sharpens the week. Every other team plays by the same rule.</p>`;
}
function twLeague(x) {
  return `${twPh("Across the league", "")}<ul class="tw-list tw-block">${x.league.map((l) => `<li><b>${esc(l.head)}</b><span class="muted">${l.id != null ? `${esc(l.line)} ${playerLink(l.id, "His page")}` : esc(l.line)}</span></li>`).join("")}</ul>`;
}
function weekTab(x) {
  const draft = trDraft(x);
  const own = x.own;
  const edit = x.head && !own;
  const s = twSums(x, draft.days);
  const chips = edit
    ? `<div class="tw-acts"><button class="st-btn small ${draft.dirty ? "primary" : "glass"}" data-act="tr-wsave" ${draft.dirty ? "" : "disabled"}>Save this week</button>${draft.dirty ? '<button class="st-btn small glass" data-act="tr-wundo">Undo changes</button>' : ""}<button class="st-btn small glass" data-act="tr-wtpl" title="Keep this week as the template for every ${esc(x.slot_label.toLowerCase())}">Keep as the template for this block</button>${x.yours ? '<button class="st-btn small glass" data-act="tr-wreset">Back to the staff\'s week</button>' : ""}</div>`
    : "";
  const note = own
    ? "The players are on their own: the rules keep your staff away. The strength staff's numbers and the facility log say who is doing the work."
    : !x.head ? "The head coach runs practice: you see his week, and you plan your own players on the Players tab."
    : x.yours ? (x.pushed ? "Your week, saved outside the rules: a staff member or a player could report it." : "Your week.") : "Your staff's week for this block, from their style. Change any period and the staff keeps the rest.";
  const fitted = x.fitted && x.fitted.length ? ` Your staff changed ${x.fitted.length === 1 ? "one thing" : x.fitted.length + " things"} to keep it inside the rules: ${x.fitted.join(" ")}` : "";
  const breaches = x.breaches.length ? `<div class="panel tr-breach"><h2>Outside the rules <small>${x.breaches.length}</small></h2><div class="items">${x.breaches.map((b) => `<div class="item"><span class="when">${esc(b.day || "Week")}</span><span>${esc(b.text)}</span></div>`).join("")}</div></div>` : "";
  const head = `<div class="tw-head"><div class="tw-hl"><div class="tw-h1">${esc(x.title)}</div><div class="tw-sub">${esc(x.sub)}</div>
    <div class="tw-rule"><a class="link" data-act="tr-wrule">The rules this week</a> ${esc(x.rule || "")}</div>
    <div class="tw-note">${esc(note + fitted)}</div>${chips}</div>${twYear(x)}</div>`;
  const cards = `<div class="tw-sum">${[twHoursCard(x, s), twContactCard(x, draft.days), twPrepCard(x), twEffortCard(x)].map((c) => `<div class="panel tw-panel">${c}</div>`).join("")}</div>`;
  const grid = `<div class="tw-sect"><h2>The week</h2><span class="muted">${x.cards.gp.game ? "Clock times, who runs each period, what's counted" : x.tier === "nfl" ? "Clock times; who runs each period, with your read of him as a teacher" : "Clock times around classes; who runs each period, with your read of him as a teacher"}</span></div>
    <div class="tw-scroll"><div class="tw-week">${draft.days.map((d, k) => twDay(x, d, k, x.days[k], edit)).join("")}</div></div>
    <div class="tw-legend"><span><b>Counted</b> = against the rule above</span><span><b>Not counted</b> = medical, academic, voluntary or the players' own</span><span>A dot after a name = full contact</span><span>Click any period, coach or player for the detail</span></div>`;
  const prep = x.effort.none
    ? `<div class="tw-sect"><h2>Each player's prep</h2><span class="muted">No read this week</span></div>
    <div class="panel tw-panel tw-noprep">The players are on their own: the rules keep your staff away, so there is no prep to read. Their five weeks pick up again when the staff may work with them, and the Players tab keeps each player's last weeks.</div>`
    : `<div class="tw-sect"><h2>Each player's prep</h2><span class="muted">${x.cards.gp.game ? "This week's plan: reps by role, installs, film, walkthroughs, his status and his own study." : "No game this week: the word reads his work on the system. In a game week the same scale reads the game plan."}</span></div>
    ${twPrepTable(x)}<div class="tw-mini">The word comes from what he really did this week, as your staff reads it. The five bars are the last five weeks, oldest first (a flat grey bar: no week on record, such as before he arrived or a week the staff couldn't work with him). Amber = underprepared or worse. The same word shows on the Players tab and in the game day report.</div>`;
  const panels = `<div class="grid3 tw-g3"><div class="tw-stack"><div class="panel tw-panel tw-explain">${twHow()}</div><div class="panel tw-panel">${twLeague(x)}</div></div>
    <div class="tw-stack"><div class="panel tw-panel">${twPh("Why your staff plans it this way", x.cards.gp.game ? "this opponent" : "this block")}<ul class="tw-why">${x.reasons.map((r) => `<li>${esc(r)}</li>`).join("") || "<li>No notes.</li>"}</ul></div><div class="panel tw-panel">${twFamilies(x, draft.days)}</div></div>
    <div class="tw-stack"><div class="panel tw-panel">${twGroups(x, draft.days)}</div><div class="panel tw-panel">${twBody(x)}</div></div></div>`;
  const n = x.numbers;
  const numbers = `<div class="tw-sect"><h2>The strength coach's numbers</h2><span class="muted">${n.date ? `${n.baseline ? "Your staff's baseline" : "Testing day"}, ${esc(n.date)}` : "No testing yet"}${n.next ? ` · next testing day ${esc(n.next)}` : ""} · from ${esc(x.staff.str ? x.staff.str.name : "the strength staff")}</span></div>${twNumbers(x)}`;
  const where = `<div class="tw-sect"><h2>Who's where</h2><span class="muted">Rehab, new arrivals, other sports, study hall</span></div>${twWhere(x)}`;
  return `<div class="tw">${head}${cards}${breaches}${grid}${prep}${panels}${numbers}${where}</div>${twSide(x, draft, edit)}`;
}
function playersTab(x) {
  const cols = [
    { k: "name", t: "Name", l: true, f: (r) => playerLink(r.id, r.name, "Development") },
    { k: "pos", t: "Pos", l: true },
    { k: "class", t: "Class", l: true, v: (r) => classRank(r.class) },
    { k: "role", t: "Role", l: true },
    { k: "ovr", t: "Rating", div: true, v: (r) => mid(r.ovr), f: (r) => range(r.ovr, null, { p: r.id, k: "ovr" }) },
    { k: "growth", t: "Growth", l: true, tip: "Your position coach's read of how he's coming along, in words", f: (r) => trGrowth(r.growth) },
    { k: "grade_words", t: "Practice", l: true, tip: "His practice grades against the room", v: (r) => r.grade },
    { k: "prep", t: "Prep", l: true, tip: "Your staff's word for his week's prep, and the last five weeks", v: (r) => (r.prep ? r.prep.word : 0), f: (r) => (r.prep ? `${twWord(r.prep.word)} ${twTrend(r.prep.five)}${r.prep.rust ? ` <small class="amber">rusty</small>` : ""}` : "") },
    { k: "practice", t: "Status", l: true, f: trStatus },
    { k: "load", t: "Load", l: true, div: true, f: trLoad, v: (r) => ["light", "moderate", "heavy", "very heavy"].indexOf(r.load) },
    { k: "load_trend", t: "Trend", l: true, hide: true },
    { k: "focus", t: "Main focus", l: true, div: true, v: (r) => r.plan.focus_label || "", f: (r) => `<select class="tr-sel" data-act="tr-focus" data-id="${r.id}" ${r.may_plan === false ? 'disabled title="The head coach plans him"' : ""}><option value="">${esc(r.plan.staff_focus ? "Staff: " + r.plan.staff_focus : "Balanced")}</option>${r.plan.options.map((o) => `<option value="${o.key}" ${o.key === r.plan.focus ? "selected" : ""}>${esc(o.label)}</option>`).join("")}</select>` },
    { k: "habit", t: "Habit", l: true, v: (r) => (r.plan.habit ? r.plan.habit.label : ""), f: (r) => (r.plan.habit ? esc(r.plan.habit.label) : '<span class="muted">-</span>') },
    { k: "body", t: "Body", l: true, v: (r) => (r.plan.body ? r.plan.body.target : 0), f: (r) => (r.plan.body ? `${r.wt} → ${r.plan.body.target} lb` : '<span class="muted">-</span>') },
    { k: "moving", t: "Learning", l: true, v: (r) => (r.plan.moving ? r.plan.moving.to : ""), f: (r) => (r.plan.moving ? `${esc(r.plan.moving.to)} <small class="muted">${r.plan.moving.progress}%</small>` : '<span class="muted">-</span>') },
    { k: "extra", t: "Extra", l: true, hide: true, v: (r) => Object.entries(r.plan.extra).filter(([, v]) => v).map(([k]) => k).join(", ") },
    { k: "rest", t: "Rest days", hide: true, v: (r) => r.plan.rest },
    { k: "sport", t: "Other sport", l: true, hide: x.tier !== "hs", v: (r) => r.sport || "" },
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
  const log = x.log.map((l) => `<div class="panel warm"><h2>Week of ${esc(l.monday)} <small>${esc(l.block || "")} · ${trPlural(l.practices, "practice")} · ${l.minutes} min on the field · ${l.snaps} team snaps${l.injuries ? ` · ${l.injuries} hurt` : ""}</small></h2>
    <div class="grid2"><div><div class="st-label">Best practice grades</div><div class="items">${l.best.map((b) => `<div class="item"><span class="when">${b.grade}</span><span>${rich(b.link)}</span></div>`).join("") || '<div class="empty">No grades.</div>'}</div></div>
    <div><div class="st-label">Battles</div><div class="items">${l.battles.map((b) => `<div class="item story"><span>${rich(b.a)} <b>${b.aw}-${b.bw}</b> ${rich(b.b)}</span></div>`).join("") || '<div class="empty">No battles this week.</div>'}</div></div></div></div>`).join("");
  const reports = x.reports.map((r) => `<div class="panel warm"><h2>${esc(r.block)} report <small>${esc(r.date)}</small></h2><div class="items">${r.lines.map(([a, b]) => `<div class="item"><span class="when">${esc(a)}</span><span>${rich(b)}</span></div>`).join("")}</div></div>`).join("");
  const hurt = x.hurt.length ? `<div class="panel warm"><h2>Hurt in practice and off the field <small>${x.hurt.length}</small></h2><div class="items">${x.hurt.map((h) => `<div class="item"><span class="when">${esc(h.from)}</span><span>${rich(h.link)}: ${esc(h.what)}${h.how && h.how !== h.what ? ` <small class="muted">(${esc(h.how)})</small>` : ""} · back about ${esc(h.until)}</span></div>`).join("")}</div></div>` : "";
  const cases = x.cases.length ? `<div class="panel tr-breach"><h2>Rule cases <small>practice outside the rules that came out</small></h2><div class="items">${x.cases.map((c) => `<div class="item"><span class="when">${esc(c.date)}</span><span><b>${esc(c.what)}</b> ${esc(c.how)}. ${esc(c.penalty)}</span></div>`).join("")}</div></div>` : "";
  const notes = `<div class="panel warm"><h2>Staff notes <small>the trainers, the strength staff and your position coaches</small></h2>${x.notes.length ? `<div class="items">${x.notes.map((n) => `<div class="item"><span class="when">${esc(n.date)}</span><span><b>${esc(n.dept)}</b> ${rich(n.text)}</span></div>`).join("")}</div>` : '<div class="empty">No notes yet. They come in after each week.</div>'}</div>`;
  return `${cases}${hurt}${notes}${reports}${log || '<div class="empty">No practice weeks logged yet this year.</div>'}`;
}
screens.training = async (a) => {
  const x = TR.reuse && TR.x ? TR.x : await api("practice");
  TR.reuse = false;
  TR.x = x;
  // A new visit to the page starts with the side panel shut.
  if (TW.route !== S.route) {
    TW.route = S.route;
    TW.side = null;
    if (TR.sel && TR.sel.tpl == null) TR.sel = null;
  }
  setColors(x.team.colors);
  const list = ["This week", "Players", "Templates", "The year", "Reports"];
  const tab = tabOf("trtab", list, a);
  title("Training", tab);
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
    const key = d.key.map((k) => `<div class="attr ${k.focus ? "tr-focus" : ""}"><span>${esc(k.label)} <small class="muted">${esc(k.family)}</small>${k.focus ? ' <span class="tag soft">Focus</span>' : ""}</span>${range(k.read, "your position coach", { p: p.id, k: k.key })}</div>`).join("");
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
      <div class="gd-form stack"><label>Run or pass<small>${lean === 0 ? "As the play caller likes" : lean > 0 ? `${trPlural(lean, "point")} more passing` : `${trPlural(-lean, "point")} more running`}</small></label>
      <input type="range" class="tr-range" min="-20" max="20" step="1" value="${lean}" data-act="tr-lean" ${x.head ? "" : "disabled"}>
      <label>Stick to the call sheet<small>${stick}% of snaps from the card when a play fits the situation; the rest your play caller's own read</small></label>
      <input type="range" class="tr-range" min="0" max="100" step="5" value="${stick}" data-act="tr-stick"></div></div>
    <div class="panel warm"><h2>${x.next ? "The next game" : "No game ahead"} <small>${esc(x.next_date || "")}</small></h2>
      ${x.next ? `<p class="bigline">${rich(typeof x.next === "string" ? x.next : x.next.text || "")}</p>` : '<p class="sub">The plan carries over to the next game on the schedule.</p>'}
      <p class="sub">The week's matchups (whose corner to throw at, bracketing, the spy) and your game rules are on the game day screen. The call sheet below is what your play caller has on his card.</p>
      <div class="st-actions"><button class="st-btn small" data-go="gameday" data-tab="Game plan">The week's game plan</button><button class="st-btn small glass" data-go="designer">Draw a play</button></div></div></div>
    ${planGlance(x)}`;
}
function planGlance(x) {
  // Where the plan stands: the card, what's going in, and this week's time for it.
  const sits = x.sheet.filter((s) => s.plays.length).length;
  const sharp = x.families.filter((f) => f.reps >= 26).length;
  const going = x.designs.filter((d) => d.installing).length;
  const p = x.pace;
  const tiles = `<div class="st-tiles tr-tiles">
    <div><b>${x.on_sheet}</b><small>${x.on_sheet === 1 ? "play" : "plays"} on the call sheet${x.on_sheet ? `, in ${sits} of ${x.sheet.length} situations` : ": your play caller calls from his own read"}</small></div>
    <div><b>${going}<i> / ${x.designs.length}</i></b><small>of your own plays going in</small></div>
    <div><b>${sharp}<i> / ${x.families.length}</i></b><small>offensive families past the base playbook</small></div>
    <div><b>${p.team_minutes}</b><small>team minutes this week</small></div>
    <div><b>${p.reps_each || 0}</b><small>${p.keys ? `reps this week for each of the ${p.keys} ${p.keys === 1 ? "thing" : "things"} on the card` : "reps a week for each new thing"}</small></div></div>`;
  const how = p.keys
    ? `At this week's pace a new play is known in about ${p.to_know} ${p.to_know === 1 ? "week" : "weeks"}. More team periods or fewer things on the card make it quicker; a play your players don't know yet stays off the field.`
    : "Put plays on the call sheet to give the team periods something to rep. Until then your players rep the base playbook.";
  return `<div class="panel warm"><h2>Where the plan stands <small>from this week's practice</small></h2>${tiles}<p class="sub">${esc(how)}</p>
    <div class="st-actions"><button class="st-btn small glass" data-act="tab" data-key="trplan" data-t="Call sheet">Call sheet</button><button class="st-btn small glass" data-act="tab" data-key="trplan" data-t="Installs">Installs</button><button class="st-btn small glass" data-go="training">This week's practice</button></div></div>`;
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
      <p class="sub">${trPlural(x.on_sheet, "play")} on the card. Plays from the playbook are known; reps on the card sharpen them. A play you drew stays off the card in games until your players know it about a third of the way.</p></div></div></div>`;
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
// The engine's y runs to the quarterback's left; drawn from behind the
// offense, his left is the screen's left.
const desX = (y) => ((DES_Y - bdSpread(y, 6, 2, 0.75)) / (2 * DES_Y)) * DES_W;
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
  const review = saved && saved.review.length ? `<div class="panel warm"><h2>${esc(x.coordinator || "Your coordinator")}'s review</h2><div class="items">${saved.review.map((r) => `<div class="item story"><span>${esc(r)}</span></div>`).join("")}</div>${saved.read && saved.read.snaps ? `<p class="sub">Called ${trPlural(saved.read.snaps, "time")}: ${saved.read.yards} yards a play, ${trPlural(saved.read.completions, "completion")}.</p>` : ""}</div>` : "";
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
  const ovr = { k: "ovr", t: "Rating", tip: "Your staff's read of his play now", v: (r) => mid(r.ovr), f: (r) => range(r.ovr, null, { p: r.id, k: "ovr" }) };
  const key = (n) => ({ k: "k" + n, t: `Key ${n + 1}`, tip: "The skills that matter most at his position, your position coach's read", v: (r) => mid(r["k" + n]), f: (r) => `<span class="tr-key"><small>${esc(r["k" + n + "_label"] || "")}</small>${range(r["k" + n], null, { p: r.id, k: r["k" + n + "_key"] })}</span>` });
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
  return `<div class="film"><div class="pad tr-pad">${pageHead({ kicker: `${esc(trCap(x.tier))} · ${x.rows.length} players`, title: `<span class="tr-tmark">${mark(x.team, 40)}</span>${esc(x.team.name)}`, sub: `Your staff reads them every day.${out ? ` ${trPlural(out, "player")} out of practice.` : ""}${hurt ? ` ${hurt} on load watch.` : ""}`, goto: gotoFor("roster", a) })}
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
// Down the board in spread units (the line at 0); each board shows only
// the depth its spots use, at the same scale across.
const bdT = (x) => -bdSpread(x, 8, 1.8, 0.9);
function bdFrame(slots) {
  const ts = [0, ...slots.map((s) => bdT(s.x))];
  const lo = Math.min(...ts) - 3.6, hi = Math.max(...ts) + 3.6;
  const span = Math.max(18, hi - lo);
  const top = (lo + hi) / 2 - span / 2;
  return { top, span, y: (x) => ((bdT(x) - top) / span) * 100 };
}
function slotCard(s, k, b, sel, fr) {
  const p = s.player;
  const prop = s.fill ? `<em title="Nobody is listed for this spot; the engine sends the best man left in the room">next man up</em>` : s.proposal && (!p || s.proposal.id !== p.id) ? `<em title="Your position coach would put ${esc(s.proposal.name)} here">Coach: ${esc(trLast(s.proposal.name))}</em>` : "";
  const drag = s.controls ? 'draggable="true"' : "";
  return `<div class="tr-slot ${sel ? "sel" : ""} ${p ? "" : "empty"} ${s.controls ? "" : "locked"} ${p && p.practice !== "full" ? "p-" + p.practice : ""}" style="left:${bdX(s.y)}%;top:${fr.y(s.x)}%" data-act="tr-slot" data-k="${k}" data-board="${esc(b.key)}" ${drag} ${p ? `data-pid="${p.id}"` : ""}>
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
  const fr = bdFrame(b.slots);
  const want = sel != null ? b.slots[sel] : null;
  const onBoard = new Set(b.slots.filter((s) => s.player).map((s) => s.player.id));
  // The pool: everyone, the slot's position first when a slot is picked.
  const pool = x.players.slice().sort((p, q) => (want ? (q.pos === want.pos) - (p.pos === want.pos) : 0) || mid(q.ovr) - mid(p.ovr));
  const poolHtml = pool.filter((p) => !want || p.pos === want.pos || keep("trpoolall", { v: false }).v).map((p) => `<div class="tr-pp ${onBoard.has(p.id) ? "on" : ""} ${p.practice !== "full" ? "p-" + p.practice : ""}" draggable="true" data-act="tr-pp" data-pid="${p.id}"><span class="pos">${esc(p.pos)}</span><b>${playerLink(p.id, p.name, "Development")}</b><small>${esc(p.class)} · ${esc(p.role || "")}${p.practice !== "full" ? ` · ${esc(p.practice)}` : ""}</small>${range(p.ovr, null, { p: p.id, k: "ovr" })}</div>`).join("");
  const props = b.slots.some((s) => s.proposal && (!s.player || s.proposal.id !== s.player.id));
  const locked = b.slots.some((s) => !s.controls);
  const changes = b.slots.map((s) => (s.proposal && s.player && s.proposal.id !== s.player.id ? `<div class="item"><span class="when">${esc(s.slot)}</span><span>${playerLink(s.proposal.id, s.proposal.name, "Development")} over ${playerLink(s.player.id, s.player.name, "Development")} <small class="muted">${esc(s.proposal.practice === "full" ? s.proposal.class : s.proposal.practice)}</small></span></div>` : "")).join("");
  return `<div class="film"><div class="pad tr-pad">${head}${tabsHtml}
    <div class="tr-boardwrap"><div class="panel warm tr-boardp"><h2>${esc(b.label)} <small>${b.yours ? "your order" : b.kind === "package" ? "your position coaches' picks: set it to make it yours" : "your staff's order"}${locked ? " · some spots are the head coach's" : ""}</small></h2>
      <div class="tr-board ${b.kind}" data-board="${esc(b.key)}" style="aspect-ratio:${(60 / fr.span).toFixed(3)};--yd:${((5 / fr.span) * 100).toFixed(2)}%"><div class="tr-los" style="top:${((-fr.top / fr.span) * 100).toFixed(2)}%"></div>${b.slots.map((s, k) => slotCard(s, k, b, sel === k, fr)).join("")}</div>
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
      trLocal();
      break;
    }
    case "tr-wper": {
      const same = TR.sel && TR.sel.tpl == null && TR.sel.d === Number(d.d) && TR.sel.i === Number(d.i);
      TR.sel = same ? null : { d: Number(d.d), i: Number(d.i), tpl: null };
      TW.side = null;
      trLocal();
      break;
    }
    case "tr-wclose": twClose(); break;
    case "tr-wcoach": TR.sel = null; TW.side = { kind: "coach", key: d.key, n: d.n != null ? Number(d.n) : null }; trLocal(); break;
    case "tr-wplayer": TR.sel = null; TW.side = { kind: "player", id: Number(d.id) }; trLocal(); break;
    case "tr-wrule": TR.sel = null; TW.side = { kind: "rule" }; trLocal(); break;
    case "tr-wprep": keep("twprep", { v: "two" }).v = d.v; trLocal(); break;
    case "tr-wextra": {
      const dv = await api("player_dev", { id: Number(d.id) });
      const ex = dv.plan.extra;
      const bits = (ex.film ? 1 : 0) | (ex.work ? 2 : 0) | (ex.lift ? 4 : 0);
      if (await trDo("practice_plan", { id: Number(d.id), extra: bits | 1 }, "An extra film session each week.")) trRedraw();
      break;
    }
    case "tr-pdone": TR.sel = null; trLocal(); break;
    case "tr-padd": {
      const { draft, tpl } = trDays();
      const day = draft.days[Number(d.d)];
      // A field period on a field day, film when the staff may only teach.
      const meta = !tpl && TR.x ? TR.x.days[Number(d.d)] : null;
      const field = !meta || !meta.allowed || meta.allowed.field;
      day.periods.push(field ? { kind: "individual", min: 20, gear: "helmets", contact: "air", tempo: "normal", who: "all", what: 0 } : { kind: "film", min: 20, gear: "none", contact: "air", tempo: "normal", who: "all", what: 0 });
      draft.dirty = true;
      TR.sel = { d: Number(d.d), i: day.periods.length - 1, tpl };
      TW.side = null;
      trLocal();
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
        // On this week's clock, earlier and later go by the day's order.
        const order = TR.sel.tpl == null && TR.x ? twLayout(TR.x, draft.days[TR.sel.d], TR.x.days[TR.sel.d]).filter((it) => it.i != null).map((it) => it.i) : ps.map((_, n) => n);
        const j = order[order.indexOf(i) + Number(d.dir)];
        if (j == null) break;
        [ps[i], ps[j]] = [ps[j], ps[i]];
        TR.sel.i = j;
      }
      draft.dirty = true;
      trLocal();
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
    case "tr-wundo": TR.week = null; TR.sel = null; trLocal(); break;
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
function twClose() {
  TR.sel = TR.sel && TR.sel.tpl != null ? TR.sel : null;
  TW.side = null;
  const a = document.querySelector(".tw-side");
  if (a) a.remove();
  document.querySelectorAll(".tw-per.on").forEach((b) => b.classList.remove("on"));
}
// Where the side panel starts: under the top bar (afterRender).
function twAfter() {
  const v = document.querySelector(".view");
  if (v && document.querySelector(".tw-side")) document.documentElement.style.setProperty("--tw-top", `${Math.round(v.getBoundingClientRect().top)}px`);
}
document.addEventListener("keydown", (ev) => {
  if (ev.key === "Escape" && document.querySelector(".tw-side")) twClose();
});
document.addEventListener("click", (ev) => {
  if (!document.querySelector(".tw-side") || !ev.target.closest) return;
  if (ev.target.closest(".tw-side, [data-act^='tr-w'], [data-act^='tr-p'], .popmenu, select, input, label")) return;
  twClose();
}, true);
document.addEventListener("change", async (ev) => {
  const el = ev.target.closest("[data-act^='tr-']");
  if (!el) return;
  const d = el.dataset;
  switch (d.act) {
    case "tr-pf": {
      const { draft } = trDays();
      if (!TR.sel || !draft) break;
      const p = draft.days[TR.sel.d].periods[TR.sel.i];
      if (d.f === "noise") p.noise = el.checked;
      else p[d.f] = d.f === "min" || d.f === "what" ? Number(el.value) : el.value;
      if (d.f === "kind") {
        const k = TR.x ? twKind(TR.x, el.value) : null;
        p.what = 0;
        p.field = k ? k.field : true;
        if (k && !k.field) { p.gear = "none"; p.contact = "air"; p.noise = false; }
      }
      draft.dirty = true;
      trLocal();
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
