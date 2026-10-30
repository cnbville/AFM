from common import *

WP = "world"
def DT(d): return f"date({d} * 86400, 'unixepoch')"
def YR(d): return f"CAST(strftime('%Y', {DT(d)}) AS INTEGER)"
def WD(d): return f"(CASE strftime('%w', {DT(d)}) WHEN '0' THEN 'Sun' WHEN '1' THEN 'Mon' WHEN '2' THEN 'Tue' WHEN '3' THEN 'Wed' WHEN '4' THEN 'Thu' WHEN '5' THEN 'Fri' ELSE 'Sat' END)"
def MD(d): return f"strftime('%m-%d', {DT(d)})"
FIRST = "(SELECT start_year - history_years FROM options)"

# ---------------------------------------------------------------- N34 dates
D1 = "(SELECT date FROM engine_dates__draft x WHERE x._parent = d._row AND x._i = 0)"
panel("N34.years", WP, "N34", "Each football year's key dates", "table",
  "The dates the calendar sets each season (calendar/anchors.rs YearDates, real anchors per year) from 1984: the combine, the new league year, the draft's first day, kickoff, the trade deadline, Thanksgiving, Black Monday and the Super Bowl; college's Labor Day weekend, first CFP ranking and title game. The weekday sits beside each date so era shifts read at a glance.",
  f"""SELECT d.season AS year, {DT('d.combine_start')} AS combine, {DT('d.league_year__date')} AS "league year", {DT(D1)} || ' ' || {WD(D1)} AS "draft day 1",
            {DT('d.nfl_kickoff')} || ' ' || {WD('d.nfl_kickoff')} AS kickoff, {DT('d.trade_deadline__date')} || ' ' || {WD('d.trade_deadline__date')} AS "trade deadline",
            CAST((d.trade_deadline__date - d.nfl_kickoff) / 7 AS INTEGER) + 1 AS "deadline week", {DT('d.thanksgiving')} AS thanksgiving, {DT('d.black_monday')} AS "Black Monday",
            {DT('d.super_bowl')} || ' ' || {WD('d.super_bowl')} AS "Super Bowl", {DT('d.labor_day')} AS "Labor Day", {DT('d.first_cfp_ranking')} AS "first CFP ranking", {DT('d.title_game')} AS "title game"
       FROM engine_dates d ORDER BY d.season DESC""",
  says=f"""SELECT printf('%d years of dates: the draft opens on a %s in %d and a %s in %d; the Super Bowl falls between %s and %s.', COUNT(*),
     (SELECT {WD(D1)} FROM engine_dates d WHERE season = 2005), 2005, (SELECT {WD(D1)} FROM engine_dates d WHERE season = 2015), 2015,
     MIN({MD('super_bowl')}), MAX({MD('super_bowl')})) FROM engine_dates""")
for c_ in ["labor_day", "first_cfp_ranking", "title_game", "black_monday", "thanksgiving", "combine_start", "trade_deadline__date", "league_year__date", "nfl_kickoff", "super_bowl"]:
    col("engine_dates", c_)
panel("N34.today", WP, "N34", "This year's calendar, one date to the next", "table",
  "Every date this football year's calendar holds (World.dates), in order: the NFL's from the Super Bowl to the next, college's and high school's, with its weekday.",
  f"""WITH d AS (SELECT * FROM dates LIMIT 1), k AS (SELECT j.key AS what, j.value AS v FROM d, json_each((SELECT json_object(""" +
  ", ".join(f"'{k}', d.{k}" for k in ["super_bowl", "tag_window_open", "combine_start", "league_year__date", "annual_meeting", "comp_picks", "pro_days", "top30_deadline",
      "udfa_rush__date", "fifth_year_option", "rookie_minicamp", "otas", "june_1", "minicamp", "rookies_report", "veterans_report", "hof_game", "hof_enshrinement", "preseason",
      "cutdown__date", "waiver_claims__date", "practice_squads", "nfl_kickoff", "thanksgiving", "christmas", "trade_deadline__date", "black_monday", "wild_card", "divisional",
      "conference_title_games", "nfl_honors", "labor_day", "media_days", "fall_camp", "rivalry_week", "conference_championships", "army_navy", "selection_day", "first_cfp_ranking",
      "heisman", "bowl_season", "cfp_first_round", "title_game", "spring_game", "portal_open"]) + """) FROM d)) j)
     SELECT (SELECT season FROM dates) AS season, replace(replace(what, '__date', ''), '_', ' ') AS date_of, """ + DT('v') + " AS date, " + WD('v') + """ AS weekday FROM k WHERE v IS NOT NULL ORDER BY v""")
for c_ in ["tag_window_open", "annual_meeting", "comp_picks", "pro_days", "top30_deadline", "udfa_rush__date", "fifth_year_option", "rookie_minicamp", "otas", "june_1", "minicamp",
           "rookies_report", "veterans_report", "hof_game", "hof_enshrinement", "preseason", "cutdown__date", "waiver_claims__date", "practice_squads", "christmas", "wild_card",
           "divisional", "conference_title_games", "nfl_honors", "media_days", "fall_camp", "rivalry_week", "conference_championships", "army_navy", "selection_day", "heisman",
           "bowl_season", "cfp_first_round", "spring_game", "portal_open", "super_bowl", "combine_start", "league_year__date", "nfl_kickoff", "thanksgiving", "trade_deadline__date",
           "black_monday", "labor_day", "first_cfp_ranking", "title_game"]:
    col("dates", c_)
rule("N34.draft_weekday", "N34", "The draft opens on a Saturday 1995-2009 and a Thursday from 2010",
  "SELECT COUNT(*) FROM engine_dates WHERE season >= 1995",
  f"SELECT d.season, {DT(D1)} AS date, {WD(D1)} AS weekday FROM engine_dates d WHERE d.season >= 1995 AND {WD(D1)} <> CASE WHEN d.season < 2010 THEN 'Sat' ELSE 'Thu' END",
  "Real: a two-day weekend draft opening on Saturday from 1995 to 2009; three days opening Thursday night from 2010.")
rule("N34.super_bowl_window", "N34", "Every Super Bowl falls between 20 January and 15 February",
  "SELECT COUNT(*) FROM engine_dates",
  f"SELECT season, {DT('super_bowl')} AS date FROM engine_dates WHERE {MD('super_bowl')} NOT BETWEEN '01-20' AND '02-15'",
  "Real Super Bowls since 1984 run from 20 January (XIX, 1985) to 13 February (LVI, 2022).")
rule("N34.league_year_window", "N34", "Free agency opens between mid-February and mid-March from 1993",
  "SELECT COUNT(*) FROM engine_dates WHERE season >= 1993",
  f"SELECT season, {DT('league_year__date')} AS date FROM engine_dates WHERE season >= 1993 AND {MD('league_year__date')} NOT BETWEEN '02-10' AND '03-20'",
  "The league year (free agency and trades) opened on 1 March 1993, in mid-to-late February through the late 1990s and 2003-2009, and in March since.")
rule("N34.thanksgiving", "N34", "Thanksgiving is the fourth Thursday of November",
  "SELECT COUNT(*) FROM engine_dates",
  f"SELECT season, {DT('thanksgiving')} AS date FROM engine_dates WHERE {WD('thanksgiving')} <> 'Thu' OR {MD('thanksgiving')} NOT BETWEEN '11-22' AND '11-28'",
  "US law since 1941: the fourth Thursday of November.")
rule("N34.kickoff_thursday", "N34", "The NFL season opens on the Thursday after Labor Day from 2002",
  "SELECT COUNT(*) FROM engine_dates WHERE season >= 2002",
  f"SELECT season, {DT('nfl_kickoff')} AS kickoff, {DT('labor_day')} AS labor_day FROM engine_dates WHERE season >= 2002 AND ({WD('nfl_kickoff')} <> 'Thu' OR nfl_kickoff - labor_day NOT BETWEEN 3 AND 10)",
  "Real: the Kickoff Game has been on the Thursday after Labor Day since 2002 (2012 moved to Wednesday for the Democratic convention).")

# ---------------------------------------------------------------- N35 rule books
panel("N35.books", WP, "N35", "The rule book each season", "table",
  "Each season's rules (calendar/rules.rs RuleBook::era from 1984, the world's own after 2026): NFL games and weeks, playoff teams, roster, game-day actives, practice squad, the cap; college postseason, CFP teams, FBS games, the FCS and D2 brackets, transfers, NIL, early signing, the four-game redshirt, five in five, revenue sharing; and what changed from the year before.",
  """WITH r AS (SELECT * FROM engine_rules), c AS (SELECT r.*, LAG(nfl_games) OVER w AS g0, LAG(nfl_playoff_teams) OVER w AS p0, LAG(nfl_roster) OVER w AS r0, LAG(nfl_actives) OVER w AS a0,
            LAG(nfl_practice_squad) OVER w AS s0, LAG(cfb_postseason) OVER w AS b0, LAG(fbs_games) OVER w AS f0, LAG(transfer) OVER w AS t0, LAG(nil) OVER w AS n0, LAG(early_signing) OVER w AS e0,
            LAG(four_game_redshirt) OVER w AS h0, LAG(salary_cap) OVER w AS cap0 FROM r WINDOW w AS (ORDER BY season))
     SELECT season AS year, nfl_games AS "NFL games", nfl_weeks AS weeks, nfl_playoff_teams AS "playoff teams", nfl_roster AS roster, nfl_actives AS actives, nfl_practice_squad AS "practice squad",
            ROUND(salary_cap / 1e6, 1) AS "cap $M", cfb_postseason AS "college postseason", cfp_teams AS "CFP teams", fbs_games AS "FBS games", fcs_playoff AS "FCS bracket", d2_playoff AS "D2 bracket",
            transfer, one_time_transfer AS "one-time transfer", nil AS NIL, early_signing AS "early signing", four_game_redshirt AS "4-game redshirt", five_in_five AS "five in five",
            ROUND(revenue_sharing / 1e6, 1) AS "revenue share $M",
            trim((CASE WHEN g0 <> nfl_games THEN 'NFL games; ' ELSE '' END) || (CASE WHEN p0 <> nfl_playoff_teams THEN 'playoff field; ' ELSE '' END) || (CASE WHEN r0 <> nfl_roster OR a0 <> nfl_actives THEN 'roster; ' ELSE '' END)
              || (CASE WHEN s0 <> nfl_practice_squad THEN 'practice squad; ' ELSE '' END) || (CASE WHEN cap0 IS NULL AND salary_cap IS NOT NULL THEN 'salary cap begins; ' ELSE '' END)
              || (CASE WHEN b0 <> cfb_postseason THEN 'college postseason; ' ELSE '' END) || (CASE WHEN f0 <> fbs_games THEN 'FBS schedule; ' ELSE '' END) || (CASE WHEN t0 <> transfer THEN 'transfer rules; ' ELSE '' END)
              || (CASE WHEN n0 <> nil THEN 'NIL; ' ELSE '' END) || (CASE WHEN e0 <> early_signing THEN 'early signing; ' ELSE '' END) || (CASE WHEN h0 <> four_game_redshirt THEN 'redshirt rule; ' ELSE '' END), '; ') AS changed
       FROM c ORDER BY season DESC""")
for c_ in ["revenue_sharing", "five_in_five", "one_time_transfer", "fcs_playoff", "d2_playoff", "salary_cap"]:
    col("engine_rules", c_)
panel("N35.changes", WP, "N35", "The 12 changes after 2026: what passed and what failed", "table",
  "Each of the 12 agreed off-field changes once the world runs past 2026 (calendar/rules.rs, odds proposed, not confirmed): the season it took effect and what it did, then every failed vote with its count. Empty until the world plays past 2026.",
  """SELECT season, change, 'passed' AS outcome, detail FROM changes__applied
     UNION ALL SELECT _0, _1, 'failed', 'votes for: ' || _2 FROM changes__failed_votes ORDER BY 1, 3""")
col("changes__applied", "season"); col("changes__applied", "change"); col("changes__applied", "detail")
col("changes__failed_votes", "_0"); col("changes__failed_votes", "_1"); col("changes__failed_votes", "_2")
REALRULES = """real AS (SELECT season,
            CASE WHEN season = 1987 THEN 15 WHEN season < 2021 THEN 16 ELSE 17 END AS games,
            CASE WHEN season < 1990 THEN 10 WHEN season < 2020 THEN 12 ELSE 14 END AS playoff,
            CASE WHEN season < 1989 THEN 0 WHEN season < 2004 THEN 5 WHEN season < 2014 THEN 8 WHEN season < 2020 THEN 10 ELSE 16 END AS squad,
            CASE WHEN season < 2006 THEN 11 ELSE 12 END AS fbs,
            CASE WHEN season < 1998 THEN 0 WHEN season < 2014 THEN 2 WHEN season < 2024 THEN 4 ELSE 12 END AS cfp FROM engine_rules WHERE season <= 2026)"""
rule("N35.real_history", "N35", "Each year's rule book matches real history (1984-2026)",
  "SELECT COUNT(*) FROM engine_rules WHERE season <= 2026",
  f"""WITH {REALRULES} SELECT r.season, 'NFL games' AS rule, e.nfl_games AS world, r.games AS real FROM real r JOIN engine_rules e USING (season) WHERE e.nfl_games <> r.games
     UNION ALL SELECT r.season, 'playoff teams', e.nfl_playoff_teams, r.playoff FROM real r JOIN engine_rules e USING (season) WHERE e.nfl_playoff_teams <> r.playoff
     UNION ALL SELECT r.season, 'practice squad', e.nfl_practice_squad, r.squad FROM real r JOIN engine_rules e USING (season) WHERE e.nfl_practice_squad NOT IN (r.squad, r.squad + 1)
     UNION ALL SELECT r.season, 'FBS games', e.fbs_games, r.fbs FROM real r JOIN engine_rules e USING (season) WHERE e.fbs_games <> r.fbs
     UNION ALL SELECT r.season, 'CFP teams', e.cfp_teams, r.cfp FROM real r JOIN engine_rules e USING (season) WHERE e.cfp_teams <> r.cfp
     UNION ALL SELECT season, 'salary cap', salary_cap, CASE WHEN season < 1994 OR season = 2010 THEN 'none' ELSE 'a cap' END FROM engine_rules WHERE season <= 2026 AND (season < 1994 OR season = 2010) = (salary_cap IS NOT NULL)""",
  "Real: 16 games (15 in the 1987 strike year) until 17 from 2021; 10 playoff teams to 1989, 12 to 2019, 14 from 2020; practice squads of 5 from 1989, 8 from 2004, 10 from 2014, 16 from 2020 (a 17th for an international player); 12 FBS games from 2006; the BCS title game from 1998, a 4-team CFP from 2014 and 12 from 2024; the cap from 1994, except the uncapped 2010 season.")
check("N35.cap_2024", WP, "N35", "The salary cap in 2024", "$M", "SELECT salary_cap / 1e6 FROM engine_rules WHERE season = 2024",
  Fixed(255.4, "NFL: 2024 salary cap $255.4M", "2024"), (0.005, 0.02, 0.0), "The rule book's cap for 2024.")
check("N35.cap_1994", WP, "N35", "The first salary cap, 1994", "$M", "SELECT salary_cap / 1e6 FROM engine_rules WHERE season = 1994",
  Fixed(34.6, "NFL: the first salary cap, $34.608M in 1994", "1994"), (0.005, 0.02, 0.0), "The rule book's cap for 1994.")

# ---------------------------------------------------------------- N36 events
panel("N36.counts", WP, "N36", "Every event the calendar has posted, by level and kind", "bars",
  "Counts of all events across every recorded year (calendar.counts), by level and kind. Pick a level.",
  """SELECT json_extract(_key, '$[0]') AS grp, json_extract(_key, '$[1]') AS label, _v AS world, NULL AS real, _v AS n FROM calendar__counts ORDER BY grp, _v DESC""")
panel("N36.by_year", WP, "N36", "Events a year by loudness", "line",
  "Events each year by loudness from the game log's event record (medium and louder every year, small ones for the last five recorded years). Steady counts across eras are expected except where rules change (the portal, NIL, the 12-team CFP).",
  """SELECT season AS year, SUM(event__loud = 'Small') AS small, SUM(event__loud = 'Medium') AS medium, SUM(event__loud = 'Big') AS big, SUM(event__loud = 'Huge') AS huge
       FROM log_events GROUP BY season ORDER BY season""", needs=G)
panel("N36.kinds", WP, "N36", "Events of each kind by year", "matrix",
  "Medium and louder events per kind (rows) and year (columns) from the game log's event record: a kind that jumps or vanishes between years shows where a system changed.",
  """SELECT event__level AS grp, event__kind AS row, season AS col, COUNT(*) AS value, NULL AS real, COUNT(*) AS n FROM log_events WHERE event__loud <> 'Small' GROUP BY event__level, event__kind, season ORDER BY 1, 2, 3""", needs=G)
panel("N36.huge", WP, "N36", "The huge moments, year by year", "table",
  "Every huge moment the calendar keeps from past years (calendar.history keeps big and huge ones): the date, level, kind, the rule it follows and its text; memorable ones marked.",
  f"""SELECT {YR('when__date')} AS year, {DT('when__date')} AS date, level, kind, rule, text, CASE WHEN memorable THEN 'yes' ELSE '' END AS memorable
       FROM calendar__history WHERE loud = 'Huge' ORDER BY when__date DESC""",
  says=f"""SELECT printf('%d huge moments over %d years: %.1f a year, %d of them games.', COUNT(*), MAX(y) - MIN(y) + 1, 1.0 * COUNT(*) / (MAX(y) - MIN(y) + 1), SUM(kind = 'Game'))
     FROM (SELECT kind, {YR('when__date')} AS y FROM calendar__history WHERE loud = 'Huge')""")
panel("N36.anniversaries", WP, "N36", "Anniversaries of big moments", "table",
  "Anniversary events the calendar posted on the dates of past big moments, with their text.",
  f"""SELECT {YR('when__date')} AS year, {DT('when__date')} AS date, level, loud, text FROM calendar__history WHERE kind = 'Anniversary'
     UNION ALL SELECT season, {DT('event__when__date')}, event__level, event__loud, event__text FROM log_events WHERE event__kind = 'Anniversary' ORDER BY 2 DESC""")
check("N36.huge_a_year", WP, "N36", "Huge moments a year (not games)", "events",
  f"SELECT 1.0 * SUM(kind <> 'Game') / (MAX(y) - MIN(y) + 1) FROM (SELECT kind, {YR('when__date')} AS y FROM calendar__history WHERE loud = 'Huge')",
  NoReal("loudness is the game's own scale; no real count exists"), (0.3, 0.7, 0.0), "Huge calendar moments that are not games, over the years the calendar covers.")
rule("N36.nfl_dates_once", "N36", "Every NFL calendar date posts once a year",
  f"SELECT COUNT(DISTINCT {YR('when__date')} || rule) FROM calendar__history WHERE level = 'Nfl' AND kind IN ('Date', 'Deadline') AND rule <> ''",
  f"""SELECT {YR('when__date')} AS year, rule, COUNT(*) AS posts FROM calendar__history WHERE level = 'Nfl' AND kind IN ('Date', 'Deadline') AND rule <> ''
      GROUP BY year, rule, text HAVING COUNT(*) > 1""",
  "Each dated NFL step (the tag deadline, the new league year, the draft) happens once a year.")

# ---------------------------------------------------------------- N37 weather
WX = """wx AS (SELECT g.season AS year, g.stream, g.home, g.date, g.conditions__temperature AS temp, COALESCE(g.conditions__snow, 0) AS snow, COALESCE(g.conditions__rain, 0) AS rain,
              COALESCE(g.conditions__wind, 0) AS wind, COALESCE(g.weather__lightning, 0) AS lightning, COALESCE(g.weather__delay, 0) AS delay, COALESCE(g.weather__indoor, 0) AS indoor,
              CAST(strftime('%m', date(g.date * 86400, 'unixepoch')) AS INTEGER) AS month FROM log_games g WHERE g.stream IN ('nfl', 'college', 'hs') AND g.home_pts IS NOT NULL)"""
col("log_games", "date")
panel("N37.counts", WP, "N37", "Weather counts each year (college games)", "line",
  "The calendar log's weather counters for college games (World.log_history): snow, rain and freezing games, lightning delays, games moved or cancelled, and heat-limited practice days. Spin-up years before the first recorded season are left out.",
  f"""SELECT year, snow_games AS snow, rain_games AS rain, freezing_games AS freezing, lightning_delays AS lightning, weather_moves AS moved, weather_cancels AS cancelled,
            heat_limited_days AS "heat days" FROM log_history WHERE year >= {FIRST} ORDER BY year""")
panel("N37.levels", WP, "N37", "Weather by level and month", "table",
  "Engine games by level and month from the game log: games, the average temperature, games below freezing, with snow, with rain, at 15+ mph wind, lightning delays and indoor games. Real: a handful of NFL snow games a season; the Big Ten and MAC see the most in November; lightning delays cluster in the South in September.",
  f"""WITH {WX} SELECT year, stream AS level, month, COUNT(*) AS games, ROUND(AVG(temp), 1) AS "°F", SUM(temp < 32) AS freezing, SUM(snow > 0) AS snow, SUM(rain > 0) AS rain,
            SUM(wind >= 15) AS "wind 15+", SUM(lightning > 0 OR delay > 0) AS lightning, SUM(indoor) AS indoor FROM wx GROUP BY year, stream, month ORDER BY year DESC, stream, month""", needs=G)
panel("N37.freezing", WP, "N37", "NFL games below freezing by home club", "bars",
  "NFL home games played below 32°F, by club, all logged seasons. Real: Green Bay, Buffalo, Chicago, New England, Cleveland and Kansas City host nearly all of them, late in the season; domes host none.",
  f"""WITH {WX} SELECT 'NFL' AS grp, {cname('home')} AS label, home AS id, SUM(temp < 32) AS world, NULL AS real, COUNT(*) AS n FROM wx WHERE stream = 'nfl' GROUP BY home HAVING SUM(temp < 32) > 0 ORDER BY world DESC""",
  says=f"""WITH {WX}, t AS (SELECT {cname('home')} AS club, SUM(temp < 32) AS n FROM wx WHERE stream = 'nfl' GROUP BY home ORDER BY n DESC LIMIT 3)
     SELECT 'Most freezing home games: ' || group_concat(club || ' ' || n, ', ') || ' (real: Green Bay, Buffalo, Chicago lead).' FROM t""", needs=G, open="e/nfl/{id}")
panel("N37.snow_states", WP, "N37", "College snow and lightning by state", "map",
  "College engine games by the home program's state: snow games per 100 games (world), with lightning delays per 100 games on hover. Real: snow in the Midwest, Mountain West and Northeast in November; lightning in the South in September.",
  f"""WITH {WX} SELECT (SELECT code FROM states WHERE _row = p.state) AS state, ROUND(100.0 * SUM(wx.snow > 0) / COUNT(*), 1) AS world, NULL AS real, COUNT(*) AS n,
            ROUND(100.0 * SUM(wx.lightning > 0 OR wx.delay > 0) / COUNT(*), 1) AS "lightning per 100"
       FROM wx JOIN programs p ON p._row = wx.home WHERE wx.stream = 'college' GROUP BY p.state ORDER BY state""", needs=G)
rule("N37.domes_dry", "N37", "Domes never have weather",
  "SELECT COUNT(*) FROM log_games g WHERE g.stream = 'nfl'",
  f"""SELECT g.season, {DT('g.date')} AS date, {cname('g.home')} AS home, g.conditions__snow AS snow, g.conditions__rain AS rain, g.conditions__wind AS wind
       FROM log_games g JOIN engine_franchises f ON f.team = g.home AND g.season BETWEEN f."from" AND f."to"
      WHERE g.stream = 'nfl' AND f.roof = 'Dome' AND COALESCE(g.neutral, 0) = 0 AND (COALESCE(g.conditions__snow, 0) > 0 OR COALESCE(g.conditions__rain, 0) > 0 OR COALESCE(g.conditions__wind, 0) > 2)""",
  "A game under a fixed roof has no snow, rain or wind.", needs=G)
check("N37.nfl_snow", WP, "N37", "NFL snow games a season", "games",
  "SELECT 1.0 * SUM(COALESCE(conditions__snow, 0) > 0) / COUNT(DISTINCT season) FROM log_games WHERE stream = 'nfl' AND COALESCE(kind, 'regular') <> 'preseason'",
  Memory(5.0, "real: a handful of NFL games a season are played in falling snow (from memory)"), (0.4, 0.8, 0.0), "NFL games with snow falling, per logged season.", needs=G)
check("N37.nfl_freezing", WP, "N37", "NFL games below freezing a season", "games",
  "SELECT 1.0 * SUM(conditions__temperature < 32) / COUNT(DISTINCT season) FROM log_games WHERE stream = 'nfl' AND COALESCE(kind, 'regular') <> 'preseason'",
  Fixed(9.2, "nfl_history.json weather_splits: 17,464 pass attempts below 32°F in 1999-2025 at about 70 a game, about 9 games a season (games with a reading, about 77% of all)", "1999-2025"), (0.3, 0.6, 0.0),
  "NFL games below 32°F per logged season.", needs=G)

# ---------------------------------------------------------------- N38 births
BORN = f"""bn AS (SELECT p.player__id AS id, p.name, p.born, p.class_of, p.stage, json_extract(p.pro__draft, '$[0]') AS drafted, p.player__position AS pos,
              CAST(strftime('%m', {DT('p.born')}) AS INTEGER) AS month, CAST(strftime('%Y', {DT('p.born')}) AS INTEGER) AS by
         FROM people p WHERE p.born IS NOT NULL AND p.class_of IS NOT NULL),
       bc AS (SELECT bn.*, class_of - (by + 18 + (month >= 9)) AS off FROM bn)"""
col("people", "pro__draft")
HELD = [0.08, 0.08, 0.08, 0.09, 0.10, 0.15, 0.17, 0.19, 0.04, 0.05, 0.06, 0.07]
MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
MN = "(CASE month " + " ".join(f"WHEN {i + 1} THEN '{m}'" for i, m in enumerate(MONTHS)) + " END)"
panel("N38.months", WP, "N38", "Birth months: every boy, the drafted and the real NFL", "bars",
  "Share of births in each month: all people in the world, and those drafted, against real NFL players born 1975-2003 (nfl_careers.json birth_months, n 14,876). Real US births run about 8.0-8.9% a month with a late-summer peak (CDC). A strong relative-age effect would tilt draftees to the months just after the 1 September school cutoff.",
  f"""WITH {BORN} SELECT * FROM (SELECT 'All' AS grp, {MN} AS label, ROUND(100.0 * COUNT(*) / (SELECT COUNT(*) FROM bn), 2) AS world, NULL AS real, COUNT(*) AS n, month AS _m FROM bn GROUP BY month
     UNION ALL SELECT 'Drafted', {MN}, ROUND(100.0 * COUNT(*) / (SELECT COUNT(*) FROM bn WHERE drafted IS NOT NULL), 2),
            ROUND(100 * json_extract({NC}, '$.birth_months.share_by_month."' || month || '"'), 2), COUNT(*), month FROM bn WHERE drafted IS NOT NULL GROUP BY month) ORDER BY grp, _m""")
panel("N38.held_back", WP, "N38", "A year older than his class, by birth month", "bars",
  "Share of boys whose class is a year later than their birth date gives (held back or started late), by birth month, against the engine's target (calendar/born.rs HELD_BACK: CDC natality 2015-2023, NCES, Deming and Dynarski 2008): summer birthdays most.",
  f"""WITH {BORN} SELECT 'Held back' AS grp, {MN} AS label, ROUND(100.0 * SUM(off = 1) / COUNT(*), 1) AS world,
            (CASE month """ + " ".join(f"WHEN {i + 1} THEN {round(h * 100, 1)}" for i, h in enumerate(HELD)) + f""" END) AS real, COUNT(*) AS n FROM bc GROUP BY month ORDER BY month""",
  says=f"""WITH {BORN} SELECT printf('%.1f%% of boys are a year older than their class and %.1f%% a year younger (target about 10%% and 1%%); %d ages contradict the class by more than a year.',
     100.0 * SUM(off = 1) / COUNT(*), 100.0 * SUM(off = -1) / COUNT(*), SUM(ABS(off) > 1)) FROM bc""")
panel("N38.draft_age", WP, "N38", "Age at the draft against real", "hist",
  "Drafted players by age on draft day (born to the last Thursday of April of the draft year) against the real share by age (nfl_careers.json draft_age, 1980-2026).",
  f"""WITH {BORN}, a AS (SELECT CAST((julianday(drafted || '-04-25') - julianday({DT('born')})) / 365.25 AS INTEGER) AS age FROM bn WHERE drafted IS NOT NULL)
     SELECT 'Drafted' AS grp, age AS x, ROUND(100.0 * COUNT(*) / (SELECT COUNT(*) FROM a), 1) AS world, ROUND(100 * json_extract({NC}, '$.draft_age.share_by_age."' || age || '"'), 1) AS real, COUNT(*) AS n
       FROM a GROUP BY age ORDER BY age""")
check("N38.held_back", WP, "N38", "Boys a year older than their class", "%",
  f"WITH {BORN} SELECT 100.0 * SUM(off = 1) / COUNT(*) FROM bc",
  Fixed(10.0, "calendar/born.rs: about 1 in 10 boys a year older than his class (NCES; Deming and Dynarski 2008)", "2010s"), (0.15, 0.35, 0.0), "Share of people whose class is a year later than their birth date gives.")
check("N38.fall_born_drafted", WP, "N38", "Drafted players born September to November", "%",
  f"WITH {BORN} SELECT 100.0 * SUM(month BETWEEN 9 AND 11) / COUNT(*) FROM bn WHERE drafted IS NOT NULL",
  Fixed(25.98, "nfl_careers.json birth_months: NFL players born 1975-2003, September 8.93% + October 8.58% + November 8.47%", "1975-2003"), (0.06, 0.15, 0.0),
  "The oldest in their class (born just after the 1 September cutoff) among the drafted.")
check("N38.draft_age_22", WP, "N38", "Drafted at 22", "%",
  f"WITH {BORN}, a AS (SELECT CAST((julianday(drafted || '-04-25') - julianday({DT('born')})) / 365.25 AS INTEGER) AS age FROM bn WHERE drafted IS NOT NULL) SELECT 100.0 * SUM(age = 22) / COUNT(*) FROM a",
  Key("nfl_careers.json:draft_age.share_by_age.22", 100.0), (0.1, 0.25, 0.0), "Share of draftees aged 22 on draft day.")
rule("N38.class_fits_birth", "N38", "No age contradicts the class year by more than a year",
  f"WITH {BORN} SELECT COUNT(*) FROM bc",
  f"WITH {BORN} SELECT id, name, {DT('born')} AS born, class_of, off FROM bc WHERE ABS(off) > 1",
  "A boy is at most a year older (held back) or a year younger (skipped) than his class.")

# ---------------------------------------------------------------- N39 the calendar log
REALNFL = """(SELECT COUNT(DISTINCT team) FROM engine_franchises WHERE h.year BETWEEN "from" AND "to") * (CASE WHEN h.year = 1987 THEN 15 WHEN h.year < 2021 THEN 16 ELSE 17 END) / 2
             + CASE WHEN h.year < 1990 THEN 9 WHEN h.year < 2020 THEN 11 ELSE 13 END"""
panel("N39.years", WP, "N39", "The calendar log, year by year", "table",
  "Each football year's counters (World.log_history): college games, opt-outs, redshirts, spring breakouts, high school games and forfeits, commitments and decommitments, flips, signings early and late, visits, early enrollees, declared and withdrew, portal entries and landings, whispers, ineligibles, combine and all-star invites, NFL games and preseason games, medical hardships, sixth years and COVID returns. Spin-up years (before the first recorded season) are marked and left out of checks.",
  f"""SELECT h.year, CASE WHEN h.year < {FIRST} THEN 'spin-up' ELSE '' END AS note, h.games AS "college games", h.opt_outs AS "opt-outs", h.four_game_redshirts AS "4-game redshirts",
            h.spring_breakouts AS "spring breakouts", h.hs_games AS "HS games", h.hs_forfeits AS forfeits, h.commits, h.decommits, h.flips, h.signed_early AS "signed early", h.signed_late AS "signed late",
            h.official_visits AS "official visits", h.unofficial_visits AS "unofficial visits", h.early_enrollees AS "early enrollees", h.declared, h.withdrew, h.portal_entries AS "portal entries",
            h.portal_landed AS landed, h.whispers, h.spring_ineligible + h.fall_ineligible AS ineligible, h.combine_invites AS "combine invites", h.all_star_invites AS "all-star invites",
            h.nfl_games AS "NFL games", {REALNFL} AS "real NFL games", h.nfl_preseason_games AS "NFL preseason", h.medical_hardships AS "medical hardships", h.sixth_years AS "sixth years", h.covid_returns AS "COVID returns"
       FROM log_history h ORDER BY h.year DESC""")
panel("N39.nfl_games", WP, "N39", "NFL games a year against real", "line",
  "NFL games each season, playoffs included (World.log_history nfl_games), against real: clubs times games over two plus the playoff games (9 to 1989, 11 to 2019, 13 from 2020): 256 + 11 = 267 from 2002, 272 + 13 = 285 from 2021. Preseason: 4 a club to 2019, none in 2020, 3 from 2021 (the Hall of Fame game on top).",
  f"""SELECT h.year, h.nfl_games AS "NFL games", {REALNFL} AS real, h.nfl_preseason_games AS preseason FROM log_history h WHERE h.year >= {FIRST} ORDER BY h.year""")
panel("N39.commits", WP, "N39", "Commitments by month and signings by division", "bars",
  "All recorded years together: verbal commitments by month (log_history commits_by_month, January first) and signings by division (signed_by_division).",
  f"""SELECT 'Commitments by month' AS grp, CASE CAST(j.key AS INTEGER) """ + " ".join(f"WHEN {i} THEN '{m}'" for i, m in enumerate(MONTHS)) + f""" END AS label, SUM(j.value) AS world, NULL AS real, SUM(j.value) AS n
       FROM log_history h, json_each(h.commits_by_month) j WHERE h.year >= {FIRST} GROUP BY j.key
     UNION ALL SELECT 'Signings by division', CAST(s._0 AS TEXT), SUM(s._1), NULL, SUM(s._1) FROM log_history__signed_by_division s JOIN log_history h ON h._row = s._parent WHERE h.year >= {FIRST} GROUP BY s._0
     ORDER BY 1""")
rule("N39.nfl_games_real", "N39", "NFL games each recorded year match the real schedule",
  f"SELECT COUNT(*) FROM log_history WHERE year >= {FIRST}",
  f"SELECT h.year, h.nfl_games AS world, {REALNFL} AS real FROM log_history h WHERE h.year >= {FIRST} AND h.year < (SELECT year FROM world) AND h.nfl_games <> {REALNFL}",
  "Regular-season games (clubs × games ÷ 2) plus the playoff games of each era.")
check("N39.preseason", WP, "N39", "NFL preseason games a year from 2021", "games",
  f"SELECT AVG(nfl_preseason_games) FROM log_history WHERE year >= MAX(2021, {FIRST}) AND year < (SELECT year FROM world)",
  Fixed(49.0, "NFL: 3 preseason games a club from 2021 (48) plus the Hall of Fame game", "2021-2025"), (0.0, 0.05, 0.0), "Preseason games a year from 2021.")

# ---------------------------------------------------------------- N40 news
panel("N40.kinds", WP, "N40", "News by kind and year", "matrix",
  "Every news and inbox item (World.news, never pruned) by kind (rows) and year (columns). NFL news posts as calendar events instead (N36).",
  """SELECT channel AS grp, COALESCE(kind, 'other') AS row, year AS col, COUNT(*) AS value, NULL AS real, COUNT(*) AS n FROM news GROUP BY channel, kind, year ORDER BY 1, 2, 3""")
panel("N40.list", WP, "N40", "The news, newest first", "table",
  "Each item: date, kind, channel, program and text (the latest 5,000).",
  f"""SELECT year, {DT('n.date')} AS date, COALESCE(n.kind, '') AS kind, n.channel, (SELECT {prog('p')} FROM programs p WHERE p._row = n.program) AS program, n.program AS id, n.text
       FROM news n ORDER BY n.date DESC LIMIT 5000""", open="e/programs/{id}")
panel("N40.carousel", WP, "N40", "The coaching carousel each year", "table",
  "Head-coach news by year and division of the program: hired, fired, left, extended. Real FBS: about 20-30 head coaching changes a year since 2010.",
  """SELECT n.year, p.division, SUM(n.kind = 'CoachHired') AS hired, SUM(n.kind = 'CoachFired') AS fired, SUM(n.kind = 'CoachLeft') AS "left", SUM(n.kind = 'CoachExtended') AS extended
       FROM news n JOIN programs p ON p._row = n.program WHERE n.kind IN ('CoachHired', 'CoachFired', 'CoachLeft', 'CoachExtended')
      GROUP BY n.year, p.division ORDER BY n.year DESC, """ + divord('p.division'))
panel("N40.diversity", WP, "N40", "FBS head coaches by background, today", "bars",
  "Today's FBS head coaches by ethnicity (coaches table), with the latest diversity report's words in the verdict. Real 2024: about 14-16 of 134 FBS head coaches Black, a handful Hispanic and Polynesian.",
  """SELECT 'FBS head coaches' AS grp, c.ethnicity AS label, COUNT(*) AS world, NULL AS real, COUNT(*) AS n FROM coaches c JOIN programs p ON p._row = c.job__employer__College
      WHERE c.job__role = 'HeadCoach' AND p.division = 'FBS' GROUP BY c.ethnicity ORDER BY world DESC""",
  says="SELECT text FROM news WHERE kind = 'DiversityReport' ORDER BY date DESC LIMIT 1")
check("N40.fbs_changes", WP, "N40", "FBS head coaches hired a year", "coaches",
  "SELECT 1.0 * COUNT(*) / COUNT(DISTINCT n.year) FROM news n JOIN programs p ON p._row = n.program WHERE n.kind = 'CoachHired' AND p.division = 'FBS' AND n.text NOT LIKE '% as %coach%' AND n.text NOT LIKE '% as %coordinator%'",
  Memory(24.0, "real: about 20-30 FBS head coaching changes a year since 2010 (from memory)"), (0.25, 0.5, 0.0), "FBS head-coach hirings in the news per year (assistants hired \"as\" a position coach or coordinator left out).")
check("N40.black_fbs_coaches", WP, "N40", "Black FBS head coaches", "%",
  """SELECT 100.0 * SUM(c.ethnicity = 'Black') / COUNT(*) FROM coaches c JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role = 'HeadCoach' AND p.division = 'FBS'""",
  Fixed(11.9, "CBS Sports 2024-05-16: 16 Black head coaches at 134 FBS schools heading into 2024 (tools/dashboard/build_data.py black_hc)", "2024"), (0.25, 0.5, 0.0), "Share of today's FBS head coaches who are Black.")
