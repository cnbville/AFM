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

use crate::attributes::{ATTRIBUTE_PROFILES, Attr, Attributes, MAX_RATING, MIN_RATING};
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

const PHYSICAL: [Attr; 6] =
    [Attr::Speed, Attr::Acceleration, Attr::Agility, Attr::Strength, Attr::Jumping, Attr::Stamina];

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
/// rest are lean, with height and weight tied together.
fn draw_frame(rng: &mut Rng) -> (f64, f64) {
    let h = rng.normal(0.0, 1.0);
    if rng.chance(0.29) {
        (75.9 + 1.5 * h, 311.0 + 4.0 * h + rng.normal(0.0, 15.0))
    } else {
        (73.1 + 2.1 * h, 220.0 + 26.0 * (0.55 * h + 0.83 * rng.normal(0.0, 1.0)))
    }
}

/// The athlete's shape: what each attribute would be if he became an elite
/// player, before talent scales it. Returns the shape and the frame.
fn draw_shape(rng: &mut Rng) -> (Attributes, (f64, f64)) {
    let (fh, fw) = draw_frame(rng);
    let mut shape = Attributes::filled(45.0);

    // Athletic profile from the body: blend what bodies like his are like at
    // the combine, then add his own explosiveness, hips and strength.
    let likeness: Vec<(Position, f64)> =
        FIELD.iter().map(|&p| (p, body_likeness(fh, fw, p) * p.profile().share + 1e-9)).collect();
    let total: f64 = likeness.iter().map(|(_, l)| l).sum();
    for a in PHYSICAL {
        shape[a] = likeness.iter().map(|&(p, l)| l * elite_shape_value(p, a)).sum::<f64>() / total;
    }
    let explosive = rng.normal(0.0, 1.0);
    shape[Attr::Speed] += 4.0 * explosive + rng.normal(0.0, 3.0);
    shape[Attr::Acceleration] += 4.0 * explosive + rng.normal(0.0, 3.0);
    shape[Attr::Jumping] += 4.0 * explosive + rng.normal(0.0, 3.0);
    shape[Attr::Agility] += 2.5 * explosive + rng.normal(0.0, 3.5);
    shape[Attr::Strength] += rng.normal(0.0, 5.0);
    shape[Attr::Stamina] += rng.normal(0.0, 5.0);

    // Skill leanings, independent of the body: a gift that shows in several
    // attributes gets a shared draw, mixed with each attribute's own.
    let mut z = || rng.normal(0.0, 1.0);
    let (arm, touch, hands, route, ball, elude) = (z(), z(), z(), z(), z(), z());
    let (block, feet, tough, tackle, shed, rush) = (z(), z(), z(), z(), z(), z());
    let (cover, mirror, leg, kick_touch, iq, recog) = (z(), z(), z(), z(), z(), z());
    let (aware, vision, poise) = (z(), z(), z());
    let mix = |a: f64, x: f64, b: f64, y: f64| (a * x + b * y) / (a * a + b * b).sqrt();
    shape[Attr::ThrowPower] = leaning(arm);
    shape[Attr::ThrowAccuracy] = leaning(mix(0.5, arm, 0.85, touch));
    shape[Attr::Catching] = leaning(hands);
    shape[Attr::RouteRunning] = leaning(mix(0.5, hands, 0.85, route));
    shape[Attr::Carrying] = leaning(mix(0.6, ball, 0.8, tough));
    shape[Attr::Elusiveness] = leaning(mix(0.85, elude, 0.4, explosive));
    shape[Attr::RunBlocking] = leaning(mix(0.65, block, 0.6, tough));
    shape[Attr::PassBlocking] = leaning(mix(0.75, block, 0.55, feet));
    shape[Attr::Tackling] = leaning(mix(0.7, tough, 0.7, tackle));
    shape[Attr::BlockShedding] = leaning(mix(0.6, tough, 0.8, shed));
    shape[Attr::PassRush] = leaning(mix(0.8, rush, 0.4, shed));
    shape[Attr::ManCoverage] = leaning(mix(0.8, cover, 0.6, mirror));
    shape[Attr::ZoneCoverage] = leaning(mix(0.7, cover, 0.7, recog));
    // A real kicking leg is rarer than any other gift.
    shape[Attr::KickPower] = leaning(leg - 0.4);
    shape[Attr::KickAccuracy] = leaning(mix(0.5, leg, 0.85, kick_touch) - 0.4);
    shape[Attr::Awareness] = leaning(mix(0.75, iq, 0.65, aware));
    shape[Attr::PlayRecognition] = leaning(mix(0.75, iq, 0.65, recog));
    shape[Attr::Vision] = leaning(vision);
    shape[Attr::Poise] = leaning(poise);
    (shape, (fh, fw))
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
            let (shape, (h, w)) = draw_shape(&mut rng);
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
        for pos in Position::ALL {
            let k = pos.index();
            let target = roster_need(pos) / total;
            let actual = (count[k] / sample.len() as f64).max(1e-4);
            scale[k] *= clamp(0.2 * (target / actual).ln(), -0.004, 0.004).exp();
        }
    }
    scale
});

fn argmax_scaled(row: &[f64; POSITION_COUNT], scale: &[f64; POSITION_COUNT]) -> usize {
    (0..POSITION_COUNT).max_by(|&a, &b| (row[a] * scale[a]).total_cmp(&(row[b] * scale[b]))).expect("positions")
}

/// The position a shape suits best, and his overall there.
fn best_fit(shape: &Attributes, frame: (f64, f64)) -> (Position, f64) {
    let row = Position::ALL.map(|p| overall_for(p, shape) * (1.0 - misfit(frame.0, frame.1, p)));
    let k = argmax_scaled(&row, &FIT_SCALE);
    (Position::ALL[k], overall_for(Position::ALL[k], shape))
}

/// Ceilings, his body and his natural position for a new athlete of the
/// given talent. Talent is how good he can become at the spot that suits him best.
pub(crate) fn create(rng: &mut Rng, talent: f64) -> (Attributes, Body, Position) {
    let (shape, (fh, fw)) = draw_shape(rng);
    let (natural, fit) = best_fit(&shape, (fh, fw));
    // Talent is his overall at his best spot. The skills that spot needs rise
    // together: a great player is good at everything his position asks, so
    // the higher his talent, the less room for uneven gifts (and none above 99).
    // Everything else scales with talent the way his leanings say.
    let scale = talent / fit;
    let top_key = Attr::ALL.iter().filter(|&&a| weight(natural, a) > 0.0).map(|&a| shape[a]).fold(fit + 1.0, f64::max);
    let spread = (0.5 * scale).min((94.0 - talent).max(0.5) / (top_key - fit));
    let mut ceilings = Attributes::default();
    for a in Attr::ALL {
        let base = if weight(natural, a) > 0.0 { talent + (shape[a] - fit) * spread } else { shape[a] * scale };
        ceilings[a] = clamp(base + rng.normal(0.0, 4.0), MIN_RATING, MAX_RATING);
    }
    // His best spot is his best spot: where another position would rate him
    // higher, the skills only that position uses are trimmed until it doesn't.
    let best = overall_for(natural, &ceilings) * 0.95;
    for _ in 0..3 {
        for q in Position::ALL {
            let excess = overall_for(q, &ceilings) - best;
            if q == natural || excess <= 0.0 {
                continue;
            }
            let own: Vec<(Attr, f64)> = Attr::ALL
                .iter()
                .filter(|&&a| weight(natural, a) == 0.0 && weight(q, a) > 0.0)
                .map(|&a| (a, weight(q, a)))
                .collect();
            let total: f64 = own.iter().map(|(_, w)| w).sum();
            if total <= 0.0 {
                continue;
            }
            for (a, _) in own {
                ceilings[a] = (ceilings[a] - excess / total).max(MIN_RATING);
            }
        }
    }
    let (dh, dw) = talent_size(talent);
    let height = clamp(fh + dh, 64.0, 82.0);
    let weight = clamp(fw + dw, 140.0, 380.0);
    let body = Body::grown(height, weight, rng);
    (ceilings, body, natural)
}

/// How sharp a staff's eye for a player's real position is, 0-1: a high
/// school coach with little time sees less than a college staff, and NFL
/// scouts see the most.
#[derive(Clone, Copy, Debug)]
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
        out[a] = p.ceilings[a] + illusion + misread.normal(0.0, noise);
    }
    out
}

/// How badly a frame misfits a position, as a share of his rating there:
/// nobody puts a 5'9" boy at tackle. Weight can move about 25 pounds with a
/// weight program, height can't. Measured on the frame he'd have as a draft
/// prospect, against the real combine.
fn misfit(height: f64, weight: f64, pos: Position) -> f64 {
    let (hm, hs, wm, ws) = combine_size(pos);
    let h = (height - hm) / hs;
    let w = ((weight - wm).abs() - 25.0).max(0.0) / ws;
    let over = |z: f64| (z.abs() - 1.0).max(0.0);
    clamp(0.04 * over(h).powi(2) + 0.04 * over(w).powi(2), 0.0, 0.5)
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
    if p.position_history.last().map(|h| h.1) == Some(pos) {
        return;
    }
    p.position = pos;
    p.position_history.push((p.age, pos));
    let (_, _, wm, _) = combine_size(pos);
    let (_, dw) = talent_size(p.original_potential);
    let target = wm + dw;
    let frame = p.body.frame_weight;
    p.body.weight = if pos.is_specialist() { frame } else { frame + clamp(0.6 * (target - frame), -25.0, 30.0) };
}

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
    let fit: Vec<[f64; POSITION_COUNT]> = players.iter().map(|p| fits(p, eye)).collect();
    let best: Vec<f64> = fit.iter().map(|f| f.iter().copied().fold(f64::MIN, f64::max)).collect();
    let mut order: Vec<usize> = (0..players.len()).collect();
    order.sort_by(|&a, &b| best[b].total_cmp(&best[a]));
    let mut filled = [0.0; POSITION_COUNT];
    for i in order {
        let scale = best[i].abs().max(1.0);
        let pick = Position::ALL
            .iter()
            .copied()
            .max_by(|&a, &b| {
                let score = |pos: Position| {
                    let k = pos.index();
                    let open = target[k] - filled[k];
                    // An open spot pulls a little; an overfilled one pushes harder.
                    let need = if open >= 0.5 {
                        0.08
                    } else if open > -0.5 {
                        0.0
                    } else {
                        0.12 * open
                    };
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
    let raw = fits(p, eye);
    let scale = raw.iter().copied().fold(1.0, f64::max);
    // Spots are weighed the way the head coach placed him: fit, plus what the spot is worth.
    let fit = Position::ALL.map(|q| raw[q.index()] / scale + premium(q));
    let now = fit[p.position.index()];
    let (pos, best) =
        Position::ALL.iter().map(|&q| (q, fit[q.index()])).max_by(|a, b| a.1.total_cmp(&b.1)).expect("positions");
    // Kickers are found, not converted: nobody moves a regular to kicker late.
    if pos == p.position || pos.is_specialist() || p.position.is_specialist() {
        return None;
    }
    if best > now + margin {
        let from = p.position;
        set_position(p, pos);
        return Some(from);
    }
    None
}
