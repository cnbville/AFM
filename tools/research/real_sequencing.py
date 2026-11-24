#!/usr/bin/env python3
"""Real play sequencing from nflverse (scheme sequencing research, R0 and E).

    real_sequencing.py <folder with pbp_Y.parquet, part_Y.parquet, ftn_Y.parquet> <out.json> [years...]

Measures (regular and post season, no garbage time: win probability 0.05-0.95):
 1 direction: a designed run's EPA and success after the offense's last two designed runs in the game went
   the same way (left/left or right/right), split by whether this run goes the same way, the other way or middle;
   outside runs (run_gap end) apart.
 2 play action by the game's run share so far (FTN is_play_action), against straight drops.
 3 the box after run streaks: defenders in the box on 1st and 2nd down by the offense's runs in its last 3 snaps.
 4 serial correlation: run rate after a run and after a pass, by down and distance.
 5 upsets: the favorite's win share by the closing spread (nflverse spread_line and result).
 6 coverages: EPA a dropback by coverage, and by the targeted man's route and coverage (participation 2023+).
"""
import json
import sys

import pandas as pd

src, out = sys.argv[1], sys.argv[2]
years = [int(y) for y in sys.argv[3:]] or [2022, 2023, 2024]
pcols = ['game_id', 'play_id', 'posteam', 'defteam', 'drive', 'down', 'ydstogo', 'yardline_100', 'qtr', 'play_type',
         'rush', 'pass', 'qb_dropback', 'qb_scramble', 'qb_kneel', 'qb_spike', 'run_location', 'run_gap', 'epa', 'success',
         'wp', 'spread_line', 'result', 'home_team', 'away_team', 'season_type', 'yards_gained', 'penalty']
pbp = pd.concat([pd.read_parquet(f'{src}/pbp_{y}.parquet', columns=pcols).assign(season=y) for y in years])
part = pd.concat([pd.read_parquet(f'{src}/part_{y}.parquet') for y in years])
ftn = pd.concat([pd.read_parquet(f'{src}/ftn_{y}.parquet') for y in years])
part = part.rename(columns={'nflverse_game_id': 'game_id'})
ftn = ftn.rename(columns={'nflverse_game_id': 'game_id', 'nflverse_play_id': 'play_id'})
ftn['play_id'] = ftn['play_id'].astype(float)
pbp['play_id'] = pbp['play_id'].astype(float)
part['play_id'] = part['play_id'].astype(float)
df = pbp.merge(part[['game_id', 'play_id', 'defenders_in_box', 'route', 'defense_coverage_type', 'defense_man_zone_type',
                     'offense_formation', 'offense_personnel']], on=['game_id', 'play_id'], how='left')
df = df.merge(ftn[['game_id', 'play_id', 'is_play_action', 'is_motion', 'is_rpo', 'is_screen_pass', 'n_defense_box']],
              on=['game_id', 'play_id'], how='left')
scrim = df[df['play_type'].isin(['run', 'pass']) & df['epa'].notna() & (df['qb_kneel'] != 1) & (df['qb_spike'] != 1)].copy()
scrim = scrim.sort_values(['game_id', 'play_id'])
scrim['designed_run'] = (scrim['play_type'] == 'run') & (scrim['qb_scramble'] != 1)
scrim['dropback'] = scrim['qb_dropback'] == 1
neutral = scrim['wp'].between(0.05, 0.95)
R = {'years': years, 'source': 'nflverse play_by_play, pbp_participation, ftn_charting (GitHub releases)'}


def stat(d):
    return {'n': int(len(d)), 'epa': round(float(d['epa'].mean()), 4) if len(d) else None,
            'success': round(float(d['success'].mean()), 4) if len(d) else None,
            'yards': round(float(d['yards_gained'].mean()), 3) if len(d) else None}


# 1 direction
runs = scrim[scrim['designed_run'] & scrim['run_location'].isin(['left', 'right', 'middle'])].copy()
g = runs.groupby(['game_id', 'posteam'])['run_location']
runs['prev1'] = g.shift(1)
runs['prev2'] = g.shift(2)
rn = runs[neutral.reindex(runs.index).fillna(False)]
d1 = {}
for scope, sub in [('all runs', rn), ('outside runs (run_gap end)', rn[rn['run_gap'] == 'end'])]:
    both = sub[(sub['prev1'] == sub['prev2']) & sub['prev1'].isin(['left', 'right'])]
    same = both[both['run_location'] == both['prev1']]
    other = both[(both['run_location'] != both['prev1']) & (both['run_location'] != 'middle')]
    mid = both[both['run_location'] == 'middle']
    one = sub[sub['prev1'].isin(['left', 'right']) & (sub['prev1'] != sub['prev2'])]
    d1[scope] = {'baseline': stat(sub), 'after two the same way: again that way': stat(same),
                 'after two the same way: the other way': stat(other), 'after two the same way: middle': stat(mid),
                 'after mixed sides: same side as last': stat(one[one['run_location'] == one['prev1']]),
                 'after mixed sides: other side': stat(one[(one['run_location'] != one['prev1']) & (one['run_location'] != 'middle')])}
    # streaks of three
    runs3 = sub.copy()
    runs3['prev3'] = g.shift(3).reindex(runs3.index)
    three = runs3[(runs3['prev1'] == runs3['prev2']) & (runs3['prev2'] == runs3['prev3']) & runs3['prev1'].isin(['left', 'right'])]
    d1[scope]['after three the same way: again'] = stat(three[three['run_location'] == three['prev1']])
    d1[scope]['after three the same way: the other way'] = stat(three[(three['run_location'] != three['prev1']) & (three['run_location'] != 'middle')])
R['1_direction'] = d1

# 2 play action by run share so far
sc = scrim.copy()
sc['run_so_far'] = sc.groupby(['game_id', 'posteam'])['designed_run'].transform(lambda s: s.shift(1).expanding().mean())
sc['plays_so_far'] = sc.groupby(['game_id', 'posteam']).cumcount()
db = sc[sc['dropback'] & sc['is_play_action'].notna() & (sc['plays_so_far'] >= 10) & neutral.reindex(sc.index).fillna(False)
        & sc['down'].isin([1, 2])]
bins = [0, 0.35, 0.45, 0.55, 1.01]
db = db.assign(share=pd.cut(db['run_so_far'], bins, right=False))
d2 = {}
for b, sub in db.groupby('share', observed=True):
    d2[str(b)] = {'play action': stat(sub[sub['is_play_action'] == True]), 'straight drop': stat(sub[sub['is_play_action'] == False])}
R['2_play_action_by_run_share_so_far (1st/2nd down, 10+ plays in)'] = d2
# play action after the last run's success
sc['last_run_success'] = sc.groupby(['game_id', 'posteam'])['success'].shift(1).where(
    sc.groupby(['game_id', 'posteam'])['designed_run'].shift(1) == True)
pa = sc[sc['dropback'] & (sc['is_play_action'] == True) & neutral.reindex(sc.index).fillna(False)]
R['2b_play_action_by_previous_snap'] = {
    'previous snap a successful run': stat(pa[pa['last_run_success'] == 1]),
    'previous snap a failed run': stat(pa[pa['last_run_success'] == 0]),
    'previous snap a pass': stat(pa[pa['last_run_success'].isna()])}

# 3 box after run streaks
sc['runs_last3'] = sc.groupby(['game_id', 'posteam'])['designed_run'].transform(lambda s: s.shift(1).rolling(3).sum())
bx = sc[sc['down'].isin([1, 2]) & sc['defenders_in_box'].notna() & neutral.reindex(sc.index).fillna(False)
        & sc['ydstogo'].between(5, 10) & sc['offense_personnel'].fillna('').str.startswith('1 RB, 1 TE, 3 WR')]
R['3_box_on_1st_2nd_and_5_10_in_11_personnel_by_runs_in_last_3'] = {
    str(int(k)): {'n': int(len(v)), 'box': round(float(v['defenders_in_box'].mean()), 3),
                  'next is a run': round(float(v['designed_run'].mean()), 3)} for k, v in bx.groupby('runs_last3')}

# 4 serial correlation
sc['prev_run'] = sc.groupby(['game_id', 'posteam'])['designed_run'].shift(1)
sc['prev_drive'] = sc.groupby(['game_id', 'posteam'])['drive'].shift(1)
sr = sc[(sc['prev_drive'] == sc['drive']) & neutral.reindex(sc.index).fillna(False)]
sr = sr.assign(dd=sr['down'].astype('Int64').astype(str) + ' & ' + pd.cut(sr['ydstogo'], [0, 3, 6, 10, 99]).astype(str))
d4 = {}
for k, v in sr.groupby('dd'):
    if len(v) < 400:
        continue
    d4[k] = {'run rate after a run': round(float(v[v['prev_run'] == True]['designed_run'].mean()), 3),
             'run rate after a pass': round(float(v[v['prev_run'] == False]['designed_run'].mean()), 3), 'n': int(len(v))}
R['4_serial_correlation_same_drive'] = d4

# 5 upsets by spread
games = df.drop_duplicates('game_id')[['game_id', 'spread_line', 'result', 'season_type']].dropna()
games = games[games['spread_line'] != 0]
games['fav_won'] = ((games['spread_line'] > 0) & (games['result'] > 0)) | ((games['spread_line'] < 0) & (games['result'] < 0))
games['fav_margin_vs_line'] = games['result'] * games['spread_line'].apply(lambda s: 1 if s > 0 else -1) - games['spread_line'].abs()
games['abs'] = games['spread_line'].abs()
d5 = {}
for b, v in games.groupby(pd.cut(games['abs'], [0, 3.1, 6.6, 9.6, 13.6, 30])):
    d5[str(b)] = {'games': int(len(v)), 'favorite wins': round(float(v['fav_won'].mean()), 3)}
R['5_upsets_by_spread'] = {'bins': d5, 'margin around the line sd': round(float(games['fav_margin_vs_line'].std()), 2),
                           'games': int(len(games))}

# 6 coverages
cv = sc[sc['dropback'] & sc['defense_coverage_type'].notna() & neutral.reindex(sc.index).fillna(False)]
R['6_epa_by_coverage'] = {k: stat(v) for k, v in cv.groupby('defense_coverage_type') if len(v) >= 200}
rt = cv[cv['route'].fillna('') != '']
cell = {}
for (r, c), v in rt.groupby(['route', 'defense_coverage_type']):
    if len(v) >= 120:
        cell[f'{r} | {c}'] = stat(v)
R['6b_targeted_route_by_coverage'] = cell
json.dump(R, open(out, 'w'), indent=1)
print(json.dumps({k: R[k] for k in ['1_direction', '2b_play_action_by_previous_snap', '5_upsets_by_spread']}, indent=1))
