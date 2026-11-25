//! Heat and patience during a talk (section 3.5). `Sized` plays them on
//! lane N1's sizes (`persona::feel`, from his makeup); `Plain` is the old
//! stand-in, kept for tests. Every number is proposed, not confirmed. The
//! game decides when he boils over, never the voice.

use super::super::Them;
use super::super::ear::{How, Turn};
use super::super::feel::{self as n1, Stir};
use super::super::makeup;
use super::super::terms::Weighed;
use super::super::wants::{Boil, Heat, Patience, Willing};
use super::store::Open;
use crate::player::Trait;
use crate::world::World;

/// Heat at which he boils over. Proposed, not confirmed.
pub const BOIL: f32 = 1.0;
/// Heat at which he walks out instead. Proposed, not confirmed.
pub const WALK: f32 = 1.35;
/// What a calm line takes off the heat. Proposed, not confirmed.
pub const COOL: f32 = 0.12;
/// What every line takes off his patience. Proposed, not confirmed.
pub const WEAR: f32 = 0.05;
/// More when nothing landed. Proposed, not confirmed.
pub const NOWHERE: f32 = 0.07;
/// Asking again after a no (section 11). Proposed, not confirmed.
pub const ASKED_AGAIN: f32 = 0.15;
/// How fast a line that went nowhere on a question he dodged or you'd
/// already asked wears him, against N1's rate, with your Relationships at
/// the top of the scale (20). Cain 2026-11-15: "we'll review the numbers
/// later": proposed, not confirmed.
pub const REPEAT_TOP: f32 = 0.5;
/// The same with your Relationships at the bottom (1). Proposed, not
/// confirmed.
pub const REPEAT_BOTTOM: f32 = 2.0;
/// Where your Relationships changes nothing: the middle of 1-20.
pub const REPEAT_MID: f32 = 10.5;

/// Whether a line got anywhere (section 14: "grinding costs time and
/// trust").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Progress {
    /// Something moved: a hit landed, or a fresh question got an answer.
    Landed,
    /// Nothing moved.
    Nowhere,
    /// Nothing moved, and the line was a question he dodged or you'd
    /// already asked: it wears him by your Relationships (`repeat_wear`).
    Grind,
}

/// How much faster than N1's rate a grinding line wears him, by your own
/// Relationships (1-20): `REPEAT_BOTTOM` at 1, 1 at `REPEAT_MID`,
/// `REPEAT_TOP` at 20, even steps on a log scale between. A coach who's good
/// with people can ask twice without annoying him.
pub fn repeat_wear(relationships: f32) -> f32 {
    let r = relationships.clamp(1.0, 20.0);
    if r >= REPEAT_MID {
        REPEAT_TOP.powf((r - REPEAT_MID) / (20.0 - REPEAT_MID))
    } else {
        REPEAT_BOTTOM.powf((REPEAT_MID - r) / (REPEAT_MID - 1.0))
    }
}

/// Your Relationships rating (1-20): your coach's in the world, 10 with no
/// career.
pub fn your_relationships(w: &World) -> f32 {
    w.your_ability().map_or(10.0, |a| a.relationships) as f32
}

/// How a talk heats him up and wears him down (lane N1's to size).
pub trait Feeling: Sync {
    /// The heat after this line: insults, threats, a lie he catches as you
    /// say it, by his temper.
    fn heat(&self, w: &World, him: Them, open: &Open, t: &Turn, caught: usize) -> Heat;
    /// His patience after this line.
    fn patience(&self, w: &World, him: Them, open: &Open, t: &Turn, progress: Progress) -> Patience;
    /// What he does past his point, if he's there.
    fn boil(&self, w: &World, him: Them, open: &Open, heat: Heat) -> Option<Boil>;
    /// The heat at which he boils over.
    fn point(&self, _w: &World, _him: Them) -> f32 {
        BOIL
    }
    /// His patience as the talk opens.
    fn start(&self, _w: &World, _him: Them) -> Patience {
        Patience::default()
    }
}

/// Heat and patience on lane N1's sizes: what stirs him by his temper, ego
/// and professionalism, cooling by the minute; patience wearing while the
/// talk goes nowhere (faster or slower on a grind, by your Relationships)
/// and with each time you ask again after a no.
pub struct Sized;

/// What each hit on him stirs (N1's `Stir`).
fn stirs(him: Them, t: &Turn, caught: usize) -> Vec<Stir> {
    use super::super::ear::Item;
    let mut out = Vec::new();
    for h in t.hits.iter().filter(|h| h.hit.on == him) {
        let promise = matches!(h.hit.item, Item::Promise(_));
        match h.hit.how {
            How::Insulted => out.push(Stir::Insult),
            How::Threatened => out.push(Stir::Threat),
            How::Stung => out.push(Stir::Sting),
            How::Doubted if h.hit.item == Item::HisWord => out.push(Stir::Dodged),
            How::Reassured if promise && h.trust > 0.0 => out.push(Stir::Apology),
            How::Reassured | How::Respected if h.trust > 0.0 => out.push(Stir::Respect),
            _ => {}
        }
        if promise && matches!(h.hit.how, How::Insulted | How::Stung) {
            out.push(Stir::BrokenPromise);
        }
    }
    out.extend(std::iter::repeat_n(Stir::CaughtLie, caught));
    out.extend(t.weighed.iter().filter(|(_, x)| *x == Weighed::Lowball).map(|_| Stir::Lowball));
    out
}

impl Feeling for Sized {
    fn heat(&self, w: &World, him: Them, open: &Open, t: &Turn, caught: usize) -> Heat {
        let m = makeup::makeup(w, him);
        let mut h = n1::cool(&m, open.heat, t.minutes);
        for s in stirs(him, t, caught) {
            h = n1::stir(&m, h, s);
        }
        h
    }

    fn patience(&self, w: &World, him: Them, open: &Open, t: &Turn, progress: Progress) -> Patience {
        let m = makeup::makeup(w, him);
        let minutes = match progress {
            Progress::Grind => t.minutes * repeat_wear(your_relationships(w)),
            _ => t.minutes,
        };
        let mut p = n1::wear(&m, open.patience, minutes, progress == Progress::Landed);
        let again = t.asked.iter().filter(|(r, wl, _)| *wl == Willing::No && open.refused_before(r)).count();
        if again > 0 {
            p = n1::asked_again(&m, p, again.min(255) as u8);
        }
        if n1::done(p) { Patience(0.0) } else { p }
    }

    fn boil(&self, w: &World, him: Them, _open: &Open, heat: Heat) -> Option<Boil> {
        n1::boil(&makeup::makeup(w, him), heat)
    }

    fn point(&self, w: &World, him: Them) -> f32 {
        n1::point(&makeup::makeup(w, him))
    }

    fn start(&self, w: &World, him: Them) -> Patience {
        n1::start(w, him)
    }
}

/// The stand-in sizes.
pub struct Plain;

impl Feeling for Plain {
    fn heat(&self, w: &World, him: Them, open: &Open, t: &Turn, caught: usize) -> Heat {
        let temper = super::z(w, him, Trait::Temper);
        let prof = super::z(w, him, Trait::Professionalism);
        let mut up = 0.0f32;
        for h in t.hits.iter().filter(|h| h.hit.on == him) {
            up += match h.hit.how {
                How::Insulted => 0.32,
                How::Threatened => 0.22,
                How::Stung => 0.08,
                _ => 0.0,
            };
            if matches!(h.hit.item, super::super::ear::Item::Promise(_))
                && matches!(h.hit.how, How::Insulted | How::Stung)
            {
                up += 0.08;
            }
        }
        up += 0.25 * caught as f32;
        let k = (1.0 + 0.35 * temper - 0.15 * prof).clamp(0.4, 1.8);
        let calm = if up == 0.0 { COOL } else { 0.0 };
        let apology =
            t.hits.iter().any(|h| h.hit.on == him && h.hit.how == How::Reassured && h.trust > 0.0) as u8 as f32;
        Heat((open.heat.0 + up * k - calm - 0.05 * apology).max(0.0))
    }

    fn patience(&self, w: &World, him: Them, open: &Open, t: &Turn, progress: Progress) -> Patience {
        let prof = super::z(w, him, Trait::Professionalism);
        let k = (1.0 - 0.2 * prof).clamp(0.6, 1.4);
        let wear = WEAR
            + match progress {
                Progress::Landed => 0.0,
                Progress::Nowhere => NOWHERE,
                Progress::Grind => NOWHERE * repeat_wear(your_relationships(w)),
            };
        // Asking again after a no was charged where the request was read.
        let again = (open.patience.0 - t.patience.0).max(0.0);
        Patience((open.patience.0 - again - wear * k).min(1.0))
    }

    fn boil(&self, w: &World, him: Them, open: &Open, heat: Heat) -> Option<Boil> {
        if heat.0 < BOIL {
            return None;
        }
        let temper = super::z(w, him, Trait::Temper);
        Some(if heat.0 >= WALK || (open.boiled == Some(Boil::Loud) && temper > 0.5) {
            Boil::WalksOut
        } else if temper > 0.3 {
            Boil::Loud
        } else {
            Boil::Quiet
        })
    }
}
