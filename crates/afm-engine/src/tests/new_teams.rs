//! New teams after 2026 (next update, lane 10, SK68): an NFL expansion
//! club through its lists, its expansion draft, its draft places and a
//! schedule; a college through its build year; a new high school's zone on
//! the first day; the yearly rates.

use crate::calendar::YearDates;
use crate::rng::Rng;
use crate::world::data::Division;
use crate::world::franchise;
use crate::world::new_teams::{college, ground_zero, nfl, school};
use crate::world::{Stage, World, WorldOptions};

fn world() -> World {
    World::empty(WorldOptions { scale: 0.01, history_years: 0, start_year: 2026, ..Default::default() })
}

#[test]
fn expansion_clubs_draft_their_squads_and_play_a_full_schedule() {
    let mut w = world();
    let year = w.year;
    let before = franchise::active_teams_in(&w, year).len();
    let markets = ground_zero::nfl_markets(&w);
    assert!(markets.len() >= 2, "open markets: {markets:?}");
    let new = nfl::award(&mut w, &markets[..2], year);
    assert_eq!(new.len(), 2);
    assert_eq!(franchise::active_teams_in(&w, year).len(), before + 2);
    assert_ne!(
        franchise::alignment_in(&w, new[0], year).map(|a| a.0),
        franchise::alignment_in(&w, new[1], year).map(|a| a.0),
        "one club in each conference"
    );
    // Your staff's guess at the lists, then the real lists and the draft.
    let guess = ground_zero::open_guess(&w, new[0]);
    let d = nfl::lists_day(&w);
    nfl::day(&mut w, d);
    let listed = w.expansion.lists.len();
    assert_eq!(listed, before * 6, "six open players a club when two draft");
    let same = guess.iter().filter(|g| w.expansion.lists.contains(&(g.0, w.people[g.1].player.id))).count();
    assert!(same * 2 >= listed, "the staff's guess mostly matches the lists: {same} of {listed}");
    let d = nfl::draft_day(&w);
    nfl::day(&mut w, d);
    for &t in &new {
        let n = w.clubs[t].roster.len();
        assert!((20..=42).contains(&n), "club {t} drafted {n}");
        assert!(w.expansion.club(t).is_some_and(|c| c.drafted));
    }
    // Their first draft: the new clubs pick first and last in alternate rounds.
    let base: Vec<usize> = franchise::active_teams_in(&w, year).into_iter().filter(|t| !new.contains(t)).collect();
    let r1 = franchise::round_order(year, 1, &base, &new);
    assert_eq!(&r1[..2], &new[..], "the new clubs pick first in round 1");
    let r2 = franchise::round_order(year, 2, &base, &new);
    assert_eq!(&r2[..2], &[new[1], new[0]], "and swap in round 2");
    assert_eq!(r1.len(), base.len() + 2);
    // A season with 34 clubs: every club plays 17 games.
    let s = crate::world::nfl_year::new_season(&w);
    for &t in &franchise::active_teams_in(&w, year) {
        let games = s.weeks.iter().flatten().filter(|g| g.0 == t || g.1 == t).count();
        assert_eq!(games, 17, "club {t} has {games} games");
    }
}

#[test]
fn a_college_starts_with_a_build_year_and_plays_the_next() {
    let mut w = world();
    let year = w.year;
    let p = ground_zero::college_candidates(&w)
        .into_iter()
        .find(|&p| matches!(w.programs[p].division, Division::FCS | Division::FBS))
        .expect("a Division I school without football");
    college::start(&mut w, p, Division::FCS, year + 1, year + 2);
    let prog = &w.programs[p];
    assert!(!prog.plays(year));
    assert!(prog.plays(year + 1) && !prog.fields_games(year + 1), "the build year fields a team and no games");
    assert!(prog.fields_games(year + 2));
    assert_eq!(prog.division, Division::FCS);
    assert!(w.new_teams.build_year(p, year + 1));
    // Its first schedule has games; the build year has none.
    let build = crate::world::schedule::build(&w, year + 1, &YearDates::new(year + 1));
    assert!(!build.iter().any(|g| g.home == p || g.away == p), "no games in the build year");
    let first = crate::world::schedule::build(&w, year + 2, &YearDates::new(year + 2));
    assert!(first.iter().any(|g| g.home == p || g.away == p), "games in its first season");
}

#[test]
fn a_new_school_takes_its_zones_underclassmen() {
    let mut w = world();
    let year = w.year;
    // Players for the zone: a full roster of every class at the parent.
    w.add_class_aged(17);
    w.add_class_aged(16);
    w.add_class_aged(15);
    w.rebuild_rosters();
    let parent = ground_zero::school_candidates(&w)
        .into_iter()
        .max_by_key(|&h| w.high_schools[h].roster.len())
        .expect("a big public school in a growing state");
    let enrollment = w.high_schools[parent].enrollment;
    let mut rng = Rng::new(7);
    let h = school::open(&mut w, parent, year, &mut rng);
    assert!(w.states[w.high_schools[h].state].schools.contains(&h));
    assert_eq!(w.high_schools[h].enrollment + w.high_schools[parent].enrollment, enrollment);
    let zone = school::zone(&w, h);
    assert!(!zone.is_empty(), "the zone has players");
    let before: Vec<(usize, i32)> = zone.iter().map(|&(i, _)| (i, w.people[i].class_of - year)).collect();
    assert!(school::class_day(&mut w));
    w.rebuild_rosters();
    let n = w.new_teams.school(h).unwrap();
    assert!(n.zoned && n.moved > 0);
    for (i, left) in before {
        let p = &w.people[i];
        assert_eq!(p.stage, Stage::HighSchool);
        match left {
            3 => assert_eq!(p.hs, h, "a zoned sophomore moves"),
            1 => assert_eq!(p.hs, parent, "a senior stays"),
            _ => {}
        }
    }
}

#[test]
fn new_teams_come_at_real_rates() {
    let w = World::empty(WorldOptions { scale: 1.0, history_years: 0, start_year: 2026, ..Default::default() });
    // About six colleges a year (NFF), from a pool that lasts decades.
    let pool = ground_zero::college_candidates(&w).len();
    assert!(pool >= 200, "candidate colleges: {pool}");
    // High schools: half the school-age growth of the growing states.
    let mean: f64 = (0..w.states.len())
        .map(|s| {
            let publics = w.states[s].schools.iter().filter(|&&h| !w.high_schools[h].private).count();
            0.5 * publics as f64 * school::young_growth(s)
        })
        .sum();
    eprintln!("new high schools a year: {mean:.1}");
    assert!((5.0..80.0).contains(&mean), "new high schools a year: {mean:.1}");
    assert!(school::young_growth(43) > 0.005, "Texas grows");
    assert_eq!(school::young_growth(48), 0.0, "West Virginia shrinks");
}
