//! Lane S5, the person screen: the page's data on top of lane G7's card (his
//! face, your staff's read in words, his moments dated, what each level gets
//! you, where he stands with every staff) and the People page's list.

use afm_game::Game;
use serde_json::{Value, json};

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    match g.call(cmd, &args) {
        Ok(v) => v,
        Err(e) => panic!("{cmd}: {e}"),
    }
}

#[test]
fn a_person_page_carries_what_the_board_draws() {
    let dir = std::env::temp_dir().join(format!("afm-ground-person-screen-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    ok(&mut g, "build_world", json!({ "seed": 22, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    assert!(g.call("ground_person", &json!({ "kind": "coach", "id": 0 })).is_err(), "no seat, no page");
    let jobs = ok(&mut g, "jobs", json!({ "level": "college" }));
    let job = jobs["jobs"][0]["ref"].clone();
    ok(&mut g, "start_career", json!({ "name": "Test Coach", "job": job, "seed": 3 }));

    // A school with a head coach, and the people around it.
    let mut coach = Value::Null;
    let mut counselor = Value::Null;
    for school in 0..60 {
        let x = ok(&mut g, "ground_school_people", json!({ "id": school }));
        for p in x["people"].as_array().unwrap() {
            match p["key"]["kind"].as_str().unwrap() {
                "coach" if coach.is_null() => coach = p["key"].clone(),
                "counselor" if counselor.is_null() => counselor = p["key"].clone(),
                _ => {}
            }
        }
        if !coach.is_null() && !counselor.is_null() {
            break;
        }
    }
    assert!(!coach.is_null() && !counselor.is_null());

    let x = ok(&mut g, "ground_person", coach.clone());
    assert!(x["name"].is_string() && x["role"].is_string());
    assert!(x["age"].as_i64().unwrap() >= 20, "a coach has an age");
    assert!(x["school"].is_u64(), "a coach has a school to open");
    assert!(x["today"].is_string() && x["next"].is_string() && x["call"].as_str().unwrap().starts_with("coach:"));
    // Standing: a level with ticks, and what it gets you and what the next would.
    let n = x["lv"]["n"].as_u64().unwrap();
    assert!(n <= 3 && x["lv"]["name"] == x["level"]);
    assert!(x["gets"]["now"]["words"].as_str().unwrap().len() > 10);
    assert_eq!(x["gets"]["next"].is_null(), n == 3);
    // A drawn head from his recipe, in an id range of its own.
    let f = &x["face"];
    assert!(f["id"].as_u64().unwrap() >= 0x8000_0000);
    assert_eq!(f["colors"].as_array().unwrap().len(), 2);
    if f["drawn_on"] == true {
        assert!(f["spec"]["genes"].is_object() && f["spec"]["colors"].is_array());
        assert!(f["heritage_words"].is_string());
    }
    // Your staff's read is words: never a number.
    for k in ["eye", "straight", "nature"] {
        assert!(x["staff_read"][k].is_null() || x["staff_read"][k].is_string(), "{k} is words");
    }
    // Moments are dated and at most five, the newest first.
    assert!(x["moments"].as_array().unwrap().len() <= 5);
    // Ties carry a level and a key to open them.
    for t in x["ties"].as_array().unwrap() {
        assert!(t["key"]["kind"].is_string() && t["lv"]["n"].is_u64());
    }
    // Every other staff he is Known with, then yours last.
    let staffs = x["staffs"].as_array().unwrap();
    assert!(staffs.last().is_some_and(|s| s["you"] == true));
    assert!(staffs.iter().filter(|s| s["you"] == true).count() == 1);
    assert!(x["talked"]["times"].is_u64());

    // A counselor has no drawn head (initials), and no other staffs' standing.
    let c = ok(&mut g, "ground_person", counselor);
    assert!(c["face"]["spec"].is_null() && c["face"]["initials"].is_string());
    assert!(c["staffs"].as_array().unwrap().is_empty());

    // The People page: nobody yet, until your staff deals with someone.
    let l = ok(&mut g, "ground_people", json!({}));
    assert_eq!(l["people"].as_array().unwrap().len().min(60), l["people"].as_array().unwrap().len());
    assert!(l["total"].as_u64().unwrap() as usize >= l["people"].as_array().unwrap().len());
    for p in l["people"].as_array().unwrap() {
        assert!(p["key"]["kind"].is_string() && p["lv"]["n"].is_u64() && p["last"].is_string());
    }
    let _ = std::fs::remove_dir_all(dir);
}
