"""Real FBS play by play year by year, 2014-2025 (world reader lane R3).

Reads cfbfastR play-by-play (sportsdataverse-data release cfbfastR_cfb_pbp, play_by_play_YEAR.parquet), saved in DIR.
FBS v FBS regular-season games only. The two-point tries, extra points and fair catches come from the play text, so they are close, not exact.

Final scores come from the cfb_schedules release (cfb_schedules_YEAR.parquet in SCHED).

Usage: python3 college_plays.py DIR SCHED > college_plays.json
Development checks only, never in-game tuning.
"""
import json, re, sys
import numpy as np
import pandas as pd

D, SD = sys.argv[1], sys.argv[2]
YEARS = range(2014, 2026)
COLS = ["season", "season_type", "game_id", "pos_team", "def_pos_team", "home", "away", "home_team_division", "away_team_division",
        "play_type", "play_text", "down", "distance", "yards_to_goal", "yards_gained", "EPA", "penalty_detail", "penalty_flag",
        "penalty_declined", "penalty_offset", "period", "drive_id", "drive_result", "drive_start_yards_to_goal", "rush", "pass",
        "sack", "int", "completion", "pass_attempt", "kickoff_play", "kickoff_tb", "kickoff_fair_catch", "kickoff_onside",
        "yds_kickoff_return", "punt_play", "punt_tb", "punt_fair_catch", "punt_blocked", "yds_punted", "yds_punt_return",
        "fg_inds", "fg_made", "yds_fg", "spread", "over_under", "pos_team_score", "def_pos_team_score", "wp_before", "TimeSecsRem",
        "turnover", "scoring", "home_team_pregame_elo"]


def r(x, n=3):
    try:
        x = float(x)
    except (TypeError, ValueError):
        return None
    return None if np.isnan(x) else round(x, n)


def share(mask):
    return r(np.mean(mask), 4) if len(mask) else None


TWO = re.compile(r"two[- ]point|2[- ]?pt\b|2 point", re.I)
TWO_FAIL = re.compile(r"fail|no good|unsuccessful", re.I)
XP_MISS = re.compile(r"(?:kick|pat|extra point)\W+(?:is\W+)?(?:missed|no good|failed|blocked)", re.I)
TD = re.compile(r"touchdown|\bTD\b", re.I)

seasons, pen_kinds, teams, lines = {}, {}, [], []
for y in YEARS:
    p = pd.read_parquet(f"{D}/play_by_play_{y}.parquet", columns=COLS)
    p = p[(p.season_type == "regular") & (p.home_team_division == "fbs") & (p.away_team_division == "fbs")]
    tg = p.game_id.nunique() * 2
    scrim = p[((p.rush == 1) | (p["pass"] == 1)) & p.down.between(1, 4)]
    drop = scrim[scrim["pass"] == 1]
    runs = scrim[scrim.rush == 1]
    att = scrim[(scrim.pass_attempt == 1) & (scrim.sack != 1)]
    neutral = scrim[scrim.down.isin([1, 2]) & scrim.wp_before.between(0.2, 0.8) & (scrim.TimeSecsRem > 120)]
    fg = p[p.fg_inds == 1]
    fgb = {}
    for lo, hi, k in [(0, 29, "under_30"), (30, 39, "30_39"), (40, 49, "40_49"), (50, 70, "50_plus")]:
        b = fg[fg.yds_fg.between(lo, hi)]
        fgb[k] = {"att_per_team_game": r(len(b) / tg, 3), "made_pct": share(b.fg_made == 1)}
    tds = p[p.play_type.fillna("").str.contains("Touchdown") | (p.play_text.fillna("").str.contains(TD) & (p.scoring == True))]
    two = tds[tds.play_text.str.contains(TWO)]
    xp = tds[~tds.play_text.str.contains(TWO)]
    four = p[(p.down == 4) & ((p.rush == 1) | (p["pass"] == 1) | (p.punt_play == 1) | (p.fg_inds == 1))]
    go = four[(four.rush == 1) | (four["pass"] == 1)]
    short_opp = four[(four.distance <= 3) & (four.yards_to_goal <= 50)]
    ko = p[(p.kickoff_play == 1) & (p.kickoff_onside != 1)]
    ko_ret = ko[(ko.kickoff_tb != 1) & (ko.kickoff_fair_catch != 1) & ko.yds_kickoff_return.notna()]
    pu = p[(p.punt_play == 1) & (p.punt_blocked != 1)]
    pu_ret = pu[(pu.punt_fair_catch != 1) & (pu.punt_tb != 1) & (pu.yds_punt_return.fillna(0) != 0)]
    pen = p[(p.penalty_flag == True) & (p.penalty_declined != True) & (p.penalty_offset != True)]
    dr = p.drop_duplicates("drive_id")
    drives = dr[dr.drive_result.notna() & ~dr.drive_result.fillna("").str.contains("KICKOFF|END OF", case=False)]
    ot = p.groupby("game_id").period.max()
    seasons[y] = {
        "games": int(tg / 2), "plays_per_team_game": r(len(scrim) / tg, 2),
        "pass_rate": r(len(drop) / max(len(scrim), 1), 4), "neutral_pass_rate": r((neutral["pass"] == 1).mean(), 4),
        "completion_pct": share(att.completion == 1), "yards_per_attempt": r(att.yards_gained.mean(), 3),
        "sack_rate": share(drop.sack == 1), "int_rate": share(att["int"] == 1), "yards_per_carry": r(runs.yards_gained.mean(), 3),
        "yards_per_play": r(scrim.yards_gained.mean(), 3), "epa_per_play": r(scrim.EPA.mean(), 4),
        "explosive_run_rate_10plus": share(runs.yards_gained >= 10), "explosive_pass_rate_20plus": share(att.yards_gained >= 20),
        "fg_by_distance": fgb, "fg_att_per_team_game": r(len(fg) / tg, 3), "fg_pct": share(fg.fg_made == 1),
        "touchdowns_per_team_game": r(len(tds) / tg, 3), "two_point_tries_per_team_game": r(len(two) / tg, 3),  # play-text match; 2014 text formats undercount
        "two_point_conv": share(~two.play_text.str.contains(TWO_FAIL)), "xp_pct": share(~xp.play_text.str.contains(XP_MISS)),
        "fourth_down_go_rate": share((four.rush == 1) | (four["pass"] == 1)), "fourth_short_opp_go_rate": share((short_opp.rush == 1) | (short_opp["pass"] == 1)),
        "fourth_down_conv": share(go.yards_gained >= go.distance),
        "punts_per_team_game": r(len(pu) / tg, 3), "punt_gross": r(pu.yds_punted.mean(), 2), "punt_fair_catch_rate": share(pu.punt_fair_catch == 1),
        "punt_return_avg": r(pu_ret.yds_punt_return.mean(), 2), "kickoff_touchback_rate": share(ko.kickoff_tb == 1),
        "kickoff_fair_catch_rate": share(ko.kickoff_fair_catch == 1), "kickoff_return_avg": r(ko_ret.yds_kickoff_return.mean(), 2),
        "onside_kicks_per_team_game": r((p.kickoff_onside == 1).sum() / tg, 3),
        "penalties_per_team_game": r(len(pen) / tg, 3), "turnovers_per_team_game": r(p[p.turnover == 1].shape[0] / tg, 3),
        "drives_per_team_game": r(len(drives) / tg, 2), "drive_start_yards_to_goal": r(drives.drive_start_yards_to_goal.mean(), 2),
        "drive_results": {str(k): r(v, 4) for k, v in drives.drive_result.value_counts(normalize=True).head(12).items()},
        "overtime_share": share(ot >= 5), "periods_reached": {int(k): int(v) for k, v in ot.value_counts().sort_index().items()},
    }
    pen_kinds[y] = {str(k): r(n / tg, 4) for k, n in pen.penalty_detail.value_counts().head(20).items()}
    off = scrim.groupby("pos_team").agg(plays=("EPA", "size"), epa=("EPA", "mean"), ypp=("yards_gained", "mean"), pass_rate=("pass", "mean"))
    games = p.groupby("pos_team").game_id.nunique()
    off["plays_per_game"] = off.plays / games
    off["season"] = y
    teams.append(off)
    sc = pd.read_parquet(f"{SD}/cfb_schedules_{y}.parquet", columns=["game_id", "home_points", "away_points"])
    sc.index = sc.game_id.astype(str)
    g = p.groupby("game_id").agg(spread=("spread", "first"), ou=("over_under", "first"))
    g.index = g.index.astype(str)
    g = g.join(sc[["home_points", "away_points"]].rename(columns={"home_points": "hs", "away_points": "as_"}), how="inner")
    lines.append(g.assign(season=y))

out = {"source": "cfbfastR play-by-play (sportsdataverse-data release cfbfastR_cfb_pbp), FBS v FBS regular seasons 2014-2025",
       "holes": "No open college play by play before 2014 or below FCS; no participation data (snap counts, personnel, coverage) at any college level.",
       "by_season": seasons, "penalty_kinds_per_team_game": pen_kinds}
t = pd.concat(teams)
t = t[t.plays >= 400]
out["team_season_ranges"] = {"years": "2014-2025, FBS offenses with 400+ plays in FBS v FBS games",
                             **{k: {f"p{q}": r(t[k].quantile(q / 100), 3) for q in (5, 25, 50, 75, 95)} for k in ["epa", "ypp", "pass_rate", "plays_per_game"]}}
bl = pd.concat(lines)
bl = bl[bl.spread.notna() & (bl.spread != 0)].copy()
bl["margin"] = bl.hs - bl.as_
sign = -1 if bl[["spread", "margin"]].corr().iloc[0, 1] < 0 else 1  # which sign means the home team is favored
bl["home_line"] = bl.spread * sign  # positive = home favored by that many points
bl["fav_won"] = np.where(bl.home_line > 0, bl.margin > 0, bl.margin < 0)
bl["ats"] = bl.margin - bl.home_line
bands = pd.cut(bl.home_line.abs(), [0, 2.5, 3.5, 6.5, 9.5, 13.5, 20.5, 80], labels=["0_2.5", "3_3.5", "4_6.5", "7_9.5", "10_13.5", "14_20.5", "21_plus"])
out["betting_lines"] = {"years": "2014-2025, FBS v FBS regular season, the line cfbfastR carries (CollegeFootballData consensus)",
                        "games": int(len(bl)), "favorite_win_rate_by_spread": {str(k): {"games": int(len(v)), "fav_win": r(v.fav_won.mean(), 4)} for k, v in bl.groupby(bands, observed=True)},
                        "note": "totals left out: the over/under cfbfastR carries does not line up with final scores (36% overs), so it is not a closing total",
                        "spread_mae": r(bl.ats.abs().mean(), 3), "home_cover_rate": share(bl.ats > 0), "push_rate": share(bl.ats == 0),
                        "dog_14plus_win_rate": r(1 - bl[bl.home_line.abs() >= 14].fav_won.mean(), 4)}
json.dump(out, sys.stdout, indent=1, default=str)
