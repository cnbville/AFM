// Lane screens: storylines (update 2, lane 1, S11). Registers screens.<name>.
"use strict";
/* The storyline engine's screens, boards 28-33, in Press Box:
 *
 *   storylines   This week (28): the stories you're in, the ones you
 *                follow, the season's ended ones, the hottest elsewhere,
 *                what's coming up in yours.
 *   storyline    One story (29): its heat and beats, what your staff knows
 *                against what the public believes, the sides and the cast,
 *                what they're saying, your moves, linked stories, follow.
 *   slStop(id)   A story meeting that stopped Continue (30), over Home.
 *   slPrevious() "Previously" when a career opens (30), once a day.
 *   chapters     The season's chapters (31): the strip of games and the
 *                stories across it, the book, one chapter open.
 *   Career story Its "Chapters" tab (33): your jobs and chapters on one
 *                line, the book, the office shelf, your names.
 *   jobpick      Pick your next job (32): what each job would hand you.
 */

I.flame = '<path d="M12 21.5c-4.2 0-7-2.8-7-6.6 0-3.4 2.3-5.4 3.8-7.9.5 1.9 1.5 3 2.8 3.5C12 7.6 13.5 5 15.8 3c.3 3.2 3.2 5.5 3.2 10.2 0 5.1-2.9 8.3-7 8.3z"/><path d="M12 21.5c-1.9 0-3.2-1.3-3.2-3.1 0-1.9 1.6-3 2.4-4.6.9 1.2 2.1 1.6 2.9 3 .4.7.6 1.2.6 1.8 0 1.7-1.2 2.9-2.7 2.9z"/>';
I.book = '<path d="M4 5.5C6.5 4.3 9.3 4.2 12 6c2.7-1.8 5.5-1.7 8-.5V19c-2.5-1.2-5.3-1.1-8 .7-2.7-1.8-5.5-1.9-8-.7z"/><path d="M12 6v13.7"/>';

navAdd({ go: "storylines", label: "This week", icon: "flame", sec: "top", after: "inbox", badge: () => (S.status && S.status.stories_need) || 0 });
navAdd({ go: "chapters", label: "Season chapters", icon: "book", sec: "world", after: "story" });
Object.assign(CRUMBS, { storylines: "This week", storyline: "Story", chapters: "Season chapters", jobpick: "Pick your next job" });

// ---------- small pieces ----------
const slHeat = (h) => (h >= 60 ? "sl-hot" : h >= 30 ? "sl-warm" : "sl-cool");
const slEmpty = (t) => `<div class="empty">${esc(t)}</div>`;
const slSigned = (d) => (d > 0 ? "+" : d < 0 ? "−" : "±") + Math.abs(d);
const SL_PHASES = ["Seed", "Spark", "Building", "Peak", "Resolved", "Aftermath"];

// The last weeks of heat as a line.
function slSpark(path, w = 132, h = 38) {
  const n = path.length;
  if (!n) return "";
  const X = (k) => (n === 1 ? w - 2 : (k / (n - 1)) * (w - 4) + 2);
  const Y = (v) => h - 3 - (v / 100) * (h - 6);
  const line = path.map((v, k) => `${X(k).toFixed(1)},${Y(v).toFixed(1)}`).join(" ");
  const last = path[n - 1];
  return `<svg class="sl-spark ${slHeat(last)}" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}" role="img" aria-label="Heat over the last ${n} week${n === 1 ? "" : "s"}"><line x1="0" x2="${w}" y1="${h - 1}" y2="${h - 1}"></line><polygon points="${X(0).toFixed(1)},${h - 2} ${line} ${X(n - 1).toFixed(1)},${h - 2}"></polygon><polyline points="${line}"></polyline><circle cx="${X(n - 1).toFixed(1)}" cy="${Y(last).toFixed(1)}" r="2.6"></circle></svg>`;
}
const slPhaseBar = (k) => `<span class="sl-ph6" aria-label="Phase: ${esc(SL_PHASES[k] || "")}">${SL_PHASES.map((_, i) => `<i class="${i < k ? "past" : i === k ? "now" : ""}"></i>`).join("")}</span>`;
function slSplit(sp, small) {
  const pa = sp.share, pb = 100 - sp.share;
  return `<span class="sl-split ${small ? "small" : ""}"><span class="sl-split-k"><span>${esc(sp.a)} <b>${pa}%</b></span><span class="muted">${esc(sp.audience || "")}</span><span><b>${pb}%</b> ${esc(sp.b)}</span></span><span class="sl-split-bar"><i style="width:${pa}%"></i><i></i></span></span>`;
}
const slChips = (chips) => (chips && chips.length ? `<span class="sl-chips">${chips.map((c) => `<span class="sl-eff t${c.tone}">${esc(c.t)}</span>`).join("")}</span>` : "");
const slFollow = (o) => `<button class="sl-pin ${o.following ? "on" : ""}" data-act="slfollow" data-id="${o.id}" data-on="${o.following ? 0 : 1}" title="${o.following ? "Following: its beats reach your inbox. Click to stop." : "Follow: its beats reach your inbox until it ends."}" aria-label="${o.following ? "Stop following" : "Follow"}">${o.following ? "✓" : "+"}</button>`;

// One story as a card (28).
function slCard(c) {
  const priv = !c.public;
  return `<article class="sl-card ${c.needs ? "needs" : ""}" data-go="storyline" data-id="${c.id}" role="link" tabindex="0">
    <span class="sl-card-top"><span class="st-label">${esc(c.fam)}</span>${c.needs ? '<span class="sl-needs"><i></i>Needs you</span>' : priv ? '<span class="sl-priv">Only the building knows</span>' : ""}</span>
    <span class="sl-q">${rich(c.q)}</span>
    <span class="sl-card-heat"><span class="sl-big ${slHeat(c.heat)}">${c.heat}<small>heat · <em class="${c.delta > 0 ? "up" : ""}">${slSigned(c.delta)}</em> this week</small></span>${slSpark(c.path)}</span>
    <span class="sl-card-phase"><span class="sl-line2"><span>You: <b>${esc(c.role || "watching")}</b></span><span>${esc(c.phaseName)}${priv ? " · private" : ""}</span></span>${slPhaseBar(c.phase)}</span>
    ${slSplit(c.split, true)}
    ${c.lq ? `<span class="sl-latest"><span>${rich(c.lq)}</span><small>${esc(c.ls)} · latest beat</small></span>` : ""}
    <span class="sl-card-next"><span>Next: ${rich(c.next || "nothing set")}</span><small>${c.beats} beat${c.beats === 1 ? "" : "s"} · ${priv ? "private" : `since ${esc(c.since)}`}</small></span>
  </article>`;
}

// ---------- This week (28) ----------
screens.storylines = async (a) => {
  const x = await api("storylines_week");
  const list = [{ name: "Your stories", badge: x.mine.length || "" }, { name: "Following", badge: x.following.length || "" }, { name: "Ended this season", badge: x.ended.length || "" }];
  const tab = tabOf("slweek", list.map((t) => t.name), a);
  title("This week", tab);
  let main;
  if (tab === "Your stories") {
    main = `<div class="sl-ph"><span><b class="sl-pt">Stories you're in</b> <span class="sub">${x.mine.length} open · sorted by heat · the people in them decide what happens next</span></span><span class="sub">heat 0-100: how much people talk about it</span></div>
      ${x.mine.length ? `<div class="sl-cards">${x.mine.map(slCard).join("")}</div>` : slEmpty(x.career ? "No story has your name in it yet. They start with a depth chart, a recruit, a losing streak, a rumour: when people start talking, it shows here." : "You need a job first: the stories come with it.")}`;
  } else if (tab === "Following") {
    main = `<div class="sl-ph"><span><b class="sl-pt">Stories you follow</b> <span class="sub">their beats reach your inbox until they end</span></span></div>
      ${x.following.length ? `<div class="sl-cards">${x.following.map(slCard).join("")}</div>` : slEmpty("You follow nothing yet. The + beside a story elsewhere follows it.")}`;
  } else {
    main = `<div class="sl-ph"><span><b class="sl-pt">Ended this season</b> <span class="sub">yours, the ones you followed and the ones around you</span></span></div>
      ${x.ended.length ? `<div class="sl-ended">${x.ended.map((e) => `<div class="sl-erow" data-go="storyline" data-id="${e.id}" role="link" tabindex="0"><span class="sl-num ${slHeat(e.heat)}">${e.heat}</span><span><b>${rich(e.q)}</b><small>${rich(e.text)}</small></span><span class="st-label">${esc(e.fam)}</span><span class="sl-out">${esc(e.outcome)}</span></div>`).join("")}</div>` : `<div class="empty">Nothing has ended yet in the ${x.year} season. Last season's stories are in ${linkTo("chapters", { season: x.year - 1 }, "its chapters")}.</div>`}`;
  }
  const others = x.others.length
    ? x.others.map((o) => `<div class="sl-orow"><span class="sl-num ${slHeat(o.heat)}">${o.heat}</span><span class="sl-otext" data-go="storyline" data-id="${o.id}" role="link" tabindex="0"><b>${rich(o.q)}</b><small>${esc(o.fam)} · ${esc(o.phase.toLowerCase())}</small></span><span class="st-label sl-tag">${esc(o.tag)}</span>${slFollow(o)}</div>`).join("")
    : slEmpty("Quiet everywhere else.");
  const ended = x.endedWeek.length ? x.endedWeek.map((e) => `<div class="sl-mini" data-go="storyline" data-id="${e.id}" role="link" tabindex="0"><b>${rich(e.q)}</b><small>${rich(e.text)}</small></div>`).join("") : '<div class="sub small">Nothing ended this week.</div>';
  const up = x.up.length ? x.up.map((u) => `<div class="sl-up"><span title="${esc(u.date)}">${esc(u.d)}</span><span>${rich(u.text)}</span></div>`).join("") : '<div class="sub small">No dates set in your stories.</div>';
  const crest = x.team ? `<span class="sl-crest">${mark(x.team, 40)}</span>` : "";
  return `<div class="pad sl">
    <header class="sl-head">${crest}<div class="sl-head-copy"><span class="st-label">${esc(x.ctx)}</span><h1 class="sl-h1">This week</h1></div>${tabs("slweek", list, tab)}<div class="sl-head-act"><button class="st-btn primary" data-act="continue">Continue</button></div></header>${gotoFor("storylines")}
    <div class="sl-week">
      <section class="panel sl-main">${main}</section>
      <aside class="panel sl-side">
        <div class="sl-ph"><b>The ${x.others.length || 8} hottest elsewhere</b><span class="sub">${esc(x.where)}</span></div>
        <div class="sl-others">${others}</div>
        <div class="sl-block"><span class="st-label">Ended this week</span>${ended}</div>
        <div class="sl-block"><span class="st-label">Coming up in your stories</span>${up}</div>
      </aside>
    </div></div>`;
};
GOTO.storylines = () => [["Inbox", "inbox"], ["Media", "media"], ["Season chapters", "chapters"], ["Career story", "story"]];

// ---------- one story (29) ----------
function slChart(c) {
  const W = 524, H = 112, days = Math.max(1, c.days);
  const X = (d) => 6 + (d / days) * 512, Y = (h) => 94 - (h / 100) * 86;
  const line = c.path.map(([d, h]) => `${X(d).toFixed(1)},${Y(h).toFixed(1)}`).join(" ");
  const last = c.path[c.path.length - 1] || [0, 0];
  const grid = [25, 50, 75].map((h) => `<line class="g" x1="0" x2="${W}" y1="${Y(h).toFixed(1)}" y2="${Y(h).toFixed(1)}"></line>`).join("");
  const marks = c.marks.map((m) => `<circle class="${m.ours ? "ours" : ""}" cx="${X(m.d).toFixed(1)}" cy="${Y(m.h).toFixed(1)}" r="${m.big ? 4.5 : 3.2}"></circle>`).join("");
  const ticks = c.ticks.map(([d, n]) => `<span style="left:${Math.min(96, Math.max(3, (X(d) / W) * 100)).toFixed(1)}%">${esc(n)}</span>`).join("");
  return `<div class="sl-chart"><svg viewBox="0 0 ${W} ${H}" role="img" aria-label="Heat over the story's life, each beat marked">${grid}<polygon points="${X(0).toFixed(1)},94 ${line} ${X(last[0]).toFixed(1)},94"></polygon><polyline points="${line}"></polyline>${marks}<line class="now" x1="${X(c.now).toFixed(1)}" x2="${X(c.now).toFixed(1)}" y1="6" y2="98"></line></svg><div class="sl-ticks">${ticks}</div></div>`;
}

screens.storyline = async (a) => {
  const x = await api("storylines_story", { id: a.id });
  title(plainText(x.q).slice(0, 60), "Story");
  const beats = x.beats.length
    ? x.beats.map((b) => `<div class="sl-bt ${b.ours ? "ours" : ""}"><span class="sl-d">${esc(b.d)}</span><span class="sl-src">${esc(b.src)}</span><span>${rich(b.text)}</span><span class="sl-h">${b.h ? "+" + b.h : "·"}</span></div>`).join("")
    : slEmpty("No beats yet.");
  const next = x.open
    ? `<div class="sl-bt next"><span class="sl-d">Next</span><span class="sl-src">${esc(x.nextD)}</span><span>${rich(upperFirst(x.next))}</span><span></span></div>`
    : x.ended ? `<div class="sl-bt next"><span class="sl-d">Ended</span><span class="sl-src">${esc(x.ended.outcome)}</span><span>${rich(x.ended.text)}</span><span></span></div>` : "";
  const sides = x.sides.map((s) => `<div class="sl-side-row"><span class="muted">${esc(s.aud)}</span>${slSplit({ a: s.a, b: s.b, share: s.share, audience: "" }, true)}</div>`).join("");
  const cast = x.cast.map((c) => `<div class="sl-cr"><span><b>${rich(c.name)}</b><small>${esc(c.role)}</small></span><span class="muted">${esc(c.side)}</span><span class="sl-mood t${c.tone}">${c.mood ? `<i></i>${esc(c.mood)}` : ""}</span></div>`).join("");
  const voices = x.voices.length ? x.voices.map((v) => `<div class="sl-vq"><span>${rich(v.quote)}</span><small>${esc(v.who)}</small></div>`).join("") : slEmpty("Nobody has said anything on the record yet.");
  const moves = x.moves.length
    ? `<div class="sl-moves">${x.moves.map((m) => `<button class="sl-mv" data-act="slmove" data-id="${x.id}" data-code="${m.code}" data-label="${esc(m.label)}" data-public="${m.public ? 1 : 0}"><b>${esc(m.label)}</b><small>${esc(m.hint)}</small>${slChips(m.chips)}</button>`).join("")}</div>`
    : `<div class="sub small">${x.mine ? (x.open ? "Nothing for you to do here today." : "It's over.") : "It isn't yours to move: you can follow it."}</div>`;
  const links = x.links.length ? `<div class="sl-links">${x.links.map((l) => `<a class="sl-chip" data-go="storyline" data-id="${l.id}">${rich(plainText(l.q))}${l.open ? "" : " · ended"}</a>`).join("")}</div>` : '<div class="sub small">It stands on its own.</div>';
  const follow = x.canFollow ? `<button class="st-btn ${x.following ? "sl-on" : "glass"}" data-act="slfollow" data-id="${x.id}" data-on="${x.following ? 0 : 1}">${x.following ? "Following ✓" : "Follow"}</button>` : "";
  const crest = x.team ? `<span class="sl-crest">${mark(x.team, 40)}</span>` : "";
  return `<div class="pad sl">
    <header class="sl-head">${crest}<div class="sl-head-copy"><span class="st-label">${esc(x.ctx)}</span><h1 class="sl-h1 sl-hq">${rich(x.q)}</h1></div><div class="sl-head-act">${follow}</div></header>${gotoFor("storyline")}
    <div class="sl-story">
      <section class="panel sl-so">
        <div class="sl-ph"><span><b class="sl-pt">The story so far</b> <span class="sub">${esc(x.span)}</span></span><span class="sl-num big ${slHeat(x.heat)}">${x.heat}</span></div>
        ${slChart(x.chart)}
        <div class="sl-beats"><div class="sl-bt head st-label"><span>Date</span><span>Source</span><span>Beat</span><span>Heat</span></div>${beats}${next}</div>
      </section>
      <section class="panel sl-tb">
        <b class="sl-pt small">What's true, what's believed</b>
        <div class="sl-truth"><span class="st-label">What your staff knows</span>${x.truth.map((t) => `<span>${rich(t)}</span>`).join("")}</div>
        <div class="sl-belief"><span class="sl-line2"><span class="st-label">What the public believes</span><span class="muted">${x.sure}% sure</span></span><b>${rich(x.belief)}</b></div>
        <div class="sl-block"><span class="st-label">Sides</span>${sides}</div>
        <div class="sl-cast"><div class="sl-cr head st-label"><span>Cast</span><span>Side</span><span>Mood</span></div>${cast}</div>
      </section>
      <section class="panel sl-say">
        <b class="sl-pt small">What they're saying now</b>
        <div class="sl-voices">${voices}</div>
        <div class="sl-line2 baseline"><b class="sl-pt small">Your moves</b><span class="sub">each one becomes a beat</span></div>
        ${x.rule ? `<p class="sl-rule">${esc(x.rule)}</p>` : ""}
        ${moves}
        <div class="sl-block end"><span class="st-label">Linked stories</span>${links}</div>
      </section>
    </div></div>`;
};
GOTO.storyline = () => [["This week", "storylines"], ["Inbox", "inbox"], ["Media", "media"]];
const upperFirst = (s) => (s ? s.charAt(0).toUpperCase() + s.slice(1) : "");

// ---------- the Continue stop and "Previously" (30) ----------
function slOverlay(html, cls) {
  document.querySelector(".sl-scrim")?.remove();
  const o = document.createElement("div");
  o.className = "sl sl-scrim";
  o.innerHTML = `<div class="sl-sheet ${cls || ""}" role="dialog" aria-modal="true">${html}</div>`;
  document.body.appendChild(o);
  const close = () => { o.remove(); document.removeEventListener("keydown", keys, true); };
  const keys = (ev) => { if (ev.key === "Escape") { ev.preventDefault(); close(); } };
  document.addEventListener("keydown", keys, true);
  o.addEventListener("mousedown", (ev) => { if (ev.target === o) close(); });
  return { o, close };
}

async function slStop(id) {
  let x;
  try { x = await api("storylines_meeting", { id }); } catch (e) { return; }
  if (x.done || !x.opts.length) return;
  let pick = 0;
  const opts = () => x.opts.map((p, i) => `<button class="sl-opt ${i === pick ? "on" : ""}" data-sl-pick="${i}"><span><b>${esc(p.label)}</b><small>${esc(p.hint)}</small></span>${slChips(p.chips)}</button>`).join("");
  const facts = x.facts.map((f) => `<div><span class="st-label">${esc(f.k)}</span><b>${rich(f.v)}</b></div>`).join("");
  const { o, close } = slOverlay(`<div class="sl-line2"><span class="sl-stop"><i></i>Continue stopped · ${esc(x.when)}</span><span class="sub">${rich(x.q)}</span></div>
      <div class="sl-stop-copy"><b class="sl-stop-title">${rich(x.title)}</b><span>${rich(x.body)}</span></div>
      <div class="sl-facts">${facts}</div>
      <div class="sl-line2"><span class="st-label">What you say</span><span class="sub">what your staff expects it to do</span></div>
      <div class="sl-opts">${opts()}</div>
      <div class="sl-line2 foot"><span class="sub">${esc(x.later)}</span><span class="sl-btns"><button class="st-btn glass" data-sl="open">Open the story</button><button class="st-btn glass" data-sl="later">Later</button><button class="st-btn primary" data-sl="go">${esc(x.go)}</button></span></div>`, "stop");
  o.addEventListener("click", async (ev) => {
    const p = ev.target.closest("[data-sl-pick]");
    if (p) { pick = Number(p.dataset.slPick); o.querySelector(".sl-opts").innerHTML = opts(); return; }
    const b = ev.target.closest("[data-sl]");
    if (!b) return;
    if (b.dataset.sl === "later") return close();
    if (b.dataset.sl === "open") { close(); go("storyline", { id: x.story }); return; }
    b.disabled = true;
    try {
      const r = await api("people_answer", { id: x.id, code: x.opts[pick].code });
      close();
      toast(plainText((r.outcome || []).join(" ")) || "Said.");
      S.status = await api("status");
      render();
    } catch (e) { b.disabled = false; toast(e.message); }
  });
}

async function slPrevious() {
  let x;
  try { x = await api("storylines_previously"); } catch (e) { return; }
  if (!x.show) return;
  const rows = x.rows.map((p) => `<div class="sl-pv"><span><b>${rich(p.q)}</b><small>${rich(p.text)}</small></span><span class="sl-pvh"><span class="sl-num ${slHeat(p.h)}">${p.h}</span><i class="${slHeat(p.h)}"><em style="width:${p.h}%"></em></i></span><span>${rich(p.n)}</span></div>`).join("");
  const saved = S.savedOn || x.saved;
  const { o, close } = slOverlay(`<div class="sl-line2"><span class="st-label">Previously · saved ${esc(saved)}</span><span class="sub">${esc(x.rec)}</span></div>
      <b class="sl-prev-title">${rich(x.title)}</b>
      <p class="sl-prev-lead">${rich(x.body)}</p>
      ${rows ? `<div class="sl-pvs"><div class="sl-pv head st-label"><span>Open story · where it stood</span><span>Heat</span><span>What's next</span></div>${rows}</div>` : ""}
      <div class="sl-line2 foot"><span class="sub">${esc(x.foot)}</span><span class="sl-btns"><button class="st-btn glass" data-sl="week">This week</button><button class="st-btn primary" data-sl="close">Pick up where you left off</button></span></div>`, "prev");
  o.addEventListener("click", (ev) => {
    const b = ev.target.closest("[data-sl]");
    const g = ev.target.closest("[data-go]");
    if (g) { close(); return; }
    if (!b) return;
    close();
    if (b.dataset.sl === "week") go("storylines");
  });
}

// ---------- the season's chapters (31) ----------
screens.chapters = async (a) => {
  const C = keep("slchap", {});
  if (a.season) C.yr = a.season;
  const x = await api("storylines_chapters", C.yr ? { season: C.yr } : {});
  const list = ["Chapters", "Results"];
  const tab = tabOf("slchtab", list, a);
  title(`The ${x.yr} season`, tab);
  const n = x.slots.length;
  const slots = x.slots.map((g) => `<div class="sl-slot r-${g.r === "·" ? "n" : g.r}"><b>${esc(g.r)}</b><small>${esc(g.o)}</small></div>`).join("");
  // The chapters across the strip, each on the first row it fits.
  const lanes = [];
  const spans = x.chapters.map((c) => {
    let row = lanes.findIndex((end) => end < c.s0);
    if (row < 0) { row = lanes.length; lanes.push(-1); }
    lanes[row] = c.s1;
    return `<button class="sl-span t${c.tone} ${(C.ch || 1) === c.k ? "on" : ""}" data-act="slch" data-k="${c.k}" style="left:${(c.s0 / n) * 100}%;width:calc(${((c.s1 - c.s0 + 1) / n) * 100}% - 4px);top:${row * 22}px" title="${esc(plainText(c.title))}">${esc(plainText(c.title))}</button>`;
  }).join("");
  const sel = x.chapters.find((c) => c.k === (C.ch || 1)) || x.chapters[0];
  let body;
  if (tab === "Results") {
    const games = x.slots.filter((g) => g.r !== "·");
    body = games.length ? `<section class="panel"><div class="sl-results">${games.map((g, i) => `<div class="sl-res r-${g.r}"><span class="sl-d">${i + 1}</span><b>${esc(g.r)}</b><span>${esc(g.o)}</span></div>`).join("")}</div></section>` : slEmpty("No games kept for this season.");
  } else if (!x.chapters.length) {
    body = slEmpty(x.now ? "No chapter yet this season. A chapter is written when a story you're in ends after people talked about it." : "No chapters from this season.");
  } else {
    const book = x.chapters.map((c) => `<button class="sl-chr ${sel && c.k === sel.k ? "on" : ""}" data-act="slch" data-k="${c.k}"><span class="sl-chk">${c.k}</span><span><b>${rich(c.title)}</b><small>${rich(c.line || c.from)}</small></span><span class="sl-chd"><small>${esc(c.dates)}</small><b class="t${c.tone}">${esc(c.outcome)}</b></span></button>`).join("");
    const left = sel.left.length ? sel.left.map((l) => `<span class="sl-lc"><small>${esc(l.k)}</small>${rich(l.t)}</span>`).join("") : '<span class="sub small">Nothing kept.</span>';
    const paid = sel.paid.map((p) => `<span class="sl-line2"><span>${esc(p.k)}</span><b class="t${p.tone}">${esc(p.v)}</b></span>`).join("");
    body = `<div class="sl-chapters">
      <section class="panel sl-book"><div class="sl-ph"><b>The book of ${x.yr}</b><span class="sub">written from the stories that ended</span></div>${book}</section>
      <section class="panel sl-chapter">
        <div class="sl-line2"><span class="st-label">Chapter ${sel.k} · ${esc(sel.dates)} · from “${rich(plainText(sel.from))}”</span><span class="sub">${esc(sel.fam)}</span></div>
        <b class="sl-ch-title">${rich(sel.title)}</b>
        <p class="sl-ch-text">${rich(sel.text)}</p>
        <div class="sl-ch-cols">
          <div><span class="st-label">The beats that mattered</span>${sel.beats.map((b) => `<div class="sl-up"><span>${esc(b.d)}</span><span>${rich(b.text)}</span></div>`).join("")}<p class="sub small">In it: ${esc(sel.cast)}</p></div>
          <div class="sl-ch-side"><div><span class="st-label">What it left</span><div class="sl-lcs">${left}</div></div>
            <div class="sl-truth"><span class="st-label">What it paid you</span>${paid}<small>Lesson points, 0.25 to 1.00 a story by its heat, go into the ratings the story tested.</small></div></div>
        </div>
      </section></div>`;
  }
  const crest = x.team ? `<span class="sl-crest">${mark(x.team, 40)}</span>` : "";
  const years = x.years.length > 1 ? `<select class="sl-year" data-sl-year>${x.years.map((y) => `<option value="${y}" ${y === x.yr ? "selected" : ""}>${y}</option>`).join("")}</select>` : `<span class="st-pill">${x.yr}</span>`;
  return `<div class="pad sl">
    <header class="sl-head">${crest}<div class="sl-head-copy"><span class="st-label">${esc(x.ctx)}</span><h1 class="sl-h1">The ${x.yr} season</h1></div>${tabs("slchtab", list, tab)}<div class="sl-head-act">${years}</div></header>${gotoFor("chapters")}
    <section class="panel sl-strip">
      <div class="sl-line2 baseline"><span><b>${esc(x.rec)}</b> <span class="sub">${esc(x.recSub)}</span></span><span class="sub">each bar is a chapter: the story it came from, from its spark to its end</span></div>
      <div class="sl-slots" style="grid-template-columns:repeat(${n},minmax(0,1fr))">${slots}</div>
      <div class="sl-spans" style="height:${Math.max(1, lanes.length) * 22}px">${spans}</div>
    </section>
    ${body}</div>`;
};
GOTO.chapters = () => [["Career story", "story"], ["This week", "storylines"], ["Schedule", "schedule"], ["History", "history"]];

// ---------- your career's chapters (33), a tab on the career story ----------
async function slCareerTab() {
  const x = await api("storylines_career");
  if (x.none) return slEmpty("You need a job first.");
  const C = keep("slcareer", {});
  const span = Math.max(1, x.to - x.from);
  const at = (y) => ((y - x.from) / span) * 100;
  const jobs = x.jobs.map((j) => `<div class="sl-job ${j.now ? "now" : ""}" style="left:${at(j.from)}%;width:${Math.max(4, at(j.to) - at(j.from))}%" title="${esc(j.role)}, ${esc(j.name)}"><b>${esc(j.name)}</b><small>${esc(j.role)}</small></div>`).join("");
  const sel = x.chapters.find((c) => c.k === C.ch) || x.chapters[x.chapters.length - 1];
  // Chapters close together are nudged apart so no dot covers another.
  const xs = [];
  x.chapters.forEach((c, i) => xs.push(Math.max(Math.min(98.5, Math.max(1.5, at(c.at))), i ? xs[i - 1] + 2.4 : -99)));
  for (let i = xs.length - 1; i >= 0; i--) xs[i] = Math.min(xs[i], i === xs.length - 1 ? 98.5 : xs[i + 1] - 2.4);
  const dots = x.chapters.map((c, i) => `<button class="sl-dot t${c.tone} ${sel && c.k === sel.k ? "on" : ""}" data-act="slcar" data-k="${c.k}" style="left:${xs[i].toFixed(2)}%" title="${esc(plainText(c.title))}">${c.k}</button>`).join("");
  const years = [];
  for (let y = Math.ceil(x.from); y <= Math.floor(x.to); y += span > 14 ? 2 : 1) years.push(`<span style="left:${at(y)}%">${y}</span>`);
  const book = x.chapters.length
    ? x.chapters.slice().reverse().map((c) => `<button class="sl-chr ${sel && c.k === sel.k ? "on" : ""}" data-act="slcar" data-k="${c.k}"><span class="sl-chk">${c.k}</span><span><b>${rich(c.title)}</b><small>${rich(c.line || c.q)}</small>${sel && c.k === sel.k && c.gave.length ? `<span class="sl-gave">${c.gave.map((g) => `<em class="t${g.tone}">${rich(g.t)}</em>`).join("")}</span>` : ""}</span><span class="sl-chd"><small>${c.season}</small></span></button>`).join("")
    : slEmpty("The book is empty. A chapter is written when a story you're in ends after people talked about it.");
  const mi = Math.min(C.mem || 0, Math.max(0, x.mems.length - 1));
  const shelf = x.mems.map((m, i) => `<button class="sl-mem ${i === mi ? "on" : ""}" data-act="slmem" data-i="${i}" title="${esc(plainText(m.text))}">${soGlyph(m.icon)}<small>${esc(m.label)}</small></button>`);
  const rows = [shelf.slice(0, 6).join(""), shelf.slice(6, 12).join("")];
  const m = x.mems[mi];
  const memo = m ? `<div class="sl-memo"><span class="st-label">${esc(m.label)} · ${esc(m.date)} ${m.year}</span><span>${rich(m.text)}</span>${m.chapter ? `<small>From chapter ${m.chapter}</small>` : ""}</div>` : '<div class="sl-memo"><span class="sub">The shelves fill as the years go: game balls, rings, framed front pages.</span></div>';
  const names = x.names.length ? x.names.map((n) => `<div class="sl-nm"><span><b class="t${n.tone}">${esc(n.name)}</b><small>${esc(n.who)}</small></span><span class="sl-d">${n.when}</span></div>`).join("") : '<div class="sub small">No name has stuck to you yet.</div>';
  return `<div class="sl">
    <section class="panel sl-strip">
      <div class="sl-line2 baseline"><span><b>${esc(x.rec)}</b> <span class="sub">${esc(x.recSub)}</span></span><span class="sub">each dot is a career chapter; pick one to read it</span></div>
      <div class="sl-career-line">${jobs}${dots}</div><div class="sl-years">${years.join("")}</div>
    </section>
    <div class="sl-career">
      <section class="panel sl-book"><div class="sl-ph"><b>The book</b><span class="sub">chapters from the stories that shaped you</span></div>${book}</section>
      <section class="panel sl-office"><div class="sl-ph"><b>Your office</b><span class="sub">${x.office} thing${x.office === 1 ? "" : "s"} on the shelves · ${linkTo("office", {}, "the whole office")}</span></div>
        <div class="sl-shelves"><div class="sl-shelf">${rows[0] || '<span class="sub small">Room for the first keepsake.</span>'}</div><div class="sl-shelf">${rows[1]}</div></div>
        <div class="sl-office-foot">${memo}<div><span class="st-label">Names you've been given</span>${names}${x.fixer ? `<p class="sub small">The papers call you “${esc(x.fixer)}”.</p>` : ""}</div></div>
      </section>
    </div></div>`;
}

// ---------- pick your next job (32) ----------
screens.jobpick = async (a) => {
  const x = await api("storylines_jobs");
  title("Pick your next job");
  if (x.none) return `<div class="pad">${slEmpty("No career yet.")}</div>`;
  if (!x.jobs.length) return `<div class="pad sl"><header class="sl-head"><div class="sl-head-copy"><span class="st-label">${esc(x.ctx)}</span><h1 class="sl-h1">Pick your next job</h1></div></header>${slEmpty("No offer and no head job you'd take is open right now. Openings come with the carousel: firings from late October, Black Monday in January, high school boards after the season.")}${gotoFor("jobpick")}</div>`;
  const P = keep("sljob", { k: 0 });
  if (a.index != null) P.k = Math.max(0, x.jobs.findIndex((j) => j.k === a.index));
  const ji = Math.min(P.k, x.jobs.length - 1);
  const J = x.jobs[ji];
  const bars = (w) => [0, 1, 2, 3].map((i) => `<i class="${i <= w ? `w${w}` : ""}"></i>`).join("");
  const list = x.jobs.map((j, i) => `<button class="sl-jr ${i === ji ? "on" : ""}" data-act="sljob" data-i="${i}"><span><b>${esc(j.name)}</b><small>${esc(j.sub)}</small><small class="muted">${esc(j.calls)}</small></span><span class="sl-jw"><b class="w${j.w}">${esc(j.word)}</b><span class="sl-vm">${bars(j.w)}</span><small>${j.n} stor${j.n === 1 ? "y" : "ies"}</small></span></button>`).join("");
  const stories = J.stories.length
    ? J.stories.map((s) => `<div class="sl-js"><div class="sl-line2"><b>${rich(s.q)}</b><span class="sl-num ${slHeat(s.heat)}">${s.heat}</span></div><div class="sl-line2 left"><span class="sl-chip">${esc(s.phase)}</span><span class="sl-lp">${esc(s.lesson)} lesson points · ${esc(s.tests)}</span></div><div class="sl-rd"><div><span class="st-label">The public</span><span>${rich(s.public)}</span></div><div><span class="st-label">${esc(J.boss)}</span><em>${rich(s.spin)}</em></div></div></div>`).join("")
    : slEmpty("Nothing is being said about it: a calm job, judged as usual.");
  const terms = J.terms.map((t) => `<div class="sl-of"><span class="muted">${esc(t.k)}</span><b>${esc(t.a)}</b><span class="muted">${esc(t.b)}</span></div>`).join("");
  const act = J.offer
    ? `<button class="st-btn primary" data-fs="accept" data-k="${J.k}">Take the job</button><button class="st-btn glass" data-fs="decline" data-k="${J.k}">Pass</button>`
    : J.ref ? `<button class="st-btn primary" data-fs="apply" data-kind="${J.ref.kind}" data-id="${J.ref.id}" data-role="${esc(J.role_key)}">Put your name in</button>` : "";
  return `<div class="pad sl">
    <header class="sl-head"><div class="sl-head-copy"><span class="st-label">${esc(x.ctx)}</span><h1 class="sl-h1">Pick your next job</h1></div><div class="sl-head-act">${linkTo("market", {}, "The job market", "st-btn glass")}</div></header>${gotoFor("jobpick")}
    <div class="sl-jobs">
      <section class="panel sl-jl"><div class="sl-ph"><b class="sl-pt">Openings</b><span class="sub">${x.jobs.length} for you</span></div>${list}
        <div class="sl-cardlet"><span class="st-label">How the market sees you</span><span class="sl-line2 left baseline"><b>${esc(x.rep)}</b><span class="sub">${esc(x.repSub)}</span></span><em>${esc(x.agent)}</em><small class="muted">${esc(x.agentWho)}</small></div>
        <div class="sl-cardlet"><span class="st-label">What the word means</span><span>Each job's word comes from the stories it carries: how many, how hot, and who is in them. Calm, Unsettled, Volatile, On fire.</span><span>The hotter the job, the better the offer and the more a story pays when you handle it. Handled badly, it costs reputation and patience.</span></div>
      </section>
      <section class="panel sl-jm"><div class="sl-line2 top"><span><span class="st-label">What you would walk into</span><b class="sl-jn">${esc(J.name)}</b><small class="muted">${esc(J.sub)}</small></span><span class="sl-jword"><b class="w${J.w}">${esc(J.word)}</b><small class="muted">${J.n} stor${J.n === 1 ? "y" : "ies"} it carries</small></span></div>${stories}<p class="sub small">${esc(x.note)}</p></section>
      <section class="panel sl-jo"><div class="sl-ph"><b class="sl-pt">The offer</b><span class="sub">against a calm job</span></div>
        <div class="sl-terms"><div class="sl-of head st-label"><span>Terms</span><span>${esc(J.name)}</span><span>A calm job</span></div>${terms}</div>
        ${J.premium.length ? `<p class="sub small">${esc(J.premium.join(" · "))}</p>` : ""}
        <div class="sl-rws"><span class="st-label gold">If you handle it</span>
          <div class="sl-rw"><span>Reputation, if its stories end well</span><b class="${/^up to/.test(J.rep) ? "t0" : ""}">${esc(J.rep)}</b></div>
          <div class="sl-rw"><span>Lesson points, every story, no season cap</span><b class="${/^\+/.test(J.lessons) ? "t0" : ""}">${esc(J.lessons)}</b></div>
          <div class="sl-rw"><span>Loyal ties</span><span>${esc(J.ties)}</span></div>
          <div class="sl-rw"><span>A career chapter</span><span>${esc(plainText(J.chapter))}</span></div>
          <div class="sl-rw"><span>An office keepsake</span><span>${esc(J.memento)}</span></div>
          <div class="sl-rw"><span>A name</span><span>${esc(J.name_given)}</span></div></div>
        <div class="sl-rws"><span class="st-label">If it goes badly</span>
          <div class="sl-rw"><span>Reputation</span><b class="${/^\u2212/.test(J.bad) ? "t2" : ""}">${esc(J.bad)}</b></div>
          <div class="sl-rw"><span>Patience</span><span class="t2">${esc(J.badP)}</span></div></div>
        <div class="sl-cardlet"><span class="st-label">What leaving starts</span><span>${esc(x.leave)}</span></div>
        <div class="sl-btns">${act}</div>
      </section>
    </div></div>`;
};
GOTO.jobpick = () => [["Job market", "market"], ["Your profile", "you"], ["Career story", "story"]];

// ---------- clicks ----------
Object.assign(CORE_MORE, {
  slfollow: async (el) => {
    try {
      const r = await api("storylines_follow", { id: Number(el.dataset.id), on: el.dataset.on === "1" });
      toast(r.following ? "Following: its beats reach your inbox until it ends." : "Stopped following it.");
      refresh(el);
    } catch (e) { toast(e.message); }
  },
  slmove: async (el) => {
    const pub = el.dataset.public === "1";
    const ok = await sure(el.dataset.label, pub ? "Said on the record: the papers and everyone in the story hear it." : "In private: it stays inside the building, unless someone talks.", pub ? "Say it" : "Do it");
    if (!ok) return;
    try {
      const r = await api("storylines_move", { id: Number(el.dataset.id), code: Number(el.dataset.code) });
      toast(plainText((r.said || []).join(" ")));
      S.status = await api("status");
      refresh(el);
    } catch (e) { toast(e.message); }
  },
  slch: (el) => { keep("slchap", {}).ch = Number(el.dataset.k); refresh(el); },
  slcar: (el) => { keep("slcareer", {}).ch = Number(el.dataset.k); refresh(el); },
  slmem: (el) => { keep("slcareer", {}).mem = Number(el.dataset.i); refresh(el); },
  sljob: (el) => { keep("sljob", { k: 0 }).k = Number(el.dataset.i); S.route.args = {}; refresh(el); },
});
document.addEventListener("change", (ev) => {
  const el = ev.target;
  if (el.matches && el.matches("[data-sl-year]")) { keep("slchap", {}).yr = Number(el.value); keep("slchap", {}).ch = 1; S.route.args = {}; refresh(el); }
});
// A card opens its story from the keyboard too.
document.addEventListener("keydown", (ev) => {
  if (ev.key !== "Enter") return;
  const c = ev.target.closest && ev.target.closest(".sl [role=link][data-go]");
  if (c) { ev.preventDefault(); c.click(); }
});
