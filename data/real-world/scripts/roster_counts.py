"""Real college roster counts by position (fix list topic 1, 2026-10-15).

Reads ESPN college rosters with CFBD recruiting links (sportsdataverse
release `espn_cfb_rosters`, files cfb_rosters_YEAR.parquet) and prints, per
division and year: players per team by position group, the share of each
group with no recruiting record (a walk-on proxy), kickers, punters and long
snappers per team with how many teams list none, and the split inside the
generic OL, DL and DB groups from the teams that list the exact spot.

With a cfbfastR play-by-play folder (release `cfbfastR_cfb_pbp`, files
play_by_play_YEAR.parquet) it also counts two-point tries per team game.

Usage: python3 roster_counts.py ROSTER_DIR [PBP_DIR]
Development checks only, never in-game tuning.
"""
import sys

import pandas as pd

GROUPS = {
    'QB': ['QB'], 'RB': ['RB', 'FB'], 'WR': ['WR'], 'TE': ['TE'],
    'OL': ['OL', 'OT', 'G', 'C'], 'DL': ['DL', 'DE', 'DT', 'NT', 'EDGE'],
    'LB': ['LB'], 'DB': ['DB', 'S', 'CB'], 'K': ['PK'], 'P': ['P'], 'LS': ['LS'],
}
SPLITS = {'OL': (['OL'], ['OT'], ['G', 'C']), 'DL': (['DL'], ['DE', 'EDGE'], ['DT', 'NT']), 'DB': (['DB'], ['CB'], ['S'])}


def no_record(v):
    return v is None or (isinstance(v, float) and v != v) or len(v) == 0


def rosters(folder):
    inv = {p: g for g, ps in GROUPS.items() for p in ps}
    years = [2022, 2023, 2024, 2025]
    frames = [pd.read_parquet(f'{folder}/cfb_rosters_{y}.parquet').assign(year=y) for y in years]
    d = pd.concat(frames)
    d['grp'] = d.position_abbreviation.map(inv)
    d['walkon'] = d.cfbd_recruit_ids.apply(no_record)
    for y in years:
        for div in ['fbs', 'fcs']:
            f = d[(d.year == y) & (d.division == div)]
            nt = f.team_id.nunique()
            size = f.groupby('team_id').size()
            per = f.groupby('grp').size() / nt
            wo = f.groupby('grp').walkon.mean()
            print(f'{y} {div}: {nt} teams, roster median {size.median():.1f} (p10 {size.quantile(.1):.0f}, p90 {size.quantile(.9):.0f})')
            print('  per team / no record:', {g: (round(float(per.get(g, 0)), 1), round(float(wo.get(g, 0)), 2)) for g in GROUPS})
            for spot in ['PK', 'P', 'LS']:
                c = f[f.position_abbreviation == spot].groupby('team_id').size().reindex(size.index, fill_value=0)
                print(f'  {spot}: {c.mean():.2f} a team, range {c.min()}-{c.max()}, teams with none {(c == 0).mean():.0%}')
    f = d[(d.division == 'fbs') & (d.year <= 2024)]
    for g, (generic, a, b) in SPLITS.items():
        na = nb = n = 0
        for lst in f.groupby(['year', 'team_id']).position_abbreviation.apply(list):
            if any(x in generic for x in lst):
                continue
            ca, cb = sum(x in a for x in lst), sum(x in b for x in lst)
            if ca + cb:
                na, nb, n = na + ca, nb + cb, n + 1
        print(f'FBS 2022-24 {g}: {n} team seasons list the exact spot; {a[0]} share {na / max(na + nb, 1):.3f}')


def two_point(folder):
    for div in ['fbs', 'fcs']:
        games = tries = tds = 0
        for y in [2022, 2023, 2024]:
            d = pd.read_parquet(f'{folder}/play_by_play_{y}.parquet',
                                columns=['season_type', 'home_team_division', 'away_team_division', 'game_id', 'play_type', 'play_text'])
            d = d[(d.season_type == 'regular') & (d.home_team_division == div) & (d.away_team_division == div)]
            txt = d.play_text.astype(str).str.lower()
            pt = d.play_type.astype(str)
            games += d.game_id.nunique()
            tries += (txt.str.contains(r'two-point|two point|2pt|2-pt|2 pt') & ~pt.str.contains('Penalty')).sum()
            tds += pt.str.contains('Touchdown').sum()
        print(f'{div} 2022-24: two-point tries {tries / (2 * games):.3f} a team game ({tries / tds:.1%} of touchdown plays)')


if __name__ == '__main__':
    rosters(sys.argv[1])
    if len(sys.argv) > 2:
        two_point(sys.argv[2])
