//! Program fortunes lane P1's quick read (`docs/design/program-fortunes.md`,
//! items 1-4) on one small world with stand-in games: how far each FBS
//! head coach's recruiting moves his classes, how far the name moved over
//! the recorded years, and how often a transfer lands at a school that
//! offered him out of high school. Before and after: the same tool on main
//! and on the lane's branch, the same seed. Not the measuring pass (that
//! is four seeds on full worlds against the real table).
//!
//! `cargo run --release -p afm-tools --bin fortunes_check -- [scale] [history_years] [seed]`

use std::collections::HashMap;

use afm_engine::world::data::Division;
use afm_engine::world::{World, WorldOptions};

fn corr(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len() as f64;
    let (mx, my) = (x.iter().sum::<f64>() / n, y.iter().sum::<f64>() / n);
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for (a, b) in x.iter().zip(y) {
        sxy += (a - mx) * (b - my);
        sxx += (a - mx).powi(2);
        syy += (b - my).powi(2);
    }
    sxy / (sxx * syy).sqrt()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let scale: f64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.1);
    let years: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(8);
    let seed: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(331);
    let t = std::time::Instant::now();
    let w =
        World::new(WorldOptions { seed, scale, history_years: years, engine_from: Some(2100), ..Default::default() });
    println!(
        "world: scale {scale}, {years} years of history, seed {seed}, built in {:.0} s",
        t.elapsed().as_secs_f64()
    );
    let fbs: Vec<usize> = (0..w.programs.len()).filter(|&j| w.programs[j].division == Division::FBS).collect();

    // 1. Classes against the head coach who signed them (the coach of the
    // fall after signing day), raw and within each program.
    let (mut skill, mut score, mut prog) = (Vec::new(), Vec::new(), Vec::new());
    for &j in &fbs {
        let p = &w.programs[j];
        for &(year, s) in &p.class_scores {
            let Some(c) = p.seasons.iter().find(|x| x.year == year).and_then(|x| x.coach) else { continue };
            let a = afm_engine::debug::coach_ability(&w.coaches[c]);
            skill.push(0.6 * a.recruiting + 0.4 * a.relationships);
            score.push(s);
            prog.push(j);
        }
    }
    let r = corr(&skill, &score);
    let demean = |v: &[f64]| -> Vec<f64> {
        let mut m: HashMap<usize, (f64, f64)> = HashMap::new();
        for (k, &j) in prog.iter().enumerate() {
            let e = m.entry(j).or_default();
            e.0 += v[k];
            e.1 += 1.0;
        }
        v.iter().zip(&prog).map(|(x, j)| x - m[j].0 / m[j].1).collect()
    };
    let rw = corr(&demean(&skill), &demean(&score));
    println!("\n## Classes and the head coach's recruiting (FBS, {} classes)", score.len());
    println!("all classes: r {r:.3}, share explained {:.1}%", 100.0 * r * r);
    println!("within each program: r {rw:.3}, share explained {:.1}%", 100.0 * rw * rw);
    let mut by: Vec<(f64, f64)> = skill.iter().copied().zip(score.iter().copied()).collect();
    by.sort_by(|a, b| a.0.total_cmp(&b.0));
    let third = by.len() / 3;
    let mean = |v: &[(f64, f64)]| v.iter().map(|x| x.1).sum::<f64>() / v.len().max(1) as f64;
    println!(
        "class score by the coach's skill, bottom third {:.1}, middle {:.1}, top {:.1}",
        mean(&by[..third]),
        mean(&by[third..2 * third]),
        mean(&by[2 * third..])
    );

    // 2. The name over the recorded years: the tradition grade's first and
    // last recorded values.
    let tg = afm_engine::world::program::GradeCat::Tradition as usize;
    let moves: Vec<f64> = fbs
        .iter()
        .filter_map(|&j| {
            let h = &w.programs[j].grade_history;
            Some(h.last()?.1[tg] - h.first()?.1[tg])
        })
        .collect();
    let sd = (moves.iter().map(|x| x * x).sum::<f64>() / moves.len().max(1) as f64).sqrt();
    let big = moves.iter().filter(|x| x.abs() >= 5.0).count();
    println!("\n## The tradition grade over the recorded years (FBS)");
    println!("change: root mean square {sd:.2} points, {big} of {} programs moved 5 points or more", moves.len());

    // 3. Transfers back to a school that offered them out of high school.
    let (mut landed, mut back) = (0, 0);
    for r in w.recruiting_log.portal.iter().filter(|r| r.from == Division::FBS) {
        let Some(to) = r.to_program.filter(|&t| t != r.from_program) else { continue };
        let Some(i) = w.people.iter().position(|p| p.player.id == r.id) else { continue };
        landed += 1;
        let p = &w.people[i];
        if p.cal.offers.contains(&to) || p.cal.early_offers.iter().any(|o| o.0 == to) {
            back += 1;
        }
    }
    println!("\n## FBS transfers who landed at a school that offered them out of high school");
    println!("{back} of {landed} ({:.1}%)", 100.0 * back as f64 / landed.max(1) as f64);
}
