//! Live talks v3, lane N5a: the high school coach answering college calls
//! in words (question 10, brought back 2026-11-15; section 6.15). A college
//! staff calls you about one of your players and asks about his football,
//! his grades or his character (`dialogue::college_calls`, D9). With a key
//! you answer in your own words over the call's three exchanges, and the
//! game reads what you did with them:
//!
//! - **Covered for him**: a claim on the worry they asked about that isn't
//!   true ("he's a 3.5 student" when he's at 2.3), or reassuring them on it
//!   when there is something there.
//! - **Straight**: a true claim that names the worry, or being straight
//!   about it ("his grades need work").
//! - **Sold him**: praise for his football, flattering him, or good words
//!   on a part they didn't worry about.
//! - Saying nothing on the worry they asked about is going around it, and
//!   counts as covering; nothing at all with no worry is keeping it short.
//!
//! Covering outweighs being straight, which outweighs selling. When the
//! call ends your word lands as the chip's would (`dialogue::call_word`):
//! his recruitment, the trail, and a year on your credibility with that
//! staff (`settle_calls`). A claim about his grades or character reads
//! "later": they find out when he's on their campus. Every size here is
//! Claude's (proposed, not confirmed).

use super::super::Them;
use super::super::ear::{Act, How, Item, Reading, Sized, Turn};
use super::super::room::{Channel, Present, Room, Seat};
use super::super::wants::{Kind, Want};
use crate::world::World;
use crate::world::folk::claims::{Stated, Truth};
use crate::world::folk::scene::{Kind as SceneKind, Scene, Who};
use crate::world::folk::{self as folk, dialogue};

/// A line covered for him.
pub const COVER: u8 = 1;
/// A line was straight about the worry.
pub const STRAIGHT: u8 = 2;
/// A line sold him.
pub const SOLD: u8 = 4;
/// The worry came up at all.
pub const ADDRESSED: u8 = 8;

/// The scene's slot that keeps what your lines did (`Scene.f[2]`, unused by
/// the call's chips).
const SLOT: usize = 2;

/// The words that say something good, and something that isn't.
const GOOD: [&str; 16] = [
    "good kid",
    "great kid",
    "clean",
    "never",
    "no trouble",
    "no issue",
    "nothing",
    "fine",
    "solid",
    "honor",
    "good student",
    "good grades",
    "character",
    "leader",
    "3.",
    "great",
];
const BAD: [&str; 15] = [
    "trouble",
    "issue",
    "problem",
    "suspend",
    "arrest",
    "fight",
    "struggl",
    "behind",
    "failing",
    "low",
    "work to do",
    "needs work",
    "immature",
    "attitude",
    "not great",
];

/// Whether a scene is a college staff's call to you.
pub fn is_call(k: SceneKind) -> bool {
    k == SceneKind::CollegeCall
}

/// The college coach on the line, or his program's recruiting
/// coordinator when the world holds no coach for it.
pub fn caller(s: &Scene) -> Them {
    s.who
        .iter()
        .find_map(|w| if let Who::Coach(c) = w { Some(Them::Coach(*c)) } else { None })
        .unwrap_or(Them::Drawn(Kind::OtherCoach, s.b))
}

/// The room: a call with him.
pub fn room(s: &Scene) -> Option<Room> {
    if !is_call(s.kind) {
        return None;
    }
    Some(Room { channel: Channel::Phone, present: vec![Present { who: caller(s), seat: Seat::Across, speaks: true }] })
}

/// The worry they asked about, 1 his grades, 2 his character, 0 none.
fn concern(s: &Scene) -> u8 {
    s.c.clamp(0, 2) as u8
}

/// The caller's card: the player he called about, what he asked, his own
/// wants and his pride.
pub fn items(w: &World, s: &Scene, wants: &[Want]) -> Option<Vec<(Item, String)>> {
    if !is_call(s.kind) {
        return None;
    }
    let mut out = Vec::new();
    let kid = s.a as u32;
    if let Some(i) = folk::person(w, kid) {
        out.push((Item::Friend(kid), format!("the player he called about, {}", w.people[i].name)));
    }
    let about = ["his football", "his grades", "his character"][concern(s) as usize];
    out.push((Item::Point("hscall.ask".into()), format!("what he asked you about: {about}")));
    for (k, v) in wants.iter().enumerate() {
        out.push((Item::Want(k as u8), format!("what he wants: {}", super::items::want_words(v.kind))));
    }
    out.push((Item::Pride, "his pride".into()));
    Some(out)
}

/// What a claim about the kid is about: 1 his grades, 2 his character, 0
/// his football; None when it isn't about him.
fn topic(s: &Scene, st: &Stated) -> Option<u8> {
    if st.about.is_some_and(|a| a != s.a as u32) {
        return None;
    }
    let v = st.value.to_lowercase();
    let school = ["gpa", "grade", "class", "student", "academ", "school"].iter().any(|k| v.contains(k));
    Some(match st.kind.as_str() {
        "off_field" => 2,
        _ if school => 1,
        "stat" | "status" | "injury" | "record" | "role" | "depth_rank" => 0,
        _ => return None,
    })
}

/// A GPA said in a claim.
fn gpa_of(v: &str) -> Option<f64> {
    v.split(|c: char| !(c.is_ascii_digit() || c == '.'))
        .filter_map(|x| x.parse::<f64>().ok())
        .find(|x| (0.5..=4.5).contains(x))
}

/// Whether a claim on his grades or character is true, and whether it
/// speaks well of him: (good, true). None for football and the rest.
fn read(w: &World, s: &Scene, st: &Stated) -> Option<(bool, bool)> {
    let t = topic(s, st)?;
    let i = folk::person(w, s.a as u32)?;
    let p = &w.people[i];
    let v = st.value.to_lowercase();
    let bad = BAD.iter().any(|k| v.contains(k));
    let good = !bad && (GOOD.iter().any(|k| v.contains(k)) || t == 1 && gpa_of(&v).is_some());
    match t {
        1 => {
            let gpa = p.bg.gpa;
            let truth = match gpa_of(&v) {
                Some(n) if !bad => n <= gpa + 0.25,
                _ if bad => gpa < 3.0,
                _ => gpa >= 2.5,
            };
            Some((good, truth))
        }
        2 => {
            let trouble = dialogue::concern(p) == 2;
            Some((good, if bad { trouble } else { !trouble }))
        }
        _ => None,
    }
}

/// A claim checked on the call: about his grades or his character, it is
/// found out later, when he's on their campus (a year on, `settle_calls`).
pub fn check(w: &World, s: &Scene, st: &Stated) -> Truth {
    match read(w, s, st) {
        Some(_) => Truth::Later,
        None => Truth::Unknown,
    }
}

/// What a line did with the call (`COVER`, `STRAIGHT`, `SOLD`, `ADDRESSED`).
pub fn marks(w: &World, s: &Scene, reading: &Reading) -> u8 {
    let worry = concern(s);
    let mut m = 0u8;
    for a in &reading.acts {
        let Act::Claim(st) = a else { continue };
        let Some(t) = topic(s, st) else { continue };
        if t == 0 {
            m |= SOLD;
            continue;
        }
        let Some((good, truth)) = read(w, s, st) else { continue };
        if t == worry {
            m |= ADDRESSED;
            m |= match (good, truth) {
                (true, false) => COVER,
                (false, true) => STRAIGHT,
                (true, true) => SOLD,
                // Running him down on what they asked: you told them, if not
                // fairly. They weigh it as a straight word.
                (false, false) => STRAIGHT,
            };
        } else if good {
            m |= SOLD;
        }
    }
    for h in &reading.hits {
        match (&h.item, h.how) {
            (Item::Friend(k), How::Flattered | How::Reassured) if *k == s.a as u32 => m |= SOLD,
            (Item::Point(k), How::Reassured) if k == "hscall.ask" && worry > 0 => m |= COVER | ADDRESSED,
            (Item::Point(k), How::Respected | How::Doubted) if k == "hscall.ask" => m |= STRAIGHT | ADDRESSED,
            _ => {}
        }
    }
    m
}

/// Your word from everything the call's lines did: 0 sold, 1 straight, 2
/// covered or kept short.
pub fn word(marks: u8) -> u8 {
    if marks & COVER != 0 {
        2
    } else if marks & STRAIGHT != 0 {
        1
    } else if marks & SOLD != 0 {
        0
    } else {
        2
    }
}

/// The game's turn on a call, on top of lane N4's: what he takes from your
/// line, in words for the voice. Hits move nothing on a call (no chip
/// does): your word lands when the call ends.
pub fn turn(w: &World, s: &Scene, reading: &Reading, t: &mut Turn) {
    if !is_call(s.kind) {
        return;
    }
    let m = marks(w, s, reading);
    for h in &mut t.hits {
        h.trust = h.trust.min(0.0).max(-0.03);
        h.mood = 0.0;
    }
    let said = if m & COVER != 0 {
        "He takes your word for it, for now."
    } else if m & STRAIGHT != 0 {
        "He thanks you for being straight with him. They'll weigh it with the film."
    } else if m & SOLD != 0 {
        "He says they'll look harder at his film."
    } else {
        "He notes it and asks if there's anything else he should know."
    };
    t.say.released.push(said.into());
}

/// Keeps what this line did on the scene.
pub fn apply(w: &mut World, id: u32, reading: &Reading, t: &Turn) {
    for h in t.hits.iter().filter(|h| h.trust != 0.0) {
        super::super::stores::nudge_trust(w, h.hit.on, h.trust);
    }
    let Some(k) = w.folk.scenes.iter().position(|s| s.id == id) else { return };
    let m = marks(w, &w.folk.scenes[k], reading);
    let s = &mut w.folk.scenes[k];
    s.f[SLOT] = f32::from(s.f[SLOT] as u8 | m);
}

/// Ends the call: your word lands as the chip's would.
pub fn end(w: &mut World, s: &mut Scene) {
    let m = s.f[SLOT] as u8;
    let worry = concern(s);
    let wd = word(m);
    // Nothing at all said with no worry asked is keeping it short.
    let judged = if wd == 2 && m & COVER == 0 && worry == 0 { 0 } else { worry };
    dialogue::call_word(w, s, wd, judged);
    s.end();
}

/// The hits a call keeps (none move anything until its end).
pub fn size(_w: &World, s: &Scene, h: &super::super::ear::Hit) -> Option<Sized> {
    if !is_call(s.kind) {
        return None;
    }
    let trust = if matches!(h.how, How::Insulted | How::Threatened) { -0.03 } else { 0.0 };
    Some(Sized { hit: h.clone(), trust, mood: 0.0, faded: false })
}
