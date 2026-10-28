//! A small stand-in book for the tests and for looking at the screens before
//! a real book is at hand. Its numbers are made up for the look and say so
//! in its meta; nothing reads it as a world.

use std::path::Path;

use rusqlite::{Connection, params};

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
    fn pick<'a>(&mut self, v: &[&'a str]) -> &'a str {
        v[(self.next() * v.len() as f64) as usize % v.len()]
    }
}

const FIRST: [&str; 12] =
    ["Marcus", "Tyler", "Jalen", "Kirkland", "Devon", "Caleb", "Andre", "Brady", "Isaiah", "Gregg", "Shawn", "Daniel"];
const LAST: [&str; 12] = [
    "Overstreet",
    "White",
    "Young",
    "Williams",
    "Bates",
    "Johnson",
    "Carter",
    "Hill",
    "Brooks",
    "Reed",
    "Mason",
    "Price",
];
const POS: [&str; 10] = ["QB", "RB", "WR", "TE", "OT", "EDGE", "DT", "LB", "CB", "S"];
const STATES: [&str; 8] = ["TX", "FL", "GA", "CA", "OH", "AL", "LA", "PA"];
const CONF: [(&str, &str); 5] =
    [("SEC", "FBS"), ("Big Ten", "FBS"), ("ACC", "FBS"), ("Mountain West", "FBS"), ("Big Sky", "FCS")];
const PROGRAMS: [&str; 20] = [
    "Georgia",
    "Texas",
    "Alabama",
    "LSU",
    "Florida",
    "Ohio State",
    "Michigan",
    "Penn State",
    "Iowa",
    "Wisconsin",
    "Clemson",
    "Miami",
    "Florida State",
    "Virginia Tech",
    "Boise State",
    "San Diego State",
    "Fresno State",
    "Montana",
    "Montana State",
    "Weber State",
];

/// Writes the stand-in book at `path` (replacing it).
pub fn write(path: &Path) -> Result<(), String> {
    let _ = std::fs::remove_file(path);
    let db = Connection::open(path).map_err(|e| e.to_string())?;
    let e = |r: rusqlite::Result<usize>| r.map(|_| ()).map_err(|e| e.to_string());
    db.execute_batch(
        "BEGIN;
        CREATE TABLE meta (key TEXT PRIMARY KEY, value);
        CREATE TABLE checks (id TEXT PRIMARY KEY, page TEXT, area TEXT, label TEXT, world REAL, real REAL, unit TEXT, source TEXT, years TEXT, mark TEXT, why TEXT, file TEXT, function TEXT, vital INTEGER, level TEXT);
        CREATE TABLE odd (id INTEGER PRIMARY KEY, page TEXT, scope TEXT, text TEXT, why TEXT, mark TEXT, entity TEXT, entity_id INTEGER, rank INTEGER);
        CREATE TABLE issues (id TEXT PRIMARY KEY, title TEXT, touches TEXT, levels TEXT, years TEXT, status TEXT, fixed_in TEXT, source TEXT);
        CREATE TABLE knobs (id TEXT PRIMARY KEY, name TEXT, value REAL, file TEXT, drives TEXT);
        CREATE TABLE coverage (tbl TEXT, page TEXT, source TEXT, years TEXT, leaves_out TEXT);
        CREATE TABLE defs (tbl TEXT, col TEXT, label TEXT, what TEXT, unit TEXT, file TEXT, function TEXT, source TEXT, years TEXT, debug INTEGER);
        CREATE TABLE conferences (id INTEGER PRIMARY KEY, name TEXT, division TEXT);
        CREATE TABLE programs (id INTEGER PRIMARY KEY, name TEXT, conference_id INTEGER, conference TEXT, division TEXT, state TEXT, prestige REAL);
        CREATE TABLE coaches (id INTEGER PRIMARY KEY, name TEXT, program_id INTEGER, level TEXT, born INTEGER, hidden_value REAL);
        CREATE TABLE people (id INTEGER PRIMARY KEY, first_name TEXT, last_name TEXT, position TEXT, state TEXT, stars INTEGER, class_year INTEGER, level TEXT, program_id INTEGER, height_in REAL, weight INTEGER, forty REAL, potential REAL, first_year INTEGER, last_year INTEGER);
        CREATE TABLE person_seasons (person_id INTEGER, year INTEGER, age INTEGER, level TEXT, program_id INTEGER, role TEXT, rating REAL, played_at REAL, potential REAL, change REAL, why TEXT);
        CREATE TABLE games (id INTEGER PRIMARY KEY, name TEXT, year INTEGER, level TEXT, home_id INTEGER, away_id INTEGER, home_points INTEGER, away_points INTEGER);
        CREATE TABLE plays (game_id INTEGER, n INTEGER, quarter INTEGER, clock TEXT, offense TEXT, down INTEGER, togo INTEGER, yardline_100 INTEGER, kind TEXT, call TEXT, yards REAL, why TEXT);
        CREATE TABLE real_players (id INTEGER PRIMARY KEY, name TEXT, position TEXT, draft_pick INTEGER, seasons INTEGER, games INTEGER, pro_bowls INTEGER);
        COMMIT;",
    )
    .map_err(|e| e.to_string())?;
    let mut r = Lcg(2026);
    for (k, v) in [
        ("name", "Stand-in book (made-up numbers, for the look)"),
        ("seed", "2026"),
        ("history_years", "30"),
        ("commit", "0000000000000000000000000000000000000000"),
        ("built", "2026-10-28"),
        ("first", "1996"),
        ("last", "2025"),
        ("has_log", "0"),
        (
            "summary",
            "A small made-up book that shows every kind of block. A real book comes from a world file and its world log.",
        ),
    ] {
        e(db.execute("INSERT INTO meta VALUES (?1, ?2)", params![k, v]))?;
    }
    type Row =
        (&'static str, &'static str, &'static str, &'static str, f64, f64, &'static str, &'static str, &'static str);
    let checks: [Row; 14] = [
        ("fbs_ypc", "games", "G24", "FBS yards a carry", 4.94, 5.12, "", "cfbfastR play by play", "2014-2025"),
        ("nfl_ypc", "games", "G24", "NFL yards a carry", 4.29, 4.50, "", "nflverse play by play", "2021-2025"),
        ("nfl_points", "games", "G1", "NFL points a team game", 30.9, 22.5, "", "nflverse games", "2021-2025"),
        ("r1_wr", "nfl", "N19", "Round 1 receivers", 13.6, 13.4, "%", "nflverse draft picks", "2000-2025"),
        ("r1_edge", "nfl", "N19", "Round 1 edge rushers", 15.0, 13.5, "%", "nflverse draft picks", "2000-2025"),
        (
            "five_drafted",
            "recruiting",
            "C30",
            "Five-stars drafted",
            87.6,
            62.0,
            "%",
            "247Sports composite and the draft",
            "2010-2021",
        ),
        ("fbs_roster", "programs", "C20", "FBS roster median", 86.0, 105.0, "players", "team rosters", "2023"),
        (
            "most_titles",
            "programs",
            "C5",
            "Most titles, one school in 15 years",
            9.0,
            6.0,
            "",
            "national champions list",
            "1869-2025",
        ),
        (
            "hs_streak",
            "high-school",
            "C25",
            "Longest high school win streak",
            248.0,
            151.0,
            "games",
            "De La Salle's record",
            "1992-2004",
        ),
        ("forty_cb", "people", "P6", "Combine forty, cornerbacks", 4.47, 4.48, "s", "NFL combine results", "2000-2025"),
        ("career_len", "people", "P13", "NFL career length", 3.4, 3.3, "seasons", "nfl_careers", "1990-2020"),
        ("hc_tenure", "coaches", "S8", "FBS head coach tenure", 5.1, 4.6, "seasons", "college_people", "2000-2025"),
        ("nfl_cap", "money", "N31", "Cap space used", 97.0, 98.5, "%", "OTC (not reachable: from memory)", "2024"),
        ("tv_slots", "world", "N36", "Prime-time games a week", 3.0, 3.0, "", "NFL schedule", "2025"),
    ];
    for (i, (id, page, area, label, w, rl, unit, src, yrs)) in checks.iter().enumerate() {
        let file = ["game/scrimmage.rs", "world/draft.rs", "world/recruiting.rs", "world/season.rs"][i % 4];
        e(db.execute(
            "INSERT INTO checks VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,NULL,?10,?11,?12,?13,NULL)",
            params![
                id,
                page,
                area,
                label,
                w,
                rl,
                unit,
                src,
                yrs,
                format!("{label}: the world against {src}."),
                file,
                "run_snap",
                (i < 12) as i64
            ],
        ))?;
    }
    for (i, (c, d)) in CONF.iter().enumerate() {
        e(db.execute("INSERT INTO conferences VALUES (?1,?2,?3)", params![i as i64 + 1, c, d]))?;
    }
    for (i, p) in PROGRAMS.iter().enumerate() {
        let ci = [0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 4, 4, 4][i];
        let (cn, dv) = CONF[ci];
        e(db.execute(
            "INSERT INTO programs VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![i as i64, p, ci as i64 + 1, cn, dv, r.pick(&STATES), (90.0 - i as f64 * 3.0 + r.next() * 5.0)],
        ))?;
        e(db.execute(
            "INSERT INTO coaches VALUES (?1,?2,?3,?4,?5,?6)",
            params![
                i as i64 + 1,
                format!("{} {}", r.pick(&FIRST), r.pick(&LAST)),
                i as i64,
                dv,
                1960 + (r.next() * 20.0) as i64,
                8.0 + r.next() * 8.0
            ],
        ))?;
    }
    for id in 1..=240i64 {
        let pos = r.pick(&POS);
        let stars = 1 + (r.next() * 5.0) as i64;
        let class = 1996 + (r.next() * 26.0) as i64;
        let prog = (r.next() * 20.0) as i64;
        let pot = 55.0 + r.next() * 40.0;
        e(db.execute(
            "INSERT INTO people VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
            params![
                id,
                r.pick(&FIRST),
                r.pick(&LAST),
                pos,
                r.pick(&STATES),
                stars,
                class,
                if r.next() < 0.3 { "NFL" } else { "FBS" },
                prog,
                70.0 + r.next() * 8.0,
                180 + (r.next() * 120.0) as i64,
                ((4.35 + r.next() * 0.6) * 100.0).round() / 100.0,
                pot,
                class - 4,
                class + 4 + (r.next() * 8.0) as i64
            ],
        ))?;
        let mut rating = 35.0 + r.next() * 15.0;
        for k in 0..(6 + (r.next() * 8.0) as i64) {
            let year = class - 4 + k;
            let ch = (r.next() * 12.0 - 3.0) * if k > 9 { -0.6 } else { 1.0 };
            let why = if ch > 6.0 {
                "Spring breakout, as his coaches hinted."
            } else if ch < -1.0 {
                "A form slump and a minor injury."
            } else {
                "Steady growth."
            };
            let level = if k < 4 {
                "HS"
            } else if k < 8 {
                "FBS"
            } else {
                "NFL"
            };
            e(db.execute(
                "INSERT INTO person_seasons VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                params![
                    id,
                    year,
                    14 + k,
                    level,
                    prog,
                    "starter",
                    (rating + ch).min(99.0),
                    rating + ch * 0.5,
                    pot,
                    ch,
                    why
                ],
            ))?;
            rating = (rating + ch).min(99.0);
        }
    }
    for g in 1..=30i64 {
        let (h, a) = ((r.next() * 20.0) as i64, (r.next() * 20.0) as i64);
        let (hp, ap) = ((r.next() * 45.0) as i64, (r.next() * 45.0) as i64);
        e(db.execute(
            "INSERT INTO games VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                g,
                format!("{} at {}, 2025", PROGRAMS[a as usize], PROGRAMS[h as usize]),
                2025,
                "FBS",
                h,
                a,
                hp,
                ap
            ],
        ))?;
        let mut spot = 75i64;
        for n in 1..=40i64 {
            let yards = (r.next() * 14.0 - 3.0).round();
            spot = (spot - yards as i64).clamp(1, 99);
            e(db.execute(
                "INSERT INTO plays VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
                params![
                    g,
                    n,
                    1 + (n - 1) / 10,
                    format!("{}:{:02}", 14 - (n % 10), (n * 7) % 60),
                    if n % 2 == 0 { "home" } else { "away" },
                    1 + n % 4,
                    10 - (n % 4) * 2,
                    spot,
                    r.pick(&["run", "pass", "pass", "run", "punt"]),
                    r.pick(&["inside zone", "mesh", "four verticals", "power", "stick"]),
                    yards,
                    r.pick(&[
                        "The guard won his block.",
                        "Pressure off the edge forced it early.",
                        "Cover 3 left the seam open."
                    ])
                ],
            ))?;
        }
    }
    for i in 1..=120i64 {
        e(db.execute(
            "INSERT INTO real_players VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![
                i,
                format!("Real player {i}"),
                r.pick(&POS),
                1 + (r.next() * 224.0) as i64,
                1 + (r.next() * 13.0) as i64,
                (r.next() * 200.0) as i64,
                (r.next() * 4.0) as i64
            ],
        ))?;
    }
    let odd: [(&str, &str, &str, &str, &str, i64); 5] = [
        (
            "people",
            "entity",
            "86.5 sacks on 653 snaps in one high school season.",
            "Far beyond any real high school season.",
            "far",
            3,
        ),
        ("high-school", "league", "One school won 248 straight.", "The real record is 151, De La Salle.", "far", 0),
        (
            "coaches",
            "entity",
            "A head coach at 299-18 with a hidden value of 13.6.",
            "The most overrated FBS head coach in the world.",
            "debug",
            0,
        ),
        ("recruiting", "league", "87.6% of five-stars get drafted.", "Real is 62%.", "far", 0),
        (
            "people",
            "data",
            "Games missed don't add up: 11 missed, yet 16 played.",
            "The development row and the season line disagree.",
            "check",
            7,
        ),
    ];
    for (i, (pg, sc, t, w, m, ent)) in odd.iter().enumerate() {
        let (et, eid): (Option<&str>, Option<i64>) = if *ent > 0 { (Some("people"), Some(*ent)) } else { (None, None) };
        e(db.execute(
            "INSERT INTO odd VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![i as i64 + 1, pg, sc, t, w, m, et, eid, i as i64],
        ))?;
    }
    for (id, t, touches, st) in [
        ("O1", "Red zone trips were never counted", "red_zone_trips", "fixed"),
        ("O2", "NFL scoring above the agreed band", "nfl_points", "open"),
        ("O3", "High school stat lines from the stand-in before 2014", "hs lines", "open"),
    ] {
        e(db.execute(
            "INSERT INTO issues VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![id, t, touches, "all", "1996-2025", st, if st == "fixed" { "d41bed5" } else { "" }, "oddities.md"],
        ))?;
    }
    for (id, n, v, f, d) in [
        ("dev.base", "Base growth a year", 2.1, "development.rs", "growth"),
        ("rec.local", "Home-state pull", 0.45, "world/recruiting.rs", "pipelines"),
    ] {
        e(db.execute("INSERT INTO knobs VALUES (?1,?2,?3,?4,?5)", params![id, n, v, f, d]))?;
    }
    for (t, pg) in [
        ("people", "people"),
        ("person_seasons", "people"),
        ("programs", "programs"),
        ("coaches", "coaches"),
        ("games", "games"),
        ("plays", "games"),
    ] {
        e(db.execute("INSERT INTO coverage VALUES (?1,?2,?3,?4,?5)", params![t, pg, "world file", "1996-2025", ""]))?;
    }
    for (t, c, l, w, u, f, fun, src, yrs, dbg) in [
        (
            "people",
            "forty",
            "Forty",
            "His forty-yard dash at the combine, hand-timed equivalent.",
            "s",
            "body.rs",
            "combine",
            "NFL combine results",
            "2000-2025",
            0,
        ),
        (
            "people",
            "potential",
            "Potential",
            "His true ceiling: hidden in the game.",
            "",
            "development.rs",
            "potential",
            "",
            "",
            1,
        ),
        (
            "person_seasons",
            "rating",
            "Rating",
            "His overall at the season's end.",
            "",
            "level_rating.rs",
            "overall",
            "",
            "",
            0,
        ),
        (
            "coaches",
            "hidden_value",
            "Hidden value",
            "What the coach is truly worth: hidden in the game.",
            "",
            "world/coaches.rs",
            "value",
            "",
            "",
            1,
        ),
    ] {
        e(db.execute(
            "INSERT INTO defs VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![t, c, l, w, u, f, fun, src, yrs, dbg],
        ))?;
    }
    Ok(())
}
