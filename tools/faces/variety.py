"""How varied are the faces within each heritage? Compares every pair of adult male faces with the face
recognition model (ArcFace, buffalo_l). Same-size groups so the numbers compare fairly."""
import json
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(__file__))
from directions import load  # noqa: E402

OUT = os.path.join(os.environ.get('AFM_FACES_DIR', '.'), 'out')
NAMES = ['asian', 'indian', 'black', 'white', 'mideast', 'latino']
SAME = 0.40  # assumed cut-off where a face recognition system would call two photos the same person


def group(emb, race, v, ok, her):
    k = NAMES.index(her)
    m = ok & (v['sex_m'] > .5) & (v['age_if'] >= 17) & (np.nan_to_num(race[:, k]) > 0.5)
    return emb[m], v['skin_L'][m]


def stats(E, L, n):
    E, L = E[:n], L[:n]
    S = E @ E.T
    iu = np.triu_indices(len(E), 1)
    s = S[iu]
    return dict(faces=len(E), mean_similarity=round(float(s.mean()), 3), lookalike_pairs=int((s > SAME).sum()),
                pairs=int(len(s)), lookalike_share=round(float((s > SAME).mean()) * 100, 2),
                closest=round(float(s.max()), 3), skin_spread=round(float(np.std(L)), 1))


def main():
    W, ok, v, race, emb = load()
    groups = {}
    for her in ['white', 'black', 'latino', 'asian']:
        E, L = group(emb, race, v, ok, her)  # from the random pool
        path = os.path.join(OUT, f'pool_{her}.npz')
        if os.path.exists(path):  # plus the heritage-targeted pool
            d = np.load(path)
            pv = {str(k): d['vals'][:, i] for i, k in enumerate(d['keys'])}
            E2, L2 = group(d['emb'], d['race'], pv, d['ok'], her)
            E, L = np.concatenate([E, E2]), np.concatenate([L, L2])
        groups[her] = (E, L)
    print({k: len(g[0]) for k, g in groups.items()})
    big = {k: g for k, g in groups.items() if len(g[0]) >= 30}
    n = min(len(g[0]) for g in big.values())  # same group size for a fair comparison
    out = {her: stats(E, L, n if her in big else len(E)) for her, (E, L) in groups.items() if len(E) >= 10}
    print(json.dumps(out, indent=1))
    json.dump(out, open(os.path.join(OUT, 'variety.json'), 'w'), indent=1)


if __name__ == '__main__':
    main()
