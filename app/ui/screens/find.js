// Finding players as a real scouting search (update 2, lane 7), in Press
// Box: Find recruits for a college staff, Find prospects for an NFL club,
// Colleges calling for a high school coach. The game runs every search:
// filters on what your staff and the public know (never a true rating),
// joined by "and" and "or", sorted by any column, 50 a page, as a table, a
// chart or side by side. Every name, school and coach opens its page; a
// kid's offers and an insider's record open beside.
"use strict";

Object.assign(CRUMBS, {
  recruit_find: "Find recruits",
  find_prospects: "Find prospects",
  hsside: "Colleges calling",
  colleges_calling: "Colleges calling",
  find_offers: "Offers",
  find_insider: "Insider",
});
navAdd({ go: "find_prospects", label: "Find prospects", icon: "find", sec: "scout", seats: ["nfl"], after: "draftboard" });
navAdd({ go: "hsside", label: "Colleges calling" });
GOTO.recruit_find = () => [["Recruiting board", "recruit_board"], ["Recruiting map", "recruit_board", { tab: "map" }], ["Recruit board", "recruitboard"], ["Visits", "visits_desk"], ["Department", "scouting"], ["Calendar", "calendar"]];
GOTO.find_prospects = () => [["Draft board", "draftboard"], ["Pre-draft", "predraft"], ["Department", "scouting"], ["Calendar", "calendar"]];
GOTO.hsside = () => [["Squad", "roster"], ["Recruiting", "recruiting"], ["Schedule", "schedule"], ["Calendar", "calendar"]];

// ---------- small pieces ----------
const frN = (n) => (n == null ? "" : Number(n).toLocaleString("en-US"));
const frStars = (n, none = "Not ranked") => (n ? `<span class="fr-stars" title="${n} stars">${"★".repeat(n)}<i>${"★".repeat(Math.max(0, 5 - n))}</i></span>` : `<span class="fr-mute">${esc(none)}</span>`);
const frMute = (t) => `<span class="fr-mute">${esc(t)}</span>`;
const frWord = (t, tone) => `<span class="fr-word${tone ? " " + tone : ""}">${esc(t)}</span>`;
const frPlural = (n, one, many) => `${frN(n)} ${n === 1 ? one : many || one + "s"}`;
const frSeat = (seat) => keep("fr-" + seat, { cls: null, level: null, pos: "ALL", place: "", q: "", filters: null, sure: "any", unseen: false, sort: null, dir: 1, page: 0, view: "table", cols: "overview", picked: [], cat: false, edit: null, x: null, y: null });
function frRead(r, id) {
  return range(r || null, null, r ? { p: id, k: "ovr" } : null);
}
function frName(r) {
  return `<div class="fr-who">${playerLink(r.id, r.name)}${r.new ? ' <span class="fr-new" title="Ranked for the first time at the latest update">New</span>' : ""}</div>`;
}
function frTeam(t, sub) {
  if (!t) return frMute("—");
  return `<span class="fr-team">${logoIn(t)}${teamLink(t)}${sub ? `<small>${esc(sub)}</small>` : ""}</span>`;
}
// A row's words with its school named as a link (minor bug 16).
function frStands(r) {
  let text = esc(r.interest || "");
  if (r.leader && r.leader.name && text.includes(esc(r.leader.name))) text = text.replace(esc(r.leader.name), teamLink(r.leader));
  const dot = r.tone === "good" ? "you" : r.tone === "bad" || r.tone === "warn" ? "beh" : "";
  const race = r.offers || r.on_board ? ` ${linkTo("recruit_race", { id: r.id }, "The race", "link fr-race")}` : "";
  return `<span class="fr-stand" title="${esc(r.reason || "")}"><i class="fr-dot ${dot}"></i>${text}</span>${race}`;
}
function frOffers(r) {
  return r.offers ? `<a class="link fr-num" data-act="fr-offers" data-id="${r.id}" title="His offers">${r.offers}</a>` : '<span class="fr-mute fr-num">0</span>';
}
function frTest(t, k, dp) {
  if (!t || t[k] == null) return frMute("—");
  return `<span class="fr-num">${esc(t[k])}</span><span class="fr-ver${t.ver ? " v" : ""}" title="${t.ver ? esc(`Measured: ${t.how || "verified"}`) : "His own or his school's numbers: hand-timed, a little fast"}">${t.ver ? "V" : "self"}</span>`;
}
function frVer(v, verified) {
  return v == null ? frMute("—") : `<span class="fr-num">${esc(v)}</span>${verified ? '<span class="fr-ver v" title="Verified">V</span>' : ""}`;
}
function frMove(m) {
  if (m == null) return frMute("—");
  if (m === 0) return '<span class="fr-mv">Same</span>';
  return `<span class="fr-mv${m < 0 ? " dn" : ""}">${m > 0 ? "▲" : "▼"} ${Math.abs(m)}</span>`;
}
// The rank cell; with `move`, the move since the last update rides beside
// the national rank (the overview has no room for its own column).
function frRank(r, move) {
  if (r.nat == null) return frMute("Not ranked");
  const mv = move && r.move ? ` ${frMove(r.move)}` : "";
  return `<span class="fr-num"><b>${r.nat}</b></span>${mv}<div class="fr-l2">${r.posrank != null ? `${esc(r.pos)} ${r.posrank} · ${esc(r.state || "")} ${r.staterank}` : ""}</div>`;
}
function frComp(r) {
  if (r.camp_stars) return `${frStars(r.camp_stars)}<div class="fr-l2">Kicking camps</div>`;
  return r.comp ? `${frStars(r.stars)}<div class="fr-l2 fr-num">${esc(r.comp.replace(/^0/, ""))}</div>` : frStars(r.stars);
}
function frBoard(r) {
  return r.on_board ? fcLink("recruit_file", "On board", { id: r.id }) : `<button class="fr-btn" data-fc="add" data-id="${r.id}" title="Add him to your recruiting board">+ Board</button>`;
}
// Your read against the services' (or the desks'), short under its header;
// the full sentence on hover.
function frVs(v) {
  if (v === "unseen") return frMute("Not seen");
  if (!v) return frMute("—");
  const short = { "You're higher": "Higher", "You're lower": "Lower" }[v] || v;
  return `<span title="${esc(v)}">${frWord(short, v === "You're higher" ? "hi" : v === "You're lower" ? "lo" : "")}</span>`;
}

// ---------- the college's column views ----------
const FR_COLLEGE = {
  overview: { label: "Overview", cols: (x) => [
    { t: "Recruit", k: "name", f: (r) => `${frName(r)}<div class="fr-l2 fr-at">${r.school ? `${teamLink(r.school)} · ` : ""}${esc(r.town || "")}</div>` },
    { t: "Pos", f: (r) => `${esc(r.pos)}<div class="fr-l2">${esc(r.cls)}</div>` },
    { t: "Composite", k: "comp", f: frComp },
    { t: x.prev_since ? `Rank · since ${x.prev_since}` : "Rank", k: "nat", hw: true, f: (r) => frRank(r, true) },
    { t: "Ht · Wt", k: "ht", f: (r) => `<span class="fr-num">${esc(r.ht)}</span><div class="fr-l2 fr-num">${esc(r.wt)} lb</div>` },
    { t: "Your staff's read", k: "now", hw: true, f: (r) => frRead(r.read, r.id) },
    { t: "Could play", k: "proj", f: (r) => (r.proj ? `${esc(r.proj)}<div class="fr-l2">${esc(r.sure || "")}</div>` : frMute("Not seen")) },
    { t: "You vs the services", k: "vs", hw: true, f: (r) => frVs(r.vs) },
    { t: "Where it stands", f: frStands, wrap: true },
    { t: "Offers", k: "offers", r: true, f: frOffers },
    { t: "", f: frBoard },
  ] },
  services: { label: "Services", cols: (x) => [
    { t: "Recruit", k: "name", f: frName },
    { t: "Pos", f: (r) => esc(r.pos) },
    ...x.services.map((s) => ({ t: s.name, k: `svc:${s.k}`, hw: true, f: (r) => { const v = r.svc.find((q) => q.k === s.k); return v && v.stars ? `${frStars(v.stars)}<div class="fr-l2">No. ${v.rank}</div>` : frMute("Not ranked"); } })),
    { t: "Composite", k: "comp", f: frComp },
    { t: "Rank · nat, pos, state", k: "nat", hw: true, f: (r) => frRank(r) },
    { t: x.prev_since ? `Since ${x.prev_since}` : "Movement", k: "move", hw: true, f: (r) => frMove(r.move) },
    { t: "Insider pick", k: "insider_conf", wrap: true, f: (r) => (r.insider ? `${teamLink(r.insider.team)} <span class="fr-l2">${r.insider.conf}%</span><div class="fr-l2"><a class="link" data-act="fr-insider" data-desk="${r.insider.k}">${esc(r.insider.name)}</a>, ${esc(r.insider.desk)}${r.insider.pct != null ? ` · ${r.insider.pct}% right last year` : ""}</div>` : frMute("No pick yet")) },
    { t: "You vs the services", k: "vs", hw: true, f: (r) => frVs(r.vs) },
  ] },
  athletic: { label: "Body and testing", cols: () => [
    { t: "Recruit", k: "name", f: frName },
    { t: "Pos", f: (r) => esc(r.pos) },
    { t: "Ht", k: "ht", r: true, f: (r) => `<span class="fr-num">${esc(r.ht)}</span>` },
    { t: "Wt", k: "wt", r: true, f: (r) => (r.wt_v ? frVer(r.wt_v, true) : `<span class="fr-num">${esc(r.wt)}</span>`) },
    { t: "Arm", k: "arm", r: true, f: (r) => frVer(r.arm, !!r.arm) },
    { t: "Hand", k: "hand", r: true, f: (r) => frVer(r.hand, !!r.hand) },
    { t: "Wing", k: "wing", r: true, f: (r) => frVer(r.wing, !!r.wing) },
    { t: "40", k: "t40", r: true, f: (r) => frTest(r.tests, "t40") },
    { t: "Shuttle", k: "tsh", r: true, f: (r) => frTest(r.tests, "tsh") },
    { t: "Vertical", k: "tvj", r: true, f: (r) => frTest(r.tests, "tvj") },
    { t: "Frame", k: "room", f: (r) => (r.frame ? `${esc(r.frame.word)}<div class="fr-l2">${r.frame.room > 0 ? `about ${r.frame.room} lb to add` : "carries his weight"}</div>` : frMute("Not seen")) },
    { t: "Other sports", k: "sport", f: (r) => (r.sport ? esc(r.sport) : frMute("None")), wrap: true },
  ] },
  production: { label: "Production", cols: () => [
    { t: "Recruit", k: "name", f: frName },
    { t: "Pos", f: (r) => esc(r.pos) },
    { t: "Classification", k: "class", f: (r) => esc(r.classification || "—") },
    { t: "Schedule", k: "sos", r: true, f: (r) => (r.sos != null ? `<span class="fr-num">${r.sos.toFixed(1)}</span>` : frMute("—")) },
    { t: "Last season", f: (r) => (r.line ? `<span class="fr-num">${esc(r.line)}</span>` : frMute("No stats")), wrap: true },
    { t: "Your staff's read", k: "now", f: (r) => frRead(r.read, r.id) },
  ] },
  recruitment: { label: "Recruitment", cols: () => [
    { t: "Recruit", k: "name", f: frName },
    { t: "Pos", f: (r) => esc(r.pos) },
    { t: "Where it stands", f: frStands, wrap: true },
    { t: "Offers", k: "offers", r: true, f: frOffers },
    { t: "Top schools", k: "top_out", f: (r) => (r.top ? `<div class="fr-wrap">${r.top.schools.map((s) => teamLink(s)).join(", ")}</div><div class="fr-l2">${esc(r.top.date)}</div>` : frMute("Not announced")), wrap: true },
    { t: "Officials", k: "officials", r: true, f: (r) => `<span class="fr-num">${r.officials} of 5</span>` },
    { t: "Visits to you", k: "visits_you", r: true, f: (r) => `<span class="fr-num">${r.visits_you}</span>` },
    { t: "Your camp", k: "camp", f: (r) => esc(r.camp) },
    { t: "Followers", k: "followers", r: true, f: (r) => `<span class="fr-num">${frN(r.followers)}</span>` },
    { t: "NIL value", k: "nil", r: true, f: (r) => (r.nil != null ? `<span class="fr-num">${money(r.nil)}</span>` : frMute("—")) },
  ] },
  ties: { label: "Ties and your staff", cols: () => [
    { t: "Recruit", k: "name", f: frName },
    { t: "Pos", f: (r) => esc(r.pos) },
    { t: "From campus", k: "miles", r: true, f: (r) => `<span class="fr-num">${frN(r.miles)} mi</span>${r.instate ? ` ${frWord("In-state")}` : ""}` },
    { t: "Ties", k: "ties", f: (r) => (r.ties.length ? `<div class="fr-wrap">${esc(r.ties.join(" · "))}</div>` : frMute("None known")), wrap: true },
    { t: "Area recruiter", k: "area", f: (r) => (r.area ? coachLink(r.area.id, r.area.name) : frMute("Nobody yet")) },
    { t: "His bond with the kid", f: (r) => esc(r.bond) },
    { t: "Your camp", k: "camp", f: (r) => esc(r.camp) },
  ] },
  academics: { label: "Academics and character", cols: () => [
    { t: "Recruit", k: "name", f: frName },
    { t: "Pos", f: (r) => esc(r.pos) },
    { t: "GPA", k: "gpa", r: true, f: (r) => `<span class="fr-num">${esc(r.gpa)}</span><div class="fr-l2">${r.transcript ? "transcript" : "his profile"}</div>` },
    { t: "Test", k: "test", r: true, f: (r) => (r.test ? `<span class="fr-num">${r.test}</span>` : frMute("Not taken")) },
    { t: "Academic risk", k: "risk", f: (r) => (!r.risk ? frMute("Not seen") : r.risk === "None" ? "None" : `<span class="fr-amber">${esc(r.risk)}</span>`) },
    { t: "Core courses", k: "core", f: (r) => (r.core.startsWith("Behind") ? `<span class="fr-amber">${esc(r.core)}</span>` : esc(r.core)) },
    { t: "Work ethic", k: "chr:0", f: (r) => (r.work ? esc(r.work) : frMute("Not seen")) },
    { t: "Scouted habit", f: (r) => (r.habit ? esc(r.habit) : frMute("Not seen")), wrap: true },
    { t: "Age for his class", k: "age_class", r: true, f: (r) => (r.age_class != null ? `<span class="fr-num">${r.age_class.toFixed(1)}</span>${r.young ? ` ${frWord("Young")}` : ""}${r.reclass ? ` ${frWord("Reclassified")}` : ""}` : frMute("—")) },
    { t: "Injuries", k: "missed", f: (r) => (r.injuries === "None" ? "None" : `<span class="fr-amber">${esc(r.injuries)}</span>`) },
  ] },
};

// ---------- the NFL club's column views ----------
const frNfl = {
  name: (r) => `<div class="fr-who">${playerLink(r.id, r.name)}${r.early ? ' <span class="fr-new" title="An early entrant">Early</span>' : ""}</div>`,
  school: (r) => `${frTeam(r.school)}<div class="fr-l2">${esc(r.conf || "")}</div>`,
  vs: (r) => frVs(r.vs),
  grade: (r) => (r.grade ? `${esc(r.grade)}<div class="fr-l2">${esc(r.proj || "")}${r.sure ? ` · ${esc(r.sure)}` : ""}</div>` : frMute("Not graded")),
};
const FR_NFL = {
  overview: { label: "Overview", cols: () => [
    { t: "Prospect", k: "name", f: frNfl.name },
    { t: "Pos", f: (r) => esc(r.pos) },
    { t: "School", f: frNfl.school },
    { t: "Class", f: (r) => esc(r.cls) },
    { t: "Draft age", k: "age_draft", r: true, f: (r) => (r.age ? `<span class="fr-num">${r.age}</span>` : frMute("—")) },
    { t: "Your board", k: "board", r: true, f: (r) => (r.board ? `<b class="fr-num">${r.board}</b>` : frMute("—")) },
    { t: "Your scouts' grade", k: "grade", f: frNfl.grade },
    { t: "Read", k: "now", f: (r) => frRead(r.read, r.id) },
    { t: "Consensus", k: "cons", r: true, f: (r) => (r.cons ? `<span class="fr-num">${r.cons}</span>` : frMute("—")) },
    { t: "Mock drafts", k: "mock_hi", f: (r) => (r.mock ? esc(r.mock) : frMute("Not mocked")) },
    { t: "You vs the desks", k: "vsd", f: frNfl.vs },
  ] },
  desks: { label: "Public desks", cols: (x) => [
    { t: "Prospect", k: "name", f: frNfl.name },
    { t: "Pos", f: (r) => esc(r.pos) },
    { t: "Your board", k: "board", r: true, f: (r) => (r.board ? `<b class="fr-num">${r.board}</b>` : frMute("—")) },
    ...x.desks.map((d) => ({ t: d.name, k: `desk:${d.k}`, r: true, f: (r) => { const v = r.desks.find((q) => q.k === d.k); return v && v.rank ? `<span class="fr-num">${v.rank}</span>` : frMute("Not ranked"); } })),
    { t: "Consensus", k: "cons", r: true, f: (r) => (r.cons ? `<b class="fr-num">${r.cons}</b>` : frMute("—")) },
    { t: "Mock drafts", k: "mock_hi", f: (r) => (r.mock ? esc(r.mock) : frMute("Not mocked")) },
  ] },
  body: { label: "Body and testing", cols: (x) => [
    { t: "Prospect", k: "name", f: frNfl.name },
    { t: "Pos", f: (r) => esc(r.pos) },
    { t: "Ht", k: "ht", r: true, f: (r) => (r.ht_v ? frVer(r.ht_v, true) : `<span class="fr-num">${esc(r.ht)}</span>`) },
    { t: "Wt", k: "wt", r: true, f: (r) => (r.wt_v ? frVer(r.wt_v, true) : `<span class="fr-num">${esc(r.wt)}</span>`) },
    { t: "Arm", k: "arm", r: true, f: (r) => frVer(r.arm, !!r.arm) },
    { t: "Hand", k: "hand", r: true, f: (r) => frVer(r.hand, !!r.hand) },
    { t: "40", k: "t40", r: true, f: (r) => frTest(r.tests, "t40") },
    { t: "Vertical", k: "tvj", r: true, f: (r) => frTest(r.tests, "tvj") },
    { t: "Bench", k: "tbp", r: true, f: (r) => frTest(r.tests, "tbp") },
    { t: "Combine", k: "combine", f: (r) => (r.combine ? `Invited<div class="fr-l2">${esc(x.combine.join("-"))}</div>` : frMute("Not invited")) },
    { t: "All-star week", k: "allstar", f: (r) => (r.all_star ? esc(r.all_star) : frMute("—")) },
  ] },
  production: { label: "Production", cols: () => [
    { t: "Prospect", k: "name", f: frNfl.name },
    { t: "Pos", f: (r) => esc(r.pos) },
    { t: "Conference", k: "conf", f: (r) => esc(r.conf || "—") },
    { t: "Starts", k: "starts", f: (r) => esc(r.starts) },
    { t: "Last season", f: (r) => (r.line ? `<span class="fr-num">${esc(r.line)}</span>` : frMute("No stats")), wrap: true },
    { t: "Scouted habit", f: (r) => (r.habit ? esc(r.habit) : frMute("Not seen")), wrap: true },
    { t: "Fit", k: "fit", f: (r) => (r.fit ? esc(r.fit) : frMute("Not seen")), wrap: true },
    { t: "Injuries", k: "missed", f: (r) => (r.injuries === "None" ? "None" : `<span class="fr-amber">${esc(r.injuries)}</span>`) },
  ] },
  process: { label: "Medical, character, visits", cols: () => [
    { t: "Prospect", k: "name", f: frNfl.name },
    { t: "Pos", f: (r) => esc(r.pos) },
    { t: "Medical read", k: "medical", f: (r) => (r.medical === "Cleared" || r.medical === "Not examined" ? esc(r.medical) : `<span class="fr-amber">${esc(r.medical)}</span>`) },
    { t: "Character read", k: "chr:0", f: (r) => (r.trouble ? `<span class="fr-amber">${esc(r.character || "Not read")} · trouble on record</span>` : r.character ? esc(r.character) : frMute("Not read")) },
    { t: "Top-30 visit", k: "t30", f: (r) => (r.top30 ? "On your list" : frMute("—")) },
    { t: "Interview", k: "interview", f: (r) => (r.interview ? "On your list" : frMute("—")) },
    { t: "All-star week", k: "allstar", f: (r) => (r.all_star ? esc(r.all_star) : frMute("—")) },
    { t: "Combine", k: "combine", f: (r) => (r.combine ? "Invited" : frMute("Not invited")) },
  ] },
};

// ---------- the search builder ----------
const FR_OPS = { ge: "at least", le: "at most", between: "between", is: "is" };
function frField(x, k) {
  for (const g of x.catalog) for (const f of g.fields) if (f.k === k) return f;
  return null;
}
function frChips(x, st) {
  const list = x.filters;
  if (!list.length) return `<span class="fr-mute">No filters: everyone your staff knows</span>`;
  let html = "";
  list.forEach((c, i) => {
    const nextOr = list[i + 1] && list[i + 1].or;
    if (i > 0) html += `<button class="fr-andor" data-act="fr-andor" data-i="${i}" title="Switch between and / or">${c.or ? "OR" : "AND"}</button>`;
    if (!c.or && nextOr) html += '<span class="fr-paren">(</span>';
    const fld = frField(x, c.k);
    const staff = fld && fld.staff ? ' <span class="fr-tag" title="Reads your staff\'s files">staff</span>' : "";
    html += `<span class="fr-chip"><span><span class="fr-mute">${esc(c.label)}</span> <b>${esc(c.words.slice(c.label.length + 1))}</b>${staff}</span><button class="fr-x" data-act="fr-rm" data-i="${i}" aria-label="Remove ${esc(c.label)}">×</button></span>`;
    if (c.or && !nextOr) html += '<span class="fr-paren">)</span>';
  });
  return html;
}
function frEditor(x, st) {
  const e = st.edit;
  if (!e) return "";
  const f = frField(x, e.k);
  if (!f) return "";
  const ops = f.kind === "text" ? ["is"] : f.kind === "flag" ? ["is"] : f.kind === "word" ? ["is", "ge", "le"] : ["ge", "le", "between", "is"];
  const op = ops.includes(e.op) ? e.op : ops[0];
  const val = (id, v) => {
    if (f.kind === "word") return `<select class="fr-sel" id="${id}">${f.opts.map((o, i) => `<option value="${i}"${String(v) === String(i) ? " selected" : ""}>${esc(o)}</option>`).join("")}</select>`;
    if (f.kind === "flag") return `<select class="fr-sel" id="${id}"><option value="yes">yes</option><option value="no"${v === "no" ? " selected" : ""}>no</option></select>`;
    return `<input class="fr-in" id="${id}" value="${esc(v ?? "")}" placeholder="${f.kind === "text" ? "a name or code" : "a number"}" ${f.kind === "num" ? 'inputmode="decimal"' : ""}>`;
  };
  const hint = f.kind === "word" ? `Worst to best: ${f.opts.join(", ")}.` : f.lower ? "Lower is better." : f.kind === "text" ? "A whole name finds only that name (Michigan, not Michigan State); part of one finds names whose words start with it." : "";
  return `<div class="fr-edit"><span class="fr-k">${esc(f.label)}</span>
    <select class="fr-sel" id="fr-op" data-act="fr-op">${ops.map((o) => `<option value="${o}"${o === op ? " selected" : ""}>${FR_OPS[o]}</option>`).join("")}</select>
    ${val("fr-v", e.v)}${op === "between" ? ` <span class="fr-mute">and</span> ${val("fr-v2", e.v2)}` : ""}
    <div class="fr-seg" role="group" aria-label="Join"><button data-act="fr-join" data-v="and" aria-pressed="${!e.or}">and</button><button data-act="fr-join" data-v="or" aria-pressed="${!!e.or}">or</button></div>
    <button class="fr-btn fr-main" data-act="fr-addgo">Add filter</button><button class="fr-btn" data-act="fr-editx">Cancel</button>
    ${hint ? `<span class="fr-mute fr-small">${esc(hint)}</span>` : ""}${f.staff ? '<span class="fr-mute fr-small">Reads your staff\'s files: "Not seen" applies.</span>' : ""}</div>`;
}
function frCatalog(x, st) {
  if (!st.cat) return "";
  return `<div class="fr-cat">${x.catalog.map((g) => `<div class="fr-catg"><h4>${esc(g.name)}</h4><ul>${g.fields.map((f) => `<li><button data-act="fr-pick" data-k="${esc(f.k)}" title="${esc(f.label)}">${esc(f.label)}</button></li>`).join("")}</ul></div>`).join("")}</div>`;
}
function frSeg(act, list, cur, label) {
  return `<div class="fr-seg" role="group" aria-label="${esc(label)}">${list.map(([v, t]) => `<button data-act="${act}" data-v="${esc(v)}" aria-pressed="${String(v) === String(cur)}">${esc(t)}</button>`).join("")}</div>`;
}
function frBuilder(x, st, seat) {
  const POS = ["QB", "RB", "WR", "TE", "OT", "IOL", "EDGE", "DT", "LB", "CB", "S", "K", "P", "LS"];
  const classes = x.classes.map((c) => [c, seat === "nfl" ? `${c} draft` : `Class of ${c}`]);
  const levels = seat === "nfl" ? [["all", "Every level"], ["fbs", "FBS"], ["fcs", "FCS"], ["small", "Division II and below"]] : [["hs", "High school"], ["juco", "Junior college"], ["portal", "Portal"]];
  const pos = `<select class="fr-sel" data-act="fr-f" data-f="pos" aria-label="Position"><option value="ALL">Every position</option>${POS.map((p) => `<option${st.pos === p ? " selected" : ""}>${p}</option>`).join("")}</select>`;
  const place = seat === "nfl"
    ? `<select class="fr-sel" data-act="fr-f" data-f="place" aria-label="Conference"><option value="">Every conference</option>${x.confs.map((c) => `<option value="${esc(c.k)}"${st.place === c.k ? " selected" : ""}>${esc(c.name)}</option>`).join("")}</select>`
    : `<select class="fr-sel" data-act="fr-f" data-f="place" aria-label="State"><option value="">Every state</option>${x.states.map((s) => `<option value="${s}"${st.place === s ? " selected" : ""}>${esc(STATE_NAMES[s] || s)}${s === x.home ? " (yours)" : ""}</option>`).join("")}</select>`;
  const saved = x.saved.map((s) => `<span class="fr-saved"><button class="fr-chipbtn" data-act="fr-open" data-k="${s.k}" title="Saved ${esc(s.since)}">${esc(s.name)}${s.new ? `<small>${s.new} new</small>` : ""}</button><button class="fr-x" data-act="fr-forget" data-k="${s.k}" aria-label="Forget ${esc(s.name)}">×</button></span>`).join("");
  return `<div class="fr-panel fr-builder">
    <div class="fr-row">${seat === "nfl" || st.level === "hs" || !st.level ? frSeg("fr-cls", classes, x.class, "Class") : ""}${frSeg("fr-level", levels, x.level, "Level")}${pos}${place}
      <input class="fr-in fr-q" data-act="fr-q" placeholder="Name or school" aria-label="Name or school" value="${esc(st.q)}"></div>
    <div class="fr-row"><span class="fr-k">Filters</span><div class="fr-chips">${frChips(x, st)}</div>
      <button class="fr-chipbtn${st.cat ? " on" : ""}" data-act="fr-cat">+ Filter</button>${!x.default ? '<button class="fr-chipbtn" data-act="fr-reset" title="Back to the default search">Default search</button>' : ""}</div>
    ${frEditor(x, st)}${frCatalog(x, st)}
    <div class="fr-row fr-split"><div class="fr-row"><span class="fr-k">How sure</span>${frSeg("fr-sure", [["any", "Any read"], ["fair", "Fair or narrow"], ["narrow", "Narrow only"]], x.sure, "How sure")}
      <span class="fr-k">Not seen</span>${frSeg("fr-unseen", [["1", "Include"], ["0", "Leave out"]], x.unseen_on ? "1" : "0", "Not seen")}</div>
      <div class="fr-row"><span class="fr-k">Saved searches</span>${saved}<button class="fr-chipbtn" data-act="fr-save">Save this search</button></div></div>
    <div class="fr-mini">${esc(x.filter_words ? `The search as the game reads it: ${x.filter_words}.` : "No filters.")} A saved search tells you in the inbox when a new ${seat === "nfl" ? "prospect" : "kid"} matches: ${seat === "nfl" ? "after rivalry week, the conference title games, the title game, the Senior Bowl, the combine and the last pro days" : "after a services update, your camp, or the high school all-star games"}.</div>
  </div>`;
}

// ---------- the results ----------
function frTable(x, st, views) {
  const v = views[st.cols] || views.overview;
  const cols = v.cols(x);
  const head = cols.map((c) => {
    const on = c.k && x.sort === c.k;
    const arrow = on ? (x.dir === -1 ? " ▼" : " ▲") : "";
    const cl = `${c.r ? "r" : ""}${c.hw ? " hw" : ""}`;
    return c.k ? `<th class="${cl}${on ? " on" : ""}"><button class="fr-th" data-act="fr-sort" data-k="${esc(c.k)}" title="Sort by ${esc(c.t)}">${esc(c.t)}${arrow}</button></th>` : `<th class="${cl}">${esc(c.t)}</th>`;
  }).join("");
  const picked = new Set(st.picked);
  const body = x.rows.map((r) => `<tr class="${picked.has(r.id) ? "on" : ""}"><td class="fr-tick"><input type="checkbox" data-act="fr-tick" data-id="${r.id}" ${picked.has(r.id) ? "checked" : ""} aria-label="Pick ${esc(r.name)} to compare"></td>${cols.map((c) => `<td class="${c.r ? "r" : ""}${c.wrap ? " wrap" : ""}">${c.f(r)}</td>`).join("")}</tr>`).join("");
  const from = x.page * 50 + 1, to = x.page * 50 + x.rows.length;
  const pager = `<div class="fr-more"><span>${x.rows.length ? `${frN(from)}-${frN(to)} of ${esc(x.count_text)}` : "Nobody matches"} · 50 a page</span><span>${x.page > 0 ? '<button class="fr-chipbtn" data-act="fr-page" data-d="-1">Previous 50</button>' : ""}${x.page + 1 < x.pages ? `<button class="fr-chipbtn" data-act="fr-page" data-d="1">Next 50</button>` : ""}</span></div>`;
  const empty = `<tr><td colspan="${cols.length + 1}"><div class="fr-empty">Nobody matches. ${x.unseen && !x.unseen_on ? `${frPlural(x.unseen, "kid")} your staff hasn't seen would: switch "Not seen" to Include, or ` : ""}take a filter off.</div></td></tr>`;
  return `<div class="fr-tbl"><table><thead><tr><th class="fr-tick"></th>${head}</tr></thead><tbody>${body || empty}</tbody></table>${pager}</div>`;
}
function frChart(x, st) {
  const c = x.chart;
  const nums = [];
  for (const g of x.catalog) for (const f of g.fields) if (f.kind === "num") nums.push(f);
  const sel = (act, cur) => `<select class="fr-sel" data-act="${act}">${nums.map((f) => `<option value="${esc(f.k)}"${f.k === cur ? " selected" : ""}>${esc(f.label)}</option>`).join("")}</select>`;
  const pts = c.points;
  const W = 900, H = 420, m = { l: 58, r: 20, t: 16, b: 44 };
  const ext = (a) => { let lo = Math.min(...a), hi = Math.max(...a); if (lo === hi) { lo -= 1; hi += 1; } const pad = (hi - lo) * 0.08; return [lo - pad, hi + pad]; };
  const [x0, x1] = pts.length ? ext(pts.map((p) => p.x)) : [0, 1];
  const [y0, y1] = pts.length ? ext(pts.map((p) => p.y)) : [0, 1];
  // A field where lower is better runs best to the right and to the top.
  const X = (v) => { const t = (v - x0) / (x1 - x0); return m.l + (c.x.lower ? 1 - t : t) * (W - m.l - m.r); };
  const Y = (v) => { const t = (v - y0) / (y1 - y0); return H - m.b - (c.y.lower ? 1 - t : t) * (H - m.t - m.b); };
  const ticks = (a, b) => [0, 1, 2, 3, 4, 5].map((i) => a + ((b - a) * i) / 5);
  const fm = (v) => (Math.abs(v) >= 100 ? Math.round(v).toLocaleString("en-US") : Math.abs(v) >= 10 ? v.toFixed(1) : v.toFixed(2));
  const dots = pts.map((p) => `<g class="fr-pt${p.picked ? " on" : ""}"><a data-go="player" data-id="${p.id}"><circle cx="${X(p.x).toFixed(1)}" cy="${Y(p.y).toFixed(1)}" r="${p.picked ? 6 : 4.5}"><title>${esc(p.name)} · ${esc(p.pos)} · ${esc(p.xs)}, ${esc(p.ys)}</title></circle></a>${p.picked ? `<text x="${(X(p.x) + 9).toFixed(1)}" y="${(Y(p.y) + 4).toFixed(1)}">${esc(p.name)}</text>` : ""}</g>`).join("");
  return `<div class="fr-panel fr-chart"><div class="fr-row"><span class="fr-k">Across</span>${sel("fr-x", c.x.k)}<span class="fr-k">Up</span>${sel("fr-y", c.y.k)}
    <span class="fr-mute fr-push">${frN(pts.length)} of the first ${frN(c.of)} by your sort${c.missing ? ` · ${frN(c.missing)} without both numbers` : ""}. Each dot opens the player; ticked players are gold and named.</span></div>
    <svg viewBox="0 0 ${W} ${H}" role="img" aria-label="${esc(c.x.label)} against ${esc(c.y.label)}">
      ${ticks(x0, x1).map((t) => `<line class="g" x1="${X(t)}" x2="${X(t)}" y1="${m.t}" y2="${H - m.b}"/><text class="t" x="${X(t)}" y="${H - m.b + 16}" text-anchor="middle">${fm(t)}</text>`).join("")}
      ${ticks(y0, y1).map((t) => `<line class="g" x1="${m.l}" x2="${W - m.r}" y1="${Y(t)}" y2="${Y(t)}"/><text class="t" x="${m.l - 8}" y="${Y(t) + 4}" text-anchor="end">${fm(t)}</text>`).join("")}
      <text class="a" x="${(W + m.l) / 2}" y="${H - 8}" text-anchor="middle">${esc(c.x.label)}${c.x.lower ? " (better to the right)" : ""}</text>
      <text class="a" x="14" y="${(H - m.b + m.t) / 2}" text-anchor="middle" transform="rotate(-90 14 ${(H - m.b + m.t) / 2})">${esc(c.y.label)}</text>
      ${dots}
    </svg>${pts.length ? "" : '<div class="fr-empty">No player in the results has both numbers.</div>'}</div>`;
}
function frCompare(x) {
  const c = x.compare;
  if (!c.players || c.players.length < 2) return `<div class="fr-panel fr-empty">Tick two to five players in the table to compare them side by side.</div>`;
  return `<div class="fr-cmp">${c.players.map((p, i) => `<div class="fr-panel"><h3>${playerLink(p.id, p.name)}</h3><div class="fr-l2">${esc(p.pos)} · ${p.team ? teamLink(p.team) : ""} · ${esc(p.cls || "")}</div>
    ${c.rows.map((r) => `<div class="fr-cmprow"><span>${esc(r.label)}</span><span class="${r.best === i ? "best" : ""}${r.unseen[i] ? " fr-mute" : ""}">${esc(r.vals[i])}</span></div>`).join("")}
    <button class="fr-btn" data-act="fr-tick" data-id="${p.id}">Take out</button></div>`).join("")}</div>
    <div class="fr-mini">The best value on each line is gold. Your staff's reads show "Not seen" where nobody has looked.</div>`;
}
function frResults(x, st, views) {
  const n = st.picked.length;
  const bar = `<div class="fr-rbar">${frSeg("fr-view", [["table", "Table"], ["chart", "Chart"], ["compare", `Compare${n ? " " + n : ""}`]], st.view, "View")}
    ${st.view === "table" ? frSeg("fr-cols", Object.entries(views).map(([k, v]) => [k, v.label]), st.cols, "Columns") : ""}
    <span class="fr-push"></span><span class="fr-sortnote">Sorted by <b>${esc(x.sort_label)}</b>, ${x.dir === -1 ? "worst first" : "best first"} <button class="fr-chipbtn" data-act="fr-flip">Flip</button></span>
    <button class="fr-chipbtn" data-act="fr-csv" title="This page as a CSV file">CSV</button></div>`;
  const body = st.view === "chart" ? frChart(x, st) : st.view === "compare" ? frCompare(x) : frTable(x, st, views);
  return bar + body;
}
function frHead(x, seat, kicker, title, sub, dist, cells, note) {
  const max = Math.max(1, ...dist.map((d) => d[1]));
  const what = seat === "nfl" ? "prospects match" : "recruits match";
  const knows = seat === "nfl" ? "your scouts know" : "your staff knows";
  return `<div class="fr-head"><div><div class="fr-k">${kicker}</div><h1 class="fr-h1">${esc(title)}</h1><div class="fr-sub">${sub}</div>
      <div class="fr-count"><b class="fr-num">${esc(x.count_text)}</b><span>${x.count === 1 ? what.replace("match", "matches").replace("prospects", "prospect").replace("recruits", "recruit") : what} · of ${esc(x.known_text)} ${knows}</span></div>
      <div class="fr-rule">Every filter reads what your ${seat === "nfl" ? "scouts" : "staff"} and the public know, never a true rating. ${seat === "nfl" ? "A prospect" : "A kid"} your ${seat === "nfl" ? "scouts haven't" : "staff hasn't"} seen reads "Not seen": include those or leave them out.</div></div>
    <div class="fr-panel"><div class="fr-ph"><span class="fr-k">${seat === "nfl" ? "By your board" : "By the composite"}</span><span class="fr-mute">of the ${esc(x.count_text)} ${x.count === 1 ? "match" : "matches"}</span></div>
      <div class="fr-dist">${dist.map(([k, n]) => `<div><span class="fr-k">${esc(k)}</span><b class="fr-num">${frN(n)}</b><div class="fr-bar"><i style="width:${Math.max(3, (n / max) * 100)}%"></i></div></div>`).join("")}</div>
      <div class="fr-cells">${cells.map(([k, v]) => `<div><span class="fr-k">${esc(k)}</span><span class="fr-v">${v}</span></div>`).join("")}</div>
      <div class="fr-mini">${note}</div></div></div>`;
}

// The args a search sends.
function frArgs(st, extra) {
  const a = { class: st.cls, level: st.level, pos: st.pos, q: st.q, sure: st.sure, unseen: st.unseen, page: st.page, compare: st.picked, picked: st.picked };
  if (st.filters) a.filters = st.filters;
  if (st.sort) a.sort = st.sort;
  if (st.dir === -1) a.dir = -1;
  if (st.x) a.x = st.x;
  if (st.y) a.y = st.y;
  return Object.assign(a, extra || {});
}
// The search's filters as the screen keeps them (from the game's reply).
const frKeepFilters = (x) => x.filters.map((c) => ({ k: c.k, op: c.op, v: c.v, v2: c.v2, or: c.or }));

screens.recruit_find = async (a) => {
  const st = frSeat("college");
  if (a && a.state) { st.place = a.state; st.page = 0; a.state = null; }
  const args = frArgs(st, { state: st.place || null });
  if (st.openSaved != null) { args.saved = st.openSaved; st.openSaved = null; }
  const x = await api("find_recruits", args);
  FR.last = { seat: "college", x };
  if (x.team && x.team.colors) setColors(x.team.colors);
  st.cls = x.class; st.level = x.level; st.sort = x.sort;
  const sub = `${esc(x.team.name)} · class of ${x.class} · ${esc(x.class_words)} · ${esc(x.update)}`;
  const cells = [
    ["Not seen yet", x.unseen ? `${esc(x.unseen_text)} <small>${x.unseen_on ? "included" : "left out"}</small>` : "None <small>left out</small>"],
    [x.since ? `New since ${x.prev_since || x.since}` : "New", `${frN(x.sums.new)} <small>ranked for the first time</small>`],
    ["On your board", `${frN(x.sums.board)} <small>of these</small>`],
    ["Committed elsewhere", `${frN(x.sums.elsewhere)} <small>still shown</small>`],
  ];
  const note = `The services rank a class from its sophomore year. The younger classes move each winter, spring, summer and fall (next: ${esc(x.next_update)}, the ${esc(x.next_update_word)} update); the signing class moves with each commitment wave. Specialists carry the kicking camps' stars.`;
  const head = frHead(x, "college", `${teamLink(x.team)} · Recruiting`, "Find recruits", sub, x.dist, cells, note);
  const foot = `<div class="fr-foot"><div class="fr-panel"><div class="fr-ph"><span class="fr-k">The services</span></div><ul class="fr-list">${x.services.map((s) => `<li><span>${esc(s.name)}</span><span>stars by service, in the Services view</span></li>`).join("")}${x.services.length > 1 ? "<li><span>Composite</span><span>their average, from 2012</span></li>" : ""}</ul></div>
    <div class="fr-panel"><div class="fr-ph"><span class="fr-k">The insiders</span></div>${x.insiders.length ? `<ul class="fr-list">${x.insiders.map((d) => `<li><span><a class="link" data-act="fr-insider" data-desk="${d.k}">${esc(d.name)}</a> <small class="fr-mute">${esc(d.desk)}</small></span><span>${d.pct != null ? `${d.pct}% right last year` : "no record yet"} · ${d.open} open</span></li>`).join("")}</ul>` : '<div class="fr-empty">No insiders log picks before 2011.</div>'}</div>
    <div class="fr-panel"><div class="fr-ph"><span class="fr-k">Your area recruiters</span></div>${x.recruiters.length ? `<ul class="fr-list">${x.recruiters.map((c) => `<li><span>${coachLink(c.id, c.name)}</span><span>${esc(c.states.slice(0, 6).join(", "))}${c.states.length > 6 ? ` +${c.states.length - 6}` : ""}</span></li>`).join("")}</ul>` : '<div class="fr-empty">No areas set.</div>'}</div></div>`;
  return `<div class="pad fr">${head}${gotoFor("recruit_find")}${frBuilder(x, st, "college")}${frResults(x, st, FR_COLLEGE)}${foot}</div>`;
};

screens.find_prospects = async () => {
  const st = frSeat("nfl");
  if (st.level == null) st.level = "all";
  const args = frArgs(st, { conf: st.place || null });
  if (st.openSaved != null) { args.saved = st.openSaved; st.openSaved = null; }
  const x = await api("find_prospects", args);
  FR.last = { seat: "nfl", x };
  if (x.team && x.team.colors) setColors(x.team.colors);
  st.cls = x.class; st.level = x.level; st.sort = x.sort;
  if (!FR_NFL[st.cols]) st.cols = "overview";
  const sub = `${esc(x.team.name)} · the ${x.class} draft${x.this_draft ? ` (${esc(x.draft_date)})` : ", a year out"} · the combine ${esc(x.combine.join("-"))}`;
  const cells = [
    ["Not seen yet", x.unseen ? `${esc(x.unseen_text)} <small>${x.unseen_on ? "included" : "left out"}</small>` : "None <small>left out</small>"],
    ["All-star week", x.senior_bowl_done ? `${frN(x.sums.all_star)} <small>played in one</small>` : `${esc(x.senior_bowl)} <small>· ${frN(x.sums.all_star)} invited</small>`],
    ["Combine invites", `${frN(x.sums.combine)} <small>of these</small>`],
    ["Top-30 visits", `${frN(x.sums.visits)} of 30 <small>on your list</small>`],
  ];
  const note = x.combine_done
    ? "Testing shows the combine's numbers, or his pro day's once it is done; school numbers are marked self."
    : `Combine numbers arrive ${esc(x.combine.join("-"))} and pro days run through April; until then testing shows school numbers, marked self.`;
  const head = frHead(x, "nfl", `${teamLink(x.team)} · Scouting`, "Find prospects", sub, x.dist, cells, note);
  const foot = `<div class="fr-foot"><div class="fr-panel"><div class="fr-ph"><span class="fr-k">The desks' records</span></div><ul class="fr-list">${x.desks.map((d) => `<li><span>${esc(d.name)}</span><span>${d.top100 ? `top 100: ${d.top100.pct} of 100 went in the first 100 picks a year, ${esc(d.top100.years)}` : "no draft scored yet"}${d.mocks ? ` · mocks ${d.mocks.exact} exact, ${d.mocks.near} within three in ${d.mocks.drafts}` : ""}</span></li>`).join("")}<li><span>Consensus</span><span>the desks' weekly top 50, then their grades</span></li></ul></div>
    <div class="fr-panel"><div class="fr-ph"><span class="fr-k">Your board</span></div><p class="fr-mute">Your board's order and grades come from your scouts' files; ${linkTo("draftboard", {}, "the draft board")} sets them. "You vs the desks" marks a prospect at least 12 places (or a third of your rank) from the consensus.</p></div>
    <div class="fr-panel"><div class="fr-ph"><span class="fr-k">The year</span></div><ul class="fr-list"><li><span>Senior Bowl</span><span>${esc(x.senior_bowl)}</span></li><li><span>The combine</span><span>${esc(x.combine.join("-"))}</span></li><li><span>The draft</span><span>${esc(x.draft_date)}</span></li></ul></div></div>`;
  return `<div class="pad fr">${head}${gotoFor("find_prospects")}${frBuilder(x, st, "nfl")}${frResults(x, st, FR_NFL)}${foot}</div>`;
};

// ---------- the high school coach: Colleges calling (FR15) ----------
screens.colleges_calling = async () => {
  const x = await api("colleges_calling");
  if (S.status && S.status.mine) setColors(S.status.mine.colors);
  const k = keep("fr-hs", { level: "all", stage: "all", kid: "" });
  const hs = keep("sc-hs", { pid: null });
  const by = (p) => {
    const s = p.by.map((b) => (b.stars ? `${b.stars}★` : "not ranked"));
    // A signed kid's class is closed: no "yet" for him.
    return s.every((t) => t === "not ranked") ? frMute(p.signed ? "Not ranked by any service" : "Not ranked anywhere yet") :`<span class="fr-num" title="${esc(p.by.map((b) => b.name).join(" · "))}">${esc(s.join(" · "))}</span>`;
  };
  const heat = (h) => (h === "Hot" ? frWord("Hot", "hi") : h === "Quiet" ? frMute("Quiet") : esc(h));
  const players = x.players.length
    ? `<div class="fr-tbl"><table><thead><tr><th>Player</th><th>Pos</th><th>Class</th><th>By service <small class="fr-mute">${esc(x.services.map((s) => s.name).join(" · "))}</small></th><th>Composite</th><th class="r">Offers</th><th>Interest</th><th>Your read</th><th></th></tr></thead><tbody>
      ${x.players.map((p) => `<tr class="${hs.pid === p.id ? "on" : ""}"><td>${playerLink(p.id, p.name)}</td><td>${esc(p.pos)}</td><td>${esc(p.cls)}<div class="fr-l2">class of ${p.class_of}</div></td><td>${by(p)}</td><td>${p.comp ? `${frStars(p.stars)} <span class="fr-l2 fr-num">${esc(p.comp.replace(/^0/, ""))}</span>` : frStars(p.stars)}</td><td class="r">${frOffers(p)}</td><td>${p.signed ? `${p.walk_on ? "Walk-on" : "Signed"}: ${teamLink(p.signed)}` : p.committed ? `${p.walk_on ? "Walk-on commit" : "Committed"}: ${teamLink(p.committed)}` : heat(p.heat)}${p.offers ? `<div class="fr-l2">${linkTo("recruit_race", { id: p.id }, "The race")}</div>` : ""}</td><td>${p.read && p.read.now ? range(p.read.now, null, { p: p.id, k: "ovr" }) : frMute("Not seen")}</td><td>${p.signed ? "" : `<button class="fr-btn" data-act="sc-promote" data-id="${p.id}" data-name="${esc(p.name)}">Call a college</button>`}</td></tr>`).join("")}
      </tbody></table></div>`
    : '<div class="fr-panel fr-empty">No players on your roster.</div>';
  const pick = x.players.find((p) => p.id === hs.pid);
  const progs = [];
  for (const l of x.lines) if (!progs.some((t) => t.ref.id === l.team.ref.id)) progs.push(l.team);
  const callBox = pick
    ? `<div class="fr-panel fr-call"><div class="fr-ph"><span class="fr-k">Call a college about ${playerLink(pick.id, pick.name)} · ${esc(pick.pos)}, class of ${pick.class_of}</span></div>
      <p class="fr-mute">Your word is worth what your record makes it. Push a player who doesn't pan out and the next call counts for less.</p>
      <div class="fr-row"><label class="fr-k" for="sc-prog">Who you call</label><select class="fr-sel" id="sc-prog"><option value="">Every staff that recruits your area</option>${progs.map((t) => `<option value="${t.ref.id}">${esc(t.name)}</option>`).join("")}</select>
      <label class="fr-k" for="sc-strength">What you say</label><select class="fr-sel" id="sc-strength">${x.strengths.map((s, i) => `<option value="${i}">${esc(s)}</option>`).join("")}</select>
      <button class="fr-btn fr-main" data-act="sc-promotego" data-id="${pick.id}">Make the call</button><button class="fr-btn" data-act="sc-promote" data-id="">Cancel</button></div></div>`
    : "";
  const kids = [...new Map(x.lines.flatMap((l) => l.about).map((p) => [p.id, p])).values()];
  const lines = x.lines.filter((l) => (k.level === "all" || (k.level === "small" ? !["FBS", "FCS"].includes(l.level) : l.level.toLowerCase() === k.level)) && (k.stage === "all" || l.stage_key === k.stage) && (!k.kid || l.about.some((p) => String(p.id) === String(k.kid))));
  const view = k.view === "map" ? "map" : "table";
  const calls = `<div class="fr-sect"><h2>Every college calling</h2><span class="fr-mute">${view === "map" ? "Around your school · filter by level, stage or player" : "Latest contact first · filter by level, stage or player"}</span><span class="fr-push"></span>${frSeg("fr-hsview", [["table", "Table"], ["map", "Map"]], view, "View")}</div>
    <div class="fr-panel fr-builder"><div class="fr-row">${frSeg("fr-hsf", [["all", "Every level"], ["fbs", "FBS"], ["fcs", "FCS"], ["small", "D2 and below"]], k.level, "Level").replace(/data-act="fr-hsf"/g, 'data-act="fr-hsf" data-f="level"')}
      ${frSeg("fr-hsf", [["all", "Any stage"], ["offer", "Offers"], ["visit", "Visits"], ["camp", "Camp invites"], ["interest", "Interest"], ["commit", "Committed or signed"]], k.stage, "Stage").replace(/data-act="fr-hsf"/g, 'data-act="fr-hsf" data-f="stage"')}
      <select class="fr-sel" data-act="fr-hskid" aria-label="Player"><option value="">Every player</option>${kids.map((p) => `<option value="${p.id}"${String(k.kid) === String(p.id) ? " selected" : ""}>${esc(p.name)}</option>`).join("")}</select></div></div>
    ${view === "map" ? frHsMap(x, lines) : `<div class="fr-tbl"><table><thead><tr><th>College</th><th>Level</th><th>Who calls</th><th>About</th><th>Stage</th><th>Last contact</th><th>What they want</th><th>How they take your word</th></tr></thead><tbody>
    ${lines.map((l) => `<tr><td>${frTeam(l.team)}</td><td>${esc(l.level)}<div class="fr-l2">${esc(l.conf)}</div></td><td>${l.caller.id != null ? coachLink(l.caller.id, l.caller.name) : esc(l.caller.name)}<div class="fr-l2">${esc(l.caller.role)}</div></td><td class="wrap">${l.about.map((p) => playerLink(p.id, p.name)).join(", ")}</td><td>${/^(Offer|Committed|Signed)/.test(l.stage) ? `<b>${esc(l.stage)}</b>` : esc(l.stage)}</td><td class="fr-num">${esc(l.last)}</td><td class="wrap">${esc(l.want)}</td><td>${l.credit_word === "Low" ? '<span class="fr-amber">Low</span>' : esc(l.credit_word)}</td></tr>`).join("") || `<tr><td colspan="8"><div class="fr-empty">${x.lines.length ? "No college matches these filters." : "No college has called about your players yet."}</div></td></tr>`}
    </tbody></table></div>`}`;
  const yourCalls = `<div class="fr-panel"><div class="fr-ph"><span class="fr-k">Your calls</span><span class="fr-mute">${x.calls.length}</span></div>${x.calls.length ? `<ul class="fr-list">${x.calls.map((c) => `<li><span>${esc(c.date)} · ${playerLink(c.player.id, c.player.name)} to ${c.to ? teamLink(c.to) : "every staff in your area"}: “${esc(c.said)}”</span><span>${c.settled ? (/right/.test(c.settled) ? esc(c.settled) : `<span class="fr-amber">${esc(c.settled)}</span>`) : "settled in two seasons"}</span></li>`).join("")}</ul>` : '<div class="fr-empty">You haven\'t called anyone yet.</div>'}</div>`;
  const s = x.sums;
  const cells = [
    ["Offers", `${frN(s.offers)} <small>to ${frPlural(s.offered, "player")}</small>`],
    ["Camp invites", frN(s.camps)],
    ["Your credibility", `${esc(s.credit_word)} <small>· ${s.calls_settled ? `${s.calls_right} of ${s.calls_settled} call${s.calls_settled === 1 ? "" : "s"} right in 5 years` : "no call settled yet"}</small>`],
    ["Services' next update", esc(x.next_update)],
  ];
  const head = `<div class="fr-head"><div><div class="fr-k">${teamLink(x.team)} · Recruiting</div><h1 class="fr-h1">Colleges calling</h1><div class="fr-sub">${esc(x.team.name)} · head coach · ${esc(S.status && S.status.today ? S.status.today : "")} · ${esc(x.period)}</div>
      <div class="fr-count"><b class="fr-num">${frN(x.lines.length)}</b><span>${x.lines.length === 1 ? "college" : "colleges"} calling about ${frPlural(Number(x.count), "of your players", "of your players")}</span></div>
      <div class="fr-rule">As a high school coach you don't search for recruits: colleges come to you. This is every college staff in touch about your players, what they want and how much they trust your word.</div></div>
    <div class="fr-panel"><div class="fr-ph"><span class="fr-k">This year</span></div><div class="fr-cells">${cells.map(([k2, v]) => `<div><span class="fr-k">${esc(k2)}</span><span class="fr-v">${v}</span></div>`).join("")}</div><div class="fr-mini">College staffs weigh what you tell them by how your past calls turned out.</div></div></div>`;
  return `<div class="pad fr">${head}${gotoFor("hsside")}<div class="fr-sect"><h2>Your players being recruited</h2><span class="fr-mute">Stars by service: ${esc(x.services.map((q) => q.name).join(" · "))}</span></div>${callBox}${players}${calls}<div class="fr-foot">${yourCalls}</div></div>`;
};
screens.hsside = screens.colleges_calling;

// ---------- the drawers beside ----------
screens.find_offers = async (a) => {
  const x = await api("find_offers", { id: a.id });
  S.title = `${x.name}'s offers`;
  return `<div class="fr pad"><div class="fr-k">${esc(x.pos)} · ${esc(x.cls)}</div><h2 class="fr-h2">${playerLink(x.id, x.name)} <small class="fr-mute">${frPlural(x.count, "offer")}</small></h2>
    <div class="fr-tbl"><table><thead><tr><th>School</th><th>Level</th><th>Offered</th><th></th></tr></thead><tbody>
    ${x.offers.map((o) => `<tr class="${o.yours ? "on" : ""}"><td>${frTeam(o.team)}</td><td>${esc(o.level)}<div class="fr-l2">${esc(o.conf)}</div></td><td>${o.date ? esc(o.date) : frMute("—")}</td><td>${o.committed ? frWord("Committed", "hi") : o.yours ? frWord("Yours") : ""}${o.listed ? ` ${frWord("On his list")}` : ""}</td></tr>`).join("") || '<tr><td colspan="4"><div class="fr-empty">No offers.</div></td></tr>'}
    </tbody></table></div><p class="fr-mini">${esc(x.note)}</p></div>`;
};
screens.find_insider = async (a) => {
  const x = await api("find_insider", a.desk != null ? { desk: a.desk } : { id: a.id });
  S.title = x.name ? `${x.name}, ${x.desk}` : "Insider picks";
  const rows = (x.picks || []).map((p) => `<tr><td>${p.player ? playerLink(p.player.id, p.player.name) : ""}<div class="fr-l2">${p.player ? `${esc(p.player.pos)} · class of ${p.player.cls}` : ""}</div></td><td>${teamLink(p.team)}</td><td class="r fr-num">${p.conf}%</td><td>${esc(p.date)}</td></tr>`).join("");
  const rec = x.last ? `<div class="fr-cells"><div><span class="fr-k">Last year</span><span class="fr-v">${x.last[1]} of ${x.last[0]} right</span></div><div><span class="fr-k">This year</span><span class="fr-v">${x.this[1]} of ${x.this[0]} right</span></div></div>` : "";
  return `<div class="fr pad"><h2 class="fr-h2">${esc(x.name || "Insider picks")}</h2>${rec}
    <div class="fr-tbl"><table><thead><tr><th>Recruit</th><th>Pick</th><th class="r">Sure</th><th>Logged</th></tr></thead><tbody>${rows || '<tr><td colspan="4"><div class="fr-empty">No open picks.</div></td></tr>'}</tbody></table></div>${x.note ? `<p class="fr-mini">${esc(x.note)}</p>` : ""}</div>`;
};

// ---------- clicks and changes (data-act "fr-...") ----------
const FR = { last: null };
function frState() {
  const scr = S.route.screen;
  return scr === "find_prospects" ? frSeat("nfl") : frSeat("college");
}
function frGo(fn) { const st = frState(); fn(st); render($(".view") ? $(".view").scrollTop : 0); }
function frFilters(st) { return st.filters ? st.filters.slice() : FR.last ? frKeepFilters(FR.last.x) : []; }
function frCsv() {
  const L = FR.last;
  if (!L) return;
  const st = frState();
  const views = L.seat === "nfl" ? FR_NFL : FR_COLLEGE;
  const cols = (views[st.cols] || views.overview).cols(L.x).filter((c) => c.t);
  const strip = (h) => { const d = document.createElement("div"); d.innerHTML = h; return d.textContent.replace(/\s+/g, " ").trim(); };
  const q = (s) => `"${String(s).replace(/"/g, '""')}"`;
  const lines = [cols.map((c) => q(c.t)).join(",")].concat(L.x.rows.map((r) => cols.map((c) => q(strip(c.f(r)))).join(",")));
  const blob = new Blob([lines.join("\n")], { type: "text/csv" });
  const a = document.createElement("a");
  a.href = URL.createObjectURL(blob);
  a.download = `${L.seat === "nfl" ? "find-prospects" : "find-recruits"}-${L.x.class}.csv`;
  a.click();
  setTimeout(() => URL.revokeObjectURL(a.href), 1000);
}
// The filter editor's typed values, kept before it is drawn again.
function frKeepTyped(e) {
  const v = $("#fr-v"), v2 = $("#fr-v2");
  if (v) e.v = v.value;
  if (v2) e.v2 = v2.value;
}
document.addEventListener("click", async (ev) => {
  const el = ev.target.closest("[data-act]");
  if (!el || !String(el.dataset.act).startsWith("fr-")) return;
  const d = el.dataset;
  if (el.tagName === "SELECT" || (el.tagName === "INPUT" && el.type !== "checkbox")) return;
  const inner = ev.target.closest("a[data-go]");
  if (inner && inner !== el && el.contains(inner)) return;
  ev.preventDefault();
  ev.stopPropagation();
  switch (d.act) {
    case "fr-cls": frGo((st) => { st.cls = Number(d.v); st.page = 0; st.picked = []; st.sort = null; st.dir = 1; }); break;
    case "fr-level": frGo((st) => { st.level = d.v; st.page = 0; st.sort = null; st.dir = 1; st.picked = []; }); break;
    case "fr-sure": frGo((st) => { st.sure = d.v; st.page = 0; }); break;
    case "fr-unseen": frGo((st) => { st.unseen = d.v === "1"; st.page = 0; }); break;
    case "fr-view": frGo((st) => { st.view = d.v; }); break;
    case "fr-cols": frGo((st) => { st.cols = d.v; }); break;
    case "fr-sort": frGo((st) => { if (st.sort === d.k) st.dir = st.dir === -1 ? 1 : -1; else { st.sort = d.k; st.dir = 1; } st.page = 0; }); break;
    case "fr-flip": frGo((st) => { st.dir = st.dir === -1 ? 1 : -1; st.page = 0; }); break;
    case "fr-page": frGo((st) => { st.page = Math.max(0, st.page + Number(d.d)); }); break;
    case "fr-cat": frGo((st) => { st.cat = !st.cat; }); break;
    case "fr-pick": frGo((st) => { st.edit = { k: d.k, op: null, v: "", v2: "", or: false }; st.cat = false; }); break;
    case "fr-editx": frGo((st) => { st.edit = null; }); break;
    case "fr-join": frGo((st) => { if (st.edit) { frKeepTyped(st.edit); st.edit.or = d.v === "or"; } }); break;
    case "fr-addgo": {
      const st = frState();
      if (!st.edit) break;
      const v = $("#fr-v"), v2 = $("#fr-v2"), op = $("#fr-op");
      const val = (e) => (!e ? "" : e.tagName === "SELECT" && /^\d+$/.test(e.value) ? Number(e.value) : e.value.trim());
      const t = { k: st.edit.k, op: op ? op.value : "is", v: val(v), v2: val(v2), or: !!st.edit.or };
      if (t.v === "" || (t.op === "between" && t.v2 === "")) { toast("Give the filter a value."); break; }
      const list = frFilters(st);
      if (!list.length) t.or = false;
      list.push(t);
      frGo((s) => { s.filters = list; s.edit = null; s.page = 0; });
      break;
    }
    case "fr-rm": frGo((st) => { const list = frFilters(st); list.splice(Number(d.i), 1); if (list[0]) list[0].or = false; st.filters = list; st.page = 0; }); break;
    case "fr-andor": frGo((st) => { const list = frFilters(st); const i = Number(d.i); if (list[i]) list[i].or = !list[i].or; st.filters = list; st.page = 0; }); break;
    case "fr-reset": frGo((st) => { st.filters = null; st.sort = null; st.dir = 1; st.page = 0; }); break;
    case "fr-tick": frGo((st) => { const id = Number(d.id); st.picked = st.picked.includes(id) ? st.picked.filter((x) => x !== id) : st.picked.concat(id).slice(-5); }); break;
    case "fr-offers": openBeside("find_offers", { id: Number(d.id) }); break;
    case "fr-insider": openBeside("find_insider", { desk: Number(d.desk) }); break;
    case "fr-csv": frCsv(); break;
    case "fr-save": {
      const L = FR.last;
      if (!L) break;
      const name = await ask("Save this search", { placeholder: "A name, like \"In-state edge rushers\"", ok: "Save", note: "Your staff tells you in the inbox when it finds someone new." });
      if (!name) break;
      try { const r = await api("find_save", { name, spec: L.x.spec }); toast(r.said); render($(".view").scrollTop); } catch (e) { toast(e.message); }
      break;
    }
    case "fr-forget": {
      if (!(await sure("Forget this saved search?", "It stops watching for new players.", "Forget it"))) break;
      try { const r = await api("find_forget", { k: Number(d.k) }); toast(r.said); render($(".view").scrollTop); } catch (e) { toast(e.message); }
      break;
    }
    case "fr-open": {
      const L = FR.last;
      const s = L && L.x.saved.find((q) => String(q.k) === d.k);
      if (!s) break;
      const sp = s.spec || {};
      frGo((st) => {
        st.cls = sp.class ?? st.cls; st.level = sp.level ?? st.level; st.pos = sp.pos || "ALL"; st.place = sp.state || sp.conf || "";
        st.q = sp.q || ""; st.filters = sp.filters ?? null; st.sure = sp.sure || "any"; st.unseen = !!sp.unseen; st.page = 0; st.openSaved = Number(d.k);
      });
      break;
    }
    case "fr-hsf": { const k = keep("fr-hs", {}); k[d.f] = d.v; render($(".view").scrollTop); break; }
    case "fr-hsview": { keep("fr-hs", {}).view = d.v; render($(".view").scrollTop); break; }
    case "fr-hsst": { const k = keep("fr-hs", {}); k.mapst = d.v && k.mapst !== d.v ? d.v : null; render($(".view").scrollTop); break; }
    // The recruiting map (board 7).
    case "fr-mapmode": { keep("frmap", {}).mode = d.v; render($(".view").scrollTop); break; }
    case "fr-mapst": { const k = keep("frmap", {}); k.st = d.v; k.pp = null; render($(".view").scrollTop); break; }
    case "fr-mapdot": { const k = keep("frmap", {}); k.st = d.st; k.pp = Number(d.id); render($(".view").scrollTop); break; }
    case "fr-mappp": { const k = keep("frmap", {}); k.pp = k.pp === Number(d.id) ? null : Number(d.id); render($(".view").scrollTop); break; }
    // The race (board 8).
    case "fr-racesnap": case "fr-racerival": case "fr-racefac": case "fr-raceev": {
      const k = keep("frrace:" + FR.race, {});
      const f = { "fr-racesnap": "snap", "fr-racerival": "rival", "fr-racefac": "fac", "fr-raceev": "ev" }[d.act];
      k[f] = f === "ev" && k.ev === Number(d.v) ? null : Number(d.v);
      render($(".view").scrollTop);
      break;
    }
  }
}, true);
document.addEventListener("change", (ev) => {
  const el = ev.target.closest("[data-act]");
  if (!el || !String(el.dataset.act).startsWith("fr-")) return;
  const d = el.dataset;
  switch (d.act) {
    case "fr-f": frGo((st) => { st[d.f] = el.value; st.page = 0; }); break;
    case "fr-q": frGo((st) => { st.q = el.value.trim(); st.page = 0; }); break;
    case "fr-x": frGo((st) => { st.x = el.value; }); break;
    case "fr-y": frGo((st) => { st.y = el.value; }); break;
    case "fr-op": frGo((st) => { if (st.edit) { frKeepTyped(st.edit); st.edit.op = el.value; } }); break;
    case "fr-hskid": { keep("fr-hs", {}).kid = el.value; render($(".view").scrollTop); break; }
  }
});
