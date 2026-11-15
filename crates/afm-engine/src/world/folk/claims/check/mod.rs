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

/// What a talk held before a typed line landed, for `landed` (lane 10: a
/// recruit's case).
pub(crate) struct Before(Option<recruit::Before>);

/// Taken by `scene::say` just before a typed line lands.
pub(crate) fn before(w: &World, s: &Scene) -> Before {
    Before(recruit::before(w, s))
}

/// After a typed line landed (`scene::say`): what its facts cost or save,
/// in the talks that check them on the spot or on a day. A recruit's and a
/// transfer's are lane 10's (`recruit::landed`).
pub(crate) fn landed(w: &mut World, id: u32, facts: &[Stated], b: Before) {
    if let Some(r) = b.0 {
        recruit::landed(w, id, facts, r);
    }
}

/// Checks one fact stated in a typed line in a scene. A claim about where
/// your player stands is about what he'll see, so it reads "later": it is
/// heard as a promise and judged on its days (the promise design, section
/// 11), never against the staff's chart.
pub fn fact(w: &World, s: &Scene, stated: &Stated) -> Truth {
    if let Some(him) = super::truth::your_player(w, s)
        && super::truth::is_standing(stated, him)
    {
        return Truth::Later;
    }
    match s.kind {
        Kind::HomeVisit
        | Kind::PortalCall
        | Kind::PortalVisit
        | Kind::SigneeRelease
        | Kind::RecruitCall
        | Kind::RecruitMessage
        | Kind::FamilyCall
        | Kind::OfficialTalk => recruit::check(w, s, stated),
        Kind::Gm | Kind::BossFirst | Kind::BossMeeting => money::check(w, stated),
        // You, the high school coach, about your player to a college staff
        // (live talks v3, lane N5a).
        Kind::CollegeCall => crate::world::folk::persona::talk::hs_call::check(w, s, stated),
        _ => roster::check(w, s, stated),
    }
}
