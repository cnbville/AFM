//! Live talks v3, lane N5d: game day and the room
//! (`docs/design/live-talks-persona.md`, sections 6.1, 6.12 and 16.3).
//! The game's side of the talks that aren't one man across a desk:
//!
//! - `speech`: your pregame and halftime speech, read as hits on each man
//!   who hears it. Each man's own part of your words tilts his landing
//!   toward a word to him alone (`game::talk::Tilt`), so `game/talk.rs`'s
//!   sizes stay and no typed line beats the best chip.
//! - `group`: the team talk, the first team meeting and the leadership
//!   council played as group talks on lane N4's conversation: every man in
//!   the room hears every line, a captain answers, the council tells you
//!   what it sees.
//! - `official`: asking an official why, arguing, and paying for it with a
//!   flag on your sideline.
//! - `decide`: a decision you told a player in a talk, made real once you
//!   confirm it (benched, starting, suspended).
//!
//! Every size here is proposed, not confirmed.

pub mod decide;
pub mod group;
pub mod official;
pub mod speech;
