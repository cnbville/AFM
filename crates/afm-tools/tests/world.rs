//! Builds a real-size world with a short history and checks it hangs together:
//! full rosters, staffs in place, a draft that mostly comes from FBS, and
//! national titles won by the strongest programs.

use std::sync::LazyLock;

use afm_engine::world::data::Division;
use afm_engine::world::{World, WorldOptions};

// The game plays every college game on the engine; the test plays only the
// last history season on it, to keep the run short.
static WORLD: LazyLock<World> = LazyLock::new(|| {
    World::new(WorldOptions { seed: 7, history_years: 3, engine_from: Some(2025), ..Default::default() })
});

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
    // Real FCS share is about 6%, and the 30-year world gives about 6%. This
    // 3-year world runs high: 11.5-12.7% on seeds 7-9 before the coaching
    // market build (2026-10-15), 12.5-13.0% after it, so the bound is 14%.
    assert!((0.02..0.14).contains(&share(Division::FCS)), "FCS share {:.2}", share(Division::FCS));
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

/// Section 4: a football year on the calendar hangs together.
#[test]
fn the_calendar_year_hangs_together() {
    let w = &*WORLD;
    let log = w.log_history.iter().find(|l| l.year == 2025).expect("the 2025 football year");
    // Commitments are announced through the year, not on a few days (T5.2).
    let months = log.commits_by_month.iter().filter(|&&c| c > 0).count();
    assert!(months >= 7, "{:?}", log.commits_by_month);
    // FBS signees mostly sign in December's early period (2017 on).
    let fbs = log.signed_by_division[Division::FBS as usize];
    assert!(fbs[0] > 3 * fbs[1], "{fbs:?}");
    // Real early entrants granted eligibility, 2022-2025: about 58-95.
    assert!((40..=130).contains(&log.declared), "{}", log.declared);
    // The portal fills over many days, not one (T3, T9).
    assert!(log.portal_by_day.len() >= 10, "{:?}", log.portal_by_day);
    // A full NFL year: 272 regular-season games and 13 playoff games.
    assert_eq!(log.nfl_games, 285);
    assert!(log.official_visits > 0 && log.unofficial_visits > 0 && log.drop_ins > 0);
}
