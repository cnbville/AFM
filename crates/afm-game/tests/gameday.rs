//! Game day (section 9) on a small world: Continue stops the morning of
//! your game, the game day screen reads the opponent, your game is played
//! live with your calls and the day keeps the result you saw; a game you
//! don't play live is the one the morning showed; a game saved mid-way
//! plays on after loading.

use afm_engine::game::play_game;
use afm_game::Game;
use serde_json::{Value, json};

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    match g.call(cmd, &args) {
        Ok(v) => v,
        Err(e) => panic!("{cmd}: {e}"),
    }
}

/// Continue to the morning of your next game.
fn to_game_morning(g: &mut Game) {
    for _ in 0..80 {
        let r = ok(g, "continue", json!({ "max_days": 300 }));
        if r["morning"] == json!(true) && r["why"][0]["text"].as_str().is_some_and(|t| t.starts_with("Game day")) {
            return;
        }
    }
    panic!("never reached a game morning");
}

/// What the morning builds is what the day plays.
fn dry_score(g: &Game) -> ([i32; 2], usize) {
    let w = g.world.as_ref().unwrap();
    let s = w.seat_setup_today().expect("your game today");
    let r = play_game(&s.params.setup(&s.home, &s.away));
    (r.stats.score, r.stats.plays.len())
}

/// Play the live game to the end: your calls on every snap, the toss, a
/// timeout, halftime with the staff's first adjustment.
fn play_live(g: &mut Game) -> Value {
    let mut st = ok(g, "gameday_kickoff", json!({}));
    let mut n = 0;
    let mut timeout = false;
    while st["status"] == "live" {
        n += 1;
        assert!(n < 1500, "the game never ends");
        let p = &st["pending"];
        let ask = p["ask"].as_str().unwrap();
        let input = match ask {
            "toss" => json!({ "kind": "toss", "choice": "defer" }),
            "snap" if p["staff_fourth"].is_string() => json!({ "kind": "call", "fourth": p["staff_fourth"] }),
            "snap"
                if !timeout
                    && p["view"]["running"] == true
                    && p["view"]["timeouts"][st["mine"].as_u64().unwrap() as usize].as_u64() == Some(3) =>
            {
                timeout = true;
                json!({ "kind": "timeout" })
            }
            "snap" if p["offense"].as_array().is_some_and(|o| o.len() > 1) => json!({ "kind": "call", "option": 1 }),
            "defense" if p["defense"].as_array().is_some_and(|o| o.len() > 1) => {
                json!({ "kind": "defense", "option": 1 })
            }
            "halftime" => {
                let a = &st["halftime"]["adjust"];
                let mut v = json!({ "kind": "halftime" });
                if let Some(c) = a.get(0).and_then(|x| x.get("change")) {
                    for (k, x) in c.as_object().unwrap() {
                        v[k] = x.clone();
                    }
                }
                v
            }
            _ => json!({ "kind": "staff" }),
        };
        st = ok(g, "live_answer", input);
    }
    assert_eq!(st["status"], "final");
    assert!(st["after"]["calls"]["yours"]["snaps"].as_u64().unwrap() > 40, "{}", st["after"]["calls"]);
    assert!(timeout, "a timeout was called");
    st
}

#[test]
fn game_day_from_the_morning_to_the_game_page() {
    let t = std::time::Instant::now();
    let dir = std::env::temp_dir().join(format!("afm-gameday-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    ok(&mut g, "build_world", json!({ "seed": 11, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    let job = ok(&mut g, "jobs", json!({ "level": "college" }))["jobs"][0]["ref"].clone();
    ok(
        &mut g,
        "start_career",
        json!({ "name": "Test Coach", "age": 44, "background": "college_assistant", "job": job, "seed": 3, "calls": "both" }),
    );
    println!("career started in {:.0}s", t.elapsed().as_secs_f64());

    // Game 1: played live with your calls.
    to_game_morning(&mut g);
    let hub = ok(&mut g, "gameday", json!({}));
    assert_eq!(hub["has_game"], true, "{hub}");
    assert_eq!(hub["today"], true);
    assert_eq!(hub["can_kick_off"], true, "{hub}");
    assert!(hub["weather"]["text"].is_string());
    // Every snap stops, on the learning clock.
    ok(&mut g, "gameday_settings", json!({ "stop": "every", "clock": "learning" }));
    ok(&mut g, "gameday_rules", json!({ "fourth": "numbers", "two": "chart" }));
    let fin = play_live(&mut g);
    let score: Vec<i64> = fin["view"]["score"].as_array().unwrap().iter().map(|x| x.as_i64().unwrap()).collect();
    let day = ok(&mut g, "day", json!({}));
    let k = day["game"]["index"].as_u64().expect("the day kept your game");
    let page = ok(&mut g, "game_page", json!({ "index": k }));
    let kept: Vec<i64> = page["sides"].as_array().unwrap().iter().map(|s| s["score"].as_i64().unwrap()).collect();
    assert_eq!(kept, score, "the result you played is the result kept");
    for key in ["key_plays", "drives", "snaps", "wp", "advanced", "officials", "stars"] {
        assert!(!page[key].is_null(), "the game page has {key}");
    }
    assert!(page["snaps"].as_array().unwrap().iter().filter(|s| s["yours"].as_u64().unwrap_or(0) > 0).count() > 40);
    assert!(page["drives"].as_array().unwrap().len() >= 16, "{}", page["drives"]);
    println!("game 1 live in {:.0}s: {score:?}", t.elapsed().as_secs_f64());

    // Game 2: the morning's game is the day's when the staff coaches it.
    ok(&mut g, "gameday_rules", json!({ "fourth": "staff", "two": "staff" }));
    to_game_morning(&mut g);
    let (dry, plays) = dry_score(&g);
    let day = ok(&mut g, "day", json!({}));
    let k = day["game"]["index"].as_u64().expect("the day kept your game");
    let page = ok(&mut g, "game_page", json!({ "index": k }));
    let kept: Vec<i64> = page["sides"].as_array().unwrap().iter().map(|s| s["score"].as_i64().unwrap()).collect();
    assert_eq!(kept, vec![dry[0] as i64, dry[1] as i64], "the morning built the day's game");
    assert_eq!(page["plays"].as_array().unwrap().len(), plays);
    println!("game 2 identity in {:.0}s", t.elapsed().as_secs_f64());

    // Game 3: film study, a hint in the plan, saved mid-game and loaded.
    to_game_morning(&mut g);
    let hub = ok(&mut g, "gameday", json!({}));
    let tapes = hub["tapes"].as_array().unwrap().clone();
    assert!(!tapes.is_empty(), "the opponent's film is in the film room: {}", hub["tapes"]);
    assert!(!hub["report"]["sections"].as_array().unwrap().is_empty(), "{}", hub["report"]);
    // Study the tapes until one shows a hint the plan can act on ("for
    // your eyes" ones only inform; which kinds a tape shows depends on what
    // happened in that game).
    let mut seen = Vec::new();
    let mut pick = None;
    for tape in &tapes {
        let hub = ok(&mut g, "gameday_study", tape["key"].clone());
        let studied = hub["tapes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["key"] == tape["key"])
            .expect("the tape studied")
            .clone();
        assert_eq!(studied["studied"], true);
        let hints = studied["hints"].as_array().unwrap();
        assert!(!hints.is_empty() && hints.len() <= 3, "{hints:?}");
        if let Some(h) = hints.iter().find(|h| h["plan"] == true) {
            pick = Some((studied["key"].clone(), h["key"].clone()));
            break;
        }
        seen.extend(hints.iter().cloned());
    }
    let Some((tape, hint)) = pick else { panic!("no hint the plan answers on {} tapes: {seen:?}", tapes.len()) };
    let mut add = tape;
    add["key"] = hint;
    add["add"] = json!(true);
    let hub = ok(&mut g, "gameday_hint", add);
    assert!(hub["finds"].as_array().unwrap().iter().any(|f| f["added"] == true), "{}", hub["finds"]);
    ok(&mut g, "gameday_settings", json!({ "stop": "situations" }));
    let mut st = ok(&mut g, "gameday_kickoff", json!({}));
    for _ in 0..6 {
        if st["status"] != "live" {
            break;
        }
        st = ok(&mut g, "live_answer", json!({ "kind": "staff" }));
    }
    let at = st["view"]["plays"].clone();
    let saved = ok(&mut g, "save", json!({ "name": "mid game" }))["saved"].as_str().unwrap().to_string();
    ok(&mut g, "load", json!({ "path": saved }));
    let again = ok(&mut g, "live", json!({}));
    assert_eq!(again["view"]["plays"], at, "the loaded game resumes at the same snap");
    let fin = ok(&mut g, "live_staff", json!({}));
    assert_eq!(fin["status"], "final");
    let score = fin["view"]["score"].clone();
    let day = ok(&mut g, "day", json!({}));
    let k = day["game"]["index"].as_u64().unwrap();
    let page = ok(&mut g, "game_page", json!({ "index": k }));
    assert_eq!(json!([page["sides"][0]["score"], page["sides"][1]["score"]]), score);
    println!("game day test in {:.0}s", t.elapsed().as_secs_f64());
    let _ = std::fs::remove_dir_all(dir);
}
