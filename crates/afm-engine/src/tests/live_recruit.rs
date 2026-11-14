//! Live talks lane 10, the college desk (LT2, LT3): a typed fact to a
//! recruit is checked where he would look. A true one is the chip; a false
//! one he can check as you say it is never saved and costs the lift, the
//! oversell and a little of your name on the trail; a NIL number he can
//! only check at signing is a lie, seen once he is yours and never if he
//! goes elsewhere, judged from a save with no key. A typed promise lands on
//! the promise it means, asked first, and leaves the case the chip leaves.

use super::legacy::seat;
use crate::world::folk::case::{self, OVERSELL};
use crate::world::folk::claims::check::recruit::{self as check, dollars, number};
use crate::world::folk::claims::{Stated, Truth};
use crate::world::folk::scene::{self, Heard, Kind as Scenes, Scene};
use crate::world::folk::talks::{self, PromiseTo, Reason};
use crate::world::folk::{Name, pitch, pledge, reach};
use crate::world::front_office::college as fc;
use crate::world::{Signed, Stage, World, recruiting};

const POINT: std::ops::Range<u32> = 7100..7300;
const PROMISE_MENU: u32 = 7520;
const PROMISE_START: u32 = 731;

/// Your program and `n` high school seniors of the class signing next,
/// each on your board with your offer out.
fn class(n: usize) -> (World, usize, Vec<usize>) {
    let mut w = seat("college");
    let mine = w.seat.and_then(|s| s.program).unwrap();
    w.add_class_aged(17);
    let next = recruiting::next_class(&w);
    let kids: Vec<usize> = (0..w.people.len()).filter(|&i| w.people[i].stage == Stage::HighSchool).take(n).collect();
    for &i in &kids {
        w.people[i].class_of = next;
        let id = w.people[i].player.id;
        fc::add(&mut w, id).unwrap();
        fc::offer(&mut w, id).unwrap();
    }
    (w, mine, kids)
}

/// The sit-down at the end of his official visit, opened.
fn sit_down(w: &mut World, i: usize, mine: usize) -> Scene {
    let id = w.people[i].player.id;
    let today = w.today;
    reach::official_talk(w, id, mine, today);
    let sid = w.folk.scenes.iter().rev().find(|s| s.kind == Scenes::OfficialTalk && s.a == id as u64).unwrap().id;
    scene::open(w, sid).unwrap()
}

fn bytes(w: &World) -> Vec<u8> {
    rmp_serde::to_vec(w).unwrap()
}

/// A copy of a world, as a save and a load make it.
fn copy(w: &World) -> World {
    rmp_serde::from_slice(&bytes(w)).unwrap()
}

fn untyped(w: &mut World) {
    for s in &mut w.folk.scenes {
        for l in &mut s.lines {
            l.typed = None;
        }
    }
}

fn typed(code: u32, facts: Vec<Stated>) -> Heard {
    let words = "Look at what we've done here and where you fit.";
    Heard { pick: code, tone: "warm".into(), words: words.into(), facts, ..Heard::default() }
}

fn fact(kind: &str, value: String) -> Stated {
    Stated { kind: kind.into(), about: None, value }
}

fn total(w: &World, i: usize, mine: usize) -> f32 {
    case::of(&w.people[i], mine).map_or(0.0, |c| c.total())
}

/// A sit-down with a point to make, in two copies of the same world.
fn two(n: usize) -> Option<(World, World, usize, usize, u32, u32)> {
    let (w, mine, kids) = class(n);
    for &i in &kids {
        let (mut a, mut b) = (copy(&w), copy(&w));
        let s = sit_down(&mut a, i, mine);
        let sb = sit_down(&mut b, i, mine);
        assert_eq!(s.id, sb.id);
        if s.done {
            continue;
        }
        if let Some(code) = s.picks.iter().chain(&s.reach).map(|p| p.code).find(|c| POINT.contains(c)) {
            return Some((a, b, i, mine, s.id, code));
        }
    }
    None
}

#[test]
fn numbers_and_money_are_read_from_the_words() {
    assert_eq!(dollars("$250,000"), Some(250_000.0));
    assert_eq!(dollars("250k a year"), Some(250_000.0));
    assert_eq!(dollars("1.2 million"), Some(1_200_000.0));
    assert_eq!(dollars("half a million"), Some(500_000.0));
    assert_eq!(dollars("$40K"), Some(40_000.0));
    assert_eq!(dollars("a lot of money"), None);
    assert_eq!(number("three first-round picks"), Some(3.0));
    assert_eq!(number("12 national titles"), Some(12.0));
    assert_eq!(number("no one coming back"), Some(0.0));
    assert_eq!(number("plenty of picks"), None);
}

#[test]
fn a_true_fact_is_the_chip_and_a_false_one_he_can_check_costs_on_the_spot() {
    let (mut a, mut b, i, mine, id, code) = two(30).expect("a sit-down with a point to make");
    let mut c = copy(&b);
    let titles = a.programs[mine].titles;
    let trail = b.folk.mine.name(Name::Trail);
    let before = total(&b, i, mine);
    // The chip, then the same point typed with a true fact: the same world.
    scene::answer(&mut a, id, code).unwrap();
    let s =
        scene::say(&mut b, id, typed(code, vec![fact("record", format!("{titles} national titles"))]), false).unwrap();
    assert_eq!(check::read(&b, &s, &fact("record", format!("{titles} national titles"))).truth, Truth::True);
    untyped(&mut b);
    assert!(bytes(&a) == bytes(&b), "a true fact counts what the point counts, and nothing more");
    // The same point with titles you don't have: he knows as you say it.
    let lie = fact("record", format!("{} national titles", titles + 5));
    let s = scene::say(&mut c, id, typed(code, vec![lie.clone()]), false).unwrap();
    assert_eq!(check::read(&c, &s, &lie), check::Found { truth: Truth::False, sees: None });
    assert!(c.folk.live.lies.is_empty(), "a false thing he can check now is never saved");
    let after = total(&c, i, mine);
    assert!(
        (after - (before - OVERSELL).max(case::CASE_FLOOR)).abs() < 1e-6,
        "the lift is lost and the oversell costs: {before} -> {after}"
    );
    assert!(after < total(&a, i, mine));
    let t = c.folk.mine.name(Name::Trail);
    assert!((t - (trail - check::CAUGHT_TRAIL)).abs() < 1e-6, "your name on the trail: {trail} -> {t}");
    assert!(s.outcome.iter().any(|o| o.contains("wasn't so")));
}

#[test]
fn a_nil_number_he_cant_check_yet_is_seen_once_hes_yours_and_never_elsewhere() {
    let (_, mut b, i, mine, id, code) = two(30).expect("a sit-down with a point to make");
    let him = b.people[i].player.id;
    let other = (0..b.programs.len()).find(|&j| j != mine).unwrap();
    let mut a = copy(&b);
    scene::answer(&mut a, id, code).unwrap();
    let money = fact("nil_amount", "$9 million a year".into());
    let s = scene::say(&mut b, id, typed(code, vec![money.clone()]), false).unwrap();
    let found = check::read(&b, &s, &money);
    let day = check::money_day(&b, i, false);
    assert_eq!(found, check::Found { truth: Truth::False, sees: Some(day) });
    assert!((total(&a, i, mine) - total(&b, i, mine)).abs() < 1e-6, "nothing costs until he can see it");
    let lie = b.folk.live.lies.iter().find(|l| l.to == PromiseTo::Player(him)).expect("the lie is saved");
    assert!(lie.recruit && !lie.seen && lie.sees == day);
    // A save with no key: the lie is judged from the world alone.
    let mut b = copy(&b);
    let signed =
        |j| Signed { program: j, walk_on: false, date: day, early_enrollee: false, flipped: false, committed: None };
    // He signs elsewhere: he never sees it.
    let mut gone = copy(&b);
    gone.people[i].cal.signed = Some(signed(other));
    let trust = gone.people[i].trust;
    talks::day(&mut gone, day);
    let l = gone.folk.live.lies.iter().find(|l| l.to == PromiseTo::Player(him)).unwrap();
    assert!(l.seen && gone.people[i].trust == trust, "gone elsewhere, it lapses at no cost");
    // He signs with you: looked at again until he arrives.
    b.people[i].cal.signed = Some(signed(mine));
    talks::day(&mut b, day);
    let l = b.folk.live.lies.iter().find(|l| l.to == PromiseTo::Player(him)).unwrap().clone();
    assert!(!l.seen && l.sees > day, "signed, not here yet");
    // He arrives: he sees it, and it costs his trust and his mood.
    b.people[i].stage = Stage::College;
    b.people[i].college = Some(mine);
    b.people[i].cal.signed = None;
    b.programs[mine].roster.push(i);
    b.people[i].trust = 0.6;
    talks::day(&mut b, l.sees);
    let l = b.folk.live.lies.iter().find(|l| l.to == PromiseTo::Player(him)).unwrap();
    assert!(l.seen);
    let t = b.people[i].trust;
    assert!(t < 0.6 && t > 0.6 - 0.22, "a broken promise's cost by his belief: {t}");
    let moods = b.folk.moods.iter().find(|x| x.0 == him).map(|x| x.1.clone()).unwrap_or_default();
    assert!(moods.iter().any(|m| m.1 == Reason::RecruitLie as u8), "his mood carries why: {moods:?}");
}

#[test]
fn a_typed_promise_lands_on_the_promise_it_means_and_asks_first() {
    let (w, mine, kids) = class(30);
    let mut tried = 0;
    for &i in &kids {
        let (mut a, mut b) = (copy(&w), copy(&w));
        let s = sit_down(&mut a, i, mine);
        sit_down(&mut b, i, mine);
        if s.done || !s.picks.iter().any(|p| p.code == PROMISE_MENU) {
            continue;
        }
        tried += 1;
        // Every promise he can be made is in reach of a typed line.
        for (kind, k) in [("start", 0), ("role", 1), ("position", 2), ("early_chance", 3), ("no_redshirt", 4)] {
            assert_eq!(pitch::typed_promise(&s, kind), Some(PROMISE_START + k), "{kind}");
        }
        assert_eq!(pitch::typed_promise(&s, "contract"), None);
        // Typed, it asks first; confirmed, it is the chip's promise.
        let err = scene::say(&mut b, s.id, typed(PROMISE_START, Vec::new()), false).unwrap_err();
        assert_eq!(err, scene::ASK_FIRST);
        scene::say(&mut b, s.id, typed(PROMISE_START, Vec::new()), true).unwrap();
        scene::answer(&mut a, s.id, PROMISE_MENU).unwrap();
        scene::answer(&mut a, s.id, PROMISE_START).unwrap();
        assert_eq!(pledge::on_case(&b, i, mine), Some(pledge::Kind::Start));
        assert_eq!(case::of(&a.people[i], mine), case::of(&b.people[i], mine), "the same case as the chip");
        // One promise a school: none is in reach after it.
        let mut c = copy(&b);
        let s = sit_down(&mut c, i, mine);
        assert!(s.done || pitch::typed_promise(&s, "role").is_none());
        if tried >= 3 {
            break;
        }
    }
    assert!(tried > 0, "some sit-down could make a promise");
}
