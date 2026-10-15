"""Real NFL numbers behind the engine physics checks (engine physics fixes).

Reads nflverse play-by-play (release "pbp", play_by_play_YEAR.csv.gz) and
participation (release "pbp_participation", pbp_participation_YEAR.csv) and
prints, as JSON:
- designed runs: yards by bucket, and yards per carry by defenders in the box;
- how quarterbacks differ: per QB season (300+ dropbacks), the spread of EPA
  per dropback, completion over expected, sack rate and interception rate;
- how running backs differ: per back season (120+ designed carries), the
  spread of yards per carry and success rate;
- pressure: pressure rate, sacks per pressure, time to throw.

Usage: python3 position_value.py DIR 2021 2022 2023 2024 2025 > position_value.json
Development checks only, never in-game tuning.
"""
import csv, gzip, json, statistics, sys
from collections import defaultdict


def f(x):
    try:
        return float(x)
    except (TypeError, ValueError):
        return None


def spread(vals):
    vals = sorted(vals)
    n = len(vals)
    q = lambda p: vals[min(n - 1, int(p * n))]
    return {"n": n, "mean": round(statistics.mean(vals), 4), "sd": round(statistics.pstdev(vals), 4),
            "p10": round(q(0.1), 4), "p50": round(q(0.5), 4), "p90": round(q(0.9), 4)}


def main():
    folder, years = sys.argv[1], sys.argv[2:]
    buckets = [(-99, 0.5, "<=0"), (0.5, 3.5, "1-3"), (3.5, 9.5, "4-9"), (9.5, 19.5, "10-19"), (19.5, 39.5, "20-39"),
               (39.5, 200, "40+")]
    runs = defaultdict(int)
    run_n = 0
    run_yds = 0.0
    box = defaultdict(lambda: [0, 0.0, 0])
    qbs = defaultdict(lambda: defaultdict(float))
    rbs = defaultdict(lambda: defaultdict(float))
    press = defaultdict(float)
    for y in years:
        part = {}
        try:
            with open(f"{folder}/part_{y}.csv", newline="") as fh:
                for r in csv.DictReader(fh):
                    part[(r["nflverse_game_id"], r["play_id"])] = r
        except FileNotFoundError:
            pass
        with gzip.open(f"{folder}/pbp_{y}.csv.gz", "rt", newline="") as fh:
            for r in csv.DictReader(fh):
                if r["season_type"] != "REG" or r["two_point_attempt"] == "1":
                    continue
                g = f(r["yards_gained"])
                if g is None:
                    continue
                p = part.get((r["game_id"], r["play_id"]))
                designed = r["rush_attempt"] == "1" and r["qb_scramble"] != "1" and r["qb_kneel"] != "1"
                if designed and r["play_type"] == "run":
                    run_n += 1
                    run_yds += g
                    for lo, hi, k in buckets:
                        if lo <= g < hi:
                            runs[k] += 1
                    if p and f(p["defenders_in_box"]) is not None:
                        b = int(f(p["defenders_in_box"]))
                        k = "<=6" if b <= 6 else ("7" if b == 7 else ">=8")
                        box[k][0] += 1
                        box[k][1] += g
                        box[k][2] += 1 if f(r["epa"]) and f(r["epa"]) > 0 else 0
                    rid = r["rusher_player_id"]
                    if rid:
                        s = rbs[(y, rid)]
                        s["n"] += 1
                        s["yds"] += g
                        s["succ"] += 1 if (f(r["epa"]) or 0) > 0 else 0
                if r["qb_dropback"] == "1" and r["passer_player_id"] or (r["qb_dropback"] == "1" and r["rusher_player_id"]):
                    qid = r["passer_player_id"] or r["rusher_player_id"]
                    s = qbs[(y, qid)]
                    s["db"] += 1
                    s["epa"] += f(r["qb_epa"]) or 0.0
                    s["sack"] += 1 if r["sack"] == "1" else 0
                    if r["pass_attempt"] == "1" and r["sack"] != "1":
                        s["att"] += 1
                        s["int"] += 1 if r["interception"] == "1" else 0
                        if f(r["cpoe"]) is not None:
                            s["cpoe"] += f(r["cpoe"])
                            s["cpoe_n"] += 1
                    if p and p["was_pressure"] in ("TRUE", "FALSE", "1", "0", "True", "False"):
                        pr = p["was_pressure"] in ("TRUE", "1", "True")
                        press["db"] += 1
                        press["pressured"] += pr
                        press["sacks"] += 1 if r["sack"] == "1" else 0
                        press["sacks_pressured"] += 1 if (pr and r["sack"] == "1") else 0
                        if f(p["time_to_throw"]) is not None:
                            press["ttt"] += f(p["time_to_throw"])
                            press["ttt_n"] += 1
    # Backs: anyone with 120+ designed carries who isn't a dropback passer.
    passers = {k[1] for k, v in qbs.items() if v["db"] >= 50}
    rb = [v for k, v in rbs.items() if v["n"] >= 120 and k[1] not in passers]
    qb = [v for v in qbs.values() if v["db"] >= 300]
    out = {
        "source": "nflverse play-by-play and participation, regular seasons " + ", ".join(years),
        "designed_runs": {
            "n": run_n,
            "yards_per_carry": round(run_yds / run_n, 3),
            "share": {k: round(runs[k] / run_n, 4) for _, _, k in buckets},
            "by_box": {k: {"n": v[0], "ypc": round(v[1] / v[0], 3), "success": round(v[2] / v[0], 4)}
                       for k, v in sorted(box.items())},
        },
        "qb_seasons_300_dropbacks": {
            "epa_per_dropback": spread([v["epa"] / v["db"] for v in qb]),
            "cpoe": spread([v["cpoe"] / v["cpoe_n"] for v in qb if v["cpoe_n"]]),
            "sack_rate": spread([v["sack"] / v["db"] for v in qb]),
            "int_rate": spread([v["int"] / v["att"] for v in qb if v["att"]]),
        },
        "rb_seasons_120_carries": {
            "yards_per_carry": spread([v["yds"] / v["n"] for v in rb]),
            "success_rate": spread([v["succ"] / v["n"] for v in rb]),
        },
        "pressure": {
            "dropbacks_charted": int(press["db"]),
            "pressure_rate": round(press["pressured"] / max(1, press["db"]), 4),
            "sack_rate": round(press["sacks"] / max(1, press["db"]), 4),
            "sacks_per_pressure": round(press["sacks_pressured"] / max(1, press["pressured"]), 4),
            "time_to_throw": round(press["ttt"] / max(1, press["ttt_n"]), 3),
        },
    }
    json.dump(out, sys.stdout, indent=1)
    print()


if __name__ == "__main__":
    main()
