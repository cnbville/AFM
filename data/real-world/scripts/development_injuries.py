"""Age curves, careers, breakouts and injuries from NFL data (research area 4).

Reads nflverse releases (https://github.com/nflverse/nflverse-data/releases):
  pbp/play_by_play_YEAR.parquet (2006-2025), snap_counts/snap_counts_YEAR.parquet (2012-2025),
  injuries/injuries_YEAR.parquet (2012-2025), players/players.parquet, draft_picks/draft_picks.parquet

Usage: python3 development_injuries.py DIR > development_injuries.json
Regular seasons only.
"""
import json, sys
import numpy as np
import pandas as pd

D = sys.argv[1]
out = {"source": "nflverse play-by-play 2006-2025, snap counts 2012-2025, injury reports 2012-2025, players, draft picks"}
r = lambda x, n=3: None if x is None or pd.isna(x) else round(float(x), n)

pl = pd.read_parquet(f"{D}/players.parquet", columns=["gsis_id", "pfr_id", "birth_date", "position", "rookie_season"])
pl["birth"] = pd.to_datetime(pl.birth_date, errors="coerce")
GROUP = {"QB": "QB", "RB": "RB", "FB": "RB", "WR": "WR", "TE": "TE", "T": "OL", "G": "OL", "C": "OL", "OT": "OL", "OG": "OL", "OL": "OL",
         "DE": "EDGE", "OLB": "EDGE", "DT": "DL", "NT": "DL", "DL": "DL", "LB": "LB", "ILB": "LB", "MLB": "LB",
         "CB": "CB", "DB": "CB", "S": "S", "FS": "S", "SS": "S", "SAF": "S", "K": "K", "P": "P"}

def age_on_sep1(birth, season):
    return ((pd.to_datetime(season.astype(int).astype(str) + "-09-01") - birth).dt.days / 365.25).floordiv(1)

# ---------- playing time by age ----------
sc = pd.concat([pd.read_parquet(f"{D}/snap_counts_{y}.parquet") for y in range(2012, 2026)])
sc = sc[sc.game_type == "REG"]
sc["pct"] = sc[["offense_pct", "defense_pct"]].max(axis=1)
ps = sc.groupby(["season", "pfr_player_id", "position"]).agg(games=("game_id", "nunique"), pct=("pct", "mean")).reset_index()
ps = ps.merge(pl[["pfr_id", "birth"]].dropna().drop_duplicates("pfr_id"), left_on="pfr_player_id", right_on="pfr_id", how="left")
ps["age"] = age_on_sep1(ps.birth, ps.season)
ps["grp"] = ps.position.map(GROUP)
ps = ps.dropna(subset=["age", "grp"])
starters = ps[(ps.pct >= 0.6) & (ps.games >= 8)]
age_tab = {}
for g_, x in starters.groupby("grp"):
    vc = x.age.value_counts(normalize=True).sort_index()
    age_tab[g_] = {"n_starter_seasons": int(len(x)), "median_age": r(x.age.median(), 1), "mode_age": int(vc.idxmax()),
                   "share_by_age": {str(int(k)): r(v) for k, v in vc.items() if v >= 0.01},
                   "share_30_plus": r((x.age >= 30).mean()), "share_33_plus": r((x.age >= 33).mean())}
out["starter_age"] = {"note": "Player-seasons with 8+ games and 60%+ of the unit's snaps (PFR positions grouped). Age on Sept 1.", **age_tab}
# Chance a starter is still a starter next year, by age.
st = starters[["season", "pfr_player_id", "grp", "age"]].copy()
nxt = set(zip(starters.season - 1, starters.pfr_player_id))
st["kept"] = [(s, p) in nxt for s, p in zip(st.season, st.pfr_player_id)]
st = st[st.season < 2025]
keep = {}
for g_, x in st.groupby("grp"):
    keep[g_] = {str(int(a)): r(y.kept.mean()) for a, y in x.groupby("age") if len(y) >= 30}
out["starter_kept_next_year_by_age"] = keep

# ---------- performance by age (delta method) ----------
cols = ["season", "season_type", "play_type", "qb_dropback", "passer_player_id", "receiver_player_id", "rusher_player_id",
        "epa", "qb_scramble", "qb_kneel", "pass_attempt", "sack", "receiving_yards"]
pbp = pd.concat([pd.read_parquet(f"{D}/play_by_play_{y}.parquet", columns=cols) for y in range(2006, 2026)])
pbp = pbp[pbp.season_type == "REG"].reset_index(drop=True)
bmap = pl.dropna(subset=["birth"]).drop_duplicates("gsis_id").set_index("gsis_id").birth

def season_table(df, idcol, minn):
    t = df.groupby(["season", idcol]).agg(n=("epa", "size"), epa=("epa", "mean")).reset_index()
    t = t[t.n >= minn]
    t["age"] = age_on_sep1(t[idcol].map(bmap), t.season)
    return t.dropna(subset=["age"])

def delta_curve(t, idcol):
    a = t.merge(t.assign(season=t.season - 1), on=[idcol, "season"], suffixes=("", "_next"))
    a["d"] = a.epa_next - a.epa
    a["w"] = 2 / (1 / a.n + 1 / a.n_next)  # harmonic mean of plays
    res, cum = {}, 0.0
    for age_, x in a.groupby("age"):
        if len(x) < 25:
            continue
        dm = np.average(x.d, weights=x.w)
        cum += dm
        res[f"{int(age_)}->{int(age_) + 1}"] = {"pairs": int(len(x)), "change": r(dm), "cumulative_from_start": r(cum)}
    return res

qb = season_table(pbp[(pbp.qb_dropback == 1)].rename(columns={"passer_player_id": "pid"}), "pid", 150)
wr = pbp[(pbp.pass_attempt == 1) & (pbp.sack == 0) & pbp.receiver_player_id.notna()].copy().reset_index(drop=True)
wr["pos"] = wr.receiver_player_id.map(pl.drop_duplicates("gsis_id").set_index("gsis_id").position)
wrt = season_table(wr[wr.pos == "WR"].rename(columns={"receiver_player_id": "pid"}), "pid", 40)
tet = season_table(wr[wr.pos == "TE"].rename(columns={"receiver_player_id": "pid"}), "pid", 30)
ru = pbp[(pbp.play_type == "run") & (pbp.qb_scramble == 0) & (pbp.qb_kneel == 0)].copy()
ru["pos"] = ru.rusher_player_id.map(pl.drop_duplicates("gsis_id").set_index("gsis_id").position)
rbt = season_table(ru[ru.pos == "RB"].rename(columns={"rusher_player_id": "pid"}), "pid", 80)
out["epa_age_curves"] = {
    "note": "Delta method: the average change in a player's EPA per play from one season to the next, by age, same player both years (weighted by plays). Survivors only, so it flatters old ages a little (players who decline badly stop playing). Cumulative = sum of the changes from the youngest age listed.",
    "QB_per_dropback": delta_curve(qb, "pid"), "RB_per_carry": delta_curve(rbt, "pid"),
    "WR_per_target": delta_curve(wrt, "pid"), "TE_per_target": delta_curve(tet, "pid")}

# ---------- careers by position and round ----------
dp = pd.read_parquet(f"{D}/draft_picks.parquet")
dp = dp[dp.season.between(2000, 2015)]
dp["years"] = (dp["to"].fillna(dp.season - 1) - dp.season + 1).clip(lower=0)
dp["grp"] = dp.position.map(GROUP)
car = {}
for g_, x in dp.dropna(subset=["grp"]).groupby("grp"):
    car[g_] = {"picks": int(len(x)), "median_years": r(x.years.median(), 1), "mean_years": r(x.years.mean(), 1),
               "share_5_plus": r((x.years >= 5).mean()), "share_10_plus": r((x.years >= 10).mean()),
               "median_games": r(x.games.fillna(0).median(), 0)}
rnd = {str(int(k)): {"median_years": r(x.years.median(), 1), "share_never_played": r((x.games.fillna(0) == 0).mean()),
                     "share_5_plus_years": r((x.years >= 5).mean()), "share_pro_bowl": r((x.probowls.fillna(0) > 0).mean())}
       for k, x in dp.groupby("round")}
out["careers_drafted_2000_2015"] = {"note": "Years from draft to last season played (PFR 'to'). Drafted players only; undrafted careers are shorter.",
                                    "by_position": car, "by_round": rnd}

# ---------- breakouts ----------
def season_yards(df, idcol, ycol):
    return df.groupby(["season", idcol])[ycol].sum().reset_index()

ry = season_yards(wr[wr.pos == "WR"].assign(yd=wr.receiving_yards.fillna(0)), "receiver_player_id", "yd")
ry["rook"] = ry.receiver_player_id.map(pl.drop_duplicates("gsis_id").set_index("gsis_id").rookie_season)
ry = ry.dropna(subset=["rook"])
ry["cy"] = ry.season - ry.rook + 1
m = ry.merge(ry.assign(season=ry.season - 1)[["season", "receiver_player_id", "yd"]], on=["season", "receiver_player_id"], suffixes=("", "_next"))
m = m[m.cy.between(1, 8)]
br = {}
for cy, x in m.groupby("cy"):
    base = x[x.yd < 500]
    br[f"year {int(cy)} to {int(cy) + 1}"] = {"wr_under_500": int(len(base)),
                                              "share_reaching_900": r((base.yd_next >= 900).mean()),
                                              "share_reaching_1200": r((base.yd_next >= 1200).mean())}
out["wr_breakouts"] = {"note": "Receivers with under 500 receiving yards one season: share who reach 900 or 1,200 the next season, by NFL career year. 2006-2025 play-by-play.", **br}

# ---------- injuries ----------
inj = pd.concat([pd.read_parquet(f"{D}/injuries_{y}.parquet") for y in range(2012, 2026)])
inj = inj[(inj.game_type == "REG") & inj.report_status.isin(["Out", "Doubtful"])]
inj["grp"] = inj.position.map(GROUP)
inj["part"] = inj.report_primary_injury.fillna("Unknown").str.split(",").str[0].str.strip().str.title()
inj = inj.merge(pl[["gsis_id", "pfr_id"]].drop_duplicates("gsis_id"), on="gsis_id", how="left")
inj = inj.sort_values(["gsis_id", "season", "week"])
inj["prev_week"] = inj.groupby(["gsis_id", "season"]).week.shift(1)
ep = inj[(inj.prev_week.isna()) | (inj.week - inj.prev_week > 1)].copy()
# Games missed: consecutive team games from the report week on in which the player took no snap.
played = set(zip(sc.pfr_player_id, sc.season, sc.week))
team_weeks = sc.groupby(["team", "season"]).week.apply(lambda w: sorted(set(w))).to_dict()
missed, season_end = [], []
for row in ep.itertuples():
    weeks = [w for w in team_weeks.get((row.team, row.season), []) if w >= row.week]
    n = 0
    for w in weeks:
        if (row.pfr_id, row.season, w) in played:
            break
        n += 1
    missed.append(n)
    season_end.append(n == len(weeks) and n > 0)
ep["missed"] = missed
ep["to_season_end"] = season_end
ep = ep[ep.pfr_id.notna()]
parts = {}
for p_, x in ep.groupby("part"):
    if len(x) < 150:
        continue
    parts[p_] = {"episodes": int(len(x)), "share": r(len(x) / len(ep)), "mean_games_missed": r(x.missed.mean(), 2),
                 "median_games_missed": r(x.missed.median(), 1), "share_0_games": r((x.missed == 0).mean()),
                 "share_4_plus": r((x.missed >= 4).mean()), "share_to_season_end": r(x.to_season_end.mean())}
out["injury_episodes_by_body_part"] = dict(sorted(parts.items(), key=lambda kv: -kv[1]["episodes"]))
# Rate per regular player-season by position.
reg = ps[ps.games >= 4].groupby("grp").size()
epg = ep.groupby("grp").size()
out["injury_episodes_per_player_season"] = {g_: r(epg.get(g_, 0) / reg[g_], 3) for g_ in reg.index}
out["games_missed_per_player_season"] = {g_: r(ep[ep.grp == g_].missed.sum() / reg[g_], 2) for g_ in reg.index}
out["injury_note"] = ("Episode = a player listed Out or Doubtful on the game-status report after not being listed the week before. "
                      "Games missed = team games from that week on with no snap for him, counted to his return or the season's end "
                      "(so injuries late in a season are cut short). Rates divide by player-seasons with 4+ games. "
                      "Players placed straight on injured reserve without a report entry are missed, so long injuries are undercounted.")
json.dump(out, sys.stdout, indent=1)
