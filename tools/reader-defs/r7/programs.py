from common import *

def P(id, area, title, kind, note, sql, says=None, needs=W, open="", page="programs"):
    return dict(id=id, page=page, area=area, title=title, kind=kind, note=note, sql=sql, says=says, needs=needs, open=open)

PANELS = [
# ---------- C1 identity ----------
P("C1.levels_by_year", "C1", "Programs at each level, season by season", "line",
  "How many programs fielded a team at each level each season (their program seasons), against the real count from the program histories (`college_history.csv`, 1984-2025). Pick a level above the chart.",
  PS + """ SELECT ps.year, ps.division AS grp, COUNT(*) AS world,
        (SELECT COUNT(*) FROM real_college_history r WHERE ps.year BETWEEN r.first_season AND r.last_season AND r.division = ps.division) AS real
   FROM ps GROUP BY ps.year, ps.division ORDER BY ps.division, ps.year""",
  says=PS + """ SELECT printf('%d programs played in %d: %s.', COUNT(*), year, group_concat(n || ' ' || division, ', ')) FROM (SELECT year, division, COUNT(*) AS n FROM ps WHERE year = (SELECT MAX(year) FROM ps) GROUP BY division ORDER BY n DESC)"""),
P("C1.today", "C1", "Programs at each level now, against the real 2026 count", "bars",
  "Programs with a team in the world's last season by level, against the real programs of 2026 (`colleges.csv` and `colleges_lower.csv`).",
  """SELECT p.division AS label, COUNT(*) AS world,
        (SELECT COUNT(*) FROM real_colleges r WHERE r.division = p.division) + (SELECT COUNT(*) FROM real_colleges_lower r WHERE r.division = p.division) AS real
   FROM programs p WHERE p._row IN (SELECT _parent FROM programs__seasons WHERE year = (SELECT MAX(year) FROM programs__seasons))
  GROUP BY p.division ORDER BY world DESC"""),
P("C1.list", "C1", "Every program", "table",
  "Each program's name, level, conference, home, enrollment, academic tier and stadium seats, with the real stadium where `colleges.csv` has it, and its prestige now. A row opens its page.",
  """SELECT p.id, COALESCE(p.real_name, p.fictional_name) AS name, p.division, p.conference, sc.code AS state, p.city, p.enrollment, p.academic_tier AS academics,
        p.stadium, COALESCE(r.stadium_capacity, rl.stadium_capacity) AS real_stadium, ROUND(p.prestige, 3) AS prestige
   FROM programs p LEFT JOIN state_codes sc ON sc.idx = p.state
   LEFT JOIN real_colleges r ON r.name = p.real_name LEFT JOIN real_colleges_lower rl ON rl.name = p.real_name
  ORDER BY CASE p.division WHEN 'FBS' THEN 0 WHEN 'FCS' THEN 1 WHEN 'D2' THEN 2 WHEN 'NAIA' THEN 3 WHEN 'D3' THEN 4 ELSE 5 END, p.prestige DESC""",
  open="e/programs/{id}"),
P("C1.stadium_gap", "C1", "Stadiums that differ from the real capacity", "table",
  "Programs whose stadium seats differ from the real capacity (`colleges.csv`) by more than 5%.",
  """SELECT p.id, COALESCE(p.real_name, p.fictional_name) AS name, p.division, p.stadium, COALESCE(r.stadium_capacity, rl.stadium_capacity) AS real_stadium,
        ROUND(100.0 * (p.stadium - COALESCE(r.stadium_capacity, rl.stadium_capacity)) / COALESCE(r.stadium_capacity, rl.stadium_capacity), 1) AS gap_pct
   FROM programs p LEFT JOIN real_colleges r ON r.name = p.real_name LEFT JOIN real_colleges_lower rl ON rl.name = p.real_name
  WHERE COALESCE(r.stadium_capacity, rl.stadium_capacity) > 0 AND ABS(p.stadium - COALESCE(r.stadium_capacity, rl.stadium_capacity)) > 0.05 * COALESCE(r.stadium_capacity, rl.stadium_capacity)
  ORDER BY ABS(gap_pct) DESC""", open="e/programs/{id}"),
P("C1.story", "C1", "Each program's story lines", "table",
  "Every dated line of each program's story (`Program.story`): facility projects, boosters, hires and moves, newest first. A row opens the program.",
  """SELECT st._0 AS year, p.id, COALESCE(p.real_name, p.fictional_name) AS program, p.division AS now, st._1 AS story
   FROM programs__story st JOIN programs p ON p._row = st._parent ORDER BY st._0 DESC, program""", open="e/programs/{id}"),
# ---------- C2 prestige ----------
P("C2.top", "C2", "The most prestigious programs", "table",
  "The top 40 programs by prestige now, with the parts it is built from (tradition, recent success, facilities, fans; 0-1), where it started, and how it ranks within its level (`prestige_rel`). Prestige = 0.15 tradition + 0.3 recent + 0.12 coach + 0.12 facilities + 0.11 boosters + 0.12 fans + 0.08 conference (`program.rs`, `prestige_of`).",
  """SELECT p.id, COALESCE(p.real_name, p.fictional_name) AS name, p.division, p.conference, ROUND(p.prestige, 3) AS prestige, ROUND(p.prestige_start, 3) AS at_start,
        ROUND(p.prestige - p.prestige_start, 3) AS change, ROUND(p.tradition, 2) AS tradition, ROUND(p.recent, 2) AS recent, ROUND(p.facilities, 2) AS facilities,
        ROUND(p.fans, 2) AS fans, ROUND(p.prestige_rel, 2) AS in_level
   FROM programs p ORDER BY p.prestige DESC LIMIT 40""", open="e/programs/{id}",
  says="""SELECT printf('%s leads on prestige (%.2f); the top 10 are %s.', (SELECT COALESCE(real_name, fictional_name) FROM programs ORDER BY prestige DESC LIMIT 1), (SELECT MAX(prestige) FROM programs),
        (SELECT group_concat(n, ', ') FROM (SELECT COALESCE(real_name, fictional_name) AS n FROM programs ORDER BY prestige DESC LIMIT 10)))"""),
P("C2.risers", "C2", "Biggest risers and fallers since the history began", "table",
  "The 15 programs whose prestige rose most and the 15 that fell most from the start of the history to now, each with its latest story line (the why the world wrote).",
  """SELECT * FROM (SELECT p.id, COALESCE(p.real_name, p.fictional_name) AS name, p.division, ROUND(p.prestige_start, 3) AS at_start, ROUND(p.prestige, 3) AS now,
        ROUND(p.prestige - p.prestige_start, 3) AS change, (SELECT _0 || ': ' || _1 FROM programs__story WHERE _parent = +p._row ORDER BY _0 DESC LIMIT 1) AS last_story
     FROM programs p ORDER BY change DESC LIMIT 15)
  UNION ALL SELECT * FROM (SELECT p.id, COALESCE(p.real_name, p.fictional_name), p.division, ROUND(p.prestige_start, 3), ROUND(p.prestige, 3),
        ROUND(p.prestige - p.prestige_start, 3) AS change, (SELECT _0 || ': ' || _1 FROM programs__story WHERE _parent = +p._row ORDER BY _0 DESC LIMIT 1)
     FROM programs p ORDER BY change ASC LIMIT 15)""", open="e/programs/{id}"),
P("C2.spread", "C2", "The spread of prestige within each level, year by year", "line",
  "The 10th, 50th and 90th percentile of prestige each year (`prestige_history`), by today's level: does the spread hold, close up or blow apart? Pick a level above the chart.",
  """WITH x AS (SELECT h._0 AS year, p.division AS grp, h._1 AS v, ROW_NUMBER() OVER (PARTITION BY h._0, p.division ORDER BY h._1) AS r,
        COUNT(*) OVER (PARTITION BY h._0, p.division) AS n FROM programs__prestige_history h JOIN programs p ON p._row = h._parent)
  SELECT year, grp, """ + PCT(0.1) + """ AS p10, """ + PCT(0.5) + """ AS p50, """ + PCT(0.9) + """ AS p90 FROM x GROUP BY year, grp ORDER BY grp, year"""),
P("C2.vs_wins", "C2", "Prestige against the last five seasons' win rate", "bars",
  "The correlation between each program's prestige now and its win rate over the last five seasons, by level. No real figure: prestige has no public measure.",
  """WITH w AS (SELECT s._parent AS prog, SUM(s.wins) * 1.0 / NULLIF(SUM(s.wins + s.losses), 0) AS wp FROM programs__seasons s
        WHERE s.year > (SELECT MAX(year) FROM programs__seasons) - 5 GROUP BY s._parent),
   x AS (SELECT p.division AS d, p.prestige AS a, w.wp AS b FROM programs p JOIN w ON w.prog = p._row WHERE w.wp IS NOT NULL)
  SELECT d AS label, ROUND((COUNT(*) * SUM(a * b) - SUM(a) * SUM(b)) / sqrt((COUNT(*) * SUM(a * a) - SUM(a) * SUM(a)) * (COUNT(*) * SUM(b * b) - SUM(b) * SUM(b))), 3) AS world, COUNT(*) AS n
   FROM x GROUP BY d ORDER BY n DESC"""),
P("C2.top10_by_decade", "C2", "The top 10 by prestige at each decade's turn", "table",
  "The ten most prestigious programs at the end of 2000, 2010, 2020 and the last year kept (`prestige_history`), to set against the real bluebloods of each era.",
  """WITH x AS (SELECT h._0 AS year, COALESCE(p.real_name, p.fictional_name) AS name, p.id, h._1 AS v, ROW_NUMBER() OVER (PARTITION BY h._0 ORDER BY h._1 DESC) AS rk
     FROM programs__prestige_history h JOIN programs p ON p._row = h._parent
    WHERE h._0 IN (2000, 2010, 2020, (SELECT MAX(_0) FROM programs__prestige_history)))
  SELECT year, rk AS rank, name, id, ROUND(v, 3) AS prestige FROM x WHERE rk <= 10 ORDER BY year DESC, rk""", open="e/programs/{id}"),
# ---------- C3 grades ----------
P("C3.by_division", "C3", "The 12 grades by level", "matrix",
  "The mean of each grade a recruit reads (55-99 within the level; A+ 93 and up, C 64, D 55) by level, now.",
  "WITH " + GRADES + """ SELECT g.grade AS row, p.division AS col, ROUND(AVG(j.value), 1) AS value FROM programs p, json_each(p.grades) j JOIN g ON g.k = j.key
   GROUP BY g.k, p.division ORDER BY g.k, CASE p.division WHEN 'FBS' THEN 0 WHEN 'FCS' THEN 1 WHEN 'D2' THEN 2 WHEN 'NAIA' THEN 3 WHEN 'D3' THEN 4 ELSE 5 END"""),
P("C3.speed", "C3", "How far each grade moves in a year", "bars",
  "The mean move of each grade from one year to the next over every program and year (`grade_history`): fast grades move 80% toward their target a year, steady ones 15%, conference strength only on realignment.",
  """WITH """ + GRADES + """, gh AS MATERIALIZED (SELECT h._parent AS prog, h._0 AS y, CAST(j.key AS INTEGER) AS k, j.value AS v FROM programs__grade_history h, json_each(h._1) j)
  SELECT g.grade AS label, ROUND(AVG(ABS(b.v - a.v)), 2) AS world, COUNT(*) AS n FROM gh a JOIN gh b ON b.prog = a.prog AND b.k = a.k AND b.y = a.y + 1 JOIN g ON g.k = a.k
  GROUP BY a.k ORDER BY a.k"""),
P("C3.academics_top", "C3", "The best academics grades", "table",
  "The 25 programs with the best academics grade, with their academic tier and admissions bar: Stanford, Northwestern, Vanderbilt and Notre Dame should be near the top.",
  """SELECT p.id, COALESCE(p.real_name, p.fictional_name) AS name, p.division, p.academic_tier AS tier, ROUND(json_extract(p.grades, '$[0]'), 1) AS academics,
        ROUND(p.admissions_bar, 2) AS admissions_bar, p.special_admits FROM programs p ORDER BY academics DESC LIMIT 25""", open="e/programs/{id}"),
# ---------- C4 seasons ----------
P("C4.points", "C4", "Points a team game by level, season by season", "line",
  "Points a team scored per game in each level's program seasons with engine totals, against the real mean (CFBD 2001-2025 for FBS, FCS from 2003, D2 and D3 from 2021-22; `college_history.json`). Pick a level.",
  PS + """ SELECT year, division AS grp, ROUND(1.0 * SUM(pf) / SUM(g), 2) AS world,
        json_extract(""" + RD("college_history.json") + """, '$.scores_by_season.by_season.' || year || '.' || division || '.points_per_team_game') AS real
   FROM ps WHERE g > 0 GROUP BY year, division ORDER BY division, year"""),
P("C4.droughts", "C4", "Longest runs without a winning season, power programs", "table",
  "Each power program's longest run of seasons without more wins than losses (all games). Real 2001-2025: Kansas 14, Duke 12, Indiana 11, Colorado, Illinois and Vanderbilt 10 (`trial3_baselines.json` p4_droughts).",
  PS + """, x AS (SELECT pid, name, year, (w > l) AS win, year - ROW_NUMBER() OVER (PARTITION BY pid, (w > l) ORDER BY year) AS run FROM ps WHERE """ + POWER + """)
  SELECT pid AS id, name, MIN(year) AS from_year, MAX(year) AS to_year, COUNT(*) AS seasons FROM x WHERE win = 0 GROUP BY pid, run ORDER BY seasons DESC LIMIT 25""",
  open="e/programs/{id}"),
P("C4.records_by_level", "C4", "Win rates by level and season", "table",
  "The spread of win rates each season by level: the share of programs at .750 or better, at .250 or worse, and unbeaten.",
  PS + """ SELECT year, division, COUNT(*) AS programs, ROUND(100.0 * AVG(1.0 * w / NULLIF(w + l, 0) >= 0.75), 1) AS pct_750_up,
        ROUND(100.0 * AVG(1.0 * w / NULLIF(w + l, 0) <= 0.25), 1) AS pct_250_down, SUM(l = 0 AND w > 0) AS unbeaten, SUM(w = 0 AND l > 0) AS winless
   FROM ps GROUP BY year, division ORDER BY year DESC, division"""),
P("C4.over_under", "C4", "Seasons furthest from expected wins (world log)", "table",
  "Each season's wins against the expected wins the world set before it (`TeamResult.expected`, from the world log's yearly program rows): the biggest over- and under-performers, and whether the head coach kept his job.",
  """WITH x AS (SELECT l.season AS year, l.id, l.result__expected AS expected, l.result__wins AS w, l.result__losses AS lo, l.program__division AS division,
        (SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = l.id) AS name FROM log_programs l)
  SELECT * FROM (SELECT year, id, name, division, w || '-' || lo AS record, ROUND(expected, 1) AS expected, ROUND(w - expected, 1) AS over FROM x WHERE expected IS NOT NULL ORDER BY over DESC LIMIT 20)
  UNION ALL SELECT * FROM (SELECT year, id, name, division, w || '-' || lo, ROUND(expected, 1), ROUND(w - expected, 1) AS over FROM x WHERE expected IS NOT NULL ORDER BY over ASC LIMIT 20)""",
  needs=WL, open="e/programs/{id}"),
# ---------- C5 titles ----------
P("C5.champions", "C5", "National champions by season and level", "table",
  "Every national champion the world crowned, by season and level, with its record and head coach. Newest first.",
  PS + """ SELECT year, division, pid AS id, name AS champion, w || '-' || l AS record, (l = 0) AS unbeaten, (SELECT name FROM coaches c WHERE c.id = ps.coach) AS coach, conference
   FROM ps WHERE nt = 1 ORDER BY year DESC, CASE division WHEN 'FBS' THEN 0 WHEN 'FCS' THEN 1 WHEN 'D2' THEN 2 WHEN 'NAIA' THEN 3 WHEN 'D3' THEN 4 ELSE 5 END""",
  open="e/programs/{id}",
  says=PS + """ SELECT printf('%d FBS titles in the world: %s won %d; %d different champions.', (SELECT COUNT(*) FROM ps WHERE nt = 1 AND division = 'FBS'),
        (SELECT name FROM ps WHERE nt = 1 AND division = 'FBS' GROUP BY pid ORDER BY COUNT(*) DESC LIMIT 1),
        (SELECT COUNT(*) AS n FROM ps WHERE nt = 1 AND division = 'FBS' GROUP BY pid ORDER BY n DESC LIMIT 1), (SELECT COUNT(DISTINCT pid) FROM ps WHERE nt = 1 AND division = 'FBS'))"""),
P("C5.rolling_most", "C5", "Most FBS titles by one school in 15 years", "line",
  "For each 15-season window ending that year: the most FBS titles one program won in it. Real band 4-6 (Alabama's 6 in 2006-2020 the most; `trial3_baselines.json`).",
  PS + """, t AS (SELECT pid, year FROM ps WHERE nt = 1 AND division = 'FBS'), e AS (SELECT DISTINCT year AS e FROM ps WHERE year >= (SELECT MIN(year) + 14 FROM ps))
  SELECT e.e AS year, (SELECT MAX(n) FROM (SELECT COUNT(*) AS n FROM t WHERE t.year BETWEEN e.e - 14 AND e.e GROUP BY pid)) AS most, 4 AS real_lo, 6 AS real_hi FROM e ORDER BY e.e"""),
P("C5.rolling_distinct", "C5", "Different FBS champions in 15 years", "line",
  "For each 15-season window ending that year: how many different programs won FBS titles in it. Real band 7-9 (8 in 2011-2025; `trial3_baselines.json`).",
  PS + """, t AS (SELECT pid, year FROM ps WHERE nt = 1 AND division = 'FBS'), e AS (SELECT DISTINCT year AS e FROM ps WHERE year >= (SELECT MIN(year) + 14 FROM ps))
  SELECT e.e AS year, (SELECT COUNT(DISTINCT pid) FROM t WHERE t.year BETWEEN e.e - 14 AND e.e) AS distinct_champions, 7 AS real_lo, 9 AS real_hi FROM e ORDER BY e.e"""),
P("C5.by_program", "C5", "National titles by program", "bars",
  "Programs with the most national titles in the world's seasons, by level (pick a level). No real line: the real all-time counts cover other years.",
  PS + """ SELECT division AS grp, name AS label, COUNT(*) AS world, pid AS id FROM ps WHERE nt = 1 GROUP BY division, pid ORDER BY division, world DESC""",
  open="e/programs/{id}"),
P("C5.lower_distinct", "C5", "Different champions per decade, every level", "table",
  "Distinct national champions in each level per decade of the world's seasons. Real FCS 2011-2025: 4 different champions in 14 seasons (North Dakota State 9; `college_history.json` champions).",
  PS + """ SELECT division, (year / 10) * 10 AS decade, COUNT(*) AS titles, COUNT(DISTINCT pid) AS distinct_champions,
        (SELECT name FROM ps p2 WHERE p2.nt = 1 AND p2.division = ps.division AND p2.year / 10 = ps.year / 10 GROUP BY p2.pid ORDER BY COUNT(*) DESC LIMIT 1) AS most_by
   FROM ps WHERE nt = 1 GROUP BY division, year / 10 ORDER BY division, decade"""),
# ---------- C6 conferences ----------
P("C6.list", "C6", "Every conference now", "table",
  "Each conference in the world's last season: its level, members, the champion and the most titles in the world's seasons. A row opens the conference's page.",
  PS + """, last AS (SELECT * FROM ps WHERE year = (SELECT MAX(year) FROM ps))
  SELECT l.conference, MIN(l.division) AS division, COUNT(*) AS members, (SELECT group_concat(name, ', ') FROM last x WHERE x.conference = l.conference AND x.ct = 1) AS champion,
        (SELECT name || ' (' || n || ')' FROM (SELECT name, SUM(ct) AS n FROM ps x WHERE x.conference = l.conference GROUP BY pid ORDER BY n DESC LIMIT 1)) AS most_titles,
        ROUND(AVG((SELECT prestige FROM programs WHERE _row = l.prog)), 3) AS prestige
   FROM last l GROUP BY l.conference ORDER BY CASE MIN(l.division) WHEN 'FBS' THEN 0 WHEN 'FCS' THEN 1 WHEN 'D2' THEN 2 WHEN 'D3' THEN 3 WHEN 'NAIA' THEN 4 ELSE 5 END, prestige DESC""",
  open="conference/{conference}"),
P("C6.champions", "C6", "Conference champions, season by season", "table",
  "Every conference title the world gave, by conference and season, with the champion's record. A conference opens its own page.",
  PS + """ SELECT conference, year, pid AS id, name AS champion, w || '-' || l AS record, division FROM ps WHERE ct = 1 ORDER BY year DESC, division, conference""",
  open="conference/{conference}"),
P("C6.most_15y", "C6", "Most conference titles by one school in 15 years", "line",
  "For each 15-season window: the most FBS conference titles one program won (shared titles count). Real 2001-2025: 10 (Boise State 2001-2015, Alabama 2011-2025; `college_history.json` fbs_conference_champions).",
  PS + """, t AS (SELECT pid, year FROM ps WHERE ct = 1 AND division = 'FBS'), e AS (SELECT DISTINCT year AS e FROM ps WHERE year >= (SELECT MIN(year) + 14 FROM ps))
  SELECT e.e AS year, (SELECT MAX(n) FROM (SELECT COUNT(*) AS n FROM t WHERE t.year BETWEEN e.e - 14 AND e.e GROUP BY pid)) AS most, 10 AS real FROM e ORDER BY e.e"""),
P("C6.strength", "C6", "Conference strength grade by year, FBS conferences", "line",
  "The conference strength grade (55-99) each year, averaged over each FBS conference's members of today (`grade_history`). Pick a conference above the chart.",
  """SELECT h._0 AS year, p.conference AS grp, ROUND(AVG(json_extract(h._1, '$[9]')), 1) AS strength FROM programs__grade_history h JOIN programs p ON p._row = h._parent
   WHERE p.division = 'FBS' GROUP BY h._0, p.conference ORDER BY p.conference, h._0"""),
P("C6.title_games", "C6", "Conference title games by conference (game log)", "table",
  "The world's conference title games each season from the game log, against each conference's real first title game (SEC 1992, Big 12 1996-2010 and 2017 on, ACC 2005, MAC 1997, Big Ten 2011, Pac-12 2011, Mountain West 2013, American 2015, Sun Belt 2018; `calendar/rules.rs` conference_rules).",
  PS + """, tg AS (SELECT g.season AS year, json_extract(g.teams, '$[0]') AS home, json_extract(g.teams, '$[1]') AS away FROM log_games g WHERE g.stream = 'college' AND g.kind = 'conference title'),
   r(conf, first) AS (VALUES ('SEC', 1992), ('Big 12', 1996), ('ACC', 2005), ('MAC', 1997), ('Big Ten', 2011), ('Pac-12', 2011), ('Mountain West', 2013), ('American', 2015), ('Sun Belt', 2018), ('CUSA', 2005))
  SELECT ps.conference, MIN(tg.year) AS first_in_log, MAX(tg.year) AS last_in_log, COUNT(*) AS games, r.first AS real_first
   FROM tg JOIN ps ON ps.prog = tg.home AND ps.year = tg.year LEFT JOIN r ON r.conf = ps.conference GROUP BY ps.conference ORDER BY games DESC""",
  needs=GL),
# ---------- C7 schedules (game log) ----------
P("C7.games_per_team", "C7", "Games per team each season by level (game log)", "table",
  "Regular-season games each team played per season, from the game log: the mean, fewest and most. The rule: 12 in FBS (11 before 2006), 13 with a trip to Hawaii.",
  PS + """, tg AS (SELECT g.season, json_extract(g.teams, '$[' || k.i || ']') AS t FROM log_games g, (SELECT 0 AS i UNION ALL SELECT 1) k
        WHERE g.stream = 'college' AND COALESCE(g.kind, 'regular') = 'regular'),
   c AS (SELECT season, t, COUNT(*) AS n FROM tg GROUP BY season, t)
  SELECT c.season AS year, ps.division, ROUND(AVG(c.n), 2) AS mean, MIN(c.n) AS fewest, MAX(c.n) AS most, COUNT(*) AS teams
   FROM c JOIN ps ON ps.prog = c.t AND ps.year = c.season GROUP BY c.season, ps.division ORDER BY year DESC, ps.division""", needs=GL),
P("C7.fcs_buy", "C7", "FBS teams that play an FCS team (game log)", "line",
  "The share of FBS teams with at least one FCS opponent each season, against the real share (CFBD 2001-2025, `college_history.json`).",
  PS + """, x AS (SELECT g.season AS year, h.pid AS t, (SELECT division FROM ps WHERE prog = json_extract(g.teams, '$[1]') AND year = g.season) AS od FROM log_games g JOIN ps h ON h.prog = json_extract(g.teams, '$[0]') AND h.year = g.season
        WHERE g.stream = 'college' AND h.division = 'FBS'
      UNION ALL SELECT g.season, a.pid, (SELECT division FROM ps WHERE prog = json_extract(g.teams, '$[0]') AND year = g.season) FROM log_games g JOIN ps a ON a.prog = json_extract(g.teams, '$[1]') AND a.year = g.season
        WHERE g.stream = 'college' AND a.division = 'FBS')
  SELECT year, ROUND(100.0 * COUNT(DISTINCT CASE WHEN od = 'FCS' THEN t END) / COUNT(DISTINCT t), 1) AS world,
        100.0 * json_extract(""" + RD("college_history.json") + """, '$.scores_by_season.by_season.' || year || '.FBS.fbs_teams_playing_fcs_share') AS real
   FROM x GROUP BY year ORDER BY year""", needs=GL),
P("C7.attendance", "C7", "FBS crowds a home game (game log)", "line",
  "The mean crowd at FBS home games (not neutral sites) each season, against the real mean (CFBD 2001-2025, `college_attendance.csv`).",
  PS + """ SELECT g.season AS year, ROUND(AVG(g.attendance)) AS world, json_extract(""" + RD("college_history.json") + """, '$.fbs_attendance.by_season.' || g.season || '.mean') AS real
   FROM log_games g JOIN ps ON ps.prog = json_extract(g.teams, '$[0]') AND ps.year = g.season
  WHERE g.stream = 'college' AND ps.division = 'FBS' AND COALESCE(g.neutral, 0) = 0 AND g.attendance > 0 GROUP BY g.season ORDER BY g.season""", needs=GL),
P("C7.days", "C7", "Which day FBS home games are played (game log)", "bars",
  "FBS regular-season home games by day of the week (shares), against the real shares of 2021-2025 (`college_history.json` fbs_game_days).",
  PS + """, w AS (SELECT CAST(strftime('%w', g.date * 86400, 'unixepoch') AS INTEGER) AS d FROM log_games g JOIN ps ON ps.prog = json_extract(g.teams, '$[0]') AND ps.year = g.season
        WHERE g.stream = 'college' AND ps.division = 'FBS' AND COALESCE(g.kind, 'regular') = 'regular'),
   names(d, label) AS (VALUES (0, 'Sunday'), (1, 'Monday'), (2, 'Tuesday'), (3, 'Wednesday'), (4, 'Thursday'), (5, 'Friday'), (6, 'Saturday')),
   rj AS (SELECT j.value AS season FROM json_each(json_extract(""" + RD("college_history.json") + """, '$.fbs_game_days.by_season')) j WHERE CAST(j.key AS INTEGER) BETWEEN 2021 AND 2025),
   rt AS (SELECT n.label, SUM(COALESCE(json_extract(rj.season, '$.' || n.label), 0)) AS c FROM rj, names n GROUP BY n.label)
  SELECT n.label, ROUND(100.0 * (SELECT COUNT(*) FROM w WHERE w.d = n.d) / (SELECT COUNT(*) FROM w), 2) AS world,
        ROUND(100.0 * (SELECT c FROM rt WHERE rt.label = n.label) / (SELECT SUM(c) FROM rt), 2) AS real FROM names n ORDER BY n.d""", needs=GL),
# ---------- C8 deals ----------
P("C8.deals", "C8", "Non-conference deals on the books", "table",
  "Every game under contract (`World.deals`) by kind: how many, how far ahead they were signed, and the fee. Payouts and horizons are proposed, not confirmed (`world/deals.rs`).",
  """SELECT kind, COUNT(*) AS deals, ROUND(AVG(year - signed), 1) AS years_ahead_mean, MIN(year - signed) AS fewest, MAX(year - signed) AS most,
        ROUND(AVG(fee)) AS fee_mean, MAX(fee) AS fee_max, MIN(year) AS first_season, MAX(year) AS last_season FROM deals GROUP BY kind ORDER BY deals DESC"""),
# ---------- C9 rivalries ----------
P("C9.series", "C9", "Rivalry series as the world carries them", "table",
  "Each listed rivalry the world tracks (`college.series`): its all-time wins and ties as loaded from real history and carried on, and who holds the trophy now.",
  """WITH r AS (SELECT _0 AS i, _1 AS a, CASE WHEN json_extract(_key, '$[0]') = _1 THEN json_extract(_key, '$[1]') ELSE json_extract(_key, '$[0]') END AS b FROM college__rivalry_of)
  SELECT r.a AS id, COALESCE(pa.real_name, pa.fictional_name) AS team, COALESCE(pb.real_name, pb.fictional_name) AS rival, json_extract(s.wins, '$[0]') || '-' || json_extract(s.wins, '$[1]') || CASE WHEN s.ties > 0 THEN '-' || s.ties ELSE '' END AS series,
        s.since, (SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = s.holder) AS trophy_holder, CASE WHEN s.real = 1 THEN 'real record loaded' ELSE 'world only' END AS source
   FROM college__series s JOIN r ON r.i = s._row JOIN programs pa ON pa._row = r.a JOIN programs pb ON pb._row = r.b ORDER BY json_extract(s.wins, '$[0]') + json_extract(s.wins, '$[1]') DESC""", open="e/programs/{id}"),
P("C9.games", "C9", "Rivalry games each season (game log)", "table",
  "Every rivalry game the game log tagged, newest first, with the score.",
  """SELECT g.season AS year, g.date, json_extract(g.teams, '$[0]') AS home, (SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = json_extract(g.teams, '$[0]')) AS home_name,
        (SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = json_extract(g.teams, '$[1]')) AS away_name, g.score, g.name
   FROM log_games g WHERE g.stream = 'college' AND COALESCE(g.rivalry, 0) != 0 ORDER BY g.season DESC, g.date""", needs=GL),
# ---------- C10 polls ----------
P("C10.final", "C10", "The last full season's final AP poll", "table",
  "The final AP top 25 of the last finished season (`college_prev.polls`), with points and first-place votes.",
  """WITH p AS (SELECT * FROM college_prev__polls WHERE kind LIKE 'Ap%' ORDER BY date DESC LIMIT 1)
  SELECT t.key + 1 AS rank, t.value AS id, (SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = t.value) AS team,
        (SELECT wins || '-' || losses FROM programs__seasons s WHERE s._parent = t.value AND s.year = (SELECT year FROM college_prev)) AS record,
        json_extract(p.points, '$[' || t.key || ']') AS points, json_extract(p.firsts, '$[' || t.key || ']') AS first_place
   FROM p, json_each(p.top) t ORDER BY t.key""", open="e/programs/{id}"),
P("C10.preseason_vs_final", "C10", "Preseason AP rank against the season's win rate", "bars",
  "The last full season: each preseason AP band's win rate against the real bands (AP preseason top 25, 2004-2025: 1-5 .825, 6-10 .716, 11-15 .677, 16-25 .641; `college_history.json`).",
  """WITH pre AS (SELECT t.key + 1 AS rank, t.value AS prog FROM college_prev__polls p, json_each(p.top) t
        WHERE p._row = (SELECT _row FROM college_prev__polls WHERE kind LIKE 'Ap%' ORDER BY date LIMIT 1)),
   y AS (SELECT year AS y FROM college_prev),
   x AS (SELECT CASE WHEN rank <= 5 THEN '1-5' WHEN rank <= 10 THEN '6-10' WHEN rank <= 15 THEN '11-15' ELSE '16-25' END AS band, 1.0 * s.wins / (s.wins + s.losses) AS wp
     FROM pre JOIN programs__seasons s ON s._parent = pre.prog AND s.year = (SELECT y FROM y))
  SELECT band AS label, ROUND(AVG(wp), 3) AS world, json_extract(""" + RD("college_history.json") + """, '$.preseason_rank_vs_season.win_pct_by_rank_band."' || band || '"') AS real, COUNT(*) AS n
   FROM x GROUP BY band ORDER BY MIN(CASE band WHEN '1-5' THEN 1 WHEN '6-10' THEN 2 WHEN '11-15' THEN 3 ELSE 4 END)"""),
# ---------- C11 voters ----------
P("C11.panels", "C11", "Poll panels and the committee", "bars",
  "How many voters sit on each panel now, against the real sizes (AP 62, Coaches 65, committee 13; `world/voters.rs`). The Harris poll (114 voters) ran 2005-2013 only.",
  """SELECT 'AP' AS label, (SELECT COUNT(*) FROM voters__ap) AS world, 62 AS real
   UNION ALL SELECT 'Coaches', (SELECT COUNT(*) FROM voters__coaches), 65 UNION ALL SELECT 'Committee', (SELECT COUNT(*) FROM voters__committee), 13"""),
# ---------- C12 postseason ----------
P("C12.bowls", "C12", "FBS bowl and playoff games each season (game log)", "line",
  "FBS bowls, playoff games and title games each season in the game log, against the real count (CFBD 2001-2025, `college_history.json` postseason).",
  PS + """ SELECT g.season AS year, COUNT(*) AS world, json_extract(""" + RD("college_history.json") + """, '$.postseason.by_season.' || g.season || '.fbs_bowl_and_playoff_games') AS real
   FROM log_games g JOIN ps ON ps.prog = json_extract(g.teams, '$[0]') AND ps.year = g.season
  WHERE g.stream = 'college' AND ps.division = 'FBS' AND g.kind IN ('bowl', 'playoff', 'title') GROUP BY g.season ORDER BY g.season""", needs=GL),
P("C12.appearances", "C12", "Bowl and playoff appearances by program (game log)", "table",
  "Each FBS program's bowl and playoff games and record in the seasons the game log holds.",
  """WITH x AS (SELECT json_extract(g.teams, '$[' || k.i || ']') AS t, g.kind, (json_extract(g.score, '$[' || k.i || ']') > json_extract(g.score, '$[' || (1 - k.i) || ']')) AS won
     FROM log_games g, (SELECT 0 AS i UNION ALL SELECT 1) k WHERE g.stream = 'college' AND g.kind IN ('bowl', 'playoff', 'title'))
  SELECT p.id, COALESCE(p.real_name, p.fictional_name) AS name, p.division, SUM(kind = 'bowl') AS bowls, SUM(kind IN ('playoff', 'title')) AS playoff_games, SUM(won) || '-' || SUM(1 - won) AS record
   FROM x JOIN programs p ON p._row = x.t GROUP BY x.t ORDER BY bowls + playoff_games DESC LIMIT 60""", needs=GL, open="e/programs/{id}"),
# ---------- C13 lower-division playoffs ----------
P("C13.fields", "C13", "Playoff fields by level (game log)", "table",
  "Playoff games and teams in each level's bracket each season from the game log. Real fields: FCS 24 from 2013 (12 before), D3 32 from 2005, NAIA 16 then 20 from 2022, JUCO 2 then 4 from 2021.",
  PS + """, x AS (SELECT g.season AS year, ps.division, json_extract(g.teams, '$[0]') AS h, json_extract(g.teams, '$[1]') AS a FROM log_games g JOIN ps ON ps.prog = json_extract(g.teams, '$[0]') AND ps.year = g.season
     WHERE g.stream = 'college' AND g.kind IN ('playoff', 'title'))
  SELECT year, division, COUNT(*) AS games, (SELECT COUNT(DISTINCT t) FROM (SELECT h AS t FROM x x2 WHERE x2.year = x.year AND x2.division = x.division UNION SELECT a FROM x x2 WHERE x2.year = x.year AND x2.division = x.division)) AS teams
   FROM x GROUP BY year, division ORDER BY year DESC, division""", needs=GL),
# ---------- C15 NCAA ----------
P("C15.ledger", "C15", "Rosters and signees against the limits, FBS", "line",
  "Each season's FBS mean roster and signees against the season's limit (`ProgramEra.ledger`): 85 scholarships with 25 a year (95 before 1992), the 105 roster from 2025.",
  """SELECT l.season AS year, ROUND(AVG(l.roster), 1) AS roster, ROUND(AVG(l."limit"), 1) AS roster_limit, ROUND(AVG(l.signees), 1) AS signees
   FROM programs__era__ledger l JOIN programs p ON p._row = l._parent WHERE p.division = 'FBS' GROUP BY l.season ORDER BY l.season"""),
P("C15.cases", "C15", "NCAA cases (rogue boosters found)", "table",
  "Every NCAA case the world opened: the program, when, what was found and the penalty (`ProgramEra.cases`). Real: a few major football cases a decade (from memory).",
  """SELECT p.id, COALESCE(p.real_name, p.fictional_name) AS program, p.division, c.* FROM programs__era__cases c JOIN programs p ON p._row = c._parent ORDER BY 4 DESC""",
  open="e/programs/{id}"),
P("C15.bans", "C15", "Programs under NCAA penalties now", "table",
  "Programs with scholarships cut or a postseason ban running (`ProgramEra.cut_until`, `ban_until`).",
  """SELECT p.id, COALESCE(p.real_name, p.fictional_name) AS program, p.division, p.era__cut AS scholarships_cut, p.era__cut_until AS cut_until, p.era__ban_until AS ban_until
   FROM programs p WHERE p.era__cut_until >= (SELECT year FROM world) OR p.era__ban_until >= (SELECT year FROM world) ORDER BY p.era__ban_until DESC""", open="e/programs/{id}"),
# ---------- C16 eligibility ----------
P("C16.roads", "C16", "The long roads each year", "line",
  "The calendar's yearly counters: four-game redshirts, redshirt tricks, medical hardship waivers, sixth years, COVID returns and declines, (`CalendarLog`). Older world files lack some.",
  """SELECT year, four_game_redshirts, redshirt_tricks, medical_hardships, sixth_years, covid_returns, covid_declined FROM log_history ORDER BY year"""),
P("C16.last_age", "C16", "Age in a player's last college season", "hist",
  "The age of every pro and alumnus in his last college season (his season rows): most real players finish at 21-23.",
  """WITH x AS (SELECT MAX(h.age) AS a FROM people__player__history h JOIN people p ON p._row = h._parent WHERE h.level IN ('FBS', 'FCS', 'D2', 'D3', 'NAIA', 'JUCO', 'College') AND p.stage = 'Pro' GROUP BY h._parent)
  SELECT a AS x, COUNT(*) AS world FROM x GROUP BY a ORDER BY a"""),
# ---------- C17 academics ----------
P("C17.bars", "C17", "Admissions bars and the classes they sign", "table",
  "The programs with the strictest admissions (Stanford 3.6, Northwestern 3.5, Notre Dame 3.4 are proposed, not confirmed; `program.rs` STRICT_ADMISSIONS): their bar, special admits, their college players' GPA at signing and stars, against their level.",
  """SELECT p.id, COALESCE(p.real_name, p.fictional_name) AS name, p.division, ROUND(p.admissions_bar, 2) AS bar, p.special_admits, ROUND(x.gpa_at_signing, 2) AS gpa, ROUND(x.stars, 2) AS stars,
        x.n AS players FROM programs p LEFT JOIN (SELECT college, AVG(gpa_at_signing) AS gpa_at_signing, AVG(stars) AS stars, COUNT(*) AS n FROM people WHERE stage = 'College' GROUP BY college) x ON x.college = p._row
  ORDER BY p.admissions_bar DESC LIMIT 40""", open="e/programs/{id}"),
P("C17.tiers", "C17", "Academic redshirts and GPA by level", "bars",
  "The share of today's college players who were academic redshirts at signing, by level, and their mean GPA at signing.",
  """SELECT p.division AS label, ROUND(100.0 * AVG(COALESCE(x.academic_redshirt, 0)), 2) AS world, ROUND(AVG(x.gpa_at_signing), 2) AS gpa, COUNT(*) AS n
   FROM people x JOIN programs p ON p._row = x.college WHERE x.stage = 'College' GROUP BY p.division ORDER BY n DESC"""),
P("C17.terms", "C17", "Ineligible players and grade-outs each year", "line",
  "Players ruled ineligible at the spring and fall term checks, and high school grade-outs, each year (`CalendarLog`).",
  """SELECT year, spring_ineligible, fall_ineligible, hs_grade_outs FROM log_history ORDER BY year"""),
# ---------- C19 seats and the AD ----------
P("C19.tenure", "C19", "How long FBS head coaches last", "bars",
  "Head coach runs at FBS programs (consecutive seasons as the season's coach) that began after the history's first season and early enough to have run 10: the share lasting at least 2, 3, 4, 5, 6, 8 and 10 seasons, against the real shares (FBS head coaches who started 2005-2019, ESPN; `college_people.json`).",
  PS + """, r AS (SELECT pid, coach, MIN(year) AS a, MAX(year) AS z, COUNT(*) AS n FROM (SELECT pid, coach, year, year - ROW_NUMBER() OVER (PARTITION BY pid, coach ORDER BY year) AS run FROM ps WHERE division = 'FBS' AND coach IS NOT NULL) GROUP BY pid, coach, run),
   ok AS (SELECT * FROM r WHERE a > (SELECT MIN(year) FROM ps) AND a <= (SELECT MAX(year) FROM ps) - 10),
   k(k) AS (VALUES (2), (3), (4), (5), (6), (8), (10))
  SELECT 'At least ' || k.k || ' seasons' AS label, ROUND(100.0 * (SELECT AVG(n >= k.k) FROM ok), 1) AS world,
        ROUND(100.0 * json_extract(""" + RD("college_people.json") + """, '$.fbs_head_coaches.share_lasting_at_least."' || k.k || '"'), 1) AS real, (SELECT COUNT(*) FROM ok) AS n FROM k"""),
P("C19.ads", "C19", "Athletic directors (debug)", "table",
  "Each program's athletic director, when he started, and his hidden judgement and patience (1-20) that set the bar a head coach must clear.",
  """SELECT p.id, COALESCE(p.real_name, p.fictional_name) AS program, p.division, p.ad__name AS ad, p.ad__since AS since, ROUND(p.ad__judgement, 1) AS judgement, ROUND(p.ad__patience, 1) AS patience
   FROM programs p WHERE p.ad__name IS NOT NULL ORDER BY CASE p.division WHEN 'FBS' THEN 0 WHEN 'FCS' THEN 1 ELSE 2 END, p.prestige DESC""", open="e/programs/{id}"),
# ---------- C20 rosters ----------
P("C20.by_position", "C20", "FBS players a roster by position", "bars",
  "College players at FBS programs today per program, by position, against the real FBS rosters (ESPN 2022-24, about 118 listed; `recruiting.rs` real_roster).",
  """WITH r(pos, v) AS (VALUES ('QB', 5.8), ('RB', 8.5), ('WR', 16.2), ('TE', 7.4), ('OT', 9.36), ('IOL', 10.14), ('EDGE', 9.1), ('DT', 8.4), ('LB', 14.6), ('CB', 10.21), ('S', 11.99), ('K', 3.5), ('P', 2.0), ('LS', 2.5)),
   t AS (SELECT COUNT(DISTINCT x.college) AS n FROM people x JOIN programs p ON p._row = x.college WHERE x.stage = 'College' AND p.division = 'FBS')
  SELECT r.pos AS label, ROUND(1.0 * COUNT(x._row) / (SELECT n FROM t), 2) AS world, r.v AS real FROM r
   LEFT JOIN people x ON x.player__position = r.pos AND x.stage = 'College' AND x.college IN (SELECT _row FROM programs WHERE division = 'FBS')
  GROUP BY r.pos ORDER BY real DESC"""),
P("C20.sizes", "C20", "Roster sizes by level", "table",
  "College players on each program's roster today, by level: the median, the smallest and largest, and how many have no kicker or no punter. Real FBS: about 105 under the 2025 limit (118 listed 2022-24); no FBS team without a kicker, 4-6% without a punter, a quarter of FCS teams without one.",
  """WITH x AS (SELECT college, COUNT(*) AS n, SUM(player__position = 'K') AS k, SUM(player__position = 'P') AS pu FROM people WHERE stage = 'College' GROUP BY college),
   r AS (SELECT p._row, p.division, COALESCE(x.n, 0) AS n, COALESCE(x.k, 0) AS k, COALESCE(x.pu, 0) AS pu FROM programs p LEFT JOIN x ON x.college = p._row
        WHERE p._row IN (SELECT _parent FROM programs__seasons WHERE year = (SELECT MAX(year) FROM programs__seasons))),
   o AS (SELECT *, ROW_NUMBER() OVER (PARTITION BY division ORDER BY n) AS rk, COUNT(*) OVER (PARTITION BY division) AS cnt FROM r)
  SELECT division, MAX(CASE WHEN rk = (cnt + 1) / 2 THEN n END) AS median, MIN(n) AS smallest, MAX(n) AS largest, ROUND(100.0 * AVG(k = 0), 1) AS pct_no_kicker,
        ROUND(100.0 * AVG(pu = 0), 1) AS pct_no_punter, COUNT(*) AS programs FROM o GROUP BY division ORDER BY programs DESC"""),
P("C20.blue_chips", "C20", "Blue chips by FBS conference", "bars",
  "The share of 4- and 5-star players on each FBS conference's rosters today, against the real blue-chip ratio of the last season in the 247Sports team talent composite (`college_history.json` talent_by_conference).",
  """WITH last AS (SELECT MAX(CAST(key AS INTEGER)) AS y FROM json_each(json_extract(""" + RD("college_history.json") + """, '$.talent_by_conference.by_season')))
  SELECT p.conference AS label, ROUND(100.0 * AVG(x.stars >= 4), 1) AS world,
        ROUND(100.0 * json_extract(""" + RD("college_history.json") + """, '$.talent_by_conference.by_season.' || (SELECT y FROM last) || '."' || p.conference || '".blue_chip_ratio'), 1) AS real, COUNT(*) AS n
   FROM people x JOIN programs p ON p._row = x.college WHERE x.stage = 'College' AND p.division = 'FBS' GROUP BY p.conference ORDER BY world DESC"""),
# ---------- C21 lineups ----------
P("C21.best_22", "C21", "The strongest rosters: each FBS program's best 22", "table",
  "The mean true overall of each FBS program's 22 best college players today (debug: true ratings), with the next 22 and the FBS rank: how far the top sits above the field.",
  """WITH r AS (SELECT x.college AS prog, x.x_overall AS o, ROW_NUMBER() OVER (PARTITION BY x.college ORDER BY x.x_overall DESC) AS rk FROM people x WHERE x.stage = 'College'),
   t AS (SELECT prog, AVG(CASE WHEN rk <= 22 THEN o END) AS top22, AVG(CASE WHEN rk BETWEEN 23 AND 44 THEN o END) AS next22 FROM r GROUP BY prog)
  SELECT p.id, COALESCE(p.real_name, p.fictional_name) AS name, p.conference, ROUND(t.top22, 1) AS best_22, ROUND(t.next22, 1) AS next_22,
        ROUND(t.top22 - (SELECT AVG(top22) FROM t JOIN programs q ON q._row = t.prog WHERE q.division = 'FBS'), 1) AS over_fbs_mean
   FROM t JOIN programs p ON p._row = t.prog WHERE p.division = 'FBS' ORDER BY t.top22 DESC LIMIT 40""", open="e/programs/{id}"),
# ---------- C22 records and production ----------
P("C22.picks", "C22", "Draft picks by program, the last ten drafts", "bars",
  "Each program's draft picks over the world's last ten drafts (`Program.draft_picks`), against its real picks over the last ten real drafts (nflverse, `nfl_draft_by_college.csv`).",
  """WITH w AS (SELECT d._parent AS prog, COUNT(*) AS n FROM programs__draft_picks d WHERE d._0 > (SELECT MAX(_0) FROM programs__draft_picks) - 10 GROUP BY d._parent),
   ry AS (SELECT MAX(season) AS y FROM real_nfl_draft_by_college)
  SELECT COALESCE(p.real_name, p.fictional_name) AS label, w.n AS world,
        (SELECT SUM(r.picks) FROM real_nfl_draft_by_college r WHERE r.season > (SELECT y FROM ry) - 10 AND (r.college = p.real_name OR r.college = REPLACE(p.real_name, ' State', ' St.'))) AS real, p.id
   FROM w JOIN programs p ON p._row = w.prog ORDER BY w.n DESC LIMIT 30""", open="e/programs/{id}"),
P("C22.picks_by_conference", "C22", "Draft picks by conference, draft by draft", "table",
  "Draft picks per draft by today's conference of the program, from `Program.draft_picks`.",
  """SELECT d._0 AS year, p.division, p.conference, COUNT(*) AS picks, COUNT(DISTINCT d._parent) AS programs FROM programs__draft_picks d JOIN programs p ON p._row = d._parent
   GROUP BY d._0, p.conference ORDER BY d._0 DESC, picks DESC"""),
P("C22.records", "C22", "School single-season records", "table",
  "Each FBS program's season records (passing, rushing and receiving yards, sacks; `Program.season_records`), the best first. Real FBS records (from memory, not checked): passing 5,833, rushing 2,628, receiving 2,060, sacks 24.",
  """WITH k(i, what) AS (VALUES (0, 'Passing yards'), (1, 'Rushing yards'), (2, 'Receiving yards'), (3, 'Sacks'))
  SELECT k.what, p.id, COALESCE(p.real_name, p.fictional_name) AS program, r.value, r.holder, r.year FROM programs__season_records r JOIN programs p ON p._row = r._parent JOIN k ON k.i = r._i
   WHERE p.division = 'FBS' ORDER BY k.i, r.value DESC""", open="e/programs/{id}"),
# ---------- C42 news ----------
P("C42.kinds", "C42", "News by kind", "table",
  "Every news item the world keeps, by kind: how many, from when to when, and a year's mean.",
  """SELECT kind, COUNT(*) AS items, MIN(year) AS first_year, MAX(year) AS last_year, ROUND(1.0 * COUNT(*) / (MAX(year) - MIN(year) + 1), 1) AS a_year FROM news GROUP BY kind ORDER BY items DESC"""),
P("C42.feed", "C42", "The latest news about programs", "table",
  "The 500 latest news items that name a program, newest first; a row opens the program.",
  """SELECT n.year, n.date, n.kind, n.program AS id, COALESCE(p.real_name, p.fictional_name) AS program, n.text FROM news n JOIN programs p ON p._row = n.program ORDER BY n.year DESC, n.date DESC LIMIT 500""",
  open="e/programs/{id}"),
# ---------- T13 moves ----------
P("T13.moves", "T13", "Every move between levels", "table",
  "Each program's moves from one level to another, from its real history through 2025 and the world's own years after (a season at a new level, or no team).",
  PS + """, x AS (SELECT pid, name, year, division, LAG(division) OVER (PARTITION BY pid ORDER BY year) AS before, LAG(year) OVER (PARTITION BY pid ORDER BY year) AS prev_year FROM ps)
  SELECT year, pid AS id, name, COALESCE(before, '') AS from_level, division AS to_level, CASE WHEN year - prev_year > 1 THEN 'after ' || (year - prev_year - 1) || ' years with no team' ELSE '' END AS gap
   FROM x WHERE before IS NOT NULL AND (before != division OR year - prev_year > 1) ORDER BY year DESC""", open="e/programs/{id}"),
P("T13.per_decade", "T13", "Moves up a level per decade", "bars",
  "Programs moving up a level (D3 to D2, D2 to FCS, FCS to FBS) per decade of seasons, the world against real (2004-2025: 21 FCS to FBS, 21 D2 or D3 up; `college_history.json` division_moves_up).",
  PS + """, x AS (SELECT year, division, LAG(division) OVER (PARTITION BY pid ORDER BY year) AS before FROM ps),
   rank(d, r) AS (VALUES ('JUCO', 0), ('NAIA', 1), ('D3', 1), ('D2', 2), ('FCS', 3), ('FBS', 4)),
   up AS (SELECT (year / 10) * 10 AS decade, before || ' to ' || division AS move FROM x JOIN rank a ON a.d = x.before JOIN rank b ON b.d = x.division WHERE b.r > a.r),
   real AS (SELECT (json_extract(j.value, '$[0]') / 10) * 10 AS decade, json_extract(j.value, '$[2]') || ' to ' || json_extract(j.value, '$[3]') AS move FROM json_each(json_extract(""" + RD("college_history.json") + """, '$.division_moves_up.moves')) j)
  SELECT decade || 's: ' || move AS label, (SELECT COUNT(*) FROM up u WHERE u.decade = m.decade AND u.move = m.move) AS world, (SELECT COUNT(*) FROM real r WHERE r.decade = m.decade AND r.move = m.move) AS real
   FROM (SELECT decade, move FROM up UNION SELECT decade, move FROM real) m ORDER BY decade, move"""),
P("T13.movers", "T13", "Movers to FBS: their first three seasons", "table",
  "Each program's first three seasons after moving up to FBS: record and win rate. Real movers often struggle at first, with exceptions (App State, James Madison).",
  PS + """, x AS (SELECT pid, name, year, division, LAG(division) OVER (PARTITION BY pid ORDER BY year) AS before FROM ps),
   m AS (SELECT pid, name, year AS moved FROM x WHERE division = 'FBS' AND before IS NOT NULL AND before != 'FBS')
  SELECT m.moved AS year, m.pid AS id, m.name, SUM(ps.w) || '-' || SUM(ps.l) AS first_three, ROUND(1.0 * SUM(ps.w) / NULLIF(SUM(ps.w + ps.l), 0), 3) AS pct
   FROM m JOIN ps ON ps.pid = m.pid AND ps.year BETWEEN m.moved AND m.moved + 2 GROUP BY m.pid, m.moved ORDER BY m.moved DESC""", open="e/programs/{id}"),
# ---------- C14 money (the Money page) ----------
P("C14.budgets", "C14", "NIL collective budgets by level", "table",
  "Each level's collective NIL budgets today (`Program.collective`): the median, the top tenth and the biggest, booster bases and big boosters. Real 2024-25: the biggest P4 collectives near $20 million a year; none before July 2021.",
  """WITH r AS (SELECT p.division, p.collective__budget AS v, p.collective__base AS base, p.collective__big_booster AS big, ROW_NUMBER() OVER (PARTITION BY p.division ORDER BY p.collective__budget) AS rk,
        COUNT(*) OVER (PARTITION BY p.division) AS n FROM programs p)
  SELECT division, MAX(CASE WHEN rk = (n + 1) / 2 THEN v END) AS median, MAX(CASE WHEN rk = CAST(0.9 * (n - 1) AS INTEGER) + 1 THEN v END) AS p90, MAX(v) AS biggest,
        ROUND(AVG(base), 3) AS booster_base, SUM(big IS NOT NULL) AS big_boosters, MAX(n) AS programs FROM r GROUP BY division ORDER BY biggest DESC""", page="money"),
P("C14.by_year", "C14", "NIL budgets and boosters by year, FBS", "line",
  "The FBS mean collective budget (in $ millions) and booster money each season from the ledger (`ProgramEra.ledger`); zero before the NIL era (July 2021).",
  """SELECT l.season AS year, ROUND(AVG(l.nil_budget) / 1e6, 2) AS nil_budget_m, ROUND(AVG(l.boosters), 3) AS boosters FROM programs__era__ledger l JOIN programs p ON p._row = l._parent
   WHERE p.division = 'FBS' GROUP BY l.season ORDER BY l.season""", page="money"),
P("C14.trend", "C14", "Collectives growing, steady, falling", "bars",
  "How many collectives are growing, steady, falling or lost a big booster, and the programs drying up (`Collective.trend`, `drying_years`).",
  """SELECT p.division || ': ' || COALESCE(p.collective__trend, '?') AS label, COUNT(*) AS world, ROUND(AVG(p.collective__drying_years), 2) AS drying_years FROM programs p GROUP BY p.division, p.collective__trend ORDER BY p.division, world DESC""",
  page="money"),
P("C14.top", "C14", "The biggest collectives", "table",
  "The 30 biggest NIL collective budgets today with their trend, booster base and big booster.",
  """SELECT p.id, COALESCE(p.real_name, p.fictional_name) AS name, p.division, p.conference, ROUND(p.collective__budget / 1e6, 2) AS budget_m, p.collective__trend AS trend,
        ROUND(p.collective__base, 3) AS base, p.collective__big_booster AS big_booster FROM programs p ORDER BY p.collective__budget DESC LIMIT 30""", page="money", open="e/programs/{id}"),
]

# The conference pages: panels filed under `conference`, each cut to one conference.
CONF = [
P("conf.says", "C6", "The conference in a line", "table",
  "One line per conference for its page's verdict.",
  PS + """, last AS (SELECT * FROM ps WHERE year = (SELECT MAX(year) FROM ps))
  SELECT l.conference, printf('%s: %d members in %d (%s); champion %s; %s won the most titles in the world''s seasons.', l.conference, COUNT(*), MAX(l.year), MIN(l.division),
        COALESCE((SELECT group_concat(name, ' and ') FROM last x WHERE x.conference = l.conference AND x.ct = 1), 'none'),
        (SELECT name || ' (' || n || ')' FROM (SELECT name, SUM(ct) AS n FROM ps x WHERE x.conference = l.conference GROUP BY pid ORDER BY n DESC LIMIT 1))) AS says
   FROM last l GROUP BY l.conference""", page="conference"),
P("conf.members", "C6", "Members now", "table",
  "The conference's programs today with prestige, titles and the last season's record.",
  """SELECT p.conference, p.id, COALESCE(p.real_name, p.fictional_name) AS program, p.division, ROUND(p.prestige, 3) AS prestige, p.titles AS national_titles, p.conference_titles,
        (SELECT s.wins || '-' || s.losses FROM programs__seasons s WHERE s._parent = +p._row ORDER BY s.year DESC LIMIT 1) AS last_season
   FROM programs p ORDER BY p.conference, p.prestige DESC""", page="conference", open="e/programs/{id}"),
P("conf.standings", "C6", "The last season's standings", "table",
  "Each member's record in the world's last season, best first; its conference title marked.",
  PS + """ SELECT conference, pid AS id, name AS program, w || '-' || l AS record, ROUND(1.0 * w / NULLIF(w + l, 0), 3) AS pct, CASE WHEN nt = 1 THEN 'national champion' WHEN ct = 1 THEN 'champion' ELSE '' END AS title
   FROM ps WHERE year = (SELECT MAX(year) FROM ps) ORDER BY conference, pct DESC""", page="conference", open="e/programs/{id}"),
P("conf.champions", "C6", "Champions by season", "table",
  "Every conference title by season (shared titles each listed).",
  PS + """ SELECT conference, year, pid AS id, name AS champion, w || '-' || l AS record FROM ps WHERE ct = 1 ORDER BY conference, year DESC""", page="conference", open="e/programs/{id}"),
P("conf.titles", "C6", "Titles by member", "bars",
  "Conference titles each program won while in the conference, in the world's seasons.",
  PS + """ SELECT conference, name AS label, SUM(ct) AS world, pid AS id FROM ps GROUP BY conference, pid HAVING SUM(ct) > 0 ORDER BY conference, world DESC""", page="conference", open="e/programs/{id}"),
P("conf.members_by_year", "C6", "Members season by season", "table",
  "Who played in the conference each season (real history through 2025, the world's after): the count and the names.",
  PS + """ SELECT conference, year, COUNT(*) AS members, group_concat(name, ', ') AS programs FROM ps GROUP BY conference, year ORDER BY conference, year DESC""", page="conference"),
P("conf.strength", "C6", "Strength grade by year", "line",
  "The conference strength grade (55-99) each year, averaged over today's members (`grade_history`).",
  """SELECT p.conference, h._0 AS year, ROUND(AVG(json_extract(h._1, '$[9]')), 1) AS strength, ROUND(AVG(h2._1), 3) AS prestige FROM programs__grade_history h JOIN programs p ON p._row = h._parent
   LEFT JOIN programs__prestige_history h2 ON h2._parent = h._parent AND h2._0 = h._0 GROUP BY p.conference, h._0 ORDER BY p.conference, h._0""", page="conference"),
]

CHECKS = [
dict(id="C1.fbs_programs", page="programs", areas="C1", title="FBS programs this season", unit="",
  world=PS + " SELECT COUNT(*) FROM ps WHERE division = 'FBS' AND year = (SELECT MAX(year) FROM ps)",
  scale=1.0, years=PS + " SELECT year, COUNT(*) FROM ps WHERE division = 'FBS' GROUP BY year", detail=None,
  real=("Fixed", 138.0, "colleges.csv (FBS programs of 2026)", "2026"), tol=(0.02, 0.06, 0.0), needs=W,
  how="Programs with an FBS season in the world's last season."),
dict(id="C4.p4_drought", page="programs", areas="C4", title="Longest power-program run without a winning season", unit="seasons",
  world=PS + """, x AS (SELECT pid, (w > l) AS win, year - ROW_NUMBER() OVER (PARTITION BY pid, (w > l) ORDER BY year) AS run FROM ps WHERE """ + POWER + """)
   SELECT MAX(n) FROM (SELECT COUNT(*) AS n FROM x WHERE win = 0 GROUP BY pid, run)""",
  scale=1.0, years=None,
  detail=PS + """, x AS (SELECT pid, name, year, (w > l) AS win, year - ROW_NUMBER() OVER (PARTITION BY pid, (w > l) ORDER BY year) AS run FROM ps WHERE """ + POWER + """)
   SELECT name || ', ' || MIN(year) || '-' || MAX(year) || '.' FROM x WHERE win = 0 GROUP BY pid, run ORDER BY COUNT(*) DESC LIMIT 1""",
  real=("Key", "trial3_baselines.json:p4_droughts#longest_run_without_winning_season_2001_2025_top10.Kansas", 1.0), tol=(0.15, 0.35, 0.0), needs=W,
  how="Each power program's runs of seasons without more wins than losses (power conferences by era, Notre Dame included); the longest."),
dict(id="C4.fbs_points", page="programs", areas="C4", title="FBS points a team game", unit="pts",
  world=PS + " SELECT 1.0 * SUM(pf) / SUM(g) FROM ps WHERE division = 'FBS' AND g > 0",
  scale=1.0, years=PS + " SELECT year, 1.0 * SUM(pf) / SUM(g) FROM ps WHERE division = 'FBS' AND g > 0 GROUP BY year", detail=None,
  real=("Fixed", 27.6, "CFBD FBS games 2014-2025 (college_history.json scores_by_season)", "2014-2025"), tol=(0.05, 0.15, 0.0), needs=W,
  how="Points for over games in FBS program seasons with engine totals."),
dict(id="C5.unbeaten_champions", page="programs", areas="C5", title="FBS champions who went unbeaten", unit="%",
  world=PS + " SELECT AVG(l = 0) FROM ps WHERE division = 'FBS' AND nt = 1", scale=100.0, years=None, detail=None,
  real=("Memory", 52.0, "FBS champions 2001-2025: 13 of 25 unbeaten (Miami 2001 to Indiana 2025)"), tol=(0.15, 0.35, 0.0), needs=W,
  how="FBS national champions with no losses, over all world seasons."),
dict(id="C5.power_champions", page="programs", areas="C5", title="FBS champions from outside the power conferences", unit="%",
  world=PS + " SELECT AVG(NOT " + POWER + ") FROM ps WHERE division = 'FBS' AND nt = 1", scale=100.0, years=None,
  detail=PS + " SELECT group_concat(name || ' ' || year, ', ') FROM ps WHERE division = 'FBS' AND nt = 1 AND NOT " + POWER,
  real=("Fixed", 0.0, "college_history.json champions: every FBS champion 2001-2025 from a power conference (BYU 1984 the last outside)", "2001-2025"), tol=(0.05, 0.15, 4.0), needs=W,
  how="FBS national champions whose conference that season was not a power conference."),
dict(id="C6.conf_titles_15y", page="programs", areas="C6", title="Most conference titles by one FBS school in 15 years", unit="",
  world=PS + """, t AS (SELECT pid, year FROM ps WHERE ct = 1 AND division = 'FBS'), e AS (SELECT DISTINCT year AS e FROM ps WHERE year >= (SELECT MIN(year) + 14 FROM ps))
   SELECT MAX(n) FROM (SELECT e.e, pid, COUNT(*) AS n FROM e JOIN t ON t.year BETWEEN e.e - 14 AND e.e GROUP BY e.e, pid)""",
  scale=1.0, years=None, detail=None,
  real=("Fixed", 10.0, "college_history.json fbs_conference_champions (Boise State 2001-2015, Alabama 2011-2025, shared titles counted)", "2001-2025"), tol=(0.2, 0.4, 0.0), needs=W,
  how="Each 15-season window: the most FBS conference titles one program won; the most over all windows."),
dict(id="C7.fbs_attendance", page="programs", areas="C7", title="FBS crowd a home game", unit="",
  world=PS + """ SELECT AVG(g.attendance) FROM log_games g JOIN ps ON ps.prog = json_extract(g.teams, '$[0]') AND ps.year = g.season
   WHERE g.stream = 'college' AND ps.division = 'FBS' AND COALESCE(g.neutral, 0) = 0 AND g.attendance > 0""", scale=1.0, years=None, detail=None,
  real=("Fixed", 40160.0, "CFBD FBS home games 2021-2025 (college_attendance.csv)", "2021-2025"), tol=(0.08, 0.2, 0.0), needs=GL,
  how="Mean attendance at FBS home games (not neutral sites) in the game log."),
dict(id="C12.fbs_postseason", page="programs", areas="C12", title="FBS bowl and playoff games a season", unit="",
  world=PS + """ SELECT AVG(n) FROM (SELECT g.season, COUNT(*) AS n FROM log_games g JOIN ps ON ps.prog = json_extract(g.teams, '$[0]') AND ps.year = g.season
   WHERE g.stream = 'college' AND ps.division = 'FBS' AND g.kind IN ('bowl', 'playoff', 'title') GROUP BY g.season)""", scale=1.0, years=None, detail=None,
  real=("Fixed", 42.8, "CFBD postseason 2021-2025 (college_history.json)", "2021-2025"), tol=(0.06, 0.15, 0.0), needs=GL,
  how="FBS bowls, playoff and title games per season in the game log."),
dict(id="C19.fbs_tenure_5", page="programs", areas="C19 S7", title="FBS head coaches who last five seasons", unit="%",
  world=PS + """, r AS (SELECT pid, coach, MIN(year) AS a, COUNT(*) AS n FROM (SELECT pid, coach, year, year - ROW_NUMBER() OVER (PARTITION BY pid, coach ORDER BY year) AS run FROM ps WHERE division = 'FBS' AND coach IS NOT NULL) GROUP BY pid, coach, run)
   SELECT AVG(n >= 5) FROM r WHERE a > (SELECT MIN(year) FROM ps) AND a <= (SELECT MAX(year) FROM ps) - 10""", scale=100.0, years=None, detail=None,
  real=("Key", "college_people.json:fbs_head_coaches.share_lasting_at_least#5", 100.0), tol=(0.1, 0.25, 0.0), needs=W,
  how="Head coach runs at FBS programs that began after the history's first season and at least ten seasons before the last: the share reaching five seasons."),
dict(id="C20.fbs_qbs", page="programs", areas="C20", title="Quarterbacks on an FBS roster", unit="",
  world="""SELECT 1.0 * SUM(x.player__position = 'QB') / COUNT(DISTINCT x.college) FROM people x JOIN programs p ON p._row = x.college WHERE x.stage = 'College' AND p.division = 'FBS'""",
  scale=1.0, years=None, detail=None, real=("Fixed", 5.8, "ESPN FBS rosters 2022-24 (recruiting.rs real_roster)", "2022-2024"), tol=(0.1, 0.25, 0.0), needs=W,
  how="College quarterbacks at FBS programs today over the FBS programs with players."),
dict(id="C22.colleges_with_pick", page="programs", areas="C22", title="Colleges with a draft pick a year", unit="",
  world="""SELECT AVG(n) FROM (SELECT _0, COUNT(DISTINCT _parent) AS n FROM programs__draft_picks GROUP BY _0 ORDER BY _0 DESC LIMIT 10)""", scale=1.0, years="""SELECT _0, COUNT(DISTINCT _parent) FROM programs__draft_picks GROUP BY _0""", detail=None,
  real=("Fixed", 97.2, "nfl_careers.json draft_by_college colleges_with_a_pick_by_year, mean of 2016-2025", "2016-2025"), tol=(0.1, 0.25, 0.0), needs=W,
  how="Programs with at least one pick (each pick is a row of the program's draft picks) in each of the world's last ten drafts, averaged."),
dict(id="T13.fcs_to_fbs", page="programs", areas="T13", title="FCS programs moving up to FBS a decade", unit="",
  world=PS + """, x AS (SELECT year, division, LAG(division) OVER (PARTITION BY pid ORDER BY year) AS before FROM ps)
   SELECT 10.0 * COUNT(*) / ((SELECT MAX(year) FROM ps) - 2025) FROM x WHERE before = 'FCS' AND division = 'FBS' AND year > 2025""", scale=1.0, years=None, detail=None,
  real=("Fixed", 9.5, "college_history.json division_moves_up: 21 FCS to FBS moves 2004-2025", "2004-2025"), tol=(0.3, 0.6, 1.0), needs=W,
  how="FCS-to-FBS moves in the world's own seasons after 2025, per ten seasons."),
]

RULES = [
dict(id="C4.wins_losses", areas="C4 T11", title="College wins equal college losses each season",
  checked="SELECT COUNT(DISTINCT year) FROM programs__seasons",
  failures="""SELECT 'season', year, year, SUM(wins) || ' wins against ' || SUM(losses) || ' losses' FROM programs__seasons GROUP BY year HAVING ABS(SUM(wins) - SUM(losses)) > 0""",
  needs=W, why="Every college game has a winner and a loser among the programs (no ties since 1996)."),
dict(id="C3.grade_range", areas="C3 T11", title="Every grade within 55-99",
  checked="SELECT COUNT(*) FROM programs",
  failures="""SELECT 'program', COALESCE(p.real_name, p.fictional_name) || ' (' || p.id || ')', NULL, 'grade ' || j.key || ' is ' || ROUND(j.value, 1) FROM programs p, json_each(p.grades) j WHERE j.value < 54.5 OR j.value > 99.5""",
  needs=W, why="Grades run 55-99 within each level (`program.rs`)."),
dict(id="C42.title_news", areas="C42 C5 T11", title="Every national title has its news",
  checked="SELECT COUNT(*) FROM programs__seasons WHERE national_title = 1",
  failures="""SELECT 'program', COALESCE(p.real_name, p.fictional_name) || ' (' || p.id || ')', s.year, 'national title in ' || s.year || ' with no NationalTitle news that year'
   FROM programs__seasons s JOIN programs p ON p._row = s._parent WHERE s.national_title = 1 AND s.year >= (SELECT MIN(year) FROM news)
     AND NOT EXISTS (SELECT 1 FROM news n WHERE n.kind LIKE 'NationalTitle%' AND n.program = s._parent AND n.year BETWEEN s.year AND s.year + 1)""",
  needs=W, why="A title in the records must be in the news the same season (`news.rs`)."),
]
