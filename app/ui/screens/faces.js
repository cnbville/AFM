// Lane screens: faces (docs/final-build/plan.md; faces F7-F11, F14, S4, X4).
// - The player page's portrait in the hero's round face slot: your photo, or
//   the silhouette with his initials until a drawn face exists; a quiet row
//   with "Change photo" (a crop box), "Copy AI prompt", "Remove your photo",
//   the "new team photo" note and his looks.
// - Small round faces in the squad, depth chart, recruiting, training and
//   contracts lists.
// - The "Faces and photos" page: a folder import with a review table and
//   undo, "Export AI prompts", one file per player, and the drawn heads switch.
// - Drawn heads (F14, on unless that switch turns them off): a face with no
//   photo is drawn in the page from his recipe the first time a screen shows
//   it (screens/heads/heads.js), then kept in the save's folder.
"use strict";
(() => {
  const LISTS = new Set(["roster", "depth", "recruiting", "training", "contracts"]);
  const cache = new Map(); // id -> face for lists
  const waiting = new Set();

  // ---------- pictures ----------
  function silhouette(f, cls) {
    const [c1, c2] = f.colors || ["#3a3f4b", "#9aa3b2"];
    const id = "fg" + Math.random().toString(36).slice(2, 8);
    return `<svg class="${cls}" viewBox="0 0 160 200" aria-label="${esc(f.initials || "")}" role="img">
      <defs><linearGradient id="${id}" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${c1}" stop-opacity=".95"/><stop offset="1" stop-color="${c1}" stop-opacity=".55"/></linearGradient></defs>
      <rect width="160" height="200" fill="#16181d"/><rect width="160" height="200" fill="url(#${id})" opacity=".55"/>
      <path d="M80 30c-19 0-33 15-33 37 0 15 6 28 15 35v10c-21 4-40 12-50 22-6 6-8 14-8 22v44h152v-44c0-8-2-16-8-22-10-10-29-18-50-22v-10c9-7 15-20 15-35 0-22-14-37-33-37z" fill="rgba(0,0,0,.42)"/>
      <path d="M8 170c0-8 2-16 8-22 10-10 29-18 50-22h28c21 4 40 12 50 22 6 6 8 14 8 22v30H8z" fill="${c2}" opacity=".35"/>
      <text x="80" y="181" text-anchor="middle" font-size="26" font-weight="800" fill="#fff" fill-opacity=".92" letter-spacing="1">${esc(f.initials || "")}</text></svg>`;
  }
  function dot(f) {
    if (f.photo) return `<img class="fc-dot" src="${f.photo}" alt="">`;
    // a drawn head is a head-and-shoulders picture: the dot shows his face
    if (f.drawn) return `<span class="fc-dot fc-dz"><img src="${f.drawn}" alt=""></span>`;
    const [c1, c2] = f.colors || ["#3a3f4b", "#9aa3b2"];
    return `<span class="fc-dot fc-ini" style="background:${c1};box-shadow:inset 0 0 0 2px ${c2}66">${esc(f.initials || "")}</span>`;
  }

  // ---------- drawn heads ----------
  // The drawing code and its data (about 20 MB) load the first time a head is
  // drawn; heads are drawn one at a time, newest asks first, so a long list
  // never holds up the screen. A computer without WebGL keeps the silhouettes.
  let headsP = null, broken = false;
  const loadScript = (src) => new Promise((res, rej) => {
    const el = document.createElement("script");
    el.src = src;
    el.onload = res;
    el.onerror = () => rej(new Error("missing " + src));
    document.head.appendChild(el);
  });
  function heads() {
    if (window.AFMHeads) return Promise.resolve(window.AFMHeads);
    headsP ||= (window.THREE ? Promise.resolve() : loadScript("screens/views/three.min.js")).then(() => loadScript("screens/heads/heads.js")).then(() => window.AFMHeads);
    return headsP;
  }
  const drawing = new Map(); // id -> promise of the drawn picture
  function drawHead(id, key, spec) {
    if (broken || !spec) return Promise.resolve(null);
    if (!drawing.has(id)) {
      drawing.set(id, heads()
        .then((H) => H.queue(spec))
        .then(async (r) => {
          await api("face_drawn_store", { id, key: key || "", data: r.url }).catch(() => {});
          return r.url;
        })
        .catch((e) => {
          broken = true;
          console.warn("drawn heads off for this session:", e);
          return null;
        }));
    }
    return drawing.get(id);
  }

  // ---------- the player page ----------
  // His picture goes in the hero's round face slot; the photo tools sit in a
  // quiet row under the page's own buttons.
  async function portrait(hero, id) {
    hero.classList.add("fc-has");
    const box = document.createElement("div");
    box.className = "fc-portrait";
    box.dataset.id = id;
    const copy = hero.querySelector(".copy") || hero;
    copy.appendChild(box);
    try {
      const f = await api("face", { id });
      fill(box, f);
    } catch (e) {
      box.remove();
    }
  }
  function fill(box, f) {
    if (!box.isConnected) return;
    const mine = f.mine && f.mine.mode !== "team" ? f.mine : null;
    const src = (mine && mine.data) || f.drawn;
    const round = box.closest(".hero").querySelector(".phero-in > .face");
    if (round) {
      if (!round._sil) round._sil = round.innerHTML;
      round.innerHTML = src ? `<img class="fc-round" src="${src}" alt="${esc(f.name)}">` : round._sil;
      round.classList.toggle("fc-pic", !!src);
      round.title = f.heritage_words || "";
    }
    const cap = mine
      ? `Your photo${mine.set ? ` · ${esc(mine.set)}` : ""}`
      : f.photo && src
        ? `${esc(f.team || "")} photo · ${f.photo.joined ? "joined" : f.photo.why} ${f.photo.year}`
        : "No picture yet";
    const note = f.mine && f.mine.new_team_photo && f.mine.mode !== "team"
      ? `<span class="fp-note">New team photo (${esc(f.photo.why)} ${f.photo.year}). <a data-fact="team" data-id="${f.id}">Show it</a></span>`
      : f.mine && f.mine.mode === "team"
        ? `<span class="fp-note"><a data-fact="mine" data-id="${f.id}">Show your photo again</a></span>`
        : "";
    box.innerHTML = `<span class="fp-cap">${cap}</span>${note}
      <button class="fc-link" data-fact="change" data-id="${f.id}">Change photo</button>
      <button class="fc-link" data-fact="prompt" data-id="${f.id}" title="An image prompt from his looks, for your own AI pictures">Copy AI prompt</button>
      ${f.mine ? `<button class="fc-link" data-fact="clear" data-id="${f.id}">Remove your photo</button>` : ""}
      <details class="fc-looks"><summary>Looks</summary><div>${[f.looks.hair, f.looks.beard, ...f.looks.marks.map((m) => m[0].toUpperCase() + m.slice(1)), `${f.looks.build[0].toUpperCase()}${f.looks.build.slice(1)} build, ${f.looks.skin}, ${f.looks.eyes}`].map((t) => `<span>${esc(t)}</span>`).join("")}</div></details>`;
    box._face = f;
    if (!src && f.drawn_on && f.spec) {
      drawHead(f.id, f.photo ? f.photo.key : "", f.spec).then((url) => {
        if (url && box.isConnected) fill(box, { ...f, drawn: url, spec: null });
      });
    }
  }
  async function refresh(id) {
    cache.delete(id);
    const box = document.querySelector(`.fc-portrait[data-id="${id}"]`);
    if (box) fill(box, await api("face", { id }));
  }

  // ---------- small faces in lists ----------
  function decorate(view) {
    const links = [...view.querySelectorAll(".tbl td a.link.name[data-go=player]:not([data-fc])")];
    if (!links.length) return;
    const need = [];
    for (const a of links) {
      a.dataset.fc = "1";
      const id = Number(a.dataset.id);
      const span = document.createElement("span");
      span.className = "fc-slot";
      span.dataset.faceId = id;
      const f = cache.get(id);
      if (f) span.innerHTML = dot(f);
      else if (!waiting.has(id)) need.push(id);
      a.parentNode.insertBefore(span, a);
    }
    if (need.length) load(need);
  }
  async function load(ids) {
    ids.forEach((i) => waiting.add(i));
    for (let k = 0; k < ids.length; k += 200) {
      const part = ids.slice(k, k + 200);
      try {
        const x = await api("faces", { ids: part });
        for (const f of x.faces) {
          cache.set(f.id, f);
          document.querySelectorAll(`.fc-slot[data-face-id="${f.id}"]`).forEach((s) => (s.innerHTML = dot(f)));
          if (f.spec) {
            drawHead(f.id, f.key, f.spec).then((url) => {
              if (!url) return;
              const g = { ...f, drawn: url, spec: null };
              cache.set(f.id, g);
              document.querySelectorAll(`.fc-slot[data-face-id="${f.id}"]`).forEach((s) => (s.innerHTML = dot(g)));
            });
          }
        }
      } catch (e) { /* the list still works without faces */ }
      part.forEach((i) => waiting.delete(i));
    }
  }

  // ---------- picking and cropping a photo ----------
  function pickFile(accept, dir) {
    return new Promise((done) => {
      const inp = document.createElement("input");
      inp.type = "file";
      inp.accept = accept;
      if (dir) { inp.webkitdirectory = true; inp.multiple = true; }
      inp.style.display = "none";
      inp.addEventListener("change", () => { done(dir ? [...inp.files] : inp.files[0] || null); inp.remove(); });
      inp.addEventListener("cancel", () => { done(null); inp.remove(); });
      document.body.appendChild(inp);
      inp.click();
    });
  }
  const ACCEPT = "image/png,image/jpeg,image/webp,image/heic,image/heif,.heic,.heif,.png,.jpg,.jpeg,.webp";
  function loadImage(file) {
    return new Promise((done, fail) => {
      const url = URL.createObjectURL(file);
      const img = new Image();
      img.onload = () => done(img);
      img.onerror = () => {
        URL.revokeObjectURL(url);
        fail(new Error(/heic|heif/i.test(file.name) ? "This computer can't read HEIC pictures here. Export it as JPEG first (Photos: File, Export)." : `Can't read ${file.name} as a picture.`));
      };
      img.src = url;
    });
  }
  // Where the face probably is: the middle of the skin-coloured part of the
  // picture's upper three quarters (YCbCr skin rule), for the first crop box.
  function faceGuess(img) {
    const W = 96, H = Math.max(1, Math.round((96 * img.naturalHeight) / img.naturalWidth));
    const c = document.createElement("canvas");
    c.width = W; c.height = H;
    const g = c.getContext("2d", { willReadFrequently: true });
    g.drawImage(img, 0, 0, W, H);
    const d = g.getImageData(0, 0, W, H).data;
    let sx = 0, sy = 0, n = 0;
    for (let y = 0; y < H * 0.75; y++) for (let x = 0; x < W; x++) {
      const i = (y * W + x) * 4, r = d[i], gg = d[i + 1], b = d[i + 2];
      const cb = 128 - 0.169 * r - 0.331 * gg + 0.5 * b, cr = 128 + 0.5 * r - 0.419 * gg - 0.081 * b;
      if (cb > 77 && cb < 127 && cr > 133 && cr < 173) { sx += x; sy += y; n++; }
    }
    if (n < W * H * 0.01) return { x: 0.5, y: 0.38 };
    return { x: sx / n / W, y: sy / n / H };
  }
  // The biggest 4:5 box with the face a little above its middle.
  function autoBox(img) {
    const iw = img.naturalWidth, ih = img.naturalHeight;
    const h = Math.min(ih, (iw * 5) / 4), w = (h * 4) / 5;
    const g = faceGuess(img);
    const x = Math.max(0, Math.min(iw - w, g.x * iw - w / 2));
    const y = Math.max(0, Math.min(ih - h, g.y * ih - h * 0.42));
    return { x, y, w, h };
  }
  function encode(img, b, maxW = 640) {
    const w = Math.min(maxW, Math.round(b.w)), h = Math.round((w * 5) / 4);
    const c = document.createElement("canvas");
    c.width = w; c.height = h;
    const g = c.getContext("2d");
    g.imageSmoothingQuality = "high";
    g.drawImage(img, b.x, b.y, b.w, b.h, 0, 0, w, h);
    return c.toDataURL("image/jpeg", 0.9);
  }
  async function changePhoto(id) {
    const file = await pickFile(ACCEPT);
    if (!file) return;
    let img;
    try { img = await loadImage(file); } catch (e) { toast(e.message); return; }
    const box = autoBox(img);
    const o = document.createElement("div");
    o.className = "overlay";
    o.innerHTML = `<div class="box fc-crop"><b>Crop his photo</b><small class="sub">Drag the box onto his face; the corner handle resizes it. The photo is copied into this save's folder and never leaves your computer.</small>
      <div class="fc-stage"><img alt=""><div class="fc-box"><i></i></div></div>
      <div class="ask-actions"><button class="st-btn glass" data-c="no">Cancel</button><button class="st-btn primary" data-c="ok">Use this photo</button></div></div>`;
    document.body.appendChild(o);
    const stage = o.querySelector(".fc-stage"), pic = stage.querySelector("img"), bx = stage.querySelector(".fc-box");
    pic.src = img.src;
    const scale = () => pic.clientWidth / img.naturalWidth;
    const draw = () => {
      const s = scale();
      Object.assign(bx.style, { left: box.x * s + "px", top: box.y * s + "px", width: box.w * s + "px", height: box.h * s + "px" });
    };
    pic.onload = draw;
    if (pic.complete) requestAnimationFrame(draw);
    let drag = null;
    bx.addEventListener("pointerdown", (ev) => {
      ev.preventDefault();
      bx.setPointerCapture(ev.pointerId);
      drag = { mode: ev.target.tagName === "I" ? "size" : "move", x: ev.clientX, y: ev.clientY, b: { ...box } };
    });
    bx.addEventListener("pointermove", (ev) => {
      if (!drag) return;
      const s = scale(), dx = (ev.clientX - drag.x) / s, dy = (ev.clientY - drag.y) / s;
      const iw = img.naturalWidth, ih = img.naturalHeight;
      if (drag.mode === "move") {
        box.x = Math.max(0, Math.min(iw - box.w, drag.b.x + dx));
        box.y = Math.max(0, Math.min(ih - box.h, drag.b.y + dy));
      } else {
        let w = Math.max(60, drag.b.w + Math.max(dx, (dy * 4) / 5));
        w = Math.min(w, iw - drag.b.x, ((ih - drag.b.y) * 4) / 5);
        box.w = w; box.h = (w * 5) / 4;
      }
      draw();
    });
    bx.addEventListener("pointerup", () => (drag = null));
    const end = () => { o.remove(); URL.revokeObjectURL(img.src); };
    o.querySelector("[data-c=no]").onclick = end;
    o.querySelector("[data-c=ok]").onclick = async () => {
      try {
        const f = await api("face_photo_set", { id, data: encode(img, box) });
        end();
        cache.delete(id);
        const el = document.querySelector(`.fc-portrait[data-id="${id}"]`);
        if (el) fill(el, f);
        toast("Photo set. It shows everywhere he appears.");
      } catch (e) { toast(e.message); }
    };
  }

  async function copyText(text) {
    try { await navigator.clipboard.writeText(text); return true; } catch (e) { /* fall back below */ }
    const t = document.createElement("textarea");
    t.value = text;
    t.style.position = "fixed"; t.style.opacity = "0";
    document.body.appendChild(t);
    t.select();
    let ok = false;
    try { ok = document.execCommand("copy"); } catch (e) { ok = false; }
    t.remove();
    return ok;
  }
  async function copyPrompt(id) {
    const box = document.querySelector(`.fc-portrait[data-id="${id}"]`);
    const text = box && box._face ? box._face.prompt : (await api("face_prompt", { id })).prompt;
    if (await copyText(text)) { toast("AI prompt copied."); return; }
    const { f } = dialog(`<b>AI prompt</b><small class="sub">Copy it into your image tool.</small><textarea rows="9" readonly>${esc(text)}</textarea>`, "Done", () => true);
    f.querySelector("textarea").select();
  }

  document.addEventListener("click", async (ev) => {
    const el = ev.target.closest("[data-fact]");
    if (!el) return;
    ev.preventDefault();
    const id = Number(el.dataset.id);
    const act = el.dataset.fact;
    try {
      if (act === "change") await changePhoto(id);
      else if (act === "prompt") await copyPrompt(id);
      else if (act === "clear") {
        if (!(await sure("Remove your photo?", "Your photo of him is deleted from this save's folder, and his team picture shows again.", "Remove photo"))) return;
        const f = await api("face_photo_clear", { id });
        cache.delete(id);
        fill(document.querySelector(`.fc-portrait[data-id="${id}"]`), f);
      } else if (act === "team" || act === "mine") {
        const f = await api("face_photo_mode", { id, mode: act === "team" ? "team" : "photo" });
        cache.delete(id);
        fill(document.querySelector(`.fc-portrait[data-id="${id}"]`), f);
      } else if (act === "faces-page") go("faces", el.dataset.kind ? { kind: el.dataset.kind, id: Number(el.dataset.tid) } : {});
      else if (act === "import") await importFolder(el.dataset.dir === "1");
      else if (act === "import-go") await importApply();
      else if (act === "import-cancel") {
        IMP.rows.forEach((r) => URL.revokeObjectURL(r.url));
        IMP.rows = null;
        render();
      }
      else if (act === "undo") {
        if (!(await sure("Undo this import?", "Every photo it set is removed, and any photo it replaced comes back.", "Undo import"))) return;
        const x = await api("faces_import_undo", { batch: Number(el.dataset.batch) });
        cache.clear();
        toast(`${x.undone} photos undone.`);
        render();
      } else if (act === "export") await exportPrompts();
      else if (act === "reveal") await api("reveal", { path: el.dataset.path });
      else if (act === "drawn") {
        const on = el.dataset.on !== "1";
        await api("face_settings_set", { drawn: on });
        cache.clear();
        drawing.clear();
        broken = false;
        toast(on ? "Drawn heads on. Each face is drawn the first time a screen shows it." : "Drawn heads off. Silhouettes show again.");
        render();
      }
    } catch (e) { toast(e.message || String(e)); }
  });

  // ---------- the Faces and photos page ----------
  const IMP = { rows: null };
  const HOW = { id: "By id", name_team: "By name and team", name_year: "By name and year", name: "By name", ambiguous: "Pick one", none: "No match" };
  async function importFolder(dir) {
    const files = dir ? await pickFile(ACCEPT, true) : await new Promise((done) => {
      const inp = document.createElement("input");
      inp.type = "file"; inp.accept = ACCEPT; inp.multiple = true; inp.style.display = "none";
      inp.addEventListener("change", () => { done([...inp.files]); inp.remove(); });
      document.body.appendChild(inp); inp.click();
    });
    const list = (files || []).filter((f) => /\.(png|jpe?g|webp|heic|heif)$/i.test(f.name));
    if (!list.length) { if (files) toast("No pictures in that folder."); return; }
    const x = await api("faces_import_match", { files: list.map((f) => f.name) });
    IMP.rows = x.files.map((m, i) => ({ ...m, file: list[i], url: URL.createObjectURL(list[i]), pick: m.match ? m.match.id : null, on: !!m.match }));
    render();
  }
  async function importApply() {
    const rows = IMP.rows.filter((r) => r.on && r.pick);
    if (!rows.length) { toast("Nothing picked to import."); return; }
    showBusy(`Cropping and keeping ${rows.length} photos…`);
    let batch = null, kept = 0, bad = [];
    try {
      for (let k = 0; k < rows.length; k += 8) {
        const items = [];
        for (const r of rows.slice(k, k + 8)) {
          try { const img = await loadImage(r.file); items.push({ id: r.pick, data: encode(img, autoBox(img)) }); URL.revokeObjectURL(img.src); } catch (e) { bad.push(r.file.name); }
        }
        const x = await api("faces_import_apply", batch ? { batch, items } : { items });
        batch = x.batch;
        kept += x.kept;
        bad = bad.concat(x.errors.map((e) => String(e.id)));
      }
    } finally { hideBusy(); }
    IMP.rows.forEach((r) => URL.revokeObjectURL(r.url));
    IMP.rows = null;
    cache.clear();
    toast(`${kept} photos imported${bad.length ? `; ${bad.length} could not be read` : ""}. Undo from this page.`);
    render();
  }
  async function exportPrompts() {
    const x = await api("roster", {});
    const out = await api("faces_export_prompts", { ids: x.rows.map((r) => r.id), label: x.team.name });
    const { f } = dialog(`<b>${out.count} AI prompts written</b><small class="sub">One text file per player in<br>${esc(out.folder)}</small>`, "Show the folder", () => true);
    f.addEventListener("submit", () => api("reveal", { path: out.folder }).catch((e) => toast(e.message)));
  }
  screens.faces = async () => {
    const b = await api("faces_import_batches");
    const st = await api("face_settings").catch(() => ({ drawn: false }));
    let review = "";
    if (IMP.rows) {
      const n = IMP.rows.filter((r) => r.on && r.pick).length;
      const rows = IMP.rows.map((r, i) => {
        const who = r.match
          ? `${playerLink(r.match.id, r.match.name)} <small class="muted">${esc(r.match.pos)} · ${esc(r.match.team || "no team")} · class of ${r.match.class_of}</small>`
          : r.candidates
            ? `<select data-imp="${i}"><option value="">Pick the player…</option>${r.candidates.map((c) => `<option value="${c.id}" ${c.id === r.pick ? "selected" : ""}>${esc(c.name)} · ${esc(c.pos)} · ${esc(c.team || "no team")} · ${c.class_of}</option>`).join("")}</select>`
            : `<span class="muted">${esc(r.why || "No match")}</span>`;
        return `<tr><td><input type="checkbox" data-imp-on="${i}" ${r.on ? "checked" : ""} ${r.pick ? "" : "disabled"}></td><td><img class="fc-thumb" src="${r.url}" alt=""></td><td class="l">${esc(r.file.name)}</td><td class="l">${who}</td><td class="l"><span class="st-pill ${r.how === "none" ? "" : r.how === "ambiguous" ? "gold" : "good"}">${HOW[r.how] || r.how}</span></td></tr>`;
      }).join("");
      review = `<div class="panel"><h2>Review <small>${IMP.rows.length} pictures · ${n} to import</small></h2>
        <p class="sub">Each picture is cropped to his face automatically (4:5). Untick any you don't want; pick the player where a name is shared.</p>
        <div class="tbl"><table><thead><tr><th></th><th></th><th class="l">File</th><th class="l">Player</th><th class="l">Matched</th></tr></thead><tbody>${rows}</tbody></table></div>
        <div style="display:flex;gap:10px"><button class="st-btn small primary" data-fact="import-go">Import ${n} photos</button><button class="st-btn small glass" data-fact="import-cancel">Cancel</button></div></div>`;
    }
    const past = b.batches.length
      ? b.batches.map((x) => `<div class="item"><span class="when">${esc(x.date || "")}</span><span>Import ${x.batch}: ${x.count} photos</span><button class="st-btn small glass" data-fact="undo" data-batch="${x.batch}">Undo</button></div>`).join("")
      : '<div class="empty">No imports yet.</div>';
    return `<div class="film"><div class="pad"><div class="h1">Faces and photos</div>
      <p class="sub">Give players your own pictures: one at a time with "Change photo" on his page, or many at once from a folder. Files are matched by name: <b>41234.jpg</b> (his id), <b>Jalen Smith - Alabama.png</b> (name and team), <b>Jalen Smith 2027.jpg</b> (name and class or draft year), or just his name when only one active player has it. Photos are copied into this save's own folder and never leave your computer.</p>
      <div class="grid2"><div class="panel"><h2>Import photos</h2><div class="sub">PNG, JPEG, WebP, and HEIC where this computer reads it.</div>
        <div style="display:flex;gap:10px;flex-wrap:wrap"><button class="st-btn small primary" data-fact="import" data-dir="1">Choose a folder…</button><button class="st-btn small glass" data-fact="import" data-dir="0">Choose pictures…</button></div></div>
      <div class="panel"><h2>Drawn heads <small>${st.drawn ? "On" : "Off"}</small></h2><div class="sub">The game draws each player without a photo itself: his face from his heritage and his own features, his haircut, beard and build this season, in his team's jersey under a studio light. A face is drawn the first time a screen shows it (a moment each) and kept in this save's folder; a new team photo draws him again. Your photos always come first.</div>
        <div><button class="st-btn small ${st.drawn ? "glass" : "primary"}" data-fact="drawn" data-on="${st.drawn ? 1 : 0}">${st.drawn ? "Turn drawn heads off" : "Turn drawn heads on"}</button></div></div>
      <div class="panel"><h2>AI prompts</h2><div class="sub">One text file per player on your squad, written from his looks (heritage, age, size, position, team colours, hair, beard and marks), ready for your own image tool.</div>
        <div><button class="st-btn small glass" data-fact="export">Export AI prompts</button></div></div></div>
      ${review}
      <div class="panel"><h2>Past imports</h2><div class="items">${past}</div></div></div></div>`;
  };
  CRUMBS.faces = "Faces and photos";
  document.addEventListener("change", (ev) => {
    const t = ev.target;
    if (!IMP.rows) return;
    if (t.dataset.imp !== undefined) {
      const r = IMP.rows[Number(t.dataset.imp)];
      r.pick = t.value ? Number(t.value) : null;
      r.on = !!r.pick;
      render($(".view") && $(".view").scrollTop);
    } else if (t.dataset.impOn !== undefined) {
      IMP.rows[Number(t.dataset.impOn)].on = t.checked;
      const n = IMP.rows.filter((r) => r.on && r.pick).length;
      const btn = document.querySelector("[data-fact=import-go]");
      if (btn) btn.textContent = `Import ${n} photos`;
    }
  });

  // ---------- watching the screens ----------
  let queued = false;
  function scan() {
    queued = false;
    const r = S.route || {};
    const view = document.querySelector(".view");
    if (!view) return;
    if (r.screen === "player") {
      const hero = view.querySelector(".hero");
      if (hero && !hero.querySelector(".fc-portrait")) portrait(hero, Number(r.args.id));
    }
    if (LISTS.has(r.screen)) decorate(view);
    if (r.screen === "roster") {
      const filters = view.querySelector(".filters");
      if (filters && !filters.querySelector("[data-fact=faces-page]")) {
        const d = document.createElement("div");
        d.className = "seg";
        d.innerHTML = `<button data-fact="faces-page">Photos and AI prompts</button>`;
        filters.appendChild(d);
      }
    }
  }
  new MutationObserver(() => {
    if (!queued) { queued = true; requestAnimationFrame(scan); }
  }).observe(document.getElementById("app"), { childList: true, subtree: true });

  window.AFMFaces = { refresh, silhouette, dot };
})();
