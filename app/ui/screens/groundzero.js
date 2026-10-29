/* Ground zero (next update, lane 10, SK68) in Press Box: the year before
 * a new team's first game. The steps ahead on one strip; then the tools of
 * the level: the other clubs' open players and your expansion draft queue,
 * the build year's squad, or the new school's zone and your home visits.
 * Every class is gz- so the app's global names never restyle it. */

I.build = '<path d="M3 21h18M5 21V10l7-5 7 5v11"/><path d="M9 21v-6h6v6"/>';
navAdd({ go: "groundzero", label: "Ground zero", icon: "build", sec: "top", after: "calendar", show: () => !!(S.status && S.status.ground) });

screens.groundzero = async () => {
  const x = await api("gz_page", {});
  title("Ground zero", x.team.name);
  const steps = `<ol class="gz-strip">${x.steps.map((s) => `<li class="${s.done ? "done" : ""}"><span>${esc(s.when)}</span><b>${esc(s.what)}</b></li>`).join("")}</ol>`;
  const sub = x.kind === "nfl" ? `Expansion club · first season ${x.first}` : x.kind === "college" ? `${x.restart ? "Football is back" : "A new program"} · build year ${x.build} · first games ${x.first}` : `A new school · opens ${x.opens}`;
  const head = `<div class="gz-head">${mark(x.team, 44)}<div><div class="st-label">Ground zero</div><h1>${esc(x.team.name)}</h1><small>${esc(sub)}</small></div></div>`;
  const body = x.kind === "nfl" ? gzNfl(x) : x.kind === "college" ? gzCollege(x) : gzSchool(x);
  return `<div class="pad sl gz">${head}${steps}${body}</div>`;
};
GOTO.groundzero = () => [["Staff", "staff"], ["Calendar", "calendar"], ["News", "news"]];

// The Handbook's page (lane 8's hbAdd), from docs/reference/how-it-works/new-teams.md.
if (typeof hbAdd === "function") {
  hbAdd({
    id: "newteams", group: "Building the team", title: "New teams",
    lede: "After 2026 the NFL adds clubs, colleges start football and growing suburbs open high schools, at about the real rates. You can start at one, or take its job later, a year before its first game.",
    how: [
      ["The NFL expands rarely", "The owners vote at their annual meeting, about three times in 56 years, and add two clubs that play two seasons later. In their first February the other clubs list their open players and the new clubs take turns in an expansion draft."],
      ["Colleges start about six a year", "Mostly in the NAIA and Division III. A build year comes first: the team signs a class, practices and holds walk-on tryouts, and plays no games until the next season."],
      ["Schools open where families move", "Districts in states whose young population grows split a big school. On the first day the zone's freshmen and sophomores move, juniors choose, and seniors stay where they started."],
      ["Your staff reads it first", "The open players carry your staff's grades and its guess at the lists; a junior's chance to come shows before the first day, and a home visit helps."],
    ],
    lv: {
      hs: "A new school plays its first varsity season with freshmen and sophomores, with no district until the next realignment. You can visit up to 12 zoned juniors' families.",
      college: "A Division I school starts at FCS, as an independent. Walk-on tryouts on summer enrollment day take the best unsigned seniors and up to 12 junior college players.",
      nfl: "The new clubs pick first and last in every round of their first draft. A club stops drafting when no listed player beats the best free agent at his spot, at 42 players or at 55% of the cap.",
    },
    where: [["Ground zero", "groundzero"], ["Staff", "staff"], ["News", "news"]],
    rel: ["draft", "recruiting", "people"],
  });
  hbScreen("groundzero", "newteams", [
    ["The steps", "The year to the first game, with the days that matter.", ""],
    ["The open players", "The other clubs' lists, your staff's grade and your queue for the expansion draft.", "Club"],
    ["The squad", "Who is on the build year's squad, signed and on the way, and committed for next year.", "The squad"],
    ["Your zone", "The players in the new school's zone and each one's chance to come.", "Your zone"],
  ]);
}

function gzNfl(x) {
  const q = x.queue || [];
  const byId = Object.fromEntries((x.open || []).map((p) => [p.id, p]));
  const rows = (x.open || []).map((p) => {
    const at = q.indexOf(p.id);
    return `<tr><td>${esc(p.club)}</td><td>${playerLink(p.id, p.name)}</td><td>${esc(p.pos)}</td><td class="n">${p.age}</td><td class="n">${p.grade}</td>
      <td class="n">${x.drafted ? "" : at >= 0 ? `<button class="gz-btn on" data-act="gzqdel" data-id="${p.id}">#${at + 1} · take off</button>` : `<button class="gz-btn" data-act="gzqadd" data-id="${p.id}">Queue</button>`}</td></tr>`;
  }).join("");
  const what = x.drafted ? "The expansion draft is done" : x.lists_in ? `The lists are in: the expansion draft is ${esc(x.draft_day)}` : `Your staff's guess: the lists come in ${esc(x.lists_day)}`;
  const open = `<section class="gz-panel"><div class="gz-ph"><b>The other clubs' open players</b><span>${what} · ${(x.open || []).length} players</span></div>
    ${rows ? `<div class="gz-tbl"><table><thead><tr><th>Club</th><th>Player</th><th>Pos</th><th class="n">Age</th><th class="n">Staff grade</th><th></th></tr></thead><tbody>${rows}</tbody></table></div>` : `<div class="gz-empty">${x.drafted ? "Your picks are on the squad." : "No list yet."}</div>`}</section>`;
  const queue = q.length ? q.map((id, k) => { const p = byId[id]; return `<li><span>${k + 1}</span>${p ? `${playerLink(p.id, p.name)} <small>${esc(p.pos)} · ${esc(p.club)}</small>` : `<small>Off the list</small>`}${k ? `<button class="gz-btn" data-act="gzqup" data-id="${id}" aria-label="Move up">↑</button>` : ""}</li>`; }).join("") : "";
  const taken = (x.taken || []).map((p) => `<li>${playerLink(p.id, p.name)} <small>${esc(p.pos)} · ${p.age}</small></li>`).join("");
  const side = `<section class="gz-panel"><div class="gz-ph"><b>${x.drafted ? "Your picks" : "Your queue"}</b><span>${x.drafted ? `${(x.taken || []).length} players` : "your staff takes the rest by grade"}</span></div>
    ${x.drafted ? `<ol class="gz-list">${taken}</ol>` : queue ? `<ol class="gz-list">${queue}</ol>` : `<div class="gz-empty">Queue the players you want first. On draft day your picks follow this order; when it runs dry your staff picks by its grades.</div>`}</section>
    <section class="gz-panel"><div class="gz-ph"><b>The club</b></div><div class="gz-kv"><span>Fee</span><span>$${(x.fee / 1e9).toFixed(1)} billion to the other owners</span><span>Market</span><span>${x.metro.toFixed(1)} million people</span>${x.partner.length ? `<span>Joining with you</span><span>${x.partner.map((t) => teamLink(t)).join(", ")}</span>` : ""}<span>The rule</span><span>Each club protects its best and lists ${x.partner.length ? 6 : 5} open players; when one loses a player it pulls one more back.</span></div></section>`;
  return `<div class="gz-main">${open}<div class="gz-side">${side}</div></div>`;
}

function gzCollege(x) {
  const n = (v, t) => `<div class="gz-stat"><b>${v}</b><span>${esc(t)}</span></div>`;
  return `<div class="gz-main"><section class="gz-panel"><div class="gz-ph"><b>The squad</b><span>${x.building ? "the build year: practices, no games" : x.first > S.status.season ? "before the build year" : "playing"}</span></div>
    <div class="gz-stats">${n(x.squad, "on the squad")}${n(x.coming, "signed and on the way")}${n(x.committed, "committed for next year")}</div>
    <p class="gz-note">Walk-on tryouts and junior college finds come ${esc(x.tryouts)}, before summer school: the best seniors nobody signed, nearest first. Portal players can come in the spring window. The first class signs with nothing on the field.</p></section>
    <div class="gz-side"><section class="gz-panel"><div class="gz-ph"><b>Where to work</b></div><div class="gz-kv"><span>Players</span><span><a class="link" data-go="recruiting">Recruiting</a> and <a class="link" data-go="find_prospects">find prospects</a></span><span>Staff</span><span><a class="link" data-go="staff">Hire your staff</a></span></div></section></div></div>`;
}

function gzSchool(x) {
  if (x.zoned) {
    return `<div class="gz-main"><section class="gz-panel"><div class="gz-ph"><b>The first day</b><span>the zone's families have chosen</span></div>
      <div class="gz-stats"><div class="gz-stat"><b>${x.moved}</b><span>moved to you</span></div><div class="gz-stat"><b>${x.stayed}</b><span>chose to stay</span></div></div></section>
      <div class="gz-side"><section class="gz-panel"><div class="gz-ph"><b>Your squad</b></div><div class="gz-kv"><span>Players</span><span><a class="link" data-go="roster">The squad</a></span></div></section></div></div>`;
  }
  const rows = x.zone.map((p) => `<tr><td>${playerLink(p.id, p.name)}</td><td>${esc(p.pos)}</td><td>${esc(p.grade)}</td><td class="n">${p.grade === "Senior" ? '<span class="gz-dim">Stays</span>' : p.chance >= 100 ? "Moves" : `${p.chance}%`}</td>
    <td class="n">${p.grade === "Junior" ? (p.pitched ? '<span class="gz-on">Visited</span>' : `<button class="gz-btn" data-act="gzpitch" data-id="${p.id}">Visit his family</button>`) : ""}</td></tr>`).join("");
  return `<div class="gz-main"><section class="gz-panel"><div class="gz-ph"><b>Your zone</b><span>${x.zone.length} players at ${x.parents.map((t) => esc(t.name)).join(", ")} live in it · the first day is ${esc(x.class_day || "in August")}</span></div>
    ${rows ? `<div class="gz-tbl"><table><thead><tr><th>Player</th><th>Pos</th><th>Grade</th><th class="n">Chance to come</th><th></th></tr></thead><tbody>${rows}</tbody></table></div>` : `<div class="gz-empty">No players live in the zone yet.</div>`}</section>
    <div class="gz-side"><section class="gz-panel"><div class="gz-ph"><b>The families choose</b><span>${x.pitched} of ${x.pitches} home visits</span></div>
      <p class="gz-note">Freshmen and sophomores in the zone must move. Juniors choose: a starter where he is tends to stay, a backup goes where he'd play, and your visit helps. Seniors finish where they started. The new freshmen come with the class.</p>
      <div class="gz-kv"><span>Old school</span><span>${x.parents.map((t) => teamLink(t)).join(", ")}</span></div></section></div></div>`;
}

Object.assign(CORE_MORE, {
  gzqadd: async (el) => { const x = await api("gz_page", {}); const q = (x.queue || []).filter((i) => i !== Number(el.dataset.id)); q.push(Number(el.dataset.id)); await api("gz_queue", { ids: q }); refresh(el); },
  gzqdel: async (el) => { const x = await api("gz_page", {}); await api("gz_queue", { ids: (x.queue || []).filter((i) => i !== Number(el.dataset.id)) }); refresh(el); },
  gzqup: async (el) => { const x = await api("gz_page", {}); const q = x.queue || []; const k = q.indexOf(Number(el.dataset.id)); if (k > 0) { [q[k - 1], q[k]] = [q[k], q[k - 1]]; await api("gz_queue", { ids: q }); } refresh(el); },
  gzpitch: async (el) => { try { await api("gz_pitch", { id: Number(el.dataset.id) }); } catch (e) { toast(e.message); } refresh(el); },
});
