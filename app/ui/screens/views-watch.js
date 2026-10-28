// Lane screens: views, the film room's player (later extras topics 2 and 3,
// screens topic 12, game day "watching"). One player for every way in: the
// Watch tab on a game page, full-screen watching (F), and the snaps of the
// game you are playing live, in the live screen's own 2D panel.
//
// The player keeps time, the play list, cut-ups, the grade sheet, drawings
// and the cameras; the views (views.js for 2D, views/views3d.js for 3D)
// only draw the moment they are handed.
"use strict";

(() => {
  const V = window.AFMViews;
  const { SET, saveSettings } = V;

  // ---------------------------------------------------------------- sources
  // Where a film's snaps come from: one of your kept games, or the game you
  // are playing live. Snaps asked for in the same moment go as one call.
  function source(kind, index) {
    const queue = new Map();
    let timer = 0;
    const flush = async () => {
      timer = 0;
      const want = [...queue.entries()];
      queue.clear();
      const ns = want.map((w) => w[0]);
      try {
        const list = kind === "live" ? await api("film_live_plays", { n: ns }) : await api("film_plays", { index, n: ns });
        const by = new Map(list.map((c) => [c.n, c]));
        for (const [n, cb] of want) {
          const c = by.get(n);
          if (c) cb.res(c);
          else cb.rej(new Error("That snap has no film."));
        }
      } catch (e) {
        for (const [, cb] of want) cb.rej(e);
      }
    };
    return {
      kind,
      index,
      open: () => (kind === "live" ? api("film_live") : api("film", { index })),
      grades: () => (kind === "live" ? api("film_live_grades") : api("film_grades", { index })),
      clip: (n) =>
        new Promise((res, rej) => {
          queue.set(n, { res, rej });
          if (!timer) timer = setTimeout(flush, 0);
        }),
      saveMarks: (marks) => (kind === "live" ? Promise.resolve() : api("film_marks", { index, marks })),
    };
  }

  // ---------------------------------------------------------------- words
  const ORD = ["", "1st", "2nd", "3rd", "4th"];
  const KIND_WORDS = { Kickoff: "Kickoff", Punt: "Punt", FieldGoal: "Field goal", ExtraPoint: "Extra point", TwoPoint: "Two-point try", Kneel: "Kneel", Spike: "Spike", NoPlay: "No play", Sack: "Sack", Scramble: "Scramble", Run: "Run", Pass: "Pass" };
  const qWord = (q) => (q > 4 ? (q === 5 ? "OT" : `${q - 4}OT`) : `Q${q}`);
  const downText = (p) => (p.down > 0 ? `${ORD[p.down] || p.down + "th"} & ${p.togo}` : KIND_WORDS[p.kind] || p.kind);
  const signed = (v, d = 1) => (v > 0 ? "+" : v < 0 ? "−" : "") + Math.abs(v).toFixed(d);
  const COLORS = ["#ffd60a", "#ff375f", "#ffffff", "#0a84ff"];
  const COLOR_NAMES = ["Yellow", "Red", "White", "Blue"];
  const TOOLS = [
    ["line", "Line", '<path d="M5 19L19 5"/>'],
    ["arrow", "Arrow", '<path d="M5 19L19 5M10 5h9v9"/>'],
    ["circle", "Circle", '<circle cx="12" cy="12" r="7"/>'],
    ["free", "Freehand", '<path d="M4 16c3-6 5 2 8-3s5-6 8-2"/>'],
  ];
  const TOOL_KIND = { line: 0, arrow: 1, circle: 2, free: 3 };
  const KIND_TOOL = ["line", "arrow", "circle", "free"];
  const ico = (p) => `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">${p}</svg>`;
  const IC = {
    play: '<path d="M7 4.5v15l12.5-7.5z" fill="currentColor" stroke="none"/>',
    pause: '<rect x="6" y="4.5" width="4" height="15" rx="1" fill="currentColor" stroke="none"/><rect x="14" y="4.5" width="4" height="15" rx="1" fill="currentColor" stroke="none"/>',
    prev: '<path d="M6 5v14M19 5l-9 7 9 7z"/>',
    next: '<path d="M18 5v14M5 5l9 7-9 7z"/>',
    back: '<path d="M11 7l-5 5 5 5M18 7l-5 5 5 5"/>',
    fwd: '<path d="M13 7l5 5-5 5M6 7l5 5-5 5"/>',
    full: '<path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5"/>',
    shrink: '<path d="M9 4v5H4M15 4v5h5M9 20v-5H4M15 20v-5h5"/>',
    close: '<path d="M6 6l12 12M18 6L6 18"/>',
    undo: '<path d="M9 14L4 9l5-5"/><path d="M4 9h10a6 6 0 010 12h-3"/>',
    restart: '<path d="M4 12a8 8 0 108-8H8"/><path d="M8 1v6h6"/>',
    eye: '<path d="M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12z"/><circle cx="12" cy="12" r="3"/>',
    side: '<rect x="3.5" y="5" width="17" height="14" rx="2"/><path d="M14.5 5v14"/>',
  };

  // ---------------------------------------------------------------- the player
  let ACTIVE = null;
  const players = new Set();

  class Player {
    constructor(root, film, opts = {}) {
      this.root = root;
      this.film = film;
      this.opts = opts;
      this.inline = !!opts.inline; // the live screen's small panel
      this.full = !!opts.full;
      this.list = [];
      this.pos = 0;
      this.clip = null;
      this.t = 0;
      this.playing = false;
      this.st = null;
      this.isolateId = null;
      this.isolate = null;
      this.picked = null;
      this.tool = null;
      this.color = 0;
      this.marks = (film.ix.marks || []).map((m) => ({ ...m }));
      this.drawing = null;
      this.inter = 0;
      this.panel = this.inline ? null : (opts.panel || "plays");
      this.listKind = opts.listKind || (SET.pace === "key" ? "key" : "all");
      this.cut = {};
      this.sheet = null;
      this.last = performance.now();
      this.loadTok = 0;
      film.fieldCanvas = film.fieldCanvas || V.paintField(film);
      this.build();
      this.view = null;
      this.useView(SET.three && SET.view === "3d" ? "3d" : "2d");
      players.add(this);
      ACTIVE = this;
      this.loop = this.loop.bind(this);
      this.raf = requestAnimationFrame(this.loop);
      const ix = film.ix;
      if (opts.list) this.setList(opts.list, opts.listName || "Cut-up");
      else this.setList(this.listFor(this.listKind), null);
      const start = opts.start != null ? this.list.indexOf(Number(opts.start)) : -1;
      this.goTo(start >= 0 ? start : 0, { play: opts.autoplay !== false && !opts.paused, card: true });
      if (ix.plays.length === 0) this.$(".vw-cap").innerHTML = '<div class="vw-cap-text">No snaps yet.</div>';
    }

    $(s) {
      return this.root.querySelector(s);
    }

    // ---------- the frame around the film
    build() {
      const ix = this.film.ix;
      const [h, a] = ix.teams;
      const u = this.film.uni;
      const teamChip = (t, side) => `<span class="vw-bug-team" style="--c:${u[side].c1}"><i></i><b>${esc(t.short || t.name)}</b><span class="vw-bug-pts" data-side="${side}">0</span></span>`;
      const side = this.inline
        ? ""
        : `<aside class="vw-side">
            <div class="vw-side-tabs" role="tablist">${[["plays", "Plays"], ["cuts", "Cut-ups"], ["grades", "Grades"], ["draw", "Draw"], ["view", "View"]]
              .map(([k, t]) => `<button role="tab" data-vw="panel" data-k="${k}" aria-selected="${this.panel === k}">${t}</button>`)
              .join("")}</div>
            <div class="vw-side-body"></div>
          </aside>`;
      this.root.innerHTML = `<div class="vw ${this.inline ? "vw-inline" : ""} ${this.full ? "vw-full overlay" : ""}" tabindex="-1">
        <div class="vw-main">
          <div class="vw-stage">
            <div class="vw-host"></div>
            <div class="vw-bug">${teamChip(a, 1)}<span class="vw-bug-at">at</span>${teamChip(h, 0)}<span class="vw-bug-clock"></span><span class="vw-bug-dd"></span></div>
            <div class="vw-ng"></div>
            <div class="vw-card" hidden></div>
            <div class="vw-man" hidden></div>
            <div class="vw-hint" hidden></div>
            <div class="vw-cap"></div>
            <div class="vw-wait" hidden><span></span>Loading the snap…</div>
          </div>
          <div class="vw-bar">
            <div class="vw-btns">
              <button class="vw-ib" data-vw="prev" title="Previous snap (Shift ←)" aria-label="Previous snap">${ico(IC.prev)}</button>
              <button class="vw-ib" data-vw="back" title="Back a tenth of a second (←)" aria-label="Back a tenth of a second">${ico(IC.back)}</button>
              <button class="vw-ib big" data-vw="play" title="Play or pause (Space)" aria-label="Play">${ico(IC.play)}</button>
              <button class="vw-ib" data-vw="fwd" title="Forward a tenth of a second (→)" aria-label="Forward a tenth of a second">${ico(IC.fwd)}</button>
              <button class="vw-ib" data-vw="next" title="Next snap (Shift →)" aria-label="Next snap">${ico(IC.next)}</button>
            </div>
            <div class="vw-scrub" role="slider" aria-label="Time in the snap" tabindex="0"><div class="vw-track"><div class="vw-fill"></div></div><div class="vw-mks"></div><div class="vw-knob"></div></div>
            <span class="vw-time st-tnum">0.0 s</span>
            <div class="vw-speed" role="group" aria-label="Speed">${[0.25, 0.5, 1, 2].map((s) => `<button data-vw="speed" data-s="${s}" aria-pressed="${SET.speed === s}">${s === 0.25 ? "¼" : s === 0.5 ? "½" : s}×</button>`).join("")}</div>
            <div class="vw-switch" role="group" aria-label="View"></div>
            ${this.inline ? "" : `<button class="vw-ib" data-vw="side" title="Show or hide the panel (P)" aria-label="Show or hide the panel" aria-pressed="${SET.side !== false}">${ico(IC.side)}</button>`}
            <button class="vw-ib" data-vw="full" title="${this.full ? "Leave full screen (Esc)" : "Full screen (F)"}" aria-label="Full screen">${ico(this.full ? IC.shrink : IC.full)}</button>
          </div>
        </div>
        ${side}
      </div>`;
      this.el = this.$(".vw");
      this.el.classList.toggle("noside", SET.side === false && !this.inline);
      this.el.addEventListener("click", (e) => this.onClick(e));
      this.el.addEventListener("input", (e) => this.onInput(e));
      this.el.addEventListener("change", (e) => this.onInput(e));
      this.el.addEventListener("pointerdown", () => (ACTIVE = this));
      this.wireScrub();
      this.syncBar();
      this.drawPanel();
      // The panel beside the film is as tall as the film and its bar.
      const main = this.$(".vw-main"), sideEl = this.$(".vw-side");
      this.ro = new ResizeObserver(() => {
        if (sideEl && !this.full && innerWidth > 1100) sideEl.style.height = main.offsetHeight + "px";
        else if (sideEl) sideEl.style.height = "";
      });
      this.ro.observe(main);
    }

    // ---------- views
    async useView(kind) {
      const host = this.$(".vw-host");
      if (kind === "3d") {
        try {
          await loadThree();
        } catch (e) {
          toast("The 3D view could not start here: " + e.message);
          kind = "2d";
        }
      }
      if (this.dead) return;
      if (this.view) this.view.dispose();
      this.view = kind === "3d" ? new window.AFMViews3D.View3D(host, this) : new V.View2D(host, this);
      this.kind = kind;
      if (kind === "3d" && !this.view.ok) {
        this.view.dispose();
        this.view = new V.View2D(host, this);
        this.kind = "2d";
        toast("This computer can't draw the 3D view; the 2D view is on.");
      }
      this.view.snapCam = true;
      if (!this.inline) {
        SET.view = this.kind;
        saveSettings();
      }
      this.syncBar();
    }
    toggleView() {
      if (!SET.three) {
        toast("Turn on the 3D view under View first.");
        return;
      }
      this.useView(this.kind === "3d" ? "2d" : "3d");
    }

    // ---------- the list
    listFor(kind) {
      const ix = this.film.ix, mine = ix.mine;
      const ok = {
        all: () => true,
        key: (p) => p.key,
        offense: (p) => p.tags.unit === "scrimmage" && p.off === (mine ?? 0),
        defense: (p) => p.tags.unit === "scrimmage" && p.off !== (mine ?? 0),
        special: (p) => p.tags.unit === "special teams",
      }[kind] || (() => true);
      return ix.plays.filter(ok).map((p) => p.n);
    }
    setList(ns, name) {
      this.list = ns.slice();
      this.listName = name;
      if (this.panel === "plays" || this.panel === "cuts") this.drawPanel();
    }

    // ---------- moving between snaps
    async goTo(i, o = {}) {
      if (!this.list.length) return;
      i = Math.max(0, Math.min(this.list.length - 1, i));
      this.pos = i;
      const n = this.list[i];
      const tok = ++this.loadTok;
      const wait = this.$(".vw-wait");
      const slow = setTimeout(() => (wait.hidden = false), 180);
      let c;
      try {
        c = await this.film.load(n);
      } catch (e) {
        clearTimeout(slow);
        wait.hidden = true;
        toast(e.message);
        return;
      }
      clearTimeout(slow);
      wait.hidden = true;
      if (tok !== this.loadTok || this.dead) return;
      this.clip = c;
      this.st = null;
      this.t = o.t != null ? Math.max(c.t0, Math.min(c.t1, o.t)) : c.t0;
      this.isolate = this.isolateId != null ? (c.ids.indexOf(this.isolateId) >= 0 ? c.ids.indexOf(this.isolateId) : null) : null;
      if (this.picked != null) {
        const j = c.ids.indexOf(this.pickedId);
        this.picked = j >= 0 ? j : null;
      }
      if (this.view) this.view.snapCam = true;
      this.playing = !!o.play;
      this.replay = this.replayed = false;
      this.inter = o.card && this.playing && !this.inline ? 1.15 : o.card && this.inline && this.playing ? 0.7 : 0;
      this.showCard(this.inter > 0);
      this.caption();
      this.markers();
      this.syncBar();
      this.highlightRow();
      if (this.panel === "grades" || this.panel === "draw") this.drawPanel();
      this.manCard();
      // The next snaps load while this one plays.
      this.film.prefetch(this.list.slice(i + 1, i + 4));
      if (this.onSnap) this.onSnap(n);
    }
    next(auto) {
      if (this.pos < this.list.length - 1) return this.goTo(this.pos + 1, { play: auto ? true : this.playing || SET.auto, card: true });
      this.playing = false;
      this.syncBar();
      if (this.onEnd) this.onEnd();
    }
    prev() {
      // Back to this snap's start first, as a remote does.
      if (this.clip && this.t > this.clip.t0 + 1.2) return this.seek(this.clip.t0);
      if (this.pos > 0) this.goTo(this.pos - 1, { play: this.playing, card: false });
    }
    seek(t) {
      if (!this.clip) return;
      this.t = Math.max(this.clip.t0, Math.min(this.clip.t1, t));
      this.inter = 0;
      this.showCard(false);
      this.syncTime();
    }
    step(dt) {
      this.playing = false;
      this.seek(this.t + dt);
      this.syncBar();
    }
    toggle() {
      if (!this.clip) return;
      if (this.clip && this.t >= this.clip.t1 - 0.01) this.t = this.clip.t0;
      this.playing = !this.playing;
      if (this.playing) this.tool = null;
      this.syncBar();
    }

    // ---------- the clock
    loop(now) {
      if (this.dead) return;
      this.raf = requestAnimationFrame(this.loop);
      const dt = Math.min(0.1, (now - this.last) / 1000);
      this.last = now;
      if (!this.root.isConnected) return this.dispose();
      if (this.manual || document.hidden || !this.clip || !this.view) return;
      const r = this.el.getBoundingClientRect();
      if (r.bottom < 0 || r.top > innerHeight || r.width === 0) return;
      this.tick(dt, now);
    }
    // One frame by hand, for recording a clip at an even frame rate.
    frameStep(dt) {
      this.manual = true;
      if (this.clip && this.view) this.tick(dt, performance.now());
    }
    tick(dt, now) {
      if (this.inter > 0) {
        this.inter -= dt * Math.max(0.5, SET.speed);
        if (this.inter <= 0) this.showCard(false);
      } else if (this.playing) {
        this.t += dt * SET.speed * (this.replay ? 0.55 : 1);
        if (this.t >= this.clip.t1) {
          this.t = this.clip.t1;
          // A big play again in the 3D view, slower, from the end zone.
          const p = this.play();
          if (this.kind === "3d" && SET.replay && !this.replayed && !this.inline && p && p.key && p.tags.unit === "scrimmage") {
            this.replayed = this.replay = true;
            this.t = Math.max(this.clip.t0, (this.clip.throwAt != null ? this.clip.throwAt : 0) - 1.4);
            this.view.snapCam = true;
            this.hint();
            return;
          }
          if (this.replay) {
            this.replay = false;
            this.hint();
            if (this.view) this.view.snapCam = true;
          }
          this.playing = false;
          this.syncBar();
          if (SET.auto || this.inline || this.opts.autoNext) {
            this.next(true);
          } else if (this.onEnd && this.pos >= this.list.length - 1) this.onEnd();
        }
      }
      this.st = V.sample(this.clip, this.t, this.st && this.st.men.length === this.clip.men ? this.st : null);
      this.view.draw(this.st, this.clip, this.film, dt);
      this.syncTime();
      this.nextGen();
      if (this.picked != null && (this._manT || 0) + 0.25 < now / 1000) {
        this._manT = now / 1000;
        this.manCard(true);
      }
    }

    // ---------- the score bug, the caption, the card between snaps
    play() {
      return this.clip ? this.film.play(this.clip.n) : null;
    }
    caption() {
      const p = this.play();
      if (!p) return;
      const ix = this.film.ix;
      const off = ix.teams[p.off];
      const bug = this.$(".vw-bug");
      bug.querySelector('[data-side="0"]').textContent = p.score[0];
      bug.querySelector('[data-side="1"]').textContent = p.score[1];
      bug.querySelectorAll(".vw-bug-team").forEach((el, k) => el.classList.toggle("ball", (k === 0 ? 1 : 0) === p.off && p.tags.unit === "scrimmage"));
      bug.querySelector(".vw-bug-clock").textContent = `${qWord(p.q)} ${p.clock}`;
      bug.querySelector(".vw-bug-dd").textContent = p.down > 0 ? `${downText(p)} · ${p.spot}` : KIND_WORDS[p.kind] || p.kind;
      const calls = [p.call && `<b>${esc(off.short || off.name)}:</b> ${esc(p.call)}`, p.defcall && `<b>${esc(ix.teams[1 - p.off].short || ix.teams[1 - p.off].name)}:</b> ${esc(p.defcall)}`].filter(Boolean).join(" · ");
      const epa = p.kind === "Kickoff" || p.kind === "ExtraPoint" ? "" : `<span class="vw-epa ${p.epa > 0 ? "up" : p.epa < 0 ? "down" : ""}" title="Expected points added for ${esc(off.name)}">${signed(p.epa, 2)} EPA</span>`;
      const pos = this.list.length > 1 ? `<span class="vw-of">${this.pos + 1} of ${this.list.length}${this.listName ? " · " + esc(this.listName) : ""}</span>` : "";
      this.$(".vw-cap").innerHTML = `<div class="vw-cap-text">${esc(p.text)}</div><div class="vw-cap-sub">${calls}${p.why ? ` <em>${esc(p.why)}</em>` : ""} ${epa} ${pos}</div>`;
    }
    showCard(on) {
      const el = this.$(".vw-card");
      const p = this.play();
      if (!on || !p) {
        el.hidden = true;
        return;
      }
      const ix = this.film.ix;
      const off = ix.teams[p.off];
      const u = this.film.uni[p.off];
      const big = p.down > 0 ? downText(p) : KIND_WORDS[p.kind] || p.kind;
      const sub = [p.down > 0 ? p.spot : "", p.tags.personnel ? p.tags.personnel + " personnel" : "", p.tags.formation].filter(Boolean).join(" · ");
      el.innerHTML = `<div class="vw-card-in" style="--c:${u.c1}"><span class="st-label">${esc(off.name)} · ${qWord(p.q)} ${esc(p.clock)}</span><b>${esc(big)}</b><small>${esc(sub)}</small></div>`;
      el.hidden = false;
    }
    // The tracking numbers as the broadcast shows them, each once it means
    // something: time to throw at the release, air yards and separation at
    // the catch, the carrier's speed while he runs.
    nextGen() {
      const el = this.$(".vw-ng");
      if (!SET.tiles || !this.clip) {
        if (el.firstChild) el.replaceChildren();
        return;
      }
      const c = this.clip, ng = c.ng || {}, t = this.t, st = this.st;
      // Live numbers (separation, the carrier's speed) change four times a
      // second of film, as a broadcast's do, so they read rather than blur.
      const beat = Math.floor(t * 4);
      const held = (k, v) => {
        const h = (this._ngHeld = this._ngHeld || {});
        const o = h[k];
        if (!o || o.beat !== beat || o.n !== c.n) h[k] = { beat, n: c.n, v };
        return h[k].v;
      };
      const tiles = [];
      if (c.throwAt != null && t >= c.throwAt && ng.ttt) tiles.push(["Time to throw", ng.ttt.toFixed(1), "s"]);
      if (c.catchAt != null && t >= c.catchAt) {
        if (ng.air != null) tiles.push(["Air yards", Math.round(ng.air), "yd"]);
        if (ng.sep != null) tiles.push(["Separation", ng.sep.toFixed(1), "yd"]);
      } else if (c.throwAt != null && t >= c.throwAt && st && st.sepNow != null && st.sepNow < 30) tiles.push(["Separation", held("sep", st.sepNow.toFixed(1)), "yd"]);
      // The carrier's speed once he runs with it (a quarterback dropping
      // back isn't carrying it yet); it stays up until he gives it up.
      if (t > 0.2 && st && st.ball.mode === 0 && st.ball.holder >= 0) {
        const m = st.men[st.ball.holder];
        const id = c.ids[st.ball.holder];
        const dropback = id === c.passer && c.los != null && (m.x - c.los) * (c.dir || 1) < 0;
        // On a pass the tiles belong to the throw until the catch is made.
        const inAir = c.throwAt != null && t >= c.throwAt && (c.catchAt == null || t < c.catchAt);
        const run = this._ngRun && this._ngRun.n === c.n && this._ngRun.id === id;
        if (m && !dropback && !inAir && (run || m.speed > 2)) {
          this._ngRun = { n: c.n, id };
          tiles.push(["Ball carrier", held("mph", (m.speed * 2.045).toFixed(1)), "mph"]);
        }
      }
      if (t < 0.1 && ng.box) tiles.push(["In the box", ng.box, ""]);
      if (t >= c.t1 - 1.2 && ng.top && ng.top[1] > 12) {
        const pl = this.film.players.get(ng.top[0]);
        tiles.push(["Top speed", ng.top[1].toFixed(1), "mph" + (pl ? ` · #${pl.num}` : "")]);
      }
      const chips = [];
      if (t >= 0 && ng.blitz) chips.push(["Blitz", true]);
      if (t >= 0 && ng.pa) chips.push(["Play action", false]);
      if (t >= 0 && ng.pressured && c.throwAt != null && t >= c.throwAt - 0.4) chips.push(["Pressured", true]);
      this.tilesTo(el, tiles.slice(0, 3), chips);
    }
    // The tiles stay put while their numbers change: a tile is built (and
    // fades in) only when it first appears, and only its number is
    // rewritten after that.
    tilesTo(el, tiles, chips) {
      const keep = (parent, sel, want, make, fill, before = null) => {
        const have = new Map([...parent.querySelectorAll(":scope > " + sel)].map((d) => [d.dataset.k, d]));
        const keys = new Set(want.map((w) => w[0]));
        for (const [k, d] of have) if (!keys.has(k)) d.remove();
        let at = parent.querySelector(":scope > " + sel);
        for (const w of want) {
          let d = have.get(w[0]);
          if (!d || !d.isConnected) {
            d = make(w);
            d.dataset.k = w[0];
            parent.insertBefore(d, at || before);
          } else if (d !== at) parent.insertBefore(d, at || before);
          fill(d, w);
          at = d.nextElementSibling;
          while (at && !at.matches(sel)) at = at.nextElementSibling;
        }
      };
      keep(
        el,
        ".vw-tile",
        tiles,
        ([k]) => {
          const d = document.createElement("div");
          d.className = "vw-tile";
          d.innerHTML = `<span>${esc(k)}</span><b class="st-tnum"></b>`;
          return d;
        },
        (d, [, v, u]) => {
          const val = `${v}\u0001${u}`;
          if (d.dataset.v === val) return;
          d.dataset.v = val;
          d.querySelector("b").innerHTML = `${esc(String(v))}<small>${esc(u)}</small>`;
        },
        el.querySelector(":scope > .vw-chips"),
      );
      let row = el.querySelector(":scope > .vw-chips");
      if (!chips.length) {
        if (row) row.remove();
        return;
      }
      if (!row) {
        row = document.createElement("div");
        row.className = "vw-chips";
      }
      if (el.lastElementChild !== row) el.appendChild(row);
      keep(
        row,
        ".vw-chip",
        chips,
        ([k, live]) => {
          const d = document.createElement("span");
          d.className = "vw-chip" + (live ? " live" : "");
          d.textContent = k;
          return d;
        },
        () => {},
      );
    }

    // ---------- the bar
    syncBar() {
      const b = this.$('[data-vw="play"]');
      if (b) {
        b.innerHTML = ico(this.playing || this.inter > 0 ? IC.pause : IC.play);
        b.setAttribute("aria-label", this.playing ? "Pause" : "Play");
      }
      this.root.querySelectorAll('[data-vw="speed"]').forEach((x) => x.setAttribute("aria-pressed", String(Number(x.dataset.s) === SET.speed)));
      const sw = this.$(".vw-switch");
      if (sw) {
        const cams = this.kind === "3d"
          ? [["coaches", "End zone", "The coaches' end-zone copy, high behind the offense"], ["all22", "All-22", "High from the sideline, all 22 men"], ["broadcast", "Broadcast", "The TV angle, low and close"], ["free", "Free", "Drag to orbit, scroll to zoom"]]
          : [["all22", "All-22", "From above, all 22 men in view"], ["tight", "Tight", "Close on the ball"], ["endzone", "End zone", "Behind the offense, the play going up the screen"], ["free", "Free", "Drag to move, scroll to zoom"]];
        const cur = this.kind === "3d" ? SET.cam3d : SET.cam2d;
        sw.innerHTML =
          (SET.three ? `<button data-vw="view" class="vw-vk" title="2D or 3D (V)" aria-pressed="${this.kind === "3d"}">${this.kind === "3d" ? "3D" : "2D"}</button>` : "") +
          cams.map(([k, t, tip], i) => `<button data-vw="cam" data-k="${k}" title="${tip} (${i + 1})" aria-pressed="${cur === k && this.isolate == null}">${t}</button>`).join("");
      }
      const draw = this.$('.vw-side [data-k="draw"]');
      if (draw) draw.classList.toggle("on", !!this.tool);
      this.el.classList.toggle("drawing", !!this.tool);
      this.hint();
    }
    syncTime() {
      const c = this.clip;
      if (!c) return;
      const u = (this.t - c.t0) / Math.max(0.01, c.t1 - c.t0);
      const pct = Math.max(0, Math.min(1, u)) * 100;
      if (Math.abs(pct - (this._pct ?? -1)) < 0.05) return;
      this._pct = pct;
      this.$(".vw-fill").style.width = pct + "%";
      this.$(".vw-knob").style.left = pct + "%";
      const tt = this.t;
      this.$(".vw-time").textContent = tt < 0 ? "Pre-snap" : `${tt.toFixed(1)} s`;
    }
    markers() {
      const c = this.clip;
      const box = this.$(".vw-mks");
      if (!c || !box) return;
      const span = Math.max(0.01, c.t1 - c.t0);
      const at = (t) => ((t - c.t0) / span) * 100;
      const mk = [];
      if (c.t0 < 0) mk.push([0, "snap", "The snap"]);
      if (c.throwAt != null) mk.push([c.throwAt, "throw", "The throw"]);
      if (c.catchAt != null) mk.push([c.catchAt, "catch", "The ball arrives"]);
      for (const f of c.flags || []) mk.push([f.t, "flag", "Flag: " + f.what]);
      for (const m of this.marks) if (m.play === c.n) mk.push([m.t, "mark", "Your drawing"]);
      box.innerHTML = mk.filter((m) => m[0] >= c.t0 && m[0] <= c.t1).map(([t, k, tip]) => `<i class="vw-mk ${k}" style="left:${at(t)}%" title="${esc(tip)}"></i>`).join("");
    }
    wireScrub() {
      const s = this.$(".vw-scrub");
      let was = false;
      const to = (e) => {
        const r = s.getBoundingClientRect();
        const u = Math.max(0, Math.min(1, (e.clientX - r.left) / r.width));
        if (this.clip) this.seek(this.clip.t0 + u * (this.clip.t1 - this.clip.t0));
      };
      s.addEventListener("pointerdown", (e) => {
        was = this.playing;
        this.playing = false;
        s.setPointerCapture(e.pointerId);
        s.classList.add("drag");
        to(e);
      });
      s.addEventListener("pointermove", (e) => s.classList.contains("drag") && to(e));
      s.addEventListener("pointerup", () => {
        s.classList.remove("drag");
        this.playing = was && this.t < (this.clip ? this.clip.t1 : 0);
        this.syncBar();
      });
      s.addEventListener("keydown", (e) => {
        if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
          e.preventDefault();
          e.stopPropagation();
          this.step(e.key === "ArrowLeft" ? -0.1 : 0.1);
        }
      });
    }
    hint() {
      const el = this.$(".vw-hint");
      if (!el) return;
      let h = "";
      if (this.replay) h = '<span class="vw-replay">Replay</span> from the end zone';
      else if (this.tool) h = `Drawing: ${TOOLS.find((t) => t[0] === this.tool)[1].toLowerCase()} in ${COLOR_NAMES[this.color].toLowerCase()} · drag on the field · <kbd>Esc</kbd> to stop`;
      else if (this.isolateId != null) {
        const pl = this.film.players.get(this.isolateId);
        h = pl ? `Isolated on #${pl.num} ${esc(pl.name)}${this.isolate == null ? " (not on the field this snap)" : ""} · <kbd>Esc</kbd> to clear` : "";
      }
      el.innerHTML = h;
      el.hidden = !h;
    }

    // ---------- picking and isolating a man
    pick(i) {
      const c = this.clip;
      if (!c) return;
      this.picked = i;
      this.pickedId = c.ids[i];
      this.manCard();
    }
    isolateOn(id) {
      if (id == null || this.isolateId === id) {
        this.isolateId = null;
        this.isolate = null;
      } else {
        this.isolateId = id;
        const j = this.clip ? this.clip.ids.indexOf(id) : -1;
        this.isolate = j >= 0 ? j : null;
      }
      if (this.view) this.view.snapCam = false;
      this.syncBar();
      this.manCard();
    }
    manCard(quiet) {
      const el = this.$(".vw-man");
      if (!el) return;
      const c = this.clip;
      if (this.picked == null || !c || this.inline) {
        el.hidden = true;
        return;
      }
      const id = c.ids[this.picked];
      const pl = this.film.players.get(id);
      if (!pl) return (el.hidden = true);
      const m = this.st && this.st.men[this.picked];
      const act = m ? V.ACT_WORDS[m.act] || "" : "";
      const grade = (c.grades || {})[id];
      const ft = Math.floor(pl.ht / 12), inch = Math.round(pl.ht - ft * 12);
      const html = `<span class="vw-man-num st-outline">${pl.num}</span>
        <div class="vw-man-who"><a class="link" data-go="player" data-id="${id}">${esc(pl.name)}</a><small>${esc(pl.pos)} · ${ft}-${inch}, ${Math.round(pl.wt)} lb · ${esc(this.film.ix.teams[pl.side].name)}</small>
        <small class="vw-man-act">${esc(act)}${m && m.speed > 1.5 ? ` · ${(m.speed * 2.045).toFixed(1)} mph` : ""}</small>
        ${grade ? `<div class="vw-man-grade"><span class="vw-mark m${grade.mark === "+" ? "p" : grade.mark === "-" ? "m" : "z"}">${grade.mark}</span>${grade.notes.map((n) => esc(n)).join(" · ")}</div>` : ""}</div>
        <div class="vw-man-btns"><button class="st-btn small glass" data-vw="iso" data-id="${id}">${this.isolateId === id ? "Clear isolate" : "Isolate"} <kbd>I</kbd></button><button class="vw-ib small" data-vw="unpick" aria-label="Close">${ico(IC.close)}</button></div>`;
      if (quiet && html === this._man) return;
      this._man = html;
      el.innerHTML = html;
      el.hidden = false;
    }

    // ---------- drawing on the film (the telestrator)
    drawStart(pt, e) {
      if (!this.clip) return;
      this.playing = false;
      this.syncBar();
      const kind = TOOL_KIND[this.tool];
      this.drawing = { play: this.clip.n, t: Math.round(this.t * 100) / 100, kind, color: this.color, pts: [pt, pt] };
      if (e && e.target && e.target.setPointerCapture) e.target.setPointerCapture(e.pointerId);
    }
    drawMove(pt) {
      const d = this.drawing;
      if (!d) return;
      if (d.kind === 3) {
        const l = d.pts[d.pts.length - 1];
        if (Math.hypot(pt[0] - l[0], pt[1] - l[1]) > 0.25 && d.pts.length < 400) d.pts.push(pt);
      } else d.pts[1] = pt;
    }
    drawEnd() {
      const d = this.drawing;
      this.drawing = null;
      if (!d) return;
      const [a, b] = [d.pts[0], d.pts[d.pts.length - 1]];
      if (d.kind !== 3 && Math.hypot(b[0] - a[0], b[1] - a[1]) < 0.6) return;
      d.pts = d.pts.map(([x, y]) => [Math.round(x * 100) / 100, Math.round(y * 100) / 100]);
      this.marks.push(d);
      this.saveMarks();
      this.markers();
      if (this.panel === "draw") this.drawPanel();
    }
    saveMarks() {
      clearTimeout(this._saveT);
      this._saveT = setTimeout(() => {
        this.film.source.saveMarks(this.marks).catch((e) => toast(e.message));
      }, 350);
    }
    // Drawings show on their snap from the moment they were drawn, and stay
    // up while the film is paused; playing on, they fade out.
    drawMarks(g, ys, t) {
      if (!SET.marks || !this.clip) return;
      const list = this.marks.filter((m) => m.play === this.clip.n);
      if (this.drawing) list.push(this.drawing);
      for (const m of list) {
        let a = 1;
        if (m !== this.drawing) {
          if (t < m.t - 0.05) continue;
          if (this.playing) a = Math.max(0, 1 - (t - m.t - 1.2) / 0.5);
          if (a <= 0) continue;
        }
        V.drawMark(g, m, ys, a);
      }
    }

    // ---------- the side panel
    drawPanel() {
      const body = this.$(".vw-side-body");
      if (!body) return;
      this.root.querySelectorAll(".vw-side-tabs button").forEach((b) => b.setAttribute("aria-selected", String(b.dataset.k === this.panel)));
      const f = { plays: () => this.panelPlays(), cuts: () => this.panelCuts(), grades: () => this.panelGrades(), draw: () => this.panelDraw(), view: () => this.panelView() }[this.panel];
      const keep = this.panel === this._panelWas ? body.scrollTop : 0;
      body.innerHTML = f ? f() : "";
      body.scrollTop = keep;
      this._panelWas = this.panel;
      if (this.panel === "plays") this.highlightRow(true);
    }
    rowHtml(p) {
      const ix = this.film.ix;
      const u = this.film.uni[p.off];
      const cls = [p.tags.result === "touchdown" ? "td" : "", p.tags.result === "turnover" ? "to" : "", p.key ? "key" : ""].join(" ");
      const epa = p.kind === "Kickoff" || p.kind === "ExtraPoint" ? "" : `<span class="vw-row-epa ${p.epa > 0 ? "up" : p.epa < 0 ? "down" : ""}">${signed(p.epa, 1)}</span>`;
      return `<button class="vw-row ${cls}" data-vw="row" data-n="${p.n}" style="--c:${u.c1}"><span class="vw-row-q st-tnum">${qWord(p.q)} ${esc(p.clock)}</span><i></i><span class="vw-row-t"><small>${esc(p.down > 0 ? downText(p) + " · " + p.spot : KIND_WORDS[p.kind] || p.kind)}${ix.teams[p.off] ? " · " + esc(ix.teams[p.off].short || ix.teams[p.off].name) : ""}</small>${esc(p.text)}</span>${epa}</button>`;
    }
    panelPlays() {
      const ix = this.film.ix;
      const mine = ix.mine;
      const name = (k) => (mine != null ? "Your" : esc(ix.teams[0].short || ix.teams[0].name));
      const kinds = [["all", "Every snap"], ["key", "Key plays"], ["offense", `${name()} offense`], ["defense", `${name()} defense`], ["special", "Special teams"]];
      const count = (k) => this.listFor(k).length;
      const chips = `<div class="vw-chiprow">${kinds.map(([k, t]) => `<button data-vw="list" data-k="${k}" aria-pressed="${!this.listName && this.listKind === k}">${t} <small>${count(k)}</small></button>`).join("")}</div>`;
      const inList = new Set(this.list);
      let lastQ = null;
      const rows = ix.plays
        .filter((p) => inList.has(p.n))
        .map((p) => {
          const head = p.q !== lastQ ? `<div class="vw-qhead">${p.q > 4 ? "Overtime" : "Quarter " + p.q}<small>${esc(ix.teams[1].short || ix.teams[1].name)} ${p.score[1]} · ${esc(ix.teams[0].short || ix.teams[0].name)} ${p.score[0]}</small></div>` : "";
          lastQ = p.q;
          return head + this.rowHtml(p);
        })
        .join("");
      const cut = this.listName ? `<div class="vw-cutnote">Playing: <b>${esc(this.listName)}</b> · ${this.list.length} snap${this.list.length === 1 ? "" : "s"} <a class="link" data-vw="list" data-k="all">Back to every snap</a></div>` : "";
      return `${chips}${cut}<div class="vw-rows">${rows || '<div class="empty">No snaps in this list.</div>'}</div>`;
    }
    highlightRow(scroll) {
      const body = this.$(".vw-side-body");
      if (!body || this.panel !== "plays" || !this.clip) return;
      body.querySelectorAll(".vw-row.on").forEach((r) => r.classList.remove("on"));
      const r = body.querySelector(`.vw-row[data-n="${this.clip.n}"]`);
      if (r) {
        r.classList.add("on");
        const br = body.getBoundingClientRect(), rr = r.getBoundingClientRect();
        if (scroll || rr.top < br.top || rr.bottom > br.bottom) body.scrollTop += rr.top - br.top - br.height * 0.35;
      }
    }

    // Cut-ups: every snap tagged by your staff, filtered the way coaches
    // break film down (all third downs against two-high, every red-zone
    // snap from 12 personnel). Inside a group any choice counts; across
    // groups all of them must.
    cutGroups() {
      const ix = this.film.ix;
      const mine = ix.mine ?? 0;
      const who = ix.mine != null ? "Your" : ix.teams[0].short || ix.teams[0].name;
      return [
        ["side", "Unit", (p) => (p.tags.unit !== "scrimmage" ? "Special teams" : p.off === mine ? `${who} offense` : `${who} defense`)],
        ["down", "Down", (p) => (p.down > 0 && p.tags.unit === "scrimmage" ? ORD[p.down] : "")],
        ["dist", "Distance", (p) => (p.tags.dist ? { short: "Short (1-3)", medium: "Medium (4-6)", long: "Long (7+)" }[p.tags.dist] : "")],
        ["zone", "Field", (p) => p.tags.zone],
        ["clock", "Clock", (p) => (p.tags.two_min ? "Two-minute" : "")],
        ["pers", "Personnel", (p) => (p.tags.personnel && p.tags.unit === "scrimmage" ? p.tags.personnel : "")],
        ["form", "Formation", (p) => (p.tags.unit === "scrimmage" ? p.tags.formation : "")],
        ["type", "Play", (p) => (p.tags.run_pass ? (p.tags.pa ? "Play action" : p.tags.run_pass === "run" ? "Run" : "Pass") : "")],
        ["cover", "Coverage", (p) => (p.tags.unit === "scrimmage" ? p.tags.cover : "")],
        ["rush", "Pressure", (p) => (p.tags.unit === "scrimmage" ? (p.tags.blitz ? "Blitz" : p.tags.rush || "") : "")],
        ["result", "Result", (p) => p.tags.result],
      ];
    }
    cutMatches(skip) {
      const groups = this.cutGroups();
      return this.film.ix.plays.filter((p) => groups.every(([k, , f]) => k === skip || !this.cut[k] || !this.cut[k].size || this.cut[k].has(f(p))));
    }
    panelCuts() {
      const groups = this.cutGroups();
      const all = this.cutMatches(null);
      const html = groups
        .map(([k, title, f]) => {
          const pool = this.cutMatches(k);
          const counts = new Map();
          for (const p of this.film.ix.plays) {
            const v = f(p);
            if (v) counts.set(v, 0);
          }
          for (const p of pool) {
            const v = f(p);
            if (v) counts.set(v, counts.get(v) + 1);
          }
          if (!counts.size) return "";
          const sel = this.cut[k] || new Set();
          const vals = [...counts.entries()].sort((a, b) => (k === "down" ? ORD.indexOf(a[0]) - ORD.indexOf(b[0]) : b[1] - a[1] || String(a[0]).localeCompare(b[0])));
          return `<div class="vw-cutg"><span class="st-label">${title}</span><div class="vw-chiprow wrap">${vals
            .map(([v, n]) => `<button data-vw="cut" data-g="${k}" data-v="${esc(v)}" aria-pressed="${sel.has(v)}" ${n === 0 && !sel.has(v) ? "disabled" : ""}>${esc(v)} <small>${n}</small></button>`)
            .join("")}</div></div>`;
        })
        .join("");
      const any = Object.values(this.cut).some((s) => s && s.size);
      const name = this.cutName();
      return `<div class="vw-cuthead"><div><b>${all.length}</b> snaps${any ? ` · ${esc(name)}` : ""}</div><div class="vw-cutbtns">${any ? '<button class="st-btn small glass" data-vw="cutclear">Clear</button>' : ""}<button class="st-btn small primary" data-vw="cutplay" ${all.length ? "" : "disabled"}>Play the cut-up</button></div></div>${html}`;
    }
    cutName() {
      const parts = [];
      for (const [k, title] of this.cutGroups()) {
        const s = this.cut[k];
        if (s && s.size) parts.push([...s].join(" or "));
        void title;
      }
      return parts.join(", ") || "Every snap";
    }

    // Grades: your position coaches grade your own men on every snap the
    // morning after: plus, zero or minus, with what he did.
    panelGrades() {
      const c = this.clip;
      const ix = this.film.ix;
      if (ix.mine == null) return '<div class="empty">Your position coaches grade your own team\'s film only.</div>';
      const g = (c && c.grades) || {};
      const ids = Object.keys(g).map(Number);
      const snap = ids.length
        ? ids
            .map((id) => {
              const pl = this.film.players.get(id) || { num: "", name: "#" + id, pos: "" };
              const x = g[id];
              return `<button class="vw-grade" data-vw="gpick" data-id="${id}"><span class="vw-mark m${x.mark === "+" ? "p" : x.mark === "-" ? "m" : "z"}">${x.mark}</span><span class="vw-grade-who"><b>#${pl.num} ${esc(pl.name)}</b> <small>${esc(pl.pos)}</small><small class="vw-grade-notes">${x.notes.map((n) => esc(n)).join(" · ")}</small></span></button>`;
            })
            .join("")
        : '<div class="empty">Nobody of yours to grade on this snap.</div>';
      let sheet;
      if (!this.sheet) {
        sheet = '<div class="vw-sheetwait"><span></span>Your position coaches are grading the game…</div>';
        if (!this._sheetAsk) {
          this._sheetAsk = true;
          this.film.source
            .grades()
            .then((s) => {
              this.sheet = s;
              if (this.panel === "grades") this.drawPanel();
            })
            .catch((e) => {
              this.sheet = { rows: [], err: e.message };
              if (this.panel === "grades") this.drawPanel();
            });
        }
      } else {
        const GROUPS = ["Quarterbacks", "Running backs", "Receivers and tight ends", "Offensive line", "Defensive line", "Linebackers", "Secondary", "Specialists"];
        let last = -1;
        sheet = (this.sheet.rows || [])
          .map((r) => {
            const head = r.group !== last ? `<div class="vw-qhead">${GROUPS[r.group] || "Others"}</div>` : "";
            last = r.group;
            const tone = r.grade >= 90 ? "up" : r.grade < 80 ? "down" : "";
            return `${head}<button class="vw-sheet" data-vw="gcut" data-id="${r.id}" title="Watch his snaps, isolated on him"><span class="vw-sheet-who"><b>#${r.num} ${esc(r.name)}</b> <small>${esc(r.pos)}</small><small class="vw-grade-notes">${r.notes.map(([k, n]) => `${esc(k)}${n > 1 ? " ×" + n : ""}`).join(" · ")}${r.busts ? ` · <span class="down">${r.busts} bust${r.busts > 1 ? "s" : ""}</span>` : ""}${r.loafs ? ` · <span class="down">${r.loafs} loaf${r.loafs > 1 ? "s" : ""}</span>` : ""}</small></span><span class="vw-sheet-n"><b class="${tone} st-tnum">${r.grade}%</b><small class="st-tnum">${r.snaps} snap${r.snaps === 1 ? "" : "s"} · +${r.plus} −${r.minus}</small></span></button>`;
          })
          .join("") || `<div class="empty">${esc(this.sheet.err || "No grades for this game.")}</div>`;
      }
      return `<div class="vw-sec"><span class="st-label">This snap</span>${snap}</div><div class="vw-sec"><span class="st-label">The game · the share of snaps each man did his job</span>${sheet}</div>`;
    }
    panelDraw() {
      const live = this.film.source.kind === "live";
      const c = this.clip;
      const mine = c ? this.marks.map((m, i) => [m, i]).filter(([m]) => m.play === c.n) : [];
      return `<div class="vw-sec"><span class="st-label">Draw on the film</span>
        <p class="vw-note">Pick a tool, then drag on the field. The film pauses while you draw${live ? "." : "; your drawings stay with this game for your staff."}</p>
        <div class="vw-tools">${TOOLS.map(([k, t, p]) => `<button data-vw="tool" data-k="${k}" aria-pressed="${this.tool === k}" title="${t}">${ico(p)}<span>${t}</span></button>`).join("")}</div>
        <div class="vw-colors">${COLORS.map((c, i) => `<button data-vw="color" data-i="${i}" aria-pressed="${this.color === i}" style="--c:${c}" title="${COLOR_NAMES[i]}" aria-label="${COLOR_NAMES[i]}"></button>`).join("")}</div>
        <div class="vw-drawbtns"><button class="st-btn small glass" data-vw="undo" ${this.marks.length ? "" : "disabled"}>${ico(IC.undo)} Undo <kbd>Z</kbd></button><button class="st-btn small glass" data-vw="clearsnap" ${mine.length ? "" : "disabled"}>Clear this snap</button></div></div>
        <div class="vw-sec"><span class="st-label">On this snap</span>${mine.length ? mine.map(([m, i]) => `<button class="vw-markrow" data-vw="jump" data-t="${m.t}"><i style="background:${COLORS[m.color]}"></i>${KIND_TOOL[m.kind]} at ${m.t < 0 ? "the set" : m.t.toFixed(1) + " s"}<span class="link" data-vw="delmark" data-i="${i}">Delete</span></button>`).join("") : '<div class="empty">No drawings on this snap yet.</div>'}</div>
        <div class="vw-sec"><span class="st-label">The whole game</span><p class="vw-note">${this.marks.length} drawing${this.marks.length === 1 ? "" : "s"} on ${((k) => `${k} snap${k === 1 ? "" : "s"}`)(new Set(this.marks.map((m) => m.play)).size)}. ${this.marks.length ? '<a class="link" data-vw="markcut">Watch the snaps you drew on</a>' : ""}</p></div>`;
    }
    panelView() {
      const sw = (k, t, sub) => `<label class="vw-sw"><input type="checkbox" data-vw="set" data-k="${k}" ${SET[k] ? "checked" : ""}><span class="vw-sw-t"><b>${t}</b>${sub ? `<small>${sub}</small>` : ""}</span><i></i></label>`;
      const sel = (k, t, opts) => `<label class="vw-sel"><b>${t}</b><select data-vw="set" data-k="${k}">${opts.map(([v, n]) => `<option value="${v}" ${String(SET[k]) === String(v) ? "selected" : ""}>${n}</option>`).join("")}</select></label>`;
      return `<div class="vw-sec"><span class="st-label">Watching</span>
          ${sw("auto", "Play on into the next snap", "Off: each snap stops at its end")}
          ${sel("pace", "Opens on", [["every", "Every snap"], ["key", "Key plays only"]])}
        </div>
        <div class="vw-sec"><span class="st-label">The 2D view</span>
          ${sw("figures", "Figures instead of dots", "Top-down players sized by their bodies")}
          ${sw("trails", "Route trails", "Each receiver's path since the snap, the target's in gold")}
          ${sw("sep", "Separation ring", "Around the target: green open, gold close, red covered")}
          ${sw("officials", "Officials and the chain crew", "")}
          ${sw("tiles", "Tracking numbers", "Time to throw, air yards, separation, speed")}
          ${sw("marks", "Your drawings", "")}
        </div>
        <div class="vw-sec"><span class="st-label">The 3D view</span>
          ${sw("three", "3D coaches' view", "The film in a stadium built from the real one: its seats, roof, turf, weather and light. Off keeps the 2D view only")}
          ${SET.three ? sel("quality", "Quality", [["auto", "Automatic for this Mac"], ["low", "Low: for older Macs"], ["medium", "Medium"], ["high", "High: smooth on an M1 and newer"]]) : ""}
          ${SET.three ? sw("replay", "Replays from a second angle", "Touchdowns, turnovers and big plays again from the end zone") : ""}
          ${SET.three ? sw("next", "The new look", "Men run curves and lean into cuts, never pass through each other, dive for low balls and go down with their tackler; pads, gloves and visors by position; dirt on the uniforms and wear on the grass as the season goes; a crowd for third downs; a broadcast camera with a man behind it") : ""}
        </div>
        <div class="vw-sec"><span class="st-label">Keys</span><div class="vw-keys">
          <span><kbd>Space</kbd> play or pause</span><span><kbd>←</kbd><kbd>→</kbd> a tenth of a second</span><span><kbd>Shift</kbd>+<kbd>←</kbd><kbd>→</kbd> previous or next snap</span>
          <span><kbd>1</kbd>–<kbd>4</kbd> cameras</span><span><kbd>V</kbd> 2D or 3D</span><span><kbd>D</kbd> draw</span><span><kbd>I</kbd> isolate the man you picked</span>
          <span><kbd>F</kbd> full screen</span><span><kbd>P</kbd> the panel</span><span><kbd>[</kbd><kbd>]</kbd> slower, faster</span><span><kbd>Esc</kbd> back out</span></div></div>`;
    }

    // ---------- events
    async onClick(e) {
      const b = e.target.closest("[data-vw]");
      if (!b || !this.el.contains(b)) return;
      const d = b.dataset;
      ACTIVE = this;
      switch (d.vw) {
        case "play": return this.toggle();
        case "prev": return this.prev();
        case "next": return this.next(false);
        case "back": return this.step(-0.1);
        case "fwd": return this.step(0.1);
        case "speed": SET.speed = Number(d.s); saveSettings(); return this.syncBar();
        case "full": return this.setFull(!this.full);
        case "side": return this.toggleSide();
        case "view": return this.toggleView();
        case "cam": return this.setCam(d.k);
        case "panel": this.panel = d.k; return this.drawPanel();
        case "row": {
          const i = this.list.indexOf(Number(d.n));
          return this.goTo(i, { play: true, card: false });
        }
        case "list": this.listKind = d.k; this.listName = null; this.setList(this.listFor(d.k), null); return this.goTo(0, { play: this.playing, card: true });
        case "cut": {
          const s = (this.cut[d.g] ||= new Set());
          if (s.has(d.v)) s.delete(d.v);
          else s.add(d.v);
          return this.drawPanel();
        }
        case "cutclear": this.cut = {}; return this.drawPanel();
        case "cutplay": {
          const ns = this.cutMatches(null).map((p) => p.n);
          if (!ns.length) return;
          this.setList(ns, this.cutName());
          this.panel = "plays";
          this.drawPanel();
          return this.goTo(0, { play: true, card: true });
        }
        case "gpick": {
          const id = Number(d.id);
          const i = this.clip ? this.clip.ids.indexOf(id) : -1;
          if (i >= 0) this.pick(i);
          return;
        }
        case "gcut": {
          const r = (this.sheet.rows || []).find((x) => x.id === Number(d.id));
          if (!r) return;
          const pl = this.film.players.get(r.id);
          this.setList(r.plays, `#${pl ? pl.num : ""} ${r.name}'s snaps`);
          this.isolateId = r.id;
          this.panel = "plays";
          this.drawPanel();
          return this.goTo(0, { play: true, card: true });
        }
        case "iso": return this.isolateOn(Number(d.id));
        case "unpick": this.picked = null; this.pickedId = null; return this.manCard();
        case "tool": return this.setTool(this.tool === d.k ? null : d.k);
        case "color": this.color = Number(d.i); this.drawPanel(); return this.syncBar();
        case "undo": return this.undoMark();
        case "clearsnap": this.marks = this.marks.filter((m) => !this.clip || m.play !== this.clip.n); this.saveMarks(); this.markers(); return this.drawPanel();
        case "delmark": e.stopPropagation(); this.marks.splice(Number(d.i), 1); this.saveMarks(); this.markers(); return this.drawPanel();
        case "jump": this.playing = false; this.seek(Number(d.t)); return this.syncBar();
        case "markcut": {
          const ns = [...new Set(this.marks.map((m) => m.play))].sort((a, b) => a - b);
          this.setList(ns, "Snaps you drew on");
          return this.goTo(0, { play: false, card: false });
        }
      }
    }
    onInput(e) {
      const el = e.target;
      if (el.dataset.vw !== "set") return;
      const k = el.dataset.k;
      const v = el.type === "checkbox" ? el.checked : el.value;
      SET[k] = v;
      saveSettings();
      if (k === "three") {
        if (!v && this.kind === "3d") this.useView("2d");
        this.drawPanel();
        this.syncBar();
      }
      if (k === "quality" && this.kind === "3d" && this.view.setQuality) this.view.setQuality(v);
      if (k === "next" && this.view) this.view.rebuild = true;
      if (k === "figures" || k === "trails") this.view && (this.view.snapCam = false);
    }
    setCam(k) {
      if (this.isolateId != null) this.isolateOn(null);
      if (this.kind === "3d") SET.cam3d = k;
      else {
        SET.cam2d = k;
        if (k !== "free" && this.view) this.view.free = null;
      }
      saveSettings();
      this.syncBar();
    }
    setTool(k) {
      if (k && this.kind === "3d" && !this.view.canDraw) this.useView("2d");
      this.tool = k;
      if (k) {
        this.playing = false;
        this.inter = 0;
        this.showCard(false);
      }
      if (this.panel === "draw") this.drawPanel();
      this.syncBar();
    }
    undoMark() {
      if (!this.marks.length) return;
      this.marks.pop();
      this.saveMarks();
      this.markers();
      if (this.panel === "draw") this.drawPanel();
    }
    // The film alone, wall to wall: the panel folds away and comes back.
    toggleSide() {
      SET.side = SET.side === false;
      saveSettings();
      this.el.classList.toggle("noside", !SET.side);
      const b = this.$('[data-vw="side"]');
      if (b) b.setAttribute("aria-pressed", String(SET.side));
      if (this.view) this.view.snapCam = true;
    }
    setFull(on) {
      this.full = on;
      this.el.classList.toggle("vw-full", on);
      this.el.classList.toggle("overlay", on);
      const b = this.$('[data-vw="full"]');
      if (b) {
        b.innerHTML = ico(on ? IC.shrink : IC.full);
        b.title = on ? "Leave full screen (Esc)" : "Full screen (F)";
      }
      if (this.view) this.view.snapCam = true;
      if (this.opts.onFull) this.opts.onFull(on);
      this.el.focus({ preventScroll: true });
    }
    key(ev) {
      const k = ev.key;
      const lower = k.length === 1 ? k.toLowerCase() : k;
      const take = () => {
        ev.preventDefault();
        ev.stopPropagation();
        ev.stopImmediatePropagation();
      };
      if (k === " " || lower === "k") return take(), this.toggle();
      if (k === "ArrowLeft" || k === "ArrowRight") {
        take();
        if (ev.shiftKey) return k === "ArrowLeft" ? this.prev() : this.next(false);
        return this.step(k === "ArrowLeft" ? -0.1 : 0.1);
      }
      if ((k === "ArrowUp" || k === "ArrowDown") && this.full) return take(), k === "ArrowUp" ? this.prev() : this.next(false);
      if (lower === "v") return take(), this.toggleView();
      if (lower === "f" && !this.inline) return take(), this.setFull(!this.full);
      if (lower === "p" && !this.inline) return take(), this.toggleSide();
      if (lower === "d" && !this.inline) return take(), this.setTool(this.tool ? null : "arrow");
      if (lower === "z" && !this.inline) return take(), this.undoMark();
      if (lower === "i") {
        take();
        if (this.picked != null && this.clip) return this.isolateOn(this.clip.ids[this.picked]);
        return this.isolateOn(null);
      }
      if (lower === "r") return take(), this.seek(this.clip ? this.clip.t0 : 0), (this.playing = true), this.syncBar();
      if (k === "[" || k === "]") {
        take();
        const sp = [0.25, 0.5, 1, 2];
        const i = Math.max(0, Math.min(3, sp.indexOf(SET.speed) + (k === "]" ? 1 : -1)));
        SET.speed = sp[i < 0 ? 2 : i];
        saveSettings();
        return this.syncBar();
      }
      if (["1", "2", "3", "4"].includes(k)) {
        take();
        const cams = this.kind === "3d" ? ["coaches", "all22", "broadcast", "free"] : ["all22", "tight", "endzone", "free"];
        return this.setCam(cams[Number(k) - 1]);
      }
      if (k === "Escape") {
        if (this.tool) return take(), this.setTool(null);
        if (this.isolateId != null) return take(), this.isolateOn(null);
        if (this.picked != null) return take(), ((this.picked = null), this.manCard());
        if (this.full && !this.opts.fullOnly) return take(), this.setFull(false);
        if (this.opts.onClose) return take(), this.opts.onClose();
      }
    }
    dispose() {
      if (this.dead) return;
      this.dead = true;
      cancelAnimationFrame(this.raf);
      if (this.ro) this.ro.disconnect();
      if (this.view) this.view.dispose();
      clearTimeout(this._saveT);
      if (this._saveT && this.film.source.kind !== "live") this.film.source.saveMarks(this.marks).catch(() => {});
      players.delete(this);
      if (ACTIVE === this) ACTIVE = [...players].pop() || null;
      // Free the film on the game's side once nothing is watching it.
      setTimeout(() => {
        if (![...players].some((p) => p.film.source.kind === this.film.source.kind)) api("film_close").catch(() => {});
      }, 1500);
    }
  }

  // Keys go to the player you last touched, while it is on screen, before
  // the rest of the game sees them (Space is Continue elsewhere).
  document.addEventListener(
    "keydown",
    (ev) => {
      const p = ACTIVE;
      if (!p || p.dead || !p.root.isConnected) return;
      if (ev.metaKey || ev.ctrlKey || ev.altKey) {
        if ((ev.metaKey || ev.ctrlKey) && ev.key.toLowerCase() === "z" && !p.inline && p.marks.length) {
          ev.preventDefault();
          ev.stopPropagation();
          p.undoMark();
        }
        return;
      }
      const typing = ev.target.closest && ev.target.closest("input, textarea, select, [contenteditable]");
      if (typing) return;
      // Another window over the film (a letter, a dialog) has the keys.
      const over = [...document.querySelectorAll(".overlay")].some((o) => !o.classList.contains("vw-full") && !o.classList.contains("lv-sheet-o"));
      if (over) return;
      const r = p.el.getBoundingClientRect();
      if (!p.full && (r.bottom < 60 || r.top > innerHeight - 60)) return;
      if (p.inline && !["v", "V", "1", "2", "3", "4", "[", "]"].includes(ev.key)) return;
      p.key(ev);
    },
    true,
  );

  // ---------------------------------------------------------------- 3D
  let threeP = null;
  function loadScript(src) {
    return new Promise((res, rej) => {
      const s = document.createElement("script");
      s.src = src;
      s.onload = res;
      s.onerror = () => rej(new Error("missing " + src));
      document.head.appendChild(s);
    });
  }
  // three.js and the 3D view load the first time you open the 3D view, off
  // the screens' first paint.
  function loadThree() {
    if (window.AFMViews3D) return Promise.resolve();
    threeP ||= loadScript("screens/views/three.min.js").then(() => loadScript("screens/views/views3d.js"));
    return threeP;
  }

  // ---------------------------------------------------------------- mounting
  async function mount(el, src, opts = {}) {
    el.dataset.on = "1";
    el.innerHTML = `<div class="vw-loading"><span class="vw-spin"></span><b>${opts.live ? "Loading the snaps so far" : "Loading the film"}</b><small>${opts.live ? "" : "The game plays again from its own seed: every snap as it happened."}</small></div>`;
    let ix;
    try {
      ix = await src.open();
    } catch (e) {
      if (el.isConnected) el.innerHTML = `<div class="vw-loading"><b>No film</b><small>${esc(e.message)}</small></div>`;
      return null;
    }
    if (!el.isConnected) return null;
    const film = new V.Film(ix, src);
    return new Player(el, film, opts);
  }

  // The game page's Watch tab: the film beside its play list.
  window.gamePageTabs = window.gamePageTabs || {};
  const pending = { n: null };
  window.gamePageTabs.Watch = async (g, a) => {
    const start = pending.n ?? a.n ?? "";
    pending.n = null;
    return `<div class="vw-mount" data-index="${a.index}" data-start="${start}"></div>`;
  };
  // A Watch link anywhere on a game page opens that snap in the Watch tab.
  document.addEventListener(
    "click",
    (ev) => {
      const el = ev.target.closest("[data-vwgo]");
      if (!el) return;
      ev.preventDefault();
      ev.stopPropagation();
      const n = Number(el.dataset.vwgo);
      const live = [...players].find((p) => p.root.closest(".vw-mount") && p.film.source.kind === "kept");
      if (live && live.root.isConnected) {
        live.setList(live.listFor("all"), null);
        live.goTo(live.list.indexOf(n), { play: true, card: true });
        live.el.scrollIntoView({ block: "nearest", behavior: "smooth" });
        return;
      }
      pending.n = n;
      S.keep.gptab = { t: "Watch" };
      render().then(() => {
        const m = document.querySelector(".vw-mount");
        if (m) m.scrollIntoView({ block: "start" });
      });
    },
    true,
  );

  // Mount any Watch tab that appears.
  const mo = new MutationObserver(() => {
    for (const el of document.querySelectorAll(".vw-mount:not([data-on])")) {
      const index = Number(el.dataset.index);
      const start = el.dataset.start ? Number(el.dataset.start) : null;
      mount(el, source("kept", index), { start, autoplay: start != null });
      // The tab is for the film: bring it up to the top of the window.
      requestAnimationFrame(() => el.isConnected && el.scrollIntoView({ block: "start", behavior: "smooth" }));
    }
    liveHook();
  });
  mo.observe(document.getElementById("app") || document.body, { childList: true, subtree: true });

  // ---------------------------------------------------------------- full screen
  // Watching any film full screen, from anywhere (the live screen's Film
  // button): closes back to where you were.
  async function watchFull(src, opts = {}) {
    const host = document.createElement("div");
    host.className = "vw-fullhost";
    document.body.appendChild(host);
    const close = () => {
      host.remove();
      if (opts.onClose) opts.onClose();
    };
    const p = await mount(host, src, { ...opts, full: true, fullOnly: true, onClose: close });
    if (!p) {
      setTimeout(close, 1600);
      return null;
    }
    const b = p.$('[data-vw="full"]');
    if (b) {
      b.innerHTML = ico(IC.close);
      b.title = "Close the film (Esc)";
      b.dataset.vw = "closefull";
      b.addEventListener("click", close);
    }
    return p;
  }

  // ---------------------------------------------------------------- live
  // The live screen: a 2D panel under its scoreboard plays each snap as it
  // is played (yours when you call it, the staff's while you watch), and
  // the Film button (R) opens every snap so far full screen.
  const LIVE = { seen: -1, game: null, panel: null };
  function liveHook() {
    if (typeof S === "undefined" || !S.route || S.route.screen !== "live") return null;
    const field = document.querySelector(".lv-room .lv-field");
    if (!field) return null;
    if (field.dataset.vw) return document.querySelector(".vw-livebox");
    field.dataset.vw = "1";
    const box = document.createElement("div");
    box.className = "vw-livebox" + (SET.liveHidden ? " off" : "");
    box.innerHTML = `<div class="vw-livehost"></div><div class="vw-livebar"><button class="st-btn small glass" data-vwlive="film" title="Every snap so far, full screen (R)">Film <kbd>R</kbd></button><button class="st-btn small glass" data-vwlive="hide">${SET.liveHidden ? "2D view" : "Field chart"}</button></div>`;
    field.parentElement.insertBefore(box, field);
    box.addEventListener("click", (e) => {
      const b = e.target.closest("[data-vwlive]");
      if (!b) return;
      e.stopPropagation();
      if (b.dataset.vwlive === "film") openLiveFilm();
      if (b.dataset.vwlive === "hide") {
        SET.liveHidden = !SET.liveHidden;
        saveSettings();
        box.classList.toggle("off", SET.liveHidden);
        b.textContent = SET.liveHidden ? "2D view" : "Field chart";
        if (!SET.liveHidden) startLivePanel(box, {});
        else if (LIVE.panel) LIVE.panel.playing = false;
      }
    });
    // While your staff calls it and you watch, the live screen hands each
    // snap over (V.liveHold); otherwise the snaps since your last call play.
    if (!SET.liveHidden && !document.querySelector(".lv-watching")) startLivePanel(box, {});
    return box;
  }
  async function startLivePanel(box, o) {
    const host = box.querySelector(".vw-livehost");
    if (!host || host.dataset.on) {
      if (o.done) o.done();
      return;
    }
    host.dataset.on = "1";
    box.classList.remove("ff");
    const src = source("live");
    let ix;
    try {
      ix = await src.open();
    } catch (e) {
      if (o.done) o.done();
      return;
    }
    if (!host.isConnected) {
      if (o.done) o.done();
      return;
    }
    // A new game starts from its first snap.
    const game = ix.date + ix.teams.map((t) => t.name).join();
    if (LIVE.game !== game) {
      LIVE.game = game;
      LIVE.seen = ix.plays.length > 1 ? ix.plays[ix.plays.length - 2].n : -1;
    }
    const last = ix.plays.length ? ix.plays[ix.plays.length - 1].n : null;
    let fresh = ix.plays.filter((p) => p.n > LIVE.seen);
    if (o.keyOnly) fresh = fresh.filter((p) => p.key);
    LIVE.seen = last ?? LIVE.seen;
    if (last == null) {
      if (o.done) o.done();
      return;
    }
    const film = new V.Film(ix, src);
    const list = fresh.length ? fresh.map((p) => p.n) : [last];
    const p = new Player(host, film, { inline: true, list, autoplay: fresh.length > 0, autoNext: true });
    if (!fresh.length) {
      await p.goTo(0, { t: 1e9 });
      if (o.done) o.done();
    } else p.onEnd = () => o.done && o.done();
    LIVE.panel = p;
  }
  async function openLiveFilm() {
    if (LIVE.panel) LIVE.panel.playing = false;
    await watchFull(source("live"), { start: LIVE.seen >= 0 ? LIVE.seen : null, paused: true });
  }
  document.addEventListener(
    "keydown",
    (ev) => {
      if (typeof S === "undefined" || !S.route || S.route.screen !== "live" || ev.metaKey || ev.ctrlKey || ev.altKey) return;
      if (ev.key.toLowerCase() !== "r" || !document.querySelector(".vw-livebox")) return;
      if (ev.target.closest && ev.target.closest("input, textarea, select, [contenteditable]")) return;
      if (document.querySelector(".overlay")) return;
      ev.preventDefault();
      ev.stopPropagation();
      openLiveFilm();
    },
    true,
  );
  // While you watch your staff call the game, the next snap waits until the
  // 2D view has shown this one: every snap, or at key-play pace only the
  // key ones (the rest go by on the field chart). A promise for the live
  // screen, or null when the 2D view is off or this snap isn't shown.
  V.liveHold = (pace, key) => {
    const box = liveHook();
    if (!box || SET.liveHidden) return null;
    if (pace === "key" && !key) {
      box.classList.add("ff");
      return null;
    }
    return new Promise((res) => {
      let done = false;
      const end = () => {
        if (!done) res((done = true));
      };
      startLivePanel(box, { done: end, keyOnly: pace === "key" });
      // Never hold the game longer than a long snap takes.
      setTimeout(end, 16000 / Math.max(0.25, SET.speed));
    });
  };

  Object.assign(V, { Player, source, mount, watchFull, loadThree, active: () => ACTIVE });
})();
