"""The real NFL two-point habit (engine gaps item 19): the share of tries
that go for two, by the score after the touchdown (the scoring team's lead
before the try) and the time, from nflverse play-by-play, regular seasons
2021-2025. Writes nfl_two_point_habit.csv; cells with fewer than 8 tries
are left out (the engine falls back on the earlier time bucket).

    python3 scripts/nfl_two_point.py <folder with pbp_2021.csv.gz ...>
"""
import sys
import numpy as np
import pandas as pd

d = sys.argv[1]
cols = ['season_type', 'qtr', 'game_seconds_remaining', 'posteam_score', 'defteam_score',
        'extra_point_attempt', 'two_point_attempt']
frames = []
for y in [2021, 2022, 2023, 2024, 2025]:
    f = pd.read_csv(f'{d}/pbp_{y}.csv.gz', usecols=cols, low_memory=False)
    frames.append(f[f.season_type == 'REG'])
p = pd.concat(frames)
t = p[(p.extra_point_attempt == 1) | (p.two_point_attempt == 1)].copy()
t = t[t.qtr <= 4]
# The score on the try's row already counts the touchdown.
t['diff'] = (t.posteam_score - t.defteam_score).clip(-25, 25).astype(int)
t['time'] = np.where(t.qtr <= 2, 0, np.where(t.qtr == 3, 1, np.where(t.game_seconds_remaining > 420, 2, 3)))
g = t.groupby(['time', 'diff']).two_point_attempt.agg(['mean', 'count']).reset_index()
g = g[g['count'] >= 8]
with open('nfl_two_point_habit.csv', 'w') as out:
    out.write('time,diff,rate,n\n')
    for r in g.itertuples():
        out.write(f'{r.time},{r.diff},{r.mean:.4f},{r.count}\n')
print('tries', len(t), 'two-point share', round(t.two_point_attempt.mean(), 4), 'cells', len(g))
