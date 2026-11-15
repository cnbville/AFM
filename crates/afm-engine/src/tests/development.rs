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
    // 6,000 prospects: the share whose ceiling falls 5+ by 18 sits near
    // 5.7% (4.5-6.9% over ten seeds of 1,500), so 1,500 put it within one
    // draw's noise of the 5% bar; a change to any position's generator
    // reshuffled it under (the safety and linebacker leanings, 2026-11-08).
    // The split ceiling's everyday coaching closes part of the room each
    // year, so a ceiling that fell wins some of it back: on this sample the
    // share falling 5+ goes from 5.6% (progression off) to 4.9%, and the
    // share rising 5+ from 13% to 16% (2026-11-15).
    for mut p in prospects(6000, "drift", 2.0) {
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
    assert!(share(&teen, |d| d <= -5.0) > 0.04);
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
    assert!((first.spring_gain.get().unwrap() - gain - 1.5).abs() < 1e-9);
}

/// Section 6b topic 5 and individual practice topic 5b: a focus grows its
/// skill faster by its minutes (about 1.3 times the climb at the usual 30
/// minutes a week, about 1.5 at 60 for an average player), his position's
/// key skills pay a quarter of the extra, his year as a whole moves well
/// inside 15%, and nothing passes its ceiling.
#[test]
fn a_training_focus_grows_its_skill_and_costs_the_rest_a_little() {
    let mut rng = Rng::new("focus");
    // [none, 30 minutes, 60 minutes]: the focus, its key skills, every skill.
    let (mut focus, mut keys, mut all) = ([0.0; 3], [0.0; 3], [0.0; 3]);
    let mut over = 0;
    for p in prospects(400, "focus", 1.0) {
        let key = Attr::ALL
            .into_iter()
            .filter(|&a| a.profile().group == crate::attributes::AttributeGroup::Technical)
            .max_by(|&a, &b| {
                crate::positions::weight(p.position, a).total_cmp(&crate::positions::weight(p.position, b))
            })
            .unwrap();
        let cost = crate::development::focus_cost_skills(p.position, key);
        for (k, m) in [None, Some(30.0), Some(60.0)].into_iter().enumerate() {
            let mut ctx = context_for(19);
            if let Some(m) = m {
                ctx.focus = Some(key);
                ctx.focus_minutes = m;
            }
            let mut a = p.clone();
            let mut ra = rng.clone();
            develop_season(&mut a, &ctx, &mut ra);
            focus[k] += a.attributes.get(key) - p.attributes.get(key);
            keys[k] += cost.iter().map(|(s, _)| a.attributes.get(*s) - p.attributes.get(*s)).sum::<f64>();
            all[k] += Attr::ALL.iter().map(|&s| a.attributes.get(s) - p.attributes.get(s)).sum::<f64>();
            if a.attributes.get(key) > a.ceilings.get(key) + 1e-6 {
                over += 1;
            }
        }
        rng.next();
    }
    let lift = |k: usize| focus[k] / focus[0];
    let paid = |k: usize| (keys[0] - keys[k]) / (focus[k] - focus[0]);
    let year = |k: usize| all[k] / all[0];
    println!(
        "focus lift {:.3} at 30, {:.3} at 60; key skills pay {:.3} and {:.3} of the extra; the year {:.3}, {:.3}",
        lift(1),
        lift(2),
        paid(1),
        paid(2),
        year(1),
        year(2)
    );
    assert!(lift(1) > 1.2 && lift(1) < 1.6, "focus lift at 30 {:.2}", lift(1));
    assert!(lift(2) > 1.4 && lift(2) < 2.0, "focus lift at 60 {:.2}", lift(2));
    assert!(paid(2) > 0.18 && paid(2) <= 0.26, "key skills pay {:.3}", paid(2));
    assert!((year(2) - 1.0).abs() < 0.15, "the year moves {:.3}", year(2));
    assert_eq!(over, 0, "a focus never passes the ceiling");
}

/// Individual practice topic 5b: a veteran's focus slows the decline of a
/// technique skill, by up to a third at 60 minutes a week.
#[test]
fn a_veterans_focus_slows_his_decline() {
    let mut rng = Rng::new("focus-veteran");
    let (mut plain, mut worked) = (0.0, 0.0);
    for mut p in prospects(300, "focus-veteran", 1.5) {
        let mut r = rng.clone();
        while p.age < 31 {
            let ctx = context_for(p.age);
            develop_season(&mut p, &ctx, &mut r);
        }
        let key = Attr::ManCoverage;
        let mut ctx = context_for(31);
        let mut a = p.clone();
        develop_season(&mut a, &ctx, &mut r.clone());
        ctx.focus = Some(key);
        ctx.focus_minutes = 60.0;
        let mut b = p.clone();
        develop_season(&mut b, &ctx, &mut r.clone());
        plain += a.attributes.get(key) - p.attributes.get(key);
        worked += b.attributes.get(key) - p.attributes.get(key);
        rng.next();
    }
    let slowed = 1.0 - worked / plain;
    println!("decline {plain:.1} plain, {worked:.1} with a focus: slowed {slowed:.3}");
    assert!(plain < 0.0, "a 31-year-old's coverage declines");
    assert!(slowed > 0.15 && slowed <= 0.34, "slowed {slowed:.3}");
}

/// Minor bug 302: the strength staff drives strength, speed and conditioning
/// at about the size a position coach has on technique, around the level's
/// typical staff. The same players and random streams in every arm, from 18
/// to 22 at FBS (the typical mix 13, the typical strength head 10).
#[test]
fn strength_staff_drives_physical_growth() {
    use crate::development::SeasonContext;
    let base = prospects(1500, "strength-staff", 1.5);
    // dev: the strength head over typical; pc: the position coach over typical.
    let run = |dev: f64, pc: f64, typical: bool| -> [f64; 4] {
        let mut sums = [0.0; 4];
        for (k, p0) in base.iter().enumerate() {
            let mut p = p0.clone();
            let mut rng = Rng::new(format!("strength-staff-{k}"));
            career(&mut p, &mut rng, 22, |ctx: &mut SeasonContext| {
                if ctx.level == crate::player::Level::FBS && typical {
                    let mix = ctx.coaching;
                    ctx.coaching = mix + 0.4 * pc;
                    ctx.physical_coaching = Some(mix + 0.4 * dev);
                    ctx.strength_dev = dev;
                    ctx.physical = 0.06 * dev;
                }
            });
            sums[0] += p.attributes.get(Attr::Strength);
            sums[1] += p.attributes.get(Attr::Stamina);
            sums[2] += p.attributes.get(Attr::Speed);
            sums[3] += p.overall();
        }
        sums.map(|s| s / base.len() as f64)
    };
    let none = run(0.0, 0.0, false);
    let typical = run(0.0, 0.0, true);
    let strong = run(10.0, 0.0, true);
    let weak = run(-5.0, 0.0, true);
    let coach = run(0.0, 10.0, true);
    let gap = |a: [f64; 4], b: [f64; 4]| [a[0] - b[0], a[1] - b[1], a[2] - b[2], a[3] - b[3]];
    eprintln!("strength 20 vs 10 (strength, stamina, speed, overall): {:?}", gap(strong, typical));
    eprintln!("strength 5 vs 10: {:?}", gap(weak, typical));
    eprintln!("position coach 20 vs 10: {:?}", gap(coach, typical));
    // The level's typical staff changes nothing.
    assert_eq!(none, typical);
    let s = gap(strong, typical);
    let c = gap(coach, typical);
    assert!(s[0] > 0.5 && s[1] > 0.3 && s[2] > 0.15, "strength staff: {s:?}");
    // The position coach no longer moves the physical side; the strength
    // staff moves it at least as much as he moves the whole player.
    assert!(c[0].abs() < 0.1 && c[2].abs() < 0.1, "position coach: {c:?}");
    assert!(s[0] > c[3], "strength {s:?} coach {c:?}");
    let w = gap(weak, typical);
    assert!(w[0] < 0.0 && w[0] > -s[0], "weak staff: {w:?}");
}

/// Minor bug 303: the strength staff's guard takes its share off muscle
/// strains and backs and leaves every other kind's chance where it was.
#[test]
fn strength_guard_trims_soft_tissue_only() {
    use crate::game::sim::{GAME_INJURIES, guarded_mix, strength_guard};
    assert_eq!(strength_guard(13.0, 13.0), 1.0);
    assert_eq!(strength_guard(20.0, 10.0), 0.85);
    assert_eq!(strength_guard(2.0, 12.0), 1.15);
    let (same, f) = guarded_mix(&GAME_INJURIES, 1.0);
    assert_eq!(f, 1.0);
    assert_eq!(same, GAME_INJURIES);
    let total: f64 = GAME_INJURIES.iter().map(|e| e.1).sum();
    let (mix, f) = guarded_mix(&GAME_INJURIES, 0.85);
    let new_total: f64 = mix.iter().map(|e| e.1).sum();
    for (&(k, before), &(_, after)) in GAME_INJURIES.iter().zip(mix.iter()) {
        let chance_before = before / total;
        let chance_after = f * after / new_total;
        let want = if k.soft_tissue() { 0.85 * chance_before } else { chance_before };
        assert!((chance_after - want).abs() < 1e-12, "{k:?}: {chance_after} against {want}");
    }
}
