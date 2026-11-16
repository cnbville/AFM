//! Ground, the offseason game (`docs/design/ground.md`). Lane G1a: the
//! areas and the map's data (`areas`). Lane G1b: the
//! people on your ground (`people`), one memory per person and holder
//! (`memory`) and standing on the four-level ladder (`standing`), with
//! day one read from the world's 30 years and your roots. Lane G7: people
//! in full (families, junior college and prep coaches, other college
//! staffs, trainers and 7v7 men with their kids, counselors with names),
//! the older stores reading through the one memory (`stores`), word that
//! travels (`word`), jobs that carry standing (`jobs`) and each person's
//! page (`card`).
//!
//! The store is sparse: only your seat's holders (your program, you and
//! your staff's men) keep memories, and only with people someone has dealt
//! with. Every other program's ground is read from its pipelines, its
//! footprint and its coaches' ties when asked (`standing::derived`), so
//! until lane G9 the world's recruiting is unchanged.

pub mod areas;
pub mod card;
pub mod jobs;
pub mod memory;
pub mod people;
pub mod standing;
pub mod stores;
pub mod word;

pub use memory::{Memory, Moment};
pub use people::{Holder, Who};
pub use standing::Level;

/// The ground store on `World.ground`.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Ground {
    /// One memory per person and holder (GR62), sorted by (who, holder).
    pub memories: Vec<Memory>,
    /// Your coach and program the day one was read for: a new job reads it
    /// again (your own ties go with you, the program's stay: GR29).
    pub(crate) seeded: Option<(u32, u32)>,
    /// The level each person last told you he stands at with your staff,
    /// so moving up is said once (GR26). Sorted by who.
    pub(crate) told: Vec<(Who, u8)>,
    /// Things already paid into the memory (a kid who started, a pulled
    /// offer, a promise judged), by key. Sorted.
    pub(crate) done: Vec<u64>,
    /// Word on its way to a man's ties and his area (GR62), by the day it
    /// lands.
    pub(crate) travel: Vec<word::Travel>,
    /// The new levels people told you lately: (day, who, level), for the
    /// morning map's shading (GR26). Kept 60 days.
    pub ups: Vec<(crate::calendar::Date, Who, u8)>,
    /// High school head jobs whose man moved on: (school, the man, the
    /// day), until the next man is in (`jobs`).
    pub(crate) vacated: Vec<(u32, Who, crate::calendar::Date)>,
    /// Test switch: the seat's ground stands still (the same-world check).
    #[serde(skip)]
    pub(crate) off: bool,
}

impl Ground {
    fn find(&self, who: Who, holder: Holder) -> Result<usize, usize> {
        self.memories.binary_search_by(|m| (m.who, m.holder).cmp(&(who, holder)))
    }

    pub fn get(&self, who: Who, holder: Holder) -> Option<&Memory> {
        self.find(who, holder).ok().map(|i| &self.memories[i])
    }

    pub fn get_mut(&mut self, who: Who, holder: Holder) -> Option<&mut Memory> {
        self.find(who, holder).ok().map(move |i| &mut self.memories[i])
    }

    /// The memory, made from `base` the first time.
    pub(crate) fn entry(&mut self, who: Who, holder: Holder, base: impl FnOnce() -> Memory) -> &mut Memory {
        let i = match self.find(who, holder) {
            Ok(i) => i,
            Err(i) => {
                self.memories.insert(i, base());
                i
            }
        };
        &mut self.memories[i]
    }

    /// Every memory with one person, any holder.
    pub fn with(&self, who: Who) -> &[Memory] {
        let a = self.memories.partition_point(|m| m.who < who);
        let b = self.memories.partition_point(|m| m.who <= who);
        &self.memories[a..b]
    }

    pub(crate) fn told(&self, who: Who) -> u8 {
        self.told.binary_search_by(|t| t.0.cmp(&who)).map_or(0, |i| self.told[i].1)
    }

    pub(crate) fn set_told(&mut self, who: Who, level: u8) {
        match self.told.binary_search_by(|t| t.0.cmp(&who)) {
            Ok(i) => self.told[i].1 = level,
            Err(i) => self.told.insert(i, (who, level)),
        }
    }

    /// Marks a key paid; false if it already was.
    pub(crate) fn once(&mut self, key: u64) -> bool {
        match self.done.binary_search(&key) {
            Ok(_) => false,
            Err(i) => {
                self.done.insert(i, key);
                true
            }
        }
    }
}

/// A key from parts (FNV-1a over the words), the same every run.
pub(crate) fn key(parts: &[u64]) -> u64 {
    let mut h = 0xCBF2_9CE4_8422_2325u64;
    for p in parts {
        for b in p.to_le_bytes() {
            h = (h ^ b as u64).wrapping_mul(0x0100_0000_01B3);
        }
    }
    h
}

/// A number in [0, 1) from the world's seed and a key: people are made the
/// same every time, and nothing here draws on the world's random stream.
pub(crate) fn unit(seed: u32, parts: &[u64]) -> f64 {
    let mut z = key(parts) ^ (seed as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64
}

/// Your seat's ground each day: standing (lane G1b), then word on its way,
/// moves on the coaching carousel, and the older stores folded in.
pub(crate) fn day(w: &mut crate::world::World, d: crate::calendar::Date) {
    standing::day(w, d);
    if w.ground.off || standing::seat_program(w).is_none() {
        return;
    }
    word::day(w, d);
    jobs::day(w, d);
    stores::sync(w);
    let cut = d.add(-60);
    w.ground.ups.retain(|u| u.0 >= cut);
}

/// A file from before lane G7: the high school coach's credit with each
/// college staff moves into the memory.
pub(crate) fn after_load(w: &mut crate::world::World) {
    stores::after_load(w);
}
