/* Your desk, the college assistant's (lane S7; docs/design/ground.md part K,
 * board 9, GR69, GR78, GR79). Your area and your position group are yours and
 * the rest is your head coach's. Your list this week (up to two kids, your
 * case, the film's word and his verdict, with "Push back in his office" and
 * "Let it ride" when he says wait), his Monday plan for you, his memory of you
 * in his words, your goals with how far along each is and his August verdict,
 * your position group's room, and what brings you offers.
 *
 *   grounddesk   your desk. A head coach has none: it points at Your men.
 *
 * It reads `ground_desk` and writes with `ground_push`, `ground_push_back`,
 * `ground_let_ride` and `ground_desk_goal` (lane G10); the kids you can bring
 * come from your leads (`ground_leads`). His trust in your eye is words, never
 * a number. Every class is gk- so the app's global names never restyle it. */
"use strict";

I.gddesk = '<rect x="3.5" y="8" width="17" height="10" rx="2"/><path d="M8 8V6.5A1.5 1.5 0 0 1 9.500 5h5A1.5 1.5 0 0 1 16 6.500V8"/><path d="M3.500 12.500h17"/>';
navAdd({ go: "grounddesk", label: "Your desk", icon: "gddesk", sec: "top", after: "home", seats: ["college"], show: () => !!(S.status && S.status.mine) && typeof foIsHead === "function" && !foIsHead() });
CRUMBS.grounddesk = "Your desk";

const GK = { faces: [], said: "" };
const gkCap = (s) => (s ? s.charAt(0).toUpperCase() + s.slice(1) : s);
const gkDate = (iso, opt) => (iso ? new Date(iso + "T12:00:00").toLocaleDateString("en-US", opt) : "");
const gkIni = (name, size = 30, tone = "") => `<span class="gk-ini ${tone}" style="--s:${size}px">${esc((name || "?").split(/[\s,]+/).filter(Boolean).map((x) => x[0]).slice(0, 2).join("").toUpperCase())}</span>`;
function gkFace(f, name, size) {
  if (!f) return gkIni(name, size);
  GK.faces.push(f);
  return `<span class="gk-face" style="--s:${size}px" data-gkface="${f.id}">${f.drawn ? `<img src="${f.drawn}" alt="${esc(name)}">` : gkIni(name, size)}</span>`;
}
const GK_VERDICT = {
  waiting: ["Waiting on the film", "wait"],
  signed: ["Signed. Offer him.", "yes"],
  wait: ["Wait", "no"],
  gone: ["He's gone elsewhere", "no"],
};
const GK_OWN = ["Trusted with more head coaches", "Known with the biggest schools", "A find"];

screens.grounddesk = async () => {
  let x;
  try { x = await api("ground_desk", {}); } catch (e) {
    title("Your desk", "");
    return `<div class="pad sl gk"><section class="gk-pb"><div class="gk-lab">Your desk</div><p class="gk-empty">${esc(e.message || String(e))}</p>${typeof foIsHead === "function" && foIsHead() ? '<button class="st-btn glass" data-go="groundstaff">Your men</button>' : ""}</section></div>`;
  }
  GK.faces = [];
  title("Your desk", `Under ${x.boss}`);
  const boss = x.boss.split(" ").slice(-1)[0];
  const said = GK.said ? `<div class="gk-said"><b>${esc(GK.said)}</b></div>` : "";
  GK.said = "";

  const top = `<div class="gk-top"><div><h1>${esc(gkDate(x.today, { weekday: "long", month: "long", day: "numeric", year: "numeric" }))}</h1><small>Your seat: ${esc(x.group ? `${x.group} coach` : "assistant")} under ${esc(x.boss)}${x.areas.length ? ` · ${esc(x.areas.join(" and "))}` : ""}</small></div><div class="gk-sp"></div>
    <div class="gk-stat"><span>Your list</span><b>${2 - x.left_this_week} of 2 this week</b></div>
    <div class="gk-stat"><span>His trust in your eye</span><b>${esc(gkCap(x.trust))}</b></div>
    <div class="gk-stat"><span>Road days</span><b>${x.plan ? (x.plan.days ? `${x.plan.days} this week` : "None, a quiet week") : "No plan yet"}</b></div>
    <button class="st-btn primary" data-go="groundweek">Plan your week</button></div>`;

  // ---- your list this week ----
  const rows = x.pushes.length ? x.pushes.map((p) => {
    const [vw, vc] = GK_VERDICT[p.verdict] || [p.verdict, ""];
    const what = p.verdict === "wait" && p.pushed_back != null
      ? (p.pushed_back ? "You pushed back in his office." : "He held.")
      : "";
    const act = p.can_push_back
      ? `<div class="gk-acts"><button class="st-btn primary" data-act="gkback" data-id="${p.id}">Push back in his office</button><button class="st-btn glass" data-act="gkride" data-id="${p.id}">Let it ride</button></div>`
      : "";
    const head = p.verdict === "wait" || p.verdict === "signed" ? (p.reason ? gkCap(p.reason.replace(/\.$/, "")) : vw) : vw;
    const answer = `<div class="gk-ans ${vc}"><div class="gk-live">Coach ${esc(boss)} · ${esc(gkDate(p.date, { month: "short", day: "numeric" }))}</div><b>${esc(head)}</b>${p.verdict === "waiting" ? "<p>He decides when the film is in.</p>" : head !== vw || what ? `<p>${head !== vw ? esc(vw) : ""} ${esc(what)}</p>` : ""}${act}</div>`;
    return `<div class="gk-push"><div class="gk-kid"><div class="gk-kh">${gkIni(p.name, 52, "sq")}<div><b>${esc(p.name)}</b>${p.find ? ' <span class="gk-chip gold">Your find</span>' : ""}<small>${esc(p.pos)} · ${esc(p.school)} · ${p.stars ? `${p.stars}-star` : "Unrated"}</small></div></div>
      <p class="gk-case"><b>Your case:</b> “${esc(p.case)}”</p>${p.film_word ? `<p class="gk-case gk-film"><b>The film:</b> “${esc(p.film_word)}”${p.film ? ` <small class="gk-dim">${esc(p.film)}</small>` : ""}</p>` : p.film ? `<p class="gk-case gk-film gk-dim">${esc(p.film)} is watching the film.</p>` : ""}</div>${answer}</div>`;
  }).join("") : '<p class="gk-empty">You haven\'t brought him a kid yet. Take one from your leads, in your area or your group, and say why he should be offered.</p>';
  const list = `<section class="gk-pb gk-list"><div class="gk-row"><h2>Your list this week</h2><span class="gk-dim sm">you bring up to two, the film's word, ${esc(boss)} decides</span></div>${rows}
    <p class="gk-note">A kid ${esc(boss)} signs is your find, when the world still rates him low. One he says wait on isn't, since your program never offered. He and the people around him still remember who pushed.</p>
    <div class="gk-bring" id="gkbring"></div></section>`;

  // ---- his Monday plan ----
  const plan = x.plan
    ? `<section class="gk-pb"><div class="gk-row"><h2>His Monday plan for you</h2><span class="gk-dim sm">${esc(gkDate(x.plan.date, { month: "short", day: "numeric" }))}</span></div>
        <div class="gk-pl"><b>${x.plan.days ? `${x.plan.days} road day${x.plan.days === 1 ? "" : "s"}` : "No road days this week"}</b><span>${esc(x.plan.area ? `In ${x.plan.area}.` : "")} ${x.plan.days ? "You plan your own route inside his days." : "A quiet week: campus and the phone."}</span></div>
        <button class="st-btn glass" data-go="groundweek">Plan your route</button></section>`
    : `<section class="gk-pb"><h2>His Monday plan for you</h2><p class="gk-empty">He hasn't given you a plan yet. It comes with his Monday meeting.</p></section>`;

  // ---- his memory of you ----
  const mem = x.memory.length ? x.memory.slice(0, 5).map((m) => `<div class="gk-m${m.good ? "" : " bad"}"><span>${esc(gkDate(m.date, { month: "short", day: "numeric" }))}</span><p>“${esc(m.words)}”</p></div>`).join("") : '<p class="gk-empty">He has not formed a view of you yet. One call he judges and he will.</p>';
  const rec = x.pushes.filter((p) => p.verdict === "signed").length;
  const args = x.pushes.filter((p) => p.pushed_back != null).length;
  const you = `<section class="gk-pb gk-boss"><div class="gk-bh">${gkFace(x.face, x.boss, 52)}<div><span class="gk-lab">Your head coach</span><b>${esc(x.boss)}</b><small>${esc(gkCap(x.trust))}. He knows you: ${esc(x.memory_level)}.</small></div></div>
    <div class="gk-lab">His memory of you · in his words</div>${mem}
    <div class="gk-tl"><div><span>Calls judged</span><b>${x.pushes.filter((p) => p.verdict === "signed" || p.verdict === "wait").length || "None yet"}</b></div><div><span>Arguments</span><b>${args || "None yet"}</b></div><div><span>Signed</span><b>${rec || "None yet"}</b></div></div>
    <p class="gk-note">His trust in your eye grows as the world catches up with your calls, one bloom at a time.</p></section>`;

  // ---- your goals ----
  const goalLab = ["", "Your area, from him", "Your position, from him", "Your own ground goal"];
  const goals = x.goals.map((g) => {
    const share = g.target ? Math.min(100, Math.round((100 * g.got) / g.target)) : 0;
    return `<div class="gk-goal${g.met === true ? " met" : g.met === false ? " miss" : ""}"><span class="gk-lab">${goalLab[g.kind] || "Goal"}</span><b>${esc(g.text)}</b><small>${g.met === true ? "Met." : g.met === false ? "Short." : g.got ? `${g.got} of ${g.target} so far.` : "None yet."}</small><div class="gk-bar"><i style="width:${share}%"></i></div></div>`;
  }).join("") || '<p class="gk-empty">Your goals come from the February meeting.</p>';
  const picks = x.choices.length
    ? `<div class="gk-lab gk-sub">Pick your own ground goal</div>${x.choices.map((c, k) => `<button class="gk-choice" data-act="gkgoal" data-k="${k}"><b>${esc(c.text)}</b><small>${esc(GK_OWN[c.own] || "")}</small></button>`).join("")}`
    : "";
  const ver = x.verdict
    ? `<div class="gk-ver ${x.verdict.score >= 1 ? "good" : x.verdict.score < 0 ? "bad" : ""}"><span class="gk-lab">His August verdict · ${x.verdict.season}</span>${x.verdict.lines.map((l) => `<p>${esc(l)}</p>`).join("")}</div>`
    : `<div class="gk-ver"><div><span class="gk-lab">His August verdict</span><b>Counts when he reviews his staff</b></div></div>`;
  const goalBox = `<section class="gk-pb"><div class="gk-row"><h2>Your goals</h2><span class="gk-dim sm">from the February meeting</span></div>${goals}${picks}${ver}</section>`;

  // ---- your group and your career ----
  const finds = x.finds.length ? `<div class="gk-cell"><span>Your finds</span><b>${x.finds.length}</b><small>${esc(x.finds.slice(0, 3).join(", "))}</small></div>` : `<div class="gk-cell"><span>Your finds</span><b>None yet</b><small>The world has yet to catch up.</small></div>`;
  const career = `<section class="gk-pb"><div class="gk-row"><h2>Your career</h2><span class="gk-dim sm">what brings offers</span></div>
    <div class="gk-cells">${x.offers.map((o) => { const [a, ...b] = o.split(": "); return b.length ? `<div class="gk-cell"><span>${esc(a)}</span><b>${esc(b.join(": "))}</b></div>` : `<div class="gk-cell"><span>Courting you</span><b>${esc(o)}</b></div>`; }).join("")}${finds}</div>
    <p class="gk-note">If ${esc(boss)} is fired, a new head coach who knows you keeps you more readily. If he trusts your eye, he counters to keep you.</p></section>`;

  return `<div class="pad sl gk">${said}${top}<div class="gk-grid"><div class="gk-col wide">${list}${plan}</div><div class="gk-col">${you}${goalBox}${career}</div></div></div>`;
};

// ---- "Bring him a kid": your leads, in your area or your group ----
// The form fills in after the page draws, so the page never waits on the leads.
async function gkFill() {
  const box = document.getElementById("gkbring");
  if (!box || box.dataset.done) return;
  box.dataset.done = "1";
  let leads = [];
  try { leads = (await api("ground_leads", {})).leads || []; } catch (e) { return; }
  if (!leads.length) { box.innerHTML = '<p class="gk-empty">Nobody on your leads yet. Find a kid on your ground first.</p>'; return; }
  box.innerHTML = `<div class="gk-lab">Bring him a kid</div><div class="gk-form"><select id="gkkid">${leads.map((l) => `<option value="${l.kid.id}">${esc(l.kid.name)} · ${esc(l.kid.pos)} · ${esc(l.kid.school)}</option>`).join("")}</select>
    <input id="gkcase" type="text" maxlength="160" placeholder="Your case, in a line (optional)"><button class="st-btn glass" data-act="gkpush">Bring him</button></div>`;
}

const gkDone = async (el, p) => {
  try {
    const r = await p;
    if (r && r.said) GK.said = Array.isArray(r.said) ? r.said.join(" ") : r.said;
  } catch (e) { toast(e.message || String(e)); }
  refresh(el);
};
Object.assign(CORE_MORE, {
  gkpush: (el) => {
    const id = document.getElementById("gkkid");
    const c = document.getElementById("gkcase");
    return gkDone(el, api("ground_push", { id: Number(id && id.value), case: c ? c.value : "" }));
  },
  gkback: (el) => gkDone(el, api("ground_push_back", { id: Number(el.dataset.id) })),
  gkride: (el) => gkDone(el, api("ground_let_ride", { id: Number(el.dataset.id) })),
  gkgoal: (el) => gkDone(el, api("ground_desk_goal", { k: Number(el.dataset.k) })),
});

// After the page draws: the form, and your head coach's head drawn from his
// recipe, kept in the save's folder for next time.
(() => {
  const after0 = afterRender;
  afterRender = function () {
    after0();
    gkFill();
    for (const f of GK.faces) {
      const box = document.querySelector(`.gk-face[data-gkface="${f.id}"]`);
      if (!box || box.querySelector("img") || !f.drawn_on || !f.spec || !window.AFMFaces || !AFMFaces.liveHead) continue;
      AFMFaces.liveHead(`gk${f.id}-${f.key}`, JSON.parse(JSON.stringify(f.spec))).then((url) => {
        if (!url) return;
        const b = document.querySelector(`.gk-face[data-gkface="${f.id}"]`);
        if (b && !b.querySelector("img")) b.innerHTML = `<img src="${url}" alt="">`;
        api("face_drawn_store", { id: f.id, key: f.key, data: url }).catch(() => {});
      });
    }
  };
})();
