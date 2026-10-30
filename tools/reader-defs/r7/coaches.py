from common import *

GL = "game log"

def P(id, area, title, kind, note, sql, says=None, needs=W, open="", page="coaches"):
    return dict(id=id, page=page, area=area, title=title, kind=kind, note=note, sql=sql, says=says, needs=needs, open=open)

# Runs of seasons one man coached a program (the season's coach, as real counts do), with how the job ended.
RUNS = PS + """, runs AS MATERIALIZED (SELECT prog, name, division, coach, MIN(year) AS a, MAX(year) AS z, COUNT(*) AS n FROM (SELECT prog, name, division, coach, year,
        year - ROW_NUMBER() OVER (PARTITION BY prog, coach ORDER BY year) AS g FROM ps WHERE coach IS NOT NULL) GROUP BY prog, coach, g),
   ended AS (SELECT r.*, (SELECT h.ended FROM coaches__history h WHERE h._parent = r.coach AND h.employer__College = r.prog AND h."to" >= r.z ORDER BY h."to" LIMIT 1) AS ended FROM runs r
        WHERE r.a > (SELECT MIN(year) FROM ps) AND r.z < (SELECT MAX(year) FROM ps))"""
NFL_RUNS = """WITH nr AS MATERIALIZED (SELECT team, coach, MIN(year) AS a, MAX(year) AS z, COUNT(*) AS n FROM (SELECT team, coach, year, year - ROW_NUMBER() OVER (PARTITION BY team, coach ORDER BY year) AS g FROM nfl_seasons) GROUP BY team, coach, g),
   nf AS (SELECT * FROM nr WHERE a > (SELECT MIN(year) FROM nfl_seasons) AND z < (SELECT MAX(year) FROM nfl_seasons))"""

# The world's last season and a coach's birth year (from his age today, a season later).
Y = "(SELECT MAX(year) FROM programs__seasons)"
# His birth year: from his birth date, else his age (a retired coach's age stops at his last job).
BY = "COALESCE(CAST(strftime('%Y', date(c.born * 86400, 'unixepoch')) AS INTEGER), CASE WHEN c.retired = 1 THEN (SELECT MAX(h.\"to\") FROM coaches__history h WHERE h._parent = +c._row) + 1 - c.age ELSE " + Y + " + 1 - c.age END)"

ROLE = """CASE WHEN {p}role = 'HeadCoach' THEN 'Head coach' WHEN {p}role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
        WHEN {p}role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN {p}role__PositionCoach IS NOT NULL THEN 'Position coach' END"""

# Every coach in a job now, with his level and role.
CJ = """WITH cj AS MATERIALIZED (SELECT c.*, CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS'
        WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level, """ + ROLE.format(p="c.job__") + """ AS role, p.prestige AS prestige
   FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL)"""

# Every finished job with its level that season (the program's real history, else today's level).
ST = """WITH st AS MATERIALIZED (SELECT h._parent AS c, h._i AS i, """ + ROLE.format(p="h.") + """ AS role, h."from" AS a, h."to" AS z, h.wins AS w, h.losses AS l, h.titles AS titles, h.ended AS ended,
        CASE WHEN h.employer__College IS NOT NULL THEN COALESCE((SELECT x._2 FROM programs__history x WHERE x._parent = h.employer__College AND h."to" BETWEEN x._0 AND x._1), p.division)
             WHEN h.employer__HighSchool IS NOT NULL THEN 'HS' WHEN h.employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level,
        h.employer__College AS college, h.employer__HighSchool AS hs, h.employer__Nfl AS nfl
   FROM coaches__history h LEFT JOIN programs p ON p._row = h.employer__College)"""

# A coach's whole career as one list: his finished jobs and the one he holds (to = the last season).
JOBS = ST + """, jobs AS MATERIALIZED (SELECT c, role, a, z, w, l, titles, ended, level, college, hs, nfl FROM st
   UNION ALL SELECT c._row, """ + ROLE.format(p="c.job__") + """, c.job__since, """ + Y + """, c.wins, c.losses, c.titles, 'current',
        CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS' WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END,
        c.job__employer__College, c.job__employer__HighSchool, c.job__employer__Nfl
   FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL)"""

GROUP_NAMES = "('QB', 'RB', 'Receivers', 'OL', 'DL', 'LB', 'DB', 'Specialists')"
TEACH = "(SELECT AVG(value) FROM json_each(ability__teaching))"
# Design means and spreads of the hidden parts by level (`world/coach.rs` generate_coach).
DESIGN = "dz(level, mean, sd) AS (VALUES ('HS', 8.0, 2.6), ('D2', 9.5, 2.3), ('D3', 9.5, 2.3), ('NAIA', 9.5, 2.3), ('JUCO', 9.5, 2.3), ('FCS', 10.5, 2.2), ('FBS', 12.0, 2.3), ('NFL', 13.5, 2.0))"
LEVEL_ORDER = "CASE level WHEN 'NFL' THEN 0 WHEN 'FBS' THEN 1 WHEN 'FCS' THEN 2 WHEN 'D2' THEN 3 WHEN 'D3' THEN 4 WHEN 'NAIA' THEN 5 WHEN 'JUCO' THEN 6 WHEN 'HS' THEN 7 ELSE 8 END"

PANELS = [
# ---------- S1 identity ----------
P("S1.ages", "S1", "Coaches' ages by level and role", "hist",
  "Every coach in a job now by age (pick a level and role above). Real FBS first-time head coaches cluster in their 40s (no open count by year).",
  CJ + """ SELECT level || ': ' || role AS grp, age AS x, COUNT(*) AS world FROM cj GROUP BY level, role, age ORDER BY """ + LEVEL_ORDER + """, role, age"""),
P("S1.age_table", "S1", "Age by level and role", "table",
  "The 10th, 50th and 90th percentile ages of coaches in a job now by level and role, and how many are ex-pros (design: HS 4%, NFL 35%, others 12%, `generate_coach`).",
  CJ + """, r AS (SELECT level, role, age AS v, ex_pro, ROW_NUMBER() OVER (PARTITION BY level, role ORDER BY age) AS r, COUNT(*) OVER (PARTITION BY level, role) AS n FROM cj)
  SELECT level, role, MAX(n) AS coaches, """ + PCT(0.1) + " AS p10, " + PCT(0.5) + " AS median, " + PCT(0.9) + """ AS p90, ROUND(100.0 * AVG(ex_pro), 1) AS ex_pro_pct,
        CASE level WHEN 'HS' THEN 4 WHEN 'NFL' THEN 35 ELSE 12 END AS design_ex_pro_pct
   FROM r GROUP BY level, role ORDER BY """ + LEVEL_ORDER + ", role"),
P("S1.first_head", "S1", "Age at the first head job", "hist",
  "Each coach's age when his first head job began (college, NFL and other leagues; high school apart), by the level of that job (pick above).",
  JOBS + """, f AS (SELECT j.c, j.level, j.a, ROW_NUMBER() OVER (PARTITION BY j.c ORDER BY j.a) AS k FROM jobs j WHERE j.role = 'Head coach' AND j.level <> 'HS')
  SELECT f.level AS grp, f.a - """ + BY + """ AS x, COUNT(*) AS world FROM f JOIN coaches c ON c._row = f.c WHERE f.k = 1 AND f.a > (SELECT MIN(year) FROM programs__seasons)
  GROUP BY f.level, x ORDER BY f.level, x"""),
P("S1.ethnicity", "S1", "Coaches by ethnicity, level and role now", "table",
  "Shares of coaches in a job now by ethnicity, for each level and role.",
  CJ + """ SELECT level, role, COUNT(*) AS coaches, ROUND(100.0 * AVG(ethnicity = 'White'), 1) AS white_pct, ROUND(100.0 * AVG(ethnicity = 'Black'), 1) AS black_pct,
        ROUND(100.0 * AVG(ethnicity = 'Hispanic'), 1) AS hispanic_pct, ROUND(100.0 * AVG(ethnicity NOT IN ('White', 'Black', 'Hispanic')), 1) AS other_pct
   FROM cj GROUP BY level, role ORDER BY """ + LEVEL_ORDER + ", role"),
P("S1.came_up", "S1", "The group each coach came up coaching, by role", "matrix",
  "Coaches in a college or NFL job now: the share of each role that came up coaching each group. Offensive coordinators should mostly come from offense, defensive coordinators from defense.",
  CJ + """ SELECT role AS row, came_up AS col, ROUND(100.0 * COUNT(*) / SUM(COUNT(*)) OVER (PARTITION BY role), 1) AS value FROM cj WHERE level <> 'HS' GROUP BY role, came_up ORDER BY role, came_up"""),
P("S1.home", "S1", "Head coaches working in their home state", "bars",
  "The share of head coaches now whose job is in the state he comes from (hidden home state), by level.",
  CJ + """ SELECT level AS label, ROUND(100.0 * AVG(cj.home_state = COALESCE((SELECT state FROM programs WHERE _row = cj.job__employer__College), (SELECT state FROM high_schools WHERE _row = cj.job__employer__HighSchool))), 1) AS world, COUNT(*) AS n
   FROM cj WHERE role = 'Head coach' AND level <> 'NFL' AND cj.home_state IS NOT NULL GROUP BY level ORDER BY """ + LEVEL_ORDER),
# ---------- S2 hidden ratings ----------
P("S2.levels", "S2", "Hidden ratings by level and role (debug)", "table",
  "Each level and role now: head coach value (`head_coach_value`), mean teaching, scheme, game management, recruiting and network, against the design's mean and spread for the level.",
  CJ + ", " + DESIGN + """ SELECT cj.level, cj.role, COUNT(*) AS coaches, ROUND(AVG(""" + HCV + """), 1) AS hc_value, ROUND(AVG(""" + TEACH + """), 1) AS teaching, ROUND(AVG(ability__scheme), 1) AS scheme,
        ROUND(AVG(ability__game_management), 1) AS game_management, ROUND(AVG(ability__recruiting), 1) AS recruiting, ROUND(AVG(ability__network), 1) AS network, dz.mean AS design_mean, dz.sd AS design_sd
   FROM cj LEFT JOIN dz ON dz.level = cj.level GROUP BY cj.level, cj.role ORDER BY """ + LEVEL_ORDER.replace("level", "cj.level") + ", cj.role"),
P("S2.value_wins", "S2", "Head coach value against winning (debug)", "bars",
  "Head coaches with 30+ college games as head coach: their win share by head coach value now, in bands. Talent drives more, so the link should be positive but loose.",
  JOBS + """, w AS (SELECT c, SUM(w) AS w, SUM(l) AS l FROM jobs WHERE role = 'Head coach' AND college IS NOT NULL GROUP BY c HAVING SUM(w + l) >= 30)
  SELECT CASE WHEN v < 9 THEN 'Value under 9' WHEN v < 11 THEN '9-11' WHEN v < 13 THEN '11-13' WHEN v < 15 THEN '13-15' ELSE '15+' END AS label, ROUND(1.0 * SUM(w.w) / SUM(w.w + w.l), 3) AS world, COUNT(*) AS n
   FROM w JOIN (SELECT _row, """ + HCV + """ AS v FROM coaches) c ON c._row = w.c GROUP BY 1 ORDER BY MIN(v)"""),
P("S2.best_teachers", "S2", "Best teacher of each group by level (debug)", "table",
  "The best teacher of each group among coaches in a job now at each level, with his teaching and his job (his rooms' growth is on the unit records).",
  CJ + ", " + GROUPS + """, t AS (SELECT cj._row AS id, cj.name, cj.level, cj.role, g.key AS k, g.value AS v, ROW_NUMBER() OVER (PARTITION BY cj.level, g.key ORDER BY g.value DESC) AS rk FROM cj, json_each(cj.ability__teaching) g WHERE cj.level IN ('NFL', 'FBS', 'FCS', 'HS'))
  SELECT t.level, grp.grp AS grp_taught, t.id, t.name, t.role, ROUND(t.v, 1) AS teaching FROM t JOIN grp ON grp.k = t.k
   WHERE t.rk = 1 ORDER BY CASE t.level WHEN 'NFL' THEN 0 WHEN 'FBS' THEN 1 WHEN 'FCS' THEN 2 ELSE 3 END, t.k""", open="e/coaches/{id}"),
P("S2.recruiters", "S2", "The best recruiters and where they work (debug)", "table",
  "The 40 best recruiters in a job now. Recruiting is not in head coach value, so an elite recruiter with a low value shows whether big programs still hire him.",
  CJ + """ SELECT _row AS id, name, level, role, ROUND(ability__recruiting, 1) AS recruiting, ROUND(""" + HCV + """, 1) AS hc_value, ROUND(prestige, 2) AS prestige
   FROM cj ORDER BY ability__recruiting DESC LIMIT 40""", open="e/coaches/{id}"),
P("S2.sportsmanship", "S2", "Sportsmanship spread (debug)", "hist",
  "Every coach's sportsmanship (design: 10 + 3.5 a spread), which sets when starters come out in blowouts.",
  """SELECT ROUND(sportsmanship) AS x, COUNT(*) AS world FROM coaches WHERE sportsmanship IS NOT NULL GROUP BY 1 ORDER BY 1"""),
# ---------- S3 ageing ----------
P("S3.by_age", "S3", "Ratings by age (debug)", "line",
  "Mean hidden parts by age over every coach-year in the world log (`log_coaches`): rising under 35, flat in the late 40s and 50s, falling after about 59, recruiting first.",
  """SELECT coach__age AS year, ROUND(AVG((coach__ability__teaching__0 + coach__ability__teaching__1 + coach__ability__teaching__2 + coach__ability__teaching__3 + coach__ability__teaching__4
        + coach__ability__teaching__5 + coach__ability__teaching__6 + coach__ability__teaching__7) / 8.0), 2) AS teaching, ROUND(AVG(coach__ability__scheme), 2) AS scheme,
        ROUND(AVG(coach__ability__game_management), 2) AS game_management, ROUND(AVG(coach__ability__recruiting), 2) AS recruiting, ROUND(AVG(coach__ability__network), 2) AS network, COUNT(*) AS n
   FROM log_coaches WHERE coach__age BETWEEN 24 AND 80 AND COALESCE(coach__retired, 0) = 0 GROUP BY coach__age ORDER BY coach__age""", needs=WL),
P("S3.old", "S3", "Coaches still working by age", "bars",
  "Of all coaches of each age band, the share in a job now; out-of-work coaches over 66 retire.",
  """SELECT CASE WHEN age < 40 THEN 'Under 40' WHEN age < 50 THEN '40-49' WHEN age < 60 THEN '50-59' WHEN age < 65 THEN '60-64' WHEN age < 70 THEN '65-69' ELSE '70+' END AS label,
        ROUND(100.0 * AVG(job__role IS NOT NULL OR job__role__PositionCoach IS NOT NULL), 1) AS world, ROUND(100.0 * AVG(COALESCE(retired, 0)), 1) AS retired_pct, COUNT(*) AS n
   FROM coaches GROUP BY 1 ORDER BY MIN(age)"""),
P("S3.ceiling", "S3", "Room left to grow (debug)", "bars",
  "Coaches in a job by age band: the mean room between his scheme, game management and mean teaching and his learning ceiling (growth only fills room under the ceiling; a part drawn above it stays), and the share with no room left in any of the three.",
  CJ + """, x AS (SELECT age, learning_ceiling - ability__scheme AS a, learning_ceiling - ability__game_management AS b, learning_ceiling - """ + TEACH + """ AS t FROM cj)
  SELECT CASE WHEN age < 35 THEN 'Under 35' WHEN age < 45 THEN '35-44' WHEN age < 55 THEN '45-54' ELSE '55+' END AS label,
        ROUND(AVG((MAX(a, 0) + MAX(b, 0) + MAX(t, 0)) / 3.0), 2) AS world, ROUND(100.0 * AVG(a <= 0.5 AND b <= 0.5 AND t <= 0.5), 1) AS no_room_pct, COUNT(*) AS n
   FROM x GROUP BY 1 ORDER BY MIN(age)"""),
# ---------- S4 personality ----------
P("S4.loyalty", "S4", "Loyalty against moving on", "bars",
  "Coaches with 10+ seasons of jobs by loyalty band: jobs per decade and the share of jobs that ended with him leaving for another job.",
  JOBS + """, x AS (SELECT c, COUNT(*) AS jobs, SUM(ended = 'left for another job') AS left_for, MAX(z) - MIN(a) + 1 AS span FROM jobs GROUP BY c HAVING span >= 10)
  SELECT CASE WHEN co.personality__loyalty < 7 THEN 'Loyalty under 7' WHEN co.personality__loyalty < 10 THEN '7-10' WHEN co.personality__loyalty < 13 THEN '10-13' ELSE '13+' END AS label,
        ROUND(AVG(10.0 * x.jobs / x.span), 2) AS world, ROUND(100.0 * SUM(x.left_for) / SUM(x.jobs), 1) AS left_for_pct, COUNT(*) AS n
   FROM x JOIN coaches co ON co._row = x.c GROUP BY 1 ORDER BY MIN(co.personality__loyalty)"""),
P("S4.ambition", "S4", "Ambition against climbing", "bars",
  "Coaches with 10+ seasons of jobs by ambition band: the share who ever held an FBS or NFL head job.",
  JOBS + """, x AS (SELECT c, MAX(role = 'Head coach' AND level IN ('FBS', 'NFL')) AS top, MAX(z) - MIN(a) + 1 AS span FROM jobs GROUP BY c HAVING span >= 10)
  SELECT CASE WHEN co.personality__ambition < 7 THEN 'Ambition under 7' WHEN co.personality__ambition < 10 THEN '7-10' WHEN co.personality__ambition < 13 THEN '10-13' ELSE '13+' END AS label,
        ROUND(100.0 * AVG(x.top), 1) AS world, COUNT(*) AS n FROM x JOIN coaches co ON co._row = x.c GROUP BY 1 ORDER BY MIN(co.personality__ambition)"""),
P("S4.temper", "S4", "Temper against health episodes", "bars",
  "Head coaches and coordinators in a job now by temper band: health episodes per 100 coaches in the last 400 days (`folk.life.health`).",
  CJ + """ SELECT CASE WHEN personality__temper < 7 THEN 'Temper under 7' WHEN personality__temper < 10 THEN '7-10' WHEN personality__temper < 13 THEN '10-13' ELSE '13+' END AS label,
        ROUND(100.0 * SUM((SELECT COUNT(*) FROM folk__life__health h WHERE h.coach = +cj._row)) / COUNT(*), 2) AS world, COUNT(*) AS n
   FROM cj WHERE role <> 'Position coach' GROUP BY 1 ORDER BY MIN(personality__temper)"""),
P("S4.traits", "S4", "Ambition, loyalty and temper", "hist",
  "Each trait's spread over every coach (design N(10, 3.5)); pick a trait above.",
  """SELECT 'Ambition' AS grp, ROUND(personality__ambition) AS x, COUNT(*) AS world FROM coaches GROUP BY 2
   UNION ALL SELECT 'Loyalty', ROUND(personality__loyalty), COUNT(*) FROM coaches GROUP BY 2
   UNION ALL SELECT 'Temper', ROUND(personality__temper), COUNT(*) FROM coaches GROUP BY 2 ORDER BY 1, 2"""),
# ---------- S5 reputation ----------
P("S5.gap_by_tenure", "S5", "Reputation against true value by seasons as head coach (debug)", "bars",
  "College head coaches now: the mean gap between reputation and head coach value (absolute) by seasons as a head coach. Reputation chases value, so the gap should shrink.",
  JOBS + """, t AS (SELECT c, SUM(z - a + 1) AS seasons FROM jobs WHERE role = 'Head coach' AND college IS NOT NULL GROUP BY c)
  SELECT CASE WHEN t.seasons <= 2 THEN '1-2 seasons' WHEN t.seasons <= 5 THEN '3-5' WHEN t.seasons <= 10 THEN '6-10' ELSE '11+' END AS label,
        ROUND(AVG(ABS(co.reputation - (""" + HCV.replace("ability__", "co.ability__") + """))), 2) AS world, COUNT(*) AS n
   FROM t JOIN coaches co ON co._row = t.c WHERE co.job__role = 'HeadCoach' AND co.job__employer__College IS NOT NULL GROUP BY 1 ORDER BY MIN(t.seasons)"""),
P("S5.over_under", "S5", "The most overrated and underrated FBS head coaches (debug)", "table",
  "FBS head coaches now ranked by reputation less head coach value: the top is overrated, the bottom underrated, each with his record and last six seasons against expectation.",
  CJ + """ SELECT _row AS id, name, (SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = cj.job__employer__College) AS program, ROUND(reputation, 1) AS reputation,
        ROUND(""" + HCV + """, 1) AS hc_value, ROUND(reputation - (""" + HCV + """), 1) AS gap, wins || '-' || losses AS record_here, career_titles AS titles, recent AS last_six_vs_expectation
   FROM cj WHERE level = 'FBS' AND role = 'Head coach' ORDER BY gap DESC""", open="e/coaches/{id}"),
P("S5.by_year", "S5", "Reputation of FBS head coaches by year", "line",
  "The world log's yearly coach rows: the 10th, 50th and 90th percentile reputation of FBS head coaches each year.",
  """WITH x AS (SELECT l.season AS year, l.coach__reputation AS v FROM log_coaches l JOIN programs p ON p._row = l.coach__job__employer__College WHERE l.coach__job__role = 'HeadCoach' AND p.division = 'FBS'),
   r AS (SELECT year, v, ROW_NUMBER() OVER (PARTITION BY year ORDER BY v) AS r, COUNT(*) OVER (PARTITION BY year) AS n FROM x)
  SELECT year, """ + PCT(0.1) + " AS p10, " + PCT(0.5) + " AS p50, " + PCT(0.9) + """ AS p90 FROM r GROUP BY year ORDER BY year""", needs=WL),
P("S5.words", "S5", "Reputation words by level", "matrix",
  "The words the game shows for each coach in a job now (17+ 'one of the best in the game', 14-17 'highly regarded', 10-14 'respected', 5-10 'unproven' bands, under 5 'unproven'; `rep_words`), as shares of each level.",
  CJ + """ SELECT level AS row, CASE WHEN reputation >= 17 THEN '17+' WHEN reputation >= 14 THEN '14-17' WHEN reputation >= 10 THEN '10-14' WHEN reputation >= 5 THEN '5-10' ELSE 'under 5' END AS col,
        ROUND(100.0 * COUNT(*) / SUM(COUNT(*)) OVER (PARTITION BY level), 1) AS value FROM cj GROUP BY 1, 2 ORDER BY """ + LEVEL_ORDER + ", 2"""),
# ---------- S6 buzz ----------
P("S6.leaders", "S6", "Buzz from unit stats", "table",
  "Every buzz row, newest first: the coach, his program, the side, the gain and its why (`stats.buzz`), and his next job after that season.",
  JOBS + """ SELECT b.year, b.coach AS id, c.name AS coach, (SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = b.program) AS program, CASE WHEN b.offense = 1 THEN 'offense' ELSE 'defense' END AS side,
        ROUND(b.gain, 2) AS gain, b.why, (SELECT j.role || ' at ' || COALESCE((SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = j.college), j.level) || ' (' || j.a || ')' FROM jobs j
          WHERE j.c = b.coach AND j.a > b.year ORDER BY j.a LIMIT 1) AS next_job
   FROM stats__buzz b JOIN coaches c ON c._row = b.coach ORDER BY b.year DESC, b.gain DESC""", open="e/coaches/{id}"),
P("S6.share", "S6", "Units that earned buzz a year", "line",
  "Each year: coaches gaining buzz, losing it, and the mean gain (`stats.buzz`). About a quarter of units should earn some.",
  """SELECT year, SUM(gain > 0) AS gained, SUM(gain < 0) AS lost, ROUND(AVG(gain), 3) AS mean_gain FROM stats__buzz GROUP BY year ORDER BY year"""),
P("S6.next", "S6", "Buzz and the next head job", "bars",
  "Coordinators in a buzz row by their gain that year: the share who became a head coach within two years.",
  """WITH x AS (SELECT b.coach, b.year, b.gain, EXISTS (SELECT 1 FROM coaches__history h WHERE h._parent = b.coach AND h.role = 'HeadCoach' AND h."from" BETWEEN b.year + 1 AND b.year + 2)
        OR EXISTS (SELECT 1 FROM coaches c WHERE c._row = b.coach AND c.job__role = 'HeadCoach' AND c.job__since BETWEEN b.year + 1 AND b.year + 2) AS head FROM stats__buzz b WHERE b.year <= """ + Y + """ - 2)
  SELECT CASE WHEN gain >= 2 THEN 'Gain 2+' WHEN gain >= 1 THEN '1-2' WHEN gain > 0 THEN 'under 1' ELSE 'lost buzz' END AS label, ROUND(100.0 * AVG(head), 1) AS world, COUNT(*) AS n FROM x GROUP BY 1 ORDER BY MIN(gain) DESC"""),
# ---------- S7 jobs and endings ----------
P("S7.tenure", "S7", "How long head coaches last", "bars",
  "Finished FBS head coaching runs (seasons as the season's coach in a row at one program) that began after the world's first season: the share lasting at least 2, 3, 4, 5, 6, 8 and 10 seasons, against real 2005-2019 starts (`college_people.json`).",
  RUNS + """, k(k) AS (VALUES (2), (3), (4), (5), (6), (8), (10))
  SELECT 'At least ' || k.k || ' seasons' AS label, ROUND(100.0 * AVG(e.n >= k.k), 1) AS world,
        100.0 * json_extract(""" + RD("college_people.json") + """, '$.fbs_head_coaches.share_lasting_at_least."' || k.k || '"') AS real, COUNT(*) AS n FROM ended e, k WHERE e.division = 'FBS' GROUP BY k.k ORDER BY k.k""",
  says=RUNS + """, r AS (SELECT n AS v, ROW_NUMBER() OVER (ORDER BY n) AS r, COUNT(*) OVER () AS n FROM ended WHERE division = 'FBS')
  SELECT printf('Finished FBS head coaching runs last %d seasons at the median (real 4); %.0f%% reach five (real 43%%).', """ + PCT(0.5) + """, 100.0 * AVG(v >= 5)) FROM r HAVING COUNT(*) > 0"""),
P("S7.endings", "S7", "How jobs ended, by level", "matrix",
  "Every finished job by its level and how it ended, as shares of the level's endings.",
  ST + """ SELECT level AS row, ended AS col, ROUND(100.0 * COUNT(*) / SUM(COUNT(*)) OVER (PARTITION BY level), 1) AS value FROM st GROUP BY level, ended ORDER BY """ + LEVEL_ORDER + ", value DESC"),
P("S7.endings_by_year", "S7", "FBS head jobs ended a year, by how", "line",
  "Each season: FBS head jobs that ended, by how (fired, left for another job, retired, other).",
  ST + """ SELECT z AS year, SUM(ended LIKE 'fired%') AS fired, SUM(ended = 'left for another job') AS left_for_another_job, SUM(ended LIKE 'retired%' OR ended LIKE 'stepped away%' OR ended LIKE 'walked away%') AS retired,
        SUM(NOT (ended LIKE 'fired%' OR ended = 'left for another job' OR ended LIKE 'retired%' OR ended LIKE 'stepped away%' OR ended LIKE 'walked away%')) AS other
   FROM st WHERE role = 'Head coach' AND level = 'FBS' GROUP BY z ORDER BY z"""),
P("S7.win_by_end", "S7", "Win share by how a head job ended", "bars",
  "College head jobs of 20+ games by how they ended: their win share. Fired coaches should sit well below those hired away.",
  ST + """ SELECT ended AS label, ROUND(1.0 * SUM(w) / SUM(w + l), 3) AS world, COUNT(*) AS n FROM st WHERE role = 'Head coach' AND college IS NOT NULL AND w + l >= 20 GROUP BY ended ORDER BY world DESC"""),
P("S7.kept_losing", "S7", "Kept while losing", "table",
  "FBS head jobs of 55+ games under .400: they should be rare.",
  JOBS + """ SELECT j.c AS id, c.name, (SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = j.college) AS program, j.a || '-' || j.z AS seasons, j.w || '-' || j.l AS record,
        ROUND(1.0 * j.w / (j.w + j.l), 3) AS pct, j.ended FROM jobs j JOIN coaches c ON c._row = j.c WHERE j.role = 'Head coach' AND j.level = 'FBS' AND j.w + j.l >= 55 AND 1.0 * j.w / (j.w + j.l) < 0.4 ORDER BY pct""",
  open="e/coaches/{id}"),
P("S7.titles", "S7", "The most titled head coaches", "table",
  "Head coaches by national titles in the world's seasons (the season's coach), with the most at one program in any 15 seasons. Real: Saban 7 in his career; no program more than 6 in 15 years.",
  PS + """, t AS (SELECT coach, pid, name, year, division FROM ps WHERE nt = 1 AND coach IS NOT NULL),
   w AS (SELECT a.coach, a.pid, COUNT(b.year) AS n FROM t a JOIN t b ON b.coach = a.coach AND b.pid = a.pid AND b.year BETWEEN a.year AND a.year + 14 GROUP BY a.coach, a.pid, a.year)
  SELECT t.coach AS id, c.name, t.division, COUNT(*) AS titles, group_concat(t.name || ' ' || t.year, ', ') AS seasons, (SELECT MAX(n) FROM w WHERE w.coach = t.coach) AS most_at_one_school_in_15
   FROM t JOIN coaches c ON c._row = t.coach GROUP BY t.coach, t.division ORDER BY titles DESC LIMIT 50""", open="e/coaches/{id}"),
P("S7.list", "S7", "Every coach", "table",
  "Every coach: his job now, career record as head coach, titles, reputation and age. A row opens his page with every job and why it ended.",
  JOBS + """, r AS (SELECT c, SUM(CASE WHEN role = 'Head coach' THEN w ELSE 0 END) AS w, SUM(CASE WHEN role = 'Head coach' THEN l ELSE 0 END) AS l, COUNT(*) AS jobs FROM jobs GROUP BY c)
  SELECT c._row AS id, c.name, c.age, COALESCE(""" + ROLE.format(p="c.job__") + """, CASE WHEN c.retired = 1 THEN 'retired' ELSE 'out of work' END) AS job,
        COALESCE((SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = c.job__employer__College), (SELECT name FROM high_schools WHERE _row = c.job__employer__HighSchool), (SELECT name FROM nfl WHERE _row = c.job__employer__Nfl)) AS at,
        r.w || '-' || r.l AS head_record, c.career_titles AS titles, r.jobs, ROUND(c.reputation, 1) AS reputation
   FROM coaches c LEFT JOIN r ON r.c = c._row WHERE r.jobs > 0 ORDER BY c.reputation DESC""", open="e/coaches/{id}"),
# ---------- S8 the carousel ----------
P("S8.fbs_changes", "S8", "New FBS head coaches a season against real", "line",
  "Each season: FBS programs whose head coach (the season's coach) differs from last season's, against real 2005-2025 (`college_people.json` changes_by_season).",
  PS + """, x AS (SELECT year, pid, coach, LAG(coach) OVER (PARTITION BY pid ORDER BY year) AS before, division FROM ps)
  SELECT x.year, SUM(x.coach IS NOT x.before AND x.before IS NOT NULL) AS world,
        json_extract(""" + RD("college_people.json") + """, '$.fbs_head_coaches.changes_by_season."' || x.year || '".new_head_coach') AS real
   FROM x WHERE x.division = 'FBS' AND x.year > (SELECT MIN(year) FROM ps) GROUP BY x.year ORDER BY x.year""",
  says=PS + """, x AS (SELECT year, pid, coach, LAG(coach) OVER (PARTITION BY pid ORDER BY year) AS before, division FROM ps)
  SELECT printf('%.1f new FBS head coaches a season (%.1f%% of programs); real 2005-2025: 20.3 (17.6%%).', 1.0 * SUM(coach IS NOT before) / COUNT(DISTINCT year), 100.0 * AVG(coach IS NOT before))
   FROM x WHERE division = 'FBS' AND before IS NOT NULL"""),
P("S8.nfl_changes", "S8", "NFL head coach changes a season against real", "line",
  "Each season: NFL clubs whose head coach differs from the season before (`nfl_seasons`), against real offseason and in-season changes (`nfl_history.json`, 2000-2025).",
  """WITH x AS (SELECT year, team, coach, LAG(coach) OVER (PARTITION BY team ORDER BY year) AS before FROM nfl_seasons)
  SELECT year, SUM(coach IS NOT before AND before IS NOT NULL) AS world,
        json_extract(""" + RD("nfl_history.json") + """, '$.head_coaches.offseason_changes_by_season."' || year || '"') AS real
   FROM x WHERE year > (SELECT MIN(year) FROM nfl_seasons) GROUP BY year ORDER BY year"""),
P("S8.sources", "S8", "Where new FBS head coaches came from", "bars",
  "New FBS head coaches' previous job, by role and level, over the world's seasons.",
  JOBS + """, n AS (SELECT c, a, role, level, LAG(role) OVER (PARTITION BY c ORDER BY a, z) AS prev_role, LAG(level) OVER (PARTITION BY c ORDER BY a, z) AS prev_level FROM jobs)
  SELECT COALESCE(prev_level || ' ' || lower(prev_role), 'first job') AS label, COUNT(*) AS world FROM n WHERE role = 'Head coach' AND level = 'FBS' AND a > (SELECT MIN(year) FROM programs__seasons)
  GROUP BY 1 ORDER BY world DESC LIMIT 20"""),
P("S8.news", "S8", "Coaching news a year", "line",
  "Each year's hires, firings, departures and extensions in the news.",
  """SELECT year, SUM(kind = 'CoachHired') AS hired, SUM(kind = 'CoachFired') AS fired, SUM(kind = 'CoachLeft') AS left, SUM(kind = 'CoachExtended') AS extended FROM news WHERE kind LIKE 'Coach%' GROUP BY year ORDER BY year"""),
P("S8.cycle", "S8", "The last cycle's moves", "table",
  "Every head job that began in the world's last season or the one now under way, with the coach's job before it and the news line that says why.",
  JOBS + """, n AS (SELECT c, a, role, level, college, hs, nfl, LAG(role) OVER (PARTITION BY c ORDER BY a, z) AS prev_role, LAG(level) OVER (PARTITION BY c ORDER BY a, z) AS prev_level,
        LAG(college) OVER (PARTITION BY c ORDER BY a, z) AS prev_college FROM jobs)
  SELECT n.a AS year, n.c AS id, co.name, n.level, COALESCE((SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = n.college), (SELECT name FROM nfl WHERE _row = n.nfl), 'high school') AS job,
        COALESCE(n.prev_level || ' ' || lower(n.prev_role), '') || COALESCE(' at ' || (SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = n.prev_college), '') AS before
   FROM n JOIN coaches co ON co._row = n.c WHERE n.role = 'Head coach' AND n.level <> 'HS' AND n.a >= """ + Y + """ ORDER BY """ + LEVEL_ORDER.replace("level", "n.level") + """, n.a DESC""", open="e/coaches/{id}"),
P("S8.fcs_up", "S8", "FCS title coaches who moved up", "table",
  "FCS head coaches who won a title and later took an FBS head job (the Cignetti path): they exist but are few.",
  PS + ", " + JOBS[len("WITH "):] + """ SELECT DISTINCT ps.coach AS id, c.name, ps.name AS fcs_program, ps.year AS title_year, j.a AS fbs_from, (SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = j.college) AS fbs_program
   FROM ps JOIN jobs j ON j.c = ps.coach AND j.role = 'Head coach' AND j.level = 'FBS' AND j.a > ps.year JOIN coaches c ON c._row = ps.coach WHERE ps.division = 'FCS' AND ps.nt = 1 ORDER BY ps.year""",
  open="e/coaches/{id}"),
# ---------- S9 hiring ----------
P("S9.searches", "S9", "Every search: who was called and who said yes", "table",
  "The world log's searches (`Search` whys), newest first: the job, its prestige, how many candidates were called, the hire and the why.",
  """SELECT w.season AS year, COALESCE((SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = w.why__Search__employer__College), (SELECT name FROM nfl WHERE _row = w.why__Search__employer__Nfl),
          (SELECT name FROM high_schools WHERE _row = w.why__Search__employer__HighSchool), 'other league') AS employer,
        COALESCE(w.why__Search__role, 'position coach') AS role, ROUND(w.why__Search__prestige, 2) AS prestige, (SELECT COUNT(*) FROM log_whys__why__Search__called k WHERE k._parent = +w._row) AS called,
        w.why__Search__hired AS id, (SELECT name FROM coaches WHERE _row = w.why__Search__hired) AS hired, w.why__Search__why AS why
   FROM log_whys w WHERE w.why__Search__why IS NOT NULL AND (w.why__Search__employer__College IS NOT NULL OR w.why__Search__employer__Nfl IS NOT NULL) ORDER BY w.season DESC, w.why__Search__prestige DESC""",
  needs=WL, open="e/coaches/{id}"),
P("S9.own_tree", "S9", "Coordinators hired from the head coach's own tree", "line",
  "Each season: new FBS coordinators whose mentor is the program's head coach that season (a tree bonus of 1.5 in the score), as a share.",
  PS + ", " + JOBS[len("WITH "):] + """ SELECT j.a AS year, ROUND(100.0 * AVG(co.mentor = ps.coach), 1) AS world, COUNT(*) AS n
   FROM jobs j JOIN coaches co ON co._row = j.c JOIN ps ON ps.prog = j.college AND ps.year = j.a WHERE j.role IN ('Offensive coordinator', 'Defensive coordinator') AND ps.division = 'FBS' GROUP BY j.a ORDER BY j.a"""),
P("S9.pipeline", "S9", "The pipeline: coaches of color among FBS coordinators and head coaches", "line",
  "Each season: the share of non-White coaches among FBS coordinators (their jobs covering the season) and among FBS head coaches (the season's coach). The gap is the hiring pipeline.",
  PS + ", " + JOBS[len("WITH "):] + """, co AS (SELECT ps.year, AVG(c.ethnicity <> 'White') AS v FROM jobs j JOIN ps ON ps.prog = j.college AND ps.year BETWEEN j.a AND j.z JOIN coaches c ON c._row = j.c
        WHERE j.role IN ('Offensive coordinator', 'Defensive coordinator') AND ps.division = 'FBS' GROUP BY ps.year),
   hc AS (SELECT ps.year, AVG(c.ethnicity <> 'White') AS v FROM ps JOIN coaches c ON c._row = ps.coach WHERE ps.division = 'FBS' GROUP BY ps.year)
  SELECT hc.year, ROUND(100.0 * co.v, 1) AS coordinators, ROUND(100.0 * hc.v, 1) AS head_coaches FROM hc LEFT JOIN co ON co.year = hc.year ORDER BY hc.year"""),
# ---------- S10 diversity ----------
P("S10.black_heads", "S10", "Black FBS head coaches by year against real", "line",
  "Each season's FBS head coaches (the season's coach) who are Black, as a share, against the real counts: 7 in 2009, 14 of 133 in 2023, 16 of 134 in 2024 (CBS Sports; `trial3-baselines.md`).",
  PS + """ SELECT ps.year, ROUND(100.0 * AVG(c.ethnicity = 'Black'), 1) AS world, SUM(c.ethnicity = 'Black') AS black, COUNT(*) AS n,
        CASE ps.year WHEN 2009 THEN 5.9 WHEN 2023 THEN 10.5 WHEN 2024 THEN 11.9 END AS real
   FROM ps JOIN coaches c ON c._row = ps.coach WHERE ps.division = 'FBS' GROUP BY ps.year ORDER BY ps.year"""),
P("S10.nfl", "S10", "NFL head coaches by ethnicity by year", "line",
  "Each season's NFL head coaches (`nfl_seasons` coach) who are not White, as a count.",
  """SELECT s.year, SUM(c.ethnicity <> 'White') AS of_color, SUM(c.ethnicity = 'Black') AS black, COUNT(*) AS n FROM nfl_seasons s JOIN coaches c ON c._row = s.coach GROUP BY s.year ORDER BY s.year"""),
P("S10.reports", "S10", "The yearly diversity reports", "table",
  "The news the world writes each year about head coaches of color.",
  """SELECT year, text FROM news WHERE kind = 'DiversityReport' ORDER BY year DESC"""),
# ---------- S11 trees ----------
P("S11.trees", "S11", "The biggest coaching trees", "table",
  "Coaches by the size of their tree: direct protégés (his mentor link), protégés who held a head job, protégés who are college head coaches now and grand-protégés. Real big trees (Saban's, Walsh's) run to a dozen or more head coaches (from memory).",
  """WITH k AS (SELECT mentor, COUNT(*) AS kids, SUM(job__role = 'HeadCoach' AND job__employer__College IS NOT NULL) AS college_heads_now FROM coaches WHERE mentor IS NOT NULL GROUP BY mentor),
   h AS (SELECT c.mentor, COUNT(DISTINCT c._row) AS heads FROM coaches c JOIN coaches__history s ON s._parent = c._row WHERE c.mentor IS NOT NULL AND s.role = 'HeadCoach' AND s.employer__HighSchool IS NULL GROUP BY c.mentor),
   g AS (SELECT p.mentor, COUNT(*) AS grand FROM coaches k2 JOIN coaches p ON p._row = k2.mentor WHERE p.mentor IS NOT NULL GROUP BY p.mentor)
  SELECT c._row AS id, c.name, k.kids AS proteges, COALESCE(h.heads, 0) AS proteges_with_head_jobs, k.college_heads_now, COALESCE(g.grand, 0) AS grand_proteges, c.career_titles AS titles,
        CASE WHEN c.retired = 1 THEN 'retired' ELSE COALESCE(""" + ROLE.format(p="c.job__") + """, 'out of work') END AS now
   FROM k JOIN coaches c ON c._row = k.mentor LEFT JOIN h ON h.mentor = k.mentor LEFT JOIN g ON g.mentor = k.mentor ORDER BY proteges_with_head_jobs DESC, k.kids DESC LIMIT 60""", open="e/coaches/{id}"),
P("S11.title_mentors", "S11", "New head coaches with a title-winning mentor", "line",
  "Each season: new college head coaches whose mentor won a national title as a head coach, as a share.",
  JOBS + """ SELECT j.a AS year, ROUND(100.0 * AVG(COALESCE(m.career_titles, 0) > 0), 1) AS world, COUNT(*) AS n FROM jobs j JOIN coaches c ON c._row = j.c LEFT JOIN coaches m ON m._row = c.mentor
   WHERE j.role = 'Head coach' AND j.college IS NOT NULL AND j.a > (SELECT MIN(year) FROM programs__seasons) GROUP BY j.a ORDER BY j.a"""),
# ---------- S12 unit records ----------
P("S12.reads", "S12", "Unit records by group", "table",
  "Coaches with 8+ seasons of credit in a group: the spread of their public growth read (growth / (seasons + 8)) and how many moved up, and (debug) how closely the read follows true teaching.",
  "WITH " + GROUPS + """, x AS (SELECT u._i AS k, u.growth / (u.seasons + 8) AS v, u.moved_up, json_extract(c.ability__teaching, '$[' || u._i || ']') AS t FROM coaches__units u JOIN coaches c ON c._row = u._parent WHERE u.seasons >= 8),
   r AS (SELECT k, v, moved_up, t, ROW_NUMBER() OVER (PARTITION BY k ORDER BY v) AS r, COUNT(*) OVER (PARTITION BY k) AS n FROM x),
   s AS (SELECT k, AVG(v) AS mv, AVG(t) AS mt, AVG(v * v) - AVG(v) * AVG(v) AS vv, AVG(t * t) - AVG(t) * AVG(t) AS vt, AVG(v * t) - AVG(v) * AVG(t) AS cv FROM x GROUP BY k)
  SELECT grp.grp AS grp_taught, MAX(r.n) AS coaches, ROUND(""" + PCT(0.1) + ", 3) AS p10, ROUND(" + PCT(0.5) + ", 3) AS median, ROUND(" + PCT(0.9) + """, 3) AS p90, ROUND(AVG(r.moved_up), 2) AS moved_up,
        ROUND(s.cv / sqrt(s.vv * s.vt), 2) AS truth_corr_debug
   FROM r JOIN s ON s.k = r.k JOIN grp ON grp.k = r.k GROUP BY r.k ORDER BY r.k"""),
P("S12.read_vs_truth", "S12", "The public record against true teaching (debug)", "bars",
  "Coaches with 8+ seasons in a group, by their true teaching of it in bands: the mean growth read (pick a group above). It should rise with the truth, noisily.",
  "WITH " + GROUPS + """ SELECT grp.grp AS grp, CASE WHEN t < 8 THEN 'Teaching under 8' WHEN t < 11 THEN '8-11' WHEN t < 14 THEN '11-14' WHEN t < 17 THEN '14-17' ELSE '17+' END AS label, ROUND(AVG(v), 3) AS world, COUNT(*) AS n
   FROM (SELECT u._i AS k, u.growth / (u.seasons + 8) AS v, json_extract(c.ability__teaching, '$[' || u._i || ']') AS t FROM coaches__units u JOIN coaches c ON c._row = u._parent WHERE u.seasons >= 8) x
   JOIN grp ON grp.k = x.k GROUP BY x.k, 2 ORDER BY x.k, MIN(t)"""),
# ---------- S13 development ----------
P("S13.poor", "S13", "Poor-coaching seasons by level", "bars",
  "Player seasons whose coaching number was under 8 (a poor-coaching risk), as a share of each level's seasons.",
  """SELECT level AS label, ROUND(100.0 * AVG(coaching < 8), 2) AS world, ROUND(AVG(coaching), 2) AS mean_coaching, COUNT(*) AS n FROM people__player__history WHERE coaching IS NOT NULL GROUP BY level ORDER BY n DESC"""),
P("S13.nfl", "S13", "NFL clubs' coaching", "table",
  "Each NFL club's coaching value, staff years and the head coach, OC and DC behind it.",
  """SELECT t._row AS id, t.name AS club, ROUND(t.coaching, 2) AS coaching, t.staff_years, (SELECT name FROM coaches WHERE _row = t.head_coach) AS head_coach, (SELECT name FROM coaches WHERE _row = t.oc) AS oc,
        (SELECT name FROM coaches WHERE _row = t.dc) AS dc, ROUND(t.win_share, 3) AS win_share FROM nfl t ORDER BY t.coaching DESC"""),
# ---------- S14 systems ----------
P("S14.fbs_offenses", "S14", "FBS offenses by year", "line",
  "How many FBS programs ran each offense each year (`fbs_systems` in the calendar log): air raid and spread option should rise through the 2000s and 2010s.",
  """SELECT year, json_extract(fbs_systems, '$[0]') AS west_coast, json_extract(fbs_systems, '$[1]') AS air_raid, json_extract(fbs_systems, '$[2]') AS wide_zone, json_extract(fbs_systems, '$[3]') AS power_gap,
        json_extract(fbs_systems, '$[4]') AS spread_option, json_extract(fbs_systems, '$[5]') AS four_three, json_extract(fbs_systems, '$[6]') AS three_four FROM log_history WHERE fbs_systems IS NOT NULL ORDER BY year"""),
P("S14.installed", "S14", "What teams run now, by level", "matrix",
  "Programs and NFL clubs now by the offense, front and coverage family installed, as shares of the level.",
  """WITH y AS (SELECT division AS level, installed__off AS o, installed__front AS f, installed__shell AS s FROM programs WHERE installed__off IS NOT NULL
        UNION ALL SELECT 'NFL', installed__off, installed__front, installed__shell FROM clubs WHERE installed__off IS NOT NULL),
   x AS (SELECT level, CASE o WHEN 0 THEN 'West Coast' WHEN 1 THEN 'Air raid' WHEN 2 THEN 'Wide zone' WHEN 3 THEN 'Power gap' ELSE 'Spread option' END AS off,
        CASE f WHEN 0 THEN '4-3 one-gap' ELSE '3-4 two-gap' END AS front, CASE s WHEN 0 THEN 'Cover 3' WHEN 1 THEN 'Quarters' WHEN 2 THEN 'Man pressure' ELSE 'Cover 2' END AS shell FROM y)
  SELECT level AS row, 'Offense: ' || off AS col, ROUND(100.0 * COUNT(*) / SUM(COUNT(*)) OVER (PARTITION BY level), 1) AS value FROM x GROUP BY level, off
   UNION ALL SELECT level, 'Front: ' || front, ROUND(100.0 * COUNT(*) / SUM(COUNT(*)) OVER (PARTITION BY level), 1) FROM x GROUP BY level, front
   UNION ALL SELECT level, 'Coverage: ' || shell, ROUND(100.0 * COUNT(*) / SUM(COUNT(*)) OVER (PARTITION BY level), 1) FROM x GROUP BY level, shell"""),
P("S14.switches", "S14", "System switches a year", "line",
  "Teams that switched an installed system each year (the calendar log).",
  """SELECT year, system_switches AS switches FROM log_history WHERE system_switches IS NOT NULL ORDER BY year"""),
P("S14.knowledge", "S14", "How well coaches know each system (debug)", "matrix",
  "Coaches in a job now: mean knowledge (0-1) of each of the 11 systems by level.",
  CJ + """, s(k, name) AS (VALUES (0, 'West Coast'), (1, 'Air raid'), (2, 'Wide zone'), (3, 'Power gap'), (4, 'Spread option'), (5, '4-3 one-gap'), (6, '3-4 two-gap'), (7, 'Cover 3'), (8, 'Quarters'), (9, 'Man pressure'), (10, 'Cover 2'))
  SELECT cj.level AS row, s.name AS col, ROUND(AVG(j.value), 2) AS value FROM cj, json_each(cj.systems__know) j JOIN s ON s.k = j.key GROUP BY cj.level, s.k ORDER BY """ + LEVEL_ORDER.replace("level", "cj.level") + ", s.k"),
P("S14.norms", "S14", "Each position's typical score in each system (debug)", "matrix",
  "The world's system norms (`system_norms`): rows are positions, columns the 11 systems. An air raid should want receivers, power gap linemen.",
  """WITH s(k, name) AS (VALUES (0, 'West Coast'), (1, 'Air raid'), (2, 'Wide zone'), (3, 'Power gap'), (4, 'Spread option'), (5, '4-3 one-gap'), (6, '3-4 two-gap'), (7, 'Cover 3'), (8, 'Quarters'), (9, 'Man pressure'), (10, 'Cover 2')),
   p(k, pos) AS (VALUES (0, 'QB'), (1, 'RB'), (2, 'FB'), (3, 'WR'), (4, 'TE'), (5, 'OT'), (6, 'IOL'), (7, 'EDGE'), (8, 'DT'), (9, 'LB'), (10, 'CB'), (11, 'S'), (12, 'K'), (13, 'P'))
  SELECT p.pos AS row, s.name AS col, ROUND(CASE s.k WHEN 0 THEN n._0 WHEN 1 THEN n._1 WHEN 2 THEN n._2 WHEN 3 THEN n._3 WHEN 4 THEN n._4 WHEN 5 THEN n._5 WHEN 6 THEN n._6 WHEN 7 THEN n._7 WHEN 8 THEN n._8 WHEN 9 THEN n._9 ELSE n._10 END, 1) AS value
   FROM system_norms n JOIN p ON p.k = n._row, s ORDER BY n._row, s.k"""),
# ---------- S16 play callers ----------
P("S16.callers", "S16", "Who calls the plays, and with what (debug)", "table",
  "Each level now: how often the head coach calls a side because the seat is empty, and the coordinators' scheme, game management and ambition (the caller's skill, reading, creativity, nerve and aggressiveness are worked out from these and his systems each game, `coach_caller`; not stored).",
  CJ + """ SELECT cj.level, cj.role, COUNT(*) AS coaches, ROUND(AVG(ability__scheme), 1) AS scheme, ROUND(AVG(ability__game_management), 1) AS game_management, ROUND(AVG(personality__ambition), 1) AS ambition,
        CASE cj.role WHEN 'Head coach' THEN ROUND(100.0 * AVG((SELECT (p.oc IS NULL) + (p.dc IS NULL) FROM programs p WHERE p._row = cj.job__employer__College)) / 2, 1) END AS calls_a_side_pct
   FROM cj WHERE cj.role <> 'Position coach' AND cj.level NOT IN ('HS', 'Other') GROUP BY cj.level, cj.role ORDER BY """ + LEVEL_ORDER.replace("level", "cj.level") + ", cj.role"),
# ---------- S17 calls ----------
P("S17.teams", "S17", "Each FBS team's calls last season", "table",
  "From the game log's team situation totals (`team_situations`): each FBS team's pass share, fourth-down go rate and blitz rate in the world's last logged season, with its coordinators.",
  JOBS + """, y AS (SELECT MAX(season) AS y FROM log_team_situations WHERE level = 'college'),
   t AS (SELECT team, SUM(CASE WHEN key = 'o|kind|pass' THEN plays END) AS pass, SUM(CASE WHEN key = 'o|kind|run' THEN plays END) AS run, SUM(CASE WHEN key = 'o|fourth|go' THEN plays END) AS go,
        SUM(CASE WHEN key IN ('o|fourth|go', 'o|fourth|punt', 'o|fourth|fg') THEN plays END) AS fourth, SUM(CASE WHEN key = 'd|blitz|true' THEN plays END) AS blitz,
        SUM(CASE WHEN key IN ('d|blitz|true', 'd|blitz|false') THEN plays END) AS dropbacks
   FROM log_team_situations WHERE level = 'college' AND season = (SELECT y FROM y) GROUP BY team)
  SELECT t.team AS id, p.real_name AS program, ROUND(100.0 * t.pass / (t.pass + t.run), 1) AS pass_pct, ROUND(100.0 * t.go / t.fourth, 1) AS fourth_go_pct, ROUND(100.0 * t.blitz / t.dropbacks, 1) AS blitz_pct,
        (SELECT co.name FROM jobs j JOIN coaches co ON co._row = j.c WHERE j.college = t.team AND j.role = 'Offensive coordinator' AND (SELECT y FROM y) BETWEEN j.a AND j.z LIMIT 1) AS oc,
        (SELECT co.name FROM jobs j JOIN coaches co ON co._row = j.c WHERE j.college = t.team AND j.role = 'Defensive coordinator' AND (SELECT y FROM y) BETWEEN j.a AND j.z LIMIT 1) AS dc
   FROM t JOIN programs p ON p._row = t.team WHERE p.division = 'FBS' ORDER BY pass_pct DESC""", needs=GL, open="e/programs/{id}"),
P("S17.spread", "S17", "How different teams' calls are", "hist",
  "The spread of FBS teams' pass share in the last logged season (pick pass share, fourth-down go rate or blitz rate above). Real FBS pass share overall is 49.9% (2023-24).",
  """WITH y AS (SELECT MAX(season) AS y FROM log_team_situations WHERE level = 'college'),
   t AS (SELECT team, SUM(CASE WHEN key = 'o|kind|pass' THEN plays END) AS pass, SUM(CASE WHEN key = 'o|kind|run' THEN plays END) AS run, SUM(CASE WHEN key = 'o|fourth|go' THEN plays END) AS go,
        SUM(CASE WHEN key IN ('o|fourth|go', 'o|fourth|punt', 'o|fourth|fg') THEN plays END) AS fourth, SUM(CASE WHEN key = 'd|blitz|true' THEN plays END) AS blitz,
        SUM(CASE WHEN key IN ('d|blitz|true', 'd|blitz|false') THEN plays END) AS dropbacks
   FROM log_team_situations WHERE level = 'college' AND season = (SELECT y FROM y) GROUP BY team)
  SELECT 'Pass share' AS grp, 2 * CAST(50.0 * pass / (pass + run) AS INTEGER) AS x, COUNT(*) AS world FROM t JOIN programs p ON p._row = t.team WHERE p.division = 'FBS' GROUP BY 2
   UNION ALL SELECT 'Fourth-down go rate', 5 * CAST(20.0 * go / fourth AS INTEGER), COUNT(*) FROM t JOIN programs p ON p._row = t.team WHERE p.division = 'FBS' AND fourth > 0 GROUP BY 2
   UNION ALL SELECT 'Blitz rate', 2 * CAST(50.0 * blitz / dropbacks AS INTEGER), COUNT(*) FROM t JOIN programs p ON p._row = t.team WHERE p.division = 'FBS' AND dropbacks > 0 GROUP BY 2 ORDER BY 1, 2""", needs=GL),
# ---------- S18 what beats what ----------
P("S18.table", "S18", "What beats what: the coaches' table", "matrix",
  "Mean yards each pass concept gained against each coverage with four rushers or a linebacker blitz, and each run against each front, as measured on the engine (`call_values.csv`, cap 40, a turnover -45). Rows are concepts and runs.",
  """SELECT concept AS row, against AS col, yards AS value FROM engine_call_values ORDER BY _row"""),
P("S18.best_worst", "S18", "Each concept's best and worst look", "table",
  "For each pass concept and run: the coverage or front it does best and worst against, in yards. Classic pairings should hold (smash and corners beat Cover 2 and Cover 3, crossers and mesh beat man, flats are poor against zone).",
  """WITH r AS (SELECT _row, concept, against, yards, ROW_NUMBER() OVER (PARTITION BY concept ORDER BY yards DESC) AS hi, ROW_NUMBER() OVER (PARTITION BY concept ORDER BY yards) AS lo FROM engine_call_values)
  SELECT concept, MAX(CASE WHEN hi = 1 THEN against END) AS best_against, MAX(CASE WHEN hi = 1 THEN yards END) AS best_yards, MAX(CASE WHEN lo = 1 THEN against END) AS worst_against,
        MAX(CASE WHEN lo = 1 THEN yards END) AS worst_yards FROM r GROUP BY concept ORDER BY MIN(_row)"""),
# ---------- S19 ideas spreading ----------
P("S19.families", "S19", "Concept families by year", "line",
  "Each division's share of snaps in each of the 14 concept families by season and its pass rate (`trends`); pick a division above.",
  """SELECT division AS grp, year, ROUND(100 * json_extract(families, '$[0]'), 1) AS inside_zone, ROUND(100 * json_extract(families, '$[1]'), 1) AS outside_zone, ROUND(100 * json_extract(families, '$[2]'), 1) AS gap,
        ROUND(100 * json_extract(families, '$[3]'), 1) AS option, ROUND(100 * json_extract(families, '$[4]'), 1) AS rpo, ROUND(100 * json_extract(families, '$[5]'), 1) AS quick, ROUND(100 * json_extract(families, '$[6]'), 1) AS dropback,
        ROUND(100 * json_extract(families, '$[7]'), 1) AS play_action, ROUND(100 * json_extract(families, '$[8]'), 1) AS screen, ROUND(100 * json_extract(families, '$[10]'), 1) AS single_high,
        ROUND(100 * json_extract(families, '$[11]'), 1) AS two_high, ROUND(100 * json_extract(families, '$[12]'), 1) AS man, ROUND(100 * json_extract(families, '$[13]'), 1) AS pressure, ROUND(100 * pass_rate, 1) AS pass_rate
   FROM trends ORDER BY division, year"""),
P("S19.novelty", "S19", "How new each family is to each division now", "matrix",
  "Current novelty (`novelty`): a family that rose over the last four seasons is new to the division's defenses.",
  """WITH f(k, name) AS (VALUES (0, 'Inside zone'), (1, 'Outside zone'), (2, 'Gap'), (3, 'Option'), (4, 'RPO'), (5, 'Quick'), (6, 'Dropback'), (7, 'Play action'), (8, 'Screen'), (9, 'Front'), (10, 'Single high'), (11, 'Two high'), (12, 'Man'), (13, 'Pressure')),
   d(k, name) AS (VALUES (0, 'FBS'), (1, 'FCS'), (2, 'D2'), (3, 'D3'), (4, 'NAIA'), (5, 'JUCO'))
  SELECT d.name AS row, f.name AS col, ROUND(CASE f.k WHEN 0 THEN n._0 WHEN 1 THEN n._1 WHEN 2 THEN n._2 WHEN 3 THEN n._3 WHEN 4 THEN n._4 WHEN 5 THEN n._5 WHEN 6 THEN n._6 WHEN 7 THEN n._7
        WHEN 8 THEN n._8 WHEN 9 THEN n._9 WHEN 10 THEN n._10 WHEN 11 THEN n._11 WHEN 12 THEN n._12 ELSE n._13 END, 3) AS value FROM novelty n JOIN d ON d.k = n._row, f ORDER BY n._row, f.k"""),
# ---------- S20 job security ----------
P("S20.reviews", "S20", "Every head coach review", "table",
  "The world log's reviews after each regular season (`Review` whys), newest and closest first: the read against the bar, the chance he was fired and what happened.",
  """SELECT w.season AS year, w.why__Review__coach AS id, (SELECT name FROM coaches WHERE _row = w.why__Review__coach) AS coach, (SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = w.why__Review__program) AS program,
        w.why__Review__tenure AS tenure, ROUND(w.why__Review__win_pct_last, 3) AS win_pct, ROUND(w.why__Review__read, 3) AS read, ROUND(w.why__Review__bar, 3) AS bar, ROUND(100 * w.why__Review__fire_chance, 1) AS fire_pct,
        w.why__Review__outcome AS outcome
   FROM log_whys w WHERE w.why__Review__coach IS NOT NULL ORDER BY w.season DESC, w.why__Review__fire_chance DESC""", needs=WL, open="e/coaches/{id}"),
P("S20.fire_by_tenure", "S20", "Firing rate by season of tenure", "bars",
  "FBS head coaching runs that began after the world's first season: of those reaching each season, the share that ended fired after it (year one should be near zero).",
  RUNS + """, k(k) AS (VALUES (1), (2), (3), (4), (5), (6), (8), (10))
  SELECT 'Season ' || k.k AS label, ROUND(100.0 * SUM(e.n = k.k AND e.ended LIKE 'fired%') / SUM(e.n >= k.k), 1) AS world, SUM(e.n >= k.k) AS n FROM ended e, k WHERE e.division = 'FBS' GROUP BY k.k ORDER BY k.k"""),
P("S20.in_season", "S20", "In-season firings a year", "line",
  "Head jobs that ended 'fired during the season' each year, by level (real FBS recent years about 5-10, no source yet).",
  ST + """ SELECT z AS year, SUM(level = 'FBS') AS fbs, SUM(level = 'FCS') AS fcs, SUM(level NOT IN ('FBS', 'FCS')) AS other FROM st WHERE ended = 'fired during the season' GROUP BY z ORDER BY z"""),
P("S20.hot_seat", "S20", "Prove-it years and the hot seat now", "table",
  "College head coaches now on a prove-it year or with a losing last two seasons, with the AD's patience and the buyout factor's base (contract years left).",
  CJ + """ SELECT cj._row AS id, cj.name, (SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = cj.job__employer__College) AS program, cj.level, """ + Y + """ - cj.job__since + 1 AS season_in_job,
        cj.prove_it, cj.recent AS last_six_vs_expectation, cj.contract__through AS contract_through, ROUND(p.ad__patience, 1) AS ad_patience
   FROM cj JOIN programs p ON p._row = cj.job__employer__College WHERE cj.role = 'Head coach' AND (cj.prove_it IS NOT NULL OR (SELECT AVG(value) FROM (SELECT value FROM json_each(cj.recent) ORDER BY key DESC LIMIT 2)) < 0)
   ORDER BY """ + LEVEL_ORDER.replace("level", "cj.level") + ", cj.prove_it IS NULL", open="e/coaches/{id}"),
# ---------- S21 pay ----------
P("S21.salary", "S21", "College head coach pay by prestige", "bars",
  "College head coaches' salary now (`contract`) by their program's prestige in fifths at each level (pick above), in $ millions.",
  CJ + """, x AS (SELECT level, contract__salary AS s, NTILE(5) OVER (PARTITION BY level ORDER BY prestige) AS q, prestige FROM cj WHERE role = 'Head coach' AND contract__salary > 0 AND level NOT IN ('HS', 'NFL', 'Other'))
  SELECT level AS grp, 'Prestige fifth ' || q AS label, ROUND(AVG(s) / 1e6, 2) AS world, ROUND(MAX(s) / 1e6, 2) AS top, COUNT(*) AS n FROM x GROUP BY level, q ORDER BY """ + LEVEL_ORDER + ", q"),
P("S21.contracts", "S21", "Contracts by level", "table",
  "College head coaches now: contract years left, salary and buyout share, by level.",
  CJ + """ SELECT level, COUNT(*) AS coaches, ROUND(AVG(contract__through - """ + Y + """), 1) AS years_left, ROUND(AVG(contract__salary) / 1e6, 2) AS salary_m, ROUND(MAX(contract__salary) / 1e6, 2) AS top_m,
        ROUND(AVG(contract__buyout_share), 2) AS buyout_share FROM cj WHERE role = 'Head coach' AND contract__through IS NOT NULL GROUP BY level ORDER BY """ + LEVEL_ORDER),
# ---------- S22 NFL ----------
P("S22.clubs", "S22", "NFL staffs and front offices now", "table",
  "Each club's head coach, coordinators, coaching value, the GM and who has the final say.",
  """SELECT t._row AS id, t.name AS club, (SELECT name FROM coaches WHERE _row = t.head_coach) AS head_coach, (SELECT name FROM coaches WHERE _row = t.oc) AS oc, (SELECT name FROM coaches WHERE _row = t.dc) AS dc,
        ROUND(t.coaching, 2) AS coaching, t.staff_years, k.office__gm__name AS gm, k.office__power AS final_say, ROUND(t.win_share, 3) AS win_share FROM nfl t LEFT JOIN clubs k ON k._row = t._row ORDER BY t.name"""),
P("S22.tenure", "S22", "How long NFL head coaches last", "bars",
  "Finished NFL head coaching runs (`nfl_seasons`) that began after the world's first NFL season: the share lasting at least 2, 3, 4, 5, 8 and 10 seasons, against real 2000-2019 (`nfl_history.json`).",
  NFL_RUNS + """, k(k) AS (VALUES (2), (3), (4), (5), (8), (10))
  SELECT 'At least ' || k.k || ' seasons' AS label, ROUND(100.0 * AVG(nf.n >= k.k), 1) AS world,
        100.0 * json_extract(""" + RD("nfl_history.json") + """, '$.head_coaches.share_lasting_at_least."' || k.k || '"') AS real, COUNT(*) AS n FROM nf, k GROUP BY k.k ORDER BY k.k"""),
P("S22.firings", "S22", "NFL head coaches fired a year and their tenure", "line",
  "NFL head jobs that ended fired each year, with the mean tenure of those fired.",
  ST + """ SELECT z AS year, SUM(ended LIKE 'fired%') AS fired, ROUND(AVG(CASE WHEN ended LIKE 'fired%' THEN z - a + 1 END), 1) AS tenure FROM st WHERE role = 'Head coach' AND level = 'NFL' GROUP BY z ORDER BY z"""),
P("S22.college_to_nfl", "S22", "College head coaches hired by the NFL", "table",
  "NFL head jobs taken by a coach whose last job was a college head job: rare, and mostly title coaches.",
  JOBS + """, n AS (SELECT c, a, role, level, LAG(role) OVER (PARTITION BY c ORDER BY a, z) AS prev_role, LAG(level) OVER (PARTITION BY c ORDER BY a, z) AS prev_level, LAG(college) OVER (PARTITION BY c ORDER BY a, z) AS prev_college FROM jobs)
  SELECT n.a AS year, n.c AS id, co.name, (SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = n.prev_college) AS from_program, co.career_titles AS titles FROM n JOIN coaches co ON co._row = n.c
   WHERE n.role = 'Head coach' AND n.level = 'NFL' AND n.prev_role = 'Head coach' AND n.prev_level NOT IN ('NFL', 'HS', 'Other') ORDER BY n.a DESC""", open="e/coaches/{id}"),
# ---------- S23 high school coaches ----------
P("S23.to_college", "S23", "High school coaches who moved into college", "table",
  "Every high school head coach whose next job was at a college: the school, the college, the role and year.",
  JOBS + """, n AS (SELECT c, a, role, level, college, LAG(level) OVER (PARTITION BY c ORDER BY a, z) AS prev_level, LAG(hs) OVER (PARTITION BY c ORDER BY a, z) AS prev_hs, LAG(role) OVER (PARTITION BY c ORDER BY a, z) AS prev_role FROM jobs)
  SELECT n.a AS year, n.c AS id, co.name, (SELECT name || ' (' || town || ')' FROM high_schools WHERE _row = n.prev_hs) AS from_school, (SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = n.college) AS to_program,
        n.level, n.role FROM n JOIN coaches co ON co._row = n.c WHERE n.prev_level = 'HS' AND n.prev_role = 'Head coach' AND n.college IS NOT NULL ORDER BY n.a DESC""", open="e/coaches/{id}"),
# ---------- S24 staffs ----------
P("S24.turnover", "S24", "New staff members a year at FBS", "line",
  "Each season: new coordinators and position coaches per FBS program (jobs that began that season).",
  JOBS + """ SELECT j.a AS year, ROUND(1.0 * SUM(j.role IN ('Offensive coordinator', 'Defensive coordinator')) / (SELECT COUNT(*) FROM programs__seasons s JOIN programs p ON p._row = s._parent WHERE s.year = j.a AND p.division = 'FBS'), 2) AS coordinators,
        ROUND(1.0 * SUM(j.role = 'Position coach') / (SELECT COUNT(*) FROM programs__seasons s JOIN programs p ON p._row = s._parent WHERE s.year = j.a AND p.division = 'FBS'), 2) AS position_coaches
   FROM jobs j WHERE j.level = 'FBS' AND j.a > (SELECT MIN(year) FROM programs__seasons) GROUP BY j.a ORDER BY j.a"""),
P("S24.holdovers", "S24", "Coordinators kept by a new head coach", "line",
  "Each season an FBS program changed head coach: the share of its coordinators from the season before who stayed on (design 40%, 75% if promoted).",
  PS + ", " + JOBS[len("WITH "):] + """, ch AS (SELECT prog, year FROM (SELECT prog, year, coach, LAG(coach) OVER (PARTITION BY prog ORDER BY year) AS before FROM ps WHERE division = 'FBS') WHERE coach IS NOT before AND before IS NOT NULL)
  SELECT ch.year, ROUND(100.0 * AVG(j.z >= ch.year), 1) AS world, COUNT(*) AS n FROM ch JOIN jobs j ON j.college = ch.prog AND j.role IN ('Offensive coordinator', 'Defensive coordinator') AND ch.year - 1 BETWEEN j.a AND j.z
   GROUP BY ch.year ORDER BY ch.year"""),
P("S24.empty", "S24", "Empty seats now", "bars",
  "Programs now missing an OC, a DC or position coaches (of eight), by level.",
  """SELECT division AS label, ROUND(100.0 * AVG(oc IS NULL OR dc IS NULL), 1) AS world, ROUND(AVG((SELECT COUNT(*) FROM json_each(position_coaches) WHERE value IS NULL)), 2) AS empty_position_seats, COUNT(*) AS n
   FROM programs WHERE _row IN (SELECT _parent FROM programs__seasons WHERE year = """ + Y + """) GROUP BY division ORDER BY n DESC"""),
P("S24.staff", "S24", "Every program's staff now", "table",
  "Each program's head coach, coordinators and position coaches now, and when the head coach last changed.",
  """SELECT p._row AS id, COALESCE(p.real_name, p.fictional_name) AS program, p.division, (SELECT name FROM coaches WHERE _row = p.head_coach) AS head_coach, p.hc_changed AS head_coach_since,
        (SELECT name FROM coaches WHERE _row = p.oc) AS oc, (SELECT name FROM coaches WHERE _row = p.dc) AS dc,
        (SELECT group_concat(c.name, ', ') FROM json_each(p.position_coaches) j JOIN coaches c ON c._row = j.value) AS position_coaches
   FROM programs p ORDER BY CASE p.division WHEN 'FBS' THEN 0 WHEN 'FCS' THEN 1 ELSE 2 END, p.prestige DESC""", open="e/programs/{id}"),
# ---------- S25 support staff ----------
P("S25.support", "S25", "Support staff ratings by level", "table",
  "Mean support head ratings (1-20) now: strength and conditioning, medical, analytics, academics, by level, and at NFL clubs (design N(5.5 + 7 x budget + 2 x era, 2.8)).",
  """SELECT division AS level, ROUND(AVG(support__strength__rating), 1) AS strength, ROUND(AVG(support__medical__rating), 1) AS medical, ROUND(AVG(support__analytics__rating), 1) AS analytics,
        ROUND(AVG(support__academic__rating), 1) AS academic, COUNT(*) AS n FROM programs GROUP BY division
   UNION ALL SELECT 'NFL', ROUND(AVG(support__strength__rating), 1), ROUND(AVG(support__medical__rating), 1), ROUND(AVG(support__analytics__rating), 1), NULL, COUNT(*) FROM clubs"""),
# ---------- S26 health and exits ----------
P("S26.exits", "S26", "Retirements and early exits a year", "line",
  "Jobs that ended in retirement, retirement for health, burnout or walking away on top each year.",
  ST + """ SELECT z AS year, SUM(ended = 'retired') AS retired, SUM(ended = 'retired for his health') AS health, SUM(ended LIKE 'stepped away%') AS burned_out, SUM(ended LIKE 'walked away%') AS on_top
   FROM st GROUP BY z HAVING SUM(ended LIKE 'retired%' OR ended LIKE 'stepped away%' OR ended LIKE 'walked away%') > 0 ORDER BY z"""),
P("S26.retire_age", "S26", "Age at retirement", "hist",
  "Coaches whose last job ended in retirement, by age then (from his age now), head coaches and the rest apart (pick above).",
  ST + """, last AS (SELECT c, MAX(z) AS z FROM st GROUP BY c)
  SELECT CASE WHEN st.role = 'Head coach' THEN 'Head coaches' ELSE 'Assistants' END AS grp, st.z - """ + BY + """ AS x, COUNT(*) AS world
   FROM st JOIN last ON last.c = st.c AND last.z = st.z JOIN coaches c ON c._row = st.c WHERE st.ended LIKE 'retired%' GROUP BY 1, 2 ORDER BY 1, 2"""),
P("S26.health", "S26", "Health episodes in the last 400 days", "table",
  "Every coach health episode the world still keeps (`folk.life.health`), with the coach and his job.",
  """SELECT date(h.date * 86400, 'unixepoch') AS date, h.coach AS id, c.name, h.what, date(h.back * 86400, 'unixepoch') AS back, CASE WHEN h.serious = 1 THEN 'serious' ELSE '' END AS serious, c.age
   FROM folk__life__health h JOIN coaches c ON c._row = h.coach ORDER BY h.date DESC""", open="e/coaches/{id}"),
# ---------- S27 recruiting reach ----------
P("S27.ties", "S27", "Ties to high schools by level and role", "table",
  "Coaches in a job now: ties to high schools (0-1, from impact players he coached), how many and how strong, by level and role.",
  """WITH t AS (SELECT CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS' WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level,
        """ + ROLE.format(p="c.job__") + """ AS role, c.ability__network, (SELECT COUNT(*) FROM coaches__ties x WHERE x._parent = +c._row) AS n, (SELECT AVG(x._v) FROM coaches__ties x WHERE x._parent = +c._row) AS s
   FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL)
  SELECT level, role, COUNT(*) AS coaches, ROUND(AVG(n), 1) AS ties, ROUND(AVG(s), 3) AS strength, ROUND(AVG(ability__network), 1) AS network_debug
   FROM t GROUP BY level, role ORDER BY """ + LEVEL_ORDER + ", role"),
P("S27.top", "S27", "The widest recruiting reach", "table",
  "The 40 coaches with the most strong ties (0.2+) now, with his job and the states they reach.",
  """WITH t AS (SELECT x._parent, COUNT(*) AS n, group_concat(DISTINCT st.code) AS states FROM coaches__ties x JOIN high_schools h ON h._row = CAST(x._key AS INTEGER) JOIN states st ON st._row = h.state WHERE x._v >= 0.2 GROUP BY x._parent)
  SELECT c._row AS id, c.name, """ + ROLE.format(p="c.job__") + """ AS role, COALESCE((SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = c.job__employer__College), '') AS program, t.n AS strong_ties, t.states
   FROM t JOIN coaches c ON c._row = t._parent ORDER BY t.n DESC LIMIT 40""", open="e/coaches/{id}"),
# ---------- S28 the Hall ----------
P("S28.hall", "S28", "Coaches in the Hall of Fame", "table",
  "College Football Hall of Fame coaches (`stats.hall`), with the why and their career head coach record. Real bar: 100+ games and .600+ (NFF).",
  JOBS + """, r AS (SELECT c, SUM(w) AS w, SUM(l) AS l FROM jobs WHERE role = 'Head coach' AND college IS NOT NULL GROUP BY c)
  SELECT h.year, h.coach AS id, h.name, h.division, h.why, r.w || '-' || r.l AS record, ROUND(1.0 * r.w / NULLIF(r.w + r.l, 0), 3) AS pct
   FROM stats__hall h LEFT JOIN coaches c ON c.name = h.name LEFT JOIN r ON r.c = c._row WHERE h.coach = 1 ORDER BY h.year DESC""", open="e/coaches/{id}"),
# ---------- S29 other leagues ----------
P("S29.other", "S29", "Other leagues' coaches", "table",
  "Head jobs in the other leagues (CFL, arena, spring leagues), current and finished, with record and what came next.",
  JOBS + """, n AS (SELECT c, a, z, role, level, w, l, ended, LEAD(level) OVER (PARTITION BY c ORDER BY a, z) AS next_level, LEAD(role) OVER (PARTITION BY c ORDER BY a, z) AS next_role FROM jobs)
  SELECT n.c AS id, co.name, n.a || '-' || n.z AS seasons, n.w || '-' || n.l AS record, n.ended, COALESCE(n.next_level || ' ' || lower(n.next_role), '') AS next_job
   FROM n JOIN coaches co ON co._row = n.c WHERE n.level = 'Other' AND n.role = 'Head coach' ORDER BY n.a DESC""", open="e/coaches/{id}"),
# ---------- S30 and S31 training ----------
P("S30.rules", "S30", "The rule books by season", "table",
  "The rules each season runs under (`calendar/rules.rs` RuleBook::era, as the engine reads them): the NFL's padded practices and offseason weeks, the transfer and redshirt rules, NIL and the playoff. The NCAA's 20-hour week (1991), summer access (2013) and camp's 25 practices (2021) are in `training/rules.rs` (`docs/design/training.md`).",
  """SELECT season, nfl_padded_practices, nfl_offseason, nfl_roster, nfl_practice_squad, cfp_teams, fbs_games, transfer, one_time_transfer, four_game_redshirt, five_in_five, nil, early_signing, revenue_sharing
   FROM engine_rules ORDER BY season DESC"""),
P("S31.prep", "S31", "Each player's latest prep word", "bars",
  "College and NFL players now by their latest weekly prep word (`train.prep`, Unprepared to Sharp; players with no word yet left out): Prepared should be the most common, Unprepared and Underprepared together under 12%.",
  """WITH w(k, word) AS (VALUES (1, 'Unprepared'), (2, 'Underprepared'), (3, 'Patchy'), (4, 'Prepared'), (5, 'Sharp'))
  SELECT x.stage AS grp, w.word AS label, ROUND(100.0 * COUNT(*) / SUM(COUNT(*)) OVER (PARTITION BY x.stage), 1) AS world, COUNT(*) AS n
   FROM people x JOIN w ON w.k = (x.player__train__prep & 7) WHERE x.stage IN ('College', 'Pro') GROUP BY x.stage, w.k ORDER BY x.stage, w.k"""),
P("S31.rust", "S31", "Rust and load now", "table",
  "College and NFL players now: the share carrying rust from poor prep, the mean rust in points of reading and technique, this week's and the chronic load (tens of units), and the share with a load spike over 1.45 (rare outside camp).",
  """SELECT stage, COUNT(*) AS players, ROUND(100.0 * AVG(json_extract(player__train__rust, '$[0]') + json_extract(player__train__rust, '$[1]') > 0), 1) AS rusty_pct,
        ROUND(AVG(json_extract(player__train__rust, '$[0]')) / 10.0, 2) AS reading_rust, ROUND(AVG(json_extract(player__train__rust, '$[1]')) / 10.0, 2) AS technique_rust,
        ROUND(AVG(json_extract(player__train__load, '$[0]')), 1) AS load_now, ROUND(AVG(json_extract(player__train__load, '$[1]')), 1) AS load_chronic,
        ROUND(100.0 * AVG(1.0 * json_extract(player__train__load, '$[0]') / NULLIF(json_extract(player__train__load, '$[1]'), 0) > 1.45), 1) AS spike_pct
   FROM people WHERE stage IN ('College', 'Pro') AND player__train__load IS NOT NULL GROUP BY stage"""),
# ---------- S34 agents (the NFL page) ----------
P("S34.agencies", "S34", "Agencies by clients", "bars",
  "NFL agents' clients summed by agency, biggest first. Real: a few big agencies hold most first-round picks (no open count).",
  """SELECT agency AS label, SUM(json_array_length(clients)) AS world, COUNT(*) AS agents, ROUND(AVG(honesty), 1) AS honesty FROM market__agents GROUP BY agency ORDER BY world DESC""", page="nfl"),
P("S34.agents", "S34", "Every agent", "table",
  "Each agent's agency, size, skill, honesty, clients and how often he was fired.",
  """SELECT name, agency, size, ROUND(skill, 1) AS skill, ROUND(honesty, 1) AS honesty, json_array_length(clients) AS clients, fired FROM market__agents ORDER BY json_array_length(clients) DESC""", page="nfl"),
]

CHECKS = [
dict(id="S8.fbs_new_head_coaches", page="coaches", areas="S8 S7", title="FBS programs with a new head coach a season", unit="%",
  world=PS + """, x AS (SELECT year, pid, coach, LAG(coach) OVER (PARTITION BY pid ORDER BY year) AS before, division FROM ps)
   SELECT AVG(coach IS NOT before) FROM x WHERE division = 'FBS' AND before IS NOT NULL""", scale=100.0,
  years=PS + """, x AS (SELECT year, pid, coach, LAG(coach) OVER (PARTITION BY pid ORDER BY year) AS before, division FROM ps)
   SELECT year, AVG(coach IS NOT before) FROM x WHERE division = 'FBS' AND before IS NOT NULL GROUP BY year""", detail=None,
  real=("Fixed", 17.6, "college_people.json fbs_head_coaches changes_by_season, mean share 2005-2025", "2005-2025"), tol=(0.15, 0.35, 0.0), needs=W,
  how="FBS program seasons whose head coach (the season's coach) differs from the season before."),
dict(id="S7.fbs_tenure_median", page="coaches", areas="S7", title="FBS head coach tenure (median, finished runs)", unit="seasons",
  world=RUNS + """, r AS (SELECT n AS v, ROW_NUMBER() OVER (ORDER BY n) AS r, COUNT(*) OVER () AS n FROM ended WHERE division = 'FBS')
   SELECT """ + PCT(0.5) + " FROM r", scale=1.0, years=None, detail=None,
  real=("Key", "college_people.json:fbs_head_coaches.completed_tenure_years#p50", 1.0), tol=(0.2, 0.4, 0.0), needs=W,
  how="Runs of seasons one man was an FBS program's head coach (the season's coach), begun after the world's first season and ended before its last: the median."),
dict(id="S8.nfl_changes", page="coaches", areas="S8 S22", title="NFL head coach changes a season", unit="",
  world="""WITH x AS (SELECT year, team, coach, LAG(coach) OVER (PARTITION BY team ORDER BY year) AS before FROM nfl_seasons)
   SELECT 1.0 * SUM(coach IS NOT before) / COUNT(DISTINCT year) FROM x WHERE before IS NOT NULL""", scale=1.0,
  years="""WITH x AS (SELECT year, team, coach, LAG(coach) OVER (PARTITION BY team ORDER BY year) AS before FROM nfl_seasons) SELECT year, SUM(coach IS NOT before) FROM x WHERE before IS NOT NULL GROUP BY year""",
  detail=None, real=("Fixed", 6.5, "nfl_history.json head_coaches offseason_changes_by_season, mean 2000-2025 (plus about 1.6 in-season)", "2000-2025"), tol=(0.2, 0.4, 0.0), needs=W,
  how="NFL club seasons whose head coach differs from the season before (`nfl_seasons`)."),
dict(id="S7.nfl_tenure_median", page="coaches", areas="S7 S22", title="NFL head coach tenure (median, finished runs)", unit="seasons",
  world=NFL_RUNS + """, r AS (SELECT n AS v, ROW_NUMBER() OVER (ORDER BY n) AS r, COUNT(*) OVER () AS n FROM nf)
   SELECT """ + PCT(0.5) + " FROM r", scale=1.0, years=None, detail=None,
  real=("Key", "nfl_history.json:head_coaches.completed_tenure_years#p50", 1.0), tol=(0.2, 0.4, 0.0), needs=W,
  how="Runs of seasons one man was a club's head coach (`nfl_seasons`), begun after the world's first NFL season and ended before its last: the median."),
dict(id="S7.fbs_two_jobs", page="coaches", areas="S7 S8", title="FBS head coaches who held two or more FBS head jobs", unit="%",
  world=JOBS + """ SELECT AVG(n >= 2) FROM (SELECT c, COUNT(DISTINCT college) AS n FROM jobs WHERE role = 'Head coach' AND level = 'FBS' GROUP BY c)""", scale=100.0, years=None, detail=None,
  real=("Key", "college_people.json:fbs_head_coaches.coaches_with_two_plus_fbs_jobs_share", 100.0), tol=(0.2, 0.4, 0.0), needs=W,
  how="Coaches who ever held an FBS head job: the share who held two or more at different programs."),
]

RULES = [
dict(id="S7.no_overlap", areas="S7 T11", title="A coach's head jobs never overlap by more than a season",
  checked="SELECT COUNT(*) FROM coaches__history WHERE role = 'HeadCoach'",
  failures="""SELECT 'coach', c.name || ' (' || a._parent || ')', a."from", 'head jobs ' || a."from" || '-' || a."to" || ' and ' || b."from" || '-' || b."to"
   FROM coaches__history a JOIN coaches__history b ON b._parent = a._parent AND b._i > a._i JOIN coaches c ON c._row = a._parent
   WHERE a.role = 'HeadCoach' AND b.role = 'HeadCoach' AND b."from" < a."to" """,
  needs=W, why="A coach holds one head job at a time; the one-season overlap of a move (he finishes one season and starts the next job that year) is allowed."),
dict(id="S7.record_adds_up", areas="S7 C4 T11", title="A head coach's college record equals his program seasons",
  checked="SELECT COUNT(DISTINCT coach) FROM programs__seasons WHERE coach IS NOT NULL",
  failures=JOBS + """, r AS (SELECT c, SUM(w) AS w, SUM(l) AS l FROM jobs WHERE role = 'Head coach' AND college IS NOT NULL AND a > (SELECT MIN(year) FROM programs__seasons) GROUP BY c),
   s AS (SELECT coach, SUM(wins) AS w, SUM(losses) AS l FROM programs__seasons WHERE coach IS NOT NULL GROUP BY coach)
   SELECT 'coach', co.name || ' (' || r.c || ')', NULL, 'jobs say ' || r.w || '-' || r.l || ', his seasons say ' || s.w || '-' || s.l FROM r JOIN s ON s.coach = r.c JOIN coaches co ON co._row = r.c
   WHERE (r.w > s.w OR r.l > s.l) AND r.w + r.l > s.w + s.l + 1
     AND r.c NOT IN (SELECT _parent FROM coaches__history WHERE ended LIKE '%during the season%')""",
  needs=W, why="Wins credited to a head job come from the seasons he coached; jobs before the history began are left out. A coach fired during the season keeps that season's games while the season's row goes to the man who finished it, and a coach hired in December may coach the bowl, so those are left out."),
dict(id="S1.age_from_birth", areas="S1 T11", title="Every coach's age counts from his birth date",
  checked="SELECT COUNT(*) FROM coaches WHERE born IS NOT NULL",
  failures="""SELECT 'coach', name || ' (' || _row || ')', NULL, 'age ' || age || ', born ' || date(born * 86400, 'unixepoch') FROM coaches
   WHERE born IS NOT NULL AND ABS(age - (CAST(strftime('%Y', (SELECT MAX(year) FROM programs__seasons) + 1 || '-09-01') AS INTEGER) - CAST(strftime('%Y', date(born * 86400, 'unixepoch')) AS INTEGER))) > 1""",
  needs=W, why="Every age shown comes from the date (final build lane 1, A1)."),
]
