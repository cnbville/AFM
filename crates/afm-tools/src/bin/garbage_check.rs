//! Garbage-time check (dynasty item 3): play many games between synthetic
//! teams with a real spread of quality and compare the 4th quarter, by the
//! score after three quarters, with real play-by-play
//! (`data/real-world/garbage_time.json`, same definitions).
//! Usage: garbage_check [fbs|nfl|hs] [games] [quality sd] [seed]

use afm_engine::game::stats::{GameStats, PlayType};
use afm_engine::game::synthetic::synthetic_team;
use afm_engine::game::{GameTeam, OffScheme, quick_game};
use afm_engine::player::Level;
use afm_engine::rng::Rng;
use rayon::prelude::*;
use serde_json::Value;
use std::collections::HashMap;

const BUCKETS: [&str; 4] = ["28+", "21-27", "15-20", "1-14"];

fn bucket(lead: i32) -> Option<usize> {
    match lead {
        28.. => Some(0),
        21..=27 => Some(1),
        15..=20 => Some(2),
        1..=14 => Some(3),
        _ => None,
    }
}

#[derive(Default, Clone)]
struct Acc {
    games: f64,
    plays: f64,
    passes: f64,
    pts: f64,
    allowed: f64,
    kneels: f64,
    gap: f64,
    gaps: f64,
    qb1_any: f64,
    qb1_games: f64,
    qb1_passes: f64,
    all_passes: f64,
    rb1_rush: f64,
    all_rush: f64,
    // 4th downs: [own, mid, fg] x [go, fg, punt]
    fourth: [[f64; 3]; 3],
}

fn scrimmage(k: PlayType) -> bool {
    matches!(k, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble)
}

/// One side's 4th quarter, the way the real script measures it.
fn add(acc: &mut Acc, s: &GameStats, side: usize) {
    acc.games += 1.0;
    acc.pts += s.quarters[3][side] as f64;
    acc.allowed += s.quarters[3][1 - side] as f64;
    // QB1 and RB1 from quarters 1-3.
    let mut passers: HashMap<u32, u32> = HashMap::new();
    let mut runners: HashMap<u32, u32> = HashMap::new();
    for p in s.plays.iter().filter(|p| p.quarter <= 3 && p.offense == side) {
        if matches!(p.kind, PlayType::Pass | PlayType::Sack)
            && let Some(q) = p.passer
        {
            *passers.entry(q).or_default() += 1;
        }
        if p.kind == PlayType::Run
            && let Some(c) = p.carrier
        {
            *runners.entry(c).or_default() += 1;
        }
    }
    let qb1 = passers.iter().max_by_key(|x| (x.1, x.0)).map(|x| *x.0);
    let rb1 = runners.iter().filter(|x| Some(*x.0) != qb1).max_by_key(|x| (x.1, x.0)).map(|x| *x.0);
    let q4: Vec<_> = s.plays.iter().filter(|p| p.quarter == 4).collect();
    let mut qb1_seen = false;
    let mut any = false;
    for (i, p) in q4.iter().enumerate() {
        if p.offense != side {
            continue;
        }
        if p.kind == PlayType::Kneel {
            acc.kneels += 1.0;
            continue;
        }
        if p.down == 4
            && matches!(
                p.kind,
                PlayType::Run
                    | PlayType::Pass
                    | PlayType::Sack
                    | PlayType::Scramble
                    | PlayType::Punt
                    | PlayType::FieldGoal
            )
        {
            let zone = if p.yardline_100 > 50.0 {
                0
            } else if p.yardline_100 > 34.0 {
                1
            } else {
                2
            };
            let what = match p.kind {
                PlayType::FieldGoal => 1,
                PlayType::Punt => 2,
                _ => 0,
            };
            acc.fourth[zone][what] += 1.0;
        }
        if !scrimmage(p.kind) {
            continue;
        }
        any = true;
        acc.plays += 1.0;
        let pass = matches!(p.kind, PlayType::Pass | PlayType::Sack);
        if pass {
            acc.passes += 1.0;
            acc.all_passes += 1.0;
            if p.passer.is_some() && p.passer == qb1 {
                acc.qb1_passes += 1.0;
            }
        } else {
            acc.all_rush += 1.0;
            if p.carrier.is_some() && p.carrier == rb1 {
                acc.rb1_rush += 1.0;
            }
        }
        if qb1.is_some() && (p.passer == qb1 || p.carrier == qb1) {
            qb1_seen = true;
        }
        if let Some(n) = q4.get(i + 1)
            && n.offense == side
            && scrimmage(n.kind)
            && n.clock < p.clock
        {
            acc.gap += p.clock - n.clock;
            acc.gaps += 1.0;
        }
    }
    if any && qb1.is_some() {
        acc.qb1_games += 1.0;
        if qb1_seen {
            acc.qb1_any += 1.0;
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (level, key) = match args.get(1).map(|s| s.as_str()) {
        Some("nfl") => (Level::NFL, "nfl"),
        Some("hs") => (Level::HS, "fbs"),
        _ => (Level::FBS, "fbs"),
    };
    let games: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(2000);
    let sd: f64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(match level {
        Level::NFL => 2.0,
        _ => 3.5,
    });
    let seed: u32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(1);
    let schemes =
        [OffScheme::WestCoast, OffScheme::AirRaid, OffScheme::WideZone, OffScheme::PowerGap, OffScheme::SpreadOption];
    let mut rng = Rng::new(seed ^ 0x77);
    let teams: Vec<GameTeam> = (0..48)
        .map(|k| {
            let q = rng.normal(0.0, sd);
            synthetic_team(
                &format!("T{k}"),
                level,
                q,
                schemes[k % 5],
                seed.wrapping_mul(7919).wrapping_add(k as u32 * 104_729),
            )
        })
        .collect();
    let t0 = std::time::Instant::now();
    let results: Vec<_> = (0..games)
        .into_par_iter()
        .map(|g| {
            let h = (g * 7) % 48;
            let a = (g * 13 + 5) % 48;
            let a = if a == h { (a + 1) % 48 } else { a };
            quick_game(&teams[h], &teams[a], level, seed as u64 * 1_000_003 + g as u64)
        })
        .collect();
    let mut lead = vec![Acc::default(); 4];
    let mut trail = vec![Acc::default(); 4];
    // QB1 still playing in the 4th by finer lead bins: [lead, trail] x bins.
    const FINE: [(i32, i32, &str); 4] = [(21, 27, "21-27"), (28, 34, "28-34"), (35, 41, "35-41"), (42, 999, "42-+")];
    let mut fine = vec![[Acc::default(), Acc::default()]; 4];
    let mut margins = Vec::new();
    let mut decisions = 0usize;
    let mut sample: Vec<String> = Vec::new();
    for r in &results {
        let s = &r.stats;
        margins.push((s.score[0] - s.score[1]) as f64);
        decisions += r.decisions.len();
        if sample.len() < 8 {
            sample.extend(r.decisions.iter().take(2).map(|d| d.2.clone()));
        }
        if s.quarters.len() < 4 {
            continue;
        }
        let after3 = [0, 1].map(|t| s.quarters[..3].iter().map(|q| q[t]).sum::<i32>());
        for side in 0..2 {
            let l = after3[side] - after3[1 - side];
            if let Some(b) = bucket(l) {
                add(&mut lead[b], s, side);
            }
            if let Some(b) = bucket(-l) {
                add(&mut trail[b], s, side);
            }
            for (k, &(lo, hi, _)) in FINE.iter().enumerate() {
                if (lo..=hi).contains(&l) {
                    add(&mut fine[k][0], s, side);
                }
                if (lo..=hi).contains(&-l) {
                    add(&mut fine[k][1], s, side);
                }
            }
        }
    }
    let n = margins.len() as f64;
    let mean = margins.iter().sum::<f64>() / n;
    let msd = (margins.iter().map(|m| (m - mean).powi(2)).sum::<f64>() / n).sqrt();
    let share = |t: f64| 100.0 * margins.iter().filter(|m| m.abs() >= t).count() as f64 / n;
    println!(
        "{:?}: {} games in {:.1}s, quality sd {sd}; final margin sd {:.1}, 28+ {:.1}%, 40+ {:.1}%, 60+ {:.1}%; {:.2} garbage-time calls a game",
        level,
        games,
        t0.elapsed().as_secs_f64(),
        msd,
        share(28.0),
        share(40.0),
        share(60.0),
        decisions as f64 / n
    );
    for d in sample.iter().take(6) {
        println!("  e.g. {d}");
    }
    let real: Value = serde_json::from_str(include_str!("../../../../data/real-world/garbage_time.json")).unwrap();
    let rv = |side: &str, b: &str, k: &str| real[key][side][b][k].as_f64().unwrap_or(f64::NAN);
    println!("\nQB1 still plays in the 4th quarter, by the score after 3 (engine | real):");
    for (k, &(_, _, name)) in FINE.iter().enumerate() {
        let r = |side: &str| {
            real["qb1_any_q4_by_lead"][key][format!("{side} {name}")]["qb1_any_q4"].as_f64().unwrap_or(f64::NAN)
        };
        let e = |a: &Acc| a.qb1_any / a.qb1_games.max(1.0);
        println!(
            "  {:<6} ahead {:.2}|{:.2} (n {:.0})   behind {:.2}|{:.2}",
            name,
            e(&fine[k][0]),
            r("lead"),
            fine[k][0].qb1_games,
            e(&fine[k][1]),
            r("trail")
        );
    }
    for (side, accs) in [("lead", &lead), ("trail", &trail)] {
        println!(
            "\n{} after 3 quarters ({}): engine | real {}",
            side,
            if side == "lead" { "ahead" } else { "behind" },
            key
        );
        println!(
            "{:<6} {:>6} {:>13} {:>13} {:>13} {:>13} {:>13} {:>13} {:>13} {:>13}",
            "",
            "games",
            "Q4 pass",
            "Q4 pts",
            "Q4 allowed",
            "Q4 plays",
            "secs/play",
            "QB1 any Q4",
            "QB1 pass sh",
            "RB1 rush sh"
        );
        for (b, a) in accs.iter().enumerate() {
            let g = a.games.max(1.0);
            let f = |x: f64, k: &str| format!("{:.2}|{:.2}", x, rv(side, BUCKETS[b], k));
            println!(
                "{:<6} {:>6.0} {:>13} {:>13} {:>13} {:>13} {:>13} {:>13} {:>13} {:>13}",
                BUCKETS[b],
                a.games,
                f(a.passes / a.plays.max(1.0), "q4_pass_rate"),
                f(a.pts / g, "q4_points_mean"),
                f(a.allowed / g, "q4_points_allowed_mean"),
                f(a.plays / g, "q4_plays_mean"),
                format!(
                    "{:.1}|{:.1}",
                    a.gap / a.gaps.max(1.0),
                    if key == "fbs" {
                        rv(side, BUCKETS[b], "q4_sec_per_play_drive_based")
                    } else {
                        rv(side, BUCKETS[b], "q4_sec_per_play_play_level")
                    }
                ),
                f(a.qb1_any / a.qb1_games.max(1.0), "pct_games_qb1_takes_any_q4_snap_given_any_snap"),
                f(a.qb1_passes / a.all_passes.max(1.0), "qb1_share_of_q4_passes"),
                f(a.rb1_rush / a.all_rush.max(1.0), "rb1_share_of_q4_rushes"),
            );
        }
        println!("  4th downs in the 4th quarter (go/fg/punt), engine | real:");
        for (b, a) in accs.iter().enumerate() {
            let zone = |z: usize, name: &str| {
                let t: f64 = a.fourth[z].iter().sum::<f64>().max(1.0);
                let r = |k: &str| rv(side, BUCKETS[b], &format!("q4_4th_{name}_{k}"));
                format!(
                    "{name} {:.0}: {:.2}/{:.2}/{:.2} | {:.2}/{:.2}/{:.2}",
                    t,
                    a.fourth[z][0] / t,
                    a.fourth[z][1] / t,
                    a.fourth[z][2] / t,
                    r("go"),
                    if name == "own" { 0.0 } else { r("fg") },
                    if name == "fg" { 0.0 } else { r("punt") }
                )
            };
            println!("  {:<6} {}   {}   {}", BUCKETS[b], zone(0, "own"), zone(1, "mid"), zone(2, "fg"));
        }
    }
}
