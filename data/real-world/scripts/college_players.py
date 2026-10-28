"""Real FBS player season lines, 2004-2025 (world reader lane R3).

Reads, saved in DIR:
  player_box_YEAR.parquet 2004-2025 (sportsdataverse-data release espn_cfb_player_box: ESPN box scores),
  cfb_team_info_YEAR.parquet 2004-2025 (release cfb_team_info: which teams were FBS that year),
  cfb_rosters_YEAR.csv 2004-2025 (https://github.com/sportsdataverse/cfbfastR-data/tree/main/rosters/csv: positions).
Every game an FBS team played counts, FCS opponents and bowls included (that is how the NCAA counts season stats since 2002).
Tackles and sacks start in 2024: before then ESPN's college boxes carry defensive lines for under half the games (2023: 52%; 2016-2022: 4-17%).

Usage: python3 college_players.py DIR > college_players.json
Development checks only, never in-game tuning.
"""
import json, sys
import numpy as np
import pandas as pd

D = sys.argv[1]
GROUP = {"QB": "QB", "RB": "RB", "FB": "RB", "WR": "WR", "TE": "TE", "OL": "OL", "OT": "OL", "OG": "OL", "C": "OL", "G": "OL", "T": "OL",
         "DL": "DL", "DE": "DL", "DT": "DL", "NT": "DL", "LB": "LB", "OLB": "LB", "ILB": "LB", "MLB": "LB",
         "DB": "DB", "CB": "DB", "S": "DB", "FS": "DB", "SS": "DB", "PK": "K", "K": "K", "P": "P", "LS": "LS"}
NUM = ["passingYards", "passingTouchdowns", "interceptions", "rushingAttempts", "rushingYards", "rushingTouchdowns", "receptions",
       "receivingYards", "receivingTouchdowns", "totalTackles", "sacks", "tacklesForLoss", "passesDefended", "punts", "puntYards",
       "puntsInside20", "longFieldGoalMade", "kickReturnYards", "puntReturnYards"]


def r(x, n=3):
    try:
        x = float(x)
    except (TypeError, ValueError):
        return None
    return None if np.isnan(x) else round(x, n)


def split(s, i):
    return pd.to_numeric(s.astype(str).str.split("/").str[i], errors="coerce")


lines, per_game, leaders, maxes = {}, {}, {}, []
for y in range(2004, 2026):
    b = pd.read_parquet(f"{D}/player_box_{y}.parquet")
    ti = pd.read_parquet(f"{D}/cfb_team_info_{y}.parquet", columns=["team_id", "classification"])
    fbs = set(ti[ti.classification == "fbs"].team_id.astype(str))
    b = b[b.team_id.astype(str).isin(fbs)].copy()
    for c in NUM:
        if c in b:
            b[c] = pd.to_numeric(b[c], errors="coerce")
    b["cmp"] = split(b.get("completions/passingAttempts", pd.Series(dtype=str)), 0)
    b["att"] = split(b.get("completions/passingAttempts", pd.Series(dtype=str)), 1)
    b["fgm"] = split(b.get("fieldGoalsMade/fieldGoalAttempts", pd.Series(dtype=str)), 0)
    b["fga"] = split(b.get("fieldGoalsMade/fieldGoalAttempts", pd.Series(dtype=str)), 1)
    b["def_int"] = np.where(b.category == "interceptions", b.interceptions, np.nan)
    b["int_thrown"] = np.where(b.category == "passing", b.interceptions, np.nan)
    ro = pd.read_csv(f"{D}/cfb_rosters_{y}.csv", usecols=["athlete_id", "position"], low_memory=False).drop_duplicates("athlete_id")
    pos = dict(zip(ro.athlete_id.astype(str), ro.position.map(lambda p: GROUP.get(str(p).upper(), "other"))))
    cols = ["cmp", "att", "passingYards", "passingTouchdowns", "int_thrown", "rushingAttempts", "rushingYards", "rushingTouchdowns",
            "receptions", "receivingYards", "receivingTouchdowns", "totalTackles", "sacks", "tacklesForLoss", "def_int", "fgm", "fga",
            "punts", "puntYards", "puntsInside20"]
    cols = [c for c in cols if c in b]
    s = b.groupby(["athlete_id", "athlete_name"])[cols].sum(min_count=1)
    s["games"] = b.groupby(["athlete_id", "athlete_name"]).game_id.nunique()
    s = s.reset_index()
    s["grp"] = s.athlete_id.astype(str).map(pos).fillna("unknown")
    team_games = b.groupby("team_id").game_id.nunique().sum()
    g = lambda c, t: int((s.get(c, pd.Series(dtype=float)) >= t).sum())
    q = lambda v, ps=(10, 50, 90, 99): {f"p{p}": r(np.nanpercentile(v, p), 1) for p in ps} if len(v.dropna()) else None
    qb = s[s.att >= 150]
    rb = s[s.rushingAttempts >= 100]
    rec = s[s.receptions >= 25]
    lines[y] = {"teams": len(fbs), "passers_4000": g("passingYards", 4000), "passers_3000": g("passingYards", 3000),
                "rushers_1000": g("rushingYards", 1000), "receivers_1000": g("receivingYards", 1000), "catches_80": g("receptions", 80),
                "pass_td_30": g("passingTouchdowns", 30), "rush_td_15": g("rushingTouchdowns", 15),
                "qb_pass_yds": q(qb.passingYards), "qb_ypa": q(qb.passingYards / qb.att), "qb_cmp_pct": q(100 * qb.cmp / qb.att),
                "rb_rush_yds": q(rb.rushingYards), "rb_ypc": q(rb.rushingYards / rb.rushingAttempts),
                "rec_yds": q(rec.receivingYards), "kicker_fg_pct": q(100 * s[s.fga >= 12].fgm / s[s.fga >= 12].fga, (10, 50, 90)),
                "punter_gross": q(s[s.punts >= 30].puntYards / s[s.punts >= 30].punts, (10, 50, 90))}
    if y >= 2024:
        dv = s[s.games >= 8]
        lines[y].update({"sacks_10": g("sacks", 10), "tackles_100": g("totalTackles", 100), "ints_6": g("def_int", 6),
                         "def_tackles": q(dv.totalTackles[dv.totalTackles > 0], (50, 90, 99)),
                         "def_sacks": q(dv.sacks[dv.sacks > 0], (50, 90, 99)), "tfl": q(dv.tacklesForLoss[dv.tacklesForLoss > 0], (50, 90, 99))})
    else:
        lines[y].update({"ints_6": g("def_int", 6)})
    pg = {}
    keep = ["passingYards", "rushingYards", "receivingYards", "receptions", "def_int"] + (["totalTackles", "sacks"] if y >= 2024 else [])
    for grp, v in s.groupby("grp"):
        pg[grp] = {c: r(v[c].sum() / team_games, 2) for c in keep
                   if c in v and v[c].sum() > 0}
    per_game[y] = pg
    lead = {}
    for c in ["passingYards", "passingTouchdowns", "rushingYards", "rushingTouchdowns", "receivingYards", "receptions", "sacks", "totalTackles", "def_int"]:
        if c in s and s[c].notna().any() and (y >= 2024 or c not in ("sacks", "totalTackles")):
            top = s.nlargest(3, c)
            lead[c] = [[n, r(v, 1)] for n, v in zip(top.athlete_name, top[c])]
            maxes.append((c, y, top.athlete_name.iat[0], r(top[c].iat[0], 1)))
    leaders[y] = lead

best = {}
for c, y, n, v in maxes:
    if c not in best or v > best[c][2]:
        best[c] = [y, n, v]
out = {"source": "ESPN college box scores (sportsdataverse-data espn_cfb_player_box) 2004-2025, positions from CollegeFootballData rosters (cfbfastR-data)",
       "holes": "Before 2004 nothing open; full tackles and sacks only for 2024-2025 (defensive interceptions every year); FCS, D2, D3 not included; positions missing for about 2% of players ('unknown').",
       "note": "Season lines sum every game an FBS team played (bowls and FCS games too). Regulars: QB 150+ attempts, backs 100+ carries, receivers 25+ catches, defenders 8+ games, kickers 12+ tries, punters 30+ punts.",
       "by_season": lines,
       "per_team_game_by_position": {"note": "each position group's total divided by team games (G18)", "by_season": per_game},
       "leaders": {"note": "top three each season", "by_season": leaders},
       "data_era_bests": {"note": "the best single season since 2004 in this data, not the all-time NCAA record", "by_stat": best}}
json.dump(out, sys.stdout, indent=1, default=str)
