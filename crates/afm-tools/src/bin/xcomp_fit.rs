//! The expected-completion model behind CPOE, fitted on the match engine's
//! own throws at each level. Real CPOE models are fitted on their own
//! league's throws (nflfastR's on the NFL's), so a league's CPOE centres at
//! zero; this does the same for each of the engine's levels and prints the
//! weights for `game::stats::XCOMP` (minor bug 178), with completion against
//! the model by air-yards and separation band. Rerun after engine changes
//! that move completions:
//! `cargo run --release --bin xcomp_fit -- 600 [seed]`

use afm_engine::game::flow::quick_game_in;
use afm_engine::game::stats::{PlayType, XCOMP_TERMS, expected_completion_at, xcomp_features};
use afm_engine::game::synthetic::synthetic_league;
use afm_engine::player::Level;
use rayon::prelude::*;

const N: usize = XCOMP_TERMS;

/// One throw: air yards, separation, pressured, complete.
type Throw = (f64, f64, bool, bool);

/// A band of throws to check the fit on: its name and who is in it.
type Band = (&'static str, fn(&Throw) -> bool);

fn sigmoid(z: f64) -> f64 {
    1.0 / (1.0 + (-z).exp())
}

fn z(b: &[f64; N], t: &Throw) -> f64 {
    b.iter().zip(xcomp_features(t.0, t.1, t.2)).map(|(b, x)| b * x).sum()
}

/// Logistic regression by Newton steps.
fn fit(throws: &[Throw]) -> [f64; N] {
    let mut b = [0.0; N];
    for _ in 0..50 {
        let mut g = [0.0; N];
        let mut h = [[0.0; N]; N];
        for t in throws {
            let x = xcomp_features(t.0, t.1, t.2);
            let p = sigmoid(z(&b, t));
            let y = t.3 as u8 as f64;
            for i in 0..N {
                g[i] += (y - p) * x[i];
                for j in 0..N {
                    h[i][j] += p * (1.0 - p) * x[i] * x[j];
                }
            }
        }
        let step = solve(h, g);
        for i in 0..N {
            b[i] += step[i];
        }
        if step.iter().map(|s| s.abs()).sum::<f64>() < 1e-10 {
            break;
        }
    }
    b
}

/// Solves h x = g (Gaussian elimination with partial pivots).
#[allow(clippy::needless_range_loop)]
fn solve(mut h: [[f64; N]; N], mut g: [f64; N]) -> [f64; N] {
    for c in 0..N {
        let p = (c..N).max_by(|&a, &b| h[a][c].abs().total_cmp(&h[b][c].abs())).unwrap();
        h.swap(c, p);
        g.swap(c, p);
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
        x[c] = (g[c] - (c + 1..N).map(|k| h[c][k] * x[k]).sum::<f64>()) / h[c][c];
    }
    x
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let games: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(600);
    let seed: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
    let mut table = Vec::new();
    for level in [Level::HS, Level::FBS, Level::FCS, Level::SmallCollege, Level::NFL] {
        let teams = synthetic_league(level, 32, seed);
        let throws: Vec<Throw> = (0..games)
            .into_par_iter()
            .flat_map_iter(|g| {
                let k = g / 2;
                let h = (k * 7) % 32;
                let a = (k * 13 + 5) % 32;
                let a = if a == h { (a + 1) % 32 } else { a };
                let (h, a) = if g.is_multiple_of(2) { (h, a) } else { (a, h) };
                let s = seed as u64 * 1_000_003 + g as u64;
                let r = quick_game_in(&teams[h], &teams[a], level, s, Default::default(), Default::default());
                r.stats
                    .plays
                    .into_iter()
                    .filter(|p| p.kind == PlayType::Pass && !p.nullified && !p.throw_away)
                    .map(|p| (p.air_yards, p.separation, p.pressured, p.complete))
                    .collect::<Vec<_>>()
            })
            .filter(|t| t.0.is_finite() && t.1.is_finite())
            .collect();
        let n = throws.len() as f64;
        let cmp = throws.iter().filter(|t| t.3).count() as f64;
        let b = fit(&throws);
        let mean = |f: &dyn Fn(&Throw) -> f64| throws.iter().map(f).sum::<f64>() / n;
        let code = mean(&|t: &Throw| expected_completion_at(level, t.0, t.1, t.2));
        println!(
            "{level:?}: {} throws, completion {:.1}%, the code's model {:.1}% (CPOE {:+.1})",
            throws.len(),
            100.0 * cmp / n,
            100.0 * code,
            100.0 * (cmp / n - code)
        );
        let bands: [Band; 11] = [
            ("air < 0", |t| t.0 < 0.0),
            ("air 0-9", |t| (0.0..10.0).contains(&t.0)),
            ("air 10-19", |t| (10.0..20.0).contains(&t.0)),
            ("air 20-29", |t| (20.0..30.0).contains(&t.0)),
            ("air 30-39", |t| (30.0..40.0).contains(&t.0)),
            ("air 40+", |t| t.0 >= 40.0),
            ("sep < 0.5", |t| t.1 < 0.5),
            ("sep 0.5-1", |t| (0.5..1.0).contains(&t.1)),
            ("sep 1-1.5", |t| (1.0..1.5).contains(&t.1)),
            ("sep 1.5+", |t| t.1 >= 1.5),
            ("pressured", |t| t.2),
        ];
        for (name, f) in bands {
            let ts: Vec<&Throw> = throws.iter().filter(|t| f(t)).collect();
            let m = ts.len().max(1) as f64;
            println!(
                "  {name:<10} {:>6} throws: completed {:.1}%, this fit {:.1}%, the code {:.1}%",
                ts.len(),
                100.0 * ts.iter().filter(|t| t.3).count() as f64 / m,
                100.0 * ts.iter().map(|t| sigmoid(z(&b, t))).sum::<f64>() / m,
                100.0 * ts.iter().map(|t| expected_completion_at(level, t.0, t.1, t.2)).sum::<f64>() / m
            );
        }
        table.push(b);
    }
    println!("\npub const XCOMP: [[f64; XCOMP_TERMS]; 5] = [");
    for b in table {
        println!("    [{}],", b.iter().map(|x| format!("{x:.4}")).collect::<Vec<_>>().join(", "));
    }
    println!("];");
}
