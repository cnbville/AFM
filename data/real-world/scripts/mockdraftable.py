"""Arm length, hand size, wingspan and the 10- and 20-yard splits per player, from MockDraftable.

The nflverse combine file has no arms, hands or splits. MockDraftable
(https://www.mockdraftable.com) shows them per player, from the combine and
pro days; its site reads them from a JSON API (/api/search and
/api/multiple-players). This script reads that API once, politely (one call
every 0.3 s), and caches every player of the draft classes asked for in
DIR/mockdraftable.json. ab_real.py links the players to nflverse ids and fits
the arm and split slopes; only those fitted numbers go into the repo, never the
per-player table.

Usage: python3 mockdraftable.py [DIR] [FIRST] [LAST]
  DIR defaults to /tmp/claude-0/abt/nflverse; classes FIRST-LAST to 2005-2025.
"""
import json
import os
import subprocess
import sys
import time
import urllib.parse

D = sys.argv[1] if len(sys.argv) > 1 else "/tmp/claude-0/abt/nflverse"
FIRST = int(sys.argv[2]) if len(sys.argv) > 2 else 2005
LAST = int(sys.argv[3]) if len(sys.argv) > 3 else 2025
SITE = "https://www.mockdraftable.com/api"
# MockDraftable's measurable keys.
KEYS = {1: "ht", 2: "wt", 3: "wingspan", 4: "arm", 5: "hand", 6: "split10", 7: "split20", 8: "forty"}


def get(path, **q):
    url = f"{SITE}/{path}?" + urllib.parse.urlencode(q)
    for wait in (0, 2, 4, 8, 16):
        time.sleep(0.3 + wait)
        r = subprocess.run(["curl", "-sS", "--max-time", "60", url], capture_output=True, text=True)
        if r.returncode == 0 and r.stdout[:1] in "[{":
            return json.loads(r.stdout)
    raise RuntimeError(f"MockDraftable did not answer: {url}")


def ids():
    out, page = [], 1
    while True:
        opts = {"beginYear": FIRST, "endYear": LAST, "sortOrder": "DESC", "page": page}
        r = get("search", opts=json.dumps(opts), pos="ATH")
        out += r["players"]
        if not r["hasNextPage"]:
            return out
        page += 1
        if page % 50 == 0:
            print(f"  search page {page}: {len(out)} players", file=sys.stderr)


def main():
    path = os.path.join(D, "mockdraftable.json")
    if os.path.exists(path):
        print(f"{path} is there already", file=sys.stderr)
        return
    os.makedirs(D, exist_ok=True)
    every = list(dict.fromkeys(ids()))
    print(f"{len(every)} players in the {FIRST}-{LAST} classes", file=sys.stderr)
    rows = []
    for i in range(0, len(every), 40):
        for p in get("multiple-players", ids=json.dumps(every[i:i + 40])):
            row = {"id": p["id"], "name": p["name"], "draft": p["draft"], "school": p.get("school"),
                   "pos": p["positions"]["primary"], "groups": p["positions"]["all"]}
            for m in p["measurements"]:
                k = KEYS.get(m["measurableKey"])
                # source 1 is the NFL combine; pro day numbers (other sources) only fill what the combine lacks.
                if k and (k not in row or m["source"] == 1):
                    row[k] = m["measurement"]
                    row[k + "_src"] = m["source"]
            rows.append(row)
        if i % 2000 == 0:
            print(f"  {len(rows)} players read", file=sys.stderr)
    with open(path, "w") as f:
        json.dump({"source": "https://www.mockdraftable.com (site API), read " + time.strftime("%Y-%m-%d"),
                   "classes": [FIRST, LAST], "players": rows}, f)
    print(f"wrote {path}: {len(rows)} players", file=sys.stderr)


if __name__ == "__main__":
    main()
