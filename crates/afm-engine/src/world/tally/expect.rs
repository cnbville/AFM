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
/// minor bugs batch 9, 2026-11-21 with scheme-fit lane SF10 once the
/// physics lanes were in, and the same day with lane SF9b, the open-field
/// tackle: the NFL's 29,671 designed runs went 4.47 yards a carry, the fit
/// 4.46, the old weights 4.22).
pub const XRUSH: [[f64; XRUSH_TERMS]; 5] = [
    // High school.
    [5.5514, -0.4296, 0.2682, -0.2518, 2.8991, -3.0419, -0.4726, -0.7852],
    // FBS.
    [5.2511, -0.2875, -0.4990, -0.9705, 2.4753, -2.4742, -0.4734, -0.4854],
    // FCS.
    [5.1248, 0.1217, 0.0119, -0.6910, 2.6205, -2.5698, -0.5990, -0.3764],
    // Small colleges.
    [5.4518, 0.1309, -0.1709, -0.7446, 2.6715, -2.7370, -0.5917, -0.5229],
    // NFL.
    [4.3622, 0.1973, 0.0030, -0.4616, 2.2016, -2.3521, -0.4243, -0.5199],
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
/// SF10 and SF9b; the NFL's 27,348 catches went 4.86 yards after the catch,
/// the fit 4.87, the old weights 4.95).
pub const XYAC: [[f64; XYAC_TERMS]; 5] = [
    // High school.
    [0.2117, 2.4057, 0.8245, 2.3334, 1.5556, 2.0741, -0.3780],
    // FBS.
    [0.3183, 3.4134, 1.4078, 2.1110, 1.3652, 1.5618, -0.3471],
    // FCS.
    [0.2951, 3.2282, 1.6522, 2.1931, 1.4507, 1.5262, -0.2628],
    // Small colleges.
    [0.5321, 3.2949, 1.3328, 2.2152, 1.4208, 1.8094, -0.1817],
    // NFL.
    [-0.9609, 3.2958, 1.1593, 2.6323, 1.3233, 0.8762, 0.2048],
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
/// SF10, whose roster lean moves play calling, and again the same day with
/// lane SF9b's rebuilt tables: every down-and-distance band within 2.3
/// points of the engine's own rate, the lead and deficit bands within 3.3).
pub const XPASS: [[f64; XPASS_TERMS]; 5] = [
    // High school.
    [-1.1213, 0.2877, 0.9141, 0.8362, 0.3316, -0.2554, -0.1824, -0.0639, -0.0656, 0.0061, 1.0144, -0.4453, -0.6689],
    // FBS.
    [-0.6599, 0.2068, 0.9260, 0.7385, 0.2574, -0.1531, -0.2779, -0.1581, 0.2310, -0.2849, 1.2840, -0.9332, -0.8745],
    // FCS.
    [-0.5437, 0.2007, 0.9564, 0.7144, 0.2255, -0.2378, -0.3838, -0.1688, 0.2252, -0.2391, 1.2443, -1.0108, -0.9218],
    // Small colleges.
    [-0.7038, 0.2075, 0.9499, 0.7796, 0.2593, -0.1947, -0.2725, -0.1317, 0.2182, -0.2851, 1.2818, -0.7985, -0.8786],
    // NFL.
    [-0.7339, 0.4097, 1.4739, 1.1459, 0.3698, -0.1796, -0.2417, 0.0175, 0.2220, -0.5385, 1.0403, -0.1989, -0.7443],
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
