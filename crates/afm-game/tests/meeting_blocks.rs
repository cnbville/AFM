//! Cain 2026-11-15: with a meeting open, Continue must not carry on past it.
//! You hold it or hand it to your staff first.

mod common;

use afm_engine::world::folk::scene::{self, Kind, Scene};
use afm_game::Game;
use serde_json::{Value, json};

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    match g.call(cmd, &args) {
        Ok(v) => v,
        Err(e) => panic!("{cmd}: {e}"),
    }
}

#[test]
fn an_open_meeting_stops_continue_until_it_is_held_or_handed_on() {
    let dir = std::env::temp_dir().join(format!("afm-meeting-blocks-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    common::build_world(&mut g, json!({ "seed": 12, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    let jobs = ok(&mut g, "jobs", json!({ "level": "college" }));
    let job = jobs["jobs"][0]["ref"].clone();
    ok(&mut g, "start_career", json!({ "name": "Test Coach", "job": job, "seed": 3 }));
    // Whatever the takeover filed is your staff's: the test's own meeting is the one under test.
    ok(&mut g, "continue", json!({ "staff": true }));

    let (id, day, now) = {
        let w = g.world.as_mut().unwrap();
        let mut s = Scene::new(Kind::TeamMeeting, w.today, "A meeting that waits");
        s.minute = Some(0);
        (scene::add(w, s), w.today, w.now())
    };
    let r = ok(&mut g, "continue", json!({}));
    assert_eq!(r["blocked"]["meeting"], json!(id), "Continue names the meeting that waits");
    assert!(r["status"].is_object(), "the status still comes back for the bar");
    {
        let w = g.world.as_ref().unwrap();
        assert_eq!(w.today, day, "no day played");
        assert_eq!(w.now(), now, "the clock did not move");
        assert!(w.folk.scenes.iter().any(|s| s.id == id && !s.done), "the meeting stays open");
    }
    // Asked again, the same: it does not give way.
    assert_eq!(ok(&mut g, "continue", json!({}))["blocked"]["meeting"], json!(id));

    // Handed to your staff, the clock goes on.
    ok(&mut g, "people_hand", json!({ "id": id }));
    let r = ok(&mut g, "continue", json!({}));
    assert!(r.get("blocked").is_none(), "nothing waits now");
    assert!(g.world.as_ref().unwrap().now() > now, "the clock moved on");
    let _ = std::fs::remove_dir_all(dir);
}
