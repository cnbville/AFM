//! What a talk keeps (sections 3.6 and 14): the open talk's own state
//! (who's in the room, the heat, his patience, what he's raised and what
//! you've already said), and every line's saved reading with your words
//! and his, so a reload plays the same world. All of it lives in
//! `claims::Live.persona` (`memory::Memory`), the live talks' one saved
//! store; an empty one is never written.

use super::super::Them;
use super::super::ear::{Act, Hit, How, Item, Turn};
use super::super::knows::Topic;
use super::super::memory::{Colour, Line, Memory, Saved, Shared, Treated, Treatment};
use super::super::room::{Present, Room};
use super::super::terms::{Give, Offer, Request};
use super::super::wants::{Boil, Heat, Patience, WantKind, Willing};
use super::{Said, channels};
use crate::calendar::Date;
use crate::world::World;

/// The most talks kept with their lines. Proposed, not confirmed.
pub const KEEP_TALKS: usize = 240;
/// The most lines kept a talk. Proposed, not confirmed.
pub const KEEP_LINES: usize = 40;
/// The most colour notes kept a person. Proposed, not confirmed.
pub const KEEP_COLOUR: usize = 8;
/// The most treatments kept in all. Proposed, not confirmed.
pub const KEEP_TREATED: usize = 400;

/// A talk while it is open.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Open {
    /// The scene it is held in.
    pub scene: u32,
    pub room: Room,
    #[serde(default)]
    pub heat: Heat,
    #[serde(default)]
    pub patience: Patience,
    /// Lines said so far.
    #[serde(default)]
    pub lines: u8,
    /// He boiled over, and how.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boiled: Option<Boil>,
    /// The wants he raised himself (once a talk each).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub raised: Vec<WantKind>,
    /// Each hit so far: on whom, on what, how (repeats fade).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hits: Vec<(Them, Item, How)>,
    /// Questions asked so far, by whom they're about.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub asked: Vec<Option<Them>>,
    /// Each question asked so far, by its topic and whom it's about: one
    /// asked again gets nowhere (Cain 2026-11-15).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub questions: Vec<(Topic, Option<Them>)>,
    /// Requests he said no to.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refused: Vec<Request>,
    /// Season review points he has given you in this talk (a push gives
    /// the next ones).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub reviewed: u8,
    /// The line count when he last raised an ask of his own: his ask is on
    /// the table only for the line right after it (Cain 2026-11-16: an ask
    /// raised once stayed "on the table" all talk, so every later line read
    /// as dodging it). 0 in a file from before it.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub raised_at: u8,
    /// The talk is over but the meeting holds a decision still to make
    /// (`wrap::decides`): no more lines, the meeting's picks decide it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub over: bool,
    /// Something personal landed in this talk (personal talk, item 8): it
    /// counts as one more week of contact with you.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub personal: bool,
    /// He went short on a question about his troubles asked too soon: the
    /// surface only for the rest of the talk.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub closed: bool,
    /// In a room, how the last line landed with each man (group talks:
    /// his tag in the room list): signed, with the way it hit him hardest.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub takes: Vec<(Them, f32, How)>,
    /// The first line only said hello (`Turn::greeting`): his opening's
    /// question is on the table for the line after it too.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub greeted: bool,
    /// The promises made in this talk (P1's indexes): his card shows them
    /// as "earlier in this talk", never in his history with you (talk
    /// replies item 5).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub promised: Vec<u32>,
    /// His counter still on the table, for your answer to it (talk replies
    /// item 3): what he wants given and how he wants to be used, his words
    /// for it, and the offer or ask it answered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub counter: Option<OnTable>,
    /// What this talk has settled so far, in order (talk replies item 11):
    /// agreed, promised, refused, still open. The cards carry it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub settled: Vec<String>,
}

/// His counter on the table (talk replies item 3).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct OnTable {
    /// The line it was said on.
    pub line: u8,
    pub asks: Vec<String>,
    pub gives: Vec<Give>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub terms: Vec<crate::world::folk::usage::Usage>,
    /// The offer he countered, when it was one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offer: Option<Offer>,
    /// The ask of yours he countered, when it was one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<Request>,
}

/// The most entries kept on a talk's settled list. Proposed, not confirmed.
pub const KEEP_SETTLED: usize = 16;

impl Open {
    pub fn new(scene: u32, room: &Room) -> Open {
        Open {
            scene,
            room: room.clone(),
            heat: Heat::default(),
            patience: Patience::default(),
            lines: 0,
            boiled: None,
            raised: Vec::new(),
            hits: Vec::new(),
            asked: Vec::new(),
            questions: Vec::new(),
            refused: Vec::new(),
            reviewed: 0,
            raised_at: 0,
            over: false,
            personal: false,
            closed: false,
            takes: Vec::new(),
            greeted: false,
            promised: Vec::new(),
            counter: None,
            settled: Vec::new(),
        }
    }

    /// His ask still on the table: the one his last line raised, if it
    /// raised one.
    pub fn asking(&self) -> Option<WantKind> {
        if self.raised_at > 0 && self.raised_at == self.lines { self.raised.last().copied() } else { None }
    }

    /// How many times this hit was already made in the talk.
    pub fn times(&self, h: &Hit) -> u8 {
        self.hits.iter().filter(|x| x.0 == h.on && x.1 == h.item && x.2 == h.how).count().min(255) as u8
    }

    /// How many times you've already asked about this man in the talk.
    pub fn pushed(&self, about: Option<Them>) -> u8 {
        if about.is_none() {
            return 0;
        }
        self.asked.iter().filter(|x| **x == about).count().min(255) as u8
    }

    /// Whether you already asked him this question in the talk.
    pub fn asked_before(&self, topic: Topic, about: Option<Them>) -> bool {
        self.questions.contains(&(topic, about))
    }

    pub fn raised(&self, k: WantKind) -> bool {
        self.raised.contains(&k)
    }

    pub fn refused_before(&self, r: &Request) -> bool {
        self.refused.contains(r)
    }
}

/// The open talk held in a scene.
pub fn open(w: &World, scene: u32) -> Option<&Open> {
    w.folk.live.persona.open.iter().find(|o| o.scene == scene)
}

/// Who's in the room for a scene's talk.
pub fn room(w: &World, scene: u32) -> Option<Room> {
    open(w, scene).map(|o| o.room.clone())
}

/// Moves the man who answers this line to the front of a talk's room, so
/// the talk reads him (a staff room: the coach the question fits,
/// `coaching::answerer`).
pub fn reseat(w: &mut World, scene: u32, who: super::super::Them) {
    if let Some(o) = w.folk.live.persona.open.iter_mut().find(|o| o.scene == scene) {
        o.room = super::super::coaching::lead_with(o.room.clone(), who);
    }
}

/// Opens a talk in a scene with a room (a channel, the people there), or
/// changes its room before the first line.
pub fn begin(w: &mut World, scene: u32, room: Room) {
    let m = &mut w.folk.live.persona.open;
    match m.iter_mut().find(|o| o.scene == scene) {
        Some(o) if o.lines == 0 => o.room = room,
        Some(_) => {}
        None => m.push(Open::new(scene, &room)),
    }
}

/// After a line: the heat, his patience, what he raised, who's in the room
/// now (someone brought in, someone asked to leave).
/// `point` is the heat at which he boils over.
pub fn advance(w: &mut World, scene: u32, room: &Room, t: &Turn, said: &Said, point: f32) {
    let mut people: Vec<Present> = room.present.clone();
    for act in &said.reading.acts {
        match act {
            Act::BringIn(who)
                if channels::allows(room.channel).bring_in
                    && channels::can_bring(w, *who)
                    && !people.iter().any(|p| p.who == *who) =>
            {
                people.push(Present { who: *who, seat: channels::brought_seat(*who), speaks: false });
            }
            Act::AskToLeave(who) if people.first().is_some_and(|p| p.who != *who) => {
                people.retain(|p| p.who != *who);
            }
            _ => {}
        }
    }
    let boiled = if t.heat.0 >= point {
        Some(match t.ends {
            Some(super::super::ear::Ends::Him(Some(b))) => b,
            _ => {
                if super::z(w, people[0].who, crate::player::Trait::Temper) > 0.3 {
                    Boil::Loud
                } else {
                    Boil::Quiet
                }
            }
        })
    } else {
        None
    };
    let m = &mut w.folk.live.persona.open;
    let k = match m.iter().position(|o| o.scene == scene) {
        Some(k) => k,
        None => {
            m.push(Open::new(scene, room));
            m.len() - 1
        }
    };
    let o = &mut m[k];
    o.room = Room { channel: room.channel, present: people };
    o.heat = t.heat;
    o.patience = t.patience;
    o.greeted |= t.greeting;
    o.lines = o.lines.saturating_add(1);
    if boiled.is_some() {
        o.boiled = boiled;
    }
    o.raised.extend(t.his_asks.iter().map(|a| a.why.kind));
    if !t.his_asks.is_empty() {
        o.raised_at = o.lines;
    }
    o.hits.extend(t.hits.iter().map(|h| (h.hit.on, h.hit.item.clone(), h.hit.how)));
    o.reviewed = o.reviewed.saturating_add(t.reviewed);
    if t.crowd.is_some() {
        o.takes = o
            .room
            .present
            .iter()
            .filter_map(|p| {
                let (tr, md, how) = super::super::crowd::take(&t.hits, p.who);
                how.map(|h| (p.who, tr + md, h))
            })
            .collect();
    }
    for act in &said.reading.acts {
        if let Act::Question { topic, about, .. } = act {
            o.asked.push(*about);
            if !o.questions.contains(&(*topic, *about)) {
                o.questions.push((*topic, *about));
            }
        }
    }
    for (r, wl, _) in &t.asked {
        if *wl == Willing::No && !o.refused.contains(r) {
            o.refused.push(r.clone());
        }
    }
    // His counter stays on the table for your answer; anything else you
    // put to him, or your answer to it, takes it off (talk replies item 3).
    let line = o.lines;
    if let Some(c) = &t.counter {
        let offer = t.weighed.iter().rev().find(|(_, wd)| matches!(wd, super::super::terms::Weighed::Counter(_)));
        let request = t.asked.iter().rev().find(|(_, wl, _)| matches!(wl, Willing::If(_)));
        o.counter = Some(OnTable {
            line,
            asks: c.asks.clone(),
            gives: c.gives.clone(),
            terms: c.terms.clone(),
            offer: offer.map(|x| x.0.clone()),
            request: request.map(|x| x.0.clone()),
        });
    } else if t.answered || !t.weighed.is_empty() {
        o.counter = None;
    }
    for x in &t.settled {
        if !o.settled.contains(x) {
            o.settled.push(x.clone());
        }
    }
    if o.settled.len() > KEEP_SETTLED {
        let cut = o.settled.len() - KEEP_SETTLED;
        o.settled.drain(..cut);
    }
}

/// The promises a talk made, kept so his card shows them as said in this
/// talk (talk replies item 5).
pub fn promised(w: &mut World, scene: u32, ks: impl Iterator<Item = u32>) {
    if let Some(o) = w.folk.live.persona.open.iter_mut().find(|o| o.scene == scene) {
        o.promised.extend(ks);
    }
}

/// Every promise made in a talk still open: his history with you leaves
/// them out (talk replies item 5).
pub fn in_open_talks(w: &World) -> Vec<u32> {
    w.folk.live.persona.open.iter().flat_map(|o| o.promised.iter().copied()).collect()
}

/// The talk is over and the meeting waits on a decision: the talk's state
/// stays (a man done talking stays done today), marked over.
pub fn over(w: &mut World, scene: u32) {
    let m = &mut w.folk.live.persona.open;
    match m.iter_mut().find(|o| o.scene == scene) {
        Some(o) => o.over = true,
        None => {
            let room = w.folk.scenes.iter().find(|s| s.id == scene).map(|s| super::channels::room_for(w, s));
            if let Some(room) = room {
                let mut o = Open::new(scene, &room);
                o.over = true;
                w.folk.live.persona.open.push(o);
            }
        }
    }
}

/// Whether a scene's talk is over while its meeting waits on a decision.
pub fn is_over(w: &World, scene: u32) -> bool {
    open(w, scene).is_some_and(|o| o.over)
}

/// The talk is over: its open state goes.
pub fn close(w: &mut World, scene: u32) {
    w.folk.live.persona.open.retain(|o| o.scene != scene);
}

/// Your word with someone the world keeps no trust for (an agent, a GM),
/// by the key `claims::Live.word` uses for him.
pub fn word_key(who: Them) -> u64 {
    match who {
        Them::Player(id) => u64::from(id),
        Them::Kin(id, k) => 1 << 40 | u64::from(id) << 8 | u64::from(k),
        Them::Coach(c) => 2 << 40 | c as u64,
        Them::Cast(r, n) => r.key(n),
        Them::Agent(a) => 4 << 40 | a as u64,
        Them::ProScout(s) => 5 << 40 | s as u64,
        Them::Drawn(_, k) => 6 << 40 | (k & 0xFF_FFFF_FFFF),
    }
}

/// Remembers how you treated him.
pub fn treat(m: &mut Memory, who: Them, how: Treated, day: Date) {
    m.treated.push(Treatment { who, how, day });
    if m.treated.len() > KEEP_TREATED {
        m.treated.remove(0);
    }
}

/// Saves one line of a talk (your words and the ear's reading; his words
/// follow with `his_words`).
pub fn save_line(m: &mut Memory, with: Them, day: Date, line: Line) {
    let k = match m.talks.iter().rposition(|t| t.with == with && t.day == day) {
        Some(k) => k,
        None => {
            m.talks.push(Saved { with, day, lines: Vec::new() });
            if m.talks.len() > KEEP_TALKS {
                m.talks.remove(0);
            }
            m.talks.len() - 1
        }
    };
    let t = &mut m.talks[k];
    t.lines.push(line);
    if t.lines.len() > KEEP_LINES {
        t.lines.remove(0);
    }
}

/// His words for the last saved line with him.
pub fn his_words(m: &mut Memory, with: Them, day: Date, words: &str) {
    if let Some(l) = m.talks.iter_mut().rev().find(|t| t.with == with && t.day == day).and_then(|t| t.lines.last_mut())
    {
        l.his = words.to_string();
    }
}

/// A detail the voice added about him (question 5).
pub fn note(m: &mut Memory, who: Them, day: Date, note: String) {
    if m.notebook.iter().any(|c| c.who == who && c.note == note) {
        return;
    }
    m.notebook.push(Colour { who, day, note });
    let mine: Vec<usize> = (0..m.notebook.len()).filter(|&k| m.notebook[k].who == who).collect();
    if mine.len() > KEEP_COLOUR {
        m.notebook.remove(mine[0]);
    }
}

/// Keeps something told about a life (personal talk, item 7), the newest
/// `home::KEEP_SHARED` a person.
pub fn share(m: &mut Memory, who: Them, day: Date, yours: bool, key: u16, words: String) {
    if m.shared.iter().any(|x| x.who == who && x.yours == yours && x.key == key && x.words == words) {
        return;
    }
    m.shared.push(Shared { who, day, yours, key, words });
    let mine: Vec<usize> = (0..m.shared.len()).filter(|&k| m.shared[k].who == who).collect();
    if mine.len() > super::super::home::KEEP_SHARED {
        m.shared.remove(mine[0]);
    }
}

/// What he has told you about his life, by kind.
pub fn told_keys(w: &World, who: Them) -> Vec<u16> {
    w.folk.live.persona.shared.iter().filter(|x| x.who == who && !x.yours).map(|x| x.key).collect()
}

/// What he has told you about his life in the last `days` days.
pub fn told_since(w: &World, who: Them, days: i32) -> Vec<u16> {
    let today = w.today;
    w.folk
        .live
        .persona
        .shared
        .iter()
        .filter(|x| x.who == who && !x.yours && x.day.days_to(today) <= days)
        .map(|x| x.key)
        .collect()
}

/// What he has told you about his life, and what you told him about yours,
/// oldest first: (yours, words).
pub fn shared(w: &World, who: Them) -> Vec<(bool, String)> {
    w.folk.live.persona.shared.iter().filter(|x| x.who == who).map(|x| (x.yours, x.words.clone())).collect()
}

/// His notebook, oldest first, for the voice.
pub fn notebook(w: &World, who: Them) -> Vec<String> {
    w.folk.live.persona.notebook.iter().filter(|c| c.who == who).map(|c| c.note.clone()).collect()
}

/// The lines of an open talk saved so far (the ear's readings with them).
pub fn lines(w: &World, who: Them, day: Date) -> &[Line] {
    w.folk.live.persona.talks.iter().rev().find(|t| t.with == who && t.day == day).map_or(&[], |t| t.lines.as_slice())
}

fn is_zero(x: &u8) -> bool {
    *x == 0
}
