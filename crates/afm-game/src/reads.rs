//! What your staff knows (scouting topic 1, screens topic 4): every rating
//! the screens show is a range your staff reads, never the true number.
//!
//! Proposed, not confirmed (the scouting files for your own seat come with
//! section 7's own side): your coaches see your own players every day, so
//! their range is narrow, 4-8 points on 1-99 by the coach's eye; any other
//! team's player shows only what is public (his measurables, his stars or
//! level rating) and "Not seen" for the rest, until your scouts look.
//! Each reader's lean is his own and lasting (a fixed offset per player and
//! attribute), so a range doesn't jump from day to day.

use afm_engine::attributes::{Attr, Category};
use afm_engine::player::Player;
use afm_engine::scouting::{ScoutOptions, potential_outlook};

/// A read of one rating: low and high, or not seen.
#[derive(Clone, Copy, Debug)]
pub enum Read {
    Range(u8, u8),
    NotSeen,
}

impl Read {
    pub fn json(self) -> serde_json::Value {
        match self {
            Read::Range(lo, hi) => serde_json::json!([lo, hi]),
            Read::NotSeen => serde_json::Value::Null,
        }
    }
}

/// A fixed lean in -1..1 for a reader, player and attribute.
fn lean(reader: u64, player: u32, attr: usize) -> f64 {
    let mut z = reader ^ (player as u64) << 20 ^ (attr as u64) << 52 ^ 0xA5F0_1C3D;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
}

/// A range around a true value, `half` wide each side, leaning by the reader.
pub fn range(truth: f64, half: f64, reader: u64, player: u32, attr: usize) -> Read {
    let centre = truth + lean(reader, player, attr) * half * 0.6;
    let lo = (centre - half).round().clamp(1.0, 99.0) as u8;
    let hi = (centre + half).round().clamp(1.0, 99.0) as u8;
    Read::Range(lo, hi.max(lo))
}

/// How your staff reads a player: `own` for your own roster, with the eye
/// of the coach who reads him (1-20).
pub fn attributes(p: &Player, own: bool, eye: f64, reader: u64) -> Vec<(Attr, Read)> {
    let half = 2.0 + (20.0 - eye.clamp(1.0, 20.0)) * 0.12;
    Attr::ALL
        .iter()
        .map(|&a| {
            let truth = p.attributes.get(a);
            let read = if own {
                range(truth, half, reader, p.id, a.index())
            } else if a.category() == Category::Physical {
                // Measurables are public; what they mean on the field is read.
                range(truth, half + 4.0, reader, p.id, a.index())
            } else {
                Read::NotSeen
            };
            (a, read)
        })
        .collect()
}

/// The overall read: narrow for your own players, wide for anyone else.
pub fn overall(p: &Player, own: bool, eye: f64, reader: u64) -> Read {
    let half = if own { 2.0 + (20.0 - eye.clamp(1.0, 20.0)) * 0.12 } else { 6.0 };
    range(p.overall(), half, reader, p.id, 99)
}

/// Potential in words only (scouting topic 7): a tier, how sure your staff
/// is, and hints. Drawn from a fixed stream per reader and season, so the
/// words hold until your staff looks again.
pub fn potential_words(
    p: &Player,
    eye: f64,
    level: afm_engine::player::Level,
    reader: u64,
    season: i32,
) -> serde_json::Value {
    let mut rng = afm_engine::rng::Rng::new((reader as u32) ^ p.id.wrapping_mul(0x9E37) ^ season as u32);
    let o = potential_outlook(p, ScoutOptions { accuracy: eye, level }, &mut rng);
    serde_json::json!({
        "tier": o.tier.phrase(),
        "sure": format!("{:?}", o.confidence),
        "hints": o.hints.iter().map(|h| h.phrase()).collect::<Vec<_>>(),
    })
}
