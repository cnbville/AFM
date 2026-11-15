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

/// How a talk heats him up and wears him down (lane N1's to size).
pub trait Feeling: Sync {
    /// The heat after this line: insults, threats, a lie he catches as you
    /// say it, by his temper.
    fn heat(&self, w: &World, him: Them, open: &Open, t: &Turn, caught: usize) -> Heat;
    /// His patience after this line.
    fn patience(&self, w: &World, him: Them, open: &Open, t: &Turn, landed: bool) -> Patience;
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
/// talk goes nowhere and with each time you ask again after a no.
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

    fn patience(&self, w: &World, him: Them, open: &Open, t: &Turn, landed: bool) -> Patience {
        let m = makeup::makeup(w, him);
        let mut p = n1::wear(&m, open.patience, t.minutes, landed);
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

    fn patience(&self, w: &World, him: Them, open: &Open, t: &Turn, landed: bool) -> Patience {
        let prof = super::z(w, him, Trait::Professionalism);
        let k = (1.0 - 0.2 * prof).clamp(0.6, 1.4);
        let wear = WEAR + if landed { 0.0 } else { NOWHERE };
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
