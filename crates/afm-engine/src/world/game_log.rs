//! Trial tooling: every college game's team totals and each player's season
//! totals, kept only when `WorldOptions::keep_game_log` is on. It only
//! watches the games, so a world with it on plays out exactly the same.
//! The game's own career stats record (section 3) is still to be designed.

use std::collections::HashMap;

use super::data::Division;
use crate::game::{GameStats, TeamStats};

/// One college game, as the box score's team totals.
#[derive(Clone, Debug)]
pub struct GameSummary {
    pub year: i32,
    pub division: Division,
    /// Program ids, home first.
    pub teams: [usize; 2],
    pub score: [i32; 2],
    pub stats: [TeamStats; 2],
    /// Kick and punt returns (count, yards), from the player lines.
    pub kr: [(u32, f64); 2],
    pub pr: [(u32, f64); 2],
    pub overtime: bool,
}

/// A player's season at one program: the headline counting stats.
#[derive(Clone, Copy, Debug, Default)]
pub struct PlayerSeason {
    pub program: usize,
    /// His listed position at the end of the season.
    pub pos: Option<crate::positions::Position>,
    pub games: u32,
    pub att: u32,
    pub cmp: u32,
    pub pass_yds: f32,
    pub pass_td: u32,
    pub int_thrown: u32,
    pub sacked: u32,
    pub rush: u32,
    pub rush_yds: f32,
    pub rush_td: u32,
    pub rec: u32,
    pub rec_yds: f32,
    pub rec_td: u32,
    pub tackles: u32,
    pub tfl: f32,
    pub sacks: f32,
    pub ints: u32,
    pub pbus: u32,
    pub pressures: u32,
    pub kr: u32,
    pub kr_yds: f32,
    pub fg_made: u32,
    pub fg_att: u32,
    pub grade_sum: f32,
    pub graded: u32,
}

#[derive(Clone, Debug, Default)]
pub struct GameLog {
    pub games: Vec<GameSummary>,
    /// (season, player id) to his season totals.
    pub players: HashMap<(i32, u32), PlayerSeason>,
    /// Player id to name, position when first seen, stars and class.
    pub names: HashMap<u32, (String, crate::positions::Position, u8, i32)>,
    /// (season, FBS program) to its starters' rating sum and count by position.
    /// (season, FBS program) to [rotation players' average, their count,
    /// starters' average college seasons, kicker, OC calling, DC calling].
    pub extras: HashMap<(i32, usize), [f64; 6]>,
    pub lineups:
        HashMap<(i32, usize), ([f64; crate::positions::POSITION_COUNT], [u32; crate::positions::POSITION_COUNT])>,
}

impl GameLog {
    pub fn add(&mut self, year: i32, division: Division, teams: [usize; 2], g: &GameStats) {
        let mut kr = [(0, 0.0); 2];
        let mut pr = [(0, 0.0); 2];
        for side in 0..2 {
            for (&id, l) in &g.players[side] {
                kr[side].0 += l.kr;
                kr[side].1 += l.kr_yds;
                pr[side].0 += l.pr;
                pr[side].1 += l.pr_yds;
                let s = self.players.entry((year, id)).or_default();
                s.program = teams[side];
                s.games += 1;
                s.att += l.att;
                s.cmp += l.cmp;
                s.pass_yds += l.pass_yds as f32;
                s.pass_td += l.pass_td;
                s.int_thrown += l.int;
                s.sacked += l.sacked;
                s.rush += l.rush;
                s.rush_yds += l.rush_yds as f32;
                s.rush_td += l.rush_td;
                s.rec += l.rec;
                s.rec_yds += l.rec_yds as f32;
                s.rec_td += l.rec_td;
                s.tackles += l.tackles + l.assists;
                s.tfl += l.tfl as f32;
                s.sacks += l.sacks as f32;
                s.ints += l.ints;
                s.pbus += l.pbus;
                s.pressures += l.pressures;
                s.kr += l.kr;
                s.kr_yds += l.kr_yds as f32;
                s.fg_made += l.fg_made.iter().sum::<u32>();
                s.fg_att += l.fg_att.iter().sum::<u32>();
                s.grade_sum += l.grade_sum as f32;
                s.graded += l.graded;
            }
        }
        self.games.push(GameSummary {
            year,
            division,
            teams,
            score: g.score,
            stats: g.team.clone(),
            kr,
            pr,
            overtime: g.overtime,
        });
    }
}
