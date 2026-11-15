//! Money facts in a typed line (live talks lane 7, LT7 and LT10): your
//! alternative at his spot, his production, his age, his injuries, an
//! off-field case (lane 8's record), other clubs' interest. Contract talks
//! and trade calls are not scenes, so lane 7 calls `check` from `fo_say`
//! too. Not built yet: every fact is unknown.

use crate::world::World;
use crate::world::folk::claims::truth::{Stated, Truth};

pub fn check(_w: &World, _stated: &Stated) -> Truth {
    Truth::Unknown
}
