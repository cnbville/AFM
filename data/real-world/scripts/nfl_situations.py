"""Real NFL situational tendencies for the match engine (section 3, topic 3.2).

Reads nflverse play-by-play (release "pbp", play_by_play_YEAR.csv.gz) and
writes how real offenses call plays by situation, how often they go for it on
fourth down, a win-probability table (nflfastR's own model, averaged), and
pass depth results used by the development checks.

The coordinator AI starts from these real tendencies and bends them by its
own scheme, aggressiveness and what is working in the game.

Usage: python3 nfl_situations.py DIR 2021 2022 2023 2024 2025 > nfl_situations.json
"""
import csv, gzip, json, sys
from collections import defaultdict

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

def main():
    folder, years = sys.argv[1], sys.argv[2:]
    pr = defaultdict(lambda: [0, 0])
    go = defaultdict(lambda: [0, 0])
    wp = defaultdict(lambda: [0.0, 0])
    depth = defaultdict(lambda: [0, 0, 0.0, 0.0])
    gaps = defaultdict(int)
    runoff = defaultdict(lambda: [0.0, 0])
    prev = None
    for y in years:
        with gzip.open(f"{folder}/pbp_{y}.csv.gz", "rt", newline="") as fh:
            for r in csv.DictReader(fh):
                if r["season_type"] != "REG":
                    continue
                pt = r["play_type"]
                down, togo = f(r["down"]), f(r["ydstogo"])
                sd, hs, q = f(r["score_differential"]), f(r["half_seconds_remaining"]), f(r["qtr"])
                gs, yl = f(r["game_seconds_remaining"]), f(r["yardline_100"])
                # Seconds between snaps, by whether the clock was running.
                if prev and prev[0] == r["game_id"] and gs is not None and prev[1] is not None and q == prev[3]:
                    gap = prev[1] - gs
                    if 0 <= gap <= 60:
                        key = prev[2]
                        runoff[key][0] += gap
                        runoff[key][1] += 1
                if pt in ("run", "pass") and r["qb_kneel"] != "1" and r["qb_spike"] != "1":
                    stopped = r["incomplete_pass"] == "1" or r["out_of_bounds"] == "1" or r["touchdown"] == "1" or r["penalty"] == "1" or r["timeout"] == "1"
                    tb = time_b(hs or 0, q or 1)
                    prev = (r["game_id"], gs, ("stopped" if stopped else "running") + "_" + tb, q)
                else:
                    prev = None
                if pt not in ("run", "pass", "punt", "field_goal") or down is None or togo is None or sd is None:
                    continue
                if r["qb_kneel"] == "1" or r["qb_spike"] == "1":
                    continue
                if pt in ("run", "pass") and down <= 3:
                    k = f"{int(down)}|{togo_b(togo)}|{score_b(sd)}|{time_b(hs or 0, q or 1)}"
                    pr[k][0] += pt == "pass"
                    pr[k][1] += 1
                if down == 4 and yl is not None:
                    yb = min(9, int(yl) // 10) * 10
                    k = f"{togo_b(togo)}|{yb}|{score_b(sd)}"
                    go[k][0] += pt in ("run", "pass")
                    go[k][1] += 1
                if r["wp"] and gs is not None and yl is not None:
                    k = f"{max(-28, min(28, int(sd // 4 * 4)))}|{int(gs // 300) * 300}|{min(3, int(yl) // 25)}"
                    wp[k][0] += f(r["wp"])
                    wp[k][1] += 1
                if pt == "pass" and r["pass_attempt"] == "1" and r["sack"] != "1" and r["air_yards"]:
                    a = f(r["air_yards"])
                    b = -5 if a < 0 else 0 if a < 10 else 10 if a < 20 else 20
                    d = depth[b]
                    d[0] += 1
                    d[1] += r["complete_pass"] == "1"
                    d[2] += f(r["yards_after_catch"]) or 0
                    d[3] += r["interception"] == "1"
                if pt == "run" and r["run_location"]:
                    gaps[r["run_location"] + "|" + (r["run_gap"] or "middle")] += 1
    out = {
        "source": "nflverse play-by-play, regular seasons " + ", ".join(years),
        "note": "pass_rate key: down|togo bucket (1,3,6,10,15)|score diff bucket (-15,-8,-1,0,7,14,15)|time (normal, late = 4th qtr last 7 min, two_minute = last 2 min of a half). fourth_go key: togo bucket|yardline_100 bucket (10s)|score bucket. win_prob key: score diff (4-pt buckets)|game seconds left (5-min buckets)|yardline_100 quarter.",
        "pass_rate": {k: {"rate": round(v[0] / v[1], 4), "n": v[1]} for k, v in sorted(pr.items()) if v[1] >= 30},
        "fourth_go": {k: {"rate": round(v[0] / v[1], 4), "n": v[1]} for k, v in sorted(go.items()) if v[1] >= 15},
        "win_prob": {k: {"wp": round(v[0] / v[1], 4), "n": v[1]} for k, v in sorted(wp.items()) if v[1] >= 30},
        "pass_by_air_yards": {str(k): {"att": v[0], "comp_pct": round(v[1] / v[0], 4), "yac_per_att": round(v[2] / v[0], 2), "int_rate": round(v[3] / v[0], 4)} for k, v in sorted(depth.items())},
        "run_direction": dict(sorted(gaps.items())),
        "seconds_between_snaps": {k: round(v[0] / v[1], 1) for k, v in sorted(runoff.items()) if v[1] >= 30},
    }
    json.dump(out, sys.stdout, indent=1)

main()
