// Live talks v3, lane N6: draws each conversation board from convo-fixtures.js in
// Chromium with the game's commands answered by the fixtures, and saves a
// screenshot of each. Usage: node convo-shots.js <out dir> [board ...]
// Needs nothing running: it serves app/ui itself.
"use strict";
const http = require("http"), fs = require("fs"), path = require("path");
const { chromium } = require("playwright");
const root = path.resolve(__dirname, "../../../app/ui");
const out = process.argv[2] || "/tmp/convo-shots";
const only = process.argv.slice(3);
fs.mkdirSync(out, { recursive: true });
global.window = global;
require("./convo-fixtures.js");
const F = global.CV_FIXTURES;
const types = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".svg": "image/svg+xml", ".png": "image/png", ".woff2": "font/woff2" };
const srv = http.createServer((q, r) => {
  const f = path.join(root, decodeURIComponent(q.url.split("?")[0]).replace(/^\/$/, "/index.html"));
  if (!f.startsWith(root) || !fs.existsSync(f) || fs.statSync(f).isDirectory()) { r.writeHead(404); return r.end(); }
  r.writeHead(200, { "content-type": types[path.extname(f)] || "application/octet-stream" });
  fs.createReadStream(f).pipe(r);
});
(async () => {
  await new Promise((r) => srv.listen(0, r));
  const port = srv.address().port;
  const b = await chromium.launch();
  for (const [name, fx] of Object.entries(F)) {
    if (only.length && !only.includes(name)) continue;
    const p = await b.newPage({ viewport: { width: 1440, height: 1000 } });
    p.on("pageerror", (e) => console.log(name, "page error:", e.message));
    p.on("console", (m) => (m.type() === "error" || process.env.CV_ALL) && console.log(name, m.type(), m.text().slice(0, 300)));
    const loaded = new Promise((r) => p.once("load", r));
    await p.route("**/api/*", async (route) => {
      await loaded;
      const cmd = route.request().url().split("/api/")[1];
      const args = JSON.parse(route.request().postData() || "{}");
      let ok = {};
      if (cmd === "hello") ok = { loaded: true };
      else if (cmd === "status") ok = { mine: { colors: fx.colors, name: "South Carolina", short: "SC", level: "college", ref: { kind: "college", id: 1 }, mark: "SC" }, you: { name: "Cain", role: "Head coach" }, date: "Tue 13 Oct 2026", dateText: "Tue 13 Oct 2026", reveals: 0, mail: 3, nav: {}, flags: {} };
      else if (cmd === "people_scene") ok = fx.scene;
      else if (cmd === "talk_asking") ok = fx.asking;
      else if (cmd === "people_player") ok = fx.pp;
      else if (cmd === "peek") ok = fx.peek || {};
      else if (cmd === "face") ok = null;
      else if (cmd === "ai_status") ok = { fake: false, key_set: true };
      if (cmd === "people_player" && !fx.pp) return route.fulfill({ contentType: "application/json", body: JSON.stringify({ err: "No." }) });
      route.fulfill({ contentType: "application/json", body: JSON.stringify({ ok }) });
    });
    await p.goto(`http://127.0.0.1:${port}/#${fx.asking ? "asking" : "people_scene?id=" + fx.scene.id}`);
    await p.waitForSelector(".cv", { timeout: 8000 }).catch(() => console.log(name, "no .cv"));
    await p.waitForTimeout(500);
    if (process.env.CV_EVAL) console.log(JSON.stringify(await p.evaluate(process.env.CV_EVAL)));
    await p.screenshot({ path: path.join(out, name + ".png") });
    await p.close();
  }
  await b.close();
  srv.close();
})();
