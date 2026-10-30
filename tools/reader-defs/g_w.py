from common import *

MP, HP = "money", "high-school"
def DT(d): return f"date({d} * 86400, 'unixepoch')"

def interp(kind, y):
    """A real anchor series at year y, straight lines between anchors; NULL outside them."""
    return (f"(SELECT a1.value + CASE WHEN a2.year IS NULL THEN 0 ELSE (a2.value - a1.value) * ({y} - a1.year) / (a2.year - a1.year) END "
            f"FROM engine_finance_anchors a1 LEFT JOIN engine_finance_anchors a2 ON a2.kind = a1.kind AND a2.year = "
            f"(SELECT MIN(year) FROM engine_finance_anchors WHERE kind = '{kind}' AND year > {y}) "
            f"WHERE a1.kind = '{kind}' AND a1.year = (SELECT MAX(year) FROM engine_finance_anchors WHERE kind = '{kind}' AND year <= {y}) "
            f"AND {y} <= (SELECT MAX(year) FROM engine_finance_anchors WHERE kind = '{kind}'))")

# NFL books: one row a club a closed year with income and costs.
NY = """ny AS (SELECT y._parent AS team, y.year, y.l,
              (SELECT SUM(j.value) FROM json_each(y.l) j WHERE j.key < 10) AS rev,
              (SELECT SUM(j.value) FROM json_each(y.l) j WHERE j.key >= 10) AS cost FROM finance__nfl__years y)"""
NS_ = "finance__nfl__seasons"
LASTNY = "(SELECT MAX(year) FROM finance__nfl__years)"

# ---------------------------------------------------------------- N30 NFL club money
panel("N30.league", MP, "N30", "League revenue by year against the real anchors", "line",
  "The league's whole revenue each year as the clubs' books add up (finance.league), in billions, against the real anchors the money follows (finance/data.rs LEAGUE_REVENUE: Forbes and Sportico, about $8.5B in 2010, $15.3B in 2019, $12.2B in 2020 with empty stadiums, $23B in 2024), straight lines between anchors.",
  f"""SELECT 'League revenue ($B)' AS grp, l._0 AS year, ROUND(l._1 / 1e9, 2) AS world, ROUND({interp('league revenue ($B)', 'l._0')}, 2) AS real FROM finance__league l ORDER BY l._0""",
  says=f"""SELECT printf('%d: the league took in $%.1fB (real anchor $%.1fB); 2020 against 2019: %.0f%% (real 80%%, the empty stadiums).', l._0, l._1 / 1e9,
     {interp('league revenue ($B)', 'l._0')}, 100.0 * (SELECT _1 FROM finance__league WHERE _0 = 2020) / (SELECT _1 FROM finance__league WHERE _0 = 2019))
     FROM finance__league l ORDER BY l._0 DESC LIMIT 1""")
panel("N30.per_club", MP, "N30", "Revenue per club: the average, the richest and the poorest", "line",
  "Each closed year's income per club in millions: the average, the top club and the bottom one, against the real average (the real league anchor over the clubs that year). Real 2024: about $720M a club, the Cowboys far ahead at about $1.2B.",
  f"""WITH {NY} SELECT ny.year, ROUND(AVG(rev) / 1e6) AS average, ROUND(MAX(rev) / 1e6) AS top, ROUND(MIN(rev) / 1e6) AS bottom,
            ROUND({interp('league revenue ($B)', 'ny.year')} * 1000 / COUNT(*)) AS real FROM ny GROUP BY ny.year ORDER BY ny.year""",
  says=f"""WITH {NY} SELECT printf('%d: the average club took in $%.0fM; the richest, %s, $%.0fM (real 2024 average about $720M, Dallas about $1.2B).', year, AVG(rev) / 1e6,
     (SELECT name FROM nfl WHERE _row = (SELECT team FROM ny n2 WHERE n2.year = ny.year ORDER BY rev DESC LIMIT 1)), MAX(rev) / 1e6) FROM ny WHERE year = {LASTNY}""")
panel("N30.books", MP, "N30", "The average club's books on 21 lines, the last closed year and the one before", "table",
  "The annual report of the average club: every income and cost line (finance/nfl.rs LINES) for the last closed money year, the year before, the change, and the lowest and highest club. Player pay is cash paid in the year, bonuses included.",
  f"""WITH ly AS (SELECT {LASTNY} AS y),
          v AS (SELECT y._parent AS team, y.year, j.key AS i, j.value AS v FROM finance__nfl__years y, json_each(y.l) j WHERE y.year >= (SELECT y FROM ly) - 1)
     SELECT f.i + 1 AS line_no, f.name AS line, CASE f.income WHEN 1 THEN 'income' ELSE 'cost' END AS side,
            ROUND(AVG(CASE WHEN v.year = (SELECT y FROM ly) THEN v.v END) / 1e6, 1) AS "average $M",
            ROUND(AVG(CASE WHEN v.year = (SELECT y FROM ly) - 1 THEN v.v END) / 1e6, 1) AS "year before $M",
            ROUND(100.0 * (AVG(CASE WHEN v.year = (SELECT y FROM ly) THEN v.v END) / NULLIF(AVG(CASE WHEN v.year = (SELECT y FROM ly) - 1 THEN v.v END), 0) - 1), 1) AS "change %",
            ROUND(MIN(CASE WHEN v.year = (SELECT y FROM ly) THEN v.v END) / 1e6, 1) AS "lowest $M", ROUND(MAX(CASE WHEN v.year = (SELECT y FROM ly) THEN v.v END) / 1e6, 1) AS "highest $M"
       FROM engine_finance_lines f JOIN v ON v.i = f.i WHERE f.level = 'nfl' GROUP BY f.i ORDER BY f.i""",
  says=f"""WITH {NY} SELECT printf('%d: the average club took in $%.0fM and spent $%.0fM; player pay and benefits were %.1f%% of revenue (real: the CBA gives players about 48%% of all revenue).',
     year, AVG(rev) / 1e6, AVG(cost) / 1e6, 100.0 * SUM(json_extract(l, '$[10]') + json_extract(l, '$[11]')) / SUM(rev)) FROM ny WHERE year = {LASTNY}""")
panel("N30.clubs", MP, "N30", "Every club's year: money, crowds, tickets and value", "table",
  "One row a club a closed year: income, costs, profit and income rank; home games, crowd a game against capacity, sellouts and blackouts; the average ticket, season-ticket renewal and waitlist; the club's value and its multiple of revenue. Owner and stadium are today's.",
  f"""WITH {NY} SELECT n.year, {club('n.team', 'n.year')} AS club, n.team AS id, ROUND(n.rev / 1e6) AS "income $M", ROUND(n.cost / 1e6) AS "costs $M",
            ROUND((n.rev - n.cost) / 1e6) AS "profit $M", RANK() OVER (PARTITION BY n.year ORDER BY n.rev DESC) AS "income rank",
            s.home_games AS "home games", CAST(s.attendance / NULLIF(s.home_games, 0) AS INTEGER) AS "crowd a game", s.capacity, s.sellouts, s.blackouts,
            ROUND(s.price, 2) AS "ticket $", ROUND(100 * s.renewal, 1) AS "renewal %", s.waitlist, ROUND(s.value / 1e9, 2) AS "value $B", ROUND(s.value / n.rev, 1) AS "value / income",
            f.owner_name AS "owner today", f.home__name AS "stadium today"
       FROM ny n LEFT JOIN {NS_} s ON s._parent = n.team AND s.year = n.year LEFT JOIN finance__nfl f ON f._row = n.team ORDER BY n.year DESC, n.rev DESC""",
  open="club/{id}/{year}")
panel("N30.attendance", MP, "N30", "Crowd a home game, club by year", "matrix",
  "Each club's average home crowd (thousands) by season, from its season summaries; hover for sellouts and blackouts. Real: about 66,000-70,000 a game across the league; blackouts were common in the 1980s and 1990s (Bengals, Buccaneers, Jaguars, Chargers) and ended after 2014.",
  f"""SELECT 'Crowd (thousands)' AS grp, {club('s._parent', 's.year')} AS row, s.year AS col, ROUND(s.attendance / 1000.0 / NULLIF(s.home_games, 0), 1) AS value, NULL AS real,
            s.home_games AS n, s.sellouts, s.blackouts FROM {NS_} s ORDER BY s._parent, s.year""",
  says=f"""SELECT printf('Across all seasons: %.0f a game; %d blackouts, %d of them after 2014 (real none after 2014).', 1.0 * SUM(attendance) / SUM(home_games), SUM(blackouts),
     SUM(CASE WHEN year > 2014 THEN blackouts ELSE 0 END)) FROM {NS_}""")
panel("N30.blackouts", MP, "N30", "Blackouts by decade and by club", "bars",
  "Home games blacked out on local TV (not sold out 72 hours before kickoff, the rule from 1973 until the league suspended it in 2015). Real: many in the 1980s and 1990s, about 10% of games in the early 1990s, mostly small markets with big stadiums (Bengals, Buccaneers, Jaguars, Chargers, Raiders); none after 2014.",
  f"""SELECT 'By decade' AS grp, ((year / 10) * 10) || 's' AS label, SUM(blackouts) AS world, NULL AS real, SUM(home_games) AS n FROM {NS_} GROUP BY year / 10
     UNION ALL SELECT 'By club', {cname('_parent')}, SUM(blackouts), NULL, SUM(home_games) FROM {NS_} GROUP BY _parent HAVING SUM(blackouts) > 0 ORDER BY 1, 2""")
panel("N30.ticket", MP, "N30", "The average ticket by year against real", "line",
  "The average ticket across the clubs each season against Team Marketing Report's Fan Cost Index (finance/data.rs TICKET_PRICE: $25.21 in 1991, $49.35 in 2000, $76.47 in 2010, $100.75 in 2019).",
  f"""SELECT 'Average ticket ($)' AS grp, s.year, ROUND(AVG(s.price), 2) AS world, ROUND({interp('average ticket ($)', 's.year')}, 2) AS real FROM {NS_} s GROUP BY s.year ORDER BY s.year""")
panel("N30.values", MP, "N30", "Club values against Forbes", "line",
  "The average club's value (billions) and its multiple of income, against the real multiple (Forbes average value over average revenue: about 3.5 in 2000, 3.8 in 2010, 7.9 in 2024) times the real revenue a club. Real 2024: $5.9B average.",
  f"""WITH {NY} SELECT 'Value ($B)' AS grp, n.year, ROUND(AVG(s.value) / 1e9, 2) AS world,
            ROUND({interp('value multiple', 'n.year')} * {interp('league revenue ($B)', 'n.year')} / COUNT(*), 2) AS real
       FROM ny n JOIN {NS_} s ON s._parent = n.team AND s.year = n.year GROUP BY n.year
     UNION ALL SELECT 'Value over income', n.year, ROUND(AVG(s.value) / AVG(n.rev), 2), ROUND({interp('value multiple', 'n.year')}, 2)
       FROM ny n JOIN {NS_} s ON s._parent = n.team AND s.year = n.year GROUP BY n.year ORDER BY 1, 2""")
panel("N30.gates", MP, "N30", "This season's home gates", "table",
  "Every home game this season (gates are kept for the current season only): the date, the opponent, the crowd against capacity, sold out, blacked out, preseason or playoff.",
  f"""SELECT {DT('g.date')} AS date, {cname('g._parent')} AS club, g._parent AS id, {cname('g.opp')} AS opponent, g.attendance AS crowd, f.home__capacity AS capacity,
            ROUND(100.0 * g.attendance / NULLIF(f.home__capacity, 0), 1) AS "full %", CASE WHEN g.sold_out THEN 'yes' ELSE '' END AS "sold out",
            CASE WHEN g.blackout THEN 'yes' ELSE '' END AS blackout, CASE WHEN g.playoff THEN 'playoff' WHEN g.preseason THEN 'preseason' ELSE 'regular' END AS game
       FROM finance__nfl__gates g LEFT JOIN finance__nfl f ON f._row = g._parent ORDER BY g.date, g._parent""", open="e/nfl/{id}")
panel("N30.stadiums", MP, "N30", "Stadiums, owners and the facility", "table",
  "Each club today: its stadium (opened, last renovation, capacity, dome, cost and public share), the owner and since when, season tickets, waitlist and renewal, the training facility's year and the amenities. Real public shares: about 70% of stadium costs before 2000, falling to about 30-40% since (stadium records in finance/data.rs).",
  """SELECT f.home__name AS stadium, (SELECT name FROM nfl WHERE _row = f._row) AS club, f._row AS id, f.home__opened AS opened, f.home__renovated AS renovated, f.home__capacity AS capacity,
            CASE WHEN f.home__dome THEN 'yes' ELSE '' END AS dome, ROUND(f.home__cost / 1e6) AS "cost $M", ROUND(100 * f.home__public) AS "public %", f.owner_name AS owner, f.owner_since AS since,
            f.season_tickets AS "season tickets", f.waitlist, ROUND(100 * f.renewal, 1) AS "renewal %", f.facility_year AS facility, ROUND(f.amenities, 2) AS amenities, f.votes_lost AS "stadium votes lost"
       FROM finance__nfl f ORDER BY f.home__opened DESC""", open="e/nfl/{id}")
col("finance__nfl", "home__dome"); col("finance__nfl", "facility_year"); col("finance__nfl", "amenities"); col("finance__nfl", "votes_lost")
panel("N30.projects", MP, "N30", "Stadium and facility projects", "table",
  "Every project under way or finished: who builds it (NFL club or college), what, the cost and the public share, money pledged, started and opening year, capacity and how far along. Real 2026: Buffalo, Tennessee, Cleveland and Chicago projects (finance/data.rs PROJECTS_2026).",
  f"""SELECT COALESCE({cname('p.owner__Nfl')}, (SELECT {prog('pr')} FROM programs pr WHERE pr._row = p.owner__College)) AS owner,
            CASE WHEN p.owner__Nfl IS NOT NULL THEN 'NFL' ELSE 'College' END AS level, p.kind, p.name, ROUND(p.cost / 1e6) AS "cost $M", ROUND(100 * p.public) AS "public %",
            ROUND(p.pledged / 1e6) AS "pledged $M", p.started, p.opens, NULLIF(p.capacity, 0) AS capacity, CASE WHEN p.done THEN 'done' ELSE ROUND(100 * p.step) || '%' END AS state
       FROM finance__projects p ORDER BY p.opens DESC, p.cost DESC""")
col("finance__projects", "owner__College"); col("finance__projects", "owner__Nfl")
GRADE = "(SELECT grade FROM engine_grades WHERE _row = json_extract(c.grades, '$[{i}]'))"
CARD_COLS = ", ".join(f'{GRADE.format(i=i)} AS "{n}"' for i, n in enumerate(["families", "food", "nutritionist", "locker room", "training room", "training staff", "weight room", "strength coaches", "travel", "head coach", "owner"]))
panel("N30.cards", MP, "N30", "The players' report cards", "table",
  "The NFLPA's team report card each year from 2023 (published 2023-2024; an arbitrator kept it internal from 2025): the club's rank and its grade in each of 11 parts (finance/data.rs CARD_PARTS).",
  f"""SELECT c.year, c.rank, {cname('c.team')} AS club, c.team AS id, {CARD_COLS} FROM finance__cards c ORDER BY c.year DESC, c.rank""", open="e/nfl/{id}",
  says="""WITH a AS (SELECT year, (SELECT AVG(value) FROM json_each(grades)) AS a FROM finance__cards WHERE year = (SELECT MAX(year) FROM finance__cards)),
          m AS (SELECT year, MAX(a) AS hi, MIN(a) AS lo FROM a)
     SELECT printf('%d: the best club averages %s, the worst %s (real 2024: Miami first, Washington last before its new owner).', year,
       (SELECT grade FROM engine_grades WHERE _row = CAST(hi + 0.5 AS INTEGER)), (SELECT grade FROM engine_grades WHERE _row = CAST(lo + 0.5 AS INTEGER))) FROM m""")
panel("N30.sales", MP, "N30", "Club sales and stadium votes", "table",
  "The owners' approvals of club sales and stadium votes as posted on the calendar (no structured sales record is kept: the text is the record). Real controlling sales since 2010: Rams 2010, Browns 2012, Bills 2014, Panthers 2018, Broncos 2022, Commanders 2023, about one every 2-3 years.",
  f"""SELECT CAST(strftime('%Y', {DT('h.when__date')}) AS INTEGER) AS year, {DT('h.when__date')} AS date, {cname('h.nfl_team')} AS club, h.nfl_team AS id, h.text
       FROM calendar__history h WHERE h.rule = 'money' ORDER BY h.when__date""", open="e/nfl/{id}",
  says="""SELECT printf('%d club sales in %d years: %.2f a year (real about 0.4).', SUM(text LIKE '%approve the sale%'), MAX(y) - MIN(y) + 1, 1.0 * SUM(text LIKE '%approve the sale%') / (MAX(y) - MIN(y) + 1))
     FROM (SELECT text, CAST(strftime('%Y', date(when__date * 86400, 'unixepoch')) AS INTEGER) AS y FROM calendar__history WHERE rule = 'money' OR level = 'Nfl')""")
check("N30.league_2024", MP, "N30", "League revenue in 2024", "$B", "SELECT _1 / 1e9 FROM finance__league WHERE _0 = 2024",
  Fixed(23.0, "Forbes and Sportico: NFL revenue about $23B in 2024 (finance/data.rs LEAGUE_REVENUE)", "2024"), (0.06, 0.15, 0.0), "The league's revenue as the clubs' books add up.")
check("N30.league_2010", MP, "N30", "League revenue in 2010", "$B", "SELECT _1 / 1e9 FROM finance__league WHERE _0 = 2010",
  Fixed(8.5, "Forbes: NFL revenue about $8.5B in 2010 (finance/data.rs LEAGUE_REVENUE)", "2010"), (0.06, 0.15, 0.0), "The league's revenue as the clubs' books add up.")
check("N30.covid_dip", MP, "N30", "League revenue in 2020 against 2019", "%",
  "SELECT 100.0 * (SELECT _1 FROM finance__league WHERE _0 = 2020) / (SELECT _1 FROM finance__league WHERE _0 = 2019)",
  Fixed(79.7, "NFL revenue fell from about $15.3B in 2019 to $12.2B in 2020 with empty stadiums (finance/data.rs LEAGUE_REVENUE)", "2019-2020"), (0.04, 0.1, 0.0),
  "2020's revenue as a share of 2019's: the empty stadiums of the COVID season.")
check("N30.club_revenue", MP, "N30", "Revenue per club in 2024", "$M", f"WITH {NY} SELECT AVG(rev) / 1e6 FROM ny WHERE year = 2024",
  Fixed(720.0, "Forbes 2024: about $720M a club ($23B over 32)", "2024"), (0.06, 0.15, 0.0), "The average club's income in the 2024 money year.")
check("N30.dallas_rank", MP, "N30", "Where Dallas ranks in income", "rank",
  f"WITH {NY}, r AS (SELECT team, RANK() OVER (ORDER BY rev DESC) AS rk FROM ny WHERE year = {LASTNY}) SELECT rk FROM r WHERE team = (SELECT _row FROM nfl WHERE name LIKE '%Cowboys')",
  Fixed(1.0, "Forbes: the Cowboys lead the league in revenue every year since the mid-2000s (about $1.2B in 2024)", "2005-2024"), (0.0, 1.0, 0.0), "Dallas's income rank among the clubs in the last closed year.")
check("N30.player_share", MP, "N30", "Player pay and benefits as a share of revenue", "%",
  f"WITH {NY} SELECT 100.0 * SUM(json_extract(l, '$[10]') + json_extract(l, '$[11]')) / SUM(rev) FROM ny WHERE year >= {LASTNY} - 4",
  Memory(44.0, "2024: the cap ($255.4M a club) plus benefits (about $60M a club) over revenue (about $720M a club); the CBA's 48.8% is of its own 'all revenue', which leaves some income out (from memory)"), (0.08, 0.18, 0.0),
  "Cash player pay plus benefits over all club income, the last five closed years.")
check("N30.attendance", MP, "N30", "Crowd a home game", "people", f"SELECT 1.0 * SUM(attendance) / SUM(home_games) FROM {NS_} WHERE year >= (SELECT MAX(year) FROM {NS_}) - 4",
  Fixed(69000.0, "NFL average attendance about 69,000 a game (2024; 66,000-70,000 since 2000)", "2020-2024"), (0.04, 0.1, 0.0), "Average home crowd over the last five seasons.")
check("N30.ticket_2019", MP, "N30", "Average ticket in 2019", "$", f"SELECT AVG(price) FROM {NS_} WHERE year = 2019",
  Fixed(100.75, "Team Marketing Report Fan Cost Index: $100.75 in 2019", "2019"), (0.05, 0.15, 0.0), "The clubs' average ticket in 2019.")
check("N30.ticket_2010", MP, "N30", "Average ticket in 2010", "$", f"SELECT AVG(price) FROM {NS_} WHERE year = 2010",
  Fixed(76.47, "Team Marketing Report Fan Cost Index: $76.47 in 2010", "2010"), (0.05, 0.15, 0.0), "The clubs' average ticket in 2010.")
check("N30.value_2024", MP, "N30", "Average club value in 2024", "$B", f"SELECT AVG(value) / 1e9 FROM {NS_} WHERE year = 2024",
  Fixed(5.9, "Forbes 2024: average NFL club value $5.9B", "2024"), (0.08, 0.2, 0.0), "The average club's value at the end of the 2024 season.")
check("N30.value_multiple", MP, "N30", "Club value over income in 2024", "x",
  f"WITH {NY} SELECT AVG(s.value) / AVG(n.rev) FROM ny n JOIN {NS_} s ON s._parent = n.team AND s.year = n.year WHERE n.year = 2024",
  Fixed(7.9, "Forbes 2024: average value over average revenue about 7.9 (finance/data.rs VALUE_MULTIPLE)", "2024"), (0.08, 0.2, 0.0), "Average value over average income, 2024.")
check("N30.sales_a_year", MP, "N30", "Club sales a year", "sales",
  """SELECT 1.0 * SUM(text LIKE '%approve the sale%') / (MAX(y) - MIN(y) + 1) FROM (SELECT text, CAST(strftime('%Y', date(when__date * 86400, 'unixepoch')) AS INTEGER) AS y FROM calendar__history WHERE level = 'Nfl')""",
  Memory(0.4, "controlling sales since 2010: Rams 2010, Browns 2012, Bills 2014, Panthers 2018, Broncos 2022, Commanders 2023 (from memory)"), (0.25, 0.6, 0.0),
  "Approved club sales on the calendar over the years it covers.")
rule("N30.no_blackouts_after_2014", "N30", "No blackouts after 2014 (the league suspended the rule in 2015)",
  f"SELECT COUNT(*) FROM {NS_} WHERE year > 2014",
  f"SELECT year, {cname('_parent')} AS club, blackouts FROM {NS_} WHERE year > 2014 AND blackouts > 0",
  "The NFL suspended its blackout rule from the 2015 season and never brought it back.")

# ---------------------------------------------------------------- N31 college department money
TIER = """(CASE WHEN p.division = 'FBS' AND p.conference IN ('SEC', 'Big Ten') THEN '1 SEC and Big Ten'
                WHEN p.division = 'FBS' AND (p.conference IN ('ACC', 'Big 12', 'Pac-12') OR p.real_name = 'Notre Dame') THEN '2 ACC, Big 12, Pac-12'
                WHEN p.division = 'FBS' THEN '3 Group of 5' WHEN p.division = 'FCS' THEN '4 FCS' WHEN p.division = 'D2' THEN '5 D2' ELSE '6 D3, NAIA, JUCO' END)"""
CY = """cy AS (SELECT y._parent AS prog, y.year, y.l,
              (SELECT SUM(j.value) FROM json_each(y.l) j WHERE j.key < 11) AS rev,
              (SELECT SUM(j.value) FROM json_each(y.l) j WHERE j.key >= 11) AS cost FROM finance__college__years y)"""
LASTCY = "(SELECT MAX(year) FROM finance__college__years)"
CS_ = "finance__college__seasons"
panel("N31.tiers", MP, "N31", "Departments by tier and year", "table",
  "Average department books per tier each closed money year (July to June; tiers by today's conference): schools, revenue and costs a school, the university's support and its share of revenue, the conference's check, football tickets, guarantee money in, and the crowd a home game. Real 2023 (finance_check.rs): SEC and Big Ten schools about $150-250M; Group of 5 $40-70M with heavy university support; FCS $15-35M.",
  f"""WITH {CY} SELECT c.year, substr({TIER}, 3) AS tier, COUNT(*) AS schools, ROUND(AVG(c.rev) / 1e6, 1) AS "revenue $M", ROUND(AVG(c.cost) / 1e6, 1) AS "costs $M",
            ROUND(AVG(json_extract(c.l, '$[10]')) / 1e6, 1) AS "university support $M", ROUND(100.0 * SUM(json_extract(c.l, '$[10]') + json_extract(c.l, '$[9]')) / SUM(c.rev), 1) AS "support and fees %",
            ROUND(AVG(json_extract(c.l, '$[0]')) / 1e6, 1) AS "conference $M", ROUND(AVG(json_extract(c.l, '$[2]')) / 1e6, 1) AS "tickets $M",
            ROUND(AVG(json_extract(c.l, '$[7]')) / 1e6, 2) AS "guarantees in $M",
            (SELECT CAST(SUM(s.attendance) / NULLIF(SUM(s.home_games), 0) AS INTEGER) FROM {CS_} s JOIN programs p2 ON p2._row = s._parent
              WHERE s.year = c.year AND {TIER.replace('p.', 'p2.')} = {TIER}) AS "crowd a game"
       FROM cy c JOIN programs p ON p._row = c.prog GROUP BY c.year, {TIER} ORDER BY c.year DESC, {TIER}""",
  says=f"""WITH {CY} SELECT printf('%d: SEC and Big Ten schools average $%.0fM, Group of 5 $%.0fM, FCS $%.0fM (real 2023: $150-250M, $40-70M, $15-35M).', {LASTCY},
     AVG(CASE WHEN {TIER} LIKE '1%' THEN c.rev END) / 1e6, AVG(CASE WHEN {TIER} LIKE '3%' THEN c.rev END) / 1e6, AVG(CASE WHEN {TIER} LIKE '4%' THEN c.rev END) / 1e6)
     FROM cy c JOIN programs p ON p._row = c.prog WHERE c.year = {LASTCY}""")
TIERCOLS = ", ".join(f"""ROUND(AVG(CASE WHEN t LIKE '{k}%' AND v.year = (SELECT y FROM ly) THEN v.v END) / 1e6, 2) AS "{n} $M\"""" for k, n in [("1", "SEC, Big Ten"), ("2", "ACC, Big 12, Pac-12"), ("3", "Group of 5"), ("4", "FCS"), ("5", "D2")])
panel("N31.books", MP, "N31", "The department's books on 24 lines by tier", "table",
  "The average department per tier on every line (finance/college.rs LINES) for the last closed money year. Revenue sharing with players starts in the 2025-26 year (the House settlement, about $20.5M a school at most).",
  f"""WITH ly AS (SELECT {LASTCY} AS y),
          v AS (SELECT y.year, {TIER} AS t, j.key AS i, j.value AS v FROM finance__college__years y JOIN programs p ON p._row = y._parent, json_each(y.l) j WHERE y.year = (SELECT y FROM ly))
     SELECT f.i + 1 AS line_no, f.name AS line, CASE f.income WHEN 1 THEN 'income' ELSE 'cost' END AS side, {TIERCOLS}
       FROM engine_finance_lines f JOIN v ON v.i = f.i WHERE f.level = 'college' GROUP BY f.i ORDER BY f.i""")
panel("N31.depts", MP, "N31", "Every department, the last closed year", "table",
  "Each program's department in the last closed money year: revenue and costs, the net, university support, the conference's check, tickets and seat donations, media and sponsors; home games, the crowd a game, sellouts, the average ticket, season tickets and reserves.",
  f"""WITH {CY} SELECT {prog()} AS program, p._row AS id, p.division, p.conference, ROUND(c.rev / 1e6, 1) AS "revenue $M", ROUND(c.cost / 1e6, 1) AS "costs $M",
            ROUND((c.rev - c.cost) / 1e6, 1) AS "net $M", ROUND(json_extract(c.l, '$[10]') / 1e6, 1) AS "university $M", ROUND(json_extract(c.l, '$[0]') / 1e6, 1) AS "conference $M",
            ROUND((json_extract(c.l, '$[2]') + json_extract(c.l, '$[3]')) / 1e6, 1) AS "tickets and seats $M", ROUND(json_extract(c.l, '$[6]') / 1e6, 1) AS "media $M",
            ROUND(json_extract(c.l, '$[16]') / 1e6, 1) AS "revenue share $M",
            s.home_games AS "home games", CAST(s.attendance / NULLIF(s.home_games, 0) AS INTEGER) AS "crowd a game", s.sellouts, ROUND(s.price, 2) AS "ticket $", s.season_tickets AS "season tickets",
            ROUND(s.reserves / 1e6, 1) AS "reserves $M"
       FROM cy c JOIN programs p ON p._row = c.prog LEFT JOIN {CS_} s ON s._parent = c.prog AND s.year = c.year WHERE c.year = {LASTCY}
      ORDER BY {divord('p.division')}, c.rev DESC""", open="e/programs/{id}")
ATT = f"""att AS (SELECT p._row AS id, {prog()} AS program, p.real_name, s.year, s.attendance, s.home_games FROM {CS_} s JOIN programs p ON p._row = s._parent WHERE p.division = 'FBS' AND s.home_games > 0),
       aw AS (SELECT id, program, real_name, SUM(attendance) AS a, SUM(home_games) AS g FROM att GROUP BY id),
       ar AS (SELECT x.id, AVG(r.attendance) AS ra FROM (SELECT DISTINCT id, real_name, year FROM att) x JOIN real_college_attendance r ON r.home_team = x.real_name AND r.season = x.year GROUP BY x.id)"""
panel("N31.attendance", MP, "N31", "FBS crowds by program against real", "bars",
  "Each FBS program's crowd a home game over the seasons the world and the real record share, against its real average (real_college_attendance: NCAA attendance by home team 2001-2025). Real leaders: Michigan, Penn State and Ohio State above 100,000.",
  f"""WITH {ATT} SELECT 'FBS' AS grp, aw.program AS label, aw.id, ROUND(1.0 * aw.a / aw.g) AS world, ROUND(ar.ra) AS real, aw.g AS n
       FROM aw LEFT JOIN ar ON ar.id = aw.id ORDER BY ar.ra IS NULL, ar.ra DESC, world DESC""", open="e/programs/{id}",
  says=f"""WITH {ATT}, k AS (SELECT 1.0 * aw.a / aw.g AS x, ar.ra AS y FROM aw JOIN ar ON ar.id = aw.id)
     SELECT printf('%d FBS programs with a real record: world crowds run %.0f%% of real on average; the two line up with a correlation of %.2f.', COUNT(*), 100.0 * SUM(x) / SUM(y),
       (AVG(x * y) - AVG(x) * AVG(y)) / sqrt((AVG(x * x) - AVG(x) * AVG(x)) * (AVG(y * y) - AVG(y) * AVG(y)))) FROM k""")
panel("N31.attendance_years", MP, "N31", "FBS crowd a game by year against real", "line",
  "The FBS crowd a home game each season against the real FBS average (real_college_attendance, the programs in the real record that year). Real: about 45,000 in the late 2000s, about 41,000 in 2023 after a slow decline.",
  f"""WITH {ATT} SELECT 'FBS' AS grp, year, ROUND(1.0 * SUM(attendance) / SUM(home_games)) AS world,
            (SELECT ROUND(AVG(attendance)) FROM real_college_attendance r WHERE r.season = att.year) AS real, SUM(home_games) AS n FROM att GROUP BY year ORDER BY year""")
panel("N31.guarantees", MP, "N31", "Guarantee games: who pays whom", "table",
  "Games with a contract fee from the game log (paid__kind 1 is a guarantee game: the bigger program buys the home game), by the home and visiting divisions: games, the average fee and the range. Real (2023-2025 contracts): FCS visitors about $400,000-700,000 at power programs; Group of 5 visitors $1.2-1.9M.",
  """SELECT g.season AS year, json_extract(g.divisions, '$[0]') AS host, json_extract(g.divisions, '$[1]') AS visitor, COUNT(*) AS games,
            ROUND(AVG(g.paid__dollars) / 1e6, 2) AS "average $M", ROUND(MIN(g.paid__dollars) / 1e6, 2) AS "lowest $M", ROUND(MAX(g.paid__dollars) / 1e6, 2) AS "highest $M"
       FROM log_games g WHERE g.stream = 'college' AND g.paid__kind = 1 GROUP BY g.season, host, visitor ORDER BY g.season DESC, games DESC""", needs=G)
panel("N31.deals", MP, "N31", "Conference media deals", "table",
  "Each FBS conference's media deal as it stands: money a school a year and when it runs out (public reporting: ESPN-SEC to 2034, the Big Ten's to 2030). After 2026 the world signs its own.",
  "SELECT conference, ROUND(per_school / 1e6, 1) AS \"a school $M\", until FROM finance__deals ORDER BY per_school DESC")
col("finance__deals", "conference"); col("finance__deals", "per_school"); col("finance__deals", "until")
check("N31.sec_revenue", MP, "N31", "SEC and Big Ten revenue a school", "$M",
  f"WITH {CY} SELECT AVG(c.rev) / 1e6 FROM cy c JOIN programs p ON p._row = c.prog WHERE c.year = {LASTCY} AND p.division = 'FBS' AND p.conference IN ('SEC', 'Big Ten')",
  Fixed(200.0, "USA Today and Knight-Newhouse 2023: SEC and Big Ten schools about $150-250M (finance_check.rs)", "2023"), (0.25, 0.5, 0.0),
  "Average department revenue of today's SEC and Big Ten schools, last closed year.")
check("N31.g5_revenue", MP, "N31", "Group of 5 revenue a school", "$M",
  f"WITH {CY} SELECT AVG(c.rev) / 1e6 FROM cy c JOIN programs p ON p._row = c.prog WHERE c.year = {LASTCY} AND {TIER} LIKE '3%'",
  Fixed(55.0, "USA Today and Knight-Newhouse 2023: Group of 5 schools about $40-70M (finance_check.rs)", "2023"), (0.25, 0.5, 0.0),
  "Average department revenue of Group of 5 schools, last closed year.")
check("N31.fcs_revenue", MP, "N31", "FCS revenue a school", "$M",
  f"WITH {CY} SELECT AVG(c.rev) / 1e6 FROM cy c JOIN programs p ON p._row = c.prog WHERE c.year = {LASTCY} AND p.division = 'FCS'",
  Fixed(25.0, "Knight-Newhouse 2023: FCS departments about $15-35M (finance_check.rs)", "2023"), (0.3, 0.6, 0.0),
  "Average department revenue of FCS schools, last closed year.")
check("N31.g5_support", MP, "N31", "Group of 5 revenue from the university and student fees", "%",
  f"WITH {CY} SELECT 100.0 * SUM(json_extract(c.l, '$[10]') + json_extract(c.l, '$[9]')) / SUM(c.rev) FROM cy c JOIN programs p ON p._row = c.prog WHERE c.year = {LASTCY} AND {TIER} LIKE '3%'",
  Memory(40.0, "Knight-Newhouse: Group of 5 departments take about a third to a half of their revenue from institutional support and student fees (from memory)"), (0.25, 0.5, 0.0),
  "University support plus student fees over all revenue, Group of 5, last closed year.")
check("N31.fbs_attendance", MP, "N31", "FBS crowd a home game", "people",
  f"SELECT 1.0 * SUM(s.attendance) / SUM(s.home_games) FROM {CS_} s JOIN programs p ON p._row = s._parent WHERE p.division = 'FBS' AND s.year = (SELECT MAX(year) FROM {CS_} WHERE home_games > 0)",
  Fixed(41000.0, "NCAA 2023: FBS average about 41,000 a game (finance_check.rs)", "2023"), (0.08, 0.2, 0.0), "FBS crowd a home game in the last season with crowds.")
check("N31.guarantee_fcs", MP, "N31", "Fee paid to an FCS visitor", "$M",
  "SELECT AVG(paid__dollars) / 1e6 FROM log_games WHERE stream = 'college' AND paid__kind = 1 AND json_extract(divisions, '$[0]') = 'FBS' AND json_extract(divisions, '$[1]') = 'FCS'",
  Memory(0.55, "FBS-FCS guarantee contracts 2023-2025: about $400,000-700,000 (from memory; the inventory's $1-2M is the Group of 5 visitor's range)"), (0.3, 0.6, 0.0),
  "Average guarantee fee an FBS host pays an FCS visitor, from the game log.", needs=G)
check("N31.revenue_share_cap", MP, "N31", "Largest revenue share paid to players", "$M",
  f"SELECT MAX(json_extract(l, '$[16]')) / 1e6 FROM finance__college__years WHERE year = {LASTCY} AND year >= 2025",
  Fixed(20.5, "House settlement: a school may share up to about $20.5M with its players in 2025-26, rising about 4% a year", "2025"), (0.05, 0.15, 0.0),
  "The biggest department's revenue-sharing line in the last closed year; no data before the 2025-26 year, when sharing began.")

# ---------------------------------------------------------------- N32 high school budgets
HB = """hb AS (SELECT h._row AS id, COALESCE(h.name, h.fictional_name) AS school, (SELECT code FROM states WHERE _row = h.state) AS state, h.classification AS class, h.enrollment,
              f.last__year AS year, f.last__l AS l, f.budget__l AS b, f.fee, f.crowd,
              (SELECT SUM(j.value) FROM json_each(f.last__l) j WHERE j.key >= 5) AS spent, (SELECT SUM(j.value) FROM json_each(f.last__l) j WHERE j.key > 5) AS beyond,
              (SELECT SUM(j.value) FROM json_each(f.last__l) j WHERE j.key < 5) AS took
         FROM finance__hs f JOIN high_schools h ON h._row = f._row)"""
panel("N32.budgets", HP, "N32", "Every school's football books, last year", "table",
  "Each school's last closed money year (July to June): money in (district, gate, playoff share, pay-to-play, boosters), what it spent and what it spent beyond the coaches' stipends, the pay-to-play fee and the crowd a game. Real: a typical public program spends $25-60K beyond stipends; big Texas programs $150K and more.",
  f"""WITH {HB} SELECT school, id, state, class, enrollment, year, ROUND(took) AS "money in $", ROUND(spent) AS "spent $", ROUND(beyond) AS "beyond stipends $",
            ROUND(json_extract(l, '$[1]')) AS "gate $", ROUND(json_extract(l, '$[4]')) AS "boosters $", ROUND(fee) AS "fee $", crowd FROM hb ORDER BY beyond DESC""", open="e/high_schools/{id}",
  says=f"""WITH {HB} SELECT printf('%d schools: half spend under $%.0f beyond stipends (real typical $25-60K); the biggest spender, %s (%s), $%.0f.', COUNT(*),
     (SELECT beyond FROM hb ORDER BY beyond LIMIT 1 OFFSET (SELECT COUNT(*) / 2 FROM hb)), (SELECT school FROM hb ORDER BY beyond DESC LIMIT 1), (SELECT state FROM hb ORDER BY beyond DESC LIMIT 1), MAX(beyond)) FROM hb""")
panel("N32.lines", HP, "N32", "The 12 budget lines: the median school, this year's budget and last year's books", "table",
  "Each line of a high school program's books (finance/hs.rs LINES): the median and the top tenth of schools for last year's actual money and this year's budget.",
  """WITH v AS (SELECT j.key AS i, j.value AS v, 'last' AS w FROM finance__hs f, json_each(f.last__l) j UNION ALL SELECT j.key, j.value, 'budget' FROM finance__hs f, json_each(f.budget__l) j),
          r AS (SELECT i, w, v, ROW_NUMBER() OVER (PARTITION BY i, w ORDER BY v) AS rn, COUNT(*) OVER (PARTITION BY i, w) AS c FROM v)
     SELECT f.i + 1 AS line_no, f.name AS line, CASE f.income WHEN 1 THEN 'income' ELSE 'cost' END AS side,
            ROUND(MAX(CASE WHEN r.w = 'last' AND r.rn = (r.c + 1) / 2 THEN r.v END)) AS "median last year $", ROUND(MAX(CASE WHEN r.w = 'last' AND r.rn = (r.c * 9 + 9) / 10 THEN r.v END)) AS "top tenth last year $",
            ROUND(MAX(CASE WHEN r.w = 'budget' AND r.rn = (r.c + 1) / 2 THEN r.v END)) AS "median budget $", ROUND(AVG(CASE WHEN r.w = 'last' THEN r.v END)) AS "average last year $"
       FROM engine_finance_lines f JOIN r ON r.i = f.i WHERE f.level = 'hs' GROUP BY f.i ORDER BY f.i""")
panel("N32.spread", HP, "N32", "Spending beyond stipends: how schools spread", "hist",
  "Schools by what they spent beyond coaches' stipends last year, in $10,000 bins. Real: most public programs $25-60K; big Texas and Georgia programs $150K and more.",
  f"""WITH {HB} SELECT 'Schools' AS grp, (CAST(beyond / 10000 AS INTEGER) * 10) || 'K' AS x, COUNT(*) AS world, COUNT(*) AS n FROM hb GROUP BY CAST(beyond / 10000 AS INTEGER) ORDER BY CAST(beyond / 10000 AS INTEGER)""")
panel("N32.crowds", HP, "N32", "Crowds by state and school size", "map",
  "The average crowd a home game by state, schools' own figures (finance.hs crowd). Real: Texas and the Deep South draw the biggest crowds, big Texas programs 10,000 and more.",
  f"""WITH {HB} SELECT state, ROUND(AVG(crowd)) AS world, NULL AS real, COUNT(*) AS n, ROUND(AVG(beyond)) AS "spent beyond stipends $" FROM hb WHERE state IS NOT NULL GROUP BY state ORDER BY state""")
check("N32.median_beyond", HP, "N32", "Median spend beyond stipends", "$",
  f"WITH {HB} SELECT beyond FROM hb ORDER BY beyond LIMIT 1 OFFSET (SELECT COUNT(*) / 2 FROM hb)",
  Fixed(42500.0, "finance_check.rs: a typical public program spends $25-60K beyond stipends", "2020s"), (0.4, 0.8, 0.0), "The middle school's spend beyond coaches' stipends, last closed year.")
check("N32.texas_top", HP, "N32", "Biggest Texas program's spend beyond stipends", "$",
  f"WITH {HB} SELECT MAX(beyond) FROM hb WHERE state = 'TX'",
  Fixed(150000.0, "finance_check.rs: big Texas programs spend $150K and more", "2020s"), (0.3, 0.6, 0.0), "The Texas school spending the most beyond stipends.")

# ---------------------------------------------------------------- N33 betting lines
panel("N33.ats", MP, "N33", "Head coaches against the spread", "table",
  "Every head coach's record against the closing spread (finance.ats: covers, misses, pushes) with his job today. Real: over many games nobody stays far beyond 60%; most sit between 45% and 55%.",
  """SELECT c.name AS coach, a._key AS id, a._0 AS covers, a._1 AS misses, a._2 AS pushes, a._0 + a._1 + a._2 AS games, ROUND(100.0 * a._0 / NULLIF(a._0 + a._1, 0), 1) AS "cover %"
       FROM finance__ats a LEFT JOIN coaches c ON c.id = a._key ORDER BY games DESC""", open="e/coaches/{id}",
  says="""SELECT printf('%d coaches with 50+ games against the line: the best covers %.1f%%, the worst %.1f%%; %d beyond 60%% (real: hardly anyone).', COUNT(*), MAX(p), MIN(p), SUM(p > 60))
     FROM (SELECT 100.0 * _0 / (_0 + _1) AS p FROM finance__ats WHERE _0 + _1 + _2 >= 50)""")
panel("N33.spread", MP, "N33", "Coaches' cover rates: how they spread", "hist",
  "Coaches with 30 or more games against the line by cover rate in 2.5-point bins. A fair market keeps the spread narrow around 50%.",
  """SELECT 'Coaches (30+ games)' AS grp, CAST(100.0 * _0 / (_0 + _1) / 2.5 AS INTEGER) * 2.5 AS x, COUNT(*) AS world, COUNT(*) AS n FROM finance__ats WHERE _0 + _1 + _2 >= 30 GROUP BY x ORDER BY x""")
panel("N33.form", MP, "N33", "What the market has learned of each team", "table",
  "The betting market's running read of each team from its games against the line (finance.form: the sum of how far it beat the spread and how many games, faded each season). Keys decode to the team: college programs carry bit 20 (lines.rs form_key).",
  """SELECT f.year, CASE WHEN f._key >= 1048576 THEN (SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = f._key - 1048576) ELSE (SELECT name FROM nfl WHERE _row = f._key) END AS team,
            ROUND(f.n, 1) AS "games (faded)", ROUND(f.sum, 1) AS "beat the line by", ROUND(f.sum / NULLIF(f.n, 0), 2) AS "a game"
       FROM finance__form f ORDER BY ABS(f.sum / NULLIF(f.n, 0)) DESC""")
panel("N33.college", MP, "N33", "College lines: how good they are", "bars",
  "College favorites' win share by the size of the spread, from the game log's closing lines. Real (finance_check.rs): college lines miss the margin by about 12.5 points and favorites win about 75%.",
  """WITH ln AS (SELECT json_extract(g.line, '$[0]') AS sp, g.home_pts - g.away_pts AS m FROM log_games g WHERE g.stream = 'college' AND g.line IS NOT NULL AND COALESCE(g.standin, 0) = 0 AND g.home_pts IS NOT NULL)
     SELECT 'College' AS grp, CASE WHEN ABS(sp) < 3 THEN '0-2.5' WHEN ABS(sp) < 7 THEN '3-6.5' WHEN ABS(sp) < 14 THEN '7-13.5' WHEN ABS(sp) < 21 THEN '14-20.5' ELSE '21+' END AS label,
            ROUND(100.0 * SUM(CASE WHEN sp < 0 THEN m > 0 ELSE m < 0 END) / SUM(m <> 0), 1) AS world, NULL AS real, COUNT(*) AS n
       FROM ln WHERE sp <> 0 GROUP BY label ORDER BY MIN(ABS(sp))""",
  says="""WITH ln AS (SELECT json_extract(g.line, '$[0]') AS sp, g.home_pts - g.away_pts AS m FROM log_games g WHERE g.stream = 'college' AND g.line IS NOT NULL AND COALESCE(g.standin, 0) = 0 AND g.home_pts IS NOT NULL)
     SELECT printf('%d college games: lines miss the margin by %.1f points (real about 12.5); favorites won %.1f%% (real about 75%%).', COUNT(*), AVG(ABS(m + sp)),
       100.0 * SUM(CASE WHEN sp < 0 THEN m > 0 ELSE m < 0 END) / SUM(m <> 0 AND sp <> 0)) FROM ln""", needs=G)
check("N33.college_miss", MP, "N33", "College spread miss against the final margin", "points",
  "SELECT AVG(ABS(g.home_pts - g.away_pts + json_extract(g.line, '$[0]'))) FROM log_games g WHERE g.stream = 'college' AND g.line IS NOT NULL AND COALESCE(g.standin, 0) = 0 AND g.home_pts IS NOT NULL",
  Fixed(12.5, "finance_check.rs: college closing lines miss the margin by about 12.5 points", "2010s-2020s"), (0.1, 0.25, 0.0), "Mean absolute gap between the college closing spread and the final margin.", needs=G)
check("N33.college_favorites", MP, "N33", "College favorites who won", "%",
  """SELECT 100.0 * SUM(CASE WHEN json_extract(line, '$[0]') < 0 THEN home_pts > away_pts ELSE home_pts < away_pts END) / COUNT(*) FROM log_games
      WHERE stream = 'college' AND line IS NOT NULL AND json_extract(line, '$[0]') <> 0 AND home_pts <> away_pts AND COALESCE(standin, 0) = 0""",
  Fixed(75.0, "finance_check.rs: college favorites win about 75%", "2010s-2020s"), (0.05, 0.12, 0.0), "Share of college games the favorite won.", needs=G)
check("N33.nfl_favorites", MP, "N33", "NFL favorites who won", "%",
  """SELECT 100.0 * SUM(CASE WHEN json_extract(line, '$[0]') < 0 THEN home_pts > away_pts ELSE home_pts < away_pts END) / COUNT(*) FROM log_games
      WHERE stream = 'nfl' AND line IS NOT NULL AND json_extract(line, '$[0]') <> 0 AND home_pts <> away_pts AND COALESCE(kind, 'regular') <> 'preseason'""",
  Fixed(66.0, "finance_check.rs: NFL favorites win about 66%", "1999-2025"), (0.05, 0.12, 0.0), "Share of NFL games the favorite won.", needs=G)
check("N33.ats_best", MP, "N33", "Best long-run cover rate among coaches", "%",
  "SELECT MAX(100.0 * _0 / (_0 + _1)) FROM finance__ats WHERE _0 + _1 + _2 >= 100",
  Memory(60.0, "real: no head coach with 100+ games stays far beyond 60% against the spread (from memory)"), (0.05, 0.12, 0.0), "The best cover rate among coaches with 100 or more games.")
