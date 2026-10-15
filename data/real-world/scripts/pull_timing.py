#!/usr/bin/env python3
"""When do real coaches pull the starting QB? Lead, time left and win probability at the pull.

QB1 = the team's passer with the most pass plays (incl. sacks) in the first half.
Pull = the team's first second-half pass play by another passer, after which QB1 takes no more snaps
(no pass, no rush). Only pulls while the team leads count as 'garbage-time pulls'; injuries and
benchings are mixed in (inferred to be the low-lead tail).

College: cfbfastR pbp FBS vs FBS 2022-2024. NFL: nflverse pbp 2018-2025 REG.
Usage: python3 pull_timing.py OUTDIR -> OUTDIR/out/pull_timing.json
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
CFB_URL = "https://github.com/sportsdataverse/sportsdataverse-data/releases/download/cfbfastR_cfb_pbp/play_by_play_{y}.parquet"
NFL_URL = "https://github.com/nflverse/nflverse-data/releases/download/pbp/play_by_play_{y}.parquet"


def fetch(url, local):
    path = os.path.join(DATA, local)
    if not os.path.exists(path) or os.path.getsize(path) < 1000:
        subprocess.run(["curl", "-sSL", "-o", path, url], check=True)
    return path


def q(s):
    s = s.dropna()
    if not len(s):
        return {"n": 0}
    v = s.quantile([0.1, 0.25, 0.5, 0.75, 0.9])
    return {"n": int(len(s)), "p10": float(v[0.1]), "p25": float(v[0.25]), "p50": float(v[0.5]),
            "p75": float(v[0.75]), "p90": float(v[0.9]), "mean": float(s.mean())}


def load_cfb():
    out = []
    for y in [2022, 2023, 2024]:
        d = pd.read_parquet(fetch(CFB_URL.format(y=y), f"cfb_pbp_{y}.parquet"),
                            columns=["game_id", "game_play_number", "period", "adj_TimeSecsRem", "pos_team", "pass", "rush",
                                     "passer_player_name", "rusher_player_name", "pos_score_diff_start", "wp_before",
                                     "home_team_division", "away_team_division", "play_text"])
        d = d[(d.home_team_division == "fbs") & (d.away_team_division == "fbs")]
        d = d.sort_values(["game_id", "game_play_number"])
        kneel = d.play_text.str.contains("kneel", case=False, na=False)
        out.append(pd.DataFrame({
            "game_id": d.game_id.astype(str), "half": np.where(d.period <= 2, 1, 2), "period": d.period,
            "secs_left": d.adj_TimeSecsRem, "team": d.pos_team, "is_pass": d["pass"] == 1,
            "is_rush": (d.rush == 1) & ~kneel, "passer": d.passer_player_name, "rusher": d.rusher_player_name,
            "lead": d.pos_score_diff_start, "wp": d.wp_before,
        }))
    return pd.concat(out, ignore_index=True)


def load_nfl():
    out = []
    for y in range(2018, 2026):
        d = pd.read_parquet(fetch(NFL_URL.format(y=y), f"pbp_{y}.parquet"),
                            columns=["game_id", "play_id", "season_type", "qtr", "game_seconds_remaining", "posteam",
                                     "play_type", "pass_attempt", "sack", "qb_scramble", "passer_id", "rusher_id",
                                     "score_differential", "wp", "two_point_attempt"])
        d = d[(d.season_type == "REG") & d.play_type.isin(["pass", "run"]) & (d.two_point_attempt != 1)].sort_values(["game_id", "play_id"])
        is_pass = (d.pass_attempt == 1) | (d.sack == 1) | (d.qb_scramble == 1)
        out.append(pd.DataFrame({
            "game_id": d.game_id, "half": np.where(d.qtr <= 2, 1, 2), "period": d.qtr, "secs_left": d.game_seconds_remaining,
            "team": d.posteam, "is_pass": is_pass, "is_rush": ~is_pass,
            "passer": d.passer_id.where(is_pass), "rusher": d.rusher_id.where(~is_pass),
            "lead": d.score_differential, "wp": d.wp,
        }))
    return pd.concat(out, ignore_index=True)


def analyse(p):
    p = p[p.team.notna()].copy()
    p["idx"] = np.arange(len(p))
    h1 = p[(p.half == 1) & p.is_pass & p.passer.notna()]
    qb1 = h1.groupby(["game_id", "team"]).passer.agg(lambda s: s.value_counts().index[0]).rename("qb1")
    p = p.join(qb1, on=["game_id", "team"])
    p["qb1_snap"] = (p.is_pass & (p.passer == p.qb1)) | (p.is_rush & (p.rusher == p.qb1))
    last_qb1 = p[p.qb1_snap].groupby(["game_id", "team"]).idx.max().rename("last_qb1")
    p = p.join(last_qb1, on=["game_id", "team"])
    cand = p[(p.half == 2) & p.is_pass & p.passer.notna() & p.qb1.notna() & (p.passer != p.qb1) & (p.idx > p.last_qb1)]
    pulls = cand.sort_values("idx").drop_duplicates(["game_id", "team"])
    # QB1's last snap (earliest possible pull moment) for confirmed QB changes
    lastrow = p.set_index("idx").loc[pulls.last_qb1.astype(int), ["period", "secs_left", "lead", "wp"]]
    lastrow.index = pulls.index
    pulls = pulls.join(lastrow.add_prefix("last_"))
    lead_pulls = pulls[pulls.lead > 0]
    trail_pulls = pulls[pulls.lead < 0]
    # outcome by lead entering Q4
    q4s = p[p.period == 4].sort_values("idx").drop_duplicates(["game_id", "team"])[["game_id", "team", "lead", "idx"]]
    q4s = q4s.rename(columns={"lead": "lead_q4", "idx": "q4_idx"})
    first_q4 = p[p.period == 4].groupby("game_id").idx.min().rename("q4_start")
    q4s = q4s.join(first_q4, on="game_id")
    pl = pulls.set_index(["game_id", "team"]).last_qb1.rename("pull_idx")
    q4s = q4s.join(pl, on=["game_id", "team"])
    by_lead = {}
    for lo, hi, lab in [(28, 999, "28+"), (21, 27, "21-27"), (14, 20, "14-20"), (7, 13, "7-13"), (1, 6, "1-6"),
                        (-6, -1, "-1 to -6"), (-20, -7, "-7 to -20"), (-27, -21, "-21 to -27"), (-999, -28, "-28 or worse")]:
        s = q4s[(q4s.lead_q4 >= lo) & (q4s.lead_q4 <= hi)]
        before = (s.pull_idx < s.q4_start).mean()
        during = (s.pull_idx >= s.q4_start).mean()
        by_lead[lab] = {"team_games": int(len(s)), "qb1_last_snap_before_q4": float(before),
                        "qb1_last_snap_in_q4": float(during), "no_confirmed_qb_change": float(s.pull_idx.isna().mean())}
    return {
        "qb1_last_snap_when_pulled_while_leading": {
            "lead": q(lead_pulls.last_lead), "minutes_left": q(lead_pulls.last_secs_left / 60), "wp": q(lead_pulls.last_wp),
            "share_last_snap_in_q2_or_earlier": float((lead_pulls.last_period <= 2).mean()),
            "share_last_snap_in_q3": float((lead_pulls.last_period == 3).mean()),
            "share_last_snap_in_q4": float((lead_pulls.last_period == 4).mean())},
        "pulls_while_leading": {"lead_at_pull": q(lead_pulls.lead), "minutes_left_at_pull": q(lead_pulls.secs_left / 60),
                                "wp_at_pull": q(lead_pulls.wp),
                                "share_in_q3": float((lead_pulls.period == 3).mean()), "share_in_q4": float((lead_pulls.period == 4).mean())},
        "pulls_while_trailing": {"deficit_at_pull": q(-trail_pulls.lead), "minutes_left_at_pull": q(trail_pulls.secs_left / 60),
                                 "wp_at_pull": q(trail_pulls.wp)},
        "by_lead_entering_q4": by_lead,
    }


def main():
    res = {"cfb_fbs_vs_fbs_2022_2024": analyse(load_cfb()), "nfl_reg_2018_2025": analyse(load_nfl())}
    with open(os.path.join(OUT, "out", "pull_timing.json"), "w") as f:
        json.dump(res, f, indent=1)
    print(json.dumps(res, indent=1))


if __name__ == "__main__":
    main()
