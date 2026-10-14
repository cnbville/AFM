//! Dynasty trace: builds the world, then splits the top FBS programs' edge on
//! the match engine into players, play callers and schemes. The best
//! programs play a round robin five ways: as they are; players only (every
//! side gets the same average staff); staff only (every side gets the same
//! reference roster); callers only and schemes only (the reference roster
//! with each staff's callers, or each staff's schemes).
//! Usage: dynasty_trace [seed] [years] [teams] [games per pair]

use afm_engine::game::{GameTeam, OffScheme, Staff, quick_game};
use afm_engine::player::Level;
use afm_engine::world::data::Division;
use afm_engine::world::{World, WorldOptions};
use rayon::prelude::*;

/// Average margin of each team against the rest, home and away in turn.
fn ratings(teams: &[GameTeam], games: usize, seed: u64) -> Vec<f64> {
    let n = teams.len();
    let pairs: Vec<(usize, usize, usize)> =
        (0..n).flat_map(|i| (i + 1..n).flat_map(move |j| (0..games).map(move |g| (i, j, g)))).collect();
    let out: Vec<(usize, usize, f64)> = pairs
        .par_iter()
        .map(|&(i, j, g)| {
            let s = seed.wrapping_mul(1_000_003).wrapping_add((i * 1000 + j) as u64 * 10_007 + g as u64);
            let m = if g % 2 == 0 {
                let r = quick_game(&teams[i], &teams[j], Level::FBS, s);
                (r.stats.score[0] - r.stats.score[1]) as f64
            } else {
                let r = quick_game(&teams[j], &teams[i], Level::FBS, s);
                (r.stats.score[1] - r.stats.score[0]) as f64
            };
            (i, j, m)
        })
        .collect();
    let mut sum = vec![0.0; n];
    for (i, j, m) in out {
        sum[i] += m;
        sum[j] -= m;
    }
    sum.iter().map(|s| s / ((n - 1) * games) as f64).collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(2026);
    let years: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(15);
    let top: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(12);
    let games: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(60);
    let w = World::new(WorldOptions { seed, history_years: years, ..Default::default() });

    let mut fbs: Vec<(usize, f64)> = w
        .programs
        .iter()
        .filter(|p| p.division == Division::FBS)
        .filter_map(|p| p.seasons.last().map(|s| (p.id, s.strength)))
        .collect();
    fbs.sort_by(|a, b| b.1.total_cmp(&a.1));
    let picked: Vec<(usize, f64)> = fbs.iter().take(top).copied().collect();
    let teams: Vec<GameTeam> = picked.iter().filter_map(|&(p, _)| w.debug_game_team(p)).collect();
    assert_eq!(teams.len(), picked.len());

    println!("# Dynasty trace: seed {seed}, {years} years, top {top} FBS programs, {games} games per pair\n");
    println!(
        "| Program | last season | lineup | offense | shell | front | OC skill/adapt | DC skill/adapt | HC game mgmt |"
    );
    println!("|---|---|---|---|---|---|---|---|---|");
    for (&(p, s), t) in picked.iter().zip(&teams) {
        let last = w.programs[p].seasons.last().unwrap();
        let st = &t.staff;
        println!(
            "| {} | {}-{} | {s:.1} | {:?} | {:?} | {:?} | {:.1}/{:.1} | {:.1}/{:.1} | {:.1} |",
            w.programs[p].real_name,
            last.wins,
            last.losses,
            st.off_scheme,
            st.shell,
            st.front,
            st.offense.skill,
            st.offense.adaptation,
            st.defense.skill,
            st.defense.adaptation,
            st.game_management
        );
    }

    // The reference roster for the staff-only runs: the median of the group.
    let reference = teams[teams.len() / 2].players.clone();
    let neutral = Staff::average(OffScheme::WestCoast);
    let as_is = teams.clone();
    let players_only: Vec<GameTeam> = teams.iter().map(|t| GameTeam { staff: neutral, ..t.clone() }).collect();
    let staff_only: Vec<GameTeam> =
        teams.iter().map(|t| GameTeam { players: reference.clone(), ..t.clone() }).collect();
    let callers_only: Vec<GameTeam> = teams
        .iter()
        .map(|t| {
            let mut s = t.staff;
            s.off_scheme = neutral.off_scheme;
            s.shell = neutral.shell;
            s.front = neutral.front;
            GameTeam { players: reference.clone(), staff: s, name: t.name.clone() }
        })
        .collect();
    let schemes_only: Vec<GameTeam> = teams
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
        ratings(&as_is, games, 1),
        ratings(&players_only, games, 2),
        ratings(&staff_only, games, 3),
        ratings(&callers_only, games, 4),
        ratings(&schemes_only, games, 5),
    ];
    println!("\nAverage margin against the rest of the group:\n");
    println!("| Program | lineup | as is | players only | staff only | callers only | schemes only |");
    println!("|---|---|---|---|---|---|---|");
    for (k, &(p, s)) in picked.iter().enumerate() {
        println!(
            "| {} | {s:.1} | {:+.1} | {:+.1} | {:+.1} | {:+.1} | {:+.1} |",
            w.programs[p].real_name, r[0][k], r[1][k], r[2][k], r[3][k], r[4][k]
        );
    }
}
