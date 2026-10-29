//! Compare (T12): two worlds (before and after a fix, or two seeds), two
//! years of one world, two groups (the SEC against the Big Ten), and a player
//! against real players like him.

use serde_json::{Value, json};

use crate::book::{Book, Filters, YEAR_COLS, cell_text, json_sql, label, q};
use crate::pages::{Check, checks, fmt, fmt_col, new_page, num, push, world_name};

/// Whether B moved toward real from A: "closer", "further", "same", or "".
fn moved(a: &Value, b: &Value, real: &Value) -> &'static str {
    match (num(a), num(b), num(real)) {
        (Some(a), Some(b), Some(r)) => {
            let (da, db) = ((a - r).abs(), (b - r).abs());
            let scale = r.abs().max(1e-9);
            if ((da - db) / scale).abs() < 0.005 {
                "same"
            } else if db < da {
                "closer"
            } else {
                "further"
            }
        }
        (Some(a), Some(b), None) if (a - b).abs() < 1e-9 => "same",
        _ => "",
    }
}

/// The "what moved" table between two sets of checks, matched by id.
pub fn moved_block(title: &str, a: &[Check], b: &[Check]) -> Value {
    let mut rows = Vec::new();
    let mut tones = Vec::new();
    let (mut closer, mut further) = (0, 0);
    for nb in b {
        let Some(na) = a.iter().find(|c| c.id == nb.id && !c.id.is_empty()) else { continue };
        let m = moved(&na.world, &nb.world, &nb.real);
        if m == "same" {
            continue;
        }
        match m {
            "closer" => closer += 1,
            "further" => further += 1,
            _ => {}
        }
        rows.push(json!([nb.label, nb.area, fmt(&na.world), fmt(&nb.world), fmt(&nb.real), m]));
        tones.push(json!(m));
    }
    let same = b
        .iter()
        .filter(|nb| a.iter().any(|na| na.id == nb.id && moved(&na.world, &nb.world, &nb.real) == "same"))
        .count();
    json!({
        "kind": "table", "title": title,
        "note": format!("{closer} closer to real, {further} further, {same} the same"),
        "cols": [{"name": "m", "label": "Measure"}, {"name": "a", "label": "Area"}, {"name": "x", "label": "A", "num": true}, {"name": "y", "label": "B", "num": true},
                 {"name": "r", "label": "Real", "num": true}, {"name": "c", "label": "Change", "moved": true}],
        "rows": rows, "empty": "No check moved."
    })
}

/// A build card for one world: commit, dates, options, switches.
fn build_card(b: &Book, tag: &str) -> Value {
    let m = crate::pages::meta(b);
    let mut pairs = vec![json!({ "k": "World", "v": world_name(b) })];
    for k in [
        "seed",
        "history_years",
        "commit",
        "built",
        "written",
        "scale",
        "engine_from",
        "hs_engine_years",
        "yearly_growth",
        "names",
        "switches",
    ] {
        if let Some(v) = m.get(k).filter(|v| !v.is_null()) {
            pairs.push(json!({ "k": label(k), "v": fmt(v) }));
        }
    }
    json!({ "kind": "kv", "title": format!("World {tag}"), "pairs": pairs })
}

/// Two worlds side by side.
pub fn worlds(a: &Book, b: &Book, f: &Filters) -> Value {
    let mut p = new_page("Compare worlds", "Compare worlds");
    let (ca, cb) = (checks(a, f), checks(b, f));
    let block = moved_block("What moved", &ca, &cb);
    let note = block["note"].as_str().unwrap_or("").to_string();
    p["verdict"] = json!(format!("{} against {}: {note}.", world_name(b), world_name(a)));
    let warn = fairness(a, b);
    if !warn.is_empty() {
        p["badges"] = json!(warn.iter().map(|w| json!({ "text": w, "ok": false })).collect::<Vec<_>>());
    }
    push(&mut p, json!({ "kind": "pair", "blocks": [build_card(a, "A"), build_card(b, "B")] }));
    push(&mut p, block);
    p
}

/// What makes two worlds' counts unfair to set side by side.
fn fairness(a: &Book, b: &Book) -> Vec<String> {
    let (ma, mb) = (crate::pages::meta(a), crate::pages::meta(b));
    let mut w = Vec::new();
    for k in ["scale", "history_years", "engine_from", "hs_engine_years", "yearly_growth", "names"] {
        let (x, y) = (ma.get(k).and_then(cell_text), mb.get(k).and_then(cell_text));
        if x.is_some() && y.is_some() && x != y {
            w.push(format!("{} differs: {} against {}", label(k), x.unwrap_or_default(), y.unwrap_or_default()));
        }
    }
    w
}

/// Numeric columns of one table under two conditions, side by side.
fn side_by_side(b: &Book, table: &str, conds: [(String, Vec<rusqlite::types::Value>); 2], names: [&str; 2]) -> Value {
    let mut rows = Vec::new();
    for c in b.numeric_cols(table) {
        if b.is_debug(table, &c) {
            continue;
        }
        let s: Vec<Value> = conds.iter().map(|(wh, v)| b.stats(table, &c, wh, v)).collect();
        let f = |v: &Value| fmt_col(&c, v);
        rows.push(json!([
            label(&c),
            f(&s[0]["mean"]),
            f(&s[1]["mean"]),
            format!("{} / {} / {}", f(&s[0]["p10"]), f(&s[0]["p50"]), f(&s[0]["p90"])),
            format!("{} / {} / {}", f(&s[1]["p10"]), f(&s[1]["p50"]), f(&s[1]["p90"])),
            s[0]["n"],
            s[1]["n"],
        ]));
    }
    json!({
        "kind": "table", "title": label(table),
        "cols": [{"name": "m", "label": "Measure"}, {"name": "a", "label": format!("{} mean", names[0]), "num": true}, {"name": "b", "label": format!("{} mean", names[1]), "num": true},
                 {"name": "ap", "label": format!("{} p10/p50/p90", names[0])}, {"name": "bp", "label": format!("{} p10/p50/p90", names[1])},
                 {"name": "an", "label": format!("{} rows", names[0]), "num": true}, {"name": "bn", "label": format!("{} rows", names[1]), "num": true}],
        "rows": rows
    })
}

fn and(wh: &str, cond: &str) -> String {
    if wh.is_empty() { format!(" WHERE {cond}") } else { format!("{wh} AND {cond}") }
}

/// Tables a compare can pick from: with a year column (two years) or a
/// filter column (two groups).
fn pickable(b: &Book, need_year: bool) -> Vec<Value> {
    b.tables
        .iter()
        .filter(|t| !b.is_debug_table(&t.name))
        .filter(|t| if need_year { t.pick(&YEAR_COLS).is_some() } else { true })
        .map(|t| json!({ "name": t.name, "cols": t.cols }))
        .collect()
}

/// The same world in two years: one table's numbers each year.
pub fn years(b: &Book, f: &Filters, args: &Value) -> Value {
    let mut p = new_page("Compare two years", &format!("{} · Compare two years", world_name(b)));
    p["pick"] = json!({ "mode": "years", "tables": pickable(b, true) });
    let (Some(table), Some(y1), Some(y2)) = (args["table"].as_str(), args["y1"].as_i64(), args["y2"].as_i64()) else {
        p["verdict"] = json!("Pick a table and two years.");
        return p;
    };
    let Some(t) = b.table(table) else {
        p["verdict"] = json!(format!("This book has no table {table}."));
        return p;
    };
    let Some(yc) = t.pick(&YEAR_COLS) else {
        p["verdict"] = json!(format!("{table} has no year column."));
        return p;
    };
    let mut g = f.clone();
    g.year = None;
    g.from = None;
    g.to = None;
    let (wh, vals) = b.where_for(t, &g);
    let cond = |y: i64| {
        let mut v = vals.clone();
        v.push(rusqlite::types::Value::Integer(y));
        (and(&wh, &format!("{} = ?{}", q(yc), v.len())), v)
    };
    p["verdict"] = json!(format!("{} in {y1} against {y2}.", label(table)));
    let (a, bb) = (y1.to_string(), y2.to_string());
    push(&mut p, side_by_side(b, table, [cond(y1), cond(y2)], [&a, &bb]));
    p
}

/// Two groups of one table: a column's two values.
pub fn groups(b: &Book, f: &Filters, args: &Value) -> Value {
    let mut p = new_page("Compare two groups", &format!("{} · Compare two groups", world_name(b)));
    p["pick"] = json!({ "mode": "groups", "tables": pickable(b, false) });
    let (Some(table), Some(col)) = (args["table"].as_str(), args["col"].as_str()) else {
        p["verdict"] = json!("Pick a table, a column and its two values.");
        return p;
    };
    let Some(t) = b.table(table).filter(|t| t.has(col)) else {
        p["verdict"] = json!(format!("{table} has no column {col}."));
        return p;
    };
    let (g1, g2) = (cell_text(&args["g1"]).unwrap_or_default(), cell_text(&args["g2"]).unwrap_or_default());
    let (wh, vals) = b.where_for(t, f);
    let cond = |g: &str| {
        let mut v = vals.clone();
        v.push(rusqlite::types::Value::Text(g.to_string()));
        (and(&wh, &format!("CAST({} AS TEXT) COLLATE NOCASE = ?{}", q(col), v.len())), v)
    };
    p["verdict"] = json!(format!("{} by {}: {g1} against {g2}.", label(table), label(col)));
    push(&mut p, side_by_side(b, table, [cond(&g1), cond(&g2)], [&g1, &g2]));
    p
}

/// A player against real players like him: the same position and a draft
/// slot within 8 picks, from the book's real careers.
pub fn player(b: &Book, args: &Value) -> Value {
    let mut p = new_page("A player against real players like him", "Compare a player");
    let Some(id) = args.get("id").filter(|v| !v.is_null()) else {
        p["verdict"] = json!("Search a player, then pick \"Players like him\" on his page.");
        return p;
    };
    let Some(people) = b.find("people").map(|t| t.name.clone()) else {
        p["verdict"] = json!("This book has no people table.");
        return p;
    };
    let row = b
        .select_maps(&format!("SELECT * FROM {} WHERE id = ?1", q(&people)), &[json_sql(id)])
        .unwrap_or_default()
        .pop()
        .unwrap_or_default();
    let name = b.name_of(&people, id).unwrap_or_default();
    let pos = ["position", "pos"].iter().find_map(|k| row.get(*k).and_then(cell_text)).unwrap_or_default();
    let pick = ["draft_pick", "pick", "overall_pick"].iter().find_map(|k| row.get(*k).and_then(|v| v.as_i64()));
    p["title"] = json!(format!("{name} against real players like him"));
    let Some(real) = b.find("real_players") else {
        p["verdict"] = json!(
            "This book carries no real careers yet (the real index's nfl_careers). The book lane adds them; this compare reads them then."
        );
        return p;
    };
    let (Some(pc), Some(kc)) = (real.pick(&["position", "pos"]), real.pick(&["draft_pick", "pick", "overall_pick"]))
    else {
        p["verdict"] = json!("The real careers table has no position or draft pick.");
        return p;
    };
    let mut vals = vec![rusqlite::types::Value::Text(pos.clone())];
    let mut wh = format!(" WHERE {} = ?1", q(pc));
    if let Some(k) = pick {
        vals.push(rusqlite::types::Value::Integer(k - 8));
        vals.push(rusqlite::types::Value::Integer(k + 8));
        wh.push_str(&format!(" AND {} BETWEEN ?2 AND ?3", q(kc)));
    }
    let rn = real.name.clone();
    let n: i64 =
        b.db.query_row(&format!("SELECT COUNT(*) FROM {}{wh}", q(&rn)), rusqlite::params_from_iter(vals.iter()), |r| {
            r.get(0)
        })
        .unwrap_or(0);
    p["verdict"] = json!(match pick {
        Some(k) => format!("{name}, {pos}, pick {k}: against {n} real {pos}s drafted within 8 picks of him."),
        None => format!("{name}, {pos}, undrafted or before the draft: against {n} real {pos}s."),
    });
    let mut rows = Vec::new();
    for c in b.numeric_cols(&rn) {
        if c == kc {
            continue;
        }
        let s = b.stats(&rn, &c, &wh, &vals);
        let mine = row.get(&c).cloned().unwrap_or(Value::Null);
        let f = |v: &Value| fmt_col(&c, v);
        rows.push(json!([label(&c), f(&mine), f(&s["p10"]), f(&s["p50"]), f(&s["p90"])]));
    }
    push(
        &mut p,
        json!({ "kind": "table", "title": "Career against the real spread",
            "cols": [{"name": "m", "label": "Measure"}, {"name": "him", "label": name, "num": true}, {"name": "p10", "label": "Real p10", "num": true},
                     {"name": "p50", "label": "Real median", "num": true}, {"name": "p90", "label": "Real p90", "num": true}],
            "rows": rows }),
    );
    p
}
