// AFM screens, update 2 lane 9C: the draft room (board V9), Press Box.
// At an NFL club on draft night the war room becomes the draft room: your
// board in tiers with the chance each man is still there at your pick, his
// public rank, your staff's mock of the picks before yours, his file in
// tabs, the trade calls priced on your GM's chart, your needs and the
// picks so far. At a college or a high school: your players' draft
// outlook. Everything is your staff's read (ranges and words); the chance
// is your staff's mock of the picks, never another club's own board.
// Clicks go through CORE_MORE (data-act "dr…"); drafting, trades and the
// clock reuse the war room's own actions (data-act "sc-…", scouting.js).

const DR = { room: null, reuse: false, file: {}, body: {} };
async function drGet() {
  if (DR.reuse && DR.room) return DR.room;
  DR.room = await api("dr_room");
  return DR.room;
}
function drLocal(el) {
  DR.reuse = true;
  Promise.resolve(refresh(el)).finally(() => { DR.reuse = false; });
}
async function drFile(id) {
  if (!(id in DR.file)) DR.file[id] = await api("scout_file", { id }).catch(() => null);
  return DR.file[id];
}
async function drBody(id) {
  if (!(id in DR.body)) DR.body[id] = await api("sb_body", { id }).catch(() => null);
  return DR.body[id];
}
const drOrd = (n) => { const s = ["th", "st", "nd", "rd"], v = n % 100; return n + (s[(v - 20) % 10] || s[v] || s[0]); };
const drPct = (v) => (v == null ? "–" : Math.round(v) + "%");
const drShort = (t) => esc(String((t && t.name) || "").split(" ").slice(-1)[0]);
const drNum = (v) => Math.round(v).toLocaleString("en-US");

navAdd({ go: "draftoutlook", label: "Draft outlook", icon: "board", sec: "scout", seats: ["college", "hs"] });
CRUMBS.draftoutlook = "Draft outlook";
CRUMBS.warroom = "Draft room";
// The war room is the draft room now (its seats stay live: set only the label).
for (const n of NAV) if (n.go === "warroom") n.label = "Draft room";
GOTO.draftoutlook = () => [["Squad", "roster"], ["Needs map", "needs"], ["My players", "myplayers"]];

// ---------- the draft room ----------
screens.warroom = async () => {
  const w = await drGet();
  SC.live = !!w.live;
  if (S.status) S.status.draft_live = SC.live;
  title("Draft room");
  if (!w.live || !w.dr) {
    if (w.record) return screens.draftrecord({ record: w.record });
    return `<div class="pad">${scHead("The draft room", "It opens on draft night, when the first round starts and your club holds a pick.", scGo("predraft"))}</div>`;
  }
  const dr = w.dr;
  setColors(w.team.colors);
  if (!SC.clock || SC.clock.pick !== w.next) SC.clock = { pick: w.next, left: w.clock_minutes * 60, speed: SC.clock ? SC.clock.speed : 1, paused: false };
  const k = keep("dr", { view: "all", sel: null, tab: "Report" });
  const at = dr.target;
  const cards = dr.cards;
  const left = cards.filter((c) => !c.gone && !c.dnd);
  const sel = cards.find((c) => c.id === k.sel) || cards.find((c) => dr.ours && c.id === dr.ours.id) || left[0] || cards[0];
  // The top bar.
  const otc = w.on_the_clock;
  const stat = (l, v, cls = "") => `<div class="dr-stat ${cls}"><span class="dr-lbl">${l}</span><b>${v}</b></div>`;
  const slots = w.my_slots.filter((s) => !s.used).slice(0, 8).map((s) => `<span class="dr-slot ${s.pick === w.next ? "now" : ""}" title="Round ${s.round}">${s.pick}</span>`).join("");
  const nav = `<div class="dr-nav">${linkTo("draftboard", {}, "Draft board", "st-btn small glass")}${linkTo("predraft", {}, "Pre-draft", "st-btn small glass")}</div>`;
  const head = `<div class="dr-head"><div class="dr-id">${mark(w.team, 44)}<div><span class="dr-lbl">${esc(w.team.name)} · ${w.year} Draft · Round ${w.round}</span><h1>Draft room</h1></div></div>${nav}</div>
    <div class="dr-stats">
      ${stat("On the clock", w.rush ? "Done" : `Pick ${w.next} · ${drShort(otc)}`, w.on_clock ? "gold" : "")}
      ${stat("Time left", `<span id="sc-clock">${scClockText()}</span>`)}
      ${stat("Our pick", at ? at : "–", "gold")}
      ${stat("Picks away", at ? dr.away : "–")}
      <div class="dr-stat dr-ctl"><span class="dr-lbl">Our picks</span><span class="dr-slots">${slots || '<span class="dr-dim">None left</span>'}</span></div>
      <div class="dr-stat dr-ctl"><span class="dr-lbl">Clock</span><span class="dr-seg"><button data-act="sc-clock" data-v="pause" aria-pressed="${!!SC.clock.paused}">${SC.clock.paused ? "Run" : "Pause"}</button><button data-act="sc-clock" data-v="10" aria-pressed="${SC.clock.speed === 10}">×10</button><button data-act="sc-clock" data-v="60" aria-pressed="${SC.clock.speed === 60}">×60</button></span></div>
    </div>`;
  // What you can do now.
  const acts = w.rush
    ? `<div class="dr-acts dr-done"><b>Every pick is in.</b><span class="dr-dim">${w.picks_made} players taken. The phones start now: the undrafted rush.</span><button class="st-btn primary" data-act="sc-rush">Open the undrafted rush</button></div>`
    : w.on_clock
      ? `<div class="dr-acts"><span class="dr-lbl gold">You're on the clock</span>${sel && !sel.gone ? `<button class="st-btn primary" data-act="sc-draft" data-id="${sel.id}" data-name="${esc(sel.name)}">Draft ${esc(sel.name)}</button>` : ""}<button class="st-btn glass" data-act="sc-auto">Let the staff pick${w.staff_pick ? ": " + esc(w.staff_pick.name) : ""}</button><span id="sc-timeup" hidden><button class="st-btn glass" data-act="sc-timeout">Let the clock run out</button></span><span class="dr-final"><span class="dr-lbl">Final say</span><b>You, head coach</b></span></div>`
      : `<div class="dr-acts"><button class="st-btn primary" data-act="sc-go" data-until="yours">Sim to my pick</button><button class="st-btn glass" data-act="sc-go" data-until="next">Next pick</button><button class="st-btn glass" data-act="sc-go" data-until="round">Finish the round</button><span class="dr-dim">${w.picks_made} of ${w.slots} picks made</span></div>`;
  // The board: views, the mock strip, the tiers.
  const V = [["all", "Everyone"], ["needs", "Needs only"], ["hide", "Hide drafted"], ["likely", at ? `Likely at ${at}` : "Likely there"]];
  const views = `<span class="dr-seg">${V.map(([v, n]) => `<button data-act="drview" data-v="${v}" aria-pressed="${k.view === v}">${n}</button>`).join("")}</span>`;
  const gn = dr.gone_note;
  const note = at && dr.away ? `~${gn.likely_go} of our ${gn.left} left likely go; ~${gn.off_board} picks go off our board` : at ? "You're on the clock: everyone left is yours to take" : "No more picks this draft";
  const strip = dr.strip.map((s) => `<div class="dr-sc ${s.on_clock ? "otc" : ""} ${s.guess && s.guess.id === sel.id ? "on" : ""}">
      <button class="dr-sc-main" ${s.guess ? `data-act="drsel" data-id="${s.guess.id}"` : "disabled"} aria-label="Pick ${s.pick}">
        <span class="dr-sc-h"><b>${s.pick} · ${drShort(s.team)}</b></span>
        <span class="dr-sc-n">${s.on_clock ? '<span class="dr-tag">on the clock</span> · ' : ""}${s.needs.length ? "needs " + s.needs.map(esc).join(" · ") : "no clear need"}</span>
        <span class="dr-sc-p">${s.guess ? `<b>${esc(s.guess.name.split(" ").slice(-1)[0])}</b><small>${esc(s.guess.pos)}</small><i>${s.guess.share}%</i>` : '<span class="dr-dim">–</span>'}</span>
      </button>
      ${w.on_clock || s.on_clock ? "" : `<button class="dr-price" data-act="sc-quote" data-slot="${s.slot}" title="Ask what moving up to ${s.pick} costs">Price</button>`}
    </div>`).join("");
  const ours = dr.ours
    ? `<button class="dr-sc dr-sc-ours ${dr.ours.id === sel.id ? "on" : ""}" data-act="drsel" data-id="${dr.ours.id}"><span class="dr-sc-h"><b class="gold">${at} · ${drShort(w.team)}</b><span class="dr-tag gold">our pick</span></span><span class="dr-sc-n">${dr.ours.fit ? "best need-fit left" : "best player likely there"}</span><span class="dr-sc-p"><b>${esc(dr.ours.name.split(" ").slice(-1)[0])}</b><small>${esc(dr.ours.pos)}</small><i>${drPct(dr.ours.there)}</i></span></button>`
    : "";
  const stripHtml = at
    ? `<div class="dr-strip-h"><span class="dr-lbl">${dr.strip.length ? `Before our pick · our staff's mock of picks ${dr.strip[0].pick} to ${dr.strip[dr.strip.length - 1].pick}${dr.strip_more ? ` (+${dr.strip_more} more)` : ""}` : "Our pick"}</span><span class="dr-dim">${esc(note)}</span></div><div class="dr-strip">${strip}${ours}</div>`
    : `<div class="dr-strip-h"><span class="dr-dim">${esc(note)}</span></div>`;
  const tierHtml = dr.tiers.map((t) => {
    const cs = cards.filter((c) => c.tier === t.tier && !(k.view === "hide" && c.gone));
    if (!cs.length) return "";
    return `<section class="dr-tier"><div class="dr-tier-h"><b>${esc(t.name)}</b><span>${esc(t.desc)}</span><span class="dr-dim">${t.left} left</span>${at ? `<span class="dr-dim">~${t.exp} at ${at}</span>` : ""}</div>
      <div class="dr-cards">${cs.map((c) => drCard(c, k.view, at, sel)).join("")}</div></section>`;
  }).join("");
  const legend = `<div class="dr-legend"><span><i class="dr-fit"></i> top-three need</span><span><b class="dr-flag">V</b> top-30 visit</span><span><b class="dr-flag warn">M</b> medical flag</span><span><b class="dr-flag warn">C</b> character flag</span><span><b class="gold">↑</b> out on a limb: 12+ spots above the public</span><span>bar: chance he's there${at ? " at " + at : ""}, from ${dr.runs} runs of our staff's mock</span></div>`;
  const board = `<div class="dr-card dr-board"><div class="dr-board-h"><div><span class="dr-lbl">Our board</span><span class="dr-dim">${cards.length} players in ${dr.tiers.length} tiers · ${cards.filter((c) => c.gone).length} gone</span></div>${views}</div>${stripHtml}${tierHtml || '<p class="dr-dim">Nobody on your board.</p>'}${legend}</div>`;
  // The side: his file, the calls, the needs, the picks.
  const side = `<div class="dr-side">${await drFileCard(sel, dr, w, k.tab)}${drCalls(w, dr)}${drNeeds(dr)}${drPicks(dr)}</div>`;
  const reveal = SC.reveal && w.revealed && SC.reveal === `${w.revealed.pick}-${w.revealed.id}`
    ? `<div class="st-moment fixed sc-reveal" data-act="sc-unreveal"><div class="st-reveal"><div class="st-kicker">The pick is in</div><div class="st-outline">${w.revealed.pick}</div><div class="st-reveal-sub">${esc(w.revealed.pos)} · ${esc(w.revealed.school)}</div><h2>${esc(w.revealed.name)}</h2>${w.revealed.take ? `<div class="st-verdict"><span><b style="font-size:13px;font-stretch:100%;color:#fff">${esc(w.revealed.take.who)}, ${esc(w.revealed.take.outlet)}</b><br>“${esc(w.revealed.take.says)}”</span></div>` : ""}<button class="st-btn primary" data-act="sc-unreveal">Back to the board</button></div></div>`
    : "";
  return `<div class="pad dr">${head}${acts}<div class="dr-grid">${board}${side}</div></div>${reveal}`;
};

function drCard(c, view, at, sel) {
  const gone = !!c.gone;
  const there = c.there;
  const dim = gone ? "gone" : (view === "needs" && !c.fit) || (view === "likely" && there != null && there < 50) ? "dim" : "";
  const limb = !gone && c.limb > 0;
  const flags = c.flags.map((f) => `<b class="dr-flag ${f === "V" ? "" : "warn"}" title="${f === "V" ? "Top-30 visit" : f === "M" ? "Medical flag" : "Character flag"}">${f}</b>`).join("");
  const foot = gone
    ? `<span class="dr-cf"><span>Pick ${c.gone.pick} · ${drShort(c.gone.team)}</span></span>`
    : at
      ? `<span class="dr-cf"><span>at ${at}</span><span class="dr-meter"><i class="${there >= 50 ? "hi" : ""}" style="width:${there ?? 100}%"></i></span><b class="${there >= 50 ? "" : "lo"}">${drPct(there ?? 100)}</b></span>`
      : `<span class="dr-cf"><span>${c.dnd ? "Do not draft" : "Available"}</span></span>`;
  return `<button class="dr-c ${dim} ${sel && c.id === sel.id ? "on" : ""} ${c.dnd ? "dnd" : ""}" data-act="drsel" data-id="${c.id}" aria-label="${esc(c.name)}">
    <span class="dr-ct"><span><b class="dr-rk">${c.rank}</b><span class="dr-pub ${limb ? "gold" : ""}">${c.public ? "pub " + c.public : "unranked"}${limb ? " ↑" : ""}</span></span><span class="dr-pos">${c.fit ? '<i class="dr-fit"></i>' : ""}${esc(c.pos)}</span></span>
    <span class="dr-cn">${esc(c.name)}</span>
    <span class="dr-cs">${esc(c.school)}</span>
    <span class="dr-cg"><span>${esc(c.grade_words)}</span><span>${flags}</span></span>
    ${foot}
  </button>`;
}

async function drFileCard(c, dr, w, tab) {
  if (!c) return `<div class="dr-card"><p class="dr-dim">Pick a player on the board.</p></div>`;
  const [f, b] = await Promise.all([drFile(c.id), drBody(c.id)]);
  const at = dr.target;
  const need = dr.needs.find((n) => n.pos === c.pos);
  const fit = need && need.top ? "Top need" : need && need.v >= 40 ? "Need" : "Depth";
  const value = c.gone ? "Gone" : !at ? "–" : c.rank <= at - 5 ? `Steal at ${at}` : c.rank <= at + 4 ? `Fair at ${at}` : `Reach at ${at}`;
  const limbW = c.public && c.limb > 0 ? ", out on a limb" : c.public && c.limb < 0 ? ", we're far lower" : "";
  const tabs = ["Report", "Testing", "Medical", "The room"];
  const t = tabs.includes(tab) ? tab : "Report";
  const s = f && f.summary;
  let body = "";
  if (t === "Report") {
    const note = s ? [s.role ? `Your staff sees ${s.role}.` : "", s.strengths && s.strengths.length ? `Best at ${s.strengths.slice(0, 3).join(", ").toLowerCase()}.` : "", s.weaknesses && s.weaknesses.length ? `Watch ${s.weaknesses.slice(0, 2).join(", ").toLowerCase()}.` : ""].filter(Boolean).join(" ") : "Your staff hasn't seen him: only his listed size and the public's word.";
    body = `<div class="dr-cells"><div><span class="dr-lbl">Our grade</span><b>${esc(c.grade_words)}</b></div><div><span class="dr-lbl">Fit</span><b class="${fit === "Top need" ? "gold" : ""}">${fit}</b></div><div><span class="dr-lbl">There at ${at || "–"}</span><b class="${c.there != null && c.there < 50 ? "dr-dim" : ""}">${c.gone ? "–" : drPct(c.there ?? (at ? 100 : null))}</b></div></div>
      ${s ? `<div class="dr-kv"><span>His level now</span><span>${range(s.now, "your staff", { p: c.id, k: "ovr" })}</span></div>` : ""}
      <div class="dr-kv"><span>Value</span><b>${value}</b></div>
      <div class="dr-kv"><span>Public rank</span><b class="${c.limb > 0 ? "gold" : ""}">${c.public ? drOrd(c.public) + limbW : "Unranked"}</b></div>
      <div class="dr-kv"><span>Size</span><b>${f ? `${esc(f.ht)} · ${f.wt}` : "–"}</b></div>
      <div class="dr-kv"><span>Status</span><b>${c.gone ? `Drafted ${c.gone.pick} by ${esc(c.gone.team.name)}` : c.dnd ? "Do not draft" : "Available"}</b></div>
      ${s && s.floor ? `<div class="dr-kv"><span>Floor and ceiling</span><b>${esc(s.floor)}</b></div>` : ""}
      <p class="dr-note">${esc(note)}</p>`;
  } else if (t === "Testing") {
    const ev = (re) => (f && f.tests ? f.tests.find((x) => re.test(x.where)) : null);
    const cmb = ev(/combine/i), pro = ev(/pro day/i);
    const fx = { forty: (v) => v.toFixed(2), cone: (v) => v.toFixed(2), shuttle: (v) => v.toFixed(2), vertical: (v) => v.toFixed(1) + '"', broad: (v) => `${Math.floor(v / 12)}'${Math.round(v % 12)}"`, reps: (v) => String(Math.round(v)) };
    const tk = { forty: "forty", cone: "cone", shuttle: "shuttle", vertical: "vertical", broad: "broad", reps: "bench" };
    const rows = (b && b.rows ? b.rows : []).filter((r) => r.key !== "wing");
    const cell = (e, r) => {
      const v = e && tk[r.key] ? e[tk[r.key]] : null;
      return typeof v === "number" && v > 0 && fx[r.key] ? fx[r.key](v) : "–";
    };
    const lines = rows.map((r) => {
      // Among the drafted men at his position, else the real combine.
      const p = r.pct ? (r.pct[1] ?? r.pct[2]) : null;
      return `<div class="dr-tr"><span>${esc(r.label)}</span><span>${tk[r.key] ? cell(cmb, r) : r.text ? esc(r.text) : "–"}</span><span class="${pro ? "" : "dr-dim"}">${tk[r.key] ? cell(pro, r) : "–"}</span><span class="dr-meter"><i class="${p != null && p < 20 ? "lo" : "hi"}" style="width:${p ?? 0}%"></i></span><b class="${p != null && p < 20 ? "warn" : ""}">${p != null ? drOrd(p) : ""}</b></div>`;
    }).join("");
    body = rows.length
      ? `<div class="dr-tr dr-trh"><span>Test</span><span>Combine</span><span>Pro day</span><span>Percentile</span><span></span></div>${lines}<p class="dr-note">Measured and timed where your staff has it; a listing is self-reported. Percentile among every drafted ${esc(c.pos)} the world remembers, or the real combine since 2000 where the world has too few.</p>`
      : `<p class="dr-dim">Nothing measured yet.</p>`;
  } else if (t === "Medical") {
    const m = f && f.medical;
    const worries = f && f.character ? f.character.filter((x) => x.word !== "Not read yet" && x.good === false) : [];
    const lists = f && f.board && f.board.lists ? f.board.lists : [];
    body = `<div class="dr-med"><div class="dr-kv"><span>Medical</span><b class="${m && m.grade >= 1 ? "warn" : ""}">${m ? esc(m.words) : "No read"}</b></div><p class="dr-note">${m ? (m.notes && m.notes.length ? m.notes.map(esc).join(" ") : "Clean: nothing on file.") + (m.examined ? "" : " Records only: your doctors haven't examined him.") : "Your doctors have nothing on him."}</p></div>
      <div class="dr-med"><div class="dr-kv"><span>Character</span><b class="${c.character === "Concern" ? "warn" : ""}">${esc(c.character || "Not read yet")}</b></div>${worries.length ? `<span class="dr-dim">What your staff noticed</span>` + worries.slice(0, 3).map((x) => `<p class="dr-note"><b>${esc(x.name)}: ${esc(x.word)}.</b> ${esc(x.note || "")}</p>`).join("") : ""}</div>
      <div class="dr-kv"><span>Interviews and visits</span><b>${lists.length ? lists.map(esc).join(" · ") : "None on your lists"}</b></div>
      ${f && f.choices && f.choices.length ? `<div class="dr-kv"><span>His choices</span><b>${f.choices.map(esc).join(", ")}</b></div>` : ""}`;
  } else {
    const v = f && f.voices ? f.voices : [];
    body = v.length
      ? `<span class="dr-lbl">Everyone on your staff with three looks or more</span>${v.map((x) => `<div class="dr-vote"><div><b>${esc(x.who)}</b><span class="dr-dim"> · ${esc(x.job)} · ${x.looks} looks</span>${x.tier ? `<b class="dr-vt">T${x.tier}</b>` : ""}</div><p>“${rich(x.says)}”</p></div>`).join("")}<div class="dr-final"><span class="dr-lbl">Final say</span><b>You, head coach</b></div>`
      : `<p class="dr-dim">Nobody has three looks at him yet, so the room has no voices on him.</p>`;
  }
  return `<div class="dr-card dr-file"><div class="dr-file-h"><span class="dr-lbl">${esc(c.pos)} · ${esc(c.school)}</span><span class="dr-dim">board ${c.rank}</span></div>
    <span class="dr-fn">${playerLink(c.id, c.name)}</span>
    <span class="dr-seg dr-tabs">${tabs.map((n) => `<button data-act="drtab" data-v="${n}" aria-pressed="${n === t}">${n}</button>`).join("")}</span>
    <div class="dr-fb">${body}</div>
    <div class="dr-file-a">${linkTo("scout", { id: c.id }, "Full scouting file", "st-btn small glass")}${w.on_clock && !c.gone ? `<button class="st-btn small primary" data-act="sc-draft" data-id="${c.id}" data-name="${esc(c.name)}">Draft him</button>` : ""}</div></div>`;
}

function drCalls(w, dr) {
  const calls = dr.calls.map((c) => {
    const o = w.offers[c.k];
    const tot = c.give + c.get || 1;
    return `<div class="dr-call"><div class="dr-call-h">${mark(c.from, 26)}<b>${esc(c.from.name)}, from ${c.their_pick}</b><span class="${c.get >= c.give ? "gold" : "dr-dim"}">${drNum(c.give)} for ${drNum(c.get)}</span></div>
      <span class="dr-dim">Our ${w.next} for ${o ? o.gets.map(esc).join(" and ") : "their pick"}</span>
      <span class="dr-split"><i style="width:${((c.give / tot) * 100).toFixed(1)}%"></i><i class="get" style="width:${((c.get / tot) * 100).toFixed(1)}%"></i></span>
      <p class="dr-note"><b>${esc(c.line.who)}:</b> ${esc(c.line.says)}</p>
      ${o && o.after ? `<span class="dr-dim">They're after ${playerLink(o.after.id, o.after.name)}, ${esc(o.after.pos)}</span>` : ""}
      <button class="st-btn small primary" data-act="sc-accept" data-k="${c.k}">Accept the trade</button></div>`;
  }).join("");
  const q = SC.quote && SC.quote.next === w.next
    ? SC.quote.offer
      ? `<div class="dr-call"><div class="dr-call-h">${mark(SC.quote.offer.from, 26)}<b>Up to pick ${SC.quote.slot + 1}</b></div><span class="dr-dim">${esc(SC.quote.offer.from.name)} would deal: ${SC.quote.offer.gets.map(esc).join(", ")}</span><button class="st-btn small primary" data-act="sc-tradeup" data-slot="${SC.quote.slot}">Make the trade</button></div>`
      : `<p class="dr-dim">Nobody will move pick ${SC.quote.slot + 1} for what you have.</p>`
    : "";
  return `<div class="dr-card"><span class="dr-lbl">Trade calls <small>on your GM's ${esc(dr.chart)} chart</small></span>${calls || `<p class="dr-dim">${w.on_clock ? "Nobody's calling about your pick yet." : "Calls come when you're on the clock. Price a move up from the strip."}</p>`}${q}</div>`;
}

function drNeeds(dr) {
  if (!dr.needs.length) return "";
  return `<div class="dr-card"><span class="dr-lbl">Our needs <small>your staff's needs map, now and next fall</small></span><div class="dr-needs">${dr.needs.map((n) => `<div class="dr-need"><span><b class="${n.top ? "gold" : ""}">${esc(n.pos)}</b><span class="dr-dim">${n.v}</span></span><span class="dr-meter"><i class="${n.top ? "gold" : "hi"}" style="width:${n.v}%"></i></span></div>`).join("")}</div>${linkTo("needs", {}, "Needs map", "dr-more")}</div>`;
}

function drPicks(dr) {
  return `<div class="dr-card"><span class="dr-lbl">Picks so far <small>B = our board · amber: taken 4+ spots before our board had him</small></span>${dr.picks.length ? `<div class="dr-picks">${dr.picks.map((p) => `<div class="dr-pk ${p.mine ? "mine" : ""}"><span class="dr-dim">${p.pick}</span><span>${drShort(p.team)}</span><span>${playerLink(p.id, p.name)} <small class="dr-dim">${esc(p.pos)}</small></span><b class="${p.early ? "warn" : p.board && p.board <= 3 ? "hi" : ""}">${p.board ? "B" + p.board : "–"}</b></div>`).join("")}</div>` : '<p class="dr-dim">No picks yet.</p>'}</div>`;
}

// ---------- the draft outlook (college and high school seats) ----------
screens.draftoutlook = async () => {
  const x = await api("dr_outlook");
  setColors(x.team.colors);
  title("Draft outlook", `${x.draft_year} NFL Draft`);
  const hs = x.seat === "hs";
  const groups = [[0, `The ${x.last} draft`], [1, `The ${x.draft_year} class`], [2, "Back for another year"], [3, `Eligible in ${x.draft_year + 1}`]];
  const row = (r) => `<div class="dr-or">
      <span class="dr-on">${playerLink(r.id, r.name)}<small class="dr-dim">${esc(r.pos)}${hs && r.school ? " · " + esc(r.school) : ""}${r.class ? " · " + esc(r.class) : ""}</small></span>
      <span><b class="dr-ow ${/Round 1/.test(r.outlook) ? "gold" : /Free|Return|early/.test(r.outlook) ? "dim" : ""}">${esc(r.outlook)}</b><small class="dr-dim" title="${esc(r.status)}">${esc(r.status)}</small></span>
      <span>${r.draft ? `${mark(r.draft.team, 22)} ${teamLink(r.draft.team)}` : r.advisory ? `<b>${esc(r.advisory)}</b><small class="dr-dim">advisory committee</small>` : '<span class="dr-dim">–</span>'}</span>
      <span>${r.scouts ? `<b>${esc(r.scouts)}</b><small class="dr-dim">NFL scouts' word</small>` : '<span class="dr-dim">–</span>'}</span>
      <span>${r.public ? `<b>${drOrd(r.public)}</b><small class="dr-dim">public${r.mock ? ` · mock ${r.mock}` : ""}</small>` : r.mock ? `<b>Mock ${r.mock}</b><small class="dr-dim">best mock</small>` : '<span class="dr-dim">–</span>'}</span>
      <span>${range(r.read, hs ? "the public read" : "your staff", { p: r.id, k: "ovr" })}${r.staff ? `<small class="dr-dim">${esc(r.staff)}</small>` : ""}</span>
    </div>`;
  const secs = groups.map(([g, name]) => {
    const rs = x.rows.filter((r) => r.group === g);
    if (!rs.length) return "";
    return `<section class="dr-card dr-osec"><span class="dr-lbl">${esc(name)} <small>${rs.length}</small></span>
      <div class="dr-or dr-orh"><span>Player</span><span>Outlook</span><span>${g === 0 ? "Club" : "Advisory"}</span><span>Scouts</span><span>Public</span><span>${hs ? "Public read" : "Your staff's read"}</span></div>${rs.map(row).join("")}</section>`;
  }).join("");
  const c = x.counts;
  const drafted = x.rows.filter((r) => r.group === 0 && r.draft).length;
  const cells = `<div class="dr-cells four"><div><span class="dr-lbl">Drafted in ${x.last}</span><b>${drafted}</b>${c.drafted - drafted ? `<small class="dr-dim">${c.drafted - drafted} signed undrafted</small>` : ""}</div><div><span class="dr-lbl">In the ${x.draft_year} class</span><b>${c.class}</b></div><div><span class="dr-lbl">Back for another year</span><b>${c.back}</b></div><div><span class="dr-lbl">Eligible in ${x.draft_year + 1}</span><b>${c.next}</b></div></div>`;
  const sub = hs
    ? `Your former players in college and the pros. Declare by ${esc(x.deadline)}; the draft opens ${esc(x.draft_day)}.`
    : `Your players three years out of high school or more. Declare by ${esc(x.deadline)}; the draft opens ${esc(x.draft_day)}.${x.bars ? ` Last year it took about ${x.bars[0]} to go in rounds 1-2 and ${x.bars[1]} in rounds 3-4.` : ""}`;
  return `<div class="pad dr"><div class="dr-head"><div class="dr-id">${mark(x.team, 44)}<div><span class="dr-lbl">${esc(x.team.name)} · ${x.draft_year} NFL Draft</span><h1>Draft outlook</h1></div></div></div>
    <p class="dr-sub">${sub}</p>${cells}
    ${secs || `<div class="dr-card"><p class="dr-dim">${hs ? "None of your former players is in the draft picture yet." : "None of your players is three years out of high school yet."}</p></div>`}</div>`;
};

Object.assign(CORE_MORE, {
  drview: (el) => { keep("dr", { view: "all", sel: null, tab: "Report" }).view = el.dataset.v; drLocal(el); },
  drsel: (el) => { keep("dr", { view: "all", sel: null, tab: "Report" }).sel = Number(el.dataset.id); drLocal(el); },
  drtab: (el) => { keep("dr", { view: "all", sel: null, tab: "Report" }).tab = el.dataset.v; drLocal(el); },
});
