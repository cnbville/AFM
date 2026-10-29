//! The world log in the game (world reader lane R2): a career writes the
//! same log a world build does, beside its save (`name.afmlog` next to
//! `name.afmsave`), so the world reader can show every day of it.
//!
//! While you play, the log is written into `logs/live.afmlog` in the game
//! folder. Opening a library world or a save links its log there (hard
//! links, no copy) and writes on after it, so its history stays whole; a
//! world or save without a log starts a new one from that day. Saving
//! links the live log beside the save. Nothing is kept in memory: records
//! go to disk a frame at a time. `AFM_WORLD_LOG=off` turns it off.

use std::path::{Path, PathBuf};

use afm_engine::world::World;
use afm_engine::world::wlog::{self, WorldLog};

/// The commit the log names for this run.
pub fn commit() -> &'static str {
    option_env!("AFM_COMMIT").unwrap_or(env!("CARGO_PKG_VERSION"))
}

/// Whether the game writes the world log (on unless `AFM_WORLD_LOG=off`).
pub fn on() -> bool {
    std::env::var("AFM_WORLD_LOG").map_or(true, |v| v != "off")
}

/// The log beside a world file or a save.
pub fn beside(path: &Path) -> PathBuf {
    path.with_extension(wlog::EXT)
}

fn live(home: &Path) -> PathBuf {
    home.join("logs").join(format!("live.{}", wlog::EXT))
}

/// A world was opened (`from`: its world file or save; none for a world
/// built in the game): the live log starts on it.
pub fn on_open(home: &Path, w: &mut World, from: Option<&Path>) {
    if !on() {
        return;
    }
    let live = live(home);
    // The last world's unsaved log goes with its unsaved days.
    if live.exists()
        && let Err(e) = std::fs::remove_dir_all(&live)
    {
        eprintln!("world log: could not clear {}: {e}", live.display());
        return;
    }
    let today = w.today.to_string();
    let had = from.map(beside).filter(|l| l.join("meta.json").exists());
    let r = match (&had, from) {
        (Some(src), Some(path)) => {
            // A library world's log names the world it came from.
            let history = path
                .extension()
                .is_some_and(|x| x == "afmworld")
                .then(|| path.file_stem().map(|s| s.to_string_lossy().to_string()))
                .flatten();
            wlog::mirror(src, &live).and_then(|_| WorldLog::resume(&live, commit(), &today, history))
        }
        _ => WorldLog::create(&live, &w.options, commit(), &today),
    };
    match r {
        Ok(log) => w.attach_log(log),
        Err(e) => eprintln!("world log: not written this session ({e})"),
    }
}

/// The career was saved to `to`: its log goes beside it as it stands (and
/// an older log there goes, when this career writes none).
pub fn on_save(w: &mut World, to: &Path) {
    let Some(log) = w.world_log() else {
        on_delete(to);
        return;
    };
    if let Err(e) = log.save_to(&beside(to)) {
        eprintln!("world log: not saved beside {} ({e})", to.display());
    }
}

/// A save renamed: its log too.
pub fn on_rename(from: &Path, to: &Path) {
    let (a, b) = (beside(from), beside(to));
    if a.exists() && a != b {
        let _ = std::fs::rename(a, b);
    }
}

/// A save deleted: its log too (a live log linked from it keeps its own links).
pub fn on_delete(path: &Path) {
    let l = beside(path);
    if l.exists() {
        let _ = std::fs::remove_dir_all(l);
    }
}
