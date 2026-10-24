# Pack the drawn heads' data for the game (faces F14): the base head with the GNM face targets, the face-shape sampler, the
# jersey and the hair library, from the v7 face pipeline (the "experiments kit" in the project folder,
# afm/faces-3d/face-variety/gnm/experiments-kit/; see tools/faces/heads/README.md).
# usage: python3 pack.py KIT_DIR GROOM_DIR OUT_DIR   (OUT_DIR is app/ui/screens/heads)
import base64, json, re, sys
import numpy as np

kit, groom, out = sys.argv[1:4]

# ---- the base head: MakeHuman's mesh (CC0) with the GNM Head targets (Apache 2.0), from the head export page
s = open(f'{kit}/export_v5.html').read()
j = s.find('const MH_META=') + len('const MH_META=')
M, _ = json.JSONDecoder().raw_decode(s[j:])
i = s.find('const MH_B64="') + len('const MH_B64="'); k = s.find('"', i)
raw = base64.b64decode(s[i:k] + '=' * (-(k - i) % 4))
SZ = {'float32': 4, 'uint32': 4, 'uint16': 2, 'int16': 2}
# what a GNM face needs: the body by age, muscle and weight, the GNM shapes, the masks, the neck and the age targets
KEEP = re.compile(r'^(macrodetails/|gnm/|afm/|neck/neck-scale-horiz|head/head-age|eyes/[lr]-eye-bag-(incr|decr)$)')
blobs, off, o = [], {}, 0
for name, (a, n, t) in M['off'].items():
    if name.startswith(('i:', 'd:')) and not KEEP.match(name[2:]): continue
    b = raw[a:a + n * SZ[t]]; blobs.append(b); off[name] = [o, n, t]; o += len(b)
    if o % 4: blobs.append(b'\0' * (4 - o % 4)); o += 4 - o % 4
meta = {k_: v for k_, v in M.items() if k_ not in ('off', 'targets')}
meta['off'] = off
meta['targets'] = [t_ for t_ in M['targets'] if KEEP.match(t_)]
open(f'{out}/head.bin', 'wb').write(b''.join(blobs))
json.dump(meta, open(f'{out}/head.json', 'w'), separators=(',', ':'))
print('head', round(o / 1e6, 2), 'MB,', len(meta['targets']), 'targets')

# ---- the face-shape sampler: GNM's identity decoder (a small network), each heritage group's blend and shift, the widened
# spread (spread_fit.py), MakeHuman's own fit (cW) and the fat each sampled face already carries (fatproj.py)
dec = np.load(f'{kit}/gnm_data/id_decoder.npz'); reg = np.load(f'{kit}/gnm/out/reg.npz')
G = json.load(open(f'{kit}/gnm/out/groups.json')); SP = json.load(open(f'{kit}/gnm/out/spread.json'))
FP = json.load(open(f'{kit}/gnm/out/fatproj.json'))
COMP = list(range(170)) + [250, 251, 252]
wb, woff, wo = [], {}, 0
for nm in ['k4', 'b4', 'k5', 'b5', 'k6', 'b6', 'k7', 'b7', 'k8', 'b8']:
    a = dec[nm].astype(np.float16); wb.append(a.tobytes()); woff[nm] = [wo, list(a.shape)]; wo += a.nbytes
open(f'{out}/shape.bin', 'wb').write(b''.join(wb))
shape = dict(off=woff, comp=COMP, cw=reg['cW'][COMP].round(5).tolist(), T=SP['T'], sd=SP['real_shape_sd'],
             groups={h: dict(blend=g['blend'], shift=np.round(g['shift'], 5).tolist(), A=np.round(SP['groups'][h]['A'], 5).tolist(),
                             fat_mean=np.round(FP['mean'][h], 5).tolist()) for h, g in G.items()},
             fat_a=FP['a'], classes=['middle_eastern', 'asian', 'white', 'black'])
json.dump(shape, open(f'{out}/shape.json', 'w'), separators=(',', ':'))
print('shape', round(wo / 1e6, 2), 'MB of weights')

# ---- the jersey: one cloth mesh made in Blender over shoulder pads, bound to the body like the hair
js = open(f'{kit}/bl/lib_jersey.js').read()
J = json.loads(js[js.find('{'):js.rfind('}') + 1])
json.dump(J, open(f'{out}/jersey.json', 'w'), separators=(',', ':'))

# ---- the hair library (bake_lib.py): copied as baked
import shutil
shutil.copy(f'{groom}/groom.bin', f'{out}/groom.bin')
gj = json.load(open(f'{groom}/groom.json'))
json.dump(gj, open(f'{out}/groom.json', 'w'), separators=(',', ':'))
print('groom', len(gj['lib']), 'entries')
