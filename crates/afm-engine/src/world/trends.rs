//! How ideas spread over the years (plays and tactics topic 9): each
//! division's mix of concept families by season (the trend lines the user
//! watches, 9.9), and how new each family still is to the defenses there
//! (9.5, 9.6). A family that grew fast is read a beat late until defenses
//! catch up; one that faded and comes back is new again. A team that runs
//! something few others do (a triple option in a spread league) is new to
//! every opponent too. Sizes are proposed, not confirmed.

use super::World;
use super::data::Division;
use crate::game::Film;
use crate::game::playbook::{FAMILY_COUNT, Family};
use crate::rng::clamp;

/// One division's season.
#[derive(Clone, Debug)]
pub struct Trend {
    pub year: i32,
    pub division: Division,
    /// Share of offensive snaps in each offense family, and of defensive
    /// snaps in each defense family (`Film::families`).
    pub families: [f64; FAMILY_COUNT],
    /// Passes over plays, from the engine games.
    pub pass_rate: f64,
}

/// How many seasons back a rise is measured over.
const RISE_YEARS: i32 = 4;

/// At fall camp, before last season's film fades: the season just played
/// goes on the trend lines and each division's novelty is set from it.
pub(crate) fn record(w: &mut World) {
    let last = w.year - 1;
    for d in Division::ALL {
        let mut film = Film::default();
        let (mut att, mut plays) = (0u32, 0u32);
        for p in w.programs.iter().filter(|p| p.division == d) {
            film.add(&p.film);
            if let Some(s) = p.seasons.last().filter(|s| s.year == last) {
                att += s.totals.att;
                plays += s.totals.att + s.totals.carries;
            }
        }
        if film.off_snaps < 100.0 {
            continue;
        }
        let families = film.families();
        let before = w.trends.iter().rev().find(|t| t.division == d && t.year <= last - RISE_YEARS).map(|t| t.families);
        w.novelty[d as usize] = match before {
            Some(b) => std::array::from_fn(|f| novelty(families[f], b[f])),
            None => [0.0; FAMILY_COUNT],
        };
        if w.recording {
            let pass_rate = att as f64 / plays.max(1) as f64;
            w.trends.push(Trend { year: last, division: d, families, pass_rate });
        }
    }
}

/// How new a family is from its share now and four seasons ago: a family
/// that doubled is about half new; one that only held its share is known.
fn novelty(now: f64, before: f64) -> f64 {
    clamp((now - before) / now.max(0.02), 0.0, 1.0) * 0.8
}

/// A team's novelty: its division's, plus what it runs far more than the
/// division does (opponents rarely see it). Defense families too: a rare
/// front is a puzzle for an offense.
pub(crate) fn team_novelty(w: &World, d: Division, own: &Film) -> [f64; FAMILY_COUNT] {
    let base = w.novelty[d as usize];
    if own.off_snaps + own.def_snaps < 40.0 {
        return base;
    }
    let mine = own.families();
    let level = w.trends.iter().rev().find(|t| t.division == d).map(|t| t.families);
    std::array::from_fn(|f| {
        let rare = match level {
            Some(l) if mine[f] > 0.04 && f != Family::Front as usize => {
                clamp((mine[f] - l[f]) / mine[f], 0.0, 1.0) * clamp(mine[f] / 0.25, 0.0, 1.0) * 0.5
            }
            _ => 0.0,
        };
        clamp(base[f] + rare, 0.0, 1.0)
    })
}
