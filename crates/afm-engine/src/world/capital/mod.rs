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
//! future changes; its past stays the same per seed. Every draw here is keyed on the world's seed and the
//! donor, never on a shared stream.
//!
//! Lane C6, high school and the NFL: five rooms a high school, the booster
//! club's room list and the district's stadium bond with your work in town,
//! the stadium shared across the district (`hs`, CP32-CP34); eight rooms a
//! club that move the players' report card, your room asks of the owner and
//! your leverage (`nfl`, CP35, CP36). Sizes are Claude's (proposed, not
//! confirmed) unless the spec gives them.

pub mod donors;
pub mod drive;
pub mod expros;
pub mod fever;
pub mod gifts;
pub mod hs;
pub mod nfl;
pub mod pledges;
pub mod rooms;
pub mod screen;

use crate::calendar::Date;
use crate::world::World;

pub use donors::{Bond, Circle, Donor, Estate, Money};
pub use drive::{Drive, Fix, Size, Stage};
pub use fever::{Moment, What};
pub use gifts::{Answer, Catch, CatchKind, Side};
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
    /// Every program's drives (lane C3), oldest first.
    pub drives: Vec<Drive>,
    /// The moments behind the fever (lane C3), the last two years or so.
    pub moments: Vec<Moment>,
    /// The catches gifts carried (CP10).
    pub catches: Vec<Catch>,
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
    /// Every high school's rooms, by school (lane C6, CP33), and the
    /// typical room's value this year.
    pub hs: Vec<hs::HsCampus>,
    pub hs_typical: [f32; hs::HS_ROOMS],
    /// The districts' stadium bonds (CP32, CP34), oldest first.
    pub bonds: Vec<hs::Bond>,
    /// Your work in town before a vote.
    pub town: Vec<hs::TownWork>,
    /// The money year you last asked your board for a bond.
    pub(crate) bond_asked: i32,
    /// The July the high school rooms last aged.
    pub(crate) hs_year: i32,
    /// Every NFL club's rooms, by team (CP35), and the league's typical.
    pub clubs: Vec<nfl::ClubRooms>,
    pub nfl_typical: [f32; rooms::ROOMS],
    /// Room renovations the owners pay for.
    pub club_builds: Vec<nfl::ClubBuild>,
    /// Your asks of the owner for a room.
    pub room_asks: Vec<nfl::RoomAsk>,
    /// The league year the club rooms last aged.
    pub(crate) nfl_year: i32,
    /// Test switch: the store stands still (the same-world check).
    #[serde(skip)]
    pub(crate) off: bool,
    /// Test and measure switches: the high school and NFL rooms read
    /// neutral (lane C6's arm without it, `facilities_check`).
    #[serde(skip)]
    pub hs_off: bool,
    #[serde(skip)]
    pub nfl_off: bool,
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
    fever::day(w, d);
    if live(w) {
        gifts::catches(w, d);
        if d.day() == 1 {
            drive::month(w, d);
        }
        // Lane C6: the high school rooms age each July; the boards'
        // windows, election nights and openings; the clubs' renovations.
        if d == w.dates.money_year || w.capital.hs.is_empty() {
            hs::year(w);
        }
        hs::day(w, d);
        nfl::day(w, d);
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
