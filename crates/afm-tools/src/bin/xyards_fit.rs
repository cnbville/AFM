//! The expected rush and expected yards-after-catch models behind rush
//! yards over expected and YAC over expected (Data overhaul items 5 and 6),
//! fitted on the match engine's own plays at each level, as Next Gen Stats
//! fits its models on the NFL's. Least squares on designed runs and on
//! catches; prints the weights for `world::tally::expect::XRUSH` and
//! `XYAC`, with each band's real average against the fit. Rerun after
//! engine changes that move runs or the run after the catch:
//! `cargo run --release --bin xyards_fit -- 600 [seed]`

use afm_engine::game::flow::quick_game_in;
use afm_engine::game::stats::PlayType;
use afm_engine::game::synthetic::synthetic_league;
use afm_engine::player::Level;
use afm_engine::world::tally::expect::{
    XRUSH_TERMS, XYAC_TERMS, depth_row, expected_rush, expected_yac, run_gap, third, xrush_features, xyac_features,
};
use rayon::prelude::*;

/// A designed run: box, outside, yards to the goal, to go, yards gained.
type Run = (u8, bool, f64, f64, f64);
/// A catch: air yards, separation, man coverage, in the middle third, YAC.
type Catch = (f64, f64, bool, bool, f64);

/// Least squares: solves (X'X) b = X'y.
fn ols<const N: usize>(rows: &[([f64; N], f64)]) -> [f64; N] {
    let mut h = [[0.0; N]; N];
    let mut g = [0.0; N];
    for (x, y) in rows {
        for i in 0..N {
            g[i] += x[i] * y;
            for j in 0..N {
                h[i][j] += x[i] * x[j];
            }
        }
    }
    solve(h, g)
}

#[allow(clippy::needless_range_loop)]
fn solve<const N: usize>(mut h: [[f64; N]; N], mut g: [f64; N]) -> [f64; N] {
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

fn dot<const N: usize>(b: &[f64; N], x: &[f64; N]) -> f64 {
    b.iter().zip(x).map(|(b, x)| b * x).sum()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let games: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(600);
    let seed: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
    let mut rush_table = Vec::new();
    let mut yac_table = Vec::new();
    for level in [Level::HS, Level::FBS, Level::FCS, Level::SmallCollege, Level::NFL] {
        let teams = synthetic_league(level, 32, seed);
        let plays: Vec<(Vec<Run>, Vec<Catch>)> = (0..games)
            .into_par_iter()
            .map(|g| {
                let k = g / 2;
                let h = (k * 7) % 32;
                let a = (k * 13 + 5) % 32;
                let a = if a == h { (a + 1) % 32 } else { a };
                let (h, a) = if g.is_multiple_of(2) { (h, a) } else { (a, h) };
                let s = seed as u64 * 1_000_003 + g as u64;
                let r = quick_game_in(&teams[h], &teams[a], level, s, Default::default(), Default::default());
                let mut runs = Vec::new();
                let mut catches = Vec::new();
                for p in r.stats.plays.iter().filter(|p| !p.nullified) {
                    if p.kind == PlayType::Run && p.carrier.is_some() {
                        let gap = run_gap(&p.call);
                        runs.push((p.box_count, matches!(gap, 0 | 4), p.yardline_100, p.togo, p.yards));
                    }
                    if p.kind == PlayType::Pass && p.complete && !p.throw_away && p.target.is_some() {
                        let middle = p.throw_y.is_some_and(|(_, at)| third(at as f64) == 1);
                        catches.push((p.air_yards, p.separation, p.coverage_man, middle, p.yac));
                    }
                }
                (runs, catches)
            })
            .collect();
        let runs: Vec<Run> = plays.iter().flat_map(|x| x.0.iter().copied()).filter(|r| r.4.is_finite()).collect();
        let catches: Vec<Catch> =
            plays.iter().flat_map(|x| x.1.iter().copied()).filter(|c| c.0.is_finite() && c.1.is_finite()).collect();
        let br: [f64; XRUSH_TERMS] =
            ols(&runs.iter().map(|r| (xrush_features(r.0, r.1, r.2, r.3), r.4)).collect::<Vec<_>>());
        let by: [f64; XYAC_TERMS] =
            ols(&catches.iter().map(|c| (xyac_features(c.0, c.1, c.2, c.3), c.4)).collect::<Vec<_>>());
        let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len().max(1) as f64;
        println!(
            "{level:?}: {} designed runs, {:.2} yards a carry (this fit {:.2}, the code {:.2}); {} catches, {:.2} YAC (fit {:.2}, code {:.2})",
            runs.len(),
            mean(&runs.iter().map(|r| r.4).collect::<Vec<_>>()),
            mean(
                &runs
                    .iter()
                    .map(|r| dot(&br, &xrush_features(r.0, r.1, r.2, r.3)).clamp(-3.0, r.2))
                    .collect::<Vec<_>>()
            ),
            mean(&runs.iter().map(|r| expected_rush(level, r.0, r.1, r.2, r.3)).collect::<Vec<_>>()),
            catches.len(),
            mean(&catches.iter().map(|c| c.4).collect::<Vec<_>>()),
            mean(&catches.iter().map(|c| dot(&by, &xyac_features(c.0, c.1, c.2, c.3)).max(0.0)).collect::<Vec<_>>()),
            mean(&catches.iter().map(|c| expected_yac(level, c.0, c.1, c.2, c.3)).collect::<Vec<_>>()),
        );
        type RunBand = (&'static str, fn(&Run) -> bool);
        let rb: [RunBand; 8] = [
            ("box 5-", |r| r.0 <= 5),
            ("box 6", |r| r.0 == 6),
            ("box 7", |r| r.0 == 7),
            ("box 8+", |r| r.0 >= 8),
            ("inside", |r| !r.1),
            ("outside", |r| r.1),
            ("goal 10-", |r| r.2 <= 10.0),
            ("to go 2-", |r| r.3 <= 2.0),
        ];
        for (name, f) in rb {
            let v: Vec<&Run> = runs.iter().filter(|r| f(r)).collect();
            let n = v.len().max(1) as f64;
            println!(
                "  run {name:<9} {:>6}: {:.2} a carry, fit {:.2}",
                v.len(),
                v.iter().map(|r| r.4).sum::<f64>() / n,
                v.iter().map(|r| dot(&br, &xrush_features(r.0, r.1, r.2, r.3)).clamp(-3.0, r.2)).sum::<f64>() / n
            );
        }
        for row in 0..4 {
            let v: Vec<&Catch> = catches.iter().filter(|c| depth_row(c.0) == row).collect();
            let n = v.len().max(1) as f64;
            println!(
                "  catch depth {row} {:>6}: {:.2} YAC, fit {:.2}",
                v.len(),
                v.iter().map(|c| c.4).sum::<f64>() / n,
                v.iter().map(|c| dot(&by, &xyac_features(c.0, c.1, c.2, c.3)).max(0.0)).sum::<f64>() / n
            );
        }
        rush_table.push(br);
        yac_table.push(by);
    }
    let names = ["High school", "FBS", "FCS", "Small colleges", "NFL"];
    println!("\npub const XRUSH: [[f64; XRUSH_TERMS]; 5] = [");
    for (b, n) in rush_table.iter().zip(names) {
        println!("    // {n}.\n    [{}],", b.iter().map(|x| format!("{x:.4}")).collect::<Vec<_>>().join(", "));
    }
    println!("];\n\npub const XYAC: [[f64; XYAC_TERMS]; 5] = [");
    for (b, n) in yac_table.iter().zip(names) {
        println!("    // {n}.\n    [{}],", b.iter().map(|x| format!("{x:.4}")).collect::<Vec<_>>().join(", "));
    }
    println!("];");
}
