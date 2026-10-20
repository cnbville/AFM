//! Reads a world file (`afm_engine::save`) and says what is in it: the
//! header, the world's date and size, and the time and memory a load takes.
//! With a second argument, plays that many seasons on from it.
//! Usage: world_file path [seasons]

use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = std::path::Path::new(args.get(1).expect("a world file"));
    let seasons: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
    let meta = afm_engine::save::read_meta(path).expect("read the header");
    println!("{meta:?}");
    let t = Instant::now();
    let mut w = afm_engine::save::read_world(path).expect("read the world");
    println!(
        "loaded in {:.0}s: {:?}, year {}, people {}, alumni {}, coaches {}, high schools {}, programs {}, memory {}",
        t.elapsed().as_secs_f64(),
        w.today,
        w.year,
        w.people.len(),
        w.alumni.len(),
        w.coaches.len(),
        w.high_schools.len(),
        w.programs.len(),
        afm_tools::memory_text()
    );
    let with_hs = w.people.iter().filter(|p| !p.hs_lines.is_empty()).count();
    println!("people with high school lines: {with_hs}");
    for _ in 0..seasons {
        let s = Instant::now();
        w.advance_year();
        println!("season to {}: {:.0}s, memory {}", w.year, s.elapsed().as_secs_f64(), afm_tools::memory_text());
    }
}
