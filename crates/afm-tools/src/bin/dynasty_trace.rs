//! Dynasty trace: builds the world and, for each season listed in
//! `AFM_CAPTURE`, takes every FBS program's engine team as that season kicked
//! off. Then it splits each program's edge on the match engine into players,
//! play callers and schemes:
//! 1. Every FBS team plays the median team as is, players only (both sides
//!    get the same average staff) and staff only (both sides get the median
//!    team's players), against the lineup strength the world rates it at.
//! 2. The best programs play a round robin five ways: as is, players only,
//!    staff only, callers only and schemes only.
//!
//! Usage: AFM_CAPTURE=2016,2025 dynasty_trace [seed] [years] [teams] [games]

use afm_engine::debug;
use afm_engine::game::{GameTeam, OffScheme, Staff, quick_game};
use afm_engine::player::Level;
use afm_engine::positions::Position;
use afm_engine::world::data::Division;
use afm_engine::world::{World, WorldOptions};
use rayon::prelude::*;

/// Margin of `a` against `b`, home and away in turn.
fn margin(a: &GameTeam, b: &GameTeam, g: usize, seed: u64) -> f64 {
    if g % 2 == 0 {
        let r = quick_game(a, b, Level::FBS, seed);
        (r.stats.score[0] - r.stats.score[1]) as f64
    } else {
        let r = quick_game(b, a, Level::FBS, seed);
        (r.stats.score[1] - r.stats.score[0]) as f64
    }
}

/// Average margin of each team against the rest, home and away in turn.
fn round_robin(teams: &[GameTeam], games: usize, seed: u64) -> Vec<f64> {
    let n = teams.len();
    let pairs: Vec<(usize, usize, usize)> =
        (0..n).flat_map(|i| (i + 1..n).flat_map(move |j| (0..games).map(move |g| (i, j, g)))).collect();
    let out: Vec<(usize, usize, f64)> = pairs
        .par_iter()
        .map(|&(i, j, g)| {
            let s = seed.wrapping_mul(1_000_003).wrapping_add((i * 1000 + j) as u64 * 10_007 + g as u64);
            (i, j, margin(&teams[i], &teams[j], g, s))
        })
        .collect();
    let mut sum = vec![0.0; n];
    for (i, j, m) in out {
        sum[i] += m;
        sum[j] -= m;
    }
    sum.iter().map(|s| s / ((n - 1) * games) as f64).collect()
}

/// Average margin of each team against one opponent.
fn against(teams: &[GameTeam], opp: &[GameTeam], games: usize, seed: u64) -> Vec<f64> {
    let jobs: Vec<(usize, usize)> = (0..teams.len()).flat_map(|i| (0..games).map(move |g| (i, g))).collect();
    let out: Vec<(usize, f64)> = jobs
        .par_iter()
        .map(|&(i, g)| {
            let s = seed.wrapping_mul(1_000_003).wrapping_add(i as u64 * 10_007 + g as u64);
            (i, margin(&teams[i], &opp[i], g, s))
        })
        .collect();
    let mut sum = vec![0.0; teams.len()];
    for (i, m) in out {
        sum[i] += m;
    }
    sum.iter().map(|s| s / games as f64).collect()
}

/// Least-squares line y = a + b x.
fn line(x: &[f64], y: &[f64]) -> (f64, f64) {
    let n = x.len() as f64;
    let (mx, my) = (x.iter().sum::<f64>() / n, y.iter().sum::<f64>() / n);
    let sxy: f64 = x.iter().zip(y).map(|(a, b)| (a - mx) * (b - my)).sum();
    let sxx: f64 = x.iter().map(|a| (a - mx).powi(2)).sum();
    let b = sxy / sxx;
    (my - b * mx, b)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(2026);
    let years: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(15);
    let top: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(12);
    let games: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(40);
    if std::env::var("AFM_CAPTURE").is_err() {
        eprintln!("Set AFM_CAPTURE to the seasons to trace, e.g. AFM_CAPTURE=2016,2025.");
        return;
    }
    let w = World::new(WorldOptions { seed, history_years: years, ..Default::default() });
    let neutral = Staff::average(OffScheme::WestCoast);
    println!("# Dynasty trace: seed {seed}, {years} years, {games} games per pairing\n");

    for (year, teams, strength) in debug::captured() {
        let record = |p: usize| {
            w.programs[p].seasons.iter().find(|s| s.year == year).map_or(String::new(), |s| {
                format!("{}-{}{}", s.wins, s.losses, if s.national_title { ", champion" } else { "" })
            })
        };
        let mut fbs: Vec<usize> =
            w.programs.iter().filter(|p| p.division == Division::FBS && teams[p.id].is_some()).map(|p| p.id).collect();
        fbs.sort_by(|&a, &b| strength[b].total_cmp(&strength[a]));
        let side: Vec<GameTeam> = fbs.iter().map(|&p| teams[p].clone().unwrap()).collect();

        // 1. Every FBS team against the median team.
        let median = side[side.len() / 2].clone();
        // AFM_SPLIT_ONLY skips parts 1 and 2.
        if std::env::var("AFM_SPLIT_ONLY").is_err() {
            let opp = vec![median.clone(); side.len()];
            let as_is = against(&side, &opp, games, 11);
            let n_side: Vec<GameTeam> = side.iter().map(|t| GameTeam { staff: neutral, ..t.clone() }).collect();
            let n_opp = vec![GameTeam { staff: neutral, ..median.clone() }; side.len()];
            let players = against(&n_side, &n_opp, games, 12);
            let s_side: Vec<GameTeam> =
                side.iter().map(|t| GameTeam { players: median.players.clone(), ..t.clone() }).collect();
            let staff = against(&s_side, &opp, games, 13);
            let x: Vec<f64> = fbs.iter().map(|&p| strength[p]).collect();
            let (a0, b0) = line(&x, &as_is);
            let (a1, b1) = line(&x, &players);
            println!("## {year}: every FBS team against the median team ({})\n", median.name);
            println!(
                "Margin per lineup point: as is {b0:.2}, players only {b1:.2}. Residual = margin minus what the lineup predicts.\n"
            );
            println!("| Program | {year} | lineup | as is | residual | players only | residual | staff only |");
            println!("|---|---|---|---|---|---|---|---|");
            let mut order: Vec<usize> = (0..fbs.len()).collect();
            order.sort_by(|&i, &j| (as_is[j] - a0 - b0 * x[j]).total_cmp(&(as_is[i] - a0 - b0 * x[i])));
            let mut shown: Vec<usize> = order[..10].to_vec();
            shown.extend((0..top).filter(|k| !order[..10].contains(k)));
            for k in shown {
                let p = fbs[k];
                println!(
                    "| {} | {} | {:.1} | {:+.1} | {:+.1} | {:+.1} | {:+.1} | {:+.1} |",
                    w.programs[p].real_name,
                    record(p),
                    x[k],
                    as_is[k],
                    as_is[k] - a0 - b0 * x[k],
                    players[k],
                    players[k] - a1 - b1 * x[k],
                    staff[k]
                );
            }

            // 2. The best programs, round robin.
            let best: Vec<GameTeam> = side.iter().take(top).cloned().collect();
            let reference = best[best.len() / 2].players.clone();
            let po: Vec<GameTeam> = best.iter().map(|t| GameTeam { staff: neutral, ..t.clone() }).collect();
            let so: Vec<GameTeam> = best.iter().map(|t| GameTeam { players: reference.clone(), ..t.clone() }).collect();
            let co: Vec<GameTeam> = best
                .iter()
                .map(|t| {
                    let mut s = t.staff;
                    s.off_scheme = neutral.off_scheme;
                    s.shell = neutral.shell;
                    s.front = neutral.front;
                    GameTeam { players: reference.clone(), staff: s, name: t.name.clone() }
                })
                .collect();
            let sc: Vec<GameTeam> = best
                .iter()
                .map(|t| {
                    let mut s = neutral;
                    s.off_scheme = t.staff.off_scheme;
                    s.shell = t.staff.shell;
                    s.front = t.staff.front;
                    GameTeam { players: reference.clone(), staff: s, name: t.name.clone() }
                })
                .collect();
            let r = [
                round_robin(&best, games, 1),
                round_robin(&po, games, 2),
                round_robin(&so, games, 3),
                round_robin(&co, games, 4),
                round_robin(&sc, games, 5),
            ];
            println!("\n## {year}: the top {top} lineups, round robin (average margin against the rest)\n");
            println!(
                "| Program | {year} | lineup | offense | shell | OC / DC calling | as is | players only | staff only | callers only | schemes only |"
            );
            println!("|---|---|---|---|---|---|---|---|---|---|---|");
            for (k, t) in best.iter().enumerate() {
                let p = fbs[k];
                let st = &t.staff;
                println!(
                    "| {} | {} | {:.1} | {:?} | {:?} | {:.1} / {:.1} | {:+.1} | {:+.1} | {:+.1} | {:+.1} | {:+.1} |",
                    w.programs[p].real_name,
                    record(p),
                    x[k],
                    st.off_scheme,
                    st.shell,
                    st.offense.skill,
                    st.defense.skill,
                    r[0][k],
                    r[1][k],
                    r[2][k],
                    r[3][k],
                    r[4][k]
                );
            }
        }

        // 3. Where a top roster's edge sits: swap one position group at a
        // time for the median team's players (players only, same staff).
        println!("\n## {year}: where each top roster's edge over the median team sits (players only)\n");
        let groups = [
            Position::QB,
            Position::RB,
            Position::WR,
            Position::TE,
            Position::OT,
            Position::IOL,
            Position::EDGE,
            Position::DT,
            Position::LB,
            Position::CB,
            Position::S,
        ];
        print!("| Program | edge |");
        for g in groups {
            print!(" {g:?} |");
        }
        println!("\n|---|---|{}", "---|".repeat(groups.len()));
        let m = GameTeam { staff: neutral, ..median.clone() };
        for k in 0..4.min(fbs.len()) {
            let t = GameTeam { staff: neutral, ..side[k].clone() };
            let base = against(std::slice::from_ref(&t), std::slice::from_ref(&m), games * 5, 21)[0];
            let swapped: Vec<GameTeam> = groups
                .iter()
                .map(|&g| {
                    let mut players: Vec<_> = t.players.iter().filter(|p| p.position != g).cloned().collect();
                    players.extend(m.players.iter().filter(|p| p.position == g).cloned().map(|mut p| {
                        p.id += 5_000_000;
                        p
                    }));
                    GameTeam { players, ..t.clone() }
                })
                .collect();
            let opp = vec![m.clone(); swapped.len()];
            let r = against(&swapped, &opp, games * 5, 22);
            print!("| {} | {base:+.1} |", w.programs[fbs[k]].real_name);
            for v in r {
                print!(" {:+.1} |", base - v);
            }
            println!();
        }
        println!("\nEach cell: how much of the edge goes when that group is swapped for the median team's.");
        println!();
    }
}
