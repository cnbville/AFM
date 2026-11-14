//! Cain 2026-11-14: a pop-up said a player committed, and nothing was in the
//! inbox (or it came hours later). The pop-up (the reveal) and the letter
//! are now one event: the letter is stamped at the stop that shows the news,
//! and the pop-up is held until its letter is filed.

use afm_engine::world::Stage;
use afm_game::Game;
use afm_game::views::{self, TeamRef};
use serde_json::{Value, json};

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    match g.call(cmd, &args) {
        Ok(v) => v,
        Err(e) => panic!("{cmd}: {e}"),
    }
}

#[test]
fn a_commitment_pop_up_and_its_letter_arrive_at_the_same_stop() {
    let dir = std::env::temp_dir().join(format!("afm-inbox-commit-sync-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    ok(&mut g, "build_world", json!({ "seed": 12, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    let jobs = ok(&mut g, "jobs", json!({ "level": "college" }));
    let job = jobs["jobs"][0]["ref"].clone();
    ok(&mut g, "start_career", json!({ "name": "Test Coach", "job": job, "seed": 3 }));
    // Reveals handed over so far (the welcome) are not the ones under test.
    ok(&mut g, "reveals", json!({}));
    ok(&mut g, "continue", json!({}));
    ok(&mut g, "reveals", json!({}));

    // A four-star senior commits to us today, in the middle of the day.
    let (name, id) = {
        let w = g.world.as_mut().unwrap();
        let Some(TeamRef::College(pi)) = views::mine(w) else { panic!("a college seat") };
        let today = w.today;
        let p = w
            .people
            .iter_mut()
            .find(|p| p.stage == Stage::HighSchool && p.cal.commit.is_none() && p.cal.signed.is_none())
            .expect("a high school recruit");
        p.stars = 4;
        p.cal.commit = Some((pi, today, false));
        (p.name.clone(), p.player.id)
    };
    ok(&mut g, "continue", json!({}));
    {
        let w = g.world.as_ref().unwrap();
        let l = w.local_now();
        assert_eq!(l.date, w.today, "the stop is on the same day");
        assert!(l.minute < 21 * 60, "a stop before the evening hour the letter used to wait for: {}", l.minute);
    }
    let reveals = ok(&mut g, "reveals", json!({}));
    let popped = reveals.as_array().unwrap().iter().any(|r| r["name"] == name.as_str() && r["kind"] == "commit");
    let mail = ok(&mut g, "mail", json!({}));
    let lettered = mail["rows"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["subject"].as_str().is_some_and(|s| s.starts_with("Commitment:") && s.ends_with(&name)));
    assert!(lettered, "the commitment is in the inbox at the stop that shows it (id {id}): {mail}");
    assert!(popped, "the pop-up shows with it: {reveals}");

    // Told once: another stop writes neither again.
    ok(&mut g, "continue", json!({}));
    assert!(
        !ok(&mut g, "reveals", json!({})).as_array().unwrap().iter().any(|r| r["name"] == name.as_str()),
        "the pop-up is not shown twice"
    );
    let mail = ok(&mut g, "mail", json!({}));
    let n = mail["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["subject"].as_str().is_some_and(|s| s.starts_with("Commitment:") && s.ends_with(&name)))
        .count();
    assert_eq!(n, 1, "one letter, not two: {mail}");
    std::fs::remove_dir_all(dir).ok();
}
