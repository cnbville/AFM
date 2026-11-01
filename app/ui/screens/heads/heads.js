// AFM drawn heads (faces F14). Built by tools/faces/heads/build.py from heads.src.js; edit those, not this file.
// The face shapes are Google's GNM Head model (Apache 2.0) on MakeHuman's base mesh (CC0).
// The drawn heads (faces F14, docs/design/faces.md): the game draws each player's head itself from his face recipe, in the
// page, with three.js. No Blender, no photo model. The head is Google's GNM Head face (real scans, Apache 2.0) on
// MakeHuman's base head and body (CC0); the face shape comes from his heritage group's sampler and his own seed; hair and
// beards are the v7 grooms, baked once on the average head and regrown on his (tools/faces/heads/bake_lib.py); the light is
// the one Cain approved for the v7 ten (2026-10-19), with the AgX "Medium High Contrast" look. The data loads the first time
// a face is drawn. Built from tools/faces/heads/heads.src.js by tools/faces/heads/build.py (do not edit heads.js by hand).
//
// AFMHeads.draw(spec) -> Promise of a WebP data URL (480 x 576, transparent background). `spec` is what the game's
// `face`/`faces` commands send (genes, looks, expression, team colours); a talk adds the expression's strength and a view
// ({yaw, down} in degrees, dialogue D1). AFMHeads.drawRecipe takes a head recipe directly.
"use strict";
(() => {
  const BASE = "screens/heads/";
  const W0 = 480, H0 = 576, SS = 2;            // the picture, and the supersampling it is drawn at
  let ready = null, E = null;
  const loadScript = (src) => new Promise((res, rej) => {
    const s = document.createElement("script"); s.src = src; s.onload = res; s.onerror = () => rej(new Error("missing " + src)); document.head.appendChild(s);
  });
  const get = async (u, bin) => { const r = await fetch(BASE + u); if (!r.ok) throw new Error("missing " + u); return bin ? r.arrayBuffer() : r.json(); };
  function init() {
    ready ||= (async () => {
      if (!window.THREE) await loadScript("screens/views/three.min.js");
      const [hm, hb, sm, sb, jm, gm, gb] = await Promise.all([get("head.json"), get("head.bin", 1), get("shape.json"), get("shape.bin", 1), get("jersey.json"), get("groom.json"), get("groom.bin", 1)]);
      E = engine(hm, hb, sm, sb, jm, gm, gb);
    })();
    ready.catch(() => { ready = null; });
    return ready;
  }

  // ---------------------------------------------------------------- small helpers
  const fract = (x) => x - Math.floor(x), ss = (a, b, x) => { const t = Math.max(0, Math.min(1, (x - a) / (b - a))); return t * t * (3 - 2 * t); };
  function h3(x, y, z) { x = fract(x * .3183099 + .1) * 17; y = fract(y * .3183099 + .1) * 17; z = fract(z * .3183099 + .1) * 17; return fract(x * y * z * (x + y + z)); }
  function n3(x, y, z) {
    const ix = Math.floor(x), iy = Math.floor(y), iz = Math.floor(z); let fx = x - ix, fy = y - iy, fz = z - iz; fx = fx * fx * (3 - 2 * fx); fy = fy * fy * (3 - 2 * fy); fz = fz * fz * (3 - 2 * fz);
    const L = (a, b, t) => a + (b - a) * t, H = (a, b, c) => h3(ix + a, iy + b, iz + c);
    return L(L(L(H(0, 0, 0), H(1, 0, 0), fx), L(H(0, 1, 0), H(1, 1, 0), fx), fy), L(L(H(0, 0, 1), H(1, 0, 1), fx), L(H(0, 1, 1), H(1, 1, 1), fx), fy), fz);
  }
  function rng(seed) { let a = seed >>> 0; return () => { a |= 0; a = a + 0x6D2B79F5 | 0; let t = Math.imul(a ^ a >>> 15, 1 | a); t = t + Math.imul(t ^ t >>> 7, 61 | t) ^ t; return ((t ^ t >>> 14) >>> 0) / 4294967296; }; }
  const gauss = (R) => { let u = 0, v = 0; while (u === 0) u = R(); v = R(); return Math.sqrt(-2 * Math.log(u)) * Math.cos(2 * Math.PI * v); };
  const hexRGB = (h) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16) / 255);
  // CIELAB (D65) to linear sRGB (render_v7.py lab_lin)
  function labLin([L, A, B]) {
    const fy = (L + 16) / 116, fx = fy + A / 500, fz = fy - B / 200, f = (t) => t ** 3 > .008856 ? t ** 3 : (t - 16 / 116) / 7.787;
    const X = .95047 * f(fx), Y = f(fy), Z = 1.08883 * f(fz), c = (v) => Math.max(0, Math.min(1, v));
    return [c(3.2406 * X - 1.5372 * Y - .4986 * Z), c(-.9689 * X + 1.8758 * Y + .0415 * Z), c(.0557 * X - .2040 * Y + 1.0570 * Z)];
  }
  // v7's per-group skin shift (instrument to photo, under this light) and lip-minus-cheek gap (measured in photos), both in Lab
  const SKSHIFT = { white: [0, 3, 0], east_asian: [0, 3, 0], southeast_asian: [0, 3, 0], south_asian: [-2, 1, 0], hispanic: [0, 3, 0], polynesian: [-6, 2, 0], black: [-9, -1, -1] };
  const LIPD = { white: [-1.2, 6.1, -4.0], black: [4.6, -.5, -8.9], hispanic: [-1.7, 6.1, -6.6], east_asian: [-2.4, 7.0, -5.0], south_asian: [-1.8, .8, -11.6],
    southeast_asian: [-2.3, 3.7, -6.8], polynesian: [-1.7, 6.1, -6.6] };
  // the hair line (groom.py hair_mask with groom_v5's front shift) and the beard area (groom.py beard_mask), in the base head's space
  function hairMask(x, y, z, rec, hl0) {
    const ax = Math.abs(x), front = ss(.2, .9, z), hy = 7.95 + hl0 * front + rec * (.10 + .22 * ss(.08, .42, ax) * ss(.75, .2, ax)), hl = 6.92 + (hy - 6.92) * ss(.05, .95, z);
    const ear = ss(.62, .72, ax) * ss(7.62, 7.5, y) * ss(-.15, .05, z) * ss(.75, .55, z), side = ss(.55, .66, ax) * ss(.95, .7, z) * ss(7.75, 7.55, y), e = .012;
    let m = ss(hl - e * .5, hl + e, y); m = Math.max(m, side * ss(7.05, 7.05 + e * 2, y) * ss(.25, .45, z)); return m * (1 - ear) * ss(6.0, 6.3, y);
  }
  function beardMask(x, y, z, G) {
    const ax = Math.abs(x), up = 6.86 + (.20 + .14 * G.cheek) * Math.pow(ss(.2, .7, ax), 1.7), lo = 6.28 + .14 * G.neck;
    let m = (1 - ss(up - .05, up + .04, y)) * ss(lo, lo + .16, y + z * .08);
    m *= 1 - ss(.66, .74, ax) * ss(6.95, 7.05, y);
    if (G.patch > .01) { const pt = n3(x * 9 + 3, y * 9 + 3, z * 9 + 3); m *= 1 + (ss(G.patch - .12, G.patch + .12, pt) - 1) * ss(.22, .4, ax); }
    return m;
  }

  // ---------------------------------------------------------------- a player's head recipe from the game's face spec
  // genes (born with) and look (this season) as `world/looks.rs` serializes them. The face shape follows newplayer_v4.py:
  // GNM's identity decoder fed his group's blend, plus the group's measured shift and the widened spread, all from his own
  // face seed; the traits GNM's recipe has and the game's genes don't (brow shape, hairline, lip tone) come from the same seed.
  const HAIR = { shaved: "shaved", buzz: "buzz", crop: "crop", crop_fade: "crop_fade", side_part: "side_part", flow: "flow", long: "long", man_bun: "man_bun",
    mullet: "mullet", two_block: "two_block", curly_top: "curly_top", waves: "waves", sponge_top: "sponge_top", high_top: "high_top", afro: "afro",
    cornrows: "cornrows", braids: "braids", twists: "twists", locs: "locs" };
  // a straight-hair cut on coily hair is cut the way barbers cut type 4 hair
  const COILY = { crop: "sponge_top", crop_fade: "sponge_top", side_part: "sponge_top", two_block: "sponge_top", curly_top: "sponge_top", flow: "afro", long: "afro", mullet: "afro", man_bun: "afro" };
  // dyes over the whole head, and tips (the outer half of each strand)
  const DYE = { bleached: "#d8c39a", honey: "#a8743a", copper: "#9a4a24" }, TIPS = { blond_tips: "#c9a46a" };
  const EXPR = { smile: ["smile", 1.6], slight_smile: ["slight_smile", 1.5], game_face: ["focused", 1.3], tight_smile: ["tight_smile", 1.5], smirk: ["smirk", 2.0] };
  // skin (render_v7.py, v5_upgrade.py): the game's skin colour gives his place in his group's range (world/looks.rs SKIN, light
  // end first), read as the same place in the group's measured cheek colour (spectrophotometer, D65: Xiao et al. 2017, Everett
  // et al. 2012); the renderer then shifts it per group so the drawn cheek matches photos. South Asian has no measured set here:
  // an estimate between Hispanic and Black (proposed, not confirmed).
  const SKIN_RANGE = { white: [0xf0c8ae, 0xd29a78], black: [0x9a6646, 0x4a2c1e], hispanic: [0xd8a47c, 0xa8734f], polynesian: [0xb88159, 0x7a5034],
    east_asian: [0xecc6a2, 0xc8996f], southeast_asian: [0xddaa80, 0xa87450], south_asian: [0xc99468, 0x5e3a24] };
  const LAB = { white: [[59.6, 5.5], [11.8, 3.1], [14.6, 2.6]], east_asian: [[58.9, 3.1], [11.4, 2.1], [14.2, 1.5]], southeast_asian: [[60.7, 4.0], [10.5, 2.3], [17.2, 2.1]],
    hispanic: [[56.6, 5.0], [11.8, 2.8], [17.1, 2.4]], black: [[41.9, 7.0], [14.1, 2.5], [17.3, 2.3]], polynesian: [[49.0, 5.0], [13.0, 2.5], [18.0, 2.3]],
    south_asian: [[50.0, 6.0], [12.5, 2.5], [17.5, 2.3]] };
  const probit = (q) => { q = Math.min(.99, Math.max(.01, q)); return 4.91 * (Math.pow(q, .14) - Math.pow(1 - q, .14)); };
  function skinLab(hex, her, mixed) {
    const c = hexRGB(hex), [lo, hi] = SKIN_RANGE[her].map((v) => [(v >> 16) / 255, (v >> 8 & 255) / 255, (v & 255) / 255]);
    let num = 0, den = 0; for (let i = 0; i < 3; i++) { num += (c[i] - lo[i]) * (hi[i] - lo[i]); den += (hi[i] - lo[i]) ** 2; }
    const z = probit(1 - Math.min(1, Math.max(0, num / den))), at = (h) => LAB[h].map(([m, sd]) => m + sd * z);
    let L = at(her); if (mixed && LAB[mixed[0]]) { const o = at(mixed[0]); L = L.map((v, i) => v + (o[i] - v) * mixed[1]); }
    return L;
  }
  function recipe(spec) {
    const gn = spec.genes, lk = spec.look, her = gn.heritage, S = E.shape;
    const R = rng(gn.face >>> 0), nr = () => gauss(R);
    // face shape
    const lab = new Float32Array(6); lab[1] = 1;
    const mixes = [[her, 1]]; if (gn.mixed) { mixes[0][1] = 1 - gn.mixed[1]; mixes.push([gn.mixed[0], gn.mixed[1]]); }
    for (const [h, wgt] of mixes) { const bl = S.groups[h].blend, tot = Object.values(bl).reduce((a, b) => a + b, 0); S.classes.forEach((c, i) => { lab[2 + i] += wgt * (bl[c] || 0) / tot; }); }
    const z = new Float32Array(64); for (let i = 0; i < 64; i++) z[i] = nr() * S.T;
    const dec = E.decode(z, lab), A = S.groups[her].A, nA = S.sd.length, noise = Array.from({ length: nA }, nr);
    const c = new Float32Array(S.comp.length), g = { "gnm/gnm-base": 1 };
    for (let k = 0; k < S.comp.length; k++) {
      let sh = 0; for (const [h, wgt] of mixes) sh += wgt * S.groups[h].shift[k];
      let a = 0; for (let m = 0; m < nA; m++) a += A[k][m] * noise[m];
      c[k] = dec[S.comp[k]] + sh + a;
      const v = c[k] - S.cw[k], nm = k < 170 ? `head_${String(k).padStart(3, "0")}` : `eyes_${String(k - 170).padStart(3, "0")}`;
      if (Math.abs(v) > 1e-3) g["gnm/gnm-" + nm] = v;
    }
    let fu = 0; const fm = S.groups[her].fat_mean; for (let k = 0; k < c.length; k++) fu += (c[k] - fm[k]) * S.fat_a[k];
    // a talk's tell (dialogue D1) draws the expression stronger or fainter than the team photo's
    const ex = EXPR[spec.expression] || EXPR.slight_smile; g["gnm/gnm-expr_" + ex[0]] = ex[1] * (spec.strength ?? 1);
    // traits from the seed (newplayer_v4.py's draws)
    const browtail = her === "east_asian" ? -.012 : 0;
    const brow = [nr() * .012 - .004, 1 + .25 * nr(), 1 + .12 * nr(), Math.max(.8, 1.05 + .2 * nr()), .015 * nr(), .08 * nr() + browtail * 2, Math.min(1.3, Math.max(.5, .95 + .18 * nr())), .01 * nr()];
    const dark = gn.darkness;
    // hair and beard
    const coily = gn.texture === "coily";
    let hair = HAIR[lk.hair] || "crop"; if (coily && COILY[hair]) hair = COILY[hair];
    if (!coily && (hair === "sponge_top" || hair === "high_top" || hair === "waves")) hair = "crop";
    // the baked long and mullet grooms stand out from the head (the bake has no gravity; neither was in the approved ten):
    // long hair is drawn as the flow at its longest, a mullet from the front as a textured crop
    if (hair === "long") hair = "flow";
    if (hair === "mullet") hair = "crop";
    if (hair === "flow" && gn.texture === "curly") hair = "flow_curly";
    const lenIn = lk.hair_len || 0, base = { flow: 5, flow_curly: 5, long: 9, mullet: 5, man_bun: 9, locs: 6, braids: 7, twists: 2, afro: 3, crop: 1.2, side_part: 2, two_block: 2, curly_top: 2.5, crop_fade: 1.2 }[hair];
    // long hair is drawn at most a tenth (the flow) to a third longer than its baked groom: the strands are regrown without
    // collisions, so much longer ones part over the scalp and fly out from the head
    const hairScale = base && lenIn > 0 ? Math.max(.55, Math.min(/flow/.test(hair) ? 1.1 : /locs|braids|twists|man_bun|afro/.test(hair) ? 1.3 : 1.8, lenIn / base)) : 1;
    // a circle beard is a goatee joined to the moustache: the baked goatee is that shape
    const bk = lk.beard === "clean" || lk.beard === "none" || !lk.beard ? null : lk.beard === "circle" ? "goatee" : lk.beard;
    const blen = { stubble: .025, moustache: .07, mous_chin: .06, goatee: .11, chin_strap: .05, full_short: .06, full: .15, full_long: .30 }[bk];
    const beardScale = bk && blen ? Math.max(.5, Math.min(2, (lk.beard_len || 0) * .254 / blen || 1)) : 1;
    let hc = gn.hair_rgb; const dy = DYE[lk.dye]; if (dy) hc = dy;
    const grey = lk.dye === "grey_cover" ? 0 : lk.grey || 0;
    const tip = lk.dye === "team_tips" ? (spec.colors || [])[1] || null : TIPS[lk.dye] || null;
    const rec = Math.max(0, Math.min(1.2, ((lk.norwood || 1) - 1) / 3));
    const w = lk.weight || 220, hin = lk.height || 74;
    return {
      age: lk.age || 21, g, her, skin: gn.skin, skinLab: skinLab(gn.skin, her, gn.mixed), eye: gn.eye_rgb, hair: hc, tip, grey, htype: gn.texture, hairKey: hair, hairScale,
      beardKey: bk ? `beard_${bk}_${coily ? "coily" : "straight"}` : null, beardScale, bg: gn.beard, rec, w, hin, mus: lk.muscle ?? .55 + .33 * (gn.muscle ?? .5),
      tr: { bfa: 0, brow, seed: (gn.face % 10000) / 100, lip: .6 * dark + .4 * nr(), hairline: .07 * nr(), crease: [gn.lid_crease ? 1 : 0, gn.crease_mm || 6],
        freck: gn.freckles || 0, moles: gn.moles || 0, scars: gn.acne_scars || 0, acne: 0, pfb: bk ? 0 : gn.razor_bumps || 0, dpn: gn.dpn || 0, lip2: gn.two_tone_lips || 0,
        bf: lk.body_fat || 15, fu_s: fu },
      jersey: spec.colors || ["#3a3f4b", "#9aa3b2"],
      // a logo from a pack you installed (lane 15): a college's at the V-neck, the NFL shield at the collar; the page hands in
      // the decoded picture
      logo: spec.logo && spec.logo.img ? spec.logo : null,
      // a talk's tell (dialogue D1): the head turned (degrees to his left) and the eyes dropped
      view: spec.view || null,
    };
  }

  // ---------------------------------------------------------------- the engine: data, the scene, drawing
  function engine(M, raw, SH, sraw, LJ, GM, graw) {
    const C = { float32: Float32Array, uint32: Uint32Array, uint16: Uint16Array, int16: Int16Array, uint8: Uint8Array, int8: Int8Array };
    const arr = (n) => { const [o, len, t] = M.off[n]; return new C[t](raw, o, len); };
    const P0 = arr("P"), TRI = arr("tri"), PTR = arr("ptr"), IND = arr("ind"), WT = arr("wt");
    const NB = M.nb, NE = M.ne, NS = M.nsub, NV = P0.length / 3;
    const TG = {}; for (const k of M.targets) if (M.off["i:" + k]) TG[k] = [arr("i:" + k), arr("d:" + k)];
    const mask = (n) => { const m = new Float32Array(NV); if (TG[n]) { const [ii, dd] = TG[n]; for (let j = 0; j < ii.length; j++) m[ii[j]] = dd[j * 3] * .0005; } return m; };
    const HM = mask("afm/headmask");
    const mask2 = (n) => { const m = new Float32Array(NV * 2); if (TG[n]) { const [ii, dd] = TG[n]; for (let j = 0; j < ii.length; j++) { m[ii[j] * 2] = dd[j * 3] * .0005; m[ii[j] * 2 + 1] = dd[j * 3 + 1] * .0005; } } return m; };
    const LIPV = mask2("afm/lipmask"), LASHV = mask2("afm/lashmask"), LIDV = mask2("afm/lidmask");
    // the sampler's network weights (float16 on disk)
    const f16 = (u) => { const s = u & 0x8000 ? -1 : 1, e = (u >> 10) & 31, f = u & 1023; return e === 0 ? s * 5.960464477539063e-8 * f : e === 31 ? (f ? NaN : s * Infinity) : s * Math.pow(2, e - 15) * (1 + f / 1024); };
    const NW = {}; for (const k in SH.off) { const [o, shp] = SH.off[k], n = shp.reduce((a, b) => a * b, 1), u = new Uint16Array(sraw, o, n), f = new Float32Array(n); for (let i = 0; i < n; i++) f[i] = f16(u[i]); NW[k] = [f, shp]; }
    function decode(z, lab) {
      let x = new Float32Array(70); x.set(z); x.set(lab, 64);
      for (const i of [4, 5, 6, 7, 8]) {
        const [K, [ni, no]] = NW["k" + i], [B] = NW["b" + i], y = new Float32Array(no);
        for (let o = 0; o < no; o++) { let s = B[o]; for (let a = 0; a < ni; a++) s += x[a] * K[a * no + o]; y[o] = i < 8 ? Math.max(0, s) : s; }
        x = y;
      }
      return x;
    }

    // ---- shape: MakeHuman's macro targets for the body (age, muscle, weight), the GNM face on top (heads_v4.js)
    function ageW(y) {
      const v = y <= 25 ? .1875 + (y - 11) / 14 * .3125 : .5 + (y - 25) / 65 * .5;
      if (v < .5) { const young = Math.max(0, (v - .1875) * 3.2); return { child: Math.max(0, Math.min(1, 5.333 * v) - young), young, old: 0 }; }
      const young = 1 - (v - .5) * 2; return { child: 0, young, old: 1 - young };
    }
    const triW = (v) => v < .5 ? { min: 1 - v / .5, average: v / .5, max: 0 } : { min: 0, average: 1 - (v - .5) / .5, max: (v - .5) / .5 };
    function weights(p) {
      const W = {}, age = p.age, aw = ageW(age), bmi = p.w * 703 / (p.hin * p.hin);
      const wv = Math.max(.3, Math.min(1, .5 + (bmi - 24) / 26)), mv = p.mus;
      for (const a in aw) if (aw[a]) W[`macrodetails/caucasian-male-${a}`] = aw[a];
      const mw = triW(mv), ww = triW(wv);
      for (const a in aw) for (const m in mw) for (const w in ww) { const x = aw[a] * mw[m] * ww[w]; if (x > 1e-4) W[`macrodetails/universal-male-${a}-${m}muscle-${w}weight`] = (W[`macrodetails/universal-male-${a}-${m}muscle-${w}weight`] || 0) + x; }
      const fat = Math.max(0, (bmi - 27) / 12);
      const add = (k, x) => { if (!x) return; const t = x > 0 ? k + "-incr" : k + "-decr"; if (TG[t]) W[t] = (W[t] || 0) + Math.abs(x); else if (TG[k]) W[k] = (W[k] || 0) + x; };
      for (const k in p.g) add(k, p.g[k]);
      const WH = { "macrodetails/caucasian-male-young": 1, "macrodetails/universal-male-young-averagemuscle-averageweight": 1 };
      // face fat follows his fat mass (DXA scans by position), less the fat the sampled GNM face already carries (fatproj.py)
      const fu = (p.tr.bf / 100 * bmi - 5.8) / .62 - (p.tr.fu_s || 0);
      if (TG["afm/face-fat"]) W["afm/face-fat"] = fu / 10;
      const st = p.hin * 2.54 - 175.6;   // head size by height (ANSUR II)
      add("neck/neck-scale-horiz", Math.min(.6, fat * .4));
      add("head/head-age", age >= 30 ? (age - 28) / 12 : age < 19 ? -.25 : 0);
      if (age >= 30) { add("eyes/l-eye-bag", .25); add("eyes/r-eye-bag", .25); }
      return { W, WH, st };
    }
    const POS = new Float32Array(NV * 3), SUB = new Float32Array(NS * 3), REF = new Float32Array(NS * 3);
    function subdiv(src, dst) { for (let r = 0; r < NS; r++) { let x = 0, y = 0, z = 0; for (let k = PTR[r]; k < PTR[r + 1]; k++) { const i = IND[k] * 3, w = WT[k]; x += src[i] * w; y += src[i + 1] * w; z += src[i + 2] * w; } dst[r * 3] = x; dst[r * 3 + 1] = y; dst[r * 3 + 2] = z; } }
    subdiv(P0, REF);
    const sub2 = (m) => { const o = new Float32Array(NS * 2); for (let r = 0; r < NS; r++) { let a = 0, b = 0; for (let k = PTR[r]; k < PTR[r + 1]; k++) { const i = IND[k] * 2, w = WT[k]; a += m[i] * w; b += m[i + 1] * w; } o[r * 2] = a; o[r * 2 + 1] = b; } return o; };
    const LIPS = sub2(LIPV), LASH = sub2(LASHV), LID = sub2(LIDV);
    const DB = new Float32Array(NV * 3), DH = new Float32Array(NV * 3);
    function morph(W, WH, st) {
      POS.set(P0); DB.fill(0); DH.fill(0);
      const keys = new Set(Object.keys(W)); for (const k in WH) keys.add(k);
      for (const k of keys) {
        const t = TG[k]; if (!t) continue; const [ii, dd] = t, mac = k.startsWith("macrodetails/");
        if (!mac) { const w = (W[k] || 0) * .0005; if (!w) continue; for (let j = 0; j < ii.length; j++) { const i = ii[j] * 3; POS[i] += dd[j * 3] * w; POS[i + 1] += dd[j * 3 + 1] * w; POS[i + 2] += dd[j * 3 + 2] * w; } continue; }
        const wb = (W[k] || 0) * .0005, wh = (WH[k] || 0) * .0005;
        for (let j = 0; j < ii.length; j++) { const i = ii[j] * 3; for (let c = 0; c < 3; c++) { DB[i + c] += dd[j * 3 + c] * wb; DH[i + c] += dd[j * 3 + c] * wh; } }
      }
      let dx = 0, dy = 0, dz = 0, n = 0;
      for (let v = 0; v < NV; v++) { const h = HM[v]; if (h > .05 && h < .95) { const i = v * 3; dx += DH[i] - DB[i]; dy += DH[i + 1] - DB[i + 1]; dz += DH[i + 2] - DB[i + 2]; n++; } }
      if (n) { dx /= n; dy /= n; dz /= n; }
      for (let v = 0; v < NV; v++) { const i = v * 3, h = HM[v]; POS[i] += DB[i] + h * (DH[i] - DB[i] - dx); POS[i + 1] += DB[i + 1] + h * (DH[i + 1] - DB[i + 1] - dy); POS[i + 2] += DB[i + 2] + h * (DH[i + 2] - DB[i + 2] - dz); }
      if (st) {
        let ey = 0, ez = 0; for (let j = 0; j < 2 * NE; j++) { ey += POS[(NB + j) * 3 + 1]; ez += POS[(NB + j) * 3 + 2]; } ey /= 2 * NE; ez /= 2 * NE; const cy = ey, cz = ez - .6;
        const sx = .0007 * st, sy = .0021 * st, sz = .0019 * st;
        for (let v = 0; v < NV; v++) { const h = HM[v]; if (!h) continue; const i = v * 3; POS[i] += POS[i] * sx * h; POS[i + 1] += (POS[i + 1] - cy) * sy * h; POS[i + 2] += (POS[i + 2] - cz) * sz * h; }
      }
      subdiv(POS, SUB);
    }

    // ---- three.js scene
    const R = new THREE.WebGLRenderer({ antialias: true, alpha: true, preserveDrawingBuffer: true });
    R.setPixelRatio(1); R.setSize(W0 * SS, H0 * SS); R.setClearColor(0x000000, 0);
    R.outputEncoding = THREE.sRGBEncoding; R.physicallyCorrectLights = true;
    R.toneMapping = THREE.CustomToneMapping; R.toneMappingExposure = Math.pow(2, -.35);
    R.shadowMap.enabled = true; R.shadowMap.type = THREE.VSMShadowMap;
    const S = new THREE.Scene(), cam = new THREE.PerspectiveCamera(22, W0 / H0, .5, 60);
    const geo = new THREE.BufferGeometry();
    geo.setAttribute("position", new THREE.BufferAttribute(SUB, 3)); geo.setAttribute("aRef", new THREE.BufferAttribute(REF, 3)); geo.setAttribute("aLip", new THREE.BufferAttribute(LIPS, 2));
    geo.setAttribute("aLash", new THREE.BufferAttribute(LASH, 2)); geo.setAttribute("aLid", new THREE.BufferAttribute(LID, 2)); geo.setIndex(new THREE.BufferAttribute(TRI, 1));
    const U = { uSkin: { value: new THREE.Color() }, uHair: { value: new THREE.Color() }, uJersey: { value: new THREE.Color() }, uHairH: { value: 0 }, uHairD: { value: 1 }, uFadeLo: { value: 0 }, uFadeHi: { value: 0 }, uLineup: { value: 0 }, uSide: { value: 1 }, uNoJ: { value: 1 }, uWaves: { value: 0 }, uRows: { value: 0 },
      uHairY: { value: 7.95 }, uRecede: { value: 0 }, uBDens: { value: 0 }, uCheek: { value: .3 }, uNeck: { value: .3 }, uBStyle: { value: 0 }, uPatch: { value: 0 }, uAge: { value: 22 }, uSeed: { value: 0 }, uDark: { value: 0 }, uScalp: { value: null }, uUseScalp: { value: 0 }, uCR: { value: new THREE.Vector3() },
      uBrowA: { value: new THREE.Vector4(0, 1, 1, 1) }, uBrowB: { value: new THREE.Vector4(0, 0, .95, 0) }, uLipT: { value: 0 }, uGnm: { value: 1 }, uTrA: { value: new THREE.Vector4() }, uTrB: { value: new THREE.Vector4() }, uLip2: { value: 0 }, uBFull: { value: 0 }, uLipR: { value: new THREE.Vector3(1, .829, .829) } };
    const NOISE=`
float h3(vec3 p){p=fract(p*.3183099+.1);p*=17.;return fract(p.x*p.y*p.z*(p.x+p.y+p.z));}
float n3(vec3 x){vec3 i=floor(x),f=fract(x);f=f*f*(3.-2.*f);
 return mix(mix(mix(h3(i),h3(i+vec3(1,0,0)),f.x),mix(h3(i+vec3(0,1,0)),h3(i+vec3(1,1,0)),f.x),f.y),mix(mix(h3(i+vec3(0,0,1)),h3(i+vec3(1,0,1)),f.x),mix(h3(i+vec3(0,1,1)),h3(i+vec3(1,1,1)),f.x),f.y),f.z);}
uniform float uHairY,uRecede,uBDens,uCheek,uNeck,uBStyle,uPatch,uHairH,uHairD,uFadeLo,uFadeHi,uLineup,uSide,uWaves,uRows,uAge,uSeed,uDark;uniform vec4 uBrowA,uBrowB;uniform float uLipT,uGnm,uLip2,uBFull;uniform vec4 uTrA,uTrB;
float hairMask(vec3 p,float sd){
 float ax=abs(p.x);
 float hy=uHairY+uRecede*(.10+.22*smoothstep(.08,.42,ax)*smoothstep(.75,.2,ax));
 float hl=mix(6.92,hy,smoothstep(.05,.95,p.z));
 float ear=smoothstep(.62,.72,ax)*smoothstep(7.62,7.5,p.y)*smoothstep(-.15,.05,p.z)*smoothstep(.75,.55,p.z);
 float side=smoothstep(.55,.66,ax)*smoothstep(.95,.7,p.z)*smoothstep(7.75,7.55,p.y);
 float e=mix(.03,.006,uLineup);
 float m=smoothstep(hl-e*.5,hl+e,p.y);
 if(sd>.99)m=max(m,side*smoothstep(7.05,7.05+e*2.,p.y)*smoothstep(.25,.45,p.z));
 else m=max(m,sd*smoothstep(.5,.7,ax)*smoothstep(1.,.55,p.z)*smoothstep(7.8,7.4,p.y)*smoothstep(6.95,7.3,p.y)*smoothstep(.15,.55,p.z));
 return m*(1.-ear)*smoothstep(6.0,6.3,p.y);
}
float fadeF(vec3 p){return uFadeHi>0.?mix(.04,1.,smoothstep(uFadeLo,uFadeHi,p.y)):1.;}
float beardMask(vec3 p,float lv){
 float ax=abs(p.x);
 float up=6.86+(.20+.14*uCheek)*pow(smoothstep(.2,.7,ax),1.7);
 float lo=6.28+.14*uNeck;
 float m=(1.-smoothstep(up-.05,up+.04,p.y))*smoothstep(lo,lo+.16,p.y+p.z*.08)*smoothstep(.2,.55,p.z);
 float lip=1.-smoothstep(.85,1.15,length(vec2(p.x/.215,(p.y-6.742)/.06)));
 m*=uGnm>.5?1.-smoothstep(.08,.4,lv):1.-lip*smoothstep(1.05,1.15,p.z);
 m*=1.-smoothstep(.66,.74,ax)*smoothstep(6.95,7.05,p.y);
 if(uBStyle>.5&&uBStyle<1.5)m*=1.-smoothstep(.19,.26,ax);
 if(uBStyle>1.5){m*=1.-smoothstep(6.56,6.64,p.y)*(1.-smoothstep(.46,.55,ax));}
 if(uPatch>.01){float pt=mix(n3(p*9.+3.),1.,.5*uBFull);m*=mix(1.,smoothstep(uPatch-.12,uPatch+.12,pt),smoothstep(.22,.4,ax));}
 return m;
}
`;
const skinMat=new THREE.MeshPhysicalMaterial({roughness:.52,metalness:0,specularIntensity:.55,sheen:1,sheenColor:new THREE.Color(0x251510),sheenRoughness:.55});
skinMat.onBeforeCompile=sh=>{
 Object.assign(sh.uniforms,U);
 sh.vertexShader=sh.vertexShader.replace("#include <common>","#include <common>\nattribute vec3 aRef;attribute vec2 aLip;attribute vec2 aLash;attribute vec2 aLid;varying vec3 vRef;varying vec2 vLip;varying vec2 vLash;varying vec2 vLid;"+NOISE)
  .replace("#include <begin_vertex>",`#include <begin_vertex>
   vRef=aRef;vLip=aLip;vLash=aLash;vLid=aLid;
   float hm=hairMask(aRef,uSide>.99?1.:0.)*fadeF(aRef);
   float rr=fract(aRef.x/.1)-.5;float ridge=uRows*(1.-smoothstep(.15,.42,abs(rr)));
   transformed+=objectNormal*hm*(uHairH+ridge*.035);`);
 sh.fragmentShader=sh.fragmentShader.replace("#include <common>","#include <common>\nuniform vec3 uSkin,uHair,uJersey,uLipR;uniform float uNoJ;varying vec3 vRef;uniform sampler2D uScalp;uniform float uUseScalp;uniform vec3 uCR;varying vec2 vLip;varying vec2 vLash;varying vec2 vLid;"+NOISE+`
   vec3 bumpN(vec3 n,float h,float s){vec3 dpx=dFdx(-vViewPosition),dpy=dFdy(-vViewPosition);float hx=dFdx(h),hy=dFdy(h);
    vec3 r1=cross(dpy,n),r2=cross(n,dpx);float det=dot(dpx,r1);vec3 g=sign(det)*(hx*r1+hy*r2);return normalize(abs(det)*n-g*s);}`)
 .replace("#include <map_fragment>",`
   vec3 p=vRef;float ax=abs(p.x);float trBump=0.;
   float jersey=(1.-uNoJ)*(1.-smoothstep(6.10,6.14,p.y+ (p.z>.4?.0:.06)));
   float collar=jersey*smoothstep(6.05,6.08,p.y+ (p.z>.4?.0:.06));
   vec3 sk=uSkin*(0.94+.12*n3(p*6.+uSeed));
   float lips=1.-smoothstep(.8,1.1,length(vec2(p.x/.2,(p.y-6.742)/.052)));lips*=smoothstep(1.08,1.18,p.z);float lipW=max(vLip.x,vLip.y);if(uGnm>.5)lips=smoothstep(.3,.7,lipW);
   float cheeks=exp(-pow(length(vec2(ax-.42,p.y-6.98))/.16,2.))*smoothstep(.8,1.1,p.z);
   float nose=exp(-pow(length(vec2(p.x,p.y-6.93))/.1,2.))*smoothstep(1.4,1.6,p.z);
   float ears=smoothstep(.68,.78,ax)*smoothstep(6.95,7.1,p.y)*smoothstep(7.65,7.5,p.y);
   vec3 red=mix(vec3(1.0,.62,.58),vec3(.9,.55,.5),uDark);
   sk=mix(sk,sk*red*1.25,(cheeks*.35+nose*.3+ears*.35)*(1.-uDark*.6));
   sk=mix(sk,sk*uLipR*(1.+.1*uLipT),lips);
   if(uLip2>0.){float lo=smoothstep(.3,.7,vLip.y)*smoothstep(.15,.35,vLip.y-vLip.x+.2);sk=mix(sk,sk*vec3(1.55,1.18,1.2),lo*uLip2*.75);}
   if(uGnm>.5){
    // per-player skin traits (proposed, not confirmed): lid crease, freckles, moles, acne scars, active acne, razor bumps, DPN
    float dmm=vLid.y*10.-5.;float cr=uTrA.x*vLid.x*exp(-pow((dmm-uTrA.y)/.55,2.));
    sk*=1.-.30*cr-.08*uTrA.x*vLid.x*smoothstep(uTrA.y+.4,uTrA.y-.6,dmm)*smoothstep(-.3,1.,dmm);
    float fz=exp(-pow(length(vec2(ax*.8,(p.y-7.02)*1.6))/.36,2.))*smoothstep(1.,1.3,p.z)*(1.-lips);
    float fr=smoothstep(.70,.80,n3(p*62.+uSeed*13.))*smoothstep(.35,.6,n3(p*9.+uSeed));
    sk=mix(sk,sk*vec3(.74,.62,.52),fr*fz*uTrA.z*.85);
    for(int i=0;i<8;i++){if(float(i)>=uTrA.w)break;vec3 q=vec3(uSeed*7.1+float(i)*3.7,uSeed*3.3+float(i)*1.9,float(i)*5.3);
     vec2 mc=vec2(mix(-.62,.62,h3(q)),mix(6.35,7.75,h3(q+11.)));float mr=mix(.010,.019,h3(q+23.));
     float awayE=min(length(vec2(ax-.308,p.y-7.29)),1.)>.13?1.:0.;float md=length(vec2(p.x-mc.x,p.y-mc.y))/mr;
     sk=mix(sk,sk*vec3(.42,.33,.28),(1.-smoothstep(.75,1.,md))*smoothstep(.5,.8,p.z)*awayE*(1.-lips));}
    float cz=exp(-pow(length(vec2(ax-.4,(p.y-6.9)*1.2))/.2,2.))*smoothstep(.7,1.,p.z);
    float pit=smoothstep(.80,.88,n3(p*75.+uSeed*5.));sk*=1.-.10*pit*cz*uTrB.x;
    float ac=smoothstep(.83,.9,n3(p*58.+uSeed*9.+40.))*max(cz,exp(-pow(length(vec2(p.x,p.y-7.6))/.3,2.))*smoothstep(1.,1.3,p.z));
    sk=mix(sk,sk*vec3(1.12,.78,.74),ac*uTrB.y*(1.-uDark*.4));
    float bz=beardMask(p,lipW);float rb=smoothstep(.78,.86,n3(p*90.+uSeed*3.));sk*=1.-.16*rb*bz*uTrB.z*smoothstep(6.3,6.6,p.y+.3*(1.-smoothstep(.3,.6,ax)));
    float dz=exp(-pow(length(vec2(ax-.33,(p.y-7.1)*1.3))/.15,2.))*smoothstep(1.,1.25,p.z);float dp=smoothstep(.82,.9,n3(p*80.+uSeed*2.));
    sk=mix(sk,sk*.45,dp*dz*uTrB.w);
    trBump=pit*cz*uTrB.x*-.8+ac*uTrB.y*.5+rb*bz*uTrB.z*.6+dp*dz*uTrB.w*.5;
   }
   float eyeSock=0.;for(int i=0;i<2;i++){vec2 c=vec2(i==0?.308:-.308,7.29);float d=length(vec2(p.x-c.x,(p.y-c.y)*1.25));eyeSock+=exp(-pow(d/.17,2.));}
   sk*=1.-eyeSock*.16*smoothstep(1.,1.25,p.z);
   sk*=mix(1.,.86,smoothstep(.9,.4,p.z)*smoothstep(6.3,6.6,p.y));
   sk*=0.97+.06*n3(p*38.);
   float strands=n3(vec3(p.x*160.,p.y*40.,p.z*160.));
   float browA=ax-.31-uBrowB.w;float browY=7.43+uBrowA.x+.035*uBrowA.y*(1.-pow(browA/(.22*uBrowA.z),2.))+uBrowB.y*browA;
   float bw=mix(.03,.012,smoothstep(-.12,.2,browA))*uBrowA.w;float brow=(1.-smoothstep(bw*.6,bw,abs(p.y-browY)))*(1.-smoothstep(.14*uBrowA.z,.22*uBrowA.z,abs(browA+.02-uBrowB.x)))*(.55+.45*strands)*uBrowB.z*smoothstep(1.,1.2,p.z)*smoothstep(.06,.12,ax);
   float hm=hairMask(p,uUseScalp>.5?0.:(uSide>.99?1.:.75));
   float ff=fadeF(p);
   float hn=n3(p*vec3(140.,60.,140.));
   float rr=fract(p.x/.1)-.5;float part=uRows*smoothstep(.36,.48,abs(rr));
   float braid=uRows>0.?.55+.45*sin((p.z*1.2-p.y)*80.+sign(rr)*abs(rr)*30.):1.;
   float wv=uWaves>0.?.5+.5*sin(length(p-vec3(0.,8.3,-.35))*115.+n3(p*6.)*2.):1.;
   float dens=hm*uHairD*(.5+.5*hn)*ff*(1.-part);
   if(uUseScalp>.5){vec3 sd=normalize(p-uCR);vec4 sc=texture2D(uScalp,vec2(atan(sd.x,sd.z)/6.28318+.5,1.-acos(clamp(sd.y,-1.,1.))/3.14159));dens=sc.r*hm*1.1;wv=sc.g*1.4;}
   float bm=beardMask(p,lipW);float stub=bm*uBDens*mix(.35+.65*n3(p*170.),.8+.2*n3(p*170.),uBFull);
   vec3 col=sk;
   col=mix(col,uHair*(0.8+.4*strands),clamp(brow*.8,0.,1.));
   if(uGnm>.5){float la=clamp(smoothstep(.15,.6,vLash.x)*.9+smoothstep(.2,.7,vLash.y)*.45,0.,.92);col=mix(col,vec3(.025,.018,.014),la);}
   col=mix(col,uHair*mix(.7+.5*hn,.55+.35*hn,uBFull),clamp(stub*smoothstep(0.,.35,bm)*(1.+.25*uBFull),0.,1.));
   col=mix(col,uHair*(.6+.6*hn)*braid*mix(1.,.55+.6*wv,uWaves),clamp(dens*1.7,0.,1.));
   vec3 jc=uJersey*(.92+.08*n3(p*vec3(200.,200.,200.)));
   col=mix(col,mix(jc,mix(jc,vec3(1.),.75),collar),jersey);
   diffuseColor.rgb=col;
   float hairAmt=clamp(dens*1.6+stub+brow,0.,1.);
   float skinH=n3(p*420.)*.6+n3(p*160.)*.4;
   float tz=exp(-pow(length(vec2(p.x,(p.y-7.35)/2.))/.25,2.))*smoothstep(1.1,1.5,p.z);
   float wr=uAge>=30.?smoothstep(30.,40.,uAge):0.;
   float lines=wr*(sin(p.y*95.)*.5+.5)*smoothstep(7.55,7.65,p.y)*smoothstep(7.95,7.85,p.y)*smoothstep(1.,1.25,p.z)*(1.-smoothstep(.25,.45,ax));
   float crow=wr*exp(-pow(length(vec2(ax-.5,p.y-7.28))/.07,2.))*(sin(atan(p.y-7.28,ax-.5)*14.)*.5+.5);
   float hh=mix(skinH*mix(.4,.22,uGnm)+lines*.9+crow*.8+trBump,hn*1.2+wv*uWaves*1.5+braid*uRows*1.5,hairAmt);
   hh=mix(hh,n3(p*vec3(120.,30.,120.))*.4,jersey);
   `)
 .replace("#include <roughnessmap_fragment>",`float roughnessFactor=roughness;roughnessFactor=mix(mix(.55,.4,tz),.36,lips);roughnessFactor=mix(roughnessFactor,.85,hairAmt);roughnessFactor=mix(roughnessFactor,.9,jersey);`)
 .replace("#include <normal_fragment_maps>",`#include <normal_fragment_maps>
   normal=bumpN(normal,hh,mix(.0035,.006,hairAmt));`);
};

    const mesh = new THREE.Mesh(geo, skinMat); mesh.castShadow = true; mesh.receiveShadow = true; S.add(mesh);
    const eyeMat=new THREE.MeshPhysicalMaterial({roughness:.15,clearcoat:1,clearcoatRoughness:.03});
const UE={uIris:{value:new THREE.Color()}};
eyeMat.onBeforeCompile=sh=>{Object.assign(sh.uniforms,UE);
 sh.vertexShader=sh.vertexShader.replace("#include <common>","#include <common>\nvarying vec3 vL;").replace("#include <begin_vertex>","#include <begin_vertex>\nvL=normalize(position);");
 sh.fragmentShader=sh.fragmentShader.replace("#include <common>","#include <common>\nvarying vec3 vL;uniform vec3 uIris;")
 .replace("#include <map_fragment>",`float a=acos(clamp(vL.z,-1.,1.));
  vec3 scl=vec3(.62,.57,.53);float ang=atan(vL.y,vL.x);
  vec3 iris=uIris*(.7+.5*sin(ang*40.)*.15+.3*smoothstep(.36,.2,a));
  vec3 c=mix(scl,iris,smoothstep(.43,.40,a));c=mix(c,vec3(.01),smoothstep(.17,.15,a));c*=mix(1.,.55,smoothstep(.4,.44,a)*smoothstep(.5,.44,a));c*=1.-.5*smoothstep(-.02,.3,vL.y);c*=1.-.3*smoothstep(.45,.8,abs(vL.x));
  diffuseColor.rgb=c;`);};

    const eyes = [0, 1].map(() => { const m = new THREE.Mesh(new THREE.SphereGeometry(1, 48, 32), eyeMat); m.receiveShadow = true; S.add(m); return m; });

    // ---- the approved light (render_v7.py LIGHT=appr: the jersey renders Cain approved): a key far to the left and above, a
    // weak fill from the right, two rims from behind, a dim grey world. Area lights become directional ones with the same
    // irradiance ratios (power over distance squared); the key casts a soft shadow.
    const amb = new THREE.HemisphereLight(0x8a8a94, 0x3a3634, .3); S.add(amb);
    const L = (col, pw, pos) => { const l = new THREE.DirectionalLight(col, pw); l.userData.pos = pos; S.add(l); S.add(l.target); return l; };
    const key = L(0xfff2e3, 1, [-6, 4, 6]), fill = L(0xebf2ff, 1, [6.5, 0, 5]), rim = L(0xffffff, 1, [3, 3, -6]), rim2 = L(0xffffff, 1, [-4, 2.5, -5]);
    const POW = [[key, 2600], [fill, 650], [rim, 2200], [rim2, 1300]], LK = 5.2 / (2600 / 88);
    key.castShadow = true; key.shadow.mapSize.set(1024, 1024); key.shadow.bias = -.0006; key.shadow.normalBias = .01; key.shadow.radius = 14; key.shadow.blurSamples = 16;
    Object.assign(key.shadow.camera, { left: -2.2, right: 2.2, top: 2.2, bottom: -2.2, near: 1, far: 30 });

    // ---- hair: the baked library (bake_lib.py), regrown on this head: each strand's root sits on its triangle, its steps
    // turn with the root's own frame; drawn as thin ribbons that face the camera, lit along the strand (Kajiya-Kay)
    const glib = GM.lib;
    const gArr = (a) => new C[a[2]](graw, a[0], a[1]);
    function hairMat() {
      const m = new THREE.MeshStandardMaterial({ vertexColors: true, roughness: 1, metalness: 0, side: THREE.DoubleSide });
      m.onBeforeCompile = (sh) => {
        sh.vertexShader = sh.vertexShader.replace("#include <common>", "#include <common>\nattribute vec3 aTan;varying vec3 vTan;").replace("#include <begin_vertex>", "#include <begin_vertex>\nvTan=normalize(normalMatrix*aTan);");
        sh.fragmentShader = sh.fragmentShader.replace("#include <common>", "#include <common>\nvarying vec3 vTan;")
          .replace("#include <lights_fragment_end>", `#include <lights_fragment_end>
          reflectedLight.directSpecular=vec3(0.);reflectedLight.indirectSpecular=vec3(0.);
          #if NUM_DIR_LIGHTS>0
          {vec3 T=normalize(vTan),V=normalize(vViewPosition);
           for(int i=0;i<NUM_DIR_LIGHTS;i++){vec3 Ld=directionalLights[i].direction;vec3 Hh=normalize(Ld+V);
            float t1=dot(normalize(T+normal*.1),Hh),t2=dot(normalize(T-normal*.16),Hh);
            float s1=pow(sqrt(max(0.,1.-t1*t1)),120.),s2=pow(sqrt(max(0.,1.-t2*t2)),20.);
            float wrap=clamp(dot(normal,Ld)*.5+.5,0.,1.);
            reflectedLight.directSpecular+=directionalLights[i].color*wrap*(s1*.025+s2*.06*diffuseColor.rgb*3.);}}
          #endif`);
      };
      return m;
    }
    const HMAT = hairMat();
    const hairMesh = new THREE.Mesh(new THREE.BufferGeometry(), HMAT); hairMesh.castShadow = true; hairMesh.receiveShadow = true; S.add(hairMesh);
    const ropeMesh = new THREE.Mesh(new THREE.BufferGeometry(), HMAT); ropeMesh.castShadow = true; ropeMesh.receiveShadow = true; S.add(ropeMesh);
    // grow one layer's strands: positions (k a strand) and each root's normal; `keep(rref)` culls by his hair line or beard genes
    function grow(Lr, N, scale, keep, Rn) {
      const n = Lr.n, k = Lr.k, tri = gArr(Lr.tri), bc = gArr(Lr.bc), d = gArr(Lr.d), sc = Lr.sc * scale;
      const pts = new Float32Array(n * k * 3), nrm = new Float32Array(n * 3), ok = new Uint8Array(n); let kept = 0;
      for (let c = 0; c < n; c++) {
        const t3 = tri[c] * 3, a = TRI[t3] * 3, b = TRI[t3 + 1] * 3, e = TRI[t3 + 2] * 3, v = bc[c * 2] / 255, w = bc[c * 2 + 1] / 255, u = 1 - v - w;
        const rx = u * REF[a] + v * REF[b] + w * REF[e], ry = u * REF[a + 1] + v * REF[b + 1] + w * REF[e + 1], rz = u * REF[a + 2] + v * REF[b + 2] + w * REF[e + 2];
        if (keep && !keep(rx, ry, rz, Rn())) continue;
        ok[c] = 1; kept++;
        let px = u * SUB[a] + v * SUB[b] + w * SUB[e], py = u * SUB[a + 1] + v * SUB[b + 1] + w * SUB[e + 1], pz = u * SUB[a + 2] + v * SUB[b + 2] + w * SUB[e + 2];
        let nx = u * N[a] + v * N[b] + w * N[e], ny = u * N[a + 1] + v * N[b + 1] + w * N[e + 1], nz = u * N[a + 2] + v * N[b + 2] + w * N[e + 2]; const nl = Math.hypot(nx, ny, nz) || 1; nx /= nl; ny /= nl; nz /= nl;
        let ex = SUB[b] - SUB[a], ey = SUB[b + 1] - SUB[a + 1], ez = SUB[b + 2] - SUB[a + 2]; const dn = ex * nx + ey * ny + ez * nz; ex -= nx * dn; ey -= ny * dn; ez -= nz * dn; const tl = Math.hypot(ex, ey, ez) || 1; ex /= tl; ey /= tl; ez /= tl;
        const bx = ny * ez - nz * ey, by = nz * ex - nx * ez, bz = nx * ey - ny * ex;
        nrm[c * 3] = nx; nrm[c * 3 + 1] = ny; nrm[c * 3 + 2] = nz;
        const o0 = c * k * 3; pts[o0] = px; pts[o0 + 1] = py; pts[o0 + 2] = pz;
        for (let j = 1; j < k; j++) {
          const q = (c * (k - 1) + j - 1) * 3, sx = d[q] * sc, sy = d[q + 1] * sc, sz = d[q + 2] * sc;
          px += sx * ex + sy * bx + sz * nx; py += sx * ey + sy * by + sz * ny; pz += sx * ez + sy * bz + sz * nz;
          const o = o0 + j * 3; pts[o] = px; pts[o + 1] = py; pts[o + 2] = pz;
        }
      }
      return { n, k, pts, nrm, ok, kept };
    }
    // camera-facing ribbons; colour per strand (his hair colour, grey hairs at his share, a little variation), darker at the
    // root (the hair around it shades it), the tip tapered
    function ribbons(parts, camPos) {
      let nv = 0, ni = 0; for (const p of parts) { nv += p.kept * p.k * 2; ni += p.kept * (p.k - 1) * 6; }
      const pos = new Float32Array(nv * 3), nor = new Float32Array(nv * 3), col = new Float32Array(nv * 3), tan = new Float32Array(nv * 3), idx = new Uint32Array(ni);
      let v = 0, ii = 0;
      for (const p of parts) {
        const { n, k, pts, nrm, ok, w, ao, hc, gc, grey, R2, tc } = p;
        for (let c = 0; c < n; c++) {
          if (!ok[c]) continue;
          const gm = R2() < grey ? .75 + .25 * R2() : 0, vary = .78 + .3 * R2(), wc = w * (.75 + .5 * R2());
          const base = [0, 1, 2].map((i) => (hc[i] + (gc[i] - hc[i]) * gm) * vary);
          const nn = [nrm[c * 3], nrm[c * 3 + 1], nrm[c * 3 + 2]];
          for (let j = 0; j < k; j++) {
            const o = (c * k + j) * 3, a = (c * k + Math.max(0, j - 1)) * 3, b = (c * k + Math.min(k - 1, j + 1)) * 3;
            let dx = pts[b] - pts[a], dy = pts[b + 1] - pts[a + 1], dz = pts[b + 2] - pts[a + 2]; const dl = Math.hypot(dx, dy, dz) || 1; dx /= dl; dy /= dl; dz /= dl;
            let vx = camPos.x - pts[o], vy = camPos.y - pts[o + 1], vz = camPos.z - pts[o + 2];
            let sx = dy * vz - dz * vy, sy = dz * vx - dx * vz, sz = dx * vy - dy * vx; const sl = Math.hypot(sx, sy, sz) || 1;
            const t = j / (k - 1), hw = wc * .5 * (1 - .6 * t * t) / sl; sx *= hw; sy *= hw; sz *= hw;
            const sh = ao * (.35 + .65 * Math.min(1, t * 2.4)) + (1 - ao);
            for (let s = 0; s < 2; s++) {
              const g = s ? 1 : -1; pos[v * 3] = pts[o] + sx * g; pos[v * 3 + 1] = pts[o + 1] + sy * g; pos[v * 3 + 2] = pts[o + 2] + sz * g;
              nor[v * 3] = nn[0]; nor[v * 3 + 1] = nn[1]; nor[v * 3 + 2] = nn[2];
              const tm = tc ? ss(.45, .7, t) : 0, cc = tm ? [0, 1, 2].map((i) => base[i] + (tc[i] * vary - base[i]) * tm) : base;
              col[v * 3] = cc[0] * sh; col[v * 3 + 1] = cc[1] * sh; col[v * 3 + 2] = cc[2] * sh;
              tan[v * 3] = dx; tan[v * 3 + 1] = dy; tan[v * 3 + 2] = dz; v++;
            }
            if (j < k - 1) { const v0 = v - 2; idx[ii++] = v0; idx[ii++] = v0 + 1; idx[ii++] = v0 + 3; idx[ii++] = v0; idx[ii++] = v0 + 3; idx[ii++] = v0 + 2; }
          }
        }
      }
      return geoOf(pos, nor, col, tan, idx);
    }
    function geoOf(pos, nor, col, tan, idx) {
      const g = new THREE.BufferGeometry(); g.setAttribute("position", new THREE.BufferAttribute(pos, 3)); g.setAttribute("normal", new THREE.BufferAttribute(nor, 3));
      g.setAttribute("color", new THREE.BufferAttribute(col, 3)); g.setAttribute("aTan", new THREE.BufferAttribute(tan, 3)); g.setIndex(new THREE.BufferAttribute(idx, 1)); return g;
    }
    // locs, twists, braids and cornrows: round ropes along the baked centre lines
    function ropes(e, N, scale, keep, Rn, hc) {
      const r = e.ropes, G = grow(r, N, scale, keep, Rn), rad = gArr(r.r), ring = 8, rv = ring + 1, k = G.k;
      const nv = G.kept * k * rv, pos = new Float32Array(nv * 3), nor = new Float32Array(nv * 3), col = new Float32Array(nv * 3), tan = new Float32Array(nv * 3), idx = new Uint32Array(G.kept * (k - 1) * ring * 6);
      let v = 0, ii = 0; const R2 = rng(7);
      for (let c = 0; c < G.n; c++) {
        if (!G.ok[c]) continue; let prev = null; const vary = .8 + .4 * R2(), P = G.pts;
        for (let j = 0; j < k; j++) {
          const o = (c * k + j) * 3, a = (c * k + Math.max(0, j - 1)) * 3, b = (c * k + Math.min(k - 1, j + 1)) * 3;
          const d = norm([P[b] - P[a], P[b + 1] - P[a + 1], P[b + 2] - P[a + 2]]);
          let s = prev ? norm(cross(cross(d, prev), d)) : norm(cross(d, [0, 0, 1])); prev = s; const bb = cross(d, s);
          const rr = rad[c * k + j] * .0004, t = j / (k - 1), sh = (.45 + .55 * Math.min(1, t * 3)) * vary;
          for (let q = 0; q <= ring; q++) {
            const an = q / ring * Math.PI * 2, nn = [s[0] * Math.cos(an) + bb[0] * Math.sin(an), s[1] * Math.cos(an) + bb[1] * Math.sin(an), s[2] * Math.cos(an) + bb[2] * Math.sin(an)];
            // the felted or braided surface: fine grooves along the rope
            const gr = .78 + .22 * Math.sin(an * 3 + t * 90 + c);
            pos[v * 3] = P[o] + nn[0] * rr; pos[v * 3 + 1] = P[o + 1] + nn[1] * rr; pos[v * 3 + 2] = P[o + 2] + nn[2] * rr; nor.set(nn, v * 3);
            col[v * 3] = hc[0] * sh * gr; col[v * 3 + 1] = hc[1] * sh * gr; col[v * 3 + 2] = hc[2] * sh * gr; tan.set(d, v * 3); v++;
          }
          if (j < k - 1) for (let q = 0; q < ring; q++) { const v0 = v - rv + q, v1 = v0 + rv; idx[ii++] = v0; idx[ii++] = v1; idx[ii++] = v1 + 1; idx[ii++] = v0; idx[ii++] = v1 + 1; idx[ii++] = v0 + 1; }
        }
      }
      return geoOf(pos, nor, col, tan, idx);
    }
    const norm = (a) => { const l = Math.hypot(a[0], a[1], a[2]) || 1; return [a[0] / l, a[1] / l, a[2] / l]; };
    const cross = (a, b) => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];

    // ---- the jersey (lib_jersey.js): one cloth mesh made over shoulder pads, refitted to his body; his team's colours and no
    // number. The game ships no logos: one shows only from a pack you installed, painted on the cloth's own coordinates so it
    // follows the fit (the V's point is at x 0, y 5.56 on the front)
    const b64 = (s, Cc) => new Cc(Uint8Array.from(atob(s), (c) => c.charCodeAt(0)).buffer);
    const dec = (o, k, Cc) => o["_" + k] || (o["_" + k] = b64(o[k], Cc));
    function libFit(b, P, N) {
      const I = dec(b, "i", Uint16Array), Wt = dec(b, "w", Uint8Array), O = dec(b, "o", Int16Array), n = b.n, out = new Float32Array(n * 3);
      for (let p = 0; p < n; p++) {
        const i0 = I[p * 3] * 3, i1 = I[p * 3 + 1] * 3, i2 = I[p * 3 + 2] * 3, w0 = Wt[p * 2] / 255, w1 = Wt[p * 2 + 1] / 255, w2 = Math.max(0, 1 - w0 - w1);
        const ax = w0 * P[i0] + w1 * P[i1] + w2 * P[i2], ay = w0 * P[i0 + 1] + w1 * P[i1 + 1] + w2 * P[i2 + 1], az = w0 * P[i0 + 2] + w1 * P[i1 + 2] + w2 * P[i2 + 2];
        let nx = w0 * N[i0] + w1 * N[i1] + w2 * N[i2], ny = w0 * N[i0 + 1] + w1 * N[i1 + 1] + w2 * N[i2 + 1], nz = w0 * N[i0 + 2] + w1 * N[i1 + 2] + w2 * N[i2 + 2]; const nl = Math.hypot(nx, ny, nz) || 1; nx /= nl; ny /= nl; nz /= nl;
        let ex = P[i1] - P[i0], ey = P[i1 + 1] - P[i0 + 1], ez = P[i1 + 2] - P[i0 + 2]; const dn = ex * nx + ey * ny + ez * nz; ex -= nx * dn; ey -= ny * dn; ez -= nz * dn; const tl = Math.hypot(ex, ey, ez) || 1; ex /= tl; ey /= tl; ez /= tl;
        const bx = ny * ez - nz * ey, by = nz * ex - nx * ez, bz = nx * ey - ny * ex, ox = O[p * 3] / 3000, oy = O[p * 3 + 1] / 3000, oz = O[p * 3 + 2] / 3000;
        out[p * 3] = ax + ox * ex + oy * bx + oz * nx; out[p * 3 + 1] = ay + ox * ey + oy * by + oz * ny; out[p * 3 + 2] = az + ox * ez + oy * bz + oz * nz;
      }
      return out;
    }
    const JU = { uBody: { value: new THREE.Color() }, uC1: { value: new THREE.Color() }, uC2: { value: new THREE.Color() }, uC3: { value: new THREE.Color() }, uW: { value: new THREE.Vector3() }, uVB: { value: LJ.vb },
      uLogo: { value: null }, uLogoOn: { value: 0 }, uLogoC: { value: new THREE.Vector2() }, uLogoS: { value: 1 } };
    const jMat = new THREE.MeshStandardMaterial({ roughness: .78, metalness: 0 });
    jMat.onBeforeCompile = (sh) => {
      Object.assign(sh.uniforms, JU);
      sh.vertexShader = sh.vertexShader.replace("#include <common>", "#include <common>\nattribute vec3 aJR;attribute float aJD;varying vec3 vJR;varying float vJD;").replace("#include <begin_vertex>", "#include <begin_vertex>\nvJR=aJR;vJD=aJD;");
      sh.fragmentShader = sh.fragmentShader.replace("#include <common>", "#include <common>\nvarying vec3 vJR;varying float vJD;uniform vec3 uBody,uC1,uC2,uC3,uW;uniform float uVB;uniform sampler2D uLogo;uniform float uLogoOn,uLogoS;uniform vec2 uLogoC;")
        .replace("#include <color_fragment>", `#include <color_fragment>
         vec3 p=vJR;float ax=abs(p.x);vec3 col=vJD<uW.x?uC1:vJD<uW.y?uC2:vJD<uW.z?uC3:uBody;
         float sd=abs(-.95*ax+6.55-p.y);col*=1.-.25*step(sd,.035)*step(.5,ax);
         vec2 lq=(p.xy-uLogoC)/uLogoS+.5;
         if(uLogoOn>.5&&p.z>.3&&lq.x>0.&&lq.x<1.&&lq.y>0.&&lq.y<1.){vec4 L=texture2D(uLogo,lq);col=mix(col,pow(L.rgb,vec3(2.2)),L.a);}
         diffuseColor.rgb=col;`);
    };
    const jerseyMesh = new THREE.Mesh(new THREE.BufferGeometry(), jMat); jerseyMesh.receiveShadow = true; jerseyMesh.castShadow = true; S.add(jerseyMesh);
    function jGeo(P, N) {
      const g = new THREE.BufferGeometry(), pos = libFit(LJ.b, P, N);
      const r = dec(LJ, "r", Int16Array), d = dec(LJ, "d", Uint8Array), Rr = new Float32Array(r.length), D = new Float32Array(d.length);
      for (let i = 0; i < r.length; i++) Rr[i] = r[i] / 3000; for (let i = 0; i < d.length; i++) D[i] = d[i] / 200;
      // the jersey's lower edge carried down out of the picture: on a slim player the cloth would end above the frame's bottom
      for (let i = 0; i < Rr.length / 3; i++) { const ry = Rr[i * 3 + 1]; if (ry < 4.75) pos[i * 3 + 1] -= 1.6 * Math.min(1, (4.75 - ry) / .22); }
      g.setAttribute("position", new THREE.BufferAttribute(pos, 3));
      g.setAttribute("aJR", new THREE.BufferAttribute(Rr, 3)); g.setAttribute("aJD", new THREE.BufferAttribute(D, 1)); g.setIndex(new THREE.BufferAttribute(dec(LJ, "t", Uint16Array), 1)); g.computeVertexNormals(); return g;
    }
    const lin = (c) => new THREE.Color(c).convertSRGBToLinear();
    // the logo as a square texture, the picture fitted inside it
    const logoCv = document.createElement("canvas"); logoCv.width = logoCv.height = 256;
    const logoTex = new THREE.CanvasTexture(logoCv); let logoOf = null;
    function setLogo(L) {
      JU.uLogoOn.value = L ? 1 : 0;
      if (!L) return;
      const im = L.img, iw = im.naturalWidth || im.width || 1, ih = im.naturalHeight || im.height || 1, k = 240 / Math.max(iw, ih);
      if (logoOf !== L.img) {
        const x = logoCv.getContext("2d");
        x.clearRect(0, 0, 256, 256); x.drawImage(im, (256 - iw * k) / 2, (256 - ih * k) / 2, iw * k, ih * k);
        logoTex.needsUpdate = true; logoOf = L.img;
      }
      JU.uLogo.value = logoTex;
      // placed by its top edge: the shield's over the collar's point, a college's logo just under the V's trim
      const size = L.at === "collar" ? .26 : .42, hgt = size * ih * k / 256, top = L.at === "collar" ? 5.58 : 5.52;
      JU.uLogoC.value.set(0, top - size / 2 + (size - hgt) / 2); JU.uLogoS.value = size;
    }

    // ---- one head
    const out = document.createElement("canvas"); out.width = W0; out.height = H0;
    function draw(p) {
      const t0 = performance.now();
      const { W, WH, st } = weights(p); morph(W, WH, st);
      const e = []; for (const s of [0, 1]) { let c = [0, 0, 0], mn = 1e9, mx = -1e9; for (let j = 0; j < NE; j++) { const i = (NB + s * NE + j) * 3; c[0] += POS[i]; c[1] += POS[i + 1]; c[2] += POS[i + 2]; mn = Math.min(mn, POS[i + 1]); mx = Math.max(mx, POS[i + 1]); } e.push([c[0] / NE, c[1] / NE, c[2] / NE, (mx - mn) / 2]); }
      geo.attributes.position.needsUpdate = true; geo.computeVertexNormals(); geo.computeBoundingSphere();
      const N = geo.attributes.normal.array;
      // camera (render_v7.py as v7 ran it, ZOOM .72 and TY -.35, here a touch closer to fill the frame as Blender's did): 22 degrees, aimed below the eyes, 11.1 units out, a little above
      const ey = (e[0][1] + e[1][1]) / 2, ez = e[0][2], tgt = new THREE.Vector3(0, ey - .87, ez - .9);
      cam.position.set(0, tgt.y + .25, tgt.z + 8 / .77); cam.lookAt(tgt); cam.updateMatrixWorld();
      const cy = ey + .35, aim = new THREE.Vector3(0, cy - .6, .5);
      for (const [l, pw] of POW) { const q = l.userData.pos; l.position.set(q[0], cy + q[1], q[2]); l.target.position.copy(aim); const d2 = l.position.distanceToSquared(aim); l.intensity = pw / d2 * LK; }
      // a turned head: the camera and the lights go round him together, so the light on his face stays the approved one
      if (p.view && (p.view.yaw || p.view.down)) {
        const q = new THREE.Quaternion().setFromEuler(new THREE.Euler(-(p.view.down || 0) * Math.PI / 180, -(p.view.yaw || 0) * Math.PI / 180, 0, "YXZ"));
        const turn = (v) => v.sub(tgt).applyQuaternion(q).add(tgt);
        turn(cam.position); cam.lookAt(tgt); cam.updateMatrixWorld();
        for (const [l] of POW) turn(l.position);
      }
      key.shadow.camera.updateProjectionMatrix();
      // hair and beard
      // hair colour as Blender's hair shader showed it: lighter hair reads richer and a little darker than its flat colour
      const hcol = new THREE.Color(p.hair), hsl = hcol.getHSL({}); hcol.setHSL(hsl.h, Math.min(1, hsl.s * 1.3), hsl.l * .85).convertSRGBToLinear();
      const hc = [hcol.r, hcol.g, hcol.b], gc = [.30, .29, .28], tc = p.tip ? [lin(p.tip).r, lin(p.tip).g, lin(p.tip).b] : null;
      const age = p.age, grey = Math.min(1, p.grey || 0);
      const he = glib[p.hairKey] || glib.crop, Rn = rng(((p.tr.seed * 100) | 0) + 11);
      const hl0 = p.tr.hairline || 0, rec = p.rec || 0;
      // a shaved head keeps only a shadow of regrowth
      const keepHair = he.skin && he.skin.shaved ? (x, y, z, r) => r < .3 && hairMask(x, y, z, rec, hl0) > .15 + .7 * r : (x, y, z, r) => hairMask(x, y, z, rec, hl0) > .15 + .7 * r;
      const parts = [];
      for (const Lr of he.layers || []) { const G = grow(Lr, N, p.hairScale || 1, keepHair, Rn); parts.push({ ...G, w: Lr.w, ao: Lr.ao, hc, gc, grey, R2: rng(Lr.n + 5), tc }); }
      if (p.beardKey && glib[p.beardKey]) {
        const be = glib[p.beardKey], G0 = p.bg || { density: .7, patch: .2, cheek: .4, neck: .3 }, Gb = { cheek: G0.cheek, neck: G0.neck, patch: G0.patch };
        const dens = .45 + .55 * (G0.density ?? G0.dens ?? .7);
        const keepBeard = (x, y, z, r) => r < dens && beardMask(x, y, z, Gb) > .25;
        // beard strands drawn wider than the baked width: a beard reads as a dense mass, as Blender's thicker curves showed it
        for (const Lr of be.layers) { const G = grow(Lr, N, p.beardScale || 1, keepBeard, Rn); parts.push({ ...G, w: Lr.w * (/full/.test(p.beardKey) ? 2.4 : /stubble/.test(p.beardKey) ? .7 : 1.6), ao: Lr.ao, hc, gc, grey: Math.min(1, grey * 1.4), R2: rng(Lr.n + 9) }); }
      }
      hairMesh.geometry.dispose(); hairMesh.geometry = ribbons(parts, cam.position);
      ropeMesh.geometry.dispose(); ropeMesh.geometry = he.ropes ? ropes(he, N, p.hairScale || 1, keepHair, Rn, hc) : new THREE.BufferGeometry();
      // the scalp under the hair, painted as v4 did (skin shader): stubble of a fade, waves, cornrow parts
      const sk = he.skin || {};
      U.uHairH.value = sk.h || 0; U.uHairD.value = (sk.d ?? 1) * (sk.shaved ? .35 : 1); U.uLineup.value = sk.lineup || 0; U.uSide.value = 1; U.uRows.value = sk.rows || 0; U.uWaves.value = sk.waves || 0;
      U.uFadeLo.value = sk.lo || 0; U.uFadeHi.value = sk.hi || 0; U.uRecede.value = rec;
      const tr = p.tr;
      U.uBrowA.value.set(tr.brow[0], tr.brow[1], tr.brow[2], tr.brow[3]); U.uBrowB.value.set(tr.brow[4], tr.brow[5], tr.brow[6], tr.brow[7]);
      U.uSeed.value = tr.seed; U.uLipT.value = tr.lip; U.uHairY.value = 7.95 + (tr.hairline || 0); U.uAge.value = age; U.uGnm.value = 1;
      const cr = tr.crease || [1, 6.5]; U.uTrA.value.set(cr[0], cr[1], tr.freck || 0, tr.moles || 0); U.uTrB.value.set(tr.scars || 0, tr.acne || 0, tr.pfb || 0, tr.dpn || 0); U.uLip2.value = tr.lip2 || 0;
      const sl = p.skinLab ? labLin(p.skinLab.map((v, i) => v + (SKSHIFT[p.her] || [0, 2, -2])[i])) : [lin(p.skin).r, lin(p.skin).g, lin(p.skin).b];
      const ld = LIPD[p.her] || LIPD.white, lr = p.skinLab ? labLin(p.skinLab.map((v, i) => v + (SKSHIFT[p.her] || [0, 2, -2])[i] + ld[i])) : null;
      U.uLipR.value.set(...(lr ? lr.map((v, i) => v / Math.max(sl[i], 1e-4)) : [1, .829, .829]));
      U.uDark.value = Math.max(0, Math.min(1, (.75 - (Math.max(...sl) + Math.min(...sl)) / Math.pow(2, .45)) / .45));
      // the beard's shadow on the skin (stubble left by the strands' gaps) stays light: the strands carry the beard
      U.uBDens.value = !p.beardKey ? 0 : /full|chin_strap/.test(p.beardKey) ? .85 : /stubble/.test(p.beardKey) ? .3 : .2; const bgn = p.bg || {}; U.uCheek.value = bgn.cheek ?? .3; U.uNeck.value = bgn.neck ?? .3; U.uBStyle.value = 0; U.uPatch.value = bgn.patch ?? .2; U.uBFull.value = 1;
      U.uSkin.value.setRGB(...sl); U.uHair.value.set(...hc).lerp(lin("#9c9890"), grey * .6);
      e.forEach((q, i) => { eyes[i].position.set(q[0], q[1], q[2] - .012); eyes[i].scale.setScalar(q[3] * .97); });
      UE.uIris.value.copy(lin(p.eye));
      // jersey
      const [c1, c2] = p.jersey; jerseyMesh.geometry.dispose(); jerseyMesh.geometry = jGeo(SUB, N);
      JU.uBody.value.copy(lin(c1)); JU.uC1.value.copy(lin(c2)); JU.uC2.value.copy(lin(c1)); JU.uC3.value.copy(lin(c1)); JU.uW.value.set(.06, .16, .16); U.uNoJ.value = 1; setLogo(p.logo);
      const t1 = performance.now();
      R.render(S, cam);
      const g2 = out.getContext("2d"); g2.clearRect(0, 0, W0, H0); g2.imageSmoothingQuality = "high"; g2.drawImage(R.domElement, 0, 0, W0, H0);
      const t2 = performance.now();
      return { canvas: out, ms: { build: t1 - t0, render: t2 - t1 }, tris: (hairMesh.geometry.index ? hairMesh.geometry.index.count / 3 : 0) + (ropeMesh.geometry.index ? ropeMesh.geometry.index.count / 3 : 0) };
    }
    return { draw, decode, shape: SH, lib: glib };
  }

  // AgX (Sobotka; the three.js r160 fit), with Blender's "Medium High Contrast" look as extra contrast in the log domain
  function agx() {
    if (THREE.ShaderChunk.tonemapping_pars_fragment.includes("AgXInset")) return;
    THREE.ShaderChunk.tonemapping_pars_fragment = THREE.ShaderChunk.tonemapping_pars_fragment.replace(/vec3 CustomToneMapping\( vec3 color \) \{ return color; \}/, `
    vec3 agxContrast(vec3 x){vec3 x2=x*x;vec3 x4=x2*x2;return 15.5*x4*x2-40.14*x4*x+31.96*x4-6.868*x2*x+.4298*x2+.1191*x-.00232;}
    vec3 CustomToneMapping(vec3 color){
      const mat3 toRec2020=mat3(vec3(.6274,.0691,.0164),vec3(.3293,.9195,.0880),vec3(.0433,.0113,.8956));
      const mat3 fromRec2020=mat3(vec3(1.6605,-.1246,-.0182),vec3(-.5876,1.1329,-.1006),vec3(-.0728,-.0083,1.1187));
      const mat3 AgXInset=mat3(vec3(.856627153315983,.137318972929847,.11189821299995),vec3(.0951212405381588,.761241990602591,.0767994186031903),vec3(.0482516061458583,.101439036467562,.811302368396859));
      const mat3 AgXOutset=mat3(vec3(1.1271005818144368,-.1413297634984383,-.14132976349843826),vec3(-.11060664309660323,1.157823702216272,-.11060664309660294),vec3(-.016493938717834573,-.016493938717834257,1.2519364065950405));
      color=toRec2020*color;color*=toneMappingExposure;color=AgXInset*color;color=max(color,1e-10);
      color=log2(color);color=(color+12.47393)/16.5;
      color=.606+(color-.606)*1.2;
      color=clamp(color,0.,1.);color=agxContrast(color);color=AgXOutset*color;
      color=pow(max(vec3(0.),color),vec3(2.2));color=fromRec2020*color;return clamp(color,0.,1.);}`);
  }

  async function drawRecipe(p) {
    await init(); agx();
    const r = E.draw(p);
    let url = r.canvas.toDataURL("image/webp", .9);
    if (!url.startsWith("data:image/webp")) {
      // Safari's canvas (the Mac app's web view) can't write WebP: a JPEG on the face slots' dark grey instead
      const c2 = document.createElement("canvas"); c2.width = W0; c2.height = H0;
      const x = c2.getContext("2d"); x.fillStyle = "#1a1c22"; x.fillRect(0, 0, W0, H0); x.drawImage(r.canvas, 0, 0);
      url = c2.toDataURL("image/jpeg", .9);
    }
    return { url, ms: r.ms, tris: r.tris };
  }
  async function draw(spec) { await init(); agx(); return drawRecipe(recipe(spec)); }
  // one face at a time, in the order asked: a list of faces draws without blocking the screen
  let chain = Promise.resolve();
  function queue(spec) { const job = chain.then(() => new Promise((res) => setTimeout(res, 0))).then(() => draw(spec)); chain = job.catch(() => {}); return job; }
  window.AFMHeads = { draw, drawRecipe, queue, recipe: async (spec) => { await init(); return recipe(spec); } };
})();
