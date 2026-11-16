//! Bodies at creation against real (physical study items 9-15): drafted
//! men's height, weight, arms, hands and wingspan by position, the arm's tie
//! to height, the heavy tail and the shortest and tallest receivers, over
//! national classes (`career::simulate_cohort`), and the share of FBS and FCS
//! men listed at 350 lb or more.
//!
//! Real: the nflverse combine 2000-2026 (drafted men's height and weight),
//! MockDraftable 2005-2025 (arms, hands, wingspan; `data/real-world/ab_real.json`),
//! cfbfastR 2025 rosters (0.41% of FBS and FCS listings at 350+).
//!
//! `body_sizes [class size] [classes] [seed]`

use afm_engine::career::{CohortOptions, CollegeLevel, simulate_cohort};
use afm_engine::player::Player;
use afm_engine::positions::Position;

/// Real drafted men: (height m, sd, weight m, sd, weight p99, arm m, sd,
/// hand m, sd, wingspan m).
const REAL: [(Position, [f64; 10]); 11] = [
    (Position::QB, [74.94, 1.72, 222.6, 10.8, 250.0, 31.89, 1.16, 9.58, 0.47, 75.85]),
    (Position::RB, [70.83, 1.79, 218.4, 16.3, 260.0, 30.89, 1.08, 9.27, 0.47, 74.29]),
    (Position::WR, [72.61, 2.26, 200.9, 15.7, 238.0, 31.84, 1.30, 9.38, 0.51, 76.32]),
    (Position::TE, [76.39, 1.36, 253.0, 8.9, 274.0, 32.99, 0.94, 9.83, 0.49, 79.09]),
    (Position::OT, [77.64, 1.18, 316.2, 13.2, 365.0, 34.21, 0.95, 10.07, 0.52, 82.35]),
    (Position::IOL, [75.80, 1.13, 311.3, 11.4, 352.0, 33.14, 1.00, 9.93, 0.51, 79.94]),
    (Position::EDGE, [75.60, 1.42, 263.6, 13.8, 298.0, 33.38, 1.04, 9.83, 0.51, 80.15]),
    (Position::DT, [75.06, 1.44, 307.9, 14.8, 347.0, 33.25, 1.05, 9.95, 0.52, 80.15]),
    (Position::LB, [73.50, 1.37, 237.6, 8.0, 259.0, 32.22, 1.05, 9.60, 0.47, 77.25]),
    (Position::CB, [71.43, 1.57, 193.6, 8.8, 215.0, 31.46, 1.04, 9.13, 0.48, 75.80]),
    (Position::S, [72.32, 1.51, 207.4, 9.3, 230.0, 31.50, 0.99, 9.33, 0.47, 75.77]),
];

fn ms(v: &[f64]) -> (f64, f64) {
    let n = v.len().max(1) as f64;
    let m = v.iter().sum::<f64>() / n;
    (m, (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / n).sqrt())
}

fn q(v: &mut [f64], p: f64) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    v[((v.len() - 1) as f64 * p) as usize]
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let size: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(250_000);
    let classes: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(4);
    let seed = args.get(3).cloned().unwrap_or_else(|| "sizes".into());
    // `AFM_PROG_PARTS` picks the progression parts, as in `calibrate` (15
    // leaves out the weight room and body projects).
    if let Some(parts) = std::env::var("AFM_PROG_PARTS").ok().and_then(|v| v.parse().ok()) {
        afm_engine::progression::set_parts(parts);
    }
    let mut drafted: Vec<Player> = Vec::new();
    let (mut college, mut heavy) = (0usize, 0usize);
    for c in 0..classes {
        let class = simulate_cohort(CohortOptions::new(size, format!("{seed}-{c}").as_str()));
        for r in class.records {
            if matches!(r.college, Some(CollegeLevel::FBS | CollegeLevel::FCS)) {
                college += 1;
                // Listed weights run about 6 lb over measured ones.
                if r.player.body.weight + 6.0 >= 350.0 {
                    heavy += 1;
                }
            }
            if r.draft_pick.is_some() {
                drafted.push(r.at_draft.map_or(r.player, |b| *b));
            }
        }
    }
    println!("body_sizes: {classes} classes of {size}, {} drafted", drafted.len());
    println!(
        "{:<5} {:>4}  {:>15}  {:>17}  {:>9}  {:>15}  {:>5}  {:>15}  {:>13}",
        "pos", "n", "height (real)", "weight (real)", "p99 (rl)", "arm (real)", "slope", "hand (real)", "wing-ht (rl)"
    );
    println!("  (frame: his weight as drawn, before the weight room and any body project)");
    for (pos, rl) in REAL {
        let g: Vec<&Player> = drafted.iter().filter(|p| p.position == pos).collect();
        if g.len() < 10 {
            continue;
        }
        let h: Vec<f64> = g.iter().map(|p| p.body.height).collect();
        let mut w: Vec<f64> = g.iter().map(|p| afm_engine::body::body_now(p).weight).collect();
        let a: Vec<f64> = g.iter().map(|p| p.body.arm).collect();
        let hd: Vec<f64> = g.iter().map(|p| p.body.hand).collect();
        let wg: Vec<f64> = g.iter().map(|p| p.body.wingspan - p.body.height).collect();
        let (hm, hs) = ms(&h);
        let (wm, ws) = ms(&w);
        let (am, asd) = ms(&a);
        let cov = h.iter().zip(&a).map(|(x, y)| (x - hm) * (y - am)).sum::<f64>() / h.len() as f64;
        let slope = cov / (hs * hs).max(1e-9);
        let (dm, dsd) = ms(&hd);
        let (gm, _) = ms(&wg);
        let (fm, _) = ms(&g.iter().map(|p| p.body.frame_weight).collect::<Vec<f64>>());
        let proj = g.iter().filter(|p| p.prog.body_lbs > 0).count() as f64 / g.len() as f64;
        let (lbs, _) = ms(&g.iter().map(|p| p.prog.body_lbs as f64).collect::<Vec<f64>>());
        println!(
            "{:<5} {:>4}  {:>5.2}/{:<4.2} ({:>5.2})  {:>5.1}/{:<4.1} ({:>5.1})  {:>3.0} ({:>3.0})  {:>5.2}/{:<4.2} ({:>5.2})  {:>5.2}  {:>5.2}/{:<4.2} ({:>5.2})  {:>5.2} ({:>5.2})  frame {:>5.1}  project {:>3.0}% {:>4.1} lb",
            format!("{pos:?}"),
            g.len(),
            hm,
            hs,
            rl[0],
            wm,
            ws,
            rl[2],
            q(&mut w, 0.99),
            rl[4],
            am,
            asd,
            rl[5],
            slope,
            dm,
            dsd,
            rl[7],
            gm,
            rl[9] - rl[0],
            fm,
            100.0 * proj,
            lbs
        );
    }
    let wr: Vec<f64> = drafted.iter().filter(|p| p.position == Position::WR).map(|p| p.body.height).collect();
    let n = wr.len().max(1) as f64;
    println!(
        "receivers 5'8 or shorter {:.1}% (real 3.7%), 6'4 or taller {:.1}% (real 10.3%)",
        100.0 * wr.iter().filter(|&&h| h <= 68.0).count() as f64 / n,
        100.0 * wr.iter().filter(|&&h| h >= 76.0).count() as f64 / n
    );
    println!(
        "FBS and FCS men listed 350+: {:.2}% of {college} (real 0.41%)",
        100.0 * heavy as f64 / college.max(1) as f64
    );
}
