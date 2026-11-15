//! The job a club grades a prospect for (draft overreach research,
//! 2026-11-15; Cain: "whatever can be fixed without engine, needs to be
//! fixed asap"). Real clubs grade a man for the role he would play in their
//! scheme: a slot or an outside receiver, a nickel or an outside corner, a
//! 4-3 hand-down end or a 3-4 stand-up rusher, a three-technique, a nose or
//! a 3-4 end, a Mike, Will or Sam, a single-high free safety, a box safety
//! or a two-deep safety. So the same edge rusher sits higher on a 3-4 board
//! than on a 4-3 one, and a slot-only receiver sits lower with a club that
//! plays two receivers.
//!
//! The rating at a job is the depth chart's spot rating where a spot exists
//! (`spots::lean_value`, the same lean and size), and the same shape for
//! the jobs the depth chart doesn't name (the receiver's two, the edge's
//! two fronts, the 3-4 end). A job the club plays only part of the time
//! (the nickel, the Sam, a 3-4 nose, the slot in a two-receiver offense)
//! is worth less to it by the snaps it misses, at the shares the engine
//! plays (`game::calling`: three receivers on about 95% of air raid and
//! spread snaps, 60% West Coast, 50% wide zone, 40% power gap; nickel or
//! dime on about two NFL snaps in three).
//!
//! The board reads this from the club's own file (each attribute's centre),
//! never the truth, and centres it on its own filed prospects at the
//! position, so it moves men within a position and leaves the position's
//! place on the board as it was. Every size here is proposed, not
//! confirmed.

use crate::attributes::Attr::{self, *};
use crate::attributes::Attributes;
use crate::game::roster::{Front, OffScheme, Shell};
use crate::positions::Position;
use crate::spots::{LEAN, SPEED_PER_LB, Spot, lean_value};
use crate::world::systems::Installed;

/// A job a club can grade a man for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Job {
    OutsideWr,
    SlotWr,
    OutsideCb,
    Nickel,
    FreeSafety,
    BoxSafety,
    /// A safety in a two-high shell, who has to play deep and come down.
    TwoDeep,
    /// The 4-3 end: a hand in the dirt, sets the edge and rushes.
    HandDownEnd,
    /// The 3-4 outside linebacker: stands up, bends the corner, drops.
    StandUpRusher,
    ThreeTech,
    Nose,
    /// The 3-4 end (four- and five-technique): two-gaps the tackle.
    FiveTech,
    Mike,
    Will,
    Sam,
}

impl Job {
    pub fn words(self) -> &'static str {
        match self {
            Job::OutsideWr => "outside receiver",
            Job::SlotWr => "slot receiver",
            Job::OutsideCb => "outside corner",
            Job::Nickel => "nickel corner",
            Job::FreeSafety => "deep safety",
            Job::BoxSafety => "box safety",
            Job::TwoDeep => "two-deep safety",
            Job::HandDownEnd => "4-3 end",
            Job::StandUpRusher => "3-4 outside linebacker",
            Job::ThreeTech => "three-technique",
            Job::Nose => "nose tackle",
            Job::FiveTech => "3-4 end",
            Job::Mike => "middle linebacker",
            Job::Will => "weak-side linebacker",
            Job::Sam => "strong-side linebacker",
        }
    }
}

/// Rating points a job loses per share of snaps it doesn't play (proposed,
/// not confirmed): a nickel at two snaps in three gives up about 2, a Sam
/// at a third gives up 4, a slot in a power offense 3.6.
pub const PART_TIME: f64 = 6.0;

/// The share of snaps in nickel or dime the engine plays at the NFL (three
/// receivers on about 70% of snaps, a nickel against them nine times in
/// ten; real 2023 about 65-70%).
pub const SUB_SHARE: f64 = 0.65;

/// The share of an offense's snaps with three or more receivers, as the
/// engine calls personnel (`game::calling`, outside short yardage).
pub fn three_wide(o: OffScheme) -> f64 {
    match o {
        OffScheme::AirRaid | OffScheme::SpreadOption => 0.95,
        OffScheme::WestCoast => 0.6,
        OffScheme::WideZone => 0.5,
        OffScheme::PowerGap => 0.4,
        OffScheme::WingT => 0.1,
    }
}

fn mix(a: &Attributes, w: &[(Attr, f64)]) -> f64 {
    w.iter().map(|&(x, v)| v * a.get(x)).sum()
}

/// What a job asks beyond the position's overall, on the spot lean's scale.
fn lean(job: Job, a: &Attributes, weight: f64, height: f64) -> f64 {
    // His true speed about a job's typical weight, as the spot leans read it.
    let speed_at = |pivot: f64| a.get(Speed) - SPEED_PER_LB * (weight - pivot);
    let size = |per_lb: f64, pivot: f64| (50.0 + per_lb * (weight - pivot)).clamp(1.0, 99.0);
    match job {
        Job::OutsideWr => {
            let v = 0.3 * speed_at(200.0)
                + mix(a, &[(Release, 0.25), (ContestedCatch, 0.2), (BallTracking, 0.15), (Jumping, 0.1)]);
            0.9 * v + 0.1 * (50.0 + 6.0 * (height - 72.0)).clamp(1.0, 99.0)
        }
        Job::SlotWr => {
            mix(a, &[(Agility, 0.3), (Acceleration, 0.25), (RouteRunning, 0.25), (Catching, 0.1), (BodyControl, 0.1)])
        }
        Job::HandDownEnd => {
            let v = mix(a, &[(PowerRush, 0.25), (BlockShedding, 0.2), (Anchor, 0.2), (HandUse, 0.2), (Strength, 0.15)]);
            0.85 * v + 0.15 * size(2.5, 262.0)
        }
        Job::StandUpRusher => {
            0.15 * speed_at(250.0)
                + mix(
                    a,
                    &[
                        (Acceleration, 0.25),
                        (FinesseRush, 0.25),
                        (Flexibility, 0.2),
                        (Pursuit, 0.1),
                        (ZoneCoverage, 0.05),
                    ],
                )
        }
        Job::FiveTech => {
            let v = mix(a, &[(Anchor, 0.25), (BlockShedding, 0.25), (Strength, 0.2), (PowerRush, 0.2), (HandUse, 0.1)]);
            0.85 * v + 0.15 * size(2.0, 295.0)
        }
        Job::OutsideCb => lean_value(Spot::CB, a, weight, height),
        Job::Nickel => lean_value(Spot::NB, a, weight, height),
        Job::FreeSafety => lean_value(Spot::FS, a, weight, height),
        Job::BoxSafety => lean_value(Spot::SS, a, weight, height),
        Job::TwoDeep => 0.5 * (lean_value(Spot::FS, a, weight, height) + lean_value(Spot::SS, a, weight, height)),
        Job::ThreeTech => lean_value(Spot::DT, a, weight, height),
        Job::Nose => lean_value(Spot::NT, a, weight, height),
        Job::Mike => lean_value(Spot::MLB, a, weight, height),
        Job::Will => lean_value(Spot::OLB, a, weight, height),
        Job::Sam => lean_value(Spot::SAM, a, weight, height),
    }
}

/// The jobs a position's men are compared across (a job's rating is its
/// lean over the mean of the others, as `spots::spot_rating` does).
fn family(pos: Position) -> &'static [Job] {
    use Position as P;
    match pos {
        P::WR => &[Job::OutsideWr, Job::SlotWr],
        P::CB => &[Job::OutsideCb, Job::Nickel],
        P::S => &[Job::FreeSafety, Job::BoxSafety],
        P::EDGE => &[Job::HandDownEnd, Job::StandUpRusher],
        P::DT => &[Job::ThreeTech, Job::Nose, Job::FiveTech],
        P::LB => &[Job::Mike, Job::Will, Job::Sam],
        _ => &[],
    }
}

/// Whether a position's men are graded for a job.
pub fn has_jobs(pos: Position) -> bool {
    !family(pos).is_empty()
}

/// The jobs a club with these systems has for a position, each with its
/// share of the snaps. A two-high shell's safeties play both halves of the
/// job, so they are graded as one two-deep safety (Cover 2's lean deep,
/// quarters' even).
fn jobs(pos: Position, inst: &Installed) -> Vec<(Job, f64)> {
    use Position as P;
    let three_four = inst.front() == Front::ThreeFour;
    match pos {
        P::WR => vec![(Job::OutsideWr, 1.0), (Job::SlotWr, three_wide(inst.offense()))],
        P::CB => vec![(Job::OutsideCb, 1.0), (Job::Nickel, SUB_SHARE)],
        P::S => match inst.shell() {
            Shell::Cover3 | Shell::ManPressure => vec![(Job::FreeSafety, 1.0), (Job::BoxSafety, 1.0)],
            Shell::Quarters | Shell::Cover2 => vec![(Job::TwoDeep, 1.0)],
        },
        P::EDGE if three_four => vec![(Job::StandUpRusher, 1.0)],
        P::EDGE => vec![(Job::HandDownEnd, 1.0)],
        // A 3-4 nose comes off for the nickel; a 4-3's two tackles stay.
        P::DT if three_four => vec![(Job::FiveTech, 1.0), (Job::Nose, 1.0 - SUB_SHARE)],
        P::DT => vec![(Job::ThreeTech, 1.0), (Job::Nose, 1.0)],
        // The 3-4's two inside backers both stay in the nickel; the 4-3's
        // Sam comes off.
        P::LB if three_four => vec![(Job::Mike, 1.0), (Job::Will, 1.0)],
        P::LB => vec![(Job::Mike, 1.0), (Job::Will, 1.0), (Job::Sam, 1.0 - SUB_SHARE)],
        _ => Vec::new(),
    }
}

/// His rating at a job over his position's overall, in rating points: his
/// lean there over the mean of the others' (`spots::spot_rating`).
pub fn lift(pos: Position, job: Job, a: &Attributes, weight: f64, height: f64) -> f64 {
    let fam = family(pos);
    if fam.len() < 2 {
        return 0.0;
    }
    let me = lean(job, a, weight, height);
    let others: f64 =
        fam.iter().filter(|&&j| j != job).map(|&j| lean(j, a, weight, height)).sum::<f64>() / (fam.len() - 1) as f64;
    LEAN * (me - others) / 2.0
}

/// The job a club grades him for and what it's worth over his overall
/// (rating points, before the board centres it), or None for a position
/// with one job. `a` is the club's read of his attributes; weight and
/// height as listed.
pub fn role(pos: Position, a: &Attributes, weight: f64, height: f64, inst: &Installed) -> Option<(Job, f64)> {
    let js = jobs(pos, inst);
    let mut best: Option<(Job, f64)> = None;
    for (j, share) in js {
        let at = match j {
            // Both halves of the job: Cover 2's deep halves lean to the
            // free safety's range, quarters asks for both evenly.
            Job::TwoDeep => {
                let deep = if inst.shell() == Shell::Cover2 { 0.75 } else { 0.5 };
                deep * lift(pos, Job::FreeSafety, a, weight, height)
                    + (1.0 - deep) * lift(pos, Job::BoxSafety, a, weight, height)
            }
            _ => lift(pos, j, a, weight, height),
        };
        let v = at - PART_TIME * (1.0 - share);
        if best.is_none_or(|b| v > b.1) {
            best = Some((j, v));
        }
    }
    best
}
