// Clicks through the game's screens in a real browser (final build rule B6):
// opens a world, starts a career at a seat, then visits every sidebar screen
// and up to N links on each, presses Continue between rounds, and reports
// every page error, console error and failed call, with a screenshot per
// screen. Needs afm_play running (cargo run --release --bin afm_play).
//
// Usage: node tools/seat-ui/crawl.mjs <world file> <seat: hs|college|nfl> [rounds] [out dir]
// PLAYWRIGHT_BROWSERS_PATH=/opt/pw-browsers, AFM_URL (default http://127.0.0.1:7700).
import { createRequire } from "module";
const require = createRequire(import.meta.url);
let pw;
try { pw = require("playwright"); } catch { pw = require("/opt/node22/lib/node_modules/playwright"); }
import fs from "fs";

const [world, seat = "college", roundsArg = "4", out = "/tmp/afm-crawl"] = process.argv.slice(2);
const rounds = Number(roundsArg);
const url = process.env.AFM_URL || "http://127.0.0.1:7700";
fs.mkdirSync(out, { recursive: true });

const problems = [];
const browser = await pw.chromium.launch();
const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
let where = "start";
page.on("pageerror", (e) => problems.push(`[${where}] page error: ${e.message}`));
page.on("console", (m) => { if (m.type() === "error") problems.push(`[${where}] console: ${m.text()}`); });
page.on("response", async (r) => {
  if (r.url().includes("/api/")) {
    try { const j = await r.json(); if (j.err !== undefined) problems.push(`[${where}] ${r.url().split("/api/")[1]}: ${j.err}`); } catch {}
  } else if (r.status() >= 400) problems.push(`[${where}] ${r.status()} ${r.url()}`);
});

// Count the game's calls in flight, so a screenshot waits for its screen.
await page.addInitScript(() => {
  const f = window.fetch.bind(window);
  window.__inflight = 0;
  window.fetch = async (...a) => { window.__inflight++; try { return await f(...a); } finally { window.__inflight--; } };
});
const settle = async (ms = 150) => {
  await page.waitForTimeout(ms);
  await page.waitForFunction(() => (window.__inflight || 0) === 0, null, { timeout: 20000 }).catch(() => {});
  await page.waitForTimeout(150);
};

const api = (cmd, args = {}) => page.evaluate(async ([c, a]) => {
  const r = await fetch("/api/" + c, { method: "POST", body: JSON.stringify(a) });
  return r.json();
}, [cmd, args]);

await page.goto(url);
await page.waitForTimeout(800);
let r = await api("open_world", { path: world });
if (r.err) { console.log("open_world:", r.err); process.exit(2); }
const jobs = (await api("jobs", { level: seat })).ok.jobs;
const job = jobs[Math.min(2, jobs.length - 1)];
const bg = { hs: "hs_coach", nfl: "nfl_assistant", college: "college_assistant" }[seat];
r = await api("start_career", { name: "Crawl Tester", age: 44, background: bg, job: job.ref, seed: 3 });
if (r.err) r = await api("start_career", { name: "Crawl Tester", age: 44, background: "college_assistant", job: job.ref, seed: 3 });
if (r.err) { console.log("start_career:", r.err); process.exit(2); }
// Enter the game screens the way the app does after taking a job.
await page.evaluate(() => (typeof enterCareer === "function" ? enterCareer() : null));
await page.waitForTimeout(1200);

// Big moments and open letters cover the page: close them first.
const clear = async () => {
  for (let k = 0; k < 6; k++) {
    const b = await page.$("[data-act=revealnext], [data-act=revealclose], [data-act=letterclose], .reveal-room button");
    if (!b) break;
    await b.click({ timeout: 2000 }).catch(() => {});
    await page.waitForTimeout(300);
  }
  await page.keyboard.press("Escape").catch(() => {});
};
const shot = async (name) => {
  const f = `${out}/${seat}-${name.replace(/[^a-z0-9_-]+/gi, "_")}.png`;
  await page.screenshot({ path: f, fullPage: false });
};
const linksPer = Number(process.env.AFM_LINKS || 6);
const seen = new Set();
for (let round = 0; round < rounds; round++) {
  const nav = await page.$$eval("aside.side [data-go], aside.side [data-nav]", (els) =>
    [...new Set(els.map((e) => e.getAttribute("data-nav") || e.getAttribute("data-go")))]);
  for (const screen of nav) {
    where = `r${round}:${screen}`;
    await clear();
    const el = await page.$(`aside.side [data-nav="${screen}"], aside.side [data-go="${screen}"]`);
    if (!el) continue;
    await clear();
    await el.click({ timeout: 5000 }).catch((e) => problems.push(`[${where}] click: ${e.message.split("\n")[0]}`));
    await settle(300);
    if (!seen.has(screen)) { await shot(`${screen}`); seen.add(screen); }
    // Follow a few links on the screen, then come back.
    const n = await page.$$eval(".view [data-go], main [data-go]", (els) => els.length).catch(() => 0);
    for (let k = 0; k < Math.min(n, linksPer); k++) {
      const links = await page.$$(".view [data-go], main [data-go]");
      const pick = links[Math.floor((k * 7919 + round * 31) % links.length)];
      if (!pick) break;
      const go = await pick.getAttribute("data-go");
      where = `r${round}:${screen}>${go}`;
      await clear();
      await pick.click({ timeout: 3000 }).catch(() => {});
      await settle(200);
      if (!seen.has(go)) { await shot(go); seen.add(go); }
      // The game's own Back (the browser's would leave the app).
      await clear();
      const back = await page.$("[data-act=back]");
      if (back) await back.click({ timeout: 2000 }).catch(() => {});
      await settle(100);
    }
  }
  // Continue (space bar) a few times between rounds.
  where = `r${round}:continue`;
  for (let c = 0; c < 3; c++) {
    const cbtn = await page.$("[data-act=continue]");
    if (cbtn) await cbtn.click().catch(() => {}); else await page.keyboard.press("Space").catch(() => {});
    await page.waitForTimeout(4000);
    for (let wait = 0; wait < 60 && (await page.$("#busy")); wait++) await page.waitForTimeout(1000);
    // Close any dialog the stop opened.
    const close = await page.$(".st-dialog button, dialog button");
    if (close) await close.click().catch(() => {});
  }
  await clear();
  await shot(`after-continue-${round}`);
}
await browser.close();
const uniq = [...new Set(problems)];
fs.writeFileSync(`${out}/${seat}-problems.txt`, uniq.join("\n"));
console.log(`${seat}: ${seen.size} screens seen, ${uniq.length} problems`);
for (const p of uniq.slice(0, 60)) console.log("  " + p);
process.exit(uniq.length ? 1 : 0);
