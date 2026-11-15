//! Role grading on the NFL boards (`world::pro::roles`): a club grades a
//! man for the job he'd play in its scheme.
use crate::attributes::{Attr::*, Attributes};
use crate::positions::Position;
use crate::world::pro::roles::{Job, role};
use crate::world::systems::Installed;

fn inst(off: usize, front: usize, shell: usize) -> Installed {
    Installed { off, front, shell, ..Default::default() }
}

fn with(pairs: &[(crate::attributes::Attr, f64)]) -> Attributes {
    let mut a = Attributes::filled(65.0);
    for &(x, v) in pairs {
        a.set(x, v);
    }
    a
}

#[test]
fn a_3_4_club_grades_a_bendy_rusher_above_a_4_3_club() {
    // Fronts: 0 is the 4-3, 1 the 3-4.
    let bend =
        with(&[(Acceleration, 88.0), (FinesseRush, 85.0), (Flexibility, 84.0), (PowerRush, 58.0), (Anchor, 55.0)]);
    let power =
        with(&[(Acceleration, 62.0), (FinesseRush, 60.0), (Flexibility, 58.0), (PowerRush, 86.0), (Anchor, 84.0)]);
    let (j34, b34) = role(Position::EDGE, &bend, 248.0, 76.0, &inst(0, 1, 0)).unwrap();
    let (j43, b43) = role(Position::EDGE, &bend, 248.0, 76.0, &inst(0, 0, 0)).unwrap();
    assert_eq!(j34, Job::StandUpRusher);
    assert_eq!(j43, Job::HandDownEnd);
    assert!(b34 > b43 + 2.0, "bend {b34:.2} on a 3-4 board, {b43:.2} on a 4-3");
    let (_, p34) = role(Position::EDGE, &power, 272.0, 77.0, &inst(0, 1, 0)).unwrap();
    let (_, p43) = role(Position::EDGE, &power, 272.0, 77.0, &inst(0, 0, 0)).unwrap();
    assert!(p43 > p34 + 2.0, "power {p43:.2} on a 4-3 board, {p34:.2} on a 3-4");
}

#[test]
fn a_slot_only_receiver_is_worth_less_to_a_two_receiver_offense() {
    // Offenses: 1 air raid (three wide nearly always), 3 power gap.
    let slot =
        with(&[(Agility, 90.0), (Acceleration, 90.0), (RouteRunning, 85.0), (Speed, 70.0), (ContestedCatch, 50.0)]);
    let (ja, air) = role(Position::WR, &slot, 180.0, 69.0, &inst(1, 0, 0)).unwrap();
    let (_, power) = role(Position::WR, &slot, 180.0, 69.0, &inst(3, 0, 0)).unwrap();
    assert_eq!(ja, Job::SlotWr);
    assert!(air > power + 1.5, "slot {air:.2} to the air raid, {power:.2} to power gap");
    // A big outside receiver grades the same either way.
    let big = with(&[(Speed, 88.0), (Release, 85.0), (ContestedCatch, 88.0), (Agility, 60.0)]);
    let (jb, b1) = role(Position::WR, &big, 215.0, 76.0, &inst(1, 0, 0)).unwrap();
    let (_, b3) = role(Position::WR, &big, 215.0, 76.0, &inst(3, 0, 0)).unwrap();
    assert_eq!(jb, Job::OutsideWr);
    assert!((b1 - b3).abs() < 1e-9);
}

#[test]
fn safeties_and_corners_grade_for_their_jobs() {
    // Shells: 0 Cover 3 (single high), 1 quarters.
    let range = with(&[(Speed, 92.0), (ZoneCoverage, 85.0), (BallTracking, 82.0), (Tackling, 55.0), (Strength, 50.0)]);
    let (j, single) = role(Position::S, &range, 200.0, 72.0, &inst(0, 0, 0)).unwrap();
    let (j2, quarters) = role(Position::S, &range, 200.0, 72.0, &inst(0, 0, 1)).unwrap();
    assert_eq!(j, Job::FreeSafety);
    assert_eq!(j2, Job::TwoDeep);
    assert!(single > quarters, "a pure centre fielder is worth most to a single-high club");
    // A quick, small corner who can't run with the deep ball plays the nickel.
    let nickel = with(&[(Agility, 92.0), (Acceleration, 90.0), (Speed, 70.0), (ManCoverage, 70.0), (Press, 50.0)]);
    let (jn, _) = role(Position::CB, &nickel, 182.0, 69.0, &inst(0, 0, 0)).unwrap();
    assert_eq!(jn, Job::Nickel);
}

#[test]
fn a_club_can_pick_a_sleeper_its_own_board_loves() {
    use crate::world::pro::scouts::Board;
    use crate::world::recruiting::{DraftRun, candidates};
    // 500 men left in the public order; club 0's board loves the 450th.
    let pool: Vec<(usize, f64)> = (0..500).map(|i| (i, 90.0 - i as f64 * 0.05)).collect();
    let mut b = Board::default();
    for &(i, g) in &pool {
        b.grade.insert(i, if i == 449 { 95.0 } else { g });
    }
    let run = DraftRun { pool, boards: vec![b], ..Default::default() };
    let c = candidates(&run, 0);
    assert!(c.contains(&449), "his own board's favourite is in reach");
    assert_eq!(c.len(), 360, "the public 300 and the board's best 60 below them");
    assert!(c[..300].iter().copied().eq(0..300));
    // A club with no board weighs the public 300.
    assert_eq!(candidates(&run, 1).len(), 300);
}
