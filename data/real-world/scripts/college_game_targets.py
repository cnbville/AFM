"""Real college game targets for the match engine checks (section 3, topic 10).

Reads cfbfastR play-by-play (sportsdataverse-data release "cfbfastR_cfb_pbp",
files play_by_play_YEAR.csv) and writes per-team-game averages and rates for
FBS-vs-FBS and FCS-vs-FCS games, field goals by distance and FBS season
leaders. Development checks only, never in-game tuning.

Usage: python3 college_game_targets.py DIR 2022 2023 2024 > college_game_targets.json
"""
import csv, json, sys
from collections import defaultdict

csv.field_size_limit(10**8)

def f(x):
    try:
        return float(x)
    except (TypeError, ValueError):
        return None

def main():
    folder, years = sys.argv[1], sys.argv[2:]
    out = {"source": "cfbfastR play-by-play (sportsdataverse), seasons " + ", ".join(years), "divisions": {}}
    acc = {d: {"tg": defaultdict(lambda: defaultdict(float)), "games": {}, "fg": defaultdict(lambda: [0, 0]), "c": defaultdict(float)} for d in ("fbs", "fcs")}
    leaders = defaultdict(lambda: defaultdict(float))
    for y in years:
        with open(f"{folder}/cfb_{y}.csv", newline="") as fh:
            for r in csv.DictReader(fh):
                if r["season_type"] != "regular":
                    continue
                hd, ad = r["home_team_division"], r["away_team_division"]
                if hd != ad or hd not in acc:
                    continue
                a = acc[hd]
                gid, pos = r["game_id"], r["pos_team"]
                a["games"][gid] = (r["home_team"], r["away_team"])
                if not pos:
                    continue
                tg = a["tg"][(gid, pos)]
                # Final score: the highest score each side shows during the game.
                for team, key in ((pos, "pos_team_score"), (r["def_pos_team"], "def_pos_team_score")):
                    sc = f(r[key])
                    if team and sc is not None:
                        side = a["tg"][(gid, team)]
                        side["final_score"] = max(side["final_score"], sc)
                if r["penalty_flag"] == "TRUE" and r["penalty_declined"] != "TRUE" and r["penalty_offset"] != "TRUE":
                    tg["penalties_on_plays_by_either"] += 1
                rush, pas = r["rush"] == "1", r["pass"] == "1"
                down = f(r["down"])
                if pas:
                    tg["dropbacks"] += 1
                    if r["sack"] in ("1", "TRUE"):
                        tg["sacks"] += 1
                    elif r["pass_attempt"] == "1":
                        tg["att"] += 1
                        tg["cmp"] += r["completion"] == "1"
                        yds = f(r["yards_gained"]) or 0
                        if r["completion"] == "1":
                            tg["pass_yards"] += yds
                            if hd == "fbs" and r["passer_player_name"]:
                                leaders[(y, "pass_yards")][r["passer_player_name"] + r["pos_team"]] += yds
                            if hd == "fbs" and r["receiver_player_name"]:
                                leaders[(y, "rec_yards")][r["receiver_player_name"] + r["pos_team"]] += yds
                        tg["int"] += r["int"] == "1"
                        tg["pass_td"] += r["pass_td"] == "1"
                if rush:
                    yds = f(r["yards_gained"]) or 0
                    tg["carries"] += 1
                    tg["rush_yards"] += yds
                    tg["rush_td"] += r["rush_td"] == "1"
                    tg["explosive_run"] += yds >= 10
                    if hd == "fbs" and r["rusher_player_name"]:
                        leaders[(y, "rush_yards")][r["rusher_player_name"] + r["pos_team"]] += yds
                if rush or pas:
                    tg["plays"] += 1
                    tg["success"] += r["success"] == "1"
                    if down == 3:
                        tg["third"] += 1
                        tg["third_conv"] += (f(r["yards_gained"]) or 0) >= (f(r["distance"]) or 99)
                    if down == 4:
                        tg["fourth_go"] += 1
                        tg["fourth_conv"] += (f(r["yards_gained"]) or 0) >= (f(r["distance"]) or 99)
                if r["fg_inds"] == "1" and r["yds_fg"]:
                    d = f(r["yds_fg"])
                    band = "under30" if d < 30 else "30-39" if d < 40 else "40-49" if d < 50 else "50+"
                    a["fg"][band][0] += 1
                    a["fg"][band][1] += r["fg_made"] in ("1", "TRUE")
                if r["punt"] == "1" and r["yds_punted"]:
                    a["c"]["punts"] += 1
                    a["c"]["punt_gross"] += f(r["yds_punted"])
    for d, a in acc.items():
        n = len(a["tg"])
        tot = defaultdict(float)
        for tg in a["tg"].values():
            for k, v in tg.items():
                tot[k] += v
        att = max(tot["att"], 1)
        out["divisions"][d] = {
            "team_games": n,
            "per_team_game": {k: round(v / n, 3) for k, v in sorted(tot.items())},
            "rates": {
                "completion_pct": round(tot["cmp"] / att, 4),
                "sack_rate": round(tot["sacks"] / tot["dropbacks"], 4),
                "yards_per_attempt": round(tot["pass_yards"] / att, 3),
                "int_rate": round(tot["int"] / att, 4),
                "yards_per_carry": round(tot["rush_yards"] / tot["carries"], 3),
                "pass_rate": round(tot["dropbacks"] / tot["plays"], 4),
                "third_down_conversion": round(tot["third_conv"] / tot["third"], 4),
                "fourth_down_conversion": round(tot["fourth_conv"] / max(tot["fourth_go"], 1), 4),
                "explosive_run_rate": round(tot["explosive_run"] / tot["carries"], 4),
                "success_rate": round(tot["success"] / tot["plays"], 4),
                "punt_gross": round(a["c"]["punt_gross"] / max(a["c"]["punts"], 1), 2),
            },
            "field_goal_pct_by_distance": {b: {"attempts": v[0], "pct": round(v[1] / v[0], 4)} for b, v in sorted(a["fg"].items())},
        }
    out["fbs_season_leaders"] = {
        stat: sorted(round(max(leaders[(y, stat)].values()), 1) for y in years)
        for stat in ("pass_yards", "rush_yards", "rec_yards")
    }
    json.dump(out, sys.stdout, indent=1)

main()
