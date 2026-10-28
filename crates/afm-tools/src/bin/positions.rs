//! Athlete-first check (section 3, topic 5.2): position shares in high school
//! and in the draft against real data, body sizes of drafted players against
//! the combine, and how often players change position. Since update 3 lane 1
//! also each position's combine forty against the real combine, skills a
//! position never uses, and how much of his top speed a high schooler has.
//! Usage: `positions [size] [seed]`.

use afm_engine::attributes::Attr;
use afm_engine::attributes::AttributeGroup;
use afm_engine::body::{Drills, body_now, combine, combine_size};
use afm_engine::career::CollegeLevel;
use afm_engine::career::{CohortOptions, simulate_cohort};
use afm_engine::development::{SeasonContext, develop_season};
use afm_engine::player::Player;
use afm_engine::player::{Level, PlayingTime, ProspectOptions, generate_prospect};
use afm_engine::positions::{POSITION_COUNT, Position, weight};
use afm_tools::stats::real_draft_shares;

const COMBINE_CSV: &str = include_str!("../../../../data/real-world/combine_by_position.csv");

/// The real combine's forty per position: (mean, sd).
fn real_forty(pos: Position) -> Option<(f64, f64)> {
    COMBINE_CSV.lines().skip(1).find_map(|line| {
        let f: Vec<&str> = line.split(',').collect();
        (f[0] == pos.name()).then(|| (f[6].parse().unwrap_or(0.0), f[7].parse().unwrap_or(0.0)))
    })
}

fn mean_sd(v: &[f64]) -> (f64, f64) {
    let n = v.len().max(1) as f64;
    let m = v.iter().sum::<f64>() / n;
    (m, (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / n).sqrt())
}

fn median(v: &mut [f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

/// His position's overall from skills only (no body): the "skills" of the
/// speed update's lean check (S1).
fn skills(p: &Player) -> f64 {
    let (mut sum, mut total) = (0.0, 0.0);
    for a in Attr::ALL {
        let w = weight(p.position, a);
        if w > 0.0 && a.profile().group != AttributeGroup::Physical {
            sum += w * p.attributes.get(a);
            total += w;
        }
    }
    sum / total.max(1e-9)
}

/// The speed update's lean check (S1): among FBS receivers, corners, backs
/// and safeties, how many are in the fastest tenth at their position with
/// skills in the bottom half (5% if speed says nothing about skills).
fn lean_check(recs: &[afm_engine::career::CareerRecord]) {
    let (mut hits, mut n) = (0usize, 0usize);
    for pos in [Position::WR, Position::CB, Position::RB, Position::S] {
        let group: Vec<&Player> = recs
            .iter()
            .filter(|r| r.college == Some(CollegeLevel::FBS))
            .map(|r| r.at_draft.as_deref().unwrap_or(&r.player))
            .filter(|p| p.position == pos)
            .collect();
        if group.is_empty() {
            continue;
        }
        let mut forty: Vec<f64> = group.iter().map(|p| Drills::of(p).expected().forty).collect();
        let mut skill: Vec<f64> = group.iter().map(|p| skills(p)).collect();
        forty.sort_by(f64::total_cmp);
        let cut = forty[forty.len() / 10];
        let mid = median(&mut skill);
        hits += group.iter().filter(|p| Drills::of(p).expected().forty <= cut && skills(p) < mid).count();
        n += group.len();
    }
    println!(
        "lean check: FBS WR/CB/RB/S in the fastest tenth with bottom-half skills: {hits} of {n} ({:.1}%, 5.0% with no lean)",
        100.0 * hits as f64 / n.max(1) as f64
    );
}

/// The biggest one-year drops among drafted players, with what the year
/// held (every trial names its big slumps and why).
fn slumps(recs: &[afm_engine::career::CareerRecord]) {
    let mut drops: Vec<(f64, String)> = Vec::new();
    for r in recs.iter().filter(|r| r.draft_pick.is_some()) {
        for h in &r.player.history {
            let d = h.overall - h.overall_start;
            if d <= -4.0 {
                let why: Vec<String> = h.events.iter().map(|e| format!("{e:?}")).collect();
                drops.push((
                    d,
                    format!(
                        "{} id {} at {} ({}, {}): {:.1} -> {:.1} [{}]",
                        h.position.name(),
                        r.player.id,
                        h.age,
                        h.level.name(),
                        h.playing_time.name(),
                        h.overall_start,
                        h.overall,
                        if why.is_empty() { "ageing".to_string() } else { why.join(", ") }
                    ),
                ));
            }
        }
    }
    drops.sort_by(|a, b| a.0.total_cmp(&b.0));
    println!("drafted players' one-year drops of 4 or more: {}", drops.len());
    for (_, line) in drops.iter().take(8) {
        println!("  {line}");
    }
}

/// High school speed (speed update S4): kids grown through four high school
/// years as starters with an average staff, their speed against its ceiling
/// each spring, and how many run 4.40 or faster, per 250,000.
fn hs_speed(n: usize) {
    let mut rng = afm_engine::rng::Rng::new("hs-speed");
    let mut share = [0.0; 4];
    let mut fast = [0usize; 4];
    let mut weight = [[0.0f64; 2]; 4];
    let ctx = SeasonContext::new(Level::HS, 10.0, PlayingTime::Starter);
    for _ in 0..n {
        let mut p = generate_prospect(&mut rng, ProspectOptions::default());
        let natural = p.position;
        p.position_history.clear();
        afm_engine::athlete::set_position(&mut p, natural);
        for (y, s) in share.iter_mut().enumerate() {
            develop_season(&mut p, &ctx, &mut rng);
            *s += p.attributes.get(Attr::Speed) / afm_engine::debug::ceilings(&p).get(Attr::Speed).max(1.0);
            fast[y] += (Drills::of(&p).expected().forty <= 4.40) as usize;
            weight[y][0] += body_now(&p).weight / p.body.weight;
            weight[y][1] += 1.0;
        }
    }
    let k = 250_000.0 / n as f64;
    for (y, name) in ["freshman", "sophomore", "junior", "senior"].iter().enumerate() {
        println!(
            "HS {name} spring: speed {:.3} of its ceiling, {:.0} at 4.40 or faster per 250k, weight {:.3} of grown",
            share[y] / n as f64,
            fast[y] as f64 * k,
            weight[y][0] / weight[y][1].max(1.0)
        );
    }
}

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
    let mut spec_forty: [Vec<f64>; 2] = Default::default();
    let mut forty: Vec<Vec<f64>> = vec![Vec::new(); POSITION_COUNT];
    let mut unused: Vec<[Vec<f64>; 4]> = vec![Default::default(); POSITION_COUNT];
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
            spec_forty[(r.player.position == Position::P) as usize].push(combine(&r.player, &mut rng).forty);
        }
        if r.draft_pick.is_some() {
            // As he was on draft day, not at the end of his career.
            let p = r.at_draft.as_deref().unwrap_or(&r.player);
            let k = p.position.index();
            drafted[k] += 1;
            drafted_n += 1;
            drafted_moved += (p.position != first) as usize;
            let b = body_now(p);
            body[k].0 += b.height;
            body[k].1 += b.weight;
            lift[k].0 += p.attributes.get(Attr::Strength);
            lift[k].1 += b.arm;
            let c = combine(p, &mut rng);
            lift[k].2 += c.bench;
            forty[k].push(c.forty);
            unused[k][0].push(p.attributes.get(Attr::Elusiveness));
            unused[k][1].push(afm_engine::debug::ceilings(p).get(Attr::Elusiveness));
            unused[k][2].push(p.attributes.get(Attr::ManCoverage));
            unused[k][3].push(afm_engine::debug::ceilings(p).get(Attr::ManCoverage));
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
    // No long snappers come out of a class (they sign as free agents).
    for pos in Position::ALL.into_iter().filter(|&p| p != Position::LS) {
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
    println!(
        "pos   drafted forty mean/sd (real), fast and slow tails   elusiveness, man coverage (medians, rating/ceiling)"
    );
    for pos in Position::ALL.into_iter().filter(|&p| p != Position::LS) {
        let k = pos.index();
        let (m, sd) = mean_sd(&forty[k]);
        let (rm, rsd) = real_forty(pos).unwrap_or((0.0, 0.0));
        // The tails against a normal curve on the real mean and spread (2.3% each).
        let slow = forty[k].iter().filter(|&&f| f > rm + 2.0 * rsd).count() as f64;
        let quick = forty[k].iter().filter(|&&f| f < rm - 2.0 * rsd).count() as f64;
        let n = forty[k].len().max(1) as f64;
        let [elu, elu_c, man, man_c] = &mut unused[k];
        let elu_p90 = {
            let mut v = elu.clone();
            v.sort_by(f64::total_cmp);
            v.get(v.len() * 9 / 10).copied().unwrap_or(0.0)
        };
        println!(
            "{:<4} {:.2}/{:.2} ({:.2}/{:.2}) tails {:4.1}% {:4.1}%   {:3.0}/{:3.0} (p90 {:3.0}) {:3.0}/{:3.0}",
            pos.name(),
            m,
            sd,
            rm,
            rsd,
            100.0 * quick / n,
            100.0 * slow / n,
            median(elu),
            median(elu_c),
            elu_p90,
            median(man),
            median(man_c)
        );
    }
    for (k, name) in ["K", "P"].iter().enumerate() {
        let n = spec[k].2.max(1.0);
        let (m, sd) = mean_sd(&spec_forty[k]);
        let pos = if k == 0 { Position::K } else { Position::P };
        let (rm, rsd) = real_forty(pos).unwrap_or((0.0, 0.0));
        println!(
            "college {name}: {:.0} players, {:.1} in / {:.0} lb, forty {m:.2}/{sd:.2} (real {rm:.2}/{rsd:.2})",
            spec[k].2,
            spec[k].0 / n,
            spec[k].1 / n
        );
    }
    println!(
        "changed position at least once: {:.1}% of the class; drafted at a different position than as a freshman: {:.1}%",
        100.0 * moved as f64 / recs.len() as f64,
        100.0 * drafted_moved as f64 / drafted_n.max(1) as f64
    );
    lean_check(recs);
    slumps(recs);
    hs_speed(20_000);
}
