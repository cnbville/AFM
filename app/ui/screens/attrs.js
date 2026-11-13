// The player page's Attributes tab (attributes page build, lane 3).
"use strict";
/* The tab draws from one command, `attr_view {id, with?}` (lane 2). By
 * default it shows only the skills in play at his position: the eight
 * Shape families plus an "Also in play" card. "All 67" shows every skill
 * grouped, the unused ones dimmed. Every bar sits on 0-99 with the
 * position's starter and elite marks, and its colour slides smoothly with
 * where the read's middle falls between them. His own read judges the bar,
 * the number, the colour and the arrows wherever he has one; the staff's
 * read stands in only where he has none, and the row says so.
 *
 *   attrTab(p)       the tab (PT.Attributes in core.js calls it)
 *   attrRow          one skill's row: bar, ticks, number, arrow
 *   attrStrip        the four cells on top (or the recruit and compare ones)
 *
 * The page state lives in keep("pattr"): view, coaches' read switch, the
 * second player. The view is also kept in the settings so it comes back
 * after a restart. */

// The gradient, as stops of u = (middle - starter) / (elite - starter):
// grey below a backup, white at the starter mark, red climbing, bright
// burgundy at the elite mark. The light theme swaps white for ink, since a
// white bar would vanish on its white page.
const ATTR_STOPS = {
  dark: [[-0.75, [115, 119, 129]], [-0.4, [170, 173, 181]], [0, [244, 245, 247]], [0.5, [238, 106, 112]], [1, [224, 28, 84]]],
  light: [[-0.75, [170, 174, 183]], [-0.4, [128, 132, 142]], [0, [52, 56, 66]], [0.5, [205, 62, 72]], [1, [168, 10, 62]]],
};
function attrRgb(u) {
  const stops = ATTR_STOPS[document.documentElement.dataset.theme === "light" ? "light" : "dark"];
  if (u <= stops[0][0]) return `rgb(${stops[0][1].join(",")})`;
  const last = stops[stops.length - 1];
  if (u >= last[0]) return `rgb(${last[1].join(",")})`;
  for (let i = 1; i < stops.length; i++) {
    if (u <= stops[i][0]) {
      const [u0, c0] = stops[i - 1], [u1, c1] = stops[i];
      const t = (u - u0) / (u1 - u0);
      return `rgb(${c0.map((v, k) => Math.round(v + (c1[k] - v) * t)).join(",")})`;
    }
  }
  return `rgb(${last[1].join(",")})`;
}
// Where a skill's read falls between his position's starter and elite marks.
function attrU(s) {
  if (s.mid == null || s.starter == null || s.elite == null) return null;
  return (s.mid - s.starter) / Math.max(3, s.elite - s.starter);
}
function attrBandWord(u) {
  return u >= 0.9 ? "Elite" : u >= 0.25 ? "Good starter" : u >= -0.25 ? "Starter level" : u >= -0.65 ? "Short of a starter" : "Backup level";
}
const attrArticle = (pos) => (/^[AEFHILMNORSX]/.test(pos) ? "an " : "a ") + pos;
const attrSpan = (r) => `${r[0]}–${r[1]}`;
const attrPct = (v) => Math.max(0, Math.min(100, v));
// The read judging a skill (his own, or the staff's where he has none).
const attrRead = (s) => (s.judge === "mine" ? s.mine : s.judge === "staff" ? s.staff : null);

// Which way a skill moved since last season, and by how much.
function attrMove(m) {
  if (!m) return null;
  const by = Math.abs(Number(m.by) || 0);
  const d = String(m.dir || "").toLowerCase();
  const sign = /^(up|\+|rose|rise)/.test(d) ? 1 : /^(down|-|fell|fall)/.test(d) ? -1 : Math.sign(Number(m.by) || 0);
  return { sign, by, from: m.from_season };
}
function attrArrow(m) {
  const mv = attrMove(m);
  if (!mv) return "";
  const when = mv.from ? ` since ${mv.from}'s read` : " since last season's read";
  if (!mv.sign || mv.by < 1) return `<span class="at-chg" title="About the same${esc(when)}"><svg class="cg same" viewBox="0 0 10 10" aria-hidden="true"><path d="M2.5 5h5"/></svg></span>`;
  const big = mv.by >= 5;
  const path = mv.sign > 0 ? (big ? "M2 5.2 5 2.2 8 5.2M2 8.6 5 5.6 8 8.6" : "M2 6.5 5 3.5 8 6.5") : big ? "M2 1.4 5 4.4 8 1.4M2 4.8 5 7.8 8 4.8" : "M2 3.5 5 6.5 8 3.5";
  const word = `${mv.sign > 0 ? "Up" : "Down"} ${big ? "clearly" : "a little"}`;
  return `<span class="at-chg" title="${esc(word + when)}"><svg class="cg ${mv.sign > 0 ? "up" : "dn"}" viewBox="0 0 10 10" aria-hidden="true"><path d="${path}"/></svg></span>`;
}
// What a skill is worth to his position, for the hover card.
function attrWeightLine(s, pos) {
  const w = Number(s.weight_pct);
  if (!Number.isFinite(w)) return "";
  const of = `${attrArticle(pos)}'s rating`;
  if (w >= 1) return `Counts about ${Math.round(w)}% of ${of}.`;
  if (w > 0) return `Counts under 1% of ${of}.`;
  return `Doesn't count toward ${of}.`;
}

// One skill's row. `o`: { a: the payload, p: the player, st: page state,
// other: the second player's skills by key (compare), why: a short reason,
// dim: not part of his job }.
function attrRow(s, o) {
  const pos = o.a.pos;
  const staffWord = o.p.mine ? "coaches" : "scouts";
  const why = o.why ? `<small class="at-why">${esc(o.why)}</small>` : "";
  const cls = ["at-row", o.why ? "has-why" : "", o.dim ? "dim" : "", o.other ? "cmp" : ""].filter(Boolean).join(" ");
  const jr = attrRead(s);
  if (!jr) {
    return `<div class="${cls} ns" data-k="${esc(s.key)}"><span class="at-nm">${esc(s.label)}</span><span class="at-tr"><u class="trk"></u></span><span class="at-num nsx">Not seen</span><span class="at-chg"></span>${why}</div>`;
  }
  const [lo, hi] = jr;
  const marks = !o.dim && s.starter != null && s.elite != null;
  const u = attrU(s);
  const color = u == null || o.dim ? "" : attrRgb(u);
  const width = Math.max(2, Math.min(hi - lo, 100 - lo));
  const pool = o.a.pool_words || pos;
  const showStaff = o.st.coach && !o.other && s.judge === "mine" && s.staff;
  let bars = `<u class="trk"></u>`;
  if (marks) bars += `<i class="zone" style="left:${attrPct(s.elite)}%"></i>`;
  bars += `<i class="fill" style="width:${attrPct(lo)}%"></i><i class="pill" style="left:${attrPct(lo)}%;width:${width}%${color ? `;background:${color}` : ""}"></i>`;
  if (marks) bars += `<b class="tk st" style="left:${attrPct(s.starter)}%" title="Starter at ${esc(pool)}: ${s.starter}"></b><b class="tk el" style="left:${attrPct(s.elite)}%" title="Elite, the top tenth of starters: ${s.elite}"></b>`;
  if (showStaff) bars += `<i class="own" style="left:${attrPct(s.staff[0])}%;width:${Math.max(2, Math.min(s.staff[1] - s.staff[0], 100 - s.staff[0]))}%" title="Your ${staffWord} read ${s.staff[0]} to ${s.staff[1]}"></i>`;
  let tail = `<span class="at-chg"></span>`;
  let small = "";
  if (o.other) {
    const t = o.other[s.key];
    const tr = t && attrRead(t);
    if (tr) bars += `<i class="oth" style="left:${attrPct(tr[0])}%;width:${Math.max(2, Math.min(tr[1] - tr[0], 100 - tr[0]))}%" title="${esc(o.st.with.name)}: ${tr[0]} to ${tr[1]}"></i>`;
    const g = tr && s.mid != null && t.mid != null ? Math.round(s.mid - t.mid) : null;
    tail = g == null ? `<span class="at-dl"></span>` : `<span class="at-dl ${g > 0 ? "pos" : ""}">${g > 0 ? "+" : g < 0 ? "−" : ""}${Math.abs(g)}</span>`;
  } else {
    if (s.judge === "staff") small = `<small title="You have no read of your own on this skill yet, so this is your ${staffWord}' read">${staffWord}</small>`;
    else if (showStaff) small = `<small title="Your ${staffWord}' read">${attrSpan(s.staff)}</small>`;
    tail = attrArrow(s.moved) || tail;
  }
  const who = s.judge === "mine" ? "Your read" : `Your ${staffWord}' read`;
  const wl = attrWeightLine(s, pos);
  return `<div class="${cls}" data-k="${esc(s.key)}"><span class="at-nm">${esc(s.label)}</span><span class="at-tr" data-rr="p:${o.p.id}" data-rrk="${esc(s.key)}" data-lo="${lo}" data-hi="${hi}" data-rrw="${esc(who)}" data-wline="${esc(wl)}" tabindex="0">${bars}</span><span class="at-num"${color ? ` style="color:${color}"` : ""}><b>${attrSpan(jr)}</b>${small}</span>${tail}${why}</div>`;
}

// A family's header: its band word and whether it is moving.
function attrBand(skills, st) {
  const us = skills.map(attrU).filter((v) => v != null);
  if (!us.length) return "";
  const u = us.reduce((x, y) => x + y, 0) / us.length;
  const ms = skills.map((s) => attrMove(s.moved)).filter(Boolean);
  const net = ms.length ? ms.reduce((x, m) => x + m.sign * m.by, 0) / ms.length : 0;
  const arrow = ms.length ? attrArrow({ dir: net > 0 ? "up" : net < 0 ? "down" : "same", by: Math.abs(net) }) : "";
  return `<span class="bnd" style="color:${attrRgb(u)}">${attrBandWord(u)}</span>${arrow}`;
}
function attrCard(name, skills, o, extra) {
  const seen = skills.filter(attrRead);
  const partial = seen.length < skills.length && seen.length ? `<span class="sm">${seen.length} of ${skills.length} seen</span>` : "";
  const head = extra != null ? extra : `${partial}${attrBand(seen, o.st)}`;
  return `<section class="at-card"><header><h3>${esc(name)}</h3><span class="hr">${head}</span></header>${skills.map((s) => attrRow(s, o)).join("")}</section>`;
}

// ---------- the strip ----------
const attrIt = (s, tail) => `<a class="at-it" data-act="attrjump" data-k="${esc(s.key)}">${esc(s.label)}</a>${tail ? `<span class="sm">${tail}</span>` : ""}`;
const attrCell = (title, items, empty) => `<div><div class="kick">${esc(title)}</div><div class="v">${items.length ? items.map((x) => `<span class="it">${x}</span>`).join("") : `<span class="sm">${esc(empty)}</span>`}</div></div>`;
function attrPotVal(v) {
  if (!v) return "";
  if (typeof v === "string") return `<b>${esc(v)}</b>`;
  const w = v.words || v.tier || v.word || "";
  const sure = v.sure || v.confidence || "";
  return w ? `<b>${esc(w)}</b>${sure ? ` <span class="sm">· ${esc(words(sure))}</span>` : ""}` : "";
}
function attrPotCell(a, p, recruit) {
  const pot = (a.strip && a.strip.potential) || {};
  let coaches = pot.coaches;
  if (!coaches && p.potential) coaches = { words: p.potential.tier, sure: p.potential.sure };
  const hints = (coaches && coaches.hints) || (p.potential && p.potential.hints) || pot.hints;
  const hint = Array.isArray(hints) ? hints[0] : hints;
  const who = p.mine ? "Your coaches" : "Your scouts";
  const mine = attrPotVal(pot.mine);
  const c = attrPotVal(coaches);
  return `<div class="pot"><div class="kick">What he could become</div>
    <div class="pr"><small>Your read</small>${mine ? `<span>${mine}</span>` : `<span class="sm">Not formed yet${recruit ? " · needs more looks" : ""}</span>`}</div>
    <div class="pr"><small>${who}</small>${c ? `<span class="dim">${c}</span>` : `<span class="sm">No read yet</span>`}</div>
    ${hint ? `<div class="hint">${esc(hint)}</div>` : ""}</div>`;
}
const attrFirst = (arr, n) => arr.slice(0, n);
function attrStrip(a, p, st, inPlay) {
  const seen = inPlay.filter((s) => attrRead(s) && s.starter != null);
  const unseen = inPlay.filter((s) => !attrRead(s));
  const recruit = unseen.length >= Math.max(3, Math.ceil(inPlay.length * 0.2));
  if (recruit) {
    const best = attrFirst([...seen].sort((x, y) => attrU(y) - attrU(x)), 3).map((s) => attrIt(s));
    const rest = unseen.length - 4;
    const nots = attrFirst(unseen, 4).map((s) => attrIt(s));
    if (rest > 0) nots.push(`<span class="sm">and ${rest} more</span>`);
    const wide = [...seen].filter((s) => attrRead(s)[1] - attrRead(s)[0] >= 8).sort((x, y) => (attrRead(y)[1] - attrRead(y)[0]) - (attrRead(x)[1] - attrRead(x)[0]));
    const look = attrFirst(wide, 3).map((s) => attrIt(s, `±${Math.round((attrRead(s)[1] - attrRead(s)[0]) / 2)}`));
    return `<div class="at-brief p4">${attrCell("Best of what you have seen", best, "Nothing seen yet")}${attrCell(`Not seen yet · ${unseen.length} of ${inPlay.length}`, nots, "")}${attrCell("Worth another look", look, "The reads you have are tight")}${attrPotCell(a, p, true)}</div>`;
  }
  const byGap = [...seen].sort((x, y) => (y.mid - y.starter) - (x.mid - x.starter));
  const strong = byGap.filter((s) => s.mid > s.starter).slice(0, 3).map((s) => attrIt(s));
  const short = [...byGap].reverse().filter((s) => s.mid < s.starter).slice(0, 3).map((s) => attrIt(s, `${Math.round(s.starter - s.mid)} under`));
  const moved = inPlay.map((s) => ({ s, m: attrMove(s.moved) })).filter((x) => x.m && x.m.sign && x.m.by >= 1).sort((x, y) => y.m.by - x.m.by).slice(0, 3).map((x) => `${attrIt(x.s)}${attrArrow(x.s.moved)}`);
  const anyMove = inPlay.some((s) => s.moved);
  return `<div class="at-brief p4">${attrCell("Strongest against a starter", strong, "Nothing above a starter yet")}${attrCell("Short of a starter", short, "Nothing short of a starter")}${attrCell("Moved since last season", moved, anyMove ? "Nothing moved clearly" : "No earlier read to compare yet")}${attrPotCell(a, p, false)}</div>`;
}
function attrCompareStrip(a, p, st, view, other) {
  const them = st.with.name;
  const gaps = view.map((s) => { const t = other[s.key]; return t && s.mid != null && t.mid != null ? { s, g: Math.round(s.mid - t.mid) } : null; }).filter(Boolean);
  const ahead = gaps.filter((x) => x.g >= 3).sort((x, y) => y.g - x.g).slice(0, 3).map((x) => attrIt(x.s, `+${x.g}`));
  const behind = gaps.filter((x) => x.g <= -3).sort((x, y) => x.g - y.g).slice(0, 3).map((x) => attrIt(x.s, `−${Math.abs(x.g)}`));
  const level = gaps.filter((x) => Math.abs(x.g) <= 2).sort((x, y) => Math.abs(x.g) - Math.abs(y.g)).slice(0, 3).map((x) => attrIt(x.s));
  return `<div class="at-brief">${attrCell(`Ahead of ${them}`, ahead, "Nothing clearly ahead")}${attrCell(`Behind ${them}`, behind, "Nothing clearly behind")}${attrCell(`Level with ${them}`, level, "Nothing level")}</div>`;
}

// ---------- the tab ----------
const attrLegendGrad = (lo, hi, mid) => `<i class="gb" style="background:linear-gradient(90deg,${[lo, mid, hi].filter(Boolean).join(",")})"></i>`;
async function attrTab(p) {
  const st = keep("pattr", {});
  if (!st.view) st.view = prefs.attrView === "all" ? "all" : "play";
  if (st.coach === undefined) st.coach = true;
  if (st.with && st.with.for !== p.id) st.with = null;
  let a, b = null;
  try {
    [a, b] = await Promise.all([api("attr_view", { id: p.id, ...(st.with ? { with: st.with.id } : {}) }), st.with ? api("attr_view", { id: st.with.id }) : null]);
  } catch (e) {
    st.with = null;
    return `<div class="panel"><div class="empty">The attributes couldn't be read just now: ${esc(e.message || String(e))}</div></div>`;
  }
  if (a.with && a.with.name && st.with) st.with.name = a.with.name;
  const byKey = Object.fromEntries(a.skills.map((s) => [s.key, s]));
  const other = b ? Object.fromEntries(b.skills.map((s) => [s.key, s])) : null;
  const o = { a, p, st, other };
  const staffWord = p.mine ? "coaches" : "scouts";
  const inPlay = a.skills.filter((s) => s.in_play);
  const all = st.view === "all";
  const pos = a.pos;

  const seg = `<div class="seg"><button data-act="attrview" data-v="play" aria-pressed="${!all}">In play at ${esc(pos)}<small>${inPlay.length}</small></button><button data-act="attrview" data-v="all" aria-pressed="${all}">All 67</button></div>`;
  const cmpBtn = `<button class="tbtn${st.with ? " on" : ""}" data-act="attrwith" data-id="${p.id}">${st.with ? `Compare with ${esc(st.with.name)} ×` : "Compare with…"}</button>`;
  const sw = st.with ? "" : `<button class="at-tgl" role="switch" aria-checked="${st.coach}" data-act="attrcoach" title="Show your ${staffWord}' read beside your own"><span class="sw${st.coach ? " on" : ""}"></span>${p.mine ? "Coaches" : "Scouts"}' read</button>`;
  const grad = [attrRgb(-0.75), attrRgb(-0.4), attrRgb(0)], grad2 = [attrRgb(0), attrRgb(1)];
  const legend = `<div class="at-lg"><div class="lgr">${st.with ? `<span><i class="p"></i>${esc(p.name.split(" ").slice(-1)[0])}</span><span><i class="m"></i>${esc(st.with.name.split(" ").slice(-1)[0])}</span>` : `<span><i class="p"></i>Your read</span><span><i class="o"></i>Your ${staffWord}' read</span>`}<span><i class="s"></i>Starter mark</span><span><i class="e"></i>Elite mark</span></div><div class="lgr"><span>Below a backup${attrLegendGrad(grad[0], grad[2], grad[1])}Starter${attrLegendGrad(grad2[0], grad2[1])}Elite</span></div></div>`;
  const tb = `<div class="at-tb">${seg}${cmpBtn}${sw}${legend}</div>`;

  let strip = "";
  if (!all) strip = st.with && other ? attrCompareStrip(a, p, st, inPlay, other) : attrStrip(a, p, st, inPlay);
  else if (st.with && other) strip = attrCompareStrip(a, p, st, a.skills, other);

  let cards = "";
  if (!all) {
    const dv = a.default_view || { families: [], also: [] };
    cards = dv.families.map((f) => attrCard(f.name, f.skills.map((k) => byKey[k]).filter(Boolean), o)).join("");
    if (dv.also && dv.also.length) {
      const rows = dv.also.map((x) => byKey[x.key] && { ...byKey[x.key], why: x.why }).filter(Boolean);
      cards += `<section class="at-card"><header><h3>Also in play</h3><span class="hr"><span class="bnd faint">read in games, not his main job</span></span></header>${rows.map((s) => attrRow(s, { ...o, why: s.why })).join("")}</section>`;
    }
  } else {
    const groups = [];
    for (const s of a.skills) {
      let g = groups.find((x) => x.name === s.group);
      if (!g) groups.push((g = { name: s.group, skills: [] }));
      g.skills.push(s);
    }
    cards = groups.map((g) => {
      const on = g.skills.filter((s) => s.in_play), off = g.skills.filter((s) => !s.in_play);
      const rows = on.map((s) => attrRow(s, o)).join("") + (off.length ? `${on.length ? '<div class="at-sep">Not part of his job</div>' : ""}${off.map((s) => attrRow(s, { ...o, dim: true })).join("")}` : "");
      return `<section class="at-card"><header><h3>${esc(g.name)}</h3><span class="hr"><span class="bnd faint">${on.length} of ${g.skills.length} in play</span></span></header>${rows}</section>`;
    }).join("");
  }
  const out = a.skills.length - inPlay.length;
  const foot = `<div class="at-foot"><span>Filled bars are your own read where you have one, and your ${staffWord}' where you don't; outlined bars are your ${staffWord}'. A narrow bar is a sure read; nobody sees a true number.</span>${all ? "" : `<span>${out} skills are not part of ${esc(attrArticle(pos))}'s job · <a class="link" data-act="attrview" data-v="all">see All 67</a></span>`}</div>`;
  if (!a.skills.some(attrRead)) {
    return `<div class="at">${tb}<div class="panel"><div class="empty">${p.mine ? "Your position coach hasn't graded him yet." : "Nobody on your staff has seen his skills yet. Put him on your list and your scouts will keep looking."}</div></div></div>`;
  }
  return `<div class="at">${tb}${strip}<div class="at-cols">${cards}</div>${foot}</div>`;
}

Object.assign(CORE_MORE, {
  attrview: (el) => {
    const st = keep("pattr", {});
    st.view = el.dataset.v === "all" ? "all" : "play";
    prefs.attrView = st.view;
    savePrefs();
    refresh(el);
  },
  attrcoach: (el) => { const st = keep("pattr", {}); st.coach = !st.coach; refresh(el); },
  attrjump: (el) => {
    const row = document.querySelector(`.at-row[data-k="${CSS.escape(el.dataset.k)}"]`);
    if (!row) return;
    row.scrollIntoView({ block: "center", behavior: prefs.motion === "reduce" ? "auto" : "smooth" });
    row.classList.add("flash");
    setTimeout(() => row.classList.remove("flash"), 1400);
  },
  // Compare opens a list of your own players; pressing it again lets go.
  attrwith: async (el) => {
    const st = keep("pattr", {});
    const id = Number(el.dataset.id);
    if (st.with) { st.with = null; refresh(el); return; }
    let x;
    try { x = await api("my_players"); } catch (e) { toast(e.message || String(e)); return; }
    const picked = new Set(keep("cmp", { ids: [] }).ids);
    const rows = x.rows.filter((r) => r.id !== id).sort((u, v) => (picked.has(v.id) - picked.has(u.id)) || String(u.name).localeCompare(String(v.name)));
    if (!rows.length) { toast("Put a few players on your list first, then compare them here."); return; }
    const names = new Map(rows.map((r) => [String(r.id), r.name]));
    moveMenu(el, "Compare with", rows.slice(0, 60).map((r) => [String(r.id), `${r.name} · ${r.pos}${r.team ? " · " + r.team.name : ""}`]), (v) => {
      st.with = { for: id, id: Number(v), name: names.get(v) };
      refresh(el);
    });
  },
});
