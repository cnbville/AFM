/* The hunt (next update, lane 11, SK67) in Press Box: one target chased
 * over seasons, at every seat.
 *
 *   yours    his card (your staff's read and yours, potential in words),
 *            the cells (games watched, who knows, his say, your boss), his
 *            games, who knows and from what, what you banked, the rivals
 *            the world knows of, the hunt so far.
 *   none     who you could hunt, read by your staff.
 *   always   the world's chases and the generational prospects, your own
 *            players others hunt, your past hunts and their look-backs.
 * Every class is hn- so the app's global names never restyle it. Only
 * reads show: ranges, words and counts. */
"use strict";

I.hunt = '<circle cx="12" cy="12" r="8"/><circle cx="12" cy="12" r="3.5"/><path d="M12 1v4M12 19v4M1 12h4M19 12h4"/>';
navAdd({ go: "hunt", label: "The hunt", icon: "hunt", sec: "scout", after: "film", seats: ["nfl", "college", "hs"] });
CRUMBS.hunt = "The hunt";
GOTO.hunt = () => (seatKind() === "nfl"
  ? [["Draft board", "draftboard"], ["Film room", "film"], ["Find prospects", "find_prospects"]]
  : seatKind() === "college"
    ? [["Recruiting", "recruiting"], ["Film room", "film"], ["Evaluation", "evaluate"]]
    : [["Squad", "roster"], ["Film room", "film"], ["Colleges calling", "hsside"]]);

const hnBtn = (act, data, html, cls = "") => `<button class="st-btn small ${cls || "glass"}" data-act="${act}" ${Object.entries(data).map(([k, v]) => `data-${k}="${esc(String(v))}"`).join(" ")}>${html}</button>`;
const hnCell = (label, big, cap, cls = "") => `<div class="hn-cell ${cls}"><div class="st-label">${esc(label)}</div><b>${big}</b>${cap ? `<small>${cap}</small>` : ""}</div>`;
const hnPanel = (title, side, body, cls = "") => `<section class="hn-panel ${cls}"><div class="hn-ph"><b>${esc(title)}</b>${side ? `<span>${side}</span>` : ""}</div>${body}</section>`;
const hnTier = (pt) => (pt ? `${esc(pt.tier)} <small class="hn-dim">${esc(pt.sure)}</small>` : '<span class="hn-dim">Not seen</span>');
const hnGen = (g) => (g ? `<span class="hn-chip gold" title="${esc(g.since ? `The best ${g.draft ? "the scouts have read" : "the services have seen"} since ${g.since.name} (${g.since.year})` : "The best the public has ever read")}">Generational</span>` : "");
const hnTeam = (t) => (t ? `${mark(t, 18)} ${teamLink(t)}` : '<span class="hn-dim">-</span>');
const HN_END = { landed: ["Landed", "gold"], lost: ["Lost", "amber"], refused: ["Refused you", "amber"], dropped: ["Let go", ""], boss: ["Ended by your boss", "amber"], gone: ["Gone", ""] };
const HN_WHY = { own: "", piled: "Joined after word got out", generational: "The generational chase" };

screens.hunt = async () => {
  const x = await api("hunt_page", {});
  const y = x.yours;
  title("The hunt", y ? y.player.name : x.team.name);
  const sub = { nfl: "One man chased to the draft: watch every game, bank the picks, keep your hand hidden.", college: "One recruit chased to signing day: watch every game, bank the money and the scholarship, keep it quiet.", hs: "One kid chased to the transfer season: his Friday nights on film, his family in the off-season, and the risk of being found out." }[x.seat];
  const head = `<div class="hn-head">${mark(x.team, 44)}<div><div class="st-label">The hunt · ${esc(x.team.name)}</div><h1>${y ? playerLink(y.player.id, y.player.name) : "Name your hunt"}</h1><small>${esc(sub)}</small></div></div>`;
  const body = y ? hnYours(x, y) : hnNone(x);
  return `<div class="pad sl hn">${head}${body}${hnWorld(x)}${hnPast(x)}</div>`;
};

function hnYours(x, y) {
  const p = y.player;
  const card = `<section class="hn-panel hn-hero"><div class="hn-who">${hnTeam(p.team)}<span class="hn-dim">·</span><span>${esc(p.pos)}</span><span class="hn-dim">·</span><span>${esc(p.class)}</span>${p.stars ? `<span>${stars(p.stars)}</span>` : ""}${hnGen(p.generational)}</div>
    <div class="hn-reads"><div><div class="st-label">Your staff</div>${range(p.staff, null, { p: p.id, k: "ovr" })}</div><div><div class="st-label">Yours</div>${yourRange(p.mine)}</div><div><div class="st-label">Potential</div><span>${hnTier(p.potential)}</span></div></div>
    <p class="hn-note">${esc(y.deadline)} Your hunt since ${esc(y.since)}${y.loud ? `, out loud since ${esc(y.loud)}` : ", kept quiet"}.</p>
    <div class="hn-acts"><button class="st-btn small glass" data-go="film" data-id="${p.id}">Watch his film now</button>${y.loud ? "" : hnBtn("hnloud", {}, "Say it out loud")}${hnBtn("hndrop", { name: p.name }, "Let him go")}</div></section>`;
  const heatCls = y.heat >= 60 ? "amber" : "";
  const cells = `<div class="hn-cells">${hnCell("His games", `${y.games_seen}`, `watched${y.games_missed ? `, ${y.games_missed} missed` : ""} · ${y.film_left} of ${y.film_week} film hours left this week`)}
    ${hnCell("Who knows", esc(y.heat_words), `<span class="hn-bar"><i class="${heatCls}" style="width:${Math.max(3, y.heat)}%"></i></span>`, heatCls)}
    ${hnCell("His say", esc(y.say.words), y.say.committed ? `Committed to ${teamLink(y.say.committed)}` : y.say.offers != null ? `${y.say.offers} offer${y.say.offers === 1 ? "" : "s"}` : "")}
    ${y.boss ? hnCell(`Your ${y.boss.title ? y.boss.title.toLowerCase() : "boss"}`, esc(y.boss.name), esc(y.boss.words)) : ""}</div>`;
  const games = y.watched.length ? `<div class="hn-tbl"><table><thead><tr><th>Game</th><th>Against</th><th class="n"></th></tr></thead><tbody>${y.watched.map((g) => `<tr><td>${esc(g.date)}</td><td>${esc(g.against)}</td><td class="n">${g.seen ? '<span class="hn-on">Watched</span>' : '<span class="hn-amber">Missed</span>'}</td></tr>`).join("")}</tbody></table></div>` : `<div class="hn-empty">No game of his since you named him. The day after each game you watch his film, from your own hours.</div>`;
  const signals = y.signals.length ? `<ul class="hn-list">${y.signals.map((s) => `<li><span class="hn-dim">${esc(s.date)}</span><span>${esc(s.what)}</span><b>${s.weight}</b></li>`).join("")}</ul>` : `<div class="hn-empty">Nothing yet: nobody has a reason to think you want him.</div>`;
  const rivals = y.rivals.length ? `<ul class="hn-list">${y.rivals.map((r) => `<li>${hnTeam(r.team)}<small>${esc(HN_WHY[r.why] || `since ${r.since}`)}</small></li>`).join("")}</ul>` : `<div class="hn-empty">Nobody the world knows of. That can change: what leaks of your interest draws rivals.</div>`;
  const side = hnBank(x, y) + hnPanel("Rivals", "who the world knows is after him", rivals) + hnPanel("The hunt so far", "", `<ul class="hn-list">${y.log.map((l) => `<li><span class="hn-dim">${esc(l.date)}</span><span>${esc(l.text)}</span></li>`).join("")}</ul>`);
  const main = hnPanel("His games", "his film the day after, from your hours", games) + hnPanel("Who knows you want him", "each sign adds to what's left hidden", signals);
  return `${card}${cells}<div class="hn-main"><div class="hn-col">${main}</div><div class="hn-side">${side}</div></div>`;
}

function hnBank(x, y) {
  const b = y.bank;
  if (x.seat === "college") {
    const nil = b.nil_era
      ? `<div class="hn-kv"><span>Set aside</span><span><b>${money(b.nil) || "$0"}</b> a year · ${money(b.left)} left in your budget</span><span>Your promise</span><span>${b.promise != null ? money(b.promise) : "None yet: the collective's usual offer"}</span></div>
        <div class="hn-row"><input class="hn-in" type="number" min="0" step="25000" value="${Math.round(b.nil)}" aria-label="NIL money set aside a year">${hnBtn("hnnil", {}, "Set aside")}</div>`
      : `<div class="hn-empty">No NIL money before July 2021: what you have is the scholarship and your time.</div>`;
    const hold = `<label class="hn-check"><input type="checkbox" data-act="hnhold" ${b.hold ? "checked" : ""}> Hold a scholarship at his position: the others there share one fewer until he decides.</label>`;
    return hnPanel("What you bank", "money and a place, kept for him", nil + hold);
  }
  if (x.seat === "nfl") {
    const pts = (v) => (v == null ? "-" : Math.round(v).toLocaleString());
    return hnPanel("What you bank", `the ${b.draft} draft, on your GM's chart`, `<div class="hn-kv"><span>His place</span><span>${b.rank ? `No. ${b.rank} on the public's board` : "Not in the public's top 50"}</span>
      <span>Your first pick</span><span>${b.first ? `No. ${b.first}` : "The order isn't set yet"}</span>
      <span>To be sure of him</span><span>${b.gap == null ? "-" : b.gap > 0 ? `${pts(b.gap)} points over your first pick` : "Your pick comes before his place"}</span>
      <span>Your picks</span><span>${pts(b.this_year)} points this draft, ${pts(b.next_year)} next</span></div>
      <p class="hn-note">Trades for picks bank more; on draft day the war room can trade up.</p>`);
  }
  return hnPanel("What you bank", "a pitch to his family", `<div class="hn-kv"><span>Pitches left</span><span>${b.pitches_left} this season</span><span>His family</span><span>${esc(y.say.words)}</span><span>Talked to them</span><span>${y.say.pitched ? esc(y.say.pitched) : "Not yet: recruit him from his player page"}</span></div>
    <p class="hn-note">Most states bar recruiting another school's player. The more people know you wanted him, the likelier it comes out.</p>`);
}

function hnNone(x) {
  const rows = (x.suggest || []).map((p) => `<tr><td>${playerLink(p.id, p.name)} ${hnGen(p.generational)}</td><td>${esc(p.pos)}</td><td>${esc(p.class)}</td><td>${hnTeam(p.team)}</td><td>${range(p.staff, null, { p: p.id, k: "ovr" })}</td><td>${yourRange(p.mine)}</td><td>${hnTier(p.potential)}</td><td class="hn-dim">${esc(p.why)}</td><td class="n">${hnBtn("hnname", { id: p.id, name: p.name }, "Hunt him")}</td></tr>`).join("");
  const how = { nfl: "Name one college player and the season bends around him: you watch every game he plays, bank the picks it will take to reach him, and try to keep the clubs ahead of you from learning you want him. His camp can refuse you.", college: "Name one recruit and the cycle bends around him: you watch every game he plays, hold him a scholarship and set money aside, and your own time goes to him first. Offers, visits and trips get around, and what gets around draws rivals. He can cut you from his list.", hs: "Name one kid in your state and the year bends around him: you watch his Friday nights on film, and the off-season is when his family gets to know you. It's against the rules almost everywhere, and the more people know, the likelier it comes out." }[x.seat];
  return hnPanel("Who you could hunt", "read by your staff, best first", `<p class="hn-note">${esc(how)} One at a time.</p>${rows ? `<div class="hn-tbl"><table><thead><tr><th>Player</th><th>Pos</th><th>Class</th><th>Team</th><th>Staff read</th><th>Yours</th><th>Potential</th><th>Why</th><th></th></tr></thead><tbody>${rows}</tbody></table></div>` : `<div class="hn-empty">Nobody to suggest yet. Any player page has the button once he's someone you can hunt.</div>`}`, "hn-wide");
}

function hnWorld(x) {
  const w = x.world;
  const gens = w.generational.length ? `<ul class="hn-list">${w.generational.map((g) => `<li>${playerLink(g.id, g.name)} <small>${esc(g.pos)} · ${g.draft ? `${g.year} draft` : `class of ${g.year}`}</small><small class="hn-dim">${g.since ? `best since ${esc(g.since.name)} (${g.since.year})` : "the best on record"}</small><b>${g.hunters} hunting</b></li>`).join("")}</ul>` : `<div class="hn-empty">None in this world yet. Once or twice a decade the public reads a prospect above every class in nine years, and everyone chases him.</div>`;
  const chases = w.chases.length ? `<div class="hn-tbl"><table><thead><tr><th>Player</th><th>Pos</th><th>Class</th><th>Team</th><th>Staff read</th><th>Known hunters</th></tr></thead><tbody>${w.chases.map((p) => `<tr><td>${playerLink(p.id, p.name)} ${hnGen(p.generational)}</td><td>${esc(p.pos)}</td><td>${esc(p.class)}</td><td>${hnTeam(p.team)}</td><td>${range(p.staff, null, { p: p.id, k: "ovr" })}</td><td class="hn-marks">${p.hunters.map((t) => `<a data-go="team" data-kind="${t.ref.kind}" data-id="${t.ref.id}" title="${esc(t.name)}">${mark(t, 20)}</a>`).join("")}</td></tr>`).join("")}</tbody></table></div>` : `<div class="hn-empty">No chase the public knows of at your level right now.</div>`;
  const hunted = (x.hunted || []).length ? hnPanel(x.seat === "hs" ? "Your players others hunt" : "Your players the clubs hunt", "the teams known to be after them", `<div class="hn-tbl"><table><thead><tr><th>Player</th><th>Pos</th><th>Class</th><th>Staff read</th><th>Known hunters</th></tr></thead><tbody>${x.hunted.map((p) => `<tr><td>${playerLink(p.id, p.name)} ${hnGen(p.generational)}</td><td>${esc(p.pos)}</td><td>${esc(p.class)}</td><td>${range(p.staff, null, { p: p.id, k: "ovr" })}</td><td class="hn-marks">${p.hunters.map((t) => `<a data-go="team" data-kind="${t.ref.kind}" data-id="${t.ref.id}" title="${esc(t.name)}">${mark(t, 20)}</a>`).join("")}</td></tr>`).join("")}</tbody></table></div>`) : "";
  return `<div class="hn-main hn-gap"><div class="hn-col">${hnPanel("The world's chases", "the most-hunted the public knows of", chases)}${hunted}</div><div class="hn-side">${hnPanel("Generational prospects", "the public's word, not a promise", gens)}</div></div>`;
}

function hnPast(x) {
  if (!x.past.length) return "";
  const rows = x.past.map((h) => {
    const [label, cls] = HN_END[h.end] || [h.end, ""];
    const lb = h.look_back ? `<span class="hn-chip ${h.look_back.hit ? "gold" : "amber"}" title="${esc(h.look_back.date)}">${h.look_back.hit ? "Paid off" : "Missed"}</span> <small>${esc(h.look_back.words)}</small>` : h.end === "landed" ? '<small class="hn-dim">Three years will tell</small>' : "";
    return `<tr><td>${playerLink(h.id, h.name)}</td><td>${esc(h.pos)}</td><td>${esc(h.since)}</td><td>${esc(h.date)}</td><td><span class="hn-chip ${cls}">${esc(label)}</span></td><td class="hn-why">${esc(h.why)}</td><td class="n">${h.games}</td><td>${lb}</td></tr>`;
  }).join("");
  return hnPanel("Your past hunts", "and what became of them", `<div class="hn-tbl"><table><thead><tr><th>Player</th><th>Pos</th><th>Named</th><th>Ended</th><th>How</th><th>Why</th><th class="n">Games</th><th>Look-back</th></tr></thead><tbody>${rows}</tbody></table></div>`, "hn-wide hn-gap");
}

// The player page's part (core.js calls it beside "Stake your name"): the
// pills. Naming him is one of his moves (lane_core/moves.rs), on the hero's
// More, the right-click menu and the rows' ⋯.
function l11Hunt(p) {
  const h = p.hunt;
  if (!h) return "";
  const gen = h.generational ? `<span class="st-pill gold" title="${esc(h.generational.since ? `The public's best since ${h.generational.since.name} (${h.generational.since.year})` : "The best the public has read")}">Generational</span>` : "";
  return h.on ? `${gen}<a class="st-pill gold" data-go="hunt" title="Your hunt: his page in The hunt">Your hunt</a>` : gen;
}

// Names him your hunt, letting the one you have go if you say so.
async function l11Name(id, name, busy) {
  if (busy && !(await sure(`Let ${busy} go?`, `Your hunt is ${busy}. Naming ${name} ends that chase; it goes to your record.`, `Hunt ${name}`))) return;
  try {
    if (busy) await api("hunt_drop", {});
    toast((await api("hunt_name", { id: Number(id) })).text);
  } catch (e) { toast(e.message || String(e)); }
  peekForget();
  refresh();
}

Object.assign(CORE_MORE, {
  hnname: (el) => l11Name(el.dataset.id, el.dataset.name, el.dataset.busy),
  hndrop: async (el) => {
    if (!(await sure(`Let ${el.dataset.name} go?`, "The hunt goes to your record, and your staff's time goes back to the board.", "Let him go"))) return;
    try { toast((await api("hunt_drop", {})).text); } catch (e) { toast(e.message); }
    refresh(el);
  },
  hnloud: async (el) => {
    if (!(await sure("Say it out loud?", "The press has it today, and everyone will know you want him. If he becomes what you said, it counts for more; if not, it counts against you.", "Say it"))) return;
    try { toast((await api("hunt_loud", {})).text); } catch (e) { toast(e.message); }
    refresh(el);
  },
  hnnil: async (el) => {
    const v = Number(el.closest(".hn-row").querySelector(".hn-in").value);
    try { await api("hunt_bank", { nil: Math.max(0, v || 0) }); toast("Set aside."); } catch (e) { toast(e.message); }
    refresh(el);
  },
  hnhold: async (el) => {
    try { await api("hunt_bank", { hold: !!el.checked }); } catch (e) { toast(e.message); }
    refresh(el);
  },
});

// The Handbook's page (lane 8's hbAdd), from docs/reference/how-it-works/hunt.md.
if (typeof hbAdd === "function") {
  hbAdd({
    id: "hunt", group: "Building the team", title: "The hunt",
    lede: "You name one player as your hunt and chase him over seasons, at any level and position. The world hunts too, and once or twice a decade it all chases one prospect.",
    how: [
      ["You watch every game", "The day after each of his games you watch his film, from your own film hours. A week with no hours left is a game missed."],
      ["You bank what he'll cost", "At a college, money set aside and a scholarship held at his spot. At a club, the picks to reach his place on the board. At a high school, a pitch and his family's trust."],
      ["Keep it quiet", "Offers, visits, trips the press saw, leaks and time all tell the world you want him, and rivals follow what they hear. Saying it out loud tells everyone."],
      ["He has a say", "A recruit cuts schools from his list, a top prospect's camp can refuse a club, a family says no. Your boss watches a chase the world knows about, and three years on the look-back says if he was what the hints said."],
    ],
    lv: {
      hs: "You chase a kid at another school in your state for the June transfer season. Most states bar it: if it comes out, your school forfeits the games he played and you sit a year. Private powers hunt your stars too.",
      college: "You chase a recruit from his sophomore year to signing day. Your own time goes to him first, and the others on your board get a little less of it.",
      nfl: "You chase a college player to the draft. Clubs that hunt the same man trade up for him; a refused pick's rights go to the club that wants him most.",
    },
    where: [["The hunt", "hunt"], ["Film room", "film"], ["News", "news"]],
    rel: ["scouting", "recruiting", "draft"],
  });
  hbScreen("hunt", "hunt", [
    ["Who knows", "How much the world knows you want him, and from what.", "Who knows"],
    ["His say", "Where you stand with him, in your staff's words.", "His say"],
    ["What you bank", "What you keep for him: money, a place, picks or a pitch.", "What you bank"],
    ["The world's chases", "The most-hunted players the public knows of, and the generational prospects.", "The world's chases"],
  ]);
}
