"""College scoring, home-field edge and margins by division (research area 1).

Reads the sportsdataverse cfb_schedules release (CFBD data):
https://github.com/sportsdataverse/sportsdataverse-data/releases/tag/cfb_schedules
files cfb_schedules_YEAR.parquet. It has FBS, FCS, D2 and D3 games with final scores.

Usage: python3 college_scores.py DIR 2014 2015 ... > college_scores.json
Only games between two teams of the same division, completed, regular season.
"""
import json, sys
import pandas as pd

D, years = sys.argv[1], [int(y) for y in sys.argv[2:]]
g = pd.concat([pd.read_parquet(f"{D}/cfb_sched_{y}.parquet") for y in years])
g = g[g.completed & (g.season_type == "regular") & (g.home_division == g.away_division) & g.home_points.notna()]
g["margin"] = g.home_points - g.away_points
g["absm"] = g.margin.abs()


def summ(x, home_only):
    h = x[~x.neutral_site] if home_only else x
    pts = pd.concat([x.home_points, x.away_points])
    return {"games": int(len(x)), "points_per_team_game": round(pts.mean(), 2), "points_sd": round(pts.std(), 2),
            "home_games": int(len(h)), "home_win": round((h.margin > 0).mean(), 3), "home_margin": round(h.margin.mean(), 2),
            "decided_by_8_or_less": round((x.absm <= 8).mean(), 3), "decided_by_28_plus": round((x.absm >= 28).mean(), 3),
            "shutout_share_of_team_games": round((pts == 0).mean(), 4)}


out = {"source": "sportsdataverse cfb_schedules (CFBD), regular season, same-division games",
       "seasons": years, "divisions": {}}
for dv, x in g.groupby("home_division"):
    out["divisions"][dv] = {"all": summ(x, True),
                            "by_season": {str(s): summ(y, True) for s, y in x.groupby("season")},
                            "conference_games_only": summ(x[x.conference_game], True)}
json.dump(out, sys.stdout, indent=1)
