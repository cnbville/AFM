//! Growth from the depth chart (minor bug 247, lane NB12): your depth
//! chart sets your players' roles for the year's step, a step reads last
//! season's real snaps at its own level, and the snaps move a year's
//! growth as the NFL's do.

use super::{career, context_for, mean, prospects};
use crate::development::develop_season;
use crate::player::{Level, Player, PlayingTime};
use crate::positions::Position;
use crate::rng::Rng;
use crate::world::season::{depth_chart, reps_read, sat_season, your_order};
use crate::world::{Reps, Stage};

fn role_of(chart: &[(usize, PlayingTime)], i: usize) -> PlayingTime {
    chart.iter().find(|c| c.0 == i).unwrap().1
}

#[test]
fn your_depth_chart_sets_the_roles_on_your_team() {
    use PlayingTime::*;
    // Quarterbacks: a senior, a junior and a true freshman the staff means
    // to redshirt. Receivers: seven, the last a redshirt too. Corners: three.
    let mut entries = vec![
        (0, Position::QB, 70.0),
        (1, Position::QB, 65.0),
        (2, Position::QB, 60.0),
        (3, Position::WR, 72.0),
        (4, Position::WR, 70.0),
        (5, Position::WR, 68.0),
        (6, Position::WR, 66.0),
        (7, Position::WR, 64.0),
        (8, Position::WR, 55.0),
        (12, Position::WR, 62.0),
        (9, Position::CB, 71.0),
        (10, Position::CB, 69.0),
        (11, Position::CB, 60.0),
    ];
    let mut chart = depth_chart(&mut entries, None, &|_| false, &|_| false);
    for c in chart.iter_mut().filter(|c| c.0 == 2 || c.0 == 8) {
        c.1 = Redshirt;
    }
    let staff = chart.clone();
    assert_eq!((role_of(&staff, 0), role_of(&staff, 1)), (Starter, Rotation));
    let id = |i: usize| 100 + i as u32;
    // You start the freshman, list the fifth receiver as your top corner,
    // and order the receivers with the redshirt seventh, past the two-deep.
    let order = vec![
        (Position::QB, vec![id(2)]),
        (Position::CB, vec![id(7)]),
        (Position::WR, vec![id(3), id(4), id(5), id(6), id(7), id(12), id(8)]),
    ];
    your_order(&mut chart, &entries, &order, &id);
    // The freshman starts and isn't redshirted; the senior drops to second.
    assert_eq!(role_of(&chart, 2), Starter);
    assert_eq!(role_of(&chart, 0), Rotation);
    assert_eq!(role_of(&chart, 1), Backup);
    // The fifth receiver starts at corner; the staff's two corners move down.
    assert_eq!(role_of(&chart, 7), Starter);
    assert_eq!(role_of(&chart, 9), Starter);
    assert_eq!(role_of(&chart, 10), Rotation);
    assert_eq!(role_of(&chart, 11), Backup);
    // A redshirt you list past the two-deep keeps the staff's call; one you
    // list inside it plays.
    assert_eq!(role_of(&chart, 8), Redshirt);
    let mut inside = staff.clone();
    your_order(&mut inside, &entries, &[(Position::WR, vec![id(3), id(4), id(5), id(6), id(8)])], &id);
    assert_eq!(role_of(&inside, 8), Backup);
    // No order: the staff's chart stands.
    let mut same = staff.clone();
    your_order(&mut same, &entries, &[], &id);
    assert_eq!(same, staff);
    // At your high school you moved the staff's top receiver to JV: your
    // varsity order leaves him off and ranks the varsity players alone, so
    // he keeps the staff's place (his JV starter's role) and the next two
    // start on varsity.
    let varsity: Vec<_> = entries.iter().copied().filter(|e| e.0 != 3).collect();
    let mut jv = staff.clone();
    your_order(&mut jv, &varsity, &[(Position::WR, vec![id(5), id(4)])], &id);
    assert_eq!(role_of(&jv, 3), Starter);
    assert_eq!((role_of(&jv, 5), role_of(&jv, 4)), (Starter, Starter));
}

#[test]
fn a_seasons_row_says_what_he_really_played() {
    use crate::world::college::real_role;
    use PlayingTime::*;
    // Cain 2026-11-07: the camp ranking started a receiver who caught one
    // ball in ten games; his snaps came on special teams.
    assert_eq!(real_role(0, 10, 13, 0.03), Backup);
    // Started every game, or most of them.
    assert_eq!(real_role(13, 13, 13, 0.95), Starter);
    assert_eq!(real_role(7, 12, 13, 0.6), Starter);
    // Won the job at midseason: six starts in twelve games.
    assert_eq!(real_role(6, 12, 12, 0.45), Starter);
    // A starter hurt in the opener, and one hurt after four starts.
    assert_eq!(real_role(1, 1, 12, 0.05), Starter);
    assert_eq!(real_role(4, 4, 12, 0.3), Starter);
    // A third receiver the first snap's personnel left off: heavy snaps.
    assert_eq!(real_role(2, 13, 13, 0.62), Starter);
    // A backup who filled in for two starts in three games played.
    assert_eq!(real_role(2, 3, 13, 0.18), Backup);
    // A starter benched after five starts who kept playing.
    assert_eq!(real_role(5, 12, 12, 0.4), Rotation);
    // A rotation lineman: never started, a third of the snaps.
    assert_eq!(real_role(0, 12, 12, 0.35), Rotation);
}

#[test]
fn a_season_off_leaves_the_step_to_the_role() {
    use PlayingTime::*;
    // College: a planned redshirt keeps the year with four games or fewer.
    assert!(sat_season(Some(Redshirt), 4, false, 4));
    assert!(!sat_season(Some(Redshirt), 5, false, 4));
    // A suspension that kept him out of every game, not one that came after
    // he had played.
    assert!(sat_season(Some(Backup), 0, true, 4));
    assert!(!sat_season(Some(Backup), 3, true, 4));
    // A bench player who never got in reads his zero snaps.
    assert!(!sat_season(Some(Backup), 0, false, 4));
    // High school: a transfer's sit year is no games at all.
    assert!(sat_season(Some(Redshirt), 0, false, 0));
    assert!(!sat_season(Some(Redshirt), 1, false, 0));
}

#[test]
fn a_step_reads_last_seasons_reps_at_its_own_level() {
    let college = Some(Reps { year: 2030, college: true, share: 0.8 });
    let hs = Some(Reps { year: 2030, college: false, share: 0.6 });
    let read = |r: Option<Reps>, stage: Stage, role: PlayingTime, trouble: bool, pos: Position| {
        reps_read(r, 2031, stage, role, trouble, pos)
    };
    let s = PlayingTime::Starter;
    assert_eq!(read(college, Stage::College, s, false, Position::QB), Some(0.8f32 as f64));
    assert_eq!(read(hs, Stage::HighSchool, s, false, Position::QB), Some(0.6f32 as f64));
    // A freshman's first college step reads his depth chart, not his high school snaps.
    assert_eq!(read(hs, Stage::College, s, false, Position::QB), None);
    assert_eq!(read(college, Stage::HighSchool, s, false, Position::QB), None);
    // Two seasons old (a year away), a redshirt or sit year, a suspension,
    // a kicker, a prep year: the role decides.
    assert_eq!(reps_read(college, 2032, Stage::College, s, false, Position::QB), None);
    assert_eq!(read(college, Stage::College, PlayingTime::Redshirt, false, Position::QB), None);
    assert_eq!(read(college, Stage::College, s, true, Position::QB), None);
    assert_eq!(read(college, Stage::College, s, false, Position::K), None);
    assert_eq!(read(hs, Stage::PrepSchool, PlayingTime::Rotation, false, Position::QB), None);
}

/// One FBS step at his age with the given role and snaps; his overall gain.
fn step(p: &Player, seed: u64, age: u32, role: PlayingTime, snaps: Option<f64>) -> (Player, f64) {
    let mut q = p.clone();
    let mut rng = Rng::new(format!("reps-{seed}-{age}").as_str());
    let mut ctx = context_for(age);
    ctx.level = Level::FBS;
    ctx.coaching = 13.0;
    ctx.playing_time = role;
    ctx.snaps = snaps;
    let before = q.overall();
    develop_season(&mut q, &ctx, &mut rng);
    let gain = q.overall() - before;
    (q, gain)
}

/// The same 4,000 prospects the bug was measured on (talent z 1.5, to 18
/// with the test helpers), each case on the same random streams.
fn freshmen() -> Vec<Player> {
    let mut rng = Rng::new("reps-to-18");
    prospects(4000, "reps", 1.5)
        .into_iter()
        .map(|mut p| {
            career(&mut p, &mut rng, 18, |_| {});
            p
        })
        .collect()
}

#[test]
fn the_snaps_decide_a_returning_players_year() {
    use PlayingTime::*;
    let ps = freshmen();
    let gain = |age: u32, role: PlayingTime, snaps: Option<f64>| {
        mean(&ps.iter().enumerate().map(|(k, p)| step(p, k as u64, age, role, snaps).1).collect::<Vec<_>>())
    };
    // A true freshman: no college snaps yet, so the chart decides.
    let (fs, fb) = (gain(18, Starter, None), gain(18, Backup, None));
    // A returning player: last season's snaps decide whatever his role.
    let (full, sat, none) = (gain(18, Starter, Some(1.0)), gain(18, Starter, Some(0.05)), gain(18, Starter, None));
    let (benched, buried) = (gain(18, Backup, Some(1.0)), gain(18, Backup, Some(0.05)));
    eprintln!("true freshman: starter {fs:+.2}, backup {fb:+.2}");
    eprintln!("starter now: last season full {full:+.2}, a backup's {sat:+.2}, no snaps read {none:+.2}");
    eprintln!("backup now: last season full {benched:+.2}, a backup's {buried:+.2}");
    assert!(fs > fb + 0.5, "your starting freshman grows as a starter");
    assert!(full > sat + 0.4 && benched > buried + 0.4, "last season's snaps move growth");
    assert!((full - none).abs() < 0.15, "a full season reads about as the starter class");
    assert!((benched - full).abs() < 0.3, "the role keeps only breakouts and injuries");
}

#[test]
fn four_college_years_with_the_snaps_a_year_behind() {
    use PlayingTime::*;
    let ps = freshmen();
    // Each year's role, and the snaps last season left (a step reads them
    // a year behind, as the NFL does): a full season 1.0, a backup's 0.05.
    let path = |roles: [(PlayingTime, Option<f64>); 4]| {
        mean(
            &ps.iter()
                .enumerate()
                .map(|(k, p)| {
                    let mut q = p.clone();
                    for (y, &(role, snaps)) in roles.iter().enumerate() {
                        q = step(&q, k as u64, 18 + y as u32, role, snaps).0;
                    }
                    q.overall() - p.overall()
                })
                .collect::<Vec<_>>(),
        )
    };
    let all = path([(Starter, None), (Starter, Some(1.0)), (Starter, Some(1.0)), (Starter, Some(1.0))]);
    let one = path([(Backup, None), (Starter, Some(0.05)), (Starter, Some(1.0)), (Starter, Some(1.0))]);
    let two = path([(Backup, None), (Backup, Some(0.05)), (Starter, Some(0.05)), (Starter, Some(1.0))]);
    eprintln!("by 22: start four {all:+.2}, sit one {one:+.2}, sit two {two:+.2}");
    assert!(all > one && one > two);
}
