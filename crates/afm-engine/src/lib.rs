//! AFM engine: the simulation behind American Football Manager.
//!
//! Pure logic, no UI. A desktop front end calls into this library.
//!
//! Hidden potential: a player's true ceiling never leaves the engine. The
//! player-facing API offers only scouted estimates (`scouting::scout_player`
//! ranges and stars, and the vaguer `scouting::potential_outlook`). Exact
//! potential is available to trial and calibration tooling through the
//! `debug` module, which only exists with the `debug` cargo feature.

// Index loops over attributes and teams read most clearly as they are.
#![allow(clippy::needless_range_loop, clippy::should_implement_trait)]

pub mod athlete;
pub mod attributes;
pub mod body;
pub mod calendar;
pub mod career;
pub mod development;
pub mod game;
pub mod league;
pub mod level_rating;
pub mod links;
pub mod player;
pub mod positions;
pub mod rng;
pub mod save;
pub mod scouting;
pub mod spots;
pub mod tendencies;
pub mod text;
pub mod world;

#[cfg(feature = "debug")]
pub mod debug;

#[cfg(test)]
mod tests;

pub use attributes::{Attr, Attributes};
pub use career::{CareerRecord, CohortOptions, CohortResult, simulate_cohort};
pub use player::{Level, Player, PlayingTime};
pub use positions::Position;
pub use rng::Rng;
