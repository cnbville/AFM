//! Debug and trial access to what the game hides: true potential and
//! ceilings. Only compiled with the `debug` feature, which the game itself
//! must never enable.

use crate::attributes::Attributes;
use crate::career::CareerRecord;
use crate::development::SeasonReport;
use crate::player::{Player, SeasonRecord};

/// True potential: the overall the player would have if every attribute reached its ceiling.
pub fn potential(p: &Player) -> f64 {
    p.potential()
}

/// Hidden ceiling per attribute.
pub fn ceilings(p: &Player) -> &Attributes {
    &p.ceilings
}

/// Mutable hidden ceilings, for trial setups.
pub fn ceilings_mut(p: &mut Player) -> &mut Attributes {
    &mut p.ceilings
}

/// Potential when the player was generated.
pub fn original_potential(p: &Player) -> f64 {
    p.original_potential
}

/// True potential at the end of a season.
pub fn season_potential(s: &SeasonRecord) -> f64 {
    s.potential
}

/// True potential at 14.
pub fn initial_potential(r: &CareerRecord) -> f64 {
    r.initial_potential
}

/// True potential when he left college (or high school).
pub fn potential_at_exit(r: &CareerRecord) -> f64 {
    r.potential_at_exit
}

/// True potential before and after a development step.
pub fn report_potential(r: &SeasonReport) -> (f64, f64) {
    (r.potential_before, r.potential_after)
}

/// True potential of a player who left the world for the pros.
pub fn alumnus_potential(a: &crate::world::Alumnus) -> f64 {
    a.potential
}

/// A world player's true potential.
pub fn person_potential(p: &crate::world::Person) -> f64 {
    p.player.potential()
}

/// How visible a departed player was to recruiters as a senior (hidden in game).
pub fn alumnus_visibility(a: &crate::world::Alumnus) -> f64 {
    a.visibility
}

/// A coach's hidden ability.
pub fn coach_ability(c: &crate::world::coach::Coach) -> crate::world::coach::Ability {
    *c.ability()
}

/// A world player's hidden background and family.
pub fn person_background(
    p: &crate::world::Person,
) -> (&crate::world::background::Background, &crate::world::background::Family) {
    (&p.bg, &p.family)
}
