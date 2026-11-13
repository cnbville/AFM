//! The ask boxes (live talks, the core registers them): `ai_ask {kind}` for
//! the analyst, your squad, Find, the next opponent, a scout, a report, a
//! source and the leads mail. Each kind's file holds its handler and its
//! flag; the core lists them once here. Every answer is a search or a page
//! the game builds itself: the AI picks the question's terms, never a
//! result.

pub mod analyst;
pub mod find;
pub mod leads;
pub mod opponent;
pub mod report;
pub mod scout;
pub mod source;
pub mod squad;

use serde_json::Value;

use crate::{Game, Reply};

/// The ask kinds, in the order their flags are added.
pub const KINDS: [&str; 8] = ["analyst", "squad", "find", "opponent", "scout", "report", "leads", "source"];

/// `ai_ask {kind, ...}`.
pub fn ask(g: &mut Game, args: &Value) -> Reply {
    match args.get("kind").and_then(Value::as_str).unwrap_or("") {
        "analyst" => analyst::ask(g, args),
        "squad" => squad::ask(g, args),
        "find" => find::ask(g, args),
        "opponent" => opponent::ask(g, args),
        "scout" => scout::ask(g, args),
        "report" => report::ask(g, args),
        "leads" => leads::ask(g, args),
        "source" => source::ask(g, args),
        k => Err(format!("Unknown question kind: {k}")),
    }
}

/// Every ask kind's flag, in `KINDS` order.
pub fn flags(g: &Game, cmd: &str, v: &mut Value) {
    analyst::flag(g, cmd, v);
    squad::flag(g, cmd, v);
    find::flag(g, cmd, v);
    opponent::flag(g, cmd, v);
    scout::flag(g, cmd, v);
    report::flag(g, cmd, v);
    leads::flag(g, cmd, v);
    source::flag(g, cmd, v);
}

/// Adds one entry to a reply's `ask` list (the contract's shape).
pub fn push(v: &mut Value, entry: Value) {
    if !v.is_object() {
        return;
    }
    match v.get_mut("ask").and_then(Value::as_array_mut) {
        Some(list) => list.push(entry),
        None => v["ask"] = Value::Array(vec![entry]),
    }
}
