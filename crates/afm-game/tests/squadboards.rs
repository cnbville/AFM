//! The player and roster boards (update 2, lane 9A) on a small world, at a
//! college, an NFL and a high school seat: every board answers, every
//! rating in them is your staff's range (never one number, never a
//! ceiling), the needs map's starters and the career arc's reads are the
//! roster's, a backup moved up on the stadium board is the order the
//! staff plays, and the reads kept at a season's end survive a save while
//! a save from before them still loads.

use afm_game::Game;
use serde_json::{Value, json};

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    match g.call(cmd, &args) {
        Ok(v) => v,
        Err(e) => panic!("{cmd}: {e}"),
    }
}

/// No hidden number anywhere in an answer: no potential or ceiling keys,
/// and every read a range at least two wide, or one wide where it touches
/// the 1-99 scale's end (`reads::range` clamps, so a 99 can read 98-99).
fn only_ranges(cmd: &str, v: &Value, path: &str) {
    match v {
        Value::Object(o) => {
            for (k, x) in o {
                let p = format!("{path}.{k}");
                assert!(
                    !matches!(k.as_str(), "potential" | "ceiling" | "true" | "truth" | "pot" | "overall_true"),
                    "{cmd}{p}: a hidden number"
                );
                if (k == "read" || k == "ovr")
                    && let Some(a) = x.as_array()
                {
                    assert_eq!(a.len(), 2, "{cmd}{p}: {x}");
                    let (lo, hi) = (a[0].as_f64().unwrap(), a[1].as_f64().unwrap());
                    let least = if lo <= 1.0 || hi >= 99.0 { 1.0 } else { 2.0 };
                    assert!(hi - lo >= least, "{cmd}{p}: {x} is too sure to be a staff's read");
                }
                only_ranges(cmd, x, &p);
            }
        }
        Value::Array(a) => {
            for (i, x) in a.iter().enumerate() {
                only_ranges(cmd, x, &format!("{path}[{i}]"));
            }
        }
        _ => {}
    }
}

fn board(g: &mut Game, cmd: &str, args: Value) -> Value {
    let v = ok(g, cmd, args);
    only_ranges(cmd, &v, "");
    v
}

/// Every board at the seat you hold; returns the roster's rows.
fn every_board(g: &mut Game, level: &str) -> Vec<Value> {
    let d = board(g, "sb_depth", json!({}));
    let tabs = d["tabs"].as_array().unwrap().clone();
    assert_eq!(tabs.len(), 4, "{level}: offense, defense, special teams, returners");
    for t in &tabs {
        for f in t["forms"].as_array().unwrap() {
            let v = board(g, "sb_depth", json!({ "tab": t["key"], "form": f["key"] }));
            assert_eq!(v["form"], f["key"], "{level}");
            for s in v["slots"].as_array().unwrap() {
                assert_eq!(s["picks"].as_array().map(Vec::len), Some(3), "{level}: three deep at {}", s["slot"]);
            }
        }
    }
    let off = board(g, "sb_depth", json!({ "tab": "offense", "form": "offense" }));
    assert_eq!(off["slots"].as_array().map(Vec::len), Some(11), "{level}: eleven on the field");
    assert!(
        off["slots"].as_array().unwrap().iter().all(|s| s["picks"][0].is_object()),
        "{level}: a starter at every spot"
    );
    assert!(off["units"].as_array().is_some_and(|u| u.len() == 6), "{level}: {}", off["units"]);

    let roster = board(g, "sb_roster", json!({}));
    let rows = roster["rows"].as_array().unwrap().clone();
    assert!(!rows.is_empty(), "{level}: a roster");
    let ids: Vec<u64> = rows.iter().filter_map(|r| r["id"].as_u64()).collect();

    // The needs map's starters now are on the roster, with the roster's read.
    for year in 0..3 {
        let n = board(g, "sb_needs", json!({ "year": year }));
        assert_eq!(n["spots"].as_array().map(Vec::len), Some(24), "{level}");
        assert_eq!(n["years"].as_array().map(Vec::len), Some(3), "{level}");
        if year > 0 {
            continue;
        }
        for s in n["spots"].as_array().unwrap() {
            let Some(id) = s["names"][0]["id"].as_u64() else { continue };
            if s["names"][0]["commit"] == json!(true) {
                continue;
            }
            assert!(
                ids.contains(&id),
                "{level}: {} starts at {} but isn't on the roster",
                s["names"][0]["name"],
                s["key"]
            );
            let r = rows.iter().find(|r| r["id"].as_u64() == Some(id)).unwrap();
            if let (Some(o), Some(v)) = (r["ovr"].as_array(), s["vals"][0].as_f64())
                && r["pos"] == s["pos"]
            {
                let m = (o[0].as_f64().unwrap() + o[1].as_f64().unwrap()) / 2.0;
                assert!((m - v).abs() <= 0.5, "{level}: {} reads {m} on the roster, {v} on the needs map", s["key"]);
            }
        }
    }

    // A few players' shape, career arc and body: today's read is the roster's.
    for r in rows.iter().step_by(7).take(4) {
        let id = r["id"].clone();
        let sh = board(g, "sb_shape", json!({ "id": id }));
        assert_eq!(sh["families"].as_array().map(Vec::len), Some(8), "{level}");
        assert_eq!(sh["read"], r["ovr"], "{level}: the shape's read is the roster's");
        let arc = board(g, "sb_arc", json!({ "id": id }));
        let now = arc["points"].as_array().unwrap().last().unwrap().clone();
        assert_eq!(now["now"], true);
        assert_eq!(now["read"], r["ovr"], "{level}: the arc's read today is the roster's");
        assert!(arc["peak"].is_null() || arc["peak"]["v"].is_number(), "{level}: {}", arc["peak"]);
        let seasons: Vec<i64> = arc["table"].as_array().unwrap().iter().filter_map(|t| t["season"].as_i64()).collect();
        assert!(seasons.windows(2).all(|x| x[0] < x[1]), "{level}: one row a season, in order: {seasons:?}");
        let body = board(g, "sb_body", json!({ "id": id }));
        assert_eq!(body["compares"].as_array().map(Vec::len), Some(3), "{level}");
        assert!(body["rows"].as_array().is_some_and(|x| x.iter().any(|t| t["key"] == "forty")), "{level}");
    }
    rows
}

#[test]
fn the_boards_at_every_seat() {
    let t = std::time::Instant::now();
    let dir = std::env::temp_dir().join(format!("afm-squadboards-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    ok(&mut g, "build_world", json!({ "seed": 17, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    println!("world built in {:.0}s", t.elapsed().as_secs_f64());
    let fresh = ok(&mut g, "save", json!({ "name": "fresh" }))["saved"].as_str().unwrap().to_string();
    let base =
        json!({ "name": "Test Coach", "age": 46, "background": "college_assistant", "seed": 5, "calls": "none" });
    let start = |g: &mut Game, job: Value| {
        let mut a = base.clone();
        a["job"] = job;
        ok(g, "start_career", a);
    };

    // A college head coach, a game into the season.
    let job = ok(&mut g, "jobs", json!({ "level": "college" }))["jobs"][0]["ref"].clone();
    start(&mut g, job);
    // Continue stops for the day's news too: step until a game is kept.
    let mut steps = 0;
    while ok(&mut g, "games", json!({}))["games"].as_array().unwrap().is_empty() && steps < 400 {
        ok(&mut g, "continue", json!({ "max_days": 300 }));
        steps += 1;
    }
    let rows = every_board(&mut g, "college");
    assert!(
        rows.iter().any(|r| r["last6"].as_array().is_some_and(|x| x.iter().any(Value::is_number))),
        "grades from the game"
    );

    // A backup moved up is the order the staff plays; given back, it's theirs.
    let off = ok(&mut g, "sb_depth", json!({ "tab": "offense", "form": "offense" }));
    let k = off["slots"].as_array().unwrap().iter().position(|s| s["slot"] == "QB").unwrap();
    let (first, second) = (off["slots"][k]["picks"][0]["id"].clone(), off["slots"][k]["picks"][1]["id"].clone());
    let moved = ok(&mut g, "sb_depth_up", json!({ "tab": "offense", "form": "offense", "slot": k, "n": 2 }));
    assert_eq!(moved["slots"][k]["picks"][0]["id"], second, "the backup starts");
    assert_eq!(moved["slots"][k]["picks"][1]["id"], first, "the starter is second");
    assert_eq!(moved["yours"], true);
    let order = ok(&mut g, "depth", json!({}));
    let qbs = order["positions"].as_array().unwrap().iter().find(|p| p["pos"] == "QB").unwrap().clone();
    assert_eq!(qbs["players"][0]["id"], second, "the flat depth chart agrees");
    let back = ok(&mut g, "sb_depth_reset", json!({ "tab": "offense", "form": "offense" }));
    assert_eq!(back["yours"], false, "the staff has it back");

    // The reads kept at a season's end: a career's first turn keeps none
    // (it starts mid-year), the next keeps every player the staff follows,
    // with the read the boards showed that day.
    let id = rows[0]["id"].as_u64().unwrap() as u32;
    {
        let w = g.world.as_mut().unwrap();
        let season = w.year - 1;
        w.career.as_mut().unwrap().kept_through = 0;
        afm_game::lane_squadboards::after_days(w);
        let c = w.career.as_ref().unwrap();
        assert_eq!(c.kept_through as i32, season, "the first turn only marks the season");
        assert!(c.kept_reads.is_empty(), "nothing kept for a season the staff didn't follow");
        w.career.as_mut().unwrap().kept_through = (season - 1) as i16;
        afm_game::lane_squadboards::after_days(w);
        let c = w.career.as_ref().unwrap();
        assert_eq!(c.kept_of(id).len(), 1, "a roster player's read is kept");
        assert_eq!(c.kept_of(id)[0].season as i32, season);
    }
    let kept = g.world.as_ref().unwrap().career.as_ref().unwrap().kept_of(id)[0];
    assert_eq!(json!(kept.ovr), rows[0]["ovr"], "the kept read is the roster's that day");
    let arc = board(&mut g, "sb_arc", json!({ "id": id }));
    assert!(
        arc["points"].as_array().unwrap().iter().any(|p| p["now"] == false && p["read"] == rows[0]["ovr"]),
        "the arc shows the kept read: {}",
        arc["points"]
    );
    // Kept through a save; a career saved before the kept reads loads empty.
    let saved = ok(&mut g, "save", json!({ "name": "kept" }))["saved"].as_str().unwrap().to_string();
    ok(&mut g, "load", json!({ "path": saved }));
    assert_eq!(g.world.as_ref().unwrap().career.as_ref().unwrap().kept_of(id), &[kept], "kept through a save");
    let mut old = serde_json::to_value(g.world.as_ref().unwrap().career.as_ref().unwrap()).unwrap();
    old.as_object_mut().unwrap().remove("kept_reads");
    old.as_object_mut().unwrap().remove("kept_through");
    let old: afm_engine::world::seat::Career = serde_json::from_value(old).expect("an older career loads");
    assert!(old.kept_reads.is_empty() && old.kept_through == 0);

    // An NFL head coach.
    ok(&mut g, "load", json!({ "path": fresh.clone() }));
    let job = ok(&mut g, "jobs", json!({ "level": "nfl" }))["jobs"][0]["ref"].clone();
    start(&mut g, job);
    let rows = every_board(&mut g, "nfl");
    assert!(rows.iter().any(|r| r["money"]["label"] == "Cap"), "contracts on the NFL roster");

    // A high school head coach at the biggest public school in Texas.
    ok(&mut g, "load", json!({ "path": fresh }));
    let hs = ok(&mut g, "jobs", json!({ "level": "hs" }));
    let big = hs["jobs"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|j| j["private"] != true && j["state"] == "TX")
        .max_by_key(|j| j["enrollment"].as_u64().unwrap_or(0))
        .unwrap()["ref"]
        .clone();
    start(&mut g, big);
    let rows = every_board(&mut g, "high school");
    assert!(rows.iter().all(|r| r["years"]["n"] == 4), "four years of high school");
    println!("done in {:.0}s", t.elapsed().as_secs_f64());
    let _ = std::fs::remove_dir_all(dir);
}
