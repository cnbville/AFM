"""Real NFL seasons year by year, 1996-2025 (world reader lane R3).

Reads, saved in DIR:
  play_by_play_YEAR.parquet 1999-2025 (nflverse-data release pbp),
  games.csv (nflverse-data release schedules, the same as nfldata data/games.csv),
  standings.csv and win_totals.csv (https://github.com/nflverse/nfldata/tree/master/data),
  trades.csv (nflverse-data release trades),
  nfl_games.csv (FiveThirtyEight, https://raw.githubusercontent.com/fivethirtyeight/nfl-elo-game/master/data/nfl_games.csv)
  for the 1996-1998 scores the nflverse files do not cover.

Usage: python3 nfl_history.py DIR > nfl_history.json
Development checks only, never in-game tuning.
"""
import json, sys
import numpy as np
import pandas as pd

D = sys.argv[1]
YEARS = range(1999, 2026)


def r(x, n=3):
    try:
        x = float(x)
    except (TypeError, ValueError):
        return None
    return None if np.isnan(x) else round(x, n)


def share(mask):
    return r(np.mean(mask), 4) if len(mask) else None


out = {"source": "nflverse play-by-play 1999-2025, nflverse schedules (games.csv) 1999-2025, nfldata standings 2002+ and win totals, nflverse trades 2002+, FiveThirtyEight nfl_games.csv for 1996-1998 scores",
       "note": "Regular seasons unless a key says playoffs. Per team game means each team's game counts once (two per game)."}

# --- scores per season, 1996-2025 ---
g = pd.read_csv(f"{D}/games.csv", low_memory=False)
g = g[g.home_score.notna() & (g.season <= 2025)]
f8 = pd.read_csv(f"{D}/nfl_games.csv")
f8 = f8[f8.season.between(1996, 1998)]
reg_old = f8[f8.playoff == 0]
scores = {}
for y in range(1996, 2026):
    if y < 1999:
        v = reg_old[reg_old.season == y]
        hs, as_, neutral = v.score1, v.score2, v.neutral == 1
        ot = None
    else:
        v = g[(g.season == y) & (g.game_type == "REG")]
        hs, as_, neutral = v.home_score, v.away_score, v.location == "Neutral"
        ot = share(v.overtime == 1)
    pts = pd.concat([hs, as_])
    m = (hs - as_)
    scores[y] = {"games": int(len(v)), "points_per_team_game": r(pts.mean(), 2), "points_sd": r(pts.std(), 2),
                 "total_per_game": r((hs + as_).mean(), 2), "margin_abs_mean": r(m.abs().mean(), 2),
                 "one_score_share": share(m.abs() <= 8), "blowout_21plus_share": share(m.abs() >= 21),
                 "home_win_pct": r(((m > 0) + 0.5 * (m == 0))[~neutral].mean(), 4), "ties": int((m == 0).sum()),
                 "overtime_share": ot, "shutout_team_games": int((pts == 0).sum())}
out["scoring_by_season"] = {"years": "1996-2025 (1996-1998 scores only, from FiveThirtyEight)", "by_season": scores}

# --- play by play per season ---
cols = ["season", "season_type", "game_id", "posteam", "play_type", "down", "ydstogo", "yardline_100", "qb_dropback", "qb_scramble",
        "sack", "interception", "complete_pass", "pass_attempt", "rush_attempt", "yards_gained", "epa", "shotgun", "no_huddle",
        "field_goal_attempt", "field_goal_result", "kick_distance", "extra_point_attempt", "extra_point_result", "two_point_attempt",
        "two_point_conv_result", "kickoff_attempt", "touchback", "punt_attempt", "penalty", "penalty_team", "penalty_type", "temp", "wind",
        "roof", "qb_kneel", "qb_spike", "return_yards", "punt_fair_catch", "kickoff_fair_catch", "fumble_lost", "qtr", "wp",
        "half_seconds_remaining", "fixed_drive", "fixed_drive_result", "punt_blocked"]
FG_BANDS = [(0, 29, "under_30"), (30, 39, "30_39"), (40, 49, "40_49"), (50, 70, "50_plus")]
seasons, pen_kinds, weather_rows = {}, {}, []
for y in YEARS:
    p = pd.read_parquet(f"{D}/play_by_play_{y}.parquet", columns=cols)
    p = p[p.season_type == "REG"]
    tg = p.game_id.nunique() * 2
    scrim = p[p.play_type.isin(["pass", "run"]) & (p.qb_kneel != 1) & (p.qb_spike != 1)]
    drop = scrim[scrim.qb_dropback == 1]
    designed = scrim[(scrim.rush_attempt == 1) & (scrim.qb_scramble != 1)]
    att = scrim[(scrim.pass_attempt == 1) & (scrim.sack != 1)]
    neutral = scrim[scrim.down.isin([1, 2]) & scrim.wp.between(0.2, 0.8) & (scrim.half_seconds_remaining > 120)]
    fg = p[p.field_goal_attempt == 1]
    fgb = {}
    for lo, hi, k in FG_BANDS:
        b = fg[fg.kick_distance.between(lo, hi)]
        fgb[k] = {"att_per_team_game": r(len(b) / tg, 3), "made_pct": share(b.field_goal_result == "made")}
    xp = p[p.extra_point_attempt == 1]
    two = p[p.two_point_attempt == 1]
    four = p[(p.down == 4) & p.play_type.isin(["pass", "run", "punt", "field_goal"])]
    go = four[four.play_type.isin(["pass", "run"])]
    short_opp = four[(four.ydstogo <= 3) & (four.yardline_100 <= 50)]
    ko = p[p.kickoff_attempt == 1]
    ko_ret = ko[(ko.touchback != 1) & (ko.kickoff_fair_catch != 1) & ko.return_yards.notna()]
    pu = p[(p.punt_attempt == 1) & (p.punt_blocked != 1)]
    pu_ret = pu[(pu.punt_fair_catch != 1) & (pu.touchback != 1) & (pu.return_yards > 0)]
    pen = p[(p.penalty == 1) & p.penalty_team.notna()]
    drives = p.groupby(["game_id", "posteam"]).fixed_drive.nunique().sum()
    dres = p.drop_duplicates(["game_id", "fixed_drive"]).fixed_drive_result.value_counts(normalize=True)
    seasons[y] = {
        "plays_per_team_game": r(len(scrim) / tg, 2), "dropback_rate": r(len(drop) / max(len(drop) + len(designed), 1), 4),
        "neutral_dropback_rate": r((neutral.qb_dropback == 1).mean(), 4),
        "shotgun_share": share(scrim.shotgun == 1), "no_huddle_share": share(scrim.no_huddle == 1),
        "completion_pct": share(att.complete_pass == 1), "yards_per_attempt": r(att.yards_gained.mean(), 3),
        "sack_rate": share(drop.sack == 1), "int_rate": share(att.interception == 1),
        "designed_run_ypc": r(designed.yards_gained.mean(), 3), "yards_per_play": r(scrim.yards_gained.mean(), 3),
        "epa_per_play": r(scrim.epa.mean(), 4), "turnovers_per_team_game": r((scrim.interception.sum() + p.fumble_lost.sum()) / tg, 3),
        "fg_by_distance": fgb, "fg_att_per_team_game": r(len(fg) / tg, 3), "fg_pct": share(fg.field_goal_result == "made"),
        "xp_att_per_team_game": r(len(xp) / tg, 3), "xp_pct": share(xp.extra_point_result == "good"),
        "two_point_tries_per_team_game": r(len(two) / tg, 3), "two_point_conv": share(two.two_point_conv_result == "success"),
        "fourth_down_go_rate": share(four.play_type.isin(["pass", "run"])),
        "fourth_short_opp_go_rate": share(short_opp.play_type.isin(["pass", "run"])),
        "fourth_down_conv": share(go.yards_gained >= go.ydstogo),
        "punts_per_team_game": r(len(pu) / tg, 3), "punt_gross": r(pu.kick_distance.mean(), 2),
        "punt_fair_catch_rate": share(pu.punt_fair_catch == 1), "punt_return_avg": r(pu_ret.return_yards.mean(), 2),
        "kickoff_touchback_rate": share(ko.touchback == 1), "kickoff_return_avg": r(ko_ret.return_yards.mean(), 2),
        "kickoff_fair_catch_rate": share(ko.kickoff_fair_catch == 1),
        "penalties_per_team_game": r(len(pen) / tg, 3), "drives_per_team_game": r(drives / tg, 2),
        "drive_results": {str(k): r(v, 4) for k, v in dres.items()},
    }
    pen_kinds[y] = {str(k): r(n / tg, 4) for k, n in pen.penalty_type.value_counts().head(20).items()}
    w = p[p.roof.isin(["outdoors", "open"])]
    wg = w.groupby("game_id").agg(temp=("temp", "first"), wind=("wind", "first"))
    weather_rows.append(pd.concat([att[att.game_id.isin(wg.index)].assign(kind="att"), fg[fg.game_id.isin(wg.index)].assign(kind="fg")])
                        [["game_id", "kind", "complete_pass", "yards_gained", "interception", "field_goal_result", "kick_distance"]]
                        .join(wg, on="game_id"))
out["plays_by_season"] = {"years": "1999-2025", "by_season": seasons}
out["penalty_kinds_per_team_game"] = {"years": "1999-2025", "note": "accepted penalties, the 20 most common kinds", "by_season": pen_kinds}

wx = pd.concat(weather_rows)
wx["temp_band"] = pd.cut(wx.temp, [-50, 31, 49, 69, 120], labels=["under_32", "32_49", "50_69", "70_plus"])
wx["wind_band"] = pd.cut(wx.wind, [-1, 4, 9, 14, 60], labels=["0_4", "5_9", "10_14", "15_plus"])


def wsplit(col):
    o = {}
    for k, v in wx.groupby(col, observed=True):
        a, f = v[v.kind == "att"], v[v.kind == "fg"]
        o[str(k)] = {"attempts": int(len(a)), "completion_pct": share(a.complete_pass == 1), "yards_per_attempt": r(a.yards_gained.mean(), 3),
                     "int_rate": share(a.interception == 1), "fg_att": int(len(f)), "fg_pct": share(f.field_goal_result == "made"),
                     "fg_pct_40_plus": share(f[f.kick_distance >= 40].field_goal_result == "made")}
    return o


gw = g[(g.game_type == "REG") & g.roof.isin(["outdoors", "open"])].copy()
gw["pts"] = gw.home_score + gw.away_score
out["weather_splits"] = {"years": "1999-2025, outdoor and open-roof games with a reading (about 77% of them)",
                         "by_temp_f": wsplit("temp_band"), "by_wind_mph": wsplit("wind_band"),
                         "total_points_by_temp_f": {str(k): r(v.pts.mean(), 2) for k, v in gw.groupby(pd.cut(gw.temp, [-50, 31, 49, 69, 120], labels=["under_32", "32_49", "50_69", "70_plus"]), observed=True)},
                         "total_points_by_wind_mph": {str(k): r(v.pts.mean(), 2) for k, v in gw.groupby(pd.cut(gw.wind, [-1, 4, 9, 14, 60], labels=["0_4", "5_9", "10_14", "15_plus"]), observed=True)},
                         "dome_total_points": r((g[(g.game_type == "REG") & (g.roof == "dome")].home_score + g[(g.game_type == "REG") & (g.roof == "dome")].away_score).mean(), 2)}

# --- champions, records and parity ---
champ = {}
for y in range(1996, 2026):
    if y < 1999:
        v = f8[(f8.season == y) & (f8.playoff == 1)].sort_values("date")
        last = v.iloc[-1]
        champ[y] = {"super_bowl_winner": last.team1 if last.score1 > last.score2 else last.team2, "playoff_games": int(len(v))}
    else:
        v = g[(g.season == y) & (g.game_type != "REG")]
        sb = v[v.game_type == "SB"]
        con = v[v.game_type == "CON"]
        if sb.empty:
            continue
        s = sb.iloc[0]
        champ[y] = {"super_bowl_winner": s.home_team if s.home_score > s.away_score else s.away_team,
                    "super_bowl_loser": s.away_team if s.home_score > s.away_score else s.home_team,
                    "super_bowl_score": f"{max(s.home_score, s.away_score):.0f}-{min(s.home_score, s.away_score):.0f}",
                    "playoff_teams": int(pd.concat([v.home_team, v.away_team]).nunique()),
                    "conference_finalists": sorted(set(con.home_team) | set(con.away_team))}
out["champions"] = {"years": "1996-2025 (team codes as in nflverse; 1996-1998 FiveThirtyEight codes)", "by_season": champ}


def team_wins():
    rows = []
    for _, x in reg_old.iterrows():
        rows += [(x.season, x.team1, (x.score1 > x.score2) + 0.5 * (x.score1 == x.score2), x.score1, x.score2),
                 (x.season, x.team2, (x.score2 > x.score1) + 0.5 * (x.score1 == x.score2), x.score2, x.score1)]
    reg = g[g.game_type == "REG"]
    for _, x in reg.iterrows():
        rows += [(x.season, x.home_team, (x.home_score > x.away_score) + 0.5 * (x.home_score == x.away_score), x.home_score, x.away_score),
                 (x.season, x.away_team, (x.away_score > x.home_score) + 0.5 * (x.home_score == x.away_score), x.away_score, x.home_score)]
    t = pd.DataFrame(rows, columns=["season", "team", "w", "pf", "pa"])
    return t.groupby(["season", "team"]).agg(wins=("w", "sum"), games=("w", "size"), pf=("pf", "sum"), pa=("pa", "sum")).reset_index()


tw = team_wins()
tw = tw[tw.season >= 1999]  # the FiveThirtyEight codes differ, so year-to-year links start in 1999
tw["pct"] = tw.wins / tw.games
nxt = tw.merge(tw.assign(season=tw.season - 1)[["season", "team", "pct"]], on=["season", "team"], suffixes=("", "_next"))
corr = {f"{a}-{b}": r(nxt[nxt.season.between(a, b)][["pct", "pct_next"]].corr().iloc[0, 1], 3) for a, b in [(1999, 2009), (2010, 2019), (2020, 2024), (1999, 2024)]}
winners = [champ[y]["super_bowl_winner"] for y in sorted(champ)]
yrs = sorted(champ)
windows = {f"{yrs[i]}-{yrs[i + 14]}": {"distinct_champions": len(set(winners[i:i + 15])), "most_titles": int(pd.Series(winners[i:i + 15]).value_counts().iat[0])}
           for i in range(0, len(yrs) - 14)}
tw["pyth_pct"] = tw.pf ** 2.37 / (tw.pf ** 2.37 + tw.pa ** 2.37)
best_k = min(np.arange(1.5, 3.5, 0.01), key=lambda k: ((tw.pf ** k / (tw.pf ** k + tw.pa ** k)) - tw.pct).pow(2).mean())
pk = tw.pf ** best_k / (tw.pf ** best_k + tw.pa ** best_k)
st = pd.read_csv(f"{D}/standings.csv")
st = st[st.season <= 2025]
st = st.sort_values(["team", "season"])
st["prev_pct"] = st.groupby("team").pct.shift(1)
st["prev_seed"] = st.groupby("team").seed.shift(1)
st["has_prev"] = st.groupby("team").season.shift(1).notna()
dw = st[(st.div_rank == 1) & st.prev_pct.notna()]
po = st[st.seed.notna() & st.has_prev]
out["parity"] = {
    "win_pct_year_to_year_corr": corr,
    "super_bowl_15yr_windows": windows,
    "repeat_champions": int(sum(1 for a, b in zip(winners, winners[1:]) if a == b)),
    "team_win_pct_sd_by_season": {int(y): r(v.pct.std(), 4) for y, v in tw.groupby("season")},
    "best_record_by_season": {int(y): f"{v.wins.max():g}-{(v.games - v.wins).loc[v.wins.idxmax()]:g}" for y, v in tw.groupby("season")},
    "pythagorean_exponent": r(best_k, 2), "pythagorean_rmse_win_pct": r(np.sqrt(((pk - tw.pct) ** 2).mean()), 4),
    "division_winners_from_losing_record_share": r((dw.prev_pct < 0.5).mean(), 4),
    "playoff_teams_new_each_year_share": r(po.prev_seed.isna().mean(), 4),
    "years": "1999-2025 for team records; 1996-2025 for champions; standings from 2002",
}

# --- betting lines and preseason win totals ---
b = g[(g.game_type == "REG") & g.spread_line.notna()].copy()
b["fav_spread"] = b.spread_line.abs()
b["fav_won"] = np.where(b.spread_line > 0, b.result > 0, np.where(b.spread_line < 0, b.result < 0, np.nan))
b["ats"] = b.result - b.spread_line
bands = pd.cut(b.fav_spread, [-0.1, 2.5, 3.5, 6.5, 9.5, 13.5, 40], labels=["0_2.5", "3_3.5", "4_6.5", "7_9.5", "10_13.5", "14_plus"])
out["betting_lines"] = {
    "years": f"{int(b.season.min())}-2025, closing lines (positive spread_line = home favored)",
    "favorite_win_rate_by_spread": {str(k): {"games": int(len(v)), "fav_win": r(v.fav_won.mean(), 4)} for k, v in b[b.spread_line != 0].groupby(bands, observed=True)},
    "spread_mae": r(b.ats.abs().mean(), 3), "home_cover_rate": share(b.ats > 0), "push_rate": share(b.ats == 0),
    "total_mae": r((b.total - b.total_line).abs().mean(), 3), "over_rate": share(b.total > b.total_line),
    "dog_7plus_win_rate": r(b[b.fav_spread >= 7].fav_won.rsub(1).mean(), 4),
    "spread_mae_by_season": {int(y): r(v.ats.abs().mean(), 3) for y, v in b.groupby("season")},
}
wt = pd.read_csv(f"{D}/win_totals.csv")
wt = wt.groupby(["season", "team"]).line.mean().reset_index()
twj = tw.merge(wt, on=["season", "team"])
out["preseason_win_totals"] = {"years": f"{int(twj.season.min())}-{int(twj.season.max())}", "teams": int(len(twj)),
                               "corr_line_vs_wins": r(twj[["line", "wins"]].corr().iloc[0, 1], 3),
                               "mae_wins": r((twj.wins - twj.line).abs().mean(), 3), "sd_wins_minus_line": r((twj.wins - twj.line).std(), 3),
                               "over_rate": share(twj.wins > twj.line)}

# --- head coaches from the game records ---
rows = []
for _, x in g[g.game_type == "REG"].sort_values(["season", "week"]).iterrows():
    rows += [(x.season, x.week, x.home_team, x.home_coach), (x.season, x.week, x.away_team, x.away_coach)]
hc = pd.DataFrame(rows, columns=["season", "week", "team", "coach"]).dropna()
ts = hc.groupby(["team", "season"]).agg(first=("coach", "first"), last=("coach", "last"), n=("coach", "nunique")).reset_index()
ts["prev_last"] = ts.groupby("team")["last"].shift(1)
chg = ts[ts.prev_last.notna()]
out["head_coaches"] = {
    "years": "1999-2025 (who coached each regular-season game; a change is any new head coach, fired, resigned or retired)",
    "offseason_changes_by_season": {int(y): int((v["first"] != v.prev_last).sum()) for y, v in chg.groupby("season")},
    "in_season_changes_by_season": {int(y): int((v.n > 1).sum()) for y, v in ts.groupby("season")},
}
ten, cur = [], {}
for (team, coach), v in hc.groupby(["team", "coach"]):
    yrs_c = sorted(v.season.unique())
    run = [yrs_c[0]]
    for a in yrs_c[1:]:
        if a == run[-1] + 1:
            run.append(a)
        else:
            ten.append((team, coach, run[0], run[-1]))
            run = [a]
    ten.append((team, coach, run[0], run[-1]))
tn = pd.DataFrame(ten, columns=["team", "coach", "start", "end"])
tn["years"] = tn.end - tn.start + 1
closed = tn[(tn.start >= 2000) & (tn.end <= 2024)]
started = tn[tn.start.between(2000, 2019)]
out["head_coaches"]["completed_tenure_years"] = {f"p{q}": r(np.percentile(closed.years, q), 1) for q in (10, 25, 50, 75, 90)}
out["head_coaches"]["tenures_started_2000_2019"] = int(len(started))
out["head_coaches"]["share_lasting_at_least"] = {int(k): r((started.years >= k).mean(), 4) for k in (2, 3, 4, 5, 8, 10)}

tr = pd.read_csv(f"{D}/trades.csv")
tr["month"] = pd.to_datetime(tr.trade_date, errors="coerce").dt.month
tg_ = tr.groupby("trade_id").agg(season=("season", "first"), month=("month", "first"), picks=("pick_round", lambda s: int(s.notna().sum())),
                                 players=("pfr_id", lambda s: int(s.notna().sum())))
out["trades"] = {"years": "2002-2025 (nflverse trades, from Pro Football Reference and news)",
                 "by_season": {int(y): {"trades": int(len(v)), "players_moved": int(v.players.sum()), "picks_moved": int(v.picks.sum()),
                                        "in_season_sep_to_nov": int(v.month.isin([9, 10, 11]).sum())}
                               for y, v in tg_[tg_.season <= 2025].groupby("season")}}

# --- schedule facts ---
ABROAD = ["Wembley", "Tottenham", "Twickenham", "Azteca", "Banorte", "Allianz", "Bayern", "Deutsche Bank Park", "Rogers Centre",
          "Corinthians", "Bernabeu", "Maracana", "Croke", "Melbourne", "MCG", "Stade de France"]
sched = {}
for y, v in g[g.game_type == "REG"].groupby("season"):
    d = pd.to_datetime(v.gameday)
    usual = v[v.location == "Home"].groupby("home_team").stadium.agg(lambda x: x.mode().iat[0])
    neutral = v.location == "Neutral"
    # games abroad: a foreign stadium (the Bills' Toronto games count as home games), or a neutral game listed at the home team's own stadium (how nflverse lists 2025's)
    abroad = v.stadium.fillna("").str.contains("|".join(ABROAD)) | (neutral & (v.stadium == v.home_team.map(usual)))
    sched[int(y)] = {"games_abroad": int(abroad.sum()),
                     "christmas_day": int(((d.dt.month == 12) & (d.dt.day == 25)).sum()),
                     "thanksgiving": int(((d.dt.month == 11) & (v.weekday == "Thursday") & d.dt.day.between(22, 28)).sum()),
                     "thursday": int((v.weekday == "Thursday").sum()), "saturday": int((v.weekday == "Saturday").sum()),
                     "monday": int((v.weekday == "Monday").sum()),
                     "sunday_night": int(((v.weekday == "Sunday") & (v.gametime >= "20:00")).sum()),
                     "neutral_site": int((v.location == "Neutral").sum())}
out["schedule_facts"] = {"years": "1999-2025", "by_season": sched}

json.dump(out, sys.stdout, indent=1, default=str)
