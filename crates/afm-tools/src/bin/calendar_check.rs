//! Section 4 checks: builds a world on the daily clock and prints each
//! year's calendar log, the loud events and the key dates, so the calendar
//! can be judged against the real one.
//! Usage: calendar_check [scale] [history_years] [seed] [engine_from]

use afm_engine::calendar::Loudness;
use afm_engine::world::{World, WorldOptions};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let scale: f64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.1);
    let years: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(2);
    let seed: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(2026);
    let engine_from: Option<i32> = args.get(4).and_then(|s| s.parse().ok());
    let t = std::time::Instant::now();
    let w = World::new(WorldOptions { seed, scale, history_years: years, engine_from, keep_game_log: true, ..Default::default() });
    println!("built in {:.0}s", t.elapsed().as_secs_f64());
    for log in w.log_history.iter().rev().take(years as usize).rev() {
        println!("--- {} ---", log.year);
        println!("{log:#?}");
    }
    for (year, events) in w.event_history.iter().rev().take(1) {
        println!("=== loud events {year} ===");
        for e in events.iter().filter(|e| e.loud >= Loudness::Big) {
            println!("{} {:<6} {:<5} {}", e.when.date.pretty(), e.level.name(), e.loud.name(), e.text);
        }
    }
    println!("NFL champions: {:?}", w.nfl_champions);
    for (k, n) in w.calendar.counts.iter() {
        println!("count {k:?}: {n}");
    }
}
