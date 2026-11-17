/* Ground's week: the meeting at the map (lane S2; docs/design/ground.md part N,
 * board 2b, GR71). Monday at 8 AM your recruiting coordinator has drafted the
 * week; you change any of it.
 *
 *   groundweek   his Monday letter, the road days as ticks, your week's free
 *                hours, the board (your men's routes to the places that want
 *                a man), the fronts with where you stand, his one call as a
 *                gold card, and the week day by day: each man on his days
 *                (drag a man to a day) with the scenes at their hour.
 *
 * It reads `ground_week` and writes `ground_road_set`, `ground_road_clear`,
 * `ground_hand` and `ground_call` (lane G3). Rivals' dotted lines and each
 * front's sign-in sheet against its rival come with the race (lane G9); the
 * scenes play in lane G5, so each shows its one line. Every class is gw- so
 * the app's global names never restyle it. */
"use strict";

I.gdweek = '<rect x="3.5" y="5" width="17" height="15" rx="3"/><path d="M3.5 10h17M8 3v4M16 3v4"/>';
navAdd({ go: "groundweek", label: "The meeting", icon: "gdweek", sec: "ground", after: "ground", seats: ["college"] });
CRUMBS.groundweek = "The meeting";

const GW = { art: {}, drag: null, hideCall: null };
async function gwArt(code) {
  if (GW.art[code] !== undefined) return GW.art[code];
  try {
    const r = await fetch(`art/ground/${code}.json`);
    GW.art[code] = r.ok ? await r.json() : null;
  } catch (e) { GW.art[code] = null; }
  return GW.art[code];
}
const gwPt = (art, lat, lon) => {
  const p = art.projection;
  const lo = p.wrap && lon > 0 ? lon - 360 : lon;
  return [p.pad + (lo - p.lon0) * p.cos * p.k, p.pad + (p.lat1 - lat) * p.k];
};
const gwIni = (name, size = 18) => `<span class="gw-ini" style="--s:${size}px">${esc((name || "?").split(/[\s,]+/).filter(Boolean).map((x) => x[0]).slice(0, 2).join("").toUpperCase())}</span>`;
const GW_DAYS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const gwDate = (iso, opt) => new Date(iso + "T12:00:00").toLocaleDateString("en-US", opt);
const gwAdd = (iso, k) => { const d = new Date(iso + "T12:00:00"); d.setDate(d.getDate() + k); return d; };
const gwH = (h) => (Math.round(h * 10) / 10).toString().replace(/\.0$/, "");

screens.groundweek = async () => {
  const x = await api("ground_week", {});
  const art = x.home ? await gwArt(x.home.state) : null;
  const b = x.beat || {};
  title("The meeting", `Week ${b.week || ""}${b.period ? " · " + b.period : ""}`);
  const co = x.coordinator;
  const coLast = co ? co.name.split(" ").pop() : "Your coordinator";
  const spent = x.roads.reduce((n, r) => n + r.days.length, 0);
  const rd = x.road;
  const hrs = x.hours;
  const monday = x.monday;

  // ---- the top bar ----
  const closes = rd && rd.closes ? `<div class="gw-stat"><span>The road closes</span><b>${esc(gwDate(rd.closes, { weekday: "long", month: "long", day: "numeric" }))}</b></div>` : "";
  const roadStat = rd && rd.pool != null ? `<div class="gw-stat"><span>Road days</span><b>${rd.left} of ${rd.pool} left</b><div class="gw-bar"><i style="width:${Math.max(2, Math.round(100 * rd.left / Math.max(1, rd.pool)))}%"></i></div></div>` : rd ? `<div class="gw-stat"><span>Road days</span><b>No limit this window</b></div>` : "";
  const hrsStat = hrs.free > 0 ? `<div class="gw-stat"><span>Your free hours</span><b>${gwH(hrs.left)} of ${gwH(hrs.free)} left</b><div class="gw-bar amb"><i style="width:${Math.min(100, Math.round(100 * (hrs.free - hrs.left) / hrs.free))}%"></i></div></div>` : "";
  const handBtn = x.handed
    ? `<button class="st-btn glass" data-act="gwhand" data-on="0">Take the week back</button>`
    : `<button class="st-btn primary" data-act="gwhand" data-on="1">Hand the week to ${esc(coLast)}</button>`;
  const top = `<div class="gw-top"><div><h1>${esc(gwDate(monday, { weekday: "long", month: "long", day: "numeric", year: "numeric" }))}</h1><small>${esc(b.line || "")}</small></div><div class="gw-sp"></div>${closes}${roadStat}${hrsStat}${handBtn}</div>`;

  // ---- the letter ----
  const letter = x.letter || [];
  const quote = letter[0] || "";
  // The plan and the calls are on the board and the days: the letter keeps what moved, and the whole of it stays one click away.
  const gwDup = /^(The plan|For you this week|Calls for you)\b/;
  const rest = letter.slice(1).filter((l) => !gwDup.test(l)).map((l) => `<p>${esc(l)}</p>`).join("");
  const whole = letter.length > 1 ? `<details class="gw-whole"><summary>The whole letter</summary>${letter.slice(1).map((l) => `<p>${esc(l)}</p>`).join("")}</details>` : "";
  const letterBox = `<section class="gw-pb"><div class="gw-who">${gwIni(co ? co.name : "?", 44)}<div><div class="gw-lab">Monday's letter</div><b>${esc(co ? co.name : "Your recruiting coordinator")}</b><small>${esc(co ? co.job : "")}</small></div></div>${quote ? `<p class="gw-q">${esc(quote)}</p>` : ""}<div class="gw-note">${rest || '<p class="gw-dim">Nothing new since last week.</p>'}</div>${whole}</section>`;

  // ---- the road days ----
  let roadBox;
  if (rd && rd.pool != null) {
    const n = Math.min(rd.left, 44), s = Math.min(spent, n);
    const ticks = Array.from({ length: n }, (_, i) => `<i class="${i < s ? "" : "k"}"></i>`).join("");
    roadBox = `<section class="gw-pb"><div class="gw-row"><div><div class="gw-lab">Road days</div><div class="gw-big"><span>${spent}</span><small>spent this week<br>of ${rd.left} left</small></div></div>${rd.closes ? `<span class="gw-chip amb">Closes ${esc(gwDate(rd.closes, { weekday: "short" }))}</span>` : ""}</div>
      <div class="gw-ticks" role="img" aria-label="${rd.left} road days left: ${spent} spent this week">${ticks}</div>
      <div class="gw-row sm"><span>${spent} in ${esc(coLast)}'s draft${x.money && x.money.flights ? `: ${esc(x.money.words)}` : ""}</span><span>${Math.max(0, rd.left - spent)} kept back</span></div></section>`;
  } else {
    roadBox = `<section class="gw-pb"><div class="gw-lab">Road days</div>${rd ? `<div class="gw-big"><span>${spent}</span><small>days out this week<br>no limit in this window</small></div>` : `<p class="gw-dim">The rules keep your staff home this week.${x.beat && x.beat.on && x.beat.on.length ? ` Next: ${esc(x.beat.on[0].what)}.` : ""}</p>`}</section>`;
  }

  // ---- your week's free hours ----
  const booked = Math.max(0, hrs.week - hrs.chosen);
  const parts = [["Film", hrs.film, "#e8e6e0"], ["Your calls", hrs.chosen, "#b9bcc4"], ["Still free", hrs.left, "#5b6070"]];
  const tot = Math.max(1, hrs.film + hrs.chosen + hrs.left);
  const hoursBox = `<section class="gw-pb"><div class="gw-row"><div class="gw-lab">Your week</div><b class="gw-tn">${gwH(hrs.free - hrs.left)} of ${gwH(hrs.free)} free hours</b></div>
    <div class="gw-ledg" aria-hidden="true">${parts.map((p) => `<i style="width:${(100 * p[1] / tot).toFixed(1)}%;background:${p[2]}"></i>`).join("")}</div>
    <div class="gw-lgrid">${parts.map((p) => `<span><i style="background:${p[2]}"></i>${p[0]}<b>${gwH(p[1])} h</b></span>`).join("")}${booked ? `<span class="wide"><i style="background:var(--gw-gold)"></i>Booked: the road and what you host<b>${gwH(booked)} h</b></span>` : ""}</div>
    <small class="gw-dim">${x.slots.filter((s) => s.yours).length} scenes come to you this week; ${esc(coLast)} plays the rest.${x.off ? " You have the week off." : ""}</small></section>`;

  // ---- the board ----
  const board = gwBoard(x, art);

  // ---- the fronts ----
  const fronts = x.fronts.length ? x.fronts.map((f) => {
    const pct = f.of ? Math.round(100 * f.known / f.of) : 0;
    const men = f.men.map((m) => esc(m.name)).join(", ");
    const tag = f.kind === "your call" ? "your call" : f.kind === "new door" ? "new door" : "the plan";
    return `<div class="gw-fr"><span class="gw-mk${f.kind === "new door" ? " new" : ""}">${f.n}</span><div><div class="gw-tt"><b>${esc(f.area.name)}</b><span class="gw-chip sm">${tag}</span></div>
      <p>${esc(men)} · ${esc(f.days_words)}${f.flight ? ", flying" : ""}. ${esc(f.why)}</p>
      <div class="gw-rc"><span class="gw-bar2"><i style="width:${Math.max(pct, f.known ? 4 : 0)}%"></i></span><b>${f.known ? `Known with ${f.known} of ${f.of}` : `Known with none of ${f.of}`}</b></div></div></div>`;
  }).join("") : `<div class="gw-empty">${x.road ? "Nobody is out this week. Drag a man to a day below to send him." : "No road this week."}</div>`;
  const frontsBox = `<section class="gw-pb"><div class="gw-row"><div class="gw-lab">${x.fronts.length === 1 ? "The front" : "The fronts"}</div><span class="gw-dim sm">where the plan sends a man</span></div>${fronts}</section>`;

  // ---- his one call ----
  const oc = x.one_call;
  const callBox = oc && GW.hideCall !== oc.who ? `<section class="gw-dec"><div class="gw-who">${gwIni(co ? co.name : "?", 30)}<div class="gw-lab gold">${esc(coLast)}'s one call</div></div>
      <div class="gw-ask">Call ${esc(oc.name)}${oc.where ? `, ${esc(oc.where)}` : ""}?</div>
      <p class="gw-why">${esc(oc.why)}</p>
      <div class="gw-chips"><span class="gw-chip">${esc(oc.role)}</span><span class="gw-chip">You: ${esc(oc.level)}</span><span class="gw-chip amb">${gwH(oc.hours * 60)} minutes of your week</span></div>
      <div class="gw-two"><button class="st-btn primary" data-act="gwcall" data-who="${esc(oc.who)}">Call him</button><button class="st-btn glass" data-act="gwleave" data-who="${esc(oc.who)}">Leave it to ${esc(coLast)}</button></div>
      ${oc.more ? `<small class="gw-dim">${oc.more} more on your list this week.</small>` : ""}</section>` : "";

  // ---- the week, day by day ----
  const days = gwDays(x);

  return `<div class="pad sl gw">${top}<div class="gw-grid"><div class="gw-col a">${letterBox}${roadBox}${hoursBox}</div><section class="gw-pb gw-boardbox"><div><div class="gw-lab">The board</div><b class="gw-h">${esc(gwHeadline(x))}</b></div>${board}</section><div class="gw-col c">${frontsBox}${callBox}</div></div>${days}</div>`;
};

function gwHeadline(x) {
  const n = x.fronts.length;
  const closes = x.road && x.road.closes ? `, and the road closes ${gwDate(x.road.closes, { weekday: "long" })}` : "";
  if (!n) return x.road ? "Nobody is on the road yet." : "No road this week.";
  return `${n === 1 ? "One front wants" : n + " fronts want"} a man${closes}.`;
}

function gwBoard(x, art) {
  if (!art) return `<div class="gw-empty">No map is drawn for this state yet.</div>`;
  const W = art.projection.w, H = art.projection.h;
  const byCode = Object.fromEntries(x.fronts.map((f) => [f.area.code, f]));
  const paths = Object.entries(art.areas).map(([k, v]) => {
    const f = byCode[k];
    return `<path class="${f ? "gw-front" : v.home && v.home !== x.home.state ? "gw-away" : "gw-area"}" d="${v.d}"/>`;
  }).join("");
  const [cx, cy] = gwPt(art, x.home.campus[0], x.home.campus[1]);
  // A front inside the state is drawn where it is; one past the frame (a
  // flight to another state) runs to the frame's edge and is listed below.
  const near = (f) => { const [tx, ty] = gwPt(art, f.label[0], f.label[1]); return { f, tx, ty, inside: tx >= 14 && tx <= W - 14 && ty >= 14 && ty <= H - 14 }; };
  const marks = x.fronts.map(near);
  const edge = (m) => {
    const dx = m.tx - cx, dy = m.ty - cy;
    const t = Math.min(dx > 0 ? (W - 16 - cx) / dx : dx < 0 ? (16 - cx) / dx : 1e9, dy > 0 ? (H - 16 - cy) / dy : dy < 0 ? (16 - cy) / dy : 1e9, 1);
    return [cx + dx * t, cy + dy * t];
  };
  const routes = marks.map((m) => {
    const [tx, ty] = m.inside ? [m.tx, m.ty] : edge(m);
    const mx = (cx + tx) / 2 + (ty - cy) * 0.12, my = (cy + ty) / 2 - (tx - cx) * 0.12;
    return `<path class="gw-route${m.f.flight ? " fly" : ""}" d="M${cx.toFixed(1)} ${cy.toFixed(1)} Q ${mx.toFixed(1)} ${my.toFixed(1)} ${tx.toFixed(1)} ${ty.toFixed(1)}" vector-effect="non-scaling-stroke"/>`;
  }).join("");
  const pct = (v, lo, hi) => Math.min(hi, Math.max(lo, v)).toFixed(2);
  const pins = marks.map((m) => {
    const f = m.f;
    const [tx, ty] = m.inside ? [m.tx, m.ty] : edge(m);
    const l = pct(tx / W * 100, 3, 97), t = pct(ty / H * 100, 3, 97);
    const mk = `<span class="gw-pin" style="left:${l}%;top:${t}%"><span class="gw-mk${f.kind === "new door" ? " new" : ""}">${f.n}</span></span>`;
    if (!m.inside) return mk;
    const side = tx / W > 0.62 ? " r" : "";
    const men = f.men.map((q) => q.name).join(", ");
    return `${mk}<span class="gw-pill${side}" style="left:${l}%;top:${t}%"><span><b>${esc(f.area.name)}</b> <small>${esc(men)} · ${esc(f.days_words)}</small></span></span>`;
  }).join("");
  const away = marks.filter((m) => !m.inside).map((m) => `<span class="gw-ao"><span class="gw-mk${m.f.kind === "new door" ? " new" : ""}">${m.f.n}</span><b>${esc(m.f.area.name)}</b><small>${esc(m.f.men.map((q) => q.name).join(", "))} · ${esc(m.f.trip.replace(/^a /, ""))}</small></span>`).join("");
  const [hx, hy] = [cx / W * 100, cy / H * 100];
  const campus = `<span class="gw-pin" style="left:${pct(hx, 3, 97)}%;top:${pct(hy, 3, 97)}%">${typeof mark === "function" && S.status && S.status.mine ? mark(S.status.mine, 24) : '<span class="gw-ini" style="--s:24px">U</span>'}</span>`;
  return `<div class="gw-map" style="aspect-ratio:${W}/${H}"><svg viewBox="0 0 ${W} ${H}" aria-hidden="true">${paths}<path class="gw-bord" d="${art.borders}" vector-effect="non-scaling-stroke"/><path class="gw-edge" d="${art.edge}" vector-effect="non-scaling-stroke"/>${routes}</svg>${pins}${campus}
    <div class="gw-legend"><span><i class="line"></i>Your men</span><span><i class="line fly"></i>Flying</span></div></div>${away ? `<div class="gw-away2"><span class="gw-lab">Past your state's edge</span>${away}</div>` : ""}`;
}

function gwDays(x) {
  const monday = x.monday;
  const slotsBy = {};
  for (const s of x.slots) (slotsBy[s.date] = slotsBy[s.date] || []).push(s);
  const roadDays = new Set(x.roads.flatMap((r) => r.days));
  const last = Math.max(4, ...x.roads.flatMap((r) => r.days), ...x.slots.map((s) => Math.round((new Date(s.date) - new Date(monday)) / 864e5)));
  const cols = [];
  for (let k = 0; k <= Math.min(6, last); k++) {
    const d = gwAdd(monday, k);
    const iso = d.toISOString().slice(0, 10);
    const out = x.roads.filter((r) => r.days.includes(k));
    const chips = out.map((r) => {
      const nm = (r.name || "").split(" ").pop();
      return `<div class="gw-dc${r.flight ? " fly" : ""}${r.yours ? " yours" : ""}" draggable="true" data-man="${r.man}" data-day="${k}" data-area="${r.area.id}" title="${esc(r.why)}${r.yours ? "" : ". Drag him to another day."}">${gwIni(r.name)}<span class="tx"><b>${esc(nm)}</b> <em>${esc(r.area.name)}</em></span><button class="gw-x" data-act="gwcut" data-man="${r.man}" data-day="${k}" aria-label="Take ${esc(nm)} off ${GW_DAYS[k]}">×</button></div>`;
    }).join("");
    const sl = (slotsBy[iso] || []).map((s) => {
      const done = s.line && !s.yours ? `<small>${esc(s.line)}</small>` : `<small>${esc(s.title)}</small>`;
      const play = s.scene != null && s.yours ? ` data-go="people_scene" data-id="${s.scene}" style="cursor:pointer"` : "";
      return `<div class="gw-sn${s.yours ? " yours" : ""}" title="${esc(s.reason)}"${play}><i></i><div><b>${esc(s.time)}</b>${done}</div></div>`;
    }).join("");
    const past = x.today && k < Math.round((new Date(x.today) - new Date(monday)) / 864e5) ? " past" : "";
    cols.push(`<div class="gw-day${slotsBy[iso] && slotsBy[iso].some((s) => s.yours) && !out.length ? " scene" : ""}${past}" data-k="${k}"><div class="gw-dh"><span>${d.getDate()}</span><div><b>${GW_DAYS[k]}</b><small>${out.length ? `${out.length} out` : slotsBy[iso] ? "At home" : "Open"}</small></div></div>${chips}${sl ? `<div class="gw-snw">${sl}</div>` : ""}</div>`);
  }
  // Men at home: drag one to a day to send him to the first front, or pick his place.
  const home = x.men.filter((m) => !m.out);
  const inFront = new Set(x.fronts.map((f) => f.area.code));
  const opts = x.fronts.map((f) => `<option value="${esc(f.area.code)}">${esc(f.area.name)}</option>`).join("")
    + (x.home.areas.some((a) => !inFront.has(a.code)) ? `<optgroup label="Another area">${x.home.areas.filter((a) => !inFront.has(a.code)).map((a) => `<option value="${esc(a.code)}">${esc(a.name)}</option>`).join("")}</optgroup>` : "");
  const tray = home.length ? `<div class="gw-tray"><span class="gw-lab">At home</span>${home.map((m) => `<span class="gw-hm${m.rule ? " ruled" : ""}" draggable="${m.rule ? "false" : "true"}" data-man="${m.man}" title="${esc(m.rule || "Drag him to a day")}">${gwIni(m.name)}<b>${esc(m.name.split(" ").pop())}</b><em>${esc(m.job)}</em>${m.rule ? `<small>${esc(m.rule)}</small>` : `<select aria-label="Where ${esc(m.name)} goes">${opts}</select>`}</span>`).join("")}</div>` : "";
  return `<section class="gw-pb gw-week"><div class="gw-row"><div class="gw-wt"><b>The week, day by day</b><span class="gw-dim sm">Drag a man to a day. Most go out three days or less, so the film gets done.</span></div>
    <div class="gw-key"><span><i class="dot on"></i>On the road</span><span><i class="dot fly"></i>Flying</span><span><i class="dot sc"></i>A scene for you</span></div></div>
    <div class="gw-days" style="--n:${cols.length}">${cols.join("")}</div>${tray}</section>`;
}

// ---- the week's changes ----
const gwSay = (r) => { if (r && r.said) toast(r.said); };
Object.assign(CORE_MORE, {
  gwhand: async (el) => {
    try { gwSay(await api("ground_hand", { on: el.dataset.on === "1" })); } catch (e) { toast(e.message); }
    refresh(el);
  },
  gwcall: async (el) => {
    try { const r = await api("ground_call", { who: el.dataset.who }); if (r.said) toast(Array.isArray(r.said) ? r.said.join(" ") : r.said); } catch (e) { toast(e.message); }
    refresh(el);
  },
  gwleave: (el) => { GW.hideCall = el.dataset.who; refresh(el); },
  gwcut: async (el) => {
    const man = +el.dataset.man, k = +el.dataset.day;
    try {
      const x = await api("ground_week", {});
      const r = x.roads.find((q) => q.man === man);
      if (!r) return;
      const left = r.days.filter((d) => d !== k);
      gwSay(left.length ? await api("ground_road_set", { man, area: r.area.id, days: left }) : await api("ground_road_clear", { man }));
    } catch (e) { toast(e.message); }
    refresh(el);
  },
});

// Drag a man to a day: a man on the road moves his day; a man at home goes
// to the chosen front for that day.
document.addEventListener("dragstart", (ev) => {
  const el = ev.target.closest && ev.target.closest(".gw-dc[draggable], .gw-hm[draggable=true]");
  if (!el) return;
  GW.drag = { man: +el.dataset.man, day: el.dataset.day != null ? +el.dataset.day : null, area: null, el };
  ev.dataTransfer.effectAllowed = "move";
  ev.dataTransfer.setData("text/plain", String(GW.drag.man));
  el.classList.add("drag");
});
document.addEventListener("dragend", () => {
  if (GW.drag) GW.drag.el.classList.remove("drag");
  document.querySelectorAll(".gw-day.over").forEach((d) => d.classList.remove("over"));
  GW.drag = null;
});
document.addEventListener("dragover", (ev) => {
  const day = ev.target.closest && ev.target.closest(".gw-day");
  if (!day || !GW.drag) return;
  ev.preventDefault();
  document.querySelectorAll(".gw-day.over").forEach((d) => d !== day && d.classList.remove("over"));
  day.classList.add("over");
});
document.addEventListener("drop", async (ev) => {
  const day = ev.target.closest && ev.target.closest(".gw-day");
  if (!day || !GW.drag) return;
  ev.preventDefault();
  const { man, day: from, el } = GW.drag;
  const to = +day.dataset.k;
  let area = GW.drag.area;
  const sel = el.querySelector && el.querySelector("select");
  if (sel) area = sel.value;
  else area = null;
  GW.drag = null;
  if (from === to) return;
  try {
    const x = await api("ground_week", {});
    const r = x.roads.find((q) => q.man === man);
    let days;
    if (r) { area = r.area.code; days = [...new Set([...r.days.filter((d) => d !== from), to])].sort(); }
    else days = [to];
    if (!area) throw new Error("Pick a place first.");
    gwSay(await api("ground_road_set", { man, area, days }));
  } catch (e) { toast(e.message); }
  refresh(day);
});
