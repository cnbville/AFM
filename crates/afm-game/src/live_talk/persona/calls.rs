//! Live talks v3, lane N5b: the negotiation calls you make from a desk.
//! `talk_agent {id}` (a pro's agent on his contract, from his contract
//! page), `talk_gm {club, id?}` (another club's GM, from the trade desk)
//! and `talk_rush {id}` (a prospect left in the undrafted rush, from the
//! draft room) each open a call (`folk::persona::negotiate`) and return
//! `{scene}`; the call plays as a v3 talk. The buttons show only with a
//! usable key (`flag`), since a call is typed lines.

use afm_engine::world::folk::persona::negotiate;
use serde_json::{Value, json};

use crate::{Game, Reply};

fn need(g: &Game) -> Result<(), String> {
    if !g.ai.persona_talks || !g.ai.usable() {
        return Err("Calls like this need live talks on.".into());
    }
    Ok(())
}

fn id(args: &Value) -> Result<u32, String> {
    args.get("id").and_then(Value::as_u64).map(|x| x as u32).ok_or_else(|| "Missing id.".into())
}

/// The calls' commands.
pub fn call(g: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    let r = match cmd {
        "talk_agent" | "talk_gm" | "talk_rush" => need(g).and_then(|_| {
            let w = g.world.as_mut().ok_or("No world is loaded.")?;
            match cmd {
                "talk_agent" => negotiate::call_agent(w, id(args)?),
                "talk_rush" => negotiate::call_rush(w, id(args)?),
                _ => {
                    let club = args.get("club").and_then(Value::as_u64).ok_or("Missing club.")? as usize;
                    negotiate::call_gm(w, club, args.get("id").and_then(Value::as_u64).map(|x| x as u32))
                }
            }
        }),
        _ => return None,
    };
    Some(r.map(|scene| json!({ "scene": scene })))
}

/// The call buttons on the desks' replies (only with a usable key): his
/// contract page, a club on the trade desk, the draft room once clubs call
/// undrafted players.
pub fn flag(g: &Game, cmd: &str, v: &mut Value) {
    if !g.ai.persona_talks || g.world.is_none() {
        return;
    }
    let call = match cmd {
        "fo_talk" if !v["kind"].is_null() => {
            v.get("id").cloned().map(|id| json!({ "cmd": "talk_agent", "args": { "id": id } }))
        }
        "fo_trade_team" => {
            v["team"]["ref"]["id"].as_u64().map(|club| json!({ "cmd": "talk_gm", "args": { "club": club } }))
        }
        "draft_live" if v["live"] == json!(true) && (v["rush"] == json!(true) || v["night"].as_u64() >= Some(2)) => {
            Some(json!({ "cmd": "talk_rush", "args": {} }))
        }
        _ => None,
    };
    if let Some(c) = call {
        v["call"] = c;
    }
}

#[cfg(test)]
#[path = "calls_tests.rs"]
mod tests;
