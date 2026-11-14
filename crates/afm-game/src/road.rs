//! The road ahead (own potential items 5 and 6, Cain 2026-11-13): where a
//! player's rating goes in each coming season, from the read that judges
//! him (`reads::Seen::judging`: yours from your first own look, your
//! staff's before), the same road on the Shape tab and in "at our place"
//! and "best for us".
//!
//! Each year he closes the share of the gap to his ceiling read that
//! players his age and level closed in this world (`yours::gap_fit`,
//! fitted as a drift plus a share of the gap, so the years bodies fade
//! still fade), and the middle never climbs past the ceiling read. The
//! band is the world's own spread of a year's growth at that age and
//! level (`growth_table`) plus the ceiling read's sd, carried by the share
//! of the gap closed by then. Two kids read the same today get different
//! roads when their ceiling reads differ.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use afm_engine::world::scouting::yours;
use afm_engine::world::{Person, Stage, World};

use crate::lane_squadboards::{growth_at, growth_table, tier_of};
use crate::reads::{self, Judging};
use crate::views;

/// One coming season on the road: the 10th, 25th, 50th, 75th and 90th
/// percentile of his rating then.
#[derive(Clone, Copy, Debug)]
pub struct Row {
    pub season: i32,
    pub age: u32,
    /// 0 high school, 1 college, 2 the NFL.
    pub tier: u8,
    pub q: [f64; 5],
}

/// A player's road: the read it starts from and each coming season.
#[derive(Clone, Debug)]
pub struct Road {
    pub from: Judging,
    pub rows: Vec<Row>,
}

impl Road {
    /// The middle of the road `years` seasons from now (0: today's read;
    /// past the road's end, its last season).
    pub fn mid(&self, years: i32) -> f64 {
        if years <= 0 || self.rows.is_empty() {
            return self.from.now.0;
        }
        self.rows[(years as usize - 1).min(self.rows.len() - 1)].q[2]
    }
}

/// The world's gap fit, kept for the season.
fn fit(w: &World) -> Arc<HashMap<(u32, u8), [f64; 2]>> {
    type G = Arc<HashMap<(u32, u8), [f64; 2]>>;
    static CACHE: Mutex<Option<((u32, i32), G)>> = Mutex::new(None);
    let key = (w.options.seed, w.year);
    let mut g = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((k, v)) = g.as_ref()
        && *k == key
    {
        return v.clone();
    }
    let v = Arc::new(yours::gap_fit(w));
    *g = Some((key, v.clone()));
    v
}

/// His coming seasons: the season, his age then and the level he'd be at
/// (a high schooler goes to college after his class year and on four
/// years later; a college player after his seasons left).
pub fn seasons(w: &World, p: &Person, n: i32) -> Vec<(i32, u32, u8)> {
    use afm_engine::world::eligibility;
    let tier_now = tier_of(views::level_of(w, p));
    let college_left = || eligibility::seasons_left(p, eligibility::division_of(p, &w.programs), w.year) as i32;
    (1..=n)
        .map(|k| {
            let season = w.year + k;
            let tier = match p.stage {
                Stage::HighSchool | Stage::PrepSchool => {
                    if season < p.class_of {
                        0
                    } else if season < p.class_of + 4 {
                        1
                    } else {
                        2
                    }
                }
                Stage::College => {
                    if k < college_left() {
                        1
                    } else {
                        2
                    }
                }
                _ => tier_now,
            };
            (season, p.player.age + k as u32, tier)
        })
        .collect()
}

/// His road over the coming eight seasons from the read that judges him
/// (None while nobody of yours has read him).
pub fn road(w: &World, i: usize) -> Option<Road> {
    let p = w.people.get(i)?;
    let from = reads::judging(w, i)?;
    Some(Road { from, rows: walk(w, &from, &seasons(w, p, 8)) })
}

/// The road from a read over the given seasons.
pub fn walk(w: &World, from: &Judging, seasons: &[(i32, u32, u8)]) -> Vec<Row> {
    let table = growth_table(w);
    let fit = fit(w);
    let (m0, half) = from.now;
    let (ceiling, csd) = from.ceiling;
    let h2 = half * half;
    let (mut p50, mut s_lo, mut s_hi, mut s_lo2, mut s_hi2) = (m0, h2, h2, h2, h2);
    // The share of the gap closed so far: how much of the ceiling read's
    // own doubt the road carries by then.
    let mut closed = 0.0;
    let mut out = Vec::with_capacity(seasons.len());
    for &(season, a, tier) in seasons {
        let q = growth_at(&table, a - 1, tier);
        let [drift, share] = yours::road_step(&fit, a - 1, tier);
        p50 = step_mid(p50, ceiling, drift, share);
        closed = 1.0 - (1.0 - closed) * (1.0 - share);
        s_lo += (q[2] - q[1]).powi(2);
        s_hi += (q[3] - q[2]).powi(2);
        s_lo2 += (q[2] - q[0]).powi(2);
        s_hi2 += (q[4] - q[2]).powi(2);
        // The ceiling read's sd at the band's own width: 25-75 is 0.674 sd
        // each side, 10-90 is 1.28.
        let c = closed * csd;
        let (c1, c2) = ((0.674 * c).powi(2), (1.28 * c).powi(2));
        let q = [
            p50 - (s_lo2 + c2).sqrt(),
            p50 - (s_lo + c1).sqrt(),
            p50,
            p50 + (s_hi + c1).sqrt(),
            p50 + (s_hi2 + c2).sqrt(),
        ]
        .map(|x| (x.clamp(1.0, 99.0) * 10.0).round() / 10.0);
        out.push(Row { season, age: a, tier, q });
    }
    out
}

/// One season on the road's middle: a drift plus a share of the gap to
/// the ceiling read, never past the ceiling on the way up.
fn step_mid(p50: f64, ceiling: f64, drift: f64, share: f64) -> f64 {
    let step = drift + share * (ceiling - p50).max(0.0);
    let mut next = p50 + step;
    if step > 0.0 {
        next = next.min(ceiling.max(p50));
    }
    next.clamp(1.0, 99.0)
}

/// The road's middle from an overall now and a ceiling over the given
/// seasons (`walk`'s middle, unrounded, without its band): what the
/// projected test numbers read the gap from, so the drills and the road
/// stay on the same road.
pub fn mids(w: &World, now: f64, ceiling: f64, seasons: &[(i32, u32, u8)]) -> Vec<f64> {
    let fit = fit(w);
    let mut p50 = now;
    seasons
        .iter()
        .map(|&(_, a, tier)| {
            let [drift, share] = yours::road_step(&fit, a - 1, tier);
            p50 = step_mid(p50, ceiling, drift, share);
            p50
        })
        .collect()
}
