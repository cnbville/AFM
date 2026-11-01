//! Update 2, lane 9C: the draft room (board V9). At an NFL club on draft
//! night: your board in tiers with the chance each man is still there at
//! your pick, his public rank, your staff's mock of the picks before yours,
//! his file in tabs, the trade calls priced on your chart, your needs and
//! the picks so far. At a college or a high school: your players' draft
//! outlook. Every read is your staff's (ranges and words), never a true
//! number; potential stays hidden.

pub mod outlook;
pub mod room;
pub mod sim;

use serde_json::Value;

use crate::{Game, Reply};

const COMMANDS: [&str; 2] = ["dr_room", "dr_outlook"];

pub fn call(game: &mut Game, cmd: &str, _args: &Value) -> Option<Reply> {
    if !COMMANDS.contains(&cmd) {
        return None;
    }
    let Some(w) = game.world.as_ref() else { return Some(Err("No world is loaded.".into())) };
    Some(match cmd {
        "dr_room" => room::view(w),
        _ => outlook::view(w),
    })
}
