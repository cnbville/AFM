// Other versions (Cain 2026-11-15): Settings can install an older published
// build of AFM when a new one is too buggy to play, and take you back to the
// latest. The Mac app only (it replaces itself).
//
// The list is the builds on the AFM repository's releases (app-<build>, kept
// for good), with the date and the What's new text this app already carries
// (changelog.js). A build can be installed from here when its release has a
// downgrade.json, the signed update made for it (.github/scripts/mac-publish.sh
// writes one for every build that has a signed app file, so builds before 486
// are not in the list). The Mac side is `install_version` (app/src-tauri).
//
// Going back is careful with your saves: it asks first, says how many saves
// the older build may not be able to open, keeps a copy of every save beside
// them (`versions_backup`: no space and no wait, saves-backup in the game
// folder) and only then installs. Then it pauses updates, so the build you
// chose stays until you choose "Go back to the latest" (`update_pin_*`, read
// by checkUpdate in app.js). A build made before the pause existed can't
// read it: its page says so, and the downloads page can be held at it
// (.github/app-hold).
"use strict";

const VER_REPO = "cnbville/AFM";
const VER_API = `https://api.github.com/repos/${VER_REPO}`;

// "0.2.540" -> 540.
const verBuild = (v) => { const m = /^0\.2\.(\d+)$/.exec(String(v || "")); return m ? Number(m[1]) : null; };
// "2026-11-15T13:57:12Z" -> "2026-11-15 13:57 UTC", the form a save's header uses.
const verUtc = (iso) => (/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}/.test(String(iso)) ? `${iso.slice(0, 10)} ${iso.slice(11, 16)} UTC` : "");
const verCan = () => !!(window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.updater);

async function verGet(path) {
  const r = await fetch(`${VER_API}/${path}`, { headers: { Accept: "application/vnd.github+json" } });
  if (!r.ok) throw new Error(r.status === 403 || r.status === 429 ? "GitHub is limiting this connection for now. Try again in a few minutes." : `GitHub answered ${r.status}.`);
  return r.json();
}

// The builds you can install, newest first.
async function verBuilds() {
  const out = [];
  for (let page = 1; page <= 3; page++) {
    const list = await verGet(`releases?per_page=100&page=${page}`);
    for (const x of list) {
      const m = /^app-(\d+)$/.exec(x.tag_name || "");
      if (!m || x.draft || !(x.assets || []).some((a) => a.name === "downgrade.json")) continue;
      out.push({ build: Number(m[1]), version: `0.2.${m[1]}`, date: x.published_at, sha: x.target_commitish });
    }
    if (list.length < 100) break;
  }
  return out.sort((a, b) => b.build - a.build);
}

// The pause on updates, if it still stands: it belongs to the build you went
// back to, so a different build running now (you installed one by hand) ends it.
async function verPin() {
  let pin = null;
  try { pin = await api("update_pin_get"); } catch (e) { return null; }
  if (!pin) return null;
  const now = (await appHello()).version;
  if (pin.version === now) return pin;
  api("update_pin_clear").catch(() => {});
  return null;
}

// What shows in Settings (the opening screen's box and the in-game page).
async function verNote() {
  if (!verCan()) return "";
  const pin = await verPin();
  return pin ? `Updates are paused: you went back to ${esc(pin.version)}${pin.from ? ` from ${esc(pin.from)}` : ""}. They start again when you go back to the latest.` : "";
}
async function verButtons() {
  if (!verCan()) return "";
  const pin = await verPin();
  return `<button type="button" class="pg-btn s st-btn small" data-ver="open">Other versions…</button>${pin ? '<button type="button" class="pg-btn s st-btn small" data-ver="latest">Go back to the latest</button>' : ""}`;
}
// The in-game Settings page's panel.
async function verPanel(row) {
  if (!verCan()) return "";
  const [pin, v] = [await verPin(), (await appHello()).version];
  const sub = pin ? `You are on ${esc(v)}, which you went back to from ${esc(pin.from || "a newer build")}. Updates are paused until you go back to the latest.` : `You are on ${esc(v)}. If a new build is too buggy to play, you can go back to an older one.`;
  return `<div class="panel"><h2>Version</h2>${row("AFM " + esc(v), sub, `<span class="ver-btns"><button class="st-btn small glass" data-ver="open">Other versions…</button>${pin ? '<button class="st-btn small primary" data-ver="latest">Go back to the latest</button>' : ""}</span>`)}</div>`;
}

// The list of builds.
async function verOpen() {
  const now = (await appHello()).version;
  const here = verBuild(now);
  toast("Looking up the published versions…");
  let list;
  try { list = await verBuilds(); } catch (e) { toast(`The list of versions can't be read right now. ${e.message || e}`); return; }
  if (!list.length) { toast("No other version can be installed from here yet."); return; }
  const rows = list.map((b) => {
    const note = (typeof CHANGELOG !== "undefined" ? CHANGELOG : []).find((e) => e.v === b.version);
    const what = note
      ? `<details><summary>${esc(note.title)}</summary><ul>${(note.notes || []).map((n) => `<li>${esc(n)}</li>`).join("")}</ul></details>`
      : '<span class="ver-none">Nothing a player could see changed.</span>';
    const tag = b.build === here ? '<span class="pg-tag">INSTALLED</span>' : b.build > here ? '<span class="pg-tag">NEWER</span>' : "";
    const btn = b.build === here ? "" : `<button type="button" class="st-btn small glass" data-ver-pick="${b.build}">Install</button>`;
    return `<div class="ver-row"><div><span class="ver-head"><b>${esc(b.version)}</b><span class="ver-date">${esc(builtText(b.date))}</span>${tag}</span>${what}</div>${btn}</div>`;
  }).join("");
  const { f, answer } = dialog(`<b>Other versions</b><small class="sub">You are on ${esc(now)}. Pick a build to install. AFM keeps a copy of your saves first and restarts when it is done.</small><div class="ver-list">${rows}</div>`, "Close", () => f.dataset.pick || null);
  f.classList.add("ver-box");
  f.querySelector("[data-dlg=no]").remove();
  f.addEventListener("click", (ev) => {
    const b = ev.target.closest("[data-ver-pick]");
    if (!b) return;
    f.dataset.pick = b.dataset.verPick;
    f.requestSubmit();
  });
  const pick = await answer;
  if (pick) await verInstall(list.find((b) => String(b.build) === String(pick)), now);
}

// Asks, keeps your saves, installs, restarts.
async function verInstall(b, now) {
  if (!b) return;
  const T = window.__TAURI__;
  const j = await api("job").catch(() => null);
  if (j && !j.finished) { toast(`${j.kind === "build" ? "A world is being built" : "A world is downloading"}: let it finish (or stop it) before changing versions.`); return; }
  // What the build can't open, and whether it can hold off updates by itself.
  const before = await verGet(`commits/${b.sha}`).then((c) => verUtc(c.commit.committer.date), () => "");
  const [chk, pauses] = await Promise.all([
    api("versions_check", { before }).catch(() => null),
    fetch(`${VER_API}/contents/app/ui/versions.js?ref=${b.sha}`, { headers: { Accept: "application/vnd.github+json" } }).then((r) => r.ok, () => null),
  ]);
  const newer = chk ? chk.newer : [];
  const named = (kind) => newer.filter((x) => x.kind === kind);
  const lines = [`<p>AFM downloads ${esc(b.version)} (${esc(builtText(b.date))}), installs it and restarts.</p>`];
  if (S.status && S.status.you) lines.push("<p><b>The career you have open closes.</b> Anything since your last save is lost, so save first if you want it.</p>");
  lines.push("<p>Your saves stay where they are. AFM first keeps a copy of every one in the <b>saves-backup</b> folder of the game folder (it takes no extra room and no time).</p>");
  if (newer.length) {
    const show = newer.slice(0, 4).map((x) => esc(x.name)).join(", ") + (newer.length > 4 ? ` and ${newer.length - 4} more` : "");
    const s = named("save").length, w = named("world").length;
    lines.push(`<p><b>${[s ? `${s} of your saves` : "", w ? `${w} of your worlds` : ""].filter(Boolean).join(" and ")}</b> (${show}) were written by a newer version than ${esc(b.version)}. It may say "made by a newer version" and not open them, or open one without what the newer version added. Don't save over those in ${esc(b.version)}: save a career under a new name.</p>`);
  } else if (chk && chk.known) {
    lines.push(`<p>None of your saves or worlds is newer than ${esc(b.version)}.</p>`);
  } else {
    lines.push(`<p>AFM couldn't tell which of your saves are newer than ${esc(b.version)}; the copy covers all of them.</p>`);
  }
  if (pauses === true) lines.push("<p>Updates pause while you stay on it. Settings, Other versions, Go back to the latest ends the pause.</p>");
  else if (pauses === false) lines.push(`<p><b>${esc(b.version)} can't pause updates itself</b>, so it looks for one when it starts and may put you straight back on the latest. It only stays while the project's downloads page is held at this build (the file .github/app-hold).</p>`);
  const { f, answer } = dialog(`<b>Go back to ${esc(b.version)}?</b><div class="sub ver-says">${lines.join("")}</div>`, `Install ${b.version}`, () => true);
  f.classList.add("ver-box");
  if ((await answer) !== true) return;

  // 1. The copy, 2. the pause, 3. the install.
  try {
    const k = await api("versions_backup", { label: `before ${b.version}` });
    if (k.left && k.left.length) toast(`These files could not be copied: ${k.left.join(", ")}.`);
  } catch (e) { toast(`Your saves could not be copied first, so nothing was changed. ${e.message || e}`); return; }
  updating = true;
  updateBar(`<b>Installing ${esc(b.version)}…</b><small id="upmb"></small>`);
  const stop = await T.event.listen("version-progress", (ev) => {
    const { got, total } = ev.payload || {};
    const el = $("#upmb");
    if (el && total) el.textContent = `${Math.round(got / 1e6)} of ${Math.round(total / 1e6)} MB`;
  }).catch(() => () => {});
  try {
    await api("update_pin_set", { version: b.version, from: now });
    await T.core.invoke("install_version", { build: b.build });
  } catch (e) {
    api("update_pin_clear").catch(() => {});
    updating = false;
    updateBar(`<b>${esc(b.version)} didn't install.</b><small>${esc(plain(e && e.message ? e.message : String(e)))}. Nothing was changed.</small><button class="st-btn small glass" data-act="updclose">Close</button>`);
    return;
  } finally {
    stop();
  }
  updateBar(`<b>${esc(b.version)} is installed.</b><small>AFM restarts now.</small>`);
  setTimeout(() => T.process.relaunch(), 600);
}

// Ends the pause and takes the newest build the normal way.
async function verLatest() {
  await api("update_pin_clear").catch(() => {});
  toast("Looking for the latest version…");
  const r = await checkUpdate(true);
  if (r === false) toast("The update page can't be reached right now. Updates are no longer paused; AFM tries again by itself.");
  else if (!updating) toast("AFM is on the latest version.");
}

document.addEventListener("click", (ev) => {
  const a = ev.target.closest("[data-ver]");
  if (!a) return;
  ev.preventDefault();
  if (a.dataset.ver === "open") verOpen().catch((e) => toast(String(e && e.message ? e.message : e)));
  else if (a.dataset.ver === "latest") verLatest().catch((e) => toast(String(e && e.message ? e.message : e)));
});
