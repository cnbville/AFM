//! Where very fast men come from, level by level (the speed lane, Cain's
//! option A on the forty card, 2026-11-16): the forty a man's speed and
//! burst stand for (no test-day noise, `Drills::means`' formula) at each
//! step of the class sim, against the real combine.
//!
//! - Made at 14: his speed and burst ceilings at his grown weight, as
//!   `athlete::create` drew them (every boy of the class).
//! - College: his ratings after his last college season (every college man;
//!   a drafted man as he was drafted).
//! - Drafted: as drafted, and one combine day each (`body::combine`).
//!
//! Also, for the drafted men: how far the forty his ratings stand for moved
//! from the one his ceilings at 14 stood for, and by position the drafted
//! men's tested spread against real.
//!
//! `speed_levels [class size] [classes] [seed]`

use afm_engine::attributes::Attr;
use afm_engine::body::{combine, forty_weight};
use afm_engine::career::{CohortOptions, simulate_cohort};
use afm_engine::debug;
use afm_engine::player::{Player, ProspectOptions, generate_prospect};
use afm_engine::positions::Position;
use afm_engine::rng::Rng;

const AT: [f64; 5] = [4.25, 4.30, 4.35, 4.40, 4.45];

/// Real drafted men a draft at or under `AT` (nflverse combine 2000-2025,
/// only the men who ran: about 0.94 of men this fast are drafted).
const REAL_DRAFTED: [f64; 5] = [0.19, 1.27, 5.92, 15.73, 33.0];

/// Real drafted men by position: forty mean, sd, and men a draft at 4.70
/// or slower (same source).
const REAL_POS: [(Position, f64, f64, f64); 4] = [
    (Position::WR, 4.474, 0.094, 0.38),
    (Position::CB, 4.467, 0.082, 0.04),
    (Position::RB, 4.516, 0.096, 0.38),
    (Position::S, 4.531, 0.087, 0.35),
];

fn rated(speed: f64, accel: f64, weight: f64) -> f64 {
    5.56 - 0.0124 * (0.75 * speed + 0.25 * accel) + forty_weight(weight)
}

fn of(p: &Player) -> f64 {
    let b = afm_engine::body::body_now(p);
    rated(p.attributes.get(Attr::Speed), p.attributes.get(Attr::Acceleration), b.weight)
}

fn ceiling_forty(p: &Player) -> f64 {
    let c = debug::ceilings(p);
    rated(c.get(Attr::Speed), c.get(Attr::Acceleration), p.body.weight)
}

/// His position's overall from skills only (`positions`' lean check, S1).
fn skills(p: &Player) -> f64 {
    let (mut sum, mut total) = (0.0, 0.0);
    for a in Attr::ALL {
        let w = afm_engine::positions::weight(p.position, a);
        if w > 0.0 && a.profile().group != afm_engine::attributes::AttributeGroup::Physical {
            sum += w * p.attributes.get(a);
            total += w;
        }
    }
    sum / total.max(1e-9)
}

fn counts(v: &[f64]) -> [usize; 5] {
    AT.map(|t| v.iter().filter(|&&x| x <= t + 1e-9).count())
}

fn line(name: &str, v: &[f64], per: f64) {
    let c = counts(v);
    let mut s = format!("{name:<22} {:>8}", v.len());
    for (k, t) in AT.iter().enumerate() {
        s += &format!("  <={t:.2} {:>6.2}", c[k] as f64 / per);
    }
    println!("{s}");
}

fn ms(v: &[f64]) -> (f64, f64) {
    let n = v.len().max(1) as f64;
    let m = v.iter().sum::<f64>() / n;
    (m, (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / n).sqrt())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let size: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(250_000);
    let classes: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(4);
    let seed = args.get(3).cloned().unwrap_or_else(|| "speed".into());
    let (mut made, mut college, mut drafted_rated, mut drafted_test) = (vec![], vec![], vec![], vec![]);
    // (position, ceiling forty at 14, rated forty at the draft, tested)
    let mut drafted: Vec<(Position, f64, f64, f64)> = vec![];
    // Where the move from 14 comes from, in speed points and pounds:
    // (drafted, speed ceiling now - at 14, speed now - ceiling now, weight now - at 14).
    let mut parts: Vec<(bool, f64, f64, f64)> = vec![];
    let mut test_rng = Rng::new(0x5EED_4011);
    // FBS skill men as they left college: (position, rated forty, skills).
    let mut fbs: Vec<(Position, f64, f64)> = vec![];
    // Drafted men's weight and tested forty (the freak check).
    let mut freaks: Vec<(Position, f64, f64)> = vec![];
    for c in 0..classes {
        let s = format!("{seed}-{c}");
        // The class sim's own streams: the same boys it makes at 14.
        let mut rng = Rng::new(s.as_str());
        let seeds: Vec<u32> = (0..size).map(|_| rng.next_u32()).collect();
        let boys: Vec<(f64, f64, f64)> = seeds
            .iter()
            .enumerate()
            .map(|(i, &sd)| {
                let p =
                    generate_prospect(&mut Rng::new(sd), ProspectOptions { id: Some(i as u32), ..Default::default() });
                (ceiling_forty(&p), debug::ceilings(&p).get(Attr::Speed), p.body.weight)
            })
            .collect();
        let at14: Vec<f64> = boys.iter().map(|b| b.0).collect();
        made.extend(at14.iter().copied());
        let class = simulate_cohort(CohortOptions::new(size, s.as_str()));
        for (i, r) in class.records.iter().enumerate() {
            if r.college.is_none() {
                continue;
            }
            let p: &Player = r.at_draft.as_deref().unwrap_or(&r.player);
            college.push(of(p));
            if r.college == Some(afm_engine::career::CollegeLevel::FBS) {
                fbs.push((p.position, of(p), skills(p)));
            }
            let c = debug::ceilings(p).get(Attr::Speed);
            parts.push((
                r.draft_pick.is_some(),
                c - boys[i].1,
                p.attributes.get(Attr::Speed) - c,
                afm_engine::body::body_now(p).weight - boys[i].2,
            ));
            if r.draft_pick.is_some() {
                let t = combine(p, &mut test_rng).forty;
                drafted_rated.push(of(p));
                drafted_test.push(t);
                drafted.push((p.position, at14[i], of(p), t));
                freaks.push((p.position, afm_engine::body::body_now(p).weight, t));
            }
        }
    }
    let per = classes.max(1) as f64;
    println!("speed_levels: {classes} classes of {size}; men a class at or under each forty");
    line("made at 14 (ceilings)", &made, per);
    line("college (ratings)", &college, per);
    line("drafted (ratings)", &drafted_rated, per);
    line("drafted (combine)", &drafted_test, per);
    let mut s = format!("{:<22} {:>8}", "real drafted (combine)", "");
    for (k, t) in AT.iter().enumerate() {
        s += &format!("  <={t:.2} {:>6.2}", REAL_DRAFTED[k]);
    }
    println!("{s}");
    // Drift: ratings at the draft against ceilings at 14.
    let d: Vec<f64> = drafted.iter().map(|x| x.2 - x.1).collect();
    let (dm, dsd) = ms(&d);
    println!("drafted: rated forty minus ceiling forty at 14: mean {dm:+.3} s, sd {dsd:.3}");
    for (name, d) in [("college", false), ("drafted", true)] {
        let g: Vec<&(bool, f64, f64, f64)> = parts.iter().filter(|x| !d || x.0).collect();
        let (a, asd) = ms(&g.iter().map(|x| x.1).collect::<Vec<_>>());
        let (b, bsd) = ms(&g.iter().map(|x| x.2).collect::<Vec<_>>());
        let (w, wsd) = ms(&g.iter().map(|x| x.3).collect::<Vec<_>>());
        println!(
            "{name}: speed ceiling now - at 14 {a:+.2}/{asd:.2} pts; speed - ceiling now {b:+.2}/{bsd:.2}; weight now - at 14 {w:+.1}/{wsd:.1} lb"
        );
    }
    for t in [4.25, 4.30] {
        let fast: Vec<&(Position, f64, f64, f64)> = drafted.iter().filter(|x| x.2 <= t).collect();
        let already = fast.iter().filter(|x| x.1 <= t).count();
        println!(
            "drafted rated <= {t:.2}: {} men, {already} already <= {t:.2} at 14; their 14 forty mean {:.3}",
            fast.len(),
            fast.iter().map(|x| x.1).sum::<f64>() / fast.len().max(1) as f64
        );
    }
    // Every position's drafted mean against the real combine (SPEED_MIX's fit).
    const CSV: &str = include_str!("../../../../data/real-world/combine_by_position.csv");
    let mut fit = String::from("fit:");
    for pos in Position::ALL {
        let real = CSV.lines().skip(1).find_map(|l| {
            let f: Vec<&str> = l.split(',').collect();
            (f[0] == pos.name()).then(|| f[6].parse::<f64>().unwrap_or(0.0))
        });
        let g: Vec<f64> = drafted.iter().filter(|x| x.0 == pos).map(|x| x.3).collect();
        if let (Some(r), true) = (real, g.len() >= 5) {
            fit += &format!(" {pos:?} {:.3}-{r:.3}={:+.3} ({})", ms(&g).0, ms(&g).0 - r, g.len());
        }
    }
    println!("{fit}");
    // The freak check (no caps on freaks): big men who fly, and the
    // fastest for their weight (forty less `forty_weight`), against real
    // drafted men 2000-2025.
    let fr = |lo: f64, t: f64| freaks.iter().filter(|x| x.1 >= lo && x.2 <= t).count() as f64 / per;
    let adj = |x: &(Position, f64, f64)| x.2 - forty_weight(x.1);
    let fa = |t: f64| freaks.iter().filter(|x| adj(x) <= t).count() as f64 / per;
    println!(
        "freaks a draft: 250+ lb <=4.50 {:.2} (real 0.58), <=4.55 {:.2} (1.73); 300+ lb <=4.95 {:.2} (3.77); 230+ lb <=4.40 {:.2} (0.50); for weight <=4.30 {:.2} (0.35), <=4.33 {:.2} (0.96), <=4.36 {:.2} (2.31)",
        fr(250.0, 4.50),
        fr(250.0, 4.55),
        fr(300.0, 4.95),
        fr(230.0, 4.40),
        fa(4.30),
        fa(4.33),
        fa(4.36)
    );
    let mut best: Vec<&(Position, f64, f64)> = freaks.iter().collect();
    best.sort_by(|a, b| a.2.total_cmp(&b.2));
    let mut b = String::from("fastest drafted (real best 4.21 Worthy 165 lb; 4.24 Oweh 257 for weight):");
    for x in best.iter().take(5) {
        b += &format!(" {:?} {:.0} lb {:.2};", x.0, x.1, x.2);
    }
    best.sort_by(|a, b| adj(a).total_cmp(&adj(b)));
    b += " for weight:";
    for x in best.iter().take(3) {
        b += &format!(" {:?} {:.0} lb {:.2};", x.0, x.1, x.2);
    }
    println!("{b}");
    // FBS men by position as they left college, and the lean check (S1):
    // the fastest tenth with bottom-half skills, 5% with no lean.
    let (mut hits, mut n) = (0usize, 0usize);
    let mut line_fbs = String::from("FBS as they left:");
    for pos in [
        Position::QB,
        Position::RB,
        Position::WR,
        Position::TE,
        Position::EDGE,
        Position::LB,
        Position::CB,
        Position::S,
    ] {
        let g: Vec<&(Position, f64, f64)> = fbs.iter().filter(|x| x.0 == pos).collect();
        if g.len() < 20 {
            continue;
        }
        let (m, sd) = ms(&g.iter().map(|x| x.1).collect::<Vec<_>>());
        line_fbs += &format!(" {pos:?} {m:.3}/{sd:.3}");
        if matches!(pos, Position::WR | Position::CB | Position::RB | Position::S) {
            let mut f: Vec<f64> = g.iter().map(|x| x.1).collect();
            f.sort_by(f64::total_cmp);
            let cut = f[f.len() / 10];
            let mut sk: Vec<f64> = g.iter().map(|x| x.2).collect();
            sk.sort_by(f64::total_cmp);
            let mid = sk[sk.len() / 2];
            hits += g.iter().filter(|x| x.1 <= cut && x.2 < mid).count();
            n += g.len();
        }
    }
    println!("{line_fbs}");
    println!(
        "lean check: FBS WR/CB/RB/S fastest tenth with bottom-half skills {:.1}% (5.0% with no lean)",
        100.0 * hits as f64 / n.max(1) as f64
    );
    for (pos, m, sd, slow) in REAL_POS {
        let g: Vec<&(Position, f64, f64, f64)> = drafted.iter().filter(|x| x.0 == pos).collect();
        let (cm, csd) = ms(&g.iter().map(|x| x.1).collect::<Vec<_>>());
        let (rm, rsd) = ms(&g.iter().map(|x| x.2).collect::<Vec<_>>());
        let (tm, tsd) = ms(&g.iter().map(|x| x.3).collect::<Vec<_>>());
        let slow_n = g.iter().filter(|x| x.3 >= 4.695).count() as f64 / per;
        println!(
            "{:<4} {:>4}  at 14 {cm:.3}/{csd:.3}  rated {rm:.3}/{rsd:.3}  combine {tm:.3}/{tsd:.3} (real {m:.3}/{sd:.3})  4.70+ {slow_n:.2} a draft (real {slow})",
            format!("{pos:?}"),
            g.len()
        );
    }
}
