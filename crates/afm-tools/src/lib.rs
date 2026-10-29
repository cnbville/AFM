#![recursion_limit = "512"]
//! Calibration and trial tooling for the AFM engine. Sees hidden potential
//! through the engine's `debug` feature, so it must never ship with the game.

/// A faster allocator for the trial tools: the match engine makes and drops
/// many small buffers every snap. Results don't depend on it. With
/// `count_heap` on it also keeps the live heap's size, so `memory_check`
/// can weigh each of the world's stores by dropping it.
#[global_allocator]
static ALLOC: Counting = Counting;

struct Counting;

static COUNTING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static LIVE: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

unsafe impl std::alloc::GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: std::alloc::Layout) -> *mut u8 {
        if COUNTING.load(std::sync::atomic::Ordering::Relaxed) {
            LIVE.fetch_add(l.size() as isize, std::sync::atomic::Ordering::Relaxed);
        }
        unsafe { mimalloc::MiMalloc.alloc(l) }
    }
    unsafe fn alloc_zeroed(&self, l: std::alloc::Layout) -> *mut u8 {
        if COUNTING.load(std::sync::atomic::Ordering::Relaxed) {
            LIVE.fetch_add(l.size() as isize, std::sync::atomic::Ordering::Relaxed);
        }
        unsafe { mimalloc::MiMalloc.alloc_zeroed(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: std::alloc::Layout) {
        if COUNTING.load(std::sync::atomic::Ordering::Relaxed) {
            LIVE.fetch_sub(l.size() as isize, std::sync::atomic::Ordering::Relaxed);
        }
        unsafe { mimalloc::MiMalloc.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: std::alloc::Layout, new: usize) -> *mut u8 {
        if COUNTING.load(std::sync::atomic::Ordering::Relaxed) {
            LIVE.fetch_add(new as isize - l.size() as isize, std::sync::atomic::Ordering::Relaxed);
        }
        unsafe { mimalloc::MiMalloc.realloc(p, l, new) }
    }
}

/// Starts counting the live heap (from zero: call it before anything is
/// built).
pub fn count_heap() {
    COUNTING.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// The live heap counted since `count_heap`, in GB.
pub fn heap_gb() -> f64 {
    LIVE.load(std::sync::atomic::Ordering::Relaxed) as f64 / 1_073_741_824.0
}

/// Resident memory now and at its peak, in GB (trial 8 finding 11: the
/// world's budget is set from these). Read from `/proc` on Linux, where the
/// trials run; elsewhere `ps` gives the current figure only.
pub fn memory_gb() -> (Option<f64>, Option<f64>) {
    let gb = |kb: f64| kb / 1_048_576.0;
    if let Ok(s) = std::fs::read_to_string("/proc/self/status") {
        let read = |key: &str| {
            s.lines()
                .find(|l| l.starts_with(key))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|v| v.parse::<f64>().ok())
                .map(gb)
        };
        return (read("VmRSS:"), read("VmHWM:"));
    }
    let pid = std::process::id().to_string();
    let out = std::process::Command::new("ps").args(["-o", "rss=", "-p", &pid]).output().ok();
    let now = out.and_then(|o| String::from_utf8(o.stdout).ok()).and_then(|s| s.trim().parse::<f64>().ok()).map(gb);
    (now, None)
}

/// `memory_gb` as text: "5.21 GB (peak 7.77 GB)".
pub fn memory_text() -> String {
    match memory_gb() {
        (Some(now), Some(peak)) => format!("{now:.2} GB (peak {peak:.2} GB)"),
        (Some(now), None) => format!("{now:.2} GB"),
        _ => "not measured".into(),
    }
}

/// A hash of what the world's years produced: every person's season rows,
/// ratings, stage and school, every alumnus, coach and news line.
pub fn fingerprint(w: &afm_engine::world::World) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for p in &w.people {
        p.name.hash(&mut h);
        format!("{:?}", p.player.history).hash(&mut h);
        format!("{:?}{:?}{:?}{}", p.stage, p.college, p.player.position, p.hs).hash(&mut h);
        format!("{:?}", p.player.attributes).hash(&mut h);
    }
    for a in &w.alumni {
        a.name.hash(&mut h);
    }
    for c in &w.coaches {
        format!("{}{:?}", c.name, c.history.len()).hash(&mut h);
    }
    for n in &w.news {
        n.text.hash(&mut h);
    }
    h.finish()
}

pub mod dashboard;
pub mod raw;
pub mod stats;
pub mod world_report;
