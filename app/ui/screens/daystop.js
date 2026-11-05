// The section stop's card (the six-part day, docs/design/day-sections.md
// topic 5; layout B, agreed 2026-11-05): each Continue stops at a section's
// first minute (5 AM, 8 AM, 1 PM, 4 PM, 7 PM, 10 PM, your local time) or at
// a live moment inside it. The card drops in under the top bar's date, over
// the screen you are on: what came since the last stop, what you're doing
// now and what's due before the next stop. A click anywhere closes it.
"use strict";

const DS_SECTIONS = ["5 AM", "8 AM", "1 PM", "4 PM", "7 PM", "10 PM"];
const DS_X = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"><path d="M6 6l12 12M18 6L6 18"/></svg>';

function dsRail(c) {
  const at = c.section;
  const rail = DS_SECTIONS.map((s, i) => `<span class="${i < at ? "past" : i === at ? "now" : ""}">${c.mark && c.mark.sec === i ? `<em style="left:${c.mark.pct}%"></em>` : ""}</span>`).join("");
  const hours = DS_SECTIONS.map((s, i) => (i === at ? `<span><b>${s}</b></span>` : `<span>${s}</span>`)).join("");
  const mark = c.mark ? `<span class="ds-mark"><i></i>${esc(c.mark.label)}</span>` : "";
  return `<div class="ds-day" aria-label="Your day in six sections"><div class="ds-dtop"><span class="st-label">Your day</span>${mark}</div><div class="ds-rail">${rail}</div><div class="ds-hours">${hours}</div></div>`;
}

function dsGo(r) {
  if (!r.go) return "<span></span>";
  if (r.go.mail) return `<button class="ds-go link" data-act="openmail" data-id="${esc(r.go.mail)}">${esc(r.go.label)} ›</button>`;
  return `<a class="ds-go link" data-go="${esc(r.go.screen)}" data-args='${esc(JSON.stringify(r.go.args || {}))}'>${esc(r.go.label)} ›</a>`;
}

function dsCard(c) {
  const rows = c.items.map((r) => `<div class="ds-row${r.mine ? " mine" : ""}${r.dl ? " dl" : ""}"><span class="ds-t">${esc(r.t)}</span><span class="ds-k">${esc(r.k)}</span><span class="ds-x">${rich(r.x)}</span>${dsGo(r)}</div>`).join("");
  const none = `<div class="ds-row"><span class="ds-t"></span><span class="ds-k"></span><span class="ds-x">Nothing new for you since ${esc(c.since)}.</span><span></span></div>`;
  const due = c.due.length
    ? c.due.map((d) => `<span class="ds-d"><b>${esc(d.when)}</b>${d.tag ? `<span class="tag gold">${esc(d.tag)}</span> ` : ""}${rich(d.what)}</span>`).join("")
    : `<span class="ds-none">${esc(c.due_none)}</span>${c.next ? `<span class="ds-next">Next: <b>${esc(c.next.when)}</b>, ${rich(c.next.what)}</span>` : ""}`;
  const links = `<div class="ds-links"><a class="gochip" data-go="inbox">Inbox</a><a class="gochip" data-go="news">News</a><a class="gochip" data-go="calendar">Today, hour by hour</a><span class="ds-hint">Click anywhere to close</span></div>`;
  const head = c.live ? `Live · ${esc(c.live)}` : `Section stop · ${esc(c.stop)} to ${esc(c.next_stop)}`;
  return `<section class="ds-card" aria-label="Since ${esc(c.since)}">
    <button class="ds-close" data-ds="close" aria-label="Close">${DS_X}</button>
    <header class="ds-head">
      <div class="ds-clock"><div class="st-label"><i></i>${head}</div><h2>${esc(c.stop)} <small>${esc(c.date)}</small></h2></div>
      ${dsRail(c)}
    </header>
    <div class="ds-since"><div class="ds-sh"><b>Since ${esc(c.since)}</b><small>${c.items.length + (c.more || 0)} new · in clock order</small></div><div class="ds-rows">${rows || none}</div>${c.more ? `<div class="ds-more">And ${c.more} more on the calendar and in the news.</div>` : ""}</div>
    <div class="ds-under">
      <div class="ds-line now"><span class="st-label">Now</span><p>${rich(c.now)}</p></div>
      <div class="ds-line"><span class="st-label">Due before ${esc(c.next_stop)}</span><div class="ds-due">${due}</div></div>
    </div>
    ${links}
  </section>`;
}

// Shows a stop's card; it stays up while the screen under it draws again.
function dsShow(card) {
  S.stopCard = card || null;
  dsDraw();
}

// Puts the card back after the frame draws (app.js render).
function dsDraw() {
  const main = $(".main");
  if (!main) return;
  main.querySelectorAll(".ds-drop, .ds-scrim").forEach((el) => el.remove());
  const d = $(".topbar .datebox");
  if (d) d.classList.toggle("ds-on", !!S.stopCard);
  if (!S.stopCard) return;
  main.insertAdjacentHTML("beforeend", `<div class="ds-scrim" data-ds="close"></div><div class="ds-drop" role="dialog" aria-label="Section stop"><span class="ds-notch"></span>${dsCard(S.stopCard)}</div>`);
}

function dsClose() {
  S.stopCard = null;
  dsDraw();
}

// A click anywhere closes it: the cross, the dimmed screen, a link in it
// (which then opens its screen), or anywhere outside it.
document.addEventListener("click", (ev) => {
  if (!S.stopCard) return;
  if (ev.target.closest("[data-act=continue]")) return;
  const inside = ev.target.closest(".ds-drop");
  if (inside && !ev.target.closest("[data-ds=close], a, button")) return;
  dsClose();
}, true);
document.addEventListener("keydown", (ev) => {
  if (ev.key === "Escape" && S.stopCard) dsClose();
});
