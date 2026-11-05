//! Growth from the depth chart (minor bug 247, lane NB12): builds a world
//! with its games on the engine and reads, season by season, what each
//! college and high school player's step was given against what he really
//! played.
//! - Role against snaps: each camp role's real snaps that season (college
//!   by division, high school varsity), and true freshmen's.
//! - A full starter's season: the snaps of the players who started all year.
//! - Growth by last season's snaps: a returning player's year (his rating
//!   at the next step against this one) by his role now and the snaps he
//!   played the season before. On main growth follows the role; with the
//!   fix it follows the snaps.
//! - Growth by class: the mean year by college season and high school
//!   grade, to see how far the world moved.
//!
//! Usage: growth_check [history_years] [seed] [scale]
//! (`AFM_ENGINE_FROM` and `AFM_HS_YEARS` as in the other checks.)

use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::Instant;

use afm_engine::player::{Level, PlayingTime};
use afm_engine::world::data::Division;
use afm_engine::world::hs_year::HsStat;
use afm_engine::world::{Stage, World, WorldOptions};

/// Which rows a table's group takes.
type Group = Box<dyn Fn(&Row) -> bool>;

/// One player's step in one season, read once the season was over.
#[derive(Clone, Copy)]
struct Row {
    year: i32,
    college: bool,
    division: Option<Division>,
    role: PlayingTime,
    /// His real offensive and defensive snaps that season (None: the
    /// season was on the stand-in).
    snaps: Option<u32>,
    /// His first season at this level (college: his first college row;
    /// high school: his first high school row).
    first: bool,
    /// Which season at this level (1: his first).
    nth: usize,
    start: f64,
    specialist: bool,
    /// Rows in his history when read: the next season's row is the one after.
    rows: usize,
}

fn role_name(r: PlayingTime) -> &'static str {
    match r {
        PlayingTime::Starter => "starter",
        PlayingTime::Rotation => "rotation",
        PlayingTime::Backup => "backup",
        PlayingTime::Redshirt => "redshirt",
        PlayingTime::PracticeSquad => "practice squad",
        PlayingTime::Unsigned => "unsigned",
    }
}

fn pct(v: &mut [f64], q: f64) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    if v.is_empty() { 0.0 } else { v[((v.len() - 1) as f64 * q).round() as usize] }
}

fn mean(v: &[f64]) -> f64 {
    if v.is_empty() { f64::NAN } else { v.iter().sum::<f64>() / v.len() as f64 }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let n = |k: usize, d: f64| a.get(k).and_then(|s| s.parse().ok()).unwrap_or(d);
    let (years, seed, scale) = (n(1, 4.0) as u32, n(2, 68.0) as u32, n(3, 0.05));
    let env = |k: &str| std::env::var(k).ok().and_then(|v| v.parse::<i64>().ok());
    let d = WorldOptions::default();
    let opts = WorldOptions {
        seed,
        history_years: years,
        scale,
        engine_from: env("AFM_ENGINE_FROM").map(|y| y as i32).or(d.engine_from),
        hs_engine_years: env("AFM_HS_YEARS").map(|n| n as u32).or(d.hs_engine_years),
        ..d
    };
    let engine_from = opts.engine_from;
    let t = Instant::now();
    // id -> his rows, one a season.
    let mut seen: HashMap<u32, Vec<Row>> = HashMap::new();
    let mut hs_years: HashSet<i32> = HashSet::new();
    // Per division: programs a season, and those whose players took no
    // snap (every game on the stand-in: a team short of 30 players).
    let mut dark: BTreeMap<String, (u32, u32)> = BTreeMap::new();
    let w = World::new_logged(opts, None, |w, _| {
        let season = w.year - 1;
        eprintln!("season {season} done, {:.0}s", t.elapsed().as_secs_f64());
        let college_engine = engine_from.is_none_or(|f| season >= f);
        let hs_engine = w.people.iter().any(|p| p.hs_lines.last().is_some_and(|l| l.year as i32 == season));
        if hs_engine {
            hs_years.insert(season);
        }
        if college_engine && season >= w.year - 1 - years as i32 {
            let mut most = vec![0u32; w.programs.len()];
            for p in &w.people {
                if matches!(p.stage, Stage::College | Stage::Declared)
                    && let Some(c) = p.college
                {
                    most[c] = most[c].max(p.last_snaps);
                }
            }
            for (c, prog) in w.programs.iter().enumerate() {
                let e = dark.entry(format!("{:?}", prog.division)).or_default();
                e.0 += 1;
                e.1 += (most[c] == 0) as u32;
            }
        }
        for p in &w.people {
            let Some(last) = p.player.history.last() else { continue };
            let college = matches!(p.stage, Stage::College | Stage::Declared) && last.level.is_college();
            let hs = p.stage == Stage::HighSchool && last.level == Level::HS;
            if !college && !hs {
                continue;
            }
            let id = p.player.id;
            let rows = p.player.history.len();
            // Read once a season: skip a row already read (no step this year).
            if seen.get(&id).and_then(|v| v.last()).is_some_and(|r| r.rows == rows) {
                continue;
            }
            let same = |l: Level| if college { l.is_college() } else { l == Level::HS };
            let nth = p.player.history.iter().filter(|h| same(h.level)).count();
            let first = nth == 1;
            let snaps = if college {
                college_engine.then_some(p.last_snaps)
            } else if hs_engine {
                Some(
                    p.hs_lines
                        .iter()
                        .filter(|l| l.year as i32 == season)
                        .map(|l| (l.get(HsStat::SnapsOff) + l.get(HsStat::SnapsDef)) as u32)
                        .sum(),
                )
            } else {
                None
            };
            let row = Row {
                year: season,
                college,
                division: if college { p.college.map(|c| w.programs[c].division) } else { None },
                role: last.playing_time,
                snaps,
                first,
                nth,
                start: last.overall_start,
                specialist: p.player.position.is_specialist(),
                rows,
            };
            seen.entry(id).or_default().push(row);
        }
        true
    })
    .expect("never stopped");
    println!(
        "world: seed {seed}, {years} years, scale {scale}, engine from {:?}, built in {:.0}s, now {}",
        engine_from,
        t.elapsed().as_secs_f64(),
        w.year
    );
    // Each read row with the next season's start at the same level.
    let mut all: Vec<(Row, Option<Row>, Option<Row>)> = Vec::new();
    for rows in seen.values() {
        for (k, r) in rows.iter().enumerate() {
            let next = rows.get(k + 1).filter(|n| n.college == r.college && n.year == r.year + 1).copied();
            let prev = k
                .checked_sub(1)
                .and_then(|j| rows.get(j))
                .filter(|p| p.college == r.college && p.year + 1 == r.year)
                .copied();
            all.push((*r, prev, next));
        }
    }
    let last_year = w.year - 1;
    let first_year = last_year - years as i32 + 1;
    let in_window = |r: &Row| r.year >= first_year;
    let groups: [(&str, Group); 4] = [
        ("FBS", Box::new(|r: &Row| r.college && r.division == Some(Division::FBS))),
        ("FCS", Box::new(|r: &Row| r.college && r.division == Some(Division::FCS))),
        ("D2 and JUCO", Box::new(|r: &Row| r.college && !matches!(r.division, Some(Division::FBS | Division::FCS)))),
        ("high school", Box::new(|r: &Row| !r.college)),
    ];
    println!("\n== Role against real snaps (non-specialists, seasons {first_year}-{last_year} on the engine) ==");
    for (name, g) in &groups {
        println!("{name}");
        for role in [PlayingTime::Starter, PlayingTime::Rotation, PlayingTime::Backup, PlayingTime::Redshirt] {
            for fresh in [false, true] {
                let mut v: Vec<f64> = all
                    .iter()
                    .map(|x| x.0)
                    .filter(|r| in_window(r) && g(r) && !r.specialist && r.role == role && r.first == fresh)
                    .filter_map(|r| r.snaps.map(|s| s as f64))
                    .collect();
                if v.is_empty() {
                    continue;
                }
                let n = v.len() as f64;
                let low = v.iter().filter(|&&s| s < 100.0).count() as f64 / n;
                let high = v.iter().filter(|&&s| s >= 450.0).count() as f64 / n;
                println!(
                    "  {:<9} {:<10} n {:>6}  mean {:>5.0}  p10 {:>4.0} p50 {:>4.0} p90 {:>4.0}  under 100 {:>5.1}%  450+ {:>5.1}%",
                    role_name(role),
                    if fresh { "first year" } else { "returning" },
                    v.len(),
                    mean(&v),
                    pct(&mut v, 0.1),
                    pct(&mut v, 0.5),
                    pct(&mut v, 0.9),
                    100.0 * low,
                    100.0 * high
                );
            }
        }
    }
    println!("\n== First-year players: camp role and real snaps (non-specialists) ==");
    for (name, g) in &groups {
        let rows: Vec<Row> =
            all.iter().map(|x| x.0).filter(|r| in_window(r) && g(r) && !r.specialist && r.first).collect();
        let with: Vec<&Row> = rows.iter().filter(|r| r.snaps.is_some()).collect();
        if with.is_empty() {
            continue;
        }
        let n = with.len() as f64;
        let share = |f: &dyn Fn(&Row) -> bool| 100.0 * with.iter().filter(|r| f(r)).count() as f64 / n;
        println!(
            "{name:<12} n {:>6}: camp starters {:>5.1}%, real 450+ snaps {:>5.1}%, camp backups or redshirts with 450+ {:>5.1}%, camp starters under 100 {:>5.1}%",
            with.len(),
            share(&|r| r.role == PlayingTime::Starter),
            share(&|r| r.snaps.unwrap() >= 450),
            share(&|r| matches!(r.role, PlayingTime::Backup | PlayingTime::Redshirt) && r.snaps.unwrap() >= 450),
            share(&|r| r.role == PlayingTime::Starter && r.snaps.unwrap() < 100),
        );
    }
    println!("\n== A full starter's season: camp starters' snaps (non-specialists) ==");
    for (name, g) in &groups {
        let mut v: Vec<f64> = all
            .iter()
            .map(|x| x.0)
            .filter(|r| in_window(r) && g(r) && !r.specialist && r.role == PlayingTime::Starter)
            .filter_map(|r| r.snaps.map(|s| s as f64))
            .collect();
        if v.is_empty() {
            continue;
        }
        println!(
            "{name:<12} n {:>6}  p50 {:>4.0}  p75 {:>4.0}  p90 {:>4.0}  max {:>4.0}",
            v.len(),
            pct(&mut v, 0.5),
            pct(&mut v, 0.75),
            pct(&mut v, 0.9),
            pct(&mut v, 1.0)
        );
    }
    println!("\n== A returning player's year by his role now and last season's snaps (non-specialists) ==");
    println!("(growth: his rating at the next step minus this one; last season on the engine)");
    let buckets: [(&str, f64, f64); 4] =
        [("0-99", 0.0, 100.0), ("100-299", 100.0, 300.0), ("300-549", 300.0, 550.0), ("550+", 550.0, 1e9)];
    for (name, g) in &groups {
        println!("{name}");
        print!("  {:<9}", "role now");
        for b in &buckets {
            print!("  {:>17}", format!("last {}", b.0));
        }
        println!();
        for role in [PlayingTime::Starter, PlayingTime::Rotation, PlayingTime::Backup] {
            print!("  {:<9}", role_name(role));
            for b in &buckets {
                let v: Vec<f64> = all
                    .iter()
                    .filter(|(r, p, nx)| {
                        in_window(r)
                            && g(r)
                            && !r.specialist
                            && r.role == role
                            && nx.is_some()
                            && p.and_then(|p| p.snaps).is_some_and(|s| (b.1..b.2).contains(&(s as f64)))
                    })
                    .map(|(r, _, nx)| nx.unwrap().start - r.start)
                    .collect();
                if v.len() < 20 {
                    print!("  {:>17}", "-");
                } else {
                    print!("  {:>+8.2} n {:>5}", mean(&v), v.len());
                }
            }
            println!();
        }
    }
    println!("\n== The mean year by class (every non-specialist with a next step) ==");
    for (name, g) in &groups {
        let mut by: BTreeMap<usize, Vec<f64>> = BTreeMap::new();
        for (r, _, nx) in &all {
            let Some(nx) = nx else { continue };
            if !in_window(r) || !g(r) || r.specialist {
                continue;
            }
            // Class: rows at this level so far.
            let k = r.nth.min(5);
            by.entry(k).or_default().push(nx.start - r.start);
        }
        let parts: Vec<String> = by.iter().map(|(k, v)| format!("year {k}: {:+.2} (n {})", mean(v), v.len())).collect();
        let every: Vec<f64> = by.values().flatten().copied().collect();
        println!("{name:<12} all {:+.3} (n {})  {}", mean(&every), every.len(), parts.join(", "));
    }
    println!("\n== College teams whose players took no snap (all games on the stand-in) ==");
    for (d, (n, none)) in &dark {
        println!("{d:<6} {none} of {n} team seasons ({:.1}%)", 100.0 * *none as f64 / (*n).max(1) as f64);
    }
    println!("\nhigh school seasons on the engine: {:?}", {
        let mut v: Vec<i32> = hs_years.into_iter().collect();
        v.sort();
        v
    });
}
