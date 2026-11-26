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
  snap_counts_YEAR.parquet 2021-2024 (nflverse-data release snap_counts: Pro Football Reference's counts),
  pbp_YEAR.csv.gz and part_YEAR.csv 2023-2024 (nflverse-data releases pbp and pbp_participation: every
  play's tacklers and the eleven men on the field).
An NFL tackle counts the way the league's combined tackles add up: a solo, a tackle with an assist
(the first man in a "(A, B)" tackle) and each assist count one each. The weekly lines and the plays
keep the defense's tackles only: a kick's coverage is not in them (kickers and punters have none).
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


MARGINS = ((0, 7), (8, 16), (17, 27), (28, 999))


def box_points(b):
    """A team's points in each game, from the boxes' touchdowns and kicking points (no two-point tries):
    close enough to sort games by their margin."""
    num = lambda c: pd.to_numeric(b[c], errors="coerce").fillna(0)
    pts = 6 * (num("rushingTouchdowns") + num("receivingTouchdowns") + num("kickReturnTouchdowns") + num("puntReturnTouchdowns")
               + num("defensiveTouchdowns")) + num("totalKickingPoints")
    return pts.groupby([b.game_id, b.team_id.astype(str)]).sum()


def margins(acc, d, points, fbs):
    """How a team game's tackles spread by its final margin, in games between two FBS teams: for the
    winner and the loser by 0-7, 8-16, 17-27 and 28 or more, the share of games, the men with a tackle,
    the top man's tackles and share, the top 11's share and the team's tackles."""
    by_game = points.reset_index()
    by_game.columns = ["game_id", "team", "pts"]
    two = by_game.groupby("game_id").filter(lambda x: len(x) == 2 and set(x.team) <= fbs)
    opp = two.merge(two, on="game_id", suffixes=("", "_o"))
    opp = opp[opp.team != opp.team_o].set_index(["game_id", "team"])
    rows = []
    for key, x in d.groupby(["game_id", "team"]):
        if key not in opp.index:
            continue
        o = opp.loc[key]
        v = np.sort(x.tackles.values)[::-1]
        rows.append((o.pts - o.pts_o, len(v), v[0], v[0] / v.sum(), v[:11].sum() / v.sum(), v.sum()))
    m = np.array(rows, dtype=float)
    m = m[m[:, 0] != 0]
    out = []
    for lost in (False, True):
        side = m[(m[:, 0] < 0) == lost]
        for lo, hi in MARGINS:
            x = side[(np.abs(side[:, 0]) >= lo) & (np.abs(side[:, 0]) <= hi)]
            out.append([len(x) / max(len(side), 1)] + list(x[:, 1:].mean(axis=0)))
    acc.add("by_margin", out)


TK = ["solo_tackle_1_player_id", "solo_tackle_2_player_id", "tackle_with_assist_1_player_id", "tackle_with_assist_2_player_id",
      "assist_tackle_1_player_id", "assist_tackle_2_player_id", "assist_tackle_3_player_id", "assist_tackle_4_player_id"]
ON = {"CB": "CB", "DB": "CB", "FS": "S", "SS": "S", "S": "S", "ILB": "LB", "MLB": "LB", "LB": "LB", "OLB": "OLB",
      "DE": "DE", "DT": "DT", "NT": "DT", "DL": "DT"}


def pbp_section():
    """The NFL's plays 2023-24: who is credited on what, and the men on the field by listed position."""
    cols = ["game_id", "play_id", "season_type", "play_type", "special", "defteam", "touchdown", "complete_pass", "incomplete_pass",
            "interception", "sack", "qb_scramble", "qb_kneel", "qb_spike", "out_of_bounds", "two_point_attempt", "rush", "qb_dropback",
            "yards_gained", "yards_after_catch", "down", "ydstogo", "yardline_100"] + TK
    rows = []
    for y in (2023, 2024):
        p = pd.read_csv(find(f"pbp_{y}.csv.gz"), usecols=cols, low_memory=False)
        p = p[p.season_type == "REG"]
        q = pd.read_csv(find(f"part_{y}.csv"), usecols=["nflverse_game_id", "play_id", "defense_players", "defense_positions",
                                                         "offense_players", "offense_positions", "offense_personnel",
                                                         "number_of_pass_rushers", "was_pressure"], low_memory=False)
        rows.append(p.merge(q.rename(columns={"nflverse_game_id": "game_id"}), on=["game_id", "play_id"], how="left"))
    p = pd.concat(rows, ignore_index=True)
    p["credits"] = p[TK].notna().sum(axis=1)
    tg = p[p.special == 0].groupby(["game_id", "defteam"]).ngroups
    s = p[(p.special == 0) & p.play_type.isin(["run", "pass"]) & (p.qb_kneel != 1) & (p.qb_spike != 1) & (p.two_point_attempt != 1)]
    un = s[s.credits == 0]
    inc = (un.incomplete_pass == 1) & (un.sack != 1)
    td = (un.touchdown == 1) & ~inc
    ob = (un.out_of_bounds == 1) & ~inc & ~td
    run = (s.rush == 1) & (s.qb_scramble != 1)
    kinds = {"runs": run, "scrambles": s.qb_scramble == 1, "sacks": s.sack == 1, "catches": s.complete_pass == 1, "interceptions": s.interception == 1}
    by_play = {}
    for k, m in kinds.items():
        x = s[m]
        by_play[k] = [r(len(x) / tg, 2), r((x.credits > 0).mean()), r(x.credits.mean()), r((x.credits >= 2).sum() / max((x.credits > 0).sum(), 1))]
    on, tk = {}, {}
    f = s[s.defense_players.notna()]
    for kind, m in (("runs", run.loc[f.index]), ("dropbacks", ~run.loc[f.index])):
        x = f[m]
        for ids, pos, t in zip(x.defense_players, x.defense_positions, x[TK].values):
            got = {v for v in t if isinstance(v, str)}
            for i, ps in zip(str(ids).split(";"), str(pos).split(";")):
                g = ON.get(ps, "other")
                on[(g, kind)] = on.get((g, kind), 0) + 1
                tk[(g, kind)] = tk.get((g, kind), 0) + (i in got)
    groups = ["DE", "DT", "OLB", "LB", "CB", "S"]
    per = {}
    for g in groups:
        a = on.get((g, "runs"), 0) + on.get((g, "dropbacks"), 0)
        per[g] = [r(100 * (tk.get((g, "runs"), 0) + tk.get((g, "dropbacks"), 0)) / max(a, 1), 2),
                  r(100 * tk.get((g, "runs"), 0) / max(on.get((g, "runs"), 0), 1), 2),
                  r(100 * tk.get((g, "dropbacks"), 0) / max(on.get((g, "dropbacks"), 0), 1), 2)]
    pos = f.defense_positions.astype(str).str.split(";")
    count = lambda want: pos.apply(lambda v: sum(ON.get(x) in want for x in v))
    dbs, saf, ilb = count(("CB", "S")), count(("S",)), count(("LB",))
    k = p[(p.special == 1) & p.play_type.isin(["kickoff", "punt"])]
    # Runs by yards gained and catches by yards after the catch: the share,
    # two men on a tackled play, out of bounds.
    edges = [(-99, 0), (0, 3), (3, 6), (6, 10), (10, 20), (20, 999)]

    def depths(x, col, first):
        out = {}
        for lo, hi in edges[first:]:
            lo = -99 if lo == edges[first][0] else lo
            y = x[(x[col] >= lo) & (x[col] < hi)]
            t = y[y.credits > 0]
            out[f"{lo}_{hi}"] = [r(len(y) / max(len(x), 1)), r((t.credits >= 2).mean() if len(t) else 0.0), r((y.out_of_bounds == 1).mean() if len(y) else 0.0)]
        return out

    # The backs a defense plays by the offense's wide receivers (one or none,
    # two, three, four or more) and the down: first and second, then third
    # and fourth with 1-2, 3-6 and 7 or more to go; inside the 3 apart.
    g = f[f.offense_personnel.notna() & (pos.apply(len) == 11)]
    wr = g.offense_personnel.astype(str).str.extract(r"(\d) WR")[0].fillna(0).astype(int).clip(1, 4)
    togo, down = g.ydstogo.astype(float), g.down.astype(float)
    col = np.where(down <= 2, 0, np.where(togo <= 2, 1, np.where(togo <= 6, 2, 3)))
    nb = dbs.loc[g.index].clip(4, 6)
    keep = g.yardline_100.astype(float) > 3
    packages = {}
    for w in range(1, 5):
        packages[str(w)] = []
        for c in range(4):
            m = keep & (wr == w) & (col == c)
            packages[str(w)].append([r((nb[m] == n).mean() if m.sum() else 0.0) for n in (4, 5, 6)] + [int(m.sum())])
    # How often the fifth back, and one of six or more, is a third safety,
    # by the offense's wide receivers.
    sf = saf.loc[g.index]
    third = {name: [r(sf[keep & (wr == w) & (nb == n)].ge(3).mean() if (keep & (wr == w) & (nb == n)).sum() else 0.0) for w in range(1, 5)]
             for name, n in (("five_backs", 5), ("six_backs", 6))}
    # The pass rush by the backs on the field: dropbacks with a charted
    # rush, their rushers, five or more, FTN's pressure flag and sacks.
    d = g[(g.qb_dropback == 1) & g.number_of_pass_rushers.notna()]
    nr, nd = d.number_of_pass_rushers.astype(float), nb.loc[d.index]
    hurried = d.was_pressure.astype(str).str.lower().isin(["true", "1", "1.0"])
    rush = {"note": "Dropbacks with a charted rush by the defense's backs on the field (4, 5, 6 or more): their share, the rushers, "
                    "five or more rushing, FTN's pressure flag (a wider count than PFR's) and sacks.",
            "columns": ["share_of_dropbacks", "rushers", "five_or_more", "pressured", "sacked"]}
    for name, n in (("four_backs", 4), ("five_backs", 5), ("six_backs", 6)):
        m = nd == n
        rush[name] = [r(m.mean()), r(nr[m].mean(), 2), r((nr[m] >= 5).mean()), r(hurried[m].mean()), r((d.sack[m] == 1).mean())]
    rush["all"] = [1.0, r(nr.mean(), 2), r((nr >= 5).mean()), r(hurried.mean()), r((d.sack == 1).mean())]
    # How far defenses differ from one another: each team season's share of
    # snaps with five or more backs and with six or more, over all snaps
    # and in the cells that hold most of the choice.
    team = g.defteam.astype(str) + g.game_id.astype(str).str[:4]
    logit = lambda x: np.log(np.clip(x, 0.005, 0.995) / (1 - np.clip(x, 0.005, 0.995)))

    def differ(mask, hit):
        t = pd.DataFrame({"team": team[mask], "h": hit[mask]}).groupby("team").h.agg(["mean", "count"])
        t = t[t["count"] >= 40]
        dev = logit(t["mean"]) - logit(hit[mask].mean())
        m = t["mean"].clip(0.02, 0.98)
        true = max(dev.var() - (1 / (t["count"] * m * (1 - m))).mean(), 0) ** 0.5
        return [r(hit[mask].mean()), r(t["mean"].quantile(0.1)), r(t["mean"].median()), r(t["mean"].quantile(0.9)), r(true), int(len(t))], dev

    light, six, early = nb >= 5, nb >= 6, col == 0
    la, dev_l = differ(keep, light)
    da, dev_d = differ(keep, six)
    by_season = pd.DataFrame({"team": g.defteam[keep], "season": g.game_id.astype(str).str[:4][keep], "l": light[keep], "d": six[keep]}) \
        .groupby(["team", "season"]).agg(l=("l", "mean"), d=("d", "mean")).unstack()
    teams = {
        "note": "Team seasons (a defense's season): the league's share, the 10th percentile, the median and the 90th of the teams' shares, "
                "their spread on the log odds with the sampling noise taken out, and the team seasons with 40 snaps or more.",
        "columns": ["league", "p10", "median", "p90", "spread_on_the_log_odds", "team_seasons"],
        "five_or_more": la,
        "six_or_more": da,
        "five_or_more_one_receiver_early": differ(keep & (wr == 1) & early, light)[0],
        "five_or_more_two_receivers_early": differ(keep & (wr == 2) & early, light)[0],
        "five_or_more_three_receivers_early": differ(keep & (wr == 3) & early, light)[0],
        "six_three_receivers_early": differ(keep & (wr == 3) & early, six)[0],
        "six_three_receivers_third_and_7": differ(keep & (wr == 3) & (col == 3), six)[0],
        "third_safety_with_five_backs": differ(keep & (nb == 5), sf >= 3)[0],
        "third_safety_with_six_backs": differ(keep & (nb == 6), sf >= 3)[0],
        "five_with_six_r": r(pd.concat([dev_l, dev_d], axis=1).corr().iloc[0, 1], 2),
        "season_to_season_r": {"five_or_more": r(by_season["l"].corr().iloc[0, 1], 2), "six_or_more": r(by_season["d"].corr().iloc[0, 1], 2)},
    }
    # The kicking team's men on kickoffs and punts (the side with the kicker
    # or the punter), and who is credited on each kind, by listed position.
    pos_of = {}
    for ids, ps in zip(pd.concat([p.offense_players, p.defense_players]), pd.concat([p.offense_positions, p.defense_positions])):
        if isinstance(ids, str) and isinstance(ps, str):
            for i, x in zip(ids.split(";"), ps.split(";")):
                pos_of.setdefault(i, {}).setdefault(x, 0)
                pos_of[i][x] += 1
    pos_of = {i: max(c, key=c.get) for i, c in pos_of.items()}
    units, unit_n, kick_tk = {}, {}, {}
    for kind, want in (("kickoff", "K"), ("punt", "P")):
        x = k[k.play_type == kind]
        u, n = {}, 0
        for a, b in zip(x.offense_positions, x.defense_positions):
            sides = [str(v).split(";") for v in (a, b) if isinstance(v, str)]
            mine = [v for v in sides if want in v and len(v) == 11]
            if len(mine) != 1:
                continue
            n += 1
            for v in mine[0]:
                u[v] = u.get(v, 0) + 1
        units[kind] = {v: r(c / max(n, 1), 2) for v, c in sorted(u.items(), key=lambda z: -z[1]) if c / max(n, 1) >= 0.005}
        unit_n[kind] = r(len(x) / tg, 2)
        t = {}
        for v in x[TK].values.ravel():
            if isinstance(v, str):
                q = pos_of.get(v, "?")
                t[q] = t.get(q, 0) + 1
        kick_tk[kind] = {v: r(c / tg, 3) for v, c in sorted(t.items(), key=lambda z: -z[1]) if c / tg >= 0.001}
    carried = s[((s.complete_pass == 1) | run) & (s.interception != 1) & (s.sack != 1)]
    ob_carried = carried[carried.out_of_bounds == 1]
    return {
        "seasons": [2023, 2024],
        "note": "Regular seasons; scrimmage plays are runs and dropbacks without kneels, spikes and tries. Every credit counts one.",
        "scrimmage": {"plays_a_team_game": r(len(s) / tg, 2), "credits_a_team_game": r(s.credits.sum() / tg, 2),
                      "tackled_plays_a_team_game": r((s.credits > 0).sum() / tg, 2), "two_or_more": r((s.credits >= 2).sum() / max((s.credits > 0).sum(), 1))},
        "untackled_a_team_game": {"incomplete": r(inc.sum() / tg, 2), "touchdown": r(td.sum() / tg, 2), "out_of_bounds": r(ob.sum() / tg, 2),
                                  "other": r((~(inc | td | ob)).sum() / tg, 2)},
        "out_of_bounds": {"a_team_game": r((s.out_of_bounds == 1).sum() / tg, 2), "with_a_tackler": r(((s.out_of_bounds == 1) & (s.credits > 0)).sum() / max((s.out_of_bounds == 1).sum(), 1))},
        "by_play": {"columns": ["a_team_game", "tackled", "credits_a_play", "two_or_more"], **by_play},
        "on_the_field": {g: r((on.get((g, "runs"), 0) + on.get((g, "dropbacks"), 0)) / len(f), 2) for g in groups},
        "credits_a_100_snaps": {"columns": ["all", "runs", "dropbacks"], **per},
        "defensive_backs_on_the_field": {str(n): r((dbs == n).mean()) for n in range(3, 8)},
        "safeties_on_the_field": {str(n): r((saf == n).mean()) for n in range(0, 5)},
        "inside_linebackers_on_the_field": {str(n): r((ilb == n).mean()) for n in range(0, 5)},
        "kicks": {"plays_a_team_game": r(len(k) / tg, 2), "credits_a_team_game": r(k.credits.sum() / tg, 2),
                  "a_team_game": unit_n, "kicking_team_by_position": units, "credits_a_team_game_by_position": kick_tk},
        "packages": {"note": "Shares of 4, 5 and 6 or more defensive backs and the snaps, by the offense's wide receivers (1 or none, 2, 3, 4 or more), "
                             "for first and second down, then third and fourth with 1-2, 3-6 and 7 or more to go; snaps inside the 3 left out. "
                             "three_or_more_safeties: their share of the snaps with five backs and with six or more, by the same receivers.",
                     **packages, "three_or_more_safeties": third, "teams": teams, "rush": rush},
        "depths": {"columns": ["share", "two_or_more", "out_of_bounds"],
                   "runs_by_yards": depths(s[run], "yards_gained", 0),
                   "catches_by_yards_after_the_catch": depths(s[s.complete_pass == 1], "yards_after_catch", 1)},
        "carried_out_of_bounds": {"a_team_game": r(len(ob_carried) / tg, 2),
                                  "credits": {str(n): r((ob_carried.credits == n).mean()) for n in range(0, 3)}},
    }


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
    "source": "ESPN college box scores (sportsdataverse-data espn_cfb_player_box) 2024-2025 with CollegeFootballData rosters (cfbfastR-data); nflverse weekly defense lines and Pro Football Reference snap counts 2021-2024; nflverse play-by-play and participation 2023-2024 (every play's tacklers, the 22 men on the field, both sides' personnel and the rushers)",
    "holes": "College defensive boxes are complete from 2024 only. No college snap counts. Rosters list many edges LB and many backs DB, and the NFL's weekly lines list 3-4 edges as LB, so the groups are the listed ones. NFL names are joined to snap counts by name, team and week (94% of tackles match).",
    "note": "FBS tackles are solo plus assists, as the boxes print them. An NFL tackle counts a solo, a tackle with an assist and an assist one each, as the league's combined tackles add up. Seasons are every game a team played (FBS: bowls and FCS games too; NFL: the 17-game regular season). Means over the seasons read.",
}

# ---- FBS
acc = Acc()
for y in (2024, 2025):
    b = pd.read_parquet(f"{D}/player_box_{y}.parquet")
    ti = pd.read_parquet(f"{D}/cfb_team_info_{y}.parquet", columns=["team_id", "classification"])
    fbs = set(ti[ti.classification == "fbs"].team_id.astype(str))
    points = box_points(b)
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
    margins(acc, d, points, fbs)
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
bm = np.mean(np.array(acc.v["by_margin"]), axis=0)
fbs["by_margin"] = {"note": "Games between two FBS teams by the final margin (from the boxes' touchdowns and kicking points): 0-7, 8-16, 17-27, 28 or more.",
                    "columns": ["share_of_games", "men_with_a_tackle", "top_man_tackles", "top_man_share", "top_11_share", "tackles"],
                    "won": [rl(x) for x in bm[:4]], "lost": [rl(x) for x in bm[4:]]}
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
    # The league's combined tackles: the first man of a "(A, B)" tackle has
    # a tackle with an assist, not a solo (2022: Oluokun 97 + 31 + 56 = 184).
    d["tackles"] = d.def_tackles_solo.fillna(0) + d.def_tackles_with_assist.fillna(0) + d.def_tackle_assists.fillna(0)
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
nfl["pbp"] = pbp_section()
out["nfl"] = nfl
print(json.dumps(out, indent=1))
