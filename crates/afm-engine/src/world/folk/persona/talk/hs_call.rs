//! Live talks v3, lane N5a: the high school coach answering college calls
//! in words (question 10, brought back 2026-11-15; section 6.15). A college
//! staff calls you about one of your players and asks about his football,
//! his grades or his character (`dialogue::college_calls`, D9). With a key
//! you answer in your own words over the call's three exchanges, and the
//! game weighs each thing you said (`weigh`):
//!
//! - **Covering for him**: a claim on the worry they asked about that isn't
//!   true ("he's a 3.5 student" when he's at 2.3), weighed by how far it is
//!   from the truth, or reassuring them on it when there is something there.
//! - **Straight**: a true claim that names the worry, or being straight
//!   about it ("his grades need work").
//! - **Selling him**: praise for his football, flattering him, or good
//!   words on a part they didn't worry about.
//! - Saying nothing on the worry they asked about is going around it, and
//!   weighs as half a cover (`DODGE`).
//!
//! Nothing is boiled down to one of the chip's three words: when the call
//! ends, everything you said lands together (`dialogue::call_said`): how
//! hard you pushed him reaches his recruitment, a sharp caller discounting
//! what he can't see on film; the trail hears each part; two different
//! things on the same worry are noticed by a sharp caller now; and a year
//! on each part is settled against what came true (`settle_calls`). What
//! the caller says back on each line comes from your credibility with that
//! staff and his judgement. A claim about his grades or character reads
//! "later": they find out when he's on their campus. Every size here is
//! Claude's (proposed, not confirmed).

use super::super::Them;
use super::super::ear::{Act, How, Item, Landing, Reading, Sized, Turn};
use super::super::knows::Topic;
use super::super::room::{Channel, Present, Room, Seat};
use super::super::wants::{Kind, Want};
use crate::world::World;
use crate::world::folk::claims::{Stated, Truth};
use crate::world::folk::dialogue::CallSaid;
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
/// The slots that add up how much you covered, were straight and sold him
/// (`Scene.f[0]`, `f[1]`, `f[3]`, unused by the call's chips).
const COVER_AT: usize = 0;
const STRAIGHT_AT: usize = 1;
const SOLD_AT: usize = 3;

/// Going around the worry they asked about, against a lie about it.
/// Proposed, not confirmed.
pub const DODGE: f32 = 0.5;
/// What a line of wording alone (no claim) weighs against a claim.
/// Proposed, not confirmed.
pub const WORDING: f32 = 0.5;
/// Your credibility with that staff, less his judgement's discount, at
/// which he takes your word on the phone. Proposed, not confirmed.
pub const BELIEVED: f64 = 0.4;

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
            (Item::Friend(k), How::Flattered | How::Reassured | How::Staked) if *k == s.a as u32 => m |= SOLD,
            (Item::Point(k), How::Reassured) if k == "hscall.ask" && worry > 0 => m |= COVER | ADDRESSED,
            (Item::Point(k), How::Respected | How::Doubted) if k == "hscall.ask" => m |= STRAIGHT | ADDRESSED,
            _ => {}
        }
    }
    m
}

/// How far a cover is from the truth: a grade point average said by how
/// far above his (a half point over is 1, held between 0.5 and 1.5), any
/// other cover 1.
fn how_far(w: &World, s: &Scene, st: &Stated) -> f32 {
    let Some(i) = folk::person(w, s.a as u32) else { return 1.0 };
    match (topic(s, st), gpa_of(&st.value.to_lowercase())) {
        (Some(1), Some(n)) => (((n - w.people[i].bg.gpa) / 0.5) as f32).clamp(0.5, 1.5),
        _ => 1.0,
    }
}

/// What one line did, weighed as said: how much it covered for him, was
/// straight and sold him.
pub fn weigh(w: &World, s: &Scene, reading: &Reading) -> CallSaid {
    let worry = concern(s);
    let mut x = CallSaid::default();
    for a in &reading.acts {
        let Act::Claim(st) = a else { continue };
        let Some(t) = topic(s, st) else { continue };
        if t == 0 {
            x.sold += 1.0;
            continue;
        }
        let Some((good, truth)) = read(w, s, st) else { continue };
        if t == worry {
            match (good, truth) {
                (true, false) => x.cover += how_far(w, s, st),
                (true, true) => x.sold += 1.0,
                // Running him down on what they asked: you told them, if
                // not fairly. They weigh it as a straight word.
                _ => x.straight += 1.0,
            }
        } else if good {
            x.sold += 1.0;
        }
    }
    for h in &reading.hits {
        match (&h.item, h.how) {
            (Item::Friend(k), How::Flattered | How::Reassured) if *k == s.a as u32 => x.sold += WORDING,
            // One strong line on what he really is sells like a claim.
            (Item::Friend(k), How::Staked) if *k == s.a as u32 => x.sold += 1.0,
            (Item::Point(k), How::Reassured) if k == "hscall.ask" && worry > 0 => x.cover += WORDING,
            (Item::Point(k), How::Respected | How::Doubted) if k == "hscall.ask" => x.straight += WORDING,
            _ => {}
        }
    }
    x
}

/// What the call's lines have added up to so far.
fn so_far(s: &Scene) -> CallSaid {
    CallSaid { cover: s.f[COVER_AT], straight: s.f[STRAIGHT_AT], sold: s.f[SOLD_AT] }
}

/// The caller's judgement, -1 to 1 around an ordinary man's.
fn sharp(w: &World, s: &Scene) -> f64 {
    super::super::makeup::Makeup::z(super::super::makeup::makeup(w, caller(s)).judgement)
}

/// The game's turn on a call, on top of lane N4's: what he takes from your
/// line, in words for the voice, from what it did and whether he believes
/// you (your credibility with his staff, less his judgement's discount).
/// Hits move nothing on a call (no chip does): your word lands when the
/// call ends.
pub fn turn(w: &World, s: &Scene, reading: &Reading, t: &mut Turn) {
    if !is_call(s.kind) {
        return;
    }
    let x = weigh(w, s, reading);
    let before = so_far(s);
    for h in &mut t.hits {
        h.trust = h.trust.clamp(-0.03, 0.0);
        h.mood = 0.0;
    }
    let belief = crate::world::scouting::hs::credibility(w, s.b as usize) - 0.15 * sharp(w, s);
    let worry = concern(s) > 0;
    // Asked what his staff makes of the kid ("what's your staff thinking
    // about him?"): they like his film enough to call, and where it goes
    // waits on what he asked. Never "he doesn't know" (the talks audit,
    // 2026-11-16).
    let kid = Them::Player(s.a as u32);
    let on_him = reading.acts.iter().any(|a| {
        matches!(a, Act::Question { topic: Topic::Football | Topic::Pitches | Topic::ScoutFile | Topic::Wants | Topic::Market, about, .. }
            if about.is_none_or(|x| x == kid))
    });
    if on_him {
        t.say.released.retain(|x| x != "He doesn't know.");
        let asked = ["his football", "his grades", "his character"][concern(s) as usize];
        t.say.released.push(format!(
            "His staff likes the kid's film enough to recruit him; where it goes from here waits on {asked}"
        ));
    }
    let r = &mut t.say.released;
    // Only against an earlier line: your first line, covering for him in a
    // respectful tone, came back as "that's not what you told me a minute
    // ago" (the talks audit, 2026-11-16).
    if worry && ((x.cover > 0.0 && before.straight > 0.0) || (x.straight > 0.0 && before.cover > 0.0)) {
        r.push("He notes that's not what you told him a minute ago.".into());
    }
    if x.cover > 0.0 {
        r.push(if belief >= BELIEVED {
            "He takes your word for it, for now.".into()
        } else {
            "He doesn't sound convinced. He says they'll check it themselves.".into()
        });
    }
    if x.straight > 0.0 {
        r.push("He thanks you for being straight with him. They'll weigh it with the film.".into());
    }
    if x.sold > 0.0 {
        r.push(if belief >= BELIEVED {
            // His own words, quoted: "so they'll look harder at his film on
            // our end" (the talks audit, 2026-11-16).
            "He says: \"We'll look harder at his film.\"".into()
        } else {
            "He says: \"The film will have to show me.\"".into()
        });
    }
    if r.is_empty() {
        r.push("He notes it and asks if there's anything else he should know.".into());
    }
}

/// Keeps what this line did on the scene.
pub fn apply(w: &mut World, id: u32, reading: &Reading, t: &Turn) {
    for h in t.hits.iter().filter(|h| h.trust != 0.0) {
        super::super::stores::nudge_trust(w, h.hit.on, h.trust);
    }
    let Some(k) = w.folk.scenes.iter().position(|s| s.id == id) else { return };
    let m = marks(w, &w.folk.scenes[k], reading);
    let x = weigh(w, &w.folk.scenes[k], reading);
    let s = &mut w.folk.scenes[k];
    s.f[SLOT] = f32::from(s.f[SLOT] as u8 | m);
    s.f[COVER_AT] += x.cover;
    s.f[STRAIGHT_AT] += x.straight;
    s.f[SOLD_AT] += x.sold;
}

/// Ends the call: everything you said lands together, weighed as said
/// (`dialogue::call_said`). Going around the worry they asked about adds
/// half a cover.
pub fn end(w: &mut World, s: &mut Scene) {
    let m = s.f[SLOT] as u8;
    let worry = concern(s);
    let mut said = so_far(s);
    if worry > 0 && m & ADDRESSED == 0 {
        said.cover += DODGE;
    }
    let sharp = sharp(w, s);
    dialogue::call_said(w, s, said, worry, sharp);
    s.end();
}

/// How a line went over with the caller (the landing read, 2026-11-17):
/// what you said lands when the call ends, all together, so a line shows
/// nothing yet unless it cost his trust (an insult, a threat).
pub fn landing(him: Them, hits: &[Sized]) -> Landing {
    match super::hits::landing(him, hits, &[]) {
        l @ (Landing::Missed | Landing::Cost) => l,
        _ => Landing::Untouched,
    }
}

/// The hits a call keeps (none move anything until its end).
pub fn size(_w: &World, s: &Scene, h: &super::super::ear::Hit) -> Option<Sized> {
    if !is_call(s.kind) {
        return None;
    }
    let trust = if matches!(h.how, How::Insulted | How::Threatened) { -0.03 } else { 0.0 };
    Some(Sized { hit: h.clone(), trust, mood: 0.0, faded: false })
}
