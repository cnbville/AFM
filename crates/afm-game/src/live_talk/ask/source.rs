//! A question for a source on a scout file (live talks lane 10 fills it), and question 5's "Ask a source" row, which needs no key (`scout_source`).

use serde_json::Value;

use crate::{Game, Reply};

/// `ai_ask {kind: "source"}`.
pub fn ask(_g: &mut Game, _args: &Value) -> Reply {
    Err(crate::live_talk::NOT_YET.into())
}

/// Adds this ask's entry to a reply's `ask` list (`live_talk::after`, only
/// with a usable key; `super::push` adds one).
pub fn flag(_g: &Game, _cmd: &str, _v: &mut Value) {}

/// `scout_source`: a source asked with no key (question 5).
pub fn source_cmd(_g: &mut Game, _args: &Value) -> Reply {
    Err(crate::live_talk::NOT_YET.into())
}

/// Question 5's `sources` row on a scout file, which needs no key.
pub fn sources(_g: &Game, _cmd: &str, _v: &mut Value) {}
