//! When recruits get offers and commit (thread "Make AI schools offer on
//! real days"): builds a world on the stand-in and prints, per signing
//! year, commitments by month, broken verbals and signing periods; then,
//! for the last class signed, the days its commitments fell on and where
//! the blue chips signed; and for the class behind it, the offers held by
//! national rank at the end of its junior year.
//! Usage: recruit_days <seed> <history years> [scale]

use afm_engine::world::data::Division;
use afm_engine::world::{Stage, World, WorldOptions};
use std::collections::BTreeMap;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(2026);
    let years: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(4);
    let scale: f64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1.0);
    let t = std::time::Instant::now();
    let w =
        World::new(WorldOptions { seed, scale, history_years: years, engine_from: Some(2100), ..Default::default() });
    println!("built in {:.0}s, today {}", t.elapsed().as_secs_f64(), w.today.pretty());
    println!("year | commits by month Jan..Dec | commits decommits flips | early late | blue verbal broken | by bar");
    for l in w.log_history.iter().rev().take(years as usize).rev() {
        println!(
            "{} | {:?} | {} {} {} | {} {} | {} {} | {}",
            l.year,
            l.commits_by_month,
            l.commits,
            l.decommits,
            l.flips,
            l.signed_early,
            l.signed_late,
            l.blue_verbal,
            l.blue_broken,
            l.bar_commits
        );
    }
    // The last class through the summer: the days its verbals fell on.
    let class = w
        .people
        .iter()
        .filter(|p| p.cal.signed.is_some() && matches!(p.stage, Stage::HighSchool | Stage::PrepSchool))
        .map(|p| p.class_of)
        .max()
        .unwrap_or(w.today.year());
    let mut days: BTreeMap<i32, u32> = BTreeMap::new();
    let mut weeks: BTreeMap<String, u32> = BTreeMap::new();
    let (mut blue, mut blue_before_senior) = (0u32, 0u32);
    for p in w.people.iter().filter(|p| p.class_of == class) {
        let when = p.cal.signed.and_then(|s| s.committed).or(p.cal.commit.map(|c| c.1));
        if p.stars >= 4 && !p.player.position.is_specialist() {
            blue += 1;
            if when.is_some_and(|d| d < afm_engine::calendar::Date::ymd(class - 1, 9, 1)) {
                blue_before_senior += 1;
            }
        }
        let Some(d) = when else { continue };
        *days.entry(d.0).or_default() += 1;
        *weeks.entry(format!("{}-{:02}", d.year(), d.month())).or_default() += 1;
    }
    let busiest: Vec<(String, u32)> = {
        let mut v: Vec<(i32, u32)> = days.iter().map(|(&d, &n)| (d, n)).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v.iter().take(8).map(|&(d, n)| (afm_engine::calendar::Date(d).pretty(), n)).collect()
    };
    let total: u32 = days.values().sum();
    println!("class {class}: {total} verbals on {} days; busiest {:?}; by month {:?}", days.len(), busiest, weeks);
    println!(
        "class {class}: blue chips committed before Sep 1 of the senior year {blue_before_senior} of {blue} ({:.0}%)",
        100.0 * blue_before_senior as f64 / blue.max(1) as f64
    );
    // Where the blue chips signed: the top 25 programs by prestige.
    let mut by_prestige: Vec<usize> =
        (0..w.programs.len()).filter(|&j| w.programs[j].division == Division::FBS).collect();
    by_prestige.sort_by(|&a, &b| w.programs[b].prestige.total_cmp(&w.programs[a].prestige));
    let top25: Vec<usize> = by_prestige.iter().take(25).copied().collect();
    for stars in [5u8, 4, 3] {
        let signed: Vec<usize> = w
            .people
            .iter()
            .filter(|p| p.class_of == class && p.stars == stars && !p.player.position.is_specialist())
            .filter_map(|p| p.cal.signed.map(|s| s.program).or(p.college))
            .collect();
        let at_top = signed.iter().filter(|j| top25.contains(j)).count();
        let fbs = signed.iter().filter(|&&j| w.programs[j].division == Division::FBS).count();
        println!(
            "class {class} {stars}-stars: {} signed, {fbs} FBS, {at_top} at the top 25 by prestige ({:.0}%)",
            signed.len(),
            100.0 * at_top as f64 / signed.len().max(1) as f64
        );
    }
    // The class behind it: offers held by national rank (stars, then the services' read).
    let next = class + 1;
    let mut juniors: Vec<(f64, usize, usize)> = w
        .people
        .iter()
        .filter(|p| p.class_of == next && p.stage == Stage::HighSchool)
        .map(|p| {
            let read = p.cal.service_read.unwrap_or(0.0);
            let early = p.cal.early_offers.len();
            let first = p.cal.early_offers.first().map_or(0, |o| o.1.0 as usize);
            (p.stars as f64 * 100.0 + read, early, first)
        })
        .collect();
    juniors.sort_by(|a, b| b.0.total_cmp(&a.0));
    let at = |r: usize| -> String {
        let r = ((r as f64 * scale) as usize).min(juniors.len().saturating_sub(1));
        let lo = r.saturating_sub(5);
        let hi = (r + 5).min(juniors.len());
        let n: usize = juniors[lo..hi].iter().map(|j| j.1).sum();
        format!("{:.1}", n as f64 / (hi - lo).max(1) as f64)
    };
    println!(
        "class {next} (juniors) early offers held near rank 1 {}, 10 {}, 100 {}, 1000 {}, 3000 {}",
        at(1),
        at(10),
        at(100),
        at(1000),
        at(3000)
    );
    let mut first_month: BTreeMap<String, u32> = BTreeMap::new();
    for p in w.people.iter().filter(|p| p.class_of == next || p.class_of == next + 1) {
        for o in &p.cal.early_offers {
            *first_month.entry(format!("{} {}-{:02}", p.class_of, o.1.year(), o.1.month())).or_default() += 1;
        }
    }
    println!("early offers by class and month: {first_month:?}");
}
