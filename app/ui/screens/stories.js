// Lane screens: stories (docs/final-build/plan.md). Registers screens.<name>.
"use strict";
/* Lane 8 · stories (final build): the stories around the football.
 *
 *   office       Your office (C6): the shelves the years fill (rings,
 *                trophies, game balls, covers, plaques, photos, gifts), in
 *                your team's colours, and the road it travelled with you.
 *   story        Your career story (A14 item 2): a profile's timeline,
 *                every season and title, the names they gave you, your
 *                fourth downs as the bots read them.
 *   media        The media desk: the front page, the morning-after grades
 *                (P5), the schedule (P6, P7), the preseason annuals (C7),
 *                the video game's ratings (C10), fantasy (C11), jersey sales
 *                (C12), the shows (C3, C9) and the nicknames (A14 4-20).
 *   traditions   Rivalry trophies (C1), uniforms you pick each week (C4),
 *                honors (C5), the pregame show (C3), stormings and fines
 *                (C2), farewells and one-day contracts (C15, C16),
 *                celebrations (C17), parades and rings (C13, C14).
 *   Player page  A "Story" tab: his names, the public's read of him (the
 *                video game, fantasy, covers, jerseys), his honors and
 *                named plays; name your own players from it.
 */

I.mic = '<rect x="9" y="3" width="6" height="11" rx="3"/><path d="M5.5 11a6.5 6.5 0 0 0 13 0M12 17.5V21M8.5 21h7"/>';
I.flag = '<path d="M5 21V4M5 4.5h11.5l-2.2 4 2.2 4H5"/>';
I.story = '<path d="M5 4.5A1.5 1.5 0 0 1 6.5 3H19v15H6.5A1.5 1.5 0 0 0 5 19.5zM5 19.5A1.5 1.5 0 0 0 6.5 21H19v-3M8.5 7h7M8.5 10.5h7M8.5 14h4"/>';
I.office = '<path d="M3 20.5h18M4.5 20.5V4h15v16.5M4.5 12h15M7.5 12V8.5h2.5V12M13.5 12V7.5h2V12M8 20.5v-4.5h3v4.5M14 20.5V17h3v3.5"/>';

navAdd({ go: "media", label: "Media", icon: "mic", sec: "world", after: "news" });
navAdd({ go: "traditions", label: "Traditions", icon: "flag", sec: "team", after: "people" });
navAdd({ go: "story", label: "Career story", icon: "story", sec: "world", after: "you" });
navAdd({ go: "office", label: "Office", icon: "office", sec: "world", after: "story" });
Object.assign(CRUMBS, { office: "Office", story: "Career story", media: "Media", traditions: "Traditions" });

// ---------- small pieces ----------
const soWho = (w) => (w ? `${rich(w.link)}${w.nick ? ` <span class="so-nick">“${esc(w.nick)}”</span>` : ""}` : "");
const soWhoPos = (w) => (w ? `${soWho(w)}${w.pos ? ` <small class="muted">${esc(w.pos)}</small>` : ""}` : "");
const soTeam = (t) => (t ? teamChip(t, 18) : "");
const soRec = (w, l, t) => `${w}-${l}${t ? `-${t}` : ""}`;
const soPct = (v) => (v == null ? "" : String(v.toFixed(3)).replace(/^0/, ""));
const soPlural = (n, one, many) => `${n} ${n === 1 ? one : many || one + "s"}`;
const soEmpty = (t) => `<div class="empty">${esc(t)}</div>`;
const soPanel = (h, body, sub) => `<section class="panel"><h2>${h}${sub ? ` <small>${sub}</small>` : ""}</h2>${body}</section>`;
const soMoney = (n) => (n >= 1e6 ? `$${(n / 1e6).toFixed(n % 1e6 ? 2 : 0)}M` : n >= 1000 ? `$${Math.round(n / 1000)}K` : `$${n}`);

// Each shelf's object, drawn in line (C6).
const SO_GLYPH = {
  ring: '<ellipse cx="24" cy="30" rx="13" ry="11"/><ellipse cx="24" cy="30" rx="8" ry="6.5"/><path d="M15 21l4-9h10l4 9"/><path d="M19 12l5-4 5 4"/>',
  trophy: '<path d="M15 8h18v9a9 9 0 0 1-18 0z"/><path d="M15 11H9a6 6 0 0 0 6 8M33 11h6a6 6 0 0 1-6 8M24 26v7M17 40h14M19 33h10v7H19z"/>',
  gameball: '<path d="M8 24c4-10 12-15 16-15s12 5 16 15c-4 10-12 15-16 15S12 34 8 24z"/><path d="M16 24h16M20 21v6M24 21v6M28 21v6"/>',
  cover: '<rect x="11" y="6" width="26" height="36" rx="2"/><path d="M15 12h18M15 16h10"/><circle cx="24" cy="28" r="6"/><path d="M17 40c1-5 4-7 7-7s6 2 7 7"/>',
  plaque: '<path d="M10 8h28v32H10z"/><path d="M15 14h18M15 19h18M15 24h12"/><circle cx="24" cy="33" r="3"/>',
  photo: '<rect x="7" y="10" width="34" height="28" rx="2"/><rect x="11" y="14" width="26" height="20"/><path d="M11 30l7-7 6 6 4-4 9 9"/><circle cx="30" cy="19" r="2"/>',
  gift: '<rect x="9" y="19" width="30" height="21" rx="1.5"/><path d="M7 14h34v5H7zM24 14v26M24 14c-3-6-11-6-9-1 1 2 6 1 9 1zM24 14c3-6 11-6 9-1-1 2-6 1-9 1z"/>',
};
const soGlyph = (k, cls = "") => `<svg class="so-glyph ${cls}" viewBox="0 0 48 48" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${SO_GLYPH[k] || SO_GLYPH.plaque}</svg>`;

// The asks that wait on you: a one-day contract, a documentary.
function soAsks(asks) {
  if (!asks || !asks.length) return "";
  return `<div class="so-asks">${asks.map((q) => `<div class="so-ask ${q.answered ? "done" : ""}">
    <div class="so-ask-copy"><div class="st-label">Needs your answer${q.answered ? "" : ` · by ${esc(q.by)}`}</div><b>${esc(q.title)}</b><p>${rich(q.text)}</p></div>
    <div class="so-ask-act">${q.answered ? `<span class="tag gold">${esc(q.answered)}</span>` : `<button class="st-btn small primary" data-act="soans" data-what="${q.what}" data-yes="1">${esc(q.yes)}</button><button class="st-btn small glass" data-act="soans" data-what="${q.what}" data-yes="0">${esc(q.no)}</button>`}</div></div>`).join("")}</div>`;
}

// A nickname as a card: the name, who, how it came and how far it went.
function soNickCard(n, opts = {}) {
  const spread = [0, 1, 2, 3].map((k) => `<i class="${!n.retired && n.spread >= k ? "on" : ""}" title="${["Coined", "Local", "National", "The booth"][k]}"></i>`).join("");
  const tags = [
    `<span class="tag ${n.kind === "Mocking" ? "due" : n.kind === "Earned" || n.kind === "Yours" ? "gold" : "soft"}">${esc(n.kind)}</span>`,
    n.merch ? '<span class="tag gold" title="On shirts">Merch</span>' : "",
    n.reaction ? `<span class="tag soft">${esc(n.reaction)}</span>` : "",
  ].join("");
  return `<article class="so-nick-card ${n.retired ? "gone" : ""}">
    <div class="so-nick-top"><span class="so-nick-name">“${esc(n.name)}”</span><span class="so-spread" title="${esc(n.stage)}">${spread}</span></div>
    ${opts.noWho ? "" : `<div class="so-nick-who">${rich(n.who.link)}${n.who.pos ? ` <small class="muted">${esc(n.who.pos)}</small>` : ""}${n.team ? ` · ${soTeam(n.team)}` : ""}</div>`}
    <p>${esc(n.moment ? n.moment.charAt(0).toUpperCase() + n.moment.slice(1) : "")}${n.moment ? "." : ""} <span class="muted">${esc(n.by.charAt(0).toUpperCase() + n.by.slice(1))}${n.outlet ? `, ${esc(n.outlet)}` : ""} · ${esc(n.coined)}</span></p>
    ${n.heir ? `<p class="small muted">The new ${rich(n.heir.link)}</p>` : ""}${n.replaced ? `<p class="small muted">Replaced “${esc(n.replaced)}”</p>` : ""}
    <div class="so-nick-tags">${tags}<span class="muted small">${esc(n.stage)}</span></div></article>`;
}
const soNicks = (list, empty, opts) => (list && list.length ? `<div class="so-nicks">${list.map((n) => soNickCard(n, opts)).join("")}</div>` : soEmpty(empty));

// ---------- the office (C6) ----------
screens.office = async (a) => {
  const x = await api("stories_office");
  title("Office");
  if (x.none) return `<div class="pad">${pageHead({ title: "Your office", goto: gotoFor("office", a, x) })}${soEmpty("You need a job first: the office comes with it.")}</div>`;
  const c = (x.team && x.team.colors) || ["#2b3140", "#11141b"];
  const count = (k) => (x.shelves.find((s) => s.key === k) || { items: [] }).items.length;
  const tiles = [["ring", "Rings"], ["trophy", "Trophies"], ["gameball", "Game balls"], ["cover", "Covers"], ["photo", "Photos"], ["gift", "Gifts"], ["plaque", "Plaques"]]
    .map(([k, t]) => `<div><b>${count(k)}</b><small>${t}</small></div>`).join("");
  const st = keep("soshelf", {});
  const shelf = (s) => {
    const all = s.items;
    const open = st[s.key];
    const shown = open ? all : all.slice(0, 10);
    const items = shown.map((m) => `<figure class="so-obj so-${s.key}" tabindex="0">${soGlyph(s.key)}<figcaption><b>${m.year}</b><span>${rich(m.text)}</span>${m.team ? `<small>${esc(m.team.name)}</small>` : ""}</figcaption></figure>`).join("");
    const none = { ring: "Room for your first ring.", trophy: "A title or a rivalry trophy goes here.", gameball: "The players hand you a ball after the big wins.", cover: "A magazine cover lands here when one puts you on it.", photo: "Photos from the days that mattered.", gift: "What people give you over the years.", plaque: "Plaques and honors." }[s.key];
    const more = all.length > 10 ? `<button class="so-more" data-act="soshelf" data-k="${s.key}">${open ? "Fewer" : `All ${all.length}`}</button>` : "";
    return `<div class="so-shelf"><div class="so-shelf-head"><span class="st-label">${esc({ ring: "Rings", trophy: "Trophies", gameball: "Game balls", cover: "Magazine covers", photo: "Photos", gift: "Gifts", plaque: "Plaques" }[s.key] || s.kind)}</span><small>${all.length || ""}</small>${more}</div><div class="so-shelf-row">${items || `<div class="so-shelf-none">${esc(none)}</div>`}</div><div class="so-board"></div></div>`;
  };
  const road = x.stints.length
    ? `<div class="items">${x.stints.slice().reverse().map((s) => `<div class="item"><span class="when">${s.now ? `${s.from}-now` : s.from === s.to ? s.from : `${s.from}-${s.to}`}</span><span>${soTeam(s.team)}${s.now ? ' <span class="tag soft">Now</span>' : ""}</span></div>`).join("")}</div>`
    : soEmpty("Your first season goes here.");
  const banners = x.titles.length ? `<div class="so-banners">${x.titles.map((t) => `<div class="so-banner" style="--c1:${t.team.colors[0]};--c2:${t.team.colors[1] || t.team.colors[0]}"><b>${t.year}</b><span>${esc(t.what)}</span><small>${esc(t.team.name)}</small></div>`).join("")}</div>` : soEmpty("No titles yet.");
  const moves = x.moves.length ? `<div class="items">${x.moves.slice().reverse().map((m) => `<div class="item"><span class="when">${esc(m.date)}</span><span>${rich(m.text)}</span></div>`).join("")}</div>` : soEmpty("You haven't moved yet. When you do, the boxes go with you.");
  const sub = x.count ? `${soPlural(x.count, "thing")} on the shelves${x.since ? ` · collecting since ${esc(x.since)}` : ""} · ${soPlural(x.wins, "career win")}` : "Bare walls for now. The years fill them.";
  return `<div class="pad">${pageHead({ kicker: x.team ? teamA(x.team) : "", title: "Your office", sub, goto: gotoFor("office", a, x) })}
    <div class="st-tiles so-tiles">${tiles}</div>
    <div class="grid2 so-office"><div class="so-room" style="--c1:${c[0]};--c2:${c[1] || c[0]}">${x.shelves.map(shelf).join("")}</div>
    <div class="col">${soPanel("Banners", banners, x.titles.length ? soPlural(x.titles.length, "title") : "")}${soPanel("Where the office has been", road)}${soPanel("Moves", moves)}</div></div></div>`;
};
GOTO.office = () => [["Career story", "story"], ["Your profile", "you"], ["Traditions", "traditions"], ["Media", "media"]];

// ---------- your career story (A14 item 2) ----------
const SO_FILTERS = [
  ["all", "Everything", null],
  ["big", "Big moments", ["First win", "Milestone", "Title", "Upset", "Rivalry", "Big win"]],
  ["titles", "Titles", ["Title"]],
  ["moves", "Moves", ["Hired", "Moved on"]],
  ["players", "Your players", ["Drafted", "Honor"]],
  ["names", "Names", ["Nickname"]],
  ["seasons", "Seasons", ["Season", "Hard year"]],
];
screens.story = async (a) => {
  const x = await api("stories_story");
  if (x.none) return `<div class="pad">${pageHead({ title: "Career story" })}${soEmpty("You need a job first: the story starts with it.")}</div>`;
  const list = ["Timeline", "Seasons", "Titles", { name: "Names", badge: x.names.length || "" }, "Fourth downs"];
  const tab = tabOf("sostory", list.map((t) => (typeof t === "string" ? t : t.name)), a);
  title("Career story", tab);
  const r = x.record;
  const games = r.w + r.l + r.t;
  const tiles = `<div class="st-tiles so-tiles">
    <div><b>${soRec(r.w, r.l, r.t)}</b><small>head coach record</small></div>
    <div><b>${games ? soPct((r.w + r.t / 2) / games) : "-"}</b><small>winning share</small></div>
    <div><b>${x.titles.length}</b><small>${x.titles.length === 1 ? "title" : "titles"}</small></div>
    <div><b>${x.players}</b><small>players coached</small></div>
    <div><b>${x.drafted}</b><small>sent to the draft</small></div>
    <div><b>${x.teams.length}</b><small>${x.teams.length === 1 ? "team" : "teams"}</small></div></div>`;
  let body = "";
  if (tab === "Timeline") {
    const f = keep("sofilter", { v: "all" });
    const kinds = (SO_FILTERS.find((z) => z[0] === f.v) || SO_FILTERS[0])[2];
    const shown = x.entries.filter((e) => !kinds || kinds.includes(e.kind)).reverse();
    const seg = `<div class="seg wrap">${SO_FILTERS.map(([v, t]) => `<button data-act="sofilter" data-v="${v}" aria-pressed="${f.v === v}">${t}</button>`).join("")}</div>`;
    let year = null;
    let rows = "";
    for (const e of shown) {
      if (e.year !== year) { year = e.year; rows += `<div class="so-year"><span>${year}</span></div>`; }
      const tagc = ["Title", "First win", "Milestone", "Big win"].includes(e.kind) ? "gold" : e.kind === "Hard year" ? "due" : "soft";
      rows += `<div class="so-entry kind-${esc(e.kind.toLowerCase().replace(/\s+/g, "-"))}"><span class="when">${esc(e.date.replace(/ \d{4}$/, ""))}</span><span class="so-entry-copy"><span class="tag ${tagc}">${esc(e.kind)}</span> ${rich(e.text)}</span>${e.team ? `<span class="so-entry-team">${mark(e.team, 18)}</span>` : "<span></span>"}</div>`;
    }
    body = `<div class="so-bar">${seg}<span class="muted small">${soPlural(shown.length, "moment")}</span></div><div class="so-timeline">${rows || soEmpty("Nothing here yet. Your story is written as it happens: the first win, the upsets, the trophies, the players you send on.")}</div>`;
  } else if (tab === "Seasons") {
    body = table("soseasons", [
      { k: "year", t: "Season" },
      { k: "team", t: "Team", l: true, f: (s) => soTeam(s.team), v: (s) => s.team.name },
      { k: "level", t: "Level", l: true },
      { k: "w", t: "W" },
      { k: "l", t: "L" },
      { k: "t", t: "T" },
      { k: "pct", t: "Pct", f: (s) => soPct(s.pct) },
      { k: "title", t: "Title", l: true, f: (s) => (s.title ? `<span class="tag gold">${esc(s.title)}</span>` : ""), v: (s) => s.title || "" },
    ], x.seasons, { empty: "Your first season goes here when it ends.", sort: "year" });
  } else if (tab === "Titles") {
    body = x.titles.length
      ? `<div class="so-banners big">${x.titles.map((t) => `<div class="so-banner" style="--c1:${t.team.colors[0]};--c2:${t.team.colors[1] || t.team.colors[0]}"><b>${t.year}</b><span>${esc(t.what)}</span><small>${teamA(t.team)}</small></div>`).join("")}</div>`
      : soEmpty("No titles yet. Every one you win hangs here.");
  } else if (tab === "Names") {
    body = soPanel("What they call you", soNicks(x.names, "No name has stuck to you yet. Coaches earn theirs over years: a style, a habit, a big call that worked.", { noWho: true }));
  } else {
    const fd = x.fourth;
    body = `<div class="st-tiles so-tiles"><div><b>${fd.calls}</b><small>fourth downs graded</small></div><div><b>${fd.calls ? Math.round((fd.agreed / fd.calls) * 100) + "%" : "-"}</b><small>matched the bot's call</small></div><div><b>${fd.given}</b><small>win-chance points given up</small></div><div><b>${fd.go_rate != null ? fd.go_rate + "%" : "-"}</b><small>went for it when it said go, this season</small></div></div>
      <p class="muted">The morning after every game the analytics crowd grades your fourth downs and late timeouts. ${linkTo("media", { tab: "Grades" }, "Read every report")}</p>`;
  }
  const kicker = x.team ? `Career story · ${teamA(x.team)}` : "Career story";
  const sub = [`Head coach since ${esc(x.since)}`, x.years ? soPlural(x.years, "year") : "first year", x.teams.map((t) => teamA(t)).join(", ")].filter(Boolean).join(" · ");
  return `<div class="pad">${pageHead({ kicker, title: `${esc(x.you)}${x.nick ? ` <span class="pnick">“${esc(x.nick)}”</span>` : ""}`, sub, goto: gotoFor("story", a, x) })}${tiles}${tabs("sostory", list, tab)}<div class="tabbody">${body}</div></div>`;
};
GOTO.story = () => [["Office", "office"], ["Your profile", "you"], ["Fourth downs", "media", { tab: "Grades" }], ["History", "history"]];

// ---------- the media desk ----------
screens.media = async (a) => {
  const k = keep("somedia", { t: "" });
  const x = await api("stories_media", { tab: a.tab || k.t, year: keep("soyear", {}).y });
  const tab = tabOf("somedia", x.tabs, { tab: x.tab });
  title("Media", tab);
  const b = x.body;
  const body = {
    "Front page": () => soFront(b),
    Grades: () => soGrades(b),
    Schedule: () => soSchedule(b),
    Annuals: () => soAnnuals(b),
    Ratings: () => soRatings(b),
    Fantasy: () => soFantasy(b),
    Jerseys: () => soJerseys(b),
    Shows: () => soShows(b),
    Nicknames: () => soNicknames(b),
  }[tab]();
  const sub = `${esc(x.date)} · what the papers, the shows and the public say${x.team ? ` about ${teamA(x.team)}` : ""}`;
  return `<div class="pad">${pageHead({ kicker: x.team ? teamA(x.team) : "The world", title: "Media", sub, goto: gotoFor("media", a, x) })}${soAsks(x.asks)}${tabs("somedia", x.tabs, tab)}<div class="tabbody">${body}</div></div>`;
};
GOTO.media = () => [["News", "news"], ["Media mail", "inbox", { key: "Media" }], ["Career story", "story"], ["Traditions", "traditions"]];

function soReport(r, open) {
  const calls = r.calls.length
    ? `<div class="tbl warm box"><table><thead><tr><th>Q</th><th>Clock</th><th class="l">Situation</th><th>Score</th><th class="l">You</th><th class="l">${esc(r.bot)}</th><th title="Win chance given up, in points">Cost</th><th class="l">What happened</th></tr></thead><tbody>${r.calls.map((c) => `<tr class="${c.agree ? "" : "so-off"}"><td>${c.q}</td><td>${esc(c.clock)}</td><td class="l">${esc(c.sit)}</td><td>${c.lead > 0 ? "+" + c.lead : c.lead}</td><td class="l">${esc(c.chose)}</td><td class="l">${c.agree ? '<span class="tag gold">Agreed</span>' : esc(c.bot)}</td><td>${c.cost ? c.cost.toFixed(1) : c.edge ? `<span class="muted">+${c.edge.toFixed(1)}</span>` : "-"}</td><td class="l">${esc(c.result)}</td></tr>`).join("")}</tbody></table></div>`
    : '<p class="muted small">No fourth-down decision in this one.</p>';
  const tone = r.tone === "Harsh" ? "due" : r.tone === "Praise" ? "gold" : "soft";
  return `<article class="so-report ${open ? "open" : ""}">
    <header><span class="so-score ${r.won ? "w" : "l"}">${r.won ? "W" : "L"} ${r.score[0]}-${r.score[1]}</span><span>vs ${soTeam(r.opp)}</span><span class="muted small">${esc(r.date)}</span><span class="tspace"></span><span class="tag ${tone}">${esc(r.tone)}</span>${r.given ? `<span class="muted small">${r.given.toFixed(1)} pts given up</span>` : ""}</header>
    ${r.lines.length ? `<div class="so-quotes">${r.lines.map((l) => `<p>${rich(l)}</p>`).join("")}</div>` : ""}
    ${calls}${r.timeouts ? `<p class="small muted">Lost holding ${soPlural(r.timeouts, "timeout")}.</p>` : ""}</article>`;
}

function soFront(b) {
  const grade = b.grade ? soReport(b.grade, true) : soEmpty("After your first game the analytics crowd grades your fourth downs here, the next morning.");
  const annual = b.annuals.length
    ? `<div class="items">${b.annuals.map((m) => `<div class="item"><span class="when">${m.rank ? `<b class="so-rank">${m.rank}</b>` : "Unranked"}</span><span><b>${esc(m.title)}</b>${m.rank ? ` picks you ${m.rank} of ${m.of}` : " leaves you out"}.${m.yours.length ? ` Yours inside: ${m.yours.map(soWho).join(", ")}.` : ""}</span></div>`).join("")}</div>`
    : soEmpty("The preseason annuals come out in the summer.");
  const rated = b.rated.length
    ? `<div class="items">${b.rated.map((r) => `<div class="item so-rated"><span class="when"><b class="so-ovr">${r.rating}</b></span><span>${soWhoPos(r.who)}</span></div>`).join("")}</div>`
    : soEmpty("The ratings come out each August.");
  const jerseys = b.jerseys.length ? `<div class="items">${b.jerseys.map((j) => `<div class="item"><span class="when">#${j.rank} · ${j.year}</span><span>${soWho(j.who)} <small class="muted">${esc(j.list)}</small></span></div>`).join("")}</div>` : soEmpty("None of yours on the latest lists.");
  const fantasy = b.fantasy.length ? `<div class="items">${b.fantasy.map((f) => `<div class="item"><span class="when">${f.rank ? `${f.who.pos}${f.rank}` : ""}</span><span>${soWho(f.who)} <small class="muted">${f.pts} pts</small></span></div>`).join("")}</div>` : "";
  const docs = b.docs.length ? `<div class="items">${b.docs.map((d) => `<div class="item"><span class="when">${d.year}</span><span><b>${esc(d.show)}</b> · ${esc(d.how)}${d.episodes ? ` · ${soPlural(d.episodes, "episode")}` : ""}${d.stars.length ? `<br><small class="muted">Breakouts: ${d.stars.map(soWho).join(", ")}</small>` : ""}</span></div>`).join("")}</div>` : "";
  const sched = b.schedule ? `<div class="st-tiles so-tiles"><div><b>${b.schedule.prime}</b><small>prime-time games</small></div><div><b>${b.schedule.short}</b><small>short weeks</small></div>${b.schedule.abroad ? `<div><b class="so-small">${esc(b.schedule.abroad)}</b><small>abroad</small></div>` : ""}</div>` : "";
  const names = soPanel("Your names", soNicks(b.names, "No nicknames on your team yet. They come from the big moments, and from you."));
  if (b.level === "hs") {
    const rivals = b.rivals.length
      ? `<div class="items">${b.rivals.map((r) => `<div class="item"><span class="when">${r.w}-${r.l}</span><span><b>${esc(r.game)}</b>${r.trophy ? ` · ${esc(r.trophy)}` : ""}<br><small class="muted">${r.holder ? `${esc(r.holder.name)} holds it` : "Not played yet"}${r.last ? ` · last played ${r.last}` : ""}</small></span></div>`).join("")}</div>`
      : soEmpty("Your school's rivalry is set at the start of the season.");
    return `<div class="grid2"><div class="col">${names}</div><div class="col">${soPanel("The rivalry", rivals)}${soPanel("Names around the state", soNicks(b.around, "No names around the state yet. The local papers coin them after the big Friday nights."))}</div></div>`;
  }
  return `<div class="grid2"><div class="col">${b.bots || b.grade ? soPanel("The morning after", grade, b.grade ? esc(b.grade.bot) : "") : ""}${names}</div>
    <div class="col">${soPanel("The annuals", annual)}${soPanel("Video game ratings", rated, b.vg ? esc(b.vg) : "")}${soPanel("Jersey sales", jerseys)}${fantasy ? soPanel("Fantasy", fantasy, "your best") : ""}${docs ? soPanel("Cameras", docs) : ""}${sched ? soPanel("Your schedule", sched) : ""}</div></div>`;
}

function soGrades(b) {
  const s = b.season;
  const tiles = `<div class="st-tiles so-tiles"><div><b>${s.calls}</b><small>fourth downs this season</small></div><div><b>${s.calls ? Math.round((s.agreed / s.calls) * 100) + "%" : "-"}</b><small>matched the bot</small></div><div><b>${s.go_said ? `${s.went}/${s.go_said}` : "-"}</b><small>went for it when it said go</small></div><div><b>${s.given}</b><small>win-chance points given up</small></div></div>`;
  const who = b.bot ? `This season ${esc(b.bot)} reads every fourth down by win probability; the critics and your boss read it.` : "No bot grades your level's fourth downs in this era: the columnists judge by how it turned out.";
  const reports = b.reports.length ? b.reports.map((r, i) => soReport(r, i === 0)).join("") : soEmpty("Nothing graded yet.");
  return `${tiles}<p class="muted">${who}</p><div class="so-reports">${reports}</div>`;
}

function soSchedule(b) {
  const rights = b.rights && b.rights.length ? `<p>Your international marketing rights: <b>${b.rights.map(esc).join(", ")}</b>. Fan clubs, sponsors and a game there every few years.</p>` : '<p class="muted">No international marketing rights for your club this year.</p>';
  if (b.none) return `${soEmpty(b.release)}${soPanel("Abroad", rights)}`;
  const tiles = `<div class="st-tiles so-tiles"><div><b>${b.prime}</b><small>prime-time games at release</small></div><div><b>${b.short}</b><small>short weeks</small></div><div><b class="so-small">${esc(b.abroad || "None")}</b><small>a game abroad</small></div></div>`;
  const rows = table("sosched", [
    { k: "date", t: "Date", l: true, v: (g) => g.iso },
    { k: "opp", t: "Opponent", l: true, f: (g) => soTeam(g.opp), v: (g) => g.opp.name },
    { k: "window", t: "Window", l: true, f: (g) => { const t = g.window.charAt(0).toUpperCase() + g.window.slice(1); return g.prime ? `<span class="tag gold">${esc(t)}</span>` : esc(t); } },
  ], b.games, { tools: false });
  return `${tiles}<div class="grid2"><div class="col">${soPanel("As released", rows, "flexes move games from here")}</div><div class="col">${soPanel("Abroad", rights)}</div></div>`;
}

function soAnnuals(b) {
  if (!b.magazines.length) return soEmpty("The preseason annuals come out in the summer.");
  const years = `<div class="seg wrap">${b.years.slice(0, 8).map((y) => `<button data-act="soyear" data-y="${y}" aria-pressed="${y === b.year}">${y}</button>`).join("")}</div>`;
  const mags = b.magazines.map((m) => {
    const top = `<ol class="so-top">${m.top.slice(0, m.nfl ? 32 : 25).map((t) => `<li class="${t.mine ? "mine" : ""}"><span class="so-rank">${t.rank}</span>${soTeam(t.team)}</li>`).join("")}</ol>`;
    const covers = m.covers.length ? `<p><span class="st-label">On the cover</span><br>${m.covers.map(soWhoPos).join("<br>")}</p>` : "";
    const all = m.all_conf.length ? `<p><span class="st-label">All-conference, your conference</span><br>${m.all_conf.map(soWhoPos).join(" · ")}</p>` : "";
    return `<section class="panel so-mag"><h2>${esc(m.title)} <small>${b.year} preview</small></h2>${covers}${top}${all}</section>`;
  }).join("");
  return `<div class="so-bar">${years}</div><div class="so-mags">${mags}</div>`;
}

function soRatings(b) {
  if (!b.released && !b.club.length) return soEmpty("The video game's ratings come out each August.");
  const mine = b.mine.length
    ? table("soratings", [
      { k: "rating", t: "Rating", f: (r) => `<b class="so-ovr">${r.rating}</b>` },
      { k: "who", t: "Player", l: true, f: (r) => soWho(r.who), v: (r) => r.who.name },
      { k: "pos", t: "Pos", l: true, f: (r) => esc(r.who.pos || ""), v: (r) => r.who.pos },
    ], b.mine, { tools: false, sort: "rating" })
    : soEmpty("None of yours rated.");
  const club = b.club.length
    ? table("so99", [
      { k: "year", t: "Year" },
      { k: "who", t: "Player", l: true, f: (r) => soWho(r.who), v: (r) => r.who.name },
      { k: "pos", t: "Pos", l: true, f: (r) => esc(r.who.pos || ""), v: (r) => r.who.pos },
      { k: "title", t: "Game", l: true, f: (r) => `<small class="muted">${esc(r.title)}</small>` },
    ], b.club, { tools: false, sort: "year" })
    : soEmpty("Nobody at 99 yet.");
  return `<p class="muted">${esc(b.note)}</p><div class="grid2"><div class="col">${soPanel("Your team in the game", mine, b.titles.map(esc).join(" · "))}</div><div class="col">${soPanel("The 99 club", club)}</div></div>`;
}

function soFantasy(b) {
  const groups = b.groups.map((g) => soPanel(g.pos, g.rows.length
    ? `<div class="tbl warm box"><table><thead><tr><th>#</th><th class="l">Player</th><th>Pts</th><th>G</th><th>Per</th><th>Last</th></tr></thead><tbody>${g.rows.map((r) => `<tr class="${r.who.mine ? "mine" : ""}"><td>${r.rank}</td><td class="l">${soWho(r.who)}${r.who.team ? ` <small class="muted">${esc(r.who.team.name)}</small>` : ""}</td><td><b>${r.pts}</b></td><td>${r.games}</td><td>${r.per}</td><td>${r.last}</td></tr>`).join("")}</tbody></table></div>`
    : soEmpty("No games yet."))).join("");
  const past = b.past.length ? `<div class="items">${b.past.map((p) => `<div class="item"><span class="when">${p.year} · ${esc(p.pos)}</span><span>${soWho(p.who)} <small class="muted">${p.pts} pts</small></span></div>`).join("")}</div>` : soEmpty("The first season's leaders land here.");
  return `<p class="muted">${esc(b.scoring)}</p><div class="so-fantasy">${groups}</div>${soPanel("Past winners", past)}`;
}

function soJerseys(b) {
  if (!b.lists.length) return soEmpty("The lists come out in the spring (the NFLPA) and August (college, from 2021).");
  return `<div class="so-mags">${b.lists.slice(0, 6).map((l) => soPanel(esc(l.title), `<ol class="so-top">${l.rows.map((r) => { const t = r.school || r.who.team; return `<li class="${r.who.mine ? "mine" : ""}"><span class="so-rank">${r.rank}</span>${soWho(r.who)}${t ? ` <small class="muted">${esc(t.name)}</small>` : ""}</li>`; }).join("")}</ol>`)).join("")}</div>`;
}

function soShows(b) {
  const docs = b.docs.length
    ? table("sodocs", [
      { k: "year", t: "Year" },
      { k: "show", t: "Series", l: true, f: (d) => `<b>${esc(d.show)}</b>` },
      { k: "team", t: "Team", l: true, f: (d) => soTeam(d.team), v: (d) => d.team.name },
      { k: "how", t: "How", l: true },
      { k: "episodes", t: "Eps" },
      { k: "stars", t: "Breakouts", l: true, wrap: true, f: (d) => d.stars.map(soWho).join(", "), v: (d) => d.stars.length },
    ], b.docs, { tools: false, sort: "year" })
    : soEmpty("No documentary seasons yet.");
  const [stops, visits] = soShowStops(b);
  return `<div class="grid2"><div class="col">${soPanel("Documentaries", docs)}</div><div class="col">${soPanel(esc(b.show), stops, `on the road since ${b.show_from}`)}${visits ? soPanel("Most visited", visits) : ""}</div></div>`;
}

// The pregame show's stops this season and the campuses it visits most.
function soShowStops(b) {
  const stops = b.stops.length
    ? `<div class="items">${b.stops.map((s) => `<div class="item ${s.past ? "" : "huge"}"><span class="when">${esc(s.date)}</span><span>${soTeam(s.host)} <span class="muted">hosts</span> ${soTeam(s.visitor)}</span></div>`).join("")}</div>`
    : soEmpty(`${b.show} picks its first stop in late August.`);
  const visits = b.visits.length ? `<div class="items">${b.visits.map((v) => `<div class="item"><span class="when">${soPlural(v.visits, "visit")}</span><span>${soTeam(v.team)} <small class="muted">last ${v.last}</small></span></div>`).join("")}</div>` : "";
  return [stops, visits];
}

// Traditions: the show on campus, without the documentaries.
function soPregame(b) {
  const [stops, visits] = soShowStops(b);
  return `<div class="grid2"><div class="col">${soPanel(esc(b.show), stops, `on the road since ${b.show_from}`)}</div><div class="col">${soPanel("Most visited", visits || soEmpty("No campus has hosted it yet."))}</div></div>`;
}

function soNicknames(b) {
  const views = [["yours", "Your team", b.yours], ["national", "Known everywhere", b.national], ["newest", "Newest", b.newest], ["coaches", "Coaches", b.coaches], ["units", "Units", b.units], ["plays", "Named plays", b.plays]];
  const st = keep("sonicks", { v: b.yours.length ? "yours" : "national" });
  const seg = `<div class="seg wrap">${views.map(([v, t, l]) => `<button data-act="sonicks" data-v="${v}" aria-pressed="${st.v === v}">${t}<small>${l.length}</small></button>`).join("")}</div>`;
  let body;
  if (st.v === "units") {
    body = b.units.length ? `<div class="so-nicks">${b.units.map((u) => `<article class="so-nick-card"><div class="so-nick-top"><span class="so-nick-name">${esc(u.name)}</span></div><div class="so-nick-who">${soTeam(u.team)} · ${esc(u.unit)}</div><p>${rich(u.why)} <span class="muted">${u.from === u.last ? u.from : `${u.from}-${u.last}`}</span></p></article>`).join("")}</div>` : soEmpty("No unit has earned a name yet. It takes a season people remember: a defense that smothers everyone, a line that moves anyone.");
  } else if (st.v === "plays") {
    body = b.plays.length ? `<div class="so-plays">${b.plays.map((p) => `<article class="so-play"><div class="so-nick-top"><span class="so-nick-name">${esc(p.name)}</span><span class="muted small">${esc(p.date)}</span></div><p>${rich(p.what)}</p>${p.call ? `<blockquote>“${esc(p.call)}”</blockquote>` : ""}<div class="so-nick-who">${p.teams.map(soTeam).join(" <span class='muted'>over</span> ")}</div></article>`).join("")}</div>` : soEmpty("No play has a name yet. The ones that get one are rare: a last-second miracle in a game that mattered.");
  } else {
    const list = (views.find((v) => v[0] === st.v) || views[0])[2];
    body = soNicks(list, st.v === "yours" ? "No names on your team yet. A big moment that fits a player's game gives him one; you can give one too, from his page." : "None yet.");
  }
  return `<div class="so-bar">${seg}${b.can_give ? '<span class="muted small">Name one of your players from his page (the Story tab).</span>' : ""}</div>${body}`;
}

// ---------- the traditions ----------
screens.traditions = async (a) => {
  const k = keep("sotrad", { t: "" });
  const x = await api("stories_traditions", { tab: a.tab || k.t });
  const tab = tabOf("sotrad", x.tabs, { tab: x.tab });
  title("Traditions", tab);
  const b = x.body;
  const body = {
    Rivalries: () => soRivalries(b),
    Uniforms: () => soUniforms(b),
    Honors: () => soHonors(b),
    "Pregame show": () => soPregame(b),
    Stormings: () => soStormings(b),
    Farewells: () => soFarewells(b),
    Celebrations: () => soCelebrations(b),
    "Parades and rings": () => soParades(b),
  }[tab]();
  const tc = (x.team && x.team.colors) || [];
  return `<div class="pad">${pageHead({ kicker: x.team ? teamA(x.team) : "The world", title: "Traditions", sub: "The trophies, the uniforms, the honors and the rituals around the games", goto: gotoFor("traditions", a, x) })}${tabs("sotrad", x.tabs, tab)}<div class="tabbody" style="${tc[0] ? `--tc1:${tc[0]};--tc2:${tc[1] || tc[0]}` : ""}">${body}</div></div>`;
};
GOTO.traditions = () => [["Team history", "team"], ["Schedule", "schedule"], ["Office", "office"], ["Media", "media"]];

function soRivalries(b) {
  if (!b.rows.length) return soEmpty(b.level === "hs" ? "Your school's rivalry is set at the start of the season." : "No listed rivalry for your team.");
  const held = b.rows.filter((r) => r.trophy && r.yours);
  const head = held.length ? `<div class="so-held">${held.map((r) => `<div class="so-held-item">${soGlyph("trophy")}<div><b>${esc(r.trophy)}</b><small>in your building · beat ${esc(r.rival.name)} for it</small></div></div>`).join("")}</div>` : "";
  return head + table("sorival", [
    { k: "game", t: "Game", l: true, f: (r) => `<b>${esc(r.game.charAt(0).toUpperCase() + r.game.slice(1))}</b>` },
    { k: "trophy", t: "Trophy", l: true, f: (r) => (r.trophy ? esc(r.trophy) : '<span class="muted">No trophy</span>'), v: (r) => r.trophy || "" },
    { k: "rival", t: "Rival", l: true, f: (r) => soTeam(r.rival), v: (r) => r.rival.name },
    { k: "rec", t: "Record", f: (r) => soRec(r.w, r.l, r.t), v: (r) => r.w - r.l },
    { k: "holder", t: "Holds it", l: true, f: (r) => (r.holder ? (r.yours ? `<span class="tag gold">You</span>` : soTeam(r.holder)) : '<span class="muted">-</span>'), v: (r) => (r.holder ? r.holder.name : "") },
    { k: "since", t: "Series", l: true, f: (r) => (r.real ? "All-time" : r.since ? `Since ${r.since}` : ""), v: (r) => r.since || 0 },
  ], b.rows, { tools: false });
}

function soUniforms(b) {
  if (b.none) return soEmpty("You need a team first.");
  const sets = b.sets.map((s) => `<button class="so-set ${s.next ? "on" : ""}" data-act="souni" data-i="${s.i}" aria-pressed="${s.next}"><span class="so-jersey ${/white|road/i.test(s.name) ? "white" : /throwback|classic|heritage/i.test(s.name) ? "retro" : /black|blackout|color rush|alternate/i.test(s.name) ? "alt" : "home"}"></span><b>${esc(s.name)}</b><small>${s.w + s.l ? `${s.w}-${s.l} in it with you` : "Not worn with you yet"}</small></button>`).join("");
  const staff = `<button class="st-btn small ${b.next == null ? "primary" : "glass"}" data-act="souni" data-i="-1">${b.next == null ? "Your staff picks each week" : "Let your staff pick again"}</button>`;
  const worn = b.worn.length ? `<div class="items">${b.worn.map((w) => `<div class="item"><span class="when">${esc(w.date)}</span><span>${esc(w.set)} <span class="tag ${w.won ? "gold" : "soft"}">${w.won ? "Won" : "Lost"}</span></span></div>`).join("")}</div>` : soEmpty("Your games will show what you wore.");
  const reveals = b.reveals.length ? `<div class="items">${b.reveals.map((r) => `<div class="item"><span class="when">${r.year}</span><span>${esc(r.what)}</span></div>`).join("")}</div>` : "";
  return `<p class="muted">${esc(b.note)}</p><div class="so-bar"><span class="st-label">Next game</span>${staff}</div><div class="so-sets">${sets}</div>
    <div class="grid2"><div class="col">${soPanel("Worn", worn)}</div><div class="col">${reveals ? soPanel("Summer reveals", reveals) : ""}</div></div>`;
}

function soHonor(h) {
  const icon = h.key === "Statue" ? "trophy" : h.key === "RingOfHonor" ? "ring" : "plaque";
  return `<article class="so-honor">${soGlyph(icon)}<div><span class="st-label">${esc(h.kind)}${h.number != null ? ` · #${h.number}` : ""}</span><b>${soWho(h.who)}</b><small>${esc(h.pos)} · ${esc(h.years)} · ${h.done ? `honored ${h.year}` : `ceremony this season`}</small></div></article>`;
}

function soHonors(b) {
  const retired = b.retired.length ? `<div class="so-retired">${b.retired.map((n) => `<span class="so-num">${n}</span>`).join("")}</div>` : '<p class="muted">No number retired yet.</p>';
  const yours = b.yours.length ? `<div class="so-honors">${b.yours.map(soHonor).join("")}</div>` : soEmpty("Your team hasn't honored a former player in your time. Greats get their night when their careers are done.");
  const league = b.league.length
    ? table("sohonors", [
      { k: "year", t: "Year" },
      { k: "kind", t: "Honor", l: true },
      { k: "who", t: "Player", l: true, f: (h) => soWho(h.who), v: (h) => h.name },
      { k: "number", t: "No.", f: (h) => (h.number != null ? h.number : "") },
      { k: "team", t: "Team", l: true, f: (h) => soTeam(h.team), v: (h) => h.team.name },
    ], b.league, { tools: false, sort: "year" })
    : soEmpty("None yet.");
  return `<div class="grid2"><div class="col">${soPanel("Retired numbers", retired)}${soPanel("Your team's honors", yours)}</div><div class="col">${soPanel("Around the game", league)}</div></div>`;
}

function soStormings(b) {
  const p = b.policy;
  const policy = p ? (p.fines ? `<p>The ${esc(p.conference)} fines a school that lets its fans on the field: <b>${soMoney(p.fines[0])}</b> the first time, <b>${soMoney(p.fines[1])}</b> the second, <b>${soMoney(p.fines[2])}</b> after that${p.from ? ` (policy since ${p.from})` : ""}. The home school pays, whoever won.</p>` : `<p>The ${esc(p.conference)} has no storming fine this season.</p>`) : "";
  const rows = b.rows.length
    ? table("sostorm", [
      { k: "date", t: "Date", l: true, v: (s) => s.year },
      { k: "team", t: "Fans of", l: true, f: (s) => soTeam(s.team), v: (s) => s.team.name },
      { k: "opp", t: "Beat", l: true, f: (s) => soTeam(s.opp), v: (s) => s.opp.name },
      { k: "score", t: "Score", f: (s) => `${s.score[0]}-${s.score[1]}`, v: (s) => s.score[0] - s.score[1] },
      { k: "fine", t: "Fine", f: (s) => (s.fine ? soMoney(s.fine) : '<span class="muted">-</span>') },
    ], b.rows, { tools: false })
    : soEmpty("No field stormed yet. It takes an upset at home over a ranked team.");
  return `${policy ? soPanel("The rule", policy) : ""}${soPanel("Fields stormed", rows)}`;
}

function soFarewells(b) {
  const tours = b.tours.length
    ? `<div class="so-honors">${b.tours.map((f) => `<article class="so-honor">${soGlyph("gift")}<div><span class="st-label">${f.year} farewell tour · ${esc(f.team.name)}</span><b>${soWho(f.who)}</b><small>${f.gifts.length ? f.gifts.map((g) => `${esc(g.gift)} (${esc(g.team.name)})`).join(" · ") : "The road gifts come as he makes his last visits."}${f.done ? " · played his last game" : ""}</small></div></article>`).join("")}</div>`
    : soEmpty("No star has announced his last season yet.");
  const one = b.one_day.length
    ? `<div class="items">${b.one_day.map((o) => `<div class="item"><span class="when">${o.year}</span><span>${soWho(o.who)} ${o.passed ? '<span class="muted">asked; you passed</span>' : `signed for a day with ${soTeam(o.team)}`}</span></div>`).join("")}</div>`
    : soEmpty("No one-day contracts yet.");
  return `${soAsks(b.asks)}<div class="grid2"><div class="col">${soPanel("Farewell tours", tours)}</div><div class="col">${soPanel("One-day contracts", one)}</div></div>`;
}

function soCelebrations(b) {
  const rows = b.rows.length ? `<div class="items">${b.rows.map((c) => `<div class="item"><span class="when">${c.year}</span><span><b>${esc(c.name)}</b> · ${soWhoPos(c.who)}</span></div>`).join("")}</div>` : soEmpty("No signature celebration yet. A showman makes one his own.");
  return `${soPanel("The rule now", `<p>${esc(b.rule)}</p>`)}${soPanel("Signature celebrations", rows)}`;
}

function soParades(b) {
  const rows = b.rows.length ? `<div class="items">${b.rows.map((p) => `<div class="item ${p.mine ? "huge" : ""}"><span class="when">${p.year}</span><span>${soTeam(p.team)} <small class="muted">${p.crowd ? `about ${p.crowd.toLocaleString()} on the route` : ""}</small></span></div>`).join("")}</div>` : soEmpty("No parade yet. Win the last game of a season and the city throws one.");
  const rings = b.rings.length ? `<div class="items">${b.rings.map((r) => `<div class="item"><span class="when">${esc(r.date)}</span><span>${rich(r.text)}</span></div>`).join("")}</div>` : soEmpty("Your rings go here.");
  return `<div class="grid2"><div class="col">${soPanel("Parades", rows)}</div><div class="col">${soPanel("Your rings", rings)}</div></div>`;
}

// ---------- the player page's Story tab ----------
PLAYER_TABS.push({
  name: "Story",
  after: "People",
  draw: async (p) => {
    const x = await api("stories_player", { id: p.id });
    const give = x.can_name ? `<button class="st-btn small primary" data-act="sonick" data-id="${x.id}" data-name="${esc(x.name)}">Give him a nickname</button>` : "";
    const names = soPanel("His names", soNicks(x.names, "No nickname has stuck. They come from a moment that fits his game, from home, or from you.", { noWho: true }) + give);
    const facts = [
      x.rating != null ? `<div><b>${x.rating}</b><small>${esc(x.vg || "video game")} rating</small></div>` : "",
      x.fantasy ? `<div><b>${x.fantasy.pts}</b><small>fantasy points · ${x.fantasy.rank} at his position</small></div>` : "",
      x.number != null ? `<div><b>${x.number}</b><small>his number</small></div>` : "",
      x.covers.length ? `<div><b>${x.covers.length}</b><small>magazine ${x.covers.length === 1 ? "cover" : "covers"}</small></div>` : "",
    ].join("");
    const list = (items, f) => (items.length ? `<div class="items">${items.map(f).join("")}</div>` : "");
    const press = [
      list(x.covers, (c) => `<div class="item"><span class="when">${c.year}</span><span>On the cover of <b>${esc(c.title)}</b></span></div>`),
      list(x.all_conf, (c) => `<div class="item"><span class="when">${c.year}</span><span>Preseason all-conference, ${esc(c.title)}</span></div>`),
      list(x.jerseys, (j) => `<div class="item"><span class="when">${j.year}</span><span>#${j.rank} on the ${esc(j.list)} list</span></div>`),
      list(x.docs, (d) => `<div class="item"><span class="when">${d.year}</span><span>A breakout star of <b>${esc(d.show)}</b></span></div>`),
    ].join("");
    const legacy = [
      x.honors.length ? `<div class="so-honors">${x.honors.map(soHonor).join("")}</div>` : "",
      list(x.plays, (pl) => `<div class="item"><span class="when">${esc(pl.date)}</span><span><b>${esc(pl.name)}</b> · ${rich(pl.what)}${pl.call ? `<br><small class="muted">“${esc(pl.call)}”</small>` : ""}</span></div>`),
      x.celebration ? `<p><span class="st-label">His celebration</span><br><b>${esc(x.celebration)}</b></p>` : "",
      x.farewell ? `<p><span class="st-label">Farewell tour, ${x.farewell.year}</span><br>${x.farewell.gifts.map((g) => `${esc(g.gift)} <small class="muted">${esc(g.team.name)}</small>`).join(" · ") || "Announced; the gifts come on the road."}</p>` : "",
      x.one_day ? `<p><span class="st-label">One-day contract</span><br>Retired a member of ${soTeam(x.one_day.team)} in ${x.one_day.year}</p>` : "",
    ].join("");
    return `${facts ? `<div class="st-tiles so-tiles">${facts}</div>` : ""}<div class="grid2"><div class="col">${names}</div><div class="col">${press ? soPanel("In the press", press) : ""}${legacy ? soPanel("What he leaves", legacy) : ""}${!press && !legacy ? soPanel("In the press", soEmpty("Nothing yet: covers, the video game, jersey lists and honors land here.")) : ""}</div></div>`;
  },
});

// ---------- clicks ----------
Object.assign(CORE_MORE, {
  soans: async (el) => {
    el.disabled = true;
    try {
      const r = await api("stories_answer", { what: el.dataset.what, yes: el.dataset.yes === "1" });
      toast(plainText(r.said));
      S.status = await api("status");
      refresh(el);
    } catch (e) { toast(e.message); el.disabled = false; }
  },
  souni: async (el) => {
    try { await api("stories_uniform", { set: Number(el.dataset.i) }); refresh(el); } catch (e) { toast(e.message); }
  },
  sonick: async (el) => {
    const id = Number(el.dataset.id);
    let ideas = [];
    try { ideas = await api("stories_suggest", { id }); } catch (e) { ideas = []; }
    const name = await ask(`A nickname for ${el.dataset.name}`, { value: ideas[0] || "", ok: "Say it at the podium", note: `${ideas.length ? `Ideas: ${ideas.join(", ")}. ` : ""}Your standing in the press, the fit and his own buy-in decide whether it sticks. A legend's name is off limits.` });
    if (!name) return;
    try { const r = await api("stories_nickname", { id, name }); toast(plainText(r.said)); refresh(el); } catch (e) { toast(e.message); }
  },
  soyear: (el) => { keep("soyear", {}).y = Number(el.dataset.y); refresh(el); },
  sofilter: (el) => { keep("sofilter", { v: "all" }).v = el.dataset.v; refresh(el); },
  sonicks: (el) => { keep("sonicks", {}).v = el.dataset.v; refresh(el); },
  soshelf: (el) => { const st = keep("soshelf", {}); st[el.dataset.k] = !st[el.dataset.k]; refresh(el); },
});
