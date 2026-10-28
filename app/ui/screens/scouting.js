// Lane screens: scouting (docs/final-build/plan.md). Registers screens.<name>.
// Section 7 from your seat (docs/design/scouting.md, screens topics 6-9):
// the scouting file, your department, the draft board and its lists, the
// pre-draft season, the war room and the undrafted rush, the draft record,
// pro scouting, a college's recruit, portal and JUCO boards, and the high
// school coach's calls to colleges. Every read is a range or words, never a
// true number; every name is a link; every page has its "Go to" line.
"use strict";

// ---------- shared pieces ----------
const SC = { askOpen: false, reveal: null, clock: null, quote: null, live: false, timer: null };
const scSeat = () => (S.status && S.status.mine ? S.status.mine.ref.kind : null);
const scScroll = () => ($(".view") ? $(".view").scrollTop : 0);
// A player's name opens his scouting file; the plain player page is one more link away.
const scName = (id, name) => `<a class="link name" data-go="scout" data-id="${id}">${esc(name)}</a>`;
const scWhere = (r) => (r.team ? teamLink(r.team) : esc(r.school || ""));
const scStars = (n) => (n ? `<span class="sc-stars">${"★".repeat(n)}</span>` : '<span class="muted">-</span>');
const scCap = (s) => String(s || "").replace(/^./, (c) => c.toUpperCase());
const scPlural = (n, one, many) => `${n} ${n === 1 ? one : many || one + "s"}`;
const scTier = (t) => (Array.isArray(t) ? t : t && t.name ? (Array.isArray(t.name) ? t.name : [t.name, ""]) : ["", ""]);
const scTeamOf = (kind, id, name) => ({ ref: { kind, id }, name });
// The "Go to" line (screens topic 17): core's gotoLine, never the screen
// you're on. Each screen also registers its map in GOTO below.
function scGoto(list) {
  return gotoLine(list.filter((x) => x && x[0] !== S.route.screen).slice(0, 5).map(([scr, label, args]) => [label, scr, args || {}]));
}
function scHead(t, sub, goto, kicker) {
  return pageHead({ kicker: kicker || "", title: t, sub: sub || "", goto: goto || "" });
}
// Tabs on core's tab bar: the tab is kept in the route, so Back returns to it.
function scTabs(key, list) {
  const names = list.map(([, label]) => label);
  const cur = tabOf(key, names, S.route.args);
  const hit = list.find(([, label]) => label === cur) || list[0];
  return { t: hit[0], html: tabs(key, list.map(([, label, n]) => ({ name: label, badge: n == null ? "" : n })), cur) };
}
function scSeg(key, field, opts) {
  const k = S.keep[key];
  return `<div class="seg">${opts.map(([v, t, n]) => `<button data-act="sc-seg" data-k="${key}" data-f="${field}" data-v="${esc(v)}" aria-pressed="${String(k[field]) === String(v)}">${esc(t)}${n != null ? `<small>${n}</small>` : ""}</button>`).join("")}</div>`;
}
// Your staff's short read in a row: the range now and the role in words.
function scNow(read, id) {
  return read && read.now ? range(read.now, null, { p: id, k: "ovr" }) : range(null);
}
function scRole(read) {
  if (!read || !read.role || read.role === "Not seen") return '<span class="muted">Not seen</span>';
  return `<span class="sc-role">${esc(read.role)}</span>${read.sure ? ` <small class="muted">· ${esc(read.sure)}</small>` : ""}`;
}
function scMed(word) {
  if (!word) return '<span class="muted">-</span>';
  const k = /Cleared/.test(word) ? "good" : /Minor/.test(word) ? "warn" : "bad";
  return `<span class="dot ${k}"></span>${esc(word)}`;
}
function scEmpty(text) {
  return `<div class="empty">${esc(text)}</div>`;
}
// Days as squares (scouting topic 3): solid used, hollow open.
function scSquares(used, limit, label) {
  if (!limit) return "";
  const n = Math.min(limit, 120);
  return `<div class="sc-days"><div class="sc-days-h"><b class="st-cond">${Math.max(0, limit - used)}</b><span>of ${limit} ${esc(label)} left · ${used} used</span></div><div class="sc-squares">${Array.from({ length: n }, (_, i) => `<i class="${i < used ? "used" : ""}"></i>`).join("")}</div></div>`;
}
async function scDo(cmd, args, said) {
  try {
    const r = await api(cmd, args);
    if (said) toast(typeof said === "function" ? said(r) : said);
    return r;
  } catch (e) {
    toast(e.message || String(e));
    return null;
  }
}
const scRedraw = () => render(scScroll());

// ---------- the sidebar and the crumbs ----------
Object.assign(I, {
  scout: '<circle cx="11" cy="11" r="6.5"/><path d="M20 20l-4.4-4.4M8.5 11h5M11 8.5v5"/>',
  board: '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M3 9h18M8 9v11M3 14h18"/>',
  room: '<circle cx="12" cy="13" r="7"/><path d="M12 9v4l2.5 2M9 3h6"/>',
  pro: '<path d="M4 19V9l8-5 8 5v10M9 19v-5h6v5"/><path d="M12 9v.01"/>',
  calls: '<path d="M5 4h4l2 5-2.5 1.5a11 11 0 0 0 5 5L15 13l5 2v4a1 1 0 0 1-1 1A16 16 0 0 1 4 5a1 1 0 0 1 1-1z"/>',
});
I.clip = '<rect x="5" y="4" width="14" height="17" rx="2"/><path d="M9 4V3h6v1M8.5 10h7M8.5 14h7M8.5 18h4"/>';
// The sidebar: a Scouting section for the NFL and college seats (between
// your team and the world); a high school coach's calls sit with his team.
if (!NAV_SECS.some(([k]) => k === "scout")) NAV_SECS.splice(NAV_SECS.findIndex(([k]) => k === "world"), 0, ["scout", "Scouting"]);
navAdd({ go: "scouting", label: "Department", icon: "scout", sec: "scout", seats: ["nfl", "college"] });
navAdd({ go: "draftboard", label: "Draft board", icon: "board", sec: "scout", seats: ["nfl"] });
navAdd({ go: "predraft", label: "Pre-draft", icon: "clip", sec: "scout", seats: ["nfl"] });
// The war room shows while the draft is live from your chair.
{
  const room = { go: "warroom", label: "War room", icon: "room", sec: "scout", badge: () => (S.status && S.status.draft_live ? "Live" : 0) };
  Object.defineProperty(room, "seats", { get: () => (S.status && S.status.draft_live ? ["nfl"] : ["none"]), enumerable: true });
  navAdd(room);
}
navAdd({ go: "proscout", label: "Pro scouting", icon: "pro", sec: "scout", seats: ["nfl"] });
navAdd({ go: "recruitboard", label: "Recruit board", icon: "board", sec: "scout", seats: ["college"] });
navAdd({ go: "hsside", label: "Colleges calling", icon: "calls", sec: "team", seats: ["hs"], after: "recruiting" });
Object.assign(CRUMBS, {
  scout: "Scouting file",
  scouting: "Scouting",
  draftboard: "Draft board",
  predraft: "Pre-draft",
  warroom: "War room",
  draftrecord: "Draft record",
  proscout: "Pro scouting",
  recruitboard: "Recruit board",
  hsside: "Colleges calling",
});
// Where each screen leads (topic 17).
GOTO.scout = (a, f) => [["Player page", "player", { id: a.id }], f && f.team ? [f.team.name, "team", f.team.ref] : null, scSeat() === "nfl" ? ["Draft board", "draftboard"] : scSeat() === "college" ? ["Recruit board", "recruitboard"] : ["Colleges calling", "hsside"], ["My players", "myplayers"]];
GOTO.scouting = () => (scSeat() === "nfl" ? [["Draft board", "draftboard"], ["Pre-draft", "predraft"], ["Pro scouting", "proscout"], ["Staff", "staff"], ["My players", "myplayers"]] : [["Recruit board", "recruitboard"], ["Recruiting", "recruiting"], ["Staff", "staff"], ["My players", "myplayers"]]);
GOTO.draftboard = () => [["Pre-draft", "predraft"], ["Department", "scouting"], ["Contracts", "contracts"], ["Squad", "roster"]];
GOTO.predraft = () => [["Draft board", "draftboard"], ["Department", "scouting"], ["Calendar", "calendar"]];
GOTO.warroom = () => [["Draft board", "draftboard"], ["Pre-draft", "predraft"], ["Squad", "roster"], ["Contracts", "contracts"]];
GOTO.draftrecord = () => [["Squad", "roster"], ["Contracts", "contracts"], ["Draft board", "draftboard"], ["Pro scouting", "proscout"]];
GOTO.proscout = () => [["Squad", "roster"], ["Contracts", "contracts"], ["Draft board", "draftboard"], ["Department", "scouting"]];
GOTO.recruitboard = () => [["Recruiting", "recruiting"], ["Department", "scouting"], ["Squad", "roster"], ["My players", "myplayers"]];
GOTO.hsside = () => [["Squad", "roster"], ["Recruiting", "recruiting"], ["Schedule", "schedule"], ["Calendar", "calendar"]];
const scGo = (screen, args, data) => gotoFor(screen, args, data);

// ---------- the scouting file (screens topic 7, scouting topics 1, 4-8) ----------
const SC_CHIP = { film: "F", live: "L", camp: "C", talk: "T" };
const SC_CHIP_T = { film: "Film", live: "Seen live", camp: "Camp or testing", talk: "Talked to people" };
const scChips = (c) => (c || []).map((x) => `<span class="sc-chip ${x}" title="${SC_CHIP_T[x] || x}">${SC_CHIP[x] || x}</span>`).join("");

screens.scout = async (a) => {
  const f = await api("scout_file", { id: a.id });
  if (f.team) setColors(f.team.colors);
  const seat = scSeat();
  const s = f.summary;
  const b = f.board || {};
  const tab = scTabs("sc-file", [
    ["Summary", "Summary"],
    ["Ratings", "Ratings"],
    ["Looks", "Looks", f.looks_n],
    ...(f.tendencies.length ? [["Tendencies", "Tendencies", f.tendencies.length]] : []),
    ["Character", "Character"],
    ["Medical", "Medical and academics"],
    ["History", "History"],
    ...(typeof evFileTab === "function" && ["nfl", "college", "hs"].includes(scSeat()) ? [["Look", "Your look"]] : []),
  ]);
  // The hero: measured or listed, the public set apart from your staff.
  const size = `${f.measured ? `<b>${esc(f.ht)}, ${f.wt}</b> measured` : `<span class="muted">listed ${esc(f.ht)}, ${f.wt}</span>`}${f.arm ? ` · arms ${f.arm.toFixed(1)}"` : ""}${f.hand ? ` · hands ${f.hand.toFixed(1)}"` : ""}`;
  const where = f.team ? teamLink(f.team) : esc(f.school || "");
  const pub = [
    f.stars ? `<b class="st-cond sc-big">${f.stars}★</b>` : "",
    b.public ? `<span>No. ${b.public} on the public board</span>` : "",
    f.followers ? `<span>${esc(f.followers)} followers</span>` : "",
    f.nil ? `<span>NIL ${money(f.nil)}</span>` : "",
  ].filter(Boolean).join("");
  const staffCard = s
    ? `<b class="sc-hero-role">${esc(s.role)}</b><span>${esc(s.floor || "")}${s.floor ? ". " : ""}Staff ${esc(s.sure)}.</span><span class="sc-dim">${scPlural(f.looks_n, "look")} by ${scPlural(f.readers_n, "person", "people")}</span>`
    : `<b class="sc-hero-role">Not seen</b><span>Your staff hasn't looked at him yet.</span>`;
  const boardLine = [
    b.rank ? `No. ${b.rank} on your board` : "",
    b.tier ? `Tier ${b.tier}` : "",
    b.dnd ? `<span class="st-pill live" title="${esc(b.dnd)}">Do not draft</span>` : "",
    b.pfa ? '<span class="st-pill good">Priority free agent</span>' : "",
    ...(b.lists || []).map((l) => `<span class="st-pill">${esc(l)}</span>`),
    f.mine ? '<span class="st-pill gold">Your player</span>' : "",
    f.followed ? '<span class="st-pill gold">In My players</span>' : "",
  ].filter(Boolean).join(" ");
  const ask = SC.askOpen && f.can_ask.length
    ? `<div class="sc-askbox st-glass"><div><b>Ask for a new look</b><small>What your seat's rules allow now. It goes on your staff's list.</small></div>${f.can_ask.map((k) => `<button class="st-btn small glass" data-act="sc-ask" data-id="${f.id}" data-kind="${esc(k)}">${esc(k)}</button>`).join("")}</div>`
    : "";
  const hero = `<div class="hero small sc-filehero"><div class="art"></div><div class="st-outline">${esc(f.pos)}</div>
    <div class="copy">
      <div class="sc-pills"><span class="st-pill">${esc(f.pos)}</span><span class="st-pill">${esc(f.class)}</span><span class="st-label" style="color:rgba(255,255,255,.72)">${where}${f.hometown ? ` · ${esc(f.hometown)}` : ""}${f.high_school && f.high_school !== f.school ? ` · ${esc(f.high_school)}` : ""}</span></div>
      <h1>${esc(f.name)}</h1>
      <div class="meta"><span>${size}</span><span>Age <b>${f.age}</b></span>${boardLine ? `<span>${boardLine}</span>` : ""}</div>
      <div class="st-actions">
        ${f.can_ask.length ? `<button class="st-btn glass" data-act="sc-askopen" aria-pressed="${SC.askOpen}">Ask for a new look</button>` : ""}
        <button class="st-btn glass" data-act="sc-follow" data-id="${f.id}" data-on="${f.followed ? 0 : 1}">${f.followed ? "Following" : "Follow"}</button>
        <button class="st-btn glass" data-go="player" data-id="${f.id}">Player page</button>
      </div>
    </div>
    <div class="sc-hero-card st-glass">
      <div><span class="st-label">The public</span><div class="sc-pub">${pub || '<span class="sc-dim">No stars, no public rank</span>'}</div></div>
      <div><span class="st-label">Your staff${S.status && S.status.mine ? " · " + esc(S.status.mine.name) : ""}</span>${staffCard}</div>
    </div></div>${ask}`;
  let body = "";
  if (tab.t === "Summary") body = scFileSummary(f);
  else if (tab.t === "Ratings") body = scFileRatings(f);
  else if (tab.t === "Looks") body = scFileLooks(f);
  else if (tab.t === "Tendencies") body = scFileTend(f);
  else if (tab.t === "Character") body = scFileChar(f);
  else if (tab.t === "Medical") body = scFileMed(f);
  else if (tab.t === "Look") body = await evFileTab(f.id);
  else body = scFileHist(f);
  const goto = scGo("scout", { id: f.id }, f);
  title(f.name, "Scouting file");
  return `${hero}<div class="pad"><div class="sc-tabrow">${tab.html}${goto}</div>${body}</div>`;
};

function scFileSummary(f) {
  const s = f.summary;
  if (!s) {
    return `<div class="panel warm sc-unseen"><h2>Your staff hasn't seen him</h2><p class="sub">No film, no live look, no call. Only his listed size and what the public says are known. ${f.can_ask.length ? "Ask for a look above and he goes on your staff's list." : "Your seat can't ask for a look at him."}</p>${f.stories.length ? `<div class="items">${f.stories.map((x) => `<div class="item story"><span>${rich(x)}</span></div>`).join("")}</div>` : ""}</div>`;
  }
  const hints = s.hints.length
    ? s.hints.map((h) => `<div class="sc-hint"><span class="st-pill">Hint</span><p>${esc(h.text)}</p>${h.by ? `<small>Found by ${esc(h.by)}</small>` : ""}</div>`).join("")
    : scEmpty("Nobody on your staff has picked anything up yet. Hints come from good evaluators, and any could be wrong.");
  const comp = s.comparison
    ? `<button class="st-row sc-comp" data-go="player" data-id="${s.comparison.id}"><span class="st-row-thumb sc-thumb">${esc(initials(s.comparison.name))}</span><span><b>${esc(s.comparison.name)}</b><small>${esc(s.comparison.where)}</small></span><span class="st-row-col">From your world's history</span></button><small class="sc-dim">Matched on body and style, never on potential.</small>`
    : scEmpty("Nobody in your world's history plays quite like him yet.");
  const b = f.board || {};
  const reads = [
    b.rank ? `<li><b>Your board: No. ${b.rank}</b>${b.tier ? `, tier ${b.tier}` : ""}.</li>` : "",
    b.public ? `<li><b>The public: No. ${b.public}.</b> ${b.rank ? (Math.abs(b.public - b.rank) >= Math.max(12, Math.round(b.rank * 0.35)) ? (b.public > b.rank ? "You are out on a limb: higher on him than the public." : "You are out on a limb: lower on him than the public.") : "Close to your own order.") : ""}</li>` : "",
    f.stars ? `<li><b>The services: ${f.stars}★.</b> ${s.role ? `Your staff: ${esc(s.role).toLowerCase()}.` : ""}</li>` : "",
    f.choices && f.choices.length ? `<li><b>His choices:</b> ${esc(f.choices.join(", "))}.</li>` : "",
  ].filter(Boolean).join("");
  const voices = f.voices.length
    ? f.voices.map((v) => `<div class="sc-voice"><div><b>${esc(v.who)}</b><span>${esc(v.job)} · ${scPlural(v.looks, "look")}</span></div><p>“${rich(v.says)}”</p></div>`).join("")
    : scEmpty("Nobody has three looks at him yet, so the room has no voices on him.");
  return `<div class="sc-sum">
    <div class="panel warm"><h2>Your staff's grade <small>${scPlural(f.looks_n, "look")} by ${scPlural(f.readers_n, "person", "people")}</small></h2>
      <div class="sc-grade"><div><b class="sc-role-big">${esc(s.role)}</b><span class="sub">Staff ${esc(s.sure)}.</span></div><div class="sc-nowread"><span class="st-label">His level now</span>${range(s.now, null, { p: f.id, k: "ovr" })}</div></div>
      <dl class="kv"><dt>Floor and ceiling</dt><dd>${esc(s.floor || "Not read yet")}</dd>
      <dt>Best at</dt><dd>${esc(s.strengths.join(", ") || "-")}</dd><dt>Watch</dt><dd>${esc(s.weaknesses.join(", ") || "-")}</dd>
      <dt>Medical</dt><dd>${scMed(s.medical)}${s.examined ? "" : ' <small class="muted">· not examined, records only</small>'}</dd></dl></div>
    <div class="sc-col"><div class="panel warm"><h2>Hints <small>real things they noticed; any could be wrong</small></h2>${hints}</div>
      <div class="panel warm"><h2>Plays like a young</h2>${comp}</div></div>
    <div class="sc-col"><div class="panel warm"><h2>Your read and the public's</h2>${reads ? `<ul class="sc-ul">${reads}</ul>` : scEmpty("The public has no word on him.")}</div>
      <div class="panel warm"><h2>The room <small>everyone with three looks or more</small></h2>${voices}</div></div>
  </div>${f.stories.length ? `<div class="panel warm"><h2>What happened</h2><div class="items">${f.stories.map((x) => `<div class="item story"><span>${rich(x)}</span></div>`).join("")}</div></div>` : ""}`;
}

function scFileRatings(f) {
  const k = keep("sc-rat", { all: false, sel: null });
  const groups = f.groups
    .map((g) => ({ ...g, attrs: g.attrs.filter((x) => k.all || x.matters || x.read) }))
    .filter((g) => g.attrs.length);
  const all = f.groups.flatMap((g) => g.attrs);
  const sel = all.find((x) => x.key === k.sel) || all.find((x) => x.read && x.matters) || all.find((x) => x.read);
  const panel = (g) => {
    const who = [...new Set(g.attrs.flatMap((x) => (x.per || []).map((p) => p.who).filter(Boolean)))];
    return `<div class="panel warm"><h2>${esc(g.name)} <small>${g.attrs.length}</small></h2><div class="opinion">${who.length ? "Opinion: " + esc(who.slice(0, 3).join(", ")) + (who.length > 3 ? ` and ${who.length - 3} more` : "") : f.measured && g.name === "Physical" ? "Measured; not graded by your staff yet" : "Not seen by your staff"}</div>
      ${g.attrs.map((x) => `<button class="attr sc-attr ${sel && sel.key === x.key ? "on" : ""}" data-act="sc-attr" data-key="${x.key}" title="${x.read ? `Read by ${x.readers}` : "Nobody on your staff has seen it"}"><span>${esc(x.label)}${x.matters ? "" : ' <small class="muted">·</small>'}</span><span class="sc-attr-r">${range(x.read, null, { p: f.id, k: x.key })}<span class="sc-chips">${scChips(x.chips)}</span></span></button>`).join("")}</div>`;
  };
  const side = sel
    ? `<div class="panel warm sc-sticky"><span class="st-label">How this range narrowed</span><h2>${esc(sel.label)}</h2><div>${range(sel.read, null, { p: f.id, k: sel.key })}</div>
      <div class="sc-chips big">${scChips(sel.chips) || '<span class="muted">No looks behind it</span>'}</div>
      <span class="st-label">Each reader's own range</span>
      ${(sel.per || []).length ? sel.per.map((p) => `<div class="attr"><span>${esc(p.who || "Your staff")}</span>${range(p.read, null, { p: f.id, k: sel.key })}</div>`).join("") + `<p class="sub">The file blends them: more looks and a better eye weigh more. ${scPlural(sel.readers, "reader")}.</p>` : '<p class="sub">Nobody on your staff has seen it, so there is nothing to blend.</p>'}
      <div class="sc-legend"><span class="st-label">Looks</span>${Object.keys(SC_CHIP).map((c) => `<span>${scChips([c])} ${SC_CHIP_T[c]}</span>`).join("")}</div></div>`
    : `<div class="panel warm">${scEmpty("No attribute has been read yet.")}</div>`;
  return `<div class="filters">${scSeg("sc-rat", "all", [[false, "What matters at his position"], [true, "Every attribute"]])}<span class="sub">Each bar is your staff's range on 1-99. A narrow bar is a sure read. Click one for who read it.</span></div>
    <div class="sc-ratings"><div class="sc-groups">${groups.map(panel).join("") || `<div class="panel warm">${scEmpty("Nothing read yet.")}</div>`}</div>${side}</div>`;
}

function scFileLooks(f) {
  const k = keep("sc-looks", { fam: "All" });
  const fams = [...new Set(f.looks.map((l) => l.family))];
  const rows = f.looks.filter((l) => k.fam === "All" || l.family === k.fam);
  const who = (l) => (l.link && l.link.kind === "coach" ? `<a class="link name" data-go="coach" data-id="${l.link.id}">${esc(l.who)}</a>` : `<b>${esc(l.who)}</b>`);
  const count = (fam) => f.looks.filter((l) => l.family === fam).length;
  return `<div class="st-tiles sc-tiles"><div><b>${f.looks_n}</b><small>looks in the file</small></div><div><b>${f.readers_n}</b><small>people read him</small></div>${fams.map((x) => `<div><b>${count(x)}</b><small>${esc(x.toLowerCase())}</small></div>`).join("")}</div>
    <div class="filters">${scSeg("sc-looks", "fam", [["All", "Every look", f.looks.length], ...fams.map((x) => [x, x, count(x)])])}</div>
    ${table("sc-looks", [
      { k: "date", t: "Date", l: true, v: (r) => f.looks.indexOf(r) * -1 },
      { k: "who", t: "Who looked", l: true, f: who },
      { k: "job", t: "Job", l: true },
      { k: "kind", t: "Kind of look", l: true, div: true, f: (r) => `<span class="st-pill">${esc(r.kind)}</span>` },
      { k: "family", t: "Family", l: true },
    ], rows, { empty: "No looks yet. Ask your staff for one from the top of the file." })}`;
}

function scFileTend(f) {
  const rows = f.tendencies.map((t) => {
    const [lo, hi] = t.read;
    const l = ((lo - 1) / 19) * 100, w = Math.max(3, ((hi - lo) / 19) * 100);
    return `<div class="sc-tend"><span><b>${esc(t.label)}</b><small>${esc(t.ends[0])} to ${esc(t.ends[1])}</small></span>
      <span class="sc-tend-t"><i class="mid"></i><i class="r" style="left:${l}%;width:${w}%"></i></span><span class="sc-tend-v"><b class="st-cond">${lo}-${hi}</b><span>${esc(scCap(t.word))}</span></span></div>`;
  }).join("");
  return `<div class="panel warm"><h2>Tendencies <small>how often he tries things, not how well · on 1-20, the tick is average</small></h2>
    <div class="opinion">A read from film: more film narrows it. ${f.mine ? "Your coaches see him every day." : ""}</div>${rows}</div>`;
}

function scFileChar(f) {
  const rows = f.character;
  const seen = rows.filter((c) => c.note);
  return `<div class="grid2"><div class="panel warm"><h2>Character <small>${rows.length} traits</small></h2>
    ${rows.length ? `<div class="sc-traits">${rows.map((c) => `<div class="sc-trait" title="${esc(c.note || "")}"><span>${esc(c.name)}</span><span>${c.word === "Not read yet" ? '<span class="muted">Not read yet</span>' : `<span class="dot ${c.good ? "good" : "warn"}"></span><b>${esc(c.word)}</b>`}</span><span class="muted">${esc(c.sure || "")}</span></div>`).join("")}</div>` : scEmpty("Your staff hasn't read his character yet: it takes calls, visits and time around him.")}</div>
    <div style="display:grid;gap:18px"><div class="panel warm"><h2>The notes <small>each says where it came from and how that source runs</small></h2>
      ${seen.length ? seen.map((c) => `<article class="sc-note"><div><span class="st-label">${esc(c.name)}</span><span class="st-pill">${esc(scCap(c.shade))}</span></div><p>${rich(c.note)}</p><small>${esc(scCap(c.source))}${c.who ? ` · ${esc(c.who)}` : ""}</small></article>`).join("") : scEmpty("No notes yet.")}</div>
    <div class="panel warm"><h2>His family</h2>${f.family.length ? `<div class="items">${f.family.map((x) => `<div class="item story"><span>${rich(x)}</span></div>`).join("")}</div>` : scEmpty("Nothing known about his family yet.")}</div></div></div>`;
}

function scFileMed(f) {
  const m = f.medical;
  const a = f.academics;
  const tests = f.tests.length
    ? tbl(["Where", "Date", "40", "Bench", "Vertical", "Broad", "3-cone", "Shuttle"], f.tests.map((t) => [esc(t.where), t.date, t.forty ? t.forty.toFixed(2) : "-", t.bench, t.vertical ? t.vertical.toFixed(1) : "-", t.broad, t.cone ? t.cone.toFixed(2) : "-", t.shuttle ? t.shuttle.toFixed(2) : "-"]))
    : scEmpty("He hasn't tested in front of your staff: no combine, pro day or camp in the file.");
  return `<div class="grid2"><div style="display:grid;gap:18px"><div class="panel warm"><h2>Medical <small>${m && m.examined ? "examined by your doctors" : "records and film only"}</small></h2>
      ${m ? `<div class="sc-medbig"><b class="st-wide">${esc(m.words)}</b><span class="sub">of: cleared, minor concern, major concern, failed</span></div>${(m.notes || []).length ? `<div class="items">${m.notes.map((x) => `<div class="item story"><span>${rich(x)}</span></div>`).join("")}</div>` : ""}${m.examined ? "" : '<div class="sc-flagline"><span class="st-pill">No exam</span><span class="st-pill">No imaging</span><span class="sub">A recheck or the combine puts your own doctors’ hands on him.</span></div>'}` : scEmpty("No medical read yet.")}</div>
    <div class="panel warm"><h2>Measured and timed</h2>${tests}</div></div>
    <div class="panel warm"><h2>Academics <small>${a.transcript ? "from his transcript" : "from what people say: no transcript yet"}</small></h2>
      <div class="st-tiles"><div><b>${a.gpa[0].toFixed(1)}-${a.gpa[1].toFixed(1)}</b><small>core GPA, your read</small></div>${a.test ? `<div><b>${a.test}</b><small>test score</small></div>` : ""}<div><b class="sc-tile-w">${esc(a.eligible)}</b><small>eligibility</small></div></div>
      <p class="sub">${a.transcript ? "Your academic staff has his transcript: the range is narrow." : "Without a transcript the GPA is a guess within a quarter point either way."}</p></div></div>`;
}

function scFileHist(f) {
  const b = f.board || {};
  return `<div class="grid2"><div class="panel warm"><h2>His path <small>season by season as the record shows it</small></h2>
    ${tbl(["Age", "Level", "Position", "Role", "Missed"], f.history.map((h) => [String(h.age), h.level, h.pos, words(h.role), h.missed]))  || scEmpty("No seasons on record yet.")}</div>
    <div style="display:grid;gap:18px"><div class="panel warm"><h2>Where he stands</h2><dl class="kv">
      <dt>Your board</dt><dd>${b.rank ? `No. ${b.rank}${b.tier ? `, tier ${b.tier}` : ""}` : "Not on it"}</dd>
      <dt>The public</dt><dd>${b.public ? `No. ${b.public}` : f.stars ? `${f.stars}★` : "Unranked"}</dd>
      ${b.dnd ? `<dt>Do not draft</dt><dd>${esc(b.dnd)}</dd>` : ""}
      ${(b.lists || []).length ? `<dt>Your lists</dt><dd>${esc(b.lists.join(", "))}</dd>` : ""}
      ${f.choices.length ? `<dt>His choices</dt><dd>${esc(f.choices.join(", "))}</dd>` : ""}
      ${f.followers ? `<dt>Followers</dt><dd>${esc(f.followers)}</dd>` : ""}
      ${f.nil ? `<dt>NIL a year</dt><dd>${money(f.nil)}</dd>` : ""}
      <dt>Home</dt><dd>${esc([f.high_school, f.hometown].filter(Boolean).join(", ") || "-")}</dd></dl></div>
    ${f.stories.length ? `<div class="panel warm"><h2>What happened</h2><div class="items">${f.stories.map((x) => `<div class="item story"><span>${rich(x)}</span></div>`).join("")}</div></div>` : ""}</div></div>`;
}

// ---------- your department (screens topic 7, scouting topics 2-3, 8, 9, 14) ----------
screens.scouting = async () => {
  const [d, st] = await Promise.all([api("scouting"), S.status ? Promise.resolve(S.status) : api("status")]);
  if (!d.seat) return `<div class="pad">${scHead("Scouting", "Start a career first.")}</div>`;
  const me = st.mine;
  if (me && me.colors) setColors(me.colors);
  const nfl = d.seat === "Nfl";
  const lb = d.lookback || { drafts: [], classes: [] };
  const lbRows = nfl ? lb.drafts : lb.classes;
  const tab = scTabs("sc-dept", [
    ["Staff", "Staff and assignments", d.people.length],
    ["Instructions", "Instructions"],
    ["Work", "Days and asks", d.asks.length || null],
    ["Mine", "My players", d.my_players.length],
    ["Seen", "Most seen", d.most_seen.length],
    ["Back", "Look-back", lbRows.length || null],
    ["Public", nfl ? "Analysts and boards" : "Public services", d.public.desks.length],
  ]);
  const looks = d.people.reduce((n, p) => n + p.looks, 0);
  const evalDays = !nfl ? `${scSquares(d.eval.fall[0], d.eval.fall[1], "fall evaluation days")}${scSquares(d.eval.spring[0], d.eval.spring[1], "spring evaluation days")}` : "";
  const hero = `<div class="sc-room st-room"><i></i><i></i><i></i><div class="sc-room-in">
    <div><span class="st-label">${esc(me ? me.name : "")} · ${nfl ? "college scouting and pro personnel" : "your staff on the road"} · ${d.season}</span><h1 class="st-wide sc-room-h">Scouting</h1>
      <div class="sc-room-facts"><span><b>${d.people.length}</b> people read players for you</span><span><b>${looks}</b> looks this football year</span><span><b>${d.my_players.length}</b> in My players</span>${d.asks.length ? `<span><b>${d.asks.length}</b> asks waiting</span>` : ""}</div></div>
    ${evalDays ? `<div class="sc-room-card st-glass">${evalDays}<small class="sc-dim">One coach out for one day is one day. Only the head coach and 10 assistants go off campus.</small></div>` : `<div class="sc-room-card st-glass"><span class="st-label">Your desk</span><b class="st-cond sc-big">${d.public.draft_year || d.season}</b><span>${d.public.consensus_date ? `Consensus board of ${esc(d.public.consensus_date)}` : "The public's boards come out in the fall"}</span></div>`}
  </div></div>`;
  let body = "";
  if (tab.t === "Staff") {
    const opts = (p) => `<select class="sc-sel" data-act="sc-assign" data-who="${p.code}"><option value="none">No assignment: his own judgment</option>${d.assign_targets.map((t) => `<option value="${esc(t.key)}" ${p.assignment === t.words ? "selected" : ""}>${esc(t.words)}</option>`).join("")}${p.assignment && !d.assign_targets.some((t) => t.words === p.assignment) ? `<option selected>${esc(p.assignment)}</option>` : ""}</select>`;
    const rec = (r) => (r.held + r.high + r.low ? `<span title="From your look-backs">Held up ${r.held} · too high ${r.high} · too low ${r.low}</span>` : '<span class="muted">No record here yet</span>');
    body = table("sc-people", [
      { k: "name", t: "Name", l: true, f: (p) => (p.extra && p.extra.coach != null ? `<a class="link name" data-go="coach" data-id="${p.extra.coach}">${esc(p.name)}</a>` : `<b>${esc(p.name)}</b>`) },
      { k: "job", t: "Job", l: true },
      { k: "area", t: nfl ? "Covers" : "Area", l: true, wrap: true, v: (p) => (p.extra && p.extra.area) || "", f: (p) => esc((p.extra && p.extra.area) || "") || '<span class="muted">-</span>' },
      { k: "looks", t: "Looks this year", div: true },
      { k: "assignment", t: "Assignment", l: true, div: true, v: (p) => p.assignment || "", f: opts },
      { k: "record", t: "Look-backs", l: true, v: (p) => p.record.held - p.record.high - p.record.low, f: (p) => rec(p.record) },
    ], d.people, { empty: "Nobody reads players for you yet." });
  } else if (tab.t === "Instructions") {
    const main = d.instructions.filter((i) => !/^focus/.test(i.key));
    const focus = d.instructions.filter((i) => /^focus/.test(i.key));
    const sw = (i) => `<button class="sc-switch" data-act="sc-instr" data-key="${i.key}" data-on="${i.on ? 0 : 1}" aria-pressed="${i.on}"><i></i><span>${esc(i.words)}</span></button>`;
    body = `<div class="grid2"><div class="panel warm"><h2>Standing instructions <small>plain choices; each man answers by his own lights</small></h2><div class="sc-switches">${main.map(sw).join("")}</div></div>
      <div class="panel warm"><h2>A focus for the year <small>one at a time</small></h2><div class="sc-switches">${focus.map(sw).join("")}</div></div></div>`;
  } else if (tab.t === "Work") {
    body = `<div class="grid2"><div class="panel warm"><h2>Asks on your staff's list <small>${d.asks.length}</small></h2>${d.asks.length ? `<div class="items">${d.asks.map((x) => `<div class="item"><span class="when">${esc(x.asked)}</span><span>${scName(x.id, x.name)} <small class="sub">· ${esc(x.kind)}</small></span></div>`).join("")}</div>` : scEmpty("No asks waiting. Ask for a look from any scouting file.")}</div>
      <div class="panel warm"><h2>Days on the road <small>the last ${d.days.length}</small></h2>${d.days.length ? `<div class="items">${d.days.map((x) => `<div class="item"><span class="when">${esc(x.date)}</span><span><b>${esc(x.who)}</b>${x.player ? ` saw ${scName(x.player.id, x.player.name)}` : ""}</span></div>`).join("")}</div>` : scEmpty(nfl ? "Your scouts' days show here as they take them." : "No evaluation days used yet. The fall period opens in September.")}</div></div>`;
  } else if (tab.t === "Mine") {
    body = scMyPlayers(d.my_players);
  } else if (tab.t === "Seen") {
    body = table("sc-seen", [
      { k: "name", t: "Name", l: true, f: (r) => scName(r.id, r.name) },
      { k: "pos", t: "Pos", l: true },
      { k: "school", t: "Where", l: true, f: scWhere },
      { k: "class", t: "Class", l: true },
      { k: "stars", t: "★", f: (r) => scStars(r.stars) },
      { k: "looks", t: "Looks", div: true },
    ], d.most_seen, { sort: "looks", empty: "Your staff hasn't filed any looks yet." });
  } else if (tab.t === "Back") {
    body = scLookback(lb, nfl);
  } else {
    body = scPublic(d.public, nfl);
  }
  const goto = scGo("scouting");
  return `<div class="film">${hero}<div class="pad"><div class="sc-tabrow">${tab.html}${goto}</div>${body}</div></div>`;
};

function scMyPlayers(rows) {
  return table("sc-mine", [
    { k: "name", t: "Name", l: true, f: (r) => scName(r.id, r.name) },
    { k: "pos", t: "Pos", l: true },
    { k: "where", t: "Where now", l: true, f: scWhere },
    { k: "class", t: "Class", l: true },
    { k: "since", t: "Since", l: true, div: true },
    { k: "why", t: "Why", l: true, wrap: true },
    { k: "first", t: "First read", l: true, div: true },
    { k: "public_then", t: "Public then", l: true },
    { k: "now", t: "Read now", div: true, v: (r) => mid(r.read && r.read.now), f: (r) => scNow(r.read, r.id) },
    { k: "role", t: "Outlook now", l: true, v: (r) => (r.read && r.read.role) || "", f: (r) => scRole(r.read) },
    { k: "x", t: "", f: (r) => `<button class="st-btn small glass" data-act="sc-unfollow" data-id="${r.id}">Stop following</button>` },
  ], rows, { empty: "Nobody yet. Follow a player from his scouting file and he's kept here for the rest of your career." });
}

function scLookback(lb, nfl) {
  const k = keep("sc-lb", { which: nfl ? "drafts" : "classes" });
  const rows = lb[k.which] || [];
  const sum = (k.which === "drafts" ? lb.draft_sum : lb.class_sum) || {};
  const pill = (r) => `<span class="st-pill ${r.v === "held" ? "good" : r.v === "too_high" || r.v === "too_low" ? "gold" : ""}">${esc(r.verdict)}</span>`;
  return `<div class="filters">${scSeg("sc-lb", "which", [["drafts", "Draft classes", lb.drafts.length], ["classes", "Recruiting classes", lb.classes.length]])}<span class="sub">What your staff graded each one then, and what he became three years on. Counts and words: never a scout's rating.</span></div>
    <div class="st-tiles sc-tiles"><div><b>${sum.held || 0}</b><small>held up</small></div><div><b>${sum.high || 0}</b><small>too high</small></div><div><b>${sum.low || 0}</b><small>too low</small></div><div><b>${sum.unfair || 0}</b><small>no fair read</small></div></div>
    ${table("sc-lbt", [
      { k: "year", t: "Year" },
      { k: "name", t: "Name", l: true, f: (r) => playerLink(r.id, r.name) },
      { k: "pos", t: "Pos", l: true },
      { k: "ours", t: "Your rank", div: true },
      { k: "public", t: "Public", f: (r) => r.public ?? '<span class="muted">-</span>' },
      { k: "pick", t: k.which === "drafts" ? "Pick" : "Stars", f: (r) => r.pick ?? '<span class="muted">-</span>' },
      { k: "looks", t: "Looks" },
      { k: "outcome", t: "Rank 3 years on", div: true, f: (r) => r.outcome ?? '<span class="muted">Not yet</span>' },
      { k: "verdict", t: "Verdict", l: true, f: pill },
    ], rows, { empty: "Nothing to judge yet: a class is graded three years on." })}`;
}

function scPublic(p, nfl) {
  const k = keep("sc-pub", { desk: null });
  const desk = p.desks.find((d) => d.k === k.desk) || p.desks[0];
  const prow = (r, i, moves) => `<div class="sc-prow"><b class="st-cond">${i + 1}</b><span>${scName(r.id, r.name)} <small class="sub">${esc(r.pos)} · ${esc(r.school)}</small></span>${moves && moves[i] != null && moves[i] !== 0 ? `<span class="sc-move ${moves[i] > 0 ? "up" : "down"}">${moves[i] > 0 ? "▲" : "▼"}${Math.abs(moves[i])}</span>` : "<span></span>"}</div>`;
  const cards = p.desks.map((d) => `<button class="sc-desk ${desk && desk.k === d.k ? "on" : ""}" data-act="sc-desk" data-k="${d.k}"><span class="st-label">${esc(d.kind)}</span><b>${esc(d.name)}</b><small>${esc(d.outlet)} · since ${d.since}</small><span class="sc-dim">Leans: ${esc(d.lean)}</span><span class="sc-dim">${d.record.drafts ? `${d.record.exact} exact, ${d.record.within3} within three, over ${scPlural(d.record.drafts, "draft")}` : "No track record yet"}</span></button>`).join("");
  return `<div class="sc-desks">${cards || scEmpty("No public desks are active this year.")}</div>
    <div class="grid3">
      <div class="panel warm"><h2>Consensus board <small>${esc(p.consensus_date || "")}</small></h2>${p.consensus.length ? `<div class="sc-plist">${p.consensus.slice(0, 32).map((r, i) => prow(r, i, p.moves)).join("")}</div>` : scEmpty("No consensus board yet.")}</div>
      <div class="panel warm"><h2>${desk ? esc(desk.name) + "'s board" : "A desk's board"} <small>${desk && desk.updated ? esc(desk.updated) : ""}</small></h2>${desk && desk.board.length ? `<div class="sc-plist">${desk.board.map((r, i) => prow(r, i)).join("")}</div>` : scEmpty("No board from this desk yet.")}</div>
      <div class="panel warm"><h2>${desk ? esc(desk.name) + "'s mock" : "Mock draft"}</h2>${desk && desk.mock.length ? `<div class="sc-plist">${desk.mock.map((r, i) => prow(r, i)).join("")}</div>` : scEmpty(nfl ? "No mock from this desk yet." : "Recruiting services don't mock drafts.")}
      ${p.scored.length ? `<span class="st-label">Last year's mocks, scored</span>${p.scored.map((s) => `<div class="sc-prow"><span></span><span><b>${esc(s.name)}</b> <small class="sub">${esc(s.outlet)}</small></span><span class="sc-dim">${s.exact}/${s.of} exact · ${s.within3} within 3</span></div>`).join("")}` : ""}</div>
    </div>`;
}

// ---------- the draft board (screens topic 7, scouting topics 9-10) ----------
const SC_LISTS = ["Top-30 visits", "Combine interviews", "Private workouts", "Medical rechecks"];
const SC_LIST_SHORT = ["V", "I", "W", "M"];
function scTierClass(t) { return `t${Math.min(8, Math.max(1, t || 8))}`; }

screens.draftboard = async () => {
  const x = await api("draft_board");
  setColors(x.team.colors);
  SC.live = x.live;
  const k = keep("sc-board", { group: "All", sel: null, hideTaken: false });
  const tab = scTabs("sc-boardtab", [["Vertical", "The board", x.rows.length], ["Horizontal", "By position"], ["Lists", "Lists"], ["Public", "Public boards"]]);
  const rows = x.rows.filter((r) => (k.group === "All" || r.group === k.group) && !(k.hideTaken && r.taken));
  const selRow = x.rows.find((r) => r.id === k.sel) || rows.find((r) => !r.taken) || x.rows[0];
  const tiers = x.tiers.map(scTier);
  const picks = x.picks.map((p) => `<div class="sc-pick" title="${p.from ? "From " + esc(p.from) : "Your own"}"><b class="st-cond">${p.pick ? p.pick : "R" + p.round}</b><span>${p.pick ? "Round " + p.round : p.from ? "via " + esc(p.from) : "your own"}</span></div>`).join("");
  const hero = `<div class="sc-room st-room"><i></i><i></i><i></i><div class="sc-room-in">
    <div><span class="st-label">${x.year} NFL Draft · ${esc(x.team.name)}${x.director ? ` · kept by ${esc(x.director)}, director of college scouting` : ""}</span>
      <div class="sc-room-title"><h1 class="st-wide sc-room-h">The board</h1>${x.live ? '<span class="st-pill live">The draft is live</span>' : ""}</div>
      <div class="sc-room-facts"><span><b>${x.rows.length}</b> graded</span><span><b>${x.rows.filter((r) => r.dnd).length}</b> do not draft</span><span><b>${x.rows.filter((r) => r.pfa).length}</b> free-agent targets</span><span>Undrafted pool <b>${money(x.pfa_pool)}</b></span></div>
      ${x.live ? '<div class="st-actions" style="margin-top:12px"><button class="st-btn primary" data-go="warroom">Enter the war room</button></div>' : ""}</div>
    <div class="sc-room-card st-glass"><span class="st-label">Your picks · ${x.picks.length}</span><div class="sc-picks">${picks || '<span class="sc-dim">No picks this year.</span>'}</div></div>
  </div></div>`;
  let main = "";
  const groups = ["All", ...x.groups];
  const filt = `<div class="filters">${scSeg("sc-board", "group", groups.map((g) => [g, g === "All" ? "Every position" : g, g === "All" ? null : x.rows.filter((r) => r.group === g).length]))}<div class="seg"><button data-act="sc-seg" data-k="sc-board" data-f="hideTaken" data-v="${!k.hideTaken}" aria-pressed="${k.hideTaken}">Hide players taken</button></div></div>`;
  if (tab.t === "Vertical") {
    let last = null;
    const body = rows.map((r) => {
      let g = "";
      if (r.tier !== last) {
        const [n, sub] = tiers[r.tier - 1] || ["Tier " + r.tier, ""];
        g = `<tr class="grouprow sc-tierrow ${scTierClass(r.tier)}"><td class="l" colspan="9"><b class="st-wide">${esc(n)}</b> <span>${esc(sub)}</span></td></tr>`;
        last = r.tier;
      }
      const marks = (r.lists || []).map((l) => `<span class="sc-mk" title="${SC_LISTS[l]}">${SC_LIST_SHORT[l]}</span>`).join("") + (r.pfa ? '<span class="sc-mk pfa" title="Priority free agent">PFA</span>' : "");
      return `${g}<tr class="${r.taken ? "sc-gone" : ""} ${r.dnd ? "sc-dnd" : ""} ${selRow && selRow.id === r.id ? "sc-on" : ""}" data-sc-sel="${r.id}">
        <td><b class="st-cond sc-rank">${r.rank}</b></td>
        <td class="l">${scName(r.id, r.name)}<small class="sc-sub">${esc(r.pos)} · ${esc(r.school)} · ${esc(r.class)}</small></td>
        <td class="l div">${esc(r.grade_words)}${r.dnd ? ' <span class="st-pill live" title="' + esc(r.dnd) + '">DND</span>' : ""}</td>
        <td class="div">${r.public ?? '<span class="muted">-</span>'}${r.limb ? ` <span class="sc-limb ${r.limb > 0 ? "up" : "down"}" title="${r.limb > 0 ? "Out on a limb: you have him far higher than the public" : "Out on a limb: you have him far lower than the public"}">LIMB${r.limb > 0 ? "↑" : "↓"}</span>` : ""}</td>
        <td>${r.mock ?? '<span class="muted">-</span>'}</td>
        <td class="l">${marks}</td>
        <td class="l">${r.taken ? '<span class="muted">Taken</span>' : r.choices && r.choices.length ? `<small class="muted">${esc(r.choices.join(", "))}</small>` : ""}</td>
        <td class="sc-move-c"><button class="sc-mini" data-act="sc-move" data-id="${r.id}" data-to="${Math.max(1, r.rank - 1)}" title="Move up one" ${r.rank === 1 ? "disabled" : ""}>▲</button><button class="sc-mini" data-act="sc-move" data-id="${r.id}" data-to="${r.rank + 1}" title="Move down one">▼</button></td></tr>`;
    }).join("");
    main = `${filt}<div class="tbl warm box sc-boardtbl"><table><thead><tr><th>#</th><th class="l">Player</th><th class="l div">Your grade</th><th class="div" title="The public's consensus rank; LIMB: far from yours">Public</th><th title="The best mock's pick">Mock</th><th class="l" title="V visit · I interview · W workout · M medical recheck">Lists</th><th class="l">Status</th><th>Move</th></tr></thead><tbody>${body || `<tr><td class="l" colspan="9">${scEmpty("Nobody at this position on your board.")}</td></tr>`}</tbody></table></div>`;
  } else if (tab.t === "Horizontal") {
    const cols = x.groups.map((g) => {
      const list = x.rows.filter((r) => r.group === g && !(k.hideTaken && r.taken)).slice(0, 30);
      return `<div class="sc-hcol"><div class="sc-hhead"><b class="st-wide">${esc(g)}</b><span class="st-cond">${x.rows.filter((r) => r.group === g).length}</span><small>${list.filter((r) => r.tier <= 3).length} with a day-one or two grade</small></div>${list.map((r) => `<button class="sc-hcard ${scTierClass(r.tier)} ${r.taken ? "gone" : ""}" data-act="sc-pick" data-id="${r.id}" title="${esc(r.name)} · ${esc(r.grade_words)}"><b>${esc(r.name.split(" ").slice(-1)[0])}</b><span>${esc(r.pos)} · ${r.rank}${r.public ? " · pub " + r.public : ""}</span></button>`).join("") || '<span class="muted">-</span>'}</div>`;
    }).join("");
    main = `<div class="filters"><span class="sub">Coloured by your round grade:</span>${tiers.map((t, i) => `<span class="sc-band ${scTierClass(i + 1)}"><i></i>${esc(t[1] || t[0])}</span>`).join("")}</div><div class="sc-hboard">${cols}</div>`;
  } else if (tab.t === "Lists") {
    main = scBoardLists(x);
  } else {
    main = scPublic(await api("public_boards"), true);
  }
  const side = tab.t === "Public" ? "" : await scBoardSide(selRow, x);
  const goto = scGoto([x.live ? ["warroom", "War room"] : null, ["predraft", "Pre-draft"], ["scouting", "Department"], ["contracts", "Contracts"], ["roster", "Squad"]]);
  return `<div class="film">${hero}<div class="pad"><div class="sc-tabrow">${tab.html}${goto}</div>${side ? `<div class="sc-boardgrid"><div class="sc-boardmain">${main}</div>${side}</div>` : main}</div></div>`;
};

async function scBoardSide(r, x) {
  if (!r) return `<aside class="panel warm sc-side">${scEmpty("Pick a player on the board.")}</aside>`;
  let f = null;
  try { f = await api("scout_file", { id: r.id }); } catch (e) { /* gone */ }
  const s = f && f.summary;
  const keyReads = f ? f.groups.flatMap((g) => g.attrs).filter((a) => a.matters && a.read).slice(0, 7) : [];
  const tiers = x.tiers.map(scTier);
  const lists = SC_LISTS.map((n, i) => {
    const on = (r.lists || []).includes(i);
    const cap = x.lists && x.lists[i];
    return `<button class="sc-switch small" data-act="sc-list" data-id="${r.id}" data-list="${i}" data-on="${on ? 0 : 1}" aria-pressed="${on}"><i></i><span>${esc(n)}${cap ? ` <small class="muted">${cap.n}/${cap.cap}</small>` : ""}</span></button>`;
  }).join("");
  return `<aside class="sc-side st-glass">
    <div class="sc-side-h"><div class="sc-pills"><span class="st-pill">${esc(r.pos)}</span><span class="st-pill">Tier ${r.tier}</span><span class="st-pill">No. ${r.rank}</span>${r.public ? `<span class="st-pill">Public ${r.public}</span>` : ""}</div>
      <h2>${scName(r.id, r.name)}</h2><div class="sub">${scWhere(r)} · ${esc(r.class)} · ${esc(r.ht)}, ${r.wt}</div></div>
    <div class="sc-side-g"><span class="st-label">Your grade</span><b>${esc(r.grade_words)}</b>${s ? `<span>${esc(s.role)} · staff ${esc(s.sure)}</span>` : '<span class="sc-dim">Not seen by your staff: graded off the public board</span>'}</div>
    ${s ? `<div class="sc-side-sec"><span class="st-label">Key reads · 1-99, his level now</span>${keyReads.map((a) => `<div class="attr"><span>${esc(a.label)}</span>${range(a.read, null, { p: r.id, k: a.key })}</div>`).join("") || '<span class="sc-dim">No key attribute read yet.</span>'}</div>
      <div class="sc-side-sec"><span class="st-label">Medical and character</span><div>${scMed(s.medical)}</div>${f.character.filter((c) => c.note).slice(0, 2).map((c) => `<p class="sc-q">“${esc(c.note)}”<small>${esc(scCap(c.source))}</small></p>`).join("")}</div>
      ${s.hints.length ? `<div class="sc-side-sec"><span class="st-label">Hints</span>${s.hints.slice(0, 2).map((h) => `<p class="sc-q">${esc(h.text)}${h.by ? `<small>${esc(h.by)}</small>` : ""}</p>`).join("")}</div>` : ""}
      ${f.voices.length ? `<div class="sc-side-sec"><span class="st-label">The meeting</span>${f.voices.slice(0, 3).map((v) => `<p class="sc-q">“${esc(v.says)}”<small>${esc(v.who)}, ${esc(v.job)}</small></p>`).join("")}<div class="sc-final"><span class="st-label">Final say</span><b>You, head coach</b></div></div>` : ""}` : ""}
    <div class="sc-side-sec"><span class="st-label">Your call</span>
      <div class="sc-row2"><select class="sc-sel" data-act="sc-tier" data-id="${r.id}">${tiers.map((t, i) => `<option value="${i + 1}" ${i + 1 === r.tier ? "selected" : ""}>${esc(t[0])} · ${esc(t[1])}</option>`).join("")}</select>
      <button class="st-btn small glass" data-act="sc-moveto" data-id="${r.id}" data-rank="${r.rank}">Move to…</button></div>
      <div class="sc-switches">${lists}
        <button class="sc-switch small" data-act="sc-dnd" data-id="${r.id}" data-on="${r.dnd ? 0 : 1}" aria-pressed="${!!r.dnd}"><i></i><span>Do not draft${r.dnd ? ` <small class="muted">${esc(r.dnd)}</small>` : ""}</span></button>
        <button class="sc-switch small" data-act="sc-pfa" data-id="${r.id}" data-on="${r.pfa ? 0 : 1}" aria-pressed="${r.pfa}"><i></i><span>Priority free agent if he goes undrafted</span></button></div>
      <button class="st-btn small primary" data-go="scout" data-id="${r.id}">Open his full file</button></div>
  </aside>`;
}

function scBoardLists(x) {
  const k = keep("sc-lists", { l: "dnd" });
  const tabs = [["dnd", "Do not draft", x.rows.filter((r) => r.dnd).length], ["pfa", "Priority free agents", x.rows.filter((r) => r.pfa).length], ...SC_LISTS.map((n, i) => [String(i), n, x.lists[i] ? `${x.lists[i].n}/${x.lists[i].cap}` : null])];
  const chips = `<div class="st-chips">${tabs.map(([v, t, n]) => `<button data-act="sc-seg" data-k="sc-lists" data-f="l" data-v="${v}" aria-pressed="${k.l === v}">${esc(t)}${n != null ? ` · ${n}` : ""}</button>`).join("")}</div>`;
  let rows, cols, sub;
  const base = [
    { k: "rank", t: "#" },
    { k: "name", t: "Player", l: true, f: (r) => `${scName(r.id, r.name)}<small class="sc-sub">${esc(r.pos)} · ${esc(r.school)}</small>` },
    { k: "grade_words", t: "Your grade", l: true, div: true },
    { k: "public", t: "Public", f: (r) => r.public ?? '<span class="muted">-</span>' },
  ];
  if (k.l === "dnd") {
    rows = x.rows.filter((r) => r.dnd);
    sub = "Off your board, each in the words of whoever took him off. Your staff won't pick him; you still can.";
    cols = [...base, { k: "dnd", t: "Why", l: true, wrap: true, div: true, f: (r) => `“${esc(r.dnd)}”` }, { k: "x", t: "", f: (r) => `<button class="st-btn small glass" data-act="sc-dnd" data-id="${r.id}" data-on="0">Put him back</button>` }];
  } else if (k.l === "pfa") {
    rows = x.rows.filter((r) => r.pfa);
    sub = `The ones you call first when the draft ends. Your undrafted pool this year: ${money(x.pfa_pool)}.`;
    cols = [...base, { k: "x", t: "", f: (r) => `<button class="st-btn small glass" data-act="sc-pfa" data-id="${r.id}" data-on="0">Take him off</button>` }];
  } else {
    const i = Number(k.l);
    rows = x.rows.filter((r) => (r.lists || []).includes(i));
    sub = `${SC_LISTS[i]}: ${x.lists[i] ? `${x.lists[i].n} of ${x.lists[i].cap} used` : ""}. Add players from the board's side panel.`;
    cols = [...base, { k: "x", t: "", f: (r) => `<button class="st-btn small glass" data-act="sc-list" data-id="${r.id}" data-list="${i}" data-on="0">Take him off</button>` }];
  }
  return `${chips}<p class="sub">${esc(sub)}</p>${table("sc-list-" + k.l, cols, rows, { empty: "Nobody on this list." })}`;
}

// ---------- the pre-draft season (scouting topic 10) ----------
screens.predraft = async () => {
  const x = await api("predraft");
  setColors(x.board.team.colors);
  const top = x.board.rows.filter((r) => !r.taken).slice(0, 80);
  // Each list adds a player from your board here (the board's side panel does too).
  const addRow = (l) => {
    if (l.rows.length >= l.cap) return `<p class="sub">The list is full: take someone off to add another.</p>`;
    const pool = x.board.rows.filter((r) => !r.taken && !(r.lists || []).includes(l.k)).slice(0, 150);
    return pool.length ? `<div class="sc-row2"><select class="sc-sel" data-sc-add="${l.k}" aria-label="Add to ${esc(l.name.toLowerCase())}">${pool.map((r) => `<option value="${r.id}">${r.rank}. ${esc(r.name)} · ${esc(r.pos)} · ${esc(r.school)}</option>`).join("")}</select><button class="st-btn small glass" data-act="sc-listadd" data-list="${l.k}">Add</button></div>` : "";
  };
  const listPanel = (l) => `<div class="panel warm"><h2>${esc(l.name)} <small>${l.rows.length} of ${l.cap}</small></h2>${l.rows.length ? `<div class="items">${l.rows.map((r) => `<div class="item sc-li"><span class="when">${esc(r.pos)}</span><span>${scName(r.id, r.name)} <small class="sub">· ${esc(r.school)}</small>${r.declined ? ' <span class="st-pill gold">Declined your visit</span>' : ""}</span><button class="sc-mini" data-act="sc-list" data-id="${r.id}" data-list="${l.k}" data-on="0" title="Take him off">✕</button></div>`).join("")}</div>` : scEmpty("Nobody yet. Pick a player from your board below.")}${addRow(l)}</div>`;
  return `<div class="film"><div class="pad">${scHead("The pre-draft season", "Visits, interviews, workouts and rechecks; who's staying in school; and the smoke every club blows before the draft.", scGoto([x.board.live ? ["warroom", "War room"] : null, ["draftboard", "Draft board"], ["scouting", "Department"], ["calendar", "Calendar"]]))}
    <div class="sc-cal">${x.calendar.map(([t, d]) => `<div class="st-glass"><span class="st-label">${esc(t)}</span><b>${esc(d)}</b></div>`).join("")}</div>
    <div class="grid2"><div class="sc-lists4">${x.lists.map(listPanel).join("")}</div>
    <div style="display:grid;gap:18px"><div class="panel warm"><h2>Plant a story <small>the media listens to clubs</small></h2>
      <p class="sub">Leak real interest to scare a club off, or a smokescreen to send them the wrong way. Other clubs read your habits.</p>
      <div class="sc-row2"><select class="sc-sel" id="sc-leak">${top.map((r) => `<option value="${r.id}">${r.rank}. ${esc(r.name)} · ${esc(r.pos)}</option>`).join("")}</select></div>
      <div class="sc-row2"><button class="st-btn small glass" data-act="sc-leak" data-real="1">Leak real interest</button><button class="st-btn small glass" data-act="sc-leak" data-real="0">Plant a smokescreen</button></div></div>
    <div class="panel warm"><h2>What's out there <small>${x.leaks.length}</small></h2>${x.leaks.length ? `<div class="items">${x.leaks.map((l) => `<div class="item"><span class="when">${esc(l.date)}</span><span>${teamLink(l.team)} on ${scName(l.player.id, l.player.name)} <small class="sub">· ${esc(l.player.pos)}</small> <span class="st-pill ${l.yours ? "gold" : /real/.test(l.read) ? "good" : ""}">${esc(l.read)}</span></span></div>`).join("")}</div>` : scEmpty("No leaks yet. They pick up in the last weeks.")}</div>
    <div class="panel warm"><h2>Staying in school or declared</h2>${x.choices.length ? `<div class="items">${x.choices.map((r) => `<div class="item"><span class="when">${esc(r.pos)}</span><span>${scName(r.id, r.name)} <small class="sub">· ${esc(r.school)}</small> · ${esc(r.choices.join(", "))}</span></div>`).join("")}</div>` : scEmpty("No choices made yet.")}</div>
    ${x.refuses.length ? `<div class="panel warm"><h2>Won't play for</h2><div class="items">${x.refuses.map((r) => `<div class="item"><span class="when">${esc(r.player.pos)}</span><span>${scName(r.player.id, r.player.name)} won't play for ${teamLink(r.team)}</span></div>`).join("")}</div></div>` : ""}</div></div>
  </div></div>`;
};

// ---------- the war room (screens topic 9, scouting topics 11-13) ----------
screens.warroom = async () => {
  const w = await api("draft_live");
  SC.live = !!w.live;
  if (S.status) S.status.draft_live = SC.live;
  title("War room");
  if (!w.live) {
    if (w.record) return screens.draftrecord({ record: w.record });
    return `<div class="pad">${scHead("The war room", "It opens on draft night, when your club is on the clock or the first round starts.", scGo("predraft"))}</div>`;
  }
  const otc = w.on_the_clock || w.team;
  setColors((otc && otc.colors) || w.team.colors);
  // The clock: real minutes, cosmetic; you can speed it up or pause it.
  if (!SC.clock || SC.clock.pick !== w.next) SC.clock = { pick: w.next, left: w.clock_minutes * 60, speed: SC.clock ? SC.clock.speed : 1, paused: false };
  const k = keep("sc-war", { group: "All" });
  const tiers = (w.tiers || []).map(scTier);
  let last = null;
  const brows = w.board.filter((r) => k.group === "All" || r.pos === k.group);
  const board = brows.map((r) => {
    let g = "";
    if (r.tier !== last) {
      g = `<tr class="grouprow sc-tierrow ${scTierClass(r.tier)}"><td class="l" colspan="6"><b class="st-wide">${esc((tiers[r.tier - 1] || [])[0] || "Tier " + r.tier)}</b> <span>${esc((tiers[r.tier - 1] || [])[1] || "")} · ${w.tier_left[r.tier - 1] || 0} left</span></td></tr>`;
      last = r.tier;
    }
    return `${g}<tr class="${r.dnd ? "sc-dnd" : ""}"><td><b class="st-cond sc-rank">${r.rank}</b></td>
      <td class="l">${scName(r.id, r.name)}<small class="sc-sub">${esc(r.pos)} · ${esc(r.school)}</small></td>
      <td>${r.public ?? '<span class="muted">-</span>'}${r.limb ? ` <span class="sc-limb ${r.limb > 0 ? "up" : "down"}">LIMB${r.limb > 0 ? "↑" : "↓"}</span>` : ""}</td>
      <td title="Your staff's guess at the last pick he lasts to">${r.last_shot ? `<b class="st-cond">${r.last_shot}</b>` : '<span class="muted">-</span>'}</td>
      <td>${w.on_clock ? `<button class="st-btn small primary sc-draftbtn" data-act="sc-draft" data-id="${r.id}" data-name="${esc(r.name)}">Draft</button>` : r.dnd ? '<span class="st-pill live">DND</span>' : ""}</td></tr>`;
  }).join("");
  const posList = [...new Set(w.board.map((r) => r.pos))].sort();
  const feed = w.feed.slice(0, 40).map((p) => `<li class="${p.mine ? "now" : ""}"><span class="n">${p.pick}</span>${mark(p.team, 30)}<span>${p.player ? `<b>${scName(p.player.id, p.player.name)}</b><small>${esc(p.player.pos)} · ${esc(p.player.school)} · ${teamLink(p.team)}</small>` : `<b>${teamLink(p.team)}</b><small>${esc(p.text)}</small>`}</span><small class="sc-feedr">R${p.round}</small></li>`).join("");
  const upcoming = w.upcoming.map((u, i) => `<li class="${i === 0 ? "now" : u.mine ? "" : "future"} ${u.mine ? "sc-mine" : ""}"><span class="n">${u.pick}</span>${mark(u.team, 26)}<span><b>${teamLink(u.team)}</b><small>Round ${u.round}${u.mine ? " · your pick" : ""}</small></span>${!u.mine && i > 0 ? `<button class="sc-mini wide" data-act="sc-quote" data-slot="${u.pick - 1}" title="Ask your GM what moving up to ${u.pick} costs">Price</button>` : "<span></span>"}</li>`).join("");
  const offers = w.offers.length
    ? w.offers.map((o) => `<div class="sc-offer st-glass"><div class="sc-offer-h">${mark(o.from, 34)}<div><b>${teamLink(o.from)} is calling</b><small>for your pick ${w.next}</small></div></div>
      <div class="st-ledger"><div><span class="st-label">You get</span>${o.gets.map((g) => `<div class="item"><span>${esc(scCap(g))}</span></div>`).join("")}</div><div><span class="st-label">They want</span><div class="item"><span>Pick ${w.next}</span></div>${o.after ? `<small class="sc-dim">They're after ${scName(o.after.id, o.after.name)}, ${esc(o.after.pos)}</small>` : ""}</div></div>
      <button class="st-btn small primary" data-act="sc-accept" data-k="${o.k}">Accept the trade</button></div>`).join("")
    : `<div class="sc-dim sc-pad">${w.on_clock ? "Nobody's calling about your pick yet." : "Calls come when you're on the clock."}</div>`;
  const quote = SC.quote && SC.quote.next === w.next
    ? SC.quote.offer
      ? `<div class="sc-offer st-glass"><div class="sc-offer-h">${mark(SC.quote.offer.from, 30)}<div><b>Up to pick ${SC.quote.slot + 1}</b><small>${teamLink(SC.quote.offer.from)} would deal</small></div></div><div class="st-ledger"><div><span class="st-label">You get</span>${SC.quote.offer.gets.slice(0, 1).map((g) => `<div class="item"><span>${esc(scCap(g))}</span></div>`).join("")}</div><div><span class="st-label">You give</span>${SC.quote.offer.gets.slice(1).map((g) => `<div class="item"><span>${esc(scCap(g))}</span></div>`).join("") || '<div class="item"><span>Your next pick and more</span></div>'}</div></div><button class="st-btn small primary" data-act="sc-tradeup" data-slot="${SC.quote.slot}">Make the trade</button></div>`
      : `<div class="sc-dim sc-pad">Nobody will move pick ${SC.quote.slot + 1} for what you have.</div>`
    : "";
  const voices = w.voices.map((v) => `<p class="sc-q">“${esc(v.says)}”<small>${esc(v.who)}</small></p>`).join("");
  const mine = w.my_slots.map((s) => `<span class="sc-slot ${s.used ? "used" : ""} ${s.pick === w.next ? "now" : ""}"><b class="st-cond">${s.pick}</b><small>R${s.round}</small></span>`).join("");
  const done = w.rush;
  const nowCard = done
    ? `<div class="st-glass sc-now"><div class="st-now-who"><span class="st-mark" style="--s:64px;background:var(--good)">✓</span><div><span class="st-label">${w.year} NFL Draft</span><h1>Every pick is in</h1><p>${w.picks_made} players taken. The phones start now: the undrafted rush.</p></div></div><div class="sc-nowact"><button class="st-btn primary" data-act="sc-rush">Open the undrafted rush</button></div></div>`
    : `<div class="st-glass sc-now ${w.on_clock ? "mine" : ""}"><div class="st-now-who">${mark(otc, 68)}<div><span class="st-label">On the clock · pick ${w.next} · round ${w.round}</span><h1>${w.on_clock ? "You're on the clock" : esc(otc ? otc.name : "")}</h1><p>${w.on_clock ? (w.staff_pick ? `Your staff would take ${scName(w.staff_pick.id, w.staff_pick.name)}, ${esc(w.staff_pick.pos)}, ${esc(w.staff_pick.school)}.` : "Your staff is still talking.") : `${w.picks_made} of ${w.slots} picks made.`}</p></div></div>
      <div class="st-timer"><b id="sc-clock">${scClockText()}</b><div class="st-bar"><i id="sc-clockbar" style="width:100%"></i></div>
        <div class="sc-clockctl"><button class="sc-mini wide" data-act="sc-clock" data-v="pause">${SC.clock.paused ? "Run" : "Pause"}</button><button class="sc-mini wide ${SC.clock.speed === 10 ? "on" : ""}" data-act="sc-clock" data-v="10">×10</button><button class="sc-mini wide ${SC.clock.speed === 60 ? "on" : ""}" data-act="sc-clock" data-v="60">×60</button></div></div></div>`;
  const actions = done
    ? ""
    : w.on_clock
      ? `<div class="sc-acts"><button class="st-btn glass" data-act="sc-auto">Let the staff pick${w.staff_pick ? ": " + esc(w.staff_pick.name) : ""}</button><span class="sc-timeup" id="sc-timeup" hidden><button class="st-btn glass" data-act="sc-timeout">Let the clock run out</button></span><span class="sc-final"><span class="st-label">Final say</span><b>You, head coach</b></span></div>`
      : `<div class="sc-acts"><button class="st-btn primary" data-act="sc-go" data-until="yours">Sim to my pick</button><button class="st-btn glass" data-act="sc-go" data-until="next">Next pick</button><button class="st-btn glass" data-act="sc-go" data-until="round">Finish the round</button></div>`;
  const reveal = SC.reveal && w.revealed && SC.reveal === `${w.revealed.pick}-${w.revealed.id}`
    ? `<div class="st-moment fixed sc-reveal" data-act="sc-unreveal"><div class="st-reveal"><div class="st-kicker">The pick is in</div><div class="st-outline">${w.revealed.pick}</div><div class="st-reveal-sub">${esc(w.revealed.pos)} · ${esc(w.revealed.school)}</div><h2>${esc(w.revealed.name)}</h2>${w.revealed.take ? `<div class="st-verdict"><span><b style="font-size:13px;font-stretch:100%;color:#fff">${esc(w.revealed.take.who)}, ${esc(w.revealed.take.outlet)}</b><br>“${esc(w.revealed.take.says)}”</span></div>` : ""}<button class="st-btn primary" data-act="sc-unreveal">Back to the board</button></div></div>`
    : "";
  return `<div class="sc-war-room st-room"><i></i><i></i><i></i><div class="sc-war-in">
    <div class="sc-war-top"><div><span class="st-label">${esc(w.team.name)} war room · ${w.year} NFL Draft</span><div class="sc-mine-slots">${mine}</div></div>${scGo("warroom")}</div>
    ${nowCard}${actions}
    <div class="sc-war">
      <div class="st-glass sc-wcol"><div class="sc-wcol-h"><h2>Your board <small>${w.board.length} left on it</small></h2><select class="sc-sel" data-act="sc-wargroup"><option value="All">Every position</option>${posList.map((p) => `<option ${k.group === p ? "selected" : ""}>${esc(p)}</option>`).join("")}</select></div>
        <div class="tbl sc-wtbl"><table><thead><tr><th>#</th><th class="l">Player</th><th title="Public rank">Pub</th><th title="Last shot: the last pick he lasts to, by your staff's guess">Last shot</th><th></th></tr></thead><tbody>${board || `<tr><td class="l" colspan="5">${scEmpty("Nobody left on your board here.")}</td></tr>`}</tbody></table></div></div>
      <div class="st-glass sc-wcol"><h2>The picks <small>newest first</small></h2><ul class="st-feed sc-feed">${feed || '<li><span></span><span></span><span class="sc-dim">No picks yet.</span></li>'}</ul></div>
      <div class="sc-wright"><div class="st-glass sc-wcol"><h2>Trade calls <small>${w.offers.length}</small></h2>${offers}${quote}</div>
        <div class="st-glass sc-wcol"><h2>Up next</h2><ul class="st-feed sc-feed">${upcoming}</ul></div>
        ${voices ? `<div class="st-glass sc-wcol"><h2>The room</h2>${voices}</div>` : ""}</div>
    </div></div></div>${reveal}`;
};
function scClockText() {
  const s = Math.max(0, Math.round(SC.clock ? SC.clock.left : 0));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}
// The clock ticks only while the war room is on screen; it never picks for you.
setInterval(() => {
  const el = document.getElementById("sc-clock");
  if (!el || !SC.clock || SC.clock.paused) return;
  SC.clock.left = Math.max(0, SC.clock.left - SC.clock.speed);
  el.textContent = scClockText();
  const bar = document.getElementById("sc-clockbar");
  if (bar && SC.clock.total == null) SC.clock.total = SC.clock.left + SC.clock.speed;
  if (bar) bar.style.width = `${(SC.clock.left / (SC.clock.total || 1)) * 100}%`;
  if (SC.clock.left <= 0) { el.classList.add("out"); const t = document.getElementById("sc-timeup"); if (t) t.hidden = false; }
}, 1000);

// ---------- the draft record and the undrafted rush ----------
screens.draftrecord = async (a) => {
  const r = (a && a.record) || (await api("draft_record"));
  if (!r) return `<div class="pad">${scHead("Draft record", "No draft from your chair yet.", scGoto([["draftboard", "Draft board"]]))}</div>`;
  setColors(r.team.colors);
  const k = keep("sc-rec", { t: "Class" });
  const tab = scTabs("sc-rec", [["Class", "Your class", r.mine.length], ["Rush", "The undrafted rush", r.rush.length], ["Camp", "Minicamp tryouts", r.minicamp.length], ["All", "Every pick", r.all.length]]);
  const hero = `<div class="sc-room st-room"><i></i><i></i><i></i><div class="sc-room-in"><div><span class="st-label">${r.year} NFL Draft · ${esc(r.team.name)}</span><h1 class="st-wide sc-room-h">Your class</h1>
    <div class="sc-room-facts"><span><b>${r.mine.length}</b> picks</span><span><b>${r.rush.filter((x) => x.mine).length}</b> signed in the rush</span><span><b>${r.trades.length}</b> trades on the day</span></div></div>
    <div class="sc-room-card st-glass"><span class="st-label">The analysts' grade</span><b class="sc-grade-big st-cond">${esc(r.grade || "-")}</b><span class="sc-dim">Picks against where the public had them</span></div></div></div>`;
  let body;
  if (tab.t === "Class") {
    body = `<div class="sc-class">${r.mine.map((p) => `<div class="sc-classcard st-glass"><div class="st-outline">${p.pick}</div><span class="st-label">Round ${p.round} · pick ${p.pick}</span><b>${playerLink(p.id, p.name)}</b><span>${esc(p.pos)} · ${esc(p.school)}</span>${p.why ? `<p class="sc-dim">${esc(p.why)}</p>` : ""}<a class="link sc-dim" data-go="scout" data-id="${p.id}">Scouting file</a></div>`).join("") || scEmpty("You made no picks.")}</div>
      ${r.trades.length ? `<div class="panel warm"><h2>Trades on the day</h2><div class="items">${r.trades.map((t) => `<div class="item story"><span>${rich(typeof t === "string" ? t : t.text || "")}</span></div>`).join("")}</div></div>` : ""}`;
  } else if (tab.t === "Rush") {
    body = table("sc-rush", [
      { k: "name", t: "Name", l: true, f: (x) => playerLink(x.id, x.name) },
      { k: "team", t: "Signed with", l: true, v: (x) => x.team.name, f: (x) => `<span class="teamcell">${mark(x.team, 20)} ${teamLink(x.team)}</span>` },
      { k: "bonus", t: "Bonus", div: true, f: (x) => money(x.bonus) },
      { k: "called", t: "Clubs that called" },
    ], r.rush, { sort: "bonus", empty: "The rush hasn't opened yet." }).replace(/<tr>(?=(?:(?!<tr>).)*?data-sc-mine)/g, "<tr>");
  } else if (tab.t === "Camp") {
    body = table("sc-camp", [
      { k: "name", t: "Name", l: true, f: (x) => playerLink(x.id, x.name) },
      { k: "pos", t: "Pos", l: true },
      { k: "clubs", t: "Tried out with", l: true, wrap: true, f: (x) => esc(x.clubs.join(", ")) },
      { k: "signed", t: "Signed", l: true, f: (x) => (x.signed ? `<b>${esc(x.signed)}</b>` : '<span class="muted">Not yet</span>') },
    ], r.minicamp, { empty: "No tryouts yet: rookie minicamps come in May." });
  } else {
    body = table("sc-all", [
      { k: "pick", t: "Pick", v: (x) => -x.pick },
      { k: "round", t: "Rd" },
      { k: "team", t: "Club", l: true, v: (x) => x.team.name, f: (x) => `<span class="teamcell">${mark(x.team, 20)} ${teamLink(x.team)}</span>` },
      { k: "name", t: "Player", l: true, div: true, f: (x) => `${playerLink(x.id, x.name)}${x.mine ? ' <span class="st-pill gold">Yours</span>' : ""}` },
      { k: "pos", t: "Pos", l: true },
      { k: "school", t: "School", l: true },
    ], r.all, { sort: "pick" });
  }
  void k;
  return `<div class="film">${hero}<div class="pad"><div class="sc-tabrow">${tab.html}${scGo("draftrecord")}</div>${body}</div></div>`;
};

// ---------- pro scouting (scouting topic 13) ----------
screens.proscout = async () => {
  const k = keep("sc-pro", { club: null, t: "Street" });
  const x = await api("pro_scouting", k.club != null ? { club: k.club } : {});
  if (S.status && S.status.mine) setColors(S.status.mine.colors);
  const tab = scTabs("sc-protab", [["Street", "The street", x.street.length], ["Club", "A club's roster"], ["Tryouts", "Tryouts", x.tryouts.length], ...(x.supp_open ? [["Supp", "Supplemental draft", x.supp.length]] : []), ["Staff", "Pro staff", x.scouts.length]]);
  const readCols = [
    { k: "name", t: "Name", l: true, f: (r) => scName(r.id, r.name) },
    { k: "pos", t: "Pos", l: true },
    { k: "age", t: "Age" },
    { k: "ht", t: "Ht", div: true, v: (r) => inches(r.ht) },
    { k: "wt", t: "Wt" },
    { k: "now", t: "Read now", div: true, v: (r) => mid(r.read && r.read.now), f: (r) => scNow(r.read, r.id) },
    { k: "role", t: "Outlook", l: true, v: (r) => (r.read && r.read.role) || "", f: (r) => scRole(r.read) },
    { k: "looks", t: "Looks", v: (r) => (r.read ? r.read.looks : 0), f: (r) => (r.read ? r.read.looks : 0) },
  ];
  let body;
  if (tab.t === "Street") {
    body = `<p class="sub">Unsigned pros: the ones your staff has read first, best read first, then the rest by what the league paid them. A tryout brings him in on Tuesday.</p>${table("sc-street", [...readCols, { k: "x", t: "", f: (r) => (r.tried ? '<span class="st-pill">Tried out</span>' : `<button class="st-btn small glass" data-act="sc-tryout" data-id="${r.id}">Bring him in</button>`) }], x.street, { empty: "Nobody worth a call is on the street right now." })}`;
  } else if (tab.t === "Club") {
    body = `<div class="filters"><div class="field" style="min-width:280px"><label>Club</label><select data-act="sc-proclub"><option value="">Pick a club</option>${x.clubs.map((c) => `<option value="${c.ref.id}" ${x.club && x.club.ref.id === c.ref.id ? "selected" : ""}>${esc(c.name)}</option>`).join("")}</select></div>${x.club ? `<span class="teamcell">${mark(x.club, 28)} ${teamLink(x.club)}</span>` : ""}</div>
      ${x.club ? table("sc-club", [...readCols, { k: "apy", t: "A year", div: true, v: (r) => (r.contract ? r.contract.apy : 0), f: (r) => (r.contract ? money(r.contract.apy) : "") }, { k: "through", t: "Through", v: (r) => (r.contract ? r.contract.through : 0), f: (r) => (r.contract ? r.contract.through : "") }], x.roster) : `<div class="panel warm">${scEmpty("Pick a club to see its roster through your pro staff's eyes.")}</div>`}`;
  } else if (tab.t === "Tryouts") {
    body = table("sc-try", [{ k: "date", t: "Date", l: true }, { k: "name", t: "Name", l: true, f: (r) => scName(r.id, r.name) }, { k: "pos", t: "Pos", l: true }, { k: "school", t: "Where", l: true, f: scWhere }], x.tryouts, { empty: "No tryouts yet." });
  } else if (tab.t === "Supp") {
    body = `<p class="sub">The supplemental draft on ${esc(x.supp_date || "")}: bid a round, and if yours is the highest you give up that round next year.</p>${table("sc-supp", [...readCols, { k: "bid", t: "Your bid", div: true, f: (r) => `<select class="sc-sel" data-act="sc-supp" data-id="${r.id}"><option value="0">No bid</option>${[1, 2, 3, 4, 5, 6, 7].map((n) => `<option value="${n}" ${r.bid === n ? "selected" : ""}>Round ${n}</option>`).join("")}</select>` }], x.supp, { empty: "Nobody eligible this year." })}
      ${x.supp_past.length ? `<div class="panel warm"><h2>Past supplemental picks</h2>${tbl(["Year", "Player", "Club", "Round"], x.supp_past.map((s) => [String(s.year), playerLink(s.id, s.name), s.team ? s.team.name : "Nobody", s.round || "-"]))}</div>` : ""}`;
  } else {
    body = `<div class="sc-cards3">${x.scouts.map((s) => `<div class="panel warm"><h2>${esc(s.name)} <small>${esc(s.job)}</small></h2><span class="st-label">Covers</span><div class="sc-clubs">${s.clubs.map((c) => `<button class="sc-clubchip" data-act="sc-proclubgo" data-id="${c.ref.id}">${mark(c, 20)} ${esc(c.name)}</button>`).join("") || '<span class="sc-dim">The whole league</span>'}</div></div>`).join("")}</div>`;
  }
  return `<div class="film"><div class="pad">${scHead("Pro scouting", "Your pro personnel staff's reads on every club's players and on the street.", scGo("proscout"))}<div class="sc-tabrow">${tab.html}</div>${body}</div></div>`;
};

// ---------- a college's recruit, portal and JUCO boards (scouting topics 15, 17) ----------
screens.recruitboard = async () => {
  const k = keep("sc-rb", { list: "board", pos: "All", cls: "All" });
  const x = await api("recruit_scouting", { list: k.list });
  if (S.status && S.status.mine) setColors(S.status.mine.colors);
  const poss = [...new Set(x.rows.map((r) => r.pos))].sort();
  const classes = [...new Set(x.rows.map((r) => r.class_of))].sort();
  const rows = x.rows.filter((r) => (k.pos === "All" || r.pos === k.pos) && (k.cls === "All" || String(r.class_of) === String(k.cls)));
  const lists = [["board", "Recruit board"], ["portal", "Transfer portal"], ["juco", "JUCO"]];
  const hero = `<div class="sc-room st-room"><i></i><i></i><i></i><div class="sc-room-in"><div><span class="st-label">${esc(S.status && S.status.mine ? S.status.mine.name : "")} · ${esc(words(x.period))}</span><h1 class="st-wide sc-room-h">${esc(lists.find((l) => l[0] === k.list)[1])}</h1>
    <div class="sc-room-facts"><span><b>${x.rows.length}</b> on the list</span><span><b>${x.rows.filter((r) => r.offered).length}</b> offered</span><span><b>${x.rows.filter((r) => r.committed).length}</b> committed to you</span><span><b>${x.rows.filter((r) => r.read && r.read.now).length}</b> seen by your staff</span></div></div>
    <div class="sc-room-card st-glass">${scSquares(x.eval.fall[0], x.eval.fall[1], "fall evaluation days")}${scSquares(x.eval.spring[0], x.eval.spring[1], "spring evaluation days")}</div></div></div>`;
  const cols = [
    { k: "name", t: "Name", l: true, f: (r) => `${scName(r.id, r.name)}${r.followed ? ' <span class="st-pill gold" title="In My players">★</span>' : ""}` },
    { k: "pos", t: "Pos", l: true },
    { k: "class", t: k.list === "board" ? "Class of" : "Year", l: true, v: (r) => (k.list === "board" ? r.class_of : r.class), f: (r) => esc(k.list === "board" ? r.class_of : r.class) },
    { k: "school", t: k.list === "board" ? "High school" : "School now", l: true, f: (r) => `${scWhere(r)}${r.state ? ` <small class="muted">${esc(r.state)}</small>` : ""}` },
    { k: "ht", t: "Ht", div: true, v: (r) => inches(r.ht) },
    { k: "wt", t: "Wt" },
    { k: "stars", t: "★", tip: "The services' stars", f: (r) => scStars(r.stars) },
    { k: "now", t: "Your read", div: true, v: (r) => mid(r.read && r.read.now), f: (r) => scNow(r.read, r.id) },
    { k: "role", t: "Outlook", l: true, v: (r) => (r.read && r.read.role) || "", f: (r) => scRole(r.read) },
    { k: "looks", t: "Looks", v: (r) => (r.read ? r.read.looks : 0), f: (r) => (r.read ? r.read.looks : 0) },
    { k: "status", t: "With you", l: true, div: true, v: (r) => (r.committed ? 3 : r.offered ? 2 : r.visits ? 1 : 0), f: (r) => (r.committed ? '<span class="st-pill good">Committed</span>' : r.offered ? '<span class="st-pill">Offered</span>' : "") + (r.visits ? ` <small class="muted">${scPlural(r.visits, "visit")}</small>` : "") },
    { k: "offers", t: "Offers" },
    { k: "x", t: "", f: (r) => `<button class="sc-mini wide" data-act="sc-follow" data-id="${r.id}" data-on="${r.followed ? 0 : 1}">${r.followed ? "Following" : "Follow"}</button>` },
  ];
  return `<div class="film">${hero}<div class="pad"><div class="sc-tabrow"><div class="tabsline sc-tabs">${lists.map(([v, t]) => `<button data-act="sc-seg" data-k="sc-rb" data-f="list" data-v="${v}" aria-selected="${k.list === v}">${t}</button>`).join("")}</div>${scGo("recruitboard")}</div>
    <div class="filters"><div class="field"><label>Position</label><select data-act="sc-rbf" data-f="pos"><option value="All">Every position</option>${poss.map((p) => `<option ${k.pos === p ? "selected" : ""}>${esc(p)}</option>`).join("")}</select></div>${k.list === "board" && classes.length > 1 ? `<div class="field"><label>Class</label><select data-act="sc-rbf" data-f="cls"><option value="All">Every class</option>${classes.map((c) => `<option ${String(k.cls) === String(c) ? "selected" : ""}>${c}</option>`).join("")}</select></div>` : ""}<span class="sub">${k.list === "board" ? "The players your staff watches, with its read on each. Ask for a look from his file." : k.list === "portal" ? "Players in the transfer portal from other programs." : "Junior college players who could come to you."}</span></div>
    ${table("sc-rbt-" + k.list, cols, rows, { empty: k.list === "portal" ? "Nobody in the portal right now. It opens after the season and in the spring." : "Nobody on this list yet." })}</div></div>`;
};

// The high school coach's side (scouting topic 16) is Colleges calling now:
// screens/find.js (update 2, lane 7). Its call box keeps the sc-promote
// handlers below.

// ---------- clicks and changes of our own (data-act "sc-...") ----------
// Capture phase: runs before app.js's handler, so a board row can be
// picked without the row-click opening its first link.
document.addEventListener("click", async (ev) => {
  const el = ev.target.closest("[data-act]");
  if (!el || !String(el.dataset.act).startsWith("sc-")) {
    if (el || ev.target.closest("[data-go],[data-sort],input,select,textarea,button,a")) return;
    const tr = ev.target.closest("[data-sc-sel]");
    if (tr) {
      ev.stopPropagation();
      S.keep["sc-board"].sel = Number(tr.dataset.scSel);
      scRedraw();
    }
    return;
  }
  if (el.tagName === "SELECT") return;
  ev.stopPropagation();
  ev.preventDefault();
  const d = el.dataset;
  const id = d.id !== undefined && d.id !== "" ? Number(d.id) : null;
  switch (d.act) {
    case "sc-seg": {
      const v = d.v === "true" ? true : d.v === "false" ? false : d.v;
      S.keep[d.k][d.f] = v;
      scRedraw();
      break;
    }
    case "sc-askopen": SC.askOpen = !SC.askOpen; scRedraw(); break;
    case "sc-ask": {
      const r = await scDo("scout_ask", { id, kind: d.kind });
      if (r) { toast(r.said); SC.askOpen = false; scRedraw(); }
      break;
    }
    case "sc-follow": {
      if (d.on === "1") {
        const why = await ask("Follow him", { value: "Keep an eye on him", ok: "Follow", note: "He goes into My players with your staff's read today, so you can look back on it." });
        if (!why) break;
        if (await scDo("follow", { id, why }, "Following: he's in My players.")) scRedraw();
      } else if (await scDo("unfollow", { id }, "Taken out of My players.")) scRedraw();
      break;
    }
    case "sc-unfollow": if (await scDo("unfollow", { id })) scRedraw(); break;
    case "sc-attr": keep("sc-rat", { all: false, sel: null }).sel = d.key; scRedraw(); break;
    case "sc-instr": if (await scDo("scout_instruct", { key: d.key, on: d.on === "1" })) scRedraw(); break;
    case "sc-desk": keep("sc-pub", { desk: null }).desk = Number(d.k); scRedraw(); break;
    case "sc-pick": S.keep["sc-board"].sel = id; scTabTo("sc-boardtab", "The board"); render(); break;
    case "sc-move": if (await scDo("board_move", { id, to: Number(d.to) })) scRedraw(); break;
    case "sc-moveto": {
      const v = await ask("Move him on your board", { value: d.rank, ok: "Move", note: "The rank he should have. Everyone below moves down one." });
      const to = Number(v);
      if (v && to >= 1 && (await scDo("board_move", { id, to }, `Moved to No. ${to}.`))) scRedraw();
      break;
    }
    case "sc-list": if (await scDo("board_list", { id, list: Number(d.list), on: d.on === "1" }, d.on === "1" ? `Added to ${SC_LISTS[Number(d.list)].toLowerCase()}.` : "Taken off the list.")) scRedraw(); break;
    case "sc-listadd": {
      const sel = document.querySelector(`[data-sc-add="${d.list}"]`);
      if (!sel || !sel.value) break;
      if (await scDo("board_list", { id: Number(sel.value), list: Number(d.list), on: true }, `Added to ${SC_LISTS[Number(d.list)].toLowerCase()}.`)) scRedraw();
      break;
    }
    case "sc-dnd": {
      let why = "";
      if (d.on === "1") {
        why = await ask("Do not draft", { value: "Your call", ok: "Take him off the board", note: "In your words: why he's off. Your staff won't pick him." });
        if (!why) break;
      }
      if (await scDo("board_dnd", { id, on: d.on === "1", why })) scRedraw();
      break;
    }
    case "sc-pfa": {
      let bonus = 0;
      if (d.on === "1") {
        const v = await ask("Priority free agent", { value: "15000", ok: "Put him on the list", note: "The signing bonus you'd offer if he goes undrafted, in dollars. It comes out of your undrafted pool." });
        if (!v) break;
        bonus = Math.max(0, Math.round(Number(String(v).replace(/[^0-9.]/g, "")) || 0));
      }
      if (await scDo("board_pfa", { id, on: d.on === "1", bonus })) scRedraw();
      break;
    }
    case "sc-leak": {
      const sel = $("#sc-leak");
      if (!sel || !sel.value) break;
      const r = await scDo("leak", { id: Number(sel.value), real: d.real === "1" });
      if (r) { toast(r.said); scRedraw(); }
      break;
    }
    case "sc-draft": {
      const ok = await sure(`Draft ${d.name}?`, "The card goes to the podium. It can't be taken back.", "Turn in the card");
      if (!ok) break;
      const r = await scDo("draft_pick", { id });
      if (r) { scAfterPick(r); }
      break;
    }
    case "sc-auto": { const r = await scDo("draft_auto", {}); if (r) scAfterPick(r); break; }
    case "sc-timeout": { const r = await scDo("draft_timeout", {}); if (r) scAfterPick(r); break; }
    case "sc-go": {
      el.disabled = true;
      showBusy(d.until === "yours" ? "The other clubs are picking…" : "On the clock…");
      const r = await scDo("draft_go", { until: d.until });
      hideBusy();
      SC.quote = null;
      if (r) render();
      break;
    }
    case "sc-accept": if (await scDo("draft_accept", { offer: Number(d.k) }, "Trade done. The new picks are yours.")) { SC.quote = null; render(); } break;
    case "sc-quote": {
      const slot = Number(d.slot);
      const r = await scDo("draft_quote", { slot });
      const w = await api("draft_live").catch(() => null);
      SC.quote = { slot, offer: r, next: w ? w.next : null };
      scRedraw();
      break;
    }
    case "sc-tradeup": if (await scDo("draft_trade_up", { slot: Number(d.slot) }, "Trade done: you moved up.")) { SC.quote = null; render(); } break;
    case "sc-clock": {
      if (!SC.clock) break;
      if (d.v === "pause") SC.clock.paused = !SC.clock.paused;
      else SC.clock.speed = SC.clock.speed === Number(d.v) ? 1 : Number(d.v);
      scRedraw();
      break;
    }
    case "sc-unreveal": SC.reveal = null; scRedraw(); break;
    case "sc-rush": {
      showBusy("The phones are ringing: the undrafted rush…");
      const r = await scDo("draft_rush", {});
      hideBusy();
      if (r) { SC.live = false; SC.clock = null; if (S.status) S.status.draft_live = false; go("draftrecord"); }
      break;
    }
    case "sc-tryout": if (await scDo("tryout", { id }, "He's in for a tryout on Tuesday.")) scRedraw(); break;
    case "sc-proclubgo": keep("sc-pro", { club: null }).club = id; scTabTo("sc-protab", "A club's roster"); render(); break;
    case "sc-promote": keep("sc-hs", { pid: null }).pid = id; scRedraw(); break;
    case "sc-promotego": {
      const prog = $("#sc-prog"), str = $("#sc-strength");
      const args = { id, strength: Number(str ? str.value : 0) };
      if (prog && prog.value !== "") args.program = Number(prog.value);
      const r = await scDo("promote", args);
      if (r) { toast(r.said); S.keep["sc-hs"].pid = null; scRedraw(); }
      break;
    }
  }
}, true);
function scTabTo(key, label) {
  keep(key, { t: label }).t = label;
  S.route.args = { ...S.route.args, tab: label };
}
function scAfterPick(r) {
  SC.quote = null;
  if (r.revealed) SC.reveal = `${r.revealed.pick}-${r.revealed.id}`;
  render();
}
document.addEventListener("change", async (ev) => {
  const el = ev.target;
  const d = el.dataset || {};
  if (!d.act || !d.act.startsWith("sc-")) return;
  switch (d.act) {
    case "sc-assign": if (await scDo("scout_assign", { who: Number(d.who), target: el.value }, "Assignment set.")) scRedraw(); break;
    case "sc-tier": if (await scDo("board_move", { id: Number(d.id), tier: Number(el.value) }, `Moved to tier ${el.value}.`)) scRedraw(); break;
    case "sc-supp": await scDo("supp_bid", { id: Number(d.id), round: Number(el.value) }, Number(el.value) ? `Bid in: round ${el.value}.` : "Bid taken back."); break;
    case "sc-proclub": keep("sc-pro", { club: null }).club = el.value === "" ? null : Number(el.value); render(); break;
    case "sc-rbf": S.keep["sc-rb"][d.f] = el.value; scRedraw(); break;
    case "sc-wargroup": keep("sc-war", { group: "All" }).group = el.value; scRedraw(); break;
  }
});

// ---------- the player page's Scouting tab ----------
// Your staff's file in short on every player page; the full file is one click on.
PLAYER_TABS.push({
  name: "Scouting",
  after: "Reports",
  show: (p) => p.stage !== "Done",
  draw: async (p) => {
    let f = null;
    try { f = await api("scout_file", { id: p.id }); } catch (e) { return scEmpty("No scouting file on him."); }
    const head = `<div class="sc-ptab-h"><span class="sub">${scPlural(f.looks_n, "look")} by ${scPlural(f.readers_n, "person", "people")} on your staff${f.board && f.board.rank ? ` · No. ${f.board.rank} on your board` : ""}</span>${linkTo("scout", { id: p.id }, "Open the full scouting file", "st-btn small primary")}</div>`;
    return head + scFileSummary(f);
  },
});
