//! Early entrants (the early entrants lane): builds a world and prints,
//! for each football year, the underclassmen read in December by round,
//! who declared and withdrew, those with a degree, and where the entrants
//! went in the draft, against the NFL's real lists.
//! Usage: entry_check [scale] [history_years] [seed] [engine_from] [hs]
//! (high school games on the stand-in, for speed, unless `hs` is given:
//! then they play as the test world plays them).

use afm_engine::world::{World, WorldOptions};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let scale: f64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1.0);
    let years: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(3);
    let seed: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(7);
    let engine_from: i32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(2100);
    let t = std::time::Instant::now();
    let w = World::new(WorldOptions {
        seed,
        scale,
        history_years: years,
        engine_from: Some(engine_from),
        hs_engine_years: if args.get(5).is_some_and(|a| a == "hs") {
            WorldOptions::default().hs_engine_years
        } else {
            None
        },
        ..Default::default()
    });
    println!("built in {:.0}s (seed {seed}, scale {scale}, {years} years)", t.elapsed().as_secs_f64());
    // Real underclassmen on the NFL's list (special eligibility plus degree
    // finishers), by draft: NFL announcements and press counts.
    let real = |draft: i32| match draft {
        2021 => "128",
        2022 => "100 (73 + 27)",
        2023 => "82 (69 + 13)",
        2024 => "about 58-60",
        2025 => "70 (55 + 15)",
        2026 => "63 (42 + 21)",
        2014..=2020 => "about 95-135",
        2000..=2011 => "about 40-60",
        _ => "-",
    };
    println!(
        "{:<6} {:>8} {:>8} {:>8} {:>8} {:>6}  {:<28} {:<40} real",
        "draft", "declared", "withdrew", "net", "degree", "undraf", "drafted by round", "declared/read by Dec round"
    );
    for l in w.log_history.iter() {
        let net = l.declared as i64 - l.withdrew as i64;
        let by: Vec<String> = l.entry_by_round.iter().map(|r| format!("{}/{}", r[1], r[0])).collect();
        println!(
            "{:<6} {:>8} {:>8} {:>8} {:>8} {:>6}  {:<28} {:<40} {}",
            l.year + 1,
            l.declared,
            l.withdrew,
            net,
            l.declared_graduates,
            l.early_drafted[7],
            format!("{:?}", &l.early_drafted[..7]),
            by.join(" "),
            real(l.year + 1)
        );
    }
}
