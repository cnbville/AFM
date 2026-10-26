//! The deeper game plan (update 2, lane 5) on a small world: a guideline
//! you set is the one the staff plays to, the staff's drafted sheet is
//! never empty and you can approve it, the standing game rules are the
//! ones game day starts from, and game day changes them for one game only.

use afm_game::Game;
use serde_json::{Value, json};

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    match g.call(cmd, &args) {
        Ok(v) => v,
        Err(e) => panic!("{cmd}: {e}"),
    }
}

/// Continue until a game is no more than a week ahead.
fn to_game_week(g: &mut Game) {
    for _ in 0..120 {
        let hub = ok(g, "gameday", json!({}));
        if hub["has_game"] == json!(true) && hub["in"].as_i64().is_some_and(|d| d <= 6) {
            return;
        }
        ok(g, "continue", json!({ "max_days": 7 }));
    }
    panic!("no game week came");
}

fn row<'a>(id: &'a Value, key: &str) -> &'a Value {
    id["rows"].as_array().unwrap().iter().find(|r| r["key"] == key).unwrap_or_else(|| panic!("no {key} row"))
}

#[test]
fn the_game_plan_from_the_identity_to_game_day() {
    let t = std::time::Instant::now();
    let dir = std::env::temp_dir().join(format!("afm-gameplan-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    ok(&mut g, "build_world", json!({ "seed": 13, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    let job = ok(&mut g, "jobs", json!({ "level": "college" }))["jobs"][0]["ref"].clone();
    ok(
        &mut g,
        "start_career",
        json!({ "name": "Test Coach", "age": 46, "background": "college_assistant", "job": job, "seed": 5, "calls": "none" }),
    );
    println!("career started in {:.0}s", t.elapsed().as_secs_f64());

    // The identity: every row, a guideline set firmly and kept.
    let id = ok(&mut g, "dp_identity", json!({}));
    for key in ["front", "man", "pressure", "system", "personnel", "pace", "run", "stick"] {
        let r = row(&id, key);
        assert!(r["name"].is_string() && r["firm_labels"].as_array().is_some_and(|f| f.len() == 3), "{r}");
    }
    assert_eq!(id["coords"].as_array().map(Vec::len), Some(2), "{}", id["coords"]);
    let id = ok(&mut g, "dp_guide", json!({ "key": "man", "to": 60, "firm": "firm" }));
    assert_eq!(row(&id, "man")["value"], json!(60.0));
    assert_eq!(row(&id, "man")["firm"], "firm");
    let id = ok(
        &mut g,
        "dp_guide",
        json!({ "changes": [{ "key": "front", "to": "4-3", "firm": "lean" }, { "key": "pace", "to": "mixed" }] }),
    );
    assert_eq!(row(&id, "front")["value"], "4-3");
    assert!(g.call("dp_guide", &json!({ "key": "system", "to": "wing_t" })).is_err(), "no wing-T in college");
    ok(&mut g, "dp_talk", json!({}));

    // The standing rules: what game day starts from.
    let rules = ok(&mut g, "dp_rules_set", json!({ "fourth": "aggressive", "two": "chart", "timeouts": "keep_two" }));
    assert_eq!(rules["rules"]["fourth"], "aggressive", "{}", rules["rules"]);
    let rules = ok(&mut g, "dp_rules_set", json!({ "cell": [0, 4, "punt"] }));
    assert_eq!(rules["grid"][0]["cells"][4]["v"], "Punt", "{}", rules["grid"][0]);
    assert_eq!(rules["rules"]["fourth"], "aggressive", "a cell keeps the preset");

    // The week: the drafted sheet is never empty, and you approve it.
    to_game_week(&mut g);
    for side in ["offense", "defense"] {
        let sheet = ok(&mut g, "dp_sheet", json!({ "side": side }));
        assert_eq!(sheet["has"], true, "{sheet}");
        assert!(!sheet["script"]["plays"].as_array().unwrap().is_empty(), "{side} has its script");
        for b in sheet["grid"].as_array().unwrap().iter().chain(sheet["extras"].as_array().unwrap()) {
            assert!(!b["plays"].as_array().unwrap().is_empty(), "{side}: {} is empty", b["name"]);
        }
        let done = ok(&mut g, "dp_sheet_review", json!({ "side": side, "approve": true }));
        assert_eq!(done["all_approved"], true, "{}", done["parts"]);
    }
    let w = g.world.as_ref().unwrap();
    assert!(w.plans.draft.as_ref().is_some_and(|d| d.every_box_filled()));

    // Game day starts from the standing rules, and changes them for this
    // game only.
    let hub = ok(&mut g, "gameday", json!({}));
    assert_eq!(hub["rules"]["fourth"], "aggressive", "{}", hub["rules"]);
    assert_eq!(hub["rules"]["timeouts"], "keep_two");
    assert_eq!(hub["rules"]["changed"], false);
    let hub = ok(&mut g, "gameday_rules", json!({ "fourth": "conservative" }));
    assert_eq!(hub["rules"]["fourth"], "conservative");
    assert_eq!(hub["rules"]["changed"], true);
    assert_eq!(hub["rules"]["cells"], true, "a game day preset keeps your cells");
    let rules = ok(&mut g, "dp_rules", json!({}));
    assert_eq!(rules["rules"]["fourth"], "aggressive", "the standing rules stay");
    assert_eq!(rules["week"]["changed"], true);

    // The week plan and the other boards answer.
    let week = ok(&mut g, "dp_week", json!({}));
    assert!(week["answers"].as_array().is_some_and(|a| !a.is_empty()), "{week}");
    assert_eq!(week["week_rules"]["fourth"], "conservative");
    for (cmd, args) in [
        ("dp_scout", json!({ "side": "offense" })),
        ("dp_scout", json!({ "side": "defense" })),
        ("dp_installs", json!({ "unit": "offense" })),
        ("dp_installs", json!({ "unit": "defense" })),
        ("dp_packages", json!({ "board": "Nickel" })),
        ("dp_like_us", json!({ "side": "offense" })),
        ("dp_fit_staff", json!({ "role": "dc" })),
        ("dp_fit_recruits", json!({ "pos": "CB" })),
    ] {
        let v = ok(&mut g, cmd, args);
        let text = v.to_string().to_lowercase();
        for bad in ["nan", "undefined", "infinity"] {
            assert!(!text.contains(&format!("\"{bad}\"")) && !text.contains(&format!(":{bad}")), "{cmd} shows {bad}");
        }
    }
    // The boards read the two-deep the engine dresses: your staff's order
    // where you set none.
    let inst = ok(&mut g, "dp_installs", json!({ "unit": "offense" }));
    assert_eq!(inst["week"], true);
    assert!(inst["rows"].as_array().is_some_and(|r| r.len() >= 8), "{}", inst["rows"]);
    // The week's chart runs to the game's eve, and a projected last bar is
    // the unit's game-day grasp.
    let chart = inst["chart"].as_array().cloned().unwrap_or_default();
    assert!(!chart.is_empty() && chart.len() <= 6, "{}", inst["chart"]);
    if let Some(last) = chart.last().filter(|c| c["proj"] == true) {
        assert_eq!(last["v"], inst["total"]["friday"], "{}", inst["chart"]);
    }
    assert!(week["two_deep"].as_array().is_some_and(|r| !r.is_empty()), "{}", week["two_deep"]);
    assert!(week["orders"][0]["players"].as_array().is_some_and(|r| !r.is_empty()), "{}", week["orders"]);

    // The game is played to this game's rules; the next week is back on
    // the standing ones.
    let before = ok(&mut g, "gameday", json!({}))["date"].clone();
    for _ in 0..20 {
        ok(&mut g, "continue", json!({ "max_days": 3 }));
        let hub = ok(&mut g, "gameday", json!({}));
        if hub["date"] != before {
            assert_eq!(hub["rules"]["fourth"], "aggressive", "back on the standing rules: {}", hub["rules"]);
            assert_eq!(hub["rules"]["changed"], false);
            break;
        }
    }
    println!("game plan test in {:.0}s", t.elapsed().as_secs_f64());
    let _ = std::fs::remove_dir_all(dir);
}
