"""Real NFL game targets for the match engine checks (section 3, topic 10).

Reads nflverse play-by-play (https://github.com/nflverse/nflverse-data,
release "pbp", files play_by_play_YEAR.csv.gz) for the regular seasons given
on the command line and writes per-team-game averages, rates, kicking by
distance, fourth downs, season leaders and an expected-points table by down,
distance and field position (nflfastR's own EP model, averaged).

Usage: python3 nfl_game_targets.py DIR 2021 2022 2023 2024 2025 > nfl_game_targets.json
These are development checks only, never in-game tuning.
"""
import csv, gzip, json, sys
from collections import defaultdict

def f(x):
    try:
        return float(x)
    except (TypeError, ValueError):
        return None

def main():
    folder, years = sys.argv[1], sys.argv[2:]
    games = {}
    team_games = defaultdict(lambda: defaultdict(float))
    c = defaultdict(float)
    fg = defaultdict(lambda: [0, 0])
    ep = defaultdict(lambda: [0.0, 0])
    leaders = defaultdict(lambda: defaultdict(float))
    for y in years:
        with gzip.open(f"{folder}/pbp_{y}.csv.gz", "rt", newline="") as fh:
            for r in csv.DictReader(fh):
                if r["season_type"] != "REG":
                    continue
                gid, pos = r["game_id"], r["posteam"]
                games[gid] = (f(r["home_score"]), f(r["away_score"]))
                if not pos:
                    continue
                tg = team_games[(gid, pos)]
                pt = r["play_type"]
                down = f(r["down"])
                # Expected points by down, distance and field position.
                if down and r["ep"] and r["yardline_100"] and pt in ("run", "pass", "punt", "field_goal"):
                    d = int(down)
                    togo = int(f(r["ydstogo"]))
                    tb = 1 if togo <= 1 else 2 if togo <= 2 else 3 if togo <= 3 else 6 if togo <= 6 else 10 if togo <= 10 else 15 if togo <= 15 else 20
                    yb = min(99, max(1, int(f(r["yardline_100"])))) // 5 * 5
                    e = ep[(d, tb, yb)]
                    e[0] += f(r["ep"]); e[1] += 1
                if r["penalty"] == "1" and r["penalty_team"]:
                    team_games[(gid, r["penalty_team"])]["penalties"] += 1
                    team_games[(gid, r["penalty_team"])]["penalty_yards"] += f(r["penalty_yards"]) or 0
                if pt == "pass":
                    if r["qb_dropback"] == "1":
                        tg["dropbacks"] += 1
                    if r["sack"] == "1":
                        tg["sacks"] += 1
                        tg["sack_yards"] += -(f(r["yards_gained"]) or 0)
                    elif r["pass_attempt"] == "1":
                        tg["att"] += 1
                        tg["cmp"] += r["complete_pass"] == "1"
                        tg["pass_yards"] += f(r["passing_yards"]) or 0
                        tg["pass_td"] += r["pass_touchdown"] == "1"
                        tg["int"] += r["interception"] == "1"
                        if (f(r["yards_gained"]) or 0) >= 20:
                            tg["explosive_pass"] += 1
                        if r["passer_player_id"]:
                            leaders[(y, "pass_yards")][r["passer_player_id"]] += f(r["passing_yards"]) or 0
                        if r["receiver_player_id"] and r["complete_pass"] == "1":
                            leaders[(y, "rec_yards")][r["receiver_player_id"]] += f(r["receiving_yards"]) or 0
                    if r["qb_scramble"] == "1":
                        tg["scrambles"] += 1
                    if r["sack"] == "1":
                        for k in ("sack_player_id", "half_sack_1_player_id", "half_sack_2_player_id"):
                            if r.get(k):
                                leaders[(y, "sacks")][r[k]] += 1 if k == "sack_player_id" else 0.5
                if pt == "run" and r["qb_kneel"] != "1":
                    tg["carries"] += 1
                    yds = f(r["rushing_yards"]) or 0
                    tg["rush_yards"] += yds
                    tg["rush_td"] += r["rush_touchdown"] == "1"
                    if yds >= 10:
                        tg["explosive_run"] += 1
                    if r["rusher_player_id"]:
                        leaders[(y, "rush_yards")][r["rusher_player_id"]] += yds
                if pt in ("run", "pass"):
                    tg["plays"] += 1
                    if r["fumble_lost"] == "1":
                        tg["fumbles_lost"] += 1
                    if down == 3:
                        tg["third"] += 1
                        tg["third_conv"] += r["third_down_converted"] == "1"
                    if down == 4:
                        tg["fourth_go"] += 1
                        tg["fourth_conv"] += r["fourth_down_converted"] == "1"
                    if r["epa"]:
                        tg["epa"] += f(r["epa"])
                    tg["success"] += r["success"] == "1"
                if pt == "field_goal" and r["kick_distance"]:
                    dist = f(r["kick_distance"])
                    band = "under30" if dist < 30 else "30-39" if dist < 40 else "40-49" if dist < 50 else "50+"
                    fg[band][0] += 1
                    fg[band][1] += r["field_goal_result"] == "made"
                if pt == "extra_point":
                    c["xp"] += 1
                    c["xp_made"] += r["extra_point_result"] == "good"
                if pt == "punt" and r["kick_distance"]:
                    c["punts"] += 1
                    c["punt_gross"] += f(r["kick_distance"])
                    c["punt_inside20"] += r["punt_inside_twenty"] == "1"
                if pt == "kickoff":
                    c["kickoffs"] += 1
                    c["kickoff_touchbacks"] += r["touchback"] == "1"
    n = len(team_games)
    tot = defaultdict(float)
    for tg in team_games.values():
        for k, v in tg.items():
            tot[k] += v
    pts = [s for g in games.values() for s in g if s is not None]
    mean_pts = sum(pts) / len(pts)
    sd_pts = (sum((p - mean_pts) ** 2 for p in pts) / len(pts)) ** 0.5
    margins = [abs(g[0] - g[1]) for g in games.values() if None not in g]
    out = {
        "source": "nflverse play-by-play, regular seasons " + ", ".join(years),
        "team_games": n,
        "per_team_game": {k: round(v / n, 3) for k, v in sorted(tot.items())},
        "points_per_team_game": round(mean_pts, 2),
        "points_sd": round(sd_pts, 2),
        "share_games_decided_by_8_or_less": round(sum(m <= 8 for m in margins) / len(margins), 3),
        "rates": {
            "completion_pct": round(tot["cmp"] / tot["att"], 4),
            "sack_rate": round(tot["sacks"] / tot["dropbacks"], 4),
            "yards_per_attempt": round(tot["pass_yards"] / tot["att"], 3),
            "int_rate": round(tot["int"] / tot["att"], 4),
            "pass_td_rate": round(tot["pass_td"] / tot["att"], 4),
            "yards_per_carry": round(tot["rush_yards"] / tot["carries"], 3),
            "pass_rate": round(tot["dropbacks"] / tot["plays"], 4),
            "third_down_conversion": round(tot["third_conv"] / tot["third"], 4),
            "fourth_down_conversion": round(tot["fourth_conv"] / tot["fourth_go"], 4),
            "explosive_run_rate": round(tot["explosive_run"] / tot["carries"], 4),
            "explosive_pass_rate": round(tot["explosive_pass"] / tot["att"], 4),
            "success_rate": round(tot["success"] / tot["plays"], 4),
            "epa_per_play": round(tot["epa"] / tot["plays"], 4),
            "xp_pct": round(c["xp_made"] / c["xp"], 4),
            "punt_gross": round(c["punt_gross"] / c["punts"], 2),
            "punt_inside20_rate": round(c["punt_inside20"] / c["punts"], 4),
            "kickoff_touchback_rate": round(c["kickoff_touchbacks"] / c["kickoffs"], 4),
        },
        "field_goal_pct_by_distance": {b: {"attempts": v[0], "pct": round(v[1] / v[0], 4)} for b, v in sorted(fg.items())},
        "season_leaders": {
            stat: sorted(
                [round(max(leaders[(y, stat)].values()), 1) for y in years]
            )
            for stat in ("pass_yards", "rush_yards", "rec_yards", "sacks")
        },
        "expected_points": [
            {"down": d, "togo": t, "yardline_100": y, "ep": round(v[0] / v[1], 3), "n": v[1]}
            for (d, t, y), v in sorted(ep.items())
            if v[1] >= 20
        ],
    }
    json.dump(out, sys.stdout, indent=1)

main()
