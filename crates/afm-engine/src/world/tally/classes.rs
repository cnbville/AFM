//! The classes tally (`classes/<season>.tally`): every draft class and
//! every Division I recruiting class since recorded history began, each
//! member with what became of him so far (items 36 and 37). The store runs
//! in the world (and the save) and grows each season; each season's file
//! is the store as that season closed, so a page reads one file for every
//! class.
//!
//! Honors (All-America, Pro Bowls, All-Pro) are the world's own books
//! (`StatBook::honors`, `NflBook`): the store keeps what the books don't,
//! the games, starts and snaps at each level, the draft and the transfers.

use std::collections::HashMap;

use crate::positions::Position;
use crate::world::World;
use crate::world::data::Division;

use super::players;

/// Seasons, games, starts and snaps at one level.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Career {
    pub seasons: u8,
    pub games: u16,
    pub starts: u16,
    pub snaps: u32,
    /// The last season he played at the level.
    pub last: i32,
}

impl Career {
    pub(crate) fn add(&mut self, s: &players::Season, season: i32) {
        if s.games == 0 {
            return;
        }
        self.seasons += 1;
        self.games += s.games;
        self.starts += s.starts;
        self.snaps += s.snaps[0] + s.snaps[1];
        self.last = season;
    }
}

/// What became of a class member so far.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Outcome {
    pub college: Career,
    pub nfl: Career,
    /// (draft year, round, overall pick) when he was drafted.
    pub drafted: Option<(i32, u8, u16)>,
    /// College moves after his first program.
    pub transfers: u8,
    /// Through which season the outcome is counted.
    pub through: i32,
}

/// One draft pick.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Pick {
    pub year: i32,
    pub round: u8,
    pub pick: u16,
    pub club: u16,
    pub id: u32,
    pub pos: Option<Position>,
    /// His last college program.
    pub program: Option<u32>,
    pub out: Outcome,
}

/// One Division I signee from high school.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Signee {
    /// His high school class (the year he signed and enrolled).
    pub class: i32,
    pub id: u32,
    pub program: u32,
    pub division: Option<Division>,
    pub stars: u8,
    pub pos: Option<Position>,
    pub home_state: u16,
    pub out: Outcome,
}

/// The classes, running.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Store {
    pub picks: Vec<Pick>,
    pub signees: Vec<Signee>,
    /// Signee by id (rebuilt after a load).
    #[serde(skip)]
    by_id: HashMap<u32, usize>,
}

impl Store {
    fn index(&mut self) {
        if self.by_id.len() != self.signees.len() {
            self.by_id = self.signees.iter().enumerate().map(|(k, s)| (s.id, k)).collect();
        }
    }

    /// The draft's picks, as `tally::draft` builds them: a signee drafted
    /// is marked in his class too.
    pub fn draft(&mut self, picks: Vec<Pick>) {
        self.index();
        for p in picks {
            if let Some(&k) = self.by_id.get(&p.id) {
                self.signees[k].out.drafted = Some((p.year, p.round, p.pick));
            }
            self.picks.push(p);
        }
    }

    /// Whether a player signed with a Division I class (and his college
    /// numbers so far).
    pub fn signee(&mut self, id: u32) -> Option<&Signee> {
        self.index();
        self.by_id.get(&id).map(|&k| &self.signees[k])
    }

    /// A season closes: its Division I freshmen from high school join their
    /// class, and every member's season is added.
    pub fn close(&mut self, w: &World, season: i32, open: &players::Open) {
        self.index();
        for p in &w.people {
            if p.class_of != season || p.juco || self.by_id.contains_key(&p.player.id) {
                continue;
            }
            let Some(&first) = p.path.first() else { continue };
            let Some(prog) = w.programs.get(first) else { continue };
            if !matches!(prog.division, Division::FBS | Division::FCS) {
                continue;
            }
            self.by_id.insert(p.player.id, self.signees.len());
            self.signees.push(Signee {
                class: season,
                id: p.player.id,
                program: first as u32,
                division: Some(prog.division),
                stars: p.stars,
                pos: Some(p.player.position),
                home_state: p.home_state as u16,
                out: Outcome::default(),
            });
        }
        let path_of =
            |id: u32| w.people.binary_search_by_key(&id, |p| p.player.id).ok().map(|k| w.people[k].path.len());
        let update = |out: &mut Outcome, id: u32| {
            if out.through >= season {
                return;
            }
            if let Some(s) = open.get(super::Level::College, id) {
                out.college.add(s, season);
            }
            if let Some(s) = open.get(super::Level::Nfl, id) {
                out.nfl.add(s, season);
            }
            out.through = season;
        };
        let Store { picks, signees, by_id } = self;
        for s in signees.iter_mut() {
            update(&mut s.out, s.id);
            if let Some(n) = path_of(s.id) {
                s.out.transfers = s.out.transfers.max(n.saturating_sub(1) as u8);
            }
        }
        for p in picks.iter_mut() {
            update(&mut p.out, p.id);
            if let Some(&k) = by_id.get(&p.id) {
                p.out.college = signees[k].out.college;
                p.out.transfers = signees[k].out.transfers;
            }
        }
    }
}

/// A pick's key in the file (draft classes first).
pub fn pick_key(p: &Pick) -> u64 {
    (p.year as u64) << 16 | p.pick as u64
}

/// A signee's key in the file: after every pick.
pub fn signee_key(s: &Signee) -> u64 {
    1 << 62 | (s.class as u64) << 32 | s.id as u64
}

/// A record of the classes file.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Member {
    Pick(Pick),
    Signee(Signee),
}
