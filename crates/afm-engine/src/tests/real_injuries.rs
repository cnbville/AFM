//! Realism audit items 1 and 14: the yearly step's injury cost comes from
//! the injuries he really had, and rare complications run past the usual
//! ranges.

use super::context_for;
use crate::attributes::Attr;
use crate::calendar::Date;
use crate::development::{YearHurt, develop_season, hurt_class, take_why};
use crate::game::sim::{InjuryKind as K, complication, injury_weeks};
use crate::player::{DevelopmentEvent as Ev, ProspectOptions, generate_prospect};
use crate::rng::Rng;

fn hurt(kind: K, days: u32) -> YearHurt {
    let mut h = YearHurt::default();
    h.add(kind, days, Date(20_000));
    h
}

fn legs(p: &crate::player::Player) -> f64 {
    Attr::ALL.iter().filter(|a| a.profile().injury_sensitive).map(|&a| p.attributes.get(a)).sum()
}

#[test]
fn keeps_the_worst_injury_and_the_days_out_over_all() {
    let mut h = hurt(K::AnkleSprain, 10);
    h.add(K::Bruise, 3, Date(20_010));
    h.add(K::Hamstring, 30, Date(20_020));
    h.add(K::Concussion, 12, Date(20_030));
    assert_eq!(h.kind, Some(K::Hamstring));
    assert_eq!(h.days, 30);
    assert_eq!(h.total, 55);
    assert_eq!(h.event(), Some(Ev::ModerateInjury));
    // A torn ACL is major whatever its days.
    h.add(K::Acl, 1, Date(20_040));
    assert_eq!(h.event(), Some(Ev::MajorInjury));
    assert_eq!(hurt_class(K::Bruise, 6), 0);
    assert_eq!(hurt_class(K::Shoulder, 60), 3);
}

#[test]
fn a_healthy_year_costs_nothing_and_rolls_no_injury() {
    for seed in 0..300 {
        let mut rng = Rng::new(seed);
        let mut p = generate_prospect(&mut rng, ProspectOptions { age: Some(20), ..Default::default() });
        let ctx = crate::development::SeasonContext { hurt: Some(YearHurt::default()), ..context_for(20) };
        let r = develop_season(&mut p, &ctx, &mut rng);
        let why = take_why().unwrap();
        assert!(why.injury.is_none(), "seed {seed}");
        assert_eq!(r.games_missed, 0);
        assert!(![Ev::MinorInjury, Ev::ModerateInjury, Ev::MajorInjury].iter().any(|&e| r.events.has(e)));
    }
}

#[test]
fn a_torn_acl_costs_growth_and_burst_and_a_shoulder_costs_no_burst() {
    let (mut acl_legs, mut shoulder_legs, mut acl_burst, mut shoulder_burst) = (0.0, 0.0, 0, 0);
    for seed in 0..400 {
        for (kind, days) in [(K::Acl, 290), (K::Shoulder, 70)] {
            let mut rng = Rng::new(seed);
            let mut p = generate_prospect(&mut rng, ProspectOptions { age: Some(24), ..Default::default() });
            let before = legs(&p);
            let ctx = crate::development::SeasonContext { hurt: Some(hurt(kind, days)), ..context_for(24) };
            let r = develop_season(&mut p, &ctx, &mut rng);
            let why = take_why().unwrap();
            assert_eq!(why.injury, Some(Ev::MajorInjury));
            assert!(why.growth_multiplier <= 0.5 + 1e-9);
            assert!(r.events.has(Ev::MajorInjury), "not named before: this row names it");
            let lost = before - legs(&p);
            if kind == K::Acl {
                acl_legs += lost;
                acl_burst += r.events.has(Ev::LostBurst) as u32;
            } else {
                shoulder_legs += lost;
                shoulder_burst += r.events.has(Ev::LostBurst) as u32;
                assert_eq!(why.injury_points, 0.0);
            }
        }
    }
    assert_eq!(shoulder_burst, 0);
    assert!(acl_burst > 30 && acl_burst < 90, "about 15% of 400: {acl_burst}");
    assert!(acl_legs > shoulder_legs + 400.0, "ACL {acl_legs:.0} vs shoulder {shoulder_legs:.0}");
}

#[test]
fn an_injury_named_on_last_seasons_row_stays_there() {
    for seed in 0..400 {
        let mut rng = Rng::new(seed);
        let mut p = generate_prospect(&mut rng, ProspectOptions { age: Some(23), ..Default::default() });
        let mut h = hurt(K::Acl, 290);
        h.named = true;
        let ctx = crate::development::SeasonContext { hurt: Some(h), ..context_for(23) };
        let r = develop_season(&mut p, &ctx, &mut rng);
        assert!(!r.events.has(Ev::MajorInjury));
        assert!(!r.events.has(Ev::LostBurst));
    }
}

#[test]
fn the_year_back_returns_the_rehab_loss_only_when_flagged() {
    let mut regained = [0.0; 2];
    for seed in 0..200 {
        for (k, rehab) in [false, true].into_iter().enumerate() {
            let mut rng = Rng::new(seed);
            let mut p = generate_prospect(&mut rng, ProspectOptions { age: Some(22), ..Default::default() });
            for a in Attr::ALL.iter().filter(|a| a.profile().injury_sensitive) {
                p.attributes.add(*a, -4.0);
            }
            let h = YearHurt { rehab, ..YearHurt::default() };
            let ctx = crate::development::SeasonContext { hurt: Some(h), ..context_for(22) };
            develop_season(&mut p, &ctx, &mut rng);
            regained[k] += take_why().unwrap().rehab_regain;
        }
    }
    assert_eq!(regained[0], 0.0);
    assert!(regained[1] > 200.0, "{}", regained[1]);
}

#[test]
fn rare_complications_run_past_the_usual_range() {
    let mut rng = Rng::new(5);
    let (mut long, n) = (0, 200_000);
    let mut top: f64 = 0.0;
    for _ in 0..n {
        let w = injury_weeks(K::Acl, &mut rng);
        if w > 56.0 {
            long += 1;
        }
        top = top.max(w);
    }
    let share = long as f64 / n as f64;
    // 6% complicate; most of those pass the old top of 56 weeks.
    assert!(share > 0.02 && share < 0.06, "{share}");
    assert!(top > 100.0 && top <= 56.0 * 2.5 + 1e-9, "{top}");
    assert_eq!(complication(K::Bruise, 0.9999), 1.0);
}
