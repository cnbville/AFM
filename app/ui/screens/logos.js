// Logo packs you install yourself (update 3, lane 15; later extras topic 2).
// AFM ships no logos. The game matches a pack's pictures to teams
// (lane_logos.rs); this file shows them in every team mark (app.js `mark`),
// the team page's watermark, letterheads, the field's midfield and the score
// bug, and it holds the Logos screen where packs are installed and checked.
//
//   logosLoad()            this world's logos (after a career or world opens)
//   logoFor(team, year)    {id, dark} for a team object, or null
//   logoMark(lg, team, s)  the logo in a team mark's slot
//   logoHero(team)         the team page's watermark, or ""
//   compLogo(name)         a competition's logo ({id, dark}) by its name here
//   logoImg(lg, cls)       an <img> that fills in when its picture arrives
//   logoImage(team)        a decoded Image for canvases (the field)
//   logoIn(team), compIn(name)  a small logo before a name in a row
//   logoChange(team)       the "Change logo" button for a team's page
const LG = { teams: {}, comps: {}, byName: {}, year: 0, show: false, pics: new Map(), imgs: new Map(), want: new Set(), timer: 0 };

async function logosLoad() {
  try {
    const m = await api("logos_map");
    LG.teams = m.teams || {};
    LG.comps = m.comps || {};
    LG.year = m.year || 0;
    LG.show = !!m.show;
  } catch (e) {
    LG.teams = {};
    LG.comps = {};
    LG.show = false;
  }
  LG.byName = {};
  for (const [ref, list] of Object.entries(LG.teams)) for (const e of list) if (!(e[3] in LG.byName)) LG.byName[e[3]] = ref;
  return LG.show;
}

// A team's logo: the one for the name it shows (a history line keeps its
// era's name), else the one for this year.
function logoFor(team, year) {
  if (!LG.show || !team) return null;
  const ref = team.ref && team.ref.kind ? `${team.ref.kind}:${team.ref.id}` : LG.byName[team.name];
  const list = ref && LG.teams[ref];
  if (!list || !list.length) return null;
  const y = year || LG.year;
  const named = team.name ? list.filter((e) => e[3] === team.name) : [];
  const pool = named.length ? named : list;
  const e = pool.find((x) => x[1] <= y && y <= x[2]) || (named.length ? named[named.length - 1] : null);
  return e ? { id: e[0], dark: !!e[4] } : null;
}

function compLogo(name) {
  if (!LG.show || !name) return null;
  const list = LG.comps[name];
  if (!list || !list.length) return null;
  const e = list.find((x) => x[1] <= LG.year && LG.year <= x[2]) || list[list.length - 1];
  return { id: e[0], dark: !!e[3] };
}

// Pictures come as data URLs and are kept as blob URLs, so a long table
// holds short links, not the pictures themselves.
function logoSrc(id) {
  const s = LG.pics.get(id);
  if (s) return s;
  LG.want.add(id);
  if (!LG.timer) LG.timer = setTimeout(logoFetch, 0);
  return null;
}
function blobOf(dataUrl) {
  const [head, body] = dataUrl.split(",");
  const mime = (head.match(/data:([^;]+)/) || [])[1] || "image/png";
  const bin = atob(body);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  return URL.createObjectURL(new Blob([bytes], { type: mime }));
}
async function logoFetch() {
  LG.timer = 0;
  const ids = [...LG.want].filter((id) => !LG.pics.has(id));
  LG.want.clear();
  for (let i = 0; i < ids.length; i += 100) {
    const part = ids.slice(i, i + 100);
    let r;
    try { r = await api("logos_pics", { ids: part }); } catch (e) { return; }
    for (const [id, data] of Object.entries(r.pics || {})) {
      const url = blobOf(data);
      LG.pics.set(id, url);
      const img = new Image();
      img.src = url;
      LG.imgs.set(id, img);
    }
    for (const id of part) if (LG.pics.has(id)) document.querySelectorAll(`[data-lgo="${CSS.escape(id)}"]`).forEach(logoFill);
  }
}
// A decoded picture by id, for the drawn heads' jerseys.
async function logoImageById(id) {
  if (!LG.imgs.has(id)) {
    try {
      const r = await api("logos_pics", { ids: [id] });
      const d = r.pics && r.pics[id];
      if (!d) return null;
      const url = blobOf(d);
      LG.pics.set(id, url);
      const img = new Image();
      img.src = url;
      LG.imgs.set(id, img);
    } catch (e) { return null; }
  }
  const img = LG.imgs.get(id);
  if (!img.complete) await new Promise((res) => { img.onload = res; img.onerror = res; });
  return img.naturalWidth ? img : null;
}
function logoFill(el) {
  const src = LG.pics.get(el.dataset.lgo);
  if (!src) return;
  if (el.tagName === "IMG") el.src = src;
  else el.innerHTML = `<img src="${src}" alt="">`;
  el.classList.add("lgo-on");
}

function logoMark(lg, team, size) {
  const c = team && team.colors ? team.colors : ["#444", "#222"];
  const src = logoSrc(lg.id);
  return `<span class="st-mark lgo${lg.dark ? " lgo-d" : ""}${src ? " lgo-on" : ""}" data-lgo="${esc(lg.id)}" style="${size ? `--s:${size}px;` : ""}--lgc:${c[0]}">${src ? `<img src="${src}" alt="">` : esc(initials(team && team.name))}</span>`;
}
function logoImg(lg, cls = "") {
  const src = logoSrc(lg.id);
  return `<img class="lgo-img ${cls}${lg.dark ? " lgo-d" : ""}${src ? " lgo-on" : ""}" data-lgo="${esc(lg.id)}" ${src ? `src="${src}"` : ""} alt="">`;
}
// A small logo before a team's or a competition's name in a row or a
// heading, or nothing.
function logoIn(team, year) {
  const lg = logoFor(team, year);
  return lg ? logoImg(lg, "lgo-in") : "";
}
function compIn(name) {
  const lg = compLogo(name);
  return lg ? logoImg(lg, "lgo-in") : "";
}
function logoHero(team) {
  const lg = logoFor(team);
  return lg ? logoImg(lg, "lgo-hero") : "";
}
function logoImage(team) {
  const lg = logoFor(team);
  if (!lg) return null;
  logoSrc(lg.id);
  const img = LG.imgs.get(lg.id);
  return img && img.complete && img.naturalWidth ? img : null;
}

// ---------- the Logos screen (Settings) ----------
CRUMBS.logos = "Logos";
CRUMBS.logos_pack = "Logo pack";
const LGR = { review: null };

const lgMB = (b) => (b >= 1 << 20 ? `${(b / (1 << 20)).toFixed(1)} MB` : `${Math.max(1, Math.round(b / 1024))} KB`);
const lgPlural = (n, w) => `${n.toLocaleString()} ${w}${n === 1 ? "" : "s"}`;

screens.logos = async () => {
  title("Logos");
  if (LGR.review) return lgReviewHtml();
  const st = await api("logos_status");
  const ids = st.packs.flatMap((p) => p.sample);
  const seg = (k, cur, on, off) => `<div class="seg">${[[true, on], [false, off]].map(([v, t]) => `<button data-lgact="set" data-k="${k}" data-v="${v}" aria-pressed="${cur === v}">${t}</button>`).join("")}</div>`;
  const packs = st.packs.length
    ? st.packs.map((p) => `<div class="lgo-pack">
        <div class="lgo-stack">${p.sample.map((id) => logoImg({ id })).join("")}</div>
        <div class="lgo-pack-copy"><b>${esc(p.name)}</b><small>Installed ${esc(p.installed)} · ${lgPlural(p.pictures, "picture")} · ${lgMB(p.bytes)}</small>
          <div class="lgo-cells"><div><span class="st-label">Teams</span><b>${p.teams.toLocaleString()}</b></div><div><span class="st-label">Competitions</span><b>${p.comps}</b></div></div></div>
        <div class="lgo-pack-acts"><button class="st-btn small" data-lgact="check" data-pack="${esc(p.id)}">Check matches</button><button class="st-btn small ghost" data-lgact="remove" data-pack="${esc(p.id)}" data-name="${esc(p.name)}">Remove</button></div>
      </div>`).join("")
    : `<div class="empty">No logo pack yet. Every team shows its drawn mark until you install one.</div>`;
  ids.forEach((id) => logoSrc(id));
  return `<div class="pad lgo">${pageHead({ kicker: "Settings", title: "Logos", sub: "AFM has no logos of its own. A pack you install is copied into the game's folder, used by every save on this computer and never sent anywhere." })}
    <div class="lgo-bar"><button class="st-btn primary" data-lgact="install">Install a logo pack…</button>
      <div class="setrow lgo-set"><div><b>Show logos</b><small>Off shows every team's drawn mark</small></div>${seg("show", st.show, "On", "Off")}</div>
      ${st.world_fictional ? `<div class="setrow lgo-set"><div><b>In this fictional-names world</b><small>Real logos beside made-up names</small></div>${seg("fictional", st.fictional, "On", "Off")}</div>` : ""}</div>
    <div class="lgo-packs">${packs}</div>
    <div class="grid2 lgo-help">
      <div class="panel"><h2>What a pack is</h2><p class="sub">A folder of pictures (SVG, PNG, JPG or WebP), one or more per team, the way Football Manager's logo packs work. Pick the folder; the game reads every picture in it and its folders, shows what it found and keeps nothing until you press "Use these logos". A later pack's logo wins for a team both have.</p></div>
      <div class="panel"><h2>How pictures find their teams</h2><p class="sub">First a mapping file anywhere in the pack, <code>logos.csv</code>, one line per picture with optional years; then a teams list in the pack (a JSON list with a name and a logo path); then each picture's own name ("Kansas_City_Chiefs_logo.svg", "iowa-st.svg"). A logo covers the years its team had that name.</p>
      <pre class="lgo-pre">file,team,from,to
nfl/titans.svg,Tennessee Titans,1999,
nfl/oilers.svg,Houston Oilers,,1996</pre></div>
    </div></div>`;
};

// The review before anything is kept.
const LGR_TABS = [["matched", "Matched"], ["unsure", "Unsure"], ["none", "No match"], ["spare", "Spare"], ["skip", "Skipped"]];
function lgReviewHtml() {
  const R = LGR.review;
  const n = (s) => R.rows.filter((r) => r.status === s).length;
  const teams = new Set(R.rows.filter((r) => r.status === "matched").map((r) => r.key)).size;
  const rows = R.rows.map((r, i) => [r, i]).filter(([r]) => r.status === R.tab);
  const shown = rows.slice(0, R.limit);
  const team = (r, i) => {
    if (r.status === "unsure") return `<div class="lgo-opts">${r.options.map((o) => `<button class="lgo-chip" data-lgact="pick" data-i="${i}" data-key="${esc(o.key)}" data-label="${esc(o.label)}" data-level="${esc(o.level)}">${esc(o.label)} <small>${esc(o.level)}</small></button>`).join("")}</div>`;
    if (!r.key) return `<span class="muted">${esc(r.how || "No team found")}</span>`;
    return `<b>${esc(r.team)}</b> <small class="muted">${esc(r.level || "")}${r.from || r.to ? ` · ${r.from || "…"}-${r.to || "…"}` : ""}</small>`;
  };
  const acts = (r, i) => {
    const find = `<button class="st-btn small ghost" data-lgact="find" data-i="${i}">${r.key ? "Change" : "Find a team"}</button>`;
    if (r.status === "skip") return `<button class="st-btn small ghost" data-lgact="unskip" data-i="${i}">Bring back</button>`;
    if (r.status === "spare") return `${r.key ? `<button class="st-btn small ghost" data-lgact="use" data-i="${i}">Use this one</button>` : ""}${find}`;
    return `${find}${r.status !== "none" ? `<button class="st-btn small ghost" data-lgact="skip" data-i="${i}">Skip</button>` : ""}`;
  };
  const body = shown.length
    ? `<div class="tbl lgo-tbl"><table><thead><tr><th></th><th class="l">Picture</th><th class="l">Team</th><th class="l">Matched by</th><th></th></tr></thead><tbody>${shown
        .map(([r, i]) => `<tr><td class="lgo-th"><img src="${R.urls[r.file] || ""}" alt="" loading="lazy" class="${R.dark[r.file] ? "lgo-d" : ""}"></td><td class="l lgo-file">${esc(r.file)}</td><td class="l">${team(r, i)}</td><td class="l lgo-how">${esc(r.how || "")}${r.variant ? " · another version" : ""}</td><td class="lgo-acts">${acts(r, i)}</td></tr>`)
        .join("")}</tbody></table></div>${rows.length > shown.length ? `<button class="st-btn small" data-lgact="more">Show ${Math.min(200, rows.length - shown.length)} more of ${rows.length - shown.length}</button>` : ""}`
    : `<div class="empty">Nothing here.</div>`;
  return `<div class="pad lgo">${pageHead({ kicker: "Settings · Logos · install", title: esc(R.name), sub: `${lgPlural(R.rows.length, "picture")} found. Nothing is kept until you press "Use these logos".${R.notes.length ? " " + esc(R.notes.join(". ")) + "." : ""}` })}
    <div class="lgo-sum"><div><span class="st-label">Teams</span><b>${teams.toLocaleString()}</b></div><div><span class="st-label">Unsure</span><b>${n("unsure")}</b></div><div><span class="st-label">No match</span><b>${n("none")}</b></div><div><span class="st-label">Spare</span><b>${n("spare")}</b><small>other pictures of a team that has one</small></div></div>
    <div class="tabs lgo-tabs">${LGR_TABS.map(([k, t]) => `<button data-lgact="tab" data-tab="${k}" aria-selected="${R.tab === k}">${t}<small>${n(k)}</small></button>`).join("")}</div>
    ${body}
    <div class="lgo-foot"><button class="st-btn primary" data-lgact="apply">Use these logos</button><button class="st-btn ghost" data-lgact="cancel">Cancel</button><span class="muted">Remove takes a whole pack out again.</span></div></div>`;
}

// Reading a picked folder: pictures by path, plus the text files that can
// name teams (a mapping file, a teams list).
function lgPickFolder() {
  return new Promise((done) => {
    const inp = document.createElement("input");
    inp.type = "file";
    inp.webkitdirectory = true;
    inp.multiple = true;
    inp.style.display = "none";
    inp.addEventListener("change", () => { done([...inp.files]); inp.remove(); });
    inp.addEventListener("cancel", () => { done(null); inp.remove(); });
    document.body.appendChild(inp);
    inp.click();
  });
}
const LG_PIC = /\.(svg|png|jpe?g|webp)$/i;
async function lgInstall() {
  const files = await lgPickFolder();
  if (!files || !files.length) return;
  const pics = files.filter((f) => LG_PIC.test(f.name) && f.size < 20 << 20);
  if (!pics.length) { toast("No pictures in that folder (SVG, PNG, JPG or WebP)."); return; }
  const path = (f) => f.webkitRelativePath || f.name;
  const texts = [];
  for (const f of files.filter((f) => /\.(csv|json)$/i.test(f.name) && f.size < 4 << 20)) texts.push({ path: path(f), text: await f.text() });
  showBusy(`Reading ${lgPlural(pics.length, "picture")}…`);
  let m;
  try { m = await api("logos_match", { files: pics.map((f) => ({ path: path(f), size: f.size })), texts }); } catch (e) { hideBusy(); toast(e.message); return; }
  hideBusy();
  const byPath = {};
  const urls = {};
  for (const f of pics) { byPath[path(f)] = f; urls[path(f)] = URL.createObjectURL(f); }
  const folder = (path(pics[0]).split("/")[0] || "Logo pack").replace(/[-_]+/g, " ");
  LGR.review = { name: folder, rows: m.rows, notes: m.notes || [], files: byPath, urls, dark: {}, tab: "matched", limit: 200 };
  go("logos", {}, { push: false });
}

// Measure a picture once: too dark for the dark screens (mean brightness
// of its drawn pixels under 0.3), and a copy at most 512 pixels across.
function lgLoad(file) {
  return new Promise((done) => {
    const img = new Image();
    img.onload = () => done(img);
    img.onerror = () => done(null);
    img.src = URL.createObjectURL(file);
  });
}
function lgRead(file) {
  return new Promise((done) => {
    const r = new FileReader();
    r.onload = () => done(String(r.result));
    r.onerror = () => done(null);
    r.readAsDataURL(file);
  });
}
async function lgPrepare(file) {
  const img = await lgLoad(file);
  if (!img) return null;
  const svg = /\.svg$/i.test(file.name);
  let dark = false;
  try {
    const c = document.createElement("canvas");
    c.width = c.height = 24;
    const g = c.getContext("2d", { willReadFrequently: true });
    const w = img.naturalWidth || 24, h = img.naturalHeight || 24, k = 24 / Math.max(w, h);
    g.drawImage(img, (24 - w * k) / 2, (24 - h * k) / 2, w * k, h * k);
    const d = g.getImageData(0, 0, 24, 24).data;
    let s = 0, n = 0;
    for (let i = 0; i < d.length; i += 4) if (d[i + 3] > 128) { s += 0.2126 * d[i] + 0.7152 * d[i + 1] + 0.0722 * d[i + 2]; n++; }
    dark = n > 0 && s / n / 255 < 0.3;
  } catch (e) { /* a picture that can't be measured stays bare */ }
  let data;
  if (svg) {
    let text = await file.text();
    // A drawing with no size of its own gets its viewBox's, so canvases (the
    // field) can draw it.
    try {
      const doc = new DOMParser().parseFromString(text, "image/svg+xml");
      const root = doc.documentElement;
      const vb = (root.getAttribute("viewBox") || "").split(/[\s,]+/).map(Number);
      if (root.nodeName === "svg" && (!root.getAttribute("width") || !root.getAttribute("height")) && vb.length === 4 && vb[2] > 0 && vb[3] > 0) {
        root.setAttribute("width", String(vb[2]));
        root.setAttribute("height", String(vb[3]));
        text = new XMLSerializer().serializeToString(doc);
      }
    } catch (e) { /* kept as it came */ }
    data = "data:image/svg+xml;base64," + btoa(unescape(encodeURIComponent(text)));
  } else if (Math.max(img.naturalWidth, img.naturalHeight) > 512 || file.size > 1 << 20) {
    const k = 512 / Math.max(img.naturalWidth, img.naturalHeight, 1);
    const c = document.createElement("canvas");
    c.width = Math.max(1, Math.round(img.naturalWidth * Math.min(1, k)));
    c.height = Math.max(1, Math.round(img.naturalHeight * Math.min(1, k)));
    c.getContext("2d").drawImage(img, 0, 0, c.width, c.height);
    data = c.toDataURL("image/png");
  } else {
    data = await lgRead(file);
    if (data && /^data:application\/octet-stream/.test(data)) data = data.replace(/^data:[^;]+/, /\.png$/i.test(file.name) ? "data:image/png" : /\.webp$/i.test(file.name) ? "data:image/webp" : "data:image/jpeg");
  }
  URL.revokeObjectURL(img.src);
  return data ? { data, dark } : null;
}
async function lgApply() {
  const R = LGR.review;
  const use = R.rows.filter((r) => r.status === "matched" && r.key);
  if (!use.length) { toast("No picture has a team yet."); return; }
  const { pack } = await api("logos_begin", { name: R.name });
  const errors = [];
  try {
    for (let i = 0; i < use.length; i += 12) {
      showBusy(`Installing logos: ${Math.min(i + 12, use.length)} of ${use.length}`);
      const batch = [];
      for (const r of use.slice(i, i + 12)) {
        const p = await lgPrepare(R.files[r.file]);
        if (!p) { errors.push(r.file); continue; }
        batch.push({ file: r.file, key: r.key, from: r.from, to: r.to, data: p.data, dark: p.dark });
      }
      const res = await api("logos_add", { pack, files: batch });
      errors.push(...(res.errors || []));
    }
    await api("logos_commit", { pack });
  } catch (e) {
    hideBusy();
    try { await api("logos_remove", { pack }); } catch (x) { /* already gone */ }
    toast(e.message);
    return;
  }
  hideBusy();
  Object.values(R.urls).forEach((u) => URL.revokeObjectURL(u));
  LGR.review = null;
  await logosReload();
  toast(`Logos installed for ${lgPlural(new Set(use.map((r) => r.key)).size, "team")}.${errors.length ? ` ${errors.length} couldn't be read.` : ""}`);
  go("logos", {}, { push: false });
}

// After a change: the new logos everywhere, the next screen drawn with them.
async function logosReload() {
  LG.pics.forEach((u) => URL.revokeObjectURL(u));
  LG.pics.clear();
  LG.imgs.clear();
  if (S.status) await logosLoad();
}

// "Change logo" on a team's page: one picture for that team, kept with
// this save beside your photos; it wins over any pack's.
function logoChange(t) {
  return t && t.ref ? `<button class="st-btn glass" data-lgact="own" data-kind="${esc(t.ref.kind)}" data-id="${t.ref.id}" data-name="${esc(t.name)}">Change logo</button>` : "";
}
async function lgOwn(kind, id, name) {
  const cur = (LG.teams[`${kind}:${id}`] || [])[0];
  const mine = !!cur && String(cur[0]).startsWith("own:");
  let got = null, drop = false;
  const { f, answer } = dialog(`<b>Change logo</b><small class="sub">${esc(name)}</small>
    <div class="lgo-own"><span class="lgo-own-pic">${cur ? logoImg({ id: cur[0], dark: !!cur[4] }) : `<span class="lgo-own-none">${esc(initials(name))}</span>`}</span>
      <div class="lgo-own-copy"><button type="button" class="st-btn small glass" data-lgown="pick">Choose a picture…</button><small class="lgo-own-file">${mine ? "Your logo" : cur ? "From a logo pack" : "No logo yet"}</small></div></div>
    <small class="sub">Kept with this save, beside your photos. A pack's logo for this team gives way to yours.</small>
    ${mine ? '<button type="button" class="st-btn small glass lgo-own-drop" data-lgown="drop">Remove your logo</button>' : ""}`, "Use this logo", () => (drop ? "drop" : got ? "use" : null));
  f.querySelector("[data-lgown=pick]").addEventListener("click", () => {
    const inp = document.createElement("input");
    inp.type = "file";
    inp.accept = ".png,.jpg,.jpeg,.webp,.svg";
    inp.addEventListener("change", async () => {
      const file = inp.files && inp.files[0];
      if (!file) return;
      const p = await lgPrepare(file);
      if (!p) { toast("That picture can't be read."); return; }
      got = p;
      f.querySelector(".lgo-own-pic").innerHTML = `<img class="lgo-img lgo-on${p.dark ? " lgo-d" : ""}" src="${p.data}" alt="">`;
      f.querySelector(".lgo-own-file").textContent = file.name;
    });
    inp.click();
  });
  const d = f.querySelector("[data-lgown=drop]");
  if (d) d.addEventListener("click", () => { drop = true; f.requestSubmit(); });
  const what = await answer;
  if (!what) return;
  if (what === "use") await api("logos_own_set", { kind, id: Number(id), data: got.data, dark: got.dark });
  else await api("logos_own_clear", { kind, id: Number(id) });
  await logosReload();
  toast(what === "use" ? (LG.show ? `${name} has your logo.` : `Kept. Logos are switched off in Settings, Logos.`) : `Your logo for ${name} is gone.`);
  render();
}

// A team for a picture: search the teams the game knows.
async function lgFindTeam(label) {
  let found = [], pick = null, n = 0;
  const { f, answer } = dialog(`<b>${esc(label)}</b><small class="sub">Type a team, a school, a conference or a bowl.</small><input class="lgo-q" placeholder="Iowa State, Chiefs, Big 12…" autocomplete="off"><div class="lgo-found"></div>`, "Use this team", () => pick || found[0] || null);
  const q = f.querySelector(".lgo-q"), out = f.querySelector(".lgo-found");
  q.addEventListener("input", async () => {
    const my = ++n;
    const r = await api("logos_teams", { q: q.value });
    if (my !== n) return;
    found = r.teams;
    pick = null;
    out.innerHTML = found.map((t, i) => `<button type="button" class="lgo-found-row" data-i="${i}"><b>${esc(t.label)}</b><small>${esc(t.level)}</small></button>`).join("") || '<div class="muted">No team by that name.</div>';
    out.querySelectorAll("button").forEach((b) => b.addEventListener("click", () => { pick = found[Number(b.dataset.i)]; f.requestSubmit(); }));
  });
  setTimeout(() => q.focus(), 30);
  return answer;
}

function lgSetKey(i, t) {
  const R = LGR.review;
  const r = R.rows[i];
  // The team's other picture in use steps aside.
  for (const o of R.rows) if (o !== r && o.status === "matched" && o.key === t.key && (o.from || 0) === (r.from || 0) && (o.to || 0) === (r.to || 0)) o.status = "spare";
  Object.assign(r, { key: t.key, team: t.label, level: t.level, status: "matched", how: r.how === "no team has this name" ? "your pick" : r.how });
}

document.addEventListener("click", async (ev) => {
  const el = ev.target.closest("[data-lgact]");
  if (!el) return;
  ev.preventDefault();
  const d = el.dataset, R = LGR.review, i = Number(d.i);
  try {
    switch (d.lgact) {
      case "install": await lgInstall(); return;
      case "own": await lgOwn(d.kind, d.id, d.name); return;
      case "set": await api("logos_set", { [d.k]: d.v === "true" }); await logosReload(); render(); return;
      case "check": go("logos_pack", { pack: d.pack }); return;
      case "remove":
        if (!(await sure(`Remove ${d.name}?`, "Its pictures are deleted from the game's folder. Teams show their drawn marks again unless another pack has them.", "Remove pack"))) return;
        await api("logos_remove", { pack: d.pack });
        await logosReload();
        render();
        return;
      case "tab": R.tab = d.tab; R.limit = 200; render(); return;
      case "more": R.limit += 200; render(); return;
      case "pick": lgSetKey(i, { key: d.key, label: d.label, level: d.level }); render(); return;
      case "find": { const t = await lgFindTeam(R.rows[i].file); if (t) { lgSetKey(i, t); render(); } return; }
      case "skip": R.rows[i].was = R.rows[i].status; R.rows[i].status = "skip"; render(); return;
      case "unskip": R.rows[i].status = R.rows[i].was || (R.rows[i].key ? "matched" : "none"); render(); return;
      case "use": lgSetKey(i, { key: R.rows[i].key, label: R.rows[i].team, level: R.rows[i].level }); render(); return;
      case "cancel": Object.values(R.urls).forEach((u) => URL.revokeObjectURL(u)); LGR.review = null; render(); return;
      case "apply": await lgApply(); return;
      case "pchange": {
        const t = await lgFindTeam(d.file);
        if (t) { await api("logos_entry_set", { pack: d.pack, n: Number(d.n), key: t.key }); await logosReload(); render(); }
        return;
      }
      case "pdrop": await api("logos_entry_set", { pack: d.pack, n: Number(d.n), key: null }); await logosReload(); render(); return;
    }
  } catch (e) {
    hideBusy();
    toast(e.message);
  }
});

// One installed pack: every picture with its team ("Check matches").
screens.logos_pack = async (a) => {
  const x = await api("logos_pack", { pack: a.pack });
  title(x.name, "Logo pack");
  const f = keep("logos_pack", { q: "" });
  const q = f.q.toLowerCase();
  const rows = x.rows.filter((r) => !q || `${r.file} ${r.team || ""}`.toLowerCase().includes(q));
  return `<div class="pad lgo">${pageHead({ kicker: "Settings · Logos", title: esc(x.name), sub: `Installed ${esc(x.installed)} · ${lgPlural(x.rows.length, "picture")}. Change a team here; the game uses it from the next screen.` })}
    <div class="filters"><div class="field"><label for="lgo-filter">Find</label><input id="lgo-filter" data-lgfilter value="${esc(f.q)}" placeholder="A team or a file"></div></div>
    <div class="tbl lgo-tbl"><table><thead><tr><th></th><th class="l">Picture</th><th class="l">Team</th><th class="l">Years</th><th></th></tr></thead><tbody>${rows
      .slice(0, 400)
      .map((r) => `<tr class="${r.key ? "" : "lgo-dropped"}"><td class="lgo-th">${logoImg({ id: r.id, dark: r.dark })}</td><td class="l lgo-file">${esc(r.file)}</td><td class="l">${r.key ? `<b>${esc(r.team)}</b> <small class="muted">${esc(r.level || "")}</small>` : '<span class="muted">Not used</span>'}</td><td class="l">${r.from || r.to ? `${r.from || "…"}-${r.to || "…"}` : '<span class="muted">Every year</span>'}</td>
        <td class="lgo-acts"><button class="st-btn small ghost" data-lgact="pchange" data-pack="${esc(x.pack)}" data-n="${r.n}" data-file="${esc(r.file)}">Change</button>${r.key ? `<button class="st-btn small ghost" data-lgact="pdrop" data-pack="${esc(x.pack)}" data-n="${r.n}">Don't use</button>` : ""}</td></tr>`)
      .join("")}</tbody></table></div>${rows.length > 400 ? `<div class="sub">${rows.length - 400} more: narrow the list with Find.</div>` : ""}</div>`;
};
document.addEventListener("change", (ev) => {
  const el = ev.target.closest("[data-lgfilter]");
  if (!el) return;
  keep("logos_pack", { q: "" }).q = el.value;
  render();
});
