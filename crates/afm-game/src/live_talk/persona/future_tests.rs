//! Your word on his future and changes to your promises in a typed talk
//! (the moves list's answers, restored to live talks v3): the ear hears
//! them as `future` and `amend` acts, the game settles them on the picks'
//! own outcome and odds.

use afm_engine::world::folk::future::{self, Word};
use afm_engine::world::folk::talks::{self, PromiseKind, PromiseTo};
use serde_json::{Value, json};

use super::super::{ear::EAR, voice::VOICE};
use super::tests::{act, ear, game, item_tag, say, scene_of, talk, voice};

fn with(kind: &str, what: &str, value: &str) -> Value {
    let mut a = act(kind, what, "none", "");
    a["value"] = json!(value);
    a
}

/// He leans toward the portal, so the point is on his sheet.
fn leaning(g: &mut crate::Game, him: u32) -> usize {
    let w = g.world.as_mut().unwrap();
    let i = afm_engine::world::folk::person(w, him).unwrap();
    w.people[i].cal.lean = 0.5;
    i
}

#[test]
fn telling_him_he_has_no_future_settles_it_and_ends_the_talk() {
    let (mut g, f, him) = game("nofuture", true);
    let f = f.unwrap();
    let i = leaning(&mut g, him);
    let id = talk(&mut g, him, "role");
    let stays = future::stays(g.world.as_ref().unwrap(), i, Word::NoFuture);
    f.push(EAR, Ok(ear(vec![with("future", "no_future", "")], vec![], false)));
    f.push(VOICE, Ok(voice("Then I'm gone.", "")));
    let v = say(&mut g, id, "I'll be honest with you. You don't have a future here.");
    assert_eq!(v["state"], "done", "{v}");
    let w = g.world.as_ref().unwrap();
    assert_eq!(future::decided(&w.people[i]), Some(!stays), "the picks' draw decides it");
    let s = scene_of(&g, id);
    assert!(s.done, "the talk ends there, as the pick's did");
    let want = if stays { "stays to fight" } else { "transfer portal" };
    assert!(s.outcome.iter().any(|x| x.contains(want)), "{:?}", s.outcome);
}

#[test]
fn the_words_need_his_point_and_you_as_his_head_coach() {
    let (mut g, f, him) = game("nopoint", true);
    let f = f.unwrap();
    let id = talk(&mut g, him, "role");
    f.push(EAR, Ok(ear(vec![with("future", "elsewhere", "")], vec![], false)));
    f.push(VOICE, Ok(voice("What?", "")));
    assert_eq!(say(&mut g, id, "You'd play more somewhere else.")["state"], "done");
    let w = g.world.as_ref().unwrap();
    let i = afm_engine::world::folk::person(w, him).unwrap();
    assert!(future::decided(&w.people[i]).is_none(), "no portal, lost job or buried point: nothing settled");
    assert!(!scene_of(&g, id).done);
}

#[test]
fn a_strength_you_name_backs_the_promise_you_make() {
    let (mut g, f, him) = game("backed", true);
    let f = f.unwrap();
    let i = leaning(&mut g, him);
    let id = talk(&mut g, him, "role");
    let (_, real) = future::named(g.world.as_ref().unwrap(), i, "speed").unwrap();
    let mut p = act("promise", "start", "none", "");
    p["when"] = json!("next_season");
    f.push(EAR, Ok(ear(vec![p, with("future", "backed", "speed")], vec![], false)));
    f.push(VOICE, Ok(voice("You noticed? Alright.", "")));
    let before = g.world.as_ref().unwrap().folk.promises.len();
    let v = say(&mut g, id, "The last staff didn't use you right. Your speed is why you'll start for me next year.");
    assert_eq!(v["state"], "done", "{v}");
    let w = g.world.as_ref().unwrap();
    assert_eq!(w.folk.promises.len(), before + 1, "one promise, backed");
    let p = w.folk.promises.last().unwrap();
    assert_eq!(p.to, PromiseTo::Player(him));
    assert_eq!(p.backed, if real { 1 } else { -1 });
    assert!(!scene_of(&g, id).done, "a promise doesn't end the talk");
}

#[test]
fn taking_back_a_promise_ends_it() {
    let (mut g, f, him) = game("takeback", true);
    let f = f.unwrap();
    talks::promise(g.world.as_mut().unwrap(), PromiseTo::Player(him), PromiseKind::Start, 0);
    let id = talk(&mut g, him, "role");
    let tag = item_tag(&g, id, "your promise to him");
    f.push(
        EAR,
        Ok(ear(vec![with("amend", "take_back", "")], vec![json!({"on": "", "item": tag, "how": "stung"})], false)),
    );
    f.push(VOICE, Ok(voice("Right. I figured.", "")));
    assert_eq!(say(&mut g, id, "Forget what I said about starting you.")["state"], "done");
    let w = g.world.as_ref().unwrap();
    assert!(!w.folk.promises.iter().any(|p| p.to == PromiseTo::Player(him) && p.kept.is_none()));
    assert!(scene_of(&g, id).outcome.iter().any(|x| x.contains("took back")), "{:?}", scene_of(&g, id).outcome);
}

#[test]
fn a_later_date_moves_the_promise() {
    let (mut g, f, him) = game("later", true);
    let f = f.unwrap();
    talks::promise(g.world.as_mut().unwrap(), PromiseTo::Player(him), PromiseKind::Start, 0);
    let id = talk(&mut g, him, "role");
    let mut a = with("amend", "later", "");
    a["when"] = json!("next_season");
    f.push(EAR, Ok(ear(vec![a], vec![], false)));
    f.push(VOICE, Ok(voice("Next year, then.", "")));
    assert_eq!(say(&mut g, id, "Let's make that next season instead.")["state"], "done");
    let w = g.world.as_ref().unwrap();
    let p = w.folk.promises.iter().find(|p| p.to == PromiseTo::Player(him) && p.kept.is_none()).unwrap();
    assert_eq!(p.terms.by, afm_engine::world::folk::promise::By::NextSeason);
}
