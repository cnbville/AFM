//! A coach asked for his read on a player (Cain 2026-11-16, "take a look
//! at Strand, I want to decide if we should cut him"): a coach on your
//! staff takes it on and answers in the same line from his own read, in
//! words only; nothing is decided and the man stays on your roster.

use super::legacy::seat;
use crate::positions::Position;
use crate::world::coach::Role;
use crate::world::folk::Team;
use crate::world::folk::persona::Them;
use crate::world::folk::persona::ear::{Act, Reading, Resolving};
use crate::world::folk::persona::evaluate::{self, Lean};
use crate::world::folk::persona::office::{self, asks, sit};
use crate::world::folk::persona::talk::{Conversation, Said, store};
use crate::world::folk::persona::terms::Request;
use crate::world::folk::scene;
use crate::world::{Stage, World};

/// A college world with eight receivers on your roster.
fn receivers() -> (World, Vec<u32>) {
    let mut w = seat("college");
    w.add_class_aged(19);
    let p = w.seat.and_then(|s| s.program).unwrap();
    let year = w.year;
    let kids: Vec<usize> = (0..w.people.len()).filter(|&i| w.people[i].stage == Stage::HighSchool).take(8).collect();
    let mut ids = Vec::new();
    for i in kids {
        let q = &mut w.people[i];
        q.stage = Stage::College;
        q.college = Some(p);
        q.class_of = year - 1;
        q.college_seasons = 1;
        q.player.position = Position::WR;
        ids.push(q.player.id);
        w.programs[p].roster.push(i);
    }
    (w, ids)
}

/// A coach on your staff: the receivers coach when there is one.
fn coach(w: &World) -> Option<usize> {
    let wr = (0..w.coaches.len()).find(|&c| {
        asks::on_your_staff(w, c)
            && w.coaches[c].job.is_some_and(|j| matches!(j.role, Role::PositionCoach(_)))
            && w.seat.and_then(|s| s.program).is_some_and(|p| {
                w.programs[p].position_coaches.get(crate::world::coach::Group::of(Position::WR) as usize)
                    == Some(&Some(c))
            })
    });
    wr.or_else(|| (0..w.coaches.len()).find(|&c| asks::on_your_staff(w, c)))
}

#[test]
fn a_coach_asked_for_his_read_gives_it_in_words_and_nothing_is_decided() {
    let (mut w, ids) = receivers();
    let Some(c) = coach(&w) else { return };
    let him = Them::Coach(c);
    let id = ids[0];
    let r = evaluate::read(&w, him, id).expect("your staff reads your own players every week");
    let (place, of) = r.place.expect("he is on your roster");
    assert_eq!(of, 8);
    assert!((1..=8).contains(&place));
    assert!(r.lean.is_some());
    // A starter or a two-deep man is kept; only a man behind it can be let go.
    let two = 2 * crate::game::roster::starters(Position::WR);
    if place <= two {
        assert!(matches!(r.lean, Some(Lean::Starter | Lean::TwoDeep)));
    } else {
        assert!(matches!(r.lean, Some(Lean::Develop | Lean::LetGo)));
    }
    // Words only: the place is the one number, never a rating.
    let words = evaluate::words(&w, him, id);
    assert!(words.len() >= 3);
    let digits: Vec<&String> = words.iter().filter(|s| s.chars().any(|ch| ch.is_ascii_digit())).collect();
    assert_eq!(digits.len(), 1, "{words:?}");
    assert!(digits[0].contains(&format!("of your {of}")));
    // Only a coach on your staff takes it on.
    let boss = crate::world::folk::persona::office::cast_them(
        &w,
        crate::world::folk::cast::CastRef::new(
            Team::seat(&w).unwrap(),
            crate::world::folk::cast::Role::AthleticDirector,
            0,
        ),
    );
    assert!(evaluate::read(&w, boss, id).is_none());

    // In a sit-down, the line is his read, said now; no confirm waits and
    // the man stays.
    let sid = sit::start(&mut w, him, Some(id)).unwrap();
    let s = scene::open(&mut w, sid).unwrap();
    let room = office::room(&w, &s).unwrap();
    store::begin(&mut w, sid, room.clone());
    let reading = Reading { acts: vec![Act::Request(Request::Evaluate(id))], ..Reading::default() };
    let conv = Conversation {
        parts: office::parts(),
        said: Said {
            scene: sid,
            words: "Take a look at him, I want to decide if we should cut him".into(),
            reading,
            promises: Vec::new(),
        },
    };
    let t = conv.turn(&w, &room, &conv.said.reading);
    assert!(t.confirm.is_empty(), "nothing to confirm: {:?}", t.confirm);
    for l in &words {
        assert!(t.say.released.contains(l), "released: {:?}", t.say.released);
    }
    conv.apply(&mut w, &t);
    let p = w.seat.and_then(|s| s.program).unwrap();
    let i = crate::world::folk::person(&w, id).unwrap();
    assert!(w.programs[p].roster.contains(&i), "asking about a cut cuts nobody");
}
