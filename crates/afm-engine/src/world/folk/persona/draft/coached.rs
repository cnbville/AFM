//! Coached interview answers (question 10: "makes interviews honest
//! work"). A prospect's agency and his training camp prep him for the
//! stock questions every club asks (tell me about your family, what
//! happened that night, why football): real since the 1990s, when the
//! agencies began paying for combine training with interview coaching in
//! it. A coached answer reads better than the truth on those topics. You
//! get past it by staying on the topic: each follow-up peels half of the
//! script off, and a sharp read hears that it's rehearsed.
//!
//! Sizes are Claude's picks, proposed, not confirmed.

use crate::calendar::Date;
use crate::player::Trait;
use crate::world::scouting::own::Topic;
use crate::world::season::key_uniform;
use crate::world::{Stage, World};

/// How far you pressed past his script on a topic in one room.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Pressed {
    pub id: u32,
    pub date: Date,
    pub topic: Topic,
    pub n: u8,
}

/// A fully coached answer's lift on its traits, in trait points toward the
/// good side. Proposed, not confirmed.
pub const SHADE: f64 = 1.5;
/// How much of the script one follow-up takes off. Proposed, not
/// confirmed.
pub const PEEL: f64 = 0.5;

/// How much of each topic the prep covers: the stock questions fully, his
/// worst tape half (agents rehearse "what I learned from it"), the
/// whiteboard not at all (you can't rehearse football you don't know).
pub fn scripted(t: Topic) -> f64 {
    match t {
        Topic::Family | Topic::Incident | Topic::Football => 1.0,
        Topic::WorstTape => 0.5,
        Topic::Whiteboard => 0.0,
    }
}

/// How much interview coaching he got, 0-1: a declared prospect from his
/// projected stock (the giants sign and prep the top of the class), a
/// winter at a testing camp, his family's money and how seriously he takes
/// it; a recruit on an official visit gets little.
pub fn prep(w: &World, i: usize) -> f64 {
    let p = &w.people[i];
    let prof = p.player.personality.traits.z(Trait::Professionalism);
    let money = p.bg.money.clamp(0.0, 1.0);
    match p.stage {
        Stage::Declared => {
            let id = p.player.id;
            let top = crate::world::scouting::public::consensus(w).is_some_and(|v| v.contains(&id)) as u8 as f64;
            let camp = w.scouting.predraft.has(id, crate::world::scouting::predraft::TESTING_CAMP) as u8 as f64;
            (0.35 + 0.25 * top + 0.15 * camp + 0.1 * money + 0.05 * prof).clamp(0.0, 0.85)
        }
        _ => (0.05 + 0.15 * money).clamp(0.0, 0.3),
    }
}

/// How far you got past his script on a topic in the room that day.
pub fn peeled(w: &World, id: u32, date: Date, t: Topic) -> u8 {
    w.folk.live.persona.draft.pressed.iter().filter(|x| x.id == id && x.date == date && x.topic == t).map(|x| x.n).sum()
}

/// The coached lift on a topic's traits for the room on `date`, in trait
/// points toward the good side: the script times how much of the topic it
/// covers, less half for each follow-up.
pub fn shade(w: &World, id: u32, date: Date, t: Topic) -> f64 {
    let Some(i) = crate::world::scouting::place(w, id) else { return 0.0 };
    SHADE * prep(w, i) * scripted(t) * PEEL.powi(peeled(w, id, date, t) as i32)
}

/// You stayed on a topic: one more layer of the script comes off.
pub fn press(w: &mut World, id: u32, t: Topic) {
    let date = w.today;
    let b = &mut w.folk.live.persona.draft;
    match b.pressed.iter_mut().find(|x| x.id == id && x.date == date && x.topic == t) {
        Some(x) => x.n = x.n.saturating_add(1).min(4),
        None => b.pressed.push(Pressed { id, date, topic: t, n: 1 }),
    }
}

/// Whether your read hears that an answer is rehearsed: more often the
/// more coached he is and the sharper your read of people (keyed to him,
/// the topic and the day, so the same answer reads the same).
pub fn rehearsed(w: &World, i: usize, t: Topic) -> Option<&'static str> {
    let p = &w.people[i];
    let cover = scripted(t) * PEEL.powi(peeled(w, p.player.id, w.today, t) as i32);
    let prep = prep(w, i) * cover;
    if prep < 0.2 {
        return None;
    }
    let sharp = crate::world::folk::talks::sharpness(w).min(1.0);
    let odds = prep * (0.2 + 0.6 * sharp);
    let u = key_uniform(0xC0AC ^ (t as u64) << 32 ^ (w.today.0 as u64) << 8, p.player.id as u64);
    (u < odds).then_some("It sounds rehearsed: the same words he'd give anyone.")
}
