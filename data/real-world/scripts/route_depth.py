#!/usr/bin/env python3
"""How deep each route is thrown in the NFL: targets by route with air yards,
the four depth bands, catch rate, yards a target and time to throw, and the same by the
target's position.

nflverse-data releases pbp (play_by_play_YEAR.csv.gz) and pbp_participation
(pbp_participation_YEAR.csv), regular seasons 2023-2025: the three seasons
charted with one route list (NGS). Air yards are nflverse's; a target is a
pass attempt that is not a sack, with a route and air yards on record.

Usage: python3 route_depth.py OUTDIR -> OUTDIR/nfl_route_depth.json (data cached in OUTDIR/data)
"""
import collections
import csv
import gzip
import json
import os
import subprocess
import sys

OUT = sys.argv[1] if len(sys.argv) > 1 else "."
DATA = os.path.join(OUT, "data")
os.makedirs(DATA, exist_ok=True)
YEARS = [2023, 2024, 2025]
PBP = "https://github.com/nflverse/nflverse-data/releases/download/pbp/play_by_play_{y}.csv.gz"
PART = "https://github.com/nflverse/nflverse-data/releases/download/pbp_participation/pbp_participation_{y}.csv"


def fetch(url, local):
    path = os.path.join(DATA, local)
    if not os.path.exists(path) or os.path.getsize(path) < 1000:
        subprocess.run(["curl", "-sSL", "-o", path, url], check=True)
    return path


def band(a):
    return 0 if a < 0 else 1 if a < 10 else 2 if a < 20 else 3


def tally():
    return {"n": 0, "air": 0.0, "bands": [0, 0, 0, 0], "catch": 0, "yards": 0.0, "ttt": 0.0, "ttt_n": 0}


routes = collections.defaultdict(tally)
positions = collections.defaultdict(tally)
total = tally()
for y in YEARS:
    plays = {}
    with gzip.open(fetch(PBP.format(y=y), f"pbp_{y}.csv.gz"), "rt") as f:
        for r in csv.DictReader(f):
            if r["season_type"] != "REG" or r["pass_attempt"] != "1" or r["sack"] == "1" or r["air_yards"] == "":
                continue
            yards = float(r["passing_yards"] or 0)
            plays[(r["game_id"], r["play_id"])] = (r["complete_pass"] == "1", float(r["air_yards"]), r["receiver_player_id"], yards)
    with open(fetch(PART.format(y=y), f"part_{y}.csv")) as f:
        for r in csv.DictReader(f):
            p = plays.get((r["nflverse_game_id"], r["play_id"]))
            if p is None or not r["route"]:
                continue
            caught, air, rec, yards = p
            ids = r["offense_players"].split(";")
            pos = r["offense_positions"].split(";")
            where = pos[ids.index(rec)] if rec in ids and len(pos) == len(ids) else "?"
            where = where if where in ("WR", "TE", "RB") else "other"
            for t in (routes[r["route"]], positions[where], total):
                t["n"] += 1
                t["air"] += air
                t["bands"][band(air)] += 1
                t["catch"] += caught
                t["yards"] += yards
                if r["time_to_throw"]:
                    t["ttt"] += float(r["time_to_throw"])
                    t["ttt_n"] += 1


def out(t):
    n = max(t["n"], 1)
    return {
        "n": t["n"],
        "share": round(t["n"] / total["n"], 4),
        "air": round(t["air"] / n, 2),
        "behind": round(t["bands"][0] / n, 3),
        "0-9": round(t["bands"][1] / n, 3),
        "10-19": round(t["bands"][2] / n, 3),
        "20+": round(t["bands"][3] / n, 3),
        "catch": round(t["catch"] / n, 3),
        "ypt": round(t["yards"] / n, 2),
        "ttt": round(t["ttt"] / max(t["ttt_n"], 1), 2),
    }


res = {
    "source": "nflverse pbp and participation (NGS routes), regular seasons; targets with a route and air yards",
    "seasons": YEARS,
    "all": out(total),
    "by_route": {k: out(v) for k, v in sorted(routes.items(), key=lambda kv: -kv[1]["n"])},
    "by_position": {k: out(v) for k, v in sorted(positions.items(), key=lambda kv: -kv[1]["n"])},
}
with open(os.path.join(OUT, "nfl_route_depth.json"), "w") as f:
    json.dump(res, f, indent=1)
print(json.dumps(res["all"]), json.dumps(res["by_position"]))
