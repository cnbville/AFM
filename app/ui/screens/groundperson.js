/* Ground's person (lane S5; docs/design/ground.md part N, board 5, GR74,
 * GR79). One person on your ground in the glass look with his place's
 * colors: his face and life, your staff's read of him, his memory of you in
 * his words (dated, the newest marked, the old fading), your word, favors,
 * his kids with you, what you've talked through, what your level with him
 * gets you and what the next would, where he stands with every staff and
 * his ties.
 *
 *   groundperson   with a person ({kind, id}): his page. Without one: the
 *                  people you have dealt with, the last in touch first.
 *
 * It reads `ground_person` and `ground_people`, and writes `ground_call`
 * (lane G3) and `face_drawn_store` (a drawn head, kept for next time). No
 * hidden number is ever on it: his eye, how straight he is and his nature
 * are words from his record and your staff's read. Every class is gp- (the
 * glass and rows reuse the school page's gd- ones) so the app's global names
 * never restyle it. */
"use strict";

I.gdpeople = '<circle cx="9" cy="8" r="3.2"/><path d="M3 20c0-3.3 2.7-6 6-6s6 2.7 6 6"/><circle cx="17" cy="9" r="2.4"/><path d="M16.5 14.2c2.6.2 4.5 2.4 4.5 5.3"/>';
navAdd({ go: "groundperson", label: "People", icon: "gdpeople", sec: "ground", after: "groundweek", seats: ["college"] });
CRUMBS.groundperson = "People";

const GP = { face: null };
const gpLink = (k, inner, cls = "") => `<a class="${cls}" data-go="groundperson" data-kind="${esc(k.kind)}" data-id="${k.id}">${inner}</a>`;
const gpCap = (s) => (s ? s.charAt(0).toUpperCase() + s.slice(1) : s);

screens.groundperson = async (args) => {
  if (!args || args.id == null) return gpList();
  const x = await api("ground_person", { kind: args.kind || "coach", id: args.id });
  title(x.name, gpCap(x.kind));
  GP.face = x.face;
  const [c1, c2] = x.face.colors;
  const sr = x.staff_read;

  // ---- the topbar ----
  const top = `<div class="gd-top"><div><h1>${esc(x.today)}</h1><small>People · ${esc(gpCap(x.kind))} · ${esc(x.held_by ? `your standing is with ${x.held_by}` : "you haven't dealt with him yet")}</small></div><div class="gd-sp"></div>
    <div class="gp-stat"><span>With you</span><b>${esc(x.level)}</b></div>
    <div class="gp-stat"><span>Next level</span><b>${esc(x.next)}</b></div>
    <div class="gp-stat"><span>Last in touch</span><b>${esc(x.last || "Never")}</b></div>
    <button class="st-btn primary" data-act="gpcall" data-who="${esc(x.call)}">Call him</button></div>`;

  // ---- the left column: his face, his life, your staff's read ----
  const f = x.face;
  const pic = f.drawn
    ? `<img class="gp-face-img" src="${f.drawn}" alt="${esc(x.name)}">`
    : `<span class="gp-mono" style="--c1:${esc(c1)};--c2:${esc(c2)}">${esc(f.initials)}</span>`;
  const school = x.school != null ? ` <a data-go="groundschool" data-id="${x.school}">${esc(x.role.split(" at ")[1] || "his school")}</a>` : "";
  const roleLine = x.school != null ? `${esc(gpCap(x.role.split(" at ")[0]))},${school}` : esc(gpCap(x.role));
  const line = (l, t) => `<div class="gp-line"><span>${l}</span><div>${t}</div></div>`;
  const life = x.life.length ? line("His life", esc(x.life.map((t, i) => (i ? t : gpCap(t))).join(". ") + ".")) : "";
  const wants = x.wants.length ? line("He wants", esc(gpCap(x.wants.join(", ")) + ".")) : "";
  const hour = x.hour ? line("His hour", `He writes and calls around ${esc(x.hour)}.`) : "";
  const left = `<section class="gd-glass gp-hero"><div class="gp-face" data-gpface="${f.id}">${pic}</div>
      <div class="gp-name"><h1>${esc(x.name)}</h1><div class="gd-sub">${roleLine}${x.age ? ` · ${x.age}` : ""}</div></div>
      <div class="gp-lines">${life}${wants}${line("He pushes", esc(gpCap(x.agenda) + "."))}${hour}</div></section>
    <section class="gd-glass"><div class="gd-lab2">Your staff's read of him</div>${
      sr.eye || sr.straight || sr.nature
        ? `${sr.eye ? line("His eye", esc(sr.eye)) : ""}${sr.straight ? line("Straight", esc(sr.straight)) : ""}${sr.nature ? line("Nature", esc(sr.nature)) : ""}`
        : '<div class="gd-empty">Your staff doesn\'t know him well enough to read him yet.</div>'}</section>`;

  // ---- the middle: his memory of you ----
  const moments = x.moments.length
    ? x.moments.map((m) => `<div class="gp-mo${m.new ? " new" : ""}${m.fade ? " fade" : ""}${m.good ? "" : " bad"}"><span class="d">${esc(m.date)}</span><div><div class="q">${esc(m.words)}</div>${m.note ? `<small>${esc(m.note)}</small>` : ""}</div></div>`).join("")
    : '<div class="gd-empty">He keeps nothing of you yet. Nobody on your staff has dealt with him.</div>';
  const favor = x.favors ? `<b>${esc(x.favors)}</b><small>${x.favor_list.length ? esc(x.favor_list[0][1]) : ""}</small>` : "<b>None either way</b><small>Nobody owes anybody.</small>";
  const kids = x.kids.length
    ? `<b>${x.kids.length}${x.kids.length === 1 ? `, ${esc(x.kids[0][1])}` : ""}</b><small>${x.kids.slice(0, 3).map((k) => `${esc(k[0])}, ${esc(k[1])}`).join("; ")}${x.kids.length > 3 ? `; and ${x.kids.length - 3} more` : ""}.</small>`
    : "<b>None yet</b><small>None of his kids has come through you.</small>";
  const word = x.your_word ? `<b>${esc(x.your_word)}</b><small>${x.his_word ? `His: ${esc(x.his_word)}.` : ""}</small>` : `<b>Nothing settled</b><small>${x.his_word ? `His: ${esc(x.his_word)}.` : "No call of yours has played out with him yet."}</small>`;
  const talked = x.talked.times
    ? `<div style="display:flex;gap:6px;flex-wrap:wrap">${x.talked.about.map((t) => `<span class="gd-chip">${esc(t)}</span>`).join("")}<span class="gd-chip dim">${x.talked.times} talk${x.talked.times === 1 ? "" : "s"}${x.last ? `, last ${esc(x.last.toLowerCase())}` : ""}</span></div>`
    : '<div class="gd-empty">You haven\'t talked it through with him yet.</div>';
  const gets = x.gets;
  const nextCell = gets.next ? `<div><span>${esc(gets.next.level)}, next</span><small>${esc(gets.next.words)}</small></div>` : `<div><span>The top</span><small>There is nothing above Home. Keep it.</small></div>`;
  const mid = `<section class="gd-glass gp-mid"><div class="gp-head"><div class="gd-lab2">His memory of you · what he keeps, in his words</div><span class="dim">one record, read by every talk</span></div>
      <p class="gp-why">${esc(x.why)}</p>
      <div class="gp-rows">${moments}</div><div class="gd-hr"></div>
      <div class="gp-mini"><div><span>Your word</span>${word}</div><div><span>Favors</span>${favor}</div><div><span>His kids with you</span>${kids}</div></div>
      <div class="gd-lab2">What you've talked through</div>${talked}
      <div class="gd-lab2">What ${esc(gets.now.level)} gets you${gets.next ? `, and what ${esc(gets.next.level)} would` : ""}</div>
      <div class="gp-mini two"><div><span class="now">${esc(gets.now.level)}, now</span><small>${esc(gets.now.words)}</small></div>${nextCell}</div>
      <p class="dim gp-foot">What you do with him reaches his close ties in a few days at about half its weight. A talk only words what's in this record.</p></section>`;

  // ---- the right column: every staff, his ties ----
  const staffs = x.staffs.length
    ? `<section class="gd-glass"><div class="gd-lab2">His standing with every staff</div>${x.staffs.map((s) => `<div class="gp-st"><div><b style="color:${s.you ? "#8fb0ff" : gdTone(s.color, 1.05)}"><a data-go="team" data-kind="college" data-id="${s.program}" style="color:inherit">${esc(s.name)}</a></b>${s.note ? `<small>${esc(s.note)}</small>` : ""}</div>${gdLv(s.level, s.you ? "you" : "")}</div>`).join("")}<small class="dim">Your staff's read from sign-in sheets and what he says. A loyal man calls the staff he's closest to first.</small></section>`
    : "";
  const ties = x.ties.length
    ? `<section class="gd-glass"><div class="gd-lab2">His ties</div>${x.ties.map((t) => gpLink(t.key, `<span class="gp-tie">${gdIni(t.name, 28)}<div><b>${esc(t.name)}</b><small>${esc(gpCap(t.role))}</small></div>${gdLv(t.lv, t.lv.n ? "you" : "")}</span>`, "gp-tielink")).join("")}</section>`
    : "";
  const right = `<div class="gp-col r">${staffs}${ties}</div>`;

  return `<div class="sl gd"><div class="gd-room gp-room" style="--c1:${esc(c1)};--c2:${esc(c2)}">${top}<div class="gp-body"><div class="gp-col l">${left}</div><div class="gp-col m">${mid}</div>${right}</div></div></div>`;
};

// Where the People page starts: everyone your staff has dealt with.
async function gpList() {
  const x = await api("ground_people", {});
  title("People", `${x.total} on your ground`);
  const rows = x.people.map((p) => gpLink(p.key, `<span class="gp-tie lg">${gdIni(p.name, 36)}<div><b>${esc(p.name)}</b><small>${esc(gpCap(p.role))} · last in touch ${esc(p.last.toLowerCase())}</small></div>${gdLv(p.lv, p.lv.n ? "you" : "")}</span>`, "gp-tielink")).join("");
  return `<div class="sl gd"><div class="gd-room gp-room" style="--c1:#1d7a55;--c2:#0a2d20"><div class="gd-top"><div><h1>People</h1><small>${esc(x.today)} · ${x.total} on your ground, the last in touch first</small></div></div>
    <section class="gd-glass gp-list">${rows || '<div class="gd-empty">Nobody yet. Visit a school or call a coach and he shows here, with what he keeps of you.</div>'}</section></div></div>`;
}

Object.assign(CORE_MORE, {
  gpcall: async (el) => {
    try {
      const r = await api("ground_call", { who: el.dataset.who });
      if (r && r.said) toast(Array.isArray(r.said) ? r.said.join(" ") : r.said);
    } catch (e) { toast(e.message); }
    refresh(el);
  },
});

// After the page draws: his head, drawn from his recipe (a counselor and a
// family keep their initials), kept in the save's folder for next time.
(() => {
  const after0 = afterRender;
  afterRender = function () {
    after0();
    const f = GP.face;
    const box = f && document.querySelector(`.gp-face[data-gpface="${f.id}"]`);
    if (!box || box.querySelector("img") || !f.drawn_on || !f.spec || !window.AFMFaces || !AFMFaces.liveHead) return;
    AFMFaces.liveHead(`gp${f.id}-${f.key}`, JSON.parse(JSON.stringify(f.spec))).then((url) => {
      if (!url) return;
      const b = document.querySelector(`.gp-face[data-gpface="${f.id}"]`);
      if (b && !b.querySelector("img")) b.innerHTML = `<img class="gp-face-img" src="${url}" alt="">`;
      api("face_drawn_store", { id: f.id, key: f.key, data: url }).catch(() => {});
    });
  };
})();
