//! Talk replies that match the game (lane A of `docs/design/talk-replies.md`,
//! agreed by Cain 2026-11-19), on the fake: what the game releases and puts
//! on the cards for a promise, an answer to his counter, his number, a
//! promise you go back on, an interview's wants, his way of talking, the
//! whole talk and "Heard as".

use afm_engine::world::folk::persona::talk::store;
use afm_engine::world::folk::talks::{Promise, PromiseKind, PromiseTo};
use afm_engine::world::folk::{Team, hiring, staffing};
use serde_json::{Value, json};

use super::super::{ear::EAR, voice::VOICE};
use super::tests::{act, ear, finish, game, ok, scene_of, talk, voice};
use crate::Game;

fn last_voice(f: &afm_ai::Fake) -> String {
    f.asked().into_iter().rev().find(|r| r.label == VOICE).map(|r| r.user).unwrap_or_default()
}

fn line(g: &mut Game, f: &afm_ai::Fake, id: u32, reading: Value, said: &str, text: &str) -> Value {
    f.push(EAR, Ok(reading));
    f.push(VOICE, Ok(voice(said, "")));
    let t = ok(g, "people_say", json!({ "id": id, "text": text }))["ticket"].as_u64().unwrap();
    let v = finish(g, t);
    assert_eq!(v["state"], "done", "{v}");
    v
}

#[test]
fn a_promise_gets_his_take_and_stays_this_talks() {
    // Items 2, 5 and 11: he takes it by his belief in your word, and on the
    // next line it is "settled so far in this talk", not his history.
    let (mut g, f, him) = game("replies-promise", true);
    let f = f.unwrap();
    let id = talk(&mut g, him, "role");
    line(&mut g, &f, id, ear(vec![act("promise", "start", "none", "")], vec![], false), "We'll see.", "You'll start.");
    let card = last_voice(&f);
    assert!(
        card.contains("He takes your word on it")
            || card.contains("He takes it warily")
            || card.contains("He doubts it"),
        "{card}"
    );
    line(&mut g, &f, id, ear(vec![act("question", "", "Mood", "")], vec![], false), "Fine.", "How's school?");
    let card = last_voice(&f);
    assert!(card.contains("Settled so far in this talk") && card.contains("the coach promised you"), "{card}");
    assert!(!card.contains("made you a promise today"), "this talk's promise isn't history: {card}");
    // The whole talk is on the card, not the last six lines.
    let lines = scene_of(&g, id).lines.len();
    assert!(lines >= 4, "{lines}");
    assert!(card.contains("You'll start."), "{card}");
}

#[test]
fn a_claim_that_goes_back_on_your_word_breaks_the_promise_and_he_says_so() {
    // Item 6 (Justis: his place wouldn't change, then "you'll be the last").
    let (mut g, f, him) = game("replies-broken", true);
    let f = f.unwrap();
    let k = {
        let w = g.world.as_mut().unwrap();
        let today = w.today;
        let mut pr = Promise::blank(PromiseTo::Player(him), PromiseKind::Place(2));
        pr.made = today.add(-60);
        pr.due = today.add(300);
        pr.belief = 0.5;
        w.folk.promises.push(pr);
        w.folk.promises.len() - 1
    };
    let id = talk(&mut g, him, "role");
    let mut claim = act("claim", "", "none", "p1");
    claim["claim"] = json!("depth_rank");
    claim["value"] = json!("you'll be the last on the depth chart");
    line(
        &mut g,
        &f,
        id,
        ear(vec![claim], vec![], false),
        "That's not what you told me.",
        "You'll be the last on the depth chart.",
    );
    let w = g.world.as_ref().unwrap();
    assert_eq!(w.folk.promises[k].kept, Some(false), "broken on the spot");
    let card = last_voice(&f);
    assert!(card.contains("He reminds you that you promised him"), "{card}");
    // The claim itself is no new promise.
    assert_eq!(w.folk.promises.len(), k + 1, "{:?}", w.folk.promises.last());
}

/// An interview for an opening on your staff with a coach from elsewhere.
fn interview(g: &mut Game) -> (u32, usize, afm_engine::world::coach::Role) {
    let w = g.world.as_mut().unwrap();
    let team = Team::seat(w).unwrap();
    // The coordinator's chair open, as for Donte.
    if let Team::College(p) = team {
        w.programs[p].oc = None;
    }
    let role = *staffing::openings(w).first().expect("an opening");
    let c = (0..w.coaches.len())
        .find(|&c| w.coaches[c].job.is_some_and(|j| Team::from_employer(j.employer) != team))
        .unwrap();
    let id = staffing::start_interview(w, c, role).unwrap();
    ok(g, "people_scene", json!({ "id": id }));
    (id, c, role)
}

#[test]
fn his_counter_names_his_number_and_a_yes_makes_it_your_offer() {
    // Items 3 and 4 (Donte: "I'd need more money" with no figure; "Consider
    // it done" did nothing).
    let (mut g, f, _) = game("replies-number", true);
    let f = f.unwrap();
    let (id, c, role) = interview(&mut g);
    let ask = hiring::ask(g.world.as_ref().unwrap(), c, role);
    let mut offer = act("offer", "", "none", "");
    offer["deal"] = json!("Staff");
    offer["gives"] = json!([{ "give": "money", "number": (ask * 0.75).round() }]);
    line(&mut g, &f, id, ear(vec![offer], vec![], false), "That's short.", "How does that sound?");
    let card = last_voice(&f);
    assert!(card.contains("he names his number: $") && card.contains(" a year over "), "{card}");
    assert!(store::open(g.world.as_ref().unwrap(), id).and_then(|o| o.counter.clone()).is_some(), "on the table");
    // The ear's card shows it, and a yes to it is your offer on his terms.
    let v = line(&mut g, &f, id, ear(vec![act("answer", "accept", "none", "")], vec![], false), "Deal.", "Done.");
    let confirm = v["confirm"].as_array().cloned().unwrap_or_default();
    assert!(!confirm.is_empty(), "the deal waits on your confirm: {v}");
    let ear_card = f.asked().into_iter().rev().find(|r| r.label == EAR).map(|r| r.user).unwrap();
    assert!(ear_card.contains("his counter: "), "{ear_card}");
}

#[test]
fn asked_for_a_ballpark_he_gives_his_number_or_a_cagey_one() {
    let (mut g, f, _) = game("replies-ballpark", true);
    let f = f.unwrap();
    let (id, _, _) = interview(&mut g);
    let mut q = act("question", "", "Money", "");
    q["words"] = json!("give me a ballpark");
    line(&mut g, &f, id, ear(vec![q], vec![], false), "Well.", "Okay give me a ball-park");
    let card = last_voice(&f);
    assert!(
        card.contains("He names his own price") || card.contains("not under") || card.contains("somewhere between"),
        "{card}"
    );
}

#[test]
fn the_voice_card_carries_how_it_landed_and_his_way_of_talking() {
    // Items 1, 8 and 10.
    let (mut g, f, him) = game("replies-voice", true);
    let f = f.unwrap();
    let id = talk(&mut g, him, "role");
    let pride = super::tests::item_tag(&g, id, "his pride");
    let hits = vec![json!({ "on": "", "item": pride, "how": "insulted" })];
    let v = line(&mut g, &f, id, ear(vec![], hits, false), "Come on.", "You're soft.");
    let card = last_voice(&f);
    assert!(card.contains("How the coach's line landed with you") && card.contains("on your pride"), "{card}");
    assert!(card.contains("Your way of talking"), "{card}");
    let heard = v["heard"].as_str().unwrap_or_default();
    assert!(heard.split("; ").count() <= 4, "{heard}");
    // His own openers go on the next card.
    line(&mut g, &f, id, ear(vec![act("question", "", "Mood", "")], vec![], false), "Good.", "How are you?");
    assert!(last_voice(&f).contains("Your lines so far began"), "{}", last_voice(&f));
}

#[test]
fn the_same_man_talks_the_same_way_every_time() {
    use afm_engine::world::folk::persona::{Them, speech};
    let (g, _, him) = game("replies-habits", false);
    let w = g.world.as_ref().unwrap();
    let a = speech::habits(w, Them::Player(him), "Test Coach");
    assert_eq!(a, speech::habits(w, Them::Player(him), "Test Coach"));
    assert!(a.len() >= 3, "{a:?}");
}
