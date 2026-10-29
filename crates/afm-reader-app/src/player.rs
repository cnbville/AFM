//! The player page's own blocks (world reader lane R6), on R5's entity
//! template: who he is in plain words, every season with what happened and
//! why, and his story as a dated timeline.
//!
//! A season row (`player::SeasonRecord`) has no year: it is worked out from
//! the birth date and the row's age (the engine's age is his age on 31
//! August as the season starts, `calendar/born.rs`), or from his class in a
//! world without birth dates. The why of each step is the world log's
//! growth row for that season when the book has the log.

use serde_json::{Map, Value, json};

use crate::book::{Book, Filters, json_sql, q};
use crate::pages::{day_text, fmt, num};

/// The bits of a season's `events` (`player::DevelopmentEvent`, in order).
pub const EVENTS: [&str; 17] = [
    "growth spurt",
    "body stalled",
    "breakout",
    "stagnated",
    "lost season",
    "suspended or ineligible",
    "dismissed",
    "slump",
    "minor injury",
    "moderate injury",
    "major injury",
    "first-year dip",
    "spring breakout",
    "lost burst for good",
    "bad weight",
    "complacent after a payday",
    "poor coaching",
];

/// A season's events in words.
pub fn event_words(bits: i64) -> Vec<&'static str> {
    EVENTS.iter().enumerate().filter(|(i, _)| bits & (1 << i) != 0).map(|(_, w)| *w).collect()
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// (year, month, day) of a book day number.
fn ymd(days: i64) -> (i64, i64, i64) {
    let t = day_text(days);
    let p: Vec<i64> = t.split('-').filter_map(|x| x.parse().ok()).collect();
    (p.first().copied().unwrap_or(0), p.get(1).copied().unwrap_or(1), p.get(2).copied().unwrap_or(1))
}

/// The football year a season row played: his birth year plus the row's
/// age, plus one when he was born after 31 August.
fn season_year(born: Option<i64>, age: i64) -> Option<i64> {
    let (y, m, _) = ymd(born.filter(|b| *b > -200_000)?);
    Some(y + age + i64::from(m >= 9))
}

fn s(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(t)) => t.clone(),
        Some(Value::Null) | None => String::new(),
        Some(x) => fmt(x),
    }
}

fn n(v: Option<&Value>) -> Option<f64> {
    v.and_then(num)
}

/// The world's date as a day number and its football year.
fn today(b: &Book) -> (Option<i64>, Option<i64>) {
    let r = b.select_maps("SELECT today, year FROM world LIMIT 1", &[]).unwrap_or_default();
    let r = r.first();
    (r.and_then(|m| m.get("today")).and_then(Value::as_i64), r.and_then(|m| m.get("year")).and_then(Value::as_i64))
}

/// Who he is in plain words: born, age and his class; body; stars; path.
fn plain_words(b: &Book, row: &Map<String, Value>) -> Vec<Value> {
    let mut items = Vec::new();
    let born = row.get("born").and_then(Value::as_i64).filter(|d| *d > -200_000);
    let (day, _) = today(b);
    if let Some(bd) = born {
        let (y, m, d) = ymd(bd);
        let mut t = format!("Born {d} {} {y}", MONTHS[(m - 1).clamp(0, 11) as usize]);
        if let Some(now) = day {
            let (ny, nm, nd) = ymd(now);
            let mut months = (ny - y) * 12 + nm - m - i64::from(nd < d);
            if months < 0 {
                months = 0;
            }
            t.push_str(&format!(", {} years {} months on the world's date", months / 12, months % 12));
        }
        // His class against the class his birth date gives (`born.rs`,
        // `class_of_birth`): one later is held back, one earlier a year young.
        if let Some(c) = row.get("class_of").and_then(Value::as_i64) {
            let natural = if m >= 9 { y + 19 } else { y + 18 };
            t.push_str(&match c - natural {
                0 => format!(", in the class his birthday gives ({c})"),
                1 => format!(", a year old for his class of {c} (held back or started school late)"),
                -1 => format!(", a year young for his class of {c} (skipped a grade or reclassified up)"),
                k => format!(", {} years off his class of {c}", k.abs()),
            });
        }
        t.push('.');
        items.push(json!({ "text": t }));
    }
    let h = n(row.get("player__body__height"));
    let w = n(row.get("player__body__weight"));
    if let (Some(h), Some(w)) = (h, w) {
        let mut t = format!("Grows into {}'{}\" and {} lb", (h / 12.0).floor(), (h % 12.0).round(), w.round());
        if let Some(a) = n(row.get("player__body__arm")) {
            t.push_str(&format!(", {a:.1} in arms"));
        }
        if let Some(x) = n(row.get("player__body__hand")) {
            t.push_str(&format!(", {x:.2} in hands"));
        }
        let lefty = row.get("player__body__left_handed").and_then(Value::as_i64) == Some(1);
        let left_foot = row.get("player__body__left_footed").and_then(Value::as_i64) == Some(1);
        t.push_str(match (lefty, left_foot) {
            (true, true) => "; throws and kicks left",
            (true, false) => "; throws left",
            (false, true) => "; kicks left",
            _ => "",
        });
        t.push('.');
        items.push(json!({ "text": t }));
    }
    let stars = row.get("stars").and_then(Value::as_i64).unwrap_or(0);
    let camp = row.get("camp_stars").and_then(Value::as_i64).unwrap_or(0);
    if stars > 0 || camp > 0 {
        let mut t = if stars > 0 { format!("A {stars}-star recruit") } else { "Unrated by the services".to_string() };
        if camp > 0 {
            t.push_str(&format!(", {camp} camp stars"));
        }
        if row.get("walk_on").and_then(Value::as_i64) == Some(1) {
            t.push_str(", a walk-on");
        }
        if row.get("juco").and_then(Value::as_i64) == Some(1) {
            t.push_str(", came through junior college");
        }
        if let Some(g) = n(row.get("gpa_at_signing")) {
            t.push_str(&format!(", {g:.2} GPA at signing"));
        }
        t.push('.');
        items.push(json!({ "text": t }));
    }
    items
}

/// Each season's growth why from the world log, by football year: the
/// terms the development step drew, biggest first.
fn growth_whys(b: &Book, id: &Value) -> std::collections::HashMap<i64, String> {
    let mut out = std::collections::HashMap::new();
    let Some(t) = b.table("log_growth") else { return out };
    let idc = ["id", "person", "player"].into_iter().find(|c| t.has(c));
    let sql = match idc {
        Some(c) => format!("SELECT season, more('log_growth', _more) AS j FROM log_growth WHERE {} = ?1", q(c)),
        None => return out,
    };
    for r in b.select_maps(&sql, &[json_sql(id)]).unwrap_or_default() {
        let (Some(season), Some(j)) = (r.get("season").and_then(Value::as_i64), r.get("j").and_then(Value::as_str))
        else {
            continue;
        };
        let Ok(v) = serde_json::from_str::<Value>(j) else { continue };
        let text = why_text(&v);
        if !text.is_empty() {
            out.insert(season, text);
        }
    }
    out
}

/// A growth row's terms in words: every number it drew that moved him,
/// biggest first, with the context it had.
pub fn why_text(v: &Value) -> String {
    let mut terms: Vec<(String, f64)> = Vec::new();
    let mut ctx: Vec<String> = Vec::new();
    let skip = ["id", "season", "level", "age", "position", "date", "coach", "coaches", "staff", "program", "team"];
    if let Some(m) = v.get("why").or(Some(v)).and_then(Value::as_object) {
        for (k, x) in m {
            if skip.contains(&k.as_str()) {
                continue;
            }
            match x {
                Value::Number(_) => {
                    let f = x.as_f64().unwrap_or(0.0);
                    if f.abs() >= 0.05 {
                        terms.push((k.replace('_', " "), f));
                    }
                }
                Value::String(t) if t.len() < 40 => ctx.push(format!("{} {t}", k.replace('_', " "))),
                Value::Bool(true) => ctx.push(k.replace('_', " ")),
                _ => {}
            }
        }
    }
    terms.sort_by(|a, b| b.1.abs().total_cmp(&a.1.abs()));
    let mut parts: Vec<String> = terms.iter().take(6).map(|(k, f)| format!("{k} {f:+.1}")).collect();
    parts.extend(ctx.into_iter().take(3));
    parts.join(", ")
}

/// The program, school or club a season was with, by level.
fn team_name(b: &Book, level: &str, idx: Option<i64>) -> String {
    let Some(i) = idx else { return String::new() };
    let (table, name, key) = match level {
        "NFL" => ("nfl", "COALESCE(name, '')", "id"),
        "HS" => ("high_schools", "COALESCE(name, '')", "_row"),
        _ => ("programs", "COALESCE(real_name, fictional_name, '')", "_row"),
    };
    if b.table(table).is_none() {
        return String::new();
    }
    b.select_maps(
        &format!("SELECT {name} AS n FROM {} WHERE {key} = ?1", q(table)),
        &[rusqlite::types::Value::Integer(i)],
    )
    .ok()
    .and_then(|r| r.first().and_then(|m| m.get("n")).and_then(Value::as_str).map(str::to_string))
    .unwrap_or_default()
}

/// The football year of each season row, by its age: from the birth date
/// when the world has one; else from his class, whose senior high school
/// season was the fall before it (`class_of - 1`), which also places a
/// held-back boy right; else his class less 18 plus his age.
fn years(row: &Map<String, Value>, rows: &[Map<String, Value>]) -> impl Fn(i64) -> Option<i64> + use<> {
    let born = row.get("born").and_then(Value::as_i64);
    let class_of = row.get("class_of").and_then(Value::as_i64);
    let last_hs =
        rows.iter().filter(|r| s(r.get("level")) == "HS").filter_map(|r| r.get("age").and_then(Value::as_i64)).max();
    move |age| {
        season_year(born, age).or_else(|| match (class_of, last_hs) {
            (Some(c), Some(h)) => Some(c - 1 + age - h),
            (Some(c), None) => Some(c - 18 + age),
            _ => None,
        })
    }
}

/// His kept stat lines by year: high school lines (`hs_lines`), the
/// college stat book (`stats.seasons`) and his pro seasons.
struct Lines {
    hs: std::collections::HashMap<i64, (i64, Vec<f64>)>,
    college: std::collections::HashMap<i64, (i64, Vec<f64>)>,
    college_programs: Vec<(i64, i64)>,
    pro: std::collections::HashMap<i64, Map<String, Value>>,
}

fn arr(v: Option<&Value>) -> Vec<f64> {
    match v {
        Some(Value::String(t)) => serde_json::from_str::<Vec<f64>>(t).unwrap_or_default(),
        Some(Value::Array(a)) => a.iter().map(|x| x.as_f64().unwrap_or(0.0)).collect(),
        _ => Vec::new(),
    }
}

/// A line's key as a pro season's stat column (`stats__<name>`).
fn pro_col(k: &str) -> &str {
    match k {
        "pyds" => "pass_yds",
        "ptd" => "pass_td",
        "ryds" => "rush_yds",
        "rtd" => "rush_td",
        "cyds" => "rec_yds",
        "ctd" => "rec_td",
        "tkl" => "tackles",
        "fgm" => "fg_made",
        "fga" => "fg_att",
        "pnet" => "punt_net",
        other => other,
    }
}

impl Lines {
    fn of(b: &Book, table: &str, own: &Value, id: &Value) -> Lines {
        let mut l = Lines {
            hs: Default::default(),
            college: Default::default(),
            college_programs: Vec::new(),
            pro: Default::default(),
        };
        let hs = format!("{table}__hs_lines");
        if b.table(&hs).is_some() {
            for r in b
                .select_maps(&format!("SELECT year, games, s FROM {} WHERE _parent = ?1", q(&hs)), &[json_sql(own)])
                .unwrap_or_default()
            {
                if let Some(y) = r.get("year").and_then(Value::as_i64) {
                    l.hs.insert(y, (r.get("games").and_then(Value::as_i64).unwrap_or(0), arr(r.get("s"))));
                }
            }
        }
        if !id.is_null() && b.table("stats__seasons").is_some() {
            for r in b
                .select_maps(
                    "SELECT year, program, games, s FROM stats__seasons WHERE id = ?1 ORDER BY year",
                    &[json_sql(id)],
                )
                .unwrap_or_default()
            {
                if let Some(y) = r.get("year").and_then(Value::as_i64) {
                    if let Some(p) = r.get("program").and_then(Value::as_i64) {
                        l.college_programs.push((y, p));
                    }
                    l.college.insert(y, (r.get("games").and_then(Value::as_i64).unwrap_or(0), arr(r.get("s"))));
                }
            }
        }
        let pro = format!("{table}__pro__seasons");
        if b.table(&pro).is_some() {
            for r in b
                .select_maps(&format!("SELECT * FROM {} WHERE _parent = ?1", q(&pro)), &[json_sql(own)])
                .unwrap_or_default()
            {
                if let Some(y) = r.get("year").and_then(Value::as_i64) {
                    l.pro.insert(y, r);
                }
            }
        }
        l
    }

    /// The season's line in a few numbers that fit the position.
    fn line(&self, level: &str, pos: &str, year: i64) -> String {
        let (games, get): (i64, Box<dyn Fn(&str) -> f64 + '_>) = match level {
            "HS" => {
                let Some((g, a)) = self.hs.get(&year) else { return String::new() };
                let ix = |k: &str| match k {
                    "att" => Some(2),
                    "cmp" => Some(3),
                    "pyds" => Some(4),
                    "ptd" => Some(5),
                    "int" => Some(6),
                    "rush" => Some(7),
                    "ryds" => Some(8),
                    "rtd" => Some(9),
                    "rec" => Some(10),
                    "cyds" => Some(11),
                    "ctd" => Some(12),
                    "tkl" => Some(13),
                    "tfl" => Some(14),
                    "sacks" => Some(15),
                    "ints" => Some(16),
                    "pbus" => Some(17),
                    "fgm" => Some(18),
                    "fga" => Some(19),
                    _ => None,
                };
                // Tackles for loss and sacks are kept in halves.
                (
                    *g,
                    Box::new(move |k| {
                        ix(k).and_then(|i| a.get(i)).copied().unwrap_or(0.0)
                            / if k == "tfl" || k == "sacks" { 2.0 } else { 1.0 }
                    }),
                )
            }
            "NFL" => {
                let Some(m) = self.pro.get(&year) else { return String::new() };
                let g = m.get("games").and_then(Value::as_i64).unwrap_or(0);
                (g, Box::new(move |k| m.get(&format!("stats__{}", pro_col(k))).and_then(num).unwrap_or(0.0)))
            }
            _ => {
                let Some((g, a)) = self.college.get(&year) else { return String::new() };
                let ix = |k: &str| match k {
                    "snaps_off" => Some(0),
                    "snaps_st" => Some(2),
                    "att" => Some(3),
                    "cmp" => Some(4),
                    "pyds" => Some(5),
                    "ptd" => Some(6),
                    "int" => Some(7),
                    "rush" => Some(16),
                    "ryds" => Some(17),
                    "rtd" => Some(18),
                    "rec" => Some(26),
                    "cyds" => Some(27),
                    "ctd" => Some(28),
                    "sacks_allowed" => Some(36),
                    "sacks" => Some(41),
                    "tkl" => Some(43),
                    "tfl" => Some(44),
                    "ints" => Some(51),
                    "pbus" => Some(52),
                    "fga" => Some(53),
                    "fgm" => Some(54),
                    "punts" => Some(58),
                    "pnet" => Some(59),
                    _ => None,
                };
                (*g, Box::new(move |k| ix(k).and_then(|i| a.get(i)).copied().unwrap_or(0.0)))
            }
        };
        let f = |k: &str| get(k);
        let i = |k: &str| format!("{:.0}", get(k));
        let mut parts: Vec<String> = Vec::new();
        match pos {
            "QB" => {
                parts.push(format!(
                    "{}/{} passing, {} yds, {} TD, {} INT",
                    i("cmp"),
                    i("att"),
                    i("pyds"),
                    i("ptd"),
                    i("int")
                ));
                if f("ryds").abs() >= 100.0 {
                    parts.push(format!("{} rush yds", i("ryds")));
                }
            }
            "RB" | "FB" => {
                parts.push(format!("{} rushes, {} yds, {} TD", i("rush"), i("ryds"), i("rtd")));
                if f("rec") > 0.0 {
                    parts.push(format!("{} catches, {} yds", i("rec"), i("cyds")));
                }
            }
            "WR" | "TE" => {
                parts.push(format!("{} catches, {} yds, {} TD", i("rec"), i("cyds"), i("ctd")));
                if f("ryds").abs() >= 50.0 {
                    parts.push(format!("{} rush yds", i("ryds")));
                }
            }
            "OT" | "IOL" | "OL" | "C" | "G" | "T" => {
                if f("snaps_off") > 0.0 {
                    parts.push(format!("{} snaps", i("snaps_off")));
                }
                if level != "HS" {
                    parts.push(format!("{} sacks allowed", i("sacks_allowed")));
                }
            }
            "K" => parts.push(format!("{}/{} field goals", i("fgm"), i("fga"))),
            "P" if f("punts") > 0.0 => parts.push(format!("{} punts, {:.1} net", i("punts"), f("pnet") / f("punts"))),
            "LS" => {
                if f("snaps_st") > 0.0 {
                    parts.push(format!("{} special teams snaps", i("snaps_st")));
                }
            }
            "CB" | "S" => {
                parts.push(format!("{} tackles, {} INT, {} PBU", i("tkl"), i("ints"), i("pbus")));
            }
            _ => {
                parts.push(format!(
                    "{} tackles, {} TFL, {} sacks",
                    i("tkl"),
                    fmt(&json!(f("tfl"))),
                    fmt(&json!(f("sacks")))
                ));
            }
        }
        let parts: Vec<String> = parts.into_iter().filter(|p| !p.starts_with("0 ") || p.contains(',')).collect();
        if games > 0 { format!("{games} g: {}", parts.join("; ")) } else { parts.join("; ") }
    }
}

/// Every season with what happened and why, oldest first.
fn seasons(b: &Book, table: &str, row: &Map<String, Value>, f: &Filters) -> Option<Value> {
    let hist = format!("{table}__player__history");
    b.table(&hist)?;
    let own = row.get("_row")?.clone();
    let id = row.get("player__id").cloned().unwrap_or(Value::Null);
    let whys = if id.is_null() { Default::default() } else { growth_whys(b, &id) };
    // College years: (year, program) from the staff he had each season,
    // then his college stat lines for any year the staff rows miss.
    let mut college: std::collections::HashMap<i64, i64> = Default::default();
    let staff = format!("{table}__college_staff");
    if b.table(&staff).is_some() {
        for r in b
            .select_maps(&format!("SELECT _0 AS y, _1 AS p FROM {} WHERE _parent = ?1", q(&staff)), &[json_sql(&own)])
            .unwrap_or_default()
        {
            if let (Some(y), Some(p)) = (r.get("y").and_then(Value::as_i64), r.get("p").and_then(Value::as_i64)) {
                college.insert(y, p);
            }
        }
    }
    let lines = Lines::of(b, table, &own, &id);
    let hs = row.get("hs").and_then(Value::as_i64);
    for (y, p) in &lines.college_programs {
        college.entry(*y).or_insert(*p);
    }
    let rows =
        b.select_maps(&format!("SELECT * FROM {} WHERE _parent = ?1 ORDER BY _i", q(&hist)), &[json_sql(&own)]).ok()?;
    if rows.is_empty() {
        return None;
    }
    let year_of = years(row, &rows);
    let mut out = Vec::new();
    let mut prev: Option<&Map<String, Value>> = None;
    let mut biggest: Option<(f64, String)> = None;
    for r in &rows {
        let age = r.get("age").and_then(Value::as_i64).unwrap_or(0);
        let year = year_of(age);
        let level = s(r.get("level"));
        let pos = s(r.get("position"));
        let start = n(r.get("overall_start"));
        let end = n(r.get("overall"));
        let change = match (start, end) {
            (Some(a), Some(z)) => Some(z - a),
            _ => None,
        };
        let team = match level.as_str() {
            "NFL" => team_name(b, "NFL", r.get("team").and_then(Value::as_i64)),
            "HS" => team_name(b, "HS", hs),
            _ => team_name(b, &level, year.and_then(|y| college.get(&y).copied())),
        };
        // What happened, in words: events, injuries, a step up, a move.
        let mut what: Vec<String> = event_words(r.get("events").and_then(Value::as_i64).unwrap_or(0))
            .into_iter()
            .filter(|w| *w != "first-year dip")
            .map(str::to_string)
            .collect();
        if let Some(p) = prev {
            if s(p.get("level")) != level {
                what.insert(0, format!("stepped up to {level}"));
            }
            if s(p.get("position")) != pos {
                what.insert(0, format!("moved from {} to {pos}", s(p.get("position"))));
            }
        }
        if let Some(a) = n(r.get("adjustment")).filter(|a| *a >= 0.05) {
            what.push(format!("first-year dip ({:.0}% of a hard one)", a * 100.0));
        }
        if let Some(g) = n(r.get("spring_gain")).filter(|g| g.abs() >= 0.05) {
            what.push(format!("spring {g:+.1}"));
        }
        let missed = r.get("games_missed").and_then(Value::as_i64).unwrap_or(0);
        if missed > 0 {
            what.push(format!("missed {missed} games"));
        }
        let why = year.and_then(|y| whys.get(&y).or_else(|| whys.get(&(y + 1)))).cloned().unwrap_or_default();
        if let Some(c) = change
            && biggest.as_ref().is_none_or(|(x, _)| c.abs() > x.abs())
        {
            biggest = Some((c, format!("{} at {age}", year.map_or(String::new(), |y| y.to_string()))));
        }
        let mut cells = vec![
            json!(year),
            json!(age),
            json!(level),
            json!(team),
            json!(pos),
            json!(s(r.get("playing_time"))),
            json!(s(r.get("depth"))),
            json!(start.map(|x| (x * 10.0).round() / 10.0)),
            json!(end.map(|x| (x * 10.0).round() / 10.0)),
            json!(change.map(|x| format!("{x:+.1}"))),
            json!(n(r.get("performance")).map(|x| (x * 10.0).round() / 10.0)),
            json!(r.get("level_rating").cloned().unwrap_or(Value::Null)),
            json!(n(r.get("coaching")).map(|x| (x * 10.0).round() / 10.0)),
            json!(year.map(|y| lines.line(&level, &pos, y)).unwrap_or_default()),
            json!(what.join(", ")),
        ];
        if f.debug {
            cells.push(json!(n(r.get("potential")).map(|x| (x * 10.0).round() / 10.0)));
            cells.push(json!(why));
        }
        out.push(Value::Array(cells));
        prev = Some(r);
    }
    let mut cols = vec![
        json!({"name": "year", "label": "Season", "num": true}),
        json!({"name": "age", "label": "Age", "num": true}),
        json!({"name": "level", "label": "Level"}),
        json!({"name": "team", "label": "Team"}),
        json!({"name": "pos", "label": "Pos"}),
        json!({"name": "role", "label": "Role"}),
        json!({"name": "depth", "label": "Depth"}),
        json!({"name": "start", "label": "Start", "num": true}),
        json!({"name": "end", "label": "End", "num": true}),
        json!({"name": "change", "label": "+/-", "num": true}),
        json!({"name": "played", "label": "Played at", "num": true}),
        json!({"name": "lr", "label": "Level rating", "num": true}),
        json!({"name": "coaching", "label": "Coaching", "num": true}),
        json!({"name": "line", "label": "Stat line"}),
        json!({"name": "what", "label": "What happened"}),
    ];
    if f.debug {
        cols.push(json!({"name": "potential", "label": "Potential", "num": true, "debug": true}));
        cols.push(json!({"name": "why", "label": "Why (world log)", "debug": true}));
    }
    let mut note = "Start is the rating he played the season at; End is after the season and the offseason. Played at is the season's form (slumps and breakouts in it). The stat line is his kept line for that year: high school lines, the college stat book, his pro seasons.".to_string();
    if let Some((c, at)) = biggest {
        note.push_str(&format!(" Biggest change: {c:+.1} in {at}."));
    }
    if whys.is_empty() && f.debug {
        note.push_str(" Each step's terms need the world log, which this book doesn't have.");
    }
    Some(
        json!({ "kind": "table", "title": "Season by season", "note": note, "cols": cols, "rows": out, "dense": true }),
    )
}

/// His notes as a dated timeline: each line goes under the year its age
/// ("at 17") or its year gives; the rest keep their order.
fn story(row: &Map<String, Value>, year_of: &dyn Fn(i64) -> Option<i64>) -> Option<Value> {
    let Some(Value::String(t)) = row.get("notes") else { return None };
    let lines: Vec<String> = serde_json::from_str(t).ok()?;
    if lines.is_empty() {
        return None;
    }
    let mut last = String::new();
    let items: Vec<Value> = lines
        .iter()
        .map(|l| {
            let year: String = l.chars().take_while(char::is_ascii_digit).collect();
            let when = if year.len() == 4 { Some(year) } else { age_in(l).and_then(year_of).map(|y| y.to_string()) };
            if let Some(w) = when {
                last = w;
            }
            json!({ "when": last.clone(), "text": l })
        })
        .collect();
    Some(
        json!({ "kind": "timeline", "title": "His story", "note": format!("{} lines from his career notes, placed by the age each gives", items.len()), "items": items }),
    )
}

/// The age a note line gives: "at 17", "At 21".
fn age_in(l: &str) -> Option<i64> {
    let w: Vec<&str> = l.split(|c: char| !c.is_ascii_alphanumeric()).collect();
    w.windows(2)
        .find(|p| p[0].eq_ignore_ascii_case("at") && p[1].len() == 2)
        .and_then(|p| p[1].parse().ok())
        .filter(|a| (13..=45).contains(a))
}

/// The blocks a player page adds after the career arc.
pub fn blocks(b: &Book, table: &str, row: &Map<String, Value>, f: &Filters) -> Vec<Value> {
    let mut out = Vec::new();
    let words = plain_words(b, row);
    if !words.is_empty() {
        out.push(json!({ "kind": "list", "title": "In plain words", "items": words, "dense": true }));
    }
    out.extend(seasons(b, table, row, f));
    let hist = format!("{table}__player__history");
    let rows = match (b.table(&hist), row.get("_row")) {
        (Some(_), Some(own)) => b
            .select_maps(&format!("SELECT age, level FROM {} WHERE _parent = ?1", q(&hist)), &[json_sql(own)])
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    out.extend(story(row, &years(row, &rows)));
    out
}
