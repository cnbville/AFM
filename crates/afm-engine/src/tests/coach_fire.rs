//! Coach fire on the field (`docs/design/coach-fire.md`): the fire habits,
//! the targeting rule by level and era, a second-half targeting ejection
//! sitting the next game's first half, and the game's flagged hits reaching
//! the league office.

use crate::calendar::Level as CalLevel;
use crate::game::GameResult;
use crate::game::rules::{Rules, Targeting};
use crate::game::sim::FoulKind;
use crate::game::stats::{HitFlag, PlayRecord, PlayType};
use crate::game::synthetic::synthetic_league;
use crate::player::Level;
use crate::positions::Position;
use crate::tendencies::Tend;
use crate::world::folk::{Team, rules};
use crate::world::{World, WorldOptions};

#[test]
fn fire_moves_the_hitting_and_guessing_habits_only() {
    let fiery: Vec<Tend> = Tend::ALL.into_iter().filter(|t| t.fiery()).collect();
    assert_eq!(
        fiery,
        vec![Tend::Contact, Tend::SnapTiming, Tend::Strip, Tend::BigHit, Tend::Downhill, Tend::JumpRoutes]
    );
}

#[test]
fn targeting_follows_each_level_and_era() {
    let at = |l: Level, y: i32| Rules::for_year(l, y, false).targeting;
    assert_eq!(at(Level::NFL, 2025), Targeting::None, "the NFL calls it unnecessary roughness");
    assert_eq!(at(Level::FBS, 2007), Targeting::None);
    assert_eq!(at(Level::FBS, 2010), Targeting::Foul, "15 yards from 2008");
    assert_eq!(at(Level::FCS, 2013), Targeting::Eject, "and an ejection from 2013");
    assert_eq!(at(Level::HS, 2012), Targeting::None);
    assert_eq!(at(Level::HS, 2013), Targeting::Foul, "NFHS: disqualified only when flagrant");
    assert_eq!(FoulKind::Targeting.yards(), (15.0, true));
    assert!(FoulKind::Targeting.personal() && !FoulKind::Offside.personal());
}

#[test]
fn a_first_half_sitter_comes_back_at_halftime() {
    let level = Level::FBS;
    let teams = synthetic_league(level, 2, 11);
    let mut home = teams[0].clone();
    // The defense's snaps from scrimmage while his staff plays its starters
    // (`garbage`: a game decided by halftime pulls them from the second
    // half's first snap).
    let defense = |p: &PlayRecord| {
        p.offense == 1
            && p.garbage[0] == 0
            && !matches!(
                p.kind,
                PlayType::Punt
                    | PlayType::FieldGoal
                    | PlayType::ExtraPoint
                    | PlayType::TwoPoint
                    | PlayType::Kickoff
                    | PlayType::NoPlay
            )
    };
    // The safety who plays most of them over eight ordinary games: the best
    // rating alone can name a backup on a roster whose flaws drew
    // differently, and one game's busiest man on the field, counting the
    // kicking units, can be a third safety who played the day his partner
    // was hurt early, whose share swings with the other side's personnel (on
    // lane EG2's games it was).
    let plain: Vec<GameResult> =
        (1..=8u64).map(|seed| crate::game::quick_game(&home, &teams[1], level, seed)).collect();
    let snaps = |id: u32| {
        plain.iter().flat_map(|r| &r.stats.plays).filter(|p| defense(p) && p.on_field[1].contains(&id)).count()
    };
    let id =
        home.players.iter().filter(|p| p.position == Position::S).map(|p| p.id).max_by_key(|&id| snaps(id)).unwrap();
    // In the same eight games with the sit he never plays a first-half
    // snap, and in the second half he is back at his usual share of them,
    // before he is hurt himself, against the games without the sit.
    let on = |p: &PlayRecord| p.on_field.iter().any(|f| f.contains(&id));
    let second_half = |games: &[GameResult], sits: bool| {
        let (mut open, mut his) = (0, 0);
        for (k, r) in games.iter().enumerate() {
            let plays = &r.stats.plays;
            assert!(!sits || !plays.iter().any(|p| p.quarter < 3 && on(p)), "he sits the first half (seed {})", k + 1);
            let hurt = plays.iter().position(|p| p.injuries.iter().any(|x| x.id == id)).unwrap_or(plays.len());
            for p in plays[..hurt].iter().filter(|p| p.quarter >= 3 && defense(p)) {
                open += 1;
                his += usize::from(on(p));
            }
        }
        (open, his)
    };
    let (open0, his0) = second_half(&plain, false);
    home.day.first_half = vec![id];
    let sat: Vec<GameResult> = (1..=8u64).map(|seed| crate::game::quick_game(&home, &teams[1], level, seed)).collect();
    let (open, his) = second_half(&sat, true);
    assert!(open >= 100 && his0 * 2 >= open0, "{open} snaps before a staff pulled its starters; {his0} of {open0} his");
    assert!(
        his * open0 * 20 >= his0 * open * 17,
        "and is back at his share: {his} of {open} snaps, {his0} of {open0} without the sit"
    );
}

fn flag(id: u32, kind: FoulKind, ejected: bool, second_half: bool) -> HitFlag {
    HitFlag { team: 0, id, kind, ejected, second_half, defenseless: true }
}

#[test]
fn a_second_half_ejection_sits_the_next_first_half_once() {
    let mut w = World::empty(WorldOptions { scale: 0.001, history_years: 0, start_year: 2026, ..Default::default() });
    let teams = [Team::College(0), Team::College(1)];
    let flags = [flag(7, FoulKind::Targeting, true, true), flag(8, FoulKind::Targeting, true, false)];
    rules::game_flags(&mut w, CalLevel::College, teams, &flags, [&[], &[]]);
    let out = |w: &World| w.folk.discipline.half_out.iter().map(|x| x.0).collect::<Vec<u32>>();
    assert_eq!(out(&w), vec![7], "only the second-half ejection carries over");
    assert_eq!(w.folk.discipline.flagged.len(), 2, "both wait for the league office");
    assert!(w.folk.discipline.engine.1);
    // He sits his next game's first half, and then it is served.
    rules::game_flags(&mut w, CalLevel::College, teams, &[], [&[7], &[]]);
    assert!(w.folk.discipline.half_out.is_empty());
    // An NFL flag is fined, never carried; a high school one counts only
    // when he is thrown out.
    rules::game_flags(&mut w, CalLevel::Nfl, [Team::Nfl(0), Team::Nfl(1)], &flags, [&[], &[]]);
    assert!(w.folk.discipline.half_out.is_empty() && w.folk.discipline.engine.0);
    let n = w.folk.discipline.flagged.len();
    let hs = [flag(9, FoulKind::UnnecessaryRoughness, false, true), flag(10, FoulKind::Targeting, true, false)];
    rules::game_flags(&mut w, CalLevel::HighSchool, [Team::High(0), Team::High(1)], &hs, [&[], &[]]);
    assert_eq!(w.folk.discipline.flagged.len(), n + 1);
}
