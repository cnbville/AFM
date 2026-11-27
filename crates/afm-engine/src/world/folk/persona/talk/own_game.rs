//! How a player sees his own game, in words for the voice (talk replies
//! audit, 2026-11-27). Cain told a tackle with Communication 8 that his
//! communication wasn't good enough, and the tackle answered "I've been
//! calling the protections since I got here, that's news to me". The voice
//! card held his Football IQ in words and nothing else about how he plays,
//! so the voice made a game up and nothing judged what the coach said
//! about it.
//!
//! Now the card carries his own read of his game: each rating against the
//! typical starter at his spot at his level (`game::synthetic::typical_starter`),
//! the way he feels it. A man sees himself with some error: a proud man
//! reads himself a little high, and a low Football IQ blurs it (a keyed
//! draw each, so the same man reads himself the same way every talk). A
//! hole as big as a tackle with Communication 8 shows through any error,
//! so he knows it and has heard it before; a small gap can be a blind spot.
//!
//! Words only, never a number: the voice says "my communication", never a
//! rating. What isn't listed is about average for where he plays, and the
//! voice is told not to claim a job, a skill or a flaw the card doesn't
//! give him.
//!
//! Every size here is Claude's, proposed, not confirmed.

use super::super::Them;
use crate::attributes::{Attr, Category};
use crate::game::synthetic::typical_starter;
use crate::player::{Level, Trait};
use crate::positions::weight;
use crate::world::folk::{self as folk, persona::makeup};
use crate::world::{Stage, World};

/// A rating this far over the typical starter's at his spot (as he reads
/// it) is a strength, this far under a weak spot. Proposed, not confirmed.
pub const NOTICE: f64 = 9.0;
/// And a real strength, or a hole he can't miss. Proposed, not confirmed.
pub const BIG: f64 = 18.0;
/// The most of each he can name, the biggest first. Proposed, not
/// confirmed.
pub const MAX_EACH: usize = 4;
/// What a proud man adds to how he reads himself, in rating points at the
/// top of the ego scale (a humble man takes as much off). Proposed, not
/// confirmed.
pub const EGO_BIAS: f64 = 4.0;
/// How far off he reads each rating, in points: the most at a low Football
/// IQ, the least at a high one. Proposed, not confirmed.
pub const BLUR_MIN: f64 = 2.0;
pub const BLUR_MAX: f64 = 7.0;
/// The Football IQ rating that reads himself cleanest.
const SHARP_IQ: f64 = 80.0;

/// The level whose typical starter he reads himself against.
fn level_of(stage: Stage) -> Level {
    match stage {
        Stage::Pro => Level::NFL,
        Stage::HighSchool | Stage::PrepSchool => Level::HS,
        _ => Level::FBS,
    }
}

/// The ratings a man can feel in himself at his spot: everything physical
/// or mental, and any other the position counts.
fn felt(pos: crate::positions::Position, a: Attr) -> bool {
    matches!(a.category(), Category::Physical | Category::Mental) || weight(pos, a) > 0.0
}

/// How he reads each rating he can feel: its attribute and the gap in
/// points to the typical starter at his spot, as he sees it. The same man
/// reads the same gap every talk.
pub fn view(w: &World, i: usize) -> Vec<(Attr, f64)> {
    let p = &w.people[i];
    let level = level_of(p.stage);
    let pos = p.player.position;
    let iq = p.player.attributes.get(Attr::FootballIq);
    let blur = BLUR_MIN + (BLUR_MAX - BLUR_MIN) * (1.0 - (iq / SHARP_IQ).clamp(0.0, 1.0));
    let ego = p.player.personality.traits.z(Trait::Ego).clamp(-1.0, 1.0);
    let key = makeup::key(Them::Player(p.player.id)) ^ 0x0A71_6A3E;
    Attr::ALL
        .into_iter()
        .filter(|&a| felt(pos, a))
        .map(|a| {
            let gap = p.player.attributes.get(a) - typical_starter(level, pos, a);
            // The draw is held to two spreads either way: nobody misreads himself
            // by a tail of the bell curve.
            let error = blur * folk::roll_n(w, key, a.index() as u64).clamp(-2.0, 2.0) + EGO_BIAS * ego;
            (a, gap + error)
        })
        .collect()
}

/// The words a man names a rating by: "his hands", "his speed". The labels
/// are the screens' ("Football IQ", "Short Accuracy").
fn named(a: Attr) -> String {
    a.label().to_lowercase()
}

/// His own read of his game for the voice's card: what he knows he's good
/// at and what he knows he isn't, and that the rest is about average.
/// Empty for anyone who isn't a player on a roster.
pub fn words(w: &World, him: Them) -> Vec<String> {
    let Them::Player(id) = him else { return Vec::new() };
    let Some(i) = folk::person(w, id) else { return Vec::new() };
    let mut v = view(w, i);
    v.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut out = Vec::new();
    let good: Vec<String> = v
        .iter()
        .filter(|(_, g)| *g >= NOTICE)
        .take(MAX_EACH)
        .map(|(a, g)| if *g >= BIG { format!("{} (a real strength)", named(*a)) } else { named(*a) })
        .collect();
    if !good.is_empty() {
        out.push(format!("what you know you do well: {}", good.join(", ")));
    }
    let bad: Vec<String> = v
        .iter()
        .rev()
        .filter(|(_, g)| *g <= -NOTICE)
        .take(MAX_EACH)
        .map(|(a, g)| {
            if *g <= -BIG {
                format!("{} (a hole you know about, and you've heard it before)", named(*a))
            } else {
                named(*a)
            }
        })
        .collect();
    if !bad.is_empty() {
        out.push(format!("where you know you're weaker than the men at your spot: {}", bad.join(", ")));
    }
    out.push(
        "everything else you'd call about average for where you play, and you don't claim a skill, a job on the field or a flaw that isn't on this list"
            .into(),
    );
    out
}
