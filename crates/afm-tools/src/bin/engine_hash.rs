//! Proof and timing tool for engine speed work: plays `games` games at each
//! level between synthetic teams (as `match_check` does) and prints, for
//! each part of a game's result (stats, injuries, film, talks and the
//! rest), one fingerprint over every game in order, then the time the
//! games took. Two builds that give every game the same result print the
//! same fingerprints; a speed-up that changes no game changes no line but
//! the time (`wlog::fp::field_prints`, map order left out).
//! Usage: engine_hash [games] [seed] [hs|fbs|nfl ...]
use afm_engine::game::synthetic::synthetic_league;
use afm_engine::game::{Conditions, Venue, quick_game_in};
use afm_engine::player::Level;
use afm_engine::world::wlog::fp::field_prints;
use rayon::prelude::*;

#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let games: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(400);
    let seed: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
    let mut levels: Vec<(&str, Level)> = args[3.min(args.len())..]
        .iter()
        .filter_map(|k| match k.as_str() {
            "hs" => Some(("hs", Level::HS)),
            "fbs" => Some(("fbs", Level::FBS)),
            "nfl" => Some(("nfl", Level::NFL)),
            _ => None,
        })
        .collect();
    if levels.is_empty() {
        levels = vec![("hs", Level::HS), ("fbs", Level::FBS), ("nfl", Level::NFL)];
    }
    for (key, level) in levels {
        let teams = synthetic_league(level, 32, seed);
        let n = teams.len();
        let t0 = std::time::Instant::now();
        let prints: Vec<Vec<(&'static str, u64, u64)>> = (0..games)
            .into_par_iter()
            .map(|g| {
                let (h, a) = (g % n, (g * 7 + 3) % n);
                let a = if a == h { (a + 1) % n } else { a };
                let venue = Venue { cond: Conditions { crowd: 0.65, ..Conditions::default() }, ..Venue::default() };
                let r = quick_game_in(
                    &teams[h],
                    &teams[a],
                    level,
                    seed as u64 * 1_000_003 + g as u64,
                    Default::default(),
                    venue,
                );
                field_prints(&r)
            })
            .collect();
        let secs = t0.elapsed().as_secs_f64();
        // Each part's prints folded over the games in order.
        let mut parts: Vec<(&'static str, u64)> = Vec::new();
        for game in &prints {
            for (k, &(name, print, _)) in game.iter().enumerate() {
                if parts.len() <= k {
                    parts.push((name, 0xcbf2_9ce4_8422_2325));
                }
                parts[k].1 = (parts[k].1 ^ print).wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
        let line: Vec<String> = parts.iter().map(|(name, h)| format!("{name}={h:016x}")).collect();
        println!("{key} {games} games seed {seed}: {}", line.join(" "));
        eprintln!("{key}: {secs:.1}s");
    }
}
