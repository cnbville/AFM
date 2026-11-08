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
//! - Redshirts at FBS: growth at the top 10 programs against the rest, all
//!   of them and the quarterbacks (item 16's check).
//! - The players results moved most, with their seasons.
//!
//! Usage: results_check [history_years] [seed] [scale]
//! `AFM_RESULTS`: off, kept (the default: on top of the dice, pull down,
//! practice counting, item 16), kept_games (practice left out), replaced,
//! kept_up, replaced_up, kept_practice (the same as kept), kept_wide (the
//! weight on the skill's whole spread, as the first build).

use std::collections::HashMap;
use std::time::Instant;

use afm_engine::attributes::Attr;
use afm_engine::player::{DevelopmentEvent as Ev, Level, PlayingTime, SeasonRecord};
use afm_engine::positions::Position;
use afm_engine::results::{Dice, Mode, Read};
use afm_engine::world::data::Division;
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
        "kept_games" => Mode { practice: false, ..on },
        "kept_practice" => Mode { practice: true, ..on },
        "kept_wide" => Mode { change: f64::INFINITY, ..on },
        x => panic!(
            "AFM_RESULTS: off, kept, kept_games, replaced, kept_up, replaced_up, kept_practice or kept_wide, not {x}"
        ),
    };
    let opts = WorldOptions { seed, history_years: years, scale, results, ..WorldOptions::default() };
    let t = Instant::now();
    // id -> (season, row index) for each season he had a row.
    let mut seen: HashMap<u32, Vec<(i32, usize)>> = HashMap::new();
    let mut times: Vec<f64> = Vec::new();
    // Per tally level (0 college, 1 NFL, 2 high school): games, points,
    // plays, yards, pass yards, dropbacks, sacks.
    let mut league = [[0.0f64; 7]; 3];
    // A college season's program: (id, season) to his school that season,
    // None when he changed schools that year (the winter portal lands
    // before the season's end here, so that season's school is unsure).
    let mut school: HashMap<(u32, i32), Option<usize>> = HashMap::new();
    let mut moves: HashMap<u32, u32> = HashMap::new();
    // Each season's FBS top 10 by prestige going into it.
    let mut top10: HashMap<i32, Vec<usize>> = HashMap::new();
    let mut last_top: Vec<usize> = Vec::new();
    // The season's evidence at its end, by level and role: player-seasons,
    // trials (every read), of them practice's.
    let mut evidence: HashMap<(usize, usize), (f64, f64, f64)> = HashMap::new();
    // What results moved by read (minor bug 348's check): per (level,
    // position, read), the player-seasons, the rating points moved, the
    // trials, practice's trials, the centres his season was read against
    // (practice's and games', each weighted by its trials) and the moves'
    // squares.
    let mut by_read: HashMap<(usize, usize, usize), [f64; 7]> = HashMap::new();
    // The tackles' block shedding a year (bug 348): per (level, role), the
    // years, the change, the part results moved, the ceiling's change and
    // the rating; each tackle's rating and ceiling at the last season's end.
    let mut shed: HashMap<(usize, usize), [f64; 5]> = HashMap::new();
    let mut shed_was: HashMap<u32, (f64, f64)> = HashMap::new();
    // Their shedding's centres by the rating's band of 10: (level, band) to
    // men, practice trials, practice's centre times them, game trials,
    // games' centre times them.
    let mut shed_band: HashMap<(usize, usize), [f64; 5]> = HashMap::new();
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
        let mut fbs: Vec<(usize, f64)> = w
            .programs
            .iter()
            .enumerate()
            .filter(|(_, c)| c.division == Division::FBS)
            .map(|(i, c)| (i, c.prestige))
            .collect();
        fbs.sort_by(|a, b| b.1.total_cmp(&a.1));
        let now_top: Vec<usize> = fbs.iter().take(10).map(|x| x.0).collect();
        top10.insert(season, if last_top.is_empty() { now_top.clone() } else { last_top.clone() });
        last_top = now_top;
        for p in &w.people {
            let (Some(l), Some(h)) = (w.results.ledgers.get(&p.player.id), p.player.history.last()) else { continue };
            if l.year != season {
                continue;
            }
            let e = evidence.entry((h.level as usize, h.playing_time as usize)).or_default();
            e.0 += 1.0;
            e.1 += l.reads.iter().map(|x| x.n as f64).sum::<f64>();
            e.2 += l.reads.iter().map(|x| x.np as f64).sum::<f64>();
            for x in &l.reads {
                let k = by_read.entry((h.level as usize, p.player.position.index(), x.read as usize)).or_default();
                k[0] += 1.0;
                k[1] += x.applied as f64;
                k[2] += x.n as f64;
                k[3] += x.np as f64;
                k[4] += (x.np * x.cp) as f64;
                k[5] += ((x.n - x.np) * x.c) as f64;
                k[6] += (x.applied as f64).powi(2);
            }
        }
        for p in &w.people {
            let len = p.player.history.len();
            let was = moves.insert(p.player.id, p.transfers).unwrap_or(0);
            if len == 0 {
                continue;
            }
            let v = seen.entry(p.player.id).or_default();
            if v.last().is_none_or(|&(_, k)| k + 1 != len) {
                v.push((season, len - 1));
                if p.player.history[len - 1].level.is_college() {
                    school.insert((p.player.id, season), p.college.filter(|_| p.transfers == was));
                }
            }
        }
        // Each tackle with a row this season: his block shedding against
        // the last season's end, and what results moved of it.
        for p in &w.people {
            if p.player.position != Position::DT {
                continue;
            }
            let id = p.player.id;
            let this = seen.get(&id).and_then(|v| v.last()).filter(|&&(s, _)| s == season).map(|&(_, k)| k);
            let Some(h) = this.and_then(|k| p.player.history.get(k)) else {
                shed_was.remove(&id);
                continue;
            };
            if !(h.level.is_college() || h.level == Level::NFL) {
                shed_was.remove(&id);
                continue;
            }
            let r = p.player.attributes.get(Attr::BlockShedding);
            let c = afm_engine::debug::ceilings(&p.player).get(Attr::BlockShedding);
            let e = w
                .results
                .ledgers
                .get(&id)
                .filter(|l| l.year == season)
                .and_then(|l| l.reads.iter().find(|e| e.read as usize == Read::Shed.index()));
            if let Some(&(r0, c0)) = shed_was.get(&id) {
                let k = shed.entry((h.level as usize, h.playing_time as usize)).or_default();
                k[0] += 1.0;
                k[1] += r - r0;
                k[2] += e.map_or(0.0, |e| e.applied as f64);
                k[3] += c - c0;
                k[4] += r;
            }
            shed_was.insert(id, (r, c));
            if let Some(e) = e {
                let b = shed_band.entry((h.level as usize, (r / 10.0).floor() as usize)).or_default();
                b[0] += 1.0;
                b[1] += e.np as f64;
                b[2] += (e.np * e.cp) as f64;
                b[3] += (e.n - e.np) as f64;
                b[4] += ((e.n - e.np) * e.c) as f64;
            }
        }
        true
    })
    .expect("never stopped");
    let last = w.year - 1;
    let pw = afm_engine::training::practice::PRACTICE_RESULTS;
    println!(
        "# Growth from results in a world: seed {seed}, {years} years, scale {scale}, results {arm} (a practice snap counts {pw} of a game snap when practice is on)\n\nBuilt in {:.0} s; seasons ended at {}.\n",
        t.elapsed().as_secs_f64(),
        times.iter().map(|s| format!("{s:.0} s")).collect::<Vec<_>>().join(", ")
    );
    // Every finished season (the current one is still open): the row, the
    // next row's start, his name and position.
    struct Seen<'a> {
        r: &'a SeasonRecord,
        next: Option<f64>,
        who: usize,
        season: i32,
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
            rows.push(Seen { r, next, who: i, season });
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

    println!("\n## Named seasons by role (big season / rough season, of player-seasons with games)\n");
    println!("| level | starter | rotation | backup | redshirt |\n|---|---|---|---|---|");
    for l in LEVELS {
        let mut line = format!("| {} |", level_name(l));
        for role in [PlayingTime::Starter, PlayingTime::Rotation, PlayingTime::Backup, PlayingTime::Redshirt] {
            let v: Vec<&Seen> = rows
                .iter()
                .filter(|x| x.r.level == l && x.r.playing_time == role && x.r.performance.get().is_some())
                .collect();
            if v.len() < 20 {
                line.push_str(" - |");
                continue;
            }
            let k = |e: Ev| 100.0 * v.iter().filter(|x| x.r.events.has(e)).count() as f64 / v.len() as f64;
            line.push_str(&format!(" {:.1}% / {:.1}% ({}) |", k(Ev::BigSeason), k(Ev::RoughSeason), v.len()));
        }
        println!("{line}");
    }

    println!(
        "\n## The season's evidence by role (trials over every read a player-season, at the season's end; practice's share)\n"
    );
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
            match evidence.get(&(l as usize, role as usize)) {
                Some(&(k, n, np)) if k >= 20.0 => {
                    line.push_str(&format!(" {:.0}, {:.0}% practice ({k:.0}) |", n / k, 100.0 * np / n.max(1e-9)))
                }
                _ => line.push_str(" - |"),
            }
        }
        println!("{line}");
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

    println!("\n## NFL starters' skills at the end (mean / sd): the spread line play reads\n");
    println!("| position | skill | starters | mean | sd |\n|---|---|---|---|---|");
    for (pos, attr) in [
        (Position::OT, Attr::PassBlocking),
        (Position::IOL, Attr::PassBlocking),
        (Position::IOL, Attr::RunBlocking),
        (Position::DT, Attr::PowerRush),
        (Position::DT, Attr::FinesseRush),
        (Position::EDGE, Attr::FinesseRush),
        (Position::DT, Attr::BlockShedding),
        (Position::LB, Attr::Tackling),
    ] {
        let v: Vec<f64> = w
            .people
            .iter()
            .filter(|p| p.player.position == pos)
            .filter(|p| {
                p.player.history.last().is_some_and(|h| h.level == Level::NFL && h.playing_time == PlayingTime::Starter)
            })
            .map(|p| p.player.attributes.get(attr))
            .collect();
        println!("| {} | {attr:?} | {} | {:.1} | {:.2} |", pos.name(), v.len(), mean(&v), sd(&v));
    }

    println!(
        "\n## What results moved by read (rating points a player-season, at the season's end; a read biased one way shows here)\n"
    );
    println!(
        "The 12 reads with the biggest mean move at each level, then every tackle's read. Centres: the league's at his last game and his last practice, rating points.\n"
    );
    println!(
        "| level | position | read | player-seasons | mean move | sd of the mean | trials a season | practice's share | games' centre | practice's centre |\n|---|---|---|---|---|---|---|---|---|---|"
    );
    for l in [Level::FBS, Level::NFL] {
        let mut v: Vec<(&(usize, usize, usize), &[f64; 7])> =
            by_read.iter().filter(|(k, x)| k.0 == l as usize && x[0] >= 200.0).collect();
        v.sort_by(|a, b| (b.1[1] / b.1[0]).abs().total_cmp(&(a.1[1] / a.1[0]).abs()).then(a.0.cmp(b.0)));
        let dt = Position::DT.index();
        let shown: Vec<_> = v.iter().take(12).chain(v.iter().skip(12).filter(|x| x.0.1 == dt)).collect();
        for (k, x) in shown {
            let m = x[1] / x[0];
            let se = ((x[6] / x[0] - m * m).max(0.0) / x[0]).sqrt();
            println!(
                "| {} | {} | {:?} | {:.0} | {:+.3} | {:.3} | {:.0} | {:.0}% | {:+.2} | {:+.2} |",
                level_name(l),
                Position::ALL[k.1].name(),
                Read::from_index(k.2),
                x[0],
                m,
                se,
                x[2] / x[0],
                100.0 * x[3] / x[2].max(1e-9),
                x[5] / (x[2] - x[3]).max(1e-9),
                x[4] / x[3].max(1e-9),
            );
        }
    }

    println!(
        "\n## Tackles' block shedding a year (each tackle with a row both seasons; his rating at the season's end against the last one's)\n"
    );
    println!(
        "| level | role | years | rating | change | from results | the rest (the yearly step, reps) | ceiling's change |\n|---|---|---|---|---|---|---|---|"
    );
    for l in [Level::FBS, Level::FCS, Level::SmallCollege, Level::NFL] {
        for role in [
            PlayingTime::Starter,
            PlayingTime::Rotation,
            PlayingTime::Backup,
            PlayingTime::Redshirt,
            PlayingTime::PracticeSquad,
        ] {
            let Some(x) = shed.get(&(l as usize, role as usize)).filter(|x| x[0] >= 20.0) else { continue };
            println!(
                "| {} | {role:?} | {:.0} | {:.1} | {:+.2} | {:+.3} | {:+.2} | {:+.2} |",
                level_name(l),
                x[0],
                x[4] / x[0],
                x[1] / x[0],
                x[2] / x[0],
                (x[1] - x[2]) / x[0],
                x[3] / x[0],
            );
        }
    }

    println!(
        "\n## Tackles' block shedding: the centres a season was read against, by his rating's band (rating points)\n"
    );
    println!(
        "| level | band | player-seasons | practice trials a man | practice's centre | game trials a man | games' centre |\n|---|---|---|---|---|---|---|"
    );
    let mut bands: Vec<_> = shed_band
        .iter()
        .filter(|(k, x)| (k.0 == Level::FBS as usize || k.0 == Level::NFL as usize) && x[0] >= 20.0)
        .collect();
    bands.sort_by_key(|x| (std::cmp::Reverse(x.0.0 == Level::FBS as usize), x.0.1));
    for (k, x) in bands {
        println!(
            "| {} | {}-{} | {:.0} | {:.0} | {:+.2} | {:.0} | {:+.2} |",
            if k.0 == Level::FBS as usize { "FBS" } else { "NFL" },
            k.1 * 10,
            k.1 * 10 + 9,
            x[0],
            x[1] / x[0],
            x[2] / x[1].max(1e-9),
            x[3] / x[0],
            x[4] / x[3].max(1e-9),
        );
    }

    println!("\n## Redshirts at FBS: growth by program (next season's start against this one's)\n");
    println!("Top 10: the FBS top 10 by prestige going into the season. A season he changed schools in is left out.\n");
    println!(
        "| group | seasons | growth mean | growth sd | moved 1+ by results | results mean | results sd |\n|---|---|---|---|---|---|---|"
    );
    let red = |top: bool, qb: bool| {
        let v: Vec<&Seen> = rows
            .iter()
            .filter(|x| x.r.level == Level::FBS && x.r.playing_time == PlayingTime::Redshirt && x.next.is_some())
            .filter(|x| !qb || x.r.position == Position::QB)
            .filter(|x| {
                let id = w.people[x.who].player.id;
                school
                    .get(&(id, x.season))
                    .copied()
                    .flatten()
                    .is_some_and(|c| top10.get(&x.season).is_some_and(|t| t.contains(&c)) == top)
            })
            .collect();
        let g: Vec<f64> = v.iter().map(|x| x.next.unwrap_or(0.0) - x.r.overall_start).collect();
        let res: Vec<f64> = v.iter().map(|x| x.r.results.get().unwrap_or(0.0)).collect();
        println!(
            "| {}, {} | {} | {:+.2} | {:.2} | {:.1}% | {:+.2} | {:.2} |",
            if qb { "quarterbacks" } else { "every position" },
            if top { "top 10" } else { "the rest" },
            v.len(),
            mean(&g),
            sd(&g),
            100.0 * res.iter().filter(|x| x.abs() >= 1.0).count() as f64 / res.len().max(1) as f64,
            mean(&res),
            sd(&res),
        );
    };
    for qb in [false, true] {
        for top in [true, false] {
            red(top, qb);
        }
    }
    let open = w.people.iter().filter(|p| p.stage == afm_engine::world::Stage::College).count();
    let ledgers = w
        .people
        .iter()
        .filter(|p| p.stage == afm_engine::world::Stage::College && w.results.ledgers.contains_key(&p.player.id))
        .count();
    println!("\nCollege players with a season of results open at the end: {ledgers} of {open}.");

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
    let mut pr: Vec<f64> = w.programs.iter().filter(|c| c.division == Division::FBS).map(|c| c.prestige).collect();
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
