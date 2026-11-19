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
use afm_engine::world::folk::persona::crowd;
use afm_engine::world::folk::persona::gameday::group;
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
    // A room (group talks): its men with how your last line landed with
    // each, its own count, your team's mark, the room's history with you,
    // and ideas you'd say to a room.
    if crowd::plays(&room) && group::is_group(s) {
        room_block(w, s, &room, &mut t);
        v["talk"] = t;
        heard_lines(g, s.id, v);
        return;
    }
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
    // Your last line read as a goodbye: the page asks whether to wrap up.
    if g.ai.persona.wrap_asked(s.id) && !s.done {
        t["wrap_ask"] = json!(true);
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

/// The page's block for a room (group talks): every man in it with how your
/// last line landed with him (item 1's own takes, kept on the open talk),
/// the room's count as the talk counts it, your team's mark in place of a
/// face, what's at stake between you and the room, and ideas for a room.
fn room_block(
    w: &World,
    s: &afm_engine::world::folk::scene::Scene,
    room: &afm_engine::world::folk::persona::room::Room,
    t: &mut Value,
) {
    use afm_engine::world::folk::persona::room::Seat;
    use afm_engine::world::folk::persona::talk::{channels, store};
    let open = store::open(w, s.id);
    let takes = open.map(|o| o.takes.clone()).unwrap_or_default();
    let leads = crowd::leaders(w, Some(s), room);
    let men: Vec<Them> = crowd::listeners(room).collect();
    let mut rows: Vec<(Them, f32, Option<&'static str>, bool)> = men
        .iter()
        .map(|&x| {
            let tk = takes.iter().find(|k| k.0 == x);
            let tag = tk.and_then(|k| crowd::tag(k.1, k.2));
            (x, tk.map_or(0.0, |k| k.1), tag, leads.contains(&x))
        })
        .collect();
    // The leaders first, then the men the line moved most.
    rows.sort_by(|a, b| b.3.cmp(&a.3).then(b.1.abs().total_cmp(&a.1.abs())));
    let mut people: Vec<Value> =
        vec![json!({ "lead": true, "name": s.title.clone(), "ini": "", "tag": "", "line": "" })];
    for (x, v, tag, lead) in rows {
        let name = channels::name(w, x);
        let ini: String = name.split_whitespace().filter_map(|c| c.chars().next()).take(2).collect();
        let pos = match x {
            Them::Player(id) => folk::person(w, id).map(|i| format!("{:?}", w.people[i].player.position)),
            _ => None,
        };
        let line = [pos, lead.then(|| "Leader".to_string())].into_iter().flatten().collect::<Vec<_>>().join(" · ");
        let kind = if v > 0.01 {
            "good"
        } else if v < -0.01 {
            "bad"
        } else {
            ""
        };
        people.push(
            json!({ "name": name, "ini": ini, "tag": tag.unwrap_or("Listening"), "tagKind": kind, "line": line }),
        );
    }
    // Your staff at the front of a position room.
    for p in room.present.iter().filter(|p| p.seat == Seat::YourStaff) {
        let name = channels::name(w, p.who);
        let ini: String = name.split_whitespace().filter_map(|c| c.chars().next()).take(2).collect();
        people.push(json!({ "name": name, "ini": ini, "tag": "Your staff", "line": "" }));
    }
    t["room"] = json!(people);
    t["roomN"] = json!(men.len());
    if let Some(team) = afm_engine::world::folk::Team::seat(w) {
        use crate::views::{TeamRef, team_colors, team_name};
        let r = match team {
            afm_engine::world::folk::Team::College(p) => Some(TeamRef::College(p)),
            afm_engine::world::folk::Team::Nfl(c) => Some(TeamRef::Nfl(c)),
            afm_engine::world::folk::Team::High(h) => Some(TeamRef::HighSchool(h)),
            afm_engine::world::folk::Team::Other(_) => None,
        };
        if let Some(r) = r {
            t["team"] = json!({ "name": team_name(w, r), "colors": team_colors(w, r), "ref": r.json() });
        }
    }
    let about = afm_engine::world::folk::persona::talk::between::room(w);
    if !about.is_empty() {
        let mut left = t.get("left").and_then(Value::as_array).cloned().unwrap_or_default();
        left.insert(0, json!({ "k": "text", "label": "You and the room", "paras": about }));
        t["left"] = json!(left);
    }
    if t.get("ideas").is_none() {
        t["ideas"] = json!(room_ideas(w, room, &leads));
    }
}

/// What you could say to a room: to the room, to its groups, to its
/// leader by name. Never one man's own business in front of everyone.
pub fn room_ideas(w: &World, room: &afm_engine::world::folk::persona::room::Room, leads: &[Them]) -> Vec<String> {
    let lvl = crowd::level(w);
    let blocs = crowd::blocs(w, room);
    let has = |b: crowd::Bloc| blocs.contains(&b);
    let mut out: Vec<String> = Vec::new();
    if has(crowd::Bloc::Seniors) {
        out.push(format!("{}, this is your team now. I need you to lead it.", cap(&crowd::Bloc::Seniors.words(lvl))));
    }
    if has(crowd::Bloc::Starters) {
        out.push("Every job in this room is open. Nobody plays on what he did last year.".into());
    }
    if has(crowd::Bloc::Defense) {
        out.push("Defense, I want you flying to the ball on every snap.".into());
    }
    if let Some(Them::Player(id)) = leads.first()
        && let Some(i) = folk::person(w, *id)
    {
        let first = w.people[i].name.split_whitespace().next().unwrap_or("").to_string();
        if !first.is_empty() {
            out.push(format!("{first}, where's this room at?"));
        }
    }
    out.push("Anyone have a problem with how we're going to do things?".into());
    out.push("Any questions for me?".into());
    out.truncate(MAX_IDEAS);
    out
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}
