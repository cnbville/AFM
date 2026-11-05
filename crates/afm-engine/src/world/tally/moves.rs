//! The moves tally (`moves/<season>.tally`, lane NB11, Data overhaul item
//! 38): every transfer portal entry since recorded history began, each
//! with how he did before the move and since. The store runs in the world
//! (and the save) like the classes; each season's file is the store as
//! that season closed, so a page reads one file for every move.
//!
//! The entries are the world's own portal records (`World::recruiting_log`);
//! the store adds what came of them: his season at the old school (the
//! stat book's games and snaps) and his seasons at the new one (the players
//! file's games, starts and snaps). Only watches.

use crate::calendar::Date;
use crate::positions::Position;
use crate::world::World;
use crate::world::data::Division;
use crate::world::records::Stat;

use super::classes::Career;
use super::players;

/// One portal entry and what came of it.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Move {
    pub id: u32,
    /// The offseason he entered (the season just played).
    pub year: i32,
    pub entered: Date,
    pub decided: Date,
    pub from: u32,
    pub from_division: Option<Division>,
    /// Where he landed (None: he found no school, or went back).
    pub to: Option<u32>,
    pub to_division: Option<Division>,
    pub returned: bool,
    /// Why he entered, in the record's words.
    pub why: String,
    /// His role the season before.
    pub role: String,
    pub pos: Option<Position>,
    pub stars: u8,
    /// His season at the old school: games and snaps.
    pub before_games: u16,
    pub before_snaps: u32,
    /// His seasons at the new school.
    pub after: Career,
}

impl Default for Move {
    fn default() -> Self {
        Move {
            id: 0,
            year: 0,
            entered: Date(0),
            decided: Date(0),
            from: 0,
            from_division: None,
            to: None,
            to_division: None,
            returned: false,
            why: String::new(),
            role: String::new(),
            pos: None,
            stars: 0,
            before_games: 0,
            before_snaps: 0,
            after: Career::default(),
        }
    }
}

/// Every move, running.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Store {
    pub moves: Vec<Move>,
    /// The world's portal records already taken in.
    pub seen: usize,
}

impl Store {
    /// A season closes: the portal records since the last close come in,
    /// and each mover's season at his new school is added.
    pub fn close(&mut self, w: &World, season: i32, open: &players::Open) {
        let log = &w.recruiting_log.portal;
        for r in log.iter().skip(self.seen) {
            let person = w.people.binary_search_by_key(&r.id, |p| p.player.id).ok().map(|k| &w.people[k]);
            let before = w
                .stats
                .career(r.id)
                .into_iter()
                .find(|s| s.year as i32 == r.year && s.program as usize == r.from_program);
            let to = if r.returned { None } else { r.to_program };
            self.moves.push(Move {
                id: r.id,
                year: r.year,
                entered: r.entered,
                decided: r.decided,
                from: r.from_program as u32,
                from_division: Some(r.from),
                to: to.map(|t| t as u32),
                to_division: if r.returned { None } else { r.to },
                returned: r.returned,
                why: r.why.to_string(),
                role: r.role.map_or(String::new(), |x| x.name().to_string()),
                pos: person.map(|p| p.player.position),
                stars: person.map_or(0, |p| p.stars),
                before_games: before.map_or(0, |s| s.games as u16),
                before_snaps: before.map_or(0, |s| (s.get(Stat::SnapsOff) + s.get(Stat::SnapsDef)) as u32),
                after: Career::default(),
            });
        }
        self.seen = log.len();
        for m in self.moves.iter_mut() {
            let Some(t) = m.to else { continue };
            if m.after.last >= season || season <= m.year {
                continue;
            }
            if let Some(s) = open.get(super::Level::College, m.id)
                && s.team == t
            {
                m.after.add(s, season);
            }
        }
    }
}

/// A move's key in the file: by the offseason, then the order it came in.
pub fn key(year: i32, k: usize) -> u64 {
    (year as u64) << 32 | k as u64
}
