//! What a plan's focus does to growth (individual practice, topic 5b): the
//! same players on the same dice, grown with no focus, with a focus at the
//! usual 30 minutes a week and at 60, by stage of career. Prints the focused
//! rating's gain, its key skills' gains (the cost), the overall, the sum of
//! every skill (steered, not added), the room left to the ceiling, and how
//! many ever passed it.
//!
//! Usage: focus_check [players] [seed] [position] [skill]

use afm_engine::attributes::Attr;
use afm_engine::debug;
use afm_engine::development::{SeasonContext, develop_season, focus_cost_skills};
use afm_engine::player::{ProspectOptions, generate_prospect};
use afm_engine::{Level, Player, PlayingTime, Position, Rng};

fn grow(p: &mut Player, coaching: f64, focus: Option<(Attr, f64)>, rng: &mut Rng) {
    let mut ctx = SeasonContext::new(level_at(p.age), coaching, PlayingTime::Starter);
    if let Some((a, m)) = focus {
        ctx.focus = Some(a);
        ctx.focus_minutes = m;
    }
    develop_season(p, &ctx, rng);
}

fn level_at(age: u32) -> Level {
    if age < 18 {
        Level::HS
    } else if age < 22 {
        Level::FBS
    } else {
        Level::NFL
    }
}

#[derive(Clone)]
struct Arm {
    target: Vec<f64>,
    cost: Vec<f64>,
    overall: Vec<f64>,
    all: Vec<f64>,
    gap_left: Vec<f64>,
    over: usize,
}

fn total(p: &Player) -> f64 {
    Attr::ALL.iter().map(|a| p.attributes.get(*a)).sum()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(2000);
    let seed: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(97);
    let pos = args
        .get(3)
        .and_then(|s| Position::ALL.iter().copied().find(|p| p.name().eq_ignore_ascii_case(s)))
        .unwrap_or(Position::CB);
    let target = args
        .get(4)
        .and_then(|s| Attr::ALL.iter().copied().find(|a| format!("{a:?}").eq_ignore_ascii_case(s)))
        .unwrap_or(Attr::ManCoverage);
    let cost = focus_cost_skills(pos, target);
    println!(
        "{} focusing on {:?}; its cost lands on {}",
        pos.name(),
        target,
        cost.iter().map(|(a, w)| format!("{a:?} ({w:.3})")).collect::<Vec<_>>().join(", ")
    );
    let arms_m = [None, Some(30.0), Some(60.0)];
    // (label, age the focus starts, years with it, coaching 1-20)
    let stages = [("college, 19", 19u32, 3u32, 12.0), ("pro, 26", 26, 3, 12.0), ("pro, 30", 30, 2, 12.0)];
    for (label, start, years, coaching) in stages {
        let y = years as usize;
        let blank = Arm {
            target: vec![0.0; y],
            cost: vec![0.0; y],
            overall: vec![0.0; y],
            all: vec![0.0; y],
            gap_left: vec![0.0; y],
            over: 0,
        };
        let mut arms = vec![blank; arms_m.len()];
        let (mut gap0, mut rating0, mut took) = (0.0, 0.0, 0usize);
        // Year one's extra at 60 minutes for the coachable (take 0.96+).
        let (mut co_n, mut co_extra) = (0usize, 0.0);
        let mut rng = Rng::new(seed);
        for i in 0..n {
            let z = 1.6 + 0.5 * rng.normal(0.0, 1.0);
            let mut p = generate_prospect(
                &mut rng,
                ProspectOptions { id: Some(900_000 + i as u32), position: Some(pos), age: Some(14), talent_z: Some(z) },
            );
            let mut r = Rng::new(seed.wrapping_mul(31).wrapping_add(i as u32));
            while p.age < start {
                grow(&mut p, coaching, None, &mut r);
            }
            let base = p.clone();
            let gap = debug::ceilings(&base).get(target) - base.attributes.get(target);
            // The young need room to grow; a veteran's focus is about decline.
            if start < 30 && gap < 1.0 {
                continue;
            }
            took += 1;
            gap0 += gap;
            rating0 += base.attributes.get(target);
            let coachable = base.personality.coachability >= 18.0;
            let mut first = [0.0; 3];
            for (k, m) in arms_m.iter().enumerate() {
                let mut q = base.clone();
                let mut rr = r.clone();
                for yy in 0..y {
                    grow(&mut q, coaching, m.map(|m| (target, m)), &mut rr);
                    if yy == 0 {
                        first[k] = q.attributes.get(target) - base.attributes.get(target);
                    }
                    let a = &mut arms[k];
                    a.target[yy] += q.attributes.get(target) - base.attributes.get(target);
                    a.cost[yy] += cost.iter().map(|(s, _)| q.attributes.get(*s) - base.attributes.get(*s)).sum::<f64>()
                        / cost.len().max(1) as f64;
                    a.overall[yy] += q.overall() - base.overall();
                    a.all[yy] += total(&q) - total(&base);
                    let left = debug::ceilings(&q).get(target) - q.attributes.get(target);
                    a.gap_left[yy] += left;
                    if left < -0.01 {
                        a.over += 1;
                    }
                }
            }
            if coachable {
                co_n += 1;
                co_extra += first[2] - first[0];
            }
        }
        let t = took.max(1) as f64;
        println!("\n{label}: {took} players, {:?} {:.1} with {:.1} to his ceiling", target, rating0 / t, gap0 / t);
        println!(
            "year | focus gain: none / 30 / 60 (extra at 30, 60) | key skills: none / 30 / 60 | overall: none / 30 / 60 | every skill: none / 30 / 60 | room left: none / 30 / 60"
        );
        for yy in 0..y {
            let g = |k: usize, f: fn(&Arm) -> &Vec<f64>| f(&arms[k])[yy] / t;
            println!(
                "{:>4} | {:>5.2} / {:>5.2} / {:>5.2} ({:+.2}, {:+.2}) | {:>5.2} / {:>5.2} / {:>5.2} | {:>5.2} / {:>5.2} / {:>5.2} | {:>6.1} / {:>6.1} / {:>6.1} | {:>4.1} / {:>4.1} / {:>4.1}",
                yy + 1,
                g(0, |a| &a.target),
                g(1, |a| &a.target),
                g(2, |a| &a.target),
                g(1, |a| &a.target) - g(0, |a| &a.target),
                g(2, |a| &a.target) - g(0, |a| &a.target),
                g(0, |a| &a.cost),
                g(1, |a| &a.cost),
                g(2, |a| &a.cost),
                g(0, |a| &a.overall),
                g(1, |a| &a.overall),
                g(2, |a| &a.overall),
                g(0, |a| &a.all),
                g(1, |a| &a.all),
                g(2, |a| &a.all),
                g(0, |a| &a.gap_left),
                g(1, |a| &a.gap_left),
                g(2, |a| &a.gap_left),
            );
        }
        println!("past the ceiling at any year end: none {} / 30 {} / 60 {}", arms[0].over, arms[1].over, arms[2].over);
        println!(
            "coachable (18+): {co_n} players, year one's extra at 60 minutes {:+.2}",
            co_extra / co_n.max(1) as f64
        );
    }
}
