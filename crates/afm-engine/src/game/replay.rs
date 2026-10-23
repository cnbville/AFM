//! Replaying a game from its seed (match engine 9.2, screens topic 12): a
//! game you can watch back is kept as everything that decided it (both
//! teams as they dressed, the rules, the weather, the seed, and your
//! answers when you played it live), never as its tracking. Watching plays
//! it again with frames on; the frames are output only (`pose.rs`), so the
//! replay is the same game, snap for snap.

use super::flow::{GameResult, GameSetup, play_game, play_game_ctl};
use super::garbage::Context;
use super::live::{Control, StopMode};
use super::roster::GameTeam;
use super::rules::Rules;
use super::sim::Conditions;

/// One game as it was set up, owned, so it can be played again.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ReplaySetup {
    pub home: GameTeam,
    pub away: GameTeam,
    pub rules: Rules,
    pub cond: Conditions,
    pub seed: u64,
    pub neutral: bool,
    pub playoff: bool,
    pub two_way_margin: [f64; 2],
    pub context: [Context; 2],
    pub wear: [f64; 2],
    pub cold_unused: [f64; 2],
    /// For the views only: the local kickoff hour (0 when unknown) and a
    /// roof over the field.
    pub kickoff: f32,
    pub indoor: bool,
    /// Your settings and every answer, for a game you played live (section
    /// 9): the game is its seed plus these.
    #[serde(default)]
    pub live: Option<Control>,
}

impl ReplaySetup {
    pub fn of(s: &GameSetup, live: Option<Control>) -> ReplaySetup {
        ReplaySetup {
            home: s.home.clone(),
            away: s.away.clone(),
            rules: s.rules,
            cond: s.cond,
            seed: s.seed,
            neutral: s.neutral,
            playoff: s.playoff,
            two_way_margin: s.two_way_margin,
            context: s.context,
            wear: s.wear,
            cold_unused: s.cold_unused,
            kickoff: s.kickoff,
            indoor: s.indoor,
            live,
        }
    }

    /// The game set up again, borrowing these teams.
    pub fn setup(&self) -> GameSetup<'_> {
        GameSetup {
            home: &self.home,
            away: &self.away,
            rules: self.rules,
            cond: self.cond,
            seed: self.seed,
            neutral: self.neutral,
            playoff: self.playoff,
            record: false,
            two_way_margin: self.two_way_margin,
            context: self.context,
            wear: self.wear,
            cold_unused: self.cold_unused,
            keep: false,
            kickoff: self.kickoff,
            indoor: self.indoor,
        }
    }

    /// Plays it again, with every snap's frames when `record` is on.
    pub fn play(&self, record: bool) -> GameResult {
        let mut s = self.setup();
        s.record = record;
        match &self.live {
            // Your answers at the points they answered; nothing stops now.
            Some(c) => {
                let mut c = c.clone();
                c.settings.stop = StopMode::Off;
                c.take_next = false;
                c.record_from = 0;
                c.pending = None;
                c.pause_at = None;
                c.asked = 0;
                play_game_ctl(&s, c).0
            }
            None => play_game(&s),
        }
    }

    /// Packed small for keeping (MessagePack through zstd): about a tenth
    /// of the teams' size in memory.
    pub fn pack(&self) -> Vec<u8> {
        let raw = rmp_serde::to_vec_named(self).unwrap_or_default();
        zstd::encode_all(&raw[..], 9).unwrap_or_default()
    }

    pub fn unpack(packed: &[u8]) -> Option<ReplaySetup> {
        let raw = zstd::decode_all(packed).ok()?;
        rmp_serde::from_slice(&raw).ok()
    }
}
