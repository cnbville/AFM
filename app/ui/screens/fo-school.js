// Front office screens: the high school seat and your career's market
// (lane 10). The school office (the people you answer to, the year's
// goals, the booster club's books, your teaching job and your hours),
// your squads (varsity, JV and freshman ball, with their games), the
// school and its feeder area, and the job market every career has.
// Buttons here carry `data-fs`; app.js's helpers do the rest.
"use strict";

Object.assign(I, {
  school: '<path d="M3 10l9-6 9 6M5 10v9h14v-9M9 19v-5h6v5M12 4v3"/>',
  map: '<path d="M3 6l6-2 6 2 6-2v14l-6 2-6-2-6 2z M9 4v14M15 6v14"/>',
  market: '<rect x="3" y="7" width="18" height="13" rx="2"/><path d="M9 7V5a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2M3 12h18"/>',
});
Object.assign(CRUMBS, { school: "School office", squads: "Varsity and JV", area: "School and feeders", market: "Job market" });

foNavAdd("hs", [
  ["school", "School office", "school"],
  ["squads", "Varsity and JV", "squad"],
  ["area", "School and feeders", "map"],
  ["recruiting", "Recruiting", "recruit"],
]);
// Every career has a job market: what's open, what you'd take, your offers.
foNavAdd("all", [["market", "Job market", "market"]]);

// Where each of the seat's screens leads (screens topic 17).
GOTO.school = () => [["Varsity and JV", "squads"], ["School and feeders", "area"], ["Finances", "money"], ["Staff", "staff"], ["Schedule", "schedule"]];
GOTO.squads = () => [["Depth chart", "depth"], ["Squad", "roster"], ["Schedule", "schedule"], ["School office", "school"]];
GOTO.area = () => [["School office", "school"], ["Recruiting", "recruiting"], ["Standings", "standings"], ["Team page", "team"]];
GOTO.market = () => [["Pick your next job", "jobpick"], ["Your profile", "you"], ["Inbox", "inbox"], ["News", "news"]];
GOTO_AFTER.push("school", "area");

const fsTrust = (t) => (/Firmly|Pleased/.test(t) ? "good" : /Watching/.test(t) ? "" : "fs-warn");
const fsPct = (a, b) => Math.max(0, Math.min(100, Math.round((a / Math.max(1, b)) * 100)));
const fsGrade = (g) => ({ 9: "Fr", 10: "So", 11: "Jr", 12: "Sr" })[g] || "";
// A person the people screens know links to his page.
const fsWho = (name, tok) => (tok ? rich(`[[s:people_cast?tok=${tok}|${name}]]`) : esc(name));

// ---------- the school office ----------
screens.school = async () => {
  const x = await api("school_office");
  setColors(x.school.colors);
  const people = x.people.map((p) => `<div class="fs-person">
      <div class="fs-who"><span class="fs-init">${esc(initials(p.name))}</span><div><b>${fsWho(p.name, p.tok)}</b><small>${esc(p.title)} · ${p.years ? `${p.years} year${p.years === 1 ? "" : "s"} here` : "new this year"}</small></div></div>
      <p class="sub">${esc(p.about)}${p.child ? ` ${p.she ? "Her" : "His"} son ${playerLink(p.child.id, p.child.name)} is on your roster.` : ""}</p>
      <div class="fs-tags"><span class="st-pill ${fsTrust(p.trust)}">${esc(p.trust)}</span><span class="fs-tag">Weighs: ${esc(p.wants)}</span><span class="fs-tag">${esc(p.patience)}</span></div>
    </div>`).join("");
  const goals = x.goals.length
    ? x.goals.map((g) => {
        const done = g.met != null;
        const pill = done ? (g.met ? '<span class="st-pill good">Met</span>' : '<span class="st-pill fs-warn">Missed</span>') : g.on_track ? '<span class="st-pill good">On track</span>' : '<span class="st-pill gold">Not yet</span>';
        const small = [g.by, g.deal, g.prog].filter(Boolean).map(esc).join(" · ");
        return `<div class="fs-goal"><div><b>${esc(g.text)}</b><small>${small}</small></div>${pill}</div>`;
      }).join("")
    : '<div class="empty">The principal sets the year\'s goals in the start-of-year meeting, before school starts; the athletic director adds the program\'s numbers the week it does.</div>';
  const b = x.boosters;
  const raised = b.raised.reduce((a, r) => a + r.x, 0), spent = b.spent.reduce((a, r) => a + r.x, 0);
  const flow = (rows, total) => rows.filter((r) => r.x > 0).map((r) => `<div class="fs-flow"><span>${esc(r.what)}</span><span class="t"><i style="width:${fsPct(r.x, total)}%"></i></span><b>${money(r.x)}</b></div>`).join("") || '<div class="empty">Nothing yet this football year.</div>';
  const nextUp = b.projects.find((p) => !p.done);
  const wish = b.projects.map((p, i) => `<tr class="${p.done ? "deep" : ""}"><td class="l">${p.done ? "✓" : p.rank}</td><td class="l wrap">${esc(p.name)}${p === nextUp ? ' <span class="st-pill gold">Next</span>' : ""}</td><td>${money(p.cost)}</td><td class="l">${p.done ? `Paid for in ${p.done}` : `<button class="st-btn small glass" data-fs="wish" data-k="${p.k}" data-d="-1" ${i === 0 ? "disabled" : ""}>Up</button> <button class="st-btn small glass" data-fs="wish" data-k="${p.k}" data-d="1" ${i === b.projects.length - 1 ? "disabled" : ""}>Down</button>`}</td></tr>`).join("");
  const ledger = b.ledger.map((l) => `<tr><td class="l">${esc(l.date)}</td><td class="l wrap">${esc(l.what)}</td><td class="${l.x < 0 ? "fs-out" : "fs-in"}">${l.x < 0 ? "-" : "+"}${money(Math.abs(l.x))}</td></tr>`).join("");
  const t = x.teaching, h = x.hours;
  const reviews = x.reviews.map((r) => `<div class="fs-review"><b>${r.year}: ${r.met} of ${r.of} goals met</b>${r.words.map(([who, text]) => `<div><span class="sub">${esc(who)}</span> ${esc(text)}</div>`).join("")}</div>`).join("");
  return `<div class="hero small"><div class="art"></div>${logoHero(x.school) || `<div class="st-outline">${esc(initials(x.school.name))}</div>`}<div class="copy">
      <div class="st-label" style="color:rgba(255,255,255,.7)">The ${esc(x.mascot)} · school office</div><h1>${esc(x.school.name)}</h1>
      <div class="meta"><span>Booster club <b>${esc(x.money)}</b> in the bank</span><span><b>${h.total.toFixed(0)}</b> hours this week</span><span>${esc(t.coordinator ? "Campus athletic coordinator" : t.subject + " teacher")}</span></div></div></div>
    <div class="pad">
      <div class="panel"><h2>The people you answer to <small>trust in their own words; nobody shows you a number</small></h2><div class="fs-people">${people}</div></div>
      <div class="grid2">
        <div class="panel"><h2>This year's goals <small>${x.year ? `set for ${x.year}` : "not set yet"}</small></h2><div class="fs-goals">${goals}</div>
          ${reviews ? `<h2 style="margin-top:6px">How past seasons were judged</h2><div class="fs-reviews">${reviews}</div>` : ""}</div>
        <div class="panel"><h2>Your job at the school <small>most high school head coaches teach</small></h2>
          <div class="st-tiles"><div><b>${money(t.salary)}</b><small>${t.coordinator ? "coordinator's salary" : "teaching salary"}</small></div>${t.stipend ? `<div><b>${money(t.stipend)}</b><small>coaching stipend</small></div>` : ""}<div><b>${money(t.total)}</b><small>a year in all</small></div></div>
          <p class="sub">${esc(t.line)}</p>
          <div class="fs-hours"><div class="fs-flow"><span>Teaching</span><span class="t"><i style="width:${fsPct(h.teach, 90)}%"></i></span><b>${h.teach.toFixed(0)} h</b></div>
          <div class="fs-flow"><span>Football</span><span class="t"><i class="acc" style="width:${fsPct(h.coach, 90)}%"></i></span><b>${h.coach.toFixed(0)} h</b></div></div>
          <p class="sub">This week: ${esc(h.what)}.</p></div>
      </div>
      <div class="panel"><h2>The booster club <small>${b.members} members · they pay for the gear, the meals, the buses and the projects on your wish list</small></h2>
        <div class="st-tiles"><div><b>${money(b.balance)}</b><small>in the bank</small></div><div><b>${money(raised)}</b><small>raised this year</small></div><div><b>${money(spent)}</b><small>spent this year</small></div></div>
        <div class="grid2"><div style="display:grid;gap:8px"><div class="st-label">Money in</div>${flow(b.raised, raised)}<div class="st-label" style="margin-top:8px">Money out</div>${flow(b.spent, spent)}</div>
        <div style="display:grid;gap:8px"><div class="st-label">Your wish list <span class="sub" style="text-transform:none;letter-spacing:0;font-weight:500">· the club votes on the next one at the end of June, when the money is there</span></div><div class="tbl"><table><thead><tr><th class="l">#</th><th class="l">Project</th><th>Cost</th><th class="l"></th></tr></thead><tbody>${wish}</tbody></table></div></div></div>
        ${ledger ? `<div class="st-label">The books</div><div class="tbl box"><table><thead><tr><th class="l">Date</th><th class="l">What</th><th>Amount</th></tr></thead><tbody>${ledger}</tbody></table></div>` : ""}
      </div>
    </div>`;
};

// ---------- squads ----------
screens.squads = async () => {
  const x = await api("school_squads");
  setColors(x.school.colors);
  const k = keep("squads", { t: "varsity" });
  if (!x.teams.find((t) => t.key === k.t)) k.t = x.teams[0].key;
  const cur = x.teams.find((t) => t.key === k.t);
  const yours = x.owner === "you";
  const sugg = x.suggestions.length
    ? `<div class="panel warm"><h2>Your staff suggests <small>${x.suggestions.length}</small></h2><div class="items">${x.suggestions.map((s) => `<div class="item"><span class="when">${esc(s.date)}</span><span>${playerLink(s.id, s.name)} <span class="sub">${esc(s.pos)}</span> to ${esc(s.to)}: ${esc(s.why)} ${yours ? `<button class="st-btn small primary" data-fs="sugg" data-id="${s.id}" data-take="1">Move him</button> <button class="st-btn small glass" data-fs="sugg" data-id="${s.id}" data-take="0">Leave him</button>` : ""}</span></div>`).join("")}</div></div>`
    : "";
  const moveCell = (r) => {
    if (!yours) return r.moved ? '<span class="st-pill gold">Moved</span>' : "";
    const opts = r.can.map((c) => `<option value="${c}">${c === "jv" ? "JV" : c[0].toUpperCase() + c.slice(1)}</option>`).join("");
    return `${r.moved ? `<span class="st-pill gold" title="Your staff had him on ${esc(r.staff)}">You moved him</span> ` : ""}${opts ? `<select class="fs-move" data-fs-move="${r.id}"><option value="">Move to…</option>${opts}</select>` : ""}`;
  };
  const cols = [
    { k: "name", t: "Name", l: true, f: (r) => playerLink(r.id, r.name) },
    { k: "pos", t: "Pos", l: true },
    { k: "grade", t: "Grade", l: true, f: (r) => fsGrade(r.grade) },
    { k: "ht", t: "Ht", div: true, v: (r) => inches(r.ht) },
    { k: "wt", t: "Wt" },
    { k: "ovr", t: "Rating", div: true, tip: "Your staff's read of his play now", v: (r) => mid(r.ovr), f: (r) => range(r.ovr, null, { p: r.id, k: "ovr" }) },
    { k: "status", t: "Status", l: true, f: (r) => (r.status ? `<span class="dot bad"></span>${esc(r.status)}` : '<span class="muted">Available</span>') },
    ...(cur.key !== "varsity" ? [{ k: "line", t: "This season", l: true, div: true, v: (r) => (r.line ? r.line.games : 0), f: (r) => (r.line ? `<span class="sub">${r.line.games} g</span> ${esc(r.line.text)}` : '<span class="muted">-</span>') }] : []),
    { k: "move", t: yours ? "Team" : "", l: true, div: true, v: (r) => (r.moved ? 1 : 0), f: moveCell },
  ];
  const games = cur.games.map((g) => `<tr><td class="l">${esc(g.date)}</td><td class="l">${g.home ? "vs" : "at"} ${teamLink(g.opp)}</td><td>${g.score ? `<b class="${g.won ? "fs-in" : g.score[0] === g.score[1] ? "" : "fs-out"}">${g.won ? "W" : g.score[0] === g.score[1] ? "T" : "L"} ${g.score[0]}-${g.score[1]}</b>` : `<span class="muted">${esc(g.note || "")}</span>`}</td><td class="l wrap">${g.top.map((t) => `${playerLink(t.id, t.name)} <span class="sub">${esc(t.text)}</span>`).join(" · ")}</td></tr>`).join("");
  return `<div class="film"><div class="pad">
    <div style="display:flex;align-items:center;gap:14px;flex-wrap:wrap">${mark(x.school, 48)}<div style="flex:1;min-width:260px"><div class="h1">${esc(x.teams.length > 1 ? x.teams.map((t) => t.name).join(", ").replace(/, ([^,]*)$/, " and $1") : "Varsity")}</div><div class="sub">${esc(x.fields)}</div></div>
      <div class="seg" title="Who sets the squads"><button data-fs="owner" data-v="you" aria-pressed="${yours}">You set them</button><button data-fs="owner" data-v="staff" aria-pressed="${!yours}">Your staff sets them</button></div></div>
    <p class="sub">${yours ? `Your staff's lists stand until you move someone${x.moves ? `: you have moved ${x.moves} player${x.moves === 1 ? "" : "s"}${x.set_on ? ", last on " + esc(x.set_on) : ""}` : ""}. A player you keep off varsity who would start there gets a JV starter's reps instead of a varsity starter's.` : esc(x.staff_does)} ${esc(x.rules)}</p>
    ${gotoFor("squads")}
    ${sugg}
    <div class="tabsline">${x.teams.map((t) => `<button data-fs="tab" data-t="${t.key}" aria-selected="${t.key === cur.key}">${esc(t.name)} <small class="sub">${t.count}${t.record ? " · " + t.record : ""}</small></button>`).join("")}</div>
    ${yours && x.moves ? '<div><button class="st-btn small glass" data-fs="reset">Give every squad back to the staff\'s lists</button></div>' : ""}
    ${table("squad-" + cur.key, cols, cur.players, { groupBy: (r) => r.pos, empty: cur.key === "varsity" ? "Nobody on this team." : "Nobody on this team now: under 22 players can't dress, so they play a team up." })}
    ${cur.key !== "varsity" ? `<div class="panel"><h2>${esc(cur.name)} games <small>Thursdays${cur.record ? " · " + cur.record : ""}</small></h2>${games ? `<div class="tbl"><table><thead><tr><th class="l">Date</th><th class="l">Opponent</th><th>Score</th><th class="l">Best lines</th></tr></thead><tbody>${games}</tbody></table></div>` : '<div class="empty">The schedule follows the varsity\'s: each week\'s game is the Thursday before Friday night.</div>'}</div>` : ""}
  </div></div>`;
};

// ---------- the school and its feeder area ----------
// The town drawn around your school (screens topic 7): the feeder middle
// schools and the youth teams where they sit, each one's road in as wide
// as its share of your freshmen, and how many of your players came from it.
function fsHash(s) {
  let h = 2166136261;
  for (let i = 0; i < s.length; i++) h = Math.imul(h ^ s.charCodeAt(i), 16777619);
  return (h >>> 0) / 4294967296;
}
function fsTownMap(x) {
  const W = 640, H = 380, cx = W / 2, cy = H / 2 + 6;
  const seed = fsHash(x.school.name + x.town);
  const [c1, c2] = x.school.colors || ["#a71930", "#322f2b"];
  const n = x.feeders.length || 1;
  // The town's streets: a loose grid turned a little, and the main roads out.
  const tilt = (seed - 0.5) * 0.5;
  const streets = [];
  for (let k = -6; k <= 6; k++) {
    const o = k * 46 + (fsHash(x.town + k) - 0.5) * 14;
    const [dx, dy] = [Math.cos(tilt), Math.sin(tilt)];
    streets.push(`<line x1="${cx - dx * 420 - dy * o}" y1="${cy - dy * 420 + dx * o}" x2="${cx + dx * 420 - dy * o}" y2="${cy + dy * 420 + dx * o}"/>`);
    streets.push(`<line x1="${cx + dy * 420 + dx * o}" y1="${cy - dx * 420 + dy * o}" x2="${cx - dy * 420 + dx * o}" y2="${cy + dx * 420 + dy * o}"/>`);
  }
  const spots = x.feeders.map((f, i) => {
    const a = seed * Math.PI * 2 + (i / n) * Math.PI * 2 + (fsHash(f.name) - 0.5) * (Math.PI / n);
    const share = Number(f.share || 0);
    const r = (f.youth ? 150 : 118) + 40 * (1 - Math.min(1, share / 40)) + fsHash(f.name + "r") * 16;
    return { f, x: cx + Math.cos(a) * r * 1.3, y: cy + Math.sin(a) * r * 0.74, share, a };
  });
  const roads = spots.map((s) => `<path d="M${cx} ${cy} Q ${(cx + s.x) / 2 + Math.sin(s.a) * 18} ${(cy + s.y) / 2 - Math.cos(s.a) * 18} ${s.x} ${s.y}" stroke-width="${(1.2 + s.share / 7).toFixed(1)}"/>`).join("");
  const marks = spots.map((s) => {
    const size = 7 + Math.sqrt(Math.max(1, s.f.boys)) * 0.9;
    const shape = s.f.youth
      ? `<rect x="${s.x - size}" y="${s.y - size}" width="${size * 2}" height="${size * 2}" rx="5" class="fs-map-y"/>`
      : `<circle cx="${s.x}" cy="${s.y}" r="${size}" class="fs-map-ms"/>`;
    // The label under the mark, or over it near the bottom edge, kept inside.
    const tx = Math.min(W - 96, Math.max(96, s.x));
    const below = s.y + size + 30 < H;
    const ny = below ? s.y + size + 14 : s.y - size - 19;
    const name = s.f.name.length > 26 ? s.f.name.slice(0, 25) + "…" : s.f.name;
    return `<g><title>${esc(s.f.name)}: ${s.f.boys} boys, ${s.share}% of your freshmen${s.f.players.length ? `, ${s.f.players.length} on your roster` : ""}</title>${shape}
      <text x="${s.x}" y="${s.y + 3.5}" text-anchor="middle" class="fs-map-in">${s.f.youth ? "Y" : "MS"}</text>
      <text x="${tx}" y="${ny}" text-anchor="middle" class="fs-map-name">${esc(name)}</text>
      <text x="${tx}" y="${ny + 13}" text-anchor="middle" class="fs-map-sub">${s.f.boys} boys · ${s.share}%${s.f.players.length ? ` · ${s.f.players.length} yours` : ""}</text></g>`;
  }).join("");
  const school = `<g><rect x="${cx - 46}" y="${cy - 26}" width="92" height="52" rx="10" fill="${c1}" stroke="${c2}" stroke-width="2"/>
    <rect x="${cx - 34}" y="${cy - 14}" width="68" height="28" rx="4" class="fs-map-field"/>
    ${[-22, -11, 0, 11, 22].map((d) => `<line x1="${cx + d}" y1="${cy - 12}" x2="${cx + d}" y2="${cy + 12}" class="fs-map-yard"/>`).join("")}
    <text x="${cx}" y="${cy + 44}" text-anchor="middle" class="fs-map-school">${esc(x.school.name)}</text></g>`;
  return `<div class="panel fs-map"><h2>${esc(x.town)} <small>your school and where its players start out</small></h2>
    <svg viewBox="0 0 ${W} ${H}" role="img" aria-label="Map of ${esc(x.town)}: your school, ${x.feeders.filter((f) => !f.youth).length} feeder middle schools and ${x.feeders.filter((f) => f.youth).length} youth teams">
      <defs><clipPath id="fs-map-clip"><rect width="${W}" height="${H}" rx="16"/></clipPath></defs>
      <g clip-path="url(#fs-map-clip)"><rect width="${W}" height="${H}" class="fs-map-ground"/><g class="fs-map-streets">${streets.join("")}</g>
      <g class="fs-map-roads">${roads}</g>${marks}${school}</g></svg>
    <div class="fs-map-key"><span><i class="ms"></i>Feeder middle school</span><span><i class="y"></i>Youth tackle team</span><span><i class="road"></i>The wider the road, the more of your freshmen it sends</span></div></div>`;
}
screens.area = async () => {
  const x = await api("school_area");
  setColors(x.school.colors);
  const feeders = x.feeders.map((f) => `<div class="fs-feeder">
      <div class="fs-who"><span class="fs-init">${f.youth ? "Y" : "MS"}</span><div><b>${esc(f.name)}</b><small>${f.youth ? `Youth tackle · league president ${fsWho(f.coach, f.tok)}` : `Feeder school · coach ${fsWho(f.coach, f.tok)}`} · ${f.boys} boys · ${f.share}% of your freshmen</small></div></div>
      <div class="sub">${esc(f.last)}. ${esc(f.next)}.</div>
      ${f.players.length ? `<div class="fs-names"><span class="st-label">On your roster</span> ${f.players.map((p) => `${playerLink(p.id, p.name)} <span class="sub">${esc(p.pos)} ${fsGrade(p.grade)}</span>`).join(", ")}</div>` : ""}
    </div>`).join("");
  const seasons = x.seasons.map((s) => `<tr><td class="l">${s.year}</td><td><b>${esc(s.record)}</b></td><td class="l">${s.coach_id != null ? `<a class="link" data-go="coach" data-id="${s.coach_id}">${esc(s.coach)}</a>` : ""}</td><td>${esc(s.points || "")}</td><td class="l">${s.title ? '<span class="st-pill gold">State champions</span>' : ""}</td></tr>`).join("");
  const coaches = x.coaches.map((c) => `<tr><td class="l"><a class="link" data-go="coach" data-id="${c.id}">${esc(c.name)}</a></td><td>${esc(c.years)}</td><td>${esc(c.record)}</td></tr>`).join("");
  const levels = ["NFL", "Free agent", "Draft pool", "FBS", "FCS", "D2", "NAIA", "D3", "JUCO", "College"];
  const alumniCols = [
    { k: "name", t: "Name", l: true, f: (r) => playerLink(r.id, r.name) },
    { k: "pos", t: "Pos", l: true },
    { k: "class", t: "Class", tip: "The year he finished high school here" },
    { k: "where", t: "Now", l: true, div: true, f: (r) => (r.team ? teamLink(r.team) : `<span class="muted">${esc(r.where)}</span>`) },
    { k: "level", t: "Level", l: true, v: (r) => levels.indexOf(r.level), f: (r) => (r.level === "NFL" ? '<span class="st-pill gold">NFL</span>' : esc(r.level)) },
  ];
  const pros = x.alumni.filter((a) => a.level === "NFL").length;
  return `<div class="hero small"><div class="art"></div>${logoHero(x.school) || `<div class="st-outline">${esc(initials(x.school.name))}</div>`}<div class="copy">
      <div class="st-label" style="color:rgba(255,255,255,.7)">${esc(x.line)}${x.private ? " · private" : ""}</div><h1>${esc(x.school.name)}</h1>
      <div class="meta"><span><b>${thousandsFs(x.enrollment)}</b> students</span><span>${esc(x.town)}, ${esc(x.state)}</span><span><b>${esc(x.record)}</b> all time</span><span><b>${x.titles}</b> state title${x.titles === 1 ? "" : "s"}</span></div></div></div>
    <div class="pad"><div class="grid2">
      <div style="display:grid;gap:18px;align-content:start">
        <div class="panel"><h2>The school</h2><dl class="kv">
          <dt>Enrollment</dt><dd>${thousandsFs(x.enrollment)} students${x.private ? ", a private school" : ""}</dd>
          <dt>Classification</dt><dd>${esc(x.class)}</dd><dt>Playoffs</dt><dd>${esc(x.district)}</dd>
          <dt>Mascot</dt><dd>The ${esc(x.mascot)}</dd><dt>The program</dt><dd>${esc(x.prestige)}</dd>
          <dt>Weight room and staff</dt><dd>${esc(x.resources)}</dd><dt>The area</dt><dd>${esc(x.money)}, ${esc(x.safety).toLowerCase()}</dd>
          <dt>Transfers in</dt><dd>${x.transfer > 0 ? `Sit ${Math.round(x.transfer * 100)}% of a varsity season without a family move` : "Eligible at once"}</dd></dl></div>
        <div class="panel"><h2>Seasons <small>the school's own book</small></h2>${seasons ? `<div class="tbl box"><table><thead><tr><th class="l">Year</th><th>Record</th><th class="l">Head coach</th><th>Points</th><th class="l"></th></tr></thead><tbody>${seasons}</tbody></table></div>` : '<div class="empty">No seasons kept.</div>'}</div>
        <div class="panel"><h2>Head coaches</h2>${coaches ? `<div class="tbl"><table><thead><tr><th class="l">Coach</th><th>Years</th><th>Record</th></tr></thead><tbody>${coaches}</tbody></table></div>` : '<div class="empty">No record.</div>'}</div>
      </div>
      <div style="display:grid;gap:18px;align-content:start">${x.feeders.length ? fsTownMap(x) : ""}
      <div class="panel"><h2>Where your players come from <small>${x.came_in} on the roster moved in or transferred</small></h2><div class="fs-feeders">${feeders}</div></div></div>
    </div>
    <div class="panel"><h2>Playing on <small>${x.alumni.length ? `${x.alumni.length} of the school's players in college and the pros${pros ? `, ${pros} on an NFL roster` : ""}` : "the school's players in college and the pros"}</small></h2>
      ${table("school-alumni", alumniCols, x.alumni, { groupBy: (r) => (["NFL", "Free agent", "Draft pool"].includes(r.level) ? r.level : "College · " + r.level), empty: "Nobody from the school is playing in college or the pros yet." })}</div>
    </div>`;
};
const thousandsFs = (v) => Number(v || 0).toLocaleString("en-US");

// ---------- the job market ----------
screens.market = async () => {
  const x = await api("market");
  if (!x) return `<div class="pad"><div class="h1">No career yet</div></div>`;
  const f = keep("market", { show: "fits", q: "" });
  const i = x.interest;
  const offers = x.offers.map((o) => `<div class="fs-offer">
      <div class="fs-who">${mark(o.team, 40)}<div><b>${esc(o.role)}, ${teamLink(o.team)}</b><small>${esc(o.line)} · offered ${esc(o.made)} · answer by ${esc(o.until)}</small></div></div>
      <div class="st-tiles"><div><b>${money(o.pay)}</b><small>a year</small></div><div><b>${o.years}</b><small>year${o.years === 1 ? "" : "s"}</small></div></div>
      <p>${esc(o.why)}</p><p class="sub">${esc(o.family)}</p>
      <div class="st-actions"><button class="st-btn primary" data-fs="accept" data-k="${o.k}">Take the job</button><button class="st-btn glass" data-fs="decline" data-k="${o.k}">Turn it down</button>${screens.jobpick ? linkTo("jobpick", { index: o.k }, "What you'd walk into", "st-btn glass") : ""}</div>
    </div>`).join("");
  const q = f.q.trim().toLowerCase();
  let open = x.openings.filter((o) => (f.show === "all" || o.fits || o.applied) && (!q || `${o.team.name} ${o.line} ${o.role}`.toLowerCase().includes(q)));
  const rows = open.slice(0, 250).map((o) => `<tr><td class="l"><span class="teamcell">${mark(o.team, 22)} <span>${teamLink(o.team)}<br><small class="sub">${esc(o.line)}</small></span></span></td><td class="l">${esc(o.role)}</td><td class="l">${esc(o.pay)}</td><td class="l">${o.decides ? `Decides around ${esc(o.decides)}` : "Open now"}</td>
    <td class="l">${o.applied ? '<span class="st-pill good">Your name is in</span>' : `<button class="st-btn small glass" data-fs="apply" data-kind="${o.team.ref.kind}" data-id="${o.team.ref.id}" data-role="${o.role_key}">Put your name in</button>`}</td></tr>`).join("");
  const chk = (k, t) => `<label class="fs-check"><input type="checkbox" data-fs-int="${k}" ${i[k] ? "checked" : ""}> ${esc(t)}</label>`;
  // The states you'd move to: chips you can take off, and a list to add one.
  const picked = x.states.filter((s) => i.states.includes(s.code));
  const chips = picked.map((s) => `<span class="fs-chip">${esc(s.name)}<button data-fs="state-del" data-code="${s.code}" aria-label="Remove ${esc(s.name)}">×</button></span>`).join("");
  const addState = `<select class="fs-move" data-fs-state-add><option value="">${picked.length ? "Add a state…" : "Only in one state…"}</option>${x.states.filter((s) => !i.states.includes(s.code)).map((s) => `<option value="${s.code}">${esc(s.name)}</option>`).join("")}</select>`;
  const fam = x.family;
  const known = (x.known || []).slice(0, 6).map((k) => `<div class="item"><span class="when">${esc(k.close)}</span><span><a class="link" data-go="coach" data-id="${k.id}">${esc(k.name)}</a> <span class="sub">· ${esc(k.now)}</span><br><small class="sub">${esc(k.how)}, ${esc(k.years)}</small></span></div>`).join("");
  const job = x.job;
  const none = `${f.show === "fits" ? "Nothing open of the kind you want right now." : "Nothing open right now."}<div class="fs-cycle">${x.cycle.map((c) => `<div><b>${esc(c.level)}</b><span>${esc(c.text)}</span></div>`).join("")}</div>`;
  return `<div class="pad">
    <div><div class="st-label">${esc(x.title)}${x.teaching_at ? ` · still teaching at ${esc(x.teaching_at)}` : ""}</div><div class="h1">Job market</div>
    <p class="sub">${job ? `You are ${esc(job.role.toLowerCase())} at ${teamLink(job.team)}. ${i.listening ? "You are taking calls about other jobs." : "You aren't taking calls about other jobs: searches pass you by unless you put your name in."}` : "You have no team. Searches call the coaches whose names are in and the ones who let it be known they want that kind of job; teams judge you as they judge every candidate, by your record, your reputation and who knows you."} Your reputation: <b>${esc(x.reputation)}</b>.</p>${gotoFor("market")}</div>
    ${offers ? `<div class="panel warm"><h2>Offers <small>the job waits for your answer; turn it down and it goes to the next name</small></h2><div class="fs-offers">${offers}</div></div>` : ""}
    <div class="fs-row">
      <div class="panel"><h2>What you'd take <small>where your agent puts your name</small></h2>
        ${job ? chk("listening", "Take calls about other jobs while I have one") : ""}
        <div class="fs-checks">${chk("college_head", "College head coach")}${chk("coordinator", "Coordinator")}${chk("position", "College position coach")}${chk("nfl", "NFL")}${chk("other", "Other leagues (CFL, UFL, arena)")}${chk("hs_head", "High school head coach")}</div>
        <div class="fs-states"><span class="st-label">Colleges</span><div class="seg">${[[0, "Any division"], [1, "FCS and up"], [2, "FBS only"]].map(([v, t]) => `<button data-fs="floor" data-v="${v}" aria-pressed="${(i.college_floor || 0) === v}">${t}</button>`).join("")}</div></div>
        <div class="fs-states"><span class="st-label">Where</span>${chips || '<span class="sub">Anywhere</span>'}${addState}</div>
        <small class="sub">NFL and other-league jobs go anywhere.</small></div>
      ${fam ? `<div class="panel"><h2>Your family <small>${esc(fam.mood)}</small></h2><dl class="kv"><dt>Home</dt><dd>${esc(fam.home || "")}${fam.since ? ` since ${fam.since}` : ""}${fam.moves ? ` · moved ${fam.moves} time${fam.moves === 1 ? "" : "s"}` : ""}</dd>${fam.status === "Married" && fam.spouse ? `<dt>Married to</dt><dd>${esc(fam.spouse)}</dd>` : fam.status ? `<dt>Status</dt><dd>${esc(fam.status)}</dd>` : ""}${fam.children.length ? `<dt>Children</dt><dd>${fam.children.map((c) => `${esc(c.name)}, ${c.age} <span class="sub">(${esc(c.school.toLowerCase())})</span>`).join("; ")}</dd>` : ""}</dl></div>` : ""}
      <div class="panel"><h2>People who know you <small>a search calls them about you</small></h2><div class="items">${known || '<div class="empty">Nobody in the world\'s history yet: they come with every staff you join.</div>'}</div>${(x.known || []).length > 6 ? '<a class="link sub" data-go="you">Everyone who knows you, on your profile</a>' : ""}</div>
    </div>
    <div class="panel"><h2>Open jobs <small>${open.length} ${f.show === "all" ? "in all" : "of the kind you want"}${open.length > 250 ? ", the first 250 by standing" : ""}</small></h2>
      <div class="filters"><div class="seg"><button data-fs="show" data-v="fits" aria-pressed="${f.show === "fits"}">What you'd take</button><button data-fs="show" data-v="all" aria-pressed="${f.show === "all"}">Every opening</button></div><input class="fs-q" id="fs-q" value="${esc(f.q)}" placeholder="Find a team or a job"></div>
      ${rows ? `<div class="tbl box"><table><thead><tr><th class="l">Team</th><th class="l">Job</th><th class="l">Pays</th><th class="l">Search</th><th class="l"></th></tr></thead><tbody>${rows}</tbody></table></div>` : `<div class="empty">${none}</div>`}
    </div>
    ${x.applied.length || x.log.length ? `<div class="grid2">
      <div class="panel"><h2>Your market so far</h2><div class="items">${x.log.map((l) => `<div class="item"><span class="when">${esc(l.date)}</span><span>${esc(l.text)}</span></div>`).join("") || '<div class="empty">Nothing yet.</div>'}</div></div>
      <div class="panel"><h2>Your name is in</h2><div class="items">${x.applied.map((a) => `<div class="item"><span class="when">${esc(a.on)}</span><span>${esc(a.role)}, ${teamLink(a.team)}</span></div>`).join("") || '<div class="empty">Nowhere right now.</div>'}</div></div>
    </div>` : ""}
  </div>`;
};

// ---------- actions ----------
document.addEventListener("click", async (ev) => {
  const a = ev.target.closest("[data-fs]");
  if (!a || a.disabled) return;
  const d = a.dataset;
  const top = () => ($(".view") ? $(".view").scrollTop : 0);
  try {
    switch (d.fs) {
      case "tab": S.keep.squads.t = d.t; render(); break;
      case "owner": await api("squads_owner", { owner: d.v }); toast(d.v === "staff" ? "Your staff sets the squads now." : "The squads are yours."); render(top()); break;
      case "reset": await api("reset_squads"); toast("Every squad is back on the staff's lists."); render(top()); break;
      case "sugg": { const r = await api("squad_suggestion", { id: Number(d.id), take: d.take === "1" }); toast(r.text); render(top()); break; }
      case "wish": {
        const x = await api("school_office");
        const order = x.boosters.projects.map((p) => p.k);
        const i = order.indexOf(Number(d.k)), j = i + Number(d.d);
        if (j < 0 || j >= order.length) break;
        [order[i], order[j]] = [order[j], order[i]];
        await api("booster_wish", { order });
        render(top());
        break;
      }
      case "show": S.keep.market.show = d.v; render(); break;
      case "floor": await api("market_interest", { college_floor: Number(d.v) }); render(top()); break;
      case "state-del": {
        const m = await api("market");
        await api("market_interest", { states: m.interest.states.filter((c) => c !== d.code) });
        render(top());
        break;
      }
      case "apply": {
        const r = await api("market_apply", { employer: { kind: d.kind, id: Number(d.id) }, role: d.role });
        toast(r.text);
        render(top());
        break;
      }
      case "accept": {
        const ok = await sure("Take the job?", "You leave your job today, and your family moves if the job is somewhere else.", "Take it");
        if (!ok) break;
        const r = await api("market_accept", { k: Number(d.k) });
        S.status = r.status;
        toast(r.text);
        S.back = []; S.fwd = [];
        go("home", {}, false);
        break;
      }
      case "decline": { const r = await api("market_decline", { k: Number(d.k) }); toast(r.text); S.status = await api("status"); render(top()); break; }
    }
  } catch (e) { toast(e.message); }
});
document.addEventListener("change", async (ev) => {
  const el = ev.target;
  try {
    if (el.dataset.fsMove) {
      const r = await api("set_squad", { id: Number(el.dataset.fsMove), squad: el.value });
      toast(r.text);
      render($(".view").scrollTop);
    } else if (el.dataset.fsInt) {
      await api("market_interest", { [el.dataset.fsInt]: el.checked });
      render($(".view").scrollTop);
    } else if (el.dataset.fsStateAdd !== undefined && el.value) {
      const m = await api("market");
      await api("market_interest", { states: [...m.interest.states, el.value] });
      render($(".view").scrollTop);
    }
  } catch (e) { toast(e.message); render(); }
});
document.addEventListener("input", (ev) => {
  if (ev.target.id !== "fs-q") return;
  S.keep.market.q = ev.target.value;
  clearTimeout(S.fsq);
  S.fsq = setTimeout(async () => { await render($(".view").scrollTop); const q = $("#fs-q"); if (q) { q.focus(); q.setSelectionRange(q.value.length, q.value.length); } }, 250);
});
