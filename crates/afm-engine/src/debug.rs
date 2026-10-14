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

/// Engine teams captured as each season kicks off, for the years listed in
/// `AFM_CAPTURE` (comma-separated): (season, every program's engine team,
/// every program's lineup strength as the world rates it).
pub type Captured = (i32, Vec<Option<crate::game::GameTeam>>, Vec<f64>);

static CAPTURED: std::sync::Mutex<Vec<Captured>> = std::sync::Mutex::new(Vec::new());

pub(crate) fn capture_teams(year: i32, teams: &[Option<crate::game::GameTeam>], strength: &[f64]) {
    let wanted = std::env::var("AFM_CAPTURE").is_ok_and(|v| v.split(',').any(|y| y.trim() == year.to_string()));
    if wanted && !teams.is_empty() {
        CAPTURED.lock().unwrap().push((year, teams.to_vec(), strength.to_vec()));
    }
}

/// Takes the captured seasons.
pub fn captured() -> Vec<Captured> {
    std::mem::take(&mut CAPTURED.lock().unwrap())
}
