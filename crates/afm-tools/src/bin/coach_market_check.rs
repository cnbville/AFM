//! The coaching market and in-season firings (realism audit items 6, 7, 8
//! and 12) on an existing world file: pay at the top and the middle of each
//! level and rung, then the world played on for a number of days on the
//! stand-in, and again: pay, counters won, in-season firings by level and
//! role, NFL interims, and assistants let go per kept staff.
//! Usage: coach_market_check <world file> <days>

use afm_engine::world::World;
use afm_engine::world::coach::{Employer, Role};
use afm_engine::world::data::Division;
use afm_engine::world::folk::hiring::pay_now;
use std::path::Path;

fn level(w: &World, e: Employer) -> Option<&'static str> {
    match e {
        Employer::College(p) => Some(match w.programs[p].division {
            Division::FBS => "FBS",
            Division::FCS => "FCS",
            _ => "small college",
        }),
        Employer::Nfl(_) => Some("NFL"),
        Employer::HighSchool(_) => Some("high school"),
        _ => None,
    }
}

fn rung(r: Role) -> &'static str {
    match r {
        Role::HeadCoach => "head coach",
        Role::OffensiveCoordinator | Role::DefensiveCoordinator => "coordinator",
        Role::PositionCoach(_) => "position coach",
    }
}

fn pay_table(w: &World, title: &str) {
    println!("\n== Pay a year, {title} ({}) ==", w.today);
    for lv in ["FBS", "FCS", "NFL"] {
        for rg in ["head coach", "coordinator", "position coach"] {
            let mut v: Vec<(f64, usize)> = w
                .coaches
                .iter()
                .enumerate()
                .filter_map(|(c, co)| {
                    let j = co.job?;
                    (level(w, j.employer) == Some(lv) && rung(j.role) == rg).then(|| (pay_now(w, c), c))
                })
                .collect();
            if v.is_empty() {
                continue;
            }
            v.sort_by(|a, b| b.0.total_cmp(&a.0));
            let q = |f: f64| v[((v.len() as f64 - 1.0) * f) as usize].0 / 1e6;
            let top: Vec<String> = v
                .iter()
                .take(5)
                .map(|&(p, c)| {
                    let at = match w.coaches[c].job.unwrap().employer {
                        Employer::College(x) => w.programs[x].name(w.options.names).to_string(),
                        Employer::Nfl(t) => w.nfl[t].name.to_string(),
                        _ => String::new(),
                    };
                    format!("{} ({at}) {:.2}", w.coaches[c].name, p / 1e6)
                })
                .collect();
            println!(
                "{lv} {rg}: n {} | top {:.2} | p90 {:.2} | median {:.2} | p10 {:.2} | top five: {}",
                v.len(),
                v[0].0 / 1e6,
                q(0.1),
                q(0.5),
                q(0.9),
                top.join("; ")
            );
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut w = afm_engine::save::read_world(Path::new(&args[1])).expect("read the world");
    w.options.engine_from = Some(2100);
    let days: u32 = args[2].parse().expect("days");
    let start = w.year;
    let news0 = w.news.len();
    pay_table(&w, "before");
    for _ in 0..days {
        w.advance_day();
    }
    pay_table(&w, "after");
    // Counters won, by year.
    println!("\n== Kept by a counter (news), by year ==");
    let mut kept: std::collections::BTreeMap<i32, u32> = Default::default();
    let mut fired_news: Vec<String> = Vec::new();
    for n in &w.news[news0.min(w.news.len())..] {
        if n.text.contains("came calling about") {
            *kept.entry(n.year).or_default() += 1;
        }
        if n.text.contains("games into") || n.text.contains("interim") {
            fired_news.push(format!("{} {}", n.year, n.text));
        }
    }
    println!("{kept:?}");
    // In-season firings and interims, from the stints.
    println!("\n== Ended during the season, {start} on (by level, rung, why) ==");
    let mut by: std::collections::BTreeMap<(i32, String, String, String), u32> = Default::default();
    for co in &w.coaches {
        for s in &co.history {
            let why = s.ended.to_string();
            if s.to < start
                || !(why.contains("during the season") || why.contains("interim") || why == "let go by the head coach")
            {
                continue;
            }
            let lv = level(&w, s.employer).unwrap_or("other").to_string();
            *by.entry((s.to, lv, rung(s.role).to_string(), why)).or_default() += 1;
        }
    }
    for ((y, lv, rg, why), n) in &by {
        println!("{y} {lv} {rg}: {why} {n}");
    }
    // Staffs that let more than four go in one carousel (item 12).
    let mut per: std::collections::HashMap<(i32, usize), u32> = Default::default();
    for co in &w.coaches {
        for s in &co.history {
            if s.to >= start
                && s.ended == "let go by the head coach"
                && let Employer::College(p) = s.employer
            {
                *per.entry((s.to, p)).or_default() += 1;
            }
        }
    }
    let mut hist: std::collections::BTreeMap<u32, u32> = Default::default();
    for n in per.values() {
        *hist.entry(*n).or_default() += 1;
    }
    println!("\n== Assistants let go by a kept head coach, per staff per carousel: {hist:?}");
    println!("\n== The season's firing and interim news ==");
    for l in fired_news.iter().take(80) {
        println!("{l}");
    }
}
