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

const shot = async (name) => {
  const f = `${out}/${seat}-${name.replace(/[^a-z0-9_-]+/gi, "_")}.png`;
  await page.screenshot({ path: f, fullPage: false });
};
const linksPer = Number(process.env.AFM_LINKS || 6);
const seen = new Set();
for (let round = 0; round < rounds; round++) {
  const nav = await page.$$eval("aside [data-go], nav [data-go]", (els) =>
    [...new Set(els.map((e) => e.getAttribute("data-go")))]);
  for (const screen of nav) {
    where = `r${round}:${screen}`;
    const el = await page.$(`aside [data-go="${screen}"], nav [data-go="${screen}"]`);
    if (!el) continue;
    await el.click().catch((e) => problems.push(`[${where}] click: ${e.message}`));
    await page.waitForTimeout(700);
    if (!seen.has(screen)) { await shot(`${screen}`); seen.add(screen); }
    // Follow a few links on the screen, then come back.
    const n = await page.$$eval(".view [data-go], main [data-go]", (els) => els.length).catch(() => 0);
    for (let k = 0; k < Math.min(n, linksPer); k++) {
      const links = await page.$$(".view [data-go], main [data-go]");
      const pick = links[Math.floor((k * 7919 + round * 31) % links.length)];
      if (!pick) break;
      const go = await pick.getAttribute("data-go");
      where = `r${round}:${screen}>${go}`;
      await pick.click({ timeout: 3000 }).catch(() => {});
      await page.waitForTimeout(500);
      if (!seen.has(go)) { await shot(go); seen.add(go); }
      await page.goBack().catch(() => {});
      await page.evaluate(() => (typeof goBack === "function" ? goBack() : null)).catch(() => {});
      await page.waitForTimeout(300);
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
  await shot(`after-continue-${round}`);
}
await browser.close();
const uniq = [...new Set(problems)];
fs.writeFileSync(`${out}/${seat}-problems.txt`, uniq.join("\n"));
console.log(`${seat}: ${seen.size} screens seen, ${uniq.length} problems`);
for (const p of uniq.slice(0, 60)) console.log("  " + p);
process.exit(uniq.length ? 1 : 0);
