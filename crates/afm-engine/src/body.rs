//! Measurables and combine testing (trial 1 note 8).
//!
//! Each player has an adult body: height, weight, arm length, hand size and
//! wingspan, drawn per position from the real NFL combine (nflverse combine
//! data, 2000-2025, `data/real-world/combine_by_position.csv`). Less talented
//! players are smaller, and the body leans with his own build (a strong player
//! is heavier, a fast one lighter). Kids grow into it through high school.
//!
//! Combine results come from attributes plus body, fitted to the real
//! position averages, with real test-day noise. Arm, hand and wingspan follow
//! height, from the 2025 combine offensive line and quarterback lists
//! (approximate: no open dataset with every player's arm length was reachable).

use std::sync::LazyLock;

use crate::attributes::Attr;
use crate::player::Player;
use crate::positions::{POSITION_COUNT, Position};
use crate::rng::{Rng, clamp};

const COMBINE_CSV: &str = include_str!("../../../data/real-world/combine_by_position.csv");

/// Height and weight per position at the combine: (height mean, sd, weight mean, sd).
static SIZES: LazyLock<[(f64, f64, f64, f64); POSITION_COUNT]> = LazyLock::new(|| {
    let mut out = [(73.0, 2.0, 230.0, 20.0); POSITION_COUNT];
    for line in COMBINE_CSV.lines().skip(1) {
        let f: Vec<&str> = line.split(',').collect();
        let Some(pos) = Position::ALL.iter().find(|p| p.name() == f[0]) else { continue };
        let num = |k: usize| f.get(k).and_then(|s| s.parse::<f64>().ok());
        if let (Some(h), Some(hs), Some(wt), Some(ws)) = (num(2), num(3), num(4), num(5)) {
            out[pos.index()] = (h, hs, wt, ws);
        }
    }
    out
});

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Body {
    /// Inches.
    pub height: f64,
    /// Pounds.
    pub weight: f64,
    /// Arm length, inches.
    pub arm: f64,
    /// Hand size, inches.
    pub hand: f64,
    /// Wingspan, inches.
    pub wingspan: f64,
    /// The weight his frame carries naturally, before a position's weight
    /// program adds or takes off pounds.
    pub frame_weight: f64,
}

/// A position's combine height and weight: (height mean, sd, weight mean, sd).
/// The real combine bench average at a position (nflverse 2000-2025).
pub fn combine_bench_mean(pos: Position) -> f64 {
    [19.3, 19.95, 14.68, 20.24, 24.35, 25.81, 23.66, 27.41, 21.97, 14.51, 16.93, 15.2, 16.0][pos.index()]
}

pub fn combine_size(pos: Position) -> (f64, f64, f64, f64) {
    SIZES[pos.index()]
}

impl Body {
    /// Arms, hands and wingspan follow height (see the module notes).
    pub(crate) fn grown(height: f64, weight: f64, rng: &mut Rng) -> Body {
        let heavy = weight >= 285.0;
        let arm = 0.425 * height + if heavy { 0.3 } else { 0.0 } + rng.normal(0.0, 0.8);
        let hand = 9.45 + 0.08 * (height - 74.0) + if heavy { 0.35 } else { 0.0 } + rng.normal(0.0, 0.35);
        let wingspan = 2.03 * arm + 12.3 + rng.normal(0.0, 1.0);
        Body { height, weight, arm, hand, wingspan, frame_weight: weight }
    }
}

fn lineman(pos: Position) -> bool {
    matches!(pos, Position::OT | Position::IOL | Position::DT | Position::EDGE)
}

/// The adult body a player grows into. `potential` is his original potential;
/// `strength_z` and `speed_z` are how far his own strength and speed ceilings
/// sit from what's typical for his position (in standard deviations). Drawn
/// on its own random stream so it doesn't change anything else about him.
pub fn adult_body(id: u32, pos: Position, potential: f64, strength_z: f64, speed_z: f64) -> Body {
    let mut rng = Rng::new(id ^ 0x5EED_B0D1);
    let (hm, hs, wm, ws) = SIZES[pos.index()];
    // The combine sees draft prospects. Kids who top out lower are smaller.
    let size = clamp((potential - 72.0) / 12.0, -2.5, 0.5);
    let h_z = 0.7 * size + rng.normal(0.0, 1.0);
    let w_z = 0.3 + 1.3 * size + 0.45 * strength_z - 0.3 * speed_z + 0.25 * h_z + rng.normal(0.0, 0.8);
    let height = clamp(hm + hs * h_z, 64.0, 82.0);
    let weight = clamp(wm + ws * w_z, 150.0, 380.0);
    let arm_offset = if lineman(pos) {
        0.3
    } else if pos == Position::QB {
        -0.5
    } else {
        0.0
    };
    let arm = 0.425 * height + arm_offset + rng.normal(0.0, 0.8);
    let hand = 9.45 + 0.08 * (height - 74.0) + if lineman(pos) { 0.35 } else { 0.0 } + rng.normal(0.0, 0.35);
    let wingspan = 2.03 * arm + 12.3 + rng.normal(0.0, 1.0);
    Body { height, weight, arm, hand, wingspan, frame_weight: weight }
}

/// His body now: boys grow through high school, late bloomers later, and
/// fill out into their twenties as strength comes.
pub fn body_now(p: &Player) -> Body {
    let a = p.body;
    let age = p.age as f64;
    let grown = clamp((age - 13.0 - 0.6 * p.bloom) / 6.0, 0.0, 1.0);
    let filled = clamp((age - 13.0 - 0.5 * p.bloom) / 9.0, 0.0, 1.0);
    let strength = p.attributes[Attr::Strength] / p.ceilings[Attr::Strength].max(1.0);
    let height = a.height - (1.0 - grown) * 4.0;
    let weight = a.weight * (0.7 + 0.25 * filled + 0.05 * clamp(strength, 0.0, 1.0));
    let k = height / a.height;
    Body {
        height,
        weight,
        arm: a.arm * k,
        hand: a.hand * (0.9 + 0.1 * grown),
        wingspan: a.wingspan * k,
        frame_weight: a.frame_weight,
    }
}

/// One day of testing. None for a drill the position doesn't run
/// (kickers skip the agility drills).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Combine {
    /// 40-yard dash, seconds.
    pub forty: f64,
    /// 225 lb bench press reps.
    pub bench: f64,
    /// Vertical jump, inches.
    pub vertical: f64,
    /// Broad jump, inches.
    pub broad: f64,
    /// 3-cone drill, seconds.
    pub cone: f64,
    /// 20-yard shuttle, seconds.
    pub shuttle: f64,
}

/// Combine results from his attributes and body, with test-day noise.
/// Coefficients fitted to the real position averages (nflverse, 2000-2025).
pub fn combine(p: &Player, rng: &mut Rng) -> Combine {
    let b = body_now(p);
    let at = |a: Attr| p.attributes[a];
    let (speed, accel, agility, strength, jumping) =
        (at(Attr::Speed), at(Attr::Acceleration), at(Attr::Agility), at(Attr::Strength), at(Attr::Jumping));
    let w = b.weight - 220.0;
    let burst = jumping.max(0.95 * accel);
    let quick = 0.7 * agility + 0.3 * accel;
    let forty = 5.56 - 0.0124 * (0.75 * speed + 0.25 * accel) + 0.0033 * w + rng.normal(0.0, 0.04);
    let vertical = 20.9 + 0.182 * burst - 0.035 * w + rng.normal(0.0, 1.5);
    let broad = 87.6 + 0.448 * burst - 0.089 * w + rng.normal(0.0, 3.0);
    // Long arms mean a longer press: fewer reps for the same strength.
    // Refit 2026-10-14 to drafted players' position means after athlete-first
    // creation (item 6; proposed, not confirmed).
    let bench = 8.05 + 0.137 * strength + 0.073 * w - 0.8 * (b.arm - 32.5) + rng.normal(0.0, 2.0);
    let cone = 7.58 - 0.007 * quick + 0.0062 * w + rng.normal(0.0, 0.08);
    let shuttle = 4.59 - 0.0045 * quick + 0.0039 * w + rng.normal(0.0, 0.06);
    let r2 = |x: f64| (x * 100.0).round() / 100.0;
    Combine {
        forty: r2(clamp(forty, 4.2, 6.0)),
        bench: clamp(bench, 0.0, 50.0).round(),
        vertical: (clamp(vertical, 18.0, 46.0) * 2.0).round() / 2.0,
        broad: clamp(broad, 80.0, 145.0).round(),
        cone: r2(clamp(cone, 6.4, 9.0)),
        shuttle: r2(clamp(shuttle, 3.8, 5.5)),
    }
}

/// Feet and inches, the way the combine writes it: "6'3 1/2\"", to the eighth.
pub fn height_text(inches: f64) -> String {
    let eighths = (inches * 8.0).round() as i64;
    let (ft, rest) = (eighths / 96, eighths % 96);
    let (whole, frac) = (rest / 8, rest % 8);
    let frac = match frac {
        0 => "",
        1 => " 1/8",
        2 => " 1/4",
        3 => " 3/8",
        4 => " 1/2",
        5 => " 5/8",
        6 => " 3/4",
        _ => " 7/8",
    };
    format!("{ft}'{whole}{frac}\"")
}
