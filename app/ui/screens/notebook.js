"use strict";
/* The notebook (Cain, 2026-11-12, playtest bug 5): an icon in the top bar
 * that opens a panel beside whatever you are looking at. Free notes in
 * categories, kept in a small file beside the save (crates/afm-game/src/
 * notebook.rs). The page keeps working while it is open and slides over to
 * make room (html.nb-open, notebook.css).
 *
 * Commands: notebook, notebook_add, notebook_edit, notebook_category. Each
 * answers with the whole notebook: { total, general, categories: [{ name,
 * count }], notes: [{ id, cat, text, made, edited }] }, newest note first.
 * A note with no category has cat "" and shows as General.
 *
 * The notes on players, coaches and letters (the inbox's "Notes", core.js
 * openNotes) are a separate thing and stay as they were; the panel's foot
 * links to them. */

// The page's hooks: the top bar's icon asks through data-act "notebook"
// (CORE_MORE, core.js).
const NOTEBOOK = { open: false, book: null, cat: null, q: "", armed: null, armTimer: 0 };
// cat: null shows every note, "" shows General, a name shows that category.

// Plain text in a note: a Mac's web view and Chrome both know "plaintext-only".
const NOTEBOOK_EDITABLE = (() => {
  const d = document.createElement("div");
  d.contentEditable = "plaintext-only";
  return d.contentEditable === "plaintext-only" ? "plaintext-only" : "true";
})();

function notebookEl() { return document.querySelector(".notebook"); }

// The icon shows it is open, and the page makes room for it.
function notebookMark() {
  document.documentElement.classList.toggle("nb-open", NOTEBOOK.open);
  const b = document.querySelector('.topbar [data-act="notebook"]');
  if (b) b.setAttribute("aria-pressed", String(NOTEBOOK.open));
}

function notebookClose() {
  clearTimeout(NOTEBOOK.armTimer);
  NOTEBOOK.open = false;
  NOTEBOOK.armed = null;
  const el = notebookEl();
  if (el) {
    // A note you were still writing is kept.
    const a = document.activeElement;
    if (a && a.classList && a.classList.contains("nb-text") && el.contains(a)) notebookSaveText(a);
    el.remove();
  }
  notebookMark();
}

async function notebookToggle() {
  if (NOTEBOOK.open) { notebookClose(); return; }
  // The notes on players and letters sit in the same place: one at a time.
  document.querySelector(".notes-sheet")?.remove();
  NOTEBOOK.open = true;
  notebookMark();
  const el = document.createElement("aside");
  el.className = "notebook";
  el.setAttribute("aria-label", "Notebook");
  el.innerHTML = `<div class="nb-head"><div><div class="st-label">Notebook</div><b class="nb-count"> </b></div><span class="tspace"></span><button class="st-icon-btn" data-nb="close" aria-label="Close the notebook" title="Close (Esc)">×</button></div>
    <div class="nb-compose">
      <textarea class="nb-new" rows="2" placeholder="Write it down, for example: No DBs below 47 man coverage." aria-label="New note"></textarea>
      <div class="nb-compose-row"><select class="nb-cat" aria-label="Category of the new note"></select><span class="tspace"></span><span class="nb-key">⌘↵</span><button class="st-btn small primary" data-nb="add">Add note</button></div>
    </div>
    <div class="nb-find"><span class="sicon">${icon("search")}</span><input class="nb-q" placeholder="Search your notes" aria-label="Search your notes" autocomplete="off" spellcheck="false"></div>
    <div class="nb-chips" role="tablist" aria-label="Categories"></div>
    <div class="nb-catbar"></div>
    <div class="nb-list"></div>
    <div class="nb-foot"><a class="link" data-nb="old">Notes on players and letters ›</a></div>`;
  document.body.appendChild(el);
  try {
    NOTEBOOK.book = await api("notebook");
  } catch (e) {
    toast(e.message);
    notebookClose();
    return;
  }
  // Closed again while it loaded.
  if (!NOTEBOOK.open || notebookEl() !== el) return;
  el.querySelector(".nb-q").value = NOTEBOOK.q;
  notebookDraw();
  el.querySelector(".nb-new").focus();
}

// The words under a category: the count, and "General" for none.
const notebookName = (c) => c || "General";
const notebookPlural = (n, one, many) => `${n} ${n === 1 ? one : many || one + "s"}`;

function notebookOptions(selected) {
  const b = NOTEBOOK.book;
  const opt = (v, t) => `<option value="${esc(v)}"${v === selected ? " selected" : ""}>${esc(t)}</option>`;
  return opt("", "General") + b.categories.map((c) => opt(c.name, c.name)).join("");
}

// Everything on the panel from the book. The new note's box keeps what you
// typed; the list is the only part that is built again for each change.
function notebookDraw() {
  const el = notebookEl();
  const b = NOTEBOOK.book;
  if (!el || !b) return;
  // A category that went (deleted, or joined into another): back to every note.
  if (NOTEBOOK.cat && !b.categories.some((c) => c.name === NOTEBOOK.cat)) NOTEBOOK.cat = null;
  el.querySelector(".nb-count").textContent = notebookPlural(b.total, "note");

  const chip = (cat, label, n) => `<button class="nb-chip" role="tab" data-nb="chip" data-cat="${esc(cat ?? "")}" data-all="${cat === null ? 1 : 0}" aria-selected="${NOTEBOOK.cat === cat}">${esc(label)}<small>${n}</small></button>`;
  el.querySelector(".nb-chips").innerHTML = chip(null, "All", b.total)
    + (b.general ? chip("", "General", b.general) : "")
    + b.categories.map((c) => chip(c.name, c.name, c.count)).join("")
    + '<button class="nb-chip nb-plus" data-nb="newcat" title="New category" aria-label="New category">+</button>';

  const bar = el.querySelector(".nb-catbar");
  const named = NOTEBOOK.cat ? b.categories.find((c) => c.name === NOTEBOOK.cat) : null;
  bar.hidden = !named;
  bar.innerHTML = named ? `<span>${esc(named.name)}</span><span class="tspace"></span><a class="link" data-nb="rename">Rename</a><a class="link" data-nb="delcat">Delete category</a>` : "";

  // The box for a new note files into the category you are looking at.
  const sel = el.querySelector(".nb-cat");
  const keep = NOTEBOOK.cat !== null ? NOTEBOOK.cat : sel.value;
  sel.innerHTML = notebookOptions(b.categories.some((c) => c.name === keep) ? keep : "") + '<option value="\u0001new">New category…</option>';
  sel.dataset.prev = sel.value;

  notebookList();
}

function notebookList() {
  const el = notebookEl();
  const b = NOTEBOOK.book;
  if (!el || !b) return;
  const q = NOTEBOOK.q.trim().toLowerCase();
  const shown = b.notes.filter((n) => (NOTEBOOK.cat === null || n.cat === NOTEBOOK.cat) && (!q || `${n.text} ${notebookName(n.cat)}`.toLowerCase().includes(q)));
  const list = el.querySelector(".nb-list");
  const top = list.scrollTop;
  list.innerHTML = shown.map((n) => `<div class="nb-note" data-id="${n.id}">
      <div class="nb-text" contenteditable="${NOTEBOOK_EDITABLE}" spellcheck="true" role="textbox" aria-multiline="true" aria-label="Note">${esc(n.text)}</div>
      <div class="nb-meta"><select class="nb-move" aria-label="Category of this note" title="Move to another category">${notebookOptions(n.cat)}</select><span class="nb-when">${esc(notebookWhen(n))}</span><span class="tspace"></span><a class="link nb-del" data-nb="del">Delete</a></div>
    </div>`).join("") || `<div class="empty">${
    b.total === 0 ? "Nothing written yet. Put a baseline, a rule of thumb or a name here. It stays with this career."
      : q ? `No note says “${esc(NOTEBOOK.q.trim())}”.`
        : `No notes in ${esc(notebookName(NOTEBOOK.cat))} yet.`}</div>`;
  list.scrollTop = top;
}
const notebookWhen = (n) => (n.made ? n.made : "") + (n.edited ? ` · edited ${n.edited}` : "");

// A new category: asked in the game's own box; answers its name, or null.
async function notebookNewCategory() {
  const name = await ask("New category", { placeholder: "For example: Baselines", ok: "Make category", note: "A category groups notes. Notes with none sit in General." });
  if (!name) return null;
  try {
    NOTEBOOK.book = await api("notebook_category", { op: "add", name });
  } catch (e) { toast(e.message); return null; }
  // The name as the notebook keeps it ("baselines" finds "Baselines").
  const kept = NOTEBOOK.book.categories.find((c) => c.name.toLowerCase() === name.replace(/\s+/g, " ").trim().toLowerCase());
  return kept ? kept.name : null;
}

async function notebookAdd() {
  const el = notebookEl();
  const ta = el.querySelector(".nb-new");
  const text = ta.value.trim();
  if (!text) { ta.focus(); return; }
  const cat = el.querySelector(".nb-cat").value;
  try {
    NOTEBOOK.book = await api("notebook_add", { text, cat });
  } catch (e) { toast(e.message); return; }
  ta.value = "";
  // The note you just wrote is in sight: its category, and no search hiding it.
  if (NOTEBOOK.cat !== null) NOTEBOOK.cat = cat;
  NOTEBOOK.q = "";
  el.querySelector(".nb-q").value = "";
  notebookDraw();
  el.querySelector(".nb-list").scrollTop = 0;
  ta.focus();
}

async function notebookRename() {
  const was = NOTEBOOK.cat;
  if (!was) return;
  const name = await ask("Rename the category", { value: was, ok: "Rename", note: "A name you already use joins the two." });
  if (!name || name === was) return;
  try {
    NOTEBOOK.book = await api("notebook_category", { op: "rename", name: was, to: name });
  } catch (e) { toast(e.message); return; }
  const kept = NOTEBOOK.book.categories.find((c) => c.name.toLowerCase() === name.replace(/\s+/g, " ").trim().toLowerCase());
  NOTEBOOK.cat = kept ? kept.name : null;
  notebookDraw();
}

async function notebookDeleteCategory() {
  const was = NOTEBOOK.cat;
  const c = NOTEBOOK.book.categories.find((x) => x.name === was);
  if (!c) return;
  if (!(await sure(`Delete the category “${c.name}”?`, c.count ? `Its ${notebookPlural(c.count, "note")} stay${c.count === 1 ? "s" : ""}, in General.` : "It has no notes.", "Delete category"))) return;
  try {
    NOTEBOOK.book = await api("notebook_category", { op: "delete", name: was });
  } catch (e) { toast(e.message); return; }
  NOTEBOOK.cat = null;
  notebookDraw();
}

// Delete takes two clicks, so one slip loses nothing.
async function notebookDelete(link) {
  const card = link.closest(".nb-note");
  const id = Number(card.dataset.id);
  if (NOTEBOOK.armed === id) {
    clearTimeout(NOTEBOOK.armTimer);
    NOTEBOOK.armed = null;
    try {
      NOTEBOOK.book = await api("notebook_edit", { id, delete: true });
    } catch (e) { toast(e.message); return; }
    notebookDraw();
    return;
  }
  clearTimeout(NOTEBOOK.armTimer);
  document.querySelectorAll(".nb-del.armed").forEach((a) => { a.classList.remove("armed"); a.textContent = "Delete"; });
  NOTEBOOK.armed = id;
  link.classList.add("armed");
  link.textContent = "Delete it?";
  NOTEBOOK.armTimer = setTimeout(() => {
    if (NOTEBOOK.armed === id) NOTEBOOK.armed = null;
    link.classList.remove("armed");
    link.textContent = "Delete";
  }, 3500);
}

// A note's words are saved when you click away from them.
async function notebookSaveText(box) {
  const card = box.closest(".nb-note");
  const id = Number(card.dataset.id);
  const note = NOTEBOOK.book.notes.find((n) => n.id === id);
  if (!note) return;
  const text = box.innerText.replace(/ /g, " ").trim();
  if (text === note.text) return;
  if (!text) {
    box.textContent = note.text;
    toast("A note can't be empty. Use Delete to remove it.");
    return;
  }
  try {
    NOTEBOOK.book = await api("notebook_edit", { id, text });
  } catch (e) {
    box.textContent = note.text;
    toast(e.message);
    return;
  }
  // Only the date line changes: the box you click into next keeps its focus.
  const now = NOTEBOOK.book.notes.find((n) => n.id === id);
  const when = card.querySelector(".nb-when");
  if (now && when) when.textContent = notebookWhen(now);
}

async function notebookMove(select) {
  const id = Number(select.closest(".nb-note").dataset.id);
  try {
    NOTEBOOK.book = await api("notebook_edit", { id, cat: select.value });
  } catch (e) { toast(e.message); return; }
  notebookDraw();
}

// ---------- clicks and keys inside the panel ----------
document.addEventListener("click", async (ev) => {
  const el = notebookEl();
  if (!el || !el.contains(ev.target)) return;
  const t = ev.target.closest("[data-nb]");
  if (!t) return;
  switch (t.dataset.nb) {
    case "close": notebookClose(); break;
    case "add": notebookAdd(); break;
    case "chip": NOTEBOOK.cat = t.dataset.all === "1" ? null : t.dataset.cat; notebookDraw(); break;
    case "newcat": {
      const made = await notebookNewCategory();
      if (made !== null) NOTEBOOK.cat = made;
      notebookDraw();
      break;
    }
    case "rename": notebookRename(); break;
    case "delcat": notebookDeleteCategory(); break;
    case "del": notebookDelete(t); break;
    case "old": notebookClose(); openNotes(); break;
  }
});
document.addEventListener("input", (ev) => {
  if (ev.target.classList && ev.target.classList.contains("nb-q") && notebookEl()?.contains(ev.target)) {
    NOTEBOOK.q = ev.target.value;
    notebookList();
  }
});
document.addEventListener("change", async (ev) => {
  const el = notebookEl();
  if (!el || !el.contains(ev.target)) return;
  const s = ev.target;
  if (s.classList.contains("nb-move")) notebookMove(s);
  else if (s.classList.contains("nb-cat")) {
    if (s.value !== "\u0001new") { s.dataset.prev = s.value; return; }
    // "New category…" in the new note's box; a cancel puts the last one back.
    const prev = s.dataset.prev || "";
    const made = await notebookNewCategory();
    if (made !== null && NOTEBOOK.cat !== null) NOTEBOOK.cat = made;
    notebookDraw();
    s.value = made !== null ? made : NOTEBOOK.book.categories.some((c) => c.name === prev) ? prev : "";
    s.dataset.prev = s.value;
  }
});
document.addEventListener("focusout", (ev) => {
  if (ev.target.classList && ev.target.classList.contains("nb-text") && notebookEl()?.contains(ev.target)) notebookSaveText(ev.target);
});
document.addEventListener("keydown", (ev) => {
  const el = notebookEl();
  if (!el || !el.contains(ev.target)) return;
  // ⌘↵ or Ctrl+↵ in the new note's box adds it.
  if (ev.key === "Enter" && (ev.metaKey || ev.ctrlKey) && ev.target.classList.contains("nb-new")) { ev.preventDefault(); notebookAdd(); return; }
  // Esc closes the panel, unless a question box is open over it.
  if (ev.key === "Escape" && !ev.defaultPrevented && !document.querySelector(".overlay")) { ev.preventDefault(); notebookClose(); }
});

// What the panel remembers belongs to one career: its notes, the category
// you were looking at and what you searched for.
function notebookForget() {
  NOTEBOOK.book = null;
  NOTEBOOK.cat = null;
  NOTEBOOK.q = "";
}

// The page draws again at every click and the icon is drawn pressed (app.js,
// frame). Leaving the career for the menus closes the panel and forgets all
// of it, as another career has another notebook.
const notebookRender = render;
render = async function (scroll) {
  const r = await notebookRender(scroll);
  if (outside(S.route)) { if (NOTEBOOK.open) notebookClose(); notebookForget(); }
  else if (NOTEBOOK.open) notebookMark();
  return r;
};

Object.assign(CORE_MORE, {
  notebook: () => notebookToggle(),
});
