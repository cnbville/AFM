//! Interviews typed (live talks lane 5 fills it): what a typed line in
//! an interview scene needs beyond the core's box. Staff interviews when
//! you hire (LT18) are the core's own, in `hiring.rs`.

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
