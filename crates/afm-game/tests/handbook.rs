//! Lane 8, the Handbook (H8-H15): every page answers at each level with
//! your level first, the guided week's card and your No. 2's first memo,
//! a task ticking, and Help set to Off ending the week and the warnings.

use afm_game::Game;
use serde_json::{Value, json};

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    match g.call(cmd, &args) {
        Ok(v) => v,
        Err(e) => panic!("{cmd}: {e}"),
    }
}

#[test]
fn the_handbook_and_the_first_week_at_each_level() {
    let dir = std::env::temp_dir().join(format!("afm-handbook-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    ok(&mut g, "build_world", json!({ "seed": 13, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    let fresh = ok(&mut g, "save", json!({ "name": "fresh" }))["saved"].as_str().unwrap().to_string();

    for (level, bg, extra) in
        [("hs", "hs_coach", "twoway"), ("college", "college_assistant", "board"), ("nfl", "nfl_assistant", "cut")]
    {
        ok(&mut g, "load", json!({ "path": fresh }));
        let jobs = ok(&mut g, "jobs", json!({ "level": level }));
        let job = jobs["jobs"][0]["ref"].clone();
        ok(
            &mut g,
            "start_career",
            json!({ "name": "Test Coach", "age": 44, "background": bg, "job": job, "seed": 3, "help": "full" }),
        );

        // Every page answers, your level first; examples are staff reads.
        for p in afm_game::lane_handbook::PAGES {
            let v = ok(&mut g, "l8_page", json!({ "page": p }));
            assert_eq!(v["order"][0], level, "{p}: {v}");
            assert!(v["live"].is_object() || v["live"].is_null(), "{p}: {v}");
        }
        assert_eq!(ok(&mut g, "l8_help", json!({}))["read"].as_array().unwrap().len(), 14);

        // The card: six tasks, the three every level starts with, then the level's.
        let card = ok(&mut g, "l8_started", json!({}));
        assert_eq!(card["show"], true, "{level}: {card}");
        let keys: Vec<&str> = card["tasks"].as_array().unwrap().iter().map(|t| t["key"].as_str().unwrap()).collect();
        assert_eq!(keys.len(), 6, "{keys:?}");
        assert_eq!(&keys[..3], ["depth", "plan", "training"], "{level}");
        assert!(keys.contains(&extra), "{level}: {keys:?}");

        // Day one's memo from your No. 2 is in, with its two links.
        let mail = ok(&mut g, "mail", json!({}));
        let memo =
            mail["rows"].as_array().unwrap().iter().find(|r| r["head"] == "memo" && r["subject"] == "The depth chart");
        let id = memo.unwrap_or_else(|| panic!("{level}: no first memo in {mail}"))["id"].clone();
        let letter = ok(&mut g, "letter", json!({ "id": id })).to_string();
        assert!(letter.contains("Day 1 of 6") && letter.contains("handbook?page=ratings"), "{level}: {letter}");

        // A task ticks; Off ends the week and the warnings.
        let card = ok(&mut g, "l8_task", json!({ "key": "depth", "done": true }));
        assert_eq!(card["done"], 1, "{card}");
        let h = ok(&mut g, "l8_help_set", json!({ "level": "off" }));
        assert_eq!(h["level"], "off", "{h}");
        assert_eq!(ok(&mut g, "l8_started", json!({}))["show"], false);
        assert!(!afm_game::lane_handbook::warnings_on(g.world.as_ref().unwrap()));
    }
    let _ = std::fs::remove_dir_all(dir);
}
