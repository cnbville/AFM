//! Plays a world file on for whole seasons with the world log on, for the
//! future run (realism audit item 25): `afm_future <world file> <seasons>`.
//!
//! The world is opened as the game opens a library world (tallies beside it
//! in `name.afmdata`, a log in `name.afmlog` if it has one, else a new log
//! from today), played on to January 1 of `seasons` years later, and
//! written as `name-janYYYY.afmworld` beside the input, with its tallies and
//! its log renamed to sit beside it. Run it again on that file to carry the
//! run on (the log carries on after its last segment), on this machine or
//! another that has the three together: a GitHub job may run six hours.
//! Nothing of the world's own code is changed; it is the game's play-on.
//! `AFM_WORLD_LOG=off` plays without a log.

use std::path::{Path, PathBuf};
use std::time::Instant;

use afm_engine::calendar::Date;

#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn resident() -> String {
    let s = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    s.lines()
        .find(|l| l.starts_with("VmRSS"))
        .map_or("?".into(), |l| l.split_whitespace().skip(1).collect::<Vec<_>>().join(" "))
}

fn size(dir: &Path) -> u64 {
    let mut n = 0;
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            n += if p.is_dir() { size(&p) } else { e.metadata().map_or(0, |m| m.len()) };
        }
    }
    n
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let src = PathBuf::from(a.get(1).expect("afm_future <world file> <seasons>"));
    let seasons: i32 = a.get(2).and_then(|s| s.parse().ok()).expect("afm_future <world file> <seasons>");
    let t = Instant::now();
    let mut w = afm_game::files::read_world(&src).unwrap_or_else(|e| panic!("{e}"));
    eprintln!(
        "read {} in {:.0} s: {} people, resident {}, today {}",
        src.display(),
        t.elapsed().as_secs_f64(),
        w.people.len(),
        resident(),
        w.today
    );
    let target = Date::ymd(w.today.year() + seasons, 1, 1);
    let stem = format!(
        "{}-jan{}",
        src.file_stem().and_then(|s| s.to_str()).unwrap().split("-jan").next().unwrap(),
        target.year()
    );
    let dir = src.parent().filter(|d| !d.as_os_str().is_empty()).unwrap_or(Path::new(".")).to_path_buf();
    let to = dir.join(format!("{stem}.afmworld"));
    let data = afm_game::tallies::beside(&src);
    w.attach_tallies(&data).unwrap_or_else(|e| panic!("tallies: {e}"));
    let log_dir = afm_game::world_log::beside(&src);
    if afm_game::world_log::on() {
        let commit = afm_game::world_log::commit();
        let today = w.today.to_string();
        let log = if log_dir.join("meta.json").exists() {
            afm_engine::world::wlog::WorldLog::resume(&log_dir, commit, &today, None)
        } else {
            afm_engine::world::wlog::WorldLog::create(&log_dir, &w.options, commit, &today)
        };
        w.attach_log(log.unwrap_or_else(|e| panic!("log: {e}")));
    }
    let mut last = Instant::now();
    let mut year = w.year;
    let t = Instant::now();
    // One call a day: print when the football year turns over.
    let done = w.play_on_to(target, |w| {
        if w.year != year {
            println!(
                "season {}: {:.0} s, resident {}, people {}, log {:.0} MB, today {}",
                year,
                last.elapsed().as_secs_f64(),
                resident(),
                w.people.len(),
                size(&log_dir) as f64 / 1e6,
                w.today
            );
            last = Instant::now();
            year = w.year;
        }
        true
    });
    assert!(done);
    println!("played to {} in {:.0} s, resident {}", w.today, t.elapsed().as_secs_f64(), resident());
    if let Some(log) = w.detach_log() {
        log.close().expect("the log closes");
    }
    w.detach_tallies();
    let t = Instant::now();
    let part = to.with_extension("afmworld.part");
    afm_engine::save::write_world(&w, &part, afm_game::world_log::commit(), &afm_game::files::now_utc(), None)
        .and_then(|_| std::fs::rename(&part, &to))
        .unwrap_or_else(|e| panic!("write: {e}"));
    println!("{} written in {:.0} s", to.display(), t.elapsed().as_secs_f64());
    // The tallies and the log go beside the new world; the old world file goes.
    for (from, to_dir) in [(data, afm_game::tallies::beside(&to)), (log_dir, afm_game::world_log::beside(&to))] {
        if from.is_dir() && from != to_dir {
            std::fs::rename(&from, &to_dir).unwrap_or_else(|e| panic!("move {}: {e}", from.display()));
        }
    }
    if src != to {
        let _ = std::fs::remove_file(&src);
    }
}
