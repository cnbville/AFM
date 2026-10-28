"""Real college recruiting, transfers, returning production and head coaches (world reader lane R3).

Reads sportsdataverse-data release files, saved in DIR:
  cfb_recruits_YEAR.parquet 2002-2026 (ESPN-rated signees by class), cfb_team_info_YEAR.parquet 2004-2025 (school, state, location),
  cfb_rosters_YEAR.csv 2004-2025 (cfbfastR-data rosters: hometown and its latitude and longitude),
  cfb_team_portal_YEAR.parquet 2015-2026, cfb_returning_production_YEAR.parquet 2004-2025,
  coach_tendencies_YEAR.parquet 2004-2025 (espn_cfb_coach_tendencies: the head coach of each FBS team season),
  cfb_schedules_YEAR.parquet 2001-2025 (team records).

Usage: python3 college_people.py DIR OUTDIR > college_people.json
Writes OUTDIR/college_recruiting_classes.csv beside the JSON.
Development checks only, never in-game tuning.
"""
import json, re, sys
import numpy as np
import pandas as pd

D, OUT = sys.argv[1], sys.argv[2]


def r(x, n=3):
    try:
        x = float(x)
    except (TypeError, ValueError):
        return None
    return None if np.isnan(x) else round(x, n)


def pct(s, ps=(10, 25, 50, 75, 90), n=1):
    s = pd.Series(s).dropna()
    return {**{f"p{p}": r(np.percentile(s, p), n) for p in ps}, "n": int(len(s))} if len(s) else None


def norm(n):
    return re.sub(r"[^a-z]", "", str(n).lower().replace(" jr", "").replace(" iii", "").replace(" ii", ""))


info = pd.concat([pd.read_parquet(f"{D}/cfb_team_info_{y}.parquet", columns=["team_id", "school", "state", "latitude", "longitude", "classification"]).assign(season=y)
                  for y in range(2004, 2026)])
info["team_id"] = info.team_id.astype(str)
last_info = info.sort_values("season").drop_duplicates("team_id", keep="last").set_index("team_id")
out = {"source": "sportsdataverse-data releases (CollegeFootballData and ESPN): cfb_recruits, cfb_team_info, cfbfastR-data rosters, cfb_team_portal, cfb_returning_production, espn_cfb_coach_tendencies, cfb_schedules"}

# --- recruiting classes ---
rc = pd.concat([pd.read_parquet(f"{D}/cfb_recruits_{y}.parquet") for y in range(2002, 2027)])
rc["season"] = rc.season.astype(int)
rc["stars"] = pd.to_numeric(rc.stars, errors="coerce")
rc["grade"] = pd.to_numeric(rc.grade, errors="coerce")
rc["team_id"] = rc.team_id.astype(str)
rc["school"] = rc.team_id.map(last_info.school)
cls = rc.groupby(["season", "school"]).agg(signees=("recruit_id", "size"), five=("stars", lambda s: int((s == 5).sum())),
                                           four=("stars", lambda s: int((s == 4).sum())), three=("stars", lambda s: int((s == 3).sum())),
                                           avg_grade=("grade", "mean")).reset_index()
cls["avg_grade"] = cls.avg_grade.round(2)
cls.to_csv(f"{OUT}/college_recruiting_classes.csv", index=False)
five = rc[rc.stars == 5]
win = {}
for a in range(2002, 2026, 5):
    v = five[five.season.between(a, a + 4)]
    if len(v):
        top = v.school.value_counts()
        win[f"{a}-{min(a + 4, 2026)}"] = {"five_stars": int(len(v)), "programs_with_one": int(top.size), "top10_share": r(top.head(10).sum() / len(v), 3),
                                          "top5": [[k, int(n)] for k, n in top.head(5).items()]}
out["recruiting"] = {"years": "classes 2002-2026, ESPN star ratings (as data/real-world/rates.json)",
                     "file": "college_recruiting_classes.csv (each school's class: signees, 5-, 4- and 3-stars, average ESPN grade)",
                     "stars_by_class": {int(y): {int(k): int(n) for k, n in v.stars.value_counts().sort_index().items() if pd.notna(k)} for y, v in rc.groupby("season")},
                     "five_star_spread_by_5yr": win,
                     "blue_chips_per_class_top_school": {int(y): int(v.groupby("school").stars.apply(lambda s: (s >= 4).sum()).max()) for y, v in rc.groupby("season")}}

# distance from home and in-state share, matching signees to rosters by name and school (the talent_by_state.csv method)
ro = pd.concat([pd.read_csv(f"{D}/cfb_rosters_{y}.csv", usecols=["first_name", "last_name", "team", "home_state", "home_latitude", "home_longitude", "season"], low_memory=False)
                for y in range(2004, 2026)])
ro["key"] = (ro.first_name.fillna("") + ro.last_name.fillna("")).map(norm) + "|" + ro.team.astype(str)
ro = ro[ro.home_latitude.notna()].drop_duplicates("key")
rc["key"] = rc.player_name.map(norm) + "|" + rc.school.astype(str)
m = rc[rc.season.between(2004, 2025)].merge(ro[["key", "home_state", "home_latitude", "home_longitude"]], on="key", how="left")
m = m.join(last_info[["state", "latitude", "longitude"]], on="team_id")
lat1, lon1, lat2, lon2 = (np.radians(m[c].astype(float)) for c in ["home_latitude", "home_longitude", "latitude", "longitude"])
m["miles"] = 3958.8 * 2 * np.arcsin(np.sqrt(np.sin((lat2 - lat1) / 2) ** 2 + np.cos(lat1) * np.cos(lat2) * np.sin((lon2 - lon1) / 2) ** 2))
mm = m[m.miles.notna()]
out["recruiting"]["distance_from_home"] = {
    "years": "classes 2004-2025", "matched_share": r(m.miles.notna().mean(), 3),
    "by_stars": {int(k): {"miles": pct(v.miles, n=0), "within_250_miles": r((v.miles <= 250).mean(), 3), "in_state": r((v.home_state == v.state).mean(), 3)}
                 for k, v in mm.groupby("stars") if pd.notna(k)}}

# --- bodies by year in school (FBS rosters) ---
GROUP = {"QB": "QB", "RB": "RB", "FB": "RB", "WR": "WR", "TE": "TE", "OL": "OL", "OT": "OL", "OG": "OL", "C": "OL", "G": "OL",
         "DL": "DL", "DE": "DL", "DT": "DL", "NT": "DL", "LB": "LB", "OLB": "LB", "ILB": "LB", "DB": "DB", "CB": "DB", "S": "DB",
         "PK": "K", "K": "K", "P": "P", "LS": "LS"}
bo = []
for y in range(2019, 2026):
    x = pd.read_csv(f"{D}/cfb_rosters_{y}.csv", usecols=["team", "height", "weight", "year", "position"], low_memory=False)
    fb = set(info[(info.season == y) & (info.classification == "fbs")].school)
    bo.append(x[x.team.isin(fb)].assign(season=y))
bo = pd.concat(bo)
bo["grp"] = bo.position.map(lambda p: GROUP.get(str(p).upper()))
bo = bo[bo.grp.notna() & bo.height.between(60, 84) & bo.weight.between(140, 420) & bo.year.between(1, 5)]
out["fbs_bodies_by_year_in_school"] = {"years": "2019-2025 FBS rosters (CollegeFootballData); height in inches, weight in pounds; year 1 = first year on the roster, redshirts included",
                                       "caution": "CFBD codes many unknown players as year 4 (and before 2019 most years as the season itself), so year 4 is a mix; years 1-3 are clean",
                                       "by_group_and_year": {f"{g}|{int(yr)}": {"n": int(len(v)), "ht_p50": r(v.height.median(), 1), "ht_p90": r(v.height.quantile(0.9), 1),
                                                                                "wt_p10": r(v.weight.quantile(0.1), 0), "wt_p50": r(v.weight.median(), 0), "wt_p90": r(v.weight.quantile(0.9), 0)}
                                                             for (g, yr), v in bo.groupby(["grp", "year"])}}

# --- the transfer portal ---
pt = pd.concat([pd.read_parquet(f"{D}/cfb_team_portal_{y}.parquet") for y in range(2015, 2027)])
pt["season"] = pd.to_numeric(pt.season, errors="coerce").astype(int)
for c in ["transfers_in_n", "transfers_out_n", "roster_n", "portal_share"]:
    pt[c] = pd.to_numeric(pt[c], errors="coerce")
out["portal"] = {"years": "2015-2026 (team level)", "by_season": {int(y): {"teams": int(len(v)), "in_per_team": pct(v.transfers_in_n, (10, 50, 90)),
                                                                           "out_per_team": pct(v.transfers_out_n, (10, 50, 90)), "total_in": int(v.transfers_in_n.sum()),
                                                                           "portal_share_of_roster": pct(v.portal_share * 100, (10, 50, 90))}
                                                                 for y, v in pt.groupby("season")}}

# --- returning production against the season ---
sc = pd.concat([pd.read_parquet(f"{D}/cfb_schedules_{y}.parquet", columns=["season", "home_id", "away_id", "home_points", "away_points", "home_division", "away_division"])
                for y in range(2003, 2026)])
sc = sc[sc.home_points.notna() & (sc.home_division == "fbs") & (sc.away_division == "fbs")]
rows = [(s, str(h), hp > ap) for s, h, hp, ap in zip(sc.season, sc.home_id, sc.home_points, sc.away_points)] + \
       [(s, str(a), ap > hp) for s, a, hp, ap in zip(sc.season, sc.away_id, sc.home_points, sc.away_points)]
wp = pd.DataFrame(rows, columns=["season", "team_id", "w"]).groupby(["season", "team_id"]).w.mean().reset_index()
rp = pd.concat([pd.read_parquet(f"{D}/cfb_returning_production_{y}.parquet") for y in range(2004, 2026)])
rp["season"] = rp.season.astype(int)
rp["team_id"] = rp.team_id.astype(str)
rp["overall_returning"] = pd.to_numeric(rp.overall_returning, errors="coerce")
j = rp.merge(wp, on=["season", "team_id"]).merge(wp.assign(season=wp.season + 1).rename(columns={"w": "w_prev"}), on=["season", "team_id"])
j["change"] = j.w - j.w_prev
out["returning_production"] = {"years": "2004-2025, FBS teams, share of last year's production back (CFBD-style, from ESPN data)",
                               "overall_returning": pct(j.overall_returning * 100, (10, 25, 50, 75, 90)),
                               "corr_with_win_pct_change": r(j[["overall_returning", "change"]].corr().iloc[0, 1], 3),
                               "win_pct_change_by_band": {str(k): r(v.change.mean(), 3) for k, v in j.groupby(pd.cut(j.overall_returning, [0, 0.4, 0.55, 0.7, 0.85, 1.01]), observed=True)}}

# --- head coaches of FBS teams ---
ct = pd.concat([pd.read_parquet(f"{D}/coach_tendencies_{y}.parquet", columns=["season", "pos_team_id", "coach", "role", "games"]) for y in range(2004, 2026)])
ct = ct[ct.role == "HC"].sort_values("games").drop_duplicates(["season", "pos_team_id"], keep="last")
ct["season"] = ct.season.astype(int)
ct = ct.sort_values(["pos_team_id", "season"])
ct["prev"] = ct.groupby("pos_team_id").coach.shift(1)
ct["prev_season"] = ct.groupby("pos_team_id").season.shift(1)
link = ct[ct.prev_season == ct.season - 1]
ten = []
for (t, c), v in ct.groupby(["pos_team_id", "coach"]):
    ys = sorted(v.season)
    run = [ys[0]]
    for a in ys[1:]:
        if a == run[-1] + 1:
            run.append(a)
        else:
            ten.append((t, c, run[0], run[-1]))
            run = [a]
    ten.append((t, c, run[0], run[-1]))
tn = pd.DataFrame(ten, columns=["team", "coach", "start", "end"])
tn["years"] = tn.end - tn.start + 1
started = tn[tn.start.between(2005, 2019)]
closed = tn[(tn.start >= 2005) & (tn.end <= 2024)]
moves = ct.groupby("coach").pos_team_id.nunique()
out["fbs_head_coaches"] = {"years": "2004-2025, the head coach ESPN credits with most of each FBS team season's games (about 115-125 teams a year)",
                           "changes_by_season": {int(y): {"teams_linked": int(len(v)), "new_head_coach": int((v.coach != v.prev).sum()),
                                                          "share": r((v.coach != v.prev).mean(), 3)} for y, v in link.groupby("season")},
                           "completed_tenure_years": pct(closed.years, (10, 25, 50, 75, 90), 1),
                           "share_lasting_at_least": {int(k): r((started.years >= k).mean(), 3) for k in (2, 3, 4, 5, 6, 8, 10)},
                           "tenures_started_2005_2019": int(len(started)),
                           "coaches_with_two_plus_fbs_jobs_share": r((moves >= 2).mean(), 3)}
json.dump(out, sys.stdout, indent=1, default=str)
