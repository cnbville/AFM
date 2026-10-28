//! Squad star mixes and roster sizes (minor bug 17): each group's average
//! college roster by the services' stars, walk-ons and squad size, the
//! signing classes' stars by level, and the scholarship signees from high
//! school the services never rated.
//! Usage: star_mix <world file> [days] | star_mix build <seed> <history years> [scale]
//! (`build` plays the history on the stand-in, as `AFM_ENGINE_FROM=2100`;
//! `AFM_SAVE=path` writes the built world; `days` plays that many days on
//! before counting, to see the fall roster).

use afm_engine::world::data::{Division, HISTORY_LAST};
use afm_engine::world::program::Program;
use afm_engine::world::{Stage, World, WorldOptions};
use std::collections::BTreeMap;

/// A group of programs by conference and level.
type Group = (&'static str, fn(&str, Division) -> bool);

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let w: World = if args.get(1).map(String::as_str) == Some("build") {
        let seed = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(2026);
        let years = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(8);
        let scale = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(1.0);
        World::new(WorldOptions {
            seed,
            history_years: years,
            scale,
            engine_from: Some(2100),
            hs_engine_years: Some(0),
            ..WorldOptions::default()
        })
    } else {
        let mut w = afm_engine::save::read_world(std::path::Path::new(&args[1])).expect("read the world");
        for _ in 0..args.get(2).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0) {
            w.advance_day();
        }
        w
    };
    if let Ok(path) = std::env::var("AFM_SAVE") {
        afm_engine::save::write_world(&w, std::path::Path::new(&path), "star_mix", "star_mix", None).expect("write");
    }
    println!("{:?}, year {}", w.today, w.year);
    let stars = |p: &afm_engine::world::Person| if p.player.position.is_specialist() { p.camp_stars } else { p.stars };
    let groups: [Group; 5] = [
        ("SEC", |c, _| c == "SEC"),
        ("Big Ten", |c, _| c == "Big Ten"),
        ("Big 12/ACC", |c, _| c == "Big 12" || c == "ACC"),
        ("Group of 5", |c, d| {
            d == Division::FBS && ["American", "AAC", "Mountain West", "Sun Belt", "MAC", "C-USA", "CUSA"].contains(&c)
        }),
        ("FCS", |_, d| d == Division::FCS),
    ];
    // Per program: [5,4,3,2,0 stars], walk-ons, unrated scholarship players
    // from high school, squad, signed and not enrolled (of them early enrollees).
    let mut rows = vec![[0u32; 10]; w.programs.len()];
    for p in &w.people {
        if p.stage != Stage::College
            && let Some(s) = p.cal.signed
        {
            rows[s.program][8] += 1;
            rows[s.program][9] += s.early_enrollee as u32;
        }
        if p.stage != Stage::College {
            continue;
        }
        let Some(j) = p.college else { continue };
        let r = &mut rows[j];
        let s = stars(p);
        r[match s {
            5 => 0,
            4 => 1,
            3 => 2,
            2 => 3,
            _ => 4,
        }] += 1;
        if p.walk_on {
            r[5] += 1;
        } else if s == 0 && !p.juco && p.transfers == 0 {
            r[6] += 1;
        }
        r[7] += 1;
    }
    println!(
        "{:<12} {:>5} {:>5} {:>5} {:>5} {:>5} {:>8} {:>9} {:>6} {:>6} {:>5}",
        "group", "5*", "4*", "3*", "2*", "0*", "walk-on", "unr.schol", "squad", "signed", "early"
    );
    for (name, f) in groups {
        let teams: Vec<usize> =
            (0..w.programs.len()).filter(|&j| f(&w.programs[j].conference, w.programs[j].division)).collect();
        let n = teams.len().max(1) as f64;
        let avg = |k: usize| teams.iter().map(|&j| rows[j][k] as f64).sum::<f64>() / n;
        println!(
            "{:<12} {:>5.1} {:>5.1} {:>5.1} {:>5.1} {:>5.1} {:>8.1} {:>9.1} {:>6.1} {:>6.1} {:>5.1}  ({} teams)",
            name,
            avg(0),
            avg(1),
            avg(2),
            avg(3),
            avg(4),
            avg(5),
            avg(6),
            avg(7),
            avg(8),
            avg(9),
            teams.len()
        );
    }
    for name in ["South Carolina", "Alabama", "Georgia", "Vanderbilt", "Michigan", "Purdue", "Memphis", "Montana"] {
        if let Some(j) = w.programs.iter().position(|p| p.real_name == name) {
            let r = rows[j];
            println!(
                "{:<12} {:>5} {:>5} {:>5} {:>5} {:>5} {:>8} {:>9} {:>6} {:>6} {:>5}",
                &name[..name.len().min(12)],
                r[0],
                r[1],
                r[2],
                r[3],
                r[4],
                r[5],
                r[6],
                r[7],
                r[8],
                r[9]
            );
        }
    }
    // The last four classes' scholarship signees from high school by the
    // group of their first school, a school's average (the composite's
    // counts are signees, not squads).
    println!("\nscholarship signees of the classes {}-{} a school (no specialists):", w.year - 4, w.year - 1);
    println!("{:<12} {:>5} {:>5} {:>5} {:>5} {:>5}", "group", "5*", "4*", "3*", "2*", "0*");
    for (name, f) in groups {
        let teams = (0..w.programs.len()).filter(|&j| f(&w.programs[j].conference, w.programs[j].division)).count();
        let mut c = [0u32; 6];
        for p in &w.people {
            if p.class_of < w.year - 4
                || p.class_of > w.year - 1
                || p.walk_on
                || p.juco
                || p.player.position.is_specialist()
            {
                continue;
            }
            let Some(&j) = p.path.first() else { continue };
            if f(&w.programs[j].conference, w.programs[j].division) {
                c[stars(p).min(5) as usize] += 1;
            }
        }
        let n = teams.max(1) as f64;
        let a = |k: usize| c[k] as f64 / n;
        println!("{:<12} {:>5.1} {:>5.1} {:>5.1} {:>5.1} {:>5.1}", name, a(5), a(4), a(3), a(2), a(0));
    }
    // Where the four-stars signed, and what they turned down (the offers
    // of their last recruiting stage), by the group of their first school.
    let p4 = |j: usize| ["SEC", "Big Ten", "Big 12", "ACC"].contains(&w.programs[j].conference.as_str());
    println!(
        "\nfour-star signees of the classes {}-{}: n, with a Power 4 offer, offers, late (February) deciders, summer commits:",
        w.year - 4,
        w.year - 1
    );
    for (name, f) in groups {
        let (mut n, mut with, mut offers, mut late, mut summer) = (0u32, 0u32, 0usize, 0u32, 0u32);
        for p in &w.people {
            if p.class_of < w.year - 4
                || p.class_of > w.year - 1
                || p.walk_on
                || p.juco
                || p.stars != 4
                || p.player.position.is_specialist()
            {
                continue;
            }
            let Some(&j) = p.path.first() else { continue };
            if !f(&w.programs[j].conference, w.programs[j].division) {
                continue;
            }
            n += 1;
            with += p.cal.offers.iter().any(|&o| o != j && p4(o)) as u32;
            offers += p.cal.offers.len();
            late += matches!(p.cal.commit_timing, Some(afm_engine::world::CommitTiming::Late)) as u32;
            summer += matches!(p.cal.commit_timing, Some(afm_engine::world::CommitTiming::Summer(_))) as u32;
        }
        println!(
            "{:<12} {:>5} {:>5} {:>6.1} {:>5} {:>5}",
            name,
            n,
            with,
            offers as f64 / n.max(1) as f64,
            late,
            summer
        );
    }
    // Each school's newcomers and leavers a season over the last four
    // seasons, from the coaches they played under: high school signees on
    // scholarship and walk-ons, junior college players and transfers in,
    // and transfers out (the world drops those who left football, so the
    // others who left can't be counted here).
    println!("\nnewcomers and leavers a school a season, {}-{}:", w.year - 4, w.year - 1);
    println!("{:<12} {:>6} {:>6} {:>6} {:>6} {:>6}", "group", "hs sch", "hs w-o", "juco", "tr in", "tr out");
    for (name, f) in groups {
        let teams = (0..w.programs.len()).filter(|&j| f(&w.programs[j].conference, w.programs[j].division)).count();
        let mut c = [0u32; 5];
        for p in &w.people {
            let mut seen: Vec<(usize, i32, i32)> = Vec::new();
            for &(y, j, _, _) in &p.college_staff {
                match seen.iter_mut().find(|s| s.0 == j) {
                    Some(s) => s.2 = s.2.max(y),
                    None => seen.push((j, y, y)),
                }
            }
            for (k, &(j, first, last)) in seen.iter().enumerate() {
                if !f(&w.programs[j].conference, w.programs[j].division) {
                    continue;
                }
                if (w.year - 4..w.year).contains(&first) {
                    let from_juco = k > 0 && w.programs[seen[k - 1].0].division == Division::JUCO;
                    c[if from_juco || k == 0 && p.juco {
                        2
                    } else if k > 0 {
                        3
                    } else if p.walk_on && p.path.len() == 1 {
                        1
                    } else {
                        0
                    }] += 1;
                }
                if (w.year - 5..w.year - 1).contains(&last) && k + 1 < seen.len() {
                    c[4] += 1;
                }
            }
        }
        let n = 4.0 * teams.max(1) as f64;
        let a = |k: usize| c[k] as f64 / n;
        println!("{:<12} {:>6.1} {:>6.1} {:>6.1} {:>6.1} {:>6.1}", name, a(0), a(1), a(2), a(3), a(4));
    }
    // The last four signing classes: where each star tier went.
    println!("\nhigh school signees of the classes {}-{} by stars and level:", w.year - 4, w.year - 1);
    let levels = [Division::FBS, Division::FCS, Division::D2, Division::NAIA, Division::D3, Division::JUCO];
    let mut by = [[0u32; 7]; 6];
    for p in &w.people {
        if p.class_of < w.year - 4 || p.class_of > w.year - 1 || p.player.position.is_specialist() {
            continue;
        }
        let s = p.stars.min(5) as usize;
        let col = p.path.first().map_or(6, |&j| levels.iter().position(|&d| d == w.programs[j].division).unwrap_or(6));
        by[5 - s][col] += 1;
    }
    println!(
        "{:<6} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6} {:>7}",
        "stars", "FBS", "FCS", "D2", "NAIA", "D3", "JUCO", "none"
    );
    for (k, r) in by.iter().enumerate() {
        println!("{:<6} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6} {:>7}", 5 - k, r[0], r[1], r[2], r[3], r[4], r[5], r[6]);
    }
    // Dynasties and parity (A6) against real: the most titles in any 15
    // years (real 6, Alabama 2009-2023), the champions of the last 15 years
    // (real 8), and titles by today's group (real 1996-2025: SEC 15).
    let fbs_in = |p: &Program, y: i32| {
        (if y <= HISTORY_LAST { p.real_status(y).map_or(p.division, |s| s.0) } else { p.division }) == Division::FBS
    };
    let champs: BTreeMap<i32, (usize, u32)> = w
        .programs
        .iter()
        .flat_map(|p| {
            p.seasons.iter().filter(|s| s.national_title && fbs_in(p, s.year)).map(move |s| (s.year, (p.id, s.losses)))
        })
        .collect();
    let years: Vec<i32> = champs.keys().copied().collect();
    let mut most = (0usize, 0usize, 0i32);
    for (k, &y0) in years.iter().enumerate() {
        let window: Vec<usize> = years[k..].iter().filter(|&&y| y < y0 + 15).map(|y| champs[y].0).collect();
        for &j in &window {
            let n = window.iter().filter(|&&x| x == j).count();
            if n > most.0 {
                most = (n, j, y0);
            }
        }
    }
    let last15: Vec<(usize, u32)> = champs.values().rev().take(15).copied().collect();
    let mut distinct: Vec<usize> = last15.iter().map(|x| x.0).collect();
    distinct.sort();
    distinct.dedup();
    println!(
        "\ntitles {}-{}: most in any 15 years {} ({} from {}; real 6); champions in the last 15 years {} (real 8); unbeaten {} of {}",
        years.first().copied().unwrap_or(0),
        years.last().copied().unwrap_or(0),
        most.0,
        w.programs[most.1].real_name,
        most.2,
        distinct.len(),
        last15.iter().filter(|x| x.1 == 0).count(),
        last15.len()
    );
    let by_group: Vec<String> = groups
        .iter()
        .map(|(name, f)| {
            let n = champs.values().filter(|c| f(&w.programs[c.0].conference, w.programs[c.0].division)).count();
            format!("{name} {n}")
        })
        .collect();
    let mut by_team: BTreeMap<&str, usize> = BTreeMap::new();
    for c in champs.values() {
        *by_team.entry(w.programs[c.0].real_name.as_str()).or_default() += 1;
    }
    let mut by_team: Vec<(&str, usize)> = by_team.into_iter().collect();
    by_team.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    println!("by group: {}", by_group.join(", "));
    println!("most titles: {}", by_team.iter().take(8).map(|(n, c)| format!("{n} {c}")).collect::<Vec<_>>().join(", "));
}
