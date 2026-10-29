//! The pages, as blocks the screens draw and the copied text reads: the
//! Overview, the topic pages (each area's checks, until its page lane gives
//! it its own sections), Tools, Your career, any table, and every entity's
//! page with the player page as the template.
//!
//! A page is `{title, crumb, verdict, badges, blocks}`. Block kinds: `hero`,
//! `tiles`, `list`, `table`, `rows` (a table paged from the book), `kv`,
//! `chart`, `note`, `links`. Every number carries its hover card in `h`.

use serde_json::{Map, Value, json};

use crate::book::{Book, Filters, YEAR_COLS, cell_text, id_of, label, q};

/// The topic pages in the sidebar's order (spec section 7), with the
/// inventory codes each one shows.
pub const TOPICS: [(&str, &str, &str); 10] = [
    ("people", "People", "P1-P24, P27-P32, T8"),
    ("recruiting", "Recruiting", "C18, C28-C41, P25, P26, T7"),
    ("programs", "Programs", "C1-C17, C19-C22, C42, T13"),
    ("high-school", "High school", "C23-C27, G7, G27, N32"),
    ("coaches", "Coaches", "S1-S31, S33"),
    ("games", "Games and stats", "G1-G6, G8-G26, G28-G38"),
    ("nfl", "NFL", "N1-N29, S34"),
    ("money", "Money", "N30, N31, N33, C14, C41"),
    ("world", "World and time", "N34-N44, N48, T9"),
    ("career", "Your career", "N45-N47, S32"),
];

/// The Tools page's tabs (T1-T6, T10-T12 and the two engine tools).
pub const TOOLS: [(&str, &str, &str); 12] = [
    ("issues", "Known issues", "T1"),
    ("knobs", "Knobs", "T2"),
    ("real", "Real index", "T3"),
    ("coverage", "Coverage map", "T4"),
    ("ids", "Ids and joins", "T5"),
    ("fitted", "Fitted world numbers", "T6"),
    ("whys", "The whys", "T10"),
    ("consistency", "Consistency checks", "T11"),
    ("compare", "Comparing worlds", "T12"),
    ("tables", "Every table", ""),
    ("replay", "Replay a logged game", "G37"),
    ("play", "Play two teams now", ""),
];

/// The page an inventory code belongs on (spec section 7).
pub fn page_of_area(code: &str) -> &'static str {
    let code = code.trim().to_ascii_uppercase();
    let (l, n) = code.split_at(code.find(|c: char| c.is_ascii_digit()).unwrap_or(code.len()));
    let n: u32 = n.parse().unwrap_or(0);
    match l {
        "P" if n == 25 || n == 26 => "recruiting",
        "P" => "people",
        "C" if n == 18 || (28..=41).contains(&n) => "recruiting",
        "C" if (23..=27).contains(&n) => "high-school",
        "C" if n == 14 => "money",
        "C" => "programs",
        "S" if n == 32 => "career",
        "S" if n == 34 => "nfl",
        "S" => "coaches",
        "G" if n == 7 || n == 27 => "high-school",
        "G" => "games",
        "N" if n == 30 || n == 31 || n == 33 => "money",
        "N" if n == 32 => "high-school",
        "N" if (34..=44).contains(&n) || n == 48 => "world",
        "N" if (45..=47).contains(&n) => "career",
        "N" => "nfl",
        "T" if n == 7 => "recruiting",
        "T" if n == 8 => "people",
        "T" if n == 9 => "world",
        "T" if n == 13 => "programs",
        _ => "tools",
    }
}

/// The marks' thresholds (the Debug Lab's; proposed, not confirmed).
pub const NEAR: f64 = 0.05;
pub const OFF: f64 = 0.15;
pub const MARK_RULE: &str =
    "Near: within 5% of real · Off: within 15% · Far off: beyond (the Debug Lab's thresholds, proposed, not confirmed)";

/// One check's result as the book keeps it.
#[derive(Clone, Default)]
pub struct Check {
    pub id: String,
    pub page: String,
    pub area: String,
    pub label: String,
    pub world: Value,
    pub real: Value,
    pub unit: String,
    pub source: String,
    pub years: String,
    pub mark: String,
    pub why: String,
    pub file: String,
    pub function: String,
    pub vital: bool,
    pub link: Value,
    /// How the book worked it out, what it needs, its SQL, the known
    /// issues that touch it and its tolerance (the book's own words).
    pub how: String,
    pub needs: String,
    pub sql: String,
    pub issues: String,
    pub tolerance: String,
}

fn get(m: &Map<String, Value>, keys: &[&str]) -> String {
    keys.iter().find_map(|k| m.get(*k).and_then(cell_text)).unwrap_or_default()
}

fn getv(m: &Map<String, Value>, keys: &[&str]) -> Value {
    keys.iter().find_map(|k| m.get(*k).filter(|v| !v.is_null()).cloned()).unwrap_or(Value::Null)
}

/// "near", "off", "far" or "" from the book's word, or worked out from the
/// two numbers when the book gives none.
pub fn mark_of(word: &str, world: &Value, real: &Value) -> String {
    let w = word.trim().to_ascii_lowercase();
    let m = match w.as_str() {
        "near" | "ok" | "green" | "good" => "near",
        "off" | "amber" | "yellow" | "warn" => "off",
        "far" | "far off" | "far_off" | "red" | "bad" => "far",
        "debug" => "debug",
        // The book has no number to mark: a log it lacks, or no real figure.
        "no data" | "no real" | "none" => "none",
        "" => "",
        _ => "check",
    };
    if !m.is_empty() {
        return m.into();
    }
    match (num(world), num(real)) {
        (Some(a), Some(b)) if b != 0.0 => {
            let rel = ((a - b) / b).abs();
            if rel <= NEAR {
                "near"
            } else if rel <= OFF {
                "off"
            } else {
                "far"
            }
        }
        _ => "",
    }
    .into()
}

pub fn num(v: &Value) -> Option<f64> {
    v.as_f64().or_else(|| v.as_str().and_then(|s| s.trim().trim_end_matches('%').replace(',', "").parse().ok()))
}

/// A number rounded to what matters, with thousands marks.
pub fn fmt(v: &Value) -> String {
    match v {
        Value::Null => "–".into(),
        Value::Number(n) if n.is_i64() => group(n.as_i64().unwrap_or(0)),
        Value::Number(n) => {
            let x = n.as_f64().unwrap_or(0.0);
            let a = x.abs();
            if x.fract() == 0.0 && a < 1e15 {
                group(x as i64)
            } else if a >= 1000.0 {
                group(x.round() as i64)
            } else if a >= 100.0 {
                format!("{x:.0}")
            } else if a >= 10.0 {
                format!("{x:.1}")
            } else if a >= 1.0 {
                format!("{x:.2}")
            } else {
                format!("{x:.3}")
            }
        }
        Value::String(s) => s.clone(),
        x => x.to_string(),
    }
}

fn group(n: i64) -> String {
    let s = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 { format!("−{out}") } else { out }
}

fn with_unit(v: &Value, unit: &str) -> String {
    let s = fmt(v);
    match unit {
        "" => s,
        "%" => format!("{s}%"),
        u => format!("{s} {u}"),
    }
}

impl Check {
    fn from_row(m: &Map<String, Value>) -> Check {
        let world = getv(m, &["world", "world_value", "value"]);
        let real = getv(m, &["real", "real_value", "target"]);
        // A check may touch several inventory areas ("G1, G3"): the first
        // places it.
        let areas = get(m, &["area", "code", "area_code", "areas"]);
        let area = areas.split([',', ' ', ';']).find(|a| !a.is_empty()).unwrap_or("").to_string();
        let mut page = get(m, &["page"]).to_ascii_lowercase().replace([' ', '_'], "-");
        if page.is_empty() && !area.is_empty() {
            page = page_of_area(&area).into();
        }
        Check {
            id: get(m, &["id", "key", "check"]),
            mark: mark_of(&get(m, &["mark", "verdict"]), &world, &real),
            label: get(m, &["label", "name", "title", "what"]),
            unit: get(m, &["unit", "units"]),
            source: get(m, &["source", "real_source"]),
            years: get(m, &["years", "real_years"]),
            why: [get(m, &["why", "words", "explain", "says"]), get(m, &["detail"])]
                .into_iter()
                .filter(|x| !x.is_empty())
                .collect::<Vec<_>>()
                .join(" "),
            file: get(m, &["file"]),
            function: get(m, &["function", "fn"]),
            vital: m.get("vital").is_some_and(|v| v.as_bool().unwrap_or(false) || v.as_i64().unwrap_or(0) != 0),
            link: getv(m, &["link", "route"]),
            how: get(m, &["how"]),
            needs: get(m, &["needs"]),
            sql: get(m, &["sql"]),
            issues: get(m, &["issues"]),
            tolerance: get(m, &["tolerance"]),
            world,
            real,
            area,
            page,
        }
    }

    /// The tile for this check, with its hover card.
    pub fn tile(&self) -> Value {
        let sub = match (num(&self.world), num(&self.real)) {
            (Some(a), Some(b)) if b != 0.0 => {
                let d = (a - b) / b * 100.0;
                let way = if d < 0.0 { "short" } else { "high" };
                format!("Real {} · {:.1}% {way}", with_unit(&self.real, &self.unit), d.abs())
            }
            _ if !self.real.is_null() => format!("Real {}", with_unit(&self.real, &self.unit)),
            _ => "No real figure in the index".into(),
        };
        json!({
            "label": self.label,
            "value": with_unit(&self.world, &self.unit),
            "sub": sub,
            "why": self.why,
            "mark": self.mark,
            "area": self.area,
            "link": self.link,
            "h": self.hover(),
        })
    }

    pub fn hover(&self) -> Value {
        json!({
            "what": self.label,
            "why": self.why,
            "file": self.file,
            "function": self.function,
            "source": self.source,
            "years": self.years,
            "area": self.area,
            "rule": if self.tolerance.is_empty() { MARK_RULE.to_string() } else { self.tolerance.clone() },
            "how": self.how,
            "needs": self.needs,
            "sql": self.sql,
            "issues": self.issues,
            "key": self.id,
        })
    }

    fn severity(&self) -> u8 {
        match self.mark.as_str() {
            "far" => 3,
            "off" => 2,
            "near" => 1,
            _ => 0,
        }
    }
}

/// Every check under the filters. With the year slider on, a book that keeps
/// checks by year gives that year's.
pub fn checks(b: &Book, f: &Filters) -> Vec<Check> {
    let Some(t) = b.find("checks") else { return Vec::new() };
    let mut g = f.clone();
    g.year = None;
    let (mut wh, mut vals) = b.where_for(t, &g);
    if let (Some(y), Some(c)) = (f.year, t.pick(&YEAR_COLS)) {
        vals.push(rusqlite::types::Value::Integer(y));
        let cond = format!("({c} IS NULL OR {c} = ?{})", vals.len(), c = q(c));
        wh = if wh.is_empty() { format!(" WHERE {cond}") } else { format!("{wh} AND {cond}") };
    }
    let rows = b.select_maps(&format!("SELECT * FROM {}{wh}", q(&t.name)), &vals).unwrap_or_default();
    rows.iter().map(Check::from_row).collect()
}

pub fn new_page(title: &str, crumb: &str) -> Value {
    json!({ "title": title, "crumb": crumb, "verdict": "", "badges": [], "blocks": [] })
}

pub fn push(p: &mut Value, block: Value) {
    if let Some(a) = p["blocks"].as_array_mut() {
        a.push(block);
    }
}

/// Counts by mark, for the sidebar and the hero.
pub fn marks(cs: &[Check]) -> Value {
    let n = |m: &str| cs.iter().filter(|c| c.mark == m).count();
    json!({ "checks": cs.len(), "near": n("near"), "off": n("off"), "far": n("far") })
}

/// The verdict in plain words for a set of checks.
pub fn verdict(cs: &[Check], what: &str) -> String {
    if cs.is_empty() {
        return format!("No checks for {what} in this book yet.");
    }
    let m = marks(cs);
    let mut far: Vec<&Check> = cs.iter().filter(|c| c.mark == "far").collect();
    far.truncate(3);
    let mut s =
        format!("{} checks for {what}: {} near real, {} off, {} far off.", cs.len(), m["near"], m["off"], m["far"]);
    if !far.is_empty() {
        let list: Vec<String> = far
            .iter()
            .map(|c| {
                let real =
                    if c.real.is_null() { String::new() } else { format!(" (real {})", with_unit(&c.real, &c.unit)) };
                format!("{} {}{real}", c.label, with_unit(&c.world, &c.unit))
            })
            .collect();
        s.push_str(&format!(" Furthest: {}.", list.join("; ")));
    }
    s
}

/// The odd things, at every level, for a page or an entity.
pub fn odd(
    b: &Book,
    page: Option<&str>,
    entity: Option<(&str, &Value)>,
    f: &Filters,
    limit: usize,
) -> (Vec<Value>, i64) {
    let Some(t) = b.find("odd") else { return (Vec::new(), 0) };
    let mut g = f.clone();
    g.year = None;
    let (mut wh, mut vals) = b.where_for(t, &g);
    let mut add = |cond: String, v: Value, vals: &mut Vec<rusqlite::types::Value>| {
        vals.push(crate::book::json_sql(&v));
        let c = cond.replace("?", &format!("?{}", vals.len()));
        wh = if wh.is_empty() { format!(" WHERE {c}") } else { format!("{wh} AND {c}") };
    };
    if let (Some(p), Some(c)) = (page, t.pick(&["page"])) {
        add(format!("LOWER(REPLACE({}, ' ', '-')) = ?", q(c)), json!(p), &mut vals);
    }
    if let Some((table, id)) = entity {
        if let Some(c) = t.pick(&["entity_id", "id_of", "ref_id"]) {
            add(format!("CAST({} AS TEXT) = CAST(? AS TEXT)", q(c)), id.clone(), &mut vals);
            if let Some(k) = t.pick(&["entity", "entity_table", "kind_table", "tbl"]) {
                add(format!("{} = ?", q(k)), json!(table), &mut vals);
            }
        } else {
            return (Vec::new(), 0);
        }
    }
    let total: i64 = b
        .db
        .query_row(&format!("SELECT COUNT(*) FROM {}{wh}", q(&t.name)), rusqlite::params_from_iter(vals.iter()), |r| {
            r.get(0)
        })
        .unwrap_or(0);
    let order = if t.has("rank") { " ORDER BY rank" } else { "" };
    let rows =
        b.select_maps(&format!("SELECT * FROM {}{wh}{order} LIMIT {limit}", q(&t.name)), &vals).unwrap_or_default();
    let items = rows
        .iter()
        .map(|m| {
            let et = get(m, &["entity", "entity_table", "kind_table", "tbl"]);
            let eid = getv(m, &["entity_id", "id_of", "ref_id"]);
            let link =
                if !et.is_empty() && !eid.is_null() { json!(format!("e/{et}/{}", fmt_id(&eid))) } else { Value::Null };
            json!({
                "text": get(m, &["text", "what", "summary", "label"]),
                "why": get(m, &["why", "facts", "detail"]),
                "mark": mark_of(&get(m, &["mark"]), &Value::Null, &Value::Null),
                "level": get(m, &["level_of", "scope", "kind"]),
                "link": link,
            })
        })
        .collect();
    (items, total)
}

/// Columns that hold a year, shown without thousands marks.
pub fn is_year(c: &str) -> bool {
    let c = c.to_ascii_lowercase();
    let last = c.rsplit("__").next().unwrap_or(&c);
    ["year", "season", "class_of", "since", "through", "new_from", "signed", "until", "changed", "drafted"]
        .iter()
        .any(|w| last == *w || last.ends_with(&format!("_{w}")) || last.starts_with(&format!("{w}_")))
}

/// A number as `fmt` gives it, but a year as the year and a day as its date.
pub fn fmt_col(col: &str, v: &Value) -> String {
    match num(v) {
        Some(x) if is_date(col) && x.fract() == 0.0 && x.abs() < 100_000.0 => day_text(x as i64),
        Some(x) if is_year(col) && (1800.0..2300.0).contains(&x) => format!("{}", x.round() as i64),
        _ => fmt(v),
    }
}

/// Columns the book keeps as days since 1 January 1970 (the engine's
/// `Date`): `date`, `born`, `signed__date`.
pub fn is_date(c: &str) -> bool {
    let last = c.rsplit("__").next().unwrap_or(c);
    matches!(last, "date" | "born" | "kickoff_date" | "until_date") || last.ends_with("_date")
}

/// A day number as `2025-08-30`.
pub fn day_text(days: i64) -> String {
    // Howard Hinnant's civil-from-days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

pub fn fmt_id(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        x => x.to_string(),
    }
}

/// The book's facts: world, seed, commit, what it was built from.
pub fn meta(b: &Book) -> Map<String, Value> {
    let mut m = Map::new();
    if let Some(t) = b.find("meta") {
        if let (Some(k), Some(v)) = (t.pick(&["key", "k", "name"]), t.pick(&["value", "v"])) {
            let rows =
                b.select_maps(&format!("SELECT {}, {} FROM {}", q(k), q(v), q(&t.name)), &[]).unwrap_or_default();
            for r in rows {
                if let Some(key) = r.get(k).and_then(cell_text) {
                    m.insert(key, r.get(v).cloned().unwrap_or(Value::Null));
                }
            }
        } else if let Ok(mut rows) = b.select_maps(&format!("SELECT * FROM {} LIMIT 1", q(&t.name)), &[])
            && let Some(r) = rows.pop()
        {
            m = r;
        }
    }
    m
}

/// The world's name for headers.
pub fn world_name(b: &Book) -> String {
    let m = meta(b);
    let g = |k: &str| m.get(k).and_then(cell_text).unwrap_or_default();
    let name = g("name");
    if !name.is_empty() {
        return name;
    }
    let seed = g("seed");
    if !seed.is_empty() {
        let commit = g("commit");
        let c = commit.get(..7).unwrap_or(&commit);
        return if c.is_empty() { format!("Seed {seed}") } else { format!("Seed {seed}, {c}") };
    }
    b.path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "World".into())
}

/// The coverage badges: what the book was built from and what is missing.
pub fn coverage_badges(b: &Book) -> Vec<Value> {
    let m = meta(b);
    let yes = |k: &str| {
        m.get(k)
            .is_some_and(|v| v.as_bool().unwrap_or(false) || v.as_i64().unwrap_or(0) != 0 || v.as_str() == Some("yes"))
    };
    let mut v = vec![json!({ "text": "World file", "ok": true })];
    // R4's book names the logs it read (`log_streams`) and what it lacks
    // (`missing`, a list of plain lines).
    let streams = m.get("log_streams").and_then(cell_text).unwrap_or_default();
    let log = match m.get("has_log").or_else(|| m.get("world_log")) {
        Some(_) => yes("has_log") || yes("world_log"),
        None => streams.len() > 2 || b.tables.iter().any(|t| t.name.starts_with("log_")) || b.find("plays").is_some(),
    };
    v.push(
        json!({ "text": if log { "World log" } else { "No world log: history and growth parts missing" }, "ok": log }),
    );
    v.push(json!({ "text": if b.find("real").is_some() || b.find("checks").is_some() { "Real index" } else { "No real index" }, "ok": b.find("real").is_some() || b.find("checks").is_some() }));
    if let Some(miss) = m.get("missing").and_then(cell_text).filter(|s| !s.is_empty() && s != "[]") {
        let lines: Vec<String> = serde_json::from_str(&miss).unwrap_or_else(|_| vec![miss.clone()]);
        for l in lines {
            // "game log (lane R1): no game, ..." → the short part as the
            // badge, the rest on hover.
            let (head, rest) = l.split_once(": ").unwrap_or((&l, ""));
            v.push(json!({ "text": format!("No {head}"), "ok": false, "h": { "what": l, "why": rest } }));
        }
    }
    v
}

fn tiles_block(title: &str, cs: &[&Check]) -> Value {
    json!({ "kind": "tiles", "title": title, "note": MARK_RULE, "tiles": cs.iter().map(|c| c.tile()).collect::<Vec<_>>() })
}

fn check_table(title: &str, cs: &[&Check]) -> Value {
    json!({
        "kind": "table",
        "title": title,
        "cols": [
            {"name": "label", "label": "Measure"}, {"name": "area", "label": "Area"},
            {"name": "world", "label": "World", "num": true}, {"name": "real", "label": "Real", "num": true},
            {"name": "mark", "label": "Mark", "mark": true}, {"name": "why", "label": "Why"}
        ],
        "rows": cs.iter().map(|c| json!([c.label, c.area, with_unit(&c.world, &c.unit), if c.real.is_null() { "–".to_string() } else { with_unit(&c.real, &c.unit) }, c.mark, c.why])).collect::<Vec<_>>(),
        "hs": cs.iter().map(|c| c.hover()).collect::<Vec<_>>(),
        "links": cs.iter().map(|c| c.link.clone()).collect::<Vec<_>>(),
    })
}

/// Page 1: the world's header and coverage, every check's verdict grouped by
/// page, the far-off list, the odd things, and what changed against the
/// world before it (`before`, the library's previous book of the same seed).
pub fn overview(b: &mut Book, f: &Filters, before: Option<(&str, Vec<Check>)>) -> Value {
    let name = world_name(b);
    let mut p = new_page("Overview", &format!("{name} · Overview"));
    let cs = checks(b, f);
    let m = meta(b);
    let gm = |k: &str| m.get(k).and_then(cell_text).unwrap_or_default();
    p["verdict"] = json!(verdict(&cs, "this world"));
    p["badges"] = json!(coverage_badges(b));
    let mut facts: Vec<String> = Vec::new();
    for (k, l) in [
        ("history_years", "years of history"),
        ("years", "years"),
        ("first", "from"),
        ("last", "to"),
        ("commit", "commit"),
        ("built", "built"),
        ("written", "world written"),
    ] {
        let v = gm(k);
        if !v.is_empty() {
            facts.push(match k {
                "commit" => format!("commit {}", v.get(..7).unwrap_or(&v)),
                "history_years" | "years" => format!("{v} {l}"),
                _ => format!("{l} {v}"),
            });
        }
    }
    let size = std::fs::metadata(&b.path).map(|x| x.len()).unwrap_or(0);
    facts.push(format!("book {:.2} GB", size as f64 / 1e9));
    facts.push(format!("{} tables", b.tables.len()));
    let mk = marks(&cs);
    push(
        &mut p,
        json!({
            "kind": "hero", "label": "World", "title": name,
            "text": gm("summary"), "tags": facts,
            "figures": [
                {"label": "Checks", "value": mk["checks"]},
                {"label": "Near", "value": mk["near"], "mark": "near"},
                {"label": "Off", "value": mk["off"], "mark": "off"},
                {"label": "Far off", "value": mk["far"], "mark": "far"}
            ]
        }),
    );
    let mut vit: Vec<&Check> = cs.iter().filter(|c| c.vital).collect();
    if vit.is_empty() {
        vit = cs.iter().collect();
        vit.sort_by_key(|c| std::cmp::Reverse(c.severity()));
        vit.truncate(12);
    }
    if !vit.is_empty() {
        push(&mut p, tiles_block("Vital signs against real", &vit));
    }
    // Every page's verdict.
    let mut rows = Vec::new();
    let mut links = Vec::new();
    for (slug, title, codes) in TOPICS.iter().chain([("tools", "Tools", "T1-T12")].iter()) {
        let here: Vec<Check> = cs.iter().filter(|c| c.page == *slug).cloned().collect();
        let m = marks(&here);
        rows.push(json!([title, codes, m["checks"], m["near"], m["off"], m["far"]]));
        links.push(json!(format!("page/{slug}")));
    }
    push(
        &mut p,
        json!({
            "kind": "table", "title": "Every page's checks",
            "cols": [{"name": "page", "label": "Page"}, {"name": "areas", "label": "Areas"}, {"name": "checks", "label": "Checks", "num": true},
                     {"name": "near", "label": "Near", "num": true, "tone": "near"}, {"name": "off", "label": "Off", "num": true, "tone": "off"}, {"name": "far", "label": "Far off", "num": true, "tone": "far"}],
            "rows": rows, "links": links
        }),
    );
    let mut far: Vec<&Check> = cs.iter().filter(|c| c.mark == "far").collect();
    far.sort_by(|a, b| a.page.cmp(&b.page));
    if !far.is_empty() {
        push(&mut p, check_table(&format!("Far off ({})", far.len()), &far));
    }
    let (items, total) = odd(b, None, None, f, 8);
    push(
        &mut p,
        json!({ "kind": "list", "title": "Odd things", "items": items, "more": {"n": total, "link": "tools/odd"},
                "empty": "This book keeps no odd-things list yet." }),
    );
    if let Some((prev, old)) = before {
        push(&mut p, crate::compare::moved_block(&format!("What changed against {prev}"), &old, &cs));
    } else {
        push(
            &mut p,
            json!({ "kind": "note", "text": "No earlier book of this seed in the library, so there is nothing to set this world against. Compare picks any two." }),
        );
    }
    p
}

/// A topic page: its checks as tiles and a full table, its odd things and
/// its tables. A page lane (R6-R8) adds the area sections on top.
pub fn topic(b: &mut Book, slug: &str, f: &Filters) -> Result<Value, String> {
    let (title, codes) = TOPICS.iter().find(|t| t.0 == slug).map(|t| (t.1, t.2)).unwrap_or(("Tools", "T1-T12"));
    let name = world_name(b);
    let mut p = new_page(title, &format!("{name} · {title}"));
    let cs: Vec<Check> = checks(b, f).into_iter().filter(|c| c.page == slug).collect();
    p["verdict"] = json!(verdict(&cs, &title.to_lowercase()));
    p["badges"] = json!([{ "text": format!("Areas {codes}"), "ok": true }]);
    let mut sorted: Vec<&Check> = cs.iter().collect();
    sorted.sort_by_key(|c| std::cmp::Reverse(c.severity()));
    if !sorted.is_empty() {
        push(&mut p, tiles_block("Against real", &sorted.iter().take(12).copied().collect::<Vec<_>>()));
        push(&mut p, check_table(&format!("Every check ({})", sorted.len()), &sorted));
    }
    let (items, total) = odd(b, Some(slug), None, f, 10);
    if total > 0 {
        push(
            &mut p,
            json!({ "kind": "list", "title": "Odd things", "items": items, "more": {"n": total, "link": format!("tools/odd/{slug}")} }),
        );
    }
    // The inventory's areas for this page and how far the book covers each.
    if let Some(cov) = b.find("coverage").map(|t| t.name.clone())
        && b.table(&cov).is_some_and(|t| t.has("page"))
    {
        let first = b.rows(&cov, &Filters::default(), Some(("page", &json!(slug))), None, false, 0, 60)?;
        if first["total"].as_i64().unwrap_or(0) > 0 {
            push(
                &mut p,
                json!({ "kind": "rows", "title": "What this page covers", "note": "Each inventory area: checked, data with no check yet, needs a log, or stored nowhere",
                        "table": cov, "col": "page", "val": slug, "first": first, "hide": ["page"] }),
            );
        }
    }
    let tabs = page_tables(b, slug);
    if !tabs.is_empty() {
        push(&mut p, links_block(b, "Tables behind this page", &tabs));
    }
    Ok(p)
}

/// The tables the book files under a page (its coverage map's `page`), else
/// none: the page lanes name theirs.
fn page_tables(b: &Book, slug: &str) -> Vec<String> {
    let Some(t) = b.find("coverage") else { return Vec::new() };
    // R4: each area lists its tables, space-separated.
    if let (Some(pc), true) = (t.pick(&["page"]), t.has("tables")) {
        let sql = format!("SELECT tables FROM {} WHERE LOWER(REPLACE({}, ' ', '-')) = ?1", q(&t.name), q(pc));
        let mut out: Vec<String> = Vec::new();
        for m in b.select_maps(&sql, &[rusqlite::types::Value::Text(slug.into())]).unwrap_or_default() {
            for n in m.get("tables").and_then(cell_text).unwrap_or_default().split([' ', ',']) {
                if !n.is_empty() && b.table(n).is_some() && !out.iter().any(|o| o == n) {
                    out.push(n.to_string());
                }
            }
        }
        return out;
    }
    let (Some(pc), Some(tc)) = (t.pick(&["page"]), t.pick(&["tbl", "table_name", "table"])) else { return Vec::new() };
    let sql =
        format!("SELECT DISTINCT {} AS t FROM {} WHERE LOWER(REPLACE({}, ' ', '-')) = ?1", q(tc), q(&t.name), q(pc));
    b.select_maps(&sql, &[rusqlite::types::Value::Text(slug.into())])
        .unwrap_or_default()
        .iter()
        .filter_map(|m| m.get("t").and_then(cell_text))
        .filter(|n| b.table(n).is_some())
        .collect()
}

fn links_block(b: &mut Book, title: &str, tables: &[String]) -> Value {
    let items: Vec<Value> = tables
        .iter()
        .map(|t| {
            let n = b.count(t);
            json!({ "text": label(t), "sub": format!("{} rows", fmt(&json!(n))), "link": format!("table/{t}") })
        })
        .collect();
    json!({ "kind": "links", "title": title, "items": items })
}

/// A table of the book, paged.
pub fn table(b: &mut Book, name: &str, f: &Filters, args: &Value) -> Result<Value, String> {
    if b.table(name).is_none() {
        return Err(format!("This book has no table {name}."));
    }
    if !f.debug && b.is_debug_table(name) {
        return Err("That table holds hidden truths; turn them on in the sidebar.".into());
    }
    let mut p = new_page(&label(name), &format!("{} · {}", world_name(b), label(name)));
    let extra = args["col"].as_str().map(|c| (c.to_string(), args["val"].clone()));
    let n = b.count(name);
    p["verdict"] = json!(format!("{} rows in {}.", fmt(&json!(n)), label(name)));
    let filt = b.filterable(b.table(name).expect("checked above"));
    p["badges"] = json!([{ "text": if filt.is_empty() { "No filter applies to this table".into() } else { format!("Filters: {}", filt.join(", ")) }, "ok": !filt.is_empty() }]);
    let first = b.rows(
        name,
        f,
        extra.as_ref().map(|(c, v)| (c.as_str(), v)),
        args["sort"].as_str(),
        args["desc"].as_bool().unwrap_or(false),
        0,
        50,
    )?;
    push(
        &mut p,
        json!({ "kind": "rows", "title": label(name), "table": name, "col": extra.as_ref().map(|e| e.0.clone()), "val": extra.map(|e| e.1), "first": first }),
    );
    Ok(p)
}

/// Columns that hold a season's rating, for the career arc.
const RATING_COLS: [&str; 7] = ["rating", "overall", "ovr", "rating_end", "end_rating", "prestige", "strength"];
const PLAYED_COLS: [&str; 3] = ["played_at", "game_rating", "played"];
const WHY_COLS: [&str; 5] = ["why", "what", "what_happened", "note", "story"];

/// Any entity's page: its row, its odd things, its career arc when a table
/// of its seasons has a rating, its seasons with their whys, and every
/// other row that points at it. The player page is this template.
pub fn entity(b: &mut Book, table: &str, id: &Value, f: &Filters) -> Result<Value, String> {
    let t = b.table(table).ok_or_else(|| format!("This book has no table {table}."))?;
    let idc = id_of(t);
    let row = b
        .select_maps(&format!("SELECT * FROM {} WHERE {} = ?1", q(table), q(idc)), &[crate::book::json_sql(id)])?
        .pop()
        .ok_or_else(|| format!("No row {} in {table}.", fmt_id(id)))?;
    let name = b
        .name_of(table, id)
        .or_else(|| log_name(b, table, &row))
        .unwrap_or_else(|| format!("{} {}", label(table), fmt_id(id)));
    let mut p = new_page(&name, &format!("{} · {} · {name}", world_name(b), label(table)));
    // The header: the row's short facts, every id as its name; lists of
    // ratings as their own table.
    let mut tags: Vec<String> = Vec::new();
    let mut pairs: Vec<Value> = Vec::new();
    let mut lists: Vec<(String, Vec<f64>, bool)> = Vec::new();
    let mut notes: Vec<(String, Vec<String>)> = Vec::new();
    let mut idlists: Vec<(String, (&'static str, &'static str), Vec<f64>)> = Vec::new();
    for c in &b.table(table).expect("checked").cols.clone() {
        // `rng__` is the person's random stream: no meaning to read.
        if c == idc || c == "_row" || c == "_more" || c.starts_with("rng__") || (!f.debug && b.is_debug(table, c)) {
            continue;
        }
        let v = row.get(c).cloned().unwrap_or(Value::Null);
        if v.is_null() {
            continue;
        }
        if let Some(xs) = number_list(&v)
            && named_list(c, xs.len()).is_some()
        {
            lists.push((c.clone(), xs, b.is_debug(table, c)));
            continue;
        }
        if let Some(xs) =
            v.as_str().filter(|t| t.starts_with("[\"")).and_then(|t| serde_json::from_str::<Vec<String>>(t).ok())
            && (xs.len() > 3 || xs.iter().any(|x| x.len() > 40))
        {
            notes.push((c.clone(), xs));
            continue;
        }
        if let Some(target) = id_list_target(c)
            && let Some(ids) = number_list(&v)
            && b.table(target.0).is_some_and(|t| t.has(target.1))
        {
            idlists.push((c.clone(), target, ids));
            continue;
        }
        let link = b.link_target(table, c).map(|t| t.to_string());
        let key = link.as_deref().map(|t| b.link_key(c, t));
        let shown = match &link {
            Some(t) if key == b.table(t).map(id_of) => b.name_of(t, &v).unwrap_or_else(|| fmt(&v)),
            _ => short_list(&v).unwrap_or_else(|| fmt_col(c, &v)),
        };
        let word = c.rsplit("__").next().unwrap_or(c);
        if ["position", "pos", "level", "stage", "state", "home_state", "stars", "conference", "division"]
            .contains(&word)
            && c.matches("__").count() <= 1
            && (word != "stars" || num(&v).is_some_and(|x| x > 0.0))
        {
            tags.push(if word == "stars" { format!("{shown} stars") } else { shown.clone() });
        }
        pairs.push(json!({
            "k": b.col_info(table, c)["label"], "v": shown, "debug": b.is_debug(table, c),
            "link": link.filter(|_| key != Some("_row") || c != "_parent").map(|t| format!("e/{t}/{}", fmt_id(&v))),
            "h": b.hover(table, c)
        }));
    }
    let people = b.find("people").is_some_and(|t| t.name == table);
    let actions = if people {
        json!([{ "text": "Players like him", "link": format!("compare/player?id={}", fmt_id(id)) }])
    } else {
        json!([])
    };
    push(
        &mut p,
        json!({ "kind": "hero", "label": label(table), "title": name, "tags": tags, "figures": [], "actions": actions }),
    );
    let (items, total) = odd(b, None, Some((table, id)), f, 20);
    if total > 0 {
        push(
            &mut p,
            json!({ "kind": "list", "title": "What doesn't look real", "note": "Found by the reader, each with its facts", "items": items, "more": {"n": total} }),
        );
    }
    // The career arc: of the tables about this one with a rating by year
    // (or by age), the one with the most points.
    let related = b.pointing_at(table);
    let own_row = row.get("_row").cloned().unwrap_or(Value::Null);
    let key_of = |rc: &str| if rc == "_parent" { own_row.clone() } else { id.clone() };
    let mut g = f.clone();
    g.year = None;
    let mut arc: Option<(usize, Value)> = None;
    for (rt, rc) in &related {
        // Only rows that are this one's own (its child tables and its log
        // rows), not every player who links to a program.
        if rc != "_parent" && rc != "id" && !rc.ends_with("_id") {
            continue;
        }
        let t = b.table(rt).expect("listed");
        let (Some(xc), Some(rcol)) = (t.pick(&YEAR_COLS).or(t.pick(&["age"])), t.pick(&RATING_COLS)) else {
            continue;
        };
        let pc = t.pick(&["potential", "true_potential", "ceiling"]).filter(|_| f.debug);
        let played = t.pick(&PLAYED_COLS);
        let mut cols = vec![xc, rcol];
        cols.extend(pc);
        cols.extend(played);
        let sql = format!(
            "SELECT {} FROM {} WHERE {} = ?1 ORDER BY {}",
            cols.iter().map(|c| q(c)).collect::<Vec<_>>().join(", "),
            q(rt),
            q(rc),
            q(xc)
        );
        let rows = b.select_maps(&sql, &[crate::book::json_sql(&key_of(rc))]).unwrap_or_default();
        if rows.len() < 2 || arc.as_ref().is_some_and(|(n, _)| *n >= rows.len()) {
            continue;
        }
        let series = |c: &str| -> Vec<Value> { rows.iter().map(|r| json!([r.get(xc), r.get(c)])).collect() };
        let mut s = vec![json!({ "name": label(rcol), "points": series(rcol) })];
        if let Some(pl) = played {
            s.push(json!({ "name": "Played at", "points": series(pl), "style": "thin" }));
        }
        if let Some(pc) = pc {
            s.push(json!({ "name": "Potential (debug)", "points": series(pc), "style": "dash", "debug": true }));
        }
        let x = if xc == "age" { "Age" } else { "Year" };
        let mark = if xc == "age" { Value::Null } else { json!(f.year) };
        arc = Some((
            rows.len(),
            json!({ "kind": "chart", "type": "line", "title": if people { "Career arc" } else { "Over the years" },
                    "note": format!("From {}", label(rt)),
                    "x": x, "series": s, "mark_year": mark }),
        ));
    }
    if let Some((_, c)) = arc {
        push(&mut p, c);
    }
    // Ratings with their ceilings beside them (debug), and tendencies with
    // the year before, each as one compact grid.
    let find = |w: &str| lists.iter().find(|(c, _, _)| c.rsplit("__").next() == Some(w));
    let pairs_of = |now: &(String, Vec<f64>, bool), was: Option<&(String, Vec<f64>, bool)>, joint: &str| {
        let names = named_list(&now.0, now.1.len()).expect("checked");
        names
            .iter()
            .enumerate()
            .map(|(i, n)| {
                let a = fmt(&json!(now.1[i]));
                let v = match was.and_then(|w| w.1.get(i)) {
                    Some(x) => format!("{a} {joint} {}", fmt(&json!(x))),
                    None => a,
                };
                json!({ "k": n, "v": v })
            })
            .collect::<Vec<Value>>()
    };
    if let Some(r) = find("attributes") {
        let ceil = find("ceilings").filter(|_| f.debug);
        let (title, note) = match ceil {
            Some(_) => ("Ratings → ceilings (debug)", "Each rating now, then its hidden ceiling"),
            None => ("Ratings", "Each rating now, 1-99"),
        };
        push(&mut p, json!({ "kind": "kv", "title": title, "note": note, "pairs": pairs_of(r, ceil, "→") }));
    }
    if let Some(t) = find("v") {
        push(
            &mut p,
            json!({ "kind": "kv", "title": "Tendencies", "note": "How often he tries each, now (and a year before), 1-20",
                    "pairs": pairs_of(t, find("was"), "· was") }),
        );
    }
    for (c, (target, key), ids) in &idlists {
        // Each reference as its name and its page (by the target's own id).
        let tt = b.table(target).expect("checked");
        let (own, name_sql) = (id_of(tt), b.name_sql(tt).unwrap_or_else(|| "''".into()));
        let sql = format!("SELECT {} AS id, {name_sql} AS name FROM {} WHERE {} = ?1", q(own), q(target), q(key));
        let items: Vec<Value> = ids
            .iter()
            .map(|x| {
                let r = b.select_maps(&sql, &[rusqlite::types::Value::Integer(*x as i64)]).unwrap_or_default();
                match r.first() {
                    Some(m) => json!({ "text": cell_text(&m["name"]).unwrap_or_default(),
                                       "link": format!("e/{target}/{}", fmt_id(&m["id"])) }),
                    None => json!({ "text": format!("{} at {}", label(target), *x as i64) }),
                }
            })
            .collect();
        push(
            &mut p,
            json!({ "kind": "list", "title": format!("{} ({})", label(c), ids.len()), "items": items, "dense": true,
                    "h": b.hover(table, c) }),
        );
    }
    for (c, xs) in &notes {
        let items: Vec<Value> = xs.iter().map(|x| json!({ "text": x })).collect();
        push(
            &mut p,
            json!({ "kind": "list", "title": label(c), "items": items, "dense": true, "h": b.hover(table, c) }),
        );
    }
    if !pairs.is_empty() {
        push(&mut p, json!({ "kind": "kv", "title": format!("{} record", label(table)), "pairs": pairs }));
    }
    // A world log row's packed record (`_more`): every field it kept.
    if b.table(table).is_some_and(|t| t.has("_more"))
        && let Some(more) = b.more_of(table, row.get("_row").unwrap_or(&Value::Null))
    {
        more_blocks(b, table, &more, f, &mut p);
    }
    // Every table that points at this one: its rows for this entity, the
    // ones with a why and a rating first.
    let mut seasons_first = related.clone();
    seasons_first.sort_by_key(|(rt, rc)| {
        let t = b.table(rt).expect("listed");
        (rc != "_parent" && rc != "id", t.pick(&WHY_COLS).is_none(), t.pick(&RATING_COLS).is_none(), rt.clone())
    });
    let mut index: Vec<Value> = Vec::new();
    for (rt, rc) in seasons_first {
        if !f.debug && b.is_debug_table(&rt) {
            continue;
        }
        let key = key_of(&rc);
        let first = b.rows(&rt, &g, Some((&rc, &key)), None, false, 0, 12)?;
        let n = first["total"].as_i64().unwrap_or(0);
        if n == 0 {
            continue;
        }
        // On Alabama's page `programs__seasons` is just "Seasons".
        let short = label(rt.strip_prefix(&format!("{table}__")).unwrap_or(&rt));
        index.push(json!({
            "text": short,
            "sub": format!("{} row{}{}", fmt(&json!(n)), if n == 1 { "" } else { "s" },
                           if rc == "_parent" || rc == "id" { "" } else { " that name it" }),
            "link": format!("table/{rt}?col={rc}&val={}", fmt_id(&key)),
        }));
        let title = match b.table(&rt).and_then(|t| t.pick(&WHY_COLS)) {
            Some(_) => format!("{short}: every row with its why"),
            None => short.clone(),
        };
        push(
            &mut p,
            json!({ "kind": "rows", "title": title, "table": rt, "col": rc, "val": key, "first": first, "hide": [rc, "_row", "_parent"] }),
        );
    }
    // Where everything about it is, before the long tables.
    if index.len() > 3
        && let Some(blocks) = p["blocks"].as_array_mut()
    {
        let at = blocks.len().min(if total > 0 { 2 } else { 1 });
        blocks.insert(
            at,
            json!({ "kind": "links", "title": format!("Everything about {name} ({} tables)", index.len()), "items": index }),
        );
    }
    p["verdict"] = json!(match total {
        0 => format!("Nothing about {name} breaks a check in this book."),
        n => format!("{n} odd thing{} about {name}.", if n == 1 { "" } else { "s" }),
    });
    Ok(p)
}

/// A world log row by whom it is about and its season: "William
/// Simmons, 2024".
fn log_name(b: &Book, table: &str, row: &Map<String, Value>) -> Option<String> {
    if !table.starts_with("log_") {
        return None;
    }
    let who = row.get("id").filter(|v| !v.is_null())?;
    let t = b.link_target(table, "id")?;
    let name = b.name_of(t, who)?;
    Some(match row.get("season").and_then(num) {
        Some(y) => format!("{name}, {}", y as i64),
        None => name,
    })
}

/// The blocks for a packed record: its plain fields as one list (nested
/// names joined the book's way, every id as its name), lists of ratings as
/// grids, lists of records as lines.
fn more_blocks(b: &Book, table: &str, more: &Value, f: &Filters, p: &mut Value) {
    let mut flat: Vec<(String, Value)> = Vec::new();
    flatten("", more, &mut flat);
    let cols = &b.table(table).expect("checked").cols;
    let mut pairs: Vec<Value> = Vec::new();
    for (k, v) in flat {
        if v.is_null()
            || v.as_array().is_some_and(Vec::is_empty)
            || cols.contains(&k)
            || (!f.debug && b.is_debug(table, &k))
            || k.starts_with("rng")
        {
            continue;
        }
        let debug = b.is_debug(table, &k);
        match &v {
            Value::Array(xs) if xs.iter().all(|x| x.is_object()) => {
                let items: Vec<Value> = xs
                    .iter()
                    .take(60)
                    .map(|x| {
                        let mut parts: Vec<(String, Value)> = Vec::new();
                        flatten("", x, &mut parts);
                        let text = parts
                            .iter()
                            .filter(|(_, v)| !v.is_null())
                            .map(|(pk, pv)| format!("{} {}", label(pk), more_text(b, table, pk, pv).0))
                            .collect::<Vec<_>>()
                            .join(" · ");
                        json!({ "text": text })
                    })
                    .collect();
                let more_n = xs.len().saturating_sub(60);
                push(
                    p,
                    json!({ "kind": "list", "title": format!("{} ({})", label(&k), xs.len()), "items": items,
                            "dense": true, "debug": debug, "more": if more_n > 0 { json!({"n": more_n}) } else { Value::Null } }),
                );
            }
            Value::Array(xs) if named_list(&k, xs.len()).is_some() && xs.iter().all(Value::is_number) => {
                let names = named_list(&k, xs.len()).expect("checked");
                let grid: Vec<Value> =
                    names.iter().zip(xs).map(|(n, x)| json!({ "k": n, "v": fmt(&json!(num(x))) })).collect();
                push(p, json!({ "kind": "kv", "title": label(&k), "pairs": grid, "debug": debug }));
            }
            _ => {
                let (shown, link) = more_text(b, table, &k, &v);
                pairs
                    .push(json!({ "k": label(&k), "v": shown, "debug": debug, "link": link, "h": b.hover(table, &k) }));
            }
        }
    }
    if !pairs.is_empty() {
        push(
            p,
            json!({ "kind": "kv", "title": "Everything the world log kept", "note": "The row's packed record, unpacked", "pairs": pairs }),
        );
    }
}

/// One packed field as text, an id as its name with its page.
fn more_text(b: &Book, table: &str, k: &str, v: &Value) -> (String, Value) {
    if let Value::Array(xs) = v {
        let parts: Vec<String> = xs.iter().take(40).map(|x| fmt_col(k, x)).collect();
        let tail = if xs.len() > 40 { format!(" … ({} values)", xs.len()) } else { String::new() };
        return (format!("{}{tail}", parts.join(", ")), Value::Null);
    }
    if let Some(t) = b.link_target(table, k)
        && b.table(t).map(id_of) == Some(b.link_key(k, t))
        && let Some(n) = b.name_of(t, v)
    {
        return (n, json!(format!("e/{t}/{}", fmt_id(v))));
    }
    (fmt_col(k, v), Value::Null)
}

/// A nested record as `a__b` → value pairs; lists stay whole.
fn flatten(prefix: &str, v: &Value, out: &mut Vec<(String, Value)>) {
    match v {
        Value::Object(m) => {
            for (k, x) in m {
                let key = if prefix.is_empty() { k.clone() } else { format!("{prefix}__{k}") };
                flatten(&key, x, out);
            }
        }
        _ if prefix.is_empty() => out.push(("value".into(), v.clone())),
        _ => out.push((prefix.to_string(), v.clone())),
    }
}

/// A long list kept as text, cut to its first values and its count.
fn short_list(v: &Value) -> Option<String> {
    let s = v.as_str()?;
    let xs: Vec<Value> = serde_json::from_str(s).ok().filter(|_| s.starts_with('['))?;
    let parts: Vec<String> = xs.iter().map(|x| cell_text(x).unwrap_or_default()).collect();
    if parts.iter().all(|p| p.len() < 12) && parts.len() <= 8 {
        return Some(parts.join(", "));
    }
    let head: Vec<String> = xs.iter().take(6).map(|x| num(x).map_or_else(|| fmt(x), |n| fmt(&json!(n)))).collect();
    Some(format!("{} … ({} values)", head.join(", "), xs.len()))
}

/// List columns that hold references, with the column they match: a
/// roster or captains are places in the people store (`_row`, measured on
/// R4's test book), a staff is coach ids.
fn id_list_target(col: &str) -> Option<(&'static str, &'static str)> {
    match col.rsplit("__").next().unwrap_or(col) {
        "roster" | "captains" => Some(("people", "_row")),
        "position_coaches" | "staff" => Some(("coaches", "id")),
        _ => None,
    }
}

/// A JSON list of numbers kept as text (`[17.5,16.2,...]`).
fn number_list(v: &Value) -> Option<Vec<f64>> {
    let s = v.as_str()?;
    if !s.starts_with('[') {
        return None;
    }
    let a: Vec<Value> = serde_json::from_str(s).ok()?;
    a.iter().map(Value::as_f64).collect()
}

/// The names of a list column's entries: the 67 attributes (ratings and
/// ceilings) and the 15 tendencies, in the engine's order.
fn named_list(col: &str, n: usize) -> Option<Vec<String>> {
    use afm_engine::attributes::{ATTRIBUTE_COUNT, Attr};
    use afm_engine::tendencies::{TEND_COUNT, Tend};
    let last = col.rsplit("__").next().unwrap_or(col);
    match last {
        "attributes" | "ceilings" | "attrs" if n == ATTRIBUTE_COUNT => {
            Some(Attr::ALL.iter().map(|a| a.label().to_string()).collect())
        }
        "v" | "was" | "tendencies" if n == TEND_COUNT && col.contains("tend") => {
            Some(Tend::ALL.iter().map(|t| t.label().to_string()).collect())
        }
        _ => None,
    }
}

/// The Tools page, one tab at a time.
pub fn tools(b: &mut Book, tab: &str, f: &Filters, args: &Value) -> Result<Value, String> {
    let (_, title, code) = TOOLS.iter().find(|t| t.0 == tab).copied().unwrap_or(("issues", "Known issues", "T1"));
    let mut p = new_page(title, &format!("{} · Tools · {title}", world_name(b)));
    p["tabs"] = json!(TOOLS.iter().map(|t| json!({ "slug": t.0, "title": t.1, "code": t.2 })).collect::<Vec<_>>());
    p["tab"] = json!(tab);
    match tab {
        "ids" | "tables" => {
            let mut rows = Vec::new();
            let mut links = Vec::new();
            let names: Vec<String> = b.tables.iter().map(|t| t.name.clone()).collect();
            for n in names {
                if !f.debug && b.is_debug_table(&n) {
                    continue;
                }
                let c = b.count(&n);
                let t = b.table(&n).expect("listed");
                let joins: Vec<String> =
                    t.cols.iter().filter_map(|c| b.link_target(&t.name, c).map(|x| format!("{c} → {x}"))).collect();
                let filt = b.filterable(t);
                rows.push(json!([n, c, t.cols.len(), id_of(t), joins.join(", "), filt.join(", ")]));
                links.push(json!(format!("table/{n}")));
            }
            p["verdict"] = json!(format!(
                "{} tables. Every `<x>_id` column links to the table it names; every name opens its page.",
                rows.len()
            ));
            push(
                &mut p,
                json!({ "kind": "table", "title": "Tables, their ids and joins",
                    "cols": [{"name": "t", "label": "Table"}, {"name": "n", "label": "Rows", "num": true}, {"name": "c", "label": "Columns", "num": true},
                             {"name": "id", "label": "Id"}, {"name": "j", "label": "Joins"}, {"name": "f", "label": "Filters"}],
                    "rows": rows, "links": links }),
            );
        }
        "compare" => {
            p["verdict"] = json!("Two worlds, two years, two groups, or a player against real players like him.");
            push(
                &mut p,
                json!({ "kind": "links", "title": "Compare", "items": [
                    {"text": "Two worlds", "sub": "Before and after a fix, or two seeds", "link": "compare/worlds"},
                    {"text": "Two years", "sub": "The same world then and now", "link": "compare/years"},
                    {"text": "Two groups", "sub": "The SEC against the Big Ten, QBs against RBs", "link": "compare/groups"},
                    {"text": "A player against real players like him", "sub": "Same position, a similar draft slot", "link": "compare/player"}
                ]}),
            );
        }
        "replay" => return crate::play::replay_page(b, p, f, args),
        "play" => return crate::play::play_page(b, p, args),
        "odd" => {
            let page = args["page"].as_str();
            let (items, total) = odd(b, page, None, f, 500);
            p["title"] = json!("Odd things");
            p["verdict"] =
                json!(format!("{total} odd things{}.", page.map(|s| format!(" on {s}")).unwrap_or_default()));
            push(
                &mut p,
                json!({ "kind": "list", "title": "Odd things", "items": items, "more": {"n": total}, "empty": "This book keeps no odd-things list yet." }),
            );
        }
        _ => {
            let kind = if tab == "consistency" && b.find("consistency").is_none() { "checks" } else { tab };
            match b.find(kind).map(|t| t.name.clone()) {
                Some(name) => {
                    let n = b.count(&name);
                    p["verdict"] = json!(format!("{} rows ({code}).", fmt(&json!(n))));
                    let first = b.rows(&name, f, None, None, false, 0, 50)?;
                    push(&mut p, json!({ "kind": "rows", "title": title, "table": name, "first": first }));
                }
                None => {
                    p["verdict"] = json!(format!(
                        "This book has no {} table yet ({code}). The book lane adds it; this tab shows it once a book carries it.",
                        title.to_lowercase()
                    ));
                }
            }
        }
    }
    Ok(p)
}

/// Your career (saves only): your seat, your record, your games, your
/// legacy and the guidelines (N45-N47, S32).
pub fn career(b: &mut Book, f: &Filters) -> Value {
    let name = world_name(b);
    let mut p = new_page("Your career", &format!("{name} · Your career"));
    let prefixes =
        ["seat", "career", "you", "legacy", "desk", "your_", "guidelines", "guide", "staff_book", "kept_reads"];
    let mine: Vec<String> =
        b.tables.iter().filter(|t| prefixes.iter().any(|x| t.name.starts_with(x))).map(|t| t.name.clone()).collect();
    if mine.is_empty() {
        p["verdict"] = json!(
            "This book holds no seat: it was built from a library world. Your career shows in a book built from one of your saves."
        );
        return p;
    }
    let cs: Vec<Check> = checks(b, f).into_iter().filter(|c| c.page == "career").collect();
    p["verdict"] = json!(verdict(&cs, "your career"));
    if !cs.is_empty() {
        let refs: Vec<&Check> = cs.iter().collect();
        push(&mut p, check_table("Checks", &refs));
    }
    for t in mine {
        let n = b.count(&t);
        if n == 0 {
            continue;
        }
        if let Ok(first) = b.rows(&t, f, None, None, false, 0, 50) {
            push(&mut p, json!({ "kind": "rows", "title": label(&t), "table": t, "first": first }));
        }
    }
    p
}
