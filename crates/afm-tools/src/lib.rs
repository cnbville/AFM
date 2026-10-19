#![recursion_limit = "512"]
//! Calibration and trial tooling for the AFM engine. Sees hidden potential
//! through the engine's `debug` feature, so it must never ship with the game.

/// A faster allocator for the trial tools: the match engine makes and drops
/// many small buffers every snap. Results don't depend on it.
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

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

pub mod dashboard;
pub mod raw;
pub mod stats;
pub mod world_report;
