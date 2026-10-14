//! Athlete-first check (section 3, topic 5.2): position shares in high school
//! and in the draft against real data, body sizes of drafted players against
//! the combine, and how often players change position.
//! Usage: `positions [size] [seed]`.

use afm_engine::attributes::Attr;
use afm_engine::body::{body_now, combine, combine_size};
use afm_engine::career::{CohortOptions, simulate_cohort};
use afm_engine::positions::{POSITION_COUNT, Position};
use afm_tools::stats::real_draft_shares;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let size: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(250_000);
    let seed = args.get(2).cloned().unwrap_or_else(|| "national-class".to_string());
    let result = simulate_cohort(CohortOptions::new(size, seed.as_str()));
    let recs = &result.records;
    let real = real_draft_shares();
    let mut hs = [0usize; POSITION_COUNT];
    let mut drafted = [0usize; POSITION_COUNT];
    let mut body = [(0.0, 0.0); POSITION_COUNT];
    // Bench check: strength, arm length and combine bench of drafted players.
    let mut lift = [(0.0, 0.0, 0.0); POSITION_COUNT];
    let mut rng = afm_engine::rng::Rng::new(7);
    // Every college kicker and punter, since only a handful get drafted.
    let mut spec = [(0.0f64, 0.0f64, 0.0f64); 2];
    let (mut moved, mut drafted_n, mut drafted_moved) = (0usize, 0usize, 0usize);
    for r in recs {
        let first = r.player.position_history[0].1;
        hs[first.index()] += 1;
        moved += (r.player.position_history.len() > 1) as usize;
        if r.college.is_some() && r.player.position.is_specialist() {
            let b = body_now(&r.player);
            let e = &mut spec[(r.player.position == Position::P) as usize];
            e.0 += b.height;
            e.1 += b.weight;
            e.2 += 1.0;
        }
        if r.draft_pick.is_some() {
            let k = r.player.position.index();
            drafted[k] += 1;
            drafted_n += 1;
            drafted_moved += (r.player.position != first) as usize;
            let b = body_now(&r.player);
            body[k].0 += b.height;
            body[k].1 += b.weight;
            lift[k].0 += r.player.attributes[Attr::Strength];
            lift[k].1 += b.arm;
            lift[k].2 += combine(&r.player, &mut rng).bench;
        }
    }
    let total_share: f64 = Position::ALL.iter().map(|p| p.profile().share).sum();
    println!("pos   HS% (roster)  draft% (real)  drafted height/weight (combine)");
    for pos in Position::ALL {
        let k = pos.index();
        let (hm, _, wm, _) = combine_size(pos);
        let n = drafted[k].max(1) as f64;
        println!(
            "{:<4} {:5.1} ({:4.1})   {:5.1} ({:4.1})   {:4.1}/{:3.0} ({:4.1}/{:3.0})",
            pos.name(),
            100.0 * hs[k] as f64 / recs.len() as f64,
            100.0 * pos.profile().share / total_share,
            100.0 * drafted[k] as f64 / drafted_n.max(1) as f64,
            100.0 * real[k],
            body[k].0 / n,
            body[k].1 / n,
            hm,
            wm
        );
    }
    println!("pos   strength  arm   bench (real)");
    for pos in Position::ALL {
        let k = pos.index();
        let n = drafted[k].max(1) as f64;
        println!(
            "{:<4} {:6.1} {:5.1} {:5.1} ({:4.1})",
            pos.name(),
            lift[k].0 / n,
            lift[k].1 / n,
            lift[k].2 / n,
            afm_engine::body::combine_bench_mean(pos)
        );
    }
    for (k, name) in ["K", "P"].iter().enumerate() {
        let n = spec[k].2.max(1.0);
        println!("college {name}: {:.0} players, {:.1} in / {:.0} lb", spec[k].2, spec[k].0 / n, spec[k].1 / n);
    }
    println!(
        "changed position at least once: {:.1}% of the class; drafted at a different position than as a freshman: {:.1}%",
        100.0 * moved as f64 / recs.len() as f64,
        100.0 * drafted_moved as f64 / drafted_n.max(1) as f64
    );
}
