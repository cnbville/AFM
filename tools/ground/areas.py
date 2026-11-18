#!/usr/bin/env python3
"""Ground's areas (lane G1a): the hand-written seeds to counties, centers and map art.

Reads `data/real-world/ground_area_seeds.csv` (per state, the areas real staffs
use, each with one to three core counties, or every county for Georgia) and the
Census county shapes from us-atlas 3.0.1 (ISC; Census cartographic boundaries).
A county no seed names joins the area of the nearest seeded county in its own
state, by centroid.

Writes:
- `data/real-world/ground_areas.json`: one row per area (state, key, name,
  nickname, area codes, counties, center, label point, square miles, real high
  schools from `hs_towns.csv` slots) and the town table's county spellings the
  atlas spells differently, each with the area it falls in.
- `app/ui/art/ground/<ST>.json`: each state's drawn areas: every area's fill
  path, the border arcs between areas, the outer edge and the projection.

Usage: python3 tools/ground/areas.py [path to the us-atlas package folder]
Without a path it downloads the package from the npm registry (the proxy blocks
jsdelivr and unpkg) into a cache folder beside this script.
"""
import csv
import json
import math
import os
import sys
import tarfile
import unicodedata
import urllib.request

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
SEEDS = os.path.join(ROOT, "data/real-world/ground_area_seeds.csv")
TOWNS = os.path.join(ROOT, "data/real-world/hs_towns.csv")
OUT = os.path.join(ROOT, "data/real-world/ground_areas.json")
ART = os.path.join(ROOT, "app/ui/art/ground")
ATLAS_URL = "https://registry.npmjs.org/us-atlas/-/us-atlas-3.0.1.tgz"

FIPS = {
    "01": "AL", "02": "AK", "04": "AZ", "05": "AR", "06": "CA", "08": "CO", "09": "CT", "10": "DE",
    "11": "DC", "12": "FL", "13": "GA", "15": "HI", "16": "ID", "17": "IL", "18": "IN", "19": "IA",
    "20": "KS", "21": "KY", "22": "LA", "23": "ME", "24": "MD", "25": "MA", "26": "MI", "27": "MN",
    "28": "MS", "29": "MO", "30": "MT", "31": "NE", "32": "NV", "33": "NH", "34": "NJ", "35": "NM",
    "36": "NY", "37": "NC", "38": "ND", "39": "OH", "40": "OK", "41": "OR", "42": "PA", "44": "RI",
    "45": "SC", "46": "SD", "47": "TN", "48": "TX", "49": "UT", "50": "VT", "51": "VA", "53": "WA",
    "54": "WV", "55": "WI", "56": "WY",
}
# Independent cities share a name with a county; the town table writes "X city".
CITY_STATES = {"VA", "MD", "MO", "NV"}
W = 800  # drawn width of every state's map
PAD = 10
TOL = 0.6  # simplification in drawn pixels


def norm(s):
    s = unicodedata.normalize("NFKD", s).encode("ascii", "ignore").decode()
    return " ".join(s.lower().replace(".", "").split())


def atlas_dir(arg):
    if arg:
        return arg
    cache = os.path.join(os.path.dirname(__file__), ".cache")
    pkg = os.path.join(cache, "package")
    if not os.path.exists(os.path.join(pkg, "counties-10m.json")):
        os.makedirs(cache, exist_ok=True)
        tgz = os.path.join(cache, "us-atlas.tgz")
        urllib.request.urlretrieve(ATLAS_URL, tgz)
        with tarfile.open(tgz) as t:
            t.extractall(cache, filter="data")
    return pkg


def miles(a, b):
    (la1, lo1), (la2, lo2) = a, b
    p1, p2 = math.radians(la1), math.radians(la2)
    h = math.sin((p2 - p1) / 2) ** 2 + math.cos(p1) * math.cos(p2) * math.sin(math.radians(lo2 - lo1) / 2) ** 2
    return 3959.0 * 2 * math.asin(min(1.0, math.sqrt(h)))


def main():
    pkg = atlas_dir(sys.argv[1] if len(sys.argv) > 1 else None)
    topo = json.load(open(os.path.join(pkg, "counties-10m.json")))
    sc, tr = topo["transform"]["scale"], topo["transform"]["translate"]
    # Arcs in quantized integers (exact endpoints for stitching).
    qarcs = []
    for a in topo["arcs"]:
        x = y = 0
        pts = []
        for dx, dy in a:
            x += dx
            y += dy
            pts.append((x, y))
        qarcs.append(pts)

    def lonlat(q, wrap=False):
        lon, lat = q[0] * sc[0] + tr[0], q[1] * sc[1] + tr[1]
        if wrap and lon > 0:
            lon -= 360
        return lon, lat

    def ring_q(idx):
        out = []
        for i in idx:
            p = qarcs[i] if i >= 0 else qarcs[~i][::-1]
            out.extend(p if not out else p[1:])
        return out

    # Counties: state, display name, polygons (lists of rings of arc ids),
    # centroid and square miles.
    counties = []
    for g in topo["objects"]["counties"]["geometries"]:
        st = FIPS.get(g["id"][:2])
        if not st or g["type"] not in ("Polygon", "MultiPolygon"):
            continue
        name = g["properties"]["name"]
        if st in CITY_STATES and int(g["id"][2:]) >= 500 and not name.endswith("City"):
            name += " city"
        polys = [g["arcs"]] if g["type"] == "Polygon" else g["arcs"]
        wrap = st == "AK"
        cx = cy = area = 0.0
        for poly in polys:
            for k, r in enumerate(poly):
                pts = [lonlat(q, wrap) for q in ring_q(r)]
                lat0 = sum(p[1] for p in pts) / len(pts)
                c = math.cos(math.radians(lat0))
                s = 0.0
                sx = sy = 0.0
                for (x1, y1), (x2, y2) in zip(pts, pts[1:] + pts[:1]):
                    cr = (x1 * c) * y2 - (x2 * c) * y1
                    s += cr
                    sx += (x1 * c + x2 * c) * cr
                    sy += (y1 + y2) * cr
                s /= 2
                if abs(s) < 1e-12:
                    continue
                # Topojson rings: outer rings clockwise, holes counterclockwise.
                sgn = 1 if k == 0 else -1
                area += sgn * abs(s)
                cx += sgn * abs(s) * (sx / (6 * s)) / c
                cy += sgn * abs(s) * (sy / (6 * s))
        if area <= 0:
            continue
        cen = (cy / area, cx / area)
        sqmi = area * 69.0 * 69.17
        counties.append({"fips": g["id"], "st": st, "name": name, "polys": polys, "cen": cen, "sqmi": sqmi})
    by_name = {}
    for i, c in enumerate(counties):
        by_name.setdefault((c["st"], norm(c["name"])), []).append(i)

    # Seeds.
    areas = []
    owner = {}
    for row in csv.DictReader(open(SEEDS)):
        a = {
            "state": row["state"],
            "key": row["key"],
            "name": row["name"],
            "nickname": row["nickname"],
            "codes": [c for c in row["area_codes"].split(";") if c],
            "seeds": [],
        }
        for c in row["counties"].split(";"):
            st, nm = (c.split(":", 1) if ":" in c else (row["state"], c))
            hits = by_name.get((st, norm(nm)))
            if not hits:
                sys.exit(f"seed county not in the atlas: {row['state']} {row['key']} {c}")
            for h in hits:
                if h in owner:
                    sys.exit(f"county seeded twice: {c}")
                owner[h] = len(areas)
                a["seeds"].append(h)
        areas.append(a)
    ids = [f"{a['state']}-{a['key']}" for a in areas]
    if len(set(ids)) != len(ids):
        sys.exit("an area key repeats in its state")
    home = {}
    for k, a in enumerate(areas):
        home.setdefault(a["state"], []).append(k)
    # Every other county joins the nearest seeded county of an area of its own state.
    for i, c in enumerate(counties):
        if i in owner:
            continue
        cands = [s for k in home.get(c["st"], []) for s in areas[k]["seeds"]]
        if not cands:
            sys.exit(f"no area in {c['st']}")
        best = min(cands, key=lambda s: miles(c["cen"], counties[s]["cen"]))
        owner[i] = owner[best]
    members = [[] for _ in areas]
    for i, k in owner.items():
        members[k].append(i)

    # The town table: each town's county to its area; spellings the atlas
    # lacks matched by the town's place to the nearest county centroid.
    aliases = {}
    schools = [0] * len(areas)
    slots = [0] * len(counties)
    for r in csv.DictReader(open(TOWNS)):
        st, cn = r["state"], r["county"]
        hits = by_name.get((st, norm(cn))) if cn else None
        if hits:
            i = hits[0]
            if counties[i]["name"] != cn:
                aliases[(st, cn)] = owner[i]
        else:
            place = (float(r["lat"]), float(r["lon"]))
            own = [j for j, c in enumerate(counties) if c["st"] == st]
            i = min(own, key=lambda j: miles(place, counties[j]["cen"]))
            if cn:
                aliases[(st, cn)] = owner[i]
        schools[owner[i]] += int(r["slots"] or 1)
        slots[i] += int(r["slots"] or 1)

    rows = []
    for k, a in enumerate(areas):
        ms = members[k]
        tot = sum(counties[i]["sqmi"] for i in ms)
        lat = sum(counties[i]["cen"][0] * counties[i]["sqmi"] for i in ms) / tot
        lon = sum(counties[i]["cen"][1] * counties[i]["sqmi"] for i in ms) / tot
        # The label sits on the member county nearest the center (inside the area).
        lab = min(ms, key=lambda i: miles((lat, lon), counties[i]["cen"]))
        names = sorted(
            (counties[i]["name"] if counties[i]["st"] == a["state"] else f"{counties[i]['st']}:{counties[i]['name']}")
            for i in ms
        )
        # How close together its schools sit: each school's own county's
        # schools per 100 square miles, averaged over the schools.
        n = sum(slots[i] for i in ms)
        near = sum(slots[i] * slots[i] / counties[i]["sqmi"] * 100 for i in ms) / n if n else 0.0
        rows.append({
            "state": a["state"],
            "key": a["key"],
            "name": a["name"],
            "nickname": a["nickname"],
            "codes": a["codes"],
            "seeds": [counties[i]["name"] for i in a["seeds"]],
            "counties": names,
            "center": [round(lat, 4), round(lon, 4)],
            "label": [round(counties[lab]["cen"][0], 4), round(counties[lab]["cen"][1], 4)],
            "sq_miles": round(tot),
            "schools": schools[k],
            "near": round(near, 2),
        })
    out = {
        "source": "tools/ground/areas.py from ground_area_seeds.csv and us-atlas 3.0.1 counties-10m (ISC; Census cartographic boundaries)",
        "areas": rows,
        "aliases": [
            {"state": st, "county": cn, "area": areas[k]["key"]} for (st, cn), k in sorted(aliases.items())
        ],
    }
    with open(OUT, "w") as f:
        json.dump(out, f, indent=0, ensure_ascii=False)
        f.write("\n")
    print(f"{len(rows)} areas, {len(counties)} counties, {len(aliases)} town-table spellings the atlas writes differently")
    print(f"wrote {os.path.relpath(OUT, ROOT)} ({os.path.getsize(OUT):,} bytes)")

    # Art per state.
    os.makedirs(ART, exist_ok=True)
    total = 0
    for st in sorted(set(c["st"] for c in counties)):
        drawn = [i for i, c in enumerate(counties) if c["st"] == st or areas[owner[i]]["state"] == st]
        wrap = st == "AK"
        lls = [lonlat(q, wrap) for i in drawn for poly in counties[i]["polys"] for r in poly for q in ring_q(r)]
        lon0, lon1 = min(p[0] for p in lls), max(p[0] for p in lls)
        lat0, lat1 = min(p[1] for p in lls), max(p[1] for p in lls)
        cos = math.cos(math.radians((lat0 + lat1) / 2))
        k = (W - 2 * PAD) / ((lon1 - lon0) * cos)
        h = round((lat1 - lat0) * k + 2 * PAD)
        if h > 1400:  # a tall state keeps its height in bounds
            k = (1400 - 2 * PAD) / (lat1 - lat0)
            h = 1400
        w = round((lon1 - lon0) * cos * k + 2 * PAD)

        def P(q):
            lon, lat = lonlat(q, wrap)
            return (PAD + (lon - lon0) * cos * k, PAD + (lat1 - lat) * k)

        simp = {}

        def arc_pts(i):
            if i not in simp:
                simp[i] = dp([P(q) for q in qarcs[i]], TOL)
            return simp[i]

        # Which areas use each arc.
        use = {}
        for i in drawn:
            for poly in counties[i]["polys"]:
                for r in poly:
                    for a in r:
                        use.setdefault(a if a >= 0 else ~a, []).append((owner[i], a))
        fills = {}
        for kk in sorted(set(owner[i] for i in drawn)):
            # The area's own edge: arcs only one of its counties uses.
            edge = []
            for a, us in use.items():
                mine = [d for o, d in us if o == kk]
                if len(mine) == 1:
                    edge.append(mine[0])
            fills[ids[kk]] = {"home": areas[kk]["state"], "d": stitch(edge, qarcs, arc_pts)}
        borders, outer = [], []
        for a, us in use.items():
            owners = set(o for o, _ in us)
            if len(owners) > 1:
                borders.append(a)
            elif len(us) == 1:
                outer.append(a)
        art = {
            "state": st,
            "source": "tools/ground/areas.py (us-atlas 3.0.1, Census cartographic boundaries)",
            "projection": {
                "w": w, "h": h, "pad": PAD, "lon0": round(lon0, 5), "lat1": round(lat1, 5),
                "cos": round(cos, 6), "k": round(k, 4), "wrap": wrap,
                "x": "pad + (lon - lon0) * cos * k (with wrap, a positive lon less 360)",
                "y": "pad + (lat1 - lat) * k",
            },
            "areas": fills,
            "borders": lines(borders, arc_pts),
            "edge": lines(outer, arc_pts),
        }
        path = os.path.join(ART, f"{st}.json")
        with open(path, "w") as f:
            json.dump(art, f, separators=(",", ":"))
        total += os.path.getsize(path)
    print(f"wrote app/ui/art/ground/*.json: {total:,} bytes for {len(set(c['st'] for c in counties))} states")


def dp(pts, tol):
    """Douglas-Peucker on one arc; its ends stay, so shared arcs still meet."""
    if len(pts) < 3:
        return pts
    keep = [False] * len(pts)
    keep[0] = keep[-1] = True
    stack = [(0, len(pts) - 1)]
    while stack:
        a, b = stack.pop()
        (x1, y1), (x2, y2) = pts[a], pts[b]
        dx, dy = x2 - x1, y2 - y1
        L = math.hypot(dx, dy)
        best, bi = -1.0, -1
        for i in range(a + 1, b):
            x, y = pts[i]
            d = abs(dy * (x - x1) - dx * (y - y1)) / L if L else math.hypot(x - x1, y - y1)
            if d > best:
                best, bi = d, i
        if best > tol:
            keep[bi] = True
            stack += [(a, bi), (bi, b)]
    return [p for p, k in zip(pts, keep) if k]


def fmt(p):
    return f"{p[0]:.1f},{p[1]:.1f}"


def stitch(directed, qarcs, arc_pts):
    """Joins an area's edge arcs (in ring direction) into closed rings."""
    def ends(d):
        a = qarcs[d] if d >= 0 else qarcs[~d][::-1]
        return a[0], a[-1]

    def pts(d):
        p = arc_pts(d if d >= 0 else ~d)
        return p if d >= 0 else p[::-1]

    start = {}
    for d in directed:
        start.setdefault(ends(d)[0], []).append(d)
    used = set()
    out = []
    for d0 in directed:
        if d0 in used:
            continue
        ring = []
        d = d0
        first = ends(d0)[0]
        while d is not None and d not in used:
            used.add(d)
            p = pts(d)
            ring.extend(p if not ring else p[1:])
            e = ends(d)[1]
            if e == first:
                break
            nxt = [x for x in start.get(e, []) if x not in used]
            d = nxt[0] if nxt else None
        if len(ring) >= 3:
            out.append("M" + "L".join(fmt(p) for p in ring) + "Z")
    return "".join(out)


def lines(arc_ids, arc_pts):
    return "".join("M" + "L".join(fmt(p) for p in arc_pts(a)) for a in sorted(arc_ids))


if __name__ == "__main__":
    main()
