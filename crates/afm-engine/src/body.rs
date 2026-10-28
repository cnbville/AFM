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

#[derive(Clone, Copy, Debug, PartialEq, Default, serde::Serialize, serde::Deserialize)]
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
    /// Throws left-handed: his blind side is his right.
    pub left_handed: bool,
    /// Kicks with his left foot: the ball spins the other way.
    pub left_footed: bool,
}

/// About one in ten throws left-handed and about one in six kicks
/// left-footed, as in the population: most lefties kick left, about one
/// righty in eight does (proposed, not confirmed).
const LEFT_HANDED: f64 = 0.1;
const RIGHTY_LEFT_FOOTED: f64 = 0.12;

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
        let left_handed = rng.chance(LEFT_HANDED);
        let left_footed = rng.chance(if left_handed { 0.6 } else { RIGHTY_LEFT_FOOTED });
        Body { height, weight, arm, hand, wingspan, frame_weight: weight, left_handed, left_footed }
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
    let left_handed = rng.chance(LEFT_HANDED);
    let left_footed = rng.chance(if left_handed { 0.6 } else { RIGHTY_LEFT_FOOTED });
    Body { height, weight, arm, hand, wingspan, frame_weight: weight, left_handed, left_footed }
}

/// How a position group fills out (update 3, A12; Cain 2026-10-25: "give
/// lineman room to grow ... a running back or wide receiver should already
/// be around 90%"): his share of his grown weight at 14, 15 and on, until
/// full. Backs, receivers, defensive backs and kickers arrive near full;
/// quarterbacks, tight ends, linebackers, edge rushers and snappers a step
/// behind; tackles, guards, centers and defensive tackles make their
/// biggest gains in the first two college years (real: line recruits
/// average about 88% of the combine's linemen, FBS starting lines 309 lb).
/// The years before 17 are proposed, not confirmed.
const FILL_SKILL: [f64; 9] = [0.80, 0.84, 0.87, 0.90, 0.94, 0.97, 0.99, 1.0, 1.0];
const FILL_MID: [f64; 9] = [0.79, 0.83, 0.86, 0.89, 0.93, 0.965, 0.99, 1.0, 1.0];
const FILL_LINE: [f64; 9] = [0.76, 0.80, 0.84, 0.87, 0.90, 0.94, 0.97, 0.99, 1.0];

/// A position's fill-out curve and the age he is full.
fn fill_curve(pos: Position) -> (&'static [f64; 9], f64) {
    match pos {
        Position::OT | Position::IOL | Position::DT => (&FILL_LINE, 22.0),
        Position::QB | Position::TE | Position::LB | Position::EDGE | Position::LS => (&FILL_MID, 21.0),
        _ => (&FILL_SKILL, 21.0),
    }
}

/// His own share against his group's when he arrives in college, in
/// shares of his grown weight (A13): most sit within 3 points (5 for
/// linemen), a few further out, so a Metcalf-type receiver arrives at 98%
/// and a late-growing tackle at 78% and bulks about 60 lb in college. Late
/// bloomers lean light. Fixed for life, on his own stream (proposed, not
/// confirmed: sd 2 points, 3.2 for linemen, one in seven 2.5 times wider,
/// six tenths of it from his bloom).
fn fill_offset(p: &Player) -> f64 {
    let mut r = Rng::new(p.id ^ 0xF111_0FF5);
    let wide = r.chance(0.15);
    let own = r.normal(0.0, 1.0) * if wide { 2.5 } else { 1.0 };
    let line = matches!(p.position, Position::OT | Position::IOL | Position::DT);
    let sd = if line { 0.032 } else { 0.02 };
    sd * (-0.6 * clamp(p.bloom / 1.1, -2.5, 2.5) + 0.8 * own)
}

/// His share of his grown weight at an age: his group's curve plus his own
/// offset, which fades as he fills out.
pub fn fill_share(p: &Player, age: f64) -> f64 {
    let (curve, full) = fill_curve(p.position);
    let k = (age - 14.0).floor();
    let base = if k < 0.0 {
        curve[0] - 0.04 * (14.0 - age)
    } else {
        let i = (k as usize).min(curve.len() - 1);
        let j = (i + 1).min(curve.len() - 1);
        let f = clamp(age - 14.0 - k, 0.0, 1.0);
        curve[i] + (curve[j] - curve[i]) * f
    };
    let fade = clamp((full - age) / (full - 18.0), 0.0, 1.0);
    clamp(base + fill_offset(p) * fade, 0.6, 1.0)
}

/// His body now: boys grow through high school, late bloomers later, and
/// fill out by position group into their early twenties (A12, A13).
pub fn body_now(p: &Player) -> Body {
    let a = p.body;
    let age = p.age as f64;
    let grown = clamp((age - 13.0 - 0.6 * p.bloom) / 6.0, 0.0, 1.0);
    let height = a.height - (1.0 - grown) * 4.0;
    let weight = a.weight * fill_share(p, age);
    let k = height / a.height;
    Body {
        height,
        weight,
        arm: a.arm * k,
        hand: a.hand * (0.9 + 0.1 * grown),
        wingspan: a.wingspan * k,
        frame_weight: a.frame_weight,
        left_handed: a.left_handed,
        left_footed: a.left_footed,
    }
}

/// What his weight adds to his forty, in seconds: 0.0033 s a pound from
/// 220 lb, fit on combine players, who are rarely lighter than about 175 lb.
/// Below that, being lighter stops making him faster (speed update S13: the
/// fit made a 155 lb receiver run 4.15 against a real record of 4.21).
pub fn forty_weight(weight: f64) -> f64 {
    0.0033 * (weight.max(175.0) - 220.0)
}

/// How fast he plays (speed update S2b): one number mixed from his burst
/// (acceleration, the first 10 yards) and his top speed, by how much each
/// matters at his position. Interior linemen almost never run past 10
/// yards; edge rushers still chase from the back side; receivers, corners
/// and safeties live on long speed, though the 10-yard split ties to their
/// playing time too. Not used for kickers, punters and snappers. Read by
/// stars, offers and starters.
pub fn play_speed(pos: Position, speed: f64, accel: f64) -> Option<f64> {
    let burst = match pos {
        Position::DT | Position::OT | Position::IOL => 0.7,
        Position::EDGE | Position::LB => 0.6,
        Position::RB | Position::TE | Position::QB => 0.5,
        Position::WR | Position::CB | Position::S => 0.4,
        Position::K | Position::P | Position::LS => return None,
    };
    Some(burst * accel + (1.0 - burst) * speed)
}

/// One day of testing. None for a drill the position doesn't run
/// (kickers skip the agility drills).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
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

/// The six drills' expected results from the ratings that drive them and
/// his weight and arm length, before the day's noise: forty, vertical,
/// broad, bench, 3-cone, shuttle. Coefficients fitted to the real position
/// averages (nflverse, 2000-2025).
#[derive(Clone, Copy, Debug)]
pub struct Drills {
    pub speed: f64,
    pub accel: f64,
    pub agility: f64,
    pub strength: f64,
    pub jumping: f64,
    pub flexibility: f64,
    pub weight: f64,
    pub arm: f64,
}

impl Drills {
    pub fn of(p: &Player) -> Drills {
        let b = body_now(p);
        let at = |a: Attr| p.attributes.get(a);
        Drills {
            speed: at(Attr::Speed),
            accel: at(Attr::Acceleration),
            agility: at(Attr::Agility),
            strength: at(Attr::Strength),
            jumping: at(Attr::Jumping),
            flexibility: at(Attr::Flexibility),
            weight: b.weight,
            arm: b.arm,
        }
    }

    fn means(&self) -> [f64; 6] {
        let w = self.weight - 220.0;
        let burst = self.jumping.max(0.95 * self.accel);
        // The 3-cone is the scouts' bend test for edge rushers: it reads hip
        // flexibility as well as feet; the short shuttle is change of direction.
        let quick = 0.7 * self.agility + 0.3 * self.accel;
        let bend = 0.55 * self.agility + 0.25 * self.accel + 0.2 * self.flexibility;
        [
            5.56 - 0.0124 * (0.75 * self.speed + 0.25 * self.accel) + forty_weight(self.weight),
            20.9 + 0.182 * burst - 0.035 * w,
            87.6 + 0.448 * burst - 0.089 * w,
            // Long arms mean a longer press: fewer reps for the same strength.
            // Refit 2026-10-14 to drafted players' position means after
            // athlete-first creation (item 6; proposed, not confirmed).
            8.05 + 0.137 * self.strength + 0.073 * w - 0.8 * (self.arm - 32.5),
            7.58 - 0.007 * bend + 0.0062 * w,
            4.59 - 0.0045 * quick + 0.0039 * w,
        ]
    }

    /// What the ratings stand for at the combine, without the test-day
    /// noise or how he runs on a track (update 2, RR4): the number a rating
    /// range is read as.
    pub fn expected(&self) -> Combine {
        let [forty, vertical, broad, bench, cone, shuttle] = self.means();
        tested(forty, vertical, broad, bench, cone, shuttle)
    }
}

fn tested(forty: f64, vertical: f64, broad: f64, bench: f64, cone: f64, shuttle: f64) -> Combine {
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

/// Combine results from his attributes and body, with test-day noise.
pub fn combine(p: &Player, rng: &mut Rng) -> Combine {
    let [forty, vertical, broad, bench, cone, shuttle] = Drills::of(p).means();
    // Speed is how fast he plays. On the track some run faster or slower than
    // they play (a stable sd 0.04 s each, on his own stream), and each run
    // adds its own 0.03 s (proposed, not confirmed).
    let track = Rng::new(p.id ^ 0x40_7AC4).normal(0.0, 0.04);
    let forty = forty + track + rng.normal(0.0, 0.03);
    let vertical = vertical + rng.normal(0.0, 1.5);
    let broad = broad + rng.normal(0.0, 3.0);
    let bench = bench + rng.normal(0.0, 2.0);
    let cone = cone + rng.normal(0.0, 0.08);
    let shuttle = shuttle + rng.normal(0.0, 0.06);
    tested(forty, vertical, broad, bench, cone, shuttle)
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
