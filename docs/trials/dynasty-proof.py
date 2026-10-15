"""Reads dynasty_proof CSV and prints, per variant and year, the numbers that
compare with the real best team (docs/trials/dynasty-proof.md).

rr (top 12 round robin): the leader's average margin against the other 11
(real: best team's rating minus the average of the rest of the top 12), its
title odds (semifinal against #4, final against #2 or #3, from the actual
win rates), and the game luck (sd around the fitted margins).
season (random FBS schedule): best team over the median, spread of team
quality, luck, and how games are decided.
Usage: python3 dynasty-proof.py proof.csv"""
import sys, csv, collections, numpy as np

rows = collections.defaultdict(list)
for r in csv.DictReader(open(sys.argv[1])):
    rows[(r["variant"], int(r["year"]), r["kind"])].append((r["home"], r["away"], float(r["margin"])))


def fit(games):
    teams = sorted({t for g in games for t in g[:2]}); ix = {t: i for i, t in enumerate(teams)}; n = len(teams)
    X = np.zeros((len(games), n + 1)); m = np.array([g[2] for g in games])
    for k, (h, a, _) in enumerate(games):
        X[k, ix[h]] = 1; X[k, ix[a]] = -1; X[k, n] = 1
    A = X.T @ X + np.eye(n + 1) * 0.5; A[n, n] -= 0.5
    b = np.linalg.solve(A, X.T @ m); r = b[:n] - b[:n].mean()
    return teams, r, b[n], (m - X @ b).std()


print("variant year | leader edge_top12 title_odds luck_rr | season: best_over_median quality_sd luck hfa decided<=8 >=28 >=40 >=60")
for (v, y, k) in sorted({(a, b, c) for (a, b, c) in rows if c == "rr"}, key=lambda x: (x[1], ["as_is", "pos", "garbage", "pos_garbage", "all3"].index(x[0]))):
    g = rows[(v, y, "rr")]
    teams, r, hfa, sd = fit(g)
    avg = collections.defaultdict(list); wins = collections.defaultdict(list)
    for h, a, m in g:
        avg[h].append(m); avg[a].append(-m)
        wins[(h, a)].append(m > 0 if m != 0 else 0.5); wins[(a, h)].append(m < 0 if m != 0 else 0.5)
    order = sorted(avg, key=lambda t: -np.mean(avg[t]))
    L = order[0]
    p = lambda o: np.mean(wins[(L, o)])
    title = p(order[3]) * 0.5 * (p(order[1]) + p(order[2]))
    s = rows[(v, y, "season")]
    st, sr, shfa, ssd = fit(s)
    marg = np.abs([x[2] for x in s])
    print(f"{v:12s} {y} | {L:14s} {np.mean(avg[L]):+5.1f} {title:4.2f} {sd:4.1f} | "
          f"{sr.max() - np.median(sr):5.1f} {sr.std():4.1f} {ssd:4.1f} {shfa:4.1f} "
          f"{(marg <= 8).mean():.3f} {(marg >= 28).mean():.3f} {(marg >= 40).mean():.3f} {(marg >= 60).mean():.3f}")
print("\nreal FBS 2011-2025 (no 2020): edge_top12 10.7 (max 15.8), title odds 0.46, best over median 31.1, quality sd 11.2, luck 13.2, hfa 3.0, decided<=8 0.367, >=28 0.211, >=40 0.067, >=60 0.004")
