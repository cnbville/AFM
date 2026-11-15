//! The interview rooms on v3 (section 6.11): the combine interview, the
//! top-30 visit and the official visit's sit-down are timed talks with the
//! prospect (`scene::Kind::CombineTalk`). Typed, each line is played as
//! lane N4's conversation on him (what he knows and tells, the hits, his
//! heat and patience). This module keeps the room's clock: the line's
//! minutes come off it and go to the interview topic the line was about,
//! so the file reads him the way the picked topic would (`own::interview`,
//! `interview_traits`). Staying on a topic is a follow-up: it peels his
//! script off (`coached`) and, on a topic with a worry your scouts flagged,
//! asks that worry straight out (lane 6's rule). When the clock runs out
//! the room is filed as the picked one is.
//!
//! Sizes are Claude's picks, proposed, not confirmed.

use super::super::ear::{Act, Show, Turn};
use super::super::knows::Topic;
use super::super::talk::Said;
use crate::world::World;
use crate::world::folk::scene::Kind;
use crate::world::scouting::own::Topic as Room;

/// The interview topic a question is about, when it is one: his trouble,
/// his family and where he comes from, what football means to him (his
/// character, his work, his football), or none (his injuries are the
/// doctors' read, the rest isn't an interview topic).
pub fn topic_of(t: Topic) -> Option<Room> {
    Some(match t {
        Topic::Case | Topic::RuleBreak => Room::Incident,
        Topic::Life | Topic::Wants | Topic::Money | Topic::Nil | Topic::Grades => Room::Family,
        Topic::Character
        | Topic::Effort
        | Topic::Mood
        | Topic::Goals
        | Topic::Jobs
        | Topic::HisWord
        | Topic::Football
        | Topic::Opponent => Room::Football,
        _ => return None,
    })
}

/// The room's clock after a typed line: its minutes off, on the topic it
/// was about; a follow-up peels his script; a rehearsed answer your read
/// hears is said; out of time, the room is filed.
pub(crate) fn count(w: &mut World, id: u32, said: &Said, t: &Turn) {
    let Some(k) = w.folk.scenes.iter().position(|s| s.id == id && s.kind == Kind::CombineTalk && !s.done) else {
        return;
    };
    let pid = w.folk.scenes[k].a as u32;
    // The last interview topic the line touched: a question's, or his
    // worst tape when you show him film.
    let mut on: Option<Room> = None;
    for a in &said.reading.acts {
        match a {
            Act::Question { topic, about, .. } if about.is_none_or(|x| x == super::super::Them::Player(pid)) => {
                if let Some(r) = topic_of(*topic) {
                    on = Some(r);
                }
            }
            Act::Show(Show::Film) => on = Some(Room::WorstTape),
            _ => {}
        }
    }
    let minutes = t.minutes.ceil().clamp(1.0, 20.0) as u8;
    let before = crate::world::folk::dialogue::spent_on(&w.folk.scenes[k]);
    let mut press = false;
    {
        let s = &mut w.folk.scenes[k];
        s.c -= minutes as i64;
        if let Some(r) = on {
            // Staying on a topic already opened is a follow-up.
            press = before[r as usize] > 0;
            crate::world::folk::dialogue::spend_topic(s, r, minutes);
            if press {
                crate::world::folk::dialogue::ask_flagged(s, r);
            }
        }
    }
    if let Some(r) = on {
        if press {
            super::coached::press(w, pid, r);
        }
        if let Some(i) = crate::world::folk::person(w, pid)
            && let Some(tell) = super::coached::rehearsed(w, i, r)
            && let Some(s) = w.folk.scenes.get_mut(k)
        {
            s.narrate(tell);
        }
    }
    let left = w.folk.scenes[k].c;
    if left <= 0 {
        if let Some(s) = w.folk.scenes.get_mut(k) {
            s.narrate("A knock on the door: time.");
        }
        super::super::talk::end(w, id);
        return;
    }
    if let Some(s) = w.folk.scenes.get_mut(k) {
        s.narrate(format!("{left} minutes left."));
        s.picks.clear();
        crate::world::folk::dialogue::combine_picks(s);
    }
}
