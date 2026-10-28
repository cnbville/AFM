"""Real college seasons year by year, 2001-2025 (world reader lane R3).

Reads sportsdataverse-data release files (https://github.com/sportsdataverse/sportsdataverse-data/releases), saved in DIR:
  cfb_schedules_YEAR.parquet 2001-2025 (cfb_schedules, from CollegeFootballData),
  linescores_YEAR.parquet 2004-2025 (espn_cfb_linescores),
  cfb_poll_analytics_YEAR.parquet 2004-2025 (the AP and coaches polls, every week),
  cfb_team_talent_YEAR.parquet 2005-2025 (247Sports team talent composite), cfb_team_info_YEAR.parquet 2004-2025;
and data/real-world/rivalries.csv from this folder (RIV, the second argument).

Usage: python3 college_history.py DIR RIV OUTDIR > college_history.json
Writes OUTDIR/college_rivalry_games.csv, OUTDIR/college_ap_polls.csv and OUTDIR/college_attendance.csv beside the JSON.
Development checks only, never in-game tuning.
"""
import csv, json, sys
import numpy as np
import pandas as pd

D, RIV, OUT = sys.argv[1], sys.argv[2], sys.argv[3]
DIVS = {"fbs": "FBS", "fcs": "FCS", "ii": "D2", "iii": "D3"}


def r(x, n=3):
    try:
        x = float(x)
    except (TypeError, ValueError):
        return None
    return None if np.isnan(x) else round(x, n)


def share(mask):
    return r(np.mean(mask), 4) if len(mask) else None


s = pd.concat([pd.read_parquet(f"{D}/cfb_schedules_{y}.parquet") for y in range(2001, 2026)])
s = s[s.home_points.notna() & s.away_points.notna()].copy()
s["date"] = pd.to_datetime(s.start_date, utc=True, format="ISO8601")
s["margin"] = s.home_points - s.away_points
s["post"] = s.season_type.str.contains("postseason")
out = {"source": "CollegeFootballData schedules via the sportsdataverse cfb_schedules release (2001-2025), ESPN line scores (2004-2025), the AP poll (2004-2025), 247Sports talent composite (2005-2025)",
       "holes": "No college scores before 2001 in reachable open data; FCS from 2003; D2 and D3 full seasons only from 2021-2022 (a handful of games before); no NAIA or JUCO at all.",
       "note": "Per team game means each team's game counts once. Same-division games unless a key says otherwise."}

# --- scoring by season and division ---
by = {}
for (y, dv), v in s[(s.home_division == s.away_division) & s.home_division.isin(DIVS)].groupby(["season", "home_division"]):
    if len(v) < 40:
        continue
    reg = v[~v.post]
    pts = pd.concat([reg.home_points, reg.away_points])
    m = reg.margin
    nn = reg[~reg.neutral_site.fillna(False).astype(bool)]
    by.setdefault(int(y), {})[DIVS[dv]] = {
        "games": int(len(reg)), "points_per_team_game": r(pts.mean(), 2), "points_sd": r(pts.std(), 2),
        "margin_abs_mean": r(m.abs().mean(), 2), "one_score_share": share(m.abs() <= 8), "blowout_28plus_share": share(m.abs() >= 28),
        "home_win_pct": r(((nn.margin > 0) + 0.5 * (nn.margin == 0)).mean(), 4), "shutout_team_games": int((pts == 0).sum())}
fx = s[((s.home_division == "fbs") & (s.away_division == "fcs")) | ((s.home_division == "fcs") & (s.away_division == "fbs"))]
fx = fx.assign(fbs_won=np.where(fx.home_division == "fbs", fx.margin > 0, fx.margin < 0))
fbs_teams = s[s.home_division == "fbs"].groupby("season").home_team.nunique()
for y, v in fx.groupby("season"):
    if int(y) in by and "FBS" in by[int(y)]:
        teams = set(np.where(v.home_division == "fbs", v.home_team, v.away_team))
        by[int(y)]["FBS"].update({"fbs_vs_fcs_games": int(len(v)), "fbs_vs_fcs_fbs_win_rate": r(v.fbs_won.mean(), 4),
                                  "fbs_teams_playing_fcs_share": r(len(teams) / fbs_teams.get(y, np.nan), 4)})

# overtime from ESPN line scores
ls = pd.concat([pd.read_parquet(f"{D}/linescores_{y}.parquet") for y in range(2004, 2026)])
ls["period"] = pd.to_numeric(ls.period, errors="coerce")
maxp = ls.groupby("game_id").period.max()
s["game_key"] = s.game_id.astype(str)
maxp.index = maxp.index.astype(str)
s["max_period"] = s.game_key.map(maxp)
for (y, dv), v in s[s.max_period.notna() & (s.home_division == s.away_division) & s.home_division.isin(["fbs", "fcs"])].groupby(["season", "home_division"]):
    if int(y) in by and DIVS[dv] in by[int(y)]:
        if len(v) >= 200:
            by[int(y)][DIVS[dv]].update({"overtime_share": share(v.max_period >= 5), "three_plus_ot_share": share(v.max_period >= 7),
                                         "linescore_games": int(len(v))})
out["scores_by_season"] = {"years": "FBS 2001-2025, FCS 2003-2025, D2 and D3 2021/2022-2025; overtime from 2004", "by_season": by}

fb = s[(s.home_division == "fbs") & (s.away_division == "fbs") & (s.season >= 2014) & ~s.post]
mt = fb.margin.abs().astype(int).clip(upper=50).value_counts(normalize=True).sort_index()
out["fbs_margin_table"] = {"years": "2014-2025, FBS v FBS regular season", "share_by_final_margin": {int(k): r(v, 4) for k, v in mt.items()}}

# --- the postseason: bowls, playoffs and champions ---
post = {}
for y, v in s[s.post].groupby("season"):
    f = v[(v.home_division == "fbs") | (v.away_division == "fbs")]
    cfp = f[f.playoff_competition.fillna("") == "cfp"]
    post[int(y)] = {"fbs_bowl_and_playoff_games": int(len(f)), "cfp_games": int(len(cfp)),
                    "bowl_names": sorted(set(f.notes.dropna().astype(str)))}


def winner(x):
    return x.home_team if x.home_points > x.away_points else x.away_team


champs = {}
for y in range(2001, 2026):
    v = s[s.season == y].sort_values("date")
    row = {}
    fbs_post = v[v.post & (v.home_division == "fbs") & (v.away_division == "fbs")]
    t = fbs_post[fbs_post.notes.fillna("").str.contains("national championship|BCS Championship", case=False)]
    t = t if len(t) else fbs_post.tail(1)
    if len(t):
        row["FBS_title_game_winner"] = winner(t.iloc[-1])
        row["FBS_title_game"] = str(t.iloc[-1].notes)
    for dv in ["fcs", "ii", "iii"]:
        late = v[(v.home_division == dv) & (v.away_division == dv) & (v.date.dt.month.isin([12, 1]))]
        if len(late) >= 3:
            last = late.iloc[-1]
            row[f"{DIVS[dv]}_last_game_winner"] = winner(last)
            row[f"{DIVS[dv]}_last_game"] = str(last.notes) if pd.notna(last.notes) else None
    champs[y] = row
out["postseason"] = {"years": "2001-2025", "by_season": post}
out["champions"] = {"years": "FBS 2001-2025; FCS 2003-2025; D2 and D3 where the season's last games are in the data",
                    "note": "FBS: the national title game, else the season's last FBS bowl. FCS, D2, D3: the winner of the division's last December or January game, which is the title game when the playoffs are in the data (check the game's name).",
                    "by_season": champs}

# conference champions: the title game when there is one, else the best conference record
conf = {}
for y, v in s[(s.season >= 2001) & s.conference_game.fillna(False).astype(bool)].groupby("season"):
    conf[int(y)] = {}
    reg = v[v.home_division == "fbs"]
    tg = reg[reg.notes.fillna("").str.contains("championship", case=False) & (reg.home_conference == reg.away_conference) & (reg.date.dt.month == 12)]
    for c, cv in reg[~reg.game_key.isin(tg.game_key)].groupby("home_conference"):
        if c is None or "Independent" in str(c):
            continue
        rec = {}
        for _, x in cv.iterrows():
            for team, won in ((x.home_team, x.margin > 0), (x.away_team, x.margin < 0)):
                w, l = rec.get(team, (0, 0))
                rec[team] = (w + won, l + (not won))
        best = max(w / max(w + l, 1) for w, l in rec.values())
        title = tg[tg.home_conference == c]
        conf[int(y)][c] = {"title_game_winner": winner(title.iloc[0]) if len(title) else None,
                           "best_record": sorted(t for t, (w, l) in rec.items() if w / max(w + l, 1) == best)}
out["fbs_conference_champions"] = {"years": "2001-2025", "note": "title game winner, and the team or teams with the best conference record in the regular season", "by_season": conf}

# --- the AP poll ---
info = pd.concat([pd.read_parquet(f"{D}/cfb_team_info_{y}.parquet")[["team_id", "school"]] for y in range(2004, 2026)]).drop_duplicates("team_id")
name = dict(zip(info.team_id.astype(str), info.school))
pa = pd.concat([pd.read_parquet(f"{D}/cfb_poll_analytics_{y}.parquet") for y in range(2004, 2026)])
pa = pa[(pa.poll == "ap") & pa["rank"].notna()].copy()
pa["season"] = pa.season.astype(int)
pa["week"] = pa.week.astype(int)
pa["season_type"] = pa.season_type.astype(int)
pa["team"] = pa.team_id.astype(str).map(name)
rows, polls = [], {}
for y, v in pa.groupby("season"):
    reg = v[v.season_type == 2]
    pre = reg[reg.week == reg.week.min()]
    fin = v[v.season_type == 3]
    fin = fin if len(fin) else reg[reg.week == reg.week.max()]
    fin = fin[fin.week == fin.week.max()]
    for label, p in (("preseason", pre), ("final", fin)):
        for _, x in p.sort_values("rank").iterrows():
            rows.append([int(y), label, int(x["rank"]), x.team, r(x.points, 0), r(x.first_place_votes, 0)])
    pr = dict(zip(pre.team, pre["rank"]))
    fr = dict(zip(fin.team, fin["rank"]))
    ones = v[v["rank"] == 1].groupby("team").week.nunique()
    polls[int(y)] = {"final_no1": next((t for t, k in fr.items() if k == 1), None), "preseason_no1": next((t for t, k in pr.items() if k == 1), None),
                     "preseason_no1_final_rank": fr.get(next((t for t, k in pr.items() if k == 1), None)),
                     "preseason_top25_finishing_ranked": int(sum(t in fr for t in pr)), "teams_ranked_no1": int(len(ones)),
                     "teams_ranked_any_week": int(v.team.nunique())}
with open(f"{OUT}/college_ap_polls.csv", "w", newline="") as fh:
    w = csv.writer(fh)
    w.writerow(["season", "poll", "rank", "team", "points", "first_place_votes"])
    w.writerows(rows)
out["ap_poll"] = {"years": "2004-2025", "file": "college_ap_polls.csv (preseason and final AP top 25 each season)", "by_season": polls}

# preseason strength against the season: AP preseason rank and win share
fbs_rec = {}
for _, x in s[(s.home_division == "fbs") | (s.away_division == "fbs")].iterrows():
    for team, won in ((x.home_team, x.margin > 0), (x.away_team, x.margin < 0)):
        k = (int(x.season), team)
        w, n = fbs_rec.get(k, (0, 0))
        fbs_rec[k] = (w + won, n + 1)
pre_rows = [(y, t, k) for y, label, k, t, *_ in rows if label == "preseason"]
pp = pd.DataFrame([(y, t, k, *fbs_rec.get((y, t), (np.nan, np.nan))) for y, t, k in pre_rows], columns=["season", "team", "rank", "wins", "games"]).dropna()
pp["win_pct"] = pp.wins / pp.games
out["preseason_rank_vs_season"] = {"years": "2004-2025, AP preseason top 25", "corr_rank_vs_win_pct": r(pp[["rank", "win_pct"]].corr().iloc[0, 1], 3),
                                   "win_pct_by_rank_band": {k: r(v.win_pct.mean(), 3) for k, v in pp.groupby(pd.cut(pp["rank"], [0, 5, 10, 15, 25], labels=["1-5", "6-10", "11-15", "16-25"]), observed=True)}}

# upsets with the ranks the schedules carry
rk = s[s.season >= 2004].copy()
rk["home_rank"] = pd.to_numeric(rk.home_rank, errors="coerce")
rk["away_rank"] = pd.to_numeric(rk.away_rank, errors="coerce")
top10 = rk[(rk.home_rank <= 10) & rk.away_rank.isna() | (rk.away_rank <= 10) & rk.home_rank.isna()]
lost = np.where(top10.home_rank <= 10, top10.margin < 0, top10.margin > 0)
out["unranked_beats_top10"] = {"years": "2004-2025", "by_season": {int(y): {"games": int(len(v)), "upsets": int(lost[top10.season.values == y].sum())} for y, v in top10.groupby("season")}}

# --- attendance ---
at = s[(s.home_division == "fbs") & s.attendance.notna() & (s.attendance > 0) & ~s.neutral_site.fillna(False).astype(bool) & ~s.post]
tm = at.groupby(["season", "home_team"]).attendance.mean().reset_index()
tm.to_csv(f"{OUT}/college_attendance.csv", index=False, float_format="%.0f")
out["fbs_attendance"] = {"years": f"{int(at.season.min())}-2025, FBS home games (not neutral sites)", "file": "college_attendance.csv (average home attendance per FBS team season)",
                         "by_season": {int(y): {"mean": r(v.attendance.mean(), 0), "p10": r(v.attendance.quantile(0.1), 0), "p50": r(v.attendance.median(), 0),
                                                "p90": r(v.attendance.quantile(0.9), 0)} for y, v in tm.groupby("season")}}

# --- Pythagorean fit and team records ---
tr = []
for _, x in s[(s.home_division == "fbs") & (s.away_division == "fbs")].iterrows():
    tr += [(x.season, x.home_team, x.home_points, x.away_points, (x.margin > 0) + 0.5 * (x.margin == 0)),
           (x.season, x.away_team, x.away_points, x.home_points, (x.margin < 0) + 0.5 * (x.margin == 0))]
t = pd.DataFrame(tr, columns=["season", "team", "pf", "pa", "w"]).groupby(["season", "team"]).agg(pf=("pf", "sum"), pa=("pa", "sum"), w=("w", "mean"), n=("w", "size")).reset_index()
t = t[t.n >= 8]
k = min(np.arange(1.5, 4.0, 0.01), key=lambda k: ((t.pf ** k / (t.pf ** k + t.pa ** k)) - t.w).pow(2).mean())
pk = t.pf ** k / (t.pf ** k + t.pa ** k)
out["fbs_pythagorean"] = {"years": "2001-2025, FBS v FBS games, team seasons with 8+ such games", "exponent": r(k, 2),
                          "rmse_win_pct": r(np.sqrt(((pk - t.w) ** 2).mean()), 4)}

# --- talent composite by conference ---
tal = pd.concat([pd.read_parquet(f"{D}/cfb_team_talent_{y}.parquet") for y in range(2015, 2026)])
tal["season"] = tal.season.astype(int)
confs = s[s.home_division == "fbs"].groupby(["season", "home_team"]).home_conference.first()
tal["school"] = tal.team_id.astype(str).map(name)
tal["conference"] = [confs.get((a, b)) for a, b in zip(tal.season, tal.school)]
out["talent_by_conference"] = {"years": "2015-2025, 247Sports team talent composite (CFBD)",
                               "by_season": {int(y): {c: {"mean": r(cv.talent_composite.mean(), 1), "blue_chip_ratio": r(cv.blue_chip_ratio.mean(), 3), "teams": int(len(cv))}
                                                      for c, cv in v.groupby("conference")} for y, v in tal.groupby("season")},
                               "fbs_talent_percentiles_by_season": {int(y): {f"p{q}": r(v.talent_composite.quantile(q / 100), 1) for q in (10, 50, 90, 99)}
                                                                    for y, v in tal[tal.conference.notna()].groupby("season")}}

# --- rivalry games 2001-2025 ---
ALIAS = {"Appalachian State": "App State", "Grambling State": "Grambling", "Hawaii": "Hawai'i", "Miami (FL)": "Miami"}
rv = list(csv.DictReader(open(RIV)))
with open(f"{OUT}/college_rivalry_games.csv", "w", newline="") as fh:
    w = csv.writer(fh)
    w.writerow(["season", "a", "b", "a_points", "b_points", "winner", "postseason", "neutral_site"])
    n = 0
    for row in rv:
        a, b = ALIAS.get(row["a"], row["a"]), ALIAS.get(row["b"], row["b"])
        g = s[((s.home_team == a) & (s.away_team == b)) | ((s.home_team == b) & (s.away_team == a))].sort_values("date")
        for _, x in g.iterrows():
            ap, bp = (x.home_points, x.away_points) if x.home_team == a else (x.away_points, x.home_points)
            w.writerow([int(x.season), row["a"], row["b"], int(ap), int(bp), "a" if ap > bp else "b" if bp > ap else "tie", bool(x.post),
                        bool(x.neutral_site)])
            n += 1
out["rivalry_games"] = {"years": "2001-2025 (FCS pairs from 2003)", "file": "college_rivalry_games.csv", "rows": n,
                        "note": "every game between the two schools of each rivalries.csv row, bowls included"}

# --- programs changing division ---
dv = pd.concat([s[["season", "home_team", "home_division"]].rename(columns={"home_team": "team", "home_division": "div"}),
                s[["season", "away_team", "away_division"]].rename(columns={"away_team": "team", "away_division": "div"})])
dv = dv[dv["div"].isin(DIVS)].groupby(["team", "season"])["div"].agg(lambda x: x.mode().iat[0]).reset_index().sort_values(["team", "season"])
dv["prev"] = dv.groupby("team")["div"].shift(1)
mv = dv[dv.prev.notna() & (dv["div"] != dv.prev) & (dv.season >= 2004)]
mv = mv[mv.apply(lambda x: list(DIVS).index(x["div"]) < list(DIVS).index(x.prev), axis=1)]
out["division_moves_up"] = {"years": "2004-2025 (FCS to FBS throughout; D2 to FCS only once D2 seasons are in the data, from 2021-2022)",
                            "moves": [[int(x.season), x.team, DIVS[x.prev], DIVS[x["div"]]] for _, x in mv.iterrows()]}

# --- the weekly calendar ---
wd = s[(s.home_division == "fbs") & ~s.post].copy()
wd["dow"] = (wd.date - pd.Timedelta(hours=5)).dt.day_name()
out["fbs_game_days"] = {"years": "2001-2025, FBS home games, regular season (day in US Eastern time, UTC-5)",
                        "by_season": {int(y): {k: int(n) for k, n in v.dow.value_counts().items()} for y, v in wd.groupby("season")}}

json.dump(out, sys.stdout, indent=1, default=str)
