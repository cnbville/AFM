//! The coverage map (inventory T4): for every inventory area, the book's
//! tables that hold its data, the checks and distributions over it, and a
//! status: checked, data with no check yet, needs a log this book lacks,
//! or stored nowhere. Also what each table is (`what`, the catalog's
//! column) and the stores each source gave.

use std::collections::{BTreeMap, BTreeSet};

use rusqlite::params;

use crate::book::Book;
use crate::log_in::LogInfo;
use crate::pages;

/// What a table holds, in words, for the catalog.
pub fn what(table: &str) -> String {
    const KNOWN: &[(&str, &str)] = &[
        (
            "people",
            "everyone alive in football today: one row per person (player__id), stage, class, college, ratings; x_overall and x_potential worked out by the reader",
        ),
        ("alumni", "everyone kept after leaving: drafted, undrafted pros and 4-5 stars who quit"),
        ("coaches", "every coach today, with his job, ratings and record"),
        ("programs", "every college program, with its seasons, history, prestige and pipelines"),
        ("high_schools", "every high school, with its yearly seasons"),
        ("clubs", "the 32 NFL clubs today: money, depth charts, scouts and files"),
        ("college", "this college season: teams, games, lines, polls, brackets"),
        ("college_prev", "last college season, kept whole"),
        ("hs_season", "this high school season: groups, schedules, results"),
        ("nfl_book", "the NFL's record book: careers, awards, Pro Bowls, All-Pro, the Hall"),
        ("stats", "the college and high school stat books, honors, streaks and records"),
        ("market", "the NFL market: agents, tags, trades, picks and yearly counts"),
        ("recruiting_log", "the recruiting log: five-stars, the portal and transfers"),
        ("log_history", "the world's yearly counts (signings, portal, declarations, high school totals)"),
        ("news", "the news feed as the world keeps it"),
        ("world", "the world's top-level facts"),
        ("engine_rules", "the rule book the engine uses each season (calendar::RuleBook::era)"),
        ("engine_dates", "each season's dates as the engine sets them (calendar::YearDates::new)"),
        ("engine_attributes", "the 67 visible attributes and 5 hidden traits, with their labels"),
        ("real_figures", "every real figure in data/real-world/index.json, with source, years and status"),
        ("real_files", "every real data file, with what it holds"),
        ("real_data", "each real data file's text, as the reader was built with it"),
        ("log_games", "game log: one row per game (season, stream, home, away, points)"),
        ("log_plays", "game log: every play, packed (read with more(table, _more))"),
        ("log_situations", "game log: play totals by situation"),
        ("log_team_situations", "game log: play totals by team and situation"),
        ("log_players", "world log: one row per person per year, packed"),
        ("log_growth", "world log: every development step with its why, packed"),
        ("log_whys", "world log: dated whys"),
        ("log_events", "world log: events with their whys"),
        ("log_news", "world log: news as written each day"),
        ("log_exits", "world log: everyone who left, with why, packed"),
        ("log_coaches", "world log: one row per coach per year, packed"),
        ("log_programs", "world log: one row per program per year, packed"),
        ("log_schools", "world log: one row per high school per year, packed"),
        ("log_clubs", "world log: one row per NFL club per year, packed"),
        ("log_snapshots", "world log: whole records on key dates, packed"),
        ("log_world", "world log: the world's yearly row with store fingerprints"),
        ("log_dropped", "world log: what the world threw away"),
        ("log_people", "world log: people rows"),
    ];
    if let Some((_, w)) = KNOWN.iter().find(|(n, _)| *n == table) {
        return w.to_string();
    }
    match table.rsplit_once("__") {
        Some((parent, field)) => format!("{field} of each {parent} row (_parent is the {parent} row)"),
        None => String::new(),
    }
}

/// The engine's types the inventory names, and the tables that hold them.
const TYPES: &[(&str, &[&str])] = &[
    ("Person", &["people"]),
    ("Player", &["people", "alumni"]),
    ("Background", &["people"]),
    ("Family", &["people"]),
    ("ProCareer", &["people", "alumni"]),
    ("ProSeason", &["people__pro__seasons", "alumni__pro__seasons"]),
    ("Contract", &["people"]),
    ("Alumnus", &["alumni"]),
    ("Club", &["clubs"]),
    ("NflTeamSeason", &["clubs"]),
    ("Program", &["programs"]),
    ("ProgramSeason", &["programs__seasons"]),
    ("SeasonRecord", &["programs__season_records"]),
    ("SeasonTotals", &["programs__seasons"]),
    ("Coach", &["coaches"]),
    ("CalendarLog", &["calendar"]),
    ("CollegeSeason", &["college"]),
    ("Game", &["college__games"]),
    ("Fixture", &["college__games"]),
    ("TeamResult", &["college__results"]),
    ("HighSchool", &["high_schools"]),
    ("HsSeason", &["hs_season"]),
    ("NflSeason", &["nfl_season"]),
    ("NflFixture", &["nfl_season"]),
    ("StatBook", &["stats"]),
    ("StatLine", &["stats__seasons"]),
    ("PlayerSeason", &["stats__seasons"]),
    ("Scout", &["clubs__scouts"]),
    ("PlayRecord", &["log_plays"]),
    ("GameStats", &["log_games"]),
    ("TeamStats", &["log_games__box"]),
];

/// The book tables each `World.a.b` or engine type in an area's "where"
/// names.
fn tables_in(text: &str, tables: &BTreeSet<String>) -> Vec<String> {
    let mut out = Vec::new();
    for (ty, ts) in TYPES {
        let named = text.match_indices(ty).any(|(i, _)| {
            let before = text[..i].chars().next_back();
            let after = text[i + ty.len()..].chars().next();
            !before.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_') && matches!(after, Some('.') | Some(':'))
        });
        if named {
            for t in *ts {
                // A type's table, or the store it lives in.
                let t = if tables.contains(*t) { t.to_string() } else { t.split("__").next().unwrap_or(t).to_string() };
                if tables.contains(&t) && !out.contains(&t) {
                    out.push(t);
                }
            }
        }
    }
    for (i, _) in text.match_indices("World.") {
        let rest = &text[i + 6..];
        let end = rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.')).unwrap_or(rest.len());
        let path = rest[..end].trim_end_matches('.').replace('.', "__");
        // The longest prefix that is a table.
        let parts: Vec<&str> = path.split("__").collect();
        for n in (1..=parts.len()).rev() {
            let t = parts[..n].join("__");
            if tables.contains(&t) {
                if !out.contains(&t) {
                    out.push(t);
                }
                break;
            }
        }
    }
    out
}

pub fn write(b: &Book, streams: &[String], log: &Option<LogInfo>) -> rusqlite::Result<()> {
    b.conn.execute_batch(
        "CREATE TABLE coverage(code TEXT PRIMARY KEY, page TEXT, title TEXT, level TEXT, status TEXT, tables TEXT,
            checks TEXT, metrics TEXT, rules TEXT, shows TEXT, needs_new TEXT, note TEXT);
         CREATE TABLE coverage_stores(source TEXT, store TEXT, tables INTEGER, rows INTEGER, what TEXT);",
    )?;
    let tables: BTreeSet<String> = b.tables().into_iter().collect();
    // Area code -> ids, from each list's areas.
    let mut by: BTreeMap<(&str, String), Vec<String>> = BTreeMap::new();
    for (kind, sql) in [
        ("checks", "SELECT id, areas FROM checks WHERE mark NOT IN ('no data')"),
        ("metrics", "SELECT id, areas FROM metrics WHERE rows > 0"),
        ("rules", "SELECT id, areas FROM consistency WHERE status IN ('ok','failed')"),
    ] {
        if let Ok((_, rows)) = b.rows(sql) {
            for r in rows {
                let id = crate::text::cell(&r[0]);
                for a in pages::expand(&crate::text::cell(&r[1])) {
                    by.entry((kind, a)).or_default().push(id.clone());
                }
            }
        }
    }
    let has_game = streams.iter().any(|s| s.starts_with("games-"));
    let has_world = log.as_ref().is_some_and(|l| l.streams.contains_key("players"));
    // Areas the book itself answers (the inventory's missing part).
    let own: &[(&str, &str)] = &[("T1", "known_issues"), ("T2", "knobs"), ("T4", "coverage"), ("T11", "consistency")];
    for (code, a) in pages::areas() {
        let s = |k: &str| a.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
        let wheres: Vec<String> = a
            .get("data")
            .and_then(|d| d.as_array())
            .into_iter()
            .flatten()
            .filter_map(|d| d.get("where").and_then(|w| w.as_str()).map(str::to_string))
            .collect();
        let all = wheres.join(" ");
        let mut tbls = tables_in(&all, &tables);
        if let Some((_, t)) = own.iter().find(|(c, _)| *c == code) {
            tbls.push(t.to_string());
        }
        let lower = all.to_lowercase();
        let wants_game = lower.contains("game log") || lower.contains("box score") || lower.contains("play log");
        let wants_world = lower.contains("world log");
        if wants_game && has_game {
            tbls.push("log_games".into());
        }
        if wants_world && has_world {
            tbls.push("log_players".into());
        }
        let ids = |k: &str| by.get(&(k, code.clone())).cloned().unwrap_or_default();
        let (checks, metrics, rules) = (ids("checks"), ids("metrics"), ids("rules"));
        let needs: Vec<String> = a
            .get("needs_new")
            .and_then(|d| d.as_array())
            .into_iter()
            .flatten()
            .filter_map(|x| x.as_str().map(str::to_string))
            .collect();
        let measured = !(checks.is_empty() && metrics.is_empty() && rules.is_empty());
        let (status, note) = if !tbls.is_empty() && measured {
            ("checked", String::new())
        } else if !tbls.is_empty() {
            ("data, no check yet", "the data is in the book; no check reads it yet".to_string())
        } else if (wants_game && !has_game) || (wants_world && !has_world) {
            ("needs the log", format!("needs the {}", if wants_game && !has_game { "game log" } else { "world log" }))
        } else if measured {
            ("checked", String::new())
        } else {
            ("not stored", "the world keeps none of it yet (see needs_new)".to_string())
        };
        b.conn.execute(
            "INSERT INTO coverage VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            params![
                code,
                pages::page_of(&code),
                s("name"),
                s("level"),
                status,
                tbls.join(" "),
                checks.join(" "),
                metrics.join(" "),
                rules.join(" "),
                s("shows"),
                serde_json::to_string(&needs).unwrap_or_default(),
                note
            ],
        )?;
    }
    // Each top-level store with its tables and rows.
    if let Ok((_, rows)) = b.rows("SELECT tbl, source, rows FROM catalog") {
        let mut stores: BTreeMap<(String, String), (i64, i64)> = BTreeMap::new();
        for r in rows {
            let t = crate::text::cell(&r[0]);
            let top = t.split("__").next().unwrap_or("").to_string();
            let e = stores.entry((crate::text::cell(&r[1]), top)).or_default();
            e.0 += 1;
            if !t.contains("__") {
                e.1 += match r[2] {
                    rusqlite::types::Value::Integer(n) => n,
                    _ => 0,
                };
            }
        }
        for ((src, store), (n, rows)) in stores {
            b.conn.execute(
                "INSERT INTO coverage_stores VALUES(?1,?2,?3,?4,?5)",
                params![src, store, n, rows, what(&store)],
            )?;
        }
    }
    Ok(())
}
