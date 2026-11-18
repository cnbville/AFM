//! Pledges, and the ones tied to a coach (Capital lane C1, CP5).
//!
//! A pledge is paid over three to five years (real campaigns). One a donor
//! made because of the coach who closed it is tied to him: when that coach
//! no longer works at the school, the donor pulls what is left, and he may
//! follow the coach to his new college, which becomes his second school
//! (CP28). Lane C3 makes and pays pledges and feels a pulled one in the
//! building; this lane keeps them and pulls them. Sizes are Claude's
//! (proposed, not confirmed).

use crate::calendar::Date;
use crate::world::World;
use crate::world::coach::Employer;

/// A pledge.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Pledge {
    pub id: u32,
    pub donor: u32,
    pub program: u32,
    /// The whole pledge and what is paid so far, in dollars.
    pub total: f64,
    pub paid: f64,
    pub made: Option<Date>,
    /// Years it is paid over.
    pub years: u8,
    /// The coach it is tied to (CP5).
    pub tied: Option<u32>,
    /// The day the donor pulled the rest.
    pub pulled: Option<Date>,
    /// The room it is for, in the spec's CP17 order.
    pub room: Option<u8>,
    /// The drive it went to (lane C3; 0: waiting for the program's next).
    pub drive: u32,
    /// The moment that brought it (the strongest the fever still held the
    /// day he said yes) and the player in it; None: no moment, plain work
    /// (lane CS3 shows it beside the gift).
    pub moment: Option<(super::fever::What, Option<u32>)>,
}

impl Pledge {
    /// What is still to come.
    pub fn left(&self) -> f64 {
        if self.pulled.is_some() { 0.0 } else { (self.total - self.paid).max(0.0) }
    }
    pub fn open(&self) -> bool {
        self.pulled.is_none() && self.paid < self.total
    }
}

/// A pledge is tied to the coach who closed it when the donor's warmth
/// toward him is at least this (he gave because of him).
pub const TIE_WARMTH: f32 = 0.7;
/// The chance a donor whose tied pledge he pulled follows the coach to his
/// new college: 0.15 at the tie's warmth, up to 0.6 at full warmth.
pub const FOLLOW_LO: f64 = 0.15;
pub const FOLLOW_HI: f64 = 0.6;
/// A follower's pull toward the coach's new school (CP28), and what is left
/// of his bond with his own school.
pub const FOLLOW_PULL: f32 = 0.5;
pub const FOLLOW_AFFINITY: f32 = 0.7;

/// A new pledge (lane C3 calls it when a donor says yes). `closer` is the
/// coach in the room: the pledge is tied to him when the donor gave because
/// of him (`TIE_WARMTH`).
#[allow(clippy::too_many_arguments)]
pub fn make(
    w: &mut World,
    donor: u32,
    program: u32,
    total: f64,
    years: u8,
    closer: Option<u32>,
    room: Option<u8>,
    day: Date,
) -> u32 {
    let tied = closer.filter(|&c| w.capital.donor(donor).is_some_and(|d| d.warmth_to(c) >= TIE_WARMTH));
    let id = w.capital.id();
    w.capital.pledges.push(Pledge {
        id,
        donor,
        program,
        total,
        paid: 0.0,
        made: Some(day),
        years: years.clamp(1, 5),
        tied,
        pulled: None,
        room,
        drive: 0,
        moment: None,
    });
    id
}

/// Whether a coach still works at a college (any job on its staff).
fn works_at(w: &World, coach: u32, p: u32) -> bool {
    w.coaches
        .get(coach as usize)
        .and_then(|c| c.job.as_ref())
        .is_some_and(|j| j.employer == Employer::College(p as usize))
}

/// Every day: an open pledge tied to a coach who left the school is pulled,
/// and the donor may follow him (CP5).
pub(crate) fn day(w: &mut World, d: Date) {
    let gone: Vec<usize> = (0..w.capital.pledges.len())
        .filter(|&k| {
            let p = &w.capital.pledges[k];
            p.open() && p.tied.is_some_and(|c| !works_at(w, c, p.program))
        })
        .collect();
    for k in gone {
        w.capital.pledges[k].pulled = Some(d);
        let (id, donor, program, coach) = {
            let p = &w.capital.pledges[k];
            (p.id, p.donor, p.program, p.tied.unwrap())
        };
        let Some(warm) = w.capital.donor(donor).map(|x| x.warmth_to(coach)) else { continue };
        let to = w.coaches.get(coach as usize).and_then(|c| c.job.as_ref()).and_then(|j| match j.employer {
            Employer::College(q)
                if q as u32 != program && w.programs[q].division != crate::world::data::Division::JUCO =>
            {
                Some(q as u32)
            }
            _ => None,
        });
        let Some(q) = to else { continue };
        let span = (1.0 - TIE_WARMTH) as f64;
        let chance = FOLLOW_LO + (FOLLOW_HI - FOLLOW_LO) * ((warm - TIE_WARMTH) as f64 / span).clamp(0.0, 1.0);
        if super::draw(w, 7, (id as u64) << 20 ^ q as u64) < chance
            && let Some(x) = w.capital.donor_mut(donor)
            && x.alive()
            && x.home != q
        {
            x.also = Some(q);
            x.also_pull = x.also_pull.max(FOLLOW_PULL);
            x.affinity *= FOLLOW_AFFINITY;
        }
    }
}
