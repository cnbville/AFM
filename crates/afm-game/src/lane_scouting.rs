//! Lane commands: scouting (docs/final-build/plan.md). `call` answers the
//! commands this lane owns and returns None for any other.

use serde_json::Value;

use crate::{Game, Reply};

pub fn call(_game: &mut Game, _cmd: &str, _args: &Value) -> Option<Reply> {
    None
}
