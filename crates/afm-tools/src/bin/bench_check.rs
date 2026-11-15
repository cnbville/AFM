//! The bench and the weight room against real (physical study items 4 and
//! 5, minor bug 385): generator boys grown from 14 to 22 (high school, then
//! four college years under a staff of 12 as starters), with each
//! position's test-day lifts at 18 and 22 and the combine bench at 22, all
//! on the top 3% by potential (about Division I's share).
//!
//! Real: Jacobson et al. 2013 (Division I over four years): bench max +34%
//! for receivers and backs, +18% for linemen. The combine (nflverse
//! 2000-2025): bench sd 4.0 to 5.5 reps at a position, 35+ reps about 3.4
//! drafted men a year.
//!
//! Then one national class (`career::simulate_cohort`): the drafted men's
//! strength, upper and lower, and combine bench by position, and the
//! class's 35+ and 40+ benchers against a real draft.
//!
//! `bench_check [boys] [seed] [class size] [classes]`

use afm_engine::attributes::Attr;
use afm_engine::career::{CohortOptions, simulate_cohort};
use afm_engine::development::{SeasonContext, develop_season};
use afm_engine::player::{Level, PlayingTime, ProspectOptions, generate_prospect};
use afm_engine::positions::Position;
use afm_engine::rng::Rng;
use rayon::prelude::*;

struct Row {
    pos: Position,
    lb18: f64,
    lb22: f64,
    squat18: f64,
    squat22: f64,
    wt18: f64,
    wt22: f64,
    st18: f64,
    st22: f64,
    combine: f64,
    potential: f64,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(40_000);
    let seed: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(385);
    let rows: Vec<Row> = (0..n)
        .into_par_iter()
        .map(|i| {
            let id = 0x0B00_0000 | i;
            let mut rng = Rng::new(seed ^ i.wrapping_mul(0x9E37_79B1));
            let mut p = generate_prospect(&mut rng, ProspectOptions { id: Some(id), ..Default::default() });
            let mut at18 = None;
            while p.age < 22 {
                let ctx = if p.age < 18 {
                    SeasonContext::new(Level::HS, 9.0, PlayingTime::Rotation)
                } else {
                    SeasonContext::new(Level::FBS, 12.0, PlayingTime::Starter)
                };
                develop_season(&mut p, &ctx, &mut rng);
                if p.age == 18 {
                    at18 = Some((afm_engine::training::seat::test_row(&p, 18), p.attributes.get(Attr::Strength)));
                }
            }
            let (t18, st18) = at18.unwrap();
            let t22 = afm_engine::training::seat::test_row(&p, 22);
            let combine = afm_engine::body::combine(&p, &mut Rng::new(id ^ 0xC0B)).bench;
            Row {
                pos: p.position,
                lb18: t18.bench as f64,
                lb22: t22.bench as f64,
                squat18: t18.squat as f64,
                squat22: t22.squat as f64,
                wt18: t18.weight as f64,
                wt22: t22.weight as f64,
                st18,
                st22: p.attributes.get(Attr::Strength),
                combine,
                potential: afm_engine::debug::potential(&p),
            }
        })
        .collect();
    let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len().max(1) as f64;
    let sd = |v: &[f64]| {
        let m = mean(v);
        (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / v.len().max(1) as f64).sqrt()
    };
    println!("bench_check: {n} boys, seed {seed}");
    println!(
        "{:<5} {:>5}  {:>14} {:>6}  {:>14} {:>6}  {:>11}  {:>11}  {:>22}",
        "pos", "n", "bench 18>22", "gain", "squat 18>22", "gain", "wt 18>22", "str 18>22", "combine sd/p99"
    );
    let mut forty_plus = 0usize;
    let mut thirty_five = 0usize;
    let mut top_n = 0usize;
    for pos in Position::ALL {
        if pos.is_specialist() {
            continue;
        }
        let all: Vec<&Row> = rows.iter().filter(|r| r.pos == pos).collect();
        if all.len() < 50 {
            continue;
        }
        // The lifts read on the top 3% by potential: about the share of
        // high school players who reach a Division I program, as Jacobson's
        // players did.
        let mut ps: Vec<f64> = all.iter().map(|r| r.potential).collect();
        ps.sort_by(|a, b| a.total_cmp(b));
        let q = ps[ps.len() * 97 / 100];
        let g: Vec<&Row> = all.iter().copied().filter(|r| r.potential >= q).collect();
        let f = |k: fn(&Row) -> f64| g.iter().map(|r| k(r)).collect::<Vec<f64>>();
        let (b18, b22, s18, s22) =
            (mean(&f(|r| r.lb18)), mean(&f(|r| r.lb22)), mean(&f(|r| r.squat18)), mean(&f(|r| r.squat22)));
        let mut top: Vec<f64> = g.iter().map(|r| r.combine).collect();
        top.sort_by(|a, b| a.total_cmp(b));
        let p99 = top[((top.len() as f64 * 0.99) as usize).min(top.len() - 1)];
        // About 1 in 100 of a class is drafted: the top 4% of the quarter.
        let mut best: Vec<&Row> = all.to_vec();
        best.sort_by(|a, b| b.potential.total_cmp(&a.potential));
        let drafted = (best.len() / 100).max(1);
        top_n += drafted;
        thirty_five += best[..drafted].iter().filter(|r| r.combine >= 35.0).count();
        forty_plus += best[..drafted].iter().filter(|r| r.combine >= 40.0).count();
        println!(
            "{:<5} {:>5}  {:>6.0} > {:<6.0} {:>+5.0}%  {:>6.0} > {:<6.0} {:>+5.0}%  {:>4.0} > {:<4.0}  {:>4.1} > {:<4.1}  {:>10.1} / {:<8.0}",
            format!("{pos:?}"),
            g.len(),
            b18,
            b22,
            100.0 * (b22 / b18 - 1.0),
            s18,
            s22,
            100.0 * (s22 / s18 - 1.0),
            mean(&f(|r| r.wt18)),
            mean(&f(|r| r.wt22)),
            mean(&f(|r| r.st18)),
            mean(&f(|r| r.st22)),
            sd(&top),
            p99
        );
    }
    println!(
        "the top 1% by potential ({top_n} men, about a draft's share): 35+ reps {thirty_five}, 40+ {forty_plus} (real a draft: 3.4 and 0.7 of 257)"
    );
    let size: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(250_000);
    let classes: u32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(4);
    let drafted: Vec<afm_engine::player::Player> = (0..classes)
        .flat_map(|c| {
            let class = simulate_cohort(CohortOptions::new(size, format!("bench{seed}-{c}").as_str()));
            class
                .records
                .into_iter()
                .filter(|r| r.draft_pick.is_some())
                .map(|r| r.at_draft.map_or(r.player, |b| *b))
                .collect::<Vec<_>>()
        })
        .collect();
    println!("\n{classes} classes of {size}: {} drafted", drafted.len());
    println!(
        "{:<5} {:>4}  {:>11}  {:>11}  {:>16}  {:>5}",
        "pos", "n", "upper m/sd", "lower m/sd", "bench m/sd (real)", "p99"
    );
    let (mut c35, mut c40) = (0usize, 0usize);
    for pos in Position::ALL {
        if pos.is_specialist() {
            continue;
        }
        let g: Vec<&afm_engine::player::Player> = drafted.iter().filter(|p| p.position == pos).collect();
        if g.is_empty() {
            continue;
        }
        let up: Vec<f64> = g.iter().map(|p| p.attributes.get(Attr::Strength)).collect();
        let lo: Vec<f64> = g.iter().map(|p| p.attributes.get(Attr::LowerBody)).collect();
        let mut b: Vec<f64> =
            g.iter().map(|p| afm_engine::body::combine(p, &mut Rng::new(p.id ^ 0xC0B)).bench).collect();
        c35 += b.iter().filter(|&&x| x >= 35.0).count();
        c40 += b.iter().filter(|&&x| x >= 40.0).count();
        b.sort_by(|a, b| a.total_cmp(b));
        let p99 = b[((b.len() as f64 * 0.99) as usize).min(b.len() - 1)];
        println!(
            "{:<5} {:>4}  {:>5.1}/{:<4.1}  {:>5.1}/{:<4.1}  {:>5.1}/{:<3.1} ({:>4.1})  {:>5.0}",
            format!("{pos:?}"),
            g.len(),
            mean(&up),
            sd(&up),
            mean(&lo),
            sd(&lo),
            mean(&b),
            sd(&b),
            afm_engine::body::combine_bench_mean(pos),
            p99
        );
    }
    let per = 257.0 / drafted.len().max(1) as f64;
    println!(
        "35+ reps {c35} ({:.1} a 257-man draft), 40+ {c40} ({:.1}); real 3.4 and 0.7",
        c35 as f64 * per,
        c40 as f64 * per
    );
}
