"""Rebuilds `college_start_records.csv` and `college_titles.csv` from Wikipedia
(minor bugs 294 and 295).

    python3 scripts/college_past.py OUTDIR [CACHE]

- Start records: every standings template of the 1979-1983 seasons at every
  level (NCAA Division I-A, I-AA, II, III and NAIA Divisions I and II; the
  season pages and the "standings templates" categories), each team's overall
  wins, losses and ties; a season missing there is read from the team's own
  season article ("1982 Georgia Southern Eagles football team", its infobox
  record).
- Titles: the champions by year of each level's championship page (the FBS
  table of NCAA-designated major selectors, every selection one title, as
  Wikipedia's totals count them; the I-AA/FCS, Division II and III, NAIA and
  NJCAA champions). Vacated titles are left out.
- Each team is matched to a world program by its football article (redirects
  followed), else by its name as shown.

Asks Wikipedia's API with a descriptive User-Agent and keeps every answer in
CACHE (default ./wiki-cache), so a rerun asks nothing again.
"""
import csv
import collections
import hashlib
import json
import os
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
REAL = os.path.dirname(HERE)
OUT = sys.argv[1] if len(sys.argv) > 1 else REAL
CACHE = sys.argv[2] if len(sys.argv) > 2 else "wiki-cache"
UA = "AFM-data-research/1.0 (https://github.com/cnbville/AFM; football sim research)"
os.makedirs(CACHE, exist_ok=True)


def api(params):
    params = dict(params)
    params["format"] = "json"
    params["formatversion"] = "2"
    q = urllib.parse.urlencode(sorted(params.items()))
    path = os.path.join(CACHE, hashlib.sha1(q.encode()).hexdigest() + ".json")
    if os.path.exists(path):
        return json.load(open(path))
    url = "https://en.wikipedia.org/w/api.php?" + q
    for attempt in range(8):
        try:
            req = urllib.request.Request(url, headers={"User-Agent": UA, "Accept-Encoding": "identity"})
            with urllib.request.urlopen(req, timeout=60) as r:
                d = json.load(r)
            json.dump(d, open(path, "w"))
            time.sleep(0.2)
            return d
        except urllib.error.HTTPError as e:
            ra = e.headers.get("Retry-After")
            time.sleep(float(ra) if ra and ra.isdigit() else min(60, 3 * 2**attempt))
        except Exception:
            time.sleep(min(60, 3 * 2**attempt))
    raise RuntimeError("failed " + url)


def _follow(q, t):
    norm = {n["from"]: n["to"] for n in q.get("normalized", [])}
    redir = {n["from"]: n["to"] for n in q.get("redirects", [])}
    f = norm.get(t, t)
    for _ in range(5):
        if f not in redir:
            break
        f = redir[f]
    return f.split("#")[0]


def wikitext_many(titles, chunk_size=50):
    out = {}
    titles = list(dict.fromkeys(titles))
    for i in range(0, len(titles), chunk_size):
        chunk = titles[i : i + chunk_size]
        d = api({"action": "query", "prop": "revisions", "rvprop": "content", "rvslots": "main", "titles": "|".join(chunk), "redirects": "1"})
        q = d.get("query", {})
        pages = {p["title"]: p for p in q.get("pages", [])}
        for t in chunk:
            f = _follow(q, t)
            p = pages.get(f)
            txt = p["revisions"][0]["slots"]["main"]["content"] if p and not p.get("missing") and p.get("revisions") else None
            out[t] = (f, txt)
    return out


def resolve_many(titles):
    out = {}
    titles = list(dict.fromkeys(t for t in titles if t))
    for i in range(0, len(titles), 50):
        chunk = titles[i : i + 50]
        q = api({"action": "query", "prop": "info", "titles": "|".join(chunk), "redirects": "1"}).get("query", {})
        pages = {p["title"]: p for p in q.get("pages", [])}
        for t in chunk:
            f = _follow(q, t)
            p = pages.get(f)
            out[t] = f if (p and not p.get("missing") and not p.get("invalid")) else None
    return out


def members(cat):
    out, cont = [], None
    while True:
        p = {"action": "query", "list": "categorymembers", "cmtitle": cat, "cmlimit": "500"}
        if cont:
            p["cmcontinue"] = cont
        d = api(p)
        out += [m["title"] for m in d.get("query", {}).get("categorymembers", [])]
        cont = d.get("continue", {}).get("cmcontinue")
        if not cont:
            return out


def url(title):
    return title if title.startswith("http") else "https://en.wikipedia.org/wiki/" + title.replace(" ", "_")


def norm(s):
    s = re.sub(r"<[^>]+>|\{\{[^{}]*\}\}|'''|''", "", s or "")
    s = s.replace("–", "-").replace("’", "'").lower()
    return re.sub(r"\s+", " ", re.sub(r"[^a-z0-9&()' .-]", " ", s)).strip()


# --- the world's programs and their football articles -----------------------
def rows(f):
    return list(csv.DictReader(open(os.path.join(REAL, f))))


progs = [{"name": r["name"], "division": r["division"], "state": r["state"], "dropped": False} for f in ["colleges.csv", "colleges_lower.csv"] for r in rows(f)]
seen = {p["name"] for p in progs}
for r in rows("college_history_dropped.csv"):
    if r["name"] not in seen:
        seen.add(r["name"])
        progs.append({"name": r["name"], "division": r["division"], "state": r["state"], "dropped": True})
mascot = {r["name"]: r["mascot"] for r in rows("mascots.csv")}
STATES = {"AL": "Alabama", "AK": "Alaska", "AZ": "Arizona", "AR": "Arkansas", "CA": "California", "CO": "Colorado", "CT": "Connecticut", "DE": "Delaware", "DC": "Washington, D.C.", "FL": "Florida", "GA": "Georgia", "HI": "Hawaii", "ID": "Idaho", "IL": "Illinois", "IN": "Indiana", "IA": "Iowa", "KS": "Kansas", "KY": "Kentucky", "LA": "Louisiana", "ME": "Maine", "MD": "Maryland", "MA": "Massachusetts", "MI": "Michigan", "MN": "Minnesota", "MS": "Mississippi", "MO": "Missouri", "MT": "Montana", "NE": "Nebraska", "NV": "Nevada", "NH": "New Hampshire", "NJ": "New Jersey", "NM": "New Mexico", "NY": "New York", "NC": "North Carolina", "ND": "North Dakota", "OH": "Ohio", "OK": "Oklahoma", "OR": "Oregon", "PA": "Pennsylvania", "RI": "Rhode Island", "SC": "South Carolina", "SD": "South Dakota", "TN": "Tennessee", "TX": "Texas", "UT": "Utah", "VT": "Vermont", "VA": "Virginia", "WA": "Washington", "WV": "West Virginia", "WI": "Wisconsin", "WY": "Wyoming"}
LINK = re.compile(r"\[\[([^\]|#]+)(?:#[^\]|]*)?(?:\|([^\]]+))?\]\]")
LISTS = ["List of NCAA Division I FBS football programs", "List of NCAA Division I FCS football programs", "List of NCAA Division II football programs", "List of NCAA Division III football programs", "List of NAIA football programs"]
table = []
for t, (f, txt) in wikitext_many(LISTS).items():
    for row in re.split(r"\n\|-", txt or ""):
        fb = [a.strip() for a, b in LINK.findall(row) if a.strip().endswith(" football")]
        cells = [c for c in re.split(r"\n\||\|\|", row) if c.strip()]
        if not fb or not cells:
            continue
        disp = set()
        for c in cells[:3]:
            for a, b in LINK.findall(c):
                disp.update([norm(b or a), norm(a)])
            disp.add(norm(LINK.sub(lambda m: m.group(2) or m.group(1), c)))
        st = [s for s in STATES.values() if f"[[{s}" in row or f"|{s}]]" in row]
        table.append({"fb": fb, "disp": disp, "states": st})
# Hand fixes: a list row or mascot that points at another school's page.
FIX = {"ULM": "Louisiana–Monroe Warhawks football", "PennWest California": "California Vulcans football", "UTRGV": "UT Rio Grande Valley Vaqueros football", "Miami (FL)": "Miami Hurricanes football", "Miami (OH)": "Miami RedHawks football", "Greensboro": "Greensboro Pride football", "Hartwick": "Hartwick Hawks football", "Lakeland": "Lakeland Muskies football", "Methodist": "Methodist Monarchs football", "Midland": "Midland Warriors football", "Missouri Valley": "Missouri Valley Vikings football", "Rocky Mountain": "Rocky Mountain Battlin' Bears football", "Butte": None, "Fullerton": None, "Jacksonville": "Jacksonville Dolphins football", "Pacific": "Pacific Tigers football", "Cal State Fullerton": "Cal State Fullerton Titans football", "Bethune-Cookman": "Bethune–Cookman Wildcats football", "Long Beach State": "Long Beach State 49ers football", "Wichita State": "Wichita State Shockers football", "UT Arlington": "UT Arlington Mavericks football", "Northeastern": "Northeastern Huskies football", "Canisius": "Canisius Golden Griffins football", "Iona": "Iona Gaels football", "La Salle": "La Salle Explorers football", "Saint Peter's": "Saint Peter's Peacocks football", "Siena": "Siena Saints football", "Morris Brown": "Morris Brown Wolverines football", "Saint Mary's (CA)": "Saint Mary's Gaels football", "St. John's (NY)": "St. John's Red Storm football", "Hofstra": "Hofstra Pride football", "Cal State Northridge": "Cal State Northridge Matadors football", "Jamestown": "Jamestown Jimmies football", "La Verne": "La Verne Leopards football", "Puget Sound": "Puget Sound Loggers football", "Concordia-Moorhead": "Concordia Cobbers football", "Minnesota State Moorhead": "Minnesota State–Moorhead Dragons football", "PennWest Clarion": "Clarion Golden Eagles football", "PennWest Edinboro": "Edinboro Fighting Scots football", "Emory & Henry": "Emory & Henry Wasps football", "Vermont State Castleton": "Castleton Spartans football"}
cands = {}
for p in progs:
    if p["name"] in FIX:
        cands[p["name"]] = [FIX[p["name"]]] if FIX[p["name"]] else []
        continue
    c = []
    base = re.sub(r"\s*\(.*?\)", "", p["name"])
    if p["name"] in mascot:
        c.append(f"{base} {mascot[p['name']]} football")
    st = STATES.get(p["state"], "")
    for r in table:
        if (norm(p["name"]) in r["disp"] or norm(base) in r["disp"]) and (not r["states"] or st in r["states"]):
            c += r["fb"]
    cands[p["name"]] = list(dict.fromkeys(c))
res = resolve_many([x for v in cands.values() for x in v])
for p in progs:
    got = [res[x] for x in cands[p["name"]] if res.get(x)]
    # A page named for another school is not this one's.
    got = [g for g in got if p["name"] in FIX or g.replace("–", "-").lower().startswith(norm(re.sub(r"\s*\(.*?\)", "", p["name"])).split(" (")[0][:4])]
    p["article"] = got[0] if got else None
arts = {p["article"] for p in progs if p["article"]}

# --- start records: 1979-1983 standings --------------------------------------
LEVELS = {"I-A": "NCAA Division I-A", "I-AA": "NCAA Division I-AA", "D2": "NCAA Division II", "D3": "NCAA Division III", "NAIA1": "NAIA Division I", "NAIA2": "NAIA Division II"}
YEARS = range(1979, 1984)
tpls = set()
for y in YEARS:
    for nm in LEVELS.values():
        tpls.update(t for t in members(f"Category:{y} {nm} football standings templates") if t.startswith("Template:"))
for t, (f, txt) in wikitext_many([f"{y} {nm} football season" for y in YEARS for nm in LEVELS.values()]).items():
    for m in re.findall(r"\{\{\s*([^{}|]*?(?:standings|records)[^{}|]*?)\s*[|}]", txt or "", re.I):
        if re.match(r"\d{4} ", m.strip()):
            tpls.add("Template:" + m.strip())


def split_top(s):
    parts, depth, cur, i = [], 0, "", 0
    while i < len(s):
        two = s[i : i + 2]
        if two in ("{{", "[["):
            depth += 1
            cur += two
            i += 2
        elif two in ("}}", "]]"):
            depth -= 1
            cur += two
            i += 2
        elif s[i] == "|" and depth == 0:
            parts.append(cur)
            cur = ""
            i += 1
        else:
            cur += s[i]
            i += 1
    return parts + [cur]


def entries_of(txt):
    low, out, i = txt.lower(), [], 0
    while True:
        j = low.find("{{cfb standings entry", i)
        if j < 0:
            return out
        depth, k = 0, j
        while k < len(txt):
            if txt.startswith("{{", k):
                depth += 1
                k += 2
            elif txt.startswith("}}", k):
                depth -= 1
                k += 2
                if depth == 0:
                    break
            else:
                k += 1
        out.append(txt[j + 2 : k - 2])
        i = k


def num(x):
    x = re.sub(r"<[^>]*>|\{\{[^{}]*\}\}", "", x or "").strip()
    return int(x) if x.isdigit() else None


E = []
for t, (f, txt) in wikitext_many(sorted(tpls)).items():
    if not txt:
        continue
    ties = bool(re.search(r"CFB Standings Start[^}]*ties\s*=\s*y", txt, re.I))
    season = int(t.split(":")[1][:4])
    for e in entries_of(txt):
        parts = split_top(e)[1:]
        team = parts[0] if parts else ""
        named, pos = {}, []
        for p in parts[1:]:
            if "=" in p and not p.strip().startswith("{{"):
                k, v = p.split("=", 1)
                named[k.strip()] = v.strip()
            else:
                pos.append(p.strip())
        nums = [num(x) for x in pos]
        if "w" in named:
            w, l, tie = num(named.get("w")), num(named.get("l")), num(named.get("t")) or 0
        elif ties and len(nums) >= 6 or len(nums) == 6:
            w, l, tie = nums[3], nums[4], nums[5] or 0
        elif not ties and len(nums) >= 4:
            w, l, tie = nums[2], nums[3], 0
        else:
            continue
        if w is None or l is None:
            continue
        target, disp = None, None
        m = re.search(r"\[\[([^\]|]+)(?:\|([^\]]+))?\]\]", team)
        if m:
            target, disp = m.group(1).strip(), (m.group(2) or m.group(1)).strip()
        m2 = re.search(r"\{\{\s*cfb link\s*\|([^{}]*)\}\}", team, re.I)
        if m2:
            kv = {k.strip(): v.strip() for k, v in (x.split("=", 1) for x in m2.group(1).split("|") if "=" in x)}
            if "team" in kv:
                target = f"{kv.get('year', '')} {kv['team']} football team".strip()
            disp = kv.get("title", disp)
        if not disp:
            disp = re.sub(r"\{\{[^{}]*\}\}|\[\[|\]\]|'''|<[^>]*>", "", team).strip()
        cand = re.sub(r" football team$", " football", re.sub(r"^\d{4} ", "", target.strip())) if target else None
        E.append({"season": season, "cand": cand, "display": disp, "w": w, "l": l, "t": tie, "src": t})
res = resolve_many([e["cand"] for e in E if e["cand"]])
by_canon, by_disp = collections.defaultdict(list), collections.defaultdict(list)
for e in E:
    e["canon"] = res.get(e["cand"]) if e["cand"] else None
    if e["canon"]:
        by_canon[e["canon"]].append(e)
    by_disp[norm(e["display"])].append(e)
recs = {}
for p in progs:
    got = list(by_canon.get(p["article"], [])) if p["article"] else []
    got += [e for e in by_disp.get(norm(p["name"]), []) if e not in got and (not e["canon"] or e["canon"] == p["article"] or e["canon"] not in arts)]
    per = {}
    for e in got:
        g = e["w"] + e["l"] + e["t"]
        if e["season"] not in per or g > per[e["season"]][3]:
            per[e["season"]] = (e["w"], e["l"], e["t"], g, e["src"])
    recs[p["name"]] = per
# A season missing from the standings: the team's own season article.
want = {}
for p in progs:
    if p["article"] and p["article"].endswith(" football") and p["division"] != "JUCO":
        for y in YEARS:
            if y not in recs[p["name"]]:
                want[f"{y} {p['article'][: -len(' football')]} football team"] = (p["name"], y)
have = [t for t, r in resolve_many(list(want)).items() if r]
for t, (f, txt) in wikitext_many(have, chunk_size=20).items():
    m = re.search(r"\|\s*record\s*=\s*([^\n]*)", txt or "")
    mm = re.match(r"\s*(\d+)\s*[–-]\s*(\d+)(?:\s*[–-]\s*(\d+))?", re.sub(r"\{\{[^{}]*\}\}|<[^>]*>|\[\[|\]\]", "", m.group(1))) if m else None
    if mm:
        name, y = want[t]
        w, l, tie = int(mm.group(1)), int(mm.group(2)), int(mm.group(3) or 0)
        recs[name][y] = (w, l, tie, w + l + tie, f)
with open(os.path.join(OUT, "college_start_records.csv"), "w", newline="") as f:
    w = csv.writer(f, lineterminator="\n")
    w.writerow(["name", "season", "wins", "losses", "ties", "source_url"])
    for p in sorted(progs, key=lambda p: p["name"]):
        for y, v in sorted(recs[p["name"]].items()):
            w.writerow([p["name"], y, v[0], v[1], v[2], url(v[4])])

# --- titles by year and level -----------------------------------------------
TLINK = re.compile(r"\[\[([^\]|]+)(?:\|([^\]]+))?\]\]|\{\{\s*cfb link\s*\|([^{}]*)\}\}", re.I)


def teams_in(s):
    out = []
    for m in TLINK.finditer(s):
        if m.group(3) is not None:
            kv = {k.strip(): v.strip() for k, v in (x.split("=", 1) for x in m.group(3).split("|") if "=" in x)}
            if "team" in kv:
                out.append((m.start(), f"{kv['team']} football", kv.get("title", kv["team"])))
            continue
        t, d = m.group(1).strip(), (m.group(2) or m.group(1)).strip()
        if t.endswith("football team"):
            out.append((m.start(), re.sub(r"^\d{4} ", "", t).replace(" football team", " football"), d))
        elif t.endswith(" football") and not re.match(r"^\d{4} ", t):
            out.append((m.start(), t, d))
    return out


def seasons_in(s):
    return [(m.start(), int(m.group(1))) for m in re.finditer(r"\[\[(\d{4}) [^\]|]*(?:season|football)[^\]|]*\|\s*'*\d{4}", s)]


PAGES = {"College football national championships in NCAA Division I FBS": "FBS", "NCAA Division I Football Championship": "FCS", "NCAA Division II Football Championship": "D2", "NCAA Division III football championship": "D3", "NAIA football national championship": "NAIA", "NAIA Division II football national championship": "NAIA", "NJCAA National Football Championship": "JUCO"}
FLOOR = {"FCS": 1978, "D2": 1973, "D3": 1973, "NAIA": 1956, "JUCO": 1900}
pages = wikitext_many(list(PAGES))
T = []
for t, lvl in PAGES.items():
    f, txt = pages[t]
    if lvl == "FBS":
        # The by-year table of major selectors: every team listed is one title.
        body = txt[: txt.find("===Total championship selections from major selectors by school===")]
        best = []
        for tb in body.split("\n{|")[1:]:
            got, cur = [], None
            for row in re.split(r"\n\|-", tb.split("\n|}")[0]):
                ss = seasons_in(row)
                if ss:
                    cur = ss[0][1]
                tm = [x for x in teams_in(row) if not ss or x[0] > ss[0][0]]
                if cur and tm and "||" in row:
                    got.append((cur, tm[0][1], tm[0][2]))
            best = got if len(got) > len(best) else best
        T += [(y, lvl, a, d, f) for y, a, d in best]
        continue
    # A champions table: the champion is the first team after the season;
    # each season from the page's longest table that has it.
    tables = txt.split("\n{|")[1:]
    rowsn = [len([r for r in re.split(r"\n\|-", tb.split("\n|}")[0]) if seasons_in(r)]) for tb in tables]
    best = {}
    for ti in sorted(range(len(tables)), key=lambda i: -rowsn[i]):
        for row in re.split(r"\n\|-", tables[ti].split("\n|}")[0]):
            cs = re.split(r"\|\||\n\s*\||\n\s*!|!!", re.sub(r"<ref[^>]*/>|<ref[^>]*>.*?</ref>", "", row, flags=re.S))
            si = next((i for i, c in enumerate(cs) if seasons_in(c)), None)
            if si is None:
                continue
            season = seasons_in(cs[si])[0][1]
            champ = next(([(a, d) for _, a, d in teams_in(c)] for c in cs[si + 1 :] if teams_in(c)), None)
            if not champ or "vacated" in row.lower() or season < FLOOR[lvl] or season in best:
                continue
            best[season] = champ
    T += [(y, lvl, a, d, f) for y, champ in best.items() for a, d in champ]
res = resolve_many([x[2] for x in T])
by_art = {}
for p in progs:
    if p["article"]:
        by_art.setdefault(p["article"], p["name"])
by_name = {norm(p["name"]): p["name"] for p in progs}


def juco(s):
    return re.sub(r" (cc|jc|community college|junior college|college)$", "", norm(s))


by_juco = {juco(p["name"]): p["name"] for p in progs if p["division"] == "JUCO"}
by_juco.update({"butler county": by_juco.get("butler"), "butler (ks)": by_juco.get("butler")})
out = set()
for y, lvl, a, d, src in T:
    canon = res.get(a) or a
    name = by_art.get(canon) or (by_juco.get(juco(d)) if lvl == "JUCO" else None) or by_name.get(norm(d))
    if name:
        out.add((name, y, lvl, url(src)))
with open(os.path.join(OUT, "college_titles.csv"), "w", newline="") as f:
    w = csv.writer(f, lineterminator="\n")
    w.writerow(["name", "season", "level", "source_url"])
    for r in sorted(out, key=lambda r: (r[0], r[1], r[2])):
        w.writerow(r)
print("start records and titles written to", OUT)
