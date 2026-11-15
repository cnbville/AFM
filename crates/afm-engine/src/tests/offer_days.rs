//! AI schools' offers and commitments on real days (thread "Make AI
//! schools offer on real days"): the weekly offers to underclassmen climb
//! between the old passes and match them on the passes' own days, the
//! recruiting weeks of the class signing next run every seventh day and on
//! the services' days, and a summer committer's own day falls inside his
//! window.

use crate::calendar::{Date, YearDates};
use crate::world::recruiting::{EARLY_OFFER_PASSES, commit_day, early_want, first_wave, visited_leader};
use crate::world::{CommitTiming, Visit, World, WorldOptions, clock};

#[test]
fn early_offers_climb_week_by_week() {
    let season = 2024;
    let jd = YearDates::new(season).junior_days.0;
    // The class of 2026 is offered from January 2024 to its junior days.
    let class_of = season + 2;
    assert!(early_want(season, jd, class_of, Date::ymd(2024, 1, 8)).is_none(), "before the first pass");
    let jan = early_want(season, jd, class_of, Date::ymd(2024, 1, 15)).unwrap();
    assert!(jan.2, "a pass's day is a services' look");
    assert!((jan.0 - EARLY_OFFER_PASSES[0].0).abs() < 1e-9 && (jan.1 - EARLY_OFFER_PASSES[0].1).abs() < 1e-9);
    let jun = early_want(season, jd, class_of, Date::ymd(2024, 6, 15)).unwrap();
    assert!((jun.0 - EARLY_OFFER_PASSES[1].0).abs() < 1e-9);
    // Mondays between the passes climb, and aren't looks.
    let mut d = Date::ymd(2024, 1, 16);
    let mut last = jan.0;
    let mut mondays = 0;
    while d < Date::ymd(2024, 6, 15) {
        if let Some((most, _, look)) = early_want(season, jd, class_of, d) {
            assert!(!look && most >= last && most <= jun.0, "{} {most}", d.pretty());
            last = most;
            mondays += 1;
        }
        d = d.add(1);
    }
    assert!((20..=22).contains(&mondays), "{mondays}");
    // Nothing goes out from the class's first wave on.
    assert!(early_want(season, jd, season + 1, first_wave(season + 1)).is_none());
}

#[test]
fn recruiting_weeks_and_services_days() {
    let y = YearDates::new(2025);
    assert_eq!(clock::recruiting_week(&y, Date::ymd(2025, 3, 20)), Some(0));
    assert_eq!(clock::recruiting_week(&y, Date::ymd(2025, 3, 27)), Some(0));
    assert_eq!(clock::recruiting_week(&y, Date::ymd(2025, 3, 21)), None);
    // The services' days run whether or not they fall on the week.
    assert_eq!(clock::recruiting_week(&y, Date::ymd(2025, 4, 30)), Some(1));
    assert_eq!(clock::recruiting_week(&y, Date::ymd(2025, 6, 15)), Some(2));
    assert_eq!(clock::recruiting_week(&y, Date::ymd(2025, 10, 31)), Some(4));
    assert_eq!(clock::recruiting_week(&y, Date::ymd(2025, 3, 13)), None);
    assert_eq!(clock::recruiting_week(&y, Date::ymd(2025, 12, 4)), None);
    let mut n = 0;
    let mut d = Date::ymd(2025, 3, 1);
    while d < Date::ymd(2025, 12, 31) {
        n += clock::recruiting_week(&y, d).is_some() as u32;
        d = d.add(1);
    }
    assert_eq!(n, 41, "37 weeks and four services' days off the week");
}

#[test]
fn a_summer_committer_decides_on_his_own_day() {
    let mut w = World::empty(WorldOptions { seed: 7, scale: 0.001, history_years: 0, ..Default::default() });
    w.add_class_aged(17);
    let spans = [40, 45, 68, 58, 30];
    let mut days = std::collections::HashSet::new();
    let mut p = w.people.iter().find(|p| p.stage == crate::world::Stage::HighSchool).unwrap().clone();
    for id in 0..400u32 {
        p.player.id = id;
        let wave = (id % 5) as u8;
        p.cal.commit_timing = Some(CommitTiming::Summer(wave));
        let d = commit_day(7, &p).unwrap();
        let (m, day) = clock::COMMIT_WAVES[wave as usize];
        let start = Date::ymd(p.class_of - 1, m, day);
        let july4 = Date::ymd(p.class_of - 1, 7, 4);
        assert!(d == july4 || (d > start && start.days_to(d) <= spans[wave as usize]), "wave {wave}: {}", d.pretty());
        days.insert(d.0);
    }
    assert!(days.len() > 150, "his own day, not a wave's: {}", days.len());
    p.cal.commit_timing = Some(CommitTiming::Early);
    assert!(commit_day(7, &p).is_none());
}

#[test]
fn an_official_to_the_leader_ends_the_wait() {
    let mut w = World::empty(WorldOptions { seed: 7, scale: 0.001, history_years: 0, ..Default::default() });
    w.add_class_aged(17);
    let mut p = w.people.iter().find(|p| p.stage == crate::world::Stage::HighSchool).unwrap().clone();
    p.cal.visits.clear();
    let race = [(3usize, 0.5), (8usize, 0.45)];
    let d = Date::ymd(2026, 6, 5);
    assert!(!visited_leader(&p, &race), "no visits");
    p.cal.visits.push((3, d, Visit::Unofficial));
    p.cal.visits.push((8, d, Visit::Official));
    assert!(!visited_leader(&p, &race), "an unofficial to the leader, an official to the next");
    p.cal.visits.push((3, d, Visit::Official));
    assert!(visited_leader(&p, &race));
    assert!(!visited_leader(&p, &[]), "no race");
}
