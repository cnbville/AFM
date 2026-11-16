//! The chain reaction (Ground GR21): a high school climbs on its old
//! players in college and the NFL and slowly falls back when they stop.

use crate::player::PlayingTime;
use crate::world::data::Division;
use crate::world::high_school::{self, CLIMB_DOWN, CLIMB_UP, Chain, alumni_value, climb, transfer_rule};
use crate::world::recruiting::move_in_chance;
use crate::world::{Stage, World, WorldOptions};

fn world() -> World {
    World::empty(WorldOptions { scale: 0.02, history_years: 0, start_year: 2030, ..Default::default() })
}

/// The state with the most schools, its schools from the lowest prestige
/// up, with nobody counted as an old player of any of them.
fn quiet_state(w: &mut World) -> Vec<usize> {
    let s = (0..w.states.len()).max_by_key(|&s| w.states[s].schools.len()).unwrap();
    let mut schools = w.states[s].schools.clone();
    schools.sort_by(|&a, &b| w.high_schools[a].prestige.total_cmp(&w.high_schools[b].prestige).then(a.cmp(&b)));
    for p in w.people.iter_mut() {
        if schools.contains(&p.hs) && p.stage != Stage::HighSchool {
            p.stage = Stage::Done;
        }
    }
    schools
}

/// Makes `n` new people old players of school `h`, starting at a power
/// program.
fn alumni(w: &mut World, h: usize, n: usize) {
    let power = (0..w.programs.len())
        .find(|&c| w.programs[c].division == Division::FBS && w.programs[c].power_conference(w.year))
        .unwrap();
    let mut p = w.people[0].clone();
    p.stage = Stage::College;
    p.college = Some(power);
    p.hs = h;
    p.last_role = Some(PlayingTime::Starter);
    p.pro = None;
    for _ in 0..n {
        w.people.push(p.clone());
    }
}

#[test]
fn a_school_whose_players_make_it_climbs_and_a_quiet_one_falls_slowly() {
    let mut w = world();
    let schools = quiet_state(&mut w);
    assert!(schools.len() >= 6, "a state with schools");
    let (low, top) = (schools[0], *schools.last().unwrap());
    let ladder_top = schools.iter().map(|&h| w.high_schools[h].chain.start).fold(0.0, f64::max);
    let (low0, top0) = (w.high_schools[low].prestige, w.high_schools[top].prestige);
    alumni(&mut w, low, 30);
    let rng = w.rng.clone();
    climb(&mut w);
    // The climb draws nothing from the world's stream.
    assert_eq!(rng.clone().next(), w.rng.clone().next());
    // The low school takes the top place on the ladder, 15% of the way a year.
    let low1 = w.high_schools[low].prestige;
    assert!((low1 - (low0 + CLIMB_UP * (ladder_top - low0))).abs() < 1e-6, "{low0} -> {low1}");
    let c = &w.high_schools[low].chain;
    assert_eq!((c.college, c.power, c.nfl), (30, 30, 0));
    assert!((c.made - 45.0).abs() < 1e-3, "30 power starters at 1.5 each: {}", c.made);
    assert_eq!(c.years, vec![(w.year, low1 as f32)]);
    // The old top school, with nobody playing, falls a step, slowly.
    let top1 = w.high_schools[top].prestige;
    assert!(top1 < top0 && top0 - top1 < 0.08 * top0, "{top0} -> {top1}");
    // Years on, the riser holds the top place and the fallen school sits low.
    for y in 1..30 {
        w.year += 1;
        let _ = y;
        climb(&mut w);
    }
    assert!((w.high_schools[low].prestige - ladder_top).abs() < 0.02, "{}", w.high_schools[low].prestige);
    assert!(w.high_schools[low].is_powerhouse() || ladder_top < 0.75);
    // With nobody else playing either, it keeps the next place down.
    let mut ladder: Vec<f64> = schools.iter().map(|&h| w.high_schools[h].chain.start).collect();
    ladder.sort_by(|a, b| b.total_cmp(a));
    assert!(w.high_schools[top].prestige < top0, "{top0} -> {}", w.high_schools[top].prestige);
    assert!((w.high_schools[top].prestige - ladder[1]).abs() < 0.02, "{}", w.high_schools[top].prestige);
    // The fall is slower than the climb.
    const { assert!(CLIMB_DOWN < CLIMB_UP) };
    // Its why says where it stood and what its players do.
    let first = w.high_schools[low].chain.years[0].0;
    let why = high_school::chain_why(&w.high_schools[low], first);
    assert!(
        why.starts_with("climbed from") && why.contains("30 old players in college (30 at power programs)"),
        "{why}"
    );
    let fell = high_school::chain_why(&w.high_schools[top], first);
    assert!(fell.starts_with("fell back"), "{fell}");
}

#[test]
fn a_state_keeps_its_ladder_while_who_holds_it_moves() {
    let mut w = world();
    let schools = quiet_state(&mut w);
    let starts = |w: &World| {
        let mut v: Vec<f64> = schools.iter().map(|&h| w.high_schools[h].chain.start).collect();
        v.sort_by(|a, b| a.total_cmp(b));
        v
    };
    let before = starts(&w);
    // Every school's old players, in reverse of its name.
    for (k, &h) in schools.iter().enumerate() {
        alumni(&mut w, h, schools.len() - k);
    }
    for _ in 0..60 {
        w.year += 1;
        climb(&mut w);
    }
    let mut now: Vec<f64> = schools.iter().map(|&h| w.high_schools[h].prestige).collect();
    now.sort_by(|a, b| a.total_cmp(b));
    for (a, b) in now.iter().zip(&before) {
        assert!((a - b).abs() < 0.01, "the ladder holds: {a} against {b}");
    }
    // The order turned over: the old bottom is on top.
    assert!(w.high_schools[schools[0]].prestige > w.high_schools[*schools.last().unwrap()].prestige);
    assert_eq!(starts(&w), before, "the starts never move");
}

#[test]
fn nobody_playing_means_nobody_moves() {
    let mut w = world();
    let schools = quiet_state(&mut w);
    let before: Vec<f64> = schools.iter().map(|&h| w.high_schools[h].prestige).collect();
    climb(&mut w);
    let after: Vec<f64> = schools.iter().map(|&h| w.high_schools[h].prestige).collect();
    assert_eq!(before, after, "ties keep today's order and today's places");
}

#[test]
fn old_players_count_by_where_they_play() {
    let mut w = world();
    let i = w.people.len();
    alumni(&mut w, 0, 1);
    w.people[i].stage = Stage::HighSchool;
    assert!(alumni_value(&w, &w.people[i]).is_none(), "a high schooler is no old player");
    let fcs = (0..w.programs.len()).find(|&c| w.programs[c].division == Division::FCS).unwrap();
    let p = &mut w.people[i];
    p.stage = Stage::College;
    p.college = Some(fcs);
    p.last_role = Some(PlayingTime::Backup);
    assert_eq!(alumni_value(&w, &w.people[i]), Some((0.3, false, false)));
    w.people[i].stage = Stage::Done;
    assert!(alumni_value(&w, &w.people[i]).is_none(), "out of football counts nothing");
}

#[test]
fn the_state_rules_and_move_ins() {
    let az = transfer_rule(
        "Transfers sit the first 50% of regular-season varsity games (since 2016), even with a residence change",
    );
    assert!(!az.move_exempt && az.sit_out == 0.5);
    let ga = transfer_rule(
        "Without bona fide move: 365 days ineligible at varsity. Bona fide whole-family move gives immediate eligibility.",
    );
    assert!(ga.move_exempt && ga.sit_out == 1.0);
    assert!((move_in_chance(0.0) - 0.1).abs() < 1e-9 && (move_in_chance(1.0) - 0.6).abs() < 1e-9);
    // Every real state's rule but Arizona's and Tennessee's lets a family move free him.
    let w = world();
    let strict: Vec<&str> = w.states.iter().filter(|s| !s.transfer.move_exempt).map(|s| s.code.as_str()).collect();
    assert_eq!(strict, vec!["AZ", "TN"], "Tennessee: later transfers sit even with a residence change");
}

#[test]
fn an_older_save_gets_its_starts_and_loads_without_the_chain() {
    let mut w = world();
    let h = 0;
    let mut json = serde_json::to_value(&w.high_schools[h]).unwrap();
    json.as_object_mut().unwrap().remove("chain");
    let old: crate::world::high_school::HighSchool = serde_json::from_value(json).unwrap();
    assert_eq!(old.chain.start, 0.0);
    w.high_schools[h] = old;
    high_school::chain_after_load(&mut w);
    assert_eq!(w.high_schools[h].chain.start, w.high_schools[h].prestige);
    let _ = Chain::new(0.5);
    // A state rule saved before move-ins reads as move exempt.
    let mut rule = serde_json::to_value(w.states[0].transfer).unwrap();
    rule.as_object_mut().unwrap().remove("move_exempt");
    let rule: high_school::TransferRule = serde_json::from_value(rule).unwrap();
    assert!(rule.move_exempt);
}
