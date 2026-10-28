//! The commands on the stand-in book: every page opens, filters and the
//! year slider narrow rows, search finds names, hidden truths hide, the
//! copied text carries the page, compare moves.

use serde_json::{Value, json};

use crate::Reader;

fn reader(tag: &str) -> (Reader, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("afm-reader-test-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("books")).unwrap();
    let book = dir.join("books").join("stand-in.afmbook");
    crate::demo::write(&book).unwrap();
    let mut r = Reader::new(dir.join("home"), dir.join("game"));
    r.call("open", &json!({ "path": book.to_string_lossy() })).unwrap();
    (r, dir)
}

fn page(r: &mut Reader, route: &str, args: Value) -> Value {
    let mut a = args;
    a["route"] = json!(route);
    r.call("page", &a).unwrap_or_else(|e| panic!("{route}: {e}"))
}

#[test]
fn every_page_opens() {
    let (mut r, dir) = reader("pages");
    let st = r.call("status", &Value::Null).unwrap();
    assert_eq!(st["open"], true);
    assert_eq!(st["nav"].as_array().unwrap().len(), 13);
    assert_eq!(st["years"], json!([1996, 2025]));
    let ov = page(&mut r, "overview", Value::Null);
    assert!(ov["verdict"].as_str().unwrap().contains("14 checks"), "{}", ov["verdict"]);
    for route in [
        "page/people",
        "page/programs",
        "page/career",
        "tools/issues",
        "tools/ids",
        "tools/coverage",
        "tools/replay",
        "tools/play",
        "tools/odd",
        "compare/years",
        "compare/groups",
        "table/people",
        "e/people/3",
        "e/programs/0",
    ] {
        let p = page(&mut r, route, Value::Null);
        assert!(!p["title"].as_str().unwrap_or("").is_empty(), "{route}");
    }
    let replay = page(&mut r, "tools/replay", json!({ "game": 2 }));
    assert_eq!(replay["blocks"][0]["snaps"].as_array().unwrap().len(), 40);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn filters_slider_and_search() {
    let (mut r, dir) = reader("filters");
    let all = r.call("rows", &json!({ "table": "people" })).unwrap();
    let qb = r.call("rows", &json!({ "table": "people", "filters": { "position": "QB" } })).unwrap();
    assert!(qb["total"].as_i64().unwrap() < all["total"].as_i64().unwrap());
    assert!(qb["rows"].as_array().unwrap().iter().all(|row| row.as_array().unwrap().iter().any(|c| c == "QB")));
    // The slider: the world as it stood in 2005 keeps people active then.
    let then = r.call("rows", &json!({ "table": "people", "filters": { "year": 2005 } })).unwrap();
    assert!(then["total"].as_i64().unwrap() < all["total"].as_i64().unwrap());
    let seasons =
        r.call("rows", &json!({ "table": "person_seasons", "filters": { "from": 2000, "to": 2001 } })).unwrap();
    let years: Vec<i64> = seasons["rows"].as_array().unwrap().iter().map(|row| row[1].as_i64().unwrap()).collect();
    assert!(years.iter().all(|y| (2000..=2001).contains(y)));
    let found = r.call("search", &json!({ "q": "Georgia" })).unwrap();
    assert_eq!(found[0]["table"], "programs");
    let p = r.call("search", &json!({ "q": "overstreet" })).unwrap();
    let p = p.as_array().unwrap();
    assert!(p.iter().any(|x| x["table"] == "people"));
    assert!(p.iter().all(|x| x["name"].as_str().unwrap().to_lowercase().contains("overstreet")));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn hidden_truths_hide() {
    let (mut r, dir) = reader("hidden");
    let on = r.call("rows", &json!({ "table": "people" })).unwrap();
    assert!(on["cols"].as_array().unwrap().iter().any(|c| c["name"] == "potential" && c["debug"] == true));
    r.call("hidden", &json!({ "on": false })).unwrap();
    let off = r.call("rows", &json!({ "table": "people" })).unwrap();
    assert!(off["cols"].as_array().unwrap().iter().all(|c| c["name"] != "potential"));
    let text = r.call("copy", &json!({ "route": "e/people/3" })).unwrap();
    assert!(!text.as_str().unwrap().contains("Potential (debug)"));
    assert!(
        r.call("page", &json!({ "route": "table/coaches" })).unwrap()["blocks"][0]["first"]["cols"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["name"] != "hidden_value")
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn copy_compare_and_player() {
    let (mut r, dir) = reader("copy");
    let text = r.call("copy", &json!({ "route": "overview" })).unwrap();
    let t = text.as_str().unwrap();
    assert!(t.starts_with("AFM World Reader · Overview · Stand-in book"), "{t}");
    assert!(t.contains("NFL points a team game: 30.9"), "{t}");
    let pl = page(&mut r, "e/people/3", Value::Null);
    let kinds: Vec<&str> = pl["blocks"].as_array().unwrap().iter().map(|b| b["kind"].as_str().unwrap()).collect();
    assert!(kinds.contains(&"chart") && kinds.contains(&"rows"), "{kinds:?}");
    let g = page(&mut r, "compare/groups", json!({ "table": "people", "col": "position", "g1": "QB", "g2": "RB" }));
    assert!(!g["blocks"][0]["rows"].as_array().unwrap().is_empty());
    let y = page(&mut r, "compare/years", json!({ "table": "person_seasons", "y1": 2000, "y2": 2010 }));
    assert!(!y["blocks"][0]["rows"].as_array().unwrap().is_empty());
    let like = page(&mut r, "compare/player", json!({ "id": 3 }));
    assert!(like["verdict"].as_str().unwrap().contains("real"), "{}", like["verdict"]);
    // Two worlds: a copy of the book with one number moved.
    let b2 = dir.join("books").join("stand-in-b.afmbook");
    std::fs::copy(dir.join("books").join("stand-in.afmbook"), &b2).unwrap();
    {
        let c = rusqlite::Connection::open(&b2).unwrap();
        c.execute("UPDATE checks SET world = 24.0 WHERE id = 'nfl_points'", []).unwrap();
    }
    r.call("open", &json!({ "path": b2.to_string_lossy() })).unwrap();
    let a = dir.join("books").join("stand-in.afmbook");
    let cw = page(&mut r, "compare/worlds", json!({ "a": a.to_string_lossy() }));
    assert!(cw["verdict"].as_str().unwrap().contains("1 closer"), "{}", cw["verdict"]);
    let _ = std::fs::remove_dir_all(dir);
}
