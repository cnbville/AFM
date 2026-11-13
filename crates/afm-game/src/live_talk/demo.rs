//! `ai_demo`: every state of the contract with set content, on the fake only,
//! so the screens (lane 2) can be built and seen before the core exists.
//!
//! `ai_demo {}` lists the states and the flags a reply carries when a key is
//! set. `ai_demo {"state": "unsure"}` gives a ticket that answers "wait" a
//! few times and then that state's answer, exactly as `ai_poll` would.

use serde_json::{Value, json};

/// The states `ai_demo` can play, in the order a screen shows them.
pub const STATES: [&str; 11] =
    ["wait", "unsure", "done", "none", "off", "cap", "table", "text", "reload", "checks", "ask_first"];

/// How many polls answer "wait" before the state's answer.
pub const WAITS: u32 = 2;

/// The flags `live_talk::after` adds to a reply when a key is set and the cap
/// is not reached (the contract in `live-talks.md`).
pub fn flags() -> Value {
    json!({
        "say": { "cmd": "people_say", "args": { "id": 12 }, "limit": 400 },
        "ask": [
            { "cmd": "ai_ask", "args": { "kind": "analyst" }, "limit": 400 },
            { "cmd": "ai_ask", "args": { "kind": "report", "player": 88 }, "button": "Write the report" },
            { "cmd": "ai_ask", "args": { "kind": "source", "player": 88 }, "limit": 400,
              "choose": { "key": "source", "options": [
                  { "value": 5, "label": "His high school coach" },
                  { "value": 6, "label": "A scout who watched him" } ] } },
            { "cmd": "ai_ask", "args": { "kind": "leads", "mail": 31 }, "auto": true }
        ],
        "facts": { "cmd": "fo_fact", "args": { "id": 12 },
                   "list": [ { "fact": 2, "label": "His backup's numbers are better" },
                             { "fact": 3, "label": "He was arrested last spring" } ] },
        "sources": { "cmd": "scout_source", "args": { "player": 88 },
                     "list": [ { "source": 5, "label": "His high school coach" } ],
                     "topics": [ { "topic": 1, "label": "His character" } ] },
    })
}

/// The answer a state gives, as `ai_poll` would.
pub fn answer(state: &str) -> Option<Value> {
    Some(match state {
        "wait" => return None,
        "unsure" => json!({
            "state": "unsure",
            "readings": [
                { "pick": 3, "heard": "a promise of more snaps, warm" },
                { "pick": 4, "heard": "straight talk about his role, calm" }
            ]
        }),
        "ask_first" => json!({
            "state": "unsure",
            "readings": [ { "pick": 505, "heard": "a one-game suspension, firm" } ]
        }),
        "done" => json!({
            "state": "done",
            "heard": "straight talk about his role, calm",
            "marks": [
                { "text": "third downs are yours", "mark": "true" },
                { "text": "you have started thirty games", "mark": "false" },
                { "text": "the backup's EPA is higher", "mark": "priced" },
                { "text": "you will start on Saturday", "mark": "later" }
            ],
            "reply": { "demo": true, "text": "Third downs? I've started thirty games here, Coach." }
        }),
        "none" => json!({ "state": "none", "note": "He doesn't follow." }),
        "off" => json!({ "state": "off", "note": "The AI took too long." }),
        "cap" => json!({ "state": "off", "note": "This month's $50 cap is reached." }),
        "table" => json!({
            "state": "done",
            "table": {
                "columns": ["Player", "Pos", "Yards per route run"],
                "rows": [["Marcus Hill", "WR", "2.9"], ["Devin Shaw", "WR", "2.6"]]
            },
            "line": "Two slot receivers stand out on yards per route run.",
            "query": [ { "field": "pos", "value": "WR" } ]
        }),
        "text" => json!({ "state": "done", "text": "He reads coverage early and wins at the top of his routes." }),
        "reload" => json!({ "state": "done", "reload": true }),
        "checks" => json!({
            "state": "done",
            "rows": [
                { "name": "Staff interview questions", "score": 28, "of": 30, "bar": 27 },
                { "name": "Exploit lines", "score": 40, "of": 40, "bar": 40 }
            ],
            "cost": 0.0105,
            "seconds": 31.4
        }),
        _ => return None,
    })
}
