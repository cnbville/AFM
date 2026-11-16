//! The playing time grade (Cain 2026-11-15, "play time ranked low on a
//! 2-10 team"): it reads next fall's depth chart, spot by spot, against the
//! level's typical starter (`staff::open_spots`), not the share of old
//! starters.

use crate::positions::{POSITION_COUNT, Position};
use crate::world::staff::open_spots;

fn room(f: impl Fn(Position) -> Vec<f64>) -> [Vec<f64>; POSITION_COUNT] {
    let mut r: [Vec<f64>; POSITION_COUNT] = Default::default();
    for pos in Position::ALL {
        r[pos.index()] = f(pos);
    }
    r
}

fn deep(level: f64) -> impl Fn(Position) -> Vec<f64> {
    move |pos| vec![level; crate::game::roster::starters(pos) + 2]
}

#[test]
fn a_weak_young_team_that_keeps_its_starters_is_more_open_than_a_strong_one() {
    let norm = [70.0; POSITION_COUNT];
    // A 2-10 team: every starter comes back, but each is 6 points under the
    // level's typical starter. A contender's returners are 6 over.
    let weak = open_spots(&room(deep(64.0)), &norm);
    let strong = open_spots(&room(deep(76.0)), &norm);
    assert!(weak > 0.75 && strong < 0.25, "weak {weak:.2}, strong {strong:.2}");
    // A typical room is half open.
    assert!((open_spots(&room(deep(70.0)), &norm) - 0.5).abs() < 1e-9);
}

#[test]
fn starters_leaving_open_spots_only_when_nobody_good_waits_behind_them() {
    let norm = [70.0; POSITION_COUNT];
    let full = open_spots(&room(deep(70.0)), &norm);
    // The quarterback leaves and nobody is behind him: that spot is open.
    let mut gone = room(deep(70.0));
    gone[Position::QB.index()].clear();
    assert!(open_spots(&gone, &norm) > full);
    // He leaves, but a backup better than the typical starter takes over.
    let mut heir = room(deep(70.0));
    heir[Position::QB.index()] = vec![75.0];
    assert!(open_spots(&heir, &norm) < full);
}

#[test]
fn kickers_stay_out_of_the_team_read() {
    let norm = [70.0; POSITION_COUNT];
    let base = open_spots(&room(deep(70.0)), &norm);
    let mut r = room(deep(70.0));
    for pos in [Position::K, Position::P, Position::LS] {
        r[pos.index()].clear();
    }
    assert!((open_spots(&r, &norm) - base).abs() < 1e-9);
}
