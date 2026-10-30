"""Real college expected points (update 3, F30: each level reads its own
expected-points table).

Reads cfbfastR play-by-play (sportsdataverse-data release "cfbfastR_cfb_pbp",
files play_by_play_YEAR.csv saved as cfb_YEAR.csv) and writes, for regular
season FBS-vs-FBS games, the mean `ep_before` (cfbfastR's expected points
model) by down, distance and field position, in the same buckets, play filter
and CSV layout as nfl_expected_points.csv (built from nflverse `ep` by
nfl_game_targets.py): runs, passes, punts and field goals.

Usage: python3 college_expected_points.py DIR OUT_DIR 2023 2024
"""
import csv, sys
from collections import defaultdict

csv.field_size_limit(10**8)

RUNS = {"Rush", "Rushing Touchdown"}
PASSES = {"Pass Reception", "Pass Incompletion", "Passing Touchdown", "Sack", "Interception Return",
          "Interception Return Touchdown", "Pass Completion", "Pass Interception Return"}
KICKS = {"Punt", "Field Goal Good", "Field Goal Missed", "Blocked Field Goal", "Blocked Punt"}


def f(x):
    try:
        return float(x)
    except (TypeError, ValueError):
        return None


def togo_b(t):
    for b in (1, 2, 3, 6, 10, 15):
        if t <= b:
            return b
    return 20


def main():
    folder, out_dir, years = sys.argv[1], sys.argv[2], sys.argv[3:]
    ep = defaultdict(lambda: [0.0, 0])
    for y in years:
        with open(f"{folder}/cfb_{y}.csv", newline="") as fh:
            for r in csv.DictReader(fh):
                if r["season_type"] != "regular":
                    continue
                if r["home_team_division"] != "fbs" or r["away_team_division"] != "fbs":
                    continue
                pt = r["play_type"]
                if pt not in RUNS and pt not in PASSES and pt not in KICKS:
                    continue
                down, togo, yl, e = f(r["down"]), f(r["distance"]), f(r["yards_to_goal"]), f(r["ep_before"])
                if down is None or togo is None or yl is None or e is None or not 1 <= down <= 4 or togo < 0:
                    continue
                k = (int(down), togo_b(togo), min(99, max(1, int(yl))) // 5 * 5)
                ep[k][0] += e
                ep[k][1] += 1
    with open(f"{out_dir}/college_expected_points.csv", "w", newline="") as fh:
        w = csv.writer(fh, lineterminator="\n")
        w.writerow(["down", "togo", "yardline_100", "ep", "n"])
        # Cells under 20 plays are left to the reader's fallback, as in the
        # NFL table.
        for k in sorted(ep):
            s, n = ep[k]
            if n >= 20:
                w.writerow([*k, f"{s / n:.3f}", n])


if __name__ == "__main__":
    main()
