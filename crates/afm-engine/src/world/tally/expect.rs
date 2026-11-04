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
/// level, after lane NB5's passing changes; the NFL's 29,598 designed
/// runs went 4.20 yards a carry, the fit 4.19).
pub const XRUSH: [[f64; XRUSH_TERMS]; 5] = [
    // High school.
    [5.1210, -0.2549, 0.4523, 0.2520, 2.9230, -3.0876, 0.0158, -0.4259],
    // FBS.
    [4.7405, -0.4165, -0.2740, -0.4703, 3.0235, -2.1937, -0.2279, -0.3717],
    // FCS.
    [4.7684, 0.0742, 0.1019, -0.1589, 3.2597, -2.4264, -0.3352, -0.3472],
    // Small colleges.
    [5.0487, -0.0318, 0.0377, -0.2621, 3.1878, -2.6544, -0.6166, -0.1816],
    // NFL.
    [4.0193, 0.1937, 0.0297, -0.3929, 2.4234, -2.0101, -0.3765, -0.1445],
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
/// the NFL's 27,430 catches went 4.73 yards after the catch, the fit 4.73).
pub const XYAC: [[f64; XYAC_TERMS]; 5] = [
    // High school.
    [1.0579, 2.9092, 0.3127, 1.1414, 1.2298, 1.7745, -0.1533],
    // FBS.
    [0.6324, 3.4606, 0.9732, 1.8245, 1.2078, 1.6466, 0.2153],
    // FCS.
    [0.9794, 3.3191, 0.9284, 1.4855, 1.2232, 1.9054, 0.1821],
    // Small colleges.
    [0.8530, 3.6522, 0.9188, 1.2637, 1.2636, 2.0220, 0.1544],
    // NFL.
    [-0.5454, 3.7522, 0.9322, 1.9252, 1.1374, 1.0084, 0.5746],
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
/// colleges, NFL), from `xpass_fit -- 600 7` (2026-11-04: every
/// down-and-distance band within 2.5 points of the engine's own rate, the
/// lead and deficit bands within 3.5).
pub const XPASS: [[f64; XPASS_TERMS]; 5] = [
    // High school.
    [-1.1706, 0.3384, 1.0278, 0.8856, 0.3139, -0.2694, -0.1362, -0.1049, 0.0083, 0.0076, 1.4311, -1.0447, -0.7041],
    // FBS.
    [-0.7080, 0.2187, 1.0177, 0.8778, 0.2240, -0.1689, -0.1812, -0.1159, 0.2825, -0.3085, 1.5727, -1.1415, -0.9416],
    // FCS.
    [-0.8451, 0.2309, 0.9695, 0.8758, 0.2705, -0.1289, -0.1806, -0.0796, 0.2799, -0.2731, 1.5928, -1.2583, -0.8914],
    // Small colleges.
    [-0.7758, 0.2207, 1.0175, 0.8850, 0.2413, -0.1762, -0.1856, -0.1060, 0.2747, -0.2894, 1.6115, -1.1565, -0.9433],
    // NFL.
    [-0.7825, 0.4133, 1.5341, 1.1530, 0.3634, -0.1882, -0.1536, 0.0140, 0.3327, -0.6988, 1.3153, -0.3214, -0.7655],
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
