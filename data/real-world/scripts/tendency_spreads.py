"""Real NFL spreads for the player tendencies (docs/design/tendencies.md).

Each tendency is tied to one real stat. This script measures that stat per
player season (or per team season where the play-by-play can't name the
player) for NFL regulars, 2016-2025, and prints its spread: the 5th, 25th,
50th, 75th and 95th percentiles. On the 1-20 tendency scale a 4 sits about
at the 5th percentile and a 16 about at the 95th (NFL sd 3.2 around 10.5).

Reads nflverse release files (https://github.com/nflverse/nflverse-data/releases):
  pbp/play_by_play_YEAR.parquet, nextgen_stats/ngs_passing.csv.gz,
  nextgen_stats/ngs_rushing.csv.gz, pfr_advstats/advstats_season_def.csv,
  players/players.csv

Usage: python3 tendency_spreads.py DIR > ../tendency_spreads.json
Regular seasons only. Development checks only, never in-game tuning.
"""
import json, sys
import numpy as np
import pandas as pd

D = sys.argv[1]
YEARS = list(range(2016, 2026))
# Kickoffs under the old rules only: from 2024 a kick into the end zone is a
# touchback to the 30 or 35 and the choice changed.
KO_YEARS = list(range(2016, 2024))
Q = [5, 25, 50, 75, 95]


def spread(s, n=3):
    s = pd.Series(s).dropna()
    return {"n": int(len(s)), **{f"p{q}": round(float(np.percentile(s, q)), n) for q in Q}}


cols = ["season", "season_type", "play_type", "qb_dropback", "qb_scramble", "qb_kneel", "id", "rusher_player_id",
        "run_gap", "run_location", "out_of_bounds", "kickoff_attempt", "kickoff_in_endzone", "touchback",
        "return_yards", "kickoff_returner_player_id", "posteam", "defteam", "punt_attempt", "punt_fair_catch",
        "punt_returner_player_id", "penalty_type", "penalty_player_id", "penalty_team",
        "forced_fumble_player_1_player_id", "forced_fumble_player_2_player_id", "solo_tackle_1_player_id",
        "solo_tackle_2_player_id", "assist_tackle_1_player_id", "assist_tackle_2_player_id",
        "tackle_with_assist_1_player_id", "tackle_with_assist_2_player_id", "tackle_for_loss_1_player_id",
        "tackle_for_loss_2_player_id", "game_id", "punt_in_endzone", "punt_out_of_bounds", "punt_downed",
        "punt_blocked", "qb_hit"]
pbp = pd.concat([pd.read_parquet(f"{D}/play_by_play_{y}.parquet", columns=cols) for y in YEARS])
pbp = pbp[pbp.season_type == "REG"]
players = pd.read_csv(f"{D}/players.csv", usecols=["gsis_id", "position"]).dropna()
pos_of = dict(zip(players.gsis_id, players.position))

out = {"source": "nflverse play-by-play, Next Gen Stats and PFR advanced defense, regular seasons 2016-2025",
       "note": "per player season among regulars unless a stat says team season; percentiles p5-p95",
       "stats": {}}
S = out["stats"]

# Scrambles: scrambles per dropback, quarterbacks with 250+ dropbacks.
db = pbp[pbp.qb_dropback == 1].groupby(["season", "id"]).agg(n=("qb_dropback", "size"), sc=("qb_scramble", "sum"))
db = db[db.n >= 250]
S["scramble_rate"] = {"tendency": "scrambles", "who": "QB, 250+ dropbacks", **spread(db.sc / db.n, 4)}

# Next Gen Stats passing (season rows, week 0): 200+ attempts.
ngp = pd.read_csv(f"{D}/ngs_passing.csv.gz")
ngp = ngp[(ngp.week == 0) & (ngp.season_type == "REG") & (ngp.attempts >= 200) & ngp.season.isin(YEARS)]
S["tight_window_pct"] = {"tendency": "gunslinger", "who": "QB, 200+ attempts",
                         "def": "attempts into a window under a yard (NGS aggressiveness)",
                         **spread(ngp.aggressiveness, 2)}
S["intended_air_yards"] = {"tendency": "gunslinger", "who": "QB, 200+ attempts", **spread(ngp.avg_intended_air_yards, 2)}
S["time_to_throw"] = {"tendency": "holds", "who": "QB, 200+ attempts", **spread(ngp.avg_time_to_throw, 3)}

# Next Gen Stats rushing: time behind the line, 100+ carries.
ngr = pd.read_csv(f"{D}/ngs_rushing.csv.gz")
ngr = ngr[(ngr.week == 0) & (ngr.season_type == "REG") & (ngr.rush_attempts >= 100) & ngr.season.isin(YEARS)]
S["time_to_los"] = {"tendency": "patient", "who": "RB, 100+ carries", **spread(ngr.avg_time_to_los, 3)}

# Designed runs by backs: share outside the tackles, and share ending out of bounds.
runs = pbp[(pbp.play_type == "run") & (pbp.qb_scramble == 0) & (pbp.qb_kneel == 0)].copy()
runs["pos"] = runs.rusher_player_id.map(pos_of)
rb = runs[runs.pos == "RB"].copy()
rb["outside"] = (rb.run_gap == "end").astype(float)
rb.loc[rb.run_location.isna(), "outside"] = np.nan
g = rb.groupby(["season", "rusher_player_id"]).agg(n=("outside", "size"), out=("outside", "mean"),
                                                  oob=("out_of_bounds", "mean"))
g = g[g.n >= 100]
S["outside_share"] = {"tendency": "bounces runs", "who": "RB, 100+ carries",
                      "def": "carries to the end (outside the tackles); the play-by-play has no called gap",
                      **spread(g.out, 3)}
S["run_oob_share"] = {"tendency": "contact willingness (low)", "who": "RB, 100+ carries",
                      "def": "carries ending out of bounds", **spread(g.oob, 4)}

# Kickoffs into the end zone brought out, per team season (touchbacks name no returner).
# The play-by-play marks a kick in the end zone only when it was brought out,
# so the deep kicks are those plus the touchbacks (some of which flew out of
# the end zone, so the share is a floor).
ko = pbp[(pbp.kickoff_attempt == 1) & ((pbp.kickoff_in_endzone == 1) | (pbp.touchback == 1))
         & pbp.season.isin(KO_YEARS)].copy()
ko["out"] = (ko.touchback == 0).astype(float)
k = ko.groupby(["season", "defteam"]).agg(n=("out", "size"), out=("out", "mean"))
k = k[k.n >= 15]
S["endzone_return_share"] = {"tendency": "returner risk", "who": "team season, 2016-2023 rules, 15+ kicks",
                             "def": "deep kicks (in the end zone or a touchback) brought out", **spread(k.out, 3)}

# Punts: fair catches per punt caught or fair-caught, per team season.
pu = pbp[(pbp.punt_attempt == 1) & (pbp.punt_blocked == 0) & (pbp.touchback == 0) & (pbp.punt_out_of_bounds == 0)
         & (pbp.punt_downed == 0)].copy()
p = pu.groupby(["season", "defteam"]).agg(n=("punt_fair_catch", "size"), fc=("punt_fair_catch", "mean"))
p = p[p.n >= 20]
S["fair_catch_rate"] = {"tendency": "returner risk (low)", "who": "team season, 20+ punts caught or fair-caught",
                        **spread(p.fc, 3)}

# Defenders: games, tackles, forced fumbles, tackles for loss, snap flags, from the play-by-play.
def ids(cols_):
    return pd.concat([pbp[["season", "game_id", c]].rename(columns={c: "pid"}) for c in cols_]).dropna()


tk = ids(["solo_tackle_1_player_id", "solo_tackle_2_player_id", "assist_tackle_1_player_id", "assist_tackle_2_player_id",
          "tackle_with_assist_1_player_id", "tackle_with_assist_2_player_id"])
games = tk.groupby(["season", "pid"]).game_id.nunique().rename("games")
tackles = tk.groupby(["season", "pid"]).size().rename("tackles")
ff = ids(["forced_fumble_player_1_player_id", "forced_fumble_player_2_player_id"]).groupby(["season", "pid"]).size().rename("ff")
tfl = ids(["tackle_for_loss_1_player_id", "tackle_for_loss_2_player_id"]).groupby(["season", "pid"]).size().rename("tfl")
snapflag = pbp[pbp.penalty_type.isin(["Defensive Offside", "Encroachment", "Neutral Zone Infraction"])]
sf = snapflag.groupby(["season", "penalty_player_id"]).size().rename("snapflags")
sf.index = sf.index.set_names(["season", "pid"])
dfn = pd.concat([games, tackles, ff, tfl, sf], axis=1).fillna(0)
dfn = dfn[dfn.games >= 10].reset_index()
dfn["pos"] = dfn.pid.map(pos_of)
front = dfn[dfn.pos.isin(["DE", "DT", "NT", "DL", "OLB"])]
S["snap_flags_per_17"] = {"tendency": "times the snap", "who": "DL and OLB, 10+ games with a tackle",
                          "def": "offside, encroachment and neutral zone flags per 17 games",
                          **spread(17 * front.snapflags / front.games, 2)}
big = dfn[dfn.tackles >= 40]
S["forced_fumbles_per_100_tackles"] = {"tendency": "goes for the strip", "who": "40+ tackles",
                                       **spread(100 * big.ff / big.tackles, 2)}
lbs = dfn[dfn.pos.isin(["LB", "ILB", "MLB", "OLB", "S", "SS", "FS"]) & (dfn.tackles >= 40)]
S["tfl_per_17"] = {"tendency": "flies downhill", "who": "LB and S, 40+ tackles", **spread(17 * lbs.tfl / lbs.games, 2)}

# PFR advanced defense: missed tackles, interceptions per target, batted balls.
adv = pd.read_csv(f"{D}/adv_def.csv")
adv = adv[adv.season.isin(YEARS)]
adv["m_tkl"] = pd.to_numeric(adv.m_tkl, errors="coerce")
adv["comb"] = pd.to_numeric(adv.comb, errors="coerce")
mt = adv[adv.comb >= 40]
S["missed_tackle_pct"] = {"tendency": "big hit (high) or sure tackle (low)", "who": "40+ combined tackles",
                          "def": "missed / (missed + combined)", **spread(mt.m_tkl / (mt.m_tkl + mt.comb), 3)}
adv["tgt"] = pd.to_numeric(adv.tgt, errors="coerce")
adv["int"] = pd.to_numeric(adv["int"], errors="coerce")
adv["yds_tgt"] = pd.to_numeric(adv.yds_tgt, errors="coerce")
cov = adv[adv.pos.isin(["CB", "S", "SS", "FS", "DB", "LCB", "RCB"]) & (adv.tgt >= 40)]
S["int_per_target"] = {"tendency": "jumps routes", "who": "CB and S, 40+ targets", **spread(cov["int"] / cov.tgt, 4)}
S["yards_per_target_allowed"] = {"tendency": "jumps routes (cost)", "who": "CB and S, 40+ targets",
                                 **spread(cov.yds_tgt, 2)}
adv["bats"] = pd.to_numeric(adv.bats, errors="coerce")
adv["g"] = pd.to_numeric(adv.g, errors="coerce")
dl = adv[adv.pos.isin(["DE", "DT", "NT", "DL", "LDE", "RDE", "LDT", "RDT"]) & (adv.g >= 10)]
S["batted_per_17"] = {"tendency": "gets his hands up", "who": "DL, 10+ games", "def": "batted passes per 17 games",
                      **spread(17 * dl.bats / dl.g, 2)}

json.dump(out, sys.stdout, indent=1)
print()
