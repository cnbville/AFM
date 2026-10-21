//! Section 3: the match engine and the stats database.
//!
//! Every game at every level runs on this one engine (topic 0): 22 tracked
//! players, battles decided by attributes, real rules per level, coaches
//! calling plays, and a full play log from which every stat is computed.
//! See `docs/design/match-engine.md` and `docs/match-engine.md`.

pub mod calling;
pub mod designer;
pub mod experiment;
pub mod flow;
pub mod garbage;
pub mod geom;
pub mod kicking;
pub mod playbook;
pub mod pose;
pub mod replay;
pub mod roster;
pub mod rules;
pub mod scrimmage;
pub mod sim;
pub mod stats;
pub mod synthetic;
pub mod tables;

pub use calling::Film;
pub use flow::{GameResult, GameSetup, Venue, play_game, quick_game, quick_game_in, quick_game_with};
pub use roster::{Caller, Front, GamePlayer, GameTeam, OffScheme, Shell, Staff};
pub use rules::Rules;
pub use replay::ReplaySetup;
pub use sim::Conditions;
pub use stats::{GameStats, PlayRecord, PlayType, StatLine, TeamStats};
