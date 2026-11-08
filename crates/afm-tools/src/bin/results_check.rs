//! Growth from results in a world (the results lane's world check): builds
//! a world with its games on the engine and reads every player's seasons
//! once the world is done (the season's named moments land at his next
//! checkpoint), so two runs on one seed, results off and on, compare.
//! - Growth per season by level and role (his next season's start against
//!   this one's).
//! - What results moved: the overall points a season, their spread, and the
//!   named big games, big seasons and rough seasons, by level.
//! - The dice's breakouts and slumps, by level.
//! - Rating levels by stage (college seniors, NFL rookies, NFL starters).
//! - The players results moved most, with their seasons.
//!
//! Usage: results_check [history_years] [seed] [scale]
//! `AFM_RESULTS`: off, kept (the default: on top of the dice, pull down),
//! replaced, kept_up, replaced_up, kept_practice (item 16 on).

use std::collections::HashMap;
use std::time::Instant;

use afm_engine::player::{DevelopmentEvent as Ev, Level, PlayingTime, SeasonRecord};
use afm_engine::results::{Dice, Mode};
use afm_engine::world::{World, WorldOptions};

fn mean(v: &[f64]) -> f64 {
    if v.is_empty() { f64::NAN } else { v.iter().sum::<f64>() / v.len() as f64 }
}

fn sd(v: &[f64]) -> f64 {
    let m = mean(v);
    (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / v.len().max(1) as f64).sqrt()
}

fn pct(v: &mut [f64], q: f64) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    v.sort_by(f64::total_cmp);
    v[((v.len() - 1) as f64 * q).round() as usize]
}

fn level_name(l: Level) -> &'static str {
    match l {
        Level::HS => "high school",
        Level::FBS => "FBS",
        Level::FCS => "FCS",
        Level::SmallCollege => "small colleges",
        Level::NFL => "NFL",
    }
}

const LEVELS: [Level; 5] = [Level::HS, Level::FBS, Level::FCS, Level::SmallCollege, Level::NFL];

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let n = |k: usize, d: f64| a.get(k).and_then(|s| s.parse().ok()).unwrap_or(d);
    let (years, seed, scale) = (n(1, 4.0) as u32, n(2, 73.0) as u32, n(3, 0.1));
    let arm = std::env::var("AFM_RESULTS").unwrap_or_else(|_| "kept".into());
    let on = Mode::default();
    let results = match arm.as_str() {
        "off" => Mode::OFF,
        "kept" => on,
        "replaced" => Mode { dice: Dice::Replaced, ..on },
        "kept_up" => Mode { pull_down: false, ..on },
        "replaced_up" => Mode { dice: Dice::Replaced, pull_down: false, ..on },
        "kept_practice" => Mode { practice: true, ..on },
        x => panic!("AFM_RESULTS: off, kept, replaced, kept_up, replaced_up or kept_practice, not {x}"),
    };
    let opts = WorldOptions { seed, history_years: years, scale, results, ..WorldOptions::default() };
    let t = Instant::now();
    // id -> (season, row index) for each season he had a row.
    let mut seen: HashMap<u32, Vec<(i32, usize)>> = HashMap::new();
    let mut times: Vec<f64> = Vec::new();
    // Per tally level (0 college, 1 NFL, 2 high school): games, points,
    // plays, yards, pass yards, dropbacks, sacks.
    let mut league = [[0.0f64; 7]; 3];
    let w = World::new_logged(opts, None, |w, _| {
        let season = w.year - 1;
        times.push(t.elapsed().as_secs_f64());
        eprintln!("season {season} done, {:.0}s", t.elapsed().as_secs_f64());
        w.tally_squads(season, |s| {
            let l = &mut league[(s.level as usize).min(2)];
            l[0] += s.games as f64;
            l[1] += s.points_for as f64;
            for (k, name) in ["plays", "yards", "pass_yards", "dropbacks", "sacks"].iter().enumerate() {
                l[2 + k] += s.box_of(0, name);
            }
        });
        for p in &w.people {
            let len = p.player.history.len();
            if len == 0 {
                continue;
            }
            let v = seen.entry(p.player.id).or_default();
            if v.last().is_none_or(|&(_, k)| k + 1 != len) {
                v.push((season, len - 1));
            }
        }
        true
    })
    .expect("never stopped");
    let last = w.year - 1;
    println!(
        "# Growth from results in a world: seed {seed}, {years} years, scale {scale}, results {arm}\n\nBuilt in {:.0} s; seasons ended at {}.\n",
        t.elapsed().as_secs_f64(),
        times.iter().map(|s| format!("{s:.0} s")).collect::<Vec<_>>().join(", ")
    );
    // Every finished season (the current one is still open): the row, the
    // next row's start, his name and position.
    struct Seen<'a> {
        r: &'a SeasonRecord,
        next: Option<f64>,
        who: usize,
    }
    let mut rows: Vec<Seen> = Vec::new();
    for (i, p) in w.people.iter().enumerate() {
        let Some(v) = seen.get(&p.player.id) else { continue };
        for (k, &(season, idx)) in v.iter().enumerate() {
            if season >= last {
                continue;
            }
            let Some(r) = p.player.history.get(idx) else { continue };
            let next = v
                .get(k + 1)
                .filter(|&&(s, j)| s == season + 1 && j == idx + 1)
                .and_then(|&(_, j)| p.player.history.get(j))
                .map(|n| n.overall_start);
            rows.push(Seen { r, next, who: i });
        }
    }
    println!(
        "{} player-seasons read, {} with a next season.\n",
        rows.len(),
        rows.iter().filter(|x| x.next.is_some()).count()
    );

    println!("## Growth per season by level and role (next season's start against this one's)\n");
    println!("| level | starter | rotation | backup | redshirt | practice squad |\n|---|---|---|---|---|---|");
    for l in LEVELS {
        let mut line = format!("| {} |", level_name(l));
        for role in [
            PlayingTime::Starter,
            PlayingTime::Rotation,
            PlayingTime::Backup,
            PlayingTime::Redshirt,
            PlayingTime::PracticeSquad,
        ] {
            let v: Vec<f64> = rows
                .iter()
                .filter(|x| x.r.level == l && x.r.playing_time == role)
                .filter_map(|x| x.next.map(|n| n - x.r.overall_start))
                .collect();
            if v.len() < 20 {
                line.push_str(" - |");
            } else {
                line.push_str(&format!(" {:+.2} ({}) |", mean(&v), v.len()));
            }
        }
        println!("{line}");
    }

    println!("\n## What results moved, and the dice, by level (player-seasons with games)\n");
    println!(
        "| level | seasons | results mean | results sd | moved 1+ either way | starters with a named big game | big season | rough season | dice breakout | dice slump |\n|---|---|---|---|---|---|---|---|---|---|"
    );
    for l in LEVELS {
        let v: Vec<&Seen> = rows.iter().filter(|x| x.r.level == l && x.r.performance.get().is_some()).collect();
        let res: Vec<f64> = v.iter().filter_map(|x| x.r.results.get()).collect();
        let starters: Vec<&&Seen> = v.iter().filter(|x| x.r.playing_time == PlayingTime::Starter).collect();
        let share = |f: &dyn Fn(&Seen) -> bool, of: &[&Seen]| {
            100.0 * of.iter().filter(|x| f(x)).count() as f64 / of.len().max(1) as f64
        };
        let st: Vec<&Seen> = starters.iter().map(|x| **x).collect();
        println!(
            "| {} | {} | {:+.2} | {:.2} | {:.1}% | {:.1}% | {:.1}% | {:.1}% | {:.1}% | {:.1}% |",
            level_name(l),
            v.len(),
            mean(&res),
            sd(&res),
            100.0 * res.iter().filter(|x| x.abs() >= 1.0).count() as f64 / res.len().max(1) as f64,
            share(&|x| x.r.events.has(Ev::BigGame), &st),
            share(&|x| x.r.events.has(Ev::BigSeason), &v),
            share(&|x| x.r.events.has(Ev::RoughSeason), &v),
            share(&|x| x.r.events.has(Ev::Breakout), &v),
            share(&|x| x.r.events.has(Ev::Slump), &v),
        );
    }

    println!("\n## What results moved by role (overall points a season, mean; a bias by role would show here)\n");
    println!("| level | starter | rotation | backup | redshirt |\n|---|---|---|---|---|");
    for l in LEVELS {
        let mut line = format!("| {} |", level_name(l));
        for role in [PlayingTime::Starter, PlayingTime::Rotation, PlayingTime::Backup, PlayingTime::Redshirt] {
            let v: Vec<f64> = rows
                .iter()
                .filter(|x| x.r.level == l && x.r.playing_time == role)
                .filter_map(|x| x.r.results.get())
                .collect();
            if v.len() < 20 {
                line.push_str(" - |");
            } else {
                line.push_str(&format!(" {:+.3} ({}) |", mean(&v), v.len()));
            }
        }
        println!("{line}");
    }

    println!("\n## Rating levels by stage (overall at the season's start: mean / 90th / 99th)\n");
    println!("| stage | seasons | overall |\n|---|---|---|");
    let stage = |label: &str, f: &dyn Fn(&SeasonRecord) -> bool| {
        let mut v: Vec<f64> = rows.iter().filter(|x| f(x.r)).map(|x| x.r.overall_start).collect();
        let m = mean(&v);
        let (p90, p99) = (pct(&mut v, 0.9), pct(&mut v, 0.99));
        println!("| {label} | {} | {m:.1} / {p90:.1} / {p99:.1} |", v.len());
    };
    stage("high school seniors (17)", &|r| r.level == Level::HS && r.age == 17);
    stage("FBS starters 21-22", &|r| {
        r.level == Level::FBS && r.playing_time == PlayingTime::Starter && (21..=22).contains(&r.age)
    });
    stage("NFL rookies (22-23)", &|r| r.level == Level::NFL && (22..=23).contains(&r.age));
    stage("NFL starters 23-25", &|r| {
        r.level == Level::NFL && r.playing_time == PlayingTime::Starter && (23..=25).contains(&r.age)
    });
    stage("NFL starters 26-29", &|r| {
        r.level == Level::NFL && r.playing_time == PlayingTime::Starter && (26..=29).contains(&r.age)
    });

    println!("\n## League numbers (every team game in the seasons played)\n");
    println!(
        "| level | team games | points a game | yards a play | pass yards a dropback | sacks a dropback |\n|---|---|---|---|---|---|"
    );
    for (k, name) in [(0, "college"), (1, "NFL"), (2, "high school")] {
        let l = league[k];
        println!(
            "| {name} | {:.0} | {:.2} | {:.3} | {:.3} | {:.4} |",
            l[0],
            l[1] / l[0].max(1.0),
            l[3] / l[2].max(1.0),
            l[4] / l[5].max(1.0),
            l[6] / l[5].max(1.0)
        );
    }

    println!("\n## The draft mix (picks in the seasons played)\n");
    let mut by_pos: std::collections::BTreeMap<&str, u32> = std::collections::BTreeMap::new();
    let (mut picks, mut fbs, mut round1) = (0u32, 0u32, 0u32);
    let first = last - years as i32 + 1;
    for p in &w.people {
        let Some((year, round, _, _)) = p.pro.as_ref().and_then(|c| c.draft) else { continue };
        if year < first || year > last + 1 {
            continue;
        }
        picks += 1;
        round1 += (round == 1) as u32;
        *by_pos.entry(p.player.position.name()).or_default() += 1;
        let college = p.player.history.iter().rev().find(|h| h.level.is_college()).map(|h| h.level);
        fbs += (college == Some(Level::FBS)) as u32;
    }
    println!(
        "{picks} picks, {:.1}% from FBS, {round1} in round 1. By position: {}.",
        100.0 * fbs as f64 / picks.max(1) as f64,
        by_pos
            .iter()
            .map(|(k, v)| format!("{k} {:.1}%", 100.0 * *v as f64 / picks.max(1) as f64))
            .collect::<Vec<_>>()
            .join(", ")
    );

    println!("\n## Program spread (FBS prestige at the end)\n");
    let mut pr: Vec<f64> = w
        .programs
        .iter()
        .filter(|c| c.division == afm_engine::world::data::Division::FBS)
        .map(|c| c.prestige)
        .collect();
    let (m, s) = (mean(&pr), sd(&pr));
    pr.sort_by(|a, b| b.total_cmp(a));
    println!("{} FBS programs: mean {m:.3}, sd {s:.3}, top 10 mean {:.3}.", pr.len(), mean(&pr[..pr.len().min(10)]));

    println!("\n## The players results moved most in a season\n");
    let mut moved: Vec<&Seen> = rows.iter().filter(|x| x.r.results.get().is_some()).collect();
    moved.sort_by(|x, y| y.r.results.get().unwrap_or(0.0).total_cmp(&x.r.results.get().unwrap_or(0.0)));
    let show = |x: &Seen| {
        let p = &w.people[x.who];
        let mut ev: Vec<&str> = Vec::new();
        for (e, s) in [
            (Ev::BigGame, "a named big game"),
            (Ev::BigSeason, "a named big season"),
            (Ev::RoughSeason, "a named rough season"),
            (Ev::Breakout, "a dice breakout"),
            (Ev::Slump, "a dice slump"),
        ] {
            if x.r.events.has(e) {
                ev.push(s);
            }
        }
        println!(
            "- {} ({}, {} {:?}, {}): {:.1} to {:.1}, results {:+.2}{}{}",
            p.name,
            x.r.position.name(),
            level_name(x.r.level),
            x.r.playing_time,
            x.r.age,
            x.r.overall_start,
            x.next.unwrap_or(x.r.overall),
            x.r.results.get().unwrap_or(0.0),
            if ev.is_empty() { String::new() } else { format!("; {}", ev.join(", ")) },
            if x.next.is_none() { " (no next season here)" } else { "" },
        );
    };
    for x in moved.iter().take(8) {
        show(x);
    }
    println!();
    for x in moved.iter().rev().take(8) {
        show(x);
    }
}
