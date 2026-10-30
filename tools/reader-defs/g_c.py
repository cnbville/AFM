from common import *

GP = "games"
def ST(k, a="s"):
    """One stat of a book season, by its key in the engine's Stat list."""
    return f"COALESCE(json_extract({a}.s, '$[' || (SELECT _row FROM engine_stats WHERE key = '{k}') || ']'), 0)"
def pv(path): return f"json_extract((SELECT value_json FROM real_figures WHERE key = 'position_value.json:{path.split('#')[0]}'), '$.{path.split('#')[1]}')"

# ---------------------------------------------------------------- G14 team situation profiles
TS = """ts AS (SELECT season AS year, team,
          SUM(CASE WHEN key IN ('o|kind|pass', 'o|kind|run') THEN plays END) AS oplays, SUM(CASE WHEN key = 'o|kind|pass' THEN plays END) AS opass,
          SUM(CASE WHEN key IN ('o|kind|pass', 'o|kind|run') THEN epa END) AS oepa, SUM(CASE WHEN key IN ('o|kind|pass', 'o|kind|run') THEN successes END) AS osucc,
          SUM(CASE WHEN key IN ('d|kind|pass', 'd|kind|run') THEN plays END) AS dplays, SUM(CASE WHEN key IN ('d|kind|pass', 'd|kind|run') THEN epa END) AS depa,
          SUM(CASE WHEN key LIKE 'o|third|%' AND (key LIKE '%|pass' OR key LIKE '%|run') THEN plays END) AS third, SUM(CASE WHEN key LIKE 'o|third|%' AND (key LIKE '%|pass' OR key LIKE '%|run') THEN firsts + tds END) AS third_conv,
          SUM(CASE WHEN key IN ('o|red zone|pass', 'o|red zone|run') THEN plays END) AS rz, SUM(CASE WHEN key IN ('o|red zone|pass', 'o|red zone|run') THEN tds END) AS rz_td,
          SUM(CASE WHEN key = 'd|blitz|true' THEN plays END) AS blitz, SUM(CASE WHEN key LIKE 'd|blitz|%' THEN plays END) AS blitz_n,
          SUM(CASE WHEN key = 'd|pressured|true' THEN plays END) AS press, SUM(CASE WHEN key LIKE 'd|pressured|%' THEN plays END) AS press_n
        FROM log_team_situations GROUP BY season, team)"""
PS = "SELECT s.wins, s.losses, s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa, s.coach FROM programs__seasons s WHERE s._parent = p._row AND s.year = ts.year"
panel("G14.profiles", GP, "G14", "FBS team fingerprints, offense and defense", "table",
  "Each FBS program's season from its team situation totals: pass rate, EPA a play on offense and allowed on defense, success rate, third-down conversion, the touchdown rate of red zone plays, how often its defense blitzed and pressured the passer, with its record and scoring margin. Sorted by net EPA a play.",
  f"""WITH {TS}
     SELECT ts.year, {prog('p')} AS program, ts.team AS id,
            (SELECT wins || '-' || losses FROM programs__seasons s WHERE s._parent = p._row AND s.year = ts.year) AS record,
            (SELECT ROUND(1.0 * (totals__points_for - totals__points_against) / NULLIF(totals__games, 0), 1) FROM programs__seasons s WHERE s._parent = p._row AND s.year = ts.year) AS "margin a game",
            ROUND(100.0 * opass / oplays, 1) AS "pass %", ROUND(oepa / oplays, 3) AS "EPA a play", ROUND(depa / dplays, 3) AS "EPA allowed a play",
            ROUND(oepa / oplays - depa / dplays, 3) AS "net EPA", ROUND(100.0 * osucc / oplays, 1) AS "success %", ROUND(100.0 * third_conv / NULLIF(third, 0), 1) AS "third down %",
            ROUND(100.0 * rz_td / NULLIF(rz, 0), 1) AS "red zone play TD %", ROUND(100.0 * blitz / NULLIF(blitz_n, 0), 1) AS "blitz %", ROUND(100.0 * press / NULLIF(press_n, 0), 1) AS "pressure %"
       FROM ts JOIN programs p ON p._row = ts.team WHERE oplays > 0 ORDER BY ts.year DESC, "net EPA" DESC""",
  says=f"""WITH {TS}, x AS (SELECT oepa / oplays - depa / dplays AS e,
              (SELECT 1.0 * (totals__points_for - totals__points_against) / NULLIF(totals__games, 0) FROM programs__seasons s WHERE s._parent = ts.team + 0 AND s.year = ts.year) AS m FROM ts WHERE oplays > 0)
     SELECT printf('%d FBS team-seasons; net EPA a play and scoring margin correlate at %.2f (real NFL: pass EPA alone correlates 0.82 with winning).', COUNT(*),
       (AVG(e * m) - AVG(e) * AVG(m)) / sqrt((AVG(e * e) - AVG(e) * AVG(e)) * (AVG(m * m) - AVG(m) * AVG(m)))) FROM x WHERE m IS NOT NULL""", needs=G, open="e/programs/{id}")

panel("G14.spread", GP, "G14", "How FBS teams spread: pass rate, blitz rate, third downs", "hist",
  "FBS team-seasons in 5-point buckets of pass rate, defensive blitz rate and third-down conversion. Real team ranges, NFL 2025 (engine_targets.json scheme_mix_nfl): blitz rate 21.1% to 51.3%, man coverage 18.3% to 44.6%; no real college team spread is on file.",
  f"""WITH {TS}, x AS (SELECT 'Pass %' AS grp, 5 * CAST(20.0 * opass / oplays AS INTEGER) AS v FROM ts WHERE oplays > 0
             UNION ALL SELECT 'Blitz %', 5 * CAST(20.0 * blitz / blitz_n AS INTEGER) FROM ts WHERE blitz_n > 0
             UNION ALL SELECT 'Third down %', 5 * CAST(20.0 * third_conv / third AS INTEGER) FROM ts WHERE third > 0)
     SELECT grp, v AS x, 100.0 * COUNT(*) / SUM(COUNT(*)) OVER (PARTITION BY grp) AS world, COUNT(*) AS n FROM x GROUP BY grp, v ORDER BY grp, v""", needs=G)

check("G14.blitz_range", GP, "G14", "Spread of FBS team blitz rates (10th to 90th percentile)", "points",
  f"""WITH {TS}, b AS (SELECT 100.0 * blitz / blitz_n AS v, ROW_NUMBER() OVER (ORDER BY 100.0 * blitz / blitz_n) AS rn, COUNT(*) OVER () AS cnt FROM ts WHERE blitz_n > 50)
     SELECT MIN(CASE WHEN rn >= 0.9 * cnt THEN v END) - MIN(CASE WHEN rn >= 0.1 * cnt THEN v END) FROM b""",
  Fixed(30.2, "engine_targets.json scheme_mix_nfl defense_blitz_team_range: NFL teams 21.1% to 51.3% (2025; no college range on file)", "2025"), (0.25, 0.6, 0.0),
  "The gap in blitz rate between the 90th and 10th percentile FBS team-season. Against the NFL's whole range (lowest to highest team), so a world gap a little under it is fine.", needs=G)
check("G14.epa_margin", GP, "G14", "Net EPA a play against scoring margin, FBS team-seasons", "correlation",
  f"""WITH {TS}, x AS (SELECT oepa / oplays - depa / dplays AS e,
              (SELECT 1.0 * (totals__points_for - totals__points_against) / NULLIF(totals__games, 0) FROM programs__seasons s WHERE s._parent = ts.team + 0 AND s.year = ts.year) AS m FROM ts WHERE oplays > 0)
     SELECT (AVG(e * m) - AVG(e) * AVG(m)) / sqrt((AVG(e * e) - AVG(e) * AVG(e)) * (AVG(m * m) - AVG(m) * AVG(m))) FROM x WHERE m IS NOT NULL""",
  NoReal("no real college team-season EPA split on file; in the NFL pass EPA alone correlates 0.815 with winning (engine_targets.json what_wins), so net EPA should sit near 0.9"),
  (0.1, 0.2, 0.0), "Correlation across FBS team-seasons of net EPA a play (offense minus defense) and points margin a game.", needs=G)

# ---------------------------------------------------------------- G15 program seasons, luck and what wins
PY = f"""py AS (SELECT p._row AS id, {prog('p')} AS program, p.division AS div, s.year, s.wins, s.losses, s.coach, s.strength, s.totals__games AS g,
                 s.totals__points_for AS pf, s.totals__points_against AS pa, s.totals__turnovers AS tov, s.totals__takeaways AS take,
                 s.totals__epa AS epa, s.totals__epa_allowed AS epa_a, s.totals__plays AS plays, s.totals__plays_allowed AS plays_a,
                 (s.wins + s.losses) * 1.0 / (1 + exp(2.37 * (ln(MAX(s.totals__points_against, 1)) - ln(MAX(s.totals__points_for, 1))))) AS pyth
            FROM programs__seasons s JOIN programs p ON p._row = s._parent WHERE COALESCE(s.totals__games, 0) > 0)"""
panel("G15.luck", GP, "G15", "Program seasons: Pythagorean wins and luck", "table",
  "Each program season with engine totals: record, points for and against a game, the wins its points 'should' bring (Pythagorean, exponent 2.37, the usual college fit; the NFL's own fit is 2.68), luck as wins above that, turnover margin, EPA and EPA allowed a play, schedule strength and coach. Sorted by luck, both ends worth reading.",
  f"""WITH {PY}
     SELECT year, div AS division, program, id, wins || '-' || losses AS record, ROUND(1.0 * pf / g, 1) AS "points for", ROUND(1.0 * pa / g, 1) AS "points against",
            ROUND(pyth, 1) AS "Pythagorean wins", ROUND(wins - pyth, 1) AS luck, COALESCE(take, 0) - COALESCE(tov, 0) AS "turnover margin",
            ROUND(epa / NULLIF(plays, 0), 3) AS "EPA a play", ROUND(epa_a / NULLIF(plays_a, 0), 3) AS "EPA allowed", ROUND(strength, 2) AS strength,
            (SELECT name FROM coaches WHERE id = py.coach) AS coach
       FROM py ORDER BY year DESC, ABS(wins - pyth) DESC""",
  says=f"""WITH {PY} SELECT printf('%d program seasons; wins miss the Pythagorean line by %.2f a season (spread of luck), %d seasons beat it by 2 or more.',
     COUNT(*), sqrt(AVG((wins - pyth) * (wins - pyth))), SUM(wins - pyth >= 2)) FROM py""", open="e/programs/{id}")

panel("G15.turnovers", GP, "G15", "Win share by turnover margin a game", "bars",
  "Every program season's turnover margin a game (takeaways minus giveaways) in half-turnover buckets, and the share of games those teams won. Turnovers swing games in real football: a +1 a game team wins far more often.",
  f"""WITH {PY}, b AS (SELECT div, CAST(ROUND(2.0 * (COALESCE(take, 0) - COALESCE(tov, 0)) / g) AS INTEGER) AS bucket, wins, losses FROM py)
     SELECT div AS grp, printf('%+.1f', bucket / 2.0) AS label, ROUND(100.0 * SUM(wins) / SUM(wins + losses), 1) AS world, NULL AS real, COUNT(*) AS n
       FROM b WHERE bucket BETWEEN -4 AND 4 GROUP BY div, bucket ORDER BY {divord('div')}, bucket""")

panel("G15.best", GP, "G15", "Best and worst offenses and defenses each season", "table",
  "Each season and division: the top three and bottom three programs in points a game and in points allowed a game, from the program season totals.",
  f"""WITH {PY}, r AS (SELECT *, 1.0 * pf / g AS ppg, 1.0 * pa / g AS papg,
                         ROW_NUMBER() OVER (PARTITION BY year, div ORDER BY 1.0 * pf / g DESC) AS o_top, ROW_NUMBER() OVER (PARTITION BY year, div ORDER BY 1.0 * pf / g) AS o_bot,
                         ROW_NUMBER() OVER (PARTITION BY year, div ORDER BY 1.0 * pa / g) AS d_top, ROW_NUMBER() OVER (PARTITION BY year, div ORDER BY 1.0 * pa / g DESC) AS d_bot FROM py)
     SELECT year, div AS division, 'Best offense' AS what, o_top AS rank, program, id, ROUND(ppg, 1) AS "points a game", wins || '-' || losses AS record FROM r WHERE o_top <= 3
     UNION ALL SELECT year, div, 'Worst offense', o_bot, program, id, ROUND(ppg, 1), wins || '-' || losses FROM r WHERE o_bot <= 3
     UNION ALL SELECT year, div, 'Best defense', d_top, program, id, ROUND(papg, 1), wins || '-' || losses FROM r WHERE d_top <= 3
     UNION ALL SELECT year, div, 'Worst defense', d_bot, program, id, ROUND(papg, 1), wins || '-' || losses FROM r WHERE d_bot <= 3
     ORDER BY year DESC, division, what, rank""", open="e/programs/{id}")

check("G15.pythag_rmse", GP, "G15", "How far FBS win shares miss the Pythagorean line", "win share",
  f"WITH {PY} SELECT sqrt(AVG(((wins - pyth) / (wins + losses)) * ((wins - pyth) / (wins + losses)))) FROM py WHERE div = 'FBS' AND wins + losses >= 8",
  Key("nfl_history.json:parity.pythagorean_rmse_win_pct"), (0.2, 0.5, 0.0),
  "Root mean square gap between each FBS season's win share and its Pythagorean win share (exponent 2.37). Real: the NFL's own gap, 0.080 (nflverse 1999-2025, exponent 2.68); no college figure is on file, and college seasons are shorter, so a slightly larger gap is fine.")
check("G15.turnover_corr", GP, "G15", "Turnover margin against win share, FBS seasons", "correlation",
  f"""WITH {PY}, x AS (SELECT 1.0 * (COALESCE(take, 0) - COALESCE(tov, 0)) / g AS t, 1.0 * wins / (wins + losses) AS w FROM py WHERE div = 'FBS' AND wins + losses > 0)
     SELECT (AVG(t * w) - AVG(t) * AVG(w)) / sqrt((AVG(t * t) - AVG(t) * AVG(t)) * (AVG(w * w) - AVG(w) * AVG(w))) FROM x""",
  NoReal("no real college turnover-margin correlation on file; real football puts it near 0.4-0.5"), (0.1, 0.2, 0.0),
  "Correlation of turnover margin a game with win share across FBS program seasons.")

# ---------------------------------------------------------------- G16 player season lines (game log)
PL = """pl AS (SELECT g.season AS year, CASE WHEN g.stream = 'nfl' THEN 'NFL' WHEN g.stream = 'college' THEN json_extract(g.divisions, '$[' || p._i || ']')
                  WHEN g.stream = 'hs' THEN 'High school' ELSE COALESCE(g.league, 'Other') END AS lv, p.id,
                  COUNT(*) AS games, SUM(COALESCE(p.pass_yds, 0)) AS pass_yds, SUM(COALESCE(p.pass_td, 0)) AS pass_td, SUM(COALESCE(p.rush_yds, 0)) AS rush_yds,
                  SUM(COALESCE(p.rush_td, 0)) AS rush_td, SUM(COALESCE(p.rec, 0)) AS rec, SUM(COALESCE(p.rec_yds, 0)) AS rec_yds, SUM(COALESCE(p.rec_td, 0)) AS rec_td,
                  SUM(COALESCE(p.tackles, 0)) AS tackles, SUM(COALESCE(p.sacks, 0)) AS sacks, SUM(COALESCE(p.ints, 0)) AS ints
             FROM log_games__players p JOIN log_games g ON g._row = p._parent
            WHERE g.stream <> 'hs' AND COALESCE(g.kind, 'regular') <> 'preseason' AND (g.stream <> 'nfl' OR COALESCE(g.kind, 'regular') = 'regular') GROUP BY 1, 2, 3)"""
LEAD = [("pass_yds", "Passing yards", "pass_yards"), ("rush_yds", "Rushing yards", "rush_yards"), ("rec_yds", "Receiving yards", "rec_yards"), ("sacks", "Sacks", "sacks"),
        ("pass_td", "Touchdown passes", None), ("tackles", "Tackles", None), ("ints", "Interceptions", None)]
def lead_real(k, lv):
    if not k: return "NULL"
    return (f"CASE {lv} WHEN 'NFL' THEN (SELECT AVG(value) FROM json_each({NGT}, '$.season_leaders.{k}')) "
            f"WHEN 'FBS' THEN (SELECT AVG(value) FROM json_each({CGT}, '$.fbs_season_leaders.{k}')) END")
panel("G16.leaders", GP, "G16", "Season leaders by level, with the real leader", "table",
  "The top five each season in each stat, from the players' engine game lines (NFL regular season only, as the real leaders count; college with bowls and playoffs, as the NCAA has since 2002; no preseason). Real: the average season leader, NFL 2021-2025 (nflverse: passing 4,963, rushing 1,710, receiving 1,811, sacks 20.1) and FBS 2022-2024 (cfbfastR: passing 4,080, rushing 1,809, receiving 1,360).",
  f"""WITH {PL}, r AS (""" + " UNION ALL ".join(
      f"SELECT year, lv, '{lab}' AS stat, {i} AS o, id, games, {k} AS value, ROW_NUMBER() OVER (PARTITION BY year, lv ORDER BY {k} DESC) AS rank, {lead_real(rk, 'lv')} AS real FROM pl"
      for i, (k, lab, rk) in enumerate(LEAD)) + f""")
     SELECT year, lv AS level, stat, rank, {pname('r.id')} AS name, id, games, value, CASE WHEN rank = 1 THEN ROUND(real) END AS "real leader"
       FROM r WHERE rank <= 5 ORDER BY year DESC, {divord('lv')}, o, rank""",
  says=f"""WITH {PL} SELECT printf('The NFL passing leader averages %.0f yards a season (real %.0f); the FBS leader %.0f (real %.0f).',
     (SELECT AVG(m) FROM (SELECT MAX(pass_yds) AS m FROM pl WHERE lv = 'NFL' GROUP BY year)), (SELECT AVG(value) FROM json_each({NGT}, '$.season_leaders.pass_yards')),
     (SELECT AVG(m) FROM (SELECT MAX(pass_yds) AS m FROM pl WHERE lv = 'FBS' GROUP BY year)), (SELECT AVG(value) FROM json_each({CGT}, '$.fbs_season_leaders.pass_yards')))""",
  needs=G, open="e/people/{id}")

panel("G16.milestones", GP, "G16", "1,000-yard rushers and receivers, 4,000-yard passers, 100-tackle and 10-sack seasons", "table",
  "How many players reach each season milestone, by level and season, from the engine game lines (a season the log only partly holds counts short).",
  f"""WITH {PL} SELECT year, lv AS level, SUM(pass_yds >= 4000) AS "4,000 passing", SUM(pass_yds >= 3000) AS "3,000 passing", SUM(rush_yds >= 1000) AS "1,000 rushing",
            SUM(rec_yds >= 1000) AS "1,000 receiving", SUM(tackles >= 100) AS "100 tackles", SUM(sacks >= 10) AS "10 sacks", SUM(ints >= 5) AS "5 interceptions"
       FROM pl GROUP BY year, lv ORDER BY year DESC, {divord('lv')}""", needs=G)

panel("G16.by_stars", GP, "G16", "College production by recruiting stars", "bars",
  "Average yards from scrimmage and tackles plus sacks per player season, by the player's recruiting stars, FBS players with a logged season. Five-stars should produce more on average, with plenty of overlap.",
  f"""WITH {PL}, st AS (SELECT season AS year, id, MAX(person__stars) AS stars FROM log_players GROUP BY season, id)
     SELECT 'Yards from scrimmage' AS grp, COALESCE(st.stars, 0) || ' stars' AS label, ROUND(AVG(rush_yds + rec_yds + pass_yds), 0) AS world, NULL AS real, COUNT(*) AS n
       FROM pl LEFT JOIN st USING (year, id) WHERE pl.lv = 'FBS' AND rush_yds + rec_yds + pass_yds > 0 GROUP BY st.stars
     UNION ALL
     SELECT 'Tackles and sacks', COALESCE(st.stars, 0) || ' stars', ROUND(AVG(tackles + sacks), 1), NULL, COUNT(*)
       FROM pl LEFT JOIN st USING (year, id) WHERE pl.lv = 'FBS' AND tackles + sacks > 0 GROUP BY st.stars ORDER BY 1, 2""", needs=G)

for k, lab, real_, src in [("pass_yds", "passing yards", 4963.0, "nfl_game_targets.json season_leaders (nflverse 2021-2025)"),
                           ("rush_yds", "rushing yards", 1709.8, "nfl_game_targets.json season_leaders (nflverse 2021-2025)"),
                           ("rec_yds", "receiving yards", 1811.2, "nfl_game_targets.json season_leaders (nflverse 2021-2025)"),
                           ("sacks", "sacks", 20.1, "nfl_game_targets.json season_leaders (nflverse 2021-2025)")]:
    check(f"G16.nfl_leader_{k}", GP, "G16", f"NFL season leader in {lab}", lab,
      f"WITH {PL} SELECT AVG(m) FROM (SELECT MAX({k}) AS m FROM pl WHERE lv = 'NFL' GROUP BY year HAVING COUNT(*) > 500)",
      Fixed(real_, src + ": the mean of five season leaders", "2021-2025"), (0.08, 0.2, 0.0), f"The top {lab} total each NFL season in the log, averaged over seasons.", needs=G)
for k, lab, real_ in [("pass_yds", "passing yards", 4080.3), ("rush_yds", "rushing yards", 1808.7), ("rec_yds", "receiving yards", 1360.3)]:
    check(f"G16.fbs_leader_{k}", GP, "G16", f"FBS season leader in {lab}", lab,
      f"WITH {PL} SELECT AVG(m) FROM (SELECT MAX({k}) AS m FROM pl WHERE lv = 'FBS' GROUP BY year HAVING COUNT(*) > 500)",
      Fixed(real_, "college_game_targets.json fbs_season_leaders (cfbfastR): the mean of the 2022, 2023 and 2024 leaders", "2022-2024"), (0.1, 0.25, 0.0),
      f"The top {lab} total each FBS season in the log, averaged over seasons.", needs=G)

# ---------------------------------------------------------------- G17 the career stats book
MET = [("QB", "EPA per dropback", f"1.0 * {ST('PassEpa')} / {ST('Dropbacks')}", f"{ST('Dropbacks')} >= 100", pv("qb_seasons_300_dropbacks.epa_per_dropback#mean"), pv("qb_seasons_300_dropbacks.epa_per_dropback#sd")),
       ("QB", "Completion % over expected", f"100.0 * ({ST('Cmp')} - {ST('XCmp')}) / {ST('Att')}", f"{ST('Att')} >= 100", pv("qb_seasons_300_dropbacks.cpoe#mean"), pv("qb_seasons_300_dropbacks.cpoe#sd")),
       ("QB", "Sack rate %", f"100.0 * {ST('Sacked')} / {ST('Dropbacks')}", f"{ST('Dropbacks')} >= 100", f"100 * {pv('qb_seasons_300_dropbacks.sack_rate#mean')}", f"100 * {pv('qb_seasons_300_dropbacks.sack_rate#sd')}"),
       ("RB", "Yards a carry", f"1.0 * {ST('RushYds')} / {ST('Rush')}", f"{ST('Rush')} >= 60", pv("rb_seasons_120_carries#yards_per_carry.mean"), pv("rb_seasons_120_carries#yards_per_carry.sd")),
       ("RB", "Rush success %", f"100.0 * {ST('RushSuccess')} / {ST('Rush')}", f"{ST('Rush')} >= 60", f"100 * {pv('rb_seasons_120_carries#success_rate.mean')}", f"100 * {pv('rb_seasons_120_carries#success_rate.sd')}"),
       ("RB", "Yards after contact a carry", f"1.0 * {ST('Yaco')} / {ST('Rush')}", f"{ST('Rush')} >= 60", "NULL", "NULL"),
       ("WR", "Yards per route run", f"1.0 * {ST('RecYds')} / {ST('Routes')}", f"{ST('Routes')} >= 150", "NULL", "NULL"),
       ("WR", "Drop rate %", f"100.0 * {ST('Drops')} / ({ST('Rec')} + {ST('Drops')})", f"{ST('Targets')} >= 30", f"100 * json_extract((SELECT value FROM real_figures WHERE key = 'engine_targets.json:ftn_flags.drop_rate_of_catchable'), '$')", "NULL"),
       ("TE", "Yards per route run", f"1.0 * {ST('RecYds')} / {ST('Routes')}", f"{ST('Routes')} >= 150", "NULL", "NULL"),
       ("EDGE", "Pressure rate %", f"100.0 * {ST('Pressures')} / {ST('RushSnaps')}", f"{ST('RushSnaps')} >= 120", "NULL", "NULL"),
       ("EDGE", "Pass rush win rate %", f"100.0 * {ST('RushWins')} / {ST('RushSnaps')}", f"{ST('RushSnaps')} >= 120", "NULL", "NULL"),
       ("DT", "Pass rush win rate %", f"100.0 * {ST('RushWins')} / {ST('RushSnaps')}", f"{ST('RushSnaps')} >= 120", "NULL", "NULL"),
       ("LB", "Missed tackle rate %", f"100.0 * {ST('Missed')} / ({ST('Tackles')} + {ST('Missed')})", f"{ST('Tackles')} >= 20", "NULL", "NULL"),
       ("CB", "Yards allowed per coverage snap", f"1.0 * {ST('YdsAllowed')} / {ST('CovSnaps')}", f"{ST('CovSnaps')} >= 150", "NULL", "NULL"),
       ("OT", "Pressures allowed per pass block %", f"100.0 * {ST('PressuresAllowed')} / {ST('PbSnaps')}", f"{ST('PbSnaps')} >= 150", "NULL", "NULL")]
MV = "mv AS (" + " UNION ALL ".join(
    f"SELECT s.division AS div, '{pos}' AS pos, '{lab}' AS metric, {i} AS o, s.id, s.year, {ex} AS v, {rm} AS rmean, {rsd} AS rsd FROM stats__seasons s WHERE s.pos = '{pos}' AND {q}"
    for i, (pos, lab, ex, q, rm, rsd) in enumerate(MET)) + ")"
panel("G17.spread", GP, "G17", "Advanced rates in the stats book: average and spread", "table",
  "Each position's headline rates over qualified college seasons in the career stats book (QBs 100+ dropbacks or attempts, backs 60+ carries, receivers 150+ routes or 30+ targets, rushers 120+ rush snaps, linebackers 20+ tackles, corners 150+ coverage snaps, tackles 150+ pass blocks): mean, spread and the 10th and 90th percentiles. Real, where on file: NFL seasons (position_value.json, QBs with 300+ dropbacks and backs with 120+ carries, 2021-2025; the drop rate of catchable balls, FTN 2022-2025). No public college spread is on file, and college spreads run wider than the NFL's.",
  f"""WITH {MV}, r AS (SELECT *, ROW_NUMBER() OVER (PARTITION BY div, metric ORDER BY v) AS rn, COUNT(*) OVER (PARTITION BY div, metric) AS cnt FROM mv WHERE v IS NOT NULL)
     SELECT div AS grp, pos, metric, COUNT(*) AS seasons, ROUND(AVG(v), 3) AS mean, ROUND(sqrt(MAX(AVG(v * v) - AVG(v) * AVG(v), 0)), 3) AS spread,
            ROUND(MIN(CASE WHEN rn >= 0.1 * cnt THEN v END), 3) AS p10, ROUND(MIN(CASE WHEN rn >= 0.9 * cnt THEN v END), 3) AS p90,
            ROUND(MAX(rmean), 3) AS "real mean", ROUND(MAX(rsd), 3) AS "real spread"
       FROM r GROUP BY div, metric ORDER BY {divord('div')}, MIN(o)""",
  says=f"""WITH {MV} SELECT printf('%d qualified FBS QB seasons: EPA per dropback spreads %.3f (NFL real %.3f).', COUNT(*), sqrt(AVG(v * v) - AVG(v) * AVG(v)), MAX(rsd))
     FROM mv WHERE div = 'FBS' AND metric = 'EPA per dropback'""")

Y2Y = [("EPA per dropback", "QB"), ("Yards a carry", "RB"), ("Yards per route run", "WR"), ("Pressure rate %", "EDGE"), ("Completion % over expected", "QB")]
panel("G17.stickiness", GP, "G17", "Year-to-year stickiness of a player's rates", "bars",
  "Correlation of a player's rate in one college season with the next, for players qualified in both (same division). Real NFL (engine_targets.json player_spread, 2021-2025): QB EPA per dropback 0.43, WR and TE EPA per target 0.28, RB EPA per carry 0.05. Skill should carry over, but less than talent alone suggests.",
  f"""WITH {MV}, p AS (SELECT a.div, a.metric, a.v AS x, b.v AS y FROM mv a JOIN mv b ON b.id = a.id AND b.metric = a.metric AND b.year = a.year + 1)
     SELECT div AS grp, metric AS label, ROUND((AVG(x * y) - AVG(x) * AVG(y)) / sqrt((AVG(x * x) - AVG(x) * AVG(x)) * (AVG(y * y) - AVG(y) * AVG(y))), 3) AS world,
            CASE metric WHEN 'EPA per dropback' THEN 0.426 END AS real, COUNT(*) AS n
       FROM p GROUP BY div, metric HAVING COUNT(*) >= 10 ORDER BY {divord('div')}, metric""")

check("G17.qb_epa_spread", GP, "G17", "Spread of FBS QB EPA per dropback (100+ dropbacks)", "EPA",
  f"WITH {MV} SELECT sqrt(AVG(v * v) - AVG(v) * AVG(v)) FROM mv WHERE div = 'FBS' AND metric = 'EPA per dropback'",
  Key("position_value.json:qb_seasons_300_dropbacks.epa_per_dropback#sd"), (0.2, 0.5, 0.0),
  "Standard deviation of EPA per dropback across qualified FBS QB seasons in the stats book. Real is the NFL's (300+ dropbacks, 2021-2025); college talent gaps are wider, so a wider world spread is expected, not a fault.")
check("G17.cpoe_spread", GP, "G17", "Spread of FBS QB completion % over expected", "points",
  f"WITH {MV} SELECT sqrt(AVG(v * v) - AVG(v) * AVG(v)) FROM mv WHERE div = 'FBS' AND metric = 'Completion % over expected'",
  Key("position_value.json:qb_seasons_300_dropbacks.cpoe#sd"), (0.25, 0.6, 0.0), "Standard deviation of CPOE across qualified FBS QB seasons; real is the NFL's.")
check("G17.ypc_spread", GP, "G17", "Spread of FBS RB yards a carry (60+ carries)", "yards",
  f"WITH {MV} SELECT sqrt(AVG(v * v) - AVG(v) * AVG(v)) FROM mv WHERE div = 'FBS' AND metric = 'Yards a carry'",
  Key("position_value.json:rb_seasons_120_carries#yards_per_carry.sd"), (0.25, 0.6, 0.0), "Standard deviation of yards a carry across qualified FBS running back seasons; real is the NFL's (120+ carries).")
check("G17.drop_rate", GP, "G17", "FBS wide receiver drop rate", "%",
  f"SELECT 100.0 * SUM({ST('Drops')}) / SUM({ST('Rec')} + {ST('Drops')}) FROM stats__seasons s WHERE s.division = 'FBS' AND s.pos IN ('WR', 'TE')",
  Key("engine_targets.json:ftn_flags.drop_rate_of_catchable", 100.0), (0.2, 0.5, 0.5), "Drops over catches plus drops, FBS receivers and tight ends in the stats book. Real: drops of catchable balls, NFL (FTN 2022-2025).")
check("G17.qb_stickiness", GP, "G17", "Year-to-year correlation of FBS QB EPA per dropback", "correlation",
  f"""WITH {MV}, p AS (SELECT a.v AS x, b.v AS y FROM mv a JOIN mv b ON b.id = a.id AND b.metric = a.metric AND b.year = a.year + 1 WHERE a.div = 'FBS' AND a.metric = 'EPA per dropback')
     SELECT (AVG(x * y) - AVG(x) * AVG(y)) / sqrt((AVG(x * x) - AVG(x) * AVG(x)) * (AVG(y * y) - AVG(y) * AVG(y))) FROM p""",
  Key("engine_targets.json:player_spread.year_to_year_corr#qb_epa_per_dropback.0"), (0.25, 0.6, 0.05), "Correlation of a QB's EPA per dropback with his next season's; real is the NFL's (103 QB pairs).")

panel("G17.seasons", GP, "G17", "The stats book: every college season", "table",
  "The career stats book's seasons for the latest year, the busiest first: games, snaps, the main line for the position and schedule strength. Open a player for every season he played.",
  f"""SELECT s.year, s.division, s.pos, s.id, {pname('s.id')} AS name, {prog('p')} AS program, s.games, {ST('SnapsOff')} + {ST('SnapsDef')} AS snaps,
            CASE WHEN s.pos = 'QB' THEN {ST('Cmp')} || '/' || {ST('Att')} || ', ' || {ST('PassYds')} || ' yds, ' || {ST('PassTd')} || ' TD, ' || {ST('Int')} || ' INT'
                 WHEN s.pos = 'RB' THEN {ST('Rush')} || ' car, ' || {ST('RushYds')} || ' yds, ' || {ST('RushTd')} || ' TD'
                 WHEN s.pos IN ('WR', 'TE') THEN {ST('Rec')} || ' rec, ' || {ST('RecYds')} || ' yds, ' || {ST('RecTd')} || ' TD'
                 WHEN s.pos IN ('K') THEN {ST('FgMade')} || '/' || {ST('FgAtt')} || ' FG'
                 WHEN s.pos IN ('P') THEN {ST('Punts')} || ' punts'
                 WHEN s.pos IN ('OT', 'IOL') THEN {ST('PbSnaps')} || ' pass blocks, ' || {ST('PressuresAllowed')} || ' pressures allowed'
                 ELSE {ST('Tackles')} || ' tkl, ' || {ST('Tfl')} || ' TFL, ' || {ST('Sacks')} || ' sacks, ' || {ST('Ints')} || ' INT' END AS line,
            ROUND(s.sos, 2) AS "schedule strength"
       FROM stats__seasons s LEFT JOIN programs p ON p.id = s.program WHERE s.year = (SELECT MAX(year) FROM stats__seasons)
      ORDER BY {divord('s.division')}, snaps DESC""", open="e/people/{id}")

# ---------------------------------------------------------------- G18 norms
panel("G18.drift", GP, "G18", "Season norms year by year: what a regular does a game", "line",
  "The stats book's norms: each season's per-game mean among regulars (6+ games and 15+ snaps a game) for one headline stat per position, by division: QB passing yards, RB rushing yards, WR receiving yards, EDGE sacks. These are the base of the era-adjusted view; jumps after rule changes should show.",
  f"""WITH n AS (SELECT json_extract(_0, '$[0]') AS year, json_extract(_0, '$[1]') AS div, json_extract(_0, '$[2]') AS pos, _1 AS v FROM stats__norms)
     SELECT div || ' ' || pos || ': ' || CASE pos WHEN 'QB' THEN 'passing yards' WHEN 'RB' THEN 'rushing yards' WHEN 'WR' THEN 'receiving yards' ELSE 'sacks' END AS grp, year,
            ROUND(json_extract(v, '$[' || (SELECT _row FROM engine_stats WHERE key = CASE pos WHEN 'QB' THEN 'PassYds' WHEN 'RB' THEN 'RushYds' WHEN 'WR' THEN 'RecYds' ELSE 'Sacks' END) || ']'), 2) AS world,
            NULL AS real, NULL AS n
       FROM n WHERE pos IN ('QB', 'RB', 'WR', 'EDGE') ORDER BY 1, 2""")

panel("G18.latest", GP, "G18", "The latest season's norms by position", "table",
  "Per-game means among regulars in the latest season, by division and position: the numbers the era and opponent adjustments scale against.",
  f"""WITH n AS (SELECT json_extract(_0, '$[0]') AS year, json_extract(_0, '$[1]') AS div, json_extract(_0, '$[2]') AS pos, _1 AS v FROM stats__norms
              WHERE json_extract(_0, '$[0]') = (SELECT MAX(json_extract(_0, '$[0]')) FROM stats__norms))
     SELECT year, div AS division, pos, """ + ", ".join(
        f"ROUND(json_extract(v, '$[' || (SELECT _row FROM engine_stats WHERE key = '{k}') || ']'), 2) AS \"{lab}\""
        for k, lab in [("SnapsOff", "offense snaps"), ("SnapsDef", "defense snaps"), ("PassYds", "passing yards"), ("RushYds", "rushing yards"), ("Rec", "catches"),
                       ("RecYds", "receiving yards"), ("Tackles", "tackles"), ("Sacks", "sacks"), ("Pressures", "pressures"), ("Pbus", "breakups")]) +
  f" FROM n ORDER BY {divord('div')}, pos")

# ---------------------------------------------------------------- G19 footprints
panel("G19.rates", GP, "G19", "Hidden-gem footprint flags: how often each is set", "bars",
  "Share of book seasons at each position flagged on each footprint metric (top 1% of the division and position, with 50+ qualified seasons), and the testing-day flag (top 0.2% nationally in the forty or the jumps). About 1% of qualified seasons per metric is the rule's design; the share here is over all seasons at the position, so it runs lower.",
  # One pass over the book: each flag's count by division, then one row per flag.
  "WITH a AS MATERIALIZED (SELECT division, COUNT(*) AS seasons, " + ", ".join(f"SUM((footprint >> {k}) & 1) AS b{k}" for k in range(16)) + """ FROM stats__seasons GROUP BY division),
       f AS (SELECT _row AS bit, name FROM engine_footprints UNION ALL SELECT 15, 'testing day'),
       x AS (SELECT a.division, a.seasons, f.bit, f.name, CASE f.bit """ + " ".join(f"WHEN {k} THEN a.b{k}" for k in range(16)) + f""" END AS flagged FROM a, f)
     SELECT division AS grp, name AS label, ROUND(100.0 * flagged / seasons, 2) AS world, 1.0 AS real, flagged AS n
       FROM x WHERE flagged > 0 ORDER BY {divord('division')}, bit""")

panel("G19.gems", GP, "G19", "Flagged seasons and what became of the player", "table",
  "Every flagged book season, small schools first: the flags in plain words, and the player's later path (drafted, his NFL seasons) from the world's records.",
  f"""WITH f AS (SELECT _row AS bit, name FROM engine_footprints UNION ALL SELECT 15, 'testing day')
     SELECT s.year, s.division, s.pos, s.id, {pname('s.id')} AS name, {prog('p')} AS program,
            (SELECT group_concat(f.name, ', ') FROM f WHERE (s.footprint >> f.bit) & 1) AS flags,
            COALESCE((SELECT 'pro: ' || json_extract(a.pro__status, '$') FROM alumni a WHERE a.player__id = s.id), (SELECT 'still in the world' FROM people WHERE player__id = s.id)) AS later
       FROM stats__seasons s LEFT JOIN programs p ON p.id = s.program WHERE s.footprint > 0 ORDER BY {divord('s.division')} DESC, s.year DESC""", open="e/people/{id}")
col("alumni", "pro__status")

check("G19.flag_rate", GP, "G19", "FBS book seasons flagged on any footprint metric", "%",
  "SELECT 100.0 * SUM((footprint & 32767) > 0) / COUNT(*) FROM stats__seasons WHERE division = 'FBS'",
  NoReal("a design rule, not real data: top 1% per metric of qualified seasons; across all seasons and 15 metrics a few percent at most"), (0.5, 1.0, 1.0),
  "Seasons with at least one footprint flag (not counting the testing day) over all FBS book seasons.")

# ---------------------------------------------------------------- G20 record books and streaks
BOOK = f"""bk AS (SELECT json_extract(b._key, '$[0].Division') AS div, json_extract(b._key, '$[0].Conference') AS conf, json_extract(b._key, '$[0].State') AS state,
                 COALESCE(json_extract(b._key, '$[1].Season'), json_extract(b._key, '$[1].Career'), json_extract(b._key, '$[1].Streak')) AS stat,
                 CASE WHEN json_extract(b._key, '$[1].Season') IS NOT NULL THEN 'Season' WHEN json_extract(b._key, '$[1].Career') IS NOT NULL THEN 'Career'
                      WHEN json_extract(b._key, '$[1].Streak') IS NOT NULL THEN 'Streak' ELSE 'Win streak' END AS book, b.*
            FROM stats__books b)"""
BOOKSTATS = "(CASE {s} WHEN 0 THEN 'passing yards' WHEN 1 THEN 'passing TDs' WHEN 2 THEN 'rushing yards' WHEN 3 THEN 'rushing TDs' WHEN 4 THEN 'receptions' WHEN 5 THEN 'receiving yards' WHEN 6 THEN 'receiving TDs' WHEN 7 THEN 'tackles' WHEN 8 THEN 'sacks' WHEN 9 THEN 'interceptions' WHEN 10 THEN 'tackles for loss' WHEN 11 THEN 'field goals' WHEN 12 THEN 'return TDs' ELSE 'stat ' || {s} END)"
STREAKS = "(CASE {s} WHEN 0 THEN 'games with a TD pass' WHEN 1 THEN '100-yard rushing games' WHEN 2 THEN 'games with a catch' ELSE 'streak ' || {s} END)"
REC = [("passing yards", 0, 5967, "B.J. Symons, Texas Tech 2003"), ("passing TDs", 1, 62, "Joe Burrow, LSU 2019"), ("rushing yards", 2, 2628, "Barry Sanders, Oklahoma State 1988"),
       ("receiving yards", 5, 2060, "Trevor Insley, Nevada 1999"), ("sacks", 8, 24, "Terrell Suggs, Arizona State 2002"), ("interceptions", 9, 14, "Al Worley, Washington 1968")]
panel("G20.books", GP, "G20", "Record books: the top of each book against the real record", "table",
  "The holder of each book in each division and in each conference and state: single season, career and streaks, with the runner-up and whether the mark is still running. Real FBS single-season records, written from memory (build_data.py fbs_records; to verify against the NCAA record book): passing 5,967, passing TDs 62, rushing 2,628, receiving 2,060, sacks 24, interceptions 14. College books start empty in 1996, so the early years fill rather than break.",
  f"""WITH {BOOK}, r(stat, real) AS (VALUES {", ".join(f"({i}, {v})" for _, i, v, _h in REC)})
     SELECT COALESCE(bk.div, bk.conf, (SELECT code FROM states WHERE _row = bk.state)) AS scope, CASE WHEN bk.div IS NOT NULL THEN 'division' WHEN bk.conf IS NOT NULL THEN 'conference' ELSE 'state' END AS kind,
            bk.book, CASE WHEN bk.book = 'Streak' THEN {STREAKS.format(s='bk.stat')} WHEN bk.book = 'Win streak' THEN 'straight wins' ELSE {BOOKSTATS.format(s='bk.stat')} END AS stat,
            bk._0__value AS record, bk._0__name AS holder, bk._0__id AS id, bk._0__program AS program, bk._0__year AS year, CASE WHEN bk._0__live THEN 'running' END AS live,
            bk._1__value AS "second", bk._1__name AS "second by",
            CASE WHEN bk.div = 'FBS' AND bk.book = 'Season' THEN r.real END AS "real FBS record"
       FROM bk LEFT JOIN r ON r.stat = bk.stat ORDER BY kind, {divord('scope')}, scope, bk.book, bk.stat""",
  says="""SELECT printf('%d record books: %d by division, %d by conference, %d by state.', COUNT(*), SUM(json_extract(_key, '$[0].Division') IS NOT NULL),
     SUM(json_extract(_key, '$[0].Conference') IS NOT NULL), SUM(json_extract(_key, '$[0].State') IS NOT NULL)) FROM stats__books""", open="e/people/{id}")

panel("G20.win_streaks", GP, "G20", "Longest winning streaks, programs and high schools", "table",
  "The longest straight-win runs held by college programs and by high schools (running or ended), from the record books and each team's running count. Real: FBS 47 straight (Oklahoma, 1953-57); high school 151 (De La Salle, 1992-2004).",
  f"""SELECT 'College' AS level, {prog('p')} AS team, w._0 AS streak, w._1 AS since FROM stats__win_streak w JOIN programs p ON p._row = w._row WHERE w._0 > 0
     UNION ALL SELECT 'High school', (SELECT COALESCE(name, fictional_name) FROM high_schools WHERE _row = h._row), h._0, h._1 FROM stats__hs_streak h WHERE h._0 > 0
     ORDER BY streak DESC LIMIT 200""")
col("stats__books", "_1__value"); col("stats__books", "_1__name")

for lab, i, v, h in REC:
    check(f"G20.fbs_season_{i}", GP, "G20", f"FBS single-season record: {lab}", lab,
      f"""WITH {BOOK} SELECT _0__value FROM bk WHERE div = 'FBS' AND book = 'Season' AND stat = {i}""",
      Memory(float(v), f"{h} (build_data.py fbs_records, from memory; to verify)"), (0.15, 0.4, 0.0),
      f"The top of the FBS single-season {lab} book. In a world with a short history the book has had few seasons to fill, so a low mark is expected.")
check("G20.fbs_win_streak", GP, "G20", "Longest FBS winning streak", "games",
  f"WITH {BOOK} SELECT MAX(_0__value) FROM bk WHERE div = 'FBS' AND book = 'Win streak'",
  Memory(47.0, "Oklahoma 1953-57 (from memory)"), (0.3, 0.7, 0.0), "The top of the FBS straight-wins book.")
check("G20.hs_win_streak", GP, "G20", "Longest high school winning streak", "games",
  "SELECT MAX(_0) FROM stats__hs_streak",
  Memory(151.0, "De La Salle (Concord, CA) 1992-2004 (from memory); trial 10 had 248"), (0.3, 0.7, 0.0), "The longest running or ended high school win streak in the world's counts.")

# ---------------------------------------------------------------- G21 records broken
panel("G21.by_year", GP, "G21", "Records broken each year", "line",
  "Records broken per year by division, single season and career books (a fill of an empty book counts too: the first seasons fill every book, then breaks settle to a handful a year).",
  f"""SELECT _1 || CASE WHEN _2__Season IS NOT NULL THEN ': season' ELSE ': career' END AS grp, _0 AS year, COUNT(*) AS world, NULL AS real, COUNT(*) AS n
       FROM stats__broken GROUP BY 1, 2 ORDER BY 1, 2""")
panel("G21.list", GP, "G21", "Every record broken", "table",
  "Each record broken: year, division, the book, the new holder and the value. Watch for absurd values (an 86.5-sack high school season was one).",
  f"""SELECT _0 AS year, _1 AS division, CASE WHEN _2__Season IS NOT NULL THEN 'Season' ELSE 'Career' END AS book, {BOOKSTATS.format(s='COALESCE(_2__Season, _2__Career)')} AS stat,
            _3 AS holder, _4 AS value FROM stats__broken ORDER BY _0 DESC, {divord('_1')}""")
check("G21.breaks_settled", GP, "G21", "Records broken a year once the books are established", "a year",
  "SELECT 1.0 * COUNT(*) / COUNT(DISTINCT _0) FROM stats__broken WHERE _0 >= (SELECT MIN(_0) FROM stats__broken) + 10",
  NoReal("no real rate of records falling is on file; trial 10 settled at 6-29 a year across all divisions after the first ten seasons"), (0.5, 1.0, 5.0),
  "Breaks per year after the books' first ten seasons (records.rs counts the books as established after 10). A world with fewer than 11 seasons has no reading.")

# ---------------------------------------------------------------- G22 honors
AW = f"""aw AS (SELECT a._0 AS year, a._1 AS award, h._key AS id, h.name, h.pos FROM stats__honors__awards a JOIN stats__honors h ON h._row = a._parent)"""
panel("G22.awards", GP, "G22", "College awards by year", "table",
  "Every award winner (Heisman, O'Brien, Walker, Biletnikoff, Mackey, Outland, Butkus, Thorpe, Groza) with his position, his book season that year and the number of All-America and all-conference picks in his career. Awards are picked from the development step's season performance, so the line shows whether they match the stats.",
  f"""WITH {AW}
     SELECT aw.year, aw.award, aw.name, aw.id, aw.pos, {prog('p')} AS program, s.games,
            CASE WHEN aw.pos = 'QB' THEN {ST('PassYds')} || ' pass yds, ' || {ST('PassTd')} || ' TD' WHEN aw.pos = 'RB' THEN {ST('RushYds')} || ' rush yds, ' || {ST('RushTd')} || ' TD'
                 WHEN aw.pos IN ('WR', 'TE') THEN {ST('RecYds')} || ' rec yds, ' || {ST('RecTd')} || ' TD' WHEN aw.pos = 'K' THEN {ST('FgMade')} || '/' || {ST('FgAtt')} || ' FG'
                 ELSE {ST('Tackles')} || ' tkl, ' || {ST('Sacks')} || ' sacks, ' || {ST('Ints')} || ' INT' END AS line,
            (SELECT COUNT(*) FROM stats__honors__all_america x JOIN stats__honors h ON h._row = x._parent WHERE h._key = aw.id) AS "All-America"
       FROM aw LEFT JOIN stats__seasons s ON s.id = aw.id AND s.year = aw.year LEFT JOIN programs p ON p.id = s.program ORDER BY aw.year DESC, aw.award""",
  says=f"""WITH {AW} SELECT printf('%d awards given; %d of %d Heisman winners were quarterbacks (real: 21 of the 25 winners 2000-2024).', COUNT(*),
     SUM(award = 'Heisman Trophy' AND pos = 'QB'), SUM(award = 'Heisman Trophy')) FROM aw""", open="e/people/{id}")

panel("G22.all_america", GP, "G22", "All-Americans and their stats", "table",
  "Each first-team All-American by year and division with his position and his book season. All-America teams take 25 a division (QB, 2 RB, 3 WR, TE, 5 OL, 4 DL, 3 LB, 4 DB, K, P).",
  f"""SELECT x._0 AS year, x._1 AS division, h.pos, h.name, h._key AS id, {prog('p')} AS program, s.games,
            {ST('SnapsOff')} + {ST('SnapsDef')} AS snaps, ROUND(1.0 * {ST('GradeSum')} / NULLIF({ST('Graded')}, 0), 3) AS "grade a play"
       FROM stats__honors__all_america x JOIN stats__honors h ON h._row = x._parent LEFT JOIN stats__seasons s ON s.id = h._key AND s.year = x._0
       LEFT JOIN programs p ON p.id = s.program ORDER BY x._0 DESC, {divord('x._1')}, h.pos""", open="e/people/{id}")

panel("G22.heisman", GP, "G22", "This season's Heisman vote", "bars",
  "The current season's Heisman ballot: points by finalist and contender (first, second and third place votes in the note). Only the current and last season keep the vote.",
  f"""SELECT 'Points' AS grp, {pname('v.id')} AS label, v.points AS world, NULL AS real, v.first AS n FROM college__heisman_vote v ORDER BY v.points DESC""")
check("G22.heisman_qbs", GP, "G22", "Heisman winners who were quarterbacks", "%",
  f"WITH {AW} SELECT 100.0 * SUM(pos = 'QB') / COUNT(*) FROM aw WHERE award = 'Heisman Trophy'",
  Memory(84.0, "21 of the 25 Heisman winners 2000-2024 were quarterbacks (from memory)"), (0.15, 0.35, 0.0), "Share of Heisman winners at quarterback.")

# ---------------------------------------------------------------- G23 College Football Hall of Fame
panel("G23.classes", GP, "G23", "College Football Hall of Fame classes", "table",
  "Inductees by year: name, player or coach, division, programs and why. Players are eligible 10 to 50 years after their last season with a first-team All-America; the ballot weighs All-America seasons, big awards, all-conference picks and career grade.",
  "SELECT year, name, CASE WHEN coach THEN 'coach' ELSE 'player' END AS who, division, programs, why FROM stats__hall ORDER BY year DESC, division",
  says="SELECT printf('%d inductees over %d classes (%.1f a class; real NFF classes run about 15-25 a year, source needed).', COUNT(*), COUNT(DISTINCT year), 1.0 * COUNT(*) / MAX(COUNT(DISTINCT year), 1)) FROM stats__hall")
panel("G23.mix", GP, "G23", "Hall of Fame mix by division", "bars",
  "Inductees by division, players and coaches. Trial 10: 264 FBS, 25 FCS, 22 D2, 28 D3, 27 NAIA, 11 JUCO and 41 coaches.",
  f"SELECT CASE WHEN coach THEN 'Coaches' ELSE 'Players' END AS grp, division AS label, COUNT(*) AS world, NULL AS real, COUNT(*) AS n FROM stats__hall GROUP BY 1, 2 ORDER BY 1, {divord('division')}")
for c_ in ["year", "name", "coach", "division", "programs", "why", "who"]:
    col("stats__hall", c_)
check("G23.class_size", GP, "G23", "College Hall of Fame class size", "a class",
  "SELECT 1.0 * COUNT(*) / COUNT(DISTINCT year) FROM stats__hall",
  NoReal("real NFF class sizes need a source (none in data/real-world); trial 10 averaged about 18"), (0.3, 0.6, 2.0), "Inductees per class.")

# ---------------------------------------------------------------- G24 coach unit buzz
panel("G24.buzz", GP, "G24", "Coach unit buzz: each unit's EPA rank and the buzz it gave", "table",
  "Each coach's unit result: his offense or defense's EPA-a-play percentile in its division, the buzz gain it gave him and why, next to his hidden scheme rating (the reader shows true ratings; the game never does). The buzz should follow the true skill, with noise.",
  f"""SELECT b.year, c.name AS coach, b.coach AS id, {prog('p')} AS program, CASE WHEN b.offense THEN 'offense' ELSE 'defense' END AS unit,
            ROUND(100 * b.pct, 0) AS percentile, b.gain, c.ability__scheme AS "scheme (hidden)", b.why
       FROM stats__buzz b LEFT JOIN coaches c ON c.id = b.coach LEFT JOIN programs p ON p.id = b.program ORDER BY b.year DESC, b.pct DESC""", open="e/coaches/{id}")
panel("G24.by_skill", GP, "G24", "Unit percentile by the coach's hidden scheme rating", "bars",
  "Average EPA percentile of the units, by the coach's true scheme rating in tens. A rising line means buzz tracks real skill.",
  """SELECT CASE WHEN b.offense THEN 'Offense' ELSE 'Defense' END AS grp, (10 * CAST(c.ability__scheme / 10 AS INTEGER)) || 's' AS label, ROUND(100 * AVG(b.pct), 1) AS world, NULL AS real, COUNT(*) AS n
       FROM stats__buzz b JOIN coaches c ON c.id = b.coach GROUP BY 1, CAST(c.ability__scheme / 10 AS INTEGER) ORDER BY 1, 2""")
check("G24.skill_corr", GP, "G24", "Unit EPA percentile against the coach's scheme rating", "correlation",
  """WITH x AS (SELECT b.pct AS y, c.ability__scheme AS x FROM stats__buzz b JOIN coaches c ON c.id = b.coach)
     SELECT (AVG(x * y) - AVG(x) * AVG(y)) / sqrt((AVG(x * x) - AVG(x) * AVG(x)) * (AVG(y * y) - AVG(y) * AVG(y))) FROM x""",
  NoReal("coaching effects on EPA once the QB is held fixed are small in real football (coach_effect_offense.json, coach_effect_defense.json), so a modest positive correlation is right"),
  (0.2, 0.4, 0.0), "Correlation of a unit's EPA percentile with its coach's hidden scheme rating, every buzz row.")
