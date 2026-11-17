//! Lane CS1, Capital's Campus screen: the map's buildings and rooms, every
//! room against the conference, the program beside yours, and a renovation
//! put to the athletic director.

use afm_game::Game;
use serde_json::{Value, json};

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    match g.call(cmd, &args) {
        Ok(v) => v,
        Err(e) => panic!("{cmd}: {e}"),
    }
}

#[test]
fn the_campus_screen_reads_your_buildings_and_asks_for_a_renovation() {
    let dir = std::env::temp_dir().join(format!("afm-capital-campus-screen-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    ok(&mut g, "build_world", json!({ "seed": 23, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    assert!(g.call("capital_campus", &json!({})).is_err(), "no seat, no campus");
    let jobs = ok(&mut g, "jobs", json!({ "level": "college" }));
    let job = jobs["jobs"][0]["ref"].clone();
    ok(&mut g, "start_career", json!({ "name": "Test Coach", "job": job, "seed": 3 }));

    let x = ok(&mut g, "capital_campus", json!({}));
    assert_eq!(x["head"], true);
    assert!(x["program"].is_string() && x["conference"].is_string());
    let halls = x["halls"].as_array().unwrap();
    assert!(!halls.is_empty(), "a campus has buildings");
    let mut seen = 0;
    for h in halls {
        assert!(h["name"].is_string());
        for r in h["rooms"].as_array().unwrap() {
            let (l, t) = (r["level"].as_u64().unwrap(), r["target"].as_u64().unwrap());
            assert!((1..=5).contains(&l) && t >= l && t <= 5, "{r}");
            seen += 1;
        }
    }
    assert!(seen >= 5, "the everyday rooms are on the map: {seen}");

    // Every room against the conference, with its tool and the AD's read.
    let rooms = x["rooms"].as_array().unwrap();
    assert_eq!(rooms.len(), 13);
    for r in rooms {
        assert!(r["name"].is_string() && r["unlocks"].is_string() && r["read"].is_string(), "{r}");
        if r["can"] == true {
            assert!(r["odds"].is_string(), "the AD's read in words: {r}");
        }
    }
    assert!(x["next"]["text"].is_string());
    // No dollar figures on screen (CP39).
    let text = x.to_string();
    assert!(!text.contains('$'), "no dollars on the campus page");
    if let Some(r) = x.get("rival").filter(|r| r.is_object()) {
        assert!(r["name"].is_string() && r["line"].is_string());
        assert!(!r["rooms"].as_array().unwrap().is_empty());
    }

    // Put a renovation to the AD: his answer comes back with the page.
    if let Some(room) = rooms.iter().find(|r| r["can"] == true) {
        let k = room["room"].clone();
        let y = ok(&mut g, "capital_renovate", json!({ "room": k }));
        assert!(y["said"].is_string(), "his answer");
        let again = y["rooms"].as_array().unwrap().iter().find(|r| r["room"] == k).unwrap().clone();
        // Yes: it is done this summer and can't be asked twice. No: asked once, answered.
        if again["renovating"] == true {
            assert_eq!(again["can"], false);
            assert!(g.call("capital_renovate", &json!({ "room": k })).is_err(), "one project at a time");
        }
    }
    assert!(g.call("capital_renovate", &json!({ "room": 99 })).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}
