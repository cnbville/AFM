//! Facts about your own players in a typed line (live talks lane 9, LT1):
//! each looked up on his sheet, plus what the sheet lacks (snap share,
//! touches). Not built yet: every fact is unknown.

use crate::world::World;
use crate::world::folk::claims::truth::{Stated, Truth};
use crate::world::folk::scene::Scene;

pub fn check(_w: &World, _s: &Scene, _stated: &Stated) -> Truth {
    Truth::Unknown
}
