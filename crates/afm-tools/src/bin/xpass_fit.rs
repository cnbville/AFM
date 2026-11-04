//! The expected pass model behind pass rate over expected (Data overhaul
//! item 16, lane NB7), fitted on the match engine's own snaps at each level,
//! as nflfastR fits its expected pass rate on the NFL's: logistic regression
//! (Newton's method) of a dropback on the situation before the call. Prints
//! the weights for `world::tally::expect::XPASS`, with each situation's real
//! rate against the fit. Rerun after engine changes that move play calling:
//! `cargo run --release --bin xpass_fit -- 600 [seed]`

use afm_engine::game::flow::quick_game_in;
use afm_engine::game::stats::PlayType;
use afm_engine::game::synthetic::synthetic_league;
use afm_engine::player::Level;
use afm_engine::world::tally::expect::{XPASS_TERMS, pass_chance, pass_features_of};
use rayon::prelude::*;

type Row = ([f64; XPASS_TERMS], f64, u8, f64);

const N: usize = XPASS_TERMS;

fn sigmoid(z: f64) -> f64 {
    1.0 / (1.0 + (-z).exp())
}

#[allow(clippy::needless_range_loop)]
fn solve(mut h: [[f64; N]; N], mut g: [f64; N]) -> [f64; N] {
    for c in 0..N {
        let p = (c..N).max_by(|&a, &b| h[a][c].abs().total_cmp(&h[b][c].abs())).unwrap();
        h.swap(c, p);
        g.swap(c, p);
        if h[c][c].abs() < 1e-12 {
            continue;
        }
        for r in c + 1..N {
            let f = h[r][c] / h[c][c];
            for k in c..N {
                h[r][k] -= f * h[c][k];
            }
            g[r] -= f * g[c];
        }
    }
    let mut x = [0.0; N];
    for c in (0..N).rev() {
        if h[c][c].abs() < 1e-12 {
            continue;
        }
        x[c] = (g[c] - (c + 1..N).map(|k| h[c][k] * x[k]).sum::<f64>()) / h[c][c];
    }
    x
}

/// Logistic regression by Newton's method, a little ridge for stability.
fn logistic(rows: &[Row]) -> [f64; N] {
    let mut b = [0.0; N];
    for _ in 0..25 {
        let mut h = [[0.0; N]; N];
        let mut g = [0.0; N];
        for (x, y, _, _) in rows {
            let p = sigmoid(b.iter().zip(x).map(|(b, x)| b * x).sum());
            let w = p * (1.0 - p);
            for i in 0..N {
                g[i] += (y - p) * x[i];
                for j in 0..N {
                    h[i][j] += w * x[i] * x[j];
                }
            }
        }
        for i in 0..N {
            h[i][i] += 1e-3;
            g[i] -= 1e-3 * b[i];
        }
        let step = solve(h, g);
        let mut moved = 0.0;
        for i in 0..N {
            b[i] += step[i];
            moved += step[i].abs();
        }
        if moved < 1e-7 {
            break;
        }
    }
    b
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let games: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(600);
    let seed: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
    let mut table = Vec::new();
    for level in [Level::HS, Level::FBS, Level::FCS, Level::SmallCollege, Level::NFL] {
        let teams = synthetic_league(level, 32, seed);
        let rows: Vec<Row> = (0..games)
            .into_par_iter()
            .flat_map(|g| {
                let k = g / 2;
                let h = (k * 7) % 32;
                let a = (k * 13 + 5) % 32;
                let a = if a == h { (a + 1) % 32 } else { a };
                let (h, a) = if g.is_multiple_of(2) { (h, a) } else { (a, h) };
                let s = seed as u64 * 1_000_003 + g as u64;
                let r = quick_game_in(&teams[h], &teams[a], level, s, Default::default(), Default::default());
                r.stats
                    .plays
                    .iter()
                    .filter(|p| !p.nullified && (1..=4).contains(&p.down))
                    .filter(|p| matches!(p.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble))
                    .map(|p| (pass_features_of(level, p), (p.kind != PlayType::Run) as u8 as f64, p.down, p.togo))
                    .collect::<Vec<_>>()
            })
            .collect();
        let b = logistic(&rows);
        let fit = |x: &[f64; N]| sigmoid(b.iter().zip(x).map(|(b, x)| b * x).sum());
        let mean = |v: &[&Row], f: &dyn Fn(&Row) -> f64| v.iter().map(|r| f(r)).sum::<f64>() / v.len().max(1) as f64;
        let all: Vec<&Row> = rows.iter().collect();
        println!(
            "{level:?}: {} snaps, {:.1}% dropbacks (this fit {:.1}%, the code {:.1}%)",
            rows.len(),
            100.0 * mean(&all, &|r| r.1),
            100.0 * mean(&all, &|r| fit(&r.0)),
            100.0 * mean(&all, &|r| pass_chance(level, &r.0)),
        );
        type Band = (&'static str, fn(&Row) -> bool);
        let bands: [Band; 8] = [
            ("1st and 10", |r| r.2 == 1 && (r.3 - 10.0).abs() < 0.5),
            ("2nd and 7+", |r| r.2 == 2 && r.3 >= 7.0),
            ("2nd and 1-3", |r| r.2 == 2 && r.3 < 4.0),
            ("3rd and 7+", |r| r.2 == 3 && r.3 >= 7.0),
            ("3rd and 1-3", |r| r.2 == 3 && r.3 < 4.0),
            ("4th down", |r| r.2 == 4),
            ("ahead 9+", |r| r.0[8] >= 9.0 / 7.0),
            ("behind 9+", |r| r.0[8] <= -9.0 / 7.0),
        ];
        for (name, f) in bands {
            let v: Vec<&Row> = rows.iter().filter(|r| f(r)).collect();
            println!(
                "  {name:<12} {:>6}: {:.1}% dropbacks, fit {:.1}%",
                v.len(),
                100.0 * mean(&v, &|r| r.1),
                100.0 * mean(&v, &|r| fit(&r.0))
            );
        }
        table.push(b);
    }
    let names = ["High school", "FBS", "FCS", "Small colleges", "NFL"];
    println!("\npub const XPASS: [[f64; XPASS_TERMS]; 5] = [");
    for (b, n) in table.iter().zip(names) {
        println!("    // {n}.\n    [{}],", b.iter().map(|x| format!("{x:.4}")).collect::<Vec<_>>().join(", "));
    }
    println!("];");
}
