//! Your speech, man by man (6.12: "your speech hits each man on his own
//! things"). The ear names which of a man's real things your words touched
//! and how; the game turns that into his own word inside the speech
//! (`game::talk::Tilt`): the move a word to him alone would be, in the
//! speech's tone, and how much of his landing it takes. The landing itself
//! stays `game/talk.rs`'s (`land`, `room_tilted`), so it lies between the
//! room's and a chip's word to him: wording never beats the best chip.
//!
//! The same reading sizes a typed word on the sideline (D6): its move and
//! tone are the word, and the item it touched sets how much of it reaches
//! him (`word_share`).

use super::super::Them;
use super::super::ear::{How, Item};
use super::super::people::People;
use super::super::talk::hits;
use super::super::wants::Wanting;
use crate::game::talk::{Move, Pick, Subject, Tilt, Tone};
use crate::world::World;

/// The move a word to him alone is, for how your words touched him: the
/// built moves of `game/talk.rs` (reassurance is praise, respect is
/// straight talk, a sting or an insult is criticism, doubt is a
/// challenge, a threat is a warning).
pub fn move_of(how: How) -> Move {
    match how {
        How::Reassured | How::Flattered | How::Staked => Move::Praise,
        How::Respected => Move::Straight,
        How::Stung | How::Insulted => Move::Criticize,
        How::Doubted => Move::Challenge,
        How::Threatened => Move::Warn,
    }
}

/// The tone an insult forces: an insult is never calm.
fn tone_for(how: How, tone: Tone) -> Tone {
    if how == How::Insulted { Tone::Fiery } else { tone }
}

/// The most of his landing his own part of a speech takes. Proposed, not
/// confirmed.
pub const MAX_SHARE: f32 = 0.9;
/// The share of a sideline word that touched nothing real of his ("great
/// job, keep working"). Proposed, not confirmed.
pub const PLAIN_SHARE: f32 = 0.55;

/// How heavy an item is for him, 0 (the lightest, 0.4) to 1 (1.7 or
/// more): `hits::weight`. His belief in you is already in the landing
/// (`game::talk::land` reads it), so it isn't counted twice here.
pub fn heft(w: &World, id: u32, item: &Item) -> Option<f32> {
    let him = Them::Player(id);
    let wants = People.wants(w, him);
    let wt = hits::weight(w, him, item, &wants)?;
    Some(((wt - 0.4) / 1.3).clamp(0.0, 1.0))
}

/// Each man's own part of a speech: for every man the ear says your words
/// touched, the strongest touch sets his word and its share (0.2 for the
/// lightest thing of his, up to 0.8 for the heaviest), and each further
/// touch adds 0.1, up to `MAX_SHARE`. A touch on nothing real of his
/// counts for nothing (section 14).
pub fn tilts(w: &World, tone: Tone, touched: &[(u32, How, Item)]) -> Vec<Tilt> {
    let mut out: Vec<Tilt> = Vec::new();
    let mut seen: Vec<(u32, f32, usize)> = Vec::new();
    for (id, how, item) in touched {
        let Some(h) = heft(w, *id, item) else { continue };
        let s = 0.2 + 0.6 * h;
        let pick = Pick { mv: move_of(*how), tone: tone_for(*how, tone), subject: Subject::One, who: *id };
        match seen.iter_mut().find(|x| x.0 == *id) {
            Some(x) => {
                let t = &mut out[x.2];
                if s > x.1 {
                    t.pick = pick;
                    x.1 = s;
                }
                t.share = (t.share.max(x.1) + 0.1).min(MAX_SHARE);
            }
            None => {
                seen.push((*id, s, out.len()));
                out.push(Tilt { id: *id, pick, share: s.min(MAX_SHARE) });
            }
        }
    }
    out
}

/// How much of a typed sideline word reaches him: all of it when it
/// touched the heaviest thing of his, `PLAIN_SHARE` when it touched
/// nothing of his. Never more than the word itself.
pub fn word_share(w: &World, id: u32, item: Option<&Item>) -> f32 {
    match item.and_then(|i| heft(w, id, i)) {
        Some(h) => PLAIN_SHARE + (1.0 - PLAIN_SHARE) * (0.3 + 0.7 * h),
        None => PLAIN_SHARE,
    }
}
