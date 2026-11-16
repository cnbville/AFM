//! What the conversation page draws beside a v3 talk (lane N6's `talk`
//! block, filled by the game): that the talk plays on the ear and the
//! voice, how you reach him, and the Ideas list that takes the moves list's
//! place when a key is usable (question 7: no answer buttons, a list of
//! what is on his mind and what you could say, a click puts the words in
//! your box to change).
//!
//! Ideas hold only what you know: the points on his fact sheet your staff
//! has seen (`talk_points::chips`), your own promises to him, one close
//! friend and his family. Never a want by name, never a number. Other lanes
//! add their own fields to the same `talk` object; this merges, it never
//! replaces what is there.

use afm_engine::world::World;
use afm_engine::world::folk::persona::Them;
use afm_engine::world::folk::persona::room::{Channel, Group};
use afm_engine::world::folk::persona::talk::{across, stakes};
use afm_engine::world::folk::{self as folk, claims, talk_points as tp, ties};
use serde_json::{Value, json};

use super::talk::plays;
use crate::Game;

/// The most ideas shown. Proposed, not confirmed.
pub const MAX_IDEAS: usize = 6;

/// A scene reply that plays as a v3 talk gets its `talk` block.
pub fn flag(g: &Game, v: &mut Value) {
    let Some(id) = v.get("id").and_then(Value::as_u64) else { return };
    let Some(w) = g.world.as_ref() else { return };
    let Some(s) = w.folk.scenes.iter().find(|s| u64::from(s.id) == id) else { return };
    if v.get("say").is_none() || !plays(g, s) {
        return;
    }
    let room = super::talk::room_of(w, s);
    let mut t = v.get("talk").filter(|t| t.is_object()).cloned().unwrap_or_else(|| json!({}));
    t["persona"] = json!(true);
    t["channel"] = json!(channel_words(room.channel));
    // What the talk is about, as a coach in your seat would know it: his
    // standing with you, his deal, the portal, the draft (`stakes::situation`).
    if let Some(him) = across(&room) {
        let about = stakes::situation(w, s, him);
        if !about.is_empty() {
            let mut left = t.get("left").and_then(Value::as_array).cloned().unwrap_or_default();
            left.insert(0, json!({ "k": "text", "label": "What's at stake", "paras": about }));
            t["left"] = json!(left);
        }
    }
    if let Some(Them::Player(pid)) = across(&room) {
        let ideas = ideas(w, pid);
        if !ideas.is_empty() && t.get("ideas").is_none() {
            t["ideas"] = json!(ideas);
        }
    }
    // The talk ended on a meeting that still holds a decision (`talk::wrap`):
    // no box, the meeting's own picks decide it.
    if afm_engine::world::folk::persona::talk::store::is_over(w, s.id) {
        t["over"] = json!(true);
    }
    // What waits on your say-so stays on the page until you answer it.
    let pending = g.ai.persona.pending(s.id);
    if !pending.is_empty() {
        t["confirm"] = json!(pending);
    }
    v["talk"] = t;
    heard_lines(g, s.id, v);
}

/// Each typed line of yours carries how it was heard, so a page drawn again
/// still shows "Heard as" and the marks (the lines come in the order you
/// said them).
fn heard_lines(g: &Game, scene: u32, v: &mut Value) {
    let mut said = g.ai.persona.heard_lines(scene);
    let Some(lines) = v.get_mut("lines").and_then(Value::as_array_mut) else { return };
    for l in lines.iter_mut().filter(|l| l["by"] == 1) {
        let Some(typed) = l["typed"].as_str().map(str::to_string) else { continue };
        let Some(at) = said.iter().position(|(w, _, _)| folk::plain(w) == typed) else { continue };
        let (_, heard, marks) = said.remove(at);
        l["heard"] = json!(heard);
        l["marks"] = json!(marks);
    }
}

/// How you reach him, for the page's header.
pub fn channel_words(c: Channel) -> &'static str {
    match c {
        Channel::InPerson(_) => "In person",
        Channel::Phone => "Phone call",
        Channel::Text => "Text",
        Channel::Video => "Video call",
        Channel::Through(_) => "Through someone else",
        Channel::Letter => "Letter",
        Channel::Group(Group::Team) => "The whole team",
        Channel::Group(_) => "A group",
    }
}

/// A point's name inside a sentence: "His slump" reads "his slump", a
/// name keeps its capital.
fn inline(name: &str) -> String {
    for lead in ["His ", "The ", "Your ", "A ", "Losing ", "Winning ", "Missing ", "Wanting ", "Coming "] {
        if let Some(rest) = name.strip_prefix(lead) {
            return format!("{}{rest}", lead.to_lowercase());
        }
    }
    name.to_string()
}

/// What you could say, from what you know. Fixed order: your word to him,
/// what is on his mind, a friend, his family.
pub fn ideas(w: &World, pid: u32) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let Some(i) = folk::person(w, pid) else { return out };
    // Your promises to him: a broken one first, then the open ones.
    if let Some(to) = Them::Player(pid).promise_to() {
        let mine = || w.folk.promises.iter().filter(move |p| p.to == to);
        for p in mine().filter(|p| p.kept == Some(false)).take(1) {
            out.push(format!("I let you down on {}. I own that.", folk::plain(&p.kind.words())));
        }
        for p in mine().filter(|p| p.kept.is_none()).take(2) {
            out.push(format!("About my word to you: {}. I haven't forgotten it.", folk::plain(&p.kind.words())));
        }
    }
    let sheet = claims::player(w, i);
    for p in tp::chips(&sheet) {
        if matches!(p.kind, claims::Kind::PromiseOpen | claims::Kind::PromiseBroken) {
            continue;
        }
        let name = folk::plain(&tp::chip_name(p));
        out.push(format!("Where's your head on {}?", inline(&name)));
    }
    if let Some(j) = ties::friends(w, pid).into_iter().find_map(|f| folk::person(w, f)) {
        let name = w.people[j].name.split_whitespace().next().unwrap_or("").to_string();
        if !name.is_empty() {
            out.push(format!("How is {name} doing? You two are close."));
        }
    }
    out.push("How are things at home?".into());
    out.truncate(MAX_IDEAS);
    out
}
