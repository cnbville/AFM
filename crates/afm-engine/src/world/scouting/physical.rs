//! How the six ratings the drills read grow in a year (projected test
//! numbers, Cain 2026-11-14, items 4 and 8): speed, acceleration, agility,
//! jumping, flexibility and strength, each fitted the way the road fits the
//! overall (`yours::gap_fit`): `change = drift + share * gap`, by age and
//! level, where the gap is his ceiling less his overall.
//!
//! The world keeps no season-by-season record of single ratings (a season
//! row holds the overall and the potential only), so the fit can't be read
//! off the records the way the overall's is. Instead it takes a sample of
//! today's players at each level, copies them, and runs one year of the
//! world's own development step on the copies (`develop_season`, the year's
//! training landed at the staff's default): every hidden thing that moves a
//! rating (his own ceilings, bloom, longevity, wear, injuries, the weight
//! room, stalls and spurts) moves it on the copy as it would on him.
//! Nothing in the world changes and no player's hidden numbers leave this
//! module: only each age and level's drift, share and the spread of what
//! the copies did around them. Each copy has a neutral staff (coaching 10,
//! the scale's average) and his last season's role.
//!
//! Display only: the game reads it for projections, no world step does.

use std::collections::HashMap;

use rayon::prelude::*;

use crate::attributes::Attr;
use crate::development::{SeasonContext, develop_season};
use crate::player::{Level, PlayingTime};
use crate::positions::Position;
use crate::rng::{Rng, clamp};
use crate::world::season::key_uniform;
use crate::world::{Stage, World};

/// The six ratings the drills read (`body::Drills`), in this order.
pub const PHYSICAL: [Attr; 6] =
    [Attr::Speed, Attr::Acceleration, Attr::Agility, Attr::Jumping, Attr::Flexibility, Attr::Strength];

/// Copies stepped at each level (high school, college, the NFL), at most.
const SAMPLE: f64 = 15_000.0;
/// Fewest copies an age and level needs for its own fit.
const MIN_CELL: usize = 30;

/// One age and level's fit.
#[derive(Clone, Debug, Default)]
pub struct Cell {
    /// Drift and share for each rating in `PHYSICAL`'s order.
    pub step: [[f64; 2]; 6],
    /// The overall's own drift and share on the same copies (a check
    /// against `yours::gap_fit`, which reads the world's records).
    pub overall: [f64; 2],
    /// What each copy did around the fit, all six together, so a
    /// projection draws whole years (a spurt in speed comes with one in
    /// acceleration).
    pub resid: Vec<[f32; 6]>,
}

/// One copy's year: gap, overall change and the six changes.
type Row = (f64, f64, [f64; 6]);

/// The fit for every age and level the world has enough players at.
#[derive(Clone, Debug, Default)]
pub struct PhysFit {
    pub cells: HashMap<(u32, u8), Cell>,
}

/// The age a pro's body reads as: a position ages its bodies up to a year
/// and a half early or late (`development::decline_onset`), so a running
/// back of 27 is fitted with the men whose bodies are as old as his.
pub fn body_age(age: u32, tier: u8, pos: Position) -> u32 {
    if tier < 2 {
        return age;
    }
    let off = clamp(pos.profile().aging_offset, -1.5, 1.5);
    (age as f64 - off).round().max(14.0) as u32
}

impl PhysFit {
    /// The fit for a year that starts at this age, level and position (the
    /// nearest age kept; None: no fit at that level).
    pub fn cell(&self, age: u32, tier: u8, pos: Position) -> Option<&Cell> {
        let a = body_age(age, tier, pos);
        for d in 0..8 {
            for k in [a + d, a.saturating_sub(d)] {
                if let Some(c) = self.cells.get(&(k, tier)) {
                    return Some(c);
                }
            }
        }
        None
    }
}

/// The level a player develops at this season (0 high school, 1 college,
/// 2 the NFL), None for anyone the yearly step doesn't reach that way.
fn level_of(w: &World, p: &crate::world::Person) -> Option<(Level, u8)> {
    match p.stage {
        Stage::HighSchool => Some((Level::HS, 0)),
        Stage::College => {
            let c = p.college?;
            Some((crate::world::division_level(w.programs.get(c)?.division), 1))
        }
        Stage::Pro if p.pro.as_ref().is_some_and(|c| c.team.is_some()) => Some((Level::NFL, 2)),
        _ => None,
    }
}

/// The fit from one year of development on copies of today's players.
pub fn physical_fit(w: &World) -> PhysFit {
    let seed = w.options.seed as u64 ^ ((w.year as u64) << 20) ^ 0x9475_1CA1;
    let mut count = [0usize; 3];
    for p in &w.people {
        if let Some((_, t)) = level_of(w, p) {
            count[t as usize] += 1;
        }
    }
    let rate = count.map(|n| if n == 0 { 0.0 } else { (SAMPLE / n as f64).min(1.0) });
    let picked: Vec<(usize, Level, u8)> = w
        .people
        .iter()
        .enumerate()
        .filter_map(|(i, p)| {
            let (lv, t) = level_of(w, p)?;
            (key_uniform(seed, p.player.id as u64) < rate[t as usize]).then_some((i, lv, t))
        })
        .collect();
    let rows: Vec<((u32, u8), Row)> = picked
        .par_iter()
        .map(|&(i, level, tier)| {
            let p = &w.people[i];
            let mut q = p.player.clone();
            // What this year still has to land lands first, so the copy
            // starts where he ends the year.
            crate::training::close_year(&mut q, None);
            let before = PHYSICAL.map(|a| q.attributes.get(a));
            let (o0, gap) = (q.overall(), (q.potential() - q.overall()).max(0.0));
            let role = p.last_role.unwrap_or(PlayingTime::Backup);
            let ctx = SeasonContext::new(level, 10.0, role);
            let mut rng = Rng::new(p.player.id ^ (seed as u32) ^ 0x51_F17);
            develop_season(&mut q, &ctx, &mut rng);
            crate::training::close_year(&mut q, None);
            let after = PHYSICAL.map(|a| q.attributes.get(a));
            let key = (body_age(p.player.age, tier, p.player.position), tier);
            (key, (gap, q.overall() - o0, std::array::from_fn(|k| after[k] - before[k])))
        })
        .collect();
    let mut by: HashMap<(u32, u8), Vec<Row>> = HashMap::new();
    for (k, r) in rows {
        by.entry(k).or_default().push(r);
    }
    let fit = |v: &[(f64, f64)]| -> [f64; 2] {
        let n = v.len() as f64;
        let (mg, mc) = (v.iter().map(|x| x.0).sum::<f64>() / n, v.iter().map(|x| x.1).sum::<f64>() / n);
        let var = v.iter().map(|x| (x.0 - mg).powi(2)).sum::<f64>() / n;
        let cov = v.iter().map(|x| (x.0 - mg) * (x.1 - mc)).sum::<f64>() / n;
        let share = if var > 1e-6 { clamp(cov / var, 0.0, 1.0) } else { 0.0 };
        [mc - share * mg, share]
    };
    let cells = by
        .into_iter()
        .filter(|(_, v)| v.len() >= MIN_CELL)
        .map(|(k, v)| {
            let step: [[f64; 2]; 6] =
                std::array::from_fn(|j| fit(&v.iter().map(|x| (x.0, x.2[j])).collect::<Vec<_>>()));
            let overall = fit(&v.iter().map(|x| (x.0, x.1)).collect::<Vec<_>>());
            let resid =
                v.iter().map(|x| std::array::from_fn(|j| (x.2[j] - step[j][0] - step[j][1] * x.0) as f32)).collect();
            (k, Cell { step, overall, resid })
        })
        .collect();
    PhysFit { cells }
}
