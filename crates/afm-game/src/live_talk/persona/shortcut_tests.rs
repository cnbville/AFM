//! Talks as a shortcut (lane B of `docs/design/talk-replies.md`, agreed by
//! Cain 2026-11-19), on the fake: an action you say you'll do runs after one
//! confirm through the screen's own command, a "Not that" on your word
//! leaves a promise, an action the game can't do today says why, and the
//! "What he's after" read sharpens as he opens up.

use afm_engine::progression as pg;
use afm_engine::world::folk::talks::{self, PromiseKind, PromiseTo};
use serde_json::{Value, json};

use super::super::{ear::EAR, voice::VOICE};
use super::tests::{act, ear, finish, game, ok, talk, voice};
use crate::Game;

fn line(g: &mut Game, f: &afm_ai::Fake, id: u32, reading: Value, text: &str) -> Value {
    f.push(EAR, Ok(reading));
    f.push(VOICE, Ok(voice("Okay, Coach.", "")));
    let t = ok(g, "people_say", json!({ "id": id, "text": text }))["ticket"].as_u64().unwrap();
    let v = finish(g, t);
    assert_eq!(v["state"], "done", "{v}");
    v
}

fn last_voice(f: &afm_ai::Fake) -> String {
    f.asked().into_iter().rev().find(|r| r.label == VOICE).map(|r| r.user).unwrap_or_default()
}

/// "I'm putting you on a project": the ear's `do` act on him, with his
/// position's first project skill.
fn project_line(g: &mut Game, f: &afm_ai::Fake, id: u32, him: u32) -> Value {
    let skill = {
        let w = g.world.as_ref().unwrap();
        let p = &w.people[afm_engine::world::folk::person(w, him).unwrap()].player;
        pg::project_skills(p).first().expect("a project skill").label().to_lowercase()
    };
    let mut a = act("do", "skill_project", "none", "p1");
    a["value"] = json!(skill);
    a["words"] = json!("I'm putting you on a project this winter.");
    line(g, f, id, ear(vec![a], vec![], false), "I'm putting you on a project this winter.")
}

fn the_card(v: &Value) -> Value {
    let c = v["confirm"].as_array().and_then(|c| c.first()).cloned().unwrap_or(Value::Null);
    assert_eq!(c["cmd"], "talk_confirm", "one confirm card: {v}");
    c
}

fn narrated(r: &Value, starts: &str) -> bool {
    r.to_string().contains(starts)
}

#[test]
fn a_project_you_say_youll_start_runs_after_one_confirm() {
    let (mut g, f, him) = game("shortcut-project", true);
    let f = f.unwrap();
    let id = talk(&mut g, him, "role");
    let v = project_line(&mut g, &f, id, him);
    let c = the_card(&v);
    assert!(c.to_string().contains("project for"), "{c}");
    assert!(last_voice(&f).contains("The coach is doing it"), "he hears it's coming");
    assert!(!talks::work_kept(g.world.as_ref().unwrap(), him, 0), "nothing runs before the confirm");
    let r = ok(&mut g, "talk_confirm", c["args"].clone());
    assert!(narrated(&r, "Done: start a"), "{r}");
    assert!(talks::work_kept(g.world.as_ref().unwrap(), him, 0), "the screen's own command started it");
}

#[test]
fn not_that_on_your_word_leaves_a_promise_he_holds_you_to() {
    let (mut g, f, him) = game("shortcut-not-that", true);
    let f = f.unwrap();
    let id = talk(&mut g, him, "role");
    let c = the_card(&project_line(&mut g, &f, id, him));
    let mut args = c["args"].clone();
    args["yes"] = json!(false);
    let r = ok(&mut g, "talk_confirm", args);
    assert!(narrated(&r, "Your word to him stands"), "{r}");
    let w = g.world.as_ref().unwrap();
    assert!(!talks::work_kept(w, him, 0));
    assert!(
        w.folk.promises.iter().any(|p| p.to == PromiseTo::Player(him) && p.kind == PromiseKind::Work(0)),
        "{:?}",
        w.folk.promises
    );
}

#[test]
fn an_action_the_game_cant_do_today_says_why() {
    // He's healthy, so there's no reserve list to bring him back from.
    let (mut g, f, him) = game("shortcut-cant", true);
    let f = f.unwrap();
    let id = talk(&mut g, him, "role");
    let mut a = act("do", "reserve_back", "none", "p1");
    a["words"] = json!("I'm bringing you back off the reserve list.");
    let v = line(&mut g, &f, id, ear(vec![a], vec![], false), "I'm bringing you back off the reserve list.");
    assert!(v.get("confirm").is_none(), "nothing to confirm: {v}");
    assert!(last_voice(&f).contains("The game can't do that today"), "{}", last_voice(&f));
}

fn after_read(g: &mut Game, id: u32) -> Vec<String> {
    let v = ok(g, "people_scene", json!({ "id": id }));
    let left = v["talk"]["left"].as_array().cloned().unwrap_or_default();
    let s = left.iter().find(|x| x["label"] == "What he's after").unwrap_or_else(|| panic!("{v}"));
    s["paras"].as_array().unwrap().iter().map(|p| p.as_str().unwrap().to_string()).collect()
}

#[test]
fn what_hes_after_sharpens_as_he_opens_up() {
    let (mut g, f, him) = game("shortcut-after", true);
    let f = f.unwrap();
    let id = talk(&mut g, him, "role");
    let first = after_read(&mut g, id);
    assert!(!first[0].starts_with("A good read"), "{first:?}");
    assert!(first.iter().all(|l| !l.chars().any(|c| c.is_ascii_digit())), "never a number: {first:?}");
    // He opens up: you ask what he wants and keep talking.
    line(&mut g, &f, id, ear(vec![act("question", "", "Wants", "p1")], vec![], false), "What do you want here?");
    for _ in 0..3 {
        line(&mut g, &f, id, ear(vec![act("question", "", "Mood", "")], vec![], false), "Tell me more.");
    }
    let later = after_read(&mut g, id);
    let rank = |s: &str| ["An early read", "A fair read", "A good read"].iter().position(|x| s.starts_with(x)).unwrap();
    assert!(rank(&later[0]) > rank(&first[0]), "{first:?} then {later:?}");
}
