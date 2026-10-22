//! Lane commands: your front office (docs/final-build/plan.md, lane 10).
//! Each desk answers its own commands and returns None for any other:
//! the NFL desk (contracts, tags, free agency, trades, the 53, the cap),
//! the college desk (board, offers, visits, portal, NIL), the high school
//! seat, and the setup's leftovers.

use serde_json::Value;

use crate::{Game, Reply};

pub mod college;
pub mod nfl;
pub mod school;
pub mod setup;

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    nfl::call(game, cmd, args)
        .or_else(|| college::call(game, cmd, args))
        .or_else(|| school::call(game, cmd, args))
        .or_else(|| setup::call(game, cmd, args))
}
