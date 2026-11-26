"""Real blitzes: how often off-ball linebackers, corners and safeties rush the passer and what it
pays (realism audit item 3's second lane, linebackers and backs behind the line).

The NFL's regular seasons 2018-2024 from Pro Football Reference's advanced defense, one row a
player season with his blitzes, pressures (hurries, knockdowns and sacks) and sacks, by PFR's
position label. PFR counts a blitz when a man who isn't a down lineman rushes, so a 3-4 outside
linebacker's blitzes are only part of his rushes: his pressures a blitz mean nothing, and he is
kept apart. The generic label "LB" holds off-ball men and edges alike, so a man with it whose
pressures are more than half his blitzes (5 or more pressures) is read as an edge by use.

Reads advstats_season_def.csv (nflverse-data release pfr_advstats).
No public source charts college blitzes by player, so there is no FBS block.

Usage: python3 blitz.py advstats_season_def.csv > blitz.json
Development checks only, never in-game tuning.
"""
import csv, json, sys

FIRST, LAST = 2018, 2024
rows = [r for r in csv.DictReader(open(sys.argv[1], newline="", encoding="utf-8")) if FIRST <= int(r["season"]) <= LAST]
TG = sum(32 * (17 if s >= 2021 else 16) for s in range(FIRST, LAST + 1))


def f(x):
    try:
        return float(x)
    except (TypeError, ValueError):
        return 0.0


def edge_by_use(r):
    return f(r["prss"]) >= 5 and f(r["prss"]) > 0.5 * max(f(r["bltz"]), 1.0)


OFF_BALL = ("LILB", "RILB", "MLB", "LLB", "RLB", "ILB")
OLB = ("LOLB", "ROLB", "OLB")
SETS = {
    "linebackers": lambda r: r["pos"].upper() in OFF_BALL or (r["pos"].upper() == "LB" and not edge_by_use(r)),
    "corners": lambda r: r["pos"].upper() in ("LCB", "RCB", "CB"),
    "safeties": lambda r: r["pos"].upper() in ("FS", "SS", "S"),
    "backs": lambda r: r["pos"].upper() in ("LCB", "RCB", "CB", "FS", "SS", "S", "DB"),
    "edges_listed_linebacker": lambda r: r["pos"].upper() in OLB or (r["pos"].upper() == "LB" and edge_by_use(r)),
    "ends": lambda r: r["pos"].upper() in ("LDE", "RDE", "DE"),
    "tackles": lambda r: r["pos"].upper() in ("DT", "NT", "LDT", "RDT", "DL"),
}


def pct(v, q):
    v = sorted(v)
    return v[min(len(v) - 1, int(q * len(v)))] if v else 0.0


out = {
    "source": "Pro Football Reference advanced defense through nflverse-data (release pfr_advstats), regular seasons",
    "seasons": [FIRST, LAST],
    "team_games": TG,
    "note": "A blitz is a rush by a man who is not a down lineman. Pressures are hurries, knockdowns and sacks. "
    "Sacks a blitz reads every sack he had against his blitzes. Edges listed as linebackers rush far more often "
    "than PFR counts blitzes for them, so only their sacks a team game are kept.",
    "groups": {},
}
for name, test in SETS.items():
    v = [r for r in rows if test(r)]
    b = sum(f(r["bltz"]) for r in v)
    p = sum(f(r["prss"]) for r in v)
    s = sum(f(r["sk"]) for r in v)
    g = {"player_seasons": len(v), "sacks_a_team_game": round(s / TG, 3)}
    if name in ("linebackers", "corners", "safeties", "backs"):
        g.update(
            {
                "blitzes_a_team_game": round(b / TG, 2),
                "pressures_a_team_game": round(p / TG, 3),
                "pressures_a_blitz": round(p / max(b, 1), 3),
                "sacks_a_blitz": round(s / max(b, 1), 4),
                "sacks_a_pressure": round(s / max(p, 1), 3),
            }
        )
        st = [r for r in v if f(r["gs"]) >= 8]
        per = [f(r["bltz"]) / max(f(r["g"]), 1) for r in st]
        g["starters"] = {
            "player_seasons": len(st),
            "blitzes_a_game_p10_p50_p90": [round(pct(per, q), 2) for q in (0.1, 0.5, 0.9)],
            "sacks_a_season": round(sum(f(r["sk"]) for r in st) / max(len(st), 1), 2),
            "pressures_a_season": round(sum(f(r["prss"]) for r in st) / max(len(st), 1), 2),
        }
    else:
        g["pressures_a_team_game"] = round(p / TG, 3)
        g["sacks_a_pressure"] = round(s / max(p, 1), 3)
    out["groups"][name] = g
out["sacks_a_team_game_all_rows"] = round(sum(f(r["sk"]) for r in rows) / TG, 3)
json.dump(out, sys.stdout, indent=1)
print()
