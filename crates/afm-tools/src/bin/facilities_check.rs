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
//!   facilities_check rooms <seed> <history years> <scale> <seasons on>
//!     Capital lane C2: builds the world the same way, prints the
//!     facilities table and the rooms at the handover, then plays the
//!     seasons on (every game on the stand-in) and prints the same each
//!     July: the FBS spread, each room's mean level and read by tier, and
//!     the year's new buildings and renovations; each level's facilities,
//!     new buildings and boosters' money (LEVELS), and lane C3's drives by
//!     stage and the hottest fevers (DRIVES, FEVER).
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

/// Capital lane C2: each room's mean level and read at FBS by tier, and
/// last July's new buildings and renovations (the world stands in January,
/// so last July is the season just played).
fn rooms_table(w: &World) {
    use afm_engine::world::capital::rooms::{ROOMS, Room};
    let year = w.year;
    let fbs: Vec<usize> = (0..w.programs.len())
        .filter(|&j| w.programs[j].division == Division::FBS && w.programs[j].plays(year))
        .collect();
    for (label, power) in [("P4", true), ("G5", false)] {
        let v: Vec<usize> = fbs.iter().copied().filter(|&j| w.programs[j].power_conference(year) == power).collect();
        let n = v.len().max(1) as f64;
        let mut line = format!("{label} rooms (level/read/share with one):");
        for r in Room::ALL {
            let k = r as usize;
            let lv = v.iter().map(|&j| w.programs[j].campus.level[k] as f64).sum::<f64>() / n;
            let rd = v.iter().map(|&j| w.programs[j].campus.read[k] as f64).sum::<f64>() / n;
            let has = v.iter().filter(|&&j| w.programs[j].campus.level[k] > 0.0).count() as f64 / n;
            line +=
                &format!(" {}={lv:.2}/{rd:.2}/{:.0}%", r.words().split_whitespace().next().unwrap_or(""), 100.0 * has);
        }
        println!("{line}");
    }
    let _ = ROOMS;
    let started = |reno: bool| {
        w.finance
            .projects
            .iter()
            .filter(|x| {
                x.started == year - 1 && x.kind == ProjectKind::Facility && x.name.ends_with(" renovation") == reno
            })
            .filter(|x| matches!(x.owner, Owner::College(j) if w.programs[j].division == Division::FBS))
            .count()
    };
    let temp = fbs.iter().filter(|&&j| w.programs[j].campus.in_temporary_space()).count();
    println!(
        "FBS projects started in July {}: {} new buildings, {} renovations; {temp} programs in temporary space",
        year - 1,
        started(false),
        started(true)
    );
}

/// Capital lane C3's world check: each level's facilities mean, the new
/// buildings started last July and since the handover, and the boosters'
/// money (the same lines on the code before the lane, for the baseline).
fn levels_table(w: &World) {
    let year = w.year;
    let start = w.options.start_year;
    let mut line = String::from("LEVELS");
    for (label, d, power) in [
        ("P4", Division::FBS, Some(true)),
        ("G5", Division::FBS, Some(false)),
        ("FCS", Division::FCS, None),
        ("D2", Division::D2, None),
        ("D3", Division::D3, None),
        ("NAIA", Division::NAIA, None),
    ] {
        let v: Vec<usize> = (0..w.programs.len())
            .filter(|&j| {
                let p = &w.programs[j];
                p.division == d && p.plays(year) && power.is_none_or(|x| p.power_conference(year) == x)
            })
            .collect();
        let n = v.len().max(1) as f64;
        let fac = v.iter().map(|&j| w.programs[j].facilities).sum::<f64>() / n;
        let nil = v.iter().map(|&j| w.programs[j].collective.boosters()).sum::<f64>() / n / 1.0e6;
        let built = |from: i32| {
            w.finance
                .projects
                .iter()
                .filter(|x| {
                    x.started >= from
                        && x.started < year
                        && x.kind == ProjectKind::Facility
                        && !x.name.ends_with(" renovation")
                })
                .filter(|x| matches!(x.owner, Owner::College(j) if v.contains(&j)))
                .count()
        };
        line += &format!(
            " | {label} n={} fac={fac:.3} new_last={} new_since={} nil={nil:.2}M",
            v.len(),
            built(year - 1),
            built(start)
        );
    }
    println!("{line}");
}

/// Capital lane C3: the drives by stage and what they raised.
fn drives_table(w: &World) {
    use afm_engine::world::capital::drive::{self, Stage};
    let year = w.year;
    let cap = &w.capital;
    let by = |s: Stage| cap.drives.iter().filter(|d| d.stage == s && !d.shelved).count();
    let started = cap.drives.iter().filter(|d| d.opened.is_some_and(|o| o.year() == year - 1)).count();
    let ground = cap.drives.iter().filter(|d| d.ground == Some(year - 1)).count();
    let shelved = cap.drives.iter().filter(|d| d.shelved && d.closed.is_some_and(|c| c.year() >= year - 1)).count();
    let closed: Vec<_> =
        cap.drives.iter().filter(|d| d.closed.is_some_and(|c| c.year() >= year - 1) && !d.shelved).collect();
    let hit = closed.iter().filter(|d| d.hit == Some(true)).count();
    let pushed = cap.drives.iter().filter(|d| d.borrowed > 0.0 && d.ground == Some(year - 1)).count();
    let over = cap.drives.iter().filter(|d| d.ground == Some(year - 1) && d.overrun.is_some()).count();
    let early = cap.drives.iter().filter(|d| d.early && d.public_on.is_some_and(|o| o.year() >= year - 1)).count();
    let share: Vec<f64> = closed.iter().map(|d| drive::raised(cap, d) / d.goal.max(1.0)).collect();
    let mean = share.iter().sum::<f64>() / share.len().max(1) as f64;
    let sizes = |s: drive::Size| cap.drives.iter().filter(|d| d.ground == Some(year - 1) && d.size == s).count();
    let fev = afm_engine::world::capital::fever::all(w);
    let mut top: Vec<(f64, usize)> =
        fev.iter().copied().enumerate().map(|(j, f)| (f, j)).filter(|x| x.0 > 0.0).collect();
    top.sort_by(|a, b| b.0.total_cmp(&a.0));
    let tops: Vec<String> = top.iter().take(5).map(|&(f, j)| format!("{} {f:.2}", w.programs[j].real_name)).collect();
    let catches = cap.catches.len();
    println!(
        "DRIVES study={} quiet={} public={} started_last={started} ground_last={ground} (full {} mid {} small {}) shelved_last={shelved} closed_last={} hit={hit} mean_raised={mean:.2} pushed={pushed} overruns={over} early={early} pledges={} catches={catches} moments={}",
        by(Stage::Study),
        by(Stage::Quiet),
        by(Stage::Public),
        sizes(drive::Size::Full),
        sizes(drive::Size::Mid),
        sizes(drive::Size::Small),
        closed.len(),
        cap.pledges.len(),
        cap.moments.len()
    );
    println!("FEVER top: {}", tops.join(", "));
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
        Some("rooms") => {
            let seed: u32 = a[2].parse().unwrap();
            let years: u32 = a[3].parse().unwrap();
            let scale: f64 = a[4].parse().unwrap();
            let on: u32 = a[5].parse().unwrap();
            let opts = WorldOptions {
                seed,
                history_years: years,
                scale,
                engine_from: Some(2100),
                hs_engine_years: Some(0),
                ..Default::default()
            };
            let t = Instant::now();
            let mut w = World::new_watched(opts, |w, s| {
                eprintln!("season {} done in {:.0} s ({:.0} s in all)", w.year - 1, s, t.elapsed().as_secs_f64())
            });
            fac_table(&w);
            rooms_table(&w);
            levels_table(&w);
            for _ in 0..on {
                w.advance_year();
                eprintln!("season {} done ({:.0} s in all)", w.year - 1, t.elapsed().as_secs_f64());
                println!("== after season {} ==", w.year - 1);
                fac_table(&w);
                rooms_table(&w);
                levels_table(&w);
                drives_table(&w);
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
