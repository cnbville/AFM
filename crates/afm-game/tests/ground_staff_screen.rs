//! Lane G10, your staff's ground: the staff rail (each man's areas, mood
//! and style from the coordinator's first proposal), moving a man's areas,
//! his page, the carousel with nobody courted, and a candidate's signs.

use afm_game::Game;
use serde_json::{Value, json};

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    match g.call(cmd, &args) {
        Ok(v) => v,
        Err(e) => panic!("{cmd}: {e}"),
    }
}

#[test]
fn the_staff_rail_reads_each_mans_ground() {
    let dir = std::env::temp_dir().join(format!("afm-ground-staff-screen-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    ok(&mut g, "build_world", json!({ "seed": 23, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    assert!(g.call("ground_staff", &json!({})).is_err(), "no seat, no staff");
    let jobs = ok(&mut g, "jobs", json!({ "level": "college" }));
    let job = jobs["jobs"][0]["ref"].clone();
    ok(&mut g, "start_career", json!({ "name": "Test Coach", "job": job, "seed": 3 }));

    let x = ok(&mut g, "ground_staff", json!({}));
    assert_eq!(x["head"], true);
    let men = x["men"].as_array().unwrap();
    assert!(men.len() >= 3, "a staff");
    // The coordinator's first proposal gives every man his areas.
    for m in men {
        let areas = m["areas"].as_array().unwrap();
        assert!(!areas.is_empty() && areas.len() <= 3, "{m}");
        assert!(m["mood"].is_string() && m["style"].is_string());
        // His head for the screen: initials always, the recipe when drawn.
        assert!(m["face"]["id"].is_u64() && m["face"]["initials"].is_string(), "{m}");
        for a in areas {
            assert!(a["level"].is_string() && a["why"].is_string());
        }
    }
    assert!(!x["proposal"]["letter"].as_array().unwrap().is_empty(), "the coordinator's letter");

    // Move the first man onto another area of his.
    let m = men[0]["man"].as_u64().unwrap();
    let area = men[1]["areas"][0]["code"].clone();
    let y = ok(&mut g, "ground_staff_areas", json!({ "man": m, "areas": [area] }));
    assert!(y["said"].is_string());
    let row = y["men"].as_array().unwrap().iter().find(|r| r["man"] == m).unwrap().clone();
    assert_eq!(row["areas"][0]["code"], area);
    assert_eq!(row["areas"][0]["yours"], true);

    // His page, the carousel and a candidate's signs.
    let page = ok(&mut g, "ground_staff_man", json!({ "man": m }));
    assert_eq!(page["man"], m);
    assert!(page["face"]["id"].is_u64(), "his head");
    assert!(page["mood"]["words"].is_string(), "the mood in words");
    assert!(page["mood"].get("value").is_none(), "never the number");
    for p in page["people"].as_array().unwrap() {
        assert!(p["key"]["kind"].is_string() && p["key"]["id"].is_u64(), "each person opens by his key: {p}");
    }
    let c = ok(&mut g, "ground_carousel", json!({}));
    assert!(c["courted"].is_null(), "nobody is after your men today");
    assert!(g.call("ground_counter", &json!({ "man": m, "kind": "title" })).is_err(), "nothing to counter");
    assert!(g.call("ground_desk", &json!({})).is_err(), "the desk is the assistant's");
    let _ = std::fs::remove_dir_all(&dir);
}
