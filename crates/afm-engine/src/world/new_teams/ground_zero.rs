//! Ground zero (SK68): a start, or a later job, at a team that doesn't
//! exist yet. An NFL expansion club in its year before the first season (a
//! front office, a coaching staff, the other clubs to scout before the
//! expansion draft); a college in its build year (practices and no games,
//! walk-on tryouts, junior college and portal finds, a first class signed
//! with nothing on the field); a new high school the district opens this
//! August (a zone to scout, families who choose, freshmen and sophomores
//! on the first varsity schedule).

use crate::rng::Rng;
use crate::world::World;
use crate::world::data::Division;

/// The markets a new club could go to, best first (indices into
/// `nfl::MARKETS`).
pub fn nfl_markets(w: &World) -> Vec<usize> {
    let mut v = super::nfl::open_markets(w);
    v.sort_by(|&a, &b| super::nfl::MARKETS[b].2.total_cmp(&super::nfl::MARKETS[a].2));
    v
}

/// An expansion club for your start: the owners award it with a partner
/// (the league adds two to keep the schedule even, as in 1995), and both
/// play next season. Returns your club's index.
pub fn nfl_club(w: &mut World, market: usize) -> usize {
    let mut rng = Rng::new(w.options.seed ^ 0x6200_0E11);
    let others: Vec<(usize, f64)> = super::nfl::open_markets(w)
        .into_iter()
        .filter(|&k| k != market)
        .map(|k| (k, super::nfl::MARKETS[k].2.powi(2)))
        .collect();
    let mut picked = vec![market];
    if !others.is_empty() {
        picked.push(rng.weighted(&others));
    }
    let first = w.year + 1;
    let teams = super::nfl::award(w, &picked, first);
    let names: Vec<&str> = teams.iter().map(|&t| w.nfl[t].name).collect();
    let text = format!(
        "The owners award expansion clubs to {}: the {} play from {first}, with an expansion draft that February and extra picks in the draft.",
        picked.iter().map(|&k| super::nfl::MARKETS[k].0).collect::<Vec<_>>().join(" and "),
        names.join(" and the "),
    );
    let d = w.today;
    w.expansion.votes.push((w.year, true));
    w.calendar.post(
        crate::calendar::Event::new(
            d,
            crate::calendar::Level::Nfl,
            crate::calendar::Kind::RuleChange,
            crate::calendar::Loudness::Huge,
            text,
        )
        .team(teams[0])
        .memorable(),
    );
    teams[0]
}

/// Colleges that could start football now, by level: programs without a
/// team that aren't starting already (the biggest first).
pub fn college_candidates(w: &World) -> Vec<usize> {
    let y = w.year;
    let mut v: Vec<usize> = (0..w.programs.len())
        .filter(|&p| {
            let x = &w.programs[p];
            !x.plays(y) && !x.plays(y + 1) && w.new_teams.program(p).is_none()
        })
        .collect();
    v.sort_by(|&a, &b| w.programs[b].enrollment.cmp(&w.programs[a].enrollment).then(a.cmp(&b)));
    v
}

/// The level a school starts at: a Division I school at FCS, the rest at
/// their own.
pub fn start_level(w: &World, p: usize) -> Division {
    match w.programs[p].division {
        Division::FBS | Division::FCS => Division::FCS,
        d => d,
    }
}

/// A college for your start: its build year is this season (practices and
/// no games) and it plays next season.
pub fn college(w: &mut World, p: usize) {
    let level = start_level(w, p);
    super::college::start(w, p, level, w.year, w.year + 1);
}

/// Schools a district could split this year: the big public schools in
/// the states whose young population grows.
pub fn school_candidates(w: &World) -> Vec<usize> {
    let mut v: Vec<usize> = (0..w.high_schools.len())
        .filter(|&h| {
            let x = &w.high_schools[h];
            !x.private && x.enrollment >= 1200 && super::school::young_growth(x.state) > 0.0
        })
        .collect();
    v.sort_by(|&a, &b| w.high_schools[b].enrollment.cmp(&w.high_schools[a].enrollment).then(a.cmp(&b)));
    v
}

/// A new high school for your start: it splits `parent`'s zone and opens
/// this August. Returns the new school's index.
pub fn school(w: &mut World, parent: usize) -> usize {
    let mut rng = Rng::new(w.options.seed ^ 0x6200_5C40 ^ parent as u32);
    let year = w.year;
    let h = super::school::open(w, parent, year, &mut rng);
    // Its own book and season entries: the season states were built at the
    // year's start, before it existed.
    super::super::hs_year::add_school(w, h);
    h
}

/// Your staff's guess at each old club's open players before the lists
/// come in: the same rule the clubs use, on your staff's grades of their
/// players (pro scouting reads): (club, person index, your staff's grade).
pub fn open_guess(w: &World, club: usize) -> Vec<(usize, usize, f64)> {
    let new = super::nfl::drafting_or_pending(w);
    super::nfl::open_lists(w, &new, |_, i| crate::world::pro::grade(w, club, i))
        .into_iter()
        .map(|(t, i, _)| (t, i, crate::world::pro::grade(w, club, i)))
        .collect()
}

/// Your staff's grade of a player, for a list on your screens.
pub fn staff_grade(w: &World, club: usize, i: usize) -> f64 {
    crate::world::pro::grade(w, club, i)
}
