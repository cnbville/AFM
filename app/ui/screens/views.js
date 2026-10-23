// Lane screens: views (docs/final-build/plan.md). The film room: watching
// your games back (later extras topics 2 and 3, screens topic 12, game day
// "watching"). One player for every way in: the game page's Watch tab, a
// full-screen watch mode, and the snaps of a game you are playing live.
//
// The game plays the film again from its seed and hands it over a snap at
// a time (film, film_play). Every snap is in the stadium's frame: x from the
// left end line (0) to the right one (120), y from the far sideline (0) to
// the near one (53.3). The 2D view draws it from above (dots, or figures as
// a setting); the 3D coaches' view (views/views3d.js, three.js vendored so
// the Mac app works offline) is the other side of one switch.
"use strict";

(() => {
  // ---------------------------------------------------------------- settings
  const SKEY = "afm.views.v1";
  const DEFAULTS = {
    view: "3d", // the view the watch screen opens in: the last one used (3D the first time)
    figures: false, // 2D: top-down figures instead of dots
    three: true, // the 3D coaches' view (Cain okayed the clip 2026-10-23); off leaves 2D only
    side: true, // the panel beside the film
    quality: "auto",
    cam2d: "all22",
    cam3d: "coaches",
    speed: 1,
    auto: true, // play on into the next snap
    replay: true, // big plays replay from a second angle (3D)
    trails: true,
    sep: true,
    names: false,
    officials: true,
    tiles: true,
    marks: true,
    pace: "every", // every snap, or key plays only
  };
  function loadSettings() {
    try {
      return { ...DEFAULTS, ...JSON.parse(localStorage.getItem(SKEY) || "{}") };
    } catch (e) {
      return { ...DEFAULTS };
    }
  }
  const SET = loadSettings();
  function saveSettings() {
    try {
      localStorage.setItem(SKEY, JSON.stringify(SET));
    } catch (e) {
      /* private window: settings last for the session */
    }
  }

  // ---------------------------------------------------------------- colours
  function hex(c) {
    const m = String(c || "#444444").replace("#", "");
    const v = m.length === 3 ? m.split("").map((x) => x + x).join("") : m.padEnd(6, "0");
    return [parseInt(v.slice(0, 2), 16), parseInt(v.slice(2, 4), 16), parseInt(v.slice(4, 6), 16)];
  }
  const lum = (c) => {
    const [r, g, b] = hex(c).map((v) => {
      v /= 255;
      return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
    });
    return 0.2126 * r + 0.7152 * g + 0.0722 * b;
  };
  const contrast = (a, b) => {
    const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p);
    return (x + 0.05) / (y + 0.05);
  };
  function mixHex(a, b, t) {
    const p = hex(a), q = hex(b);
    return "#" + p.map((v, i) => Math.round(v + (q[i] - v) * t).toString(16).padStart(2, "0")).join("");
  }
  // Uniforms: the home team in its colour, the visitors in white (NCAA and
  // NFHS rules; NFL custom), numbers in the colour that reads.
  function uniforms(teams) {
    return teams.map((t, side) => {
      const [c1, c2] = t.colors || ["#555555", "#999999"];
      const pick = (bg, ...opts) => opts.find((o) => contrast(bg, o) >= 3) || (lum(bg) > 0.4 ? "#111111" : "#ffffff");
      const home = side === 0;
      const jersey = home ? c1 : "#f2f2f0";
      const num = home ? pick(c1, c2, "#ffffff", "#111111") : pick(jersey, c1, c2, "#111111");
      const trim = home ? (contrast(c1, c2) > 1.6 ? c2 : "#ffffff") : c1;
      const helmet = lum(c1) < 0.02 && contrast(c1, c2) > 2 ? c1 : c1;
      const stripe = contrast(helmet, c2) > 1.5 ? c2 : "#ffffff";
      const pants = home ? (lum(c2) > 0.5 ? c2 : "#e9e9e6") : contrast("#f2f2f0", c2) > 1.4 ? c2 : "#dcdcd8";
      return { jersey, num, trim, helmet, stripe, pants, c1, c2, dot: jersey, ring: home ? (contrast(c1, "#ffffff") > 2 ? "#ffffff" : "#111111") : c1 };
    });
  }

  // ---------------------------------------------------------------- the film
  // A film: the index (teams, venue, players, plays) and the snaps loaded so
  // far. `source` says where snaps come from: a kept game, or the game you
  // are playing live.
  const ACT = { stand: 0, stance: 1, run: 2, block: 3, engaged: 4, rush: 5, cover: 6, pocket: 7, throw: 8, catch: 9, carry: 10, tackle: 11, wrapped: 12, fall: 13, down: 14, getup: 15, celebrate: 16, fake: 17, kneel: 18 };
  const ACT_WORDS = ["standing", "in his stance", "running", "blocking", "taking on a block", "rushing", "in coverage", "in the pocket", "throwing", "catching", "carrying", "tackling", "wrapped up", "going down", "down", "getting up", "celebrating", "faking", "kneeling"];

  class Film {
    constructor(index, source) {
      this.ix = index;
      this.source = source;
      this.clips = new Map();
      this.loading = new Map();
      this.players = new Map();
      for (const p of index.players) {
        const [id, side, num, pos, name, ht, wt, arm, skin] = p;
        this.players.set(id, { id, side, num, pos, name, ht, wt, arm, skin, last: String(name).split(" ").slice(-1)[0] });
      }
      this.uni = uniforms(index.teams);
      this.byN = new Map(index.plays.map((p, i) => [p.n, i]));
    }
    play(n) {
      return this.ix.plays[this.byN.get(n)];
    }
    // One snap, decoded off the critical path: fetched, then turned into
    // typed arrays in an idle slice so a playing view never stalls.
    load(n) {
      if (this.clips.has(n)) return Promise.resolve(this.clips.get(n));
      if (this.loading.has(n)) return this.loading.get(n);
      const p = this.source.clip(n).then(
        (raw) =>
          new Promise((res) => {
            const go = () => {
              const c = decode(raw, this);
              this.clips.set(n, c);
              this.loading.delete(n);
              res(c);
            };
            if (window.requestIdleCallback) requestIdleCallback(go, { timeout: 120 });
            else setTimeout(go, 0);
          }),
      );
      p.catch(() => this.loading.delete(n));
      this.loading.set(n, p);
      // Keep the last sixty snaps decoded; older ones load again if asked.
      if (this.clips.size > 60) this.clips.delete(this.clips.keys().next().value);
      return p;
    }
    prefetch(ns) {
      for (const n of ns) if (!this.clips.has(n) && !this.loading.has(n)) this.load(n).catch(() => {});
    }
  }

  // A snap's numbers: every frame's ball and men in a Float32Array, the
  // distance each man has run (for strides), the officials' spots.
  function decode(raw, film) {
    const S = raw.stride, f = Float32Array.from(raw.f), n = raw.ids.length;
    const frames = f.length / S;
    const t = new Float32Array(frames);
    for (let k = 0; k < frames; k++) t[k] = f[k * S];
    const walked = new Float32Array(frames * n);
    for (let k = 1; k < frames; k++) {
      for (let i = 0; i < n; i++) {
        const a = (k - 1) * S + 6 + i * 5, b = k * S + 6 + i * 5;
        walked[k * n + i] = walked[(k - 1) * n + i] + Math.hypot(f[b] - f[a], f[b + 1] - f[a + 1]);
      }
    }
    // c.n stays the play's number; c.men is how many men it tracks.
    const c = { ...raw, f, S, men: n, frames, t, walked, t0: t[0], t1: t[frames - 1] };
    c.snapAt = 0;
    c.throwAt = null;
    c.catchAt = null;
    for (let k = 1; k < frames; k++) {
      const mode = f[k * S + 4], was = f[(k - 1) * S + 4];
      if (c.throwAt == null && mode === 1 && was === 0 && t[k] > 0) c.throwAt = t[k - 1];
      if (c.throwAt != null && c.catchAt == null && was === 1 && mode !== 1) c.catchAt = t[k];
    }
    c.side = raw.ids.map((id) => (film.players.get(id) || { side: 0 }).side);
    c.crew = officials(c, film);
    return c;
  }

  // The frame index at time t and the share toward the next one.
  function at(c, t) {
    if (t <= c.t0) return [0, 0];
    if (t >= c.t1) return [c.frames - 1, 0];
    let lo = 0, hi = c.frames - 1;
    while (hi - lo > 1) {
      const m = (lo + hi) >> 1;
      if (c.t[m] <= t) lo = m;
      else hi = m;
    }
    return [lo, (t - c.t[lo]) / Math.max(1e-6, c.t[lo + 1] - c.t[lo])];
  }
  const angLerp = (a, b, u) => {
    let d = b - a;
    while (d > Math.PI) d -= 2 * Math.PI;
    while (d < -Math.PI) d += 2 * Math.PI;
    return a + d * u;
  };
  // Where everything is at time t, smoothed from the engine's ten frames a
  // second to the screen's sixty.
  function sample(c, t, out) {
    const [k, u] = at(c, t);
    const k2 = Math.min(c.frames - 1, k + 1), S = c.S, f = c.f;
    const A = k * S, B = k2 * S;
    const L = (o) => f[A + o] + (f[B + o] - f[A + o]) * u;
    out = out || { men: Array.from({ length: c.men }, () => ({})), ball: {} };
    out.t = t;
    out.k = k;
    out.ball.x = L(1);
    out.ball.y = L(2);
    out.ball.z = L(3);
    const near = u < 0.5 ? A : B;
    out.ball.mode = f[near + 4];
    out.ball.holder = f[near + 5];
    for (let i = 0; i < c.men; i++) {
      const o = 6 + i * 5, m = out.men[i];
      m.x = L(o);
      m.y = L(o + 1);
      m.face = angLerp(f[A + o + 2], f[B + o + 2], u);
      m.act = f[near + o + 3];
      m.with = f[near + o + 4];
      m.walked = c.walked[k * c.men + i] + (c.walked[k2 * c.men + i] - c.walked[k * c.men + i]) * u;
      const dt = Math.max(0.05, c.t[k2] - c.t[k]);
      m.speed = k2 > k ? Math.hypot(f[B + o] - f[A + o], f[B + o + 1] - f[A + o + 1]) / dt : 0;
      m.i = i;
    }
    // The officials and the chains.
    out.crew = c.crew.map((o) => {
      const [kk, uu] = at({ t: o.t, t0: o.t[0], t1: o.t[o.t.length - 1], frames: o.t.length }, t);
      const k3 = Math.min(o.t.length - 1, kk + 1);
      return { role: o.role, x: o.x[kk] + (o.x[k3] - o.x[kk]) * uu, y: o.y[kk] + (o.y[k3] - o.y[kk]) * uu, face: o.face, flag: o.flag && t >= o.flag.t ? o.flag : null };
    });
    return out;
  }

  // The officials for the level and era in their real spots (NFL seven,
  // major college eight from 2023, high school five), moving with the play.
  function officials(c, film) {
    const v = film.ix.venue, count = v.officials || 7, level = film.ix.level;
    const los = c.los != null ? c.los : null;
    const dir = c.dir || 1;
    const S = c.S, f = c.f;
    const roles =
      count === 5 ? ["R", "U", "HL", "LJ", "BJ"] : count === 8 ? ["R", "U", "DJ", "LJ", "SJ", "FJ", "BJ", "CJ"] : ["R", "U", "DJ", "LJ", "SJ", "FJ", "BJ"];
    // Each official's spot relative to the ball at the snap: (yards toward
    // the defense, yards across from the ball, or a sideline).
    const NEAR = 53.33 + 1.2, FAR = -1.2;
    const base = (role, bx, by) => {
      const back = (d) => bx - dir * d;
      const ahead = (d) => bx + dir * d;
      const umpBack = level === "nfl";
      switch (role) {
        case "R": return [back(12), by + dir * 4.5];
        case "U": return umpBack ? [back(13), by - dir * 4.5] : [ahead(6), by];
        case "CJ": return [back(12), by - dir * 4.5];
        case "DJ": case "HL": return [bx, FAR];
        case "LJ": return [bx, NEAR];
        case "SJ": return [ahead(20), FAR + 0.4];
        case "FJ": return [ahead(20), NEAR - 0.4];
        case "BJ": return [ahead(count === 5 ? 18 : 25), 26.67 + 3];
      }
      return [bx, by];
    };
    const first = 0;
    const bx0 = los != null ? los : f[first * S + 1];
    const by0 = Math.min(40, Math.max(13, f[first * S + 2]));
    const out = [];
    for (const role of roles) {
      const xs = new Float32Array(c.frames), ys = new Float32Array(c.frames);
      let [x, y] = base(role, bx0, by0);
      const wing = role === "DJ" || role === "HL" || role === "LJ";
      const deep = role === "SJ" || role === "FJ" || role === "BJ";
      for (let k = 0; k < c.frames; k++) {
        const t = c.t[k];
        const bx = f[k * S + 1], by = f[k * S + 2];
        let tx = x, ty = y;
        if (t > 0.3) {
          if (wing) tx = bx; // the wings mirror the ball down their line
          else if (deep) {
            // Keep the ball between them and the goal line they guard.
            const want = bx + dir * (role === "BJ" ? 12 : 8);
            tx = dir > 0 ? Math.max(x, want) : Math.min(x, want);
            if (role === "BJ") ty = 26.67 + (by - 26.67) * 0.4;
          } else {
            // The referee and the umpire close on the ball behind the play.
            const d = Math.hypot(bx - x, by - y);
            if (d > 9) { tx = bx - (bx - x) * (9 / d); ty = by - (by - y) * (9 / d); }
          }
        }
        const dx = tx - x, dy = ty - y, d = Math.hypot(dx, dy), step = 0.62; // about 6 yards a second
        if (d > step) { x += (dx / d) * step; y += (dy / d) * step; } else { x = tx; y = ty; }
        xs[k] = x;
        ys[k] = y;
      }
      out.push({ role, t: c.t, x: xs, y: ys, face: role === "LJ" ? -Math.PI / 2 : role === "DJ" || role === "HL" ? Math.PI / 2 : dir > 0 ? Math.PI : 0 });
    }
    // Flags: the nearest official to the fouler throws it when the foul
    // comes.
    for (const fl of c.flags || []) {
      const i = c.ids.indexOf(fl.id);
      const [k] = at(c, fl.t);
      const px = i >= 0 ? f[k * S + 6 + i * 5] : f[k * S + 1], py = i >= 0 ? f[k * S + 7 + i * 5] : f[k * S + 2];
      let best = null, bd = 1e9;
      for (const o of out) {
        const d = Math.hypot(o.x[k] - px, o.y[k] - py);
        if (d < bd) { bd = d; best = o; }
      }
      if (best && !best.flag) best.flag = { t: fl.t, x: px + (best.x[k] - px) * 0.35, y: py + (best.y[k] - py) * 0.35, what: fl.what };
    }
    // The chain crew on the far sideline: the rods at the series' start
    // and the line to gain, the box at the ball.
    if (c.los != null) {
      const box = { role: "box", t: new Float32Array([c.t0]), x: new Float32Array([c.los]), y: new Float32Array([-2.4]), face: Math.PI / 2 };
      out.push(box);
      if (c.ltg != null && c.series != null) {
        out.push({ role: "rod", t: new Float32Array([c.t0]), x: new Float32Array([c.series]), y: new Float32Array([-2.4]), face: Math.PI / 2 });
        out.push({ role: "rod", t: new Float32Array([c.t0]), x: new Float32Array([c.ltg]), y: new Float32Array([-2.4]), face: Math.PI / 2 });
      }
    }
    return out;
  }

  // ---------------------------------------------------------------- the field
  // The field at 24 pixels a yard, drawn once per film: markings by level
  // (hash marks, numbers, arrows), end zones in the home team's colours with
  // its name, the midfield mark, grass mowed in stripes or turf, the bench
  // areas, the apron or the track.
  const PX = 24;
  const FIELD_W = 120, FIELD_H = 160 / 3;
  const MARGIN_X = 12, MARGIN_Y = 9;
  // The field as paint. The 3D view asks for the grass as it is under the
  // sun (`natural`): less of the screen's saturated green, so lit and tone
  // mapped it still reads as turf.
  // A team's monogram for the midfield mark, the way real fields carry one:
// an NFL club's city (KC, NY, SF, a lone C for Chicago), a college's own
// acronym (UCLA, LSU) or its block letter (M, O), a high school's initials.
function monogram(name, level) {
  const words = String(name || "").split(/\s+/).filter(Boolean);
  if (level === "nfl" && words.length > 1) words.pop();
  const caps = (w) => /^[A-Z&]{2,4}$/.test(w);
  if (words.length === 1 && caps(words[0])) return words[0].replace(/&/g, "");
  const kept = words.filter((w) => /^[A-Z]/.test(w) && !/^(State|University|High|School|HS|Academy|Prep)$/.test(w));
  const use = kept.length ? kept : words;
  if (!use.length) return "";
  if (use.length === 1) return use[0][0].toUpperCase();
  return use.map((w) => (caps(w) ? w.replace(/&/g, "") : w[0].toUpperCase())).join("").slice(0, 3);
}

function paintField(film, opts = {}) {
    const ix = film.ix, v = ix.venue, home = ix.teams[0], u = film.uni[0];
    const nat = !!opts.natural;
    const W = (FIELD_W + 2 * MARGIN_X) * PX, H = (FIELD_H + 2 * MARGIN_Y) * PX;
    const cv = document.createElement("canvas");
    cv.width = W;
    cv.height = H;
    const g = cv.getContext("2d");
    const X = (x) => (x + MARGIN_X) * PX, Y = (y) => (y + MARGIN_Y) * PX;
    const turf = v.surface === "turf";
    const wet = v.rain, snow = v.snow;
    // Surround: a track at a high school that has one, else the apron.
    g.fillStyle = nat ? (turf ? "#3b5a3c" : "#44603a") : turf ? "#1d4a2a" : "#22502b";
    g.fillRect(0, 0, W, H);
    if (v.track) {
      g.fillStyle = "#7a3a2c";
      const r = (FIELD_H / 2 + 7) * PX;
      g.beginPath();
      g.roundRect ? g.roundRect(X(-9), Y(-8), (FIELD_W + 18) * PX, (FIELD_H + 16) * PX, r) : g.rect(X(-9), Y(-8), (FIELD_W + 18) * PX, (FIELD_H + 16) * PX);
      g.fill();
      g.strokeStyle = "rgba(255,255,255,.35)";
      g.lineWidth = 1.2;
      for (let lane = 1; lane < 6; lane++) {
        g.beginPath();
        const o = lane * 1.1;
        g.roundRect ? g.roundRect(X(-9 + o), Y(-8 + o), (FIELD_W + 18 - 2 * o) * PX, (FIELD_H + 16 - 2 * o) * PX, r - o * PX) : 0;
        g.stroke();
      }
      g.fillStyle = nat ? (turf ? "#3f603f" : "#4a663d") : turf ? "#1f5230" : "#2a5f31";
      g.fillRect(X(-5.5), Y(-4.5), (FIELD_W + 11) * PX, (FIELD_H + 9) * PX);
    }
    // The playing field: stripes every five yards.
    for (let i = 0; i < 24; i++) {
      const x0 = i * 5;
      const light = i % 2 === 0;
      g.fillStyle = nat
        ? turf
          ? light ? "#4e7646" : "#476d41"
          : light ? "#5a7d43" : "#51723d"
        : turf
          ? light ? "#2b6e37" : "#276633"
          : light ? "#2f7a39" : "#2a6f34";
      g.fillRect(X(x0), Y(0), 5 * PX + 1, FIELD_H * PX);
    }
    if (!turf) {
      // Grass: a fine grain across the stripes.
      g.globalAlpha = 0.06;
      for (let y = 0; y < FIELD_H * PX; y += 3) {
        g.fillStyle = (y / 3) % 2 ? "#000" : "#fff";
        g.fillRect(X(0), Y(0) + y, FIELD_W * PX, 1);
      }
      g.globalAlpha = 1;
    }
    // End zones in the home team's colours.
    const ez = mixHex(home.colors[0], "#000000", 0.18);
    g.fillStyle = ez;
    g.fillRect(X(0), Y(0), 10 * PX, FIELD_H * PX);
    g.fillRect(X(110), Y(0), 10 * PX, FIELD_H * PX);
    const nameColor = contrast(ez, home.colors[1]) > 2.2 ? home.colors[1] : "#ffffff";
    g.save();
    g.fillStyle = nameColor;
    g.strokeStyle = "rgba(0,0,0,.35)";
    g.lineWidth = 3;
    g.textAlign = "center";
    g.textBaseline = "middle";
    const name = String(home.name).toUpperCase();
    const size = Math.min(6.2, (FIELD_H * 0.86) / Math.max(6, name.length * 0.62)) * PX;
    g.font = `900 ${size}px Archivo, "Arial Black", sans-serif`;
    for (const [x, r] of [[5, -Math.PI / 2], [115, Math.PI / 2]]) {
      g.save();
      g.translate(X(x), Y(FIELD_H / 2));
      g.rotate(r);
      g.strokeText(name, 0, 0);
      g.fillText(name, 0, 0);
      g.restore();
    }
    g.restore();
    // Lines: four inches wide (0.11 yards), the goal lines eight in the NFL.
    const line = (x0, y0, x1, y1, w = 0.11) => {
      g.lineWidth = w * PX;
      g.beginPath();
      g.moveTo(X(x0), Y(y0));
      g.lineTo(X(x1), Y(y1));
      g.stroke();
    };
    g.strokeStyle = snow ? "rgba(255,255,255,.75)" : "#f5f5f2";
    g.lineCap = "butt";
    // The border: six feet of white outside the sidelines and end lines.
    g.fillStyle = "#f5f5f2";
    g.fillRect(X(-2), Y(-2), (FIELD_W + 4) * PX, 2 * PX);
    g.fillRect(X(-2), Y(FIELD_H), (FIELD_W + 4) * PX, 2 * PX);
    g.fillRect(X(-2), Y(0), 2 * PX, FIELD_H * PX);
    g.fillRect(X(FIELD_W), Y(0), 2 * PX, FIELD_H * PX);
    for (let x = 10; x <= 110; x += 5) line(x, 0, x, FIELD_H, x === 10 || x === 110 ? (ix.level === "nfl" ? 0.22 : 0.11) : 0.11);
    // Hash marks every yard: the inbounds lines and the sideline ticks.
    const hash = ix.hash || (ix.level === "nfl" ? 3.08 : ix.level === "college" ? 6.67 : 8.89);
    const mid = FIELD_H / 2;
    for (let x = 11; x < 110; x++) {
      if (x % 5 === 0) continue;
      line(x, 0.2, x, 0.87);
      line(x, FIELD_H - 0.87, x, FIELD_H - 0.2);
      line(x, mid - hash - 0.67, x, mid - hash);
      line(x, mid + hash, x, mid + hash + 0.67);
    }
    // The try line: a short dash at the three (college and high school) or
    // two (NFL) yard line.
    const tryAt = ix.level === "nfl" ? 2 : 3;
    for (const x of [10 + tryAt, 110 - tryAt]) line(x, mid - 0.5, x, mid + 0.5);
    // Numbers every ten yards, their tops toward the middle, arrows to the
    // nearer goal.
    const numY = ix.level === "nfl" ? 12 : ix.level === "college" ? 10 : 9;
    g.fillStyle = snow ? "rgba(255,255,255,.75)" : "#f5f5f2";
    g.font = `800 ${2 * PX}px Archivo, Arial, sans-serif`;
    g.textAlign = "center";
    g.textBaseline = "middle";
    for (let x = 20; x <= 100; x += 10) {
      const n = x <= 60 ? x - 10 : 110 - x;
      for (const [y, r] of [[numY, Math.PI], [FIELD_H - numY, 0]]) {
        g.save();
        g.translate(X(x), Y(y));
        g.rotate(r);
        g.fillText(String(n), 0, 0);
        if (n !== 50) {
          // The arrow points to the nearer goal line.
          const side = x < 60 ? -1 : 1;
          const ax = (r ? -side : side) * 1.85 * PX;
          g.beginPath();
          g.moveTo(ax + (r ? -side : side) * 0.45 * PX, -0.55 * PX);
          g.lineTo(ax + (r ? -side : side) * 0.45 * PX, -0.05 * PX);
          g.lineTo(ax + (r ? -side : side) * 0.95 * PX, -0.3 * PX);
          g.closePath();
          g.fill();
        }
        g.restore();
      }
    }
    // The midfield mark: the home team's monogram in a ring.
    const ini = monogram(home.name, ix.level);
    g.save();
    g.translate(X(60), Y(mid));
    g.fillStyle = mixHex(home.colors[0], "#000", 0.1);
    g.strokeStyle = nameColor;
    g.lineWidth = 0.25 * PX;
    g.beginPath();
    g.arc(0, 0, 4.2 * PX, 0, Math.PI * 2);
    g.fill();
    g.stroke();
    g.fillStyle = nameColor;
    g.font = `900 ${[3.6, 3.6, 3.0, 2.4][ini.length - 1] * PX}px Archivo, "Arial Black", sans-serif`;
    g.fillText(ini, 0, 0.2 * PX, 7 * PX);
    g.restore();
    // The team areas: between the 25s (the 32s in the NFL), the coaching
    // box in front, the benches behind. Home on the near side.
    const area = ix.level === "nfl" ? 32 : 25;
    for (const [side, y0, y1] of [[0, FIELD_H + 2.6, FIELD_H + 6.6], [1, -6.6, -2.6]]) {
      const t = ix.teams[side];
      g.fillStyle = "rgba(0,0,0,.18)";
      g.fillRect(X(10 + area), Y(y0), (100 - 2 * area) * PX, (y1 - y0) * PX);
      g.strokeStyle = "rgba(255,255,255,.7)";
      g.setLineDash([0.5 * PX, 0.4 * PX]);
      g.lineWidth = 0.1 * PX;
      g.strokeRect(X(10 + area), Y(y0), (100 - 2 * area) * PX, (y1 - y0) * PX);
      g.setLineDash([]);
      g.fillStyle = mixHex(t.colors[0], "#000", 0.25);
      const by = side === 0 ? y1 - 1.1 : y0;
      for (let k = 0; k < 4; k++) g.fillRect(X(10 + area + 4 + k * ((100 - 2 * area - 8) / 4)), Y(by), ((100 - 2 * area - 8) / 4 - 1.2) * PX, 1.1 * PX);
    }
    // Goalposts seen from above: the crossbar at the back of each end zone.
    const halfPosts = ix.level === "hs" ? 23.33 / 6 : 18.5 / 6;
    g.strokeStyle = "#ffd60a";
    g.lineWidth = 0.22 * PX;
    for (const x of [0, 120]) {
      line(x, mid - halfPosts, x, mid + halfPosts, 0.22);
      line(x, mid, x + (x ? 1.6 : -1.6), mid, 0.22);
    }
    g.strokeStyle = "#f5f5f2";
    // Weather on the turf: a wet sheen, or snow building.
    if (wet) {
      const gr = g.createLinearGradient(0, 0, W, H);
      gr.addColorStop(0, "rgba(180,200,230,.10)");
      gr.addColorStop(1, "rgba(30,40,60,.16)");
      g.fillStyle = gr;
      g.fillRect(0, 0, W, H);
    }
    if (snow) {
      g.fillStyle = "rgba(240,244,250,.28)";
      g.fillRect(0, 0, W, H);
      for (let i = 0; i < 2600; i++) {
        const r = ((i * 9301 + 49297) % 233280) / 233280, s = ((i * 4421 + 1731) % 99991) / 99991;
        g.fillStyle = `rgba(255,255,255,${0.12 + 0.2 * r})`;
        g.fillRect(r * W, s * H, 2 + 3 * s, 2 + 3 * r);
      }
    }
    return cv;
  }

  // ---------------------------------------------------------------- the 2D view
  // Cameras: the coaches' All-22 (high from the sideline, all 22 in view,
  // following the ball), tight on the ball, the end-zone view (behind the
  // offense, turned so it attacks up the screen), and free (drag and zoom).
  class View2D {
    constructor(host, player) {
      this.player = player;
      this.cv = document.createElement("canvas");
      this.cv.className = "vw-canvas";
      host.appendChild(this.cv);
      this.g = this.cv.getContext("2d");
      this.cam = { x: 60, y: FIELD_H / 2, scale: 10, rot: 0 };
      this.free = null;
      this.hover = null;
      this.wire();
    }
    dispose() {
      this.cv.remove();
    }
    resize() {
      const r = this.cv.getBoundingClientRect();
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      const w = Math.max(1, Math.round(r.width * dpr)), h = Math.max(1, Math.round(r.height * dpr));
      if (this.cv.width !== w || this.cv.height !== h) {
        this.cv.width = w;
        this.cv.height = h;
      }
      this.dpr = dpr;
      return [w, h];
    }
    // Where the camera wants to be for this frame.
    target(st, c, w, h) {
      const mode = this.player.isolate != null ? "iso" : SET.cam2d;
      const ball = st.ball;
      const dir = c.dir || 1;
      if (mode === "free" && this.free) return { ...this.free, rot: 0 };
      if (mode === "iso") {
        const m = st.men[this.player.isolate] || ball;
        return { x: m.x, y: m.y, scale: Math.min(w / 34, h / 22), rot: 0 };
      }
      if (mode === "tight") return { x: ball.x + dir * 4, y: Math.min(FIELD_H - 8, Math.max(8, ball.y)), scale: Math.min(w / 40, h / 26), rot: 0 };
      if (mode === "endzone") {
        // Turned so the offense goes up the screen; 34 yards of depth.
        const s = Math.min(w / (FIELD_H + 8), h / 40);
        return { x: ball.x + dir * 8, y: FIELD_H / 2, scale: s, rot: dir > 0 ? -Math.PI / 2 : Math.PI / 2 };
      }
      // All-22: all 22 men inside, from the widest split to the deepest
      // safety, never tighter than a formation and its first steps.
      let lo = ball.x, hi = ball.x, ylo = ball.y, yhi = ball.y;
      for (const m of st.men) {
        lo = Math.min(lo, m.x);
        hi = Math.max(hi, m.x);
        ylo = Math.min(ylo, m.y);
        yhi = Math.max(yhi, m.y);
      }
      const span = Math.max(40, hi - lo + 14);
      const across = Math.min(FIELD_H + 10, Math.max(32, yhi - ylo + 12));
      const s = Math.min(w / span, h / across);
      const half = h / s / 2;
      const y = half >= FIELD_H / 2 + 5 ? FIELD_H / 2 : Math.max(half - 5, Math.min(FIELD_H + 5 - half, (ylo + yhi) / 2));
      return { x: (lo + hi) / 2, y, scale: s, rot: 0 };
    }
    toScreen(x, y) {
      const c = this.cam, w = this.cv.width, h = this.cv.height;
      const dx = x - c.x, dy = y - c.y;
      const cs = Math.cos(c.rot), sn = Math.sin(c.rot);
      return [w / 2 + (dx * cs - dy * sn) * c.scale, h / 2 + (dx * sn + dy * cs) * c.scale];
    }
    toField(px, py) {
      const c = this.cam, w = this.cv.width, h = this.cv.height;
      const dx = (px - w / 2) / c.scale, dy = (py - h / 2) / c.scale;
      const cs = Math.cos(-c.rot), sn = Math.sin(-c.rot);
      return [c.x + dx * cs - dy * sn, c.y + dx * sn + dy * cs];
    }
    wire() {
      const cv = this.cv;
      let drag = null;
      cv.addEventListener("pointerdown", (e) => {
        const p = this.player;
        const [px, py] = [e.offsetX * this.dpr, e.offsetY * this.dpr];
        if (p.tool) return p.drawStart(this.toField(px, py), e);
        drag = { px, py, cam: { ...this.cam }, moved: false };
        cv.setPointerCapture(e.pointerId);
      });
      cv.addEventListener("pointermove", (e) => {
        const p = this.player;
        const [px, py] = [e.offsetX * this.dpr, e.offsetY * this.dpr];
        if (p.tool) return p.drawMove(this.toField(px, py));
        if (drag) {
          const dx = (px - drag.px) / this.cam.scale, dy = (py - drag.py) / this.cam.scale;
          if (Math.abs(dx) + Math.abs(dy) > 0.5) drag.moved = true;
          if (drag.moved) {
            SET.cam2d = "free";
            this.free = { x: drag.cam.x - dx, y: drag.cam.y - dy, scale: drag.cam.scale };
            p.syncBar();
          }
        } else this.hoverAt(px, py);
      });
      cv.addEventListener("pointerup", (e) => {
        const p = this.player;
        if (p.tool) return p.drawEnd();
        if (drag && !drag.moved) {
          const hit = this.hitAt(e.offsetX * this.dpr, e.offsetY * this.dpr);
          if (hit != null) p.pick(hit);
        }
        drag = null;
      });
      cv.addEventListener(
        "wheel",
        (e) => {
          e.preventDefault();
          const k = Math.exp(-e.deltaY * 0.0015);
          const base = this.free || { ...this.cam };
          SET.cam2d = "free";
          this.free = { x: base.x, y: base.y, scale: Math.max(3, Math.min(60, base.scale * k)) };
          this.player.syncBar();
        },
        { passive: false },
      );
      cv.addEventListener("pointerleave", () => (this.hover = null));
    }
    hitAt(px, py) {
      const st = this.player.st;
      if (!st) return null;
      let best = null, bd = 1e9;
      for (const m of st.men) {
        const [sx, sy] = this.toScreen(m.x, m.y);
        const d = Math.hypot(sx - px, sy - py);
        if (d < bd) { bd = d; best = m.i; }
      }
      return bd < Math.max(14 * this.dpr, this.cam.scale * 1.4) ? best : null;
    }
    hoverAt(px, py) {
      this.hover = this.hitAt(px, py);
      this.cv.style.cursor = this.player.tool ? "crosshair" : this.hover != null ? "pointer" : "grab";
    }
    draw(st, c, film, dt) {
      const [w, h] = this.resize();
      const g = this.g;
      const want = this.target(st, c, w, h);
      // The camera eases toward its spot: a steady operator, not a jolt.
      const k = this.snapCam ? 1 : 1 - Math.pow(0.02, dt);
      this.snapCam = false;
      this.cam.x += (want.x - this.cam.x) * k;
      this.cam.y += (want.y - this.cam.y) * k;
      this.cam.scale += (want.scale - this.cam.scale) * k;
      this.cam.rot = angLerp(this.cam.rot, want.rot, k);
      const cam = this.cam;
      g.setTransform(1, 0, 0, 1, 0, 0);
      g.fillStyle = "#0d1a12";
      g.fillRect(0, 0, w, h);
      g.setTransform(cam.scale * Math.cos(cam.rot), cam.scale * Math.sin(cam.rot), -cam.scale * Math.sin(cam.rot), cam.scale * Math.cos(cam.rot), w / 2, h / 2);
      g.translate(-cam.x, -cam.y);
      // Field (yards): the painted canvas is in pixels from -margin.
      g.drawImage(film.fieldCanvas, -MARGIN_X, -MARGIN_Y, FIELD_W + 2 * MARGIN_X, FIELD_H + 2 * MARGIN_Y);
      const ys = 1 / cam.scale; // one screen pixel in yards
      // The line of scrimmage and the line to gain.
      if (c.los != null) {
        g.lineWidth = Math.max(0.2, 3 * ys);
        g.strokeStyle = "rgba(64,128,255,.9)";
        g.beginPath();
        g.moveTo(c.los, 0);
        g.lineTo(c.los, FIELD_H);
        g.stroke();
        if (c.ltg != null && c.ltg > 10 && c.ltg < 110) {
          g.strokeStyle = "rgba(255,214,10,.95)";
          g.beginPath();
          g.moveTo(c.ltg, 0);
          g.lineTo(c.ltg, FIELD_H);
          g.stroke();
        }
      }
      const p = this.player;
      // Drawings on the turf.
      p.drawMarks(g, ys, st.t);
      // Route trails: each eligible man's path since the snap, the target's
      // in gold.
      if (SET.trails && st.t > 0) this.trails(g, c, st, film, ys);
      const men = st.men;
      const dim = p.isolate != null;
      // The officials and the chain crew.
      if (SET.officials) for (const o of st.crew) this.official(g, o, ys);
      // The men, the ball carrier's ring, then the ball.
      const order = men.slice().sort((a, b) => a.y - b.y);
      for (const m of order) this.man(g, m, c, film, ys, dim && m.i !== p.isolate, st);
      if (SET.sep && c.target != null) this.separation(g, c, st, ys);
      this.ball(g, st.ball, ys);
      // Flags on the turf.
      for (const o of st.crew) {
        if (!o.flag) continue;
        const u = Math.min(1, (st.t - o.flag.t) / 0.45);
        const x = o.x + (o.flag.x - o.x) * u, y = o.y + (o.flag.y - o.y) * u;
        g.fillStyle = "#ffd60a";
        g.beginPath();
        g.arc(x, y, Math.max(0.35, 5 * ys) * (1 + (1 - u) * 0.6), 0, Math.PI * 2);
        g.fill();
      }
      // Names on hover, and the isolated man's name.
      const label = p.isolate != null ? p.isolate : this.hover;
      if (label != null && men[label]) this.nameTag(g, men[label], c, film, ys);
      g.setTransform(1, 0, 0, 1, 0, 0);
    }
    trails(g, c, st, film, ys) {
      const elig = new Set([c.target, c.carrier, c.passer].filter((x) => x != null));
      for (let i = 0; i < c.men; i++) {
        const id = c.ids[i];
        const pl = film.players.get(id);
        if (!pl) continue;
        const skill = ["WR", "TE", "RB"].includes(pl.pos) && c.side[i] === c.team;
        if (!skill && !elig.has(id)) continue;
        if (c.side[i] !== c.team && !elig.has(id)) continue;
        const isT = id === c.target, isC = id === c.carrier;
        g.strokeStyle = isT ? "rgba(255,214,10,.95)" : isC ? "rgba(255,255,255,.9)" : "rgba(255,255,255,.42)";
        g.lineWidth = Math.max(0.12, (isT ? 2.6 : 1.6) * ys);
        g.lineJoin = "round";
        g.beginPath();
        let started = false;
        for (let k = 0; k < c.frames && c.t[k] <= st.t; k++) {
          if (c.t[k] < 0) continue;
          const o = k * c.S + 6 + i * 5;
          if (!started) { g.moveTo(c.f[o], c.f[o + 1]); started = true; } else g.lineTo(c.f[o], c.f[o + 1]);
        }
        const m = st.men[i];
        if (started) g.lineTo(m.x, m.y);
        g.stroke();
      }
    }
    separation(g, c, st, ys) {
      const ti = c.ids.indexOf(c.target);
      if (ti < 0 || st.t <= 0 || (c.catchAt != null && st.t > c.catchAt + 0.4)) return;
      const m = st.men[ti];
      let d = 99;
      for (const o of st.men) if (c.side[o.i] !== c.side[ti]) d = Math.min(d, Math.hypot(o.x - m.x, o.y - m.y));
      const col = d > 2.5 ? "48,209,88" : d > 1.2 ? "255,214,10" : "255,55,95";
      g.strokeStyle = `rgba(${col},.9)`;
      g.lineWidth = Math.max(0.1, 2 * ys);
      g.beginPath();
      g.arc(m.x, m.y, Math.min(8, d), 0, Math.PI * 2);
      g.stroke();
      st.sepNow = d;
    }
    man(g, m, c, film, ys, faded, st) {
      const id = c.ids[m.i], pl = film.players.get(id) || { num: "", side: c.side[m.i], wt: 220, ht: 73 };
      const u = film.uni[pl.side] || film.uni[0];
      g.globalAlpha = faded ? 0.38 : 1;
      const carrier = st.ball.mode === 0 && st.ball.holder === m.i;
      const down = m.act === ACT.down || m.act === ACT.fall;
      if (SET.figures) this.figure(g, m, pl, u, ys);
      else {
        // A dot one and a half yards across, the number inside.
        const r = 0.82;
        g.fillStyle = u.dot;
        g.strokeStyle = u.ring;
        g.lineWidth = Math.max(0.08, 1.4 * ys);
        g.beginPath();
        if (down) g.ellipse(m.x, m.y, r * 1.25, r * 0.7, m.face, 0, Math.PI * 2);
        else g.arc(m.x, m.y, r, 0, Math.PI * 2);
        g.fill();
        g.stroke();
        // A notch shows where he faces.
        g.fillStyle = u.num;
        g.beginPath();
        g.arc(m.x + Math.cos(m.face) * r * 0.78, m.y + Math.sin(m.face) * r * 0.78, 0.17, 0, Math.PI * 2);
        g.fill();
      }
      // The number stays the same size at every zoom (12 px), on its badge
      // when figures are drawn.
      const fs = Math.max(0.62, 12 * ys);
      g.font = `800 ${fs}px Archivo, Arial, sans-serif`;
      g.textAlign = "center";
      g.textBaseline = "middle";
      const [nx, ny] = SET.figures ? [m.x - Math.cos(m.face) * 1.25, m.y - Math.sin(m.face) * 1.25] : [m.x, m.y + fs * 0.06];
      g.save();
      g.translate(nx, ny);
      g.rotate(-this.cam.rot);
      if (SET.figures) {
        g.lineWidth = fs * 0.28;
        g.strokeStyle = "rgba(0,0,0,.72)";
        g.strokeText(String(pl.num), 0, 0);
        g.fillStyle = "#fff";
      } else g.fillStyle = u.num;
      g.fillText(String(pl.num), 0, 0);
      g.restore();
      if (carrier) {
        g.strokeStyle = "rgba(255,214,10,.95)";
        g.lineWidth = Math.max(0.12, 2.2 * ys);
        g.beginPath();
        g.arc(m.x, m.y, 1.45, 0, Math.PI * 2);
        g.stroke();
      }
      if (this.player.isolate === m.i) {
        g.strokeStyle = "rgba(255,255,255,.95)";
        g.setLineDash([0.4, 0.3]);
        g.lineWidth = Math.max(0.1, 1.6 * ys);
        g.beginPath();
        g.arc(m.x, m.y, 2.0, 0, Math.PI * 2);
        g.stroke();
        g.setLineDash([]);
      }
      g.globalAlpha = 1;
    }
    // A top-down figure like a play diagram come to life: shoulder pads and
    // helmet in the team's colours, arms and legs by what he is doing,
    // sized by his body.
    figure(g, m, pl, u, ys) {
      const wt = pl.wt || 220, ht = pl.ht || 73;
      const sh = 0.36 + (wt - 180) / 1100 + (ht - 70) / 260; // half shoulder width, yards
      const depth = 0.22 + (wt - 180) / 1800;
      const act = m.act;
      const sw = Math.sin(m.walked * 2.2);
      const run = Math.min(1, m.speed / 6);
      g.save();
      g.translate(m.x, m.y);
      g.rotate(m.face);
      const k = 1.32; // drawn a third larger so they read at a distance
      g.scale(k, k);
      if (act === ACT.down || act === ACT.fall) {
        const L = act === ACT.fall ? 1.0 : 1.7;
        g.fillStyle = u.pants;
        this.ell(g, -0.15, 0, 0.42, sh * 0.6);
        g.fillStyle = u.jersey;
        this.ell(g, L * 0.45, 0, L * 0.5 + 0.1, sh * 0.85);
        g.fillStyle = u.helmet;
        this.ell(g, L + 0.18, 0, 0.21, 0.21);
        g.restore();
        return;
      }
      // Legs: a stride by speed, the feet set in a stance.
      g.fillStyle = u.pants;
      const stance = act === ACT.stance || act === ACT.kneel;
      const st = stance ? 0.22 : sw * 0.36 * run;
      this.ell(g, st, -sh * 0.38, 0.17, 0.1);
      this.ell(g, -st, sh * 0.38, 0.17, 0.1);
      // Arms by act.
      const skin = pl.skin || "#a86b3c";
      let arms = [[0.14 - sw * 0.22 * run, -sh * 1.05], [0.14 + sw * 0.22 * run, sh * 1.05]];
      if (act === ACT.block || act === ACT.engaged || act === ACT.rush || act === ACT.tackle || act === ACT.wrapped) arms = [[0.62, -sh * 0.55], [0.62, sh * 0.55]];
      else if (act === ACT.catch) arms = [[0.7, -0.14], [0.7, 0.14]];
      else if (act === ACT.carry) arms = [[0.12 - sw * 0.22, -sh * 1.05], [0.28, sh * 0.6]];
      else if (act === ACT.throw) arms = [[0.55, -sh * 0.6], [0.65, sh * 0.95]];
      else if (act === ACT.pocket) arms = [[0.32, -sh * 0.4], [0.32, sh * 0.4]];
      else if (act === ACT.celebrate) arms = [[0.1, -sh * 1.5], [0.1, sh * 1.5]];
      else if (act === ACT.stance) arms = [[0.45, -sh * 0.5], [0.45, sh * 0.5]];
      g.strokeStyle = skin;
      g.lineWidth = 0.15;
      g.lineCap = "round";
      arms.forEach(([ax, ay], j) => {
        g.beginPath();
        g.moveTo(0, (j ? 1 : -1) * sh * 0.8);
        g.lineTo(ax, ay);
        g.stroke();
      });
      // Shoulder pads (the jersey), the helmet with its stripe and mask.
      g.fillStyle = u.jersey;
      this.ell(g, 0, 0, depth, sh);
      g.strokeStyle = "rgba(0,0,0,.45)";
      g.lineWidth = 0.05;
      g.stroke();
      g.fillStyle = u.trim;
      g.fillRect(-0.05, -sh * 0.98, 0.1, sh * 0.22);
      g.fillRect(-0.05, sh * 0.76, 0.1, sh * 0.22);
      g.fillStyle = u.helmet;
      this.ell(g, 0.04, 0, 0.23, 0.23);
      g.strokeStyle = u.stripe;
      g.lineWidth = 0.06;
      g.beginPath();
      g.moveTo(-0.18, 0);
      g.lineTo(0.26, 0);
      g.stroke();
      g.strokeStyle = "#c3c7cc";
      g.lineWidth = 0.05;
      g.beginPath();
      g.arc(0.04, 0, 0.27, -0.85, 0.85);
      g.stroke();
      g.restore();
    }
    ell(g, x, y, rx, ry) {
      g.beginPath();
      g.ellipse(x, y, rx, ry, 0, 0, Math.PI * 2);
      g.fill();
    }
    ball(g, b, ys) {
      // The ball's height shows by its size and its shadow's offset.
      const k = 1 + Math.min(2.2, b.z / 4);
      if (b.z > 0.6) {
        g.fillStyle = "rgba(0,0,0,.32)";
        g.beginPath();
        g.ellipse(b.x + b.z * 0.18, b.y + b.z * 0.14, 0.36, 0.22, 0, 0, Math.PI * 2);
        g.fill();
      }
      g.fillStyle = "#8a4a22";
      g.strokeStyle = "#f2e6d8";
      g.lineWidth = Math.max(0.04, 1 * ys);
      g.beginPath();
      g.ellipse(b.x, b.y, 0.36 * k, 0.22 * k, 0.3, 0, Math.PI * 2);
      g.fill();
      g.stroke();
    }
    official(g, o, ys) {
      if (o.role === "box" || o.role === "rod") {
        g.fillStyle = o.role === "box" ? "#ff9f0a" : "#f2f2f2";
        g.fillRect(o.x - 0.25, o.y - 0.25, 0.5, 0.5);
        g.strokeStyle = "rgba(255,255,255,.5)";
        g.lineWidth = Math.max(0.05, ys);
        if (o.role === "rod") {
          g.beginPath();
          g.moveTo(o.x, o.y + 0.3);
          g.lineTo(o.x, o.y + 1.1);
          g.stroke();
        }
        return;
      }
      // Black and white stripes; the referee's white cap.
      g.save();
      g.translate(o.x, o.y);
      g.fillStyle = "#f4f4f4";
      g.beginPath();
      g.arc(0, 0, 0.55, 0, Math.PI * 2);
      g.fill();
      g.fillStyle = "#111";
      for (let s = -2; s <= 2; s += 2) g.fillRect(s * 0.18 - 0.07, -0.5, 0.14, 1.0);
      g.fillStyle = o.role === "R" ? "#ffffff" : "#111";
      g.beginPath();
      g.arc(0, 0, 0.22, 0, Math.PI * 2);
      g.fill();
      g.restore();
    }
    nameTag(g, m, c, film, ys) {
      const pl = film.players.get(c.ids[m.i]);
      if (!pl) return;
      const text = `#${pl.num} ${pl.name} · ${pl.pos} · ${ACT_WORDS[m.act] || ""}`;
      g.save();
      g.translate(m.x, m.y - 2.1);
      g.rotate(-this.cam.rot);
      const fs = 12 * ys;
      g.font = `700 ${fs}px Archivo, Arial, sans-serif`;
      const w = g.measureText(text).width + 14 * ys;
      g.fillStyle = "rgba(12,13,17,.82)";
      g.beginPath();
      if (g.roundRect) g.roundRect(-w / 2, -fs * 1.05, w, fs * 1.9, 6 * ys);
      else g.rect(-w / 2, -fs * 1.05, w, fs * 1.9);
      g.fill();
      g.fillStyle = "#fff";
      g.textAlign = "center";
      g.textBaseline = "middle";
      g.fillText(text, 0, 0);
      g.restore();
    }
  }

  // ---------------------------------------------------------------- drawings
  // One drawing on the turf, in yards: a line, an arrow, a circle or a
  // freehand stroke, about four pixels wide at any zoom, with a dark edge
  // so it reads on grass and on paint.
  const MARK_COLORS = ["#ffd60a", "#ff375f", "#ffffff", "#0a84ff"];
  function drawMark(g, m, ys, alpha = 1) {
    const pts = m.pts;
    if (!pts || !pts.length) return;
    g.save();
    g.globalAlpha = alpha;
    g.lineCap = "round";
    g.lineJoin = "round";
    const path = () => {
      g.beginPath();
      if (m.kind === 2) {
        const [a, b] = [pts[0], pts[pts.length - 1]];
        g.arc(a[0], a[1], Math.max(0.3, Math.hypot(b[0] - a[0], b[1] - a[1])), 0, Math.PI * 2);
      } else if (m.kind === 3) {
        g.moveTo(pts[0][0], pts[0][1]);
        for (let i = 1; i < pts.length; i++) {
          const [x0, y0] = pts[i - 1], [x1, y1] = pts[i];
          g.quadraticCurveTo(x0, y0, (x0 + x1) / 2, (y0 + y1) / 2);
        }
        const l = pts[pts.length - 1];
        g.lineTo(l[0], l[1]);
      } else {
        g.moveTo(pts[0][0], pts[0][1]);
        g.lineTo(pts[pts.length - 1][0], pts[pts.length - 1][1]);
      }
    };
    const w = Math.max(0.12, 4.2 * ys);
    g.strokeStyle = "rgba(0,0,0,.45)";
    g.lineWidth = w + 3 * ys;
    path();
    g.stroke();
    g.strokeStyle = MARK_COLORS[m.color] || MARK_COLORS[0];
    g.lineWidth = w;
    path();
    g.stroke();
    if (m.kind === 1) {
      // The arrowhead along the last stretch.
      const a = pts[0], b = pts[pts.length - 1];
      const ang = Math.atan2(b[1] - a[1], b[0] - a[0]);
      const L = Math.max(0.9, 16 * ys);
      g.fillStyle = MARK_COLORS[m.color] || MARK_COLORS[0];
      g.beginPath();
      g.moveTo(b[0] + Math.cos(ang) * L * 0.35, b[1] + Math.sin(ang) * L * 0.35);
      g.lineTo(b[0] + Math.cos(ang + 2.55) * L, b[1] + Math.sin(ang + 2.55) * L);
      g.lineTo(b[0] + Math.cos(ang - 2.55) * L, b[1] + Math.sin(ang - 2.55) * L);
      g.closePath();
      g.fill();
    }
    g.restore();
  }

  // ---------------------------------------------------------------- exports
  window.AFMViews = Object.assign(window.AFMViews || {}, {
    SET, saveSettings, Film, decode, sample, at, angLerp, paintField, View2D, uniforms, ACT, ACT_WORDS, drawMark, MARK_COLORS,
    FIELD_W, FIELD_H, PX, MARGIN_X, MARGIN_Y, contrast, mixHex, lum, hex,
  });
})();
