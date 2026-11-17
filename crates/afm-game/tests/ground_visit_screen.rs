//! Lane S3, the school visit screen: the visit's part of the scene the page
//! draws (the stops so far, where the talk is, the hours, the eye in a word,
//! the kids with the new names marked), the commands it plays on (a stop, a
//! pick in the office), and the cards it reads (the school, the coach, the
//! new lead). Built on lane G5's scene.

mod common;

use afm_game::Game;
use serde_json::{Value, json};

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    match g.call(cmd, &args) {
        Ok(v) => v,
        Err(e) => panic!("{cmd}: {e}"),
    }
}

/// Plays days until a school visit waits for you, handing every other
/// meeting to your staff on the way.
fn find_visit(g: &mut Game) -> Option<u32> {
    for _ in 0..400 {
        let r = ok(g, "continue", json!({}));
        let Some(id) = r["blocked"]["meeting"].as_u64() else { continue };
        let s = ok(g, "people_scene", json!({ "id": id }));
        if s["ground"]["kind"] == "school_visit" {
            return Some(id as u32);
        }
        let _ = g.call("people_hand", &json!({ "id": id }));
    }
    None
}

#[test]
fn a_school_visit_carries_what_the_board_draws() {
    let dir = std::env::temp_dir().join(format!("afm-ground-visit-screen-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    common::build_world(&mut g, json!({ "seed": 22, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    let jobs = ok(&mut g, "jobs", json!({ "level": "college" }));
    let job = jobs["jobs"][0]["ref"].clone();
    ok(&mut g, "start_career", json!({ "name": "Test Coach", "job": job, "seed": 3 }));

    let id = find_visit(&mut g).expect("a school visit comes to your seat in the first spring");
    let s = ok(&mut g, "people_scene", json!({ "id": id }));
    let v = &s["ground"];
    // The page's frame: the school, the coach across, the hours and the eye in a word.
    assert!(v["school"]["name"].is_string() && v["who"]["key"].as_str().unwrap().starts_with("coach:"));
    assert_eq!(v["minutesTotal"], 240);
    assert_eq!(v["minutesLeft"], 240);
    assert_eq!(v["at"], "");
    assert!(v["start"].is_u64() && v["time"].is_string());
    assert!(["sharp", "good", "fair", "weak"].contains(&v["eye"]["words"].as_str().unwrap()));
    assert!(v["eye"]["tired"].is_boolean() && v["manIsYou"].is_boolean());
    // The sign-in sheet is on the scene's tables, and the stops are its picks.
    assert!(s["docs"].as_array().unwrap().iter().any(|d| d["title"] == "The sign-in sheet"));
    let codes: Vec<u64> = s["picks"].as_array().unwrap().iter().map(|p| p["code"].as_u64().unwrap()).collect();
    assert!(codes.contains(&1) && codes.contains(&900), "the office and heading out: {codes:?}");
    // The cards the page reads.
    let school = ok(&mut g, "ground_school", json!({ "id": v["school"]["id"] }));
    assert_eq!(school["school"]["colors"].as_array().unwrap().len(), 2);
    assert!(school["went"].is_array() && school["kids"].is_u64());
    let key = v["who"]["key"].as_str().unwrap().split(':').collect::<Vec<_>>();
    let coach = ok(&mut g, "ground_person", json!({ "kind": key[0], "id": key[1].parse::<u64>().unwrap() }));
    assert!(coach["lv"]["n"].is_u64() && coach["staffs"].is_array());

    // The office: the talk is in his office, 60 minutes gone, and his list
    // writes new names into your leads.
    let s = ok(&mut g, "people_answer", json!({ "id": id, "code": 1 }));
    let _ = s;
    let s = ok(&mut g, "people_scene", json!({ "id": id }));
    assert_eq!(s["ground"]["at"], "office");
    assert_eq!(s["ground"]["minutesLeft"], 180);
    assert!(s["picks"].as_array().unwrap().iter().any(|p| p["code"] == 101), "ask him about his kids");
    ok(&mut g, "people_answer", json!({ "id": id, "code": 101 }));
    let s = ok(&mut g, "people_scene", json!({ "id": id }));
    let stops = s["ground"]["stops"].as_array().unwrap();
    assert!(stops.iter().any(|x| x["key"] == "office") && stops.iter().any(|x| x["key"] == "ask"));
    // Every new name has a lead card with the five blanks and its news.
    for k in s["ground"]["kids"].as_array().unwrap().iter().filter(|k| k["new"] == true) {
        let c = ok(&mut g, "ground_lead", json!({ "id": k["id"] }));
        assert_eq!(c["blanks"].as_array().unwrap().len(), 5);
        assert!(c["kid"]["name"].is_string());
    }
    // Leaving ends it with its line.
    ok(&mut g, "people_answer", json!({ "id": id, "code": 109 }));
    let s = ok(&mut g, "people_answer", json!({ "id": id, "code": 900 }));
    assert_eq!(s["done"], true);
    let s = ok(&mut g, "people_scene", json!({ "id": id }));
    assert!(!s["outcome"].as_array().unwrap().is_empty());
    let _ = std::fs::remove_dir_all(dir);
}
