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

/// A book with one NFL game from the game log, two clubs, a season, a
/// draft pick and an agent (lane R8).
fn r8_book(dir: &std::path::Path) -> std::path::PathBuf {
    let path = dir.join("r8.afmbook");
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute_batch(
        r#"CREATE TABLE nfl(_row INTEGER PRIMARY KEY, id, name, head_coach);
           INSERT INTO nfl VALUES (0, 0, 'Kansas City Chiefs', 0), (1, 1, 'Buffalo Bills', 1);
           CREATE TABLE coaches(_row INTEGER PRIMARY KEY, id, name);
           INSERT INTO coaches VALUES (0, 0, 'Andy Reed'), (1, 1, 'Sean Mack');
           CREATE TABLE log_games(_row INTEGER PRIMARY KEY, season, stream, home, away, home_pts, away_pts, date, kind, week, line, neutral);
           INSERT INTO log_games VALUES (0, 2025, 'nfl', 0, 1, 24, 17, '2025-09-07', 'regular', 1, '[-3.5,44.5]', NULL),
                                        (1, 2025, 'nfl', 1, 0, 20, 23, '2025-11-02', 'regular', 9, '[-1.5,47.5]', NULL);
           CREATE TABLE log_games__box(_row INTEGER PRIMARY KEY, _parent, _i, points, yards, pass_yards, rush_yards, plays, penalties, clock_possession);
           INSERT INTO log_games__box VALUES (0, 0, 0, 24, 380, 250, 130, 64, 5, 1850), (1, 0, 1, 17, 300, 210, 90, 60, 14, 1750),
                                             (2, 1, 0, 20, 330, 230, 100, 62, 6, 1800), (3, 1, 1, 23, 350, 240, 110, 63, 7, 1800);
           CREATE TABLE log_games__quarters(_row INTEGER PRIMARY KEY, _parent, _i, _0, _1);
           INSERT INTO log_games__quarters VALUES (0, 0, 0, 7, 3), (1, 0, 1, 7, 7), (2, 0, 2, 3, 0), (3, 0, 3, 7, 7);
           CREATE TABLE log_games__players(_row INTEGER PRIMARY KEY, _parent, _i, _j, id, pos, att, cmp, pass_yds, rush, rush_yds, tackles, snaps_def);
           INSERT INTO log_games__players VALUES (0, 0, 0, 0, 15, 'QB', 30, 20, 250, 2, 8, NULL, NULL), (1, 0, 1, 0, 17, 'LB', NULL, NULL, NULL, NULL, NULL, 9, 60);
           CREATE TABLE log_games__drives(_row INTEGER PRIMARY KEY, _parent, _i, q, clock, start, "end", plays, secs, result, points, team);
           INSERT INTO log_games__drives VALUES (0, 0, 0, 1, 900, 25, 100, 8, 240, 'Touchdown', 7, NULL), (1, 0, 1, 1, 660, 30, 41, 4, 100, 'Punt', NULL, 1);
           CREATE TABLE log_games__scoring(_row INTEGER PRIMARY KEY, _parent, _i, q, clock, kind, what, yards, carrier, passer, target, kicker, team, after, try__kind, try__good);
           INSERT INTO log_games__scoring VALUES (0, 0, 0, 1, 660, 'Pass', 'touchdown', 12, 15, 15, 16, NULL, NULL, '[7,0]', 'kick', 1);
           CREATE TABLE people(_row INTEGER PRIMARY KEY, player__id, name, player__position, player__age, pro__team, pro__status, pro__contract__kind, pro__earnings, pro__agent, pro__draft, college, x_overall);
           INSERT INTO people VALUES (0, 15, 'Pat Holmes', 'QB', 30, 0, 'Roster', 'Extension', 250000000, 0, '[2017,1,10,0]', NULL, 90),
                                     (1, 16, 'Travis Kell', 'TE', 35, 0, 'PracticeSquad', 'Veteran', 90000000, 0, NULL, NULL, 80);
           CREATE TABLE nfl_seasons(_row INTEGER PRIMARY KEY, year, team, wins, losses, ties, coach, round);
           INSERT INTO nfl_seasons VALUES (0, 2025, 0, 13, 4, 0, 0, 5), (1, 2025, 1, 11, 6, 0, 1, 2);
           CREATE TABLE nfl_champions(_row INTEGER PRIMARY KEY, _0, _1);
           INSERT INTO nfl_champions VALUES (0, 2025, 0);
           CREATE TABLE market__agents(_row INTEGER PRIMARY KEY, name, agency, size, skill, honesty, clients, rel, fired);
           INSERT INTO market__agents VALUES (0, 'Drew Rosen', 'Big Sports', 3, 15, 0.8, 120, '[0.1,-0.3]', 0);"#,
    )
    .unwrap();
    path
}

#[test]
fn game_club_draft_and_agent_pages() {
    let dir = std::env::temp_dir().join(format!("afm-reader-test-r8-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = r8_book(&dir);
    let mut r = Reader::new(dir.join("home"), dir.join("game"));
    r.call("open", &json!({ "path": path.to_string_lossy() })).unwrap();
    let kinds = |p: &Value| {
        p["blocks"].as_array().unwrap().iter().map(|b| b["kind"].as_str().unwrap().to_string()).collect::<Vec<_>>()
    };

    let g = page(&mut r, "game/0", Value::Null);
    assert_eq!(g["title"], "Buffalo Bills 17, Kansas City Chiefs 24");
    assert!(g["blocks"][0]["text"].as_str().unwrap().contains("Chiefs covered"), "{}", g["blocks"][0]["text"]);
    for k in ["drives", "tabtables"] {
        assert!(kinds(&g).contains(&k.to_string()), "{k}: {:?}", kinds(&g));
    }
    let shown = serde_json::to_string(&g["blocks"]).unwrap();
    assert!(shown.contains("Travis Kell 12-yard pass from Pat Holmes (kick good)"), "{shown}");
    let team_box = g["blocks"].as_array().unwrap().iter().find(|b| b["title"] == "Team box").unwrap();
    let pens = team_box["rows"].as_array().unwrap().iter().find(|r| r[0] == "Penalties").unwrap();
    assert_eq!(pens, &json!(["Penalties", "14", "5", "8", "6"]));
    assert_eq!(page(&mut r, "e/log_games/0", Value::Null)["title"], g["title"]);

    let c = page(&mut r, "club/0", Value::Null);
    assert_eq!(c["title"], "Kansas City Chiefs");
    assert!(kinds(&c).contains(&"strip".to_string()), "{:?}", kinds(&c));
    let roster = c["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["title"].as_str().unwrap_or("").starts_with("Roster"))
        .unwrap();
    assert_eq!(roster["rows"][1][3], "practice squad");
    assert_eq!(page(&mut r, "e/nfl/0", Value::Null)["title"], "Kansas City Chiefs");

    let s = page(&mut r, "club/0/2025", Value::Null);
    assert!(s["verdict"].as_str().unwrap().contains("13-4 in 2025 and won the Super Bowl"), "{}", s["verdict"]);
    let sched = s["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["title"].as_str().unwrap_or("").starts_with("Schedule"))
        .unwrap();
    assert_eq!(sched["links"], json!(["game/0", "game/1"]));

    let d = page(&mut r, "draft/2017", Value::Null);
    assert!(d["verdict"].as_str().unwrap().starts_with("1 pick on record"), "{}", d["verdict"]);
    assert_eq!(page(&mut r, "draft/1990", Value::Null)["blocks"].as_array().unwrap().len(), 1);

    let a = page(&mut r, "e/market__agents/0", Value::Null);
    assert_eq!(a["title"], "Drew Rosen");
    let standing = a["blocks"].as_array().unwrap().iter().find(|b| b["title"] == "Standing with each club").unwrap();
    assert_eq!(standing["rows"][0][0], "Buffalo Bills");
    let _ = std::fs::remove_dir_all(dir);
}
