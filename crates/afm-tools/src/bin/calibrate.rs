//! Prints the headline statistics of a simulated national class so the model
//! can be tuned against real-world rates.
//! Usage: cargo run --release --bin calibrate -- [size] [seed]

use std::time::Instant;

use afm_engine::career::{CohortOptions, simulate_cohort};
use afm_tools::stats::summarize;

fn main() {
    let mut args = std::env::args().skip(1);
    let size: usize = args.next().map_or(250_000, |s| s.replace('_', "").parse().expect("size must be a number"));
    let seed = args.next().unwrap_or_else(|| "calibrate".to_string());
    let t0 = Instant::now();
    let result = simulate_cohort(CohortOptions::new(size, seed));
    eprintln!("simulated {size} players in {:.2}s", t0.elapsed().as_secs_f64());
    println!("{}", serde_json::to_string_pretty(&summarize(&result)).unwrap());
}
