//! When recruits get offers and commit (thread "Make AI schools offer on
//! real days"): builds a world on the stand-in and prints, per signing
//! year, commitments by month, broken verbals and signing periods; then,
//! for the last class signed, the days its commitments fell on and where
//! the blue chips signed; and for the class behind it, the offers held by
//! national rank at the end of its junior year.
//! Usage: recruit_days <seed> <history years> [scale]
//!    or: recruit_days play <world file> (plays a saved world through the
//!        next signing day and prints, for the class signing then, when its
//!        blue chips committed and why the rest waited: thread "Get blue
//!        chips committing on time"). AFM_WHY=1 also prints, on each of
//!        those days, why the open blue chips wait (his lead over his bar,
//!        whether he has seen the leader, his timing).

use afm_engine::world::data::Division;
use afm_engine::world::{Stage, World, WorldOptions};
use std::collections::BTreeMap;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("play") {
        play(&args[2]);
        return;
    }
    let seed: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(2026);
    let years: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(4);
    let scale: f64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1.0);
    let t = std::time::Instant::now();
    let w =
        World::new(WorldOptions { seed, scale, history_years: years, engine_from: Some(2100), ..Default::default() });
    println!("built in {:.0}s, today {}", t.elapsed().as_secs_f64(), w.today.pretty());
    println!(
        "year | commits by month Jan..Dec | commits decommits flips | early late | blue verbal broken | by bar | blue signed, committed there before Sep 1"
    );
    for l in w.log_history.iter().rev().take(years as usize).rev() {
        println!(
            "{} | {:?} | {} {} {} | {} {} | {} {} | {} | {} {} ({:.0}%)",
            l.year,
            l.commits_by_month,
            l.commits,
            l.decommits,
            l.flips,
            l.signed_early,
            l.signed_late,
            l.blue_verbal,
            l.blue_broken,
            l.bar_commits,
            l.blue_signed,
            l.blue_signed_summer,
            100.0 * l.blue_signed_summer as f64 / l.blue_signed.max(1) as f64
        );
    }
    // The last class through the summer: the days its verbals fell on.
    let class = w
        .people
        .iter()
        .filter(|p| p.cal.signed.is_some() && matches!(p.stage, Stage::HighSchool | Stage::PrepSchool))
        .map(|p| p.class_of)
        .max()
        .unwrap_or(w.today.year());
    let mut days: BTreeMap<i32, u32> = BTreeMap::new();
    let mut weeks: BTreeMap<String, u32> = BTreeMap::new();
    for p in w.people.iter().filter(|p| p.class_of == class) {
        let when = p.cal.signed.and_then(|s| s.committed).or(p.cal.commit.map(|c| c.1));
        let Some(d) = when else { continue };
        *days.entry(d.0).or_default() += 1;
        *weeks.entry(format!("{}-{:02}", d.year(), d.month())).or_default() += 1;
    }
    let busiest: Vec<(String, u32)> = {
        let mut v: Vec<(i32, u32)> = days.iter().map(|(&d, &n)| (d, n)).collect();
        v.sort_by_key(|a| std::cmp::Reverse(a.1));
        v.iter().take(8).map(|&(d, n)| (afm_engine::calendar::Date(d).pretty(), n)).collect()
    };
    let total: u32 = days.values().sum();
    println!("class {class}: {total} verbals on {} days; busiest {:?}; by month {:?}", days.len(), busiest, weeks);
    // Where the blue chips signed: the top 25 programs by prestige.
    let mut by_prestige: Vec<usize> =
        (0..w.programs.len()).filter(|&j| w.programs[j].division == Division::FBS).collect();
    by_prestige.sort_by(|&a, &b| w.programs[b].prestige.total_cmp(&w.programs[a].prestige));
    let top25: Vec<usize> = by_prestige.iter().take(25).copied().collect();
    for stars in [5u8, 4, 3] {
        let signed: Vec<usize> = w
            .people
            .iter()
            .filter(|p| p.class_of == class && p.stars == stars && !p.player.position.is_specialist())
            .filter_map(|p| p.cal.signed.map(|s| s.program).or(p.college))
            .collect();
        let at_top = signed.iter().filter(|j| top25.contains(j)).count();
        let fbs = signed.iter().filter(|&&j| w.programs[j].division == Division::FBS).count();
        println!(
            "class {class} {stars}-stars: {} signed, {fbs} FBS, {at_top} at the top 25 by prestige ({:.0}%)",
            signed.len(),
            100.0 * at_top as f64 / signed.len().max(1) as f64
        );
    }
    // The class behind it: offers held by national rank (stars, then the services' read).
    let next = class + 1;
    let mut juniors: Vec<(f64, usize, usize)> = w
        .people
        .iter()
        .filter(|p| p.class_of == next && p.stage == Stage::HighSchool)
        .map(|p| {
            let read = p.cal.service_read.get().unwrap_or(0.0);
            let early = p.cal.early_offers.len();
            let first = p.cal.early_offers.first().map_or(0, |o| o.1.0 as usize);
            (p.stars as f64 * 100.0 + read, early, first)
        })
        .collect();
    juniors.sort_by(|a, b| b.0.total_cmp(&a.0));
    let at = |r: usize| -> String {
        let r = ((r as f64 * scale) as usize).min(juniors.len().saturating_sub(1));
        let lo = r.saturating_sub(5);
        let hi = (r + 5).min(juniors.len());
        let n: usize = juniors[lo..hi].iter().map(|j| j.1).sum();
        format!("{:.1}", n as f64 / (hi - lo).max(1) as f64)
    };
    println!(
        "class {next} (juniors) early offers held near rank 1 {}, 10 {}, 100 {}, 1000 {}, 3000 {}",
        at(1),
        at(10),
        at(100),
        at(1000),
        at(3000)
    );
    let mut first_month: BTreeMap<String, u32> = BTreeMap::new();
    for p in w.people.iter().filter(|p| p.class_of == next || p.class_of == next + 1) {
        for o in &p.cal.early_offers {
            *first_month.entry(format!("{} {}-{:02}", p.class_of, o.1.year(), o.1.month())).or_default() += 1;
        }
    }
    println!("early offers by class and month: {first_month:?}");
}

/// What a recruit of the class looked like on a snapshot day.
#[derive(Clone, Copy)]
struct Snap {
    stars: u8,
    timing: Option<afm_engine::world::CommitTiming>,
    day: Option<afm_engine::calendar::Date>,
    committed: bool,
    offers: usize,
}

fn snap(w: &World, class: i32) -> std::collections::HashMap<u32, Snap> {
    (0..w.people.len())
        .filter(|&i| {
            let p = &w.people[i];
            p.class_of == class && p.stage == Stage::HighSchool && !p.player.position.is_specialist()
        })
        .map(|i| {
            let p = &w.people[i];
            let offers = {
                let mut v: Vec<usize> =
                    p.cal.offers.iter().copied().chain(p.cal.early_offers.iter().map(|o| o.0)).collect();
                v.sort_unstable();
                v.dedup();
                v.len()
            };
            (
                p.player.id,
                Snap {
                    stars: p.stars,
                    timing: p.cal.commit_timing,
                    day: afm_engine::world::scouting::services::decision_day(w, i),
                    committed: p.cal.commit.is_some(),
                    offers,
                },
            )
        })
        .collect()
}

/// Plays a saved world through its next February signing day.
fn play(path: &str) {
    use afm_engine::calendar::Date;
    let t = std::time::Instant::now();
    let mut w = afm_engine::save::read_world(std::path::Path::new(path)).expect("read the world");
    w.options.engine_from = Some(2100);
    let start = w.today;
    // The class whose summer is ahead: its senior season starts this fall.
    let season = start.year();
    let class = season + 1;
    let sep1 = Date::ymd(season, 9, 1);
    let end = Date::ymd(class, 2, 20);
    eprintln!("read in {:.0} s, today {}, class {class}", t.elapsed().as_secs_f64(), start.pretty());
    let mut snaps: Vec<(Date, std::collections::HashMap<u32, Snap>)> = Vec::new();
    let marks =
        [Date::ymd(season, 6, 1), Date::ymd(season, 7, 1), Date::ymd(season, 8, 1), sep1, Date::ymd(season, 12, 1)];
    while w.today < end {
        if marks.contains(&w.today) {
            let s = snap(&w, class);
            let blue: Vec<&Snap> = s.values().filter(|x| x.stars >= 4).collect();
            let c = blue.iter().filter(|x| x.committed).count();
            println!(
                "{}: blue chips (stars that day) committed {c} of {} ({:.0}%)",
                w.today.pretty(),
                blue.len(),
                100.0 * c as f64 / blue.len().max(1) as f64
            );
            if std::env::var("AFM_WHY").is_ok() {
                why_open(&w, class);
            }
            snaps.push((w.today, s));
        }
        w.advance_day();
    }
    println!("played to {} in {:.0} s", w.today.pretty(), t.elapsed().as_secs_f64());
    if let Some(l) = w.log_history.iter().chain(std::iter::once(&w.log)).find(|l| l.year == season) {
        println!(
            "class {class}, counted at signing: {} blue chips signed, {} committed there before Sep 1 ({:.0}%); commits by month {:?}; blue verbals {}, broken {}, by bar {}",
            l.blue_signed,
            l.blue_signed_summer,
            100.0 * l.blue_signed_summer as f64 / l.blue_signed.max(1) as f64,
            l.commits_by_month,
            l.blue_verbal,
            l.blue_broken,
            l.bar_commits
        );
    }
    let at_sep = &snaps.iter().find(|s| s.0 == sep1).expect("Sep 1").1;
    // The class's final blue chips (the measure in `main`).
    let finals: Vec<&afm_engine::world::Person> =
        w.people.iter().filter(|p| p.class_of == class && p.stars >= 4 && !p.player.position.is_specialist()).collect();
    let mut rows: BTreeMap<String, u32> = BTreeMap::new();
    let mut by_month: BTreeMap<String, u32> = BTreeMap::new();
    let mut before = 0;
    let on_sep = finals.iter().filter(|p| at_sep.get(&p.player.id).is_some_and(|s| s.committed)).count();
    for p in &finals {
        let when = p.cal.signed.and_then(|s| s.committed).or(p.cal.commit.map(|c| c.1));
        if when.is_some_and(|d| d < sep1) {
            before += 1;
        }
        *by_month.entry(when.map_or("never".into(), |d| format!("{}-{:02}", d.year(), d.month()))).or_default() += 1;
        let Some(s) = at_sep.get(&p.player.id) else {
            *rows.entry("not in class on Sep 1".into()).or_default() += 1;
            continue;
        };
        let state = if s.committed {
            if when.is_some_and(|d| d < sep1) {
                "committed Sep 1, kept"
            } else {
                "committed Sep 1, later flipped or fell through"
            }
        } else {
            "open on Sep 1"
        };
        let due = match s.day {
            Some(d) if d < sep1 => "day before Sep 1",
            Some(_) => "day after Sep 1",
            None => "no day",
        };
        *rows.entry(format!("{state} | was {}-star | {:?} | {due}", s.stars, s.timing)).or_default() += 1;
        if !s.committed {
            *rows.entry(format!("open on Sep 1 with {} offers", s.offers.min(10))).or_default() += 1;
        }
    }
    println!(
        "class {class}: final blue chips committed somewhere on Sep 1 of the senior year {} of {} ({:.0}%); with a signing still on file that dates it before Sep 1 {before} (early enrollees on campus have none)",
        on_sep,
        finals.len(),
        100.0 * on_sep as f64 / finals.len().max(1) as f64
    );
    println!("their commitment month (to where they signed): {by_month:?}");
    for (k, v) in rows {
        println!("  {v:5} {k}");
    }
}

/// Why the open blue chips are open (debug): his timing, whether he has
/// seen the leader (any visit, an official), and his lead over his bar.
fn why_open(w: &World, class: i32) {
    use afm_engine::world::front_office::college::{hs_bar, race_at};
    let mut rows: BTreeMap<String, u32> = BTreeMap::new();
    for i in 0..w.people.len() {
        let p = &w.people[i];
        if p.class_of != class
            || p.stage != Stage::HighSchool
            || p.stars < 4
            || p.player.position.is_specialist()
            || p.cal.commit.is_some()
        {
            continue;
        }
        let race = race_at(w, i);
        let Some(top) = race.first() else {
            *rows.entry("no race".into()).or_default() += 1;
            continue;
        };
        let lead = race.get(1).map_or(0.06, |s| top.1 - s.1) / hs_bar(w, i).max(1e-6);
        let any = p.cal.visits.iter().any(|v| v.0 == top.0);
        let off = p.cal.visits.iter().any(|v| v.0 == top.0 && v.2 == afm_engine::world::Visit::Official);
        let band = if lead >= 1.0 {
            "lead >= bar"
        } else if lead >= 0.5 {
            "lead half to 1 bar"
        } else {
            "lead < half bar"
        };
        let seen = if off {
            "official to leader"
        } else if any {
            "unofficial to leader"
        } else {
            "leader unseen"
        };
        let t = match p.cal.commit_timing {
            Some(afm_engine::world::CommitTiming::Summer(n)) => format!("Summer({n})"),
            x => format!("{x:?}"),
        };
        *rows.entry(format!("{band} | {seen}")).or_default() += 1;
        *rows.entry(format!("timing {t}")).or_default() += 1;
        *rows.entry(format!("offers {}", p.cal.offers.len())).or_default() += 1;
    }
    println!("{}: open blue chips", w.today.pretty());
    for (k, v) in rows {
        println!("    {v:5} {k}");
    }
}
