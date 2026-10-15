//! Dynasty proof: replays the rosters of the seasons in `AFM_CAPTURE` with
//! rough versions of the unanswered trial 4 fixes switched on, to see whether
//! they bring the best team's edge down to the real best team's before any
//! of them is built for real. Prints CSV (variant, year, kind, home, away,
//! margin) for `docs/trials/dynasty-proof.py`:
//! - `rr`: the top 12 lineups, round robin, home and away in turn.
//! - `season`: every FBS team plays 24 random FBS opponents.
//!
//! Variants (rough stand-ins, not the designs):
//! - `as_is`: the engine today.
//! - `pos`: position values (proposal 1). Each player's attributes move
//!   toward his position's FBS average so that a rating point there is worth
//!   the middle of the proposed band instead of what the engine pays now.
//! - `garbage`: garbage time (proposal 3, `game::experiment`).
//! - `pos_garbage`: both.
//! - `all3`: both, and every team on the same schemes (proposal 2's "no
//!   scheme wins by itself", at its limit).
//!
//! Usage: AFM_CAPTURE=2021,2025 dynasty_proof [seed] [years] [games] > out.csv

use afm_engine::attributes::ATTRIBUTE_COUNT;
use afm_engine::debug;
use afm_engine::game::{GameTeam, OffScheme, Staff, experiment, quick_game};
use afm_engine::player::Level;
use afm_engine::positions::Position;
use afm_engine::world::data::Division;
use afm_engine::world::{World, WorldOptions};
use rayon::prelude::*;

/// Share of the quarterback's value per rating point the engine pays now
/// (`docs/match-engine.md`, measured position values) and the middle of the
/// proposed band. Lines are left alone: the rough version only shrinks.
fn shrink(pos: Position) -> f64 {
    let (now, band): (f64, f64) = match pos {
        Position::RB => (0.74, 0.10),
        Position::EDGE => (0.60, 0.225),
        Position::S => (0.45, 0.13),
        Position::WR => (0.44, 0.225),
        Position::LB => (0.42, 0.10),
        Position::DT => (0.41, 0.175),
        Position::TE => (0.39, 0.125),
        Position::CB => (0.37, 0.185),
        _ => (1.0, 1.0),
    };
    band / now
}

fn compress(teams: &[GameTeam]) -> Vec<GameTeam> {
    // Each position's FBS average, attribute by attribute.
    let mut sum: std::collections::HashMap<Position, ([f64; ATTRIBUTE_COUNT], f64)> = Default::default();
    for t in teams {
        for p in &t.players {
            let e = sum.entry(p.position).or_insert(([0.0; ATTRIBUTE_COUNT], 0.0));
            for k in 0..ATTRIBUTE_COUNT {
                e.0[k] += p.attrs.0[k];
            }
            e.1 += 1.0;
        }
    }
    teams
        .iter()
        .map(|t| {
            let mut t = t.clone();
            for p in &mut t.players {
                let f = shrink(p.position);
                let (s, n) = sum[&p.position];
                for k in 0..ATTRIBUTE_COUNT {
                    let m = s[k] / n;
                    p.attrs.0[k] = m + f * (p.attrs.0[k] - m);
                }
            }
            t
        })
        .collect()
}

fn margin(a: &GameTeam, b: &GameTeam, seed: u64) -> f64 {
    let r = quick_game(a, b, Level::FBS, seed);
    (r.stats.score[0] - r.stats.score[1]) as f64
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(2026);
    let years: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(15);
    let games: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(40);
    if std::env::var("AFM_CAPTURE").is_err() {
        eprintln!("Set AFM_CAPTURE to the seasons to replay, e.g. AFM_CAPTURE=2021,2025.");
        return;
    }
    let w = World::new(WorldOptions { seed, history_years: years, ..Default::default() });
    let neutral = Staff::average(OffScheme::WestCoast);
    println!("variant,year,kind,home,away,margin");
    for (year, teams, strength) in debug::captured() {
        let mut fbs: Vec<usize> =
            w.programs.iter().filter(|p| p.division == Division::FBS && teams[p.id].is_some()).map(|p| p.id).collect();
        fbs.sort_by(|&a, &b| strength[b].total_cmp(&strength[a]));
        let name: Vec<String> = fbs.iter().map(|&p| w.programs[p].real_name.replace(',', "")).collect();
        let base: Vec<GameTeam> = fbs.iter().map(|&p| teams[p].clone().unwrap()).collect();
        let squeezed = compress(&base);
        // The random schedule: 24 rounds of random pairings, the same for
        // every variant.
        let mut state = year as u64 * 7 + 3;
        let mut next = |n: usize| {
            state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            ((state >> 33) as usize) % n
        };
        let mut sched = Vec::new();
        for _ in 0..24 {
            let mut idx: Vec<usize> = (0..fbs.len()).collect();
            for k in (1..idx.len()).rev() {
                idx.swap(k, next(k + 1));
            }
            for c in idx.chunks(2).filter(|c| c.len() == 2) {
                sched.push((c[0], c[1]));
            }
        }
        let top = 12.min(fbs.len());
        let rr: Vec<(usize, usize, usize)> =
            (0..top).flat_map(|i| (i + 1..top).flat_map(move |j| (0..games).map(move |g| (i, j, g)))).collect();

        for variant in ["as_is", "pos", "garbage", "pos_garbage", "all3"] {
            experiment::set_garbage_time(variant.contains("garbage") || variant == "all3");
            let mut side =
                if variant.starts_with("pos") || variant == "all3" { squeezed.clone() } else { base.clone() };
            if variant == "all3" {
                for t in &mut side {
                    t.staff.off_scheme = neutral.off_scheme;
                    t.staff.front = neutral.front;
                    t.staff.shell = neutral.shell;
                }
            }
            let out: Vec<String> = rr
                .par_iter()
                .map(|&(i, j, g)| {
                    let s = (year as u64) * 1_000_003 + (i * 1000 + j) as u64 * 10_007 + g as u64;
                    let (h, a) = if g % 2 == 0 { (i, j) } else { (j, i) };
                    format!("{variant},{year},rr,{},{},{}", name[h], name[a], margin(&side[h], &side[a], s))
                })
                .collect();
            for l in out {
                println!("{l}");
            }
            let out: Vec<String> = sched
                .par_iter()
                .enumerate()
                .map(|(k, &(h, a))| {
                    let s = (year as u64) * 999_983 + k as u64;
                    format!("{variant},{year},season,{},{},{}", name[h], name[a], margin(&side[h], &side[a], s))
                })
                .collect();
            for l in out {
                println!("{l}");
            }
            eprintln!("{year} {variant} done");
        }
    }
    experiment::set_garbage_time(false);
}
