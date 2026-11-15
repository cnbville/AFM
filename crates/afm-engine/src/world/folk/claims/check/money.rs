//! Money facts in a typed line (live talks lane 7, LT7 and LT10): your
//! alternative at his spot, his production, his age, his injuries, an
//! off-field case (lane 8's record), other clubs' interest. For the GM and
//! boss scenes; the money desks' typed lines (`fo_say`) play on v3 as
//! negotiation calls (lane N5b) and recruit calls (lane N5a), checked by
//! those kinds' own rules in `check::fact`. Not built yet: every fact is
//! unknown.

use crate::world::World;
use crate::world::folk::claims::truth::{Stated, Truth};

pub fn check(_w: &World, _stated: &Stated) -> Truth {
    Truth::Unknown
}
