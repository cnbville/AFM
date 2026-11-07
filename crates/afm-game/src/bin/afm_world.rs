//! Builds a full world as a world file, the way the game's "Build a world"
//! button does: `afm_world <seed> <years> <folder> [scale]`. The "Build a
//! world" workflow runs it to publish worlds on the worlds page; a scale
//! under 1 makes a smaller world for checks.
//!
//! With `AFM_PAUSE_AFTER=<minutes>` (over 0, and the world log off) the build stops
//! at the first season's end after that many minutes and writes the world
//! so far as `name.afmworld.half`, beside its tallies so far
//! (`name.afmdata.part`); `afm_world resume <folder>/name.afmworld.half`
//! carries it on, on this machine or another, and pauses the same way
//! (NB13: a GitHub job may run six hours, and a 30-year world runs more).
//! The seasons come out as in a build that never paused.
//!
//! With `AFM_PRINTS=<file>` each season's end appends a line to `file`: the
//! season and a fingerprint of every store of the world (`fp::field_prints`).
//! Two builds of one seed must write the same lines (the world speed lane's
//! proof); the time they take is left out of the times printed.
//! `afm_world prints <world file>` prints a written world's store prints.

use std::time::Instant;

use afm_game::jobs::{self, Part};

/// A faster allocator for the world build (world speed lane): the match
/// engine makes and drops many small buffers every snap, on every core.
/// Results don't depend on it (the trial tools have used it all along).
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let t = Instant::now();
    // Minutes; empty or 0 never pauses.
    let pause_after = std::env::var("AFM_PAUSE_AFTER")
        .ok()
        .and_then(|m| m.trim().parse::<f64>().ok())
        .filter(|&m| m > 0.0)
        .map(|m| m * 60.0);
    let prints = std::env::var_os("AFM_PRINTS").map(std::path::PathBuf::from);
    let mut printing = 0.0;
    let pause = |w: &afm_engine::world::World| {
        if let Some(to) = &prints {
            let p = Instant::now();
            let line: Vec<String> =
                afm_engine::world::wlog::fp::field_prints(w).iter().map(|(k, v)| format!("{k}={v:016x}")).collect();
            let line = format!("{} {}\n", w.year - 1, line.join(" "));
            let mut f = std::fs::OpenOptions::new().create(true).append(true).open(to).expect("AFM_PRINTS file");
            std::io::Write::write_all(&mut f, line.as_bytes()).expect("AFM_PRINTS write");
            printing += p.elapsed().as_secs_f64();
        }
        let s = t.elapsed().as_secs_f64() - printing;
        eprintln!("season {} done, {s:.0}s, {} left", w.year - 1, w.history_left());
        pause_after.is_some_and(|p| s >= p)
    };
    // `afm_world prints <world file>`: the store prints of a written world.
    if a.get(1).map(String::as_str) == Some("prints") {
        let path = std::path::PathBuf::from(a.get(2).expect("afm_world prints <world file>"));
        let w = afm_engine::save::read_world(&path).unwrap_or_else(|e| panic!("{e}"));
        for (k, v) in afm_engine::world::wlog::fp::field_prints(&w) {
            println!("{k}={v:016x}");
        }
        return;
    }
    let part = if a.get(1).map(String::as_str) == Some("resume") {
        let half = std::path::PathBuf::from(a.get(2).expect("afm_world resume <folder>/name.afmworld.half"));
        jobs::resume_world_file(&half, pause)
    } else {
        let seed = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(2026);
        let years = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(30);
        let dir = std::path::PathBuf::from(a.get(3).map_or(".", String::as_str));
        let scale = a.get(4).and_then(|s| s.parse().ok()).unwrap_or(1.0);
        jobs::build_world_in_parts(&dir, seed, years, scale, pause)
    };
    match part.unwrap_or_else(|e| panic!("{e}")) {
        Part::Done(to) => eprintln!("{} written in {:.0}s", to.display(), t.elapsed().as_secs_f64()),
        Part::Half(half) => eprintln!("paused: {} written in {:.0}s", half.display(), t.elapsed().as_secs_f64()),
    }
}
