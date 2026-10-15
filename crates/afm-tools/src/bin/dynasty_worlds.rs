//! Builds a whole world with the dynasty prototype switches on
//! (`game::experiment`) and prints the title and margin checks of world
//! trial 5, finding 6, so the rough fixes can be judged over 30 years.
//!
//! Usage: AFM_EXPERIMENT=pos,garbage,schemes dynasty_worlds [seed] [years]

use afm_engine::debug;
use afm_engine::game::experiment;
use afm_engine::world::data::Division;
use afm_engine::world::{World, WorldOptions};
use std::collections::HashMap;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(2026);
    let years: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(30);
    let on = std::env::var("AFM_EXPERIMENT").unwrap_or_default();
    experiment::set_position_values(on.contains("pos"));
    experiment::set_same_schemes(on.contains("schemes"));
    let w = World::new(WorldOptions { seed, history_years: years, ..Default::default() });
    println!("# Dynasty worlds: seed {seed}, {years} years, switches [{on}]\n");

    let fbs: Vec<usize> = w.programs.iter().filter(|p| p.division == Division::FBS).map(|p| p.id).collect();
    let mut champ: HashMap<i32, usize> = HashMap::new();
    let mut best_margin: HashMap<i32, (f64, usize)> = HashMap::new();
    for &p in &fbs {
        for s in &w.programs[p].seasons {
            if s.national_title {
                champ.insert(s.year, p);
            }
            let t = &s.totals;
            if t.games > 0 {
                let m = (t.points_for - t.points_against) as f64 / t.games as f64;
                let e = best_margin.entry(s.year).or_insert((f64::MIN, p));
                if m > e.0 {
                    *e = (m, p);
                }
            }
        }
    }
    let mut ys: Vec<i32> = champ.keys().copied().collect();
    ys.sort();
    println!("| Year | Champion | Best average margin |\n|---|---|---|");
    for y in &ys {
        let (m, p) = best_margin.get(y).copied().unwrap_or((0.0, 0));
        println!("| {y} | {} | {:+.1} ({}) |", w.programs[champ[y]].real_name, m, w.programs[p].real_name);
    }
    let mut total: HashMap<usize, u32> = HashMap::new();
    for p in champ.values() {
        *total.entry(*p).or_default() += 1;
    }
    let mut t: Vec<(usize, u32)> = total.into_iter().collect();
    t.sort_by_key(|b| std::cmp::Reverse(b.1));
    println!(
        "\nTitles: {}",
        t.iter().map(|(p, n)| format!("{} {n}", w.programs[*p].real_name)).collect::<Vec<_>>().join(", ")
    );
    let (mut most, mut fewest) = ((0, String::new(), 0), (99, 0));
    for &s in &ys {
        let win: Vec<usize> = ys.iter().filter(|&&y| y >= s && y < s + 15).map(|y| champ[y]).collect();
        if win.len() < 15 {
            continue;
        }
        let mut c: HashMap<usize, u32> = HashMap::new();
        for p in &win {
            *c.entry(*p).or_default() += 1;
        }
        let (p, n) = c.iter().max_by_key(|x| x.1).map(|(p, n)| (*p, *n)).unwrap();
        if n > most.0 {
            most = (n, w.programs[p].real_name.clone(), s);
        }
        if c.len() < fewest.0 {
            fewest = (c.len(), s);
        }
    }
    // The top program's window, season by season: why it won.
    if let Some(top) = w.programs.iter().find(|p| p.real_name == most.1) {
        println!(
            "\n{} from {}: record, lineup rank and margin rank among FBS, head coach (hidden recruiting, game management, scheme)\n",
            top.real_name, most.2
        );
        println!(
            "| Year | Record | Title | Lineup rank | Margin rank | Coach | Rec | GM | Scheme |\n|---|---|---|---|---|---|---|---|---|"
        );
        for s in top.seasons.iter().filter(|s| s.year >= most.2 && s.year < most.2 + 15) {
            let rank = |f: &dyn Fn(&afm_engine::world::program::ProgramSeason) -> f64| {
                let me = f(s);
                1 + fbs
                    .iter()
                    .filter_map(|&p| w.programs[p].seasons.iter().find(|x| x.year == s.year))
                    .filter(|x| f(x) > me)
                    .count()
            };
            let lr = rank(&|x| x.strength);
            let mr = rank(&|x| {
                if x.totals.games == 0 {
                    f64::MIN
                } else {
                    (x.totals.points_for - x.totals.points_against) as f64 / x.totals.games as f64
                }
            });
            let (name, a) = match s.coach.and_then(|c| w.coaches.iter().find(|x| x.id == c)) {
                Some(c) => (c.name.clone(), Some(debug::coach_ability(c))),
                None => (String::new(), None),
            };
            println!(
                "| {} | {}-{} | {} | {lr} | {mr} | {name} | {:.1} | {:.1} | {:.1} |",
                s.year,
                s.wins,
                s.losses,
                if s.national_title { "yes" } else { "" },
                a.map_or(0.0, |a| a.recruiting),
                a.map_or(0.0, |a| a.game_management),
                a.map_or(0.0, |a| a.scheme)
            );
        }
        println!();
    }
    let margins: Vec<f64> = best_margin.values().map(|x| x.0).collect();
    println!("Most titles in any 15 years: {} ({}, from {})", most.0, most.1, most.2);
    println!("Fewest distinct champions in a 15-year window: {} (from {})", fewest.0, fewest.1);
    println!(
        "Best team's average margin: mean {:+.1}, range {:+.1} to {:+.1}",
        margins.iter().sum::<f64>() / margins.len() as f64,
        margins.iter().cloned().fold(f64::MAX, f64::min),
        margins.iter().cloned().fold(f64::MIN, f64::max)
    );
    // Programs with no winning season in the last 15 years.
    let last = *ys.last().unwrap_or(&0);
    let none: Vec<String> = fbs
        .iter()
        .filter(|&&p| {
            let ss: Vec<_> = w.programs[p].seasons.iter().filter(|s| s.year > last - 15).collect();
            ss.len() >= 15 && ss.iter().all(|s| s.wins <= s.losses)
        })
        .map(|&p| w.programs[p].real_name.clone())
        .collect();
    println!("FBS programs with no winning season in the last 15 years: {} ({})", none.len(), none.join(", "));
}
