"""Real NFL numbers for the A/B tests (afm/ab-tests/plan.md, "Real numbers to compare against").

The A/B tests ask whether a body number or a rating changes a player's stats
in the engine. This script measures the same things in real NFL play, per
player season, regular seasons 2018-2025, and writes ab_real.json:

- slopes: an outcome regressed on a body measure (height, weight, forty, cone,
  arm length, 10-yard split),
  weighted by the sample. "raw" is body only; "held" adds the draft slot
  (log of the overall pick, undrafted = 260 with an undrafted dummy) and season
  dummies, so talent is held about equal. 95% bands are cluster-robust by player.
- spreads: p5, p50, p95, mean, sd and n per player season, and sd_true, the sd
  with the expected sampling noise taken out.
- deltas: kickers late and close against the rest; OL false starts at home
  against on the road.
- means: the pooled league value of each spread key among the qualified rows.
- gaps: what no free file covers.

Reads nflverse release files (https://github.com/nflverse/nflverse-data/releases),
downloaded with curl into DIR when missing:
  pbp/play_by_play_YEAR.csv.gz, snap_counts/snap_counts_YEAR.csv,
  pfr_advstats/advstats_season_{def,pass,rush,rec}.csv,
  pfr_advstats/advstats_week_{def,pass,rush}_YEAR.csv (noise estimates),
  nextgen_stats/ngs_{passing,rushing,receiving}.csv.gz, combine/combine.csv,
  players/players.csv
and arm length, hand size, wingspan and the 10-yard split per player from
MockDraftable (mockdraftable.py caches DIR/mockdraftable.json; only the fitted
numbers are written here).

Usage: python3 ab_real.py [DIR] [OUT]
  DIR defaults to /tmp/claude-0/abt/nflverse, OUT to ../ab_real.json next to this script.
Regular seasons only. Checks for the engine's A/B tests, never in-game tuning.
"""
import json
import os
import subprocess
import sys

import numpy as np
import pandas as pd

D = sys.argv[1] if len(sys.argv) > 1 else "/tmp/claude-0/abt/nflverse"
OUT = sys.argv[2] if len(sys.argv) > 2 else os.path.normpath(
    os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "ab_real.json"))
os.makedirs(D, exist_ok=True)
BASE = "https://github.com/nflverse/nflverse-data/releases/download"
YEARS = list(range(2018, 2026))
# Kickoff rule eras: 2018-2022 the old kickoff; 2023 a fair catch inside the 25 went to the 25 (left out);
# 2024 the dynamic kickoff with the touchback at the 30; 2025 the touchback at the 35.
KO_ERAS = (("", list(range(2018, 2023))), ("_2024", [2024]), ("_2025", [2025]))
UNDRAFTED = 260


def fetch(tag, name):
    path = os.path.join(D, name)
    if not os.path.exists(path) or os.path.getsize(path) < 1000:
        subprocess.run(["curl", "-sSL", "-o", path, f"{BASE}/{tag}/{name}"], check=True)
    return path


def num(df, cols):
    for c in cols:
        df[c] = pd.to_numeric(df[c], errors="coerce")
    return df


def one_row(df):
    """PFR season files give a traded player one row per team and a '2TM' total; keep the total."""
    df = df.copy()
    tm = "tm" if "tm" in df else "team"
    df["_tot"] = df[tm].astype(str).str.contains("TM").astype(int)
    df = df.sort_values("_tot", ascending=False).drop_duplicates(["season", "pfr_id"])
    return df.drop(columns="_tot")


# ---------------------------------------------------------------- inputs
PBP_COLS = [
    "season", "season_type", "week", "game_id", "home_team", "posteam", "defteam", "play_type", "desc",
    "qb_dropback", "qb_scramble", "qb_kneel", "qb_spike", "sack", "pass_attempt", "complete_pass",
    "incomplete_pass", "interception", "pass_location", "air_yards", "yards_after_catch", "yards_gained",
    "passer_id", "receiver_player_id", "rusher_player_id", "down", "ydstogo", "yardline_100", "first_down",
    "touchdown", "td_player_id", "fumble", "fumbled_1_player_id", "fumbled_2_player_id",
    "forced_fumble_player_1_player_id", "forced_fumble_player_2_player_id", "solo_tackle_1_player_id",
    "solo_tackle_2_player_id", "assist_tackle_1_player_id", "assist_tackle_2_player_id",
    "assist_tackle_3_player_id", "assist_tackle_4_player_id", "tackle_with_assist_1_player_id",
    "tackle_with_assist_2_player_id", "pass_defense_1_player_id", "pass_defense_2_player_id",
    "penalty_type", "penalty_player_id", "penalty_team", "field_goal_attempt", "field_goal_result",
    "kick_distance", "kicker_player_id", "qtr", "score_differential", "kickoff_attempt", "touchback",
    "return_yards", "kickoff_returner_player_id", "kickoff_fair_catch", "punt_attempt", "punt_blocked",
    "punter_player_id", "punt_inside_twenty", "punt_returner_player_id", "punt_fair_catch", "cp",
    "two_point_attempt"]
pbp = pd.concat([pd.read_csv(fetch("pbp", f"play_by_play_{y}.csv.gz"), usecols=PBP_COLS, low_memory=False)
                 for y in YEARS])
pbp = pbp[pbp.season_type == "REG"].copy()
pbp["two_point_attempt"] = pbp.two_point_attempt.fillna(0)

snaps = pd.concat([pd.read_csv(fetch("snap_counts", f"snap_counts_{y}.csv")) for y in YEARS])
snaps = snaps[snaps.game_type == "REG"]
snap_season = snaps.groupby(["season", "pfr_player_id"]).agg(
    off=("offense_snaps", "sum"), dfn=("defense_snaps", "sum"), st=("st_snaps", "sum")).reset_index()
snap_season = snap_season.rename(columns={"pfr_player_id": "pfr_id"})

adv_def = one_row(pd.read_csv(fetch("pfr_advstats", "advstats_season_def.csv")))
adv_pass = one_row(pd.read_csv(fetch("pfr_advstats", "advstats_season_pass.csv")))
adv_rush = one_row(pd.read_csv(fetch("pfr_advstats", "advstats_season_rush.csv")))
adv_rec = one_row(pd.read_csv(fetch("pfr_advstats", "advstats_season_rec.csv")))
for a in (adv_def, adv_pass, adv_rush, adv_rec):
    a.drop(a.index[~a.season.isin(YEARS)], inplace=True)
num(adv_def, ["g", "int", "tgt", "cmp", "yds", "td", "rat", "dadot", "yac", "prss", "sk", "comb", "m_tkl", "hrry", "qbkd"])
num(adv_pass, ["pass_attempts", "throwaways", "spikes", "bad_throws", "batted_balls", "times_pressured",
               "pa_pass_att", "pa_pass_yards"])
num(adv_rush, ["att", "ybc", "yac", "brk_tkl", "ybc_att", "yac_att"])
num(adv_rec, ["tgt", "rec", "yac", "yac_r", "brk_tkl", "drop", "adot"])


def week(kind):
    w = pd.concat([pd.read_csv(fetch("pfr_advstats", f"advstats_week_{kind}_{y}.csv")) for y in YEARS])
    return w[w.game_type == "REG"].rename(columns={"pfr_player_id": "pfr_id"})


wk_def, wk_pass, wk_rush = week("def"), week("pass"), week("rush")

ngs = {}
for k in ("passing", "rushing", "receiving"):
    g = pd.read_csv(fetch("nextgen_stats", f"ngs_{k}.csv.gz"))
    g = g[(g.season_type == "REG") & g.season.isin(YEARS)].rename(columns={"player_gsis_id": "gsis_id"})
    ngs[k] = (g[g.week == 0].copy(), g[g.week > 0].copy())

players = pd.read_csv(fetch("players", "players.csv"), low_memory=False)
combine = pd.read_csv(fetch("combine", "combine.csv"))

# ---------------------------------------------------------------- people: ids, position group, body, draft slot
PFF = {"QB": "QB", "HB": "RB", "FB": "FB", "WR": "WR", "TE": "TE", "T": "OT", "G": "IOL", "C": "IOL",
       "DI": "DT", "ED": "EDGE", "LB": "LB", "CB": "CB", "S": "S", "K": "K", "P": "P", "LS": "LS"}
NGSP = {"QB": "QB", "RB": "RB", "FB": "FB", "WR": "WR", "SLOT_WR": "WR", "TE": "TE", "T": "OT", "G": "IOL",
        "C": "IOL", "INTERIOR_LINE": "DT", "EDGE": "EDGE", "MLB": "LB", "OLB": "LB", "ILB": "LB", "CB": "CB",
        "SLOT_CB": "CB", "SAFETY": "S", "HIGH_SAFETY": "S", "K": "K", "P": "P"}
ROSTER = {"QB": "QB", "RB": "RB", "FB": "FB", "WR": "WR", "TE": "TE", "OT": "OT", "T": "OT", "G": "IOL",
          "OG": "IOL", "C": "IOL", "OL": "IOL", "DE": "EDGE", "EDGE": "EDGE", "DT": "DT", "NT": "DT", "DL": "DT",
          "LB": "LB", "ILB": "LB", "MLB": "LB", "OLB": "LB", "CB": "CB", "DB": "CB", "S": "S", "SAF": "S",
          "FS": "S", "SS": "S", "K": "K", "P": "P", "LS": "LS"}

pl = players[["gsis_id", "pfr_id", "pff_position", "ngs_position", "position", "height", "weight",
              "draft_pick", "rookie_season"]].copy()
pl["group"] = pl.pff_position.map(PFF)
pl["group"] = pl.group.fillna(pl.ngs_position.map(NGSP)).fillna(pl.position.map(ROSTER))
cmb = combine[combine.pfr_id.notna()].drop_duplicates("pfr_id").copy()
hft = cmb.ht.str.split("-", expand=True).astype(float)
cmb["ht_in"] = hft[0] * 12 + hft[1]
cmb = cmb.rename(columns={"wt": "wt_lb", "pos": "combine_pos"})
cmb["slot_c"] = cmb.draft_ovr
cmb.loc[cmb.draft_ovr.isna() & (cmb.season < 2026), "slot_c"] = UNDRAFTED
people = pl.merge(cmb[["pfr_id", "ht_in", "wt_lb", "forty", "cone", "shuttle", "slot_c", "combine_pos"]],
                  on="pfr_id", how="outer")
people["group"] = people.group.fillna(people.combine_pos.map(ROSTER))
people["in_combine"] = people.ht_in.notna() | people.forty.notna()
people["ht"] = people.ht_in.fillna(people.height)
people["wt"] = people.wt_lb.fillna(people.weight)
people["slot"] = people.slot_c.fillna(people.draft_pick)
people.loc[people.slot.isna() & people.rookie_season.notna(), "slot"] = UNDRAFTED
people["undrafted"] = (people.slot >= UNDRAFTED).astype(float)
people.loc[people.slot.isna(), "undrafted"] = np.nan
# Arms and splits: MockDraftable (mockdraftable.py caches it), linked by name and draft class to the
# combine's pfr_id, then players.csv for the rest. A name twice in one class is left out.
MD_PATH = os.path.join(D, "mockdraftable.json")
if not os.path.exists(MD_PATH):
    subprocess.run([sys.executable, os.path.join(os.path.dirname(os.path.abspath(__file__)), "mockdraftable.py"), D],
                   check=True)
SUFFIX = {"jr", "sr", "ii", "iii", "iv", "v"}


def norm(name):
    w = "".join(ch if ch.isalpha() or ch == " " else "" for ch in str(name).lower().replace("-", " ")).split()
    return " ".join(x for x in w if x not in SUFFIX)


md = pd.DataFrame(json.load(open(MD_PATH))["players"])
md["key"] = md.name.map(norm)
md = md[~md.duplicated(["key", "draft"], keep=False)]
c_ids = combine[combine.pfr_id.notna()][["pfr_id", "player_name", "season"]].copy()
c_ids["key"] = c_ids.player_name.map(norm)
c_ids = c_ids[~c_ids.duplicated(["key", "season"], keep=False)]
p_ids = players[players.pfr_id.notna()][["pfr_id", "display_name", "draft_year", "rookie_season"]].copy()
p_ids["key"] = p_ids.display_name.map(norm)
p_ids["draft"] = p_ids.draft_year.fillna(p_ids.rookie_season)
p_ids = p_ids[p_ids.draft.notna() & ~p_ids.duplicated(["key", "draft"], keep=False)]
md = md.merge(c_ids.rename(columns={"season": "draft"})[["key", "draft", "pfr_id"]], on=["key", "draft"], how="left")
md = md.merge(p_ids[["key", "draft", "pfr_id"]].rename(columns={"pfr_id": "pfr_p"}), on=["key", "draft"], how="left")
md["pfr_id"] = md.pfr_id.fillna(md.pfr_p)
md = md[md.pfr_id.notna()].drop_duplicates("pfr_id")
# Splits from the combine only (source 1): pro day splits are hand-timed and run fast.
md["split"] = md.split10.where(md.split10_src == 1)
people = people.merge(md[["pfr_id", "arm", "split", "hand", "wingspan"]], on="pfr_id", how="left")
MD_LINK = {"mockdraftable players": int(len(json.load(open(MD_PATH))["players"])),
           "linked to a pfr_id": int(len(md)), "with arms": int(md.arm.notna().sum()),
           "with a combine 10-yard split": int(md.split.notna().sum())}
BODY = ["ht", "wt", "forty", "cone", "shuttle", "arm", "split", "slot", "undrafted", "group", "in_combine"]
by_gsis = people[people.gsis_id.notna()].drop_duplicates("gsis_id").set_index("gsis_id")
by_pfr = people[people.pfr_id.notna()].drop_duplicates("pfr_id").set_index("pfr_id")
gsis_to_pfr = by_gsis.pfr_id.dropna().to_dict()
group_of = by_gsis.group.to_dict()


LINKS = {}


def link(name, out):
    n = len(out)
    LINKS[name] = {"rows": n, "with_position": int(out.group.notna().sum()),
                   "with_combine": int(out.in_combine.fillna(False).astype(bool).sum())}
    if "pfr_id" in out:
        LINKS[name]["with_pfr_id"] = int(out.pfr_id.notna().sum())


def body_gsis(df, col="gsis_id", name=None):
    out = df.join(by_gsis[BODY + ["pfr_id"]], on=col)
    out["pid"] = out[col]
    if name:
        link(name, out)
    return out


def body_pfr(df, col="pfr_id", name=None):
    out = df.join(by_pfr[BODY], on=col)
    out["pid"] = out[col]
    if name:
        link(name, out)
    return out


# ---------------------------------------------------------------- statistics
def wls(d, x, y, w, held):
    d = d.dropna(subset=[x, y, w] + (["slot", "undrafted"] if held else []))
    d = d[d[w] > 0]
    cols = [np.ones(len(d)), d[x].to_numpy(float)]
    if held:
        cols += [np.log(d.slot.clip(lower=1).to_numpy(float)), d.undrafted.to_numpy(float)]
        for s in sorted(d.season.unique())[1:]:
            cols.append((d.season == s).to_numpy(float))
    X = np.column_stack(cols)
    Y = d[y].to_numpy(float)
    W = d[w].to_numpy(float)
    XtW = X.T * W
    A = np.linalg.pinv(XtW @ X)
    b = A @ (XtW @ Y)
    e = Y - X @ b
    u = X * (W * e)[:, None]
    g = pd.DataFrame(u).groupby(d.pid.to_numpy()).sum().to_numpy()
    G, N, K = len(g), len(d), X.shape[1]
    meat = g.T @ g * (G / (G - 1)) * ((N - 1) / (N - K))
    se = float(np.sqrt((A @ meat @ A)[1, 1]))
    return {"b": float(b[1]), "lo": float(b[1] - 1.96 * se), "hi": float(b[1] + 1.96 * se)}, N, d


UNITS = {"ht": ("height", "per +1 inch", 1.0), "wt": ("weight", "per +10 lb", 0.1),
         "forty": ("forty", "per +0.1 s", 10.0), "cone": ("3-cone", "per +0.1 s", 10.0),
         "shuttle": ("shuttle", "per +0.1 s", 10.0), "arm": ("arm length", "per +1 inch", 1.0),
         "split": ("10-yard split", "per +0.1 s", 10.0)}
SLOPES = {}


def r(v, n=4):
    return None if v is None or not np.isfinite(v) else round(float(v), n)


def slope(key, d, xb, y, w, min_sample, source, ylabel):
    d = d.copy()
    d["_x"] = d[xb] * UNITS[xb][2]
    qualified = len(d.dropna(subset=[y]))
    raw, n, used = wls(d, "_x", y, w, False)
    held, nh, _ = wls(d, "_x", y, w, True)
    mean_y = float(np.average(used[y], weights=used[w]))
    SLOPES[key] = {"x": UNITS[xb][0], "y": ylabel, "unit": UNITS[xb][1],
                   "raw": {k: r(v, 5) for k, v in raw.items()}, "held": {k: r(v, 5) for k, v in held.items()},
                   "n": int(n), "n_held": int(nh), "n_qualified": int(qualified), "players": int(used.pid.nunique()),
                   "mean_y": r(mean_y, 4), "min_sample": min_sample, "source": source}


SPREADS, MEANS = {}, {}


def spread(key, v, noise_var, min_sample, source, pooled, noise, nd=4):
    v = pd.Series(v, dtype=float)
    nv = pd.Series(noise_var, dtype=float).reindex(v.index)
    ok = v.notna() & np.isfinite(v)
    v, nv = v[ok], nv[ok]
    var = float(v.var(ddof=1))
    mnv = float(nv.mean()) if nv.notna().any() else np.nan
    sd_true = float(np.sqrt(max(0.0, var - mnv))) if np.isfinite(mnv) else None
    SPREADS[key] = {"p5": r(np.percentile(v, 5), nd), "p50": r(np.percentile(v, 50), nd),
                    "p95": r(np.percentile(v, 95), nd), "mean": r(v.mean(), nd), "sd": r(np.sqrt(var), nd),
                    "n": int(len(v)), "sd_true": r(sd_true, nd), "noise_sd": r(np.sqrt(mnv), nd) if np.isfinite(mnv) else None,
                    "noise": noise, "min_sample": min_sample, "source": source}
    MEANS[key] = r(pooled, nd)


def binom(p, n):
    return p * (1 - p) / n


def week_var(wk, key, n_col, x_col):
    """Per-unit variance from game rows: sum n_g (x_g - season mean)^2 / (games - 1).
    Game-to-game changes in true level count as noise too, so sd_true is a floor."""
    w = wk[[*key, n_col, x_col]].dropna()
    w = w[w[n_col] > 0].copy()
    w["nx"] = w[n_col] * w[x_col]
    s = w.groupby(key).agg(n=(n_col, "sum"), nx=("nx", "sum"), games=(n_col, "size"))
    w = w.join(s.nx.div(s.n).rename("m"), on=key)
    w["ss"] = w[n_col] * (w[x_col] - w.m) ** 2
    s["ss"] = w.groupby(key).ss.sum()
    s["unit_var"] = s.ss / (s.games - 1).where(s.games > 1)
    return s.unit_var


def ids(df, cols):
    return pd.concat([df[["season", c]].rename(columns={c: "gsis_id"}) for c in cols]).dropna()


# ---------------------------------------------------------------- quarterbacks
real = pbp[pbp.play_type.isin(["pass", "run"]) & (pbp.two_point_attempt == 0)]
db = real[real.qb_dropback == 1].copy()
db["att"] = ((db.pass_attempt == 1) & (db.sack == 0) & (db.qb_spike.fillna(0) == 0)).astype(float)
db["cmp"] = db.complete_pass * db.att
db["int_"] = db.interception * db.att
db["scr_yds"] = db.yards_gained.where(db.qb_scramble == 1, 0.0)
db["has_air"] = (db.att == 1) & db.air_yards.notna()
db["air"] = db.air_yards.where(db.has_air, 0.0)
db["air2"] = db.air ** 2
db["deep"] = (db.has_air & (db.air_yards >= 20)).astype(float)
db["mid_att"] = (db.att == 1) & (db.pass_location == "middle")
db["mid_cmp"] = db.mid_att & (db.complete_pass == 1)
db["cpv"] = (db.cp * (1 - db.cp)).where(db.att == 1, 0.0).fillna(0)
qb = db.groupby(["season", "passer_id"]).agg(
    dropbacks=("qb_dropback", "size"), scr=("qb_scramble", "sum"), scr_yds=("scr_yds", "sum"),
    scr_yds2=("scr_yds", lambda s: (s ** 2).sum()), sacks=("sack", "sum"), att=("att", "sum"), cmp=("cmp", "sum"),
    ints=("int_", "sum"), air_n=("has_air", "sum"), air=("air", "sum"), air2=("air2", "sum"), deep=("deep", "sum"),
    mid_att=("mid_att", "sum"), mid_cmp=("mid_cmp", "sum")).reset_index().rename(columns={"passer_id": "gsis_id"})
qb = body_gsis(qb, name="play-by-play passer seasons (gsis_id)")
qb = qb[(qb.group == "QB") & (qb.dropbacks >= 150)].copy()
qb["scramble_rate"] = qb.scr / qb.dropbacks
qb["yds_per_scramble"] = (qb.scr_yds / qb.scr).where(qb.scr >= 10)
qb["sack_rate"] = qb.sacks / qb.dropbacks
qb["mid_cmp_pct"] = (qb.mid_cmp / qb.mid_att).where(qb.mid_att >= 20)
qb["adot"] = qb.air / qb.air_n
qb["deep_share"] = qb.deep / qb.air_n
qb["int_rate"] = qb.ints / qb.att
SRC_PBP = "nflverse play-by-play, regular seasons 2018-2025"
MS_QB = "QB, 150+ dropbacks"
slope("qb_forty__scramble_rate", qb, "forty", "scramble_rate", "dropbacks", MS_QB, SRC_PBP + " (qb_scramble / qb_dropback)", "scrambles a dropback")
slope("qb_forty__yds_per_scramble", qb, "forty", "yds_per_scramble", "scr", MS_QB + " and 10+ scrambles", SRC_PBP, "yards a scramble")
slope("qb_forty__sack_rate", qb, "forty", "sack_rate", "dropbacks", MS_QB, SRC_PBP, "sacks a dropback")
slope("qb_ht__mid_cmp_pct", qb, "ht", "mid_cmp_pct", "mid_att", MS_QB + " and 20+ middle attempts",
      SRC_PBP + " (pass_location middle)", "completion rate, middle of the field")

ap = body_pfr(adv_pass, name="PFR passing seasons (pfr_id)")
ap = ap[(ap.group == "QB") & (ap.pass_attempts >= 150)].copy()
ap["batted_per_att"] = ap.batted_balls / ap.pass_attempts
slope("qb_ht__batted_per_att", ap, "ht", "batted_per_att", "pass_attempts", "QB, 150+ attempts",
      "PFR advanced passing (batted_balls / pass_attempts), 2018-2025", "batted balls an attempt")

spread("qb_scramble_rate", qb.scramble_rate, binom(qb.scramble_rate, qb.dropbacks), MS_QB, SRC_PBP,
       qb.scr.sum() / qb.dropbacks.sum(), "binomial")
qv = qb.air2 / qb.air_n - qb.adot ** 2
spread("qb_adot", qb.adot, qv / qb.air_n, MS_QB, SRC_PBP + " (air_yards on attempts)", qb.air.sum() / qb.air_n.sum(),
       "per throw", 2)
spread("qb_deep_share", qb.deep_share, binom(qb.deep_share, qb.air_n), MS_QB, SRC_PBP + " (air_yards >= 20)",
       qb.deep.sum() / qb.air_n.sum(), "binomial")
spread("qb_int_rate", qb.int_rate, binom(qb.int_rate, qb.att), MS_QB, SRC_PBP, qb.ints.sum() / qb.att.sum(), "binomial")
spread("qb_sack_rate", qb.sack_rate, binom(qb.sack_rate, qb.dropbacks), MS_QB, SRC_PBP,
       qb.sacks.sum() / qb.dropbacks.sum(), "binomial")

ngp, ngp_w = ngs["passing"]
ngp = ngp[ngp.attempts >= 150].copy()
for col, key, nd in (("completion_percentage_above_expectation", "qb_cpoe", 2), ("avg_time_to_throw", "qb_ttt", 3)):
    uv = week_var(ngp_w, ["season", "gsis_id"], "attempts", col)
    nv = ngp.join(uv.rename("uv"), on=["season", "gsis_id"]).uv / ngp.attempts
    spread(key, ngp[col], nv, "QB, 150+ attempts (NGS season row)", "Next Gen Stats passing, 2018-2025",
           np.average(ngp[col], weights=ngp.attempts), "per throw, from game rows", nd)
SPREADS["qb_cpoe"]["unit"] = "percentage points"

wp = wk_pass.groupby(["season", "pfr_id"]).agg(w_sacked=("times_sacked", "sum"), w_press=("times_pressured", "sum"))
ap = ap.join(wp, on=["season", "pfr_id"])
spp = ap[ap.w_press >= 30]
v = spp.w_sacked / spp.w_press
spread("qb_sacks_per_pressure", v, binom(v, spp.w_press), "QB, 150+ attempts and 30+ pressures",
       "PFR advanced passing, game rows (times_sacked / times_pressured)", spp.w_sacked.sum() / spp.w_press.sum(), "binomial")
v = ap.throwaways / ap.pass_attempts
spread("qb_throwaway_rate", v, binom(v, ap.pass_attempts), "QB, 150+ attempts", "PFR advanced passing (throwaways / attempts)",
       ap.throwaways.sum() / ap.pass_attempts.sum(), "binomial")
den = ap.pass_attempts - ap.throwaways - ap.spikes
v = ap.bad_throws / den
spread("qb_bad_throw_pct", v, binom(v, den), "QB, 150+ attempts",
       "PFR advanced passing (bad_throws / (attempts - throwaways - spikes))", ap.bad_throws.sum() / den.sum(), "binomial")
pa = ap[ap.pa_pass_att >= 40].copy()
pa["pa_ypa"] = pa.pa_pass_yards / pa.pa_pass_att
# Per-attempt variance of passing yards (0 on incompletions) from the play-by-play, league-wide.
pyd = db[db.att == 1].yards_gained.where(db[db.att == 1].complete_pass == 1, 0.0)
spread("qb_pa_ypa", pa.pa_ypa, float(pyd.var()) / pa.pa_pass_att, "QB, 150+ attempts and 40+ play-action attempts",
       "PFR advanced passing (pa_pass_yards / pa_pass_att), 2019-2023 only (the file has no play action for 2018, 2024, 2025)",
       pa.pa_pass_yards.sum() / pa.pa_pass_att.sum(), "per throw, league-wide variance", 2)

# Team pressure and sack rates (team season).
tdb = db.groupby(["season", "posteam"]).agg(dropbacks=("qb_dropback", "size"), sacks=("sack", "sum"))
tp = wk_pass.groupby(["season", "team"]).times_pressured.sum()
tp.index = tp.index.set_names(["season", "posteam"])
tdb = tdb.join(tp)
v = tdb.sacks / tdb.dropbacks
spread("team_sack_rate", v, binom(v, tdb.dropbacks), "team season, all", SRC_PBP, tdb.sacks.sum() / tdb.dropbacks.sum(), "binomial")
tq = tdb[tdb.times_pressured.notna()]
v = tq.times_pressured / tq.dropbacks
spread("team_pressure_rate", v, binom(v, tq.dropbacks), "team season, all",
       "PFR advanced passing game rows (times_pressured summed per team) over play-by-play dropbacks",
       tq.times_pressured.sum() / tq.dropbacks.sum(), "binomial")
TEAM_LINK = {"team seasons": int(len(tdb)), "with pressures": int(len(tq))}

# ---------------------------------------------------------------- running backs
runs = real[(real.play_type == "run") & (real.qb_scramble == 0) & (real.qb_kneel.fillna(0) == 0)
            & real.rusher_player_id.notna()].copy()
runs["y"] = runs.yards_gained
runs["y2"] = runs.y ** 2
runs["run20"] = (runs.y >= 20).astype(float)
runs["sy"] = (runs.down.isin([3, 4]) & (runs.ydstogo == 1)).astype(float)
runs["sy_conv"] = runs.sy * ((runs.first_down == 1) | (runs.touchdown == 1)).astype(float)
runs["fum"] = ((runs.fumbled_1_player_id == runs.rusher_player_id) | (runs.fumbled_2_player_id == runs.rusher_player_id)).astype(float)
rb = runs.groupby(["season", "rusher_player_id"]).agg(
    rushes=("y", "size"), yds=("y", "sum"), y2=("y2", "sum"), run20=("run20", "sum"), sy=("sy", "sum"),
    sy_conv=("sy_conv", "sum"), fum_r=("fum", "sum")).reset_index().rename(columns={"rusher_player_id": "gsis_id"})
tg = real[(real.play_type == "pass") & (real.pass_attempt == 1) & (real.sack == 0) & real.receiver_player_id.notna()].copy()
rec = tg[tg.complete_pass == 1].copy()
rec["fum"] = ((rec.fumbled_1_player_id == rec.receiver_player_id) | (rec.fumbled_2_player_id == rec.receiver_player_id)).astype(float)
rcv = rec.groupby(["season", "receiver_player_id"]).agg(recs=("fum", "size"), fum_c=("fum", "sum"))
rcv.index = rcv.index.set_names(["season", "gsis_id"])
rb = rb.join(rcv, on=["season", "gsis_id"]).fillna({"recs": 0, "fum_c": 0})
rb = body_gsis(rb, name="play-by-play rusher seasons (gsis_id)")
rb = rb[(rb.group == "RB") & (rb.rushes >= 100)].copy()
rb["touches"] = rb.rushes + rb.recs
rb["fumble_per_touch"] = (rb.fum_r + rb.fum_c) / rb.touches
rb["run20_rate"] = rb.run20 / rb.rushes
rb["short_yd_conv"] = (rb.sy_conv / rb.sy).where(rb.sy >= 3)
rb["ypc"] = rb.yds / rb.rushes
MS_RB = "RB, 100+ designed rushes"
slope("rb_wt__short_yd_conv", rb, "wt", "short_yd_conv", "sy", MS_RB + " and 3+ carries on 3rd or 4th and 1",
      SRC_PBP + " (first down or touchdown)", "conversion rate on 3rd and 4th and 1")
slope("rb_wt__run20_rate", rb, "wt", "run20_rate", "rushes", MS_RB, SRC_PBP, "rushes of 20+ yards a carry")
slope("rb_wt__fumble_per_touch", rb, "wt", "fumble_per_touch", "touches", MS_RB,
      SRC_PBP + " (fumbles on his carries and catches, lost or not)", "fumbles a touch")
slope("rb_forty__run20_rate", rb, "forty", "run20_rate", "rushes", MS_RB, SRC_PBP, "rushes of 20+ yards a carry")

ar = body_pfr(adv_rush, name="PFR rushing seasons (pfr_id)")
ar = ar[(ar.group == "RB") & (ar.att >= 100)].copy()
ar["yaco_per_att"] = ar.yac / ar.att
ar["ybc_per_att"] = ar.ybc / ar.att
ar["brk_tkl_per_att"] = ar.brk_tkl / ar.att
SRC_RUSH = "PFR advanced rushing, 2018-2025"
MS_AR = "RB, 100+ rushes (PFR)"
slope("rb_wt__yaco_per_att", ar, "wt", "yaco_per_att", "att", MS_AR, SRC_RUSH, "yards after contact a carry")
slope("rb_wt__brk_tkl_per_att", ar, "wt", "brk_tkl_per_att", "att", MS_AR, SRC_RUSH, "broken tackles a carry")
slope("rb_wt__ybc_per_att", ar, "wt", "ybc_per_att", "att", MS_AR, SRC_RUSH, "yards before contact a carry")
slope("rb_forty__ybc_per_att", ar, "forty", "ybc_per_att", "att", MS_AR, SRC_RUSH, "yards before contact a carry")
slope("rb_cone__ybc_per_att", ar, "cone", "ybc_per_att", "att", MS_AR, SRC_RUSH, "yards before contact a carry")
slope("rb_cone__brk_tkl_per_att", ar, "cone", "brk_tkl_per_att", "att", MS_AR, SRC_RUSH, "broken tackles a carry")

ngr, ngr_w = ngs["rushing"]
ngr = body_gsis(ngr[ngr.rush_attempts >= 100].copy(), name="NGS rushing season rows, 100+ rushes (gsis_id)")
ngr = ngr[ngr.group == "RB"].copy()
slope("rb_forty__ryoe_per_att", ngr, "forty", "rush_yards_over_expected_per_att", "rush_attempts",
      "RB, 100+ rushes (NGS season row)", "Next Gen Stats rushing, 2018-2025", "rush yards over expected a carry")

uv = week_var(ngr_w, ["season", "gsis_id"], "rush_attempts", "rush_yards_over_expected_per_att")
nv = ngr.join(uv.rename("uv"), on=["season", "gsis_id"]).uv / ngr.rush_attempts
spread("rb_ryoe_per_att", ngr.rush_yards_over_expected_per_att, nv, "RB, 100+ rushes (NGS season row)",
       "Next Gen Stats rushing", np.average(ngr.rush_yards_over_expected_per_att, weights=ngr.rush_attempts),
       "per carry, from game rows", 3)
for col, key in (("yac", "rb_yaco_per_att"), ("ybc", "rb_ybc_per_att")):
    wcol = {"yac": "rushing_yards_after_contact_avg", "ybc": "rushing_yards_before_contact_avg"}[col]
    uv = week_var(wk_rush, ["season", "pfr_id"], "carries", wcol)
    nv = ar.join(uv.rename("uv"), on=["season", "pfr_id"]).uv / ar.att
    spread(key, ar[col] / ar.att, nv, MS_AR, SRC_RUSH, ar[col].sum() / ar.att.sum(), "per carry, from game rows", 3)
spread("rb_brk_tkl_per_att", ar.brk_tkl_per_att, ar.brk_tkl_per_att / ar.att, MS_AR, SRC_RUSH,
       ar.brk_tkl.sum() / ar.att.sum(), "Poisson")
spread("rb_fumble_per_touch", rb.fumble_per_touch, binom(rb.fumble_per_touch, rb.touches), MS_RB, SRC_PBP,
       (rb.fum_r + rb.fum_c).sum() / rb.touches.sum(), "binomial")
spread("rb_ypc", rb.ypc, (rb.y2 / rb.rushes - rb.ypc ** 2) / rb.rushes, MS_RB, SRC_PBP + " (designed runs)",
       rb.yds.sum() / rb.rushes.sum(), "per carry", 3)

# ---------------------------------------------------------------- receivers and tight ends
tg["cmp"] = tg.complete_pass.astype(float)
tg["has_air"] = tg.air_yards.notna()
tg["air"] = tg.air_yards.fillna(0)
tg["air2"] = tg.air ** 2
tg["deep"] = (tg.air_yards >= 20).astype(float)
tg["deep_cmp"] = tg.deep * tg.cmp
tg["rz"] = (tg.yardline_100 <= 20).astype(float)
tg["rz_td"] = tg.rz * ((tg.touchdown == 1) & (tg.td_player_id == tg.receiver_player_id)).astype(float)
tg["yac_c"] = tg.yards_after_catch.where(tg.cmp == 1, 0.0).fillna(0)
tg["yac2"] = tg.yac_c ** 2
wr = tg.groupby(["season", "receiver_player_id"]).agg(
    tgt=("cmp", "size"), cmp=("cmp", "sum"), air_n=("has_air", "sum"), air=("air", "sum"), air2=("air2", "sum"),
    deep=("deep", "sum"), deep_cmp=("deep_cmp", "sum"), rz=("rz", "sum"), rz_td=("rz_td", "sum"),
    yac=("yac_c", "sum"), yac2=("yac2", "sum")).reset_index().rename(columns={"receiver_player_id": "gsis_id"})
wr = body_gsis(wr, name="play-by-play target seasons (gsis_id)")
wr = wr[wr.tgt >= 40].copy()
wr["catch_rate"] = wr.cmp / wr.tgt
wr["deep_catch_rate"] = (wr.deep_cmp / wr.deep).where(wr.deep >= 3)
wr["rz_td_rate"] = (wr.rz_td / wr.rz).where(wr.rz >= 3)
wr["adot"] = wr.air / wr.air_n
wr["deep_tgt_share"] = wr.deep / wr.air_n
MS_WR = "40+ targets"
for g, p in (("WR", "wr"), ("TE", "te")):
    s = wr[wr.group == g]
    slope(f"{p}_ht__rz_td_per_rz_tgt", s, "ht", "rz_td_rate", "rz", f"{g}, {MS_WR} and 3+ inside the 20",
          SRC_PBP + " (targets inside the 20, his touchdown)", "touchdowns a red zone target")
    slope(f"{p}_ht__catch_rate", s, "ht", "catch_rate", "tgt", f"{g}, {MS_WR}", SRC_PBP, "catches a target")
    slope(f"{p}_forty__adot", s, "forty", "adot", "air_n", f"{g}, {MS_WR}", SRC_PBP + " (air_yards on targets)",
          "air yards a target")
s = wr[wr.group == "WR"]
slope("wr_arm__catch_rate", s, "arm", "catch_rate", "tgt", "WR, " + MS_WR, SRC_PBP + "; arms from MockDraftable",
      "catches a target")
slope("wr_arm__deep_catch_rate", s, "arm", "deep_catch_rate", "deep", "WR, 40+ targets and 3+ deep",
      SRC_PBP + " (air_yards >= 20); arms from MockDraftable", "catches a deep target")
slope("wr_ht__deep_catch_rate", s, "ht", "deep_catch_rate", "deep", "WR, 40+ targets and 3+ deep",
      SRC_PBP + " (air_yards >= 20)", "catches a deep target")
slope("wr_forty__deep_tgt_share", s, "forty", "deep_tgt_share", "air_n", "WR, 40+ targets", SRC_PBP + " (air_yards >= 20)",
      "share of targets 20+ yards downfield")
for g, p in (("WR", "wr"), ("TE", "te")):
    s = wr[wr.group == g]
    spread(f"{p}_catch_rate", s.catch_rate, binom(s.catch_rate, s.tgt), f"{g}, {MS_WR}", SRC_PBP,
           s.cmp.sum() / s.tgt.sum(), "binomial")

arc = body_pfr(adv_rec, name="PFR receiving seasons (pfr_id)")
arc = arc[arc.tgt >= 40].copy()
arc["yac_per_rec"] = arc.yac / arc.rec
arc["brk_tkl_per_rec"] = arc.brk_tkl / arc.rec
arc["drop_pct"] = arc["drop"] / arc.tgt
s = arc[arc.group == "WR"]
SRC_REC = "PFR advanced receiving, 2018-2025"
slope("wr_wt__yac_per_rec", s, "wt", "yac_per_rec", "rec", "WR, 40+ targets (PFR)", SRC_REC, "yards after the catch a catch")
slope("wr_wt__brk_tkl_per_rec", s, "wt", "brk_tkl_per_rec", "rec", "WR, 40+ targets (PFR)", SRC_REC, "broken tackles a catch")
spread("wr_drop_pct", s.drop_pct, binom(s.drop_pct, s.tgt), "WR, 40+ targets (PFR)", SRC_REC + " (drops / targets)",
       s["drop"].sum() / s.tgt.sum(), "binomial")
# YAC a catch: PFR's value, noise from his own play-by-play catches.
pv = wr.set_index(["season", "pfr_id"])
pv = (pv.yac2 / pv.cmp - (pv.yac / pv.cmp) ** 2) / pv.cmp
pv = pv[~pv.index.duplicated()]
nv = s.join(pv.rename("nv"), on=["season", "pfr_id"]).nv
spread("wr_yac_per_rec", s.yac_per_rec, nv, "WR, 40+ targets (PFR)", SRC_REC + "; noise from his play-by-play catches",
       s.yac.sum() / s.rec.sum(), "per catch", 3)

ngc, ngc_w = ngs["receiving"]
ngc = body_gsis(ngc[ngc.targets >= 40].copy(), name="NGS receiving season rows, 40+ targets (gsis_id)")
SRC_NGC = "Next Gen Stats receiving, 2018-2025"
for g, p in (("WR", "wr"), ("TE", "te")):
    s = ngc[ngc.group == g]
    slope(f"{p}_forty__separation", s, "forty", "avg_separation", "targets", f"{g}, 40+ targets (NGS season row)", SRC_NGC,
          "yards of separation at the catch point")
    uv = week_var(ngc_w, ["season", "gsis_id"], "targets", "avg_separation")
    nv = s.join(uv.rename("uv"), on=["season", "gsis_id"]).uv / s.targets
    spread(f"{p}_separation", s.avg_separation, nv, f"{g}, 40+ targets (NGS season row)", SRC_NGC,
           np.average(s.avg_separation, weights=s.targets), "per target, from game rows", 3)
s = ngc[ngc.group == "WR"]
slope("wr_forty__cushion", s, "forty", "avg_cushion", "targets", "WR, 40+ targets (NGS season row)", SRC_NGC,
      "yards of cushion at the snap")

# ---------------------------------------------------------------- defenders
pbu_plays = pbp[(pbp.incomplete_pass == 1) & (pbp.interception == 0) & (pbp.two_point_attempt == 0)]
pbu = ids(pbu_plays, ["pass_defense_1_player_id", "pass_defense_2_player_id"]).groupby(["season", "gsis_id"]).size()
tk = ids(pbp, ["solo_tackle_1_player_id", "solo_tackle_2_player_id", "assist_tackle_1_player_id",
               "assist_tackle_2_player_id", "assist_tackle_3_player_id", "assist_tackle_4_player_id",
               "tackle_with_assist_1_player_id", "tackle_with_assist_2_player_id"]).groupby(["season", "gsis_id"]).size()
ff = ids(pbp, ["forced_fumble_player_1_player_id", "forced_fumble_player_2_player_id"]).groupby(["season", "gsis_id"]).size()
pdef = pd.concat([pbu.rename("pbu"), tk.rename("pbp_tkl"), ff.rename("ff")], axis=1).fillna(0).reset_index()
pdef["pfr_id"] = pdef.gsis_id.map(gsis_to_pfr)
pdef = pdef.dropna(subset=["pfr_id"]).groupby(["season", "pfr_id"])[["pbu", "pbp_tkl", "ff"]].sum()
_tc = ["solo_tackle_1_player_id", "solo_tackle_2_player_id", "assist_tackle_1_player_id",
       "assist_tackle_2_player_id", "assist_tackle_3_player_id", "assist_tackle_4_player_id",
       "tackle_with_assist_1_player_id", "tackle_with_assist_2_player_id"]
_runs = pbp[(pbp.play_type == "run") & (pbp.qb_kneel.fillna(0) == 0) & (pbp.two_point_attempt == 0)]
_x = pd.concat([ids(_runs, _tc).groupby(["season", "gsis_id"]).size().rename("rtk"),
                ids(_runs[_runs.yards_gained <= 3], _tc).groupby(["season", "gsis_id"]).size().rename("rstop"),
                ids(_runs[_runs.yards_gained >= 10], _tc).groupby(["season", "gsis_id"]).size().rename("rlong")],
               axis=1).fillna(0).reset_index()
_x["pfr_id"] = _x.gsis_id.map(gsis_to_pfr)
_x = _x.dropna(subset=["pfr_id"]).groupby(["season", "pfr_id"])[["rtk", "rstop", "rlong"]].sum()
pdef = pdef.join(_x, how="left").fillna(0)

dd = adv_def.merge(snap_season[["season", "pfr_id", "dfn"]], on=["season", "pfr_id"], how="left")
LINKS["PFR defense seasons with snap counts"] = {"rows": int(len(dd)), "with_snaps": int(dd.dfn.notna().sum())}
dd = dd.join(pdef, on=["season", "pfr_id"])
dd = body_pfr(dd, name="PFR defense seasons (pfr_id)")
dd = dd[dd.dfn >= 200].copy()
dd[["pbu", "pbp_tkl", "ff"]] = dd[["pbu", "pbp_tkl", "ff"]].fillna(0)
dd["prss_per_snap"] = dd.prss / dd.dfn
dd["sack_per_snap"] = dd.sk / dd.dfn
dd["mt_n"] = dd.m_tkl + dd.comb
dd["mtkl_pct"] = (dd.m_tkl / dd.mt_n).where(dd.mt_n > 0)
cov = dd.tgt >= 30
dd["cmp_allowed"] = (dd.cmp / dd.tgt).where(cov)
dd["yds_per_tgt"] = (dd.yds / dd.tgt).where(cov)
dd["int_per_tgt"] = (dd["int"] / dd.tgt).where(cov)
dd["pbu_per_tgt"] = (dd.pbu / dd.tgt).where(cov)
dd["rating_allowed"] = dd.rat.where(cov)
dd["yac_per_cmp"] = (dd.yac / dd.cmp).where(cov & (dd.cmp > 0))
dd["yds_per_cmp"] = (dd.yds / dd.cmp).where(cov & (dd.cmp > 0))
dd["dadot_"] = dd.dadot.where(cov)
dd["tgt_per_snap"] = dd.tgt / dd.dfn
dd["ff_per_tkl"] = (dd.ff / dd.pbp_tkl).where(dd.pbp_tkl >= 20)
SRC_DEF = "PFR advanced defense 2018-2025, defense snaps from nflverse snap counts"
MS_D = "200+ defense snaps"
MS_C = "200+ defense snaps and 30+ targets in coverage"
dt, edge, lb, cb, sf = (dd[dd.group == g] for g in ("DT", "EDGE", "LB", "CB", "S"))
slope("dt_wt__prss_per_snap", dt, "wt", "prss_per_snap", "dfn", "DT, " + MS_D, SRC_DEF, "pressures a defense snap")
slope("dt_wt__sack_per_snap", dt, "wt", "sack_per_snap", "dfn", "DT, " + MS_D, SRC_DEF, "sacks a defense snap")
slope("dt_forty__prss_per_snap", dt, "forty", "prss_per_snap", "dfn", "DT, " + MS_D, SRC_DEF, "pressures a defense snap")
slope("edge_wt__prss_per_snap", edge, "wt", "prss_per_snap", "dfn", "EDGE, " + MS_D, SRC_DEF, "pressures a defense snap")
slope("edge_forty__prss_per_snap", edge, "forty", "prss_per_snap", "dfn", "EDGE, " + MS_D, SRC_DEF, "pressures a defense snap")
slope("edge_forty__sack_per_snap", edge, "forty", "sack_per_snap", "dfn", "EDGE, " + MS_D, SRC_DEF, "sacks a defense snap")
slope("lb_wt__mtkl_pct", lb, "wt", "mtkl_pct", "mt_n", "LB, " + MS_D, SRC_DEF + " (missed / (missed + combined))",
      "missed tackle share")
slope("lb_forty__cmp_allowed", lb, "forty", "cmp_allowed", "tgt", "LB, " + MS_C, SRC_DEF, "completions allowed a target")
slope("lb_forty__yds_per_tgt", lb, "forty", "yds_per_tgt", "tgt", "LB, " + MS_C, SRC_DEF, "yards allowed a target")
slope("lb_ht__cmp_allowed", lb, "ht", "cmp_allowed", "tgt", "LB, " + MS_C, SRC_DEF, "completions allowed a target")
slope("cb_ht__cmp_allowed", cb, "ht", "cmp_allowed", "tgt", "CB, " + MS_C, SRC_DEF, "completions allowed a target")
slope("cb_ht__int_per_tgt", cb, "ht", "int_per_tgt", "tgt", "CB, " + MS_C, SRC_DEF, "interceptions a target")
slope("cb_ht__pbu_per_tgt", cb, "ht", "pbu_per_tgt", "tgt", "CB, " + MS_C,
      "play-by-play pass_defense_1/2 on incompletions over PFR targets", "breakups a target")
slope("cb_ht__yds_per_tgt", cb, "ht", "yds_per_tgt", "tgt", "CB, " + MS_C, SRC_DEF, "yards allowed a target")
slope("cb_ht__rating_allowed", cb, "ht", "rating_allowed", "tgt", "CB, " + MS_C, SRC_DEF, "passer rating allowed")
slope("cb_ht__tgt_per_snap", cb, "ht", "tgt_per_snap", "dfn", "CB, " + MS_D, SRC_DEF, "targets a defense snap")
slope("cb_wt__yac_per_cmp", cb, "wt", "yac_per_cmp", "cmp", "CB, " + MS_C, SRC_DEF, "yards after the catch a completion allowed")
slope("cb_wt__mtkl_pct", cb, "wt", "mtkl_pct", "mt_n", "CB, " + MS_D, SRC_DEF, "missed tackle share")
slope("cb_forty__yds_per_cmp", cb, "forty", "yds_per_cmp", "cmp", "CB, " + MS_C, SRC_DEF, "yards a completion allowed")
slope("cb_forty__dadot", cb, "forty", "dadot_", "tgt", "CB, " + MS_C, SRC_DEF, "depth of target when targeted")
slope("cb_cone__cmp_allowed", cb, "cone", "cmp_allowed", "tgt", "CB, " + MS_C, SRC_DEF, "completions allowed a target")
slope("s_wt__mtkl_pct", sf, "wt", "mtkl_pct", "mt_n", "S, " + MS_D, SRC_DEF, "missed tackle share")
slope("s_wt__ff_per_tkl", sf, "wt", "ff_per_tkl", "pbp_tkl", "S, " + MS_D + " and 20+ tackles",
      "play-by-play forced fumbles over play-by-play tackles", "forced fumbles a tackle")
slope("s_ht__cmp_allowed", sf, "ht", "cmp_allowed", "tgt", "S, " + MS_C, SRC_DEF, "completions allowed a target")
slope("s_forty__yds_per_cmp", sf, "forty", "yds_per_cmp", "cmp", "S, " + MS_C, SRC_DEF, "yards a completion allowed")
for _c in ("rtk", "rstop", "rlong"):
    dd[_c] = dd[_c].fillna(0)
dd["rtk_per_snap"] = dd.rtk / dd.dfn
dd["rstop_per_snap"] = dd.rstop / dd.dfn
dd["rlong_share"] = (dd.rlong / dd.rtk).where(dd.rtk >= 15)
dd["tkl_per_snap"] = dd.pbp_tkl / dd.dfn
# Run support and coverage by a safety's weight (proposal 4 of the A/B
# report): run tackles and stops a defense snap, the share of his run
# tackles 10+ yards downfield, all tackles, and coverage per target.
sf = dd[dd.group == "S"]
for _y, _w, _ms, _lab in (("rtk_per_snap", "dfn", MS_D, "run tackles a defense snap"),
                          ("rstop_per_snap", "dfn", MS_D, "run tackles at 3 yards or less a defense snap"),
                          ("rlong_share", "rtk", MS_D + " and 15+ run tackles", "share of his run tackles after 10+ yards"),
                          ("tkl_per_snap", "dfn", MS_D, "tackles a defense snap"),
                          ("pbu_per_tgt", "tgt", MS_C, "breakups a target"),
                          ("cmp_allowed", "tgt", MS_C, "completions allowed a target")):
    slope(f"s_wt__{_y}", sf, "wt", _y, _w, "S, " + _ms, SRC_DEF, _lab)
SRC_MD = "; arms and combine 10-yard splits from MockDraftable"
for g, p, s_ in (("DT", "dt", dt), ("EDGE", "edge", edge)):
    for xb in ("arm", "split"):
        slope(f"{p}_{xb}__prss_per_snap", s_, xb, "prss_per_snap", "dfn", f"{g}, {MS_D}", SRC_DEF + SRC_MD,
              "pressures a defense snap")
        slope(f"{p}_{xb}__sack_per_snap", s_, xb, "sack_per_snap", "dfn", f"{g}, {MS_D}", SRC_DEF + SRC_MD,
              "sacks a defense snap")
slope("cb_arm__cmp_allowed", cb, "arm", "cmp_allowed", "tgt", "CB, " + MS_C, SRC_DEF + SRC_MD, "completions allowed a target")
slope("cb_arm__int_per_tgt", cb, "arm", "int_per_tgt", "tgt", "CB, " + MS_C, SRC_DEF + SRC_MD, "interceptions a target")
slope("cb_arm__pbu_per_tgt", cb, "arm", "pbu_per_tgt", "tgt", "CB, " + MS_C,
      "play-by-play pass_defense_1/2 on incompletions over PFR targets" + SRC_MD, "breakups a target")
slope("cb_arm__rating_allowed", cb, "arm", "rating_allowed", "tgt", "CB, " + MS_C, SRC_DEF + SRC_MD, "passer rating allowed")

for g, p in (("DT", "dt"), ("EDGE", "edge")):
    s = dd[dd.group == g]
    for stat, col in (("prss", "prss_per_snap"), ("sack", "sack_per_snap")):
        num_ = s.prss if stat == "prss" else s.sk
        spread(f"{p}_{stat}_per_snap", s[col], binom(s[col], s.dfn), f"{g}, {MS_D}", SRC_DEF, num_.sum() / s.dfn.sum(), "binomial")

uvy = week_var(wk_def, ["season", "pfr_id"], "def_targets", "def_yards_allowed_per_tgt")
uvr = week_var(wk_def, ["season", "pfr_id"], "def_targets", "def_passer_rating_allowed")
for g, p in (("LB", "lb"), ("CB", "cb"), ("S", "s")):
    s = dd[dd.group == g]
    m = s[s.mt_n > 0]
    spread(f"{p}_mtkl_pct", m.mtkl_pct, binom(m.mtkl_pct, m.mt_n), f"{g}, {MS_D}", SRC_DEF + " (missed / (missed + combined))",
           m.m_tkl.sum() / m.mt_n.sum(), "binomial")
    c = s[s.tgt >= 30]
    spread(f"{p}_cmp_allowed", c.cmp_allowed, binom(c.cmp_allowed, c.tgt), f"{g}, {MS_C}", SRC_DEF, c.cmp.sum() / c.tgt.sum(), "binomial")
    if p in ("lb", "cb"):
        nv = c.join(uvy.rename("uv"), on=["season", "pfr_id"]).uv / c.tgt
        spread(f"{p}_yds_per_tgt", c.yds_per_tgt, nv, f"{g}, {MS_C}", SRC_DEF, c.yds.sum() / c.tgt.sum(), "per target, from game rows", 3)
cb_c = cb[cb.tgt >= 30]
spread("cb_int_per_tgt", cb_c.int_per_tgt, binom(cb_c.int_per_tgt, cb_c.tgt), "CB, " + MS_C, SRC_DEF,
       cb_c["int"].sum() / cb_c.tgt.sum(), "binomial")
spread("cb_pbu_per_tgt", cb_c.pbu_per_tgt, binom(cb_c.pbu_per_tgt.clip(upper=1), cb_c.tgt), "CB, " + MS_C,
       "play-by-play pass_defense_1/2 on incompletions over PFR targets", cb_c.pbu.sum() / cb_c.tgt.sum(), "binomial")
nv = cb_c.join(uvr.rename("uv"), on=["season", "pfr_id"]).uv / cb_c.tgt
spread("cb_rating_allowed", cb_c.rating_allowed, nv, "CB, " + MS_C, SRC_DEF,
       np.average(cb_c.rating_allowed, weights=cb_c.tgt), "per target, from game rows", 2)
sff = sf[sf.pbp_tkl >= 20]
v = sff.ff / sff.pbp_tkl
spread("s_ff_per_tkl", v, binom(v, sff.pbp_tkl), "S, " + MS_D + " and 20+ tackles",
       "play-by-play forced fumbles over play-by-play tackles", sff.ff.sum() / sff.pbp_tkl.sum(), "binomial")

# ---------------------------------------------------------------- penalties per snap
pen = pbp[pbp.penalty_player_id.notna() & pbp.penalty_type.notna()].copy()
pen["pfr_id"] = pen.penalty_player_id.map(gsis_to_pfr)
PEN = {"holding": ["Offensive Holding"], "false_start": ["False Start"],
       "offside": ["Defensive Offside", "Neutral Zone Infraction", "Encroachment"],
       "dpi": ["Defensive Pass Interference"], "dholding": ["Defensive Holding"]}
pc = pd.DataFrame({k: pen[pen.penalty_type.isin(v)].groupby(["season", "pfr_id"]).size() for k, v in PEN.items()}).fillna(0)
ps = body_pfr(snap_season.join(pc, on=["season", "pfr_id"]).fillna({k: 0 for k in PEN}), name="snap-count player seasons (pfr_id)")
SRC_PEN = "play-by-play penalty_player_id and penalty_type (first flag on a play) over nflverse snap counts"
for key, grp, k, snap_col in (("ol_holding_per_snap", ("OT", "IOL"), "holding", "off"),
                              ("ol_false_start_per_snap", ("OT", "IOL"), "false_start", "off"),
                              ("dl_offside_per_snap", ("DT", "EDGE"), "offside", "dfn"),
                              ("cb_dpi_per_snap", ("CB",), "dpi", "dfn"),
                              ("cb_holding_per_snap", ("CB",), "dholding", "dfn")):
    s = ps[ps.group.isin(grp) & (ps[snap_col] >= 200)]
    v = s[k] / s[snap_col]
    spread(key, v, v / s[snap_col], f"{'/'.join(grp)}, 200+ {'offense' if snap_col == 'off' else 'defense'} snaps",
           SRC_PEN, s[k].sum() / s[snap_col].sum(), "Poisson", 5)
SPREADS["dl_offside_per_snap"]["def"] = "defensive offside, neutral zone infraction and encroachment"

# ---------------------------------------------------------------- kickers, punters, returners
fg = pbp[(pbp.field_goal_attempt == 1) & pbp.kick_distance.notna()].copy()
fg["good"] = (fg.field_goal_result == "made").astype(float)
SRC_K = SRC_PBP
for key, lo, hi, ms in (("k_fg_pct_40_49", 40, 49, 10), ("k_fg_pct_50p", 50, 99, 5)):
    b = fg[(fg.kick_distance >= lo) & (fg.kick_distance <= hi)]
    k = b.groupby(["season", "kicker_player_id"]).good.agg(["size", "mean"])
    k = k[k["size"] >= ms]
    spread(key, k["mean"], binom(k["mean"], k["size"]), f"kicker season, {ms}+ attempts from {lo}{'-' + str(hi) if hi < 99 else '+'} yards",
           SRC_K, b.good.mean(), "binomial")
ko = pbp[(pbp.kickoff_attempt == 1) & ~pbp.desc.str.contains("onside", case=False, na=False)].copy()
for suf, yrs in KO_ERAS:
    key = "k_touchback_rate" + suf
    b = ko[ko.season.isin(yrs)]
    k = b.groupby(["season", "kicker_player_id"]).touchback.agg(["size", "mean"])
    k = k[k["size"] >= 40]
    spread(key, k["mean"], binom(k["mean"], k["size"]), f"kicker season, 40+ kickoffs (onside kicks left out), {yrs[0]}-{yrs[-1]}" if len(yrs) > 1 else f"kicker season, 40+ kickoffs (onside kicks left out), {yrs[0]}",
           SRC_PBP, b.touchback.mean(), "binomial")
SPREADS["k_touchback_rate_2024"]["note"] = "dynamic kickoff, touchback to the 30"
SPREADS["k_touchback_rate_2025"]["note"] = "dynamic kickoff, touchback to the 35"
pu = pbp[(pbp.punt_attempt == 1) & (pbp.punt_blocked == 0) & pbp.kick_distance.notna()].copy()
pu["net"] = pu.kick_distance - pu.return_yards.fillna(0) - 20 * pu.touchback.fillna(0)
pg = pu.groupby(["season", "punter_player_id"]).agg(n=("net", "size"), gross=("kick_distance", "mean"),
                                                    g2=("kick_distance", "var"), net=("net", "mean"), n2=("net", "var"),
                                                    in20=("punt_inside_twenty", "mean"))
pg = pg[pg.n >= 40]
MS_P = "punter season, 40+ punts (blocked punts left out)"
spread("p_gross", pg.gross, pg.g2 / pg.n, MS_P, SRC_PBP, pu.kick_distance.mean(), "per punt", 2)
spread("p_net", pg.net, pg.n2 / pg.n, MS_P, SRC_PBP + " (gross - return yards - 20 on a touchback)", pu.net.mean(), "per punt", 2)
spread("p_inside20_rate", pg.in20, binom(pg.in20, pg.n), MS_P, SRC_PBP, pu.punt_inside_twenty.mean(), "binomial")
kr = pbp[(pbp.kickoff_attempt == 1) & pbp.kickoff_returner_player_id.notna() & (pbp.touchback == 0)
         & (pbp.kickoff_fair_catch.fillna(0) == 0) & pbp.return_yards.notna()]
for suf, yrs in KO_ERAS:
    key = "kr_avg" + suf
    b = kr[kr.season.isin(yrs)]
    k = b.groupby(["season", "kickoff_returner_player_id"]).return_yards.agg(["size", "mean", "var"])
    k = k[k["size"] >= 15]
    spread(key, k["mean"], k["var"] / k["size"], f"returner season, 15+ kickoff returns, {yrs[0]}-{yrs[-1]}" if len(yrs) > 1 else f"returner season, 15+ kickoff returns, {yrs[0]}", SRC_PBP,
           b.return_yards.mean(), "per return", 2)
pr = pbp[(pbp.punt_attempt == 1) & pbp.punt_returner_player_id.notna()].copy()
pr["muff"] = pr.desc.str.contains("MUFFS", na=False).astype(float)
ret = pr[(pr.punt_fair_catch.fillna(0) == 0) & (pr.muff == 0) & pr.return_yards.notna()]
k = ret.groupby(["season", "punt_returner_player_id"]).return_yards.agg(["size", "mean", "var"])
k = k[k["size"] >= 15]
spread("pr_avg", k["mean"], k["var"] / k["size"], "returner season, 15+ punt returns", SRC_PBP, ret.return_yards.mean(), "per return", 2)
k = pr.groupby(["season", "punt_returner_player_id"]).muff.agg(["size", "mean"])
k = k[k["size"] >= 20]
spread("returner_muff_rate", k["mean"], binom(k["mean"], k["size"]),
       "returner season, 20+ punts fielded (returned, fair caught or muffed)",
       SRC_PBP + " ('MUFFS' in the play text on punts)", pr.muff.mean(), "binomial")

# ---------------------------------------------------------------- deltas
DELTAS = {}
fg["late_close"] = (fg.qtr >= 4) & (fg.score_differential.abs() <= 8)
kl = {}
for name, lo, hi in (("30-39", 30, 39), ("40-49", 40, 49), ("50+", 50, 99)):
    b = fg[(fg.kick_distance >= lo) & (fg.kick_distance <= hi)]
    L, R = b[b.late_close], b[~b.late_close]
    pl_, pr_ = L.good.mean(), R.good.mean()
    se = np.sqrt(binom(pl_, len(L)) + binom(pr_, len(R)))
    # The same kicks at the same distances: each late kick against the rest's rate at its yard line.
    rate_at = R.groupby("kick_distance").good.mean()
    exp = L.kick_distance.map(rate_at).fillna(pr_)
    adj = pl_ - exp.mean()
    se_adj = np.sqrt(binom(pl_, len(L)))
    kl[name] = {"late_close": {"att": int(len(L)), "pct": r(pl_), "mean_dist": r(L.kick_distance.mean(), 2)},
                "rest": {"att": int(len(R)), "pct": r(pr_), "mean_dist": r(R.kick_distance.mean(), 2)},
                "diff": {"b": r(pl_ - pr_), "lo": r(pl_ - pr_ - 1.96 * se), "hi": r(pl_ - pr_ + 1.96 * se)},
                "diff_same_distance": {"b": r(adj), "lo": r(adj - 1.96 * se_adj), "hi": r(adj + 1.96 * se_adj)}}
DELTAS["k_late_close"] = {"def": "FG make rate in the 4th quarter or overtime with the score within 8, against every other attempt",
                          "bands": kl, "source": SRC_PBP,
                          "note": "diff_same_distance compares each late kick with the rest's rate at the same yard line"}

gm = snaps.copy()
gm["home"] = gm.game_id.str.split("_").str[3] == gm.team
gm = gm.rename(columns={"pfr_player_id": "pfr_id"}).join(by_pfr.group, on="pfr_id")
fs = pen[(pen.penalty_type == "False Start")].copy()
fs["group"] = fs.penalty_player_id.map(group_of)
fs["home"] = fs.penalty_team == fs.home_team


def home_road(groups, no2020=False):
    g = gm[gm.group.isin(groups) & ((gm.season != 2020) if no2020 else True)]
    f = fs[fs.group.isin(groups) & ((fs.season != 2020) if no2020 else True)]
    sn_ = g.groupby("home").offense_snaps.sum()
    fl = f.home.value_counts()
    h_n, a_n = float(fl.get(True, 0)), float(fl.get(False, 0))
    h_s, a_s = float(sn_.get(True, 0)), float(sn_.get(False, 0))
    rh, ra = h_n / h_s, a_n / a_s
    se = np.sqrt(h_n / h_s ** 2 + a_n / a_s ** 2)
    return {"home": {"flags": int(h_n), "snaps": int(h_s), "rate": r(rh, 5)},
            "road": {"flags": int(a_n), "snaps": int(a_s), "rate": r(ra, 5)},
            "diff_home_minus_road": {"b": r(rh - ra, 5), "lo": r(rh - ra - 1.96 * se, 5), "hi": r(rh - ra + 1.96 * se, 5)},
            "ratio_road_to_home": r(ra / rh, 3)}


DELTAS["home_road_false_start"] = {
    "def": "OL false starts a snap at home against on the road (OL offense snaps from the snap counts, home from the game id)",
    "OL": home_road(["OT", "IOL"]), "OL_without_2020": home_road(["OT", "IOL"], True),
    "OT": home_road(["OT"]), "IOL": home_road(["IOL"]), "source": SRC_PEN,
    "note": "tackles false start more on the road, interior linemen (centers most) more at home; 2020 had few or no crowds"}

# ---------------------------------------------------------------- arms and splits by position (generator check)
ARMS = {"source": "MockDraftable, draft classes 2005-2025, linked to nflverse position groups; splits from the combine only",
        "link": MD_LINK, "groups": {}}
pp = people[people.pfr_id.notna()].drop_duplicates("pfr_id")
for g in ("QB", "RB", "WR", "TE", "OT", "IOL", "DT", "EDGE", "LB", "CB", "S"):
    row = {}
    for col, nd in (("arm", 2), ("split", 3), ("hand", 2), ("wingspan", 2)):
        v = pp.loc[pp.group == g, col].dropna()
        if len(v) >= 30:
            row[col] = {"n": int(len(v)), "mean": r(v.mean(), nd), "sd": r(v.std(ddof=1), nd),
                        "p10": r(np.percentile(v, 10), nd), "p50": r(np.percentile(v, 50), nd),
                        "p90": r(np.percentile(v, 90), nd)}
    ARMS["groups"][g] = row

# ---------------------------------------------------------------- links and gaps
LINKS["players.csv rows with both gsis_id and pfr_id"] = f"{int((players.gsis_id.notna() & players.pfr_id.notna()).sum())} of {len(players)}"
LINKS["combine rows with pfr_id"] = f"{int(combine.pfr_id.notna().sum())} of {len(combine)}"
LINKS["team seasons with PFR pressures"] = f"{TEAM_LINK['with pressures']} of {TEAM_LINK['team seasons']}"
LINKS["note"] = ("rows are player seasons before any sample cut; with_position = a position group found through "
                 "players.csv or the combine; with_combine = a combine row (height or forty); each slope's n against "
                 "n_qualified says how many qualified rows had the body measure")
GAPS = [
    "No free file has contested catches by defender or by receiver at the catch point (PFF and Sports Info Solutions are paid).",
    "No free file has pressures or sacks allowed per blocker, so offensive line body slopes are not possible; only team pressure and sack rates.",
    "Arm length, hand size, wingspan and the 10-yard split are not in the nflverse combine file; they come from MockDraftable (mockdraftable.py reads its site API: combine numbers first, pro day numbers where the combine has none; splits from the combine only), linked by name and draft class, and a name that appears twice in one class is left out (see arms.link for the counts).",
    "10-yard splits were timed at the combine for only part of each class (the electronic split is not listed for every runner), so the split slopes rest on fewer players than the forty's.",
    "Offensive tackle arm length against pressures allowed needs per-blocker pressures, which no free file has.",
    "Forty and cone are combine runs only (no pro days), so players who skipped a drill drop out of that slope; the fastest and the top picks skip more often.",
    "Combine weight is the weight at the combine; later playing weight is not tracked per season. Players with no combine row use players.csv height and weight (their current listing).",
    "Position groups come from PFF's position in players.csv (ED is EDGE, DI is DT), else NGS, else the roster position (OLB without a PFF or NGS label counts as LB). One group per career.",
    "Pressures and sacks are per defense snap, not per pass-rush snap: free files have no pass-rush snap count.",
    "Play action comes from PFR passing for 2019-2023 only; the play-by-play has no play-action column.",
    "Penalties count only the first flag on a play, as the play-by-play names it.",
    "Tackles, forced fumbles and breakups come from the play-by-play's official stat columns; missed tackles and targets in coverage are PFR charting.",
    "Separation and cushion are Next Gen Stats season rows, which list only receivers with enough targets; per-route separation is not free.",
    "sd_true from game rows treats game-to-game changes in a player's true level as noise too, so it is a floor.",
]

# Searched 2026-11-06 for about 15 minutes; no open, licensed download. MockDraftable's site API was read
# instead once the environment's network allowed it (mockdraftable.py).
ARM_SEARCH = {
    "result": "no free, licensed, downloadable file of combine arm lengths or 10-yard splits; MockDraftable's site API is read instead (mockdraftable.py), and only the fitted numbers are kept here",
    "checked": [
        "nflverse combine release (combine.csv): height, weight, forty, bench, vertical, broad jump, cone, shuttle; no arms, hands or splits",
        "MockDraftable (arm, hand, wingspan, 10- and 20-yard splits per player): web pages only, no bulk export; its code (github.com/marcusdarmstrong/mockdraftable-web, MIT) holds code, not the data",
        "github.com/mario-j/combine-parser: a scraper of MockDraftable, no data file, no licence",
        "github.com/Johnny-Codes/NFL-Combine-Data: a 2018 scraper of nflcombineresults.com (hand size, arm length, no splits), no licence, so not used",
        "github.com/rmcurtis43/combineR (MIT): reads Pro Football Reference combine pages, which carry no arms or splits",
        "AFIT-R MODA R package: a 1999-2015 combine set with a ten-yard column; its page (rdrr.io) is not reachable from this machine, and its players mostly retired before 2018",
        "nflcombineresults.com: not reachable from this machine",
    ],
    "used": "MockDraftable through its site API (/api/search, /api/multiple-players), the 2005-2025 draft classes, read once and cached outside the repo",
}

out = {"source": "nflverse play-by-play, snap counts, PFR advanced stats, Next Gen Stats, combine and players, NFL regular seasons 2018-2025; arms and 10-yard splits from MockDraftable",
       "made_by": "data/real-world/scripts/ab_real.py",
       "use": "real numbers for the engine's A/B tests (afm/ab-tests/plan.md): engine pure slope against held, as-recruited against raw",
       "notes": {"rates": "fractions, not percents (except qb_cpoe, in percentage points, and passer rating)",
                 "held": "body plus log(overall pick), undrafted = pick 260 with an undrafted dummy, plus season dummies",
                 "bands": "95%, cluster-robust by player, weighted least squares with the sample as weight",
                 "sd_true": "sqrt(max(0, var - mean noise var)); noise is binomial, Poisson, or per-play variance over the sample",
                 "means": "pooled over the qualified rows of each spread key (sum over sum, or sample-weighted)"},
       "arm_search": ARM_SEARCH, "arms": ARMS,
       "links": LINKS, "slopes": SLOPES, "spreads": SPREADS, "deltas": DELTAS, "means": MEANS, "gaps": GAPS}
with open(OUT, "w") as f:
    json.dump(out, f, indent=1)
    f.write("\n")
print(f"wrote {OUT}: {len(SLOPES)} slopes, {len(SPREADS)} spreads", file=sys.stderr)
