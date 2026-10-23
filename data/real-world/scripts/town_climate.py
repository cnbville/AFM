"""Builds data/real-world/town_climate.csv: NOAA 1991-2020 monthly climate
normals for every town the world plays football in (U-e, final build).

Inputs (downloaded by hand, passed as arguments):
  1. the NOAA archive us-climate-normals_1991-2020_v1.0.1_monthly_multivariate_
     by-station_c20230404.tar.gz, extracted to a folder (one CSV per station),
     from https://noaa-normals-pds.s3.amazonaws.com/normals-monthly/1991-2020/archive/
  2. a US city list with coordinates (kelvins/US-Cities-Database us_cities.csv,
     https://raw.githubusercontent.com/kelvins/US-Cities-Database/main/csv/us_cities.csv)

Run from the repository root:
  python3 -I data/real-world/scripts/town_climate.py <normals dir> <us_cities.csv>

Each town takes the nearest station with complete monthly normals for the mean
high (MLY-TMAX-NORMAL), mean low (MLY-TMIN-NORMAL) and days with 0.01 inch of
precipitation or more (MLY-PRCP-AVGNDS-GE001HI). A first-order station (USW)
within 15 miles wins over a closer co-op station, since its record is the
longest. Towns the city list doesn't know keep their state's main town.
"""

import csv
import glob
import math
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
DATA = os.path.join(ROOT, "data", "real-world")
FRANCHISE = os.path.join(ROOT, "crates", "afm-engine", "src", "world", "franchise.rs")
CALDATA = os.path.join(ROOT, "crates", "afm-engine", "src", "calendar", "data.rs")

# Towns in the data that the city list spells differently or places by a
# neighbouring city (the stadium's or campus's own town is used).
ALIASES = {
    ("CO", "USAF Academy"): "Colorado Springs",
    ("NY", "West Point"): "Highland Falls",
    ("NY", "Bronx"): "New York",
    ("PA", "University Park"): "State College",
    ("NV", "Paradise"): "Las Vegas",
    ("MD", "Landover"): "Upper Marlboro",
    ("MA", "Foxborough"): "Foxboro",
    ("NJ", "East Rutherford"): "Rutherford",
    ("MA", "Chestnut Hill"): "Newton",
    ("CA", "Stanford"): "Palo Alto",
    ("NC", "Chapel Hill"): "Chapel Hill",
    ("SC", "Clemson"): "Clemson",
    ("DC", "Washington"): "Washington",
    ("OH", "Oxford"): "Oxford",
    ("IN", "Notre Dame"): "South Bend",
    ("VA", "Charlottesville"): "Charlottesville",
    ("NY", "Orchard Park"): "Orchard Park",
    ("LA", "Thibodaux"): "Thibodaux",
    ("TX", "College Station"): "College Station",
    ("AK", "Utqiagvik"): "Barrow",
    ("AL", "Vestavia Hills"): "Birmingham",
    ("CA", "Los Altos Hills"): "Los Altos",
    ("CO", "Centennial"): "Englewood",
    ("CO", "Greenwood Village"): "Englewood",
    ("CO", "Highlands Ranch"): "Littleton",
    ("CT", "Storrs"): "Mansfield Center",
    ("FL", "Ave Maria"): "Immokalee",
    ("FL", "Coral Gables"): "Miami",
    ("FL", "Miami Gardens"): "Opa Locka",
    ("FL", "Nassau"): "Yulee",
    ("IA", "Pleasant Hill"): "Des Moines",
    ("ID", "Coeur d'Alene"): "Coeur D Alene",
    ("IL", "East St. Louis"): "East Saint Louis",
    ("IL", "Urbana-Champaign"): "Champaign",
    ("IN", "Saint Mary-of-the-Woods"): "Terre Haute",
    ("KY", "Crestview Hills"): "Covington",
    ("ME", "Hermon"): "Bangor",
    ("MI", "Kentwood"): "Grand Rapids",
    ("MI", "Orchard Lake"): "Pontiac",
    ("MN", "Arden Hills"): "Saint Paul",
    ("MN", "Roseville"): "Saint Paul",
    ("MN", "Woodbury"): "Saint Paul",
    ("MS", "D'Iberville"): "Biloxi",
    ("NC", "Winston-Salem"): "Winston Salem",
    ("NJ", "Carneys Point"): "Penns Grove",
    ("NJ", "Union Township"): "Union",
    ("NJ", "Upper Montclair"): "Montclair",
    ("NY", "Brookville"): "Glen Cove",
    ("NY", "Kings Point"): "Great Neck",
    ("NY", "Williamsville"): "Buffalo",
    ("OH", "University Heights"): "Cleveland",
    ("PA", "Lower Oxford"): "Oxford",
    ("PA", "Moon Township"): "Coraopolis",
    ("PA", "St. Davids"): "Wayne",
    ("PA", "Wilkes-Barre"): "Wilkes Barre",
    ("TN", "McKenzie"): "Mc Kenzie",
    ("TN", "Tusculum"): "Greeneville",
    ("TX", "Brownsville/Edinburg"): "Edinburg",
    ("TX", "University Park"): "Dallas",
    ("VA", "Ettrick"): "Petersburg",
}


def miles(a, b):
    (la1, lo1), (la2, lo2) = a, b
    p1, p2 = math.radians(la1), math.radians(la2)
    dp, dl = p2 - p1, math.radians(lo2 - lo1)
    h = math.sin(dp / 2) ** 2 + math.cos(p1) * math.cos(p2) * math.sin(dl / 2) ** 2
    return 3959.0 * 2 * math.asin(math.sqrt(h))


def towns():
    """Every (state, town) the world places a team in."""
    out = set()
    for f in ("colleges.csv", "colleges_lower.csv"):
        for r in csv.DictReader(open(os.path.join(DATA, f), encoding="utf-8")):
            if r["city"].strip():
                out.add((r["state"].strip(), r["city"].strip()))
    for r in csv.DictReader(open(os.path.join(DATA, "talent_by_state.csv"), encoding="utf-8")):
        for t in r["top_hometowns"].split(";"):
            if t.strip():
                out.add((r["state"].strip(), t.strip()))
    for r in csv.DictReader(open(os.path.join(DATA, "hs_states.csv"), encoding="utf-8")):
        for p in r["top_5_powerhouses"].split(";"):
            m = re.search(r"\(([^)]+)\)", p)
            if m:
                out.add((r["state"].strip(), m.group(1).strip()))
    # The generated schools' real towns (`hs_towns.csv`, final build).
    for r in csv.DictReader(open(os.path.join(DATA, "hs_towns.csv"), encoding="utf-8")):
        out.add((r["state"].strip(), r["town"].strip()))
    for r in csv.DictReader(open(os.path.join(DATA, "nfl_teams.csv"), encoding="utf-8")):
        out.add((r["state"].strip(), r["city"].strip()))
    for m in re.finditer(r'e\(\s*\w+,\s*\w+,\s*"[^"]+",\s*"([^"]+)",\s*"([A-Z]{2})"', open(FRANCHISE).read()):
        out.add((m.group(2), m.group(1)))
    for m in re.finditer(r'b!\("[^"]+",\s*"([A-Z]{2})",\s*"([^"]+)"', open(CALDATA).read()):
        out.add((m.group(1), m.group(2)))
    return sorted(out)


def city_coords(path):
    c = {}
    # The school towns carry their own population-weighted centres.
    for r in csv.DictReader(open(os.path.join(DATA, "hs_towns.csv"), encoding="utf-8")):
        c[(r["state"], r["town"].lower())] = (float(r["lat"]), float(r["lon"]))
    for r in csv.DictReader(open(path, encoding="utf-8")):
        key = (r["STATE_CODE"], r["CITY"].lower())
        c.setdefault(key, (float(r["LATITUDE"]), float(r["LONGITUDE"])))
    return c


def station_normals(folder):
    """Stations with complete monthly highs, lows and wet days."""
    out = []
    for f in glob.glob(os.path.join(folder, "US*.csv")):
        rows = list(csv.DictReader(open(f, encoding="utf-8")))
        if len(rows) != 12:
            continue
        try:
            hi = [float(r["MLY-TMAX-NORMAL"]) for r in rows]
            lo = [float(r["MLY-TMIN-NORMAL"]) for r in rows]
            wet = [float(r["MLY-PRCP-AVGNDS-GE001HI"]) for r in rows]
        except (KeyError, ValueError):
            continue
        if min(hi) < -60 or min(lo) < -80 or min(wet) < 0:
            continue
        r0 = rows[0]
        out.append(
            {
                "id": r0["STATION"],
                "name": r0["NAME"].strip(),
                "at": (float(r0["LATITUDE"]), float(r0["LONGITUDE"])),
                "elev_ft": float(r0["ELEVATION"]) * 3.28084,
                "hi": hi,
                "lo": lo,
                "wet": wet,
            }
        )
    return out


def main():
    folder, cities_path = sys.argv[1], sys.argv[2]
    coords = city_coords(cities_path)
    stations = station_normals(folder)
    print(f"{len(stations)} stations with full normals", file=sys.stderr)
    rows = []
    missing = []
    for state, town in towns():
        name = ALIASES.get((state, town), town)
        at = coords.get((state, name.lower()))
        if at is None and name.startswith("St. "):
            at = coords.get((state, ("Saint " + name[4:]).lower()))
        if at is None:
            missing.append(f"{town}, {state}")
            continue
        near = [s for s in stations if abs(s["at"][0] - at[0]) < 1.5 and abs(s["at"][1] - at[1]) < 2.0] or stations
        ranked = sorted(near, key=lambda s: miles(at, s["at"]))
        best = ranked[0]
        for s in ranked[:25]:
            d = miles(at, s["at"])
            if d > 15:
                break
            if s["id"].startswith("USW"):
                best = s
                break
        d = miles(at, best["at"])
        rows.append(
            [state, town, best["id"], best["name"], f"{d:.1f}"]
            + [f"{x:.0f}" for x in best["hi"]]
            + [f"{x:.0f}" for x in best["lo"]]
            + [f"{x:.0f}" for x in best["wet"]]
        )
    out = os.path.join(DATA, "town_climate.csv")
    with open(out, "w", newline="", encoding="utf-8") as f:
        w = csv.writer(f)
        w.writerow(
            ["state", "town", "station", "station_name", "miles"]
            + [f"high_{m}" for m in range(1, 13)]
            + [f"low_{m}" for m in range(1, 13)]
            + [f"wet_{m}" for m in range(1, 13)]
        )
        w.writerows(rows)
    print(f"{len(rows)} towns written to {out}", file=sys.stderr)
    if missing:
        print("not in the city list (state main town kept): " + "; ".join(missing), file=sys.stderr)


if __name__ == "__main__":
    main()
