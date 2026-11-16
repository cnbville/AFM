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
/// Reps at 225 a point of upper-body strength within a position (physical
/// study item 4, minor bug 385). The 0.137 in `Drills::means` is the slope
/// across positions' means; at it a strength spread of 7 points moved the
/// bench under a rep, so drafted men's bench spread half the real one and a
/// college program barely showed. Set so drafted men's bench spreads as
/// wide as the real combine's at each position (4.0 to 5.4 reps) with the
/// test day's 2 reps and the lifting knack, on average over the positions
/// (`bench_check`, four classes). Proposed, not confirmed.
pub const BENCH_SLOPE: f64 = 0.35;

/// The drafted men's mean upper-body strength at each position (sixteen
/// classes of 250,000, `top_check`, refitted in step 4c with the thin top
/// and the 94 rule gone): the point the within-position slope turns about.
pub const BENCH_CENTER: [f64; POSITION_COUNT] =
    [69.4, 71.7, 66.7, 80.5, 86.2, 84.2, 78.4, 86.2, 73.5, 66.6, 67.7, 50.0, 50.0, 50.0];

/// Reps each position's drafted men sat off the real combine's mean under
/// the cross-position fit (refitted with real arms in step 4a, and on the
/// same sixteen classes in step 4c): added so each drafted position benches
/// the real mean. Kickers, punters and snappers none.
pub const BENCH_OFFSET: [f64; POSITION_COUNT] =
    [1.39, 0.91, -1.9, -0.86, -1.41, -0.09, 2.37, 1.85, 2.22, -1.88, -0.27, 0.0, 0.0, 0.0];

/// The drafted men's mean weight (eight classes, `top_check`): the weight
/// `BENCH_SLOPE` holds at.
pub const BENCH_WEIGHT: f64 = 249.0;

/// The slope's lean past the cross-position one. Reps at 225 follow the
/// pounds on the bar (Epley: reps = 30 × (max ÷ 225 − 1)), and a strength
/// point is a share of his own body's pull, so a point is worth more reps
/// the heavier he is: a 305 lb tackle about 0.49 a point, a 200 lb receiver
/// 0.32 (physical study item 3). With one slope for all, big men's bench
/// spread 4.1-4.3 reps (real 4.8-5.5) and the skill spots' 4.9-5.7 (real
/// 4.0-4.2), and the 35-rep tail fell short. Kickers, punters and snappers
/// keep the old read. Proposed, not confirmed.
fn bench_lean(pos: Position, weight: f64) -> f64 {
    if pos.is_specialist() { 0.0 } else { BENCH_SLOPE * weight / BENCH_WEIGHT - 0.137 }
}

/// The spread of the lifting knack (reps at 225): how a man presses beyond
/// his strength, the groove, arch and grip that are his own for life.
/// Proposed, not confirmed.
pub const LIFT_SD: f64 = 1.5;

/// His lifting knack (reps at 225), on his own keyed stream.
pub fn lift_knack(p: &Player) -> f64 {
    Rng::new(p.id ^ 0x11F7_B3C5).normal(0.0, LIFT_SD)
}

/// The real combine bench average at a position (nflverse 2000-2025).
pub fn combine_bench_mean(pos: Position) -> f64 {
    [19.3, 19.95, 14.68, 20.24, 24.35, 25.81, 23.66, 27.41, 21.97, 14.51, 16.93, 15.2, 16.0][pos.index()]
}

pub fn combine_size(pos: Position) -> (f64, f64, f64, f64) {
    SIZES[pos.index()]
}

/// Drafted men's arms, hands and wingspan by position (MockDraftable,
/// draft classes 2005-2025, `data/real-world/ab_real.json`): (arm mean, sd,
/// hand mean, sd, wingspan mean, sd). Linebackers' arm sd is the other
/// spots' 1.05 (one bad row sets theirs at 1.8). Specialists are the
/// quarterbacks'.
const LIMBS: [(f64, f64, f64, f64, f64, f64); POSITION_COUNT] = [
    (31.89, 1.16, 9.58, 0.47, 75.85, 2.14),  // QB
    (30.89, 1.08, 9.27, 0.47, 74.29, 2.40),  // RB
    (31.84, 1.30, 9.38, 0.51, 76.32, 3.02),  // WR
    (32.99, 0.94, 9.83, 0.49, 79.09, 2.04),  // TE
    (34.21, 0.95, 10.07, 0.52, 82.35, 2.15), // OT
    (33.14, 1.00, 9.93, 0.51, 79.94, 2.14),  // IOL
    (33.38, 1.04, 9.83, 0.51, 80.15, 2.56),  // EDGE
    (33.25, 1.05, 9.95, 0.52, 80.15, 2.43),  // DT
    (32.22, 1.05, 9.60, 0.47, 77.25, 2.25),  // LB
    (31.46, 1.04, 9.13, 0.48, 75.80, 2.22),  // CB
    (31.50, 0.99, 9.33, 0.47, 75.77, 2.13),  // S
    (31.89, 1.16, 9.58, 0.47, 75.85, 2.14),  // K
    (31.89, 1.16, 9.58, 0.47, 75.85, 2.14),  // P
    (31.89, 1.16, 9.58, 0.47, 75.85, 2.14),  // LS
];

/// How much arm comes with an inch of height (physical study item 12):
/// real linemen 0.16-0.23, everyone else 0.33-0.41, so a short lineman who
/// makes it often has long arms. Proposed, not confirmed: 0.20 and 0.37.
pub fn arm_per_inch(pos: Position) -> f64 {
    if lineman(pos) { 0.20 } else { 0.37 }
}

/// A hand's inch per inch of height (kept from the 2025 lists).
const HAND_PER_INCH: f64 = 0.08;

/// Wingspan per inch of arm (both arms plus the shoulders).
const WING_PER_ARM: f64 = 2.03;

/// His arms, hands and wingspan (physical study items 11-13): each spot's
/// real mean at its real height, his height's share at the spot's own
/// slope, and his own part, sized so each spread is real's.
fn limbs(pos: Position, height: f64, rng: &mut Rng) -> (f64, f64, f64) {
    let (am, asd, hm_, hsd, wm, wsd) = LIMBS[pos.index()];
    let (h0, h_sd, _, _) = SIZES[pos.index()];
    let own = |sd: f64, tied: f64| (sd * sd - tied * tied).max(0.0).sqrt().max(0.3 * sd);
    let b = arm_per_inch(pos);
    let arm = am + b * (height - h0) + rng.normal(0.0, own(asd, b * h_sd));
    let hand = hm_ + HAND_PER_INCH * (height - h0) + rng.normal(0.0, own(hsd, HAND_PER_INCH * h_sd));
    let wingspan = wm + WING_PER_ARM * (arm - am) + rng.normal(0.0, own(wsd, WING_PER_ARM * asd));
    (arm, hand, wingspan)
}

impl Body {
    /// Arms, hands and wingspan by his spot's real ones (`limbs`).
    pub(crate) fn grown(pos: Position, height: f64, weight: f64, rng: &mut Rng) -> Body {
        let (arm, hand, wingspan) = limbs(pos, height, rng);
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
    // A linebacker drafted high is no heavier than the combine's (bug 310:
    // synthetic NFL linebackers ran 242-245 lb, real 4-3 starters 232-235).
    // 0.6 sd off his weight brings them to 235-238 lb (proposed, not
    // confirmed).
    let off = if pos == Position::LB { 0.6 } else { 0.0 };
    let w_z = 0.3 + 1.3 * size + 0.45 * strength_z - 0.3 * speed_z + 0.25 * h_z + rng.normal(0.0, 0.8) - off;
    let (h_lo, h_hi) = crate::athlete::HEIGHT_RANGE;
    let (w_lo, w_hi) = crate::athlete::WEIGHT_RANGE;
    let height = clamp(hm + hs * h_z, h_lo, h_hi);
    let weight = clamp(wm + ws * w_z, w_lo, w_hi);
    let (arm, hand, wingspan) = limbs(pos, height, &mut rng);
    let left_handed = rng.chance(LEFT_HANDED);
    let left_footed = rng.chance(if left_handed { 0.6 } else { RIGHTY_LEFT_FOOTED });
    Body { height, weight, arm, hand, wingspan, frame_weight: weight, left_handed, left_footed }
}

/// How a position group fills out (update 3, A12; Cain 2026-10-25: "give
/// lineman room to grow ... a running back or wide receiver should already
/// be around 90%"): his share of his grown weight at 14, 15 and on, until
/// full. Backs, receivers, defensive backs and kickers arrive near full;
/// quarterbacks, tight ends, linebackers, edge rushers and snappers a step
/// behind; tackles, guards, centers and defensive tackles arrive at 96%
/// and add the rest in college (sizes item 3, Cain 2026-10-28: real
/// freshman linemen are listed at 303 lb against 311 for seniors, so a
/// tackle who'll be 310 arrives at about 298). The years before 17, and
/// the linemen's 17 and college years, are proposed, not confirmed.
const FILL_SKILL: [f64; 9] = [0.80, 0.84, 0.87, 0.90, 0.94, 0.97, 0.99, 1.0, 1.0];
const FILL_MID: [f64; 9] = [0.79, 0.83, 0.86, 0.89, 0.93, 0.965, 0.99, 1.0, 1.0];
const FILL_LINE: [f64; 9] = [0.76, 0.80, 0.84, 0.87, 0.96, 0.975, 0.988, 0.996, 1.0];

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
    fill_offset_of(p.position, p.bloom, own)
}

/// The share of his grown weight a fill offset stands for at a position,
/// from his bloom and his own draw (`fill_offset`): public so a projection
/// can draw the offsets it can't see (projected test numbers).
pub fn fill_offset_of(pos: Position, bloom: f64, own: f64) -> f64 {
    let line = matches!(pos, Position::OT | Position::IOL | Position::DT);
    let sd = if line { 0.032 } else { 0.02 };
    sd * (-0.6 * clamp(bloom / 1.1, -2.5, 2.5) + 0.8 * own)
}

/// A draw of the own part of a fill offset as `fill_offset` makes it: one
/// in seven 2.5 times wider (`wide_u` and `z` are the draw's uniform and
/// normal).
pub fn fill_own_draw(wide_u: f64, z: f64) -> f64 {
    z * if wide_u < 0.15 { 2.5 } else { 1.0 }
}

/// His share of his grown weight at an age: his group's curve plus his own
/// offset, which fades as he fills out.
pub fn fill_share(p: &Player, age: f64) -> f64 {
    fill_with(p.position, age, fill_offset(p))
}

/// His group's share of grown weight at an age, before his own offset.
pub fn group_fill(pos: Position, age: f64) -> f64 {
    let (curve, _) = fill_curve(pos);
    let k = (age - 14.0).floor();
    if k < 0.0 {
        curve[0] - 0.04 * (14.0 - age)
    } else {
        let i = (k as usize).min(curve.len() - 1);
        let j = (i + 1).min(curve.len() - 1);
        let f = clamp(age - 14.0 - k, 0.0, 1.0);
        curve[i] + (curve[j] - curve[i]) * f
    }
}

/// The share of grown weight at an age for a given fill offset: his
/// group's curve plus the offset, which fades as he fills out.
pub fn fill_with(pos: Position, age: f64, offset: f64) -> f64 {
    let (_, full) = fill_curve(pos);
    let fade = clamp((full - age) / (full - 18.0), 0.0, 1.0);
    clamp(group_fill(pos, age) + offset * fade, 0.6, 1.0)
}

/// How much of his grown height a boy still has to grow at 13 to 20, as a
/// share of it: the CDC charts' median boy, 5.3 in short at 14, 0.55 at 17,
/// 0.3 at 18 and grown at 20, out of a grown man's 69.6 in (update 3, A15).
const HEIGHT_LEFT: [f64; 8] = [0.1178, 0.0761, 0.0441, 0.0221, 0.0079, 0.0043, 0.0017, 0.0];

/// His share of height still to grow at an age, between the chart's years.
fn height_left(age: f64) -> f64 {
    let x = clamp(age - 13.0, 0.0, 7.0);
    let k = (x.floor() as usize).min(6);
    HEIGHT_LEFT[k] + (HEIGHT_LEFT[k + 1] - HEIGHT_LEFT[k]) * (x - k as f64)
}

/// The share of his grown height a boy has at an age for a bloom (the
/// median boy's curve, moved by his bloom's timing; 1 once grown).
pub fn height_share(age: f64, bloom: f64) -> f64 {
    1.0 - height_left(age - 0.6 * bloom)
}

/// His body now: boys grow through high school, late bloomers later, and
/// fill out by position group into their early twenties (A12, A13).
pub fn body_now(p: &Player) -> Body {
    let a = p.body;
    // By the date (A16): his whole-year age and the part of a year since.
    let age = p.age as f64 + f64::from(p.grow);
    // Height on the median boy's curve, his bloom moving its timing (A15).
    let left = height_left(age - 0.6 * p.bloom);
    let grown = 1.0 - left / HEIGHT_LEFT[0];
    let height = a.height * (1.0 - left);
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

/// The sd of his first-step gift, in seconds of his sprint's time constant
/// (`start_knack`).
pub const START_KNACK: f64 = 0.08;

/// His first step beyond his ratings (physical study item 2, the 10-yard
/// split): his own for life, on his own stream, added to his sprint's time
/// constant (less is quicker). Real drafted men at the same forty and weight
/// split 0.04 s apart (nflverse 2000-2025); the ratings alone gave 0.01 in
/// the engine, since speed and acceleration share one draw. 0.08 is about
/// 0.036 s of split, the rest is the test day. Held to 2.5 sd. Cain's
/// option A, chosen 2026-11-16.
pub fn start_knack(id: u32) -> f64 {
    clamp(Rng::new(id ^ 0x5_7A27).normal(0.0, START_KNACK), -2.5 * START_KNACK, 2.5 * START_KNACK)
}

/// How fast he builds to top speed: the time constant of his sprint, in
/// seconds, from his acceleration, his weight and his first-step gift
/// (`start_knack`; 0 for a typical man). Fit with `ten_split` so every
/// weight band's drafted men split within 0.01 s of real (1.55 under 200 lb
/// to 1.81 over 315), 0.346-0.350 of the forty (real 0.345), with the
/// big men's split moving 0.33 s a second of forty (real 0.28; split_check).
/// Held so his top speed stays under 11.8 yards a second (24 mph): a man
/// who runs a 4.30 doesn't start slowly.
pub fn sprint_tau(accel: f64, weight: f64, knack: f64, forty: f64) -> f64 {
    let tau = clamp(0.84 + 0.0015 * (60.0 - accel) + 0.0004 * (weight - 220.0) + knack, 0.5, 1.3);
    tau.min((forty - 3.4) / SPRINT_TAU_K).max(0.3)
}

/// How much of his sprint's time constant his top speed makes up
/// (`top_speed`): 0.92 keeps his run in the engine the same 0.06 s under his
/// forty whatever his start (it was 1.0, which made a slow starter run his
/// forty 0.03 s quicker than a quick one at the same rating).
pub const SPRINT_TAU_K: f64 = 0.92;

/// Top speed, yards per second, from his forty and his sprint's time
/// constant: with the build-up in `ten_split` he runs his forty in the
/// engine (0.06 s quick, as it always has).
pub fn top_speed(forty: f64, tau: f64) -> f64 {
    40.0 / (forty - SPRINT_TAU_K * tau).max(3.4)
}

/// His grip for braking and cutting, yards per second squared, from his
/// acceleration and weight (`GamePlayer::accel`). The burst from a standing
/// start is `sprint_tau`.
pub fn grip(accel: f64, weight: f64) -> f64 {
    // Fit with `drills` so the sim's 5-10-5 shuttle and 3-cone move per
    // point as the combine model does (`combine`), the way the 40 was fit
    // (dynasty item 1; was 0.035 a point, which made a quick player 3-8
    // times quicker in the sim than at the combine). Below the pro range
    // (most high school players) the gap narrows (proposed, not confirmed).
    let ac = if accel >= 60.0 { 4.65 + 0.008 * (accel - 60.0) } else { 4.65 + 0.0046 * (accel - 60.0) };
    clamp(ac - 0.006 * (weight - 220.0), 2.2, 7.5)
}

/// The 10-yard split of a man with this forty, sprint and grip, run the
/// way the engine runs him from a standing start (`Agent::steer`, 0.1 s
/// steps): the test day's split is the one he plays with.
pub fn ten_split(forty: f64, tau: f64, grip: f64) -> f64 {
    const DT: f64 = 0.1;
    let vmax = top_speed(forty, tau);
    let (mut v, mut x, mut t) = (0.0f64, 0.0f64, 0.0f64);
    while t < 6.0 {
        let up = ((vmax - v).max(0.0) / tau).max(grip * 0.2) * DT;
        v += up.min((vmax - v).max(0.0));
        let was = x;
        x += v * DT;
        if x >= 10.0 {
            return t + DT * (10.0 - was) / (x - was);
        }
        t += DT;
    }
    t
}

/// What his first-step gift does to his 10-yard split on a run with this
/// forty, seconds (negative is quicker): `split_of` against the split his
/// ratings give.
pub fn knack_split(p: &Player, forty: f64) -> f64 {
    let d = Drills::of(p);
    split_of(p, forty) - ten_split(forty, sprint_tau(d.accel, d.weight, 0.0, forty), grip(d.accel, d.weight))
}

/// The spread of the first-step gift in seconds of split, about 0.036
/// (`START_KNACK` through `ten_split` at a typical start).
pub const KNACK_SPLIT_SD: f64 = 0.036;

/// His own 10-yard split on a run with this forty: his acceleration,
/// weight and first-step gift (`start_knack`), no test-day noise.
pub fn split_of(p: &Player, forty: f64) -> f64 {
    let d = Drills::of(p);
    ten_split(forty, sprint_tau(d.accel, d.weight, start_knack(p.id), forty), grip(d.accel, d.weight))
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
    /// The forty's 10-yard split, seconds (physical study item 2): run the
    /// way the engine runs him, with his first-step gift (`split_of`). 0 in
    /// a test day saved before it was kept.
    #[serde(default)]
    pub split: f64,
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
    /// His position: how wide the 3-cone and shuttle spread there.
    pub pos: Position,
    /// His lifting knack, in reps at 225 (item 4 of the physical study):
    /// his own for life (`lift_knack`), 0 for a made-up man.
    pub lift: f64,
    /// His lower body (`Attr::LowerBody`): the legs in the jumps.
    pub lower: f64,
}

/// How the 3-cone and shuttle spread at each position (update 3, S14): as
/// wide as the real combine there (nflverse 2000-2025), with the world's
/// averages kept; before it they spread about half as wide. Half of the
/// missing spread comes from the ratings, stretched around the average
/// drafted player at the position, and half from how well he runs the
/// drill beyond them, his own for life (both proposed, not confirmed): all
/// of it from the ratings would make an average high schooler a second
/// slower. (bend middle, 3-cone stretch, 3-cone knack sd, quickness middle,
/// shuttle stretch, shuttle knack sd), measured with `positions` on 8 seeds;
/// kickers, punters and snappers are left as they were. The bend middles
/// moved with E1's bend weights by k/(k-1) times the shift in the drafted
/// players' average bend (8 seeds), so each position's average 3-cone holds.
const AGILE_SPREAD: [(f64, f64, f64, f64, f64, f64); POSITION_COUNT] = [
    (73.84, 3.23, 0.18, 72.03, 3.27, 0.13), // QB
    (80.87, 3.86, 0.18, 82.97, 3.76, 0.12), // RB
    (79.82, 3.29, 0.16, 81.91, 3.26, 0.11), // WR
    (79.98, 3.19, 0.16, 80.61, 3.09, 0.11), // TE
    (75.68, 3.85, 0.25, 73.95, 3.38, 0.15), // OT
    (73.02, 3.97, 0.25, 71.17, 3.83, 0.16), // IOL
    (80.11, 4.09, 0.21, 81.66, 3.88, 0.14), // EDGE
    (74.55, 3.88, 0.22, 73.65, 3.75, 0.15), // DT
    (80.45, 3.42, 0.17, 81.71, 3.51, 0.12), // LB
    (82.63, 3.31, 0.15, 83.85, 3.44, 0.11), // CB
    (80.58, 3.54, 0.16, 81.94, 3.54, 0.11), // S
    (0.0, 1.00, 0.0, 0.0, 1.00, 0.0),       // K
    (0.0, 1.00, 0.0, 0.0, 1.00, 0.0),       // P
    (0.0, 1.00, 0.0, 0.0, 1.00, 0.0),       // LS
];

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
            pos: p.position,
            lift: lift_knack(p),
            lower: at(Attr::LowerBody),
        }
    }

    /// The 3-cone is the scouts' bend test for edge rushers: it reads hip
    /// flexibility as well as feet. E1 (Cain's yes 2026-11-08): bend's share
    /// rose from 0.2 to 0.35 (Agility 0.55 to 0.45, Acceleration 0.25 to
    /// 0.2), so a 3-cone tells a scout about bend as real scouts read it;
    /// each position's bend middle in `AGILE_SPREAD` moved to keep its
    /// average 3-cone (proposed, not confirmed).
    pub fn bend(&self) -> f64 {
        0.45 * self.agility + 0.2 * self.accel + 0.35 * self.flexibility
    }

    /// The short shuttle is change of direction.
    pub fn quick(&self) -> f64 {
        0.7 * self.agility + 0.3 * self.accel
    }

    fn means(&self) -> [f64; 6] {
        let w = self.weight - 220.0;
        let pos = self.pos;
        // The jumps read his spring and his legs (physical study item 5,
        // Cain 2026-11-15: lower-body strength in the jumps).
        let burst = (1.0 - JUMP_LEGS) * self.jumping.max(0.95 * self.accel) + JUMP_LEGS * self.lower;
        let (bm, ck, _, qm, sk, _) = AGILE_SPREAD[pos.index()];
        let bend = bm + ck * (self.bend() - bm);
        let quick = qm + sk * (self.quick() - qm);
        [
            5.56 - 0.0124 * (0.75 * self.speed + 0.25 * self.accel) + forty_weight(self.weight),
            20.9 + 0.182 * burst - 0.035 * w + spot(&VERT_OFFSET, pos),
            87.6 + 0.448 * burst - 0.089 * w + spot(&BROAD_OFFSET, pos),
            // Long arms mean a longer press: fewer reps for the same strength.
            // Refit 2026-10-14 to drafted players' position means after
            // athlete-first creation (item 6; proposed, not confirmed).
            // Within a position the bench reads upper-body strength at the
            // real slope (`BENCH_SLOPE`, physical study item 4), about the
            // drafted men's mean there, each position's drafted mean set on
            // the real combine's.
            8.05 + 0.137 * self.strength + 0.073 * w - 0.8 * (self.arm - 32.5)
                + bench_lean(self.pos, self.weight) * (self.strength - BENCH_CENTER[self.pos.index()])
                + BENCH_OFFSET[self.pos.index()]
                + self.lift,
            7.58 - 0.007 * bend + 0.0062 * w + spot(&CONE_OFFSET, pos),
            4.59 - 0.0045 * quick + 0.0039 * w + spot(&SHUTTLE_OFFSET, pos),
        ]
    }

    /// What the ratings stand for at the combine, without the test-day
    /// noise or how he runs on a track (update 2, RR4): the number a rating
    /// range is read as.
    /// The split is the one his ratings give at that forty, without his
    /// first-step gift (`start_knack`).
    pub fn expected(&self) -> Combine {
        let [forty, vertical, broad, bench, cone, shuttle] = thin_drills(self.pos, self.means());
        let split = ten_split(forty, sprint_tau(self.accel, self.weight, 0.0, forty), grip(self.accel, self.weight));
        tested(forty, vertical, broad, bench, cone, shuttle, split)
    }

    /// `expected` before it is rounded and held to the drills' limits, in
    /// the order forty, vertical, broad, bench, 3-cone, shuttle (projected
    /// test numbers work on these, then round as `rounded` does).
    pub fn raw(&self) -> [f64; 6] {
        self.means()
    }
}

/// Six raw drill numbers (`Drills::raw`'s order) rounded and held to the
/// drills' limits the way a test day writes them, with the split they go
/// with (`ten_split`, or 0 where none is shown).
pub fn rounded(v: [f64; 6], split: f64) -> Combine {
    tested(v[0], v[1], v[2], v[3], v[4], v[5], split)
}

/// How far a man runs from his ratings on the track, as a stable sd in
/// seconds (`combine`).
pub const TRACK_SD: f64 = 0.04;

/// The sd of the 3-cone and shuttle knacks at a position (`drill_knack`).
pub fn knack_sd(pos: Position) -> (f64, f64) {
    let (_, _, cone, _, _, shuttle) = AGILE_SPREAD[pos.index()];
    (cone, shuttle)
}

/// A test day's own noise on each drill at the combine or a pro day, in
/// `Drills::raw`'s order (`combine`).
pub const DAY_SD: [f64; 6] = [0.03, 1.5, 3.0, 2.0, 0.08, 0.06];

/// A test day writes what he did: the forty and the agility drills to the
/// hundredth, the vertical to the half inch, the broad to the inch. No
/// printed limits (physical study item 8: real men have run past every one
/// the game had, Byron Jones's 147-inch broad among them); only bounds no
/// man reaches, so a broken number can't print.
fn tested(forty: f64, vertical: f64, broad: f64, bench: f64, cone: f64, shuttle: f64, split: f64) -> Combine {
    let r2 = |x: f64| (x * 100.0).round() / 100.0;
    Combine {
        split: if split > 0.0 { r2(clamp(split, 1.0, 3.5)) } else { 0.0 },
        forty: r2(clamp(forty, 3.5, 9.0)),
        bench: clamp(bench, 0.0, 99.0).round(),
        vertical: (clamp(vertical, 0.0, 70.0) * 2.0).round() / 2.0,
        broad: clamp(broad, 0.0, 200.0).round(),
        cone: r2(clamp(cone, 5.0, 14.0)),
        shuttle: r2(clamp(shuttle, 3.0, 9.0)),
    }
}

/// How well he runs the 3-cone and shuttle beyond his ratings, in seconds
/// (S14): his own for life, on his own stream, at his position's spread.
pub fn drill_knack(p: &Player) -> (f64, f64) {
    let (_, _, cone, _, _, shuttle) = AGILE_SPREAD[p.position.index()];
    let mut rng = Rng::new(p.id ^ 0xC0_4E5);
    let (a, b) = (rng.normal(0.0, 1.0), rng.normal(0.0, 1.0));
    // An edge who runs one change-of-direction drill well runs the other
    // well too (E1): real edges' 3-cone and shuttle go together 0.62; with
    // independent knacks the engine's drafted edges gave 0.41.
    let r = if p.position == Position::EDGE { EDGE_KNACK_TOGETHER } else { 0.0 };
    (cone * a, shuttle * (r * a + (1.0 - r * r).sqrt() * b))
}

/// How high and far he jumps beyond his ratings, in inches (physical study
/// item 5): his own for life, on his own stream, one spring for both jumps
/// (real drafted men's vertical and broad go together about 0.7 within a
/// position). A plain normal: the fat tail it had (a t of 5 degrees) ran
/// drafted verticals to 52 inches over 24 drafts and 44+ to 1.4 a draft
/// (real best 46.5 in 25 years, 0.35 a draft); the normal gives 47.5 and
/// 0.50 (`drill_tails`, step 4c). Test days only, like the agility knacks.
pub fn jump_knack(p: &Player) -> (f64, f64) {
    let pos = p.position;
    let mut rng = Rng::new(p.id ^ 0x1A_3B7);
    let a = rng.normal(0.0, 1.0);
    let b = rng.normal(0.0, 1.0);
    let r = JUMP_TOGETHER;
    (spot(&VERT_KNACK, pos) * a, spot(&BROAD_KNACK, pos) * (r * a + (1.0 - r * r).sqrt() * b))
}

/// The jump knack's sd by position (vertical, broad), in inches.
pub fn jump_knack_sd(pos: Position) -> (f64, f64) {
    (spot(&VERT_KNACK, pos), spot(&BROAD_KNACK, pos))
}

/// Real drafted men's 3-cone and shuttle by position (nflverse 2000-2025):
/// (mean, sd), the scale `thin_fast` reads. Specialists none.
const CONE_REAL: [(f64, f64); POSITION_COUNT] = [
    (7.12, 0.22),
    (7.08, 0.24),
    (6.94, 0.20),
    (7.15, 0.22),
    (7.80, 0.28),
    (7.76, 0.28),
    (7.24, 0.25),
    (7.68, 0.27),
    (7.11, 0.21),
    (6.93, 0.21),
    (6.97, 0.21),
    (0.0, 0.0),
    (0.0, 0.0),
    (0.0, 0.0),
];
const SHUTTLE_REAL: [(f64, f64); POSITION_COUNT] = [
    (4.30, 0.16),
    (4.27, 0.16),
    (4.21, 0.15),
    (4.34, 0.15),
    (4.73, 0.18),
    (4.69, 0.19),
    (4.38, 0.16),
    (4.64, 0.18),
    (4.26, 0.14),
    (4.16, 0.15),
    (4.18, 0.14),
    (0.0, 0.0),
    (0.0, 0.0),
    (0.0, 0.0),
];

/// The fast end of a change-of-direction drill thins out (physical study
/// item 7): real tails are thinner than a bell (past 2.5 sd 1.6 and 3.7 a
/// thousand at the 3-cone and shuttle, a bell 6.2), a body can only turn so
/// fast. Past `knee` sds under his position's real mean, each further
/// stretch counts for less and never past `knee + room` sds.
pub fn thin_cone(pos: Position, time: f64) -> f64 {
    thin_fast(time, CONE_REAL[pos.index()])
}

/// The shuttle's thin fast end (`thin_fast`).
pub fn thin_shuttle(pos: Position, time: f64) -> f64 {
    thin_fast(time, SHUTTLE_REAL[pos.index()])
}

/// Six raw drill numbers (`Drills::raw`'s order) with the 3-cone and
/// shuttle's thin fast end, as a body can run them (`thin_fast`).
pub fn thin_drills(pos: Position, mut v: [f64; 6]) -> [f64; 6] {
    v[4] = thin_cone(pos, v[4]);
    v[5] = thin_shuttle(pos, v[5]);
    v
}

fn thin_fast(time: f64, (m, sd): (f64, f64)) -> f64 {
    if sd <= 0.0 {
        return time;
    }
    let z = (time - m) / sd;
    if z >= -THIN_KNEE {
        return time;
    }
    m + sd * (-THIN_KNEE - THIN_ROOM * ((-z - THIN_KNEE) / THIN_ROOM).tanh())
}

/// The lower body's share of what the jumps read (Cain 2026-11-15:
/// lower-body strength in the jumps), the rest his spring (Jumping, or his
/// burst when it is more). Proposed, not confirmed.
pub const JUMP_LEGS: f64 = 0.2;

/// Each position's inches and seconds on the jumps and agility drills
/// (physical study items 5-7), in the order QB, RB, WR, TE, OT, IOL, EDGE,
/// DT, LB, CB, S, so drafted men's averages are the real combine's
/// (nflverse 2000-2025): the broad ran 9 in long at quarterback, 5-6 at
/// backs and 3-4 at receivers, corners and safeties before. Fitted with
/// `drill_tails` (eight 250,000 classes). Specialists none.
const VERT_OFFSET: [f64; 11] = [-2.8, -1.1, -0.3, -0.1, -1.0, -1.1, -0.1, -1.3, 0.3, -0.1, 0.3];
const BROAD_OFFSET: [f64; 11] = [-8.7, -4.8, -2.7, -1.1, -5.2, -5.0, -2.3, -5.0, -1.4, -2.7, -2.8];
const CONE_OFFSET: [f64; 11] = [0.10, 0.11, 0.05, -0.03, 0.14, 0.05, -0.06, 0.09, -0.03, 0.10, -0.02];
const SHUTTLE_OFFSET: [f64; 11] = [0.05, 0.04, 0.05, -0.01, 0.12, 0.04, -0.02, 0.04, -0.07, 0.06, 0.01];

/// The jump knack's spread by position, in inches (vertical, then broad),
/// the part of real's spread the ratings and the day's noise leave
/// (`jump_knack`). Fitted with `drill_tails`. Proposed, not confirmed.
const VERT_KNACK: [f64; 11] = [1.95, 2.24, 2.5, 1.69, 1.46, 1.65, 2.3, 1.65, 2.1, 2.4, 2.6];
const BROAD_KNACK: [f64; 11] = [2.8, 3.5, 4.3, 1.5, 1.5, 3.2, 4.05, 3.2, 2.9, 4.3, 4.4];

/// How much his vertical and broad knacks go together (real drafted men's
/// two jumps about 0.7 within a position). Proposed, not confirmed.
const JUMP_TOGETHER: f64 = 0.7;

/// Where the fast end of the 3-cone and shuttle starts to thin, in sds
/// under the position's real mean, and how much further it can reach
/// (`thin_fast`). Fitted with `drill_tails`: 3-cones of 6.50 or faster
/// 0.38 a draft (real 0.35 of the men who ran it), shuttles of 3.85 0.75
/// (real 0.58). Proposed, not confirmed.
const THIN_KNEE: f64 = 0.9;
const THIN_ROOM: f64 = 1.0;

/// A position's entry in an 11-spot table (specialists 0).
fn spot(t: &[f64; 11], pos: Position) -> f64 {
    t.get(pos.index()).copied().unwrap_or(0.0)
}

/// How much an edge's 3-cone and shuttle knacks go together (E1), set so
/// drafted edges' two drills go together as real ones do (0.62, 2005-2025
/// combine edges with both): 0.7 gives 0.60 (`positions`, one class; 0.42
/// gave 0.42, 0.8 gave 0.68). Other positions keep independent knacks
/// (proposed, not confirmed).
const EDGE_KNACK_TOGETHER: f64 = 0.7;

/// Combine results from his attributes and body, with test-day noise.
pub fn combine(p: &Player, rng: &mut Rng) -> Combine {
    let [forty, vertical, broad, bench, cone, shuttle] = Drills::of(p).means();
    // Speed is how fast he plays. On the track some run faster or slower than
    // they play (a stable sd 0.04 s each, on his own stream), and each run
    // adds its own 0.03 s (proposed, not confirmed).
    let track = Rng::new(p.id ^ 0x40_7AC4).normal(0.0, 0.04);
    let forty = forty + track + rng.normal(0.0, 0.03);
    let (vert_knack, broad_knack) = jump_knack(p);
    let vertical = vertical + vert_knack + rng.normal(0.0, 1.5);
    let broad = broad + broad_knack + rng.normal(0.0, 3.0);
    let bench = bench + rng.normal(0.0, 2.0);
    let (cone_knack, shuttle_knack) = drill_knack(p);
    let cone = thin_cone(p.position, cone + cone_knack) + rng.normal(0.0, 0.08);
    let shuttle = thin_shuttle(p.position, shuttle + shuttle_knack) + rng.normal(0.0, 0.06);
    // The split of that same run, with its own 0.02 s (laser timing at 10
    // yards; drawn last so the six drills keep their draws).
    let split = split_of(p, forty) + rng.normal(0.0, SPLIT_DAY_SD);
    tested(forty, vertical, broad, bench, cone, shuttle, split)
}

/// A test day's own noise on the 10-yard split, seconds (`combine`).
pub const SPLIT_DAY_SD: f64 = 0.02;

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
