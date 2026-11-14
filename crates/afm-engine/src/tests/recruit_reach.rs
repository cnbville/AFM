//! The recruit's other talks (talk facts lane L6; `world::folk::{reach,
//! pledge}`, `calendar::contact`): a call is three exchanges at half a
//! sit-down and a message one at a quarter, on the era's rules; his
//! family calls when something they weigh changes, and opens on it; the
//! official visit ends in your office; a promise to a recruit pulls by how
//! far they believe it and is judged on what he sees once he's yours; and
//! AI staffs' calls and official-visit sit-downs fill their cases in one
//! step under the one cap.

use super::legacy::seat;
use crate::calendar::{Date, Weekday};
use crate::world::data::Division;
use crate::world::folk::case::{self, CASE_TOP, MESSAGE, PHONE};
use crate::world::folk::claims::{self, Ear, Group, Kind, Tier};
use crate::world::folk::scene::{self, Kind as Scenes, Scene};
use crate::world::folk::talks::{self, PromiseKind, PromiseTo};
use crate::world::folk::{pledge, reach};
use crate::world::front_office::college as fc;
use crate::world::{Signed, Stage, World, recruiting};

const POINT: std::ops::Range<u32> = 7100..7300;
const GROUP: std::ops::Range<u32> = 7500..7507;
const LEAVE: u32 = 719;
const PROMISE_MENU: u32 = 7520;

/// Your FBS program and `n` high school seniors of the class signing next.
fn class(n: usize) -> (World, usize, Vec<usize>) {
    let mut w = seat("college");
    let mine = w.seat.and_then(|s| s.program).unwrap();
    w.add_class_aged(17);
    let next = recruiting::next_class(&w);
    let kids: Vec<usize> = (0..w.people.len()).filter(|&i| w.people[i].stage == Stage::HighSchool).take(n).collect();
    for &i in &kids {
        w.people[i].class_of = next;
    }
    (w, mine, kids)
}

/// Plays a talk to its end: the first point on the table each exchange,
/// counting the points made.
fn play(w: &mut World, mut s: Scene) -> (Scene, usize) {
    let mut made = 0;
    for _ in 0..30 {
        if s.done {
            break;
        }
        let code = s
            .picks
            .iter()
            .chain(&s.reach)
            .find(|p| POINT.contains(&p.code))
            .map(|p| p.code)
            .or_else(|| s.picks.iter().find(|p| GROUP.contains(&p.code)).map(|p| p.code))
            .unwrap_or(LEAVE);
        made += usize::from(POINT.contains(&code));
        s = scene::answer(w, s.id, code).expect("an answer on the table");
    }
    assert!(s.done, "the talk ends");
    (s, made)
}

#[test]
fn a_call_is_three_exchanges_at_half_and_the_rules_hold_you_to_a_day() {
    let (mut w, mine, kids) = class(10);
    let mut full = 0;
    for &i in &kids {
        let id = w.people[i].player.id;
        // From 2023 a school may call him as often as it likes; the game's
        // own pace is a call a day.
        assert!(reach::call_rule(&w, id).is_ok(), "{:?}", reach::call_rule(&w, id));
        let sid = reach::call(&mut w, id).expect("you may call him");
        assert!(reach::call(&mut w, id).is_err(), "one call a day");
        let s = scene::open(&mut w, sid).unwrap();
        assert!(s.lines.iter().any(|l| l.by == 2), "he speaks on the phone");
        let (s, made) = play(&mut w, s);
        assert!(made <= 3, "a call is 3 exchanges, not {made}");
        full += usize::from(made == 3);
        // Each point lands at half a sit-down's size at most.
        let c = case::of(&w.people[i], mine).copied().unwrap_or_default();
        for g in Group::ALL {
            assert!(c.groups[g as usize] <= 0.03 * PHONE + 1e-6, "{g:?} {}", c.groups[g as usize]);
        }
        assert!(s.f[0] <= 3.0 * 0.03 * PHONE + 1e-6);
        assert_eq!(reach::contacts(&w, id), 1, "the call counts as a contact");
    }
    assert!(full > 0, "some call used all three exchanges");
}

#[test]
fn a_message_is_one_exchange_at_a_quarter() {
    let (mut w, mine, kids) = class(10);
    let mut sent = 0;
    for &i in &kids {
        let id = w.people[i].player.id;
        let sid = reach::message(&mut w, id).expect("you may write to him");
        assert!(reach::message(&mut w, id).is_err(), "one message a day");
        let s = scene::open(&mut w, sid).unwrap();
        // You write first: nothing of his before your words, and no free
        // questions in a message.
        assert!(!s.lines.iter().any(|l| l.by == 2));
        assert!(!s.picks.iter().any(|p| [709, 7601, 7602, 7603].contains(&p.code)));
        let before = case::of(&w.people[i], mine).map_or(0.0, |c| c.total());
        let (s, made) = play(&mut w, s);
        assert!(made <= 1, "a message is one exchange");
        if made == 1 {
            sent += 1;
            let after = case::of(&w.people[i], mine).map_or(0.0, |c| c.total());
            assert!(after - before <= 0.03 * MESSAGE + 1e-6, "a message lands at a quarter: {}", after - before);
            assert!(s.lines.iter().any(|l| l.by == 2), "he answers");
        }
    }
    assert!(sent > 0, "some message made a point");
}

#[test]
fn his_family_calls_about_a_commitment_at_his_spot_and_opens_on_it() {
    let (mut w, mine, kids) = class(400);
    // A recruit holding your offer whose family weighs playing time.
    let i = *kids
        .iter()
        .find(|&&i| {
            claims::group_cares(&w.people[i], Ear::Decider)[Group::PlayingTime as usize] >= reach::FAMILY_CARE + 0.1
        })
        .expect("a family that weighs playing time");
    let id = w.people[i].player.id;
    fc::add(&mut w, id).unwrap();
    fc::offer(&mut w, id).unwrap();
    // Another recruit at his spot commits to you today.
    let pos = w.people[i].player.position;
    let other = *kids.iter().find(|&&k| k != i && w.people[k].player.position == pos).expect("another at his spot");
    let today = w.today;
    w.people[other].cal.commit = Some((mine, today, false));
    let (ev, g) = reach::family_reason(&w, i, mine).expect("the family has something to call about");
    assert_eq!((ev, g), (Kind::CommitAtSpot, Group::PlayingTime));
    // Within the week his day comes up, and they call.
    for _ in 0..7 {
        let d = w.today;
        reach::day(&mut w, mine, d);
        if w.folk.scenes.iter().any(|s| s.kind == Scenes::FamilyCall && s.a == id as u64) {
            break;
        }
        w.today = w.today.add(1);
    }
    let sid = w.folk.scenes.iter().find(|s| s.kind == Scenes::FamilyCall && s.a == id as u64).expect("they call").id;
    let s = scene::open(&mut w, sid).unwrap();
    let name = w.people[other].name.clone();
    assert!(s.lines.iter().any(|l| l.by == 2 && l.text.contains(&name)), "they open on what changed: {:?}", s.lines);
    let (_, made) = play(&mut w, s);
    assert!(made <= 2, "a family's call is one or two exchanges");
    // What they called about is talked through: no second call about it.
    assert!(reach::family_reason(&w, i, mine).is_none_or(|x| x.0 != Kind::CommitAtSpot));
}

#[test]
fn the_official_visit_ends_with_your_case_and_a_promise() {
    let (mut w, mine, kids) = class(30);
    let mut promised = 0;
    for &i in &kids {
        let id = w.people[i].player.id;
        fc::add(&mut w, id).unwrap();
        fc::offer(&mut w, id).unwrap();
        let today = w.today;
        reach::official_talk(&mut w, id, mine, today);
        let sid = w.folk.scenes.iter().rev().find(|s| s.kind == Scenes::OfficialTalk && s.a == id as u64).unwrap().id;
        let mut s = scene::open(&mut w, sid).unwrap();
        assert!(s.stops, "it waits for you");
        // You can promise him something any time while your offer is out.
        assert!(s.done || s.picks.iter().any(|p| p.code == PROMISE_MENU));
        let mut made = 0;
        for _ in 0..30 {
            if s.done {
                break;
            }
            let point = s.picks.iter().chain(&s.reach).find(|p| POINT.contains(&p.code)).map(|p| p.code);
            // At the end: the promise of his position (731 + 2).
            let code = point.or_else(|| s.picks.iter().find(|p| p.code == 733).map(|p| p.code)).unwrap_or(LEAVE);
            made += usize::from(point.is_some());
            s = scene::answer(&mut w, s.id, code).unwrap();
        }
        assert!(s.done && made <= 6);
        if pledge::on_case(&w, i, mine) == Some(pledge::Kind::Position) {
            promised += 1;
            let c = *case::of(&w.people[i], mine).unwrap();
            assert!(c.pledge > 0.0 && c.pledge <= pledge::PULL[2] as f32 * 2.5, "{}", c.pledge);
            assert!(case::pull(&w.people[i], mine) >= (c.total() + c.pledge) as f64 - 1e-9);
        }
    }
    assert!(promised > 0, "the talk ends on the promise");
}

#[test]
fn a_recruits_promise_is_his_once_he_signs_and_judged_on_what_he_sees() {
    let (mut w, mine, kids) = class(4);
    let i = kids[0];
    let id = w.people[i].player.id;
    fc::add(&mut w, id).unwrap();
    fc::offer(&mut w, id).unwrap();
    let (pull, belief) = pledge::make(&mut w, i, mine, pledge::Kind::Position, Ear::Decider, 0.5).unwrap();
    assert!(pull > 0.0 && (0.0..=1.0).contains(&belief));
    assert!(pledge::make(&mut w, i, mine, pledge::Kind::Start, Ear::Decider, 0.5).is_none(), "one promise a school");
    // He signs with you: the promise is yours, due at the season's end.
    let today = w.today;
    w.people[i].cal.signed = Some(Signed {
        program: mine,
        walk_on: false,
        date: today,
        early_enrollee: false,
        flipped: false,
        committed: None,
    });
    pledge::log_signed(&mut w, mine);
    pledge::log_signed(&mut w, mine);
    let mine_ps: Vec<_> = w.folk.promises.iter().filter(|p| p.to == PromiseTo::Player(id)).cloned().collect();
    assert_eq!(mine_ps.len(), 1, "logged once");
    let p = &mine_ps[0];
    assert!(matches!(p.kind, PromiseKind::Recruit(2, _)));
    assert!(p.due > today);
    // On your roster at his position: kept. Moved: broken. Never came: it
    // lapses.
    w.people[i].stage = Stage::College;
    w.people[i].college = Some(mine);
    w.people[i].cal.signed = None;
    w.programs[mine].roster.push(i);
    assert!(talks::kept(&w, p));
    let was = w.people[i].player.position;
    w.people[i].player.position =
        crate::positions::Position::ALL.into_iter().find(|&x| x != was && !x.is_specialist()).unwrap();
    assert!(!talks::kept(&w, p), "you moved him");
    w.people[i].player.position = was;
    w.people[i].college = None;
    w.programs[mine].roster.retain(|&k| k != i);
    w.people[i].stage = Stage::HighSchool;
    assert!(talks::lapsed(&w, p), "he never came");
}

#[test]
fn ai_staffs_call_and_sit_down_in_one_step_under_the_cap() {
    let (mut w, mine, kids) = class(200);
    let others: Vec<usize> = (0..w.programs.len())
        .filter(|&j| j != mine && matches!(w.programs[j].division, Division::FBS | Division::FCS))
        .collect();
    for (k, &i) in kids.iter().enumerate() {
        w.people[i].cal.offers = (1..=3).map(|x| others[(k * 7 + x * 41) % others.len()]).collect();
    }
    // A Wednesday: the staffs' calls.
    let mut d = w.today;
    while d.weekday() != Weekday::Wed {
        d = d.add(1);
    }
    w.today = d;
    reach::ai_calls(&mut w, d);
    let mut cases = 0;
    for &i in &kids {
        for c in &w.people[i].cal.cases {
            cases += 1;
            assert!(c.prog as usize != mine, "your calls are yours to make");
            for g in Group::ALL {
                assert!(c.groups[g as usize] <= 0.03 * PHONE + 1e-6, "a call lands at half");
            }
        }
    }
    let offers = kids.len() * 3;
    assert!(
        cases > offers / 4 && cases < offers,
        "about half the offering staffs called this week: {cases} of {offers}"
    );
    // An official visit's sit-down lands at full size, and many talks stay
    // under the one cap.
    for &i in &kids {
        let j = w.people[i].cal.offers[0];
        for k in 0..5 {
            reach::ai_official_talk(&mut w, i, j, d.add(k));
        }
        let c = case::of(&w.people[i], j).unwrap();
        assert!(c.total() <= CASE_TOP + 1e-6);
    }
    // Before 2023 a junior can't be called in the fall.
    assert!(crate::calendar::contact::call_rule(&w.dates, Date::ymd(2019, 10, 2), 2021, true, &[]).is_err());
    let _ = Tier::Fair;
}
