"""Builds data/real-world/hs_towns.csv: the real towns each state's generated
high schools are placed in (final build, lane 5).

A real county names its schools for its towns (Big Horn County, Wyoming has
Lovell, Greybull, Burlington and Basin), so every state gets as many real
towns as it has programs: its cities first, a city's share of schools
following its population (about one high school per 35,000 people, real
for big districts: Houston ISD has about 2.3 million people and some 40
high schools besides its charters), then its small towns.

Input (downloaded by hand, passed as the argument): the uszipcode
project's ZIP code database (2010 Census population, land area, median
household income, county and coordinates for every ZIP code, each tied to
its post office town),
https://github.com/MacHu-GWU/uszipcode-project/releases/download/1.0.1.db/simple_db.sqlite

Run from the repository root:
  python3 -I data/real-world/scripts/hs_towns.py <simple_db.sqlite>

A town's numbers add up its ZIP codes. Towns are taken biggest first until
the state's programs (`hs_states.csv`) have a place each, with a tenth to
spare, so hamlets without a high school stay out.

Columns: state, town, county, population, slots (how many of the state's
generated schools the town takes at most), density (people per square
mile), income (median household income, dollars), latitude, longitude.
"""

import csv
import math
import os
import sqlite3
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
DATA = os.path.join(ROOT, "data", "real-world")

PEOPLE_PER_SCHOOL = 35_000
MOST_IN_A_CITY = 40
# Room beyond the programs, so the world's seeds spread over more towns.
SPARE = 1.1


def main():
    db = sqlite3.connect(sys.argv[1])
    towns = {}
    q = "select state, major_city, county, population, land_area_in_sqmi, median_household_income, lat, lng from simple_zipcode where zipcode_type = 'STANDARD'"
    for st, city, county, pop, land, income, la, lo in db.execute(q):
        if not city or not st or not pop:
            continue
        t = towns.setdefault((st, city), {"county": county or "", "pop": 0, "land": 0.0, "inc": 0.0, "la": 0.0, "lo": 0.0})
        t["pop"] += pop
        t["land"] += land or 0.0
        t["inc"] += (income or 0) * pop
        t["la"] += (la or 0.0) * pop
        t["lo"] += (lo or 0.0) * pop
    programs = {}
    for r in csv.DictReader(open(os.path.join(DATA, "hs_states.csv"), encoding="utf-8")):
        programs[r["state"].strip()] = int(float(r["approx_programs"] or 0))
    rows = []
    for st, n in sorted(programs.items()):
        mine = sorted(((k[1], v) for k, v in towns.items() if k[0] == st), key=lambda x: (-x[1]["pop"], x[0]))
        need = math.ceil(n * SPARE)
        have = 0
        for town, v in mine:
            if have >= need:
                break
            pop = v["pop"]
            slots = max(1, min(MOST_IN_A_CITY, round(pop / PEOPLE_PER_SCHOOL)))
            county = v["county"].removesuffix(" County").removesuffix(" Parish").removesuffix(" Borough")
            density = pop / v["land"] if v["land"] > 0 else 0.0
            rows.append(
                [st, town, county, pop, slots, f"{density:.0f}", f"{v['inc'] / pop:.0f}", f"{v['la'] / pop:.4f}", f"{v['lo'] / pop:.4f}"]
            )
            have += slots
    out = os.path.join(DATA, "hs_towns.csv")
    with open(out, "w", newline="", encoding="utf-8") as f:
        w = csv.writer(f)
        w.writerow(["state", "town", "county", "population", "slots", "density", "income", "lat", "lon"])
        w.writerows(rows)
    print(f"{len(rows)} towns written to {out}", file=sys.stderr)


if __name__ == "__main__":
    main()
