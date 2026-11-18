//! Coach fire on the field (`docs/design/coach-fire.md`): the fire habits,
//! the targeting rule by level and era, a second-half targeting ejection
//! sitting the next game's first half, and the game's flagged hits reaching
//! the league office.

use crate::calendar::Level as CalLevel;
use crate::game::rules::{Rules, Targeting};
use crate::game::sim::FoulKind;
use crate::game::stats::{HitFlag, PlayType};
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
    let s = (0..home.players.len())
        .filter(|&i| home.players[i].position == Position::S)
        .max_by(|&a, &b| home.players[a].overall_at(Position::S).total_cmp(&home.players[b].overall_at(Position::S)))
        .unwrap();
    let id = home.players[s].id;
    home.day.first_half = vec![id];
    let r = crate::game::quick_game(&home, &teams[1], level, 5);
    let on = |half: bool| {
        r.stats
            .plays
            .iter()
            .filter(|p| (p.quarter >= 3) == half)
            .filter(|p| p.on_field.iter().any(|f| f.contains(&id)))
            .count()
    };
    assert_eq!(on(false), 0, "he sits the first half");
    // He starts the second half and plays it while his team has everyone
    // in: a blowout sends a beaten team to its backups (garbage time), and
    // this game became one when engine gaps lane EG2 changed its draws (his
    // team down 3-38 late in the third quarter, after 7 snaps of his).
    let snaps: Vec<_> = r
        .stats
        .plays
        .iter()
        .filter(|p| p.quarter >= 3 && p.offense == 1 && p.garbage[0] == 0)
        .filter(|p| matches!(p.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble))
        .collect();
    let played = snaps.iter().filter(|p| p.on_field[1].contains(&id)).count();
    assert!(snaps.len() >= 5, "a second half to play: {} snaps before garbage time", snaps.len());
    assert!(snaps[0].on_field[1].contains(&id), "he starts the second half");
    assert!(played * 10 >= snaps.len() * 9, "and plays it: {played} of {} snaps", snaps.len());
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
