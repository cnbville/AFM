//! Chemistry between players who've played together (X5): a quarterback
//! and his receivers learn each other's timing, and a secondary learns to
//! pass men off and talk through the checks. Built from the seasons each
//! pair spent on the same team (and this one so far), so a new receiver or
//! a rebuilt secondary starts cold and warms over a season or two (real:
//! timing between a quarterback and a new receiver takes about a season of
//! reps; teams returning their secondary blow fewer coverages).
//!
//! The engine reads it as each player's `chem` (0-1): a receiver's with his
//! starting quarterback, the quarterback's with his top three targets, a
//! defensive back's with the other starting defensive backs. Typical pairs
//! sit near 0.45, where nothing changes; a cold pair throws a little later
//! and wider and busts more option routes, a warm one throws on time
//! (proposed, not confirmed: at most a few points of completion rate).

use crate::game::roster::{GamePlayer, GameTeam};
use crate::positions::Position;
use crate::world::{Person, Stage, World};

/// What a team's games count as for chemistry.
#[derive(Clone, Copy)]
pub(crate) enum Unit {
    College,
    Nfl,
    HighSchool,
}

/// Seasons a pair spent on the same team before this one, plus half a
/// season for this one when both are here (spring, camp and the games so far).
fn seasons_together(w: &World, a: &Person, b: &Person, unit: Unit) -> f64 {
    let past = match unit {
        Unit::College => a
            .college_staff
            .iter()
            .filter(|x| x.0 < w.year && b.college_staff.iter().any(|y| y.0 == x.0 && y.1 == x.1))
            .count() as f64,
        Unit::Nfl => match (a.pro.as_ref(), b.pro.as_ref()) {
            (Some(ca), Some(cb)) => ca
                .seasons
                .iter()
                .filter(|s| {
                    s.year < w.year
                        && s.team.is_some()
                        && cb.seasons.iter().any(|t| t.year == s.year && t.team == s.team)
                })
                .count() as f64,
            _ => 0.0,
        },
        // High school teammates: the seasons both were in the program
        // (varsity from about the tenth grade).
        Unit::HighSchool => {
            let season = w.dates.season;
            let prior = |p: &Person| (3 - (p.class_of - season - 1)).clamp(0, 3) - 1;
            prior(a).min(prior(b)).max(0) as f64
        }
    };
    past + 0.5
}

fn chem_of(years: f64) -> f32 {
    (1.0 - (-0.55 * years).exp()) as f32
}

/// Sets each passer's, receiver's and defensive back's chemistry on a game
/// team from its people.
pub(crate) fn apply(w: &World, team: &mut GameTeam, unit: Unit) {
    let person =
        |g: &GamePlayer| super::scouting::place(w, g.id).map(|i| &w.people[i]).filter(|p| p.stage != Stage::Done);
    let by_exp = |pos: &[Position]| {
        let mut v: Vec<usize> = (0..team.players.len()).filter(|&k| pos.contains(&team.players[k].position)).collect();
        v.sort_by(|&a, &b| team.players[b].experience.total_cmp(&team.players[a].experience).then(a.cmp(&b)));
        v
    };
    let qbs = by_exp(&[Position::QB]);
    let targets = by_exp(&[Position::WR, Position::TE, Position::RB]);
    let dbs = by_exp(&[Position::CB, Position::S]);
    let mut set: Vec<(usize, f32)> = Vec::new();
    if let Some(&qb) = qbs.first()
        && let Some(q) = person(&team.players[qb])
    {
        let mut top = Vec::new();
        for &k in &targets {
            let c = person(&team.players[k]).map_or(0.3, |r| chem_of(seasons_together(w, q, r, unit)));
            set.push((k, c));
            if top.len() < 3 {
                top.push(c);
            }
        }
        if !top.is_empty() {
            set.push((qb, top.iter().sum::<f32>() / top.len() as f32));
        }
    }
    let starters: Vec<usize> = dbs.iter().take(4).copied().collect();
    for &k in &dbs {
        let Some(me) = person(&team.players[k]) else { continue };
        let others: Vec<f64> = starters
            .iter()
            .filter(|&&j| j != k)
            .filter_map(|&j| person(&team.players[j]).map(|o| seasons_together(w, me, o, unit)))
            .collect();
        if !others.is_empty() {
            set.push((k, chem_of(others.iter().sum::<f64>() / others.len() as f64)));
        }
    }
    // Your own routes on air beyond an average staff's (update 2, lane 4,
    // TR20): the quarterback and his targets, a little warmer or colder.
    let air =
        qbs.first().and_then(|&k| person(&team.players[k])).filter(|q| yours(w, q)).map_or(0.0, |_| w.training.air);
    for (k, c) in set {
        let pos = team.players[k].position;
        let air = if matches!(pos, Position::QB | Position::WR | Position::TE | Position::RB) { air } else { 0.0 };
        team.players[k].chem = (c + air).clamp(0.0, 1.0);
    }
}

/// Whether a player is on your team.
fn yours(w: &World, p: &Person) -> bool {
    let Some(s) = w.seat else { return false };
    match s.level {
        crate::calendar::Level::Nfl => p.pro.as_ref().and_then(|c| c.team).is_some_and(|t| s.nfl_team == Some(t)),
        crate::calendar::Level::HighSchool => p.stage == Stage::HighSchool && s.program == Some(p.hs),
        _ => p.stage == Stage::College && p.college.is_some() && p.college == s.program,
    }
}
