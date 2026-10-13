//! Builds a real-size world with a short history and checks it hangs together:
//! full rosters, staffs in place, a draft that mostly comes from FBS, and
//! national titles won by the strongest programs.

use std::sync::LazyLock;

use afm_engine::world::data::Division;
use afm_engine::world::{World, WorldOptions};

static WORLD: LazyLock<World> =
    LazyLock::new(|| World::new(WorldOptions { seed: 7, history_years: 3, ..Default::default() }));

#[test]
fn rosters_and_staffs_are_full() {
    let w = &*WORLD;
    let fbs: Vec<_> = w.programs.iter().filter(|p| p.division == Division::FBS).collect();
    let avg = fbs.iter().map(|p| p.roster.len()).sum::<usize>() as f64 / fbs.len() as f64;
    assert!((80.0..=105.0).contains(&avg), "FBS rosters average {avg:.0}");
    assert!(w.programs.iter().all(|p| p.head_coach.is_some()));
    assert!(w.high_schools.iter().all(|s| s.coach.is_some()));
}

#[test]
fn the_draft_comes_mostly_from_fbs() {
    let w = &*WORLD;
    let picks: Vec<_> = w.alumni.iter().filter(|a| a.pick.is_some()).collect();
    assert_eq!(picks.len(), 257 * 3);
    let share = |d: Division| {
        picks.iter().filter(|a| w.programs[*a.colleges.last().unwrap()].division == d).count() as f64
            / picks.len() as f64
    };
    assert!(share(Division::FBS) > 0.85, "FBS share {:.2}", share(Division::FBS));
    // Real FCS share is about 6%.
    assert!((0.02..0.12).contains(&share(Division::FCS)), "FCS share {:.2}", share(Division::FCS));
}

#[test]
fn champions_are_strong_programs() {
    let w = &*WORLD;
    for p in w.programs.iter().filter(|p| p.division == Division::FBS) {
        for s in p.seasons.iter().filter(|s| s.national_title) {
            assert!(s.wins >= 10, "{} won a title at {}-{}", p.real_name, s.wins, s.losses);
        }
    }
}
