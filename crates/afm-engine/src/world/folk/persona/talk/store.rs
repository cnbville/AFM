//! What a talk keeps (sections 3.6 and 14): the open talk's own state
//! (who's in the room, the heat, his patience, what he's raised and what
//! you've already said), and every line's saved reading with your words
//! and his, so a reload plays the same world. All of it lives in
//! `claims::Live.persona` (`memory::Memory`), the live talks' one saved
//! store; an empty one is never written.

use super::super::Them;
use super::super::ear::{Act, Hit, How, Item, Turn};
use super::super::memory::{Colour, Line, Memory, Saved, Treated, Treatment};
use super::super::room::{Present, Room};
use super::super::terms::Request;
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
    /// Requests he said no to.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refused: Vec<Request>,
}

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
            refused: Vec::new(),
        }
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
pub fn advance(w: &mut World, scene: u32, room: &Room, t: &Turn, said: &Said) {
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
    let boiled = if t.heat.0 >= super::feel::BOIL {
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
    o.lines = o.lines.saturating_add(1);
    if boiled.is_some() {
        o.boiled = boiled;
    }
    o.raised.extend(t.his_asks.iter().map(|a| a.why.kind));
    o.hits.extend(t.hits.iter().map(|h| (h.hit.on, h.hit.item.clone(), h.hit.how)));
    for act in &said.reading.acts {
        if let Act::Question { about, .. } = act {
            o.asked.push(*about);
        }
    }
    for (r, wl, _) in &t.asked {
        if *wl == Willing::No && !o.refused.contains(r) {
            o.refused.push(r.clone());
        }
    }
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

/// His notebook, oldest first, for the voice.
pub fn notebook(w: &World, who: Them) -> Vec<String> {
    w.folk.live.persona.notebook.iter().filter(|c| c.who == who).map(|c| c.note.clone()).collect()
}

/// The lines of an open talk saved so far (the ear's readings with them).
pub fn lines(w: &World, who: Them, day: Date) -> &[Line] {
    w.folk.live.persona.talks.iter().rev().find(|t| t.with == who && t.day == day).map_or(&[], |t| t.lines.as_slice())
}
