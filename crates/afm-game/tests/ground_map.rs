//! Lane S1, Ground's map screen and school page: one state's areas with who
//! leads each, where you stand, fog where your ground hasn't opened it, the
//! area you picked in full, what people kept of you, and a school as a place.

use afm_game::Game;
use serde_json::{Value, json};

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    match g.call(cmd, &args) {
        Ok(v) => v,
        Err(e) => panic!("{cmd}: {e}"),
    }
}

#[test]
fn the_map_and_a_school_read_from_the_world() {
    let dir = std::env::temp_dir().join(format!("afm-ground-map-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    ok(&mut g, "build_world", json!({ "seed": 21, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    // Ground is a college seat's game: before one, the screens say so.
    assert!(g.call("ground_map", &json!({})).is_err(), "no seat, no map");
    let jobs = ok(&mut g, "jobs", json!({ "level": "college" }));
    let job = jobs["jobs"][0]["ref"].clone();
    ok(&mut g, "start_career", json!({ "name": "Test Coach", "job": job, "seed": 3 }));

    let m = ok(&mut g, "ground_map", json!({}));
    let areas = m["areas"].as_array().unwrap();
    assert!(!areas.is_empty(), "your state has areas");
    assert_eq!(m["state"]["yours"], true, "the map opens on your own state");
    // Every area names itself, says where you stand and whether it is fogged;
    // a fogged area keeps its words to itself (GR2) and shows nobody by name.
    for a in areas {
        assert!(a["code"].as_str().unwrap().starts_with(m["state"]["code"].as_str().unwrap()));
        assert!(a["you"]["level"]["name"].is_string());
        assert_eq!(a["fog"].as_bool().unwrap(), a["you"]["known"].as_u64().unwrap() == 0);
        if a["fog"] == true {
            assert!(a["words"].as_array().unwrap().is_empty(), "fog hides the area's words");
        }
        assert!(a["label"].as_array().unwrap().len() == 2);
    }
    assert!(areas.iter().any(|a| a["leader"].is_object()), "somebody leads somewhere");
    // The area picked comes in full, and the picked code is one of the state's.
    let picked = m["picked"].as_str().unwrap();
    assert!(areas.iter().any(|a| a["code"] == picked));
    let d = &m["detail"];
    assert_eq!(d["code"], picked);
    assert!(d["trip"].is_string() && d["schools"].as_u64().unwrap() > 0);
    if d["fog"] == true {
        assert!(d["people"].as_array().unwrap().is_empty(), "fogged areas name no people");
        assert!(!d["public"].as_array().unwrap().is_empty(), "the public view still shows schools");
    }
    assert!(m["period"]["name"].is_string() && m["today"].is_string());

    // Asking for another area, and for another state, moves the map.
    let other = areas.iter().map(|a| a["code"].as_str().unwrap()).find(|c| *c != picked);
    if let Some(c) = other {
        assert_eq!(ok(&mut g, "ground_map", json!({ "area": c }))["picked"], c);
    }
    let tx = ok(&mut g, "ground_map", json!({ "state": "TX" }));
    assert_eq!(tx["state"]["code"], "TX");
    assert!(tx["areas"].as_array().unwrap().len() > 5, "Texas splits into many areas");

    // A school is a place: its colors, people, where its kids went, its seasons.
    let school_id = d["public"][0]["school"].as_u64().or_else(|| d["people"][0]["school"].as_u64()).unwrap();
    let s = ok(&mut g, "ground_school", json!({ "id": school_id }));
    assert_eq!(s["school"]["id"], school_id);
    assert_eq!(s["school"]["colors"].as_array().unwrap().len(), 2);
    assert!(!s["folks"].as_array().unwrap().is_empty(), "a school has people");
    assert!(s["area"]["code"].is_string() && s["trip"].is_string());
    for p in s["folks"].as_array().unwrap() {
        // What a man remembers of you shows only once you are Known to him.
        if p["level"]["n"] == 0 {
            assert!(p["moment"].is_null() && p["why"].is_null(), "a stranger remembers nothing");
        }
    }
    assert!(g.call("ground_school", &json!({ "id": 99_999_999u64 })).is_err());
    let _ = std::fs::remove_dir_all(dir);
}
