//! Builds a world with its hidden history and prints headline checks.
//! Usage: world [scale] [history_years] [seed]

use std::time::Instant;

use afm_engine::debug;
use afm_engine::world::data::Division;
use afm_engine::world::{World, WorldOptions};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let scale: f64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.1);
    let years: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(10);
    let seed: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(2026);
    let t = Instant::now();
    let w = World::new(WorldOptions { seed, scale, history_years: years, ..Default::default() });
    println!(
        "built {} years at scale {scale} in {:.1}s; {} active players, {} pros",
        years,
        t.elapsed().as_secs_f64(),
        w.people.len(),
        w.alumni.len()
    );
    afm_tools::world_report::print(&w);
    afm_tools::world_report::talent(&w);
    afm_tools::world_report::juco_blue_chips(&w);
    let _ = debug::alumnus_potential;
    let _ = Division::FBS;
}
