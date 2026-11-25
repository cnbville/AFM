"""Real defenders: who makes the tackles, sacks and tackles for loss (realism audit item 3).

FBS 2024-2025 and the NFL 2021-2024 (regular seasons): the nth best season a year, the counts
over the marks, a team leader's share, a team's tackles by rank within one game and over a
season, the same inside each group (line, linebackers, backs), and for the NFL the snap shares by
rank and the tackles a 100 defense snaps by listed position.

Reads, saved in DIR:
  player_box_YEAR.parquet 2024-2025 (sportsdataverse-data release espn_cfb_player_box: ESPN box scores),
  cfb_team_info_YEAR.parquet 2024-2025 (release cfb_team_info: which teams were FBS that year),
  cfb_rosters_YEAR.csv 2024-2025 (https://github.com/sportsdataverse/cfbfastR-data/tree/main/rosters/csv: positions and weights),
  player_stats_def_YEAR.parquet 2021-2024 (nflverse-data release player_stats: weekly defense lines),
  snap_counts_YEAR.parquet 2021-2024 (nflverse-data release snap_counts: Pro Football Reference's counts).
ESPN's college boxes carry full defensive lines from 2024 only. Rosters label many edges LB and many
backs DB; the NFL's weekly lines list 3-4 edges as LB. A college tackle shared by two men is an
assist each and no solo; the files' totals are solo plus assists.

Usage: python3 defenders.py DIR > defenders.json
Development checks only, never in-game tuning.
"""
import json, os, sys
import numpy as np
import pandas as pd

D = sys.argv[1]
RANKS = [1, 2, 3, 5, 10, 25, 50, 100, 200]
MARKS = {"sacks": (10, 12, 15, 20), "tfl": (15, 20, 25, 30), "tackles": (100, 120, 130, 150)}
GROUP = {"DL": "line", "DE": "line", "DT": "line", "NT": "line", "EDGE": "line", "LB": "linebackers", "OLB": "linebackers",
         "ILB": "linebackers", "MLB": "linebackers", "DB": "backs", "CB": "backs", "S": "backs", "FS": "backs", "SS": "backs"}


def r(x, n=3):
    return round(float(x), n)


def rl(v, n=3):
    return [r(x, n) for x in v]


def find(*names):
    for n in names:
        if os.path.exists(f"{D}/{n}"):
            return f"{D}/{n}"
        if os.path.exists(f"{D}/nfl/{n}"):
            return f"{D}/nfl/{n}"
    raise FileNotFoundError(names)


class Acc:
    """Means over seasons of whatever each season adds."""

    def __init__(self):
        self.v = {}

    def add(self, key, val):
        self.v.setdefault(key, []).append(np.asarray(val, dtype=float))

    def mean(self, key, n=3):
        m = np.mean(np.array(self.v[key]), axis=0)
        return rl(m, n) if m.ndim else r(m, n)


def ladders(acc, p):
    """p: one row a player season with sacks, tfl, tackles."""
    for c in MARKS:
        v = np.sort(p[c].fillna(0).values)[::-1]
        acc.add(f"ladder_{c}", [v[n - 1] for n in RANKS])
        acc.add(f"over_{c}", [(v >= t).sum() for t in MARKS[c]])


def leaders(acc, s):
    """s: one row a player and team season with team, sacks, tfl, tackles."""
    for c in MARKS:
        g = s.groupby("team")[c]
        share = (g.max() / g.sum()).dropna()
        acc.add(f"lead_{c}", [share.mean(), share.quantile(0.1), share.median(), share.quantile(0.9), share.max(), g.max().mean()])
    rows, men = [], []
    for _, g in s.groupby("team"):
        v = np.sort(g.tackles.fillna(0).values)[::-1]
        rows.append([v[i] / v.sum() if i < len(v) else 0.0 for i in range(60)])
        men.append((v > 0).sum())
    m = np.mean(rows, axis=0)
    acc.add("season_ranks", list(m[:11]) + [m[:11].sum(), m[11:22].sum(), 1 - m[:22].sum(), np.mean(men)])


def games(acc, d, keys):
    """d: one row a player game with a tackle, the game's keys, grp and tackles."""
    rows, men = [], []
    for _, x in d.groupby(keys):
        v = np.sort(x.tackles.values)[::-1]
        rows.append([v[i] / v.sum() if i < len(v) else 0.0 for i in range(40)])
        men.append(len(v))
    m = np.mean(rows, axis=0)
    acc.add("game_ranks", list(m[:11]) + [m[:11].sum(), m[11:22].sum(), 1 - m[:22].sum(), np.mean(men)])
    n = len(rows)
    for g in ("line", "linebackers", "backs"):
        x = d[d.grp == g]
        ranks = np.zeros(8)
        for _, v in x.groupby(keys).tackles:
            q = np.sort(v.values)[::-1][:8]
            ranks[: len(q)] += q
        acc.add(f"group_game_{g}", list(ranks / n) + [len(x) / n])
    return n


def shape(acc, out):
    out["ladder"] = {"ranks": RANKS, **{c: acc.mean(f"ladder_{c}", 2) for c in MARKS}}
    out["a_year_at_or_over"] = {c: dict(zip(map(str, MARKS[c]), acc.mean(f"over_{c}", 2))) for c in MARKS}
    names = ["share", "share_p10", "share_p50", "share_p90", "share_max", "count"]
    out["team_leader"] = {c: dict(zip(names, acc.mean(f"lead_{c}"))) for c in MARKS}
    for k in ("season_ranks", "game_ranks"):
        m = acc.mean(k)
        out[f"tackles_by_{k}"] = {"ranks_1_11": m[:11], "top_11": m[11], "next_11": m[12], "rest": m[13], "men_with_a_tackle": m[14]}
    out["by_group"] = {}
    for g in ("line", "linebackers", "backs"):
        m = acc.mean(f"group_game_{g}", 2)
        out["by_group"][g] = {"a_team_game": acc.mean(f"group_total_{g}", 2), "game_ranks_1_8": m[:8], "men_with_a_tackle": m[8],
                              "season_ranks_1_6_a_team_game": acc.mean(f"group_season_{g}", 2)}


out = {
    "source": "ESPN college box scores (sportsdataverse-data espn_cfb_player_box) 2024-2025 with CollegeFootballData rosters (cfbfastR-data); nflverse weekly defense lines and Pro Football Reference snap counts 2021-2024",
    "holes": "College defensive boxes are complete from 2024 only. No college snap counts. Rosters list many edges LB and many backs DB, and the NFL's weekly lines list 3-4 edges as LB, so the groups are the listed ones. NFL names are joined to snap counts by name, team and week (94% of tackles match).",
    "note": "Tackles are solo plus assists. Seasons are every game a team played (FBS: bowls and FCS games too; NFL: the 17-game regular season). Means over the seasons read.",
}

# ---- FBS
acc = Acc()
for y in (2024, 2025):
    b = pd.read_parquet(f"{D}/player_box_{y}.parquet")
    ti = pd.read_parquet(f"{D}/cfb_team_info_{y}.parquet", columns=["team_id", "classification"])
    fbs = set(ti[ti.classification == "fbs"].team_id.astype(str))
    b = b[b.team_id.astype(str).isin(fbs)].copy()
    b["tackles"] = pd.to_numeric(b.totalTackles, errors="coerce").fillna(0)
    b["solo"] = pd.to_numeric(b.soloTackles, errors="coerce").fillna(0)
    b["sacks"] = pd.to_numeric(b.sacks, errors="coerce").fillna(0)
    b["tfl"] = pd.to_numeric(b.tacklesForLoss, errors="coerce").fillna(0)
    b["team"] = b.team_id.astype(str)
    d = b[b.tackles > 0].copy()
    ro = pd.read_csv(f"{D}/cfb_rosters_{y}.csv", usecols=["athlete_id", "position", "weight"], low_memory=False).drop_duplicates("athlete_id")
    d["athlete_id"] = pd.to_numeric(d.athlete_id, errors="coerce")
    d = d.merge(ro, on="athlete_id", how="left")
    d["grp"] = d.position.astype(str).str.upper().map(GROUP).fillna("other")
    tot = d.groupby(["game_id", "team"]).tackles.sum()
    d = d.set_index(["game_id", "team"]).loc[tot[tot >= 20].index].reset_index()
    n = games(acc, d, ["game_id", "team"])
    acc.add("per_team_game", [d.tackles.sum() / n, d.solo.sum() / n, d.sacks.sum() / n, d.tfl.sum() / n])
    s = d.groupby(["team", "athlete_id", "grp"]).agg(tackles=("tackles", "sum"), sacks=("sacks", "sum"), tfl=("tfl", "sum"), games=("game_id", "nunique")).reset_index()
    ladders(acc, s)
    leaders(acc, s)
    tg = d.groupby("team").game_id.nunique()
    s["tg"] = s.team.map(tg)
    for g in ("line", "linebackers", "backs", "other"):
        x = d[d.grp == g]
        acc.add(f"group_total_{g}", [x.tackles.sum() / n, x.sacks.sum() / n, x.tfl.sum() / n])
        if g == "other":
            continue
        xs = s[s.grp == g].copy()
        xs["rank"] = xs.groupby("team").tackles.rank(ascending=False, method="first")
        acc.add(f"group_season_{g}", [(xs[xs["rank"] == k].tackles / xs[xs["rank"] == k].tg).mean() for k in range(1, 7)])
    x = d[d.grp == "linebackers"]
    for lo, hi in ((0, 225), (225, 240), (240, 255), (255, 400)):
        q = x[(x.weight >= lo) & (x.weight < hi)]
        acc.add(f"lb_{lo}", [q.tackles.sum() / n, q.sacks.sum() / n, q.tfl.sum() / n, len(q) / n])
    # The season's top tacklers of a team: the games they played of the team's, and a game.
    s["rank"] = s.groupby("team").tackles.rank(ascending=False, method="first")
    for lo, hi in ((1, 1), (2, 3), (4, 6), (7, 11), (12, 22)):
        q = s[(s["rank"] >= lo) & (s["rank"] <= hi)]
        acc.add(f"played_{lo}", [np.mean(q.games / q.tg), np.mean(q.tackles / q.games)])
fbs = {"seasons": [2024, 2025], "per_team_game": dict(zip(["tackles", "solo", "sacks", "tfl"], acc.mean("per_team_game", 2)))}
shape(acc, fbs)
fbs["by_group"]["other"] = {"a_team_game": acc.mean("group_total_other", 2)}
fbs["by_group"]["note"] = "a_team_game is tackles, sacks, tackles for loss"
fbs["linebackers_by_weight"] = {f"{lo}-{hi}": dict(zip(["tackles", "sacks", "tfl", "men_with_a_tackle"], acc.mean(f"lb_{lo}")))
                                for lo, hi in ((0, 225), (225, 240), (240, 255), (255, 400))}
fbs["season_top_tacklers"] = {f"{lo}-{hi}": dict(zip(["games_played_share", "tackles_a_game_played"], acc.mean(f"played_{lo}")))
                              for lo, hi in ((1, 1), (2, 3), (4, 6), (7, 11), (12, 22))}
out["fbs"] = fbs

# ---- NFL
acc = Acc()
SNAP = {"DE": "DE", "DT": "DT", "NT": "DT", "LB": "LB", "ILB": "LB", "MLB": "LB", "OLB": "LB", "CB": "CB", "DB": "CB", "S": "S", "FS": "S", "SS": "S"}
PG = {"DL": "line", "LB": "linebackers", "DB": "backs"}
rate_rows = []
for y in (2021, 2022, 2023, 2024):
    d = pd.read_parquet(find(f"player_stats_def_{y}.parquet", f"def_{y}.parquet"))
    d = d[d.season_type == "REG"].copy()
    d["tackles"] = d.def_tackles_solo.fillna(0) + d.def_tackle_assists.fillna(0)
    d["sacks"] = d.def_sacks.fillna(0)
    d["tfl"] = d.def_tackles_for_loss.fillna(0)
    d["grp"] = d.position_group.map(PG).fillna("other")
    n = d.groupby(["team", "week"]).ngroups
    acc.add("per_team_game", [d.tackles.sum() / n, d.def_tackles_solo.fillna(0).sum() / n, d.sacks.sum() / n, d.tfl.sum() / n])
    p = d.groupby("player_id").agg(tackles=("tackles", "sum"), sacks=("sacks", "sum"), tfl=("tfl", "sum"))
    ladders(acc, p)
    s = d.groupby(["team", "player_id", "grp"]).agg(tackles=("tackles", "sum"), sacks=("sacks", "sum"), tfl=("tfl", "sum")).reset_index()
    leaders(acc, s)
    games(acc, d[d.tackles > 0], ["team", "week"])
    for g in ("line", "linebackers", "backs"):
        x = d[d.grp == g]
        acc.add(f"group_total_{g}", [x.tackles.sum() / n, x.sacks.sum() / n, x.tfl.sum() / n])
        xs = s[s.grp == g].copy()
        xs["rank"] = xs.groupby("team").tackles.rank(ascending=False, method="first")
        acc.add(f"group_season_{g}", [(xs[xs["rank"] == k].tackles / 17.0).mean() for k in range(1, 7)])
    sn = pd.read_parquet(find(f"snap_counts_{y}.parquet", f"snaps_{y}.parquet"))
    sn = sn[(sn.game_type == "REG") & (sn.defense_snaps > 0)].copy()
    sn["g"] = sn.position.map(SNAP)
    sn = sn[sn.g.notna()]
    for g, x in sn.groupby("g"):
        rows = []
        for _, t in x.groupby(["game_id", "team"]):
            v = np.sort(t.defense_pct.values)[::-1]
            rows.append([v[i] if i < len(v) else 0.0 for i in range(6)] + [v.sum()])
        acc.add(f"snaps_{g}", np.mean(rows, axis=0))
    d["name"] = d.player_display_name.str.lower().str.replace(r"[^a-z ]", "", regex=True)
    sn["name"] = sn.player.str.lower().str.replace(r"[^a-z ]", "", regex=True)
    m = sn.merge(d[["name", "team", "week", "tackles", "sacks", "tfl"]], on=["name", "team", "week"], how="left").fillna({"tackles": 0, "sacks": 0, "tfl": 0})
    m["tg"] = sn.groupby(["game_id", "team"]).ngroups
    rate_rows.append(m)
nfl = {"seasons": [2021, 2022, 2023, 2024], "per_team_game": dict(zip(["tackles", "solo", "sacks", "tfl"], acc.mean("per_team_game", 2)))}
shape(acc, nfl)
nfl["by_group"]["note"] = "a_team_game is tackles, sacks, tackles for loss; 3-4 edges are listed LB"
nfl["snap_share_by_rank_in_a_game"] = {g: {"ranks_1_6": acc.mean(f"snaps_{g}", 2)[:6], "on_the_field": acc.mean(f"snaps_{g}", 2)[6]} for g in ("DE", "DT", "LB", "CB", "S")}
m = pd.concat(rate_rows)
tg = sum(x.tg.iloc[0] for x in rate_rows)
nfl["per_100_defense_snaps"] = {}
for g, x in m.groupby("g"):
    sn = x.defense_snaps.sum()
    st = x[x.defense_snaps >= 50]
    nfl["per_100_defense_snaps"][g] = {"snaps_a_team_game": r(sn / tg, 1), "tackles": r(100 * x.tackles.sum() / sn, 2), "sacks": r(100 * x.sacks.sum() / sn),
                                       "tfl": r(100 * x.tfl.sum() / sn), "tackles_with_50_snaps_in_the_game": r(100 * st.tackles.sum() / st.defense_snaps.sum(), 2)}
out["nfl"] = nfl
print(json.dumps(out, indent=1))
