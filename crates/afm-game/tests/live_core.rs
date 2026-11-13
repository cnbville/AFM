//! Live talks, the core (lane 3), on a real save: a typed line replays the
//! same from the same save and the same reading, and equals the clicked
//! chip; every reply of a typed talk has nothing hidden; and a save made
//! with live talks loads and plays with no key. It saves and loads a world,
//! so it runs in Full tests only.

mod common;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use afm_ai::Fake;
use afm_ai::fake::body;
use afm_engine::world::coach::{Group, Role};
use afm_engine::world::data::Division;
use afm_engine::world::folk::scene;
use afm_engine::world::folk::{Team, staffing};
use afm_engine::world::seat::{Job, You};
use afm_engine::world::{World, WorldOptions};
use afm_game::Game;
use serde_json::{Value, json};

fn you() -> You {
    let teaching = vec![12.0; afm_engine::world::coach::GROUP_COUNT];
    serde_json::from_value(json!({
        "name": "Test Coach", "age": 45, "ethnicity": "White", "home_state": 0, "ex_pro": false, "reputation": 60.0,
        "teaching": teaching, "scheme": 12.0, "game_management": 12.0,
        "motivation": 12.0, "discipline": 12.0, "recruiting": 12.0, "network": 12.0, "relationships": 12.0,
        "style": { "risk": 10.0, "freedom": 10.0, "physical": 10.0, "ball_hunting": 10.0, "mould": 10.0 },
        "yours": { "play_calling": 12.0, "adaptation": 12.0, "evaluation": 12.0, "negotiating": 12.0 }
    }))
    .unwrap()
}

fn home(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("afm-live-core-it-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn game(name: &str, fake: bool) -> (Game, Arc<Fake>) {
    let mut g = Game::new(home(name));
    let mut w = World::empty(WorldOptions { scale: 0.001, history_years: 0, start_year: 2037, ..Default::default() });
    let job = Job::College(w.programs.iter().position(|p| p.division == Division::FBS).unwrap());
    w.start_career(&you(), job, 7);
    g.world = Some(w);
    let f = Arc::new(Fake::new());
    if fake {
        g.ai.use_fake(f.clone());
    }
    (g, f)
}

/// Every answer walked for hidden numbers.
fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    let v = g.call(cmd, &args).unwrap_or_else(|e| panic!("{cmd}: {e}"));
    common::nothing_hidden(cmd, &v, "");
    v
}

fn finish(g: &mut Game, ticket: u64) -> Value {
    let t = Instant::now();
    loop {
        let v = ok(g, "ai_poll", json!({ "ticket": ticket }));
        if v["state"] != "wait" || t.elapsed() > Duration::from_secs(5) {
            return v;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn reading(pick: &str) -> Value {
    let v = json!({ "pick": pick, "also": "none", "tone": "firm", "promise": null, "claims": [], "named": [], "fit": "clear" });
    body(&v.to_string(), 700, 60)
}

fn interview(g: &mut Game) -> u32 {
    let w = g.world.as_mut().unwrap();
    let team = Team::seat(w).unwrap();
    let c = (0..w.coaches.len())
        .find(|&c| w.coaches[c].job.is_none_or(|j| Team::from_employer(j.employer) != team))
        .unwrap();
    let id = staffing::start_interview(w, c, Role::PositionCoach(Group::QB)).unwrap();
    ok(g, "people_scene", json!({ "id": id }));
    id
}

fn tag(g: &Game, id: u32, code: u32) -> String {
    let s = g.world.as_ref().unwrap().folk.scenes.iter().find(|s| s.id == id).unwrap();
    format!("o{}", scene::options(s).iter().position(|p| p.code == code).unwrap() + 1)
}

/// The world's talk as it stands, typed words aside.
fn snapshot(g: &Game) -> String {
    let w = g.world.as_ref().unwrap();
    let mut scenes = w.folk.scenes.clone();
    for s in &mut scenes {
        for l in &mut s.lines {
            l.typed = None;
        }
    }
    format!("{}\n{:?}", serde_json::to_string(&scenes).unwrap(), w.folk.staff.interviewed)
}

/// A typed line read as `code`. With `writer` none the writer's call fails
/// and the bank's line stays.
fn typed(g: &mut Game, f: &Fake, id: u32, code: u32, writer: Option<&str>) -> Value {
    f.push("scene_reader", Ok(reading(&tag(g, id, code))));
    match writer {
        Some(t) => f.push_text("scene_writer", t),
        None => f.push("scene_writer", Err(afm_ai::Failure::Timeout)),
    }
    let t = ok(g, "people_say", json!({ "id": id, "text": "Say a kid won't listen. What then?" }))["ticket"]
        .as_u64()
        .unwrap();
    finish(g, t)
}

#[test]
fn a_typed_line_replays_the_same_from_the_same_save() {
    let (mut g, f) = game("replay", true);
    let id = interview(&mut g);
    let saved = ok(&mut g, "save", json!({ "name": "before" }))["saved"].as_str().unwrap().to_string();
    let done = typed(&mut g, &f, id, 61, None);
    assert_eq!(done["state"], "done", "{done}");
    let first = snapshot(&g);
    // The same save and the same reading: the same world.
    ok(&mut g, "load", json!({ "path": saved }));
    let done = typed(&mut g, &f, id, 61, None);
    assert_eq!(done["state"], "done", "{done}");
    assert_eq!(snapshot(&g), first, "the same reading on the same save");
    // The clicked chip from the same save: the same world.
    ok(&mut g, "load", json!({ "path": saved }));
    ok(&mut g, "people_answer", json!({ "id": id, "code": 61 }));
    assert_eq!(snapshot(&g), first, "typed and clicked land the same");
    // The writer's words, when they pass, are what the save keeps.
    ok(&mut g, "load", json!({ "path": saved }));
    let words = "That's the job, Coach. I'd teach it the same way every day.";
    assert_eq!(typed(&mut g, &f, id, 61, Some(words))["state"], "done");
    ok(&mut g, "save", json!({ "name": "after" }));
    let s = g.world.as_ref().unwrap().folk.scenes.iter().find(|s| s.id == id).unwrap();
    assert_eq!(s.lines.last().unwrap().text, words);
}

#[test]
fn a_save_made_with_live_talks_plays_without_them() {
    let (mut g, f) = game("nokey", true);
    let id = interview(&mut g);
    assert_eq!(typed(&mut g, &f, id, 60, Some("I'd use them the way they already play."))["state"], "done");
    let saved = ok(&mut g, "save", json!({ "name": "typed" }))["saved"].as_str().unwrap().to_string();
    let (mut h, _) = game("nokey2", false);
    ok(&mut h, "load", json!({ "path": saved }));
    let v = ok(&mut h, "people_scene", json!({ "id": id }));
    assert!(v.get("say").is_none(), "no key, no box");
    let mine = v["lines"].as_array().unwrap().iter().rev().find(|l| l["by"] == 1).unwrap().clone();
    assert_eq!(mine["typed"], "Say a kid won't listen. What then?", "your words came back with the save");
    let next = v["picks"][0]["code"].as_u64().unwrap();
    assert!(ok(&mut h, "people_answer", json!({ "id": id, "code": next }))["lines"].is_array());
}
