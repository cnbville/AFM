from common import *

GP = "games"
HP = "high-school"
def part(e, i): return f"json_extract('[\"' || replace({e}, '|', '\",\"') || '\"]', '$[{i}]')"

# Every pro season, active players and alumni, with name and position.
PRO = """pro AS (SELECT s.year, s.team, s.games, s.starts, s.stats__targets, s.stats__rec, s.stats__rec_yds, s.stats__rush, s.stats__rush_yds, s.stats__pass_yds, s.stats__pass_epa, s.stats__dropbacks, s.stats__att, s.stats__cmp, s.stats__x_cmp, s.stats__sacks, s.stats__pass_td, s.stats__tackles, s.stats__ints, s.stats__rec_td, s.stats__rush_td, s.stats__grade_sum, s.stats__graded, p.name, p.player__id AS pid, p.player__position AS pos FROM people__pro__seasons s JOIN people p ON p._row = s._parent
          UNION ALL SELECT s.year, s.team, s.games, s.starts, s.stats__targets, s.stats__rec, s.stats__rec_yds, s.stats__rush, s.stats__rush_yds, s.stats__pass_yds, s.stats__pass_epa, s.stats__dropbacks, s.stats__att, s.stats__cmp, s.stats__x_cmp, s.stats__sacks, s.stats__pass_td, s.stats__tackles, s.stats__ints, s.stats__rec_td, s.stats__rush_td, s.stats__grade_sum, s.stats__graded, a.name, a.player__id, a.player__position FROM alumni__pro__seasons s JOIN alumni a ON a._row = s._parent)"""
for c_ in ["stats__targets", "stats__rec", "stats__rec_yds", "stats__rush", "stats__rush_yds", "stats__pass_yds", "stats__pass_epa", "stats__dropbacks", "stats__att",
           "stats__cmp", "stats__x_cmp", "stats__sacks", "stats__pass_td", "stats__tackles", "stats__ints", "stats__rec_td", "stats__rush_td", "stats__grade_sum", "stats__graded"]:
    col("people__pro__seasons", c_); col("alumni__pro__seasons", c_)

# ---------------------------------------------------------------- G25 NFL player seasons
NLEAD = [("stats__pass_yds", "Passing yards", "pass_yards"), ("stats__rush_yds", "Rushing yards", "rush_yards"), ("stats__rec_yds", "Receiving yards", "rec_yards"),
         ("stats__sacks", "Sacks", "sacks"), ("stats__pass_td", "Touchdown passes", None), ("stats__ints", "Interceptions", None)]
panel("G25.leaders", GP, "G25", "NFL season leaders from the pros' records", "table",
  "The top five each NFL season in each stat, from every pro's season record in the world file (complete history, regular season). Real: the season leaders of 2021-2025 (nflverse), shown beside each year's leader as their mean: passing 4,963, rushing 1,710, receiving 1,811, sacks 20.1.",
  f"""WITH {PRO}, r AS (""" + " UNION ALL ".join(
      f"""SELECT year, '{lab}' AS stat, {i} AS o, pid AS id, name, pos, team, games, {k} AS value, ROW_NUMBER() OVER (PARTITION BY year ORDER BY {k} DESC) AS rank,
                 {f"(SELECT AVG(value) FROM json_each({NGT}, '$.season_leaders.{rk}'))" if rk else "NULL"} AS real FROM pro WHERE {k} > 0"""
      for i, (k, lab, rk) in enumerate(NLEAD)) + f""")
     SELECT year, stat, rank, name, id, pos, {club('r.team', 'r.year')} AS club, games, ROUND(value, 1) AS value, CASE WHEN rank = 1 THEN ROUND(real) END AS "real leader"
       FROM r WHERE rank <= 5 ORDER BY year DESC, o, rank""",
  says=f"""WITH {PRO} SELECT printf('The NFL passing leader averages %.0f yards (real %.0f) and the sack leader %.1f (real 20.1) over %d seasons on record.',
     (SELECT AVG(m) FROM (SELECT MAX(stats__pass_yds) AS m FROM pro GROUP BY year)), (SELECT AVG(value) FROM json_each({NGT}, '$.season_leaders.pass_yards')),
     (SELECT AVG(m) FROM (SELECT MAX(stats__sacks) AS m FROM pro GROUP BY year)), (SELECT COUNT(DISTINCT year) FROM pro))""", open="e/people/{id}")

panel("G25.targets", GP, "G25", "Target shares: a team's top three and by position", "bars",
  "Each NFL team-season's targets (team-seasons the world holds whole, 450+ targets: older seasons keep only the players still on record): the share of the top, second and third target, and the shares by position (receivers, tight ends, backs). Real NFL 2021-2025 (engine_targets.json): top three 23.5%, 17.1%, 13.0%; WR 59.3%, TE 21.9%, RB 18.1%.",
  f"""WITH {PRO}, t0 AS (SELECT year, team, pos, stats__targets AS tg, ROW_NUMBER() OVER (PARTITION BY year, team ORDER BY stats__targets DESC) AS rk,
                         SUM(stats__targets) OVER (PARTITION BY year, team) AS tot FROM pro WHERE stats__targets > 0), t AS (SELECT * FROM t0 WHERE tot >= 450)
     SELECT 'Top three' AS grp, 'Target ' || rk AS label, ROUND(100.0 * AVG(1.0 * tg / tot), 1) AS world,
            CASE rk WHEN 1 THEN 23.5 WHEN 2 THEN 17.1 WHEN 3 THEN 13.0 END AS real, COUNT(*) AS n FROM t WHERE rk <= 3 GROUP BY rk
     UNION ALL
     SELECT 'By position', pos, ROUND(100.0 * SUM(tg) / (SELECT SUM(tg) FROM t), 1),
            ROUND(100 * json_extract((SELECT value_json FROM real_figures WHERE key = 'engine_targets.json:target_share_by_position'), '$.' || pos), 1), COUNT(*)
       FROM t WHERE pos IN ('WR', 'TE', 'RB') GROUP BY pos""")

panel("G25.rates", GP, "G25", "NFL players' rates: spread against real", "table",
  "Qualified NFL seasons (QBs 300+ dropbacks, backs 120+ carries): EPA per dropback, completion over expected and yards a carry, with mean, spread and the 10th and 90th percentiles. Real: nflverse 2021-2025 seasons with the same minimums (position_value.json).",
  f"""WITH {PRO}, m AS (SELECT 'EPA per dropback' AS metric, 1.0 * stats__pass_epa / stats__dropbacks AS v, 'qb_seasons_300_dropbacks.epa_per_dropback' AS rk FROM pro WHERE stats__dropbacks >= 300
             UNION ALL SELECT 'Completion % over expected', 100.0 * (stats__cmp - stats__x_cmp) / stats__att, 'qb_seasons_300_dropbacks.cpoe' FROM pro WHERE stats__dropbacks >= 300
             UNION ALL SELECT 'Yards a carry', 1.0 * stats__rush_yds / stats__rush, 'rb_seasons_120_carries' FROM pro WHERE stats__rush >= 120 AND pos = 'RB'),
       r AS (SELECT *, ROW_NUMBER() OVER (PARTITION BY metric ORDER BY v) AS rn, COUNT(*) OVER (PARTITION BY metric) AS cnt FROM m)
     SELECT metric, COUNT(*) AS seasons, ROUND(AVG(v), 3) AS mean, ROUND(sqrt(AVG(v * v) - AVG(v) * AVG(v)), 3) AS spread,
            ROUND(MIN(CASE WHEN rn >= 0.1 * cnt THEN v END), 3) AS p10, ROUND(MIN(CASE WHEN rn >= 0.9 * cnt THEN v END), 3) AS p90,
            ROUND(json_extract((SELECT value_json FROM real_figures WHERE key = 'position_value.json:' || r.rk), CASE WHEN metric = 'Yards a carry' THEN '$.yards_per_carry.mean' ELSE '$.mean' END), 3) AS "real mean",
            ROUND(json_extract((SELECT value_json FROM real_figures WHERE key = 'position_value.json:' || r.rk), CASE WHEN metric = 'Yards a carry' THEN '$.yards_per_carry.sd' ELSE '$.sd' END), 3) AS "real spread"
       FROM r GROUP BY metric, rk""")

check("G25.qb_epa_spread", GP, "G25", "Spread of NFL QB EPA per dropback (300+ dropbacks)", "EPA",
  f"WITH {PRO}, m AS (SELECT 1.0 * stats__pass_epa / stats__dropbacks AS v FROM pro WHERE stats__dropbacks >= 300) SELECT sqrt(AVG(v * v) - AVG(v) * AVG(v)) FROM m",
  Key("position_value.json:qb_seasons_300_dropbacks.epa_per_dropback#sd"), (0.15, 0.4, 0.0), "Standard deviation of EPA per dropback across qualified NFL QB seasons in the world.")
check("G25.qb_epa_mean", GP, "G25", "NFL QB EPA per dropback, qualified seasons", "EPA",
  f"WITH {PRO} SELECT AVG(1.0 * stats__pass_epa / stats__dropbacks) FROM pro WHERE stats__dropbacks >= 300",
  Key("position_value.json:qb_seasons_300_dropbacks.epa_per_dropback#mean"), (0.5, 1.2, 0.04), "Mean EPA per dropback of qualified NFL QB seasons.")
check("G25.ypc_spread", GP, "G25", "Spread of NFL RB yards a carry (120+ carries)", "yards",
  f"WITH {PRO}, m AS (SELECT 1.0 * stats__rush_yds / stats__rush AS v FROM pro WHERE stats__rush >= 120 AND pos = 'RB') SELECT sqrt(AVG(v * v) - AVG(v) * AVG(v)) FROM m",
  Key("position_value.json:rb_seasons_120_carries#yards_per_carry.sd"), (0.2, 0.5, 0.0), "Standard deviation of yards a carry across qualified NFL running back seasons.")
check("G25.top_target", GP, "G25", "NFL team's top target share", "%",
  f"""WITH {PRO}, t AS (SELECT 1.0 * stats__targets / SUM(stats__targets) OVER (PARTITION BY year, team) AS sh, SUM(stats__targets) OVER (PARTITION BY year, team) AS tot,
                ROW_NUMBER() OVER (PARTITION BY year, team ORDER BY stats__targets DESC) AS rk FROM pro WHERE stats__targets > 0) SELECT 100.0 * AVG(sh) FROM t WHERE rk = 1 AND tot >= 450""",
  Fixed(23.5, "engine_targets.json target_share_top3_season (nflverse 2021-2025)", "2021-2025"), (0.1, 0.25, 0.0), "Mean share of a team-season's targets that went to its most-targeted player (team-seasons held whole, 450+ targets).")
check("G25.wr_target_share", GP, "G25", "NFL targets to wide receivers", "%",
  f"WITH {PRO} SELECT 100.0 * SUM(CASE WHEN pos = 'WR' THEN stats__targets END) / SUM(stats__targets) FROM pro WHERE pos IN ('WR', 'TE', 'RB')",
  Key("engine_targets.json:target_share_by_position#WR", 100.0), (0.06, 0.15, 0.0), "Share of NFL targets to WRs (of WR, TE and RB targets).")

# ---------------------------------------------------------------- G26 NFL honors and records
panel("G26.awards", GP, "G26", "NFL awards by year", "table",
  "MVP, Offensive and Defensive Player and Rookie of the Year, Comeback Player and Super Bowl MVP, each with the winner's position and club. Real: QBs win most MVPs (21 of 25 in 2000-2024, from memory).",
  f"""SELECT a._0 AS year, a._1 AS award, a._3 AS name, a._2 AS id, COALESCE((SELECT player__position FROM people WHERE player__id = a._2), (SELECT player__position FROM alumni WHERE player__id = a._2)) AS pos,
            {club('a._4', 'a._0')} AS club FROM nfl_book__awards a ORDER BY a._0 DESC, a._1""",
  says="""SELECT printf('%d MVPs on record, %d of them quarterbacks.', COUNT(*), SUM(COALESCE((SELECT player__position FROM people WHERE player__id = a._2), (SELECT player__position FROM alumni WHERE player__id = a._2)) = 'QB'))
     FROM nfl_book__awards a WHERE a._1 = 'MVP'""", open="e/people/{id}")
panel("G26.hall", GP, "G26", "Pro Football Hall of Fame: classes and position mix", "bars",
  "Inductees by position, and this year's leading candidates' scores. Trial 10 had 3 WRs against 23 interior linemen among 138, where the real Hall has many receivers; real classes are 5-8 a year.",
  """SELECT 'Inducted' AS grp, _3 AS label, COUNT(*) AS world, NULL AS real, COUNT(*) AS n FROM nfl_book__hall GROUP BY _3
     UNION ALL SELECT 'Candidates', pos, COUNT(*), NULL, COUNT(*) FROM nfl_book__candidates GROUP BY pos ORDER BY 1, 3 DESC""")
panel("G26.hall_list", GP, "G26", "Hall of Fame inductees and candidates", "table",
  "Every inductee with the case for him, and the candidates still waiting with their ballot score.",
  """SELECT _0 AS year, 'inducted' AS status, _2 AS name, _1 AS id, _3 AS pos, _4 AS why, NULL AS score FROM nfl_book__hall
     UNION ALL SELECT retired, 'candidate', name, id, pos, why, ROUND(score, 1) FROM nfl_book__candidates ORDER BY status DESC, year DESC""", open="e/people/{id}")
panel("G26.records", GP, "G26", "The NFL record book and records broken", "table",
  "Each record: the stat, single season or career, the value, the holder and year, and whether a world player holds it (the book is seeded with real marks before 1984, from memory, to verify). Below, every record broken.",
  """SELECT stat, CASE WHEN career THEN 'career' ELSE 'season' END AS book, ROUND(value, 1) AS value, holder, year, CASE WHEN world THEN 'world' ELSE 'real (seeded)' END AS holder_from FROM nfl_book__records
     UNION ALL SELECT _1, 'broken', ROUND(_3, 1), _2, _0, 'world' FROM nfl_book__broken ORDER BY 2, 1, 5""")
panel("G26.all_pro", GP, "G26", "All-Pro and Pro Bowl counts by position", "bars",
  "First and second-team All-Pro picks and Pro Bowl selections, by position, all years on record.",
  """SELECT 'All-Pro, team ' || _1 AS grp, _3 AS label, COUNT(*) AS world, NULL AS real, COUNT(*) AS n FROM nfl_book__all_pro GROUP BY _1, _3
     UNION ALL SELECT 'Pro Bowl', COALESCE((SELECT player__position FROM people WHERE player__id = b._1), (SELECT player__position FROM alumni WHERE player__id = b._1), '?'), COUNT(*), NULL, COUNT(*)
       FROM nfl_book__pro_bowls b GROUP BY 2 ORDER BY 1, 3 DESC""")
check("G26.mvp_qbs", GP, "G26", "NFL MVPs who were quarterbacks", "%",
  """SELECT 100.0 * SUM(COALESCE((SELECT player__position FROM people WHERE player__id = a._2), (SELECT player__position FROM alumni WHERE player__id = a._2)) = 'QB') / COUNT(*) FROM nfl_book__awards a WHERE a._1 = 'MVP'""",
  Memory(84.0, "21 of the 25 NFL MVPs 2000-2024 were quarterbacks; Faulk, Alexander, Tomlinson and Peterson were not (from memory)"), (0.15, 0.35, 0.0), "Share of world MVPs at quarterback.")
check("G26.hall_class", GP, "G26", "Pro Football Hall of Fame class size", "a class",
  "SELECT 1.0 * COUNT(*) / COUNT(DISTINCT _0) FROM nfl_book__hall", Memory(6.5, "real modern-era classes run 5-8 a year (from memory)"), (0.3, 0.6, 0.0),
  "Inductees per class year.")
check("G26.hall_receivers", GP, "G26", "Receivers among Hall of Fame inductees", "%",
  "SELECT 100.0 * SUM(_3 IN ('WR', 'TE')) / COUNT(*) FROM nfl_book__hall", Memory(17.0, "about 1 in 6 modern-era inductees are receivers or tight ends (from memory)"), (0.3, 0.6, 3.0),
  "Share of inductees at WR or TE.")

# ---------------------------------------------------------------- G27 high school lines and state books
HSL = """hsl AS (SELECT l.year, l.school, l.pos, l.games, l.s, p.name, p.player__id AS id FROM people__hs_lines l JOIN people p ON p._row = l._parent
          UNION ALL SELECT l.year, l.school, l.pos, l.games, l.s, a.name, a.player__id FROM alumni__hs_lines l JOIN alumni a ON a._row = l._parent)"""
for c_ in ["_parent", "year", "school", "pos", "games", "s"]:
    col("alumni__hs_lines", c_)
def HS(i): return f"COALESCE(json_extract(hsl.s, '$[{i}]'), 0)"
HSTOP = [("Passing yards", HS(4)), ("Rushing yards", HS(8)), ("Rushing TDs", HS(9)), ("Receiving yards", HS(11)), ("Tackles", HS(13)), ("Sacks", f"{HS(15)} / 2.0"), ("Interceptions", HS(16))]
panel("G27.tops", HP, "G27", "High school season bests against real", "table",
  "The best high school seasons in each stat, from every kept high school line (engine games 2014 on; players who left football are not kept). Real, from memory (world trial 10 notes; real high school books need a source): receiving seasons over 1,500 yards are common, the sack record is near 40 and interception seasons reach 10-15.",
  # The top ten of each stat by a bounded sort, not a ranking of every line.
  f"""WITH {HSL}, r AS (""" + " UNION ALL ".join(
      f"SELECT * FROM (SELECT '{lab}' AS stat, {i} AS o, year, id, name, pos, school, games, {ex} AS value FROM hsl ORDER BY value DESC LIMIT 10)"
      for i, (lab, ex) in enumerate(HSTOP)) + f"""), rk AS (SELECT r.*, ROW_NUMBER() OVER (PARTITION BY o ORDER BY value DESC) AS rank FROM r)
     SELECT stat, rank, value, name, id, pos, year, (SELECT COALESCE(name, fictional_name) || ' (' || (SELECT code FROM states WHERE _row = h.state) || ')' FROM high_schools h WHERE h.id = rk.school) AS school, games
       FROM rk ORDER BY o, rank""",
  says=f"""WITH {HSL} SELECT printf('%d high school seasons kept; the best sack season is %.1f (real record near 40) and %d seasons have 30 or more sacks.', COUNT(*), MAX({HS(15)} / 2.0), SUM({HS(15)} >= 60)) FROM hsl""", open="e/people/{id}")
panel("G27.spread", HP, "G27", "High school season lines: how they spread", "hist",
  "Kept high school seasons of players with 5+ games, in buckets: passing yards (QBs), rushing yards (RBs), receiving yards (WRs) and sacks (edge rushers).",
  f"""WITH {HSL}, x AS (SELECT 'QB passing yards' AS grp, 250 * CAST({HS(4)} / 250 AS INTEGER) AS v FROM hsl WHERE pos = 'QB' AND games >= 5
             UNION ALL SELECT 'RB rushing yards', 250 * CAST({HS(8)} / 250 AS INTEGER) FROM hsl WHERE pos = 'RB' AND games >= 5
             UNION ALL SELECT 'WR receiving yards', 250 * CAST({HS(11)} / 250 AS INTEGER) FROM hsl WHERE pos = 'WR' AND games >= 5
             UNION ALL SELECT 'EDGE sacks', CAST({HS(15)} / 2 AS INTEGER) FROM hsl WHERE pos = 'EDGE' AND games >= 5)
     SELECT grp, v AS x, 100.0 * COUNT(*) / SUM(COUNT(*)) OVER (PARTITION BY grp) AS world, COUNT(*) AS n FROM x GROUP BY grp, v ORDER BY grp, v""")
check("G27.top_sacks", HP, "G27", "Best high school sack season", "sacks",
  f"WITH {HSL} SELECT MAX({HS(15)} / 2.0) FROM hsl", Memory(40.0, "the real national high school season sack record is near 40 (from memory, world-trial10.md)"), (0.3, 0.8, 0.0),
  "The most sacks in one kept high school season.")
check("G27.top_receiving", HP, "G27", "Best high school receiving season", "yards",
  f"WITH {HSL} SELECT MAX({HS(11)}) FROM hsl", Memory(2500.0, "real state leaders pass 1,500 receiving yards every year and the top seasons reach about 2,500 (from memory)"), (0.25, 0.5, 0.0),
  "The most receiving yards in one kept high school season.")

# ---------------------------------------------------------------- G28 snaps and usage
SN = """sn AS (SELECT g._row AS gid, g.season AS year, CASE WHEN g.stream = 'nfl' THEN 'NFL' WHEN g.stream = 'college' THEN json_extract(g.divisions, '$[' || p._i || ']') ELSE g.stream END AS lv,
                 p._i AS side, p.id, p.pos, COALESCE(p.snaps_off, 0) AS off, COALESCE(p.snaps_def, 0) AS def, COALESCE(p.rush, 0) AS car, COALESCE(p.targets, 0) AS tg
            FROM log_games__players p JOIN log_games g ON g._row = p._parent WHERE g.stream IN ('nfl', 'college') AND COALESCE(g.kind, 'regular') = 'regular'),
       tm AS (SELECT gid, side, MAX(off) AS toff, MAX(def) AS tdef FROM sn GROUP BY gid, side),
       sh AS (SELECT sn.*, CASE WHEN sn.pos IN ('QB', 'RB', 'WR', 'TE', 'OT', 'IOL') THEN 1.0 * off / NULLIF(MAX(off) OVER w, 0) ELSE 1.0 * def / NULLIF(MAX(def) OVER w, 0) END AS share
            FROM sn WINDOW w AS (PARTITION BY gid, side)),
       rk AS (SELECT sh.*, ROW_NUMBER() OVER (PARTITION BY gid, side, CASE WHEN pos IN ('OT', 'IOL') THEN 'OL' ELSE pos END ORDER BY share DESC) AS rank FROM sh)"""
SNR = [("QB", "QB", 1), ("RB", "RB", 3), ("WR", "WR", 5), ("TE", "TE", 3), ("OL", "OL", 7), ("EDGE", "EDGE", 5), ("DT", "DL_interior", 4), ("LB", "LB_off_ball", 4), ("CB", "CB", 5), ("S", "S", 4)]
panel("G28.shares", GP, "G28", "Snap share by depth: starters and rotation against real", "bars",
  "Average share of the team's snaps for the first, second and later man at each position (by snaps that game), NFL and FBS regular-season games. Real NFL 2021-2025 (engine_targets.json snap_shares): QB1 96%, RB1 65% and RB2 32%, the top two corners and safeties near every snap, edge and interior linemen in a rotation.",
  f"""WITH {SN} SELECT lv AS grp, CASE WHEN pos IN ('OT', 'IOL') THEN 'OL' ELSE pos END || rank AS label, ROUND(100 * AVG(share), 1) AS world,
            ROUND(100 * json_extract((SELECT value_json FROM real_figures WHERE key = 'engine_targets.json:snap_shares.' ||
                CASE CASE WHEN pos IN ('OT', 'IOL') THEN 'OL' ELSE pos END {" ".join(f"WHEN '{a}' THEN '{b}'" for a, b, _ in SNR)} END), '$.rank_mean[' || (rank - 1) || ']'), 1) AS real,
            COUNT(*) AS n
       FROM rk WHERE lv IN ('NFL', 'FBS') AND rank <= CASE CASE WHEN pos IN ('OT', 'IOL') THEN 'OL' ELSE pos END {" ".join(f"WHEN '{a}' THEN {k}" for a, _, k in SNR)} ELSE 0 END
      GROUP BY lv, CASE WHEN pos IN ('OT', 'IOL') THEN 'OL' ELSE pos END, rank
      ORDER BY lv, CASE CASE WHEN pos IN ('OT', 'IOL') THEN 'OL' ELSE pos END {" ".join(f"WHEN '{a}' THEN {i}" for i, (a, _, _k) in enumerate(SNR))} END, rank""", needs=G)
panel("G28.carries", GP, "G28", "Lead back's share of running back carries a game", "hist",
  "Each team game's top running back's share of the backs' carries, in 5-point buckets. Real NFL (engine_targets.json rb_carry_share): mean 70%, 10th percentile 50%, 90th 92%.",
  f"""WITH {SN}, c AS (SELECT lv, gid, side, MAX(car) AS top, SUM(car) AS tot FROM sn WHERE pos = 'RB' GROUP BY lv, gid, side HAVING SUM(car) >= 5)
     SELECT lv AS grp, 5 * CAST(20.0 * top / tot AS INTEGER) AS x, 100.0 * COUNT(*) / SUM(COUNT(*)) OVER (PARTITION BY lv) AS world, COUNT(*) AS n
       FROM c WHERE lv IN ('NFL', 'FBS') GROUP BY lv, x ORDER BY lv, x""", needs=G)
panel("G28.emergency", GP, "G28", "Emergency snaps: men playing out of their position group", "line",
  "Share of team games with any emergency snaps (a player from another group forced in after injuries or a thin depth chart), by level and season. Trial 10: 1,233 of 143,682 college games.",
  f"""WITH b AS (SELECT g.season AS year, CASE WHEN g.stream = 'nfl' THEN 'NFL' WHEN g.stream = 'college' THEN json_extract(g.divisions, '$[' || b._i || ']') WHEN g.stream = 'hs' THEN 'High school' ELSE COALESCE(g.league, 'Other') END AS lv,
                    COALESCE(b.emergency_snaps, 0) AS e FROM log_games__box b JOIN log_games g ON g._row = b._parent)
     SELECT lv AS grp, year, ROUND(100.0 * SUM(e > 0) / COUNT(*), 2) AS world, NULL AS real, COUNT(*) AS n FROM b GROUP BY lv, year ORDER BY lv, year""", needs=G)
col("log_games__box", "emergency_snaps")
for c_ in ["snaps_off", "snaps_def", "rush", "targets", "pos"]:
    col("log_games__players", c_)
check("G28.qb1_share", GP, "G28", "NFL starting QB's share of snaps", "%",
  f"WITH {SN} SELECT 100 * AVG(share) FROM rk WHERE lv = 'NFL' AND pos = 'QB' AND rank = 1",
  Key("engine_targets.json:snap_shares.QB#rank_mean.0", 100.0), (0.03, 0.08, 0.0), "Mean share of team offensive snaps for the QB who played most, NFL regular-season games.", needs=G)
check("G28.rb1_share", GP, "G28", "NFL lead back's share of snaps", "%",
  f"WITH {SN} SELECT 100 * AVG(share) FROM rk WHERE lv = 'NFL' AND pos = 'RB' AND rank = 1",
  Key("engine_targets.json:snap_shares.RB#rank_mean.0", 100.0), (0.1, 0.25, 0.0), "Mean share of team offensive snaps for the back who played most.", needs=G)
check("G28.lead_back_carries", GP, "G28", "NFL lead back's share of RB carries a game", "%",
  f"WITH {SN}, c AS (SELECT gid, side, MAX(car) AS top, SUM(car) AS tot FROM sn WHERE pos = 'RB' AND lv = 'NFL' GROUP BY gid, side HAVING SUM(car) >= 5) SELECT 100.0 * AVG(1.0 * top / tot) FROM c",
  Key("engine_targets.json:rb_carry_share.game_lead_back#mean", 100.0), (0.08, 0.2, 0.0), "Mean share of the backs' carries taken by the top back, NFL team games.", needs=G)
check("G28.cb2_share", GP, "G28", "NFL second corner's share of snaps", "%",
  f"WITH {SN} SELECT 100 * AVG(share) FROM rk WHERE lv = 'NFL' AND pos = 'CB' AND rank = 2",
  Key("engine_targets.json:snap_shares.CB#rank_mean.1", 100.0), (0.06, 0.15, 0.0), "Mean share of team defensive snaps for the second corner.", needs=G)

# ---------------------------------------------------------------- G29 grades
GR = """gr AS (SELECT g.season AS year, CASE WHEN g.stream = 'nfl' THEN 'NFL' WHEN g.stream = 'college' THEN json_extract(g.divisions, '$[' || p._i || ']') ELSE g.stream END AS lv,
                 p.id, p.pos, SUM(COALESCE(p.grade_sum, 0)) AS gs, SUM(COALESCE(p.graded, 0)) AS gp FROM log_games__players p JOIN log_games g ON g._row = p._parent
            WHERE g.stream IN ('nfl', 'college') GROUP BY 1, 2, 3, 4 HAVING SUM(COALESCE(p.graded, 0)) >= 200)"""
panel("G29.by_pos", GP, "G29", "Player grades a play by position", "table",
  "Grade a graded play, per player season with 200+ graded plays, by level and position: mean, spread and the 10th and 90th percentiles. The grade sums each play's credit and blame (a penalty -0.8, a sack taken -0.4 and so on, GameStats::book); spreads should be alike across positions so awards and the Hall ballot treat them fairly.",
  f"""WITH {GR}, r AS (SELECT lv, pos, 1.0 * gs / gp AS v, ROW_NUMBER() OVER (PARTITION BY lv, pos ORDER BY 1.0 * gs / gp) AS rn, COUNT(*) OVER (PARTITION BY lv, pos) AS cnt FROM gr)
     SELECT lv AS grp, pos, COUNT(*) AS players, ROUND(AVG(v), 4) AS mean, ROUND(sqrt(AVG(v * v) - AVG(v) * AVG(v)), 4) AS spread,
            ROUND(MIN(CASE WHEN rn >= 0.1 * cnt THEN v END), 4) AS p10, ROUND(MIN(CASE WHEN rn >= 0.9 * cnt THEN v END), 4) AS p90
       FROM r GROUP BY lv, pos ORDER BY lv, pos""", needs=G)
panel("G29.top", GP, "G29", "Top-graded player seasons", "table",
  "The five best grades a play at each position per level and season, from the game log, with games and plays graded.",
  f"""WITH {GR}, r AS (SELECT gr.*, ROW_NUMBER() OVER (PARTITION BY year, lv, pos ORDER BY 1.0 * gs / gp DESC) AS rank FROM gr)
     SELECT year, lv AS level, pos, rank, {pname('r.id')} AS name, id, gp AS "plays graded", ROUND(1.0 * gs / gp, 4) AS "grade a play"
       FROM r WHERE rank <= 5 ORDER BY year DESC, lv, pos, rank""", needs=G, open="e/people/{id}")
check("G29.spread_ratio", GP, "G29", "Grade spread: widest position over narrowest, NFL", "ratio",
  f"""WITH {GR}, s AS (SELECT pos, sqrt(AVG((1.0 * gs / gp) * (1.0 * gs / gp)) - AVG(1.0 * gs / gp) * AVG(1.0 * gs / gp)) AS sd FROM gr WHERE lv = 'NFL' GROUP BY pos HAVING COUNT(*) >= 20)
     SELECT MAX(sd) / MIN(sd) FROM s""",
  NoReal("a fairness rule, not real data: grade spreads should be alike across positions, within about 2 to 1"), (0.5, 1.0, 0.5),
  "The widest position's grade spread over the narrowest, NFL player seasons with 200+ graded plays.", needs=G)

# ---------------------------------------------------------------- G30 weather
WX = f"""wx AS (SELECT g._row AS gid, g.season AS year, g.stream, COALESCE(g.conditions__temperature, g.weather__temperature) AS temp, COALESCE(g.conditions__wind, g.weather__wind) AS wind,
                 COALESCE(g.conditions__rain, g.weather__rain, 0) AS rain, COALESCE(g.conditions__snow, g.weather__snow, 0) AS snow,
                 COALESCE(g.weather__indoor, g.indoor, g.site__indoor, 0) AS indoor FROM log_games g WHERE {ENGINE})"""
for c_ in ["indoor", "site__indoor"]:
    col("log_games", c_)
TB = "(CASE WHEN temp < 32 THEN 'under_32' WHEN temp < 50 THEN '32_49' WHEN temp < 70 THEN '50_69' ELSE '70_plus' END)"
WB = "(CASE WHEN wind < 5 THEN '0_4' WHEN wind < 10 THEN '5_9' WHEN wind < 15 THEN '10_14' ELSE '15_plus' END)"
panel("G30.splits", GP, "G30", "Passing and kicking by temperature and wind against real", "table",
  "NFL outdoor games split by game-time temperature and wind: completion %, yards an attempt, interception rate and field goal %, from the box scores. Real: nflverse 1999-2025, outdoor and open-roof games with a reading (nfl_history.json weather_splits). Cold and wind should cost a little, not a lot.",
  f"""WITH {WX}, b AS (SELECT wx.*, b.* FROM wx JOIN log_games__box b ON b._parent = wx.gid WHERE wx.stream = 'nfl' AND NOT wx.indoor AND wx.temp IS NOT NULL),
       x AS (SELECT 'Temperature' AS split, {TB} AS bucket, 'by_temp_f' AS rk, * FROM b UNION ALL SELECT 'Wind', {WB}, 'by_wind_mph', * FROM b)
     SELECT split, bucket, SUM(att) AS attempts, ROUND(100.0 * SUM(cmp) / SUM(att), 1) AS "completion %",
            ROUND(100 * json_extract({NH}, '$.weather_splits.' || MAX(rk) || '.' || bucket || '.completion_pct'), 1) AS "real completion %",
            ROUND(1.0 * SUM(pass_yards + COALESCE(sack_yds, 0)) / SUM(att), 2) AS "yards an attempt", ROUND(json_extract({NH}, '$.weather_splits.' || MAX(rk) || '.' || bucket || '.yards_per_attempt'), 2) AS "real yards an attempt",
            ROUND(100.0 * SUM(ints) / SUM(att), 2) AS "int %", ROUND(100 * json_extract({NH}, '$.weather_splits.' || MAX(rk) || '.' || bucket || '.int_rate'), 2) AS "real int %",
            ROUND(100.0 * SUM(fg_made) / NULLIF(SUM(fg_att), 0), 1) AS "FG %", ROUND(100 * json_extract({NH}, '$.weather_splits.' || MAX(rk) || '.' || bucket || '.fg_pct'), 1) AS "real FG %"
       FROM x GROUP BY split, bucket ORDER BY split, bucket""", needs=G)
panel("G30.years", GP, "G30", "Weather each year: snow, rain, cold, lightning, moves and heat", "table",
  "The world's yearly weather counts for college games, and high school heat-limited practice days (the calendar's history row).",
  """SELECT year, games, snow_games AS snow, rain_games AS rain, freezing_games AS freezing, lightning_delays AS "lightning delays", weather_moves AS moved, weather_cancels AS cancelled,
            heat_limited_days AS "heat-limited days" FROM log_history ORDER BY year""", needs=L)
panel("G30.weather_mix", GP, "G30", "Game-day conditions by level", "bars",
  "Share of engine games played indoors, in rain, in snow, below freezing and in 15+ mph wind, by level.",
  f"""WITH {WX}, l AS (SELECT CASE stream WHEN 'nfl' THEN 'NFL' WHEN 'college' THEN 'College' WHEN 'hs' THEN 'High school' ELSE 'Other' END AS lv, * FROM wx)
     SELECT lv AS grp, 'Indoors' AS label, ROUND(100.0 * SUM(indoor) / COUNT(*), 1) AS world, NULL AS real, COUNT(*) AS n FROM l GROUP BY lv
     UNION ALL SELECT lv, 'Rain', ROUND(100.0 * SUM(rain > 0 AND NOT indoor) / COUNT(*), 1), NULL, COUNT(*) FROM l GROUP BY lv
     UNION ALL SELECT lv, 'Snow', ROUND(100.0 * SUM(snow > 0 AND NOT indoor) / COUNT(*), 1), NULL, COUNT(*) FROM l GROUP BY lv
     UNION ALL SELECT lv, 'Below 32°F', ROUND(100.0 * SUM(temp < 32 AND NOT indoor) / COUNT(*), 1), NULL, COUNT(*) FROM l GROUP BY lv
     UNION ALL SELECT lv, 'Wind 15+ mph', ROUND(100.0 * SUM(wind >= 15 AND NOT indoor) / COUNT(*), 1), NULL, COUNT(*) FROM l GROUP BY lv ORDER BY 1""", needs=G)
for c_ in ["snow_games", "rain_games", "freezing_games", "lightning_delays", "weather_moves", "weather_cancels", "heat_limited_days", "games"]:
    col("log_history", c_)
check("G30.nfl_cold_completion", GP, "G30", "NFL completion % below 32°F", "%",
  f"WITH {WX} SELECT 100.0 * SUM(b.cmp) / SUM(b.att) FROM wx JOIN log_games__box b ON b._parent = wx.gid WHERE wx.stream = 'nfl' AND NOT wx.indoor AND wx.temp < 32",
  Key("nfl_history.json:weather_splits.by_temp_f#under_32.completion_pct", 100.0), (0.05, 0.12, 0.0), "Completions over attempts in NFL games below freezing.", needs=G)
check("G30.nfl_wind_fg", GP, "G30", "NFL field goal % in 15+ mph wind", "%",
  f"WITH {WX} SELECT 100.0 * SUM(b.fg_made) / SUM(b.fg_att) FROM wx JOIN log_games__box b ON b._parent = wx.gid WHERE wx.stream = 'nfl' AND NOT wx.indoor AND wx.wind >= 15",
  Key("nfl_history.json:weather_splits.by_wind_mph#15_plus.fg_pct", 100.0), (0.06, 0.15, 0.0), "Field goals made over tried in NFL games with wind of 15 mph or more.", needs=G)
check("G30.nfl_wind_ypa", GP, "G30", "NFL yards an attempt in 15+ mph wind", "yards",
  f"WITH {WX} SELECT 1.0 * SUM(b.pass_yards + COALESCE(b.sack_yds, 0)) / SUM(b.att) FROM wx JOIN log_games__box b ON b._parent = wx.gid WHERE wx.stream = 'nfl' AND NOT wx.indoor AND wx.wind >= 15",
  Key("nfl_history.json:weather_splits.by_wind_mph#15_plus.yards_per_attempt"), (0.06, 0.15, 0.0), "Gross passing yards over attempts in windy NFL games.", needs=G)

# ---------------------------------------------------------------- G31 game context
panel("G31.tags", GP, "G31", "Why games mattered: tags by year", "line",
  "College games carrying each tag (rivalry, revenge, a coach's return, hometown, milestone, ranked matchup, playoff spot, job on the line) per season. Trial 10: hometown 34,272, job on the line 26,161, revenge 9,428, rivalry 2,038 over 31 years.",
  """SELECT j.value AS grp, g.season AS year, COUNT(*) AS world, NULL AS real, COUNT(*) AS n FROM log_games g, json_each(g.tags) j
      WHERE g.stream = 'college' AND g.tags IS NOT NULL GROUP BY j.value, g.season ORDER BY 1, 2""", needs=G)
panel("G31.loud", GP, "G31", "How loud games were, and upsets by loudness", "table",
  "College and NFL games by loudness (small, medium, big, huge), with the favorite's win share where a line was set. Huge games are meant to be rare: 1-3 a team season at most, some seasons none.",
  f"""WITH x AS (SELECT CASE WHEN stream = 'nfl' THEN 'NFL' ELSE 'College' END AS lv, COALESCE(loud, 'none') AS loud, json_extract(line, '$[0]') AS sp, home_pts - away_pts AS m
               FROM log_games g WHERE {ENGINE} AND stream IN ('college', 'nfl'))
     SELECT lv AS grp, loud, COUNT(*) AS games, SUM(sp IS NOT NULL AND sp <> 0) AS "with a line",
            ROUND(100.0 * SUM(CASE WHEN sp < 0 THEN m > 0 WHEN sp > 0 THEN m < 0 END) / NULLIF(SUM(sp IS NOT NULL AND sp <> 0), 0), 1) AS "favorite won %"
       FROM x GROUP BY lv, loud ORDER BY lv, CASE loud WHEN 'huge' THEN 1 WHEN 'big' THEN 2 WHEN 'medium' THEN 3 WHEN 'small' THEN 4 ELSE 5 END""", needs=G)
panel("G31.games", GP, "G31", "The loudest games of the latest season and why", "table",
  "Every big or huge game of the latest season, with its tags and the plain-words reasons, kickoff, attendance and the result.",
  f"""SELECT g.season AS year, g._row AS game, g.date, CASE WHEN g.stream = 'nfl' THEN {club('g.home', 'g.season')} ELSE (SELECT {prog('p')} FROM programs p WHERE p._row = g.home) END AS home,
            CASE WHEN g.stream = 'nfl' THEN {club('g.away', 'g.season')} ELSE (SELECT {prog('p')} FROM programs p WHERE p._row = g.away) END AS away,
            g.home_pts || '-' || g.away_pts AS score, g.loud, g.tags, g.why, g.kickoff, g.attendance
       FROM log_games g WHERE g.loud IN ('big', 'huge') AND g.season = (SELECT MAX(season) FROM log_games) ORDER BY g.loud DESC, g.date""", needs=G, open="game/{game}")
check("G31.huge_share", GP, "G31", "College games that were huge for at least one side", "%",
  "SELECT 100.0 * SUM(huge_for LIKE '%true%') / COUNT(*) FROM log_games WHERE stream = 'college'",
  NoReal("a design target, not real data: 1-3 huge games a team season, some none (build_data.py huge_share_1_to_3)"), (0.5, 1.0, 2.0),
  "Share of college games marked huge for the home or the away side.", needs=G)

# ---------------------------------------------------------------- G32 lines
LN = f"""ln AS (SELECT CASE WHEN g.stream = 'nfl' THEN 'NFL' ELSE 'College' END AS lv, g.season AS year, json_extract(g.line, '$[0]') AS sp, json_extract(g.line, '$[1]') AS total,
                 g.home_pts - g.away_pts AS m, g.home_pts + g.away_pts AS pts, COALESCE(g.neutral, 0) AS neutral
            FROM log_games g WHERE {ENGINE} AND g.line IS NOT NULL AND g.stream IN ('nfl', 'college') AND COALESCE(g.kind, 'regular') <> 'preseason')"""
SPB = "(CASE WHEN ABS(sp) < 3 THEN '0_2.5' WHEN ABS(sp) < 4 THEN '3_3.5' WHEN ABS(sp) < 7 THEN '4_6.5' WHEN ABS(sp) < 10 THEN '7_9.5' WHEN ABS(sp) < 14 THEN '10_13.5' ELSE '14_plus' END)"
panel("G32.calibration", GP, "G32", "Favorites' win share by the size of the spread", "bars",
  "How often the favorite won, by the closing spread (the home spread: negative means the home team is favored). Real: NFL closing lines 1999-2025 (nfl_history.json betting_lines). A well-set line wins more often as the spread grows, in step with real.",
  f"""WITH {LN} SELECT lv AS grp, {SPB} AS label, ROUND(100.0 * SUM(CASE WHEN sp < 0 THEN m > 0 ELSE m < 0 END) / SUM(m <> 0), 1) AS world,
            ROUND(100 * json_extract({NH}, '$.betting_lines.favorite_win_rate_by_spread."' || {SPB} || '".fav_win'), 1) AS real, COUNT(*) AS n
       FROM ln WHERE sp <> 0 GROUP BY lv, {SPB} ORDER BY lv, MIN(ABS(sp))""",
  says=f"""WITH {LN} SELECT printf('NFL lines miss the final margin by %.1f points on average (real %.1f) and the total by %.1f (real %.1f).',
     AVG(ABS(m + sp)), json_extract({NH}, '$.betting_lines.spread_mae'), AVG(ABS(pts - total)), json_extract({NH}, '$.betting_lines.total_mae')) FROM ln WHERE lv = 'NFL'""", needs=G)
panel("G32.accuracy", GP, "G32", "How good the lines are, by level and year", "table",
  "Each season: the mean miss of the spread against the final margin and of the total against points, how often the home side covered and the over hit, underdogs of 7+ winning, and the home edge (real margin against the line's implied edge). Real NFL 1999-2025: spread miss 10.3, total miss 10.6, home cover 47.6%, over 48.8%, 7+ underdogs 20.3%; home margin 2.06 against an implied 1.63 (2021-2025).",
  f"""WITH {LN} SELECT lv AS grp, year, COUNT(*) AS games, ROUND(AVG(ABS(m + sp)), 2) AS "spread miss", ROUND(AVG(ABS(pts - total)), 2) AS "total miss",
            ROUND(100.0 * SUM(m + sp > 0) / SUM(m + sp <> 0), 1) AS "home covered %", ROUND(100.0 * SUM(pts > total) / SUM(pts <> total), 1) AS "over %",
            ROUND(100.0 * SUM(ABS(sp) >= 7 AND CASE WHEN sp < 0 THEN m < 0 ELSE m > 0 END) / NULLIF(SUM(ABS(sp) >= 7), 0), 1) AS "7+ dogs won %",
            ROUND(AVG(CASE WHEN NOT neutral THEN m END), 2) AS "home margin", ROUND(AVG(CASE WHEN NOT neutral THEN -sp END), 2) AS "implied home edge"
       FROM ln GROUP BY lv, year ORDER BY lv, year""", needs=G)
check("G32.nfl_spread_miss", GP, "G32", "NFL spread miss against the final margin", "points",
  f"WITH {LN} SELECT AVG(ABS(m + sp)) FROM ln WHERE lv = 'NFL'", Key("nfl_history.json:betting_lines.spread_mae"), (0.08, 0.2, 0.0),
  "Mean absolute gap between the closing spread and the final margin, NFL games.", needs=G)
check("G32.nfl_total_miss", GP, "G32", "NFL total miss against points scored", "points",
  f"WITH {LN} SELECT AVG(ABS(pts - total)) FROM ln WHERE lv = 'NFL'", Key("nfl_history.json:betting_lines.total_mae"), (0.08, 0.2, 0.0),
  "Mean absolute gap between the closing total and the points scored.", needs=G)
check("G32.nfl_dogs", GP, "G32", "NFL underdogs of 7+ who won", "%",
  f"WITH {LN} SELECT 100.0 * SUM(CASE WHEN sp < 0 THEN m < 0 ELSE m > 0 END) / COUNT(*) FROM ln WHERE lv = 'NFL' AND ABS(sp) >= 7",
  Key("nfl_history.json:betting_lines.dog_7plus_win_rate", 100.0), (0.15, 0.35, 0.0), "Upsets by 7-point or bigger underdogs.", needs=G)
check("G32.nfl_home_cover", GP, "G32", "NFL home teams covering the spread", "%",
  f"WITH {LN} SELECT 100.0 * SUM(m + sp > 0) / SUM(m + sp <> 0) FROM ln WHERE lv = 'NFL'", Key("nfl_history.json:betting_lines.home_cover_rate", 100.0), (0.06, 0.15, 0.0),
  "Home sides beating the spread, pushes left out.", needs=G)

# ---------------------------------------------------------------- G33 injuries in games
INJ = """inj AS (SELECT CASE WHEN g.stream = 'nfl' THEN 'NFL' WHEN g.stream = 'college' THEN 'College' WHEN g.stream = 'hs' THEN 'High school' ELSE COALESCE(g.league, 'Other') END AS lv,
                  g.season AS year, i.*, CASE i.kind WHEN 'Acl' THEN 'Knee' WHEN 'Knee' THEN 'Knee' WHEN 'AnkleSprain' THEN 'Ankle' WHEN 'HighAnkleSprain' THEN 'Ankle' WHEN 'Shoulder' THEN 'Shoulder'
                       WHEN 'Concussion' THEN 'Concussion' WHEN 'Hamstring' THEN 'Hamstring' WHEN 'Back' THEN 'Back' WHEN 'Foot' THEN 'Foot'
                       WHEN 'Groin' THEN 'Groin' WHEN 'Calf' THEN 'Calf' WHEN 'Neck' THEN 'Neck' WHEN 'Hip' THEN 'Hip' WHEN 'Quad' THEN 'Quadricep' END AS part
             FROM log_games__injuries i JOIN log_games g ON g._row = i._parent)"""
panel("G33.mix", GP, "G33", "Injuries of a week or more by kind against real body parts", "bars",
  "Share of game injuries that cost a week or more, by kind. Real: NFL injury episodes by body part (development_injuries.json, games missed); where two kinds share a body part (knee and ACL, ankle and high ankle) the real share is split between them as the world splits them; kinds with no real body part of their own (bruise, hand, ribs, fracture) have no real bar. Foot, groin, calf, neck, hip and quad are their own kinds since minor bug 145.",
  f"""WITH {INJ}, k AS (SELECT lv, kind, part, COUNT(*) AS n FROM inj WHERE weeks >= 1 GROUP BY lv, kind, part),
       t AS (SELECT lv, SUM(n) AS tot FROM k GROUP BY lv), pt AS (SELECT lv, part, SUM(n) AS pn FROM k GROUP BY lv, part)
     SELECT k.lv AS grp, k.kind AS label, ROUND(100.0 * k.n / t.tot, 1) AS world,
            ROUND(100 * json_extract((SELECT value_json FROM real_figures WHERE key = 'development_injuries.json:injury_episodes_by_body_part.' || k.part), '$.share') * k.n / pt.pn, 1) AS real, k.n
       FROM k JOIN t USING (lv) LEFT JOIN pt ON pt.lv = k.lv AND pt.part = k.part ORDER BY k.lv, world DESC""", needs=G)
panel("G33.weeks", GP, "G33", "Weeks out by kind of injury", "table",
  "Each kind's count, mean weeks out, share that played on, and the quarter it happened in. Real NFL mean games missed by body part (development_injuries.json): knee 3.5, ankle 2.8, shoulder 3.2, concussion 2.2, hamstring 2.6, back 3.9.",
  f"""WITH {INJ}, w AS (SELECT lv AS grp, kind, COUNT(*) AS injuries, ROUND(AVG(weeks), 2) AS "mean weeks out", SUM(weeks >= 1) AS "a week or more",
            ROUND(100.0 * SUM(COALESCE(plays_on, 0)) / COUNT(*), 1) AS "played on %",
            MAX(part) AS part FROM inj GROUP BY lv, kind)
     SELECT w.*, json_extract((SELECT value_json FROM real_figures WHERE key = 'development_injuries.json:injury_episodes_by_body_part.' || w.part), '$.mean_games_missed') AS "real games missed"
       FROM w ORDER BY grp, injuries DESC""", needs=G)
panel("G33.rate", GP, "G33", "Injuries a team game, by level and year", "line",
  "Game injuries that cost a week or more, per team game, each season.",
  f"""WITH {INJ}, tg AS (SELECT CASE WHEN stream = 'nfl' THEN 'NFL' WHEN stream = 'college' THEN 'College' WHEN stream = 'hs' THEN 'High school' ELSE COALESCE(league, 'Other') END AS lv, season AS year, 2 * COUNT(*) AS n
               FROM log_games g WHERE EXISTS (SELECT 1 FROM log_games__box b WHERE b._parent = g._row) GROUP BY 1, 2)
     SELECT tg.lv AS grp, tg.year, ROUND(1.0 * (SELECT COUNT(*) FROM inj WHERE inj.lv = tg.lv AND inj.year = tg.year AND weeks >= 1) / tg.n, 3) AS world, NULL AS real, tg.n
       FROM tg ORDER BY 1, 2""", needs=G)
for c_ in ["plays_on", "weeks", "kind", "q", "player", "team"]:
    col("log_games__injuries", c_)
check("G33.knee_share", GP, "G33", "Knee injuries among NFL injuries of a week or more", "%",
  f"WITH {INJ} SELECT 100.0 * SUM(part = 'Knee') / COUNT(*) FROM inj WHERE lv = 'NFL' AND weeks >= 1",
  Key("development_injuries.json:injury_episodes_by_body_part.Knee#share", 100.0), (0.2, 0.5, 0.0), "Knee and ACL injuries over all game injuries costing a week or more.", needs=G)
check("G33.ankle_share", GP, "G33", "Ankle injuries among NFL injuries of a week or more", "%",
  f"WITH {INJ} SELECT 100.0 * SUM(part = 'Ankle') / COUNT(*) FROM inj WHERE lv = 'NFL' AND weeks >= 1",
  Key("development_injuries.json:injury_episodes_by_body_part.Ankle#share", 100.0), (0.2, 0.5, 0.0), "Ankle and high ankle sprains over all game injuries costing a week or more.", needs=G)
check("G33.knee_4plus", GP, "G33", "NFL knee injuries that cost four weeks or more", "%",
  f"WITH {INJ} SELECT 100.0 * AVG(weeks >= 4) FROM inj WHERE lv = 'NFL' AND part = 'Knee'",
  Key("development_injuries.json:injury_episodes_by_body_part.Knee#share_4_plus", 100.0), (0.2, 0.5, 0.0),
  "Share of NFL knee injuries in games that keep the player out four weeks or more. Real counts games missed per injury report episode, cut at the season's end, so a share of 4+ compares fairly where a mean would not (real mean 3.49 games, median 2).", needs=G)

# ---------------------------------------------------------------- G34 one game snap by snap
panel("G34.games", GP, "G34", "Games with every snap kept", "table",
  "Every logged game with its play-by-play, latest first: open one for the full game page (drive chart, win chance, scoring plays, player lines and every snap with its why).",
  f"""SELECT g.season AS year, g._row AS game, g.date, g.stream AS level, CASE WHEN g.stream = 'nfl' THEN {club('g.home', 'g.season')} WHEN g.stream = 'college' THEN (SELECT {prog('p')} FROM programs p WHERE p._row = g.home) ELSE json_extract(g.names, '$[0]') END AS home,
            CASE WHEN g.stream = 'nfl' THEN {club('g.away', 'g.season')} WHEN g.stream = 'college' THEN (SELECT {prog('p')} FROM programs p WHERE p._row = g.away) ELSE json_extract(g.names, '$[1]') END AS away,
            g.home_pts || '-' || g.away_pts AS score, lp.n AS snaps
       FROM log_plays lp JOIN log_games g ON g.season = lp.season AND g.stream = lp.stream AND g.date = lp.date AND g.home = lp.home AND g.away = lp.away
      WHERE g.season = (SELECT MAX(season) FROM log_plays) ORDER BY g.date DESC, g._row""", needs=G, open="game/{game}")
col("log_plays", "home"); col("log_plays", "away")
rule("G34.score_from_scoring", "G34", "The scoring plays add up to the final score",
  "SELECT COUNT(*) FROM log_games g WHERE EXISTS (SELECT 1 FROM log_games__scoring s WHERE s._parent = g._row)",
  """SELECT g._row AS game, g.season AS year, g.home_pts || '-' || g.away_pts AS final, (SELECT s.after FROM log_games__scoring s WHERE s._parent = g._row ORDER BY s._i DESC LIMIT 1) AS last_scoring
       FROM log_games g WHERE EXISTS (SELECT 1 FROM log_games__scoring s WHERE s._parent = g._row)
        AND (SELECT s.after FROM log_games__scoring s WHERE s._parent = g._row ORDER BY s._i DESC LIMIT 1) <> json_array(g.home_pts, g.away_pts)""",
  "Each game's last scoring play leaves the score at the final; a gap means a score the scoring list missed (a safety or a return) or a final changed after play.", needs=G)
rule("G34.box_points", "G34", "Each team's box score points equal its final score", "SELECT COUNT(*) FROM log_games__box",
  """SELECT g._row AS game, g.season AS year, b._i AS side, b.points, CASE b._i WHEN 0 THEN g.home_pts ELSE g.away_pts END AS final
       FROM log_games__box b JOIN log_games g ON g._row = b._parent WHERE COALESCE(b.points, 0) <> CASE b._i WHEN 0 THEN g.home_pts ELSE g.away_pts END""",
  "The box score's points and the game's final are written from the same game, so they must agree.", needs=G)
rule("G34.players_attempts", "G34", "Player pass attempts add up to the team's", "SELECT COUNT(*) FROM log_games__box",
  """SELECT g._row AS game, g.season AS year, b._i AS side, b.att AS team, (SELECT SUM(COALESCE(p.att, 0)) FROM log_games__players p WHERE p._parent = b._parent AND p._i = b._i) AS players
       FROM log_games__box b JOIN log_games g ON g._row = b._parent
      WHERE EXISTS (SELECT 1 FROM log_games__players p WHERE p._parent = b._parent) AND COALESCE(b.att, 0) <> (SELECT SUM(COALESCE(p.att, 0)) FROM log_games__players p WHERE p._parent = b._parent AND p._i = b._i)""",
  "Every pass attempt is credited to a passer, so the player lines sum to the team line.", needs=G)
rule("G34.drives_plays", "G34", "Drive points add up to the offense's points", "SELECT COUNT(*) FROM log_games__box b WHERE EXISTS (SELECT 1 FROM log_games__drives d WHERE d._parent = b._parent)",
  """SELECT g._row AS game, g.season AS year, b._i AS side, (SELECT SUM(COALESCE(d.points, 0)) FROM log_games__drives d WHERE d._parent = b._parent AND COALESCE(d.team, 0) = b._i) AS drive_points,
            b.points, COALESCE(b.def_td, 0) AS defense_tds
       FROM log_games__box b JOIN log_games g ON g._row = b._parent
      WHERE EXISTS (SELECT 1 FROM log_games__drives d WHERE d._parent = b._parent)
        AND (SELECT SUM(COALESCE(d.points, 0)) FROM log_games__drives d WHERE d._parent = b._parent AND COALESCE(d.team, 0) = b._i) > COALESCE(b.points, 0)""",
  "A drive's points can never add to more than the team scored (return and defensive scores come on top, so fewer is fine).", needs=G)
for c_ in ["def_td", "points"]:
    col("log_games__box", c_)

# ---------------------------------------------------------------- G35 film tapes
panel("G35.tapes", GP, "G35", "Film tapes kept for the coming season", "table",
  "Each tape the scouting staffs can study: the date, level, teams, score and how many snaps it holds, with the protection and pass-rush charts kept on it.",
  """SELECT t._row AS tape, t.date, t.level, json_extract(t.names, '$[0]') AS home, json_extract(t.names, '$[1]') AS away, t.score,
            (SELECT COUNT(*) FROM gameday__tapes__snaps s WHERE s._parent = t._row) AS snaps,
            (SELECT COUNT(*) FROM gameday__tapes__protection p WHERE p._parent = t._row) AS "protection rows", (SELECT COUNT(*) FROM gameday__tapes__rush r WHERE r._parent = t._row) AS "rush rows"
       FROM gameday__tapes t ORDER BY t.date DESC""")
panel("G35.habits", GP, "G35", "What the tapes show: pass rate, blitz and man coverage by down", "table",
  "Across every kept tape: plays by down with the pass rate, blitz rate, man coverage share, play action and pressure, the habits a staff reads from film.",
  """SELECT s.down, COUNT(*) AS snaps, ROUND(100.0 * SUM(s.kind = 'Pass') / COUNT(*), 1) AS "pass %", ROUND(100.0 * SUM(COALESCE(s.blitz, 0)) / COUNT(*), 1) AS "blitz %",
            ROUND(100.0 * SUM(COALESCE(s.man, 0)) / COUNT(*), 1) AS "man %", ROUND(100.0 * SUM(COALESCE(s.play_action, 0)) / NULLIF(SUM(s.kind = 'Pass'), 0), 1) AS "play action %",
            ROUND(100.0 * SUM(COALESCE(s.pressured, 0)) / NULLIF(SUM(s.kind = 'Pass'), 0), 1) AS "pressured %", ROUND(AVG(s.epa), 3) AS "EPA a play"
       FROM gameday__tapes__snaps s WHERE s.down BETWEEN 1 AND 4 GROUP BY s.down ORDER BY s.down""")
for c_ in ["blitz", "man", "play_action", "pressured", "epa", "kind", "down"]:
    col("gameday__tapes__snaps", c_)

# ---------------------------------------------------------------- G36 EPA and win probability
panel("G36.epa", GP, "G36", "EPA a play by level against real", "bars",
  "Expected points added per play, from the box scores (the NFL's expected points table is used at every level, so college EPA leans off a college table's line). Real: NFL 2021-2025 about -0.004 a play (nflverse).",
  f"""WITH b AS (SELECT CASE WHEN g.stream = 'nfl' THEN 'NFL' WHEN g.stream = 'college' THEN json_extract(g.divisions, '$[' || b._i || ']') WHEN g.stream = 'hs' THEN 'High school' ELSE COALESCE(g.league, 'Other') END AS lv,
                    b.epa, b.plays FROM log_games__box b JOIN log_games g ON g._row = b._parent)
     SELECT 'EPA a play' AS grp, lv AS label, ROUND(SUM(epa) / SUM(plays), 4) AS world, CASE WHEN lv = 'NFL' THEN json_extract({NGT}, '$.rates.epa_per_play') END AS real, COUNT(*) AS n
       FROM b GROUP BY lv ORDER BY {divord('lv')}""", needs=G)
panel("G36.wp", GP, "G36", "Win probability calibration: said against happened", "bars",
  "Every snap of the latest season's logged NFL games, by the offense's win probability before the snap (10-point buckets), against the share of those snaps whose offense went on to win (ties left out). A calibrated model lines up: snaps at 70-80% end in wins about 75% of the time. The model's table comes from nflverse (nfl_win_prob.csv).",
  """WITH g AS (SELECT lp._more, gm.home_pts - gm.away_pts AS m FROM log_plays lp JOIN log_games gm ON gm.season = lp.season AND gm.stream = lp.stream AND gm.date = lp.date AND gm.home = lp.home AND gm.away = lp.away
              WHERE lp.stream = 'nfl' AND lp.season = (SELECT MAX(season) FROM log_plays WHERE stream = 'nfl') AND gm.home_pts <> gm.away_pts),
       p AS (SELECT CAST(10 * json_extract(j.value, '$.wp_before') AS INTEGER) AS b,
                    CASE WHEN COALESCE(json_extract(j.value, '$.offense'), 0) = 0 THEN g.m > 0 ELSE g.m < 0 END AS won
               FROM g, json_each(more('log_plays', g._more), '$.plays') j WHERE json_extract(j.value, '$.wp_before') IS NOT NULL AND COALESCE(json_extract(j.value, '$.offense'), 0) < 2)
     SELECT 'Offense won %' AS grp, (10 * MIN(b, 9)) || '-' || (10 * MIN(b, 9) + 10) || '%' AS label, ROUND(100.0 * AVG(won), 1) AS world, 10 * MIN(b, 9) + 5 AS real, COUNT(*) AS n
       FROM p GROUP BY MIN(b, 9) ORDER BY MIN(b, 9)""",
  says="""SELECT 'The real bar is each bucket's middle: a calibrated model wins as often as it says.'""", needs=G)
check("G36.nfl_epa", GP, "G36", "NFL EPA a play", "EPA",
  "SELECT SUM(b.epa) / SUM(b.plays) FROM log_games__box b JOIN log_games g ON g._row = b._parent WHERE g.stream = 'nfl'",
  Key("nfl_game_targets.json:rates.epa_per_play"), (1.0, 3.0, 0.02), "Box score EPA over plays, every NFL game in the log. The real figure is near zero, so the check reads an absolute gap of 0.02.", needs=G)

# ---------------------------------------------------------------- G37 level charts
panel("G37.levels", GP, "G37", "Each level's passing and running this season and last", "table",
  "The world's level charts (kept for two seasons): team games, dropbacks, pressure rate, sacks a dropback, air yards a throw, time to throw, runs, yards a carry, yards before contact, stuffed runs and 10+ runs. Keys: NFL, FBS, FCS, small colleges, then each state's high schools. Real NFL (engine_targets.json, position_value.json): pressure about 29.5% of dropbacks, time to throw 2.71 s.",
  """SELECT c.year, CASE t._0 WHEN 0 THEN 'NFL' WHEN 1 THEN 'FBS' WHEN 2 THEN 'FCS' WHEN 3 THEN 'Small college' ELSE 'High school ' || COALESCE((SELECT code FROM states WHERE _row = t._0 - 10), t._0 - 10) END AS level,
            t._1__games AS "team games", t._1__dropbacks AS dropbacks, ROUND(100.0 * t._1__pressured / NULLIF(t._1__dropbacks, 0), 1) AS "pressure %",
            ROUND(100.0 * t._1__sacks / NULLIF(t._1__dropbacks, 0), 1) AS "sack %", ROUND(1.0 * t._1__air / NULLIF(t._1__throws, 0), 2) AS "air yards", ROUND(1.0 * t._1__ttt / NULLIF(t._1__throws, 0), 2) AS "time to throw",
            t._1__runs AS runs, ROUND(1.0 * t._1__run_yds / NULLIF(t._1__runs, 0), 2) AS "yards a carry", ROUND(1.0 * t._1__before / NULLIF(t._1__runs, 0), 2) AS "before contact",
            ROUND(100.0 * t._1__stuffed / NULLIF(t._1__runs, 0), 1) AS "stuffed %", ROUND(100.0 * t._1__ten / NULLIF(t._1__runs, 0), 1) AS "10+ runs %"
       FROM charts__seasons__tallies t JOIN charts__seasons c ON c._row = t._parent ORDER BY c.year DESC, t._0""")
for c_ in ["_1__games", "_1__dropbacks", "_1__pressured", "_1__sacks", "_1__throws", "_1__air", "_1__ttt", "_1__runs", "_1__run_yds", "_1__before", "_1__stuffed", "_1__ten"]:
    col("charts__seasons__tallies", c_)
check("G37.nfl_pressure", GP, "G37", "NFL pressure rate (level chart)", "%",
  "SELECT 100.0 * SUM(t._1__pressured) / SUM(t._1__dropbacks) FROM charts__seasons__tallies t WHERE t._0 = 0",
  Key("position_value.json:pressure#pressure_rate", 100.0), (0.12, 0.3, 0.0), "Pressured dropbacks over dropbacks in the NFL level chart.")
check("G37.nfl_ttt", GP, "G37", "NFL time to throw (level chart)", "seconds",
  "SELECT 1.0 * SUM(t._1__ttt) / SUM(t._1__throws) FROM charts__seasons__tallies t WHERE t._0 = 0",
  Key("position_value.json:pressure#time_to_throw"), (0.06, 0.15, 0.0), "Seconds to throw over throws in the NFL level chart.")

# ---------------------------------------------------------------- G38 engine calibration rows from the world's games
ROWS38 = [("Points a team game", "AVG(points)", f"json_extract({NGT}, '$.points_per_team_game')"),
          ("Plays a team game", "AVG(plays)", f"json_extract({NGT}, '$.per_team_game.plays')"),
          ("Pass attempts", "AVG(att)", f"json_extract({NGT}, '$.per_team_game.att')"),
          ("Completion %", "100.0 * SUM(cmp) / SUM(att)", f"100 * json_extract({NGT}, '$.rates.completion_pct')"),
          ("Yards an attempt (gross)", "1.0 * SUM(pass_yards + COALESCE(sack_yds, 0)) / SUM(att)", f"json_extract({NGT}, '$.rates.yards_per_attempt')"),
          ("Sacks", "AVG(sacks)", f"json_extract({NGT}, '$.per_team_game.sacks')"),
          ("Interceptions", "AVG(ints)", f"json_extract({NGT}, '$.per_team_game.int')"),
          ("Fumbles lost", "AVG(fumbles_lost)", f"json_extract({NGT}, '$.per_team_game.fumbles_lost')"),
          ("Carries", "AVG(carries)", f"json_extract({NGT}, '$.per_team_game.carries')"),
          ("Yards a carry", "1.0 * SUM(rush_yards) / SUM(carries)", f"json_extract({NGT}, '$.rates.yards_per_carry')"),
          ("Third-down conversion %", "100.0 * SUM(third_conv) / SUM(third_att)", f"100 * json_extract({NGT}, '$.rates.third_down_conversion')"),
          ("Success rate %", "100.0 * SUM(successes) / SUM(plays)", f"100 * json_extract({NGT}, '$.rates.success_rate')"),
          ("Explosive plays", "AVG(explosive)", f"json_extract({NGT}, '$.per_team_game.explosive_pass') + json_extract({NGT}, '$.per_team_game.explosive_run')"),
          ("Penalties", "AVG(penalties)", f"json_extract({NGT}, '$.per_team_game.penalties')"),
          ("Red zone trips", "AVG(red_zone_trips)", "json_extract((SELECT value FROM real_figures WHERE key = 'engine_targets.json:red_zone_and_returns.red_zone_trips_per_team_game'), '$')"),
          ("Red zone TD % a trip", "100.0 * SUM(red_zone_tds) / SUM(red_zone_trips)", "100 * json_extract((SELECT value FROM real_figures WHERE key = 'engine_targets.json:red_zone_and_returns.red_zone_td_rate_per_trip'), '$')"),
          ("Field goals tried", "AVG(fg_att)", "1.85"),
          ("Drives", "AVG(drives)", "NULL")]
panel("G38.rows", GP, "G38", "The engine's calibration rows, measured on the world's NFL games", "table",
  "match_check runs the engine on synthetic teams; this table measures the same rows on the world's own NFL regular-season games, next to real (nflverse 2021-2025) and the gap. Rows only the plays can give (time to throw spread, first tackler, targets by route and coverage) live on the situations pages. The synthetic run's last results are in docs/match-engine.md.",
  "WITH b AS (SELECT b.* FROM log_games__box b JOIN log_games g ON g._row = b._parent WHERE g.stream = 'nfl' AND COALESCE(g.kind, 'regular') = 'regular') SELECT stat, world, real, ROUND(100.0 * (world - real) / real, 1) AS \"gap %\", n FROM (" + " UNION ALL ".join(
      f"SELECT {i} AS o, '{lab}' AS stat, ROUND({ex}, 3) AS world, ROUND({rex}, 3) AS real, COUNT(*) AS n FROM b" for i, (lab, ex, rex) in enumerate(ROWS38)) + ") ORDER BY o",
  says="""WITH b AS (SELECT b.* FROM log_games__box b JOIN log_games g ON g._row = b._parent WHERE g.stream = 'nfl' AND COALESCE(g.kind, 'regular') = 'regular')
     SELECT printf('%d NFL team games measured: %.1f points and %.2f gross yards an attempt a team game.', COUNT(*), AVG(points), 1.0 * SUM(pass_yards + COALESCE(sack_yds, 0)) / SUM(att)) FROM b""", needs=G)
for c_ in ["sack_yds", "explosive", "red_zone_trips", "red_zone_tds", "drives", "third_conv", "third_att", "successes"]:
    col("log_games__box", c_)
