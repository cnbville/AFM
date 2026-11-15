//! One strong line against generic praise (agreed 2026-11-15): a line that
//! stakes you on him ("you're my guy, you're going to break records for
//! us") weighs a lot when he believes you and little when he doesn't;
//! generic praise counts a little each and stops adding up after three or
//! four lines; a stake said again right away doesn't double it.

use afm_engine::world::folk::persona::Them;
use afm_engine::world::folk::persona::ear::{Hit, How, Item};
use afm_engine::world::folk::persona::talk::{Parts, channels, hits, store};
use afm_engine::world::folk::talks::{Promise, PromiseKind, PromiseTo};
use afm_engine::player::PlayingTime;
use serde_json::{Value, json};

use super::super::{ear::EAR, voice::VOICE};
use super::tests::{ear, game, item_tag, say, scene_of, talk, trust, voice};
use crate::Game;

const STAKE: &str = "Man, I'm not going to fuck around here. You're my guy, you're going to break records for us.";

/// His trust in you and the job he sees for himself.
fn set_up(g: &mut Game, him: u32, t: f32, role: PlayingTime) {
    let w = g.world.as_mut().unwrap();
    let i = afm_engine::world::folk::person(w, him).unwrap();
    w.people[i].trust = t;
    w.people[i].last_role = Some(role);
}

/// What a line did to him: trust now plus the mood the talk holds for him.
fn now(g: &Game, him: u32, id: u32) -> f32 {
    trust(g, him) + scene_of(g, id).f[1]
}

/// One typed line with one hit on his pride, and what it moved.
fn line(g: &mut Game, f: &afm_ai::Fake, him: u32, id: u32, how: &str, words: &str) -> f32 {
    let pride = item_tag(g, id, "his pride");
    f.push(EAR, Ok(ear(vec![], vec![json!({ "on": "", "item": pride, "how": how })], false)));
    f.push(VOICE, Ok(voice("Appreciate that, Coach.", "")));
    let before = now(g, him, id);
    let v: Value = say(g, id, words);
    assert_eq!(v["state"], "done", "{v}");
    now(g, him, id) - before
}

#[test]
fn one_stake_he_believes_outweighs_four_lines_of_praise_and_praise_stops_adding_up() {
    // A starter who trusts you.
    let (mut g, f, him) = game("stake-one", true);
    let f = f.unwrap();
    set_up(&mut g, him, 0.85, PlayingTime::Starter);
    let id = talk(&mut g, him, "role");
    let stake = line(&mut g, &f, him, id, "staked", STAKE);

    let (mut g, f, him) = game("stake-praise", true);
    let f = f.unwrap();
    set_up(&mut g, him, 0.85, PlayingTime::Starter);
    let id = talk(&mut g, him, "role");
    let mut praise = Vec::new();
    for (k, how) in ["flattered", "respected", "reassured", "flattered", "respected", "reassured"].iter().enumerate() {
        praise.push(line(&mut g, &f, him, id, how, &format!("You're a good addition to our room. ({k})")));
    }
    let four: f32 = praise[..4].iter().sum();
    eprintln!("a stake he believes: {stake}; six lines of praise: {praise:?}, the first four {four}");
    assert!(stake > 0.12, "a stake he believes is a big lift: {stake}");
    assert!(stake > 2.0 * four, "one stake outweighs four lines of praise: {stake} vs {four}");
    // Flattery can cost a professional a little trust (a pro sees through it), so it's the lines together
    // that count a little.
    assert!(four > 0.0, "praise still counts a little: {praise:?}");
    // From the fifth line of it on, nothing.
    assert!(praise[4].abs() < 1e-4 && praise[5].abs() < 1e-4, "{praise:?}");
}

#[test]
fn a_stake_said_again_counts_nothing_in_the_talk_and_little_soon_after() {
    let (mut g, f, him) = game("stake-again", true);
    let f = f.unwrap();
    set_up(&mut g, him, 0.85, PlayingTime::Starter);
    let id = talk(&mut g, him, "role");
    // What a stake would do in a fresh talk, before any.
    let size = |g: &Game| {
        let w = g.world.as_ref().unwrap();
        let s = scene_of(g, id);
        let open = store::Open::new(id, &channels::one_on_one(&s));
        let wants = Parts::today().wanting.wants(w, Them::Player(him));
        let h = Hit { on: Them::Player(him), item: Item::Pride, how: How::Staked };
        hits::size(w, &h, &wants, &open, 0).unwrap()
    };
    let fresh = size(&g);
    let first = line(&mut g, &f, him, id, "staked", STAKE);
    let again = line(&mut g, &f, him, id, "staked", "I mean it. You're my guy.");
    let later = size(&g);
    eprintln!("stake: fresh {fresh:?}, first {first}, again in the talk {again}, a fresh talk today {later:?}");
    assert!(first > 0.1, "{first}");
    assert!(again.abs() < 1e-4, "said again in the talk it doesn't double: {again}");
    let k = (later.trust + later.mood) / (fresh.trust + fresh.mood);
    assert!(
        (k - hits::STAKE_AGAIN).abs() < 0.05,
        "made again in a talk the same day it counts {} of a fresh one: {k}",
        hits::STAKE_AGAIN
    );
}

#[test]
fn a_man_who_doesnt_trust_you_wont_buy_the_big_line() {
    let (mut g, f, him) = game("stake-doubt", true);
    let f = f.unwrap();
    set_up(&mut g, him, 0.15, PlayingTime::Backup);
    {
        // You broke your word to him this season.
        let w = g.world.as_mut().unwrap();
        let today = w.today;
        let mut p = Promise::blank(PromiseTo::Player(him), PromiseKind::Start);
        p.made = today.add(-60);
        p.due = today.add(-10);
        p.kept = Some(false);
        w.folk.promises.push(p);
    }
    let belief = hits::stake_belief(g.world.as_ref().unwrap(), him).unwrap();
    let id = talk(&mut g, him, "role");
    let before = trust(&g, him);
    let moved = line(&mut g, &f, him, id, "staked", STAKE);
    eprintln!("a stake to a man who doesn't trust you: belief {belief}, moved {moved}");
    assert!(belief < 0.4, "he doubts it: {belief}");
    assert!(trust(&g, him) < before, "he's heard that before: trust {before} -> {}", trust(&g, him));
    assert!(moved < 0.0, "{moved}");
}
