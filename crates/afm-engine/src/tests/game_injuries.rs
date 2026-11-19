//! Engine gaps lane EG1 (`docs/design/engine-gaps.md`, items 1-3): every
//! collision can hurt both men, men get hurt with nobody touching them,
//! and a hurt man goes to the sideline and comes back after a few snaps,
//! a series or not at all, with the concussion rules of his level and era.

use std::collections::{HashMap, HashSet};

use crate::game::flow::CHECK_OFF;
use crate::game::rules::{Concussion, Rules};
use crate::game::sim::{Cause, InjuryKind};
use crate::game::synthetic::synthetic_league;
use crate::game::{GameResult, GameSetup, PlayRecord, PlayType, play_game, quick_game};
use crate::player::Level;
use crate::positions::Position;

#[test]
fn the_concussion_rules_follow_each_level_and_era() {
    let c = |l: Level, y: i32| Rules::for_year(l, y, false).concussion;
    assert!(c(Level::NFL, 2009).no_same_day && !c(Level::NFL, 2008).no_same_day, "the NFL from 2009");
    assert!(c(Level::FBS, 2010).no_same_day && !c(Level::FBS, 2009).no_same_day, "college from 2010");
    assert!(c(Level::HS, 2010).no_same_day && !c(Level::HS, 2009).no_same_day, "high school from 2010");
    assert!(
        c(Level::HS, 2020).removal_sits && !c(Level::FBS, 2020).removal_sits,
        "a removal sits the day in high school"
    );
    let now = |l: Level| c(l, 2025).caught;
    assert!(now(Level::NFL) > now(Level::FBS) && now(Level::FBS) > now(Level::HS));
    assert!(c(Level::NFL, 2000).caught < now(Level::NFL), "fewer were caught before the rule");
    assert!(c(Level::NFL, 1995).length < c(Level::NFL, 2005).length && c(Level::NFL, 2005).length < 1.0);
    assert_eq!(c(Level::NFL, 2008).checks, 0.0, "no checks of men who weren't hurt before the rule");
    assert!(c(Level::NFL, 2025).checks > c(Level::NFL, 2010).checks, "spotters from 2011");
}

/// A snap from scrimmage, where each side's own unit takes the field.
fn scrimmage(k: PlayType) -> bool {
    matches!(
        k,
        PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble | PlayType::Kneel | PlayType::Spike
    )
}

fn games(level: Level, n: u64, rules: Option<Rules>) -> (Vec<GameResult>, HashMap<u32, Position>) {
    let teams = synthetic_league(level, 2 * n as usize, 41);
    let pos: HashMap<u32, Position> = teams.iter().flat_map(|t| t.players.iter().map(|p| (p.id, p.position))).collect();
    let res = (0..n)
        .map(|g| {
            let (h, a) = (&teams[2 * g as usize], &teams[2 * g as usize + 1]);
            match rules {
                Some(r) => {
                    let mut s = GameSetup::new(h, a, level, 7_300_011 + g);
                    s.rules = r;
                    play_game(&s)
                }
                None => quick_game(h, a, level, 7_300_011 + g),
            }
        })
        .collect();
    (res, pos)
}

#[test]
fn every_kind_of_collision_hurts_and_linemen_get_hurt() {
    let (res, pos) = games(Level::NFL, 24, None);
    let all: Vec<_> = res.iter().flat_map(|r| r.injuries.iter().map(|(_, i)| i)).collect();
    let causes: HashSet<Cause> = all.iter().map(|i| i.cause).collect();
    for c in [Cause::Tackled, Cause::Tackling, Cause::Block, Cause::Pile, Cause::Sprint, Cause::Plant] {
        assert!(causes.contains(&c), "no injury from {c:?} in 24 games: {causes:?}");
    }
    let line = all.iter().filter(|i| matches!(pos.get(&i.id), Some(Position::OT | Position::IOL))).count();
    assert!(line >= 10, "offensive linemen hurt {line} times in 24 games of {}", all.len());
    // Nobody touched him: strains go to the men who run.
    let strains = all.iter().filter(|i| i.cause == Cause::Sprint).count();
    assert!(strains >= 5, "{strains} sprint strains in 24 games");
}

#[test]
fn hurt_men_come_back_but_never_from_a_torn_ligament_or_a_concussion() {
    let (res, _) = games(Level::NFL, 24, None);
    let (mut back, mut checked) = (0, 0);
    for r in &res {
        // A man can be hurt twice in a game: the nth injury of his is on the
        // nth play that hurt him.
        let mut seen: HashMap<u32, usize> = HashMap::new();
        for (team, i) in &r.injuries {
            let nth = seen.entry(i.id).or_default();
            let k = *nth;
            *nth += 1;
            let Some(b) = i.back else { continue };
            back += 1;
            assert!(b >= 1, "he sits at least a play");
            assert!(!i.plays_on, "he went off");
            assert!(
                !matches!(
                    i.kind,
                    InjuryKind::Acl | InjuryKind::Achilles | InjuryKind::Fracture | InjuryKind::Concussion
                ),
                "{:?} came back in today's NFL",
                i.kind
            );
            // A starter back in is on the field again when his unit has
            // snaps left after his time on the sideline (a quarterback back
            // at the two-minute warning may never see the ball again). He
            // can play from the snap after his last one sitting.
            let plays = &r.stats.plays;
            let side = (plays.iter().enumerate())
                .filter(|(_, p)| p.injuries.iter().any(|x| x.id == i.id))
                .nth(k)
                .map(|(n, _)| n)
                .unwrap();
            let hurt = &plays[side];
            let on_offense = hurt.on_field[0].contains(&i.id);
            let unit = |p: &PlayRecord| scrimmage(p.kind) && (p.offense == *team) == on_offense;
            let on = |p: &PlayRecord| p.on_field.iter().any(|f| f.contains(&i.id));
            let before: Vec<&PlayRecord> = plays[..=side].iter().filter(|p| unit(p)).collect();
            let after: Vec<&PlayRecord> =
                plays[side + 1..].iter().filter(|p| p.n > hurt.n + b as u32 && unit(p)).collect();
            let starter = before.len() >= 10 && 2 * before.iter().filter(|p| on(p)).count() >= before.len();
            if unit(hurt) && starter && after.len() >= 10 {
                checked += 1;
                assert!(
                    after.iter().any(|p| on(p)),
                    "team {team}: {} back after {b} plays but never on the field in {} snaps",
                    i.id,
                    after.len()
                );
            }
        }
        // A man cleared by a concussion check sits the exam out first.
        for (_, id, b) in &r.checks {
            if let Some(b) = b {
                assert!(*b as f64 >= CHECK_OFF, "{id} back {b} plays after a check");
            }
        }
    }
    assert!(back >= 5, "only {back} men came back in 24 games");
    assert!(checked >= 3, "only {checked} starters came back with snaps left in 24 games");
    let cleared: usize = res.iter().map(|r| r.checks.iter().filter(|c| c.2.is_some()).count()).sum();
    assert!(cleared >= 5, "only {cleared} men cleared by a concussion check came back");
}

#[test]
fn before_the_rule_a_concussed_man_could_go_back_in() {
    let mut old = Rules::for_level(Level::NFL);
    old.concussion = Concussion::for_year(Level::NFL, 2005);
    let (res, _) = games(Level::NFL, 60, Some(old));
    let heads: Vec<_> = res
        .iter()
        .flat_map(|r| r.injuries.iter().map(|(_, i)| i))
        .filter(|i| i.kind == InjuryKind::Concussion)
        .collect();
    assert!(!heads.is_empty());
    assert!(heads.iter().any(|i| i.back.is_some()), "none of {} concussions went back in", heads.len());
    assert!(res.iter().all(|r| r.checks.is_empty()), "no checks before the rule");
}

#[test]
fn a_high_school_check_sits_the_rest_of_the_game() {
    let (res, _) = games(Level::HS, 30, None);
    let checks: Vec<_> = res.iter().flat_map(|r| r.checks.iter()).collect();
    assert!(checks.iter().all(|c| c.2.is_none()), "a high school removal comes back");
}
