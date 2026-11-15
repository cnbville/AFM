//! Live talks v3's shared contract (`world::folk::persona`): the stores
//! save and load, an empty store leaves a save as it was, and every trait
//! has a fake. No behaviour is tested here: there is none yet.

use crate::calendar::Date;
use crate::world::folk::claims::{Live, Stated};
use crate::world::folk::persona::Them;
use crate::world::folk::persona::ear::{Act, ActKind, Hit, How, Item, Reading, Resolving, Turn};
use crate::world::folk::persona::fake::Fake;
use crate::world::folk::persona::knows::{Answer, Knows, TellParts, Tells, Told, Topic};
use crate::world::folk::persona::memory::{Colour, HisLie, Line, Memory, Saved, Unseen};
use crate::world::folk::persona::room::{Channel, Place, Present, Room, Seat};
use crate::world::folk::persona::terms::{Bound, Measure, Offer, Request, Scope, Term, Valuing, Weighed};
use crate::world::folk::persona::wants::{Acting, Kind, Wanting, Willing};
use crate::world::folk::rules::RuleKind;
use crate::world::{World, WorldOptions};

fn named<T: serde::Serialize>(x: &T) -> Vec<u8> {
    rmp_serde::to_vec_named(x).unwrap()
}

fn term() -> Term {
    Term {
        measure: Measure { key: "kicker.fg_line".into(), who: None },
        number: 37.0,
        bound: Bound::AtMost,
        scope: Scope::Game,
        words: "you kick from the 37 in".into(),
        ..Default::default()
    }
}

#[test]
fn empty_memory_leaves_a_save_as_it_was() {
    let live = Live::default();
    assert!(live.is_empty());
    // An empty Live writes an empty map: no `persona` key.
    assert_eq!(named(&live), named(&std::collections::BTreeMap::<String, u8>::new()));
}

#[test]
fn memory_round_trips_in_live() {
    let me = Them::Player(7);
    let mut live = Live::default();
    let reading = Reading {
        acts: vec![
            Act::Question { topic: Topic::Mood, about: Some(Them::Player(9)), words: "how's Tre".into() },
            Act::Promise(term()),
            Act::Claim(Stated { kind: "role".into(), about: Some(7), value: "starter".into() }),
            Act::Request(Request::Mentor(9)),
        ],
        hits: vec![Hit { on: me, item: Item::Friend(9), how: How::Reassured }],
        ..Default::default()
    };
    live.persona = Memory {
        his_lies: vec![HisLie {
            by: me,
            topic: Topic::Nil,
            about: None,
            said: Date(100),
            shows: Some(Date(140)),
            words: "Georgia's at 200".into(),
            caught: false,
        }],
        notebook: vec![Colour { who: me, day: Date(100), note: "his dog is called Bo".into() }],
        unseen: vec![Unseen { player: 9, rule: RuleKind::Curfew, day: Date(90), there: vec![7], found: false }],
        talks: vec![Saved {
            with: me,
            day: Date(100),
            lines: vec![Line { you: "hey".into(), reading, his: "coach".into() }],
        }],
        ..Default::default()
    };
    assert!(!live.is_empty());
    let back: Live = rmp_serde::from_slice(&named(&live)).unwrap();
    assert_eq!(back.persona, live.persona);
    assert_eq!(back.persona.talks[0].lines[0].reading.acts[1].kind(), ActKind::Promise);
}

#[test]
fn every_trait_has_a_fake() {
    let mut w = World::empty(WorldOptions { scale: 0.001, history_years: 0, start_year: 2037, ..Default::default() });
    let him = Them::Player(1);
    let f = Fake {
        told: Told { answer: Answer::Partly, released: vec!["he's fine".into()], ..Default::default() },
        willing: Willing::Yes,
        weighed: Weighed::Lowball,
        ..Default::default()
    };
    assert!(f.knows(&w, him, Topic::Mood, None).is_empty());
    assert_eq!(f.tell(&w, him, &[], &TellParts::default()).answer, Answer::Partly);
    assert_eq!(f.kind(&w, him), Kind::Player);
    assert!(f.wants(&w, him).is_empty());
    assert_eq!(f.willing(&w, him, &Request::KeepQuiet), Willing::Yes);
    let _ = f.run(&mut w, him, &Request::KeepQuiet);
    assert_eq!(f.weigh(&w, him, &Offer::default()), Weighed::Lowball);
    let room = Room {
        channel: Channel::InPerson(Place::Home),
        present: vec![Present { who: him, seat: Seat::Across, speaks: true }],
    };
    let t: Turn = f.turn(&w, &room, &Reading::default());
    assert!(t.told.is_empty());
    f.apply(&mut w, &t);
}
