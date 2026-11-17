//! Capital, the facilities layer (`docs/design/capital.md`).
//!
//! Lane C1, the donors: every college program's 15 to 30 named donors
//! (fewer below the Power 4) with wealth, age, loyalty, wants, a circle and
//! a memory of each coach (`donors`); the development office's wealth read
//! as a range over a hidden number (`screen`, CP3); a broken promise
//! spreading through the donor's circle (`donors::word`, CP4); pledges tied
//! to the coach who closed them (`pledges`, CP5); ex-pros who join their
//! school's donors at their second contract (`expros`, CP6); and donors
//! with ties to two schools (`Donor::also`, CP28).
//!
//! Lane C2, the rooms (`rooms`): thirteen rooms a program with levels 1-5
//! that share out the facilities number and each unlock a tool (CP17-CP19),
//! aging read against the conference (CP22, CP23), renovations and new
//! buildings (CP18, CP21) and the build year in temporary space (CP26).
//!
//! The store only keeps people and their memory: nothing else in the world
//! reads it yet (lanes C3 and C4 do), so every result of a world stays the
//! same per seed. Every draw here is keyed on the world's seed and the
//! donor, never on a shared stream. Sizes are Claude's (proposed, not
//! confirmed) unless the spec gives them.

pub mod donors;
pub mod expros;
pub mod pledges;
pub mod rooms;
pub mod screen;

use crate::calendar::Date;
use crate::world::World;

pub use donors::{Bond, Circle, Donor, Estate, Money};
pub use pledges::Pledge;

/// The Capital store on `World.capital`.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Capital {
    /// Every college donor, living and recently dead, by id (sorted).
    pub donors: Vec<Donor>,
    /// The circles donors move in (golf friends, a church table, the tech
    /// founders' dinner), by id (sorted).
    pub circles: Vec<Circle>,
    /// Pledges to buildings, paid over years (lane C3 makes and pays them).
    pub pledges: Vec<Pledge>,
    /// Estate gifts left by donors who died, for lane C3 to pay.
    pub estates: Vec<Estate>,
    /// Pros already weighed as ex-pro donors (player ids, sorted): one look
    /// at his first big deal after his rookie contract.
    pub(crate) pros: Vec<u32>,
    /// The next donor, circle and pledge id.
    pub(crate) next: u32,
    /// The last year the donors were brought up to date.
    pub(crate) year: i32,
    /// Test switch: the store stands still (the same-world check).
    #[serde(skip)]
    pub(crate) off: bool,
}

impl Capital {
    pub(crate) fn id(&mut self) -> u32 {
        self.next += 1;
        self.next
    }
    pub fn donor(&self, id: u32) -> Option<&Donor> {
        self.donors.binary_search_by_key(&id, |d| d.id).ok().map(|k| &self.donors[k])
    }
    pub(crate) fn donor_mut(&mut self, id: u32) -> Option<&mut Donor> {
        self.donors.binary_search_by_key(&id, |d| d.id).ok().map(move |k| &mut self.donors[k])
    }
    pub fn circle(&self, id: u32) -> Option<&Circle> {
        self.circles.binary_search_by_key(&id, |c| c.id).ok().map(|k| &self.circles[k])
    }
}

/// The donors' side of a day (called after the books): the first day the
/// store is built (a new world, or an older file), on the money year's day
/// (1 July) the donors age, die, join and sync with the boosters, and
/// ex-pros join; every day, pledges tied to a coach who left are pulled.
pub(crate) fn day(w: &mut World, d: Date) {
    if w.capital.off {
        return;
    }
    if w.capital.year == 0 || (d == w.dates.money_year && w.capital.year < w.year) {
        donors::year(w);
        expros::year(w);
        w.capital.year = w.year;
    }
    pledges::day(w, d);
}

/// A keyed draw in 0-1 for a purpose and a key (its own stream: no other
/// result in the world moves).
pub(crate) fn draw(w: &World, what: u64, key: u64) -> f64 {
    crate::world::season::key_uniform(w.options.seed as u64 ^ 0xCA91_7A10_0000 ^ what << 44, key)
}
