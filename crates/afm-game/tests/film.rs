//! The film room (later extras topics 2 and 3): a college career's first
//! game is played again from its seed with frames on and every snap comes
//! out whole, in the stadium's frame, with a jersey number for every man.

use afm_game::Game;
use serde_json::{Value, json};

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    match g.call(cmd, &args) {
        Ok(v) => v,
        Err(e) => panic!("{cmd}: {e}"),
    }
}

#[test]
fn a_kept_game_plays_back_as_film() {
    let dir = std::env::temp_dir().join(format!("afm-film-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    // A saved career with a game kept loads straight in (quick runs while
    // building the views); otherwise a small world and a college job.
    if let Ok(p) = std::env::var("AFM_FILM_LOAD") {
        ok(&mut g, "load", json!({ "path": p }));
    } else {
        ok(&mut g, "build_world", json!({ "seed": 7, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
        let jobs = ok(&mut g, "jobs", json!({ "level": "college" }));
        let job = jobs["jobs"][0]["ref"].clone();
        ok(
            &mut g,
            "start_career",
            json!({ "name": "Film Coach", "age": 44, "background": "college_assistant", "job": job, "seed": 5, "home_state": "TX" }),
        );
    }
    let mut tries = 0;
    while ok(&mut g, "games", json!({}))["games"].as_array().unwrap().is_empty() {
        ok(&mut g, "continue", json!({ "max_days": 400 }));
        tries += 1;
        assert!(tries < 40, "no game was kept");
    }
    let keep = std::env::var("AFM_FILM_SAVE").ok();
    if let Some(p) = &keep {
        ok(&mut g, "save", json!({ "name": p }));
        println!("saved under {}", dir.display());
    }
    let k = ok(&mut g, "games", json!({}))["games"][0]["index"].as_u64().unwrap();
    let t = std::time::Instant::now();
    let film = ok(&mut g, "film", json!({ "index": k }));
    println!("film opened in {:.0} ms", t.elapsed().as_secs_f64() * 1000.0);
    let players = film["players"].as_array().unwrap();
    assert!(players.len() > 80, "{} players", players.len());
    // Every man has a number, unique on his team.
    for side in 0..2 {
        let mut nums: Vec<u64> = players.iter().filter(|p| p[1] == side).map(|p| p[2].as_u64().unwrap()).collect();
        let n = nums.len();
        nums.sort();
        nums.dedup();
        assert_eq!(nums.len(), n, "two men share a number");
    }
    assert!(film["venue"]["seats"].as_u64().unwrap() > 1000, "{}", film["venue"]);
    let plays = film["plays"].as_array().unwrap();
    assert!(plays.len() > 100);
    let mut graded = 0;
    let mut kinds = std::collections::BTreeMap::new();
    for p in plays {
        let n = p["n"].as_u64().unwrap();
        let clip = ok(&mut g, "film_play", json!({ "index": k, "n": n }));
        let stride = clip["stride"].as_u64().unwrap() as usize;
        let ids = clip["ids"].as_array().unwrap().len();
        assert_eq!(stride, 6 + ids * 5);
        let f = clip["f"].as_array().unwrap();
        assert!(!f.is_empty() && f.len().is_multiple_of(stride), "play {n}: {} numbers", f.len());
        let frames = f.len() / stride;
        assert!(frames >= 20, "play {n} ({}) has {frames} frames", p["kind"]);
        *kinds.entry(p["kind"].as_str().unwrap().to_string()).or_insert(0) += 1;
        // Every spot sits on or near the field, in the stadium's frame.
        for fr in f.chunks(stride) {
            for k in 0..ids {
                let x = fr[6 + k * 5].as_f64().unwrap();
                let y = fr[7 + k * 5].as_f64().unwrap();
                if !((-15.0..=135.0).contains(&x) && (-6.0..=60.0).contains(&y)) {
                    // Where this man went over the snap, and the ball.
                    for f2 in f.chunks(stride).step_by(3) {
                        eprintln!(
                            "t {:.1} man ({:.1}, {:.1}) act {} ball ({:.1}, {:.1}, {:.1}) mode {} holder {}",
                            f2[0].as_f64().unwrap(),
                            f2[6 + k * 5].as_f64().unwrap(),
                            f2[7 + k * 5].as_f64().unwrap(),
                            f2[9 + k * 5],
                            f2[1].as_f64().unwrap(),
                            f2[2].as_f64().unwrap(),
                            f2[3].as_f64().unwrap(),
                            f2[4],
                            f2[5]
                        );
                    }
                    eprintln!(
                        "man {} of {} ({}), clip {}",
                        k,
                        ids,
                        clip["ids"][k],
                        json!({ "los": clip["los"], "dir": clip["dir"], "team": clip["team"], "target": clip["target"], "carrier": clip["carrier"], "defender": clip["defender"] })
                    );
                }
                assert!(
                    (-15.0..=135.0).contains(&x) && (-6.0..=60.0).contains(&y),
                    "play {n} ({}, {}) at {} s: a man at ({x}, {y})",
                    p["kind"],
                    p["text"],
                    fr[0]
                );
            }
        }
        graded += clip["grades"].as_object().unwrap().len();
    }
    println!("kinds {kinds:?}, {graded} grades");
    assert!(graded > 500, "{graded} grades");
    ok(
        &mut g,
        "film_marks",
        json!({ "index": k, "marks": [{ "play": plays[3]["n"], "t": 1.5, "kind": 1, "color": 0, "pts": [[40.0, 20.0], [50.0, 25.0]] }] }),
    );
    assert_eq!(ok(&mut g, "film", json!({ "index": k }))["marks"].as_array().unwrap().len(), 1);
    ok(&mut g, "film_close", json!({}));
    if keep.is_none() {
        let _ = std::fs::remove_dir_all(dir);
    }
}
