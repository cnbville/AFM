//! The prospects tally (`prospects/<season>.tally`, lane NB11, Data
//! overhaul items 35 and 37): what a recruiting analyst keeps on high
//! school prospects. Two kinds of record:
//! - `Prospect`: one a kid a season: each high school game he played on the
//!   match engine, with his line (MaxPreps' game log), and each time the
//!   services ranked his class that season (his stars, national, position
//!   and state rank and composite rating at the time);
//! - `Signed`: one a Division I signee, written the season his class joins
//!   the classes file: his last ranking before he enrolled, so class
//!   rankings can be worked out on the composite as the services do.
//!
//! Who is kept: a high schooler with stars from the services or a college
//! offer on the day of the game (proposed, not confirmed: the kids
//! recruiting analysts follow; every high schooler would be about ten
//! times the size). The services' lists are kept for every ranked kid.
//!
//! Only watches: it reads the game's lines and the services' order.

use std::collections::BTreeMap;

use crate::calendar::Date;
use crate::positions::Position;
use crate::world::hs_year::HS_STAT_COUNT;
use crate::world::{Stage, World};

/// One high school game's line.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct HsGame {
    pub date: Date,
    /// The game's key in the season's games file (None: the game wasn't
    /// counted there).
    pub key: Option<u64>,
    pub school: u32,
    pub opp: u32,
    /// 0 home, 1 away, 2 a neutral site.
    pub site: u8,
    /// His school's points, then the opponent's.
    pub score: [u16; 2],
    /// Whether he took a snap.
    pub played: bool,
    /// His counts, sparse: (`HsStat` code, count; sacks and tackles for
    /// loss in halves, as `HsLine`).
    pub s: Vec<(u8, i16)>,
}

impl Default for HsGame {
    fn default() -> Self {
        HsGame { date: Date(0), key: None, school: 0, opp: 0, site: 0, score: [0; 2], played: false, s: Vec::new() }
    }
}

impl HsGame {
    /// A count by `HsStat` code (halves turned to whole sacks and tackles
    /// for loss).
    pub fn get(&self, code: u8) -> f64 {
        let v = self.s.iter().find(|x| x.0 == code).map_or(0.0, |x| x.1 as f64);
        if code == crate::world::hs_year::HsStat::Tfl as u8 || code == crate::world::hs_year::HsStat::Sacks as u8 {
            v / 2.0
        } else {
            v
        }
    }
}

/// One services' update of his class.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Rank {
    pub date: Date,
    pub stars: u8,
    /// National, at his position and in his home state (1 best).
    pub rank: u32,
    pub pos_rank: u16,
    pub state_rank: u16,
    /// The composite rating, in ten-thousandths (.9834 is 9834).
    pub rating: u16,
}

/// A kid's season.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Prospect {
    pub id: u32,
    pub class_of: i32,
    pub pos: Option<Position>,
    pub home_state: u16,
    pub games: Vec<HsGame>,
    pub ranks: Vec<Rank>,
}

/// A Division I signee's last ranking before he enrolled.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Signed {
    pub id: u32,
    pub class: i32,
    pub program: u32,
    pub pos: Option<Position>,
    pub home_state: u16,
    pub stars: u8,
    pub last: Option<Rank>,
}

/// A record of the prospects file.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Row {
    Prospect(Prospect),
    Signed(Signed),
}

/// A kid's key: his id.
pub fn key(id: u32) -> u64 {
    id as u64
}

/// A signee's key: after every kid, by class.
pub fn signed_key(class: i32, id: u32) -> u64 {
    1 << 62 | (class as u64) << 32 | id as u64
}

/// The season being counted.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Open {
    pub rows: BTreeMap<u32, Prospect>,
    /// Each ranked kid's latest ranking, carried from season to season
    /// until his class has signed and gone.
    pub last: BTreeMap<u32, Rank>,
}

/// Whether a high schooler is followed: stars or an offer.
pub fn followed(p: &crate::world::Person) -> bool {
    matches!(p.stage, Stage::HighSchool | Stage::PrepSchool)
        && (p.stars > 0 || !p.cal.offers.is_empty() || !p.cal.early_offers.is_empty())
}

impl Open {
    fn row(&mut self, p: &crate::world::Person) -> &mut Prospect {
        self.rows.entry(p.player.id).or_insert_with(|| Prospect {
            id: p.player.id,
            class_of: p.class_of,
            pos: Some(p.player.position),
            home_state: p.home_state as u16,
            ..Default::default()
        })
    }

    /// A game's lines: each followed kid's.
    #[allow(clippy::too_many_arguments)]
    pub fn game(
        &mut self,
        w: &World,
        date: Date,
        key: Option<u64>,
        schools: [usize; 2],
        lines: &[Vec<(usize, [i32; HS_STAT_COUNT], bool)>; 2],
        score: [u32; 2],
        neutral: bool,
    ) {
        for side in 0..2 {
            for &(i, c, on) in &lines[side] {
                let Some(p) = w.people.get(i) else { continue };
                if !followed(p) {
                    continue;
                }
                let s: Vec<(u8, i16)> = c
                    .iter()
                    .enumerate()
                    .filter(|x| *x.1 != 0)
                    .map(|(k, &v)| (k as u8, v.clamp(i16::MIN as i32, i16::MAX as i32) as i16))
                    .collect();
                let g = HsGame {
                    date,
                    key,
                    school: schools[side] as u32,
                    opp: schools[1 - side] as u32,
                    site: if neutral { 2 } else { side as u8 },
                    score: [score[side].min(u16::MAX as u32) as u16, score[1 - side].min(u16::MAX as u32) as u16],
                    played: on,
                    s,
                };
                self.row(p).games.push(g);
            }
        }
    }

    /// The services' order for a class on a day (`services::record`).
    pub fn ranks(
        &mut self,
        w: &World,
        class_of: i32,
        d: Date,
        ids: &[u32],
        ratings: &std::collections::HashMap<u32, (u32, f64)>,
    ) {
        let mut by_pos: BTreeMap<Position, u16> = BTreeMap::new();
        let mut by_state: BTreeMap<usize, u16> = BTreeMap::new();
        for (k, &id) in ids.iter().enumerate() {
            let Ok(i) = w.people.binary_search_by_key(&id, |p| p.player.id) else { continue };
            let p = &w.people[i];
            let pr = by_pos.entry(p.player.position).or_insert(0);
            *pr += 1;
            let sr = by_state.entry(p.home_state).or_insert(0);
            *sr += 1;
            let rating = ratings.get(&id).map_or(0.0, |r| r.1);
            let r = Rank {
                date: d,
                stars: p.stars,
                rank: k as u32 + 1,
                pos_rank: *pr,
                state_rank: *sr,
                rating: (rating * 10_000.0).round().clamp(0.0, u16::MAX as f64) as u16,
            };
            let row = self.row(p);
            row.class_of = class_of;
            match row.ranks.last_mut() {
                Some(x) if x.date == d => *x = r,
                _ => row.ranks.push(r),
            }
            self.last.insert(id, r);
        }
    }

    /// The season closes: its records, the signees of the class that
    /// joined the classes file this season with their last ranking, and the
    /// open store starts over (the latest rankings of classes still to come
    /// are carried).
    pub fn close(&mut self, w: &World, season: i32, signees: &[super::classes::Signee]) -> Vec<(u64, Row)> {
        let mut out: Vec<(u64, Row)> =
            std::mem::take(&mut self.rows).into_values().map(|p| (key(p.id), Row::Prospect(p))).collect();
        for s in signees.iter().filter(|s| s.class == season) {
            out.push((
                signed_key(s.class, s.id),
                Row::Signed(Signed {
                    id: s.id,
                    class: s.class,
                    program: s.program,
                    pos: s.pos,
                    home_state: s.home_state,
                    stars: s.stars,
                    last: self.last.get(&s.id).copied(),
                }),
            ));
        }
        out.sort_by_key(|r| r.0);
        // A class that signed this season or before is done.
        let done: Vec<u32> = self
            .last
            .keys()
            .copied()
            .filter(|&id| {
                w.people.binary_search_by_key(&id, |p| p.player.id).map_or(true, |i| w.people[i].class_of <= season)
            })
            .collect();
        for id in done {
            self.last.remove(&id);
        }
        out
    }
}
