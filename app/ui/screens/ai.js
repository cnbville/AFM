// Live talks, lane 2: the boxes (docs/reference/how-it-works/live-talks.md,
// "Lane 2, the boxes"). A box is one strip on a screen where you type a line
// (say.js: the talk box, ask.js: the ask box). This file holds what every box
// shares and the settings card:
//   - the state of each box (aiBox), kept by a key per talk or page, so a
//     screen that draws again (a chip, a tab) finds its box as it was;
//   - the call and the poll (aiCall): a box sends its command, gets a ticket
//     and asks ai_poll until the answer is not "wait";
//   - the "Heard as" line and the marks (aiHeardHtml);
//   - the demo switch (aiDevOn): off by default, and only on the fake client
//     (AFM_AI_FAKE=1). On, every place shows its box against ai_demo, whatever
//     the reply carries, and a small bar picks the state ai_demo plays;
//   - the settings card (aiSettingsPanel), in Settings and in the pregame
//     page's settings.
// Every top-level name here starts with "ai" (the screens share one scope).
// The AI only reads your words and writes theirs: nothing on these pages
// decides a result. With no key, no network, no credit or at the cap no box
// shows, and the chips work as before.
"use strict";

const aiState = {
  st: null, // the last ai_status
  fake: false, // the browser build runs on the fake client
  boxes: {}, // key -> a box's state
  said: {}, // place key -> the lines you typed, with how they were heard
  card: { test: null, checks: null, msg: null }, // the settings card
};
const AI_POLL_MS = 450;
const AI_LIMIT = 400;
// What ai_demo can play, in the order the demo bar lists them.
const AI_DEMO = [
  ["done", "Done"], ["wait", "Waiting"], ["unsure", "Heard two ways"], ["ask_first", "Ask first"], ["none", "Doesn't follow"],
  ["off", "Off (no network)"], ["cap", "Cap reached"], ["table", "A search ran"], ["text", "A text answer"], ["reload", "Reload the page"],
];
// The four marks on a fact in your line.
const AI_MARKS = { true: ["t", "True"], false: ["f", "Not true"], priced: ["p", "In the price"], later: ["l", "Checked later"] };
const aiSleep = (ms) => new Promise((r) => setTimeout(r, ms));

// ---------- the status, and the demo switch ----------
async function aiProbe() {
  try { aiState.st = await api("ai_status"); } catch (e) { aiState.st = null; }
  aiState.fake = !!(aiState.st && aiState.st.fake);
  aiDevPaint();
  return aiState.st;
}
// The switch: Settings has it (or ?aidemo in the address), and only the fake
// client can answer it, so a real key never meets a made-up answer.
function aiDevOn() {
  if (!aiState.fake) return false;
  let q = false;
  try { q = /[?&]aidemo\b/.test(location.search); } catch (e) { /* no address */ }
  return !!prefs.aiDemo || q;
}
const aiDemoState = () => (AI_DEMO.some((d) => d[0] === prefs.aiDemoState) ? prefs.aiDemoState : "done");
// A box shows when the reply carries its flag, or the switch is on (then the
// place's own preset stands in for the flag).
// A preset is marked, so a chip or a row that needs no key knows it is only the demo.
const aiFlag = (x, key, preset) => (x && x[key]) || (aiDevOn() && preset ? { ...preset, preset: true } : null);

// The small bar at the bottom of the window while the switch is on.
function aiDevPaint() {
  const on = aiDevOn();
  let bar = document.getElementById("ai-devbar");
  // The page keeps room under its last row while the bar floats over it.
  document.documentElement.classList.toggle("ai-dev", on);
  if (!on) { if (bar) bar.remove(); return; }
  if (!document.body) return;
  if (!bar) {
    bar = document.createElement("div");
    bar.id = "ai-devbar";
    document.body.appendChild(bar);
  }
  bar.innerHTML = `<span class="ai-lb">Live talks demo</span><select aria-label="What the next box answers" data-ai-demo>${AI_DEMO.map(([v, t]) => `<option value="${v}" ${aiDemoState() === v ? "selected" : ""}>${esc(t)}</option>`).join("")}</select><button type="button" class="ai-btn sm" data-act="aidemooff">Switch off</button>`;
}

// ---------- boxes ----------
// One box's state. Phases: ready (typing), wait, unsure, none, off, done.
function aiBox(key, kind, flag, words) {
  const b = (aiState.boxes[key] ||= { key, kind, text: "", phase: "ready", run: 0, sel: 0, used: {}, last: null, answer: null, route: null });
  b.kind = kind;
  // How a box draws itself again when its answer comes (a card or row of
  // another kind sets b.html itself).
  b.html = kind === "btn" ? aiBtnHtml : kind === "facts" ? aiFactsHtml : b.kind === "source" && b.html ? b.html : aiBoxHtml;
  b.flag = flag;
  b.w = words;
  return b;
}
// The element of a box on the page now.
function aiEl(key) {
  return [...document.querySelectorAll(".ai-box")].find((e) => e.dataset.ai === key) || null;
}
function aiPaint(b, keepFocus) {
  const el = aiEl(b.key);
  if (!el) return;
  const had = document.activeElement && el.contains(document.activeElement);
  const at = had && document.activeElement.selectionStart;
  el.outerHTML = typeof b.html === "function" ? b.html(b) : "";
  if (keepFocus || had) {
    const f = aiEl(b.key) && aiEl(b.key).querySelector("[data-ai-in]");
    if (f && !f.readOnly) { f.focus(); try { f.setSelectionRange(at ?? f.value.length, at ?? f.value.length); } catch (e) { /* not a text field */ } }
  }
  aiGrow();
}
// The text field grows with what is typed, up to five lines.
function aiGrow() {
  document.querySelectorAll(".ai-box [data-ai-in]").forEach((f) => {
    f.style.height = "auto";
    f.style.height = Math.min(f.scrollHeight + 2, 140) + "px";
  });
}
const aiHere = (b) => !b.route || S.route.screen === b.route;

// ---------- the call, and the poll ----------
// Sends a box's command (or ai_demo's state, with the switch on) and waits
// for the ticket's answer. alive() says the box still wants it: a line you
// took back, or a page you left, stops the wait. Returns an answer in the
// contract's shapes: {state: "unsure"|"done"|"none"|"off", ...}, or null.
async function aiCall(cmd, args, alive, demo) {
  let r;
  // demo false: a command the fake answers for itself (the key's test).
  try { r = aiDevOn() && demo !== false ? await api("ai_demo", { state: demo || aiDemoState() }) : await api(cmd, args); } catch (e) { return { state: "off", note: e.message }; }
  return aiSettle(r, alive);
}
async function aiSettle(r, alive) {
  if (r && r.ticket != null) {
    for (;;) {
      let a;
      try { a = await api("ai_poll", { ticket: r.ticket }); } catch (e) { return { state: "off", note: e.message }; }
      if (!alive()) return null;
      // The ticket rides along: a pick of a reading is sent with it.
      if (a.state !== "wait") return a.ticket == null ? { ...a, ticket: r.ticket } : a;
      await aiSleep(AI_POLL_MS);
    }
  }
  if (!alive()) return null;
  // A reply to a pick may already be an answer, or the screen's own reply.
  if (r && r.state) return r;
  return { state: "done", reply: r };
}

// ---------- the "Heard as" line and the marks ----------
function aiMarksHtml(marks) {
  return (marks || []).map((m) => {
    const [c, t] = AI_MARKS[m.mark] || AI_MARKS.later;
    const label = m.mark === "later" && m.when ? "Checked " + m.when : t;
    return `<div class="ai-cl"><span class="ai-mk ai-mk-${c}"><i></i>${esc(label)}</span><span>${esc(m.text)}</span></div>`;
  }).join("");
}
function aiHeardHtml(h) {
  if (!h) return "";
  return `${h.heard ? `<div class="ai-ha"><span class="ai-lb">Heard as</span><span>${esc(h.heard)}</span></div>` : ""}${aiMarksHtml(h.marks)}`;
}
const aiNorm = (s) => String(s ?? "").replace(/<[^>]*>/g, " ").replace(/\s+/g, " ").trim().toLowerCase();
// What you typed in a place, and how it was heard, kept to put under your
// line when the page draws its log again. A log that carries its own heard
// and marks on a line (the core's `heard` and `marks`) needs none of this.
function aiRemember(placeKey, text, heard, marks) {
  (aiState.said[placeKey] ||= []).push({ text: aiNorm(text), heard, marks });
}
// aiYours(placeKey)(line): the heard-as html for one of your lines, or "".
function aiYours(placeKey) {
  const list = (aiState.said[placeKey] || []).map((s) => ({ ...s, used: false }));
  return (l) => {
    if (!l) return "";
    if (l.heard || (l.marks && l.marks.length)) return aiHeardHtml(l);
    const n = aiNorm(l.text);
    const at = list.find((s) => !s.used && s.text === n);
    if (!at) return "";
    at.used = true;
    return aiHeardHtml(at);
  };
}

// A table in an answer (a search the page could not run itself).
function aiTableHtml(t) {
  if (!t || !t.columns || !t.rows) return "";
  return `<div class="ai-table"><table class="ai-dtab"><thead><tr>${t.columns.map((c, i) => `<th class="${i > 2 ? "r" : "l"}">${esc(c)}</th>`).join("")}</tr></thead><tbody>${t.rows.map((r) => `<tr>${r.map((c, i) => `<td class="${i > 2 ? "r" : "l"}">${rich(String(c))}</td>`).join("")}</tr>`).join("")}</tbody></table></div>`;
}
// An answer in words: a scout's, a source's. A report keeps its headings
// ("## Who he is" lines) and its paragraphs.
function aiTextHtml(text, by, small, report) {
  const lines = String(text || "").split(/\n+/).filter((l) => l.trim());
  const body = lines.map((l) => (/^#{1,3}\s/.test(l) ? `<h3>${esc(l.replace(/^#{1,3}\s*/, ""))}</h3>` : `<p>${rich(l)}</p>`)).join("");
  return `<div class="ai-ans${report ? " ai-rep" : ""}">${by ? `<div class="ai-by"><b>${esc(by)}</b>${small ? `<span class="small">${esc(small)}</span>` : ""}</div>` : ""}${body}</div>`;
}

// ---------- the settings card ----------
const AI_MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
function aiNextMonth(month) {
  const m = /^(\d{4})-(\d\d)$/.exec(month || "");
  return m ? "1 " + AI_MONTHS[Number(m[2]) % 12] : "next month";
}
function aiCardHtml(st) {
  const row = (t, sub, ctl, extra) => `<div class="setrow"><div><b>${t}</b><small>${sub}</small>${extra || ""}</div><div class="ai-ctl">${ctl || ""}</div></div>`;
  const c = aiState.card;
  const warn = (t) => `<div class="ai-line"><span class="ai-warn">${esc(t)}</span></div>`;
  if (!st) {
    return `<h2>Live talks <small>optional · the game plays fully without them</small></h2>${row("Not available", "This build of the game has no live talks.", "")}`;
  }
  const head = `<h2>Live talks <small>optional · the game plays fully without them</small></h2><p class="small ai-lede">Claude Haiku 5.5 reads your words and writes what comes back. The game decides every result.</p>`;
  const demo = st.fake
    ? row("Demo", "This browser build answers every call on the fake client, and nothing leaves this machine. The demo shows every box on every page and lets you pick what it answers.", `<div class="seg"><button data-act="aidemo" data-v="" aria-pressed="${!aiDevOn()}">Off</button><button data-act="aidemo" data-v="1" aria-pressed="${aiDevOn()}">On</button></div>`)
    : "";
  let key;
  if (!st.key_set) {
    key = row("Your key", "A key made for AFM alone in the Anthropic Console, with its own spend limit. It stays in your Mac's Keychain and never goes into a save, a world file or a log.",
      `<input class="ai-inp" type="password" autocomplete="off" spellcheck="false" placeholder="Paste your key" aria-label="Your key" data-ai-key><button type="button" class="ai-btn pri" data-act="aikeysave" disabled>Save key</button>`,
      c.msg ? warn(c.msg) : "")
      + row("With no key", "The game plays on the chips and the line bank, as it always has. Nothing is missing.", "");
  } else {
    const where = st.kept === "keychain"
      ? "In your Mac's Keychain. After an app update, macOS may ask for your Mac password once."
      : "Kept in memory only: the game forgets it when it closes.";
    let state = "";
    if (c.test && c.test.phase === "wait") state = `<div class="ai-line"><span class="ai-pbar"><i></i></span><span class="small">Testing…</span></div>`;
    else if (c.test && c.test.phase === "ok") state = `<div class="ai-line"><span class="ai-ok">${esc(c.test.text)}</span></div>`;
    else if (c.test && c.test.phase === "bad") state = warn(c.test.text);
    else if (st.note && !st.cap_reached) state = warn(/not accepted/i.test(st.note) ? "That key did not work. Check it, remove it and paste it again." : `${st.note} The chips work as before.`);
    key = row(`Key ending ${esc(st.key_ending || "")}`, where, `<button type="button" class="ai-btn" data-act="aikeytest"${c.test && c.test.phase === "wait" ? " disabled" : ""}>Test</button><button type="button" class="ai-btn" data-act="aikeyclear">Remove key</button>`, state);
  }
  let month = "";
  if (st.key_set || st.fake) {
    const pct = st.cap ? Math.min(100, (st.spent / st.cap) * 100) : 0;
    const money = (v) => "$" + Number(v).toFixed(2);
    month = row("This month", "Spend against the $" + st.cap + " cap. At the cap everything goes back to the chips, with a one-line note.",
      `<div class="ai-sp2"><b>${money(st.spent)} <span class="small">of $${st.cap}</span></b><span class="ai-mbar${st.cap_reached ? " am" : ""}"><i style="width:${pct.toFixed(2)}%"></i></span></div>`,
      st.cap_reached ? warn(`The cap for this month is reached. Typed lines are off until ${aiNextMonth(st.month)}. The chips work as before.`) : "");
  }
  let checks = "";
  if (st.key_set || st.fake) {
    const ch = c.checks;
    const running = ch && ch.phase === "wait";
    let extra = "", report = "";
    if (running) extra = `<div class="ai-wide"><span class="ai-pbar"><i></i></span><div class="small">Working…</div></div>`;
    else if (ch && ch.phase === "bad") extra = warn(ch.note);
    if (ch && ch.phase === "done") {
      const under = ch.rows.filter((r) => r.score < r.bar).length;
      report = `<div class="ai-rep">${ch.rows.map((r) => `<div class="ai-rr"><span class="ai-nm">${esc(r.name)}</span><span class="ai-cbar${r.score < r.bar ? " am" : ""}"><i style="width:${(r.score / r.of) * 100}%"></i><u style="left:${(r.bar / r.of) * 100}%"></u></span><span class="ai-sc"><b>${r.score}</b> <small>of ${r.of}</small></span></div>`).join("")}<div class="ai-rf"><span>${under ? `Needs the tick or more. ${under === 1 ? "One row is" : under + " rows are"} under it.` : "Every row is at the tick or above."}</span><span>$${Number(ch.cost || 0).toFixed(2)} · ${Math.round(ch.seconds || 0)} seconds</span></div></div>`;
    }
    checks = row("Run checks", "Sends the check lines through the reader and the writer once. It costs about 10 to 20 cents.", `<button type="button" class="ai-btn" data-act="aichecks"${running ? " disabled" : ""}>${running ? "Running" : "Run checks"}</button>`, extra) + report;
  }
  return head + demo + key + month + checks;
}
// The panel Settings draws (core.js) and the pregame page's dialog holds.
async function aiSettingsPanel() {
  const st = await aiProbe();
  if (!st) return "";
  return `<div class="panel ai-card" data-ai-card>${aiCardHtml(st)}</div>`;
}
function aiCardPaint() {
  document.querySelectorAll("[data-ai-card]").forEach((el) => { el.innerHTML = aiCardHtml(aiState.st); });
}
async function aiCardFresh() { await aiProbe(); aiCardPaint(); }

const aiCardAct = {
  aidemo: (el) => { prefs.aiDemo = el.dataset.v === "1" ? 1 : 0; savePrefs(); aiDevPaint(); aiCardPaint(); },
  aidemooff: () => { prefs.aiDemo = 0; savePrefs(); aiDevPaint(); aiCardPaint(); if (S.route && !outside(S.route)) refresh(); },
  aikeysave: async (el) => {
    const inp = el.closest("[data-ai-card]").querySelector("[data-ai-key]");
    const key = inp ? inp.value.trim() : "";
    if (!key) return;
    el.disabled = true;
    try {
      const r = await api("ai_key_set", { key });
      aiState.card.msg = null;
      aiState.card.test = null;
      if (r && r.keychain_note) toast(r.keychain_note);
    } catch (e) { aiState.card.msg = e.message; }
    await aiCardFresh();
  },
  aikeyclear: async () => {
    try { await api("ai_key_clear"); } catch (e) { toast(e.message); }
    aiState.card = { test: null, checks: null, msg: null };
    await aiCardFresh();
  },
  aikeytest: async () => {
    aiState.card.test = { phase: "wait" };
    aiCardPaint();
    const a = await aiCall("ai_test", {}, () => true, false);
    const t = a || { state: "off", note: "No answer." };
    if (t.state === "done") {
      aiState.card.test = { phase: "ok", text: `Works. One small call, ${Number(t.seconds || 0).toFixed(1)} s.` };
    } else {
      const n = t.note || "The test did not finish.";
      aiState.card.test = { phase: "bad", text: /not accepted/i.test(n) ? "That key did not work. Check it, remove it and paste it again." : /could not be reached|took too long/i.test(n) ? "No connection. Typed lines are off until it comes back. The chips work as before." : n };
    }
    await aiCardFresh();
  },
  aichecks: async () => {
    aiState.card.checks = { phase: "wait" };
    aiCardPaint();
    const a = await aiCall("ai_checks", {}, () => true, "checks");
    if (a && a.state === "done" && a.rows) aiState.card.checks = { phase: "done", rows: a.rows, cost: a.cost, seconds: a.seconds };
    else aiState.card.checks = { phase: "bad", note: (a && a.note) || "The checks did not finish." };
    await aiCardFresh();
  },
};
Object.assign(CORE_MORE, aiCardAct);

// The key's field wakes its button; Enter saves (inside the pregame dialog
// it would close the dialog), and the demo bar's list picks the state.
document.addEventListener("input", (ev) => {
  const k = ev.target.closest && ev.target.closest("[data-ai-key]");
  if (!k) return;
  const b = k.closest("[data-ai-card]").querySelector('[data-act="aikeysave"]');
  if (b) b.disabled = !k.value.trim();
});
document.addEventListener("keydown", (ev) => {
  const k = ev.target.closest && ev.target.closest("[data-ai-key]");
  if (k && ev.key === "Enter") { ev.preventDefault(); ev.stopPropagation(); const b = k.closest("[data-ai-card]").querySelector('[data-act="aikeysave"]'); if (b && !b.disabled) b.click(); }
}, true);
document.addEventListener("change", (ev) => {
  const d = ev.target.closest && ev.target.closest("[data-ai-demo]");
  if (d) { prefs.aiDemoState = d.value; savePrefs(); }
});

// What the pregame page's settings dialog holds under its own lines.
async function aiPregameCard() {
  const p = await aiSettingsPanel();
  return p ? `<div class="ai-pg">${p}</div>` : "";
}

// The first look, once the page is up: is this the fake client?
aiProbe();
