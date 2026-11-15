//! Every team's testing records (Cain 2026-11-15): a strength staff tests
//! its players on the level's testing days (TR5), and the records stay with
//! the team, so a coach who takes over a team finds the seasons of numbers
//! the staff before him kept on every player there.
//!
//! The book lives on the player (`Person.tests`), tagged with the team that
//! tested him: when he moves on, the first testing day at his new team
//! drops his old team's rows, and nothing reads rows of another team
//! meanwhile. It keeps his last `KEEP` days at his team. Rows are packed to
//! 20 bytes and written to world files as bytes. Generic on purpose: what a
//! row holds comes from `seat::test_row`, whatever drives the drills.

use super::rules::Tier;
use super::seat::{TestDay, TestRow};
use crate::calendar::{Date, Level as Cal};
use crate::world::{Stage, World};

/// The testing days kept per player at his team: five college years (three
/// a year), seven or more NFL or high school years (two a year).
pub const KEEP: usize = 15;

/// A team, packed: the level in the top two bits, the index below.
pub fn team_key(tier: Tier, index: usize) -> u32 {
    let t = match tier {
        Tier::High => 0u32,
        Tier::College => 1,
        Tier::Pro => 2,
    };
    t << 30 | (index as u32 & 0x3FFF_FFFF)
}

/// The team a player trains with now (high school or prep school, college,
/// an NFL club), or None.
pub fn team_of(p: &crate::world::Person) -> Option<u32> {
    match p.stage {
        Stage::HighSchool | Stage::PrepSchool => Some(team_key(Tier::High, p.hs)),
        Stage::College => p.college.map(|c| team_key(Tier::College, c)),
        Stage::Pro => p.pro.as_ref().and_then(|c| c.team).map(|t| team_key(Tier::Pro, t)),
        _ => None,
    }
}

/// One testing day in a player's book, packed (20 bytes): the lifts in
/// fives of pounds, the times in hundredths, the vertical in half inches
/// with the baseline flag in its top bit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    pub date: Date,
    pub team: u32,
    forty: u16,
    shuttle: u16,
    weight: u16,
    split: u8,
    bench: u8,
    squat: u8,
    clean: u8,
    reps: u8,
    vert: u8,
}

const BASE_BIT: u8 = 0x80;

impl Entry {
    pub fn new(date: Date, team: u32, r: &TestRow, baseline: bool) -> Entry {
        let five = |x: u16| (x / 5).min(255) as u8;
        Entry {
            date,
            team,
            forty: r.forty,
            shuttle: r.shuttle,
            weight: r.weight,
            split: r.split.min(255) as u8,
            bench: five(r.bench),
            squat: five(r.squat),
            clean: five(r.clean),
            reps: r.reps,
            vert: r.vertical.min(0x7F) | if baseline { BASE_BIT } else { 0 },
        }
    }

    /// Your staff's baseline in its first week, not a testing day.
    pub fn baseline(&self) -> bool {
        self.vert & BASE_BIT != 0
    }

    pub fn row(&self, id: u32) -> TestRow {
        TestRow {
            id,
            bench: self.bench as u16 * 5,
            reps: self.reps,
            squat: self.squat as u16 * 5,
            clean: self.clean as u16 * 5,
            forty: self.forty,
            split: self.split as u16,
            shuttle: self.shuttle,
            vertical: self.vert & !BASE_BIT,
            weight: self.weight,
        }
    }

    fn bytes(&self) -> [u8; 20] {
        let mut b = [0u8; 20];
        b[0..4].copy_from_slice(&self.date.0.to_le_bytes());
        b[4..8].copy_from_slice(&self.team.to_le_bytes());
        b[8..10].copy_from_slice(&self.forty.to_le_bytes());
        b[10..12].copy_from_slice(&self.shuttle.to_le_bytes());
        b[12..14].copy_from_slice(&self.weight.to_le_bytes());
        b[14..20].copy_from_slice(&[self.split, self.bench, self.squat, self.clean, self.reps, self.vert]);
        b
    }

    fn from_bytes(b: &[u8; 20]) -> Entry {
        let u16_at = |k: usize| u16::from_le_bytes([b[k], b[k + 1]]);
        Entry {
            date: Date(i32::from_le_bytes([b[0], b[1], b[2], b[3]])),
            team: u32::from_le_bytes([b[4], b[5], b[6], b[7]]),
            forty: u16_at(8),
            shuttle: u16_at(10),
            weight: u16_at(12),
            split: b[14],
            bench: b[15],
            squat: b[16],
            clean: b[17],
            reps: b[18],
            vert: b[19],
        }
    }
}

/// A player's testing days at his team, oldest first.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TestBook(Box<[Entry]>);

impl TestBook {
    /// Adds a day: rows from another team go (the records stay with that
    /// team), then the oldest past `KEEP`.
    pub fn record(&mut self, e: Entry) {
        let mut v: Vec<Entry> = self.0.iter().copied().filter(|x| x.team == e.team && x.date != e.date).collect();
        v.push(e);
        v.sort_by_key(|x| x.date);
        if v.len() > KEEP {
            v.drain(..v.len() - KEEP);
        }
        self.0 = v.into_boxed_slice();
    }

    /// His days at this team, oldest first.
    pub fn at(&self, team: u32) -> impl Iterator<Item = &Entry> {
        self.0.iter().filter(move |x| x.team == team)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl serde::Serialize for TestBook {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let b: Vec<u8> = self.0.iter().flat_map(|e| e.bytes()).collect();
        s.serialize_bytes(&b)
    }
}

impl<'de> serde::Deserialize<'de> for TestBook {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> serde::de::Visitor<'de> for V {
            type Value = TestBook;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("testing days as bytes")
            }
            fn visit_bytes<E: serde::de::Error>(self, b: &[u8]) -> Result<TestBook, E> {
                if !b.len().is_multiple_of(20) {
                    return Err(E::custom("testing days: not a whole number of rows"));
                }
                Ok(TestBook(b.as_chunks::<20>().0.iter().map(Entry::from_bytes).collect()))
            }
            fn visit_byte_buf<E: serde::de::Error>(self, b: Vec<u8>) -> Result<TestBook, E> {
                self.visit_bytes(&b)
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut a: A) -> Result<TestBook, A::Error> {
                let mut b = Vec::new();
                while let Some(x) = a.next_element::<u8>()? {
                    b.push(x);
                }
                self.visit_bytes(&b)
            }
        }
        d.deserialize_bytes(V)
    }
}

/// Your team (its key) and its players now, as `seat::seat_team` reads them.
pub fn seat_roster(w: &World) -> Option<(u32, Vec<usize>)> {
    let seat = w.seat?;
    let (key, roster) = match seat.level {
        Cal::Nfl => {
            let t = seat.nfl_team?;
            (team_key(Tier::Pro, t), w.clubs.get(t)?.roster.clone())
        }
        Cal::HighSchool => {
            let h = seat.program?;
            (team_key(Tier::High, h), w.high_schools.get(h)?.roster.clone())
        }
        _ => {
            let c = seat.program?;
            (team_key(Tier::College, c), w.programs.get(c)?.roster.clone())
        }
    };
    let roster = roster
        .into_iter()
        .filter(|&i| {
            w.people.get(i).is_some_and(|p| matches!(p.stage, Stage::HighSchool | Stage::College | Stage::Pro))
        })
        .collect();
    Some((key, roster))
}

/// A team's testing days for these players, oldest first: every day any of
/// them tested there, the rows of those who did.
pub fn team_days(w: &World, team: u32, roster: &[usize]) -> Vec<TestDay> {
    let mut days: std::collections::BTreeMap<Date, TestDay> = std::collections::BTreeMap::new();
    for &i in roster {
        let p = &w.people[i];
        for e in p.tests.at(team) {
            let d = days.entry(e.date).or_insert_with(|| TestDay { date: e.date, rows: Vec::new(), baseline: false });
            d.baseline |= e.baseline();
            d.rows.push(e.row(p.player.id));
        }
    }
    days.into_values().collect()
}

/// Your team's testing days: those the staff before you kept and your own.
pub fn seat_days(w: &World) -> Vec<TestDay> {
    seat_roster(w).map_or_else(Vec::new, |(k, r)| team_days(w, k, &r))
}

/// One of your players' testing days at your team, oldest first (date, his
/// numbers, a baseline).
pub fn seat_rows(w: &World, person: usize) -> Vec<(Date, TestRow, bool)> {
    let Some((key, _)) = seat_roster(w) else { return Vec::new() };
    let p = &w.people[person];
    p.tests.at(key).map(|e| (e.date, e.row(p.player.id), e.baseline())).collect()
}

/// The day your staff first tested at your team (its baseline), or None
/// before it has: days before it are the staff before yours.
pub fn seat_since(w: &World) -> Option<Date> {
    let (key, _) = seat_roster(w)?;
    w.training.tested.filter(|t| t.0 == key).map(|t| t.1)
}
