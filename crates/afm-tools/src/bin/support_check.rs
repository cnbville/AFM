//! College money and the support staff market (realism audit items 9 and
//! 10): loads a world file, prints each tier's football staff and recruiting
//! lines against what its departments earn, and the support heads by level
//! (rating, pay, age, years in the job); then plays the world on for a
//! number of days (games on the stand-in) and prints the same with the
//! market's moves each year.
//! Usage: support_check <world file> [days]

use afm_engine::world::World;
use afm_engine::world::data::Division;
use afm_engine::world::finance::college::{self, INCOME, POSTSEASON, RECRUITING, STAFF, SUPPORT};
use afm_engine::world::people::Dept;
use std::path::Path;

fn q(v: &mut [f64], p: f64) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(f64::total_cmp);
    v[((v.len() - 1) as f64 * p).round() as usize]
}

fn m(x: f64) -> String {
    if x >= 1.0e6 { format!("{:.1}M", x / 1.0e6) } else { format!("{:.0}K", x / 1.0e3) }
}

fn books(w: &World) {
    println!("\n== Football staff and recruiting lines by tier, against what each department earns ({})", w.year);
    let names = ["Power", "Group of 5", "FCS", "below"];
    for t in 0..4u8 {
        let (mut earned, mut staff, mut rec, mut ratio, mut level) = (vec![], vec![], vec![], vec![], vec![]);
        let (mut fit_s, mut fit_r) = (vec![], vec![]);
        for (p, prog) in w.programs.iter().enumerate() {
            if !prog.plays(w.year) || college::tier(w, p, w.year) != t {
                continue;
            }
            let d = &w.finance.college[p];
            if d.budget.year == 0 {
                continue;
            }
            let b = &d.budget;
            let last = d.years.last().map_or(0.0, |y| y.l[POSTSEASON] as f64 + y.l[7] as f64);
            let e = b.sum(0..INCOME) - b.l[SUPPORT] as f64 + last;
            let push = college::football_push(prog.prestige, d.reserves as f64, e);
            fit_s.push(b.l[STAFF] as f64 / (e * push).max(1.0));
            fit_r.push(b.l[RECRUITING] as f64 / (e * push).max(1.0));
            earned.push(e);
            staff.push(b.l[STAFF] as f64);
            rec.push(b.l[RECRUITING] as f64);
            ratio.push((b.l[STAFF] + b.l[RECRUITING]) as f64 / e.max(1.0));
            level.push(d.level as f64);
        }
        let n = earned.len();
        println!(
            "{:<11} n={n:>3}  earned {} / {} / {} (max {})  staff {} / {} / {} (max {})  recruiting {} / {} / {} (max {})  share of earnings {:.3}  level {:.2} / {:.2} / {:.2}",
            names[t as usize],
            m(q(&mut earned, 0.1)),
            m(q(&mut earned, 0.5)),
            m(q(&mut earned, 0.9)),
            m(q(&mut earned, 1.0)),
            m(q(&mut staff, 0.1)),
            m(q(&mut staff, 0.5)),
            m(q(&mut staff, 0.9)),
            m(q(&mut staff, 1.0)),
            m(q(&mut rec, 0.1)),
            m(q(&mut rec, 0.5)),
            m(q(&mut rec, 0.9)),
            m(q(&mut rec, 1.0)),
            q(&mut ratio, 0.5),
            q(&mut level, 0.1),
            q(&mut level, 0.5),
            q(&mut level, 0.9),
        );
        println!(
            "            fitted shares of earnings x push: staff {:.4}, recruiting {:.4}",
            q(&mut fit_s, 0.5),
            q(&mut fit_r, 0.5)
        );
    }
    // The richest ten and what they spend.
    let mut rows: Vec<(f64, f64, f64, String)> = w
        .programs
        .iter()
        .enumerate()
        .filter(|(p, x)| x.plays(w.year) && w.finance.college[*p].budget.year != 0)
        .map(|(p, x)| {
            let b = &w.finance.college[p].budget;
            (
                b.sum(0..INCOME) - b.l[SUPPORT] as f64,
                b.l[STAFF] as f64,
                b.l[RECRUITING] as f64,
                x.name(afm_engine::world::names::NameMode::Real).to_string(),
            )
        })
        .collect();
    rows.sort_by(|a, b| b.0.total_cmp(&a.0));
    for r in rows.iter().take(8) {
        println!("  {:<24} earns {}  staff {}  recruiting {}", r.3, m(r.0), m(r.1), m(r.2));
    }
}

/// Which programs a row of the table counts.
type Keep = fn(&World, usize) -> bool;

fn heads(w: &World) {
    println!(
        "\n== Support heads by level ({}): rating mean (p10-p90), pay median, age median, years in the job",
        w.year
    );
    let groups: [(&str, Keep); 4] = [
        ("FBS", |w, p| w.programs[p].division == Division::FBS),
        ("FCS", |w, p| w.programs[p].division == Division::FCS),
        ("D2/NAIA/JUCO", |w, p| matches!(w.programs[p].division, Division::D2 | Division::NAIA | Division::JUCO)),
        ("D3", |w, p| w.programs[p].division == Division::D3),
    ];
    for d in Dept::ALL {
        for (name, keep) in groups {
            let (mut r, mut pay, mut age, mut yrs) = (vec![], vec![], vec![], vec![]);
            for p in 0..w.programs.len() {
                if !w.programs[p].plays(w.year) || !keep(w, p) {
                    continue;
                }
                let s = d.post(&w.programs[p].support);
                if s.rating == 0 {
                    continue;
                }
                r.push(s.rating as f64);
                pay.push(s.pay as f64);
                age.push((w.year - s.born) as f64);
                yrs.push((w.year - s.since) as f64);
            }
            let mean = r.iter().sum::<f64>() / r.len().max(1) as f64;
            println!(
                "  {:<11} {:<13} {mean:>5.1} ({:>2.0}-{:<2.0})  pay {:>6}  age {:>2.0}  years {:.0}",
                d.name(),
                name,
                q(&mut r, 0.1),
                q(&mut r, 0.9),
                m(q(&mut pay, 0.5)),
                q(&mut age, 0.5),
                q(&mut yrs, 0.5)
            );
        }
        if d.nfl() {
            let mut r: Vec<f64> =
                w.clubs.iter().map(|c| d.post(&c.support).rating as f64).filter(|&x| x > 0.0).collect();
            let mut pay: Vec<f64> = w.clubs.iter().map(|c| d.post(&c.support).pay as f64).collect();
            let mean = r.iter().sum::<f64>() / r.len().max(1) as f64;
            println!(
                "  {:<11} {:<13} {mean:>5.1} ({:>2.0}-{:<2.0})  pay {:>6}",
                d.name(),
                "NFL",
                q(&mut r, 0.1),
                q(&mut r, 0.9),
                m(q(&mut pay, 0.5))
            );
        }
    }
    // Do the best heads sit at the richest programs? Top-20 FBS by staff line
    // against the bottom 20.
    let mut fbs: Vec<(f64, f64)> = (0..w.programs.len())
        .filter(|&p| w.programs[p].plays(w.year) && w.programs[p].division == Division::FBS)
        .map(|p| (w.finance.college[p].budget.l[STAFF] as f64, w.programs[p].support.strength.rating as f64))
        .collect();
    fbs.sort_by(|a, b| b.0.total_cmp(&a.0));
    let top = fbs.iter().take(20).map(|x| x.1).sum::<f64>() / 20.0;
    let bot = fbs.iter().rev().take(20).map(|x| x.1).sum::<f64>() / 20.0;
    println!("  FBS strength heads: richest 20 staff lines {top:.1}, poorest 20 {bot:.1}");
    let mut top_pay: Vec<(f32, String, u8)> = w
        .programs
        .iter()
        .map(|p| {
            (
                p.support.strength.pay,
                p.name(afm_engine::world::names::NameMode::Real).to_string(),
                p.support.strength.rating,
            )
        })
        .collect();
    top_pay.sort_by(|a, b| b.0.total_cmp(&a.0));
    let list: Vec<String> = top_pay.iter().take(5).map(|x| format!("{} {} ({})", x.1, m(x.0 as f64), x.2)).collect();
    println!("  best-paid college strength heads: {}", list.join(", "));
    println!("  free agents on the market: {}", w.support_market.free.len());
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let t = std::time::Instant::now();
    let mut w = afm_engine::save::read_world(Path::new(&args[1])).expect("read the world");
    eprintln!("read in {:.0} s, {:?}", t.elapsed().as_secs_f64(), w.today);
    w.options.engine_from = Some(2100);
    books(&w);
    heads(&w);
    // `market <years>`: the market on its own, with a stand-in carousel.
    if args.get(2).map(String::as_str) == Some("market") {
        let years: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(10);
        for k in 0..years {
            afm_engine::debug::support_market_years(&mut w, 1, 0.18);
            if k % 3 == 2 || k + 1 == years {
                println!("\n######## market year {}", w.year);
                if let Some(y) = w.support_market.years.last() {
                    println!("{y:?}");
                }
                heads(&w);
            }
        }
        return;
    }
    let days: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
    let mut year = w.year;
    for _ in 0..days {
        w.advance_day();
        // After each July market (NFL camp) and once a new season's books are open.
        if w.year != year || w.today == w.dates.veterans_report.add(10) {
            year = w.year;
            println!("\n######## {:?} (season {})", w.today, w.year);
            if let Some(y) = w.support_market.years.last() {
                println!("market so far: {y:?}");
            }
            books(&w);
            heads(&w);
        }
    }
    if days > 0 {
        for y in &w.support_market.years {
            println!("market {y:?}");
        }
    }
}
