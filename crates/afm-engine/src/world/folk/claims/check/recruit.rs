//! Facts to a recruit or a transfer in a typed line (live talks lane 10,
//! LT2, LT3 and LT9): each looked up on his sheet, with the day he can see
//! one he can't check yet. Not built yet: every fact is unknown.

use crate::world::World;
use crate::world::folk::claims::truth::{Stated, Truth};
use crate::world::folk::scene::Scene;

pub fn check(_w: &World, _s: &Scene, _stated: &Stated) -> Truth {
    Truth::Unknown
}
