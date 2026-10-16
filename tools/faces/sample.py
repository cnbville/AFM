"""Generate N faces (fixed seed), read each one, save codes + readings to out/pool.npz every 100 faces."""
import os
import sys
import time

import cv2
import numpy as np
import torch

sys.path.insert(0, os.path.dirname(__file__))
from gen import Gen  # noqa: E402
from label import Reader  # noqa: E402

torch.set_num_threads(4)
N = int(sys.argv[1]) if len(sys.argv) > 1 else 1600
OUT = os.path.join(os.environ.get('AFM_FACES_DIR', '.'), 'out')
os.makedirs(os.path.join(OUT, 'pool'), exist_ok=True)

g = Gen()
r = Reader()
z = torch.randn(N, 512, generator=torch.Generator().manual_seed(2026))
W = g.z_to_w(z).numpy()
keys = ['sex_m', 'age_if', 'age_df', 'yaw', 'pitch', 'det', 'fullness', 'jaw_w', 'smile', 'beard', 'skin_L']
rows, race, emb, ok = [], [], [], []
t0 = time.time()
B = 4
for b in range(0, N, B):
    imgs = g.synth(W[b:b + B], 512)
    for j, im in enumerate(imgs):
        i = b + j
        small = cv2.resize(im, (256, 256), interpolation=cv2.INTER_AREA)
        cv2.imwrite(os.path.join(OUT, 'pool', f'{i:05d}.jpg'), cv2.cvtColor(small, cv2.COLOR_RGB2BGR),
                    [cv2.IMWRITE_JPEG_QUALITY, 88])
        o = r.read(small)
        if o is None:
            ok.append(False)
            rows.append([np.nan] * len(keys))
            race.append(np.full(6, np.nan, np.float32))
            emb.append(np.zeros(512, np.float32))
            continue
        ok.append(True)
        rows.append([o[k] for k in keys])
        race.append(o['race'])
        emb.append(o['emb'])
    done = b + B
    if done % 100 == 0 or done >= N:
        n = len(rows)
        np.savez(os.path.join(OUT, 'pool.npz'), w=W[:n], ok=np.array(ok), vals=np.array(rows, np.float32),
                 keys=np.array(keys), race=np.array(race), emb=np.array(emb))
        el = time.time() - t0
        print(f'{n}/{N} faces, {el / 60:.1f} min, {el / n:.2f} s each', flush=True)
