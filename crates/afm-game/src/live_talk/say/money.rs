//! The money desks typed (live talks v3): the box on a pro's contract page,
//! on the trade desk and on a recruit's NIL promise. `fo_say {desk, ..,
//! text}` plays your line as a v3 talk with the man across (lane N4's
//! conversation): his agent on his contract (lane N5b's call), the other
//! club's GM (lane N5b's call), or the recruit on the phone
//! (lane N5a's recruit call). The call you're already on today is picked
//! up; else the desk opens one by its own rules (an agent's cold days, the
//! NCAA's contact rules), and a refusal comes back as the box's note. A deal
//! you say is weighed by the desk that prices it (lane N3's `Deals`,
//! `front_office::nfl::judge`, `front_office::deals::evaluate`, the
//! recruit's appeal) and waits on your confirm. There are no chips: what
//! he's open to is on the call's own page as ideas.
//!
//! With no key, or with `Ai.persona_talks` off, the box doesn't show and
//! `fo_say` says why; the desks' own boxes (the offer, the trade builder,
//! the NIL promise) work as ever.

use afm_engine::world::World;
use afm_engine::world::folk::persona::negotiate::{AGENT, GM};
use afm_engine::world::folk::scene::{self, Heard, Kind, Scene};
use serde_json::{Value, json};

use crate::Game;
use crate::live_talk::card::Card;

/// A line's limit in a desk's box: a scene's own.
const LIMIT: usize = 400;

/// Which desk a box is on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Desk {
    /// A pro's contract page: his agent (or him, with none).
    Contract,
    /// The trade desk: another club's GM.
    Trade,
    /// A recruit's NIL promise: a call with him.
    Nil,
}

impl Desk {
    /// The desk named in a box's args: `desk`, else the older `nil` and
    /// `trade` marks, else a contract.
    pub fn of(args: &Value) -> Desk {
        match args.get("desk").and_then(Value::as_str) {
            Some("nil") => Desk::Nil,
            Some("trade") => Desk::Trade,
            Some(_) => Desk::Contract,
            None if args.get("nil").and_then(Value::as_bool) == Some(true) => Desk::Nil,
            None if args.get("trade").and_then(Value::as_bool) == Some(true) => Desk::Trade,
            None => Desk::Contract,
        }
    }

    fn key(self) -> &'static str {
        match self {
            Desk::Contract => "contract",
            Desk::Trade => "trade",
            Desk::Nil => "nil",
        }
    }
}

/// Whether the desks' boxes are on: a usable key and v3 talks.
fn on(g: &Game) -> bool {
    g.ai.persona_talks && g.ai.usable() && g.world.is_some()
}

/// The `say` flag a desk's reply carries.
fn box_flag(desk: Desk, args: Value) -> Value {
    let mut a = args;
    a["desk"] = json!(desk.key());
    json!({ "cmd": "fo_say", "args": a, "limit": LIMIT })
}

/// Adds the desks' boxes to a reply (`live_talk::after`, only with a usable
/// key): the negotiation calls' buttons (lane N5b), and the typed box on
/// his contract page, a club on the trade desk and a recruit's NIL promise.
pub fn flag(g: &Game, cmd: &str, v: &mut Value) {
    crate::live_talk::persona::calls::flag(g, cmd, v);
    if !on(g) || v.get("say").is_some() {
        return;
    }
    let f = match cmd {
        // The same pages that offer the call (`calls::flag`).
        "fo_talk" if !v["kind"].is_null() => {
            v.get("id").cloned().map(|id| box_flag(Desk::Contract, json!({ "id": id })))
        }
        "fo_trade_team" => v["team"]["ref"]["id"].as_u64().map(|club| box_flag(Desk::Trade, json!({ "club": club }))),
        "recruit_file" if v["nil_era"] == json!(true) && v["recruitable"] == json!(true) => {
            v.get("id").cloned().map(|id| box_flag(Desk::Nil, json!({ "id": id })))
        }
        _ => None,
    };
    if let Some(f) = f {
        v["say"] = f;
    }
}

/// Adds what this lane's scenes need to the reader's card (lane 3's typed
/// path; the desks play on v3, so nothing).
pub fn reader(_w: &World, _s: &Scene, _card: &mut Card) {}

/// Reads what this lane's scenes need from the reader's answer onto `Heard`.
pub fn heard(_w: &World, _s: &Scene, _reading: &Value, _heard: &mut Heard) {}

/// Adds what the game released for this lane's scenes to the writer's card.
pub fn writer(_w: &World, _s: &Scene, _heard: &Heard, _card: &mut Card) {}

fn num(args: &Value, k: &str) -> Option<u64> {
    args.get(k).and_then(Value::as_u64)
}

/// The call you're on today with the man across this desk, if it's still
/// going: the agent's call on his contract, the GM's call (about the same
/// man, or about nobody in particular), the recruit's call.
pub fn on_the_line(w: &World, desk: Desk, args: &Value) -> Option<u32> {
    let today = w.today;
    let live = |s: &&Scene| !s.done && s.date == today;
    let id = num(args, "id");
    w.folk
        .scenes
        .iter()
        .rev()
        .filter(live)
        .find(|s| match desk {
            Desk::Contract => s.kind == Kind::Negotiation && s.b == AGENT && Some(s.a) == id,
            Desk::Trade => {
                s.kind == Kind::Negotiation
                    && s.b == GM
                    && Some(s.c.max(0) as u64) == num(args, "club")
                    && id.is_none_or(|x| s.a == x)
            }
            Desk::Nil => s.kind == Kind::RecruitCall && Some(s.a) == id,
        })
        .map(|s| s.id)
}

/// Opens the desk's call by its own command, so its rules hold: an agent's
/// cold days and whether you may talk terms (`talk_agent`), a club you may
/// call (`talk_gm`), the NCAA's contact rules and your day's calls
/// (`recruit_call`).
fn open_call(g: &mut Game, desk: Desk, args: &Value) -> Result<u32, String> {
    let (cmd, a) = match desk {
        Desk::Contract => ("talk_agent", json!({ "id": num(args, "id").ok_or("Missing id.")? })),
        Desk::Trade => {
            let mut a = json!({ "club": num(args, "club").ok_or("Missing club.")? });
            if let Some(id) = num(args, "id") {
                a["id"] = json!(id);
            }
            ("talk_gm", a)
        }
        Desk::Nil => ("recruit_call", json!({ "id": num(args, "id").ok_or("Missing id.")? })),
    };
    let r = g.call(cmd, &a)?;
    let sid = r["scene"].as_u64().ok_or("The call didn't open.")? as u32;
    // Its first lines are on before your line goes in, as they would be
    // had you opened its page first.
    let w = g.world.as_mut().ok_or("No world is loaded.")?;
    scene::open(w, sid);
    Ok(sid)
}

/// `fo_say {desk, id?, club?, text}`: your line to the man across a desk,
/// played on the call you're on with him (or one the desk opens now).
/// Returns `{scene, ticket}`: the screen takes you to the call, where his
/// answer comes. A call the desk won't open today answers `{state: "none",
/// note}` with its reason, and your words stay in the box.
pub fn say(g: &mut Game, args: &Value) -> crate::Reply {
    if !g.ai.usable() {
        return Err("No key is set.".into());
    }
    if !g.ai.persona_talks {
        return Err("Typed lines on the desks need live talks on.".into());
    }
    let text = args.get("text").and_then(Value::as_str).unwrap_or("").trim().to_string();
    if text.is_empty() {
        return Err("Say something first.".into());
    }
    if text.chars().count() > LIMIT {
        return Err(format!("A line is {LIMIT} characters at most."));
    }
    let desk = Desk::of(args);
    let w = g.world.as_ref().ok_or("No world is loaded.")?;
    let sid = match on_the_line(w, desk, args) {
        Some(sid) => sid,
        None => match open_call(g, desk, args) {
            Ok(sid) => sid,
            Err(note) => return Ok(json!({ "state": "none", "note": note })),
        },
    };
    let w = g.world.as_ref().ok_or("No world is loaded.")?;
    let s = w.folk.scenes.iter().find(|s| s.id == sid).ok_or("No such call.")?;
    if !crate::live_talk::persona::talk::plays(g, s) {
        return Err("Typed lines on the desks need live talks on.".into());
    }
    let mut r = super::people_say(g, &json!({ "id": sid, "text": text }))?;
    r["scene"] = json!(sid);
    Ok(r)
}

#[cfg(test)]
#[path = "money_tests.rs"]
mod tests;
