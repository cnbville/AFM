#!/usr/bin/env python3
"""Rosters from a world trial's raw export, for `match_check` (AFM_ROSTERS).

Usage: export_rosters.py RAW_DIR OUT.json [DIVISION]

Reads programs.jsonl and people.jsonl from a world trial's raw export
(AFM_RAW, unzipped) and writes every program of the division (default FBS)
with the college players on it at the end of the run: position, the 67
attributes, the 15 tendencies and the personality. DIVISION NFL reads
pros.jsonl instead: every club's players on its roster (exports from
2026-10-19 on carry their attributes). The engine fills in what the export
leaves out (body, hidden traits) from the position. Trial tooling only.
"""
import json
import sys


def main():
    raw, out = sys.argv[1], sys.argv[2]
    div = sys.argv[3] if len(sys.argv) > 3 else "FBS"
    if div == "NFL":
        return nfl(raw, out)
    progs = {}
    with open(f"{raw}/programs.jsonl") as f:
        for line in f:
            p = json.loads(line)
            if p.get("div") == div:
                progs[p["id"]] = {"name": p.get("name", str(p["id"])), "conf": p.get("conf", ""), "players": []}
    with open(f"{raw}/people.jsonl") as f:
        for line in f:
            d = json.loads(line)
            if d.get("stage") != "College" or d.get("college") not in progs or not d.get("detail"):
                continue
            det = d["detail"]
            progs[d["college"]]["players"].append({
                "pos": d["pos"],
                "attrs": {a[0]: a[1] for a in det["attrs"]},
                "tend": {t[0]: t[1] for t in det.get("tend", [])},
                "personality": det.get("personality", {}),
            })
    teams = [t for t in progs.values() if len(t["players"]) >= 40]
    json.dump({"division": div, "teams": teams}, open(out, "w"), separators=(",", ":"))
    n = sum(len(t["players"]) for t in teams)
    print(f"{len(teams)} {div} teams, {n} players -> {out}")


def row(pos, det):
    return {
        "pos": pos,
        "attrs": {a[0]: a[1] for a in det["attrs"]},
        "tend": {t[0]: t[1] for t in det.get("tend", [])},
        "personality": det.get("personality", {}),
    }


def nfl(raw, out):
    clubs = {}
    with open(f"{raw}/pros.jsonl") as f:
        for line in f:
            d = json.loads(line)
            if not d.get("active") or d.get("status") != "Roster" or not d.get("team") or not d.get("detail"):
                continue
            clubs.setdefault(d["team"], {"name": d["team"], "conf": "NFL", "players": []})["players"].append(row(d["pos"], d["detail"]))
    teams = [t for t in clubs.values() if len(t["players"]) >= 40]
    json.dump({"division": "NFL", "teams": teams}, open(out, "w"), separators=(",", ":"))
    n = sum(len(t["players"]) for t in teams)
    print(f"{len(teams)} NFL teams, {n} players -> {out}")


if __name__ == "__main__":
    main()
