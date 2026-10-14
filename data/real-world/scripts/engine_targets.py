"""Real NFL targets for the engine physics fixes and the tactics checks (research area 1).

Reads nflverse release files (https://github.com/nflverse/nflverse-data/releases):
  pbp/play_by_play_YEAR.parquet, pbp_participation/pbp_participation_YEAR.parquet,
  ftn_charting/ftn_charting_YEAR.parquet, snap_counts/snap_counts_YEAR.parquet,
  players/players.parquet, contracts/historical_contracts.parquet
and nfldata games.csv (https://raw.githubusercontent.com/nflverse/nfldata/master/data/games.csv).

Usage: python3 engine_targets.py DIR > engine_targets.json
Regular seasons only. Development checks only, never in-game tuning.
"""
import json, sys
import pandas as pd
import numpy as np

D = sys.argv[1]
YEARS = [2021, 2022, 2023, 2024, 2025]
FTN_YEARS = [2022, 2023, 2024, 2025]


def r(x, n=3):
    return None if x is None or (isinstance(x, float) and np.isnan(x)) else round(float(x), n)


cols = ["game_id", "play_id", "season", "season_type", "posteam", "defteam", "home_team", "play_type",
        "down", "ydstogo", "yardline_100", "wp", "yards_gained", "epa", "success", "qb_dropback", "qb_scramble",
        "qb_kneel", "qb_spike", "sack", "interception", "complete_pass", "pass_attempt", "rush_attempt",
        "air_yards", "rusher_player_id", "receiver_player_id", "passer_player_id", "touchdown", "shotgun",
        "no_huddle", "score_differential", "qtr", "fixed_drive", "fixed_drive_result", "posteam_score_post",
        "kickoff_attempt", "punt_attempt", "return_yards", "touchback", "kickoff_fair_catch", "punt_fair_catch",
        "return_touchdown", "punt_returner_player_id", "kickoff_returner_player_id", "td_team"]
pbp = pd.concat([pd.read_parquet(f"{D}/play_by_play_{y}.parquet", columns=cols) for y in YEARS])
pbp = pbp[pbp.season_type == "REG"]
pbp["key"] = pbp.game_id + "_" + pbp.play_id.astype(int).astype(str)

part = pd.concat([pd.read_parquet(f"{D}/pbp_participation_{y}.parquet").assign(season=y) for y in YEARS])
part["key"] = part.nflverse_game_id + "_" + part.play_id.astype(int).astype(str)
part = part.drop(columns=["season"])
ftn = pd.concat([pd.read_parquet(f"{D}/ftn_charting_{y}.parquet") for y in FTN_YEARS])
ftn["key"] = ftn.nflverse_game_id + "_" + ftn.nflverse_play_id.astype(int).astype(str)
ftn = ftn.drop(columns=["season", "week"])

out = {"source": "nflverse play-by-play, participation (NGS), FTN charting and snap counts; regular seasons",
       "seasons": YEARS}

# ---------- designed runs and box counts ----------
runs = pbp[(pbp.play_type == "run") & (pbp.qb_scramble == 0) & (pbp.qb_kneel == 0)].merge(part, on="key", how="inner")


def blockers(row_pos):
    if not isinstance(row_pos, str):
        return np.nan
    p = row_pos.split(";")
    return sum(x in ("T", "G", "C", "OL", "OT", "OG", "TE", "FB") for x in p)


runs["blockers"] = runs.offense_positions.map(blockers)
runs["box"] = runs.defenders_in_box
runs["box_adv"] = runs.box - runs.blockers
neutral = (runs.down.isin([1, 2])) & (runs.yardline_100.between(20, 80)) & (runs.wp.between(0.1, 0.9)) & (runs.ydstogo >= 5)


def run_split(df, col, vals):
    res = {}
    for v in vals:
        s = df[df[col] == v] if not isinstance(v, tuple) else df[df[col].between(*v)]
        if len(s) < 100:
            continue
        name = str(v) if not isinstance(v, tuple) else f"{v[0]}-{v[1]}"
        res[name] = {"n": int(len(s)), "share": r(len(s) / len(df)), "ypc": r(s.yards_gained.mean(), 2),
                     "median": r(s.yards_gained.median(), 1), "epa": r(s.epa.mean()),
                     "success": r(s.success.mean()), "stuff_rate_0_or_less": r((s.yards_gained <= 0).mean()),
                     "explosive_10plus": r((s.yards_gained >= 10).mean())}
    return res


out["runs_by_box"] = {
    "note": "Designed runs (no scrambles or kneels). Box = defenders in the box (NGS). Heavy boxes come more in short yardage, so the 'neutral' split (1st/2nd and 5+, ball between the 20s, win probability 10-90%) is the fairer test.",
    "all": run_split(runs, "box", [5, 6, 7, 8, 9]),
    "neutral": run_split(runs[neutral], "box", [5, 6, 7, 8, 9]),
    "neutral_by_box_minus_blockers": run_split(runs[neutral], "box_adv", [-3, -2, -1, 0, 1, 2, 3]),
    "blockers_note": "Blockers = offensive linemen + tight ends + fullbacks on the field (a tight end split wide still counts, so this slightly overstates blockers).",
}

# ---------- dropbacks: pressure, coverage, rushers, routes ----------
db = pbp[(pbp.qb_dropback == 1) & (pbp.qb_spike == 0)].merge(part, on="key", how="inner")


def pass_split(s, total=None):
    att = s[s.pass_attempt == 1]
    att = att[att.sack == 0]
    return {"n": int(len(s)), "share": r(len(s) / total) if total else None, "epa": r(s.epa.mean()),
            "success": r(s.success.mean()), "comp": r(att.complete_pass.mean()),
            "ypa": r(att.yards_gained.mean(), 2), "sack_rate": r(s.sack.mean()),
            "int_rate": r(att.interception.mean()), "scramble_rate": r(s.qb_scramble.mean())}


out["dropbacks"] = {"all": pass_split(db)}
out["dropbacks"]["by_pressure"] = {str(k): pass_split(g, len(db)) for k, g in db.groupby("was_pressure")}
out["dropbacks"]["pressure_rate_by_rushers"] = {
    str(int(k)): {"n": int(len(g)), "share": r(len(g) / len(db)), "pressure": r(g.was_pressure.mean()),
                  "sack_rate": r(g.sack.mean()), "epa": r(g.epa.mean()), "time_to_throw": r(g.time_to_throw.mean(), 2)}
    for k, g in db.groupby("number_of_pass_rushers") if len(g) > 300}
ttt = db.time_to_throw.dropna()
out["dropbacks"]["time_to_throw"] = {"mean": r(ttt.mean(), 2), "p10": r(ttt.quantile(.1), 2),
                                     "p50": r(ttt.median(), 2), "p90": r(ttt.quantile(.9), 2)}
cov = db[db.defense_coverage_type.notna() & (db.defense_coverage_type != "")]
out["dropbacks"]["by_coverage"] = {k: pass_split(g, len(cov)) for k, g in cov.groupby("defense_coverage_type") if len(g) > 300}
mz = db[db.defense_man_zone_type.isin(["MAN_COVERAGE", "ZONE_COVERAGE"])]
out["dropbacks"]["by_man_zone"] = {k: pass_split(g, len(mz)) for k, g in mz.groupby("defense_man_zone_type")}
out["dropbacks"]["coverage_by_season"] = {
    str(s): {k: r(v) for k, v in g.defense_coverage_type.value_counts(normalize=True).items() if v > 0.01}
    for s, g in cov.groupby("season")}

# Route of the targeted receiver against coverage (a concept-versus-coverage proxy).
tg = db[(db.pass_attempt == 1) & (db.sack == 0) & db.route.notna() & (db.route != "")]


def tgt(s):
    return {"n": int(len(s)), "epa": r(s.epa.mean()), "catch": r(s.complete_pass.mean()),
            "ypt": r(s.yards_gained.mean(), 2), "int": r(s.interception.mean())}


out["targets_by_route"] = {k: tgt(g) for k, g in tg.groupby("route") if len(g) > 300}
rc = {}
for (rt, cv), g in tg[tg.defense_coverage_type.notna()].groupby(["route", "defense_coverage_type"]):
    if len(g) >= 150:
        rc.setdefault(rt, {})[cv] = tgt(g)
out["targets_by_route_and_coverage"] = rc
rmz = {}
for (rt, cv), g in tg[tg.defense_man_zone_type.isin(["MAN_COVERAGE", "ZONE_COVERAGE"])].groupby(["route", "defense_man_zone_type"]):
    if len(g) >= 150:
        rmz.setdefault(rt, {})[cv] = tgt(g)
out["targets_by_route_man_zone"] = rmz

# ---------- FTN charting: play-action, RPO, screens, motion, blitz ----------
f = pbp[pbp.season.isin(FTN_YEARS) & pbp.play_type.isin(["pass", "run"]) & (pbp.qb_kneel == 0) & (pbp.qb_spike == 0)].merge(ftn, on="key", how="inner")
fdb = f[f.qb_dropback == 1]
out["play_action"] = {"seasons": FTN_YEARS,
                      "rate_of_dropbacks": r(fdb.is_play_action.mean()),
                      "pa": pass_split(fdb[fdb.is_play_action]), "no_pa": pass_split(fdb[~fdb.is_play_action])}
out["play_action"]["pa_air_yards"] = r(fdb[fdb.is_play_action & (fdb.pass_attempt == 1)].air_yards.mean(), 2)
out["play_action"]["no_pa_air_yards"] = r(fdb[~fdb.is_play_action & (fdb.pass_attempt == 1)].air_yards.mean(), 2)
# Does play-action need a working run game? Split teams by their designed-run EPA that season.
tr = f[(f.play_type == "run") & (f.qb_scramble == 0)].groupby(["season", "posteam"]).epa.mean()
q = pd.qcut(tr, 4, labels=["worst_run_quarter", "q2", "q3", "best_run_quarter"])
fdb = fdb.join(q.rename("runq"), on=["season", "posteam"])
gap = {}
for k, g in fdb.groupby("runq", observed=True):
    a, b = g[g.is_play_action].epa.mean(), g[~g.is_play_action].epa.mean()
    gap[str(k)] = {"pa_epa": r(a), "no_pa_epa": r(b), "pa_gain": r(a - b), "n_pa": int(g.is_play_action.sum())}
out["play_action"]["gain_by_team_run_game"] = gap
# Same for the run game within a game: rushing success so far that game is too noisy; season split is the standard test.
out["play_action"]["note"] = "Gain = EPA per dropback with play-action minus without. Teams split into quarters by their designed-run EPA that season."
flags = {}
for c in ["is_rpo", "is_screen_pass", "is_motion", "is_no_huddle", "is_qb_out_of_pocket"]:
    flags[c] = {"rate_of_dropbacks": r(fdb[c].mean()), "with": pass_split(fdb[fdb[c]]), "without": pass_split(fdb[~fdb[c]])}
bl = fdb.n_blitzers.fillna(0)
flags["blitz_any"] = {"rate_of_dropbacks": r((bl > 0).mean()), "with": pass_split(fdb[bl > 0]), "without": pass_split(fdb[bl == 0])}
out["ftn_flags"] = flags
out["ftn_flags"]["interception_worthy_rate"] = r(fdb.is_interception_worthy.mean())
out["ftn_flags"]["drop_rate_of_catchable"] = r(fdb[fdb.is_catchable_ball].is_drop.mean())
out["ftn_flags"]["qb_fault_sack_share"] = r(fdb[fdb.sack == 1].is_qb_fault_sack.mean())
fr = f[(f.play_type == "run") & (f.qb_scramble == 0)]
out["ftn_flags"]["runs_with_motion"] = {"rate": r(fr.is_motion.mean()), "with": run_split(fr[fr.is_motion].assign(a=1), "a", [1]),
                                        "without": run_split(fr[~fr.is_motion].assign(a=1), "a", [1])}

# ---------- scheme mix, 2025 ----------
s25 = pbp[(pbp.season == 2025) & pbp.play_type.isin(["pass", "run"]) & (pbp.qb_kneel == 0) & (pbp.qb_spike == 0)].merge(part, on="key").merge(ftn, on="key", how="left")


def pers(p):
    if not isinstance(p, str):
        return None
    d = {}
    for part_ in p.split(","):
        bits = part_.strip().split(" ")
        if len(bits) == 2:
            d[bits[1]] = int(bits[0])
    return f"{d.get('RB', 0)}{d.get('TE', 0)}"


s25["pers"] = s25.offense_personnel.map(pers)


def dpers(pos):
    if not isinstance(pos, str):
        return None
    p = pos.split(";")
    db_ = sum(x in ("CB", "S", "FS", "SS", "DB") for x in p)
    return {4: "base (4 DBs)", 5: "nickel (5)", 6: "dime (6)"}.get(db_, f"{db_} DBs")


s25["dpers"] = s25.defense_positions.map(dpers)
team_rate = lambda col, val: s25.groupby("posteam")[col].apply(lambda x: (x == val).mean())
mix = {"seasons": [2025],
       "offense_personnel": {k: r(v) for k, v in s25.pers.value_counts(normalize=True).head(8).items()},
       "formation": {k: r(v) for k, v in s25.offense_formation.value_counts(normalize=True).items() if v > 0.005},
       "qb_location_ftn": {k: r(v) for k, v in s25.qb_location.value_counts(normalize=True).items() if v > 0.005},
       "defense_personnel": {k: r(v) for k, v in s25.dpers.value_counts(normalize=True).items() if v > 0.005},
       "pass_rate_all_plays": r((s25.qb_dropback == 1).mean()),
       "shotgun_rate": r(s25.shotgun.mean()), "no_huddle_rate": r(s25.no_huddle.mean()),
       "motion_rate": r(s25.is_motion.mean()), "play_action_rate_dropbacks": r(s25[s25.qb_dropback == 1].is_play_action.mean()),
       "rpo_rate_all_plays": r(s25.is_rpo.mean()),
       "team_range": {
           "11_personnel": [r(team_rate("pers", "11").min()), r(team_rate("pers", "11").max())],
           "12_personnel": [r(team_rate("pers", "12").min()), r(team_rate("pers", "12").max())],
           "21_personnel": [r(team_rate("pers", "21").min()), r(team_rate("pers", "21").max())],
           "shotgun": [r(s25.groupby("posteam").shotgun.mean().min()), r(s25.groupby("posteam").shotgun.mean().max())],
           "play_action_dropbacks": [r(s25[s25.qb_dropback == 1].groupby("posteam").is_play_action.mean().min()),
                                     r(s25[s25.qb_dropback == 1].groupby("posteam").is_play_action.mean().max())],
           "motion": [r(s25.groupby("posteam").is_motion.mean().min()), r(s25.groupby("posteam").is_motion.mean().max())]}}
d25 = s25[s25.qb_dropback == 1]
cv25 = d25[d25.defense_coverage_type.notna() & (d25.defense_coverage_type != "")]
mix["defense_coverage"] = {k: r(v) for k, v in cv25.defense_coverage_type.value_counts(normalize=True).items() if v > 0.005}
mix["defense_man_share"] = r((d25.defense_man_zone_type == "MAN_COVERAGE").sum() / d25.defense_man_zone_type.isin(["MAN_COVERAGE", "ZONE_COVERAGE"]).sum())
mix["defense_blitz_rate"] = r((d25.n_blitzers.fillna(0) > 0).mean())
mix["defense_man_share_team_range"] = [r(x) for x in d25[d25.defense_man_zone_type.isin(["MAN_COVERAGE", "ZONE_COVERAGE"])].groupby("defteam").defense_man_zone_type.apply(lambda x: (x == "MAN_COVERAGE").mean()).agg(["min", "max"])]
mix["defense_blitz_team_range"] = [r(x) for x in d25.groupby("defteam").n_blitzers.apply(lambda x: (x.fillna(0) > 0).mean()).agg(["min", "max"])]
out["scheme_mix_nfl"] = mix

# ---------- snap shares and rotation ----------
sc = pd.concat([pd.read_parquet(f"{D}/snap_counts_{y}.parquet") for y in YEARS])
sc = sc[sc.game_type == "REG"]
groups = {"QB": ("offense", ["QB"], 1), "RB": ("offense", ["RB", "FB"], 3), "WR": ("offense", ["WR"], 5),
          "TE": ("offense", ["TE"], 3), "OL": ("offense", ["T", "G", "C", "OL"], 7),
          "DL_interior": ("defense", ["DT", "NT"], 4), "EDGE": ("defense", ["DE", "OLB"], 5),
          "LB_off_ball": ("defense", ["LB", "ILB", "MLB"], 4), "CB": ("defense", ["CB", "DB"], 5),
          "S": ("defense", ["S", "FS", "SS"], 4)}
snaps = {"note": "Per team game: players at the position sorted by their share of the unit's snaps; rank 1 = most snaps that game. 'players_10pct' = how many played at least 10% of snaps. PFR position labels: EDGE = DE + OLB (some 3-4 OLBs are off-ball), interior = DT + NT."}
for name, (side, pos, k) in groups.items():
    col = f"{side}_pct"
    g = sc[sc.position.isin(pos)]
    ranks = g.sort_values(col, ascending=False).groupby(["game_id", "team"])[col].apply(lambda x: list(x[:k]) + [0] * (k - len(x[:k])))
    arr = np.array(ranks.tolist())
    cnt = g[g[col] >= 0.10].groupby(["game_id", "team"]).size()
    snaps[name] = {"rank_mean": [r(x) for x in arr.mean(axis=0)], "players_10pct_mean": r(cnt.mean(), 2),
                   "rank1_p10_p90": [r(np.quantile(arr[:, 0], .1)), r(np.quantile(arr[:, 0], .9))]}
out["snap_shares"] = snaps

# Lead-back carry share, season level (team's RB carries).
pl = pd.read_parquet(f"{D}/players.parquet", columns=["gsis_id", "position"])
pos_of = dict(zip(pl.gsis_id, pl.position))
rr = pbp[(pbp.play_type == "run") & (pbp.qb_scramble == 0) & (pbp.qb_kneel == 0) & pbp.rusher_player_id.notna()].copy()
rr["pos"] = rr.rusher_player_id.map(pos_of)
rb = rr[rr.pos.isin(["RB", "FB"])]
sea = rb.groupby(["season", "posteam", "rusher_player_id"]).size().groupby(["season", "posteam"]).apply(lambda x: sorted(x, reverse=True))
lead = sea.map(lambda x: x[0] / sum(x))
second = sea.map(lambda x: (x[1] / sum(x)) if len(x) > 1 else 0)
gm = rb.groupby(["game_id", "posteam", "rusher_player_id"]).size().groupby(["game_id", "posteam"]).apply(lambda x: x.max() / x.sum())
out["rb_carry_share"] = {"season_lead_back": {"mean": r(lead.mean()), "p10": r(lead.quantile(.1)), "p90": r(lead.quantile(.9))},
                         "season_second_back": r(second.mean()),
                         "game_lead_back": {"mean": r(gm.mean()), "p10": r(gm.quantile(.1)), "p90": r(gm.quantile(.9))},
                         "qb_share_of_designed_runs": r((rr.pos == "QB").mean()),
                         "note": "Season shares include games the lead back missed, so they run lower than per-game shares."}
t = pbp[(pbp.pass_attempt == 1) & (pbp.sack == 0) & pbp.receiver_player_id.notna()].copy()
t["pos"] = t.receiver_player_id.map(pos_of)
ts = t.groupby(["season", "posteam", "receiver_player_id"]).size().groupby(["season", "posteam"]).apply(lambda x: sorted(x / x.sum(), reverse=True)[:3])
out["target_share_top3_season"] = [r(np.mean([x[i] for x in ts])) for i in range(3)]
out["target_share_by_position"] = {k: r(v) for k, v in t.pos.value_counts(normalize=True).items() if v > 0.01}

# ---------- red zone, returns ----------
dr = pbp[pbp.fixed_drive_result.notna()].groupby(["game_id", "posteam", "fixed_drive"]).agg(minyl=("yardline_100", "min"), res=("fixed_drive_result", "first"))
rz = dr[dr.minyl <= 20]
ko = pbp[(pbp.kickoff_attempt == 1) & pbp.kickoff_returner_player_id.notna()]
pu = pbp[(pbp.punt_attempt == 1) & pbp.punt_returner_player_id.notna()]
out["red_zone_and_returns"] = {
    "red_zone_td_rate_per_trip": r((rz.res == "Touchdown").mean()), "red_zone_score_rate": r(rz.res.isin(["Touchdown", "Field goal"]).mean()),
    "red_zone_trips_per_team_game": r(len(rz) / pbp.groupby(["game_id", "posteam"]).ngroups, 2),
    "kick_return_avg_by_season": {str(s): r(g.return_yards.mean(), 1) for s, g in ko.groupby("season")},
    "kick_return_rate_by_season": {str(s): r(len(ko[ko.season == s]) / len(g), 3) for s, g in pbp[pbp.kickoff_attempt == 1].groupby("season")},
    "punt_return_avg": r(pu.return_yards.mean(), 1),
    "punt_return_rate": r(len(pu) / (pbp.punt_attempt == 1).sum()),
    "note": "A red-zone trip = a drive that ran at least one play at or inside the 20 (Pro-Football-Reference counts the same way). 2025 kickoffs use the new dynamic kickoff rule."}

# ---------- position value: how much players at each spot really differ ----------
qb = pbp[(pbp.qb_dropback == 1) & pbp.passer_player_id.notna()].groupby(["season", "passer_player_id"]).agg(n=("epa", "size"), epa=("epa", "mean"))
qb = qb[qb.n >= 250]
rbs = rb.groupby(["season", "rusher_player_id"]).agg(n=("epa", "size"), epa=("epa", "mean"), suc=("success", "mean"))
rbs = rbs[rbs.n >= 120]
wr = t[t.pos.isin(["WR", "TE"])].groupby(["season", "receiver_player_id"]).agg(n=("epa", "size"), epa=("epa", "mean"))
wr = wr[wr.n >= 60]
team_db = pbp[pbp.qb_dropback == 1].groupby(["game_id", "posteam"]).size().mean()
team_rush = rb.groupby(["game_id", "posteam"]).size().mean()
out["player_spread"] = {
    "note": "Spread (10th to 90th percentile) of per-play EPA among regular players, times the plays they touch in a game. Not causal: it mixes the player with his teammates and scheme. It shows why QB spread dwarfs RB spread.",
    "qb_epa_per_dropback_p10_p90": [r(qb.epa.quantile(.1)), r(qb.epa.quantile(.9))],
    "qb_points_per_game_p10_to_p90": r((qb.epa.quantile(.9) - qb.epa.quantile(.1)) * team_db, 2),
    "rb_epa_per_carry_p10_p90": [r(rbs.epa.quantile(.1)), r(rbs.epa.quantile(.9))],
    "rb_points_per_game_p10_to_p90_at_15_carries": r((rbs.epa.quantile(.9) - rbs.epa.quantile(.1)) * 15, 2),
    "wr_te_epa_per_target_p10_p90": [r(wr.epa.quantile(.1)), r(wr.epa.quantile(.9))],
    "wr_points_per_game_p10_to_p90_at_7_targets": r((wr.epa.quantile(.9) - wr.epa.quantile(.1)) * 7, 2),
    "team_dropbacks_per_game": r(team_db, 1), "team_rb_carries_per_game": r(team_rush, 1)}
# Year-to-year stability: how much of a player's per-play EPA carries over (signal vs noise).
def yoy(df):
    a = df.reset_index()
    a2 = a.assign(season=a.season + 1)
    m = a.merge(a2, on=["season", a.columns[1]], suffixes=("", "_prev"))
    return r(m.epa.corr(m.epa_prev)), int(len(m))
out["player_spread"]["year_to_year_corr"] = {"qb_epa_per_dropback": yoy(qb), "rb_epa_per_carry": yoy(rbs), "wr_te_epa_per_target": yoy(wr)}

# What predicts winning: team pass vs run efficiency.
tg_ = pbp[pbp.play_type.isin(["pass", "run"]) & (pbp.qb_kneel == 0)].copy()
tg_["is_pass"] = tg_.qb_dropback == 1
off = tg_.groupby(["season", "posteam", "is_pass"]).epa.mean().unstack()
off.columns = ["rush_epa", "pass_epa"]
dfn = tg_.groupby(["season", "defteam", "is_pass"]).epa.mean().unstack()
dfn.columns = ["def_rush_epa", "def_pass_epa"]
g = pd.read_csv(f"{D}/games.csv")
g = g[(g.game_type == "REG") & g.season.isin(YEARS) & g.result.notna()]
w = pd.concat([pd.DataFrame({"season": g.season, "team": g.home_team, "m": g.result}),
               pd.DataFrame({"season": g.season, "team": g.away_team, "m": -g.result})]).groupby(["season", "team"]).m.mean()
tm = off.join(dfn.rename_axis(["season", "posteam"])).join(w.rename_axis(["season", "posteam"]).rename("margin"))
out["what_wins"] = {"note": "Correlation of each team-season efficiency with point margin per game, 2021-25 (160 team seasons). Squared = share of margin explained alone.",
                    **{c: r(tm[c].corr(tm.margin)) for c in ["pass_epa", "rush_epa", "def_pass_epa", "def_rush_epa"]},
                    "sd_pass_epa": r(tm.pass_epa.std()), "sd_rush_epa": r(tm.rush_epa.std())}

# ---------- contracts: what teams pay by position ----------
c = pd.read_parquet(f"{D}/contracts.parquet", columns=["player", "position", "year_signed", "apy_cap_pct", "is_active"])
c = c[c.is_active & (c.year_signed >= 2022)]
pay = {}
for p_, gg in c.groupby("position"):
    s = gg.sort_values("apy_cap_pct", ascending=False).apy_cap_pct
    if len(s) >= 20:
        pay[p_] = {"top1": r(s.iloc[0]), "top5_avg": r(s.iloc[:5].mean()), "top20_avg": r(s.iloc[:20].mean()), "n": int(len(s))}
out["contracts_cap_pct"] = {"note": "Active contracts signed 2022+, average per year as a share of the cap when signed (OverTheCap via nflverse).",
                            **dict(sorted(pay.items(), key=lambda kv: -kv[1]["top5_avg"]))}

# ---------- home field ----------
ga = pd.read_csv(f"{D}/games.csv")
ga = ga[(ga.game_type == "REG") & ga.result.notna() & (ga.location == "Home") & (ga.season <= 2025)]
hf = {}
for s, gg in ga.groupby("season"):
    hf[str(s)] = {"home_win": r((gg.result > 0).mean() + 0.5 * (gg.result == 0).mean()), "margin": r(gg.result.mean(), 2),
                  "spread_implied": r(gg.spread_line.mean(), 2), "games": int(len(gg))}
spans = {}
for a, b in [(1999, 2009), (2010, 2019), (2021, 2025)]:
    gg = ga[ga.season.between(a, b)]
    spans[f"{a}-{b}"] = {"home_win": r((gg.result > 0).mean()), "margin": r(gg.result.mean(), 2), "spread_implied": r(gg.spread_line.mean(), 2)}
out["home_field_nfl"] = {"by_season": hf, "by_span": spans,
                         "note": "Regular season, neutral-site games removed. 2020 was played mostly without fans. spread_line is the closing betting line (home perspective), the market's estimate of the home edge plus team strength."}

json.dump(out, sys.stdout, indent=1)
