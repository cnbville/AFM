//! How players grow in a year, from the world's own records: the carry the
//! Needs map and the staff's roster calls both read (staff urgency, item
//! 7: "the growth carry moves to the engine so the Needs map and the
//! advice read the same numbers").
//!
//! Each row of a player's history is one season's change in overall at an
//! age and level. The table keeps the 10th, 25th, 50th, 75th and 90th
//! percentile of that change for every age and level with 20 rows or more,
//! read from every third person (everyone still in football).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::World;
use crate::player::Level;

/// A year's growth at (age, tier): the 10th, 25th, 50th, 75th and 90th
/// percentile of the change in overall.
pub type Table = HashMap<(u32, u8), [f64; 5]>;

/// The middle of a year's growth.
pub const MID: usize = 2;
/// A good case: the table's upper band, so a late bloomer gets his chance
/// (staff urgency; proposed, not confirmed).
pub const GOOD: usize = 3;

/// A quantile of a list (sorted in place; 0 when empty).
pub fn quantile(v: &mut [f64], q: f64) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    let x = q.clamp(0.0, 1.0) * (v.len() - 1) as f64;
    let (lo, hi) = (x.floor() as usize, x.ceil() as usize);
    v[lo] + (v[hi] - v[lo]) * (x - lo as f64)
}

/// 0 high school, 1 college, 2 the NFL.
pub fn tier_of(l: Level) -> u8 {
    match l {
        Level::HS => 0,
        Level::NFL => 2,
        _ => 1,
    }
}

/// The table read fresh from the world's records.
pub fn table(w: &World) -> Table {
    let mut by: HashMap<(u32, u8), Vec<f64>> = HashMap::new();
    for p in w.people.iter().step_by(3) {
        for h in &p.player.history {
            if h.age >= 14 && h.age <= 38 {
                by.entry((h.age, tier_of(h.level))).or_default().push(h.overall - h.overall_start);
            }
        }
    }
    by.into_iter()
        .filter(|(_, v)| v.len() >= 20)
        .map(|(k, mut v)| {
            let q = [0.1, 0.25, 0.5, 0.75, 0.9].map(|x| quantile(&mut v, x));
            (k, q)
        })
        .collect()
}

/// The table kept for the season, for the screens (one world open at a
/// time). The world's own steps read `table` fresh, so two worlds in one
/// process never share a table.
pub fn cached(w: &World) -> Arc<Table> {
    type Kept = Option<((u32, i32, usize), Arc<Table>)>;
    static CACHE: Mutex<Kept> = Mutex::new(None);
    let key = (w.options.seed, w.year, w.people.len());
    let mut g = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((k, v)) = g.as_ref()
        && *k == key
    {
        return v.clone();
    }
    let t = Arc::new(table(w));
    *g = Some((key, t.clone()));
    t
}

/// A year's growth quantiles at an age and level (the nearest age kept).
pub fn at(table: &Table, age: u32, tier: u8) -> [f64; 5] {
    for d in 0..6 {
        for a in [age + d, age.saturating_sub(d)] {
            if let Some(q) = table.get(&(a, tier)) {
                return *q;
            }
        }
    }
    [0.0; 5]
}

/// Carries a read forward a year at a time at one band of the table (`MID`
/// or `GOOD`): the read now, then after each year, at the level of each.
pub fn carry(table: &Table, start: f64, age: u32, tiers: &[u8], band: usize) -> Vec<f64> {
    let mut v = vec![start];
    let mut x = start;
    for (k, &tier) in tiers.iter().enumerate() {
        x = (x + at(table, age + k as u32, tier)[band]).clamp(1.0, 99.0);
        v.push(x);
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantiles_read_a_sorted_list() {
        let mut v = vec![5.0, 1.0, 3.0, 2.0, 4.0];
        assert_eq!(quantile(&mut v, 0.5), 3.0);
        assert_eq!(quantile(&mut v, 0.0), 1.0);
        assert_eq!(quantile(&mut v, 1.0), 5.0);
    }

    #[test]
    fn a_carry_adds_each_year_at_its_band() {
        let mut t = Table::new();
        t.insert((20, 1), [-1.0, 0.0, 2.0, 4.0, 6.0]);
        t.insert((21, 1), [-1.0, 0.0, 1.0, 3.0, 5.0]);
        assert_eq!(carry(&t, 60.0, 20, &[1, 1], MID), vec![60.0, 62.0, 63.0]);
        assert_eq!(carry(&t, 60.0, 20, &[1, 1], GOOD), vec![60.0, 64.0, 67.0]);
    }
}
