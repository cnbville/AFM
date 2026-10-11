//! A level rating says how good a player is against everyone he could line up
//! with or against, on a 1-99 scale. The pro game uses the absolute rating
//! itself; high school and college get this translation so that a 60-rated
//! junior reads as the force he is against high school opponents.
//!
//! - High school: compared with every high schooler of the same age, like a
//!   national recruiting ranking.
//! - College: compared with every FBS player of any age, like EA Sports College
//!   Football. FCS and small-college players are put on the same FBS scale
//!   after the same discount NFL scouts give their tape.
//!
//! Positions mature at different ages: at 17 the best quarterbacks and
//! linemen rate lower than the best running backs and corners, though they are
//! just as far ahead of their peers. So a player is first placed against his
//! own position (see `PositionAligner`) and then ranked nationally, the way EA
//! Sports College Football's top 100 spans every position.
//!
//! The scale is anchored to national ranks, using how rare top ratings are in
//! EA College Football (about 11,000 players a season): 90+ is roughly the top
//! 100 (an All-American; for high schoolers, an All-American Bowl player), 95+
//! the top 8, and only a player about 5 points clear of the No. 2 reaches 99.
//! EA rated nobody 99 at launch in College Football 25 or 26, and one player in 27.

use crate::player::Level;
use crate::positions::{POSITION_COUNT, Position};
use crate::rng::round_half_up;

#[derive(Clone, Debug, Default)]
pub struct LevelScale {
    /// (overall, level rating) points, ascending in overall.
    pub points: Vec<(f64, f64)>,
}

/// Rating points per overall point above the No. 2 player in the pool.
const ABOVE_TOP_SLOPE: f64 = 0.4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pool {
    College,
    HS,
}

struct Anchors {
    /// Quantile from the bottom, and the rating there.
    quantile: [(f64, f64); 2],
    /// Rank from the top in a reference-sized pool, and the rating there.
    rank: [(f64, f64); 4],
}

fn anchors(pool: Pool) -> Anchors {
    match pool {
        Pool::College => Anchors {
            quantile: [(0.01, 55.0), (0.5, 72.0)],
            rank: [(700.0, 85.0), (100.0, 90.0), (8.0, 95.0), (2.0, 96.5)],
        },
        Pool::HS => Anchors {
            quantile: [(0.01, 5.0), (0.5, 50.0)],
            rank: [(2000.0, 80.0), (100.0, 90.0), (8.0, 95.0), (2.0, 96.5)],
        },
    }
}

/// Group a season is rated in: high schoolers by age, every college player together, pros not at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LevelGroup {
    HS(u32),
    College,
}

pub fn level_group(level: Level, age: u32) -> Option<LevelGroup> {
    match level {
        Level::NFL => None,
        Level::HS => Some(LevelGroup::HS(age)),
        _ => Some(LevelGroup::College),
    }
}

/// Whether a season sets the scale for its group (only FBS seasons set the college scale).
pub fn sets_level_scale(level: Level) -> bool {
    matches!(level, Level::HS | Level::FBS)
}

/// Build the scale for one group from the overalls that set it. The top ranks
/// are national counts (the top 100 is the top 100 however big the pool is),
/// so they are only scaled when the simulated class is smaller than a real one.
pub fn build_level_scale(overalls: &[f64], pool: Pool, class_share: f64) -> LevelScale {
    let mut s = overalls.to_vec();
    s.sort_unstable_by(|a, b| b.total_cmp(a));
    let n = s.len();
    if n == 0 {
        return LevelScale::default();
    }
    let a = anchors(pool);
    let mut points: Vec<(f64, f64)> = Vec::with_capacity(6);
    for (q, rating) in a.quantile {
        points.push((s[(n - 1).min((n as f64 * (1.0 - q)).floor() as usize)], rating));
    }
    for (rank, rating) in a.rank {
        let k = (round_half_up(rank * class_share).max(1.0) as usize) - 1;
        points.push((s[(n - 1).min(k)], rating));
    }
    points.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    // Keep the curve strictly increasing in overall: in a tiny pool two anchors can share a value.
    let mut clean: Vec<(f64, f64)> = Vec::with_capacity(points.len());
    for p in points {
        match clean.last_mut() {
            Some(last) if p.0 <= last.0 => last.1 = last.1.max(p.1),
            _ => clean.push(p),
        }
    }
    LevelScale { points: clean }
}

/// Maps an overall at one position onto the all-position scale of the same
/// group, so that the same standing among his position's peers gives the same
/// value: each position's median and 99th percentile line up with the pool's.
/// Positions with too few players to measure are left as they are.
#[derive(Clone, Debug)]
pub struct PositionAligner {
    maps: [Option<(f64, f64)>; POSITION_COUNT],
    all_mid: f64,
}

impl PositionAligner {
    pub fn new(samples: &[(Position, f64)]) -> Self {
        let all: Vec<f64> = samples.iter().map(|s| s.1).collect();
        let mut by_position: [Vec<f64>; POSITION_COUNT] = Default::default();
        for &(pos, x) in samples {
            by_position[pos.index()].push(x);
        }
        let (all_mid, all_high) = quantiles(all);
        let mut maps = [None; POSITION_COUNT];
        for (i, xs) in by_position.into_iter().enumerate() {
            if xs.len() < 30 {
                continue;
            }
            let (mid, high) = quantiles(xs);
            if high - mid < 0.5 {
                continue;
            }
            maps[i] = Some((mid, (all_high - all_mid) / (high - mid)));
        }
        PositionAligner { maps, all_mid }
    }

    #[inline]
    pub fn align(&self, position: Position, overall: f64) -> f64 {
        match self.maps[position.index()] {
            Some((mid, slope)) => self.all_mid + (overall - mid) * slope,
            None => overall,
        }
    }
}

/// Median and 99th percentile.
fn quantiles(mut xs: Vec<f64>) -> (f64, f64) {
    if xs.is_empty() {
        return (0.0, 0.0);
    }
    xs.sort_unstable_by(f64::total_cmp);
    let at = |q: f64| xs[(xs.len() - 1).min((q * (xs.len() - 1) as f64).floor() as usize)];
    (at(0.5), at(0.99))
}

pub fn level_rating(overall: f64, level: Level, scale: Option<&LevelScale>) -> u8 {
    let scale = match scale {
        Some(s) if level != Level::NFL && !s.points.is_empty() => s,
        _ => return clamp_rating(overall),
    };
    let pts = &scale.points;
    let top = pts[pts.len() - 1];
    if overall >= top.0 {
        return clamp_rating(top.1 + ABOVE_TOP_SLOPE * (overall - top.0));
    }
    let bottom = pts[0];
    if overall <= bottom.0 {
        let slope = pts.get(1).map_or(1.0, |next| (next.1 - bottom.1) / (next.0 - bottom.0));
        return clamp_rating(bottom.1 - slope * (bottom.0 - overall));
    }
    for i in 1..pts.len() {
        let (x1, y1) = pts[i];
        if overall <= x1 {
            let (x0, y0) = pts[i - 1];
            return clamp_rating(y0 + ((overall - x0) / (x1 - x0)) * (y1 - y0));
        }
    }
    clamp_rating(top.1)
}

fn clamp_rating(x: f64) -> u8 {
    round_half_up(x).clamp(1.0, 99.0) as u8
}
