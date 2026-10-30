# Shared SQL pieces for lane R7's panels and checks.

W = "world file"
WL = "world log"
GL = "game log"

# Every program season with its level and conference that year: the real
# history through 2025 (programs__history), else today's.
PS = """WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
        s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
        s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
        COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
   FROM programs__seasons s JOIN programs p ON p._row = s._parent
   LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1)"""

# A power program that season (world/voters.rs power_conf by era).
POWER = """(conference IN ('SEC', 'Big Ten', 'ACC') OR (conference = 'Big 12' AND year >= 1996)
        OR (conference IN ('Big Eight', 'SWC') AND year <= 1995) OR (conference IN ('Pac-10', 'Pac-12') AND year <= 2023)
        OR (conference = 'Big East' AND year <= 2013) OR name = 'Notre Dame')"""

# A coach's current job with its level (the program's division today).
CJ = """WITH cj AS MATERIALIZED (SELECT c.*, CASE WHEN c.job__employer__College IS NOT NULL THEN (SELECT division FROM programs WHERE _row = c.job__employer__College)
        WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS' WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level
   FROM coaches c WHERE c.job__role IS NOT NULL)"""

# Every finished job (stint) with its level.
ST = """WITH st AS MATERIALIZED (SELECT h._parent AS c, h.role AS role, h."from" AS a, h."to" AS z, h.wins AS w, h.losses AS l, h.titles AS titles, h.ended AS ended,
        CASE WHEN h.employer__College IS NOT NULL THEN (SELECT division FROM programs WHERE _row = h.employer__College)
             WHEN h.employer__HighSchool IS NOT NULL THEN 'HS' WHEN h.employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level,
        h.employer__College AS college, h.employer__HighSchool AS hs, h.employer__Nfl AS nfl
   FROM coaches__history h)"""

# The 12 grades and the 8 teaching groups by index.
GRADES = """g(k, grade) AS (VALUES (0, 'Academics'), (1, 'Pro pipeline'), (2, 'Playing time'), (3, 'Tradition'), (4, 'Title chances'),
        (5, 'Coach prestige'), (6, 'Facilities'), (7, 'NIL'), (8, 'Brand exposure'), (9, 'Conference strength'), (10, 'Stadium atmosphere'), (11, 'Campus life'))"""
GROUPS = """grp(k, grp) AS (VALUES (0, 'QB'), (1, 'RB'), (2, 'Receivers'), (3, 'OL'), (4, 'DL'), (5, 'LB'), (6, 'DB'), (7, 'Specialists'))"""

# The real data files as JSON.
def RD(f):
    return f"(SELECT json FROM real_data WHERE file = '{f}')"

# Percentile columns of `v` within partitions (needs `r` and `n` from a window).
def PCT(p, v="v"):
    return f"MAX(CASE WHEN r = CAST({p} * (n - 1) AS INTEGER) + 1 THEN {v} END)"

# Head coach value from the hidden parts (world/coach.rs head_coach_value).
HCV = """(0.3 * (SELECT AVG(value) FROM json_each(ability__teaching)) + 0.2 * ability__scheme + 0.15 * ability__game_management
        + 0.15 * ability__motivation + 0.1 * ability__discipline + 0.1 * ability__relationships)"""
