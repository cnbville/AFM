//! A career on a small world (section 8): build a world, take a college
//! job, set a depth chart, play into the season and read every screen.

use afm_game::Game;
use serde_json::{Value, json};

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    match g.call(cmd, &args) {
        Ok(v) => v,
        Err(e) => panic!("{cmd}: {e}"),
    }
}

#[test]
fn a_college_career_plays_through_its_first_games() {
    let t = std::time::Instant::now();
    let dir = std::env::temp_dir().join(format!("afm-career-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    ok(&mut g, "build_world", json!({ "seed": 7, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    println!("world built in {:.0}s", t.elapsed().as_secs_f64());
    let setup = ok(&mut g, "setup", json!({}));
    assert!(setup["backgrounds"].as_array().unwrap().len() >= 10);
    let jobs = ok(&mut g, "jobs", json!({ "level": "college" }));
    let job = jobs["jobs"][0]["ref"].clone();
    ok(
        &mut g,
        "start_career",
        json!({ "name": "Test Coach", "age": 41, "background": "college_assistant", "job": job, "seed": 99 }),
    );
    let home = ok(&mut g, "home", json!({}));
    assert!(home["team"]["name"].is_string(), "{home}");
    let roster = ok(&mut g, "roster", json!({}));
    let rows = roster["rows"].as_array().unwrap();
    assert!(rows.len() > 40, "{} players", rows.len());
    // Your own players come as ranges, never one number.
    assert!(rows[0]["ovr"].is_array());
    let id = rows[0]["id"].as_u64().unwrap();
    let page = ok(&mut g, "player", json!({ "id": id }));
    assert!(
        page["groups"].as_array().unwrap().iter().all(|g| g["attrs"]
            .as_array()
            .unwrap()
            .iter()
            .all(|a| a["read"].is_array()))
    );
    ok(&mut g, "note_add", json!({ "id": id, "text": "Looks quicker than his film." }));
    assert_eq!(ok(&mut g, "player", json!({ "id": id }))["notes"].as_array().unwrap().len(), 1);
    // Your depth chart: put the last quarterback first.
    let depth = ok(&mut g, "depth", json!({}));
    let qbs: Vec<u64> =
        depth["positions"][0]["players"].as_array().unwrap().iter().map(|p| p["id"].as_u64().unwrap()).collect();
    if qbs.len() > 1 {
        let mut order = qbs.clone();
        order.rotate_right(1);
        let d = ok(&mut g, "set_depth", json!({ "pos": "QB", "ids": order }));
        assert_eq!(d["positions"][0]["players"][0]["id"].as_u64().unwrap(), order[0]);
    }
    // Play into the season: Continue until your team has a kept game.
    let mut steps = 0;
    while ok(&mut g, "games", json!({}))["games"].as_array().unwrap().is_empty() && steps < 400 {
        ok(&mut g, "continue", json!({ "max_days": 30 }));
        steps += 1;
    }
    let game = ok(&mut g, "game", json!({ "index": 0 }));
    assert!(game["plays"].as_array().unwrap().len() > 80, "a whole game's plays");
    for cmd in ["schedule", "standings", "staff", "inbox", "news", "calendar", "you", "status"] {
        ok(&mut g, cmd, json!({}));
    }
    assert!(!ok(&mut g, "search", json!({ "q": "test coach" }))["coaches"].as_array().unwrap().is_empty());
    // Save and load: the loaded career plays on.
    let saved = ok(&mut g, "save", json!({ "name": "test career" }));
    let path = saved["saved"].as_str().unwrap().to_string();
    ok(&mut g, "load", json!({ "path": path }));
    assert_eq!(ok(&mut g, "player", json!({ "id": id }))["notes"].as_array().unwrap().len(), 1);
    ok(&mut g, "continue", json!({ "max_days": 10 }));
    assert_eq!(ok(&mut g, "library", json!({}))["saves"].as_array().unwrap().len(), 1);
    println!("career test in {:.0}s", t.elapsed().as_secs_f64());
    let _ = std::fs::remove_dir_all(dir);
}
