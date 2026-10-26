//! Squad star mixes and roster sizes (minor bug 17): each group's average
//! college roster by the services' stars, walk-ons and squad size, the
//! signing classes' stars by level, and the scholarship signees from high
//! school the services never rated.
//! Usage: star_mix <world file> [days] | star_mix build <seed> <history years> [scale]
//! (`build` plays the history on the stand-in, as `AFM_ENGINE_FROM=2100`;
//! `AFM_SAVE=path` writes the built world; `days` plays that many days on
//! before counting, to see the fall roster).

use afm_engine::world::data::Division;
use afm_engine::world::{Stage, World, WorldOptions};

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
    let groups: [(&str, fn(&str, Division) -> bool); 5] = [
        ("SEC", |c, _| c == "SEC"),
        ("Big Ten", |c, _| c == "Big Ten"),
        ("Big 12/ACC", |c, _| c == "Big 12" || c == "ACC"),
        ("Group of 5", |c, d| d == Division::FBS && ["American", "AAC", "Mountain West", "Sun Belt", "MAC", "C-USA", "CUSA"].contains(&c)),
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
    println!("{:<12} {:>5} {:>5} {:>5} {:>5} {:>5} {:>8} {:>9} {:>6} {:>6} {:>5}", "group", "5*", "4*", "3*", "2*", "0*", "walk-on", "unr.schol", "squad", "signed", "early");
    for (name, f) in groups {
        let teams: Vec<usize> = (0..w.programs.len()).filter(|&j| f(&w.programs[j].conference, w.programs[j].division)).collect();
        let n = teams.len().max(1) as f64;
        let avg = |k: usize| teams.iter().map(|&j| rows[j][k] as f64).sum::<f64>() / n;
        println!(
            "{:<12} {:>5.1} {:>5.1} {:>5.1} {:>5.1} {:>5.1} {:>8.1} {:>9.1} {:>6.1} {:>6.1} {:>5.1}  ({} teams)",
            name, avg(0), avg(1), avg(2), avg(3), avg(4), avg(5), avg(6), avg(7), avg(8), avg(9), teams.len()
        );
    }
    for name in ["South Carolina", "Alabama", "Georgia", "Vanderbilt", "Michigan", "Purdue", "Memphis", "Montana"] {
        if let Some(j) = w.programs.iter().position(|p| p.real_name == name) {
            let r = rows[j];
            println!(
                "{:<12} {:>5} {:>5} {:>5} {:>5} {:>5} {:>8} {:>9} {:>6} {:>6} {:>5}",
                &name[..name.len().min(12)], r[0], r[1], r[2], r[3], r[4], r[5], r[6], r[7], r[8], r[9]
            );
        }
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
    println!("{:<6} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6} {:>7}", "stars", "FBS", "FCS", "D2", "NAIA", "D3", "JUCO", "none");
    for (k, r) in by.iter().enumerate() {
        println!("{:<6} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6} {:>7}", 5 - k, r[0], r[1], r[2], r[3], r[4], r[5], r[6]);
    }
}
