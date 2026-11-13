//! Contract talks and trade calls typed (live talks lane 7 fills it):
//! `fo_say` with any desk but "nil", and question 5's fact chips, which
//! need no key (`fo_fact`).

use afm_engine::world::World;
use afm_engine::world::folk::scene::{Heard, Scene};
use serde_json::Value;

use crate::Game;
use crate::live_talk::card::Card;

/// Adds this lane's own box to a reply (`live_talk::after`, only with a
/// usable key). A scene's talk box is the core's: this is for what the
/// scene box doesn't cover.
pub fn flag(_g: &Game, _cmd: &str, _v: &mut Value) {}

/// Adds what this lane's scenes need to the reader's card.
pub fn reader(_w: &World, _s: &Scene, _card: &mut Card) {}

/// Reads what this lane's scenes need from the reader's answer onto `Heard`.
pub fn heard(_w: &World, _s: &Scene, _reading: &Value, _heard: &mut Heard) {}

/// Adds what the game released for this lane's scenes to the writer's card.
pub fn writer(_w: &World, _s: &Scene, _heard: &Heard, _card: &mut Card) {}

/// `fo_say` on a contract talk or a trade call.
pub fn say(_g: &mut Game, _args: &Value) -> crate::Reply {
    Err(crate::live_talk::NOT_YET.into())
}

/// `fo_fact`: a fact chip on a contract talk, a trade call or a NIL form.
pub fn fact(_g: &mut Game, _args: &Value) -> crate::Reply {
    Err(crate::live_talk::NOT_YET.into())
}

/// Question 5's fact chips (`facts`), which need no key: added to the
/// talk's reply by `live_talk::after` before the key is looked at.
pub fn facts(_g: &Game, _cmd: &str, _v: &mut Value) {}
