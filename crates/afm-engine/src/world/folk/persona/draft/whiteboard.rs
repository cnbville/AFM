//! The live whiteboard (question 10: "reads his football IQ"). In an
//! interview you put a play from his spot on the board and he explains it:
//! the protection and the hot throw for a quarterback, his run fit and his
//! drop for a linebacker. How many of its six assignments he gets right
//! comes from his Football IQ and his play recognition; how clearly he
//! explains it from his communication. It reads all three into your file,
//! tighter than an interview's talk alone, and sharper the better you know
//! his position yourself (your evaluation and your teaching of his group,
//! as a coach reader's judge).
//!
//! The picked whiteboard topic (`own::Topic::Whiteboard`) feeds the same
//! read from its minutes and its end-of-day quiz; a typed room keeps its
//! sessions in `Book.boards`. Sizes are Claude's picks, proposed, not
//! confirmed.

use crate::attributes::Attr;
use crate::calendar::Date;
use crate::positions::Position;
use crate::world::World;
use crate::world::scouting::file::{FileRead, sharp};
use crate::world::season::key_normal;

/// One whiteboard session in a typed room.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Board {
    pub id: u32,
    pub date: Date,
    pub minutes: u8,
    /// Assignments he got right, of six.
    pub got: u8,
}

/// A session's minutes in an 18-minute room (longer in a longer room, as
/// the picked topics scale). Proposed, not confirmed.
pub const MINUTES: f32 = 6.0;
/// The base misread of each rating on a six-minute session, in rating
/// points: Football IQ, play recognition, communication. Proposed, not
/// confirmed (an interview's talk alone reads them at 7.0, 8.5 and 7.0).
pub const SD: [(Attr, f64); 3] = [(Attr::FootballIq, 4.0), (Attr::PlayRecognition, 5.0), (Attr::Communication, 6.0)];

/// What goes on the board for his spot, easiest first.
pub fn assignments(pos: Position) -> [&'static str; 6] {
    match pos {
        Position::QB => [
            "the protection call against a four-man front",
            "the hot throw against a zero blitz",
            "the coverage read on his drop",
            "the check from run to pass against a loaded box",
            "the progression on the concept",
            "the late rotation of the safety",
        ],
        Position::RB => [
            "his track and his read on the zone run",
            "the protection slide and his man in it",
            "the pickup on a delayed blitz",
            "the check-release route",
            "his option route against man and zone",
            "the cut-back when the backside flows",
        ],
        Position::WR => [
            "his split and his route",
            "the sight adjustment against the blitz",
            "the option route against man and zone",
            "where to settle against zone",
            "his block on the run from motion",
            "the release against press",
        ],
        Position::TE => [
            "his block on the base run",
            "his route and its depth",
            "the protection when he stays in",
            "the sight adjustment against the blitz",
            "his option route against the linebacker",
            "the new assignment from motion",
        ],
        Position::OT | Position::IOL => [
            "his man on the base run",
            "the slide call",
            "the twist pickup",
            "the combination block and the climb",
            "who he takes against an overload",
            "the line's call against a moved front",
        ],
        Position::EDGE => [
            "his gap against the run",
            "his rush lane",
            "the stunt and when he loops",
            "setting the edge on the outside run",
            "the read on the zone read",
            "his drop on the zone blitz",
        ],
        Position::DT => [
            "his gap and his technique",
            "the twist with the end",
            "how to play the double team",
            "his rush lane",
            "the screen read",
            "the shift before the snap",
        ],
        Position::LB => [
            "his run fit",
            "his drop against the formation",
            "the check against motion",
            "his blitz path",
            "the call to the front",
            "the answer to an empty set",
        ],
        Position::CB => [
            "his leverage in the call",
            "the check against a bunch",
            "the switch call on crossers",
            "his run support",
            "the answer to motion",
            "the disguise and the rotation at the snap",
        ],
        Position::S => [
            "his part of the coverage",
            "the rotation at the snap",
            "the check against motion",
            "his fit from the box",
            "the call to the corners",
            "the answer to trips",
        ],
        Position::K | Position::P | Position::LS => [
            "his spot on the kick",
            "the protection on the unit",
            "the timing of the operation",
            "his lane in coverage",
            "the call on a fake",
            "the rule on a muffed kick",
        ],
    }
}

/// How many of six he gets right: 0.6 Football IQ and 0.4 play
/// recognition, as the picked topic's quiz reads it, keyed to the day.
pub fn got(w: &World, i: usize, salt: u64) -> u8 {
    let p = &w.people[i];
    let a = &p.player.attributes;
    let skill = 0.6 * a.get(Attr::FootballIq) + 0.4 * a.get(Attr::PlayRecognition);
    let x = (skill - 40.0) / 50.0 + 0.12 * key_normal(0x0B0A ^ (w.today.0 as u64) ^ salt << 32, p.player.id as u64);
    (6.0 * x.clamp(0.0, 1.0)).round() as u8
}

/// The words of a session: what he had and what he missed, and how he put
/// it across.
pub fn words(w: &World, i: usize, got: u8) -> Vec<String> {
    let p = &w.people[i];
    let list = assignments(p.player.position);
    let g = got.min(6) as usize;
    let mut out = Vec::new();
    match g {
        0 => out.push(format!("He couldn't get {} right, or anything after it.", list[0])),
        6 => out.push(format!("He had all of it, through {}.", list[5])),
        _ => out.push(format!("He had {} and {}; he missed {}.", list[0], list[g - 1], list[g])),
    }
    let comm = p.player.attributes.get(Attr::Communication);
    out.push(
        if comm >= 70.0 {
            "He taught it back to you like a coach."
        } else if comm >= 50.0 {
            "He got it across plainly."
        } else {
            "He knew more than he could say: it came out in pieces."
        }
        .into(),
    );
    out
}

/// A typed room's session: the minutes come off the clock and go to the
/// whiteboard topic (so its traits are read too), and the session is kept
/// for the file. Returns his explanation in words, or None when there's no
/// room open with him or no time left.
pub fn run(w: &mut World, id: u32) -> Option<Vec<String>> {
    let k = w
        .folk
        .scenes
        .iter()
        .position(|s| s.kind == crate::world::folk::scene::Kind::CombineTalk && !s.done && s.a == id as u64)?;
    let i = crate::world::folk::person(w, id)?;
    let room = w.folk.scenes[k].f[0].max(1.0);
    let minutes = ((MINUTES * room / 18.0).round() as u8).clamp(3, 20);
    if w.folk.scenes[k].c < minutes as i64 {
        return Some(vec!["There isn't time left to put a play on the board.".into()]);
    }
    let salt = w.folk.live.persona.draft.boards.iter().filter(|b| b.id == id).count() as u64;
    let got = got(w, i, salt);
    let s = &mut w.folk.scenes[k];
    s.c -= minutes as i64;
    crate::world::folk::dialogue::spend_topic(s, crate::world::scouting::own::Topic::Whiteboard, minutes);
    let date = w.today;
    w.folk.live.persona.draft.boards.push(Board { id, date, minutes, got });
    Some(words(w, i, got))
}

/// Your eye for his spot: your evaluation and your teaching of his group,
/// as a coach reader's judge (0.6 and 0.4).
fn eye(w: &World, i: usize) -> f64 {
    let Some(c) = crate::world::scouting::readers::you(w) else { return 10.0 };
    let co = &w.coaches[c];
    let g = crate::world::coach::Group::of(w.people[i].player.position) as usize;
    let teach = co.ability().teaching.get(g).copied().unwrap_or(10.0);
    0.6 * co.evaluation() + 0.4 * teach
}

/// What the whiteboard adds to your file (applied after the file's own
/// looks, for your seat): each session reads Football IQ, play recognition
/// and communication at its spread, sharper with more minutes and a
/// sharper eye; the picked topic's minutes and its quiz read the same way.
/// The overall now is worked out again from the new centres.
pub(crate) fn file(w: &World, id: u32, f: &mut FileRead) {
    let Some(i) = crate::world::scouting::place(w, id) else { return };
    let mut sessions: Vec<(Date, u8, u64)> = w
        .folk
        .live
        .persona
        .draft
        .boards
        .iter()
        .filter(|b| b.id == id)
        .map(|b| (b.date, b.minutes, 0xB0A2 ^ b.got as u64))
        .collect();
    // The picked whiteboard topic, unless a typed session that day already
    // stands for it.
    for iv in w.scouting.own.interviews.iter().filter(|x| x.id == id) {
        let m = iv.spent.iter().find(|x| x.0 == crate::world::scouting::own::Topic::Whiteboard).map_or(0, |x| x.1);
        if m > 0 && !sessions.iter().any(|s| s.0 == iv.date) {
            sessions.push((iv.date, m, 0xB0A3));
        }
    }
    if sessions.is_empty() {
        return;
    }
    let p = &w.people[i];
    let sharpness = sharp(eye(w, i));
    for (date, minutes, salt) in sessions {
        for (k, &(a, base)) in SD.iter().enumerate() {
            let sd = base * (6.0 / minutes.max(1) as f64).sqrt() * sharpness;
            let truth = p.player.attributes.get(a);
            let v = sd * key_normal(salt ^ (date.0 as u64) << 16 ^ (k as u64) << 48, id as u64);
            let r = &mut f.attrs[a.index()];
            let (acc, prec) = if r.seen && r.sd > 0.0 {
                ((r.centre - truth) / (r.sd * r.sd), 1.0 / (r.sd * r.sd))
            } else {
                (0.0, 0.0)
            };
            let acc = acc + v / (sd * sd);
            let prec = prec + 1.0 / (sd * sd);
            r.centre = (truth + acc / prec).clamp(1.0, 99.0);
            r.sd = (1.0 / prec).sqrt();
            r.seen = true;
            r.guess = false;
            f.centres.set(a, r.centre);
        }
    }
    f.now.0 = crate::positions::overall_for(p.player.position, &f.centres);
}
