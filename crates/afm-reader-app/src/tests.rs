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

/// A small book in R4's names, written by hand: one held-back boy with his
/// seasons, stat lines, college staff and notes, and two keyed panels.
fn r6_book(dir: &std::path::Path) -> std::path::PathBuf {
    let path = dir.join("r6.afmbook");
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute_batch(
        r#"CREATE TABLE world(year, today);
           INSERT INTO world VALUES (2026, 20493);
           CREATE TABLE programs(_row INTEGER PRIMARY KEY, id, real_name, fictional_name, division, state);
           INSERT INTO programs VALUES (0, 0, 'Iowa State', NULL, 'FBS', 14);
           CREATE TABLE high_schools(_row INTEGER PRIMARY KEY, name, state);
           INSERT INTO high_schools VALUES (0, 'Ames', 14);
           CREATE TABLE nfl(_row INTEGER PRIMARY KEY, id, name);
           INSERT INTO nfl VALUES (0, 0, 'Arizona Cardinals');
           CREATE TABLE people(_row INTEGER PRIMARY KEY, player__id, name, class_of, stage, hs, stars, player__position, notes);
           INSERT INTO people VALUES (0, 77, 'Cole Becker', 2024, 'College', 0, 3, 'RB',
             '["Named first-team all-state at 18.","At 19 he started at Iowa State.","Rushed for 1,000 yards."]');
           CREATE TABLE people__player__history(_row INTEGER PRIMARY KEY, _parent, _i, age, level, position, playing_time, overall_start, overall,
             potential, performance, level_rating, coaching, events, games_missed, adjustment, spring_gain, team, depth);
           INSERT INTO people__player__history VALUES
             (0, 0, 0, 17, 'HS', 'RB', 'Starter', 60, 63, 80, 62, NULL, 10, 0, 0, NULL, NULL, NULL, 1),
             (1, 0, 1, 18, 'HS', 'RB', 'Starter', 63, 66, 80, 65, NULL, 10, 4, 0, NULL, NULL, NULL, 1),
             (2, 0, 2, 19, 'FBS', 'RB', 'Starter', 66, 70, 80, 69, NULL, 12, 0, 2, NULL, NULL, NULL, 1);
           CREATE TABLE people__college_staff(_row INTEGER PRIMARY KEY, _parent, _i, _0, _1, _2, _3);
           INSERT INTO people__college_staff VALUES (0, 0, 0, 2024, 0, NULL, NULL);
           CREATE TABLE people__hs_lines(_row INTEGER PRIMARY KEY, _parent, _i, year, school, pos, games, s);
           INSERT INTO people__hs_lines VALUES (0, 0, 0, 2023, 0, 'RB', 11, '[0,0,0,0,0,0,0,210,1650,19,12,140,1,0,0,0,0,0,0,0,0]');
           CREATE TABLE stats__seasons(_row INTEGER PRIMARY KEY, id, year, division, pos, program, games, s);
           INSERT INTO stats__seasons VALUES (0, 77, 2024, 'FBS', 'RB', 0, 12,
             '[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,180,1012,9,0,0,0,0,0,0,0,21,160,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]');
           CREATE TABLE panels(id TEXT PRIMARY KEY, page TEXT, area TEXT, title TEXT, kind TEXT, note TEXT,
             needs TEXT, open TEXT, cols TEXT, rows TEXT, n INTEGER, more INTEGER, says TEXT, status TEXT, sql TEXT, seconds REAL);
           INSERT INTO panels VALUES ('class.says', 'class', 'T8', 'The class in a line', 'table', '', 'world file', '',
             '["class","says"]', '[[2023,"The class of 2023: small."],[2024,"The class of 2024: one held-back back."]]', 2, 0, NULL, 'ok', '', 0);
           INSERT INTO panels VALUES ('state.stars', 'state', 'C28', 'Seniors by stars', 'bars', '', 'world file', '',
             '["state","label","world"]', '[["IA","3 stars",4],["TX","3 stars",90]]', 2, 0, NULL, 'ok', '', 0);"#,
    )
    .unwrap();
    path
}

#[test]
fn player_page_and_keyed_pages() {
    let dir = std::env::temp_dir().join(format!("afm-reader-test-r6-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = r6_book(&dir);
    let b = crate::book::Book::open(&path).unwrap();
    let row = b.select_maps("SELECT * FROM people WHERE _row = 0", &[]).unwrap().remove(0);
    let blocks = crate::player::blocks(&b, "people", &row, &crate::book::Filters::default());
    let seasons = blocks.iter().find(|x| x["title"] == "Season by season").expect("a season table");
    let rows = seasons["rows"].as_array().unwrap();
    // Held back a year: his senior season (18) was the fall before his class
    // year, so his first college season (19) is 2024, not class - 18 + age.
    assert_eq!(rows[1][0], 2023, "{rows:?}");
    assert_eq!(rows[2][0], 2024, "{rows:?}");
    assert_eq!(rows[2][3], "Iowa State");
    assert_eq!(rows[1][3], "Ames");
    let line = |r: &Value| {
        r.as_array().unwrap().iter().filter_map(Value::as_str).find(|t| t.contains(" g: ")).unwrap_or("").to_string()
    };
    assert_eq!(line(&rows[2]), "12 g: 180 rushes, 1012 yds, 9 TD; 21 catches, 160 yds");
    assert_eq!(line(&rows[1]), "11 g: 210 rushes, 1650 yds, 19 TD; 12 catches, 140 yds");
    let what = rows[2]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .any(|t| t.contains("stepped up to FBS") && t.contains("missed 2 games"));
    assert!(what, "{:?}", rows[2]);
    let story = blocks.iter().find(|x| x["kind"] == "timeline").expect("his story");
    let when: Vec<&str> = story["items"].as_array().unwrap().iter().map(|i| i["when"].as_str().unwrap()).collect();
    assert_eq!(when, ["2023", "2024", "2024"]);
    drop(b);

    let mut r = Reader::new(dir.join("home"), dir.join("game"));
    r.call("open", &json!({ "path": path.to_string_lossy() })).unwrap();
    let class = page(&mut r, "class/2024", Value::Null);
    assert!(class["verdict"].as_str().unwrap().contains("one held-back back"), "{}", class["verdict"]);
    let state = page(&mut r, "state/IA", Value::Null);
    let shown = serde_json::to_string(&state["blocks"]).unwrap();
    assert!(shown.contains("IA") && !shown.contains("\"TX\""), "{shown}");
    let _ = std::fs::remove_dir_all(dir);
}

/// A small book for R7's pages: a program with two head coaches and a
/// conference move, the coach who taught the other, a high school, and a
/// keyed conference panel.
fn r7_book(dir: &std::path::Path) -> std::path::PathBuf {
    let path = dir.join("r7.afmbook");
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute_batch(
        r#"CREATE TABLE world(year, today);
           INSERT INTO world VALUES (2025, 20300);
           CREATE TABLE state_codes(idx, code);
           INSERT INTO state_codes VALUES (14, 'IA');
           CREATE TABLE programs(_row INTEGER PRIMARY KEY, id, real_name, fictional_name, division, conference, prestige, state, head_coach);
           INSERT INTO programs VALUES (0, 0, 'Iowa State', NULL, 'FBS', 'Big 12', 0.5, 14, 2), (1, 1, 'Iowa', NULL, 'FBS', 'Big Ten', 0.6, 14, NULL);
           CREATE TABLE programs__history(_row INTEGER PRIMARY KEY, _parent, _i, _0, _1, _2, _3);
           INSERT INTO programs__history VALUES (0, 0, 0, 1990, 1995, 'FBS', 'Big Eight'), (1, 0, 1, 1996, 2025, 'FBS', 'Big 12');
           CREATE TABLE programs__seasons(_row INTEGER PRIMARY KEY, _parent, _i, year, wins, losses, conference_title, national_title, coach, strength);
           INSERT INTO programs__seasons VALUES
             (0, 0, 0, 1995, 3, 8, 0, 0, 1, 0.4), (1, 0, 1, 1996, 2, 9, 0, 0, 1, 0.4),
             (2, 0, 2, 1997, 8, 4, 0, 0, 2, 0.5), (3, 0, 3, 1998, 11, 2, 1, 0, 2, 0.6);
           CREATE TABLE coaches(_row INTEGER PRIMARY KEY, id, name, age, mentor, job__role, job__employer__College, job__since, wins, losses, career_titles, retired);
           INSERT INTO coaches VALUES (0, 0, 'Old Teacher', 70, NULL, NULL, NULL, NULL, 0, 0, 0, 1),
             (1, 1, 'Dan Fired', 58, 0, NULL, NULL, NULL, 0, 0, 0, 0),
             (2, 2, 'Matt Builder', 49, 1, 'HeadCoach', 0, 1996, 19, 6, 0, 0);
           CREATE TABLE coaches__history(_row INTEGER PRIMARY KEY, _parent, _i, role, "from", "to", wins, losses, titles, ended, employer__College, employer__HighSchool);
           INSERT INTO coaches__history VALUES (0, 1, 0, 'HeadCoach', 1994, 1996, 5, 17, 0, 'fired', 0, NULL),
             (1, 2, 0, 'OffensiveCoordinator', 1990, 1996, 0, 0, 0, 'left for another job', 1, NULL);
           CREATE TABLE high_schools(_row INTEGER PRIMARY KEY, id, name, state, coach);
           INSERT INTO high_schools VALUES (0, 0, 'Ames', 14, NULL);
           CREATE TABLE high_schools__seasons(_row INTEGER PRIMARY KEY, _parent, _i, year, wins, losses, state_title);
           INSERT INTO high_schools__seasons VALUES (0, 0, 0, 2024, 12, 1, 1);
           CREATE TABLE panels(id TEXT PRIMARY KEY, page TEXT, area TEXT, title TEXT, kind TEXT, note TEXT,
             needs TEXT, open TEXT, cols TEXT, rows TEXT, n INTEGER, more INTEGER, says TEXT, status TEXT, sql TEXT, seconds REAL);
           INSERT INTO panels VALUES ('conf.says', 'conference', 'C6', 'The conference in a line', 'table', '', 'world file', '',
             '["conference","says"]', '[["Big 12","The Big 12: Iowa State won it in 1998."],["Big Ten","The Big Ten: Iowa."]]', 2, 0, NULL, 'ok', '', 0);
           INSERT INTO panels VALUES ('conf.members', 'conference', 'C6', 'Members now', 'table', '', 'world file', '',
             '["conference","program"]', '[["Big 12","Iowa State"],["Big Ten","Iowa"]]', 2, 0, NULL, 'ok', '', 0);"#,
    )
    .unwrap();
    path
}

#[test]
fn program_coach_and_conference_pages() {
    let dir = std::env::temp_dir().join(format!("afm-reader-test-r7-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = r7_book(&dir);
    let mut r = Reader::new(dir.join("home"), dir.join("game"));
    r.call("open", &json!({ "path": path.to_string_lossy() })).unwrap();
    let block = |p: &Value, title: &str| -> Value {
        p["blocks"].as_array().unwrap().iter().find(|b| b["title"] == title).cloned().unwrap_or_else(|| {
            let titles: Vec<&Value> = p["blocks"].as_array().unwrap().iter().map(|b| &b["title"]).collect();
            panic!("no {title} in {titles:?}")
        })
    };

    // The program: its conferences and head coaches as lanes, each head
    // coach's run from the season rows (1995-1996 and 1997-1998).
    let prog = page(&mut r, "e/programs/0", Value::Null);
    let lanes = serde_json::to_string(&block(&prog, "Through the years")).unwrap();
    assert!(lanes.contains("Big Eight") && lanes.contains("Big 12") && lanes.contains("Matt Builder"), "{lanes}");
    let heads = block(&prog, "Head coaches");
    let shown = serde_json::to_string(&heads["rows"]).unwrap();
    assert!(shown.contains("Dan Fired") && shown.contains("5-17") && shown.contains("fired"), "{shown}");

    // One season, with links to the seasons either side.
    let season = page(&mut r, "season/0/1998", Value::Null);
    assert!(season["title"].as_str().unwrap().contains("1998"), "{}", season["title"]);
    assert!(serde_json::to_string(&season).unwrap().contains("11-2"));

    // The coach: his jobs as lanes, every job's why, and his tree up and down.
    let coach = page(&mut r, "e/coaches/1", Value::Null);
    let jobs = serde_json::to_string(&block(&coach, "His jobs over the years")).unwrap();
    assert!(jobs.contains("Iowa State"), "{jobs}");
    let tree = block(&coach, "His coaching tree");
    let t = serde_json::to_string(&tree).unwrap();
    assert!(t.contains("Old Teacher") && t.contains("Matt Builder") && t.contains("e/coaches/2"), "{t}");

    // The conference page: its own line, its rows only.
    let conf = page(&mut r, "conference/Big 12", Value::Null);
    assert!(conf["verdict"].as_str().unwrap().contains("won it in 1998"), "{}", conf["verdict"]);
    let shown = serde_json::to_string(&conf["blocks"]).unwrap();
    assert!(shown.contains("Iowa State") && !shown.contains("\"Iowa\""), "{shown}");

    // The high school.
    let hs = page(&mut r, "e/high_schools/0", Value::Null);
    assert!(hs["title"].as_str().unwrap().contains("Ames"), "{}", hs["title"]);
    let _ = std::fs::remove_dir_all(dir);
}
