// ======================================================================
// The squad's quick filter (Cain, 2026-11-04: "which players have sub 60
// speed on my WRs or CBs"). One bar on every squad view (Overview and the
// working views) and on other teams' squads: a search box, any attribute
// or stat against a number (more than one rule reads "and"), and the
// positions. The matching players show as you type.
//
// Every attribute is your staff's read as a range, never the true number:
// a rule matches the range's middle, as the tables sort, and a read
// nobody has made ("Not seen") matches no rule. Potential is never a
// field. The fields come with the squad (`quick_filter.rs`, `catalog`).
// How it works: docs/reference/how-it-works/squad-filter.md.
// ======================================================================
const QF = { reuse: false, cache: {} };
const QF_OPS = [["<", "under"], ["≤", "at most"], ["=", "exactly"], ["≥", "at least"], [">", "over"]];
const QF_POS = [["QB", "RB", "WR", "TE", "OT", "IOL"], ["EDGE", "DT", "LB", "CB", "S"], ["K", "P", "LS"]];
const QF_RULES = 4;
const qfCopy = (x) => (typeof structuredClone === "function" ? structuredClone(x) : JSON.parse(JSON.stringify(x)));

// The squad's replies, kept for a redraw the bar asks for: typing in the
// box draws the screen again from what it has, without asking the game.
async function qfGet(cmd, args = {}) {
  const key = cmd + "|" + JSON.stringify(args);
  if (QF.reuse && QF.cache[key]) return qfCopy(QF.cache[key]);
  const x = await api(cmd, args);
  QF.cache[key] = qfCopy(x);
  return x;
}
// Whose squad: yours (every view shares one filter) or another team's.
const qfScope = (a) => (!a || !a.kind || (typeof trMine === "function" && trMine(a)) ? "mine" : `${a.kind}${a.id}`);
function qfState(scope) {
  const st = keep("qf:" + scope, { q: "", rules: [], pos: [] });
  if (!st.rules.length) st.rules.push({ k: "", op: "<", v: "" });
  return st;
}
// The catalog's fields by key, each with its group.
function qfFields(groups) {
  const m = new Map();
  for (const g of groups || []) for (const f of g.fields) m.set(f.k, { ...f, group: g.name });
  return m;
}
// A number typed for a field: a height reads 6-2, 6'2" or 74 (inches).
function qfNum(text, kind) {
  // "1,000" is a thousand (the screen prints its numbers that way); a
  // lone comma before one or two digits is a decimal point.
  const raw = String(text ?? "").trim();
  const t = /^\d{1,3}(,\d{3})+(\.\d+)?$/.test(raw) ? raw.replace(/,/g, "") : raw.replace(",", ".");
  if (!t) return null;
  if (kind === "height") {
    const m = t.match(/^(\d)\s*(?:'|-|ft|\s)\s*(\d{1,2}(?:\.\d+)?)?\s*(?:"|in)?$/i);
    if (m) return Number(m[1]) * 12 + Number(m[2] || 0);
  }
  const n = Number(t);
  return Number.isFinite(n) ? n : null;
}
// A field's range for one player (a read), or null.
function qfRead(r, f) {
  if (f.k === "ovr") return r.ovr || null;
  const m = /^a(\d+)$/.exec(f.k);
  return m && r.attrs ? r.attrs[Number(m[1])] || null : null;
}
// A field's number for one player: a read's middle, his age or body, his
// season in your games. Null when there is none (a read not seen).
function qfVal(r, f) {
  if (f.kind === "read") { const x = qfRead(r, f); return x ? (x[0] + x[1]) / 2 : null; }
  if (f.k === "ht") return inches(r.ht);
  if (f.k.startsWith("p:")) { const v = r.prod ? r.prod[f.k.slice(2)] : null; return typeof v === "number" ? v : null; }
  const v = r[f.k];
  return typeof v === "number" ? v : null;
}
function qfTest(n, op, m, round) {
  switch (op) {
    case "<": return n < m;
    case "≤": return n <= m;
    case "≥": return n >= m;
    case ">": return n > m;
    // "Exactly" a whole number: a read's middle rounds (57-62 is 60);
    // a count is exact (a 4.5-sack season is not 5).
    default: return Math.abs(n - m) < 0.05 || (round && Number.isInteger(m) && Math.round(n) === m);
  }
}
// The rules that filter: a field picked and a number typed.
function qfLive(st, fields) {
  return st.rules.map((u) => ({ u, f: fields.get(u.k) })).filter((x) => x.f && qfNum(x.u.v, x.f.kind) != null);
}
const qfNorm = (s) => String(s || "").normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
function qfOn(st, fields) {
  return !!(st.q.trim() || st.pos.length || qfLive(st, fields).length);
}
// The rows that match the bar: every rule, one of the positions picked,
// and every word typed somewhere in his name, hometown or position.
function qfApply(rows, st, fields) {
  const live = qfLive(st, fields);
  const words = qfNorm(st.q).split(/\s+/).filter(Boolean);
  return rows.filter((r) => {
    if (st.pos.length && !st.pos.includes(r.pos)) return false;
    if (words.length) {
      const text = qfNorm(`${r.name} ${r.home || ""} ${r.pos || ""}`);
      if (!words.every((w) => text.includes(w))) return false;
    }
    return live.every(({ u, f }) => {
      const n = qfVal(r, f);
      return n != null && qfTest(n, u.op, qfNum(u.v, f.kind), f.kind === "read");
    });
  });
}
// A number as the screen prints it: a rate to its decimals, a half sack
// as 4.5, a whole count with its thousands.
function qfFmt(f, n) {
  if (f.dp) return n.toFixed(f.dp);
  return Number.isInteger(n) ? n.toLocaleString() : (Math.round(n * 10) / 10).toLocaleString();
}
function qfShow(f, r) {
  if (f.kind === "read") return range(qfRead(r, f), null, { p: r.id, k: f.key });
  const n = qfVal(r, f);
  if (n == null) return '<span class="muted">-</span>';
  if (f.k === "ht") return esc(r.ht || "");
  return esc(qfFmt(f, n));
}
// A table's columns for the fields the rules name, after his class (or
// his position): the field you filter on is always in view.
function qfCols(cols, st, fields, key) {
  const have = new Set(cols.map((c) => c.k));
  const add = [];
  for (const { f } of qfLive(st, fields)) {
    if (have.has(f.k) || have.has("qf:" + f.k)) continue;
    have.add("qf:" + f.k);
    const csv = (r) => { const x = f.kind === "read" ? qfRead(r, f) : null; return f.kind === "read" ? (x ? `${x[0]}-${x[1]}` : "") : f.k === "ht" ? r.ht : qfVal(r, f) ?? ""; };
    add.push({ k: "qf:" + f.k, t: f.label, tip: `${f.label} · ${f.kind === "read" ? "your staff's read" : f.group} · the quick filter's field`, div: true, v: (r) => qfVal(r, f), f: (r) => qfShow(f, r), csv });
  }
  if (key) qfTidy(key, [...cols, ...add]);
  if (!add.length) return cols;
  let at = cols.findIndex((c) => c.k === "class");
  if (at < 0) at = cols.findIndex((c) => c.k === "pos");
  const out = cols.slice();
  out.splice(at + 1, 0, ...add);
  return out;
}
// A rule taken away takes its column: the table's sort and its own
// filters on that column go with it (else the sort stays on a column
// that is gone and the position groups never come back).
function qfTidy(key, cols) {
  const gone = (k) => k && String(k).startsWith("qf:") && !cols.some((c) => c.k === k);
  const s = S.keep["sort:" + key];
  if (s) {
    if (gone(s.k2)) s.k2 = null;
    if (gone(s.k)) { s.k = s.k2 || null; s.dir = s.dir2 || -1; s.k2 = null; }
  }
  const t = S.keep["tbl:" + key];
  if (t && t.filters) t.filters = t.filters.filter((f) => !gone(f.k));
}
// The Overview's line under a name: the read or number each rule names.
function qfHits(r, st, fields) {
  const seen = new Set();
  return qfLive(st, fields).filter(({ f }) => !seen.has(f.k) && seen.add(f.k)).map(({ f }) => {
    const x = f.kind === "read" ? qfRead(r, f) : null;
    const val = f.kind === "read" ? (x ? `${x[0]}-${x[1]}` : "not seen") : f.k === "ht" ? r.ht : qfVal(r, f) == null ? "-" : qfFmt(f, qfVal(r, f));
    return `<span class="qf-hit" title="${esc(f.label)}${f.kind === "read" ? ": your staff's read" : ""}">${esc(f.label)} <b>${esc(val)}</b></span>`;
  }).join("");
}
// The bar. `rows` are the squad before the bar (for the counts), `shown`
// how many it leaves.
function qfBar(scope, groups, rows, shown) {
  const st = qfState(scope);
  const fields = qfFields(groups);
  const on = qfOn(st, fields);
  const opts = (u) => `<option value="">Any attribute or stat</option>${(groups || []).map((g) => `<optgroup label="${esc(g.name)}">${g.fields.map((f) => `<option value="${esc(f.k)}" ${f.k === u.k ? "selected" : ""}>${esc(f.label)}</option>`).join("")}</optgroup>`).join("")}`;
  const rule = (u, i) => {
    const f = fields.get(u.k);
    const bad = u.v !== "" && f && qfNum(u.v, f.kind) == null;
    const hint = f && f.kind === "height" ? "6-2" : f && f.kind === "read" ? "60" : "Value";
    return `<span class="qf-rule${f && qfNum(u.v, f.kind) != null ? " on" : ""}">${i ? '<i class="qf-and">and</i>' : ""}<select data-qf="k" data-i="${i}" aria-label="Attribute or stat" title="${f ? esc(f.group) : "Pick an attribute, his body or a stat"}">${opts(u)}</select><select data-qf="op" data-i="${i}" aria-label="Comparison">${QF_OPS.map(([o, t]) => `<option value="${o}" ${o === u.op ? "selected" : ""} title="${t}">${o}</option>`).join("")}</select><input data-qf="v" data-i="${i}" value="${esc(u.v)}" placeholder="${hint}" inputmode="decimal" autocomplete="off" aria-label="Value"${bad ? ' aria-invalid="true"' : ""}>${i || u.k || u.v ? `<button class="qf-x" data-act="qf-del" data-i="${i}" aria-label="Remove this rule" title="Remove this rule">×</button>` : ""}</span>`;
  };
  const count = (p) => rows.filter((r) => r.pos === p).length;
  const chips = QF_POS.map((g) => g.filter((p) => count(p) || st.pos.includes(p))).filter((g) => g.length).map((g) => `<span class="qf-pg">${g.map((p) => `<button class="qf-chip" data-act="qf-pos" data-v="${p}" aria-pressed="${st.pos.includes(p)}">${p}<small>${count(p)}</small></button>`).join("")}</span>`).join("");
  const read = qfLive(st, fields).some(({ f }) => f.kind === "read");
  return `<div class="qf${on ? " on" : ""}" data-scope="${esc(scope)}" role="search" aria-label="Quick filter">
    <div class="qf-line"><label class="qf-find">${icon("search")}<input type="search" data-qf="q" value="${esc(st.q)}" placeholder="Find a player by name or hometown" autocomplete="off" spellcheck="false" aria-label="Find a player"></label>
      <span class="qf-rules">${st.rules.map(rule).join("")}${st.rules.length < QF_RULES ? '<button class="tbtn qf-add" data-act="qf-add" title="Add another rule: every rule must hold">+ And</button>' : ""}</span>
      <span class="tspace"></span>
      <span class="qf-count" title="${read ? "A read matches on the middle of your staff's range; one nobody has seen matches no rule." : ""}">${on ? `<b>${shown}</b> of ${rows.length} players` : `${rows.length} players`}</span>${on ? '<button class="tbtn" data-act="qf-clear">Clear</button>' : ""}</div>
    <div class="qf-line qf-posline" role="group" aria-label="Positions"><span class="qf-lbl">Positions</span>${chips}${st.pos.length ? '<button class="qf-link" data-act="qf-allpos">Every position</button>' : ""}</div>
  </div>`;
}
// The empty table or board's word when the bar leaves nobody.
function qfNone(scope) {
  return `<div class="qf-none"><b>Nobody matches the quick filter.</b> <button class="qf-link" data-act="qf-clear" data-scope="${esc(scope)}">Clear it</button></div>`;
}

// ---------- the bar's controls ----------
// Draws the screen again from the replies it has; the box you were
// typing in keeps its place.
function qfLocal(el, focus) {
  QF.reuse = true;
  if (typeof SB !== "undefined") SB.reuse = true;
  const a = document.activeElement;
  const f = focus || (a && a.dataset && a.dataset.qf ? { qf: a.dataset.qf, i: a.dataset.i, s: a.selectionStart, e: a.selectionEnd } : null);
  // The page beside draws on its own: the box goes back where it was.
  const beside = !!(el && el.closest && el.closest(".beside"));
  return Promise.resolve(refresh(el)).finally(() => {
    QF.reuse = false;
    if (typeof SB !== "undefined") SB.reuse = false;
    if (!f) return;
    const box = document.querySelector(beside ? ".beside" : ".view") || document;
    const n = box.querySelector(`.qf [data-qf="${f.qf}"]${f.i != null ? `[data-i="${f.i}"]` : ""}`);
    if (!n) return;
    n.focus();
    try { if (f.s != null) n.setSelectionRange(f.s, f.e); } catch (e) { /* a select has no caret */ }
  });
}
const qfOf = (el) => { const box = el.closest(".qf, [data-scope]"); return box ? qfState(box.dataset.scope) : null; };
document.addEventListener("input", (ev) => {
  const t = ev.target;
  if (!t.dataset || !t.dataset.qf || t.tagName === "SELECT") return;
  const st = qfOf(t);
  if (!st) return;
  if (t.dataset.qf === "q") st.q = t.value;
  else if (t.dataset.qf === "v") st.rules[Number(t.dataset.i)].v = t.value;
  clearTimeout(QF.typing);
  QF.typing = setTimeout(() => qfLocal(t), 90);
});
document.addEventListener("change", (ev) => {
  const t = ev.target;
  if (!t.dataset || !t.dataset.qf || t.tagName !== "SELECT") return;
  const st = qfOf(t);
  if (!st) return;
  const u = st.rules[Number(t.dataset.i)];
  if (!u) return;
  u[t.dataset.qf] = t.value;
  // A field picked: on to its number.
  qfLocal(t, t.dataset.qf === "k" && t.value ? { qf: "v", i: t.dataset.i } : null);
});
document.addEventListener("keydown", (ev) => {
  const t = ev.target;
  if (ev.key !== "Escape" || !t.dataset || !t.dataset.qf || t.tagName === "SELECT") return;
  const st = qfOf(t);
  if (!st || !t.value) return;
  ev.preventDefault();
  t.value = "";
  if (t.dataset.qf === "q") st.q = "";
  else st.rules[Number(t.dataset.i)].v = "";
  qfLocal(t);
});
document.addEventListener("click", (ev) => {
  const el = ev.target.closest("[data-act]");
  if (!el || !String(el.dataset.act).startsWith("qf-")) return;
  const st = qfOf(el);
  if (!st) return;
  ev.preventDefault();
  switch (el.dataset.act) {
    case "qf-pos": {
      const p = el.dataset.v;
      st.pos = st.pos.includes(p) ? st.pos.filter((x) => x !== p) : [...st.pos, p];
      break;
    }
    case "qf-allpos": st.pos = []; break;
    case "qf-add": st.rules.push({ k: "", op: "<", v: "" }); return qfLocal(el, { qf: "k", i: String(st.rules.length - 1) });
    case "qf-del": {
      st.rules.splice(Number(el.dataset.i), 1);
      break;
    }
    case "qf-clear": st.q = ""; st.pos = []; st.rules = []; break;
    default: return;
  }
  qfLocal(el);
});
