//! Lane S2, the meeting at the map: the week's board data the screen draws
//! (the campus on the map, the areas of your state, the coordinator, the
//! fronts the plan sends a man to, his one call), on top of lane G3's plan.

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
fn the_week_carries_what_the_board_draws() {
    let dir = std::env::temp_dir().join(format!("afm-ground-week-screen-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    common::build_world(&mut g, json!({ "seed": 22, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    assert!(g.call("ground_week", &json!({})).is_err(), "no seat, no week");
    let jobs = ok(&mut g, "jobs", json!({ "level": "college" }));
    let job = jobs["jobs"][0]["ref"].clone();
    ok(&mut g, "start_career", json!({ "name": "Test Coach", "job": job, "seed": 3 }));

    let x = ok(&mut g, "ground_week", json!({}));
    // The map's frame: your state, your campus and the state's areas.
    let home = &x["home"];
    assert_eq!(home["state"].as_str().unwrap().len(), 2);
    assert_eq!(home["campus"].as_array().unwrap().len(), 2);
    let areas = home["areas"].as_array().unwrap();
    assert!(!areas.is_empty(), "your state has areas to send a man to");
    for a in areas {
        assert!(a["code"].as_str().unwrap().starts_with(home["state"].as_str().unwrap()));
    }
    assert!(x["today"].is_string() && x["monday"].is_string());
    assert!(x["coordinator"]["name"].is_string(), "a head coach has a recruiting coordinator");

    // A front for every area the plan sends someone to, in the plan's order.
    let roads = x["roads"].as_array().unwrap();
    let fronts = x["fronts"].as_array().unwrap();
    let mut seen: Vec<&Value> = Vec::new();
    for r in roads {
        if !seen.contains(&&r["area"]["code"]) {
            seen.push(&r["area"]["code"]);
        }
    }
    assert_eq!(fronts.len(), seen.len());
    for (k, f) in fronts.iter().enumerate() {
        assert_eq!(f["n"].as_u64().unwrap(), k as u64 + 1);
        assert_eq!(&f["area"]["code"], seen[k]);
        assert!(f["of"].as_u64().unwrap() >= f["known"].as_u64().unwrap());
        assert!(!f["men"].as_array().unwrap().is_empty());
        assert!(["your call", "new door", "the plan"].contains(&f["kind"].as_str().unwrap()));
    }
    // Where the road runs, it names the day it closes.
    if !x["road"].is_null() {
        assert!(x["road"]["closes"].as_str().unwrap() >= x["monday"].as_str().unwrap());
    }
    // His one call is a call on your list this week, with the standing you have there.
    let to_make = x["calls"].as_array().unwrap().iter().filter(|c| c["state"] == "to make").count();
    if to_make == 0 {
        assert!(x["one_call"].is_null());
    } else {
        assert!(x["one_call"]["who"].is_string() && x["one_call"]["level"].is_string());
        assert_eq!(x["one_call"]["more"].as_u64().unwrap() as usize, to_make - 1);
    }
    // A man sent to a front by code lands in it.
    if let Some(m) = x["men"].as_array().unwrap().iter().find(|m| m["rule"].is_null() && m["out"] == false) {
        let code = areas[0]["code"].clone();
        let day = x["road"]["open_days"].as_array().and_then(|d| d.first().cloned());
        if let Some(day) = day {
            let r = ok(&mut g, "ground_road_set", json!({ "man": m["man"], "area": code, "days": [day] }));
            assert!(r["fronts"].as_array().unwrap().iter().any(|f| f["area"]["code"] == code));
        }
    }
    let _ = std::fs::remove_dir_all(dir);
}
