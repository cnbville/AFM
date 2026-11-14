//! Interviews typed (live talks lane 6, LT11): the combine interview, the
//! top-30 visit and the official visit's sit-down (`Kind::CombineTalk`).
//! A typed question lands through the core's generic reader on one of the
//! room's own options: a topic (961-965), pressing him (967), a worry your
//! scouts flagged (970 and up, one a flagged trait) or wrapping up (969).
//! What the room gains is the engine's: a typed question takes minutes by
//! its length (`dialogue::typed_minutes`, from `Heard.len`), and a flagged
//! worry asked straight out reads that trait closer (`own::WORRY_SD`).
//! Staff interviews when you hire (LT18) are the core's own, in `hiring.rs`.

use afm_engine::world::World;
use afm_engine::world::folk::scene::{Heard, Kind, Scene};
use afm_engine::world::scouting::file::{TRAIT_NAMES, TRAITS};
use serde_json::Value;

use crate::Game;
use crate::live_talk::card::Card;

/// The first flagged worry's code (`dialogue.rs`, `combine_picks`).
pub const WORRY: u32 = 970;

/// Adds this lane's own box to a reply (`live_talk::after`, only with a
/// usable key). The room's talk box is the core's.
pub fn flag(_g: &Game, _cmd: &str, _v: &mut Value) {}

/// The reader's card: the room's clock, so a long question reads as one.
pub fn reader(_w: &World, s: &Scene, card: &mut Card) {
    if s.kind == Kind::CombineTalk {
        card.facts.push(format!("The room has {} minutes left.", s.c.max(0)));
    }
}

/// Nothing beyond the generic reading: the option carries the topic or the
/// worry, and the length rides on `Heard.len`.
pub fn heard(_w: &World, _s: &Scene, _reading: &Value, _heard: &mut Heard) {}

/// The writer's card: a worry asked straight out is named, so his answer
/// speaks to it; the answer itself is the game's line.
pub fn writer(_w: &World, s: &Scene, heard: &Heard, card: &mut Card) {
    if s.kind != Kind::CombineTalk {
        return;
    }
    if let Some(k) = heard.pick.checked_sub(WORRY).map(|k| k as usize).filter(|k| *k < TRAITS) {
        card.released.push(format!(
            "The coach asked him straight about what the scouts flagged: {}.",
            TRAIT_NAMES[k].to_lowercase()
        ));
    }
}

#[cfg(test)]
#[path = "gain_tests.rs"]
mod tests;
