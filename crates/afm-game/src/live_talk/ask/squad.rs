//! Your squad's filter bar asked in words (live talks lane 4 fills it).

use serde_json::Value;

use crate::{Game, Reply};

/// `ai_ask {kind: "squad"}`.
pub fn ask(_g: &mut Game, _args: &Value) -> Reply {
    Err(crate::live_talk::NOT_YET.into())
}

/// Adds this ask's entry to a reply's `ask` list (`live_talk::after`, only
/// with a usable key; `super::push` adds one).
pub fn flag(_g: &Game, _cmd: &str, _v: &mut Value) {}
