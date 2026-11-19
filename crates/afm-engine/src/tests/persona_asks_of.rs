//! An ask put to someone else in the room (ear audit item 4,
//! `Reading::asks_of`): the man you said it to weighs it, answers it and
//! does it; the man across only hears you ask, and is never asked himself.
//! One who isn't in the room is asked nothing.

use super::legacy::seat;
use crate::player::PlayingTime;
use crate::world::folk::persona::Them;
use crate::world::folk::persona::ear::{Reading, Resolving};
use crate::world::folk::persona::fake::Fake;
use crate::world::folk::persona::room::{Present, Seat};
use crate::world::folk::persona::talk::{self, Conversation, Parts, Said, asked_of_words, channels, store};
use crate::world::folk::persona::terms::Request;
use crate::world::folk::persona::wants::{Ran, Willing};
use crate::world::folk::talks::{self, Subject};
use crate::world::folk::{Team, facts};
use crate::world::{Stage, World};

/// Your college program with one starter on it, a talk with him open, and
/// one of your assistants.
fn talk_with_staff() -> (World, u32, u32, usize) {
    let mut w = seat("college");
    let p = w.seat.and_then(|s| s.program).unwrap();
    w.add_class_aged(19);
    let year = w.year;
    let i = (0..w.people.len()).find(|&i| w.people[i].stage == Stage::HighSchool).unwrap();
    let q = &mut w.people[i];
    q.stage = Stage::College;
    q.college = Some(p);
    q.class_of = year - 2;
    q.college_seasons = 2;
    q.last_role = Some(PlayingTime::Starter);
    w.programs[p].roster.push(i);
    let id = w.people[i].player.id;
    let me = w.career.as_ref().unwrap().coach;
    let team = Team::seat(&w).unwrap();
    let c = facts::assistants(&w, team).into_iter().map(|x| x.0).find(|&c| c != me).expect("an assistant");
    let sid = talks::start_talk(&mut w, id, Subject::Role).unwrap();
    (w, id, sid, c)
}

#[test]
fn an_ask_to_someone_else_in_the_room_is_his_to_answer_and_do() {
    let (mut w, id, sid, c) = talk_with_staff();
    let s = talk::scene(&w, sid).cloned().unwrap();
    let mut room = channels::room_for(&w, &s);
    room.present.push(Present { who: Them::Coach(c), seat: Seat::YourStaff, speaks: false });
    store::begin(&mut w, sid, room.clone());
    let fake = Fake {
        willing: Willing::Yes,
        ran: Ran { done: vec!["He'll stay after practice with him.".into()], ..Ran::default() },
        ..Fake::default()
    };
    let mut parts = Parts::today();
    parts.acting = &fake;
    let ask = Request::WorkWith(id);
    let reading = Reading { asks_of: vec![(Them::Coach(c), ask.clone())], ..Reading::default() };
    let conv = Conversation {
        parts,
        said: Said { scene: sid, words: "Coach, stay after practice with him.".into(), reading, promises: Vec::new() },
    };
    let t = conv.turn(&w, &room, &conv.said.reading);
    // Your assistant weighs it; the man across is asked nothing.
    assert_eq!(t.asked_of, vec![(Them::Coach(c), ask.clone(), Willing::Yes)]);
    assert!(t.asked.is_empty(), "{:?}", t.asked);
    assert!(t.confirm.is_empty(), "an ask that costs you nothing needs no confirm");
    assert!(t.say.heard.iter().any(|h| h.starts_with("He hears you ask ")), "{:?}", t.say.heard);
    assert!(!t.greeting, "an ask is no greeting");
    conv.apply(&mut w, &t);
    let name = crate::world::folk::plain(&channels::name(&w, Them::Coach(c)));
    let said: Vec<String> = talk::scene(&w, sid).unwrap().lines.iter().map(|l| l.text.clone()).collect();
    assert!(said.iter().any(|l| *l == asked_of_words(&name, &Willing::Yes)), "{said:?}");
    assert!(said.iter().any(|l| l == "He'll stay after practice with him."), "it ran: {said:?}");
    // An ask that costs you waits on your yes, named for him.
    let scout = Request::FindOut(id, crate::world::folk::persona::knows::Topic::Effort);
    let reading = Reading { asks_of: vec![(Them::Coach(c), scout.clone())], ..Reading::default() };
    let conv = Conversation { parts, said: Said { scene: sid, words: String::new(), reading, promises: Vec::new() } };
    let t = conv.turn(&w, &room, &conv.said.reading);
    assert!(
        t.confirm.iter().any(|x| matches!(x, crate::world::folk::persona::ear::Confirm::RequestOf(d, r) if *d == Them::Coach(c) && *r == scout)),
        "{:?}",
        t.confirm
    );
    // A man who isn't in the room is asked nothing.
    let away = channels::room_for(&w, &s);
    let reading = Reading { asks_of: vec![(Them::Coach(c), ask)], ..Reading::default() };
    let conv = Conversation { parts, said: Said { scene: sid, words: String::new(), reading, promises: Vec::new() } };
    let t = conv.turn(&w, &away, &conv.said.reading);
    assert!(t.asked_of.is_empty() && t.asked.is_empty());
}

#[test]
fn his_answer_reads_by_how_he_took_it() {
    assert_eq!(asked_of_words("Gil Foster", &Willing::Yes), "Gil Foster will do it.");
    assert_eq!(asked_of_words("Gil Foster", &Willing::No), "Gil Foster won't do that.");
    assert!(asked_of_words("Gil Foster", &Willing::If(Default::default())).contains("wants something back"));
}
