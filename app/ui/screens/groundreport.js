/* Ground's report (lane S8; docs/design/ground.md part N, board 8b, GR77,
 * GR79). The offseason by name, written at the verdict the week before your
 * first game: "Your first spring". Your goals as verdicts with why, your
 * finds with faces and their bloom steps, your ground February to August by
 * area (the headline area called out, lost ground marked), who outworked
 * whom in each area in signs (never their numbers), the miss and its fix, a
 * setback, your AD's word and the paper's.
 *
 *   groundreport   with {year}: that spring's report. Without: the newest.
 *
 * It reads `ground_report` (lane G6). The marks on the ground chart are your
 * own standing against the four levels, never a hidden number, and a rival's
 * side is counted in the sign-in sheets his name is on. Every class is gr- so
 * the app's global names never restyle it. */
"use strict";

I.gdreport = '<path d="M6 3.5h9l4 4V20.5H6z"/><path d="M14.5 3.5V8H19"/><path d="M9 12h7M9 15.5h7"/>';
navAdd({ go: "groundreport", label: "Ground report", icon: "gdreport", sec: "ground", after: "groundperson", seats: ["college"] });
CRUMBS.groundreport = "Ground report";

const GRP = { faces: [] };
const grCap = (s) => (s ? s.charAt(0).toUpperCase() + s.slice(1) : s);
const grDate = (iso, opt) => (iso ? new Date(iso + "T12:00:00").toLocaleDateString("en-US", opt) : "");
const grIni = (name, size = 36, tone = "", txt = "") => `<span class="gr-ini ${tone}" style="--s:${size}px">${esc(txt || (name || "?").split(/[\s,]+/).filter(Boolean).map((x) => x[0]).slice(0, 2).join("").toUpperCase())}</span>`;

// The verdict's mark: a check for met, a ring part-filled for mostly and
// partly, an empty ring with a cross for a miss.
function grMark(v) {
  if (v === "Done") return '<svg width="30" height="30" viewBox="0 0 30 30" aria-hidden="true"><circle cx="15" cy="15" r="13" fill="#e8e6e0"/><path d="M8.5 15.5l4.2 4.2 8.8-9" fill="none" stroke="#07140a" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/></svg>';
  const share = v === "Mostly" ? 0.72 : v === "Partly" ? 0.36 : 0;
  const c = 75.4;
  const ring = `<circle cx="15" cy="15" r="12" fill="none" stroke="rgba(255,255,255,.16)" stroke-width="4"/>${share ? `<circle cx="15" cy="15" r="12" fill="none" stroke="#ffb340" stroke-width="4" stroke-linecap="round" stroke-dasharray="${(c * share).toFixed(1)} ${c}" transform="rotate(-90 15 15)"/>` : '<path d="M10.5 10.5l9 9M19.5 10.5l-9 9" fill="none" stroke="#ffb340" stroke-width="3" stroke-linecap="round"/>'}`;
  return `<svg width="30" height="30" viewBox="0 0 30 30" aria-hidden="true">${ring}</svg>`;
}
const grWord = (v) => (v === "Done" ? "Met" : v);
const grSlot = (g) => (g.dropped ? "A goal you dropped" : { needs: "Your needs", ad: "Your AD's", yours: "Your goal", hunt: "The hunt" }[g.slot] || g.slot_words);

screens.groundreport = async (args) => {
  let x;
  try { x = await api("ground_report", args && args.year ? { year: args.year } : {}); } catch (e) {
    title("Ground report", "");
    return `<div class="pad sl gr"><section class="gr-pb"><div class="gr-lab">Ground report</div><p class="gr-empty">${esc(e.message || String(e))}</p></section></div>`;
  }
  if (!x.ready) {
    title("Ground report", "Written at the verdict");
    return `<div class="pad sl gr"><div class="gr-top"><div><h1>The ground report</h1><small>Not written yet</small></div></div>
      <section class="gr-pb gr-wait"><div class="gr-lab">Your first spring · still going</div>
        <p class="gr-q">The report comes at the verdict, on ${esc(grDate(x.verdict, { weekday: "long", month: "long", day: "numeric" }))}.</p>
        <p class="gr-empty">It reads your goals as verdicts, the kids you found, where your ground moved from February to August, who outworked whom and what you missed. Until then the week, the map and the leads are where the spring is played.</p>
        <div class="gr-two"><button class="st-btn glass" data-go="groundweek">The meeting</button><button class="st-btn glass" data-go="ground">The map</button></div></section></div>`;
  }
  title("Ground report", x.title);
  const goals = x.goals;
  const cnt = (v) => goals.filter((g) => g.verdict === v && !g.dropped).length;
  const tally = [[cnt("Done"), "met"], [cnt("Mostly"), "mostly"], [cnt("Partly"), "partly"], [cnt("Missed") + goals.filter((g) => g.dropped).length, "missed"]].filter((t) => t[0]).map((t) => `${t[0]} ${t[1]}`).join(", ") || "None set";
  const blooming = x.finds.filter((f) => f.steps > 0).length;
  const missCount = (x.miss ? 1 : 0);
  GRP.faces = x.finds.map((f) => f.kid);

  // ---- the top bar ----
  const years = x.years.length > 1
    ? `<label class="gr-pick"><span>Spring</span><select data-act="grspring">${x.years.map((y) => `<option value="${y.year}"${y.year === x.year ? " selected" : ""}>${esc(y.title)}</option>`).join("")}</select></label>`
    : "";
  const top = `<div class="gr-top"><div><h1>${esc(grDate(x.date, { weekday: "long", month: "long", day: "numeric", year: "numeric" }))}</h1><small>The ground report · the week before your first game</small></div><div class="gr-sp"></div>
    <div class="gr-stat"><span>Your goals</span><b>${esc(tally)}</b></div>
    <div class="gr-stat"><span>Finds this year</span><b>${x.finds.length ? `${x.finds.length}${blooming ? `, and ${blooming} blooming` : ""}` : "None"}</b></div>
    <div class="gr-stat"><span>Misses</span><b class="${missCount ? "amb" : ""}">${missCount}</b></div>${years}
    <button class="st-btn primary" data-go="inbox">Read the verdict memo</button></div>`;

  // ---- the title and the verdicts ----
  const m = /^(.*?)(spring)(.*)$/i.exec(x.title);
  const big = m ? `${esc(m[1])}<span>${esc(m[2])}</span>` : esc(x.title);
  const tail = m && m[3].trim() ? `<div class="gr-tail">${esc(m[3].trim())}</div>` : "";
  const met = cnt("Done");
  const sum = `${met === goals.length && met ? "Every goal met." : met ? `${met} of ${goals.length} goals met.` : "No goal met yet."}${x.finds.length ? ` ${x.finds.length} find${x.finds.length === 1 ? "" : "s"}${blooming ? `, ${blooming} already blooming` : ""}.` : ""}${x.miss ? " One miss that stings." : ""}`;
  const tiles = goals.map((g) => {
    const v = g.dropped ? "Missed" : g.verdict;
    const cls = v === "Done" ? "met" : v === "Missed" ? "miss" : "most";
    return `<div class="gr-gt ${cls}"><span class="gr-lab">${esc(grSlot(g))}${g.bold ? " · bold" : ""}</span><div class="gr-rg">${grMark(v)}<span class="gr-w">${esc(g.dropped ? "Dropped" : grWord(v))}</span></div><b class="gr-gw">${esc(g.words)}</b><p>${esc(g.why)}</p></div>`;
  }).join("");
  const hero = `<section class="gr-pb gr-hero"><div class="gr-title"><div class="gr-lab">The ground report · ${esc(grDate(x.from, { month: "long", day: "numeric" }))} to ${esc(grDate(x.date, { month: "long", day: "numeric" }))}</div><div class="gr-t">${big}</div>${tail}<div class="gr-dim">${esc(sum)}</div></div><div class="gr-tiles">${tiles || '<div class="gr-empty">No goals were set this spring.</div>'}</div></section>`;

  // ---- the finds ----
  const stepWords = ["World catches up", "He plays", "A name"];
  const finds = x.finds.length ? x.finds.map((f) => {
    const hot = f.steps > 0;
    const state = /^signed/.test(f.state) ? ["SIGNED", "good"] : /^committed to you/.test(f.state) ? ["COMMITTED", "good"] : /^lost/.test(f.state) ? ["LOST", "amb"] : /^committed to/.test(f.state) ? ["WENT ELSEWHERE", "amb"] : ["STILL OPEN", ""];
    const pill = f.steps >= 3 ? '<span class="gr-pill gold">A NAME</span>' : hot ? `<span class="gr-pill gold">${f.steps >= 2 ? "BLOOMED" : "BLOOMING"}</span>` : "";
    const then = f.stars_then ? `Offered as a ${f.stars_then}-star.` : "Offered unrated.";
    const blooms = f.blooms.length ? ` ${f.blooms[f.blooms.length - 1]}` : f.stars_now > f.stars_then ? ` A ${f.stars_now}-star now.` : "";
    const steps = `<div class="gr-stp">${[0, 1, 2].map((i) => `<i class="${f.steps > i ? "on" : ""}"></i>`).join("")}</div>${hot ? `<div class="gr-stl">${stepWords.map((s) => `<span>${s}</span>`).join("")}</div>` : ""}`;
    return `<div class="gr-fp${hot ? " hot" : ""}"><div class="gr-ph" data-grface="${f.kid}">${grIni(f.name, 96, "sq")}</div><div><div class="gr-fn"><b>${esc(f.name)}</b>${pill}<span class="gr-pill ${state[1]}">${state[0]}</span></div><small class="gr-dim">${esc(f.pos)}</small><p>${then} ${esc(grCap(f.state))}.${esc(blooms)}</p>${steps}</div></div>`;
  }).join("") : '<div class="gr-empty">You made no finds this spring. A find is a kid the world rated low and your man offered first.</div>';
  const men = x.men.length ? `<div class="gr-men"><div class="gr-lab">Who found them</div>${x.men.map((p) => `<div class="gr-man">${grIni(p.name, 26)}<span>${esc(p.name)}</span><b>${p.finds} find${p.finds === 1 ? "" : "s"}${p.blooms ? `, ${p.blooms} bloom${p.blooms === 1 ? "" : "s"}` : ""}${p.big ? `, ${p.big} big` : ""}</b></div>`).join("")}</div>` : "";
  const findsBox = `<section class="gr-pb gr-finds"><div class="gr-row"><div class="gr-lab">Finds</div><span class="gr-dim sm">made, signed and blooming</span></div>${finds}${men}</section>`;

  // ---- your ground, February to August ----
  const head = x.headline != null ? x.areas[x.headline] : null;
  const headBox = head
    ? `<div class="gr-head"><div class="gr-hn"><span class="gr-o">${esc(head.feb)}</span><span class="gr-ar">&rarr;</span><span class="gr-s">${esc(head.aug)}</span></div><div><div class="gr-hh">${esc(head.name)} moved most</div><div class="gr-dim sm">${head.leader ? `${esc(head.leader)} leads it now.` : "Nobody leads it."}</div></div></div>`
    : `<div class="gr-head"><div><div class="gr-hh">Nothing moved far</div><div class="gr-dim sm">No area gained more than a few points between the meeting and now.</div></div></div>`;
  const rows = [...x.areas].sort((a, b) => b.moved - a.moved);
  const pos = (p) => `${Math.max(0, Math.min(100, p))}%`;
  const dumb = rows.length ? rows.map((a) => {
    const lo = Math.min(a.feb_pts, a.pts), hi = Math.max(a.feb_pts, a.pts);
    const hl = head && a.area === head.area ? " hl" : "";
    const none = a.aug === "Stranger" && a.feb === "Stranger" && a.pts < 2;
    return `<div class="${hl.trim()}"><span class="${none ? "gr-dim" : ""}">${a.lost ? '<b class="amb">' : "<b>"}${esc(a.name)}</b></span><div class="gr-tr">${[25, 50, 75].map((g) => `<i class="gr-gl" style="left:${g}%"></i>`).join("")}${hi - lo > 0.5 ? `<span class="gr-seg${a.lost ? " down" : ""}" style="left:${pos(lo)};width:${(Math.min(100, hi) - lo).toFixed(1)}%"></span>` : ""}<span class="gr-a" style="left:${pos(a.feb_pts)}"></span><span class="gr-b${a.lost ? " down" : ""}" style="left:${pos(a.pts)}"></span></div><span class="gr-v">${a.feb === a.aug ? esc(a.aug) : `${esc(a.feb)} &rarr; <b class="${a.lost ? "amb" : ""}">${esc(a.aug)}</b>`}</span></div>`;
  }).join("") : '<div class="gr-empty">Your ground was not read at the February meeting, so there is nothing to set it against.</div>';
  const ground = `<section class="gr-pb gr-ground"><div class="gr-lab">Your ground, February to August</div>${headBox}
    <div class="gr-db" role="img" aria-label="Your standing by area, February and August"><div><span></span><div class="gr-ax"><span>Stranger</span><span>Known</span><span>Trusted</span><span>Home</span></div><span></span></div>${dumb}</div>
    <div class="gr-key"><span><i class="o"></i>February</span><span><i class="f"></i>August</span><span><i class="a"></i>Lost ground</span></div></section>`;

  // ---- who outworked whom ----
  const vs = x.outworked.length ? x.outworked.map((o) => {
    const r = o.rival;
    const tot = Math.max(1, o.yours + (r ? r.n : 0));
    const lead = r && r.n > o.yours + 1 ? "rival" : r && o.yours > r.n + 1 ? "you" : "close";
    const chip = lead === "rival" ? `<span class="gr-chip amb">${esc(r.name)}</span>` : lead === "you" ? '<span class="gr-chip">You</span>' : '<span class="gr-chip">Close</span>';
    const text = r
      ? `Your staff was in ${o.yours} time${o.yours === 1 ? "" : "s"}. ${esc(r.name)}'s name was on ${r.n} of the sheets.`
      : `Your staff was in ${o.yours} time${o.yours === 1 ? "" : "s"}. Nobody else's name was on the sheets.`;
    return `<div class="gr-vs ${lead}"><div class="gr-vh"><b>${esc(o.name)}</b>${chip}</div><div class="gr-vr">${grIni("You", 32, "", "YOU")}<b>${o.yours}</b><div class="gr-split"><i style="flex:${o.yours};background:var(--gr-ivory)"></i><i style="flex:${r ? r.n : 0};background:var(--gr-rival)"></i></div><b>${r ? r.n : 0}</b>${r ? grIni(r.name, 32, "rv") : grIni("?", 32)}</div><p>${text}</p></div>`;
  }).join("") : '<div class="gr-empty">No sign-in sheet in your areas carried a name from your staff or a rival this spring.</div>';
  const outBox = `<section class="gr-pb gr-out"><div class="gr-row"><div class="gr-lab">Who outworked whom</div><span class="gr-dim sm">in signs, never their numbers</span></div>${vs}</section>`;

  // ---- the miss, the setback, the words ----
  const mp = x.miss_parts;
  const missBox = mp
    ? `<section class="gr-miss"><div class="gr-fog"><span>MISS</span></div><div class="gr-mb"><div class="gr-lab amb">The miss · and the fix</div><b class="amb">${esc(mp.head)}</b><p>${esc(mp.body)}</p><div class="gr-ctr"><span>The fix: a man where nobody of yours goes.</span><button class="st-btn glass" data-go="ground">Open the map</button></div></div></section>`
    : `<section class="gr-pb gr-miss none"><div class="gr-lab">The miss</div><p class="gr-empty">Nothing that got away from your own ground this spring.</p></section>`;
  const sp = x.setback_parts;
  const setBox = sp
    ? `<section class="gr-pb gr-set"><div class="gr-lab">A setback</div><b>${esc(sp.head)}</b><p>${esc(sp.body)}</p></section>`
    : `<section class="gr-pb gr-set"><div class="gr-lab">A setback</div><p class="gr-empty">No find of yours was oversold this spring.</p></section>`;
  const ad = x.ad_parts, pp = x.paper_parts;
  const words = `<section class="gr-pb gr-words"><div class="gr-wd">${grIni(ad.who || "AD", 34, "nv")}<div><div class="gr-lab">${esc(ad.who ? `${ad.who}, your AD` : "Your AD's word")}</div><p class="gr-q">${esc(ad.line)}</p>${ad.after ? `<small class="gr-dim">${esc(ad.after)}</small>` : ""}</div></div>
    ${pp.line ? `<div class="gr-wd">${grIni("PR", 34, "tn")}<div><div class="gr-lab">The paper</div><p class="gr-q dim">${esc(pp.line)}</p></div></div>` : ""}</section>`;

  return `<div class="pad sl gr">${top}${hero}<div class="gr-grid">${findsBox}${ground}${outBox}</div><div class="gr-low">${missBox}${setBox}${words}</div></div>`;
};

document.addEventListener("change", (ev) => {
  if (ev.target.dataset && ev.target.dataset.act === "grspring") go("groundreport", { year: Number(ev.target.value) }, false);
});

// After the page draws: each find's head, drawn from his recipe or the photo
// you set, kept in the save's folder for next time (the initials stay when
// drawn heads are off).
(() => {
  const after0 = afterRender;
  afterRender = function () {
    after0();
    for (const id of GRP.faces) {
      const box = document.querySelector(`.gr-ph[data-grface="${id}"]`);
      if (!box || box.querySelector("img")) continue;
      api("face", { id }).then((f) => {
        const b = document.querySelector(`.gr-ph[data-grface="${id}"]`);
        if (!b || b.querySelector("img")) return;
        const mine = f.mine && f.mine.mode !== "team" ? f.mine.data : null;
        const src = mine || f.drawn;
        if (src) { b.innerHTML = `<img src="${src}" alt="${esc(f.name)}">`; return; }
        if (!f.drawn_on || !f.spec || !window.AFMFaces || !AFMFaces.liveHead) return;
        AFMFaces.liveHead(`gr${f.id}-${f.drawn_key}`, JSON.parse(JSON.stringify(f.spec))).then((url) => {
          if (!url) return;
          const c = document.querySelector(`.gr-ph[data-grface="${id}"]`);
          if (c && !c.querySelector("img")) c.innerHTML = `<img src="${url}" alt="">`;
          api("face_drawn_store", { id: f.id, key: f.drawn_key || "", data: url }).catch(() => {});
        });
      }).catch(() => {});
    }
  };
})();
