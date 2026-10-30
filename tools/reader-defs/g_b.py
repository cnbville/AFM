from common import *

GP = "games"
SIT = "log_situations"
# Pipe-separated key parts as JSON arrays: part(key, 1) is the second field.
def part(e, i): return f"json_extract('[\"' || replace({e}, '|', '\",\"') || '\"]', '$[{i}]')"
# NFL and FBS situation rows with their label.
LV = "CASE WHEN level = 'nfl' THEN 'NFL' ELSE \"group\" END"
NF = "(level = 'nfl' OR (level = 'college' AND \"group\" = 'FBS'))"
def gt(path): return f"json_extract({GT}, '$.{path}')"
# Team games with a box score: level label, season, box row.
BOX = """bx AS (SELECT b.*, g.season AS year, g._row AS gid,
             CASE WHEN g.stream = 'nfl' THEN 'NFL' WHEN g.stream = 'college' THEN json_extract(g.divisions, '$[' || b._i || ']')
                  WHEN g.stream = 'hs' THEN 'High school' ELSE COALESCE(g.league, 'Other') END AS lv
        FROM log_games__box b JOIN log_games g ON g._row = b._parent)"""
col("log_games", "league")

# ---------------------------------------------------------------- G9 fourth downs
panel("G9.go_real", GP, "G9", "Fourth-down go rate by field position and distance against real", "matrix",
  "Share of fourth downs where the offense went for it, by yards from the end zone (rows) and yards to go (columns), in the real data's own buckets. Real, after each cell: the real go rate of each score bucket, weighted by how often the world was in it, so the two compare like for like. NFL: nflverse 2021-2025 (nfl_fourth_go.csv); FBS: cfbfastR 2023-2024 (college_fourth_go.csv). Real go rates rose sharply after 2018, so older seasons should sit lower.",
  f"""WITH k AS (SELECT level, {LV} AS lv, plays, {part("substr(key, 13)", 0)} AS togo, {part("substr(key, 13)", 1)} AS yl,
                    {part("substr(key, 13)", 2)} AS score, {part("substr(key, 13)", 3)} AS choice FROM {SIT} WHERE key LIKE 'real fourth|%' AND {NF}),
       c AS (SELECT level, lv, CAST(togo AS INTEGER) AS togo, CAST(yl AS INTEGER) AS yl, CAST(score AS INTEGER) AS score,
                    SUM(CASE WHEN choice = 'go' THEN plays ELSE 0 END) AS go, SUM(plays) AS n FROM k GROUP BY level, lv, togo, yl, score),
       r AS (SELECT 'nfl' AS level, togo, yardline_100 AS yl, score, rate FROM real_nfl_fourth_go
             UNION ALL SELECT 'college', togo, yardline_100, score, rate FROM real_college_fourth_go)
     SELECT c.lv AS grp, 'Own ' || (100 - c.yl) || ' to ' || (90 - c.yl) AS row, 'To go ' || c.togo AS col,
            ROUND(100.0 * SUM(c.go) / SUM(c.n), 1) AS value,
            ROUND(100.0 * SUM(r.rate * c.n) / SUM(CASE WHEN r.rate IS NOT NULL THEN c.n END), 1) AS real, SUM(c.n) AS n
       FROM c LEFT JOIN r ON r.level = c.level AND r.togo = c.togo AND r.yl = c.yl AND r.score = c.score
      GROUP BY c.lv, c.yl, c.togo ORDER BY c.lv, c.yl DESC, c.togo""",
  says=f"""WITH k AS (SELECT level, plays, {part("substr(key, 13)", 3)} AS choice FROM {SIT} WHERE key LIKE 'real fourth|%' AND level = 'nfl')
     SELECT printf('NFL teams go for it on %.1f%% of fourth downs and FBS teams on %.1f%%; the map shows where.',
       (SELECT 100.0 * SUM(CASE WHEN choice = 'go' THEN plays END) / SUM(plays) FROM k),
       (SELECT 100.0 * SUM(CASE WHEN {part("substr(key, 13)", 3)} = 'go' THEN plays END) / SUM(plays) FROM {SIT} WHERE key LIKE 'real fourth|%' AND level = 'college' AND "group" = 'FBS'))""", needs=G)

ZONES = "(CASE {z} WHEN 'backed up (own 1-5)' THEN 1 WHEN 'own 6-20' THEN 2 WHEN 'own 21-40' THEN 3 WHEN 'midfield' THEN 4 WHEN 'opp 21-40' THEN 5 WHEN 'red zone (6-20)' THEN 6 WHEN 'goal (1-5)' THEN 7 ELSE 8 END)"
TOGO = "(CASE {t} WHEN '1' THEN 1 WHEN '2-3' THEN 2 WHEN '4-6' THEN 3 WHEN '7-10' THEN 4 ELSE 5 END)"
panel("G9.choices", GP, "G9", "What teams do on fourth down, by field zone and distance", "table",
  "Every fourth down by the engine's own field zones and distance: how often teams go, punt or kick, and how often a go converts (a first down or a touchdown). Every season and level in the log.",
  f"""WITH k AS (SELECT level, {LV} AS lv, {part("key", 1)} AS zone, {part("key", 2)} AS togo, {part("key", 3)} AS choice, plays, firsts, tds FROM {SIT}
              WHERE key LIKE 'fourth|%' AND {NF})
     SELECT lv AS grp, zone, togo AS "to go", SUM(plays) AS n,
            ROUND(100.0 * SUM(CASE WHEN choice = 'go' THEN plays END) / SUM(plays), 1) AS "go %",
            ROUND(100.0 * SUM(CASE WHEN choice = 'punt' THEN plays END) / SUM(plays), 1) AS "punt %",
            ROUND(100.0 * SUM(CASE WHEN choice = 'fg' THEN plays END) / SUM(plays), 1) AS "field goal %",
            ROUND(100.0 * SUM(CASE WHEN choice = 'go' THEN firsts + tds END) / SUM(CASE WHEN choice = 'go' THEN plays END), 1) AS "go converted %"
       FROM k GROUP BY lv, zone, togo ORDER BY lv, {ZONES.format(z='zone')}, {TOGO.format(t='togo')}""", needs=G)

panel("G9.conversion", GP, "G9", "How often a fourth-down go converts, by distance", "bars",
  "Goes on fourth down that gained a first down or scored, by yards to go. Real: all distances together, NFL 53.7% (nflverse 2021-2025) and FBS 53.6% (cfbfastR 2022-2024); no real split by distance is on file.",
  f"""WITH k AS (SELECT {LV} AS lv, {part("key", 2)} AS togo, plays, firsts + tds AS conv FROM {SIT} WHERE key LIKE 'fourth|%|go' AND {NF})
     SELECT lv AS grp, 'To go ' || togo AS label, ROUND(100.0 * SUM(conv) / SUM(plays), 1) AS world, NULL AS real, SUM(plays) AS n FROM k GROUP BY lv, togo
     UNION ALL
     SELECT lv, 'All', ROUND(100.0 * SUM(conv) / SUM(plays), 1),
            ROUND(100.0 * CASE lv WHEN 'NFL' THEN json_extract({NGT}, '$.rates.fourth_down_conversion') ELSE json_extract({CGT}, '$.divisions.fbs.rates.fourth_down_conversion') END, 1), SUM(plays)
       FROM k GROUP BY lv""", needs=G)

panel("G9.by_team", GP, "G9", "FBS teams on fourth down", "table",
  "Each FBS program's fourth downs in a season (team situation totals): goes, punts and field goals, its go rate, how its goes converted, and the head coach that season. Sorted by go rate; the spread between teams is the coaches' aggressiveness showing.",
  f"""WITH t AS (SELECT season AS year, team, SUM(CASE WHEN key = 'o|fourth|go' THEN plays END) AS go,
                    SUM(CASE WHEN key = 'o|fourth|punt' THEN plays END) AS punt, SUM(CASE WHEN key = 'o|fourth|fg' THEN plays END) AS fg,
                    SUM(CASE WHEN key = 'o|fourth|go' THEN firsts + tds END) AS conv
               FROM log_team_situations WHERE key LIKE 'o|fourth|%' GROUP BY season, team)
     SELECT t.year, {prog('p')} AS program, t.team AS id, COALESCE(t.go, 0) AS go, COALESCE(t.punt, 0) AS punt, COALESCE(t.fg, 0) AS "field goals",
            ROUND(100.0 * COALESCE(t.go, 0) / (COALESCE(t.go, 0) + COALESCE(t.punt, 0) + COALESCE(t.fg, 0)), 1) AS "go %",
            ROUND(100.0 * t.conv / NULLIF(t.go, 0), 1) AS "converted %",
            (SELECT c.name FROM programs__seasons s JOIN coaches c ON c.id = s.coach WHERE s._parent = p._row AND s.year = t.year) AS coach
       FROM t JOIN programs p ON p._row = t.team ORDER BY t.year DESC, "go %" DESC""", needs=G, open="e/programs/{id}")

check("G9.nfl_conversion", GP, "G9", "NFL fourth-down conversion on goes", "%",
  "SELECT 100.0 * SUM(b.fourth_conv) / SUM(b.fourth_att) FROM log_games__box b JOIN log_games g ON g._row = b._parent WHERE g.stream = 'nfl'",
  Key("nfl_game_targets.json:rates.fourth_down_conversion", 100.0), (0.08, 0.2, 0.0), "Box score fourth-down conversions over tries, every NFL game in the log.", needs=G)
check("G9.nfl_tries", GP, "G9", "NFL fourth-down goes a team game", "a team game",
  "SELECT AVG(COALESCE(b.fourth_att, 0)) FROM log_games__box b JOIN log_games g ON g._row = b._parent WHERE g.stream = 'nfl'",
  Key("nfl_game_targets.json:per_team_game.fourth_go"), (0.15, 0.4, 0.2), "Box score fourth-down tries per NFL team game. Real 2021-2025 is high by history: teams went far more after 2018.", needs=G)
check("G9.fbs_tries", GP, "G9", "FBS fourth-down goes a team game", "a team game",
  f"WITH {BOX} SELECT AVG(COALESCE(fourth_att, 0)) FROM bx WHERE lv = 'FBS'",
  Key("college_game_targets.json:divisions.fbs#per_team_game.fourth_go"), (0.15, 0.4, 0.2), "Box score fourth-down tries per FBS team game (engine games).", needs=G)
check("G9.fbs_conversion", GP, "G9", "FBS fourth-down conversion on goes", "%",
  f"WITH {BOX} SELECT 100.0 * SUM(fourth_conv) / SUM(fourth_att) FROM bx WHERE lv = 'FBS'",
  Key("college_game_targets.json:divisions.fbs#rates.fourth_down_conversion", 100.0), (0.08, 0.2, 0.0), "Box score conversions over tries, FBS team games.", needs=G)
check("G9.nfl_go_rate", GP, "G9", "NFL go rate on fourth down, all fourth downs", "%",
  f"SELECT 100.0 * SUM(CASE WHEN key LIKE '%|go' THEN plays END) / SUM(plays) FROM {SIT} WHERE key LIKE 'fourth|%' AND level = 'nfl'",
  NoReal("the real go rates are kept by bucket only (nfl_fourth_go.csv); the G9 map compares them cell by cell"), (0.1, 0.25, 0.0),
  "Goes over all fourth downs (go, punt, field goal) in the NFL situation totals.", needs=G)

# ---------------------------------------------------------------- G10 kicking
BANDS = [("Under 30", "u30", "under30"), ("30-39", "30s", "30-39"), ("40-49", "40s", "40-49"), ("50+", "50", "50+")]
panel("G10.fg_curve", GP, "G10", "Field goals made by distance against real", "bars",
  "Field goal make rate by distance band, every engine game's box score. Real: NFL 2021-2025 (nflverse) and FBS 2022-2024 (cfbfastR), the same bands.",
  f"WITH {BOX} " + " UNION ALL ".join(
    f"""SELECT lv AS grp, '{lab}' AS label, ROUND(100.0 * SUM(fg_made_{k}) / NULLIF(SUM(fg_att_{k}), 0), 1) AS world,
              ROUND(100.0 * CASE lv WHEN 'NFL' THEN json_extract({NGT}, '$.field_goal_pct_by_distance."{rk}".pct')
                                    WHEN 'FBS' THEN json_extract({CGT}, '$.divisions.fbs.field_goal_pct_by_distance."{rk}".pct') END, 1) AS real,
              SUM(fg_att_{k}) AS n FROM bx GROUP BY lv""" for lab, k, rk in BANDS),
  says=f"""WITH {BOX} SELECT printf('NFL kickers make %.1f%% from 50 and beyond (real %.1f%%) and %.1f%% of extra points (real %.1f%%).',
     (SELECT 100.0 * SUM(fg_made_50) / SUM(fg_att_50) FROM bx WHERE lv = 'NFL'), 100 * json_extract({NGT}, '$.field_goal_pct_by_distance."50+".pct'),
     (SELECT 100.0 * SUM(xp_made) / SUM(xp_att) FROM bx WHERE lv = 'NFL'), 100 * json_extract({NGT}, '$.rates.xp_pct'))""", needs=G)

SPEC = [("Field goals tried", "AVG(COALESCE(fg_att, 0))", "CASE lv WHEN 'NFL' THEN 1.85 END"),
        ("Share of tries from 50+ %", "100.0 * SUM(fg_att_50) / NULLIF(SUM(fg_att), 0)", f"CASE lv WHEN 'NFL' THEN 100.0 * json_extract({NGT}, '$.field_goal_pct_by_distance.\"50+\".attempts') / (SELECT SUM(value ->> 'attempts') FROM json_each({NGT}, '$.field_goal_pct_by_distance')) END"),
        ("Extra point made %", "100.0 * SUM(xp_made) / NULLIF(SUM(xp_att), 0)", f"CASE lv WHEN 'NFL' THEN 100 * json_extract({NGT}, '$.rates.xp_pct') END"),
        ("Two-point tries", "AVG(COALESCE(two_point, 0))", "NULL"),
        ("Two-point good %", "100.0 * SUM(two_point_good) / NULLIF(SUM(two_point), 0)", "NULL"),
        ("Punts", "AVG(COALESCE(punts, 0))", "NULL"),
        ("Punt gross yards", "1.0 * SUM(punt_yds) / NULLIF(SUM(punts), 0)", f"CASE lv WHEN 'NFL' THEN json_extract({NGT}, '$.rates.punt_gross') WHEN 'FBS' THEN json_extract({CGT}, '$.divisions.fbs.rates.punt_gross') END"),
        ("Punt net yards", "1.0 * SUM(punt_net) / NULLIF(SUM(punts), 0)", "NULL"),
        ("Punts inside the 20 %", "100.0 * SUM(punt_in20) / NULLIF(SUM(punts), 0)", f"CASE lv WHEN 'NFL' THEN 100 * json_extract({NGT}, '$.rates.punt_inside20_rate') END"),
        ("Punt touchbacks %", "100.0 * SUM(punt_tb) / NULLIF(SUM(punts), 0)", "NULL"),
        ("Kickoff touchbacks %", "100.0 * SUM(ko_tb) / NULLIF(SUM(kickoffs), 0)", f"CASE lv WHEN 'NFL' THEN 100 * json_extract({NGT}, '$.rates.kickoff_touchback_rate') END"),
        ("Kick returns", "AVG(COALESCE(kr, 0))", "NULL"),
        ("Kick return average", "1.0 * SUM(kr_yds) / NULLIF(SUM(kr), 0)", "CASE lv WHEN 'NFL' THEN 25.9 END"),
        ("Punt returns", "AVG(COALESCE(pr, 0))", "NULL"),
        ("Punt return average", "1.0 * SUM(pr_yds) / NULLIF(SUM(pr), 0)", "CASE lv WHEN 'NFL' THEN 9.5 END"),
        ("Return touchdowns", "AVG(COALESCE(kr_td, 0) + COALESCE(pr_td, 0))", "CASE lv WHEN 'NFL' THEN 0.024 END"),
        ("Kicks blocked", "AVG(COALESCE(kicks_blocked, 0))", "NULL")]
panel("G10.special", GP, "G10", "The kicking game a team game, by level", "table",
  "Kicking and returns from every engine game's box score: tries and make rates, punts, kickoffs and returns, a team game. Real for the NFL: nflverse 2021-2025 (nfl_game_targets.json) and match_check's return rows (kick return 25.9, punt return 9.5, return touchdowns 0.012 a team game each); FBS punt gross from cfbfastR. The 2025 dynamic kickoff changed kickoffs, so the touchback and return lines depend on the season.",
  f"WITH {BOX} SELECT grp, stat, world, real, n FROM (" + " UNION ALL ".join(
    f"SELECT lv AS grp, {i} AS o, '{lab}' AS stat, ROUND({ex}, 2) AS world, ROUND({rex}, 2) AS real, COUNT(*) AS n FROM bx GROUP BY lv" for i, (lab, ex, rex) in enumerate(SPEC))
  + f") ORDER BY {divord('grp')}, o", needs=G)

panel("G10.kickoffs", GP, "G10", "Kickoffs year by year: touchbacks and returns", "line",
  "Kickoff touchback share and kick return average each season, NFL and FBS box scores. Real: NFL touchbacks 54.9% over 2021-2025; the dynamic kickoff (NFL 2025) brought 74.6% of kickoffs back and averaged 25.9 a return.",
  f"""WITH {BOX} SELECT lv || ': touchback %' AS grp, year, ROUND(100.0 * SUM(ko_tb) / NULLIF(SUM(kickoffs), 0), 1) AS world,
            CASE WHEN lv = 'NFL' THEN ROUND(100 * json_extract({NGT}, '$.rates.kickoff_touchback_rate'), 1) END AS real, SUM(kickoffs) AS n
       FROM bx WHERE lv IN ('NFL', 'FBS') GROUP BY lv, year
     UNION ALL
     SELECT lv || ': return average', year, ROUND(1.0 * SUM(kr_yds) / NULLIF(SUM(kr), 0), 1), CASE WHEN lv = 'NFL' THEN 25.9 END, SUM(kr)
       FROM bx WHERE lv IN ('NFL', 'FBS') GROUP BY lv, year ORDER BY 1, 2""", needs=G)

panel("G10.kickers", GP, "G10", "Kickers' seasons from the game log", "table",
  "Each kicker's season from his engine game lines: field goals by band, long, extra points and clutch kicks (tries with the game on the line). Sorted by field goals made.",
  f"""WITH k AS (SELECT g.season AS year, g.stream AS level, p.id, SUM(p.fg_att) AS att, SUM(p.fg_made) AS made, MAX(p.fg_long) AS long,
                    SUM(COALESCE(p.xp_made, 0)) AS xp, SUM(COALESCE(p.xp_att, 0)) AS xp_att, SUM(COALESCE(p.clutch_made, 0)) AS clutch, SUM(COALESCE(p.clutch_att, 0)) AS clutch_att,
                    COUNT(*) AS games
               FROM log_games__players p JOIN log_games g ON g._row = p._parent WHERE p.fg_att > 0 OR p.xp_att > 0 GROUP BY g.season, g.stream, p.id)
     SELECT year, level, id, {pname('k.id')} AS name, games, made || '/' || att AS "field goals", ROUND(100.0 * made / NULLIF(att, 0), 1) AS "made %",
            long, xp || '/' || xp_att AS "extra points", clutch || '/' || clutch_att AS clutch
       FROM k WHERE att >= 5 ORDER BY year DESC, made DESC""", needs=G, open="e/people/{id}")
for c_ in ["fg_long", "clutch_att", "clutch_made", "xp_att", "xp_made"]:
    col("log_games__players", c_)
for c_ in ["two_point", "two_point_good", "kicks_blocked", "punt_tb", "punt_net", "punt_in20", "ko_tb", "kickoffs", "kr_td", "pr_td",
           "fg_att_u30", "fg_made_u30", "fg_att_30s", "fg_made_30s", "fg_att_40s", "fg_made_40s", "fg_att_50", "fg_made_50", "xp_att", "xp_made"]:
    col("log_games__box", c_)

for lab, k, rk in BANDS:
    check(f"G10.nfl_fg_{k}", GP, "G10", f"NFL field goals made from {lab}", "%",
      f"SELECT 100.0 * SUM(b.fg_made_{k}) / SUM(b.fg_att_{k}) FROM log_games__box b JOIN log_games g ON g._row = b._parent WHERE g.stream = 'nfl'",
      Key(f"nfl_game_targets.json:field_goal_pct_by_distance#{rk}.pct", 100.0), (0.04, 0.1, 0.0), f"Box score makes over tries from {lab} yards, every NFL game in the log.", needs=G)
check("G10.nfl_xp", GP, "G10", "NFL extra points made", "%",
  "SELECT 100.0 * SUM(b.xp_made) / SUM(b.xp_att) FROM log_games__box b JOIN log_games g ON g._row = b._parent WHERE g.stream = 'nfl'",
  Key("nfl_game_targets.json:rates.xp_pct", 100.0), (0.02, 0.05, 0.0), "Box score extra points made over tried (the 33-yard kick since 2015).", needs=G)
check("G10.nfl_punt_gross", GP, "G10", "NFL punt gross average", "yards",
  "SELECT 1.0 * SUM(b.punt_yds) / SUM(b.punts) FROM log_games__box b JOIN log_games g ON g._row = b._parent WHERE g.stream = 'nfl'",
  Key("nfl_game_targets.json:rates.punt_gross"), (0.04, 0.1, 0.0), "Punt yards over punts, NFL box scores.", needs=G)
check("G10.nfl_punt_in20", GP, "G10", "NFL punts downed inside the 20", "%",
  "SELECT 100.0 * SUM(b.punt_in20) / SUM(b.punts) FROM log_games__box b JOIN log_games g ON g._row = b._parent WHERE g.stream = 'nfl'",
  Key("nfl_game_targets.json:rates.punt_inside20_rate", 100.0), (0.1, 0.25, 0.0), "Punts inside the 20 over punts, NFL box scores.", needs=G)
check("G10.nfl_ko_touchbacks", GP, "G10", "NFL kickoff touchbacks", "%",
  "SELECT 100.0 * SUM(b.ko_tb) / SUM(b.kickoffs) FROM log_games__box b JOIN log_games g ON g._row = b._parent WHERE g.stream = 'nfl' AND g.season BETWEEN 2016 AND 2023",
  Key("nfl_game_targets.json:rates.kickoff_touchback_rate", 100.0), (0.15, 0.35, 0.0),
  "Touchbacks over kickoffs, NFL box scores of 2016-2023 only (kickoffs from the 35, touchbacks to the 25, before the 2024 and 2025 dynamic kickoff). The real figure averages 2021-2025, so it runs a little under the pre-2024 rate; a world without those seasons in its log has no reading.", needs=G)
check("G10.nfl_fg_tries", GP, "G10", "NFL field goals tried a team game", "a team game",
  "SELECT AVG(COALESCE(b.fg_att, 0)) FROM log_games__box b JOIN log_games g ON g._row = b._parent WHERE g.stream = 'nfl'",
  Fixed(1.85, "match_check.rs FG attempts row (nflverse 2021-2025)", "2021-2025"), (0.12, 0.3, 0.0), "Field goal tries per NFL team game.", needs=G)
check("G10.nfl_kr_avg", GP, "G10", "NFL kick return average", "yards",
  "SELECT 1.0 * SUM(b.kr_yds) / SUM(b.kr) FROM log_games__box b JOIN log_games g ON g._row = b._parent WHERE g.stream = 'nfl'",
  Fixed(25.9, "match_check.rs return rows (NFL 2025, the dynamic kickoff)", "2025"), (0.08, 0.2, 0.0), "Kick return yards over returns, NFL box scores.", needs=G)
check("G10.nfl_pr_avg", GP, "G10", "NFL punt return average", "yards",
  "SELECT 1.0 * SUM(b.pr_yds) / SUM(b.pr) FROM log_games__box b JOIN log_games g ON g._row = b._parent WHERE g.stream = 'nfl'",
  Fixed(9.5, "match_check.rs return rows (nflverse 2021-2025)", "2021-2025"), (0.12, 0.3, 0.0), "Punt return yards over returns, NFL box scores.", needs=G)
check("G10.nfl_return_tds", GP, "G10", "NFL return touchdowns a team game", "a team game",
  "SELECT AVG(COALESCE(b.kr_td, 0) + COALESCE(b.pr_td, 0)) FROM log_games__box b JOIN log_games g ON g._row = b._parent WHERE g.stream = 'nfl'",
  Fixed(0.024, "match_check.rs return rows: 0.012 kick and 0.012 punt return touchdowns a team game", "2021-2025"), (0.3, 0.7, 0.01), "Kick and punt return touchdowns per NFL team game.", needs=G)
check("G10.nfl_pick_sixes", GP, "G10", "NFL pick sixes a team game", "a team game",
  "SELECT 1.0 * SUM(d.result = 'Pick six') / (2.0 * COUNT(DISTINCT g._row)) FROM log_games g JOIN log_games__drives d ON d._parent = g._row WHERE g.stream = 'nfl'",
  Fixed(0.05, "match_check.rs pick six row (nflverse 2021-2025)", "2021-2025"), (0.3, 0.7, 0.01), "Drives that end in an interception returned for a touchdown, per NFL team game.", needs=G)
check("G10.fbs_punt_gross", GP, "G10", "FBS punt gross average", "yards",
  f"WITH {BOX} SELECT 1.0 * SUM(punt_yds) / SUM(punts) FROM bx WHERE lv = 'FBS'",
  Key("college_game_targets.json:divisions.fbs#rates.punt_gross"), (0.04, 0.1, 0.0), "Punt yards over punts, FBS box scores.", needs=G)
check("G10.fbs_fg_50", GP, "G10", "FBS field goals made from 50+", "%",
  f"WITH {BOX} SELECT 100.0 * SUM(fg_made_50) / SUM(fg_att_50) FROM bx WHERE lv = 'FBS'",
  Key("college_game_targets.json:divisions.fbs#field_goal_pct_by_distance.50+.pct", 100.0), (0.08, 0.2, 0.0), "Box score makes over tries from 50 yards and beyond, FBS.", needs=G)

# ---------------------------------------------------------------- G11 penalties
PEN = [("false start", "False Start"), ("offensive holding", "Offensive Holding"), ("pass interference", "Defensive Pass Interference"),
       ("defensive holding", "Defensive Holding"), ("offside", "Defensive Offside"), ("unnecessary roughness", "Unnecessary Roughness"),
       ("delay of game", "Delay of Game"), ("illegal formation", "Illegal Formation"), ("illegal block in the back", "Illegal Block Above the Waist"),
       ("roughing the passer", "Roughing the Passer"), ("face mask", "Face Mask"), ("offensive pass interference", "Offensive Pass Interference"),
       ("unsportsmanlike conduct", "Unsportsmanlike Conduct"), ("intentional grounding", "Intentional Grounding"), ("ineligible man downfield", "Ineligible Downfield Pass")]
PENMAP = "pm(kind, real) AS (VALUES " + ", ".join(f"('{a}', '{b}')" for a, b in PEN) + ")"
REALPEN = f"""rp AS (SELECT pm.kind, AVG(json_extract(y.value, '$."' || pm.real || '"')) AS real FROM pm, json_each({NH}, '$.penalty_kinds_per_team_game.by_season') y
                WHERE CAST(y.key AS INTEGER) BETWEEN 2021 AND 2025 GROUP BY pm.kind)"""
panel("G11.by_kind", GP, "G11", "Accepted fouls by kind a team game against real", "bars",
  "Accepted penalties by kind per team game, from every logged game's penalty list. Real: the NFL's accepted penalties by kind, nflverse play-by-play averaged over 2021-2025 (nfl_history.json; offside here is the real defensive offside alone, illegal block is the real illegal block above the waist). No real college split by kind is on file.",
  f"""WITH {PENMAP}, {REALPEN},
       games AS (SELECT CASE WHEN stream = 'nfl' THEN 'NFL' WHEN stream = 'college' THEN 'College' WHEN stream = 'hs' THEN 'High school' ELSE COALESCE(league, 'Other') END AS lv, COUNT(*) AS n
                   FROM log_games g WHERE {ENGINE} GROUP BY 1),
       p AS (SELECT CASE WHEN g.stream = 'nfl' THEN 'NFL' WHEN g.stream = 'college' THEN 'College' WHEN g.stream = 'hs' THEN 'High school' ELSE COALESCE(g.league, 'Other') END AS lv,
                    p.kind, COUNT(*) AS n FROM log_games__penalties p JOIN log_games g ON g._row = p._parent WHERE COALESCE(p.accepted, 0) = 1 GROUP BY 1, 2)
     SELECT p.lv AS grp, p.kind AS label, ROUND(1.0 * p.n / (2 * games.n), 3) AS world, CASE WHEN p.lv = 'NFL' THEN ROUND(rp.real, 3) END AS real, p.n
       FROM p JOIN games USING (lv) LEFT JOIN rp ON rp.kind = p.kind ORDER BY grp, world DESC""",
  says=f"""WITH {PENMAP}, {REALPEN}, p AS (SELECT p.kind, COUNT(*) AS n FROM log_games__penalties p JOIN log_games g ON g._row = p._parent WHERE g.stream = 'nfl' AND COALESCE(p.accepted, 0) = 1 GROUP BY p.kind),
       n AS (SELECT 2.0 * COUNT(*) AS tg FROM log_games WHERE stream = 'nfl')
     SELECT printf('NFL offensive holding is called %.2f times a team game (real %.2f) and false starts %.2f (real %.2f).',
       (SELECT n FROM p WHERE kind = 'offensive holding') / (SELECT tg FROM n), (SELECT real FROM rp WHERE kind = 'offensive holding'),
       (SELECT n FROM p WHERE kind = 'false start') / (SELECT tg FROM n), (SELECT real FROM rp WHERE kind = 'false start'))""", needs=G)

panel("G11.by_year", GP, "G11", "Penalties and yards a team game, year by year", "line",
  "Accepted penalties and their yards per team game each season, from box scores. Real: NFL 2021-2025 averages 6.0 penalties and 49.4 yards; FBS has 5.4 accepted penalties on plays by either side a team game (cfbfastR 2022-2024, not quite the same count).",
  f"""WITH {BOX} SELECT lv || ': penalties' AS grp, year, ROUND(AVG(COALESCE(penalties, 0)), 2) AS world,
            CASE lv WHEN 'NFL' THEN json_extract({NGT}, '$.per_team_game.penalties') WHEN 'FBS' THEN json_extract({CGT}, '$.divisions.fbs.per_team_game.penalties_on_plays_by_either') END AS real, COUNT(*) AS n
       FROM bx GROUP BY lv, year
     UNION ALL SELECT lv || ': yards', year, ROUND(AVG(COALESCE(penalty_yds, 0)), 1), CASE lv WHEN 'NFL' THEN json_extract({NGT}, '$.per_team_game.penalty_yards') END, COUNT(*)
       FROM bx GROUP BY lv, year ORDER BY 1, 2""", needs=G)

panel("G11.site", GP, "G11", "Declined fouls and the road team's pre-snap fouls", "table",
  "Each foul kind in the situation totals: how many were called, the share declined, and the share called on the visiting team (crowd noise should push road false starts and delays up). NFL and FBS.",
  f"""WITH a AS (SELECT {LV} AS lv, {part("key", 1)} AS kind, SUM(CASE WHEN key LIKE '%|accepted' THEN plays END) AS acc, SUM(CASE WHEN key LIKE '%|declined' THEN plays END) AS dec
               FROM {SIT} WHERE key LIKE 'penalty|%' AND {NF} GROUP BY 1, 2),
       s AS (SELECT {LV} AS lv, {part("key", 1)} AS kind, SUM(CASE WHEN key LIKE '%|away' THEN plays END) AS away, SUM(plays) AS n
               FROM {SIT} WHERE key LIKE 'penalty site|%' AND {NF} GROUP BY 1, 2)
     SELECT a.lv AS grp, a.kind, COALESCE(a.acc, 0) + COALESCE(a.dec, 0) AS called, ROUND(100.0 * COALESCE(a.dec, 0) / (COALESCE(a.acc, 0) + COALESCE(a.dec, 0)), 1) AS "declined %",
            ROUND(100.0 * s.away / s.n, 1) AS "on the road team %"
       FROM a LEFT JOIN s USING (lv, kind) ORDER BY grp, called DESC""", needs=G)

panel("G11.flags", GP, "G11", "Flags a game: the crew's draw", "hist",
  "Accepted penalties on both teams in each game. Crews differ (each crew leans on holding, interference, personal fouls and pre-snap fouls in its own way), so the spread should be wider than chance alone.",
  f"""WITH f AS (SELECT CASE WHEN g.stream = 'nfl' THEN 'NFL' WHEN g.stream = 'college' THEN 'College' WHEN g.stream = 'hs' THEN 'High school' ELSE COALESCE(g.league, 'Other') END AS grp,
                    (SELECT COUNT(*) FROM log_games__penalties p WHERE p._parent = g._row AND COALESCE(p.accepted, 0) = 1) AS x
               FROM log_games g WHERE {ENGINE} AND EXISTS (SELECT 1 FROM log_games__box b WHERE b._parent = g._row))
     SELECT grp, x, 100.0 * COUNT(*) / SUM(COUNT(*)) OVER (PARTITION BY grp) AS world, COUNT(*) AS n FROM f GROUP BY grp, x ORDER BY grp, x""", needs=G)

for kind, real_, title in [("offensive holding", 1.26, "offensive holding"), ("false start", 1.1, "false starts"), ("pass interference", 0.42, "defensive pass interference")]:
    k = kind.replace(" ", "_")
    check(f"G11.nfl_{k}", GP, "G11", f"NFL {title} a team game", "a team game",
      f"""SELECT 1.0 * (SELECT COUNT(*) FROM log_games__penalties p JOIN log_games g ON g._row = p._parent WHERE g.stream = 'nfl' AND COALESCE(p.accepted, 0) = 1 AND p.kind = '{kind}')
                / (2 * (SELECT COUNT(*) FROM log_games WHERE stream = 'nfl'))""",
      Fixed(real_, "match_check.rs foul rows (Claude's reading of nflpenalties.com 2023; diagnosis only)", "2023"), (0.2, 0.45, 0.05),
      f"Accepted {title} over NFL team games.", needs=G)
check("G11.nfl_penalty_yards", GP, "G11", "NFL penalty yards a team game", "yards",
  "SELECT AVG(COALESCE(b.penalty_yds, 0)) FROM log_games__box b JOIN log_games g ON g._row = b._parent WHERE g.stream = 'nfl'",
  Key("nfl_game_targets.json:per_team_game.penalty_yards"), (0.12, 0.3, 0.0), "Accepted penalty yards per NFL team game, box scores.", needs=G)

# ---------------------------------------------------------------- G12 officiating
panel("G12.reviews", GP, "G12", "Replay and close calls, by level and year", "table",
  "Each season's officiating from the game log: close calls, wrong rulings, coach's challenges and how many were won, booth reviews and replay assists, calls overturned and wrong calls left standing, a game. Real NFL 2024: 133 challenges in 285 games (50 won), booth reviews about 0.7 a game with a little over half reversed (docs/match-engine.md).",
  f"""SELECT CASE WHEN stream = 'nfl' THEN 'NFL' WHEN stream = 'college' THEN 'College' WHEN stream = 'hs' THEN 'High school' ELSE COALESCE(league, 'Other') END AS grp,
            season AS year, COUNT(*) AS games,
            ROUND(AVG(COALESCE(officiating__close_calls, 0)), 2) AS "close calls", ROUND(AVG(COALESCE(officiating__wrong, 0)), 2) AS "wrong rulings",
            ROUND(1.0 * SUM((SELECT COUNT(*) FROM log_games__officiating__calls c WHERE c._parent = g._row AND c.reviewer LIKE 'Challenge%')) / COUNT(*), 2) AS challenges,
            ROUND(100.0 * SUM((SELECT COUNT(*) FROM log_games__officiating__calls c WHERE c._parent = g._row AND c.reviewer LIKE 'Challenge%' AND c.overturned))
                  / NULLIF(SUM((SELECT COUNT(*) FROM log_games__officiating__calls c WHERE c._parent = g._row AND c.reviewer LIKE 'Challenge%')), 0), 1) AS "challenges won %",
            ROUND(AVG(COALESCE(officiating__booth_reviews, 0)), 2) AS "booth reviews", ROUND(AVG(COALESCE(officiating__assists, 0)), 2) AS assists,
            ROUND(AVG(COALESCE(officiating__overturned, 0)), 2) AS overturned, ROUND(AVG(COALESCE(officiating__wrong_left_standing, 0)), 2) AS "wrong left standing"
       FROM log_games g WHERE {ENGINE} GROUP BY 1, 2 ORDER BY 1, 2""",
  says="""SELECT printf('NFL games average %.2f challenges (real about 0.47) and %.2f booth reviews (real about 0.7); %.2f wrong calls a game are left standing.',
     1.0 * (SELECT COUNT(*) FROM log_games__officiating__calls c JOIN log_games g ON g._row = c._parent WHERE g.stream = 'nfl' AND c.reviewer LIKE 'Challenge%') / COUNT(*), AVG(COALESCE(officiating__booth_reviews, 0)), AVG(COALESCE(officiating__wrong_left_standing, 0))) FROM log_games WHERE stream = 'nfl'""", needs=G)

panel("G12.calls", GP, "G12", "Close calls by kind: ruled right, wrong, reviewed, reversed", "table",
  "Every close call in the log by kind (catch, spot, goal line, fumble, interception) and who looked at it again (the booth, a coach's challenge, a replay assist, or nobody): how many were wrong on the field and how many were overturned.",
  """SELECT CASE WHEN g.stream = 'nfl' THEN 'NFL' WHEN g.stream = 'college' THEN 'College' ELSE g.stream END AS grp, c.call AS kind,
            CASE WHEN c.reviewer LIKE 'Challenge%' THEN 'Challenge' ELSE c.reviewer END AS reviewer, COUNT(*) AS calls,
            ROUND(100.0 * SUM(COALESCE(c.wrong, 0)) / COUNT(*), 1) AS "wrong on the field %", ROUND(100.0 * SUM(COALESCE(c.overturned, 0)) / COUNT(*), 1) AS "overturned %"
       FROM log_games__officiating__calls c JOIN log_games g ON g._row = c._parent GROUP BY 1, 2, 3 ORDER BY 1, calls DESC""", needs=G)

panel("G12.toss", GP, "G12", "The coin toss: what the winner chose", "bars",
  "Opening coin toss choices by level. Real: NFL toss winners defer about 82% of the time (docs/match-engine.md).",
  """SELECT CASE WHEN stream = 'nfl' THEN 'NFL' WHEN stream = 'college' THEN 'College' WHEN stream = 'hs' THEN 'High school' ELSE COALESCE(league, 'Other') END AS grp,
            toss__choice AS label, ROUND(100.0 * COUNT(*) / SUM(COUNT(*)) OVER (PARTITION BY stream, league), 1) AS world,
            CASE WHEN stream = 'nfl' AND toss__choice = 'Defer' THEN 82.0 END AS real, COUNT(*) AS n
       FROM log_games WHERE toss__choice IS NOT NULL GROUP BY stream, league, toss__choice ORDER BY grp, world DESC""", needs=G)

panel("G12.notes", GP, "G12", "Reviewed calls, the latest season", "table",
  "Each close call that was reviewed or ruled wrong in the latest season, with the officials' own note; open the game to see the play.",
  """SELECT g.season AS year, g._row AS game, c.call AS kind, c.reviewer, CASE WHEN c.overturned THEN 'overturned' WHEN c.wrong THEN 'wrong, stood' ELSE 'confirmed' END AS outcome, c.note
       FROM log_games__officiating__calls c JOIN log_games g ON g._row = c._parent
      WHERE g.season = (SELECT MAX(season) FROM log_games) AND (c.reviewer <> 'None' OR COALESCE(c.wrong, 0) = 1) ORDER BY g._row, c._i""", needs=G, open="game/{game}")

check("G12.nfl_challenges", GP, "G12", "NFL coach's challenges a game", "a game",
  "SELECT 1.0 * (SELECT COUNT(*) FROM log_games__officiating__calls c JOIN log_games g ON g._row = c._parent WHERE g.stream = 'nfl' AND COALESCE(g.kind, 'regular') <> 'preseason' AND c.reviewer LIKE 'Challenge%') / (SELECT COUNT(*) FROM log_games WHERE stream = 'nfl' AND COALESCE(kind, 'regular') <> 'preseason')",
  Fixed(0.467, "NFL 2024: 133 coach's challenges in 285 games (docs/match-engine.md, 'Live games, officials and the toss')", "2024"), (0.2, 0.5, 0.05),
  "Challenges per NFL game, both teams, counted from the game's list of close calls.", needs=G)
check("G12.nfl_challenges_won", GP, "G12", "NFL challenges won", "%",
  "SELECT 100.0 * SUM(COALESCE(c.overturned, 0)) / COUNT(*) FROM log_games__officiating__calls c JOIN log_games g ON g._row = c._parent WHERE g.stream = 'nfl' AND COALESCE(g.kind, 'regular') <> 'preseason' AND c.reviewer LIKE 'Challenge%'",
  Fixed(37.6, "NFL 2024: 50 of 133 challenges won (docs/match-engine.md)", "2024"), (0.15, 0.4, 0.0), "Challenges that overturned the call.", needs=G)
check("G12.nfl_booth", GP, "G12", "NFL booth reviews a game", "a game",
  "SELECT AVG(COALESCE(officiating__booth_reviews, 0)) FROM log_games WHERE stream = 'nfl'",
  Fixed(0.7, "NFL booth reviews about 0.7 a game (docs/match-engine.md)", "2024"), (0.2, 0.5, 0.05), "Booth reviews per NFL game.", needs=G)
check("G12.nfl_defer", GP, "G12", "NFL toss winners who defer", "%",
  "SELECT 100.0 * SUM(toss__choice = 'Defer') / COUNT(*) FROM log_games WHERE stream = 'nfl' AND toss__choice IS NOT NULL",
  Fixed(82.0, "NFL toss winners defer about 82% (docs/match-engine.md)", "2018-2024"), (0.06, 0.15, 0.0), "Toss winners choosing to defer.", needs=G)

# ---------------------------------------------------------------- G13 garbage time and the clock
Q4B = ["lead 28+", "lead 21-27", "lead 15-20", "lead 1-14", "trail 1-14", "trail 15-20", "trail 21-27", "trail 28+"]
panel("G13.q4_lead", GP, "G13", "Fourth-quarter pass rate by the score after three quarters", "bars",
  "Share of fourth-quarter plays that were passes, by the team's lead or deficit after three quarters, in the real data's own buckets. Real: FBS against FBS 2022-2024 (cfbfastR) and NFL regular seasons (nflverse), garbage_time.json. Teams far ahead run the clock; teams far behind throw.",
  f"""WITH k AS (SELECT {LV} AS lv, {part("key", 1)} AS b, {part("key", 2)} AS kind, plays FROM {SIT} WHERE key LIKE 'q4|%' AND {NF}),
       o(b, o) AS (VALUES {", ".join(f"('{b}', {i})" for i, b in enumerate(Q4B))})
     SELECT k.lv AS grp, k.b AS label, ROUND(100.0 * SUM(CASE WHEN kind = 'pass' THEN plays END) / SUM(plays), 1) AS world,
            ROUND(100.0 * json_extract({GT}, '$.' || CASE k.lv WHEN 'NFL' THEN 'nfl' ELSE 'fbs' END || '.' || substr(k.b, 1, instr(k.b, ' ') - 1) || '."' || substr(k.b, instr(k.b, ' ') + 1) || '".q4_pass_rate'), 1) AS real,
            SUM(plays) AS n
       FROM k JOIN o ON o.b = k.b WHERE kind IN ('pass', 'run') GROUP BY k.lv, k.b ORDER BY k.lv, o.o""",
  says=f"""SELECT printf('FBS teams up 28 or more after three quarters throw on %.1f%% of fourth-quarter plays (real %.1f%%); down 28 or more, %.1f%% (real %.1f%%).',
     (SELECT 100.0 * SUM(CASE WHEN key = 'q4|lead 28+|pass' THEN plays END) / SUM(plays) FROM {SIT} WHERE key IN ('q4|lead 28+|pass', 'q4|lead 28+|run') AND "group" = 'FBS'),
     100 * {gt('fbs.lead."28+".q4_pass_rate')},
     (SELECT 100.0 * SUM(CASE WHEN key = 'q4|trail 28+|pass' THEN plays END) / SUM(plays) FROM {SIT} WHERE key IN ('q4|trail 28+|pass', 'q4|trail 28+|run') AND "group" = 'FBS'),
     100 * {gt('fbs.trail."28+".q4_pass_rate')})""", needs=G)

panel("G13.two_minute", GP, "G13", "Two-minute drill: pass rate by score", "bars",
  "Share of plays that were passes in the last two minutes of a half, by the offense's score state. No real split is on file; the expectation is clear (behind throws nearly always, ahead runs).",
  f"""WITH k AS (SELECT {LV} AS lv, {part("key", 1)} AS b, {part("key", 2)} AS kind, plays FROM {SIT} WHERE key LIKE 'two minute|%' AND {NF})
     SELECT lv AS grp, b AS label, ROUND(100.0 * SUM(CASE WHEN kind = 'pass' THEN plays END) / SUM(plays), 1) AS world, NULL AS real, SUM(plays) AS n
       FROM k WHERE kind IN ('pass', 'run') GROUP BY lv, b ORDER BY lv, world""", needs=G)

panel("G13.clock", GP, "G13", "Clock habits: kneels, spikes and plays out of bounds", "table",
  "Each level's clock plays from the situation totals: kneels and spikes, and the share of passes and runs that ended out of bounds. Real NFL: 10.7% of plays end out of bounds (completions 20.7%, designed runs 5.5%), match_check's rows.",
  f"""WITH k AS (SELECT {LV} AS lv, season, key, plays FROM {SIT} WHERE {NF} AND (key LIKE 'kind|%' OR key LIKE 'out of bounds|%')),
       tg AS (SELECT CASE WHEN g.stream = 'nfl' THEN 'NFL' ELSE json_extract(g.divisions, '$[0]') END AS lv, g.season, 2 * COUNT(*) AS n FROM log_games g
               WHERE g.stream IN ('nfl', 'college') AND {ENGINE} GROUP BY 1, 2)
     SELECT k.lv AS grp, k.season AS year, MAX(tg.n) AS "team games",
            ROUND(1.0 * SUM(CASE WHEN key = 'kind|kneel' THEN plays END) / MAX(tg.n), 2) AS "kneels a team game",
            ROUND(1.0 * SUM(CASE WHEN key = 'kind|spike' THEN plays END) / MAX(tg.n), 2) AS "spikes a team game",
            ROUND(100.0 * SUM(CASE WHEN key = 'out of bounds|pass' THEN plays END) / SUM(CASE WHEN key = 'kind|pass' THEN plays END), 1) AS "passes out of bounds %",
            ROUND(100.0 * SUM(CASE WHEN key = 'out of bounds|run' THEN plays END) / SUM(CASE WHEN key = 'kind|run' THEN plays END), 1) AS "runs out of bounds %",
            ROUND(100.0 * SUM(CASE WHEN key LIKE 'out of bounds|%' THEN plays END) / SUM(CASE WHEN key IN ('kind|pass', 'kind|run') THEN plays END), 1) AS "all out of bounds %"
       FROM k LEFT JOIN tg ON tg.lv = k.lv AND tg.season = k.season GROUP BY k.lv, k.season ORDER BY 1, 2""", needs=G)

panel("G13.bench", GP, "G13", "When coaches pull their starters", "table",
  "Every bench call in the latest season's logged games, in the coach's own words: when he pulled his quarterback or emptied the bench, the score, and how he read the chance of a comeback. Real: FBS coaches pull a leading quarterback at a median lead of 29 points (cfbfastR 2022-2024); NFL at 25 (garbage_time.json pull_timing).",
  """SELECT g.season AS year, g._row AS game, d.what AS "the call" FROM log_games__decisions d JOIN log_games g ON g._row = d._parent
      WHERE g.season = (SELECT MAX(season) FROM log_games) ORDER BY g._row, d._i""", needs=G, open="game/{game}")

check("G13.fbs_q4_lead28_pass", GP, "G13", "FBS fourth-quarter pass rate when up 28+ after three quarters", "%",
  f"SELECT 100.0 * SUM(CASE WHEN key = 'q4|lead 28+|pass' THEN plays END) / SUM(plays) FROM {SIT} WHERE key IN ('q4|lead 28+|pass', 'q4|lead 28+|run') AND \"group\" = 'FBS'",
  Key("garbage_time.json:fbs.lead#28+.q4_pass_rate", 100.0), (0.12, 0.3, 0.0), "Fourth-quarter passes over passes and runs, FBS teams leading by 28 or more after three quarters.", needs=G)
check("G13.fbs_q4_trail28_pass", GP, "G13", "FBS fourth-quarter pass rate when down 28+ after three quarters", "%",
  f"SELECT 100.0 * SUM(CASE WHEN key = 'q4|trail 28+|pass' THEN plays END) / SUM(plays) FROM {SIT} WHERE key IN ('q4|trail 28+|pass', 'q4|trail 28+|run') AND \"group\" = 'FBS'",
  Key("garbage_time.json:fbs.trail#28+.q4_pass_rate", 100.0), (0.12, 0.3, 0.0), "The same, trailing by 28 or more.", needs=G)
check("G13.nfl_q4_close_pass", GP, "G13", "NFL fourth-quarter pass rate, leading by 1-14", "%",
  f"SELECT 100.0 * SUM(CASE WHEN key = 'q4|lead 1-14|pass' THEN plays END) / SUM(plays) FROM {SIT} WHERE key IN ('q4|lead 1-14|pass', 'q4|lead 1-14|run') AND level = 'nfl'",
  Key("garbage_time.json:nfl.lead#1-14.q4_pass_rate", 100.0), (0.1, 0.25, 0.0), "Fourth-quarter passes over passes and runs, NFL teams ahead by 1-14 after three quarters.", needs=G)
check("G13.nfl_out_of_bounds", GP, "G13", "NFL plays ending out of bounds", "%",
  f"SELECT 100.0 * SUM(CASE WHEN key LIKE 'out of bounds|%' THEN plays END) / SUM(CASE WHEN key IN ('kind|pass', 'kind|run') THEN plays END) FROM {SIT} WHERE level = 'nfl'",
  Fixed(10.7, "match_check.rs out-of-bounds row (nflverse 2021-2025)", "2021-2025"), (0.15, 0.35, 0.0), "Passes and runs that ended out of bounds over all passes and runs.", needs=G)
