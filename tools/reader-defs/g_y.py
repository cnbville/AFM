from common import *

WP = "world"
def DT(d): return f"date({d} * 86400, 'unixepoch')"
def YR(d): return f"CAST(strftime('%Y', {DT(d)}) AS INTEGER)"
def team(nfl, college, y="NULL"):
    return f"COALESCE({club(nfl, y) if y != 'NULL' else cname(nfl)}, (SELECT {prog('p')} FROM programs p WHERE p._row = {college}))"
def ids_names(js, n=5):
    """The first n names of a JSON list of player ids."""
    return f"(SELECT group_concat(nm, ', ') FROM (SELECT {pname('j.value')} AS nm FROM json_each({js}) j WHERE j.key < {n} ORDER BY j.key))"
def pos_of(idx):
    return f"COALESCE((SELECT player__position FROM people WHERE player__id = {idx}), (SELECT position FROM alumni WHERE player__id = {idx}))"

# ---------------------------------------------------------------- N41 stories
panel("N41.nicknames", WP, "N41", "Every nickname the world coined", "table",
  "Each nickname: who carries it, the name, kind and theme, the day it was coined, by whom and where, the moment behind it, the team, how well it stuck, the reaction, merchandise, and whether it was retired. Real: a few nicknames stick each year league-wide, nearly all for stars and big moments (stories_facts.md).",
  f"""SELECT {YR('n.coined')} AS year, COALESCE({pname('n.who__Player')}, (SELECT name FROM coaches WHERE id = n.who__Coach)) AS who, n.who__Player AS id, n.name AS nickname, n.kind, n.theme,
            {DT('n.coined')} AS coined, n.by, n.outlet, n.moment, {team('n.team__Nfl', 'n.team__College')} AS team, ROUND(n.stick, 2) AS stick, n.reaction,
            CASE WHEN n.merch THEN 'yes' ELSE '' END AS merch, CASE WHEN n.retired THEN 'retired' ELSE '' END AS retired, n.last_year AS "last used"
       FROM stories__nicknames n ORDER BY n.coined DESC""", open="e/people/{id}",
  says=f"""SELECT printf('%d nicknames over %d years: %.1f a year; %.0f%% stuck (stick 0.8 or more), %d sold merchandise.', COUNT(*), MAX(y) - MIN(y) + 1, 1.0 * COUNT(*) / (MAX(y) - MIN(y) + 1),
     100.0 * SUM(stick >= 0.8) / COUNT(*), SUM(merch)) FROM (SELECT {YR('coined')} AS y, stick, merch FROM stories__nicknames)""")
for c_ in ["who__Coach", "team__College", "who__Player", "team__Nfl", "merch", "retired", "last_year"]:
    col("stories__nicknames", c_)
panel("N41.units", WP, "N41", "Unit names", "table",
  "Names a unit earned (a defense, a secondary, an offense or a line): the team, the unit, the name, the years it lasted and why it was coined. A name should last while the unit stays good.",
  f"""SELECT u."from", u."last", {team('u.team__Nfl', 'u.team__College')} AS team, u.unit, u.name, u."last" - u."from" + 1 AS seasons, u.why FROM stories__units u ORDER BY u."from" DESC""")
col("stories__units", "team__College"); col("stories__units", "team__Nfl")
panel("N41.plays", WP, "N41", "Named plays", "table",
  "Plays the world named: the name, date and level, the teams, the players, what happened and the call on air. Real: named plays come from big games and late winners.",
  f"""SELECT {YR('p.date')} AS year, p.name, {DT('p.date')} AS date, p.level,
            (SELECT group_concat(COALESCE({cname('t.Nfl')}, (SELECT {prog('pr')} FROM programs pr WHERE pr._row = t.College)), ' v ') FROM stories__plays__teams t WHERE t._parent = p._row) AS teams,
            {ids_names('p.players')} AS players, p.what, p.call FROM stories__plays p ORDER BY p.date DESC""")
col("stories__plays__teams", "_parent"); col("stories__plays__teams", "Nfl"); col("stories__plays__teams", "College")
check("N41.nfl_nicknames", WP, "N41", "New NFL nicknames a year", "nicknames",
  f"SELECT 1.0 * COUNT(*) / (MAX(y) - MIN(y) + 1) FROM (SELECT {YR('coined')} AS y, team__Nfl FROM stories__nicknames) WHERE team__Nfl IS NOT NULL",
  Memory(4.0, "real: a few new nicknames stick each year league-wide, mostly for stars after big moments (stories_facts.md; from memory)"), (0.5, 1.0, 0.0),
  "Nicknames coined for NFL players and coaches per year.")

# ---------------------------------------------------------------- N42 traditions
panel("N42.honors", WP, "N42", "Retired numbers, rings of honor and statues", "table",
  "Each team honor: the team, the kind, the player, his position and number, the years he played, the year it was given and whether the ceremony has been held.",
  f"""SELECT h.year, {team('h.team__Nfl', 'h.team__College')} AS team, h.kind, h.name, h.player AS id, h.pos, h.number, h.years, CASE WHEN h.ceremony_done THEN 'held' ELSE 'to come' END AS ceremony
       FROM stories__traditions__honors h ORDER BY h.year DESC""", open="e/people/{id}")
col("stories__traditions__honors", "team__Nfl"); col("stories__traditions__honors", "team__College")
# A small world may have no honors or stormings yet: the book makes these columns (PR #174).
for c_ in ["kind", "player", "name", "pos", "number", "year", "years", "ceremony_done"]:
    col("stories__traditions__honors", c_)
for c_ in ["date", "team", "opp", "score", "fine"]:
    col("stories__traditions__stormings", c_)
panel("N42.policies", WP, "N42", "NFL clubs' honors against their real policy", "table",
  "Each NFL club's honors in the world (retired numbers, ring of honor, statues) beside its real policy and real count of retired numbers (retired_numbers_policy.csv). Real: some clubs never retire numbers (Dallas, Oakland and Las Vegas), the Giants and Bears many.",
  f"""SELECT n.name AS club, n._row AS id, SUM(h.kind = 'RetiredJersey') AS "retired (world)", SUM(h.kind LIKE 'Ring%') AS "ring of honor (world)", SUM(h.kind = 'Statue') AS "statues (world)",
            r.policy AS "real policy", r.count_retired_numbers_approx AS "real retired numbers", r.ring_of_honor_since AS "real ring since"
       FROM nfl n LEFT JOIN stories__traditions__honors h ON h.team__Nfl = n._row LEFT JOIN real_retired_numbers_policy r ON r.team = n.name GROUP BY n._row ORDER BY n.name""", open="e/nfl/{id}")
SEC_TIER = """(SELECT CASE WHEN r.first = '' THEN NULL ELSE r.first END FROM real_storming_fines r WHERE r.conference = 'SEC' AND {y} >= r.from_year AND (r.to_year = '' OR {y} <= r.to_year))"""
panel("N42.stormings", WP, "N42", "Field stormings and fines", "table",
  "Each field storming: the date, the home program and conference, the visitor, the score, and the fine, beside the real first-offence fine of the home conference that year (storming_fines.csv). Real SEC: $5,000 then $50,000 from 2015, $100,000 from 2017 and $500,000 flat from 2025; the Big Ten and ACC fine little or nothing.",
  f"""SELECT {YR('s.date')} AS year, {DT('s.date')} AS date, (SELECT {prog('p')} FROM programs p WHERE p._row = s.team) AS home, s.team AS id, (SELECT conference FROM programs WHERE _row = s.team) AS conference,
            (SELECT {prog('p')} FROM programs p WHERE p._row = s.opp) AS visitor, json_extract(s.score, '$[0]') || '-' || json_extract(s.score, '$[1]') AS score, s.fine,
            (SELECT CASE WHEN r.first = '' THEN NULL ELSE r.first END FROM real_storming_fines r WHERE r.conference = (SELECT conference FROM programs WHERE _row = s.team)
              AND {YR('s.date')} >= r.from_year AND (r.to_year = '' OR {YR('s.date')} <= r.to_year)) AS "real first fine"
       FROM stories__traditions__stormings s ORDER BY s.date DESC""", open="e/programs/{id}")
panel("N42.parades", WP, "N42", "Title parades", "table",
  "Each title parade with its crowd, beside the real parades of the same club (parades.csv). Real NFL parades draw hundreds of thousands, about 850,000 on average since 2002 (a million in Boston, Kansas City and Philadelphia).",
  f"""SELECT p._0 AS year, {team('p._1__Nfl', 'p._1__College')} AS team, p._2 AS crowd,
            (SELECT ROUND(AVG(r.crowd_estimate)) FROM real_parades r WHERE r.team = {cname('p._1__Nfl')} AND r.crowd_estimate <> '') AS "real crowd (club average)"
       FROM stories__traditions__parades p ORDER BY p._0 DESC""",
  says="""SELECT printf('NFL parades draw %.0f on average (real about 853,000 since 2002); college parades %.0f (real 30,000-80,000).', AVG(CASE WHEN _1__Nfl IS NOT NULL THEN _2 END), AVG(CASE WHEN _1__College IS NOT NULL THEN _2 END))
     FROM stories__traditions__parades""")
col("stories__traditions__parades", "_1__Nfl"); col("stories__traditions__parades", "_1__College")
panel("N42.uniforms", WP, "N42", "Uniform reveals", "table",
  "New uniform sets each year by program, as revealed.",
  f"""SELECT u._1 AS year, (SELECT {prog('p')} FROM programs p WHERE p._row = u._0__College) AS program, u._0__College AS id, u._2 AS "the set" FROM stories__traditions__uniforms__reveals u ORDER BY u._1 DESC""",
  open="e/programs/{id}")
col("stories__traditions__uniforms__reveals", "_0__College")
panel("N42.show", WP, "N42", "The pregame show's stops", "table",
  "Each Saturday's stop of the national pregame show this season: the host and the visitor; then every host's visits across the world's history.",
  f"""SELECT 'stop' AS row_kind, {DT('s._0')} AS date, (SELECT {prog('p')} FROM programs p WHERE p._row = s._1) AS host, (SELECT {prog('p')} FROM programs p WHERE p._row = s._2) AS visitor, NULL AS visits, NULL AS last
       FROM stories__traditions__show__stops s
     UNION ALL SELECT 'visits', NULL, (SELECT {prog('p')} FROM programs p WHERE p._row = v._0), NULL, v._1, v._2 FROM stories__traditions__show__visits v ORDER BY 1 DESC, 2, 5 DESC""")
col("stories__traditions__show__stops", "_0"); col("stories__traditions__show__visits", "_0")
check("N42.nfl_parade_crowd", WP, "N42", "NFL title parade crowd", "people",
  "SELECT AVG(_2) FROM stories__traditions__parades WHERE _1__Nfl IS NOT NULL",
  Fixed(853000.0, "parades.csv: 18 NFL parades 2002-2025 with a crowd estimate average about 853,000 (several rows from memory, to verify)", "2002-2025"), (0.3, 0.6, 0.0),
  "Average crowd at the world's NFL title parades.")
rule("N42.sec_fines", "N42", "SEC storming fines follow the real schedule",
  f"SELECT COUNT(*) FROM stories__traditions__stormings s JOIN programs p ON p._row = s.team LEFT JOIN programs__history h ON h._parent = s.team AND {YR('s.date')} BETWEEN h._0 AND h._1 WHERE COALESCE(h._3, p.conference) = 'SEC'",
  f"""SELECT {YR('s.date')} AS year, {prog('p')} AS home, s.fine,
            (SELECT r.first || ' / ' || r.second || ' / ' || r.third_or_later FROM real_storming_fines r WHERE r.conference = 'SEC' AND {YR('s.date')} >= r.from_year AND (r.to_year = '' OR {YR('s.date')} <= r.to_year)) AS "real tiers"
       FROM stories__traditions__stormings s JOIN programs p ON p._row = s.team
       LEFT JOIN programs__history h ON h._parent = s.team AND {YR('s.date')} BETWEEN h._0 AND h._1
      WHERE COALESCE(h._3, p.conference) = 'SEC' AND s.fine NOT IN (SELECT x FROM (SELECT r.first AS x FROM real_storming_fines r WHERE r.conference = 'SEC' AND {YR('s.date')} >= r.from_year AND (r.to_year = '' OR {YR('s.date')} <= r.to_year)
            UNION SELECT r.second FROM real_storming_fines r WHERE r.conference = 'SEC' AND {YR('s.date')} >= r.from_year AND (r.to_year = '' OR {YR('s.date')} <= r.to_year)
            UNION SELECT r.third_or_later FROM real_storming_fines r WHERE r.conference = 'SEC' AND {YR('s.date')} >= r.from_year AND (r.to_year = '' OR {YR('s.date')} <= r.to_year)))""",
  "Every SEC fine is one of that year's real tiers (storming_fines.csv): first, second, third or later offence. A program counts in the SEC only in its SEC years (minor bug 174: Oklahoma's 2021 storming, a Big 12 year with no fine, read as an SEC fine of 0).")

# ---------------------------------------------------------------- N43 media, fame, fans
panel("N43.magazines", WP, "N43", "Preseason magazines", "table",
  "Each preseason magazine: the year, the title, its cover players and its top five teams (college) or clubs (NFL). Real titles and years: magazines.csv.",
  f"""SELECT m.year, m.title, CASE WHEN m.nfl THEN 'NFL' ELSE 'College' END AS level, {ids_names('m.covers')} AS covers,
            (SELECT group_concat(t, ', ') FROM (SELECT CASE WHEN m.nfl THEN {cname('j.value')} ELSE (SELECT {prog('p')} FROM programs p WHERE p._row = j.value) END AS t
               FROM json_each(m.top) j WHERE j.key < 5 ORDER BY j.key)) AS "top five"
       FROM stories__media__magazines m ORDER BY m.year DESC, m.nfl, m.title""")
panel("N43.docs", WP, "N43", "Documentaries and Hard Knocks against real", "table",
  "Each documentary season in the world: the show, the team, how it was chosen (volunteered or forced by the league's rule: no first-year coach, no playoff team of the last two years, none of the last ten years' clubs), episodes and featured players; beside the real Hard Knocks club and how it was chosen that year (hard_knocks.csv).",
  f"""SELECT d.year, d.show, COALESCE({cname('d.team__Nfl')}, (SELECT {prog('p')} FROM programs p WHERE p._row = d.team__College)) AS team, d.how, d.episodes, {ids_names('d.stars')} AS stars,
            (SELECT r.team || ' (' || r.how || ')' FROM real_hard_knocks r WHERE r.year = d.year AND r.edition = 'training camp') AS "real Hard Knocks"
       FROM stories__media__docs d ORDER BY d.year DESC""")
col("stories__media__docs", "team__College"); col("stories__media__docs", "team__Nfl")
FAME = """fm AS (SELECT p.player__id AS id, p.name, p.player__position AS pos, p.stage, p.followers, p.brand, p.stars,
              CASE WHEN p.stage = 'Pro' THEN (SELECT name FROM nfl WHERE _row = p.pro__team) WHEN p.college IS NOT NULL THEN (SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = p.college) END AS team
         FROM people p WHERE p.followers > 0)"""
col("people", "pro__team"); col("people", "followers"); col("people", "brand")
panel("N43.fame", WP, "N43", "The most followed players and their brand money", "table",
  "The 500 most-followed players: position, stage, team, followers, brand money a year (ads, NIL and endorsements) and stars. Real: the top NFL earners make $10-50M a year off the field; followers rise from the late 2000s, quarterbacks and big markets most.",
  f"""WITH {FAME} SELECT name, id, pos, stage, team, followers, ROUND(brand / 1e6, 2) AS "brand $M a year", stars FROM fm ORDER BY followers DESC LIMIT 500""", open="e/people/{id}",
  says=f"""WITH {FAME} SELECT printf('The most followed: %s (%s, %d followers). Quarterbacks hold %d of the top 25; the top brand earner makes $%.1fM a year.',
     (SELECT name FROM fm ORDER BY followers DESC LIMIT 1), (SELECT pos FROM fm ORDER BY followers DESC LIMIT 1), MAX(followers),
     (SELECT COUNT(*) FROM (SELECT pos FROM fm ORDER BY followers DESC LIMIT 25) WHERE pos = 'QB'), MAX(brand) / 1e6) FROM fm""")
panel("N43.by_position", WP, "N43", "Followers by position and stage", "bars",
  "Average followers by position, for pros and for college players. Real: quarterbacks first by far, then receivers and running backs; linemen least.",
  f"""WITH {FAME} SELECT CASE WHEN stage = 'Pro' THEN 'Pros' ELSE 'College and younger' END AS grp, pos AS label, ROUND(AVG(followers)) AS world, NULL AS real, COUNT(*) AS n
       FROM fm GROUP BY grp, pos ORDER BY grp, world DESC""")
panel("N43.fans", WP, "N43", "NFL fans' mood by club", "bars",
  "Each club's fan mood today (front office fans, 0 to 1) with its last season's wins on hover. Mood should track wins against what was expected.",
  f"""SELECT 'Fans' AS grp, n.name AS label, n._row AS id, ROUND(c.office__fans, 2) AS world, NULL AS real,
            (SELECT wins FROM nfl_seasons s WHERE s.team = n._row ORDER BY s.year DESC LIMIT 1) AS "last season wins", 1 AS n
       FROM clubs c JOIN nfl n ON n._row = c._row ORDER BY world DESC""", open="e/nfl/{id}",
  says="""WITH k AS (SELECT c.office__fans AS x, (SELECT wins FROM nfl_seasons s WHERE s.team = c._row ORDER BY s.year DESC LIMIT 1) AS y FROM clubs c)
     SELECT printf('Fan mood and last season''s wins correlate at %.2f across the 32 clubs.', (AVG(x * y) - AVG(x) * AVG(y)) / sqrt((AVG(x * x) - AVG(x) * AVG(x)) * (AVG(y * y) - AVG(y) * AVG(y)))) FROM k""")
col("clubs", "office__fans")
check("N43.top_brand", WP, "N43", "The top brand earner's money a year", "$M", "SELECT MAX(brand) / 1e6 FROM people",
  Memory(30.0, "Forbes 2024: the top NFL players earn about $10-50M a year off the field (from memory)"), (0.4, 0.8, 0.0), "The largest yearly brand money (ads, NIL, endorsements) of any player.")
check("N43.qb_top_followed", WP, "N43", "Quarterbacks among the 25 most-followed pros", "players",
  "SELECT COUNT(*) FROM (SELECT player__position AS pos FROM people WHERE stage = 'Pro' ORDER BY followers DESC LIMIT 25) WHERE pos = 'QB'",
  Memory(8.0, "real 2024 Instagram followings: about a third of the most-followed active NFL players are quarterbacks (from memory)"), (0.4, 0.8, 0.0),
  "Quarterbacks among the 25 pros with the most followers.")
rule("N43.hard_knocks_once", "N43", "At most one Hard Knocks training camp a year",
  "SELECT COUNT(DISTINCT year) FROM stories__media__docs WHERE show = 'Hard Knocks'",
  "SELECT year, COUNT(*) AS seasons FROM stories__media__docs WHERE show = 'Hard Knocks' GROUP BY year HAVING COUNT(*) > 1",
  "HBO films one club's training camp a year (hard_knocks.csv; the in-season and offseason editions are separate shows).")

# ---------------------------------------------------------------- N44 storylines
ST = """st AS (SELECT s._row AS sid, s.id, s.kind, s.subject, s.level, s.team__Nfl, s.team__College, s.stakes, s.truth, s.belief, s.sure, s.heat, s.peak_heat, s.phase, s.public, s.opened, s.closed, s.ending, s.season, s.fame, s.market, 'ended' AS state
              FROM storylines__ended s
         UNION ALL SELECT s._row, s.id, s.kind, s.subject, s.level, s.team__Nfl, s.team__College, s.stakes, s.truth, s.belief, s.sure, s.heat, s.peak_heat, s.phase, s.public, s.opened, NULL, s.ending, s.season, s.fame, s.market, 'open'
              FROM storylines__open s),
       cast_all AS (SELECT 'ended' AS st, c._parent AS sid, c._i, c.part, c.who__Player, c.who__Coach, c.who__Team__Nfl, c.who__Team__College FROM storylines__ended__cast c
         UNION ALL SELECT 'open', c._parent, c._i, c.part, c.who__Player, c.who__Coach, c.who__Team__Nfl, c.who__Team__College FROM storylines__open__cast c)"""
for t_ in ["storylines__ended", "storylines__open"]:
    for c_ in ["team__Nfl", "team__College", "ending", "closed", "subject"]:
        col(t_, c_)
KIND = "(SELECT name FROM engine_story_kinds WHERE _row = {k})"
FAM = "(SELECT family FROM engine_story_kinds WHERE _row = {k})"
panel("N44.counts", WP, "N44", "Stories opened and gone public by season and kind", "table",
  "Each season's stories by kind (storylines.counts, kept for every season): the family, how many opened and how many went public. 117 kinds in 14 families (storylines/catalogue.rs, each with a real example).",
  f"""SELECT c._0 AS year, {FAM.format(k='c._1')} AS family, {KIND.format(k='c._1')} AS kind, c._2 AS opened, c._3 AS "gone public", ROUND(100.0 * c._3 / NULLIF(c._2, 0)) AS "public %"
       FROM storylines__counts c ORDER BY c._0 DESC, c._2 DESC""")
panel("N44.families", WP, "N44", "Stories by family and how they end", "bars",
  "Stories kept in full (open, and ended in the last two seasons) by family: how many fizzled and how many reached an aftermath. Real: most rumours fizzle; battles and hot seats nearly always end.",
  f"""WITH {ST} SELECT {FAM.format(k='kind')} AS grp, phase AS label, COUNT(*) AS world, NULL AS real, COUNT(*) AS n FROM st GROUP BY grp, phase ORDER BY grp, world DESC""")
panel("N44.stories", WP, "N44", "Every story kept in full", "table",
  "Open stories and those ended in the last two seasons: kind and family, the cast (up to three, with their parts), level and team, the stakes, the truth against what the public believes and how sure it is, heat today and at its peak, the phase, when it opened and closed, the ending, and its beats.",
  f"""WITH {ST} SELECT st.season AS year, {KIND.format(k='st.kind')} AS kind, {FAM.format(k='st.kind')} AS family, (SELECT group_concat(nm, ', ') FROM (SELECT COALESCE({pname('c.who__Player')}, (SELECT name FROM coaches WHERE id = c.who__Coach), {cname('c.who__Team__Nfl')},
               (SELECT {prog('p')} FROM programs p WHERE p._row = c.who__Team__College)) || ' (' || lower(c.part) || ')' AS nm FROM cast_all c WHERE c.sid = st.sid AND c.st = st.state ORDER BY c._i LIMIT 3)) AS cast,
            (SELECT c.who__Player FROM cast_all c WHERE c.sid = st.sid AND c.st = st.state AND c.who__Player IS NOT NULL ORDER BY c._i LIMIT 1) AS id, st.level,
            {team('st.team__Nfl', 'st.team__College')} AS team, st.stakes, ROUND(st.truth, 2) AS truth, ROUND(st.belief, 2) AS belief, ROUND(st.sure, 2) AS sure, ROUND(st.heat) AS heat,
            ROUND(st.peak_heat) AS "peak heat", st.phase, st.state, {DT('st.opened')} AS opened, {DT('st.closed')} AS closed,
            (SELECT name FROM engine_story_endings e WHERE e.kind = st.kind AND e.i = st.ending) AS ending, ROUND(st.market, 2) AS market
       FROM st ORDER BY st.opened DESC LIMIT 5000""", open="e/people/{id}")
panel("N44.heat", WP, "N44", "Peak heat by the month a story opened", "bars",
  "Average peak heat of stories by the month they opened, and in big and small markets on hover. Real: heat peaks in big markets and in the slow summer news months.",
  f"""WITH {ST} SELECT st.level AS grp, strftime('%m', {DT('st.opened')}) AS label, ROUND(AVG(st.peak_heat), 1) AS world, NULL AS real, COUNT(*) AS n,
            ROUND(AVG(CASE WHEN st.market >= 0.7 THEN st.peak_heat END), 1) AS "big markets", ROUND(AVG(CASE WHEN st.market < 0.4 THEN st.peak_heat END), 1) AS "small markets"
       FROM st GROUP BY st.level, label ORDER BY st.level, label""")
panel("N44.beats", WP, "N44", "The latest beats", "table",
  "The last 2,000 beats across stories: the date, the story's kind, what happened, who spoke and the heat it added.",
  f"""WITH b AS (SELECT b.date, b.what, b.by, b.heat, b.text, s.kind,
                    (SELECT c.who__Player FROM storylines__ended__cast c WHERE c._parent = s._row AND c.who__Player IS NOT NULL ORDER BY c._i LIMIT 1) AS id
               FROM storylines__ended__beats b JOIN storylines__ended s ON s._row = b._parent
             UNION ALL SELECT b.date, b.what, b.by, b.heat, b.text, s.kind,
                    (SELECT c.who__Player FROM storylines__open__cast c WHERE c._parent = s._row AND c.who__Player IS NOT NULL ORDER BY c._i LIMIT 1)
               FROM storylines__open__beats b JOIN storylines__open s ON s._row = b._parent)
     SELECT {YR('date')} AS year, {DT('date')} AS date, {KIND.format(k='kind')} AS story, what, by, heat, text, id FROM b ORDER BY date DESC LIMIT 2000""", open="e/people/{id}")
check("N44.fizzled", WP, "N44", "Ended stories that fizzled", "%", "SELECT 100.0 * SUM(phase = 'Fizzled') / COUNT(*) FROM storylines__ended",
  NoReal("no real count of storylines that fizzle"), (0.3, 0.7, 0.0), "Share of ended storylines whose last phase was Fizzled.")
check("N44.nfl_qb_battles", WP, "N44", "NFL quarterback battles a season", "stories",
  f"""WITH {ST} SELECT 1.0 * COUNT(*) / COUNT(DISTINCT season) FROM st WHERE level = 'Nfl' AND kind = (SELECT _row FROM engine_story_kinds WHERE key = 'qb_battle')""",
  Memory(8.0, "real: about 6-10 NFL clubs open camp with a named quarterback competition each year (from memory)"), (0.4, 0.8, 0.0), "NFL quarterback battles opened per season.")
check("N44.nfl_hot_seats", WP, "N44", "NFL head coaches fired a season", "coaches",
  "SELECT 1.0 * COUNT(*) / COUNT(DISTINCT year) FROM (SELECT s.year, s.team, s.coach FROM nfl_seasons s JOIN nfl_seasons n ON n.team = s.team AND n.year = s.year + 1 WHERE n.coach <> s.coach)",
  Memory(6.8, "real: about 6-8 NFL head coaches replaced a year 2000-2025 (from memory)"), (0.25, 0.5, 0.0), "Clubs with a new head coach the next season, per year.")

# ---------------------------------------------------------------- N48 export meta and options
panel("N48.options", WP, "N48", "The world's options", "table",
  "The options the world was made with (World.options): seed, scale, history years, start year, names, the fictional league, high school engine years, the game log, yearly growth. The first recorded season is the start year less the history years; earlier rows are the world's spin-up and the checks leave them out. The book's own facts (world file, commits, dates) and every store's rows are on the overview and the coverage map.",
  """SELECT 'seed' AS option, CAST(seed AS TEXT) AS value FROM options UNION ALL SELECT 'scale', CAST(scale AS TEXT) FROM options UNION ALL SELECT 'history years', CAST(history_years AS TEXT) FROM options
     UNION ALL SELECT 'start year', CAST(start_year AS TEXT) FROM options UNION ALL SELECT 'names', names FROM options UNION ALL SELECT 'fictional league', CASE WHEN fictional_league THEN 'yes' ELSE 'no' END FROM options
     UNION ALL SELECT 'high school engine years', CAST(hs_engine_years AS TEXT) FROM options UNION ALL SELECT 'game log kept', CASE WHEN keep_game_log THEN 'yes' ELSE 'no' END FROM options
     UNION ALL SELECT 'yearly growth', CAST(yearly_growth AS TEXT) FROM options UNION ALL SELECT 'faces version', CAST(faces_version AS TEXT) FROM world""")
# ---------------------------------------------------------------- T9 pop culture
FPOS = "(CASE {p} WHEN 0 THEN 'QB' WHEN 1 THEN 'RB' WHEN 2 THEN 'WR' WHEN 3 THEN 'TE' END)"
panel("T9.fantasy", WP, "T9", "Fantasy leaders by position against real", "table",
  "Each season's top scorer at QB, RB, WR and TE (standard scoring before 2017, half-point PPR 2017-2018, full PPR from 2019; stories/media.rs fantasy_points) against the real top scorer of that season (nfl_careers.json fantasy_ppr, full PPR, so compare from 2019).",
  f"""SELECT f._0 AS year, {FPOS.format(p='f._1')} AS pos, {pname('f._2')} AS player, f._2 AS id, ROUND(f._3, 1) AS points,
            json_extract({NC}, '$.fantasy_ppr.by_season."' || f._0 || '".' || {FPOS.format(p='f._1')} || '.top1') AS "real top",
            json_extract({NC}, '$.fantasy_ppr.by_season."' || f._0 || '".' || {FPOS.format(p='f._1')} || '.top12') AS "real 12th"
       FROM stories__media__fantasy_leaders f ORDER BY f._0 DESC, f._1""", open="e/people/{id}")
panel("T9.fantasy_now", WP, "T9", "This season's fantasy lines", "table",
  "Every player's fantasy line this season: points, games and last week's points, by position, with the rank at his position.",
  f"""SELECT {FPOS.format(p='f.pos')} AS pos, RANK() OVER (PARTITION BY f.pos ORDER BY f.pts DESC) AS rank, {pname('f._key')} AS player, f._key AS id, ROUND(f.pts, 1) AS points, f.games,
            ROUND(f.pts / NULLIF(f.games, 0), 1) AS "a game", ROUND(f.last, 1) AS "last week"
       FROM stories__media__fantasy f WHERE f.pos <= 3 ORDER BY f.pos, f.pts DESC""", open="e/people/{id}")
panel("T9.jerseys", WP, "T9", "Jersey sales top lists", "table",
  "Each year's best-selling jerseys, NFL and college: rank, player, position and team. Real: mostly quarterbacks and big-market stars, with rookies breaking in.",
  f"""SELECT j._0 AS year, CASE WHEN j._1 THEN 'College' ELSE 'NFL' END AS level, CAST(x.key AS INTEGER) + 1 AS rank, {pname('x.value')} AS player, x.value AS id, {pos_of('x.value')} AS pos
       FROM stories__media__jerseys j, json_each(j._2) x ORDER BY j._0 DESC, j._1, CAST(x.key AS INTEGER)""", open="e/people/{id}")
VG = """vg AS (SELECT r._key AS id, r._v AS rating, p.x_overall AS overall, p.player__position AS pos, CASE WHEN p.stage IN ('Pro', 'Done') THEN 'NFL game' ELSE 'College game' END AS game
              FROM stories__media__ratings r JOIN people p ON p.player__id = r._key WHERE p.x_overall IS NOT NULL),
        vr AS (SELECT vg.*, RANK() OVER (PARTITION BY game ORDER BY rating) AS ra, RANK() OVER (PARTITION BY game ORDER BY overall) AS rb FROM vg)"""
panel("T9.vg_bins", WP, "T9", "The video games' ratings against true overall", "table",
  "Players by their video-game rating in five-point bins, the NFL game and the college game apart (each rates players against their own level): how many, their average true overall (debug) and its range, and the positions most common in the bin. The rank correlation in the verdict says how close each game's read sits to the truth.",
  f"""WITH {VG} SELECT game AS grp, (CAST(rating / 5 AS INTEGER) * 5) || '-' || (CAST(rating / 5 AS INTEGER) * 5 + 4) AS rating, COUNT(*) AS players, ROUND(AVG(overall), 1) AS "true overall",
            ROUND(MIN(overall), 1) AS lowest, ROUND(MAX(overall), 1) AS highest,
            (SELECT group_concat(pos, ', ') FROM (SELECT pos FROM vg v2 WHERE v2.game = vg.game AND CAST(v2.rating / 5 AS INTEGER) = CAST(vg.rating / 5 AS INTEGER) GROUP BY pos ORDER BY COUNT(*) DESC LIMIT 3)) AS "most common"
       FROM vg GROUP BY game, CAST(rating / 5 AS INTEGER) ORDER BY game DESC, CAST(rating / 5 AS INTEGER) DESC""",
  says=f"""WITH {VG}, k AS (SELECT game, COUNT(*) AS n, (AVG(ra * rb) - AVG(ra) * AVG(rb)) / sqrt((AVG(ra * ra) - AVG(ra) * AVG(ra)) * (AVG(rb * rb) - AVG(rb) * AVG(rb))) AS r FROM vr GROUP BY game)
     SELECT group_concat(printf('%s: %d players, rank correlation with true overall %.2f', game, n, r), '; ') || '.' FROM k""")
panel("T9.vg_top", WP, "T9", "The video game's top-rated players each year", "table",
  "Each title's top-rated players (rating 99 club and the rest of the top list): year, title, player, position and rating. Real: quarterbacks and edge rushers lead the top lists.",
  f"""SELECT v._0 AS year, v._1 AS title, {pname('v._2')} AS player, v._2 AS id, {pos_of('v._2')} AS pos, v._3 AS rating FROM stories__media__vg_top v ORDER BY v._0 DESC, v._1, v._3 DESC""", open="e/people/{id}")
panel("T9.preseason_vs_final", WP, "T9", "Preseason magazines against the final poll", "table",
  "Each college magazine's preseason top 25 last season against the final poll: how many finished ranked, and how the covers' teams finished. Real AP preseason top 25s 2004-2025: about 15 of 25 finish in the final top 25.",
  f"""WITH fin AS (SELECT top FROM prev_polls WHERE kind = 'Final' LIMIT 1), yr AS (SELECT {YR('(SELECT MIN(date) FROM prev_polls)')} AS y)
     SELECT m.year, m.title, (SELECT COUNT(*) FROM json_each(m.top) t WHERE t.key < 25 AND t.value IN (SELECT value FROM fin, json_each(fin.top))) AS "finished ranked",
            {ids_names('m.covers')} AS covers
       FROM stories__media__magazines m WHERE m.nfl = 0 AND m.year = (SELECT y FROM yr) ORDER BY m.title""")
panel("T9.careers", WP, "T9", "Farewell tours, signature celebrations and one-day contracts", "table",
  "Star farewell seasons with their gifts, signature celebrations by when they were born, and one-day contracts to retire with a club, beside the real one-day contracts (one_day_contracts.csv, some rows from memory).",
  f"""SELECT f.year, 'farewell' AS what, {pname('f.id')} AS player, f.id, {cname('f.team')} AS team,
            (SELECT group_concat(g._1, '; ') FROM stories__careers__farewells__gifts g WHERE g._parent = f._row) AS detail FROM stories__careers__farewells f
     UNION ALL SELECT c._2, 'celebration', {pname('c._0')}, c._0, NULL, c._1 FROM stories__careers__celebrations c
     UNION ALL SELECT o._1, 'one-day contract', {pname('o._0')}, o._0, {cname('o._2')}, NULL FROM stories__careers__one_day o
     UNION ALL SELECT r.year, 'real one-day contract', r.player, NULL, r.team, r.source FROM real_one_day_contracts r ORDER BY 1 DESC, 2""", open="e/people/{id}")
col("stories__careers__farewells__gifts", "_parent")
check("T9.fantasy_qb", WP, "T9", "Top fantasy QB's points (full PPR seasons)", "points",
  "SELECT AVG(_3) FROM stories__media__fantasy_leaders WHERE _1 = 0 AND _0 >= 2019",
  Fixed(402.8, "nfl_careers.json fantasy_ppr: the top QB averaged 402.8 points 2019-2025", "2019-2025"), (0.1, 0.25, 0.0), "The season's top QB's fantasy points, 2019 on.")
check("T9.fantasy_rb", WP, "T9", "Top fantasy RB's points (full PPR seasons)", "points",
  "SELECT AVG(_3) FROM stories__media__fantasy_leaders WHERE _1 = 1 AND _0 >= 2019",
  Fixed(395.1, "nfl_careers.json fantasy_ppr: the top RB averaged 395.1 points 2019-2025", "2019-2025"), (0.1, 0.25, 0.0), "The season's top RB's fantasy points, 2019 on.")
check("T9.fantasy_wr", WP, "T9", "Top fantasy WR's points (full PPR seasons)", "points",
  "SELECT AVG(_3) FROM stories__media__fantasy_leaders WHERE _1 = 2 AND _0 >= 2019",
  Fixed(388.9, "nfl_careers.json fantasy_ppr: the top WR averaged 388.9 points 2019-2025", "2019-2025"), (0.1, 0.25, 0.0), "The season's top WR's fantasy points, 2019 on.")
check("T9.fantasy_te", WP, "T9", "Top fantasy TE's points (full PPR seasons)", "points",
  "SELECT AVG(_3) FROM stories__media__fantasy_leaders WHERE _1 = 3 AND _0 >= 2019",
  Fixed(286.1, "nfl_careers.json fantasy_ppr: the top TE averaged 286.1 points 2019-2025", "2019-2025"), (0.1, 0.25, 0.0), "The season's top TE's fantasy points, 2019 on.")
check("T9.jersey_qbs", WP, "T9", "Quarterbacks in the NFL jersey top 10", "players",
  f"SELECT 1.0 * SUM({pos_of('x.value')} = 'QB') / COUNT(DISTINCT j._0) FROM stories__media__jerseys j, json_each(j._2) x WHERE j._1 = 0 AND CAST(x.key AS INTEGER) < 10",
  Memory(4.0, "NFLPA top-10 jersey sales 2019-2024: about four quarterbacks a year (from memory)"), (0.3, 0.6, 0.0), "Quarterbacks among each year's ten best-selling NFL jerseys.")
check("T9.preseason_hits", WP, "T9", "Preseason top 25 teams that finish ranked", "teams",
  f"""WITH fin AS (SELECT top FROM prev_polls WHERE kind = 'Final' LIMIT 1)
     SELECT AVG(n) FROM (SELECT (SELECT COUNT(*) FROM json_each(m.top) t WHERE t.key < 25 AND t.value IN (SELECT value FROM fin, json_each(fin.top))) AS n
       FROM stories__media__magazines m WHERE m.nfl = 0 AND m.year = {YR('(SELECT MIN(date) FROM prev_polls)')})""",
  Fixed(14.9, "real_college_ap_polls: the AP preseason top 25 put 14.9 teams in the final top 25 on average, 2004-2025", "2004-2025"), (0.15, 0.3, 0.0),
  "Last season's college magazines: their top-25 teams that finished in the final poll.")
check("T9.one_day", WP, "T9", "One-day contracts a year", "contracts",
  "SELECT CASE WHEN (SELECT COUNT(*) FROM stories__careers__farewells) > 0 THEN 1.0 * COUNT(*) / NULLIF(MAX(_1) - MIN(_1) + 1, 0) END FROM stories__careers__one_day",
  Fixed(1.1, "one_day_contracts.csv: 24 one-day contracts 2005-2026 (some rows from memory, to verify)", "2005-2026"), (0.4, 0.8, 0.0), "One-day contracts signed per year.")
check("T9.vg_corr", WP, "T9", "NFL video-game rating against true overall: rank correlation", "r",
  f"WITH {VG} SELECT (AVG(ra * rb) - AVG(ra) * AVG(rb)) / sqrt((AVG(ra * ra) - AVG(ra) * AVG(ra)) * (AVG(rb * rb) - AVG(rb) * AVG(rb))) FROM vr WHERE game = 'NFL game'",
  NoReal("no real video game publishes its ratings beside a true overall"), (0.3, 0.7, 0.0), "Rank correlation of NFL players' video-game ratings with their true overall.")
