//! Autosave (Cain, 2026-11-14): a quick save every 10 minutes of play that
//! never stops the screen.
//!
//! A save takes seconds on a full world, and the game cannot copy the world
//! to write it on the side (a copy is 3 to 4 GB, which an 8 GB Mac does not
//! have). So the save is taken the way a database takes a snapshot: the game
//! forks itself at a safe moment, and the copy of the process, which shares
//! every page of the world with the game until one of them changes it, writes
//! the file and exits. The game goes straight on.
//!
//! - Safe moment: the fork happens inside `autosave_tick`, a command like any
//!   other, so the game is between two commands, never in the middle of a day.
//! - Consistent: the child sees the world exactly as it stood at that moment,
//!   whatever the game does next.
//! - Atomic: the child writes through the same `write_world` as a save by
//!   hand, into `name.partial`, and renames it over the old autosave when it
//!   is whole. A crash or a kill at any point leaves the old file as it was.
//! - One at a time: a second autosave never starts while one runs, and a save
//!   by hand (or a bug pack, or opening another world) ends a running one
//!   first, so two saves never overlap.
//! - Nothing new to remember: the file is `saves/Autosave - <coach>.afmsave`,
//!   one per coach, replaced each time, listed on the Load page like any save.
//!   The world log, the tallies, the notebook and the depth plan go beside it
//!   as they stand at the same moment; a career's pictures do not.
//! - Quiet: the child runs at a lower priority, and an autosave starts only
//!   when something was done since the last save of any kind, and never while
//!   a world is being built or downloaded.
//!
//! The screens call `autosave_tick` about every 30 seconds while a career is
//! open; the game decides whether 10 minutes have passed. Macs and Linux
//! only (`fork`); elsewhere the tick answers `off`.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::{Game, Reply, depth_plan, notebook, setup, tallies, world_log};

/// How long a career plays between autosaves.
pub const EVERY: Duration = Duration::from_secs(600);
/// A save that takes longer than this is ended (a child stuck in a lock).
const STUCK: Duration = Duration::from_secs(20 * 60);
/// The child's niceness: the screens come first.
#[cfg(unix)]
const NICE: i32 = 10;

#[cfg(unix)]
mod sys {
    unsafe extern "C" {
        pub fn fork() -> i32;
        pub fn _exit(code: i32) -> !;
        pub fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
        pub fn kill(pid: i32, sig: i32) -> i32;
        pub fn setpriority(which: i32, who: u32, prio: i32) -> i32;
    }
    pub const SIGKILL: i32 = 9;
    pub const EINTR: i32 = 4;
}

/// Where the last autosave stands.
#[derive(Clone, Debug)]
enum Phase {
    /// Nothing yet this career.
    Idle,
    /// The child is writing (its process number, when it started).
    Saving { pid: i32, since: Instant, partial: PathBuf },
    /// The file is whole: when, and how long it took.
    Saved { at: Instant, took: f64 },
    /// It failed: when, and a plain line why.
    Failed { at: Instant, note: String },
}

/// The phase and the number of the save it belongs to, shared with the
/// thread that waits for the child.
struct Shared {
    phase: Phase,
    token: u64,
}

pub struct Autosave {
    shared: Arc<Mutex<Shared>>,
    /// Play between autosaves (tests shorten it).
    pub every: Duration,
    /// The last save of any kind, or the day the career opened.
    last: Instant,
    /// Commands run so far that may have changed something, and the count
    /// when the last save was taken.
    activity: u64,
    saved_at: u64,
}

impl Default for Autosave {
    fn default() -> Autosave {
        Autosave {
            shared: Arc::new(Mutex::new(Shared { phase: Phase::Idle, token: 0 })),
            every: EVERY,
            last: Instant::now(),
            activity: 0,
            saved_at: 0,
        }
    }
}

/// Commands that change nothing and so don't count as play.
fn is_idle_call(cmd: &str) -> bool {
    cmd.starts_with("autosave_") || matches!(cmd, "hello" | "library" | "job" | "online_worlds" | "ai_status")
}

impl Autosave {
    /// Every command passes here: any but the idle ones count as play.
    pub fn touch(&mut self, cmd: &str) {
        if !is_idle_call(cmd) {
            self.activity += 1;
        }
    }

    /// A career opened or began: the clock starts again and nothing is
    /// owed. A save still running is ended.
    pub fn reset(&mut self) {
        self.cancel();
        self.last = Instant::now();
        self.saved_at = self.activity;
        self.set(Phase::Idle);
    }

    /// A save by hand was taken: its moment counts as the last save. Ends
    /// a running autosave first (call `cancel` before writing).
    pub fn saved_by_hand(&mut self) {
        self.last = Instant::now();
        self.saved_at = self.activity;
    }

    fn set(&self, phase: Phase) {
        if let Ok(mut s) = self.shared.lock() {
            s.phase = phase;
        }
    }

    fn phase(&self) -> Phase {
        self.shared.lock().map_or(Phase::Idle, |s| s.phase.clone())
    }

    /// Ends a running autosave and takes its half-written file away. The
    /// old autosave stays as it was.
    pub fn cancel(&mut self) {
        let Ok(mut s) = self.shared.lock() else { return };
        if let Phase::Saving { pid, partial, .. } = s.phase.clone() {
            #[cfg(unix)]
            {
                // SAFETY: SIGKILL to the child this game forked.
                unsafe { sys::kill(pid, sys::SIGKILL) };
            }
            let _ = pid;
            let _ = std::fs::remove_file(partial);
            s.token += 1;
            s.phase = Phase::Idle;
        }
    }

    /// The answer the screens read.
    pub fn status(&self) -> Value {
        let ago = |t: Instant| t.elapsed().as_secs();
        let due = self.every.saturating_sub(self.last.elapsed()).as_secs();
        match self.phase() {
            Phase::Idle => json!({ "state": "idle", "next_in": due }),
            Phase::Saving { since, .. } => json!({ "state": "saving", "for": ago(since) }),
            Phase::Saved { at, took } => json!({ "state": "saved", "age": ago(at), "took": took, "next_in": due }),
            Phase::Failed { at, note } => json!({ "state": "failed", "age": ago(at), "note": note, "next_in": due }),
        }
    }
}

/// Where this career's autosave is written.
pub fn path_for(g: &Game) -> Option<PathBuf> {
    let name = g.world.as_ref()?.career.as_ref()?.you.name.clone();
    let name = setup::file_safe(&name);
    let stem = if name.is_empty() { "Autosave".to_string() } else { format!("Autosave - {name}") };
    Some(g.home.join("saves").join(format!("{stem}.afmsave")))
}

/// Whether a world is being built, downloaded or played on beside the game
/// (two worlds in memory: no room for a third copy's pages).
fn job_running(g: &Game) -> bool {
    g.job.lock().ok().and_then(|j| j.as_ref().map(|j| !j.finished)).unwrap_or(false)
}

/// `autosave_tick`: starts an autosave when one is due, and answers where it
/// stands. Cheap when nothing is due.
pub fn tick(g: &mut Game) -> Reply {
    #[cfg(not(unix))]
    {
        let _ = g;
        return Ok(json!({ "state": "off" }));
    }
    #[cfg(unix)]
    {
        // A save that has run too long is ended.
        if let Phase::Saving { since, .. } = g.autosave.phase()
            && since.elapsed() > STUCK
        {
            g.autosave.cancel();
            g.autosave
                .set(Phase::Failed { at: Instant::now(), note: "The autosave took too long and was stopped.".into() });
        }
        let a = &g.autosave;
        let owed = a.activity != a.saved_at;
        let due = a.last.elapsed() >= a.every;
        let running = matches!(a.phase(), Phase::Saving { .. });
        if running || !due || !owed || job_running(g) || g.world.as_ref().is_none_or(|w| w.career.is_none()) {
            return Ok(g.autosave.status());
        }
        let Some(path) = path_for(g) else { return Ok(g.autosave.status()) };
        if let Err(note) = start(g, &path) {
            g.autosave.set(Phase::Failed { at: Instant::now(), note });
        }
        // The next one counts from now, so a failed one is not retried every tick.
        g.autosave.last = Instant::now();
        g.autosave.saved_at = g.autosave.activity;
        Ok(g.autosave.status())
    }
}

/// Puts the files that go beside the save in place, then forks. The parent
/// returns at once; the child writes the world and exits.
#[cfg(unix)]
fn start(g: &mut Game, path: &Path) -> Result<(), String> {
    // What goes beside the save is taken at the same moment as the world.
    let from = g.save_path.clone();
    if let Some(w) = g.world.as_mut() {
        world_log::on_save(w, path);
        tallies::on_save(w, path);
    }
    notebook::on_autosave(&g.home, from.as_deref(), path);
    depth_plan::on_autosave(&g.home, from.as_deref(), path);

    let w = g.world.as_ref().ok_or("No world is loaded.")?;
    // SAFETY: the child only writes the world to a file and leaves with
    // `_exit`, touching no lock another thread of the game could hold (the
    // allocator is fork-safe on Linux and on a Mac). A child that stuck
    // anyway is ended by the 20 minute limit.
    let pid = unsafe { sys::fork() };
    if pid < 0 {
        return Err(format!("The autosave could not start: {}", std::io::Error::last_os_error()));
    }
    if pid == 0 {
        child(w, path);
    }
    let token = {
        let mut s = g.autosave.shared.lock().map_err(|_| "The autosave state is locked.".to_string())?;
        s.token += 1;
        s.phase = Phase::Saving { pid, since: Instant::now(), partial: path.with_extension("partial") };
        s.token
    };
    let shared = g.autosave.shared.clone();
    let started = Instant::now();
    std::thread::spawn(move || {
        let mut status = 0;
        let ok = loop {
            // SAFETY: waits for the child this thread was started for.
            let r = unsafe { sys::waitpid(pid, &mut status, 0) };
            if r == pid {
                // Left by itself with code 0.
                break status & 0x7f == 0 && (status >> 8) & 0xff == 0;
            }
            if r < 0 && std::io::Error::last_os_error().raw_os_error() != Some(sys::EINTR) {
                break false;
            }
        };
        if let Ok(mut s) = shared.lock()
            && s.token == token
        {
            s.phase = if ok {
                Phase::Saved { at: Instant::now(), took: started.elapsed().as_secs_f64() }
            } else {
                Phase::Failed {
                    at: Instant::now(),
                    note: "The autosave did not finish. Your last one is still there.".into(),
                }
            };
        }
    });
    Ok(())
}

/// The forked copy: writes the world as it stands and leaves.
#[cfg(unix)]
fn child(w: &afm_engine::world::World, path: &Path) -> ! {
    // No sound: a message here could wait on a lock the game held.
    std::panic::set_hook(Box::new(|_| {}));
    // SAFETY: lowers this process's own priority.
    unsafe { sys::setpriority(0, 0, NICE) };
    let ok = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| crate::files::write_world(w, path).is_ok()))
        .unwrap_or(false);
    // SAFETY: ends this process without running the game's destructors.
    unsafe { sys::_exit(if ok { 0 } else { 1 }) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_play_makes_an_autosave_owed() {
        let mut a = Autosave::default();
        for c in ["autosave_tick", "autosave_status", "hello", "library", "job"] {
            a.touch(c);
        }
        assert_eq!(a.activity, a.saved_at);
        a.touch("continue");
        assert_ne!(a.activity, a.saved_at);
        a.saved_by_hand();
        assert_eq!(a.activity, a.saved_at);
    }

    #[test]
    fn the_status_says_where_it_stands() {
        let a = Autosave::default();
        assert_eq!(a.status()["state"], "idle");
        a.set(Phase::Saved { at: Instant::now(), took: 12.5 });
        assert_eq!(a.status()["state"], "saved");
        a.set(Phase::Failed { at: Instant::now(), note: "no".into() });
        assert_eq!(a.status()["note"], "no");
        a.set(Phase::Saving { pid: 1, since: Instant::now(), partial: PathBuf::new() });
        assert_eq!(a.status()["state"], "saving");
    }
}
