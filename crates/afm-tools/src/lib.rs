#![recursion_limit = "512"]
//! Calibration and trial tooling for the AFM engine. Sees hidden potential
//! through the engine's `debug` feature, so it must never ship with the game.

/// A faster allocator for the trial tools: the match engine makes and drops
/// many small buffers every snap. Results don't depend on it.
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

pub mod dashboard;
pub mod raw;
pub mod stats;
pub mod world_report;
