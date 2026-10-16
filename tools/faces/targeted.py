"""Heritage-targeted sampling. The face model learned from Flickr photos, so some heritages are rare in a plain
random draw. Instead of editing a face's heritage (which looks fake), draw many codes, score each code with
the fitted heritage, sex, age and glasses directions (cheap: no picture needed), and only draw the pictures of
the best matches. Every face stays a natural sample of the model. The picked faces are then read and checked.

Usage: targeted.py <heritage> <n_codes> <n_draw>   (heritage: black, latino, asian, white)"""
import os
import sys

import cv2
import numpy as np
import torch

sys.path.insert(0, os.path.dirname(__file__))
from gen import Gen  # noqa: E402
from label import Reader  # noqa: E402
from parse import Parser  # noqa: E402

torch.set_num_threads(4)
OUT = os.path.join(os.environ.get('AFM_FACES_DIR', '.'), 'out')

her, n_codes, n_draw = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
D = np.load(os.path.join(OUT, 'directions.npz'))
mu = D['mu']
g = Gen()
seed = {'black': 11, 'latino': 12, 'asian': 13, 'white': 14}[her]
z = torch.randn(n_codes, 512, generator=torch.Generator().manual_seed(9000 + seed))
W = np.concatenate([g.z_to_w(z[i:i + 5000]).numpy() for i in range(0, n_codes, 5000)])
Wc = W - mu
proj = lambda k: Wc @ D['raw_' + k]  # noqa: E731
zs = lambda x: (x - x.mean()) / x.std()  # noqa: E731
score = 2.0 * zs(proj('race_' + her)) + 1.0 * zs(proj('sex')) - 1.0 * zs(proj('glasses')).clip(0) \
    - 0.5 * np.abs(zs(proj('age')) + 0.4) - 0.5 * np.abs(zs(proj('yaw')))
if 'raw_hat' in D.files:
    score -= 1.0 * zs(proj('hat')).clip(0)
top = np.argsort(-score)[:n_draw]
r, prs = Reader(), Parser()
keys = ['sex_m', 'age_if', 'age_df', 'yaw', 'pitch', 'det', 'fullness', 'jaw_w', 'smile', 'beard', 'skin_L']
pkeys = ['glasses', 'hat', 'hair', 'top_hair', 'earring']
os.makedirs(os.path.join(OUT, f'pool_{her}'), exist_ok=True)
rows, race, emb, ok = [], [], [], []
for j, i in enumerate(top):
    im = g.synth(W[i][None], 512)[0]
    small = cv2.resize(im, (256, 256), interpolation=cv2.INTER_AREA)
    cv2.imwrite(os.path.join(OUT, f'pool_{her}', f'{j:05d}.jpg'), cv2.cvtColor(small, cv2.COLOR_RGB2BGR),
                [cv2.IMWRITE_JPEG_QUALITY, 88])
    o = r.read(small)
    pf = prs.features(small)
    if o is None:
        ok.append(False)
        rows.append([np.nan] * (len(keys) + len(pkeys)))
        race.append(np.full(6, np.nan, np.float32))
        emb.append(np.zeros(512, np.float32))
        continue
    ok.append(True)
    rows.append([o[k] for k in keys] + [pf[k] for k in pkeys])
    race.append(o['race'])
    emb.append(o['emb'])
np.savez(os.path.join(OUT, f'pool_{her}.npz'), w=W[top], ok=np.array(ok), vals=np.array(rows, np.float32),
         keys=np.array(keys + pkeys), race=np.array(race), emb=np.array(emb))
names = ['asian', 'indian', 'black', 'white', 'mideast', 'latino']
rc = np.array(race)[np.array(ok)]
topr = rc.argmax(1)
print(her, f'{n_codes} codes scored, {n_draw} drawn; read as:', {names[k]: int((topr == k).sum()) for k in range(6)})
