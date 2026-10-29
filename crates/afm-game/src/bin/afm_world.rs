//! Builds a full world as a world file, the way the game's "Build a world"
//! button does: `afm_world <seed> <years> <folder>`. The "Build a world"
//! workflow runs it to publish worlds on the worlds page.

use std::time::Instant;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let seed = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(2026);
    let years = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(30);
    let dir = std::path::PathBuf::from(a.get(3).map_or(".", String::as_str));
    let t = Instant::now();
    let to = afm_game::jobs::build_world_file(&dir, seed, years, 1.0, |n, year| {
        if n > 0 {
            eprintln!("season {year} done ({n}), {:.0}s", t.elapsed().as_secs_f64());
        }
        true
    })
    .unwrap_or_else(|e| panic!("{e}"));
    eprintln!("{} written in {:.0}s", to.display(), t.elapsed().as_secs_f64());
}
