from common import *

def P(id, area, title, kind, note, sql, says=None, needs=W, open="", page="high-school"):
    return dict(id=id, page=page, area=area, title=title, kind=kind, note=note, sql=sql, says=says, needs=needs, open=open)

# Every high school season with its school and state.
HS = """WITH hs AS MATERIALIZED (SELECT s._parent AS school, h.name || ' (' || h.town || ')' AS name, st.code AS state, h.private AS private, h.enrollment AS enrollment,
        s.year AS year, s.wins AS w, s.losses AS l, s.state_title AS t, s.coach AS coach, json_extract(s.points, '$[0]') AS pf, json_extract(s.points, '$[1]') AS pa
   FROM high_schools__seasons s JOIN high_schools h ON h._row = s._parent JOIN states st ON st._row = h.state)"""

# Each state's real champions a year: its classes and private classes, or its equity brackets.
# (Select and Non-Select doubles the classes; non-public brackets, a private bracket and an Open Division add theirs.)
def N(c):
    return f"(CASE WHEN COALESCE({c}, '') = '' THEN 0 ELSE length({c}) - length(replace({c}, ';', '')) + 1 END)"
BRACKETS = """rb AS (SELECT state, (CASE WHEN brackets LIKE 'equity:%' THEN """ + N("substr(brackets, 8)") + """ WHEN brackets LIKE 'enrollment:%' THEN CAST(substr(brackets, 12) AS INTEGER) ELSE """ + N("classes") + """ END)
        * (CASE WHEN private_rule = 'select' THEN 2 ELSE 1 END) + """ + N("private_classes") + """
        + (CASE WHEN private_rule LIKE 'nonpublic:%' THEN """ + N("substr(private_rule, 11)") + """ WHEN private_rule LIKE 'private:%' THEN 1 ELSE 0 END)
        + (CASE WHEN COALESCE(open, '') <> '' THEN 1 ELSE 0 END) AS real
   FROM real_hs_playoff_formats)"""

# A state book's ten entries, one row each (`stats.books`).
BOOK = """bk AS (SELECT b._key AS k, j.value AS i, CASE j.value WHEN 0 THEN _0__value WHEN 1 THEN _1__value WHEN 2 THEN _2__value WHEN 3 THEN _3__value WHEN 4 THEN _4__value
        WHEN 5 THEN _5__value WHEN 6 THEN _6__value WHEN 7 THEN _7__value WHEN 8 THEN _8__value ELSE _9__value END AS value,
        CASE j.value WHEN 0 THEN _0__name WHEN 1 THEN _1__name WHEN 2 THEN _2__name WHEN 3 THEN _3__name WHEN 4 THEN _4__name
        WHEN 5 THEN _5__name WHEN 6 THEN _6__name WHEN 7 THEN _7__name WHEN 8 THEN _8__name ELSE _9__name END AS name,
        CASE j.value WHEN 0 THEN _0__year WHEN 1 THEN _1__year WHEN 2 THEN _2__year WHEN 3 THEN _3__year WHEN 4 THEN _4__year
        WHEN 5 THEN _5__year WHEN 6 THEN _6__year WHEN 7 THEN _7__year WHEN 8 THEN _8__year ELSE _9__year END AS year,
        CASE j.value WHEN 0 THEN _0__id WHEN 1 THEN _1__id WHEN 2 THEN _2__id WHEN 3 THEN _3__id WHEN 4 THEN _4__id
        WHEN 5 THEN _5__id WHEN 6 THEN _6__id WHEN 7 THEN _7__id WHEN 8 THEN _8__id ELSE _9__id END AS id
   FROM stats__books b, json_each('[0,1,2,3,4,5,6,7,8,9]') j WHERE b._key LIKE '[{"State":%')"""

# The 12 state book stats (`records::RECORD_STATS` as `hs_year.rs` HS_BOOKS uses them) and
# the 21 line stats (`HsStat`), with real season marks from memory where trial 10 has one.
HS_STATS = """hst(k, stat, line, real) AS (VALUES (0, 'Passing yards', 4, NULL), (1, 'Passing TDs', 5, NULL), (2, 'Rushing yards', 8, NULL), (3, 'Rushing TDs', 9, NULL),
        (4, 'Receptions', 10, NULL), (5, 'Receiving yards', 11, 1500), (6, 'Receiving TDs', 12, NULL), (7, 'Tackles', 13, NULL), (8, 'Sacks', 15, 40),
        (9, 'Interceptions', 16, 12), (10, 'Tackles for loss', 14, NULL), (11, 'Field goals', 18, NULL))"""

# Every line's record-book stats as columns, read once (sacks and tackles for loss in halves).
STAT_LINES = [4, 5, 8, 9, 10, 11, 12, 13, 15, 16, 14, 18]
LINES = "x AS MATERIALIZED (SELECT _parent, year, school, pos, games, " + ", ".join(
    f"json_extract(s, '$[{i}]')" + (" / 2.0" if i in (14, 15) else "") + f" AS c{i}" for i in STAT_LINES) + " FROM people__hs_lines)"

PANELS = [
# ---------- C23 schools ----------
P("C23.per_state", "C23", "High schools per state against the real count", "map",
  "Football-playing high schools in each state against the real count (MaxPreps 2026, `hs_states.csv` approx_programs). Hover a state for both.",
  """SELECT st.code AS state, COUNT(h._row) AS world, r.approx_programs AS real, COUNT(h._row) AS n
   FROM states st LEFT JOIN high_schools h ON h.state = st._row LEFT JOIN real_hs_states r ON r.state = st.code GROUP BY st._row ORDER BY st.code""",
  says="""WITH x AS (SELECT st.code, COUNT(h._row) AS w, r.approx_programs AS r FROM states st LEFT JOIN high_schools h ON h.state = st._row LEFT JOIN real_hs_states r ON r.state = st.code GROUP BY st._row)
   SELECT printf('%,d high schools against about %,d real; %d of %d states match their real count.', SUM(w), SUM(r), SUM(w = r), COUNT(*)) FROM x"""),
P("C23.real_powerhouses", "C23", "The real top five of each state in the world", "table",
  "Each state's real top five programs (MaxPreps 2026, `hs_states.csv` top_5_powerhouses), how many of them the world has, and where they rank by prestige in their state now. A real top program should sit near the top.",
  """WITH RECURSIVE sp(state, rest, one) AS (SELECT state, top_5_powerhouses || ';', NULL FROM real_hs_states
        UNION ALL SELECT state, substr(rest, instr(rest, ';') + 1), trim(substr(rest, 1, instr(rest, ';') - 1)) FROM sp WHERE rest <> ''),
   rk AS (SELECT h._row, h.name || ' (' || h.town || ')' AS name, st.code AS state, h.prestige, RANK() OVER (PARTITION BY h.state ORDER BY h.prestige DESC) AS rank FROM high_schools h JOIN states st ON st._row = h.state)
  SELECT sp.state, COUNT(sp.one) AS real_top5, COUNT(rk._row) AS in_world, group_concat(sp.one || CASE WHEN rk._row IS NULL THEN ' (missing)' ELSE ' #' || rk.rank END, '; ') AS ranks_now,
        ROUND(AVG(rk.prestige), 2) AS mean_prestige
   FROM sp LEFT JOIN rk ON rk.state = sp.state AND rk.name = sp.one WHERE sp.one IS NOT NULL AND sp.one <> '' GROUP BY sp.state ORDER BY in_world, sp.state""",
  says="""WITH RECURSIVE sp(state, rest, one) AS (SELECT state, top_5_powerhouses || ';', NULL FROM real_hs_states
        UNION ALL SELECT state, substr(rest, instr(rest, ';') + 1), trim(substr(rest, 1, instr(rest, ';') - 1)) FROM sp WHERE rest <> ''),
   rk AS (SELECT h.name || ' (' || h.town || ')' AS name, st.code AS state, RANK() OVER (PARTITION BY h.state ORDER BY h.prestige DESC) AS rank FROM high_schools h JOIN states st ON st._row = h.state)
  SELECT printf('%d of %d real top-five programs are in the world; %d of those rank in their state''s top ten by prestige now.', COUNT(rk.name), COUNT(*), SUM(rk.rank <= 10))
   FROM sp LEFT JOIN rk ON rk.state = sp.state AND rk.name = sp.one WHERE sp.one IS NOT NULL AND sp.one <> ''"""),
P("C23.powerhouses", "C23", "Powerhouses per state", "map",
  "Schools with prestige 0.75 or more (a powerhouse, `high_school.rs`) in each state now: a few per state, not dozens.",
  """SELECT st.code AS state, SUM(h.prestige >= 0.75) AS world, COUNT(*) AS n FROM high_schools h JOIN states st ON st._row = h.state GROUP BY st._row ORDER BY st.code""",
  says="""WITH x AS (SELECT st.code, SUM(h.prestige >= 0.75) AS n FROM high_schools h JOIN states st ON st._row = h.state GROUP BY st._row)
   SELECT printf('%d powerhouses (prestige 0.75+): %d to %d a state.', SUM(n), MIN(n), MAX(n)) FROM x"""),
P("C23.national", "C23", "The national powerhouses", "table",
  "The schools that pull from out of state (`high_school.rs` NATIONAL_POWERHOUSES): prestige, state titles, roster size, how many of the roster have a home in another state (a boarder keeps his; a family that moves takes the new state) and how many boarded. Real IMG Academy's roster is mostly out-of-state boarders (from memory).",
  """SELECT h._row AS id, h.name || ' (' || h.town || ')' AS school, st.code AS state, ROUND(h.prestige, 2) AS prestige, h.state_titles AS titles, json_array_length(h.roster) AS roster,
        (SELECT COUNT(*) FROM people x WHERE x.hs = +h._row AND x.stage = 'HighSchool' AND x.home_state <> h.state) AS from_out_of_state,
        (SELECT COUNT(*) FROM people x WHERE x.hs = +h._row AND x.stage = 'HighSchool' AND x.notes LIKE '%left home to board%') AS boarders
   FROM high_schools h JOIN states st ON st._row = h.state WHERE h.national = 1 ORDER BY h.prestige DESC""", open="e/high_schools/{id}"),
P("C23.prestige", "C23", "Prestige spread, public and private", "hist",
  "Every school's prestige now in steps of 0.05, public and private apart (pick above the chart). Prestige history by year needs the world log (next panel).",
  """SELECT CASE WHEN private = 1 THEN 'Private' ELSE 'Public' END AS grp, ROUND(CAST(prestige / 0.05 AS INTEGER) * 0.05, 2) AS x, COUNT(*) AS world FROM high_schools GROUP BY 1, 2 ORDER BY 1, 2"""),
P("C23.prestige_years", "C23", "Prestige by year", "line",
  "The spread of every school's prestige each year from the world log's yearly school rows (10th, 50th and 90th percentiles) and the number of powerhouses.",
  """WITH x AS (SELECT season AS year, school__prestige AS v FROM log_schools),
   r AS (SELECT year, v, ROW_NUMBER() OVER (PARTITION BY year ORDER BY v) AS r, COUNT(*) OVER (PARTITION BY year) AS n FROM x)
  SELECT year, """ + PCT(0.1) + " AS p10, " + PCT(0.5) + " AS p50, " + PCT(0.9) + """ AS p90, SUM(v >= 0.75) AS powerhouses FROM r GROUP BY year ORDER BY year""", needs=WL),
P("C23.money", "C23", "Resources by the area's money", "bars",
  "Mean school resources (0-1: weight room, staff, film, academic support) by the area's money in fifths, poorest first: resources should follow the town's income.",
  """WITH x AS (SELECT resources, area_money, NTILE(5) OVER (ORDER BY area_money) AS q FROM high_schools)
  SELECT 'Fifth ' || q || ' (money ' || printf('%.2f', MIN(area_money)) || '-' || printf('%.2f', MAX(area_money)) || ')' AS label, ROUND(AVG(resources), 3) AS world, COUNT(*) AS n FROM x GROUP BY q ORDER BY q"""),
P("C23.private", "C23", "Private schools and their titles by state", "table",
  "Each state's private share of schools against the private share of its state titles over the world's seasons; states that split public and private champions should show the two close.",
  HS + """ SELECT state, COUNT(DISTINCT school) AS schools, ROUND(100.0 * COUNT(DISTINCT CASE WHEN private = 1 THEN school END) / COUNT(DISTINCT school), 1) AS private_pct,
        SUM(t) AS titles, ROUND(100.0 * SUM(t * private) / NULLIF(SUM(t), 0), 1) AS private_title_pct,
        (SELECT COALESCE(private_classes, '') FROM real_hs_playoff_formats r WHERE r.state = hs.state) AS real_private_classes
   FROM hs GROUP BY state ORDER BY private_title_pct DESC"""),
P("C23.list", "C23", "Every high school", "table",
  "Each school's state, town, class, enrollment, private or public, resources, prestige, state titles and its head coach now. A row opens its page.",
  """SELECT h._row AS id, h.name || ' (' || h.town || ')' AS school, st.code AS state, h.classification + 1 AS class, h.enrollment, CASE WHEN h.private = 1 THEN 'private' ELSE 'public' END AS kind,
        ROUND(h.resources, 2) AS resources, ROUND(h.prestige, 3) AS prestige, h.state_titles AS titles, c.name AS coach
   FROM high_schools h JOIN states st ON st._row = h.state LEFT JOIN coaches c ON c._row = h.coach ORDER BY h.prestige DESC""", open="e/high_schools/{id}"),
# ---------- C24 seasons, titles, streaks ----------
P("C24.champions", "C24", "State champions a year by state", "bars",
  "Each state's champions in the world's last season against its real brackets a year (`hs_playoff_formats.csv`: its classes, doubled where Select and Non-Select split, plus private-association and non-public classes and an Open Division), biggest gaps first.",
  "WITH " + BRACKETS + """, w AS (SELECT st.code AS state, SUM(s.state_title) AS n FROM high_schools__seasons s JOIN high_schools h ON h._row = s._parent JOIN states st ON st._row = h.state
        WHERE s.year = (SELECT MAX(year) FROM high_schools__seasons) GROUP BY st.code)
  SELECT w.state AS label, w.n AS world, rb.real FROM w LEFT JOIN rb ON rb.state = w.state ORDER BY ABS(w.n - COALESCE(rb.real, w.n)) DESC, w.state""",
  says="WITH " + BRACKETS + """, w AS (SELECT st.code AS state, SUM(s.state_title) AS n FROM high_schools__seasons s JOIN high_schools h ON h._row = s._parent JOIN states st ON st._row = h.state
        WHERE s.year = (SELECT MAX(year) FROM high_schools__seasons) GROUP BY st.code)
  SELECT printf('%d state champions last season against %d real brackets; %d states match their real count.', SUM(w.n), SUM(rb.real), SUM(w.n = rb.real)) FROM w JOIN rb ON rb.state = w.state"""),
P("C24.most_15y", "C24", "Most state titles by one school in 15 seasons", "table",
  "For each school, the most state titles it won in any 15 straight seasons, with the seasons; the top 50. Real powerhouses rarely win more than about 10 in 15 (from memory, no open record book).",
  HS + """, t AS (SELECT school, name, state, year FROM hs WHERE t = 1),
   win AS (SELECT a.school, a.name, a.state, a.year AS start, COUNT(b.year) AS n FROM t a JOIN t b ON b.school = a.school AND b.year BETWEEN a.year AND a.year + 14 GROUP BY a.school, a.year)
  SELECT school AS id, name AS school, state, MAX(n) AS titles_in_15, start || '-' || (start + 14) AS seasons FROM win GROUP BY school ORDER BY titles_in_15 DESC LIMIT 50""", open="e/high_schools/{id}",
  says=HS + """, t AS (SELECT school, name, year FROM hs WHERE t = 1),
   win AS (SELECT a.name, a.year AS start, COUNT(b.year) AS n FROM t a JOIN t b ON b.school = a.school AND b.year BETWEEN a.year AND a.year + 14 GROUP BY a.school, a.year)
  SELECT printf('%s won %d state titles in 15 seasons (%d-%d).', name, n, start, start + 14) FROM win ORDER BY n DESC LIMIT 1"""),
P("C24.streaks", "C24", "Longest winning streaks", "table",
  "The state record books' winning streaks (`stats.books`, WinStreak), longest first across states, with the season each ended or the year it is still running. Real record: De La Salle's 151 (1992-2004).",
  "WITH " + BOOK + """ SELECT CAST(json_extract(bk.k, '$[0].State') AS INTEGER) AS sidx, (SELECT code FROM states WHERE _row = json_extract(bk.k, '$[0].State')) AS state, bk.name AS school, CAST(bk.value AS INTEGER) AS games, bk.year,
        151 AS real_record FROM bk WHERE bk.k LIKE '%"WinStreak"%' AND bk.value IS NOT NULL ORDER BY bk.value DESC LIMIT 100""",
  says="WITH " + BOOK + """ SELECT printf('Longest streak %d straight (%s, %d); %d streaks of 100+; the real record is 151.', MAX(value), (SELECT name FROM bk b2 WHERE b2.k LIKE '%"WinStreak"%' ORDER BY value DESC LIMIT 1),
        (SELECT year FROM bk b2 WHERE b2.k LIKE '%"WinStreak"%' ORDER BY value DESC LIMIT 1), SUM(value >= 100)) FROM bk WHERE k LIKE '%"WinStreak"%' HAVING MAX(value) > 0"""),
P("C24.active_streaks", "C24", "Longest winning streaks still running", "table",
  "Schools with the longest winning streak still running now (`stats.hs_streak`).",
  """SELECT h._row AS id, h.name || ' (' || h.town || ')' AS school, st.code AS state, x._0 AS games, ROUND(h.prestige, 2) AS prestige
   FROM stats__hs_streak x JOIN high_schools h ON h._row = x._row JOIN states st ON st._row = h.state WHERE x._0 > 0 ORDER BY x._0 DESC LIMIT 50""", open="e/high_schools/{id}"),
P("C24.unbeaten", "C24", "Unbeaten seasons and champions a year", "line",
  "Each season: schools with no losses, state champions, and champions who went unbeaten.",
  HS + """ SELECT year, SUM(l = 0 AND w > 0) AS unbeaten, SUM(t) AS champions, SUM(t = 1 AND l = 0) AS unbeaten_champions FROM hs GROUP BY year ORDER BY year"""),
P("C24.dynasties", "C24", "Most state titles", "table",
  "Schools with the most state titles in the world's seasons, with their first and last title and their best run of straight titles.",
  HS + """, t AS (SELECT school, name, state, year, year - ROW_NUMBER() OVER (PARTITION BY school ORDER BY year) AS run FROM hs WHERE t = 1),
   r AS (SELECT school, COUNT(*) AS n, MIN(year) AS a, MAX(year) AS z FROM t GROUP BY school, run)
  SELECT t.school AS id, t.name AS school, t.state, COUNT(*) AS titles, MIN(t.year) AS first, MAX(t.year) AS last, (SELECT MAX(n) FROM r WHERE r.school = t.school) AS best_run
   FROM t GROUP BY t.school ORDER BY titles DESC LIMIT 60""", open="e/high_schools/{id}"),
P("C24.by_state", "C24", "Champions by state and season", "table",
  "Every state champion by season (pick a state above; the slider picks a season), with its record.",
  HS + """ SELECT state AS grp, year, school AS id, name AS champion, w || '-' || l AS record, CASE WHEN private = 1 THEN 'private' ELSE 'public' END AS kind FROM hs WHERE t = 1 ORDER BY state, year DESC, name""",
  open="e/high_schools/{id}"),
# ---------- C25 the high school year ----------
P("C25.dates", "C25", "Each state's season this year", "table",
  "Each state's first game and finals this season (`hs_season.states`) beside the real playoff start and finals (SI 2026 guide, `hs_states.csv` playoff_format).",
  """SELECT st.code AS state, date(x.start * 86400, 'unixepoch') AS start, date(x.finals * 86400, 'unixepoch') AS finals, json_array_length(x.weeks) AS regular_weeks, json_array_length(x.rounds) AS playoff_rounds,
        r.playoff_format AS real_playoffs
   FROM hs_season__states x JOIN states st ON st._row = x._row LEFT JOIN real_hs_states r ON r.state = st.code ORDER BY x.start, st.code"""),
P("C25.counters", "C25", "The high school year's counters", "line",
  "The calendar log's yearly counts: forfeits, no contests, grade-outs, homecomings, senior nights, all-state picks and heat-limited practice days.",
  """SELECT year, hs_forfeits AS forfeits, hs_no_contests AS no_contests, hs_grade_outs AS grade_outs, hs_homecomings AS homecomings, hs_senior_nights AS senior_nights, hs_all_state AS all_state,
        heat_limited_days FROM log_history ORDER BY year"""),
P("C25.engine", "C25", "Games on the match engine", "line",
  "High school games each year and how many ran on the match engine (2014 on); stand-in seasons have no points.",
  """SELECT year, hs_games AS games, hs_engine_games AS engine_games FROM log_history WHERE hs_games > 0 ORDER BY year"""),
P("C25.all_state", "C25", "All-state picks per 100 high school players", "map",
  "Current high school players named all-state at least once, per 100 players, by the state they play in.",
  """SELECT st.code AS state, ROUND(100.0 * AVG(COALESCE(x.cal__all_state, 0) > 0), 1) AS world, COUNT(*) AS n
   FROM people x JOIN high_schools h ON h._row = x.hs JOIN states st ON st._row = h.state WHERE x.stage = 'HighSchool' GROUP BY st._row ORDER BY st.code"""),
P("C25.spring", "C25", "Spring ball: notices, breakouts and fizzles", "line",
  "Each year's spring notices, breakouts and fizzles (the calendar log).",
  """SELECT year, spring_notices AS notices, spring_breakouts AS breakouts, spring_fizzles AS fizzles FROM log_history WHERE spring_notices > 0 ORDER BY year"""),
# ---------- C26 coaches ----------
P("C26.tenure", "C26", "High school head coaches' years in the job", "hist",
  "Current head coaches' seasons in their job, and finished high school head jobs' lengths (pick above). Real long tenures are common (no open count).",
  """SELECT 'Current' AS grp, MIN((SELECT MAX(year) FROM high_schools__seasons) - job__since + 1, 30) AS x, COUNT(*) AS world FROM coaches WHERE job__employer__HighSchool IS NOT NULL AND job__role = 'HeadCoach' GROUP BY x
   UNION ALL SELECT 'Finished', MIN("to" - "from" + 1, 30), COUNT(*) FROM coaches__history WHERE employer__HighSchool IS NOT NULL AND role = 'HeadCoach' GROUP BY 2 ORDER BY 1, 2"""),
P("C26.churn", "C26", "Head coach changes a season, by school prestige", "bars",
  "The share of seasons a school starts with a new head coach (the season's coach against last season's), by prestige band, and the season after a state title. Real dynasties keep their coach for decades (Bruce Rollinson at Mater Dei 1989-2021, from memory).",
  """WITH s AS MATERIALIZED (SELECT s._parent AS h, s.coach AS c, LAG(s.coach) OVER (PARTITION BY s._parent ORDER BY s.year) AS prev,
          LAG(s.state_title) OVER (PARTITION BY s._parent ORDER BY s.year) AS won FROM high_schools__seasons s)
   SELECT CASE WHEN h.prestige >= 0.85 THEN '1 Prestige 0.85 and up' WHEN h.prestige >= 0.6 THEN '2 Prestige 0.6-0.85' WHEN h.prestige >= 0.3 THEN '3 Prestige 0.3-0.6' ELSE '4 Prestige under 0.3' END AS label,
          ROUND(100.0 * AVG(s.c <> s.prev), 1) AS world, COUNT(*) AS n FROM s JOIN high_schools h ON h._row = s.h WHERE s.prev IS NOT NULL AND s.c IS NOT NULL GROUP BY label
   UNION ALL SELECT '5 The season after a state title', ROUND(100.0 * AVG(c <> prev), 1), COUNT(*) FROM s WHERE prev IS NOT NULL AND c IS NOT NULL AND won = 1
   UNION ALL SELECT '6 The season after no title', ROUND(100.0 * AVG(c <> prev), 1), COUNT(*) FROM s WHERE prev IS NOT NULL AND c IS NOT NULL AND won = 0 ORDER BY label"""),
P("C26.churn_schools", "C26", "Schools that change head coach most", "table",
  "Schools by head coach changes across the world's seasons, with their state titles: a school that wins every year should keep its coach.",
  """WITH s AS MATERIALIZED (SELECT s._parent AS h, s.state_title AS t, s.coach AS c, LAG(s.coach) OVER (PARTITION BY s._parent ORDER BY s.year) AS prev FROM high_schools__seasons s)
   SELECT h._row AS id, h.name AS school, st.code AS state, ROUND(h.prestige, 2) AS prestige, COUNT(*) + 1 AS seasons, SUM(s.c <> s.prev) AS changes, SUM(s.t) AS titles
   FROM s JOIN high_schools h ON h._row = s.h LEFT JOIN states st ON st._row = h.state WHERE s.prev IS NOT NULL AND s.c IS NOT NULL
   GROUP BY s.h ORDER BY changes DESC, titles DESC LIMIT 100""", open="e/high_schools/{id}"),
P("C26.titles", "C26", "Coaches with the most state titles", "table",
  "Head coaches credited with the most state titles (the season's coach), with their schools and career high school record.",
  HS + """ SELECT hs.coach AS id, c.name, SUM(hs.t) AS titles, COUNT(*) AS seasons, SUM(hs.w) || '-' || SUM(hs.l) AS record, group_concat(DISTINCT hs.name) AS schools
   FROM hs JOIN coaches c ON c._row = hs.coach WHERE hs.coach IS NOT NULL GROUP BY hs.coach HAVING SUM(hs.t) > 0 ORDER BY titles DESC, SUM(hs.w) DESC LIMIT 60""", open="e/coaches/{id}"),
P("C26.wins", "C26", "Most high school wins by a coach", "table",
  "Head coaches by high school wins as the season's coach (real ladder: John McKissick 621 at Summerville, SC).",
  HS + """ SELECT hs.coach AS id, c.name, SUM(hs.w) AS wins, SUM(hs.l) AS losses, COUNT(*) AS seasons, SUM(hs.t) AS titles, MIN(hs.year) || '-' || MAX(hs.year) AS years, 621 AS real_record
   FROM hs JOIN coaches c ON c._row = hs.coach WHERE hs.coach IS NOT NULL GROUP BY hs.coach ORDER BY wins DESC LIMIT 60""", open="e/coaches/{id}"),
P("C26.to_college", "C26", "High school coaches moving into college", "line",
  "Each year: high school coaches whose next job was at a college, by the college's level now (rare in real life).",
  ST + """, n AS (SELECT c, a, level, LAG(level) OVER (PARTITION BY c ORDER BY a) AS before FROM st)
  SELECT a AS year, SUM(level = 'FBS') AS fbs, SUM(level = 'FCS') AS fcs, SUM(level IN ('D2', 'D3', 'NAIA', 'JUCO')) AS lower FROM n WHERE before = 'HS' AND level NOT IN ('HS', 'NFL', 'Other') GROUP BY a ORDER BY a"""),
P("C26.network", "C26", "A coach's network and how seen his players are", "bars",
  "Current high school players' mean visibility (0-1) by their head coach's network rating (hidden, 1-20) in bands, with their mean stars: a big network should get players seen more, at the same talent.",
  """SELECT CASE WHEN c.ability__network < 7 THEN 'Network under 7' WHEN c.ability__network < 10 THEN '7-10' WHEN c.ability__network < 13 THEN '10-13' WHEN c.ability__network < 16 THEN '13-16' ELSE '16+' END AS label,
        ROUND(AVG(x.visibility), 3) AS world, ROUND(AVG(x.stars), 2) AS stars, ROUND(AVG(x.x_overall), 1) AS overall, COUNT(*) AS n
   FROM people x JOIN high_schools h ON h._row = x.hs JOIN coaches c ON c._row = h.coach
   WHERE x.stage = 'HighSchool' GROUP BY 1 ORDER BY MIN(c.ability__network)"""),
# ---------- C27 moves and ages ----------
P("C27.transfers", "C27", "Top recruits who changed high schools", "line",
  "Of each class's 100 best recruits by stars and visibility, how many have a high school transfer note (`notes`: 'transferred from'). Real: many top recruits move at least once (no open count).",
  """WITH x AS (SELECT class_of, notes, ROW_NUMBER() OVER (PARTITION BY class_of ORDER BY stars DESC, visibility DESC) AS r FROM people WHERE class_of IS NOT NULL AND stage IN ('HighSchool', 'College', 'Declared', 'Pro'))
  SELECT class_of AS year, SUM(notes LIKE '%transferred from%') AS of_top_100, SUM(notes LIKE '%left home to board%') AS boarded FROM x WHERE r <= 100 GROUP BY class_of HAVING COUNT(*) = 100 ORDER BY class_of"""),
P("C27.draw", "C27", "Where the national powerhouses' players come from", "table",
  "Each national powerhouse's current players by home state, most first.",
  """SELECT h._row AS id, h.name || ' (' || h.town || ')' AS school, sh.code AS from_state, COUNT(*) AS players
   FROM high_schools h JOIN people x ON x.hs = h._row AND x.stage = 'HighSchool' JOIN states sh ON sh._row = x.home_state WHERE h.national = 1 GROUP BY h._row, sh.code ORDER BY h._row, players DESC""", open="e/high_schools/{id}"),
P("C27.sit_out", "C27", "Transfers by the state's sit-out rule", "bars",
  "The share of current high school players with a transfer note, by their state's rule (`states.transfer`: a sit-out or not); a sit-out should mean fewer moves.",
  """SELECT CASE WHEN st.transfer__sit_out > 0 THEN 'Sit-out rule' ELSE 'No sit-out' END || CASE WHEN st.transfer__free_once = 1 THEN ', one free move' ELSE '' END AS label,
        ROUND(100.0 * AVG(x.notes LIKE '%transferred from%'), 2) AS world, COUNT(*) AS n
   FROM people x JOIN high_schools h ON h._row = x.hs JOIN states st ON st._row = h.state WHERE x.stage = 'HighSchool' GROUP BY 1 ORDER BY 1"""),
# ---------- G7 games ----------
P("G7.per_game", "G7", "Points, plays and yards a team game", "line",
  "Each engine year's high school totals per team game (the calendar log). Real points about 26 a team game, from memory.",
  """SELECT year, ROUND(1.0 * hs_totals__points_for / hs_totals__games, 1) AS points, ROUND(1.0 * hs_totals__plays / hs_totals__games, 1) AS plays,
        ROUND(1.0 * hs_totals__yards / hs_totals__plays, 2) AS yards_a_play, ROUND(1.0 * hs_totals__turnovers / hs_totals__games, 2) AS turnovers, 26 AS real
   FROM log_history WHERE hs_totals__games > 0 ORDER BY year"""),
P("G7.state_points", "G7", "Points a team game by state", "map",
  "Points for over games in each state's school seasons on the engine (2014 on).",
  HS + """ SELECT state, ROUND(1.0 * SUM(pf) / NULLIF(SUM(w + l), 0), 1) AS world, 26 AS real, COUNT(*) AS n FROM hs WHERE pf > 0 GROUP BY state ORDER BY state"""),
P("G7.brackets", "G7", "This season's classes and brackets by state", "table",
  "Each state's classes this season (`hs_season.groups`): schools, survivors, against the real classes and field sizes (`hs_playoff_formats.csv`).",
  """SELECT st.code AS state, COUNT(*) AS classes, SUM(json_array_length(g.schools)) AS schools, MIN(json_array_length(g.schools)) || '-' || MAX(json_array_length(g.schools)) AS class_sizes,
        r.classes AS real_classes, r.private_classes AS real_private, r.fields AS real_fields
   FROM hs_season__groups g JOIN states st ON st._row = g.state LEFT JOIN real_hs_playoff_formats r ON r.state = st.code GROUP BY st._row ORDER BY st.code"""),
P("G7.records", "G7", "Season records a year", "line",
  "Shares of school seasons at 10+ wins, at 0 wins and the mean games a school played, by year.",
  HS + """ SELECT year, ROUND(100.0 * AVG(w >= 10), 1) AS pct_10_wins, ROUND(100.0 * AVG(w = 0), 1) AS pct_winless, ROUND(AVG(w + l), 2) AS games FROM hs GROUP BY year ORDER BY year"""),
# ---------- G27 stat lines and record books ----------
P("G27.bests", "G27", "Best high school seasons against real", "bars",
  "The best season in the world's high school lines (engine years) for each record-book stat, against a real season mark where trial 10 has one (from memory: receiving 1,500 yards is common among leaders, sacks record about 40, interceptions 10-15).",
  "WITH " + HS_STATS + ", " + LINES + """, m AS (SELECT """ + ", ".join(f"MAX(c{i}) AS m{i}" for i in STAT_LINES) + """ FROM x)
  SELECT hst.stat AS label, CASE hst.line """ + " ".join(f"WHEN {i} THEN m{i}" for i in STAT_LINES) + """ END AS world, hst.real FROM hst, m ORDER BY hst.k"""),
P("G27.leaders", "G27", "Season leaders by stat", "table",
  "The 25 best high school seasons for each record-book stat (pick a stat above), with player, school, year and games. A row opens the player.",
  "WITH " + HS_STATS + ", " + LINES + ", y AS (" + " UNION ALL ".join(f"SELECT * FROM (SELECT {i} AS line, _parent, year, school, pos, games, c{i} AS v FROM x ORDER BY c{i} DESC LIMIT 25)" for i in STAT_LINES) + """),
   r AS (SELECT hst.stat AS grp, hst.k, y.*, ROW_NUMBER() OVER (PARTITION BY y.line ORDER BY y.v DESC) AS rk FROM y JOIN hst ON hst.line = y.line)
  SELECT r.grp, r.rk AS rank, p.player__id AS id, p.name, r.pos, h.name || ' (' || h.town || ')' AS school, r.year, r.games, r.v AS value
   FROM r JOIN people p ON p._row = r._parent LEFT JOIN high_schools h ON h._row = r.school WHERE r.rk <= 25 ORDER BY r.k, r.rk""", open="e/people/{id}"),
P("G27.big_seasons", "G27", "Big seasons a year", "line",
  "Each year's count of seasons with 30+ sacks (impossible-looking; real record about 40), 1,500+ receiving yards (common among real leaders) and 10+ interceptions.",
  """SELECT year, SUM(json_extract(s, '$[15]') >= 60) AS sacks_30, SUM(json_extract(s, '$[11]') >= 1500) AS rec_1500, SUM(json_extract(s, '$[16]') >= 10) AS int_10, COUNT(*) AS lines
   FROM people__hs_lines GROUP BY year ORDER BY year"""),
P("G27.state_books", "G27", "State record books", "table",
  "Each state's record book (pick a state above): the top entry of every season and career list and the winning streak (`stats.books`).",
  "WITH " + HS_STATS + ", " + BOOK + """ SELECT (SELECT code FROM states WHERE _row = json_extract(bk.k, '$[0].State')) AS grp,
        CASE WHEN bk.k LIKE '%WinStreak%' THEN 'Winning streak' WHEN bk.k LIKE '%Season%' THEN 'Season: ' || (SELECT stat FROM hst WHERE k = json_extract(bk.k, '$[1].Season'))
             ELSE 'Career: ' || (SELECT stat FROM hst WHERE k = json_extract(bk.k, '$[1].Career')) END AS book,
        bk.name AS holder, ROUND(bk.value, 1) AS value, bk.year
   FROM bk WHERE bk.i = 0 ORDER BY grp, book"""),
# ---------- N32 budgets ----------
P("N32.budgets", "N32", "High school football spending by state", "table",
  "Each state's high school football spending last money year beyond coaching stipends (equipment, travel, game day, field, booster purchases, meals; `finance.hs` last): the median, the top tenth and the biggest, with the median crowd. Real: a typical public program spends $25-60K; big Texas programs $150K+ (`finance_check.rs`).",
  """WITH x AS (SELECT st.code AS state, (SELECT SUM(value) FROM json_each(f.last__l) WHERE key >= 6) AS v, f.crowd FROM finance__hs f JOIN high_schools h ON h._row = f._row JOIN states st ON st._row = h.state),
   r AS (SELECT state, v, crowd, ROW_NUMBER() OVER (PARTITION BY state ORDER BY v) AS r, ROW_NUMBER() OVER (PARTITION BY state ORDER BY crowd) AS rc, COUNT(*) OVER (PARTITION BY state) AS n FROM x)
  SELECT state, ROUND(""" + PCT(0.5) + ") AS median, ROUND(" + PCT(0.9) + """) AS p90, ROUND(MAX(v)) AS biggest, MAX(CASE WHEN rc = CAST(0.5 * (n - 1) AS INTEGER) + 1 THEN crowd END) AS median_crowd, MAX(n) AS schools
   FROM r GROUP BY state ORDER BY median DESC""",
  says="""WITH x AS (SELECT (SELECT SUM(value) FROM json_each(f.last__l) WHERE key >= 6) AS v FROM finance__hs f), r AS (SELECT v, ROW_NUMBER() OVER (ORDER BY v) AS r, COUNT(*) OVER () AS n FROM x)
  SELECT printf('Median school spends $%,d a year beyond stipends (real public programs $25-60K); the top tenth $%,d or more.', """ + PCT(0.5) + ", " + PCT(0.9) + """) FROM r"""),
P("N32.lines", "N32", "Where the money comes from and goes", "bars",
  "Every school's books last money year summed by line (`finance.hs` LINES), in dollars a school.",
  """WITH l(k, name) AS (VALUES (0, 'District allocation'), (1, 'Gate receipts'), (2, 'Playoff gate share'), (3, 'Pay-to-play fees'), (4, 'Booster club'), (5, 'Coaching stipends'),
        (6, 'Equipment and reconditioning'), (7, 'Buses and travel'), (8, 'Officials and game day'), (9, 'Field, lights and upkeep'), (10, 'Booster purchases'), (11, 'Team meals'))
  SELECT l.name AS label, ROUND(AVG(j.value)) AS world, COUNT(*) AS n FROM finance__hs f, json_each(f.last__l) j JOIN l ON l.k = j.key GROUP BY l.k ORDER BY l.k"""),
]

CHECKS = [
dict(id="C23.schools", page="high-school", areas="C23", title="High schools with football", unit="",
  world="SELECT COUNT(*) FROM high_schools", scale=1.0, years=None,
  detail="""SELECT 'Most: ' || group_concat(code || ' ' || n, ', ') || '.' FROM (SELECT st.code, COUNT(*) AS n FROM high_schools h JOIN states st ON st._row = h.state GROUP BY st._row ORDER BY n DESC LIMIT 3)""",
  real=("Fixed", 14134.0, "hs_states.csv approx_programs summed (MaxPreps 2026)", "2026"), tol=(0.03, 0.08, 0.0), needs=W,
  how="Every high school in the world."),
dict(id="C24.champions", page="high-school", areas="C24 G7", title="State champions a year", unit="",
  world="SELECT SUM(state_title) FROM high_schools__seasons WHERE year = (SELECT MAX(year) FROM high_schools__seasons)", scale=1.0,
  years="SELECT year, SUM(state_title) FROM high_schools__seasons GROUP BY year", detail=None,
  real=("Fixed", 350.0, "hs_playoff_formats.csv: each state's classes (doubled for Select and Non-Select), private-association and non-public classes, Open Divisions", "2025-26"), tol=(0.05, 0.15, 0.0), needs=W,
  how="School seasons with a state title in the world's last season."),
dict(id="C24.streak", page="high-school", areas="C24 G7", title="Longest high school winning streak", unit="games",
  world="""SELECT MAX(MAX(COALESCE(_0__value, 0), COALESCE(_1__value, 0))) FROM stats__books WHERE _key LIKE '%"WinStreak"%'""", scale=1.0, years=None,
  detail="""SELECT _0__name || ', ended or running in ' || _0__year || '.' FROM stats__books WHERE _key LIKE '%"WinStreak"%' ORDER BY _0__value DESC LIMIT 1""",
  real=("Memory", 151.0, "De La Salle (Concord, CA), 1992-2004: the national record"), tol=(0.15, 0.35, 0.0), needs=W,
  how="The longest winning streak in the state record books."),
dict(id="G27.sacks", page="high-school", areas="G27", title="Most sacks in a high school season", unit="sacks",
  world="SELECT MAX(json_extract(s, '$[15]')) / 2.0 FROM people__hs_lines", scale=1.0, years=None,
  detail="""SELECT p.name || ' (' || l.pos || '), ' || l.year || ': ' || (json_extract(l.s, '$[15]') / 2.0) || ' sacks in ' || l.games || ' games.' FROM people__hs_lines l JOIN people p ON p._row = l._parent ORDER BY json_extract(l.s, '$[15]') DESC LIMIT 1""",
  real=("Memory", 40.0, "trial 10: the real season record is about 40, from memory"), tol=(0.15, 0.4, 0.0), needs=W,
  how="The best season's sacks in the high school lines (kept in halves)."),
dict(id="G27.interceptions", page="high-school", areas="G27", title="Most interceptions in a high school season", unit="",
  world="SELECT MAX(json_extract(s, '$[16]')) FROM people__hs_lines", scale=1.0, years=None, detail=None,
  real=("Memory", 12.0, "trial 10: real leaders 10-15 a season, from memory"), tol=(0.2, 0.4, 0.0), needs=W,
  how="The best season's interceptions in the high school lines."),
]

RULES = [
dict(id="C24.wins_losses", areas="C24 T11", title="High school wins equal losses each season",
  checked="SELECT COUNT(DISTINCT year) FROM high_schools__seasons",
  failures="""SELECT 'season', year, year, SUM(wins) || ' wins against ' || SUM(losses) || ' losses' FROM high_schools__seasons GROUP BY year HAVING SUM(wins) <> SUM(losses)""",
  needs=W, why="Every high school game is between two schools with a winner (forfeits count both ways; `hs_year.rs`)."),
dict(id="C24.titles_count", areas="C24 T11", title="Each school's title count matches its seasons",
  checked="SELECT COUNT(*) FROM high_schools",
  failures="""WITH t AS (SELECT _parent, SUM(state_title) AS n FROM high_schools__seasons GROUP BY _parent)
   SELECT 'high school', h.name || ' (' || h._row || ')', NULL, h.state_titles || ' titles stored, ' || t.n || ' in its seasons'
   FROM high_schools h JOIN t ON t._parent = h._row WHERE h.state_titles < t.n""",
  needs=W, why="`state_titles` counts every title, the real ones before the world's seasons included, so it can only be at or above the seasons' count."),
dict(id="C24.one_champion", areas="C24 T11", title="One champion per class each season",
  checked="SELECT COUNT(*) FROM hs_season__groups",
  failures="""SELECT 'state', st.code, (SELECT MAX(year) FROM high_schools__seasons), COUNT(*) || ' classes, ' || (SELECT SUM(s.state_title) FROM high_schools__seasons s JOIN high_schools h ON h._row = s._parent
        WHERE h.state = g.state AND s.year = (SELECT MAX(year) FROM high_schools__seasons)) || ' champions'
   FROM hs_season__groups g JOIN states st ON st._row = g.state GROUP BY g.state
   HAVING COUNT(*) <> (SELECT SUM(s.state_title) FROM high_schools__seasons s JOIN high_schools h ON h._row = s._parent WHERE h.state = g.state AND s.year = (SELECT MAX(year) FROM high_schools__seasons))""",
  needs=W, why="Each class's bracket crowns one champion (`hs_playoffs.rs`)."),
]
