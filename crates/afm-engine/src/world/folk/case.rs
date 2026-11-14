//! A recruit's case per school (talk facts design, section 6, "How much it
//! counts"; `afm/talk-facts/proposal.md` in the project folder): what every
//! call, message and visit a school makes to a recruit or a transfer adds
//! up to.
//!
//! - **A point lands** by today's pitch formula: 0.012 x the speaker's skill
//!   x (how much he cares x (0.3 + 0.7 x your proof) - 0.6). Overselling a
//!   weak point to a family that cares costs 0.006 more (`lands`).
//! - **Each group counts at the best point made in it**, so up to seven
//!   groups add up, and no point has a weight of its own: its weight is how
//!   much he cares (`Case`).
//! - **Coming back to a point that landed** adds half its size in a later
//!   talk, then a quarter (`claims::Talked::again`), so the same point can
//!   at most double. A fact that got better counts in full.
//! - **The whole case is capped** at +0.06 and -0.03 per school (Cain
//!   2026-11-12: "i'll trust you in this"). A call lands at half of a
//!   sit-down.
//! - **A staff in one step** (`one_step`): every AI staff, and your own
//!   staff when you hand a talk, covers the groups its call or visit allows,
//!   picking with noise from the recruiting rating of the man making it.
//!   His proof is the program's own grade on each group (as every pitch read
//!   it before), never graded against the recruit's other schools: that
//!   would need the recruit's race, which reads this case, and a sheet for
//!   every school and recruit each day.
//!
//! Every size here is Claude's (proposed, not confirmed) unless it names
//! Cain's answer.

use super::claims::{Ear, Group, Tier};
use crate::calendar::Date;
use crate::player::PlayingTime;
use crate::world::program::GradeCat;
use crate::world::{Person, World};

/// A point's rate: today's pitch formula (`0.012 x skill x (...)`).
/// Proposed, not confirmed.
pub const RATE: f64 = 0.012;
/// What a point must clear to help: how much he cares x (0.3 + 0.7 x your
/// proof) over this. Proposed, not confirmed.
pub const CLEAR: f64 = 0.6;
/// One point's pull is kept inside these, in appeal points (today's).
/// Proposed, not confirmed.
pub const POINT_RANGE: (f32, f32) = (-0.012, 0.03);
/// Overselling a weak point to a family that cares costs this much more.
/// Proposed, not confirmed.
pub const OVERSELL: f32 = 0.006;
/// The whole case per school, at most (Cain 2026-11-12, question 3: "i'll
/// trust you in this").
pub const CASE_TOP: f32 = 0.06;
/// The whole case per school, at least. Proposed, not confirmed.
pub const CASE_FLOOR: f32 = -0.03;
/// A phone call lands at half of a sit-down (as built). Proposed, not
/// confirmed.
pub const PHONE: f32 = 0.5;
/// A call is three exchanges (Cain agreed 2026-11-12).
pub const CALL_EXCHANGES: usize = 3;
/// A sit-down is six exchanges (Cain agreed 2026-11-12).
pub const SIT_DOWN_EXCHANGES: usize = 6;
/// A point made in a message lands at a quarter of a sit-down (talk facts
/// design, section 6, "Calls and messages"; section 13). Proposed, not
/// confirmed.
pub const MESSAGE: f32 = 0.25;
/// A message is one exchange: you name one thing and he answers.
pub const MESSAGE_EXCHANGES: usize = 1;
/// His family's call is one or two exchanges, their question first.
/// Proposed, not confirmed.
pub const FAMILY_EXCHANGES: usize = 2;
/// Each answer you gave him straight raises how far he believes your
/// promises by this much, up to `STRAIGHT_MOST` answers. Proposed, not
/// confirmed.
pub const STRAIGHT_BELIEF: f64 = 0.04;
/// The straight answers that count toward his belief. Proposed, not
/// confirmed.
pub const STRAIGHT_MOST: u8 = 3;

/// How well the man making a point sells it: his recruiting and how he is
/// with people (today's pitch skill, 0.3 to 1.5).
pub fn skill(w: &World, coach: Option<usize>) -> f64 {
    let a = coach.map(|c| *w.coaches[c].ability());
    (0.6 + 0.04 * a.map_or(0.0, |a| a.recruiting - 10.0) + 0.03 * a.map_or(0.0, |a| a.relationships - 10.0))
        .clamp(0.3, 1.5)
}

/// What one point adds to a school's pull on him at a sit-down, in appeal
/// points: how much he cares (1.0 as much as the rest), your proof next to
/// his other schools, and how well the man saying it sells (section 6).
pub fn lands(w: &World, care: f32, proof: Tier, coach: Option<usize>) -> f32 {
    let mut lean = (RATE * skill(w, coach) * (care as f64 * (0.3 + 0.7 * proof.value() as f64) - CLEAR)) as f32;
    if proof == Tier::Weak && care > 1.0 {
        lean -= OVERSELL;
    }
    lean.clamp(POINT_RANGE.0, POINT_RANGE.1)
}

/// One school's case with one recruit: the best point it has made in each
/// group, what coming back to points that landed added, and the answers it
/// gave him straight.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Case {
    /// The school (a program index).
    pub prog: u16,
    /// The best point made in each group (`Group` order), in appeal points.
    pub groups: [f32; 7],
    /// Which groups have a point: one bit a group.
    pub made: u8,
    /// What coming back to points that landed added.
    pub again: f32,
    /// Answers given straight where the proof was weak (he believes your
    /// promises a little more).
    pub straight: u8,
    /// The points made in this case, one bit each by its place in the
    /// catalogue (`claims::Kind::ALL`): a point he heard for another case
    /// is new here.
    pub points: [u64; 4],
    /// The last day a point was made.
    pub on: Date,
    /// A promise made to a high school recruit in this case (lane L6): its
    /// kind + 1 (`pledge::Kind`), 0 for none. One a school.
    pub promise: u8,
    /// What the promise pulls, in appeal points, beside the case and its
    /// cap, as a transfer's promise does.
    pub pledge: f32,
    /// His position when you promised it (`Position as u8`).
    pub pos: u8,
    /// The day you promised it.
    pub promised: Date,
    /// Logged as the school's promise once he signed with it.
    pub logged: bool,
}

impl Case {
    pub fn new(prog: usize) -> Case {
        Case { prog: prog as u16, ..Case::default() }
    }

    /// Whether the point at `place` in the catalogue was made in this case.
    pub fn heard(&self, place: usize) -> bool {
        place < 256 && self.points[place / 64] >> (place % 64) & 1 != 0
    }

    /// Marks the point at `place` in the catalogue as made in this case.
    pub fn hear(&mut self, place: usize) {
        if place < 256 {
            self.points[place / 64] |= 1 << (place % 64);
        }
    }

    /// Whether a point has been made in a group.
    pub fn has(&self, g: Group) -> bool {
        self.made & (1 << g as u8) != 0
    }

    /// What the case adds to the school's pull on him: every group's best
    /// point and the comebacks, under the cap.
    pub fn total(&self) -> f32 {
        let groups: f32 = Group::ALL.iter().filter(|&&g| self.has(g)).map(|&g| self.groups[g as usize]).sum();
        (groups + self.again).clamp(CASE_FLOOR, CASE_TOP)
    }

    /// Whether anything has been said.
    pub fn is_empty(&self) -> bool {
        self.made == 0 && self.again == 0.0 && self.straight == 0
    }

    /// A point made today in group `g`, worth `lean` in full, of which
    /// `again` counts (`Talked::again`: 1 the first time or once the fact
    /// got better, a half or a quarter when you come back to one that
    /// landed, 0 in the same talk). A case with nothing yet in the group
    /// takes it in full: what he heard before went into another case (a
    /// school you left, or a case made fresh since). Returns what the total
    /// moved.
    pub fn add(&mut self, g: Group, lean: f32, again: f32, today: Date) -> f32 {
        let before = self.total();
        let k = g as usize;
        if again >= 1.0 || !self.has(g) {
            if !self.has(g) || lean > self.groups[k] {
                self.groups[k] = lean;
                self.made |= 1 << k;
            }
        } else if again > 0.0 && lean > 0.0 {
            self.again += again * lean;
        }
        self.on = self.on.max(today);
        self.total() - before
    }

    /// Another case for the same school folded in: each group at the
    /// better of the two (more scenes never raise the ceiling).
    pub fn merge(&mut self, o: &Case) {
        for g in Group::ALL {
            if o.has(g) {
                self.add(g, o.groups[g as usize], 1.0, o.on);
            }
        }
        self.again += o.again;
        self.straight = self.straight.saturating_add(o.straight);
        if self.promise == 0 && o.promise != 0 {
            (self.promise, self.pledge, self.pos, self.promised) = (o.promise, o.pledge, o.pos, o.promised);
        }
        for (a, b) in self.points.iter_mut().zip(o.points) {
            *a |= b;
        }
        self.on = self.on.max(o.on);
    }

    /// How much more he believes your promises for your straight answers.
    pub fn belief(&self) -> f64 {
        STRAIGHT_BELIEF * self.straight.min(STRAIGHT_MOST) as f64
    }
}

/// A recruit's case with a school, if any school has made one.
pub fn of(p: &Person, j: usize) -> Option<&Case> {
    p.cal.cases.iter().find(|c| c.prog as usize == j)
}

/// What a school's talks add to its pull on a high school recruit: its
/// case and its promise (`pledge`), or for a save from before the case, the living room's lean it
/// kept (`home_talks`).
pub fn pull(p: &Person, j: usize) -> f64 {
    match of(p, j) {
        Some(c) => (c.total() + c.pledge) as f64,
        None => p.cal.home_talks.iter().filter(|x| x.0 == j).map(|x| x.1 as f64).sum(),
    }
}

/// A high school recruit's case with a school, to change (made empty when
/// there is none).
pub(crate) fn case_mut(w: &mut World, i: usize, j: usize) -> &mut Case {
    let v = &mut w.people[i].cal.cases;
    let k = match v.iter().position(|c| c.prog as usize == j) {
        Some(k) => k,
        None => {
            // A save from before the case: the living room's lean it kept
            // still counts, in no group.
            let mut c = Case::new(j);
            c.again = w.people[i].cal.home_talks.iter().filter(|x| x.0 == j).map(|x| x.1).sum();
            let v = &mut w.people[i].cal.cases;
            v.push(c);
            v.len() - 1
        }
    };
    &mut w.people[i].cal.cases[k]
}

/// What a school can back each group up with for a high school recruit,
/// 0-1, in `Group` order: its grades, its offer, the miles, and for the
/// people the head coach himself (how long he has been there, how he is
/// with people). A staff's own read in one step.
pub(crate) fn backing(w: &World, i: usize, j: usize) -> [f64; 7] {
    // What the school really offers him: yours only what you promised.
    let nil = crate::world::recruiting::offer_money(w, j, i);
    let floor = 0.6 * w.programs[j].grades[GradeCat::Nil as usize] / 100.0;
    backing_with(w, i, j, (nil / 1.0e6).powf(0.6).min(1.0).max(floor))
}

/// `backing` with the money group's own backing (0-1) given.
pub(crate) fn backing_with(w: &World, i: usize, j: usize, money: f64) -> [f64; 7] {
    let p = &w.people[i];
    let prog = &w.programs[j];
    let g = |c: GradeCat| prog.grades[c as usize] / 100.0;
    let near = crate::world::recruiting::closeness(crate::world::recruiting::home_miles(&w.high_schools, p, prog));
    let people = prog.head_coach.map_or(0.3, |c| {
        let years = w.coaches[c].job.map_or(0, |jb| (w.year - jb.since).max(0)) as f64;
        (0.25 + 0.06 * years.min(8.0) + 0.02 * (w.coaches[c].ability().relationships - 10.0)).clamp(0.1, 1.0)
    });
    [
        g(GradeCat::PlayingTime),
        money,
        g(GradeCat::Academics),
        near,
        people,
        (g(GradeCat::ProPipeline) + g(GradeCat::CoachPrestige)) / 2.0,
        (g(GradeCat::TitleChances) + g(GradeCat::BrandExposure)) / 2.0,
    ]
}

/// What the role he'd have backs the playing-time group with.
pub(crate) fn role_backing(role: PlayingTime) -> f64 {
    match role {
        PlayingTime::Starter => 0.95,
        PlayingTime::Rotation => 0.6,
        _ => 0.25,
    }
}

/// What a program can back each group up with for a transfer: the role
/// he'd have there, the money it would pay him, the rest as for a recruit.
pub(crate) fn transfer_backing(w: &World, i: usize, j: usize, role: PlayingTime, nil: f64) -> [f64; 7] {
    let mut b = backing_with(w, i, j, (nil.max(0.0) / 1.0e6).powf(0.6).min(1.0));
    b[Group::PlayingTime as usize] = role_backing(role);
    b
}

/// How far a man's read of what a recruit weighs strays, by his recruiting
/// rating (today's pick: 0.6 x (1.6 - 0.06 x rating), at least 0.12).
pub(crate) fn read_noise(w: &World, coach: Option<usize>) -> f64 {
    let r = coach.map_or(10.0, |c| w.coaches[c].ability().recruiting);
    0.6 * (1.6 - 0.06 * r).max(0.2)
}

/// A staff's case in one step (section 6: "AI staffs build the same case in
/// one step"): the man making it covers up to `points` groups, the ones his
/// read says land (how much he reads the listener caring times what the
/// school can back up, over what a point must clear), best first, and at
/// least the one he reads best. Each lands by `lands` at `scale` (a call is
/// `PHONE`). `back` is the school's backing per group (`backing`,
/// `transfer_backing`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn one_step(
    w: &World,
    i: usize,
    j: usize,
    coach: Option<usize>,
    ear: Ear,
    back: [f64; 7],
    points: usize,
    scale: f32,
    seed: u64,
) -> Case {
    let care = super::claims::group_cares(&w.people[i], ear);
    let sd = read_noise(w, coach);
    let tier = |k: usize| Tier::of(back[k] as f32);
    let mut read: Vec<(f64, usize)> = (0..Group::ALL.len())
        .map(|k| {
            let truth = care[k] as f64 * (0.3 + 0.7 * tier(k).value() as f64);
            (truth + sd * crate::game::talk::normal(seed, k as u64), k)
        })
        .collect();
    read.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut c = Case::new(j);
    for (n, &(r, k)) in read.iter().enumerate().take(points.max(1)) {
        if n > 0 && r <= CLEAR {
            break;
        }
        let g = Group::ALL[k];
        c.add(g, scale * lands(w, care[k], tier(k), coach), 1.0, w.today);
    }
    c
}

/// The groups a staff's one-step case covered, best first (what your
/// staff brings back when you hand it a talk).
pub(crate) fn covered(c: &Case) -> Vec<Group> {
    let mut v: Vec<Group> = Group::ALL.into_iter().filter(|&g| c.has(g)).collect();
    v.sort_by(|a, b| c.groups[*b as usize].total_cmp(&c.groups[*a as usize]).then(a.cmp(b)));
    v
}
