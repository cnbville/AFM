//! Heat and patience during a talk (lane N1, section 3.5; section 11 for
//! asking again and lowballs). The game decides both from his makeup; the
//! voice only says what comes of them.
//!
//! - Heat (0 calm, 1 past his point) rises with what stirs him (an insult,
//!   a lie he catches, a promise you broke, a threat, a lowball), by his
//!   temper, his ego for what hits his pride, and less for a pro. It cools
//!   a little every minute, more with an apology he believes. Past his
//!   point he goes quiet, raises his voice or walks out (`Boil`).
//! - Patience (1 fresh, 0 done) wears with every minute that goes nowhere,
//!   faster for a hot or proud man, and with every time you ask again
//!   after a no. An agent's or a cast person's own patience seeds it.
//!
//! Every size here is Claude's (proposed, not confirmed).

use super::Them;
use super::makeup::{self, Makeup};
use super::wants::{Boil, Heat, Patience};
use crate::world::World;

/// What stirs him, or calms him.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stir {
    /// An insult: his pride.
    Insult,
    /// Something that stung: a hard truth about his play.
    Sting,
    /// He caught you in a lie.
    CaughtLie,
    /// A promise you broke came up.
    BrokenPromise,
    /// A threat or an ultimatum.
    Threat,
    Ultimatum,
    /// A deal far under his worth.
    Lowball,
    /// You asked again after a no.
    AskedAgain,
    /// You dodged his question.
    Dodged,
    /// You keep pushing him on someone.
    Grind,
    /// An apology he believes.
    Apology,
    /// Respect, reassurance: it cools a little.
    Respect,
}

/// How hard each stirs him before his makeup, as heat (a negative cools).
pub fn base(s: Stir) -> f32 {
    match s {
        Stir::Insult => 0.35,
        Stir::Sting => 0.12,
        Stir::CaughtLie => 0.3,
        Stir::BrokenPromise => 0.25,
        Stir::Threat => 0.2,
        Stir::Ultimatum => 0.3,
        Stir::Lowball => 0.18,
        Stir::AskedAgain => 0.08,
        Stir::Dodged => 0.06,
        Stir::Grind => 0.07,
        Stir::Apology => -0.2,
        Stir::Respect => -0.05,
    }
}

/// How much of a stir lands as heat for this man: his temper most, his ego
/// for what hits his pride, less for a pro.
pub fn scale(m: &Makeup, s: Stir) -> f32 {
    let z = |v: f64| Makeup::z(v) as f32;
    let pride = matches!(s, Stir::Insult | Stir::Sting | Stir::Lowball | Stir::Threat | Stir::Ultimatum);
    let up = 1.0 + 0.7 * z(m.temper) + if pride { 0.5 * z(m.ego) } else { 0.0 } - 0.35 * z(m.professionalism);
    match s {
        // An apology cools a warm man more, a proud one less.
        Stir::Apology | Stir::Respect => (1.0 + 0.4 * z(m.warmth) - 0.3 * z(m.ego)).clamp(0.3, 1.6),
        _ => up.clamp(0.25, 2.2),
    }
}

/// The heat after a stir.
pub fn stir(m: &Makeup, heat: Heat, s: Stir) -> Heat {
    Heat((heat.0 + base(s) * scale(m, s)).clamp(0.0, 1.0))
}

/// Heat cools by this share a minute, faster for a calm man.
pub fn cool(m: &Makeup, heat: Heat, minutes: f32) -> Heat {
    let rate = (0.04 - 0.02 * Makeup::z(m.temper) as f32).clamp(0.01, 0.07);
    Heat(heat.0 * (1.0 - rate).powf(minutes.max(0.0)))
}

/// His point: the heat past which he boils over. A pro holds longer, a
/// hothead shorter.
pub fn point(m: &Makeup) -> f32 {
    (0.7 + 0.15 * Makeup::z(m.professionalism) as f32 - 0.2 * Makeup::z(m.temper) as f32).clamp(0.4, 0.95)
}

/// What he does past his point (the game decides, never the voice): a
/// proud man well past it walks out, a hot one gets loud, the rest go
/// quiet.
pub fn boil(m: &Makeup, heat: Heat) -> Option<Boil> {
    let p = point(m);
    if heat.0 < p {
        return None;
    }
    let past = heat.0 - p;
    if past >= 0.15 && m.ego >= 12.0 || heat.0 >= 0.98 {
        Some(Boil::WalksOut)
    } else if m.temper >= 12.0 {
        Some(Boil::Loud)
    } else {
        Some(Boil::Quiet)
    }
}

/// His patience at the start of a talk: his own (a cast person's and an
/// agent's are kept), lower if he's already hot with you from last time.
pub fn start(w: &World, him: Them) -> Patience {
    let m = makeup::makeup(w, him);
    let mut p = (0.55 + 0.45 * (m.patience / 20.0)) as f32;
    // A talk that went hot lately costs some of it (3.6: how you treated
    // him).
    let hot = w
        .folk
        .live
        .persona
        .treated
        .iter()
        .filter(|t| t.who == him && t.day.days_to(w.today) <= 30)
        .filter(|t| matches!(t.how, super::memory::Treated::Hot | super::memory::Treated::Lowballed))
        .count();
    p -= 0.1 * hot.min(3) as f32;
    Patience(p.clamp(0.2, 1.0))
}

/// How fast a talk that goes nowhere wears him, a minute.
pub fn wear_rate(m: &Makeup) -> f32 {
    let z = |v: f64| Makeup::z(v) as f32;
    (0.012 * (1.0 - 0.8 * z(m.patience) + 0.3 * z(m.ego) + 0.3 * z(m.temper))).clamp(0.003, 0.03)
}

/// His patience after some minutes: wearing while nothing moves, holding
/// (a little back) while the talk gets somewhere.
pub fn wear(m: &Makeup, p: Patience, minutes: f32, progress: bool) -> Patience {
    let v = if progress { p.0 + 0.01 * minutes.min(5.0) } else { p.0 - wear_rate(m) * minutes.max(0.0) };
    Patience(v.clamp(0.0, 1.0))
}

/// Asking again after a no costs patience (section 11), more each time.
pub fn asked_again(m: &Makeup, p: Patience, times: u8) -> Patience {
    let cost = 0.08 * times.max(1) as f32 * (1.0 + 0.4 * Makeup::z(m.ego) as f32).max(0.4);
    Patience((p.0 - cost).clamp(0.0, 1.0))
}

/// He's done: out of patience, he ends the talk.
pub fn done(p: Patience) -> bool {
    p.0 <= 0.05
}
