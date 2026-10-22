//! Section 6b checks (training): builds a small world and prints the
//! yearly growth of every player by level and age, from his closed season
//! rows (a row closes at his next yearly checkpoint), so a world with
//! growth landing week by week can be set beside one with the yearly step.
//! Also the practice injuries, the load flags and the time the training
//! weeks took.
//! Usage: training_check [scale] [history_years] [seed] [engine_from] [years_measured]
//! `AFM_YEARLY=1` runs the old yearly step instead (the A/B baseline);
//! `AFM_PROFILE=1` prints where each year's time went.

use afm_engine::player::Level;
use afm_engine::world::{Stage, World, WorldOptions};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let scale: f64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.1);
    let years: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(4);
    let seed: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(2026);
    let engine_from: Option<i32> = args.get(4).and_then(|s| s.parse().ok());
    let measured: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(2);
    let t = std::time::Instant::now();
    let yearly_growth = std::env::var_os("AFM_YEARLY").is_some();
    let w = World::new(WorldOptions { seed, scale, history_years: years, engine_from, yearly_growth, ..Default::default() });
    println!(
        "built in {:.0}s (scale {scale}, {years} years, seed {seed}, {})",
        t.elapsed().as_secs_f64(),
        if yearly_growth { "yearly step" } else { "weekly landing" }
    );
    grades(&w);
    report(&w, measured);
}

/// Mean yearly change in overall by level and age, over each active
/// player's last `measured` closed rows (a row closes when the next opens).
fn report(w: &World, measured: usize) {
    // (level, age) -> (sum, sum of squares, n)
    let mut cells: std::collections::BTreeMap<(u8, u32), (f64, f64, u32)> = Default::default();
    let mut by_level: std::collections::BTreeMap<u8, (f64, u32)> = Default::default();
    for p in &w.people {
        if p.stage == Stage::Done {
            continue;
        }
        let h = &p.player.history;
        if h.len() < 2 {
            continue;
        }
        let closed = &h[..h.len() - 1];
        for r in closed.iter().rev().take(measured) {
            let lv = level_code(r.level);
            let g = r.overall - r.overall_start;
            let c = cells.entry((lv, r.age)).or_insert((0.0, 0.0, 0));
            c.0 += g;
            c.1 += g * g;
            c.2 += 1;
            let b = by_level.entry(lv).or_insert((0.0, 0));
            b.0 += g;
            b.1 += 1;
        }
    }
    println!("level  age      n   mean growth   sd");
    for ((lv, age), (s, s2, n)) in &cells {
        if *n < 30 {
            continue;
        }
        let m = s / *n as f64;
        let sd = (s2 / *n as f64 - m * m).max(0.0).sqrt();
        println!("{:<6} {:>3} {:>6} {:>12.2} {:>6.2}", level_name(*lv), age, n, m, sd);
    }
    println!("by level:");
    for (lv, (s, n)) in &by_level {
        println!("  {:<6} n {:>7}  mean {:.2}", level_name(*lv), n, s / (*n).max(1) as f64);
    }
}

/// Practice grades and load across the active players: how many carry a
/// grade, its spread, and the load flags.
fn grades(w: &World) {
    let mut g: Vec<f64> = Vec::new();
    let mut spikes = 0;
    let mut active = 0;
    for p in &w.people {
        if !matches!(p.stage, Stage::College | Stage::Pro) {
            continue;
        }
        active += 1;
        if p.player.train.grade > 0 {
            g.push(p.player.train.grade as f64);
        }
        if p.player.train.spike() > 1.4 {
            spikes += 1;
        }
    }
    let n = g.len().max(1) as f64;
    let m = g.iter().sum::<f64>() / n;
    let sd = (g.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / n).sqrt();
    println!(
        "practice grades: {} of {active} college and pro players graded, mean {m:.1}, sd {sd:.1}; {spikes} with a load spike",
        g.len()
    );
    println!("train store: {} bytes a player", std::mem::size_of::<afm_engine::training::Train>());
    // The fit the high school model uses: a college player's practice grade
    // against his rating over his position group's average on his team.
    let (mut sx, mut sy, mut sxx, mut sxy, mut n) = (0.0, 0.0, 0.0, 0.0, 0.0);
    let mut pts: Vec<(f64, f64)> = Vec::new();
    for prog in &w.programs {
        let mut by_pos: std::collections::HashMap<_, (f64, f64)> = Default::default();
        for &i in &prog.roster {
            let p = &w.people[i].player;
            let e = by_pos.entry(p.position).or_insert((0.0, 0.0));
            e.0 += p.overall();
            e.1 += 1.0;
        }
        for &i in &prog.roster {
            let p = &w.people[i].player;
            if p.train.grade == 0 {
                continue;
            }
            let (s, k) = by_pos[&p.position];
            let x = p.overall() - s / k;
            let y = p.train.grade as f64 - 60.0;
            pts.push((x, y));
            sx += x;
            sy += y;
            sxx += x * x;
            sxy += x * y;
            n += 1.0;
        }
    }
    if n > 10.0 {
        let slope = (n * sxy - sx * sy) / (n * sxx - sx * sx);
        let icpt = (sy - slope * sx) / n;
        let resid = (pts.iter().map(|(x, y)| (y - icpt - slope * x).powi(2)).sum::<f64>() / n).sqrt();
        println!(
            "college fit: grade - 60 = {icpt:.2} + {slope:.3} x (overall - position group mean); residual sd {resid:.2} (running grade), about {:.1} a week",
            resid / 0.539
        );
    }
}

fn level_code(l: Level) -> u8 {
    match l {
        Level::HS => 0,
        Level::SmallCollege => 1,
        Level::FCS => 2,
        Level::FBS => 3,
        Level::NFL => 4,
    }
}

fn level_name(c: u8) -> &'static str {
    ["HS", "D2", "FCS", "FBS", "NFL"][c as usize]
}
