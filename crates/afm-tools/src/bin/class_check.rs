//! Class sizes and transfers in (realism audit items 11 and 13): builds a
//! world, plays it on and prints, for each cycle, the FBS programs' high
//! school classes (from the classes tally, walk-ons included; each fall's
//! freshmen on and off aid for the seasons played here) and the transfers
//! each program took from the portal: the average, the 90th percentile,
//! the most, and how many programs took 10, 20 or 30 or more; then each
//! fall's freshmen at the Division II, NAIA, Division III and junior
//! college programs.
//! Usage: class_check [scale] [history_years] [seed] [seasons]

use afm_engine::calendar::Date;
use afm_engine::world::data::Division;
use afm_engine::world::{Stage, World, WorldOptions};
use std::collections::HashMap;

fn line(label: &str, mut v: Vec<usize>) {
    if v.is_empty() {
        return;
    }
    v.sort_unstable();
    let n = v.len();
    let mean = v.iter().sum::<usize>() as f64 / n as f64;
    let at = |q: f64| v[((n as f64 * q) as usize).min(n - 1)];
    let over = |t: usize| v.iter().filter(|&&x| x >= t).count();
    println!(
        "{label:<34} n {n:>3}  mean {mean:>5.1}  p50 {:>3}  p90 {:>3}  max {:>3}  >=10 {:>3}  >=20 {:>3}  >=30 {:>3}  >=40 {:>3}",
        at(0.5),
        at(0.9),
        v[n - 1],
        over(10),
        over(20),
        over(30),
        over(40)
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let scale: f64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.25);
    let years: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(8);
    let seed: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(151);
    let seasons: i32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(2);
    let t = std::time::Instant::now();
    let mut w = World::new(WorldOptions {
        seed,
        scale,
        history_years: years,
        engine_from: Some(2100),
        hs_engine_years: None,
        ..Default::default()
    });
    println!("built in {:.0}s; today {} (world year {})", t.elapsed().as_secs_f64(), w.today.pretty(), w.year);
    let first = w.year;
    let fbs: Vec<usize> = (0..w.programs.len()).filter(|&j| w.programs[j].division == Division::FBS).collect();
    // Freshmen each fall in the seasons played here: [on aid, walk-on].
    let mut fresh: Vec<(i32, Vec<[usize; 2]>)> = Vec::new();
    let end = Date::ymd(first + seasons, 9, 2);
    while w.today < end {
        let d = w.today;
        w.advance_day();
        if d.month() == 9 && d.day() == 1 {
            let mut by = vec![[0usize; 2]; w.programs.len()];
            for p in &w.people {
                if p.stage == Stage::College
                    && p.class_of == d.year()
                    && p.path.len() == 1
                    && let Some(j) = p.college
                    && (!p.juco || w.programs[j].division == Division::JUCO)
                {
                    by[j][p.walk_on as usize] += 1;
                }
            }
            fresh.push((d.year(), by));
        }
    }
    println!("played to {} in {:.0}s", w.today.pretty(), t.elapsed().as_secs_f64());
    // High school classes from the tally (walk-ons included).
    let mut classes: HashMap<(i32, usize), usize> = HashMap::new();
    for s in &w.tallies.classes.signees {
        if s.division == Some(Division::FBS) {
            *classes.entry((s.class, s.program as usize)).or_default() += 1;
        }
    }
    let mut moves: HashMap<(i32, usize), usize> = HashMap::new();
    for r in &w.recruiting_log.portal {
        if let Some(j) = r.to_program
            && !r.returned
            && w.programs[j].division == Division::FBS
        {
            *moves.entry((r.year, j)).or_default() += 1;
        }
    }
    let lo = first - years as i32 + 2;
    for y in lo..first + seasons {
        println!("-- {y}");
        let c: Vec<usize> =
            fbs.iter().map(|&j| classes.get(&(y, j)).copied().unwrap_or(0)).filter(|&x| x > 0).collect();
        line("FBS high school class (tally)", c);
        if let Some((_, by)) = fresh.iter().find(|(s, _)| *s == y) {
            line("FBS freshmen on aid (Sep 1)", fbs.iter().map(|&j| by[j][0]).collect());
            line("FBS freshmen walk-ons (Sep 1)", fbs.iter().map(|&j| by[j][1]).collect());
        }
        // Transfers in after the season just played (y - 1), for fall y.
        line(
            "FBS transfers in (after y-1)",
            fbs.iter().map(|&j| moves.get(&(y - 1, j)).copied().unwrap_or(0)).collect(),
        );
    }
    // Below Division I: each fall's freshmen (on aid and walk-ons) a program.
    for d in [Division::D2, Division::NAIA, Division::D3, Division::JUCO] {
        let progs: Vec<usize> =
            (0..w.programs.len()).filter(|&j| w.programs[j].division == d && w.programs[j].plays(w.year)).collect();
        for (y, by) in &fresh {
            line(&format!("{d:?} {y} freshmen, all"), progs.iter().map(|&j| by[j][0] + by[j][1]).collect());
            line(&format!("{d:?} {y} freshmen on aid"), progs.iter().map(|&j| by[j][0]).collect());
        }
    }
    let mut roster: Vec<usize> = fbs
        .iter()
        .map(|&j| w.programs[j].roster.iter().filter(|&&i| w.people[i].stage == Stage::College).count())
        .collect();
    roster.sort_unstable();
    println!(
        "FBS rosters today: mean {:.1}, min {}, max {}",
        roster.iter().sum::<usize>() as f64 / roster.len() as f64,
        roster[0],
        roster[roster.len() - 1]
    );
}
