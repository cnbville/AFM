// AFM World Reader: the screens (lane R5). One page, plain JS. Every page is
// a list of blocks from the reader's `page` command; this file draws them,
// and keeps the sidebar, search, filters, the year slider, hover cards and
// "Copy for Claude" on every screen.
"use strict";

const TAURI = window.__TAURI__;
async function api(cmd, args) {
  if (TAURI && TAURI.core) return TAURI.core.invoke("call", { cmd, args: args === undefined ? null : args });
  const r = await fetch("/api/" + cmd, { method: "POST", body: JSON.stringify(args === undefined ? null : args) });
  const j = await r.json();
  if ("err" in j) throw new Error(j.err);
  return j.ok;
}

const $ = (s, el) => (el || document).querySelector(s);
const esc = (s) => String(s == null ? "" : s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]);
const store = {
  get(k, d) { try { const v = localStorage.getItem("afmr." + k); return v == null ? d : JSON.parse(v); } catch (e) { return d; } },
  set(k, v) { try { localStorage.setItem("afmr." + k, JSON.stringify(v)); } catch (e) { /* private window */ } },
};

let ST = null;           // the reader's status: the open book, nav, filter values
let PAGE = null;         // the page on screen
let F = store.get("filters", {});
let H = [];              // hover cards of the page on screen

const FILTERS = [["level", "Level"], ["division", "Division"], ["conference", "Conference"], ["position", "Position"], ["stars", "Stars"], ["state", "State"]];
const MARK_WORD = { near: "Near", off: "Off", far: "Far off", debug: "Debug", check: "Check", none: "No data", closer: "Closer", further: "Further", same: "Same" };

function fmt(v) {
  if (v == null || v === "") return "–";
  if (typeof v === "number") {
    if (Number.isInteger(v)) return v.toLocaleString("en-US");
    const a = Math.abs(v);
    return a >= 1000 ? Math.round(v).toLocaleString("en-US") : a >= 100 ? v.toFixed(0) : a >= 10 ? v.toFixed(1) : a >= 1 ? v.toFixed(2) : v.toFixed(3);
  }
  return String(v);
}
const mark = (m) => (m && MARK_WORD[m] ? `<span class="mk ${m === "debug" ? "dbg" : m}" data-h="${hv({ what: MARK_WORD[m], rule: PAGE && PAGE.rule })}">${MARK_WORD[m]}</span>` : "");
const hv = (h) => { if (!h) return ""; H.push(h); return H.length - 1; };
const route = () => (location.hash.replace(/^#\/?/, "") || "overview");
function parseRoute() {
  const [path, qs] = route().split("?");
  const args = {};
  new URLSearchParams(qs || "").forEach((v, k) => { args[k] = /^-?\d+$/.test(v) ? Number(v) : v; });
  return { path, args };
}
const go = (r) => { location.hash = "#/" + r; };

function toast(t) {
  const el = $("#toast");
  el.textContent = t;
  el.style.display = "block";
  clearTimeout(toast.t);
  toast.t = setTimeout(() => (el.style.display = "none"), 2200);
}

// ---------- The sidebar ----------
async function refresh() {
  ST = await api("status");
  drawSide();
  drawFilters();
}

async function drawSide() {
  const hb = $("#hidden");
  hb.setAttribute("aria-pressed", ST.hidden ? "true" : "false");
  $("#hidden-sub").textContent = ST.hidden ? "True potential, ceilings and hidden ratings show, marked debug." : "Off: nothing hidden in the game shows. Use it for a world you play.";
  const nav = $("#nav");
  if (!ST.open) {
    nav.innerHTML = `<div class="lbl" style="padding: 0 12px 6px">Pages</div><span class="sub" style="padding: 0 12px">Open a world to read it.</span>`;
  } else {
    const r = route().split("?")[0];
    const on = (slug) => (r === slug || r.startsWith(slug + "/") || r === "page/" + slug || (slug === "overview" && r === "overview")) ? "on" : "";
    nav.innerHTML = `<div class="lbl" style="padding: 0 12px 6px">Pages</div>` + ST.nav.map((n) => {
      const href = n.slug === "overview" ? "overview" : n.slug === "tools" ? "tools/issues" : n.slug === "compare" ? "compare/worlds" : "page/" + n.slug;
      const m = n.marks || {};
      const chips = m.far ? `<span class="mk far">${m.far} far off</span>` : m.off ? `<span class="mk off">${m.off} off</span>` : n.odd ? `<span class="mk off">${n.odd} odd</span>` : "";
      return `<a class="${on(n.slug)}" href="#/${href}" title="${esc(n.codes || "")}">${esc(n.title)} <span class="marks">${chips}</span></a>`;
    }).join("");
  }
  try {
    const lib = await api("library");
    const cur = ST.open ? ST.path : "";
    // A book opened from elsewhere on disk shows too, at the top.
    if (cur && !lib.books.some((b) => b.path === cur)) lib.books.unshift({ path: cur, name: cur.split(/[\\/]/).pop().replace(/\.afmbook$/, ""), mb: ST.mb });
    $("#worlds").innerHTML = lib.books.length
      ? lib.books.map((b) => `<a class="world ${b.path === cur ? "on" : ""}" data-open="${esc(b.path)}"><div class="t">${esc(b.name)}</div><div class="sub">${b.gb >= 0.1 ? b.gb + " GB book" : b.mb != null ? b.mb + " MB book" : "Book"}</div></a>`).join("")
      : `<span class="sub" style="padding: 0 12px">No books on this Mac yet.</span>`;
  } catch (e) { $("#worlds").innerHTML = `<span class="sub">${esc(e.message || e)}</span>`; }
}

// ---------- Filters and the year slider ----------
function drawFilters() {
  const box = $("#filters");
  if (!ST || !ST.open) { box.hidden = true; $("#active").innerHTML = ""; return; }
  const vals = ST.values || {};
  const sel = (k, l) => {
    const list = vals[k] || [];
    const cur = (F[k] || [])[0] || "";
    return `<label><span class="lbl">${l}</span><select class="field" data-f="${k}"><option value="">Any</option>${list.map((v) => `<option ${v === cur ? "selected" : ""}>${esc(v)}</option>`).join("")}</select></label>`;
  };
  const yrs = ST.years || [1990, 2026];
  const y = F.year || yrs[1];
  box.innerHTML = FILTERS.map(([k, l]) => sel(k, l)).join("") +
    `<label><span class="lbl">From</span><input class="field yr" type="number" data-f="from" value="${F.from || ""}" placeholder="${yrs[0]}"></label>` +
    `<label><span class="lbl">To</span><input class="field yr" type="number" data-f="to" value="${F.to || ""}" placeholder="${yrs[1]}"></label>` +
    `<div class="slider"><span class="lbl" id="yr-l">${F.year ? "As it stood in " + F.year : "Year slider: today"}</span><input type="range" id="yr" min="${yrs[0]}" max="${yrs[1]}" value="${y}" aria-label="The world as it stood in a year"></div>` +
    `<button class="btn sm" type="button" id="yr-now">Today</button><button class="btn sm" type="button" id="f-clear">Clear</button>`;
  drawActive();
}

function drawActive() {
  const chips = [];
  FILTERS.forEach(([k, l]) => (F[k] || []).forEach((v) => chips.push(`<span class="chip">${l} ${esc(v)}<button type="button" data-x="${k}" aria-label="Remove">×</button></span>`)));
  if (F.from || F.to) chips.push(`<span class="chip">Years ${F.from || "…"}–${F.to || "…"}<button type="button" data-x="years" aria-label="Remove">×</button></span>`);
  if (F.year) chips.push(`<span class="chip">As it stood in ${F.year}<button type="button" data-x="year" aria-label="Remove">×</button></span>`);
  if (ST && ST.open && !ST.hidden) chips.push(`<span class="chip bad">Hidden truths off</span>`);
  $("#active").innerHTML = chips.join("");
}

function setF(k, v) {
  if (v === "" || v == null) delete F[k]; else F[k] = v;
  store.set("filters", F);
  drawActive();
  clearTimeout(setF.t);
  setF.t = setTimeout(render, 150);
}

// ---------- Blocks ----------
const YEARISH = /^(year|season)$|_year$|^year_|_season$/i;
// The book keeps dates as days since 1 January 1970.
const dayText = (d) => new Date(d * 86400000).toISOString().slice(0, 10);
function cellHtml(v, col, names) {
  if (col.mark) return mark(v) || esc(v);
  if (col.moved) return mark(v) || esc(v);
  const n = typeof v === "number" ? v : NaN;
  const shown = names && names[String(v)] != null ? names[String(v)]
    : col.fmt === "date" && Number.isInteger(n) && Math.abs(n) < 100000 ? dayText(n)
    : (col.fmt === "year" || YEARISH.test(col.name || "")) && n >= 1800 && n < 2300 ? String(v)
    : v == null && YEARISH.test(col.name || "") ? "–" : fmt(v);
  if (col.link && v != null && v !== "") return `<a href="#/e/${esc(col.link)}/${encodeURIComponent(v)}">${esc(shown)}</a>`;
  return esc(shown);
}

function tableHtml(b) {
  const cols = b.cols || [];
  const rows = b.rows || [];
  if (!rows.length) return `<p class="sub">${esc(b.empty || "No rows.")}</p>`;
  const head = cols.map((c) => `<th class="${c.num ? "n" : ""}" data-h="${hv(c.h)}">${esc(c.label || c.name)}</th>`).join("");
  const body = rows.map((r, i) => {
    const link = b.links && b.links[i];
    const h = b.hs && b.hs[i];
    const tone = b.tones && b.tones[i];
    return `<tr class="${link ? "click" : ""}" ${link ? `data-go="${esc(link)}"` : ""} ${h ? `data-h="${hv(h)}"` : ""}>` + r.map((v, j) => {
      const c = cols[j] || {};
      const t = c.tone && v ? ` t-${c.tone}` : "";
      return `<td class="${c.num ? "n" : ""}${t}"${c.minw ? ` style="min-width:${Number(c.minw)}px"` : ""}>${cellHtml(v, c)}</td>`;
    }).join("") + "</tr>";
  }).join("");
  return `<div class="tbl"><table><thead><tr>${head}</tr></thead><tbody>${body}</tbody></table></div>`;
}

// A table paged from the book: sortable, filtered, every id a link.
function rowsHtml(b, id) {
  const f = b.first;
  if (!f) return "";
  const hide = new Set(b.hide || []);
  const cols = f.cols.map((c, i) => ({ c, i })).filter((x) => !hide.has(x.c.name));
  const head = cols.map(({ c }) => {
    const arrow = b.sort === c.name ? `<span class="arrow">${b.desc ? "↓" : "↑"}</span>` : "";
    return `<th class="sort ${c.debug ? "dbgc" : ""}" data-sort="${esc(c.name)}" data-h="${hv(Object.assign({ what: c.label }, c.h || {}))}">${esc(c.label)}${c.debug ? ' <span class="mk dbg">debug</span>' : ""}${arrow}</th>`;
  }).join("");
  const idx = f.cols.findIndex((c) => c.name === "id");
  // A world log row keeps more than its columns: its whole record opens
  // on its own page.
  const rec = f.record ? f.cols.findIndex((c) => c.name === "_row") : -1;
  const recHead = rec >= 0 ? `<th data-h="${hv({ what: "Record", note: "The row's whole record, every field the world log kept" })}">Record</th>` : "";
  const body = f.rows.map((r) => {
    let open = "";
    if (b.open && idx >= 0) open = b.open + r[idx];
    else if (idx >= 0 && f.cols[idx].link) open = `e/${f.cols[idx].link}/${r[idx]}`;
    return `<tr class="${open ? "click" : ""}" ${open ? `data-go="${esc(open)}"` : ""}>` + cols.map(({ c, i }) => {
      const v = r[i];
      const nm = f.names && f.names[c.name];
      const n = typeof v === "number" && !(nm && nm[String(v)] != null);
      return `<td class="${n ? "n" : ""} ${c.debug ? "dbgc" : ""}">${cellHtml(v, c, nm)}</td>`;
    }).join("") + (rec >= 0 ? `<td><a href="#/e/${esc(b.table || f.table)}/${r[rec]}">Record</a></td>` : "") + "</tr>";
  }).join("");
  const from = f.total ? f.offset + 1 : 0;
  const to = Math.min(f.offset + f.rows.length, f.total);
  const filt = f.filterable && f.filterable.length ? `Filters here: ${f.filterable.join(", ")}` : "No filter applies to this table";
  return `<div class="tbl"><table><thead><tr>${head}${recHead}</tr></thead><tbody>${body || `<tr><td class="sub">No rows under these filters.</td></tr>`}</tbody></table></div>
    <div class="pager"><span class="sub">${fmt(from)}–${fmt(to)} of ${fmt(f.total)} · ${esc(filt)}</span>
    <span><button class="btn sm" type="button" data-page="${id}" data-off="${Math.max(0, f.offset - 50)}" ${f.offset ? "" : "disabled"}>Previous</button>
    <button class="btn sm" type="button" data-page="${id}" data-off="${f.offset + 50}" ${to < f.total ? "" : "disabled"}>Next</button>
    ${b.table ? `<a class="btn sm" href="#/table/${esc(b.table)}${b.col ? `?col=${encodeURIComponent(b.col)}&val=${encodeURIComponent(b.val)}` : ""}">Whole table</a>` : ""}</span></div>`;
}

function chartHtml(b) {
  const S = b.series || [];
  const pts = S.flatMap((s) => s.points.filter((p) => p[1] != null));
  if (pts.length < 2) return `<p class="sub">Too few points for a chart.</p>`;
  const W = 1000, Hh = 280, L = 44, R = 12, T = 12, B = 30;
  const xs = pts.map((p) => +p[0]), ys = pts.map((p) => +p[1]);
  const x0 = Math.min(...xs), x1 = Math.max(...xs);
  let y0 = Math.floor(Math.min(...ys) / 10) * 10, y1 = Math.ceil(Math.max(...ys) / 10) * 10;
  if (y1 === y0) y1 = y0 + 10;
  const X = (x) => L + ((x - x0) / Math.max(1, x1 - x0)) * (W - L - R);
  const Y = (y) => T + (1 - (y - y0) / (y1 - y0)) * (Hh - T - B);
  const colors = ["#f0607c", "#b1b6c0", "#7c818c", "#ffd60a"];
  let g = "";
  for (let y = y0; y <= y1; y += Math.max(5, Math.round((y1 - y0) / 5 / 5) * 5)) g += `<line class="ax" x1="${L}" x2="${W - R}" y1="${Y(y)}" y2="${Y(y)}"/><text x="${L - 8}" y="${Y(y) + 4}" text-anchor="end">${y}</text>`;
  const step = Math.max(1, Math.round((x1 - x0) / 10));
  for (let x = x0; x <= x1; x += step) g += `<text x="${X(x)}" y="${Hh - 8}" text-anchor="middle">${x}</text>`;
  if (b.mark_year && b.mark_year >= x0 && b.mark_year <= x1) g += `<line x1="${X(b.mark_year)}" x2="${X(b.mark_year)}" y1="${T}" y2="${Hh - B}" stroke="#ffd60a" stroke-dasharray="3 4"/>`;
  S.forEach((s, i) => {
    const p = s.points.filter((q) => q[1] != null).map((q) => `${X(+q[0]).toFixed(1)},${Y(+q[1]).toFixed(1)}`).join(" ");
    const dash = s.style === "dash" ? ' stroke-dasharray="6 5"' : "";
    const w = s.style === "thin" ? 1.5 : 3;
    g += `<polyline fill="none" stroke="${colors[i % 4]}" stroke-width="${w}"${dash} points="${p}"/>`;
    if (i === 0) s.points.forEach((q) => { if (q[1] != null) g += `<circle cx="${X(+q[0])}" cy="${Y(+q[1])}" r="3.5" fill="${colors[0]}"><title>${q[0]}: ${fmt(q[1])}</title></circle>`; });
  });
  const legend = S.map((s, i) => `<span class="key"><i style="background:${colors[i % 4]}"></i>${esc(s.name)}${s.debug ? ' <span class="mk dbg">debug</span>' : ""}</span>`).join("");
  return `<div class="legend">${legend}</div><div class="chart"><svg viewBox="0 0 ${W} ${Hh}" role="img" aria-label="${esc(b.title)}">${g}</svg></div>`;
}

function replayHtml(b, id) {
  const cols = b.cols || [];
  const snaps = b.snaps || [];
  const cur = Math.min(b.cur || 0, snaps.length - 1);
  const s = snaps[cur] || [];
  const si = cols.findIndex((c) => c.name === b.spot);
  const spot = si >= 0 && typeof s[si] === "number" ? s[si] : null;
  // yardline_100: yards to the end zone the offense is going for.
  const pct = spot == null ? null : 9.09 + (100 - spot) * 0.8182;
  const now = cols.map((c, i) => `<div><span class="lbl">${esc(c.label)}</span><br><b class="num" style="font-size:18px">${esc(fmt(s[i]))}</b></div>`).join("");
  const list = snaps.map((r, i) => `<tr class="click ${i === cur ? "cur" : ""}" data-snap="${id}:${i}">` + r.map((v) => `<td class="${typeof v === "number" ? "n" : ""}">${esc(fmt(v))}</td>`).join("") + "</tr>").join("");
  return `<div class="row-form"><button class="btn sm" data-step="${id}:-1" type="button">Previous snap</button><button class="btn sm" data-step="${id}:1" type="button">Next snap</button>
    <button class="btn sm" data-auto="${id}" type="button">${b.auto ? "Pause" : "Play"}</button><span class="sub">Snap ${cur + 1} of ${snaps.length}</span></div>
    ${pct == null ? "" : `<div class="field-bar" aria-label="Ball on the ${spot}"><div class="ez" style="left:0"></div><div class="ez" style="right:0"></div><div class="los" style="left:${pct}%"></div><div class="ball" style="left:${pct}%"></div></div>`}
    <div class="snapnow">${now}</div>
    <div class="tbl" style="max-height:420px;overflow:auto"><table><thead><tr>${cols.map((c) => `<th>${esc(c.label)}</th>`).join("")}</tr></thead><tbody>${list}</tbody></table></div>`;
}

function blockHtml(b, id) {
  const title = b.title ? `<div class="head"><h2>${esc(b.title)}</h2>${b.note ? `<span class="sub">${esc(b.note)}</span>` : ""}${b.more && b.more.n > (b.items || []).length ? (b.more.link ? `<a class="go" href="#/${esc(b.more.link)}">All ${fmt(b.more.n)} →</a>` : `<span class="go">${fmt(b.more.n)} in all</span>`) : ""}</div>` : "";
  switch (b.kind) {
    case "hero": {
      const figs = (b.figures || []).map((f) => `<div><span class="lbl ${f.mark ? "t-" + f.mark : ""}">${esc(f.label)}</span><span class="big ${f.mark ? "t-" + f.mark : ""}">${esc(fmt(f.value))}</span></div>`).join("");
      return `<section class="panel hero"><div style="display:flex;flex-direction:column;gap:12px;flex:1 1 420px;min-width:0"><span class="lbl">${esc(b.label || "")}</span><h1>${esc(b.title)}</h1>${b.text ? `<p>${esc(b.text)}</p>` : ""}<div class="tags">${(b.tags || []).map((t) => `<span class="tag">${esc(t)}</span>`).join("")}</div>${(b.actions || []).length ? `<div class="tags">${b.actions.map((a) => `<a class="btn sm" href="#/${esc(a.link)}">${esc(a.text)}</a>`).join("")}</div>` : ""}</div>${figs ? `<div class="figs">${figs}</div>` : ""}</section>`;
    }
    case "tiles":
      return `<section class="sec">${title}<div class="tiles">${(b.tiles || []).map((t) => `<a class="tile" ${t.link ? `href="#/${esc(t.link)}"` : ""} data-h="${hv(t.h)}"><div class="row1"><span class="lbl">${esc(t.label)}</span>${mark(t.mark)}</div><span class="big">${esc(t.value)}</span><span class="sub">${esc(t.sub)}</span></a>`).join("")}</div></section>`;
    case "list": {
      const items = b.items || [];
      const body = items.length ? items.map((i) => `<div class="it"><p>${i.link ? `<a href="#/${esc(i.link)}">${esc(i.text)}</a>` : esc(i.text)}${i.why ? `<span class="why">${esc(i.why)}</span>` : ""}</p>${mark(i.mark)}</div>`).join("") : `<p class="sub">${esc(b.empty || "Nothing here.")}</p>`;
      return `<section class="panel">${title}<div class="items${b.dense ? " dense" : ""}">${body}</div></section>`;
    }
    case "links":
      return `<section class="sec">${title}<div class="links">${(b.items || []).map((i) => `<a href="#/${esc(i.link)}"><b>${esc(i.text)}</b><span class="sub">${esc(i.sub || "")}</span></a>`).join("")}</div></section>`;
    case "table":
      return `<section class="panel sec">${title}${tableHtml(b)}</section>`;
    case "rows":
      return `<section class="panel sec" id="blk-${id}">${title}${rowsHtml(b, id)}</section>`;
    case "kv":
      return `<section class="panel sec">${title}<div class="kv">${(b.pairs || []).map((p) => `<div data-h="${hv(p.h)}"><span>${esc(p.k)}${p.debug ? ' <span class="mk dbg">debug</span>' : ""}</span><span>${p.link ? `<a href="#/${esc(p.link)}">${esc(p.v)}</a>` : esc(p.v)}</span></div>`).join("")}</div></section>`;
    case "pair":
      return `<div class="pair">${(b.blocks || []).map((x, i) => blockHtml(x, id + "-" + i)).join("")}</div>`;
    case "chart":
      return `<section class="panel sec">${title}${chartHtml(b)}</section>`;
    case "replay":
      return `<section class="panel sec" id="blk-${id}">${title}${replayHtml(b, id)}</section>`;
    case "note":
      return `<p class="sub" style="margin:0">${esc(b.text)}</p>`;
    default:
      // The page lanes' views (charts.js).
      return window.extraBlock ? window.extraBlock(b, id) : "";
  }
}

// ---------- Pages ----------
function pageArgs(extra) {
  const { path, args } = parseRoute();
  return Object.assign({ route: path, filters: F }, args, extra || {});
}

async function render() {
  H = [];
  const el = $("#page");
  if (!ST) await refresh();
  const { path } = parseRoute();
  drawSide();
  if (path === "library" || !ST.open) return library();
  el.innerHTML = `<p class="loading">Reading…</p>`;
  const t0 = performance.now();
  try {
    PAGE = await api("page", pageArgs());
  } catch (e) {
    el.innerHTML = `<div class="err">${esc(e.message || e)}</div>`;
    return;
  }
  PAGE.rule = "Near: within 5% of real · Off: within 15% · Far off: beyond (the Debug Lab's thresholds, proposed, not confirmed)";
  $("#crumb").textContent = PAGE.crumb || PAGE.title;
  document.title = `${PAGE.title} · AFM World Reader`;
  drawPage(t0);
}

function drawPage(t0) {
  H = [];
  const p = PAGE;
  let html = "";
  if (p.tabs) html += `<div class="tabs">${p.tabs.map((t) => `<a class="${t.slug === p.tab ? "on" : ""}" href="#/tools/${t.slug}">${esc(t.title)}${t.code ? ` <span class="sub">${t.code}</span>` : ""}</a>`).join("")}</div>`;
  if (p.compare) html += compareBar(p);
  const firstHero = p.blocks && p.blocks[0] && p.blocks[0].kind === "hero";
  if (!firstHero) html += `<div class="sec"><h1 class="wide" style="margin:0;font-size:34px;font-weight:800;letter-spacing:-.02em">${esc(p.title)}</h1></div>`;
  const badges = (p.badges || []).map((b) => `<span class="chip ${b.ok ? "" : "bad"}" data-h="${b.h ? hv(b.h) : ""}">${esc(b.text)}</span>`).join("");
  const verdict = `<p class="verdict">${esc(p.verdict || "")}</p>${badges ? `<div class="chips">${badges}</div>` : ""}`;
  (p.blocks || []).forEach((b, i) => {
    html += blockHtml(b, i);
    if (i === 0 && firstHero) html += verdict;
  });
  // A function, so a "$1.2B" in the verdict is not read as a group.
  if (!firstHero) html = html.replace(/(<\/h1><\/div>)/, (m) => m + verdict);
  if (p.play) html += playForm(p);
  if (t0) html += `<p class="sub" style="margin:0">Read in ${Math.round(performance.now() - t0)} ms from the book.</p>`;
  $("#page").innerHTML = html;
}

// ---------- The library ----------
async function library() {
  const el = $("#page");
  $("#crumb").textContent = "World library";
  el.innerHTML = `<div class="sec"><h1 class="wide" style="margin:0;font-size:34px;font-weight:800">World library</h1><p class="verdict">Books on the worlds page download in one click. Books on this Mac open at once. A world file or a save gets its book built here, read from the world once.</p><div id="job"></div></div><div id="lib-online" class="panel"><p class="loading">Looking at the worlds page…</p></div><div id="lib-local"></div>`;
  let lib;
  try { lib = await api("library"); } catch (e) { $("#lib-local").innerHTML = `<div class="err">${esc(e.message || e)}</div>`; return; }
  const row = (b, act) => `<div class="it"><p><b>${esc(b.name)}</b><span class="why">${[b.gb >= 0.1 ? b.gb + " GB" : b.mb != null ? b.mb + " MB" : "", b.seed ? "seed " + b.seed : "", b.years ? b.years + " years" : "", b.commit ? "commit " + String(b.commit).slice(0, 7) : "", b.career || "", b.date || ""].filter(Boolean).map(esc).join(" · ")}</span></p>${act}</div>`;
  const books = lib.books.map((b) => row(b, `<button class="btn sm" data-open="${esc(b.path)}" type="button">Open</button>`)).join("") || `<p class="sub">None yet. Download one above, or open a book file.</p>`;
  const files = lib.worlds.concat(lib.saves).map((w) => row(w, w.book ? `<button class="btn sm" data-open="${esc(w.path)}" type="button">Open its book</button>` : `<button class="btn sm" data-build="${esc(w.path)}" type="button">Build a book</button>`)).join("") || `<p class="sub">No world files or saves in the game's folder.</p>`;
  $("#lib-local").innerHTML = `<div class="pair"><section class="panel"><div class="head"><h2>Books on this Mac</h2><button class="btn sm" id="open-file" type="button">Open a file…</button></div><div class="items">${books}</div></section>
    <section class="panel"><div class="head"><h2>The game's worlds and saves</h2></div><div class="items">${files}</div></section></div>`;
  pollJob();
  try {
    const on = await api("online");
    const list = on.books.map((b) => row({ name: b.title || b.name, gb: (b.mb / 1000).toFixed(2), date: (b.date || "").slice(0, 10) }, b.have ? `<span class="mk near">On this Mac</span>` : `<button class="btn sm pri" data-dl="${esc(b.tag)}" type="button">Download</button>`)).join("");
    $("#lib-online").innerHTML = `<div class="head"><h2>On the worlds page</h2><span class="sub">${esc(on.repo)}</span></div><div class="items">${list || `<p class="sub">No world there carries a book yet. The world build publishes one beside each new world.</p>`}</div>`;
  } catch (e) {
    $("#lib-online").innerHTML = `<div class="head"><h2>On the worlds page</h2></div><p class="sub">${esc(e.message || e)}</p>`;
  }
}

async function pollJob() {
  const box = $("#job");
  if (!box) return;
  const j = await api("job");
  if (!j.kind) return;
  box.innerHTML = `<div class="sec" style="margin:8px 0 14px"><span class="sub">${esc(j.error || j.text)}</span><div class="progress"><span style="width:${Math.round((j.done || 0) * 100)}%"></span></div></div>`;
  if (j.running) setTimeout(pollJob, 700);
  else if (j.made && !j.error) { await openPath(j.made); }
}

// Builds a world file's or save's book on this Mac, then opens it.
async function buildBook(p) {
  try {
    await api("build", { path: p });
    if (route() !== "library") go("library"); else pollJob();
  } catch (err) { toast(err.message || String(err)); }
}

async function openPath(p) {
  try {
    const r = await api("open", { path: p });
    if (r.needs_book) { await buildBook(r.path); return; }
    ST = r;
    go("overview");
    await refresh();
    render();
  } catch (e) { toast(e.message || String(e)); }
}

async function openFile() {
  let p = null;
  if (TAURI && TAURI.dialog) {
    p = await TAURI.dialog.open({ multiple: false, filters: [{ name: "World books, worlds and saves", extensions: ["afmbook", "afmworld", "afmsave", "001"] }] });
  } else {
    p = prompt("Path of a book (.afmbook), a world (.afmworld) or a save (.afmsave):");
  }
  if (p) openPath(p);
}

// ---------- Compare ----------
function compareBar(p) {
  const modes = [["worlds", "Two worlds"], ["years", "Two years"], ["groups", "Two groups"], ["player", "A player against real"]];
  const tabs = `<div class="tabs">${modes.map(([m, l]) => `<a class="${p.compare === m ? "on" : ""}" href="#/compare/${m}">${l}</a>`).join("")}</div>`;
  const { args } = parseRoute();
  let form = "";
  if (p.compare === "worlds") {
    form = `<div class="row-form"><label><span class="lbl">World A</span><select class="field" id="cmp-a"><option value="">Pick a book…</option></select></label><span class="sub">World B is the one open now: ${esc(ST.world)}.</span></div>`;
    setTimeout(async () => {
      const lib = await api("library");
      const s = $("#cmp-a");
      if (!s) return;
      s.innerHTML += lib.books.filter((b) => b.path !== ST.path).map((b) => `<option value="${esc(b.path)}" ${b.path === args.a ? "selected" : ""}>${esc(b.name)}</option>`).join("");
    });
  } else if (p.compare === "years" || p.compare === "groups") {
    const tables = (p.pick && p.pick.tables) || [];
    const t = tables.find((x) => x.name === args.table);
    const opts = tables.map((x) => `<option ${x.name === args.table ? "selected" : ""}>${esc(x.name)}</option>`).join("");
    form = `<div class="row-form"><label><span class="lbl">Table</span><select class="field" id="cmp-t"><option value="">Pick…</option>${opts}</select></label>`;
    if (p.compare === "years") {
      form += `<label><span class="lbl">Year 1</span><input class="field yr" type="number" id="cmp-y1" value="${args.y1 || ""}"></label><label><span class="lbl">Year 2</span><input class="field yr" type="number" id="cmp-y2" value="${args.y2 || ""}"></label>`;
    } else {
      const cols = t ? t.cols : [];
      form += `<label><span class="lbl">Column</span><select class="field" id="cmp-c"><option value="">Pick…</option>${cols.map((c) => `<option ${c === args.col ? "selected" : ""}>${esc(c)}</option>`).join("")}</select></label>
        <label><span class="lbl">Group 1</span><select class="field" id="cmp-g1"></select></label><label><span class="lbl">Group 2</span><select class="field" id="cmp-g2"></select></label>`;
      if (args.table && args.col) setTimeout(async () => {
        const vals = await api("values", { table: args.table, col: args.col });
        const o = (g) => vals.map((v) => `<option ${String(v.v) === String(args[g]) ? "selected" : ""} value="${esc(v.v)}">${esc(v.v)} (${fmt(v.n)})</option>`).join("");
        if ($("#cmp-g1")) { $("#cmp-g1").innerHTML = `<option value="">Pick…</option>` + o("g1"); $("#cmp-g2").innerHTML = `<option value="">Pick…</option>` + o("g2"); }
      });
    }
    form += `<button class="btn sm pri" id="cmp-go" type="button">Compare</button></div>`;
  } else if (p.compare === "player") {
    form = `<div class="row-form"><label><span class="lbl">Player</span><input class="field" id="cmp-p" type="search" placeholder="Search a player" autocomplete="off"></label><div id="cmp-pr" class="sub"></div></div>`;
  }
  return `<section class="panel sec">${tabs}${form}</section>`;
}

function compareGo() {
  const p = PAGE;
  const q = new URLSearchParams();
  const v = (id) => ($(id) || {}).value || "";
  q.set("table", v("#cmp-t"));
  if (p.compare === "years") { q.set("y1", v("#cmp-y1")); q.set("y2", v("#cmp-y2")); }
  if (p.compare === "groups") { q.set("col", v("#cmp-c")); q.set("g1", v("#cmp-g1")); q.set("g2", v("#cmp-g2")); }
  go(`compare/${p.compare}?${q}`);
}

// ---------- Play two teams now ----------
function playForm(p) {
  return `<section class="panel sec"><div class="head"><h2>Play two teams now</h2></div><div class="row-form">
    <label><span class="lbl">Home</span><input class="field" id="pl-h" type="search" placeholder="Search a program" autocomplete="off"></label>
    <label><span class="lbl">Road</span><input class="field" id="pl-a" type="search" placeholder="Search a program" autocomplete="off"></label>
    <label><span class="lbl">Seed</span><input class="field yr" id="pl-s" type="number" value="2026"></label>
    <button class="btn pri" id="pl-go" type="button" ${p.play.table ? "" : "disabled"}>Play now</button></div><div id="pl-pick" class="sub"></div><div id="pl-out"></div></section>`;
}
const PICK = {};
async function pickSearch(input, table, slot) {
  const res = await api("search", { q: input.value, limit: 40 });
  const list = res.filter((r) => !table || r.table === table).slice(0, 8);
  $("#pl-pick").innerHTML = list.map((r) => `<a class="tag" data-pick="${slot}" data-id="${esc(r.id)}" data-name="${esc(r.name)}">${esc(r.name)}</a>`).join(" ");
}

// ---------- Events ----------
document.addEventListener("click", async (e) => {
  const t = e.target.closest("[data-open],[data-go],[data-sort],[data-page],[data-x],[data-dl],[data-build],[data-step],[data-snap],[data-auto],[data-pick],#copy,#hidden,#filters-btn,#yr-now,#f-clear,#open-file,#cmp-go,#pl-go,.results a");
  if (!t) { if (!e.target.closest(".searchbox")) $("#results").hidden = true; return; }
  if (t.matches(".results a")) { $("#results").hidden = true; return; }
  if (t.dataset.open) { e.preventDefault(); return openPath(t.dataset.open); }
  if (t.dataset.go && !e.target.closest("a")) return go(t.dataset.go);
  if (t.dataset.sort) {
    const blk = t.closest("section");
    const id = blk && blk.id.replace("blk-", "");
    const b = PAGE.blocks[id];
    if (!b) return;
    b.desc = b.sort === t.dataset.sort ? !b.desc : false;
    b.sort = t.dataset.sort;
    return pageRows(id, 0);
  }
  if (t.dataset.page) return pageRows(t.dataset.page, +t.dataset.off);
  if (t.dataset.x) {
    const k = t.dataset.x;
    if (k === "years") { delete F.from; delete F.to; } else delete F[k];
    store.set("filters", F);
    drawFilters();
    return render();
  }
  if (t.dataset.dl) { try { await api("download", { tag: t.dataset.dl }); pollJob(); } catch (err) { toast(err.message || String(err)); } return; }
  if (t.dataset.build) return buildBook(t.dataset.build);
  if (t.dataset.step || t.dataset.snap) {
    const [id, n] = (t.dataset.step || t.dataset.snap).split(":");
    const b = PAGE.blocks[id];
    b.cur = t.dataset.step ? Math.max(0, Math.min((b.snaps || []).length - 1, (b.cur || 0) + +n)) : +n;
    return redrawBlock(id);
  }
  if (t.dataset.auto) {
    const id = t.dataset.auto;
    const b = PAGE.blocks[id];
    b.auto = !b.auto;
    const tick = () => {
      if (!b.auto || PAGE.blocks[id] !== b) return;
      if ((b.cur || 0) >= b.snaps.length - 1) { b.auto = false; redrawBlock(id); return; }
      b.cur = (b.cur || 0) + 1;
      redrawBlock(id);
      setTimeout(tick, 900);
    };
    redrawBlock(id);
    setTimeout(tick, 900);
    return;
  }
  if (t.dataset.pick) { PICK[t.dataset.pick] = { id: +t.dataset.id, name: t.dataset.name }; $(t.dataset.pick === "h" ? "#pl-h" : "#pl-a").value = t.dataset.name; $("#pl-pick").innerHTML = ""; return; }
  switch (t.id) {
    case "copy": return copyPage();
    case "hidden": await api("hidden", { on: !ST.hidden }); await refresh(); return render();
    case "filters-btn": { const f = $("#filters"); f.hidden = !f.hidden; t.setAttribute("aria-expanded", String(!f.hidden)); return; }
    case "yr-now": delete F.year; store.set("filters", F); drawFilters(); return render();
    case "f-clear": F = {}; store.set("filters", F); drawFilters(); return render();
    case "open-file": return openFile();
    case "cmp-go": return compareGo();
    case "pl-go": {
      if (!PICK.h || !PICK.a) { toast("Pick both teams from the search."); return; }
      $("#pl-out").innerHTML = `<p class="loading">Playing… The first game loads the world (about a minute).</p>`;
      try {
        const g = await api("play", { home: PICK.h.id, away: PICK.a.id, seed: +$("#pl-s").value || 2026 });
        H = [];
        $("#pl-out").innerHTML = `<p class="verdict">${esc(g.verdict)}</p>` + g.blocks.map((b, i) => blockHtml(b, "g" + i)).join("");
        PAGE.played = g;
      } catch (err) { $("#pl-out").innerHTML = `<div class="err">${esc(err.message || err)}</div>`; }
      return;
    }
  }
});

document.addEventListener("change", (e) => {
  const t = e.target;
  if (t.dataset && t.dataset.f) {
    const k = t.dataset.f;
    if (k === "from" || k === "to") return setF(k, t.value ? +t.value : "");
    return setF(k, t.value ? [t.value] : "");
  }
  if (t.id === "cmp-a") go(`compare/worlds?a=${encodeURIComponent(t.value)}`);
  if (t.id === "cmp-t" && PAGE.compare === "groups") go(`compare/groups?table=${encodeURIComponent(t.value)}`);
  if (t.id === "cmp-c") go(`compare/groups?table=${encodeURIComponent($("#cmp-t").value)}&col=${encodeURIComponent(t.value)}`);
});

document.addEventListener("input", (e) => {
  const t = e.target;
  if (t.id === "yr") {
    $("#yr-l").textContent = "As it stood in " + t.value;
    clearTimeout(t._t);
    t._t = setTimeout(() => { F.year = +t.value; store.set("filters", F); drawActive(); render(); }, 250);
  }
  if (t.id === "q") search(t.value);
  if (t.id === "pl-h" || t.id === "pl-a") { clearTimeout(t._t); t._t = setTimeout(() => pickSearch(t, PAGE.play && PAGE.play.table, t.id === "pl-h" ? "h" : "a"), 200); }
  if (t.id === "cmp-p") {
    clearTimeout(t._t);
    t._t = setTimeout(async () => {
      const res = (await api("search", { q: t.value, limit: 40 })).filter((r) => /people|players|persons/.test(r.table)).slice(0, 8);
      $("#cmp-pr").innerHTML = res.map((r) => `<a class="tag" href="#/compare/player?id=${encodeURIComponent(r.id)}">${esc(r.name)} <span class="sub">${esc(r.detail || "")}</span></a>`).join(" ");
    }, 200);
  }
});

async function search(q) {
  const box = $("#results");
  clearTimeout(search.t);
  if (!q || q.trim().length < 2 || !ST || !ST.open) { box.hidden = true; return; }
  search.t = setTimeout(async () => {
    const res = await api("search", { q, limit: 24 });
    box.innerHTML = res.length ? res.map((r) => `<a href="#/e/${esc(r.table)}/${encodeURIComponent(r.id)}"><span><b>${esc(r.name)}</b>${r.detail ? ` <span class="sub">${esc(r.detail)}</span>` : ""}</span><span class="k">${esc(String(r.table).replace(/_/g, " "))}</span></a>`).join("") : `<p class="sub" style="padding:8px 12px">Nothing matches “${esc(q)}”.</p>`;
    box.hidden = false;
  }, 160);
}

document.addEventListener("keydown", (e) => {
  if (e.target.id === "q" && e.key === "Enter") { const a = $("#results a"); if (a) { location.hash = a.getAttribute("href"); $("#results").hidden = true; } }
  if (e.target.id === "q" && e.key === "Escape") $("#results").hidden = true;
});

async function pageRows(id, off) {
  const b = PAGE.blocks[id];
  const f = await api("rows", { table: b.table, filters: F, col: b.col || undefined, val: b.val, sort: b.sort, desc: !!b.desc, offset: off, limit: 50 });
  b.first = f;
  redrawBlock(id);
}

function redrawBlock(id) {
  const el = $("#blk-" + id);
  if (!el) return;
  const tmp = document.createElement("div");
  tmp.innerHTML = blockHtml(PAGE.blocks[id], id);
  el.replaceWith(tmp.firstElementChild);
}

async function copyPage() {
  if (!ST || !ST.open) return;
  try {
    const text = await api("copy", pageArgs());
    try { await navigator.clipboard.writeText(text); } catch (e) {
      const ta = document.createElement("textarea");
      ta.value = text; document.body.appendChild(ta); ta.select(); document.execCommand("copy"); ta.remove();
    }
    toast(`Copied ${text.split("\n").length} lines for Claude`);
  } catch (e) { toast(e.message || String(e)); }
}

// The hover card: what the number is, how it is worked out, where the real
// figure comes from.
document.addEventListener("mouseover", (e) => {
  const t = e.target.closest("[data-h]");
  const hc = $("#hc");
  if (!t || t.dataset.h === "") { hc.style.display = "none"; return; }
  const h = H[+t.dataset.h];
  if (!h) { hc.style.display = "none"; return; }
  const parts = [];
  if (h.what) parts.push(`<b>${esc(h.what)}</b>`);
  if (h.why && h.why !== h.what) parts.push(`<div>${esc(h.why)}</div>`);
  if (h.unit) parts.push(`<div class="r">Unit: ${esc(h.unit)}</div>`);
  if (h.file || h.function) parts.push(`<div class="r">Worked out in <code>${esc(h.file || "")}</code>${h.function ? `, <code>${esc(h.function)}</code>` : ""}</div>`);
  if (h.source) parts.push(`<div class="r">Real: ${esc(h.source)}${h.years ? `, ${esc(h.years)}` : ""}</div>`);
  else if ("source" in h) parts.push(`<div class="r">No real figure in the index.</div>`);
  if (h.how) parts.push(`<div class="r">How: ${esc(h.how)}</div>`);
  if (h.needs && h.needs !== "world file") parts.push(`<div class="r">Needs the ${esc(h.needs)}.</div>`);
  if (h.issues) parts.push(`<div class="r">Known issues: ${esc(h.issues)}</div>`);
  if (h.area) parts.push(`<div class="r">Inventory area ${esc(h.area)}</div>`);
  if (h.rule) parts.push(`<div class="r">${esc(h.rule)}</div>`);
  if (h.sql) parts.push(`<div class="r"><code>${esc(h.sql.length > 240 ? h.sql.slice(0, 240) + "…" : h.sql)}</code></div>`);
  if (h.debug) parts.push(`<div class="r">Debug: hidden in the game.</div>`);
  if (h.missing) parts.push(`<div class="r">No definition in this book yet for <code>${esc(h.key)}</code>.</div>`);
  else if (h.key && !h.rule) parts.push(`<div class="r"><code>${esc(h.key)}</code></div>`);
  if (!parts.length) { hc.style.display = "none"; return; }
  hc.innerHTML = parts.join("");
  hc.style.display = "block";
  const r = t.getBoundingClientRect();
  const w = hc.offsetWidth, hh = hc.offsetHeight;
  hc.style.left = Math.max(8, Math.min(window.innerWidth - w - 8, r.left)) + "px";
  hc.style.top = (r.bottom + hh + 12 < window.innerHeight ? r.bottom + 8 : Math.max(8, r.top - hh - 8)) + "px";
});

// The Mac app updates itself from the downloads page (reader-latest).
async function checkUpdate() {
  if (!TAURI || !TAURI.updater) return;
  let up = null;
  try { up = await TAURI.updater.check(); } catch (e) { return; }
  if (!up) return;
  const chip = document.createElement("button");
  chip.className = "btn sm pri";
  chip.type = "button";
  chip.textContent = `Update to ${up.version}`;
  chip.onclick = async () => {
    chip.disabled = true;
    chip.textContent = "Updating…";
    try { await up.downloadAndInstall(); await TAURI.process.relaunch(); } catch (e) { chip.textContent = "Update failed"; }
  };
  $(".side-foot").prepend(chip);
}

window.addEventListener("hashchange", render);
checkUpdate();
refresh().then(render).catch((e) => { $("#page").innerHTML = `<div class="err">${esc(e.message || e)}</div>`; });
