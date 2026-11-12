// The position depth overview (Cain, 2026-11-12). At the start of a save a
// coach checks each group's depth for the next falls by hand, on the Depth
// chart, and picks the position again on every recruiting page. This puts
// one strip of the nine groups, the four offseason steps and a hover card
// with the exact players on the Recruiting board, Find recruits and the
// Transfer portal.
//
// It adds no data and works nothing out twice. The words in a cell and the
// players in its card are the Needs map's own (sb_needs: its verdicts, its
// starters, its list of men by fall); the step counts are the desks' own
// (recruit_board's class, portal_desk's roster and targets, board_cap's
// roster places); "If he goes" is one sort and one subtraction on the Needs
// map's reads. Everything is fetched after the page draws, so a page opens
// as fast as it did before; the strip fills in a moment later.
"use strict";

// The nine groups, as the Squad's rail has them.
const DV_GROUPS = [
  ["QB", "Quarterbacks", ["QB"]],
  ["RB", "Running backs", ["RB"]],
  ["WR", "Wide receivers", ["WR"]],
  ["TE", "Tight ends", ["TE"]],
  ["OL", "Offensive line", ["OT", "IOL"]],
  ["DL", "Defensive line", ["EDGE", "DT"]],
  ["LB", "Linebackers", ["LB"]],
  ["DB", "Defensive backs", ["CB", "S"]],
  ["ST", "Specialists", ["K", "P", "LS"]],
];
const DV_RANK = { Hole: 0, Thin: 1, Average: 2, Solid: 3, Strength: 4 };
const DV_TTL = 10000;
// sel: the pick every page follows (g a group key or "ALL"; p one position
// inside the group; raw a page's own group such as Offense). seen: the
// string each page last showed, to tell a pick made on the page itself.
const DV = { sel: { g: "ALL", p: null, raw: null }, seen: {}, data: {}, at: {}, st: {}, portalPage: null };

const dvGroupOf = (pos) => DV_GROUPS.find((g) => g[2].includes(pos));
const dvGroupKey = (k) => DV_GROUPS.find((g) => g[0] === k);
const dvStamp = () => (S.status && S.status.today) || "";

// ---------- the one position pick ----------
// A page's own position string as the shared pick. The board keeps a group
// as "g:OL"; Find and the portal take OL, DL, DB and Specialists.
function dvFromPage(v) {
  if (!v || v === "ALL") return { g: "ALL", p: null, raw: null };
  if (v.startsWith("g:")) return { g: v.slice(2), p: null, raw: null };
  const g = dvGroupOf(v);
  if (g) return { g: g[0], p: g[2].length > 1 ? v : null, raw: null };
  const named = { OL: "OL", DL: "DL", DB: "DB", Specialists: "ST" }[v];
  return named ? { g: named, p: null, raw: null } : { g: "ALL", p: null, raw: v };
}
// The shared pick as a page's own string.
function dvToPage(page, s) {
  if (s.g === "ALL") return page === "find" && s.raw ? s.raw : "ALL";
  if (s.p) return s.p;
  const g = dvGroupKey(s.g);
  if (!g) return "ALL";
  if (g[2].length === 1) return g[2][0];
  if (page === "board") return `g:${s.g}`;
  return s.g === "ST" ? "Specialists" : s.g;
}
// Called as a page draws, with the position it holds. A pick made on the
// page itself (its own menu, a saved search, a plan link) becomes the shared
// pick; otherwise the shared pick, made on the strip or another page, is
// written into the page. Returns what the page shows.
function dvSync(page, cur) {
  const seen = DV.seen[page];
  const own = seen !== undefined ? cur !== seen : DV.sel.g === "ALL" && !DV.sel.raw && cur && cur !== "ALL";
  if (own) DV.sel = dvFromPage(cur);
  const want = dvToPage(page, DV.sel);
  DV.seen[page] = want;
  return want;
}
// The positions a page's pick covers (the board's filter).
function dvPosHas(v, pos) {
  if (!v || v === "ALL") return true;
  if (v.startsWith("g:")) {
    const g = dvGroupKey(v.slice(2));
    return !!g && g[2].includes(pos);
  }
  return v === pos;
}

// ---------- what the screens already know ----------
// A page hands over its own answer so nothing is asked twice.
function dvOwn(key, x) {
  DV.data[key] = x;
  DV.at[key] = Date.now();
  DV.st[key] = dvStamp();
}
// What each piece is asked from. The portal's counts need no entrant rows: no
// entrant matches this name, so none are built.
const DV_ASK = {
  needs: ["sb_needs", { year: 0 }],
  board: ["recruit_board", {}],
  portal: ["portal_desk", { pos: "NONE" }],
  cap: ["board_cap", {}],
};
const DV_LIVE = {};
function dvLoad(key) {
  if (DV.data[key] && DV.st[key] === dvStamp() && Date.now() - DV.at[key] < DV_TTL) return Promise.resolve();
  if (!DV_LIVE[key]) {
    const [cmd, args] = DV_ASK[key];
    DV_LIVE[key] = api(cmd, args).then((x) => dvOwn(key, x)).catch(() => {}).finally(() => { delete DV_LIVE[key]; });
  }
  return DV_LIVE[key];
}
const dvFetch = (keys = Object.keys(DV_ASK)) => Promise.all(keys.map(dvLoad));
// After each of the three pages draws (core.js, afterRender): fetch what is
// missing, then fill the strip; a page that was left meanwhile is skipped.
async function dvAfter() {
  const boxes = [...document.querySelectorAll(".dv[data-dv-page]")];
  if (!boxes.length) return;
  await dvFetch();
  for (const box of boxes) {
    if (!box.isConnected) continue;
    // The strip is a help, never the page: a surprise in its data leaves the page as it was.
    try {
      const html = dvInner(box.dataset.dvPage);
      if (box.innerHTML !== html) box.innerHTML = html;
    } catch (e) {
      console.error(e);
    }
  }
}
function dvBlock(page) {
  return `<div class="dv" data-dv-page="${page}">${dvInner(page)}</div>`;
}
function dvInner(page) {
  const x = DV.data.needs;
  if (!x || !x.spots) return '<div class="dv-skel" aria-hidden="true"></div>';
  return dvSteps(page) + dvStrip(page, x);
}

// ---------- the words of a cell ----------
// Where a group is weakest over the three falls, in the Needs map's own
// words: the first fall it reads Thin and the first it reads Hole.
function dvWords(x, grp) {
  const spots = x.spots.filter((s) => grp[2].includes(s.pos));
  if (!spots.length) return { text: "–", low: false, worst: [2, 2, 2], spots };
  const yy = (n) => `’${String(x.falls[n]).slice(2)}`;
  const worst = [0, 1, 2].map((n) => Math.min(...spots.map((s) => DV_RANK[s.verdicts[n]] ?? 2)));
  const parts = [];
  let floor = 2;
  worst.forEach((r, n) => {
    if (r <= 1 && r < floor) { parts.push(`${r === 0 ? "Hole" : "Thin"} ${yy(n)}`); floor = r; }
  });
  if (parts.length) return { text: parts.map((p, i) => (i ? p[0].toLowerCase() + p.slice(1) : p)).join(", "), low: true, worst, spots };
  return { text: `${Math.min(...worst) >= 3 ? "Solid" : "No gaps"} to ${yy(2)}`, low: false, worst, spots };
}
const dvGroupFull = (grp) => grp[1];

// The page's own number under the words.
function dvNote(page, grp) {
  const d = DV.data;
  if (page === "portal") {
    const mine = d.portal && d.portal.roster ? d.portal.roster.filter((r) => grp[2].includes(r.pos)) : null;
    if (!mine) return "";
    const left = mine.filter((r) => r.in_portal).length;
    if (left) return `<small class="low">${left} of yours in the portal</small>`;
    const restless = mine.filter((r) => r.rank === 1 || r.rank === 2).length;
    return restless ? `<small>${restless} restless</small>` : '<small class="dim">–</small>';
  }
  const bp = d.board && d.board.class ? d.board.class.by_pos.filter((p) => grp[2].includes(p.pos)) : null;
  if (!bp) return "";
  const open = bp.reduce((a, p) => a + p.open, 0);
  return `<small>${open ? `${open} open` : "none open"}</small>`;
}

function dvStrip(page, x) {
  const cells = DV_GROUPS.map((grp) => {
    const w = dvWords(x, grp);
    const on = DV.sel.g === grp[0];
    return `<button type="button" class="dv-cell ${w.low ? "low" : ""}" data-dv="cell" data-g="${grp[0]}" data-peek="depth" aria-pressed="${on}" aria-label="${esc(dvGroupFull(grp))}: ${esc(w.text)}"><b>${grp[0]}</b><span class="dv-w">${esc(w.text)}</span>${dvNote(page, grp)}</button>`;
  }).join("");
  return `<div class="dv-strip" role="group" aria-label="Depth by position group">${cells}</div>`;
}

// ---------- the four steps ----------
function dvCurrentStep(page) {
  if (page !== "portal") return 4;
  const k = S.keep.fcp;
  return k && k.tab === "roster" ? 2 : 3;
}
function dvSteps(page) {
  const d = DV.data;
  const cur = dvCurrentStep(page);
  const cap = d.cap && d.cap.scholarships;
  const steps = [];
  // 1. Roster cuts: the roster places used (the NIL desk's own count).
  if (cap) {
    const n = cap.whole ? cap.counted : cap.roster;
    const lim = cap.whole ? cap.cap : cap.limit;
    const main = cap.whole ? `${n} of ${lim}` : lim != null ? `${cap.onAid} of ${lim} on aid` : `${n} on the roster`;
    const over = lim != null && (cap.whole ? n : cap.onAid) > lim;
    const note = over ? `${(cap.whole ? n : cap.onAid) - lim} over` : lim != null ? `${lim - (cap.whole ? n : cap.onAid)} to spare` : "";
    steps.push({ n: 1, t: "Roster cuts", main, note, low: over });
  } else steps.push({ n: 1, t: "Roster cuts", main: "Your squad", note: "" });
  // 2 and 3: the portal desk's roster and targets.
  const p = d.portal;
  if (p && p.roster) {
    const left = p.roster.filter((r) => r.in_portal).length;
    const restless = p.roster.filter((r) => !r.in_portal && (r.rank === 1 || r.rank === 2)).length;
    steps.push({ n: 2, t: "Leavers", main: left ? `${left} in the portal` : "Nobody in the portal", note: restless ? `${restless} more restless` : "Nobody restless", low: restless > 0 });
    const live = (p.targets || []).filter((t) => !t.settled);
    const offered = live.filter((t) => t.offered).length;
    steps.push({ n: 3, t: "Portal targets", main: `${offered} offered`, note: `${live.length} on your list`, low: live.length > offered });
  } else {
    steps.push({ n: 2, t: "Leavers", main: "Your roster", note: "" });
    steps.push({ n: 3, t: "Portal targets", main: "Your list", note: "" });
  }
  // 4. The class: the board's own commitments and places.
  const c = d.board && d.board.class;
  if (c) {
    const left = Math.max(0, c.slots - c.commits);
    steps.push({ n: 4, t: `Class of ${c.year}`, main: `${c.commits + c.signed} in`, note: `${left} ${left === 1 ? "place" : "places"} left`, low: left > 0 });
  } else steps.push({ n: 4, t: "The class", main: "Your board", note: "" });
  const html = steps.map((s) => `<button type="button" class="dv-step ${s.n === cur ? "on" : ""}" data-dv="step" data-n="${s.n}" ${s.n === cur ? 'aria-current="step"' : ""}><i>${s.n}</i><span><b>${esc(s.t)}</b><small>${esc(s.main)}</small>${s.note ? `<small class="${s.low ? "low" : "dim"}">${esc(s.note)}</small>` : ""}</span></button>`).join("");
  return `<div class="dv-steps" role="group" aria-label="The four offseason steps">${html}</div>`;
}

// ---------- the hover card ----------
// Every man in the group by fall, bold where he starts, with who leaves, who
// is in the portal and who arrives, and what each starter's exit would cost.
function dvCard(g) {
  const d = DV.data, x = d.needs, grp = dvGroupKey(g);
  if (!x || !grp) return "";
  const yy = (n) => `’${String(x.falls[n]).slice(2)}`;
  const w = dvWords(x, grp);
  const men = (x.men || []).filter((m) => grp[2].includes(m.pos));
  const spotAt = new Map(x.spots.map((s, i) => [s.key, i]));
  // A spot in the map's own words ("Left guard"), never its letters.
  const spotName = (k) => ((x.spots.find((s) => s.key === k) || {}).full || String(k).replace(/\d$/, "")).toLowerCase();
  const start = (m) => m.spots.findIndex((s) => s);
  const lastCount = new Map();
  men.forEach((m) => { const l = dvLast(m); lastCount.set(l, (lastCount.get(l) || 0) + 1); });
  const who = (m) => (lastCount.get(dvLast(m)) > 1 ? `${String(m.name)[0]}. ${dvLast(m)}` : dvLast(m));
  const shown = men
    .filter((m) => start(m) >= 0 || m.portal || m.commit)
    .sort((a, b) => (start(a) < 0 ? 9 : start(a)) - (start(b) < 0 ? 9 : start(b))
      || (spotAt.get(a.spots[start(a)]) ?? 99) - (spotAt.get(b.spots[start(b)]) ?? 99)
      || Math.max(...b.vals.map((v) => v ?? 0)) - Math.max(...a.vals.map((v) => v ?? 0)));
  const rowsMax = 14;
  const rows = shown.slice(0, rowsMax);
  const more = men.length - rows.length;
  const stayed = new Map((x.portal || []).map((q) => [q.id, q]));
  const cell = (m, n) => {
    const v = m.vals[n];
    if (v == null) return `<td class="dv-v${m.portal ? " low" : ""}">–</td>`;
    return `<td class="dv-v${m.spots[n] ? " on" : ""}">${v}</td>`;
  };
  const tag = (m) => {
    if (m.portal) return `<span class="low">in the portal${stayed.get(m.id) && stayed.get(m.id).on_hold ? ", plays on through the playoffs" : ""}</span>`;
    if (m.commit) { const f = m.vals.findIndex((v) => v != null); return `${f >= 0 ? x.falls[f] + " " : ""}commit`; }
    if (m.gone && m.gone.at >= 1) return `${m.draft_risk ? "likely to declare" : "leaves"} after ${yy(m.gone.at - 1)}`;
    return "";
  };
  const goes = (m) => {
    if (m.portal) {
      const q = stayed.get(m.id), s = q && q.stayed ? q.stayed.find((z) => z && z.role === "Starter") : null;
      const any = q && q.stayed ? q.stayed.find((z) => z) : null;
      const yr = (z) => String(z.fall > 100 ? z.fall : x.falls[z.fall]).slice(2);
      return s ? `<span class="low">would start at ${esc(spotName(s.spot))} in ’${yr(s)}</span>` : any ? `<span class="low">would be ${esc(String(any.words || any.role || "").replace(/^Starter at /, "a starter at ").toLowerCase())}</span>` : "";
    }
    const n = start(m);
    if (n < 0 || m.vals.filter((v) => v != null).length < 2) return "";
    const next = men.filter((q) => q !== m && q.pos === m.pos && !q.portal && q.vals[n] != null && !q.spots[n]).sort((a, b) => b.vals[n] - a.vals[n])[0];
    if (!next) return `<span class="low" title="Nobody else at ${esc(m.pos)} is ready in ${yy(n)}">nobody behind him</span>`;
    const drop = m.vals[n] - next.vals[n];
    return `<span title="In ${yy(n)} the next man up is ${esc(next.name)}, ${next.vals[n]}">${drop > 0 ? `−${drop}` : "no drop"} · ${esc(who(next))} ${next.vals[n]}</span>`;
  };
  const body = rows.map((m) => {
    const n = start(m);
    const place = n >= 0 ? spotName(m.spots[n]) : m.pos;
    const small = [place, m.commit ? "" : m.classes[Math.max(0, n)] || m.classes[0], tag(m)].filter(Boolean).join(" · ");
    return `<tr class="dv-r${m.portal ? " gone" : ""}"><td class="dv-who">${playerLink(m.id, who(m))}<small>${small}</small></td>${[0, 1, 2].map((i) => cell(m, i)).join("")}<td class="dv-go">${goes(m)}</td></tr>`;
  }).join("");
  const holes = w.spots.map((s) => ({ s, at: [0, 1, 2].filter((n) => s.names[n] === null) })).filter((h) => h.at.length);
  const holeRows = holes.map((h) => `<tr class="dv-r"><td class="dv-who low">Nobody <small>${esc(spotName(h.s.key))} in ${h.at.map(yy).join(", ")}</small></td>${[0, 1, 2].map((n) => `<td class="dv-v low">${h.at.includes(n) ? "hole" : ""}</td>`).join("")}<td></td></tr>`).join("");
  // Why it reads weak: each weak spot's first weak fall, in the map's own names.
  const why = [];
  for (const s of w.spots) {
    for (let n = 0; n < 3; n++) {
      const v = s.verdicts[n];
      if (!(v === "Hole" || v === "Thin") || (n > 0 && (DV_RANK[s.verdicts[n - 1]] ?? 2) <= DV_RANK[v])) continue;
      const nm = s.names[n], prev = n > 0 ? s.names[n - 1] : null;
      const key = spotName(s.key);
      if (!nm) why.push(`${yy(n)} ${key}: ${prev ? `nobody behind ${dvLast(prev)}` : "nobody plays here now"}.`);
      else why.push(`${yy(n)} ${key}: ${v.toLowerCase()}, ${dvLast(nm)} ${s.vals[n]}${prev && prev.id !== nm.id ? ` steps in for ${dvLast(prev)}` : ""}.`);
    }
  }
  const whyHtml = why.length ? `<div class="dv-why"><span class="dv-k low">Why it reads weak</span><p>${why.slice(0, 5).map(esc).join(" ")}${why.length > 5 ? ` ${why.length - 5} more in the Needs map.` : ""}</p></div>` : "";
  // The class plan and the portal's own numbers, where the page has them.
  const foot = [];
  const bp = d.board && d.board.class ? d.board.class.by_pos.filter((p) => grp[2].includes(p.pos)) : null;
  if (bp) {
    const sum = (k) => bp.reduce((a, p) => a + p[k], 0);
    const open = sum("open");
    foot.push(`Class plan for ${g}: chase ${sum("need")} · committed ${sum("committed") + sum("signed")} · <b>open ${open}</b>${bp.length > 1 && open ? ` (${bp.filter((p) => p.open).map((p) => `${esc(p.pos)} ${p.open}`).join(", ")})` : ""}`);
  }
  // On the portal page itself, the entrants at this group when the page is on it.
  const pp = DV.portalPage;
  if (S.route.screen === "portal_desk" && pp && pp.div === "ALL" && pp.pos === dvToPage("portal", { g, p: null, raw: null })) foot.push(`Entrants at ${g}: <b>${pp.shown}</b> · see them in the Entrants tab`);
  const note = !x.men ? '<p class="dv-note">The Needs map\'s list of players isn\'t in this game\'s answer yet.</p>' : "";
  const picked = DV.sel.g === g && !DV.sel.p;
  return `<div class="dv-card"><div class="dv-ch"><b>${esc(g)} · ${men.length} ${men.length === 1 ? "player" : "players"}</b><span class="${w.low ? "low" : ""}">${esc(w.text)}</span></div>
    ${note}<table class="dv-t"><thead><tr><th>Depth by fall · bold starts</th>${[0, 1, 2].map((n) => `<th>${yy(n)}</th>`).join("")}<th>If he goes</th></tr></thead><tbody>${body}${holeRows}</tbody></table>
    ${more > 0 ? `<div class="dv-more">+ ${more} more behind them</div>` : ""}${whyHtml}
    <div class="dv-foot">${foot.map((f) => `<span>${f}</span>`).join("")}<span><b>Click</b> ${picked ? "clears the filter" : `filters this page to ${esc(g)}`} · <b>Shift-click</b> opens the Needs map beside it</span></div></div>`;
}
const dvLast = (m) => String(m.name).split(" ").slice(1).join(" ") || String(m.name);

// ---------- clicks ----------
// The weakest spot of a group, to open the Needs map on it.
function dvOpenNeeds(g) {
  const x = DV.data.needs, grp = dvGroupKey(g);
  if (x && grp) {
    let best = null;
    for (const s of x.spots.filter((q) => grp[2].includes(q.pos))) {
      for (let n = 0; n < 3; n++) {
        const r = DV_RANK[s.verdicts[n]] ?? 2;
        if (!best || r < best.r) best = { r, k: s.key, n };
      }
    }
    if (best) { keep("sbneeds", { k: "QB" }).k = best.k; keep("sbneedy", { y: 0 }).y = best.n; }
  }
  openBeside("needs", {});
}
function dvStep(n) {
  if (n === 1) return go("roster");
  if (n === 2 || n === 3) {
    const k = keep("fcp", { tab: "entrants", pos: "ALL", div: "ALL" });
    k.tab = n === 2 ? "roster" : "targets";
    // The portal opens on your roster the first time you visit; a step you pick is your choice of tab.
    k.seen = true;
    return S.route.screen === "portal_desk" ? render($(".view") ? $(".view").scrollTop : 0) : go("portal_desk");
  }
  return go("recruit_board", { tab: "board" });
}
document.addEventListener("click", (ev) => {
  const link = ev.target.closest && ev.target.closest(".dv-card a[data-go]");
  if (link) { peekHide(); return; }
  const el = ev.target.closest && ev.target.closest("[data-dv]");
  if (!el) return;
  const d = el.dataset;
  peekHide();
  if (d.dv === "cell") {
    if (ev.shiftKey) return dvOpenNeeds(d.g);
    DV.sel = DV.sel.g === d.g && !DV.sel.p ? { g: "ALL", p: null, raw: null } : { g: d.g, p: null, raw: null };
    // On the board the position filter lives on the Full board tab.
    if (DV.sel.g !== "ALL" && S.route.screen === "recruit_board") keep("fcb", {}).tab = "board";
    render($(".view") ? $(".view").scrollTop : 0);
  } else if (d.dv === "step") dvStep(Number(d.n));
  else if (d.dv === "clear") {
    DV.sel = { g: "ALL", p: null, raw: null };
    render($(".view") ? $(".view").scrollTop : 0);
  }
});
// The pick the strip made, shown beside a page's own position filter.
function dvPickPill() {
  const s = DV.sel;
  if (s.g === "ALL") return "";
  const grp = dvGroupKey(s.g);
  return `<span class="dv-pick">${esc(grp ? grp[1] : s.g)}${s.p ? ` · ${esc(s.p)}` : ""} (from the strip) <button type="button" data-dv="clear" aria-label="Clear the position pick">×</button></span>`;
}
