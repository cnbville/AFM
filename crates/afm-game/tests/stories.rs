//! Lane 8, the stories: every stories page's data comes back at a college
//! seat (the office, your career story, each Media and Traditions tab the
//! seat shows, a player's Story tab), a name you give sticks only to your
//! own player and never to a legend's, and the uniform you pick holds.

use afm_game::Game;
use serde_json::{Value, json};

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    match g.call(cmd, &args) {
        Ok(v) => v,
        Err(e) => panic!("{cmd}: {e}"),
    }
}

#[test]
fn every_stories_page_answers_at_a_college_seat() {
    let dir = std::env::temp_dir().join(format!("afm-stories-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    ok(&mut g, "build_world", json!({ "seed": 12, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    let jobs = ok(&mut g, "jobs", json!({ "level": "college" }));
    let job = jobs["jobs"][0]["ref"].clone();
    ok(&mut g, "start_career", json!({ "name": "Test Coach", "job": job, "seed": 3 }));

    // The office opens with its shelves and the job you hold on its road.
    let office = ok(&mut g, "stories_office", json!({}));
    assert_eq!(office["shelves"].as_array().unwrap().len(), 7, "{office}");
    let road = office["stints"].as_array().unwrap();
    assert_eq!(road.len(), 1, "{office}");
    assert_eq!(road[0]["now"], true, "{office}");

    // Your story has the hire from the first day, at the one team you have had.
    let story = ok(&mut g, "stories_story", json!({}));
    assert!(story["entries"].as_array().unwrap().iter().any(|e| e["kind"] == "Hired"), "{story}");
    assert_eq!(story["teams"].as_array().unwrap().len(), 1, "{story}");
    assert_eq!(story["fourth"]["given"], 0.0, "{story}");

    // Every tab the seat shows answers, and a college seat has no fantasy.
    for (cmd, first) in [("stories_media", "Front page"), ("stories_traditions", "Rivalries")] {
        let page = ok(&mut g, cmd, json!({}));
        let tabs: Vec<String> =
            page["tabs"].as_array().unwrap().iter().map(|t| t.as_str().unwrap().to_string()).collect();
        assert_eq!(tabs[0], first, "{page}");
        assert!(!tabs.iter().any(|t| t == "Fantasy"), "{tabs:?}");
        for t in &tabs {
            let v = ok(&mut g, cmd, json!({ "tab": t }));
            assert_eq!(v["tab"], t.as_str(), "{cmd} {t}");
            assert!(v["body"].is_object(), "{cmd} {t}: {v}");
        }
    }

    // A name you give: never a legend's, never another team's player.
    let roster = ok(&mut g, "roster", json!({}));
    let mine = roster["rows"][0]["id"].as_u64().unwrap();
    assert!(g.call("stories_nickname", &json!({ "id": mine, "name": "Megatron" })).is_err());
    assert!(g.call("stories_nickname", &json!({ "id": mine, "name": "  " })).is_err());
    let given = ok(&mut g, "stories_nickname", json!({ "id": mine, "name": "The Iron Mule" }));
    assert!(given["said"].as_str().is_some_and(|s| !s.is_empty()), "{given}");
    assert!(given["names"].as_array().unwrap().iter().any(|n| n["name"] == "The Iron Mule"), "{given}");
    let ours: Vec<u64> = roster["rows"].as_array().unwrap().iter().filter_map(|r| r["id"].as_u64()).collect();
    let theirs = g
        .world
        .as_ref()
        .unwrap()
        .people
        .iter()
        .filter(|p| p.stage == afm_engine::world::Stage::College)
        .map(|p| p.player.id as u64)
        .find(|id| !ours.contains(id))
        .unwrap();
    assert!(g.call("stories_nickname", &json!({ "id": theirs, "name": "Big Train" })).is_err());
    let page = ok(&mut g, "stories_player", json!({ "id": mine }));
    assert!(page["names"].as_array().unwrap().iter().any(|n| n["name"] == "The Iron Mule"), "{page}");

    // The uniform you pick for the next game holds; -1 hands it back to the staff.
    let sets = ok(&mut g, "stories_traditions", json!({ "tab": "Uniforms" }));
    let n = sets["body"]["sets"].as_array().map_or(0, Vec::len);
    assert!(n >= 2, "{sets}");
    let picked = ok(&mut g, "stories_uniform", json!({ "set": 1 }));
    assert_eq!(picked["body"]["next"], 1, "{picked}");
    assert!(g.call("stories_uniform", &json!({ "set": 99 })).is_err());
    let back = ok(&mut g, "stories_uniform", json!({ "set": -1 }));
    assert!(back["body"]["next"].is_null(), "{back}");

    // A few weeks on, still one hire on the timeline, and the pages still answer.
    ok(&mut g, "holiday", json!({ "days": 30 }));
    let story = ok(&mut g, "stories_story", json!({}));
    assert_eq!(story["entries"].as_array().unwrap().iter().filter(|e| e["kind"] == "Hired").count(), 1, "{story}");
    ok(&mut g, "stories_media", json!({ "tab": "Nicknames" }));
    let _ = std::fs::remove_dir_all(dir);
}
