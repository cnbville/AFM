//! A talk that ends a meeting (thread "Let talking end meetings cleanly"):
//! a goodbye, typed or the Wrap it up button, ends a boss or staff meeting
//! or one of the older meetings with no cost for leaving; what you didn't
//! argue stands; End always closes the talk, and a decision it leaves open
//! moves to its own screen with a line in your inbox (talk replies item 12).

use afm_engine::world::folk::persona::talk::store;
use afm_engine::world::folk::persona::{makeup, meetings};
use afm_engine::world::folk::scene::{self, Kind, Scene};
use afm_engine::world::folk::{Team, boss};
use serde_json::json;

use super::super::{ear::EAR, voice::VOICE};
use super::tests::{act, ear, finish, game, ok, scene_of, trust, voice};
use crate::Game;

fn meeting(g: &mut Game, kind: Kind, him: u32) -> u32 {
    let w = g.world.as_mut().unwrap();
    let mut s = Scene::new(kind, w.today, "A meeting");
    s.a = him as u64;
    s.stops = false;
    s.place = if kind == Kind::ParentCall { "On the phone".into() } else { "Your office".into() };
    let sid = scene::add(w, s);
    ok(g, "people_scene", json!({ "id": sid }));
    sid
}

fn boss_meeting(g: &mut Game) -> u32 {
    let w = g.world.as_mut().unwrap();
    let mut s = Scene::new(Kind::BossFirst, w.today, "Meeting the athletic director");
    s.team = Team::seat(w);
    s.stops = false;
    s.place = "His office".into();
    let sid = scene::add(w, s);
    ok(g, "people_scene", json!({ "id": sid }));
    sid
}

fn goodbye(g: &mut Game, f: &afm_ai::Fake, id: u32, words: &str, reply: &str) -> serde_json::Value {
    f.push(EAR, Ok(ear(vec![act("end", "", "none", "")], vec![], false)));
    f.push(VOICE, Ok(voice(reply, "")));
    let t = ok(g, "people_say", json!({ "id": id, "text": words }))["ticket"].as_u64().unwrap();
    let v = finish(g, t);
    // The line counts as said, and the page asks before it wraps up.
    assert!(!scene_of(g, id).done, "the goodbye alone doesn't close the talk");
    assert_eq!(ok(g, "people_scene", json!({ "id": id }))["talk"]["wrap_ask"], true);
    ok(g, "talk_wrap", json!({ "id": id }));
    v
}

#[test]
fn continue_keeps_the_talk_open_and_the_line_counts() {
    let (mut g, f, him) = game("wrap-continue", true);
    let f = f.unwrap();
    let id = meeting(&mut g, Kind::ParentCall, him);
    f.push(EAR, Ok(ear(vec![act("end", "", "none", "")], vec![], false)));
    f.push(VOICE, Ok(voice("Alright, Coach.", "")));
    let t =
        ok(&mut g, "people_say", json!({ "id": id, "text": "Okay, that's all for now." }))["ticket"].as_u64().unwrap();
    assert_eq!(finish(&mut g, t)["state"], "done");
    let v = ok(&mut g, "people_scene", json!({ "id": id }));
    assert_eq!(v["talk"]["wrap_ask"], true, "{}", v["talk"]);
    assert!(scene_of(&g, id).lines.iter().any(|l| l.by == 1), "the line still counts as said");
    // Continue: the ask goes away and the talk is open as if it hadn't ended.
    let v = ok(&mut g, "talk_wrap", json!({ "id": id, "keep": true }));
    assert!(v["talk"].get("wrap_ask").is_none(), "{}", v["talk"]);
    assert!(!scene_of(&g, id).done);
    f.push(EAR, Ok(ear(vec![act("question", "", "Mood", "")], vec![], false)));
    f.push(VOICE, Ok(voice("Still worried, Coach.", "")));
    let t = ok(&mut g, "people_say", json!({ "id": id, "text": "How is he at home?" }))["ticket"].as_u64().unwrap();
    assert_eq!(finish(&mut g, t)["state"], "done");
    assert!(!scene_of(&g, id).done);
}

#[test]
fn saying_end_the_talk_is_no_question() {
    let (mut g, f, him) = game("wrap-explicit", true);
    let f = f.unwrap();
    let id = meeting(&mut g, Kind::ParentCall, him);
    f.push(EAR, Ok(ear(vec![act("end", "", "none", "")], vec![], false)));
    f.push(VOICE, Ok(voice("Alright.", "")));
    let t = ok(&mut g, "people_say", json!({ "id": id, "text": "End the talk." }))["ticket"].as_u64().unwrap();
    assert_eq!(finish(&mut g, t)["state"], "done");
    assert!(scene_of(&g, id).done, "said outright, it ends");
}

#[test]
fn a_goodnight_ends_the_boss_meeting_with_no_cost_and_the_goals_stand() {
    let (mut g, f, _) = game("wrap-boss", true);
    let f = f.unwrap();
    let id = boss_meeting(&mut g);
    let team = Team::seat(g.world.as_ref().unwrap()).unwrap();
    let w = g.world.as_ref().unwrap();
    let goals = w.folk.goals.clone();
    assert!(!goals.is_empty() && !scene_of(&g, id).picks.is_empty(), "the goals are set, the chips are up");
    let t0 = boss::trust(w, team);
    let v = goodbye(&mut g, &f, id, "Goodnight Stephen", "Goodnight. Drive safe.");
    assert_eq!(v["state"], "done", "{v}");
    let s = scene_of(&g, id);
    assert!(s.done && s.handed.is_none() && s.picks.is_empty(), "{s:?}");
    assert!(s.outcome[0].starts_with("You wrapped it up"), "{:?}", s.outcome);
    let w = g.world.as_ref().unwrap();
    assert!((boss::trust(w, team) - t0).abs() < 1e-6, "leaving costs nothing");
    assert_eq!(format!("{:?}", w.folk.goals), format!("{goals:?}"), "the goals stand as set");
    assert!(store::open(w, id).is_none(), "the talk is closed");
}

#[test]
fn wrap_it_up_ends_a_parents_call_without_typing() {
    let (mut g, f, him) = game("wrap-button", true);
    let f = f.unwrap();
    let id = meeting(&mut g, Kind::ParentCall, him);
    let parent = meetings::parent(g.world.as_ref().unwrap(), him);
    f.push(EAR, Ok(ear(vec![act("question", "", "Mood", "")], vec![], false)));
    f.push(VOICE, Ok(voice("He's fine, Coach. Just worried.", "")));
    let t =
        ok(&mut g, "people_say", json!({ "id": id, "text": "How's he doing at home?" }))["ticket"].as_u64().unwrap();
    assert_eq!(finish(&mut g, t)["state"], "done");
    let p0 = makeup::trust(g.world.as_ref().unwrap(), parent);
    let s0 = trust(&g, him);
    let v = ok(&mut g, "talk_wrap", json!({ "id": id }));
    assert_eq!(v["done"], true, "{v}");
    assert!(scene_of(&g, id).handed.is_none());
    let w = g.world.as_ref().unwrap();
    assert_eq!(makeup::trust(w, parent), p0, "the parent's trust doesn't move for a goodbye");
    assert_eq!(trust(&g, him), s0, "nor his son's");
}

/// Every line of your inbox from today with this rule.
fn inbox(g: &Game, rule: &str) -> Vec<String> {
    let w = g.world.as_ref().unwrap();
    let today = w.today;
    w.calendar
        .events
        .range(today..today.add(1))
        .flat_map(|(_, evs)| evs.iter())
        .filter(|e| e.rule == rule)
        .map(|e| e.text.clone())
        .collect()
}

#[test]
fn end_closes_a_meeting_that_holds_a_decision_and_the_decision_moves_to_its_screen() {
    // Talk replies item 12 (Cain 2026-11-19: "When i confirm 'end' on a
    // talk it should close the convo entirely").
    let (mut g, f, him) = game("wrap-decide", true);
    let f = f.unwrap();
    let id = meeting(&mut g, Kind::Discipline, him);
    assert!(!scene_of(&g, id).picks.is_empty());
    assert_eq!(goodbye(&mut g, &f, id, "We'll talk about this tomorrow. Go home.", "Yes, Coach.")["state"], "done");
    let s = scene_of(&g, id);
    assert!(s.done && s.picks.is_empty(), "the talk is over and closed: {s:?}");
    assert!(s.outcome[0].starts_with("You ended the talk"), "{:?}", s.outcome);
    assert!(s.outcome[0].contains("set it from your team rules"), "{:?}", s.outcome);
    assert!(store::open(g.world.as_ref().unwrap(), id).is_none(), "the talk is closed");
    let lines = inbox(&g, "people: decision");
    assert!(lines.iter().any(|l| l.starts_with("Still your call")), "{lines:?}");
    assert!(g.call("people_say", &json!({ "id": id, "text": "One more thing." })).is_err());
}

#[test]
fn ending_an_interview_leaves_him_waiting_on_the_staff_screen() {
    use afm_engine::world::coach::{Group, Role};
    use afm_engine::world::folk::{hiring, staffing};
    let (mut g, f, _) = game("wrap-interview", true);
    let f = f.unwrap();
    let (c, id) = {
        let w = g.world.as_mut().unwrap();
        let team = Team::seat(w).unwrap();
        let c = (0..w.coaches.len())
            .find(|&c| w.coaches[c].job.is_none_or(|j| Team::from_employer(j.employer) != team))
            .unwrap();
        (c, staffing::start_interview(w, c, Role::PositionCoach(Group::QB)).unwrap())
    };
    ok(&mut g, "people_scene", json!({ "id": id }));
    assert_eq!(goodbye(&mut g, &f, id, "Thanks for coming in. I'll be in touch.", "Sounds good.")["state"], "done");
    let s = scene_of(&g, id);
    assert!(s.done, "an interview the talk ended is closed");
    assert!(s.outcome[0].contains("staff screen"), "{:?}", s.outcome);
    let w = g.world.as_ref().unwrap();
    let t = hiring::talk(w, c).expect("a hiring talk waits for your offer");
    assert_eq!(t.waiting, Some(w.today), "his patience runs from today");
    assert!(inbox(&g, "people: decision").iter().any(|l| l.contains("staff screen")));
}
