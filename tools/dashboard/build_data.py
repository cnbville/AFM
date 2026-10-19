#!/usr/bin/env python3
"""Debug dashboard v2: turns the raw output of world trials into the data the
dashboard page reads. Debug and trial tooling only (hidden values included).

Usage: build_data.py OUT_DIR RUN_DIR [RUN_DIR ...]
Each RUN_DIR holds report.txt, dash.json, cards.json and raw/ (the
AFM_RAW export, .jsonl or .jsonl.gz). Writes OUT_DIR/seed-<seed>.json per
run and OUT_DIR/real.json (the real-football reference values).
"""

import gzip
import json
import math
import os
import random
import re
import statistics as st
import sys
from collections import Counter, defaultdict
from datetime import datetime

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
REAL_DIR = os.path.join(REPO, "data", "real-world")

POWER4 = {"SEC", "Big Ten", "Big 12", "ACC"}
DIV_RANK = {"FBS": 0, "FCS": 1, "D2": 2, "JUCO": 3}


def jl(run, name):
    """Rows of one raw export file (plain or gzip)."""
    for path, opener in ((f"{run}/raw/{name}.jsonl", open), (f"{run}/raw/{name}.jsonl.gz", gzip.open)):
        if os.path.exists(path):
            with opener(path, "rt") as f:
                for line in f:
                    if line.strip():
                        yield json.loads(line)
            return
    raise FileNotFoundError(f"{run}/raw/{name}.jsonl")


def r(x, n=1):
    if x is None or (isinstance(x, float) and (math.isnan(x) or math.isinf(x))):
        return None
    return round(x, n)


def pct(a, b, n=1):
    return r(100.0 * a / b, n) if b else None


def quant(xs, qs=(0.1, 0.25, 0.5, 0.75, 0.9)):
    xs = sorted(xs)
    if not xs:
        return [None] * len(qs)
    return [r(xs[min(len(xs) - 1, int(q * (len(xs) - 1) + 0.5))]) for q in qs]


def corr(xs, ys):
    if len(xs) < 3:
        return None
    mx, my = st.fmean(xs), st.fmean(ys)
    sx = math.sqrt(sum((x - mx) ** 2 for x in xs))
    sy = math.sqrt(sum((y - my) ** 2 for y in ys))
    if sx == 0 or sy == 0:
        return None
    return r(sum((x - mx) * (y - my) for x, y in zip(xs, ys)) / (sx * sy), 3)


def ols(X, y):
    """Least squares with an intercept, pure Python (normal equations).
    Returns (coefficients without the intercept, R squared)."""
    n, k = len(X), len(X[0]) + 1
    A = [[1.0] + list(row) for row in X]
    ata = [[sum(A[i][a] * A[i][b] for i in range(n)) for b in range(k)] for a in range(k)]
    aty = [sum(A[i][a] * y[i] for i in range(n)) for a in range(k)]
    # Gauss-Jordan with a small ridge for stability.
    for a in range(k):
        ata[a][a] += 1e-6
    m = [row[:] + [aty[i]] for i, row in enumerate(ata)]
    for c in range(k):
        p = max(range(c, k), key=lambda i: abs(m[i][c]))
        m[c], m[p] = m[p], m[c]
        if abs(m[c][c]) < 1e-12:
            continue
        for i in range(k):
            if i != c:
                f = m[i][c] / m[c][c]
                for j in range(c, k + 1):
                    m[i][j] -= f * m[c][j]
    beta = [m[i][k] / m[i][i] if abs(m[i][i]) > 1e-12 else 0.0 for i in range(k)]
    pred = [sum(b * a for b, a in zip(beta, row)) for row in A]
    my = st.fmean(y)
    ss_t = sum((v - my) ** 2 for v in y)
    ss_r = sum((v - p) ** 2 for v, p in zip(y, pred))
    return beta[1:], (1 - ss_r / ss_t) if ss_t else None, beta[0]


# ---------------------------------------------------------------------------
# Real-football reference values (every one names its source)
# ---------------------------------------------------------------------------


def real_values():
    cgt = json.load(open(f"{REAL_DIR}/college_game_targets.json"))
    nfl = json.load(open(f"{REAL_DIR}/nfl_game_targets.json"))
    t3 = json.load(open(f"{REAL_DIR}/trial3_baselines.json"))
    rates = json.load(open(f"{REAL_DIR}/rates.json"))
    eng = json.load(open(f"{REAL_DIR}/engine_targets.json"))
    nsit = json.load(open(f"{REAL_DIR}/nfl_situations.json"))
    vit = {}
    for div, key in (("FBS", "fbs"), ("FCS", "fcs")):
        d = cgt["divisions"][key]
        pg, rt = d["per_team_game"], d["rates"]
        vit[div] = {
            "points": pg["final_score"],
            "plays": pg["plays"],
            "pass_rate": 100 * rt["pass_rate"],
            "cmp_pct": 100 * rt["completion_pct"],
            "ypa": rt["yards_per_attempt"],
            "ypc": rt["yards_per_carry"],
            "sack_rate": 100 * rt["sack_rate"],
            "int_rate": 100 * rt["int_rate"],
            "third_pct": 100 * rt["third_down_conversion"],
            "fourth_go": pg["fourth_go"],
            "fourth_pct": 100 * rt["fourth_down_conversion"],
            "success_rate": 100 * rt["success_rate"],
            # Accepted fouls on a team's own snaps, by either side: summed over
            # both teams that is every foul in the game, so per team it equals
            # the fouls each team commits on average.
            "penalties": pg["penalties_on_plays_by_either"],
            "fg_pct": None,
            "punt_gross": rt.get("punt_gross"),
        }
        fg = d.get("field_goal_pct_by_distance", {})
        vit[div]["fg_by_distance"] = {k: 100 * v["pct"] for k, v in fg.items()}
    # FBS game shape: CFBD schedules 2011-2025 (world-trial4-proposals.md, table 1).
    vit["FBS"].update({
        "points": 27.8, "one_score": 36.7, "blowout": 21.1, "margin40": 6.7, "margin60": 0.4,
        "home_win": 58.4, "team_margin_sd": 11.2, "luck_sd": 13.0, "home_edge": 2.5,
    })
    nflv = {
        "points": nfl["points_per_team_game"], "plays": nfl["per_team_game"]["plays"],
        "pass_rate": 100 * nfl["rates"]["pass_rate"], "cmp_pct": 100 * nfl["rates"]["completion_pct"],
        "ypa": nfl["rates"]["yards_per_attempt"], "ypc": nfl["rates"]["yards_per_carry"],
        "sack_rate": 100 * nfl["rates"]["sack_rate"], "int_rate": 100 * nfl["rates"]["int_rate"],
        "third_pct": 100 * nfl["rates"]["third_down_conversion"], "one_score": 100 * nfl["share_games_decided_by_8_or_less"],
        "kickoff_tb": 100 * nfl["rates"]["kickoff_touchback_rate"], "punt_gross": nfl["rates"]["punt_gross"],
        "fg_by_distance": {k: 100 * v["pct"] for k, v in nfl["field_goal_pct_by_distance"].items()},
    }
    # Fourth-down go rates (NFL 2021-25) by distance and field zone, all scores.
    go = defaultdict(lambda: [0.0, 0])
    for row in open(f"{REAL_DIR}/nfl_fourth_go.csv").read().splitlines()[1:]:
        togo, yl, score, rate, n = row.split(",")
        togo, yl, rate, n = int(togo), int(yl), float(rate), int(n)
        tb = "1" if togo <= 1 else "2-3" if togo <= 3 else "4-6" if togo <= 6 else "7-10" if togo <= 10 else "11+"
        mid = yl + 5
        fb = field_bucket(mid)
        go[(fb, tb)][0] += rate * n
        go[(fb, tb)][1] += n
    nfl_go = {f"{k[0]}|{k[1]}": r(100 * v[0] / v[1]) for k, v in go.items() if v[1] >= 30}
    star_rows = rates["draft_rate_by_star"]["computed_espn_ratings"]["rows"]
    draft_pos = {}
    for row in open(f"{REAL_DIR}/draft_positions_2000_2025.csv").read().splitlines()[1:]:
        p = row.split(",")
        draft_pos[p[0]] = {k: float(v) for k, v in zip(
            ["QB", "RB", "WR", "TE", "OT", "IOL", "EDGE", "DT", "LB", "CB", "S", "K", "P"], p[3:])}
    talent = {}
    for row in open(f"{REAL_DIR}/talent_by_state.csv").read().splitlines()[1:]:
        p = row.split(",")
        talent[p[0]] = float(p[1])
    box = {k: {"share": 100 * v["share"], "ypc": v["ypc"], "success": 100 * v["success"]} for k, v in eng["runs_by_box"]["all"].items()}
    return {
        "vitals": vit, "nfl": nflv, "nfl_fourth_go": nfl_go,
        "fbs_leaders": cgt.get("fbs_season_leaders"),
        "fbs_records": {
            "_note": "FBS single-season records (NCAA record book). Written from memory, not checked against the book.",
            "pass_yds": 5967, "pass_td": 62, "rush_yds": 2628, "rec_yds": 2060, "sacks": 24, "ints": 14,
        },
        "balance": {
            "titles_15yr_most": [4, 6], "titles_15yr_distinct": [7, 9], "win_pct_corr": 53.6,
            "unbeaten_per_year": 9 / 15, "no_winning_season_15yr": 0,
            "best_15yr": t3["best_15yr_records_2011_2025"], "droughts": t3["p4_droughts"],
            "top_team_margin": [22, 39],
        },
        "draft_by_star": star_rows, "draft_star_share": t3["draft_picks_by_star"],
        "draft_positions": draft_pos, "fbs_signees_by_state": talent,
        "black_fbs_hc_share": 100 * 16 / 134,
        "names": t3["name_repeats"],
        "nfl_box": box, "nfl_pass_rate": nsit["pass_rate"],
        # Section 4 references. Rule facts are exact; the rest are Claude's
        # memory of public reports, marked "to check".
        "calendar": {
            "nfl_games": 285,
            "early_entrants": [58, 98],
            "fbs_signed_early_pct": [80, 90],
            "christmas_games": [2, 3],
            "abroad_2025": 7, "abroad_2026": 9,
            "abroad_by_year": {2005: 1, 2006: 0, 2007: 1, 2008: 2, 2009: 2, 2010: 2, 2011: 2, 2012: 2, 2013: 3, 2014: 3,
                               2015: 3, 2016: 4, 2017: 5, 2018: 3, 2019: 5, 2020: 0, 2021: 2, 2022: 5, 2023: 5, 2024: 5, 2025: 7},
            "huge_share_1_to_3": 50,
            "_notes": {
                "nfl_games": "Rule fact: 17 games for 32 teams (272) plus 13 playoff games.",
                "early_entrants": "Underclassmen granted special eligibility, 2021-2024 NFL announcements (from memory, to check).",
                "fbs_signed_early_pct": "Share of FBS signees who sign in the December period since 2018 (247Sports and On3 reports, from memory, to check).",
                "christmas_games": "NFL games on Christmas Day, 2020-2025 seasons (rule history).",
                "abroad_by_year": "Regular-season NFL games outside the US, including the Bills in Toronto 2008-2013 (NFL International Series history, from memory, to check).",
                "huge_share_1_to_3": "Design target from T8.1, not real data: most seasons have 1 to 3 huge regular-season games, some none.",
            },
        },
        "sources": {
            "college": cgt["source"], "nfl": nfl["source"], "baselines": t3["source"],
            "game_shape": "CFBD schedules 2011-2025 (docs/trials/world-trial4-proposals.md)",
            "black_hc": "CBS Sports 2024-05-16: 16 Black head coaches at 134 FBS schools heading into 2024",
        },
    }


def togo_bucket(t):
    return "1" if t <= 1 else "2-3" if t <= 3 else "4-6" if t <= 6 else "7-10" if t <= 10 else "11+"


def field_bucket(y):
    if y <= 5:
        return "goal (1-5)"
    if y <= 20:
        return "red zone (6-20)"
    if y <= 40:
        return "opp 21-40"
    if y <= 60:
        return "midfield"
    if y <= 80:
        return "own 21-40"
    if y <= 95:
        return "own 6-20"
    return "backed up (own 1-5)"


# ---------------------------------------------------------------------------
# One run
# ---------------------------------------------------------------------------


class Run:
    def __init__(self, path):
        self.path = path
        self.meta = json.load(open(f"{path}/raw/meta.json"))
        self.dash = json.load(open(f"{path}/dash.json"))
        self.cards = json.load(open(f"{path}/cards.json")) if os.path.exists(f"{path}/cards.json") else []
        self.report = open(f"{path}/report.txt").read() if os.path.exists(f"{path}/report.txt") else ""
        self.seed = self.meta["seed"]
        self.first, self.last = self.meta["first"], self.meta["last"]
        self.programs = list(jl(path, "programs"))
        self.prog = {p["id"]: p for p in self.programs}
        self.by_name = {p["name"]: p for p in self.programs}
        self.games = list(jl(path, "games"))
        self.coaches = list(jl(path, "coaches"))
        self.coach_by_id = {c["id"]: c for c in self.coaches}
        self.alumni = list(jl(path, "alumni"))
        self.people = list(jl(path, "people"))
        self.lineups = list(jl(path, "lineups"))
        self.situ = list(jl(path, "situations"))
        self.team_situ = list(jl(path, "team_situations"))
        self.nfl = list(jl(path, "nfl"))
        self.news = list(jl(path, "news"))
        self.years = list(range(self.first, self.last + 1))

    def player_seasons(self):
        return jl(self.path, "player_seasons")


# --- Vitals: every box-score number, by division and year --------------------


def game_metrics(games):
    tg = 2 * len(games)
    if not games:
        return {}
    s = defaultdict(float)
    margins, team_pts = [], []
    for g in games:
        m = abs(g["score"][0] - g["score"][1])
        margins.append(m)
        s["home_wins"] += g["score"][0] > g["score"][1]
        s["ot"] += g["ot"]
        for side in (0, 1):
            t = g["stats"][side]
            team_pts.append(g["score"][side])
            for k in ("plays", "dropbacks", "att", "cmp", "pass_yards", "carries", "rush_yards", "sacks", "ints",
                      "third_att", "third_conv", "fourth_att", "fourth_conv", "turnovers", "penalties", "explosive",
                      "punts", "fg_att", "fg_made", "successes", "epa", "rz_trips", "rz_tds", "drives", "pressures",
                      "first_downs", "penalty_yds", "fumbles_lost"):
                s[k] += t[k]
            s["kr"] += g["kr"][side][0]
            s["kr_yds"] += g["kr"][side][1]
            s["pr"] += g["pr"][side][0]
            s["pr_yds"] += g["pr"][side][1]
    n = len(games)
    return {
        "games": n,
        "points": r(sum(team_pts) / tg),
        "points_sd": r(st.pstdev(team_pts)),
        "plays": r(s["plays"] / tg),
        "pass_rate": pct(s["dropbacks"], s["plays"]),
        "cmp_pct": pct(s["cmp"], s["att"]),
        "ypa": r(s["pass_yards"] / max(1, s["att"]), 2),
        "ypc": r(s["rush_yards"] / max(1, s["carries"]), 2),
        "sack_rate": pct(s["sacks"], s["dropbacks"]),
        "pressure_rate": pct(s["pressures"], s["dropbacks"]),
        "int_rate": pct(s["ints"], s["att"], 2),
        "third_pct": pct(s["third_conv"], s["third_att"]),
        "fourth_go": r(s["fourth_att"] / tg, 2),
        "fourth_pct": pct(s["fourth_conv"], s["fourth_att"]),
        "success_rate": pct(s["successes"], s["plays"]),
        "epa_play": r(s["epa"] / max(1, s["plays"]), 3),
        "turnovers": r(s["turnovers"] / tg, 2),
        "fumbles_lost": r(s["fumbles_lost"] / tg, 2),
        "penalties": r(s["penalties"] / tg, 2),
        "penalty_yds": r(s["penalty_yds"] / tg),
        "explosive": r(s["explosive"] / tg, 2),
        "first_downs": r(s["first_downs"] / tg),
        "punts": r(s["punts"] / tg, 2),
        "fg_att": r(s["fg_att"] / tg, 2),
        "fg_pct": pct(s["fg_made"], s["fg_att"]),
        "rz_td_pct": pct(s["rz_tds"], s["rz_trips"]),
        "pts_per_drive": r(sum(team_pts) / max(1, s["drives"]), 2),
        "kr_avg": r(s["kr_yds"] / max(1, s["kr"])),
        "pr_avg": r(s["pr_yds"] / max(1, s["pr"])),
        "kr_per_game": r(s["kr"] / tg, 2),
        "home_win": pct(s["home_wins"], n),
        "ot": pct(s["ot"], n),
        "margin": r(st.fmean(margins)),
        "one_score": pct(sum(m <= 8 for m in margins), n),
        "blowout": pct(sum(m >= 28 for m in margins), n),
        "margin40": pct(sum(m >= 40 for m in margins), n),
        "margin60": pct(sum(m >= 60 for m in margins), n),
        "shutouts": pct(sum(p == 0 for p in team_pts), len(team_pts)),
        "fifty_plus": pct(sum(p >= 50 for p in team_pts), len(team_pts)),
    }


def vitals(run):
    out = {}
    for div in ("FBS", "FCS", "D2", "JUCO"):
        gs = [g for g in run.games if g["div"] == div]
        if not gs:
            continue
        by = defaultdict(list)
        for g in gs:
            by[g["year"]].append(g)
        out[div] = {"all": game_metrics(gs), "by_year": {y: game_metrics(by[y]) for y in sorted(by)}}
    return out


def histograms(run):
    out = {}
    for div in ("FBS", "FCS", "D2", "JUCO"):
        gs = [g for g in run.games if g["div"] == div]
        if not gs:
            continue
        mh = Counter(min(abs(g["score"][0] - g["score"][1]) // 7, 10) for g in gs)
        ph = Counter(min(p // 7, 12) for g in gs for p in g["score"])
        out[div] = {
            "margin": [pct(mh[i], len(gs)) for i in range(11)],
            "points": [pct(ph[i], 2 * len(gs)) for i in range(13)],
        }
    return out


# --- Competitive balance ------------------------------------------------------


def season_rows(run, div="FBS"):
    rows = []
    for p in run.programs:
        if p["div"] != div:
            continue
        for s in p["seasons"]:
            if s["year"] < run.first or s["g"] == 0:
                continue
            rows.append((p, s))
    return rows


def balance(run):
    fbs = [p for p in run.programs if p["div"] == "FBS"]
    seasons = {(p["id"], s["year"]): s for p in fbs for s in p["seasons"] if s["year"] >= run.first and s["g"]}
    champs = {}
    for (pid, y), s in seasons.items():
        if s["nat"]:
            champs[y] = run.prog[pid]["name"]
    # A program's seasons carry no division, so one that moved up would
    # count its FCS titles here; the news names the level of each title.
    try:
        for n in jl(run.path, "news"):
            m = re.match(r"(.*) wins the FBS national title", n["text"]) if n["kind"] == "NationalTitle" else None
            if m and n["year"] >= run.first:
                champs[n["year"]] = m.group(1)
    except OSError:
        pass
    titles = Counter(champs.values())
    windows = []
    for a in range(run.first, run.last - 13):
        w = [champs.get(y) for y in range(a, a + 15) if champs.get(y)]
        c = Counter(w)
        top = c.most_common(1)[0] if c else ("-", 0)
        windows.append({"from": a, "to": a + 14, "most": top[1], "by": top[0], "distinct": len(c)})
    # Year-to-year win% correlation, unbeaten seasons, spread of team quality.
    by_year = defaultdict(dict)
    for (pid, y), s in seasons.items():
        by_year[y][pid] = s
    yearly = []
    for y in run.years:
        cur = by_year.get(y, {})
        prev = by_year.get(y - 1, {})
        common = [pid for pid in cur if pid in prev]
        wp = lambda s: s["w"] / max(1, s["w"] + s["l"])
        mg = [(s["pf"] - s["pa"]) / s["g"] for s in cur.values()]
        yearly.append({
            "year": y,
            "champ": champs.get(y),
            "win_corr": corr([wp(prev[p]) for p in common], [wp(cur[p]) for p in common]) if common else None,
            "unbeaten": sum(1 for s in cur.values() if s["l"] == 0),
            "top_margin": r(max(mg)) if mg else None,
            "margin35": sum(1 for m in mg if m >= 35),
            "team_margin_sd": r(st.pstdev(mg)) if len(mg) > 1 else None,
            "gini_wins": r(gini([s["w"] for s in cur.values()]), 3) if cur else None,
        })
    # Upsets and calibration from games with lineup strength.
    fg = run.dash["log"]["fbs_games"]
    bins = [(0, 2), (2, 4), (4, 6), (6, 9), (9, 13), (13, 99)]
    gap_rows = []
    for era in ("all",) + tuple(range(run.first, run.last + 1, 10)):
        rows = fg if era == "all" else [g for g in fg if era <= g[0] < era + 10]
        out = []
        for lo, hi in bins:
            sel = [g for g in rows if lo <= abs(g[5] - g[6]) < hi]
            fav_w = sum(1 for g in sel if (g[5] >= g[6]) == (g[3] > g[4]))
            mar = [(g[3] - g[4]) * (1 if g[5] >= g[6] else -1) for g in sel]
            out.append({"gap": f"{lo}-{hi}" if hi < 99 else f"{lo}+", "n": len(sel), "fav_win": pct(fav_w, len(sel)), "fav_margin": r(st.fmean(mar)) if mar else None})
        gap_rows.append({"era": era if era == "all" else f"{era}-{era + 9}", "bins": out})
    # Luck: margin around what lineup strength predicts, and the home edge.
    luck = []
    for y in run.years:
        sel = [g for g in fg if g[0] == y]
        if len(sel) < 30:
            continue
        X = [[g[5] - g[6]] for g in sel]
        Y = [g[3] - g[4] for g in sel]
        beta, r2, icpt = ols(X, Y)
        res = [yy - (icpt + beta[0] * x[0]) for x, yy in zip(X, Y)]
        luck.append({"year": y, "pts_per_strength": r(beta[0], 2), "home_edge": r(icpt, 2), "luck_sd": r(st.pstdev(res)), "r2": r(r2, 3)})
    # Pythagorean luck: actual win% minus the points-based expectation.
    pyth = []
    for (pid, y), s in seasons.items():
        e = s["pf"] ** 2.37 / max(1e-9, s["pf"] ** 2.37 + s["pa"] ** 2.37)
        pyth.append(s["w"] / max(1, s["w"] + s["l"]) - e)
    # Prestige: movement and churn.
    pres = []
    for p in fbs:
        h = dict((y, v) for y, v in p["prestige_history"])
        pres.append({"name": p["name"], "conf": p["conf"], "start": p["prestige_start"], "end": p["prestige"], "change": r(p["prestige"] - p["prestige_start"], 2)})
    pres.sort(key=lambda x: x["change"])
    churn = []
    for y in run.years[5:]:
        a, b = [], []
        for p in fbs:
            h = dict((yy, v) for yy, v in p["prestige_history"])
            if y in h and y - 5 in h:
                a.append(h[y - 5])
                b.append(h[y])
        churn.append({"year": y, "rank_corr_5yr": spearman(a, b)})
    # Power 4 droughts: longest run without a winning season.
    droughts = []
    for p in fbs:
        if p["conf"] not in POWER4 and p["name"] != "Notre Dame":
            continue
        run_len, best, wins = 0, 0, 0
        for s in sorted(p["seasons"], key=lambda s: s["year"]):
            if s["year"] < run.first or not s["g"]:
                continue
            if s["w"] > s["l"]:
                wins += 1
                run_len = 0
            else:
                run_len += 1
                best = max(best, run_len)
        droughts.append({"name": p["name"], "longest": best, "winning_seasons": wins})
    droughts.sort(key=lambda d: -d["longest"])
    best_records = []
    for p in fbs:
        ss = [s for s in p["seasons"] if s["year"] >= run.last - 14 and s["g"]]
        w, l = sum(s["w"] for s in ss), sum(s["l"] for s in ss)
        best_records.append({"name": p["name"], "w": w, "l": l, "pct": r(w / max(1, w + l), 3)})
    best_records.sort(key=lambda x: -x["pct"])
    no_win_15 = sum(1 for p in fbs if not any(s["w"] > s["l"] for s in p["seasons"] if s["year"] >= run.last - 14 and s["g"]))
    return {
        "champs": champs, "titles": titles.most_common(), "windows": windows, "yearly": yearly, "gaps": gap_rows,
        "luck": luck, "pyth_sd": r(100 * st.pstdev(pyth)) if pyth else None,
        "prestige_moves": pres[:10] + pres[-10:], "churn": churn, "droughts": droughts[:12],
        "best_15yr": best_records[:8], "no_winning_season_15yr": no_win_15,
    }


def gini(xs):
    xs = sorted(xs)
    n = len(xs)
    if n == 0 or sum(xs) == 0:
        return 0.0
    cum = sum((i + 1) * x for i, x in enumerate(xs))
    return (2 * cum) / (n * sum(xs)) - (n + 1) / n


def spearman(a, b):
    if len(a) < 3:
        return None
    ra = {i: k for k, i in enumerate(sorted(range(len(a)), key=lambda i: a[i]))}
    rb = {i: k for k, i in enumerate(sorted(range(len(b)), key=lambda i: b[i]))}
    return corr([ra[i] for i in range(len(a))], [rb[i] for i in range(len(b))])


# --- What wins ------------------------------------------------------------------


EXTRA_NAMES = ["rotation players' rating", "rotation players", "starters' college seasons", "kicker", "OC calling", "DC calling"]


def what_wins(run):
    seasons = {(p["id"], s["year"]): s for p in run.programs if p["div"] == "FBS" for s in p["seasons"] if s["g"]}
    rows = []
    pos_names = None
    for lu in run.lineups:
        s = seasons.get((lu["program"], lu["year"]))
        if not s or not lu.get("extras"):
            continue
        pos_names = [p[0] for p in lu["pos"]]
        rows.append((lu["year"], [p[1] for p in lu["pos"]], [p[2] for p in lu["pos"]], lu["extras"], (s["pf"] - s["pa"]) / s["g"]))
    if not rows:
        return {}
    # Positions every team fields (K and P included when present).
    use = [i for i, nm in enumerate(pos_names) if sum(1 for rw in rows if rw[2][i] > 0) > 0.8 * len(rows)]
    means = {i: st.fmean([rw[1][i] for rw in rows if rw[2][i] > 0]) for i in use}

    def fit(sel):
        X = [[(rw[1][i] if rw[2][i] > 0 else means[i]) for i in use] + [rw[3][0], rw[3][2], rw[3][4], rw[3][5]] for rw in sel]
        Y = [rw[4] for rw in sel]
        beta, r2, _ = ols(X, Y)
        names = [pos_names[i] for i in use] + ["depth (rotation rating)", "experience (college seasons)", "OC calling", "DC calling"]
        sds = [st.pstdev([x[j] for x in X]) for j in range(len(names))]
        return {"r2": r(r2, 3), "n": len(sel), "terms": [{"name": n, "per_point": r(b, 3), "per_sd": r(b * sd, 2), "sd": r(sd, 2)} for n, b, sd in zip(names, beta, sds)]}

    out = {"all": fit(rows), "eras": []}
    for a in range(run.first, run.last + 1, 10):
        sel = [rw for rw in rows if a <= rw[0] < a + 10]
        if len(sel) > 50:
            out["eras"].append({"era": f"{a}-{a + 9}", **fit(sel)})
    # Simple correlations of each input with margin, all years.
    Y = [rw[4] for rw in rows]
    out["corr"] = [{"name": pos_names[i], "r": corr([(rw[1][i] if rw[2][i] > 0 else means[i]) for rw in rows], Y)} for i in use]
    out["corr"] += [{"name": EXTRA_NAMES[j], "r": corr([rw[3][j] for rw in rows], Y)} for j in range(6)]
    return out


# --- Situations -----------------------------------------------------------------


def situations(run):
    agg = defaultdict(lambda: defaultdict(lambda: [0.0] * 8))
    yearly = defaultdict(lambda: defaultdict(lambda: [0.0] * 8))
    for row in run.situ:
        a = agg[row["div"]][row["key"]]
        for i, v in enumerate(row["v"]):
            a[i] += v
        if row["div"] == "FBS":
            k = row["key"]
            if k.startswith(("kind|", "fourth|", "blitz|", "coverage|", "play action|", "pressured|", "depth|", "drop", "throw away", "injury|", "personnel|")):
                y = yearly[row["year"]][k]
                for i, v in enumerate(row["v"]):
                    y[i] += v
    games = Counter(g["div"] for g in run.games)
    out = {}
    for div, a in agg.items():
        tg = 2 * games[div]

        def stat(v):
            n = v[0]
            return {"n": int(n), "ypp": r(v[1] / n, 2) if n else None, "epa": r(v[2] / n, 3) if n else None,
                    "success": pct(v[3], n), "first": pct(v[4], n), "td": pct(v[5], n, 2), "to": pct(v[6], n, 2), "extra": pct(v[7], n)}

        d = {}
        # Down and distance.
        dd = []
        for down in "1234":
            for tb in ("1", "2-3", "4-6", "7-10", "11+"):
                run_ = a.get(f"dd|{down}|{tb}|run")
                pas = a.get(f"dd|{down}|{tb}|pass")
                rn, pn = (run_[0] if run_ else 0), (pas[0] if pas else 0)
                if rn + pn < 20:
                    continue
                dd.append({"down": down, "togo": tb, "n": int(rn + pn), "pass_share": pct(pn, rn + pn),
                           "run": stat(run_) if run_ else None, "pass": stat(pas) if pas else None})
        d["dd"] = dd
        # Fourth down decisions.
        fourth = []
        zones = ["backed up (own 1-5)", "own 6-20", "own 21-40", "midfield", "opp 21-40", "red zone (6-20)", "goal (1-5)"]
        for z in zones:
            for tb in ("1", "2-3", "4-6", "7-10", "11+"):
                go, punt, fgk = (a.get(f"fourth|{z}|{tb}|{k}") for k in ("go", "punt", "fg"))
                n = sum(x[0] for x in (go, punt, fgk) if x)
                if n < 10:
                    continue
                fourth.append({"zone": z, "togo": tb, "n": int(n), "go": pct(go[0] if go else 0, n), "punt": pct(punt[0] if punt else 0, n),
                               "fg": pct(fgk[0] if fgk else 0, n), "go_conv": pct((go[4] + go[5]) if go else 0, go[0] if go else 0)})
        d["fourth"] = fourth
        d["field"] = [{"zone": z, **{k: stat(a[f"field|{z}|{k}"]) for k in ("run", "pass") if f"field|{z}|{k}" in a}} for z in zones]
        states = ["down 17+", "down 9-16", "down 1-8", "tied", "up 1-8", "up 9-16", "up 17+"]
        d["state"] = []
        for q in ("Q1", "Q2", "Q3", "Q4", "OT"):
            row = {"q": q}
            for s in states:
                rn = a.get(f"state|{q}|{s}|run", [0] * 8)[0]
                pn = a.get(f"state|{q}|{s}|pass", [0] * 8)[0]
                row[s] = pct(pn, rn + pn) if rn + pn >= 30 else None
            d["state"].append(row)
        d["two_minute"] = {s: pct(a.get(f"two minute|{s}|pass", [0] * 8)[0], a.get(f"two minute|{s}|pass", [0] * 8)[0] + a.get(f"two minute|{s}|run", [0] * 8)[0]) for s in states}

        def group(prefix, kinds=("run", "pass"), top=None, min_n=50):
            rows = defaultdict(dict)
            for k, v in a.items():
                if not k.startswith(prefix + "|"):
                    continue
                parts = k.split("|")
                if parts[-1] in kinds and len(parts) >= 3:
                    rows["|".join(parts[1:-1])][parts[-1]] = v
            res = []
            for name, kv in rows.items():
                n = sum(v[0] for v in kv.values())
                if n < min_n:
                    continue
                res.append({"name": name, "n": int(n), "per_game": r(n / tg, 2), **{kk: stat(vv) for kk, vv in kv.items()}})
            res.sort(key=lambda x: -x["n"])
            return res[:top] if top else res

        d["box"] = group("box")
        d["blitz"] = group("blitz")
        d["coverage"] = group("coverage")
        d["personnel"] = group("personnel")
        d["defense"] = group("defense", top=30)
        d["after_turnover"] = {k: stat(a[f"after turnover|{k}"]) for k in ("run", "pass") if f"after turnover|{k}" in a}
        calls = defaultdict(list)
        for k, v in a.items():
            if k.startswith("call|"):
                _, kind, name = k.split("|", 2)
                calls[kind].append({"name": name, **stat(v), "per_game": r(v[0] / tg, 2)})
        for kind in calls:
            calls[kind].sort(key=lambda x: -x["n"])
        d["calls"] = {k: v[:40] for k, v in calls.items()}
        d["calls_distinct"] = {k: len(v) for k, v in calls.items()}
        d["pass"] = {
            "play_action": {k: stat(a[f"play action|{k}"]) for k in ("true", "false") if f"play action|{k}" in a},
            "pressured": {k: stat(a[f"pressured|{k}"]) for k in ("true", "false") if f"pressured|{k}" in a},
            "depth": [{"depth": k, **stat(a[f"depth|{k}"])} for k in ("behind line", "0-9", "10-19", "20+") if f"depth|{k}" in a],
            "time_to_throw": [{"t": k, **stat(a[f"time to throw|{k}"])} for k in ("none", "<2.0s", "2.0-2.5s", "2.5-3.0s", "3.0-3.5s", "3.5s+") if f"time to throw|{k}" in a],
            "drops_per_game": r(a.get("drop", [0])[0] / tg, 2), "throw_aways_per_game": r(a.get("throw away", [0])[0] / tg, 2),
        }
        pens = []
        for k, v in a.items():
            if k.startswith("penalty|"):
                _, foul, acc = k.split("|")
                pens.append({"foul": foul, "status": acc, "per_game": r(v[0] / tg, 3)})
        pens.sort(key=lambda x: -x["per_game"])
        d["penalties"] = pens
        kicks = []
        for k, v in a.items():
            if k.startswith("kick|"):
                _, kind, rng = k.split("|")
                lo = int(re.match(r"-?\d+", rng).group(0))
                kicks.append({"kind": kind, "from": lo, "n": int(v[0]), "avg": r(v[1] / v[0]) if v[0] else None, "rate": pct(v[7], v[0])})
        kicks.sort(key=lambda x: (x["kind"], x["from"]))
        d["kicks"] = kicks
        fgb = defaultdict(lambda: [0, 0])
        for kk in kicks:
            if kk["kind"] == "fg":
                b = "under30" if kk["from"] < 30 else "30-39" if kk["from"] < 40 else "40-49" if kk["from"] < 50 else "50+"
                fgb[b][0] += kk["n"] * (kk["rate"] or 0) / 100
                fgb[b][1] += kk["n"]
        d["fg_by_distance"] = {b: pct(v[0], v[1]) for b, v in fgb.items()}
        punts = a.get("kind|punt")
        ko = a.get("kind|kickoff")
        d["punt_gross"] = r(punts[1] / punts[0]) if punts and punts[0] else None
        d["punt_tb"] = pct(punts[7], punts[0]) if punts else None
        d["kickoff_tb"] = pct(ko[7], ko[0]) if ko else None
        d["injuries_per_game"] = r(sum(v[0] for k, v in a.items() if k.startswith("injury|")) / max(1, tg), 3)
        runs = a.get("kind|run")
        d["missed_tackles_per_carry"] = r(runs[7] / runs[0], 3) if runs and runs[0] else None
        d["kinds"] = {k.split("|")[1]: {"per_game": r(v[0] / tg, 2), **stat(v)} for k, v in a.items() if k.startswith("kind|")}
        out[div] = d
    # FBS drift: a few play-level rates by year.
    drift = []
    for y in sorted(yearly):
        a = yearly[y]
        g = lambda k: a.get(k, [0] * 8)
        passes = g("kind|pass")[0]
        fourth_n = sum(v[0] for k, v in a.items() if k.startswith("fourth|"))
        fourth_go = sum(v[0] for k, v in a.items() if k.startswith("fourth|") and k.endswith("|go"))
        blitz_t = sum(v[0] for k, v in a.items() if k.startswith("blitz|true"))
        blitz_all = sum(v[0] for k, v in a.items() if k.startswith("blitz|"))
        man = sum(v[0] for k, v in a.items() if k.startswith("coverage|man"))
        cov = sum(v[0] for k, v in a.items() if k.startswith("coverage|"))
        pers11 = sum(v[0] for k, v in a.items() if k.startswith("personnel|11|"))
        pers = sum(v[0] for k, v in a.items() if k.startswith("personnel|"))
        deep = g("depth|20+")[0]
        depth_all = sum(v[0] for k, v in a.items() if k.startswith("depth|"))
        drift.append({
            "year": y,
            "fourth_go_rate": pct(fourth_go, fourth_n),
            "blitz_rate": pct(blitz_t, blitz_all),
            "man_rate": pct(man, cov),
            "play_action_rate": pct(g("play action|true")[0], passes),
            "pressure_rate": pct(g("pressured|true")[0], passes),
            "deep_share": pct(deep, depth_all),
            "drop_rate": pct(g("drop")[0], depth_all),
            "personnel_11": pct(pers11, pers),
            "injuries_per_play": r(1000 * sum(v[0] for k, v in a.items() if k.startswith("injury|")) / max(1, sum(v[0] for k, v in a.items() if k.startswith("kind|"))), 2),
        })
    return {"divs": out, "drift": drift}


def team_profiles(run):
    """Per FBS team-season: pass share, fourth-down aggression, red zone, pressure."""
    t = defaultdict(lambda: defaultdict(lambda: [0.0] * 8))
    for row in run.team_situ:
        a = t[(row["program"], row["year"])][row["key"]]
        for i, v in enumerate(row["v"]):
            a[i] += v
    out = {}
    for key, a in t.items():
        g = lambda k: a.get(k, [0.0] * 8)
        o_run, o_pass = g("o|kind|run")[0], g("o|kind|pass")[0]
        o_rz = [g("o|red zone|run"), g("o|red zone|pass")]
        rz_n = o_rz[0][0] + o_rz[1][0]
        f_go = g("o|fourth|go")[0]
        f_all = f_go + g("o|fourth|punt")[0] + g("o|fourth|fg")[0]
        out[key] = {
            "pass_share": pct(o_pass, o_run + o_pass),
            "o_epa": r((g("o|kind|run")[2] + g("o|kind|pass")[2]) / max(1, o_run + o_pass), 3),
            "d_epa": r((g("d|kind|run")[2] + g("d|kind|pass")[2]) / max(1, g("d|kind|run")[0] + g("d|kind|pass")[0]), 3),
            "fourth_go_rate": pct(f_go, f_all),
            "rz_td_per_play": pct(o_rz[0][5] + o_rz[1][5], rz_n),
            "pressured_allowed": pct(g("o|pressured|true")[0], o_pass),
            "pressure_forced": pct(g("d|pressured|true")[0], g("d|kind|pass")[0]),
            "blitz_rate": pct(g("d|blitz|true")[0], g("d|kind|pass")[0]),
            "third_conv": pct(sum(a[k][4] + a[k][5] for k in a if k.startswith("o|third|")), sum(a[k][0] for k in a if k.startswith("o|third|"))),
        }
    return out


# --- Players: lifecycle, draft, records, names -----------------------------------


def lifecycle(run):
    first_class, last_class = run.first, run.last - 6
    pool = [a for a in run.alumni if first_class <= a["class_of"] <= last_class]
    # The world keeps every drafted player and every four- or five-star, so
    # draft rates divide by the signing-class totals, and the college path
    # columns exist only for four- and five-stars.
    signed = Counter()
    complete = set()
    for y, c in run.meta["signing_classes"]:
        if first_class <= y <= last_class:
            complete.add(y)
            for s, n in enumerate(c):
                signed[s] += n
    stars = defaultdict(lambda: Counter())
    for a in pool:
        if a["class_of"] not in complete:
            continue
        c = stars[a["stars"]]
        c["kept"] += 1
        c["drafted"] += a["pick"] is not None
        c["r1"] += a["pick"] is not None and a["pick"] <= 32
        c["undrafted"] += a["pick"] is None
        c["juco"] += a["juco"]
        c["college"] += bool(a["colleges"])
        c["transfer"] += len(a["colleges"]) > 1
    by_star = []
    for s in sorted(set(stars) | {k for k, v in signed.items() if v}):
        c, n = stars[s], signed[s]
        full = s >= 4
        by_star.append({"stars": s, "n": n, "drafted": pct(c["drafted"], n, 2), "r1": pct(c["r1"], n, 2),
                        "college": pct(c["college"], c["kept"]) if full else None, "left": pct(c["undrafted"], c["kept"]) if full else None,
                        "juco": pct(c["juco"], c["kept"]) if full else None, "transfer": pct(c["transfer"], c["kept"]) if full else None})
    drafted = [a for a in run.alumni if a["pick"] and a["year"] > run.first]
    star_share = Counter(min(a["stars"], 5) if a["stars"] >= 2 else 0 for a in drafted)
    star_share_r1 = Counter(min(a["stars"], 5) if a["stars"] >= 2 else 0 for a in drafted if a["pick"] <= 32)
    # Transfers: moves between divisions, and up or down in prestige.
    moves = Counter()
    for a in run.alumni:
        cs = a["college_ids"]
        for x, y in zip(cs, cs[1:]):
            dx, dy = run.prog[x]["div"], run.prog[y]["div"]
            if dx == dy:
                px, py = run.prog[x]["prestige"], run.prog[y]["prestige"]
                moves[f"{dx} to {dy}, " + ("up" if py > px + 0.05 else "down" if py < px - 0.05 else "sideways")] += 1
            else:
                moves[f"{dx} to {dy}"] += 1
    # Development curves from every season record we have (college players and kept alumni).
    curves = defaultdict(list)
    group = {"QB": "QB", "RB": "RB", "WR": "WR", "TE": "TE", "OT": "OL", "IOL": "OL", "EDGE": "DL", "DT": "DL", "LB": "LB", "CB": "DB", "S": "DB", "K": "K/P", "P": "K/P"}
    for src in (run.people, run.alumni):
        for q in src:
            det = q.get("detail")
            if not det:
                continue
            for s in det["seasons"]:
                curves[(group.get(s["pos"], s["pos"]), s["age"])].append(s["end"])
    dev = defaultdict(list)
    for (g, age), xs in sorted(curves.items()):
        if len(xs) >= 20:
            dev[g].append({"age": age, "n": len(xs), "q": quant(xs)})
    # Year-to-year change by age (growth, slumps and breakouts).
    jumps = defaultdict(list)
    for src in (run.people, run.alumni):
        for q in src:
            det = q.get("detail")
            if not det:
                continue
            for s in det["seasons"]:
                if s["start"] is not None and s["end"] is not None:
                    jumps[s["age"]].append(s["end"] - s["start"])
    growth = [{"age": age, "n": len(xs), "q": quant(xs, (0.05, 0.25, 0.5, 0.75, 0.95)), "slump": pct(sum(1 for x in xs if x < -1), len(xs)), "breakout": pct(sum(1 for x in xs if x > 6), len(xs))}
              for age, xs in sorted(jumps.items()) if len(xs) >= 50]
    # Potential against what drafted players reached by draft day.
    gaps = [a["potential"] - a["overall"] for a in drafted]
    reach = {"n": len(gaps), "q": quant(gaps), "within3": pct(sum(1 for g in gaps if g <= 3), len(gaps))}
    booms = [a for a in drafted if a["pick"] <= 32 and a["stars"] <= 2]
    busts = [a for a in run.alumni if a["stars"] == 5 and a["pick"] is None and first_class <= a["class_of"] <= last_class]
    # Who leaves football (notes give the reason).
    reasons = Counter()
    for a in pool:
        if a["left"]:
            for n in a["notes"]:
                reasons[re.sub(r"\d+(\.\d+)?", "#", n)[:70]] += 1
    gpa = defaultdict(list)
    for a in pool:
        if a["gpa"] is not None:
            gpa[a["stars"]].append(a["gpa"])
    current = Counter((q["stage"], q["college_seasons"]) for q in run.people)
    return {
        "by_star": by_star, "drafted_star_share": {k: pct(v, len(drafted)) for k, v in star_share.items()},
        "drafted_star_share_r1": {k: pct(v, sum(star_share_r1.values())) for k, v in star_share_r1.items()},
        "moves": moves.most_common(), "dev": dev, "growth": growth, "reach": reach,
        "booms": [[a["year"], a["pick"], a["name"], a["pos"], a["stars"], " > ".join(a["colleges"])] for a in booms][:40],
        "busts_n": len(busts), "busts_share": pct(len(busts), sum(1 for a in pool if a["stars"] == 5)),
        "busts": [[a["class_of"], a["name"], a["pos"], " > ".join(a["colleges"]), a["left"], r(a["overall"]), r(a["potential"]), "; ".join(a["notes"][:2])] for a in busts][:30],
        "left_reasons": reasons.most_common(15), "gpa": {s: quant(v) for s, v in sorted(gpa.items())},
        "current": [[k[0], k[1], v] for k, v in sorted(current.items())],
        "pool_classes": [first_class, last_class],
    }


def recruiting(run):
    sc = [(y, c) for y, c in run.meta["signing_classes"] if y >= run.first]
    signee = dict(run.meta["fbs_signees_by_state"])
    tot = sum(signee.values()) or 1
    # Distance from home to the first college.
    centers = {}
    geo = open(os.path.join(REPO, "crates/afm-engine/src/world/geo.rs")).read()
    for m in re.finditer(r'\("([A-Z]{2})", (-?[\d.]+), (-?[\d.]+)\)', geo):
        centers[m.group(1)] = (float(m.group(2)), float(m.group(3)))

    def miles(a, b):
        (la1, lo1), (la2, lo2) = centers[a], centers[b]
        p = math.pi / 180
        h = math.sin((la2 - la1) * p / 2) ** 2 + math.cos(la1 * p) * math.cos(la2 * p) * math.sin((lo2 - lo1) * p / 2) ** 2
        return 3959 * 2 * math.asin(math.sqrt(h))

    dist = defaultdict(list)
    instate = defaultdict(lambda: [0, 0])
    for a in run.alumni:
        if not a["college_ids"] or a["class_of"] < run.first:
            continue
        p = run.prog[a["college_ids"][0]]
        if p["div"] != "FBS" or a["state"] not in centers or p["state"] not in centers:
            continue
        d = miles(a["state"], p["state"])
        dist[a["stars"]].append(d)
        instate[a["stars"]][0] += a["state"] == p["state"]
        instate[a["stars"]][1] += 1
    nil = []
    for p in run.programs:
        if p["div"] != "FBS":
            continue
        ss = [s for s in p["seasons"] if s["year"] > run.last - 5 and s["g"]]
        if ss:
            nil.append([p["name"], p["conf"], round(p["nil_budget"] / 1e6, 2), r(sum(s["w"] for s in ss) / max(1, sum(s["w"] + s["l"] for s in ss)) * 100), r(p["prestige"], 2)])
    # The rosters as they stand at the end: who is on them and from where.
    ros = defaultdict(Counter)
    for q in run.people:
        if q["stage"] == "College" and q.get("college") is not None:
            p = run.prog[q["college"]]
            c = ros[q["college"]]
            c["n"] += 1
            c[q["pos"]] += 1
            c["home"] += q.get("state") == p["state"]
    roster_div = {}
    for div in ("FBS", "FCS", "D2", "JUCO"):
        ids = [i for i, p in run.prog.items() if p["div"] == div and ros[i]["n"]]
        if ids:
            roster_div[div] = {"teams": len(ids), "no_k": pct(sum(1 for i in ids if ros[i]["K"] == 0), len(ids)), "no_p": pct(sum(1 for i in ids if ros[i]["P"] == 0), len(ids)),
                               "neither": pct(sum(1 for i in ids if ros[i]["K"] + ros[i]["P"] == 0), len(ids)),
                               "per_team": {pos: r(sum(ros[i][pos] for i in ids) / len(ids), 2) for pos in ("QB", "RB", "WR", "TE", "OT", "IOL", "EDGE", "DT", "LB", "CB", "S", "K", "P")},
                               "home_share": quant([100 * ros[i]["home"] / ros[i]["n"] for i in ids])}
    home = sorted(([run.prog[i]["name"], run.prog[i]["state"], run.prog[i]["conf"], ros[i]["n"], pct(ros[i]["home"], ros[i]["n"]), ros[i]["K"], ros[i]["P"], ros[i]["QB"]]
                   for i in ros if run.prog[i]["div"] == "FBS" and ros[i]["n"]), key=lambda x: -x[4])
    return {
        "signing_classes": sc, "fbs_signees_share": {k: pct(v, tot) for k, v in signee.items()},
        "distance": {s: {"n": len(v), "q": quant(v)} for s, v in sorted(dist.items())},
        "in_state": {s: pct(v[0], v[1]) for s, v in sorted(instate.items())},
        "nil": nil, "nil_corr": corr([x[2] for x in nil], [x[3] for x in nil]),
        "rosters": roster_div, "roster_home": home,
    }


def coaches(run):
    stints = []
    for c in run.coaches:
        for s in c["career"]:
            at = s["at"]
            if at.get("div") == "FBS" and s["role"] == "head coach":
                to = s["to"] if s["to"] is not None else run.last + 1
                stints.append({"coach": c, "school": at["college"], "from": s["from"], "to": to, "w": s["w"], "l": s["l"], "ended": s["ended"], "titles": s["titles"]})
    hires = Counter(s["from"] for s in stints if s["from"] >= run.first)
    exits = defaultdict(Counter)
    for s in stints:
        if s["ended"] != "current" and s["to"] >= run.first:
            exits[s["to"]][s["ended"]] += 1
    done = [s for s in stints if s["ended"] != "current" and s["from"] >= run.first]
    tenure = Counter(min(s["to"] - s["from"], 15) for s in done)
    # Stint records count only recorded seasons, so judge stints by games played.
    ability = [[s["coach"]["name"], s["school"], s["coach"]["hidden"]["hc_value"], s["to"] - s["from"], pct(s["w"], s["w"] + s["l"]), s["titles"], s["ended"]]
               for s in stints if s["w"] + s["l"] >= 30]
    by_end = defaultdict(list)
    for s in done:
        if s["w"] + s["l"]:
            by_end[s["ended"]].append(100 * s["w"] / (s["w"] + s["l"]))
    stubborn = [[s["coach"]["name"], s["school"], s["from"], s["to"], s["w"], s["l"], s["ended"]] for s in stints
                if s["w"] + s["l"] >= 55 and s["w"] / (s["w"] + s["l"]) < 0.4]
    proteges = Counter()
    for c in run.coaches:
        if c["mentor"] is not None and any(s["at"].get("div") == "FBS" and s["role"] == "head coach" for s in c["career"]):
            proteges[c["mentor"]] += 1
    trees = [[run.coach_by_id[m]["name"] if m in run.coach_by_id else f"coach {m}", n] for m, n in proteges.most_common(15)]
    hire_age = [s["coach"]["age"] - (run.last + 1 - s["from"]) for s in stints if s["from"] >= run.first]
    units = defaultdict(list)
    for c in run.coaches:
        for g, read, seasons, moved in c["units"]:
            if seasons >= 8:
                units[g].append(read)
    div = {int(y): v for y, v in run.dash["diversity"].items()}
    eth_share = [{"year": y, **{k: pct(n, sum(v.values())) for k, n in v.items()}} for y, v in sorted(div.items())]
    return {
        "hires": sorted(hires.items()), "exits": {y: dict(c) for y, c in sorted(exits.items())},
        "tenure": [tenure[i] for i in range(16)], "tenure_median": st.median([s["to"] - s["from"] for s in done]) if done else None,
        "ability": ability, "ability_corr": corr([a[2] for a in ability], [a[4] for a in ability]),
        "win_by_end": {k: {"n": len(v), "q": quant(v)} for k, v in by_end.items()},
        "stubborn": stubborn[:30], "trees": trees, "hire_age": quant(hire_age), "units": {g: quant(v) for g, v in units.items()},
        "ethnicity": eth_share, "ex_pro": pct(sum(1 for s in stints if s["coach"]["ex_pro"] and s["ended"] == "current"), sum(1 for s in stints if s["ended"] == "current")),
    }


def draft(run):
    cols = run.dash["draft_cols"]
    rows = [dict(zip(cols, d)) for d in run.dash["draft"]]
    bands = {"pick_1": lambda p: p == 1, "picks_1_10": lambda p: p <= 10, "round_1": lambda p: p <= 32, "all": lambda p: True}
    pos = {}
    for b, f in bands.items():
        sel = [d for d in rows if f(d["pick"])]
        c = Counter(d["pos"] for d in sel)
        pos[b] = {k: pct(v, len(sel)) for k, v in c.items()}
    rnd = lambda p: min(7, (p - 1) // 32 + 1)
    by_round = []
    for k in range(1, 8):
        sel = [d for d in rows if rnd(d["pick"]) == k]
        by_round.append({"round": k, "n": len(sel), "overall": quant([d["overall"] for d in sel]), "potential": quant([d["potential"] for d in sel])})
    path = Counter()
    conf = Counter()
    yearly = []
    for y in sorted({d["year"] for d in rows}):
        sel = [d for d in rows if d["year"] == y]
        divs = Counter(re.findall(r"\((FBS|FCS|D2|JUCO)\)$", d["path"])[0] if re.findall(r"\((FBS|FCS|D2|JUCO)\)$", d["path"]) else "?" for d in sel)
        qb_top = sum(1 for d in sel if d["pick"] <= 10 and d["pos"] == "QB")
        yearly.append({"year": y, "n": len(sel), "fcs_share": pct(divs.get("FCS", 0), len(sel)), "qb_top10": qb_top,
                       "r1_overall": r(st.fmean([d["overall"] for d in sel if d["pick"] <= 32])) if any(d["pick"] <= 32 for d in sel) else None})
        path.update(divs)
    for d in rows:
        last = re.sub(r" \((FBS|FCS|D2|JUCO)\)$", "", d["path"].split(" -> ")[-1])
        p = run.by_name.get(last)
        conf[p["conf"] if p else "?"] += 1
    team = defaultdict(list)
    for d in rows:
        if d["pick"] <= 64:
            team[d["team"]].append(d["overall"] - (90 - d["pick"] * 0.25))
    fo = [[t["name"], t["draft_skill"], r(st.fmean(team[t["name"]]), 2) if team[t["name"]] else None, t["coaching"], t["owner"]] for t in run.nfl]
    return {"positions": pos, "by_round": by_round, "path": dict(path), "conf": conf.most_common(20), "yearly": yearly,
            "front_office": fo, "front_office_corr": corr([f[1] for f in fo if f[2] is not None], [f[2] for f in fo if f[2] is not None]),
            "first_rounders": [[d["year"], d["pick"], d["name"], d["pos"], d["stars"], d["path"], d["team"], d["overall"], d["potential"]] for d in rows if d["pick"] <= 32]}


def records(run):
    stats = ["pass_yds", "pass_td", "rush_yds", "rec_yds", "sacks", "ints", "tackles", "tfl", "rec", "rush_td"]
    top = defaultdict(lambda: defaultdict(list))
    thresholds = defaultdict(Counter)
    qual = defaultdict(lambda: defaultdict(list))
    for s in run.player_seasons():
        if run.prog[s["program"]]["div"] != "FBS":
            continue
        y = s["year"]
        for k in stats:
            top[y][k].append((s[k], s["name"], s["pos"], run.prog[s["program"]]["name"], s["stars"]))
        thresholds[y]["passers_4000"] += s["pass_yds"] >= 4000
        thresholds[y]["passers_3000"] += s["pass_yds"] >= 3000
        thresholds[y]["rushers_1000"] += s["rush_yds"] >= 1000
        thresholds[y]["receivers_1000"] += s["rec_yds"] >= 1000
        thresholds[y]["sacks_10"] += s["sacks"] >= 10
        if s["att"] >= 200:
            qual[y]["cmp"].append(100 * s["cmp"] / s["att"])
            qual[y]["ypa"].append(s["pass_yds"] / s["att"])
        if s["rush"] >= 150:
            qual[y]["ypc"].append(s["rush_yds"] / s["rush"])
    leaders = {}
    best = {}
    record_falls = Counter()
    for y in sorted(top):
        leaders[y] = {}
        for k in stats:
            xs = sorted(top[y][k], key=lambda t: -t[0])[:5]
            leaders[y][k] = [[r(x[0]), x[1], x[2], x[3], x[4]] for x in xs]
            if xs and (k not in best or xs[0][0] > best[k][0]):
                if k in best:
                    record_falls[k] += 1
                best[k] = (xs[0][0], xs[0][1], xs[0][3], y)
    return {
        "leaders": leaders, "best": {k: [r(v[0]), v[1], v[2], v[3]] for k, v in best.items()}, "record_falls": dict(record_falls),
        "thresholds": {y: dict(c) for y, c in sorted(thresholds.items())},
        "qualified": {y: {k: quant(v) for k, v in d.items()} for y, d in sorted(qual.items())},
    }


def names(run):
    strip = lambda n: re.sub(r" (Jr\.|II|III|IV)$", "", n).lower()
    college = [strip(q["name"]) for q in run.people if q["stage"] == "College"]
    everyone = [strip(q["name"]) for q in run.people] + [strip(a["name"]) for a in run.alumni]
    rnd = random.Random(run.seed)
    shares = {}
    for size in (1000, 5000, 10000, 22843):
        if len(college) >= size:
            vals = []
            for _ in range(5):
                s = rnd.sample(college, size)
                c = Counter(s)
                vals.append(sum(1 for x in s if c[x] > 1) / size)
            shares[size] = r(100 * st.fmean(vals), 2)
    c = Counter(college)
    allc = Counter(everyone)
    full = [q["name"] for q in run.people] + [a["name"] for a in run.alumni]
    firsts = Counter(n.split(" ")[0] for n in full)
    lasts = Counter(re.sub(r" (Jr\.|II|III|IV)$", "", n).split(" ")[-1] for n in full)
    return {
        "college_pool": len(college), "share_by_pool": shares, "most_common_college": c.most_common(12),
        "everyone_pool": len(everyone), "everyone_share": pct(sum(1 for x in everyone if allc[x] > 1), len(everyone)),
        "suffix": pct(sum(1 for n in full if re.search(r" (Jr\.|II|III|IV)$", n)), len(full)),
        "hyphen": pct(sum(1 for n in full if "-" in n.split(" ")[-1] or "-" in n), len(full)),
        "initials": pct(sum(1 for n in full if re.match(r"^[A-Z]{2} ", n)), len(full)),
        "distinct_first": len(firsts), "distinct_last": len(lasts),
        "top_first": firsts.most_common(15), "top_last": lasts.most_common(15),
        "coach_sample": [c["name"] for c in sorted(run.coaches, key=lambda c: -c["rep"])[:12]],
    }


# --- Anomaly finder --------------------------------------------------------------


def anomalies(run, real, recs, coach, life):
    out = []
    rec = real["fbs_records"]
    labels = {"pass_yds": "passing yards", "pass_td": "passing TDs", "rush_yds": "rushing yards", "rec_yds": "receiving yards", "sacks": "sacks", "ints": "interceptions"}
    for y, d in recs["leaders"].items():
        for k, lab in labels.items():
            for v, name, pos, school, stars in d.get(k, [])[:3]:
                if v is not None and v > rec[k]:
                    out.append({"kind": "Stat line beyond the real record", "year": y, "title": f"{name} ({pos}, {school}): {v:g} {lab}",
                                "why": f"The FBS single-season record is {rec[k]:g}. One season past it can happen; several a year means the engine inflates {lab}."})
    offense = {"QB", "RB", "WR", "TE", "OT", "IOL", "K", "P"}
    for s in run.player_seasons():
        if run.prog[s["program"]]["div"] != "FBS" or s["pos"] is None:
            continue
        school = run.prog[s["program"]]["name"]
        if s["att"] >= 100 and s["pos"] != "QB":
            out.append({"kind": "Player in the wrong role", "year": s["year"], "title": f"{s['name']} ({s['pos']}, {school}): {s['att']} passes, {s['pass_td']} TDs",
                        "why": "Listed at another position but played quarterback most of the season. The depth chart moves a player when his rating there beats the listed players by 4 points; his listed position stays the same, and a lineman or back out-throwing the quarterbacks points at a thin quarterback room or a player whose skills don't match his position."})
        if s["sacks"] >= 5 and s["pos"] in offense:
            out.append({"kind": "Player in the wrong role", "year": s["year"], "title": f"{s['name']} ({s['pos']}, {school}): {s['sacks']:g} sacks",
                        "why": "An offensive player with five or more sacks is playing both ways as a pass rusher. FBS two-way starters are very rare in real football."})
    for s in coach["stubborn"]:
        out.append({"kind": "Coach kept while losing", "year": s[3], "title": f"{s[0]} at {s[1]}, {s[2]}-{s[3]}: {s[4]}-{s[5]}",
                    "why": f"About five or more recorded seasons below a .400 win rate; the stint ended as '{s[6]}'. Real FBS schools rarely keep a coach that long at that record."})
    for p in run.programs:
        if p["div"] != "FBS":
            continue
        h = sorted(p["prestige_history"])
        for (y0, v0), (y1, v1) in zip(h, h[5:]):
            if v1 - v0 >= 0.25:
                out.append({"kind": "Fast rise", "year": y1, "title": f"{p['name']}: prestige {v0:.2f} to {v1:.2f} in five years",
                            "why": "A jump this size in five years is rare in real football (Indiana 2024-25 is the modern example). Check the story for its cause."})
                break
        for s in p["seasons"]:
            if s["g"] and s["year"] >= run.first and (s["pf"] - s["pa"]) / s["g"] >= 40:
                out.append({"kind": "Team-season far beyond real", "year": s["year"], "title": f"{p['name']} {s['year']}: {s['w']}-{s['l']}, {(s['pf'] - s['pa']) / s['g']:.1f} average margin",
                            "why": "The best real FBS team-seasons average about +22 to +39. An average margin of 40 or more has not happened in modern FBS."})
    for row in life["busts"]:
        if not row[4]:
            out.append({"kind": "Five-star never drafted", "year": row[0], "title": f"{row[1]} ({row[2]}): {row[3]}",
                        "why": f"Rating {row[5]} against hidden potential {row[6]}. Notes: {row[7] or 'none'}. Real five-stars are drafted about 62% of the time."})
    out.sort(key=lambda x: (x["kind"], x["year"]))
    return out


# --- Explorer tables ---------------------------------------------------------------


def explorer(run, profiles):
    lu = {(x["program"], x["year"]): x for x in run.lineups}
    rows = []
    cols = ["year", "program", "div", "conf", "w", "l", "win_pct", "pf_g", "pa_g", "margin_g", "strength", "nat", "conf_title", "coach", "coach_hc_value",
            "coach_scheme", "cmp_pct", "ypa", "ypc", "sacks_g", "sacks_allowed_g", "takeaways_g", "turnovers_g", "plays_g",
            "pass_share", "o_epa", "d_epa", "fourth_go_rate", "pressured_allowed", "pressure_forced", "blitz_rate", "third_conv",
            "QB", "RB", "WR", "TE", "OT", "IOL", "EDGE", "DT", "LB", "CB", "S", "depth", "experience", "oc_call", "dc_call", "prestige"]
    for p in run.programs:
        if p["div"] not in ("FBS", "FCS"):
            continue
        ph = dict((y, v) for y, v in p["prestige_history"])
        for s in p["seasons"]:
            if s["year"] < run.first or not s["g"]:
                continue
            g = s["g"]
            c = run.coach_by_id.get(s["coach"]) if s["coach"] is not None else None
            prof = profiles.get((p["id"], s["year"]), {})
            l = lu.get((p["id"], s["year"]))
            posv = {x[0]: x[1] for x in l["pos"]} if l else {}
            ex = l["extras"] if l and l.get("extras") else [None] * 6
            rows.append([s["year"], p["name"], p["div"], p["conf"], s["w"], s["l"], pct(s["w"], s["w"] + s["l"]), r(s["pf"] / g), r(s["pa"] / g), r((s["pf"] - s["pa"]) / g),
                         s["strength"], s["nat"], s["conf"], c["name"] if c else None, c["hidden"]["hc_value"] if c else None, c["hidden"]["scheme"] if c else None,
                         pct(s["cmp"], s["att"]), r(s["pass"] / max(1, s["att"]), 2), r(s["rush"] / max(1, s["carries"]), 2), r(s["sacks"] / g, 2), r(s["sacks_allowed"] / g, 2),
                         r(s["takeaways"] / g, 2), r(s["to"] / g, 2), r(s["plays"] / g, 1),
                         prof.get("pass_share"), prof.get("o_epa"), prof.get("d_epa"), prof.get("fourth_go_rate"), prof.get("pressured_allowed"), prof.get("pressure_forced"), prof.get("blitz_rate"), prof.get("third_conv"),
                         *[posv.get(k) for k in ("QB", "RB", "WR", "TE", "OT", "IOL", "EDGE", "DT", "LB", "CB", "S")], ex[0], ex[2], ex[4], ex[5], ph.get(s["year"])])
    pcols = ["year", "pick", "name", "pos", "stars", "eth", "state", "path", "team", "overall", "potential", "juco", "class_of", "left"]
    players = []
    for a in run.alumni:
        if a["pick"] is None and a["stars"] < 5:
            continue
        if a["year"] <= run.first:
            continue
        path = " > ".join(a["colleges"])
        players.append([a["year"], a["pick"], a["name"], a["pos"], a["stars"], a["eth"], a["state"], path, a["team"], r(a["overall"]), r(a["potential"]), a["juco"], a["class_of"], a["left"]])
    ccols = ["name", "eth", "age", "school", "from", "to", "w", "l", "win_pct", "titles", "ended", "hc_value", "scheme", "game_mgmt", "motivation", "recruiting", "development", "sportsmanship", "ex_pro", "came_up"]
    crow = []
    for c in run.coaches:
        for s in c["career"]:
            if s["role"] == "head coach" and s["at"].get("div") in ("FBS", "FCS") and (s["to"] or run.last + 1) >= run.first:
                h = c["hidden"]
                crow.append([c["name"], c["eth"], c["age"], s["at"]["college"], s["from"], s["to"], s["w"], s["l"], pct(s["w"], s["w"] + s["l"]), s["titles"], s["ended"],
                             h["hc_value"], h["scheme"], h["game_mgmt"], h["motivation"], h["recruiting"], r(st.fmean(h["teaching"])), h.get("sportsmanship"), c["ex_pro"], c["came_up"]])
    return {"seasons": {"cols": cols, "rows": rows}, "players": {"cols": pcols, "rows": players}, "coaches": {"cols": ccols, "rows": crow}}


def program_pages(run):
    out = {}
    for p in run.programs:
        if p["div"] != "FBS":
            continue
        out[p["name"]] = {
            "conf": p["conf"], "state": p["state"], "city": p["city"], "stadium": p["stadium"], "tradition": p["tradition"], "facilities": p["facilities"],
            "fans": p["fans"], "prestige_start": p["prestige_start"], "prestige": p["prestige"], "titles": p["titles"], "conf_titles": p["conf_titles"],
            "nil": round(p["nil_budget"] / 1e6, 2), "story": [s for s in p["story"] if s[0] >= run.first][-40:],
            "grades": list(zip(p["grade_names"], p["grade_history"][-1][1])) if p["grade_history"] else [],
            "pipeline": sorted(p["pipeline"], key=lambda x: -x[1])[:8],
            "picks": draft_rounds(p, run),
        }
    return out


def draft_rounds(p, run):
    c = Counter()
    for y, rd in p["draft_picks"]:
        if y >= run.first:
            c[rd] += 1
    return [c[k] for k in range(1, 8)]


def player_pages(run):
    pages = []
    for c in run.cards:
        pages.append({"source": "card", **c})
    for a in run.alumni:
        if a["pick"] and a["pick"] <= 32 and a.get("detail") and a["year"] > run.first:
            d = a["detail"]
            pages.append({"source": "first round", "tag": f"Round 1 of the {a['year']} draft", "name": a["name"], "pos": a["pos"], "stars": a["stars"],
                          "state": a["state"], "hs": a["hs"], "path": " -> ".join(a["colleges"]), "pick": a["pick"], "year": a["year"], "class_of": a["class_of"],
                          "team": a["team"], "overall": a["overall"], "potential": a["potential"], "ethnicity": a["eth"], "gpa": a["gpa"], "juco": a["juco"],
                          "background": a["background"], "notes": a["notes"], "personality": d["personality"], "bloom": d["bloom"], "injury": d["injury"],
                          "attributes": d["attrs"], "seasons": d["seasons"], "positions": d["positions"]})
    return pages


PORTAL_WINDOWS = (
    ("Players at programs with a new head coach", "coach"),
    ("The spring transfer window", "spring"),
    ("The transfer window", "winter"),
)


def portal_windows(years, events):
    """The portal's windows (coach change, winter, spring), each with its
    entries by day. The year's log keeps one row per window and day, so a
    December date can come twice (the coach-change and winter windows
    overlap), and a football year holds April of one cycle and December and
    January of the next. So the rows are split by window, in the order the
    year logged them (each window's days in a run, late entrants after), and
    merged by date within each window. A window is labelled with the season
    it follows: the spring window belongs to the season before."""
    wins = []
    for e in events:
        if e.get("rule") != "NCAA: transfer window":
            continue
        kind = next((k for start, k in PORTAL_WINDOWS if e["text"].startswith(start)), None)
        if not kind:
            continue
        m = re.search(r"until \w+ (\w+ \d+, \d+)", e["text"])
        close = datetime.strptime(m.group(1), "%b %d, %Y").date().isoformat() if m else e["date"]
        wins.append({"kind": kind, "season": e["year"] - (kind == "spring"), "open": e["date"], "close": close,
                     "days": Counter()})
    wins.sort(key=lambda w: w["open"])
    for y in years:
        cur, prev = None, None
        for d, n in y["portal_by_day"]:
            # A new run of days is a new window's rows.
            if prev is not None and d <= prev:
                cur = None
            prev = d
            latest = next((w for w in reversed(wins) if w["open"] <= d), None)
            if latest is None:
                continue
            # Past the end of one window and into a later one.
            if cur is None or (latest["open"] > cur["open"] and d > cur["close"]):
                cur = latest
            cur["days"][d] += n
    out = []
    for w in wins:
        total = sum(w["days"].values())
        out.append({
            "kind": w["kind"], "season": w["season"], "open": w["open"], "close": w["close"], "entries": total,
            "first_day_pct": pct(w["days"].get(w["open"], 0), total),
            "busiest_pct": pct(max(w["days"].values(), default=0), total),
            "by_day": sorted([d, n] for d, n in w["days"].items()),
        })
    return out


def calendar_view(run, real):
    """Section 4: each football year's counters, the stakes per team, the
    portal's days and a year at a glance from the dated events."""
    try:
        years = list(jl(run.path, "calendar_years"))
    except FileNotFoundError:
        return None
    try:
        events = list(jl(run.path, "calendar_events"))
    except FileNotFoundError:
        events = []
    try:
        champs = list(jl(run.path, "nfl_champions"))
    except FileNotFoundError:
        champs = []
    n_fbs = sum(1 for p in run.programs if p.get("div") == "FBS") or 134
    windows = portal_windows(years, events)
    rows = []
    for y in years:
        div = y["signed_by_division"]
        fbs_total = div[0][0] + div[0][1]
        huge = Counter(min(n, 4) for _, n in y["huge_by_team"])
        huge[0] = max(0, n_fbs - sum(huge.values()))
        nfl_huge = Counter(min(n, 4) for _, n in y["nfl_huge_by_team"])
        # Entries by date, the windows' rows for the same day added up.
        by_date = Counter()
        for d, n in y["portal_by_day"]:
            by_date[d] += n
        portal = sorted([d, n] for d, n in by_date.items())
        # First-day share of each window after this season (coach change,
        # winter, and the next spring's).
        first_day = {w["kind"]: w["first_day_pct"] for w in windows if w["season"] == y["year"]}
        rows.append({
            "year": y["year"],
            "commits_by_month": y["commits_by_month"], "commits": y["commits"], "decommits": y["decommits"], "flips": y["flips"],
            "fbs_early_pct": pct(div[0][0], fbs_total), "fcs_early_pct": pct(div[1][0], div[1][0] + div[1][1]),
            "signed_by_division": div, "early_enrollees": y["early_enrollees"],
            "visits": [y["official_visits"], y["unofficial_visits"], y["drop_ins"]],
            "declared": y["declared"], "withdrew": y["withdrew"],
            "portal": [y["portal_entries"], y["portal_landed"], y["portal_returned"]], "portal_days": len(portal),
            "portal_first_day": first_day, "portal_by_day": portal,
            "opt_outs": y["opt_outs"], "bowl_practice_gain": y["bowl_practice_gain"],
            "four_game_redshirts": y["four_game_redshirts"], "redshirt_tricks": y["redshirt_tricks"],
            "spring": [y["spring_notices"], y["spring_breakouts"], y["spring_fizzles"]],
            "weather": [y["snow"], y["rain"], y["freezing"], y["lightning"], y["moved"], y["cancelled"]],
            "heat_days": y["heat_days"],
            "hs": [y["hs_games"], y["hs_forfeits"], y["hs_no_contests"], y["hs_grade_outs"], y["hs_all_state"]],
            "ineligible": [y["spring_ineligible"], y["fall_ineligible"]],
            "invites": [y["combine_invites"], y["all_star_invites"]],
            "college_huge": [huge.get(k, 0) for k in range(5)], "nfl_huge": [nfl_huge.get(k, 0) for k in range(5)],
            "college_huge_share_1_3": pct(sum(huge.get(k, 0) for k in (1, 2, 3)), n_fbs),
            "nfl_huge_share_1_3": pct(sum(nfl_huge.get(k, 0) for k in (1, 2, 3)), 32),
            "rule_changes": y["rule_changes"], "nfl_games": y["nfl_games"],
            "whispers": y["whispers"], "sit_out": y["sit_out_transfers"],
        })
    # Events: loud counts per year, Continue stops, and the last year at a glance.
    loud = defaultdict(Counter)
    stops = defaultdict(set)
    kinds = defaultdict(Counter)
    for e in events:
        loud[e["year"]][e["loud"]] += 1
        kinds[e["year"]][f'{e["level"]}|{e["kind"]}'] += 1
        # Continue stops on Big and Huge moments and on decisions (T8).
        if e["loud"] in ("Huge", "Big") or e["decision"]:
            stops[e["year"]].add(e["date"])
    by_year = {y: {"loud": dict(loud[y]), "stop_days": len(stops[y])} for y in loud}
    last = max(loud) if loud else None
    glance = defaultdict(list)
    abroad, christmas, flexed, anniv = Counter(), Counter(), Counter(), Counter()
    for e in events:
        # Games abroad come from the schedule release line ("..., 5 abroad."):
        # single games are small events, which the export leaves out.
        if e["level"] == "Nfl" and "schedule is released" in e["text"]:
            # A year with none has no count in the line.
            m = re.search(r"(\d+) abroad", e["text"])
            abroad[e["year"]] = int(m.group(1)) if m else 0
        if e["level"] == "Nfl" and e["date"][5:] == "12-25" and e["kind"] == "Game" and " beat " in e["text"]:
            christmas[e["year"]] += 1
        if e["rule"] == "NFL: flexible scheduling":
            flexed[e["year"]] += 1
        if e["kind"] == "Anniversary":
            anniv[e["year"]] += 1
        if e["year"] == last and e["loud"] in ("Huge", "Big"):
            glance[e["date"][:7]].append([e["date"], e["loud"], e["level"], e["text"]])
    return {
        "years": rows, "events": by_year, "n_fbs": n_fbs, "portal_windows": windows,
        "kinds_last": kinds[last].most_common(40) if last else [],
        "glance_year": last, "glance": {m: v for m, v in sorted(glance.items())},
        "abroad": dict(abroad), "christmas": dict(christmas), "flexed": dict(flexed), "anniversaries": dict(anniv),
        "nfl_champions": [[c["year"], c["team"]] for c in champs],
    }


def slumps(run):
    """Every big slump (the trial rule): a season where the rating fell 2 or
    more points, or the play fell 5 or more below the rating with a slump.
    The reason comes from that season's development events."""
    out = []
    seen = set()
    for src in (run.people, run.alumni):
        for q in src:
            det = q.get("detail")
            if not det:
                continue
            key = (q["name"], q.get("class_of"))
            if key in seen:
                continue
            seen.add(key)
            for s in det["seasons"]:
                if s["start"] is None or s["end"] is None:
                    continue
                drop = s["end"] - s["start"]
                form = (s["perf"] - s["start"]) if s.get("perf") is not None else 0
                ev = s.get("ev", [])
                if drop > -2 and not (form <= -5 and "slump" in ev):
                    continue
                why = []
                if "majorInjury" in ev:
                    why.append("major injury")
                elif "moderateInjury" in ev:
                    why.append("moderate injury")
                elif "minorInjury" in ev:
                    why.append("minor injury")
                if s.get("missed"):
                    why.append(f'{s["missed"]} game{"" if s["missed"] == 1 else "s"} missed')
                for k, t in (("slump", "a form slump"), ("lostSeason", "a lost season"), ("suspended", "suspended or ineligible"),
                             ("dismissed", "dismissed"), ("stagnation", "stagnation"), ("bodyStalled", "body stopped developing"),
                             ("poorCoaching", "teaching that cost ability")):
                    if k in ev:
                        why.append(t)
                if "firstYear" in ev:
                    dip = s.get("dip")
                    why.append(f"first year at this level (dip {dip:.2})" if dip is not None else "first year at this level")
                if s["age"] >= 30:
                    why.append(f'age {s["age"]} decline')
                if s.get("coach") is not None and s["coach"] < 7:
                    why.append(f'poor coaching ({s["coach"]})')
                if s["role"] in ("redshirt", "unsigned", "practiceSquad") and drop <= -2:
                    why.append("no snaps")
                year = (q.get("class_of") or 0) + s["age"] - 18 if q.get("class_of") else None
                out.append([year, q["name"], s["pos"], s["age"], s["level"], s["role"], s["start"], s["end"], r(drop),
                            s.get("perf"), r(form), ", ".join(why) or "no event recorded (the yearly step)"])
    out.sort(key=lambda x: (x[8], x[10]))
    return {"n": len(out), "rating_drops": sum(1 for x in out if x[8] <= -2), "form_slumps": sum(1 for x in out if x[10] <= -5),
            "rows": out[:400]}


# --- Section 5: the living NFL -------------------------------------------------

# Top-5 average salary by position as a share of the cap (2025, the same
# shares the engine's market starts from, `money.rs` `top5_share`).
NFL_TOP5 = {"QB": 23.4, "EDGE": 15.8, "WR": 13.6, "DT": 11.7, "CB": 10.8, "OT": 9.85, "S": 8.1, "IOL": 7.7,
            "LB": 7.5, "RB": 7.4, "TE": 6.3, "K": 2.3, "P": 1.5, "LS": 0.7}


def opt_jl(run, name):
    try:
        return list(jl(run.path, name))
    except FileNotFoundError:
        return []


def nfl_view(run, real):
    """The living NFL: engine numbers from NFL games, the market's books by
    year, cap use, trades and comp picks, pay by position, who holds the
    rosters, careers by draft round, honors, records, the Hall of Fame,
    front offices and agents."""
    games = opt_jl(run, "nfl_games")
    if not games and not opt_jl(run, "nfl_market"):
        return None
    out = {}
    by = defaultdict(list)
    for g in games:
        by[g["year"]].append(g)
    out["vitals"] = {"all": game_metrics(games), "by_year": {y: game_metrics(by[y]) for y in sorted(by)}}
    mh = Counter(min(abs(g["score"][0] - g["score"][1]) // 7, 10) for g in games)
    out["margin_hist"] = [pct(mh[i], len(games)) for i in range(11)] if games else []
    out["market"] = opt_jl(run, "nfl_market")
    trades, comp = Counter(), defaultdict(Counter)
    trade_rows = []
    for t in opt_jl(run, "nfl_trades"):
        if t.get("comp"):
            comp[t["year"]][t["kind"]] += 1
            continue
        m = re.search(r"(\d{4})$", t["date"])
        y = int(m.group(1)) if m else None
        trades[y] += 1
        trade_rows.append([t["date"], t["buyer"], t["seller"], len(t.get("picks") or []), t["text"]])
    out["trades"] = {"by_year": sorted([y, n] for y, n in trades.items() if y), "rows": trade_rows[-150:]}
    out["comp"] = sorted([y, c.get("FreeAgency", 0), c.get("Supplemental", 0), c.get("Diversity", 0)] for y, c in comp.items())
    # Pros: pay, roster makeup, ages and careers.
    pros = opt_jl(run, "pros")
    last = run.last
    pay = defaultdict(list)
    biggest = []
    makeup = defaultdict(Counter)
    ages = defaultdict(list)
    by_round = defaultdict(list)
    tags_by_pos = Counter()
    for q in pros:
        d = q.get("deal")
        gen = any("generated veteran" in n for n in q.get("notes", []))
        origin = "pathway" if q.get("ipp") else "generated" if gen else "drafted" if q.get("draft") else "undrafted"
        if q["active"] and d and q["status"] in ("Roster", "Injured", "Reserve") and d["last"] >= last:
            pay[q["pos"]].append(d["apy"])
            biggest.append([q["name"], q["pos"], d["team"], q["age"], d["years"], round(d["apy"] / 1e6, 2), round(d["guaranteed"] / 1e6, 1), d["signed"], d["kind"]])
        seasons = q.get("seasons", [])
        if q.get("tags"):
            tags_by_pos[q["pos"]] += q["tags"]
        last_season = max((s["year"] for s in seasons), default=None)
        for s in seasons:
            if s.get("team") and (s["g"] > 0 or s.get("ps_weeks", 0) > 0):
                makeup[s["year"]][origin] += 1
                if s["g"] > 0 and last_season is not None:
                    ages[s["year"]].append(q["age"] - ((last if q["active"] else last_season) - s["year"]))
        dr = q.get("draft")
        if dr and dr[0] <= last - 8:
            played = sum(1 for s in seasons if s["g"] > 0)
            by_round[dr[1]].append(played)
    # The cap this year: each club's payroll plus its room (carryover included), the median club.
    clubs = opt_jl(run, "nfl_clubs")
    cap = st.median([c["payroll"] + c["space"] - c["carryover"] for c in clubs if c["roster"] > 0]) if clubs else None
    out["cap"] = cap
    out["pay"] = []
    for pos, v in sorted(pay.items()):
        v.sort(reverse=True)
        top5 = st.fmean(v[:5]) if v else None
        out["pay"].append([pos, len(v), round(top5 / 1e6, 2) if top5 else None, round(st.median(v) / 1e6, 2) if v else None,
                           pct(top5, cap, 2) if top5 and cap else None, NFL_TOP5.get(pos)])
    biggest.sort(key=lambda x: -x[5])
    out["biggest"] = biggest[:25]
    out["makeup"] = [[y, makeup[y]["drafted"], makeup[y]["undrafted"], makeup[y]["generated"], makeup[y]["pathway"]] for y in sorted(makeup)]
    out["age"] = [[y, r(st.fmean(a))] for y, a in sorted(ages.items()) if a]
    out["career_by_round"] = [[k, len(v), r(st.fmean(v)), pct(sum(x >= 4 for x in v), len(v))] for k, v in sorted(by_round.items())]
    out["tags_by_pos"] = sorted(tags_by_pos.items(), key=lambda x: -x[1])
    # Honors, records, the Hall.
    awards, hall, broken, records = [], [], [], []
    for h_ in opt_jl(run, "nfl_honors"):
        k = h_["kind"]
        if k == "award":
            awards.append([h_["year"], h_["award"], h_["name"], h_["team"]])
        elif k == "hall":
            hall.append([h_["year"], h_["name"], h_["pos"], h_["why"]])
        elif k == "broken":
            broken.append([h_["year"], h_["stat"], h_["holder"], h_["value"]])
        elif k == "record":
            records.append([h_["stat"], "career" if h_["career"] else "season", h_["value"], h_["holder"], h_["year"], "world" if h_["world"] else "real"])
    out["awards"] = sorted(awards)
    out["hall"] = sorted(hall)
    out["broken"] = sorted(broken)
    out["records"] = records
    # Front offices and agents at the end of the world.
    out["offices"] = [[c["team"], c["mode"], c["why"], c["fans"], c["gm"]["name"], c["gm"]["since"], c["gm"]["skill"],
                       "classic" if c["gm"]["classic_chart"] else "surplus", c["gm"]["fixation"], c["owner"]["meddling"],
                       c["owner"]["pet"], c["gm_changes"], round(c["payroll"] / 1e6, 1), round(c["dead"] / 1e6, 1)]
                      for c in clubs if c["roster"] > 0]
    agents = opt_jl(run, "agents")
    agencies = Counter()
    for a in agents:
        agencies[a["agency"]] += len(a["clients"]) if isinstance(a["clients"], list) else a["clients"]
    out["agencies"] = agencies.most_common(15)
    out["agents"] = len(agents)
    return out


# --- Tendencies (2026-10-19): how often players try things, by level ---------

# Mirrors `Tend::ALL` in crates/afm-engine/src/tendencies.rs: key, label, the
# positions that own it.
TENDS = [
    ("bouncesRuns", "Bounces runs", {"RB"}),
    ("contactWillingness", "Contact willingness", {"QB", "RB", "WR", "TE", "LB", "CB", "S"}),
    ("reachesBallOut", "Reaches the ball out", {"QB", "RB", "WR", "TE", "LB", "CB", "S"}),
    ("patientOrDownhill", "Patient or downhill", {"RB"}),
    ("scrambles", "Scrambles", {"QB"}),
    ("gunslinger", "Gunslinger or checkdown", {"QB"}),
    ("holdsForBigPlay", "Holds for the big play", {"QB"}),
    ("returnerRisk", "Returner risk", {"RB", "WR", "CB", "S"}),
    ("rushesUpfield", "Rushes upfield or keeps contain", {"EDGE", "DT"}),
    ("timesTheSnap", "Times the snap", {"EDGE", "DT"}),
    ("getsHandsUp", "Gets his hands up", {"EDGE", "DT"}),
    ("goesForStrip", "Goes for the strip", {"EDGE", "DT", "LB", "CB", "S"}),
    ("bigHit", "Big hit or sure tackle", {"EDGE", "DT", "LB", "CB", "S"}),
    ("fliesDownhill", "Flies downhill or reads first", {"LB", "S"}),
    ("jumpsRoutes", "Jumps routes", {"LB", "CB", "S"}),
]
# The designed spread among regulars (`tendencies::level_sd`).
TEND_SD = {"NFL": 3.2, "FBS": 3.8, "FCS": 4.0, "D2": 4.2, "JUCO": 4.2, "HS": 4.8}
TEND_LEVELS = ["HS", "JUCO", "D2", "FCS", "FBS", "NFL"]


def tend_view(run):
    """Tendencies in the world: the spread by level against the designed one,
    how far coaching moved them, the drift over the recorded future seasons,
    whether a player's numbers show in his season stats, and the coaches'
    styles."""
    by_year = opt_jl(run, "tend_by_year")
    rows = opt_jl(run, "tendencies")
    if not by_year and not rows:
        return None
    out = {"keys": [[k, l, sorted(o)] for k, l, o in TENDS], "design_sd": TEND_SD}
    levels = [l for l in TEND_LEVELS if by_year and l in by_year[-1]["groups"]]
    out["levels"] = levels
    out["drift"] = {l: {k: [[y["year"], y["groups"][l][k]["mean"], y["groups"][l][k]["sd"]] for y in by_year if l in y["groups"]] for k, _, _ in TENDS} for l in levels}
    out["last"] = by_year[-1]["groups"] if by_year else {}
    out["years"] = [y["year"] for y in by_year]
    # Coached habits: share of owned numbers coaching moved 2+ points down or up.
    idx = {k: i for i, (k, _, _) in enumerate(TENDS)}
    coached = []
    for l in levels:
        sel = [r for r in rows if r["lvl"] == l]
        line = [l, len(sel)]
        for k, _, own in TENDS:
            i = idx[k]
            d = [r["v"][i] - r["was"][i] for r in sel if r["pos"] in own]
            line.append(pct(sum(1 for x in d if x <= -2), len(d)) if d else None)
        coached.append(line)
    out["coached"] = coached
    # Do the numbers show in the stats? Last recorded season, players with volume.
    last = run.last
    ps = {"FBS": {}, "NFL": {}}
    for s in run.player_seasons():
        if s["year"] == last:
            ps["FBS"][s["id"]] = s
    # Pro season lines carry fewer stats (no sacks taken, returns or tackles
    # for loss): those checks run at FBS only.
    for p in opt_jl(run, "pros"):
        for s in p["seasons"]:
            if s["year"] == last and s["team"] and s["g"]:
                ps["NFL"][p["id"]] = {"g": s["g"], "cmp": s["pass"][0], "att": s["pass"][1], "pass_yds": s["pass"][2], "int": s["pass"][4],
                                      "rush": s["rush"][0], "tackles": s["def"][0], "sacks": s["def"][1], "ints": s["def"][2], "pbus": s["def"][3]}
    checks = [
        ("QB", "scrambles", "QB runs per pass or run", lambda s: s["rush"] / max(1, s["att"] + s["rush"]), lambda s: s["att"] >= 150),
        ("QB", "gunslinger", "Interceptions per pass", lambda s: s["int"] / max(1, s["att"]), lambda s: s["att"] >= 150),
        ("QB", "gunslinger", "Yards per completion", lambda s: s["pass_yds"] / max(1, s["cmp"]), lambda s: s["att"] >= 150),
        ("QB", "holdsForBigPlay", "Sacks per dropback", lambda s: s["sacked"] / max(1, s["att"] + s["sacked"]), lambda s: s["att"] >= 150),
        ("RB", "returnerRisk", "Kick returns a game", lambda s: s["kr"] / max(1, s["g"]), lambda s: s["kr"] >= 5),
        ("WR", "returnerRisk", "Kick returns a game", lambda s: s["kr"] / max(1, s["g"]), lambda s: s["kr"] >= 5),
        ("CB", "jumpsRoutes", "Interceptions a game", lambda s: s["ints"] / max(1, s["g"]), lambda s: s["g"] >= 8),
        ("S", "jumpsRoutes", "Interceptions a game", lambda s: s["ints"] / max(1, s["g"]), lambda s: s["g"] >= 8),
        ("LB", "fliesDownhill", "Tackles for loss per tackle", lambda s: s["tfl"] / max(1, s["tackles"]), lambda s: s["tackles"] >= 30),
        ("S", "fliesDownhill", "Tackles for loss per tackle", lambda s: s["tfl"] / max(1, s["tackles"]), lambda s: s["tackles"] >= 30),
        ("EDGE", "getsHandsUp", "Passes batted or defensed a game", lambda s: s["pbus"] / max(1, s["g"]), lambda s: s["g"] >= 8),
        ("DT", "getsHandsUp", "Passes batted or defensed a game", lambda s: s["pbus"] / max(1, s["g"]), lambda s: s["g"] >= 8),
        ("EDGE", "rushesUpfield", "Sacks a game", lambda s: s["sacks"] / max(1, s["g"]), lambda s: s["g"] >= 8),
    ]
    res = []
    for pos, k, stat, f, ok in checks:
        i = idx[k]
        for l in ("FBS", "NFL"):
            pairs = []
            for row in rows:
                line = ps[l].get(row["id"]) if row["lvl"] == l and row["pos"] == pos else None
                if line is None:
                    continue
                try:
                    if ok(line):
                        pairs.append((row["v"][i], f(line)))
                except KeyError:
                    break
            if len(pairs) < 20:
                continue
            pairs.sort()
            fifth = max(1, len(pairs) // 5)
            lo = st.mean(y for _, y in pairs[:fifth])
            hi = st.mean(y for _, y in pairs[-fifth:])
            res.append([l, pos, next(t[1] for t in TENDS if t[0] == k), stat, len(pairs), r(corr([x for x, _ in pairs], [y for _, y in pairs]), 2), r(lo, 3), r(hi, 3)])
    out["results"] = res
    # Coaches' styles: head coaches in a job now, by level.
    sty = defaultdict(list)
    for c in run.coaches:
        st_ = c.get("style")
        cur = next((j for j in c.get("career", []) if j.get("ended") == "current"), None)
        if not st_ or not cur or cur.get("role") != "head coach":
            continue
        at = cur.get("at") or {}
        lvl = "NFL" if at.get("nfl") else at.get("div") or ("HS" if at.get("hs") else None)
        if lvl:
            sty[lvl].append(st_)
    names = ["risk", "freedom", "physical", "ball_hunting", "mould"]
    out["styles"] = [[l, len(v)] + [f"{r(st.mean(x[n] for x in v))} ± {r(st.pstdev([x[n] for x in v]))}" for n in names] for l, v in sty.items() if v]
    return out


def build(run, real):
    print(f"seed {run.seed}: {len(run.games)} games, {len(run.alumni)} alumni, {len(run.people)} people", file=sys.stderr)
    profiles = team_profiles(run)
    recs = records(run)
    coach = coaches(run)
    life = lifecycle(run)
    data = {
        "seed": run.seed, "first": run.first, "last": run.last, "files": run.meta["files"],
        "vitals": vitals(run), "hist": histograms(run), "balance": balance(run), "wins": what_wins(run),
        "situ": situations(run), "life": life, "recruit": recruiting(run), "coaches": coach, "draft": draft(run),
        "records": recs, "names": names(run), "anomalies": anomalies(run, real, recs, coach, life),
        "explorer": explorer(run, profiles), "programs": program_pages(run), "players": player_pages(run),
        "calendar": calendar_view(run, real), "slumps": slumps(run), "nfl": nfl_view(run, real),
        "tend": tend_view(run),
    }
    return data


def main():
    out_dir, runs = sys.argv[1], sys.argv[2:]
    os.makedirs(out_dir, exist_ok=True)
    real = real_values()
    json.dump(real, open(f"{out_dir}/real.json", "w"), separators=(",", ":"))
    seeds = []
    for path in runs:
        run = Run(path)
        data = build(run, real)
        name = f"seed-{run.seed}.json"
        # The player pages go in their own file: a 50-year world's pages
        # alone are about 14 MB, and an artifact file may hold 16 MB.
        pages = f"seed-{run.seed}-players.json"
        dump = lambda obj, f: json.dump(obj, open(f"{out_dir}/{f}", "w"), separators=(",", ":"),
                                        default=lambda o: dict(o) if isinstance(o, Counter) else str(o))
        dump(data.pop("players", []), pages)
        dump(data, name)
        seeds.append({"seed": run.seed, "file": name, "players": pages, "first": run.first, "last": run.last})
        for f in (name, pages):
            print(f"wrote {out_dir}/{f}: {os.path.getsize(f'{out_dir}/{f}') / 1e6:.1f} MB", file=sys.stderr)
    json.dump({"seeds": seeds}, open(f"{out_dir}/index.json", "w"))


if __name__ == "__main__":
    main()
