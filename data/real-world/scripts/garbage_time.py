#!/usr/bin/env python3
"""Part B: garbage time behaviour by lead after three quarters.

College: cfbfastR play-by-play, FBS 2022-2024 (sportsdataverse-data release cfbfastR_cfb_pbp,
         play_by_play_<year>.parquet).
NFL:     nflverse-data release pbp, play_by_play_<year>.parquet, 2018-2025 regular season.

Usage: python3 garbage_time.py OUTDIR   -> OUTDIR/out/garbage_time.json (data cached in OUTDIR/data)
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
BANDS = [(28, 999, "28+"), (21, 27, "21-27"), (15, 20, "15-20"), (1, 14, "1-14")]


def fetch(url, local):
    path = os.path.join(DATA, local)
    if not os.path.exists(path) or os.path.getsize(path) < 1000:
        subprocess.run(["curl", "-sSL", "-o", path, url], check=True)
    return path


def load_cfb(years):
    cols = ["game_id", "season", "game_play_number", "period", "TimeSecsRem", "pos_team", "home", "away",
            "pos_team_score", "def_pos_team_score", "play_type", "play_text", "rush", "pass", "pass_attempt", "sack",
            "passer_player_name", "rusher_player_name", "down", "home_team_division", "away_team_division", "season_type",
            "yards_to_goal", "distance", "drive_id", "drive_start_period", "drive_end_period", "drive_time_minutes_elapsed", "drive_time_seconds_elapsed"]
    out = []
    for y in years:
        d = pd.read_parquet(fetch(CFB_URL.format(y=y), f"cfb_pbp_{y}.parquet"), columns=cols)
        d = d.sort_values(["game_id", "game_play_number"], kind="stable")
        ph = d.pos_team == d.home
        d["home_post"] = np.where(ph, d.pos_team_score, d.def_pos_team_score)
        d["away_post"] = np.where(ph, d.def_pos_team_score, d.pos_team_score)
        d.loc[d.pos_team.isna(), ["home_post", "away_post"]] = np.nan
        kneel = d.play_text.str.contains("kneel", case=False, na=False) & (d.rush == 1)
        t = pd.DataFrame({
            "game_id": d.game_id.astype(str), "year": d.season, "period": d.period,
            "t_rem": d.TimeSecsRem, "pos_team": d.pos_team, "home": d.home, "away": d.away,
            "home_post": d.home_post, "away_post": d.away_post,
            "is_pass": (d["pass"] == 1), "is_rush": (d.rush == 1) & ~kneel, "is_kneel": kneel,
            "passer": d.passer_player_name, "rusher": d.rusher_player_name, "down": d.down,
            "is_punt": d.play_type.str.contains("Punt", na=False),
            "is_fg": d.play_type.str.contains("Field Goal", na=False),
            "is_kick": d.play_type.str.contains("Kickoff", na=False),
            "deep": np.nan, "timeout_team": None,
            "div_home": d.home_team_division, "div_away": d.away_team_division,
            "ytg": d.yards_to_goal, "togo": d.distance,
            "drive_id": d.drive_id, "drive_q4_only": (d.drive_start_period == 4) & (d.drive_end_period == 4),
            "drive_secs": d.drive_time_minutes_elapsed * 60 + d.drive_time_seconds_elapsed,
        })
        out.append(t)
    return pd.concat(out, ignore_index=True)


def load_nfl(years):
    cols = ["game_id", "season", "season_type", "play_id", "qtr", "quarter_seconds_remaining", "posteam", "home_team",
            "away_team", "total_home_score", "total_away_score", "play_type", "pass_attempt", "sack", "rush",
            "qb_scramble", "qb_kneel", "qb_spike", "passer_id", "rusher_id", "down", "air_yards", "timeout",
            "timeout_team", "two_point_attempt", "yardline_100", "ydstogo"]
    out = []
    for y in years:
        d = pd.read_parquet(fetch(NFL_URL.format(y=y), f"pbp_{y}.parquet"), columns=cols)
        d = d[d.season_type == "REG"].sort_values(["game_id", "play_id"])
        scrim = d.play_type.isin(["pass", "run"]) & (d.two_point_attempt != 1)
        is_pass = scrim & ((d.pass_attempt == 1) | (d.sack == 1)) & (d.qb_scramble != 1)
        is_rush = scrim & ~is_pass
        t = pd.DataFrame({
            "game_id": d.game_id, "year": d.season, "period": d.qtr, "t_rem": d.quarter_seconds_remaining,
            "pos_team": d.posteam, "home": d.home_team, "away": d.away_team,
            "home_post": d.total_home_score, "away_post": d.total_away_score,
            "is_pass": is_pass, "is_rush": is_rush, "is_kneel": d.play_type == "qb_kneel",
            "passer": d.passer_id.where(is_pass), "rusher": d.rusher_id.where(is_rush), "down": d.down,
            "is_punt": d.play_type == "punt", "is_fg": d.play_type == "field_goal",
            "is_kick": d.play_type == "kickoff",
            "deep": np.where(is_pass & (d.pass_attempt == 1) & (d.sack != 1), (d.air_yards >= 20).astype(float), np.nan),
            "timeout_team": d.timeout_team.where(d.timeout == 1),
            "ytg": d.yardline_100, "togo": d.ydstogo,
            "div_home": "nfl", "div_away": "nfl",
        })
        out.append(t)
    return pd.concat(out, ignore_index=True)


def team_games(p):
    """One row per (game, team) with Q4 behaviour, relative to the score after three quarters."""
    p = p.copy()
    p["home_post"] = p.groupby("game_id").home_post.transform(lambda s: s.ffill().cummax()).fillna(0)
    p["away_post"] = p.groupby("game_id").away_post.transform(lambda s: s.ffill().cummax()).fillna(0)
    # time each row consumes = time until next row in the same period (clipped)
    nxt = p.groupby(["game_id", "period"]).t_rem.shift(-1)
    p["elapsed"] = (p.t_rem - nxt).clip(lower=0, upper=60).fillna(0)
    g3 = p[p.period <= 3].groupby("game_id")[["home_post", "away_post"]].last()
    g4 = p[p.period <= 4].groupby("game_id")[["home_post", "away_post"]].last()
    has4 = p[p.period == 4].groupby("game_id").size()
    meta = p.groupby("game_id")[["home", "away", "year", "div_home", "div_away"]].first()
    games = meta.join(g3.add_suffix("_3")).join(g4.add_suffix("_4"))
    games = games[games.index.isin(has4.index)]

    early = p[p.period <= 3]
    q4 = p[p.period == 4]
    # starters from the first three quarters
    qb1 = early[early.is_pass & early.passer.notna()].groupby(["game_id", "pos_team"]).passer.agg(lambda s: s.value_counts().index[0])
    rr = early[early.is_rush & early.rusher.notna()].copy()
    rr = rr.join(qb1.rename("qb1"), on=["game_id", "pos_team"])
    rr = rr[rr.rusher != rr.qb1]
    rb1 = rr.groupby(["game_id", "pos_team"]).rusher.agg(lambda s: s.value_counts().index[0])
    q4 = q4.join(qb1.rename("qb1"), on=["game_id", "pos_team"]).join(rb1.rename("rb1"), on=["game_id", "pos_team"])
    q4["qb1_pass"] = q4.is_pass & (q4.passer == q4.qb1)
    q4["rb1_rush"] = q4.is_rush & (q4.rusher == q4.rb1)
    q4["qb1_any"] = q4.qb1_pass | (q4.is_rush & (q4.rusher == q4.qb1))
    q4["scrim"] = q4.is_pass | q4.is_rush | q4.is_kneel
    q4["fourth"] = (q4.down == 4) & (q4.is_pass | q4.is_rush | q4.is_punt | q4.is_fg)
    q4["went"] = q4.fourth & (q4.is_pass | q4.is_rush)
    q4["top"] = np.where(q4.is_kick, 0, q4.elapsed)
    for z, (lo, hi) in {"own": (51, 100), "mid": (35, 50), "fg": (0, 34)}.items():
        inz = q4.fourth & (q4.ytg >= lo) & (q4.ytg <= hi)
        q4["f_" + z] = inz
        q4["w_" + z] = inz & q4.went
        q4["k_" + z] = inz & q4.is_fg
        q4["p_" + z] = inz & q4.is_punt
    q4["f_short"] = q4.fourth & (q4.togo <= 3)
    q4["w_short"] = q4.f_short & q4.went
    agg = q4.groupby(["game_id", "pos_team"]).agg(
        passes=("is_pass", "sum"), rushes=("is_rush", "sum"), kneels=("is_kneel", "sum"), plays=("scrim", "sum"),
        top=("top", "sum"), qb1_pass=("qb1_pass", "sum"), qb1_any=("qb1_any", "sum"), rb1_rush=("rb1_rush", "sum"),
        fourth=("fourth", "sum"), went=("went", "sum"), deep=("deep", "sum"), deep_n=("deep", "count"),
        **{f"{a}_{z}": (f"{a}_{z}", "sum") for a in "fwkp" for z in ["own", "mid", "fg"]},
        f_short=("f_short", "sum"), w_short=("w_short", "sum"),
    )
    if "drive_secs" in q4:
        dq = q4[q4.drive_q4_only & q4.drive_id.notna()]
        dr = dq.groupby(["game_id", "drive_id"]).agg(pos_team=("pos_team", lambda s: s.mode().iloc[0] if s.notna().any() else None),
                                                     secs=("drive_secs", "first"), dplays=("scrim", "sum")).reset_index()
        dr = dr[(dr.secs <= 900) & (dr.dplays > 0)]
        dagg = dr.groupby(["game_id", "pos_team"]).agg(drive_secs=("secs", "sum"), drive_plays=("dplays", "sum"))
        agg = agg.join(dagg, how="left")
    to = q4[q4.timeout_team.notna()].groupby(["game_id", "timeout_team"]).size()
    rows = []
    for gid, g in games.iterrows():
        m3 = g.home_post_3 - g.away_post_3
        for team, opp, sign, own3, own4, opp3, opp4, div, odiv in [
            (g.home, g.away, 1, g.home_post_3, g.home_post_4, g.away_post_3, g.away_post_4, g.div_home, g.div_away),
            (g.away, g.home, -1, g.away_post_3, g.away_post_4, g.home_post_3, g.home_post_4, g.div_away, g.div_home),
        ]:
            lead3 = sign * m3
            a = agg.loc[(gid, team)] if (gid, team) in agg.index else None
            r = {"game_id": gid, "year": g.year, "team": team, "div": div, "opp_div": odiv, "lead3": lead3,
                 "pts_q4": own4 - own3, "opp_pts_q4": opp4 - opp3,
                 "timeouts_q4": int(to.get((gid, team), 0)) if len(to) else np.nan}
            if a is not None:
                r.update(a.to_dict())
            rows.append(r)
    return pd.DataFrame(rows)


def summarize(tg, side):
    """side='lead' uses teams ahead after Q3 by the band; 'trail' uses teams behind by the band."""
    res = {}
    for lo, hi, lab in BANDS:
        if side == "lead":
            s = tg[(tg.lead3 >= lo) & (tg.lead3 <= hi)]
        else:
            s = tg[(-tg.lead3 >= lo) & (-tg.lead3 <= hi)]
        s = s.fillna({"passes": 0, "rushes": 0, "kneels": 0, "plays": 0, "top": 0, "qb1_pass": 0, "qb1_any": 0,
                      "rb1_rush": 0, "fourth": 0, "went": 0, "deep": 0, "deep_n": 0,
                      **{f"{a}_{z}": 0 for a in "fwkp" for z in ["own", "mid", "fg"]}, "f_short": 0, "w_short": 0})
        pr = s.passes.sum() / max(s.passes.sum() + s.rushes.sum(), 1)
        with_pass = s[s.passes > 0]
        with_snap = s[s.plays - s.kneels > 0]
        with_rush = s[s.rushes > 0]
        d = {
            "team_games": int(len(s)),
            "q4_pass_rate": float(pr),
            "q4_pass_rate_game_mean": float((s.passes / (s.passes + s.rushes)).replace([np.inf], np.nan).mean()),
            "q4_points_mean": float(s.pts_q4.mean()),
            "q4_points_allowed_mean": float(s.opp_pts_q4.mean()),
            "q4_plays_mean": float(s.plays.mean()),
            "q4_kneels_mean": float(s.kneels.mean()),
            "q4_top_sec_mean": float(s.top.mean()),
            "q4_sec_per_play": float(s.top.sum() / max(s.plays.sum(), 1)),
            "qb1_share_of_q4_passes": float(s.qb1_pass.sum() / max(s.passes.sum(), 1)),
            "pct_games_qb1_passes_in_q4_given_any_pass": float((with_pass.qb1_pass > 0).mean()) if len(with_pass) else None,
            "pct_games_qb1_takes_any_q4_snap_given_any_snap": float((with_snap.qb1_any > 0).mean()) if len(with_snap) else None,
            "rb1_share_of_q4_rushes": float(s.rb1_rush.sum() / max(s.rushes.sum(), 1)),
            "pct_games_rb1_rushes_in_q4_given_any_rush": float((with_rush.rb1_rush > 0).mean()) if len(with_rush) else None,
            "q4_fourth_down_go_rate": float(s.went.sum() / max(s.fourth.sum(), 1)),
            "q4_fourth_downs": int(s.fourth.sum()),
        }
        for z in ["own", "mid", "fg"]:
            n = s["f_" + z].sum()
            d[f"q4_4th_{z}_n"] = int(n)
            d[f"q4_4th_{z}_go"] = float(s["w_" + z].sum() / n) if n else None
            d[f"q4_4th_{z}_fg"] = float(s["k_" + z].sum() / n) if n else None
            d[f"q4_4th_{z}_punt"] = float(s["p_" + z].sum() / n) if n else None
        d["q4_4th_short_go"] = float(s.w_short.sum() / max(s.f_short.sum(), 1))
        d["q4_4th_short_n"] = int(s.f_short.sum())
        if "drive_secs" in s and s.drive_secs.notna().any():
            d["q4_sec_per_play_drive_based"] = float(s.drive_secs.sum() / max(s.drive_plays.sum(), 1))
        if s.deep_n.sum() > 0:
            d["deep_share_of_q4_attempts_airyds20"] = float(s.deep.sum() / s.deep_n.sum())
        if "timeouts_q4" in s and s.timeouts_q4.notna().any():
            d["q4_timeouts_mean"] = float(s.timeouts_q4.mean())
        res[lab] = d
    return res


def main():
    res = {}
    cfb = team_games(load_cfb([2022, 2023, 2024]))
    cfb.to_csv(os.path.join(OUT, "out", "cfb_team_games.csv.gz"), index=False)
    fbs_both = cfb[(cfb["div"] == "fbs") & (cfb["opp_div"] == "fbs")]
    fbs_vs_fcs = cfb[(cfb["div"] == "fbs") & (cfb["opp_div"] == "fcs")]
    res["cfb_fbs_vs_fbs_2022_2024"] = {"lead": summarize(fbs_both, "lead"), "trail": summarize(fbs_both, "trail")}
    res["cfb_fbs_vs_fcs_2022_2024_fbs_side"] = {"lead": summarize(fbs_vs_fcs, "lead")}
    for y in [2022, 2023, 2024]:
        res[f"cfb_fbs_vs_fbs_{y}"] = {"lead": summarize(fbs_both[fbs_both.year == y], "lead")}
    nfl = team_games(load_nfl(list(range(2018, 2026))))
    nfl.to_csv(os.path.join(OUT, "out", "nfl_team_games.csv.gz"), index=False)
    res["nfl_reg_2018_2025"] = {"lead": summarize(nfl, "lead"), "trail": summarize(nfl, "trail")}
    with open(os.path.join(OUT, "out", "garbage_time.json"), "w") as f:
        json.dump(res, f, indent=1)
    print("wrote", os.path.join(OUT, "out", "garbage_time.json"))


if __name__ == "__main__":
    main()
