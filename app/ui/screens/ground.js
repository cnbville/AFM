/* Ground's map and school page (lane S1; docs/design/ground.md part N, board 1,
 * GR2, GR5, GR6, GR8, GR11, GR70, GR79).
 *
 *   ground         the state in its leaders' colors, your standing beside each
 *                  area, fog where your ground hasn't opened it, your signees,
 *                  the areas' legends and rivals' pipelines, what people kept
 *                  of you lately, and the area you pick: who leads it, you,
 *                  its people by name.
 *   groundschool   a school as a place: its colors, people, where its kids
 *                  went, its seasons and the kids you have signed from it.
 *
 * Data screens use the Press Box look (dark blocks, ivory text, one gold
 * accent for what needs a call, amber for what slips); the school page uses
 * the glass look with the school's own colors (GR79). Every class is gd- so
 * the app's global names never restyle it. Your men's routes, road days and
 * the week's hours come with the week (lane G3). */
"use strict";

I.gdmap = '<path d="M12 21s-7-6.2-7-11a7 7 0 0 1 14 0c0 4.8-7 11-7 11z"/><circle cx="12" cy="10" r="2.5"/>';
navAdd({ go: "ground", label: "The map", icon: "gdmap", sec: "ground", seats: ["college"] });
CRUMBS.ground = "The map";
CRUMBS.groundschool = "School";

const GD_ART = {};
async function gdArt(code) {
  if (GD_ART[code]) return GD_ART[code];
  try {
    const r = await fetch(`art/ground/${code}.json`);
    GD_ART[code] = r.ok ? await r.json() : null;
  } catch (e) { GD_ART[code] = null; }
  return GD_ART[code];
}

// A team color toned down for a fill: toward the sea so nothing shouts (GR79).
function gdTone(hex, amt = 0.5) {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex || "");
  if (!m) return "#3a3f4a";
  const n = parseInt(m[1], 16), c = [(n >> 16) & 255, (n >> 8) & 255, n & 255];
  const lum = (0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]) / 255;
  const k = lum > 0.62 ? amt * 0.7 : amt;
  let mix = c.map((v, i) => Math.round(v * k + [16, 18, 23][i] * (1 - k)));
  // A very dark color (navy, black) would vanish into the sea: lift it a little.
  if ((0.2126 * mix[0] + 0.7152 * mix[1] + 0.0722 * mix[2]) / 255 < 0.15) mix = mix.map((v, i) => Math.round(v * 0.65 + [84, 96, 122][i] * 0.35));
  return `rgb(${mix[0]},${mix[1]},${mix[2]})`;
}
const gdPt = (art, lat, lon) => {
  const p = art.projection;
  const lo = p.wrap && lon > 0 ? lon - 360 : lon;
  return [p.pad + (lo - p.lon0) * p.cos * p.k, p.pad + (p.lat1 - lat) * p.k];
};
const gdLv = (lv, cls = "you") => `<span class="gd-lv ${cls}">${esc(lv.name)} <i>${[1, 2, 3].map((n) => `<b${lv.n >= n ? ' class="on"' : ""}></b>`).join("")}</i></span>`;
const gdIni = (name, size = 28) => `<span class="gd-ini" style="--s:${size}px">${esc((name || "?").split(/\s+/).map((x) => x[0]).slice(0, 2).join("").toUpperCase())}</span>`;
const gdLayers = () => keep("gdlayers", { owners: true, pins: true, pipes: true });

screens.ground = async (args) => {
  const x = await api("ground_map", { state: args && args.state, area: args && args.area });
  const art = await gdArt(x.state.code);
  title("The map", x.state.name);
  const L = gdLayers();
  const areas = Object.fromEntries(x.areas.map((a) => [a.code, a]));
  const sel = x.picked;
  const W = art ? art.projection.w : 800, H = art ? art.projection.h : 800;

  // ---- the map ----
  let svg = "", labs = "", pins = "";
  if (art) {
    const paths = Object.entries(art.areas).map(([k, v]) => {
      const a = areas[k];
      if (!a) return `<path class="gd-away" d="${v.d}"/>`;
      const own = a.leader && L.owners ? gdTone(a.leader.colors[0], a.leader.yours ? 0.62 : 0.5) : "var(--gd-blank)";
      const cls = `gd-area${k === sel ? " sel" : ""}${a.fog ? " fog" : ""}`;
      return `<path class="${cls}" d="${v.d}" style="fill:${own}" data-act="gdarea" data-area="${esc(k)}" role="button" tabindex="0" aria-label="${esc(a.name)}"><title>${esc(a.name)}</title></path>`;
    }).join("");
    const fog = Object.entries(art.areas).filter(([k]) => areas[k] && areas[k].fog).map(([k, v]) => `<path class="gd-fogp" d="${v.d}"/>`).join("");
    const picked = sel && art.areas[sel] ? `<path class="gd-selp" d="${art.areas[sel].d}"/>` : "";
    svg = `<defs><pattern id="gdhatch" width="9" height="9" patternUnits="userSpaceOnUse" patternTransform="rotate(45)"><line x1="0" y1="0" x2="0" y2="9" stroke="rgba(255,255,255,.13)" stroke-width="3"/></pattern></defs>
      ${paths}<g style="fill:url(#gdhatch);pointer-events:none">${fog}</g><path class="gd-bord" d="${art.borders}"/><path class="gd-edge" d="${art.edge}"/>${picked}`;
    // Pipelines: each area's strongest rival, and the picked area's few more,
    // drawn toward the campus and cut at the frame.
    if (L.pipes) {
      const lines = [];
      for (const a of x.areas) {
        const [lx, ly] = gdPt(art, a.label[0], a.label[1]);
        const ps = a.code === sel ? a.pipelines.slice(0, 3) : a.pipelines.slice(0, 1);
        for (const p of ps) {
          if (p.strength < 0.12) continue;
          let [tx, ty] = gdPt(art, p.at[0], p.at[1]);
          const t = Math.min(1, ...[tx < 0 ? (0 - lx) / (tx - lx) : 1, tx > W ? (W - lx) / (tx - lx) : 1, ty < 0 ? (0 - ly) / (ty - ly) : 1, ty > H ? (H - ly) / (ty - ly) : 1]);
          const ex = lx + (tx - lx) * t, ey = ly + (ty - ly) * t;
          lines.push(`<line x1="${lx.toFixed(1)}" y1="${ly.toFixed(1)}" x2="${ex.toFixed(1)}" y2="${ey.toFixed(1)}" stroke="${esc(p.color)}" stroke-width="${a.code === sel ? 2 : 1.4}" stroke-dasharray="2 5" stroke-linecap="round" opacity="${a.code === sel ? 0.95 : 0.6}"><title>${esc(p.name)} into ${esc(a.name)}</title></line>`);
          if (t < 1 || (a.code === sel && Math.hypot(tx - lx, ty - ly) > 70)) {
            const near = t < 1 ? [ex, ey] : [tx, ty];
            const side = near[0] > W - 90 ? "end" : "start";
            lines.push(`<text x="${(near[0] + (side === "end" ? -6 : 6)).toFixed(1)}" y="${(near[1] + 4).toFixed(1)}" text-anchor="${side}" class="gd-rv">${esc(p.name)}</text>`);
          }
        }
      }
      svg += `<g>${lines.join("")}</g>`;
    }
    if (L.pins) {
      const dots = [];
      for (const a of x.areas) {
        for (const p of a.legends) { const [px, py] = gdPt(art, p.at[0], p.at[1]); dots.push(`<circle cx="${px.toFixed(1)}" cy="${py.toFixed(1)}" r="3.4" class="gd-leg"><title>${esc(p.name)}, ${esc(p.pos)}, ${esc(p.school_name)}: ${esc(p.what)}</title></circle>`); }
        for (const p of a.signees) { const [px, py] = gdPt(art, p.at[0], p.at[1]); dots.push(`<circle cx="${px.toFixed(1)}" cy="${py.toFixed(1)}" r="3.6" class="gd-sig"><title>${esc(p.name)}, ${esc(p.pos)}, ${esc(p.school_name)}: ${esc(p.what)}</title></circle>`); }
      }
      pins = dots.join("");
    }
    svg += `<g>${pins}</g>`;
    // Labels: the area's name, who leads it and where you stand. Crowded
    // ones shrink to a name or hide, and gdFit() decides which.
    const order = [...x.areas].sort((a, b) => (a.code === sel ? -1 : b.code === sel ? 1 : b.people - a.people));
    labs = order.map((a) => {
      const [lx, ly] = gdPt(art, a.label[0], a.label[1]);
      const who = a.leader ? (a.leader.yours ? "yours" : a.leader.name) : "";
      return `<div class="gd-lab${a.fog ? " fog" : ""}${a.code === sel ? " sel" : ""}" data-act="gdarea" data-area="${esc(a.code)}" style="left:${Math.min(84, Math.max(16, lx / W * 100)).toFixed(2)}%;top:${Math.min(94, Math.max(6, ly / H * 100)).toFixed(2)}%"><b>${esc(a.name)}</b><small>${esc(a.nickname)}${who ? ` · ${esc(who)}${a.leader && a.leader.contested ? ", contested" : ""}` : ""}</small>${gdLv(a.you.level, a.you.level.n ? "you" : "")}</div>`;
    }).join("");
  }
  const layerBtn = (k, label) => `<button aria-pressed="${!!L[k]}" data-act="gdlayer" data-k="${k}">${label}</button>`;
  const map = `<div class="gd-mapcol" style="--w:${W};--h:${H}">
    <div class="gd-layers" role="group" aria-label="Map layers">${layerBtn("owners", "Owners")}${layerBtn("pins", "Pins")}${layerBtn("pipes", "Pipelines")}</div>
    <div class="gd-mapwrap">
      <svg viewBox="0 0 ${W} ${H}" class="gd-svg" role="group" aria-label="${esc(x.state.name)} by recruiting area, shaded by who leads each">${svg}</svg>
      <div class="gd-labs">${labs}</div></div>
    <div class="gd-legend">
      <div><span class="gd-dot sig"></span>Your signees since ${x.season - 3}</div>
      <div><span class="gd-dot leg"></span>The area's legends</div>
      <div><svg width="22" height="8" aria-hidden="true"><line x1="1" y1="4" x2="21" y2="4" stroke="#f07a1c" stroke-width="1.6" stroke-dasharray="2 4"/></svg>Rivals' pipelines</div>
      <div><svg width="22" height="10" aria-hidden="true"><rect width="22" height="10" rx="3" fill="#1a1d24"/><line x1="4" y1="10" x2="12" y2="0" stroke="rgba(255,255,255,.25)" stroke-width="2"/><line x1="12" y1="10" x2="20" y2="0" stroke="rgba(255,255,255,.25)" stroke-width="2"/></svg>Fog: your ground hasn't opened it</div>
    </div></div>`;

  // ---- the side ----
  const lately = x.lately.length ? x.lately.map((m) => `<div class="gd-row" ${m.school != null ? `data-go="groundschool" data-id="${m.school}"` : ""}>${gdIni(m.name, 30)}<div><b class="${m.good ? "" : "amber"}">${esc(m.name)}</b><small>${esc(m.role)}: “${esc(m.words)}”</small></div></div>`).join("") : `<div class="gd-empty">Quiet since ${esc(x.since)}. Nobody on your ground has said anything new.</div>`;
  const head = `<section class="gd-pb"><h2>Lately <small>${x.lately.length ? `${x.lately.length} since ${esc(x.since)}` : "what people kept of you"}</small></h2><div class="gd-rows">${lately}</div></section>`;
  const picked = x.detail ? gdDetail(x, x.detail, areas[x.picked]) : "";
  const list = `<section class="gd-pb gd-list"><h2>${esc(x.state.name)} <small>${x.areas.length} area${x.areas.length === 1 ? "" : "s"}</small></h2><div class="gd-rows tight">${[...x.areas].sort((a, b) => b.you.level.points - a.you.level.points).map((a) => `<div class="gd-row${a.code === sel ? " pick" : ""}" data-act="gdarea" data-area="${esc(a.code)}"><span class="gd-sw" style="background:${a.leader ? gdTone(a.leader.colors[0], 0.9) : "transparent"}"></span><div><b>${esc(a.name)}</b><small>${a.leader ? `${a.leader.yours ? "Yours" : esc(a.leader.name)}${a.leader.contested ? ", contested" : ""}` : "Nobody leads it"}</small></div>${gdLv(a.you.level, a.you.level.n ? "you" : "")}</div>`).join("")}</div></section>`;

  const states = `<select class="gd-state" aria-label="State">${x.states.map((s) => `<option value="${s.code}"${s.code === x.state.code ? " selected" : ""}>${esc(s.name)}</option>`).join("")}</select>`;
  const topbar = `<div class="gd-top"><div><h1>${esc(x.today)}</h1><small>${esc(x.period.name)}: ${esc(x.period.note)}</small></div><div class="gd-sp"></div>${x.state.yours ? "" : `<button class="st-btn small glass" data-act="gdhome">Back to your state</button>`}<label class="gd-pick"><span class="st-label">Looking at</span>${states}</label></div>`;
  return `<div class="pad sl gd">${topbar}<div class="gd-body"><div class="gd-left">${art ? map : `<div class="gd-empty">No map is drawn for ${esc(x.state.name)} yet.</div>`}</div><aside class="gd-side">${head}${picked}${list}</aside></div></div>`;
};

function gdDetail(x, d, a) {
  const lead = d.lead
    ? `<div><span>Leads it</span><b style="color:${gdTone(a.leader ? a.leader.colors[0] : "#888", 1)}">${d.lead.yours ? "You" : esc(d.lead.name)}</b>${d.fog ? "" : `<small>${esc(d.lead.line)}${d.lead.coach ? ` ${esc(d.lead.coach.name)}${d.lead.coach.years ? `, ${d.lead.coach.years} year${d.lead.coach.years === 1 ? "" : "s"}` : ""}.` : ""}</small>`}</div>`
    : `<div><span>Leads it</span><b>Nobody</b><small>No program has a hold here.</small></div>`;
  const you = `<div><span>You</span><b>${gdLv(a.you.level, a.you.level.n ? "you" : "")}</b><small>${d.fog ? "Your ground hasn't opened it." : `With ${a.you.known} of its ${a.you.of} head coaches${a.you.since ? `, since ${esc(a.you.since)}` : ""}.`}</small></div>`;
  const words = d.fog
    ? `<p class="gd-note">${esc(a.public)}</p><p class="gd-note dim">Fog: the public view shows here (rosters, stars, box scores). What its coaches say, and what the area is like, opens when you are Known with someone there. A school visit does it.</p>`
    : `<p class="gd-note">${esc(a.public)}${a.words.length ? ` ${esc(a.words.map((w) => w.charAt(0).toUpperCase() + w.slice(1)).join(". "))}.` : ""}</p>`;
  const who = d.fog
    ? `<div class="gd-lab2">Biggest schools, by the public view</div><div class="gd-rows tight">${d.public.map((s) => `<div class="gd-row" data-go="groundschool" data-id="${s.school}">${gdIni(s.name, 26)}<div><b>${esc(s.name)}</b><small>${esc(s.town)} · ${s.enrollment.toLocaleString()} students</small></div><span class="gd-lv">Stranger <i><b></b><b></b><b></b></i></span></div>`).join("")}</div>`
    : `<div class="gd-lab2">Who matters here</div><div class="gd-rows tight">${d.people.map((p) => `<div class="gd-row" data-go="groundperson" data-kind="${esc(p.who.kind)}" data-id="${p.who.id}">${gdIni(p.name, 26)}<div><b>${esc(p.name)}</b><small>${esc(p.role)}</small></div>${gdLv(p.level, p.level.n ? "you" : "")}</div>`).join("") || '<div class="gd-empty">No head coaches on file here.</div>'}</div>`;
  const cells = `<div class="gd-kv c3"><div><span>From campus</span><b>${esc(d.trip.replace(/^a /, "A "))}</b><small>${d.a_day} school${d.a_day === 1 ? "" : "s"} a day</small></div><div><span>Signed</span><b class="tn">${d.signed}</b><small>since ${x.season - 3}</small></div><div><span>Schools</span><b class="tn">${d.schools}</b><small>${d.heads} head coaches</small></div></div>`;
  return `<section class="gd-pb gd-detail"><div><div class="st-label">Selected area</div><h2 class="gd-h">${esc(d.name)}</h2><div class="gd-sub">${esc(d.nickname)}${d.codes && !d.nickname.includes(d.codes.split(",")[0]) ? ` · area code ${esc(d.codes)}` : ""} · ${esc(d.counties)}</div></div>
    <div class="gd-kv c2">${lead}${you}</div>${words}<div class="gd-hr"></div>${who}${cells}</section>`;
}

// A school as a place (GR11), in the glass look with the school's colors.
screens.groundschool = async (args) => {
  const x = await api("ground_school", { id: args.id });
  const s = x.school;
  title(s.name, `${s.town}, ${s.state}`);
  const [c1, c2] = s.colors;
  const folks = x.folks.map((p) => `<div class="gd-person${p.head ? " head" : ""}" data-go="groundperson" data-kind="${esc(p.who.kind)}" data-id="${p.who.id}" style="cursor:pointer">${gdIni(p.name, 40)}<div class="gd-pb2"><b>${esc(p.name)}</b><small>${esc(p.role)}</small>${p.why ? `<small class="why">${esc(p.why)}</small>` : '<small class="why dim">Nobody on your staff has dealt with him yet.</small>'}${p.moment ? `<p class="gd-q ${p.moment.good ? "" : "bad"}">${esc(p.moment.words)} <small>${esc(p.moment.date)}</small></p>` : ""}</div>${gdLv(p.level, p.level.n ? "you" : "")}</div>`).join("");
  const maxk = Math.max(1, ...x.went.map((w) => w.kids));
  const went = x.went.length ? x.went.map((w) => `<div class="gd-went"><span class="gd-sw" style="background:${gdTone(w.color, 0.9)}"></span><b>${teamLinkById(w)}</b><span class="gd-bar"><i style="width:${Math.max(6, Math.round(100 * w.kids / maxk))}%;background:${gdTone(w.color, 0.9)}"></i></span><span class="tn">${w.kids}${w.drafted ? ` · ${w.drafted} drafted` : ""}</span></div>`).join("") : '<div class="gd-empty">Nobody from here has gone on in this world\'s years.</div>';
  const seasons = x.seasons.map((z) => `<div class="gd-ssn"><span class="tn">${z.year}</span><b class="tn">${z.w}-${z.l}</b>${z.state_title ? '<span class="gd-chip gold">State champs</span>' : z.title ? `<span class="gd-chip">${esc(z.title)}</span>` : ""}</div>`).join("");
  const mine = x.signees.length ? x.signees.slice(0, 12).map((p) => `<div class="gd-ssn">${p.player ? `<a data-go="player" data-id="${p.player}">${esc(p.name)}</a>` : esc(p.name)}<span class="dim">${esc(p.pos)} · class of ${p.class}</span></div>`).join("") : '<div class="gd-empty">You haven\'t signed anyone from here.</div>';
  const moreMine = x.signees.length > 12 ? `<small class="dim">and ${x.signees.length - 12} more.</small>` : "";
  const legends = x.legends.length ? x.legends.map((p) => `<div class="gd-ssn">${esc(p.name)}<span class="dim">${esc(p.pos)} · ${esc(p.what)}</span></div>`).join("") : "";
  const facts = [
    s.big ? "In the state's biggest classification" : "", s.private ? "Private" : "Public", `${s.enrollment.toLocaleString()} students`,
    s.state_titles ? `${s.state_titles} state title${s.state_titles === 1 ? "" : "s"}` : "", x.trip,
  ].filter(Boolean);
  const tape = `Season records back to ${x.first_season || "the start of this world"}. Film online (Hudl) from 2008: before it, a small school's tape was what its coach could send.`;
  return `<div class="sl gd"><div class="gd-room" style="--c1:${esc(c1)};--c2:${esc(c2)}">
    <div class="gd-glass gd-hero"><div class="gd-crest" style="background:linear-gradient(160deg,${esc(c1)},${esc(c2)})">${esc((s.name.match(/[A-Z]/g) || ["?"]).slice(0, 2).join(""))}</div>
      <div><div class="st-label">${esc(x.area.nickname)} · <a data-go="ground" data-args='${esc(JSON.stringify({ state: s.state, area: x.area.code }))}'>${esc(x.area.name)}</a></div><h1>${esc(s.name)}</h1><div class="gd-sub">${esc(s.town)}, ${esc(s.state_name)} · ${facts.map(esc).join(" · ")}</div></div></div>
    <div class="gd-cols"><div class="gd-glass"><h2>The people</h2>${folks}${x.head_since ? `<small class="dim">Head coach since ${x.head_since}.</small>` : ""}</div>
      <div class="gd-glass"><h2>Where its kids went</h2><div class="gd-rows tight">${went}</div><small class="dim">${x.kids} on to college, ${x.drafted} drafted.</small></div>
      <div class="gd-glass"><h2>Its seasons</h2>${seasons || '<div class="gd-empty">No seasons on file.</div>'}<small class="dim">${esc(tape)}</small></div></div>
    <div class="gd-cols two"><div class="gd-glass"><h2>Yours from here</h2>${mine}${moreMine}</div><div class="gd-glass"><h2>Its legends</h2>${legends || '<div class="gd-empty">None the world remembers yet.</div>'}</div></div></div></div>`;
};
const teamLinkById = (w) => `<a data-go="team" data-kind="college" data-id="${w.program}">${esc(w.name)}</a>${w.yours ? " (yours)" : ""}`;

// Layers, areas and the state picker.
Object.assign(CORE_MORE, {
  gdarea: (el) => {
    S.route.args = { ...S.route.args, area: el.dataset.area };
    refresh(el);
  },
  gdlayer: (el) => {
    const L = gdLayers();
    L[el.dataset.k] = !L[el.dataset.k];
    refresh(el);
  },
  gdhome: () => { S.route.args = {}; refresh(); },
});
document.addEventListener("change", (ev) => {
  const el = ev.target.closest && ev.target.closest(".gd-state");
  if (!el) return;
  S.route.args = { state: el.value };
  refresh(el);
});
document.addEventListener("keydown", (ev) => {
  if ((ev.key === "Enter" || ev.key === " ") && ev.target.matches && ev.target.matches(".gd-area")) { ev.preventDefault(); ev.target.dispatchEvent(new MouseEvent("click", { bubbles: true })); }
});

// After the page draws: crowded labels shrink to a name, then hide (the list
// beside lists every area and picks it).
(() => {
  const after0 = afterRender;
  afterRender = function () {
    after0();
    // The map is drawn in its own units and scaled to the window: marks that
    // must stay readable (dots, lines, rival names) are sized by that scale.
    const svg = document.querySelector(".gd-svg");
    if (svg && svg.clientWidth) {
      const k = svg.viewBox.baseVal.width / svg.clientWidth;
      svg.querySelectorAll("circle").forEach((c) => c.setAttribute("r", (c.classList.contains("gd-sig") ? 4.2 : 3.8) * k));
      svg.querySelectorAll(".gd-rv").forEach((e) => { e.style.fontSize = `${11.5 * k}px`; e.style.strokeWidth = `${3 * k}px`; });
      svg.querySelectorAll("line").forEach((e) => { const sel = e.getAttribute("opacity") > 0.9; e.setAttribute("stroke-width", (sel ? 2 : 1.5) * k); e.setAttribute("stroke-dasharray", `${2 * k} ${5 * k}`); });
      svg.querySelector(".gd-selp")?.setAttribute("stroke-width", 2.4 * k);
      svg.querySelector(".gd-bord")?.setAttribute("stroke-width", 1.1 * k);
      svg.querySelector(".gd-edge")?.setAttribute("stroke-width", 1.4 * k);
    }
    const labs = [...document.querySelectorAll(".gd-lab")];
    if (!labs.length) return;
    const placed = [];
    const clash = (r) => placed.some((q) => !(r.right < q.left || r.left > q.right || r.bottom < q.top || r.top > q.bottom));
    for (const l of labs) {
      l.classList.remove("compact", "hid");
      let r = l.getBoundingClientRect();
      if (clash(r) && !l.classList.contains("sel")) {
        l.classList.add("compact");
        r = l.getBoundingClientRect();
        if (clash(r)) { l.classList.add("hid"); continue; }
      }
      placed.push(r);
    }
  };
})();
