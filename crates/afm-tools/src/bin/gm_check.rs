//! NFL general managers' time in the job (realism audit item 12): builds a
//! world on the stand-in, then plays it on and counts every GM change by
//! why, the head coaches fired beside them, and the current GMs' tenure
//! each summer. Usage: gm_check [scale] [history_years] [seed] [seasons]

use afm_engine::world::coach::{Employer, Role};
use afm_engine::world::{World, WorldOptions};
use std::collections::BTreeMap;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let scale: f64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.1);
    let years: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(20);
    let seed: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(331);
    let seasons: i32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(8);
    let t0 = std::time::Instant::now();
    let mut w =
        World::new(WorldOptions { seed, scale, history_years: years, engine_from: Some(2100), ..Default::default() });
    let first = w.year;
    println!(
        "world: scale {scale}, {years} years of history, seed {seed}, built in {:.0} s",
        t0.elapsed().as_secs_f64()
    );
    let total: u32 = w.clubs.iter().map(|c| c.office.gm_changes).sum();
    println!("history: {total} GM changes in {years} years ({:.1} a year)", total as f64 / years as f64);
    tenure_line(&w, "start");
    let mut gm: Vec<String> = w.clubs.iter().map(|c| c.office.gm.name.clone()).collect();
    let mut why: BTreeMap<i32, BTreeMap<String, u32>> = BTreeMap::new();
    let mut done: Vec<i32> = Vec::new();
    let mut since: Vec<i32> = w.clubs.iter().map(|c| c.office.gm.since).collect();
    let end = first + seasons;
    let mut summer = first;
    while w.year < end {
        w.advance_day();
        // An expansion club joins with its first GM.
        for c in w.clubs.iter().skip(gm.len()) {
            gm.push(c.office.gm.name.clone());
            since.push(c.office.gm.since);
        }
        for t in 0..w.clubs.len() {
            let o = &w.clubs[t].office;
            if o.gm.name != gm[t] {
                let team = w.nfl[t].name;
                let text = w
                    .calendar
                    .all()
                    .filter(|e| e.text.contains("replace general manager") && e.text.contains(team))
                    .last()
                    .map(|e| e.text.clone())
                    .unwrap_or_default();
                let r = text.rsplit('(').next().unwrap_or("?").trim_end_matches(").").to_string();
                *why.entry(w.year).or_default().entry(r).or_default() += 1;
                done.push(w.year - since[t]);
                gm[t] = o.gm.name.clone();
                since[t] = o.gm.since;
            }
        }
        if w.today.month() == 7 && w.today.day() == 1 && w.year > summer {
            summer = w.year;
            tenure_line(&w, &format!("July {}", w.year));
        }
    }
    println!("GM changes by season and why:");
    let mut all: BTreeMap<String, u32> = BTreeMap::new();
    for (y, m) in &why {
        let n: u32 = m.values().sum();
        println!("  {y}: {n} {m:?}");
        for (k, v) in m {
            *all.entry(k.clone()).or_default() += v;
        }
    }
    let n = why.len().max(1) as f64;
    let sum: u32 = all.values().sum();
    println!(
        "  a year: {:.1} ({})",
        sum as f64 / n,
        all.iter().map(|(k, v)| format!("{k} {:.1}", *v as f64 / n)).collect::<Vec<_>>().join(", ")
    );
    done.sort();
    if !done.is_empty() {
        let mean = done.iter().sum::<i32>() as f64 / done.len() as f64;
        println!("tenure of the GMs who left: median {}, mean {mean:.1} (n {})", done[done.len() / 2], done.len());
    }
    // Head coaches fired in the seasons played on.
    let mut hc = 0;
    for c in &w.coaches {
        for s in &c.history {
            if matches!(s.employer, Employer::Nfl(_))
                && s.role == Role::HeadCoach
                && s.ended.starts_with("fired")
                && s.to >= first
            {
                hc += 1;
            }
        }
    }
    println!("NFL head coaches fired: {hc} in {} seasons ({:.1} a year)", why.len(), hc as f64 / n);
    let trust: Vec<f64> = w.clubs.iter().map(|c| c.office.owner.trust).collect();
    let mt = trust.iter().sum::<f64>() / trust.len() as f64;
    println!("owner trust now: mean {mt:.2}, under 0.45: {}", trust.iter().filter(|&&x| x < 0.45).count());
    println!("done in {:.0} s", t0.elapsed().as_secs_f64());
}

fn tenure_line(w: &World, when: &str) {
    let year = w.year;
    let mut t: Vec<i32> =
        w.clubs.iter().filter(|c| !c.office.gm.name.is_empty()).map(|c| year - c.office.gm.since).collect();
    t.sort();
    if t.is_empty() {
        return;
    }
    let mean = t.iter().sum::<i32>() as f64 / t.len() as f64;
    println!(
        "{when}: current GMs' years in the job mean {mean:.1}, median {}, longest {} ({} clubs)",
        t[t.len() / 2],
        t[t.len() - 1],
        t.len()
    );
}
