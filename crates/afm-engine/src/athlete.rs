//! Athlete-first player creation (section 3, topic 5.2).
//!
//! A 14-year-old is an athlete before he is a position. Creation draws his
//! body first (height and build, from the real combine pooled over the
//! positions high schools field), then his athletic traits from that body
//! plus his own explosiveness, then skill leanings (arm, hands, toughness,
//! cover instincts, a kicking leg, football IQ) that have nothing to do with
//! any position. Only then does a coach place him, by what he sees and what
//! his roster needs. Positions keep changing as he grows and as college and
//! pro staffs look at him.
//!
//! Talent still sets how good he can become at the spot that suits him best,
//! so the calibrated talent model (five-star and draft rates) carries over.
//! At a spot that suits him less, he is worse: a kid with a quarterback's arm
//! stuck at safety, or a safety's eye wasted at receiver.

use crate::attributes::{ATTRIBUTE_PROFILES, Attr, Attributes, MIN_RATING, RATING_BOUND};
use crate::body::{Body, combine_size};
use crate::player::Player;
use crate::positions::{POSITION_COUNT, Position, elite_shape_value, overall_for, weight};
use std::sync::LazyLock;

use crate::rng::{Rng, clamp};

/// Positions on the field, kickers aside: their bodies set the athletic
/// profile a body tends to come with.
const FIELD: [Position; 11] = [
    Position::QB,
    Position::RB,
    Position::WR,
    Position::TE,
    Position::OT,
    Position::IOL,
    Position::EDGE,
    Position::DT,
    Position::LB,
    Position::CB,
    Position::S,
];

/// The positions a coach places a boy at: all but long snapper. High school
/// snappers play another position; colleges list them as snappers at
/// signing day (fix list 1.7).
const PLACED: [Position; 13] = [
    Position::QB,
    Position::RB,
    Position::WR,
    Position::TE,
    Position::OT,
    Position::IOL,
    Position::EDGE,
    Position::DT,
    Position::LB,
    Position::CB,
    Position::S,
    Position::K,
    Position::P,
];

const PHYSICAL: [Attr; 8] = [
    Attr::Speed,
    Attr::Acceleration,
    Attr::Agility,
    Attr::Strength,
    Attr::Balance,
    Attr::Flexibility,
    Attr::Jumping,
    Attr::Stamina,
];

/// How much the lower and upper body's own parts go together (physical
/// study item 4b; proposed, not confirmed: bench and squat strength go
/// together about 0.7 to 0.8 among trained athletes, McGuigan and
/// Winchester 2008).
pub const LOWER_TIE: f64 = 0.75;

/// The lower body's own part given the upper body's (`upper`, drawn at
/// spread `sd`): tied at `LOWER_TIE`, the rest drawn on a stream keyed to
/// what is already drawn, so no other draw in the generator moves.
pub(crate) fn lower_with(upper: f64, sd: f64, key: u32) -> f64 {
    let z = Rng::new(key ^ (upper as f32).to_bits() ^ 0x10B0_D1E5).normal(0.0, 1.0);
    LOWER_TIE * upper + (1.0 - LOWER_TIE * LOWER_TIE).sqrt() * sd * z
}

/// Standard normal cumulative distribution (Abramowitz and Stegun 7.1.26).
pub fn phi(x: f64) -> f64 {
    let t = 1.0 / (1.0 + 0.3275911 * x.abs() / std::f64::consts::SQRT_2);
    let poly = t * (0.254829592 + t * (-0.284496736 + t * (1.421413741 + t * (-1.453152027 + t * 1.061405429))));
    let erf = 1.0 - poly * (-x * x / 2.0).exp();
    if x >= 0.0 { 0.5 * (1.0 + erf) } else { 0.5 * (1.0 - erf) }
}

/// A skill leaning on the elite-shape scale (about 40 to 97). Most kids sit
/// near 60; a strong natural gift (two standard deviations) reaches the mid 90s.
fn leaning(z: f64) -> f64 {
    38.0 + 59.0 * phi(1.2 * (z - 0.3))
}

/// How much a body looks like the typical body at a position, from the real
/// combine: 1 at the position's average, falling off with distance.
fn body_likeness(height: f64, weight: f64, pos: Position) -> f64 {
    let (hm, hs, wm, ws) = combine_size(pos);
    let dh = (height - hm) / hs;
    let dw = (weight - wm) / ws;
    (-0.5 * (dh * dh + dw * dw)).exp()
}

/// How talent shrinks a body: kids who top out lower are smaller than draft
/// prospects. Returns (inches, pounds) added to the draft-prospect frame.
pub(crate) fn talent_size(talent: f64) -> (f64, f64) {
    let size = clamp((talent - 72.0) / 12.0, -2.5, 0.5);
    (0.7 * size * 1.6, (0.3 + 1.3 * size) * 14.0)
}

/// The frame a boy grows into, as if he were a draft prospect: about three in
/// ten have a lineman's build (the share of linemen on real rosters), the
/// rest are lean, with height and weight tied together. The heights are
/// tuned so grown heights by position match the real ones (update 3, A18:
/// FBS against 2025 rosters, drafted players against the combine; were 75.9
/// and 73.1).
fn draw_frame(rng: &mut Rng) -> (f64, f64) {
    let h = rng.normal(0.0, 1.0);
    if rng.chance(0.29) {
        (76.4 + 1.5 * h, 311.0 + 4.0 * h + rng.normal(0.0, 15.0))
    } else {
        (73.5 + 2.1 * h, 220.0 + 26.0 * (0.55 * h + 0.83 * rng.normal(0.0, 1.0)))
    }
}

/// The heavy side of a tackle's, guard's or defensive tackle's weight
/// against its light side (item 10): real linemen's heavy tail runs long
/// (0.41% of FBS and FCS listings at 350+, drafted tackles' heaviest 1%
/// 365 lb). It widens his weight once his spot is chosen (`fitted_size`),
/// not his frame, so who becomes a lineman and where stays as drawn.
/// Proposed, not confirmed: sized on `body_sizes`. One stretch can't give
/// real's narrow middle and long tail: the placement's weight program
/// takes a heavy boy 60% of the way back (at most 25 lb). At 1.6 drafted
/// tackles weigh 315 (sd 15, real 13), their heaviest 1% 346 (real 365);
/// at 2.0 the tail reaches 353 but the class sim then drafts tackles at
/// 16% of picks (real 8%, its own 12% at 1.6).
pub const HEAVY_SIDE: f64 = 1.6;

/// The widest body a boy can be drawn with (physical study item 9: no
/// creation limits inside what people are). Real combine men run 64 to 82
/// in and up to 384 lb, real rosters list men past 400.
pub const HEIGHT_RANGE: (f64, f64) = (58.0, 88.0);
pub const WEIGHT_RANGE: (f64, f64) = (120.0, 480.0);

/// Each position's size fit at creation, after his spot is chosen
/// (physical study items 14 and 15, and the rest of the real combine's
/// heights and weights): (inches added, the extra height spread's sd,
/// pounds added, the weight spread's scale about the combine's mean; for
/// tackles, guards and defensive tackles only on the light side, the heavy
/// side taking `HEAVY_SIDE`).
/// Without it the spots that share a frame squeeze each other's sizes:
/// receivers' heights ran sd 1.51 (real 2.26) with no 5'8 man in a
/// thousand drafts (real 3.7%), and receivers 7.6 lb, corners 3.5 and
/// safeties 2.2 heavy. Fitted on `body_sizes` (four 250,000 classes,
/// drafted men against the combine 2000-2026). Proposed, not confirmed.
pub const SIZE_FIT: [(f64, f64, f64, f64); POSITION_COUNT] = [
    (0.70, 0.0, -8.6, 0.84),    // QB
    (-0.27, 1.29, 4.3, 1.07),   // RB
    (-0.39, 1.68, -12.7, 0.95), // WR
    (0.14, 0.0, -7.0, 0.57),    // TE
    (0.16, 0.0, -6.8, 0.76),    // OT
    (0.0, 0.0, -0.5, 0.64),     // IOL
    (-0.05, 0.0, -2.6, 0.79),   // EDGE
    (0.0, 0.0, -14.5, 0.83),    // DT
    (-0.33, 1.13, -0.2, 0.92),  // LB
    (0.26, 0.97, -6.2, 0.49),   // CB
    (0.41, 1.25, -9.6, 0.65),   // S
    (0.0, 0.0, 0.0, 1.0),       // K
    (0.0, 0.0, 0.0, 1.0),       // P
    (0.0, 0.0, 0.0, 1.0),       // LS
];

/// His height and weight after his spot's size fit (`SIZE_FIT`), from his
/// frame and the talent's shrink. The extra height draw is on its own
/// stream keyed by his frame, so no other draw moves.
pub(crate) fn fitted_size(natural: Position, fh: f64, fw: f64, talent: f64) -> (f64, f64) {
    let (dh, dw) = talent_size(talent);
    let (fit_h, extra, fit_w, scale) = SIZE_FIT[natural.index()];
    let mut r = Rng::new(fh.to_bits() as u32 ^ (fw.to_bits() >> 20) as u32 ^ 0x51_2E5F);
    let height = clamp(fh + dh + fit_h + r.normal(0.0, extra), HEIGHT_RANGE.0, HEIGHT_RANGE.1);
    let wm = combine_size(natural).2;
    let w = lb_weight(natural, fw + dw + fit_w);
    // An interior lineman's heavy side runs long (item 10).
    let heavy = matches!(natural, Position::OT | Position::IOL | Position::DT) && w > wm;
    let weight = clamp(wm + if heavy { HEAVY_SIDE } else { scale } * (w - wm), WEIGHT_RANGE.0, WEIGHT_RANGE.1);
    (height, weight)
}

/// The athlete's shape: what each attribute would be if he became an elite
/// player, before talent scales it. Returns the shape, the frame, his will
/// to compete (a standard normal, which his personality shares) and his
/// explosiveness (a standard normal: his own speed and burst roll).
fn draw_shape(rng: &mut Rng) -> (Attributes, (f64, f64), f64, f64) {
    use Attr::*;
    let (fh, fw) = draw_frame(rng);
    let mut shape = Attributes::filled(45.0);

    // Athletic profile from the body: blend what bodies like his are like at
    // the combine, then add his own explosiveness, hips and strength.
    let likeness: Vec<(Position, f64)> =
        FIELD.iter().map(|&p| (p, body_likeness(fh, fw, p) * p.profile().share + 1e-9)).collect();
    let total: f64 = likeness.iter().map(|(_, l)| l).sum();
    for a in PHYSICAL {
        shape.set(a, likeness.iter().map(|&(p, l)| l * elite_shape_value(p, a)).sum::<f64>() / total);
    }
    let explosive = rng.normal(0.0, 1.0);
    shape.add(Speed, 4.0 * explosive + rng.normal(0.0, 3.0));
    shape.add(Acceleration, 4.0 * explosive + rng.normal(0.0, 3.0));
    shape.add(Jumping, 4.0 * explosive + rng.normal(0.0, 3.0));
    shape.add(Agility, 2.5 * explosive + rng.normal(0.0, 3.5));
    let own_strength = rng.normal(0.0, 5.0);
    shape.add(Strength, own_strength);
    // The lower body moves with the upper mostly, but men differ (physical
    // study item 4b), on its own stream so no other draw moves.
    shape.set(LowerBody, shape.get(Strength) - own_strength + lower_with(own_strength, 5.0, fh.to_bits() as u32));
    shape.add(Stamina, rng.normal(0.0, 5.0));
    // Balance and bend are their own gifts (proposed, not confirmed: about
    // as spread as agility).
    shape.add(Balance, 1.5 * explosive + rng.normal(0.0, 4.0));
    shape.add(Flexibility, 1.0 * explosive + rng.normal(0.0, 5.0));

    // Skill leanings, independent of the body: a gift that shows in several
    // attributes gets a shared draw, mixed with each attribute's own. The
    // gifts follow what scouts see together: an arm, ball placement, a
    // passer's mechanics and his feel; hands, routes, tracking and body
    // control; a lineman's hands and his power; cover instincts; a leg; the
    // mind (football IQ, recognition, the snap, decisions, poise, focus,
    // self-control, leading); and the will to compete under it all.
    let mut z = || rng.normal(0.0, 1.0);
    let (arm, place, mech, feel, hands, route) = (z(), z(), z(), z(), z(), z());
    let (ball, elude, power, block, feet, tough) = (z(), z(), z(), z(), z(), z());
    let (tackle, shed, rush, cover, mirror, leg) = (z(), z(), z(), z(), z(), z());
    let (kick_touch, iq, recog, aware, vision, poise) = (z(), z(), z(), z(), z(), z());
    let (track, body, hand, snap, compete, engine) = (z(), z(), z(), z(), z(), z());
    let (focus, disc, lead, hit, antic, decide) = (z(), z(), z(), z(), z(), z());
    let (punt_leg, punt_touch, split) = (z(), z(), z());
    let mix = |parts: &[(f64, f64)]| {
        let norm = parts.iter().map(|(w, _)| w * w).sum::<f64>().sqrt();
        parts.iter().map(|(w, x)| w * x).sum::<f64>() / norm
    };
    shape.set(ArmStrength, leaning(arm));
    shape.set(ShortAccuracy, leaning(mix(&[(0.3, arm), (0.9, place)])));
    shape.set(MediumAccuracy, leaning(mix(&[(0.45, arm), (0.85, place)])));
    shape.set(DeepAccuracy, leaning(mix(&[(0.7, arm), (0.7, place)])));
    shape.set(Touch, leaning(mix(&[(0.8, place), (0.5, feel)])));
    shape.set(ThrowOnMove, leaning(mix(&[(0.6, place), (0.5, arm), (0.5, explosive)])));
    shape.set(QuickRelease, leaning(mix(&[(0.7, mech), (0.5, arm)])));
    shape.set(Footwork, leaning(mix(&[(0.8, mech), (0.5, feet)])));
    shape.set(PocketPresence, leaning(mix(&[(0.7, feel), (0.5, poise)])));
    shape.set(BallHandling, leaning(mix(&[(0.6, mech), (0.6, hands)])));
    shape.set(EyeManipulation, leaning(mix(&[(0.7, feel), (0.6, iq)])));
    shape.set(BallSecurity, leaning(mix(&[(0.6, ball), (0.8, tough)])));
    shape.set(BallCarrierVision, leaning(vision));
    shape.set(Elusiveness, leaning(mix(&[(0.85, elude), (0.4, explosive)])));
    shape.set(LegDrive, leaning(mix(&[(0.7, power), (0.5, tough), (0.3, compete)])));
    shape.set(Catching, leaning(hands));
    shape.set(ContestedCatch, leaning(mix(&[(0.6, hands), (0.5, compete), (0.4, power)])));
    shape.set(BallTracking, leaning(mix(&[(0.7, track), (0.5, hands)])));
    shape.set(BodyControl, leaning(mix(&[(0.7, body), (0.5, explosive)])));
    shape.set(RouteRunning, leaning(mix(&[(0.5, hands), (0.85, route)])));
    shape.set(Release, leaning(mix(&[(0.7, route), (0.4, hand), (0.4, explosive)])));
    shape.set(PassBlocking, leaning(mix(&[(0.75, block), (0.55, feet)])));
    shape.set(RunBlocking, leaning(mix(&[(0.65, block), (0.6, tough)])));
    shape.set(BlockingInSpace, leaning(mix(&[(0.6, block), (0.6, feet), (0.3, explosive)])));
    shape.set(HandUse, leaning(mix(&[(0.7, hand), (0.4, block), (0.4, rush)])));
    shape.set(Anchor, leaning(mix(&[(0.7, power), (0.5, block), (0.3, tough)])));
    shape.set(PowerRush, leaning(mix(&[(0.7, rush), (0.6, power)])));
    shape.set(FinesseRush, leaning(mix(&[(0.8, rush), (0.4, explosive), (0.3, hand)])));
    shape.set(BlockShedding, leaning(mix(&[(0.6, tough), (0.8, shed)])));
    shape.set(ManCoverage, leaning(mix(&[(0.8, cover), (0.6, mirror)])));
    shape.set(ZoneCoverage, leaning(mix(&[(0.7, cover), (0.7, recog)])));
    shape.set(Press, leaning(mix(&[(0.6, mirror), (0.5, power), (0.4, hand)])));
    shape.set(Disguise, leaning(mix(&[(0.6, cover), (0.7, iq)])));
    shape.set(Tackling, leaning(mix(&[(0.7, tough), (0.7, tackle)])));
    shape.set(Pursuit, leaning(mix(&[(0.7, tackle), (0.6, recog), (0.3, engine)])));
    shape.set(Blitzing, leaning(mix(&[(0.6, rush), (0.6, recog), (0.3, explosive)])));
    shape.set(Traffic, leaning(mix(&[(0.6, shed), (0.6, recog), (0.4, body)])));
    // A real kicking leg is rarer than any other gift.
    // A real leg comes with a kicker's or punter's body: lean, and taller for
    // punting's longer lever (item 7; proposed, not confirmed).
    let kick_body = body_likeness(fh, fw, Position::K).max(body_likeness(fh, fw, Position::P));
    shape.set(KickPower, leaning(leg + 0.8 * (kick_body - 0.4) - 0.4));
    // One kicker is lethal short, another long (Raichel, topic 7): one base
    // and a lean, with a big leg helping from distance.
    shape.set(ShortKickAccuracy, leaning(mix(&[(0.2, leg), (0.9, kick_touch)]) - 0.4 + 0.3 * split));
    shape.set(LongKickAccuracy, leaning(mix(&[(0.55, leg), (0.75, kick_touch)]) - 0.4 - 0.3 * split));
    shape.set(KickTrajectory, leaning(mix(&[(0.6, kick_touch), (0.6, leg)]) - 0.4));
    shape.set(KickoffTechnique, leaning(mix(&[(0.5, leg), (0.7, kick_touch)]) - 0.4));
    let punt = mix(&[(0.7, leg), (0.7, punt_leg)]);
    shape.set(PuntPower, leaning(punt + 0.8 * (kick_body - 0.4) - 0.4));
    shape.set(PuntPlacement, leaning(mix(&[(0.4, punt), (0.8, punt_touch)]) - 0.4));
    shape.set(GetOff, leaning(mix(&[(0.6, kick_touch), (0.5, mech)]) - 0.4));
    // Long snapping is a rare, taught knack; most centers never learn it.
    shape.set(LongSnapping, leaning(snap - 0.6));
    shape.set(ShotgunSnap, leaning(mix(&[(0.6, snap), (0.6, hands)])));
    shape.set(FootballIq, leaning(mix(&[(0.75, iq), (0.65, aware)])));
    shape.set(PlayRecognition, leaning(mix(&[(0.75, iq), (0.65, recog)])));
    shape.set(Anticipation, leaning(mix(&[(0.6, iq), (0.5, recog), (0.5, antic)])));
    shape.set(Decisions, leaning(mix(&[(0.6, iq), (0.5, poise), (0.5, decide)])));
    shape.set(Composure, leaning(poise));
    shape.set(Concentration, leaning(mix(&[(0.8, focus), (0.4, poise)])));
    shape.set(Discipline, leaning(mix(&[(0.8, disc), (0.4, iq)])));
    shape.set(Communication, leaning(mix(&[(0.6, iq), (0.7, lead)])));
    shape.set(Toughness, leaning(mix(&[(0.8, tough), (0.5, compete)])));
    shape.set(Motor, leaning(mix(&[(0.8, engine), (0.5, compete)])));
    shape.set(Competitiveness, leaning(compete));
    shape.set(Physicality, leaning(mix(&[(0.6, hit), (0.5, tough), (0.4, compete)])));
    (shape, (fh, fw), compete, explosive)
}

/// How much one standard deviation of talent is worth in rating points
/// (`player::talent_from_z`, below its tail).
const TALENT_SD: f64 = 6.5;

/// How widely speed for his size varies among boys with the same build, in
/// rating points (speed update S1, S11; proposed, not confirmed: set so
/// quarterbacks and linemen, whose spots barely read speed, keep the real
/// combine's spread of about 0.18 s).
const SPEED_SD: f64 = 13.0;

/// How much of a class's talent spread is left among the FBS players he is
/// judged against for "no lean" (speed update S1: the fastest tenth at a
/// position, half of them below the position's skills): the variance of
/// talent above about the top 2.5% of a class, against the whole class.
const FBS_TALENT_SHARE: f64 = 0.13;

/// What boys built like a position's players run for their size on
/// average, before anyone picks them, as the 0.75 speed + 0.25
/// acceleration mix of the forty (speed update S11, S12). Set so each
/// position's drafted players run the real combine's average forty
/// (2000-2025, `data/real-world/combine_by_position.csv`), measured over
/// eight classes with the `positions` tool: a spot that reads speed picks
/// its fastest on the way to the draft, so its boys start slower than its
/// pros run. Kickers and punters run like real kickers and punters
/// (4.91). Quarterbacks stay about 0.05 s slower than the combine: the
/// fastest boys with an arm suit receiver or safety better. Refit by the
/// speed lane (2026-11-17, `speed_levels`, eight classes) so each
/// position's drafted average held when speed stopped drifting with the
/// yearly drive and the fast end thinned (`thin_roll`). Order:
/// `Position::ALL`.
const SPEED_MIX: [f64; 14] = [61.0, 44.0, 47.9, 42.2, 40.0, 37.7, 42.5, 38.0, 32.5, 35.1, 50.2, 46.3, 50.8, 45.0];

/// How much his speed roll leans toward his talent at a spot whose overall
/// counts speed and burst at `share`: enough that among FBS players, the
/// ones a staff judges speed and skills against, speed says nothing about
/// his skills (Cain 2026-10-25: "No lean"). A spot that barely reads speed
/// barely leans (quarterbacks 0.25, tackles 0.32); a receiver's leans most
/// (0.89), since speed is a fifth of what makes him a receiver.
fn speed_lean(share: f64) -> f64 {
    if share <= 0.0 {
        return 0.0;
    }
    let (a, v) = (share * SPEED_SD, FBS_TALENT_SHARE);
    let b = TALENT_SD * v;
    clamp((-b + (b * b + 4.0 * a * a * (1.0 - v)).sqrt()) / (2.0 * a * (1.0 - v)), 0.0, 0.95)
}

/// Where a speed roll starts to thin, in sds, how much each sd past it
/// counts, and the least lean a spot needs for it (the speed lane,
/// physical study item 2: real speed tails are thinner than a bell). At a
/// spot that reads speed, football talent leans his roll up, and with no
/// end to it the best receivers, corners and backs ran 4.2s far more often
/// than real ones: drafted men at 4.30 or faster 4.0 a draft, real 1.27.
/// Past the knee each further sd of roll counts for `SPEED_THIN` of one, with
/// no top, so a freak still runs a 4.2. Quarterbacks and linemen, whose
/// rolls are mostly their own explosiveness, keep the bell: their fast men
/// were already as common as real ones (300+ lb at 4.95 or faster). Fitted
/// with `speed_levels`. Proposed, not confirmed.
const SPEED_KNEE: f64 = 2.5;
const SPEED_THIN: f64 = 0.4;
const THIN_LEAN: f64 = 0.6;

fn thin_roll(r: f64, lean: f64) -> f64 {
    if lean < THIN_LEAN || r <= SPEED_KNEE { r } else { SPEED_KNEE + SPEED_THIN * (r - SPEED_KNEE) }
}

fn speed_mix(pos: Position) -> f64 {
    SPEED_MIX[pos.index()]
}

/// A body's speed for his size and his burst against it, as the mean of
/// the positions whose bodies look like his (weighted as in `draw_shape`).
/// Kickers and punters run like real kickers and punters (S12). The lead
/// is how much more burst than top speed his kind of body has, from the
/// positions' elite shapes: big men get off the ball better than they run.
fn speed_base(frame: (f64, f64), natural: Position) -> (f64, f64) {
    if matches!(natural, Position::K | Position::P) {
        return (speed_mix(natural), 0.0);
    }
    let (mut mix, mut lead, mut total) = (0.0, 0.0, 0.0);
    for p in FIELD {
        let l = body_likeness(frame.0, frame.1, p) * p.profile().share + 1e-9;
        mix += l * speed_mix(p);
        lead += l * (elite_shape_value(p, Attr::Acceleration) - elite_shape_value(p, Attr::Speed));
        total += l;
    }
    (mix / total, lead / total)
}

/// The z-score behind a talent (`player::talent_from_z` inverted).
fn talent_z(talent: f64) -> f64 {
    let knee = 45.0 + TALENT_SD * 1.6;
    if talent <= knee {
        (talent - 45.0) / TALENT_SD
    } else {
        1.6 + (-TALENT_SD + (TALENT_SD * TALENT_SD + 11.0 * (talent - knee)).sqrt()) / 5.5
    }
}

/// How a staff weighs a fit at each position against the others. A coach
/// judges a boy against what players at each position usually look like, and
/// positions that blend many skills (linebacker, safety, tight end) rarely
/// show one standout trait the way a receiver's hands or a lineman's size do.
/// These factors are set once, from a large sample of athletes, so that the
/// spots athletes suit best come out in the real shares of high school
/// rosters. They describe how coaches compare, not any one player.
static FIT_SCALE: LazyLock<[f64; POSITION_COUNT]> = LazyLock::new(|| {
    let mut rng = Rng::new("athlete-fit-calibration");
    let sample: Vec<[f64; POSITION_COUNT]> = (0..40_000)
        .map(|_| {
            let (shape, (h, w), _, _) = draw_shape(&mut rng);
            Position::ALL.map(|p| overall_for(p, &shape) * (1.0 - misfit(h, w, p)))
        })
        .collect();
    let total: f64 = Position::ALL.iter().map(|&p| roster_need(p)).sum();
    let mut scale = [1.0; POSITION_COUNT];
    for _ in 0..300 {
        let mut count = [0.0; POSITION_COUNT];
        for row in &sample {
            count[argmax_scaled(row, &scale)] += 1.0;
        }
        for pos in PLACED {
            let k = pos.index();
            let target = roster_need(pos) / total;
            let actual = (count[k] / sample.len() as f64).max(1e-4);
            scale[k] *= clamp(0.2 * (target / actual).ln(), -0.004, 0.004).exp();
        }
    }
    scale
});

fn argmax_scaled(row: &[f64; POSITION_COUNT], scale: &[f64; POSITION_COUNT]) -> usize {
    PLACED
        .iter()
        .map(|p| p.index())
        .max_by(|&a, &b| (row[a] * scale[a]).total_cmp(&(row[b] * scale[b])))
        .expect("positions")
}

/// The position a shape suits best, and his overall there.
fn best_fit(shape: &Attributes, frame: (f64, f64)) -> (Position, f64) {
    let row = Position::ALL.map(|p| overall_for(p, shape) * (1.0 - misfit(frame.0, frame.1, p)));
    let k = argmax_scaled(&row, &FIT_SCALE);
    (Position::ALL[k], overall_for(Position::ALL[k], shape))
}

/// Ceilings, his body, his natural position and his will to compete (a
/// standard normal) for a new athlete of the given talent. Talent is how
/// good he can become at the spot that suits him best.
pub(crate) fn create(rng: &mut Rng, talent: f64) -> (Attributes, Body, Position, f64) {
    let (shape, (fh, fw), compete, explosive) = draw_shape(rng);
    let (natural, fit) = best_fit(&shape, (fh, fw));
    // Talent is his overall at his best spot. The skills that spot needs rise
    // together, each about his talent as his leanings say.
    // Everything else scales with talent the way his leanings say.
    let scale = talent / fit;
    // Physical study item 21: the old "94 rule" squeezed an elite's key
    // skills together so none passed 94 (a 99 wall's leftover). Real
    // first-rounders' profiles are no more even than anyone's, so his
    // leanings spread at every talent; the thin top and the shift below
    // keep his overall at his talent.
    let spread = 0.5 * scale;
    // Speed and burst are his own roll, not his football talent's (speed
    // update S1, S2): his explosiveness on top of what bodies like his run.
    // A spot that reads speed counts it in his talent, so there his roll
    // leans toward his talent and his skills carry the rest, with no lean
    // left between speed and skills among the players who make it.
    let speed_share = weight(natural, Attr::Speed) + weight(natural, Attr::Acceleration);
    let lean = speed_lean(speed_share);
    let roll = thin_roll(lean * talent_z(talent) + (1.0 - lean * lean).sqrt() * explosive, lean);
    let (mix, lead) = speed_base((fh, fw), natural);
    let speed = mix - 0.25 * lead + SPEED_SD * roll;
    let accel = mix + 0.75 * lead + SPEED_SD * roll;
    let mut base = Attributes::default();
    for a in Attr::ALL {
        let v = match a {
            Attr::Speed => speed,
            Attr::Acceleration => accel,
            _ if weight(natural, a) > 0.0 => talent + (shape.get(a) - fit) * spread,
            _ => shape.get(a) * scale,
        };
        base.set(a, v);
    }
    // His skills make up what his speed doesn't, so his overall there is his
    // talent, read after the thin top (a key past the knee counts for less,
    // so an elite's skills carry a little more to make it up).
    if speed_share < 1.0 {
        for _ in 0..8 {
            let mut thinned = base;
            thin_tops(&mut thinned);
            let short = (talent - overall_for(natural, &thinned)) / (1.0 - speed_share);
            if short.abs() < 0.01 {
                break;
            }
            for a in Attr::ALL {
                if weight(natural, a) > 0.0 && !matches!(a, Attr::Speed | Attr::Acceleration) {
                    base.add(a, short);
                }
            }
        }
    }
    // His body ties to his leanings at safety and linebacker (safety
    // proposal D, linebacker L3; Cain's yes 2026-11-07): a heavier man leans
    // to hitting (and a slower linebacker too), a lighter or faster one to
    // range and coverage, at the same overall.
    body_lean(&mut base, natural, talent, fw, roll, speed_share);
    let mut ceilings = Attributes::default();
    let mut strength_noise = 0.0;
    for a in Attr::ALL {
        if a == Attr::LowerBody {
            continue;
        }
        let n = rng.normal(0.0, 4.0);
        if a == Attr::Strength {
            strength_noise = n;
        }
        ceilings.set(a, clamp(base.get(a) + n, MIN_RATING, RATING_BOUND));
    }
    let n = lower_with(strength_noise, 4.0, base.get(Attr::LowerBody).to_bits() as u32);
    ceilings.set(Attr::LowerBody, clamp(base.get(Attr::LowerBody) + n, MIN_RATING, RATING_BOUND));
    // His best spot is his best spot: where another position would rate him
    // higher, the skills only that position uses are trimmed until it doesn't.
    let best = overall_for(natural, &ceilings) * 0.95;
    for _ in 0..3 {
        for q in PLACED {
            let excess = overall_for(q, &ceilings) - best;
            if q == natural || excess <= 0.0 {
                continue;
            }
            // Only skills: the body is his own, and elusiveness is rated for
            // his size (update 3, A2), so a Sewell keeps his moves.
            let own: Vec<(Attr, f64)> = Attr::ALL
                .iter()
                .filter(|&&a| {
                    weight(natural, a) == 0.0
                        && weight(q, a) > 0.0
                        && a != Attr::Elusiveness
                        && ATTRIBUTE_PROFILES[a.index()].group != crate::attributes::AttributeGroup::Physical
                })
                .map(|&a| (a, weight(q, a)))
                .collect();
            let total: f64 = own.iter().map(|(_, w)| w).sum();
            if total <= 0.0 {
                continue;
            }
            for (a, _) in own {
                ceilings.set(a, (ceilings.get(a) - excess / total).max(MIN_RATING));
            }
        }
    }
    let (height, weight) = fitted_size(natural, fh, fw, talent);
    let body = Body::grown(natural, height, weight, rng);
    if natural == Position::EDGE {
        edge_lean(&mut ceilings, body.height);
    }
    thin_tops(&mut ceilings);
    (ceilings, body, natural, compete)
}

/// Where a rating or ceiling starts to thin out, and how fast it thins
/// past it (physical study items 1, 17 and 18, Cain 2026-11-15: no 99
/// wall, a thin top). With the wall gone the draw and the growth ran to
/// 130-150 (`top_check`, eight classes). Past the knee each point of draw
/// counts for less, on a log curve with no top, so a freak still stands
/// out but 100+ is about as rare as a 4.25 forty: a drafted man's key skill
/// past 100 about 0.1 to 0.7 a draft (real 4.25 forties 0.19). Strength and
/// lower body keep a fat top (item 3). Proposed, not confirmed.
pub const TOP_KNEE: f64 = 95.0;
pub const TOP_ROOM: f64 = 3.0;

/// A value past the knee, thinned (`TOP_KNEE`).
pub fn thin_top(v: f64) -> f64 {
    let (k, r) = (TOP_KNEE, TOP_ROOM);
    if v <= k { v } else { k + r * (1.0 + (v - k) / r).ln() }
}

/// The draw a thinned value stands for (`thin_top` undone).
pub fn raw_top(v: f64) -> f64 {
    let (k, r) = (TOP_KNEE, TOP_ROOM);
    if v <= k { v } else { k + r * (((v - k) / r).exp() - 1.0) }
}

/// A rating or ceiling that moved from `start` to `end`: a rise past the
/// knee is taken on the draw's scale and thinned back, so it counts for
/// less the nearer he is to the top, as at creation (physical study items 1
/// and 17). A fall, or a rise that stays under the knee, is as it was.
pub fn top_up(start: f64, end: f64) -> f64 {
    if end <= start || end <= TOP_KNEE {
        return end;
    }
    thin_top(raw_top(start) + (end - start))
}

/// `top_up` for one attribute: the strength pair keeps its fat top (item 3)
/// and moves as it was.
pub fn top_up_attr(a: Attr, start: f64, end: f64) -> f64 {
    if matches!(a, Attr::Strength | Attr::LowerBody) { end } else { top_up(start, end) }
}

/// Raises (or lowers) one ceiling by `d` through `top_up_attr`.
pub fn move_ceiling(c: &mut Attributes, a: Attr, d: f64) {
    let was = c.get(a);
    c.set(a, top_up_attr(a, was, was + d));
}

/// Every ceiling but the strength pair through `thin_top`.
pub fn thin_tops(c: &mut Attributes) {
    for a in Attr::ALL {
        if !matches!(a, Attr::Strength | Attr::LowerBody) {
            c.set(a, thin_top(c.get(a)));
        }
    }
}

/// Bug 330: a tall edge's skills stop trading rush for coverage and
/// recognition. At the same overall the generator gave 6-5½ edges about 1.2
/// points an inch more Zone Coverage, Play Recognition, Pursuit and
/// Anticipation and 0.6 to 1.1 less Hand Use, Finesse Rush and Power Rush
/// than 6-2 ones, so tall edges rushed worse as recruited (-0.46 pts of
/// pressures a snap an inch, real raw +0.02); long arms the same through
/// their height. `EDGE_TALL_LEAN` points an inch over the combine's height
/// move back, the edge overall kept: as recruited, height now costs -0.16
/// pts of pressures a snap an inch and arms -0.06 (4,000 games a test, with
/// the edge lane's rush; main -0.46 and -0.26, real raw flat). The spread is untouched, so rare bodies with rare skills still
/// come out (proposed, not confirmed).
const EDGE_TALL_LEAN: f64 = 1.4;

fn edge_lean(c: &mut Attributes, height: f64) {
    let (hm, _, _, _) = combine_size(Position::EDGE);
    let x = EDGE_TALL_LEAN * (height - hm);
    if x == 0.0 {
        return;
    }
    let from =
        [(Attr::ZoneCoverage, 1.0), (Attr::PlayRecognition, 1.0), (Attr::Pursuit, 1.0), (Attr::Anticipation, 0.9)];
    let to = [(Attr::HandUse, 1.0), (Attr::FinesseRush, 0.85), (Attr::PowerRush, 0.55)];
    let wf: f64 = from.iter().map(|&(a, f)| f * weight(Position::EDGE, a)).sum();
    let wt: f64 = to.iter().map(|&(a, f)| f * weight(Position::EDGE, a)).sum();
    for (a, f) in from {
        c.set(a, clamp(c.get(a) - f * x, MIN_RATING, RATING_BOUND));
    }
    for (a, f) in to {
        c.set(a, clamp(c.get(a) + f * x * wf / wt, MIN_RATING, RATING_BOUND));
    }
}

/// A linebacker plays near the combine's weight (bug 310): a frame far off
/// it carries less of the gap, so a heavy, slow linebacker isn't a
/// defensive end's body.
fn lb_weight(natural: Position, w: f64) -> f64 {
    if natural != Position::LB {
        return w;
    }
    let wm = combine_size(Position::LB).2;
    wm + LB_WEIGHT_SPREAD * (w - wm)
}

/// The share of a linebacker frame's distance from the combine's weight he
/// keeps: half brings the generator's spread from 20 lb to 9 (real 10), its
/// heavy men (over 245) from 259 to 250 lb (real 253) and its slow ones
/// (over 4.82) from 253 to 242 (real 245). Proposed, not confirmed.
const LB_WEIGHT_SPREAD: f64 = 0.5;

/// Rating points moved from range to hitting per ten pounds over the
/// combine's weight: safeties 2, linebackers 3, and 1.5 more per sd of a
/// linebacker's speed roll under the mean (D, L3). Proposed, not confirmed:
/// sized so the generator's heavy and slow men match the real raw slopes on
/// run tackles, stops and catch rate allowed.
const LEAN_S: f64 = 2.0;
const LEAN_LB: f64 = 3.0;
const LEAN_LB_SLOW: f64 = 1.5;

/// The body-to-leaning tie at safety and linebacker (D, L3): rating points
/// moved from range to hitting per ten pounds over the combine's weight at
/// his position (and per sd of speed roll under the mean at linebacker),
/// with his overall there kept at his talent.
fn body_lean(base: &mut Attributes, natural: Position, talent: f64, fw: f64, roll: f64, speed_share: f64) {
    let (hit, range): (&[Attr], &[Attr]) = match natural {
        Position::S => (
            &[Attr::Tackling, Attr::Strength, Attr::LowerBody, Attr::Physicality],
            &[Attr::ZoneCoverage, Attr::ManCoverage, Attr::Pursuit],
        ),
        Position::LB => (
            &[Attr::Tackling, Attr::BlockShedding, Attr::Strength, Attr::LowerBody],
            &[Attr::ZoneCoverage, Attr::ManCoverage, Attr::Pursuit],
        ),
        _ => return,
    };
    let w = fitted_size(natural, 0.0, fw, talent).1;
    let z = (w - combine_size(natural).2) / 10.0;
    let x = match natural {
        Position::S => LEAN_S * z,
        _ => LEAN_LB * z - LEAN_LB_SLOW * roll,
    };
    let x = clamp(x, -12.0, 12.0);
    for &a in hit {
        base.add(a, x);
    }
    for &a in range {
        base.add(a, -x);
    }
    if speed_share < 1.0 {
        let short = (talent - overall_for(natural, base)) / (1.0 - speed_share);
        for a in Attr::ALL {
            if weight(natural, a) > 0.0 && !matches!(a, Attr::Speed | Attr::Acceleration) {
                base.add(a, short);
            }
        }
    }
}

/// How sharp a staff's eye for a player's real position is, 0-1: a high
/// school coach with little time sees less than a college staff, and NFL
/// scouts see the most.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Eye {
    /// Share of the read that comes from where he is headed rather than what he is now.
    pub insight: f64,
    /// Misreads in rating points, attribute by attribute.
    pub noise: f64,
    /// Which staff is looking: each staff misreads a player its own way.
    pub staff: u32,
}

impl Eye {
    /// A coach's eye from his 1-20 skill (teaching and scheme).
    pub fn coach(skill: f64) -> Eye {
        let s = clamp(skill / 20.0, 0.0, 1.0);
        Eye { insight: 0.25 + 0.5 * s, noise: 7.0 - 5.0 * s, staff: 0 }
    }
    /// The same eye belonging to a particular staff.
    pub fn staff(self, staff: u32) -> Eye {
        Eye { staff, ..self }
    }
    pub const NFL_SCOUTS: Eye = Eye { insight: 0.65, noise: 2.5, staff: 0x4E46_4C00 };
}

/// What a staff thinks each attribute will become. Staffs read where a player
/// is headed with errors that depend on their eye and on how much film he
/// has, and an early bloomer's body fools the less sharp ones: at 15 a boy
/// who matured early looks faster and stronger than he will be next to his
/// peers, and a late bloomer looks smaller than he is.
fn perceived(p: &Player, eye: Eye) -> Attributes {
    // A staff's misread of a player sticks: the same staff sees him the same way next year.
    let mut misread = Rng::new(p.id.wrapping_mul(0x9E37_79B9) ^ eye.staff);
    let age = p.age as f64;
    // Older players have more film: every staff reads them better.
    let noise = eye.noise * clamp(1.0 - 0.1 * (age - 14.0), 0.5, 1.0);
    let bloom_shows = (1.0 - eye.insight) * clamp((19.0 - age) / 5.0, 0.0, 1.0);
    let mut out = Attributes::default();
    for a in Attr::ALL {
        let physical = ATTRIBUTE_PROFILES[a.index()].group == crate::attributes::AttributeGroup::Physical;
        let illusion = if physical { -3.0 * p.bloom * bloom_shows } else { 0.0 };
        out.set(a, p.ceilings.get(a) + illusion + misread.normal(0.0, noise));
    }
    out
}

/// How badly a frame misfits a position, as a share of his rating there:
/// nobody puts a 5'9" boy at tackle. Weight can move about 25 pounds with a
/// weight program, height can't. Measured on the frame he'd have as a draft
/// prospect, against the real combine. Coaches start to mind height half a
/// spread from the position's average (update 3, A18; was a whole spread,
/// which left backs and safeties tall and interior linemen short).
fn misfit(height: f64, weight: f64, pos: Position) -> f64 {
    let (hm, hs, wm, ws) = combine_size(pos);
    let h = (height - hm) / hs;
    let w = ((weight - wm).abs() - 25.0).max(0.0) / ws;
    let over = |z: f64, band: f64| (z.abs() - band).max(0.0);
    clamp(0.04 * over(h, 0.5).powi(2) + 0.04 * over(w, 1.0).powi(2), 0.0, 0.5)
}

fn player_misfit(p: &Player, pos: Position) -> f64 {
    let (dh, dw) = talent_size(p.original_potential);
    misfit(p.body.height - dh, p.body.frame_weight - dw, pos)
}

/// How well he fits each position as a staff sees him, in rating points.
pub fn fits(p: &Player, eye: Eye) -> [f64; POSITION_COUNT] {
    let seen = perceived(p, eye);
    let mut out = [0.0; POSITION_COUNT];
    for pos in Position::ALL {
        out[pos.index()] = overall_for(pos, &seen) * (1.0 - player_misfit(p, pos));
    }
    out
}

/// Moves a player to a position: his weight follows the position's (a
/// receiver turned tight end eats, a tight end turned tackle eats more) within
/// what his frame allows, and his card records the change.
pub fn set_position(p: &mut Player, pos: Position) {
    // Already there by his card and where he plays. A man whose card says
    // one spot and who plays another is still moved (realism audit item 3:
    // an athlete from another sport was put at his sport's spot without his
    // card, so the move back to where his body fits never took and he
    // played out of position for life; a save's men like him move at their
    // staff's next look).
    let listed = p.position_history.last().map(|h| h.1) == Some(pos);
    if listed && p.position == pos {
        return;
    }
    // Rust from poor prep took his old position's technique: it comes back
    // before he moves.
    crate::training::prep::unrust(p);
    p.position = pos;
    if !listed {
        p.position_history.reserve_exact(1);
        p.position_history.push((p.age, pos));
    }
    // His spot at the new position: where he suits best (a staff sets the
    // whole room's spots each spring).
    let s = crate::spots::spots_of(pos)
        .iter()
        .copied()
        .filter(|&s| s != crate::spots::Spot::FB)
        .max_by(|&a, &b| {
            let r = |s| crate::spots::spot_rating(s, &p.attributes, p.body.weight, p.body.height);
            r(a).total_cmp(&r(b))
        })
        .unwrap_or(crate::spots::spots_of(pos)[0]);
    p.spot = s;
    p.spots_played |= s.bit();
    let target = position_weight(p, pos);
    let frame = p.body.frame_weight;
    p.body.weight = if pos == Position::LS {
        // A long snapper plays at a snapper's weight, cutting or adding to
        // it (real FBS and NFL snappers: about 6'2", 235-250 lb).
        clamp(frame, LS_MIN_WEIGHT, LS_MAX_WEIGHT)
    } else if pos.is_specialist() {
        // Kickers and punters weigh what real ones do (update 3, A19: NFL
        // kickers 194 lb, punters 213, where the frame alone made them 224
        // and 225): the combine's spread at his spot, keeping how big his
        // frame runs among lean builds.
        let (_, _, wm, ws) = combine_size(pos);
        clamp(wm + ws * (frame - 220.0) / 26.0, wm - 2.5 * ws, wm + 2.5 * ws)
    } else {
        placed_weight(frame, target)
    };
    // The new spot's technique he never learned (individual practice).
    crate::craft::moved(p);
}

/// A position's playing weight for him: the combine's average there, less
/// what lower talent takes off a body.
pub(crate) fn position_weight(p: &Player, pos: Position) -> f64 {
    position_weight_for(p.original_potential, pos)
}

/// `position_weight` for a talent (a ceiling read stands in for it on a
/// projection).
pub fn position_weight_for(talent: f64, pos: Position) -> f64 {
    let (_, _, wm, _) = combine_size(pos);
    wm + talent_size(talent).1
}

/// Where a position's weight program puts a frame when he moves there:
/// 60% of the way to the position's weight, at most 25 lb off or 30 lb on.
pub fn placed_weight(frame: f64, target: f64) -> f64 {
    frame + clamp(0.6 * (target - frame), -25.0, 30.0)
}

/// The weights a long snapper plays at (real FBS and NFL rosters: 220-265
/// lb, most 235-250).
pub const LS_MIN_WEIGHT: f64 = 220.0;
pub const LS_MAX_WEIGHT: f64 = 265.0;

/// Within a family of spots, staffs put the better athlete at the harder
/// one: the best lineman plays tackle, the best pass rusher the edge, the
/// best cover man corner. As a share of a player's fit.
fn premium(pos: Position) -> f64 {
    match pos {
        Position::OT | Position::EDGE => 0.015,
        Position::CB => 0.01,
        _ => 0.0,
    }
}

/// Real shares of high school players by position: what a roster needs.
fn roster_need(pos: Position) -> f64 {
    pos.profile().share
}

/// A head coach places one incoming class on his roster: his best athletes
/// first, each where he fits best, weighed against what the roster still needs
/// and what each spot is worth.
pub fn place_class(players: &mut [&mut Player], eye: Eye) {
    if players.is_empty() {
        return;
    }
    let n = players.len() as f64;
    let total_share: f64 = Position::ALL.iter().map(|&p| roster_need(p)).sum();
    let target: Vec<f64> = Position::ALL.iter().map(|&p| roster_need(p) / total_share * n).collect();
    // Each read is keyed by the player and the staff: on all cores, in
    // order (world speed lane).
    use rayon::prelude::*;
    let fit: Vec<[f64; POSITION_COUNT]> = players.par_iter().with_min_len(64).map(|p| fits(p, eye)).collect();
    let best: Vec<f64> = fit.iter().map(|f| PLACED.iter().map(|q| f[q.index()]).fold(f64::MIN, f64::max)).collect();
    let mut order: Vec<usize> = (0..players.len()).collect();
    order.sort_by(|&a, &b| best[b].total_cmp(&best[a]));
    let mut filled = [0.0; POSITION_COUNT];
    for i in order {
        let scale = best[i].abs().max(1.0);
        // Kickers and punters are found, not converted, here as in
        // `better_position`: only a boy built as one (his natural spot, set
        // when he is made) is listed there. A back or receiver with a leg
        // plays his own spot and punts on the side when his team has no
        // punter (the game gives the ball to the best leg). Coaches had been
        // filling open kicking spots with their best athletes, and colleges
        // then recruited them as specialists: in a half-size seed 7 world 33
        // of 39 Black college kickers and punters had been built as backs,
        // receivers, linebackers or linemen (minor bug 165).
        let leg = players[i].position.kicks();
        let pick = PLACED
            .iter()
            .copied()
            .filter(|q| leg || !q.kicks())
            .max_by(|&a, &b| {
                let score = |pos: Position| {
                    let k = pos.index();
                    let open = target[k] - filled[k];
                    // An open spot pulls a little, by how open it is; an
                    // overfilled one pushes harder. A class is a quarter
                    // of a roster, so a spot's share of 18 boys is a
                    // fraction (tackles 1.5): with a flat pull to half a
                    // boy open and no push until half a boy over, each
                    // spot rounded to whole boys, which sent tackles to
                    // two a class (11-12% of freshmen against a real 8.5%)
                    // and left guards and centers at two of 2.2 (proposed,
                    // not confirmed).
                    let need = if open > 0.0 { 0.08 * open.min(1.0) } else { 0.12 * open };
                    fit[i][k] / scale + need + premium(pos)
                };
                score(a).total_cmp(&score(b))
            })
            .expect("positions");
        filled[pick.index()] += 1.0;
        // His first real position: where he'd suit by nature was never on a depth chart.
        players[i].position_history.clear();
        set_position(players[i], pick);
    }
}

/// A staff takes a fresh look at where a player fits and moves him when
/// another spot is clearly better: growth, a new staff or the pros' needs.
/// `margin` is how much better the new spot must look, as a share of his
/// current fit (switching costs a staff something, so they need to be sure).
/// Returns the position he moved from.
pub fn reconsider(p: &mut Player, eye: Eye, margin: f64) -> Option<Position> {
    let to = better_position(p, eye, margin)?;
    let from = p.position;
    set_position(p, to);
    Some(from)
}

/// Where a staff would move a player on a fresh look, without moving him
/// (see `reconsider`).
pub fn better_position(p: &Player, eye: Eye, margin: f64) -> Option<Position> {
    // Kickers and snappers are found, not converted: nobody moves a regular there late.
    if p.position.is_specialist() {
        return None;
    }
    let raw = fits(p, eye);
    let scale = PLACED.iter().map(|q| raw[q.index()]).fold(1.0, f64::max);
    // Spots are weighed the way the head coach placed him: fit, plus what the spot is worth.
    let fit = Position::ALL.map(|q| raw[q.index()] / scale + premium(q));
    let now = fit[p.position.index()];
    let (pos, best) =
        PLACED.iter().map(|&q| (q, fit[q.index()])).max_by(|a, b| a.1.total_cmp(&b.1)).expect("positions");
    // Kickers are found, not converted: nobody moves a regular to kicker late.
    if pos == p.position || pos.is_specialist() || p.position.is_specialist() {
        return None;
    }
    // A staff almost never moves a man back where he came from (fix list
    // 3.9): it takes three times the usual gap (proposed, not confirmed).
    let back = p.position_history.iter().any(|&(_, q)| q == pos);
    if best > now + if back { 3.0 * margin } else { margin } { Some(pos) } else { None }
}
