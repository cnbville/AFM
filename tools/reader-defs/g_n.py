from common import *

NP = "nfl"
def DT(d): return f"date({d} * 86400, 'unixepoch')"
def club_now(t): return f"(SELECT name FROM nfl WHERE _row = {t})"
DIVN = "(CASE WHEN {y} < 2002 THEN CASE {d} % 4 WHEN 0 THEN 'East' WHEN 1 THEN 'Central' ELSE 'West' END ELSE CASE {d} % 4 WHEN 0 THEN 'East' WHEN 1 THEN 'North' WHEN 2 THEN 'South' ELSE 'West' END END)"
YEARS = "yrs(y) AS (SELECT 1984 UNION ALL SELECT y + 1 FROM yrs WHERE y < (SELECT MAX(year) FROM nfl_seasons))"

# ---------------------------------------------------------------- N1 franchises
panel("N1.eras", NP, "N1", "NFL franchises by era: name, city, division, roof", "table",
  "Every franchise era on its real dates (franchise.rs ERAS, real history): name, home city and state, conference and division, roof, and the fictional name the fictional-league option shows. A gap between eras is a franchise gone dark (Cleveland 1996-98).",
  f"""SELECT f.team, f."from", f."to", f.name, f.fictional, f.city, f.state, f.conference, {DIVN.format(y='f."from"', d='f.div')} AS division, f.roof
       FROM engine_franchises f WHERE f."to" >= 1984 ORDER BY f.team, f."from" """,
  says=f"""WITH RECURSIVE {YEARS} SELECT printf('%d clubs play in %d (real 32 since 2002); %d franchise eras since 1984.', (SELECT COUNT(DISTINCT team) FROM engine_franchises WHERE (SELECT MAX(year) FROM nfl_seasons) BETWEEN "from" AND "to"),
     (SELECT MAX(year) FROM nfl_seasons), (SELECT COUNT(*) FROM engine_franchises WHERE "to" >= 1984))""", open="e/nfl/{team}")
panel("N1.count", NP, "N1", "Clubs in the league each year", "line",
  "Franchises active each year from the engine's era table, against the real count: 28 to 1994, 30 from 1995 (Carolina and Jacksonville), 30 through 1998 with Cleveland dark, 31 from 1999, 32 from 2002 (Houston). Moves after 2026 are the owners' votes.",
  f"""WITH RECURSIVE {YEARS} SELECT 'Clubs' AS grp, y AS year, (SELECT COUNT(DISTINCT team) FROM engine_franchises WHERE y BETWEEN "from" AND "to") AS world,
            CASE WHEN y < 1995 THEN 28 WHEN y < 1999 THEN 30 WHEN y < 2002 THEN 31 ELSE 32 END AS real, NULL AS n FROM yrs ORDER BY y""")
check("N1.clubs_1990", NP, "N1", "Clubs in 1990", "clubs", "SELECT COUNT(DISTINCT team) FROM engine_franchises WHERE 1990 BETWEEN \"from\" AND \"to\"",
  Fixed(28.0, "NFL history: 28 clubs 1976-1994", "1990"), (0.0, 0.04, 0.0), "Franchises active in 1990 in the engine's era table.")
check("N1.clubs_1997", NP, "N1", "Clubs in 1997 (Cleveland dark)", "clubs", "SELECT COUNT(DISTINCT team) FROM engine_franchises WHERE 1997 BETWEEN \"from\" AND \"to\"",
  Fixed(30.0, "NFL history: 30 clubs 1995-1998, Cleveland dark 1996-98", "1997"), (0.0, 0.04, 0.0), "Franchises active in 1997.")
check("N1.clubs_2005", NP, "N1", "Clubs in 2005", "clubs", "SELECT COUNT(DISTINCT team) FROM engine_franchises WHERE 2005 BETWEEN \"from\" AND \"to\"",
  Fixed(32.0, "NFL history: 32 clubs since Houston joined in 2002", "2005"), (0.0, 0.04, 0.0), "Franchises active in 2005.")

# ---------------------------------------------------------------- N2 seasons
ROUND = "(CASE {r} WHEN 0 THEN 'out' WHEN 1 THEN 'wild card' WHEN 2 THEN 'divisional' WHEN 3 THEN 'conference title' WHEN 4 THEN 'Super Bowl' WHEN 5 THEN 'champion' ELSE {r} END)"
panel("N2.standings", NP, "N2", "Every NFL team-season: record, coach and how far it went", "table",
  "Each club's season: wins, losses and ties, the head coach who finished it, and the round reached. Points for and against come from the game log where it holds the season.",
  f"""SELECT s.year, {club('s.team', 's.year')} AS club, s.team AS id, s.wins || '-' || s.losses || CASE WHEN s.ties > 0 THEN '-' || s.ties ELSE '' END AS record,
            (SELECT name FROM coaches WHERE id = s.coach) AS coach, {ROUND.format(r='s.round')} AS reached,
            (SELECT SUM(CASE WHEN g.home = s.team THEN g.home_pts ELSE g.away_pts END) FROM log_games g WHERE g.stream = 'nfl' AND g.season = s.year AND COALESCE(g.kind, 'regular') = 'regular' AND s.team IN (g.home, g.away)) AS "points for",
            (SELECT SUM(CASE WHEN g.home = s.team THEN g.away_pts ELSE g.home_pts END) FROM log_games g WHERE g.stream = 'nfl' AND g.season = s.year AND COALESCE(g.kind, 'regular') = 'regular' AND s.team IN (g.home, g.away)) AS "points against"
       FROM nfl_seasons s ORDER BY s.year DESC, s.wins DESC""",
  says=f"""SELECT printf('%d team-seasons over %d years: %.1f a year won 13 or more (real about 3-4), %.1f won 3 or fewer.', COUNT(*), COUNT(DISTINCT year),
     1.0 * SUM(wins >= 13) / COUNT(DISTINCT year), 1.0 * SUM(wins <= 3) / COUNT(DISTINCT year)) FROM nfl_seasons""", open="club/{id}/{year}")
panel("N2.wins", NP, "N2", "Win totals: how seasons spread", "hist",
  "Every team-season by wins (regular season). Real spread: about 3-4 clubs a year win 13+ and as many win 3 or fewer; an unbeaten regular season is very rare (one 16-0, 2007).",
  "SELECT 'Team-seasons' AS grp, wins AS x, 100.0 * COUNT(*) / SUM(COUNT(*)) OVER () AS world, COUNT(*) AS n FROM nfl_seasons GROUP BY wins ORDER BY wins")
panel("N2.champions", NP, "N2", "Super Bowl champions by year", "table",
  "Each season's champion, with its record and coach; repeat champions stand out.",
  f"""SELECT c._0 AS year, {club('c._1', 'c._0')} AS champion, c._1 AS id, (SELECT wins || '-' || losses FROM nfl_seasons s WHERE s.year = c._0 AND s.team = c._1) AS record,
            (SELECT name FROM coaches WHERE id = (SELECT coach FROM nfl_seasons s WHERE s.year = c._0 AND s.team = c._1)) AS coach,
            CASE WHEN (SELECT _1 FROM nfl_champions p WHERE p._0 = c._0 - 1) = c._1 THEN 'repeat' END AS repeat
       FROM nfl_champions c ORDER BY c._0 DESC""", open="club/{id}/{year}")
check("N2.big_seasons", NP, "N2", "NFL clubs winning 13 or more a year", "a year",
  "SELECT 1.0 * SUM(wins >= 13) / COUNT(DISTINCT year) FROM nfl_seasons", Memory(3.5, "about 3-4 clubs a year since 1984 (from memory)"), (0.3, 0.7, 0.5),
  "Team-seasons with 13+ wins per year.")
check("N2.ties", NP, "N2", "NFL ties a year", "a year", "SELECT 0.5 * SUM(ties) / COUNT(DISTINCT year) FROM nfl_seasons",
  Memory(1.0, "about 0-2 ties a year since 2012 (from memory)"), (0.6, 1.5, 0.5), "Tied games per season (each tie counts on both clubs).")
rule("N2.one_coach_one_club", "N2", "No head coach finishes two clubs' seasons in one year", "SELECT COUNT(*) FROM nfl_seasons WHERE coach IS NOT NULL",
  """SELECT year, coach, (SELECT name FROM coaches WHERE id = s.coach) AS name, COUNT(*) AS clubs FROM nfl_seasons s WHERE coach IS NOT NULL GROUP BY year, coach HAVING COUNT(*) > 1""",
  "The carousel moves a coach between clubs, but only one club's season can end with him.")

# ---------------------------------------------------------------- N3 schedule
def slots(src, yr):
    return " UNION ALL ".join(
        f"""SELECT {yr} AS year, s._row AS week, {k} AS k, s._{k}__at__date AS date, s._{k}__window AS window, s._{k}__score AS score, s._{k}__line AS line,
                  s._{k}__attendance AS att, json_extract(w._{k}, '$[0]') AS home, json_extract(w._{k}, '$[1]') AS away
             FROM {src}__slots s JOIN {src}__weeks w ON w._row = s._row WHERE s._{k}__at__date IS NOT NULL""" for k in range(16))
SL = f"sl AS ({slots('nfl_season', '(SELECT year FROM nfl_season)')} UNION ALL {slots('nfl_prev', '(SELECT year FROM nfl_prev)')})"
for src in ["nfl_season", "nfl_prev"]:
    for k in range(16):
        for f in ["at__date", "window", "score", "line", "attendance"]:
            col(f"{src}__slots", f"_{k}__{f}")
        col(f"{src}__weeks", f"_{k}")
PRIME = "('SundayNight', 'MondayNight', 'ThursdayNight', 'Opener')"
panel("N3.windows", NP, "N3", "Games by TV window, this season and last", "bars",
  "Every scheduled game by its window: Sunday afternoon, Sunday, Monday and Thursday night, the opener, Thanksgiving, Black Friday, Christmas, Saturday and games abroad. Real 2026 (nfl_schedule_2026.csv): 9 games abroad; recent seasons about 17 Monday, 17 Sunday night and 16 Thursday night games.",
  f"WITH {SL} SELECT year AS grp, COALESCE(window, 'unset') AS label, COUNT(*) AS world, NULL AS real, COUNT(*) AS n FROM sl GROUP BY year, window ORDER BY year, world DESC")
panel("N3.primetime", NP, "N3", "Primetime and big games by club", "table",
  "Each club's night games (Sunday, Monday, Thursday and the opener), holiday games and games abroad this season and last, with its huge-game count. Real: 1 to 6 primetime games a year, the big brands most.",
  f"""WITH {SL}, t AS (SELECT year, home AS team, window FROM sl UNION ALL SELECT year, away, window FROM sl)
     SELECT year, {club('t.team', 't.year')} AS club, team AS id, SUM(window IN {PRIME}) AS primetime, SUM(window IN ('Thanksgiving', 'Christmas', 'BlackFriday')) AS holiday,
            SUM(window = 'International') AS abroad, COUNT(*) AS games
       FROM t GROUP BY year, team ORDER BY year DESC, primetime DESC""", open="club/{id}/{year}")
panel("N3.byes", NP, "N3", "Bye weeks this season", "bars",
  "Clubs on bye in each week of this season (week counted from the first game). Real: byes fall between weeks 5 and 14.",
  f"""WITH b AS (SELECT CAST((j.value - (SELECT MIN(_0__at__date) FROM nfl_season__slots)) / 7 AS INTEGER) + 1 AS week FROM nfl_season, json_each(nfl_season.byes) j)
     SELECT 'Clubs on bye' AS grp, 'Week ' || week AS label, COUNT(*) AS world, NULL AS real, COUNT(*) AS n FROM b GROUP BY week ORDER BY week""")
col("nfl_season", "byes")
check("N3.abroad", NP, "N3", "NFL games abroad this season", "games", f"WITH {SL} SELECT SUM(window = 'International') FROM sl WHERE year = (SELECT year FROM nfl_season)",
  Fixed(9.0, "nfl_schedule_2026.csv: 9 games abroad in 2026 (1 in 2007, rising since)", "2026"), (0.25, 0.6, 1.0), "Games played at a site abroad this season. Real rose from 1 in 2007 to 9 in 2026, so an older season reads lower.")
check("N3.primetime_max", NP, "N3", "Most primetime games for one club in a season", "games",
  f"WITH {SL}, t AS (SELECT year, home AS team, window FROM sl UNION ALL SELECT year, away, window FROM sl) SELECT MAX(n) FROM (SELECT SUM(window IN {PRIME}) AS n FROM t GROUP BY year, team)",
  Memory(6.0, "real: 1 to 6 primetime games a club a year, the big brands most (from memory)"), (0.2, 0.5, 0.0), "The busiest club's night games in a season.")
rule("N3.byes_in_window", "N3", "Every bye falls between weeks 5 and 14", "SELECT json_array_length(byes) FROM nfl_season",
  """SELECT j.key AS team, CAST((j.value - (SELECT MIN(_0__at__date) FROM nfl_season__slots)) / 7 AS INTEGER) + 1 AS week FROM nfl_season, json_each(nfl_season.byes) j
      WHERE CAST((j.value - (SELECT MIN(_0__at__date) FROM nfl_season__slots)) / 7 AS INTEGER) + 1 NOT BETWEEN 5 AND 14""",
  "The real NFL schedules byes in weeks 5-14 only.")

# ---------------------------------------------------------------- N4 rosters and depth
ROST = "ro AS (SELECT p.player__id AS id, p.name, p.player__position AS pos, p.player__age AS age, p.pro__team AS team, p.pro__status AS status, p.pro__draft AS draft FROM people p WHERE p.pro__status IS NOT NULL)"
for c_ in ["pro__team", "pro__status", "pro__draft", "pro__earnings", "pro__accrued", "pro__first_year", "pro__agent", "pro__teams", "pro__tags", "pro__trust", "pro__hold", "pro__wants_out", "mood", "mood_why",
           "pro__contract__kind", "pro__contract__team", "pro__contract__signed", "pro__contract__signing_bonus", "pro__ps_seasons", "pro__suspended", "pro__exempt"]:
    col("people", c_)
REAL53 = [("QB", 2.5), ("RB", 3.5), ("WR", 6.0), ("TE", 3.0), ("OT", 4.0), ("IOL", 4.5), ("EDGE", 4.0), ("DT", 4.0), ("LB", 5.0), ("CB", 5.5), ("S", 4.0), ("K", 1.0), ("P", 1.0), ("LS", 1.0)]
panel("N4.positions", NP, "N4", "Players per position on a 53", "bars",
  "Average players at each position on a club's active roster now. Real (from memory, typical 53s): QB 2-3, RB 3-4, WR 5-7, TE 3, OL 8-9 (about 4 tackles and 4-5 interior), DL 6-9, LB 5-7, CB 5-6, S 4, one K, P and LS.",
  f"""WITH {ROST}, r(pos, real) AS (VALUES {", ".join(f"('{p}', {v})" for p, v in REAL53)})
     SELECT 'Per club' AS grp, ro.pos AS label, ROUND(1.0 * COUNT(*) / (SELECT COUNT(DISTINCT team) FROM ro WHERE status = 'Roster'), 2) AS world, r.real, COUNT(*) AS n
       FROM ro LEFT JOIN r ON r.pos = ro.pos WHERE ro.status = 'Roster' GROUP BY ro.pos ORDER BY world DESC""")
panel("N4.clubs", NP, "N4", "Each club's books now: roster, practice squad, reserve", "table",
  "Each club's players by status now, the roster's average age and the share of the roster that went undrafted.",
  f"""WITH {ROST} SELECT {club_now('team')} AS club, team AS id, SUM(status = 'Roster') AS roster, SUM(status = 'PracticeSquad') AS "practice squad", SUM(status = 'Reserve') AS reserve,
            ROUND(AVG(CASE WHEN status = 'Roster' THEN age END), 1) AS "average age", ROUND(100.0 * SUM(status = 'Roster' AND draft IS NULL) / SUM(status = 'Roster'), 1) AS "undrafted %"
       FROM ro WHERE team IS NOT NULL AND status <> 'FreeAgent' GROUP BY team ORDER BY team""", open="e/nfl/{id}")
panel("N4.depth", NP, "N4", "Depth charts: the top two at each spot", "table",
  "Each club's depth chart now, the top two at every spot with rating, age and role (ratings are true values; the game shows only staff reads).",
  f"""SELECT {club_now('d._parent')} AS club, d._parent AS team, d._i AS spot, d._j + 1 AS depth, {pname('d.id')} AS name, d.id, ROUND(d.rating, 1) AS rating, d.age, d.role
       FROM nfl__depth d WHERE d._j < 2 ORDER BY d._parent, d._i, d._j""", open="e/people/{id}")
check("N4.undrafted", NP, "N4", "Undrafted players on 53-man rosters", "%",
  f"WITH {ROST} SELECT 100.0 * SUM(draft IS NULL) / COUNT(*) FROM ro WHERE status = 'Roster'",
  Memory(30.0, "about 30% of 53-man rosters went undrafted (from memory)"), (0.2, 0.5, 0.0), "Share of active roster players with no draft record. Spin-up veterans generated before the first season count as undrafted.")
check("N4.average_age", NP, "N4", "Average age on NFL rosters", "years", f"WITH {ROST} SELECT AVG(age) FROM ro WHERE status = 'Roster'",
  Memory(26.2, "real 53-man rosters average about 26 (from memory)"), (0.03, 0.08, 0.0), "Mean age of active roster players now.")
rule("N4.one_kicker", "N4", "In season every club carries exactly one kicker, punter and long snapper",
  "SELECT COUNT(DISTINCT pro__team) FROM people WHERE pro__status = 'Roster' AND (SELECT today FROM world) BETWEEN (SELECT cutdown__date FROM dates) AND (SELECT super_bowl FROM dates)",
  f"""WITH {ROST} SELECT team, {club_now('team')} AS club, SUM(pos = 'K') AS k, SUM(pos = 'P') AS p, SUM(pos = 'LS') AS ls FROM ro WHERE status = 'Roster' AND (SELECT today FROM world) BETWEEN (SELECT cutdown__date FROM dates) AND (SELECT super_bowl FROM dates) GROUP BY team
      HAVING SUM(pos = 'K') <> 1 OR SUM(pos = 'P') <> 1 OR SUM(pos = 'LS') <> 1""",
  "Real clubs keep one specialist each from the cutdown to the Super Bowl (a second kicker only in a rare injury week); offseason 90-man rosters carry two, so the rule reads only in season.")

# ---------------------------------------------------------------- N5 one pro career
PROC = """pc AS (SELECT p.player__id AS id, p.name, p.player__position AS pos, 1 AS active, p.pro__draft AS draft, p.pro__first_year AS first, p.pro__earnings AS earnings, p.pro__accrued AS accrued,
                 json_array_length(p.pro__teams) AS clubs, (SELECT COUNT(*) FROM people__pro__seasons s WHERE s._parent = p._row AND s.games > 0) AS seasons,
                 (SELECT MAX(year) FROM people__pro__seasons s WHERE s._parent = p._row) AS last, p.born FROM people p WHERE p.pro__status IS NOT NULL
          UNION ALL
          SELECT a.player__id, a.name, a.position, 0, a.pro__draft, a.pro__first_year, a.pro__earnings, a.pro__accrued, json_array_length(a.pro__teams),
                 (SELECT COUNT(*) FROM alumni__pro__seasons s WHERE s._parent = a._row AND s.games > 0), (SELECT MAX(year) FROM alumni__pro__seasons s WHERE s._parent = a._row), a.born
            FROM alumni a WHERE a.pro__status IS NOT NULL OR a.pro__first_year IS NOT NULL)"""
for c_ in ["pro__draft", "pro__first_year", "pro__earnings", "pro__accrued", "pro__teams", "pro__status", "born", "position", "year", "pick", "colleges", "stars", "home_state", "nfl_team"]:
    col("alumni", c_)
panel("N5.careers", NP, "N5", "Pro careers: draft slot, seasons, clubs and money", "table",
  "Every pro career on record, active and retired: draft year, round and pick (or undrafted), first season, seasons played, clubs, accrued seasons and career earnings. Open one for the whole career page.",
  f"""WITH {PROC} SELECT name, id, pos, CASE WHEN active THEN 'active' ELSE 'retired' END AS status, json_extract(draft, '$[0]') AS "draft year", json_extract(draft, '$[1]') AS round,
            json_extract(draft, '$[2]') AS pick, first AS "first season", seasons, clubs, accrued, ROUND(earnings / 1e6, 2) AS "earnings ($M)"
       FROM pc ORDER BY earnings DESC""", open="e/people/{id}")
panel("N5.length", NP, "N5", "Career length by draft round", "bars",
  "Seasons played by drafted players whose careers have ended, by the round they went in. Real (from memory): round 1 about 9 seasons, round 7 about 2-3.",
  f"""WITH {PROC} SELECT 'Seasons' AS grp, 'Round ' || json_extract(draft, '$[1]') AS label, ROUND(AVG(seasons), 2) AS world,
            CASE json_extract(draft, '$[1]') WHEN 1 THEN 9.0 WHEN 7 THEN 2.5 END AS real, COUNT(*) AS n
       FROM pc WHERE NOT active AND draft IS NOT NULL GROUP BY json_extract(draft, '$[1]') ORDER BY json_extract(draft, '$[1]')""")
check("N5.round1_length", NP, "N5", "Seasons played by retired first-round picks", "seasons",
  f"WITH {PROC} SELECT AVG(seasons) FROM pc WHERE NOT active AND json_extract(draft, '$[1]') = 1",
  Memory(9.0, "first-round picks play about 9 seasons (from memory)"), (0.2, 0.45, 0.0), "Mean seasons with a game for first-round picks whose careers have ended.")

# ---------------------------------------------------------------- N6 pro stat lines and the career book
PS = """ps AS (SELECT year, stats__att AS att, stats__cmp AS cmp, stats__pass_yds AS yds, stats__int AS ints, stats__sacked AS sacked, stats__dropbacks AS db, stats__fg_att AS fga, stats__fg_made AS fgm
            FROM people__pro__seasons UNION ALL SELECT year, stats__att, stats__cmp, stats__pass_yds, stats__int, stats__sacked, stats__dropbacks, stats__fg_att, stats__fg_made FROM alumni__pro__seasons)"""
for c_ in ["stats__int", "stats__sacked", "stats__fg_att", "stats__fg_made"]:
    col("people__pro__seasons", c_); col("alumni__pro__seasons", c_)
panel("N6.league", NP, "N6", "League passing year by year from the pros' records", "line",
  "League-wide completion %, yards an attempt, interception % and sack % from every pro season on record. Real NFL 2021-2025 (nflverse): 64.5%, 7.05, 2.3%, 6.8%. Older seasons keep only the players still on record, so the early years are thin.",
  f"""WITH {PS} SELECT 'Completion %' AS grp, year, ROUND(100.0 * SUM(cmp) / SUM(att), 1) AS world, ROUND(100 * json_extract({NGT}, '$.rates.completion_pct'), 1) AS real, SUM(att) AS n FROM ps GROUP BY year
     UNION ALL SELECT 'Yards an attempt', year, ROUND(1.0 * SUM(yds) / SUM(att), 2), json_extract({NGT}, '$.rates.yards_per_attempt'), SUM(att) FROM ps GROUP BY year
     UNION ALL SELECT 'Interception %', year, ROUND(100.0 * SUM(ints) / SUM(att), 2), ROUND(100 * json_extract({NGT}, '$.rates.int_rate'), 2), SUM(att) FROM ps GROUP BY year
     UNION ALL SELECT 'Sack %', year, ROUND(100.0 * SUM(sacked) / SUM(db), 2), ROUND(100 * json_extract({NGT}, '$.rates.sack_rate'), 2), SUM(db) FROM ps GROUP BY year ORDER BY 1, 2""")
CAREER = [("pass_yds", "Passing yards", 89214), ("pass_td", "Passing TDs", 649), ("rush_yds", "Rushing yards", 18355), ("rec_yds", "Receiving yards", 22895), ("sacks", "Sacks", 200), ("ints", "Interceptions", 81)]
panel("N6.career", NP, "N6", "Career leaders in the NFL career book", "table",
  "The top ten careers in each stat from the world's NFL career book (every game a pro played in the world). Real all-time marks, from memory: passing 89,214 (Brady), passing TDs 649, rushing 18,355 (Emmitt Smith), receiving 22,895 (Rice), sacks 200 (Bruce Smith; official since 1982), interceptions 81 (Krause).",
  "WITH r AS (" + " UNION ALL ".join(f"SELECT '{lab}' AS stat, {i} AS o, _key AS id, {k} AS value, {v} AS real, ROW_NUMBER() OVER (ORDER BY {k} DESC) AS rank FROM nfl_book__careers" for i, (k, lab, v) in enumerate(CAREER)) +
  f") SELECT stat, rank, {pname('r.id')} AS name, id, ROUND(value) AS value, CASE WHEN rank = 1 THEN real END AS \"real record\" FROM r WHERE rank <= 10 ORDER BY o, rank",
  open="e/people/{id}")
for k, _l, _v in CAREER:
    col("nfl_book__careers", k)
check("N6.league_completion", NP, "N6", "League completion %, the latest season", "%",
  f"WITH {PS} SELECT 100.0 * SUM(cmp) / SUM(att) FROM ps WHERE year = (SELECT MAX(year) FROM ps)",
  Key("nfl_game_targets.json:rates.completion_pct", 100.0), (0.04, 0.1, 0.0), "Completions over attempts across every pro season of the latest year.")
check("N6.league_ypa", NP, "N6", "League yards an attempt, the latest season", "yards",
  f"WITH {PS} SELECT 1.0 * SUM(yds) / SUM(att) FROM ps WHERE year = (SELECT MAX(year) FROM ps)",
  Key("nfl_game_targets.json:rates.yards_per_attempt"), (0.05, 0.12, 0.0), "Passing yards over attempts across every pro season of the latest year (gross of sacks, as players are credited).")

# ---------------------------------------------------------------- N7 contracts
CY = """cy AS (SELECT p.player__id AS id, p.name, p.player__position AS pos, p.pro__contract__team AS team, p.pro__contract__kind AS kind, p.pro__contract__signed AS signed,
                 COUNT(y._row) AS years, SUM(y.base) + COALESCE(p.pro__contract__signing_bonus, 0) AS total, SUM(COALESCE(y.guaranteed, 0)) + COALESCE(p.pro__contract__signing_bonus, 0) AS gtd,
                 (SUM(y.base) + COALESCE(p.pro__contract__signing_bonus, 0)) / MAX(COUNT(y._row), 1) AS apy, SUM(y.void) AS voids
            FROM people p JOIN people__pro__contract__years y ON y._parent = p._row WHERE p.pro__contract__kind IS NOT NULL AND NOT COALESCE(y.void, 0) GROUP BY p._row)"""
for c_ in ["base", "guaranteed", "void", "year"]:
    col("people__pro__contract__years", c_)
CAPPOS = [("QB", "QB"), ("RB", "RB"), ("WR", "WR"), ("TE", "TE"), ("OT", "LT"), ("IOL", "LG"), ("EDGE", "ED"), ("DT", "IDL"), ("LB", "LB"), ("CB", "CB"), ("S", "S"), ("K", "K"), ("P", "P"), ("LS", "LS")]
CAPMAP = "cm(pos, rk) AS (VALUES " + ", ".join(f"('{a}', '{b}')" for a, b in CAPPOS) + ")"
CAP = "(SELECT salary_cap FROM rules)"
panel("N7.top", NP, "N7", "The top deal at each position as a share of the cap", "bars",
  "The largest average per year at each position now, as a share of this year's cap, against the real top deal (engine_targets.json contracts_cap_pct, OverTheCap 2025; tackles read against the left tackle line, interior linemen the left guard's).",
  f"""WITH {CY}, {CAPMAP}, t AS (SELECT pos, MAX(apy) AS top, COUNT(*) AS n FROM cy GROUP BY pos)
     SELECT 'Top deal, % of cap' AS grp, t.pos AS label, ROUND(100.0 * t.top / {CAP}, 2) AS world,
            ROUND(100 * json_extract((SELECT value_json FROM real_figures WHERE key = 'engine_targets.json:contracts_cap_pct.' || cm.rk), '$.top1'), 2) AS real, t.n
       FROM t LEFT JOIN cm ON cm.pos = t.pos ORDER BY world DESC""")
panel("N7.deals", NP, "N7", "Every deal on the books", "table",
  "Each current contract: club, kind, signing year, years, total value, guarantees and the average per year, largest first.",
  f"""WITH {CY} SELECT name, id, pos, {club_now('team')} AS club, kind, signed, years, ROUND(total / 1e6, 2) AS "total ($M)", ROUND(gtd / 1e6, 2) AS "guaranteed ($M)",
            ROUND(apy / 1e6, 2) AS "per year ($M)", ROUND(100.0 * apy / {CAP}, 2) AS "% of cap" FROM cy ORDER BY apy DESC""", open="e/people/{id}")
panel("N7.kinds", NP, "N7", "Contracts by kind", "bars", "Current deals by kind: rookie, undrafted, veteran, minimum, extension, franchise tag, tender, futures, practice squad.",
  f"WITH {CY} SELECT 'Deals' AS grp, kind AS label, COUNT(*) AS world, NULL AS real, COUNT(*) AS n FROM cy GROUP BY kind ORDER BY world DESC")
for pos, rk in [("QB", "QB"), ("EDGE", "ED"), ("WR", "WR")]:
    check(f"N7.top_{pos.lower()}", NP, "N7", f"Top {pos} deal as a share of the cap", "%",
      f"WITH {CY} SELECT 100.0 * MAX(apy) / {CAP} FROM cy WHERE pos = '{pos}'", Key(f"engine_targets.json:contracts_cap_pct.{rk}#top1", 100.0), (0.15, 0.35, 0.0),
      f"The largest {pos} average per year now over this year's cap. Real: OverTheCap 2025.")

# ---------------------------------------------------------------- N8 cap and books
panel("N8.dead", NP, "N8", "Dead money by cause and year", "table",
  "Every dead-money charge summed by year and cause (cap cuts, releases, trades, retirements, voided years, practice squad and so on), with its share of the league's caps.",
  f"""SELECT d.year, d.why AS cause, COUNT(*) AS charges, ROUND(SUM(d.amount) / 1e6, 1) AS "$M",
            ROUND(100.0 * SUM(d.amount) / (32 * COALESCE((SELECT salary_cap FROM engine_rules r WHERE r.season = d.year), {CAP})), 2) AS "% of the caps"
       FROM clubs__dead d GROUP BY d.year, d.why ORDER BY d.year DESC, SUM(d.amount) DESC""")
panel("N8.opening_day", NP, "N8", "Opening-day cap use, dead money and carryover across clubs", "table",
  "The market's yearly read of the 32 clubs on opening day: the lowest, middle and highest club in cap use, dead money and carryover (shares of the cap), and whether the cash floor was paid. Real (from memory): median cap use about 95-100% with carryover, dead money median 5-10%, a purging club 20-30%.",
  """SELECT year, ROUND(100 * json_extract(cap_use, '$[0]'), 1) AS "cap use low %", ROUND(100 * json_extract(cap_use, '$[1]'), 1) AS "cap use median %", ROUND(100 * json_extract(cap_use, '$[2]'), 1) AS "cap use high %",
            ROUND(100 * json_extract(dead, '$[1]'), 1) AS "dead median %", ROUND(100 * json_extract(dead, '$[2]'), 1) AS "dead high %",
            ROUND(100 * json_extract(carryover, '$[1]'), 1) AS "carryover median %", ROUND(100 * json_extract(carryover, '$[2]'), 1) AS "carryover high %", floor_paid AS "cash floor paid"
       FROM market__years ORDER BY year""")
for c_ in ["cap_use", "dead", "carryover", "floor_paid"]:
    col("market__years", c_)
check("N8.median_cap_use", NP, "N8", "Median club's opening-day cap use", "%", "SELECT 100 * AVG(json_extract(cap_use, '$[1]')) FROM market__years WHERE json_extract(cap_use, '$[1]') > 0",
  Memory(97.0, "median club uses about 95-100% of the cap on opening day (from memory)"), (0.05, 0.12, 0.0), "The market's opening-day median cap use, averaged over years.")
check("N8.median_dead", NP, "N8", "Median club's dead money", "% of cap", "SELECT 100 * AVG(json_extract(dead, '$[1]')) FROM market__years WHERE json_extract(dead, '$[1]') IS NOT NULL",
  Memory(7.5, "median club carries 5-10% of the cap in dead money (from memory)"), (0.35, 0.8, 0.0), "The market's opening-day median dead money, averaged over years.")

# ---------------------------------------------------------------- N9 the market's year
MY = ["ufa", "resigned", "signed_new", "perfect", "swayed", "tags", "transition", "tenders", "sheets", "sheets_matched", "options_taken", "options_declined", "extensions", "rookie_extensions",
      "restructures", "cap_cuts", "june1", "futures", "waiver_claims", "poached", "signed_up", "settlements", "trouble", "suspensions", "draft_trades", "udfa_signed", "holdouts", "holdins",
      "bluffs", "bluffs_called", "agents_fired", "promises_broken", "trades", "deadline_trades", "picks_traded", "comp_picks", "other_league", "ipp"]
for c_ in MY:
    col("market__years", c_)
panel("N9.years", NP, "N9", "The market's year: every counter", "table",
  "Each league year's market: unrestricted free agents, re-signed, signed elsewhere, deals at first offer, players swayed by a better offer, tags, tenders, offer sheets, options, extensions, restructures, cap cuts, June 1 releases, futures, claims and the rest. Before 1993 there was no free agency (1989-92 Plan B).",
  "SELECT year, " + ", ".join(f'{c} AS "{c.replace("_", " ")}"' for c in MY) + " FROM market__years ORDER BY year")
panel("N9.flow", NP, "N9", "Free agency year by year", "line",
  "Unrestricted free agents, re-signed with their club and signed with a new one each year. Real (from memory): about 550 UFAs a year, 40-50% re-signing.",
  """SELECT 'Free agents' AS grp, year, ufa AS world, 550 AS real, ufa AS n FROM market__years UNION ALL SELECT 'Re-signed', year, resigned, NULL, resigned FROM market__years
     UNION ALL SELECT 'Signed elsewhere', year, signed_new, NULL, signed_new FROM market__years ORDER BY 1, 2""")
LAST5 = "year > (SELECT MAX(year) - 5 FROM market__years)"
for c_, lab, v, src, tol in [("ufa", "Unrestricted free agents a year", 550.0, "about 550 a year (from memory)", (0.2, 0.5, 0.0)),
                             ("tags", "Franchise tags a year", 7.5, "6-9 a year recently (from memory)", (0.3, 0.7, 1.0)),
                             ("sheets", "Offer sheets a year", 1.0, "0-2 a year (from memory)", (0.8, 2.0, 1.0)),
                             ("trades + draft_trades", "Trades a year (players or picks)", 91.2, "nflverse trades 2021-2025 (nfl_history.json trades, pick-only trades included): 86-96 a year", (0.2, 0.5, 0.0)),
                             ("deadline_trades", "In-season trades a year", 17.6, "nflverse trades September-November 2021-2025: 15-21 a year", (0.3, 0.7, 2.0))]:
    key = "trades" if c_.startswith("trades") else c_
    check(f"N9.{key}", NP, "N12" if key in ("trades", "deadline_trades") else "N9", lab, "a year", f"SELECT AVG({c_}) FROM market__years WHERE {LAST5}",
          Fixed(v, src, "2021-2025") if key in ("trades", "deadline_trades") else Memory(v, src), tol, f"Mean of the market's yearly count over its last five league years, as the real figure is recent.")
check("N9.waiver_claims", NP, "N9", "Waiver claims a year", "a year", f"SELECT AVG(waiver_claims) FROM market__years WHERE {LAST5}",
  NoReal("the real figure we have (25-40 claims a year, from memory) counts only the claims after the final cutdown; the world counts every claim of the year"), (0.3, 0.7, 0.0),
  "Mean of the market's yearly waiver claims over its last five league years.")
check("N9.resigned_share", NP, "N9", "Free agents who re-sign with their club", "%", f"SELECT 100.0 * SUM(resigned) / SUM(resigned + signed_new) FROM market__years WHERE {LAST5}",
  Memory(45.0, "about 40-50% of UFAs re-sign (from memory)"), (0.2, 0.45, 0.0), "Re-signed over all who signed (re-signed plus signed elsewhere), the last five league years.")

# ---------------------------------------------------------------- N10 pay ladder
panel("N10.tags", NP, "N10", "Franchise and transition tag prices by position", "table",
  "Each year's tag amounts by position as shares of the cap (franchise, then transition), from the five-year top-five average since 2011.",
  """SELECT t._0 AS year, j.key AS spot, ROUND(100 * json_extract(j.value, '$[0]'), 2) AS "franchise % of cap", ROUND(100 * json_extract(j.value, '$[1]'), 2) AS "transition % of cap"
       FROM market__tag_shares t, json_each(t._1) j ORDER BY t._0 DESC, j.key""")
panel("N10.ladder", NP, "N10", "The pay ladder's reads by position", "table",
  "The February consensus read of veterans by position, best first (the first ten reads at each spot): the scale prices are read against. Rank 1-2 sits near the top-five share, the 16th about 36%.",
  "SELECT _row AS spot, " + ", ".join(f"ROUND(_{k}, 1) AS \"{k + 1}\"" for k in range(10)) + " FROM market__ladder ORDER BY _row")
for k in range(10):
    col("market__ladder", f"_{k}")

# ---------------------------------------------------------------- N11 tags, tenders, options
panel("N11.counts", NP, "N11", "Tags, tenders, sheets, options and extensions by year", "line",
  "Each year's count of franchise and transition tags, restricted tenders, offer sheets (and matched), fifth-year options taken and declined, extensions, rookie extensions and restructures.",
  " UNION ALL ".join(f"SELECT '{lab}' AS grp, year, {c} AS world, NULL AS real, {c} AS n FROM market__years" for c, lab in [("tags", "Franchise tags"), ("transition", "Transition tags"), ("tenders", "Tenders"), ("sheets", "Offer sheets"),
      ("options_taken", "Options taken"), ("options_declined", "Options declined"), ("extensions", "Extensions"), ("rookie_extensions", "Rookie extensions"), ("restructures", "Restructures")]) + " ORDER BY 1, 2")
panel("N11.tagged", NP, "N11", "Players tagged now, and how often", "table", "Every pro tagged by his current club at least once, with position and how many times.",
  f"SELECT p.name, p.player__id AS id, p.player__position AS pos, {club_now('p.pro__team')} AS club, p.pro__tags AS tags FROM people p WHERE COALESCE(p.pro__tags, 0) > 0 ORDER BY p.pro__tags DESC",
  open="e/people/{id}")
check("N11.option_rate", NP, "N11", "Fifth-year options taken", "%", "SELECT 100.0 * SUM(options_taken) / NULLIF(SUM(options_taken + options_declined), 0) FROM market__years",
  Memory(50.0, "about half of fifth-year options are taken (from memory)"), (0.25, 0.6, 0.0), "Options taken over options decided, all years.")

# ---------------------------------------------------------------- N12 trades
panel("N12.trades", NP, "N12", "Every trade", "table",
  "Each trade: date, buyer and seller (club names of the day), the player and the picks that moved, in the world's own words.",
  f"""SELECT {DT('t.date')} AS date, {club('t.buyer', f"CAST(strftime('%Y', {DT('t.date')}) AS INTEGER)")} AS buyer, {club('t.seller', f"CAST(strftime('%Y', {DT('t.date')}) AS INTEGER)")} AS seller,
            {pname('t.player')} AS player, t.player AS id, (SELECT group_concat(p._0 || ' round ' || p._1, ', ') FROM market__picks__trades__picks p WHERE p._parent = t._row) AS picks, t.text
       FROM market__picks__trades t ORDER BY t.date DESC""", open="e/people/{id}")
panel("N12.per_year", NP, "N12", "Trades each year against real", "line",
  "Player trades, in-season trades, draft-day trades and picks traded each year. Real: nflverse trades (nfl_history.json, from Pro Football Reference and news), 2002-2025.",
  f"""SELECT 'Trades' AS grp, year, trades AS world, json_extract({NH}, '$.trades.by_season."' || year || '".trades') AS real, trades AS n FROM market__years
     UNION ALL SELECT 'In-season trades', year, deadline_trades, json_extract({NH}, '$.trades.by_season."' || year || '".in_season_sep_to_nov'), deadline_trades FROM market__years
     UNION ALL SELECT 'Picks traded', year, picks_traded, json_extract({NH}, '$.trades.by_season."' || year || '".picks_moved'), picks_traded FROM market__years
     UNION ALL SELECT 'Draft-day trades', year, draft_trades, NULL, draft_trades FROM market__years ORDER BY 1, 2""")
panel("N12.funnel", NP, "N12", "Where trade calls die (the latest year)", "funnel",
  "The trade funnel: buyers who called, a target found, a seller willing, value met, talks done, a package agreed.",
  """WITH f AS (SELECT trade_funnel FROM market__years ORDER BY year DESC LIMIT 1)
     SELECT 'Calls' AS grp, CASE CAST(j.key AS INTEGER) WHEN 0 THEN 'Buyers called' WHEN 1 THEN 'Target found' WHEN 2 THEN 'Seller willing' WHEN 3 THEN 'Value met' WHEN 4 THEN 'Talks done' ELSE 'Package agreed' END AS label,
            j.value AS world, NULL AS real, j.value AS n FROM f, json_each(f.trade_funnel) j ORDER BY CAST(j.key AS INTEGER)""")
col("market__years", "trade_funnel")

# ---------------------------------------------------------------- N13 picks
panel("N13.comp", NP, "N13", "Compensatory picks by year, round and kind", "table",
  "Comp picks awarded each spring by round and kind (formula, supplemental seventh, diversity). Real (from memory): 32 a year plus a few diversity picks, at most four per club; recently about 6, 4, 6, 8 and 9 in rounds 3-7.",
  "SELECT year, kind, " + ", ".join(f"SUM(round = {r}) AS \"round {r}\"" for r in range(3, 8)) + ", COUNT(*) AS picks FROM market__picks__comp GROUP BY year, kind ORDER BY year DESC, kind")
panel("N13.clubs", NP, "N13", "Clubs that live on comp picks", "bars", "Comp picks per club, all years. Real: Baltimore, New England and the Rams have lived on them.",
  f"SELECT 'Comp picks' AS grp, {club_now('team')} AS label, COUNT(*) AS world, NULL AS real, COUNT(*) AS n FROM market__picks__comp GROUP BY team ORDER BY world DESC")
check("N13.comp_a_year", NP, "N13", "Compensatory picks a year", "a year", "SELECT 1.0 * COUNT(*) / COUNT(DISTINCT year) FROM market__picks__comp WHERE kind <> 'Diversity'",
  Memory(32.0, "32 a year by the CBA (formula plus supplemental)"), (0.1, 0.3, 0.0), "Formula and supplemental comp picks per year.")
rule("N13.four_per_club", "N13", "No club gets more than four comp picks in a year", "SELECT COUNT(DISTINCT year || '-' || team) FROM market__picks__comp",
  "SELECT year, team, COUNT(*) AS picks FROM market__picks__comp WHERE kind <> 'Diversity' GROUP BY year, team HAVING COUNT(*) > 4", "The CBA caps formula comp picks at four a club a year.")

# ---------------------------------------------------------------- N14 waivers, squads, IR
panel("N14.years", NP, "N14", "Waivers, squads, elevations, IR and futures by year", "table",
  "Each year's claims, practice squad poaching, squad players signed up, injury settlements and futures deals, with game-day elevations and IR weeks per club from the pro seasons. Real (from memory): 25-40 cutdown claims, 10-25 elevations a club (from 2020), 10-15 futures deals a club.",
  """SELECT m.year, m.waiver_claims AS claims, m.poached, m.signed_up AS "signed up", m.settlements, m.futures, ROUND(m.futures / 32.0, 1) AS "futures a club",
            (SELECT ROUND(SUM(elevations) / 32.0, 1) FROM people__pro__seasons s WHERE s.year = m.year) AS "elevations a club",
            (SELECT ROUND(SUM(ir_weeks) / 32.0, 1) FROM people__pro__seasons s WHERE s.year = m.year) AS "IR weeks a club",
            (SELECT ROUND(SUM(ps_weeks) / 32.0 / 18, 1) FROM people__pro__seasons s WHERE s.year = m.year) AS "squad size (average)"
       FROM market__years m ORDER BY m.year""")
for c_ in ["elevations", "ir_weeks", "ps_weeks"]:
    col("people__pro__seasons", c_)
check("N14.elevations", NP, "N14", "Game-day elevations a club a season", "a club", "SELECT SUM(elevations) / 32.0 FROM people__pro__seasons WHERE year = (SELECT MAX(year) FROM people__pro__seasons)",
  Memory(17.5, "about 10-25 a club a season since 2020 (from memory)"), (0.35, 0.8, 0.0), "Elevations across the league in the latest season over 32.")
check("N14.futures", NP, "N14", "Futures deals a club a year", "a club", "SELECT AVG(futures) / 32.0 FROM market__years WHERE year > (SELECT MIN(year) FROM market__years)",
  Memory(12.5, "10-15 futures deals a club a year (from memory)"), (0.3, 0.7, 0.0), "Futures deals per club per year.")

# ---------------------------------------------------------------- N15 holdouts, trust, mood
panel("N15.unhappy", NP, "N15", "Pros who are unhappy: holdouts, trade requests, low trust", "table",
  "Every active pro holding out or in, wanting out, or with low trust in his club, with his mood and its main reason in his own terms.",
  f"""SELECT p.name, p.player__id AS id, p.player__position AS pos, {club_now('p.pro__team')} AS club, ROUND(p.pro__trust, 1) AS trust, p.pro__hold AS hold, CASE WHEN p.pro__wants_out THEN 'yes' END AS "wants out",
            ROUND(p.mood, 1) AS mood, p.mood_why AS why FROM people p WHERE p.pro__status IS NOT NULL AND (p.pro__hold IS NOT NULL OR p.pro__wants_out OR p.pro__trust < 30) ORDER BY p.pro__trust""", open="e/people/{id}")
panel("N15.clubs", NP, "N15", "Team mood by club, with last season's record", "table",
  "Each club's team mood and captains now, next to its last season's record. Winning clubs should be happier.",
  f"""SELECT {club_now('c._row')} AS club, c._row AS id, ROUND(c.team_mood, 1) AS "team mood", (SELECT wins || '-' || losses FROM nfl_seasons s WHERE s.team = c._row ORDER BY year DESC LIMIT 1) AS "last record",
            (SELECT group_concat({pname('j.value')}, ', ') FROM json_each(c.captains) j) AS captains FROM clubs c ORDER BY c.team_mood DESC""", open="e/nfl/{id}")
check("N15.holdouts", NP, "N15", "Holdouts a year", "a year", "SELECT AVG(holdouts) FROM market__years WHERE year > (SELECT MIN(year) FROM market__years)",
  Memory(2.0, "1-3 notable holdouts a year (from memory)"), (0.6, 1.5, 1.0), "Holdouts the market counts per year.")
check("N15.mood_wins", NP, "N15", "Team mood against last season's wins", "correlation",
  """WITH x AS (SELECT c.team_mood AS m, (SELECT wins FROM nfl_seasons s WHERE s.team = c._row ORDER BY year DESC LIMIT 1) AS w FROM clubs c)
     SELECT (AVG(m * w) - AVG(m) * AVG(w)) / sqrt((AVG(m * m) - AVG(m) * AVG(m)) * (AVG(w * w) - AVG(w) * AVG(w))) FROM x""",
  NoReal("winning clubs should be happier: a clearly positive correlation"), (0.5, 1.0, 0.0), "Correlation across clubs of team mood and last season's wins.")
col("clubs", "team_mood"); col("clubs", "captains")

# ---------------------------------------------------------------- N16 retirement
RET = """ret AS (SELECT a.player__id AS id, a.name, a.position AS pos, (SELECT MAX(year) FROM alumni__pro__seasons s WHERE s._parent = a._row) AS last,
                  CAST(strftime('%Y', date(a.born * 86400, 'unixepoch')) AS INTEGER) AS born_year FROM alumni a WHERE a.pro__first_year IS NOT NULL)"""
RAGE = [("QB", 35.0), ("RB", 28.0), ("WR", 30.0), ("K", 37.0), ("P", 37.0), ("LS", 36.0)]
panel("N16.age", NP, "N16", "Retirement age by position", "bars",
  "Age in the last season of pros whose careers are over, by position. Real (from memory): QB about 35, RB about 28, specialists about 37.",
  f"""WITH {RET}, r(pos, real) AS (VALUES {", ".join(f"('{p}', {v})" for p, v in RAGE)})
     SELECT 'Age at the end' AS grp, ret.pos AS label, ROUND(AVG(last - born_year), 1) AS world, r.real, COUNT(*) AS n FROM ret LEFT JOIN r ON r.pos = ret.pos WHERE last IS NOT NULL GROUP BY ret.pos ORDER BY world""")
panel("N16.farewells", NP, "N16", "Farewell seasons, celebrations and one-day contracts", "table",
  "Stars who announced a farewell season, retirement celebrations and one-day contracts to retire with a club. Real: about 1-2 one-day contracts a year (one_day_contracts.csv).",
  f"""SELECT f.year, 'farewell season' AS what, {pname('f.id')} AS name, f.id, {club('f.team', 'f.year')} AS club FROM stories__careers__farewells f
     UNION ALL SELECT o._1, 'one-day contract', {pname('o._0')}, o._0, {club('o._2', 'o._1')} FROM stories__careers__one_day o
     UNION ALL SELECT c._2, 'celebration: ' || c._1, {pname('c._0')}, c._0, NULL FROM stories__careers__celebrations c ORDER BY 1 DESC""", open="e/people/{id}")
for t_, cs in [("stories__careers__farewells", ["id", "year", "team"]), ("stories__careers__one_day", ["_0", "_1", "_2"]), ("stories__careers__celebrations", ["_0", "_1", "_2"])]:
    for c_ in cs: col(t_, c_)
check("N16.qb_age", NP, "N16", "QB age at retirement", "years", f"WITH {RET} SELECT AVG(last - born_year) FROM ret WHERE pos = 'QB' AND last IS NOT NULL",
  Memory(35.0, "QBs retire about 35 (from memory)"), (0.06, 0.15, 0.0), "Mean age in the last season of retired quarterbacks.")
check("N16.rb_age", NP, "N16", "RB age at retirement", "years", f"WITH {RET} SELECT AVG(last - born_year) FROM ret WHERE pos = 'RB' AND last IS NOT NULL",
  Memory(28.0, "running backs retire about 28 (from memory)"), (0.06, 0.15, 0.0), "Mean age in the last season of retired running backs.")
check("N16.one_day", NP, "N16", "One-day contracts a year", "a year", "SELECT CASE WHEN (SELECT COUNT(*) FROM stories__careers__farewells) + (SELECT COUNT(*) FROM stories__careers__one_day) > 0 THEN 1.0 * COUNT(*) / MAX(COUNT(DISTINCT _1), 1) END FROM stories__careers__one_day",
  Memory(1.5, "about 1-2 a year (one_day_contracts.csv, from memory)"), (0.5, 1.0, 1.0), "One-day contracts per year that had any.")

# ---------------------------------------------------------------- N17 front offices
GMC = ["skill", "film_lean", "sunk_cost", "impatience", "not_my_guy", "loyalty", "recency", "copycat", "overconfidence", "risk", "name_value", "classic_chart", "pays_backs", "restructures", "early_extensions"]
OWN = ["meddling", "cash_will", "patience", "pr", "draft_override", "trust"]
panel("N17.offices", NP, "N17", "Front offices: each club's GM and owner", "table",
  "Each club's general manager (since when and his habits: judgment, film lean, sunk cost, impatience, not-my-guy, loyalty, recency, copycat, overconfidence, risk, name value, pick chart, paying his own picks, restructures, early extensions), the owner's traits, who holds power, the club's mode (contend or rebuild) and why, the fans, and GM changes. Real GM tenure median about 4-6 years.",
  f"""SELECT {club_now('c._row')} AS club, c._row AS id, c.office__gm__name AS gm, c.office__gm__since AS since, """ +
  ", ".join(f"ROUND(c.office__gm__{k}, 2) AS \"{k.replace('_', ' ')}\"" for k in GMC) + ", " +
  ", ".join(f"ROUND(c.office__owner__{k}, 2) AS \"owner {k.replace('_', ' ')}\"" for k in OWN) +
  """, c.office__power AS power, c.office__mode AS mode, c.office__why AS why, ROUND(c.office__fans, 1) AS fans, c.office__gm_changes AS "GM changes" FROM clubs c ORDER BY c._row""", open="e/nfl/{id}")
for k in GMC: col("clubs", f"office__gm__{k}")
for k in OWN: col("clubs", f"office__owner__{k}")
for k in ["office__gm__name", "office__gm__since", "office__power", "office__mode", "office__why", "office__fans", "office__gm_changes"]: col("clubs", k)
check("N17.gm_tenure", NP, "N17", "GM tenure so far", "years", "SELECT AVG((SELECT MAX(year) FROM nfl_seasons) - office__gm__since) FROM clubs",
  Memory(5.0, "real GM tenures run a median of about 4-6 years (from memory)"), (0.35, 0.8, 0.0), "Mean years in the job of the current GMs (a tenure still running reads short of its end).")
check("N17.fans_wins", NP, "N17", "Fan mood against last season's wins", "correlation",
  """WITH x AS (SELECT c.office__fans AS m, (SELECT wins FROM nfl_seasons s WHERE s.team = c._row ORDER BY year DESC LIMIT 1) AS w FROM clubs c)
     SELECT (AVG(m * w) - AVG(m) * AVG(w)) / sqrt((AVG(m * m) - AVG(m) * AVG(m)) * (AVG(w * w) - AVG(w) * AVG(w))) FROM x""",
  NoReal("fans follow wins over expectation: a clearly positive correlation with wins"), (0.5, 1.0, 0.0), "Correlation across clubs of fan mood and last season's wins.")

# ---------------------------------------------------------------- N18 agents
panel("N18.agents", NP, "N18", "Agents: agencies, skill, honesty and clients", "table",
  "Every agent: agency and its size (3 a giant, 2 mid-size, 1 a one-man shop), skill, honesty, clients and times fired. Open one for his clients and his standing with each club.",
  "SELECT _row AS id, name, agency, size, ROUND(skill, 1) AS skill, ROUND(honesty, 2) AS honesty, clients, fired FROM market__agents ORDER BY clients DESC", open="e/market__agents/{id}")
panel("N18.concentration", NP, "N18", "Clients by agency size, and first-round picks", "bars",
  "Share of all clients and of first-round picks now in the league held by each agency size. Real: a few giant agencies hold most first-rounders.",
  """SELECT 'All clients' AS grp, CASE a.size WHEN 3 THEN 'Giant' WHEN 2 THEN 'Mid-size' ELSE 'One-man shop' END AS label, ROUND(100.0 * SUM(a.clients) / (SELECT SUM(clients) FROM market__agents), 1) AS world, NULL AS real, SUM(a.clients) AS n
       FROM market__agents a GROUP BY a.size
     UNION ALL
     SELECT 'First-round picks', CASE a.size WHEN 3 THEN 'Giant' WHEN 2 THEN 'Mid-size' ELSE 'One-man shop' END, ROUND(100.0 * COUNT(*) / (SELECT COUNT(*) FROM people WHERE json_extract(pro__draft, '$[1]') = 1 AND pro__agent IS NOT NULL), 1), NULL, COUNT(*)
       FROM people p JOIN market__agents a ON a._row = p.pro__agent WHERE json_extract(p.pro__draft, '$[1]') = 1 GROUP BY a.size ORDER BY 1, 2""")
check("N18.giants_round1", NP, "N18", "First-round picks with a giant agency", "%",
  "SELECT 100.0 * SUM(a.size = 3) / COUNT(*) FROM people p JOIN market__agents a ON a._row = p.pro__agent WHERE json_extract(p.pro__draft, '$[1]') = 1",
  Memory(70.0, "CAA, Athletes First, Klutch and a few others hold most first-rounders (from memory)"), (0.2, 0.45, 0.0), "Share of active first-round picks represented by a giant agency.")

# ---------------------------------------------------------------- N19 honors (the NFL page's view)
panel("N19.awards_pos", NP, "N19", "Award winners by position", "table",
  "Each award's winners by position, all years on record. Real (from memory): MVP about 80% QBs since 1984; DPOY mostly edge rushers and interior linemen; rookies of the year mostly first-rounders.",
  """WITH a AS (SELECT a._1 AS award, COALESCE((SELECT player__position FROM people WHERE player__id = a._2), (SELECT player__position FROM alumni WHERE player__id = a._2), '?') AS pos FROM nfl_book__awards a)
     SELECT award, pos, COUNT(*) AS winners, ROUND(100.0 * COUNT(*) / SUM(COUNT(*)) OVER (PARTITION BY award), 1) AS "share %" FROM a GROUP BY award, pos ORDER BY award, winners DESC""")
panel("N19.pro_bowls_wins", NP, "N19", "Pro Bowl picks by club against wins", "table",
  "Pro Bowl selections per club and season next to the club's wins: good teams should send more.",
  f"""SELECT b._0 AS year, {club('b._2', 'b._0')} AS club, COUNT(*) AS "Pro Bowls", (SELECT wins FROM nfl_seasons s WHERE s.year = b._0 AND s.team = b._2) AS wins
       FROM nfl_book__pro_bowls b GROUP BY b._0, b._2 ORDER BY b._0 DESC, COUNT(*) DESC""")
check("N19.dpoy_front", NP, "N19", "Defensive Players of the Year from the front seven", "%",
  """SELECT 100.0 * SUM(COALESCE((SELECT player__position FROM people WHERE player__id = a._2), (SELECT player__position FROM alumni WHERE player__id = a._2)) IN ('EDGE', 'DT', 'LB')) / COUNT(*)
       FROM nfl_book__awards a WHERE a._1 = 'Defensive Player of the Year'""",
  Memory(80.0, "most DPOYs are edge rushers, interior linemen and linebackers (from memory)"), (0.15, 0.35, 0.0), "Share of DPOY winners at EDGE, DT or LB.")
check("N19.pro_bowl_wins", NP, "N19", "Pro Bowl picks against club wins", "correlation",
  """WITH x AS (SELECT COUNT(*) AS p, (SELECT wins FROM nfl_seasons s WHERE s.year = b._0 AND s.team = b._2) AS w FROM nfl_book__pro_bowls b GROUP BY b._0, b._2)
     SELECT (AVG(p * w) - AVG(p) * AVG(w)) / sqrt((AVG(p * p) - AVG(p) * AVG(p)) * (AVG(w * w) - AVG(w) * AVG(w))) FROM x WHERE w IS NOT NULL""",
  NoReal("good teams send more players: a clearly positive correlation"), (0.5, 1.0, 0.0), "Correlation across club-seasons of Pro Bowl picks and wins.")

# ---------------------------------------------------------------- N20 the draft
DR = """dr AS (SELECT p.player__id AS id, p.name, p.player__position AS pos, json_extract(p.pro__draft, '$[0]') AS year, json_extract(p.pro__draft, '$[1]') AS round,
                 json_extract(p.pro__draft, '$[2]') AS pick, json_extract(p.pro__draft, '$[3]') AS team FROM people p WHERE p.pro__draft IS NOT NULL
          UNION ALL SELECT a.player__id, a.name, a.position, json_extract(a.pro__draft, '$[0]'), json_extract(a.pro__draft, '$[1]'), json_extract(a.pro__draft, '$[2]'), json_extract(a.pro__draft, '$[3]')
            FROM alumni a WHERE a.pro__draft IS NOT NULL)"""
panel("N20.positions", NP, "N20", "Who goes where: positions by pick band against real", "table",
  "Position shares of the first pick, picks 1-5, picks 1-10, round 1, round 2, round 3 and rounds 4-7, every draft on record. Real: nflverse drafts 2000-2025 (draft_positions_2000_2025.csv); QB 73% of first picks.",
  f"""WITH {DR}, b AS (SELECT dr.*, CASE WHEN pick = 1 AND round = 1 THEN 'pick_1' END AS b1, CASE WHEN round = 1 AND pick <= 5 THEN 'picks_1_5' END AS b5,
                          CASE WHEN round = 1 AND pick <= 10 THEN 'picks_1_10' END AS b10, CASE round WHEN 1 THEN 'round_1' WHEN 2 THEN 'round_2' WHEN 3 THEN 'round_3' ELSE 'rounds_4_7' END AS br FROM dr),
       x AS (SELECT b1 AS band, pos FROM b WHERE b1 IS NOT NULL UNION ALL SELECT b5, pos FROM b WHERE b5 IS NOT NULL UNION ALL SELECT b10, pos FROM b WHERE b10 IS NOT NULL UNION ALL SELECT br, pos FROM b)
     SELECT band, pos, COUNT(*) AS picks, ROUND(100.0 * COUNT(*) / SUM(COUNT(*)) OVER (PARTITION BY band), 1) AS "share %",
            (SELECT CASE pos WHEN 'QB' THEN pct_QB WHEN 'RB' THEN pct_RB WHEN 'WR' THEN pct_WR WHEN 'TE' THEN pct_TE WHEN 'OT' THEN pct_OT WHEN 'IOL' THEN pct_IOL WHEN 'EDGE' THEN pct_EDGE
                    WHEN 'DT' THEN pct_DT WHEN 'LB' THEN pct_LB WHEN 'CB' THEN pct_CB WHEN 'S' THEN pct_S WHEN 'K' THEN pct_K WHEN 'P' THEN pct_P END FROM real_draft_positions_2000_2025 r WHERE r.band = x.band) AS "real %"
       FROM x GROUP BY band, pos ORDER BY CASE band WHEN 'pick_1' THEN 1 WHEN 'picks_1_5' THEN 2 WHEN 'picks_1_10' THEN 3 WHEN 'round_1' THEN 4 WHEN 'round_2' THEN 5 WHEN 'round_3' THEN 6 ELSE 7 END, picks DESC""")
panel("N20.picks", NP, "N20", "Every draft pick", "table",
  "Each pick on record: year, round and pick, club (name of the day), player and position. Open a class for its whole board.",
  f"WITH {DR} SELECT year, round, pick, {club('dr.team', 'dr.year')} AS club, name, id, pos FROM dr ORDER BY year DESC, round, pick", open="draft/{year}")
check("N20.qb_first", NP, "N20", "First overall picks who were quarterbacks", "%",
  f"WITH {DR} SELECT 100.0 * SUM(pos = 'QB') / COUNT(*) FROM dr WHERE round = 1 AND pick = 1",
  Fixed(73.1, "draft_positions_2000_2025.csv (nflverse): 19 of 26 first picks", "2000-2025"), (0.2, 0.45, 0.0), "Share of first overall picks at quarterback.")

# ---------------------------------------------------------------- N21 pre-draft
panel("N21.years", NP, "N21", "The pre-draft season each year", "table",
  "Declarations and withdrawals, combine invites and all-star invites (and declines), by year, from the calendar's history. Real (from memory): 60-80 early entrants recently plus fourth-year juniors, a few withdrawals, about 320-340 combine invites.",
  "SELECT year, declared, withdrew, combine_invites AS \"combine invites\", all_star_invites AS \"all-star invites\", all_star_declined AS \"all-star declines\" FROM log_history ORDER BY year", needs=L)
for c_ in ["declared", "withdrew", "combine_invites", "all_star_invites", "all_star_declined"]:
    col("log_history", c_)
panel("N21.reads", NP, "N21", "December reads: where the pro scouts project prospects", "table",
  "Every prospect with a December read now: the grade, score and projected round, his pro position, declare plan, combine invite and all-star game. These are live and clear after the draft.",
  """SELECT p.name, p.player__id AS id, p.player__position AS pos, COALESCE(p.cal__pro_position, p.player__position) AS "pro position", p.cal__draft_read__grade AS grade,
            ROUND(p.cal__draft_read__score, 1) AS score, p.cal__draft_read__round AS round, p.cal__declare_plan AS "declare plan", CASE WHEN p.cal__combine_invite THEN 'yes' END AS combine, p.cal__all_star AS "all-star"
       FROM people p WHERE p.cal__draft_read__round IS NOT NULL ORDER BY p.cal__draft_read__score DESC""", open="e/people/{id}")
for c_ in ["cal__pro_position", "cal__draft_read__grade", "cal__draft_read__score", "cal__draft_read__round", "cal__declare_plan", "cal__combine_invite", "cal__all_star"]:
    col("people", c_)
check("N21.combine_invites", NP, "N21", "Combine invites a year", "a year", "SELECT AVG(combine_invites) FROM log_history WHERE combine_invites > 0",
  Memory(330.0, "about 320-340 combine invites a year (from memory)"), (0.08, 0.2, 0.0), "Combine invites per year.", needs=L)
check("N21.early_entrants", NP, "N21", "Underclassmen declaring a year", "a year", "SELECT AVG(declared) FROM log_history WHERE declared > 0",
  Memory(90.0, "about 50-130 underclassmen a year; recently 60-80 plus fourth-year juniors (from memory)"), (0.35, 0.8, 0.0), "Early entrants per year.", needs=L)

# ---------------------------------------------------------------- N22 club scouting
panel("N22.departments", NP, "N22", "Each club's scouting department", "table",
  "Scouts per club by role, with the department's average judging, potential eye, work and conviction (true values). Real (from memory): 8-12 area scouts and 2-4 national scouts a club.",
  f"""SELECT {club_now('s._parent')} AS club, s._parent AS id, COUNT(*) AS scouts, SUM(s.role = 'Area') AS area, SUM(s.role = 'National') AS national, SUM(s.role = 'Director') AS directors,
            ROUND(AVG(s.judge), 1) AS judging, ROUND(AVG(s.potential), 1) AS "potential eye", ROUND(AVG(s.work), 1) AS work, ROUND(AVG(s.conviction), 1) AS conviction, ROUND(AVG(s.pay) / 1000, 0) AS "pay ($K)"
       FROM clubs__scouts s GROUP BY s._parent ORDER BY s._parent""", open="e/nfl/{id}")
check("N22.area_scouts", NP, "N22", "Area scouts a club", "a club", "SELECT 1.0 * SUM(role = 'Area') / COUNT(DISTINCT _parent) FROM clubs__scouts",
  Memory(10.0, "8-12 area scouts a club (from memory)"), (0.25, 0.6, 0.0), "Area scouts per club now.")

# ---------------------------------------------------------------- N23 public desks
panel("N23.desks", NP, "N23", "Public draft desks and how they score", "table",
  "The analysts and services: outlet, kind, years active, lean and skill (true), and their record scoring mocks and boards.",
  "SELECT name, outlet, kind, \"from\", \"to\", lean, skill, record FROM scouting__desks ORDER BY skill DESC")
panel("N23.scored", NP, "N23", "Last year's mock scoring", "table", "Each desk's scored mock: exact hits and near hits in round one.",
  "SELECT (SELECT name FROM scouting__desks d WHERE d._row = s._0) AS desk, s._1 AS exact, s._2 AS near, s._3 AS picks FROM scouting__public__scored s ORDER BY s._1 DESC")
col("scouting__desks", "to")
check("N23.mock_hits", NP, "N23", "Exact first-round hits by the best mock", "picks", "SELECT MAX(_1) FROM scouting__public__scored",
  Memory(10.0, "good real mocks hit about 8-12 exact picks in round one (from memory)"), (0.3, 0.7, 2.0), "The best desk's exact hits in last year's first round.")

# ---------------------------------------------------------------- N24 undrafted
panel("N24.udfa", NP, "N24", "Undrafted signings and rookie deals", "line",
  "Undrafted free agents signed each year (real about 500-600 with tryouts) and current rookie and undrafted contracts.",
  "SELECT 'Undrafted signed' AS grp, year, udfa_signed AS world, 550 AS real, udfa_signed AS n FROM market__years ORDER BY year")
check("N24.udfa", NP, "N24", "Undrafted signings a year", "a year", "SELECT AVG(udfa_signed) FROM market__years WHERE udfa_signed > 0",
  Memory(550.0, "about 500-600 undrafted signings a year including tryouts (from memory)"), (0.2, 0.5, 0.0), "Undrafted signings per year.")
check("N24.rookie_scale", NP, "N24", "First pick's rookie deal per year", "$M",
  f"""WITH {CY} SELECT apy / 1e6 FROM cy JOIN people p ON p.player__id = cy.id WHERE json_extract(p.pro__draft, '$[1]') = 1 AND json_extract(p.pro__draft, '$[2]') = 1 AND cy.kind = 'Rookie'
      ORDER BY json_extract(p.pro__draft, '$[0]') DESC LIMIT 1""",
  Fixed(11.6, "rookie_scale_2026.csv (OverTheCap): pick 1 about $11.6M a year", "2026"), (0.15, 0.4, 0.0), "Average per year of the latest first overall pick's rookie deal.")

# ---------------------------------------------------------------- N25 supplemental draft
panel("N25.picks", NP, "N25", "Supplemental draft picks", "table", "Players taken in a supplemental draft (held through 2019): year, club, round; the winner forfeits that round next year.",
  f"SELECT year, name, player AS id, {club('team', 'year')} AS club, round FROM scouting__supplemental ORDER BY year DESC", open="e/people/{id}")
for c_ in ["year", "player", "name", "team", "round"]:
    col("scouting__supplemental", c_)
check("N25.rate", NP, "N25", "Supplemental picks a year", "a year", "SELECT 1.0 * COUNT(*) / MAX((SELECT COUNT(DISTINCT year) FROM nfl_seasons WHERE year <= 2019), 1) FROM scouting__supplemental",
  Memory(0.4, "about one player every 2-3 years (from memory)"), (0.6, 1.5, 0.5), "Supplemental picks per season through 2019.")

# ---------------------------------------------------------------- N26 other leagues
panel("N26.seasons", NP, "N26", "Other pro leagues: champions and finals", "table",
  "Each other league's seasons: champion, runner-up and the final, or why play stopped. Leagues exist only in their real years.",
  """SELECT s.year, l.name AS league, s.champion, s.runner_up AS "runner-up", s.final_score AS final, s.note,
            (SELECT COUNT(*) FROM others__seasons__games g WHERE g._parent = s._row) AS games FROM others__seasons s LEFT JOIN engine_leagues l ON l._row = s.league ORDER BY s.year DESC, l.name""")
panel("N26.points", NP, "N26", "Points a team game by league against real", "bars",
  "Points per team game in every logged other-league game, and from each league's final tables. Real (from memory): CFL about 24, arena about 52, spring leagues about 20-21, the 1980s USFL about 23.",
  """SELECT 'Game log' AS grp, league AS label, ROUND((AVG(home_pts) + AVG(away_pts)) / 2, 1) AS world,
            CASE league WHEN 'CFL' THEN 24 WHEN 'Arena Football League' THEN 52 WHEN 'USFL' THEN 23 WHEN 'UFL' THEN 20.5 WHEN 'XFL' THEN 20.5 WHEN 'AAF' THEN 20.5 END AS real, COUNT(*) AS n
       FROM log_games WHERE stream = 'other' GROUP BY league
     UNION ALL
     SELECT 'Final tables', l.name, ROUND(1.0 * SUM(t.pf) / SUM(t.w + t.l + t.t), 1),
            CASE l.name WHEN 'CFL' THEN 24 WHEN 'Arena Football League' THEN 52 WHEN 'USFL' THEN 23 WHEN 'UFL' THEN 20.5 WHEN 'XFL' THEN 20.5 WHEN 'AAF' THEN 20.5 END, COUNT(*)
       FROM others__seasons__table t JOIN others__seasons s ON s._row = t._parent JOIN engine_leagues l ON l._row = s.league GROUP BY l.name ORDER BY 1, 2""")
panel("N26.players", NP, "N26", "Pros' seasons in other leagues", "table", "Each pro's seasons abroad or in spring leagues: league, team, games, the main stats and pay.",
  """SELECT o.year, l.name AS league, p.name, p.player__id AS id, p.player__position AS pos, o.games, o.starts, o.pass_yds AS "pass yds", o.rush_yds AS "rush yds", o.rec_yds AS "rec yds",
            o.tackles, o.sacks, ROUND(o.pay / 1000) AS "pay ($K)" FROM people__pro__other o JOIN people p ON p._row = o._parent LEFT JOIN engine_leagues l ON l._row = o.league ORDER BY o.year DESC""", open="e/people/{id}")
check("N26.cfl_points", NP, "N26", "CFL points a team game", "points", "SELECT (AVG(home_pts) + AVG(away_pts)) / 2 FROM log_games WHERE stream = 'other' AND league = 'CFL'",
  Memory(24.0, "the CFL averages about 24 a team game (from memory)"), (0.12, 0.3, 0.0), "Points per team game in logged CFL games.", needs=G)
rule("N26.real_years", "N26", "Other leagues play only in their real years", "SELECT COUNT(*) FROM others__seasons",
  "SELECT s.year, l.name, l.\"from\", l.\"to\" FROM others__seasons s JOIN engine_leagues l ON l._row = s.league WHERE s.year NOT BETWEEN l.\"from\" AND l.\"to\"",
  "Each league runs only between its real first and last seasons.")

# ---------------------------------------------------------------- N27 off-field trouble
panel("N27.years", NP, "N27", "Discipline by year: fines, suspensions, ejections, cases", "table",
  "Each year's NFL fines and hit suspensions, college targeting ejections, high school ejections, and the market's off-field cases and suspensions. Real (from memory): 30-60 NFL players arrested a year (USA Today database), hundreds of fines.",
  """SELECT d._0 AS year, d._1 AS "NFL fines", d._2 AS "NFL hit suspensions", d._3 AS "college targeting ejections", d._4 AS "high school ejections",
            (SELECT trouble FROM market__years m WHERE m.year = d._0) AS "off-field cases", (SELECT suspensions FROM market__years m WHERE m.year = d._0) AS suspensions
       FROM folk__discipline__years d ORDER BY d._0""")
panel("N27.cases", NP, "N27", "Court cases and rulings", "table", "Every closed case with its outcome in plain words, and open ones with their next step.",
  f"""SELECT {DT('c.opened')} AS opened, c.level, c.what, {pname('c.id')} AS name, c.id, CASE WHEN c.guilty THEN 'guilty' ELSE 'not guilty or dropped' END AS ruling, c.outcome FROM folk__courts__closed c
     UNION ALL SELECT {DT('o.opened')}, o.level, o.what, {pname('o.id')}, o.id, 'open: ' || COALESCE(o.step, ''), NULL FROM folk__courts__open o ORDER BY 1 DESC""", open="e/people/{id}")
for c_ in ["id", "level", "what", "opened", "step", "guilty", "outcome"]:
    col("folk__courts__closed", c_)
for c_ in ["id", "level", "what", "opened", "step"]:
    col("folk__courts__open", c_)
check("N27.cases_a_year", NP, "N27", "NFL off-field cases a year", "a year", "SELECT AVG(trouble) FROM market__years WHERE year > (SELECT MIN(year) FROM market__years)",
  Memory(45.0, "30-60 NFL players arrested a year (USA Today database, from memory)"), (0.35, 0.8, 0.0), "Off-field cases per year in the market's count.")
check("N27.fines", NP, "N27", "NFL fines a year", "a year", "SELECT AVG(_1) FROM folk__discipline__years",
  Memory(300.0, "hundreds of on-field fines a year (from memory)"), (0.4, 0.9, 0.0), "NFL fines per year.")

# ---------------------------------------------------------------- N28 life away from football
panel("N28.events", NP, "N28", "Life events by kind and year", "table",
  "Life away from football: births, marriages, an ill parent, a loss, divorce, money trouble, community work, homesickness, graduation, by year, with the yearly births, deaths and coaches' health episodes.",
  f"""SELECT CAST(strftime('%Y', {DT('e.date')}) AS INTEGER) AS year, e.event, COUNT(*) AS events FROM folk__life__events e GROUP BY 1, 2
     UNION ALL SELECT _0, 'births (all)', _1 FROM folk__life__years UNION ALL SELECT _0, 'deaths (all)', _2 FROM folk__life__years UNION ALL SELECT _0, 'coach health episodes', _3 FROM folk__life__years
     ORDER BY 1 DESC, 3 DESC""")
panel("N28.health", NP, "N28", "Coaches' health episodes", "table", "Each coach's health episode with when he is back and how serious, next to his age.",
  f"""SELECT {DT('h.date')} AS date, (SELECT name FROM coaches WHERE id = h.coach) AS coach, h.coach AS id, (SELECT age FROM coaches WHERE id = h.coach) AS age, h.what, {DT('h.back')} AS back,
            CASE WHEN h.serious THEN 'serious' END AS serious FROM folk__life__health h ORDER BY h.date DESC""", open="e/coaches/{id}")
check("N28.coach_health_age", NP, "N28", "Age of coaches with a health episode against all coaches", "years",
  "SELECT AVG((SELECT age FROM coaches WHERE id = h.coach)) - (SELECT AVG(age) FROM coaches) FROM folk__life__health h",
  NoReal("episodes should come more with age: coaches with one should be older than the average coach"), (0.5, 1.0, 2.0), "Mean age of coaches with a health episode minus the mean of all coaches.")

# ---------------------------------------------------------------- N29 ties and staff deals
panel("N29.ties", NP, "N29", "Ties between people: kinds and strength", "bars",
  "Friendships, mentors and family ties by kind, with their average strength, and how many are still together.",
  """SELECT 'Ties' AS grp, kind AS label, COUNT(*) AS world, NULL AS real, COUNT(*) AS n FROM folk__ties__list GROUP BY kind
     UNION ALL SELECT 'Average strength', kind, ROUND(AVG(strength), 1), NULL, COUNT(*) FROM folk__ties__list GROUP BY kind
     UNION ALL SELECT 'Still together %', kind, ROUND(100.0 * SUM(together) / COUNT(*), 1), NULL, COUNT(*) FROM folk__ties__list GROUP BY kind ORDER BY 1, 2""")
panel("N29.salaries", NP, "N29", "Coaches' pay by level and role", "table",
  "Staff deals by the coach's level and role now: count, median and top salary, and buyouts owed. Real (from memory): NFL coordinators $1-4M, FBS power coordinators $1-2.5M, high school stipends a few thousand.",
  """WITH d AS (SELECT CASE WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' WHEN c.job__employer__College IS NOT NULL THEN 'College' WHEN c.job__employer__HighSchool IS NOT NULL THEN 'High school' ELSE 'Other' END AS level,
                    COALESCE(c.job__role, 'Position coach') AS role, s.salary, s.buyout, ROW_NUMBER() OVER (PARTITION BY 1 ORDER BY s.salary) AS rn FROM folk__staff__deals s JOIN coaches c ON c.id = s.coach),
       r AS (SELECT d.*, ROW_NUMBER() OVER (PARTITION BY level, role ORDER BY salary) AS k, COUNT(*) OVER (PARTITION BY level, role) AS cnt FROM d)
     SELECT level, role, COUNT(*) AS deals, ROUND(MIN(CASE WHEN k >= cnt / 2.0 THEN salary END) / 1000, 0) AS "median ($K)", ROUND(MAX(salary) / 1000, 0) AS "top ($K)", ROUND(SUM(buyout) / 1e6, 2) AS "buyouts ($M)"
       FROM r GROUP BY level, role ORDER BY level, role""")
for c_ in ["job__employer__Nfl", "job__employer__College", "job__employer__HighSchool", "job__role"]:
    col("coaches", c_)
rule("N29.mentors", "N29", "Mentor ties come from veterans with leadership", "SELECT COUNT(*) FROM folk__ties__list WHERE kind = 'Mentor'",
  """SELECT t.a, t.b, t.strength FROM folk__ties__list t JOIN people p ON p.player__id = t.a WHERE t.kind = 'Mentor' AND p.player__personality__traits__leadership < 13""",
  "A mentor tie is drawn only from a veteran with leadership 13 or more (people design).")
