"""Real baselines for world trial 3's findings (research area 3).

Reads (all open data):
- sportsdataverse cfb_schedules 2001-2025 (CFBD scores):
  https://github.com/sportsdataverse/sportsdataverse-data/releases/tag/cfb_schedules
- data/real-world/colleges.csv (2026 conferences)
- sportsdataverse cfb_recruits 2010-2021 (ESPN star ratings of signees):
  https://github.com/sportsdataverse/sportsdataverse-data/releases/tag/cfb_recruits
- nflverse draft_picks, combine, players: https://github.com/nflverse/nflverse-data/releases
- cfbfastR-data rosters 2024: https://github.com/sportsdataverse/cfbfastR-data/tree/main/rosters/csv

Usage: python3 trial3_baselines.py DIR COLLEGES_CSV > trial3_baselines.json
"""
import json, sys, re
import numpy as np
import pandas as pd

D, CC = sys.argv[1], sys.argv[2]
out = {"source": "CFBD scores 2001-2025, ESPN recruit ratings, nflverse draft/combine/players, CFBD rosters"}

# ---------- 1-2. dynasties, titles and droughts ----------
g = pd.concat([pd.read_parquet(f"{D}/cfb_sched_{y}.parquet") for y in range(2001, 2026)])
g = g[g.completed & g.home_points.notna()]
# National champion = winner of the season's last FBS-vs-FBS game (BCS title game 2001-2013, CFP title game 2014+).
champs = {}
for s, x in g[(g.season_type == "postseason") & (g.home_division == "fbs") & (g.away_division == "fbs")].groupby("season"):
    last = x.sort_values("start_date").iloc[-1]
    champs[int(s)] = last.home_team if last.home_points > last.away_points else last.away_team
out["title_game_winners"] = champs
win15 = {}
for a in range(2001, 2012):
    c = pd.Series([champs[y] for y in range(a, a + 15)]).value_counts()
    win15[f"{a}-{a + 14}"] = {"most_titles": int(c.iloc[0]), "by": c.index[0], "distinct_champions": int(len(c))}
out["titles_per_15_years"] = win15

fb = g[g.home_division.eq("fbs") | g.away_division.eq("fbs")]
rows = []
for side, opp in (("home", "away"), ("away", "home")):
    t = fb[fb[f"{side}_division"] == "fbs"]
    rows.append(pd.DataFrame({"season": t.season, "team": t[f"{side}_team"],
                              "w": (t[f"{side}_points"] > t[f"{opp}_points"]).astype(int),
                              "l": (t[f"{side}_points"] < t[f"{opp}_points"]).astype(int)}))
rec = pd.concat(rows).groupby(["team", "season"])[["w", "l"]].sum().reset_index()
rec["pct"] = rec.w / (rec.w + rec.l)
rec["winning"] = rec.w > rec.l
rec["unbeaten"] = (rec.l == 0) & (rec.w >= 10)

cols = pd.read_csv(CC)
p4conf = ["SEC", "Big Ten", "Big 12", "ACC"]
p4 = set(cols[cols.conference.isin(p4conf)].name)
# CFBD school names differ a little from colleges.csv; match on the school part of the name.
cfbd = set(rec.team)
name_map = {n: n for n in p4 if n in cfbd}
for n in p4 - set(name_map):
    alt = n.replace("State", "St").replace("(FL)", "").strip()
    hits = [c for c in cfbd if c.replace("State", "St").startswith(alt)]
    if hits:
        name_map[n] = sorted(hits, key=len)[0]
p4t = sorted(name_map.values()) + ["Notre Dame"]
r4 = rec[rec.team.isin(p4t)]
out["p4_programs_matched"] = len(set(r4.team))
out["p4_unmatched"] = sorted(p4 - set(name_map))

def longest_drought(s):
    best = cur = 0
    for v in s:
        cur = 0 if v else cur + 1
        best = max(best, cur)
    return best

dr = r4.sort_values("season").groupby("team").winning.apply(longest_drought).sort_values(ascending=False)
w1125 = r4[r4.season.between(2011, 2025)].groupby("team").winning.sum().sort_values()
out["p4_droughts"] = {
    "note": "Current (2026) SEC, Big Ten, Big 12, ACC members plus Notre Dame; a winning season = more wins than losses (all games). Includes seasons some schools spent outside a power conference.",
    "longest_run_without_winning_season_2001_2025_top10": {k: int(v) for k, v in dr.head(10).items()},
    "winning_seasons_2011_2025_fewest10": {k: int(v) for k, v in w1125.head(10).items()},
    "programs_with_zero_winning_seasons_2011_2025": int((w1125 == 0).sum()),
    "programs_with_zero_in_any_15yr_window_2001_2025": int(sum(
        any(x[(x.season >= a) & (x.season <= a + 14)].winning.sum() == 0 for a in range(2001, 2012))
        for _, x in r4.groupby("team"))),
}
top = r4[r4.season.between(2011, 2025)].groupby("team")[["w", "l"]].sum()
top["pct"] = top.w / (top.w + top.l)
top = top.sort_values("pct", ascending=False)
out["best_15yr_records_2011_2025"] = {k: f"{int(v.w)}-{int(v.l)} ({v.pct:.3f})" for k, v in top.head(5).iterrows()}
out["unbeaten_seasons_2011_2025_fbs"] = int(rec[rec.season.between(2011, 2025) & rec.unbeaten].shape[0])
out["unbeaten_by_team_2011_2025"] = {k: int(v) for k, v in rec[rec.season.between(2011, 2025) & rec.unbeaten].team.value_counts().items()}
rp = rec.sort_values("season")
rp["prev"] = rp.groupby("team").pct.shift(1)
out["fbs_win_pct_year_to_year_corr"] = round(float(rp.pct.corr(rp.prev)), 3)

# ---------- 5. draft rates by stars, and where draftees came from ----------
rcr = pd.concat([pd.read_parquet(f"{D}/recruits_{y}.parquet") for y in range(2010, 2022)])
norm = lambda s: re.sub(r"[^a-z]", "", re.sub(r"\b(jr|sr|ii|iii|iv)\b", "", str(s).lower()))
rcr["key"] = rcr.player_name.map(norm)
dp = pd.read_parquet(f"{D}/draft_picks.parquet")
dp = dp[dp.season.between(2013, 2025)]
dp["key"] = dp.pfr_player_name.map(norm)
first = rcr.sort_values("season").drop_duplicates("key")
m = dp.merge(first[["key", "stars", "season"]].rename(columns={"season": "class"}), on="key", how="left")
m = m[m["class"].isna() | ((m.season - m["class"]).between(2, 7))]
share = m.stars.fillna(0).value_counts(normalize=True).sort_index()
out["draft_picks_by_star"] = {
    "note": "Draft picks 2013-2025 matched by name to ESPN-rated FBS/FCS signees (classes 2010-2021, drafted 2-7 years later). 0 = no match: unrated, walk-on, JUCO or prep signee, or a name mismatch.",
    "share": {str(int(k)): round(float(v), 3) for k, v in share.items()},
    "share_round1": {str(int(k)): round(float(v), 3) for k, v in m[m["round"] == 1].stars.fillna(0).value_counts(normalize=True).sort_index().items()},
    "picks": int(len(m))}

# ---------- 8. measurables: kickers, punters, QBs ----------
cb = pd.read_parquet(f"{D}/combine.parquet")
cb = cb[cb.season >= 2000]
meas = {}
for pos in ["QB", "K", "P", "WR", "RB", "LB", "TE", "OT", "DE"]:
    x = cb[cb.pos == pos]
    f = x.forty.dropna()
    meas[pos] = {"n": int(len(x)), "weight_mean": round(float(x.wt.mean()), 1), "forty_n": int(len(f)),
                 "forty_mean": round(float(f.mean()), 2), "forty_p10": round(float(f.quantile(.1)), 2),
                 "forty_p90": round(float(f.quantile(.9)), 2), "forty_runners_share": round(len(f) / max(len(x), 1), 2),
                 "broad_mean": round(float(x.broad_jump.mean()), 1), "vertical_mean": round(float(x.vertical.mean()), 1)}
out["combine_by_position_2000_2025"] = meas
out["combine_note"] = "Only players invited to the combine; many kickers and some QBs skip running, so forty_runners_share is shown. Specialists who run are likely the faster ones."

# ---------- 9. full-name repeats among real players ----------
ro = pd.read_csv(f"{D}/roster_cfb_2024.csv")
ro["full"] = (ro.first_name.fillna("") + " " + ro.last_name.fillna("")).str.lower().str.strip()
ro = ro[ro.full.str.len() > 2]
def rep_share(names):
    c = names.value_counts()
    return round(float(names.map(c).gt(1).mean()), 4)
pl = pd.read_parquet(f"{D}/players.parquet", columns=["display_name"])
pln = pl.display_name.dropna().str.lower().str.replace(r"\s+(jr\.?|sr\.?|ii|iii|iv)$", "", regex=True)
rng = np.random.default_rng(1)
curve = {}
for n in [1000, 5000, 10000, len(ro)]:
    idx = rng.choice(len(ro), size=min(n, len(ro)), replace=False)
    curve[str(len(idx))] = rep_share(ro.full.iloc[idx].reset_index(drop=True))
out["name_repeats"] = {
    "note": "Share of players whose full name (first + last, suffixes dropped for NFL) is shared with at least one other player in the same pool.",
    "cfbd_rosters_2024_all_divisions_pool": int(len(ro)), "cfbd_rosters_2024_share": rep_share(ro.full),
    "cfbd_2024_by_pool_size": curve,
    "nflverse_all_players_pool": int(len(pln)), "nflverse_all_players_share": rep_share(pln),
    "most_common_2024": {k: int(v) for k, v in ro.full.value_counts().head(8).items()}}

json.dump(out, sys.stdout, indent=1)
