//! The progression update's class-sim checks (`docs/design/progression.md`
//! and `progression-everyday.md`, "Checks at the build"): one national
//! class with the split ceiling on and the same class with it off.
//!
//! `cargo run --release -p afm-tools --bin prog_check -- [size] [seed]`
//!
//! Prints, per position and limited skill: collegians whose best college
//! season reached 70+ and 80+ (off, on); the room at college entry
//! (median, p90, p99); and the elite-tools-bad-technique test (top-quarter
//! tools for the skill, bottom-third rating at entry; best college season,
//! median, off and on, by knack tier).

use afm_engine::career::{CohortOptions, simulate_cohort};
use afm_engine::positions::Position;
use afm_engine::progression::{self, LIMITED, kind_of};

fn q(v: &mut [f64], p: f64) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    v[((p * (v.len() - 1) as f64).round() as usize).min(v.len() - 1)]
}

/// The families the designs measured, one per position group (never lumped).
const FAMILIES: &[(Position, afm_engine::attributes::Attr)] = {
    use afm_engine::attributes::Attr as A;
    use afm_engine::positions::Position as P;
    &[
        (P::EDGE, A::FinesseRush),
        (P::DT, A::PowerRush),
        (P::OT, A::PassBlocking),
        (P::IOL, A::PassBlocking),
        (P::WR, A::RouteRunning),
        (P::TE, A::RunBlocking),
        (P::RB, A::PassBlocking),
        (P::LB, A::ZoneCoverage),
        (P::CB, A::ManCoverage),
        (P::S, A::ManCoverage),
        (P::S, A::Tackling),
        (P::QB, A::Footwork),
        (P::WR, A::ContestedCatch),
        (P::CB, A::Press),
        (P::EDGE, A::PowerRush),
        (P::LB, A::Pursuit),
        (P::LB, A::Blitzing),
        (P::RB, A::LegDrive),
        (P::QB, A::ThrowOnMove),
        (P::WR, A::Release),
        (P::DT, A::BlockShedding),
        (P::OT, A::RunBlocking),
        (P::IOL, A::RunBlocking),
    ]
};

/// Collegians at a position whose best college season in a limited skill
/// (by place in `LIMITED`) reached each bar.
fn stars(rs: &afm_engine::career::CohortResult) -> Vec<[[u32; 2]; progression::LIMITED_COUNT]> {
    let mut out = vec![[[0u32; 2]; progression::LIMITED_COUNT]; Position::ALL.len()];
    for pp in rs.records.iter().filter_map(|r| r.prog_probe.as_ref()) {
        for (o, &b) in out[pp.position.index()].iter_mut().zip(pp.best.iter()) {
            if b >= 70.0 {
                o[0] += 1;
            }
            if b >= 80.0 {
                o[1] += 1;
            }
        }
    }
    out
}

/// `prog_check fit [size] [rounds] [seeds...]`: sizes a trim for every
/// family (each limited skill at each position where it is a key skill)
/// so its 70+ and 80+ counts with every part on match the counts with the
/// split ceiling off, and prints the table. A trim has two slopes (past 60
/// and past 75, `progression::trim_at`): the first is moved by the 70+
/// count's miss and the second by the 80+ count's.
fn fit(args: &[String]) {
    if let Some(f) = std::env::var("AFM_TRIM_LIMIT").ok().and_then(|v| v.parse::<f64>().ok()) {
        progression::set_trim_limit_share(f);
    }
    let size: usize = args.first().and_then(|s| s.parse().ok()).unwrap_or(250_000);
    let rounds: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(6);
    let seeds: Vec<String> =
        if args.len() > 2 { args[2..].to_vec() } else { vec!["fitA".to_string(), "fitB".to_string()] };
    let sum = |v: Vec<Vec<[[u32; 2]; progression::LIMITED_COUNT]>>| {
        let mut t = vec![[[0u32; 2]; progression::LIMITED_COUNT]; Position::ALL.len()];
        for x in v {
            for p in 0..t.len() {
                for k in 0..progression::LIMITED_COUNT {
                    t[p][k][0] += x[p][k][0];
                    t[p][k][1] += x[p][k][1];
                }
            }
        }
        t
    };
    progression::set_enabled(false);
    let off = sum(seeds.iter().map(|s| stars(&simulate_cohort(CohortOptions::new(size, s.as_str())))).collect());
    progression::set_enabled(true);
    let key = |pos: Position, k: usize| {
        let top = LIMITED.iter().map(|&a| afm_engine::positions::weight(pos, a)).fold(0.0, f64::max);
        !pos.is_specialist() && top > 0.0 && afm_engine::positions::weight(pos, LIMITED[k]) >= 0.5 * top
    };
    let mut t = progression::trims();
    // A family with a trim in the table is fitted too, key or not.
    let first = t.clone();
    let key = |pos: Position, k: usize| key(pos, k) || first[pos.index()][k] != [0.0; 2];
    // A key family with no trim yet starts small.
    for pos in Position::ALL {
        for (k, v) in t[pos.index()].iter_mut().enumerate() {
            if key(pos, k) && *v == [0.0; 2] {
                *v = [0.2, 0.06];
            }
        }
    }
    for round in 0..rounds {
        progression::set_trims(Some(t.clone()));
        let on = sum(seeds.iter().map(|s| stars(&simulate_cohort(CohortOptions::new(size, s.as_str())))).collect());
        println!("round {round}");
        for pos in Position::ALL {
            for k in 0..progression::LIMITED_COUNT {
                if !key(pos, k) {
                    continue;
                }
                let (o, n) = (off[pos.index()][k], on[pos.index()][k]);
                let e70 = ((n[0] as f64 + 2.0) / (o[0] as f64 + 2.0)).ln();
                let e80 = ((n[1] as f64 + 2.0) / (o[1] as f64 + 2.0)).ln();
                let [a, b] = t[pos.index()][k];
                println!(
                    "  {:<5} {:<15} trim {:>4.2}/{:>4.2} (at 70 {:>4.1}, 80 {:>4.1}, 90 {:>4.1})  70+ {:>5} > {:>5}   80+ {:>4} > {:>4}",
                    format!("{pos:?}"),
                    format!("{:?}", LIMITED[k]),
                    a,
                    b,
                    10.0 * a,
                    20.0 * a + 5.0 * b,
                    30.0 * a + 15.0 * b,
                    o[0],
                    n[0],
                    o[1],
                    n[1]
                );
                // The 70+ count mostly reads natural ceilings of 70-80 (the
                // first slope); the 80+ count adds the second.
                // Both slopes together stay between 0 and 0.8, so a trim
                // never shrinks as the ceiling rises (the second slope can
                // flatten it) and a higher natural ceiling always keeps a
                // higher trimmed one.
                let a2 = (a as f64 + 0.5 * e70).clamp(0.0, 0.8);
                let b2 = (b as f64 + 0.8 * (e80 - 0.7 * e70)).clamp(-a2 - 0.6, 0.8 - a2);
                t[pos.index()][k] = [a2 as f32, b2 as f32];
            }
        }
    }
    println!("fitted table (paste between the FITTED_TRIMS markers in progression.rs):");
    for pos in Position::ALL {
        for k in 0..progression::LIMITED_COUNT {
            let [a, b] = t[pos.index()][k];
            if a > 0.0 || b != 0.0 {
                println!("    t[{}][{k}] = [{a:.2}, {b:.2}]; // {pos:?} {:?}", pos.index(), LIMITED[k]);
            }
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // AFM_TOP_SLOW: the span of closing's slowing past 75 (0 off).
    if let Some(f) = std::env::var("AFM_TOP_SLOW").ok().and_then(|v| v.parse::<f64>().ok()) {
        progression::set_top_slow(f);
    }
    if let Some(f) = std::env::var("AFM_SLOW_FROM").ok().and_then(|v| v.parse::<f64>().ok()) {
        progression::set_slow_from(f);
    }
    if args.get(1).is_some_and(|a| a == "drift") {
        use afm_engine::attributes::Attr as A;
        for pos in Position::ALL {
            print!("{pos:?}:");
            for a in [
                A::Speed,
                A::Acceleration,
                A::Agility,
                A::Flexibility,
                A::Strength,
                A::Balance,
                A::PlayRecognition,
                A::FootballIq,
                A::Toughness,
                A::Motor,
                A::Composure,
            ] {
                print!(
                    "  {a:?} {:+.1}/{:+.1}/{:+.1}",
                    progression::drift_at(pos, a, 18),
                    progression::drift_at(pos, a, 22),
                    progression::drift_at(pos, a, 26)
                );
            }
            println!();
        }
        return;
    }
    if args.get(1).is_some_and(|a| a == "fit") {
        fit(&args[2..]);
        return;
    }
    let size: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(250_000);
    let seed = args.get(2).cloned().unwrap_or_else(|| "progcheck".to_string());
    progression::set_enabled(false);
    let off = simulate_cohort(CohortOptions::new(size, seed.as_str()));
    progression::set_enabled(true);
    // AFM_PROG_PARTS: everyday 1, breakouts 2, projects 4, trims 8 (all 15).
    let parts: u8 = std::env::var("AFM_PROG_PARTS").ok().and_then(|v| v.parse().ok()).unwrap_or(15);
    progression::set_parts(parts);
    // AFM_TRIM_ALL: every trim in the table set to this (a test of the lever).
    if let Some(x) = std::env::var("AFM_TRIM_ALL").ok().and_then(|v| v.parse::<f32>().ok()) {
        let t: Vec<[[f32; 2]; progression::LIMITED_COUNT]> = progression::trims()
            .iter()
            .map(|row| row.map(|v| if v[0] > 0.0 { [x, 0.3 * x] } else { [0.0; 2] }))
            .collect();
        progression::set_trims(Some(t));
    }
    let on = simulate_cohort(CohortOptions::new(size, seed.as_str()));
    println!("prog_check: class of {size}, seed {seed}, parts {parts}");
    println!();
    println!(
        "{:<5} {:<16} {:>6} {:>6} {:>6} {:>6}   {:>5} {:>5} {:>5}   {:>22} {:>22} {:>22}",
        "pos", "skill", "70off", "70on", "80off", "80on", "room", "p90", "p99", "knack (n off>on)", "middling", "none"
    );
    for &(pos, a) in FAMILIES {
        let k = LIMITED.iter().position(|&x| x == a).unwrap();
        let count = |rs: &afm_engine::career::CohortResult, t: f32| {
            rs.records
                .iter()
                .filter_map(|r| r.prog_probe.as_ref())
                .filter(|pp| pp.position == pos && pp.best[k] >= t)
                .count()
        };
        let mut rooms: Vec<f64> = on
            .records
            .iter()
            .filter_map(|r| r.prog_probe.as_ref())
            .filter(|pp| pp.position == pos)
            .map(|pp| (pp.limit[k] - pp.ceiling[k]) as f64)
            .collect();
        // Elite tools, bad technique: top quarter of the rank map, bottom
        // third of the rating, among this class's collegians at the spot.
        let group: Vec<usize> = (0..on.records.len().min(off.records.len()))
            .filter(|&i| {
                on.records[i].prog_probe.as_ref().is_some_and(|pp| pp.position == pos)
                    && off.records[i].prog_probe.as_ref().is_some_and(|pp| pp.position == pos)
            })
            .collect();
        let mut tools: Vec<f64> =
            group.iter().map(|&i| on.records[i].prog_probe.as_ref().unwrap().mapped[k] as f64).collect();
        let mut film: Vec<f64> =
            group.iter().map(|&i| on.records[i].prog_probe.as_ref().unwrap().rating[k] as f64).collect();
        let t75 = q(&mut tools, 0.75);
        let f33 = q(&mut film, 0.33);
        let kind = kind_of(a).unwrap();
        let mut tiers: [(Vec<f64>, Vec<f64>); 3] = Default::default();
        for &i in &group {
            let pp = on.records[i].prog_probe.as_ref().unwrap();
            if (pp.mapped[k] as f64) < t75 || (pp.rating[k] as f64) > f33 {
                continue;
            }
            let tier = progression::tier(progression::knack(&on.records[i].player, kind)) as usize;
            tiers[tier].0.push(off.records[i].prog_probe.as_ref().unwrap().best[k] as f64);
            tiers[tier].1.push(pp.best[k] as f64);
        }
        let tier = |t: &mut (Vec<f64>, Vec<f64>)| {
            let n = t.0.len();
            format!("{:>4} {:>5.1}>{:<5.1}", n, q(&mut t.0, 0.5), q(&mut t.1, 0.5))
        };
        println!(
            "{:<5} {:<16} {:>6} {:>6} {:>6} {:>6}   {:>5.1} {:>5.1} {:>5.1}   {:>22} {:>22} {:>22}",
            format!("{pos:?}"),
            format!("{a:?}"),
            count(&off, 70.0),
            count(&on, 70.0),
            count(&off, 80.0),
            count(&on, 80.0),
            q(&mut rooms, 0.5),
            q(&mut rooms, 0.9),
            q(&mut rooms, 0.99),
            tier(&mut tiers[0]),
            tier(&mut tiers[1]),
            tier(&mut tiers[2]),
        );
    }
    // The read's move for a boy who grows like his class should sit near 0
    // on average (the drift taken off): 14 to college entry, and to his
    // last college season, by position (the off arm, the same players).
    println!();
    for &(pos, a) in FAMILIES {
        let k = LIMITED.iter().position(|&x| x == a).unwrap();
        let v: Vec<(f64, f64)> = off
            .records
            .iter()
            .filter_map(|r| r.prog_probe.as_ref())
            .filter(|pp| pp.position == pos && pp.mapped14[k] > 0.0)
            .map(|pp| ((pp.mapped[k] - pp.mapped14[k]) as f64, (pp.mapped_last[k] - pp.mapped14[k]) as f64))
            .collect();
        let n = v.len().max(1) as f64;
        println!(
            "read move {:<5} {:<16} n {:>5}  14>entry {:+.2}  14>last {:+.2}",
            format!("{pos:?}"),
            format!("{a:?}"),
            v.len(),
            v.iter().map(|x| x.0).sum::<f64>() / n,
            v.iter().map(|x| x.1).sum::<f64>() / n
        );
    }
    let picks = |rs: &afm_engine::career::CohortResult| rs.records.iter().filter(|r| r.draft_pick.is_some()).count();
    println!();
    println!("drafted: off {}, on {}", picks(&off), picks(&on));
    for pos in Position::ALL {
        let (mut n, mut pr) = (0u32, 0u32);
        for pp in on.records.iter().filter_map(|r| r.prog_probe.as_ref()).filter(|pp| pp.position == pos) {
            n += pp.seasons as u32;
            pr += pp.project_seasons as u32;
        }
        if n > 0 {
            print!("{pos:?} {:.1}%  ", 100.0 * pr as f64 / n as f64);
        }
    }
    println!("of college seasons on a skill project");
}
