"""Pick players from the pool by heritage and build, draw each at 17, 22, 28 and 34 by moving his code along
the age, build, beard and hairline directions, make media-day headshots, and measure identity drift."""
import json
import os
import sys

import cv2
import numpy as np
import torch
from PIL import Image, ImageDraw, ImageFont

sys.path.insert(0, os.path.dirname(__file__))
from directions import load  # noqa: E402
from gen import Gen  # noqa: E402
from label import Reader, RACES  # noqa: E402
from parse import Parser  # noqa: E402

torch.set_num_threads(4)
OUT = os.path.join(os.environ.get('AFM_FACES_DIR', '.'), 'out')
AGES = [17, 22, 28, 34]
READER_BIAS = {17: 4, 22: 4, 28: 5, 34: 6}  # by eye: the reader reads young adults a few years old

# Names come from the game's own name code (world/names.rs, born 2009). Weights follow a real career arc.
# beard: target beard level per age in standard deviations around the adult male mean (genes + choice).
# hairline: change in hair above the brows per age, in standard deviations (negative = receding).
PLAYERS = [
    dict(key='prude', name='Caleb Prude', pos='OT', heritage='black', ht='6-6',
         w={17: 268, 22: 311, 28: 320, 34: 306}, build_k=10.0, build_sd=6.0,
         beard={17: -1.6, 22: -0.2, 28: 1.4, 34: 1.6}, hairline={17: 0, 22: 0, 28: 0, 34: 0},
         teams={17: ['Bessemer City HS', '#0b3d2e'], 22: ['Alabama', '#9e1b32'], 28: ['Ravens', '#241773'],
                34: ['Chiefs', '#c8102e']}),
    dict(key='mcspadden', name='Derek McSpadden', pos='QB', heritage='white', ht='6-4',
         w={17: 192, 22: 217, 28: 224, 34: 228}, build_k=2.0, build_sd=0.0,
         beard={17: -1.6, 22: -0.8, 28: 0.0, 34: 0.4}, hairline={17: 0, 22: 0, 28: -0.5, 34: -1.1},
         teams={17: ['Waukee HS', '#4b2e83'], 22: ['Iowa State', '#c8102e'], 28: ['Packers', '#203731'],
                34: ['Jets', '#125740']}),
    dict(key='meza', name='Leonardo Meza', pos='LB', heritage='latino', ht='6-2',
         w={17: 214, 22: 237, 28: 245, 34: 241}, build_k=5.0, build_sd=1.2,
         beard={17: -1.4, 22: 0.4, 28: 0.9, 34: 1.0}, hairline={17: 0, 22: 0, 28: 0, 34: -0.2},
         teams={17: ['Edinburg North HS', '#002d72'], 22: ['Texas A&M', '#500000'], 28: ['Texans', '#03202f'],
                34: ['Cowboys', '#041e42']}),
]


def dirs():
    d = np.load(os.path.join(OUT, 'directions.npz'))
    names = [str(n) for n in d['names']]
    return {n: (d[f'd_{n}'], float(d['slopes'][i]), float(d['sds'][i])) for i, n in enumerate(names)}


def load_pool(her):
    """The plain random pool plus the heritage-targeted pool for this heritage, if one was drawn.
    Returns image paths alongside, since the two pools keep their pictures in different folders."""
    W, ok, v, race, emb = load()
    paths = [os.path.join(OUT, 'pool', f'{i:05d}.jpg') for i in range(len(W))]
    path = os.path.join(OUT, f'pool_{her}.npz')
    if os.path.exists(path):
        d = np.load(path)
        tv = {str(k): d['vals'][:, i] for i, k in enumerate(d['keys'])}
        v = {k: np.concatenate([v[k], tv[k]]) for k in tv}
        W = np.concatenate([W, d['w']])
        ok = np.concatenate([ok, d['ok']])
        race = np.concatenate([race, d['race']])
        paths += [os.path.join(OUT, f'pool_{her}', f'{i:05d}.jpg') for i in range(len(d['w']))]
    return paths, W, ok, v, race


def candidates(p, W, ok, v, race, ref, n=10):
    hi = {'black': 2, 'white': 3, 'latino': 5, 'asian': 0}[p['heritage']]
    good = ok & (v['det'] > 0.7) & (np.abs(v['yaw']) < 15) & (np.abs(v['pitch']) < 15)
    good &= (v['sex_m'] > .5) & (v['age_if'] >= 19) & (v['age_if'] <= 40)
    good &= (v['glasses'] < 0.005) & (v['hat'] < 0.005)
    prob = np.nan_to_num(race[:, hi])
    good &= (prob > 0.3) & (np.nan_to_num(race).argmax(1) == hi)
    idx = np.where(good)[0]
    return idx[np.argsort(-prob[idx])][:n]


def media_day(rgb, lab, team, color, label):
    h, w = lab.shape
    fg = (~np.isin(lab, [0, 18])).astype(np.float32)
    fg = cv2.GaussianBlur(fg, (0, 0), 2.0)[..., None]
    c = np.array([int(color[i:i + 2], 16) for i in (1, 3, 5)], np.float32)
    top, bot = np.minimum(c * 1.35 + 30, 255), c * 0.55
    grad = (top[None, :] * (1 - np.linspace(0, 1, h)[:, None]) + bot[None, :] * np.linspace(0, 1, h)[:, None])
    bg = np.repeat(grad[:, None, :], w, 1)
    img = rgb.astype(np.float32)
    cloth = (lab == 16)
    if cloth.any():
        lum = img[..., :3].mean(-1)
        rel = lum / max(np.median(lum[cloth]), 1)
        jersey = np.clip(c[None, None, :] * rel[..., None] * 1.05, 0, 255)
        m = cv2.GaussianBlur(cloth.astype(np.float32), (0, 0), 1.5)[..., None]
        img = img * (1 - m) + jersey * m
    out = (img * fg + bg * (1 - fg)).clip(0, 255).astype(np.uint8)
    pil = Image.fromarray(out)
    d = ImageDraw.Draw(pil, 'RGBA')
    try:
        font = ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf', 30)
    except OSError:
        font = ImageFont.load_default()
    tw = d.textlength(label.upper(), font=font)
    d.text(((w - tw) / 2, 14), label.upper(), font=font, fill=(255, 255, 255, 60))
    return np.array(pil)


def main(mode):
    W, ok, v, race, emb = load()
    adult_m = ok & (v['sex_m'] > .5) & (v['age_if'] >= 17)
    ref = v['fullness'][adult_m]
    if mode == 'candidates':
        sheet = []
        for p in PLAYERS:
            paths, pW, pok, pv, prace = load_pool(p['heritage'])
            idx = candidates(p, pW, pok, pv, prace, ref)
            print(p['name'], list(map(int, idx)))
            row = [cv2.imread(paths[i])[:, :, ::-1] for i in idx]
            row += [np.zeros((256, 256, 3), np.uint8)] * (10 - len(row))
            sheet.append(np.concatenate(row, 1))
        Image.fromarray(np.concatenate(sheet, 0)).save(os.path.join(OUT, 'candidates.jpg'))
        return
    picks = json.loads(sys.argv[2])  # {"prude": pool_index, ...}
    D = dirs()
    g, r, prs = Gen(), Reader(), Parser()
    male_adult = ok & (v['sex_m'] > .5) & (v['age_if'] >= 17)
    beard_mu, beard_sd = np.nanmean(v['beard'][male_adult]), np.nanstd(v['beard'][male_adult])
    full_sd = D['fullness'][2]
    results = []
    os.makedirs(os.path.join(OUT, 'shots'), exist_ok=True)
    for p in PLAYERS:
        i0 = picks[p['key']]
        paths, pW, pok, pv, prace = load_pool(p['heritage'])
        pname = os.path.relpath(paths[i0], OUT)
        w0 = pW[i0].copy()
        base_age = float(pv['age_if'][i0])
        base_beard_z = (float(pv['beard'][i0]) - beard_mu) / beard_sd
        rows = []
        for a in AGES:
            dfull = (p['build_sd'] + p['build_k'] * (p['w'][a] - p['w'][22]) / p['w'][22]) * full_sd / D['fullness'][1]
            dbeard = (p['beard'][a] - base_beard_z) * beard_sd / D['beard'][1]
            dhair = p['hairline'][a] * D['top_hair'][2] / D['top_hair'][1]
            # Age: closed loop on the age reader. The reader reads these adult faces older than they look
            # (about +6 years by eye), so aim for target + READER_BIAS. Faces respond to about half the
            # step the pool regression predicts, so each correction uses half the slope.
            goal = a + READER_BIAS[a]
            dage = (goal - base_age) / D['age'][1] * 2.0
            for _ in range(4):
                w = w0 + dage * D['age'][0] + dfull * D['fullness'][0] + dbeard * D['beard'][0] + dhair * D['top_hair'][0]
                img = g.synth(w[None], 512)[0]
                o = r.read(cv2.resize(img, (256, 256), interpolation=cv2.INTER_AREA))
                if o is None or abs(o['age_if'] - goal) <= 2:
                    break
                dage += (goal - o['age_if']) / (D['age'][1] * 0.5)
            lab = prs.labels(img)
            team, color = p['teams'][a]
            shot = media_day(img, lab, team, color, team)
            fn = f"{p['key']}_{a}.jpg"
            Image.fromarray(shot).save(os.path.join(OUT, 'shots', fn), quality=90)
            Image.fromarray(img).save(os.path.join(OUT, 'shots', 'raw_' + fn), quality=90)
            rows.append(dict(age=a, file=fn, read_age=int(o['age_if']) if o else None,
                             read_age2=round(float(o['age_df']), 1) if o else None,
                             heritage=RACES[int(o['race'].argmax())] if o else None,
                             heritage_p=round(float(o['race'].max()), 2) if o else None,
                             emb=o['emb'].tolist() if o else None, weight=p['w'][a], team=team,
                             steps=dict(age=round(float(dage), 2), build=round(float(dfull), 2), beard=round(float(dbeard), 2),
                                        hair=round(float(dhair), 2))))
        ref = np.array(rows[1]['emb'])
        for rw in rows:
            rw['same_person'] = round(float(np.dot(ref, np.array(rw['emb']))), 3) if rw['emb'] else None
            del rw['emb']
        results.append(dict(key=p['key'], name=p['name'], pos=p['pos'], ht=p['ht'], heritage=p['heritage'],
                            pool=pname, pool_index=int(i0), base_age=base_age, rows=rows))
        print(p['name'], [(rw['age'], rw['read_age'], rw['heritage'], rw['same_person']) for rw in rows], flush=True)
    json.dump(results, open(os.path.join(OUT, 'results.json'), 'w'), indent=1)


if __name__ == '__main__':
    main(sys.argv[1])
