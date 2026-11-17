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
use afm_engine::world::folk::persona::ear::{Confirm, Ends, Resolving, Turn};
use afm_engine::world::folk::persona::gameday::{decide, group};
use afm_engine::world::folk::persona::negotiate::{self, Played};
use afm_engine::world::folk::persona::room::Room;
use afm_engine::world::folk::persona::talk::{self, Conversation, Parts, Said, channels, store};
use afm_engine::world::folk::persona::terms::{Decision, Request as Ask, Valuing, Weighed};
use afm_engine::world::folk::persona::{asks, deals, meetings, office};
use afm_engine::world::folk::scene::{Kind, Scene};
use serde_json::{Value, json};

use super::{Resolve, VoiceCard, cards, ear, voice};
use crate::live_talk::tickets::{self, Next, Step};
use crate::{Game, Reply};

/// How long a confirm waits. Proposed, not confirmed.
pub const WAIT_TTL: Duration = Duration::from_secs(900);

/// Whether a scene's typed lines play as a v3 talk: your one-on-ones, the
/// meetings with the people in your building (lane N5c), the draft side's
/// talks (lane N5e), the negotiation calls (lane N5b), a recruit's and a
/// transfer's talks and a college staff's call to you, the high school
/// coach (lane N5a), the group talks (the team talk, the first team
/// meeting, the council: lane N5d), and the older meetings (a parent's
/// call, discipline, the captains, the parents, a call from someone you
/// know, your job interview, the interview rooms...: `meetings`) when
/// someone is across.
pub fn plays(g: &Game, s: &Scene) -> bool {
    if meetings::plays(s.kind) {
        return g.ai.persona_talks && g.world.as_ref().is_some_and(|w| meetings::has_room(w, s));
    }
    g.ai.persona_talks
        && (matches!(s.kind, Kind::OneOnOne | Kind::DraftTalk)
            || office::plays(s.kind)
            || negotiate::is_call(s)
            || talk::recruit::is_recruit(s.kind)
            || talk::hs_call::is_call(s.kind)
            || group::is_group(s))
}

/// One of the older meetings with nobody across to talk to (a briefing
/// with no staff): it takes no typed line, so none is read onto a chip.
pub fn nobody(g: &Game, s: &Scene) -> bool {
    g.ai.persona_talks && meetings::plays(s.kind) && g.world.as_ref().is_some_and(|w| !meetings::has_room(w, s))
}

/// The room a scene's talk is held in: the open talk's, else the one its
/// lane sets (someone in your building, lane N5c; a negotiation's call,
/// lane N5b; a group's room, lane N5d), else a one-on-one.
pub fn room_of(w: &World, s: &Scene) -> Room {
    store::room(w, s.id)
        .or_else(|| office::plays(s.kind).then(|| office::room(w, s)).flatten())
        .or_else(|| negotiate::room(w, s))
        .or_else(|| meetings::room(w, s))
        .or_else(|| group::room(w, s))
        .unwrap_or_else(|| channels::room_for(w, s))
}

/// The parts a scene's talk is played with: lane N5c's for the people in
/// your building, lane N5e's for the draft side, else lane N1's people and
/// lane N3's deals and requests under a negotiation's own (lane N5b,
/// `negotiate::Played`).
pub fn parts_for<'a>(s: &Scene, played: &'a Played) -> Parts<'a> {
    parts_for_kind(s.kind, played)
}

/// `parts_for` by the scene's kind.
fn parts_for_kind(kind: Kind, played: &Played) -> Parts<'_> {
    if office::plays(kind) || meetings::plays(kind) || kind == Kind::DraftTalk {
        parts_of(kind)
    } else {
        played.parts()
    }
}

/// The lanes' own parts by the scene's kind (lanes N5c and N5e), else lane
/// N1's people and lane N3's deals and requests. The older meetings play on
/// lane N5c's office parts (lane N1's people with what your staff, your
/// boss and other coaches know and do; for anyone else they are lane N1's
/// and N3's own), and the interview rooms on lane N5e's.
pub fn parts_of(kind: Kind) -> Parts<'static> {
    if kind == Kind::CombineTalk {
        afm_engine::world::folk::persona::draft::facet::parts()
    } else if office::plays(kind) || meetings::plays(kind) {
        office::parts()
    } else if kind == Kind::DraftTalk {
        afm_engine::world::folk::persona::draft::facet::parts()
    } else {
        parts()
    }
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
    /// Each item's line as first drawn, read against the world then (a
    /// redraw has no world to read).
    words: Vec<String>,
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
                    let text = w.words.get(k).cloned().unwrap_or_else(|| confirm_words(c));
                    Some(json!({ "cmd": "talk_confirm", "args": { "id": w.scene, "ticket": ticket, "k": k },
                        "text": text }))
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

/// `people_say` on a talk that plays v3: `{id, text}`.
pub fn say(g: &mut Game, id: u32, text: String) -> Reply {
    let w = g.world.as_ref().ok_or("No world is loaded.")?;
    let s = w.folk.scenes.iter().find(|s| s.id == id).ok_or("No such meeting.")?;
    let room = room_of(w, s);
    let played = Played::new(w, s);
    let p = parts_for(s, &played);
    let (card, keys) =
        if group::is_group(s) { super::group::ear(w, s, &room, &text) } else { cards::ear(w, s, &room, &p, &text) };
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
    // The manner (coach voice): a "!" or capitals is hot, a curse
    // profane, and a curse with "you" in it aimed at the man.
    let swore = ["fuck", "shit", "damn", "hell"].iter().any(|x| line.contains(x));
    let hot = card.line.contains('!') || card.line.chars().filter(|c| c.is_ascii_uppercase()).count() > 8;
    let manner = json!({
        "heat": if hot { "hot" } else { "even" },
        "language": if swore { "profane" } else { "clean" },
        "aim": if swore && line.contains("you") { "man" } else { "out" },
        "care": "neutral",
        "say": if line.contains('?') { "ask" } else { "tell" },
    });
    let v = json!({ "acts": acts, "hits": [], "dodged": false, "not_talk": false, "manner": manner });
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
        let room = match store::room(w, id) {
            Some(r) => r,
            None => {
                let r = room_of(w, &s);
                store::begin(w, id, r.clone());
                r
            }
        };
        let played = Played::new(w, &s);
        // The man the talk is about: the player across, or a call's client,
        // prospect or trade target.
        let pos = match talk::across(&room) {
            Some(Them::Player(pid)) => Some(pid),
            _ => negotiate::subject(w, played.facet),
        }
        .and_then(|pid| folk::person(w, pid))
        .map(|i| w.people[i].player.position);
        let tags = ear::Tags { today: w.today, pos };
        let reading = ear::keep_open(tags.resolve(&er, &keys), &words);
        let reading =
            if matches!(talk::across(&room), Some(Them::Player(_))) { ear::own_work(reading) } else { reading };
        let promises = tags.promises(&er, &keys);
        let is_group = group::is_group(&s);
        // A line to the whole room is one hit on each man in it (lane N5d).
        let reading = if is_group { group::expand(&room, reading) } else { reading };
        let conv = Conversation {
            parts: parts_for(&s, &played),
            said: Said { scene: id, words: words.clone(), reading, promises },
        };
        let mut t = conv.turn(w, &room, &conv.said.reading);
        // A negotiation's own answer, claim and asks (lane N5b).
        let notes = negotiate::said(w, played.facet, &mut t);
        let heard = cards::heard(w, &room, &t);
        let mut vc = cards::voice(w, &s, &room, &conv.parts, &t, &words, &keys);
        if is_group {
            // A captain answers for the room; the voice hears the room, not
            // a roll call.
            vc.room = vec![format!("{}, every one of them listening", cards::channel_words(room.channel))];
        }
        conv.apply(w, &t);
        negotiate::applied(w, played.facet, id, &t, &conv.said.promises);
        if is_group
            && let Some(line) = group::landed(w, &t.hits, talk::across(&room))
            && let Some(sc) = w.folk.scenes.iter_mut().find(|x| x.id == id)
        {
            sc.narrate(line);
        }
        // What lane N5c makes of it: a reference's read on the coach's page,
        // a boss's no kept on record (the older meetings' bosses too).
        if meetings::plays(s.kind) {
            meetings::after(w, id, &room, &conv.said.reading, &t);
        }
        if office::plays(s.kind) || meetings::plays(s.kind) {
            let lines = office::after(w, id, &room, &conv.said.reading, &t);
            if let Some(sc) = w.folk.scenes.iter_mut().find(|x| x.id == id) {
                for l in lines {
                    sc.narrate(l);
                }
            }
        }
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
            return Step::Call(req, voice_step(id, Outcome { words, heard, marks, confirm }, vc, card_text, t, notes));
        }
        his_line(g, id, &vc, &t, None, &card_text);
        staff_notes(g, id, &notes);
        Step::Done(done(g, id, Outcome { words, heard, marks, confirm }))
    })
}

/// The voice's step: his words, or the fallback.
fn voice_step(id: u32, out: Outcome, vc: VoiceCard, card_text: String, t: Turn, notes: Vec<String>) -> Next {
    Box::new(move |g, r| {
        let said = r.ok().and_then(|x| x.json);
        his_line(g, id, &vc, &t, said.as_ref(), &card_text);
        staff_notes(g, id, &notes);
        Step::Done(done(g, id, out))
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
    // Who answered: his parents doing the talking, his agent on the money.
    let by = t.speaker.map(|(x, _)| x);
    // What he asked earlier and let go stays let go: the card says so, and
    // the voice still pressed it on every line after (`voice::let_go`).
    let read = said.and_then(|v| voice::read(v, card_text));
    let read = match read {
        Some((line, colour)) if !vc.asked_before.is_empty() && !vc.pressing => {
            voice::let_go(&line).map(|line| (line, colour))
        }
        x => x,
    };
    // His last line when time is up asks nothing new: the call ends right
    // after it, with no line of yours to answer it.
    let read = match read {
        Some((line, colour)) if matches!(t.ends, Some(Ends::Time)) => Some((voice::no_new_question(&line), colour)),
        x => x,
    };
    match read {
        Some((line, colour)) => talk::reply(w, id, &line, by, tell, colour),
        None => {
            talk::reply(w, id, &voice::fallback(vc, t), by, tell, None);
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
            (Confirm::Request(r @ Ask::Money(_)), Some((w, _))) => {
                format!("Go ahead: {}", asks::request_words_in(w, r))
            }
            // The man it's on, by name (Cain 2026-11-16: "his coach is let
            // go" on the card for letting the coordinator himself go).
            (Confirm::Decision(d), Some((w, _)))
                if !matches!(d, Decision::CoachLetGo) || matches!(him, Them::Coach(_)) =>
            {
                format!("Make it so: {}", channels::decision_named(*d, &folk::plain(&channels::name(w, him))))
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
        Waiting { scene: id, him, items: t.confirm.iter().cloned().map(Some).collect(), words, born: Instant::now() },
    );
    list
}

fn confirm_words(c: &Confirm) -> String {
    match c {
        Confirm::Request(Ask::Other(x)) if x == talk::recruit::VISIT => "Set his official visit".into(),
        Confirm::Offer(o) => format!("Make the offer: {}", folk::plain(&o.words)),
        Confirm::Request(r) => format!("Go ahead: {}", asks::request_words(r)),
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
        let lines = office::confirm(w, id, him, &item).unwrap_or_else(|| run(w, id, him, &item));
        if let Some(s) = w.folk.scenes.iter_mut().find(|s| s.id == id) {
            for l in lines {
                s.narrate(l);
            }
        }
    }
    g.run("people_scene", &json!({ "id": id }))
}

/// What a line to a talk that is over gets: the meeting waits on your
/// decision.
pub const OVER: &str = "The talk is over. What's left is your call.";

/// `talk_wrap`: `{id}` ends the talk the way a goodbye does, with no cost
/// for leaving (thread "Let talking end meetings cleanly"): a one-on-one
/// lands as its end chip does, a meeting closes with what you didn't argue
/// standing, and one that holds a decision keeps it as its own picks
/// (`talk::wrap`). Only on a talk that plays v3.
pub fn wrap(g: &mut Game, args: &Value) -> Reply {
    let id = args.get("id").and_then(Value::as_u64).ok_or("Missing id.")? as u32;
    let w = g.world.as_ref().ok_or("No world is loaded.")?;
    let s = w.folk.scenes.iter().find(|s| s.id == id).ok_or("No such meeting.")?;
    if s.done {
        return Err("That talk is over.".into());
    }
    if !plays(g, s) {
        return Err("This talk ends on its own picks.".into());
    }
    let w = g.world.as_mut().ok_or("No world is loaded.")?;
    talk::end(w, id);
    g.run("people_scene", &json!({ "id": id }))
}

/// Runs one confirmed item: a request through `Acting::run`, an offer
/// through its desk (a negotiation's, lane N5b; a recruit's, lane N5a) or
/// lane N3's `agree`, a cut through the desk and any other decision on its
/// own screen until its lane wires it.
fn run(w: &mut World, scene: u32, him: Them, c: &Confirm) -> Vec<String> {
    let played = match w.folk.scenes.iter().find(|s| s.id == scene) {
        Some(s) => Played::new(w, s),
        None => Played::of(negotiate::Facet::None),
    };
    let kind = w.folk.scenes.iter().find(|s| s.id == scene).map_or(Kind::OneOnOne, |s| s.kind);
    let recruit = w.folk.scenes.iter().find(|s| s.id == scene).cloned().filter(|s| talk::recruit::is_recruit(s.kind));
    match c {
        // A recruit's visit: the desk sets it on its first open date, and
        // he may still say no.
        Confirm::Request(Ask::Other(x)) if x == talk::recruit::VISIT => {
            vec![recruit.map_or_else(|| "That talk is over.".into(), |s| talk::recruit::set_visit(w, &s))]
        }
        // A recruit's offer: weighed again as you confirm (nothing moved
        // in between). Taking time, it's left with his family and they
        // answer on their day; a yes is made real on the recruiting desk.
        Confirm::Offer(o) if recruit.is_some() => match talk::DEALS.weigh(w, him, o) {
            Weighed::Later(by) => {
                deals::offered(w, him, o, &Weighed::Later(by));
                vec![format!("You leave it with them: {}. Their answer comes by {by}.", folk::plain(&o.words))]
            }
            Weighed::Yes => {
                let a = deals::agree(w, &talk::DEALS, him, o);
                let mut out = vec![format!("Agreed: {}.", folk::plain(&o.words))];
                out.extend(a.done.iter().cloned());
                out.extend(talk::recruit::real(w, him, &a));
                out
            }
            _ => vec!["They've thought better of it: it's off the table.".into()],
        },
        Confirm::Request(r) => {
            let ran = parts_for_kind(kind, &played).acting.run(w, him, r);
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
        Confirm::Decision(d) => decide::run(w, him, *d)
            .or_else(|| negotiate::decide(w, him, *d))
            .unwrap_or_else(|| vec![format!("Make it real on its own screen: {}.", channels::decision_words(*d))]),
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

#[cfg(test)]
#[path = "recruit_tests.rs"]
mod recruit_tests;

#[cfg(test)]
#[path = "future_tests.rs"]
mod future_tests;

#[cfg(test)]
#[path = "n7_tests.rs"]
pub(crate) mod n7_tests;

#[cfg(test)]
#[path = "stake_tests.rs"]
mod stake_tests;

#[cfg(test)]
#[path = "meetings_tests.rs"]
mod meetings_tests;

#[cfg(test)]
#[path = "weighed_tests.rs"]
mod weighed_tests;

#[cfg(test)]
#[path = "wrap_tests.rs"]
mod wrap_tests;

#[cfg(test)]
#[path = "goals_tests.rs"]
mod goals_tests;

#[cfg(test)]
#[path = "commit_tests.rs"]
mod commit_tests;

#[cfg(test)]
#[path = "replay_tests.rs"]
mod replay_tests;
