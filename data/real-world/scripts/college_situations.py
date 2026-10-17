"""Real college play calling by situation (trial 4 topic 5: college calling
from real college play-by-play).

Reads cfbfastR play-by-play (sportsdataverse-data release "cfbfastR_cfb_pbp",
files play_by_play_YEAR.csv saved as cfb_YEAR.csv) and writes, for regular
season FBS-vs-FBS games, the pass rate by down, distance, score and clock and
the fourth-down go rate by distance, field position and score, in the same
buckets and CSV layout as the NFL tables (nfl_pass_rate.csv, nfl_fourth_go.csv)
built by nfl_situations.py. Sacks count as passes; kneels and spikes are left
out.

Usage: python3 college_situations.py DIR OUT_DIR 2023 2024
"""
import csv, sys
from collections import defaultdict

csv.field_size_limit(10**8)


def f(x):
    try:
        return float(x)
    except (TypeError, ValueError):
        return None


def togo_b(t):
    return 1 if t <= 1 else 3 if t <= 3 else 6 if t <= 6 else 10 if t <= 10 else 15


def score_b(s):
    for b in (-15, -8, -1):
        if s <= b:
            return b
    if s == 0:
        return 0
    for b in (7, 14):
        if s <= b:
            return b
    return 15


def time_b(half_sec, qtr):
    if half_sec <= 120:
        return "two_minute"
    if qtr == 4 and half_sec <= 420:
        return "late"
    return "normal"


RUNS = {"Rush", "Rushing Touchdown"}
PASSES = {"Pass Reception", "Pass Incompletion", "Passing Touchdown", "Sack", "Interception Return",
          "Interception Return Touchdown", "Pass Completion", "Pass Interception Return"}
KICKS = {"Punt", "Field Goal Good", "Field Goal Missed", "Blocked Field Goal", "Blocked Punt"}


def main():
    folder, out_dir, years = sys.argv[1], sys.argv[2], sys.argv[3:]
    pr = defaultdict(lambda: [0, 0])
    go = defaultdict(lambda: [0, 0])
    for y in years:
        with open(f"{folder}/cfb_{y}.csv", newline="") as fh:
            for r in csv.DictReader(fh):
                if r["season_type"] != "regular":
                    continue
                if r["home_team_division"] != "fbs" or r["away_team_division"] != "fbs":
                    continue
                pt = r["play_type"]
                text = r["play_text"].lower()
                if "kneel" in text or "spike" in text:
                    continue
                down, togo = f(r["down"]), f(r["distance"])
                sd, hs, q, yl = f(r["pos_score_diff_start"]), f(r["TimeSecsRem"]), f(r["period"]), f(r["yards_to_goal"])
                if down is None or togo is None or sd is None or hs is None or q is None:
                    continue
                run, passing = pt in RUNS, pt in PASSES
                if (run or passing) and 1 <= down <= 3:
                    k = (int(down), togo_b(togo), score_b(sd), time_b(hs, q))
                    pr[k][0] += passing
                    pr[k][1] += 1
                if down == 4 and yl is not None and (run or passing or pt in KICKS):
                    k = (togo_b(togo), min(9, int(yl) // 10) * 10, score_b(sd))
                    go[k][0] += run or passing
                    go[k][1] += 1
    with open(f"{out_dir}/college_pass_rate.csv", "w", newline="") as fh:
        w = csv.writer(fh, lineterminator="\n")
        w.writerow(["down", "togo", "score", "time", "rate", "n"])
        for k, v in sorted(pr.items(), key=lambda kv: (kv[0][0], kv[0][1], kv[0][2], kv[0][3])):
            if v[1] >= 30:
                w.writerow([*k, round(v[0] / v[1], 4), v[1]])
    with open(f"{out_dir}/college_fourth_go.csv", "w", newline="") as fh:
        w = csv.writer(fh, lineterminator="\n")
        w.writerow(["togo", "yardline_100", "score", "rate", "n"])
        for k, v in sorted(go.items()):
            if v[1] >= 15:
                w.writerow([*k, round(v[0] / v[1], 4), v[1]])


main()
