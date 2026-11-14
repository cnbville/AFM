//! Live talks lane 5 (LT6), on a real small world: the pregame and halftime
//! room typed. A typed line is the chip's Say it; the pregame talk can be
//! changed until the team is on the field; the halftime talk happens once and
//! a typed line never lands without the ask-first step; and a save after a
//! typed halftime talk plays the same game as the chip's. It builds a world
//! and plays a game, so it runs in Full tests only.

mod common;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use afm_ai::Fake;
use afm_ai::fake::body;
use afm_game::Game;
use serde_json::{Value, json};

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

/// A day at a time to the morning of your next game.
fn to_game_morning(g: &mut Game) {
    for _ in 0..400 {
        let r = ok(g, "day", json!({}));
        if r["morning"] == json!(true) && r["why"][0]["text"].as_str().is_some_and(|t| t.starts_with("Game day")) {
            return;
        }
    }
    panic!("never reached a game morning");
}

fn to_kickoff(g: &mut Game) {
    for _ in 0..8 {
        if ok(g, "gameday_now", json!({}))["at_kick"] == json!(true) {
            return;
        }
        ok(g, "continue", json!({}));
    }
    panic!("never reached the kickoff");
}

/// A reader's answer as the API sends it.
fn reading(mv: &str, tone: &str, subject: &str, who: &str, fit: &str, also: &str) -> Value {
    let v = json!({ "move": mv, "also": also, "tone": tone, "subject": subject, "who": who, "fit": fit });
    body(&v.to_string(), 700, 60)
}

fn typed(g: &mut Game, f: &Fake, moment: &str, r: Value) -> Value {
    f.push("room_reader", Ok(r));
    let t = ok(g, "talk_typed", json!({ "moment": moment, "text": "Listen up. Here is where we are." }))["ticket"]
        .as_u64()
        .unwrap();
    finish(g, t)
}

/// The staff's calls to the final whistle.
fn play_out(g: &mut Game) -> Value {
    let mut st = ok(g, "live", json!({}));
    for _ in 0..2000 {
        if st["status"] != "live" {
            return st;
        }
        st = ok(g, "live_answer", json!({ "kind": "staff" }));
    }
    panic!("the game never ends");
}

/// The kept pregame talk, as the room reports it.
fn said(g: &Game) -> Value {
    let w = g.world.as_ref().unwrap();
    serde_json::to_value(w.gameday.pregame).unwrap()
}

fn path(v: &Value) -> String {
    v["saved"].as_str().unwrap().to_string()
}

#[test]
fn a_typed_talk_in_the_room_is_the_chips_say_it() {
    let t = Instant::now();
    let dir: PathBuf = std::env::temp_dir().join(format!("afm-live-rooms-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut g = Game::new(dir.clone());
    ok(&mut g, "build_world", json!({ "seed": 21, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    let job = ok(&mut g, "jobs", json!({ "level": "college" }))["jobs"][0]["ref"].clone();
    ok(
        &mut g,
        "start_career",
        json!({ "name": "Test Coach", "age": 44, "background": "college_assistant", "job": job, "seed": 3, "calls": "both" }),
    );
    let f = Arc::new(Fake::new());
    g.ai.use_fake(f.clone());
    to_game_morning(&mut g);
    println!("at the game morning in {:.0}s", t.elapsed().as_secs_f64());
    let morning = path(&ok(&mut g, "save", json!({ "name": "morning" })));

    // The room carries the box while you still have a talk to give.
    let room = ok(&mut g, "talk_room", json!({}));
    assert_eq!(room["status"], "choose", "{room}");
    assert_eq!(room["say"]["cmd"], "talk_typed");
    assert_eq!(room["say"]["args"]["moment"], "pregame");
    assert_eq!(room["say"]["limit"], 400);

    // A line the reader can't use changes nothing.
    let v = typed(&mut g, &f, "pregame", reading("none", "calm", "us", "none", "not_a_line", "none"));
    assert_eq!(v["state"], "none", "{v}");
    assert_eq!(said(&g), Value::Null, "nothing was kept");

    // A reading that is unsure shows both and keeps nothing until you pick.
    let v = typed(&mut g, &f, "pregame", reading("challenge", "fiery", "them", "none", "unsure", "warn"));
    assert_eq!(v["state"], "unsure", "{v}");
    let rs = v["readings"].as_array().unwrap();
    assert_eq!(rs.len(), 2);
    assert!(rs.iter().all(|r| r["final"] == false && r["heard"].is_string()));
    assert_eq!(said(&g), Value::Null, "still nothing kept");
    let second = rs[1]["pick"].clone();
    let v = ok(&mut g, "talk_typed", json!({ "moment": "pregame", "ticket": 1, "pick": second }));
    assert_eq!(v["state"], "done", "{v}");
    assert_eq!(v["reply"]["status"], "said");
    assert_eq!(v["reply"]["pick"]["move"], "warn");
    assert!(v["reply"].get("say").is_none(), "no box on a talk you have said");

    // Typed equals clicked: the same kept talk and the same room after.
    let v = typed(&mut g, &f, "pregame", reading("challenge", "fiery", "them", "none", "clear", "none"));
    assert_eq!(v["state"], "done", "{v}");
    assert_eq!(v["heard"], "challenge about them, fiery");
    let (kept, after) = (said(&g), ok(&mut g, "talk_room", json!({})));
    ok(&mut g, "load", json!({ "path": morning }));
    assert_eq!(said(&g), Value::Null);
    ok(&mut g, "talk_say", json!({ "move": "challenge", "tone": "fiery", "subject": "them", "who": 0 }));
    assert_eq!(said(&g), kept, "typed and clicked keep the same talk");
    // The words the room draws (your line, your coordinators') come from the
    // bank and depend on what the screen already showed (minor bug 314): the
    // same talk, the numbers and the picks, not those.
    let mut clicked = ok(&mut g, "talk_room", json!({}));
    let mut typed_room = after;
    for r in [&mut clicked, &mut typed_room] {
        for k in ["say", "line", "staff"] {
            r.as_object_mut().unwrap().remove(k);
        }
    }
    assert_eq!(clicked, typed_room, "the same room after");

    // A pregame talk can be changed until the team is on the field.
    let v = typed(&mut g, &f, "pregame", reading("praise", "warm", "us", "none", "clear", "none"));
    assert_eq!(v["state"], "done", "a talk said can be said another way: {v}");
    assert_eq!(said(&g)[1]["mv"], "Praise");

    // A man named is subject one with his id, and only a man in the room.
    ok(&mut g, "talk_say", json!({ "undo": true }));
    let room = ok(&mut g, "talk_room", json!({}));
    let cap = room["captains"]["voices"].as_array().unwrap().iter().find(|v| v["you"] != true).unwrap()["id"].clone();
    let v = typed(&mut g, &f, "pregame", reading("praise", "warm", "one", "p1", "clear", "none"));
    assert_eq!(v["state"], "done", "{v}");
    let pick = &said(&g)[1];
    assert_eq!((pick["subject"].as_str(), &pick["who"]), (Some("One"), &cap), "{pick}");
    assert!(v["heard"].as_str().unwrap().starts_with("praise to "), "{v}");
    ok(&mut g, "talk_say", json!({ "undo": true }));
    let v = typed(&mut g, &f, "pregame", reading("praise", "warm", "one", "p99", "clear", "none"));
    assert_eq!(v["state"], "done");
    assert_eq!(
        (said(&g)[1]["subject"].as_str(), said(&g)[1]["who"].as_u64()),
        (Some("Us"), Some(0)),
        "a man not in the room is no man"
    );
    // Handing it to a captain is the chip's: calm, to a captain.
    let v = typed(&mut g, &f, "pregame", reading("captain", "fiery", "stakes", "p1", "clear", "none"));
    assert_eq!(v["state"], "done");
    let pick = &said(&g)[1];
    assert_eq!((pick["mv"].as_str(), pick["tone"].as_str(), &pick["who"]), (Some("Captain"), Some("Calm"), &cap));
    // No key: no box, no typed line (the chips go on).
    std::fs::create_dir_all(dir.join("plain")).unwrap();
    let mut plain = Game::new(dir.join("plain"));
    ok(&mut plain, "load", json!({ "path": morning }));
    let room = ok(&mut plain, "talk_room", json!({}));
    assert!(room.get("say").is_none(), "no key, no box");
    assert!(plain.call("talk_typed", &json!({ "moment": "pregame", "text": "Go get them." })).is_err());
    assert_eq!(
        ok(&mut plain, "talk_say", json!({ "move": "praise", "tone": "warm", "subject": "us" }))["status"],
        "said"
    );

    // Kickoff: the pregame room is closed to typed lines.
    ok(&mut g, "talk_say", json!({ "undo": true }));
    to_kickoff(&mut g);
    let mut st = ok(&mut g, "gameday_kickoff", json!({}));
    let v = g.call("talk_typed", &json!({ "moment": "pregame", "text": "Too late." }));
    assert_eq!(v, Err("The team is already on the field.".into()));
    println!("kickoff in {:.0}s", t.elapsed().as_secs_f64());

    // Play to the half with the staff's calls.
    for n in 0..2000 {
        assert!(n < 1999 && st["status"] == "live", "the game ended before its half: {st}");
        if st["pending"]["ask"] == "halftime" {
            break;
        }
        st = ok(&mut g, "live_answer", json!({ "kind": "staff" }));
    }
    let half = path(&ok(&mut g, "save", json!({ "name": "half" })));
    let room = ok(&mut g, "talk_room", json!({}));
    assert_eq!((room["status"].as_str(), room["moment"].as_str()), (Some("choose"), Some("halftime")), "{room}");
    assert_eq!(room["say"]["args"]["moment"], "halftime");
    let live_before = ok(&mut g, "live", json!({}));

    // A typed halftime line always shows its reading first and keeps nothing.
    let v = typed(&mut g, &f, "halftime", reading("challenge", "fiery", "stakes", "none", "clear", "none"));
    assert_eq!(v["state"], "unsure", "{v}");
    let rs = v["readings"].as_array().unwrap();
    assert_eq!(rs.len(), 1);
    assert_eq!(rs[0]["final"], true);
    assert_eq!(rs[0]["heard"], "challenge about the stakes, fiery");
    assert_eq!(ok(&mut g, "live", json!({})), live_before, "reading it changed nothing");
    assert_eq!(ok(&mut g, "talk_room", json!({}))["status"], "choose");
    // A line that isn't one, or a reading with nothing on the table.
    let v = typed(&mut g, &f, "halftime", reading("none", "calm", "us", "none", "not_a_line", "none"));
    assert_eq!(v["state"], "none");
    // A pick that isn't one the room can make is refused whatever its ticket.
    assert!(g.call("talk_typed", &json!({ "moment": "halftime", "ticket": 1, "pick": 9 })).is_err());
    assert_eq!(ok(&mut g, "live", json!({})), live_before);

    // Confirmed, it lands as the chip's talk, once.
    let pick = rs[0]["pick"].clone();
    let v = ok(&mut g, "talk_typed", json!({ "moment": "halftime", "ticket": 1, "pick": pick }));
    assert_eq!(v["state"], "done", "{v}");
    assert_eq!(v["reply"]["status"], "said");
    assert_eq!(v["reply"]["pick"]["move"], "challenge");
    let typed_live = ok(&mut g, "live", json!({}));
    let again = g.call("talk_typed", &json!({ "moment": "halftime", "text": "One more thing." }));
    assert_eq!(again, Err("The talk is for halftime, once.".into()));
    let again = g.call("talk_typed", &json!({ "moment": "halftime", "ticket": 2, "pick": pick }));
    assert_eq!(again, Err("The talk is for halftime, once.".into()));
    assert!(ok(&mut g, "talk_room", json!({})).get("say").is_none(), "no box once it is said");
    assert!(g.call("live_answer", &json!({ "kind": "talk", "move": "praise" })).is_err());

    // The chip from the same save is the same game; a save after the typed
    // talk plays on the same.
    let after = path(&ok(&mut g, "save", json!({ "name": "half-typed" })));
    ok(&mut g, "load", json!({ "path": half }));
    ok(
        &mut g,
        "live_answer",
        json!({ "kind": "talk", "move": "challenge", "tone": "fiery", "subject": "stakes", "who": 0 }),
    );
    assert_eq!(ok(&mut g, "live", json!({})), typed_live, "typed and clicked, the same game at the half");
    let clicked = play_out(&mut g);
    ok(&mut g, "load", json!({ "path": after }));
    assert_eq!(ok(&mut g, "live", json!({})), typed_live, "a save after the typed talk loads as it was");
    let replay = play_out(&mut g);
    assert_eq!(replay["status"], "final");
    assert_eq!(replay["view"], clicked["view"], "the same game to the final whistle");
    println!("live rooms test in {:.0}s", t.elapsed().as_secs_f64());
    let _ = std::fs::remove_dir_all(dir);
}
