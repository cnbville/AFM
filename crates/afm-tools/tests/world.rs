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
    let searching =
        w.programs.iter().filter(|p| p.plays(w.year) && p.head_coach.is_none() && p.division == Division::FBS).count();
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
    let expected: usize =
        drafts.iter().map(|&y| 7 * 32 + w.market.picks.comp.iter().filter(|c| c.year == y).count()).sum();
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
    // The hard cap holds on opening day, and teams spend most of it.
    // Every year once the spin-up's first classes are seniors.
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

#[test]
fn people_live() {
    use afm_engine::player::Trait;
    use afm_engine::world::franchise::active_teams;
    use afm_engine::world::pro::money;
    let w = &*WORLD;
    // Section 6: every club and FBS program has its support heads and captains.
    for t in active_teams(w.year) {
        let c = &w.clubs[t];
        assert!(c.support.strength.rating > 0 && c.support.medical.rating > 0, "the {} lack staff", w.nfl[t].name);
        // Six voted; one who has left since the vote is off the list.
        assert!((3..=6).contains(&c.captains.len()), "the {} have {} captains", w.nfl[t].name, c.captains.len());
        assert!(
            c.captains.iter().all(|&i| w.people[i].pro.is_some()),
            "the {} captains point at non-pros",
            w.nfl[t].name
        );
    }
    let fbs: Vec<_> = w.programs.iter().filter(|p| p.division == Division::FBS && p.plays(w.year)).collect();
    assert!(fbs.iter().all(|p| p.support.academic.rating > 0));
    assert!(fbs.iter().filter(|p| p.captains.len() >= 3).count() * 10 >= fbs.len() * 9);
    // Traits average near 10 with a real spread.
    let traits: Vec<f64> =
        w.people.iter().take(50_000).map(|p| p.player.personality.traits.get(Trait::Professionalism)).collect();
    let mean = traits.iter().sum::<f64>() / traits.len() as f64;
    assert!((9.0..12.0).contains(&mean), "professionalism averages {mean:.2}");
    // Moods spread both ways among college players.
    let moods: Vec<f32> = w.people.iter().filter(|p| p.college.is_some() && p.pro.is_none()).map(|p| p.mood).collect();
    let low = moods.iter().filter(|&&m| m < -0.2).count() as f64 / moods.len() as f64;
    let high = moods.iter().filter(|&&m| m > 0.2).count() as f64 / moods.len() as f64;
    eprintln!("college moods: {:.1}% low, {:.1}% high of {}", 100.0 * low, 100.0 * high, moods.len());
    assert!(low > 0.015 && high > 0.015, "moods {low:.3} low, {high:.3} high");
    // Pros get into trouble at about the real rate, and some are suspended.
    let m = w.market.years.iter().find(|m| m.year == w.year - 1).unwrap();
    eprintln!(
        "{}: {} cases, {} suspensions, {} poached, {} settlements",
        m.year, m.trouble, m.suspensions, m.poached, m.settlements
    );
    assert!((15..=150).contains(&m.trouble), "{} cases in {}", m.trouble, m.year);
    // Fix list S6.3: no player is paid more than half the cap in a year.
    let cap = money::price_cap(w.year - 1);
    for p in w.people.iter() {
        if let Some(s) = p.pro.as_ref().and_then(|c| c.seasons.iter().find(|s| s.year == w.year - 1)) {
            assert!(s.pay <= 0.5 * cap, "{} paid {:.0} against a cap of {:.0}", p.name, s.pay, cap);
        }
    }
}

#[test]
fn clubs_scout_and_draft_from_their_own_boards() {
    use afm_engine::world::franchise::active_teams;
    use afm_engine::world::pro::scouts::Role;
    let w = &*WORLD;
    // Section 7: every club has a director, national and area scouts, and
    // its files hold only the coming draft class (the last rush cleared
    // the rest; the world ends in February, after the fall's school calls).
    let class: std::collections::HashSet<u32> = w
        .people
        .iter()
        .filter(|p| matches!(p.stage, afm_engine::world::Stage::College | afm_engine::world::Stage::Declared))
        .map(|p| p.player.id)
        .collect();
    for t in active_teams(w.year) {
        let s = &w.clubs[t].scouts;
        assert_eq!(s.iter().filter(|x| x.role == Role::Director).count(), 1, "the {} directors", w.nfl[t].name);
        assert!(s.iter().filter(|x| x.role == Role::National).count() >= 1);
        assert!((5..=10).contains(&s.iter().filter(|x| x.role == Role::Area).count()));
        let files = &w.clubs[t].files;
        assert!(!files.is_empty(), "the {} have no files on the coming class", w.nfl[t].name);
        assert!(files.keys().all(|id| class.contains(id)), "the {} kept files on a past class", w.nfl[t].name);
    }
    // The last draft: looks filed, players signed in the rush, picks
    // explained from each club's own board.
    let m = w.market.years.iter().rev().find(|m| m.udfa_signed > 0).expect("no undrafted signings");
    eprintln!(
        "{}: {} looks, {} draft-day trades, {} undrafted signed",
        m.year, m.scout_looks, m.draft_trades, m.udfa_signed
    );
    assert!(m.scout_looks > 1_000, "{} looks in {}", m.scout_looks, m.year);
    let own = w.people.iter().filter(|p| p.notes.iter().any(|n| n.contains("their board grade"))).count();
    assert!(own > 0, "no pick names its board");
}

/// A seed gives the same world on any number of cores (bug check, build
/// 9): two small worlds built on one core and on four match person by
/// person, draft pick by draft pick and in the NFL's money.
#[test]
fn a_seed_gives_the_same_world_on_any_core_count() {
    use std::hash::{Hash, Hasher};
    let build = |threads: usize| {
        let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).build().unwrap();
        pool.install(|| {
            let w = World::new(WorldOptions {
                seed: 11,
                scale: 0.05,
                history_years: 2,
                engine_from: Some(2100),
                ..Default::default()
            });
            let mut h = std::collections::hash_map::DefaultHasher::new();
            for p in &w.people {
                p.player.id.hash(&mut h);
                p.player.overall().to_bits().hash(&mut h);
                p.mood.to_bits().hash(&mut h);
                (p.stage as u8).hash(&mut h);
                p.college.hash(&mut h);
                p.pro.as_ref().map(|c| (c.team, c.contract.as_ref().map(|k| k.apy().to_bits()))).hash(&mut h);
            }
            for c in &w.clubs {
                c.captains.hash(&mut h);
                c.scouts.len().hash(&mut h);
            }
            for m in &w.market.years {
                (m.year, m.signed_new, m.trades, m.draft_trades, m.udfa_signed, m.trouble).hash(&mut h);
            }
            (h.finish(), w.people.len())
        })
    };
    let (one, n) = build(1);
    let (four, m) = build(4);
    eprintln!("{n} people, hashes {one:x} and {four:x}");
    assert_eq!(n, m);
    assert_eq!(one, four, "the same seed built a different world on 1 and 4 cores");
}

/// The history's franchise changes (trial 9 crashed in 1995, when the new
/// Panthers and Jaguars had a draft but no scouts) and every era of the
/// history the 3-year world doesn't reach: a small world from 1990 to
/// 2010 on the quick stand-in, through the expansion of 1995, the Browns'
/// move to Baltimore (1996), their return (1999), the Texans (2002) and
/// the uncapped year (2010). Every club that picks has a department,
/// every look in a file names someone who exists, and the returning
/// Browns hired a new department.
#[test]
fn a_history_through_the_expansion_years_runs() {
    use afm_engine::world::pro::scouts::{GM, Role};
    let w = World::new(WorldOptions {
        seed: 5,
        scale: 0.05,
        history_years: 9,
        start_year: 2011,
        engine_from: Some(2100),
        ..Default::default()
    });
    for (t, c) in w.clubs.iter().enumerate() {
        if !afm_engine::world::franchise::active(t, w.year) {
            continue;
        }
        let name = &w.nfl[t].name;
        assert!(c.scouts.iter().any(|s| s.role == Role::Director), "{name} have no scouting director");
        for (id, looks) in &c.files {
            for l in looks {
                assert!(l.who >= GM || (l.who as usize) < c.scouts.len(), "{name}: a look at {id} by scout {}", l.who);
            }
        }
    }
    // The Browns (7) went dark in 1996 and came back in 1999.
    assert!(w.clubs[7].scouts.iter().all(|s| s.since >= 1998), "the returning Browns kept scouts from before 1998");
    // Every year once the spin-up's first classes are seniors.
    for m in w.market.years.iter().filter(|m| (1995..=2010).contains(&m.year)) {
        assert!(m.scout_looks > 0, "no scouting in {}", m.year);
    }
}
