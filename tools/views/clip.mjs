// The film room in a real browser (lane 13): loads a saved career, opens a
// kept game's Watch tab, checks the 2D view, the panels and the keys, then
// turns on the 3D view and records a clip of a few snaps for Cain's okay.
// Reports every page error, console error and failed call.
//
// Usage: node tools/views/clip.mjs <save file> [out dir] [game index]
// Needs afm_play running (cargo run --release --bin afm_play) and
// PLAYWRIGHT_BROWSERS_PATH=/opt/pw-browsers; AFM_URL (default
// http://127.0.0.1:7700). AFM_CLIP=0 skips the 3D view, AFM_REC=0 only its
// recording.
import { createRequire } from "module";
import fs from "fs";
const require = createRequire(import.meta.url);
let pw;
try { pw = require("playwright"); } catch { pw = require("/opt/node-tools/node_modules/playwright"); }

const [save, out = "/tmp/afm-views", gameArg] = process.argv.slice(2);
const url = process.env.AFM_URL || "http://127.0.0.1:7700";
fs.mkdirSync(out, { recursive: true });
const problems = [];
const browser = await pw.chromium.launch({ args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--ignore-gpu-blocklist"] });
const W = 1600, H = 960;
const ctx = await browser.newContext({ viewport: { width: W, height: H }, deviceScaleFactor: 1 });
const page = await ctx.newPage();
let where = "start";
page.on("pageerror", (e) => problems.push(`[${where}] page error: ${e.message}`));
page.on("console", (m) => { if (m.type() === "error" || m.type() === "warning") problems.push(`[${where}] console ${m.type()}: ${m.text()}`); });
page.on("response", async (r) => {
  if (r.url().includes("/api/")) {
    try { const j = await r.json(); if (j.err !== undefined) problems.push(`[${where}] ${r.url().split("/api/")[1]}: ${j.err}`); } catch {}
  } else if (r.status() >= 400) problems.push(`[${where}] ${r.status()} ${r.url()}`);
});
const api = (cmd, args = {}) => page.evaluate(async ([c, a]) => (await (await fetch("/api/" + c, { method: "POST", body: JSON.stringify(a) })).json()), [cmd, args]);
const shot = (name) => page.screenshot({ path: `${out}/${name}.png` });
const wait = (ms) => page.waitForTimeout(ms);

await page.goto(url);
await wait(600);
let r = await api("load", { path: save });
if (r.err) { console.log("load:", r.err); process.exit(2); }
await page.evaluate(() => enterCareer());
await wait(1200);
// Big moments and open letters cover the page: close them first.
const clear = async () => {
  for (let k = 0; k < 6; k++) {
    const b = await page.$("[data-act=revealnext], [data-act=revealclose], [data-act=letterclose], .reveal-room button");
    if (!b) break;
    await b.click({ timeout: 2000 }).catch(() => {});
    await wait(300);
  }
  await page.keyboard.press("Escape").catch(() => {});
};
await clear();
const games = (await api("games")).ok.games;
if (!games.length) { console.log("no kept games"); process.exit(2); }
const k = gameArg != null ? Number(gameArg) : games[games.length - 1].index;
console.log(`${games.length} kept games; watching game ${k}`);

// The Watch tab.
where = "watch";
// The 2D view first (the film room's first opening is 3D).
await page.evaluate((k) => { AFMViews.SET.view = "2d"; AFMViews.saveSettings(); S.keep.gptab = { t: "Watch" }; go("game", { index: k }); }, k);
await page.waitForSelector(".vw-stage canvas", { timeout: 60000 });
await clear();
await wait(2500);
await shot("1-watch-2d-dots");
// Every panel of the side.
for (const p of ["cuts", "grades", "draw", "view", "plays"]) {
  where = "panel " + p;
  await page.click(`.vw-side [data-k="${p}"]`);
  await wait(p === "grades" ? 3500 : 500);
  await shot(`2-panel-${p}`);
}
// Keys: pause, step, next snap, cameras.
where = "keys";
await page.click(".vw-stage canvas", { position: { x: 30, y: 300 } });
await page.keyboard.press("Space");
await wait(300);
await page.keyboard.press("ArrowRight");
await page.keyboard.press("Shift+ArrowRight");
await wait(1800);
for (const c of ["2", "3", "1"]) {
  await page.keyboard.press(c);
  await wait(1200);
  await shot(`3-cam-${c}`);
}
// Figures instead of dots, and a man picked and isolated.
where = "figures";
await page.evaluate(() => { AFMViews.SET.figures = true; });
await page.keyboard.press("Space");
await wait(2200);
await shot("4-figures");
const box = await page.$eval(".vw-stage", (el) => { const r = el.getBoundingClientRect(); return [r.x, r.y, r.width, r.height]; });
// Pick a man: click the middle of the field, where the play is.
await page.mouse.click(box[0] + box[2] / 2, box[1] + box[3] / 2);
await wait(400);
await shot("5-pick");
// A cut-up: third downs.
where = "cutup";
await page.click('.vw-side [data-k="cuts"]');
await wait(300);
const third = await page.$('.vw-side [data-vw="cut"][data-g="down"][data-v="3rd"]');
if (third) {
  await third.click();
  await wait(200);
  await page.click('.vw-side [data-vw="cutplay"]');
  await wait(2500);
  await shot("6-cutup-third-downs");
}
// Drawing: an arrow on the paused film.
where = "draw";
await page.keyboard.press("Space");
await page.keyboard.press("d");
await wait(200);
await page.mouse.move(box[0] + box[2] * 0.4, box[1] + box[3] * 0.6);
await page.mouse.down();
await page.mouse.move(box[0] + box[2] * 0.55, box[1] + box[3] * 0.35, { steps: 8 });
await page.mouse.up();
await wait(600);
await shot("7-drawing");
await page.keyboard.press("Escape");
// Full screen.
where = "full";
await page.keyboard.press("f");
await wait(1500);
await shot("8-full-2d");
await page.keyboard.press("Escape");
await wait(600);

if (process.env.AFM_CLIP !== "0") {
  // The 3D view: on in the View panel (the default), then V.
  where = "3d";
  await page.evaluate(() => { AFMViews.SET.three = true; AFMViews.SET.figures = false; AFMViews.saveSettings(); });
  await page.click('.vw-side [data-k="view"]');
  await page.keyboard.press("f");
  await wait(400);
  await page.keyboard.press("v");
  await page.waitForFunction(() => !!window.AFMViews3D, null, { timeout: 30000 });
  await wait(4000);
  await shot("9-3d-coaches");
  for (const c of ["2", "3"]) {
    await page.keyboard.press(c);
    await wait(2500);
    await shot(`10-3d-cam-${c}`);
  }
  await page.keyboard.press("1");
  const fps = await page.evaluate(() => new Promise((res) => {
    let n = 0;
    const t0 = performance.now();
    const f = () => { n++; if (performance.now() - t0 < 3000) requestAnimationFrame(f); else res((n * 1000) / (performance.now() - t0)); };
    requestAnimationFrame(f);
  }));
  console.log(`3D frames a second here (software GL): ${fps.toFixed(1)}`);

}
if (process.env.AFM_CLIP !== "0" && process.env.AFM_REC !== "0") {
  // The clip: three snaps in 3D at an even 30 frames a second, each frame
  // stepped by hand so a slow software renderer still gives smooth film.
  where = "record";
  // Recorded at High, what an M1 runs; software GL just takes longer.
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.evaluate(() => {
    AFMViews.SET.quality = "high";
    const p = AFMViews.active();
    if (p.view && p.view.setQuality) p.view.setQuality("high");
    // The film alone: the panel folded away, the tools check's drawings
    // hidden.
    if (AFMViews.SET.side !== false) p.toggleSide();
    AFMViews.SET.marks = false;
  });
  await wait(1500);
  const picks = await page.evaluate(() => {
    const p = AFMViews.active();
    const plays = p.film.ix.plays.filter((x) => x.tracked && x.tags.unit === "scrimmage");
    const pass = plays.find((x) => x.key && x.kind === "Pass" && x.yards >= 15) || plays.find((x) => x.kind === "Pass" && x.yards >= 10);
    const run = plays.find((x) => x.kind === "Run" && x.yards >= 8 && x !== pass) || plays.find((x) => x.kind === "Run");
    const sack = plays.find((x) => x.kind === "Sack");
    return [pass && [pass.n, "coaches"], run && [run.n, "broadcast"], sack && [sack.n, "all22"]].filter(Boolean);
  });
  const frames = `${out}/frames`;
  fs.rmSync(frames, { recursive: true, force: true });
  fs.mkdirSync(frames, { recursive: true });
  let f = 0;
  const grab = async () => page.screenshot({ path: `${frames}/${String(f++).padStart(5, "0")}.jpg`, type: "jpeg", quality: 92 });
  for (const [n, cam] of picks) {
    await page.evaluate(async ([n, cam]) => {
      const p = AFMViews.active();
      p.manual = true;
      AFMViews.SET.auto = false;
      p.setList(p.listFor("all"), null);
      await p.goTo(p.list.indexOf(n), { play: true, card: true });
      p.setCam(cam);
    }, [n, cam]);
    for (let k = 0; k < 30 * 30; k++) {
      const done = await page.evaluate(() => {
        const p = AFMViews.active();
        p.frameStep(1 / 30);
        return !p.playing && p.inter <= 0;
      });
      await grab();
      if (done) break;
    }
    for (let k = 0; k < 20; k++) await grab();
  }
  console.log(`clip: ${f} frames of ${picks.length} snaps`);
}
console.log(problems.length ? problems.join("\n") : "no problems");
await browser.close();
