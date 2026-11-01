"""Real NFL careers, the draft, the combine, contracts and season lines (world reader lane R3).

Reads nflverse release files (https://github.com/nflverse/nflverse-data/releases), saved in DIR:
  draft_picks.csv (draft_picks), players.csv (players), combine.csv (combine),
  historical_contracts.csv.gz (contracts), roster_YEAR.csv 1999-2025 (rosters),
  stats_player_reg_YEAR.csv 1999-2025 (stats_player).

Usage: python3 nfl_careers.py DIR OUTDIR > nfl_careers.json
Writes OUTDIR/nfl_draft_by_college.csv beside the JSON.
Regular seasons only. Development checks only, never in-game tuning.
"""
import json, re, sys
import numpy as np
import pandas as pd

D, OUT = sys.argv[1], sys.argv[2]
PS = (5, 10, 25, 50, 75, 90, 95)


def r(x, n=3):
    if x is None:
        return None
    try:
        x = float(x)
    except (TypeError, ValueError):
        return None
    return None if np.isnan(x) else round(x, n)


def pct(s, ps=PS, n=2):
    s = pd.Series(s).dropna()
    if s.empty:
        return None
    out = {f"p{p}": r(np.percentile(s, p), n) for p in ps}
    out["mean"] = r(s.mean(), n)
    out["n"] = int(len(s))
    return out


GROUP = {"QB": "QB", "RB": "RB", "FB": "RB", "HB": "RB", "WR": "WR", "TE": "TE",
         "T": "OL", "OT": "OL", "G": "OL", "OG": "OL", "C": "OL", "OL": "OL", "IOL": "OL", "LT": "OL", "RT": "OL",
         "DE": "DL", "DT": "DL", "NT": "DL", "DL": "DL", "EDGE": "DL", "ED": "DL", "IDL": "DL",
         "LB": "LB", "OLB": "LB", "ILB": "LB", "MLB": "LB",
         "CB": "DB", "S": "DB", "FS": "DB", "SS": "DB", "DB": "DB", "SAF": "DB",
         "K": "K", "PK": "K", "P": "P", "LS": "LS"}


def grp(p):
    return GROUP.get(str(p).upper(), "other")


out = {"source": "nflverse-data releases: draft_picks, players, combine, contracts (OverTheCap), rosters, stats_player",
       "note": "Regular seasons. Ages at September 1 of the season. Careers counted from the seasonal rosters (any status listed that season, practice squad and reserve lists included)."}

# --- the draft ---
dp = pd.read_csv(f"{D}/draft_picks.csv", low_memory=False)
dp["grp"] = dp["position"].map(grp)
a = dp[(dp.season >= 2000) & dp.age.notna()]
out["draft_age"] = {
    "years": "2000-2026 drafts",
    "by_round": {int(k): pct(v.age, n=1) for k, v in a.groupby("round") if k <= 7},
    "by_group": {k: pct(v.age, n=1) for k, v in a.groupby("grp")},
    "share_by_age": {int(k): r(v, 4) for k, v in a.age.round().value_counts(normalize=True).sort_index().items()},
}

c = dp[(dp.season >= 1980) & (dp.season <= 2015)].copy()
c["played"] = c.games.fillna(0) > 0
c["seasons"] = np.where(c.played, c["to"].fillna(c.season) - c.season + 1, 0)


def career(v):
    return {"n": int(len(v)), "never_played": r(1 - v.played.mean(), 4),
            "seasons": pct(v.seasons, (10, 25, 50, 75, 90), 1),
            "share_4plus_seasons": r((v.seasons >= 4).mean(), 4), "share_8plus": r((v.seasons >= 8).mean(), 4),
            "share_12plus": r((v.seasons >= 12).mean(), 4),
            "games_mean": r(v.games.fillna(0).mean(), 1), "seasons_started_mean": r(v.seasons_started.fillna(0).mean(), 2),
            "pro_bowl_share": r((v.probowls.fillna(0) > 0).mean(), 4), "all_pro_share": r((v.allpro.fillna(0) > 0).mean(), 4),
            "hall_share": r(v.hof.fillna(False).astype(bool).mean(), 4), "w_av_mean": r(v.w_av.fillna(0).mean(), 1)}


out["career_by_round"] = {
    "years": "draft classes 1980-2015 (seasons = last season played minus draft year plus one; players still active count to 2025)",
    "by_round": {int(k): career(v) for k, v in c.groupby("round") if k <= 12},
    "by_round_and_group": {f"{int(k[0])}|{k[1]}": {"n": int(len(v)), "seasons_p50": r(v.seasons.median(), 1),
                                                    "share_8plus": r((v.seasons >= 8).mean(), 3)}
                           for k, v in c[c["round"] <= 7].groupby(["round", "grp"])},
}
out["draft_by_position_year"] = {int(y): {k: int(n) for k, n in v.grp.value_counts().items()}
                                 for y, v in dp[dp.season >= 1980].groupby("season")}
bc = dp[dp.season >= 1980].groupby(["season", "college"]).agg(picks=("pick", "size"), round1=("round", lambda s: int((s == 1).sum())),
                                                             top100=("pick", lambda s: int((s <= 100).sum()))).reset_index()
bc.to_csv(f"{OUT}/nfl_draft_by_college.csv", index=False)
out["draft_by_college"] = {"years": "1980-2026", "file": "nfl_draft_by_college.csv (picks, first-rounders and top-100 picks per college per draft)",
                           "colleges_with_a_pick_by_year": {int(y): int(v.college.nunique()) for y, v in dp[dp.season >= 1980].groupby("season")},
                           "top_college_picks_by_year": {int(y): int(v.picks.max()) for y, v in bc.groupby("season")}}
comp = []
for (y, rd), v in dp[(dp.season >= 1994) & dp["round"].between(3, 7)].groupby(["season", "round"]):
    extra = len(v) - 32
    if extra > 0:
        comp.append(v.sort_values("pick").tail(extra))
comp = pd.concat(comp)
out["compensatory_picks"] = {"years": "1994-2026, taken as the picks past the 32nd in rounds 3-7 (close: a forfeited pick shifts it by one)",
                             "per_year": {int(y): int(len(v)) for y, v in comp.groupby("season")},
                             "per_club_per_year": pct(comp.groupby(["season", "team"]).size().reindex(pd.MultiIndex.from_product([sorted(comp.season.unique()), sorted(dp[dp.season >= 2002].team.unique())]), fill_value=0), (25, 50, 75, 90, 99), 1),
                             "most_by_club_2000_2025": {k: int(n) for k, n in comp[comp.season >= 2000].team.value_counts().head(8).items()}}
out["long_snappers_drafted"] = {int(y): int(n) for y, n in dp[dp.position == "LS"].groupby("season").size().items()}
out["kickers_punters_drafted"] = {int(y): {"K": int((v.position == "K").sum()), "P": int((v.position == "P").sum())}
                                  for y, v in dp[dp.season >= 1980].groupby("season")}

# --- rosters: careers, undrafted, retirement age, position changes ---
ro = pd.concat([pd.read_csv(f"{D}/roster_{y}.csv", low_memory=False,
                            usecols=["season", "team", "position", "birth_date", "gsis_id", "full_name", "status", "years_exp"])
                for y in range(1999, 2026)])
ro = ro[ro.gsis_id.notna()]
pl = pd.read_csv(f"{D}/players.csv", low_memory=False)
first = ro.groupby("gsis_id").season.min()
last = ro.groupby("gsis_id").season.max()
nseas = ro.groupby("gsis_id").season.nunique()
pos = ro.groupby("gsis_id").position.agg(lambda s: s.mode().iat[0] if not s.mode().empty else None)
bd = ro.groupby("gsis_id").birth_date.first()
car = pd.DataFrame({"first": first, "last": last, "seasons": nseas, "pos": pos, "birth": bd})
car = car.join(pl.set_index("gsis_id")[["draft_round", "draft_year", "rookie_season"]], how="left")
car["grp"] = car.pos.map(grp)
car["drafted"] = car.draft_round.notna()
ent = car[(car["first"] >= 2000) & (car["first"] <= 2015) & ((car.rookie_season.isna()) | (car.rookie_season >= 2000))]
out["roster_careers"] = {
    "years": "players first on a roster 2000-2015, seasons on any roster through 2025",
    "undrafted": pct(ent[~ent.drafted].seasons, (10, 25, 50, 75, 90), 1),
    "undrafted_share_3plus": r((ent[~ent.drafted].seasons >= 3).mean(), 4),
    "undrafted_share_6plus": r((ent[~ent.drafted].seasons >= 6).mean(), 4),
    "drafted": pct(ent[ent.drafted].seasons, (10, 25, 50, 75, 90), 1),
    "undrafted_by_group": {k: pct(v.seasons, (25, 50, 75, 90), 1) for k, v in ent[~ent.drafted].groupby("grp")},
}
ro2 = ro.join(car[["drafted"]], on="gsis_id")
out["undrafted_share_of_rosters"] = {int(y): r(1 - v.drafted.mean(), 4) for y, v in ro2.groupby("season")}

done = car[(car["last"] >= 2000) & (car["last"] <= 2020) & car.birth.notna()].copy()
done["age"] = (pd.to_datetime(done["last"].astype(int).astype(str) + "-09-01") - pd.to_datetime(done.birth, errors="coerce")).dt.days / 365.25
done = done[done.age.between(19, 50)]
out["last_season_age"] = {
    "years": "players whose last roster season was 2000-2020 (age at September 1 of that season)",
    "by_group": {k: pct(v.age, n=1) for k, v in done.groupby("grp")},
    "by_position": {k: pct(v.age, n=1) for k, v in done.groupby("pos") if len(v) >= 30},
    "by_group_4plus_seasons": {k: pct(v.age, n=1) for k, v in done[done.seasons >= 4].groupby("grp")},
    "seasons_by_group": {k: pct(v.seasons, (25, 50, 75, 90), 1) for k, v in done[done["first"] >= 2000].groupby("grp")},
}
age_ro = ro.copy()
age_ro["age"] = (pd.to_datetime(age_ro.season.astype(str) + "-09-01") - pd.to_datetime(age_ro.birth_date, errors="coerce")).dt.days / 365.25
out["roster_age_by_season"] = {int(y): pct(v.age, (10, 50, 90), 1) for y, v in age_ro[age_ro.age.between(19, 50)].groupby("season")}

dpj = dp[(dp.season >= 2000) & (dp.season <= 2020) & dp.gsis_id.notna()][["gsis_id", "position", "grp"]].join(car[["pos", "grp"]].rename(columns={"grp": "grp_now"}), on="gsis_id")
dpj = dpj[dpj.pos.notna()]
out["position_changes"] = {
    "years": "drafted 2000-2020: drafted position against the position listed most often on NFL rosters",
    "same_group_share": {k: r((v.grp == v.grp_now).mean(), 4) for k, v in dpj.groupby("grp")},
    "to_group": {k: {g: r(s, 4) for g, s in v.grp_now.value_counts(normalize=True).head(4).items()} for k, v in dpj.groupby("grp")},
}

bdt = pd.to_datetime(pl.birth_date, errors="coerce")
bm = bdt[(bdt.dt.year >= 1975) & (bdt.dt.year <= 2003)].dt.month
out["birth_months"] = {"years": "NFL players born 1975-2003 (nflverse players)", "n": int(len(bm)),
                       "share_by_month": {int(k): r(v, 4) for k, v in bm.value_counts(normalize=True).sort_index().items()},
                       "note": "for the relative-age check; compare with US births by month (CDC, not reachable here: about 8.0-8.9% a month, August highest)"}

# --- the combine ---
cb = pd.read_csv(f"{D}/combine.csv", low_memory=False)
cb["grp"] = cb.pos.map(grp)
drills = ["forty", "bench", "vertical", "broad_jump", "cone", "shuttle"]
out["combine"] = {
    "years": f"{int(cb.season.min())}-{int(cb.season.max())}",
    "invites_by_year": {int(y): int(len(v)) for y, v in cb.groupby("season")},
    "long_snappers_by_year": {int(y): int((v.pos == "LS").sum()) for y, v in cb.groupby("season")},
    "drill_share_by_year": {int(y): {d: r(v[d].notna().mean(), 3) for d in drills} for y, v in cb.groupby("season")},
    "drafted_share_by_year": {int(y): r(v.draft_round.notna().mean(), 3) for y, v in cb.groupby("season")},
}
def inches(h):
    m = re.match(r"^(\d+)-(\d+)$", str(h))
    return float(m.group(1)) * 12 + float(m.group(2)) if m else np.nan


cb["ht_in"] = cb.ht.map(inches)
pairs = [("forty", "vertical"), ("forty", "broad_jump"), ("vertical", "broad_jump"), ("cone", "shuttle"), ("forty", "cone"),
         ("forty", "shuttle"), ("bench", "wt"), ("forty", "wt"), ("ht_in", "wt"), ("vertical", "cone")]
out["combine"]["drill_correlations"] = {
    k: {f"{x}~{y}": r(v[[x, y]].dropna().corr().iloc[0, 1], 3) for x, y in pairs if v[[x, y]].dropna().shape[0] >= 40}
    for k, v in cb.groupby("grp")}
cb["era"] = pd.cut(cb.season, [1999, 2009, 2019, 2030], labels=["2000-09", "2010-19", "2020-25"])
out["combine"]["body_by_era"] = {f"{g}|{e}": {"ht_in": r(v.ht_in.mean(), 2), "wt": r(v.wt.mean(), 1), "forty": r(v.forty.mean(), 3), "n": int(len(v))}
                                 for (g, e), v in cb.groupby(["grp", "era"], observed=True)}

# --- contracts ---
ct = pd.read_csv(f"{D}/historical_contracts.csv.gz", low_memory=False)
ct = ct[ct.year_signed.between(2006, 2025) & ct.apy_cap_pct.notna()]
ct["era"] = pd.cut(ct.year_signed, [2005, 2010, 2015, 2020, 2025], labels=["2006-10", "2011-15", "2016-20", "2021-25"])
out["contracts"] = {
    "years": "signed 2006-2025 (OverTheCap via nflverse; early years thin)",
    "apy_cap_pct_by_position_era": {f"{p}|{e}": pct(v.apy_cap_pct * 100, (10, 25, 50, 75, 90, 99), 2)
                                    for (p, e), v in ct.groupby(["position", "era"], observed=True) if len(v) >= 20},
    "top_apy_cap_pct_by_year": {f"{int(y)}|{p}": r(v.apy_cap_pct.max() * 100, 2) for (y, p), v in ct.groupby(["year_signed", "position"])},
    "signed_per_year": {int(y): int(n) for y, n in ct.groupby("year_signed").size().items()},
    "years_by_position": {p: pct(v.years, (25, 50, 75, 90), 1) for p, v in ct.groupby("position") if len(v) >= 50},
}

# --- season lines, kickers, fantasy ---
st = pd.concat([pd.read_csv(f"{D}/stats_player_reg_{y}.csv", low_memory=False) for y in range(1999, 2026)])
lines = {}
for y, v in st.groupby("season"):
    qb = v[(v.position == "QB") & (v.attempts >= 224)]
    rb = v[(v.position.isin(["RB", "FB"])) & (v.carries >= 100)]
    rec = v[(v.position.isin(["WR", "TE", "RB"])) & (v.targets >= 50)]
    df = v[v.position_group.isin(["DL", "LB", "DB"]) & (v.games >= 8)]
    k = v[(v.fg_att >= 15)]
    pu = v[(v.pt_att >= 30)]
    lines[int(y)] = {
        "passers_4000": int((v.passing_yards >= 4000).sum()), "passers_3000": int((v.passing_yards >= 3000).sum()),
        "rushers_1000": int((v.rushing_yards >= 1000).sum()), "receivers_1000": int((v.receiving_yards >= 1000).sum()),
        "catches_100": int((v.receptions >= 100).sum()), "sacks_10": int((v.def_sacks >= 10).sum()),
        "ints_6": int((v.def_interceptions >= 6).sum()),
        "qb_pass_yds": pct(qb.passing_yards, (10, 50, 90), 0), "qb_ypa": pct(qb.passing_yards / qb.attempts, (10, 50, 90), 2),
        "qb_td_int": pct(qb.passing_tds / qb.passing_interceptions.replace(0, np.nan), (10, 50, 90), 2),
        "rb_rush_yds": pct(rb.rushing_yards, (10, 50, 90), 0), "rb_ypc": pct(rb.rushing_yards / rb.carries, (10, 50, 90), 2),
        "rec_yds": pct(rec.receiving_yards, (10, 50, 90), 0), "catch_rate": pct(rec.receptions / rec.targets, (10, 50, 90), 3),
        "def_sacks": pct(df.def_sacks[df.def_sacks > 0], (50, 90, 99), 1),
        "def_tackles": pct(df.def_tackles_solo + df.def_tackle_assists, (50, 90, 99), 0),
        "kicker_fg_pct": pct(k.fg_made / k.fg_att, (10, 50, 90), 3), "kicker_long": pct(k.fg_long, (10, 50, 90), 0),
        "punter_gross": pct(pu.pt_yards / pu.pt_att, (10, 50, 90), 1), "punter_net": pct(pu.pt_net_yards / pu.pt_att, (10, 50, 90), 1),
        "punter_inside20_rate": pct(pu.pt_inside_20 / pu.pt_att, (10, 50, 90), 3),
    }
out["season_lines"] = {"years": "1999-2025", "note": "regulars: QB 224+ attempts, backs 100+ carries, receivers 50+ targets, defenders 8+ games, kickers 15+ FG tries, punters 30+ punts",
                       "by_season": lines}
fan = {}
for y, v in st.groupby("season"):
    fan[int(y)] = {}
    for p in ["QB", "RB", "WR", "TE"]:
        f = v[v.position == p].fantasy_points_ppr.sort_values(ascending=False).reset_index(drop=True)
        if len(f) >= 24:
            fan[int(y)][p] = {"top1": r(f[0], 1), "top6": r(f[5], 1), "top12": r(f[11], 1), "top24": r(f[23], 1)}
out["fantasy_ppr"] = {"years": "1999-2025", "note": "full-PPR points of the 1st, 6th, 12th and 24th best player at each position", "by_season": fan}

json.dump(out, sys.stdout, indent=1)
