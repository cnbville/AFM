//! The draft room (update 2, lane 9C) on a small world: at an NFL club the
//! board keeps its numbers as men go, each chance he's there is a share of
//! the staff's mock and only for men still there, the mock strip names the
//! picks before yours, the calls are priced, nothing hidden shows; at a
//! college and a high school the draft outlook answers before and after
//! the draft.

use afm_game::Game;
use serde_json::{Value, json};

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    match g.call(cmd, &args) {
        Ok(v) => v,
        Err(e) => panic!("{cmd}: {e}"),
    }
}

/// No hidden number in an answer: no potential or ceiling keys, and every
/// read a range.
fn nothing_hidden(cmd: &str, v: &Value, path: &str) {
    match v {
        Value::Object(o) => {
            for (k, x) in o {
                let p = format!("{path}.{k}");
                assert!(!matches!(k.as_str(), "potential" | "ceiling" | "truth" | "overall_true"), "{cmd}{p}");
                if k == "read"
                    && let Some(a) = x.as_array()
                {
                    let (lo, hi) = (a[0].as_f64().unwrap(), a[1].as_f64().unwrap());
                    assert!(hi - lo >= 2.0, "{cmd}{p}: {x} is too sure to be a staff's read");
                }
                nothing_hidden(cmd, x, &p);
            }
        }
        Value::Array(a) => a.iter().enumerate().for_each(|(i, x)| nothing_hidden(cmd, x, &format!("{path}[{i}]"))),
        _ => {}
    }
}

fn room(g: &mut Game) -> Value {
    let v = ok(g, "dr_room", json!({}));
    nothing_hidden("dr_room", &v, "");
    let dr = &v["dr"];
    let cards = dr["cards"].as_array().unwrap();
    // Board numbers run 1, 2, 3 ... with the gone men keeping theirs.
    for (k, c) in cards.iter().enumerate() {
        assert_eq!(c["rank"].as_u64(), Some(k as u64 + 1), "{c}");
        if c["gone"].is_object() {
            assert!(c["there"].is_null(), "a gone man has no chance: {c}");
            assert!(c["gone"]["pick"].as_u64().is_some());
        } else if let Some(t) = c["there"].as_f64() {
            assert!((0.0..=100.0).contains(&t), "{c}");
        }
    }
    // On the clock everyone left is there; the strip is the picks before yours.
    if v["on_clock"] == json!(true) {
        assert_eq!(dr["target"].as_u64(), v["next"].as_u64());
        assert!(cards.iter().filter(|c| c["gone"].is_null()).all(|c| c["there"] == json!(100.0)), "{}", dr["cards"]);
        assert!(dr["strip"].as_array().unwrap().is_empty());
    }
    if let Some(at) = dr["target"].as_u64() {
        for s in dr["strip"].as_array().unwrap() {
            let p = s["pick"].as_u64().unwrap();
            assert!(p >= v["next"].as_u64().unwrap() && p < at, "{s}");
            assert_ne!(s["team"]["mine"], json!(true));
        }
    }
    for c in dr["calls"].as_array().unwrap() {
        assert!(c["give"].as_f64().unwrap() > 0.0 && c["get"].as_f64().unwrap() > 0.0, "{c}");
        assert!(c["line"]["says"].as_str().is_some_and(|s| !s.is_empty()));
    }
    v
}

#[test]
fn the_draft_room_and_the_outlook() {
    let t = std::time::Instant::now();
    let dir = std::env::temp_dir().join(format!("afm-draftroom-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    ok(&mut g, "build_world", json!({ "seed": 7, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    let fresh = ok(&mut g, "save", json!({ "name": "fresh" }))["saved"].as_str().unwrap().to_string();
    let start = |g: &mut Game, job: &Value, bg: &str| {
        ok(g, "load", json!({ "path": &fresh }));
        ok(g, "start_career", json!({ "name": "Test Coach", "age": 45, "background": bg, "job": job, "seed": 3 }));
    };
    // A college: the outlook before the draft.
    let jobs = ok(&mut g, "jobs", json!({ "level": "college" }))["jobs"].as_array().unwrap().clone();
    start(&mut g, &jobs[0]["ref"], "college_assistant");
    let o = ok(&mut g, "dr_outlook", json!({}));
    nothing_hidden("dr_outlook", &o, "");
    assert_eq!(o["seat"], "college");
    assert!(o["rows"].as_array().unwrap().iter().all(|r| r["status"].as_str().is_some_and(|s| !s.is_empty())));
    assert!(o["rows"].as_array().unwrap().iter().all(|r| r["invites"].is_array()), "the Squad's Draft view reads them");
    assert!(g.call("dr_room", &json!({})).is_ok(), "the room answers off draft night");
    // The Visits desk invites from your board (SC7): each target's open
    // days by kind of visit.
    let v = ok(&mut g, "visits_desk", json!({}));
    if let Some(t) = v["targets"].as_array().and_then(|t| t.first()) {
        let opts = ok(&mut g, "visit_options", json!({ "id": t["id"] }));
        assert_eq!(opts["options"].as_array().unwrap().len(), 4, "{opts}");
    }

    // An NFL club with an early pick.
    ok(&mut g, "load", json!({ "path": &fresh }));
    let jobs = ok(&mut g, "jobs", json!({ "level": "nfl" }))["jobs"].as_array().unwrap().clone();
    let mut best: Option<(u64, Value)> = None;
    for j in &jobs {
        start(&mut g, &j["ref"], "nfl_assistant");
        let picks = ok(&mut g, "draft_board", json!({}))["picks"].clone();
        if let Some(pick) = picks[0]["pick"].as_u64()
            && best.as_ref().is_none_or(|b| pick < b.0)
        {
            best = Some((pick, j["ref"].clone()));
            if picks[0]["round"].as_u64().is_some_and(|r| r <= 2) {
                break;
            }
        }
    }
    let (_, job) = best.expect("an NFL job with a pick");
    start(&mut g, &job, "nfl_assistant");
    assert!(g.call("dr_outlook", &json!({})).is_err(), "the outlook is for a college or a high school");
    let mut steps = 0;
    while !ok(&mut g, "draft_live", json!({}))["live"].as_bool().unwrap() && steps < 300 {
        ok(&mut g, "continue", json!({ "max_days": 60 }));
        steps += 1;
    }
    println!("to the draft in {:.0}s", t.elapsed().as_secs_f64());
    let r = room(&mut g);
    assert!(r["on_clock"].as_bool().unwrap());
    assert!(r["dr"]["cards"].as_array().unwrap().len() >= 10);
    assert!(r["dr"]["needs"].as_array().is_some_and(|n| !n.is_empty()), "your needs");
    // The same room gives the same mock.
    assert_eq!(ok(&mut g, "dr_room", json!({}))["dr"], r["dr"]);
    let first = r["staff_pick"]["id"].as_u64().unwrap();
    ok(&mut g, "draft_pick", json!({ "id": first }));
    // Between picks: chances below 100 for some, the mock strip filled.
    let r = room(&mut g);
    let mine = r["dr"]["cards"].as_array().unwrap().iter().find(|c| c["id"].as_u64() == Some(first)).cloned();
    assert!(mine.is_some_and(|c| c["gone"]["team"]["mine"] == json!(true)), "your pick stays on the board, struck");
    if r["dr"]["target"].as_u64().is_some_and(|t| t > r["next"].as_u64().unwrap()) {
        assert!(!r["dr"]["strip"].as_array().unwrap().is_empty(), "{}", r["dr"]);
        let some_below =
            r["dr"]["cards"].as_array().unwrap().iter().any(|c| c["there"].as_f64().is_some_and(|x| x < 100.0));
        assert!(some_below, "some men may go before your pick");
    }
    let mut guard = 0;
    let mut r = r;
    while !r["rush"].as_bool().unwrap() && guard < 60 {
        if r["on_clock"].as_bool().unwrap() {
            ok(&mut g, "draft_auto", json!({}));
        } else {
            ok(&mut g, "draft_go", json!({ "until": "yours" }));
        }
        r = room(&mut g);
        guard += 1;
    }
    assert!(r["rush"].as_bool().unwrap(), "every pick is in");
    ok(&mut g, "draft_rush", json!({}));

    // A high school: its former players after the draft (a fresh career
    // on the world as it was: its last draft is the one before).
    ok(&mut g, "load", json!({ "path": &fresh }));
    let jobs = ok(&mut g, "jobs", json!({ "level": "hs" }))["jobs"].as_array().unwrap().clone();
    start(&mut g, &jobs[0]["ref"], "college_assistant");
    let o = ok(&mut g, "dr_outlook", json!({}));
    nothing_hidden("dr_outlook", &o, "");
    assert_eq!(o["seat"], "hs");
    assert!(
        o["rows"].as_array().unwrap().iter().all(|r| r["advisory"].is_null()),
        "the committee writes to the college"
    );
    println!("done in {:.0}s", t.elapsed().as_secs_f64());
    let _ = std::fs::remove_dir_all(dir);
}
