"""How often real NFL offenses fake the run before a throw (scheme-fit item 3).

Reads nflverse play-by-play (release "pbp", play_by_play_YEAR.csv.gz) and
FTN charting (release "ftn_charting", ftn_charting_YEAR.csv) and writes the
play-action share of dropbacks as a logistic model with three parts, the
down and distance, the score and clock, and the field:

    rate = 1 / (1 + exp(-(down_togo[down, togo] + score_time[score, time] + field[zone])))

fitted by maximum likelihood (each bucket's predicted play action matches
its real count), so extreme cells stay below one. A dropback is a qb_dropback play
that FTN charted, screens in (a screen can come off a fake), RPOs out (they
are their own call). The buckets are the engine's: downs 1, 2 and 3 (third
and fourth), yards to go 1, 3, 6, 10, 15 and more, the score as the pass
rate table has it, the clock (two_minute: the last two minutes of a half;
late: the fourth quarter's last seven minutes), and the field (yards from
the end zone: 5, 10, 20 and more).

Usage: python3 play_action_rate.py DIR 2022 2023 2024 2025 > nfl_play_action_rate.csv
"""
import csv, gzip, sys
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


def field_b(yl):
    return 5 if yl <= 5 else 10 if yl <= 10 else 20 if yl <= 20 else 99


def main():
    folder, years = sys.argv[1], sys.argv[2:]
    cells = defaultdict(lambda: [0, 0])  # (dt, st, fz) -> [dropbacks, play action]
    for y in years:
        ftn = {}
        with open(f"{folder}/ftn_charting_{y}.csv", newline="") as fh:
            for r in csv.DictReader(fh):
                ftn[(r["nflverse_game_id"], str(int(float(r["nflverse_play_id"]))))] = r
        with gzip.open(f"{folder}/play_by_play_{y}.csv.gz", "rt", newline="") as fh:
            for r in csv.DictReader(fh):
                if r["season_type"] != "REG" or r["qb_dropback"] != "1":
                    continue
                pid = f(r["play_id"])
                fr = ftn.get((r["game_id"], str(int(pid)))) if pid is not None else None
                if fr is None or fr["is_rpo"] in ("TRUE", "True", "1", "true"):
                    continue
                down, togo, yl = f(r["down"]), f(r["ydstogo"]), f(r["yardline_100"])
                sd, hs, q = f(r["score_differential"]), f(r["half_seconds_remaining"]), f(r["qtr"])
                if None in (down, togo, yl, sd, hs, q):
                    continue
                pa = fr["is_play_action"] in ("TRUE", "True", "1", "true")
                key = ((min(int(down), 3), togo_b(togo)), (score_b(int(sd)), time_b(hs, q)), field_b(yl))
                cells[key][0] += 1
                cells[key][1] += pa
    # Maximum likelihood, one bucket at a time: Newton steps on each
    # bucket's log odds until its predicted play action matches the count.
    import math

    sig = lambda x: 1.0 / (1.0 + math.exp(-x))
    lo = [defaultdict(float) for _ in range(3)]
    for _ in range(100):
        for k in range(3):
            groups = defaultdict(list)
            for key, (n, p) in cells.items():
                rest = sum(lo[j][key[j]] for j in range(3) if j != k)
                groups[key[k]].append((n, p, rest))
            for b, rows in groups.items():
                x = lo[k][b]
                for _ in range(20):
                    g = sum(p - n * sig(r + x) for n, p, r in rows)
                    h = sum(n * sig(r + x) * (1.0 - sig(r + x)) for n, p, r in rows)
                    if h <= 1e-9:
                        break
                    step = max(-2.0, min(2.0, g / h))
                    x += step
                    if abs(step) < 1e-10:
                        break
                lo[k][b] = max(-12.0, min(12.0, x))
    # A close game on the open field (within a score, normal clock; 21+
    # yards out) reads 0 on score and field, so the down-and-distance part
    # alone is the rate there.
    ref_s = lo[1][(0, "normal")]
    ref_f = lo[2][99]
    totals = [defaultdict(lambda: [0, 0]) for _ in range(3)]
    for key, (n, p) in cells.items():
        for j in range(3):
            totals[j][key[j]][0] += n
            totals[j][key[j]][1] += p
    w = csv.writer(sys.stdout, lineterminator="\n")
    w.writerow(["factor", "a", "b", "logit", "n", "real_rate"])
    for (d, t) in sorted(lo[0]):
        n, p = totals[0][(d, t)]
        w.writerow(["down_togo", d, t, round(lo[0][(d, t)] + ref_s + ref_f, 4), n, round(p / n, 4)])
    for (s, tm) in sorted(lo[1], key=lambda x: (x[1], x[0])):
        n, p = totals[1][(s, tm)]
        w.writerow(["score_time", s, tm, round(lo[1][(s, tm)] - ref_s, 4), n, round(p / n, 4)])
    for z in sorted(lo[2]):
        n, p = totals[2][z]
        w.writerow(["field", z, "", round(lo[2][z] - ref_f, 4), n, round(p / n, 4)])
    n = sum(c[0] for c in cells.values())
    p = sum(c[1] for c in cells.values())
    print(f"# {n} dropbacks, play action {p / n:.4f}", file=sys.stderr)


if __name__ == "__main__":
    main()
