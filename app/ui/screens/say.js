// Live talks, lane 2: the talk box (docs/reference/how-it-works/live-talks.md,
// "Lane 2, the boxes"). One strip under a talk's chips where you type what you
// would say: "Say it your way", a counter, and a button. Frames 1 to 3 and 8
// of afm/live-talks/lanes/lane-0.md.
//
// A page shows it with one call, aiSayBox(reply, key, options), where the
// reply is the screen's own reply: the box shows only when the reply carries
// `say` (live_talk::after adds it when a key is set and the cap is not
// reached), or when the demo switch is on (ai.js) and the place's `preset`
// stands in. Nothing else on the page changes. The box keeps its own state
// under `key` (ai.js, aiBox), so the page can draw again and find it as it
// was. `options`:
//   preset    {cmd, args, limit}: what the flag would be, for the demo switch
//   words     the strip's own words (title, button, placeholder, foot, ...)
//   wrap      "panel": the strip draws its own card (no empty card when hidden)
//   log       the key of the talk's log, so "Heard as" finds your line there
//   echo      true on a page with no log of its own: your line, how it was
//             heard and what came back sit under the strip
//   apply     async (reply, box): what the page does with the reply the line
//             landed with. Default: draw the page again.
// The strip's states are the contract's answers (ai_poll): ready and typing,
// wait, unsure (two readings to pick from, or one to confirm when the step
// can't be undone), none, off, done.
"use strict";

const AI_SAY_WORDS = {
  title: "Say it your way",
  btn: "Say it",
  ph: "Type what you'd say. The chips above still work.",
  who: "Your staff",
  reading: "is reading what you said.",
  nofollow: "Nothing changed. Your words are still in the box.",
  off: "Typed lines",
  offTail: "The chips work as before.",
  cant: "It can't be taken back once it is done.",
  foot: "",
};

function aiSayBox(x, key, o = {}) {
  const flag = aiFlag(x, "say", o.preset);
  if (!flag) return "";
  const b = aiBox(key, "say", flag, { ...AI_SAY_WORDS, ...(o.words || {}) });
  b.o = o;
  return aiBoxHtml(b);
}

// ---------- drawing a box (the talk box and the ask box share it) ----------
function aiBoxHtml(b) {
  const w = b.w;
  const key = esc(b.key);
  const wrap = b.o && b.o.wrap === "panel" ? " panel" : "";
  const limit = (b.flag && b.flag.limit) || AI_LIMIT;
  const n = b.text.length;
  const title = wrap && b.o.heading ? `<h2>${esc(b.o.heading)}${b.o.sub ? ` <small>${esc(b.o.sub)}</small>` : ""}</h2>` : "";
  const div = (inner, cls) => `<div class="ai-box${wrap}${cls ? " " + cls : ""}" data-ai="${key}" data-aikind="${b.kind}">${title}${inner}</div>`;
  // Off: the strip is replaced by one note, once. After that no box and no gap.
  if (b.phase === "off") {
    if (b.offShown) return "";
    b.offShown = true;
    return div(`<div class="ai-note"><span class="ai-lb">${esc(w.off)}</span><span>are off for now. ${esc(b.note || "")} ${esc(w.offTail)}</span></div>`);
  }
  const head = (left, right) => `<div class="ai-h"><span class="ai-lb">${esc(left)}</span><span class="ai-cnt" ${right == null ? "data-ai-cnt" : ""}>${right == null ? `${n} / ${limit}` : esc(right)}</span></div>`;
  // Unsure: two or more readings to pick from, or the one a step that can't
  // be undone asks about first.
  if (b.phase === "unsure") {
    const rs = b.readings || [];
    if (rs.length < 2) {
      const r = rs[0] || {};
      return div(`<div class="ai-conf"><p class="ai-quote">${esc(b.sent || b.text)}</p>
        <div class="ai-ha"><span class="ai-lb">Heard as</span><span>${esc(r.heard || "")}</span></div>
        <div class="ai-cant"><span class="tag due">Can't be undone</span><span>${esc(r.why || w.cant)}</span></div>
        <div class="ai-acts"><button type="button" class="ai-btn pri" data-act="aipicked" data-ai="${key}">Do it</button><button type="button" class="ai-btn" data-act="aiagain" data-ai="${key}">Not that</button></div>
        <a class="ai-lnk" role="button" tabindex="0" data-act="aiagain" data-ai="${key}">Say it another way</a></div>`, "unsure");
    }
    const rows = rs.map((r, i) => `<button type="button" class="ai-rd${i === b.sel ? " on" : ""}" role="radio" aria-checked="${i === b.sel}" data-act="aipick" data-ai="${key}" data-i="${i}"><i></i><span><span class="ai-lb">Heard as</span>${esc(r.heard || "")}</span></button>`).join("");
    return div(`${head(rs.length === 2 ? "Heard two ways" : "Heard " + rs.length + " ways", "Pick the one you meant")}<div class="ai-rds" role="radiogroup" aria-label="How you were heard">${rows}</div>
      <div class="ai-f"><button type="button" class="ai-btn pri" data-act="aipicked" data-ai="${key}">Say it this way</button><button type="button" class="ai-btn" data-act="aiagain" data-ai="${key}">Say it another way</button></div>`, "unsure");
  }
  const locked = b.phase === "wait";
  const choose = b.flag && b.flag.choose && b.flag.choose.options && b.flag.choose.options.length
    ? `<select class="ai-pick" aria-label="Who you ask" data-ai-choose data-ai="${key}"${locked ? " disabled" : ""}>${b.flag.choose.options.map((op) => `<option value="${esc(op.value)}"${String(op.value) === String(b.choice ?? b.flag.choose.options[0].value) ? " selected" : ""}>${esc(op.label)}</option>`).join("")}</select>` : "";
  const field = `<textarea class="ai-fld${n ? " typed" : ""}${locked ? " lock" : ""}" data-ai-in data-ai="${key}" rows="1" maxlength="${limit}" placeholder="${esc(w.ph)}" aria-label="${esc(w.title)}"${locked ? " readonly" : ""}>${esc(b.text)}</textarea>`;
  const go = locked
    ? `<button type="button" class="ai-btn dim" disabled>${esc(w.btn)}</button>`
    : `<button type="button" class="ai-btn pri" data-act="aisend" data-ai="${key}"${n ? "" : " disabled"}>${esc(w.btn)}</button>`;
  const typing = w.foot && !locked && !choose
    ? `${field}<div class="ai-b"><span class="small">${esc(w.foot)}</span>${go}</div>`
    : `<div class="ai-f">${choose}${field}${go}</div>`;
  let after = "";
  if (locked) after = `<span class="ai-pbar"><i></i></span><div class="ai-note"><span class="ai-lb">${esc(w.who)}</span><span>${esc(w.reading)}</span></div>`;
  else if (b.phase === "none") after = `<div class="ai-note"><b>${esc(b.note || "It doesn't follow.")}</b><span>${esc(w.nofollow)}</span></div>`;
  else if (b.kind === "say" && b.last && (b.o.echo || (b.last.reply && b.last.reply.demo))) after = aiEchoHtml(b);
  else if (b.kind !== "say") after = aiAskAfter(b);
  return div(`${head(w.title)}${typing}${b.kind !== "say" && w.cap ? `<span class="ai-cap">${esc(w.cap)}</span>` : ""}${after}`);
}
// What you just said, how it was heard and what came back, on a page that
// draws no log of its own.
function aiEchoHtml(b) {
  const l = b.last;
  const back = l.reply && typeof l.reply.text === "string" ? `<p class="ai-back">${rich(l.reply.text)}</p>` : "";
  return `<div class="ai-echo"><p class="ai-you">${esc(l.text)}</p><div class="ai-heard">${aiHeardHtml(l)}</div>${back}</div>`;
}

// ---------- what the states do ----------
// A reply to your line lands on the page the way a chip's reply does.
async function aiRedraw(reply) {
  if (reply && reply.done) { try { S.status = await api("status"); } catch (e) { /* the page draws anyway */ } }
  await refresh();
}
async function aiSayDone(b, a, text) {
  const reply = a.reply || {};
  b.phase = "ready";
  b.text = "";
  b.last = { text, heard: a.heard, marks: a.marks, reply };
  aiRemember(b.o && b.o.log ? b.o.log : b.key, text, a.heard, a.marks);
  if (reply.demo || !aiHere(b)) { aiPaint(b); return; }
  if (b.o && b.o.apply) await b.o.apply(reply, b); else await aiRedraw(reply);
}
// An answer arrives: every state but done is shared by both boxes.
async function aiLand(b, a, text) {
  if (!a) return;
  if (a.ticket != null) b.ticket = a.ticket;
  if (a.state === "unsure") { b.phase = "unsure"; b.readings = a.readings || []; b.sel = 0; aiPaint(b); return; }
  if (a.state === "none") { b.phase = "none"; b.note = a.note; aiPaint(b); return; }
  if (a.state === "off") { b.phase = "off"; b.note = a.note; b.offShown = false; aiPaint(b); return; }
  if (a.state === "done") {
    if (b.kind === "say") await aiSayDone(b, a, text); else await aiAskDone(b, a, text);
    return;
  }
  b.phase = "off"; b.note = "The answer could not be used."; b.offShown = false; aiPaint(b);
}
// Sends what is typed: the box's command with its args and the text (and the
// pick of its list), then waits for the ticket.
async function aiSend(b, noText) {
  const text = noText ? "" : b.text.trim();
  if ((!text && !noText) || b.phase === "wait") return;
  b.sent = text;
  b.phase = "wait";
  b.route = S.route.screen;
  const run = ++b.run;
  aiPaint(b);
  const args = { ...(b.flag.args || {}), ...(text ? { text } : {}) };
  if (b.flag.choose) args[b.flag.choose.key] = b.choice ?? b.flag.choose.options[0].value;
  await aiLand(b, await aiCall(b.flag.cmd, args, () => b.run === run), text);
}
// A reading picked: the box sends its command again with the ticket and the pick.
async function aiPicked(b) {
  const r = (b.readings || [])[b.sel] || (b.readings || [])[0];
  if (!r) return;
  b.phase = "wait";
  const run = ++b.run;
  aiPaint(b);
  const args = { ...(b.flag.args || {}), ticket: b.ticket, pick: r.pick };
  if (b.flag.choose) args[b.flag.choose.key] = b.choice ?? b.flag.choose.options[0].value;
  await aiLand(b, await aiCall(b.flag.cmd, args, () => b.run === run, "done"), b.sent || b.text);
}
// Back to the typed words, unchanged: "Say it another way", "Not that".
function aiAgain(b) {
  b.run++;
  b.phase = "ready";
  b.readings = null;
  aiPaint(b, true);
}

// ---------- the box's controls ----------
const aiOf = (el) => aiState.boxes[el && el.dataset && el.dataset.ai];
Object.assign(CORE_MORE, {
  aisend: (el) => { const b = aiOf(el); if (b) aiSend(b, el.dataset.notext === "1"); },
  aipick: (el) => { const b = aiOf(el); if (!b) return; b.sel = Number(el.dataset.i); aiPaint(b); },
  aipicked: (el) => { const b = aiOf(el); if (b) aiPicked(b); },
  aiagain: (el) => { const b = aiOf(el); if (b) aiAgain(b); },
});
document.addEventListener("input", (ev) => {
  const f = ev.target.closest && ev.target.closest("[data-ai-in]");
  if (!f) return;
  const b = aiOf(f);
  if (!b) return;
  b.text = f.value;
  const box = f.closest(".ai-box");
  const cnt = box.querySelector("[data-ai-cnt]");
  if (cnt) cnt.textContent = `${f.value.length} / ${(b.flag && b.flag.limit) || AI_LIMIT}`;
  const btn = box.querySelector('[data-act="aisend"]');
  if (btn) btn.disabled = !f.value.trim();
  f.classList.toggle("typed", !!f.value);
  f.style.height = "auto";
  f.style.height = Math.min(f.scrollHeight + 2, 140) + "px";
});
document.addEventListener("change", (ev) => {
  const c = ev.target.closest && ev.target.closest("[data-ai-choose]");
  const b = c && aiOf(c);
  if (b) b.choice = c.value;
});
// Enter sends; Shift+Enter is a new line. The keys the page listens for
// (Space is Continue, ← → move through the mail) never reach a field.
document.addEventListener("keydown", (ev) => {
  const f = ev.target.closest && ev.target.closest("[data-ai-in]");
  if (!f) return;
  if (!ev.metaKey && !ev.ctrlKey && ev.key !== "Escape") ev.stopPropagation();
  if (ev.key === "Enter" && !ev.shiftKey && !ev.metaKey && !ev.isComposing) {
    ev.preventDefault();
    const b = aiOf(f);
    if (b) aiSend(b);
  }
}, true);
document.addEventListener("keydown", (ev) => {
  const a = ev.target.closest && ev.target.closest('a[data-act="aiagain"]');
  if (a && (ev.key === "Enter" || ev.key === " ")) { ev.preventDefault(); a.click(); }
});
