//! Update 3's code fixes (lane 3): history rule books, college overtime,
//! designed plays on film and in the play caller's own calls, the college
//! expected points and game injuries that carry into the week.

use crate::calendar::Date;
use crate::game::designer::{Design, save};
use crate::game::flow::quick_game;
use crate::game::playbook::*;
use crate::game::rules::Overtime;
use crate::game::sim::{Injury, InjuryKind};
use crate::game::synthetic::synthetic_team;
use crate::game::{OffScheme, Rules, tables};
use crate::player::Level;
use crate::world::{World, WorldOptions};

/// Each era plays its own book: the NFL's kickoff spot, touchback, extra
/// point and overtime; college overtime's shootout from 2019 (F25, F26).
#[test]
fn rule_books_follow_history() {
    let old = Rules::for_year(Level::NFL, 2005, false);
    assert_eq!((old.kickoff_from, old.kickoff_touchback, old.xp_from), (30.0, 20.0, 2.0));
    assert_eq!(old.overtime, Overtime::SuddenDeath);
    assert_eq!(Rules::for_year(Level::NFL, 2011, true).overtime, Overtime::NflModified { minutes: 15 });
    assert_eq!(Rules::for_year(Level::NFL, 2020, false).overtime, Overtime::NflModified { minutes: 10 });
    assert_eq!(Rules::for_year(Level::NFL, 2020, true).overtime, Overtime::NflModified { minutes: 15 });
    let now = Rules::for_year(Level::NFL, 2025, false);
    assert!(now.nfl_kickoff && now.overtime == Overtime::NflPeriod && now.xp_from == 15.0);
    assert_eq!(Rules::for_year(Level::FBS, 2015, false).college_ot(), Some((3, 99)));
    assert_eq!(Rules::for_year(Level::FBS, 2019, false).college_ot(), Some((3, 5)));
    assert_eq!(Rules::for_year(Level::FBS, 2024, false).college_ot(), Some((2, 3)));
    assert_eq!(Rules::for_year(Level::NFL, 0, false).overtime, Rules::for_level(Level::NFL).overtime);
}

/// Below the NFL fourth downs and EPA read the real college table (F30).
#[test]
fn college_reads_its_own_expected_points() {
    let nfl = tables::expected_points_in(false, 1, 10.0, 75.0);
    let college = tables::expected_points_in(true, 1, 10.0, 75.0);
    assert!((nfl - 1.03).abs() < 0.05 && (college - 0.71).abs() < 0.05, "{nfl} {college}");
    assert!(tables::expected_points_in(true, 1, 10.0, 5.0) > tables::expected_points_in(true, 1, 10.0, 50.0));
}

/// A designed play is filed on film under the library concept it looks
/// like, and a play caller with it on his sheet calls it himself (F27,
/// F28).
#[test]
fn designed_plays_go_on_film_and_into_the_calls() {
    let d = Design {
        name: "lane three smash".into(),
        formation: Formation::plain(Personnel::P11, Backfield::Shotgun, 1.0),
        routes: vec![(ORole::Z, Route::Hitch(5.0)), (ORole::Slot, Route::Corner(12.0))],
        reads: vec![ORole::Slot, ORole::Z],
        protection: Protection::Standard,
        drop: Drop::Quick,
        play_action: false,
    };
    let k = save(&d, Level::NFL).expect("legal");
    assert_eq!(pass_concept(film_concept(k)).name, "smash");
    assert_eq!(film_concept(concept("mesh")), concept("mesh"));
    let mut a = synthetic_team("A", Level::NFL, 0.0, OffScheme::WestCoast, 11);
    let b = synthetic_team("B", Level::NFL, 0.0, OffScheme::WestCoast, 22);
    // On the sheet in a box no snap reaches, so only the caller's own
    // calls can use it.
    a.prep.sheet = vec![(250, vec![(Personnel::P11, Backfield::Shotgun, Set::Doubles, PlayKind::Pass(k))])];
    let mut called = 0;
    for seed in 0..6 {
        let g = quick_game(&a, &b, Level::NFL, seed);
        called += g.stats.plays.iter().filter(|p| p.call.contains("lane three smash")).count();
    }
    assert!(called > 0, "the design was never called");
}

/// A college or high school game injury keeps the player out after the
/// game, on your team and on others alike; one who played on is not out
/// (A1).
#[test]
fn game_injuries_carry_into_the_week() {
    let mut w = World::empty(WorldOptions { scale: 0.002, ..Default::default() });
    let day = Date::ymd(2026, 9, 12);
    let hurt = |id: u32, side: usize, weeks: f64, plays_on: bool| {
        (side, Injury { side, p: 0, id, kind: InjuryKind::AnkleSprain, weeks, plays_on })
    };
    let list = [hurt(7, 0, 2.0, false), hurt(8, 1, 1.0, false), hurt(9, 1, 3.0, true)];
    crate::training::seat::game_injuries(&mut w, day, &list, [0.0; 2], Some(0));
    assert!(w.training.hurt_out(7, day.add(10)) && !w.training.hurt_out(7, day.add(20)));
    assert!(w.training.hurt_out(8, day.add(5)) && w.training.other_hurt(8, day.add(5)).is_some());
    assert!(!w.training.hurt_out(9, day.add(1)));
}
