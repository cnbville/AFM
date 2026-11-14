/* The CFL draft from a CFL club's chair (SK77), on Showtime: your board
 * from your club's file, the Bureau's three editions, the pre-draft season
 * (the combines and the Canadians down south), the draft room (your pick,
 * your staff's pick, calls for your pick) and the Global Draft. Only your
 * staff's reads show: ranges, words and counts. Classes are cf- so the
 * app's global names never restyle them. */
"use strict";

navAdd({ go: "cfldraft", label: "CFL Draft", icon: "board", sec: "scout", seats: ["other"], show: () => !!(S.status && S.status.cfl), badge: () => (S.status && S.status.cfl_live ? "Live" : 0) });
CRUMBS.cfldraft = "CFL Draft";
GOTO.cfldraft = () => [["Department", "scouting"], ["Evaluation", "evaluate"], ["Film room", "film"]];

const CF = { said: "" };
const cfBtn = (act, data, html, cls = "") => `<button class="st-btn small ${cls || "glass"}" data-act="${act}" ${Object.entries(data).map(([k, v]) => `data-${k}="${esc(String(v))}"`).join(" ")}>${html}</button>`;
const cfWho = (r) => `${scName(r.id, r.name)}<small class="cf-sub">${esc(r.pos)} · ${esc(r.from)}${r.note ? ` · ${esc(r.note)}` : ""}</small>`;
const cfDash = '<span class="muted">-</span>';
const cfPanel = (h, side, body, cls = "") => `<section class="cf-panel ${cls}"><h2>${esc(h)}${side ? ` <small>${side}</small>` : ""}</h2>${body}</section>`;
const cfTable = (head, rows, empty) => (rows.length ? `<div class="cf-scroll"><table class="tbl cf-tbl"><thead><tr>${head.map(([t, l]) => `<th class="${l ? "l" : "num"}">${esc(t)}</th>`).join("")}</tr></thead><tbody>${rows.join("")}</tbody></table></div>` : `<div class="empty">${esc(empty)}</div>`);

screens.cfldraft = async (a) => {
  const x = await api("cfl_draft");
  title("CFL Draft", String(x.year));
  if (x.club && x.club.colors) setColors(x.club.colors);
  if (S.status) S.status.cfl_live = !!x.live;
  const list = [["Board", "Your board", x.board.length], ["Bureau", "Bureau", x.bureau.length || null], ["Pre", "Pre-draft"], ["Room", x.live ? "Draft room" : "The picks", x.live ? "Live" : x.national_picks.length || null]];
  if (x.has_global) list.push(["Global", "Global Draft", x.global_picks.length || null]);
  // The clock stops on your pick and opens the room.
  if (a && a.cfltab && S.route.args) { S.route.args.tab = a.cfltab; delete S.route.args.cfltab; }
  const tab = scTabs("cfl", list);
  const live = x.live;
  const kicker = live ? `${x.year} ${live.global ? "Global Draft" : "CFL Draft"} · round ${live.round} · pick ${live.pick}` : `${x.year} CFL Draft · ${x.draft_date}`;
  const head = live && live.on_clock
    ? `<section class="cf-hero on"><div><span class="st-label">${esc(kicker)}</span><h1 class="st-wide">You're on the clock</h1></div><div class="cf-acts">${live.staff_pick ? cfBtn("cf-pick", { id: live.staff_pick.id, name: live.staff_pick.name }, `Draft ${esc(live.staff_pick.name)}`, "primary") : ""}${cfBtn("cf-auto", {}, "Let your staff pick")}</div></section>`
    : `<section class="cf-hero"><div><span class="st-label">${esc(kicker)}</span><h1 class="st-wide">${live ? `Pick ${live.pick} of ${live.picks}` : "The CFL Draft"}</h1><p class="cf-dim">${x.rounds} rounds, ${x.teams} clubs. Your board is your club's file: your scouts' looks, your coaches' film and your own days out.</p></div></section>`;
  const said = CF.said ? `<div class="cf-said">${esc(CF.said)}</div>` : "";
  CF.said = "";
  const body = { Board: cfBoard, Bureau: cfBureau, Pre: cfPre, Room: cfRoom, Global: cfGlobal }[tab.t](x);
  return `<div class="pad cf">${head}${said}${tab.html}${body}${scGoto(GOTO.cfldraft())}</div>`;
};

function cfBoard(x, compact) {
  const live = x.live;
  const there = live && live.next_mine ? `There at pick ${live.next_mine}` : "";
  // The room is the mock-up's: the board, the grade, the Bureau, who's
  // there at your next pick.
  const head = [["#", false], ["Your board, best left", true], ...(compact ? [] : [["Your staff's read", true]]), ["Grade", true], ["Bureau", false]];
  if (there) head.push([there, false]);
  if (live && live.on_clock) head.push(["", false]);
  const rows = x.board.map((r) => `<tr><td><b class="st-cond">${r.rank}</b></td><td class="l">${cfWho(r)}</td>${compact ? "" : `<td class="l">${scNow(r.read, r.id)} ${scRole(r.read)}</td>`}<td class="l">${esc(r.grade)}</td><td>${r.bureau ?? cfDash}</td>${there ? `<td title="Your staff's mock of the picks before yours, ten times">${r.there} in 10</td>` : ""}${live && live.on_clock ? `<td>${cfBtn("cf-pick", { id: r.id, name: r.name }, "Draft", "primary")}</td>` : ""}</tr>`);
  return cfPanel("Your board", "the class still on it, by your club's grade", cfTable(head, rows, "The class comes out on U Sports' opening weekend in late August."));
}

function cfBureau(x) {
  if (!x.bureau.length) return cfPanel("CFL Scouting Bureau", "", '<div class="empty">The Bureau\'s first top 20 comes out in mid September.</div>');
  // One row a man, his place in each edition, the latest first.
  const eds = x.bureau.slice().reverse();
  const men = new Map();
  eds.forEach((b, k) => b.rows.forEach((r) => { if (!men.has(r.id)) men.set(r.id, { r, at: [] }); men.get(r.id).at[k] = r.rank; }));
  const list = [...men.values()].sort((a, b) => (a.at[0] ?? 99) - (b.at[0] ?? 99) || (a.at[1] ?? 99) - (b.at[1] ?? 99) || (a.at[2] ?? 99) - (b.at[2] ?? 99));
  const head = [["Player", true], ...eds.map((b) => [b.edition, false]), ["Your board", false]];
  const rows = list.map((m) => `<tr><td class="l">${cfWho(m.r)}</td>${eds.map((_, k) => `<td>${m.at[k] ? `<b class="st-cond">${m.at[k]}</b>` : cfDash}</td>`).join("")}<td>${m.r.yours ?? cfDash}</td></tr>`);
  return cfPanel("CFL Scouting Bureau", `the top 20 of each edition · ${eds.map((b) => `${esc(b.edition)} ${esc(b.date)}`).join(" · ")}`, cfTable(head, rows, "Nobody listed."));
}

function cfTests(list, label, note) {
  if (!list) return "";
  const head = [["Player", true], ["Ht", false], ["Wt", false], ["40", false], ["Bench", false], ["Vert", false], ["Broad", false], ["Shuttle", false], ["Your board", false]];
  const ft = (n) => `${Math.floor(n / 12)}'${Math.round(n % 12)}"`;
  const rows = list.map((c) => `<tr><td class="l">${cfWho(c)}</td><td>${esc(c.ht)}</td><td>${c.wt}</td><td>${c.forty.toFixed(2)}</td><td>${c.bench}</td><td>${c.vert.toFixed(1)}</td><td>${ft(c.broad)}</td><td>${c.shuttle.toFixed(2)}</td><td>${c.board ?? cfDash}</td></tr>`);
  return cfPanel(label, note, cfTable(head, rows, "Nobody tested."));
}

function cfPre(x) {
  const steps = `<div class="cf-steps">${x.steps.map((s) => `<div class="cf-step ${s.state}"><b>${esc(s.t)}</b><span>${esc(s.d)}</span><small>${esc(s.date)}</small></div>`).join("")}</div>`;
  const tests = x.national
    ? cfTests(x.national, "CFL Combine", `every club gets the same numbers · ${x.national.length} tested`)
    : cfPanel("CFL Combine", "", '<div class="empty">The national combine is in mid March: tests for every club and fifteen interviews for your director.</div>');
  const inv = cfTests(x.invitational, "Invitational combine", "early March · the best nine go on to the national");
  const south = cfTable([["Player", true], ["Your staff's read", true], ["Where he stands", true], ["Your board", false]], x.south.map((r) => `<tr><td class="l">${cfWho(r)}</td><td class="l">${scNow(r.read, r.id)} ${scRole(r.read)}</td><td class="l">${esc(r.what)}</td><td>${r.board ?? cfDash}</td></tr>`), "No Canadians at American colleges in this class.");
  return `${steps}${tests}${inv}${cfPanel("South of the border", "the NFL draft comes first · draft him anyway and you hold his CFL rights", south)}`;
}

function cfPicks(list, empty) {
  return cfTable([["#", false], ["Round", false], ["Club", true], ["Player", true], ["From", true], ["Now", true]], list.map((p) => `<tr><td><b class="st-cond">${p.n}</b></td><td>${p.round}</td><td class="l">${teamLink(p.club)}</td><td class="l">${scName(p.id, p.name)} <small class="cf-sub">${esc(p.pos)}</small></td><td class="l">${esc(p.from)}</td><td class="l cf-dim">${esc(p.now)}</td></tr>`), empty);
}

function cfRoom(x) {
  const live = x.live;
  if (!live) return cfPanel(`The ${x.year} picks`, "", cfPicks(x.national_picks, `The draft is on ${x.draft_date}. When it opens, the clock stops on your picks.`));
  const offers = live.offers.map((o) => `<div class="cf-call"><span class="st-label">${esc(o.club.name)} on the phone</span><p>${esc(o.text)}.</p><div class="cf-acts">${cfBtn("cf-offer", { k: o.k, say: "take" }, "Take it", "primary")}${cfBtn("cf-offer", { k: o.k, say: "more" }, "Ask for more")}${cfBtn("cf-offer", { k: o.k, say: "no" }, "No")}</div></div>`).join("");
  const trades = live.trades.length ? `<div class="cf-dim">${live.trades.map(esc).join("<br>")}</div>` : "";
  const staff = live.on_clock && live.staff_pick ? `<p class="cf-dim">Your staff would take ${scName(live.staff_pick.id, live.staff_pick.name)}, ${esc(live.staff_pick.pos)}, ${esc(live.staff_pick.from)}.</p>` : "";
  const made = live.made.length
    ? `<div class="cf-made">${live.made.map((p) => `<div><b class="st-cond">${p.n}</b><span><b>${esc(p.club.name)}</b> · ${scName(p.id, p.name)} <span class="cf-dim">${esc(p.pos)} · ${esc(p.from)}${p.now ? ` · ${esc(p.now)}` : ""}</span></span></div>`).join("")}</div>`
    : '<div class="empty">None yet.</div>';
  const side = `${offers}${cfPanel("Picks so far", `${live.pick - 1} of ${live.picks}`, made)}${trades}`;
  return `<div class="cf-room"><div class="cf-main">${staff}${cfBoard(x, true)}</div><div class="cf-side">${side}</div></div>`;
}

function cfGlobal(x) {
  const done = x.global_picks.length ? cfPanel("Global Draft picks", "", cfPicks(x.global_picks, "")) : "";
  const pool = cfTable([["Player", true], ["Your staff's read", true]], x.global_class.map((r) => `<tr><td class="l">${cfWho(r)}</td><td class="l">${scNow(r.read, r.id)} ${scRole(r.read)}</td></tr>`), "The global class comes out in February, measured at the invitational combine.");
  return `${done}${cfPanel("The global class", "two rounds the day after the national draft · mostly kickers and punters", pool)}`;
}

document.addEventListener("click", async (ev) => {
  const el = ev.target.closest("[data-act]");
  if (!el || !String(el.dataset.act).startsWith("cf-")) return;
  ev.stopPropagation();
  ev.preventDefault();
  const d = el.dataset;
  try {
    if (d.act === "cf-pick") {
      if (!(await sure(`Draft ${d.name}?`, "The card goes in. It can't be taken back.", "Turn in the card"))) return;
      await api("cfl_pick", { id: Number(d.id) });
    } else if (d.act === "cf-auto") {
      await api("cfl_auto", {});
    } else if (d.act === "cf-offer") {
      const r = await api("cfl_offer", { k: Number(d.k), say: d.say });
      CF.said = r.said || "";
    }
    if (S.status) S.status = await api("status");
    render(0);
  } catch (e) {
    toast(e.message || String(e));
  }
});
