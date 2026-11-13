//! Checking a typed fact (live talks, the core): one door, `fact`, and one
//! file for each lane that fills a kind of fact. A fact is checked by
//! finding its point on the fact sheet; each file adds only what the sheet
//! lacks.
//!
//! - `roster`: your own players (lane 9: snap share, touches).
//! - `recruit`: a recruit or a transfer (lane 10: the depth chart he will
//!   see, a NIL number at signing).
//! - `money`: contract talks and trade calls (lane 7: your alternative at
//!   his spot, his production, his age, his injuries, an off-field case,
//!   other clubs' interest). It needs nothing from the sheet.
//!
//! Until a lane fills its file, every fact reads `Truth::Unknown` and counts
//! for nothing, as a fact that isn't on the sheet does.

pub mod money;
pub mod recruit;
pub mod roster;

use super::truth::{Stated, Truth};
use crate::world::World;
use crate::world::folk::scene::{Kind, Scene};

/// Checks one fact stated in a typed line in a scene.
pub fn fact(w: &World, s: &Scene, stated: &Stated) -> Truth {
    match s.kind {
        Kind::HomeVisit | Kind::PortalCall | Kind::PortalVisit | Kind::SigneeRelease => recruit::check(w, s, stated),
        Kind::Gm | Kind::BossFirst | Kind::BossMeeting => money::check(w, stated),
        _ => roster::check(w, s, stated),
    }
}
