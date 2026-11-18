//! Coaches answer with real specifics (Cain 2026-11-17): a coordinator
//! asked "what type of offense will you be running?" wouldn't hand over his
//! scheme "on a napkin", a recruiter claimed "the best kid in the state" and
//! wouldn't name him, a staff room answered through one nameless "he", and
//! "respected on his pride" came with "he doesn't like the question". These
//! prove that a coach of yours tells his system and his recruiting from the
//! world, straight, that a proud man's tell follows how the line landed,
//! and that a staff room's answer comes from the coach it fits.

use super::legacy::seat;
use crate::world::World;
use crate::world::folk::Team;
use crate::world::folk::persona::coaching;
use crate::world::folk::persona::ear::{Act, Reading};
use crate::world::folk::persona::knows::{Answer, Knows, TellParts, Tells, Topic};
use crate::world::folk::persona::makeup::Makeup;
use crate::world::folk::persona::people::People;
use crate::world::folk::persona::room::{Channel, Group, Place, Present, Room, Seat};
use crate::world::folk::persona::{Them, telling};

/// Your college world and two coaches on your staff.
fn staff() -> (World, usize, usize) {
    let w = seat("college");
    let team = Team::seat(&w).unwrap();
    let me = w.career.as_ref().map(|c| c.coach);
    let v: Vec<usize> =
        crate::world::folk::facts::assistants(&w, team).into_iter().map(|x| x.0).filter(|&c| Some(c) != me).collect();
    assert!(v.len() >= 2, "two staff coaches");
    (w, v[0], v[1])
}

#[test]
fn a_coach_of_yours_tells_his_scheme_straight() {
    let (w, c, _) = staff();
    let him = Them::Coach(c);
    let known = People.knows(&w, him, Topic::Scheme, None);
    assert!(!known.is_empty(), "his scheme is the world's");
    let all = known.iter().map(|k| k.words.as_str()).collect::<Vec<_>>().join(" | ");
    let names = ["West Coast", "air raid", "wide zone", "power gap", "spread option", "4-3", "3-4", "kicking game"];
    assert!(names.iter().any(|n| all.contains(n)), "names his system: {all}");
    // However little he trusts you, he tells his head coach his scheme.
    let cold = TellParts { trust: 0.0, professionalism: 1.0, ..Default::default() };
    let told = People.tell(&w, him, &known, &cold);
    assert_eq!(told.answer, Answer::Straight, "{told:?}");
    assert!(!told.holding_back);
    // The same for his unit and his recruiting.
    for t in [Topic::Football, Topic::Recruiting] {
        let k = People.knows(&w, him, t, None);
        assert!(!k.is_empty(), "{t:?}");
        assert_eq!(People.tell(&w, him, &k, &cold).answer, Answer::Straight, "{t:?}");
    }
}

#[test]
fn a_recruiter_tells_his_area_and_his_board() {
    let (w, c, d) = staff();
    let k = People.knows(&w, Them::Coach(c), Topic::Recruiting, None);
    assert!(!k.is_empty());
    assert!(
        k[0].words.starts_with("he recruits ") || k[0].words.starts_with("he has no recruiting area"),
        "{:?}",
        k[0].words
    );
    // Every recruit he can name is a real high schooler or a signing of his.
    for i in coaching::named(&w, c) {
        assert!(k.iter().any(|x| x.words.contains(&w.people[i].name)), "{} named in {k:?}", w.people[i].name);
    }
    // Asked about a colleague's recruiting, he tells what the staff room
    // knows; another school's coach is a reference's to read.
    let col = People.knows(&w, Them::Coach(c), Topic::Recruiting, Some(Them::Coach(d)));
    assert!(!col.is_empty());
}

#[test]
fn a_proud_mans_tell_follows_how_the_line_landed() {
    let (w, _, _) = staff();
    let m = Makeup {
        temper: 10.0,
        loyalty: 10.0,
        professionalism: 10.0,
        ego: 18.0,
        honesty: 10.0,
        patience: 10.0,
        warmth: 10.0,
        leadership: 10.0,
        greed: 10.0,
        ambition: 10.0,
        iq: 10.0,
        judgement: 10.0,
    };
    const PROUD: &str = "He doesn't like the question.";
    let fair: Vec<String> = (0..300).map(|s| telling::tell_words(&w, &m, Answer::Partly, s, false)).collect();
    assert!(!fair.iter().any(|t| t.contains(PROUD)), "a fair question never draws it");
    let stung: Vec<String> = (0..300).map(|s| telling::tell_words(&w, &m, Answer::Partly, s, true)).collect();
    let weak = stung.iter().any(|t| t == "He shifts in his chair.");
    assert!(weak || stung.iter().any(|t| t.contains(PROUD)), "a question that stung him can");
}

#[test]
fn a_staff_room_is_answered_by_the_coach_it_fits() {
    let (w, c, d) = staff();
    let room = Room {
        channel: Channel::Group(Group::Staff),
        present: vec![
            Present { who: Them::Coach(c), seat: Seat::Listener, speaks: false },
            Present { who: Them::Coach(d), seat: Seat::Listener, speaks: false },
        ],
    };
    let ask = |topic, about| Reading {
        acts: vec![Act::Question { topic, about, words: String::new() }],
        ..Reading::default()
    };
    // A question that names a coach: him.
    assert_eq!(coaching::answerer(&w, &room, &ask(Topic::Mood, Some(Them::Coach(d)))), Some(Them::Coach(d)));
    // Recruiting: the man with the best record.
    let best = if coaching::record_score(&w, d) > coaching::record_score(&w, c) { d } else { c };
    assert_eq!(coaching::answerer(&w, &room, &ask(Topic::Recruiting, None)), Some(Them::Coach(best)));
    // He leads the room the talk reads.
    let led = coaching::lead_with(room.clone(), Them::Coach(d));
    assert_eq!(led.present[0].who, Them::Coach(d));
    assert_eq!(led.present.len(), 2);
    // A one-on-one is never reseated.
    let one = Room {
        channel: Channel::InPerson(Place::Office),
        present: vec![Present { who: Them::Coach(c), seat: Seat::Across, speaks: true }],
    };
    assert_eq!(coaching::answerer(&w, &one, &ask(Topic::Recruiting, None)), None);
}
