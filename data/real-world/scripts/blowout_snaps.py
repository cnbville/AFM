#!/usr/bin/env python3
"""Part C: do NFL starters play less in blowouts? Snap share by final margin and by margin after Q3.

Sources (nflverse-data releases): snap_counts/snap_counts_<year>.parquet, schedules/games.parquet,
pbp/play_by_play_<year>.parquet (score after three quarters). Regular seasons 2018-2025.

Starter = for each team-season, the player with the most offensive (defensive) snaps at the position:
QB1, RB1, WR1-WR3, TE1, OL1-OL5 (T/G/C), and on defense DL1-DL4 (DE/DT/NT), LB1-LB2, CB1-CB2, S1-S2.
Only games the starter played (snaps > 0) count, so games he missed are not mistaken for benching.

Usage: python3 blowout_snaps.py OUTDIR -> OUTDIR/out/blowout_snaps.json
"""
import json
import os
import subprocess
import sys

import numpy as np
import pandas as pd

OUT = sys.argv[1] if len(sys.argv) > 1 else "."
DATA = os.path.join(OUT, "data")
os.makedirs(DATA, exist_ok=True)
os.makedirs(os.path.join(OUT, "out"), exist_ok=True)
BASE = "https://github.com/nflverse/nflverse-data/releases/download"
SEASONS = list(range(2018, 2026))


def fetch(tag, name, local=None):
    local = local or name
    path = os.path.join(DATA, local)
    if not os.path.exists(path) or os.path.getsize(path) < 1000:
        subprocess.run(["curl", "-sSL", "-o", path, f"{BASE}/{tag}/{name}"], check=True)
    return path


ROLES = {
    "QB1": ("off", ["QB"], 1), "RB1": ("off", ["RB"], 1), "WR1-3": ("off", ["WR"], 3), "TE1": ("off", ["TE"], 1),
    "OL1-5": ("off", ["T", "G", "C", "OL"], 5),
    "DL1-4": ("def", ["DE", "DT", "NT", "DL"], 4), "LB1-2": ("def", ["LB", "ILB", "OLB", "MLB"], 2),
    "CB1-2": ("def", ["CB"], 2), "S1-2": ("def", ["FS", "SS", "S"], 2),
}


def main():
    snaps = pd.concat([pd.read_parquet(fetch("snap_counts", f"snap_counts_{y}.parquet")) for y in SEASONS])
    snaps = snaps[snaps.game_type == "REG"]
    games = pd.read_parquet(fetch("schedules", "games.parquet"))
    games = games[games.season.isin(SEASONS) & (games.game_type == "REG")][["game_id", "home_team", "away_team", "result"]]

    # margin after three quarters from pbp
    q3 = []
    for y in SEASONS:
        p = pd.read_parquet(fetch("pbp", f"play_by_play_{y}.parquet", f"pbp_{y}.parquet"),
                            columns=["game_id", "season_type", "qtr", "play_id", "total_home_score", "total_away_score"])
        p = p[(p.season_type == "REG") & (p.qtr <= 3)].sort_values(["game_id", "play_id"])
        q3.append(p.groupby("game_id")[["total_home_score", "total_away_score"]].max())
    q3 = pd.concat(q3)
    q3["home_lead3"] = q3.total_home_score - q3.total_away_score
    games = games.merge(q3[["home_lead3"]], left_on="game_id", right_index=True, how="left")

    s = snaps.merge(games, on="game_id", how="inner")
    s["is_home"] = s.team == s.home_team
    s["is_away"] = s.team == s.away_team
    unmatched = int((~(s.is_home | s.is_away)).sum())
    s = s[s.is_home | s.is_away]
    sign = np.where(s.is_home, 1, -1)
    s["margin"] = sign * s.result
    s["lead3"] = sign * s.home_lead3

    res = {"meta": {"unmatched_team_rows": unmatched, "seasons": SEASONS}}
    for role, (side, poss, k) in ROLES.items():
        col = "offense_snaps" if side == "off" else "defense_snaps"
        pct = "offense_pct" if side == "off" else "defense_pct"
        r = s[s.position.isin(poss)]
        tot = r.groupby(["season", "team", "pfr_player_id"])[col].sum().reset_index()
        tot["rank"] = tot.groupby(["season", "team"])[col].rank(ascending=False, method="first")
        starters = tot[tot["rank"] <= k][["season", "team", "pfr_player_id"]]
        g = r.merge(starters, on=["season", "team", "pfr_player_id"])
        g = g[g[col] > 0]
        out = {}
        for lab, mask in [
            ("won_by_24plus", g.margin >= 24), ("lost_by_24plus", g.margin <= -24), ("within_8", g.margin.abs() <= 8),
            ("led_by_21plus_after_Q3", g.lead3 >= 21), ("trailed_by_21plus_after_Q3", g.lead3 <= -21),
            ("within_8_after_Q3", g.lead3.abs() <= 8),
        ]:
            x = g.loc[mask, pct]
            out[lab] = {"player_games": int(len(x)), "mean": float(x.mean()), "median": float(x.median()),
                        "share_below_90": float((x < 0.9).mean()), "share_below_75": float((x < 0.75).mean()),
                        "share_below_60": float((x < 0.6).mean())}
        res[role] = out
    with open(os.path.join(OUT, "out", "blowout_snaps.json"), "w") as f:
        json.dump(res, f, indent=1)
    print("wrote", os.path.join(OUT, "out", "blowout_snaps.json"))


if __name__ == "__main__":
    main()
