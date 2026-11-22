//! Staff urgency measured on one small world (`docs/design/staff-urgency.md`,
//! section 5: "the build measures the total and who leaves, by role and
//! level, before and after"). Builds a world with games on the stand-in and
//! prints the portal by recorded year: entrants a program by level, FBS
//! entrants by role and by why, the men told they aren't in the plans, the
//! trims before camp, rosters against the limit, and the FBS staffs' calls
//! on the world's last day.
//!
//! Run it twice on one seed: `AFM_URGENCY=off` is the world before (the AI's
//! staffs never move on from anyone), the default the world after.
//! Usage: urgency_check [scale] [history_years] [seed] (defaults 0.25, 6, 311)

use std::collections::BTreeMap;

use afm_engine::player::PlayingTime;
use afm_engine::world::data::Division;
use afm_engine::world::{Stage, World, WorldOptions};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let scale: f64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.25);
    let years: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(6);
    let seed: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(311);
    let arm = std::env::var("AFM_URGENCY").unwrap_or_else(|_| "on".into());
    let t = std::time::Instant::now();
    let w =
        World::new(WorldOptions { seed, scale, history_years: years, engine_from: Some(2100), ..Default::default() });
    println!(
        "world: scale {scale}, {years} years of history, seed {seed}, staff urgency {arm}, built in {:.0} s",
        t.elapsed().as_secs_f64()
    );
    portal(&w);
    rosters(&w);
    calls(&w);
}

fn div_name(d: Division) -> &'static str {
    match d {
        Division::FBS => "FBS",
        Division::FCS => "FCS",
        Division::D2 => "D2",
        Division::D3 => "D3",
        Division::NAIA => "NAIA",
        Division::JUCO => "JUCO",
    }
}

fn role_name(r: Option<PlayingTime>) -> &'static str {
    match r {
        Some(PlayingTime::Starter) => "starter",
        Some(PlayingTime::Rotation) => "rotation",
        Some(PlayingTime::Backup) => "backup",
        _ => "none",
    }
}

fn programs(w: &World, d: Division, year: i32) -> usize {
    w.programs.iter().filter(|p| p.division == d && p.plays(year)).count().max(1)
}

fn portal(w: &World) {
    let log = &w.recruiting_log.portal;
    let mut years: Vec<i32> = log.iter().map(|r| r.year).collect();
    years.sort();
    years.dedup();
    println!("\n## Entrants a program by cycle (the season just played)");
    println!("cycle   FBS    FCS     D2     D3   NAIA   FBS told  FCS told  D2 told  FBS landed%");
    for &y in &years {
        let at = |d: Division| log.iter().filter(|r| r.year == y && r.from == d).count();
        let told =
            |d: Division| log.iter().filter(|r| r.year == y && r.from == d && r.why == "not in the plans").count();
        let fbs = log.iter().filter(|r| r.year == y && r.from == Division::FBS).count();
        let fbs_landed = log.iter().filter(|r| r.year == y && r.from == Division::FBS && r.to.is_some()).count();
        let per = |d: Division| at(d) as f64 / programs(w, d, y + 1) as f64;
        println!(
            "{y}  {:>5.1}  {:>5.1}  {:>5.1}  {:>5.1}  {:>5.1}  {:>8.1}  {:>8.1}  {:>7.1}  {:>10.0}",
            per(Division::FBS),
            per(Division::FCS),
            per(Division::D2),
            per(Division::D3),
            per(Division::NAIA),
            told(Division::FBS) as f64 / programs(w, Division::FBS, y + 1) as f64,
            told(Division::FCS) as f64 / programs(w, Division::FCS, y + 1) as f64,
            told(Division::D2) as f64 / programs(w, Division::D2, y + 1) as f64,
            100.0 * fbs_landed as f64 / fbs.max(1) as f64,
        );
    }
    println!("\n## FBS entrants by role (share of the cycle's FBS entrants)");
    println!("cycle   starter  rotation  backup  none");
    for &y in &years {
        let mut by: BTreeMap<&str, usize> = BTreeMap::new();
        let mut n = 0;
        for r in log.iter().filter(|r| r.year == y && r.from == Division::FBS) {
            *by.entry(role_name(r.role)).or_default() += 1;
            n += 1;
        }
        let s = |k: &str| 100.0 * *by.get(k).unwrap_or(&0) as f64 / n.max(1) as f64;
        println!("{y}  {:>7.1}  {:>8.1}  {:>6.1}  {:>4.1}", s("starter"), s("rotation"), s("backup"), s("none"));
    }
    println!("\n## FBS entrants by why, all cycles (share)");
    let mut by: BTreeMap<String, usize> = BTreeMap::new();
    let fbs: Vec<_> = log.iter().filter(|r| r.from == Division::FBS).collect();
    for r in &fbs {
        *by.entry(r.why.to_string()).or_default() += 1;
    }
    for (k, v) in by {
        println!("{k:<22} {:>5.1}%", 100.0 * v as f64 / fbs.len().max(1) as f64);
    }
    println!("\n## Told men by role and level, all cycles");
    for d in [Division::FBS, Division::FCS, Division::D2] {
        let t: Vec<_> = log.iter().filter(|r| r.from == d && r.why == "not in the plans").collect();
        let landed = t.iter().filter(|r| r.to.is_some()).count();
        let down = t.iter().filter(|r| r.to.is_some_and(|x| x as usize > d as usize)).count();
        let mut by: BTreeMap<&str, usize> = BTreeMap::new();
        for r in &t {
            *by.entry(role_name(r.role)).or_default() += 1;
        }
        println!(
            "{}: {} told, landed {:.0}%, a level down {:.0}%, by role {:?}",
            div_name(d),
            t.len(),
            100.0 * landed as f64 / t.len().max(1) as f64,
            100.0 * down as f64 / t.len().max(1) as f64,
            by
        );
    }
}

fn rosters(w: &World) {
    let trims = w
        .people
        .iter()
        .flat_map(|p| p.notes.iter())
        .chain(w.alumni.iter().flat_map(|a| a.notes.iter()))
        .filter(|n| n.contains("to reach the roster limit before camp"))
        .count();
    println!("\n## Rosters");
    println!("cut to reach the limit before camp, all years: {trims}");
    for d in [Division::FBS, Division::FCS] {
        let mut sizes: Vec<usize> = w
            .programs
            .iter()
            .filter(|p| p.division == d && p.plays(w.year))
            .map(|p| p.roster.iter().filter(|&&i| w.people[i].stage == Stage::College).count())
            .collect();
        sizes.sort();
        if sizes.is_empty() {
            continue;
        }
        let q = |x: f64| sizes[((sizes.len() - 1) as f64 * x).round() as usize];
        println!(
            "{} rosters on {}: min {} p10 {} median {} p90 {} max {}",
            div_name(d),
            w.today,
            q(0.0),
            q(0.1),
            q(0.5),
            q(0.9),
            q(1.0)
        );
    }
}

fn calls(w: &World) {
    use afm_engine::world::folk::roster_call;
    println!("\n## The staffs' calls on {} (a program, mean)", w.today);
    let table = afm_engine::world::growth::table(w);
    let marks = roster_call::LevelMarks::of(w);
    for d in [Division::FBS, Division::FCS] {
        let progs: Vec<usize> =
            (0..w.programs.len()).filter(|&j| w.programs[j].division == d && w.programs[j].plays(w.year)).collect();
        let mut t = [0usize; 3];
        for &j in &progs {
            let x = roster_call::tally(w, j, &table, &marks);
            for k in 0..3 {
                t[k] += x[k];
            }
        }
        let n = progs.len().max(1) as f64;
        println!(
            "{}: move on {:.1}, bring a man in over {:.1}, keep {:.1}",
            div_name(d),
            t[0] as f64 / n,
            t[1] as f64 / n,
            t[2] as f64 / n
        );
    }
    // The calls by reason, and the market's spread, as the window's trace
    // prints them.
    let all: Vec<usize> = (0..w.programs.len())
        .filter(|&j| {
            matches!(w.programs[j].division, Division::FBS | Division::FCS | Division::D2)
                && w.programs[j].plays(w.year)
        })
        .collect();
    roster_call::trace(w, &all, &table, &marks);
}
