//! Facilities check (facilities lane, Cain 2026-11-08): where every FBS
//! program's facilities number stands when the world hands over, and what
//! a facility project buys in recruiting from the day ground breaks.
//!
//! Usage:
//!   facilities_check table <seed> <history years> <scale> [world out]
//!     builds a world (every game on the stand-in) and prints the FBS
//!     facilities spread, the Big Ten in order and one ROW line per FBS
//!     program (name, conference, facilities, grade, prestige).
//!   facilities_check arm <world> <A|B|C> <years> <signees out>
//!     plays the world on from its file. A: as it is. B: the treated
//!     programs' facilities set to the top (1.0) and read in full, as if
//!     their buildings had opened (the 2026-11-07 check). C: the treated
//!     programs break ground on a top building that opens two years on,
//!     facilities unchanged until it opens (item 2: the pull from the
//!     groundbreaking). Treated: the three lowest-facilities programs in
//!     each power conference plus Northwestern and the lowest in the SEC,
//!     ACC and Big 12 (16). Each class's signees go to the out file, read
//!     from the signed letter (`cal.signed`), then the college.
//!   facilities_check pool <A file> <B or C file> <treated list>
//!     is done in Python (`pooled.py` in the project folder's
//!     afm/facilities/tools).

use afm_engine::world::data::Division;
use afm_engine::world::finance::{Owner, Project, ProjectKind};
use afm_engine::world::program::GradeCat;
use afm_engine::world::{Stage, World, WorldOptions};
use std::io::Write;
use std::path::Path;
use std::time::Instant;

fn fac_table(w: &World) {
    let year = w.year;
    let mut fbs: Vec<usize> = (0..w.programs.len())
        .filter(|&j| w.programs[j].division == Division::FBS && w.programs[j].plays(year))
        .collect();
    fbs.sort_by(|&a, &b| w.programs[b].facilities.total_cmp(&w.programs[a].facilities));
    let p4: Vec<usize> = fbs.iter().copied().filter(|&j| w.programs[j].power_conference(year)).collect();
    let g5: Vec<usize> = fbs.iter().copied().filter(|&j| !w.programs[j].power_conference(year)).collect();
    let q = |v: &[usize], x: f64| w.programs[v[((v.len() - 1) as f64 * x).round() as usize]].facilities;
    let mean_sd = |v: &[usize]| {
        let m = v.iter().map(|&j| w.programs[j].facilities).sum::<f64>() / v.len() as f64;
        let s = (v.iter().map(|&j| (w.programs[j].facilities - m).powi(2)).sum::<f64>() / v.len() as f64).sqrt();
        (m, s)
    };
    for (label, v) in [("FBS", &fbs), ("P4", &p4), ("G5", &g5)] {
        let (m, s) = mean_sd(v);
        println!(
            "{label} {} programs in {year}: facilities mean {m:.3} sd {s:.3}; top {:.2}, top tenth {:.2}, median {:.2}, bottom tenth {:.2}, bottom {:.2}",
            v.len(),
            q(v, 0.0),
            q(v, 0.1),
            q(v, 0.5),
            q(v, 0.9),
            q(v, 1.0)
        );
    }
    let open = |j: usize| {
        w.finance
            .projects
            .iter()
            .filter(|x| x.owner == Owner::College(j) && !x.done)
            .map(|x| x.name.clone())
            .collect::<Vec<_>>()
    };
    let mut b10 = 0;
    for (rank, &j) in fbs.iter().enumerate() {
        let p = &w.programs[j];
        if p.conference == "Big Ten" {
            b10 += 1;
            println!(
                "  Big Ten #{b10:>2} (FBS #{:>3}) {:<16} fac {:.3} grade {:.1} prestige {:.3} building {:?}",
                rank + 1,
                p.real_name,
                p.facilities,
                p.grades[GradeCat::Facilities as usize],
                p.prestige,
                open(j)
            );
        }
    }
    for (rank, &j) in fbs.iter().enumerate() {
        let p = &w.programs[j];
        println!(
            "ROW\t{}\t{}\t{}\t{:.4}\t{:.1}\t{:.4}\t{}",
            rank + 1,
            p.real_name,
            p.conference,
            p.facilities,
            p.grades[GradeCat::Facilities as usize],
            p.prestige,
            open(j).join("; ")
        );
    }
}

/// The three lowest-facilities programs in each power conference, then
/// Northwestern and the lowest left in the SEC, ACC and Big 12 (16 in all).
fn treated(w: &World) -> Vec<usize> {
    let year = w.year;
    let mut out = Vec::new();
    let low = |conf: &str, skip: &[usize]| {
        let mut v: Vec<usize> = (0..w.programs.len())
            .filter(|&j| w.programs[j].conference == conf && w.programs[j].plays(year) && !skip.contains(&j))
            .collect();
        v.sort_by(|&a, &b| w.programs[a].facilities.total_cmp(&w.programs[b].facilities));
        v
    };
    if let Some(nw) = w.programs.iter().position(|p| p.real_name == "Northwestern") {
        out.push(nw);
    }
    for conf in ["SEC", "ACC", "Big 12"] {
        let v = low(conf, &out);
        out.extend(v.into_iter().take(1));
    }
    for conf in ["SEC", "ACC", "Big 12", "Big Ten"] {
        let v = low(conf, &out);
        out.extend(v.into_iter().take(3));
    }
    out
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    match a.get(1).map(String::as_str) {
        Some("table") => {
            let seed: u32 = a[2].parse().unwrap();
            let years: u32 = a[3].parse().unwrap();
            let scale: f64 = a[4].parse().unwrap();
            let opts = WorldOptions {
                seed,
                history_years: years,
                scale,
                engine_from: Some(2100),
                hs_engine_years: Some(0),
                ..Default::default()
            };
            let t = Instant::now();
            let w = World::new_watched(opts, |w, s| {
                eprintln!("season {} done in {:.0} s ({:.0} s in all)", w.year - 1, s, t.elapsed().as_secs_f64())
            });
            fac_table(&w);
            if let Some(out) = a.get(5) {
                afm_engine::save::write_world(&w, Path::new(out), "scratch", "facilities_check", None).expect("write");
                eprintln!("written in {:.0} s in all", t.elapsed().as_secs_f64());
            }
        }
        Some("arm") => {
            let t = Instant::now();
            let mut w = afm_engine::save::read_world(Path::new(&a[2])).expect("read");
            w.options.engine_from = Some(2100);
            let arm = a[3].as_str();
            let years: u32 = a[4].parse().unwrap();
            eprintln!("read in {:.0} s, {:?}", t.elapsed().as_secs_f64(), w.today);
            let tr = treated(&w);
            for &j in &tr {
                match arm {
                    "B" => {
                        let p = &mut w.programs[j];
                        let before = p.facilities;
                        p.facilities = 1.0;
                        p.grades[GradeCat::Facilities as usize] = 99.0;
                        // prestige_of is linear in facilities (0.12 a unit).
                        let old = p.prestige;
                        p.prestige = (old + 0.12 * (1.0 - before)).min(1.0);
                        p.prestige_rel *= p.prestige / old.max(1e-6);
                    }
                    "C" => {
                        let year = w.year;
                        let step = (1.0 - w.programs[j].facilities).max(0.0) as f32;
                        w.finance.projects.retain(|x| !(x.owner == Owner::College(j) && !x.done));
                        w.finance.projects.push(Project {
                            owner: Owner::College(j),
                            kind: ProjectKind::Facility,
                            name: "football building".into(),
                            cost: 200.0e6,
                            public: 0.0,
                            pledged: 120.0e6,
                            started: year,
                            opens: year + 2,
                            capacity: 0,
                            step,
                            done: false,
                            asked: false,
                        });
                        afm_engine::world::staff::facilities_grade_now(&mut w, j);
                    }
                    _ => {}
                }
            }
            for &j in &tr {
                let p = &w.programs[j];
                println!(
                    "TREATED {} ({}) fac {:.3} grade {:.1} prestige {:.3} rel {:.3}",
                    p.real_name,
                    p.conference,
                    p.facilities,
                    p.grades[GradeCat::Facilities as usize],
                    p.prestige,
                    p.prestige_rel
                );
            }
            let mut out = std::fs::File::create(&a[5]).unwrap();
            for _ in 0..years {
                let year = w.year;
                let t = Instant::now();
                w.advance_year();
                eprintln!("year {year} played in {:.0} s", t.elapsed().as_secs_f64());
                for p in &w.people {
                    let Some((j, walk)) =
                        p.cal.signed.as_ref().map(|s| (s.program, s.walk_on)).or(p.college.map(|c| (c, p.walk_on)))
                    else {
                        continue;
                    };
                    if p.class_of != year + 1 || !matches!(p.stage, Stage::College | Stage::HighSchool) {
                        continue;
                    }
                    let prog = &w.programs[j];
                    writeln!(
                        out,
                        "{}\t{}\t{}\t{}\t{:?}\t{}\t{}\t{:.2}\t{:.2}\t{}\t{}",
                        year + 1,
                        p.player.id,
                        prog.real_name,
                        prog.conference,
                        prog.division,
                        p.stars,
                        p.player.position,
                        p.player.overall(),
                        afm_engine::debug::potential(&p.player),
                        p.home_state,
                        walk as u8
                    )
                    .unwrap();
                }
                for &j in &tr {
                    let p = &w.programs[j];
                    println!(
                        "AFTER {} {} fac {:.3} grade {:.1} prestige {:.3} rel {:.3} class {:?}",
                        year,
                        p.real_name,
                        p.facilities,
                        p.grades[GradeCat::Facilities as usize],
                        p.prestige,
                        p.prestige_rel,
                        p.class_scores.last()
                    );
                }
            }
        }
        _ => eprintln!(
            "usage: facilities_check table <seed> <history years> <scale> [world out] | arm <world> <A|B|C> <years> <out>"
        ),
    }
}
