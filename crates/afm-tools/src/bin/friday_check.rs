//! The Friday night budget (final build): every high school game in a live
//! save runs on the match engine. Builds a world with its history on the
//! quick stand-in (the people are the same size), then plays the live
//! season day by day to the end of the high school year, printing each
//! Friday's (and Saturday's) time, its engine games and memory, and the
//! slowest football day.
//! Usage: friday_check [scale] [seed] [history_years]

use std::time::Instant;

use afm_engine::world::{World, WorldOptions};

fn main() {
    afm_tools::count_heap();
    let args: Vec<String> = std::env::args().collect();
    let scale: f64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1.0);
    let seed: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(2026);
    let years: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1);
    let t = Instant::now();
    let opts = WorldOptions {
        seed,
        scale,
        history_years: years,
        engine_from: Some(2026),
        hs_engine_years: Some(0),
        ..Default::default()
    };
    let mut w = World::new(opts);
    println!(
        "world built in {:.0}s ({} high schools): heap {:.2} GB, resident {}",
        t.elapsed().as_secs_f64(),
        w.high_schools.len(),
        afm_tools::heap_gb(),
        afm_tools::memory_text()
    );
    let year = w.year;
    let (mut worst, mut total, mut fridays) = (0.0f64, 0.0, 0);
    let end = afm_engine::calendar::Date::ymd(year, 12, 24);
    while w.year == year && w.today < end {
        let before = w.log.hs_engine_games;
        let day = w.today;
        let s = Instant::now();
        w.advance_day();
        let dt = s.elapsed().as_secs_f64();
        if w.year != year {
            break;
        }
        let games = w.log.hs_engine_games - before;
        if games > 0 {
            worst = worst.max(dt);
            total += dt;
            fridays += 1;
            println!(
                "{day} ({:?}): {games} engine games in {dt:.1}s, heap {:.2} GB, resident {}",
                day.weekday(),
                afm_tools::heap_gb(),
                afm_tools::memory_text()
            );
        }
    }
    let l = &w.log;
    println!(
        "{fridays} game days, {total:.0}s in all, slowest {worst:.1}s; season: {} games, {} on the engine, {} forfeits",
        l.hs_games, l.hs_engine_games, l.hs_forfeits
    );
}
