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
/// colleges, NFL), from `xyards_fit -- 600` (600 games a level; fitted
/// 2026-11-04 after lane NB5's passing changes, refitted 2026-11-07 after
/// minor bugs batch 9, and 2026-11-21 with scheme-fit lane SF10 once the
/// physics lanes were in: the NFL's 29,811 designed runs went 4.22 yards a
/// carry, the fit 4.21).
pub const XRUSH: [[f64; XRUSH_TERMS]; 5] = [
    // High school.
    [5.4680, -0.4681, 0.0185, -0.1900, 2.3124, -2.7353, -0.0741, -0.8579],
    // FBS.
    [4.5691, 0.2806, -0.0358, -0.6627, 2.1990, -2.0886, -0.2318, -0.3417],
    // FCS.
    [5.1857, -0.0595, -0.3963, -0.8470, 2.3862, -2.3213, -0.1542, -0.3476],
    // Small colleges.
    [5.4804, -0.2145, -0.4076, -0.8671, 2.2975, -2.4090, -0.2329, -0.6147],
    // NFL.
    [4.0698, 0.2988, -0.0112, -0.2736, 1.8780, -1.9414, -0.1263, -0.5511],
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

/// The YAC model's weights by level, from `xyards_fit -- 600` (2026-11-04,
/// refitted 2026-11-07 after minor bugs batch 9 and 2026-11-21 with lane
/// SF10; the NFL's 26,941 catches went 4.95 yards after the catch, the fit
/// 4.95).
pub const XYAC: [[f64; XYAC_TERMS]; 5] = [
    // High school.
    [0.6108, 2.2674, 0.8062, 1.7165, 1.4910, 1.7259, -0.4739],
    // FBS.
    [0.2379, 2.9471, 1.3150, 2.6304, 1.4335, 1.2154, -0.3097],
    // FCS.
    [0.4009, 3.3637, 1.1924, 2.6613, 1.4179, 1.5919, -0.2697],
    // Small colleges.
    [0.4824, 3.2729, 1.2069, 1.2433, 1.4095, 1.6235, -0.1330],
    // NFL.
    [-0.9184, 2.9852, 1.0243, 3.0724, 1.3445, 0.9635, 0.1987],
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

/// How many terms the expected pass model reads.
pub const XPASS_TERMS: usize = 13;

/// What the expected pass model reads of a scrimmage snap before it is
/// called, as nflfastR's expected pass rate reads the down, the distance,
/// the field, the score, the clock and the win chance: a constant (first
/// down), second, third and fourth down, the log of the yards to go, two
/// yards or less to go, the yards to the goal line (in hundreds), inside
/// the ten, the score margin in touchdowns (to three either way), the
/// margin times the share of the game gone, the last two minutes of a
/// half, the offense's win chance less a half, and three yards or less on
/// third or fourth down. Proposed, not confirmed: the features.
#[allow(clippy::too_many_arguments)]
pub fn xpass_features(
    level: Level,
    down: u8,
    togo: f64,
    to_goal: f64,
    margin: i32,
    quarter: u8,
    clock: f64,
    wp: f64,
) -> [f64; XPASS_TERMS] {
    let on = |b: bool| b as u8 as f64;
    let qlen = if level == Level::HS { 720.0 } else { 900.0 };
    let gone =
        if quarter >= 5 { 1.0 } else { ((quarter.max(1) - 1) as f64 + (1.0 - clock / qlen).clamp(0.0, 1.0)) / 4.0 };
    let m = (margin as f64 / 7.0).clamp(-3.0, 3.0);
    [
        1.0,
        on(down == 2),
        on(down == 3),
        on(down == 4),
        togo.max(1.0).ln(),
        on(togo <= 2.0),
        to_goal / 100.0,
        on(to_goal <= 10.0),
        m,
        m * gone,
        on(matches!(quarter, 2 | 4) && clock <= 120.0),
        wp - 0.5,
        on(down >= 3 && togo <= 3.0),
    ]
}

/// The expected pass model's weights by level (HS, FBS, FCS, small
/// colleges, NFL), from `xpass_fit -- 600 7` (2026-11-04, refitted
/// 2026-11-07 after minor bugs batch 9, and 2026-11-21 with scheme-fit lane
/// SF10, whose roster lean moves play calling: every down-and-distance band
/// within 2.4 points of the engine's own rate, the lead and deficit bands
/// within 2.8).
pub const XPASS: [[f64; XPASS_TERMS]; 5] = [
    // High school.
    [-1.1722, 0.2959, 0.9830, 0.9235, 0.3575, -0.1666, -0.2129, -0.0401, -0.0246, -0.0172, 1.0074, -0.5559, -0.7115],
    // FBS.
    [-0.6602, 0.2158, 1.0083, 0.8342, 0.2714, -0.2002, -0.3310, -0.1401, 0.2370, -0.2633, 1.2439, -1.0050, -0.8962],
    // FCS.
    [-0.5753, 0.1905, 0.9615, 0.6692, 0.2257, -0.2489, -0.3462, -0.0690, 0.2426, -0.2605, 1.2292, -1.0144, -0.8889],
    // Small colleges.
    [-0.6938, 0.2196, 0.9672, 0.7974, 0.2639, -0.1270, -0.3408, -0.1355, 0.2506, -0.2788, 1.2518, -1.0671, -0.9794],
    // NFL.
    [-0.7055, 0.3755, 1.5569, 1.2012, 0.3565, -0.1626, -0.2167, -0.0365, 0.2082, -0.5369, 0.9942, -0.2014, -0.7701],
];

/// The chance a snap is a dropback (a throw, a sack or a scramble) by the
/// level's own play calling in that situation.
pub fn pass_chance(level: Level, x: &[f64; XPASS_TERMS]) -> f64 {
    let z: f64 = XPASS[row(level)].iter().zip(x).map(|(b, x)| b * x).sum();
    1.0 / (1.0 + (-z).exp())
}

/// A scrimmage snap's expected pass rate, from the situation it was called
/// in (the offense's view).
pub fn expected_pass(level: Level, p: &crate::game::stats::PlayRecord) -> f64 {
    pass_chance(level, &pass_features_of(level, p))
}

/// A snap's expected pass features.
pub fn pass_features_of(level: Level, p: &crate::game::stats::PlayRecord) -> [f64; XPASS_TERMS] {
    let o = p.offense.min(1);
    xpass_features(level, p.down, p.togo, p.yardline_100, p.score[o] - p.score[1 - o], p.quarter, p.clock, p.wp_before)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The expected pass rate reads the situation as play callers do:
    /// third and long throws, third and short runs more, and a team far
    /// behind late throws more than one far ahead.
    #[test]
    fn the_expected_pass_rate_reads_the_situation() {
        for level in [Level::HS, Level::FBS, Level::NFL] {
            let x = |down, togo, margin, quarter, clock, wp| {
                pass_chance(level, &xpass_features(level, down, togo, 60.0, margin, quarter, clock, wp))
            };
            let first = x(1, 10.0, 0, 1, 600.0, 0.5);
            let long = x(3, 9.0, 0, 1, 600.0, 0.5);
            let short = x(3, 1.0, 0, 1, 600.0, 0.5);
            assert!(long > first + 0.15 && short < long - 0.2, "{level:?}: {first} {long} {short}");
            assert!((0.3..0.7).contains(&first), "{level:?}: first and ten {first}");
            let (behind, ahead) = (x(1, 10.0, -17, 4, 300.0, 0.05), x(1, 10.0, 17, 4, 300.0, 0.95));
            assert!(behind > ahead + 0.15, "{level:?}: behind {behind} ahead {ahead}");
        }
    }
}
