// Live talks, lane 2: the ask box (docs/reference/how-it-works/live-talks.md,
// "Lane 2, the boxes"). A question in your words over what your staff can
// see: the stat finder, Find, the squad filter's bar, the next opponent, a
// scout, a report, a source, and the leads mail. Frames 4 to 6 and 8 of
// afm/live-talks/lanes/lane-0.md. The strip is the talk box's (say.js,
// aiBoxHtml); what differs is the answer.
//
// A page asks for its boxes with aiAskBox(list, kinds, key, options), where
// the list is the reply's `ask` (the contract: an entry is a question box, a
// button with no text, or a call the page makes once as it opens) and the
// kinds are the ones this place takes ("analyst", "find", "squad",
// "opponent", "scout", "report", "source", "leads"). Nothing shows when the
// reply carries none of them (and the demo switch is off). `options`:
//   presets   entries standing in for the flag under the demo switch
//   words     the strip's own words
//   query     async (query, line, table, box): the page takes the search into
//             its own filters and runs it (the stat finder, Find, the squad
//             bar). Without it the answer's chips, line and table show in the
//             box (the opponent card).
//   wrap      "panel": the strip draws its own card
// An entry may also carry, all optional: `label` (the strip's title),
// `placeholder`, `caption`, `name` and `role` (who answers), `button`,
// `wait` (the wait line), `rerun` ({cmd, args}: takes a chip away and asks
// the page's own command for the table again).
"use strict";

const AI_ASK_WORDS = {
  analyst: {
    title: "Ask your analyst", ph: "Ask in your words: 'which freshman backs ran for 1,000 yards since 2010?'",
    who: "Your analyst", reading: "is reading your question.", nofollow: "Nothing was searched. Your words are still in the box.",
    done: "Your analyst filled in the search below. Change anything and press Search.", off: "Questions in your words", offTail: "The filters work as before.",
  },
  find: {
    title: "Ask in your words", ph: "Ask in your words: 'a safety from Texas, fast, with a high academic grade'.",
    who: "Your analyst", reading: "is reading your question.", nofollow: "Nothing was searched. Your words are still in the box.",
    done: "Your analyst set the filters below. Change anything.", off: "Questions in your words", offTail: "The filters work as before.",
  },
  squad: {
    title: "Ask in your words", ph: "Find players in your words: 'receivers over 6-2 who run under 4.55'.",
    who: "Your analyst", reading: "is reading your question.", nofollow: "Nothing was searched. Your words are still in the box.",
    done: "Your analyst set the rules above. Change anything.", off: "Questions in your words", offTail: "The bar works as before.",
  },
  opponent: {
    title: "Ask your analyst", ph: "What do they do on third and long?",
    who: "Your analyst", reading: "is reading your question.", nofollow: "Nothing was searched. Your words are still in the box.",
    done: "Your analyst ran it on the snaps your staff has charted.", off: "Questions in your words", offTail: "The page works as before.",
  },
  scout: {
    title: "Ask your scout", ph: "What would you like to know about him?",
    who: "Your scout", reading: "is looking at his file.", nofollow: "Nothing was answered. Your words are still in the box.",
    off: "Questions in your words", offTail: "The file reads as before.",
  },
  report: {
    title: "The report", btn: "Write the report", again: "Write it again", who: "Your staff", reading: "is writing the report from his file.",
    nofollow: "Nothing was written.", off: "The report", offTail: "The file reads as before.",
    cap: "A report written from the file: ranges, trait reads, the medical grade, comparisons. Nothing outside the file goes in.",
  },
  source: {
    title: "Ask a source", ph: "What would you like to ask?",
    who: "Your source", reading: "is thinking about it.", nofollow: "Nothing was answered. Your words are still in the box.",
    off: "Questions in your words", offTail: "The file reads as before.",
    cap: "People who know him. Pick who you ask. They know what they know, and some of it is a guess.",
  },
};

// The entries of a reply's ask list this place takes, or the demo's presets.
function aiAskItems(list, kinds, presets) {
  const own = (Array.isArray(list) ? list : []).filter((e) => e && e.args && kinds.includes(e.args.kind));
  return own.length ? own : aiDevOn() ? (presets || []).map((p) => ({ ...p, preset: true })) : [];
}
function aiAskBox(list, kinds, key, o = {}) {
  return aiAskItems(list, kinds, o.presets).filter((e) => !e.auto).map((e, i) => aiAskOne(e, `${key}:${e.args.kind}:${i}`, o)).join("");
}
function aiAskOne(e, key, o) {
  const kind = e.args.kind;
  const base = AI_ASK_WORDS[kind] || AI_ASK_WORDS.analyst;
  const words = { ...base, ...(e.label ? { title: e.label } : {}), ...(e.placeholder ? { ph: e.placeholder } : {}), ...(e.wait ? { reading: e.wait } : {}), ...(e.caption ? { cap: e.caption } : {}), ...(e.button ? { btn: e.button } : {}), ...(o.words || {}) };
  const b = aiBox(key, e.button ? "btn" : "ask", e, words);
  b.o = o;
  b.e = e;
  return aiBoxHtml(b);
}

// A button with no text ("Write the report"): one press, one answer.
function aiBtnHtml(b) {
  const w = b.w;
  const key = esc(b.key);
  const wrap = b.o && b.o.wrap === "panel" ? " panel" : "";
  const div = (inner) => `<div class="ai-box ai-btnbox${wrap}" data-ai="${key}" data-aikind="btn">${inner}</div>`;
  if (b.phase === "off") {
    if (b.offShown) return "";
    b.offShown = true;
    return div(`<div class="ai-note"><span class="ai-lb">${esc(w.off)}</span><span>are off for now. ${esc(b.note || "")} ${esc(w.offTail)}</span></div>`);
  }
  const wait = b.phase === "wait";
  const again = b.answer ? w.again || "Ask again" : null;
  const label = wait ? "Working" : again || w.btn || "Go";
  const caption = w.cap ? `<span class="ai-cap">${esc(w.cap)}</span>` : "";
  const none = b.phase === "none" ? `<div class="ai-note"><b>${esc(b.note || "Nothing came back.")}</b><span>${esc(w.nofollow)}</span></div>` : "";
  return div(`<div class="ai-h"><span class="ai-lb">${esc(w.title)}</span></div>
    <div class="ai-acts"><button type="button" class="ai-btn${wait ? " dim" : again ? "" : " pri"} sm" data-act="aisend" data-notext="1" data-ai="${key}"${wait ? " disabled" : ""}>${esc(label)}</button></div>
    ${wait ? `<span class="ai-pbar"><i></i></span><div class="ai-note"><span class="ai-lb">${esc(w.who)}</span><span>${esc(w.reading)}</span></div>` : ""}${caption}${none}${wait ? "" : aiAskAfter(b)}`);
}

// ---------- an answer ----------
// What sits under the strip: the answer in words, or what a search ran.
function aiAskAfter(b) {
  const a = b.answer;
  if (!a) return "";
  const w = b.w;
  if (a.type === "text") return aiTextHtml(a.text, a.by, a.small, b.e && b.e.args.kind === "report");
  const line = a.line ? `<p class="ai-line2">${esc(a.line)}</p>` : "";
  // The page took the search into its own filters: one note says so.
  if (b.o && b.o.query) return `<div class="ai-note"><span class="small">${esc(w.done || "")}</span></div>${line}`;
  const chips = (a.query || []).length ? `<div class="ai-fgroup"><div class="ai-top"><span class="ai-lb ai-gd">Set from your question</span><span class="small">Your analyst only reads the search. The page runs it.</span></div><div class="ai-frow">${a.query.map((q, i) => `<span class="ai-chip set"><span>${aiQueryLabel(q)}</span>${b.e && b.e.rerun ? `<u role="button" tabindex="0" title="Take it away" data-act="aichipx" data-ai="${esc(b.key)}" data-i="${i}">×</u>` : ""}</span>`).join("")}</div></div>` : "";
  return `${chips}${line}${aiTableHtml(a.table)}`;
}
// One entry of a query as a chip: "Down 3", "Distance at least 7".
const AI_OPS = { ge: "at least", le: "at most", between: "between", is: "", ">=": "at least", "<=": "at most", ">": "over", "<": "under", "≥": "at least", "≤": "at most", "=": "" };
function aiQueryLabel(q) {
  const name = q.label || String(q.field || q.stat || "").replace(/[_:]+/g, " ").replace(/^./, (c) => c.toUpperCase());
  const op = AI_OPS[q.op] || "";
  const val = q.words != null ? q.words : Array.isArray(q.value) ? q.value.join(", ") : q.v2 != null ? `${q.value} and ${q.v2}` : String(q.value);
  return `<span class="ai-mu">${esc(name)}</span> <b>${esc((op ? op + " " : "") + val)}</b>`;
}
// A box's answer arrives (the states in common are aiLand's, in say.js).
async function aiAskDone(b, a, text) {
  b.phase = "ready";
  const kind = b.e && b.e.args.kind;
  if (a.reload) {
    b.answer = null;
    if (!aiHere(b)) { aiPaint(b); return; }
    if (b.o && b.o.reload) await b.o.reload(b); else await refresh();
    return;
  }
  if (a.query || a.table) {
    b.answer = { type: "query", line: a.line, query: a.query || [], table: a.table };
    if (b.o && b.o.query && aiHere(b)) { try { await b.o.query(a.query || [], a.line, a.table, b); } catch (e) { toast(e.message); aiPaint(b); } return; }
    aiPaint(b);
    return;
  }
  const t = typeof a.text === "string" ? a.text : a.reply && typeof a.reply.text === "string" ? a.reply.text : null;
  if (t != null) {
    const pick = b.flag.choose ? b.flag.choose.options.find((op) => String(op.value) === String(b.choice ?? b.flag.choose.options[0].value)) : null;
    b.answer = {
      type: "text", text: t,
      by: (b.e && b.e.by) || (kind === "report" ? "Scouting report" : pick ? pick.label : b.e && b.e.name) || b.w.title,
      small: (b.e && b.e.byNote) || (kind === "report" ? "Written from your file" : b.e && b.e.role) || "",
    };
  }
  aiPaint(b);
}
// A chip taken away, on a page with a command of its own to ask again.
Object.assign(CORE_MORE, {
  aichipx: async (el) => {
    const b = aiOf(el);
    if (!b || !b.answer || !b.e || !b.e.rerun) return;
    const query = b.answer.query.filter((_, i) => i !== Number(el.dataset.i));
    try {
      const r = await api(b.e.rerun.cmd, { ...(b.e.rerun.args || {}), query });
      b.answer = { type: "query", line: r.line, query: r.query || query, table: r.table };
      aiPaint(b);
    } catch (e) { toast(e.message); }
  },
});

// ---------- the pages' own filters take the search ----------
const aiNum = (v) => (v !== "" && v != null && Number.isFinite(Number(v)) ? Number(v) : v);
// The stat finder: its filters (keep "statfind") and conditions.
async function aiQueryStat(query, line, table, b) {
  const K = keep("statfind", { find: "seasons", conds: [] });
  const keys = ["find", "level", "division", "state", "from", "to", "pos", "class", "rs", "age_min", "age_max", "min_len"];
  const set = [];
  for (const k of keys) delete K[k];
  K.find = "seasons";
  K.conds = [];
  for (const q of query) {
    if (q.field === "cond" || q.stat) {
      K.conds.push({ stat: q.stat, op: /^(le|<=|≤|<)$/.test(q.op || "") ? "<=" : ">=", v: aiNum(q.value) });
      set.push("cond");
    } else if (keys.includes(q.field)) {
      K[q.field] = ["from", "to", "age_min", "age_max", "min_len"].includes(q.field) ? aiNum(q.value) : q.value;
      set.push(q.field);
    }
  }
  K.ai = { set, line: line || "" };
  await refresh(aiEl(b.key));
}
// The squad bar: its search box, its positions and its rules.
async function aiQuerySquad(query, line, table, b) {
  const scope = b.o.scope || "mine";
  const st = qfState(scope);
  st.q = "";
  st.pos = [];
  st.rules = [];
  for (const q of query) {
    if (q.field === "pos") st.pos = [...new Set([...st.pos, ...[].concat(q.value)])];
    else if (q.field === "q") st.q = String(q.value);
    else {
      const op = { ge: "≥", le: "≤", ">=": "≥", "<=": "≤", is: "=", "=": "=", ">": ">", "<": "<", "≥": "≥", "≤": "≤" }[q.op] || "≥";
      st.rules.push({ k: q.field, op, v: String(q.value), ai: true });
    }
  }
  if (!st.rules.length) st.rules.push({ k: "", op: "<", v: "" });
  await qfLocal(aiEl(b.key));
}
// Find recruits and Find prospects: their class, level, position, place, name and filters.
async function aiQueryFind(query, line, table, b) {
  const st = frState();
  const filters = [];
  for (const q of query) {
    if (q.field === "pos") st.pos = String(q.value);
    else if (q.field === "class") st.cls = Number(q.value);
    else if (q.field === "level") st.level = String(q.value);
    else if (q.field === "place") st.place = String(q.value);
    else if (q.field === "q") st.q = String(q.value);
    else if (q.field === "sure") st.sure = String(q.value);
    else if (q.field === "unseen") st.unseen = !!q.value;
    else if (FR.last && frField(FR.last.x, q.field)) {
      const op = { ">=": "ge", "<=": "le", "≥": "ge", "≤": "le", ">": "ge", "<": "le", "=": "is" }[q.op] || q.op || "ge";
      filters.push({ k: q.field, op, v: aiNum(q.value), v2: q.v2 != null ? aiNum(q.v2) : "", or: false });
    }
  }
  st.filters = filters;
  st.page = 0;
  st.picked = [];
  await render($(".view") ? $(".view").scrollTop : 0);
}

// The stat finder's "Set from your question" group around its filters, and
// its set fields lit up.
function aiSetFrom(html, K) {
  if (!K || !K.ai || !K.ai.set.length) return html;
  return `<div class="ai-fgroup ai-setgroup"><div class="ai-top"><span class="ai-lb ai-gd">Set from your question</span><span class="small">The page runs the search. Your analyst only reads it.</span></div>${html}</div>`;
}
function aiMarkSet() {
  const K = S.keep.statfind;
  if (S.route.screen !== "statfind" || !K || !K.ai) return;
  for (const f of K.ai.set) {
    if (f === "cond") document.querySelectorAll(".sf-cond").forEach((c) => c.classList.add("set"));
    else document.querySelectorAll(`[data-act="sfset"][data-k="${f}"]`).forEach((c) => c.classList.add("set"));
  }
}
{
  const after0 = afterRender;
  afterRender = function () { after0(); aiMarkSet(); aiGrow(); };
}

// ---------- fact chips (question 5: they need no key) ----------
// "Bring it up": the facts you can raise in a contract talk, a trade call
// or a NIL form. A chip sends the flag's command with its fact and the
// page's reply lands as the page's own does (options.apply).
function aiFacts(x, key, o = {}) {
  const f = aiFlag(x, "facts", o.preset);
  if (!f || !(f.list || []).length) return "";
  const b = aiBox(key + ":facts", "facts", f, {});
  b.o = o;
  b.html = aiFactsHtml;
  return aiFactsHtml(b);
}
function aiFactsHtml(b) {
  const f = b.flag;
  const wrap = b.o && b.o.wrap === "panel" ? " panel" : "";
  return `<div class="ai-box ai-facts${wrap}" data-ai="${esc(b.key)}" data-aikind="facts"><div class="ai-sy"><div class="ai-h"><span class="ai-lb">Bring it up</span><span class="ai-cnt">${f.list.length} fact${f.list.length === 1 ? "" : "s"}</span></div>
    <div class="ai-fchips">${f.list.map((c) => `<button type="button" class="ai-fchip${b.used[c.fact] ? " used" : ""}" data-act="aifact" data-ai="${esc(b.key)}" data-fact="${c.fact}">${b.used[c.fact] ? "<i>✓</i>" : ""}${esc(c.label)}</button>`).join("")}</div>
    ${b.o.foot ? `<div class="ai-b"><span class="small">${esc(b.o.foot)}</span></div>` : ""}</div></div>`;
}
Object.assign(CORE_MORE, {
  aifact: async (el) => {
    const b = aiOf(el);
    if (!b || !b.flag) return;
    const fact = Number(el.dataset.fact);
    if (b.used[fact]) return;
    try {
      // The demo has no command behind a chip: it only marks it brought up.
      const r = b.flag.preset ? { demo: true } : await api(b.flag.cmd, { ...(b.flag.args || {}), fact });
      b.used[fact] = true;
      if (r && r.demo) { aiPaint(b); return; }
      if (b.o.apply) await b.o.apply(r, b); else await aiRedraw(r);
    } catch (e) { toast(e.message); }
  },
});

// ---------- the scout file (frame 5, frame 8) ----------
// "Ask about him": a scout, the report, a source, from the file's `ask`
// list, and (with no key) the sources row from its `sources` list.
const AI_SCOUT_PRESETS = (f) => [
  { cmd: "ai_ask", args: { kind: "scout", player: f.id }, limit: 400, name: "Your area scout", role: "Area recruiter", caption: "He saw him live and answers from his own file, in words and ranges." },
  { cmd: "ai_ask", args: { kind: "report", player: f.id }, button: "Write the report" },
  { cmd: "ai_ask", args: { kind: "source", player: f.id }, limit: 400, choose: { key: "source", options: [{ value: 5, label: "His high school coach" }, { value: 6, label: "A scout who watched him" }] } },
];
function aiScoutPanel(f) {
  const kinds = ["scout", "report", "source"];
  const entries = aiAskItems(f.ask, kinds, AI_SCOUT_PRESETS(f));
  const src = aiFlag(f, "sources", { cmd: "scout_source", args: { player: f.id }, list: [{ source: 5, label: "His high school coach" }, { source: 6, label: "A scout who watched him" }], topics: [{ topic: 1, label: "How he takes coaching" }, { topic: 2, label: "His character" }] });
  const typedSource = entries.some((e) => e.args.kind === "source");
  if (!entries.length && !src) return "";
  const rows = entries.map((e, i) => {
    return `<div class="ai-arow">${aiAskOne(e, `scout:${f.id}:${e.args.kind}:${i}`, { wrap: "" })}</div>`;
  }).join("");
  const srow = src && !typedSource ? `<div class="ai-arow">${aiSourceRowHtml(f, src)}</div>` : "";
  return `<section class="panel ai-asks"><h2>Ask about him <small>${entries.length ? "your staff answers from your file" : "people who know him"}</small></h2><div class="ai-askrows">${rows}${srow}</div></section>`;
}
// The sources row: who, what about, and Ask. It needs no key.
function aiSourceRowHtml(f, src) {
  const key = `src:${f.id}`;
  const b = aiBox(key, "source", src, {});
  b.html = () => aiSourceRowHtml(f, src);
  const list = src.list || [];
  const topics = src.topics || [];
  const sel = (name, items, cur, k, label) => `<select class="ai-pick" aria-label="${label}" data-ai-srcpick="${name}" data-ai="${esc(key)}">${items.map((it) => `<option value="${it[k]}"${String(it[k]) === String(cur ?? items[0][k]) ? " selected" : ""}>${esc(it.label)}</option>`).join("")}</select>`;
  const ans = b.answer ? aiTextHtml(b.answer.text, b.answer.by, b.answer.small) : "";
  return `<div class="ai-box ai-srcrow" data-ai="${esc(key)}" data-aikind="source"><div class="ai-h"><span class="ai-lb">Ask a source</span></div>
    <div class="ai-srow">${list.length ? sel("source", list, b.source, "source", "Who you ask") : ""}${topics.length ? sel("topic", topics, b.topic, "topic", "What about") : ""}<button type="button" class="ai-btn ${b.answer ? "" : "pri"}" data-act="aisrc" data-ai="${esc(key)}">${b.answer ? "Ask again" : "Ask"}</button></div>
    <span class="ai-cap">People who know him. Pick who and what. They know what they know, and some of it is a guess.</span>${ans}</div>`;
}
document.addEventListener("change", (ev) => {
  const p = ev.target.closest && ev.target.closest("[data-ai-srcpick]");
  const b = p && aiOf(p);
  if (b) b[p.dataset.aiSrcpick] = p.value;
});
Object.assign(CORE_MORE, {
  aisrc: async (el) => {
    const b = aiOf(el);
    if (!b || !b.flag) return;
    const src = b.flag;
    const args = { ...(src.args || {}), source: Number(b.source ?? (src.list[0] || {}).source), ...((src.topics || []).length ? { topic: Number(b.topic ?? src.topics[0].topic) } : {}) };
    try {
      const r = src.preset ? { demo: true, text: "He takes hard coaching well. He asks what he did wrong and goes and fixes it." } : await api(src.cmd, args);
      const pick = (src.list || []).find((s) => s.source === args.source);
      const t = r && (typeof r.text === "string" ? r.text : r.answer && r.answer.text);
      if (t) { b.answer = { text: t, by: (r.by && r.by.name) || (pick && pick.label) || "A source", small: (r.by && r.by.role) || "" }; aiPaint(b); }
      if (!r || !r.demo) await refresh(el);
    } catch (e) { toast(e.message); }
  },
});

// ---------- the leads mail (frame 6) ----------
// The mail's own call, made once as it opens: the wait line sits where the
// picks will be (inside the letter, after its words), and when the answer
// comes the letter asks for itself again.
const aiAutos = {};
function aiLetterEntry(m) {
  const own = (m.ask || []).find((e) => e && e.auto && e.args && e.args.kind === "leads");
  if (own) return own;
  return aiDevOn() && m.head === "memo" ? { cmd: "ai_ask", args: { kind: "leads", mail: m.id }, auto: true } : null;
}
// The wait line, drawn inside the letter's body while the call runs.
function aiLetterHtml(m) {
  const e = aiLetterEntry(m);
  const st = e && aiAutos[m.id];
  if (!st || st.phase !== "wait") return "";
  const first = String(m.from || "").split(/\s+/)[0] || "Your analyst";
  return `<div class="ai-wait" role="status"><span>${esc(e.wait || `${first} is picking the best few and writing why.`)}</span><span class="ai-wbar"><i></i></span></div>`;
}
// Called as a letter opens, before it is drawn: the first time only.
function aiLetterBegin(m) {
  const e = aiLetterEntry(m);
  if (!e || aiAutos[m.id]) return;
  aiAutos[m.id] = { phase: "wait" };
  aiLetterGo(m.id, e);
}
async function aiLetterGo(id, e) {
  const a = await aiCall(e.cmd, e.args, () => true);
  aiAutos[id].phase = "done";
  document.querySelectorAll(".ai-wait").forEach((w) => w.remove());
  // The letter is read again: the picks are in it now. Only if it is still the one open.
  if (a && a.state === "done" && a.reload && typeof MAIL !== "undefined" && MAIL.cur && String(MAIL.cur.id) === String(id)) await showLetter(id);
}
