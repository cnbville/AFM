//! The world's building blocks: data, grades, coach reputation, backgrounds.

use super::mean;
use crate::attributes::Attr;
use crate::player::{ProspectOptions, generate_prospect};
use crate::rng::Rng;
use crate::world::background::{BackgroundSeed, apply_background, generate_background, generate_family, hardship};
use crate::world::coach::{CoachTier, TitleKind, generate_coach};
use crate::world::data::{COLLEGES, Division, STATES, hs_nil};
use crate::world::high_school::transfer_rule;
use crate::world::program::{GradeCat, GradeSpeed, letter, move_grade};

#[test]
fn real_data_loads() {
    let count = |d: Division| COLLEGES.iter().filter(|c| c.division == d).count();
    assert_eq!((count(Division::FBS), count(Division::FCS), count(Division::D2)), (138, 127, 160));
    assert_eq!(STATES.len(), 51);
    // 46 states plus DC allow high school NIL (On3, May 2026).
    assert_eq!(STATES.iter().filter(|s| hs_nil(&s.code)).count(), 47);
    assert!(!hs_nil("AL") && hs_nil("CA"));
    let ca = STATES.iter().find(|s| s.code == "CA").unwrap();
    assert_eq!(ca.powerhouses.len(), 5);
}

#[test]
fn two_conference_titles_move_title_chances_far() {
    // Topic 7: no EA-style C- to C+ after back-to-back titles.
    let mut g = 70.0;
    assert_eq!(letter(g), "C-");
    for _ in 0..2 {
        g = move_grade(g, 93.0, GradeCat::TitleChances.speed(), false);
    }
    assert!(g >= 87.0, "title chances only reached {g:.1} ({})", letter(g));
    // A steady grade like tradition barely moves in two years.
    let mut t = 70.0;
    for _ in 0..2 {
        t = move_grade(t, 93.0, GradeCat::Tradition.speed(), false);
    }
    assert!(t < 78.0);
    assert_eq!(GradeCat::ConferenceStrength.speed(), GradeSpeed::Realignment);
}

#[test]
fn big_results_boost_reputation_massively_and_assistants_move_slower() {
    let mut rng = Rng::new("coach");
    let base = generate_coach(0, "A".into(), crate::world::ethnicity::Ethnicity::White, CoachTier::Fcs, 45, &mut rng);
    let mut champ = base.clone();
    let mut plain = base.clone();
    let mut assistant = base.clone();
    champ.update_reputation(0.2, TitleKind::Conference, 0.0, true, 0.5);
    plain.update_reputation(0.2, TitleKind::None, 0.0, true, 0.5);
    assistant.update_reputation(0.2, TitleKind::Conference, 0.0, false, 0.5);
    assert!(champ.reputation - plain.reputation >= 2.0);
    assert!(assistant.reputation - base.reputation < champ.reputation - base.reputation);
    // Recency bias: last season outweighs the one before.
    let mut a = base.clone();
    let mut b = base.clone();
    a.recent = vec![-0.2, 0.2];
    b.recent = vec![0.2, -0.2];
    assert!(a.hiring_buzz() > b.hiring_buzz() + 2.0);
}

#[test]
fn background_marks_are_noteworthy_not_driving() {
    let mut rng = Rng::new("backgrounds");
    let mut poor = Vec::new();
    let mut rich = Vec::new();
    let mut grit_hard = Vec::new();
    let mut grit_easy = Vec::new();
    for k in 0..4000 {
        let resources = if k % 2 == 0 { 0.1 } else { 0.9 };
        let money = if k % 2 == 0 { 0.15 } else { 0.85 };
        let seed = BackgroundSeed { area_money: money, area_safety: money, school_resources: resources };
        let mut p = generate_prospect(&mut rng, ProspectOptions { talent_z: Some(1.5), ..Default::default() });
        let before = p.potential();
        let (mut bg, structure) = generate_background(seed, &mut rng);
        let family = generate_family(&bg, Some(structure), &mut rng);
        apply_background(&mut p, &mut bg, &family, &mut rng);
        let lift = p.potential() - before;
        if k % 2 == 0 {
            poor.push(lift);
        } else {
            rich.push(lift);
        }
        if hardship(&bg) > 0.6 {
            grit_hard.push(bg.grit);
        } else if hardship(&bg) < 0.3 {
            grit_easy.push(bg.grit);
        }
        assert!(p.ceilings[Attr::Speed] <= 99.0);
    }
    let gap = mean(&poor) - mean(&rich);
    // Kids without resources keep a little more hidden upside: small, not decisive.
    assert!(gap > 0.3 && gap < 3.0, "upside gap {gap:.2}");
    assert!(mean(&grit_hard) > mean(&grit_easy) + 0.8);
}

#[test]
fn state_transfer_rules_read_from_summaries() {
    assert_eq!(transfer_rule("Transfers sit the first 50% of regular-season varsity games").sit_out, 0.5);
    assert_eq!(transfer_rule("transfers after the deadline sit 365 days").sit_out, 1.0);
    assert!(transfer_rule("One-time transfer allowed without sitting").free_once);
}
