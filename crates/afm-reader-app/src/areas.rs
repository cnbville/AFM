//! The topic pages' areas (lane R6, for every page lane): each inventory
//! area with the spec's four layers. First its verdict in plain words,
//! then its checks against real as tiles, then its panels and
//! distributions (worked out when the book was built), last the rows and
//! pages behind every number.
//!
//! - `page/<topic>`: the page's verdict, the worst tiles, every area's
//!   header with its verdict and tiles, and the odd things.
//! - `area/<code>`: one area whole.

use serde_json::{Map, Value, json};

use crate::book::{Book, Filters, cell_text, q};
use crate::pages::{self, Check, TOPICS, fmt, new_page, push, world_name};

/// One inventory area as the book's coverage map keeps it.
#[derive(Clone, Default)]
pub struct Area {
    pub code: String,
    pub page: String,
    pub title: String,
    pub status: String,
    pub tables: Vec<String>,
    pub shows: String,
    pub needs_new: Vec<String>,
    pub note: String,
}

/// Inventory codes in order: P1, P2 ... P32, then C, S, G, N, T.
fn order(code: &str) -> (usize, u32) {
    let l = code.chars().next().unwrap_or('X');
    let n = code[1..].parse().unwrap_or(0);
    ("PCSGNT".find(l).unwrap_or(9), n)
}

/// The areas of a topic page, in order, from the book's coverage map.
pub fn areas(b: &Book, slug: &str) -> Vec<Area> {
    let Some(t) = b.find("coverage") else { return Vec::new() };
    if !t.has("code") {
        return Vec::new();
    }
    let rows = b.select_maps(&format!("SELECT * FROM {}", q(&t.name)), &[]).unwrap_or_default();
    let mut out: Vec<Area> = rows
        .iter()
        .map(|m| {
            let s = |k: &str| m.get(k).and_then(cell_text).unwrap_or_default();
            Area {
                code: s("code"),
                page: s("page"),
                title: s("title"),
                status: s("status"),
                tables: s("tables").split_whitespace().map(str::to_string).collect(),
                shows: s("shows"),
                needs_new: serde_json::from_str(&s("needs_new")).unwrap_or_default(),
                note: s("note"),
            }
        })
        .filter(|a| a.page == slug || pages::page_of_area(&a.code) == slug)
        .collect();
    out.sort_by_key(|a| order(&a.code));
    out
}

/// The checks touching an area (any of their areas), worst first.
fn area_checks(b: &Book, all: &[Check], code: &str) -> Vec<Check> {
    let Some(t) = b.find("checks") else { return Vec::new() };
    let Some(ac) = t.pick(&["areas", "area"]) else {
        return all.iter().filter(|c| c.area == code).cloned().collect();
    };
    let sql = format!(
        "SELECT id FROM {} WHERE (' ' || REPLACE(REPLACE({}, ',', ' '), ';', ' ') || ' ') LIKE ?1",
        q(&t.name),
        q(ac)
    );
    let ids: Vec<String> = b
        .select_maps(&sql, &[rusqlite::types::Value::Text(format!("% {code} %"))])
        .unwrap_or_default()
        .iter()
        .filter_map(|m| m.get("id").and_then(cell_text))
        .collect();
    let mut cs: Vec<Check> = all.iter().filter(|c| ids.contains(&c.id) || c.area == code).cloned().collect();
    cs.sort_by_key(|c| std::cmp::Reverse(sev(c)));
    cs
}

fn sev(c: &Check) -> u8 {
    match c.mark.as_str() {
        "far" => 3,
        "off" => 2,
        "near" => 1,
        _ => 0,
    }
}

/// A panel as the book keeps it.
#[derive(Clone, Default)]
pub struct PanelRow {
    pub id: String,
    pub area: String,
    pub title: String,
    pub kind: String,
    pub note: String,
    pub needs: String,
    pub open: String,
    pub cols: Vec<String>,
    pub rows: Vec<Vec<Value>>,
    pub more: i64,
    pub says: String,
    pub status: String,
    pub sql: String,
}

/// Columns a panel's rows are filtered on, as the filters bar names them.
const PANEL_FILTERS: [(&str, &str); 6] = [
    ("level", "level"),
    ("division", "division"),
    ("conference", "conference"),
    ("position", "position"),
    ("stars", "stars"),
    ("state", "state"),
];

pub fn panels(b: &Book, area: Option<&str>, page: Option<&str>) -> Vec<PanelRow> {
    let Some(t) = b.table("panels") else { return Vec::new() };
    let (wh, v) = match (area, page) {
        (Some(a), _) => (" WHERE area = ?1", vec![rusqlite::types::Value::Text(a.into())]),
        (None, Some(p)) => (" WHERE page = ?1", vec![rusqlite::types::Value::Text(p.into())]),
        _ => ("", Vec::new()),
    };
    let rows = b.select_maps(&format!("SELECT * FROM {}{wh} ORDER BY rowid", q(&t.name)), &v).unwrap_or_default();
    rows.iter().map(panel_of).collect()
}

fn panel_of(m: &Map<String, Value>) -> PanelRow {
    let s = |k: &str| m.get(k).and_then(cell_text).unwrap_or_default();
    PanelRow {
        id: s("id"),
        area: s("area"),
        title: s("title"),
        kind: s("kind"),
        note: s("note"),
        needs: s("needs"),
        open: s("open"),
        cols: serde_json::from_str(&s("cols")).unwrap_or_default(),
        rows: serde_json::from_str(&s("rows")).unwrap_or_default(),
        more: m.get("more").and_then(Value::as_i64).unwrap_or(0),
        says: s("says"),
        status: s("status"),
        sql: s("sql"),
    }
}

/// Holds hidden truths: shown only with the switch on.
fn is_debug(p: &PanelRow) -> bool {
    p.title.contains("(debug)") || p.cols.iter().any(|c| crate::book::debug_name(c))
}

/// A panel as a block: its rows under the filters bar's choices.
pub fn panel_block(p: &PanelRow, f: &Filters) -> Value {
    let mut rows = p.rows.clone();
    for (key, col) in PANEL_FILTERS {
        let want: &Vec<String> = match key {
            "level" => &f.level,
            "division" => &f.division,
            "conference" => &f.conference,
            "position" => &f.position,
            "stars" => &f.stars,
            _ => &f.state,
        };
        if want.is_empty() {
            continue;
        }
        if let Some(i) = p.cols.iter().position(|c| c == col) {
            rows.retain(|r| {
                r.get(i).and_then(cell_text).is_some_and(|v| want.iter().any(|w| w.eq_ignore_ascii_case(&v)))
            });
        }
    }
    if let (Some(i), true) = (p.cols.iter().position(|c| c == "year"), f.from.is_some() || f.to.is_some()) {
        rows.retain(|r| {
            // A line over ages or ratings keeps its `year` axis whole.
            let y = r.get(i).and_then(Value::as_i64).unwrap_or(0);
            !(1800..=2300).contains(&y) || f.from.is_none_or(|a| y >= a) && f.to.is_none_or(|z| y <= z)
        });
    }
    let unit = p.note.split("Unit: ").nth(1).and_then(|u| u.split(['.', ';']).next()).unwrap_or("").trim().to_string();
    json!({
        "kind": "panel", "pkind": p.kind, "id": p.id, "code": p.area, "title": p.title, "note": p.note,
        "needs": p.needs, "open": p.open, "cols": p.cols, "rows": rows, "more": p.more, "says": p.says,
        "status": p.status, "sql": p.sql, "unit": unit,
    })
}

/// An area's verdict: its checks' marks and the furthest, then what its
/// panels say, else what the coverage map says.
fn area_verdict(a: &Area, cs: &[Check], ps: &[PanelRow]) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !cs.is_empty() {
        let n = |m: &str| cs.iter().filter(|c| c.mark == m).count();
        let mut s = format!(
            "{} check{} against real: {} near, {} off, {} far off.",
            cs.len(),
            if cs.len() == 1 { "" } else { "s" },
            n("near"),
            n("off"),
            n("far")
        );
        if let Some(c) = cs.iter().find(|c| c.mark == "far" || c.mark == "off") {
            s.push_str(&format!(" {}.", c.why.trim_end_matches('.')));
        }
        parts.push(s);
    }
    for p in ps.iter().filter(|p| !p.says.is_empty()).take(3) {
        parts.push(p.says.clone());
    }
    if parts.is_empty() {
        parts.push(match a.status.as_str() {
            "not stored" => {
                "The world keeps none of this yet; the list below says what would have to be stored.".into()
            }
            "needs the log" => format!("This book has no world log, so this area is empty ({}).", a.note),
            _ if !ps.is_empty() => {
                "No real figure to set this against yet: the tables below show the world as it is.".into()
            }
            _ => "The data is in the book; no check or panel reads it yet.".into(),
        });
    }
    parts.join(" ")
}

fn marks_of(cs: &[Check]) -> Value {
    let n = |m: &str| cs.iter().filter(|c| c.mark == m).count();
    json!({ "near": n("near"), "off": n("off"), "far": n("far") })
}

fn area_block(a: &Area, cs: &[Check], ps: &[PanelRow], link: bool) -> Value {
    json!({
        "kind": "area", "code": a.code, "title": a.title, "status": a.status, "verdict": area_verdict(a, cs, ps),
        "shows": a.shows, "missing": a.needs_new, "marks": marks_of(cs),
        "link": if link { json!(format!("area/{}", a.code)) } else { Value::Null },
    })
}

fn tiles(cs: &[Check]) -> Value {
    json!({ "kind": "tiles", "note": pages::MARK_RULE, "tiles": cs.iter().map(Check::tile).collect::<Vec<_>>() })
}

/// A topic page made of its areas (People, Recruiting and every page whose
/// book carries panels).
pub fn topic(b: &mut Book, slug: &str, f: &Filters) -> Result<Value, String> {
    let (title, codes) = TOPICS.iter().find(|t| t.0 == slug).map(|t| (t.1, t.2)).unwrap_or(("Tools", ""));
    let all_areas = areas(b, slug);
    let ps_all: Vec<PanelRow> = panels(b, None, Some(slug)).into_iter().filter(|x| f.debug || !is_debug(x)).collect();
    // A page no lane has given panels yet keeps the shell's page.
    if all_areas.is_empty() || ps_all.is_empty() {
        return pages::topic(b, slug, f);
    }
    let name = world_name(b);
    let mut p = new_page(title, &format!("{name} · {title}"));
    let all = pages::checks(b, f);
    let page_checks: Vec<Check> = all.iter().filter(|c| c.page == slug).cloned().collect();
    let mut verdict = pages::verdict(&page_checks, &title.to_lowercase());
    let stored = all_areas.iter().filter(|a| a.status != "not stored").count();
    verdict.push_str(&format!(
        " {} areas ({codes}): {stored} with data in this book, {} panels worked out.",
        all_areas.len(),
        ps_all.iter().filter(|x| x.status == "ok").count()
    ));
    p["verdict"] = json!(verdict);
    p["badges"] = json!(pages::coverage_badges(b));
    let mut worst: Vec<&Check> = page_checks.iter().filter(|c| c.mark == "far" || c.mark == "off").collect();
    worst.sort_by_key(|c| std::cmp::Reverse(sev(c)));
    worst.truncate(8);
    if !worst.is_empty() {
        push(
            &mut p,
            json!({ "kind": "tiles", "title": "Furthest from real", "note": pages::MARK_RULE,
                    "tiles": worst.iter().map(|c| c.tile()).collect::<Vec<_>>() }),
        );
    }
    // The index: every area with its status and marks.
    let mut rows = Vec::new();
    let mut links = Vec::new();
    let mut per: Vec<(Area, Vec<Check>, Vec<PanelRow>)> = Vec::new();
    for a in &all_areas {
        let cs = area_checks(b, &all, &a.code);
        let ps: Vec<PanelRow> = ps_all.iter().filter(|x| x.area == a.code).cloned().collect();
        let m = marks_of(&cs);
        rows.push(json!([a.code, a.title, a.status, cs.len(), m["near"], m["off"], m["far"], ps.len()]));
        links.push(json!(format!("area/{}", a.code)));
        per.push((a.clone(), cs, ps));
    }
    push(
        &mut p,
        json!({
            "kind": "table", "title": "Every area on this page",
            "cols": [{"name": "code", "label": "Area"}, {"name": "title", "label": "What"}, {"name": "status", "label": "In this book"},
                     {"name": "checks", "label": "Checks", "num": true}, {"name": "near", "label": "Near", "num": true, "tone": "near"},
                     {"name": "off", "label": "Off", "num": true, "tone": "off"}, {"name": "far", "label": "Far off", "num": true, "tone": "far"},
                     {"name": "panels", "label": "Panels", "num": true}],
            "rows": rows, "links": links
        }),
    );
    let (items, total) = pages::odd(b, Some(slug), None, f, 10);
    if total > 0 {
        push(
            &mut p,
            json!({ "kind": "list", "title": "Odd things", "items": items, "more": {"n": total, "link": format!("tools/odd/{slug}")} }),
        );
    }
    // Each area's verdict and tiles; its page has the panels.
    for (a, cs, ps) in &per {
        push(&mut p, area_block(a, cs, ps, true));
        if !cs.is_empty() {
            push(&mut p, tiles(cs));
        }
    }
    Ok(p)
}

/// One area whole: verdict, tiles, panels, distributions, the rows behind it.
pub fn area(b: &mut Book, code: &str, f: &Filters) -> Result<Value, String> {
    let code = code.to_ascii_uppercase();
    let slug = pages::page_of_area(&code);
    let list = areas(b, slug);
    let i = list
        .iter()
        .position(|a| a.code == code)
        .ok_or_else(|| format!("This book's coverage map has no area {code}."))?;
    let a = list[i].clone();
    let page_title = TOPICS.iter().find(|t| t.0 == slug).map_or("Tools", |t| t.1);
    let mut p = new_page(&format!("{code} · {}", a.title), &format!("{} · {page_title} · {code}", world_name(b)));
    let all = pages::checks(b, f);
    let cs = area_checks(b, &all, &code);
    let ps: Vec<PanelRow> = panels(b, Some(&code), None).into_iter().filter(|x| f.debug || !is_debug(x)).collect();
    p["verdict"] = json!(area_verdict(&a, &cs, &ps));
    let mut badges = vec![json!({ "text": format!("{page_title} page"), "ok": true })];
    badges.push(json!({ "text": format!("In this book: {}", a.status), "ok": a.status == "checked" }));
    p["badges"] = json!(badges);
    push(&mut p, area_block(&a, &cs, &ps, false));
    if !cs.is_empty() {
        push(&mut p, tiles(&cs));
    }
    for x in &ps {
        push(&mut p, panel_block(x, f));
    }
    // Distributions the book keeps for this area: each metric's spread by
    // its groups.
    for m in metrics_of(b, &code) {
        if !f.debug && (m.0.contains("potential") || m.1.to_lowercase().contains("(debug)")) {
            continue;
        }
        if let Some(block) = dist_block(b, &m.0, &m.1, &m.2) {
            push(&mut p, block);
        }
    }
    // The rows behind it.
    let behind: Vec<String> =
        a.tables.iter().filter(|t| b.table(t).is_some() && (f.debug || !b.is_debug_table(t))).cloned().collect();
    let mut items: Vec<Value> = Vec::new();
    for t in &behind {
        let n = b.count(t);
        items.push(
            json!({ "text": crate::book::label(t), "sub": format!("{} rows", fmt(&json!(n))), "link": format!("table/{t}") }),
        );
    }
    if !items.is_empty() {
        push(&mut p, json!({ "kind": "links", "title": "The rows behind it", "items": items }));
    }
    let nav: Vec<Value> = [i.checked_sub(1), Some(i + 1)]
        .into_iter()
        .flatten()
        .filter_map(|j| list.get(j))
        .map(|x| json!({ "text": format!("{} · {}", x.code, x.title), "link": format!("area/{}", x.code) }))
        .chain(std::iter::once(json!({ "text": format!("The {page_title} page"), "link": format!("page/{slug}") })))
        .collect();
    push(&mut p, json!({ "kind": "links", "title": "Next", "items": nav }));
    Ok(p)
}

/// The metrics filed under an area: (id, title, unit).
fn metrics_of(b: &Book, code: &str) -> Vec<(String, String, String)> {
    let Some(t) = b.find("metrics") else { return Vec::new() };
    if !t.has("areas") {
        return Vec::new();
    }
    let sql = format!("SELECT id, title, unit FROM {} WHERE (' ' || areas || ' ') LIKE ?1 AND rows > 0", q(&t.name));
    b.select_maps(&sql, &[rusqlite::types::Value::Text(format!("% {code} %"))])
        .unwrap_or_default()
        .iter()
        .map(|m| {
            let s = |k: &str| m.get(k).and_then(cell_text).unwrap_or_default();
            (s("id"), s("title"), s("unit"))
        })
        .collect()
}

/// A metric's spread by group (all years) as a panel table: n, mean, sd
/// and the 10th, 50th and 90th percentiles.
fn dist_block(b: &Book, id: &str, title: &str, unit: &str) -> Option<Value> {
    let t = b.find("dist")?;
    let sql = format!(
        "SELECT grp_kind AS grp, grp AS label, n, mean, sd, p10, p50, p90 FROM {} WHERE metric = ?1 AND year IS NULL
         ORDER BY grp_kind, n DESC",
        q(&t.name)
    );
    let rows = b.select_maps(&sql, &[rusqlite::types::Value::Text(id.into())]).ok()?;
    if rows.is_empty() {
        return None;
    }
    let cols = ["grp", "label", "n", "mean", "sd", "p10", "p50", "p90"];
    let data: Vec<Value> = rows
        .iter()
        .map(|m| Value::Array(cols.iter().map(|c| m.get(*c).cloned().unwrap_or(Value::Null)).collect()))
        .collect();
    Some(json!({
        "kind": "panel", "pkind": "table", "id": id, "title": format!("{title}: spread by group"),
        "note": format!("The book's distribution {id}, every year together{}. Pick a group above the table.", if unit.is_empty() { String::new() } else { format!("; unit: {unit}") }),
        "cols": cols, "rows": data, "status": "ok", "unit": unit,
    }))
}

/// "Copy for Claude" for the blocks this module adds.
pub fn text_block(out: &mut String, b: &Value) {
    let s = |v: &Value| match v {
        Value::Null => String::new(),
        Value::String(x) => x.clone(),
        x => fmt(x),
    };
    match b["kind"].as_str().unwrap_or("") {
        "area" => {
            out.push_str(&format!(
                "\n# {} {} [{}]\n{}\n",
                s(&b["code"]),
                s(&b["title"]),
                s(&b["status"]),
                s(&b["verdict"])
            ));
        }
        "panel" => {
            out.push_str(&format!("\n## {} ({}, {})\n", s(&b["title"]), s(&b["code"]), s(&b["pkind"])));
            if !s(&b["says"]).is_empty() {
                out.push_str(&format!("{}\n", s(&b["says"])));
            }
            let cols: Vec<String> = b["cols"].as_array().map(|a| a.iter().map(s).collect()).unwrap_or_default();
            out.push_str(&format!("{}\n", cols.join(" | ")));
            let rows = b["rows"].as_array().cloned().unwrap_or_default();
            for r in rows.iter().take(60) {
                let cells: Vec<String> = r
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .enumerate()
                            .map(|(i, v)| match (cols.get(i).map(String::as_str), v.as_i64()) {
                                (Some("year" | "class" | "season"), Some(y)) => y.to_string(),
                                _ => s(v).replace(['\n', '|'], " "),
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                out.push_str(&format!("{}\n", cells.join(" | ")));
            }
            if rows.len() > 60 {
                out.push_str(&format!("… {} more rows\n", rows.len() - 60));
            }
        }
        "timeline" => {
            for i in b["items"].as_array().into_iter().flatten() {
                let why = s(&i["why"]);
                out.push_str(&format!(
                    "- {}: {}{}\n",
                    s(&i["when"]),
                    s(&i["text"]),
                    if why.is_empty() { String::new() } else { format!(" ({why})") }
                ));
            }
        }
        "meters" => {
            for g in b["groups"].as_array().into_iter().flatten() {
                let items: Vec<String> = g["items"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|m| format!("{} {}", s(&m["k"]), s(&m["v"])))
                    .collect();
                out.push_str(&format!("{}: {}\n", s(&g["name"]), items.join(", ")));
            }
        }
        _ => {}
    }
}

/// An entity page made of panels filed under `page` (`class`, `state`):
/// each panel's rows for this one key (its `key` column), and the panels
/// with no such column whole, as the league's frame around it.
pub fn keyed(b: &mut Book, page: &str, key: &str, val: &str, f: &Filters) -> Result<Value, String> {
    let ps: Vec<PanelRow> = panels(b, None, Some(page)).into_iter().filter(|x| f.debug || !is_debug(x)).collect();
    if ps.is_empty() {
        return Err(format!("This book has no {page} pages yet: build it with a reader from lane R6 on."));
    }
    let (title, crumb) = match page {
        "class" => (format!("The class of {val}"), "Recruiting classes"),
        "state" => (format!("{} ({val})", state_name(val)), "States"),
        _ => (val.to_string(), page),
    };
    let mut p = new_page(&title, &format!("{} · {crumb} · {title}", world_name(b)));
    let mut says: Vec<String> = Vec::new();
    let mut blocks: Vec<Value> = Vec::new();
    for x in &ps {
        let mut x = x.clone();
        if let Some(i) = x.cols.iter().position(|c| c == key) {
            x.rows.retain(|r| r.get(i).and_then(cell_text).is_some_and(|v| v.eq_ignore_ascii_case(val)));
            if x.rows.is_empty() {
                continue;
            }
            // The words line for this one key: a `says` column on its first row.
            if let Some(si) = x.cols.iter().position(|c| c == "says")
                && let Some(t) = x.rows.first().and_then(|r| r.get(si)).and_then(cell_text)
            {
                says.push(t);
            }
            // Every row now has the same key: the column says nothing more.
            if x.kind != "map" {
                if x.open.contains(&format!("{{{key}}}")) {
                    x.open.clear();
                }
                x.cols.remove(i);
                for r in &mut x.rows {
                    if i < r.len() {
                        r.remove(i);
                    }
                }
            }
        }
        blocks.push(panel_block(&x, f));
    }
    p["verdict"] =
        json!(if says.is_empty() { format!("{title}: every panel below is this one's rows.") } else { says.join(" ") });
    push(&mut p, json!({ "kind": "hero", "label": crumb, "title": title, "tags": [], "figures": [] }));
    for blk in blocks {
        push(&mut p, blk);
    }
    Ok(p)
}

/// A state's name from its two letters.
pub fn state_name(code: &str) -> &'static str {
    const NAMES: [(&str, &str); 51] = [
        ("AL", "Alabama"),
        ("AK", "Alaska"),
        ("AZ", "Arizona"),
        ("AR", "Arkansas"),
        ("CA", "California"),
        ("CO", "Colorado"),
        ("CT", "Connecticut"),
        ("DE", "Delaware"),
        ("DC", "District of Columbia"),
        ("FL", "Florida"),
        ("GA", "Georgia"),
        ("HI", "Hawaii"),
        ("ID", "Idaho"),
        ("IL", "Illinois"),
        ("IN", "Indiana"),
        ("IA", "Iowa"),
        ("KS", "Kansas"),
        ("KY", "Kentucky"),
        ("LA", "Louisiana"),
        ("ME", "Maine"),
        ("MD", "Maryland"),
        ("MA", "Massachusetts"),
        ("MI", "Michigan"),
        ("MN", "Minnesota"),
        ("MS", "Mississippi"),
        ("MO", "Missouri"),
        ("MT", "Montana"),
        ("NE", "Nebraska"),
        ("NV", "Nevada"),
        ("NH", "New Hampshire"),
        ("NJ", "New Jersey"),
        ("NM", "New Mexico"),
        ("NY", "New York"),
        ("NC", "North Carolina"),
        ("ND", "North Dakota"),
        ("OH", "Ohio"),
        ("OK", "Oklahoma"),
        ("OR", "Oregon"),
        ("PA", "Pennsylvania"),
        ("RI", "Rhode Island"),
        ("SC", "South Carolina"),
        ("SD", "South Dakota"),
        ("TN", "Tennessee"),
        ("TX", "Texas"),
        ("UT", "Utah"),
        ("VT", "Vermont"),
        ("VA", "Virginia"),
        ("WA", "Washington"),
        ("WV", "West Virginia"),
        ("WI", "Wisconsin"),
        ("WY", "Wyoming"),
    ];
    NAMES.iter().find(|(c, _)| c.eq_ignore_ascii_case(code)).map_or("State", |(_, n)| n)
}
