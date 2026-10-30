//! The programs, high school and coaches pages' checks, consistency checks
//! and panels (world reader lane R7): every inventory area of Programs
//! (C1-C17, C19-C22, C42, T13), Money's C14, High school (C23-C27, G7, G27,
//! N32), Coaches (S1-S31, S33), S34 on the NFL page and the conference
//! pages. Types in `defs.rs` and `panels.rs`; the shared pieces:
//! - a program season's level and conference are its real history's through
//!   2025 (`programs__history`: from, to, level, conference), else today's;
//! - a coach's job's role is `role` (head coach, coordinators) or
//!   `role__PositionCoach` (the group); a job's years overlap a season at
//!   each end, so tenure counts runs of the season's coach, as real data does;
//! - a child table's `_parent` has no type: compare it with `+x._row` so its
//!   index is used;
//! - real figures in the panels are read straight from the real files
//!   (`real_data.json`, `real_<csv>` tables).
//!
//! Written with a generator from one list; edit here and keep the style.

use crate::defs::{Check, Metric, Outlier, Real, Rule};
use crate::panels::Panel;

const W: &str = "world file";
const WL: &str = "world log";
const GL: &str = "game log";

/// Columns the SQL below reads that a small or early world may lack.
pub const COLUMNS: &[(&str, &str)] = &[
    ("coaches", "born"),
    ("coaches", "job__employer__Other"),
    ("coaches__history", "employer__Other"),
    ("high_schools", "county"),
    ("high_schools", "oc"),
    ("high_schools", "dc"),
    ("high_schools__seasons", "playoff_wins"),
    ("high_schools__seasons", "semifinal"),
    ("college_prev__polls", "points"),
    ("college_prev__polls", "firsts"),
    ("log_schools", "school__prestige"),
    ("log_coaches", "coach__retired"),
    ("log_coaches", "coach__reputation"),
    ("log_coaches", "coach__job__role"),
    ("log_coaches", "coach__job__employer__College"),
    ("log_programs", "result__expected"),
    ("log_programs", "result__wins"),
    ("log_programs", "result__losses"),
    ("people", "player__train__prep"),
    ("people", "player__train__rust"),
    ("log_coaches", "coach__ability__teaching__0"),
    ("log_coaches", "coach__ability__teaching__1"),
    ("log_coaches", "coach__ability__teaching__2"),
    ("log_coaches", "coach__ability__teaching__3"),
    ("log_coaches", "coach__ability__teaching__4"),
    ("log_coaches", "coach__ability__teaching__5"),
    ("log_coaches", "coach__ability__teaching__6"),
    ("log_coaches", "coach__ability__teaching__7"),
    ("log_coaches", "coach__ability__scheme"),
    ("log_coaches", "coach__ability__game_management"),
    ("log_coaches", "coach__ability__recruiting"),
    ("log_coaches", "coach__ability__network"),
];

pub const CHECKS: &[Check] = &[
    Check {
        id: "C1.fbs_programs",
        page: "programs",
        areas: "C1",
        title: r#"FBS programs this season"#,
        unit: "",
        world: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1) SELECT COUNT(*) FROM ps WHERE division = 'FBS' AND year = (SELECT MAX(year) FROM ps)"#,
        scale: 1.0,
        years: Some(
            r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1) SELECT year, COUNT(*) FROM ps WHERE division = 'FBS' GROUP BY year"#,
        ),
        detail: None,
        real: Real::Fixed(138.0, r#"colleges.csv (FBS programs of 2026)"#, r#"2026"#),
        tol: (0.02, 0.06, 0.0),
        needs: W,
        how: r#"Programs with an FBS season in the world's last season."#,
    },
    Check {
        id: "C4.p4_drought",
        page: "programs",
        areas: "C4",
        title: r#"Longest power-program run without a winning season"#,
        unit: "seasons",
        world: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1), x AS (SELECT pid, (w > l) AS win, year - ROW_NUMBER() OVER (PARTITION BY pid, (w > l) ORDER BY year) AS run FROM ps WHERE (conference IN ('SEC', 'Big Ten', 'ACC') OR (conference = 'Big 12' AND year >= 1996)
         OR (conference IN ('Big Eight', 'SWC') AND year <= 1995) OR (conference IN ('Pac-10', 'Pac-12') AND year <= 2023)
         OR (conference = 'Big East' AND year <= 2013) OR name = 'Notre Dame'))
         SELECT MAX(n) FROM (SELECT COUNT(*) AS n FROM x WHERE win = 0 GROUP BY pid, run)"#,
        scale: 1.0,
        years: None,
        detail: Some(
            r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1), x AS (SELECT pid, name, year, (w > l) AS win, year - ROW_NUMBER() OVER (PARTITION BY pid, (w > l) ORDER BY year) AS run FROM ps WHERE (conference IN ('SEC', 'Big Ten', 'ACC') OR (conference = 'Big 12' AND year >= 1996)
         OR (conference IN ('Big Eight', 'SWC') AND year <= 1995) OR (conference IN ('Pac-10', 'Pac-12') AND year <= 2023)
         OR (conference = 'Big East' AND year <= 2013) OR name = 'Notre Dame'))
         SELECT name || ', ' || MIN(year) || '-' || MAX(year) || '.' FROM x WHERE win = 0 GROUP BY pid, run ORDER BY COUNT(*) DESC LIMIT 1"#,
        ),
        real: Real::Key(
            r#"trial3_baselines.json:p4_droughts#longest_run_without_winning_season_2001_2025_top10.Kansas"#,
            1.0,
        ),
        tol: (0.15, 0.35, 0.0),
        needs: W,
        how: r#"Each power program's runs of seasons without more wins than losses (power conferences by era, Notre Dame included); the longest."#,
    },
    Check {
        id: "C4.fbs_points",
        page: "programs",
        areas: "C4",
        title: r#"FBS points a team game"#,
        unit: "pts",
        world: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1) SELECT 1.0 * SUM(pf) / SUM(g) FROM ps WHERE division = 'FBS' AND g > 0"#,
        scale: 1.0,
        years: Some(
            r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1) SELECT year, 1.0 * SUM(pf) / SUM(g) FROM ps WHERE division = 'FBS' AND g > 0 GROUP BY year"#,
        ),
        detail: None,
        real: Real::Fixed(27.6, r#"CFBD FBS games 2014-2025 (college_history.json scores_by_season)"#, r#"2014-2025"#),
        tol: (0.05, 0.15, 0.0),
        needs: W,
        how: r#"Points for over games in FBS program seasons with engine totals."#,
    },
    Check {
        id: "C5.unbeaten_champions",
        page: "programs",
        areas: "C5",
        title: r#"FBS champions who went unbeaten"#,
        unit: "%",
        world: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1) SELECT AVG(l = 0) FROM ps WHERE division = 'FBS' AND nt = 1"#,
        scale: 100.0,
        years: None,
        detail: None,
        real: Real::Memory(52.0, r#"FBS champions 2001-2025: 13 of 25 unbeaten (Miami 2001 to Indiana 2025)"#),
        tol: (0.15, 0.35, 0.0),
        needs: W,
        how: r#"FBS national champions with no losses, over all world seasons."#,
    },
    Check {
        id: "C5.power_champions",
        page: "programs",
        areas: "C5",
        title: r#"FBS champions from outside the power conferences"#,
        unit: "%",
        world: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1) SELECT AVG(NOT (conference IN ('SEC', 'Big Ten', 'ACC') OR (conference = 'Big 12' AND year >= 1996)
         OR (conference IN ('Big Eight', 'SWC') AND year <= 1995) OR (conference IN ('Pac-10', 'Pac-12') AND year <= 2023)
         OR (conference = 'Big East' AND year <= 2013) OR name = 'Notre Dame')) FROM ps WHERE division = 'FBS' AND nt = 1"#,
        scale: 100.0,
        years: None,
        detail: Some(
            r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1) SELECT group_concat(name || ' ' || year, ', ') FROM ps WHERE division = 'FBS' AND nt = 1 AND NOT (conference IN ('SEC', 'Big Ten', 'ACC') OR (conference = 'Big 12' AND year >= 1996)
         OR (conference IN ('Big Eight', 'SWC') AND year <= 1995) OR (conference IN ('Pac-10', 'Pac-12') AND year <= 2023)
         OR (conference = 'Big East' AND year <= 2013) OR name = 'Notre Dame')"#,
        ),
        real: Real::Fixed(
            0.0,
            r#"college_history.json champions: every FBS champion 2001-2025 from a power conference (BYU 1984 the last outside)"#,
            r#"2001-2025"#,
        ),
        tol: (0.05, 0.15, 4.0),
        needs: W,
        how: r#"FBS national champions whose conference that season was not a power conference."#,
    },
    Check {
        id: "C6.conf_titles_15y",
        page: "programs",
        areas: "C6",
        title: r#"Most conference titles by one FBS school in 15 years"#,
        unit: "",
        world: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1), t AS (SELECT pid, year FROM ps WHERE ct = 1 AND division = 'FBS'), e AS (SELECT DISTINCT year AS e FROM ps WHERE year >= (SELECT MIN(year) + 14 FROM ps))
         SELECT MAX(n) FROM (SELECT e.e, pid, COUNT(*) AS n FROM e JOIN t ON t.year BETWEEN e.e - 14 AND e.e GROUP BY e.e, pid)"#,
        scale: 1.0,
        years: None,
        detail: None,
        real: Real::Fixed(
            10.0,
            r#"college_history.json fbs_conference_champions (Boise State 2001-2015, Alabama 2011-2025, shared titles counted)"#,
            r#"2001-2025"#,
        ),
        tol: (0.2, 0.4, 0.0),
        needs: W,
        how: r#"Each 15-season window: the most FBS conference titles one program won; the most over all windows."#,
    },
    Check {
        id: "C7.fbs_attendance",
        page: "programs",
        areas: "C7",
        title: r#"FBS crowd a home game"#,
        unit: "",
        world: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1) SELECT AVG(g.attendance) FROM log_games g JOIN ps ON ps.prog = json_extract(g.teams, '$[0]') AND ps.year = g.season
         WHERE g.stream = 'college' AND ps.division = 'FBS' AND COALESCE(g.neutral, 0) = 0 AND g.attendance > 0"#,
        scale: 1.0,
        years: None,
        detail: None,
        real: Real::Fixed(40160.0, r#"CFBD FBS home games 2021-2025 (college_attendance.csv)"#, r#"2021-2025"#),
        tol: (0.08, 0.2, 0.0),
        needs: GL,
        how: r#"Mean attendance at FBS home games (not neutral sites) in the game log."#,
    },
    Check {
        id: "C12.fbs_postseason",
        page: "programs",
        areas: "C12",
        title: r#"FBS bowl and playoff games a season"#,
        unit: "",
        world: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1) SELECT AVG(n) FROM (SELECT g.season, COUNT(*) AS n FROM log_games g JOIN ps ON ps.prog = json_extract(g.teams, '$[0]') AND ps.year = g.season
         WHERE g.stream = 'college' AND ps.division = 'FBS' AND g.kind IN ('bowl', 'playoff', 'title') GROUP BY g.season)"#,
        scale: 1.0,
        years: None,
        detail: None,
        real: Real::Fixed(42.8, r#"CFBD postseason 2021-2025 (college_history.json)"#, r#"2021-2025"#),
        tol: (0.06, 0.15, 0.0),
        needs: GL,
        how: r#"FBS bowls, playoff and title games per season in the game log."#,
    },
    Check {
        id: "C19.fbs_tenure_5",
        page: "programs",
        areas: "C19 S7",
        title: r#"FBS head coaches who last five seasons"#,
        unit: "%",
        world: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1), r AS (SELECT pid, coach, MIN(year) AS a, COUNT(*) AS n FROM (SELECT pid, coach, year, year - ROW_NUMBER() OVER (PARTITION BY pid, coach ORDER BY year) AS run FROM ps WHERE division = 'FBS' AND coach IS NOT NULL) GROUP BY pid, coach, run)
         SELECT AVG(n >= 5) FROM r WHERE a > (SELECT MIN(year) FROM ps) AND a <= (SELECT MAX(year) FROM ps) - 10"#,
        scale: 100.0,
        years: None,
        detail: None,
        real: Real::Key(r#"college_people.json:fbs_head_coaches.share_lasting_at_least#5"#, 100.0),
        tol: (0.1, 0.25, 0.0),
        needs: W,
        how: r#"Head coach runs at FBS programs that began after the history's first season and at least ten seasons before the last: the share reaching five seasons."#,
    },
    Check {
        id: "C20.fbs_qbs",
        page: "programs",
        areas: "C20",
        title: r#"Quarterbacks on an FBS roster"#,
        unit: "",
        world: r#"SELECT 1.0 * SUM(x.player__position = 'QB') / COUNT(DISTINCT x.college) FROM people x JOIN programs p ON p._row = x.college WHERE x.stage = 'College' AND p.division = 'FBS'"#,
        scale: 1.0,
        years: None,
        detail: None,
        real: Real::Fixed(5.8, r#"ESPN FBS rosters 2022-24 (recruiting.rs real_roster)"#, r#"2022-2024"#),
        tol: (0.1, 0.25, 0.0),
        needs: W,
        how: r#"College quarterbacks at FBS programs today over the FBS programs with players."#,
    },
    Check {
        id: "C22.colleges_with_pick",
        page: "programs",
        areas: "C22",
        title: r#"Colleges with a draft pick a year"#,
        unit: "",
        world: r#"SELECT AVG(n) FROM (SELECT _0, COUNT(DISTINCT _parent) AS n FROM programs__draft_picks GROUP BY _0 ORDER BY _0 DESC LIMIT 10)"#,
        scale: 1.0,
        years: Some(r#"SELECT _0, COUNT(DISTINCT _parent) FROM programs__draft_picks GROUP BY _0"#),
        detail: None,
        real: Real::Fixed(
            97.2,
            r#"nfl_careers.json draft_by_college colleges_with_a_pick_by_year, mean of 2016-2025"#,
            r#"2016-2025"#,
        ),
        tol: (0.1, 0.25, 0.0),
        needs: W,
        how: r#"Programs with at least one pick (each pick is a row of the program's draft picks) in each of the world's last ten drafts, averaged."#,
    },
    Check {
        id: "T13.fcs_to_fbs",
        page: "programs",
        areas: "T13",
        title: r#"FCS programs moving up to FBS a decade"#,
        unit: "",
        world: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1), x AS (SELECT year, division, LAG(division) OVER (PARTITION BY pid ORDER BY year) AS before FROM ps)
         SELECT 10.0 * COUNT(*) / ((SELECT MAX(year) FROM ps) - 2025) FROM x WHERE before = 'FCS' AND division = 'FBS' AND year > 2025"#,
        scale: 1.0,
        years: None,
        detail: None,
        real: Real::Fixed(
            9.5,
            r#"college_history.json division_moves_up: 21 FCS to FBS moves 2004-2025"#,
            r#"2004-2025"#,
        ),
        tol: (0.3, 0.6, 1.0),
        needs: W,
        how: r#"FCS-to-FBS moves in the world's own seasons after 2025, per ten seasons."#,
    },
    Check {
        id: "C23.schools",
        page: "high-school",
        areas: "C23",
        title: r#"High schools with football"#,
        unit: "",
        world: r#"SELECT COUNT(*) FROM high_schools"#,
        scale: 1.0,
        years: None,
        detail: Some(
            r#"SELECT 'Most: ' || group_concat(code || ' ' || n, ', ') || '.' FROM (SELECT st.code, COUNT(*) AS n FROM high_schools h JOIN states st ON st._row = h.state GROUP BY st._row ORDER BY n DESC LIMIT 3)"#,
        ),
        real: Real::Fixed(14134.0, r#"hs_states.csv approx_programs summed (MaxPreps 2026)"#, r#"2026"#),
        tol: (0.03, 0.08, 0.0),
        needs: W,
        how: r#"Every high school in the world."#,
    },
    Check {
        id: "C24.champions",
        page: "high-school",
        areas: "C24 G7",
        title: r#"State champions a year"#,
        unit: "",
        world: r#"SELECT SUM(state_title) FROM high_schools__seasons WHERE year = (SELECT MAX(year) FROM high_schools__seasons)"#,
        scale: 1.0,
        years: Some(r#"SELECT year, SUM(state_title) FROM high_schools__seasons GROUP BY year"#),
        detail: None,
        real: Real::Fixed(
            350.0,
            r#"hs_playoff_formats.csv: each state's classes (doubled for Select and Non-Select), private-association and non-public classes, Open Divisions"#,
            r#"2025-26"#,
        ),
        tol: (0.05, 0.15, 0.0),
        needs: W,
        how: r#"School seasons with a state title in the world's last season."#,
    },
    Check {
        id: "C24.streak",
        page: "high-school",
        areas: "C24 G7",
        title: r#"Longest high school winning streak"#,
        unit: "games",
        world: r#"SELECT MAX(MAX(COALESCE(_0__value, 0), COALESCE(_1__value, 0))) FROM stats__books WHERE _key LIKE '%"WinStreak"%'"#,
        scale: 1.0,
        years: None,
        detail: Some(
            r#"SELECT _0__name || ', ended or running in ' || _0__year || '.' FROM stats__books WHERE _key LIKE '%"WinStreak"%' ORDER BY _0__value DESC LIMIT 1"#,
        ),
        real: Real::Memory(151.0, r#"De La Salle (Concord, CA), 1992-2004: the national record"#),
        tol: (0.15, 0.35, 0.0),
        needs: W,
        how: r#"The longest winning streak in the state record books."#,
    },
    Check {
        id: "G27.sacks",
        page: "high-school",
        areas: "G27",
        title: r#"Most sacks in a high school season"#,
        unit: "sacks",
        world: r#"SELECT MAX(json_extract(s, '$[15]')) / 2.0 FROM people__hs_lines"#,
        scale: 1.0,
        years: None,
        detail: Some(
            r#"SELECT p.name || ' (' || l.pos || '), ' || l.year || ': ' || (json_extract(l.s, '$[15]') / 2.0) || ' sacks in ' || l.games || ' games.' FROM people__hs_lines l JOIN people p ON p._row = l._parent ORDER BY json_extract(l.s, '$[15]') DESC LIMIT 1"#,
        ),
        real: Real::Memory(40.0, r#"trial 10: the real season record is about 40, from memory"#),
        tol: (0.15, 0.4, 0.0),
        needs: W,
        how: r#"The best season's sacks in the high school lines (kept in halves)."#,
    },
    Check {
        id: "G27.interceptions",
        page: "high-school",
        areas: "G27",
        title: r#"Most interceptions in a high school season"#,
        unit: "",
        world: r#"SELECT MAX(json_extract(s, '$[16]')) FROM people__hs_lines"#,
        scale: 1.0,
        years: None,
        detail: None,
        real: Real::Memory(12.0, r#"trial 10: real leaders 10-15 a season, from memory"#),
        tol: (0.2, 0.4, 0.0),
        needs: W,
        how: r#"The best season's interceptions in the high school lines."#,
    },
    Check {
        id: "S8.fbs_new_head_coaches",
        page: "coaches",
        areas: "S8 S7",
        title: r#"FBS programs with a new head coach a season"#,
        unit: "%",
        world: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1), x AS (SELECT year, pid, coach, LAG(coach) OVER (PARTITION BY pid ORDER BY year) AS before, division FROM ps)
         SELECT AVG(coach IS NOT before) FROM x WHERE division = 'FBS' AND before IS NOT NULL"#,
        scale: 100.0,
        years: Some(
            r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1), x AS (SELECT year, pid, coach, LAG(coach) OVER (PARTITION BY pid ORDER BY year) AS before, division FROM ps)
         SELECT year, AVG(coach IS NOT before) FROM x WHERE division = 'FBS' AND before IS NOT NULL GROUP BY year"#,
        ),
        detail: None,
        real: Real::Fixed(
            17.6,
            r#"college_people.json fbs_head_coaches changes_by_season, mean share 2005-2025"#,
            r#"2005-2025"#,
        ),
        tol: (0.15, 0.35, 0.0),
        needs: W,
        how: r#"FBS program seasons whose head coach (the season's coach) differs from the season before."#,
    },
    Check {
        id: "S7.fbs_tenure_median",
        page: "coaches",
        areas: "S7",
        title: r#"FBS head coach tenure (median, finished runs)"#,
        unit: "seasons",
        world: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1), runs AS MATERIALIZED (SELECT prog, name, division, coach, MIN(year) AS a, MAX(year) AS z, COUNT(*) AS n FROM (SELECT prog, name, division, coach, year,
         year - ROW_NUMBER() OVER (PARTITION BY prog, coach ORDER BY year) AS g FROM ps WHERE coach IS NOT NULL) GROUP BY prog, coach, g),
         ended AS (SELECT r.*, (SELECT h.ended FROM coaches__history h WHERE h._parent = r.coach AND h.employer__College = r.prog AND h."to" >= r.z ORDER BY h."to" LIMIT 1) AS ended FROM runs r
         WHERE r.a > (SELECT MIN(year) FROM ps) AND r.z < (SELECT MAX(year) FROM ps)), r AS (SELECT n AS v, ROW_NUMBER() OVER (ORDER BY n) AS r, COUNT(*) OVER () AS n FROM ended WHERE division = 'FBS')
         SELECT MAX(CASE WHEN r = CAST(0.5 * (n - 1) AS INTEGER) + 1 THEN v END) FROM r"#,
        scale: 1.0,
        years: None,
        detail: None,
        real: Real::Key(r#"college_people.json:fbs_head_coaches.completed_tenure_years#p50"#, 1.0),
        tol: (0.2, 0.4, 0.0),
        needs: W,
        how: r#"Runs of seasons one man was an FBS program's head coach (the season's coach), begun after the world's first season and ended before its last: the median."#,
    },
    Check {
        id: "S8.nfl_changes",
        page: "coaches",
        areas: "S8 S22",
        title: r#"NFL head coach changes a season"#,
        unit: "",
        world: r#"WITH x AS (SELECT year, team, coach, LAG(coach) OVER (PARTITION BY team ORDER BY year) AS before FROM nfl_seasons)
         SELECT 1.0 * SUM(coach IS NOT before) / COUNT(DISTINCT year) FROM x WHERE before IS NOT NULL"#,
        scale: 1.0,
        years: Some(
            r#"WITH x AS (SELECT year, team, coach, LAG(coach) OVER (PARTITION BY team ORDER BY year) AS before FROM nfl_seasons) SELECT year, SUM(coach IS NOT before) FROM x WHERE before IS NOT NULL GROUP BY year"#,
        ),
        detail: None,
        real: Real::Fixed(
            6.5,
            r#"nfl_history.json head_coaches offseason_changes_by_season, mean 2000-2025 (plus about 1.6 in-season)"#,
            r#"2000-2025"#,
        ),
        tol: (0.2, 0.4, 0.0),
        needs: W,
        how: r#"NFL club seasons whose head coach differs from the season before (`nfl_seasons`)."#,
    },
    Check {
        id: "S7.nfl_tenure_median",
        page: "coaches",
        areas: "S7 S22",
        title: r#"NFL head coach tenure (median, finished runs)"#,
        unit: "seasons",
        world: r#"WITH nr AS MATERIALIZED (SELECT team, coach, MIN(year) AS a, MAX(year) AS z, COUNT(*) AS n FROM (SELECT team, coach, year, year - ROW_NUMBER() OVER (PARTITION BY team, coach ORDER BY year) AS g FROM nfl_seasons) GROUP BY team, coach, g),
         nf AS (SELECT * FROM nr WHERE a > (SELECT MIN(year) FROM nfl_seasons) AND z < (SELECT MAX(year) FROM nfl_seasons)), r AS (SELECT n AS v, ROW_NUMBER() OVER (ORDER BY n) AS r, COUNT(*) OVER () AS n FROM nf)
         SELECT MAX(CASE WHEN r = CAST(0.5 * (n - 1) AS INTEGER) + 1 THEN v END) FROM r"#,
        scale: 1.0,
        years: None,
        detail: None,
        real: Real::Key(r#"nfl_history.json:head_coaches.completed_tenure_years#p50"#, 1.0),
        tol: (0.2, 0.4, 0.0),
        needs: W,
        how: r#"Runs of seasons one man was a club's head coach (`nfl_seasons`), begun after the world's first NFL season and ended before its last: the median."#,
    },
    Check {
        id: "S7.fbs_two_jobs",
        page: "coaches",
        areas: "S7 S8",
        title: r#"FBS head coaches who held two or more FBS head jobs"#,
        unit: "%",
        world: r#"WITH st AS MATERIALIZED (SELECT h._parent AS c, h._i AS i, CASE WHEN h.role = 'HeadCoach' THEN 'Head coach' WHEN h.role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN h.role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN h.role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, h."from" AS a, h."to" AS z, h.wins AS w, h.losses AS l, h.titles AS titles, h.ended AS ended,
         CASE WHEN h.employer__College IS NOT NULL THEN COALESCE((SELECT x._2 FROM programs__history x WHERE x._parent = h.employer__College AND h."to" BETWEEN x._0 AND x._1), p.division)
         WHEN h.employer__HighSchool IS NOT NULL THEN 'HS' WHEN h.employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level,
         h.employer__College AS college, h.employer__HighSchool AS hs, h.employer__Nfl AS nfl
         FROM coaches__history h LEFT JOIN programs p ON p._row = h.employer__College), jobs AS MATERIALIZED (SELECT c, role, a, z, w, l, titles, ended, level, college, hs, nfl FROM st
         UNION ALL SELECT c._row, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END, c.job__since, (SELECT MAX(year) FROM programs__seasons), c.wins, c.losses, c.titles, 'current',
         CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS' WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END,
         c.job__employer__College, c.job__employer__HighSchool, c.job__employer__Nfl
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL) SELECT AVG(n >= 2) FROM (SELECT c, COUNT(DISTINCT college) AS n FROM jobs WHERE role = 'Head coach' AND level = 'FBS' GROUP BY c)"#,
        scale: 100.0,
        years: None,
        detail: None,
        real: Real::Key(r#"college_people.json:fbs_head_coaches.coaches_with_two_plus_fbs_jobs_share"#, 100.0),
        tol: (0.2, 0.4, 0.0),
        needs: W,
        how: r#"Coaches who ever held an FBS head job: the share who held two or more at different programs."#,
    },
];

pub const METRICS: &[Metric] = &[];

pub const RULES: &[Rule] = &[
    Rule {
        id: "C4.wins_losses",
        areas: "C4 T11",
        title: r#"College wins equal college losses each season"#,
        checked: r#"SELECT COUNT(DISTINCT year) FROM programs__seasons"#,
        failures: r#"SELECT 'season', year, year, SUM(wins) || ' wins against ' || SUM(losses) || ' losses' FROM programs__seasons GROUP BY year HAVING ABS(SUM(wins) - SUM(losses)) > 0"#,
        needs: W,
        why: r#"Every college game has a winner and a loser among the programs (no ties since 1996)."#,
    },
    Rule {
        id: "C3.grade_range",
        areas: "C3 T11",
        title: r#"Every grade within 55-99"#,
        checked: r#"SELECT COUNT(*) FROM programs"#,
        failures: r#"SELECT 'program', COALESCE(p.real_name, p.fictional_name) || ' (' || p.id || ')', NULL, 'grade ' || j.key || ' is ' || ROUND(j.value, 1) FROM programs p, json_each(p.grades) j WHERE j.value < 54.5 OR j.value > 99.5"#,
        needs: W,
        why: r#"Grades run 55-99 within each level (`program.rs`)."#,
    },
    Rule {
        id: "C42.title_news",
        areas: "C42 C5 T11",
        title: r#"Every national title has its news"#,
        checked: r#"SELECT COUNT(*) FROM programs__seasons WHERE national_title = 1"#,
        failures: r#"SELECT 'program', COALESCE(p.real_name, p.fictional_name) || ' (' || p.id || ')', s.year, 'national title in ' || s.year || ' with no NationalTitle news that year'
         FROM programs__seasons s JOIN programs p ON p._row = s._parent WHERE s.national_title = 1 AND s.year >= (SELECT MIN(year) FROM news)
         AND NOT EXISTS (SELECT 1 FROM news n WHERE n.kind LIKE 'NationalTitle%' AND n.program = s._parent AND n.year BETWEEN s.year AND s.year + 1)"#,
        needs: W,
        why: r#"A title in the records must be in the news the same season (`news.rs`)."#,
    },
    Rule {
        id: "C24.wins_losses",
        areas: "C24 T11",
        title: r#"High school wins equal losses each season"#,
        checked: r#"SELECT COUNT(DISTINCT year) FROM high_schools__seasons"#,
        failures: r#"SELECT 'season', year, year, SUM(wins) || ' wins against ' || SUM(losses) || ' losses' FROM high_schools__seasons GROUP BY year HAVING SUM(wins) <> SUM(losses)"#,
        needs: W,
        why: r#"Every high school game is between two schools with a winner (forfeits count both ways; `hs_year.rs`)."#,
    },
    Rule {
        id: "C24.titles_count",
        areas: "C24 T11",
        title: r#"Each school's title count matches its seasons"#,
        checked: r#"SELECT COUNT(*) FROM high_schools"#,
        failures: r#"WITH t AS (SELECT _parent, SUM(state_title) AS n FROM high_schools__seasons GROUP BY _parent)
         SELECT 'high school', h.name || ' (' || h._row || ')', NULL, h.state_titles || ' titles stored, ' || t.n || ' in its seasons'
         FROM high_schools h JOIN t ON t._parent = h._row WHERE h.state_titles < t.n"#,
        needs: W,
        why: r#"`state_titles` counts every title, the real ones before the world's seasons included, so it can only be at or above the seasons' count."#,
    },
    Rule {
        id: "C24.one_champion",
        areas: "C24 T11",
        title: r#"One champion per class each season"#,
        checked: r#"SELECT COUNT(*) FROM hs_season__groups"#,
        failures: r#"SELECT 'state', st.code, (SELECT MAX(year) FROM high_schools__seasons), COUNT(*) || ' classes, ' || (SELECT SUM(s.state_title) FROM high_schools__seasons s JOIN high_schools h ON h._row = s._parent
         WHERE h.state = g.state AND s.year = (SELECT MAX(year) FROM high_schools__seasons)) || ' champions'
         FROM hs_season__groups g JOIN states st ON st._row = g.state GROUP BY g.state
         HAVING COUNT(*) <> (SELECT SUM(s.state_title) FROM high_schools__seasons s JOIN high_schools h ON h._row = s._parent WHERE h.state = g.state AND s.year = (SELECT MAX(year) FROM high_schools__seasons))"#,
        needs: W,
        why: r#"Each class's bracket crowns one champion (`hs_playoffs.rs`)."#,
    },
    Rule {
        id: "S7.no_overlap",
        areas: "S7 T11",
        title: r#"A coach's head jobs never overlap by more than a season"#,
        checked: r#"SELECT COUNT(*) FROM coaches__history WHERE role = 'HeadCoach'"#,
        failures: r#"SELECT 'coach', c.name || ' (' || a._parent || ')', a."from", 'head jobs ' || a."from" || '-' || a."to" || ' and ' || b."from" || '-' || b."to"
         FROM coaches__history a JOIN coaches__history b ON b._parent = a._parent AND b._i > a._i JOIN coaches c ON c._row = a._parent
         WHERE a.role = 'HeadCoach' AND b.role = 'HeadCoach' AND b."from" < a."to" "#,
        needs: W,
        why: r#"A coach holds one head job at a time; the one-season overlap of a move (he finishes one season and starts the next job that year) is allowed."#,
    },
    Rule {
        id: "S7.record_adds_up",
        areas: "S7 C4 T11",
        title: r#"A head coach's college record equals his program seasons"#,
        checked: r#"SELECT COUNT(DISTINCT coach) FROM programs__seasons WHERE coach IS NOT NULL"#,
        failures: r#"WITH st AS MATERIALIZED (SELECT h._parent AS c, h._i AS i, CASE WHEN h.role = 'HeadCoach' THEN 'Head coach' WHEN h.role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN h.role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN h.role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, h."from" AS a, h."to" AS z, h.wins AS w, h.losses AS l, h.titles AS titles, h.ended AS ended,
         CASE WHEN h.employer__College IS NOT NULL THEN COALESCE((SELECT x._2 FROM programs__history x WHERE x._parent = h.employer__College AND h."to" BETWEEN x._0 AND x._1), p.division)
         WHEN h.employer__HighSchool IS NOT NULL THEN 'HS' WHEN h.employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level,
         h.employer__College AS college, h.employer__HighSchool AS hs, h.employer__Nfl AS nfl
         FROM coaches__history h LEFT JOIN programs p ON p._row = h.employer__College), jobs AS MATERIALIZED (SELECT c, role, a, z, w, l, titles, ended, level, college, hs, nfl FROM st
         UNION ALL SELECT c._row, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END, c.job__since, (SELECT MAX(year) FROM programs__seasons), c.wins, c.losses, c.titles, 'current',
         CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS' WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END,
         c.job__employer__College, c.job__employer__HighSchool, c.job__employer__Nfl
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL), r AS (SELECT c, SUM(w) AS w, SUM(l) AS l FROM jobs WHERE role = 'Head coach' AND college IS NOT NULL AND a > (SELECT MIN(year) FROM programs__seasons) GROUP BY c),
         s AS (SELECT coach, SUM(wins) AS w, SUM(losses) AS l FROM programs__seasons WHERE coach IS NOT NULL GROUP BY coach)
         SELECT 'coach', co.name || ' (' || r.c || ')', NULL, 'jobs say ' || r.w || '-' || r.l || ', his seasons say ' || s.w || '-' || s.l FROM r JOIN s ON s.coach = r.c JOIN coaches co ON co._row = r.c
         WHERE (r.w > s.w OR r.l > s.l) AND r.w + r.l > s.w + s.l + 1
         AND r.c NOT IN (SELECT _parent FROM coaches__history WHERE ended LIKE '%during the season%')"#,
        needs: W,
        why: r#"Wins credited to a head job come from the seasons he coached; jobs before the history began are left out. A coach fired during the season keeps that season's games while the season's row goes to the man who finished it, and a coach hired in December may coach the bowl, so those are left out."#,
    },
    Rule {
        id: "S1.age_from_birth",
        areas: "S1 T11",
        title: r#"Every coach's age counts from his birth date"#,
        checked: r#"SELECT COUNT(*) FROM coaches WHERE born IS NOT NULL"#,
        failures: r#"SELECT 'coach', name || ' (' || _row || ')', NULL, 'age ' || age || ', born ' || date(born * 86400, 'unixepoch') FROM coaches
         WHERE born IS NOT NULL AND ABS(age - (CAST(strftime('%Y', (SELECT MAX(year) FROM programs__seasons) + 1 || '-09-01') AS INTEGER) - CAST(strftime('%Y', date(born * 86400, 'unixepoch')) AS INTEGER))) > 1"#,
        needs: W,
        why: r#"Every age shown comes from the date (final build lane 1, A1)."#,
    },
];

pub const OUTLIERS: &[Outlier] = &[];

pub const PANELS: &[Panel] = &[
    Panel {
        id: "C1.levels_by_year",
        page: "programs",
        area: "C1",
        title: r#"Programs at each level, season by season"#,
        kind: "line",
        note: r#"How many programs fielded a team at each level each season (their program seasons), against the real count from the program histories (`college_history.csv`, 1984-2025). Pick a level above the chart."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1) SELECT ps.year, ps.division AS grp, COUNT(*) AS world,
         (SELECT COUNT(*) FROM real_college_history r WHERE ps.year BETWEEN r.first_season AND r.last_season AND r.division = ps.division) AS real
         FROM ps GROUP BY ps.year, ps.division ORDER BY ps.division, ps.year"#,
        says: Some(
            r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1) SELECT printf('%d programs played in %d: %s.', COUNT(*), year, group_concat(n || ' ' || division, ', ')) FROM (SELECT year, division, COUNT(*) AS n FROM ps WHERE year = (SELECT MAX(year) FROM ps) GROUP BY division ORDER BY n DESC)"#,
        ),
        needs: W,
        open: "",
    },
    Panel {
        id: "C1.today",
        page: "programs",
        area: "C1",
        title: r#"Programs at each level now, against the real 2026 count"#,
        kind: "bars",
        note: r#"Programs with a team in the world's last season by level, against the real programs of 2026 (`colleges.csv` and `colleges_lower.csv`)."#,
        sql: r#"SELECT p.division AS label, COUNT(*) AS world,
         (SELECT COUNT(*) FROM real_colleges r WHERE r.division = p.division) + (SELECT COUNT(*) FROM real_colleges_lower r WHERE r.division = p.division) AS real
         FROM programs p WHERE p._row IN (SELECT _parent FROM programs__seasons WHERE year = (SELECT MAX(year) FROM programs__seasons))
         GROUP BY p.division ORDER BY world DESC"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C1.list",
        page: "programs",
        area: "C1",
        title: r#"Every program"#,
        kind: "table",
        note: r#"Each program's name, level, conference, home, enrollment, academic tier and stadium seats, with the real stadium where `colleges.csv` has it, and its prestige now. A row opens its page."#,
        sql: r#"SELECT p.id, COALESCE(p.real_name, p.fictional_name) AS name, p.division, p.conference, sc.code AS state, p.city, p.enrollment, p.academic_tier AS academics,
         p.stadium, COALESCE(r.stadium_capacity, rl.stadium_capacity) AS real_stadium, ROUND(p.prestige, 3) AS prestige
         FROM programs p LEFT JOIN state_codes sc ON sc.idx = p.state
         LEFT JOIN real_colleges r ON r.name = p.real_name LEFT JOIN real_colleges_lower rl ON rl.name = p.real_name
         ORDER BY CASE p.division WHEN 'FBS' THEN 0 WHEN 'FCS' THEN 1 WHEN 'D2' THEN 2 WHEN 'NAIA' THEN 3 WHEN 'D3' THEN 4 ELSE 5 END, p.prestige DESC"#,
        says: None,
        needs: W,
        open: "e/programs/{id}",
    },
    Panel {
        id: "C1.stadium_gap",
        page: "programs",
        area: "C1",
        title: r#"Stadiums that differ from the real capacity"#,
        kind: "table",
        note: r#"Programs whose stadium seats differ from the real capacity (`colleges.csv`) by more than 5%."#,
        sql: r#"SELECT p.id, COALESCE(p.real_name, p.fictional_name) AS name, p.division, p.stadium, COALESCE(r.stadium_capacity, rl.stadium_capacity) AS real_stadium,
         ROUND(100.0 * (p.stadium - COALESCE(r.stadium_capacity, rl.stadium_capacity)) / COALESCE(r.stadium_capacity, rl.stadium_capacity), 1) AS gap_pct
         FROM programs p LEFT JOIN real_colleges r ON r.name = p.real_name LEFT JOIN real_colleges_lower rl ON rl.name = p.real_name
         WHERE COALESCE(r.stadium_capacity, rl.stadium_capacity) > 0 AND ABS(p.stadium - COALESCE(r.stadium_capacity, rl.stadium_capacity)) > 0.05 * COALESCE(r.stadium_capacity, rl.stadium_capacity)
         ORDER BY ABS(gap_pct) DESC"#,
        says: None,
        needs: W,
        open: "e/programs/{id}",
    },
    Panel {
        id: "C1.story",
        page: "programs",
        area: "C1",
        title: r#"Each program's story lines"#,
        kind: "table",
        note: r#"Every dated line of each program's story (`Program.story`): facility projects, boosters, hires and moves, newest first. A row opens the program."#,
        sql: r#"SELECT st._0 AS year, p.id, COALESCE(p.real_name, p.fictional_name) AS program, p.division AS now, st._1 AS story
         FROM programs__story st JOIN programs p ON p._row = st._parent ORDER BY st._0 DESC, program"#,
        says: None,
        needs: W,
        open: "e/programs/{id}",
    },
    Panel {
        id: "C2.top",
        page: "programs",
        area: "C2",
        title: r#"The most prestigious programs"#,
        kind: "table",
        note: r#"The top 40 programs by prestige now, with the parts it is built from (tradition, recent success, facilities, fans; 0-1), where it started, and how it ranks within its level (`prestige_rel`). Prestige = 0.15 tradition + 0.3 recent + 0.12 coach + 0.12 facilities + 0.11 boosters + 0.12 fans + 0.08 conference (`program.rs`, `prestige_of`)."#,
        sql: r#"SELECT p.id, COALESCE(p.real_name, p.fictional_name) AS name, p.division, p.conference, ROUND(p.prestige, 3) AS prestige, ROUND(p.prestige_start, 3) AS at_start,
         ROUND(p.prestige - p.prestige_start, 3) AS change, ROUND(p.tradition, 2) AS tradition, ROUND(p.recent, 2) AS recent, ROUND(p.facilities, 2) AS facilities,
         ROUND(p.fans, 2) AS fans, ROUND(p.prestige_rel, 2) AS in_level
         FROM programs p ORDER BY p.prestige DESC LIMIT 40"#,
        says: Some(
            r#"SELECT printf('%s leads on prestige (%.2f); the top 10 are %s.', (SELECT COALESCE(real_name, fictional_name) FROM programs ORDER BY prestige DESC LIMIT 1), (SELECT MAX(prestige) FROM programs),
         (SELECT group_concat(n, ', ') FROM (SELECT COALESCE(real_name, fictional_name) AS n FROM programs ORDER BY prestige DESC LIMIT 10)))"#,
        ),
        needs: W,
        open: "e/programs/{id}",
    },
    Panel {
        id: "C2.risers",
        page: "programs",
        area: "C2",
        title: r#"Biggest risers and fallers since the history began"#,
        kind: "table",
        note: r#"The 15 programs whose prestige rose most and the 15 that fell most from the start of the history to now, each with its latest story line (the why the world wrote)."#,
        sql: r#"SELECT * FROM (SELECT p.id, COALESCE(p.real_name, p.fictional_name) AS name, p.division, ROUND(p.prestige_start, 3) AS at_start, ROUND(p.prestige, 3) AS now,
         ROUND(p.prestige - p.prestige_start, 3) AS change, (SELECT _0 || ': ' || _1 FROM programs__story WHERE _parent = +p._row ORDER BY _0 DESC LIMIT 1) AS last_story
         FROM programs p ORDER BY change DESC LIMIT 15)
         UNION ALL SELECT * FROM (SELECT p.id, COALESCE(p.real_name, p.fictional_name), p.division, ROUND(p.prestige_start, 3), ROUND(p.prestige, 3),
         ROUND(p.prestige - p.prestige_start, 3) AS change, (SELECT _0 || ': ' || _1 FROM programs__story WHERE _parent = +p._row ORDER BY _0 DESC LIMIT 1)
         FROM programs p ORDER BY change ASC LIMIT 15)"#,
        says: None,
        needs: W,
        open: "e/programs/{id}",
    },
    Panel {
        id: "C2.spread",
        page: "programs",
        area: "C2",
        title: r#"The spread of prestige within each level, year by year"#,
        kind: "line",
        note: r#"The 10th, 50th and 90th percentile of prestige each year (`prestige_history`), by today's level: does the spread hold, close up or blow apart? Pick a level above the chart."#,
        sql: r#"WITH x AS (SELECT h._0 AS year, p.division AS grp, h._1 AS v, ROW_NUMBER() OVER (PARTITION BY h._0, p.division ORDER BY h._1) AS r,
         COUNT(*) OVER (PARTITION BY h._0, p.division) AS n FROM programs__prestige_history h JOIN programs p ON p._row = h._parent)
         SELECT year, grp, MAX(CASE WHEN r = CAST(0.1 * (n - 1) AS INTEGER) + 1 THEN v END) AS p10, MAX(CASE WHEN r = CAST(0.5 * (n - 1) AS INTEGER) + 1 THEN v END) AS p50, MAX(CASE WHEN r = CAST(0.9 * (n - 1) AS INTEGER) + 1 THEN v END) AS p90 FROM x GROUP BY year, grp ORDER BY grp, year"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C2.vs_wins",
        page: "programs",
        area: "C2",
        title: r#"Prestige against the last five seasons' win rate"#,
        kind: "bars",
        note: r#"The correlation between each program's prestige now and its win rate over the last five seasons, by level. No real figure: prestige has no public measure."#,
        sql: r#"WITH w AS (SELECT s._parent AS prog, SUM(s.wins) * 1.0 / NULLIF(SUM(s.wins + s.losses), 0) AS wp FROM programs__seasons s
         WHERE s.year > (SELECT MAX(year) FROM programs__seasons) - 5 GROUP BY s._parent),
         x AS (SELECT p.division AS d, p.prestige AS a, w.wp AS b FROM programs p JOIN w ON w.prog = p._row WHERE w.wp IS NOT NULL)
         SELECT d AS label, ROUND((COUNT(*) * SUM(a * b) - SUM(a) * SUM(b)) / sqrt((COUNT(*) * SUM(a * a) - SUM(a) * SUM(a)) * (COUNT(*) * SUM(b * b) - SUM(b) * SUM(b))), 3) AS world, COUNT(*) AS n
         FROM x GROUP BY d ORDER BY n DESC"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C2.top10_by_decade",
        page: "programs",
        area: "C2",
        title: r#"The top 10 by prestige at each decade's turn"#,
        kind: "table",
        note: r#"The ten most prestigious programs at the end of 2000, 2010, 2020 and the last year kept (`prestige_history`), to set against the real bluebloods of each era."#,
        sql: r#"WITH x AS (SELECT h._0 AS year, COALESCE(p.real_name, p.fictional_name) AS name, p.id, h._1 AS v, ROW_NUMBER() OVER (PARTITION BY h._0 ORDER BY h._1 DESC) AS rk
         FROM programs__prestige_history h JOIN programs p ON p._row = h._parent
         WHERE h._0 IN (2000, 2010, 2020, (SELECT MAX(_0) FROM programs__prestige_history)))
         SELECT year, rk AS rank, name, id, ROUND(v, 3) AS prestige FROM x WHERE rk <= 10 ORDER BY year DESC, rk"#,
        says: None,
        needs: W,
        open: "e/programs/{id}",
    },
    Panel {
        id: "C3.by_division",
        page: "programs",
        area: "C3",
        title: r#"The 12 grades by level"#,
        kind: "matrix",
        note: r#"The mean of each grade a recruit reads (55-99 within the level; A+ 93 and up, C 64, D 55) by level, now."#,
        sql: r#"WITH g(k, grade) AS (VALUES (0, 'Academics'), (1, 'Pro pipeline'), (2, 'Playing time'), (3, 'Tradition'), (4, 'Title chances'),
         (5, 'Coach prestige'), (6, 'Facilities'), (7, 'NIL'), (8, 'Brand exposure'), (9, 'Conference strength'), (10, 'Stadium atmosphere'), (11, 'Campus life')) SELECT g.grade AS row, p.division AS col, ROUND(AVG(j.value), 1) AS value FROM programs p, json_each(p.grades) j JOIN g ON g.k = j.key
         GROUP BY g.k, p.division ORDER BY g.k, CASE p.division WHEN 'FBS' THEN 0 WHEN 'FCS' THEN 1 WHEN 'D2' THEN 2 WHEN 'NAIA' THEN 3 WHEN 'D3' THEN 4 ELSE 5 END"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C3.speed",
        page: "programs",
        area: "C3",
        title: r#"How far each grade moves in a year"#,
        kind: "bars",
        note: r#"The mean move of each grade from one year to the next over every program and year (`grade_history`): fast grades move 80% toward their target a year, steady ones 15%, conference strength only on realignment."#,
        sql: r#"WITH g(k, grade) AS (VALUES (0, 'Academics'), (1, 'Pro pipeline'), (2, 'Playing time'), (3, 'Tradition'), (4, 'Title chances'),
         (5, 'Coach prestige'), (6, 'Facilities'), (7, 'NIL'), (8, 'Brand exposure'), (9, 'Conference strength'), (10, 'Stadium atmosphere'), (11, 'Campus life')), gh AS MATERIALIZED (SELECT h._parent AS prog, h._0 AS y, CAST(j.key AS INTEGER) AS k, j.value AS v FROM programs__grade_history h, json_each(h._1) j)
         SELECT g.grade AS label, ROUND(AVG(ABS(b.v - a.v)), 2) AS world, COUNT(*) AS n FROM gh a JOIN gh b ON b.prog = a.prog AND b.k = a.k AND b.y = a.y + 1 JOIN g ON g.k = a.k
         GROUP BY a.k ORDER BY a.k"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C3.academics_top",
        page: "programs",
        area: "C3",
        title: r#"The best academics grades"#,
        kind: "table",
        note: r#"The 25 programs with the best academics grade, with their academic tier and admissions bar: Stanford, Northwestern, Vanderbilt and Notre Dame should be near the top."#,
        sql: r#"SELECT p.id, COALESCE(p.real_name, p.fictional_name) AS name, p.division, p.academic_tier AS tier, ROUND(json_extract(p.grades, '$[0]'), 1) AS academics,
         ROUND(p.admissions_bar, 2) AS admissions_bar, p.special_admits FROM programs p ORDER BY academics DESC LIMIT 25"#,
        says: None,
        needs: W,
        open: "e/programs/{id}",
    },
    Panel {
        id: "C4.points",
        page: "programs",
        area: "C4",
        title: r#"Points a team game by level, season by season"#,
        kind: "line",
        note: r#"Points a team scored per game in each level's program seasons with engine totals, against the real mean (CFBD 2001-2025 for FBS, FCS from 2003, D2 and D3 from 2021-22; `college_history.json`). Pick a level."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1) SELECT year, division AS grp, ROUND(1.0 * SUM(pf) / SUM(g), 2) AS world,
         json_extract((SELECT json FROM real_data WHERE file = 'college_history.json'), '$.scores_by_season.by_season.' || year || '.' || division || '.points_per_team_game') AS real
         FROM ps WHERE g > 0 GROUP BY year, division ORDER BY division, year"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C4.droughts",
        page: "programs",
        area: "C4",
        title: r#"Longest runs without a winning season, power programs"#,
        kind: "table",
        note: r#"Each power program's longest run of seasons without more wins than losses (all games). Real 2001-2025: Kansas 14, Duke 12, Indiana 11, Colorado, Illinois and Vanderbilt 10 (`trial3_baselines.json` p4_droughts)."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1), x AS (SELECT pid, name, year, (w > l) AS win, year - ROW_NUMBER() OVER (PARTITION BY pid, (w > l) ORDER BY year) AS run FROM ps WHERE (conference IN ('SEC', 'Big Ten', 'ACC') OR (conference = 'Big 12' AND year >= 1996)
         OR (conference IN ('Big Eight', 'SWC') AND year <= 1995) OR (conference IN ('Pac-10', 'Pac-12') AND year <= 2023)
         OR (conference = 'Big East' AND year <= 2013) OR name = 'Notre Dame'))
         SELECT pid AS id, name, MIN(year) AS from_year, MAX(year) AS to_year, COUNT(*) AS seasons FROM x WHERE win = 0 GROUP BY pid, run ORDER BY seasons DESC LIMIT 25"#,
        says: None,
        needs: W,
        open: "e/programs/{id}",
    },
    Panel {
        id: "C4.records_by_level",
        page: "programs",
        area: "C4",
        title: r#"Win rates by level and season"#,
        kind: "table",
        note: r#"The spread of win rates each season by level: the share of programs at .750 or better, at .250 or worse, and unbeaten."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1) SELECT year, division, COUNT(*) AS programs, ROUND(100.0 * AVG(1.0 * w / NULLIF(w + l, 0) >= 0.75), 1) AS pct_750_up,
         ROUND(100.0 * AVG(1.0 * w / NULLIF(w + l, 0) <= 0.25), 1) AS pct_250_down, SUM(l = 0 AND w > 0) AS unbeaten, SUM(w = 0 AND l > 0) AS winless
         FROM ps GROUP BY year, division ORDER BY year DESC, division"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C4.over_under",
        page: "programs",
        area: "C4",
        title: r#"Seasons furthest from expected wins (world log)"#,
        kind: "table",
        note: r#"Each season's wins against the expected wins the world set before it (`TeamResult.expected`, from the world log's yearly program rows): the biggest over- and under-performers, and whether the head coach kept his job."#,
        sql: r#"WITH x AS (SELECT l.season AS year, l.id, l.result__expected AS expected, l.result__wins AS w, l.result__losses AS lo, l.program__division AS division,
         (SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = l.id) AS name FROM log_programs l)
         SELECT * FROM (SELECT year, id, name, division, w || '-' || lo AS record, ROUND(expected, 1) AS expected, ROUND(w - expected, 1) AS over FROM x WHERE expected IS NOT NULL ORDER BY over DESC LIMIT 20)
         UNION ALL SELECT * FROM (SELECT year, id, name, division, w || '-' || lo, ROUND(expected, 1), ROUND(w - expected, 1) AS over FROM x WHERE expected IS NOT NULL ORDER BY over ASC LIMIT 20)"#,
        says: None,
        needs: WL,
        open: "e/programs/{id}",
    },
    Panel {
        id: "C5.champions",
        page: "programs",
        area: "C5",
        title: r#"National champions by season and level"#,
        kind: "table",
        note: r#"Every national champion the world crowned, by season and level, with its record and head coach. Newest first."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1) SELECT year, division, pid AS id, name AS champion, w || '-' || l AS record, (l = 0) AS unbeaten, (SELECT name FROM coaches c WHERE c.id = ps.coach) AS coach, conference
         FROM ps WHERE nt = 1 ORDER BY year DESC, CASE division WHEN 'FBS' THEN 0 WHEN 'FCS' THEN 1 WHEN 'D2' THEN 2 WHEN 'NAIA' THEN 3 WHEN 'D3' THEN 4 ELSE 5 END"#,
        says: Some(
            r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1) SELECT printf('%d FBS titles in the world: %s won %d; %d different champions.', (SELECT COUNT(*) FROM ps WHERE nt = 1 AND division = 'FBS'),
         (SELECT name FROM ps WHERE nt = 1 AND division = 'FBS' GROUP BY pid ORDER BY COUNT(*) DESC LIMIT 1),
         (SELECT COUNT(*) AS n FROM ps WHERE nt = 1 AND division = 'FBS' GROUP BY pid ORDER BY n DESC LIMIT 1), (SELECT COUNT(DISTINCT pid) FROM ps WHERE nt = 1 AND division = 'FBS'))"#,
        ),
        needs: W,
        open: "e/programs/{id}",
    },
    Panel {
        id: "C5.rolling_most",
        page: "programs",
        area: "C5",
        title: r#"Most FBS titles by one school in 15 years"#,
        kind: "line",
        note: r#"For each 15-season window ending that year: the most FBS titles one program won in it. Real band 4-6 (Alabama's 6 in 2006-2020 the most; `trial3_baselines.json`)."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1), t AS (SELECT pid, year FROM ps WHERE nt = 1 AND division = 'FBS'), e AS (SELECT DISTINCT year AS e FROM ps WHERE year >= (SELECT MIN(year) + 14 FROM ps))
         SELECT e.e AS year, (SELECT MAX(n) FROM (SELECT COUNT(*) AS n FROM t WHERE t.year BETWEEN e.e - 14 AND e.e GROUP BY pid)) AS most, 4 AS real_lo, 6 AS real_hi FROM e ORDER BY e.e"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C5.rolling_distinct",
        page: "programs",
        area: "C5",
        title: r#"Different FBS champions in 15 years"#,
        kind: "line",
        note: r#"For each 15-season window ending that year: how many different programs won FBS titles in it. Real band 7-9 (8 in 2011-2025; `trial3_baselines.json`)."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1), t AS (SELECT pid, year FROM ps WHERE nt = 1 AND division = 'FBS'), e AS (SELECT DISTINCT year AS e FROM ps WHERE year >= (SELECT MIN(year) + 14 FROM ps))
         SELECT e.e AS year, (SELECT COUNT(DISTINCT pid) FROM t WHERE t.year BETWEEN e.e - 14 AND e.e) AS distinct_champions, 7 AS real_lo, 9 AS real_hi FROM e ORDER BY e.e"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C5.by_program",
        page: "programs",
        area: "C5",
        title: r#"National titles by program"#,
        kind: "bars",
        note: r#"Programs with the most national titles in the world's seasons, by level (pick a level). No real line: the real all-time counts cover other years."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1) SELECT division AS grp, name AS label, COUNT(*) AS world, pid AS id FROM ps WHERE nt = 1 GROUP BY division, pid ORDER BY division, world DESC"#,
        says: None,
        needs: W,
        open: "e/programs/{id}",
    },
    Panel {
        id: "C5.lower_distinct",
        page: "programs",
        area: "C5",
        title: r#"Different champions per decade, every level"#,
        kind: "table",
        note: r#"Distinct national champions in each level per decade of the world's seasons. Real FCS 2011-2025: 4 different champions in 14 seasons (North Dakota State 9; `college_history.json` champions)."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1) SELECT division, (year / 10) * 10 AS decade, COUNT(*) AS titles, COUNT(DISTINCT pid) AS distinct_champions,
         (SELECT name FROM ps p2 WHERE p2.nt = 1 AND p2.division = ps.division AND p2.year / 10 = ps.year / 10 GROUP BY p2.pid ORDER BY COUNT(*) DESC LIMIT 1) AS most_by
         FROM ps WHERE nt = 1 GROUP BY division, year / 10 ORDER BY division, decade"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C6.list",
        page: "programs",
        area: "C6",
        title: r#"Every conference now"#,
        kind: "table",
        note: r#"Each conference in the world's last season: its level, members, the champion and the most titles in the world's seasons. A row opens the conference's page."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1), last AS (SELECT * FROM ps WHERE year = (SELECT MAX(year) FROM ps))
         SELECT l.conference, MIN(l.division) AS division, COUNT(*) AS members, (SELECT group_concat(name, ', ') FROM last x WHERE x.conference = l.conference AND x.ct = 1) AS champion,
         (SELECT name || ' (' || n || ')' FROM (SELECT name, SUM(ct) AS n FROM ps x WHERE x.conference = l.conference GROUP BY pid ORDER BY n DESC LIMIT 1)) AS most_titles,
         ROUND(AVG((SELECT prestige FROM programs WHERE _row = l.prog)), 3) AS prestige
         FROM last l GROUP BY l.conference ORDER BY CASE MIN(l.division) WHEN 'FBS' THEN 0 WHEN 'FCS' THEN 1 WHEN 'D2' THEN 2 WHEN 'D3' THEN 3 WHEN 'NAIA' THEN 4 ELSE 5 END, prestige DESC"#,
        says: None,
        needs: W,
        open: "conference/{conference}",
    },
    Panel {
        id: "C6.champions",
        page: "programs",
        area: "C6",
        title: r#"Conference champions, season by season"#,
        kind: "table",
        note: r#"Every conference title the world gave, by conference and season, with the champion's record. A conference opens its own page."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1) SELECT conference, year, pid AS id, name AS champion, w || '-' || l AS record, division FROM ps WHERE ct = 1 ORDER BY year DESC, division, conference"#,
        says: None,
        needs: W,
        open: "conference/{conference}",
    },
    Panel {
        id: "C6.most_15y",
        page: "programs",
        area: "C6",
        title: r#"Most conference titles by one school in 15 years"#,
        kind: "line",
        note: r#"For each 15-season window: the most FBS conference titles one program won (shared titles count). Real 2001-2025: 10 (Boise State 2001-2015, Alabama 2011-2025; `college_history.json` fbs_conference_champions)."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1), t AS (SELECT pid, year FROM ps WHERE ct = 1 AND division = 'FBS'), e AS (SELECT DISTINCT year AS e FROM ps WHERE year >= (SELECT MIN(year) + 14 FROM ps))
         SELECT e.e AS year, (SELECT MAX(n) FROM (SELECT COUNT(*) AS n FROM t WHERE t.year BETWEEN e.e - 14 AND e.e GROUP BY pid)) AS most, 10 AS real FROM e ORDER BY e.e"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C6.strength",
        page: "programs",
        area: "C6",
        title: r#"Conference strength grade by year, FBS conferences"#,
        kind: "line",
        note: r#"The conference strength grade (55-99) each year, averaged over each FBS conference's members of today (`grade_history`). Pick a conference above the chart."#,
        sql: r#"SELECT h._0 AS year, p.conference AS grp, ROUND(AVG(json_extract(h._1, '$[9]')), 1) AS strength FROM programs__grade_history h JOIN programs p ON p._row = h._parent
         WHERE p.division = 'FBS' GROUP BY h._0, p.conference ORDER BY p.conference, h._0"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C6.title_games",
        page: "programs",
        area: "C6",
        title: r#"Conference title games by conference (game log)"#,
        kind: "table",
        note: r#"The world's conference title games each season from the game log, against each conference's real first title game (SEC 1992, Big 12 1996-2010 and 2017 on, ACC 2005, MAC 1997, Big Ten 2011, Pac-12 2011, Mountain West 2013, American 2015, Sun Belt 2018; `calendar/rules.rs` conference_rules)."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1), tg AS (SELECT g.season AS year, json_extract(g.teams, '$[0]') AS home, json_extract(g.teams, '$[1]') AS away FROM log_games g WHERE g.stream = 'college' AND g.kind = 'conference title'),
         r(conf, first) AS (VALUES ('SEC', 1992), ('Big 12', 1996), ('ACC', 2005), ('MAC', 1997), ('Big Ten', 2011), ('Pac-12', 2011), ('Mountain West', 2013), ('American', 2015), ('Sun Belt', 2018), ('CUSA', 2005))
         SELECT ps.conference, MIN(tg.year) AS first_in_log, MAX(tg.year) AS last_in_log, COUNT(*) AS games, r.first AS real_first
         FROM tg JOIN ps ON ps.prog = tg.home AND ps.year = tg.year LEFT JOIN r ON r.conf = ps.conference GROUP BY ps.conference ORDER BY games DESC"#,
        says: None,
        needs: GL,
        open: "",
    },
    Panel {
        id: "C7.games_per_team",
        page: "programs",
        area: "C7",
        title: r#"Games per team each season by level (game log)"#,
        kind: "table",
        note: r#"Regular-season games each team played per season, from the game log: the mean, fewest and most. The rule: 12 in FBS (11 before 2006), 13 with a trip to Hawaii."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1), tg AS (SELECT g.season, json_extract(g.teams, '$[' || k.i || ']') AS t FROM log_games g, (SELECT 0 AS i UNION ALL SELECT 1) k
         WHERE g.stream = 'college' AND COALESCE(g.kind, 'regular') = 'regular'),
         c AS (SELECT season, t, COUNT(*) AS n FROM tg GROUP BY season, t)
         SELECT c.season AS year, ps.division, ROUND(AVG(c.n), 2) AS mean, MIN(c.n) AS fewest, MAX(c.n) AS most, COUNT(*) AS teams
         FROM c JOIN ps ON ps.prog = c.t AND ps.year = c.season GROUP BY c.season, ps.division ORDER BY year DESC, ps.division"#,
        says: None,
        needs: GL,
        open: "",
    },
    Panel {
        id: "C7.fcs_buy",
        page: "programs",
        area: "C7",
        title: r#"FBS teams that play an FCS team (game log)"#,
        kind: "line",
        note: r#"The share of FBS teams with at least one FCS opponent each season, against the real share (CFBD 2001-2025, `college_history.json`)."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1), x AS (SELECT g.season AS year, h.pid AS t, (SELECT division FROM ps WHERE prog = json_extract(g.teams, '$[1]') AND year = g.season) AS od FROM log_games g JOIN ps h ON h.prog = json_extract(g.teams, '$[0]') AND h.year = g.season
         WHERE g.stream = 'college' AND h.division = 'FBS'
         UNION ALL SELECT g.season, a.pid, (SELECT division FROM ps WHERE prog = json_extract(g.teams, '$[0]') AND year = g.season) FROM log_games g JOIN ps a ON a.prog = json_extract(g.teams, '$[1]') AND a.year = g.season
         WHERE g.stream = 'college' AND a.division = 'FBS')
         SELECT year, ROUND(100.0 * COUNT(DISTINCT CASE WHEN od = 'FCS' THEN t END) / COUNT(DISTINCT t), 1) AS world,
         100.0 * json_extract((SELECT json FROM real_data WHERE file = 'college_history.json'), '$.scores_by_season.by_season.' || year || '.FBS.fbs_teams_playing_fcs_share') AS real
         FROM x GROUP BY year ORDER BY year"#,
        says: None,
        needs: GL,
        open: "",
    },
    Panel {
        id: "C7.attendance",
        page: "programs",
        area: "C7",
        title: r#"FBS crowds a home game (game log)"#,
        kind: "line",
        note: r#"The mean crowd at FBS home games (not neutral sites) each season, against the real mean (CFBD 2001-2025, `college_attendance.csv`)."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1) SELECT g.season AS year, ROUND(AVG(g.attendance)) AS world, json_extract((SELECT json FROM real_data WHERE file = 'college_history.json'), '$.fbs_attendance.by_season.' || g.season || '.mean') AS real
         FROM log_games g JOIN ps ON ps.prog = json_extract(g.teams, '$[0]') AND ps.year = g.season
         WHERE g.stream = 'college' AND ps.division = 'FBS' AND COALESCE(g.neutral, 0) = 0 AND g.attendance > 0 GROUP BY g.season ORDER BY g.season"#,
        says: None,
        needs: GL,
        open: "",
    },
    Panel {
        id: "C7.days",
        page: "programs",
        area: "C7",
        title: r#"Which day FBS home games are played (game log)"#,
        kind: "bars",
        note: r#"FBS regular-season home games by day of the week (shares), against the real shares of 2021-2025 (`college_history.json` fbs_game_days)."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1), w AS (SELECT CAST(strftime('%w', g.date * 86400, 'unixepoch') AS INTEGER) AS d FROM log_games g JOIN ps ON ps.prog = json_extract(g.teams, '$[0]') AND ps.year = g.season
         WHERE g.stream = 'college' AND ps.division = 'FBS' AND COALESCE(g.kind, 'regular') = 'regular'),
         names(d, label) AS (VALUES (0, 'Sunday'), (1, 'Monday'), (2, 'Tuesday'), (3, 'Wednesday'), (4, 'Thursday'), (5, 'Friday'), (6, 'Saturday')),
         rj AS (SELECT j.value AS season FROM json_each(json_extract((SELECT json FROM real_data WHERE file = 'college_history.json'), '$.fbs_game_days.by_season')) j WHERE CAST(j.key AS INTEGER) BETWEEN 2021 AND 2025),
         rt AS (SELECT n.label, SUM(COALESCE(json_extract(rj.season, '$.' || n.label), 0)) AS c FROM rj, names n GROUP BY n.label)
         SELECT n.label, ROUND(100.0 * (SELECT COUNT(*) FROM w WHERE w.d = n.d) / (SELECT COUNT(*) FROM w), 2) AS world,
         ROUND(100.0 * (SELECT c FROM rt WHERE rt.label = n.label) / (SELECT SUM(c) FROM rt), 2) AS real FROM names n ORDER BY n.d"#,
        says: None,
        needs: GL,
        open: "",
    },
    Panel {
        id: "C8.deals",
        page: "programs",
        area: "C8",
        title: r#"Non-conference deals on the books"#,
        kind: "table",
        note: r#"Every game under contract (`World.deals`) by kind: how many, how far ahead they were signed, and the fee. Payouts and horizons are proposed, not confirmed (`world/deals.rs`)."#,
        sql: r#"SELECT kind, COUNT(*) AS deals, ROUND(AVG(year - signed), 1) AS years_ahead_mean, MIN(year - signed) AS fewest, MAX(year - signed) AS most,
         ROUND(AVG(fee)) AS fee_mean, MAX(fee) AS fee_max, MIN(year) AS first_season, MAX(year) AS last_season FROM deals GROUP BY kind ORDER BY deals DESC"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C9.series",
        page: "programs",
        area: "C9",
        title: r#"Rivalry series as the world carries them"#,
        kind: "table",
        note: r#"Each listed rivalry the world tracks (`college.series`): its all-time wins and ties as loaded from real history and carried on, and who holds the trophy now."#,
        sql: r#"WITH r AS (SELECT _0 AS i, _1 AS a, CASE WHEN json_extract(_key, '$[0]') = _1 THEN json_extract(_key, '$[1]') ELSE json_extract(_key, '$[0]') END AS b FROM college__rivalry_of)
         SELECT r.a AS id, COALESCE(pa.real_name, pa.fictional_name) AS team, COALESCE(pb.real_name, pb.fictional_name) AS rival, json_extract(s.wins, '$[0]') || '-' || json_extract(s.wins, '$[1]') || CASE WHEN s.ties > 0 THEN '-' || s.ties ELSE '' END AS series,
         s.since, (SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = s.holder) AS trophy_holder, CASE WHEN s.real = 1 THEN 'real record loaded' ELSE 'world only' END AS source
         FROM college__series s JOIN r ON r.i = s._row JOIN programs pa ON pa._row = r.a JOIN programs pb ON pb._row = r.b ORDER BY json_extract(s.wins, '$[0]') + json_extract(s.wins, '$[1]') DESC"#,
        says: None,
        needs: W,
        open: "e/programs/{id}",
    },
    Panel {
        id: "C9.games",
        page: "programs",
        area: "C9",
        title: r#"Rivalry games each season (game log)"#,
        kind: "table",
        note: r#"Every rivalry game the game log tagged, newest first, with the score."#,
        sql: r#"SELECT g.season AS year, g.date, json_extract(g.teams, '$[0]') AS home, (SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = json_extract(g.teams, '$[0]')) AS home_name,
         (SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = json_extract(g.teams, '$[1]')) AS away_name, g.score, g.name
         FROM log_games g WHERE g.stream = 'college' AND COALESCE(g.rivalry, 0) != 0 ORDER BY g.season DESC, g.date"#,
        says: None,
        needs: GL,
        open: "",
    },
    Panel {
        id: "C10.final",
        page: "programs",
        area: "C10",
        title: r#"The last full season's final AP poll"#,
        kind: "table",
        note: r#"The final AP top 25 of the last finished season (`college_prev.polls`), with points and first-place votes."#,
        sql: r#"WITH p AS (SELECT * FROM college_prev__polls WHERE kind LIKE 'Ap%' ORDER BY date DESC LIMIT 1)
         SELECT t.key + 1 AS rank, t.value AS id, (SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = t.value) AS team,
         (SELECT wins || '-' || losses FROM programs__seasons s WHERE s._parent = t.value AND s.year = (SELECT year FROM college_prev)) AS record,
         json_extract(p.points, '$[' || t.key || ']') AS points, json_extract(p.firsts, '$[' || t.key || ']') AS first_place
         FROM p, json_each(p.top) t ORDER BY t.key"#,
        says: None,
        needs: W,
        open: "e/programs/{id}",
    },
    Panel {
        id: "C10.preseason_vs_final",
        page: "programs",
        area: "C10",
        title: r#"Preseason AP rank against the season's win rate"#,
        kind: "bars",
        note: r#"The last full season: each preseason AP band's win rate against the real bands (AP preseason top 25, 2004-2025: 1-5 .825, 6-10 .716, 11-15 .677, 16-25 .641; `college_history.json`)."#,
        sql: r#"WITH pre AS (SELECT t.key + 1 AS rank, t.value AS prog FROM college_prev__polls p, json_each(p.top) t
         WHERE p._row = (SELECT _row FROM college_prev__polls WHERE kind LIKE 'Ap%' ORDER BY date LIMIT 1)),
         y AS (SELECT year AS y FROM college_prev),
         x AS (SELECT CASE WHEN rank <= 5 THEN '1-5' WHEN rank <= 10 THEN '6-10' WHEN rank <= 15 THEN '11-15' ELSE '16-25' END AS band, 1.0 * s.wins / (s.wins + s.losses) AS wp
         FROM pre JOIN programs__seasons s ON s._parent = pre.prog AND s.year = (SELECT y FROM y))
         SELECT band AS label, ROUND(AVG(wp), 3) AS world, json_extract((SELECT json FROM real_data WHERE file = 'college_history.json'), '$.preseason_rank_vs_season.win_pct_by_rank_band."' || band || '"') AS real, COUNT(*) AS n
         FROM x GROUP BY band ORDER BY MIN(CASE band WHEN '1-5' THEN 1 WHEN '6-10' THEN 2 WHEN '11-15' THEN 3 ELSE 4 END)"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C11.panels",
        page: "programs",
        area: "C11",
        title: r#"Poll panels and the committee"#,
        kind: "bars",
        note: r#"How many voters sit on each panel now, against the real sizes (AP 62, Coaches 65, committee 13; `world/voters.rs`). The Harris poll (114 voters) ran 2005-2013 only."#,
        sql: r#"SELECT 'AP' AS label, (SELECT COUNT(*) FROM voters__ap) AS world, 62 AS real
         UNION ALL SELECT 'Coaches', (SELECT COUNT(*) FROM voters__coaches), 65 UNION ALL SELECT 'Committee', (SELECT COUNT(*) FROM voters__committee), 13"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C12.bowls",
        page: "programs",
        area: "C12",
        title: r#"FBS bowl and playoff games each season (game log)"#,
        kind: "line",
        note: r#"FBS bowls, playoff games and title games each season in the game log, against the real count (CFBD 2001-2025, `college_history.json` postseason)."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1) SELECT g.season AS year, COUNT(*) AS world, json_extract((SELECT json FROM real_data WHERE file = 'college_history.json'), '$.postseason.by_season.' || g.season || '.fbs_bowl_and_playoff_games') AS real
         FROM log_games g JOIN ps ON ps.prog = json_extract(g.teams, '$[0]') AND ps.year = g.season
         WHERE g.stream = 'college' AND ps.division = 'FBS' AND g.kind IN ('bowl', 'playoff', 'title') GROUP BY g.season ORDER BY g.season"#,
        says: None,
        needs: GL,
        open: "",
    },
    Panel {
        id: "C12.appearances",
        page: "programs",
        area: "C12",
        title: r#"Bowl and playoff appearances by program (game log)"#,
        kind: "table",
        note: r#"Each FBS program's bowl and playoff games and record in the seasons the game log holds."#,
        sql: r#"WITH x AS (SELECT json_extract(g.teams, '$[' || k.i || ']') AS t, g.kind, (json_extract(g.score, '$[' || k.i || ']') > json_extract(g.score, '$[' || (1 - k.i) || ']')) AS won
         FROM log_games g, (SELECT 0 AS i UNION ALL SELECT 1) k WHERE g.stream = 'college' AND g.kind IN ('bowl', 'playoff', 'title'))
         SELECT p.id, COALESCE(p.real_name, p.fictional_name) AS name, p.division, SUM(kind = 'bowl') AS bowls, SUM(kind IN ('playoff', 'title')) AS playoff_games, SUM(won) || '-' || SUM(1 - won) AS record
         FROM x JOIN programs p ON p._row = x.t GROUP BY x.t ORDER BY bowls + playoff_games DESC LIMIT 60"#,
        says: None,
        needs: GL,
        open: "e/programs/{id}",
    },
    Panel {
        id: "C13.fields",
        page: "programs",
        area: "C13",
        title: r#"Playoff fields by level (game log)"#,
        kind: "table",
        note: r#"Playoff games and teams in each level's bracket each season from the game log. Real fields: FCS 24 from 2013 (12 before), D3 32 from 2005, NAIA 16 then 20 from 2022, JUCO 2 then 4 from 2021."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1), x AS (SELECT g.season AS year, ps.division, json_extract(g.teams, '$[0]') AS h, json_extract(g.teams, '$[1]') AS a FROM log_games g JOIN ps ON ps.prog = json_extract(g.teams, '$[0]') AND ps.year = g.season
         WHERE g.stream = 'college' AND g.kind IN ('playoff', 'title'))
         SELECT year, division, COUNT(*) AS games, (SELECT COUNT(DISTINCT t) FROM (SELECT h AS t FROM x x2 WHERE x2.year = x.year AND x2.division = x.division UNION SELECT a FROM x x2 WHERE x2.year = x.year AND x2.division = x.division)) AS teams
         FROM x GROUP BY year, division ORDER BY year DESC, division"#,
        says: None,
        needs: GL,
        open: "",
    },
    Panel {
        id: "C15.ledger",
        page: "programs",
        area: "C15",
        title: r#"Rosters and signees against the limits, FBS"#,
        kind: "line",
        note: r#"Each season's FBS mean roster and signees against the season's limit (`ProgramEra.ledger`): 85 scholarships with 25 a year (95 before 1992), the 105 roster from 2025."#,
        sql: r#"SELECT l.season AS year, ROUND(AVG(l.roster), 1) AS roster, ROUND(AVG(l."limit"), 1) AS roster_limit, ROUND(AVG(l.signees), 1) AS signees
         FROM programs__era__ledger l JOIN programs p ON p._row = l._parent WHERE p.division = 'FBS' GROUP BY l.season ORDER BY l.season"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C15.cases",
        page: "programs",
        area: "C15",
        title: r#"NCAA cases (rogue boosters found)"#,
        kind: "table",
        note: r#"Every NCAA case the world opened: the program, when, what was found and the penalty (`ProgramEra.cases`). Real: a few major football cases a decade (from memory)."#,
        sql: r#"SELECT p.id, COALESCE(p.real_name, p.fictional_name) AS program, p.division, c.* FROM programs__era__cases c JOIN programs p ON p._row = c._parent ORDER BY 4 DESC"#,
        says: None,
        needs: W,
        open: "e/programs/{id}",
    },
    Panel {
        id: "C15.bans",
        page: "programs",
        area: "C15",
        title: r#"Programs under NCAA penalties now"#,
        kind: "table",
        note: r#"Programs with scholarships cut or a postseason ban running (`ProgramEra.cut_until`, `ban_until`)."#,
        sql: r#"SELECT p.id, COALESCE(p.real_name, p.fictional_name) AS program, p.division, p.era__cut AS scholarships_cut, p.era__cut_until AS cut_until, p.era__ban_until AS ban_until
         FROM programs p WHERE p.era__cut_until >= (SELECT year FROM world) OR p.era__ban_until >= (SELECT year FROM world) ORDER BY p.era__ban_until DESC"#,
        says: None,
        needs: W,
        open: "e/programs/{id}",
    },
    Panel {
        id: "C16.roads",
        page: "programs",
        area: "C16",
        title: r#"The long roads each year"#,
        kind: "line",
        note: r#"The calendar's yearly counters: four-game redshirts, redshirt tricks, medical hardship waivers, sixth years, COVID returns and declines, (`CalendarLog`). Older world files lack some."#,
        sql: r#"SELECT year, four_game_redshirts, redshirt_tricks, medical_hardships, sixth_years, covid_returns, covid_declined FROM log_history ORDER BY year"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C16.last_age",
        page: "programs",
        area: "C16",
        title: r#"Age in a player's last college season"#,
        kind: "hist",
        note: r#"The age of every pro and alumnus in his last college season (his season rows): most real players finish at 21-23."#,
        sql: r#"WITH x AS (SELECT MAX(h.age) AS a FROM people__player__history h JOIN people p ON p._row = h._parent WHERE h.level IN ('FBS', 'FCS', 'D2', 'D3', 'NAIA', 'JUCO', 'College') AND p.stage = 'Pro' GROUP BY h._parent)
         SELECT a AS x, COUNT(*) AS world FROM x GROUP BY a ORDER BY a"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C17.bars",
        page: "programs",
        area: "C17",
        title: r#"Admissions bars and the classes they sign"#,
        kind: "table",
        note: r#"The programs with the strictest admissions (Stanford 3.6, Northwestern 3.5, Notre Dame 3.4 are proposed, not confirmed; `program.rs` STRICT_ADMISSIONS): their bar, special admits, their college players' GPA at signing and stars, against their level."#,
        sql: r#"SELECT p.id, COALESCE(p.real_name, p.fictional_name) AS name, p.division, ROUND(p.admissions_bar, 2) AS bar, p.special_admits, ROUND(x.gpa_at_signing, 2) AS gpa, ROUND(x.stars, 2) AS stars,
         x.n AS players FROM programs p LEFT JOIN (SELECT college, AVG(gpa_at_signing) AS gpa_at_signing, AVG(stars) AS stars, COUNT(*) AS n FROM people WHERE stage = 'College' GROUP BY college) x ON x.college = p._row
         ORDER BY p.admissions_bar DESC LIMIT 40"#,
        says: None,
        needs: W,
        open: "e/programs/{id}",
    },
    Panel {
        id: "C17.tiers",
        page: "programs",
        area: "C17",
        title: r#"Academic redshirts and GPA by level"#,
        kind: "bars",
        note: r#"The share of today's college players who were academic redshirts at signing, by level, and their mean GPA at signing."#,
        sql: r#"SELECT p.division AS label, ROUND(100.0 * AVG(COALESCE(x.academic_redshirt, 0)), 2) AS world, ROUND(AVG(x.gpa_at_signing), 2) AS gpa, COUNT(*) AS n
         FROM people x JOIN programs p ON p._row = x.college WHERE x.stage = 'College' GROUP BY p.division ORDER BY n DESC"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C17.terms",
        page: "programs",
        area: "C17",
        title: r#"Ineligible players and grade-outs each year"#,
        kind: "line",
        note: r#"Players ruled ineligible at the spring and fall term checks, and high school grade-outs, each year (`CalendarLog`)."#,
        sql: r#"SELECT year, spring_ineligible, fall_ineligible, hs_grade_outs FROM log_history ORDER BY year"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C19.tenure",
        page: "programs",
        area: "C19",
        title: r#"How long FBS head coaches last"#,
        kind: "bars",
        note: r#"Head coach runs at FBS programs (consecutive seasons as the season's coach) that began after the history's first season and early enough to have run 10: the share lasting at least 2, 3, 4, 5, 6, 8 and 10 seasons, against the real shares (FBS head coaches who started 2005-2019, ESPN; `college_people.json`)."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1), r AS (SELECT pid, coach, MIN(year) AS a, MAX(year) AS z, COUNT(*) AS n FROM (SELECT pid, coach, year, year - ROW_NUMBER() OVER (PARTITION BY pid, coach ORDER BY year) AS run FROM ps WHERE division = 'FBS' AND coach IS NOT NULL) GROUP BY pid, coach, run),
         ok AS (SELECT * FROM r WHERE a > (SELECT MIN(year) FROM ps) AND a <= (SELECT MAX(year) FROM ps) - 10),
         k(k) AS (VALUES (2), (3), (4), (5), (6), (8), (10))
         SELECT 'At least ' || k.k || ' seasons' AS label, ROUND(100.0 * (SELECT AVG(n >= k.k) FROM ok), 1) AS world,
         ROUND(100.0 * json_extract((SELECT json FROM real_data WHERE file = 'college_people.json'), '$.fbs_head_coaches.share_lasting_at_least."' || k.k || '"'), 1) AS real, (SELECT COUNT(*) FROM ok) AS n FROM k"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C19.ads",
        page: "programs",
        area: "C19",
        title: r#"Athletic directors (debug)"#,
        kind: "table",
        note: r#"Each program's athletic director, when he started, and his hidden judgement and patience (1-20) that set the bar a head coach must clear."#,
        sql: r#"SELECT p.id, COALESCE(p.real_name, p.fictional_name) AS program, p.division, p.ad__name AS ad, p.ad__since AS since, ROUND(p.ad__judgement, 1) AS judgement, ROUND(p.ad__patience, 1) AS patience
         FROM programs p WHERE p.ad__name IS NOT NULL ORDER BY CASE p.division WHEN 'FBS' THEN 0 WHEN 'FCS' THEN 1 ELSE 2 END, p.prestige DESC"#,
        says: None,
        needs: W,
        open: "e/programs/{id}",
    },
    Panel {
        id: "C20.by_position",
        page: "programs",
        area: "C20",
        title: r#"FBS players a roster by position"#,
        kind: "bars",
        note: r#"College players at FBS programs today per program, by position, against the real FBS rosters (ESPN 2022-24, about 118 listed; `recruiting.rs` real_roster)."#,
        sql: r#"WITH r(pos, v) AS (VALUES ('QB', 5.8), ('RB', 8.5), ('WR', 16.2), ('TE', 7.4), ('OT', 9.36), ('IOL', 10.14), ('EDGE', 9.1), ('DT', 8.4), ('LB', 14.6), ('CB', 10.21), ('S', 11.99), ('K', 3.5), ('P', 2.0), ('LS', 2.5)),
         t AS (SELECT COUNT(DISTINCT x.college) AS n FROM people x JOIN programs p ON p._row = x.college WHERE x.stage = 'College' AND p.division = 'FBS')
         SELECT r.pos AS label, ROUND(1.0 * COUNT(x._row) / (SELECT n FROM t), 2) AS world, r.v AS real FROM r
         LEFT JOIN people x ON x.player__position = r.pos AND x.stage = 'College' AND x.college IN (SELECT _row FROM programs WHERE division = 'FBS')
         GROUP BY r.pos ORDER BY real DESC"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C20.sizes",
        page: "programs",
        area: "C20",
        title: r#"Roster sizes by level"#,
        kind: "table",
        note: r#"College players on each program's roster today, by level: the median, the smallest and largest, and how many have no kicker or no punter. Real FBS: about 105 under the 2025 limit (118 listed 2022-24); no FBS team without a kicker, 4-6% without a punter, a quarter of FCS teams without one."#,
        sql: r#"WITH x AS (SELECT college, COUNT(*) AS n, SUM(player__position = 'K') AS k, SUM(player__position = 'P') AS pu FROM people WHERE stage = 'College' GROUP BY college),
         r AS (SELECT p._row, p.division, COALESCE(x.n, 0) AS n, COALESCE(x.k, 0) AS k, COALESCE(x.pu, 0) AS pu FROM programs p LEFT JOIN x ON x.college = p._row
         WHERE p._row IN (SELECT _parent FROM programs__seasons WHERE year = (SELECT MAX(year) FROM programs__seasons))),
         o AS (SELECT *, ROW_NUMBER() OVER (PARTITION BY division ORDER BY n) AS rk, COUNT(*) OVER (PARTITION BY division) AS cnt FROM r)
         SELECT division, MAX(CASE WHEN rk = (cnt + 1) / 2 THEN n END) AS median, MIN(n) AS smallest, MAX(n) AS largest, ROUND(100.0 * AVG(k = 0), 1) AS pct_no_kicker,
         ROUND(100.0 * AVG(pu = 0), 1) AS pct_no_punter, COUNT(*) AS programs FROM o GROUP BY division ORDER BY programs DESC"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C20.blue_chips",
        page: "programs",
        area: "C20",
        title: r#"Blue chips by FBS conference"#,
        kind: "bars",
        note: r#"The share of 4- and 5-star players on each FBS conference's rosters today, against the real blue-chip ratio of the last season in the 247Sports team talent composite (`college_history.json` talent_by_conference)."#,
        sql: r#"WITH last AS (SELECT MAX(CAST(key AS INTEGER)) AS y FROM json_each(json_extract((SELECT json FROM real_data WHERE file = 'college_history.json'), '$.talent_by_conference.by_season')))
         SELECT p.conference AS label, ROUND(100.0 * AVG(x.stars >= 4), 1) AS world,
         ROUND(100.0 * json_extract((SELECT json FROM real_data WHERE file = 'college_history.json'), '$.talent_by_conference.by_season.' || (SELECT y FROM last) || '."' || p.conference || '".blue_chip_ratio'), 1) AS real, COUNT(*) AS n
         FROM people x JOIN programs p ON p._row = x.college WHERE x.stage = 'College' AND p.division = 'FBS' GROUP BY p.conference ORDER BY world DESC"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C21.best_22",
        page: "programs",
        area: "C21",
        title: r#"The strongest rosters: each FBS program's best 22"#,
        kind: "table",
        note: r#"The mean true overall of each FBS program's 22 best college players today (debug: true ratings), with the next 22 and the FBS rank: how far the top sits above the field."#,
        sql: r#"WITH r AS (SELECT x.college AS prog, x.x_overall AS o, ROW_NUMBER() OVER (PARTITION BY x.college ORDER BY x.x_overall DESC) AS rk FROM people x WHERE x.stage = 'College'),
         t AS (SELECT prog, AVG(CASE WHEN rk <= 22 THEN o END) AS top22, AVG(CASE WHEN rk BETWEEN 23 AND 44 THEN o END) AS next22 FROM r GROUP BY prog)
         SELECT p.id, COALESCE(p.real_name, p.fictional_name) AS name, p.conference, ROUND(t.top22, 1) AS best_22, ROUND(t.next22, 1) AS next_22,
         ROUND(t.top22 - (SELECT AVG(top22) FROM t JOIN programs q ON q._row = t.prog WHERE q.division = 'FBS'), 1) AS over_fbs_mean
         FROM t JOIN programs p ON p._row = t.prog WHERE p.division = 'FBS' ORDER BY t.top22 DESC LIMIT 40"#,
        says: None,
        needs: W,
        open: "e/programs/{id}",
    },
    Panel {
        id: "C22.picks",
        page: "programs",
        area: "C22",
        title: r#"Draft picks by program, the last ten drafts"#,
        kind: "bars",
        note: r#"Each program's draft picks over the world's last ten drafts (`Program.draft_picks`), against its real picks over the last ten real drafts (nflverse, `nfl_draft_by_college.csv`)."#,
        sql: r#"WITH w AS (SELECT d._parent AS prog, SUM(d._1) AS n FROM programs__draft_picks d WHERE d._0 > (SELECT MAX(_0) FROM programs__draft_picks) - 10 GROUP BY d._parent),
         ry AS (SELECT MAX(season) AS y FROM real_nfl_draft_by_college)
         SELECT COALESCE(p.real_name, p.fictional_name) AS label, w.n AS world,
         (SELECT SUM(r.picks) FROM real_nfl_draft_by_college r WHERE r.season > (SELECT y FROM ry) - 10 AND (r.college = p.real_name OR r.college = REPLACE(p.real_name, ' State', ' St.'))) AS real, p.id
         FROM w JOIN programs p ON p._row = w.prog ORDER BY w.n DESC LIMIT 30"#,
        says: None,
        needs: W,
        open: "e/programs/{id}",
    },
    Panel {
        id: "C22.picks_by_conference",
        page: "programs",
        area: "C22",
        title: r#"Draft picks by conference, draft by draft"#,
        kind: "table",
        note: r#"Draft picks per draft by today's conference of the program, from `Program.draft_picks`."#,
        sql: r#"SELECT d._0 AS year, p.division, p.conference, SUM(d._1) AS picks, COUNT(*) AS programs FROM programs__draft_picks d JOIN programs p ON p._row = d._parent
         GROUP BY d._0, p.conference ORDER BY d._0 DESC, picks DESC"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C22.records",
        page: "programs",
        area: "C22",
        title: r#"School single-season records"#,
        kind: "table",
        note: r#"Each FBS program's season records (passing, rushing and receiving yards, sacks; `Program.season_records`), the best first. Real FBS records (from memory, not checked): passing 5,833, rushing 2,628, receiving 2,060, sacks 24."#,
        sql: r#"WITH k(i, what) AS (VALUES (0, 'Passing yards'), (1, 'Rushing yards'), (2, 'Receiving yards'), (3, 'Sacks'))
         SELECT k.what, p.id, COALESCE(p.real_name, p.fictional_name) AS program, r.value, r.holder, r.year FROM programs__season_records r JOIN programs p ON p._row = r._parent JOIN k ON k.i = r._i
         WHERE p.division = 'FBS' ORDER BY k.i, r.value DESC"#,
        says: None,
        needs: W,
        open: "e/programs/{id}",
    },
    Panel {
        id: "C42.kinds",
        page: "programs",
        area: "C42",
        title: r#"News by kind"#,
        kind: "table",
        note: r#"Every news item the world keeps, by kind: how many, from when to when, and a year's mean."#,
        sql: r#"SELECT kind, COUNT(*) AS items, MIN(year) AS first_year, MAX(year) AS last_year, ROUND(1.0 * COUNT(*) / (MAX(year) - MIN(year) + 1), 1) AS a_year FROM news GROUP BY kind ORDER BY items DESC"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C42.feed",
        page: "programs",
        area: "C42",
        title: r#"The latest news about programs"#,
        kind: "table",
        note: r#"The 500 latest news items that name a program, newest first; a row opens the program."#,
        sql: r#"SELECT n.year, n.date, n.kind, n.program AS id, COALESCE(p.real_name, p.fictional_name) AS program, n.text FROM news n JOIN programs p ON p._row = n.program ORDER BY n.year DESC, n.date DESC LIMIT 500"#,
        says: None,
        needs: W,
        open: "e/programs/{id}",
    },
    Panel {
        id: "T13.moves",
        page: "programs",
        area: "T13",
        title: r#"Every move between levels"#,
        kind: "table",
        note: r#"Each program's moves from one level to another, from its real history through 2025 and the world's own years after (a season at a new level, or no team)."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1), x AS (SELECT pid, name, year, division, LAG(division) OVER (PARTITION BY pid ORDER BY year) AS before, LAG(year) OVER (PARTITION BY pid ORDER BY year) AS prev_year FROM ps)
         SELECT year, pid AS id, name, COALESCE(before, '') AS from_level, division AS to_level, CASE WHEN year - prev_year > 1 THEN 'after ' || (year - prev_year - 1) || ' years with no team' ELSE '' END AS gap
         FROM x WHERE before IS NOT NULL AND (before != division OR year - prev_year > 1) ORDER BY year DESC"#,
        says: None,
        needs: W,
        open: "e/programs/{id}",
    },
    Panel {
        id: "T13.per_decade",
        page: "programs",
        area: "T13",
        title: r#"Moves up a level per decade"#,
        kind: "bars",
        note: r#"Programs moving up a level (D3 to D2, D2 to FCS, FCS to FBS) per decade of seasons, the world against real (2004-2025: 21 FCS to FBS, 21 D2 or D3 up; `college_history.json` division_moves_up)."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1), x AS (SELECT year, division, LAG(division) OVER (PARTITION BY pid ORDER BY year) AS before FROM ps),
         rank(d, r) AS (VALUES ('JUCO', 0), ('NAIA', 1), ('D3', 1), ('D2', 2), ('FCS', 3), ('FBS', 4)),
         up AS (SELECT (year / 10) * 10 AS decade, before || ' to ' || division AS move FROM x JOIN rank a ON a.d = x.before JOIN rank b ON b.d = x.division WHERE b.r > a.r),
         real AS (SELECT (json_extract(j.value, '$[0]') / 10) * 10 AS decade, json_extract(j.value, '$[2]') || ' to ' || json_extract(j.value, '$[3]') AS move FROM json_each(json_extract((SELECT json FROM real_data WHERE file = 'college_history.json'), '$.division_moves_up.moves')) j)
         SELECT decade || 's: ' || move AS label, (SELECT COUNT(*) FROM up u WHERE u.decade = m.decade AND u.move = m.move) AS world, (SELECT COUNT(*) FROM real r WHERE r.decade = m.decade AND r.move = m.move) AS real
         FROM (SELECT decade, move FROM up UNION SELECT decade, move FROM real) m ORDER BY decade, move"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "T13.movers",
        page: "programs",
        area: "T13",
        title: r#"Movers to FBS: their first three seasons"#,
        kind: "table",
        note: r#"Each program's first three seasons after moving up to FBS: record and win rate. Real movers often struggle at first, with exceptions (App State, James Madison)."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1), x AS (SELECT pid, name, year, division, LAG(division) OVER (PARTITION BY pid ORDER BY year) AS before FROM ps),
         m AS (SELECT pid, name, year AS moved FROM x WHERE division = 'FBS' AND before IS NOT NULL AND before != 'FBS')
         SELECT m.moved AS year, m.pid AS id, m.name, SUM(ps.w) || '-' || SUM(ps.l) AS first_three, ROUND(1.0 * SUM(ps.w) / NULLIF(SUM(ps.w + ps.l), 0), 3) AS pct
         FROM m JOIN ps ON ps.pid = m.pid AND ps.year BETWEEN m.moved AND m.moved + 2 GROUP BY m.pid, m.moved ORDER BY m.moved DESC"#,
        says: None,
        needs: W,
        open: "e/programs/{id}",
    },
    Panel {
        id: "C14.budgets",
        page: "money",
        area: "C14",
        title: r#"NIL collective budgets by level"#,
        kind: "table",
        note: r#"Each level's collective NIL budgets today (`Program.collective`): the median, the top tenth and the biggest, booster bases and big boosters. Real 2024-25: the biggest P4 collectives near $20 million a year; none before July 2021."#,
        sql: r#"WITH r AS (SELECT p.division, p.collective__budget AS v, p.collective__base AS base, p.collective__big_booster AS big, ROW_NUMBER() OVER (PARTITION BY p.division ORDER BY p.collective__budget) AS rk,
         COUNT(*) OVER (PARTITION BY p.division) AS n FROM programs p)
         SELECT division, MAX(CASE WHEN rk = (n + 1) / 2 THEN v END) AS median, MAX(CASE WHEN rk = CAST(0.9 * (n - 1) AS INTEGER) + 1 THEN v END) AS p90, MAX(v) AS biggest,
         ROUND(AVG(base), 3) AS booster_base, SUM(big IS NOT NULL) AS big_boosters, MAX(n) AS programs FROM r GROUP BY division ORDER BY biggest DESC"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C14.by_year",
        page: "money",
        area: "C14",
        title: r#"NIL budgets and boosters by year, FBS"#,
        kind: "line",
        note: r#"The FBS mean collective budget (in $ millions) and booster money each season from the ledger (`ProgramEra.ledger`); zero before the NIL era (July 2021)."#,
        sql: r#"SELECT l.season AS year, ROUND(AVG(l.nil_budget) / 1e6, 2) AS nil_budget_m, ROUND(AVG(l.boosters), 3) AS boosters FROM programs__era__ledger l JOIN programs p ON p._row = l._parent
         WHERE p.division = 'FBS' GROUP BY l.season ORDER BY l.season"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C14.trend",
        page: "money",
        area: "C14",
        title: r#"Collectives growing, steady, falling"#,
        kind: "bars",
        note: r#"How many collectives are growing, steady, falling or lost a big booster, and the programs drying up (`Collective.trend`, `drying_years`)."#,
        sql: r#"SELECT p.division || ': ' || COALESCE(p.collective__trend, '?') AS label, COUNT(*) AS world, ROUND(AVG(p.collective__drying_years), 2) AS drying_years FROM programs p GROUP BY p.division, p.collective__trend ORDER BY p.division, world DESC"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C14.top",
        page: "money",
        area: "C14",
        title: r#"The biggest collectives"#,
        kind: "table",
        note: r#"The 30 biggest NIL collective budgets today with their trend, booster base and big booster."#,
        sql: r#"SELECT p.id, COALESCE(p.real_name, p.fictional_name) AS name, p.division, p.conference, ROUND(p.collective__budget / 1e6, 2) AS budget_m, p.collective__trend AS trend,
         ROUND(p.collective__base, 3) AS base, p.collective__big_booster AS big_booster FROM programs p ORDER BY p.collective__budget DESC LIMIT 30"#,
        says: None,
        needs: W,
        open: "e/programs/{id}",
    },
    Panel {
        id: "conf.says",
        page: "conference",
        area: "C6",
        title: r#"The conference in a line"#,
        kind: "table",
        note: r#"One line per conference for its page's verdict."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1), last AS (SELECT * FROM ps WHERE year = (SELECT MAX(year) FROM ps))
         SELECT l.conference, printf('%s: %d members in %d (%s); champion %s; %s won the most titles in the world''s seasons.', l.conference, COUNT(*), MAX(l.year), MIN(l.division),
         COALESCE((SELECT group_concat(name, ' and ') FROM last x WHERE x.conference = l.conference AND x.ct = 1), 'none'),
         (SELECT name || ' (' || n || ')' FROM (SELECT name, SUM(ct) AS n FROM ps x WHERE x.conference = l.conference GROUP BY pid ORDER BY n DESC LIMIT 1))) AS says
         FROM last l GROUP BY l.conference"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "conf.members",
        page: "conference",
        area: "C6",
        title: r#"Members now"#,
        kind: "table",
        note: r#"The conference's programs today with prestige, titles and the last season's record."#,
        sql: r#"SELECT p.conference, p.id, COALESCE(p.real_name, p.fictional_name) AS program, p.division, ROUND(p.prestige, 3) AS prestige, p.titles AS national_titles, p.conference_titles,
         (SELECT s.wins || '-' || s.losses FROM programs__seasons s WHERE s._parent = +p._row ORDER BY s.year DESC LIMIT 1) AS last_season
         FROM programs p ORDER BY p.conference, p.prestige DESC"#,
        says: None,
        needs: W,
        open: "e/programs/{id}",
    },
    Panel {
        id: "conf.standings",
        page: "conference",
        area: "C6",
        title: r#"The last season's standings"#,
        kind: "table",
        note: r#"Each member's record in the world's last season, best first; its conference title marked."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1) SELECT conference, pid AS id, name AS program, w || '-' || l AS record, ROUND(1.0 * w / NULLIF(w + l, 0), 3) AS pct, CASE WHEN nt = 1 THEN 'national champion' WHEN ct = 1 THEN 'champion' ELSE '' END AS title
         FROM ps WHERE year = (SELECT MAX(year) FROM ps) ORDER BY conference, pct DESC"#,
        says: None,
        needs: W,
        open: "e/programs/{id}",
    },
    Panel {
        id: "conf.champions",
        page: "conference",
        area: "C6",
        title: r#"Champions by season"#,
        kind: "table",
        note: r#"Every conference title by season (shared titles each listed)."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1) SELECT conference, year, pid AS id, name AS champion, w || '-' || l AS record FROM ps WHERE ct = 1 ORDER BY conference, year DESC"#,
        says: None,
        needs: W,
        open: "e/programs/{id}",
    },
    Panel {
        id: "conf.titles",
        page: "conference",
        area: "C6",
        title: r#"Titles by member"#,
        kind: "bars",
        note: r#"Conference titles each program won while in the conference, in the world's seasons."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1) SELECT conference, name AS label, SUM(ct) AS world, pid AS id FROM ps GROUP BY conference, pid HAVING SUM(ct) > 0 ORDER BY conference, world DESC"#,
        says: None,
        needs: W,
        open: "e/programs/{id}",
    },
    Panel {
        id: "conf.members_by_year",
        page: "conference",
        area: "C6",
        title: r#"Members season by season"#,
        kind: "table",
        note: r#"Who played in the conference each season (real history through 2025, the world's after): the count and the names."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1) SELECT conference, year, COUNT(*) AS members, group_concat(name, ', ') AS programs FROM ps GROUP BY conference, year ORDER BY conference, year DESC"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "conf.strength",
        page: "conference",
        area: "C6",
        title: r#"Strength grade by year"#,
        kind: "line",
        note: r#"The conference strength grade (55-99) each year, averaged over today's members (`grade_history`)."#,
        sql: r#"SELECT p.conference, h._0 AS year, ROUND(AVG(json_extract(h._1, '$[9]')), 1) AS strength, ROUND(AVG(h2._1), 3) AS prestige FROM programs__grade_history h JOIN programs p ON p._row = h._parent
         LEFT JOIN programs__prestige_history h2 ON h2._parent = h._parent AND h2._0 = h._0 GROUP BY p.conference, h._0 ORDER BY p.conference, h._0"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C23.per_state",
        page: "high-school",
        area: "C23",
        title: r#"High schools per state against the real count"#,
        kind: "map",
        note: r#"Football-playing high schools in each state against the real count (MaxPreps 2026, `hs_states.csv` approx_programs). Hover a state for both."#,
        sql: r#"SELECT st.code AS state, COUNT(h._row) AS world, r.approx_programs AS real, COUNT(h._row) AS n
         FROM states st LEFT JOIN high_schools h ON h.state = st._row LEFT JOIN real_hs_states r ON r.state = st.code GROUP BY st._row ORDER BY st.code"#,
        says: Some(
            r#"WITH x AS (SELECT st.code, COUNT(h._row) AS w, r.approx_programs AS r FROM states st LEFT JOIN high_schools h ON h.state = st._row LEFT JOIN real_hs_states r ON r.state = st.code GROUP BY st._row)
         SELECT printf('%,d high schools against about %,d real; %d of %d states match their real count.', SUM(w), SUM(r), SUM(w = r), COUNT(*)) FROM x"#,
        ),
        needs: W,
        open: "",
    },
    Panel {
        id: "C23.real_powerhouses",
        page: "high-school",
        area: "C23",
        title: r#"The real top five of each state in the world"#,
        kind: "table",
        note: r#"Each state's real top five programs (MaxPreps 2026, `hs_states.csv` top_5_powerhouses), how many of them the world has, and where they rank by prestige in their state now. A real top program should sit near the top."#,
        sql: r#"WITH RECURSIVE sp(state, rest, one) AS (SELECT state, top_5_powerhouses || ';', NULL FROM real_hs_states
         UNION ALL SELECT state, substr(rest, instr(rest, ';') + 1), trim(substr(rest, 1, instr(rest, ';') - 1)) FROM sp WHERE rest <> ''),
         rk AS (SELECT h._row, h.name || ' (' || h.town || ')' AS name, st.code AS state, h.prestige, RANK() OVER (PARTITION BY h.state ORDER BY h.prestige DESC) AS rank FROM high_schools h JOIN states st ON st._row = h.state)
         SELECT sp.state, COUNT(sp.one) AS real_top5, COUNT(rk._row) AS in_world, group_concat(sp.one || CASE WHEN rk._row IS NULL THEN ' (missing)' ELSE ' #' || rk.rank END, '; ') AS ranks_now,
         ROUND(AVG(rk.prestige), 2) AS mean_prestige
         FROM sp LEFT JOIN rk ON rk.state = sp.state AND rk.name = sp.one WHERE sp.one IS NOT NULL AND sp.one <> '' GROUP BY sp.state ORDER BY in_world, sp.state"#,
        says: Some(
            r#"WITH RECURSIVE sp(state, rest, one) AS (SELECT state, top_5_powerhouses || ';', NULL FROM real_hs_states
         UNION ALL SELECT state, substr(rest, instr(rest, ';') + 1), trim(substr(rest, 1, instr(rest, ';') - 1)) FROM sp WHERE rest <> ''),
         rk AS (SELECT h.name || ' (' || h.town || ')' AS name, st.code AS state, RANK() OVER (PARTITION BY h.state ORDER BY h.prestige DESC) AS rank FROM high_schools h JOIN states st ON st._row = h.state)
         SELECT printf('%d of %d real top-five programs are in the world; %d of those rank in their state''s top ten by prestige now.', COUNT(rk.name), COUNT(*), SUM(rk.rank <= 10))
         FROM sp LEFT JOIN rk ON rk.state = sp.state AND rk.name = sp.one WHERE sp.one IS NOT NULL AND sp.one <> ''"#,
        ),
        needs: W,
        open: "",
    },
    Panel {
        id: "C23.powerhouses",
        page: "high-school",
        area: "C23",
        title: r#"Powerhouses per state"#,
        kind: "map",
        note: r#"Schools with prestige 0.75 or more (a powerhouse, `high_school.rs`) in each state now: a few per state, not dozens."#,
        sql: r#"SELECT st.code AS state, SUM(h.prestige >= 0.75) AS world, COUNT(*) AS n FROM high_schools h JOIN states st ON st._row = h.state GROUP BY st._row ORDER BY st.code"#,
        says: Some(
            r#"WITH x AS (SELECT st.code, SUM(h.prestige >= 0.75) AS n FROM high_schools h JOIN states st ON st._row = h.state GROUP BY st._row)
         SELECT printf('%d powerhouses (prestige 0.75+): %d to %d a state.', SUM(n), MIN(n), MAX(n)) FROM x"#,
        ),
        needs: W,
        open: "",
    },
    Panel {
        id: "C23.national",
        page: "high-school",
        area: "C23",
        title: r#"The national powerhouses"#,
        kind: "table",
        note: r#"The schools that pull from out of state (`high_school.rs` NATIONAL_POWERHOUSES): prestige, state titles, roster size, how many of the roster have a home in another state (a boarder keeps his; a family that moves takes the new state) and how many boarded. Real IMG Academy's roster is mostly out-of-state boarders (from memory)."#,
        sql: r#"SELECT h._row AS id, h.name || ' (' || h.town || ')' AS school, st.code AS state, ROUND(h.prestige, 2) AS prestige, h.state_titles AS titles, json_array_length(h.roster) AS roster,
         (SELECT COUNT(*) FROM people x WHERE x.hs = +h._row AND x.stage = 'HighSchool' AND x.home_state <> h.state) AS from_out_of_state,
         (SELECT COUNT(*) FROM people x WHERE x.hs = +h._row AND x.stage = 'HighSchool' AND x.notes LIKE '%left home to board%') AS boarders
         FROM high_schools h JOIN states st ON st._row = h.state WHERE h.national = 1 ORDER BY h.prestige DESC"#,
        says: None,
        needs: W,
        open: "e/high_schools/{id}",
    },
    Panel {
        id: "C23.prestige",
        page: "high-school",
        area: "C23",
        title: r#"Prestige spread, public and private"#,
        kind: "hist",
        note: r#"Every school's prestige now in steps of 0.05, public and private apart (pick above the chart). Prestige history by year needs the world log (next panel)."#,
        sql: r#"SELECT CASE WHEN private = 1 THEN 'Private' ELSE 'Public' END AS grp, ROUND(CAST(prestige / 0.05 AS INTEGER) * 0.05, 2) AS x, COUNT(*) AS world FROM high_schools GROUP BY 1, 2 ORDER BY 1, 2"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C23.prestige_years",
        page: "high-school",
        area: "C23",
        title: r#"Prestige by year"#,
        kind: "line",
        note: r#"The spread of every school's prestige each year from the world log's yearly school rows (10th, 50th and 90th percentiles) and the number of powerhouses."#,
        sql: r#"WITH x AS (SELECT season AS year, school__prestige AS v FROM log_schools),
         r AS (SELECT year, v, ROW_NUMBER() OVER (PARTITION BY year ORDER BY v) AS r, COUNT(*) OVER (PARTITION BY year) AS n FROM x)
         SELECT year, MAX(CASE WHEN r = CAST(0.1 * (n - 1) AS INTEGER) + 1 THEN v END) AS p10, MAX(CASE WHEN r = CAST(0.5 * (n - 1) AS INTEGER) + 1 THEN v END) AS p50, MAX(CASE WHEN r = CAST(0.9 * (n - 1) AS INTEGER) + 1 THEN v END) AS p90, SUM(v >= 0.75) AS powerhouses FROM r GROUP BY year ORDER BY year"#,
        says: None,
        needs: WL,
        open: "",
    },
    Panel {
        id: "C23.money",
        page: "high-school",
        area: "C23",
        title: r#"Resources by the area's money"#,
        kind: "bars",
        note: r#"Mean school resources (0-1: weight room, staff, film, academic support) by the area's money in fifths, poorest first: resources should follow the town's income."#,
        sql: r#"WITH x AS (SELECT resources, area_money, NTILE(5) OVER (ORDER BY area_money) AS q FROM high_schools)
         SELECT 'Fifth ' || q || ' (money ' || printf('%.2f', MIN(area_money)) || '-' || printf('%.2f', MAX(area_money)) || ')' AS label, ROUND(AVG(resources), 3) AS world, COUNT(*) AS n FROM x GROUP BY q ORDER BY q"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C23.private",
        page: "high-school",
        area: "C23",
        title: r#"Private schools and their titles by state"#,
        kind: "table",
        note: r#"Each state's private share of schools against the private share of its state titles over the world's seasons; states that split public and private champions should show the two close."#,
        sql: r#"WITH hs AS MATERIALIZED (SELECT s._parent AS school, h.name || ' (' || h.town || ')' AS name, st.code AS state, h.private AS private, h.enrollment AS enrollment,
         s.year AS year, s.wins AS w, s.losses AS l, s.state_title AS t, s.coach AS coach, json_extract(s.points, '$[0]') AS pf, json_extract(s.points, '$[1]') AS pa
         FROM high_schools__seasons s JOIN high_schools h ON h._row = s._parent JOIN states st ON st._row = h.state) SELECT state, COUNT(DISTINCT school) AS schools, ROUND(100.0 * COUNT(DISTINCT CASE WHEN private = 1 THEN school END) / COUNT(DISTINCT school), 1) AS private_pct,
         SUM(t) AS titles, ROUND(100.0 * SUM(t * private) / NULLIF(SUM(t), 0), 1) AS private_title_pct,
         (SELECT COALESCE(private_classes, '') FROM real_hs_playoff_formats r WHERE r.state = hs.state) AS real_private_classes
         FROM hs GROUP BY state ORDER BY private_title_pct DESC"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C23.list",
        page: "high-school",
        area: "C23",
        title: r#"Every high school"#,
        kind: "table",
        note: r#"Each school's state, town, class, enrollment, private or public, resources, prestige, state titles and its head coach now. A row opens its page."#,
        sql: r#"SELECT h._row AS id, h.name || ' (' || h.town || ')' AS school, st.code AS state, h.classification + 1 AS class, h.enrollment, CASE WHEN h.private = 1 THEN 'private' ELSE 'public' END AS kind,
         ROUND(h.resources, 2) AS resources, ROUND(h.prestige, 3) AS prestige, h.state_titles AS titles, c.name AS coach
         FROM high_schools h JOIN states st ON st._row = h.state LEFT JOIN coaches c ON c._row = h.coach ORDER BY h.prestige DESC"#,
        says: None,
        needs: W,
        open: "e/high_schools/{id}",
    },
    Panel {
        id: "C24.champions",
        page: "high-school",
        area: "C24",
        title: r#"State champions a year by state"#,
        kind: "bars",
        note: r#"Each state's champions in the world's last season against its real brackets a year (`hs_playoff_formats.csv`: its classes, doubled where Select and Non-Select split, plus private-association and non-public classes and an Open Division), biggest gaps first."#,
        sql: r#"WITH rb AS (SELECT state, (CASE WHEN brackets LIKE 'equity:%' THEN (CASE WHEN COALESCE(substr(brackets, 8), '') = '' THEN 0 ELSE length(substr(brackets, 8)) - length(replace(substr(brackets, 8), ';', '')) + 1 END) WHEN brackets LIKE 'enrollment:%' THEN CAST(substr(brackets, 12) AS INTEGER) ELSE (CASE WHEN COALESCE(classes, '') = '' THEN 0 ELSE length(classes) - length(replace(classes, ';', '')) + 1 END) END)
         * (CASE WHEN private_rule = 'select' THEN 2 ELSE 1 END) + (CASE WHEN COALESCE(private_classes, '') = '' THEN 0 ELSE length(private_classes) - length(replace(private_classes, ';', '')) + 1 END)
         + (CASE WHEN private_rule LIKE 'nonpublic:%' THEN (CASE WHEN COALESCE(substr(private_rule, 11), '') = '' THEN 0 ELSE length(substr(private_rule, 11)) - length(replace(substr(private_rule, 11), ';', '')) + 1 END) WHEN private_rule LIKE 'private:%' THEN 1 ELSE 0 END)
         + (CASE WHEN COALESCE(open, '') <> '' THEN 1 ELSE 0 END) AS real
         FROM real_hs_playoff_formats), w AS (SELECT st.code AS state, SUM(s.state_title) AS n FROM high_schools__seasons s JOIN high_schools h ON h._row = s._parent JOIN states st ON st._row = h.state
         WHERE s.year = (SELECT MAX(year) FROM high_schools__seasons) GROUP BY st.code)
         SELECT w.state AS label, w.n AS world, rb.real FROM w LEFT JOIN rb ON rb.state = w.state ORDER BY ABS(w.n - COALESCE(rb.real, w.n)) DESC, w.state"#,
        says: Some(
            r#"WITH rb AS (SELECT state, (CASE WHEN brackets LIKE 'equity:%' THEN (CASE WHEN COALESCE(substr(brackets, 8), '') = '' THEN 0 ELSE length(substr(brackets, 8)) - length(replace(substr(brackets, 8), ';', '')) + 1 END) WHEN brackets LIKE 'enrollment:%' THEN CAST(substr(brackets, 12) AS INTEGER) ELSE (CASE WHEN COALESCE(classes, '') = '' THEN 0 ELSE length(classes) - length(replace(classes, ';', '')) + 1 END) END)
         * (CASE WHEN private_rule = 'select' THEN 2 ELSE 1 END) + (CASE WHEN COALESCE(private_classes, '') = '' THEN 0 ELSE length(private_classes) - length(replace(private_classes, ';', '')) + 1 END)
         + (CASE WHEN private_rule LIKE 'nonpublic:%' THEN (CASE WHEN COALESCE(substr(private_rule, 11), '') = '' THEN 0 ELSE length(substr(private_rule, 11)) - length(replace(substr(private_rule, 11), ';', '')) + 1 END) WHEN private_rule LIKE 'private:%' THEN 1 ELSE 0 END)
         + (CASE WHEN COALESCE(open, '') <> '' THEN 1 ELSE 0 END) AS real
         FROM real_hs_playoff_formats), w AS (SELECT st.code AS state, SUM(s.state_title) AS n FROM high_schools__seasons s JOIN high_schools h ON h._row = s._parent JOIN states st ON st._row = h.state
         WHERE s.year = (SELECT MAX(year) FROM high_schools__seasons) GROUP BY st.code)
         SELECT printf('%d state champions last season against %d real brackets; %d states match their real count.', SUM(w.n), SUM(rb.real), SUM(w.n = rb.real)) FROM w JOIN rb ON rb.state = w.state"#,
        ),
        needs: W,
        open: "",
    },
    Panel {
        id: "C24.most_15y",
        page: "high-school",
        area: "C24",
        title: r#"Most state titles by one school in 15 seasons"#,
        kind: "table",
        note: r#"For each school, the most state titles it won in any 15 straight seasons, with the seasons; the top 50. Real powerhouses rarely win more than about 10 in 15 (from memory, no open record book)."#,
        sql: r#"WITH hs AS MATERIALIZED (SELECT s._parent AS school, h.name || ' (' || h.town || ')' AS name, st.code AS state, h.private AS private, h.enrollment AS enrollment,
         s.year AS year, s.wins AS w, s.losses AS l, s.state_title AS t, s.coach AS coach, json_extract(s.points, '$[0]') AS pf, json_extract(s.points, '$[1]') AS pa
         FROM high_schools__seasons s JOIN high_schools h ON h._row = s._parent JOIN states st ON st._row = h.state), t AS (SELECT school, name, state, year FROM hs WHERE t = 1),
         win AS (SELECT a.school, a.name, a.state, a.year AS start, COUNT(b.year) AS n FROM t a JOIN t b ON b.school = a.school AND b.year BETWEEN a.year AND a.year + 14 GROUP BY a.school, a.year)
         SELECT school AS id, name AS school, state, MAX(n) AS titles_in_15, start || '-' || (start + 14) AS seasons FROM win GROUP BY school ORDER BY titles_in_15 DESC LIMIT 50"#,
        says: Some(
            r#"WITH hs AS MATERIALIZED (SELECT s._parent AS school, h.name || ' (' || h.town || ')' AS name, st.code AS state, h.private AS private, h.enrollment AS enrollment,
         s.year AS year, s.wins AS w, s.losses AS l, s.state_title AS t, s.coach AS coach, json_extract(s.points, '$[0]') AS pf, json_extract(s.points, '$[1]') AS pa
         FROM high_schools__seasons s JOIN high_schools h ON h._row = s._parent JOIN states st ON st._row = h.state), t AS (SELECT school, name, year FROM hs WHERE t = 1),
         win AS (SELECT a.name, a.year AS start, COUNT(b.year) AS n FROM t a JOIN t b ON b.school = a.school AND b.year BETWEEN a.year AND a.year + 14 GROUP BY a.school, a.year)
         SELECT printf('%s won %d state titles in 15 seasons (%d-%d).', name, n, start, start + 14) FROM win ORDER BY n DESC LIMIT 1"#,
        ),
        needs: W,
        open: "e/high_schools/{id}",
    },
    Panel {
        id: "C24.streaks",
        page: "high-school",
        area: "C24",
        title: r#"Longest winning streaks"#,
        kind: "table",
        note: r#"The state record books' winning streaks (`stats.books`, WinStreak), longest first across states, with the season each ended or the year it is still running. Real record: De La Salle's 151 (1992-2004)."#,
        sql: r#"WITH bk AS (SELECT b._key AS k, j.value AS i, CASE j.value WHEN 0 THEN _0__value WHEN 1 THEN _1__value WHEN 2 THEN _2__value WHEN 3 THEN _3__value WHEN 4 THEN _4__value
         WHEN 5 THEN _5__value WHEN 6 THEN _6__value WHEN 7 THEN _7__value WHEN 8 THEN _8__value ELSE _9__value END AS value,
         CASE j.value WHEN 0 THEN _0__name WHEN 1 THEN _1__name WHEN 2 THEN _2__name WHEN 3 THEN _3__name WHEN 4 THEN _4__name
         WHEN 5 THEN _5__name WHEN 6 THEN _6__name WHEN 7 THEN _7__name WHEN 8 THEN _8__name ELSE _9__name END AS name,
         CASE j.value WHEN 0 THEN _0__year WHEN 1 THEN _1__year WHEN 2 THEN _2__year WHEN 3 THEN _3__year WHEN 4 THEN _4__year
         WHEN 5 THEN _5__year WHEN 6 THEN _6__year WHEN 7 THEN _7__year WHEN 8 THEN _8__year ELSE _9__year END AS year,
         CASE j.value WHEN 0 THEN _0__id WHEN 1 THEN _1__id WHEN 2 THEN _2__id WHEN 3 THEN _3__id WHEN 4 THEN _4__id
         WHEN 5 THEN _5__id WHEN 6 THEN _6__id WHEN 7 THEN _7__id WHEN 8 THEN _8__id ELSE _9__id END AS id
         FROM stats__books b, json_each('[0,1,2,3,4,5,6,7,8,9]') j WHERE b._key LIKE '[{"State":%') SELECT CAST(json_extract(bk.k, '$[0].State') AS INTEGER) AS sidx, (SELECT code FROM states WHERE _row = json_extract(bk.k, '$[0].State')) AS state, bk.name AS school, CAST(bk.value AS INTEGER) AS games, bk.year,
         151 AS real_record FROM bk WHERE bk.k LIKE '%"WinStreak"%' AND bk.value IS NOT NULL ORDER BY bk.value DESC LIMIT 100"#,
        says: Some(
            r#"WITH bk AS (SELECT b._key AS k, j.value AS i, CASE j.value WHEN 0 THEN _0__value WHEN 1 THEN _1__value WHEN 2 THEN _2__value WHEN 3 THEN _3__value WHEN 4 THEN _4__value
         WHEN 5 THEN _5__value WHEN 6 THEN _6__value WHEN 7 THEN _7__value WHEN 8 THEN _8__value ELSE _9__value END AS value,
         CASE j.value WHEN 0 THEN _0__name WHEN 1 THEN _1__name WHEN 2 THEN _2__name WHEN 3 THEN _3__name WHEN 4 THEN _4__name
         WHEN 5 THEN _5__name WHEN 6 THEN _6__name WHEN 7 THEN _7__name WHEN 8 THEN _8__name ELSE _9__name END AS name,
         CASE j.value WHEN 0 THEN _0__year WHEN 1 THEN _1__year WHEN 2 THEN _2__year WHEN 3 THEN _3__year WHEN 4 THEN _4__year
         WHEN 5 THEN _5__year WHEN 6 THEN _6__year WHEN 7 THEN _7__year WHEN 8 THEN _8__year ELSE _9__year END AS year,
         CASE j.value WHEN 0 THEN _0__id WHEN 1 THEN _1__id WHEN 2 THEN _2__id WHEN 3 THEN _3__id WHEN 4 THEN _4__id
         WHEN 5 THEN _5__id WHEN 6 THEN _6__id WHEN 7 THEN _7__id WHEN 8 THEN _8__id ELSE _9__id END AS id
         FROM stats__books b, json_each('[0,1,2,3,4,5,6,7,8,9]') j WHERE b._key LIKE '[{"State":%') SELECT printf('Longest streak %d straight (%s, %d); %d streaks of 100+; the real record is 151.', MAX(value), (SELECT name FROM bk b2 WHERE b2.k LIKE '%"WinStreak"%' ORDER BY value DESC LIMIT 1),
         (SELECT year FROM bk b2 WHERE b2.k LIKE '%"WinStreak"%' ORDER BY value DESC LIMIT 1), SUM(value >= 100)) FROM bk WHERE k LIKE '%"WinStreak"%' HAVING MAX(value) > 0"#,
        ),
        needs: W,
        open: "",
    },
    Panel {
        id: "C24.active_streaks",
        page: "high-school",
        area: "C24",
        title: r#"Longest winning streaks still running"#,
        kind: "table",
        note: r#"Schools with the longest winning streak still running now (`stats.hs_streak`)."#,
        sql: r#"SELECT h._row AS id, h.name || ' (' || h.town || ')' AS school, st.code AS state, x._0 AS games, ROUND(h.prestige, 2) AS prestige
         FROM stats__hs_streak x JOIN high_schools h ON h._row = x._row JOIN states st ON st._row = h.state WHERE x._0 > 0 ORDER BY x._0 DESC LIMIT 50"#,
        says: None,
        needs: W,
        open: "e/high_schools/{id}",
    },
    Panel {
        id: "C24.unbeaten",
        page: "high-school",
        area: "C24",
        title: r#"Unbeaten seasons and champions a year"#,
        kind: "line",
        note: r#"Each season: schools with no losses, state champions, and champions who went unbeaten."#,
        sql: r#"WITH hs AS MATERIALIZED (SELECT s._parent AS school, h.name || ' (' || h.town || ')' AS name, st.code AS state, h.private AS private, h.enrollment AS enrollment,
         s.year AS year, s.wins AS w, s.losses AS l, s.state_title AS t, s.coach AS coach, json_extract(s.points, '$[0]') AS pf, json_extract(s.points, '$[1]') AS pa
         FROM high_schools__seasons s JOIN high_schools h ON h._row = s._parent JOIN states st ON st._row = h.state) SELECT year, SUM(l = 0 AND w > 0) AS unbeaten, SUM(t) AS champions, SUM(t = 1 AND l = 0) AS unbeaten_champions FROM hs GROUP BY year ORDER BY year"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C24.dynasties",
        page: "high-school",
        area: "C24",
        title: r#"Most state titles"#,
        kind: "table",
        note: r#"Schools with the most state titles in the world's seasons, with their first and last title and their best run of straight titles."#,
        sql: r#"WITH hs AS MATERIALIZED (SELECT s._parent AS school, h.name || ' (' || h.town || ')' AS name, st.code AS state, h.private AS private, h.enrollment AS enrollment,
         s.year AS year, s.wins AS w, s.losses AS l, s.state_title AS t, s.coach AS coach, json_extract(s.points, '$[0]') AS pf, json_extract(s.points, '$[1]') AS pa
         FROM high_schools__seasons s JOIN high_schools h ON h._row = s._parent JOIN states st ON st._row = h.state), t AS (SELECT school, name, state, year, year - ROW_NUMBER() OVER (PARTITION BY school ORDER BY year) AS run FROM hs WHERE t = 1),
         r AS (SELECT school, COUNT(*) AS n, MIN(year) AS a, MAX(year) AS z FROM t GROUP BY school, run)
         SELECT t.school AS id, t.name AS school, t.state, COUNT(*) AS titles, MIN(t.year) AS first, MAX(t.year) AS last, (SELECT MAX(n) FROM r WHERE r.school = t.school) AS best_run
         FROM t GROUP BY t.school ORDER BY titles DESC LIMIT 60"#,
        says: None,
        needs: W,
        open: "e/high_schools/{id}",
    },
    Panel {
        id: "C24.by_state",
        page: "high-school",
        area: "C24",
        title: r#"Champions by state and season"#,
        kind: "table",
        note: r#"Every state champion by season (pick a state above; the slider picks a season), with its record."#,
        sql: r#"WITH hs AS MATERIALIZED (SELECT s._parent AS school, h.name || ' (' || h.town || ')' AS name, st.code AS state, h.private AS private, h.enrollment AS enrollment,
         s.year AS year, s.wins AS w, s.losses AS l, s.state_title AS t, s.coach AS coach, json_extract(s.points, '$[0]') AS pf, json_extract(s.points, '$[1]') AS pa
         FROM high_schools__seasons s JOIN high_schools h ON h._row = s._parent JOIN states st ON st._row = h.state) SELECT state AS grp, year, school AS id, name AS champion, w || '-' || l AS record, CASE WHEN private = 1 THEN 'private' ELSE 'public' END AS kind FROM hs WHERE t = 1 ORDER BY state, year DESC, name"#,
        says: None,
        needs: W,
        open: "e/high_schools/{id}",
    },
    Panel {
        id: "C25.dates",
        page: "high-school",
        area: "C25",
        title: r#"Each state's season this year"#,
        kind: "table",
        note: r#"Each state's first game and finals this season (`hs_season.states`) beside the real playoff start and finals (SI 2026 guide, `hs_states.csv` playoff_format)."#,
        sql: r#"SELECT st.code AS state, date(x.start * 86400, 'unixepoch') AS start, date(x.finals * 86400, 'unixepoch') AS finals, json_array_length(x.weeks) AS regular_weeks, json_array_length(x.rounds) AS playoff_rounds,
         r.playoff_format AS real_playoffs
         FROM hs_season__states x JOIN states st ON st._row = x._row LEFT JOIN real_hs_states r ON r.state = st.code ORDER BY x.start, st.code"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C25.counters",
        page: "high-school",
        area: "C25",
        title: r#"The high school year's counters"#,
        kind: "line",
        note: r#"The calendar log's yearly counts: forfeits, no contests, grade-outs, homecomings, senior nights, all-state picks and heat-limited practice days."#,
        sql: r#"SELECT year, hs_forfeits AS forfeits, hs_no_contests AS no_contests, hs_grade_outs AS grade_outs, hs_homecomings AS homecomings, hs_senior_nights AS senior_nights, hs_all_state AS all_state,
         heat_limited_days FROM log_history ORDER BY year"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C25.engine",
        page: "high-school",
        area: "C25",
        title: r#"Games on the match engine"#,
        kind: "line",
        note: r#"High school games each year and how many ran on the match engine (2014 on); stand-in seasons have no points."#,
        sql: r#"SELECT year, hs_games AS games, hs_engine_games AS engine_games FROM log_history WHERE hs_games > 0 ORDER BY year"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C25.all_state",
        page: "high-school",
        area: "C25",
        title: r#"All-state picks per 100 high school players"#,
        kind: "map",
        note: r#"Current high school players named all-state at least once, per 100 players, by the state they play in."#,
        sql: r#"SELECT st.code AS state, ROUND(100.0 * AVG(COALESCE(x.cal__all_state, 0) > 0), 1) AS world, COUNT(*) AS n
         FROM people x JOIN high_schools h ON h._row = x.hs JOIN states st ON st._row = h.state WHERE x.stage = 'HighSchool' GROUP BY st._row ORDER BY st.code"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C25.spring",
        page: "high-school",
        area: "C25",
        title: r#"Spring ball: notices, breakouts and fizzles"#,
        kind: "line",
        note: r#"Each year's spring notices, breakouts and fizzles (the calendar log)."#,
        sql: r#"SELECT year, spring_notices AS notices, spring_breakouts AS breakouts, spring_fizzles AS fizzles FROM log_history WHERE spring_notices > 0 ORDER BY year"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C26.tenure",
        page: "high-school",
        area: "C26",
        title: r#"High school head coaches' years in the job"#,
        kind: "hist",
        note: r#"Current head coaches' seasons in their job, and finished high school head jobs' lengths (pick above). Real long tenures are common (no open count)."#,
        sql: r#"SELECT 'Current' AS grp, MIN((SELECT MAX(year) FROM high_schools__seasons) - job__since + 1, 30) AS x, COUNT(*) AS world FROM coaches WHERE job__employer__HighSchool IS NOT NULL AND job__role = 'HeadCoach' GROUP BY x
         UNION ALL SELECT 'Finished', MIN("to" - "from" + 1, 30), COUNT(*) FROM coaches__history WHERE employer__HighSchool IS NOT NULL AND role = 'HeadCoach' GROUP BY 2 ORDER BY 1, 2"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C26.churn",
        page: "high-school",
        area: "C26",
        title: r#"Head coach changes a season, by school prestige"#,
        kind: "bars",
        note: r#"The share of seasons a school starts with a new head coach (the season's coach against last season's), by prestige band, and the season after a state title. Real dynasties keep their coach for decades (Bruce Rollinson at Mater Dei 1989-2021, from memory)."#,
        sql: r#"WITH s AS MATERIALIZED (SELECT s._parent AS h, s.coach AS c, LAG(s.coach) OVER (PARTITION BY s._parent ORDER BY s.year) AS prev,
         LAG(s.state_title) OVER (PARTITION BY s._parent ORDER BY s.year) AS won FROM high_schools__seasons s)
         SELECT CASE WHEN h.prestige >= 0.85 THEN '1 Prestige 0.85 and up' WHEN h.prestige >= 0.6 THEN '2 Prestige 0.6-0.85' WHEN h.prestige >= 0.3 THEN '3 Prestige 0.3-0.6' ELSE '4 Prestige under 0.3' END AS label,
         ROUND(100.0 * AVG(s.c <> s.prev), 1) AS world, COUNT(*) AS n FROM s JOIN high_schools h ON h._row = s.h WHERE s.prev IS NOT NULL AND s.c IS NOT NULL GROUP BY label
         UNION ALL SELECT '5 The season after a state title', ROUND(100.0 * AVG(c <> prev), 1), COUNT(*) FROM s WHERE prev IS NOT NULL AND c IS NOT NULL AND won = 1
         UNION ALL SELECT '6 The season after no title', ROUND(100.0 * AVG(c <> prev), 1), COUNT(*) FROM s WHERE prev IS NOT NULL AND c IS NOT NULL AND won = 0 ORDER BY label"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C26.churn_schools",
        page: "high-school",
        area: "C26",
        title: r#"Schools that change head coach most"#,
        kind: "table",
        note: r#"Schools by head coach changes across the world's seasons, with their state titles: a school that wins every year should keep its coach."#,
        sql: r#"WITH s AS MATERIALIZED (SELECT s._parent AS h, s.state_title AS t, s.coach AS c, LAG(s.coach) OVER (PARTITION BY s._parent ORDER BY s.year) AS prev FROM high_schools__seasons s)
         SELECT h._row AS id, h.name AS school, st.code AS state, ROUND(h.prestige, 2) AS prestige, COUNT(*) + 1 AS seasons, SUM(s.c <> s.prev) AS changes, SUM(s.t) AS titles
         FROM s JOIN high_schools h ON h._row = s.h LEFT JOIN states st ON st._row = h.state WHERE s.prev IS NOT NULL AND s.c IS NOT NULL
         GROUP BY s.h ORDER BY changes DESC, titles DESC LIMIT 100"#,
        says: None,
        needs: W,
        open: "e/high_schools/{id}",
    },
    Panel {
        id: "C26.titles",
        page: "high-school",
        area: "C26",
        title: r#"Coaches with the most state titles"#,
        kind: "table",
        note: r#"Head coaches credited with the most state titles (the season's coach), with their schools and career high school record."#,
        sql: r#"WITH hs AS MATERIALIZED (SELECT s._parent AS school, h.name || ' (' || h.town || ')' AS name, st.code AS state, h.private AS private, h.enrollment AS enrollment,
         s.year AS year, s.wins AS w, s.losses AS l, s.state_title AS t, s.coach AS coach, json_extract(s.points, '$[0]') AS pf, json_extract(s.points, '$[1]') AS pa
         FROM high_schools__seasons s JOIN high_schools h ON h._row = s._parent JOIN states st ON st._row = h.state) SELECT hs.coach AS id, c.name, SUM(hs.t) AS titles, COUNT(*) AS seasons, SUM(hs.w) || '-' || SUM(hs.l) AS record, group_concat(DISTINCT hs.name) AS schools
         FROM hs JOIN coaches c ON c._row = hs.coach WHERE hs.coach IS NOT NULL GROUP BY hs.coach HAVING SUM(hs.t) > 0 ORDER BY titles DESC, SUM(hs.w) DESC LIMIT 60"#,
        says: None,
        needs: W,
        open: "e/coaches/{id}",
    },
    Panel {
        id: "C26.wins",
        page: "high-school",
        area: "C26",
        title: r#"Most high school wins by a coach"#,
        kind: "table",
        note: r#"Head coaches by high school wins as the season's coach (real ladder: John McKissick 621 at Summerville, SC)."#,
        sql: r#"WITH hs AS MATERIALIZED (SELECT s._parent AS school, h.name || ' (' || h.town || ')' AS name, st.code AS state, h.private AS private, h.enrollment AS enrollment,
         s.year AS year, s.wins AS w, s.losses AS l, s.state_title AS t, s.coach AS coach, json_extract(s.points, '$[0]') AS pf, json_extract(s.points, '$[1]') AS pa
         FROM high_schools__seasons s JOIN high_schools h ON h._row = s._parent JOIN states st ON st._row = h.state) SELECT hs.coach AS id, c.name, SUM(hs.w) AS wins, SUM(hs.l) AS losses, COUNT(*) AS seasons, SUM(hs.t) AS titles, MIN(hs.year) || '-' || MAX(hs.year) AS years, 621 AS real_record
         FROM hs JOIN coaches c ON c._row = hs.coach WHERE hs.coach IS NOT NULL GROUP BY hs.coach ORDER BY wins DESC LIMIT 60"#,
        says: None,
        needs: W,
        open: "e/coaches/{id}",
    },
    Panel {
        id: "C26.to_college",
        page: "high-school",
        area: "C26",
        title: r#"High school coaches moving into college"#,
        kind: "line",
        note: r#"Each year: high school coaches whose next job was at a college, by the college's level now (rare in real life)."#,
        sql: r#"WITH st AS MATERIALIZED (SELECT h._parent AS c, h.role AS role, h."from" AS a, h."to" AS z, h.wins AS w, h.losses AS l, h.titles AS titles, h.ended AS ended,
         CASE WHEN h.employer__College IS NOT NULL THEN (SELECT division FROM programs WHERE _row = h.employer__College)
         WHEN h.employer__HighSchool IS NOT NULL THEN 'HS' WHEN h.employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level,
         h.employer__College AS college, h.employer__HighSchool AS hs, h.employer__Nfl AS nfl
         FROM coaches__history h), n AS (SELECT c, a, level, LAG(level) OVER (PARTITION BY c ORDER BY a) AS before FROM st)
         SELECT a AS year, SUM(level = 'FBS') AS fbs, SUM(level = 'FCS') AS fcs, SUM(level IN ('D2', 'D3', 'NAIA', 'JUCO')) AS lower FROM n WHERE before = 'HS' AND level NOT IN ('HS', 'NFL', 'Other') GROUP BY a ORDER BY a"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C26.network",
        page: "high-school",
        area: "C26",
        title: r#"A coach's network and how seen his players are"#,
        kind: "bars",
        note: r#"Current high school players' mean visibility (0-1) by their head coach's network rating (hidden, 1-20) in bands, with their mean stars: a big network should get players seen more, at the same talent."#,
        sql: r#"SELECT CASE WHEN c.ability__network < 7 THEN 'Network under 7' WHEN c.ability__network < 10 THEN '7-10' WHEN c.ability__network < 13 THEN '10-13' WHEN c.ability__network < 16 THEN '13-16' ELSE '16+' END AS label,
         ROUND(AVG(x.visibility), 3) AS world, ROUND(AVG(x.stars), 2) AS stars, ROUND(AVG(x.x_overall), 1) AS overall, COUNT(*) AS n
         FROM people x JOIN high_schools h ON h._row = x.hs JOIN coaches c ON c._row = h.coach
         WHERE x.stage = 'HighSchool' GROUP BY 1 ORDER BY MIN(c.ability__network)"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C27.transfers",
        page: "high-school",
        area: "C27",
        title: r#"Top recruits who changed high schools"#,
        kind: "line",
        note: r#"Of each class's 100 best recruits by stars and visibility, how many have a high school transfer note (`notes`: 'transferred from'). Real: many top recruits move at least once (no open count)."#,
        sql: r#"WITH x AS (SELECT class_of, notes, ROW_NUMBER() OVER (PARTITION BY class_of ORDER BY stars DESC, visibility DESC) AS r FROM people WHERE class_of IS NOT NULL AND stage IN ('HighSchool', 'College', 'Declared', 'Pro'))
         SELECT class_of AS year, SUM(notes LIKE '%transferred from%') AS of_top_100, SUM(notes LIKE '%left home to board%') AS boarded FROM x WHERE r <= 100 GROUP BY class_of HAVING COUNT(*) = 100 ORDER BY class_of"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "C27.draw",
        page: "high-school",
        area: "C27",
        title: r#"Where the national powerhouses' players come from"#,
        kind: "table",
        note: r#"Each national powerhouse's current players by home state, most first."#,
        sql: r#"SELECT h._row AS id, h.name || ' (' || h.town || ')' AS school, sh.code AS from_state, COUNT(*) AS players
         FROM high_schools h JOIN people x ON x.hs = h._row AND x.stage = 'HighSchool' JOIN states sh ON sh._row = x.home_state WHERE h.national = 1 GROUP BY h._row, sh.code ORDER BY h._row, players DESC"#,
        says: None,
        needs: W,
        open: "e/high_schools/{id}",
    },
    Panel {
        id: "C27.sit_out",
        page: "high-school",
        area: "C27",
        title: r#"Transfers by the state's sit-out rule"#,
        kind: "bars",
        note: r#"The share of current high school players with a transfer note, by their state's rule (`states.transfer`: a sit-out or not); a sit-out should mean fewer moves."#,
        sql: r#"SELECT CASE WHEN st.transfer__sit_out > 0 THEN 'Sit-out rule' ELSE 'No sit-out' END || CASE WHEN st.transfer__free_once = 1 THEN ', one free move' ELSE '' END AS label,
         ROUND(100.0 * AVG(x.notes LIKE '%transferred from%'), 2) AS world, COUNT(*) AS n
         FROM people x JOIN high_schools h ON h._row = x.hs JOIN states st ON st._row = h.state WHERE x.stage = 'HighSchool' GROUP BY 1 ORDER BY 1"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "G7.per_game",
        page: "high-school",
        area: "G7",
        title: r#"Points, plays and yards a team game"#,
        kind: "line",
        note: r#"Each engine year's high school totals per team game (the calendar log). Real points about 26 a team game, from memory."#,
        sql: r#"SELECT year, ROUND(1.0 * hs_totals__points_for / hs_totals__games, 1) AS points, ROUND(1.0 * hs_totals__plays / hs_totals__games, 1) AS plays,
         ROUND(1.0 * hs_totals__yards / hs_totals__plays, 2) AS yards_a_play, ROUND(1.0 * hs_totals__turnovers / hs_totals__games, 2) AS turnovers, 26 AS real
         FROM log_history WHERE hs_totals__games > 0 ORDER BY year"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "G7.state_points",
        page: "high-school",
        area: "G7",
        title: r#"Points a team game by state"#,
        kind: "map",
        note: r#"Points for over games in each state's school seasons on the engine (2014 on)."#,
        sql: r#"WITH hs AS MATERIALIZED (SELECT s._parent AS school, h.name || ' (' || h.town || ')' AS name, st.code AS state, h.private AS private, h.enrollment AS enrollment,
         s.year AS year, s.wins AS w, s.losses AS l, s.state_title AS t, s.coach AS coach, json_extract(s.points, '$[0]') AS pf, json_extract(s.points, '$[1]') AS pa
         FROM high_schools__seasons s JOIN high_schools h ON h._row = s._parent JOIN states st ON st._row = h.state) SELECT state, ROUND(1.0 * SUM(pf) / NULLIF(SUM(w + l), 0), 1) AS world, 26 AS real, COUNT(*) AS n FROM hs WHERE pf > 0 GROUP BY state ORDER BY state"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "G7.brackets",
        page: "high-school",
        area: "G7",
        title: r#"This season's classes and brackets by state"#,
        kind: "table",
        note: r#"Each state's classes this season (`hs_season.groups`): schools, survivors, against the real classes and field sizes (`hs_playoff_formats.csv`)."#,
        sql: r#"SELECT st.code AS state, COUNT(*) AS classes, SUM(json_array_length(g.schools)) AS schools, MIN(json_array_length(g.schools)) || '-' || MAX(json_array_length(g.schools)) AS class_sizes,
         r.classes AS real_classes, r.private_classes AS real_private, r.fields AS real_fields
         FROM hs_season__groups g JOIN states st ON st._row = g.state LEFT JOIN real_hs_playoff_formats r ON r.state = st.code GROUP BY st._row ORDER BY st.code"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "G7.records",
        page: "high-school",
        area: "G7",
        title: r#"Season records a year"#,
        kind: "line",
        note: r#"Shares of school seasons at 10+ wins, at 0 wins and the mean games a school played, by year."#,
        sql: r#"WITH hs AS MATERIALIZED (SELECT s._parent AS school, h.name || ' (' || h.town || ')' AS name, st.code AS state, h.private AS private, h.enrollment AS enrollment,
         s.year AS year, s.wins AS w, s.losses AS l, s.state_title AS t, s.coach AS coach, json_extract(s.points, '$[0]') AS pf, json_extract(s.points, '$[1]') AS pa
         FROM high_schools__seasons s JOIN high_schools h ON h._row = s._parent JOIN states st ON st._row = h.state) SELECT year, ROUND(100.0 * AVG(w >= 10), 1) AS pct_10_wins, ROUND(100.0 * AVG(w = 0), 1) AS pct_winless, ROUND(AVG(w + l), 2) AS games FROM hs GROUP BY year ORDER BY year"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "G27.bests",
        page: "high-school",
        area: "G27",
        title: r#"Best high school seasons against real"#,
        kind: "bars",
        note: r#"The best season in the world's high school lines (engine years) for each record-book stat, against a real season mark where trial 10 has one (from memory: receiving 1,500 yards is common among leaders, sacks record about 40, interceptions 10-15)."#,
        sql: r#"WITH hst(k, stat, line, real) AS (VALUES (0, 'Passing yards', 4, NULL), (1, 'Passing TDs', 5, NULL), (2, 'Rushing yards', 8, NULL), (3, 'Rushing TDs', 9, NULL),
         (4, 'Receptions', 10, NULL), (5, 'Receiving yards', 11, 1500), (6, 'Receiving TDs', 12, NULL), (7, 'Tackles', 13, NULL), (8, 'Sacks', 15, 40),
         (9, 'Interceptions', 16, 12), (10, 'Tackles for loss', 14, NULL), (11, 'Field goals', 18, NULL)), x AS MATERIALIZED (SELECT _parent, year, school, pos, games, json_extract(s, '$[4]') AS c4, json_extract(s, '$[5]') AS c5, json_extract(s, '$[8]') AS c8, json_extract(s, '$[9]') AS c9, json_extract(s, '$[10]') AS c10, json_extract(s, '$[11]') AS c11, json_extract(s, '$[12]') AS c12, json_extract(s, '$[13]') AS c13, json_extract(s, '$[15]') / 2.0 AS c15, json_extract(s, '$[16]') AS c16, json_extract(s, '$[14]') / 2.0 AS c14, json_extract(s, '$[18]') AS c18 FROM people__hs_lines), m AS (SELECT MAX(c4) AS m4, MAX(c5) AS m5, MAX(c8) AS m8, MAX(c9) AS m9, MAX(c10) AS m10, MAX(c11) AS m11, MAX(c12) AS m12, MAX(c13) AS m13, MAX(c15) AS m15, MAX(c16) AS m16, MAX(c14) AS m14, MAX(c18) AS m18 FROM x)
         SELECT hst.stat AS label, CASE hst.line WHEN 4 THEN m4 WHEN 5 THEN m5 WHEN 8 THEN m8 WHEN 9 THEN m9 WHEN 10 THEN m10 WHEN 11 THEN m11 WHEN 12 THEN m12 WHEN 13 THEN m13 WHEN 15 THEN m15 WHEN 16 THEN m16 WHEN 14 THEN m14 WHEN 18 THEN m18 END AS world, hst.real FROM hst, m ORDER BY hst.k"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "G27.leaders",
        page: "high-school",
        area: "G27",
        title: r#"Season leaders by stat"#,
        kind: "table",
        note: r#"The 25 best high school seasons for each record-book stat (pick a stat above), with player, school, year and games. A row opens the player."#,
        sql: r#"WITH hst(k, stat, line, real) AS (VALUES (0, 'Passing yards', 4, NULL), (1, 'Passing TDs', 5, NULL), (2, 'Rushing yards', 8, NULL), (3, 'Rushing TDs', 9, NULL),
         (4, 'Receptions', 10, NULL), (5, 'Receiving yards', 11, 1500), (6, 'Receiving TDs', 12, NULL), (7, 'Tackles', 13, NULL), (8, 'Sacks', 15, 40),
         (9, 'Interceptions', 16, 12), (10, 'Tackles for loss', 14, NULL), (11, 'Field goals', 18, NULL)), x AS MATERIALIZED (SELECT _parent, year, school, pos, games, json_extract(s, '$[4]') AS c4, json_extract(s, '$[5]') AS c5, json_extract(s, '$[8]') AS c8, json_extract(s, '$[9]') AS c9, json_extract(s, '$[10]') AS c10, json_extract(s, '$[11]') AS c11, json_extract(s, '$[12]') AS c12, json_extract(s, '$[13]') AS c13, json_extract(s, '$[15]') / 2.0 AS c15, json_extract(s, '$[16]') AS c16, json_extract(s, '$[14]') / 2.0 AS c14, json_extract(s, '$[18]') AS c18 FROM people__hs_lines), y AS (SELECT * FROM (SELECT 4 AS line, _parent, year, school, pos, games, c4 AS v FROM x ORDER BY c4 DESC LIMIT 25) UNION ALL SELECT * FROM (SELECT 5 AS line, _parent, year, school, pos, games, c5 AS v FROM x ORDER BY c5 DESC LIMIT 25) UNION ALL SELECT * FROM (SELECT 8 AS line, _parent, year, school, pos, games, c8 AS v FROM x ORDER BY c8 DESC LIMIT 25) UNION ALL SELECT * FROM (SELECT 9 AS line, _parent, year, school, pos, games, c9 AS v FROM x ORDER BY c9 DESC LIMIT 25) UNION ALL SELECT * FROM (SELECT 10 AS line, _parent, year, school, pos, games, c10 AS v FROM x ORDER BY c10 DESC LIMIT 25) UNION ALL SELECT * FROM (SELECT 11 AS line, _parent, year, school, pos, games, c11 AS v FROM x ORDER BY c11 DESC LIMIT 25) UNION ALL SELECT * FROM (SELECT 12 AS line, _parent, year, school, pos, games, c12 AS v FROM x ORDER BY c12 DESC LIMIT 25) UNION ALL SELECT * FROM (SELECT 13 AS line, _parent, year, school, pos, games, c13 AS v FROM x ORDER BY c13 DESC LIMIT 25) UNION ALL SELECT * FROM (SELECT 15 AS line, _parent, year, school, pos, games, c15 AS v FROM x ORDER BY c15 DESC LIMIT 25) UNION ALL SELECT * FROM (SELECT 16 AS line, _parent, year, school, pos, games, c16 AS v FROM x ORDER BY c16 DESC LIMIT 25) UNION ALL SELECT * FROM (SELECT 14 AS line, _parent, year, school, pos, games, c14 AS v FROM x ORDER BY c14 DESC LIMIT 25) UNION ALL SELECT * FROM (SELECT 18 AS line, _parent, year, school, pos, games, c18 AS v FROM x ORDER BY c18 DESC LIMIT 25)),
         r AS (SELECT hst.stat AS grp, hst.k, y.*, ROW_NUMBER() OVER (PARTITION BY y.line ORDER BY y.v DESC) AS rk FROM y JOIN hst ON hst.line = y.line)
         SELECT r.grp, r.rk AS rank, p.player__id AS id, p.name, r.pos, h.name || ' (' || h.town || ')' AS school, r.year, r.games, r.v AS value
         FROM r JOIN people p ON p._row = r._parent LEFT JOIN high_schools h ON h._row = r.school WHERE r.rk <= 25 ORDER BY r.k, r.rk"#,
        says: None,
        needs: W,
        open: "e/people/{id}",
    },
    Panel {
        id: "G27.big_seasons",
        page: "high-school",
        area: "G27",
        title: r#"Big seasons a year"#,
        kind: "line",
        note: r#"Each year's count of seasons with 30+ sacks (impossible-looking; real record about 40), 1,500+ receiving yards (common among real leaders) and 10+ interceptions."#,
        sql: r#"SELECT year, SUM(json_extract(s, '$[15]') >= 60) AS sacks_30, SUM(json_extract(s, '$[11]') >= 1500) AS rec_1500, SUM(json_extract(s, '$[16]') >= 10) AS int_10, COUNT(*) AS lines
         FROM people__hs_lines GROUP BY year ORDER BY year"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "G27.state_books",
        page: "high-school",
        area: "G27",
        title: r#"State record books"#,
        kind: "table",
        note: r#"Each state's record book (pick a state above): the top entry of every season and career list and the winning streak (`stats.books`)."#,
        sql: r#"WITH hst(k, stat, line, real) AS (VALUES (0, 'Passing yards', 4, NULL), (1, 'Passing TDs', 5, NULL), (2, 'Rushing yards', 8, NULL), (3, 'Rushing TDs', 9, NULL),
         (4, 'Receptions', 10, NULL), (5, 'Receiving yards', 11, 1500), (6, 'Receiving TDs', 12, NULL), (7, 'Tackles', 13, NULL), (8, 'Sacks', 15, 40),
         (9, 'Interceptions', 16, 12), (10, 'Tackles for loss', 14, NULL), (11, 'Field goals', 18, NULL)), bk AS (SELECT b._key AS k, j.value AS i, CASE j.value WHEN 0 THEN _0__value WHEN 1 THEN _1__value WHEN 2 THEN _2__value WHEN 3 THEN _3__value WHEN 4 THEN _4__value
         WHEN 5 THEN _5__value WHEN 6 THEN _6__value WHEN 7 THEN _7__value WHEN 8 THEN _8__value ELSE _9__value END AS value,
         CASE j.value WHEN 0 THEN _0__name WHEN 1 THEN _1__name WHEN 2 THEN _2__name WHEN 3 THEN _3__name WHEN 4 THEN _4__name
         WHEN 5 THEN _5__name WHEN 6 THEN _6__name WHEN 7 THEN _7__name WHEN 8 THEN _8__name ELSE _9__name END AS name,
         CASE j.value WHEN 0 THEN _0__year WHEN 1 THEN _1__year WHEN 2 THEN _2__year WHEN 3 THEN _3__year WHEN 4 THEN _4__year
         WHEN 5 THEN _5__year WHEN 6 THEN _6__year WHEN 7 THEN _7__year WHEN 8 THEN _8__year ELSE _9__year END AS year,
         CASE j.value WHEN 0 THEN _0__id WHEN 1 THEN _1__id WHEN 2 THEN _2__id WHEN 3 THEN _3__id WHEN 4 THEN _4__id
         WHEN 5 THEN _5__id WHEN 6 THEN _6__id WHEN 7 THEN _7__id WHEN 8 THEN _8__id ELSE _9__id END AS id
         FROM stats__books b, json_each('[0,1,2,3,4,5,6,7,8,9]') j WHERE b._key LIKE '[{"State":%') SELECT (SELECT code FROM states WHERE _row = json_extract(bk.k, '$[0].State')) AS grp,
         CASE WHEN bk.k LIKE '%WinStreak%' THEN 'Winning streak' WHEN bk.k LIKE '%Season%' THEN 'Season: ' || (SELECT stat FROM hst WHERE k = json_extract(bk.k, '$[1].Season'))
         ELSE 'Career: ' || (SELECT stat FROM hst WHERE k = json_extract(bk.k, '$[1].Career')) END AS book,
         bk.name AS holder, ROUND(bk.value, 1) AS value, bk.year
         FROM bk WHERE bk.i = 0 ORDER BY grp, book"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "N32.budgets",
        page: "high-school",
        area: "N32",
        title: r#"High school football spending by state"#,
        kind: "table",
        note: r#"Each state's high school football spending last money year beyond coaching stipends (equipment, travel, game day, field, booster purchases, meals; `finance.hs` last): the median, the top tenth and the biggest, with the median crowd. Real: a typical public program spends $25-60K; big Texas programs $150K+ (`finance_check.rs`)."#,
        sql: r#"WITH x AS (SELECT st.code AS state, (SELECT SUM(value) FROM json_each(f.last__l) WHERE key >= 6) AS v, f.crowd FROM finance__hs f JOIN high_schools h ON h._row = f._row JOIN states st ON st._row = h.state),
         r AS (SELECT state, v, crowd, ROW_NUMBER() OVER (PARTITION BY state ORDER BY v) AS r, ROW_NUMBER() OVER (PARTITION BY state ORDER BY crowd) AS rc, COUNT(*) OVER (PARTITION BY state) AS n FROM x)
         SELECT state, ROUND(MAX(CASE WHEN r = CAST(0.5 * (n - 1) AS INTEGER) + 1 THEN v END)) AS median, ROUND(MAX(CASE WHEN r = CAST(0.9 * (n - 1) AS INTEGER) + 1 THEN v END)) AS p90, ROUND(MAX(v)) AS biggest, MAX(CASE WHEN rc = CAST(0.5 * (n - 1) AS INTEGER) + 1 THEN crowd END) AS median_crowd, MAX(n) AS schools
         FROM r GROUP BY state ORDER BY median DESC"#,
        says: Some(
            r#"WITH x AS (SELECT (SELECT SUM(value) FROM json_each(f.last__l) WHERE key >= 6) AS v FROM finance__hs f), r AS (SELECT v, ROW_NUMBER() OVER (ORDER BY v) AS r, COUNT(*) OVER () AS n FROM x)
         SELECT printf('Median school spends $%,d a year beyond stipends (real public programs $25-60K); the top tenth $%,d or more.', MAX(CASE WHEN r = CAST(0.5 * (n - 1) AS INTEGER) + 1 THEN v END), MAX(CASE WHEN r = CAST(0.9 * (n - 1) AS INTEGER) + 1 THEN v END)) FROM r"#,
        ),
        needs: W,
        open: "",
    },
    Panel {
        id: "N32.lines",
        page: "high-school",
        area: "N32",
        title: r#"Where the money comes from and goes"#,
        kind: "bars",
        note: r#"Every school's books last money year summed by line (`finance.hs` LINES), in dollars a school."#,
        sql: r#"WITH l(k, name) AS (VALUES (0, 'District allocation'), (1, 'Gate receipts'), (2, 'Playoff gate share'), (3, 'Pay-to-play fees'), (4, 'Booster club'), (5, 'Coaching stipends'),
         (6, 'Equipment and reconditioning'), (7, 'Buses and travel'), (8, 'Officials and game day'), (9, 'Field, lights and upkeep'), (10, 'Booster purchases'), (11, 'Team meals'))
         SELECT l.name AS label, ROUND(AVG(j.value)) AS world, COUNT(*) AS n FROM finance__hs f, json_each(f.last__l) j JOIN l ON l.k = j.key GROUP BY l.k ORDER BY l.k"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S1.ages",
        page: "coaches",
        area: "S1",
        title: r#"Coaches' ages by level and role"#,
        kind: "hist",
        note: r#"Every coach in a job now by age (pick a level and role above). Real FBS first-time head coaches cluster in their 40s (no open count by year)."#,
        sql: r#"WITH cj AS MATERIALIZED (SELECT c.*, CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS'
         WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, p.prestige AS prestige
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL) SELECT level || ': ' || role AS grp, age AS x, COUNT(*) AS world FROM cj GROUP BY level, role, age ORDER BY CASE level WHEN 'NFL' THEN 0 WHEN 'FBS' THEN 1 WHEN 'FCS' THEN 2 WHEN 'D2' THEN 3 WHEN 'D3' THEN 4 WHEN 'NAIA' THEN 5 WHEN 'JUCO' THEN 6 WHEN 'HS' THEN 7 ELSE 8 END, role, age"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S1.age_table",
        page: "coaches",
        area: "S1",
        title: r#"Age by level and role"#,
        kind: "table",
        note: r#"The 10th, 50th and 90th percentile ages of coaches in a job now by level and role, and how many are ex-pros (design: HS 4%, NFL 35%, others 12%, `generate_coach`)."#,
        sql: r#"WITH cj AS MATERIALIZED (SELECT c.*, CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS'
         WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, p.prestige AS prestige
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL), r AS (SELECT level, role, age AS v, ex_pro, ROW_NUMBER() OVER (PARTITION BY level, role ORDER BY age) AS r, COUNT(*) OVER (PARTITION BY level, role) AS n FROM cj)
         SELECT level, role, MAX(n) AS coaches, MAX(CASE WHEN r = CAST(0.1 * (n - 1) AS INTEGER) + 1 THEN v END) AS p10, MAX(CASE WHEN r = CAST(0.5 * (n - 1) AS INTEGER) + 1 THEN v END) AS median, MAX(CASE WHEN r = CAST(0.9 * (n - 1) AS INTEGER) + 1 THEN v END) AS p90, ROUND(100.0 * AVG(ex_pro), 1) AS ex_pro_pct,
         CASE level WHEN 'HS' THEN 4 WHEN 'NFL' THEN 35 ELSE 12 END AS design_ex_pro_pct
         FROM r GROUP BY level, role ORDER BY CASE level WHEN 'NFL' THEN 0 WHEN 'FBS' THEN 1 WHEN 'FCS' THEN 2 WHEN 'D2' THEN 3 WHEN 'D3' THEN 4 WHEN 'NAIA' THEN 5 WHEN 'JUCO' THEN 6 WHEN 'HS' THEN 7 ELSE 8 END, role"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S1.first_head",
        page: "coaches",
        area: "S1",
        title: r#"Age at the first head job"#,
        kind: "hist",
        note: r#"Each coach's age when his first head job began (college, NFL and other leagues; high school apart), by the level of that job (pick above)."#,
        sql: r#"WITH st AS MATERIALIZED (SELECT h._parent AS c, h._i AS i, CASE WHEN h.role = 'HeadCoach' THEN 'Head coach' WHEN h.role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN h.role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN h.role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, h."from" AS a, h."to" AS z, h.wins AS w, h.losses AS l, h.titles AS titles, h.ended AS ended,
         CASE WHEN h.employer__College IS NOT NULL THEN COALESCE((SELECT x._2 FROM programs__history x WHERE x._parent = h.employer__College AND h."to" BETWEEN x._0 AND x._1), p.division)
         WHEN h.employer__HighSchool IS NOT NULL THEN 'HS' WHEN h.employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level,
         h.employer__College AS college, h.employer__HighSchool AS hs, h.employer__Nfl AS nfl
         FROM coaches__history h LEFT JOIN programs p ON p._row = h.employer__College), jobs AS MATERIALIZED (SELECT c, role, a, z, w, l, titles, ended, level, college, hs, nfl FROM st
         UNION ALL SELECT c._row, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END, c.job__since, (SELECT MAX(year) FROM programs__seasons), c.wins, c.losses, c.titles, 'current',
         CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS' WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END,
         c.job__employer__College, c.job__employer__HighSchool, c.job__employer__Nfl
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL), f AS (SELECT j.c, j.level, j.a, ROW_NUMBER() OVER (PARTITION BY j.c ORDER BY j.a) AS k FROM jobs j WHERE j.role = 'Head coach' AND j.level <> 'HS')
         SELECT f.level AS grp, f.a - COALESCE(CAST(strftime('%Y', date(c.born * 86400, 'unixepoch')) AS INTEGER), CASE WHEN c.retired = 1 THEN (SELECT MAX(h."to") FROM coaches__history h WHERE h._parent = +c._row) + 1 - c.age ELSE (SELECT MAX(year) FROM programs__seasons) + 1 - c.age END) AS x, COUNT(*) AS world FROM f JOIN coaches c ON c._row = f.c WHERE f.k = 1 AND f.a > (SELECT MIN(year) FROM programs__seasons)
         GROUP BY f.level, x ORDER BY f.level, x"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S1.ethnicity",
        page: "coaches",
        area: "S1",
        title: r#"Coaches by ethnicity, level and role now"#,
        kind: "table",
        note: r#"Shares of coaches in a job now by ethnicity, for each level and role."#,
        sql: r#"WITH cj AS MATERIALIZED (SELECT c.*, CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS'
         WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, p.prestige AS prestige
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL) SELECT level, role, COUNT(*) AS coaches, ROUND(100.0 * AVG(ethnicity = 'White'), 1) AS white_pct, ROUND(100.0 * AVG(ethnicity = 'Black'), 1) AS black_pct,
         ROUND(100.0 * AVG(ethnicity = 'Hispanic'), 1) AS hispanic_pct, ROUND(100.0 * AVG(ethnicity NOT IN ('White', 'Black', 'Hispanic')), 1) AS other_pct
         FROM cj GROUP BY level, role ORDER BY CASE level WHEN 'NFL' THEN 0 WHEN 'FBS' THEN 1 WHEN 'FCS' THEN 2 WHEN 'D2' THEN 3 WHEN 'D3' THEN 4 WHEN 'NAIA' THEN 5 WHEN 'JUCO' THEN 6 WHEN 'HS' THEN 7 ELSE 8 END, role"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S1.came_up",
        page: "coaches",
        area: "S1",
        title: r#"The group each coach came up coaching, by role"#,
        kind: "matrix",
        note: r#"Coaches in a college or NFL job now: the share of each role that came up coaching each group. Offensive coordinators should mostly come from offense, defensive coordinators from defense."#,
        sql: r#"WITH cj AS MATERIALIZED (SELECT c.*, CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS'
         WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, p.prestige AS prestige
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL) SELECT role AS row, came_up AS col, ROUND(100.0 * COUNT(*) / SUM(COUNT(*)) OVER (PARTITION BY role), 1) AS value FROM cj WHERE level <> 'HS' GROUP BY role, came_up ORDER BY role, came_up"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S1.home",
        page: "coaches",
        area: "S1",
        title: r#"Head coaches working in their home state"#,
        kind: "bars",
        note: r#"The share of head coaches now whose job is in the state he comes from (hidden home state), by level."#,
        sql: r#"WITH cj AS MATERIALIZED (SELECT c.*, CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS'
         WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, p.prestige AS prestige
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL) SELECT level AS label, ROUND(100.0 * AVG(cj.home_state = COALESCE((SELECT state FROM programs WHERE _row = cj.job__employer__College), (SELECT state FROM high_schools WHERE _row = cj.job__employer__HighSchool))), 1) AS world, COUNT(*) AS n
         FROM cj WHERE role = 'Head coach' AND level <> 'NFL' AND cj.home_state IS NOT NULL GROUP BY level ORDER BY CASE level WHEN 'NFL' THEN 0 WHEN 'FBS' THEN 1 WHEN 'FCS' THEN 2 WHEN 'D2' THEN 3 WHEN 'D3' THEN 4 WHEN 'NAIA' THEN 5 WHEN 'JUCO' THEN 6 WHEN 'HS' THEN 7 ELSE 8 END"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S2.levels",
        page: "coaches",
        area: "S2",
        title: r#"Hidden ratings by level and role (debug)"#,
        kind: "table",
        note: r#"Each level and role now: head coach value (`head_coach_value`), mean teaching, scheme, game management, recruiting and network, against the design's mean and spread for the level."#,
        sql: r#"WITH cj AS MATERIALIZED (SELECT c.*, CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS'
         WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, p.prestige AS prestige
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL), dz(level, mean, sd) AS (VALUES ('HS', 8.0, 2.6), ('D2', 9.5, 2.3), ('D3', 9.5, 2.3), ('NAIA', 9.5, 2.3), ('JUCO', 9.5, 2.3), ('FCS', 10.5, 2.2), ('FBS', 12.0, 2.3), ('NFL', 13.5, 2.0)) SELECT cj.level, cj.role, COUNT(*) AS coaches, ROUND(AVG((0.3 * (SELECT AVG(value) FROM json_each(ability__teaching)) + 0.2 * ability__scheme + 0.15 * ability__game_management
         + 0.15 * ability__motivation + 0.1 * ability__discipline + 0.1 * ability__relationships)), 1) AS hc_value, ROUND(AVG((SELECT AVG(value) FROM json_each(ability__teaching))), 1) AS teaching, ROUND(AVG(ability__scheme), 1) AS scheme,
         ROUND(AVG(ability__game_management), 1) AS game_management, ROUND(AVG(ability__recruiting), 1) AS recruiting, ROUND(AVG(ability__network), 1) AS network, dz.mean AS design_mean, dz.sd AS design_sd
         FROM cj LEFT JOIN dz ON dz.level = cj.level GROUP BY cj.level, cj.role ORDER BY CASE cj.level WHEN 'NFL' THEN 0 WHEN 'FBS' THEN 1 WHEN 'FCS' THEN 2 WHEN 'D2' THEN 3 WHEN 'D3' THEN 4 WHEN 'NAIA' THEN 5 WHEN 'JUCO' THEN 6 WHEN 'HS' THEN 7 ELSE 8 END, cj.role"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S2.value_wins",
        page: "coaches",
        area: "S2",
        title: r#"Head coach value against winning (debug)"#,
        kind: "bars",
        note: r#"Head coaches with 30+ college games as head coach: their win share by head coach value now, in bands. Talent drives more, so the link should be positive but loose."#,
        sql: r#"WITH st AS MATERIALIZED (SELECT h._parent AS c, h._i AS i, CASE WHEN h.role = 'HeadCoach' THEN 'Head coach' WHEN h.role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN h.role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN h.role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, h."from" AS a, h."to" AS z, h.wins AS w, h.losses AS l, h.titles AS titles, h.ended AS ended,
         CASE WHEN h.employer__College IS NOT NULL THEN COALESCE((SELECT x._2 FROM programs__history x WHERE x._parent = h.employer__College AND h."to" BETWEEN x._0 AND x._1), p.division)
         WHEN h.employer__HighSchool IS NOT NULL THEN 'HS' WHEN h.employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level,
         h.employer__College AS college, h.employer__HighSchool AS hs, h.employer__Nfl AS nfl
         FROM coaches__history h LEFT JOIN programs p ON p._row = h.employer__College), jobs AS MATERIALIZED (SELECT c, role, a, z, w, l, titles, ended, level, college, hs, nfl FROM st
         UNION ALL SELECT c._row, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END, c.job__since, (SELECT MAX(year) FROM programs__seasons), c.wins, c.losses, c.titles, 'current',
         CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS' WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END,
         c.job__employer__College, c.job__employer__HighSchool, c.job__employer__Nfl
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL), w AS (SELECT c, SUM(w) AS w, SUM(l) AS l FROM jobs WHERE role = 'Head coach' AND college IS NOT NULL GROUP BY c HAVING SUM(w + l) >= 30)
         SELECT CASE WHEN v < 9 THEN 'Value under 9' WHEN v < 11 THEN '9-11' WHEN v < 13 THEN '11-13' WHEN v < 15 THEN '13-15' ELSE '15+' END AS label, ROUND(1.0 * SUM(w.w) / SUM(w.w + w.l), 3) AS world, COUNT(*) AS n
         FROM w JOIN (SELECT _row, (0.3 * (SELECT AVG(value) FROM json_each(ability__teaching)) + 0.2 * ability__scheme + 0.15 * ability__game_management
         + 0.15 * ability__motivation + 0.1 * ability__discipline + 0.1 * ability__relationships) AS v FROM coaches) c ON c._row = w.c GROUP BY 1 ORDER BY MIN(v)"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S2.best_teachers",
        page: "coaches",
        area: "S2",
        title: r#"Best teacher of each group by level (debug)"#,
        kind: "table",
        note: r#"The best teacher of each group among coaches in a job now at each level, with his teaching and his job (his rooms' growth is on the unit records)."#,
        sql: r#"WITH cj AS MATERIALIZED (SELECT c.*, CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS'
         WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, p.prestige AS prestige
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL), grp(k, grp) AS (VALUES (0, 'QB'), (1, 'RB'), (2, 'Receivers'), (3, 'OL'), (4, 'DL'), (5, 'LB'), (6, 'DB'), (7, 'Specialists')), t AS (SELECT cj._row AS id, cj.name, cj.level, cj.role, g.key AS k, g.value AS v, ROW_NUMBER() OVER (PARTITION BY cj.level, g.key ORDER BY g.value DESC) AS rk FROM cj, json_each(cj.ability__teaching) g WHERE cj.level IN ('NFL', 'FBS', 'FCS', 'HS'))
         SELECT t.level, grp.grp AS grp_taught, t.id, t.name, t.role, ROUND(t.v, 1) AS teaching FROM t JOIN grp ON grp.k = t.k
         WHERE t.rk = 1 ORDER BY CASE t.level WHEN 'NFL' THEN 0 WHEN 'FBS' THEN 1 WHEN 'FCS' THEN 2 ELSE 3 END, t.k"#,
        says: None,
        needs: W,
        open: "e/coaches/{id}",
    },
    Panel {
        id: "S2.recruiters",
        page: "coaches",
        area: "S2",
        title: r#"The best recruiters and where they work (debug)"#,
        kind: "table",
        note: r#"The 40 best recruiters in a job now. Recruiting is not in head coach value, so an elite recruiter with a low value shows whether big programs still hire him."#,
        sql: r#"WITH cj AS MATERIALIZED (SELECT c.*, CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS'
         WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, p.prestige AS prestige
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL) SELECT _row AS id, name, level, role, ROUND(ability__recruiting, 1) AS recruiting, ROUND((0.3 * (SELECT AVG(value) FROM json_each(ability__teaching)) + 0.2 * ability__scheme + 0.15 * ability__game_management
         + 0.15 * ability__motivation + 0.1 * ability__discipline + 0.1 * ability__relationships), 1) AS hc_value, ROUND(prestige, 2) AS prestige
         FROM cj ORDER BY ability__recruiting DESC LIMIT 40"#,
        says: None,
        needs: W,
        open: "e/coaches/{id}",
    },
    Panel {
        id: "S2.sportsmanship",
        page: "coaches",
        area: "S2",
        title: r#"Sportsmanship spread (debug)"#,
        kind: "hist",
        note: r#"Every coach's sportsmanship (design: 10 + 3.5 a spread), which sets when starters come out in blowouts."#,
        sql: r#"SELECT ROUND(sportsmanship) AS x, COUNT(*) AS world FROM coaches WHERE sportsmanship IS NOT NULL GROUP BY 1 ORDER BY 1"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S3.by_age",
        page: "coaches",
        area: "S3",
        title: r#"Ratings by age (debug)"#,
        kind: "line",
        note: r#"Mean hidden parts by age over every coach-year in the world log (`log_coaches`): rising under 35, flat in the late 40s and 50s, falling after about 59, recruiting first."#,
        sql: r#"SELECT coach__age AS year, ROUND(AVG((coach__ability__teaching__0 + coach__ability__teaching__1 + coach__ability__teaching__2 + coach__ability__teaching__3 + coach__ability__teaching__4
         + coach__ability__teaching__5 + coach__ability__teaching__6 + coach__ability__teaching__7) / 8.0), 2) AS teaching, ROUND(AVG(coach__ability__scheme), 2) AS scheme,
         ROUND(AVG(coach__ability__game_management), 2) AS game_management, ROUND(AVG(coach__ability__recruiting), 2) AS recruiting, ROUND(AVG(coach__ability__network), 2) AS network, COUNT(*) AS n
         FROM log_coaches WHERE coach__age BETWEEN 24 AND 80 AND COALESCE(coach__retired, 0) = 0 GROUP BY coach__age ORDER BY coach__age"#,
        says: None,
        needs: WL,
        open: "",
    },
    Panel {
        id: "S3.old",
        page: "coaches",
        area: "S3",
        title: r#"Coaches still working by age"#,
        kind: "bars",
        note: r#"Of all coaches of each age band, the share in a job now; out-of-work coaches over 66 retire."#,
        sql: r#"SELECT CASE WHEN age < 40 THEN 'Under 40' WHEN age < 50 THEN '40-49' WHEN age < 60 THEN '50-59' WHEN age < 65 THEN '60-64' WHEN age < 70 THEN '65-69' ELSE '70+' END AS label,
         ROUND(100.0 * AVG(job__role IS NOT NULL OR job__role__PositionCoach IS NOT NULL), 1) AS world, ROUND(100.0 * AVG(COALESCE(retired, 0)), 1) AS retired_pct, COUNT(*) AS n
         FROM coaches GROUP BY 1 ORDER BY MIN(age)"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S3.ceiling",
        page: "coaches",
        area: "S3",
        title: r#"Room left to grow (debug)"#,
        kind: "bars",
        note: r#"Coaches in a job by age band: the mean room between his scheme, game management and mean teaching and his learning ceiling (growth only fills room under the ceiling; a part drawn above it stays), and the share with no room left in any of the three."#,
        sql: r#"WITH cj AS MATERIALIZED (SELECT c.*, CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS'
         WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, p.prestige AS prestige
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL), x AS (SELECT age, learning_ceiling - ability__scheme AS a, learning_ceiling - ability__game_management AS b, learning_ceiling - (SELECT AVG(value) FROM json_each(ability__teaching)) AS t FROM cj)
         SELECT CASE WHEN age < 35 THEN 'Under 35' WHEN age < 45 THEN '35-44' WHEN age < 55 THEN '45-54' ELSE '55+' END AS label,
         ROUND(AVG((MAX(a, 0) + MAX(b, 0) + MAX(t, 0)) / 3.0), 2) AS world, ROUND(100.0 * AVG(a <= 0.5 AND b <= 0.5 AND t <= 0.5), 1) AS no_room_pct, COUNT(*) AS n
         FROM x GROUP BY 1 ORDER BY MIN(age)"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S4.loyalty",
        page: "coaches",
        area: "S4",
        title: r#"Loyalty against moving on"#,
        kind: "bars",
        note: r#"Coaches with 10+ seasons of jobs by loyalty band: jobs per decade and the share of jobs that ended with him leaving for another job."#,
        sql: r#"WITH st AS MATERIALIZED (SELECT h._parent AS c, h._i AS i, CASE WHEN h.role = 'HeadCoach' THEN 'Head coach' WHEN h.role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN h.role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN h.role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, h."from" AS a, h."to" AS z, h.wins AS w, h.losses AS l, h.titles AS titles, h.ended AS ended,
         CASE WHEN h.employer__College IS NOT NULL THEN COALESCE((SELECT x._2 FROM programs__history x WHERE x._parent = h.employer__College AND h."to" BETWEEN x._0 AND x._1), p.division)
         WHEN h.employer__HighSchool IS NOT NULL THEN 'HS' WHEN h.employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level,
         h.employer__College AS college, h.employer__HighSchool AS hs, h.employer__Nfl AS nfl
         FROM coaches__history h LEFT JOIN programs p ON p._row = h.employer__College), jobs AS MATERIALIZED (SELECT c, role, a, z, w, l, titles, ended, level, college, hs, nfl FROM st
         UNION ALL SELECT c._row, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END, c.job__since, (SELECT MAX(year) FROM programs__seasons), c.wins, c.losses, c.titles, 'current',
         CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS' WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END,
         c.job__employer__College, c.job__employer__HighSchool, c.job__employer__Nfl
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL), x AS (SELECT c, COUNT(*) AS jobs, SUM(ended = 'left for another job') AS left_for, MAX(z) - MIN(a) + 1 AS span FROM jobs GROUP BY c HAVING span >= 10)
         SELECT CASE WHEN co.personality__loyalty < 7 THEN 'Loyalty under 7' WHEN co.personality__loyalty < 10 THEN '7-10' WHEN co.personality__loyalty < 13 THEN '10-13' ELSE '13+' END AS label,
         ROUND(AVG(10.0 * x.jobs / x.span), 2) AS world, ROUND(100.0 * SUM(x.left_for) / SUM(x.jobs), 1) AS left_for_pct, COUNT(*) AS n
         FROM x JOIN coaches co ON co._row = x.c GROUP BY 1 ORDER BY MIN(co.personality__loyalty)"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S4.ambition",
        page: "coaches",
        area: "S4",
        title: r#"Ambition against climbing"#,
        kind: "bars",
        note: r#"Coaches with 10+ seasons of jobs by ambition band: the share who ever held an FBS or NFL head job."#,
        sql: r#"WITH st AS MATERIALIZED (SELECT h._parent AS c, h._i AS i, CASE WHEN h.role = 'HeadCoach' THEN 'Head coach' WHEN h.role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN h.role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN h.role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, h."from" AS a, h."to" AS z, h.wins AS w, h.losses AS l, h.titles AS titles, h.ended AS ended,
         CASE WHEN h.employer__College IS NOT NULL THEN COALESCE((SELECT x._2 FROM programs__history x WHERE x._parent = h.employer__College AND h."to" BETWEEN x._0 AND x._1), p.division)
         WHEN h.employer__HighSchool IS NOT NULL THEN 'HS' WHEN h.employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level,
         h.employer__College AS college, h.employer__HighSchool AS hs, h.employer__Nfl AS nfl
         FROM coaches__history h LEFT JOIN programs p ON p._row = h.employer__College), jobs AS MATERIALIZED (SELECT c, role, a, z, w, l, titles, ended, level, college, hs, nfl FROM st
         UNION ALL SELECT c._row, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END, c.job__since, (SELECT MAX(year) FROM programs__seasons), c.wins, c.losses, c.titles, 'current',
         CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS' WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END,
         c.job__employer__College, c.job__employer__HighSchool, c.job__employer__Nfl
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL), x AS (SELECT c, MAX(role = 'Head coach' AND level IN ('FBS', 'NFL')) AS top, MAX(z) - MIN(a) + 1 AS span FROM jobs GROUP BY c HAVING span >= 10)
         SELECT CASE WHEN co.personality__ambition < 7 THEN 'Ambition under 7' WHEN co.personality__ambition < 10 THEN '7-10' WHEN co.personality__ambition < 13 THEN '10-13' ELSE '13+' END AS label,
         ROUND(100.0 * AVG(x.top), 1) AS world, COUNT(*) AS n FROM x JOIN coaches co ON co._row = x.c GROUP BY 1 ORDER BY MIN(co.personality__ambition)"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S4.temper",
        page: "coaches",
        area: "S4",
        title: r#"Temper against health episodes"#,
        kind: "bars",
        note: r#"Head coaches and coordinators in a job now by temper band: health episodes per 100 coaches in the last 400 days (`folk.life.health`)."#,
        sql: r#"WITH cj AS MATERIALIZED (SELECT c.*, CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS'
         WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, p.prestige AS prestige
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL) SELECT CASE WHEN personality__temper < 7 THEN 'Temper under 7' WHEN personality__temper < 10 THEN '7-10' WHEN personality__temper < 13 THEN '10-13' ELSE '13+' END AS label,
         ROUND(100.0 * SUM((SELECT COUNT(*) FROM folk__life__health h WHERE h.coach = +cj._row)) / COUNT(*), 2) AS world, COUNT(*) AS n
         FROM cj WHERE role <> 'Position coach' GROUP BY 1 ORDER BY MIN(personality__temper)"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S4.traits",
        page: "coaches",
        area: "S4",
        title: r#"Ambition, loyalty and temper"#,
        kind: "hist",
        note: r#"Each trait's spread over every coach (design N(10, 3.5)); pick a trait above."#,
        sql: r#"SELECT 'Ambition' AS grp, ROUND(personality__ambition) AS x, COUNT(*) AS world FROM coaches GROUP BY 2
         UNION ALL SELECT 'Loyalty', ROUND(personality__loyalty), COUNT(*) FROM coaches GROUP BY 2
         UNION ALL SELECT 'Temper', ROUND(personality__temper), COUNT(*) FROM coaches GROUP BY 2 ORDER BY 1, 2"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S5.gap_by_tenure",
        page: "coaches",
        area: "S5",
        title: r#"Reputation against true value by seasons as head coach (debug)"#,
        kind: "bars",
        note: r#"College head coaches now: the mean gap between reputation and head coach value (absolute) by seasons as a head coach. Reputation chases value, so the gap should shrink."#,
        sql: r#"WITH st AS MATERIALIZED (SELECT h._parent AS c, h._i AS i, CASE WHEN h.role = 'HeadCoach' THEN 'Head coach' WHEN h.role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN h.role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN h.role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, h."from" AS a, h."to" AS z, h.wins AS w, h.losses AS l, h.titles AS titles, h.ended AS ended,
         CASE WHEN h.employer__College IS NOT NULL THEN COALESCE((SELECT x._2 FROM programs__history x WHERE x._parent = h.employer__College AND h."to" BETWEEN x._0 AND x._1), p.division)
         WHEN h.employer__HighSchool IS NOT NULL THEN 'HS' WHEN h.employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level,
         h.employer__College AS college, h.employer__HighSchool AS hs, h.employer__Nfl AS nfl
         FROM coaches__history h LEFT JOIN programs p ON p._row = h.employer__College), jobs AS MATERIALIZED (SELECT c, role, a, z, w, l, titles, ended, level, college, hs, nfl FROM st
         UNION ALL SELECT c._row, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END, c.job__since, (SELECT MAX(year) FROM programs__seasons), c.wins, c.losses, c.titles, 'current',
         CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS' WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END,
         c.job__employer__College, c.job__employer__HighSchool, c.job__employer__Nfl
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL), t AS (SELECT c, SUM(z - a + 1) AS seasons FROM jobs WHERE role = 'Head coach' AND college IS NOT NULL GROUP BY c)
         SELECT CASE WHEN t.seasons <= 2 THEN '1-2 seasons' WHEN t.seasons <= 5 THEN '3-5' WHEN t.seasons <= 10 THEN '6-10' ELSE '11+' END AS label,
         ROUND(AVG(ABS(co.reputation - ((0.3 * (SELECT AVG(value) FROM json_each(co.ability__teaching)) + 0.2 * co.ability__scheme + 0.15 * co.ability__game_management
         + 0.15 * co.ability__motivation + 0.1 * co.ability__discipline + 0.1 * co.ability__relationships)))), 2) AS world, COUNT(*) AS n
         FROM t JOIN coaches co ON co._row = t.c WHERE co.job__role = 'HeadCoach' AND co.job__employer__College IS NOT NULL GROUP BY 1 ORDER BY MIN(t.seasons)"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S5.over_under",
        page: "coaches",
        area: "S5",
        title: r#"The most overrated and underrated FBS head coaches (debug)"#,
        kind: "table",
        note: r#"FBS head coaches now ranked by reputation less head coach value: the top is overrated, the bottom underrated, each with his record and last six seasons against expectation."#,
        sql: r#"WITH cj AS MATERIALIZED (SELECT c.*, CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS'
         WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, p.prestige AS prestige
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL) SELECT _row AS id, name, (SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = cj.job__employer__College) AS program, ROUND(reputation, 1) AS reputation,
         ROUND((0.3 * (SELECT AVG(value) FROM json_each(ability__teaching)) + 0.2 * ability__scheme + 0.15 * ability__game_management
         + 0.15 * ability__motivation + 0.1 * ability__discipline + 0.1 * ability__relationships), 1) AS hc_value, ROUND(reputation - ((0.3 * (SELECT AVG(value) FROM json_each(ability__teaching)) + 0.2 * ability__scheme + 0.15 * ability__game_management
         + 0.15 * ability__motivation + 0.1 * ability__discipline + 0.1 * ability__relationships)), 1) AS gap, wins || '-' || losses AS record_here, career_titles AS titles, recent AS last_six_vs_expectation
         FROM cj WHERE level = 'FBS' AND role = 'Head coach' ORDER BY gap DESC"#,
        says: None,
        needs: W,
        open: "e/coaches/{id}",
    },
    Panel {
        id: "S5.by_year",
        page: "coaches",
        area: "S5",
        title: r#"Reputation of FBS head coaches by year"#,
        kind: "line",
        note: r#"The world log's yearly coach rows: the 10th, 50th and 90th percentile reputation of FBS head coaches each year."#,
        sql: r#"WITH x AS (SELECT l.season AS year, l.coach__reputation AS v FROM log_coaches l JOIN programs p ON p._row = l.coach__job__employer__College WHERE l.coach__job__role = 'HeadCoach' AND p.division = 'FBS'),
         r AS (SELECT year, v, ROW_NUMBER() OVER (PARTITION BY year ORDER BY v) AS r, COUNT(*) OVER (PARTITION BY year) AS n FROM x)
         SELECT year, MAX(CASE WHEN r = CAST(0.1 * (n - 1) AS INTEGER) + 1 THEN v END) AS p10, MAX(CASE WHEN r = CAST(0.5 * (n - 1) AS INTEGER) + 1 THEN v END) AS p50, MAX(CASE WHEN r = CAST(0.9 * (n - 1) AS INTEGER) + 1 THEN v END) AS p90 FROM r GROUP BY year ORDER BY year"#,
        says: None,
        needs: WL,
        open: "",
    },
    Panel {
        id: "S5.words",
        page: "coaches",
        area: "S5",
        title: r#"Reputation words by level"#,
        kind: "matrix",
        note: r#"The words the game shows for each coach in a job now (17+ 'one of the best in the game', 14-17 'highly regarded', 10-14 'respected', 5-10 'unproven' bands, under 5 'unproven'; `rep_words`), as shares of each level."#,
        sql: r#"WITH cj AS MATERIALIZED (SELECT c.*, CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS'
         WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, p.prestige AS prestige
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL) SELECT level AS row, CASE WHEN reputation >= 17 THEN '17+' WHEN reputation >= 14 THEN '14-17' WHEN reputation >= 10 THEN '10-14' WHEN reputation >= 5 THEN '5-10' ELSE 'under 5' END AS col,
         ROUND(100.0 * COUNT(*) / SUM(COUNT(*)) OVER (PARTITION BY level), 1) AS value FROM cj GROUP BY 1, 2 ORDER BY CASE level WHEN 'NFL' THEN 0 WHEN 'FBS' THEN 1 WHEN 'FCS' THEN 2 WHEN 'D2' THEN 3 WHEN 'D3' THEN 4 WHEN 'NAIA' THEN 5 WHEN 'JUCO' THEN 6 WHEN 'HS' THEN 7 ELSE 8 END, 2"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S6.leaders",
        page: "coaches",
        area: "S6",
        title: r#"Buzz from unit stats"#,
        kind: "table",
        note: r#"Every buzz row, newest first: the coach, his program, the side, the gain and its why (`stats.buzz`), and his next job after that season."#,
        sql: r#"WITH st AS MATERIALIZED (SELECT h._parent AS c, h._i AS i, CASE WHEN h.role = 'HeadCoach' THEN 'Head coach' WHEN h.role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN h.role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN h.role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, h."from" AS a, h."to" AS z, h.wins AS w, h.losses AS l, h.titles AS titles, h.ended AS ended,
         CASE WHEN h.employer__College IS NOT NULL THEN COALESCE((SELECT x._2 FROM programs__history x WHERE x._parent = h.employer__College AND h."to" BETWEEN x._0 AND x._1), p.division)
         WHEN h.employer__HighSchool IS NOT NULL THEN 'HS' WHEN h.employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level,
         h.employer__College AS college, h.employer__HighSchool AS hs, h.employer__Nfl AS nfl
         FROM coaches__history h LEFT JOIN programs p ON p._row = h.employer__College), jobs AS MATERIALIZED (SELECT c, role, a, z, w, l, titles, ended, level, college, hs, nfl FROM st
         UNION ALL SELECT c._row, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END, c.job__since, (SELECT MAX(year) FROM programs__seasons), c.wins, c.losses, c.titles, 'current',
         CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS' WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END,
         c.job__employer__College, c.job__employer__HighSchool, c.job__employer__Nfl
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL) SELECT b.year, b.coach AS id, c.name AS coach, (SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = b.program) AS program, CASE WHEN b.offense = 1 THEN 'offense' ELSE 'defense' END AS side,
         ROUND(b.gain, 2) AS gain, b.why, (SELECT j.role || ' at ' || COALESCE((SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = j.college), j.level) || ' (' || j.a || ')' FROM jobs j
         WHERE j.c = b.coach AND j.a > b.year ORDER BY j.a LIMIT 1) AS next_job
         FROM stats__buzz b JOIN coaches c ON c._row = b.coach ORDER BY b.year DESC, b.gain DESC"#,
        says: None,
        needs: W,
        open: "e/coaches/{id}",
    },
    Panel {
        id: "S6.share",
        page: "coaches",
        area: "S6",
        title: r#"Units that earned buzz a year"#,
        kind: "line",
        note: r#"Each year: coaches gaining buzz, losing it, and the mean gain (`stats.buzz`). About a quarter of units should earn some."#,
        sql: r#"SELECT year, SUM(gain > 0) AS gained, SUM(gain < 0) AS lost, ROUND(AVG(gain), 3) AS mean_gain FROM stats__buzz GROUP BY year ORDER BY year"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S6.next",
        page: "coaches",
        area: "S6",
        title: r#"Buzz and the next head job"#,
        kind: "bars",
        note: r#"Coordinators in a buzz row by their gain that year: the share who became a head coach within two years."#,
        sql: r#"WITH x AS (SELECT b.coach, b.year, b.gain, EXISTS (SELECT 1 FROM coaches__history h WHERE h._parent = b.coach AND h.role = 'HeadCoach' AND h."from" BETWEEN b.year + 1 AND b.year + 2)
         OR EXISTS (SELECT 1 FROM coaches c WHERE c._row = b.coach AND c.job__role = 'HeadCoach' AND c.job__since BETWEEN b.year + 1 AND b.year + 2) AS head FROM stats__buzz b WHERE b.year <= (SELECT MAX(year) FROM programs__seasons) - 2)
         SELECT CASE WHEN gain >= 2 THEN 'Gain 2+' WHEN gain >= 1 THEN '1-2' WHEN gain > 0 THEN 'under 1' ELSE 'lost buzz' END AS label, ROUND(100.0 * AVG(head), 1) AS world, COUNT(*) AS n FROM x GROUP BY 1 ORDER BY MIN(gain) DESC"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S7.tenure",
        page: "coaches",
        area: "S7",
        title: r#"How long head coaches last"#,
        kind: "bars",
        note: r#"Finished FBS head coaching runs (seasons as the season's coach in a row at one program) that began after the world's first season: the share lasting at least 2, 3, 4, 5, 6, 8 and 10 seasons, against real 2005-2019 starts (`college_people.json`)."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1), runs AS MATERIALIZED (SELECT prog, name, division, coach, MIN(year) AS a, MAX(year) AS z, COUNT(*) AS n FROM (SELECT prog, name, division, coach, year,
         year - ROW_NUMBER() OVER (PARTITION BY prog, coach ORDER BY year) AS g FROM ps WHERE coach IS NOT NULL) GROUP BY prog, coach, g),
         ended AS (SELECT r.*, (SELECT h.ended FROM coaches__history h WHERE h._parent = r.coach AND h.employer__College = r.prog AND h."to" >= r.z ORDER BY h."to" LIMIT 1) AS ended FROM runs r
         WHERE r.a > (SELECT MIN(year) FROM ps) AND r.z < (SELECT MAX(year) FROM ps)), k(k) AS (VALUES (2), (3), (4), (5), (6), (8), (10))
         SELECT 'At least ' || k.k || ' seasons' AS label, ROUND(100.0 * AVG(e.n >= k.k), 1) AS world,
         100.0 * json_extract((SELECT json FROM real_data WHERE file = 'college_people.json'), '$.fbs_head_coaches.share_lasting_at_least."' || k.k || '"') AS real, COUNT(*) AS n FROM ended e, k WHERE e.division = 'FBS' GROUP BY k.k ORDER BY k.k"#,
        says: Some(
            r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1), runs AS MATERIALIZED (SELECT prog, name, division, coach, MIN(year) AS a, MAX(year) AS z, COUNT(*) AS n FROM (SELECT prog, name, division, coach, year,
         year - ROW_NUMBER() OVER (PARTITION BY prog, coach ORDER BY year) AS g FROM ps WHERE coach IS NOT NULL) GROUP BY prog, coach, g),
         ended AS (SELECT r.*, (SELECT h.ended FROM coaches__history h WHERE h._parent = r.coach AND h.employer__College = r.prog AND h."to" >= r.z ORDER BY h."to" LIMIT 1) AS ended FROM runs r
         WHERE r.a > (SELECT MIN(year) FROM ps) AND r.z < (SELECT MAX(year) FROM ps)), r AS (SELECT n AS v, ROW_NUMBER() OVER (ORDER BY n) AS r, COUNT(*) OVER () AS n FROM ended WHERE division = 'FBS')
         SELECT printf('Finished FBS head coaching runs last %d seasons at the median (real 4); %.0f%% reach five (real 43%%).', MAX(CASE WHEN r = CAST(0.5 * (n - 1) AS INTEGER) + 1 THEN v END), 100.0 * AVG(v >= 5)) FROM r HAVING COUNT(*) > 0"#,
        ),
        needs: W,
        open: "",
    },
    Panel {
        id: "S7.endings",
        page: "coaches",
        area: "S7",
        title: r#"How jobs ended, by level"#,
        kind: "matrix",
        note: r#"Every finished job by its level and how it ended, as shares of the level's endings."#,
        sql: r#"WITH st AS MATERIALIZED (SELECT h._parent AS c, h._i AS i, CASE WHEN h.role = 'HeadCoach' THEN 'Head coach' WHEN h.role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN h.role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN h.role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, h."from" AS a, h."to" AS z, h.wins AS w, h.losses AS l, h.titles AS titles, h.ended AS ended,
         CASE WHEN h.employer__College IS NOT NULL THEN COALESCE((SELECT x._2 FROM programs__history x WHERE x._parent = h.employer__College AND h."to" BETWEEN x._0 AND x._1), p.division)
         WHEN h.employer__HighSchool IS NOT NULL THEN 'HS' WHEN h.employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level,
         h.employer__College AS college, h.employer__HighSchool AS hs, h.employer__Nfl AS nfl
         FROM coaches__history h LEFT JOIN programs p ON p._row = h.employer__College) SELECT level AS row, ended AS col, ROUND(100.0 * COUNT(*) / SUM(COUNT(*)) OVER (PARTITION BY level), 1) AS value FROM st GROUP BY level, ended ORDER BY CASE level WHEN 'NFL' THEN 0 WHEN 'FBS' THEN 1 WHEN 'FCS' THEN 2 WHEN 'D2' THEN 3 WHEN 'D3' THEN 4 WHEN 'NAIA' THEN 5 WHEN 'JUCO' THEN 6 WHEN 'HS' THEN 7 ELSE 8 END, value DESC"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S7.endings_by_year",
        page: "coaches",
        area: "S7",
        title: r#"FBS head jobs ended a year, by how"#,
        kind: "line",
        note: r#"Each season: FBS head jobs that ended, by how (fired, left for another job, retired, other)."#,
        sql: r#"WITH st AS MATERIALIZED (SELECT h._parent AS c, h._i AS i, CASE WHEN h.role = 'HeadCoach' THEN 'Head coach' WHEN h.role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN h.role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN h.role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, h."from" AS a, h."to" AS z, h.wins AS w, h.losses AS l, h.titles AS titles, h.ended AS ended,
         CASE WHEN h.employer__College IS NOT NULL THEN COALESCE((SELECT x._2 FROM programs__history x WHERE x._parent = h.employer__College AND h."to" BETWEEN x._0 AND x._1), p.division)
         WHEN h.employer__HighSchool IS NOT NULL THEN 'HS' WHEN h.employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level,
         h.employer__College AS college, h.employer__HighSchool AS hs, h.employer__Nfl AS nfl
         FROM coaches__history h LEFT JOIN programs p ON p._row = h.employer__College) SELECT z AS year, SUM(ended LIKE 'fired%') AS fired, SUM(ended = 'left for another job') AS left_for_another_job, SUM(ended LIKE 'retired%' OR ended LIKE 'stepped away%' OR ended LIKE 'walked away%') AS retired,
         SUM(NOT (ended LIKE 'fired%' OR ended = 'left for another job' OR ended LIKE 'retired%' OR ended LIKE 'stepped away%' OR ended LIKE 'walked away%')) AS other
         FROM st WHERE role = 'Head coach' AND level = 'FBS' GROUP BY z ORDER BY z"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S7.win_by_end",
        page: "coaches",
        area: "S7",
        title: r#"Win share by how a head job ended"#,
        kind: "bars",
        note: r#"College head jobs of 20+ games by how they ended: their win share. Fired coaches should sit well below those hired away."#,
        sql: r#"WITH st AS MATERIALIZED (SELECT h._parent AS c, h._i AS i, CASE WHEN h.role = 'HeadCoach' THEN 'Head coach' WHEN h.role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN h.role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN h.role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, h."from" AS a, h."to" AS z, h.wins AS w, h.losses AS l, h.titles AS titles, h.ended AS ended,
         CASE WHEN h.employer__College IS NOT NULL THEN COALESCE((SELECT x._2 FROM programs__history x WHERE x._parent = h.employer__College AND h."to" BETWEEN x._0 AND x._1), p.division)
         WHEN h.employer__HighSchool IS NOT NULL THEN 'HS' WHEN h.employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level,
         h.employer__College AS college, h.employer__HighSchool AS hs, h.employer__Nfl AS nfl
         FROM coaches__history h LEFT JOIN programs p ON p._row = h.employer__College) SELECT ended AS label, ROUND(1.0 * SUM(w) / SUM(w + l), 3) AS world, COUNT(*) AS n FROM st WHERE role = 'Head coach' AND college IS NOT NULL AND w + l >= 20 GROUP BY ended ORDER BY world DESC"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S7.kept_losing",
        page: "coaches",
        area: "S7",
        title: r#"Kept while losing"#,
        kind: "table",
        note: r#"FBS head jobs of 55+ games under .400: they should be rare."#,
        sql: r#"WITH st AS MATERIALIZED (SELECT h._parent AS c, h._i AS i, CASE WHEN h.role = 'HeadCoach' THEN 'Head coach' WHEN h.role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN h.role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN h.role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, h."from" AS a, h."to" AS z, h.wins AS w, h.losses AS l, h.titles AS titles, h.ended AS ended,
         CASE WHEN h.employer__College IS NOT NULL THEN COALESCE((SELECT x._2 FROM programs__history x WHERE x._parent = h.employer__College AND h."to" BETWEEN x._0 AND x._1), p.division)
         WHEN h.employer__HighSchool IS NOT NULL THEN 'HS' WHEN h.employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level,
         h.employer__College AS college, h.employer__HighSchool AS hs, h.employer__Nfl AS nfl
         FROM coaches__history h LEFT JOIN programs p ON p._row = h.employer__College), jobs AS MATERIALIZED (SELECT c, role, a, z, w, l, titles, ended, level, college, hs, nfl FROM st
         UNION ALL SELECT c._row, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END, c.job__since, (SELECT MAX(year) FROM programs__seasons), c.wins, c.losses, c.titles, 'current',
         CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS' WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END,
         c.job__employer__College, c.job__employer__HighSchool, c.job__employer__Nfl
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL) SELECT j.c AS id, c.name, (SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = j.college) AS program, j.a || '-' || j.z AS seasons, j.w || '-' || j.l AS record,
         ROUND(1.0 * j.w / (j.w + j.l), 3) AS pct, j.ended FROM jobs j JOIN coaches c ON c._row = j.c WHERE j.role = 'Head coach' AND j.level = 'FBS' AND j.w + j.l >= 55 AND 1.0 * j.w / (j.w + j.l) < 0.4 ORDER BY pct"#,
        says: None,
        needs: W,
        open: "e/coaches/{id}",
    },
    Panel {
        id: "S7.titles",
        page: "coaches",
        area: "S7",
        title: r#"The most titled head coaches"#,
        kind: "table",
        note: r#"Head coaches by national titles in the world's seasons (the season's coach), with the most at one program in any 15 seasons. Real: Saban 7 in his career; no program more than 6 in 15 years."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1), t AS (SELECT coach, pid, name, year, division FROM ps WHERE nt = 1 AND coach IS NOT NULL),
         w AS (SELECT a.coach, a.pid, COUNT(b.year) AS n FROM t a JOIN t b ON b.coach = a.coach AND b.pid = a.pid AND b.year BETWEEN a.year AND a.year + 14 GROUP BY a.coach, a.pid, a.year)
         SELECT t.coach AS id, c.name, t.division, COUNT(*) AS titles, group_concat(t.name || ' ' || t.year, ', ') AS seasons, (SELECT MAX(n) FROM w WHERE w.coach = t.coach) AS most_at_one_school_in_15
         FROM t JOIN coaches c ON c._row = t.coach GROUP BY t.coach, t.division ORDER BY titles DESC LIMIT 50"#,
        says: None,
        needs: W,
        open: "e/coaches/{id}",
    },
    Panel {
        id: "S7.list",
        page: "coaches",
        area: "S7",
        title: r#"Every coach"#,
        kind: "table",
        note: r#"Every coach: his job now, career record as head coach, titles, reputation and age. A row opens his page with every job and why it ended."#,
        sql: r#"WITH st AS MATERIALIZED (SELECT h._parent AS c, h._i AS i, CASE WHEN h.role = 'HeadCoach' THEN 'Head coach' WHEN h.role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN h.role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN h.role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, h."from" AS a, h."to" AS z, h.wins AS w, h.losses AS l, h.titles AS titles, h.ended AS ended,
         CASE WHEN h.employer__College IS NOT NULL THEN COALESCE((SELECT x._2 FROM programs__history x WHERE x._parent = h.employer__College AND h."to" BETWEEN x._0 AND x._1), p.division)
         WHEN h.employer__HighSchool IS NOT NULL THEN 'HS' WHEN h.employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level,
         h.employer__College AS college, h.employer__HighSchool AS hs, h.employer__Nfl AS nfl
         FROM coaches__history h LEFT JOIN programs p ON p._row = h.employer__College), jobs AS MATERIALIZED (SELECT c, role, a, z, w, l, titles, ended, level, college, hs, nfl FROM st
         UNION ALL SELECT c._row, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END, c.job__since, (SELECT MAX(year) FROM programs__seasons), c.wins, c.losses, c.titles, 'current',
         CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS' WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END,
         c.job__employer__College, c.job__employer__HighSchool, c.job__employer__Nfl
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL), r AS (SELECT c, SUM(CASE WHEN role = 'Head coach' THEN w ELSE 0 END) AS w, SUM(CASE WHEN role = 'Head coach' THEN l ELSE 0 END) AS l, COUNT(*) AS jobs FROM jobs GROUP BY c)
         SELECT c._row AS id, c.name, c.age, COALESCE(CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END, CASE WHEN c.retired = 1 THEN 'retired' ELSE 'out of work' END) AS job,
         COALESCE((SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = c.job__employer__College), (SELECT name FROM high_schools WHERE _row = c.job__employer__HighSchool), (SELECT name FROM nfl WHERE _row = c.job__employer__Nfl)) AS at,
         r.w || '-' || r.l AS head_record, c.career_titles AS titles, r.jobs, ROUND(c.reputation, 1) AS reputation
         FROM coaches c LEFT JOIN r ON r.c = c._row WHERE r.jobs > 0 ORDER BY c.reputation DESC"#,
        says: None,
        needs: W,
        open: "e/coaches/{id}",
    },
    Panel {
        id: "S8.fbs_changes",
        page: "coaches",
        area: "S8",
        title: r#"New FBS head coaches a season against real"#,
        kind: "line",
        note: r#"Each season: FBS programs whose head coach (the season's coach) differs from last season's, against real 2005-2025 (`college_people.json` changes_by_season)."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1), x AS (SELECT year, pid, coach, LAG(coach) OVER (PARTITION BY pid ORDER BY year) AS before, division FROM ps)
         SELECT x.year, SUM(x.coach IS NOT x.before AND x.before IS NOT NULL) AS world,
         json_extract((SELECT json FROM real_data WHERE file = 'college_people.json'), '$.fbs_head_coaches.changes_by_season."' || x.year || '".new_head_coach') AS real
         FROM x WHERE x.division = 'FBS' AND x.year > (SELECT MIN(year) FROM ps) GROUP BY x.year ORDER BY x.year"#,
        says: Some(
            r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1), x AS (SELECT year, pid, coach, LAG(coach) OVER (PARTITION BY pid ORDER BY year) AS before, division FROM ps)
         SELECT printf('%.1f new FBS head coaches a season (%.1f%% of programs); real 2005-2025: 20.3 (17.6%%).', 1.0 * SUM(coach IS NOT before) / COUNT(DISTINCT year), 100.0 * AVG(coach IS NOT before))
         FROM x WHERE division = 'FBS' AND before IS NOT NULL"#,
        ),
        needs: W,
        open: "",
    },
    Panel {
        id: "S8.nfl_changes",
        page: "coaches",
        area: "S8",
        title: r#"NFL head coach changes a season against real"#,
        kind: "line",
        note: r#"Each season: NFL clubs whose head coach differs from the season before (`nfl_seasons`), against real offseason and in-season changes (`nfl_history.json`, 2000-2025)."#,
        sql: r#"WITH x AS (SELECT year, team, coach, LAG(coach) OVER (PARTITION BY team ORDER BY year) AS before FROM nfl_seasons)
         SELECT year, SUM(coach IS NOT before AND before IS NOT NULL) AS world,
         json_extract((SELECT json FROM real_data WHERE file = 'nfl_history.json'), '$.head_coaches.offseason_changes_by_season."' || year || '"') AS real
         FROM x WHERE year > (SELECT MIN(year) FROM nfl_seasons) GROUP BY year ORDER BY year"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S8.sources",
        page: "coaches",
        area: "S8",
        title: r#"Where new FBS head coaches came from"#,
        kind: "bars",
        note: r#"New FBS head coaches' previous job, by role and level, over the world's seasons."#,
        sql: r#"WITH st AS MATERIALIZED (SELECT h._parent AS c, h._i AS i, CASE WHEN h.role = 'HeadCoach' THEN 'Head coach' WHEN h.role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN h.role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN h.role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, h."from" AS a, h."to" AS z, h.wins AS w, h.losses AS l, h.titles AS titles, h.ended AS ended,
         CASE WHEN h.employer__College IS NOT NULL THEN COALESCE((SELECT x._2 FROM programs__history x WHERE x._parent = h.employer__College AND h."to" BETWEEN x._0 AND x._1), p.division)
         WHEN h.employer__HighSchool IS NOT NULL THEN 'HS' WHEN h.employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level,
         h.employer__College AS college, h.employer__HighSchool AS hs, h.employer__Nfl AS nfl
         FROM coaches__history h LEFT JOIN programs p ON p._row = h.employer__College), jobs AS MATERIALIZED (SELECT c, role, a, z, w, l, titles, ended, level, college, hs, nfl FROM st
         UNION ALL SELECT c._row, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END, c.job__since, (SELECT MAX(year) FROM programs__seasons), c.wins, c.losses, c.titles, 'current',
         CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS' WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END,
         c.job__employer__College, c.job__employer__HighSchool, c.job__employer__Nfl
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL), n AS (SELECT c, a, role, level, LAG(role) OVER (PARTITION BY c ORDER BY a, z) AS prev_role, LAG(level) OVER (PARTITION BY c ORDER BY a, z) AS prev_level FROM jobs)
         SELECT COALESCE(prev_level || ' ' || lower(prev_role), 'first job') AS label, COUNT(*) AS world FROM n WHERE role = 'Head coach' AND level = 'FBS' AND a > (SELECT MIN(year) FROM programs__seasons)
         GROUP BY 1 ORDER BY world DESC LIMIT 20"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S8.news",
        page: "coaches",
        area: "S8",
        title: r#"Coaching news a year"#,
        kind: "line",
        note: r#"Each year's hires, firings, departures and extensions in the news."#,
        sql: r#"SELECT year, SUM(kind = 'CoachHired') AS hired, SUM(kind = 'CoachFired') AS fired, SUM(kind = 'CoachLeft') AS left, SUM(kind = 'CoachExtended') AS extended FROM news WHERE kind LIKE 'Coach%' GROUP BY year ORDER BY year"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S8.cycle",
        page: "coaches",
        area: "S8",
        title: r#"The last cycle's moves"#,
        kind: "table",
        note: r#"Every head job that began in the world's last season or the one now under way, with the coach's job before it and the news line that says why."#,
        sql: r#"WITH st AS MATERIALIZED (SELECT h._parent AS c, h._i AS i, CASE WHEN h.role = 'HeadCoach' THEN 'Head coach' WHEN h.role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN h.role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN h.role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, h."from" AS a, h."to" AS z, h.wins AS w, h.losses AS l, h.titles AS titles, h.ended AS ended,
         CASE WHEN h.employer__College IS NOT NULL THEN COALESCE((SELECT x._2 FROM programs__history x WHERE x._parent = h.employer__College AND h."to" BETWEEN x._0 AND x._1), p.division)
         WHEN h.employer__HighSchool IS NOT NULL THEN 'HS' WHEN h.employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level,
         h.employer__College AS college, h.employer__HighSchool AS hs, h.employer__Nfl AS nfl
         FROM coaches__history h LEFT JOIN programs p ON p._row = h.employer__College), jobs AS MATERIALIZED (SELECT c, role, a, z, w, l, titles, ended, level, college, hs, nfl FROM st
         UNION ALL SELECT c._row, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END, c.job__since, (SELECT MAX(year) FROM programs__seasons), c.wins, c.losses, c.titles, 'current',
         CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS' WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END,
         c.job__employer__College, c.job__employer__HighSchool, c.job__employer__Nfl
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL), n AS (SELECT c, a, role, level, college, hs, nfl, LAG(role) OVER (PARTITION BY c ORDER BY a, z) AS prev_role, LAG(level) OVER (PARTITION BY c ORDER BY a, z) AS prev_level,
         LAG(college) OVER (PARTITION BY c ORDER BY a, z) AS prev_college FROM jobs)
         SELECT n.a AS year, n.c AS id, co.name, n.level, COALESCE((SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = n.college), (SELECT name FROM nfl WHERE _row = n.nfl), 'high school') AS job,
         COALESCE(n.prev_level || ' ' || lower(n.prev_role), '') || COALESCE(' at ' || (SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = n.prev_college), '') AS before
         FROM n JOIN coaches co ON co._row = n.c WHERE n.role = 'Head coach' AND n.level <> 'HS' AND n.a >= (SELECT MAX(year) FROM programs__seasons) ORDER BY CASE n.level WHEN 'NFL' THEN 0 WHEN 'FBS' THEN 1 WHEN 'FCS' THEN 2 WHEN 'D2' THEN 3 WHEN 'D3' THEN 4 WHEN 'NAIA' THEN 5 WHEN 'JUCO' THEN 6 WHEN 'HS' THEN 7 ELSE 8 END, n.a DESC"#,
        says: None,
        needs: W,
        open: "e/coaches/{id}",
    },
    Panel {
        id: "S8.fcs_up",
        page: "coaches",
        area: "S8",
        title: r#"FCS title coaches who moved up"#,
        kind: "table",
        note: r#"FCS head coaches who won a title and later took an FBS head job (the Cignetti path): they exist but are few."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1), st AS MATERIALIZED (SELECT h._parent AS c, h._i AS i, CASE WHEN h.role = 'HeadCoach' THEN 'Head coach' WHEN h.role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN h.role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN h.role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, h."from" AS a, h."to" AS z, h.wins AS w, h.losses AS l, h.titles AS titles, h.ended AS ended,
         CASE WHEN h.employer__College IS NOT NULL THEN COALESCE((SELECT x._2 FROM programs__history x WHERE x._parent = h.employer__College AND h."to" BETWEEN x._0 AND x._1), p.division)
         WHEN h.employer__HighSchool IS NOT NULL THEN 'HS' WHEN h.employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level,
         h.employer__College AS college, h.employer__HighSchool AS hs, h.employer__Nfl AS nfl
         FROM coaches__history h LEFT JOIN programs p ON p._row = h.employer__College), jobs AS MATERIALIZED (SELECT c, role, a, z, w, l, titles, ended, level, college, hs, nfl FROM st
         UNION ALL SELECT c._row, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END, c.job__since, (SELECT MAX(year) FROM programs__seasons), c.wins, c.losses, c.titles, 'current',
         CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS' WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END,
         c.job__employer__College, c.job__employer__HighSchool, c.job__employer__Nfl
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL) SELECT DISTINCT ps.coach AS id, c.name, ps.name AS fcs_program, ps.year AS title_year, j.a AS fbs_from, (SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = j.college) AS fbs_program
         FROM ps JOIN jobs j ON j.c = ps.coach AND j.role = 'Head coach' AND j.level = 'FBS' AND j.a > ps.year JOIN coaches c ON c._row = ps.coach WHERE ps.division = 'FCS' AND ps.nt = 1 ORDER BY ps.year"#,
        says: None,
        needs: W,
        open: "e/coaches/{id}",
    },
    Panel {
        id: "S9.searches",
        page: "coaches",
        area: "S9",
        title: r#"Every search: who was called and who said yes"#,
        kind: "table",
        note: r#"The world log's searches (`Search` whys), newest first: the job, its prestige, how many candidates were called, the hire and the why."#,
        sql: r#"SELECT w.season AS year, COALESCE((SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = w.why__Search__employer__College), (SELECT name FROM nfl WHERE _row = w.why__Search__employer__Nfl),
         (SELECT name FROM high_schools WHERE _row = w.why__Search__employer__HighSchool), 'other league') AS employer,
         COALESCE(w.why__Search__role, 'position coach') AS role, ROUND(w.why__Search__prestige, 2) AS prestige, (SELECT COUNT(*) FROM log_whys__why__Search__called k WHERE k._parent = +w._row) AS called,
         w.why__Search__hired AS id, (SELECT name FROM coaches WHERE _row = w.why__Search__hired) AS hired, w.why__Search__why AS why
         FROM log_whys w WHERE w.why__Search__why IS NOT NULL AND (w.why__Search__employer__College IS NOT NULL OR w.why__Search__employer__Nfl IS NOT NULL) ORDER BY w.season DESC, w.why__Search__prestige DESC"#,
        says: None,
        needs: WL,
        open: "e/coaches/{id}",
    },
    Panel {
        id: "S9.own_tree",
        page: "coaches",
        area: "S9",
        title: r#"Coordinators hired from the head coach's own tree"#,
        kind: "line",
        note: r#"Each season: new FBS coordinators whose mentor is the program's head coach that season (a tree bonus of 1.5 in the score), as a share."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1), st AS MATERIALIZED (SELECT h._parent AS c, h._i AS i, CASE WHEN h.role = 'HeadCoach' THEN 'Head coach' WHEN h.role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN h.role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN h.role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, h."from" AS a, h."to" AS z, h.wins AS w, h.losses AS l, h.titles AS titles, h.ended AS ended,
         CASE WHEN h.employer__College IS NOT NULL THEN COALESCE((SELECT x._2 FROM programs__history x WHERE x._parent = h.employer__College AND h."to" BETWEEN x._0 AND x._1), p.division)
         WHEN h.employer__HighSchool IS NOT NULL THEN 'HS' WHEN h.employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level,
         h.employer__College AS college, h.employer__HighSchool AS hs, h.employer__Nfl AS nfl
         FROM coaches__history h LEFT JOIN programs p ON p._row = h.employer__College), jobs AS MATERIALIZED (SELECT c, role, a, z, w, l, titles, ended, level, college, hs, nfl FROM st
         UNION ALL SELECT c._row, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END, c.job__since, (SELECT MAX(year) FROM programs__seasons), c.wins, c.losses, c.titles, 'current',
         CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS' WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END,
         c.job__employer__College, c.job__employer__HighSchool, c.job__employer__Nfl
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL) SELECT j.a AS year, ROUND(100.0 * AVG(co.mentor = ps.coach), 1) AS world, COUNT(*) AS n
         FROM jobs j JOIN coaches co ON co._row = j.c JOIN ps ON ps.prog = j.college AND ps.year = j.a WHERE j.role IN ('Offensive coordinator', 'Defensive coordinator') AND ps.division = 'FBS' GROUP BY j.a ORDER BY j.a"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S9.pipeline",
        page: "coaches",
        area: "S9",
        title: r#"The pipeline: coaches of color among FBS coordinators and head coaches"#,
        kind: "line",
        note: r#"Each season: the share of non-White coaches among FBS coordinators (their jobs covering the season) and among FBS head coaches (the season's coach). The gap is the hiring pipeline."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1), st AS MATERIALIZED (SELECT h._parent AS c, h._i AS i, CASE WHEN h.role = 'HeadCoach' THEN 'Head coach' WHEN h.role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN h.role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN h.role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, h."from" AS a, h."to" AS z, h.wins AS w, h.losses AS l, h.titles AS titles, h.ended AS ended,
         CASE WHEN h.employer__College IS NOT NULL THEN COALESCE((SELECT x._2 FROM programs__history x WHERE x._parent = h.employer__College AND h."to" BETWEEN x._0 AND x._1), p.division)
         WHEN h.employer__HighSchool IS NOT NULL THEN 'HS' WHEN h.employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level,
         h.employer__College AS college, h.employer__HighSchool AS hs, h.employer__Nfl AS nfl
         FROM coaches__history h LEFT JOIN programs p ON p._row = h.employer__College), jobs AS MATERIALIZED (SELECT c, role, a, z, w, l, titles, ended, level, college, hs, nfl FROM st
         UNION ALL SELECT c._row, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END, c.job__since, (SELECT MAX(year) FROM programs__seasons), c.wins, c.losses, c.titles, 'current',
         CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS' WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END,
         c.job__employer__College, c.job__employer__HighSchool, c.job__employer__Nfl
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL), co AS (SELECT ps.year, AVG(c.ethnicity <> 'White') AS v FROM jobs j JOIN ps ON ps.prog = j.college AND ps.year BETWEEN j.a AND j.z JOIN coaches c ON c._row = j.c
         WHERE j.role IN ('Offensive coordinator', 'Defensive coordinator') AND ps.division = 'FBS' GROUP BY ps.year),
         hc AS (SELECT ps.year, AVG(c.ethnicity <> 'White') AS v FROM ps JOIN coaches c ON c._row = ps.coach WHERE ps.division = 'FBS' GROUP BY ps.year)
         SELECT hc.year, ROUND(100.0 * co.v, 1) AS coordinators, ROUND(100.0 * hc.v, 1) AS head_coaches FROM hc LEFT JOIN co ON co.year = hc.year ORDER BY hc.year"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S10.black_heads",
        page: "coaches",
        area: "S10",
        title: r#"Black FBS head coaches by year against real"#,
        kind: "line",
        note: r#"Each season's FBS head coaches (the season's coach) who are Black, as a share, against the real counts: 7 in 2009, 14 of 133 in 2023, 16 of 134 in 2024 (CBS Sports; `trial3-baselines.md`)."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1) SELECT ps.year, ROUND(100.0 * AVG(c.ethnicity = 'Black'), 1) AS world, SUM(c.ethnicity = 'Black') AS black, COUNT(*) AS n,
         CASE ps.year WHEN 2009 THEN 5.9 WHEN 2023 THEN 10.5 WHEN 2024 THEN 11.9 END AS real
         FROM ps JOIN coaches c ON c._row = ps.coach WHERE ps.division = 'FBS' GROUP BY ps.year ORDER BY ps.year"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S10.nfl",
        page: "coaches",
        area: "S10",
        title: r#"NFL head coaches by ethnicity by year"#,
        kind: "line",
        note: r#"Each season's NFL head coaches (`nfl_seasons` coach) who are not White, as a count."#,
        sql: r#"SELECT s.year, SUM(c.ethnicity <> 'White') AS of_color, SUM(c.ethnicity = 'Black') AS black, COUNT(*) AS n FROM nfl_seasons s JOIN coaches c ON c._row = s.coach GROUP BY s.year ORDER BY s.year"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S10.reports",
        page: "coaches",
        area: "S10",
        title: r#"The yearly diversity reports"#,
        kind: "table",
        note: r#"The news the world writes each year about head coaches of color."#,
        sql: r#"SELECT year, text FROM news WHERE kind = 'DiversityReport' ORDER BY year DESC"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S11.trees",
        page: "coaches",
        area: "S11",
        title: r#"The biggest coaching trees"#,
        kind: "table",
        note: r#"Coaches by the size of their tree: direct protégés (his mentor link), protégés who held a head job, protégés who are college head coaches now and grand-protégés. Real big trees (Saban's, Walsh's) run to a dozen or more head coaches (from memory)."#,
        sql: r#"WITH k AS (SELECT mentor, COUNT(*) AS kids, SUM(job__role = 'HeadCoach' AND job__employer__College IS NOT NULL) AS college_heads_now FROM coaches WHERE mentor IS NOT NULL GROUP BY mentor),
         h AS (SELECT c.mentor, COUNT(DISTINCT c._row) AS heads FROM coaches c JOIN coaches__history s ON s._parent = c._row WHERE c.mentor IS NOT NULL AND s.role = 'HeadCoach' AND s.employer__HighSchool IS NULL GROUP BY c.mentor),
         g AS (SELECT p.mentor, COUNT(*) AS grand FROM coaches k2 JOIN coaches p ON p._row = k2.mentor WHERE p.mentor IS NOT NULL GROUP BY p.mentor)
         SELECT c._row AS id, c.name, k.kids AS proteges, COALESCE(h.heads, 0) AS proteges_with_head_jobs, k.college_heads_now, COALESCE(g.grand, 0) AS grand_proteges, c.career_titles AS titles,
         CASE WHEN c.retired = 1 THEN 'retired' ELSE COALESCE(CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END, 'out of work') END AS now
         FROM k JOIN coaches c ON c._row = k.mentor LEFT JOIN h ON h.mentor = k.mentor LEFT JOIN g ON g.mentor = k.mentor ORDER BY proteges_with_head_jobs DESC, k.kids DESC LIMIT 60"#,
        says: None,
        needs: W,
        open: "e/coaches/{id}",
    },
    Panel {
        id: "S11.title_mentors",
        page: "coaches",
        area: "S11",
        title: r#"New head coaches with a title-winning mentor"#,
        kind: "line",
        note: r#"Each season: new college head coaches whose mentor won a national title as a head coach, as a share."#,
        sql: r#"WITH st AS MATERIALIZED (SELECT h._parent AS c, h._i AS i, CASE WHEN h.role = 'HeadCoach' THEN 'Head coach' WHEN h.role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN h.role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN h.role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, h."from" AS a, h."to" AS z, h.wins AS w, h.losses AS l, h.titles AS titles, h.ended AS ended,
         CASE WHEN h.employer__College IS NOT NULL THEN COALESCE((SELECT x._2 FROM programs__history x WHERE x._parent = h.employer__College AND h."to" BETWEEN x._0 AND x._1), p.division)
         WHEN h.employer__HighSchool IS NOT NULL THEN 'HS' WHEN h.employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level,
         h.employer__College AS college, h.employer__HighSchool AS hs, h.employer__Nfl AS nfl
         FROM coaches__history h LEFT JOIN programs p ON p._row = h.employer__College), jobs AS MATERIALIZED (SELECT c, role, a, z, w, l, titles, ended, level, college, hs, nfl FROM st
         UNION ALL SELECT c._row, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END, c.job__since, (SELECT MAX(year) FROM programs__seasons), c.wins, c.losses, c.titles, 'current',
         CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS' WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END,
         c.job__employer__College, c.job__employer__HighSchool, c.job__employer__Nfl
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL) SELECT j.a AS year, ROUND(100.0 * AVG(COALESCE(m.career_titles, 0) > 0), 1) AS world, COUNT(*) AS n FROM jobs j JOIN coaches c ON c._row = j.c LEFT JOIN coaches m ON m._row = c.mentor
         WHERE j.role = 'Head coach' AND j.college IS NOT NULL AND j.a > (SELECT MIN(year) FROM programs__seasons) GROUP BY j.a ORDER BY j.a"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S12.reads",
        page: "coaches",
        area: "S12",
        title: r#"Unit records by group"#,
        kind: "table",
        note: r#"Coaches with 8+ seasons of credit in a group: the spread of their public growth read (growth / (seasons + 8)) and how many moved up, and (debug) how closely the read follows true teaching."#,
        sql: r#"WITH grp(k, grp) AS (VALUES (0, 'QB'), (1, 'RB'), (2, 'Receivers'), (3, 'OL'), (4, 'DL'), (5, 'LB'), (6, 'DB'), (7, 'Specialists')), x AS (SELECT u._i AS k, u.growth / (u.seasons + 8) AS v, u.moved_up, json_extract(c.ability__teaching, '$[' || u._i || ']') AS t FROM coaches__units u JOIN coaches c ON c._row = u._parent WHERE u.seasons >= 8),
         r AS (SELECT k, v, moved_up, t, ROW_NUMBER() OVER (PARTITION BY k ORDER BY v) AS r, COUNT(*) OVER (PARTITION BY k) AS n FROM x),
         s AS (SELECT k, AVG(v) AS mv, AVG(t) AS mt, AVG(v * v) - AVG(v) * AVG(v) AS vv, AVG(t * t) - AVG(t) * AVG(t) AS vt, AVG(v * t) - AVG(v) * AVG(t) AS cv FROM x GROUP BY k)
         SELECT grp.grp AS grp_taught, MAX(r.n) AS coaches, ROUND(MAX(CASE WHEN r = CAST(0.1 * (n - 1) AS INTEGER) + 1 THEN v END), 3) AS p10, ROUND(MAX(CASE WHEN r = CAST(0.5 * (n - 1) AS INTEGER) + 1 THEN v END), 3) AS median, ROUND(MAX(CASE WHEN r = CAST(0.9 * (n - 1) AS INTEGER) + 1 THEN v END), 3) AS p90, ROUND(AVG(r.moved_up), 2) AS moved_up,
         ROUND(s.cv / sqrt(s.vv * s.vt), 2) AS truth_corr_debug
         FROM r JOIN s ON s.k = r.k JOIN grp ON grp.k = r.k GROUP BY r.k ORDER BY r.k"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S12.read_vs_truth",
        page: "coaches",
        area: "S12",
        title: r#"The public record against true teaching (debug)"#,
        kind: "bars",
        note: r#"Coaches with 8+ seasons in a group, by their true teaching of it in bands: the mean growth read (pick a group above). It should rise with the truth, noisily."#,
        sql: r#"WITH grp(k, grp) AS (VALUES (0, 'QB'), (1, 'RB'), (2, 'Receivers'), (3, 'OL'), (4, 'DL'), (5, 'LB'), (6, 'DB'), (7, 'Specialists')) SELECT grp.grp AS grp, CASE WHEN t < 8 THEN 'Teaching under 8' WHEN t < 11 THEN '8-11' WHEN t < 14 THEN '11-14' WHEN t < 17 THEN '14-17' ELSE '17+' END AS label, ROUND(AVG(v), 3) AS world, COUNT(*) AS n
         FROM (SELECT u._i AS k, u.growth / (u.seasons + 8) AS v, json_extract(c.ability__teaching, '$[' || u._i || ']') AS t FROM coaches__units u JOIN coaches c ON c._row = u._parent WHERE u.seasons >= 8) x
         JOIN grp ON grp.k = x.k GROUP BY x.k, 2 ORDER BY x.k, MIN(t)"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S13.poor",
        page: "coaches",
        area: "S13",
        title: r#"Poor-coaching seasons by level"#,
        kind: "bars",
        note: r#"Player seasons whose coaching number was under 8 (a poor-coaching risk), as a share of each level's seasons."#,
        sql: r#"SELECT level AS label, ROUND(100.0 * AVG(coaching < 8), 2) AS world, ROUND(AVG(coaching), 2) AS mean_coaching, COUNT(*) AS n FROM people__player__history WHERE coaching IS NOT NULL GROUP BY level ORDER BY n DESC"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S13.nfl",
        page: "coaches",
        area: "S13",
        title: r#"NFL clubs' coaching"#,
        kind: "table",
        note: r#"Each NFL club's coaching value, staff years and the head coach, OC and DC behind it."#,
        sql: r#"SELECT t._row AS id, t.name AS club, ROUND(t.coaching, 2) AS coaching, t.staff_years, (SELECT name FROM coaches WHERE _row = t.head_coach) AS head_coach, (SELECT name FROM coaches WHERE _row = t.oc) AS oc,
         (SELECT name FROM coaches WHERE _row = t.dc) AS dc, ROUND(t.win_share, 3) AS win_share FROM nfl t ORDER BY t.coaching DESC"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S14.fbs_offenses",
        page: "coaches",
        area: "S14",
        title: r#"FBS offenses by year"#,
        kind: "line",
        note: r#"How many FBS programs ran each offense each year (`fbs_systems` in the calendar log): air raid and spread option should rise through the 2000s and 2010s."#,
        sql: r#"SELECT year, json_extract(fbs_systems, '$[0]') AS west_coast, json_extract(fbs_systems, '$[1]') AS air_raid, json_extract(fbs_systems, '$[2]') AS wide_zone, json_extract(fbs_systems, '$[3]') AS power_gap,
         json_extract(fbs_systems, '$[4]') AS spread_option, json_extract(fbs_systems, '$[5]') AS four_three, json_extract(fbs_systems, '$[6]') AS three_four FROM log_history WHERE fbs_systems IS NOT NULL ORDER BY year"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S14.installed",
        page: "coaches",
        area: "S14",
        title: r#"What teams run now, by level"#,
        kind: "matrix",
        note: r#"Programs and NFL clubs now by the offense, front and coverage family installed, as shares of the level."#,
        sql: r#"WITH y AS (SELECT division AS level, installed__off AS o, installed__front AS f, installed__shell AS s FROM programs WHERE installed__off IS NOT NULL
         UNION ALL SELECT 'NFL', installed__off, installed__front, installed__shell FROM clubs WHERE installed__off IS NOT NULL),
         x AS (SELECT level, CASE o WHEN 0 THEN 'West Coast' WHEN 1 THEN 'Air raid' WHEN 2 THEN 'Wide zone' WHEN 3 THEN 'Power gap' ELSE 'Spread option' END AS off,
         CASE f WHEN 0 THEN '4-3 one-gap' ELSE '3-4 two-gap' END AS front, CASE s WHEN 0 THEN 'Cover 3' WHEN 1 THEN 'Quarters' WHEN 2 THEN 'Man pressure' ELSE 'Cover 2' END AS shell FROM y)
         SELECT level AS row, 'Offense: ' || off AS col, ROUND(100.0 * COUNT(*) / SUM(COUNT(*)) OVER (PARTITION BY level), 1) AS value FROM x GROUP BY level, off
         UNION ALL SELECT level, 'Front: ' || front, ROUND(100.0 * COUNT(*) / SUM(COUNT(*)) OVER (PARTITION BY level), 1) FROM x GROUP BY level, front
         UNION ALL SELECT level, 'Coverage: ' || shell, ROUND(100.0 * COUNT(*) / SUM(COUNT(*)) OVER (PARTITION BY level), 1) FROM x GROUP BY level, shell"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S14.switches",
        page: "coaches",
        area: "S14",
        title: r#"System switches a year"#,
        kind: "line",
        note: r#"Teams that switched an installed system each year (the calendar log)."#,
        sql: r#"SELECT year, system_switches AS switches FROM log_history WHERE system_switches IS NOT NULL ORDER BY year"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S14.knowledge",
        page: "coaches",
        area: "S14",
        title: r#"How well coaches know each system (debug)"#,
        kind: "matrix",
        note: r#"Coaches in a job now: mean knowledge (0-1) of each of the 11 systems by level."#,
        sql: r#"WITH cj AS MATERIALIZED (SELECT c.*, CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS'
         WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, p.prestige AS prestige
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL), s(k, name) AS (VALUES (0, 'West Coast'), (1, 'Air raid'), (2, 'Wide zone'), (3, 'Power gap'), (4, 'Spread option'), (5, '4-3 one-gap'), (6, '3-4 two-gap'), (7, 'Cover 3'), (8, 'Quarters'), (9, 'Man pressure'), (10, 'Cover 2'))
         SELECT cj.level AS row, s.name AS col, ROUND(AVG(j.value), 2) AS value FROM cj, json_each(cj.systems__know) j JOIN s ON s.k = j.key GROUP BY cj.level, s.k ORDER BY CASE cj.level WHEN 'NFL' THEN 0 WHEN 'FBS' THEN 1 WHEN 'FCS' THEN 2 WHEN 'D2' THEN 3 WHEN 'D3' THEN 4 WHEN 'NAIA' THEN 5 WHEN 'JUCO' THEN 6 WHEN 'HS' THEN 7 ELSE 8 END, s.k"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S14.norms",
        page: "coaches",
        area: "S14",
        title: r#"Each position's typical score in each system (debug)"#,
        kind: "matrix",
        note: r#"The world's system norms (`system_norms`): rows are positions, columns the 11 systems. An air raid should want receivers, power gap linemen."#,
        sql: r#"WITH s(k, name) AS (VALUES (0, 'West Coast'), (1, 'Air raid'), (2, 'Wide zone'), (3, 'Power gap'), (4, 'Spread option'), (5, '4-3 one-gap'), (6, '3-4 two-gap'), (7, 'Cover 3'), (8, 'Quarters'), (9, 'Man pressure'), (10, 'Cover 2')),
         p(k, pos) AS (VALUES (0, 'QB'), (1, 'RB'), (2, 'FB'), (3, 'WR'), (4, 'TE'), (5, 'OT'), (6, 'IOL'), (7, 'EDGE'), (8, 'DT'), (9, 'LB'), (10, 'CB'), (11, 'S'), (12, 'K'), (13, 'P'))
         SELECT p.pos AS row, s.name AS col, ROUND(CASE s.k WHEN 0 THEN n._0 WHEN 1 THEN n._1 WHEN 2 THEN n._2 WHEN 3 THEN n._3 WHEN 4 THEN n._4 WHEN 5 THEN n._5 WHEN 6 THEN n._6 WHEN 7 THEN n._7 WHEN 8 THEN n._8 WHEN 9 THEN n._9 ELSE n._10 END, 1) AS value
         FROM system_norms n JOIN p ON p.k = n._row, s ORDER BY n._row, s.k"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S16.callers",
        page: "coaches",
        area: "S16",
        title: r#"Who calls the plays, and with what (debug)"#,
        kind: "table",
        note: r#"Each level now: how often the head coach calls a side because the seat is empty, and the coordinators' scheme, game management and ambition (the caller's skill, reading, creativity, nerve and aggressiveness are worked out from these and his systems each game, `coach_caller`; not stored)."#,
        sql: r#"WITH cj AS MATERIALIZED (SELECT c.*, CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS'
         WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, p.prestige AS prestige
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL) SELECT cj.level, cj.role, COUNT(*) AS coaches, ROUND(AVG(ability__scheme), 1) AS scheme, ROUND(AVG(ability__game_management), 1) AS game_management, ROUND(AVG(personality__ambition), 1) AS ambition,
         CASE cj.role WHEN 'Head coach' THEN ROUND(100.0 * AVG((SELECT (p.oc IS NULL) + (p.dc IS NULL) FROM programs p WHERE p._row = cj.job__employer__College)) / 2, 1) END AS calls_a_side_pct
         FROM cj WHERE cj.role <> 'Position coach' AND cj.level NOT IN ('HS', 'Other') GROUP BY cj.level, cj.role ORDER BY CASE cj.level WHEN 'NFL' THEN 0 WHEN 'FBS' THEN 1 WHEN 'FCS' THEN 2 WHEN 'D2' THEN 3 WHEN 'D3' THEN 4 WHEN 'NAIA' THEN 5 WHEN 'JUCO' THEN 6 WHEN 'HS' THEN 7 ELSE 8 END, cj.role"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S17.teams",
        page: "coaches",
        area: "S17",
        title: r#"Each FBS team's calls last season"#,
        kind: "table",
        note: r#"From the game log's team situation totals (`team_situations`): each FBS team's pass share, fourth-down go rate and blitz rate in the world's last logged season, with its coordinators."#,
        sql: r#"WITH st AS MATERIALIZED (SELECT h._parent AS c, h._i AS i, CASE WHEN h.role = 'HeadCoach' THEN 'Head coach' WHEN h.role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN h.role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN h.role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, h."from" AS a, h."to" AS z, h.wins AS w, h.losses AS l, h.titles AS titles, h.ended AS ended,
         CASE WHEN h.employer__College IS NOT NULL THEN COALESCE((SELECT x._2 FROM programs__history x WHERE x._parent = h.employer__College AND h."to" BETWEEN x._0 AND x._1), p.division)
         WHEN h.employer__HighSchool IS NOT NULL THEN 'HS' WHEN h.employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level,
         h.employer__College AS college, h.employer__HighSchool AS hs, h.employer__Nfl AS nfl
         FROM coaches__history h LEFT JOIN programs p ON p._row = h.employer__College), jobs AS MATERIALIZED (SELECT c, role, a, z, w, l, titles, ended, level, college, hs, nfl FROM st
         UNION ALL SELECT c._row, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END, c.job__since, (SELECT MAX(year) FROM programs__seasons), c.wins, c.losses, c.titles, 'current',
         CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS' WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END,
         c.job__employer__College, c.job__employer__HighSchool, c.job__employer__Nfl
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL), y AS (SELECT MAX(season) AS y FROM log_team_situations WHERE level = 'college'),
         t AS (SELECT team, SUM(CASE WHEN key = 'o|kind|pass' THEN plays END) AS pass, SUM(CASE WHEN key = 'o|kind|run' THEN plays END) AS run, SUM(CASE WHEN key = 'o|fourth|go' THEN plays END) AS go,
         SUM(CASE WHEN key IN ('o|fourth|go', 'o|fourth|punt', 'o|fourth|fg') THEN plays END) AS fourth, SUM(CASE WHEN key = 'd|blitz|true' THEN plays END) AS blitz,
         SUM(CASE WHEN key IN ('d|blitz|true', 'd|blitz|false') THEN plays END) AS dropbacks
         FROM log_team_situations WHERE level = 'college' AND season = (SELECT y FROM y) GROUP BY team)
         SELECT t.team AS id, p.real_name AS program, ROUND(100.0 * t.pass / (t.pass + t.run), 1) AS pass_pct, ROUND(100.0 * t.go / t.fourth, 1) AS fourth_go_pct, ROUND(100.0 * t.blitz / t.dropbacks, 1) AS blitz_pct,
         (SELECT co.name FROM jobs j JOIN coaches co ON co._row = j.c WHERE j.college = t.team AND j.role = 'Offensive coordinator' AND (SELECT y FROM y) BETWEEN j.a AND j.z LIMIT 1) AS oc,
         (SELECT co.name FROM jobs j JOIN coaches co ON co._row = j.c WHERE j.college = t.team AND j.role = 'Defensive coordinator' AND (SELECT y FROM y) BETWEEN j.a AND j.z LIMIT 1) AS dc
         FROM t JOIN programs p ON p._row = t.team WHERE p.division = 'FBS' ORDER BY pass_pct DESC"#,
        says: None,
        needs: GL,
        open: "e/programs/{id}",
    },
    Panel {
        id: "S17.spread",
        page: "coaches",
        area: "S17",
        title: r#"How different teams' calls are"#,
        kind: "hist",
        note: r#"The spread of FBS teams' pass share in the last logged season (pick pass share, fourth-down go rate or blitz rate above). Real FBS pass share overall is 49.9% (2023-24)."#,
        sql: r#"WITH y AS (SELECT MAX(season) AS y FROM log_team_situations WHERE level = 'college'),
         t AS (SELECT team, SUM(CASE WHEN key = 'o|kind|pass' THEN plays END) AS pass, SUM(CASE WHEN key = 'o|kind|run' THEN plays END) AS run, SUM(CASE WHEN key = 'o|fourth|go' THEN plays END) AS go,
         SUM(CASE WHEN key IN ('o|fourth|go', 'o|fourth|punt', 'o|fourth|fg') THEN plays END) AS fourth, SUM(CASE WHEN key = 'd|blitz|true' THEN plays END) AS blitz,
         SUM(CASE WHEN key IN ('d|blitz|true', 'd|blitz|false') THEN plays END) AS dropbacks
         FROM log_team_situations WHERE level = 'college' AND season = (SELECT y FROM y) GROUP BY team)
         SELECT 'Pass share' AS grp, 2 * CAST(50.0 * pass / (pass + run) AS INTEGER) AS x, COUNT(*) AS world FROM t JOIN programs p ON p._row = t.team WHERE p.division = 'FBS' GROUP BY 2
         UNION ALL SELECT 'Fourth-down go rate', 5 * CAST(20.0 * go / fourth AS INTEGER), COUNT(*) FROM t JOIN programs p ON p._row = t.team WHERE p.division = 'FBS' AND fourth > 0 GROUP BY 2
         UNION ALL SELECT 'Blitz rate', 2 * CAST(50.0 * blitz / dropbacks AS INTEGER), COUNT(*) FROM t JOIN programs p ON p._row = t.team WHERE p.division = 'FBS' AND dropbacks > 0 GROUP BY 2 ORDER BY 1, 2"#,
        says: None,
        needs: GL,
        open: "",
    },
    Panel {
        id: "S18.table",
        page: "coaches",
        area: "S18",
        title: r#"What beats what: the coaches' table"#,
        kind: "matrix",
        note: r#"Mean yards each pass concept gained against each coverage with four rushers or a linebacker blitz, and each run against each front, as measured on the engine (`call_values.csv`, cap 40, a turnover -45). Rows are concepts and runs."#,
        sql: r#"SELECT concept AS row, against AS col, yards AS value FROM engine_call_values ORDER BY _row"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S18.best_worst",
        page: "coaches",
        area: "S18",
        title: r#"Each concept's best and worst look"#,
        kind: "table",
        note: r#"For each pass concept and run: the coverage or front it does best and worst against, in yards. Classic pairings should hold (smash and corners beat Cover 2 and Cover 3, crossers and mesh beat man, flats are poor against zone)."#,
        sql: r#"WITH r AS (SELECT _row, concept, against, yards, ROW_NUMBER() OVER (PARTITION BY concept ORDER BY yards DESC) AS hi, ROW_NUMBER() OVER (PARTITION BY concept ORDER BY yards) AS lo FROM engine_call_values)
         SELECT concept, MAX(CASE WHEN hi = 1 THEN against END) AS best_against, MAX(CASE WHEN hi = 1 THEN yards END) AS best_yards, MAX(CASE WHEN lo = 1 THEN against END) AS worst_against,
         MAX(CASE WHEN lo = 1 THEN yards END) AS worst_yards FROM r GROUP BY concept ORDER BY MIN(_row)"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S19.families",
        page: "coaches",
        area: "S19",
        title: r#"Concept families by year"#,
        kind: "line",
        note: r#"Each division's share of snaps in each of the 14 concept families by season and its pass rate (`trends`); pick a division above."#,
        sql: r#"SELECT division AS grp, year, ROUND(100 * json_extract(families, '$[0]'), 1) AS inside_zone, ROUND(100 * json_extract(families, '$[1]'), 1) AS outside_zone, ROUND(100 * json_extract(families, '$[2]'), 1) AS gap,
         ROUND(100 * json_extract(families, '$[3]'), 1) AS option, ROUND(100 * json_extract(families, '$[4]'), 1) AS rpo, ROUND(100 * json_extract(families, '$[5]'), 1) AS quick, ROUND(100 * json_extract(families, '$[6]'), 1) AS dropback,
         ROUND(100 * json_extract(families, '$[7]'), 1) AS play_action, ROUND(100 * json_extract(families, '$[8]'), 1) AS screen, ROUND(100 * json_extract(families, '$[10]'), 1) AS single_high,
         ROUND(100 * json_extract(families, '$[11]'), 1) AS two_high, ROUND(100 * json_extract(families, '$[12]'), 1) AS man, ROUND(100 * json_extract(families, '$[13]'), 1) AS pressure, ROUND(100 * pass_rate, 1) AS pass_rate
         FROM trends ORDER BY division, year"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S19.novelty",
        page: "coaches",
        area: "S19",
        title: r#"How new each family is to each division now"#,
        kind: "matrix",
        note: r#"Current novelty (`novelty`): a family that rose over the last four seasons is new to the division's defenses."#,
        sql: r#"WITH f(k, name) AS (VALUES (0, 'Inside zone'), (1, 'Outside zone'), (2, 'Gap'), (3, 'Option'), (4, 'RPO'), (5, 'Quick'), (6, 'Dropback'), (7, 'Play action'), (8, 'Screen'), (9, 'Front'), (10, 'Single high'), (11, 'Two high'), (12, 'Man'), (13, 'Pressure')),
         d(k, name) AS (VALUES (0, 'FBS'), (1, 'FCS'), (2, 'D2'), (3, 'D3'), (4, 'NAIA'), (5, 'JUCO'))
         SELECT d.name AS row, f.name AS col, ROUND(CASE f.k WHEN 0 THEN n._0 WHEN 1 THEN n._1 WHEN 2 THEN n._2 WHEN 3 THEN n._3 WHEN 4 THEN n._4 WHEN 5 THEN n._5 WHEN 6 THEN n._6 WHEN 7 THEN n._7
         WHEN 8 THEN n._8 WHEN 9 THEN n._9 WHEN 10 THEN n._10 WHEN 11 THEN n._11 WHEN 12 THEN n._12 ELSE n._13 END, 3) AS value FROM novelty n JOIN d ON d.k = n._row, f ORDER BY n._row, f.k"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S20.reviews",
        page: "coaches",
        area: "S20",
        title: r#"Every head coach review"#,
        kind: "table",
        note: r#"The world log's reviews after each regular season (`Review` whys), newest and closest first: the read against the bar, the chance he was fired and what happened."#,
        sql: r#"SELECT w.season AS year, w.why__Review__coach AS id, (SELECT name FROM coaches WHERE _row = w.why__Review__coach) AS coach, (SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = w.why__Review__program) AS program,
         w.why__Review__tenure AS tenure, ROUND(w.why__Review__win_pct_last, 3) AS win_pct, ROUND(w.why__Review__read, 3) AS read, ROUND(w.why__Review__bar, 3) AS bar, ROUND(100 * w.why__Review__fire_chance, 1) AS fire_pct,
         w.why__Review__outcome AS outcome
         FROM log_whys w WHERE w.why__Review__coach IS NOT NULL ORDER BY w.season DESC, w.why__Review__fire_chance DESC"#,
        says: None,
        needs: WL,
        open: "e/coaches/{id}",
    },
    Panel {
        id: "S20.fire_by_tenure",
        page: "coaches",
        area: "S20",
        title: r#"Firing rate by season of tenure"#,
        kind: "bars",
        note: r#"FBS head coaching runs that began after the world's first season: of those reaching each season, the share that ended fired after it (year one should be near zero)."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1), runs AS MATERIALIZED (SELECT prog, name, division, coach, MIN(year) AS a, MAX(year) AS z, COUNT(*) AS n FROM (SELECT prog, name, division, coach, year,
         year - ROW_NUMBER() OVER (PARTITION BY prog, coach ORDER BY year) AS g FROM ps WHERE coach IS NOT NULL) GROUP BY prog, coach, g),
         ended AS (SELECT r.*, (SELECT h.ended FROM coaches__history h WHERE h._parent = r.coach AND h.employer__College = r.prog AND h."to" >= r.z ORDER BY h."to" LIMIT 1) AS ended FROM runs r
         WHERE r.a > (SELECT MIN(year) FROM ps) AND r.z < (SELECT MAX(year) FROM ps)), k(k) AS (VALUES (1), (2), (3), (4), (5), (6), (8), (10))
         SELECT 'Season ' || k.k AS label, ROUND(100.0 * SUM(e.n = k.k AND e.ended LIKE 'fired%') / SUM(e.n >= k.k), 1) AS world, SUM(e.n >= k.k) AS n FROM ended e, k WHERE e.division = 'FBS' GROUP BY k.k ORDER BY k.k"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S20.in_season",
        page: "coaches",
        area: "S20",
        title: r#"In-season firings a year"#,
        kind: "line",
        note: r#"Head jobs that ended 'fired during the season' each year, by level (real FBS recent years about 5-10, no source yet)."#,
        sql: r#"WITH st AS MATERIALIZED (SELECT h._parent AS c, h._i AS i, CASE WHEN h.role = 'HeadCoach' THEN 'Head coach' WHEN h.role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN h.role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN h.role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, h."from" AS a, h."to" AS z, h.wins AS w, h.losses AS l, h.titles AS titles, h.ended AS ended,
         CASE WHEN h.employer__College IS NOT NULL THEN COALESCE((SELECT x._2 FROM programs__history x WHERE x._parent = h.employer__College AND h."to" BETWEEN x._0 AND x._1), p.division)
         WHEN h.employer__HighSchool IS NOT NULL THEN 'HS' WHEN h.employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level,
         h.employer__College AS college, h.employer__HighSchool AS hs, h.employer__Nfl AS nfl
         FROM coaches__history h LEFT JOIN programs p ON p._row = h.employer__College) SELECT z AS year, SUM(level = 'FBS') AS fbs, SUM(level = 'FCS') AS fcs, SUM(level NOT IN ('FBS', 'FCS')) AS other FROM st WHERE ended = 'fired during the season' GROUP BY z ORDER BY z"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S20.hot_seat",
        page: "coaches",
        area: "S20",
        title: r#"Prove-it years and the hot seat now"#,
        kind: "table",
        note: r#"College head coaches now on a prove-it year or with a losing last two seasons, with the AD's patience and the buyout factor's base (contract years left)."#,
        sql: r#"WITH cj AS MATERIALIZED (SELECT c.*, CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS'
         WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, p.prestige AS prestige
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL) SELECT cj._row AS id, cj.name, (SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = cj.job__employer__College) AS program, cj.level, (SELECT MAX(year) FROM programs__seasons) - cj.job__since + 1 AS season_in_job,
         cj.prove_it, cj.recent AS last_six_vs_expectation, cj.contract__through AS contract_through, ROUND(p.ad__patience, 1) AS ad_patience
         FROM cj JOIN programs p ON p._row = cj.job__employer__College WHERE cj.role = 'Head coach' AND (cj.prove_it IS NOT NULL OR (SELECT AVG(value) FROM (SELECT value FROM json_each(cj.recent) ORDER BY key DESC LIMIT 2)) < 0)
         ORDER BY CASE cj.level WHEN 'NFL' THEN 0 WHEN 'FBS' THEN 1 WHEN 'FCS' THEN 2 WHEN 'D2' THEN 3 WHEN 'D3' THEN 4 WHEN 'NAIA' THEN 5 WHEN 'JUCO' THEN 6 WHEN 'HS' THEN 7 ELSE 8 END, cj.prove_it IS NULL"#,
        says: None,
        needs: W,
        open: "e/coaches/{id}",
    },
    Panel {
        id: "S21.salary",
        page: "coaches",
        area: "S21",
        title: r#"College head coach pay by prestige"#,
        kind: "bars",
        note: r#"College head coaches' salary now (`contract`) by their program's prestige in fifths at each level (pick above), in $ millions."#,
        sql: r#"WITH cj AS MATERIALIZED (SELECT c.*, CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS'
         WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, p.prestige AS prestige
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL), x AS (SELECT level, contract__salary AS s, NTILE(5) OVER (PARTITION BY level ORDER BY prestige) AS q, prestige FROM cj WHERE role = 'Head coach' AND contract__salary > 0 AND level NOT IN ('HS', 'NFL', 'Other'))
         SELECT level AS grp, 'Prestige fifth ' || q AS label, ROUND(AVG(s) / 1e6, 2) AS world, ROUND(MAX(s) / 1e6, 2) AS top, COUNT(*) AS n FROM x GROUP BY level, q ORDER BY CASE level WHEN 'NFL' THEN 0 WHEN 'FBS' THEN 1 WHEN 'FCS' THEN 2 WHEN 'D2' THEN 3 WHEN 'D3' THEN 4 WHEN 'NAIA' THEN 5 WHEN 'JUCO' THEN 6 WHEN 'HS' THEN 7 ELSE 8 END, q"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S21.contracts",
        page: "coaches",
        area: "S21",
        title: r#"Contracts by level"#,
        kind: "table",
        note: r#"College head coaches now: contract years left, salary and buyout share, by level."#,
        sql: r#"WITH cj AS MATERIALIZED (SELECT c.*, CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS'
         WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, p.prestige AS prestige
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL) SELECT level, COUNT(*) AS coaches, ROUND(AVG(contract__through - (SELECT MAX(year) FROM programs__seasons)), 1) AS years_left, ROUND(AVG(contract__salary) / 1e6, 2) AS salary_m, ROUND(MAX(contract__salary) / 1e6, 2) AS top_m,
         ROUND(AVG(contract__buyout_share), 2) AS buyout_share FROM cj WHERE role = 'Head coach' AND contract__through IS NOT NULL GROUP BY level ORDER BY CASE level WHEN 'NFL' THEN 0 WHEN 'FBS' THEN 1 WHEN 'FCS' THEN 2 WHEN 'D2' THEN 3 WHEN 'D3' THEN 4 WHEN 'NAIA' THEN 5 WHEN 'JUCO' THEN 6 WHEN 'HS' THEN 7 ELSE 8 END"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S22.clubs",
        page: "coaches",
        area: "S22",
        title: r#"NFL staffs and front offices now"#,
        kind: "table",
        note: r#"Each club's head coach, coordinators, coaching value, the GM and who has the final say."#,
        sql: r#"SELECT t._row AS id, t.name AS club, (SELECT name FROM coaches WHERE _row = t.head_coach) AS head_coach, (SELECT name FROM coaches WHERE _row = t.oc) AS oc, (SELECT name FROM coaches WHERE _row = t.dc) AS dc,
         ROUND(t.coaching, 2) AS coaching, t.staff_years, k.office__gm__name AS gm, k.office__power AS final_say, ROUND(t.win_share, 3) AS win_share FROM nfl t LEFT JOIN clubs k ON k._row = t._row ORDER BY t.name"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S22.tenure",
        page: "coaches",
        area: "S22",
        title: r#"How long NFL head coaches last"#,
        kind: "bars",
        note: r#"Finished NFL head coaching runs (`nfl_seasons`) that began after the world's first NFL season: the share lasting at least 2, 3, 4, 5, 8 and 10 seasons, against real 2000-2019 (`nfl_history.json`)."#,
        sql: r#"WITH nr AS MATERIALIZED (SELECT team, coach, MIN(year) AS a, MAX(year) AS z, COUNT(*) AS n FROM (SELECT team, coach, year, year - ROW_NUMBER() OVER (PARTITION BY team, coach ORDER BY year) AS g FROM nfl_seasons) GROUP BY team, coach, g),
         nf AS (SELECT * FROM nr WHERE a > (SELECT MIN(year) FROM nfl_seasons) AND z < (SELECT MAX(year) FROM nfl_seasons)), k(k) AS (VALUES (2), (3), (4), (5), (8), (10))
         SELECT 'At least ' || k.k || ' seasons' AS label, ROUND(100.0 * AVG(nf.n >= k.k), 1) AS world,
         100.0 * json_extract((SELECT json FROM real_data WHERE file = 'nfl_history.json'), '$.head_coaches.share_lasting_at_least."' || k.k || '"') AS real, COUNT(*) AS n FROM nf, k GROUP BY k.k ORDER BY k.k"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S22.firings",
        page: "coaches",
        area: "S22",
        title: r#"NFL head coaches fired a year and their tenure"#,
        kind: "line",
        note: r#"NFL head jobs that ended fired each year, with the mean tenure of those fired."#,
        sql: r#"WITH st AS MATERIALIZED (SELECT h._parent AS c, h._i AS i, CASE WHEN h.role = 'HeadCoach' THEN 'Head coach' WHEN h.role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN h.role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN h.role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, h."from" AS a, h."to" AS z, h.wins AS w, h.losses AS l, h.titles AS titles, h.ended AS ended,
         CASE WHEN h.employer__College IS NOT NULL THEN COALESCE((SELECT x._2 FROM programs__history x WHERE x._parent = h.employer__College AND h."to" BETWEEN x._0 AND x._1), p.division)
         WHEN h.employer__HighSchool IS NOT NULL THEN 'HS' WHEN h.employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level,
         h.employer__College AS college, h.employer__HighSchool AS hs, h.employer__Nfl AS nfl
         FROM coaches__history h LEFT JOIN programs p ON p._row = h.employer__College) SELECT z AS year, SUM(ended LIKE 'fired%') AS fired, ROUND(AVG(CASE WHEN ended LIKE 'fired%' THEN z - a + 1 END), 1) AS tenure FROM st WHERE role = 'Head coach' AND level = 'NFL' GROUP BY z ORDER BY z"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S22.college_to_nfl",
        page: "coaches",
        area: "S22",
        title: r#"College head coaches hired by the NFL"#,
        kind: "table",
        note: r#"NFL head jobs taken by a coach whose last job was a college head job: rare, and mostly title coaches."#,
        sql: r#"WITH st AS MATERIALIZED (SELECT h._parent AS c, h._i AS i, CASE WHEN h.role = 'HeadCoach' THEN 'Head coach' WHEN h.role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN h.role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN h.role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, h."from" AS a, h."to" AS z, h.wins AS w, h.losses AS l, h.titles AS titles, h.ended AS ended,
         CASE WHEN h.employer__College IS NOT NULL THEN COALESCE((SELECT x._2 FROM programs__history x WHERE x._parent = h.employer__College AND h."to" BETWEEN x._0 AND x._1), p.division)
         WHEN h.employer__HighSchool IS NOT NULL THEN 'HS' WHEN h.employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level,
         h.employer__College AS college, h.employer__HighSchool AS hs, h.employer__Nfl AS nfl
         FROM coaches__history h LEFT JOIN programs p ON p._row = h.employer__College), jobs AS MATERIALIZED (SELECT c, role, a, z, w, l, titles, ended, level, college, hs, nfl FROM st
         UNION ALL SELECT c._row, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END, c.job__since, (SELECT MAX(year) FROM programs__seasons), c.wins, c.losses, c.titles, 'current',
         CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS' WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END,
         c.job__employer__College, c.job__employer__HighSchool, c.job__employer__Nfl
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL), n AS (SELECT c, a, role, level, LAG(role) OVER (PARTITION BY c ORDER BY a, z) AS prev_role, LAG(level) OVER (PARTITION BY c ORDER BY a, z) AS prev_level, LAG(college) OVER (PARTITION BY c ORDER BY a, z) AS prev_college FROM jobs)
         SELECT n.a AS year, n.c AS id, co.name, (SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = n.prev_college) AS from_program, co.career_titles AS titles FROM n JOIN coaches co ON co._row = n.c
         WHERE n.role = 'Head coach' AND n.level = 'NFL' AND n.prev_role = 'Head coach' AND n.prev_level NOT IN ('NFL', 'HS', 'Other') ORDER BY n.a DESC"#,
        says: None,
        needs: W,
        open: "e/coaches/{id}",
    },
    Panel {
        id: "S23.to_college",
        page: "coaches",
        area: "S23",
        title: r#"High school coaches who moved into college"#,
        kind: "table",
        note: r#"Every high school head coach whose next job was at a college: the school, the college, the role and year."#,
        sql: r#"WITH st AS MATERIALIZED (SELECT h._parent AS c, h._i AS i, CASE WHEN h.role = 'HeadCoach' THEN 'Head coach' WHEN h.role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN h.role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN h.role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, h."from" AS a, h."to" AS z, h.wins AS w, h.losses AS l, h.titles AS titles, h.ended AS ended,
         CASE WHEN h.employer__College IS NOT NULL THEN COALESCE((SELECT x._2 FROM programs__history x WHERE x._parent = h.employer__College AND h."to" BETWEEN x._0 AND x._1), p.division)
         WHEN h.employer__HighSchool IS NOT NULL THEN 'HS' WHEN h.employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level,
         h.employer__College AS college, h.employer__HighSchool AS hs, h.employer__Nfl AS nfl
         FROM coaches__history h LEFT JOIN programs p ON p._row = h.employer__College), jobs AS MATERIALIZED (SELECT c, role, a, z, w, l, titles, ended, level, college, hs, nfl FROM st
         UNION ALL SELECT c._row, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END, c.job__since, (SELECT MAX(year) FROM programs__seasons), c.wins, c.losses, c.titles, 'current',
         CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS' WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END,
         c.job__employer__College, c.job__employer__HighSchool, c.job__employer__Nfl
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL), n AS (SELECT c, a, role, level, college, LAG(level) OVER (PARTITION BY c ORDER BY a, z) AS prev_level, LAG(hs) OVER (PARTITION BY c ORDER BY a, z) AS prev_hs, LAG(role) OVER (PARTITION BY c ORDER BY a, z) AS prev_role FROM jobs)
         SELECT n.a AS year, n.c AS id, co.name, (SELECT name || ' (' || town || ')' FROM high_schools WHERE _row = n.prev_hs) AS from_school, (SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = n.college) AS to_program,
         n.level, n.role FROM n JOIN coaches co ON co._row = n.c WHERE n.prev_level = 'HS' AND n.prev_role = 'Head coach' AND n.college IS NOT NULL ORDER BY n.a DESC"#,
        says: None,
        needs: W,
        open: "e/coaches/{id}",
    },
    Panel {
        id: "S24.turnover",
        page: "coaches",
        area: "S24",
        title: r#"New staff members a year at FBS"#,
        kind: "line",
        note: r#"Each season: new coordinators and position coaches per FBS program (jobs that began that season)."#,
        sql: r#"WITH st AS MATERIALIZED (SELECT h._parent AS c, h._i AS i, CASE WHEN h.role = 'HeadCoach' THEN 'Head coach' WHEN h.role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN h.role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN h.role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, h."from" AS a, h."to" AS z, h.wins AS w, h.losses AS l, h.titles AS titles, h.ended AS ended,
         CASE WHEN h.employer__College IS NOT NULL THEN COALESCE((SELECT x._2 FROM programs__history x WHERE x._parent = h.employer__College AND h."to" BETWEEN x._0 AND x._1), p.division)
         WHEN h.employer__HighSchool IS NOT NULL THEN 'HS' WHEN h.employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level,
         h.employer__College AS college, h.employer__HighSchool AS hs, h.employer__Nfl AS nfl
         FROM coaches__history h LEFT JOIN programs p ON p._row = h.employer__College), jobs AS MATERIALIZED (SELECT c, role, a, z, w, l, titles, ended, level, college, hs, nfl FROM st
         UNION ALL SELECT c._row, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END, c.job__since, (SELECT MAX(year) FROM programs__seasons), c.wins, c.losses, c.titles, 'current',
         CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS' WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END,
         c.job__employer__College, c.job__employer__HighSchool, c.job__employer__Nfl
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL) SELECT j.a AS year, ROUND(1.0 * SUM(j.role IN ('Offensive coordinator', 'Defensive coordinator')) / (SELECT COUNT(*) FROM programs__seasons s JOIN programs p ON p._row = s._parent WHERE s.year = j.a AND p.division = 'FBS'), 2) AS coordinators,
         ROUND(1.0 * SUM(j.role = 'Position coach') / (SELECT COUNT(*) FROM programs__seasons s JOIN programs p ON p._row = s._parent WHERE s.year = j.a AND p.division = 'FBS'), 2) AS position_coaches
         FROM jobs j WHERE j.level = 'FBS' AND j.a > (SELECT MIN(year) FROM programs__seasons) GROUP BY j.a ORDER BY j.a"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S24.holdovers",
        page: "coaches",
        area: "S24",
        title: r#"Coordinators kept by a new head coach"#,
        kind: "line",
        note: r#"Each season an FBS program changed head coach: the share of its coordinators from the season before who stayed on (design 40%, 75% if promoted)."#,
        sql: r#"WITH ps AS MATERIALIZED (SELECT s._parent AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.conference_title AS ct, s.national_title AS nt, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division, COALESCE(h._3, p.conference) AS conference
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1), st AS MATERIALIZED (SELECT h._parent AS c, h._i AS i, CASE WHEN h.role = 'HeadCoach' THEN 'Head coach' WHEN h.role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN h.role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN h.role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, h."from" AS a, h."to" AS z, h.wins AS w, h.losses AS l, h.titles AS titles, h.ended AS ended,
         CASE WHEN h.employer__College IS NOT NULL THEN COALESCE((SELECT x._2 FROM programs__history x WHERE x._parent = h.employer__College AND h."to" BETWEEN x._0 AND x._1), p.division)
         WHEN h.employer__HighSchool IS NOT NULL THEN 'HS' WHEN h.employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level,
         h.employer__College AS college, h.employer__HighSchool AS hs, h.employer__Nfl AS nfl
         FROM coaches__history h LEFT JOIN programs p ON p._row = h.employer__College), jobs AS MATERIALIZED (SELECT c, role, a, z, w, l, titles, ended, level, college, hs, nfl FROM st
         UNION ALL SELECT c._row, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END, c.job__since, (SELECT MAX(year) FROM programs__seasons), c.wins, c.losses, c.titles, 'current',
         CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS' WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END,
         c.job__employer__College, c.job__employer__HighSchool, c.job__employer__Nfl
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL), ch AS (SELECT prog, year FROM (SELECT prog, year, coach, LAG(coach) OVER (PARTITION BY prog ORDER BY year) AS before FROM ps WHERE division = 'FBS') WHERE coach IS NOT before AND before IS NOT NULL)
         SELECT ch.year, ROUND(100.0 * AVG(j.z >= ch.year), 1) AS world, COUNT(*) AS n FROM ch JOIN jobs j ON j.college = ch.prog AND j.role IN ('Offensive coordinator', 'Defensive coordinator') AND ch.year - 1 BETWEEN j.a AND j.z
         GROUP BY ch.year ORDER BY ch.year"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S24.empty",
        page: "coaches",
        area: "S24",
        title: r#"Empty seats now"#,
        kind: "bars",
        note: r#"Programs now missing an OC, a DC or position coaches (of eight), by level."#,
        sql: r#"SELECT division AS label, ROUND(100.0 * AVG(oc IS NULL OR dc IS NULL), 1) AS world, ROUND(AVG((SELECT COUNT(*) FROM json_each(position_coaches) WHERE value IS NULL)), 2) AS empty_position_seats, COUNT(*) AS n
         FROM programs WHERE _row IN (SELECT _parent FROM programs__seasons WHERE year = (SELECT MAX(year) FROM programs__seasons)) GROUP BY division ORDER BY n DESC"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S24.staff",
        page: "coaches",
        area: "S24",
        title: r#"Every program's staff now"#,
        kind: "table",
        note: r#"Each program's head coach, coordinators and position coaches now, and when the head coach last changed."#,
        sql: r#"SELECT p._row AS id, COALESCE(p.real_name, p.fictional_name) AS program, p.division, (SELECT name FROM coaches WHERE _row = p.head_coach) AS head_coach, p.hc_changed AS head_coach_since,
         (SELECT name FROM coaches WHERE _row = p.oc) AS oc, (SELECT name FROM coaches WHERE _row = p.dc) AS dc,
         (SELECT group_concat(c.name, ', ') FROM json_each(p.position_coaches) j JOIN coaches c ON c._row = j.value) AS position_coaches
         FROM programs p ORDER BY CASE p.division WHEN 'FBS' THEN 0 WHEN 'FCS' THEN 1 ELSE 2 END, p.prestige DESC"#,
        says: None,
        needs: W,
        open: "e/programs/{id}",
    },
    Panel {
        id: "S25.support",
        page: "coaches",
        area: "S25",
        title: r#"Support staff ratings by level"#,
        kind: "table",
        note: r#"Mean support head ratings (1-20) now: strength and conditioning, medical, analytics, academics, by level, and at NFL clubs (design N(5.5 + 7 x budget + 2 x era, 2.8))."#,
        sql: r#"SELECT division AS level, ROUND(AVG(support__strength__rating), 1) AS strength, ROUND(AVG(support__medical__rating), 1) AS medical, ROUND(AVG(support__analytics__rating), 1) AS analytics,
         ROUND(AVG(support__academic__rating), 1) AS academic, COUNT(*) AS n FROM programs GROUP BY division
         UNION ALL SELECT 'NFL', ROUND(AVG(support__strength__rating), 1), ROUND(AVG(support__medical__rating), 1), ROUND(AVG(support__analytics__rating), 1), NULL, COUNT(*) FROM clubs"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S26.exits",
        page: "coaches",
        area: "S26",
        title: r#"Retirements and early exits a year"#,
        kind: "line",
        note: r#"Jobs that ended in retirement, retirement for health, burnout or walking away on top each year."#,
        sql: r#"WITH st AS MATERIALIZED (SELECT h._parent AS c, h._i AS i, CASE WHEN h.role = 'HeadCoach' THEN 'Head coach' WHEN h.role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN h.role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN h.role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, h."from" AS a, h."to" AS z, h.wins AS w, h.losses AS l, h.titles AS titles, h.ended AS ended,
         CASE WHEN h.employer__College IS NOT NULL THEN COALESCE((SELECT x._2 FROM programs__history x WHERE x._parent = h.employer__College AND h."to" BETWEEN x._0 AND x._1), p.division)
         WHEN h.employer__HighSchool IS NOT NULL THEN 'HS' WHEN h.employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level,
         h.employer__College AS college, h.employer__HighSchool AS hs, h.employer__Nfl AS nfl
         FROM coaches__history h LEFT JOIN programs p ON p._row = h.employer__College) SELECT z AS year, SUM(ended = 'retired') AS retired, SUM(ended = 'retired for his health') AS health, SUM(ended LIKE 'stepped away%') AS burned_out, SUM(ended LIKE 'walked away%') AS on_top
         FROM st GROUP BY z HAVING SUM(ended LIKE 'retired%' OR ended LIKE 'stepped away%' OR ended LIKE 'walked away%') > 0 ORDER BY z"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S26.retire_age",
        page: "coaches",
        area: "S26",
        title: r#"Age at retirement"#,
        kind: "hist",
        note: r#"Coaches whose last job ended in retirement, by age then (from his age now), head coaches and the rest apart (pick above)."#,
        sql: r#"WITH st AS MATERIALIZED (SELECT h._parent AS c, h._i AS i, CASE WHEN h.role = 'HeadCoach' THEN 'Head coach' WHEN h.role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN h.role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN h.role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, h."from" AS a, h."to" AS z, h.wins AS w, h.losses AS l, h.titles AS titles, h.ended AS ended,
         CASE WHEN h.employer__College IS NOT NULL THEN COALESCE((SELECT x._2 FROM programs__history x WHERE x._parent = h.employer__College AND h."to" BETWEEN x._0 AND x._1), p.division)
         WHEN h.employer__HighSchool IS NOT NULL THEN 'HS' WHEN h.employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level,
         h.employer__College AS college, h.employer__HighSchool AS hs, h.employer__Nfl AS nfl
         FROM coaches__history h LEFT JOIN programs p ON p._row = h.employer__College), last AS (SELECT c, MAX(z) AS z FROM st GROUP BY c)
         SELECT CASE WHEN st.role = 'Head coach' THEN 'Head coaches' ELSE 'Assistants' END AS grp, st.z - COALESCE(CAST(strftime('%Y', date(c.born * 86400, 'unixepoch')) AS INTEGER), CASE WHEN c.retired = 1 THEN (SELECT MAX(h."to") FROM coaches__history h WHERE h._parent = +c._row) + 1 - c.age ELSE (SELECT MAX(year) FROM programs__seasons) + 1 - c.age END) AS x, COUNT(*) AS world
         FROM st JOIN last ON last.c = st.c AND last.z = st.z JOIN coaches c ON c._row = st.c WHERE st.ended LIKE 'retired%' GROUP BY 1, 2 ORDER BY 1, 2"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S26.health",
        page: "coaches",
        area: "S26",
        title: r#"Health episodes in the last 400 days"#,
        kind: "table",
        note: r#"Every coach health episode the world still keeps (`folk.life.health`), with the coach and his job."#,
        sql: r#"SELECT date(h.date * 86400, 'unixepoch') AS date, h.coach AS id, c.name, h.what, date(h.back * 86400, 'unixepoch') AS back, CASE WHEN h.serious = 1 THEN 'serious' ELSE '' END AS serious, c.age
         FROM folk__life__health h JOIN coaches c ON c._row = h.coach ORDER BY h.date DESC"#,
        says: None,
        needs: W,
        open: "e/coaches/{id}",
    },
    Panel {
        id: "S27.ties",
        page: "coaches",
        area: "S27",
        title: r#"Ties to high schools by level and role"#,
        kind: "table",
        note: r#"Coaches in a job now: ties to high schools (0-1, from impact players he coached), how many and how strong, by level and role."#,
        sql: r#"WITH t AS (SELECT CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS' WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level,
         CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, c.ability__network, (SELECT COUNT(*) FROM coaches__ties x WHERE x._parent = +c._row) AS n, (SELECT AVG(x._v) FROM coaches__ties x WHERE x._parent = +c._row) AS s
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL)
         SELECT level, role, COUNT(*) AS coaches, ROUND(AVG(n), 1) AS ties, ROUND(AVG(s), 3) AS strength, ROUND(AVG(ability__network), 1) AS network_debug
         FROM t GROUP BY level, role ORDER BY CASE level WHEN 'NFL' THEN 0 WHEN 'FBS' THEN 1 WHEN 'FCS' THEN 2 WHEN 'D2' THEN 3 WHEN 'D3' THEN 4 WHEN 'NAIA' THEN 5 WHEN 'JUCO' THEN 6 WHEN 'HS' THEN 7 ELSE 8 END, role"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S27.top",
        page: "coaches",
        area: "S27",
        title: r#"The widest recruiting reach"#,
        kind: "table",
        note: r#"The 40 coaches with the most strong ties (0.2+) now, with his job and the states they reach."#,
        sql: r#"WITH t AS (SELECT x._parent, COUNT(*) AS n, group_concat(DISTINCT st.code) AS states FROM coaches__ties x JOIN high_schools h ON h._row = CAST(x._key AS INTEGER) JOIN states st ON st._row = h.state WHERE x._v >= 0.2 GROUP BY x._parent)
         SELECT c._row AS id, c.name, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, COALESCE((SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = c.job__employer__College), '') AS program, t.n AS strong_ties, t.states
         FROM t JOIN coaches c ON c._row = t._parent ORDER BY t.n DESC LIMIT 40"#,
        says: None,
        needs: W,
        open: "e/coaches/{id}",
    },
    Panel {
        id: "S28.hall",
        page: "coaches",
        area: "S28",
        title: r#"Coaches in the Hall of Fame"#,
        kind: "table",
        note: r#"College Football Hall of Fame coaches (`stats.hall`), with the why and their career head coach record. Real bar: 100+ games and .600+ (NFF)."#,
        sql: r#"WITH st AS MATERIALIZED (SELECT h._parent AS c, h._i AS i, CASE WHEN h.role = 'HeadCoach' THEN 'Head coach' WHEN h.role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN h.role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN h.role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, h."from" AS a, h."to" AS z, h.wins AS w, h.losses AS l, h.titles AS titles, h.ended AS ended,
         CASE WHEN h.employer__College IS NOT NULL THEN COALESCE((SELECT x._2 FROM programs__history x WHERE x._parent = h.employer__College AND h."to" BETWEEN x._0 AND x._1), p.division)
         WHEN h.employer__HighSchool IS NOT NULL THEN 'HS' WHEN h.employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level,
         h.employer__College AS college, h.employer__HighSchool AS hs, h.employer__Nfl AS nfl
         FROM coaches__history h LEFT JOIN programs p ON p._row = h.employer__College), jobs AS MATERIALIZED (SELECT c, role, a, z, w, l, titles, ended, level, college, hs, nfl FROM st
         UNION ALL SELECT c._row, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END, c.job__since, (SELECT MAX(year) FROM programs__seasons), c.wins, c.losses, c.titles, 'current',
         CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS' WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END,
         c.job__employer__College, c.job__employer__HighSchool, c.job__employer__Nfl
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL), r AS (SELECT c, SUM(w) AS w, SUM(l) AS l FROM jobs WHERE role = 'Head coach' AND college IS NOT NULL GROUP BY c)
         SELECT h.year, h.coach AS id, h.name, h.division, h.why, r.w || '-' || r.l AS record, ROUND(1.0 * r.w / NULLIF(r.w + r.l, 0), 3) AS pct
         FROM stats__hall h LEFT JOIN coaches c ON c.name = h.name LEFT JOIN r ON r.c = c._row WHERE h.coach = 1 ORDER BY h.year DESC"#,
        says: None,
        needs: W,
        open: "e/coaches/{id}",
    },
    Panel {
        id: "S29.other",
        page: "coaches",
        area: "S29",
        title: r#"Other leagues' coaches"#,
        kind: "table",
        note: r#"Head jobs in the other leagues (CFL, arena, spring leagues), current and finished, with record and what came next."#,
        sql: r#"WITH st AS MATERIALIZED (SELECT h._parent AS c, h._i AS i, CASE WHEN h.role = 'HeadCoach' THEN 'Head coach' WHEN h.role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN h.role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN h.role__PositionCoach IS NOT NULL THEN 'Position coach' END AS role, h."from" AS a, h."to" AS z, h.wins AS w, h.losses AS l, h.titles AS titles, h.ended AS ended,
         CASE WHEN h.employer__College IS NOT NULL THEN COALESCE((SELECT x._2 FROM programs__history x WHERE x._parent = h.employer__College AND h."to" BETWEEN x._0 AND x._1), p.division)
         WHEN h.employer__HighSchool IS NOT NULL THEN 'HS' WHEN h.employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END AS level,
         h.employer__College AS college, h.employer__HighSchool AS hs, h.employer__Nfl AS nfl
         FROM coaches__history h LEFT JOIN programs p ON p._row = h.employer__College), jobs AS MATERIALIZED (SELECT c, role, a, z, w, l, titles, ended, level, college, hs, nfl FROM st
         UNION ALL SELECT c._row, CASE WHEN c.job__role = 'HeadCoach' THEN 'Head coach' WHEN c.job__role = 'OffensiveCoordinator' THEN 'Offensive coordinator'
         WHEN c.job__role = 'DefensiveCoordinator' THEN 'Defensive coordinator' WHEN c.job__role__PositionCoach IS NOT NULL THEN 'Position coach' END, c.job__since, (SELECT MAX(year) FROM programs__seasons), c.wins, c.losses, c.titles, 'current',
         CASE WHEN c.job__employer__College IS NOT NULL THEN p.division WHEN c.job__employer__HighSchool IS NOT NULL THEN 'HS' WHEN c.job__employer__Nfl IS NOT NULL THEN 'NFL' ELSE 'Other' END,
         c.job__employer__College, c.job__employer__HighSchool, c.job__employer__Nfl
         FROM coaches c LEFT JOIN programs p ON p._row = c.job__employer__College WHERE c.job__role IS NOT NULL OR c.job__role__PositionCoach IS NOT NULL), n AS (SELECT c, a, z, role, level, w, l, ended, LEAD(level) OVER (PARTITION BY c ORDER BY a, z) AS next_level, LEAD(role) OVER (PARTITION BY c ORDER BY a, z) AS next_role FROM jobs)
         SELECT n.c AS id, co.name, n.a || '-' || n.z AS seasons, n.w || '-' || n.l AS record, n.ended, COALESCE(n.next_level || ' ' || lower(n.next_role), '') AS next_job
         FROM n JOIN coaches co ON co._row = n.c WHERE n.level = 'Other' AND n.role = 'Head coach' ORDER BY n.a DESC"#,
        says: None,
        needs: W,
        open: "e/coaches/{id}",
    },
    Panel {
        id: "S30.rules",
        page: "coaches",
        area: "S30",
        title: r#"The rule books by season"#,
        kind: "table",
        note: r#"The rules each season runs under (`calendar/rules.rs` RuleBook::era, as the engine reads them): the NFL's padded practices and offseason weeks, the transfer and redshirt rules, NIL and the playoff. The NCAA's 20-hour week (1991), summer access (2013) and camp's 25 practices (2021) are in `training/rules.rs` (`docs/design/training.md`)."#,
        sql: r#"SELECT season, nfl_padded_practices, nfl_offseason, nfl_roster, nfl_practice_squad, cfp_teams, fbs_games, transfer, one_time_transfer, four_game_redshirt, five_in_five, nil, early_signing, revenue_sharing
         FROM engine_rules ORDER BY season DESC"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S31.prep",
        page: "coaches",
        area: "S31",
        title: r#"Each player's latest prep word"#,
        kind: "bars",
        note: r#"College and NFL players now by their latest weekly prep word (`train.prep`, Unprepared to Sharp; players with no word yet left out): Prepared should be the most common, Unprepared and Underprepared together under 12%."#,
        sql: r#"WITH w(k, word) AS (VALUES (1, 'Unprepared'), (2, 'Underprepared'), (3, 'Patchy'), (4, 'Prepared'), (5, 'Sharp'))
         SELECT x.stage AS grp, w.word AS label, ROUND(100.0 * COUNT(*) / SUM(COUNT(*)) OVER (PARTITION BY x.stage), 1) AS world, COUNT(*) AS n
         FROM people x JOIN w ON w.k = (x.player__train__prep & 7) WHERE x.stage IN ('College', 'Pro') GROUP BY x.stage, w.k ORDER BY x.stage, w.k"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S31.rust",
        page: "coaches",
        area: "S31",
        title: r#"Rust and load now"#,
        kind: "table",
        note: r#"College and NFL players now: the share carrying rust from poor prep, the mean rust in points of reading and technique, this week's and the chronic load (tens of units), and the share with a load spike over 1.45 (rare outside camp)."#,
        sql: r#"SELECT stage, COUNT(*) AS players, ROUND(100.0 * AVG(json_extract(player__train__rust, '$[0]') + json_extract(player__train__rust, '$[1]') > 0), 1) AS rusty_pct,
         ROUND(AVG(json_extract(player__train__rust, '$[0]')) / 10.0, 2) AS reading_rust, ROUND(AVG(json_extract(player__train__rust, '$[1]')) / 10.0, 2) AS technique_rust,
         ROUND(AVG(json_extract(player__train__load, '$[0]')), 1) AS load_now, ROUND(AVG(json_extract(player__train__load, '$[1]')), 1) AS load_chronic,
         ROUND(100.0 * AVG(1.0 * json_extract(player__train__load, '$[0]') / NULLIF(json_extract(player__train__load, '$[1]'), 0) > 1.45), 1) AS spike_pct
         FROM people WHERE stage IN ('College', 'Pro') AND player__train__load IS NOT NULL GROUP BY stage"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S34.agencies",
        page: "nfl",
        area: "S34",
        title: r#"Agencies by clients"#,
        kind: "bars",
        note: r#"NFL agents' clients summed by agency, biggest first. Real: a few big agencies hold most first-round picks (no open count)."#,
        sql: r#"SELECT agency AS label, SUM(json_array_length(clients)) AS world, COUNT(*) AS agents, ROUND(AVG(honesty), 1) AS honesty FROM market__agents GROUP BY agency ORDER BY world DESC"#,
        says: None,
        needs: W,
        open: "",
    },
    Panel {
        id: "S34.agents",
        page: "nfl",
        area: "S34",
        title: r#"Every agent"#,
        kind: "table",
        note: r#"Each agent's agency, size, skill, honesty, clients and how often he was fired."#,
        sql: r#"SELECT name, agency, size, ROUND(skill, 1) AS skill, ROUND(honesty, 1) AS honesty, json_array_length(clients) AS clients, fired FROM market__agents ORDER BY json_array_length(clients) DESC"#,
        says: None,
        needs: W,
        open: "",
    },
];
