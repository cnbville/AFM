//! Drafted men's combine drills against real (physical study items 2, 5-8):
//! each position's mean, spread and best 1% at the forty, vertical, broad,
//! bench, 3-cone and shuttle, and the tails a draft: forties of 4.30 and
//! 4.25 or faster, 3-cones of 6.50, shuttles of 3.85, 35 and 40 reps, 42
//! and 44 inch verticals and 140 inch broads. One test day each at the
//! draft (`body::combine`), every drafted man testing every drill.
//!
//! Real: the nflverse combine, drafted men 2000-2025 (26 drafts), only the
//! men who ran each drill (about 60-85% of picks), so real's counts a
//! draft sit a little under what every man testing would give.
//!
//! `drill_tails [class size] [classes] [seed]`

use afm_engine::body::combine;
use afm_engine::career::{CohortOptions, simulate_cohort};
use afm_engine::player::Player;
use afm_engine::positions::Position;
use afm_engine::rng::Rng;

/// Real drafted men by position: (mean, sd, best 1%) for the forty,
/// vertical, broad, bench, 3-cone and shuttle.
const REAL: [(Position, [[f64; 3]; 6]); 11] = [
    (
        Position::QB,
        [
            [4.80, 0.18, 4.41],
            [31.66, 3.25, 38.5],
            [111.8, 6.81, 127.0],
            [22.1, 1.83, 24.9],
            [7.12, 0.22, 6.73],
            [4.30, 0.16, 3.95],
        ],
    ),
    (
        Position::RB,
        [
            [4.55, 0.13, 4.33],
            [34.62, 3.21, 41.5],
            [119.1, 5.97, 132.0],
            [20.4, 4.84, 31.5],
            [7.08, 0.24, 6.66],
            [4.27, 0.16, 3.93],
        ],
    ),
    (
        Position::WR,
        [
            [4.47, 0.09, 4.28],
            [35.99, 3.00, 42.6],
            [122.4, 5.73, 136.0],
            [15.2, 4.10, 26.5],
            [6.94, 0.20, 6.56],
            [4.21, 0.15, 3.89],
        ],
    ),
    (
        Position::TE,
        [
            [4.73, 0.13, 4.42],
            [33.76, 3.02, 41.8],
            [116.8, 5.76, 131.7],
            [20.8, 4.22, 32.7],
            [7.15, 0.22, 6.76],
            [4.34, 0.15, 4.01],
        ],
    ),
    (
        Position::OT,
        [
            [5.20, 0.17, 4.81],
            [28.72, 3.07, 35.6],
            [104.4, 6.46, 118.0],
            [24.9, 4.76, 37.0],
            [7.80, 0.28, 7.25],
            [4.73, 0.18, 4.37],
        ],
    ),
    (
        Position::IOL,
        [
            [5.23, 0.16, 4.89],
            [28.45, 3.08, 35.5],
            [103.3, 6.38, 118.5],
            [26.7, 5.35, 41.7],
            [7.76, 0.28, 7.27],
            [4.69, 0.19, 4.25],
        ],
    ),
    (
        Position::EDGE,
        [
            [4.76, 0.15, 4.42],
            [33.92, 3.12, 41.5],
            [117.3, 6.36, 132.0],
            [24.1, 4.66, 36.0],
            [7.24, 0.25, 6.73],
            [4.38, 0.16, 4.03],
        ],
    ),
    (
        Position::DT,
        [
            [5.07, 0.16, 4.75],
            [29.42, 2.91, 35.8],
            [105.9, 5.94, 119.0],
            [27.9, 5.46, 44.0],
            [7.68, 0.27, 7.09],
            [4.64, 0.18, 4.22],
        ],
    ),
    (
        Position::LB,
        [
            [4.66, 0.12, 4.42],
            [34.63, 3.13, 42.0],
            [118.4, 6.32, 132.9],
            [22.6, 4.42, 34.0],
            [7.11, 0.21, 6.68],
            [4.26, 0.14, 3.92],
        ],
    ),
    (
        Position::CB,
        [
            [4.47, 0.08, 4.28],
            [36.36, 2.87, 43.0],
            [123.3, 5.68, 136.0],
            [15.0, 4.04, 25.4],
            [6.93, 0.21, 6.49],
            [4.16, 0.15, 3.84],
        ],
    ),
    (
        Position::S,
        [
            [4.53, 0.09, 4.34],
            [36.19, 2.95, 43.5],
            [121.9, 6.06, 137.7],
            [17.2, 4.23, 28.0],
            [6.97, 0.21, 6.55],
            [4.18, 0.14, 3.85],
        ],
    ),
];

/// The tails a draft: (drill index, at or past, lower is better, real a draft).
const TAILS: [(usize, f64, bool, f64); 9] = [
    (0, 4.30, true, 1.27),
    (0, 4.25, true, 0.19),
    (4, 6.50, true, 0.35),
    (5, 3.85, true, 0.58),
    (3, 35.0, false, 3.54),
    (3, 40.0, false, 0.77),
    (1, 42.0, false, 2.31),
    (1, 44.0, false, 0.35),
    (2, 140.0, false, 0.15),
];

const NAMES: [&str; 6] = ["forty", "vertical", "broad", "bench", "3-cone", "shuttle"];

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
    let seed = args.get(3).cloned().unwrap_or_else(|| "drills".into());
    let mut drafted: Vec<Player> = Vec::new();
    for c in 0..classes {
        let class = simulate_cohort(CohortOptions::new(size, format!("{seed}-{c}").as_str()));
        drafted.extend(
            class.records.into_iter().filter(|r| r.draft_pick.is_some()).map(|r| r.at_draft.map_or(r.player, |b| *b)),
        );
    }
    let mut rng = Rng::new(0xD811_7A15);
    let tests: Vec<(Position, [f64; 6])> = drafted
        .iter()
        .map(|p| {
            let c = combine(p, &mut rng);
            (p.position, [c.forty, c.vertical, c.broad, c.bench, c.cone, c.shuttle])
        })
        .collect();
    println!("drill_tails: {classes} classes of {size}, {} drafted; mean/sd/best 1% (real)", drafted.len());
    for (pos, rl) in REAL {
        let g: Vec<&[f64; 6]> = tests.iter().filter(|t| t.0 == pos).map(|t| &t.1).collect();
        if g.len() < 10 {
            continue;
        }
        let mut line = format!("{:<5} {:>4}", format!("{pos:?}"), g.len());
        for k in 0..6 {
            let mut v: Vec<f64> = g.iter().map(|t| t[k]).collect();
            let (m, s) = ms(&v);
            let low = matches!(k, 0 | 4 | 5);
            let best = q(&mut v, if low { 0.01 } else { 0.99 });
            let d = if k == 1 || k == 2 || k == 3 { 1 } else { 2 };
            line += &format!(
                "  {} {:.d$}/{:.d$}/{:.d$} ({:.d$}/{:.d$}/{:.d$})",
                NAMES[k], m, s, best, rl[k][0], rl[k][1], rl[k][2]
            );
        }
        println!("{line}");
    }
    let drafts = classes.max(1) as f64;
    for (k, at, low, real) in TAILS {
        let n = tests.iter().filter(|t| if low { t.1[k] <= at } else { t.1[k] >= at }).count() as f64;
        println!("{} {} {at}: {:.2} a draft (real {real})", NAMES[k], if low { "<=" } else { ">=" }, n / drafts);
    }
    for (k, name) in NAMES.iter().enumerate() {
        let mut v: Vec<f64> = tests.iter().map(|t| t.1[k]).collect();
        let low = matches!(k, 0 | 4 | 5);
        v.sort_by(|a, b| a.total_cmp(b));
        let best = if low { v[0] } else { v[v.len() - 1] };
        let same = v.iter().filter(|&&x| x == best).count();
        println!("{name} best {best} ({same} men at it)");
    }
}
