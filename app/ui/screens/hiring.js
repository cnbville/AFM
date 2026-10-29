// Lane 18: finding and hiring coaches (the coaching update's C21-C61).
// Every coach's read from your side (a stranger's is the whole 1-20
// scale; film, talk, an interview and an honest word narrow it), his
// units' record, the talks and an offer with years, promises and results
// clauses; the Hiring search with filters and pages; coaches to watch.
// Class names carry the lane's "hr-" prefix.

// A read on the hidden 1 to 20 scale: a bar and its range.
function hrBar(r, label) {
  const [lo, hi] = r;
  const m = (lo + hi) / 2;
  const tip = `Your read${label ? ` of his ${label.toLowerCase()}` : ""}: ${lo} to ${hi} on the hidden 1 to 20 scale. A narrow bar is a sure read.`;
  return `<span class="hr-bar" title="${esc(tip)}"><span class="hr-t"><i style="left:${((lo - 1) / 19) * 100}%;width:${Math.max(2, ((hi - lo) / 19) * 100)}%;background:${rateColor(m * 5)}"></i></span><b>${lo === hi ? lo : `${lo}-${hi}`}</b></span>`;
}

// The same read for a table cell: the range over a slim bar.
function hrMini(r, label) {
  const [lo, hi] = r;
  const tip = `Your read of his ${label}: ${lo} to ${hi} on the hidden 1 to 20 scale. A narrow bar is a sure read.`;
  return `<span class="hr-mini" title="${esc(tip)}"><b>${lo === hi ? lo : `${lo}-${hi}`}</b><span class="hr-t"><i style="left:${((lo - 1) / 19) * 100}%;width:${Math.max(4, ((hi - lo) / 19) * 100)}%;background:${rateColor(((lo + hi) / 2) * 5)}"></i></span></span>`;
}

// The coach page's Ratings tab for a coach who isn't on your staff (C24).
async function hrReadTab(id) {
  let x;
  try { x = await api("hiring_coach", { coach: id }); } catch (e) { return `<div class="empty">${esc(e.message)}</div>`; }
  return hrReadHtml(x);
}

function hrReadHtml(x) {
  const groups = ["Teaching", "Coaching", "Game day", "Recruiting"];
  const facetRows = groups.map((g) => {
    const rows = x.facets.filter((f) => f.group === g);
    return `<div class="hr-fg"><span class="st-label">${esc(g)}</span>${rows.map((f) => `<div class="hr-row"><span>${esc(f.label.replace(/^Teaching, /, ""))}</span>${hrBar(f.read, f.label)}</div>`).join("")}${g === "Coaching" ? `<div class="hr-row"><span>Ambition</span>${hrBar(x.ambition, "ambition")}</div>` : ""}</div>`;
  }).join("");
  const spent = x.film + x.talked;
  const asked = x.asked.length
    ? `<div class="items">${x.asked.map((a) => `<div class="item"><span class="when">${esc(a.date)}</span><span><b>${esc(a.who)}</b>: “${esc(a.words)}” <small class="muted">${esc(a.straight)}, you think</small></span></div>`).join("")}</div>`
    : "";
  const sources = x.sources.length
    ? `<div class="hr-src">${x.sources.map((s) => `<div class="hr-srow"><span>${esc(s.label)} <small class="muted">${s.warm ? `tie ${s.warm}` : "a stranger to you"}</small></span><button class="st-btn small glass" data-act="hrsource" data-id="${x.coach}" data-src="${esc(s.code)}" title="Half an hour; you owe him a favour">Ask for an honest read</button></div>`).join("")}</div>`
    : '<div class="empty">Nobody you could call knows him.</div>';
  const units = x.units.length
    ? table("hr-units", [{ k: "year", t: "Season" }, { k: "team", t: "Team", l: true, f: (r) => rich(r.team), v: (r) => plainText(r.team) }, { k: "side", t: "Side", l: true }, { k: "ppg", t: "Points", tip: "Points a game for an offense, allowed for a defense" }, { k: "rank", t: "Rank", l: true, f: (r) => esc(r.rank ? `${r.rank} ${r.pool ? `in ${r.pool}` : ""}` : "—") }], x.units, { tools: false })
    : '<div class="empty">He hasn\'t run a side in a season the game kept.</div>';
  const rooms = x.rooms.length
    ? `<div class="items">${x.rooms.map((r) => `<div class="item"><span class="when">${esc(r.group)}</span><span>${esc(r.growth)} <small class="muted">· about ${r.seasons} season${r.seasons === 1 ? "" : "s"}${r.movedUp >= 1 ? ` · ${r.movedUp} moved up` : ""}</small></span></div>`).join("")}</div>`
    : '<div class="empty">No room of his has a record yet.</div>';
  const talk = x.talk ? `<p class="hr-talk">${esc(x.talk)}</p>${(x.log || []).length ? `<div class="items">${x.log.map((l) => `<div class="item"><span class="when">${esc(l.date)}</span><span>${rich(l.text)}</span></div>`).join("")}</div>` : ""}` : "";
  const jobs = x.yours
    ? '<div class="empty">He\'s on your staff.</div>'
    : x.openings.length
      ? `<div class="hr-jobs">${x.openings.map((o) => `<div class="hr-job"><span><b>${esc(o.name)}</b> <small class="muted">asks about ${esc(o.asks)}${o.outside ? " · outside the group he came up in" : ""}</small></span><span class="hr-acts"><button class="st-btn small glass" data-act="ppinterview" data-id="${x.coach}" data-role="${o.key}">Interview</button><button class="st-btn small primary" data-act="hroffer" data-id="${x.coach}" data-role="${o.key}">Make an offer</button></span></div>`).join("")}</div>`
      : '<div class="empty">You have no open job. Let someone go first.</div>';
  return `<div class="hr"><div class="grid2"><div class="col"><section class="panel"><h2>Your read <small>on the hidden 1 to 20 scale · a stranger's is the whole scale</small></h2>${facetRows}
      <div class="hr-pot"><span class="st-label">Potential</span><b>${esc(x.potential)}</b><small class="muted">${esc(x.sure)}</small></div></section></div>
    <div class="col"><section class="panel"><h2>Look into him <small>${spent ? `${hrHours(x.film)} on his film, ${hrHours(x.talked)} asking around` : "you haven't looked yet"} · ${hrHours(x.hoursLeft)} left this week</small></h2>
      <div class="hr-acts"><button class="st-btn small primary" data-act="hrfilm" data-id="${x.coach}" title="Two hours of your week">Watch his film</button><button class="st-btn small glass" data-act="hraround" data-id="${x.coach}" title="An hour: his players, his colleagues">Ask around</button></div>
      <span class="st-label hr-mt">People who know him</span>${sources}${asked ? `<span class="st-label hr-mt">What you heard</span>${asked}` : ""}</section>
      <section class="panel"><h2>Hire him <small>${esc(x.now)}</small></h2>${talk}${jobs}</section>
      <section class="panel"><h2>His units <small>points a game, ranked at his level</small></h2>${units}</section>
      <section class="panel"><h2>His rooms <small>how his players grew, from what you could see</small></h2>${rooms}</section></div></div></div>`;
}

function hrHours(h) {
  const v = Math.round(h * 2) / 2;
  return `${v} hour${v === 1 ? "" : "s"}`;
}

// The offer (C31, C34, C46, C54): salary, years, promises, clauses.
async function hrOffer(coach, role) {
  let x;
  try { x = await api("hiring_coach", { coach }); } catch (e) { toast(e.message); return; }
  const job = x.openings.find((o) => o.key === role);
  if (!job) { toast("That job isn't open."); return; }
  const prom = x.promises.map((p) => `<label class="hr-chk"><input type="checkbox" name="pr" value="${esc(p.key)}"><span>${esc(p.name)}</span><small class="muted">${esc(p.believe)}</small></label>`).join("");
  const opts = `<option value="">None</option>${x.clauses.map((c) => `<option value="${c.mark}.${c.reward}">${esc(c.words)}</option>`).join("")}`;
  const html = `<b>Offer ${esc(x.name)} the ${esc(job.name.toLowerCase())} job</b>
    <small class="sub">He asks about ${esc(job.asks)}.${x.talk ? ` ${esc(x.talk)}` : ""}${job.outside ? " It's outside the group he came up in: an adaptable man minds less." : ""} A low offer brings a warning, not the end; a promise counts by how much he believes it.</small>
    <div class="hr-form"><label>Salary a year<input name="salary" value="${Math.round(job.asksValue / 1000) * 1000}" autocomplete="off"></label>
    <label>Years<select class="ssel" name="years">${[1, 2, 3, 4, 5].map((y) => `<option value="${y}" ${y === 2 ? "selected" : ""}>${y}</option>`).join("")}</select></label></div>
    <span class="st-label">Promises <small>only what you control</small></span><div class="hr-proms">${prom}</div>
    <span class="st-label">Results clauses <small>they reward results, never promise them</small></span><div class="hr-form"><select class="ssel" name="c1" aria-label="First clause">${opts}</select><select class="ssel" name="c2" aria-label="Second clause">${opts}</select></div>`;
  const { answer } = dialog(html, "Make the offer", (f) => ({
    salary: Number(String(f.elements.salary.value).replace(/[^0-9.]/g, "")),
    years: Number(f.elements.years.value),
    promises: [...f.querySelectorAll("input[name=pr]:checked")].map((i) => i.value),
    clauses: [f.elements.c1.value, f.elements.c2.value].filter(Boolean).filter((v, i, a) => a.indexOf(v) === i).map((v) => { const [m, r] = v.split("."); return { mark: Number(m), reward: Number(r) }; }),
  }));
  const t = await answer;
  if (!t) return;
  if (!t.salary) { toast("That isn't a salary."); return; }
  try {
    const r = await api("hiring_offer", { coach, role, ...t });
    toast(plainText(r.text));
    refresh();
  } catch (e) { toast(e.message); }
}

// The Hiring search (C21, C26): filters, sorts and pages over every coach.
const HR_LEVELS = [["", "Every level"], ["nfl", "NFL"], ["fbs", "FBS"], ["fcs", "FCS"], ["d2", "Division II"], ["d3", "Division III"], ["naia", "NAIA"], ["juco", "Junior college"], ["hs", "High school"], ["other", "Other pro leagues"], ["none", "Out of work"]];
const HR_JOBS = [["", "Any job"], ["hc", "Head coaches"], ["coord", "Coordinators"], ["pc", "Position coaches"], ["junior", "GAs, analysts, quality control"], ["none", "Out of work"]];
const HR_GROUPS = [["", "Came up as anything"], ["qb", "Quarterbacks"], ["rb", "Running backs"], ["wr", "Receivers"], ["ol", "Offensive line"], ["dl", "Defensive line"], ["lb", "Linebackers"], ["db", "Defensive backs"], ["st", "Special teams"]];
const HR_SORTS = [["", "Reputation"], ["read", "Your read"], ["potential", "Potential (your read)"], ["age", "Youngest"], ["asks", "Cheapest ask"]];

async function hrSearch(x) {
  const roles = [...x.openings.map((o) => [o.key, o.name, true]), ["oc", "Offensive coordinator"], ["dc", "Defensive coordinator"], ["qb", "Quarterbacks coach"], ["rb", "Running backs coach"], ["wr", "Wide receivers coach"], ["ol", "Offensive line coach"], ["dl", "Defensive line coach"], ["lb", "Linebackers coach"], ["db", "Defensive backs coach"], ["st", "Special teams coordinator"]].filter((r, i, arr) => arr.findIndex((y) => y[0] === r[0]) === i);
  const k = keep("pshire", { role: x.openings[0] ? x.openings[0].key : "oc" });
  const F = keep("hrfilter", { page: 0 });
  const seg = `<div class="seg wrap">${roles.map(([v, t, open]) => `<button data-act="pprole" data-v="${v}" aria-pressed="${k.role === v}">${esc(t)}${open ? " <small>open</small>" : ""}</button>`).join("")}</div>`;
  const c = await api("hiring_search", { role: k.role, ...F });
  const open = x.openings.some((o) => o.key === k.role);
  const sel = (key, list) => `<select class="ssel hr-f" data-k="${key}" aria-label="${key}">${list.map(([v, t]) => `<option value="${v}" ${(F[key] || "") === v ? "selected" : ""}>${esc(t)}</option>`).join("")}</select>`;
  const num = (key, ph) => `<input class="hr-num" data-k="${key}" value="${F[key] ?? ""}" placeholder="${ph}" inputmode="numeric" aria-label="${ph}">`;
  const chk = (key, t) => `<label class="hr-tog"><input type="checkbox" class="hr-f" data-k="${key}" ${F[key] ? "checked" : ""}>${esc(t)}</label>`;
  const filters = `<div class="hr-filters">${sel("level", HR_LEVELS)}${sel("job", HR_JOBS)}${sel("cameUp", HR_GROUPS)}${num("ageMin", "Age from")}${num("ageMax", "to")}${num("repMin", "Reputation from")}${chk("known", "Known to you")}${chk("free", "Out of work")}<span class="hr-sp"></span>${sel("sort", HR_SORTS)}</div>`;
  // Two lines a cell keep every column in view at 1280: who he is over his
  // age and roots, his job over his name in the game, the potential word
  // over the hours you've put in.
  const tags = (r) => `${r.outside ? ' <span class="tag" title="The job is outside the group he came up in">Outside his group</span>' : ""}${r.blocked ? ' <span class="tag due" title="His team can block a lateral move">Can be blocked</span>' : ""}${r.talk ? ` <span class="tag gold" title="${esc(r.talk)}">Talking</span>` : ""}`;
  const cols = [
    { k: "link", t: "Coach", l: true, f: (r) => `<span class="hr-two">${rich(r.link)}<small>${r.age} · came up with the ${esc(r.cameUp)}${tags(r)}</small></span>`, v: (r) => plainText(r.link) },
    { k: "now", t: "Now", l: true, wrap: true, f: (r) => `<span class="hr-two">${rich(r.now)}<small>${esc(cap(r.rep))}</small></span>`, v: (r) => plainText(r.now) },
    { k: "read", t: c.coord ? "Teaching, his side" : "Teaching", tip: "Your read on the hidden 1 to 20 scale", f: (r) => hrMini(r.read, "teaching"), v: (r) => r.read[0] + r.read[1] },
    ...(c.coord ? [{ k: "scheme", t: "Scheme", f: (r) => hrMini(r.scheme, "scheme"), v: (r) => r.scheme[0] + r.scheme[1] }, { k: "calling", t: "Calling", f: (r) => hrMini(r.calling, "play calling"), v: (r) => r.calling[0] + r.calling[1] }] : []),
    { k: "potential", t: "Potential", l: true, wrap: true, f: (r) => `<span class="hr-two">${esc(r.potential)}${r.hours ? `<small>${r.hours} ${r.hours === 1 ? "hour" : "hours"} in</small>` : ""}</span>` },
    { k: "asks", t: "Asks", v: (r) => r.asksValue },
    { k: "act", t: "", f: (r) => `<span class="hr-acts"><button class="st-btn small glass" data-act="ppinterview" data-id="${r.coach}" data-role="${c.role}">Interview</button><button class="st-btn small primary" data-act="hroffer" data-id="${r.coach}" data-role="${c.role}" ${open ? "" : 'disabled title="The job isn\'t open on your staff: let someone go first"'}>Offer</button></span>`, v: () => 0 },
  ];
  const pager = c.pages > 1 ? `<div class="hr-pager"><button class="st-btn small glass" data-act="hrpage" data-v="${c.page - 1}" ${c.page <= 0 ? "disabled" : ""}>Previous</button><span>Page ${c.page + 1} of ${c.pages}</span><button class="st-btn small glass" data-act="hrpage" data-v="${c.page + 1}" ${c.page + 1 >= c.pages ? "disabled" : ""}>Next</button></div>` : "";
  return `<section class="panel hr"><h2>${esc(c.roleName)} <small>${Number(c.total).toLocaleString()} coaches · ${open ? "any of them can be offered the job" : "the job isn't open: let someone go first, or keep him in mind"} · ${hrHours(c.hours)} left this week</small></h2>${seg}${filters}${table("hr-cand", cols, c.rows, { empty: "Nobody matches these filters.", tools: false })}${pager}</section>`;
}

// Coaches to watch (C42).
const HR_WATCH_LEVELS = [["college", "College"], ["fbs", "FBS"], ["fcs", "FCS"], ["d2", "Division II"], ["d3", "Division III"], ["naia", "NAIA"], ["juco", "Junior college"], ["hs", "High schools"], ["nfl", "NFL"]];
async function hrWatch() {
  const W = keep("hrwatch", { level: "college", stat: "" });
  const x = await api("hiring_watch", { level: W.level, stat: W.stat });
  const seg = `<div class="seg wrap">${HR_WATCH_LEVELS.map(([v, t]) => `<button data-act="hrwl" data-v="${v}" aria-pressed="${W.level === v}">${esc(t)}</button>`).join("")}</div>`;
  const stats = `<select class="ssel hr-ws" aria-label="Stat"><option value="">Every stat</option>${x.stats.map((s) => `<option value="${esc(s)}" ${W.stat === s ? "selected" : ""}>${esc(cap(s))}</option>`).join("")}</select>`;
  const rows = table("hr-watch", [
    { k: "team", t: "Team", l: true, f: (r) => rich(r.team), v: (r) => plainText(r.team) },
    { k: "stat", t: "What stood out", l: true, f: (r) => esc(cap(r.stat)) },
    { k: "value", t: "A game", v: (r) => Number(r.value) },
    { k: "average", t: "Level average", v: (r) => Number(r.average) },
    { k: "rank", t: "Rank", l: true },
    { k: "pool", t: "Among", l: true, hide: true },
    { k: "coaches", t: "In charge", l: true, wrap: true, f: (r) => r.coaches.map((c) => `${rich(c.link)} <small class="muted">${esc(c.role)}</small>`).join(", "), v: (r) => r.coaches.length },
  ], x.rows, { empty: "No unit stood out, or the season's numbers aren't in yet." });
  const rooms = x.rooms.length
    ? table("hr-rooms", [{ k: "link", t: "Coach", l: true, f: (r) => rich(r.link), v: (r) => plainText(r.link) }, { k: "now", t: "Now", l: true }, { k: "group", t: "Room", l: true }, { k: "growth", t: "His players", l: true }], x.rooms, { tools: false })
    : '<div class="empty">No room stands out yet.</div>';
  return `<div class="hr"><section class="panel"><h2>Units that stood out <small>${x.year} season, well above their level's average</small></h2><div class="hr-filters">${seg}<span class="hr-sp"></span>${stats}</div>${rows}</section>
    <section class="panel"><h2>Rooms that grow players <small>position coaches whose players grow well above the norm</small></h2>${rooms}</section></div>`;
}

async function hrLook(el, cmd, extra) {
  try {
    const x = await api(cmd, { coach: Number(el.dataset.id), ...(extra || {}) });
    toast(plainText(x.text));
    refresh(el);
  } catch (e) { toast(e.message); }
}

// A coach page's or a network row's hire button (C23): his open jobs.
async function hrHireMenu(el) {
  const id = Number(el.dataset.id);
  let x;
  try { x = await api("hiring_coach", { coach: id }); } catch (e) { toast(e.message); return; }
  if (x.yours) { toast("He's on your staff."); return; }
  if (!x.openings.length) { toast("You have no open job. Let someone go first."); return; }
  menuAt(el, `<div class="st-label">Hire ${esc(x.name)}</div>${x.openings.map((o) => `<div class="mrow hr-mrow"><span>${esc(o.name)} <small class="muted">asks about ${esc(o.asks)}</small></span><span><a class="link small" data-hr="i" data-role="${o.key}">Interview</a> · <a class="link small" data-hr="o" data-role="${o.key}">Offer</a></span></div>`).join("")}${x.talk ? `<small class="sub">${esc(x.talk)}</small>` : ""}`, (e) => {
    const a = e.target.closest("[data-hr]");
    if (!a) return;
    closeMenu();
    if (a.dataset.hr === "i") ppGoScene(api("people_interview", { coach: id, role: a.dataset.role }));
    else hrOffer(id, a.dataset.role);
  });
}

Object.assign(CORE_MORE, {
  hroffer: (el) => hrOffer(Number(el.dataset.id), el.dataset.role),
  hrfilm: (el) => hrLook(el, "hiring_film"),
  hraround: (el) => hrLook(el, "hiring_around"),
  hrsource: (el) => hrLook(el, "hiring_source", { source: el.dataset.src }),
  hrhire: (el, ev) => { if (ev) ev.stopPropagation(); hrHireMenu(el); },
  hrpage: (el) => { keep("hrfilter", { page: 0 }).page = Math.max(0, Number(el.dataset.v)); refresh(el); },
  hrwl: (el) => { keep("hrwatch", { level: "college", stat: "" }).level = el.dataset.v; refresh(el); },
});

// The filters apply when they change (a number on Enter or when the field
// is left).
document.addEventListener("change", (ev) => {
  const el = ev.target;
  if (!el.classList) return;
  if (el.classList.contains("hr-ws")) {
    keep("hrwatch", { level: "college", stat: "" }).stat = el.value;
    refresh(el);
    return;
  }
  if (!el.classList.contains("hr-num") && !el.classList.contains("hr-f")) return;
  const F = keep("hrfilter", { page: 0 });
  const v = el.type === "checkbox" ? el.checked : el.value.trim();
  F[el.dataset.k] = el.classList.contains("hr-num") ? (v === "" ? undefined : Number(v)) : v;
  F.page = 0;
  refresh(el);
});
