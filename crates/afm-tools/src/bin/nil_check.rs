//! NIL money by program and era (lane NB9's check, B18): builds a world up
//! to the start of a year and saves it, or plays a saved world on and prints
//! each July's payday and each class's signed NIL offers by level.
//! It reads only what every build has, so the same file can run on main and
//! on a lane to compare them.
//! Usage: nil_check build <seed> <history years> <scale> <start year> <out>
//!        nil_check play <world file> <until year>

use afm_engine::world::data::Division;
use afm_engine::world::{Stage, World, WorldOptions};
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("build") => {
            // Saved after every recorded year as `<out>.part`, so a build cut
            // short (a machine restart) goes on from the last year.
            let n = |k: usize| args[k].parse::<f64>().expect("a number");
            let start_year = n(5) as i32;
            let out = Path::new(&args[6]);
            let part = out.with_extension("part");
            let save = |w: &World, to: &Path| {
                afm_engine::save::write_world(w, to, "nil_check", "nil_check", None).expect("write");
            };
            let t = std::time::Instant::now();
            let w = if part.exists() {
                let mut w = afm_engine::save::read_world(&part).expect("read the part");
                eprintln!("going on from {:?}", w.today);
                while w.year < start_year {
                    w.advance_year();
                    save(&w, &part);
                    eprintln!("year {} at {:.0}s", w.year, t.elapsed().as_secs_f64());
                }
                w
            } else {
                let opts = WorldOptions {
                    seed: n(2) as u32,
                    history_years: n(3) as u32,
                    scale: n(4),
                    start_year,
                    keep_game_log: false,
                    engine_from: Some(2100),
                    ..Default::default()
                };
                let first = start_year - opts.history_years as i32;
                World::new_recorded(opts, None, None, |w, _| {
                    if w.year > first {
                        save(w, &part);
                    }
                    eprintln!("year {} at {:.0}s", w.year, t.elapsed().as_secs_f64());
                    true
                })
                .expect("never stopped")
            };
            eprintln!("built in {:.0}s, {:?}", t.elapsed().as_secs_f64(), w.today);
            save(&w, out);
        }
        Some("play") => {
            let mut w = afm_engine::save::read_world(Path::new(&args[2])).expect("read the world");
            w.options.engine_from = Some(2100);
            let until: i32 = args[3].parse().expect("a year");
            eprintln!("read, {:?}", w.today);
            let mut classes_done = std::collections::HashSet::new();
            while w.today.year() < until || w.today.month() < 8 {
                w.advance_day();
                if w.today.month() == 7 && w.today.day() == 2 {
                    payday(&w);
                }
                // The class signed: its offers, once March comes.
                if w.today.month() == 3 && w.today.day() == 1 && classes_done.insert(w.today.year()) {
                    offers(&w, w.today.year());
                }
            }
        }
        _ => eprintln!("usage: nil_check build <seed> <years> <scale> <start year> <out> | play <file> <until year>"),
    }
}

fn group(w: &World, j: usize) -> Option<&'static str> {
    let p = &w.programs[j];
    match p.division {
        Division::FBS if p.power_four() => Some("P4"),
        Division::FBS => Some("G5"),
        Division::FCS => Some("FCS"),
        _ => None,
    }
}

fn m(x: f64) -> String {
    if x >= 1e6 { format!("${:.2}M", x / 1e6) } else { format!("${:.0}K", x / 1e3) }
}

fn payday(w: &World) {
    let year = w.today.year();
    println!("== payday {year} (July 1) ==");
    for g in ["P4", "G5", "FCS"] {
        let progs: Vec<usize> = (0..w.programs.len()).filter(|&j| group(w, j) == Some(g)).collect();
        let mut totals: Vec<(f64, usize)> = Vec::new();
        let mut deals: Vec<(f64, usize)> = Vec::new();
        for &j in &progs {
            let mut t = 0.0;
            for p in w.people.iter().enumerate().filter(|(_, p)| p.stage == Stage::College && p.college == Some(j)) {
                let x = (p.1.nil_income - p.1.brand as f64).max(0.0);
                t += x;
                deals.push((x, p.0));
            }
            totals.push((t, j));
        }
        totals.sort_by(|a, b| b.0.total_cmp(&a.0));
        deals.sort_by(|a, b| b.0.total_cmp(&a.0));
        let budget: f64 = progs.iter().map(|&j| w.programs[j].collective.budget).sum();
        let school: f64 = progs.iter().map(|&j| afm_engine::world::ncaa::revenue_share(&w.programs[j], year)).sum();
        let paid: f64 = totals.iter().map(|t| t.0).sum();
        let n = deals.len().max(1) as f64;
        let under = deals.iter().filter(|d| d.0 < 10_000.0).count() as f64 / n;
        let million = deals.iter().filter(|d| d.0 >= 1.0e6).count();
        let half = deals.iter().filter(|d| d.0 >= 0.5e6).count();
        let med = totals.get(totals.len() / 2).map_or(0.0, |t| t.0);
        println!(
            "{g}: {} programs, budgets {} (revenue share {}), paid {} ({:.0}%), program median {}, players {} ({:.1}% under $10K, {half} at $500K+, {million} at $1M+)",
            progs.len(),
            m(budget),
            m(school),
            m(paid),
            100.0 * paid / budget.max(1.0),
            m(med),
            deals.len(),
            100.0 * under
        );
        let tops: Vec<String> = totals.iter().take(3).map(|t| format!("{} {}", w.program_name(t.1), m(t.0))).collect();
        println!("  top programs: {}", tops.join(", "));
        let top: Vec<String> = deals
            .iter()
            .take(5)
            .map(|d| {
                let p = &w.people[d.1];
                format!(
                    "{} {}★ {} {}",
                    p.player.position.name(),
                    p.stars,
                    w.program_name(p.college.unwrap_or(0)),
                    m(d.0)
                )
            })
            .collect();
        println!("  top deals: {}", top.join("; "));
    }
}

fn offers(w: &World, year: i32) {
    println!("== signed NIL offers, class of {year} ==");
    for g in ["P4", "G5", "FCS"] {
        let mut by: [Vec<f64>; 6] = Default::default();
        for j in (0..w.programs.len()).filter(|&j| group(w, j) == Some(g)) {
            // Kept by the season before: the class of 2026 signs in the 2025 season.
            for o in w.programs[j].era.offers.iter().filter(|o| o.0 == year - 1) {
                by[o.2.min(5) as usize].push(o.3);
            }
        }
        let line: Vec<String> = (2..=5)
            .rev()
            .map(|s| {
                let v = &mut by[s];
                v.sort_by(|a, b| a.total_cmp(b));
                let mean = v.iter().sum::<f64>() / (v.len().max(1) as f64);
                format!("{s}★ {} mean {} max {}", v.len(), m(mean), m(v.last().copied().unwrap_or(0.0)))
            })
            .collect();
        println!("{g}: {}", line.join("; "));
    }
}
