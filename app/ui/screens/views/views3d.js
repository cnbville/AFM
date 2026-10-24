// Lane screens: views, the 3D coaches' view (later extras topic 3, Cain's
// mock-up B). three.js (vendored beside this file, r149, MIT) draws the
// same moment the 2D view draws: the men built from their bodies (height,
// weight, arms, skin) in their team's uniform and number, posed by what
// each is doing, the ball in its spiral or end over end, the officials and
// the chain crew, in a stadium built from the real one (its shape, seats
// and crowd, roof, turf, track, press box and scoreboard) under the day's
// light and weather. Cameras: the coaches' end-zone copy, the All-22 from
// the sideline, the broadcast angle, isolated on one man, and free.
//
// On by default since Cain okayed the clip (2026-10-23); the "3D coaches' view"
// setting turns it off.
"use strict";

(() => {
  const T = window.THREE;
  const V = window.AFMViews;
  const { SET, FIELD_W, FIELD_H, MARGIN_X, MARGIN_Y, ACT } = V;
  // Yards everywhere: x along the field from the left end line, z across
  // from the far sideline, y up. The field's middle is the origin.
  const WX = (x) => x - FIELD_W / 2;
  const WZ = (y) => y - FIELD_H / 2;
  const TAU = Math.PI * 2;
  const clamp = (v, a, b) => Math.max(a, Math.min(b, v));
  const lerp = (a, b, u) => a + (b - a) * u;

  // ---------------------------------------------------------------- quality
  // High: 60 frames a second on an 8 GB M1 (shadows, the full crowd,
  // sideline players, sharp at the screen's own resolution). Low: 30 on an
  // older Intel Mac (blob shadows, a thinner crowd, half resolution).
  const QUAL = {
    low: { dpr: 1, shadow: 0, crowd: 0.28, side: 0, fps: 30, aa: false, parts: 900, overlay: 6, detail: 0 },
    medium: { dpr: 1.5, shadow: 1024, crowd: 0.6, side: 10, fps: 60, aa: true, parts: 2200, overlay: 8, detail: 1 },
    high: { dpr: 2, shadow: 2048, crowd: 1, side: 22, fps: 60, aa: true, parts: 4000, overlay: 10, detail: 2 },
  };
  function gpuName() {
    try {
      const c = document.createElement("canvas");
      const gl = c.getContext("webgl2") || c.getContext("webgl");
      if (!gl) return "";
      const ext = gl.getExtension("WEBGL_debug_renderer_info");
      return ext ? String(gl.getParameter(ext.UNMASKED_RENDERER_WEBGL)) : String(gl.getParameter(gl.RENDERER));
    } catch (e) {
      return "";
    }
  }
  function autoQuality() {
    const g = gpuName();
    if (/Apple M\d|Apple GPU/i.test(g)) return "high";
    if (/Intel|SwiftShader|llvmpipe|Software/i.test(g)) return "low";
    return "medium";
  }

  // A fixed random stream from a string (the same stadium every time).
  function rng(seed) {
    let h = 2166136261;
    for (const ch of String(seed)) h = Math.imul(h ^ ch.charCodeAt(0), 16777619);
    return () => {
      h ^= h << 13;
      h ^= h >>> 17;
      h ^= h << 5;
      return ((h >>> 0) % 100000) / 100000;
    };
  }
  const col = (hex) => new T.Color(hex);

  // ---------------------------------------------------------------- textures
  function canvasTex(w, h, draw, opts = {}) {
    const c = document.createElement("canvas");
    c.width = w;
    c.height = h;
    draw(c.getContext("2d"), w, h);
    const t = new T.CanvasTexture(c);
    t.encoding = T.sRGBEncoding;
    if (opts.repeat) {
      t.wrapS = t.wrapT = T.RepeatWrapping;
      t.repeat.set(opts.repeat[0], opts.repeat[1]);
    }
    if (opts.aniso) t.anisotropy = opts.aniso;
    return t;
  }
  // A jersey number in the team's number colour with its outline.
  function numberTex(num, color, edge) {
    return canvasTex(128, 128, (g) => {
      g.textAlign = "center";
      g.textBaseline = "middle";
      g.font = '900 92px Archivo, "Arial Black", sans-serif';
      g.lineWidth = 9;
      g.strokeStyle = edge;
      g.strokeText(String(num), 64, 70);
      g.fillStyle = color;
      g.fillText(String(num), 64, 70);
    });
  }
  const blobTex = (dark = 0.55) =>
    canvasTex(64, 64, (g) => {
      const r = g.createRadialGradient(32, 32, 2, 32, 32, 31);
      r.addColorStop(0, `rgba(0,0,0,${dark})`);
      r.addColorStop(1, "rgba(0,0,0,0)");
      g.fillStyle = r;
      g.fillRect(0, 0, 64, 64);
    });
  // Officials' stripes.
  const stripeTex = () =>
    canvasTex(64, 64, (g) => {
      g.fillStyle = "#f4f4f4";
      g.fillRect(0, 0, 64, 64);
      g.fillStyle = "#111";
      for (let x = 0; x < 64; x += 16) g.fillRect(x, 0, 8, 64);
    }, { repeat: [3, 1] });

  // ---------------------------------------------------------------- bodies
  // Shared unit shapes, scaled to each man.
  let GEO = null;
  function geos() {
    if (GEO) return GEO;
    GEO = {
      // Capsules one yard tall and half a yard across, scaled to each limb.
      cap: new T.CapsuleGeometry(0.25, 0.5, 3, 8),
      capLo: new T.CapsuleGeometry(0.25, 0.5, 2, 6),
      ball: new T.SphereGeometry(1, 14, 10),
      head: new T.SphereGeometry(1, 16, 12),
      headLo: new T.SphereGeometry(1, 10, 8),
      box: new T.BoxGeometry(1, 1, 1),
      mask: new T.TorusGeometry(1, 0.09, 4, 10, Math.PI),
      plane: new T.PlaneGeometry(1, 1),
      cyl: new T.CylinderGeometry(1, 1, 1, 10),
      ring: new T.RingGeometry(0.86, 1, 40),
    };
    GEO.ball.scale(1, 0.62, 0.62);
    return GEO;
  }
  // One man: legs, hips and torso with shoulder pads, helmet and mask, arms;
  // his height, build and arm length from the player model.
  function makeMan(kit, body, q, mats, extra = {}) {
    const G = geos();
    const H = clamp((body.ht || 73) / 36, 1.6, 2.3); // inches to yards
    const w = clamp(Math.sqrt((body.wt || 220) / 230), 0.78, 1.32);
    const armLen = clamp((body.arm || 32.5) / 36, 0.78, 1.02);
    const root = new T.Group();
    // Turned to his facing first, then rolled about his own forward line.
    root.rotation.order = "YXZ";
    const m = (geo, mat, sx, sy, sz) => {
      const o = new T.Mesh(geo, mat);
      o.scale.set(sx, sy, sz);
      o.castShadow = q.shadow > 0;
      return o;
    };
    const cap = q.detail ? G.cap : G.capLo;
    // A limb: radii across (x, z) and length, in yards.
    const limb = (mat, rx, len, rz) => m(cap, mat, rx * 4, len, rz * 4);
    const dim = {
      H,
      w,
      hip: 0.52 * H,
      thigh: 0.245 * H,
      shin: 0.25 * H,
      torso: 0.27 * H,
      hipW: 0.085 * H * w,
      shW: 0.12 * H * w + 0.06,
      upper: 0.165 * H * armLen,
      fore: 0.2 * H * armLen,
    };
    const hips = new T.Group();
    root.add(hips);
    const leg = (s) => {
      const g = new T.Group();
      g.position.set(0, 0, s * dim.hipW);
      const thigh = limb(mats.pants, 0.085 * w, dim.thigh * 1.05, 0.09 * w);
      thigh.position.y = -dim.thigh / 2;
      g.add(thigh);
      const knee = new T.Group();
      knee.position.y = -dim.thigh;
      const shin = limb(mats.sock, 0.055 * w, dim.shin * 1.02, 0.058 * w);
      shin.position.y = -dim.shin / 2;
      knee.add(shin);
      if (q.detail) {
        const shoe = m(G.box, mats.shoe, 0.15, 0.05, 0.075);
        shoe.position.set(0.04, -dim.shin + 0.005, 0);
        knee.add(shoe);
      }
      g.add(knee);
      hips.add(g);
      return { g, knee };
    };
    const L = leg(-1), R = leg(1);
    const torso = new T.Group();
    hips.add(torso);
    const belly = limb(mats.jersey, 0.14 * w, dim.torso * 1.02, 0.19 * w);
    belly.position.y = dim.torso * 0.46;
    torso.add(belly);
    // Pads by position on the next look: a lineman's wide and tall, a
    // receiver's or a back's cut down, a kicker's barely there.
    const pw = extra.pads || 1;
    const pads = m(G.box, mats.jersey, 0.3 * w * (0.6 + 0.4 * pw), 0.14 * pw, dim.shW * 2.1 * pw);
    pads.position.y = dim.torso * 0.86;
    torso.add(pads);
    if (kit.num) {
      const f = new T.Mesh(G.plane, kit.num);
      f.scale.set(0.22, 0.22, 1);
      f.rotation.y = Math.PI / 2;
      f.position.set(0.14 * w + 0.006, dim.torso * 0.55, 0);
      torso.add(f);
      const b = new T.Mesh(G.plane, kit.num);
      b.scale.set(0.26, 0.26, 1);
      b.rotation.y = -Math.PI / 2;
      b.position.set(-0.14 * w - 0.006, dim.torso * 0.6, 0);
      torso.add(b);
    }
    const neck = new T.Group();
    neck.position.y = dim.torso + 0.04;
    torso.add(neck);
    const helmet = m(q.detail ? G.head : G.headLo, mats.helmet, 0.135, 0.13, 0.12);
    helmet.position.y = 0.12;
    neck.add(helmet);
    if (q.detail) {
      const mask = m(G.mask, mats.mask, 0.1, 0.1, 0.105);
      mask.rotation.set(0, Math.PI / 2, Math.PI / 2);
      mask.position.set(0.1, 0.08, 0);
      neck.add(mask);
      if (mats.stripe) {
        const st = m(G.box, mats.stripe, 0.24, 0.012, 0.025);
        st.position.set(0.0, 0.25, 0);
        neck.add(st);
      }
      if (extra.visor) {
        const vz = m(G.box, extra.visor, 0.01, 0.045, 0.16);
        vz.position.set(0.128, 0.125, 0);
        neck.add(vz);
      }
    }
    if (extra.cap) {
      const c = m(G.cyl, extra.cap, 0.12, 0.05, 0.12);
      c.position.y = 0.24;
      neck.add(c);
      helmet.material = mats.skin;
      helmet.scale.set(0.11, 0.12, 0.1);
    }
    const arm = (s) => {
      const g = new T.Group();
      g.position.set(0, dim.torso * 0.84, s * dim.shW);
      const up = limb(mats.sleeve, 0.062 * w, dim.upper * 1.05, 0.062 * w);
      up.position.y = -dim.upper / 2;
      g.add(up);
      const el = new T.Group();
      el.position.y = -dim.upper;
      const fo = limb(extra.long ? mats.sleeve : mats.skin, 0.045 * w, dim.fore * 1.05, 0.045 * w);
      fo.position.y = -dim.fore / 2;
      el.add(fo);
      if (extra.gloves && q.detail) {
        const hand = m(G.headLo, extra.gloves, 0.045 * w, 0.06, 0.04);
        hand.position.y = -dim.fore - 0.035;
        el.add(hand);
      }
      g.add(el);
      torso.add(g);
      return { g, el };
    };
    const AL = arm(-1), AR = arm(1);
    // An invisible column to pick him with.
    const hit = new T.Mesh(G.cyl, mats.hit);
    hit.scale.set(0.6, H, 0.6);
    hit.position.y = H / 2;
    root.add(hit);
    // A soft shadow on the turf under him.
    const blob = new T.Mesh(G.plane, mats.blob);
    blob.rotation.x = -Math.PI / 2;
    blob.position.y = 0.02;
    blob.scale.set(1.3 * w, 1.3 * w, 1);
    blob.renderOrder = 1;
    root.add(blob);
    // High on the next look: a tight, darker shadow where he meets the
    // turf, the way light fails under a body and between bodies.
    let core = null;
    if (extra.core) {
      core = new T.Mesh(G.plane, extra.core);
      core.rotation.x = -Math.PI / 2;
      core.position.y = 0.025;
      core.scale.set(0.62 * w, 0.5 * w, 1);
      core.renderOrder = 1;
      root.add(core);
    }
    return { root, hips, torso, neck, L, R, AL, AR, dim, hit, blob, core, fall: 0, actWas: -1, actT: 0 };
  }

  // Bend his joints for what he is doing. Angles are about his own axes:
  // forward swing is positive, a bent knee negative.
  function pose(man, m, t, dt) {
    const d = man.dim;
    const act = m.act;
    if (act !== man.actWas) {
      man.actWas = act;
      man.actT = t;
    }
    const since = t - man.actT;
    const run = clamp(m.speed / 7.5, 0, 1);
    const next = SET.next;
    // A full stride (two steps) is about two and a half yards at speed. On
    // the next look the stride is counted step by step, so a man speeding
    // up lengthens his stride without his feet skipping.
    let ph;
    if (next) {
      const dw = m.walked - (man.wasWalked == null ? m.walked : man.wasWalked);
      man.wasWalked = m.walked;
      if (Math.abs(dw) < 3) man.ph = (man.ph || 0) + (Math.abs(dw) / (1.6 + 1.0 * run)) * TAU;
      ph = man.ph || 0;
    } else ph = (m.walked / (1.6 + 1.0 * run)) * TAU;
    const sw = Math.sin(ph), cw = Math.cos(ph);
    let lean = 0.06 + 0.26 * run;
    let l = 0.65 * run * sw, r = -0.65 * run * sw;
    let kl = -Math.max(0, -Math.sin(ph + 0.9)) * 1.3 * run - 0.08, kr = -Math.max(0, Math.sin(ph + 0.9)) * 1.3 * run - 0.08;
    let al = -0.7 * run * sw, ar = 0.7 * run * sw, el = -0.4 - 0.9 * run, er = -0.4 - 0.9 * run;
    let spreadL = 0.08, spreadR = 0.08;
    let fallTo = 0;
    let roll = 0;
    let lift = 0;
    const walk = !run && m.speed > 0.3;
    if (walk) {
      l = 0.25 * sw;
      r = -0.25 * sw;
      al = -0.15 * sw;
      ar = 0.15 * sw;
    }
    switch (act) {
      case ACT.stance: {
        // Linemen in a three-point stance, backs and receivers in a two-point.
        const low = man.lineman;
        lean = low ? 1.15 : 0.55;
        l = r = low ? 1.0 : 0.55;
        kl = kr = low ? -1.55 : -0.9;
        l += 0.12;
        r -= 0.12;
        al = low ? 1.75 : 0.9;
        ar = low ? 0.6 : 0.9;
        el = low ? 0 : -0.5;
        er = low ? -0.9 : -0.5;
        break;
      }
      case ACT.block:
      case ACT.engaged:
      case ACT.rush:
        lean = 0.42 + 0.15 * run;
        al = ar = 1.35;
        el = er = -0.55;
        spreadL = spreadR = 0.3;
        if (!run) {
          l = 0.35 + 0.1 * Math.sin(t * 7 + man.seed);
          r = -0.1;
          kl = kr = -0.55;
        }
        // The man winning the rep drives low into his man; the one losing
        // is stood up, his arms pushed high.
        if (next && man.win) {
          lean += man.win > 0 ? 0.14 : -0.2;
          if (man.win < 0) {
            al = ar = 1.7;
            kl = kr = -0.35;
          }
        }
        break;
      case ACT.tackle:
      case ACT.wrapped:
        lean = act === ACT.tackle ? 0.75 : -0.2;
        al = ar = act === ACT.tackle ? 1.4 : 0.7;
        el = er = -0.9;
        spreadL = spreadR = 0.55;
        break;
      case ACT.cover:
        lean = 0.2 + 0.2 * run;
        if (!run) {
          l = 0.3;
          r = -0.2;
          kl = kr = -0.6;
          al = ar = 0.35;
        }
        break;
      case ACT.pocket:
      case ACT.fake:
        lean = 0.12;
        al = ar = 0.75;
        el = er = -1.6;
        spreadL = spreadR = -0.25;
        if (act === ACT.fake) {
          ar = 1.1;
          er = -0.2;
        }
        break;
      case ACT.throw: {
        const u = clamp(since / 0.35, 0, 1);
        lean = 0.18 + 0.2 * u;
        ar = lerp(2.9, 1.25, u);
        er = lerp(-1.9, -0.15, u);
        al = 0.9;
        el = -1.2;
        l = 0.45;
        r = -0.35;
        break;
      }
      case ACT.catch:
        al = ar = 2.3;
        el = er = -0.35;
        spreadL = spreadR = -0.1;
        lean = 0.05;
        // Hands to where the ball arrives: up for a high ball, out in front
        // at the chest, low for a ball at his knees; a ball over his reach
        // is a leap, one out of reach or at his shoetops a dive.
        if (next && man.reach) {
          const R = man.reach;
          const sh = (d.hip + d.torso * 0.84) * Math.cos(lean);
          const out = Math.max(0.25, Math.hypot(R.fwd, R.side));
          al = ar = Math.atan2(out, sh - R.z);
          el = er = -0.2;
          roll = clamp(R.side * 0.3, -0.4, 0.4);
          if (R.z > d.H * 1.3) {
            lift = clamp(R.z - d.H * 1.22, 0.15, 0.5);
            al = ar = 2.95;
            l = 0.5;
            kl = -1.3;
            r = -0.1;
            kr = -0.6;
          } else if (R.dive) {
            fallTo = 0.82;
            al = ar = 2.95;
            el = er = -0.05;
            l = -0.35;
            r = -0.6;
            kl = kr = -0.2;
          }
        }
        break;
      case ACT.carry:
        // The ball tucked high and tight on his outside arm.
        ar = 0.55;
        er = -2.0;
        spreadR = -0.15;
        lean = 0.22 + 0.25 * run;
        break;
      case ACT.celebrate:
        al = ar = 2.9 + 0.2 * Math.sin(t * 9 + man.seed);
        el = er = -0.15;
        spreadL = spreadR = 0.35;
        lean = -0.12;
        break;
      case ACT.kneel:
        l = 1.55;
        kl = -1.55;
        r = -0.1;
        kr = -1.6;
        lean = 0.15;
        al = ar = 0.8;
        el = er = -1.5;
        break;
      case ACT.fall:
        fallTo = clamp(since / 0.55, 0, 1);
        lean = 0.5;
        al = ar = 1.6;
        break;
      case ACT.down:
        fallTo = 1;
        al = ar = 2.6;
        el = er = -0.2;
        l = r = 0.05;
        kl = kr = -0.15;
        break;
      case ACT.getup:
        fallTo = clamp(1 - since / 0.8, 0, 1);
        break;
      case ACT.stand:
        if (!run && !walk) {
          al = ar = 0.05 + 0.03 * Math.sin(t * 1.5 + man.seed);
          el = er = -0.15;
        }
        break;
    }
    // Leaning into it: forward when he speeds up, back when he brakes, and
    // into the turn on a cut, as far as the push against the ground takes
    // him (gravity is 10.7 yards a second squared).
    if (next && m.ax != null && act !== ACT.down && act !== ACT.fall && act !== ACT.stance) {
      const cf = Math.cos(m.face), sf = Math.sin(m.face);
      const ahead = m.ax * cf + m.ay * sf, across = -m.ax * sf + m.ay * cf;
      const moving = clamp(m.speed / 3, 0, 1);
      lean += clamp(Math.atan2(ahead, 10.7) * 0.9, -0.22, 0.35) * (0.4 + 0.6 * moving);
      roll += clamp(Math.atan2(across, 10.7) * 0.9, -0.38, 0.38) * moving;
    }
    // Ease the joints so a change of act never snaps.
    const k = 1 - Math.pow(0.0005, dt);
    const S = man.sm || (man.sm = { lean, l, r, kl, kr, al, ar, el, er, sl: spreadL, sr: spreadR, fall: fallTo, roll });
    const E = (key, v) => (S[key] += (v - S[key]) * k);
    E("lean", lean); E("l", l); E("r", r); E("kl", kl); E("kr", kr); E("al", al); E("ar", ar); E("el", el); E("er", er); E("sl", spreadL); E("sr", spreadR);
    S.roll = (S.roll || 0) + (roll - (S.roll || 0)) * (1 - Math.pow(0.002, dt));
    S.lift = (S.lift || 0) + (lift - (S.lift || 0)) * (1 - Math.pow(0.0005, dt));
    man.root.rotation.x = next ? S.roll : 0;
    S.fall += (fallTo - S.fall) * (1 - Math.pow(0.002, dt));
    man.L.g.rotation.z = S.l;
    man.L.knee.rotation.z = S.kl;
    man.R.g.rotation.z = S.r;
    man.R.knee.rotation.z = S.kr;
    man.torso.rotation.z = -S.lean;
    man.AL.g.rotation.z = S.al;
    man.AL.g.rotation.x = -S.sl;
    man.AL.el.rotation.z = -S.el;
    man.AR.g.rotation.z = S.ar;
    man.AR.g.rotation.x = S.sr;
    man.AR.el.rotation.z = -S.er;
    // The hips sit as high as the straighter leg reaches.
    const reach = (a, kk) => d.thigh * Math.cos(a) + d.shin * Math.cos(a + kk);
    const hy = Math.max(reach(S.l, S.kl), reach(S.r, S.kr)) + 0.03;
    man.hips.position.y = lerp(hy, 0.14, S.fall) + (next ? S.lift : 0);
    man.hips.rotation.z = -S.fall * (Math.PI / 2 - 0.12);
    man.hips.position.x = S.fall * 0.25;
  }

  // ---------------------------------------------------------------- the next look
  // View only (Cain's list, 2026-10-23): nothing here changes where the
  // engine puts anyone, only how it looks getting there.
  const DOWN = (a) => a === ACT.fall || a === ACT.down;
  // Where man i is at time t, straight from the engine's frames.
  function posAt(c, i, t) {
    const [k, u] = V.at(c, t);
    const k2 = Math.min(c.frames - 1, k + 1), o = 6 + i * 5;
    const f = c.f, A = k * c.S, B = k2 * c.S;
    return [f[A + o] + (f[B + o] - f[A + o]) * u, f[A + o + 1] + (f[B + o + 1] - f[A + o + 1]) * u];
  }
  function nextLook(st, c) {
    const men = st.men, n = men.length;
    for (const m of men) {
      m.dx = m.x;
      m.dy = m.y;
      m.win = 0;
      m.pile = 0;
    }
    // Bodies never pass through each other: men closer than their
    // shoulders allow are eased apart (two passes), men locked up in a
    // block or a tackle only as far as chest to chest. Men in their stance
    // keep the engine's spots, and men on the ground can pile up.
    const free = (m) => m.act !== ACT.stance && m.act !== ACT.kneel && !DOWN(m.act);
    for (let pass = 0; pass < 2; pass++) {
      for (let i = 0; i < n; i++) {
        const a = men[i];
        if (!free(a)) continue;
        for (let j = i + 1; j < n; j++) {
          const b = men[j];
          if (!free(b)) continue;
          const ex = b.dx - a.dx, ey = b.dy - a.dy, d2 = ex * ex + ey * ey;
          const pair = a.with === j || b.with === i;
          const min = !pair ? 0.62 : a.act === ACT.tackle || a.act === ACT.wrapped || b.act === ACT.tackle || b.act === ACT.wrapped ? 0.42 : 0.5;
          if (d2 >= min * min) continue;
          const d = Math.sqrt(d2);
          const ux = d > 1e-4 ? ex / d : Math.cos(i * 2.4), uy = d > 1e-4 ? ey / d : Math.sin(i * 2.4);
          const push = (min - d) / 2;
          a.dx -= ux * push;
          a.dy -= uy * push;
          b.dx += ux * push;
          b.dy += uy * push;
        }
      }
    }
    for (let i = 0; i < n; i++) {
      const m = men[i];
      const j = m.with;
      const mate = j != null && j < n && j !== i ? men[j] : null;
      // Who is winning a block: over the next half second, does the pair
      // move toward his man (he drives him) or toward him (he gives ground)?
      if (mate && (m.act === ACT.block || m.act === ACT.engaged || m.act === ACT.rush) && st.t > 0) {
        const [x0, y0] = posAt(c, i, st.t), [x1, y1] = posAt(c, i, st.t + 0.5);
        const [u0, v0] = posAt(c, j, st.t), [u1, v1] = posAt(c, j, st.t + 0.5);
        const ex = u0 - x0, ey = v0 - y0, e = Math.hypot(ex, ey) || 1;
        const drift = (((x1 + u1) - (x0 + u0)) / 2 * ex + ((y1 + v1) - (y0 + v0)) / 2 * ey) / e;
        m.win = drift > 0.12 ? 1 : drift < -0.12 ? -1 : 0;
      }
      // Going down: the way his momentum and his tackler's take him.
      if (m.act === ACT.fall) {
        let vx = m.vx || 0, vy = m.vy || 0;
        if (mate) {
          vx += mate.vx || 0;
          vy += mate.vy || 0;
        }
        let tw = 0;
        if (Math.hypot(vx, vy) > 1.2) {
          tw = Math.atan2(vy, vx) - m.face;
          while (tw > Math.PI) tw -= TAU;
          while (tw < -Math.PI) tw += TAU;
        }
        m.twist = clamp(tw, -2.2, 2.2);
      }
      // The catch: the ball's spot against his body, a dive when it is low
      // or past his reach.
      if (i === c.catcher && c.catchBall) {
        const [bx, by, bz] = c.catchBall;
        const ex = bx - m.x, ey = by - m.y, cf = Math.cos(m.face), sf = Math.sin(m.face);
        const fwd = ex * cf + ey * sf, side = -ex * sf + ey * cf;
        // A ball over his standing reach (about 1.3 times his height) is a
        // leap, never a dive.
        const dive = bz < 2.2 && (bz < 0.45 || Math.hypot(fwd, side) > 1.25);
        m.reach = { fwd, side, z: bz, dive };
        if (dive && m.act === ACT.catch) m.twist = clamp(Math.atan2(side, Math.max(0.2, fwd)), -1.2, 1.2);
      }
    }
    // A pile: each man on the ground lies on the ones under him.
    for (let i = 0; i < n; i++) {
      const a = men[i];
      if (!DOWN(a.act)) continue;
      let under = 0;
      for (let j = 0; j < i; j++) {
        const b = men[j];
        if (DOWN(b.act) && Math.hypot(a.x - b.x, a.y - b.y) < 0.8) under++;
      }
      a.pile = Math.min(3, under) * 0.24;
    }
  }

  // ---------------------------------------------------------------- stadium
  // A rounded rectangle around the field (half lengths a along the field,
  // b across, corner radius r): n points evenly along it, with their
  // outward normals, counterclockwise from the near side's middle.
  function rrect(a, b, r, n) {
    const sa = 2 * (a - r), sb = 2 * (b - r), arc = (Math.PI / 2) * r;
    const corner = (cx, cz, f0) => (s) => {
      const f = f0 + s / r;
      return { x: cx + r * Math.sin(f), z: cz + r * Math.cos(f), nx: Math.sin(f), nz: Math.cos(f) };
    };
    const pieces = [
      [sa / 2, (s) => ({ x: s, z: b, nx: 0, nz: 1 })],
      [arc, corner(a - r, b - r, 0)],
      [sb, (s) => ({ x: a, z: b - r - s, nx: 1, nz: 0 })],
      [arc, corner(a - r, -(b - r), Math.PI / 2)],
      [sa, (s) => ({ x: a - r - s, z: -b, nx: 0, nz: -1 })],
      [arc, corner(-(a - r), -(b - r), Math.PI)],
      [sb, (s) => ({ x: -a, z: -(b - r) + s, nx: -1, nz: 0 })],
      [arc, corner(-(a - r), b - r, (3 * Math.PI) / 2)],
      [sa / 2, (s) => ({ x: -(a - r) + s, z: b, nx: 0, nz: 1 })],
    ];
    const per = pieces.reduce((t, p) => t + p[0], 0);
    const pts = [];
    for (let i = 0; i < n; i++) {
      let s = (i / n) * per;
      for (const [len, f] of pieces) {
        if (s <= len) {
          pts.push(f(s));
          break;
        }
        s -= len;
      }
    }
    return { pts, per };
  }
  // Stands from the venue: a bowl, a horseshoe, two sides or bleachers, as
  // many tiers as the seats need; the crowd sits in them as full as the
  // home crowd's noise says.
  function buildStadium(scene, film, q, mats) {
    const v = film.ix.venue || {};
    const home = film.ix.teams[0];
    const rnd = rng(v.name || home.name);
    const group = new T.Group();
    scene.add(group);
    const seats = v.seats || 20000;
    const fill = v.fill || 0.8;
    const shape = v.shape || "bowl";
    const seatColor = new T.Color(home.colors[0]).lerp(new T.Color("#3a3f47"), 0.55);
    const concrete = new T.MeshLambertMaterial({ color: "#6d6f73" });
    const seatTex = canvasTex(8, 64, (g) => {
      for (let y = 0; y < 64; y += 8) {
        g.fillStyle = "#" + seatColor.getHexString();
        g.fillRect(0, y, 8, 5);
        g.fillStyle = "#56585c";
        g.fillRect(0, y + 5, 8, 3);
      }
    });
    seatTex.wrapS = seatTex.wrapT = T.RepeatWrapping;
    const seatMat = new T.MeshLambertMaterial({ map: seatTex, side: T.DoubleSide });
    const crowd = { pos: [], col: [], phase: [], side: [] };
    const fanCols = [home.colors[0], home.colors[1] || "#ffffff", "#e8e4dc", "#2b2d31", "#5a6270", "#b23b3b", "#2c4f8a"];
    const awayCol = film.ix.teams[1].colors[0];
    // One stand: a sloped band from (inner offset, h0) outward `rows` rows,
    // over the parts of the path `keep` says.
    const stand = (a, b, r, off, rows, depth, rise, h0, keep, density) => {
      const n = 240;
      const { pts: path, per } = rrect(a, b, r, n);
      const pos = [], uv = [], idx = [];
      const outer = off + rows * depth;
      let made = 0;
      for (let i = 0; i <= n; i++) {
        const p = path[i % n];
        const kept = keep(p);
        pos.push(p.x + p.nx * off, h0, p.z + p.nz * off, p.x + p.nx * outer, h0 + rows * rise, p.z + p.nz * outer);
        uv.push(i / 6, 0, i / 6, rows / 8);
        if (i > 0 && kept && keep(path[(i - 1) % n])) {
          const k = (i - 1) * 2;
          idx.push(k, k + 1, k + 2, k + 1, k + 3, k + 2);
          made++;
        }
      }
      if (!made) return;
      const geo = new T.BufferGeometry();
      geo.setAttribute("position", new T.Float32BufferAttribute(pos, 3));
      geo.setAttribute("uv", new T.Float32BufferAttribute(uv, 2));
      geo.setIndex(idx);
      geo.computeVertexNormals();
      const mesh = new T.Mesh(geo, seatMat);
      mesh.receiveShadow = q.shadow > 0;
      group.add(mesh);
      // The back wall and the fascia under the front row.
      const wall = [], widx = [];
      for (let i = 0; i <= n; i++) {
        const p = path[i % n];
        wall.push(p.x + p.nx * outer, h0 + rows * rise + 1.6, p.z + p.nz * outer, p.x + p.nx * outer, 0, p.z + p.nz * outer, p.x + p.nx * off, h0, p.z + p.nz * off, p.x + p.nx * off, 0, p.z + p.nz * off);
        if (i > 0 && keep(p) && keep(path[(i - 1) % n])) {
          const k = (i - 1) * 4;
          widx.push(k, k + 4, k + 1, k + 1, k + 4, k + 5, k + 2, k + 3, k + 6, k + 3, k + 7, k + 6);
        }
      }
      const wg = new T.BufferGeometry();
      wg.setAttribute("position", new T.Float32BufferAttribute(wall, 3));
      wg.setIndex(widx);
      wg.computeVertexNormals();
      group.add(new T.Mesh(wg, new T.MeshLambertMaterial({ color: "#3b3e44", side: T.DoubleSide })));
      // The crowd: one point a fan in a seat (a share of them at Low).
      const kept = path.map((p, i) => (keep(p) ? i : -1)).filter((i) => i >= 0);
      const len = (kept.length / n) * per;
      const want = Math.min(60000, Math.round(((rows * len) / 0.55) * fill * q.crowd * density));
      for (let j = 0; j < want; j++) {
        const i = kept[Math.floor(rnd() * kept.length)];
        const p = path[i];
        const row = Math.floor(rnd() * rows);
        const o = off + (row + 0.55) * depth;
        const jit = (rnd() - 0.5) * 2.2;
        const tx = -p.nz, tz = p.nx;
        crowd.pos.push(p.x + p.nx * o + tx * jit, h0 + row * rise + 0.55, p.z + p.nz * o + tz * jit);
        const away = p.x > a * 0.6 && p.z < -b * 0.3 && rnd() < 0.75;
        const c = new T.Color(away ? (rnd() < 0.6 ? awayCol : "#d8d8d8") : fanCols[Math.floor(rnd() * (rnd() < 0.55 ? 2 : fanCols.length))]);
        c.offsetHSL(0, 0, (rnd() - 0.5) * 0.18);
        crowd.col.push(c.r, c.g, c.b);
        crowd.phase.push(rnd() * TAU);
        crowd.side.push(away ? 1 : 0);
      }
    };
    const all = () => true;
    if (shape === "bleachers") {
      // High school and small college: aluminum bleachers on both sides,
      // the home side bigger, a press box on top of it.
      const homeSeats = seats * 0.68, awaySeats = seats - homeSeats;
      const len = clamp(Math.sqrt(seats) * 1.25, 44, 110);
      const blo = (side, n) => {
        const rows = clamp(Math.round(n / (len / 0.5)), 5, 40);
        const depth = 0.75, rise = 0.32;
        const z0 = side * (FIELD_H / 2 + 8);
        const frame = new T.Group();
        const seatsM = new T.Mesh(geos().box, new T.MeshLambertMaterial({ color: "#a9adb3" }));
        const run = rows * depth, h = rows * rise;
        const slope = Math.atan2(h, run);
        seatsM.scale.set(len, 0.12, Math.hypot(run, h));
        seatsM.rotation.x = side * slope;
        seatsM.position.set(0, 1.1 + h / 2, z0 + side * run / 2);
        frame.add(seatsM);
        for (let x = -len / 2; x <= len / 2; x += 6) {
          const post = new T.Mesh(geos().box, new T.MeshLambertMaterial({ color: "#6b6e72" }));
          post.scale.set(0.15, h + 1.1, 0.15);
          post.position.set(x, (h + 1.1) / 2, z0 + side * run);
          frame.add(post);
        }
        group.add(frame);
        for (let j = 0; j < n * fill * q.crowd; j++) {
          const row = Math.floor(rnd() * rows);
          crowd.pos.push((rnd() - 0.5) * len, 1.1 + row * rise + 0.7, z0 + side * (row + 0.5) * depth);
          const c = new T.Color(side > 0 ? fanCols[Math.floor(rnd() * fanCols.length)] : rnd() < 0.5 ? awayCol : "#cfcfcf");
          c.offsetHSL(0, 0, (rnd() - 0.5) * 0.2);
          crowd.col.push(c.r, c.g, c.b);
          crowd.phase.push(rnd() * TAU);
          crowd.side.push(side > 0 ? 0 : 1);
        }
        return { rows, run, h, z0 };
      };
      const hs = blo(1, homeSeats);
      blo(-1, awaySeats);
      if (v.press_box) {
        const pb = new T.Mesh(geos().box, [concrete, concrete, concrete, concrete, new T.MeshLambertMaterial({ color: "#1e2a33", emissive: "#2a3a48", emissiveIntensity: film.ix.venue.light === "night" ? 0.8 : 0.1 }), concrete]);
        pb.scale.set(Math.min(26, len * 0.4), 3.2, 3.6);
        pb.position.set(0, 1.1 + hs.h + 1.8, hs.z0 + hs.run + 1.2);
        group.add(pb);
      }
    } else {
      const a = FIELD_W / 2 + 14, b = FIELD_H / 2 + 11, r = 20;
      const keep =
        shape === "two sides" ? (p) => Math.abs(p.nz) > 0.7 && Math.abs(p.x) < a - 8 :
        shape === "horseshoe" ? (p) => !(p.nx > 0.5) :
        all;
      const ring = rrect(a, b, r, 240);
      const keptShare = ring.pts.filter(keep).length / 240;
      const perRow = (ring.per * keptShare) / 0.52;
      let rows = Math.round(seats / perRow);
      const tiers = rows > 90 ? 3 : rows > 42 ? 2 : 1;
      let off = 0, h0 = 0.9;
      for (let k = 0; k < tiers; k++) {
        const n = Math.round(rows / tiers);
        const depth = 0.9, rise = k === 0 ? 0.42 : 0.6;
        stand(a, b, r, off, n, depth, rise, h0, keep, 1);
        // Suites and the concourse between tiers: the next tier sits back
        // and up, overhanging a little.
        off += n * depth - 4;
        h0 += n * rise + 5.5;
      }
      // The rim: lights along the top at night, a roof over a dome.
      const top = h0 + 2;
      if (v.roof === "dome" || (v.roof === "retractable" && v.closed)) {
        const roof = new T.Mesh(new T.SphereGeometry(1, 32, 12, 0, TAU, 0, Math.PI / 2), new T.MeshLambertMaterial({ color: "#d9dcdf", side: T.BackSide }));
        roof.scale.set(a + off + 6, 28, b + off + 6);
        roof.position.y = top - 2;
        group.add(roof);
        // The roof's trusses.
        const truss = new T.MeshLambertMaterial({ color: "#a7abb0" });
        for (let i = -3; i <= 3; i++) {
          const t2 = new T.Mesh(geos().box, truss);
          t2.scale.set(0.5, 0.5, (b + off) * 2);
          t2.position.set(i * 22, top + 24, 0);
          group.add(t2);
        }
      } else if (v.roof === "retractable") {
        const rail = new T.MeshLambertMaterial({ color: "#8b9096" });
        for (const s of [-1, 1]) {
          const m2 = new T.Mesh(geos().box, rail);
          m2.scale.set((a + off) * 2, 1.2, 10);
          m2.position.set(0, top + 8, s * (b + off - 6));
          group.add(m2);
        }
      }
      // The video board over one end zone.
      if (shape !== "two sides") {
        const board = new T.Mesh(geos().box, mats.board);
        board.scale.set(1.5, 11, 36);
        board.position.set(a + 16, 24, 0);
        board.userData.board = true;
        group.add(board);
        mats.boardMesh = board;
      }
    }
    // The ground around the field: the apron and the walls' shadow.
    const ground = new T.Mesh(new T.PlaneGeometry(420, 320), new T.MeshLambertMaterial({ color: v.surface === "turf" ? "#1d3d26" : "#244a2a" }));
    ground.rotation.x = -Math.PI / 2;
    ground.position.y = -0.04;
    ground.receiveShadow = q.shadow > 0;
    group.add(ground);
    if (crowd.pos.length) group.add(crowdPoints(crowd, mats));
    // Light towers around a field without a bowl's rim lights.
    if (v.light !== "day" || shape === "bleachers") {
      const pole = new T.MeshLambertMaterial({ color: "#7c8086" });
      const bank = new T.MeshBasicMaterial({ color: v.light === "night" || v.light === "dusk" ? "#fffbe8" : "#d8d8d0" });
      const spots = shape === "bleachers" ? [[-35, 1], [0, 1], [35, 1], [-35, -1], [0, -1], [35, -1]] : [];
      for (const [x, s] of spots) {
        const p = new T.Mesh(geos().cyl, pole);
        p.scale.set(0.3, 26, 0.3);
        p.position.set(x, 13, s * (FIELD_H / 2 + 18));
        group.add(p);
        const b2 = new T.Mesh(geos().box, bank);
        b2.scale.set(5, 3, 0.5);
        b2.position.set(x, 26, s * (FIELD_H / 2 + 17.6));
        b2.lookAt(x, 0, 0);
        group.add(b2);
      }
    }
    return group;
  }
  // The crowd as points that stand and bounce when their team scores.
  function crowdPoints(c, mats) {
    const g = new T.BufferGeometry();
    g.setAttribute("position", new T.Float32BufferAttribute(c.pos, 3));
    g.setAttribute("color", new T.Float32BufferAttribute(c.col, 3));
    g.setAttribute("phase", new T.Float32BufferAttribute(c.phase, 1));
    g.setAttribute("side", new T.Float32BufferAttribute(c.side, 1));
    const m = new T.ShaderMaterial({
      uniforms: { time: { value: 0 }, cheer: { value: new T.Vector2(0, 0) }, scale: { value: 600 }, light: { value: 1 } },
      vertexShader: `attribute float phase; attribute float side; attribute vec3 color; uniform float time; uniform vec2 cheer; uniform float scale; varying vec3 vC;
        void main(){ vec3 p = position; float ch = side < 0.5 ? cheer.x : cheer.y;
          p.y += abs(sin(time * 8.0 + phase)) * 0.32 * ch + sin(time * 1.1 + phase) * 0.03;
          vC = color; vec4 mv = modelViewMatrix * vec4(p, 1.0); gl_PointSize = clamp(0.62 * scale / -mv.z, 1.0, 14.0); gl_Position = projectionMatrix * mv; }`,
      fragmentShader: `varying vec3 vC; uniform float light; void main(){ vec2 d = gl_PointCoord - 0.5; float r = dot(d, d); if (r > 0.25) discard; gl_FragColor = vec4(vC * light * (1.0 - r * 1.4), 1.0); }`,
    });
    mats.crowd = m;
    const pts = new T.Points(g, m);
    pts.frustumCulled = false;
    return pts;
  }
  // Goalposts: the gooseneck behind the end line, the crossbar ten feet up,
  // uprights 18 feet 6 inches apart (23 feet 4 at a high school).
  function goalposts(scene, film) {
    const mat = new T.MeshLambertMaterial({ color: "#ffd60a" });
    const hs = film.ix.level === "hs";
    const half = (hs ? 23.33 : 18.5) / 6;
    const G = geos();
    for (const x of [0, FIELD_W]) {
      const s = x ? 1 : -1;
      const g = new T.Group();
      g.position.set(WX(x), 0, 0);
      const base = new T.Mesh(G.cyl, mat);
      base.scale.set(0.16, 3.33, 0.16);
      base.position.set(s * 2, 1.66, 0);
      g.add(base);
      const neck = new T.Mesh(G.cyl, mat);
      neck.scale.set(0.1, 2, 0.1);
      neck.rotation.z = Math.PI / 2;
      neck.position.set(s * 1, 3.33, 0);
      g.add(neck);
      const bar = new T.Mesh(G.cyl, mat);
      bar.scale.set(0.08, half * 2, 0.08);
      bar.rotation.x = Math.PI / 2;
      bar.position.set(0, 3.33, 0);
      g.add(bar);
      for (const zz of [-half, half]) {
        const up = new T.Mesh(G.cyl, mat);
        up.scale.set(0.07, 10, 0.07);
        up.position.set(0, 3.33 + 5, zz);
        g.add(up);
        const ribbon = new T.Mesh(G.box, new T.MeshLambertMaterial({ color: "#ff9f0a" }));
        ribbon.scale.set(0.05, 1.2, 0.12);
        ribbon.position.set(0, 13.9, zz);
        g.add(ribbon);
      }
      scene.add(g);
    }
    // Pylons at the corners of the end zones.
    const py = new T.MeshLambertMaterial({ color: "#ff6a00" });
    for (const x of [0, 10, 110, 120])
      for (const y of [0, FIELD_H]) {
        const p = new T.Mesh(G.box, py);
        p.scale.set(0.11, 0.5, 0.11);
        p.position.set(WX(x), 0.25, WZ(y));
        scene.add(p);
      }
  }

  // ---------------------------------------------------------------- the view
  class View3D {
    constructor(host, player) {
      this.player = player;
      this.host = host;
      this.canDraw = true;
      this.qName = SET.quality === "auto" ? autoQuality() : SET.quality;
      this.q = QUAL[this.qName] || QUAL.medium;
      try {
        this.renderer = new T.WebGLRenderer({ antialias: this.q.aa, powerPreference: "high-performance" });
      } catch (e) {
        this.ok = false;
        return;
      }
      this.ok = true;
      const r = this.renderer;
      r.outputEncoding = T.sRGBEncoding;
      r.toneMapping = T.ACESFilmicToneMapping;
      r.toneMappingExposure = 1.05;
      r.shadowMap.enabled = this.q.shadow > 0;
      r.shadowMap.type = T.PCFSoftShadowMap;
      this.cv = r.domElement;
      this.cv.className = "vw-canvas";
      host.appendChild(this.cv);
      this.scene = new T.Scene();
      this.camera = new T.PerspectiveCamera(30, 16 / 9, 0.5, 1200);
      this.camPos = new T.Vector3(0, 30, 80);
      this.camAt = new T.Vector3(0, 0, 0);
      this.fov = 30;
      this.orbit = { th: 0.5, ph: 0.9, r: 70, at: new T.Vector3() };
      this.ray = new T.Raycaster();
      this.lastDraw = 0;
      this.slow = 0;
      this.tags = document.createElement("div");
      this.tags.className = "vw3-tags";
      host.appendChild(this.tags);
      this.wire();
    }
    setQuality(name) {
      this.qName = name === "auto" ? autoQuality() : name;
      this.q = QUAL[this.qName] || QUAL.medium;
      this.rebuild = true;
    }
    resize() {
      const rr = this.host.getBoundingClientRect();
      const w = Math.max(1, Math.round(rr.width)), h = Math.max(1, Math.round(rr.height));
      const dpr = Math.min(window.devicePixelRatio || 1, this.q.dpr);
      if (this.w !== w || this.h !== h || this.dpr !== dpr) {
        this.w = w;
        this.h = h;
        this.dpr = dpr;
        this.renderer.setPixelRatio(dpr);
        this.renderer.setSize(w, h, false);
        this.camera.aspect = w / h;
        this.camera.updateProjectionMatrix();
      }
    }

    // ---------- building the scene for a film
    build(film) {
      const S = this.scene;
      while (S.children.length) S.remove(S.children[0]);
      this.disposeScene();
      this.film = film;
      const ix = film.ix, v = ix.venue || {};
      const q = this.q;
      const G = geos();
      const mats = (this.mats = { list: [] });
      const keep = (m) => (mats.list.push(m), m);
      mats.hit = keep(new T.MeshBasicMaterial({ visible: false }));
      mats.blob = keep(new T.MeshBasicMaterial({ map: blobTex(), transparent: true, depthWrite: false }));
      mats.board = keep(new T.MeshLambertMaterial({ color: "#111", emissive: "#ffffff", emissiveMap: null }));
      // The sky, the sun and the lights by the hour, the month and the roof.
      this.light(v);
      // The field: the 2D view's painted field on the turf.
      const fieldKey = SET.next ? "fieldCanvas3dNext" : "fieldCanvas3d";
      film[fieldKey] = film[fieldKey] || V.paintField(film, { natural: true, wear: SET.next });
      const ft = new T.CanvasTexture(film[fieldKey]);
      ft.encoding = T.sRGBEncoding;
      ft.anisotropy = this.renderer.capabilities.getMaxAnisotropy();
      this.fieldTex = ft;
      const fieldM = new T.Mesh(new T.PlaneGeometry(FIELD_W + 2 * MARGIN_X, FIELD_H + 2 * MARGIN_Y), keep(new T.MeshLambertMaterial({ map: ft, color: new T.Color(0.78, 0.78, 0.78) })));
      fieldM.rotation.x = -Math.PI / 2;
      fieldM.receiveShadow = q.shadow > 0;
      S.add(fieldM);
      this.fieldM = fieldM;
      // Drawings and route trails on a sheet just over the turf.
      const OV = q.overlay;
      this.ovPx = OV;
      const oc = document.createElement("canvas");
      oc.width = Math.round((FIELD_W + 2 * MARGIN_X) * OV);
      oc.height = Math.round((FIELD_H + 2 * MARGIN_Y) * OV);
      this.ovCanvas = oc;
      this.ovTex = new T.CanvasTexture(oc);
      this.ovTex.encoding = T.sRGBEncoding;
      const ovM = new T.Mesh(fieldM.geometry, keep(new T.MeshBasicMaterial({ map: this.ovTex, transparent: true, depthWrite: false })));
      ovM.rotation.x = -Math.PI / 2;
      ovM.position.y = 0.03;
      ovM.renderOrder = 2;
      S.add(ovM);
      this.ovM = ovM;
      // The line of scrimmage and the line to gain, on the turf under the men.
      const line = (color) => {
        const m = new T.Mesh(G.box, keep(new T.MeshBasicMaterial({ color, transparent: true, opacity: 0.88, depthWrite: false })));
        m.scale.set(0.14, 0.01, FIELD_H);
        m.position.y = 0.035;
        m.renderOrder = 3;
        S.add(m);
        return m;
      };
      this.los = line("#3d7dff");
      this.ltg = line("#ffd60a");
      goalposts(S, film);
      this.stadium = buildStadium(S, film, q, mats);
      if (mats.crowd) mats.crowd.uniforms.light.value = this.night ? 0.62 : 1;
      // Every man in the film, built once from his body.
      const U = film.uni;
      const kits = U.map((u, side) => {
        const mk = (c, extra = {}) => keep(new T.MeshLambertMaterial({ color: c, ...extra }));
        return {
          jersey: mk(u.jersey),
          sleeve: mk(u.jersey),
          pants: mk(u.pants),
          sock: mk(side === 0 ? u.c1 : "#f2f2f0"),
          helmet: keep(new T.MeshStandardMaterial({ color: u.helmet, roughness: 0.32, metalness: 0.15 })),
          stripe: mk(u.stripe),
          mask: mk("#c8ccd0"),
          shoe: mk("#141414"),
          hit: mats.hit,
          blob: mats.blob,
          nums: new Map(),
          u,
        };
      });
      this.kits = kits;
      this.skins = new Map();
      this.men = new Map();
      const lineman = new Set(["OT", "IOL", "DT", "EDGE", "LS"]);
      // The next look: each position group's own jersey, pants and socks,
      // so the dirt can find the linemen first; gear by position and by the
      // day (long sleeves in the cold, gloves, visors).
      const next = SET.next;
      this.next = next;
      const GROUP = { OT: 0, IOL: 0, DT: 0, EDGE: 0, LS: 0, RB: 1, FB: 1, TE: 1, LB: 1, WR: 2, CB: 2, S: 2, QB: 3, K: 4, P: 4 };
      const PADS = [1.12, 1.03, 0.92, 0.95, 0.8];
      this.dirt = [];
      const groupKit = (side, g) => {
        const kit = kits[side];
        const k2 = (kit.groups ||= []);
        if (k2[g]) return k2[g];
        const mk = (base) => {
          const mm = keep(base.clone());
          this.dirt.push({ mat: mm, base: base.color.clone(), g, part: base === kit.pants ? 1 : base === kit.sock ? 0.8 : 0.7 });
          return mm;
        };
        k2[g] = { ...kit, jersey: mk(kit.jersey), sleeve: mk(kit.sleeve), pants: mk(kit.pants), sock: mk(kit.sock) };
        return k2[g];
      };
      const visorM = next ? keep(new T.MeshStandardMaterial({ color: "#20242a", roughness: 0.15, metalness: 0.4, transparent: true, opacity: 0.85 })) : null;
      const coreM = next && q.detail === 2 ? keep(new T.MeshBasicMaterial({ map: blobTex(0.8), transparent: true, depthWrite: false })) : null;
      const gloveMs = kits.map((kit) => [keep(new T.MeshLambertMaterial({ color: kit.u.c1 })), keep(new T.MeshLambertMaterial({ color: "#16171a" })), keep(new T.MeshLambertMaterial({ color: "#ecebe6" }))]);
      const cold = v.temperature != null && v.temperature < 45, cool = v.temperature != null && v.temperature < 60;
      const gear = (pl) => {
        if (!next) return {};
        const g = GROUP[pl.pos] ?? 2;
        const r = rng("gear" + pl.id);
        const roll = (p) => r() < p;
        const line = g === 0;
        return {
          pads: PADS[g],
          visor: roll(line ? 0.12 : g === 4 ? 0.05 : 0.35) ? visorM : null,
          long: roll(cold ? (line ? 0.35 : 0.75) : cool ? (line ? 0.05 : 0.15) : 0.02),
          gloves: g === 4 || (g === 3 && !roll(0.2)) ? null : roll(line ? 0.75 : 0.95) ? gloveMs[pl.side][line ? (roll(0.5) ? 2 : 1) : roll(0.6) ? 0 : 1] : null,
          core: coreM,
          group: g,
        };
      };
      for (const pl of film.players.values()) {
        const kit = kits[pl.side];
        let num = kit.nums.get(pl.num);
        if (!num) {
          const t = numberTex(pl.num, kit.u.num, kit.u.side === 0 ? kit.u.trim : "rgba(0,0,0,.35)");
          num = keep(new T.MeshLambertMaterial({ map: t, transparent: true, alphaTest: 0.4 }));
          kit.nums.set(pl.num, num);
        }
        let skin = this.skins.get(pl.skin);
        if (!skin) this.skins.set(pl.skin, (skin = keep(new T.MeshLambertMaterial({ color: pl.skin }))));
        const g = gear(pl);
        const man = makeMan({ num }, pl, q, { ...(next ? groupKit(pl.side, g.group) : kit), skin }, g);
        man.lineman = lineman.has(pl.pos);
        man.seed = (pl.id % 97) / 15;
        man.root.visible = false;
        man.hit.userData.id = pl.id;
        S.add(man.root);
        this.men.set(pl.id, man);
      }
      // The officials and the chain crew.
      const stripe = keep(new T.MeshLambertMaterial({ map: stripeTex() }));
      const black = keep(new T.MeshLambertMaterial({ color: "#16171a" }));
      const white = keep(new T.MeshLambertMaterial({ color: "#f1f1f1" }));
      const vest = keep(new T.MeshLambertMaterial({ color: "#ff8f1f" }));
      const offSkin = keep(new T.MeshLambertMaterial({ color: "#c58c5c" }));
      this.crew = [];
      for (let i = 0; i < 14; i++) {
        const chain = i >= 8;
        const o = makeMan({}, { ht: 71, wt: 205, arm: 32 }, { ...q, detail: Math.min(1, q.detail), shadow: q.shadow }, { jersey: chain ? vest : stripe, sleeve: chain ? vest : stripe, pants: black, sock: black, helmet: offSkin, skin: offSkin, shoe: black, mask: black, hit: mats.hit, blob: mats.blob }, { cap: i === 0 ? white : black });
        o.root.visible = false;
        o.seed = i;
        S.add(o.root);
        this.crew.push(o);
      }
      // The chains: two rods with orange tops and the chain between, the box.
      const rodM = keep(new T.MeshLambertMaterial({ color: "#ff8f1f" }));
      this.rods = [0, 1, 2].map((k) => {
        const g = new T.Group();
        const p = new T.Mesh(G.cyl, black);
        p.scale.set(0.04, 2.4, 0.04);
        p.position.y = 1.2;
        g.add(p);
        const top = new T.Mesh(k === 2 ? G.box : G.cyl, rodM);
        top.scale.set(k === 2 ? 0.5 : 0.22, 0.35, k === 2 ? 0.08 : 0.22);
        top.position.y = 2.45;
        g.add(top);
        g.visible = false;
        S.add(g);
        return g;
      });
      const chain = new T.Mesh(G.box, keep(new T.MeshLambertMaterial({ color: "#2a2a2a" })));
      chain.scale.set(10, 0.03, 0.03);
      chain.visible = false;
      S.add(chain);
      this.chain = chain;
      // Players off the field stand along their team's sideline.
      this.side = [];
      const area = ix.level === "nfl" ? 32 : 25;
      for (const side of [0, 1]) {
        const ids = [...film.players.values()].filter((p) => p.side === side).map((p) => p.id);
        const n = Math.min(q.side, ids.length);
        for (let k = 0; k < n; k++) {
          const pl = film.players.get(ids[k]);
          const kit = kits[side];
          const skin = this.skins.get(pl.skin) || offSkin;
          const man = makeMan({ num: kit.nums.get(pl.num) }, pl, { ...q, detail: 0, shadow: 0 }, { ...kit, skin });
          const x = 10 + area + 3 + (k / Math.max(1, n - 1)) * (100 - 2 * area - 6);
          const y = side === 0 ? FIELD_H + 3 + (k % 3) * 0.9 : -3 - (k % 3) * 0.9;
          man.root.position.set(WX(x + (k % 2) * 0.6), 0, WZ(y));
          man.root.rotation.y = side === 0 ? Math.PI / 2 : -Math.PI / 2;
          man.lineman = false;
          man.seed = k;
          man.id = pl.id;
          S.add(man.root);
          this.side.push(man);
        }
      }
      // The ball.
      this.ball = new T.Mesh(G.ball, keep(new T.MeshStandardMaterial({ color: "#7b3f1d", roughness: 0.55 })));
      this.ball.scale.setScalar(0.16);
      this.ball.castShadow = q.shadow > 0;
      S.add(this.ball);
      // Rings under the carrier, the target and the man you isolated.
      const ringM = (c) => keep(new T.MeshBasicMaterial({ color: c, transparent: true, opacity: 0.9, depthWrite: false, side: T.DoubleSide }));
      this.rings = {};
      for (const [k, c] of [["carrier", "#ffd60a"], ["iso", "#ffffff"], ["sep", "#30d158"]]) {
        const m = new T.Mesh(G.ring, ringM(c));
        m.rotation.x = -Math.PI / 2;
        m.position.y = 0.04;
        m.renderOrder = 4;
        m.visible = false;
        S.add(m);
        this.rings[k] = m;
      }
      this.weather(v);
      this.boardFor = null;
      this.rebuild = false;
      this.renderer.shadowMap.enabled = q.shadow > 0;
    }
    light(v) {
      const S = this.scene;
      const night = v.light === "night", dusk = v.light === "dusk", inside = v.light === "indoors";
      const sky = inside ? "#2c2f35" : night ? "#070b16" : dusk ? "#e39a6a" : v.rain || v.snow ? "#8c96a3" : "#86b6e8";
      const top = inside ? "#1c1e22" : night ? "#02040a" : dusk ? "#3a4a78" : v.rain || v.snow ? "#5d6774" : "#3d7cc9";
      const skyTex = canvasTex(4, 256, (g) => {
        const gr = g.createLinearGradient(0, 0, 0, 256);
        gr.addColorStop(0, top);
        gr.addColorStop(0.55, sky);
        gr.addColorStop(1, night ? "#1a2230" : sky);
        g.fillStyle = gr;
        g.fillRect(0, 0, 4, 256);
      });
      const dome = new T.Mesh(new T.SphereGeometry(900, 24, 12), new T.MeshBasicMaterial({ map: skyTex, side: T.BackSide, fog: false, depthWrite: false }));
      dome.rotation.x = Math.PI;
      dome.renderOrder = -1;
      S.add(dome);
      this.mats.list.push(dome.material);
      S.background = new T.Color(top);
      const hemi = new T.HemisphereLight(night ? "#9fb4ff" : "#dfeaff", night ? "#13210f" : "#2e4a22", inside ? 0.9 : night ? 0.45 : 0.7);
      S.add(hemi);
      // The sun from the southwest by mid-afternoon, low and orange at dusk;
      // at night and indoors the stadium's own lights from high above.
      const sun = new T.DirectionalLight(dusk ? "#ffc58a" : night || inside ? "#fffaf0" : "#fff6e4", night ? 0.95 : inside ? 0.7 : dusk ? 0.85 : 1.05);
      if (night || inside) sun.position.set(-20, 110, 60);
      else if (dusk) sun.position.set(-150, 30, 90);
      else sun.position.set(-90, 95, 70);
      sun.castShadow = this.q.shadow > 0;
      if (sun.castShadow) {
        sun.shadow.mapSize.set(this.q.shadow, this.q.shadow);
        const c = sun.shadow.camera;
        c.left = -40;
        c.right = 40;
        c.top = 40;
        c.bottom = -40;
        c.near = 10;
        c.far = 400;
        sun.shadow.bias = -0.0006;
        sun.shadow.normalBias = 0.02;
      }
      S.add(sun);
      S.add(sun.target);
      this.sun = sun;
      this.sunOff = sun.position.clone();
      if (night || inside) {
        for (const [x, z] of [[-60, -50], [60, -50], [-60, 50], [60, 50]]) {
          const l = new T.DirectionalLight("#fff8ea", 0.22);
          l.position.set(x, 80, z);
          S.add(l);
        }
      }
      S.fog = new T.Fog(top, v.rain || v.snow ? 120 : 260, v.rain || v.snow ? 420 : 900);
      this.night = night;
    }
    weather(v) {
      if (!(v.rain || v.snow)) {
        this.parts = null;
        return;
      }
      const n = this.q.parts;
      const pos = new Float32Array(n * 3);
      const r = rng("weather");
      for (let i = 0; i < n; i++) {
        pos[i * 3] = (r() - 0.5) * 80;
        pos[i * 3 + 1] = r() * 30;
        pos[i * 3 + 2] = (r() - 0.5) * 60;
      }
      const g = new T.BufferGeometry();
      g.setAttribute("position", new T.BufferAttribute(pos, 3));
      const m = new T.PointsMaterial({ color: v.snow ? "#ffffff" : "#b8c6d8", size: v.snow ? 0.18 : 0.06, transparent: true, opacity: v.snow ? 0.9 : 0.55, depthWrite: false });
      this.mats.list.push(m);
      const p = new T.Points(g, m);
      p.frustumCulled = false;
      this.scene.add(p);
      this.parts = { p, pos, snow: !!v.snow, wind: (v.wind || 0) * (v.wind_along || 0) * 0.05 };
    }

    // ---------- each frame
    draw(st, c, film, dt) {
      if (!this.ok) return;
      const manual = this.player.manual;
      const real = performance.now();
      if (!manual && this.q.fps < 60 && real - this.lastDraw < 1000 / this.q.fps - 2) return;
      const frameDt = manual ? dt : Math.min(0.1, (real - (this.lastDraw || real)) / 1000) || dt;
      this.lastDraw = real;
      // The scene's own clock (the crowd, the weather), even when recording.
      this.clock = (this.clock || 0) + frameDt;
      const now = this.clock * 1000;
      if (this.film !== film || this.rebuild || this.next !== SET.next) this.build(film);
      this.resize();
      const p = this.player;
      const next = this.next;
      if (next) {
        if (this.dirtFor !== c.n) this.soil(c, film);
        nextLook(st, c);
      }
      // The men on the field this snap; the rest stand down.
      const on = new Set(c.ids);
      for (const [id, man] of this.men) if (!on.has(id)) man.root.visible = false;
      for (const m of st.men) {
        const id = c.ids[m.i];
        const man = this.men.get(id);
        if (!man) continue;
        man.root.visible = true;
        if (next) {
          man.win = m.win || 0;
          man.reach = m.i === c.catcher && m.act === ACT.catch ? m.reach : null;
          if ((m.act === ACT.fall || (m.act === ACT.catch && man.reach && man.reach.dive)) && man.twistFor !== m.act) {
            man.twistFor = m.act;
            man.twist = m.twist || 0;
          } else if (m.act !== ACT.fall && m.act !== ACT.down && m.act !== ACT.getup && m.act !== ACT.catch) man.twistFor = -1;
        }
        man.root.position.set(WX(next ? m.dx : m.x), 0, WZ(next ? m.dy : m.y));
        pose(man, m, st.t, frameDt);
        if (next) {
          // He goes down the way the hit takes him, his tackler with him,
          // and a pile stacks up rather than sinking into the turf.
          const fall = man.sm ? man.sm.fall : 0;
          man.root.rotation.y = -(m.face + (man.twistFor === -1 ? 0 : man.twist || 0) * Math.min(1, fall * 1.6));
          man.hips.position.y += (m.pile || 0) * fall;
          // His shadow lies along him when he is down.
          const lie = fall * man.dim.H;
          man.blob.position.x = lie * 0.42;
          man.blob.scale.x = 1.3 * man.dim.w + lie * 0.8;
          if (man.core) {
            man.core.position.x = lie * 0.42;
            man.core.scale.x = 0.62 * man.dim.w + lie * 0.75;
          }
        } else man.root.rotation.y = -m.face;
      }
      for (const man of this.side) {
        man.root.visible = !on.has(man.id);
        if (man.root.visible) pose(man, { act: ACT.stand, speed: 0, walked: 0 }, st.t, frameDt);
      }
      // The ball: in his hands when held, else where the engine has it, in
      // its spiral (passes) or end over end (kicks).
      const b = st.ball;
      let bx = b.x, by = b.y, bz = Math.max(0.12, b.z);
      if (b.mode === 0 && b.holder >= 0 && st.men[b.holder]) {
        const h = st.men[b.holder];
        const man = this.men.get(c.ids[h.i]);
        const tuck = h.act === ACT.carry ? 0.28 : h.act === ACT.stance ? 0.45 : 0.32;
        const side = h.act === ACT.carry ? 0.2 : 0;
        const hx = next ? h.dx : h.x, hy = next ? h.dy : h.y;
        bx = hx + Math.cos(h.face) * tuck - Math.sin(h.face) * side;
        by = hy + Math.sin(h.face) * tuck + Math.cos(h.face) * side;
        bz = h.act === ACT.stance ? 0.12 : man ? man.dim.H * (h.act === ACT.throw ? 0.9 : 0.62) : 1.2;
        if (h.act === ACT.down || h.act === ACT.fall) bz = 0.25;
      }
      this.ball.position.set(WX(bx), bz, WZ(by));
      const kick = c.kind === "Kickoff" || c.kind === "Punt" || c.kind === "FieldGoal" || c.kind === "ExtraPoint";
      const Q = this.q3 || (this.q3 = { yaw: new T.Quaternion(), tilt: new T.Quaternion(), spin: new T.Quaternion(), Y: new T.Vector3(0, 1, 0), X: new T.Vector3(1, 0, 0), Z: new T.Vector3(0, 0, 1) });
      if (b.mode === 1 && this.lastBall) {
        // In the air: the nose along the flight, a spiral on a pass, end
        // over end on a kick.
        const vx = bx - this.lastBall.x, vy = by - this.lastBall.y, vz = bz - this.lastBall.z;
        if (Math.hypot(vx, vy) > 1e-4) this.head = Math.atan2(vy, vx);
        // A throw under pressure comes out a wobbling duck on the next look.
        const duck = next && !kick && c.ng && c.ng.pressured;
        this.spin = (this.spin || 0) + frameDt * (kick ? 8 : duck ? 15 : 28);
        const air = c.throwAt != null ? Math.max(0, st.t - c.throwAt) : 0;
        const wob = duck ? 0.32 * Math.sin(air * 8.5) : 0;
        Q.yaw.setFromAxisAngle(Q.Y, -(this.head || 0) + wob);
        Q.tilt.setFromAxisAngle(Q.Z, kick ? this.spin : Math.atan2(vz, Math.hypot(vx, vy) || 1e-4) + (duck ? 0.22 * Math.cos(air * 8.5) : 0));
        Q.spin.setFromAxisAngle(Q.X, kick ? 0 : this.spin);
        this.ball.quaternion.copy(Q.yaw).multiply(Q.tilt).multiply(Q.spin);
      } else if (b.mode === 0 && st.men[b.holder]) {
        Q.yaw.setFromAxisAngle(Q.Y, -st.men[b.holder].face);
        Q.tilt.setFromAxisAngle(Q.Z, 0.35);
        this.ball.quaternion.copy(Q.yaw).multiply(Q.tilt);
      } else if (next && b.mode === 2 && c.looseAt != null) {
        // A loose ball bounces the way a football does, high and crooked
        // and dying away, on its way to the spot where the engine has it.
        const u = Math.max(0, st.t - c.looseAt), fade = Math.exp(-u * 2.2);
        bz = Math.max(bz, 0.12 + 0.6 * fade * Math.abs(Math.sin(u * 8.5)));
        this.ball.position.y = bz;
        Q.tilt.setFromAxisAngle(Q.Z, u * 11 * fade + 0.4);
        Q.spin.setFromAxisAngle(Q.X, u * 7 * fade);
        this.ball.quaternion.copy(Q.tilt).multiply(Q.spin);
      } else this.ball.quaternion.set(0, 0, 0, 1);
      this.lastBall = { x: bx, y: by, z: bz };
      // The lines.
      this.los.visible = c.los != null;
      if (c.los != null) this.los.position.x = WX(c.los);
      this.ltg.visible = c.ltg != null && c.ltg > 10 && c.ltg < 110;
      if (this.ltg.visible) this.ltg.position.x = WX(c.ltg);
      // The crew and the chains: officials face the ball and run with the
      // play; the chain crew holds the rods and the box on the far side.
      for (const o of this.crew) o.root.visible = false;
      for (const rod of this.rods) rod.visible = false;
      let k = 0, r = 0;
      if (SET.officials) {
        for (const o of st.crew) {
          const chain = o.role === "box" || o.role === "rod";
          const j = chain ? (o.role === "box" ? 2 : r++) : -1;
          if (chain && j > 2) continue;
          const man = chain ? this.crew[8 + j] : this.crew[k++];
          if (chain) {
            const rod = this.rods[j];
            rod.visible = true;
            rod.position.set(WX(o.x), 0, WZ(o.y));
          }
          if (!man) continue;
          man.root.visible = true;
          const was = man.lastPos || [o.x, o.y];
          const step = Math.hypot(o.x - was[0], o.y - was[1]);
          man.walked = (man.walked || 0) + step;
          man.lastPos = [o.x, o.y];
          if (chain) {
            man.root.position.set(WX(o.x) - 0.35, 0, WZ(o.y) - 0.45);
            man.root.rotation.y = -Math.PI / 2;
            pose(man, { act: ACT.stand, speed: 0, walked: 0 }, st.t, frameDt);
            continue;
          }
          man.root.position.set(WX(o.x), 0, WZ(o.y));
          man.root.rotation.y = -Math.atan2(b.y - o.y, b.x - o.x);
          const throwing = o.flag && st.t - o.flag.t < 0.6 && st.t >= o.flag.t;
          pose(man, { act: throwing ? ACT.throw : ACT.stand, speed: Math.min(6, step / Math.max(0.016, frameDt)), walked: man.walked }, st.t, frameDt);
        }
      }
      this.chain.visible = r >= 2;
      if (this.chain.visible) {
        const [a1, a2] = [this.rods[0].position.x, this.rods[1].position.x];
        this.chain.scale.x = Math.abs(a2 - a1);
        this.chain.position.set((a1 + a2) / 2, 0.05, this.rods[0].position.z);
      }
      // Rings: the carrier, the man you isolated, the target's separation.
      const ring = (key, m, rad, color) => {
        const o = this.rings[key];
        o.visible = !!m;
        if (!m) return;
        o.position.x = WX(m.x);
        o.position.z = WZ(m.y);
        o.scale.setScalar(rad);
        if (color) o.material.color.set(color);
      };
      ring("carrier", b.mode === 0 && b.holder >= 0 && st.t > 0 ? st.men[b.holder] : null, 1.0);
      ring("iso", p.isolate != null ? st.men[p.isolate] : null, 1.5);
      const ti = c.target != null ? c.ids.indexOf(c.target) : -1;
      if (SET.sep && ti >= 0 && st.t > 0 && !(c.catchAt != null && st.t > c.catchAt + 0.4)) {
        let d = 99;
        const m = st.men[ti];
        for (const o of st.men) if (c.side[o.i] !== c.side[ti]) d = Math.min(d, Math.hypot(o.x - m.x, o.y - m.y));
        st.sepNow = d;
        ring("sep", m, Math.min(8, Math.max(0.6, d)), d > 2.5 ? "#30d158" : d > 1.2 ? "#ffd60a" : "#ff375f");
      } else this.rings.sep.visible = false;
      // The overlay: drawings and trails, redrawn a few times a second.
      if (now - (this.ovAt || 0) > (p.drawing ? 33 : 90) || this.ovN !== c.n || manual) {
        this.ovAt = now;
        this.ovN = c.n;
        this.overlay(st, c, film);
      }
      this.cheer(st, c, film, now);
      this.scoreboard(c, film);
      this.moveWeather(frameDt);
      this.cameraFor(st, c, frameDt);
      // The sun's shadow follows the play.
      if (this.sun.castShadow) {
        this.sun.target.position.set(this.camAt.x, 0, 0);
        this.sun.position.copy(this.sunOff).add(this.sun.target.position);
      }
      this.renderer.render(this.scene, this.camera);
      this.label(st, c, film);
      // On Automatic, a computer that can't keep up steps down a level.
      if (SET.quality === "auto" && this.qName !== "low" && !manual) {
        this.slow = frameDt > 0.045 ? this.slow + frameDt : Math.max(0, this.slow - frameDt * 0.5);
        if (this.slow > 2.5) {
          this.qName = this.qName === "high" ? "medium" : "low";
          this.q = QUAL[this.qName];
          this.rebuild = true;
          this.slow = 0;
        }
      }
    }
    overlay(st, c, film) {
      const g = this.ovCanvas.getContext("2d");
      const px = this.ovPx;
      g.setTransform(1, 0, 0, 1, 0, 0);
      g.clearRect(0, 0, this.ovCanvas.width, this.ovCanvas.height);
      g.setTransform(px, 0, 0, px, MARGIN_X * px, MARGIN_Y * px);
      const ys = 1.4 / px;
      const p = this.player;
      // An empty sheet is hidden rather than uploaded: a cleared canvas can
      // reach the GPU late and leave the last snap's trails on the turf.
      const marks = SET.marks && (p.drawing || p.marks.some((m) => m.play === c.n));
      const trails = SET.trails && st.t > 0;
      this.ovM.visible = !!(marks || trails);
      if (!this.ovM.visible) return;
      p.drawMarks(g, ys, st.t);
      if (trails) {
        for (let i = 0; i < c.men; i++) {
          const id = c.ids[i];
          const pl = film.players.get(id);
          if (!pl) continue;
          const elig = id === c.target || id === c.carrier || id === c.passer;
          const skill = ["WR", "TE", "RB"].includes(pl.pos) && c.side[i] === c.team;
          if (!skill && !elig) continue;
          g.strokeStyle = id === c.target ? "rgba(255,214,10,.95)" : id === c.carrier ? "rgba(255,255,255,.9)" : "rgba(255,255,255,.45)";
          g.lineWidth = id === c.target ? 0.32 : 0.22;
          g.lineJoin = g.lineCap = "round";
          g.beginPath();
          let started = false;
          for (let k = 0; k < c.frames && c.t[k] <= st.t; k++) {
            if (c.t[k] < 0) continue;
            const o = k * c.S + 6 + i * 5;
            if (!started) {
              g.moveTo(c.f[o], c.f[o + 1]);
              started = true;
            } else g.lineTo(c.f[o], c.f[o + 1]);
          }
          if (started) g.lineTo(st.men[i].x, st.men[i].y);
          g.stroke();
        }
      }
      this.ovTex.needsUpdate = true;
    }
    // The next look's dirt: uniforms pick it up as the game goes, the
    // linemen first and the kickers hardly at all, mud on wet grass twice as
    // fast, the black crumbs of artificial turf far slower.
    soil(c, film) {
      this.dirtFor = c.n;
      const v = film.ix.venue || {};
      const i = film.byN.get(c.n) || 0;
      const progress = film.ix.plays.length > 1 ? i / (film.ix.plays.length - 1) : 0;
      const turf = v.surface === "turf";
      const wet = v.rain ? 1.9 : v.snow ? 1.3 : 1;
      const dirt = new T.Color(turf ? "#2b2b2b" : v.rain ? "#4a3a26" : "#5d4c33");
      const SHARE = [1, 0.85, 0.6, 0.4, 0.12];
      for (const d of this.dirt) {
        const amount = clamp(progress * SHARE[d.g] * (turf ? 0.25 : 1) * wet * 0.34 * d.part, 0, 0.6);
        d.mat.color.copy(d.base).lerp(dirt, amount);
      }
    }
    // The home crowd stands and bounces when its team scores or takes the
    // ball away; the visitors' corner when theirs does.
    cheer(st, c, film, now) {
      const m = this.mats.crowd;
      if (!m) return;
      m.uniforms.time.value = now / 1000;
      const p = film.play(c.n);
      let h = 0, a = 0;
      if (p) {
        const res = p.tags.result;
        const late = st.t > (c.catchAt != null ? c.catchAt : c.t1 - 2.2);
        if (late && (res === "touchdown" || res === "turnover" || res === "explosive")) {
          const good = res === "turnover" ? 1 - p.off : p.off;
          if (good === 0) h = res === "explosive" ? 0.6 : 1;
          else a = res === "explosive" ? 0.6 : 1;
        }
        // On the next look a crowd also gets up for its defense on third
        // and fourth down, loudest before the snap.
        if (this.next && p.down >= 3 && st.t < 0.6) {
          const loud = p.down === 4 ? 0.8 : 0.5;
          if (p.off === 1) h = Math.max(h, loud);
          else a = Math.max(a, loud * 0.6);
        }
      }
      const u = m.uniforms.cheer.value;
      u.x += (h - u.x) * 0.06;
      u.y += (a - u.y) * 0.06;
      m.uniforms.scale.value = this.h * this.dpr * 0.5;
    }
    // The video board: the score, the quarter, the clock, the down.
    scoreboard(c, film) {
      const p = film.play(c.n);
      const board = this.mats.boardMesh;
      if (!board || !p) return;
      const key = `${p.score[0]}-${p.score[1]}-${p.q}-${p.clock}-${p.down}`;
      if (key === this.boardFor) return;
      this.boardFor = key;
      const ix = film.ix;
      const tex = canvasTex(512, 160, (g) => {
        g.fillStyle = "#06070a";
        g.fillRect(0, 0, 512, 160);
        g.textBaseline = "middle";
        g.font = '800 34px Archivo, Arial, sans-serif';
        [1, 0].forEach((s, k) => {
          const t = ix.teams[s];
          g.fillStyle = t.colors[0];
          g.fillRect(16, 18 + k * 64, 10, 50);
          g.fillStyle = "#fff";
          g.textAlign = "left";
          g.fillText(String(t.short || t.name).slice(0, 12).toUpperCase(), 38, 44 + k * 64);
          g.textAlign = "right";
          g.font = '900 46px Archivo, Arial, sans-serif';
          g.fillText(String(p.score[s]), 360, 44 + k * 64);
          g.font = '800 34px Archivo, Arial, sans-serif';
        });
        g.textAlign = "center";
        g.fillStyle = "#ffd60a";
        g.fillText(p.q > 4 ? "OT" : "Q" + p.q, 440, 44);
        g.fillStyle = "#fff";
        g.fillText(p.clock, 440, 108);
      });
      if (board.material.emissiveMap) board.material.emissiveMap.dispose();
      board.material.emissiveMap = tex;
      board.material.emissive.set("#ffffff");
      board.material.color.set("#000000");
      board.material.needsUpdate = true;
    }
    moveWeather(dt) {
      const w = this.parts;
      if (!w) return;
      const pos = w.pos;
      const fall = w.snow ? 1.6 : 14;
      for (let i = 0; i < pos.length; i += 3) {
        pos[i + 1] -= fall * dt;
        pos[i] += (w.wind + (w.snow ? Math.sin(pos[i + 1] + i) * 0.4 : 0)) * dt;
        if (pos[i + 1] < 0) pos[i + 1] += 30;
      }
      w.p.geometry.attributes.position.needsUpdate = true;
      w.p.position.set(this.camAt.x, 0, 0);
    }

    // ---------- cameras
    cameraFor(st, c, dt) {
      const p = this.player;
      const dir = c.dir || 1;
      const ball = st.ball;
      const mode = p.isolate != null ? "iso" : p.replay ? "replay" : SET.cam3d;
      let pos, at, fov;
      const aspect = this.camera.aspect || 1.7;
      const fovFor = (w, d) => clamp((2 * Math.atan(Math.tan(Math.atan(w / 2 / d)) / aspect) * 180) / Math.PI, 8, 70);
      let lo = ball.x, hi = ball.x;
      for (const m of st.men) {
        lo = Math.min(lo, m.x);
        hi = Math.max(hi, m.x);
      }
      if (mode === "free") {
        const o = this.orbit;
        if (!o.set) {
          o.at.set(WX(ball.x), 0, WZ(ball.y));
          o.set = true;
        }
        pos = new T.Vector3(o.at.x + o.r * Math.sin(o.ph) * Math.sin(o.th), o.r * Math.cos(o.ph), o.at.z + o.r * Math.sin(o.ph) * Math.cos(o.th));
        at = o.at.clone();
        fov = 40;
      } else if (mode === "iso") {
        const m = st.men[p.isolate] || ball;
        pos = new T.Vector3(WX(m.x - dir * 9), 5.5, WZ(m.y) + 6);
        at = new T.Vector3(WX(m.x + dir * 2), 1, WZ(m.y));
        fov = 42;
      } else if (mode === "replay") {
        // From high behind the end zone the offense attacks, tight on the
        // ball, a few yards off the middle so the posts stay out of it.
        const ex = dir > 0 ? FIELD_W + 14 : -14;
        pos = new T.Vector3(WX(ex), 13, WZ(FIELD_H / 2 + (ball.y - FIELD_H / 2) * 0.35 + 9));
        at = new T.Vector3(WX(ball.x), Math.max(0.8, ball.z * 0.8), WZ(ball.y));
        fov = clamp(fovFor(22, Math.abs(ex - ball.x) + 6), 10, 40);
      } else if (mode === "all22") {
        // High from the press box side, all 22 men inside the frame.
        const cx = (lo + hi) / 2;
        const span = Math.max(42, hi - lo + 14);
        const d = 92;
        pos = new T.Vector3(WX(cx), 52, WZ(FIELD_H / 2) + 78);
        at = new T.Vector3(WX(cx), 0, 2);
        fov = fovFor(span, d + 10);
      } else if (mode === "broadcast") {
        // The TV angle: lower, closer, a step behind the line, leading the ball.
        const lead = ball.mode === 1 ? 0 : dir * 6;
        pos = new T.Vector3(WX(ball.x - dir * 6), 17, WZ(FIELD_H) + 44);
        at = new T.Vector3(WX(ball.x + lead), 0.5, WZ(clamp(ball.y, 10, FIELD_H - 6)));
        const deep = Math.abs(hi - lo) > 40 && ball.mode === 1;
        fov = fovFor(deep ? 54 : 38, 60);
      } else {
        // The coaches' end-zone copy: high behind the offense, fixed in
        // place, panning and zooming to keep the play in.
        // Across the screen is across the field: frame the men's width,
        // the formation's splits and the deepest safety.
        const ex = dir > 0 ? -22 : FIELD_W + 22;
        const los = c.los != null ? c.los : ball.x;
        let ylo = ball.y, yhi = ball.y;
        for (const m of st.men) {
          if (Math.abs(m.x - ball.x) > 28) continue;
          ylo = Math.min(ylo, m.y);
          yhi = Math.max(yhi, m.y);
        }
        const reach = Math.max(14, (dir > 0 ? hi - los : los - lo) + 6);
        const mid = los + dir * Math.min(reach * 0.4, 24);
        const across = clamp(yhi - ylo + 10, 30, FIELD_H + 4);
        pos = new T.Vector3(WX(ex), 26, WZ(lerp(FIELD_H / 2, (ylo + yhi) / 2, 0.3)));
        at = new T.Vector3(WX(mid), 0, WZ(lerp(FIELD_H / 2, (ylo + yhi) / 2, 0.6)));
        fov = fovFor(across, Math.abs(mid - ex));
      }
      // The next look's broadcast camera has a man behind it: he sees the
      // play change a beat late and swings after it, overshooting a little.
      if (this.next && mode === "broadcast") {
        const L = this.lag;
        if (!L || this.snapCam || L.mode !== mode) this.lag = { mode, buf: [], x: at.clone(), v: new T.Vector3() };
        else {
          L.buf.push({ c: this.clock, at: at.clone() });
          while (L.buf.length > 1 && L.buf[1].c <= this.clock - 0.2) L.buf.shift();
          const want = L.buf[0].at;
          const om = 4.2, ze = 0.72;
          for (let left = Math.min(dt, 0.25); left > 1e-4; left -= 1 / 120) {
            const h = Math.min(left, 1 / 120);
            L.v.x += ((want.x - L.x.x) * om * om - 2 * ze * om * L.v.x) * h;
            L.v.z += ((want.z - L.x.z) * om * om - 2 * ze * om * L.v.z) * h;
            L.x.x += L.v.x * h;
            L.x.z += L.v.z * h;
          }
          pos.x += L.x.x - at.x;
          at.x = L.x.x;
          at.z = L.x.z;
        }
      } else this.lag = null;
      const k = this.snapCam ? 1 : this.next && mode === "broadcast" ? 1 - Math.pow(0.2, dt) : 1 - Math.pow(0.03, dt);
      this.snapCam = false;
      this.camPos.lerp(pos, k);
      this.camAt.lerp(at, k);
      this.fov += (fov - this.fov) * k;
      this.camera.position.copy(this.camPos);
      this.camera.fov = this.fov;
      this.camera.updateProjectionMatrix();
      this.camera.lookAt(this.camAt);
    }

    // ---------- the mouse: pick a man, draw on the turf, orbit when free
    toField(e) {
      const r = this.cv.getBoundingClientRect();
      const nx = ((e.clientX - r.left) / r.width) * 2 - 1, ny = -((e.clientY - r.top) / r.height) * 2 + 1;
      this.ray.setFromCamera({ x: nx, y: ny }, this.camera);
      const o = this.ray.ray.origin, d = this.ray.ray.direction;
      if (Math.abs(d.y) < 1e-4) return null;
      const s = -o.y / d.y;
      if (s < 0) return null;
      return [o.x + d.x * s + FIELD_W / 2, o.z + d.z * s + FIELD_H / 2];
    }
    wire() {
      const cv = this.cv;
      let drag = null;
      cv.addEventListener("pointerdown", (e) => {
        const p = this.player;
        if (p.tool) {
          const pt = this.toField(e);
          if (pt) p.drawStart(pt, e);
          return;
        }
        drag = { x: e.clientX, y: e.clientY, moved: false, th: this.orbit.th, ph: this.orbit.ph, at: this.orbit.at.clone(), pan: e.shiftKey || e.button === 2 };
        cv.setPointerCapture(e.pointerId);
      });
      cv.addEventListener("pointermove", (e) => {
        const p = this.player;
        if (p.tool) {
          if (p.drawing) {
            const pt = this.toField(e);
            if (pt) p.drawMove(pt);
          }
          return;
        }
        if (!drag) return;
        const dx = e.clientX - drag.x, dy = e.clientY - drag.y;
        if (Math.abs(dx) + Math.abs(dy) > 4) drag.moved = true;
        if (!drag.moved) return;
        if (SET.cam3d !== "free") {
          // Dragging from a fixed camera starts the free one where it is.
          const o = this.orbit;
          const rel = this.camPos.clone().sub(this.camAt);
          o.r = rel.length();
          o.ph = Math.acos(clamp(rel.y / o.r, -1, 1));
          o.th = Math.atan2(rel.x, rel.z);
          o.at.copy(this.camAt);
          o.set = true;
          drag.th = o.th;
          drag.ph = o.ph;
          drag.at = o.at.clone();
          SET.cam3d = "free";
          p.syncBar();
        }
        const o = this.orbit;
        if (drag.pan) {
          const s = o.r / 400;
          o.at.set(drag.at.x - dx * s * Math.cos(o.th) - dy * s * Math.sin(o.th), 0, drag.at.z + dx * s * Math.sin(o.th) - dy * s * Math.cos(o.th));
        } else {
          o.th = drag.th - dx * 0.006;
          o.ph = clamp(drag.ph - dy * 0.005, 0.12, 1.45);
        }
      });
      cv.addEventListener("pointerup", (e) => {
        const p = this.player;
        if (p.tool) return p.drawEnd();
        if (drag && !drag.moved) this.pickAt(e);
        drag = null;
      });
      cv.addEventListener("contextmenu", (e) => e.preventDefault());
      cv.addEventListener(
        "wheel",
        (e) => {
          e.preventDefault();
          const o = this.orbit;
          if (SET.cam3d !== "free") {
            const rel = this.camPos.clone().sub(this.camAt);
            o.r = rel.length();
            o.ph = Math.acos(clamp(rel.y / o.r, -1, 1));
            o.th = Math.atan2(rel.x, rel.z);
            o.at.copy(this.camAt);
            o.set = true;
            SET.cam3d = "free";
            this.player.syncBar();
          }
          o.r = clamp(o.r * Math.exp(e.deltaY * 0.0012), 6, 260);
        },
        { passive: false },
      );
      cv.addEventListener("pointerleave", () => (this.hover = null));
      cv.addEventListener("mousemove", (e) => {
        if (this.player.tool) return;
        const now = performance.now();
        if (now - (this.hoverAt || 0) < 60) return;
        this.hoverAt = now;
        this.hover = this.hitTest(e);
        cv.style.cursor = this.hover != null ? "pointer" : "grab";
      });
    }
    hitTest(e) {
      const r = this.cv.getBoundingClientRect();
      this.ray.setFromCamera({ x: ((e.clientX - r.left) / r.width) * 2 - 1, y: -((e.clientY - r.top) / r.height) * 2 + 1 }, this.camera);
      const hits = [...this.men.values()].filter((m) => m.root.visible).map((m) => m.hit);
      const x = this.ray.intersectObjects(hits, false)[0];
      if (!x) return null;
      const c = this.player.clip;
      const i = c ? c.ids.indexOf(x.object.userData.id) : -1;
      return i >= 0 ? i : null;
    }
    pickAt(e) {
      const i = this.hitTest(e);
      if (i != null) this.player.pick(i);
    }
    // Names over heads: the man you picked or are hovering on.
    label(st, c, film) {
      const p = this.player;
      const i = p.isolate != null ? p.isolate : this.hover;
      if (i == null || !st.men[i]) {
        if (this.tags.innerHTML) this.tags.innerHTML = "";
        return;
      }
      const m = st.men[i];
      const man = this.men.get(c.ids[i]);
      const pl = film.players.get(c.ids[i]);
      if (!man || !pl) return;
      const v = new T.Vector3(WX(m.x), man.dim.H + 0.5, WZ(m.y)).project(this.camera);
      const x = ((v.x + 1) / 2) * this.w, y = ((1 - v.y) / 2) * this.h;
      const html = `<div class="vw3-tag" style="transform:translate(${x.toFixed(0)}px,${y.toFixed(0)}px)">#${pl.num} ${esc(pl.name)} · ${esc(pl.pos)}</div>`;
      this.tags.innerHTML = html;
    }

    disposeScene() {
      if (this.mats) for (const m of this.mats.list) {
        if (m.map) m.map.dispose();
        if (m.emissiveMap) m.emissiveMap.dispose();
        m.dispose();
      }
      if (this.fieldTex) this.fieldTex.dispose();
      if (this.ovTex) this.ovTex.dispose();
      if (this.stadium) this.stadium.traverse((o) => {
        if (o.geometry && !Object.values(GEO || {}).includes(o.geometry)) o.geometry.dispose();
        if (o.material) [].concat(o.material).forEach((mm) => {
          if (mm.map) mm.map.dispose();
          mm.dispose();
        });
      });
      if (this.parts) this.parts.p.geometry.dispose();
    }
    dispose() {
      if (!this.ok) return;
      this.disposeScene();
      this.renderer.dispose();
      this.renderer.forceContextLoss && this.renderer.forceContextLoss();
      this.cv.remove();
      this.tags.remove();
      this.ok = false;
    }
  }

  window.AFMViews3D = { View3D, autoQuality, QUAL };
})();
