"""Checks for figures the code wrote from memory (world reader lane R3).

Reads, saved in NFL:
  roster_YEAR.csv 1999-2025 (nflverse-data release rosters: jersey numbers by position),
  trades.csv (nflverse-data release trades), games.csv (https://github.com/nflverse/nfldata/tree/master/data: schedules);
and in SCHED: cfb_schedules_YEAR.parquet 2001-2025 (sportsdataverse-data release cfb_schedules).

What each part checks:
  jersey_numbers: the NFL number rules by era (stories/numbers.rs).
  trade_deadline: the week the NFL trade deadline followed (calendar/anchors.rs trade_deadline_week).
  college_holiday_games: which rivalry games fell on Thanksgiving or Black Friday (world/schedule.rs HOLIDAY_GAMES).
College dates are ESPN start times in UTC moved back 5 hours, so a late Friday kick stays on Friday.

Usage: python3 memory_checks.py NFL SCHED > memory_checks.json
Development checks only, never in-game tuning.
"""
import json, sys
import pandas as pd

N, SD = sys.argv[1], sys.argv[2]
GROUP = {"QB": "QB", "RB": "RB", "FB": "RB", "HB": "RB", "WR": "WR", "TE": "TE", "T": "OL", "G": "OL", "C": "OL", "OL": "OL", "OT": "OL",
         "OG": "OL", "DE": "DL", "DT": "DL", "NT": "DL", "DL": "DL", "LB": "LB", "OLB": "LB", "ILB": "LB", "MLB": "LB", "CB": "DB",
         "S": "DB", "FS": "DB", "SS": "DB", "DB": "DB", "SAF": "DB", "K": "K", "PK": "K", "P": "P", "LS": "LS"}
ERAS = [(1999, 2003), (2004, 2014), (2015, 2020), (2021, 2022), (2023, 2025)]
BANDS = [(0, 0), (1, 9), (10, 19), (20, 29), (30, 39), (40, 49), (50, 59), (60, 69), (70, 79), (80, 89), (90, 99)]

# Jersey numbers: one row per player and season.
ro = []
for y in range(1999, 2026):
    d = pd.read_csv(f"{N}/roster_{y}.csv", usecols=["season", "position", "jersey_number", "gsis_id", "full_name"], low_memory=False)
    ro.append(d.drop_duplicates(["gsis_id", "jersey_number"]))
ro = pd.concat(ro)
ro = ro[ro.jersey_number.notna()].copy()
ro["n"] = ro.jersey_number.astype(int)
ro["grp"] = ro.position.map(lambda p: GROUP.get(str(p).upper()))
ro = ro[ro.grp.notna()]
band = lambda n: next(f"{a}" if a == b else f"{a}-{b}" for a, b in BANDS if a <= n <= b)
ro["band"] = ro.n.map(band)
jersey = {}
for a, b in ERAS:
    e = ro[ro.season.between(a, b)]
    jersey[f"{a}-{b}"] = {g: {k: round(v, 4) for k, v in v.band.value_counts(normalize=True).items() if v >= 0.005}
                          for g, v in e.groupby("grp")}
firsts = {}
for label, mask in [("WR in 10-19", (ro.grp == "WR") & ro.n.between(10, 19)), ("LB in 40-49", (ro.grp == "LB") & ro.n.between(40, 49)),
                    ("RB, WR, TE, LB or DB in 1-9", ro.grp.isin(["RB", "WR", "TE", "LB", "DB"]) & ro.n.between(1, 9)),
                    ("DB in 1-9", (ro.grp == "DB") & ro.n.between(1, 9)), ("anyone in 0", ro.n == 0)]:
    hit = ro[mask]
    firsts[label] = {"first_season": int(hit.season.min()) if len(hit) else None,
                     "players_by_season": {int(k): int(v) for k, v in hit.groupby("season").full_name.nunique().items()}}

# Trade deadline: the last in-season trade each year and how many regular-season weeks were done by then.
tr = pd.read_csv(f"{N}/trades.csv", usecols=["season", "trade_date"])
tr["d"] = pd.to_datetime(tr.trade_date, errors="coerce")
g = pd.read_csv(f"{N}/games.csv", usecols=["season", "week", "game_type", "gameday"])
g = g[g.game_type == "REG"]
g["d"] = pd.to_datetime(g.gameday)
week_end = g.groupby(["season", "week"]).d.max()
deadline = {}
for y, t in tr.groupby("season"):
    ins = t[(t.d.dt.year == y) & (t.d.dt.month.between(9, 11))]
    if not len(ins) or y not in week_end.index.get_level_values(0):
        continue
    last = ins.d.max()
    done = int((week_end.loc[y] < last).sum())
    deadline[int(y)] = {"last_in_season_trade": str(last.date()), "weekday": last.day_name(), "weeks_played_before": done,
                        "trades_that_day": int((ins.d == last).sum())}

# College holiday games: which day each listed pair met in November.
PAIRS = [("Texas", "Texas A&M"), ("Ole Miss", "Mississippi State"), ("Nebraska", "Iowa"), ("Missouri", "Arkansas"), ("LSU", "Arkansas"),
         ("Washington", "Washington State"), ("Oregon", "Oregon State"), ("Toledo", "Bowling Green"), ("UCF", "South Florida"),
         ("Virginia", "Virginia Tech"), ("Pittsburgh", "West Virginia")]
games = []
for y in range(2001, 2026):
    s = pd.read_parquet(f"{SD}/cfb_schedules_{y}.parquet", columns=["season", "start_date", "home_team", "away_team", "home_division", "away_division"])
    games.append(s)
cg = pd.concat(games)
cg["d"] = pd.to_datetime(cg.start_date, format="ISO8601", utc=True) - pd.Timedelta(hours=5)
cg = cg[cg.d.dt.month == 11]
thanks = lambda y: pd.Timestamp(y, 11, 1) + pd.Timedelta(days=(3 - pd.Timestamp(y, 11, 1).weekday()) % 7 + 21)


def day(row):
    t = thanks(row.season).date()
    dd = row.d.date()
    return "thanksgiving" if dd == t else "black_friday" if dd == t + pd.Timedelta(days=1) else row.d.day_name().lower()


holiday = {}
for a, b in PAIRS:
    m = cg[((cg.home_team == a) & (cg.away_team == b)) | ((cg.home_team == b) & (cg.away_team == a))]
    by = {int(r.season): day(r) for r in m.itertuples()}
    out = {}
    for y in range(2001, 2026):
        out.setdefault(by.get(y, "not in November"), []).append(y)
    holiday[f"{a} v {b}"] = out
fbs = cg[(cg.home_division == "fbs") & (cg.away_division == "fbs")].copy()
fbs["day"] = [day(r) for r in fbs.itertuples()]
per_year = {int(y): {"thanksgiving": int((v.day == "thanksgiving").sum()), "black_friday": int((v.day == "black_friday").sum())}
            for y, v in fbs.groupby("season")}

out = {"source": "nflverse-data rosters and trades, nfldata games.csv, sportsdataverse-data cfb_schedules (ESPN)",
       "years": "NFL 1999-2025, college 2001-2025",
       "holes": "Nothing before 1999 (NFL) or 2001 (college); rosters list a player's last number of the season, so a mid-season change shows once.",
       "jersey_numbers": {"note": "share of players at each position group wearing each band, by era (bands under 0.5% left out)",
                          "by_era": jersey, "first_seasons": firsts},
       "trade_deadline": {"note": "the last trade between September and November, and how many regular-season weeks had finished by then; "
                                  "a deadline-day flurry makes this the deadline in most years", "by_season": deadline},
       "college_holiday_games": {"note": "the day each pair met in November (thanksgiving, black_friday, or the weekday); FBS v FBS games on each holiday by year",
                                 "by_pair": holiday, "fbs_games_by_year": per_year}}
json.dump(out, sys.stdout, indent=1, default=str)
