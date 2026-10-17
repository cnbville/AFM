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
    // A head job is empty only while its search runs (part D 18); the
    // world stops in February, when the last links of the carousel's
    // chains (and jobs left for the NFL in late January) are still open.
    // A program that doesn't field a team this year needs no coach.
    let no_coach: Vec<_> = w
        .programs
        .iter()
        .filter(|p| p.plays(w.year) && p.head_coach.is_none() && !w.head_search_open(p.id))
        .map(|p| format!("{} ({:?}, plays {})", p.real_name, p.division, p.plays(w.year)))
        .collect();
    assert!(no_coach.is_empty(), "no head coach and no search: {no_coach:?} on {:?}", w.today);
    let searching = w.programs.iter().filter(|p| p.head_coach.is_none() && p.division == Division::FBS).count();
    assert!(searching <= 4, "{searching} FBS head jobs still open on {:?}", w.today);
    assert!(w.high_schools.iter().all(|s| s.coach.is_some()));
}

#[test]
fn the_draft_comes_mostly_from_fbs() {
    let w = &*WORLD;
    let picks: Vec<_> = w.alumni.iter().filter(|a| a.pick.is_some()).collect();
    // Seven rounds for 32 teams, plus the compensatory picks awarded that
    // year (32 from 1994, and diversity picks from 2021).
    let drafts: std::collections::BTreeSet<i32> = picks.iter().map(|a| a.year).collect();
    assert_eq!(drafts.len(), 3);
    let expected: usize = drafts
        .iter()
        .map(|&y| 7 * 32 + w.market.picks.comp.iter().filter(|c| c.year == y).count())
        .sum();
    assert_eq!(picks.len(), expected);
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

#[test]
fn the_stats_book_keeps_every_division() {
    let w = &*WORLD;
    let b = &w.stats;
    let (last, _) = *b.year_index.last().expect("a season in the book");
    let recs = b.year(last);
    for d in Division::ALL {
        let n = recs.iter().filter(|r| r.division == d).count();
        assert!(n > 300, "{n} {d:?} seasons in {last}");
    }
    assert!(recs.iter().any(|r| r.footprint != 0), "no footprints in {last}");
    assert!(b.norms.keys().any(|k| k.0 == last));
    assert!(!b.books.is_empty());
    // The first-team All-America: 25 a level.
    let fbs =
        b.honors.values().flat_map(|h| h.all_america.iter()).filter(|x| x.0 == last && x.1 == Division::FBS).count();
    assert_eq!(fbs, 25, "FBS first-team All-Americans in {last}");
    // Every season is a career page: his seasons come back in order.
    let r = &recs[0];
    let career = b.career(r.id);
    assert!(career.iter().any(|s| s.year as i32 == last && s.program == r.program));
}

#[test]
fn the_nfl_lives() {
    use afm_engine::world::franchise::active_teams;
    use afm_engine::world::pro::money;
    use afm_engine::world::pro::trades::CompKind;
    let w = &*WORLD;
    // Every team holds an offseason roster of people.
    for t in active_teams(w.year) {
        let held = w.clubs[t].roster.len();
        assert!((45..=95).contains(&held), "the {} hold {held} players", w.nfl[t].name);
    }
    // The hard cap holds on opening day.
    for m in w.market.years.iter().filter(|m| money::capped(m.year) && m.cap_use[1] > 0.0) {
        assert!(m.cap_use[2] <= 1.001, "{}: a team at {:.3} of the cap on opening day", m.year, m.cap_use[2]);
        assert!(m.cap_use[1] >= 0.8, "{}: the median team at {:.3} of the cap", m.year, m.cap_use[1]);
    }
    // Compensatory picks: 32 a year (supplemental picks fill what free
    // agency leaves), at most four a team for free agency.
    let years: std::collections::BTreeSet<i32> = w.market.picks.comp.iter().map(|c| c.year).collect();
    assert!(!years.is_empty());
    for y in years {
        let of = |k: CompKind| w.market.picks.comp.iter().filter(move |c| c.year == y && c.kind == k);
        assert_eq!(of(CompKind::FreeAgency).count() + of(CompKind::Supplemental).count(), 32, "{y}");
        for t in active_teams(y) {
            assert!(of(CompKind::FreeAgency).filter(|c| c.team == t).count() <= 4, "{y}");
        }
    }
    // A full league year moves players: free agents to new teams, trades.
    let m = w.market.years.iter().find(|m| m.year == w.year - 1).unwrap();
    assert!(m.signed_new >= 50, "{} free agents changed teams in {}", m.signed_new, m.year);
    assert!(m.trades >= 1, "no trades in {}", m.year);
    assert!(m.futures >= 100, "{} reserve/future deals in {}", m.futures, m.year);
}
