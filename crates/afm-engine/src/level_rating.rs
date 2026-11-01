//! A level rating says how good a player is against everyone he could line up
//! with or against, on a 1-99 scale. The pro game uses the absolute rating
//! itself; high school and college get this translation so that a 60-rated
//! junior reads as the force he is against high school opponents.
//!
//! - High school: compared with every high schooler of the same age, like a
//!   national recruiting ranking.
//! - College: compared with every FBS player of any age. FCS and
//!   small-college players are put on the same FBS scale after the same
//!   discount NFL scouts give their tape.
//!
//! Positions mature at different ages: at 17 the best quarterbacks and
//! linemen rate lower than the best running backs and corners, though they are
//! just as far ahead of their peers. So a player is first placed against his
//! own position (see `PositionAligner`) and then ranked nationally, the way an
//! All-America team or a national recruiting top 100 spans every position.
//!
//! The top of the scale follows real honours, not a video game (Cain,
//! 2026-10-24):
//! - College, per season: 95+ is about the consensus All-Americans (the NCAA
//!   counts about 26-30 a year), 90+ about AP's three All-America teams
//!   (about 80 players).
//! - High school, per class: 95+ is about the 5-stars (about 32 a class on
//!   the 247Sports Composite), 90+ about the All-American Bowl's roster
//!   (about 100 seniors).
//! - Only a player about 5 points clear of the No. 2 reaches 99.
//!
//! The honour anchors sit at 89.5 and 94.5, where a shown rating rounds up
//! to 90 and 95, so the counts are of the numbers a card shows.
//!
//! In the world the anchors are pooled over the last five seasons
//! (`SCALE_SEASONS`, `pooled_level_scale`), so a loaded year shows more
//! 90s and a thin one fewer, instead of every year being squeezed into the
//! same counts (Cain, 2026-10-24). The label never feeds growth.

use crate::player::Level;
use crate::positions::{POSITION_COUNT, Position};
use crate::rng::round_half_up;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct LevelScale {
    /// (overall, level rating) points, ascending in overall.
    pub points: Vec<(f64, f64)>,
}

/// Rating points per overall point above the No. 2 player in the pool.
const ABOVE_TOP_SLOPE: f64 = 0.4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
            rank: [(700.0, 85.0), (80.0, 89.5), (28.0, 94.5), (2.0, 96.5)],
        },
        Pool::HS => Anchors {
            quantile: [(0.01, 5.0), (0.5, 50.0)],
            rank: [(2000.0, 80.0), (100.0, 89.5), (32.0, 94.5), (2.0, 96.5)],
        },
    }
}

/// The anchors of a pool in words, bottom first, for the badge's hover
/// (minor bug 56): "Bottom 1%" 55, "The middle" 72, "No. 700" 85 and so on,
/// read from `anchors`, never typed twice. Each is the number a card shows
/// there: the honour anchors sit at 89.5 and 94.5 and show as 90 and 95.
pub fn marks(pool: Pool) -> Vec<(String, f64)> {
    let a = anchors(pool);
    let mut v: Vec<(String, f64)> = a
        .quantile
        .iter()
        .map(|&(q, r)| {
            (if q >= 0.5 { "The middle".to_string() } else { format!("Bottom {:.0}%", q * 100.0) }, round_half_up(r))
        })
        .collect();
    v.extend(a.rank.iter().map(|&(rank, r)| (format!("No. {}", thousands(rank as u64)), round_half_up(r))));
    v
}

fn thousands(n: u64) -> String {
    if n >= 1000 { format!("{},{:03}", n / 1000, n % 1000) } else { n.to_string() }
}

/// One group's anchors over its last `SCALE_SEASONS` seasons, oldest first,
/// at the start and at the end of each season.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ScaleHistory {
    pub group: LevelGroup,
    pub start: Vec<SeasonAnchors>,
    pub end: Vec<SeasonAnchors>,
    /// The newest season's end-of-year position aligner, so a staff's read
    /// can be placed on the same scale (minor bug 49). Empty on an older file.
    #[serde(default)]
    pub align: Option<PositionAligner>,
}

impl ScaleHistory {
    /// Add a season, dropping the oldest past `SCALE_SEASONS`.
    pub fn push(&mut self, start: SeasonAnchors, end: SeasonAnchors) {
        for (seasons, a) in [(&mut self.start, start), (&mut self.end, end)] {
            seasons.push(a);
            if seasons.len() > SCALE_SEASONS {
                seasons.remove(0);
            }
        }
    }
}

/// Group a season is rated in: high schoolers by age, every college player together, pros not at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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

/// Seasons a group's scale is pooled over in the world (Cain, 2026-10-24).
pub const SCALE_SEASONS: usize = 5;

/// One season's anchors for a group: (overall at the anchor, rating there),
/// always in the same order, empty when nobody set the scale.
pub type SeasonAnchors = Vec<(f64, f64)>;

/// Build the scale for one group from the overalls that set it. The top ranks
/// are national counts (the top 100 is the top 100 however big the pool is),
/// so they are only scaled when the simulated class is smaller than a real one.
pub fn build_level_scale(overalls: &[f64], pool: Pool, class_share: f64) -> LevelScale {
    scale_from(season_anchors(overalls, pool, class_share))
}

/// One season's anchors, before they are pooled or turned into a scale.
pub fn season_anchors(overalls: &[f64], pool: Pool, class_share: f64) -> SeasonAnchors {
    let mut s = overalls.to_vec();
    s.sort_unstable_by(|a, b| b.total_cmp(a));
    let n = s.len();
    if n == 0 {
        return Vec::new();
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
    points
}

/// The scale from several seasons' anchors: each anchor sits at the average
/// overall it had over those seasons, so the No. 100 is about the No. 100 of
/// a typical recent year and a strong year's best rate above the usual
/// counts. Seasons without anchors are left out.
pub fn pooled_level_scale(seasons: &[SeasonAnchors]) -> LevelScale {
    let used: Vec<&SeasonAnchors> = seasons.iter().filter(|a| !a.is_empty()).collect();
    let Some(first) = used.first() else { return LevelScale::default() };
    let n = used.len() as f64;
    let points = (0..first.len()).map(|i| (used.iter().map(|a| a[i].0).sum::<f64>() / n, first[i].1)).collect();
    scale_from(points)
}

fn scale_from(mut points: SeasonAnchors) -> LevelScale {
    if points.is_empty() {
        return LevelScale::default();
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
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PositionAligner {
    maps: [Option<(f64, f64)>; POSITION_COUNT],
    all_mid: f64,
    /// Kickers, punters and long snappers on a draft board (`for_draft`);
    /// empty elsewhere.
    #[serde(default)]
    ladders: Vec<Ladder>,
}

/// A specialist position on a draft board: his grade read off his place in
/// his position's class.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct Ladder {
    position: Position,
    /// (class grade, draft grade), best first.
    rungs: Vec<(f64, f64)>,
    /// How the grade moves past the rungs.
    slope: f64,
    /// The most any read of one is worth.
    cap: f64,
}

impl Ladder {
    /// Between two rungs a read falls between their grades; past either
    /// end it moves at the slope.
    fn read(&self, x: f64) -> f64 {
        let r = &self.rungs;
        let (first, last) = (r[0], r[r.len() - 1]);
        let g = if x >= first.0 {
            first.1 + (x - first.0) * self.slope
        } else {
            r.windows(2)
                .find(|w| x >= w[1].0)
                .map(|w| {
                    let ((hi, ghi), (lo, glo)) = (w[0], w[1]);
                    if hi - lo < 1e-9 { ghi } else { glo + (ghi - glo) * (x - lo) / (hi - lo) }
                })
                .unwrap_or(last.1 + (x - last.0) * self.slope)
        };
        g.min(self.cap)
    }
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
        PositionAligner { maps, all_mid, ladders: Vec::new() }
    }

    /// The draft board's scale: aligned like `new`, but a kicker, punter or
    /// long snapper is worth the pick clubs take him at, not his standing
    /// among his own. Lining up every position's best put the best punter
    /// in the top 20. Each specialist position is read off its own class in
    /// order instead: its best at the class's grade at 45% of the draft's
    /// `slots` (kickers), 50% (punters) or 70% (long snappers), each next
    /// one a quarter of the draft lower, the rest past the draft (Claude's
    /// pick, logged). A read between two of his class falls between their
    /// grades, so one great kicker doesn't push the rest out of the draft;
    /// a read above the class's best rises at the position's usual slope to
    /// at most the grade at 18% of the draft (kickers), 27% (punters) or 70%
    /// (long snappers). Clubs then take them a round or two later than that,
    /// only with a need: about two kickers and two punters a draft, as real
    /// clubs do (2000-2025: one kicker in round one, no punter before pick
    /// 70).
    pub fn for_draft(samples: &[(Position, f64)], slots: usize) -> Self {
        let mut a = Self::new(samples);
        let mut all: Vec<f64> = samples.iter().filter(|s| !s.0.is_specialist()).map(|&(p, x)| a.align(p, x)).collect();
        all.sort_unstable_by(|x, y| y.total_cmp(x));
        if all.is_empty() {
            return a;
        }
        let at = |share: f64| all[((slots as f64 * share).round() as usize).clamp(1, all.len()) - 1];
        for (pos, first, rungs, cap) in
            [(Position::K, 0.45, 4, 0.18), (Position::P, 0.50, 4, 0.27), (Position::LS, 0.70, 2, 0.70)]
        {
            let mut raw: Vec<f64> = samples.iter().filter(|s| s.0 == pos).map(|s| s.1).collect();
            if raw.is_empty() {
                continue;
            }
            raw.sort_unstable_by(|x, y| y.total_cmp(x));
            let slope = a.maps[pos.index()].map_or(1.0, |m| m.1);
            let rungs = raw.iter().take(rungs).enumerate().map(|(k, &x)| (x, at(first + 0.25 * k as f64))).collect();
            a.ladders.push(Ladder { position: pos, rungs, slope, cap: at(cap) });
        }
        a
    }

    #[inline]
    pub fn align(&self, position: Position, overall: f64) -> f64 {
        if let Some(l) = self.ladders.iter().find(|l| l.position == position) {
            return l.read(overall);
        }
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
