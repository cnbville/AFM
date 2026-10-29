//! "Copy for Claude": any page as short plain text, so a thread starts from
//! the numbers instead of writing a new reader.

use serde_json::Value;

use crate::book::{Book, Filters, cell_text, id_of};
use crate::pages::fmt;

/// Rows of a table the text keeps; the rest are counted.
const MAX_ROWS: usize = 40;

fn s(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(x) => x.clone(),
        x => fmt(x),
    }
}

fn mark(m: &str) -> &str {
    match m {
        "near" => "near",
        "off" => "off",
        "far" => "FAR OFF",
        "debug" => "debug",
        _ => "",
    }
}

fn table(out: &mut String, cols: &[Value], rows: &[Value], total: Option<i64>) {
    let heads: Vec<String> = cols.iter().map(|c| s(&c["label"]).replace('|', "/")).collect();
    out.push_str(&heads.join(" | "));
    out.push('\n');
    for r in rows.iter().take(MAX_ROWS) {
        let cells: Vec<String> =
            r.as_array().map(|a| a.iter().map(|c| s(c).replace(['\n', '|'], " ")).collect()).unwrap_or_default();
        out.push_str(&cells.join(" | "));
        out.push('\n');
    }
    let n = total.unwrap_or(rows.len() as i64);
    let shown = rows.len().min(MAX_ROWS) as i64;
    if n > shown {
        out.push_str(&format!("… {} more rows\n", fmt(&Value::from(n - shown))));
    }
}

fn block(out: &mut String, b: &Value) {
    let title = s(&b["title"]);
    if !title.is_empty() && b["kind"] != "hero" {
        out.push_str(&format!("\n## {title}\n"));
    }
    if let Some(n) = b["note"].as_str().filter(|n| !n.is_empty()) {
        out.push_str(&format!("({n})\n"));
    }
    match b["kind"].as_str().unwrap_or("") {
        "hero" => {
            let tags: Vec<String> = b["tags"].as_array().map(|a| a.iter().map(s).collect()).unwrap_or_default();
            if !tags.is_empty() {
                out.push_str(&format!("{}\n", tags.join(" · ")));
            }
            if let Some(t) = b["text"].as_str().filter(|t| !t.is_empty()) {
                out.push_str(&format!("{t}\n"));
            }
            let figs: Vec<String> = b["figures"]
                .as_array()
                .map(|a| a.iter().map(|f| format!("{} {}", s(&f["label"]), s(&f["value"]))).collect())
                .unwrap_or_default();
            if !figs.is_empty() {
                out.push_str(&format!("{}\n", figs.join(" · ")));
            }
        }
        "tiles" => {
            for t in b["tiles"].as_array().into_iter().flatten() {
                let m = mark(t["mark"].as_str().unwrap_or(""));
                out.push_str(&format!(
                    "- {}: {} ({}){}\n",
                    s(&t["label"]),
                    s(&t["value"]),
                    s(&t["sub"]),
                    if m.is_empty() { String::new() } else { format!(" [{m}]") }
                ));
            }
        }
        "list" | "links" => {
            let items = b["items"].as_array().cloned().unwrap_or_default();
            if items.is_empty()
                && let Some(e) = b["empty"].as_str()
            {
                out.push_str(&format!("{e}\n"));
            }
            for i in &items {
                let m = mark(i["mark"].as_str().unwrap_or(""));
                let why =
                    [s(&i["why"]), s(&i["sub"])].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(" ");
                out.push_str(&format!(
                    "- {}{}{}\n",
                    s(&i["text"]),
                    if why.is_empty() { String::new() } else { format!(" ({why})") },
                    if m.is_empty() { String::new() } else { format!(" [{m}]") }
                ));
            }
            if let Some(n) = b["more"]["n"].as_i64().filter(|n| *n as usize > items.len()) {
                out.push_str(&format!("… {} in all\n", fmt(&Value::from(n))));
            }
        }
        "table" => {
            let rows = b["rows"].as_array().cloned().unwrap_or_default();
            if rows.is_empty() {
                out.push_str(&format!("{}\n", b["empty"].as_str().unwrap_or("No rows.")));
            } else {
                table(out, b["cols"].as_array().map(Vec::as_slice).unwrap_or(&[]), &rows, None);
            }
        }
        "rows" => {
            let f = &b["first"];
            let cols = f["cols"].as_array().cloned().unwrap_or_default();
            // Linked ids as their names.
            let rows: Vec<Value> = f["rows"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|r| {
                    let cells = r.as_array().cloned().unwrap_or_default();
                    Value::Array(
                        cells
                            .into_iter()
                            .enumerate()
                            .map(|(i, v)| {
                                let col = cols.get(i).and_then(|c| c["name"].as_str()).unwrap_or("");
                                let key = cell_text(&v).unwrap_or_default();
                                match &f["names"][col][&key] {
                                    Value::String(n) => Value::String(n.clone()),
                                    _ if crate::pages::is_year(col) => Value::String(key),
                                    _ => v,
                                }
                            })
                            .collect(),
                    )
                })
                .collect();
            table(out, f["cols"].as_array().map(Vec::as_slice).unwrap_or(&[]), &rows, f["total"].as_i64());
        }
        "replay" => {
            let rows = b["snaps"].as_array().cloned().unwrap_or_default();
            table(out, b["cols"].as_array().map(Vec::as_slice).unwrap_or(&[]), &rows, None);
        }
        "kv" => {
            for p in b["pairs"].as_array().into_iter().flatten() {
                let dbg = if p["debug"].as_bool() == Some(true) { " (debug)" } else { "" };
                out.push_str(&format!("{}: {}{dbg}\n", s(&p["k"]), s(&p["v"])));
            }
        }
        "pair" => {
            for x in b["blocks"].as_array().into_iter().flatten() {
                block(out, x);
            }
        }
        "chart" => {
            for sr in b["series"].as_array().into_iter().flatten() {
                let pts: Vec<String> = sr["points"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|p| format!("{} {}", cell_text(&p[0]).unwrap_or_default(), s(&p[1])))
                    .collect();
                out.push_str(&format!("{}: {}\n", s(&sr["name"]), pts.join(", ")));
            }
        }
        "note" => out.push_str(&format!("{}\n", s(&b["text"]))),
        _ => {}
    }
}

/// The whole page as text: a header line, the filters, the verdict, then
/// each block.
pub fn render(page: &Value, world: &str, filters: &str) -> String {
    let mut out = format!("AFM World Reader · {} · {world}\n", s(&page["title"]));
    if !filters.is_empty() {
        out.push_str(&format!("Filters: {filters}\n"));
    }
    let badges: Vec<String> =
        page["badges"].as_array().map(|a| a.iter().map(|b| s(&b["text"])).collect()).unwrap_or_default();
    if !badges.is_empty() {
        out.push_str(&format!("Data: {}\n", badges.join(" · ")));
    }
    let v = s(&page["verdict"]);
    if !v.is_empty() {
        out.push_str(&format!("{v}\n"));
    }
    for b in page["blocks"].as_array().into_iter().flatten() {
        block(&mut out, b);
    }
    out
}

/// The page of R4's book (`Book::text`) an app route shows, when there is
/// one: the overview, a topic page, a Tools tab the book prints, a table,
/// a person, alumnus, coach, program, school or club, or any other row.
pub fn book_page(b: &Book, route: &str) -> Option<String> {
    if !b.tables.iter().any(|t| t.name == "book_meta") {
        return None;
    }
    let path = route.split('?').next().unwrap_or(route);
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    Some(match parts.as_slice() {
        [] | ["overview"] => "overview".into(),
        ["page", slug] | [slug @ "career"] => slug.to_string(),
        ["tools", tab @ ("issues" | "knobs" | "coverage" | "tables")] => tab.to_string(),
        ["tools", "consistency"] => "checks".into(),
        ["tools", "odd", ..] => "odd".into(),
        ["table", name] => format!("table {name}"),
        ["e", table, id] => {
            let kind = match *table {
                "people" => "person",
                "alumni" => "alumnus",
                "coaches" => "coach",
                "programs" => "program",
                "high_schools" => "school",
                "clubs" | "nfl" => "club",
                _ => "",
            };
            if kind.is_empty() {
                // R4 prints any other row by its place (`_row`).
                let t = b.table(table)?;
                if id_of(t) != "_row" {
                    return None;
                }
                format!("record {table} {id}")
            } else {
                format!("{kind} {id}")
            }
        }
        _ => return None,
    })
}

/// The app's filters as R4's (one value each; the first picked).
pub fn book_filters(f: &Filters) -> afm_reader::Filters {
    let one = |v: &[String]| v.first().cloned();
    afm_reader::Filters {
        level: one(&f.level),
        division: one(&f.division),
        conference: one(&f.conference),
        position: one(&f.position),
        stars: f.stars.first().and_then(|s| s.parse().ok()),
        state: one(&f.state),
        year: f.year,
        year_from: f.from,
        year_to: f.to,
        debug: f.debug,
        ..Default::default()
    }
}

/// The same filters as `afm_read` words (`level=FBS year=2030 debug`).
pub fn filter_words(f: &Filters) -> String {
    let rf = book_filters(f);
    let mut w: Vec<String> = Vec::new();
    for (k, v) in [
        ("level", rf.level),
        ("division", rf.division),
        ("conference", rf.conference),
        ("pos", rf.position),
        ("state", rf.state),
    ] {
        if let Some(v) = v {
            w.push(format!("{k}={v}"));
        }
    }
    for (k, v) in [("stars", rf.stars), ("year", rf.year), ("from", rf.year_from), ("to", rf.year_to)] {
        if let Some(v) = v {
            w.push(format!("{k}={v}"));
        }
    }
    if rf.debug {
        w.push("debug".into());
    }
    w.join(" ")
}
