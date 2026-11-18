//! The team's testing records (Cain 2026-11-15): a coach who takes over a
//! team finds the numbers the staff before him kept, runs his own baseline
//! in his first week, and can call a testing day that costs that day's
//! training.

mod common;

use afm_game::Game;
use serde_json::{Value, json};

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    match g.call(cmd, &args) {
        Ok(v) => v,
        Err(e) => panic!("{cmd}: {e}"),
    }
}

#[test]
fn a_new_coach_inherits_the_teams_testing_and_can_call_his_own() {
    let t = std::time::Instant::now();
    let dir = std::env::temp_dir().join(format!("afm-testing-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    common::build_world(&mut g, json!({ "seed": 13, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    println!("world built in {:.0}s", t.elapsed().as_secs_f64());
    let jobs = ok(&mut g, "jobs", json!({ "level": "college" }));
    let job = jobs["jobs"][0]["ref"].clone();
    ok(
        &mut g,
        "start_career",
        json!({ "name": "Test Coach", "age": 45, "background": "college_assistant", "job": job, "seed": 3, "home_state": "TX" }),
    );
    // Day one: the staff before yours kept the team's testing days.
    let n = ok(&mut g, "practice", json!({}))["numbers"].clone();
    assert!(n["date"].is_string(), "the team's records: {n}");
    assert_eq!(n["previous"], true, "{n}");
    assert!(!n["rows"].as_array().unwrap().is_empty(), "{n}");
    let inherited = n["date"].as_str().unwrap().to_string();
    let id = n["rows"][0]["id"].as_u64().unwrap();
    // Your staff's first week: its own baseline, with the old days behind it.
    let mut n = n;
    for _ in 0..40 {
        if n["baseline"] == true {
            break;
        }
        ok(&mut g, "continue", json!({ "staff": true }));
        n = ok(&mut g, "practice", json!({}))["numbers"].clone();
    }
    assert_eq!(n["baseline"], true, "{n}");
    assert_eq!(n["previous"], false, "{n}");
    assert!(n["older"].as_array().unwrap().iter().any(|d| d.as_str() == Some(inherited.as_str())), "{n}");
    // His page lists both, newest first.
    let body = ok(&mut g, "sb_body", json!({ "id": id }));
    let tests = body["tests"].as_array().unwrap();
    assert!(tests.len() >= 2, "{body}");
    assert_eq!(tests[0]["who"], "Your baseline", "{tests:?}");
    assert!(tests.iter().any(|x| x["who"] == "Previous staff"), "{tests:?}");
    // Call a testing day: it shows as called, and once the week has run the
    // day's numbers are your staff's.
    let calls = n["calls"].as_array().cloned().unwrap_or_default();
    let pick = calls.iter().find(|c| c["called"] == false).expect("a day to call").clone();
    let n = ok(&mut g, "practice_testing", json!({ "day": pick["day"] }))["numbers"].clone();
    assert!(n["calls"].as_array().unwrap().iter().any(|c| c["day"] == pick["day"] && c["called"] == true), "{n}");
    let day = pick["text"].as_str().unwrap().to_string();
    let mut n = n;
    for _ in 0..60 {
        if n["date"].as_str() == Some(day.as_str()) {
            break;
        }
        ok(&mut g, "continue", json!({ "staff": true }));
        n = ok(&mut g, "practice", json!({}))["numbers"].clone();
    }
    assert_eq!(n["date"].as_str(), Some(day.as_str()), "{n}");
    assert_eq!(n["baseline"], false, "{n}");
    assert_eq!(n["previous"], false, "{n}");
    let _ = std::fs::remove_dir_all(&dir);
}
