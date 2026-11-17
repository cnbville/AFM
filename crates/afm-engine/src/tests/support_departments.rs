//! Support staff as full departments (`docs/design/support-departments.md`):
//! each head runs stored people sized by level and money; a typical
//! department reads as the level's typical head (question 3); a thin staff
//! shortchanges the backups; the richer staffs read better; the new reads
//! stay inside their agreed sizes; high schools have trainers at the CATCH
//! ON shares; an older file is staffed on load.

use super::legacy::seat;
use crate::world::World;
use crate::world::data::Division;
use crate::world::departments::{self as dp, Area, Job};
use crate::world::finance::college::STAFF;
use crate::world::people::{Who, staff_support};

/// A college seat with its books open and a year of the support market.
fn world() -> World {
    let mut w = seat("college");
    crate::world::finance::ensure(&mut w);
    staff_support(&mut w);
    w
}

fn count(w: &World, who: Who, job: Job) -> usize {
    dp::people_of(w, who).iter().filter(|a| a.job == job).count()
}

fn levels(w: &World, lvl: usize) -> Vec<Who> {
    let mut v: Vec<Who> = (0..w.programs.len()).filter(|&p| w.programs[p].plays(w.year)).map(Who::Prog).collect();
    v.extend((0..w.clubs.len()).filter(|&t| crate::world::franchise::active_in(w, t, w.year)).map(Who::Club));
    v.into_iter().filter(|&who| dp::level(w, who) == lvl).collect()
}

#[test]
fn departments_are_sized_by_level_and_money() {
    let w = world();
    // Power 4: 2-3 dietitians, 3-5 assistant trainers, 4-10 tutors.
    let p4 = levels(&w, 0);
    assert!(p4.len() > 40, "Power 4 programs: {}", p4.len());
    for &who in &p4 {
        assert!((2..=3).contains(&count(&w, who, Job::Dietitian)), "Power 4 dietitians");
        assert!((3..=5).contains(&count(&w, who, Job::Trainer)), "Power 4 trainers");
        assert!((4..=10).contains(&count(&w, who, Job::Tutor)), "Power 4 tutors");
        assert_eq!(count(&w, who, Job::PainSpecialist), 0, "a pain specialist is the NFL's");
    }
    // Division III: the heads alone, shared across the department.
    for who in levels(&w, 4) {
        assert!(dp::people_of(&w, who).is_empty(), "Division III keeps no one below the heads");
    }
    // The NFL in 2037: a clinician and a pain specialist at every club
    // (2019 on), three assistant trainers, 1-6 analysts.
    let nfl = levels(&w, 5);
    assert!(nfl.len() >= 32);
    for &who in &nfl {
        assert_eq!(count(&w, who, Job::Clinician), 1);
        assert_eq!(count(&w, who, Job::PainSpecialist), 1);
        assert_eq!(count(&w, who, Job::Trainer), 3);
        assert!((1..=6).contains(&count(&w, who, Job::Analyst)));
        assert_eq!(count(&w, who, Job::Tutor), 0);
    }
    // Everyone is rated, paid and of working age.
    for who in p4.iter().chain(&nfl) {
        for a in dp::people_of(&w, *who) {
            assert!((1..=20).contains(&a.rating) && a.pay > 0.0 && (18..=75).contains(&(w.year - a.born)), "{a:?}");
        }
    }
    // The money decides inside the range: the richest Power 4 staff lines
    // keep more people than the poorest.
    let mut rows: Vec<(f64, usize)> = p4
        .iter()
        .map(|&who| {
            let Who::Prog(p) = who else { unreachable!() };
            (w.finance.college[p].budget.l[STAFF] as f64, dp::people_of(&w, who).len())
        })
        .collect();
    rows.sort_by(|a, b| b.0.total_cmp(&a.0));
    let half = rows.len() / 2;
    let rich = rows[..half].iter().map(|r| r.1 as f64).sum::<f64>() / half as f64;
    let poor = rows[half..].iter().map(|r| r.1 as f64).sum::<f64>() / (rows.len() - half) as f64;
    assert!(rich > poor + 1.0, "the richer half keeps more people: {rich:.1} against {poor:.1}");
}

#[test]
fn a_typical_department_reads_as_the_typical_head() {
    let w = world();
    // Question 3: each level's mean read of a headed department is the
    // level's mean head, so the world's averages don't move.
    for d in [Division::FBS, Division::FCS, Division::D2, Division::D3] {
        let list: Vec<Who> = (0..w.programs.len())
            .filter(|&p| w.programs[p].division == d && !w.programs[p].roster.is_empty() && w.programs[p].plays(w.year))
            .map(Who::Prog)
            .collect();
        if list.len() < 5 {
            continue;
        }
        for area in [Area::Strength, Area::Medical, Area::Academic, Area::Engagement] {
            let mean = list.iter().map(|&who| dp::team(&w, who, area)).sum::<f64>() / list.len() as f64;
            let typ = dp::typical_head(&w, list[0], area).unwrap();
            // Reads clamp at 1 and 20, so allow a hair.
            assert!((mean - typ).abs() < 0.05, "{d:?} {area:?}: mean read {mean:.3}, typical head {typ:.3}");
        }
        for area in [Area::Diet, Area::Science, Area::Clinic] {
            let mean = list.iter().map(|&who| dp::team(&w, who, area)).sum::<f64>() / list.len() as f64;
            assert!(mean.abs() < 0.05, "{d:?} {area:?}: the new reads average 0 over typical, {mean:.3}");
        }
    }
}

#[test]
fn a_thin_staff_shortchanges_the_backups_and_money_buys_a_better_one() {
    let w = world();
    // Tiers: starters first. With 30 starters, 30 in the rotation and 60
    // more, a staff covering fewer than all gives the starters more.
    let mut thin = 0;
    for who in levels(&w, 2).into_iter().chain(levels(&w, 3)) {
        let t = dp::tiers(&w, who, Area::Medical, [30, 30, 60]);
        assert!(t[0] >= t[1] && t[1] >= t[2], "{t:?}");
        if t[0] > t[2] + 0.1 {
            thin += 1;
        }
    }
    assert!(thin > 20, "FCS and smaller staffs leave the backups short: {thin}");
    // The richest FBS staff lines read better than the poorest.
    let mut fbs: Vec<(f64, f64, f64)> = (0..w.programs.len())
        .filter(|&p| w.programs[p].plays(w.year) && w.programs[p].division == Division::FBS)
        .map(|p| {
            (
                w.finance.college[p].budget.l[STAFF] as f64,
                dp::team(&w, Who::Prog(p), Area::Strength),
                dp::team(&w, Who::Prog(p), Area::Diet),
            )
        })
        .collect();
    fbs.sort_by(|a, b| b.0.total_cmp(&a.0));
    let n = 20.min(fbs.len() / 2);
    let top = |k: usize| fbs.iter().take(n).map(|x| if k == 1 { x.1 } else { x.2 }).sum::<f64>() / n as f64;
    let bot = |k: usize| fbs.iter().rev().take(n).map(|x| if k == 1 { x.1 } else { x.2 }).sum::<f64>() / n as f64;
    assert!(top(1) > bot(1), "strength: richest {:.2}, poorest {:.2}", top(1), bot(1));
    assert!(top(2) > bot(2), "diet: richest {:.2}, poorest {:.2}", top(2), bot(2));
}

#[test]
fn the_new_reads_keep_their_agreed_sizes() {
    // Question 4: weight pace only, 20% at most.
    assert_eq!(dp::diet_pace(0.0), 1.0);
    assert!((dp::diet_pace(30.0) - 1.2).abs() < 1e-9 && (dp::diet_pace(-30.0) - 0.8).abs() < 1e-9);
    // Question 5: a sports scientist up to 8% against a typical one, read
    // through the lab, capped with the strength guard at 20% together.
    assert!((dp::science_guard(20.0, 5.0) - 0.92).abs() < 1e-9);
    assert!(dp::science_guard(5.0, 1.0) > dp::science_guard(5.0, 5.0), "a level 1 lab reads less");
    assert!((dp::soft_guard(0.85, 0.92) - 0.8).abs() < 1e-9 && (dp::soft_guard(1.15, 1.08) - 1.2).abs() < 1e-9);
    // Question 6: a clinician up to 30% faster.
    assert!((dp::clinic_speed(20.0) - 1.3).abs() < 1e-9 && dp::clinic_speed(0.0) == 1.0);
}

#[test]
fn high_schools_have_trainers_at_the_catch_on_shares() {
    let w = world();
    let (mut full, mut part, mut none) = (0.0f64, 0.0f64, 0.0f64);
    for s in w.high_schools.iter().filter(|s| !s.private) {
        match &s.trainer {
            Some(a) if a.share >= 1.0 => full += 1.0,
            Some(_) => part += 1.0,
            None => none += 1.0,
        }
    }
    let n = full + part + none;
    assert!(n > 100.0);
    // Public schools: 37% full-time, 32% part-time, 31% none.
    assert!((full / n - 0.37).abs() < 0.08, "full-time {:.2}", full / n);
    assert!((part / n - 0.32).abs() < 0.08, "part-time {:.2}", part / n);
    assert!((none / n - 0.31).abs() < 0.08, "none {:.2}", none / n);
    // A school with a full-time trainer reads better than one with none.
    let with = w.high_schools.iter().position(|s| s.trainer.as_ref().is_some_and(|a| a.share >= 1.0 && a.rating >= 8));
    let without = w.high_schools.iter().position(|s| s.trainer.is_none());
    if let (Some(a), Some(b)) = (with, without) {
        assert!(dp::hs_medical(&w, a) > dp::hs_medical(&w, b));
    }
}

#[test]
fn an_older_file_is_staffed_on_load_and_assistants_step_up() {
    let mut w = world();
    for p in w.programs.iter_mut() {
        p.support.people.clear();
        p.support.staffed = false;
    }
    for c in w.clubs.iter_mut() {
        c.support.people.clear();
        c.support.staffed = false;
    }
    w.support_market.typical.clear();
    dp::ensure(&mut w);
    assert!(!w.support_market.typical.is_empty());
    let p4 = levels(&w, 0);
    assert!(p4.iter().all(|&who| !dp::people_of(&w, who).is_empty()), "every Power 4 staff is filled again");
    // An open head trainer's post goes to an assistant trainer far more
    // often than not when one of his own is the best man around.
    let before = w.support_market.years.last().copied().unwrap_or_default();
    for &who in p4.iter().take(30) {
        let Who::Prog(p) = who else { continue };
        if let Some(a) = w.programs[p].support.people.iter_mut().find(|a| a.job == Job::Trainer) {
            a.rating = 20;
        }
        w.programs[p].support.medical = Default::default();
    }
    w.year += 1;
    staff_support(&mut w);
    let after = *w.support_market.years.last().unwrap();
    assert!(after.year != before.year || after.stepped_up > before.stepped_up);
    assert!(after.stepped_up >= 15, "own assistants stepped up: {}", after.stepped_up);
}
