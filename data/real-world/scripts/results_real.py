"""Real year-to-year stability for growth from results (afm/game-reps/proposal.md).

Growth from results moves a skill by the season's shown level over his rating,
weighted by how much a season of that stat really tells: half the move at the
volume where a real season's stat is half signal. This script measures, for
each of the engine's skill reads (`crates/afm-engine/src/results/mod.rs`,
`Read`), the real stat's year-to-year correlation among NFL players who
qualified in both seasons, and their mean volume, regular seasons 2018-2025,
and writes results_real.json:

- reads: per read, the stat used, r (Pearson, consecutive seasons, both
  qualified), n pairs, the qualifying volume and the mean volume V of the
  qualified seasons, and sd, the spread of the stat across qualified seasons.
- gaps: the reads no free file covers (they take the proposed 0.35).

Reads nflverse release files (https://github.com/nflverse/nflverse-data/releases),
downloaded with curl into DIR when missing:
  pbp/play_by_play_YEAR.csv.gz, ftn_charting/ftn_charting_YEAR.csv (2022+),
  pfr_advstats/advstats_season_{def,pass,rush}.csv, snap_counts/snap_counts_YEAR.csv,
  nextgen_stats/ngs_{rushing,receiving}.csv.gz

Usage: python3 results_real.py [DIR] [OUT]
Checks for the engine's growth tables, never in-game tuning.
"""
import json
import os
import subprocess
import sys

import numpy as np
import pandas as pd

D = sys.argv[1] if len(sys.argv) > 1 else "/tmp/claude-0/nflv"
OUT = sys.argv[2] if len(sys.argv) > 2 else os.path.normpath(
    os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "results_real.json"))
os.makedirs(D, exist_ok=True)
BASE = "https://github.com/nflverse/nflverse-data/releases/download"
YEARS = list(range(2018, 2026))


def fetch(tag, name):
    path = os.path.join(D, name)
    if not os.path.exists(path) or os.path.getsize(path) < 1000:
        subprocess.run(["curl", "-sSL", "-o", path, f"{BASE}/{tag}/{name}"], check=True)
    return path


PBP_COLS = ["game_id", "play_id", "season", "season_type", "play_type", "pass_attempt", "sack", "interception",
            "complete_pass", "air_yards", "cpoe", "cp", "passer_player_id", "receiver_player_id",
            "rusher_player_id", "fumble", "fumbled_1_player_id", "rush_attempt", "qb_scramble",
            "field_goal_result", "kick_distance", "extra_point_result", "kicker_player_id", "punter_player_id",
            "punt_inside_twenty", "yardline_100", "punt_blocked", "success", "solo_tackle_1_player_id",
            "assist_tackle_1_player_id", "assist_tackle_2_player_id", "tackle_with_assist_1_player_id",
            "defteam", "two_point_attempt"]


def pbp():
    parts = []
    for y in YEARS:
        p = fetch("pbp", f"play_by_play_{y}.csv.gz")
        df = pd.read_csv(p, usecols=lambda c: c in PBP_COLS, low_memory=False)
        parts.append(df[df.season_type == "REG"])
    return pd.concat(parts, ignore_index=True)


def ftn():
    parts = []
    for y in range(2022, 2026):
        try:
            parts.append(pd.read_csv(fetch("ftn_charting", f"ftn_charting_{y}.csv"), low_memory=False))
        except Exception:
            pass
    if not parts:
        return None
    f = pd.concat(parts, ignore_index=True)
    return f.rename(columns={"nflverse_game_id": "game_id", "nflverse_play_id": "play_id"})


def y2y(df, key, stat, vol, qual):
    """Year-to-year Pearson r of `stat` among player seasons with `vol` >= qual in both years."""
    d = df[df[vol] >= qual][[key, "season", stat, vol]].dropna()
    nxt = d.copy()
    nxt["season"] = nxt["season"] - 1
    m = d.merge(nxt, on=[key, "season"], suffixes=("", "_next"))
    if len(m) < 20:
        return None
    r = float(np.corrcoef(m[stat], m[stat + "_next"])[0, 1])
    return {"r": round(r, 3), "pairs": int(len(m)), "qualify": qual, "V": round(float(d[vol].mean()), 1),
            "sd": round(float(d[stat].std()), 4), "seasons": int(len(d))}


def per_season(df, key, value, name):
    g = df.groupby([key, "season"])[value].agg(["mean", "count"]).reset_index()
    return g.rename(columns={key: "id", "mean": name, "count": "n"})


def main():
    out = {}
    p = pbp()
    passes = p[(p.pass_attempt == 1) & (p.sack == 0) & p.air_yards.notna() & p.cpoe.notna()].copy()
    passes["over"] = passes.cpoe / 100.0
    for name, lo, hi, q in (("ShortThrow", -99, 10, 100), ("MediumThrow", 10, 20, 50), ("DeepThrow", 20, 99, 30)):
        d = passes[(passes.air_yards >= lo) & (passes.air_yards < hi)]
        out[name] = {"stat": f"CPOE, air yards {lo}..{hi}", **y2y(per_season(d, "passer_player_id", "over", "x"),
                                                                    "id", "x", "n", q)}
    att = p[(p.pass_attempt == 1) & (p.sack == 0) & p.passer_player_id.notna()].copy()
    att["picked"] = -att.interception
    out["Picks"] = {"stat": "-INT per attempt", **y2y(per_season(att, "passer_player_id", "picked", "x"),
                                                     "id", "x", "n", 200)}
    rec = passes[passes.receiver_player_id.notna()].copy()
    out["Hands"] = {"stat": "receiver catch rate over expected (complete - cp)",
                    **y2y(per_season(rec, "receiver_player_id", "over", "x"), "id", "x", "n", 50)}
    out["Tracking"] = {"stat": "receiver catch rate over expected, air 20+",
                       **y2y(per_season(rec[rec.air_yards >= 20], "receiver_player_id", "over", "x"),
                             "id", "x", "n", 15)}
    # Fumbles per touch: carries and receptions.
    touches = pd.concat([
        p[(p.rush_attempt == 1) & p.rusher_player_id.notna()][["rusher_player_id", "season", "fumble",
                                                               "fumbled_1_player_id"]]
        .rename(columns={"rusher_player_id": "id"}),
        p[(p.complete_pass == 1) & p.receiver_player_id.notna()][["receiver_player_id", "season", "fumble",
                                                                 "fumbled_1_player_id"]]
        .rename(columns={"receiver_player_id": "id"}),
    ])
    touches["safe"] = -((touches.fumble == 1) & (touches.fumbled_1_player_id == touches.id)).astype(float)
    out["Security"] = {"stat": "-fumbles per touch", **y2y(per_season(touches, "id", "safe", "x"), "id", "x", "n",
                                                          150)}
    # Kicks for the distance: made against the league's rate at that yard.
    fg = p[p.field_goal_result.notna() & p.kicker_player_id.notna()].copy()
    fg["made"] = (fg.field_goal_result == "made").astype(float)
    fg["yd"] = fg.kick_distance.round()
    fg["over"] = fg.made - fg.groupby("yd").made.transform("mean")
    xp = p[p.extra_point_result.notna() & p.kicker_player_id.notna()].copy()
    xp["over"] = (xp.extra_point_result == "good").astype(float) - (xp.extra_point_result == "good").mean()
    short = pd.concat([fg[fg.kick_distance < 40][["kicker_player_id", "season", "over"]],
                       xp[["kicker_player_id", "season", "over"]]])
    out["ShortKick"] = {"stat": "FG under 40 and XP made over the league's rate at the yard",
                        **y2y(per_season(short, "kicker_player_id", "over", "x"), "id", "x", "n", 30)}
    out["LongKick"] = {"stat": "FG 40+ made over the league's rate at the yard",
                       **y2y(per_season(fg[fg.kick_distance >= 40], "kicker_player_id", "over", "x"), "id", "x", "n",
                             10)}
    pu = p[(p.play_type == "punt") & p.punter_player_id.notna() & (p.punt_blocked != 1)].copy()
    pu["in20"] = pu.punt_inside_twenty.fillna(0)
    pu["band"] = (pu.yardline_100 // 5)
    pu["over"] = pu.in20 - pu.groupby("band").in20.transform("mean")
    out["Placement"] = {"stat": "punts inside the 20 over the league's rate from that spot",
                        **y2y(per_season(pu, "punter_player_id", "over", "x"), "id", "x", "n", 30)}
    # Run stops by defenders, per defensive snap.
    runs = p[(p.rush_attempt == 1) & (p.qb_scramble != 1) & (p.success == 0)]
    stoppers = pd.concat([runs[c].dropna() for c in ("solo_tackle_1_player_id", "assist_tackle_1_player_id",
                                                      "assist_tackle_2_player_id", "tackle_with_assist_1_player_id")
                          if c in runs.columns])
    _ = stoppers
    # FTN charting (2022+): out of the pocket, contested.
    f = ftn()
    if f is not None:
        j = passes.merge(f[["game_id", "play_id", "is_qb_out_of_pocket", "is_contested_ball", "is_throw_away"]],
                         on=["game_id", "play_id"])
        j = j[j.is_throw_away != True]  # noqa: E712
        mv = j[j.is_qb_out_of_pocket == True]  # noqa: E712
        out["MovingThrow"] = {"stat": "CPOE out of the pocket (FTN, 2022+)",
                              **y2y(per_season(mv, "passer_player_id", "over", "x"), "id", "x", "n", 25)}
        ct = j[(j.is_contested_ball == True) & j.receiver_player_id.notna()]  # noqa: E712
        out["Contested"] = {"stat": "receiver catch rate over expected on contested balls (FTN, 2022+)",
                            **y2y(per_season(ct, "receiver_player_id", "over", "x"), "id", "x", "n", 12)}
    # PFR advanced: pressure to sack, broken tackles, yards after contact,
    # missed tackles, completions allowed, pressures.
    pas = pd.read_csv(fetch("pfr_advstats", "advstats_season_pass.csv"))
    sacks = p[(p.sack == 1) & p.passer_player_id.notna()].groupby(["passer_player_id", "season"]).size()
    _ = sacks
    pas = pas[pas.times_pressured > 0].copy()
    # PFR counts sacks inside pressures; nflverse's season file has no sacks,
    # so the hurries and hits stand in (a pressure that isn't a sack).
    pas["escape"] = (pas.times_hurried + pas.times_hit) / pas.times_pressured
    out["Pocket"] = {"stat": "share of pressures that are hurries or hits, not sacks (PFR)",
                     **y2y(pas, "pfr_id", "escape", "times_pressured", 60)}
    ru = pd.read_csv(fetch("pfr_advstats", "advstats_season_rush.csv"))
    ru = ru[ru.att > 0].copy()
    ru["brk"] = ru.brk_tkl / ru.att
    out["Broken"] = {"stat": "broken tackles per carry (PFR)", **y2y(ru, "pfr_id", "brk", "att", 100)}
    out["LegDrive"] = {"stat": "yards after contact per carry (PFR)", **y2y(ru, "pfr_id", "yac_att", "att", 100)}
    de = pd.read_csv(fetch("pfr_advstats", "advstats_season_def.csv"))
    de["att_tk"] = de.comb + de.m_tkl
    de = de[de.att_tk > 0].copy()
    de["made"] = -de.m_tkl / de.att_tk
    out["Tackle"] = {"stat": "-missed tackles per attempt (PFR)", **y2y(de, "pfr_id", "made", "att_tk", 40)}
    cov = de[de.tgt > 0].copy()
    cov["held"] = -cov.cmp / cov.tgt
    for k in ("ManCover", "ZoneCover"):
        out[k] = {"stat": "-completions allowed per target (PFR; man and zone together)",
                  **y2y(cov, "pfr_id", "held", "tgt", 40)}
    sn = pd.concat([pd.read_csv(fetch("snap_counts", f"snap_counts_{y}.csv")) for y in YEARS])
    sn = sn[sn.game_type == "REG"].groupby(["pfr_player_id", "season"]).defense_snaps.sum().reset_index()
    pr = de.merge(sn, left_on=["pfr_id", "season"], right_on=["pfr_player_id", "season"])
    pr = pr[pr.defense_snaps > 0].copy()
    pr["prss_snap"] = pr.prss / pr.defense_snaps
    rushers = pr[pr.pos.isin(["DE", "DT", "OLB", "EDGE", "DL", "NT"])]
    for k in ("PowerRush", "FinesseRush"):
        out[k] = {"stat": "pressures per defensive snap, edge and interior rushers (PFR; moves together)",
                  **y2y(rushers, "pfr_id", "prss_snap", "defense_snaps", 300)}
    # NGS: rush yards over expected, separation, YAC over expected.
    ng = pd.read_csv(fetch("nextgen_stats", "ngs_rushing.csv.gz"))
    ng = ng[(ng.week == 0) & (ng.season_type == "REG")]
    out["Vision"] = {"stat": "rush yards over expected per carry (NGS)",
                     **y2y(ng, "player_gsis_id", "rush_yards_over_expected_per_att", "rush_attempts", 100)}
    nr = pd.read_csv(fetch("nextgen_stats", "ngs_receiving.csv.gz"))
    nr = nr[(nr.week == 0) & (nr.season_type == "REG")]
    out["Separation"] = {"stat": "average separation at the catch point (NGS)",
                         **y2y(nr, "player_gsis_id", "avg_separation", "targets", 50)}
    out["AfterCatch"] = {"stat": "YAC over expected per reception (NGS)",
                         **y2y(nr, "player_gsis_id", "avg_yac_above_expectation", "receptions", 40)}
    gaps = {
        "Poise": "no free pressure flag on throws (FTN charts none); proposed 0.35",
        "Release": "no free press or jam charting; proposed 0.35",
        "PassBlock": "pressures allowed per pass block are PFF's; proposed 0.35",
        "RunBlock": "run block wins are PFF's or ESPN's; proposed 0.35",
        "Anchor": "no free power-rush charting; proposed 0.35",
        "Shed": "no free run-block-shed charting; proposed 0.35",
        "Recognition": "run stops per run snap need snap participation by play; proposed 0.35",
    }
    with open(OUT, "w") as fh:
        json.dump({"source": "nflverse regular seasons 2018-2025 (FTN 2022+)", "reads": out, "gaps": gaps}, fh,
                  indent=1)
    for k, v in out.items():
        print(f"{k:<12} r {v['r']:+.3f}  pairs {v['pairs']:>4}  V {v['V']:>7.1f}  sd {v['sd']:.4f}  {v['stat']}")


if __name__ == "__main__":
    main()
