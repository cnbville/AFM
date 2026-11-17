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
//! Lane C3, the money and the drive: big moments start the fever (`fever`,
//! CP7, CP8); donors give donor by donor between NIL and buildings, some
//! gifts with a catch (`gifts`, CP9, CP10); every program's capital
//! campaign in its real stages, with your goal, an early launch, borrowing,
//! overruns and the development office (`drive`, CP11, CP12, CP14-CP16,
//! CP24), on the same rules for the AI (CP27). From the season the world
//! hands over, drives replace the ADs' old July decisions, so the world's
//! future changes; its past stays the same per seed.
//!
//! Lane C4, your week and scenes: meeting a donor takes a slot in your
//! week (`week`, CP2), all year but only the suite before a home game in
//! season (CP13); the lunch, the dinner and the rooms of donors play as
//! scenes (`scenes`); you choose the three rooms a recruit tours on his
//! official visit (`tour`, CP20); names go up on buildings and rooms, and a
//! named donor's scandal is your call (`names`, CP25); and the
//! groundbreaking, the reveal video and the ribbon cutting keep their
//! data for the big moments (`ceremony`, CP38). Only your seat has them.
//!
//! Every draw here is keyed on the world's seed and the
//! donor, never on a shared stream. Sizes are Claude's (proposed, not
//! confirmed) unless the spec gives them.
//!
//! Lane C5, the race and the press (`race`): a rival's new room on a lost
//! recruit's why-line (CP29), rivals' studies and quiet phases heard only as
//! rumors from your own donors (CP30), and the press's facilities pecking
//! order of every conference each July (CP31). News and text only.

pub mod ceremony;
pub mod donors;
pub mod drive;
pub mod expros;
pub mod fever;
pub mod gifts;
pub mod names;
pub mod pledges;
pub mod race;
pub mod rooms;
pub mod scenes;
pub mod screen;
pub mod tour;
pub mod week;

use crate::calendar::Date;
use crate::world::World;

pub use donors::{Bond, Circle, Donor, Estate, Money};
pub use drive::{Drive, Fix, Size, Stage};
pub use fever::{Moment, What};
pub use gifts::{Answer, Catch, CatchKind, Side};
pub use pledges::Pledge;
pub use race::{Pecking, Placed, Rumor};

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
    /// Every program's drives (lane C3), oldest first.
    pub drives: Vec<Drive>,
    /// The moments behind the fever (lane C3), the last two years or so.
    pub moments: Vec<Moment>,
    /// The catches gifts carried (CP10).
    pub catches: Vec<Catch>,
    /// Rumors of rivals' drives your donors passed on (lane C5, CP30), the
    /// last four years.
    pub rumors: Vec<Rumor>,
    /// The press's facilities pecking order of every conference each July
    /// (lane C5, CP31), the last 30 years.
    pub pecking: Vec<Pecking>,
    /// Lane C4: the donor meetings you booked (CP2), the tours you chose
    /// (CP20, sorted by recruit), names on buildings and rooms and the
    /// drives already named, named donors' scandals (CP25), your program's
    /// big moments (CP38), and whether the names up at the handover were
    /// set.
    pub booked: Vec<week::Booked>,
    pub tours: Vec<tour::Tour>,
    pub names: Vec<names::Name>,
    pub named: Vec<u32>,
    pub scandals: Vec<names::Scandal>,
    pub ceremonies: Vec<ceremony::Ceremony>,
    pub seeded: bool,
    /// The college season and the draft year last looked at for moments.
    pub(crate) scanned: i32,
    pub(crate) drafted: i32,
    /// Your NIL asks this year (donor, year): one a donor a year.
    pub(crate) nil_asks: Vec<(u32, i32)>,
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
    if d == w.dates.money_year {
        race::pecking(w);
    }
    pledges::day(w, d);
    fever::day(w, d);
    if live(w) {
        gifts::catches(w, d);
        if d.day() == 1 {
            drive::month(w, d);
            race::rumors(w, d);
        }
        names::day(w, d);
        ceremony::day(w, d);
    }
}

/// The drives run: from the season the world hands over (the rooms live),
/// in a recorded world.
pub(crate) fn live(w: &World) -> bool {
    w.year >= w.options.start_year && w.recording && !w.capital.off
}

/// A keyed draw in 0-1 for a purpose and a key (its own stream: no other
/// result in the world moves).
pub(crate) fn draw(w: &World, what: u64, key: u64) -> f64 {
    crate::world::season::key_uniform(w.options.seed as u64 ^ 0xCA91_7A10_0000 ^ what << 44, key)
}
