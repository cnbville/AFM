//! Live talks, the core (lane 3): a typed line lands on a scene's own
//! options through the same dispatch as a click, so the world after is the
//! same; a pick that can't be undone is refused until the reading is
//! confirmed; a pick that only moves around the talk is never landed; what
//! the line carried rides on the scene for one answer and is never saved;
//! and the cost of a promise, now written once, gives today's numbers.

use super::legacy::seat;
use crate::calendar::Weekday;
use crate::player::PlayingTime;
use crate::world::World;
use crate::world::coach::{Group, Role};
use crate::world::folk::scene::{self, Heard, Kind, Pick, PickKind, Scene, Who};
use crate::world::folk::talks::{self, PromiseKind, PromiseTo, Reason};
use crate::world::folk::{Team, finals, staffing};

/// A coach who isn't on your staff.
fn outsider(w: &World) -> usize {
    let team = Team::seat(w).unwrap();
    (0..w.coaches.len())
        .find(|&c| w.coaches[c].job.is_none_or(|j| Team::from_employer(j.employer) != team))
        .expect("a coach elsewhere")
}

/// An open staff interview with a coach from elsewhere.
fn interview(w: &mut World) -> u32 {
    let c = outsider(w);
    let id = staffing::start_interview(w, c, Role::PositionCoach(Group::QB)).unwrap();
    scene::open(w, id).unwrap();
    id
}

fn typed(code: u32, words: &str) -> Heard {
    Heard { pick: code, tone: "calm".into(), words: words.into(), ..Heard::default() }
}

fn bytes(w: &World) -> Vec<u8> {
    rmp_serde::to_vec(w).unwrap()
}

fn yours(s: &Scene) -> usize {
    s.lines.iter().filter(|l| l.by == 1).count()
}

/// Takes the typed words out of a world's scenes, to compare it with the
/// world after the chip.
fn untyped(w: &mut World) {
    for s in &mut w.folk.scenes {
        for l in &mut s.lines {
            l.typed = None;
        }
    }
}

#[test]
fn a_typed_question_is_the_clicked_one() {
    for code in [60, 61, 62, 63] {
        let (mut a, mut b) = (seat("college"), seat("college"));
        let (ia, ib) = (interview(&mut a), interview(&mut b));
        assert_eq!(ia, ib);
        let clicked = scene::answer(&mut a, ia, code).unwrap();
        let words = "So walk me through what you'd do with the guys we already have here.";
        let said = scene::say(&mut b, ib, typed(code, words), false).unwrap();
        assert_eq!(yours(&clicked), yours(&said), "{code}: the same count of your lines");
        assert_eq!(clicked.lines.len(), said.lines.len(), "{code}: never a second line");
        let mine = said.lines.iter().rev().find(|l| l.by == 1).unwrap();
        assert_eq!(mine.typed.as_deref(), Some(words), "{code}: your words on the pick's own line");
        assert!(said.heard.is_none() && b.folk.scenes.iter().all(|s| s.heard.is_none()), "never kept");
        // The same world after, the typed words aside.
        untyped(&mut b);
        assert!(bytes(&a) == bytes(&b), "{code}: the world after a typed question is the chip's");
        // The next question is offered the same way.
        let next: Vec<u32> = said.picks.iter().map(|p| p.code).collect();
        assert_eq!(next, clicked.picks.iter().map(|p| p.code).collect::<Vec<_>>());
    }
}

#[test]
fn the_same_reading_on_the_same_save_gives_the_same_world() {
    let (mut a, mut b) = (seat("college"), seat("college"));
    let (ia, ib) = (interview(&mut a), interview(&mut b));
    for (w, id) in [(&mut a, ia), (&mut b, ib)] {
        scene::say(w, id, typed(62, "Where do you recruit, and who would come with you?"), false).unwrap();
        scene::say(w, id, typed(63, "What's it going to take to get you here?"), false).unwrap();
    }
    assert!(bytes(&a) == bytes(&b));
}

#[test]
fn a_line_that_ends_the_interview_shows_your_words() {
    let mut w = seat("college");
    let id = interview(&mut w);
    scene::say(&mut w, id, typed(60, "How would you use our quarterbacks?"), false).unwrap();
    let before = yours(w.folk.scenes.iter().find(|s| s.id == id).unwrap());
    let s = scene::say(&mut w, id, typed(69, "That's all I needed. Thanks for coming in."), false).unwrap();
    assert!(s.done, "69 ends it, typed as clicked");
    assert_eq!(yours(&s), before + 1, "an answer that writes no line of yours gets one");
    let last = s.lines.iter().rev().find(|l| l.by == 1).unwrap();
    assert_eq!(last.text, "Thank him and wrap it up", "the chip's words, as the game read it");
    assert_eq!(last.typed.as_deref(), Some("That's all I needed. Thanks for coming in."));
    assert!(scene::say(&mut w, id, typed(60, "One more thing"), false).is_err(), "a meeting that is over");
}

#[test]
fn a_line_is_refused_when_it_reaches_nothing_on_the_table() {
    let mut w = seat("college");
    let id = interview(&mut w);
    assert!(scene::say(&mut w, id, typed(61, ""), false).is_err(), "an empty line");
    let long = "x".repeat(scene::LIMIT + 1);
    assert!(scene::say(&mut w, id, typed(61, &long), false).is_err(), "over the limit");
    assert!(scene::say(&mut w, id, typed(999, "Hello"), false).is_err(), "not an option");
    // Nothing moved: the interview still waits on its first question.
    let s = w.folk.scenes.iter().find(|s| s.id == id).unwrap();
    assert_eq!((s.step, yours(s)), (0, 0));
}

/// A scene with a person, opened, with the picks given.
fn bare(kind: Kind, picks: Vec<Pick>, reach: Vec<Pick>) -> Scene {
    let mut s = Scene::new(kind, crate::calendar::Date(0), "Test");
    s.opened = true;
    s.who.push(Who::Group("the team".into()));
    s.picks = picks;
    s.reach = reach;
    s
}

fn pick(code: u32) -> Pick {
    Pick { label: format!("Option {code}"), mv: "Ask".into(), tone: "calm".into(), code, ..Pick::default() }
}

#[test]
fn a_typed_line_never_lands_on_a_nav_pick() {
    let mut w = seat("college");
    let mut s = bare(Kind::TeamMeeting, vec![pick(10)], vec![pick(11), pick(19).of(PickKind::Nav)]);
    s.date = w.today;
    let id = scene::add(&mut w, s);
    let s = w.folk.scenes.iter().find(|s| s.id == id).unwrap().clone();
    let codes: Vec<u32> = scene::options(&s).iter().map(|p| p.code).collect();
    assert_eq!(codes, [10, 11], "the table and what's behind 'something else', never a nav pick");
    assert_eq!(
        scene::say(&mut w, id, typed(19, "Let's talk about something else."), true).unwrap_err(),
        "That answer isn't on the table."
    );
}

#[test]
fn a_final_pick_asks_first_and_no_caller_can_skip_it() {
    let mut w = seat("college");
    let s = bare(Kind::TeamMeeting, vec![pick(10), pick(12).of(PickKind::Final)], Vec::new());
    assert!(!scene::asks_first(&s, &s.picks[0]));
    assert!(scene::asks_first(&s, &s.picks[1]), "marked final on the pick");
    // The table: codes repeat across kinds, so the kind decides.
    assert!(finals::is_final(Kind::Firing, 70) && !finals::is_final(Kind::Trainer, 70));
    assert!(finals::is_final(Kind::Discipline, 505) && !finals::is_final(Kind::Discipline, 500));
    let t = bare(Kind::Discipline, vec![pick(500), pick(505)], Vec::new());
    assert!(!scene::asks_first(&t, &t.picks[0]) && scene::asks_first(&t, &t.picks[1]), "listed in the table");
    let mut rows = finals::FINAL.to_vec();
    rows.sort_by_key(|r| (format!("{:?}", r.0), r.1));
    rows.dedup();
    assert_eq!(rows.len(), finals::FINAL.len(), "no row twice");
    // A real one: letting a coach go.
    let team = Team::seat(&w).unwrap();
    let c = (0..w.coaches.len()).find(|&c| {
        w.coaches[c].job.is_some_and(|j| Team::from_employer(j.employer) == team && j.role != Role::HeadCoach)
    });
    let Some(c) = c else { panic!("an assistant on your staff") };
    let id = staffing::start_firing(&mut w, c).unwrap();
    scene::open(&mut w, id).unwrap();
    let err = scene::say(&mut w, id, typed(71, "I'm sorry. I'll help you land somewhere."), false).unwrap_err();
    assert_eq!(err, scene::ASK_FIRST);
    let s = w.folk.scenes.iter().find(|s| s.id == id).unwrap();
    assert!(!s.done && s.step == 0, "nothing happened");
    assert!(w.coaches[c].job.is_some_and(|j| Team::from_employer(j.employer) == team), "he still works for you");
    let s = scene::say(&mut w, id, typed(71, "I'm sorry. I'll help you land somewhere."), true).unwrap();
    assert!(s.done);
    assert!(w.coaches[c].job.is_none_or(|j| Team::from_employer(j.employer) != team), "confirmed, he is gone");
}

#[test]
fn press_media_stories_and_one_on_ones_take_no_typed_line() {
    for kind in [Kind::Press, Kind::PressIntro, Kind::Media, Kind::Story, Kind::OneOnOne] {
        let mut w = seat("college");
        let mut s = bare(kind, vec![pick(100)], Vec::new());
        s.date = w.today;
        let id = scene::add(&mut w, s);
        let s = w.folk.scenes.iter().find(|s| s.id == id).unwrap();
        assert!(!scene::takes_typed(s), "{kind:?}");
        let err = scene::say(&mut w, id, typed(100, "We're here to win."), true).unwrap_err();
        assert_eq!(err, "This talk takes no typed lines.", "{kind:?}");
    }
    assert!(!talks::ONE_ON_ONE_TYPED, "lane 9 turns it on");
    // A scene with nobody in it (a report) has no box either.
    let mut s = bare(Kind::TeamMeeting, vec![pick(10)], Vec::new());
    s.who.clear();
    assert!(!scene::takes_typed(&s));
}

#[test]
fn heard_rides_on_the_scene_for_one_answer_only() {
    let mut w = seat("college");
    let id = interview(&mut w);
    let mut h = typed(61, "Tell me how you'd handle our toughest kid.");
    h.named = vec![1, 2];
    scene::say(&mut w, id, h, false).unwrap();
    assert!(w.folk.scenes.iter().all(|s| s.heard.is_none()), "cleared right after the dispatch");
    // A save never carries it.
    let back: World = rmp_serde::from_slice(&rmp_serde::to_vec(&w).unwrap()).unwrap();
    assert!(back.folk.scenes.iter().all(|s| s.heard.is_none()));
    let s = back.folk.scenes.iter().find(|s| s.id == id).unwrap();
    assert!(s.lines.iter().any(|l| l.typed.is_some()), "your words are saved with the line");
    // A chip after it lands as a chip.
    let s = scene::answer(&mut w, id, 62).unwrap();
    assert!(s.lines.iter().rev().find(|l| l.by == 1).unwrap().typed.is_none());
}

#[test]
fn the_lies_store_waits_for_the_promise_design() {
    use crate::world::folk::claims::{self, Lie, Stated};
    let mut w = seat("college");
    let i = Team::seat(&w).unwrap().roster(&w)[0];
    let id = w.people[i].player.id;
    let (trust, mood, straight) = (w.people[i].trust, w.people[i].mood, w.folk.mine.straight);
    let fact = Stated { kind: "depth_rank".into(), about: Some(id), value: "starter".into() };
    let today = w.today;
    claims::truth::file_lie(
        &mut w,
        Lie { to: PromiseTo::Player(id), fact, said: today, sees: today, seen: false, who: String::new() },
    );
    claims::truth::check_lies(&mut w, today);
    assert!(w.folk.live.lies[0].seen, "its day came");
    // It costs nothing until Cain answers the promise design.
    assert_eq!((w.people[i].trust, w.people[i].mood, w.folk.mine.straight), (trust, mood, straight));
    // Trust in your word: an agent or a GM starts in the middle.
    assert_eq!(w.folk.live.word_of(7), claims::truth::WORD_START);
    w.folk.live.set_word(7, 1.4);
    assert_eq!(w.folk.live.word_of(7), 1.0);
    // An old save opens with an empty store, and an empty one isn't written.
    let mut folk = serde_json::to_value(&w.folk).unwrap();
    folk.as_object_mut().unwrap().remove("live");
    let back: crate::world::folk::Folk = serde_json::from_value(folk).unwrap();
    assert!(back.live.is_empty());
    let fresh = crate::world::folk::Folk::default();
    assert!(serde_json::to_value(&fresh).unwrap().get("live").is_none());
    assert_eq!(Reason::from_u8(Reason::LieSeen as u8), Reason::LieSeen);
    assert_eq!(Reason::from_u8(Reason::HardballHeard as u8), Reason::HardballHeard);
    assert_eq!(Reason::from_u8(Reason::RecruitLie as u8), Reason::RecruitLie);
}

/// A player on your roster who isn't a starter, and the next weekday that
/// isn't a Monday.
fn backup(w: &mut World) -> (usize, u32) {
    let i = Team::seat(w).unwrap().roster(w)[0];
    let id = w.people[i].player.id;
    w.people[i].last_role = Some(PlayingTime::Backup);
    for (_, ids) in w.seat_depth.iter_mut() {
        ids.retain(|&x| x != id);
    }
    w.people[i].trust = 0.6;
    w.people[i].mood = 0.0;
    (i, id)
}

#[test]
fn a_promise_costs_what_it_cost_before_the_move() {
    for public in [false, true] {
        let mut w = seat("college");
        let (i, id) = backup(&mut w);
        talks::promise(&mut w, PromiseTo::Player(id), PromiseKind::Start, 0);
        let k = w.folk.promises.len() - 1;
        w.folk.promises[k].public = public;
        let mut d = w.today;
        while d.weekday() == Weekday::Mon {
            d = d.add(1);
        }
        w.folk.promises[k].due = d;
        let straight = w.folk.mine.straight;
        talks::day(&mut w, d);
        assert_eq!(w.folk.promises[k].kept, Some(false));
        let cost = if public { 2.0 } else { 1.0 };
        let dt = -0.22 * cost;
        assert!((w.people[i].trust - (0.6 + dt)).abs() < 1e-6, "{public}: trust {}", w.people[i].trust);
        let note = w.folk.moods.iter().find(|x| x.0 == id).and_then(|x| x.1.last().copied()).unwrap();
        assert_eq!((note.1, note.2), (Reason::PromiseBroken as u8, (dt * 1.4 * 100.0).round() as i8));
        assert!((w.folk.mine.straight - (straight - 0.08 * cost).max(-1.0)).abs() < 1e-6);
    }
    // Kept: a start he got.
    let mut w = seat("college");
    let (i, id) = backup(&mut w);
    talks::promise(&mut w, PromiseTo::Player(id), PromiseKind::Snaps, 0);
    let k = w.folk.promises.len() - 1;
    w.people[i].last_role = Some(PlayingTime::Rotation);
    let mut d = w.today;
    while d.weekday() == Weekday::Mon {
        d = d.add(1);
    }
    w.folk.promises[k].due = d;
    let straight = w.folk.mine.straight;
    talks::day(&mut w, d);
    assert_eq!(w.folk.promises[k].kept, Some(true));
    assert!((w.people[i].trust - 0.7).abs() < 1e-6);
    assert!((w.folk.mine.straight - (straight + 0.04).min(1.0)).abs() < 1e-6);
}
