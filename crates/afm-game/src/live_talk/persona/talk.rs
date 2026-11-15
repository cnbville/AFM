//! A typed line in a talk, lane N4's way (two calls a line, agreed
//! 2026-11-14): `people_say {id, text}` builds the ear's card and returns a
//! ticket. When the ear's answer is back, `ai_poll` resolves its tags
//! (`ear::Tags`), the game plays its turn (`talk::Conversation`: what he
//! knows and tells, the hits sized inside today's caps, his asks, the
//! heat, the end) and makes it real, all under the lock. The same ticket
//! then carries the voice's call; his words take the scene's line if they
//! pass the filter, else a short line by how it went and the game's own
//! words for what it released.
//!
//! With no usable key nothing here runs: the box is never shown and the
//! chips play as before. Your players first: the one-on-one is the talk it
//! plays today (`ON`); the other facets join with their lanes.
//!
//! What waits on your confirm (question 4: an offer, a request he agreed
//! to, a decision) is kept by ticket (`Waits`) until you send
//! `talk_confirm {id, ticket, k, yes}`. Never saved: it isn't in the world
//! until you confirm.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use afm_ai::{Failure, Request};
use afm_engine::world::World;
use afm_engine::world::folk;
use afm_engine::world::folk::persona::Them;
use afm_engine::world::folk::persona::deals;
use afm_engine::world::folk::persona::ear::{Confirm, Resolving, Turn};
use afm_engine::world::folk::persona::negotiate::{self, Played};
use afm_engine::world::folk::persona::room::Room;
use afm_engine::world::folk::persona::talk::{self, Conversation, Parts, Said, channels, store};
use afm_engine::world::folk::scene::{Kind, Scene};
use serde_json::{Value, json};

use super::{Resolve, VoiceCard, cards, ear, voice};
use crate::live_talk::tickets::{self, Next, Step};
use crate::{Game, Reply};

/// How long a confirm waits. Proposed, not confirmed.
pub const WAIT_TTL: Duration = Duration::from_secs(900);

/// Whether a scene's typed lines play as a v3 talk: your one-on-ones and
/// the negotiation calls (lane N5b).
pub fn plays(g: &Game, s: &Scene) -> bool {
    g.ai.persona_talks && (s.kind == Kind::OneOnOne || negotiate::is_call(s))
}

/// The room a talk is in: the one it began in, a negotiation's call, or a
/// one-on-one.
fn room_of(w: &World, s: &Scene) -> Room {
    store::room(w, s.id).or_else(|| negotiate::room(w, s)).unwrap_or_else(|| channels::one_on_one(s))
}

/// What waits on your confirm, by ticket.
#[derive(Debug, Default)]
pub struct Waits {
    map: BTreeMap<u64, Waiting>,
}

#[derive(Debug)]
struct Waiting {
    scene: u32,
    him: Them,
    /// Each item, until it's confirmed or dropped.
    items: Vec<Option<Confirm>>,
    born: Instant,
}

impl Waits {
    pub fn prune(&mut self) {
        self.map.retain(|_, w| w.born.elapsed() < WAIT_TTL);
    }
    pub fn len(&self) -> usize {
        self.map.len()
    }
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

/// The parts a talk is played with: lane N1's people and lane N3's deals
/// and requests.
pub fn parts() -> Parts<'static> {
    Parts::today()
}

/// `people_say` on a talk that plays v3: `{id, text}`.
pub fn say(g: &mut Game, id: u32, text: String) -> Reply {
    let w = g.world.as_ref().ok_or("No world is loaded.")?;
    let s = w.folk.scenes.iter().find(|s| s.id == id).ok_or("No such meeting.")?;
    let room = room_of(w, s);
    let played = Played::new(w, s);
    let p = played.parts();
    let (card, keys) = cards::ear(w, s, &room, &p, &text);
    let today = crate::views::date_text(w.today);
    let (y, m, d) = w.today.parts();
    let body = card.text(&format!("{today} ({y:04}-{m:02}-{d:02})"));
    let req = Request::new(ear::EAR, ear::EAR_RULES, &body).schema(ear::schema()).max_tokens(ear::EAR_TOKENS);
    if let Some(f) = g.ai.browser_fake() {
        f.push(ear::EAR, Ok(stand_in(&card)));
    }
    let at = s.lines.len();
    let next = ear_step(id, at, text, keys);
    tickets::start(g, req, next, false).map(|t| json!({ "ticket": t })).map_err(|f| f.note())
}

/// The browser build's stand-in ear (`AFM_AI_FAKE=1`): a question about
/// how he's doing when the line asks something, else nothing touched.
fn stand_in(card: &super::EarCard) -> Value {
    let q = card.line.contains('?');
    let v = json!({
        "acts": if q { json!([{ "act": "question", "topic": "Mood", "about": "", "claim": "none", "value": "",
            "deal": "none", "what": "", "when": "", "words": "" }]) } else { json!([]) },
        "hits": [], "dodged": false, "not_talk": false
    });
    afm_ai::fake::body(&v.to_string(), 900, 60)
}

/// The ear's step, inside `ai_poll` under the lock.
fn ear_step(id: u32, at: usize, words: String, keys: super::EarKeys) -> Next {
    Box::new(move |g, r| {
        let reply = match r {
            Ok(x) => x,
            Err(f) => return Step::off(&f),
        };
        let Some(v) = reply.json else { return Step::off(&Failure::BadReply("no reading".into())) };
        let Some(er) = ear::reply(&v) else { return Step::off(&Failure::BadReply("no reading".into())) };
        let Some(w) = g.world.as_mut() else { return Step::Done(tickets::off("No world is loaded.")) };
        let Some(s) = w.folk.scenes.iter().find(|s| s.id == id).cloned() else {
            return Step::Done(tickets::off("No such meeting."));
        };
        if s.done || s.lines.len() != at {
            return Step::Done(tickets::off(crate::live_talk::say::MOVED_ON));
        }
        let played = Played::new(w, &s);
        // The man the talk is about: the player across, or a call's client,
        // prospect or trade target.
        let pos = match talk::across(&channels::one_on_one(&s)) {
            Some(Them::Player(pid)) => Some(pid),
            _ => negotiate::subject(w, played.facet),
        }
        .and_then(|pid| folk::person(w, pid))
        .map(|i| w.people[i].player.position);
        let tags = ear::Tags { today: w.today, pos };
        let reading = tags.resolve(&er, &keys);
        let promises = tags.promises(&er, &keys);
        let room = match store::room(w, id) {
            Some(r) => r,
            None => {
                let r = room_of(w, &s);
                store::begin(w, id, r.clone());
                r
            }
        };
        let conv =
            Conversation { parts: played.parts(), said: Said { scene: id, words: words.clone(), reading, promises } };
        let mut t = conv.turn(w, &room, &conv.said.reading);
        // A negotiation's own answer, claim and asks (lane N5b).
        let notes = negotiate::said(w, played.facet, &mut t);
        let heard = cards::heard(w, &room, &t);
        let vc = cards::voice(w, &s, &room, &conv.parts, &t, &words, &keys);
        conv.apply(w, &t);
        negotiate::applied(w, played.facet, id, &t, &conv.said.promises);
        let marks = marks(w, &t);
        let confirm = waits(g, id, &room, &t);
        let card_text = vc.text();
        if g.ai.usable() {
            let req = Request::new(voice::VOICE, voice::VOICE_RULES, &card_text)
                .schema(voice::schema())
                .max_tokens(voice::VOICE_TOKENS);
            if let Some(f) = g.ai.browser_fake() {
                let line = vc.released.first().cloned().unwrap_or_else(|| "I hear you, Coach.".into());
                f.push(
                    voice::VOICE,
                    Ok(afm_ai::fake::body(&json!({ "line": line, "colour": "" }).to_string(), 600, 40)),
                );
            }
            return Step::Call(req, voice_step(id, heard, marks, confirm, vc, card_text, t, notes));
        }
        his_line(g, id, &vc, &t, None, &card_text);
        staff_notes(g, id, &notes);
        Step::Done(done(g, id, &heard, marks, confirm))
    })
}

/// The voice's step: his words, or the fallback.
#[allow(clippy::too_many_arguments)]
fn voice_step(
    id: u32,
    heard: String,
    marks: Vec<Value>,
    confirm: Vec<Value>,
    vc: VoiceCard,
    card_text: String,
    t: Turn,
    notes: Vec<String>,
) -> Next {
    Box::new(move |g, r| {
        let said = r.ok().and_then(|x| x.json);
        his_line(g, id, &vc, &t, said.as_ref(), &card_text);
        staff_notes(g, id, &notes);
        Step::Done(done(g, id, &heard, marks, confirm))
    })
}

/// Your staff's notes under his line (a negotiation's: your read of a
/// claim, your cap, the call back to come).
fn staff_notes(g: &mut Game, id: u32, notes: &[String]) {
    let Some(w) = g.world.as_mut() else { return };
    if let Some(s) = w.folk.scenes.iter_mut().find(|s| s.id == id) {
        for n in notes {
            s.narrate(folk::plain(n));
        }
    }
}

/// His line on the scene: the voice's if it passed the filter, else the
/// fallback with the game's own words for what it released.
fn his_line(g: &mut Game, id: u32, vc: &VoiceCard, t: &Turn, said: Option<&Value>, card_text: &str) {
    let Some(w) = g.world.as_mut() else { return };
    let tell = Some(t.say.tell.clone()).filter(|x| !x.is_empty());
    match said.and_then(|v| voice::read(v, card_text)) {
        Some((line, colour)) => talk::reply(w, id, &line, tell, colour),
        None => {
            talk::reply(w, id, &voice::fallback(vc, t), tell, None);
            if let Some(s) = w.folk.scenes.iter_mut().find(|s| s.id == id) {
                for r in &t.say.released {
                    s.narrate(folk::plain(r));
                }
            }
        }
    }
}

/// Each claim you made, as the game marks it.
fn marks(w: &World, t: &Turn) -> Vec<Value> {
    t.claims
        .iter()
        .filter_map(|(f, truth)| {
            let mark = truth.mark()?;
            Some(json!({ "text": format!("{}: {}", f.kind.replace('_', " "), f.value), "mark": mark }))
        })
        .chain(t.promised.iter().filter_map(|k| {
            let p = w.folk.promises.get(*k as usize)?;
            Some(json!({ "text": format!("promise: {}", p.kind.words()), "mark": "later" }))
        }))
        .collect()
}

/// Keeps what waits on your confirm and lists it for the screen.
fn waits(g: &mut Game, id: u32, room: &Room, t: &Turn) -> Vec<Value> {
    if t.confirm.is_empty() {
        return Vec::new();
    }
    let Some(him) = talk::across(room) else { return Vec::new() };
    let ticket = tickets::polling(g).unwrap_or(0);
    let f = g.world.as_ref().and_then(|w| w.folk.scenes.iter().find(|s| s.id == id).map(|s| (w, s)));
    let words: Vec<String> = t
        .confirm
        .iter()
        .map(|c| match (c, f) {
            (Confirm::Offer(o), Some((w, s))) => {
                negotiate::confirm_words(w, negotiate::facet(w, s), o).unwrap_or_else(|| confirm_words(c))
            }
            _ => confirm_words(c),
        })
        .collect();
    let list: Vec<Value> = words
        .iter()
        .enumerate()
        .map(|(k, text)| json!({ "cmd": "talk_confirm", "args": { "id": id, "ticket": ticket, "k": k }, "text": text }))
        .collect();
    g.ai.persona.map.insert(
        ticket,
        Waiting { scene: id, him, items: t.confirm.iter().cloned().map(Some).collect(), born: Instant::now() },
    );
    list
}

fn confirm_words(c: &Confirm) -> String {
    match c {
        Confirm::Offer(o) => format!("Make the offer: {}", folk::plain(&o.words)),
        Confirm::Request(r) => format!("Go ahead: {r:?}"),
        Confirm::Decision(d) => format!("Make it so: {}", channels::decision_words(*d)),
    }
}

/// `talk_confirm {id, ticket, k, yes}`: runs what you confirmed by the
/// game's own rules, or drops it.
pub fn confirm(g: &mut Game, args: &Value) -> Reply {
    let id = args.get("id").and_then(Value::as_u64).ok_or("Missing id.")? as u32;
    let ticket = args.get("ticket").and_then(Value::as_u64).ok_or("Missing ticket.")?;
    let k = args.get("k").and_then(Value::as_u64).ok_or("Missing k.")? as usize;
    let yes = args.get("yes").and_then(Value::as_bool).unwrap_or(true);
    let wt = g.ai.persona.map.get_mut(&ticket).ok_or("That answer is gone.")?;
    if wt.scene != id {
        return Err("That isn't on the table.".into());
    }
    let him = wt.him;
    // Each item is confirmed or dropped once; the rest stay.
    let item = wt.items.get_mut(k).and_then(Option::take).ok_or("That isn't on the table.")?;
    if wt.items.iter().all(Option::is_none) {
        g.ai.persona.map.remove(&ticket);
    }
    let w = g.world.as_mut().ok_or("No world is loaded.")?;
    if yes {
        let lines = run(w, id, him, &item);
        if let Some(s) = w.folk.scenes.iter_mut().find(|s| s.id == id) {
            for l in lines {
                s.narrate(l);
            }
        }
    }
    g.run("people_scene", &json!({ "id": id }))
}

/// Runs one confirmed item: a request through `Acting::run`, an offer
/// through its desk (a negotiation's, lane N5b) or lane N3's `agree`, a
/// cut through the desk and any other decision on its own screen until
/// its lane wires it.
fn run(w: &mut World, scene: u32, him: Them, c: &Confirm) -> Vec<String> {
    let played = match w.folk.scenes.iter().find(|s| s.id == scene) {
        Some(s) => Played::new(w, s),
        None => Played::of(negotiate::Facet::None),
    };
    match c {
        Confirm::Request(r) => {
            let ran = played.parts().acting.run(w, him, r);
            ran.done
        }
        Confirm::Offer(o) => {
            if let Some(lines) = negotiate::make_real(w, played.facet, him, o) {
                return lines;
            }
            // Lane N3 makes it real: its promises through P1, his lean,
            // and the deal kept with your words.
            let a = deals::agree(w, &talk::DEALS, him, o);
            let mut out = vec![format!("Agreed: {}.", folk::plain(&o.words))];
            out.extend(a.done);
            out
        }
        Confirm::Decision(d) => negotiate::decide(w, him, *d)
            .unwrap_or_else(|| vec![format!("Make it real on its own screen: {}.", channels::decision_words(*d))]),
    }
}

/// The poll's done answer: "Heard as", the marks, what waits on your
/// confirm, and the scene's normal reply.
fn done(g: &mut Game, id: u32, heard: &str, marks: Vec<Value>, confirm: Vec<Value>) -> Value {
    let reply = g.run("people_scene", &json!({ "id": id })).unwrap_or(Value::Null);
    let mut v = json!({ "state": "done", "heard": heard, "marks": marks, "reply": reply });
    if !confirm.is_empty() {
        v["confirm"] = Value::Array(confirm);
    }
    v
}

#[cfg(test)]
#[path = "talk_tests.rs"]
mod tests;
