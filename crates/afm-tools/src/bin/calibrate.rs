//! Prints the headline statistics of a simulated national class so the model
//! can be tuned against real-world rates.
//! Usage: cargo run --release --bin calibrate -- [size] [seed]
//! `AFM_PROG=off` turns the split ceiling off and `AFM_PROG_PARTS` picks
//! its parts (everyday 1, breakouts 2, projects 4, trims 8, the weight
//! room 16), for the
//! build's A/B checks; it also prints the quarterbacks' and backs' median
//! last season.

use std::time::Instant;

use afm_engine::career::{CohortOptions, simulate_cohort};
use afm_tools::stats::summarize;

fn main() {
    let mut args = std::env::args().skip(1);
    let size: usize = args.next().map_or(250_000, |s| s.replace('_', "").parse().expect("size must be a number"));
    let seed = args.next().unwrap_or_else(|| "calibrate".to_string());
    if std::env::var("AFM_PROG").is_ok_and(|v| v == "off") {
        afm_engine::progression::set_enabled(false);
    }
    if let Some(parts) = std::env::var("AFM_PROG_PARTS").ok().and_then(|v| v.parse().ok()) {
        afm_engine::progression::set_parts(parts);
    }
    let t0 = Instant::now();
    let result = simulate_cohort(CohortOptions::new(size, seed));
    eprintln!("simulated {size} players in {:.2}s", t0.elapsed().as_secs_f64());
    println!("{}", serde_json::to_string_pretty(&summarize(&result)).unwrap());
    let last = |p| afm_tools::stats::median_last_age(&result.records, p);
    use afm_engine::positions::Position;
    eprintln!("last season median: quarterbacks {} backs {}", last(Position::QB), last(Position::RB));
}
