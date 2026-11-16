//! The chain reaction (Ground GR21): a high school climbs on its old
//! players in college and the NFL and slowly falls back when they stop.

use crate::player::PlayingTime;
use crate::world::data::Division;
use crate::world::high_school::{
    self, CLIMB_CAP, CLIMB_DOWN, CLIMB_MIN_SCHOOLS, CLIMB_UP, Chain, alumni_value, climb, state_moves, transfer_rule,
};
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

/// Old players in line with each school's name: about 20 at the top of
/// the state, none at the bottom.
fn in_line(w: &mut World, schools: &[usize]) {
    for &h in schools {
        let n = (w.high_schools[h].prestige * 20.0).round() as usize;
        alumni(w, h, n);
    }
}

fn mean(w: &World, schools: &[usize]) -> f64 {
    schools.iter().map(|&h| w.high_schools[h].prestige).sum::<f64>() / schools.len() as f64
}

#[test]
fn a_school_whose_players_make_it_climbs_and_a_quiet_one_falls_slowly() {
    let mut w = world();
    let schools = quiet_state(&mut w);
    assert!(schools.len() >= CLIMB_MIN_SCHOOLS, "a state with schools");
    let (low, top) = (schools[0], *schools.last().unwrap());
    let (low0, top0) = (w.high_schools[low].prestige, w.high_schools[top].prestige);
    let mean0 = mean(&w, &schools);
    in_line(&mut w, &schools);
    // The low school sends 30 to power programs; the top school's stop.
    alumni(&mut w, low, 30);
    let top_count = (top0 * 20.0).round() as usize;
    let mut gone = 0;
    for p in w.people.iter_mut().filter(|p| p.hs == top && p.stage == Stage::College) {
        p.stage = Stage::Done;
        gone += 1;
    }
    assert_eq!(gone, top_count);
    let rng = w.rng.clone();
    climb(&mut w);
    // The climb draws nothing from the world's stream.
    assert_eq!(rng.clone().next(), w.rng.clone().next());
    // A year's step: the riser up at most its cap, the quiet one down, slower.
    let (low1, top1) = (w.high_schools[low].prestige, w.high_schools[top].prestige);
    assert!(low1 > low0 && low1 - low0 <= CLIMB_UP * CLIMB_CAP + 0.01, "{low0} -> {low1}");
    assert!(top1 < top0 && top0 - top1 <= CLIMB_DOWN * CLIMB_CAP + 0.01, "{top0} -> {top1}");
    assert!(low1 - low0 > top0 - top1, "a climb takes hold faster than a name fades");
    // The state's average holds.
    assert!((mean(&w, &schools) - mean0).abs() < 1e-3, "{mean0} -> {}", mean(&w, &schools));
    let c = &w.high_schools[low].chain;
    assert_eq!(
        (c.college as usize, c.power as usize, c.nfl),
        ((low0 * 20.0).round() as usize + 30, (low0 * 20.0).round() as usize + 30, 0)
    );
    assert_eq!(c.years, vec![(w.year, low1 as f32)]);
    // Years on, the riser has climbed far and the quiet school fallen.
    let first = w.year;
    for _ in 1..15 {
        w.year += 1;
        climb(&mut w);
    }
    assert!(w.high_schools[low].prestige > low0 + 0.3, "{low0} -> {}", w.high_schools[low].prestige);
    assert!(w.high_schools[top].prestige < top0 - 0.1, "{top0} -> {}", w.high_schools[top].prestige);
    // Its why says where it stood and what its players do.
    let why = high_school::chain_why(&w.high_schools[low], first);
    assert!(why.starts_with("climbed from") && why.contains("old players in college"), "{why}");
    let fell = high_school::chain_why(&w.high_schools[top], first);
    assert!(fell.starts_with("fell back"), "{fell}");
}

#[test]
fn the_order_follows_the_players_and_the_state_average_holds() {
    let mut w = world();
    let schools = quiet_state(&mut w);
    let mean0 = mean(&w, &schools);
    // Every school's old players, in reverse of its name.
    for (k, &h) in schools.iter().enumerate() {
        alumni(&mut w, h, schools.len() - k);
    }
    for _ in 0..60 {
        w.year += 1;
        climb(&mut w);
    }
    // The order turned over: the old bottom is on top.
    assert!(w.high_schools[schools[0]].prestige > w.high_schools[*schools.last().unwrap()].prestige);
    assert!((mean(&w, &schools) - mean0).abs() < 0.03, "{mean0} -> {}", mean(&w, &schools));
    let starts: Vec<f64> = schools.iter().map(|&h| w.high_schools[h].chain.start).collect();
    assert!(starts.iter().zip(&schools).all(|(s, &h)| *s > 0.0 && w.high_schools[h].chain.years.len() == 60));
}

#[test]
fn a_small_state_holds() {
    let few: Vec<(f64, f64)> = (0..CLIMB_MIN_SCHOOLS - 1).map(|k| (k as f64 / 10.0, (10 - k) as f64)).collect();
    assert!(state_moves(&few).iter().all(|&d| d == 0.0));
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
