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
use afm_engine::world::folk::persona::room::Room;
use afm_engine::world::folk::persona::talk::{self, Conversation, Parts, Said, channels, store};
use afm_engine::world::folk::scene::{Kind, Scene};
use serde_json::{Value, json};

use super::{Resolve, VoiceCard, cards, ear, voice};
use crate::live_talk::tickets::{self, Next, Step};
use crate::{Game, Reply};

/// How long a confirm waits. Proposed, not confirmed.
pub const WAIT_TTL: Duration = Duration::from_secs(900);

/// Whether a scene's typed lines play as a v3 talk: your one-on-ones, and
/// the draft side's rooms (lane N5e: a combine interview, the draft talks).
pub fn plays(g: &Game, s: &Scene) -> bool {
    g.ai.persona_talks && matches!(s.kind, Kind::OneOnOne | Kind::CombineTalk | Kind::DraftTalk)
}

/// What waits on your confirm, by ticket, and how each typed line was
/// heard, so a page drawn again (a reload, another screen and back) still
/// shows both. Neither is saved: a confirm is not in the world until you
/// make it, and "Heard as" is the screen's note on the line.
#[derive(Debug, Default)]
pub struct Waits {
    map: BTreeMap<u64, Waiting>,
    said: BTreeMap<u32, Vec<Heard>>,
}

/// One typed line's "Heard as" and marks.
#[derive(Debug, Clone)]
struct Heard {
    words: String,
    heard: String,
    marks: Vec<Value>,
}

/// The most scenes whose lines are remembered. Proposed, not confirmed.
const KEEP_SCENES: usize = 64;

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
    /// What waits on your confirm in a scene, as the page draws it.
    pub fn pending(&self, scene: u32) -> Vec<Value> {
        self.map
            .iter()
            .filter(|(_, w)| w.scene == scene)
            .flat_map(|(ticket, w)| {
                w.items.iter().enumerate().filter_map(move |(k, c)| {
                    let c = c.as_ref()?;
                    Some(json!({ "cmd": "talk_confirm", "args": { "id": w.scene, "ticket": ticket, "k": k },
                        "text": confirm_words(c) }))
                })
            })
            .collect()
    }
    fn remember(&mut self, scene: u32, words: &str, heard: &str, marks: &[Value]) {
        if self.said.len() >= KEEP_SCENES && !self.said.contains_key(&scene) {
            self.said.pop_first();
        }
        self.said.entry(scene).or_default().push(Heard {
            words: words.to_string(),
            heard: heard.to_string(),
            marks: marks.to_vec(),
        });
    }
    /// How your typed lines in a scene were heard, in the order you said them.
    pub fn heard_lines(&self, scene: u32) -> Vec<(String, String, Vec<Value>)> {
        self.said
            .get(&scene)
            .map(|v| v.iter().map(|h| (h.words.clone(), h.heard.clone(), h.marks.clone())).collect())
            .unwrap_or_default()
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

/// The parts a scene's talk is played with: the draft side's rooms on lane
/// N5e's facet, every other talk on `parts`.
pub fn parts_for(kind: Kind) -> Parts<'static> {
    match kind {
        Kind::CombineTalk | Kind::DraftTalk => afm_engine::world::folk::persona::draft::facet::parts(),
        _ => parts(),
    }
}

/// `people_say` on a talk that plays v3: `{id, text}`.
pub fn say(g: &mut Game, id: u32, text: String) -> Reply {
    let w = g.world.as_ref().ok_or("No world is loaded.")?;
    let s = w.folk.scenes.iter().find(|s| s.id == id).ok_or("No such meeting.")?;
    let room = store::room(w, id).unwrap_or_else(|| channels::one_on_one(s));
    let p = parts_for(s.kind);
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

/// The browser build's stand-in ear (`AFM_AI_FAKE=1`): a line with a
/// question mark is a question about how he's doing, one that says
/// "promise" a promise of a start by midseason, one that says "bench" a
/// decision to bench him (it waits on your confirm); anything else touches
/// nothing. It lets the page's marks and confirm cards be checked in the
/// browser with no key.
fn stand_in(card: &super::EarCard) -> Value {
    let line = card.line.to_lowercase();
    let mut acts = Vec::new();
    let act = |kind: &str, what: &str, topic: &str, when: &str| {
        json!({ "act": kind, "topic": topic, "about": "", "claim": "none", "value": "",
            "deal": "none", "what": what, "when": when, "words": "" })
    };
    if line.contains('?') {
        acts.push(act("question", "", "Mood", ""));
    }
    if line.contains("promise") {
        acts.push(act("promise", "start", "none", "midseason"));
    }
    if line.contains("bench") {
        acts.push(act("decision", "benched", "none", ""));
    }
    let v = json!({ "acts": acts, "hits": [], "dodged": false, "not_talk": false });
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
        let pos = match talk::across(&channels::one_on_one(&s)) {
            Some(Them::Player(pid)) => folk::person(w, pid).map(|i| w.people[i].player.position),
            _ => None,
        };
        let tags = ear::Tags { today: w.today, pos };
        let reading = tags.resolve(&er, &keys);
        let promises = tags.promises(&er, &keys);
        let room = match store::room(w, id) {
            Some(r) => r,
            None => {
                let r = channels::one_on_one(&s);
                store::begin(w, id, r.clone());
                r
            }
        };
        let conv = Conversation {
            parts: parts_for(s.kind),
            said: Said { scene: id, words: words.clone(), reading, promises },
        };
        let t = conv.turn(w, &room, &conv.said.reading);
        let heard = cards::heard(w, &room, &t);
        let vc = cards::voice(w, &s, &room, &conv.parts, &t, &words, &keys);
        conv.apply(w, &t);
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
            return Step::Call(req, voice_step(id, Outcome { words, heard, marks, confirm }, vc, card_text, t));
        }
        his_line(g, id, &vc, &t, None, &card_text);
        Step::Done(done(g, id, Outcome { words, heard, marks, confirm }))
    })
}

/// The voice's step: his words, or the fallback.
fn voice_step(id: u32, out: Outcome, vc: VoiceCard, card_text: String, t: Turn) -> Next {
    Box::new(move |g, r| {
        let said = r.ok().and_then(|x| x.json);
        his_line(g, id, &vc, &t, said.as_ref(), &card_text);
        Step::Done(done(g, id, out))
    })
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
    let list: Vec<Value> = t
        .confirm
        .iter()
        .enumerate()
        .map(|(k, c)| {
            json!({ "cmd": "talk_confirm", "args": { "id": id, "ticket": ticket, "k": k }, "text": confirm_words(c) })
        })
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
        let kind = w.folk.scenes.iter().find(|s| s.id == id).map_or(Kind::OneOnOne, |s| s.kind);
        let lines = run(w, him, &item, kind);
        if let Some(s) = w.folk.scenes.iter_mut().find(|s| s.id == id) {
            for l in lines {
                s.narrate(l);
            }
        }
    }
    g.run("people_scene", &json!({ "id": id }))
}

/// Runs one confirmed item: a request through `Acting::run`, an offer
/// through lane N3's `agree`, a decision
/// on its own screen until its lane wires it.
fn run(w: &mut World, him: Them, c: &Confirm, kind: Kind) -> Vec<String> {
    match c {
        Confirm::Request(r) => {
            let ran = parts_for(kind).acting.run(w, him, r);
            ran.done
        }
        Confirm::Offer(o) => {
            // Lane N3 makes it real: its promises through P1, his lean,
            // and the deal kept with your words.
            let a = deals::agree(w, &talk::DEALS, him, o);
            let mut out = vec![format!("Agreed: {}.", folk::plain(&o.words))];
            out.extend(a.done);
            out
        }
        Confirm::Decision(d) => {
            vec![format!("Make it real on its own screen: {}.", channels::decision_words(*d))]
        }
    }
}

/// What a line came to: your words, how they were heard, the marks and what
/// waits on your confirm.
struct Outcome {
    words: String,
    heard: String,
    marks: Vec<Value>,
    confirm: Vec<Value>,
}

/// The poll's done answer: "Heard as", the marks, what waits on your
/// confirm, and the scene's normal reply.
fn done(g: &mut Game, id: u32, out: Outcome) -> Value {
    let Outcome { words, heard, marks, confirm } = out;
    g.ai.persona.remember(id, &words, &heard, &marks);
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
