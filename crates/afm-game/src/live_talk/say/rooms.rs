//! The pregame and halftime room typed (live talks lane 5 or 6 fills it):
//! `talk_typed`, since the room is not a scene. The core registers the
//! command and this stub.

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

/// `talk_typed`: a typed line in the pregame or halftime room.
pub fn typed(_g: &mut Game, _args: &Value) -> crate::Reply {
    Err(crate::live_talk::NOT_YET.into())
}
