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

/// A coach's hidden sportsmanship (dynasty item 3), 1-20.
pub fn coach_sportsmanship(c: &crate::world::coach::Coach) -> f64 {
    c.sportsmanship
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

/// A world player's hidden reads (the world reader, lane R2): how visible
/// he is to recruiters, his grades' trend, and the role he left when he
/// transferred.
pub fn person_hidden(p: &crate::world::Person) -> (f64, f64, Option<crate::player::PlayingTime>) {
    (p.visibility, p.gpa_trend, p.transfer_from)
}

/// A coach's hidden learning ceiling, home state and the age his decline
/// starts from.
pub fn coach_hidden(c: &crate::world::coach::Coach) -> (f64, Option<usize>, f64) {
    (c.learning_ceiling, c.home_state, c.decline_from)
}

/// The world's inner stores that `world_store` reads.
pub const WORLD_STORES: &[&str] = &[
    "college",
    "college_prev",
    "hs_season",
    "nfl_season",
    "nfl_prev",
    "prev_polls",
    "searches",
    "deals",
    "realign",
    "portal_pending",
    "portal_market",
    "announcements",
    "moved_cycle",
    "kept_schedules",
    "college_spread",
    "draft_bars",
    "system_norms",
    "novelty",
    "prev_dates",
    "coach_window",
    "recording",
];

/// One of the world's inner stores (`WORLD_STORES`) as JSON, for the world
/// reader: the season states, polls, searches and the rest the screens
/// never show whole. Map keys that are not text become their JSON text.
pub fn world_store(w: &crate::world::World, name: &str) -> Option<serde_json::Value> {
    fn json<T: serde::Serialize + ?Sized>(v: &T) -> Option<serde_json::Value> {
        let bytes = rmp_serde::to_vec_named(v).ok()?;
        rmp_serde::from_slice::<crate::world::wlog::read::Any>(&bytes).ok().map(|a| a.0)
    }
    match name {
        "college" => json(&w.college),
        "college_prev" => json(&w.college_prev),
        "hs_season" => json(&w.hs_season),
        "nfl_season" => json(&w.nfl_season),
        "nfl_prev" => json(&w.nfl_prev),
        "prev_polls" => json(&w.prev_polls),
        "searches" => json(&w.searches),
        "deals" => json(&w.deals),
        "realign" => json(&w.realign),
        "portal_pending" => json(&w.portal_pending),
        "portal_market" => json(&w.portal_market),
        "announcements" => json(&w.announcements),
        "moved_cycle" => json(&w.moved_cycle),
        "kept_schedules" => json(&w.kept_schedules),
        "college_spread" => json(&w.college_spread),
        "draft_bars" => json(&w.draft_bars),
        "system_norms" => json(&w.system_norms),
        "novelty" => json(&w.novelty),
        "prev_dates" => json(&w.prev_dates),
        "coach_window" => json(&w.coach_window),
        "recording" => json(&w.recording),
        _ => None,
    }
}

/// A school's appeal to a recruit, term by term (dynasty thread trial
/// tooling): see `world::recruiting::debug_terms`.
pub fn recruit_terms(w: &crate::world::World, person: usize, program: usize) -> [f64; 21] {
    crate::world::recruiting::debug_terms(w, person, program)
}
