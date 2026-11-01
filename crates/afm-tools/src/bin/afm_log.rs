//! The world log (world reader lane R2): build a world with its log, and
//! read a log back.
//!
//! - `afm_log build <seed> <years> <scale> <folder> [future]`: builds a
//!   world with the log on (`AFM_WORLD_LOG=off`: off, to time the
//!   difference), playing `future` more seasons after the history, and
//!   prints each season's time, memory and the log's size so far
//!   (`AFM_ENGINE_FROM` and `AFM_HS_YEARS` set the engine's years).
//! - `afm_log book <world file> <folder>`: writes a year-end book and the
//!   year's close for a world file as it stands, timed: what a full-size
//!   year's rows cost.
//! - `afm_log sizes <log>`: records and bytes by kind and year.
//! - `afm_log dump <log> <kind> [year]`: the records as JSON lines.

use std::path::{Path, PathBuf};
use std::time::Instant;

use afm_engine::world::wlog::{WorldLog, read};
use afm_engine::world::{World, WorldOptions};

fn main() {
    afm_tools::count_heap();
    let a: Vec<String> = std::env::args().collect();
    let arg = |k: usize| a.get(k).map(String::as_str);
    match arg(1) {
        Some("build") => build(&a[2..]),
        Some("book") => book(Path::new(arg(2).expect("a world file")), Path::new(arg(3).unwrap_or("out"))),
        Some("sizes") => sizes(Path::new(arg(2).expect("a log folder"))),
        Some("dump") => dump(Path::new(arg(2).expect("a log folder")), arg(3).expect("a kind"), arg(4)),
        _ => eprintln!(
            "afm_log build <seed> <years> <scale> <folder> [future] | book <world file> <folder> | sizes <log> | dump <log> <kind> [year]"
        ),
    }
}

fn bytes(dir: &Path) -> u64 {
    read::segments(dir).map_or(0, |s| s.iter().filter_map(|s| s.path.metadata().ok()).map(|m| m.len()).sum())
}

fn build(a: &[String]) {
    let n = |k: usize, d: f64| a.get(k).and_then(|s| s.parse().ok()).unwrap_or(d);
    let (seed, years, scale) = (n(0, 2026.0) as u32, n(1, 30.0) as u32, n(2, 1.0));
    let folder = PathBuf::from(a.get(3).map_or("out", String::as_str));
    let future = n(4, 0.0) as u32;
    let on = std::env::var("AFM_WORLD_LOG").map_or(true, |v| v != "off");
    // AFM_ENGINE_FROM=year keeps the seasons before it on the stand-in and
    // AFM_HS_YEARS=n puts the history's last n high school years on the
    // engine (the world build: every college season and 12), to time one
    // full-size season with every level on the engine.
    let env = |k: &str| std::env::var(k).ok().and_then(|v| v.parse::<i64>().ok());
    let d = WorldOptions::default();
    let opts = WorldOptions {
        seed,
        history_years: years,
        scale,
        keep_game_log: false,
        engine_from: env("AFM_ENGINE_FROM").map(|y| y as i32).or(d.engine_from),
        hs_engine_years: env("AFM_HS_YEARS").map(|n| n as u32).or(d.hs_engine_years),
        ..d
    };
    let dir = folder.join(format!("world-{seed}-{years}y.{}", afm_engine::world::wlog::EXT));
    let log = on.then(|| WorldLog::create(&dir, &opts, "afm_log", "").expect("the log folder"));
    let t = Instant::now();
    let mut last = Instant::now();
    let mut line = |w: &World| {
        println!(
            "season {}: {:.0}s, heap {:.2} GB, resident {}, people {}, log {:.0} MB",
            w.year - 1,
            last.elapsed().as_secs_f64(),
            afm_tools::heap_gb(),
            afm_tools::memory_text(),
            w.people.len(),
            bytes(&dir) as f64 / 1e6
        );
        last = Instant::now();
    };
    let mut w = World::new_logged(opts, log, |w, _| {
        line(w);
        true
    })
    .expect("never stopped");
    println!("history built in {:.0}s (log {})", t.elapsed().as_secs_f64(), if on { "on" } else { "off" });
    for _ in 0..future {
        w.advance_year();
        line(&w);
    }
    if let Some(log) = w.detach_log() {
        let counts = log.counts();
        log.close().expect("the log closes");
        println!("records: {counts:?}");
        sizes(&dir);
    }
    println!(
        "all done in {:.0}s, resident {}, fingerprint {:016x}",
        t.elapsed().as_secs_f64(),
        afm_tools::memory_text(),
        afm_tools::fingerprint(&w)
    );
}

fn book(world: &Path, folder: &Path) {
    let t = Instant::now();
    let mut w = afm_engine::save::read_world(world).expect("a world file");
    println!(
        "read in {:.0}s: {} people, resident {}",
        t.elapsed().as_secs_f64(),
        w.people.len(),
        afm_tools::memory_text()
    );
    let dir = folder.join(format!("book.{}", afm_engine::world::wlog::EXT));
    w.attach_log(WorldLog::create(&dir, &w.options, "afm_log", &w.today.to_string()).expect("the log folder"));
    let t = Instant::now();
    afm_engine::world::wlog::book_now(&w);
    let log = w.detach_log().expect("attached");
    log.close().expect("the log closes");
    println!("book written in {:.1}s, resident {}", t.elapsed().as_secs_f64(), afm_tools::memory_text());
    sizes(&dir);
}

fn sizes(dir: &Path) {
    let segs = read::segments(dir).expect("a log folder");
    let mut by_kind: std::collections::BTreeMap<&str, (u64, u64)> = Default::default();
    let mut by_year: std::collections::BTreeMap<i32, u64> = Default::default();
    for s in &segs {
        let size = s.path.metadata().map_or(0, |m| m.len());
        let n = read::each::<serde::de::IgnoredAny>(&s.path, |_| {}).unwrap_or(0);
        let e = by_kind.entry(&s.kind).or_default();
        e.0 += n;
        e.1 += size;
        *by_year.entry(s.year).or_default() += size;
    }
    let total: u64 = by_kind.values().map(|x| x.1).sum();
    println!("{:<10} {:>12} {:>10} {:>9}", "kind", "records", "MB", "B/record");
    for (k, (n, b)) in &by_kind {
        println!("{k:<10} {n:>12} {:>10.1} {:>9.0}", *b as f64 / 1e6, *b as f64 / (*n).max(1) as f64);
    }
    println!("total {:.1} MB", total as f64 / 1e6);
    for (y, b) in &by_year {
        println!("  {y}: {:.1} MB", *b as f64 / 1e6);
    }
}

fn dump(dir: &Path, kind: &str, year: Option<&str>) {
    let year: Option<i32> = year.and_then(|y| y.parse().ok());
    for s in read::kind(dir, kind).expect("a log folder") {
        if year.is_some_and(|y| y != s.year) {
            continue;
        }
        read::each_json(&s.path, |v| println!("{v}")).expect("a readable segment");
    }
}
