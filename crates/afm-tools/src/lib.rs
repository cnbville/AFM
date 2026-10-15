#![recursion_limit = "512"]
//! Calibration and trial tooling for the AFM engine. Sees hidden potential
//! through the engine's `debug` feature, so it must never ship with the game.

pub mod dashboard;
pub mod raw;
pub mod stats;
pub mod world_report;
