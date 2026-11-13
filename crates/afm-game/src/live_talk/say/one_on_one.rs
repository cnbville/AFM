//! One-on-ones typed (live talks lane 9 fills it, with the engine's
//! hook `talks::one_on_one_say` and `talks::ONE_ON_ONE_TYPED`).

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

/// This lane's own reader rules and schema for its scenes, when the core's
/// generic ones don't fit. None uses the core's.
pub fn rules(_s: &Scene) -> Option<(&'static str, Value)> {
    None
}
