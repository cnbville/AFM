//! The analysts' models behind the Data section's decisions and game pages
//! (lane NB8), fitted on the match engine's own plays at each level, as
//! nfl4th fits its models on the NFL's: how often a 3rd or 4th down play
//! converts and how far a converted one gets, by yards to go; where a punt
//! leaves the other side, by the punt's spot; field goals made by
//! distance; extra points and two-point tries; and post-game win
//! expectancy, a logistic model of winning on a game's five factors.
//! Prints the tables for `world::tally::decide`, with the level's real
//! rates beside the fit. Rerun after engine changes that move conversions,
//! punts, kicks or scoring:
//! `cargo run --release --bin decide_fit -- 1500 [seed]`

use afm_engine::game::flow::quick_game_in;
use afm_engine::game::stats::{PlayRecord, PlayType};
use afm_engine::game::synthetic::synthetic_league;
use afm_engine::player::Level;
use afm_engine::world::tally::calls::punt_to;
use afm_engine::world::tally::decide::{
    self, FG_BANDS, PGWE_TERMS, PUNT_BANDS, TOGO_BANDS, fg_band, garbage, pgwe_features, punt_band, togo_band,
};
use afm_engine::world::tally::games::adv;
use rayon::prelude::*;

#[derive(Default, Clone)]
struct Tally {
    /// By yards to go, open field and goal to go: tries, converted, yards
    /// gained on conversions.
    conv: [[[f64; 3]; TOGO_BANDS]; 2],
    /// By punt spot: punts with a next snap, the other side's start summed
    /// (yards to the goal it attacks).
    punt: [[f64; 2]; PUNT_BANDS],
    /// By distance: tries, made.
    fg: [[f64; 2]; FG_BANDS],
    xp: [f64; 2],
    two: [f64; 2],
    /// One row per game: the features and the home side's result.
    games: Vec<([f64; PGWE_TERMS], f64)>,
}

impl Tally {
    fn add(&mut self, b: &Tally) {
        for g in 0..2 {
            for t in 0..TOGO_BANDS {
                for k in 0..3 {
                    self.conv[g][t][k] += b.conv[g][t][k];
                }
            }
        }
        for t in 0..PUNT_BANDS {
            self.punt[t][0] += b.punt[t][0];
            self.punt[t][1] += b.punt[t][1];
        }
        for t in 0..FG_BANDS {
            self.fg[t][0] += b.fg[t][0];
            self.fg[t][1] += b.fg[t][1];
        }
        for k in 0..2 {
            self.xp[k] += b.xp[k];
            self.two[k] += b.two[k];
        }
        self.games.extend(b.games.iter().copied());
    }
}

fn scrimmage(p: &PlayRecord) -> bool {
    matches!(p.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble)
}

/// Logistic regression by Newton's method (with a touch of ridge so a
/// thin level still solves).
fn logit<const N: usize>(rows: &[([f64; N], f64)]) -> [f64; N] {
    let mut b = [0.0; N];
    for _ in 0..40 {
        let mut h = [[0.0; N]; N];
        let mut g = [0.0; N];
        for (x, y) in rows {
            let z: f64 = b.iter().zip(x).map(|(b, x)| b * x).sum();
            let p = 1.0 / (1.0 + (-z).exp());
            let w = (p * (1.0 - p)).max(1e-6);
            for i in 0..N {
                g[i] += (y - p) * x[i];
                for j in 0..N {
                    h[i][j] += w * x[i] * x[j];
                }
            }
        }
        for (i, row) in h.iter_mut().enumerate() {
            row[i] += 1e-3;
        }
        let step = solve(h, g);
        let mut big = 0.0f64;
        for i in 0..N {
            b[i] += step[i];
            big = big.max(step[i].abs());
        }
        if big < 1e-9 {
            break;
        }
    }
    b
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

/// A rate with a thin band filled from its neighbour's (or a floor).
fn rates(n: &[[f64; 2]], floor: f64) -> Vec<f64> {
    let mut v: Vec<f64> = n.iter().map(|x| if x[0] >= 30.0 { x[1] / x[0] } else { f64::NAN }).collect();
    for i in 0..v.len() {
        if v[i].is_nan() {
            v[i] = (0..v.len()).filter(|&j| !v[j].is_nan()).min_by_key(|&j| j.abs_diff(i)).map_or(floor, |j| v[j]);
        }
    }
    v
}

fn row_text(v: &[f64], d: usize) -> String {
    v.iter().map(|x| format!("{x:.d$}")).collect::<Vec<_>>().join(", ")
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let games: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1500);
    let seed: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
    let names = ["High school", "FBS", "FCS", "Small colleges", "NFL"];
    let mut out: Vec<Fit> = Vec::new();
    for level in [Level::HS, Level::FBS, Level::FCS, Level::SmallCollege, Level::NFL] {
        let teams = synthetic_league(level, 32, seed);
        let parts: Vec<Tally> = (0..games)
            .into_par_iter()
            .map(|g| {
                let k = g / 2;
                let h = (k * 7) % 32;
                let a = (k * 13 + 5) % 32;
                let a = if a == h { (a + 1) % 32 } else { a };
                let (h, a) = if g.is_multiple_of(2) { (h, a) } else { (a, h) };
                let s = seed as u64 * 1_000_003 + g as u64;
                let r = quick_game_in(&teams[h], &teams[a], level, s, Default::default(), Default::default());
                let mut t = Tally::default();
                let plays = &r.stats.plays;
                for (i, p) in plays.iter().enumerate() {
                    if p.nullified || p.offense > 1 {
                        continue;
                    }
                    if scrimmage(p) && matches!(p.down, 3 | 4) && p.quarter <= 4 && !garbage(level, p) {
                        let goal = p.togo >= p.yardline_100 - 0.01;
                        let made = p.first_down || (p.touchdown && !p.defense_td);
                        let c = &mut t.conv[goal as usize][togo_band(p.togo)];
                        c[0] += 1.0;
                        if made {
                            c[1] += 1.0;
                            c[2] += p.yards.max(p.togo);
                        }
                    }
                    match p.kind {
                        PlayType::Punt => {
                            if let Some(start) = punt_to(plays, i) {
                                let b = &mut t.punt[punt_band(p.yardline_100)];
                                b[0] += 1.0;
                                b[1] += 100.0 - start as f64;
                            }
                        }
                        PlayType::FieldGoal => {
                            let b = &mut t.fg[fg_band(p.yardline_100 + 17.0)];
                            b[0] += 1.0;
                            b[1] += p.kick_good as u8 as f64;
                        }
                        PlayType::ExtraPoint if p.quarter <= 4 => {
                            t.xp[0] += 1.0;
                            t.xp[1] += p.kick_good as u8 as f64;
                        }
                        PlayType::TwoPoint if p.quarter <= 4 => {
                            t.two[0] += 1.0;
                            t.two[1] += (p.touchdown && !p.turnover) as u8 as f64;
                        }
                        _ => {}
                    }
                }
                let a = adv(&r.stats);
                let y = match r.stats.score[0].cmp(&r.stats.score[1]) {
                    std::cmp::Ordering::Greater => 1.0,
                    std::cmp::Ordering::Less => 0.0,
                    _ => 0.5,
                };
                t.games.push((pgwe_features(&a, &r.stats.team), y));
                t
            })
            .collect();
        let mut t = Tally::default();
        for p in &parts {
            t.add(p);
        }
        let conv: Vec<Vec<f64>> =
            (0..2).map(|g| rates(&t.conv[g].iter().map(|c| [c[0], c[1]]).collect::<Vec<_>>(), 0.1)).collect();
        let gain: Vec<Vec<f64>> = (0..2)
            .map(|g| {
                rates(&t.conv[g].iter().map(|c| [c[1], c[2]]).collect::<Vec<_>>(), 10.0)
                    .iter()
                    .enumerate()
                    .map(|(b, x)| x.max([1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 10.0, 15.0, 16.0][b]))
                    .collect()
            })
            .collect();
        let punt = rates(&t.punt, 75.0);
        let fg = rates(&t.fg, 0.5);
        let xp = t.xp[1] / t.xp[0].max(1.0);
        let two = t.two[1] / t.two[0].max(1.0);
        let w = logit(&t.games);
        // How well the fit reads the games: its mean against the home
        // side's share of wins, and how often its favourite won.
        let p: Vec<f64> = t
            .games
            .iter()
            .map(|(x, _)| 1.0 / (1.0 + (-w.iter().zip(x).map(|(b, x)| b * x).sum::<f64>()).exp()))
            .collect();
        let home = t.games.iter().map(|g| g.1).sum::<f64>() / t.games.len().max(1) as f64;
        let right = t.games.iter().zip(&p).filter(|(g, p)| (g.1 - 0.5) * (**p - 0.5) > 0.0).count() as f64
            / t.games.iter().filter(|g| g.1 != 0.5).count().max(1) as f64;
        println!(
            "{level:?}: {} games; 3rd and 4th downs {} open, {} goal to go; {} punts; {} field goals {:.1}%; XP {:.1}% of {}; two {:.1}% of {}",
            t.games.len(),
            t.conv[0].iter().map(|c| c[0]).sum::<f64>(),
            t.conv[1].iter().map(|c| c[0]).sum::<f64>(),
            t.punt.iter().map(|c| c[0]).sum::<f64>(),
            t.fg.iter().map(|c| c[0]).sum::<f64>(),
            100.0 * t.fg.iter().map(|c| c[1]).sum::<f64>() / t.fg.iter().map(|c| c[0]).sum::<f64>().max(1.0),
            100.0 * xp,
            t.xp[0],
            100.0 * two,
            t.two[0],
        );
        println!(
            "  post-game win expectancy: home won {:.1}%, the fit's mean {:.1}%, its favourite won {:.1}%",
            100.0 * home,
            100.0 * p.iter().sum::<f64>() / p.len().max(1) as f64,
            100.0 * right
        );
        println!("  by yards to go (n): {}", row_text(&t.conv[0].iter().map(|c| c[0]).collect::<Vec<_>>(), 0));
        let r = decide::row(level);
        println!("  the code's CONV: {}", row_text(&decide::CONV[r], 3));
        out.push(Fit { conv, gain, punt, fg, xp, two, pgwe: w });
    }
    println!("\n// From `decide_fit -- {games} {seed}`.");
    let block = |name: &str, ty: &str, f: &dyn Fn(usize) -> String| {
        println!("pub const {name}: {ty} = [");
        for (k, n) in names.iter().enumerate() {
            println!("    // {n}.\n    [{}],", f(k));
        }
        println!("];");
    };
    block("CONV", "[[f64; TOGO_BANDS]; 5]", &|k| row_text(&out[k].conv[0], 3));
    block("CONV_GOAL", "[[f64; TOGO_BANDS]; 5]", &|k| row_text(&out[k].conv[1], 3));
    block("GAIN", "[[f64; TOGO_BANDS]; 5]", &|k| row_text(&out[k].gain[0], 1));
    block("PUNT", "[[f64; PUNT_BANDS]; 5]", &|k| row_text(&out[k].punt, 1));
    block("FG", "[[f64; FG_BANDS]; 5]", &|k| row_text(&out[k].fg, 3));
    println!("pub const XP: [f64; 5] = [{}];", row_text(&out.iter().map(|o| o.xp).collect::<Vec<_>>(), 3));
    println!("pub const TWO: [f64; 5] = [{}];", row_text(&out.iter().map(|o| o.two).collect::<Vec<_>>(), 3));
    block("PGWE", "[[f64; PGWE_TERMS]; 5]", &|k| row_text(&out[k].pgwe, 4));
}

/// One level's fitted tables.
struct Fit {
    conv: Vec<Vec<f64>>,
    gain: Vec<Vec<f64>>,
    punt: Vec<f64>,
    fg: Vec<f64>,
    xp: f64,
    two: f64,
    pgwe: [f64; PGWE_TERMS],
}
