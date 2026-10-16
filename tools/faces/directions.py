"""Find edit directions in the face model's code space (InterFaceGAN-style): fit a linear model from the
512-number code to each reading over the sample pool, then make each edit direction independent of the
others (conditional manipulation), so changing age does not change heritage, glasses, smile or pose."""
import os
import sys

import numpy as np
from sklearn.linear_model import LogisticRegression, RidgeCV
from sklearn.model_selection import cross_val_score

OUT = os.path.join(os.environ.get('AFM_FACES_DIR', '.'), 'out')


def load():
    d = np.load(os.path.join(OUT, 'pool.npz'))
    keys = [str(k) for k in d['keys']]
    vals = {k: d['vals'][:, i] for i, k in enumerate(keys)}
    n = len(d['w'])
    p = np.load(os.path.join(OUT, 'pool_parse.npz'))
    for i, k in enumerate(p['keys']):
        vals[str(k)] = p['vals'][:n, i]
    race = d['race']
    names = ['asian', 'indian', 'black', 'white', 'mideast', 'latino']
    for i, nm in enumerate(names):
        pr = np.clip(race[:, i], 1e-4, 1 - 1e-4)
        vals['race_' + nm] = np.log(pr / (1 - pr))
    return d['w'], d['ok'], vals, race, d['emb']


def fit(Wc, y, mask, kind='ridge'):
    X, t = Wc[mask], y[mask]
    if kind == 'logit':
        m = LogisticRegression(C=0.05, max_iter=3000)
        score = cross_val_score(m, X, t, cv=5, scoring='accuracy').mean()
        m.fit(X, t)
        coef = m.coef_[0]
    else:
        m = RidgeCV(alphas=np.logspace(0, 4, 13))
        score = cross_val_score(m, X, t, cv=5, scoring='r2').mean()
        m.fit(X, t)
        coef = m.coef_
    return coef / np.linalg.norm(coef), float(score), int(mask.sum())


def orthogonal(v, others):
    basis = []
    for o in others:
        if o is None:
            continue
        u = o.copy()
        for b in basis:
            u -= (u @ b) * b
        nrm = np.linalg.norm(u)
        if nrm > 1e-6:
            basis.append(u / nrm)
    u = v.copy()
    for b in basis:
        u -= (u @ b) * b
    return u / np.linalg.norm(u)


def main():
    W, ok, v, race, emb = load()
    mu = W[ok].mean(0)
    Wc = W - mu
    good = ok & (v['det'] > 0.6) & (np.abs(v['yaw']) < 35)
    male = good & (v['sex_m'] > 0.5)
    adult = good & (v['age_if'] >= 17)
    raw, report = {}, {}

    def add(name, y, mask, kind='ridge'):
        if kind == 'logit' and min((y[mask] > .5).sum(), (y[mask] <= .5).sum()) < 5:
            report[name] = ('skipped: too few of one class', int(mask.sum()), kind)
            return
        raw[name], s, n = fit(Wc, y, mask, kind)
        report[name] = (s, n, kind)

    add('age', v['age_if'], good & (v['age_if'] >= 6) & (v['age_if'] <= 75))
    add('sex', v['sex_m'], good, 'logit')
    for nm in ['black', 'white', 'asian', 'latino', 'indian', 'mideast']:
        add('race_' + nm, v['race_' + nm], good)
    add('glasses', (v['glasses'] > 0.01).astype(float), good, 'logit')
    add('hat', (v['hat'] > 0.01).astype(float), good, 'logit')
    add('smile', v['smile'], adult)
    add('yaw', v['yaw'], good)
    add('pitch', v['pitch'], good)
    add('fullness', v['fullness'], adult)
    add('beard', v['beard'], male & (v['age_if'] >= 17))
    add('top_hair', v['top_hair'], male & (v['age_if'] >= 17))

    races = ['race_' + n for n in ['black', 'white', 'asian', 'latino', 'indian', 'mideast']]
    common = ['sex', 'glasses', 'hat', 'smile', 'yaw', 'pitch'] + races
    R = lambda names: [raw.get(n) for n in names]  # noqa: E731
    final = {
        'age': orthogonal(raw['age'], R(common + ['fullness', 'beard', 'top_hair'])),
        'fullness': orthogonal(raw['fullness'], R(common + ['age', 'beard', 'top_hair'])),
        'beard': orthogonal(raw['beard'], R(common + ['age', 'fullness', 'top_hair'])),
        'top_hair': orthogonal(raw['top_hair'], R(common + ['age', 'fullness', 'beard'])),
        'smile': orthogonal(raw['smile'], R(['sex', 'glasses', 'hat', 'yaw', 'pitch', 'age', 'fullness', 'beard'] + races)),
    }
    # calibration: reading units per unit step along each final direction (over the masks used above)
    masks = {'age': good & (v['age_if'] >= 6) & (v['age_if'] <= 75), 'fullness': adult,
             'beard': male & (v['age_if'] >= 17), 'top_hair': male & (v['age_if'] >= 17), 'smile': adult}
    target = {'age': v['age_if'], 'fullness': v['fullness'], 'beard': v['beard'], 'top_hair': v['top_hair'],
              'smile': v['smile']}
    slopes, sds = {}, {}
    for k, d in final.items():
        p = Wc[masks[k]] @ d
        t = target[k][masks[k]]
        slope = np.polyfit(p, t, 1)[0]
        slopes[k] = float(slope)
        sds[k] = float(np.std(t))
        r = np.corrcoef(p, t)[0, 1]
        report[k + ' (final)'] = (float(r), int(masks[k].sum()), f'slope {slope:.2f}/unit, sd {np.std(t):.3f}')
    np.savez(os.path.join(OUT, 'directions.npz'), mu=mu, **{f'd_{k}': d for k, d in final.items()},
             **{f'raw_{k}': d for k, d in raw.items()}, slopes=np.array([slopes[k] for k in final]),
             sds=np.array([sds[k] for k in final]), names=np.array(list(final)))
    for k, r in report.items():
        print(f'{k:16s} {r}')
    print('pool:', int(ok.sum()), 'faces read;', int(good.sum()), 'usable;', int(male.sum()), 'male')
    top = race[good].argmax(1)
    names = ['asian', 'indian', 'black', 'white', 'mideast', 'latino']
    print('heritage shares (usable):', {names[i]: int((top == i).sum()) for i in range(6)})
    print('adult males by heritage:', {names[i]: int(((top == i) & (v['sex_m'][good] > .5) &
                                                      (v['age_if'][good] >= 17)).sum()) for i in range(6)})


if __name__ == '__main__':
    sys.exit(main())
