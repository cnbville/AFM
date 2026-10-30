//! The program, program season, high school and conference pages (world
//! reader lane R7), on R5's entity template: who it is in plain words, its
//! levels, conferences and head coaches over the years as lanes, every
//! season with what happened, its staff, records and news.
//!
//! A program season (`programs__seasons`) keeps its year, record, titles,
//! head coach, strength and the engine's team totals. Its division and
//! conference come from the program's real history (`programs__history`,
//! through 2025) and after it from the world log's yearly program row, else
//! today's. Conference records, expected wins, final ranks and the staff of
//! each year are the world log's (`log_programs`).

use serde_json::{Map, Value, json};

use crate::book::{Book, Filters, json_sql, q};
use crate::pages::{fmt, new_page, push, world_name};

pub fn s(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(t)) => t.clone(),
        Some(Value::Null) | None => String::new(),
        Some(x) => fmt(x),
    }
}

pub fn n(v: Option<&Value>) -> Option<f64> {
    v.and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|t| t.parse().ok())))
}

pub fn i(v: Option<&Value>) -> Option<i64> {
    v.and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|x| x as i64)))
}

/// A table cell that opens a page: its words and the route.
pub fn cell(text: impl Into<String>, link: impl Into<String>) -> Value {
    let (t, l) = (text.into(), link.into());
    if l.is_empty() { json!({ "t": t }) } else { json!({ "t": t, "l": l }) }
}

/// Rows of a query, empty when the book lacks a table it reads.
pub fn rows(b: &Book, sql: &str, vals: &[rusqlite::types::Value]) -> Vec<Map<String, Value>> {
    b.select_maps(sql, vals).unwrap_or_else(|e| {
        // A missing table is an older or smaller world; anything else is worth a line.
        if !e.contains("no such table") {
            eprintln!("reader: {e}: {}", sql.split_whitespace().collect::<Vec<_>>().join(" "));
        }
        Vec::new()
    })
}

pub fn has(b: &Book, t: &str) -> bool {
    b.table(t).is_some()
}

/// A program's name by its index.
pub fn program_name(b: &Book, id: i64) -> String {
    b.name_of("programs", &json!(id)).unwrap_or_else(|| format!("Program {id}"))
}

/// A coach's name by his index.
pub fn coach_name(b: &Book, id: i64) -> String {
    b.name_of("coaches", &json!(id)).unwrap_or_else(|| format!("Coach {id}"))
}

/// Drops the columns of every table block that are empty in all its rows
/// (a column an older or smaller book never filled).
pub fn tidy(blocks: &mut [Value]) {
    for bl in blocks.iter_mut().filter(|x| x["kind"] == "table") {
        let n = bl["cols"].as_array().map_or(0, Vec::len);
        let Some(rows) = bl["rows"].as_array() else { continue };
        if rows.is_empty() {
            continue;
        }
        let blank = |v: Option<&Value>| matches!(v, None | Some(Value::Null)) || v.and_then(Value::as_str) == Some("");
        let gone: Vec<usize> =
            (0..n).filter(|&j| rows.iter().all(|r| blank(r.as_array().and_then(|a| a.get(j))))).collect();
        for &j in gone.iter().rev() {
            if let Some(c) = bl["cols"].as_array_mut() {
                c.remove(j);
            }
            for r in bl["rows"].as_array_mut().into_iter().flatten() {
                if let Some(a) = r.as_array_mut()
                    && j < a.len()
                {
                    a.remove(j);
                }
            }
        }
    }
}

/// A person's stage in words: `HighSchool` → "in high school".
pub fn stage_words(st: &str) -> String {
    match st {
        "HighSchool" => "in high school".into(),
        "College" => "in college".into(),
        "Declared" => "declared for the draft".into(),
        "Pro" => "pro".into(),
        "Done" => "done playing".into(),
        x => x.to_lowercase(),
    }
}

/// A record as words: "11-2".
pub fn record(w: i64, l: i64) -> String {
    format!("{w}-{l}")
}

/// The share of games won, "0.846".
pub fn pct(w: i64, l: i64) -> String {
    if w + l == 0 {
        String::new()
    } else {
        format!("{:.3}", w as f64 / (w + l) as f64).trim_start_matches('0').to_string()
    }
}

/// How a job ended, as a tone for its band: fired, left, retired, current.
pub fn end_tone(ended: &str) -> &'static str {
    let e = ended.to_ascii_lowercase();
    if e.is_empty() || e == "current" {
        "current"
    } else if e.starts_with("fired")
        || e.starts_with("let go")
        || e.starts_with("not retained")
        || e.contains("show-cause")
    {
        "fired"
    } else if e.starts_with("left") {
        "left"
    } else {
        "retired"
    }
}

/// The 12 program grades in the engine's order (`program::GradeCat`).
pub const GRADES: [&str; 12] = [
    "Academics",
    "Pro pipeline",
    "Playing time",
    "Tradition",
    "Title chances",
    "Coach prestige",
    "Facilities",
    "NIL",
    "Brand exposure",
    "Conference strength",
    "Stadium atmosphere",
    "Campus life",
];

/// The eight teaching groups (`coach::Group`).
pub const GROUPS: [&str; 8] = ["QB", "RB", "Receivers", "OL", "DL", "LB", "DB", "Specialists"];

/// A grade as its letter (`program::letter`: 55-99 within the level).
pub fn letter(g: f64) -> &'static str {
    const L: [(f64, &str); 12] = [
        (93.0, "A+"),
        (88.0, "A"),
        (84.0, "A-"),
        (80.0, "B+"),
        (76.0, "B"),
        (72.0, "B-"),
        (68.0, "C+"),
        (64.0, "C"),
        (60.0, "C-"),
        (57.0, "D+"),
        (55.0, "D"),
        (0.0, "D-"),
    ];
    L.iter().find(|(x, _)| g >= *x).map_or("D-", |(_, l)| l)
}

/// Each season's level and conference: (year, division, conference),
/// from the real history, the world log's rows, else today's.
/// The last season of the real college history (`data::HISTORY_LAST`).
const REAL_LAST: i64 = 2025;

fn levels(b: &Book, own: &Value, row: &Map<String, Value>) -> Vec<(i64, String, String, bool)> {
    let mut out: Vec<(i64, String, String, bool)> = Vec::new();
    if has(b, "programs__history") {
        for r in rows(
            b,
            "SELECT _0 AS a, _1 AS z, _2 AS d, _3 AS c FROM programs__history WHERE _parent = ?1 ORDER BY _0",
            &[json_sql(own)],
        ) {
            let (Some(a), Some(z)) = (i(r.get("a")), i(r.get("z"))) else { continue };
            for y in a..=z.min(a + 200) {
                out.push((y, s(r.get("d")), s(r.get("c")), false));
            }
        }
    }
    // The real history runs through 2025 and keeps only the years a
    // program differed from today (`Program::history`); a season before
    // then with no row was today's level and conference.
    let last_real = out.iter().map(|x| x.0).max().unwrap_or(0).max(REAL_LAST);
    let id = row.get("id").cloned().unwrap_or(Value::Null);
    let mut logged: std::collections::HashMap<i64, (String, String)> = Default::default();
    if has(b, "log_programs") {
        for r in rows(
            b,
            "SELECT season, program__division AS d, program__conference AS c FROM log_programs WHERE id = ?1",
            &[json_sql(&id)],
        ) {
            if let Some(y) = i(r.get("season")) {
                logged.insert(y, (s(r.get("d")), s(r.get("c"))));
            }
        }
    }
    let seasons = rows(b, "SELECT year FROM programs__seasons WHERE _parent = ?1 ORDER BY year", &[json_sql(own)]);
    let (dn, cn) = (s(row.get("division")), s(row.get("conference")));
    for r in seasons {
        let Some(y) = i(r.get("year")) else { continue };
        if y <= last_real {
            out.push((y, dn.clone(), cn.clone(), false));
            continue;
        }
        let (d, c) = logged.get(&y).cloned().unwrap_or_else(|| (dn.clone(), cn.clone()));
        out.push((y, d, c, true));
    }
    out.sort_by_key(|x| x.0);
    out.dedup_by_key(|x| x.0);
    out
}

/// Runs of equal values as bands: (from, to, value, world years).
fn runs<T: PartialEq + Clone>(xs: &[(i64, T, bool)]) -> Vec<(i64, i64, T, bool)> {
    let mut out: Vec<(i64, i64, T, bool)> = Vec::new();
    for (y, v, w) in xs {
        match out.last_mut() {
            Some(l) if l.2 == *v && l.1 + 1 == *y && l.3 == *w => l.1 = *y,
            _ => out.push((*y, *y, v.clone(), *w)),
        }
    }
    out
}

/// A head coach's run at a program: (coach, from, to, wins, losses,
/// conference titles, national titles).
type Run = (Option<i64>, i64, i64, i64, i64, i64, i64);

/// The program's head coaches by season, as runs.
fn head_coaches(b: &Book, own: &Value) -> Vec<Run> {
    let mut out: Vec<Run> = Vec::new();
    for r in rows(
        b,
        "SELECT year, coach, wins, losses, conference_title AS ct, national_title AS nt FROM programs__seasons
          WHERE _parent = ?1 ORDER BY year",
        &[json_sql(own)],
    ) {
        let y = i(r.get("year")).unwrap_or(0);
        let c = i(r.get("coach"));
        let (w, l) = (i(r.get("wins")).unwrap_or(0), i(r.get("losses")).unwrap_or(0));
        let (ct, nt) = (i(r.get("ct")).unwrap_or(0), i(r.get("nt")).unwrap_or(0));
        match out.last_mut() {
            Some(x) if x.0 == c && x.2 + 1 == y => {
                x.2 = y;
                x.3 += w;
                x.4 += l;
                x.5 += ct;
                x.6 += nt;
            }
            _ => out.push((c, y, y, w, l, ct, nt)),
        }
    }
    out
}

/// How a coach's job at an employer ended (his stint's words), by the
/// year it ended: `employer__College = id`.
fn stint_end(b: &Book, coach: i64, kind: &str, emp: i64, to: i64) -> Option<String> {
    if !has(b, "coaches__history") {
        return None;
    }
    let col = format!("employer__{kind}");
    if !b.table("coaches__history").is_some_and(|t| t.has(&col)) {
        return None;
    }
    let sql = format!(
        "SELECT h.ended FROM coaches__history h JOIN coaches c ON c._row = h._parent
          WHERE c.id = ?1 AND h.{} = ?2 AND h.role = 'HeadCoach' AND h.\"to\" BETWEEN ?3 - 1 AND ?3 + 1
          ORDER BY ABS(h.\"to\" - ?3) LIMIT 1",
        q(&col)
    );
    rows(b, &sql, &[coach.into(), emp.into(), to.into()]).first().map(|r| s(r.get("ended"))).filter(|e| !e.is_empty())
}

/// This year's news about a program, school or coach, by `news__refs`.
pub fn news_of(b: &Book, link: &str, id: i64, year: Option<i64>, limit: usize) -> Vec<Map<String, Value>> {
    let col = format!("to__{link}");
    let refs = b.table("news__refs").is_some_and(|t| t.has(&col));
    let prog = link == "Program" && b.table("news").is_some_and(|t| t.has("program"));
    if !refs && !prog {
        return Vec::new();
    }
    let mut cond: Vec<String> = Vec::new();
    if refs {
        cond.push(format!("n._row IN (SELECT _parent FROM news__refs WHERE {} = ?1)", q(&col)));
    }
    if prog {
        cond.push("n.program = ?1".into());
    }
    let yr = if year.is_some() { " AND n.year = ?2" } else { "" };
    let sql = format!(
        "SELECT n.year, n.date, n.kind, n.text FROM news n WHERE ({}){yr} ORDER BY n.year DESC, n.date DESC LIMIT {limit}",
        cond.join(" OR ")
    );
    let mut vals: Vec<rusqlite::types::Value> = vec![id.into()];
    if let Some(y) = year {
        vals.push(y.into());
    }
    rows(b, &sql, &vals)
}

/// A news kind in words: `CoachHired` → "coach hired".
pub fn kind_words(k: &str) -> String {
    let mut out = String::new();
    for (j, ch) in k.chars().enumerate() {
        if ch.is_ascii_uppercase() && j > 0 {
            out.push(' ');
        }
        out.push(ch.to_ascii_lowercase());
    }
    out
}

/// Who a program is, in plain words.
fn program_words(b: &Book, row: &Map<String, Value>, lv: &[(i64, String, String, bool)]) -> Vec<Value> {
    let mut items = Vec::new();
    let real = s(row.get("real_name"));
    let made = s(row.get("fictional_name"));
    let name = if real.is_empty() { made.clone() } else { real.clone() };
    let state = s(row.get("state"));
    let state_code = i(row.get("state"))
        .and_then(|k| {
            rows(b, "SELECT code FROM state_codes WHERE idx = ?1", &[k.into()]).first().map(|r| s(r.get("code")))
        })
        .unwrap_or(state);
    let mut t = format!(
        "{name} plays {} football in the {}, in {}, {state_code}.",
        s(row.get("division")),
        s(row.get("conference")),
        s(row.get("city"))
    );
    if !real.is_empty() && !made.is_empty() && made != real {
        t.push_str(&format!(" Its made-up name is {made}."));
    }
    items.push(json!({ "text": t }));
    let mut facts: Vec<String> = Vec::new();
    if let Some(e) = n(row.get("enrollment")) {
        facts.push(format!("{} students", fmt(&json!(e))));
    }
    let tier = s(row.get("academic_tier"));
    if !tier.is_empty() {
        facts.push(format!("academic tier {tier}"));
    }
    if let Some(x) = n(row.get("stadium")) {
        facts.push(format!("a {}-seat stadium", fmt(&json!(x))));
    }
    if !facts.is_empty() {
        items.push(json!({ "text": format!("{}.", capital(&facts.join(", "))) }));
    }
    if let (Some(p), Some(p0)) = (n(row.get("prestige")), n(row.get("prestige_start"))) {
        let div = s(row.get("division"));
        let rank = rows(
            b,
            "SELECT COUNT(*) + 1 AS r, (SELECT COUNT(*) FROM programs WHERE division = ?2) AS n FROM programs WHERE division = ?2 AND prestige > ?1",
            &[p.into(), div.clone().into()],
        );
        let r = rank.first();
        items.push(json!({ "text": format!(
            "Prestige {p:.2} now ({} of {} in {div}), {p0:.2} when the history began: {}.",
            ordinal(i(r.and_then(|r| r.get("r"))).unwrap_or(0)),
            i(r.and_then(|r| r.get("n"))).unwrap_or(0),
            match p - p0 { d if d > 0.08 => "risen a lot", d if d > 0.02 => "risen", d if d < -0.08 => "fallen a lot", d if d < -0.02 => "fallen", _ => "about the same" }
        ) }));
    }
    let (titles, conf) = (i(row.get("titles")).unwrap_or(0), i(row.get("conference_titles")).unwrap_or(0));
    items.push(json!({ "text": format!("{titles} national title{} and {conf} conference title{} in the world's seasons.", if titles == 1 { "" } else { "s" }, if conf == 1 { "" } else { "s" }) }));
    // Moves between levels.
    let mut moves: Vec<String> = Vec::new();
    for w in lv.windows(2) {
        if w[0].1 != w[1].1 {
            let to = if w[1].1.is_empty() { "no team".to_string() } else { w[1].1.clone() };
            let from = if w[0].1.is_empty() { "no team".to_string() } else { w[0].1.clone() };
            moves.push(format!("{} {from} to {to}", w[1].0));
        }
    }
    if !moves.is_empty() {
        items.push(json!({ "text": format!("Level moves: {}.", moves.join("; ")) }));
    }
    if let Some(c) = i(row.get("head_coach")) {
        let since = i(row.get("hc_changed"));
        items.push(json!({ "text": format!("Head coach {}{}.", coach_name(b, c), since.map_or(String::new(), |y| format!(" since {y}"))),
                           "link": format!("e/coaches/{c}") }));
    }
    let ad = s(row.get("ad__name"));
    if !ad.is_empty() {
        items.push(json!({ "text": format!("Athletic director {ad}{}.", i(row.get("ad__since")).map_or(String::new(), |y| format!(" since {y}"))) }));
    }
    items
}

pub fn capital(t: &str) -> String {
    let mut c = t.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

pub fn ordinal(k: i64) -> String {
    let suf = match (k % 10, k % 100) {
        (1, x) if x != 11 => "st",
        (2, x) if x != 12 => "nd",
        (3, x) if x != 13 => "rd",
        _ => "th",
    };
    format!("{k}{suf}")
}

/// The lanes: level, conference and head coach by season, the story's
/// moves as pins.
fn program_lanes(b: &Book, row: &Map<String, Value>, own: &Value, lv: &[(i64, String, String, bool)]) -> Value {
    let id = i(row.get("id")).unwrap_or(0);
    let lvl: Vec<(i64, String, bool)> = lv.iter().map(|x| (x.0, x.1.clone(), x.3)).collect();
    let conf: Vec<(i64, (String, String), bool)> = lv.iter().map(|x| (x.0, (x.2.clone(), x.1.clone()), x.3)).collect();
    let band = |a: i64, z: i64, text: String, tone: &str, world: bool| {
        json!({ "from": a, "to": z, "text": text, "tone": if world { format!("{tone} t-world") } else { tone.to_string() },
                "sub": if world { "the world's years" } else { "" } })
    };
    let level_items: Vec<Value> = runs(&lvl)
        .into_iter()
        .map(|(a, z, d, w)| {
            let (t, tone) =
                if d.is_empty() { ("No team".to_string(), "dark".to_string()) } else { (d.clone(), d.clone()) };
            band(a, z, t, &tone, w)
        })
        .collect();
    let conf_items: Vec<Value> = runs(&conf)
        .into_iter()
        .map(|(a, z, (c, d), w)| {
            let tone = if d.is_empty() { "dark".to_string() } else { d };
            let t = if c.is_empty() { "No team".to_string() } else { c.clone() };
            let mut x = band(a, z, t, &tone, w);
            if !c.is_empty() {
                x["link"] = json!(format!("conference/{}", urlish(&c)));
            }
            x
        })
        .collect();
    let coach_items: Vec<Value> = head_coaches(b, own)
        .into_iter()
        .map(|(c, a, z, w, l, ct, nt)| {
            let name = c.map_or("No head coach".to_string(), |c| coach_name(b, c));
            let ended = c.and_then(|c| stint_end(b, c, "College", id, z)).unwrap_or_default();
            let mut sub = record(w, l);
            if nt > 0 {
                sub.push_str(&format!(", {nt} national"));
            }
            if ct > 0 {
                sub.push_str(&format!(", {ct} conference"));
            }
            if !ended.is_empty() {
                sub.push_str(&format!("; {ended}"));
            }
            json!({ "from": a, "to": z, "text": name, "sub": sub, "tone": end_tone(&ended),
                    "link": c.map(|c| format!("e/coaches/{c}")) })
        })
        .collect();
    let marks: Vec<Value> = if has(b, "programs__story") {
        rows(b, "SELECT _0 AS y, _1 AS t FROM programs__story WHERE _parent = ?1 ORDER BY _0", &[json_sql(own)])
            .iter()
            .map(|r| json!({ "year": i(r.get("y")), "text": s(r.get("t")) }))
            .collect()
    } else {
        Vec::new()
    };
    let first =
        [level_items.first(), coach_items.first()].into_iter().flatten().filter_map(|x| x["from"].as_i64()).min();
    json!({
        "kind": "bands", "title": "Through the years",
        "note": "Its level and conference each season (real history through 2025, hatched: the world's own years after), its head coaches with their record and how each job ended, and the story's moves as pins",
        "x0": first, "lanes": [
            { "label": "Level", "items": level_items },
            { "label": "Conference", "items": conf_items },
            { "label": "Head coach", "items": coach_items },
        ],
        "marks": marks,
        "keys": [
            {"text": "Level:"}, {"tone": "FBS", "text": "FBS"}, {"tone": "FCS", "text": "FCS"}, {"tone": "D2", "text": "D2"},
            {"tone": "D3", "text": "D3"}, {"tone": "NAIA", "text": "NAIA"}, {"tone": "JUCO", "text": "JUCO"}, {"tone": "dark", "text": "No team"},
            {"text": "Head coach:"}, {"tone": "current", "text": "Still there"}, {"tone": "left", "text": "Left for a job"},
            {"tone": "retired", "text": "Retired or stepped away"}, {"tone": "fired", "text": "Fired"},
        ],
    })
}

/// A conference name as a route part.
pub fn urlish(c: &str) -> String {
    c.replace('/', "%2F").replace(' ', "%20")
}

/// Every season of a program with what happened.
fn program_seasons(
    b: &Book,
    row: &Map<String, Value>,
    own: &Value,
    lv: &[(i64, String, String, bool)],
) -> Option<Value> {
    let id = i(row.get("id")).unwrap_or(0);
    let seasons = rows(b, "SELECT * FROM programs__seasons WHERE _parent = ?1 ORDER BY year", &[json_sql(own)]);
    if seasons.is_empty() {
        return None;
    }
    let prestige: std::collections::HashMap<i64, f64> = if has(b, "programs__prestige_history") {
        rows(b, "SELECT _0 AS y, _1 AS p FROM programs__prestige_history WHERE _parent = ?1", &[json_sql(own)])
            .iter()
            .filter_map(|r| Some((i(r.get("y"))?, n(r.get("p"))?)))
            .collect()
    } else {
        Default::default()
    };
    // The world log's row for each season: conference record, expected
    // wins, preseason strength, incidents, roster size.
    let mut logged: std::collections::HashMap<i64, Map<String, Value>> = Default::default();
    if has(b, "log_programs") {
        for r in rows(
            b,
            "SELECT season, json_extract(more('log_programs', _more), '$.result') AS r FROM log_programs WHERE id = ?1",
            &[json_sql(&json!(id))],
        ) {
            if let (Some(y), Some(Value::String(t))) = (i(r.get("season")), r.get("r"))
                && let Ok(Value::Object(m)) = serde_json::from_str::<Value>(t)
            {
                logged.insert(y, m);
            }
        }
    }
    let mut news: std::collections::HashMap<i64, Vec<String>> = Default::default();
    for r in news_of(b, "Program", id, None, 2000) {
        if let Some(y) = i(r.get("year")) {
            let k = s(r.get("kind"));
            if [
                "CoachHired",
                "CoachFired",
                "CoachLeft",
                "NationalTitle",
                "ConferenceTitle",
                "NcaaPenalty",
                "Realignment",
            ]
            .iter()
            .any(|x| k.starts_with(x))
            {
                news.entry(y).or_default().push(s(r.get("text")));
            }
        }
    }
    let conf_of: std::collections::HashMap<i64, String> = lv.iter().map(|x| (x.0, x.2.clone())).collect();
    let mut out = Vec::new();
    for r in &seasons {
        let y = i(r.get("year")).unwrap_or(0);
        let (w, l) = (i(r.get("wins")).unwrap_or(0), i(r.get("losses")).unwrap_or(0));
        let g = n(r.get("totals__games")).filter(|g| *g > 0.0);
        let per = |c: &str| g.and_then(|g| n(r.get(c)).map(|x| format!("{:.1}", x / g)));
        let coach = i(r.get("coach"));
        let lg = logged.get(&y).or_else(|| logged.get(&(y + 1)));
        let conf_rec = lg.and_then(|m| Some(record(i(m.get("conf_wins"))?, i(m.get("conf_losses"))?)));
        let expected = lg.and_then(|m| n(m.get("expected")));
        let mut titles = Vec::new();
        if i(r.get("national_title")) == Some(1) {
            titles.push("national");
        }
        if i(r.get("conference_title")) == Some(1) {
            titles.push("conference");
        }
        let what = news.get(&y).map(|v| v.join(" ")).unwrap_or_default();
        out.push(json!([
            cell(y.to_string(), format!("season/{id}/{y}")),
            conf_of.get(&y).cloned().unwrap_or_default(),
            record(w, l),
            pct(w, l),
            conf_rec,
            expected.map(|x| format!("{x:.1}")),
            coach.map_or(json!(""), |c| cell(coach_name(b, c), format!("e/coaches/{c}"))),
            n(r.get("strength")).map(|x| (x * 10.0).round() / 10.0),
            prestige.get(&y).map(|x| (x * 1000.0).round() / 1000.0),
            per("totals__points_for"),
            per("totals__points_against"),
            titles.join(", "),
            what,
        ]));
    }
    let mut cols = json!([
        {"name": "year", "label": "Season"}, {"name": "conference", "label": "Conference"},
        {"name": "record", "label": "W-L"}, {"name": "pct", "label": "Pct", "num": true},
        {"name": "conf", "label": "Conf"}, {"name": "expected", "label": "Expected wins", "num": true},
        {"name": "coach", "label": "Head coach"}, {"name": "strength", "label": "Strength", "num": true},
        {"name": "prestige", "label": "Prestige after", "num": true},
        {"name": "pf", "label": "Points for a game", "num": true}, {"name": "pa", "label": "Against", "num": true},
        {"name": "titles", "label": "Titles"}, {"name": "what", "label": "What happened (news)", "minw": 320},
    ]);
    // Without the world log the conference record and expected wins are
    // empty in every row: leave the two columns out.
    if logged.is_empty() {
        for row in out.iter_mut().chain(std::iter::once(&mut cols)) {
            if let Some(a) = row.as_array_mut() {
                a.drain(4..6);
            }
        }
    }
    let note = if logged.is_empty() {
        "Each season's record, coach, strength and engine totals from the world file. Conference records and expected wins need the world log, which this book doesn't have. A season opens its own page."
    } else {
        "Each season's record, coach, strength and engine totals; conference record and expected wins from the world log's yearly row. A season opens its own page."
    };
    Some(
        json!({ "kind": "table", "title": "Season by season", "note": note, "cols": cols, "rows": out, "dense": true }),
    )
}

/// Its head coaches as a table: years, record, titles, how it ended.
fn coach_table(b: &Book, row: &Map<String, Value>, own: &Value) -> Option<Value> {
    let id = i(row.get("id")).unwrap_or(0);
    let hc = head_coaches(b, own);
    if hc.is_empty() {
        return None;
    }
    let out: Vec<Value> = hc
        .iter()
        .rev()
        .map(|(c, a, z, w, l, ct, nt)| {
            let ended = c.and_then(|c| stint_end(b, c, "College", id, *z)).unwrap_or_default();
            json!([
                c.map_or(json!("No head coach"), |c| cell(coach_name(b, c), format!("e/coaches/{c}"))),
                if a == z { a.to_string() } else { format!("{a}-{z}") },
                z - a + 1,
                record(*w, *l),
                pct(*w, *l),
                ct,
                nt,
                ended,
            ])
        })
        .collect();
    Some(json!({
        "kind": "table", "title": "Head coaches", "note": "Each head coach's run here from the season rows, newest first; how it ended from his own job record",
        "cols": [{"name": "coach", "label": "Coach"}, {"name": "years", "label": "Years"}, {"name": "seasons", "label": "Seasons", "num": true},
                 {"name": "record", "label": "W-L"}, {"name": "pct", "label": "Pct", "num": true},
                 {"name": "ct", "label": "Conf titles", "num": true}, {"name": "nt", "label": "National", "num": true},
                 {"name": "ended", "label": "How it ended"}],
        "rows": out, "dense": true
    }))
}

/// Its grades each year as a heat grid (a panel the charts draw).
fn grades_block(b: &Book, own: &Value) -> Option<Value> {
    if !has(b, "programs__grade_history") {
        return None;
    }
    let mut data = Vec::new();
    for r in rows(
        b,
        "SELECT _0 AS y, _1 AS g FROM programs__grade_history WHERE _parent = ?1 ORDER BY _0 DESC LIMIT 40",
        &[json_sql(own)],
    ) {
        let y = i(r.get("y")).unwrap_or(0);
        let gs: Vec<f64> = match r.get("g") {
            Some(Value::String(t)) => serde_json::from_str(t).unwrap_or_default(),
            _ => Vec::new(),
        };
        for (k, g) in gs.iter().enumerate() {
            data.push(json!([y.to_string(), GRADES.get(k).copied().unwrap_or("?"), (g * 10.0).round() / 10.0]));
        }
    }
    if data.is_empty() {
        return None;
    }
    Some(json!({
        "kind": "panel", "pkind": "matrix", "id": "program-grades", "title": "Its grades each year",
        "note": "The 12 grades a recruit reads, 55-99 within its level (A+ 93 and up, A 88, B 76, C 64, D 55); newest first",
        "cols": ["row", "col", "value"], "rows": data, "status": "ok",
    }))
}

/// Who holds each seat now, the athletic director and the support heads.
fn staff_block(b: &Book, row: &Map<String, Value>) -> Option<Value> {
    let mut pairs: Vec<Value> = Vec::new();
    let mut seat = |k: &str, c: Option<i64>| {
        if let Some(c) = c {
            pairs.push(json!({ "k": k, "v": coach_name(b, c), "link": format!("e/coaches/{c}") }));
        }
    };
    seat("Head coach", i(row.get("head_coach")));
    seat("On the sideline now", i(row.get("sideline")));
    seat("Offensive coordinator", i(row.get("oc")));
    seat("Defensive coordinator", i(row.get("dc")));
    if let Some(Value::String(t)) = row.get("position_coaches")
        && let Ok(xs) = serde_json::from_str::<Vec<Option<i64>>>(t)
    {
        for (k, c) in xs.iter().enumerate() {
            seat(&format!("{} coach", GROUPS.get(k).copied().unwrap_or("Position")), *c);
        }
    }
    let ad = s(row.get("ad__name"));
    if !ad.is_empty() {
        let mut v = ad;
        if let Some(y) = i(row.get("ad__since")) {
            v.push_str(&format!(" (since {y})"));
        }
        pairs.push(json!({ "k": "Athletic director", "v": v }));
        for (k, c) in [("AD's judgement", "ad__judgement"), ("AD's patience", "ad__patience")] {
            if let Some(x) = n(row.get(c)) {
                pairs.push(json!({ "k": k, "v": format!("{x:.1}"), "debug": true }));
            }
        }
    }
    for (k, c) in [
        ("Strength and conditioning", "support__strength"),
        ("Medical", "support__medical"),
        ("Analytics", "support__analytics"),
        ("Academic support", "support__academic"),
        ("Player engagement", "support__engagement"),
    ] {
        if let Some(x) = n(row.get(&format!("{c}__rating"))) {
            let name = s(row.get(&format!("{c}__name")));
            let since = i(row.get(&format!("{c}__since")));
            pairs.push(json!({ "k": k, "v": format!("{x:.1}{}{}", if name.is_empty() { String::new() } else { format!(" · {name}") }, since.map_or(String::new(), |y| format!(" since {y}"))) }));
        }
    }
    if pairs.is_empty() {
        return None;
    }
    Some(
        json!({ "kind": "kv", "title": "Its staff now", "note": "Every seat, the AD who judges the head coach (his ratings hidden, 1-20), and the support heads (1-20)", "pairs": pairs }),
    )
}

/// Draft picks by year and the school's season records.
fn production(b: &Book, row: &Map<String, Value>, own: &Value) -> Vec<Value> {
    let mut out = Vec::new();
    if has(b, "programs__draft_picks") {
        // One row per pick, (draft year, round): counted by draft.
        let picks: Vec<Value> = rows(
            b,
            "SELECT y, COUNT(*) AS n, SUM(r = 1) AS r1, group_concat(r, ', ') AS rounds
               FROM (SELECT _0 AS y, _1 AS r FROM programs__draft_picks WHERE _parent = ?1 ORDER BY _0, _1) GROUP BY y ORDER BY y DESC",
            &[json_sql(own)],
        )
        .iter()
        .map(|r| json!([i(r.get("y")), i(r.get("n")), i(r.get("r1")), s(r.get("rounds"))]))
        .collect();
        if !picks.is_empty() {
            let total: i64 = picks.iter().filter_map(|p| p[1].as_i64()).sum();
            let first: i64 = picks.iter().filter_map(|p| p[2].as_i64()).sum();
            out.push(json!({
                "kind": "table", "title": format!("Draft picks by year ({total} in all, {first} in round 1)"),
                "cols": [{"name": "year", "label": "Draft"}, {"name": "n", "label": "Picks", "num": true},
                         {"name": "r1", "label": "Round 1", "num": true}, {"name": "rounds", "label": "Rounds"}],
                "rows": picks, "dense": true
            }));
        }
    }
    let _ = row;
    if has(b, "programs__season_records") {
        let recs = rows(
            b,
            "SELECT _i, value, holder, year FROM programs__season_records WHERE _parent = ?1 ORDER BY _i",
            &[json_sql(own)],
        );
        let what = ["Passing yards", "Rushing yards", "Receiving yards", "Sacks"];
        let pairs: Vec<Value> = recs
            .iter()
            .map(|r| {
                let k = i(r.get("_i")).unwrap_or(0) as usize;
                let holder = i(r.get("holder"));
                json!({ "k": what.get(k).copied().unwrap_or("Record"),
                        "v": format!("{} by {} in {}", s(r.get("value")), holder.map_or("?".into(), |h| person_name(b, h)), s(r.get("year"))),
                        "link": holder.map(|h| format!("e/people/{h}")) })
            })
            .collect();
        if !pairs.is_empty() {
            out.push(json!({ "kind": "kv", "title": "School season records", "pairs": pairs }));
        }
    }
    out
}

/// A person's name by his player id, from the people or the alumni kept.
pub fn person_name(b: &Book, id: i64) -> String {
    for t in ["people", "alumni"] {
        let Some(tb) = b.table(t) else { continue };
        let idc = crate::book::id_of(tb);
        if let Some(r) = rows(b, &format!("SELECT name FROM {t} WHERE {} = ?1", q(idc)), &[id.into()]).first() {
            return s(r.get("name"));
        }
    }
    format!("Player {id}")
}

/// Its news as a dated list, newest first.
fn news_block(b: &Book, link: &str, id: i64) -> Option<Value> {
    let items: Vec<Value> = news_of(b, link, id, None, 80)
        .iter()
        .map(|r| {
            let when = match i(r.get("date")) {
                Some(d) if d.abs() < 200_000 => crate::pages::day_text(d),
                _ => s(r.get("year")),
            };
            json!({ "when": when, "text": s(r.get("text")), "why": kind_words(&s(r.get("kind"))) })
        })
        .collect();
    if items.is_empty() {
        return None;
    }
    Some(
        json!({ "kind": "timeline", "title": "In the news", "note": format!("The latest {} items that name it", items.len()), "items": items }),
    )
}

/// The blocks a program page adds after the header.
pub fn program_blocks(b: &Book, row: &Map<String, Value>, f: &Filters) -> Vec<Value> {
    let _ = f;
    let own = row.get("_row").cloned().unwrap_or(Value::Null);
    let lv = levels(b, &own, row);
    let mut out =
        vec![json!({ "kind": "list", "title": "In plain words", "items": program_words(b, row, &lv), "dense": true })];
    out.push(program_lanes(b, row, &own, &lv));
    out.extend(program_seasons(b, row, &own, &lv));
    out.extend(coach_table(b, row, &own));
    out.extend(staff_block(b, row));
    out.extend(grades_block(b, &own));
    out.extend(production(b, row, &own));
    out.extend(news_block(b, "Program", i(row.get("id")).unwrap_or(0)));
    tidy(&mut out);
    out
}

/// One program season: its record and totals, the world log's row (staff,
/// conference record, expected wins), its games and its roster.
pub fn season_page(b: &mut Book, prog: &str, year: i64, f: &Filters) -> Result<Value, String> {
    let _ = f;
    let pid: i64 = prog.parse().map_err(|_| format!("No program {prog}."))?;
    let prow = rows(b, "SELECT * FROM programs WHERE id = ?1", &[pid.into()])
        .pop()
        .ok_or_else(|| format!("No program {prog}."))?;
    let name = program_name(b, pid);
    let own = prow.get("_row").cloned().unwrap_or(Value::Null);
    let title = format!("{name} {year}");
    let mut p = new_page(&title, &format!("{} · Programs · {name} · {year}", world_name(b)));
    let srow =
        rows(b, "SELECT * FROM programs__seasons WHERE _parent = ?1 AND year = ?2", &[json_sql(&own), year.into()])
            .pop();
    let lv = levels(b, &own, &prow);
    let (div, conf) = lv
        .iter()
        .find(|x| x.0 == year)
        .map_or((s(prow.get("division")), s(prow.get("conference"))), |x| (x.1.clone(), x.2.clone()));
    let mut tags = vec![div.clone(), conf.clone()];
    tags.retain(|t| !t.is_empty());
    let mut figures = Vec::new();
    let mut verdict = format!("{name} in {year}.");
    if let Some(r) = &srow {
        let (w, l) = (i(r.get("wins")).unwrap_or(0), i(r.get("losses")).unwrap_or(0));
        figures.push(json!({ "label": "Record", "value": record(w, l) }));
        if let Some(x) = n(r.get("strength")) {
            figures.push(json!({ "label": "Strength", "value": (x * 10.0).round() / 10.0 }));
        }
        let mut t = Vec::new();
        if i(r.get("national_title")) == Some(1) {
            t.push("national champions");
        }
        if i(r.get("conference_title")) == Some(1) {
            t.push("conference champions");
        }
        let coach = i(r.get("coach"));
        verdict = format!(
            "{name} went {} in {year}{}{}.",
            record(w, l),
            if t.is_empty() { String::new() } else { format!(", {}", t.join(" and ")) },
            coach.map_or(String::new(), |c| format!(" under {}", coach_name(b, c)))
        );
    }
    p["verdict"] = json!(verdict);
    push(
        &mut p,
        json!({ "kind": "hero", "label": "Program season", "title": title, "tags": tags, "figures": figures,
                          "actions": [{"text": format!("{name}'s page"), "link": format!("e/programs/{pid}")}] }),
    );
    if let Some(r) = &srow {
        let mut pairs = Vec::new();
        if let Some(c) = i(r.get("coach")) {
            pairs.push(json!({ "k": "Head coach", "v": coach_name(b, c), "link": format!("e/coaches/{c}") }));
        }
        for (k, v) in r {
            if let Some(stat) = k.strip_prefix("totals__") {
                pairs.push(json!({ "k": crate::book::label(stat), "v": fmt(v) }));
            }
        }
        push(
            &mut p,
            json!({ "kind": "kv", "title": "The season's numbers", "note": "The season row: the engine's team totals for its games", "pairs": pairs }),
        );
    }
    // The world log's yearly row for this season.
    if has(b, "log_programs") {
        let logged = rows(
            b,
            "SELECT season, more('log_programs', _more) AS m FROM log_programs WHERE id = ?1 AND season IN (?2, ?2 + 1) ORDER BY season",
            &[pid.into(), year.into()],
        );
        if let Some(Value::String(t)) = logged.first().and_then(|r| r.get("m"))
            && let Ok(m) = serde_json::from_str::<Value>(t)
        {
            let r = &m["result"];
            let mut pairs = Vec::new();
            for (k, label) in [
                ("conf_wins", "Conference wins"),
                ("conf_losses", "Conference losses"),
                ("expected", "Expected wins"),
                ("preseason", "Preseason strength"),
                ("incidents", "Off-field incidents"),
                ("roster", "Roster"),
            ] {
                if !r[k].is_null() {
                    pairs.push(json!({ "k": label, "v": fmt(&r[k]) }));
                }
            }
            let pr = &m["program"];
            let mut seat = |k: &str, v: &Value| {
                if let Some(c) = v.as_i64() {
                    pairs.push(json!({ "k": k, "v": coach_name(b, c), "link": format!("e/coaches/{c}") }));
                }
            };
            seat("Head coach (log)", &pr["head_coach"]);
            seat("Offensive coordinator", &pr["oc"]);
            seat("Defensive coordinator", &pr["dc"]);
            for (k, c) in pr["position_coaches"].as_array().into_iter().flatten().enumerate() {
                seat(&format!("{} coach", GROUPS.get(k).copied().unwrap_or("Position")), c);
            }
            if !pairs.is_empty() {
                push(
                    &mut p,
                    json!({ "kind": "kv", "title": "From the world log", "note": format!("Its program row written at the {} signing day book, after the season", year + 1), "pairs": pairs }),
                );
            }
        }
    }
    // Its games (the game log).
    if has(b, "log_games") {
        let games = rows(
            b,
            "SELECT g._row, g.date, g.kind, g.name, g.teams, g.score, g.neutral, g.week FROM log_games g
              WHERE g.stream = 'college' AND g.season = ?2
                AND (json_extract(g.teams, '$[0]') = ?1 OR json_extract(g.teams, '$[1]') = ?1) ORDER BY g.date",
            &[pid.into(), year.into()],
        );
        let out: Vec<Value> = games
            .iter()
            .map(|g| {
                let teams: Vec<i64> = match g.get("teams") {
                    Some(Value::String(t)) => serde_json::from_str(t).unwrap_or_default(),
                    _ => Vec::new(),
                };
                let score: Vec<i64> = match g.get("score") {
                    Some(Value::String(t)) => serde_json::from_str(t).unwrap_or_default(),
                    _ => Vec::new(),
                };
                let home = teams.first() == Some(&pid);
                let opp = if home { teams.get(1) } else { teams.first() }.copied().unwrap_or(-1);
                let (us, them) = match (score.first(), score.get(1)) {
                    (Some(h), Some(a)) => {
                        if home {
                            (*h, *a)
                        } else {
                            (*a, *h)
                        }
                    }
                    _ => (0, 0),
                };
                let site = if i(g.get("neutral")) == Some(1) {
                    "neutral"
                } else if home {
                    "home"
                } else {
                    "away"
                };
                let kind = s(g.get("kind"));
                let nm = s(g.get("name"));
                json!([
                    // The game log keeps a date as text; a world day count shows as one.
                    match g.get("date") {
                        Some(Value::String(d)) => Some(d.clone()),
                        d => i(d).map(crate::pages::day_text),
                    },
                    cell(program_name(b, opp), format!("season/{opp}/{year}")),
                    site,
                    if nm.is_empty() { kind } else { format!("{kind}: {nm}") },
                    if score.len() == 2 {
                        format!(
                            "{} {us}-{them}",
                            if us > them {
                                "W"
                            } else if us < them {
                                "L"
                            } else {
                                "T"
                            }
                        )
                    } else {
                        String::new()
                    },
                    cell("Game", format!("e/log_games/{}", s(g.get("_row")))),
                ])
            })
            .collect();
        if !out.is_empty() {
            push(
                &mut p,
                json!({ "kind": "table", "title": "Its games", "note": "From the game log; a game opens its record",
                "cols": [{"name": "date", "label": "Date"}, {"name": "opp", "label": "Opponent"}, {"name": "site", "label": "Site"},
                         {"name": "kind", "label": "Kind"}, {"name": "score", "label": "Result"}, {"name": "game", "label": ""}],
                "rows": out, "dense": true }),
            );
        }
    }
    // Its roster that year (the world log's player rows).
    if has(b, "log_players") && b.table("log_players").is_some_and(|t| t.has("person__college")) {
        let ros = rows(
            b,
            "SELECT person__player__position AS pos, COUNT(*) AS n, SUM(person__stars >= 4) AS blue, ROUND(AVG(person__stars), 2) AS stars
               FROM log_players WHERE person__college = ?1 AND person__stage = 'College' AND season = ?2 + 1
              GROUP BY pos ORDER BY n DESC",
            &[pid.into(), year.into()],
        );
        let out: Vec<Value> =
            ros.iter().map(|r| json!([s(r.get("pos")), i(r.get("n")), i(r.get("blue")), n(r.get("stars"))])).collect();
        if !out.is_empty() {
            push(
                &mut p,
                json!({ "kind": "table", "title": "Its roster by position", "note": "The world log's player rows at the signing day after the season: the players still on it then",
                "cols": [{"name": "pos", "label": "Position"}, {"name": "n", "label": "Players", "num": true}, {"name": "blue", "label": "4-5 stars", "num": true}, {"name": "stars", "label": "Stars (mean)", "num": true}],
                "rows": out, "dense": true }),
            );
        }
    }
    let items: Vec<Value> = news_of(b, "Program", pid, Some(year), 60)
        .iter()
        .map(|r| json!({ "when": i(r.get("date")).filter(|d| d.abs() < 200_000).map_or(s(r.get("year")), crate::pages::day_text), "text": s(r.get("text")), "why": kind_words(&s(r.get("kind"))) }))
        .collect();
    if !items.is_empty() {
        push(&mut p, json!({ "kind": "timeline", "title": "That year in the news", "items": items }));
    }
    let nav: Vec<Value> = [year - 1, year + 1]
        .iter()
        .filter(|y| {
            !rows(
                b,
                "SELECT 1 FROM programs__seasons WHERE _parent = ?1 AND year = ?2",
                &[json_sql(&own), (**y).into()],
            )
            .is_empty()
        })
        .map(|y| json!({ "text": format!("{name} {y}"), "link": format!("season/{pid}/{y}") }))
        .chain(std::iter::once(json!({ "text": format!("{name}'s page"), "link": format!("e/programs/{pid}") })))
        .collect();
    push(&mut p, json!({ "kind": "links", "title": "Next", "items": nav }));
    if let Some(bl) = p["blocks"].as_array_mut() {
        tidy(bl);
    }
    Ok(p)
}

/// The blocks a high school page adds after the header.
pub fn school_blocks(b: &Book, row: &Map<String, Value>, f: &Filters) -> Vec<Value> {
    let own = row.get("_row").cloned().unwrap_or(Value::Null);
    let id = i(row.get("id")).unwrap_or(0);
    let mut out = Vec::new();
    // In plain words.
    let mut items = Vec::new();
    let st = i(row.get("state"))
        .and_then(|k| {
            rows(b, "SELECT code FROM state_codes WHERE idx = ?1", &[k.into()]).first().map(|r| s(r.get("code")))
        })
        .unwrap_or_default();
    let name = s(row.get("name"));
    let mut t = format!(
        "{name} is a {} high school in {}{}, {st}",
        if i(row.get("private")) == Some(1) { "private" } else { "public" },
        s(row.get("town")),
        {
            let c = s(row.get("county"));
            if c.is_empty() { String::new() } else { format!(" ({c} County)") }
        }
    );
    if let Some(e) = n(row.get("enrollment")) {
        t.push_str(&format!(", {} students", fmt(&json!(e))));
    }
    if let Some(c) = i(row.get("classification")) {
        t.push_str(&format!(", in the state's class {c} this year"));
    }
    t.push('.');
    items.push(json!({ "text": t, "link": if st.is_empty() { Value::Null } else { json!(format!("state/{st}")) } }));
    let mut facts = Vec::new();
    if let Some(p) = n(row.get("prestige")) {
        facts.push(format!("prestige {p:.2}{}", if p >= 0.75 { " (a powerhouse)" } else { "" }));
    }
    if i(row.get("national")) == Some(1) {
        facts.push("one of the national powerhouses that draw from out of state".into());
    }
    if let Some(r) = n(row.get("resources")) {
        facts.push(format!("resources {r:.2} of 1"));
    }
    if let Some(m) = n(row.get("area_money")) {
        facts.push(format!("area money {m:.2}"));
    }
    if i(row.get("real")) == Some(1) {
        facts.push("a real program".into());
    }
    if !facts.is_empty() {
        items.push(json!({ "text": format!("{}.", capital(&facts.join(", "))) }));
    }
    let titles = i(row.get("state_titles")).unwrap_or(0);
    items.push(json!({ "text": format!("{titles} state title{} in the world's seasons.", if titles == 1 { "" } else { "s" }) }));
    for (k, c) in [("Head coach", "coach"), ("Offensive coordinator", "oc"), ("Defensive coordinator", "dc")] {
        if let Some(x) = i(row.get(c)) {
            items.push(json!({ "text": format!("{k}: {}.", coach_name(b, x)), "link": format!("e/coaches/{x}") }));
        }
    }
    out.push(json!({ "kind": "list", "title": "In plain words", "items": items, "dense": true }));
    // Seasons and coaches.
    let seasons = rows(b, "SELECT * FROM high_schools__seasons WHERE _parent = ?1 ORDER BY year", &[json_sql(&own)]);
    if !seasons.is_empty() {
        let mut runs_: Vec<(Option<i64>, i64, i64, i64, i64, i64)> = Vec::new();
        for r in &seasons {
            let y = i(r.get("year")).unwrap_or(0);
            let c = i(r.get("coach"));
            let (w, l) = (i(r.get("wins")).unwrap_or(0), i(r.get("losses")).unwrap_or(0));
            let st = i(r.get("state_title")).unwrap_or(0);
            match runs_.last_mut() {
                Some(x) if x.0 == c && x.2 + 1 == y => {
                    x.2 = y;
                    x.3 += w;
                    x.4 += l;
                    x.5 += st;
                }
                _ => runs_.push((c, y, y, w, l, st)),
            }
        }
        let coach_items: Vec<Value> = runs_
            .iter()
            .map(|(c, a, z, w, l, st)| {
                let ended = c.and_then(|c| stint_end(b, c, "HighSchool", id, *z)).unwrap_or_default();
                json!({ "from": a, "to": z, "text": c.map_or("No coach on record".into(), |c| coach_name(b, c)),
                        "sub": format!("{}{}{}", record(*w, *l), if *st > 0 { format!(", {st} state title{}", if *st == 1 { "" } else { "s" }) } else { String::new() }, if ended.is_empty() { String::new() } else { format!("; {ended}") }),
                        "tone": end_tone(&ended), "link": c.map(|c| format!("e/coaches/{c}")) })
            })
            .collect();
        let title_items: Vec<Value> = seasons
            .iter()
            .filter(|r| i(r.get("state_title")) == Some(1))
            .map(|r| {
                let y = i(r.get("year")).unwrap_or(0);
                json!({ "from": y, "to": y, "text": "", "tone": "FBS", "sub": "state title" })
            })
            .collect();
        out.push(json!({ "kind": "bands", "title": "Its coaches over the years", "note": "Each head coach's run with his record and how it ended; state titles below",
            "lanes": [{ "label": "Head coach", "items": coach_items }, { "label": "State titles", "items": title_items }],
            "keys": [{"tone": "current", "text": "Still there"}, {"tone": "left", "text": "Left for a job"}, {"tone": "retired", "text": "Retired or stepped away"}, {"tone": "fired", "text": "Fired"}] }));
        let rows_out: Vec<Value> = seasons
            .iter()
            .rev()
            .map(|r| {
                let pts: Vec<i64> = match r.get("points") {
                    Some(Value::String(t)) => serde_json::from_str(t).unwrap_or_default(),
                    _ => Vec::new(),
                };
                let g = i(r.get("wins")).unwrap_or(0) + i(r.get("losses")).unwrap_or(0);
                let c = i(r.get("coach"));
                json!([
                    i(r.get("year")),
                    record(i(r.get("wins")).unwrap_or(0), i(r.get("losses")).unwrap_or(0)),
                    if i(r.get("state_title")) == Some(1) {
                        "state champion"
                    } else if i(r.get("semifinal")) == Some(1) {
                        "semifinal"
                    } else {
                        ""
                    },
                    i(r.get("playoff_wins")),
                    c.map_or(json!(""), |c| cell(coach_name(b, c), format!("e/coaches/{c}"))),
                    // A stand-in season keeps no points (0-0): show none.
                    pts.first()
                        .filter(|_| g > 0 && pts.iter().any(|p| *p > 0))
                        .map(|x| format!("{:.1}", *x as f64 / g as f64)),
                    pts.get(1)
                        .filter(|_| g > 0 && pts.iter().any(|p| *p > 0))
                        .map(|x| format!("{:.1}", *x as f64 / g as f64)),
                ])
            })
            .collect();
        out.push(json!({ "kind": "table", "title": "Season by season", "note": "Newest first; points come from engine seasons only (the stand-in seasons before keep none)",
            "cols": [{"name": "year", "label": "Season"}, {"name": "record", "label": "W-L"}, {"name": "how", "label": "Playoffs"},
                     {"name": "pw", "label": "Playoff wins", "num": true}, {"name": "coach", "label": "Head coach"},
                     {"name": "pf", "label": "Points for a game", "num": true}, {"name": "pa", "label": "Against", "num": true}],
            "rows": rows_out, "dense": true }));
    }
    // Players from here: on record now and the alumni kept.
    let mut players: Vec<Value> = Vec::new();
    if b.table("people").is_some_and(|t| t.has("hs")) {
        let draft = if b.table("people").is_some_and(|t| t.has("pro__draft")) { "pro__draft" } else { "NULL" };
        for r in rows(
            b,
            &format!(
                "SELECT player__id AS id, name, class_of, player__position AS pos, stars, stage, college, {draft} AS draft FROM people
                  WHERE hs = ?1 ORDER BY stars DESC, class_of DESC LIMIT 80"
            ),
            &[id.into()],
        ) {
            let pid = i(r.get("id")).unwrap_or(0);
            // The draft as the world keeps it: [year, round, pick, club].
            let dr: Vec<i64> = match r.get("draft") {
                Some(Value::String(t)) => serde_json::from_str(t).unwrap_or_default(),
                _ => Vec::new(),
            };
            players.push(json!([
                cell(s(r.get("name")), format!("e/people/{pid}")),
                i(r.get("class_of")),
                s(r.get("pos")),
                i(r.get("stars")),
                stage_words(&s(r.get("stage"))),
                i(r.get("college")).map_or(json!(""), |c| cell(program_name(b, c), format!("e/programs/{c}"))),
                if dr.len() >= 3 { format!("{} round {}, pick {}", dr[0], dr[1], dr[2]) } else { String::new() },
            ]));
        }
    }
    if b.table("alumni").is_some_and(|t| t.has("high_school")) {
        let idc = b.table("alumni").map_or("_row", crate::book::id_of);
        for r in rows(
            b,
            &format!(
                "SELECT {} AS id, name, class_of, position AS pos, stars, year, left_football AS quit, colleges FROM alumni
                  WHERE high_school = ?1 ORDER BY stars DESC, class_of DESC LIMIT 80",
                q(idc)
            ),
            &[id.into()],
        ) {
            let last: Option<i64> = match r.get("colleges") {
                Some(Value::String(t)) => serde_json::from_str::<Vec<i64>>(t).ok().and_then(|v| v.last().copied()),
                _ => None,
            };
            let gone = match (i(r.get("quit")), i(r.get("year"))) {
                (Some(1), Some(y)) => format!("left football, {y}"),
                (_, Some(y)) => format!("career over, {y}"),
                _ => "career over".into(),
            };
            players.push(json!([
                cell(s(r.get("name")), format!("e/alumni/{}", s(r.get("id")))),
                i(r.get("class_of")),
                s(r.get("pos")),
                i(r.get("stars")),
                gone,
                last.map_or(json!(""), |c| cell(program_name(b, c), format!("e/programs/{c}"))),
                ""
            ]));
        }
    }
    if !players.is_empty() {
        out.push(json!({ "kind": "table", "title": format!("Players from here ({})", players.len()), "note": "On record now and the alumni the world keeps after they leave, best recruits first; a draft as year, round and pick",
            "cols": [{"name": "name", "label": "Player"}, {"name": "class_of", "label": "Class", "fmt": "year"}, {"name": "pos", "label": "Pos"}, {"name": "stars", "label": "Stars", "num": true},
                     {"name": "stage", "label": "Now"}, {"name": "college", "label": "College"}, {"name": "drafted", "label": "Drafted"}],
            "rows": players, "dense": true }));
    }
    let _ = f;
    out.extend(news_block(b, "School", id));
    tidy(&mut out);
    out
}
