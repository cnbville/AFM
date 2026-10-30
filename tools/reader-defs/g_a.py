from common import *

GP = "games"
# ---------------------------------------------------------------- G1
panel("G1.points_by_year", GP, "G1", "Points a team game, year by year, by division", "line",
  "Every engine game between two programs of the same division, regular season. Real: CollegeFootballData through sportsdataverse (2014-2025 for FBS, FCS, D2 and D3; FBS back to 2001 from cfb_schedules). Stand-in games are left out: they have no box score.",
  f"""WITH {COLLEGE_TG}
     SELECT div AS grp, year, ROUND(AVG(pf), 2) AS world,
            COALESCE(json_extract({CS}, '$.divisions.' || {realdiv('div')} || '.by_season."' || year || '".points_per_team_game'),
                     CASE WHEN div = 'FBS' THEN json_extract({CH}, '$.scores_by_season.by_season."' || year || '".FBS.points_per_team_game') END) AS real,
            COUNT(*) AS n
       FROM tg WHERE div = odiv AND COALESCE(kind, 'regular') = 'regular' GROUP BY div, year ORDER BY {divord('div')}, year""",
  says=f"""WITH {COLLEGE_TG} SELECT printf('FBS teams score %.1f a game against each other (real %.1f, 2014-2025); %d engine team games across %d divisions.',
       (SELECT AVG(pf) FROM tg WHERE div = 'FBS' AND odiv = 'FBS'), json_extract({CS}, '$.divisions.fbs.all.points_per_team_game'),
       (SELECT COUNT(*) FROM tg), (SELECT COUNT(DISTINCT div) FROM tg))""", needs=G)

panel("G1.margin", GP, "G1", "Final margins: how close games are", "hist",
  "Share of games by final margin (0 to 40 points, then 41 and more), between two programs of the same division. Real: FBS against FBS in the regular season 2014-2025 (college_history.json fbs_margin_table). Key numbers 3, 7, 10 and 14 should stand out as in real football.",
  f"""WITH RECURSIVE bins(x) AS (SELECT 0 UNION ALL SELECT x + 1 FROM bins WHERE x < 41),
       m AS (SELECT json_extract(g.divisions, '$[0]') AS grp, MIN(ABS(g.home_pts - g.away_pts), 41) AS x FROM log_games g
              WHERE g.stream = 'college' AND {ENGINE} AND {REG} AND json_extract(g.divisions, '$[0]') = json_extract(g.divisions, '$[1]')),
       t AS (SELECT grp, COUNT(*) AS n FROM m GROUP BY grp),
       r AS (SELECT CAST(key AS INTEGER) AS x, value AS v FROM json_each({CH}, '$.fbs_margin_table.share_by_final_margin'))
     SELECT t.grp, CASE WHEN bins.x = 41 THEN '41+' ELSE CAST(bins.x AS TEXT) END AS x,
            100.0 * (SELECT COUNT(*) FROM m WHERE m.grp = t.grp AND m.x = bins.x) / t.n AS world,
            CASE WHEN t.grp = 'FBS' THEN 100.0 * (SELECT SUM(v) FROM r WHERE (bins.x < 41 AND r.x = bins.x) OR (bins.x = 41 AND r.x >= 41)) END AS real,
            (SELECT COUNT(*) FROM m WHERE m.grp = t.grp AND m.x = bins.x) AS n
       FROM t, bins ORDER BY {divord('t.grp')}, bins.x""", needs=G)

def shape_rows(div):
    rd = realdiv(f"'{div}'")
    return f"""SELECT '{div}' AS grp, 1 AS o, 'Decided by 8 or less' AS label, 100.0 * AVG(ABS(pf - pa) <= 8) AS world, 100.0 * json_extract({CS}, '$.divisions.' || {rd} || '.all.decided_by_8_or_less') AS real, COUNT(*) / 2 AS n FROM tg WHERE div = '{div}' AND odiv = '{div}'
       UNION ALL SELECT '{div}', 2, 'Decided by 28 or more', 100.0 * AVG(ABS(pf - pa) >= 28), 100.0 * json_extract({CS}, '$.divisions.' || {rd} || '.all.decided_by_28_plus'), COUNT(*) / 2 FROM tg WHERE div = '{div}' AND odiv = '{div}'
       UNION ALL SELECT '{div}', 3, 'Decided by 40 or more', 100.0 * AVG(ABS(pf - pa) >= 40), CASE WHEN '{div}' = 'FBS' THEN 6.7 END, COUNT(*) / 2 FROM tg WHERE div = '{div}' AND odiv = '{div}'
       UNION ALL SELECT '{div}', 4, 'Decided by 60 or more', 100.0 * AVG(ABS(pf - pa) >= 60), CASE WHEN '{div}' = 'FBS' THEN 0.4 END, COUNT(*) / 2 FROM tg WHERE div = '{div}' AND odiv = '{div}'
       UNION ALL SELECT '{div}', 5, 'Home team won (not neutral)', 100.0 * AVG(pf > pa), 100.0 * json_extract({CS}, '$.divisions.' || {rd} || '.all.home_win'), COUNT(*) FROM tg WHERE div = '{div}' AND odiv = '{div}' AND home = 1 AND neutral = 0
       UNION ALL SELECT '{div}', 6, 'Team games shut out', 100.0 * AVG(pf = 0), 100.0 * json_extract({CS}, '$.divisions.' || {rd} || '.all.shutout_share_of_team_games'), COUNT(*) FROM tg WHERE div = '{div}' AND odiv = '{div}'
       UNION ALL SELECT '{div}', 7, 'Team games with 50 or more', 100.0 * AVG(pf >= 50), NULL, COUNT(*) FROM tg WHERE div = '{div}' AND odiv = '{div}'
       UNION ALL SELECT '{div}', 8, 'Games to overtime', 100.0 * AVG(ot > 0), NULL, COUNT(*) / 2 FROM tg WHERE div = '{div}' AND odiv = '{div}'"""
panel("G1.shape", GP, "G1", "Score shape against real, by division", "bars",
  "Games between two programs of the same division, regular season and postseason. Real: college_scores.json (CollegeFootballData 2014-2025) for FBS, FCS, D2 and D3; 40+ and 60+ margins for FBS from the Debug Lab's 2011-2025 table. No real overtime share or 50-point share is on file; NAIA and JUCO have no real scores at all.",
  f"WITH {COLLEGE_TG} SELECT grp, label, world, real, n FROM (" + " UNION ALL ".join(shape_rows(d) for d in ["FBS", "FCS", "D2", "D3", "NAIA", "JUCO"]) + f") WHERE n > 0 ORDER BY {divord('grp')}, o", needs=G)

panel("G1.key_numbers", GP, "G1", "FBS key numbers: games ending by exactly", "bars",
  "Share of FBS against FBS games ending by each margin, against real (2014-2025). A field goal (3), a touchdown (7), both (10) and two touchdowns (14) are football's key numbers.",
  f"""WITH m AS (SELECT ABS(g.home_pts - g.away_pts) AS x FROM log_games g WHERE g.stream = 'college' AND {ENGINE} AND {REG} AND g.divisions = '["FBS","FBS"]'),
       k(x) AS (VALUES (1), (2), (3), (4), (6), (7), (8), (10), (14), (17), (21), (24), (28))
     SELECT 'By ' || k.x AS label, 100.0 * (SELECT COUNT(*) FROM m WHERE m.x = k.x) / MAX(1, (SELECT COUNT(*) FROM m)) AS world,
            100.0 * json_extract({CH}, '$.fbs_margin_table.share_by_final_margin."' || k.x || '"') AS real, (SELECT COUNT(*) FROM m WHERE m.x = k.x) AS n
       FROM k ORDER BY k.x""", needs=G)

panel("G1.cross", GP, "G1", "Games across divisions: how often the higher division wins", "bars",
  "Every engine game between programs of two divisions (buy games and the like), by pair, the share won by the higher division (FBS, FCS, D2, NAIA, D3, JUCO in that order). Real: FBS over FCS 2001-2025 (college_history.json, every season's FBS-FCS games). No real rate for the other pairs is on file.",
  f"""WITH x AS (SELECT CASE WHEN {divord("json_extract(g.divisions, '$[0]')")} <= {divord("json_extract(g.divisions, '$[1]')")}
                         THEN json_extract(g.divisions, '$[0]') || ' over ' || json_extract(g.divisions, '$[1]') ELSE json_extract(g.divisions, '$[1]') || ' over ' || json_extract(g.divisions, '$[0]') END AS label,
                    CASE WHEN {divord("json_extract(g.divisions, '$[0]')")} <= {divord("json_extract(g.divisions, '$[1]')")} THEN g.home_pts > g.away_pts ELSE g.away_pts > g.home_pts END AS won
               FROM log_games g WHERE g.stream = 'college' AND {ENGINE} AND json_extract(g.divisions, '$[0]') != json_extract(g.divisions, '$[1]')),
       r AS (SELECT SUM(json_extract(value, '$.FBS.fbs_vs_fcs_games') * json_extract(value, '$.FBS.fbs_vs_fcs_fbs_win_rate')) / SUM(json_extract(value, '$.FBS.fbs_vs_fcs_games')) AS v
               FROM json_each({CH}, '$.scores_by_season.by_season'))
     SELECT label, 100.0 * AVG(won) AS world, CASE WHEN label = 'FBS over FCS' THEN 100.0 * (SELECT v FROM r) END AS real, COUNT(*) AS n FROM x GROUP BY label ORDER BY n DESC""", needs=G)

panel("G1.team_seasons", GP, "G1", "Best and worst scoring team seasons, each year", "table",
  "Each season's ten highest and ten lowest points a game among FBS programs, from the programs' season totals (every season of the world, spin-up years included). Real: the best FBS offenses score about 45 a game and the worst about 13 (from memory; no real list of team seasons is on file).",
  f"""WITH s AS (SELECT s.year, s._parent AS id, {prog()} AS program, s.wins || '-' || s.losses AS record, s.totals__games AS games,
                    ROUND(1.0 * s.totals__points_for / s.totals__games, 1) AS points_for, ROUND(1.0 * s.totals__points_against / s.totals__games, 1) AS points_against
               FROM programs__seasons s JOIN programs p ON p._row = s._parent WHERE p.division = 'FBS' AND s.totals__games > 0),
       r AS (SELECT *, ROW_NUMBER() OVER (PARTITION BY year ORDER BY points_for DESC) AS hi, ROW_NUMBER() OVER (PARTITION BY year ORDER BY points_for) AS lo FROM s)
     SELECT year, CASE WHEN hi <= 10 THEN 'best' ELSE 'worst' END AS end_of_list, program, record, games, points_for, points_against, id FROM r WHERE hi <= 10 OR lo <= 10 ORDER BY year DESC, points_for DESC""",
  open="e/programs/{id}")

panel("G1.extremes", GP, "G1", "The most lopsided and highest-scoring college games", "table",
  "The 60 biggest margins and the 60 highest combined scores among engine college games, every division. Each opens its game page.",
  f"""WITH x AS (SELECT g._row AS id, g.season AS year, g.date, json_extract(g.divisions, '$[0]') || ' v ' || json_extract(g.divisions, '$[1]') AS divisions,
                    (SELECT {prog()} FROM programs p WHERE p._row = g.home) || ' ' || g.home_pts || ', ' || (SELECT {prog()} FROM programs p WHERE p._row = g.away) || ' ' || g.away_pts AS score,
                    ABS(g.home_pts - g.away_pts) AS margin, g.home_pts + g.away_pts AS total, COALESCE(g.kind, 'regular') AS kind
               FROM log_games g WHERE g.stream = 'college' AND {ENGINE})
     SELECT * FROM (SELECT 'margin' AS list, * FROM x ORDER BY margin DESC LIMIT 60) UNION ALL SELECT * FROM (SELECT 'total', * FROM x ORDER BY total DESC LIMIT 60)""",
  needs=G, open="game/{id}")

check("G1.fbs_one_score", GP, "G1", "FBS games decided by 8 or less", "%",
  "SELECT AVG(ABS(home_pts - away_pts) <= 8) FROM log_games g WHERE g.stream = 'college' AND " + ENGINE + " AND g.divisions = '[\"FBS\",\"FBS\"]'",
  Key("college_scores.json:divisions.fbs#all.decided_by_8_or_less", 100.0), (0.08, 0.2, 0.0),
  "Every engine game between two FBS programs.", scale=100.0,
  years="SELECT season, AVG(ABS(home_pts - away_pts) <= 8) FROM log_games g WHERE g.stream = 'college' AND " + ENGINE + " AND g.divisions = '[\"FBS\",\"FBS\"]' GROUP BY season", needs=G)
check("G1.fbs_blowouts", GP, "G1", "FBS games decided by 28 or more", "%",
  "SELECT AVG(ABS(home_pts - away_pts) >= 28) FROM log_games g WHERE g.stream = 'college' AND " + ENGINE + " AND g.divisions = '[\"FBS\",\"FBS\"]'",
  Key("college_scores.json:divisions.fbs#all.decided_by_28_plus", 100.0), (0.1, 0.25, 0.0),
  "Every engine game between two FBS programs.", scale=100.0, needs=G)
check("G1.fbs_home_win", GP, "G1", "FBS home teams' win share", "%",
  "SELECT AVG(home_pts > away_pts) FROM log_games g WHERE g.stream = 'college' AND " + ENGINE + " AND g.divisions = '[\"FBS\",\"FBS\"]' AND COALESCE(g.neutral, 0) = 0",
  Key("college_scores.json:divisions.fbs#all.home_win", 100.0), (0.05, 0.12, 0.0),
  "FBS against FBS engine games at one side's home (neutral sites left out).", scale=100.0, needs=G)
check("G1.fbs_home_margin", GP, "G1", "FBS home margin", "pts",
  "SELECT AVG(home_pts - away_pts) FROM log_games g WHERE g.stream = 'college' AND " + ENGINE + " AND g.divisions = '[\"FBS\",\"FBS\"]' AND COALESCE(g.neutral, 0) = 0",
  Key("college_scores.json:divisions.fbs#all.home_margin"), (0.15, 0.4, 0.5),
  "Home points less away points, FBS against FBS at one side's home.", needs=G)
check("G1.fbs_points_sd", GP, "G1", "Spread of FBS team scores (sd)", "pts",
  "WITH " + COLLEGE_TG + " SELECT sqrt(AVG(pf * pf) - AVG(pf) * AVG(pf)) FROM tg WHERE div = 'FBS' AND odiv = 'FBS'",
  Key("college_scores.json:divisions.fbs#all.points_sd"), (0.08, 0.2, 0.0),
  "Standard deviation of a team's points, FBS against FBS engine games.", needs=G)
check("G1.fbs_shutouts", GP, "G1", "FBS team games shut out", "%",
  "WITH " + COLLEGE_TG + " SELECT AVG(pf = 0) FROM tg WHERE div = 'FBS' AND odiv = 'FBS'",
  Key("college_scores.json:divisions.fbs#all.shutout_share_of_team_games", 100.0), (0.2, 0.5, 0.3),
  "Team games with no points, FBS against FBS engine games.", scale=100.0, needs=G)
check("G1.fbs_by_3", GP, "G1", "FBS games ending by exactly 3", "%",
  "SELECT AVG(ABS(home_pts - away_pts) = 3) FROM log_games g WHERE g.stream = 'college' AND " + ENGINE + " AND " + REG + " AND g.divisions = '[\"FBS\",\"FBS\"]'",
  Key("college_history.json:fbs_margin_table#share_by_final_margin.3", 100.0), (0.15, 0.35, 0.5),
  "FBS against FBS regular-season engine games.", scale=100.0, needs=G)
check("G1.fbs_by_7", GP, "G1", "FBS games ending by exactly 7", "%",
  "SELECT AVG(ABS(home_pts - away_pts) = 7) FROM log_games g WHERE g.stream = 'college' AND " + ENGINE + " AND " + REG + " AND g.divisions = '[\"FBS\",\"FBS\"]'",
  Key("college_history.json:fbs_margin_table#share_by_final_margin.7", 100.0), (0.15, 0.35, 0.5),
  "FBS against FBS regular-season engine games.", scale=100.0, needs=G)
for d, k in [("D2", "ii"), ("D3", "iii")]:
    check(f"G1.{d.lower()}_points", GP, "G1", f"{d} points a team game", "pts",
      "WITH " + COLLEGE_TG + f" SELECT AVG(pf) FROM tg WHERE div = '{d}' AND odiv = '{d}'",
      Key(f"college_scores.json:divisions.{k}#all.points_per_team_game"), (0.08, 0.2, 0.0),
      f"Engine games between two {d} programs.", needs=G)
check("G1.fbs_over_fcs", GP, "G1", "FBS wins over FCS", "%",
  "SELECT AVG(CASE WHEN json_extract(divisions, '$[0]') = 'FBS' THEN home_pts > away_pts ELSE away_pts > home_pts END) FROM log_games g WHERE g.stream = 'college' AND " + ENGINE + " AND g.divisions IN ('[\"FBS\",\"FCS\"]', '[\"FCS\",\"FBS\"]')",
  Fixed(92.3, "college_history.json scores_by_season: FBS won 92.3% of 2,340 FBS v FCS games, weighted by games (CollegeFootballData)", "2001-2025"), (0.04, 0.1, 0.0),
  "Every engine game between an FBS and an FCS program: the FBS side's win share.", scale=100.0, needs=G)

# ---------------------------------------------------------------- G2
def box_avg_rows(div_filter, rows):
    out = []
    for i, (label, expr, real_expr) in enumerate(rows):
        out.append(f"SELECT {i} AS o, '{label}' AS label, ROUND(AVG({expr}), 3) AS world, {real_expr} AS real, COUNT(*) AS n FROM fb")
    return " UNION ALL ".join(out)

FB = f"""fb AS (SELECT b.*, g.season AS year, json_extract(g.divisions, '$[' || b._i || ']') AS div FROM log_games__box b JOIN log_games g ON g._row = b._parent
                WHERE g.stream = 'college' AND {ENGINE} AND json_extract(g.divisions, '$[0]') = json_extract(g.divisions, '$[1]'))"""
def cgt(div, path): return f"json_extract({CGT}, '$.divisions.' || lower({div}) || '.{path}')"
G2ROWS = [
  ("Plays", "plays", cgt("div", "per_team_game.plays")),
  ("Pass attempts", "att", cgt("div", "per_team_game.att")),
  ("Completions", "cmp", cgt("div", "per_team_game.cmp")),
  ("Pass yards", "pass_yards", cgt("div", "per_team_game.pass_yards")),
  ("Carries", "carries", cgt("div", "per_team_game.carries")),
  ("Rush yards", "rush_yards", cgt("div", "per_team_game.rush_yards")),
  ("Pass touchdowns", "pass_td", cgt("div", "per_team_game.pass_td")),
  ("Rush touchdowns", "rush_td", cgt("div", "per_team_game.rush_td")),
  ("Sacks", "sacks", cgt("div", "per_team_game.sacks")),
  ("Interceptions thrown", "ints", cgt("div", "per_team_game.int")),
  ("Fumbles lost", "fumbles_lost", cgt("div", "per_team_game.fumbles_lost")),
  ("Turnovers", "turnovers", "NULL"),
  ("First downs", "first_downs", "NULL"),
  ("Third downs tried", "third_att", cgt("div", "per_team_game.third")),
  ("Fourth downs tried", "fourth_att", cgt("div", "per_team_game.fourth_go")),
  ("Penalties", "penalties", cgt("div", "per_team_game.penalties")),
  ("Penalty yards", "penalty_yds", cgt("div", "per_team_game.penalty_yards")),
  ("Explosive plays", "explosive", "NULL"),
  ("Successful plays", "successes", cgt("div", "per_team_game.success")),
  ("Drives", "drives", "NULL"),
  ("Punts", "punts", "NULL"),
  ("Field goals tried", "fg_att", "NULL"),
  ("Red zone trips", "red_zone_trips", "NULL"),
  ("Red zone touchdowns", "red_zone_tds", "NULL"),
  ("Kick returns", "kr", "NULL"),
  ("Punt returns", "pr", "NULL"),
  ("Pressures", "pressures", "NULL"),
  ("QB hits", "qb_hits", "NULL"),
  ("Tackles for loss", "tfl", "NULL"),
  ("Expected points added", "epa", cgt("div", "per_team_game.epa")),
  ("Ball-in-play seconds (the box top line)", "time_of_possession", "NULL"),
  ("Game-clock possession, minutes", "clock_possession / 60.0", "30"),
]
panel("G2.vitals", GP, "G2", "Team box score a game against real, by division", "table",
  "Every box score number a team game, all engine games between two programs of the same division. Real: cfbfastR play-by-play 2022-2024 for FBS and FCS (college_game_targets.json); blank where no real figure is on file. Ball-in-play seconds is the engine's own count of live-ball time, not the game clock; the game clock is the line below it.",
  f"WITH {FB} SELECT grp, stat, world, real, n FROM (" + " UNION ALL ".join(
        f"SELECT div AS grp, {i} AS o, '{lab}' AS stat, ROUND(AVG({ex}), 2) AS world, ROUND({rex}, 2) AS real, COUNT(*) AS n FROM fb GROUP BY div" for i, (lab, ex, rex) in enumerate(G2ROWS)) + f") ORDER BY {divord('grp')}, o",
  says=f"""WITH {FB} SELECT printf('FBS teams run %.1f plays a game (real %.1f) for %.1f yards a play; red zone trips are counted since the game log (%.1f a team game).',
     (SELECT AVG(plays) FROM fb WHERE div = 'FBS'), json_extract({CGT}, '$.divisions.fbs.per_team_game.plays'),
     (SELECT 1.0 * SUM(yards) / SUM(plays) FROM fb WHERE div = 'FBS'), (SELECT AVG(red_zone_trips) FROM fb WHERE div = 'FBS'))""", needs=G)

def rate(label, num, den, real_expr, scale=100.0):
    return (label, num, den, real_expr, scale)
G2RATES = [
  ("Completion %", "cmp", "att", cgt("div", "rates.completion_pct"), 100),
  ("Yards an attempt", "pass_yards", "att", cgt("div", "rates.yards_per_attempt"), 1),
  ("Yards a carry", "rush_yards", "carries", cgt("div", "rates.yards_per_carry"), 1),
  ("Pass rate %", "dropbacks", "plays", cgt("div", "rates.pass_rate"), 100),
  ("Sack rate %", "sacks", "dropbacks", cgt("div", "rates.sack_rate"), 100),
  ("Interception rate %", "ints", "att", cgt("div", "rates.int_rate"), 100),
  ("Third-down conversion %", "third_conv", "third_att", cgt("div", "rates.third_down_conversion"), 100),
  ("Fourth-down conversion %", "fourth_conv", "fourth_att", cgt("div", "rates.fourth_down_conversion"), 100),
  ("Success rate %", "successes", "plays", cgt("div", "rates.success_rate"), 100),
  ("Explosive play rate %", "explosive", "plays", "NULL", 100),
  ("Yards a play", "yards", "plays", "NULL", 1),
  ("EPA a play", "epa", "plays", cgt("div", "rates.epa_per_play"), 1),
  ("Points a drive", "points", "drives", "NULL", 1),
  ("Red zone TD %", "red_zone_tds", "red_zone_trips", "NULL", 100),
  ("FG made %", "fg_made", "fg_att", cgt("div", "rates.fg_pct"), 100),
  ("Average drive start (yards from own goal)", "start_sum", "drives", "NULL", 1),
]
panel("G2.rates", GP, "G2", "Rates against real, by division", "bars",
  "Rates over every engine team game between two programs of the same division (sums over sums). Real: cfbfastR 2022-2024 for FBS and FCS. EPA uses the NFL's expected points table at every level, so college EPA leans off the real college line.",
  f"WITH {FB} " + " UNION ALL ".join(
     f"SELECT div AS grp, '{lab}' AS label, ROUND({sc} * SUM({n}) / NULLIF(SUM({d}), 0), 3) AS world, ROUND({sc} * {rex}, 3) AS real, COUNT(*) AS n FROM fb GROUP BY div"
     for lab, n, d, rex, sc in G2RATES), needs=G)

panel("G2.spread", GP, "G2", "How team games spread: plays, pass rate and yards an attempt", "hist",
  "FBS team games in buckets: plays run, the share of plays that were dropbacks, and yards an attempt. A healthy engine shows a spread, not a spike. No real spread per team game is on file; the real FBS team-season range of plays a game is 60.7 (5th percentile) to 78.5 (95th).",
  f"""WITH {FB}, f AS (SELECT * FROM fb WHERE div = 'FBS'),
       x AS (SELECT 'Plays' AS grp, 5 * CAST(plays / 5 AS INTEGER) AS v FROM f
             UNION ALL SELECT 'Pass rate %', 5 * CAST(100.0 * dropbacks / NULLIF(plays, 0) / 5 AS INTEGER) FROM f
             UNION ALL SELECT 'Yards an attempt', CAST(1.0 * pass_yards / NULLIF(att, 0) AS INTEGER) FROM f WHERE att >= 10)
     SELECT grp, v AS x, 100.0 * COUNT(*) / SUM(COUNT(*)) OVER (PARTITION BY grp) AS world, COUNT(*) AS n FROM x WHERE v IS NOT NULL GROUP BY grp, v ORDER BY grp, v""", needs=G)

panel("G2.by_year", GP, "G2", "FBS vitals year by year against real", "line",
  "FBS against FBS engine games each season: plays a team game, yards a play and completion %. Real: cfbfastR's season lines (college_plays.json, 2014-2025).",
  f"""WITH {FB}
     SELECT 'Plays a team game' AS grp, year, ROUND(AVG(plays), 2) AS world, json_extract({CP}, '$.by_season."' || year || '".plays_per_team_game') AS real FROM fb WHERE div = 'FBS' GROUP BY year
     UNION ALL SELECT 'Yards a play', year, ROUND(1.0 * SUM(yards) / SUM(plays), 3), json_extract({CP}, '$.by_season."' || year || '".yards_per_play') FROM fb WHERE div = 'FBS' GROUP BY year
     UNION ALL SELECT 'Completion %', year, ROUND(100.0 * SUM(cmp) / SUM(att), 2), 100 * json_extract({CP}, '$.by_season."' || year || '".completion_pct') FROM fb WHERE div = 'FBS' GROUP BY year
     UNION ALL SELECT 'Pass rate %', year, ROUND(100.0 * SUM(dropbacks) / SUM(plays), 2), 100 * json_extract({CP}, '$.by_season."' || year || '".pass_rate') FROM fb WHERE div = 'FBS' GROUP BY year
     ORDER BY 1, 2""", needs=G)

check("G2.fbs_success", GP, "G2", "FBS success rate", "%",
  f"WITH {FB} SELECT 1.0 * SUM(successes) / SUM(plays) FROM fb WHERE div = 'FBS'",
  Key("college_game_targets.json:divisions.fbs#rates.success_rate", 100.0), (0.06, 0.15, 0.0),
  "Successful plays over plays, FBS against FBS engine games.", scale=100.0, needs=G)
check("G2.fbs_third_down", GP, "G2", "FBS third-down conversion", "%",
  f"WITH {FB} SELECT 1.0 * SUM(third_conv) / SUM(third_att) FROM fb WHERE div = 'FBS'",
  Key("college_game_targets.json:divisions.fbs#rates.third_down_conversion", 100.0), (0.06, 0.15, 0.0),
  "Third downs converted over tried, FBS against FBS engine games.", scale=100.0, needs=G)
check("G2.fbs_turnovers", GP, "G2", "FBS interceptions thrown a team game", "",
  f"WITH {FB} SELECT AVG(ints) FROM fb WHERE div = 'FBS'",
  Key("college_game_targets.json:divisions.fbs#per_team_game.int"), (0.1, 0.25, 0.0),
  "FBS against FBS engine games.", needs=G)
check("G2.fbs_penalties", GP, "G2 G11", "FBS penalties a team game", "",
  f"WITH {FB} SELECT AVG(penalties) FROM fb WHERE div = 'FBS'",
  Key("college_game_targets.json:divisions.fbs#per_team_game.penalties"), (0.1, 0.25, 0.0),
  "Accepted penalties a team game, FBS against FBS engine games.", needs=G)
check("G2.red_zone_counted", GP, "G2", "Red zone trips counted (FBS, a team game)", "",
  f"WITH {FB} SELECT AVG(red_zone_trips) FROM fb WHERE div = 'FBS'",
  Memory(3.6, "FBS teams average about 3.5-4 red zone trips a game (cfbstats team red zone tables)"), (0.15, 0.35, 0.0),
  "A drive with a snap at the opponent's 20 or closer counts once (game/drives.rs). Before the game log these read 0 in every game.", needs=G)

# ---------------------------------------------------------------- G3
NB = f"""nb AS (SELECT b.*, g.season AS year, g.kind AS kind FROM log_games__box b JOIN log_games g ON g._row = b._parent WHERE g.stream = 'nfl' AND {ENGINE} AND {REG})"""
def ngt(path): return f"json_extract({NGT}, '$.{path}')"
G3ROWS = [
  ("Points", "points", ngt("points_per_team_game")), ("Plays", "plays", ngt("per_team_game.plays")),
  ("Pass attempts", "att", ngt("per_team_game.att")), ("Completions", "cmp", ngt("per_team_game.cmp")),
  ("Dropbacks", "dropbacks", ngt("per_team_game.dropbacks")), ("Pass yards (gross)", "pass_yards", ngt("per_team_game.pass_yards")),
  ("Sack yards", "sack_yds", ngt("per_team_game.sack_yards")), ("Carries", "carries", ngt("per_team_game.carries")),
  ("Rush yards", "rush_yards", ngt("per_team_game.rush_yards")), ("Pass touchdowns", "pass_td", ngt("per_team_game.pass_td")),
  ("Rush touchdowns", "rush_td", ngt("per_team_game.rush_td")), ("Sacks", "sacks", ngt("per_team_game.sacks")),
  ("Interceptions thrown", "ints", ngt("per_team_game.int")), ("Fumbles lost", "fumbles_lost", ngt("per_team_game.fumbles_lost")),
  ("Third downs tried", "third_att", ngt("per_team_game.third")), ("Third downs made", "third_conv", ngt("per_team_game.third_conv")),
  ("Fourth downs tried", "fourth_att", ngt("per_team_game.fourth_go")), ("Fourth downs made", "fourth_conv", ngt("per_team_game.fourth_conv")),
  ("Penalties", "penalties", ngt("per_team_game.penalties")), ("Penalty yards", "penalty_yds", ngt("per_team_game.penalty_yards")),
  ("Successful plays", "successes", ngt("per_team_game.success")), ("Expected points added", "epa", ngt("per_team_game.epa")),
  ("Drives", "drives", "11.0"), ("Field goals tried", "fg_att", "1.85"), ("Punts", "punts", "NULL"), ("Red zone trips", "red_zone_trips", "NULL"),
  ("Red zone touchdowns", "red_zone_tds", "NULL"), ("Extra points tried", "xp_att", "NULL"), ("Two-point tries", "two_point", "NULL"),
  ("Kickoffs", "kickoffs", "NULL"), ("Kickoff touchbacks", "ko_tb", "NULL"), ("Kick returns", "kr", "NULL"), ("Punt returns", "pr", "NULL"),
  ("QB hits", "qb_hits", "NULL"), ("Pressures", "pressures", "NULL"),
]
panel("G3.vitals", GP, "G3", "NFL team box score a game against real", "bars",
  "Every regular-season NFL engine game. Real: nflverse play-by-play 2021-2025 (nfl_game_targets.json), drives and field goal tries from Pro Football Reference's drive stats 2021-2024 (match_check's rows). Pass yards are gross (before sacks).",
  f"WITH {NB} " + " UNION ALL ".join(f"SELECT '{lab}' AS label, ROUND(AVG({ex}), 3) AS world, ROUND({rex}, 3) AS real, COUNT(*) AS n FROM nb" for lab, ex, rex in G3ROWS),
  says=f"""WITH {NB} SELECT printf('NFL teams score %.1f a game (real %.1f) on %.1f plays (real %.1f).', AVG(points), {ngt('points_per_team_game')}, AVG(plays), {ngt('per_team_game.plays')}) FROM nb""", needs=G)

NRATES = [
  ("Completion %", "cmp", "att", ngt("rates.completion_pct"), 100), ("Yards an attempt", "pass_yards", "att", ngt("rates.yards_per_attempt"), 1),
  ("Sack rate %", "sacks", "dropbacks", ngt("rates.sack_rate"), 100), ("Interception rate %", "ints", "att", ngt("rates.int_rate"), 100),
  ("Pass TD rate %", "pass_td", "att", ngt("rates.pass_td_rate"), 100), ("Yards a carry", "rush_yards", "carries", ngt("rates.yards_per_carry"), 1),
  ("Pass rate %", "dropbacks", "plays", ngt("rates.pass_rate"), 100), ("Third-down conversion %", "third_conv", "third_att", ngt("rates.third_down_conversion"), 100),
  ("Fourth-down conversion %", "fourth_conv", "fourth_att", ngt("rates.fourth_down_conversion"), 100), ("Success rate %", "successes", "plays", ngt("rates.success_rate"), 100),
  ("EPA a play", "epa", "plays", ngt("rates.epa_per_play"), 1), ("Extra point %", "xp_made", "xp_att", ngt("rates.xp_pct"), 100),
  ("Punt gross yards", "punt_yds", "punts", ngt("rates.punt_gross"), 1), ("Punts inside the 20 %", "punt_in20", "punts", ngt("rates.punt_inside20_rate"), 100),
  ("Kickoff touchback %", "ko_tb", "kickoffs", ngt("rates.kickoff_touchback_rate"), 100), ("Points a drive", "points", "drives", "2.0", 1),
  ("Red zone TD % (trips)", "red_zone_tds", "red_zone_trips", "0.598", 100), ("Average drive start (from own goal)", "start_sum", "drives", "29.5", 1),
]
panel("G3.rates", GP, "G3", "NFL rates against real", "bars",
  "Sums over sums, every regular-season NFL engine game. Real: nflverse 2021-2025; points a drive and drive start from Pro Football Reference 2021-2024; red zone TD share 59.8% a trip (engine_targets.json).",
  f"WITH {NB} " + " UNION ALL ".join(f"SELECT '{lab}' AS label, ROUND({sc} * SUM({n}) / NULLIF(SUM({d}), 0), 3) AS world, ROUND({sc} * {rex}, 3) AS real, COUNT(*) AS n FROM nb" for lab, n, d, rex, sc in NRATES), needs=G)

panel("G3.by_year", GP, "G3", "NFL scoring and pace by season against real", "line",
  "Each season's regular-season engine games: points a team game, plays a team game and the share of games within one score. Real: nflverse by season (nfl_history.json, 1996-2025). Every world game uses today's rule book (code oddity 14), so history seasons play today's kickoff and overtime.",
  f"""WITH {NB}
     SELECT 'Points a team game' AS grp, year, ROUND(AVG(points), 2) AS world, json_extract({NH}, '$.scoring_by_season.by_season."' || year || '".points_per_team_game') AS real FROM nb GROUP BY year
     UNION ALL SELECT 'Plays a team game', year, ROUND(AVG(plays), 2), json_extract({NH}, '$.plays_by_season.by_season."' || year || '".plays_per_team_game') FROM nb GROUP BY year
     UNION ALL SELECT 'Completion %', year, ROUND(100.0 * SUM(cmp) / SUM(att), 2), 100 * json_extract({NH}, '$.plays_by_season.by_season."' || year || '".completion_pct') FROM nb GROUP BY year
     UNION ALL SELECT 'Yards a play', year, ROUND(1.0 * SUM(yards) / SUM(plays), 3), json_extract({NH}, '$.plays_by_season.by_season."' || year || '".yards_per_play') FROM nb GROUP BY year
     ORDER BY 1, 2""", needs=G)

panel("G3.shape", GP, "G3", "NFL game shape against real", "bars",
  "Regular-season engine games: close games, blowouts, home wins, overtime, ties and shutouts. Real: nflverse 2021-2025 for the one-score share, and the season lines of nfl_history.json averaged over 2021-2025 for the rest.",
  f"""WITH x AS (SELECT g.season AS year, g.home_pts AS h, g.away_pts AS a, COALESCE(g.ot, 0) AS ot, COALESCE(g.neutral, 0) AS neutral FROM log_games g WHERE g.stream = 'nfl' AND {ENGINE} AND {REG}),
       r AS (SELECT AVG(json_extract(value, '$.blowout_21plus_share')) AS blow, AVG(json_extract(value, '$.home_win_pct')) AS home, AVG(json_extract(value, '$.overtime_share')) AS ot,
                    SUM(json_extract(value, '$.ties')) * 1.0 / SUM(json_extract(value, '$.games')) AS ties, SUM(json_extract(value, '$.shutout_team_games')) * 1.0 / SUM(2 * json_extract(value, '$.games')) AS shut,
                    AVG(json_extract(value, '$.margin_abs_mean')) AS margin
               FROM json_each({NH}, '$.scoring_by_season.by_season') WHERE CAST(key AS INTEGER) >= 2021)
     SELECT 'Decided by 8 or less' AS label, 100.0 * AVG(ABS(h - a) <= 8) AS world, 100.0 * {ngt('share_games_decided_by_8_or_less')} AS real, COUNT(*) AS n FROM x
     UNION ALL SELECT 'Decided by 21 or more', 100.0 * AVG(ABS(h - a) >= 21), 100.0 * (SELECT blow FROM r), COUNT(*) FROM x
     UNION ALL SELECT 'Home team won', 100.0 * AVG(h > a), 100.0 * (SELECT home FROM r), COUNT(*) FROM x WHERE neutral = 0
     UNION ALL SELECT 'Went to overtime', 100.0 * AVG(ot > 0), 100.0 * (SELECT ot FROM r), COUNT(*) FROM x
     UNION ALL SELECT 'Ties', 100.0 * AVG(h = a), 100.0 * (SELECT ties FROM r), COUNT(*) FROM x
     UNION ALL SELECT 'Team games shut out', 50.0 * AVG((h = 0) + (a = 0)), 100.0 * (SELECT shut FROM r), COUNT(*) FROM x""", needs=G)

panel("G3.games_per_year", GP, "G3", "NFL games each season by kind", "table",
  "How many NFL games the game log holds each season, by kind, against the real count (272 regular-season games and 13 playoff games from 2021; 256 and 11 or 12 before). Preseason games are played on the engine too.",
  """SELECT season AS year, COALESCE(kind, 'regular') AS kind, COUNT(*) AS games, SUM(COALESCE(standin, 0)) AS stand_in, SUM(COALESCE(ot, 0) > 0) AS overtime, SUM(home_pts = away_pts) AS ties
       FROM log_games WHERE stream = 'nfl' GROUP BY season, kind ORDER BY season DESC, games DESC""", needs=G)

check("G3.nfl_home_win", GP, "G3", "NFL home teams' win share", "%",
  "SELECT AVG(home_pts > away_pts) FROM log_games g WHERE g.stream = 'nfl' AND " + ENGINE + " AND " + REG + " AND COALESCE(g.neutral, 0) = 0",
  Fixed(53.8, "engine_targets.json home_field_nfl (nflverse games, 2021-2025)", "2021-2025"), (0.04, 0.1, 0.0),
  "Regular-season NFL engine games at one side's home.", scale=100.0, needs=G)
check("G3.nfl_overtime", GP, "G3", "NFL games to overtime", "%",
  "SELECT AVG(COALESCE(ot, 0) > 0) FROM log_games g WHERE g.stream = 'nfl' AND " + ENGINE + " AND " + REG,
  Fixed(6.1, "nfl_history.json scoring_by_season overtime_share, 2021-2025 weighted by games: 83 of 1,359 (nflverse)", "2021-2025"), (0.25, 0.6, 1.0),
  "Regular-season NFL engine games.", scale=100.0, needs=G)
check("G3.nfl_games_2021", GP, "G3 N3", "NFL regular-season games a season (2021 on)", "",
  "SELECT AVG(n) FROM (SELECT season, COUNT(*) AS n FROM log_games WHERE stream = 'nfl' AND COALESCE(kind, 'regular') = 'regular' AND season >= 2021 GROUP BY season)",
  Fixed(272.0, "17 games for 32 clubs (calendar/rules.rs RuleBook::era)", "2021-2025"), (0.001, 0.01, 0.0),
  "Regular-season NFL games in the game log per season from 2021.", needs=G)

# ---------------------------------------------------------------- G4
panel("G4.champions", GP, "G4", "Super Bowl champions, the world against real", "table",
  "Each season's champion in the world beside the real winner of the same season (nfl_history.json, 1996-2025). A season the world played after 2025 has no real winner.",
  f"""SELECT c._0 AS year, {club('c._1', 'c._0')} AS world_champion, json_extract({NH}, '$.champions.by_season."' || c._0 || '".super_bowl_winner') AS real_champion,
            (SELECT COUNT(*) FROM nfl_champions c2 WHERE c2._1 = c._1 AND c2._0 <= c._0) AS its_titles_so_far, c._1 AS id
       FROM nfl_champions c ORDER BY c._0 DESC""", open="e/nfl/{id}")

panel("G4.parity", GP, "G4", "Parity: spread of wins and how sticky they are", "line",
  "Each season: the spread (sd) of clubs' win shares, and the correlation of a club's win share with the year before. Real: nflverse (nfl_history.json parity): win share sd by season, year-to-year correlation 0.33 over 1999-2024.",
  f"""WITH s AS (SELECT year, team, (wins + 0.5 * ties) * 1.0 / (wins + losses + ties) AS w FROM nfl_seasons WHERE wins + losses + ties > 0),
       sd AS (SELECT year, sqrt(AVG(w * w) - AVG(w) * AVG(w)) AS v FROM s GROUP BY year),
       pr AS (SELECT a.year, a.w AS x, b.w AS y FROM s a JOIN s b ON b.team = a.team AND b.year = a.year - 1),
       co AS (SELECT year, (AVG(x * y) - AVG(x) * AVG(y)) / sqrt((AVG(x * x) - AVG(x) * AVG(x)) * (AVG(y * y) - AVG(y) * AVG(y))) AS r FROM pr GROUP BY year HAVING COUNT(*) > 5)
     SELECT 'Win share sd' AS grp, sd.year, ROUND(sd.v, 3) AS world, json_extract({NH}, '$.parity.team_win_pct_sd_by_season."' || sd.year || '"') AS real FROM sd
     UNION ALL SELECT 'Year-to-year correlation', co.year, ROUND(co.r, 3), json_extract({NH}, '$.parity.win_pct_year_to_year_corr."1999-2024"') FROM co
     ORDER BY 1, 2""")

panel("G4.extremes", GP, "G4", "Win totals: great and awful seasons a year", "line",
  "Each season: clubs with 13 or more wins and with 3 or fewer, and the best record. Real: about 3-4 clubs at each end a year; one 16-0 season (2007) since 1984 (nfl_history.json best record by season).",
  f"""SELECT year, SUM(wins >= 13) AS thirteen_plus, SUM(wins <= 3) AS three_or_fewer, MAX(wins) AS most_wins,
            json_extract({NH}, '$.parity.best_record_by_season."' || year || '"') AS real
       FROM nfl_seasons GROUP BY year ORDER BY year""")

panel("G4.windows", GP, "G4", "Titles in every 15 seasons", "line",
  "For each 15-season window ending in a year: distinct Super Bowl champions and the most titles one club won. Real: 9-12 distinct champions and at most 3-4 titles by one club in any 15 seasons since 1996 (nfl_history.json).",
  f"""WITH e AS (SELECT DISTINCT _0 AS e FROM nfl_champions WHERE _0 >= (SELECT MIN(_0) + 14 FROM nfl_champions)),
       w AS (SELECT e.e, c._1 AS t, COUNT(*) AS n FROM e JOIN nfl_champions c ON c._0 BETWEEN e.e - 14 AND e.e GROUP BY e.e, c._1)
     SELECT e AS year, COUNT(*) AS distinct_champions, MAX(n) AS most_titles,
            json_extract({NH}, '$.parity.super_bowl_15yr_windows."' || (e - 14) || '-' || e || '".distinct_champions') AS real
       FROM w GROUP BY e ORDER BY e""")

panel("G4.seeds", GP, "G4", "How far each seed goes (this and last season)", "table",
  "The playoff field of the two seasons the world keeps whole: each club's seed, where it went out and its record. The world keeps only the round reached for older seasons.",
  f"""SELECT s.year, {club('t.key', 's.year')} AS club, json_extract(s.seed, '$[' || t.key || ']') AS seed, json_extract(s.wins, '$[' || t.key || ']') || '-' || json_extract(s.losses, '$[' || t.key || ']') AS record,
            CASE json_extract(s.out_round, '$[' || t.key || ']') WHEN 5 THEN 'champion' WHEN 4 THEN 'lost the Super Bowl' WHEN 3 THEN 'lost in the conference title game' WHEN 2 THEN 'lost in the divisional round' WHEN 1 THEN 'lost in the wild card round' ELSE 'out' END AS went, t.key AS id
       FROM (SELECT * FROM nfl_season UNION ALL SELECT * FROM nfl_prev) s, json_each(s.seed) t WHERE t.value > 0 ORDER BY s.year DESC, json_extract(s.seed, '$[' || t.key || ']')""",
  open="club/{id}/{year}")

check("G4.parity_corr", GP, "G4 N2", "NFL win share year-to-year correlation", "",
  "WITH s AS (SELECT year, team, (wins + 0.5 * ties) * 1.0 / (wins + losses + ties) AS w FROM nfl_seasons WHERE wins + losses + ties > 0), pr AS (SELECT a.w AS x, b.w AS y FROM s a JOIN s b ON b.team = a.team AND b.year = a.year - 1) SELECT (AVG(x * y) - AVG(x) * AVG(y)) / sqrt((AVG(x * x) - AVG(x) * AVG(x)) * (AVG(y * y) - AVG(y) * AVG(y))) FROM pr",
  Key("nfl_history.json:parity.win_pct_year_to_year_corr#1999-2024"), (0.15, 0.35, 0.05),
  "Every club season against the same club's season before, all seasons the world holds.")
check("G4.most_titles_15", GP, "G4 N2", "Most Super Bowls by one club in any 15 seasons", "",
  "WITH e AS (SELECT DISTINCT _0 AS e FROM nfl_champions WHERE _0 >= (SELECT MIN(_0) + 14 FROM nfl_champions)), w AS (SELECT e.e, c._1, COUNT(*) AS n FROM e JOIN nfl_champions c ON c._0 BETWEEN e.e - 14 AND e.e GROUP BY 1, 2) SELECT MAX(n) FROM w",
  Fixed(4.0, "nfl_history.json parity.super_bowl_15yr_windows (New England, 2001-2015 and later windows)", "1996-2025"), (0.0, 0.26, 0.0),
  "Champions over every 15-season window the world holds; needs 15 seasons.")

# ---------------------------------------------------------------- G5
panel("G5.titles_ladder", GP, "G5", "National titles in every 15 seasons", "line",
  "For each 15-season window ending in a year: distinct FBS champions and the most titles one program won. Real: at most 6 (Alabama 2006-2020) and 7-9 distinct champions (trial3_baselines.json).",
  f"""WITH t AS (SELECT s._parent AS prog, s.year AS y FROM programs__seasons s JOIN programs p ON p._row = s._parent WHERE p.division = 'FBS' AND s.national_title = 1),
       e AS (SELECT DISTINCT y AS e FROM t WHERE y >= (SELECT MIN(y) + 14 FROM t)),
       c AS (SELECT e.e, t.prog, COUNT(*) AS n FROM e JOIN t ON t.y BETWEEN e.e - 14 AND e.e GROUP BY e.e, t.prog)
     SELECT e AS year, MAX(n) AS most_titles, COUNT(*) AS distinct_champions,
            json_extract({T3}, '$.titles_per_15_years."' || (e - 14) || '-' || e || '".most_titles') AS real FROM c GROUP BY e ORDER BY e""")

panel("G5.champions", GP, "G5", "National champions by year, beside the real ones", "table",
  "Each season's FBS champion in the world and the real title game winner of the same season (college_history.json, 2001-2025), with the champion's record and how many titles it had by then.",
  f"""SELECT s.year, {prog()} AS world_champion, s.wins || '-' || s.losses AS record,
            (SELECT COUNT(*) FROM programs__seasons s2 WHERE s2._parent = s._parent AND s2.national_title = 1 AND s2.year <= s.year) AS titles_so_far,
            json_extract({CH}, '$.champions.by_season."' || s.year || '".FBS_title_game_winner') AS real_champion, s._parent AS id
       FROM programs__seasons s JOIN programs p ON p._row = s._parent WHERE s.national_title = 1 AND p.division = 'FBS' ORDER BY s.year DESC""",
  open="e/programs/{id}")

panel("G5.unbeaten", GP, "G5", "Unbeaten FBS seasons and stickiness", "line",
  "Each season: FBS programs with no loss, and the correlation of a program's win share with the year before. Real: 9 unbeaten seasons in 2011-2025 (about 0.6 a year) and a year-to-year correlation of 0.536 (trial3_baselines.json).",
  f"""WITH s AS (SELECT s.year, s._parent AS t, s.wins, s.losses, 1.0 * s.wins / NULLIF(s.wins + s.losses, 0) AS w FROM programs__seasons s JOIN programs p ON p._row = s._parent WHERE p.division = 'FBS'),
       pr AS (SELECT a.year, a.w AS x, b.w AS y FROM s a JOIN s b ON b.t = a.t AND b.year = a.year - 1 WHERE a.w IS NOT NULL AND b.w IS NOT NULL),
       co AS (SELECT year, (AVG(x * y) - AVG(x) * AVG(y)) / sqrt((AVG(x * x) - AVG(x) * AVG(x)) * (AVG(y * y) - AVG(y) * AVG(y))) AS r FROM pr GROUP BY year HAVING COUNT(*) > 10)
     SELECT s.year, SUM(s.losses = 0 AND s.wins > 0) AS unbeaten, ROUND((SELECT r FROM co WHERE co.year = s.year), 3) AS year_to_year,
            0.6 AS real FROM s GROUP BY s.year ORDER BY s.year""")

panel("G5.postseason", GP, "G5", "Bowls, playoffs and title games each season", "table",
  "College postseason games in the game log by season, division and kind, against the real FBS count of bowl and playoff games (college_history.json, 2001-2025). Opt-outs are the world's yearly count.",
  f"""SELECT g.season AS year, json_extract(g.divisions, '$[0]') AS division, COALESCE(g.kind, 'regular') AS kind, COUNT(*) AS games, SUM(COALESCE(g.standin, 0)) AS stand_in,
            CASE WHEN json_extract(g.divisions, '$[0]') = 'FBS' AND g.kind IN ('bowl', 'playoff', 'title') THEN json_extract({CH}, '$.postseason.by_season."' || g.season || '".fbs_bowl_and_playoff_games') END AS real_fbs_postseason,
            (SELECT opt_outs FROM log_history h WHERE h.year = g.season) AS opt_outs
       FROM log_games g WHERE g.stream = 'college' AND COALESCE(g.kind, 'regular') != 'regular' GROUP BY 1, 2, 3 ORDER BY 1 DESC, 2, 3""", needs=G)

panel("G5.droughts", GP, "G5", "Longest runs without a winning season (power programs)", "table",
  "For every program in a power conference today: its longest run of seasons in the world without a winning record, and the run still going. Real: Kansas's 14 seasons in 2001-2025 is the longest among power programs (trial3_baselines.json).",
  f"""WITH s AS (SELECT s._parent AS id, s.year, s.wins > s.losses AS good FROM programs__seasons s JOIN programs p ON p._row = s._parent
               WHERE p.conference IN ('SEC', 'Big Ten', 'Big 12', 'ACC', 'Pac-12')),
       g AS (SELECT id, year, good, year - ROW_NUMBER() OVER (PARTITION BY id, good ORDER BY year) AS run FROM s),
       r AS (SELECT id, MIN(year) AS from_year, MAX(year) AS to_year, COUNT(*) AS seasons FROM g WHERE good = 0 GROUP BY id, run)
     SELECT {prog()} AS program, p.conference, MAX(r.seasons) AS longest, (SELECT seasons FROM r r2 WHERE r2.id = r.id AND r2.to_year = (SELECT MAX(year) FROM programs__seasons)) AS still_going,
            json_extract({T3}, '$.p4_droughts.longest_run_without_winning_season_2001_2025_top10."' || {prog()} || '"') AS real_2001_2025, r.id
       FROM r JOIN programs p ON p._row = r.id GROUP BY r.id ORDER BY longest DESC LIMIT 60""", open="e/programs/{id}")

check("G5.unbeaten_rate", GP, "G5", "Unbeaten FBS seasons a year", "",
  "SELECT 1.0 * SUM(s.losses = 0 AND s.wins > 0) / COUNT(DISTINCT s.year) FROM programs__seasons s JOIN programs p ON p._row = s._parent WHERE p.division = 'FBS'",
  Fixed(0.6, "trial3_baselines.json: 9 unbeaten FBS seasons in 2011-2025", "2011-2025"), (0.3, 0.8, 0.2),
  "FBS program seasons with wins and no loss, over the seasons the world holds.")

# ---------------------------------------------------------------- G6
panel("G6.series", GP, "G6", "The 78 rivalries: the world's record against real", "table",
  "Each listed rivalry's series as the world keeps it (wins each side since it began counting, ties, the trophy holder), beside the real record loaded at the start (real_rivalries: all-time record through 1995 and the trophy holder then). Real head-to-head results for 1996-2025 are in college_rivalry_games.csv (1,667 games).",
  f"""SELECT r.a || ' and ' || r.b AS rivalry, json_extract(s.wins, '$[0]') AS wins_a, json_extract(s.wins, '$[1]') AS wins_b, s.ties, s.since,
            (SELECT {prog()} FROM programs p WHERE p._row = s.holder) AS holder, CASE WHEN s.real THEN 'yes' ELSE 'no' END AS real_record_loaded,
            r.a_wins_thru_1995 || '-' || r.b_wins_thru_1995 || '-' || r.ties_thru_1995 AS real_through_1995, r.first_year AS real_first_year, r.pauses AS real_pauses,
            (SELECT SUM(winner = 'a') || '-' || SUM(winner = 'b') FROM real_college_rivalry_games x WHERE x.a = r.a AND x.b = r.b) AS real_since_2001
       FROM college_prev__series s LEFT JOIN real_rivalries r ON r._row = s._row + 1 ORDER BY s._row""")

panel("G6.games", GP, "G6", "Rivalry games in the game log", "table",
  "Every game the schedule marked as a rivalry game, newest first: the score and how far it was from the line. Real: 1,667 rivalry games 1996-2025 in college_rivalry_games.csv for a season-by-season look.",
  f"""SELECT g._row AS id, g.season AS year, g.date, (SELECT {prog()} FROM programs p WHERE p._row = g.home) AS home, g.home_pts, (SELECT {prog()} FROM programs p WHERE p._row = g.away) AS away, g.away_pts,
            json_extract(g.line, '$[0]') AS home_line, g.loud FROM log_games g WHERE g.stream = 'college' AND COALESCE(g.rivalry, 0) != 0 ORDER BY g.date DESC""",
  needs=G, open="game/{id}")

panel("G6.upsets", GP, "G6", "Upsets in rivalry games against the rest", "bars",
  "Share of college engine games won by the side the line made the underdog, rivalry games against all others. Real: about a quarter of FBS games go to the underdog (college_plays.json betting lines, 2014-2025).",
  f"""WITH x AS (SELECT COALESCE(g.rivalry, 0) != 0 AS riv, json_extract(g.line, '$[0]') AS l, g.home_pts - g.away_pts AS m FROM log_games g WHERE g.stream = 'college' AND {ENGINE} AND json_extract(g.line, '$[0]') != 0)
     SELECT CASE WHEN riv THEN 'Rivalry games' ELSE 'Other games' END AS label, 100.0 * AVG((l < 0 AND m < 0) OR (l > 0 AND m > 0)) AS world, 26.0 AS real, COUNT(*) AS n FROM x GROUP BY riv""", needs=G)

# ---------------------------------------------------------------- G7 (high school page)
HP = "high-school"
panel("G7.yearly", HP, "G7", "High school football year by year", "line",
  "The world's yearly totals: engine games, points and plays a team game (log_history hs_totals). Engine games run for the high school years set in the world options; earlier seasons ran on the stand-in and score no points. Real: about 26 points a team game nationally (from memory; MaxPreps has no open data).",
  """SELECT year, hs_engine_games AS engine_games, ROUND(1.0 * hs_totals__points_for / NULLIF(hs_totals__games, 0), 2) AS points_a_team_game,
            ROUND(1.0 * hs_totals__plays / NULLIF(hs_totals__games, 0), 2) AS plays_a_team_game, ROUND(1.0 * hs_totals__yards / NULLIF(hs_totals__plays, 0), 2) AS yards_a_play,
            26.0 AS real FROM log_history WHERE hs_totals__games > 0 ORDER BY year""")

panel("G7.counters", HP, "G7", "Forfeits, no-contests, grade-outs, homecomings and senior nights", "line",
  "The high school year's counters (log_history). No real national counts are on file.",
  """SELECT year, hs_forfeits AS forfeits, hs_no_contests AS no_contests, hs_grade_outs AS grade_outs, hs_homecomings AS homecomings, hs_senior_nights AS senior_nights
       FROM log_history WHERE hs_games > 0 ORDER BY year""")

panel("G7.games", HP, "G7", "High school game shape (game log)", "bars",
  "High school engine games in the game log (both teams and the score, since the world reader): points, plays, running-clock games and shutouts. Real from memory: about 26 points a team game; no open national figures.",
  f"""WITH x AS (SELECT g.home_pts AS h, g.away_pts AS a, b0.plays AS p0, b1.plays AS p1 FROM log_games g
                 LEFT JOIN log_games__box b0 ON b0._parent = g._row AND b0._i = 0 LEFT JOIN log_games__box b1 ON b1._parent = g._row AND b1._i = 1
                WHERE g.stream = 'hs' AND {ENGINE})
     SELECT 'Points a team game' AS label, ROUND((AVG(h) + AVG(a)) / 2, 2) AS world, 26.0 AS real, COUNT(*) AS n FROM x
     UNION ALL SELECT 'Plays a team game', ROUND((AVG(p0) + AVG(p1)) / 2, 2), NULL, COUNT(p0) FROM x
     UNION ALL SELECT 'Games won by 35 or more (running clock)', ROUND(100.0 * AVG(ABS(h - a) >= 35), 2), NULL, COUNT(*) FROM x
     UNION ALL SELECT 'Team games shut out (%)', ROUND(50.0 * AVG((h = 0) + (a = 0)), 2), NULL, COUNT(*) FROM x""", needs=G)

panel("G7.streaks", HP, "G7", "Longest high school winning streaks", "table",
  "The longest straight wins the world's high schools have run (the stat book's high school streaks), beside the real national record: 151 straight by De La Salle (California), 1992-2004.",
  """SELECT h.name AS school, (SELECT code FROM states WHERE _row = h.state) AS state, s._0 AS streak, s._1 AS since, 151 AS real_record, h._row AS id FROM stats__hs_streak s JOIN high_schools h ON h._row = s._row WHERE s._0 > 0 ORDER BY s._0 DESC LIMIT 60""",
  open="e/high_schools/{id}")

panel("G7.state_titles", HP, "G7", "State titles: the most by one school in 15 seasons, by state", "table",
  "For each state: the school with the most state titles in the world's seasons, and how many distinct champions the state has had. Real: each state's powerhouses of 2026 are in hs_states.csv.",
  """WITH t AS (SELECT (SELECT code FROM states WHERE _row = h.state) AS state, h._row AS id, h.name, COUNT(*) AS n FROM high_schools__seasons s JOIN high_schools h ON h._row = s._parent WHERE s.state_title = 1 GROUP BY h._row)
     SELECT state, name AS most_titles, MAX(n) AS titles, (SELECT COUNT(*) FROM t t2 WHERE t2.state = t.state) AS distinct_champions, id FROM t GROUP BY state ORDER BY titles DESC""",
  open="e/high_schools/{id}")

check("G7.hs_points_log", HP, "G7", "High school points a team game (game log)", "pts",
  "SELECT (AVG(home_pts) + AVG(away_pts)) / 2 FROM log_games g WHERE g.stream = 'hs' AND " + ENGINE,
  Memory(26.0, "about 26 points a team game nationally (MaxPreps, from memory; world-trial10.md)"), (0.12, 0.3, 0.0),
  "Every high school engine game in the game log.", needs=G)
check("G7.longest_streak", HP, "G7 G20", "Longest high school winning streak", "games",
  "SELECT MAX(_0) FROM stats__hs_streak",
  Fixed(151.0, "De La Salle (CA), 1992-2004, the national record (NFHS record book)", "1992-2004"), (0.15, 0.4, 0.0),
  "The stat book's straight wins per school (stats.hs_streak).")

# ---------------------------------------------------------------- G8
SIT = "log_situations"
panel("G8.families", GP, "G8", "What the situation totals hold", "table",
  "Each key family of the season's situation totals (the game log sums every engine snap by key): how many keys and plays it holds, by level. The first part of a key is its family (kind, dd, field, fourth, state, two minute, call, defense, personnel, box, blitz, coverage, depth, pressured, time to throw, kick, return, penalty, injury and the real-data buckets).",
  f"""SELECT level AS grp, substr(key, 1, instr(key || '|', '|') - 1) AS family, COUNT(*) AS keys, SUM(plays) AS plays, COUNT(DISTINCT season) AS seasons
       FROM {SIT} WHERE "group" NOT LIKE 'cross%' GROUP BY level, family ORDER BY level, plays DESC""", needs=G)

def dd_real(down, togo):
    return None
panel("G8.pass_rate_dd", GP, "G8", "Pass rate by down and distance against real", "matrix",
  "Share of plays that were dropbacks by down (rows) and yards to go (columns), from the situation totals' real-data buckets (to go 1, 3, 6, 10, 15). Real, after each cell: the real pass rate of each score and clock bucket, weighted by how often the world was in it, so the two compare like for like. nflverse 2021-2025 (nfl_pass_rate.csv); cfbfastR 2023-2024 for FBS (college_pass_rate.csv).",
  f"""WITH k AS (SELECT level, "group" AS g, key, plays, CASE WHEN key LIKE '%|pass' THEN plays ELSE 0 END AS pass,
                    CASE WHEN key LIKE '%|pass' THEN substr(key, 16, length(key) - 20) ELSE substr(key, 16, length(key) - 19) END AS cell FROM {SIT}
               WHERE key LIKE 'real pass rate|%' AND "group" IN ('NFL', 'FBS')),
       c AS (SELECT level, cell, SUM(pass) AS pass, SUM(plays) AS plays FROM k GROUP BY level, cell),
       p AS (SELECT level, cell, pass, plays, CAST(substr(cell, 1, 1) AS INTEGER) AS down,
                    CAST(substr(cell, 3, instr(substr(cell, 3), '|') - 1) AS INTEGER) AS togo FROM c),
       r AS (SELECT 'nfl' AS level, down || '|' || togo || '|' || score || '|' || time AS cell, rate FROM real_nfl_pass_rate
             UNION ALL SELECT 'college', down || '|' || togo || '|' || score || '|' || time, rate FROM real_college_pass_rate)
     SELECT CASE p.level WHEN 'nfl' THEN 'NFL' ELSE 'FBS' END AS grp, 'Down ' || p.down AS row, 'To go ' || p.togo AS col,
            ROUND(100.0 * SUM(p.pass) / SUM(p.plays), 1) AS value,
            ROUND(100.0 * SUM(r.rate * p.plays) / SUM(CASE WHEN r.rate IS NOT NULL THEN p.plays END), 1) AS real, SUM(p.plays) AS n
       FROM p LEFT JOIN r ON r.level = p.level AND r.cell = p.cell WHERE p.down BETWEEN 1 AND 4 GROUP BY p.level, p.down, p.togo ORDER BY grp, p.down, p.togo""", needs=G)

panel("G8.depth", GP, "G8", "Throws by depth: share and completion against real", "bars",
  "Every engine pass attempt by air yards (the real-data buckets: behind the line, 0-9, 10-19, 20 and more). Real: NFL 2021-2025 shares 17.6%, 50.1%, 20.7%, 11.5% and completion 78.8%, 70.9%, 54.5%, 36.0% (match_check's rows).",
  f"""WITH d AS (SELECT level, key, SUM(plays) AS n, SUM(extra) AS cmp FROM {SIT} WHERE key LIKE 'real air yards|%' GROUP BY level, key),
       t AS (SELECT level, SUM(n) AS tot FROM d GROUP BY level),
       r(key, share, cmp) AS (VALUES ('real air yards|-5', 17.6, 78.8), ('real air yards|0', 50.1, 70.9), ('real air yards|10', 20.7, 54.5), ('real air yards|20', 11.5, 36.0))
     SELECT d.level AS grp, replace(replace(replace(replace(d.key, 'real air yards|-5', 'Behind the line'), 'real air yards|0', '0-9 yards'), 'real air yards|10', '10-19 yards'), 'real air yards|20', '20+ yards') || ': share' AS label,
            ROUND(100.0 * d.n / t.tot, 1) AS world, r.share AS real, d.n FROM d JOIN t USING (level) LEFT JOIN r ON r.key = d.key
     UNION ALL
     SELECT d.level, replace(replace(replace(replace(d.key, 'real air yards|-5', 'Behind the line'), 'real air yards|0', '0-9 yards'), 'real air yards|10', '10-19 yards'), 'real air yards|20', '20+ yards') || ': completed',
            ROUND(100.0 * d.cmp / d.n, 1), r.cmp, d.n FROM d LEFT JOIN r ON r.key = d.key ORDER BY 1, 2""", needs=G)

panel("G8.pressure", GP, "G8", "Pressure, blitz, man coverage, play action and time to throw", "bars",
  "From the situation totals: dropbacks under pressure, sacks per pressure, completion when pressured, blitz share, man share, play-action share and its yards a play. Real: NFL figures from engine_targets.json and position_value.json (FTN and nflverse 2021-2025): pressure 28.3%, sacks per pressure 13.9%, completion when pressured 48.9%.",
  f"""WITH s AS (SELECT level, key, SUM(plays) AS n, SUM(yards) AS y, SUM(extra) AS x FROM {SIT} WHERE "group" NOT LIKE 'cross%' AND (key LIKE 'pressured|%' OR key LIKE 'blitz|%|pass' OR key LIKE 'coverage|%|pass' OR key LIKE 'play action|%') GROUP BY level, key),
       p AS (SELECT level, SUM(CASE WHEN key = 'pressured|true' THEN n END) AS pr, SUM(CASE WHEN key LIKE 'pressured|%' THEN n END) AS db,
                    SUM(CASE WHEN key = 'blitz|true|pass' THEN n END) AS bl, SUM(CASE WHEN key LIKE 'blitz|%' THEN n END) AS bt,
                    SUM(CASE WHEN key = 'coverage|man|pass' THEN n END) AS man, SUM(CASE WHEN key LIKE 'coverage|%' THEN n END) AS ct,
                    SUM(CASE WHEN key = 'play action|true' THEN n END) AS pa, SUM(CASE WHEN key LIKE 'play action|%' THEN n END) AS pt,
                    SUM(CASE WHEN key = 'play action|true' THEN y END) AS pay
               FROM s GROUP BY level)
     SELECT level AS grp, 'Dropbacks pressured %' AS label, ROUND(100.0 * pr / db, 1) AS world, 28.3 AS real, db AS n FROM p
     UNION ALL SELECT level, 'Dropbacks blitzed %', ROUND(100.0 * bl / bt, 1), NULL, bt FROM p
     UNION ALL SELECT level, 'Dropbacks against man %', ROUND(100.0 * man / ct, 1), NULL, ct FROM p
     UNION ALL SELECT level, 'Play action %', ROUND(100.0 * pa / pt, 1), NULL, pt FROM p
     UNION ALL SELECT level, 'Play-action yards a play', ROUND(1.0 * pay / pa, 2), NULL, pa FROM p ORDER BY 1, 2""", needs=G)

panel("G8.box", GP, "G8", "Yards a carry by men in the box", "bars",
  "Designed runs by the defenders in the box (situation key box). Real NFL 2021-2025: 6 or fewer 4.66, 7 4.18, 8 or more 3.50 yards a carry (engine_targets.json runs_by_box.all, weighted by runs).",
  f"""WITH b AS (SELECT level, CAST(substr(key, 5, instr(substr(key, 5) || '|', '|') - 1) AS INTEGER) AS box, SUM(plays) AS n, SUM(yards) AS y FROM {SIT}
               WHERE key LIKE 'box|%' AND "group" NOT LIKE 'cross%' GROUP BY level, box)
     SELECT level AS grp, CASE WHEN box <= 6 THEN '6 or fewer' WHEN box = 7 THEN '7' ELSE '8 or more' END AS label, ROUND(1.0 * SUM(y) / SUM(n), 2) AS world,
            CASE WHEN box <= 6 THEN 4.66 WHEN box = 7 THEN 4.18 ELSE 3.50 END AS real, SUM(n) AS n FROM b GROUP BY level, label ORDER BY level, label""", needs=G)

panel("G8.explorer", GP, "G8", "Every situation key: plays, yards, EPA, success", "table",
  "The season's situation totals key by key (the latest season, keys with 20 plays or more, up to the panel's 5,000 rows): plays, yards a play, EPA a play, success rate, first downs, touchdowns, turnovers and the key's extra count. Read each key's family on the families panel above; `afm_read <book> table log_situations` pages through all of them.",
  f"""SELECT season AS year, level, "group", key, plays, ROUND(1.0 * yards / NULLIF(plays, 0), 2) AS yards_a_play, ROUND(epa / NULLIF(plays, 0), 3) AS epa_a_play,
            ROUND(100.0 * successes / NULLIF(plays, 0), 1) AS success_pct, firsts, tds, turnovers, extra
       FROM {SIT} WHERE plays >= 20 AND season = (SELECT MAX(season) FROM {SIT}) ORDER BY level, plays DESC""", needs=G)

check("G8.nfl_pressure", GP, "G8", "NFL dropbacks under pressure", "%",
  f"SELECT 1.0 * SUM(CASE WHEN key = 'pressured|true' THEN plays END) / SUM(plays) FROM {SIT} WHERE level = 'nfl' AND key LIKE 'pressured|%'",
  Fixed(28.3, "engine_targets.json dropbacks.by_pressure: 28.3% of dropbacks pressured (FTN and nflverse 2021-2025; position_value.json's FTN subset reads 29.5%)", "2022-2025"), (0.08, 0.2, 0.0),
  "Dropbacks with the pressured flag over all dropbacks, NFL engine games.", scale=100.0, needs=G)
check("G8.nfl_deep_share", GP, "G8", "NFL throws 20+ air yards", "%",
  f"SELECT 1.0 * SUM(CASE WHEN key = 'real air yards|20' THEN plays END) / SUM(plays) FROM {SIT} WHERE level = 'nfl' AND key LIKE 'real air yards|%'",
  Fixed(11.5, "nflverse 2021-2025 (match_check depth rows)", "2021-2025"), (0.1, 0.25, 0.0),
  "Pass attempts by air yards, NFL engine games.", scale=100.0, needs=G)
check("G8.nfl_box8_ypc", GP, "G8", "NFL yards a carry against 8+ in the box", "yds",
  f"SELECT 1.0 * SUM(yards) / SUM(plays) FROM {SIT} WHERE level = 'nfl' AND key LIKE 'box|%|run' AND CAST(substr(key, 5) AS INTEGER) IN (8, 9)",
  Fixed(3.50, "engine_targets.json runs_by_box.all: 8 and 9 in the box, weighted by runs (nflverse and FTN 2021-2025)", "2021-2025"), (0.08, 0.2, 0.0),
  "Runs against eight or nine in the box, NFL engine games (minor bug 136: it counted the passes against them too).", needs=G)
