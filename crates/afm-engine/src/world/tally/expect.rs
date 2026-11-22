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

// Fitted tables: a refit can land a value near a named constant.
#![allow(clippy::approx_constant)]

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
/// minor bugs batch 9, 2026-11-21 with scheme-fit lane SF10 once the
/// physics lanes were in, the same day with lane SF9b, the open-field
/// tackle, and with engine gaps lane EG6, the red zone: the NFL's 29,732
/// designed runs went 4.36 yards a carry, the fit 4.36, the old weights
/// 4.43).
pub const XRUSH: [[f64; XRUSH_TERMS]; 5] = [
    // High school.
    [5.1961, -0.0146, 0.6573, 0.3728, 2.6168, -3.3383, -0.3937, -0.9728],
    // FBS.
    [4.8770, -0.0029, -0.2808, -0.7392, 2.5193, -2.5370, -0.5276, -0.5124],
    // FCS.
    [5.6661, -0.4340, -0.5799, -1.0955, 2.4351, -2.8414, -0.6816, -0.5824],
    // Small colleges.
    [5.6258, -0.1892, -0.4442, -1.0380, 2.6363, -2.8369, -0.6697, -0.4518],
    // NFL.
    [4.2838, 0.2688, 0.0781, -0.1894, 1.9418, -2.5851, -0.4595, -0.7997],
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
/// refitted 2026-11-07 after minor bugs batch 9 and 2026-11-21 with lanes
/// SF10, SF9b and EG6; the NFL's 27,099 catches went 4.76 yards after the
/// catch, the fit 4.76, the old weights 4.86).
pub const XYAC: [[f64; XYAC_TERMS]; 5] = [
    // High school.
    [0.7434, 2.2891, 0.6845, 1.9234, 1.5181, 0.7755, -0.5762],
    // FBS.
    [0.0605, 2.9867, 1.6038, 2.0873, 1.5173, 0.5219, -0.2913],
    // FCS.
    [0.3696, 3.0679, 1.4095, 2.1075, 1.5140, 0.8316, -0.2675],
    // Small colleges.
    [0.4742, 2.9469, 1.1835, 2.4375, 1.4854, 1.0687, -0.1739],
    // NFL.
    [-0.8879, 3.0725, 1.1991, 2.2445, 1.3732, 0.1440, 0.0948],
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
/// SF10, whose roster lean moves play calling, again the same day with
/// lane SF9b's rebuilt tables, and with engine gaps lane EG6, the red zone,
/// whose short field moves the call: every down-and-distance band within
/// 3.1 points of the engine's own rate, the lead and deficit bands within
/// 3.3).
pub const XPASS: [[f64; XPASS_TERMS]; 5] = [
    // High school.
    [-1.1000, 0.3132, 0.9314, 0.8840, 0.3328, -0.1443, -0.2479, -0.1896, -0.0412, -0.0172, 0.9762, -0.5326, -0.7120],
    // FBS.
    [-0.7064, 0.2344, 0.9490, 0.8078, 0.2725, -0.1253, -0.2903, -0.1409, 0.2729, -0.3185, 1.1936, -0.9985, -0.8898],
    // FCS.
    [-0.7644, 0.2330, 0.9938, 0.7799, 0.2678, -0.1095, -0.1973, -0.1900, 0.2526, -0.2909, 1.2158, -0.9669, -0.9735],
    // Small colleges.
    [-0.7091, 0.2504, 0.9696, 0.7518, 0.2622, -0.1448, -0.3006, -0.2307, 0.2422, -0.2743, 1.2593, -1.0013, -0.9300],
    // NFL.
    [-0.8198, 0.4721, 1.5595, 1.2206, 0.3734, -0.1983, -0.1595, -0.0410, 0.2668, -0.5716, 1.0286, -0.3998, -0.7236],
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
