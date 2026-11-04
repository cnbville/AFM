//! Rush yards over expected and yards after the catch over expected (Data
//! overhaul items 5 and 6). Next Gen Stats fit both on the NFL's own plays;
//! these are fitted on the match engine's own plays at each level, as the
//! expected-completion model behind CPOE is (`game::stats::XCOMP`), so a
//! level's average back and receiver sit near zero. Refit after engine
//! changes that move runs or the catch and run:
//! `cargo run --release --bin xyards_fit -- 600`.
//!
//! The engine keeps no defenders' positions at the handoff or the catch, so
//! each model reads what the play record keeps: for a designed run, the
//! men in the box, inside or outside, how much field is left and the
//! distance to go; for a catch, how deep it was thrown, the receiver's
//! separation, man or zone coverage and whether he caught it in the middle
//! of the field. Least squares on the engine's plays at each level.
//! Proposed, not confirmed: the features.

use crate::player::Level;

/// The levels in `XRUSH` and `XYAC` order (as `XCOMP`'s).
fn row(level: Level) -> usize {
    level as usize
}

/// How many terms the expected rush model reads.
pub const XRUSH_TERMS: usize = 8;

/// What the expected rush model reads of a designed run: a constant (an
/// inside run against five or fewer in the box, with room, not short
/// yardage), six, seven and eight or more in the box, an outside run, ten
/// yards or less to the goal line, eleven to twenty, and two yards or less
/// to go.
pub fn xrush_features(box_count: u8, outside: bool, to_goal: f64, togo: f64) -> [f64; XRUSH_TERMS] {
    let on = |b: bool| b as u8 as f64;
    [
        1.0,
        on(box_count == 6),
        on(box_count == 7),
        on(box_count >= 8),
        on(outside),
        on(to_goal <= 10.0),
        on(to_goal > 10.0 && to_goal <= 20.0),
        on(togo <= 2.0),
    ]
}

/// The expected rush model's weights by level (HS, FBS, FCS, small
/// colleges, NFL), from `xyards_fit -- 600` (2026-11-04, 600 games a
/// level; the NFL's 29,243 designed runs went 4.18 yards a carry, the fit
/// 4.16).
pub const XRUSH: [[f64; XRUSH_TERMS]; 5] = [
    // High school.
    [5.1792, -0.3610, 0.3188, 0.0879, 2.9511, -2.8618, -0.0465, -0.5879],
    // FBS.
    [4.5983, -0.2065, -0.1394, -0.4321, 3.0022, -2.3221, -0.3244, -0.2406],
    // FCS.
    [4.7506, 0.0014, 0.1100, -0.2659, 3.3600, -2.5496, -0.3292, -0.2902],
    // Small colleges.
    [5.2385, -0.3220, -0.1706, -0.4033, 3.2378, -2.7502, -0.3924, -0.2089],
    // NFL.
    [4.0534, 0.0912, -0.1352, -0.3057, 2.3922, -1.8980, -0.1430, -0.3047],
];

/// Expected yards on a designed run, no more than the field left.
pub fn expected_rush(level: Level, box_count: u8, outside: bool, to_goal: f64, togo: f64) -> f64 {
    let x = xrush_features(box_count, outside, to_goal, togo);
    let y: f64 = XRUSH[row(level)].iter().zip(x).map(|(b, x)| b * x).sum();
    y.clamp(-3.0, to_goal.max(0.0))
}

/// How many terms the expected yards-after-catch model reads.
pub const XYAC_TERMS: usize = 7;

/// What the YAC model reads of a catch: a constant (a short catch, 0-9
/// yards downfield, in zone coverage, outside the middle third, with no
/// separation), caught behind the line, 10-19 yards downfield, 20 or more,
/// the receiver's separation in yards (up to five), man coverage, and
/// caught in the middle third of the field.
pub fn xyac_features(air: f64, sep: f64, man: bool, middle: bool) -> [f64; XYAC_TERMS] {
    let on = |b: bool| b as u8 as f64;
    [1.0, on(air < 0.0), on((10.0..20.0).contains(&air)), on(air >= 20.0), sep.clamp(0.0, 5.0), on(man), on(middle)]
}

/// The YAC model's weights by level, from `xyards_fit -- 600` (2026-11-04;
/// the NFL's 27,246 catches went 4.73 yards after the catch, the fit 4.73).
pub const XYAC: [[f64; XYAC_TERMS]; 5] = [
    // High school.
    [0.8114, 2.7201, 0.5541, 1.6486, 1.3033, 1.8200, 0.0296],
    // FBS.
    [0.4988, 3.6047, 0.9671, 1.6559, 1.2282, 1.7913, 0.2472],
    // FCS.
    [0.8464, 3.3867, 0.7758, 1.2176, 1.2306, 2.1844, 0.2227],
    // Small colleges.
    [0.6853, 3.7298, 0.6847, 1.5440, 1.3419, 1.8932, 0.1373],
    // NFL.
    [-0.3130, 3.3187, 0.6037, 1.7310, 1.1204, 0.9546, 0.5043],
];

/// Expected yards after the catch, never below zero.
pub fn expected_yac(level: Level, air: f64, sep: f64, man: bool, middle: bool) -> f64 {
    let x = xyac_features(air, sep, man, middle);
    let y: f64 = XYAC[row(level)].iter().zip(x).map(|(b, x)| b * x).sum();
    y.max(0.0)
}

/// The run's gap from its call, from the offense's view: 0 left end, 1 left
/// inside, 2 middle, 3 right inside, 4 right end (as the game page's runs
/// by gap read it). An option's handoff reads its run part.
pub fn run_gap(call: &str) -> u8 {
    let c = call.to_lowercase();
    let c = c.split(" with ").next().unwrap_or("");
    let left = c.ends_with("left");
    let right = c.ends_with("right");
    let outside = ["outside zone", "toss", "sweep", "option"].iter().any(|x| c.contains(x));
    let middle = ["draw", "sneak"].iter().any(|x| c.contains(x)) || !(left || right);
    if middle {
        2
    } else if outside {
        if left { 0 } else { 4 }
    } else if left {
        1
    } else {
        3
    }
}

/// The third of the field a ball arrived in, from the offense's view: 0
/// left, 1 middle, 2 right (the offense's left is high y, 0 to 53.3).
pub fn third(y: f64) -> usize {
    let w = 53.33 / 3.0;
    if y > 2.0 * w {
        0
    } else if y >= w {
        1
    } else {
        2
    }
}

/// A throw's depth row: 0 behind the line, 1 short (0-9 yards), 2
/// intermediate (10-19), 3 deep (20 or more).
pub fn depth_row(air: f64) -> usize {
    if air < 0.0 {
        0
    } else if air < 10.0 {
        1
    } else if air < 20.0 {
        2
    } else {
        3
    }
}
