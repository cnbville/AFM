//! A talk that ends a meeting (thread "Let talking end meetings cleanly"):
//! a goodbye, typed or the Wrap it up button, ends a boss or staff meeting
//! or one of the older meetings with no cost for leaving; what you didn't
//! argue stands; a meeting that holds a decision keeps it as its own picks,
//! with the talk over.

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
    finish(g, t)
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

#[test]
fn a_decision_the_talk_didnt_settle_stays_as_the_meetings_picks() {
    let (mut g, f, him) = game("wrap-decide", true);
    let f = f.unwrap();
    let id = meeting(&mut g, Kind::Discipline, him);
    let picks = scene_of(&g, id).picks.clone();
    assert!(!picks.is_empty());
    assert_eq!(goodbye(&mut g, &f, id, "We'll talk about this tomorrow. Go home.", "Yes, Coach.")["state"], "done");
    let s = scene_of(&g, id);
    assert!(!s.done && s.picks == picks, "the punishment is still yours to decide");
    assert!(store::is_over(g.world.as_ref().unwrap(), id), "the talk is over");
    // The page shows no box and the decision; a line is refused.
    let v = ok(&mut g, "people_scene", json!({ "id": id }));
    assert_eq!(v["talk"]["over"], true, "{}", v["talk"]);
    assert!(g.call("people_say", &json!({ "id": id, "text": "One more thing." })).is_err());
    // The meeting's own pick decides it.
    let code = picks[0].code;
    ok(&mut g, "people_answer", json!({ "id": id, "code": code }));
}
