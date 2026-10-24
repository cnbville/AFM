# Bake the v7 grooms into the game's hair library: every haircut and beard is groomed once, with the v7 Python grooms, on the
# average GNM head; a sample of its strands is kept and bound to the head mesh at the root (triangle + barycentric), the strand
# itself stored as int8 steps in the root's own frame (edge, cross, normal). The game regrows them on any head in the page.
# usage (from the kit's bl/ folder, which holds the v7 groom modules): python3 bake_lib.py ../heads/1/head_21.json out_dir [only,keys]
import sys, json, math, time
import numpy as np
from scipy.spatial import cKDTree
import groom as g
import groom_v5 as g5
import groom_v6 as g6
import groom_v7 as g7

src, out = sys.argv[1], sys.argv[2]
ONLY = set(sys.argv[3].split(',')) if len(sys.argv) > 3 else None
H = g.Head(src); p = H.p
P0, N0, T = H.P.copy(), H.N.copy(), H.T
LIP = np.array(H.d['lip'], float).reshape(-1, 2); LIPW = LIP.max(1)
g.hair_mask = g5.hair_mask_hl(0.0)
_bm = g.beard_mask
def _beard(R_, G, style):
    m = _bm(R_, G, style)
    return m * (1 - g.ss(.45, .8, LIPW)) if len(R_) == len(LIPW) else m
g.beard_mask = _beard
up_ = (LIP[:, 0] > .45) & (LIP[:, 1] < .05)
Pu = H.P[up_]; o = np.argsort(Pu[:, 0]); Pu = Pu[o]
bins = np.array_split(np.arange(len(Pu)), 14)
H.lip_top = (np.array([Pu[b, 0].mean() for b in bins if len(b)]), np.array([Pu[b, 1].max() for b in bins if len(b)]))
FULLG = {'dens': 1.0, 'patch': 0.0, 'cheek': 1.0, 'neck': 1.0}
p['rec'] = 0.0

cen = H.P[T].mean(1); ctree = cKDTree(cen)

def bind(pts, k, n, seed):
    """Keep n strands, resampled to k points along their length, bound at the root."""
    pts = np.asarray(pts, float)
    ok = np.isfinite(pts).all((1, 2)) & (np.linalg.norm(pts[:, -1] - pts[:, 0], axis=1) > 1e-5)
    pts = pts[ok]
    rng = np.random.default_rng(seed)
    if len(pts) > n: pts = pts[rng.choice(len(pts), n, replace=False)]
    # arc-length resample
    seg = np.linalg.norm(np.diff(pts, axis=1), axis=2); cum = np.concatenate([np.zeros((len(pts), 1)), np.cumsum(seg, 1)], 1)
    tot = np.maximum(cum[:, -1:], 1e-9); u = np.linspace(0, 1, k)[None, :] * tot
    q = np.empty((len(pts), k, 3))
    for i in range(len(pts)):
        for c in range(3): q[i, :, c] = np.interp(u[i], cum[i], pts[i, :, c])
    root = q[:, 0]
    _, tri = ctree.query(root, k=1)
    a, b_, c_ = H.P[T[tri, 0]], H.P[T[tri, 1]], H.P[T[tri, 2]]
    v0, v1, v2 = b_ - a, c_ - a, root - a
    d00 = (v0 * v0).sum(1); d01 = (v0 * v1).sum(1); d11 = (v1 * v1).sum(1); d20 = (v2 * v0).sum(1); d21 = (v2 * v1).sum(1)
    den = d00 * d11 - d01 * d01; bv = (d11 * d20 - d01 * d21) / den; bw = (d00 * d21 - d01 * d20) / den
    bv = np.clip(bv, 0, 1); bw = np.clip(bw, 0, 1 - bv); bu = 1 - bv - bw
    Rp = bu[:, None] * a + bv[:, None] * b_ + bw[:, None] * c_
    Nn = g.nrm(bu[:, None] * H.N[T[tri, 0]] + bv[:, None] * H.N[T[tri, 1]] + bw[:, None] * H.N[T[tri, 2]])
    e = v0; t = g.nrm(e - (e * Nn).sum(1, keepdims=True) * Nn); bb = np.cross(Nn, t)
    q = q - root[:, None] + Rp[:, None]                       # snap the root onto the surface point
    d = np.diff(q, axis=1)                                    # steps
    loc = np.stack([(d * t[:, None]).sum(2), (d * bb[:, None]).sum(2), (d * Nn[:, None]).sum(2)], 2)
    lift = ((q[:, 0] - Rp) * Nn).sum(1)
    sc = max(float(np.abs(loc).max()) / 127, 1e-6)
    dq = np.clip(np.round(loc / sc), -127, 127).astype(np.int8)
    bc = np.round(np.stack([bv, bw], 1) * 255).astype(np.uint8)
    rref = (bu[:, None] * H.R[T[tri, 0]] + bv[:, None] * H.R[T[tri, 1]] + bw[:, None] * H.R[T[tri, 2]])
    return dict(n=int(len(q)), k=int(k), sc=sc, tri=tri.astype(np.uint16), bc=bc, d=dq), rref

BLOBS = []; META = {}; OFF = [0]
def blob(a):
    a = np.ascontiguousarray(a); o = OFF[0]; b = a.tobytes(); BLOBS.append(b); OFF[0] += len(b)
    pad = (-OFF[0]) % 4
    if pad: BLOBS.append(b'\0' * pad); OFF[0] += pad
    return [o, int(a.size), str(a.dtype)]

def layer(pts, k, n, seed, w, ao, tile, rad=None):
    L, rref = bind(pts, k, n, seed)
    out = dict(n=L['n'], k=L['k'], sc=L['sc'], tri=blob(L['tri']), bc=blob(L['bc']), d=blob(L['d']), w=w, ao=ao, tile=tile)
    return out

def ropes(axes, rad, k, n, seed):
    L, _ = bind(axes, k, n, seed)
    r = np.asarray(rad, float)
    if r.ndim == 1: r = np.repeat(r[:, None], k, 1)
    if r.shape[1] != k:
        t = np.linspace(0, 1, r.shape[1]); r = np.stack([np.interp(np.linspace(0, 1, k), t, x) for x in r])
    return dict(n=L['n'], k=L['k'], sc=L['sc'], tri=blob(L['tri']), bc=blob(L['bc']), d=blob(L['d']), r=blob(np.round(np.clip(r[:L['n']] / .0004, 0, 255)).astype(np.uint8)))

def run(name, fn):
    if ONLY and name not in ONLY: return
    t0 = time.time()
    for a in ('ropes', 'loc_axes', 'fade', 'shaved', 'beard_base'):
        if hasattr(H, a): delattr(H, a)
    H.P, H.N = P0.copy(), N0.copy()
    e = fn()
    META[name] = e
    print(name, round(time.time() - t0, 1), 's', {k: (v['n'] if isinstance(v, dict) and 'n' in v else '') for k, v in e.items()}, flush=True)

def hm(lineup, rec=0.0):
    return g.hair_mask(H.R, rec, lineup)

def two(pts, rad, nA, nB, k, seed, wA, wB, tile, aoA=.6, aoB=1.0):
    return [layer(pts, k, nA, seed, wA, aoA, tile), layer(pts, k, nB, seed + 1, wB, aoB, tile)]

# ---------------------------------------------------------------- haircuts (straight and wavy hair: tile 0; coily: tile 1)
def e_shaved():
    pts, rad = g7.style_shaved(H, hm(1.0)); return dict(layers=[layer(pts, 3, 14000, 1, .0035, .9, 1)], skin=dict(h=.002, d=.55, lineup=1, shaved=1))
def e_buzz():
    rng = np.random.default_rng(36)
    P, R, N, mk = H.sample_roots((hm(1.0) > .5).astype(float), 40000, 36)
    crown = g.crown_point(H); d = g.flow_dir(H, P, crown, gravity=.2)
    L = .035 + .025 * rng.random(len(P)); s = np.linspace(0, 1, 4)
    dirc = g.nrm(N * .55 + d)
    pts = P[:, None] + dirc[:, None] * (L[:, None, None] * s[None, :, None]) + (N[:, None] * .004 * s[None, :, None] ** 2)
    return dict(layers=[layer(pts, 4, 16000, 2, .0045, .8, 0)], skin=dict(h=.006, d=.9, lineup=.6))
def e_crop():
    pts, rad = g.style_crop(H); return dict(layers=two(pts, rad, 3500, 9000, 8, 3, .03, .006, 0), skin=dict(h=.008, d=1, lineup=.7))
def e_crop_fade():
    pts, rad = g5.style_crop_fade(H, hm(.6)); return dict(layers=two(pts, rad, 3500, 9000, 8, 4, .03, .006, 0), skin=dict(h=.008, d=1, lineup=.9, lo=7.50, hi=7.94))
def e_side_part():
    pts, rad = g6.style_side_part_v6(H, hm(.3), side=1); return dict(layers=two(pts, rad, 3500, 9000, 9, 5, .03, .006, 0), skin=dict(h=.008, d=1, lineup=.3))
def e_flow():
    pts, rad = g7.style_flow(H, hm(.3), length=1.3, curl=.4); return dict(layers=two(pts, rad, 3500, 9000, 12, 6, .03, .006, 0), skin=dict(h=.006, d=.95, lineup=0))
def e_flow_curly():
    pts, rad = g7.style_flow(H, hm(.3), length=1.25, curl=.85); return dict(layers=two(pts, rad, 3500, 9000, 14, 7, .03, .006, 0), skin=dict(h=.006, d=.95, lineup=0))
def e_long():
    pts, rad = g7.style_flow(H, hm(.3), length=2.4, curl=.45); return dict(layers=two(pts, rad, 3500, 9000, 14, 8, .03, .006, 0), skin=dict(h=.006, d=.95, lineup=0))
def e_man_bun():
    pts, rad = g6.style_bun(H, g.hair_mask(H.R, 0, .6)); return dict(layers=two(pts, rad, 3500, 9000, 12, 9, .03, .006, 0), skin=dict(h=.006, d=.95, lineup=.3))
def e_mullet():
    pts, rad = g7.style_flow(H, hm(.3), length=1.5, curl=.45)
    r0 = pts[:, 0]; Rr = H.R[cKDTree(H.P).query(r0)[1]]
    top = g.ss(7.55, 7.75, Rr[:, 1]) * g.ss(-.1, .3, Rr[:, 2])          # business in front: top and sides cut short
    keep = 1 - .82 * top
    pts = r0[:, None] + (pts - r0[:, None]) * keep[:, None, None]
    return dict(layers=two(pts, rad, 3500, 9000, 12, 10, .03, .006, 0), skin=dict(h=.006, d=.95, lineup=.3))
def e_two_block():
    pts, rad = g6.style_two_block(H, hm(.6)); return dict(layers=two(pts, rad, 3500, 9000, 9, 11, .03, .006, 0), skin=dict(h=.008, d=1, lineup=.7, lo=7.45, hi=7.6))
def e_curly_top():
    pts, rad = g7.style_curly_top(H, hm(.6)); return dict(layers=two(pts, rad, 3000, 9000, 16, 12, .024, .006, 1), skin=dict(h=.008, d=1, lineup=.9, lo=7.48, hi=7.86))
def e_waves():
    pts, rad = g.style_waves(H, 7.45, 7.75); return dict(layers=[layer(pts, 6, 14000, 13, .0055, .85, 1)], skin=dict(h=.01, d=1, lineup=1, lo=7.45, hi=7.75, waves=1))
def e_sponge_top():
    pts, rad = g7.style_afro_taper(H, hm(1.0), top_h=.30); return dict(layers=two(pts, rad, 3000, 10000, 14, 14, .02, .006, 1), skin=dict(h=.01, d=1, lineup=1, lo=7.42, hi=7.80))
def e_high_top():
    pts, rad = g7.style_afro_taper(H, hm(1.0), top_h=.75, lo=7.50, hi=7.86); return dict(layers=two(pts, rad, 3000, 10000, 14, 15, .02, .006, 1), skin=dict(h=.01, d=1, lineup=1, lo=7.50, hi=7.86))
def e_afro():
    pts, rad = g7.style_afro_taper(H, hm(.6), top_h=.6, lo=6.9, hi=7.3); return dict(layers=two(pts, rad, 3500, 11000, 14, 16, .02, .006, 1), skin=dict(h=.01, d=1, lineup=.6))
def e_twists():
    H.ropes = []; pts, rad = g.style_twists(H, .42)
    ax = np.concatenate([c for c, r in H.ropes]); rr = np.concatenate([r for c, r in H.ropes])
    fz = pts[2 * len(H.ropes[0][0]):]                            # fibres (after the two rope cores)
    return dict(layers=[layer(fz, 10, 6000, 17, .005, .9, 1)], ropes=ropes(ax, rr, 16, 4000, 17), skin=dict(h=.012, d=1, lineup=.6))
def e_locs():
    pts, rad = g7.style_locs(H, g.hair_mask(H.R, 0, .6), length=1.4)
    ax = H.loc_axes; nt, K = ax.shape[:2]; s = np.linspace(0, 1, K)
    prof = (1 + .12 * g.ss(.80, .93, s) - .9 * g.ss(.94, 1.0, s) ** 2)[None, :] * .036 * (.75 + .25 * g.ss(0, .05, s))[None, :]
    fz = pts[nt:]
    return dict(layers=[layer(fz, 12, 7000, 18, .005, .9, 1)], ropes=ropes(ax, np.repeat(prof, nt, 0), 18, nt, 18), skin=dict(h=.012, d=1, lineup=.6))
def e_braids():
    pts, rad = g7.style_locs(H, g.hair_mask(H.R, 0, .6), length=1.7, thick=.026, seed=41)
    ax = H.loc_axes; nt, K = ax.shape[:2]; s = np.linspace(0, 1, K)
    prof = (1 - .5 * g.ss(.92, 1.0, s) ** 2)[None, :] * .026
    fz = pts[nt:]
    return dict(layers=[layer(fz, 12, 4000, 19, .004, .9, 1)], ropes=ropes(ax, np.repeat(prof, nt, 0), 18, nt, 19), skin=dict(h=.006, d=1, lineup=1, parts=1))
def e_cornrows():
    # straight-back cornrows: rows about 2.2 cm apart from the front hairline over the crown to the nape, each a flat braid
    # lying on the scalp (the rope runs along the row, half sunk in), with the scalp showing in the parts between
    rows = []
    xs = np.arange(-.5, .51, .11)
    for x0 in xs:
        a = np.linspace(-.35, 1.0, 40)                    # angle from the forehead over the top to the nape
        dirs = np.stack([np.full_like(a, x0 * .9), np.cos(a * math.pi) * .2 + np.sin(a * math.pi * .95), np.cos(a * math.pi * .92)], 1)
        dirs = g.nrm(dirs)
        q = H.project(H.smap, H.c[None] + dirs, .012)
        rr = cKDTree(H.P).query(q)[1]; m = g.hair_mask(H.R, 0, 1.0)[rr]
        q = q[m > .5]
        if len(q) > 6: rows.append(q)
    K = 24; ax = []
    for q in rows:
        seg = np.r_[0, np.cumsum(np.linalg.norm(np.diff(q, axis=0), axis=1))]; u = np.linspace(0, seg[-1], K)
        ax.append(np.stack([np.interp(u, seg, q[:, c]) for c in range(3)], 1))
    ax = np.array(ax); r = np.full(ax.shape[:2], .028)
    return dict(ropes=ropes(ax, r, K, len(ax), 20), skin=dict(h=0, d=1, lineup=1, rows=1))

# ---------------------------------------------------------------- beards (genes at full coverage; the game culls by his genes)
def beard(kind, coil, ln, seed):
    p['bg'] = dict(FULLG); bs = dict(k=kind, len=ln, chin=.6)
    if kind in ('full', 'boxed') and coil: pts, rad = g.style_beard(H, bs, True)
    elif kind in ('full', 'boxed'): pts, rad = g5.style_beard_v5(H, bs, LIPW)
    else: pts, rad = g6.style_beard_v6(H, bs, LIPW, coil)
    if isinstance(pts, tuple): pts = pts[0]
    k = 4 if kind == 'stubble' else (12 if coil else 8)
    n = 9000 if kind in ('full', 'boxed', 'stubble') else 5000
    return dict(layers=[layer(pts, k, n, seed, .006 if not coil else .005, .8, 3 if not coil else 1)], len=ln)
BEARDS = [('stubble', 'stubble', .025), ('moustache', 'moustache', .07), ('mous_chin', 'mous_chin', .06), ('goatee', 'goatee', .11),
          ('chin_strap', 'chinstrap', .05), ('full_short', 'boxed', .06), ('full', 'full', .15), ('full_long', 'full', .30)]

T0 = time.time()
for nm, fn in [('shaved', e_shaved), ('buzz', e_buzz), ('crop', e_crop), ('crop_fade', e_crop_fade), ('side_part', e_side_part),
               ('flow', e_flow), ('flow_curly', e_flow_curly), ('long', e_long), ('man_bun', e_man_bun), ('mullet', e_mullet),
               ('two_block', e_two_block), ('curly_top', e_curly_top), ('waves', e_waves), ('sponge_top', e_sponge_top),
               ('high_top', e_high_top), ('afro', e_afro), ('twists', e_twists), ('locs', e_locs), ('braids', e_braids), ('cornrows', e_cornrows)]:
    run(nm, fn)
for i, (gk, kind, ln) in enumerate(BEARDS):
    for coil in (False, True):
        run(f'beard_{gk}_{"coily" if coil else "straight"}', lambda: beard(kind, coil, ln, 50 + i))
json.dump(dict(off=None, lib=META, version=1), open(f'{out}/groom.json', 'w'))
open(f'{out}/groom.bin', 'wb').write(b''.join(BLOBS))
print('total', round(OFF[0] / 1e6, 2), 'MB in', round(time.time() - T0, 1), 's')
