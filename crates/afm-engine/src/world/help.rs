//! Your in-game help (update 3, lane 8: H8-H15): the Help setting, the
//! guided first week at a new job, the calendar moments already explained
//! and the Handbook pages you've read. The game crate writes it
//! (`lane_handbook.rs`); nothing in the world reads it.

use crate::calendar::Date;

/// How much help the game gives (H15).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Level {
    /// The guided week, first-time notes and warnings.
    #[default]
    Full,
    /// First-time notes and warnings, no guided week.
    Hints,
    Off,
}

/// One task of the guided week: which one, the day it comes up, and
/// whether you've done it.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Task {
    pub key: String,
    pub day: Date,
    pub done: bool,
}

/// The guided week at one job.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Week {
    /// The team it is for (the game crate's team code).
    pub team: u32,
    pub start: Date,
    pub tasks: Vec<Task>,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Help {
    pub level: Level,
    /// The team the help last saw you at (the game crate's team code): a
    /// change is a new job.
    pub team: Option<u32>,
    /// Jobs started in this career (the first one counts).
    pub jobs: u16,
    /// The day you first came to the current job.
    pub since: Option<Date>,
    pub week: Option<Week>,
    /// Calendar moments already explained (H14), by key.
    pub firsts: Vec<String>,
    /// Handbook pages you've opened, by id.
    pub read: Vec<String>,
}
