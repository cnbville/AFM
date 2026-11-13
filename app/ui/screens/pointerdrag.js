// Picking a man up and putting him down with the mouse (playtest bug 14).
//
// This is drawn from plain pointer events, not the browser's own drag and
// drop. In the Mac app (WKWebView inside Tauri) the native drag started and
// followed the mouse, but the page never got the drop, so a release did
// nothing, even inside a position group (PR #304 had only been tried in
// Chromium). Pointer events reach the page the same way in every webview.
//
// A screen registers a kind with `pdragKind` and this file does the rest:
//   source   selector of what can be picked up (give it `data-drag`)
//   target   selector of what can take it
//   over     the class a target wears while the held man is over it
//   payload  (sourceEl) => what is held ({ name, ... }), or null to refuse
//   why      (targetEl, payload) => null when the target takes him, else the
//            plain reason it can't (shown to the person)
//   begin / end   (payload, sourceEl) / () — screen colours while a man is held
//   drop     async (targetEl, payload) => the save
// A drop that can't save always says why in a toast: it is never silent.
"use strict";

const PD = { kinds: [], hold: null, live: null, hush: false };
// How far the mouse moves (px) before a press becomes a drag, so a click on
// a spot still just picks it.
const PD_MOVE = 5;

function pdragKind(kind) {
  PD.kinds.push(kind);
}

function pdragTarget(kind, x, y) {
  const under = document.elementFromPoint(x, y);
  return under && under.closest ? under.closest(kind.target) : null;
}

// The nearest box that scrolls up and down around an element (the page's
// `.view`), else the page itself.
function pdragScroller(el) {
  for (let e = el.parentElement; e && e !== document.body; e = e.parentElement) {
    const o = getComputedStyle(e).overflowY;
    if ((o === "auto" || o === "scroll") && e.scrollHeight > e.clientHeight + 1) return e;
  }
  return document.scrollingElement || document.documentElement;
}

function pdragBegin(h) {
  const ghost = document.createElement("div");
  ghost.className = "pd-ghost";
  ghost.textContent = h.payload.name || "";
  document.body.appendChild(ghost);
  document.body.classList.add("pd-active");
  h.el.classList.add("pd-src");
  PD.live = { ...h, ghost, over: null, x: h.x, y: h.y, scroller: pdragScroller(h.el), raf: 0 };
  if (h.kind.begin) h.kind.begin(h.payload, h.el);
  PD.live.raf = requestAnimationFrame(pdragEdge);
}

// Near the top or bottom of the box that scrolls, it scrolls while the man
// is held, so a spot or a row that isn't on screen yet can be reached (the
// browser's own drag did this for us; a depth beside the field sits well
// below the stadium).
function pdragEdge() {
  const L = PD.live;
  if (!L) return;
  const sp = L.scroller;
  const page = sp === document.scrollingElement || sp === document.documentElement;
  const box = page ? { top: 0, bottom: window.innerHeight } : sp.getBoundingClientRect();
  const top = Math.max(box.top, 0), bottom = Math.min(box.bottom, window.innerHeight);
  const zone = Math.min(90, (bottom - top) / 4);
  let dy = 0;
  if (L.y < top + zone) dy = -Math.ceil(((top + zone - L.y) / zone) * 24);
  else if (L.y > bottom - zone) dy = Math.ceil(((L.y - (bottom - zone)) / zone) * 24);
  if (dy) {
    sp.scrollTop += dy;
    pdragHover(L.x, L.y);
  }
  L.raf = requestAnimationFrame(pdragEdge);
}

function pdragMove(ev) {
  const L = PD.live;
  L.x = ev.clientX;
  L.y = ev.clientY;
  L.ghost.style.transform = `translate(${ev.clientX + 14}px, ${ev.clientY + 16}px)`;
  pdragHover(ev.clientX, ev.clientY);
}

// What the held man is over now: it lights up if it takes him.
function pdragHover(x, y) {
  const L = PD.live;
  const t = pdragTarget(L.kind, x, y);
  if (t !== L.over) {
    if (L.over) L.over.classList.remove(L.kind.over);
    L.over = t;
  }
  const ok = !!t && !L.kind.why(t, L.payload);
  if (t) t.classList.toggle(L.kind.over, ok);
  L.ghost.classList.toggle("no", !!t && !ok);
}

// Ends a hold or a drag and puts the screen back. `swallow` keeps the click
// the browser sends after a drag from also picking the spot under it.
function pdragEnd(swallow) {
  const L = PD.live;
  PD.hold = null;
  PD.live = null;
  if (!L) return;
  cancelAnimationFrame(L.raf);
  L.ghost.remove();
  document.body.classList.remove("pd-active");
  L.el.classList.remove("pd-src");
  if (L.over) L.over.classList.remove(L.kind.over);
  if (L.kind.end) L.kind.end();
  if (swallow) {
    PD.hush = true;
    setTimeout(() => { PD.hush = false; }, 80);
  }
}

document.addEventListener("pointerdown", (ev) => {
  pdragEnd(false);
  if (ev.button !== 0 || ev.isPrimary === false || !ev.target.closest) return;
  for (const kind of PD.kinds) {
    const el = ev.target.closest(kind.source);
    if (!el) continue;
    // A button or field inside a source (the "move up" arrow) keeps its click.
    const inner = ev.target.closest("button, input, select, textarea, label");
    if (inner && inner !== el && el.contains(inner)) return;
    const payload = kind.payload(el);
    if (!payload) return;
    PD.hold = { kind, el, payload, id: ev.pointerId, x: ev.clientX, y: ev.clientY };
    return;
  }
});

document.addEventListener("pointermove", (ev) => {
  const h = PD.hold;
  if (!h || ev.pointerId !== h.id) return;
  // The button came up where the page couldn't see it (outside the window).
  if (ev.pointerType === "mouse" && !(ev.buttons & 1)) return pdragEnd(false);
  if (!PD.live) {
    if (Math.hypot(ev.clientX - h.x, ev.clientY - h.y) < PD_MOVE) return;
    pdragBegin(h);
  }
  pdragMove(ev);
});

document.addEventListener("pointerup", async (ev) => {
  const h = PD.hold;
  if (!h || ev.pointerId !== h.id) return;
  const L = PD.live;
  if (!L) {
    PD.hold = null; // a click, not a drag
    return;
  }
  const { kind, payload } = L;
  const t = pdragTarget(kind, ev.clientX, ev.clientY);
  pdragEnd(true);
  if (!t) {
    toast(kind.nowhere || "Nothing changed: drop him on a spot to move him.");
    return;
  }
  const why = kind.why(t, payload);
  if (why) {
    toast(why);
    return;
  }
  try {
    await kind.drop(t, payload);
  } catch (e) {
    toast((e && e.message) || String(e) || "That drop could not be saved.");
  }
});

document.addEventListener("pointercancel", () => pdragEnd(false));
document.addEventListener("keydown", (ev) => {
  if (ev.key === "Escape" && (PD.live || PD.hold)) pdragEnd(true);
});
window.addEventListener("blur", () => pdragEnd(false));
// The click that follows a drag must not also pick the spot under the mouse.
document.addEventListener("click", (ev) => {
  if (!PD.hush) return;
  ev.stopImmediatePropagation();
  ev.preventDefault();
}, true);
// The browser's own drag of a link or picture inside a source would end this
// one, and a drag must not select text.
document.addEventListener("dragstart", (ev) => {
  const t = ev.target;
  if (t && t.closest && PD.kinds.some((k) => t.closest(k.source))) ev.preventDefault();
}, true);
document.addEventListener("selectstart", (ev) => {
  if (PD.live) ev.preventDefault();
});
