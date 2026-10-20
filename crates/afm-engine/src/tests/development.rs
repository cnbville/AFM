use super::{career, context_for, mean, prospects};
use crate::attributes::{Attr, MAX_RATING, MIN_RATING};
use crate::development::{MAX_POTENTIAL_SHIFT, develop_season};
use crate::player::{DevelopmentEvent as Ev, Player, PlayingTime, ProspectOptions, generate_prospect};
use crate::positions::Position;
use crate::rng::Rng;

#[test]
fn peaks_speed_in_the_mid_20s_and_awareness_around_30() {
    let mut rng = Rng::new("curves");
    let mut speed_peak = Vec::new();
    let mut awareness_peak = Vec::new();
    let mut speed_loss_by_33 = Vec::new();
    for mut p in prospects(400, "curves", 3.0) {
        let mut best_speed = (0.0, 0.0);
        let mut best_awareness = (0.0, 0.0);
        while p.age < 36 {
            {
                let ctx = context_for(p.age);
                develop_season(&mut p, &ctx, &mut rng)
            };
            let age = p.age as f64;
            if p.attributes.get(Attr::Speed) > best_speed.0 {
                best_speed = (p.attributes.get(Attr::Speed), age);
            }
            if p.attributes.get(Attr::FootballIq) > best_awareness.0 {
                best_awareness = (p.attributes.get(Attr::FootballIq), age);
            }
            if p.age == 33 {
                speed_loss_by_33.push(best_speed.0 - p.attributes.get(Attr::Speed));
            }
        }
        speed_peak.push(best_speed.1);
        awareness_peak.push(best_awareness.1);
    }
    assert!(mean(&speed_peak) > 21.0 && mean(&speed_peak) < 27.0, "speed peak {}", mean(&speed_peak));
    assert!(mean(&awareness_peak) > 28.0);
    assert!(mean(&awareness_peak) > mean(&speed_peak) + 4.0);
    assert!(mean(&speed_loss_by_33) > 5.0);
}

#[test]
fn keeps_kickers_and_quarterbacks_going_longer_than_running_backs() {
    let peak_age = |position: Position| {
        let mut rng = Rng::new(format!("peak-{position}"));
        let ages: Vec<f64> = (0..300)
            .map(|_| {
                let mut p = generate_prospect(
                    &mut rng,
                    ProspectOptions { position: Some(position), talent_z: Some(3.0), ..Default::default() },
                );
                let mut best = (0.0, 0.0);
                while p.age < 40 {
                    {
                        let ctx = context_for(p.age);
                        develop_season(&mut p, &ctx, &mut rng)
                    };
                    if p.overall() > best.0 {
                        best = (p.overall(), p.age as f64);
                    }
                }
                best.1
            })
            .collect();
        mean(&ages)
    };
    let rb = peak_age(Position::RB);
    assert!(peak_age(Position::QB) > rb + 3.0);
    assert!(peak_age(Position::K) > rb + 5.0);
}

#[test]
fn rewards_work_ethic_coaching_and_playing_time() {
    let run = |work_ethic: Option<f64>, coaching: Option<f64>, playing_time: Option<PlayingTime>| {
        let mut rng = Rng::new("levers");
        let overalls: Vec<f64> = prospects(400, "levers", 2.5)
            .into_iter()
            .map(|mut p| {
                if let Some(w) = work_ethic {
                    p.personality.work_ethic = w;
                }
                career(&mut p, &mut rng, 22, |ctx| {
                    if let Some(c) = coaching {
                        ctx.coaching = c;
                    }
                    if let Some(pt) = playing_time {
                        ctx.playing_time = pt;
                    }
                });
                p.overall()
            })
            .collect();
        mean(&overalls)
    };
    let baseline = run(None, None, None);
    assert!(run(Some(18.0), None, None) > baseline + 1.5);
    assert!(run(Some(3.0), None, None) < baseline - 1.5);
    assert!(run(None, Some(18.0), None) > run(None, Some(5.0), None) + 2.0);
    assert!(run(None, None, Some(PlayingTime::Starter)) > run(None, None, Some(PlayingTime::Backup)) + 1.5);
}

#[test]
fn lets_late_bloomers_catch_up_with_early_bloomers() {
    let mut rng = Rng::new("bloom");
    let mut early: Vec<(f64, f64)> = Vec::new();
    let mut late: Vec<(f64, f64)> = Vec::new();
    for mut p in prospects(1500, "bloom", 2.5) {
        let bloom = p.bloom;
        career(&mut p, &mut rng, 16, |_| {});
        let at16 = p.overall();
        career(&mut p, &mut rng, 24, |_| {});
        if bloom < -0.8 {
            early.push((at16, p.overall()));
        } else if bloom > 0.8 {
            late.push((at16, p.overall()));
        }
    }
    let m = |xs: &[(f64, f64)], f: fn(&(f64, f64)) -> f64| mean(&xs.iter().map(f).collect::<Vec<_>>());
    let gap16 = m(&early, |x| x.0) - m(&late, |x| x.0);
    let gap24 = m(&early, |x| x.1) - m(&late, |x| x.1);
    assert!(gap16 > 2.0, "gap at 16 {gap16}");
    assert!(gap24.abs() < gap16 / 2.0, "gap at 24 {gap24}");
}

#[test]
fn takes_explosiveness_away_with_a_major_injury() {
    let mut rng = Rng::new("injury");
    let mut hurt = Vec::new();
    let mut healthy = Vec::new();
    for mut p in prospects(3000, "injury", 3.0) {
        career(&mut p, &mut rng, 23, |_| {});
        let before = p.attributes.get(Attr::Speed);
        let report = develop_season(&mut p, &context_for(23), &mut rng);
        let change = p.attributes.get(Attr::Speed) - before;
        if report.events.has(Ev::MajorInjury) {
            hurt.push(change);
        } else {
            healthy.push(change);
        }
    }
    assert!(hurt.len() > 20);
    assert!(mean(&hurt) < mean(&healthy) - 2.0);
}

#[test]
fn moves_potential_at_most_one_band_per_checkpoint_unless_a_breakout_injury_or_stalled_body() {
    let mut rng = Rng::new("bands");
    let mut checked = 0;
    let mut breakouts = 0;
    for mut p in prospects(1500, "bands", 1.5) {
        let mut prev = p.potential();
        while p.age < 22 {
            let r = {
                let ctx = context_for(p.age);
                develop_season(&mut p, &ctx, &mut rng)
            };
            let shift = (r.potential_after - prev).abs();
            if r.events.has(Ev::Breakout) {
                assert!(shift <= 2.0 * MAX_POTENTIAL_SHIFT + 2.0);
                breakouts += 1;
            } else if !r.events.has(Ev::MajorInjury) && !r.events.has(Ev::BodyStalled) {
                // Allowance for snaps-driven ceiling growth and players outgrowing a ceiling.
                assert!(shift <= MAX_POTENTIAL_SHIFT + 1.5, "shift {shift}");
                checked += 1;
            }
            prev = r.potential_after;
        }
    }
    assert!(checked > 8000);
    assert!(breakouts > 100);
}

#[test]
fn moves_potential_both_ways_and_mostly_before_24() {
    let mut rng = Rng::new("drift");
    let mut teen = Vec::new();
    let mut late = Vec::new();
    for mut p in prospects(1500, "drift", 2.0) {
        let p14 = p.potential();
        career(&mut p, &mut rng, 18, |_| {});
        teen.push(p.potential() - p14);
        career(&mut p, &mut rng, 24, |_| {});
        let p24 = p.potential();
        career(&mut p, &mut rng, 27, |_| {});
        late.push(p.potential() - p24);
    }
    let share = |xs: &[f64], f: fn(f64) -> bool| xs.iter().filter(|&&d| f(d)).count() as f64 / xs.len() as f64;
    assert!(share(&teen, |d| d >= 5.0) > 0.1);
    assert!(share(&teen, |d| d <= -5.0) > 0.05);
    // From 24 to 27 the ceiling barely rises.
    assert!(share(&late, |d| d >= 5.0) < 0.02);
}

#[test]
fn keeps_every_rating_within_1_99_and_current_at_or_below_its_ceiling() {
    let mut rng = Rng::new("bounds");
    let mut all = prospects(300, "bounds", 4.5);
    all.extend(prospects(300, "bounds-low", -2.0));
    for mut p in all {
        while p.age < 40 {
            {
                let ctx = context_for(p.age);
                develop_season(&mut p, &ctx, &mut rng)
            };
            for k in Attr::ALL {
                let (a, c) = (p.attributes.get(k), p.ceilings.get(k));
                assert!(
                    a >= MIN_RATING && c <= MAX_RATING && a <= c,
                    "{} {} at age {}: {a} / {c}",
                    p.id,
                    k.key(),
                    p.age
                );
            }
        }
    }
}

#[test]
fn is_deterministic_for_a_seed() {
    let make = || -> Player {
        let mut p = generate_prospect(&mut Rng::new(7), ProspectOptions { id: Some(1), ..Default::default() });
        career(&mut p, &mut Rng::new(8), 30, |_| {});
        p
    };
    assert_eq!(make(), make());
}

#[test]
fn names_a_first_year_without_drawing_anything() {
    // The first-year mark is only a mark: with or without a dip the same
    // draws are made, and the row names the first year only with one.
    for seed in 0..50u32 {
        let base = prospects(1, &format!("first-year-{seed}"), 1.0).remove(0);
        let run = |dip: f64| {
            let (mut p, mut rng) = (base.clone(), Rng::new(seed));
            let mut ctx = context_for(18);
            ctx.adjustment = dip;
            develop_season(&mut p, &ctx, &mut rng);
            (p.history[0].events.has(Ev::FirstYear), rng.next_u32())
        };
        let (none, after_none) = run(0.0);
        let (some, after_some) = run(0.6);
        assert!(!none && some);
        assert_eq!(after_none, after_some);
    }
}

#[test]
fn a_spring_step_lands_on_the_season_row() {
    // The step goes on the row of the season before it, so the next row
    // starts where that one ends.
    let mut rng = Rng::new("spring-row");
    let mut p =
        generate_prospect(&mut rng, ProspectOptions { age: Some(18), talent_z: Some(2.0), ..Default::default() });
    develop_season(&mut p, &context_for(18), &mut rng);
    let gain = crate::development::spring_jump(&mut p, 0.25, &mut rng);
    assert!(gain > 0.0);
    crate::development::record_spring_step(&mut p, gain);
    let row = &p.history[0];
    assert!(row.events.has(Ev::SpringBreakout) && row.spring_gain == Some(gain) && row.spring_open.is_none());
    develop_season(&mut p, &context_for(19), &mut rng);
    assert!((p.history[1].overall_start - p.history[0].overall).abs() < 1e-9);
}

#[test]
fn an_early_enrollees_spring_step_opens_his_first_college_row() {
    // F2: the step comes before his first college row, so that row opens
    // with it and his high school row ends where it ended.
    let mut rng = Rng::new("spring-open");
    let mut p =
        generate_prospect(&mut rng, ProspectOptions { age: Some(17), talent_z: Some(2.0), ..Default::default() });
    develop_season(&mut p, &context_for(17), &mut rng);
    let hs_end = p.history[0].overall;
    let gain = crate::development::spring_jump(&mut p, 0.25, &mut rng);
    assert!(gain > 0.0);
    develop_season(&mut p, &context_for(18), &mut rng);
    crate::development::open_row_with_spring_step(&mut p, gain);
    let (hs, first) = (&p.history[0], &p.history[1]);
    assert!(hs.spring_gain.is_none() && !hs.events.has(Ev::SpringBreakout) && hs.overall == hs_end);
    assert!(first.events.has(Ev::SpringBreakout) && first.spring_gain == Some(gain));
    // The opening step is marked apart, so the rating he played the season
    // at reads back as the row's start plus the step.
    assert_eq!(first.spring_open, Some(gain));
    assert!((first.overall_start - hs_end).abs() < 1e-9);
    // A later spring step after the season adds to the total, not the opening part.
    crate::development::record_spring_step(&mut p, 1.5);
    let first = &p.history[1];
    assert_eq!(first.spring_open, Some(gain));
    assert!((first.spring_gain.unwrap() - gain - 1.5).abs() < 1e-9);
}
