//! The coaching carousel against real (minor bugs 124, 127, 128, 129 and
//! 148): builds a world and measures what the book's Coaches, World N40 and
//! High school C26 pages measure, without writing a world file or a book.
//! - FBS head coaches (124): new head coaches a season, the median finished
//!   run, the share lasting five seasons, the share who held two FBS jobs.
//! - Who the head coaches are (127, 148): NFL head coaches of color and
//!   Black FBS head coaches, each season and at the end.
//! - Overlapping head jobs (128): pairs of a coach's head jobs that overlap
//!   by more than the move's season (the book's `S7.no_overlap`).
//! - High school champions (129): how often a school starts the season after
//!   a state title with a new head coach, and the schools that change most.
//!
//! Usage: carousel_check [history_years] [seed] [scale] [future]
//! (`AFM_ENGINE_FROM=2100` keeps the history on the stand-in, for speed).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::Instant;

use afm_engine::world::coach::{Employer, Role};
use afm_engine::world::data::Division;
use afm_engine::world::ethnicity::Ethnicity;
use afm_engine::world::{World, WorldOptions};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let n = |k: usize, d: f64| a.get(k).and_then(|s| s.parse().ok()).unwrap_or(d);
    let (years, seed, scale, future) = (n(1, 30.0) as u32, n(2, 2026.0) as u32, n(3, 1.0), n(4, 0.0) as u32);
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
    let t = Instant::now();
    let mut w = World::new_logged(opts, None, |w, _| {
        eprintln!("season {} done, {:.0}s", w.year - 1, t.elapsed().as_secs_f64());
        true
    })
    .expect("never stopped");
    for _ in 0..future {
        w.advance_year();
    }
    println!("world: seed {seed}, {years} years, scale {scale}, +{future}, built in {:.0}s", t.elapsed().as_secs_f64());
    if let Ok(path) = std::env::var("AFM_COACH_CSV") {
        coach_csv(&w, &path);
    }
    fbs_turnover(&w);
    who(&w);
    overlaps(&w);
    champions(&w);
}

/// A program's division in a season (its real past where it differs).
fn division(w: &World, p: usize, year: i32) -> Option<Division> {
    let prog = &w.programs[p];
    match prog.history.iter().find(|h| (h.0..=h.1).contains(&year)) {
        Some(h) => h.2,
        None => Some(prog.division),
    }
}

fn pct(x: usize, of: usize) -> f64 {
    100.0 * x as f64 / of.max(1) as f64
}

fn fbs_turnover(w: &World) {
    // (program, year, coach) for every FBS season with a coach.
    let mut seasons: Vec<(usize, i32, usize)> = Vec::new();
    for (p, prog) in w.programs.iter().enumerate() {
        for s in &prog.seasons {
            if let Some(c) = s.coach
                && division(w, p, s.year) == Some(Division::FBS)
            {
                seasons.push((p, s.year, c));
            }
        }
    }
    let first = w.programs.iter().flat_map(|p| p.seasons.iter().map(|s| s.year)).min().unwrap_or(0);
    let last = w.programs.iter().flat_map(|p| p.seasons.iter().map(|s| s.year)).max().unwrap_or(0);
    let by: HashMap<(usize, i32), usize> = seasons.iter().map(|&(p, y, c)| ((p, y), c)).collect();
    let (mut changes, mut linked, mut changes05, mut linked05) = (0, 0, 0, 0);
    for &(p, y, c) in &seasons {
        if let Some(&b) = by.get(&(p, y - 1)) {
            linked += 1;
            changes += usize::from(b != c);
            if y >= 2005 {
                linked05 += 1;
                changes05 += usize::from(b != c);
            }
        }
    }
    // Runs: consecutive FBS seasons of one man at one program.
    let mut runs: Vec<(i32, i32)> = Vec::new();
    let mut short: BTreeMap<String, usize> = BTreeMap::new();
    let mut sorted = seasons.clone();
    sorted.sort();
    let mut i = 0;
    while i < sorted.len() {
        let (p, a, c) = sorted[i];
        let mut z = a;
        while i + 1 < sorted.len() && sorted[i + 1].0 == p && sorted[i + 1].2 == c && sorted[i + 1].1 == z + 1 {
            i += 1;
            z += 1;
        }
        runs.push((a, z));
        // How the one- and two-season runs ended.
        if z - a <= 1 && a > first && z < last {
            let co = &w.coaches[c];
            let s = co
                .history
                .iter()
                .find(|s| s.employer == Employer::College(p) && s.role == Role::HeadCoach && s.to >= z);
            let why = s.map_or("no head stint (interim)".to_string(), |s| s.ended.to_string());
            *short.entry(format!("{} season(s): {why}", z - a + 1)).or_default() += 1;
        }
        i += 1;
    }
    let mut done: Vec<i32> = runs.iter().filter(|r| r.0 > first && r.1 < last).map(|r| r.1 - r.0 + 1).collect();
    done.sort();
    let median = done.get((done.len().saturating_sub(1)) / 2).copied().unwrap_or(0);
    let early: Vec<i32> = runs.iter().filter(|r| r.0 > first && r.0 <= last - 10).map(|r| r.1 - r.0 + 1).collect();
    let five = early.iter().filter(|&&n| n >= 5).count();
    // Two FBS head jobs: every head stint and current head job at an FBS program.
    let mut jobs: HashMap<usize, HashSet<usize>> = HashMap::new();
    for (c, co) in w.coaches.iter().enumerate() {
        for s in &co.history {
            if s.role == Role::HeadCoach
                && let Employer::College(p) = s.employer
                && division(w, p, s.to) == Some(Division::FBS)
            {
                jobs.entry(c).or_default().insert(p);
            }
        }
        if let Some(j) = co.job
            && j.role == Role::HeadCoach
            && let Employer::College(p) = j.employer
            && w.programs[p].division == Division::FBS
        {
            jobs.entry(c).or_default().insert(p);
        }
    }
    let two = jobs.values().filter(|s| s.len() >= 2).count();
    // Why FBS head jobs ended, and where a fired man went next.
    let mut why: BTreeMap<String, usize> = BTreeMap::new();
    let mut next: BTreeMap<&str, usize> = BTreeMap::new();
    for co in &w.coaches {
        for (i, s) in co.history.iter().enumerate() {
            if s.role == Role::HeadCoach
                && let Employer::College(p) = s.employer
                && division(w, p, s.to) == Some(Division::FBS)
            {
                *why.entry(s.ended.to_string()).or_default() += 1;
                if s.ended.starts_with("fired") || s.ended.starts_with("not retained") {
                    let after =
                        co.history.get(i + 1).map(|x| (x.employer, x.role)).or(co.job.map(|j| (j.employer, j.role)));
                    let k = match after {
                        None => "no job again",
                        Some((Employer::College(q), Role::HeadCoach)) if w.programs[q].division == Division::FBS => {
                            "FBS head coach"
                        }
                        Some((_, Role::HeadCoach)) => "head coach lower or elsewhere",
                        Some((Employer::College(q), _)) if w.programs[q].division == Division::FBS => "FBS assistant",
                        Some((Employer::Nfl(_), _)) => "NFL assistant",
                        Some(_) => "assistant elsewhere",
                    };
                    *next.entry(k).or_default() += 1;
                }
            }
        }
    }
    println!("\n124 FBS head coaches ({first}-{last})");
    println!(
        "  new head coach a season: {:.1}% (2005 on {:.1}%), real 17.6%",
        pct(changes, linked),
        pct(changes05, linked05)
    );
    println!("  finished runs: median {median} (n {}), real 4", done.len());
    println!("  runs lasting five seasons: {:.1}% (n {}), real 43.3%", pct(five, early.len()), early.len());
    let lasting: Vec<String> = [2, 3, 4, 6, 8]
        .iter()
        .map(|&k| format!("{k}+ {:.0}%", pct(early.iter().filter(|&&n| n >= k).count(), early.len())))
        .collect();
    println!("  lasting: {} (real 2+ 92, 3+ 80, 4+ 63, 6+ 32, 8+ 15)", lasting.join(", "));
    println!("  two or more FBS head jobs: {:.1}% of {}, real 30.2%", pct(two, jobs.len()), jobs.len());
    let total: usize = why.values().sum();
    let mut why: Vec<_> = why.into_iter().collect();
    why.sort_by_key(|x| std::cmp::Reverse(x.1));
    println!(
        "  why FBS head jobs ended: {}",
        why.iter().take(8).map(|(k, n)| format!("{k} {:.0}%", pct(*n, total))).collect::<Vec<_>>().join(", ")
    );
    println!("  a fired FBS head coach's next job: {next:?}");
    let mut short: Vec<_> = short.into_iter().collect();
    short.sort_by_key(|x| std::cmp::Reverse(x.1));
    println!("  short runs: {}", short.iter().take(10).map(|(k, n)| format!("{k} {n}")).collect::<Vec<_>>().join(", "));
}

fn who(w: &World) {
    let color = |c: usize| w.coaches[c].ethnicity != Ethnicity::White;
    let black = |c: usize| w.coaches[c].ethnicity == Ethnicity::Black;
    // NFL head coaches by season (`nfl_seasons`), FBS by program season.
    let mut nfl: BTreeMap<i32, (usize, usize)> = BTreeMap::new();
    for s in &w.nfl_seasons {
        if let Some(c) = s.coach {
            let e = nfl.entry(s.year / 5 * 5).or_default();
            e.0 += usize::from(color(c));
            e.1 += 1;
        }
    }
    let mut fbs: BTreeMap<i32, (usize, usize)> = BTreeMap::new();
    for (p, prog) in w.programs.iter().enumerate() {
        for s in &prog.seasons {
            if let Some(c) = s.coach
                && division(w, p, s.year) == Some(Division::FBS)
            {
                let e = fbs.entry(s.year / 5 * 5).or_default();
                e.0 += usize::from(black(c));
                e.1 += 1;
            }
        }
    }
    println!("\n127 NFL head coaches of color, by five seasons (real 2024: 9 of 32, 28%)");
    println!("  {}", nfl.iter().map(|(y, (k, n))| format!("{y}: {:.0}%", pct(*k, *n))).collect::<Vec<_>>().join(", "));
    println!("148 Black FBS head coaches, by five seasons (real 2009 7 of 120, 2023 14 of 133, 2024 16 of 134)");
    println!("  {}", fbs.iter().map(|(y, (k, n))| format!("{y}: {:.1}%", pct(*k, *n))).collect::<Vec<_>>().join(", "));
    // Now, by level and role, the book's S1.ethnicity.
    let mut now: BTreeMap<(String, &str), (usize, usize, usize)> = BTreeMap::new();
    for (c, co) in w.coaches.iter().enumerate() {
        let Some(j) = co.job else { continue };
        let level = match j.employer {
            Employer::College(p) => format!("{:?}", w.programs[p].division),
            Employer::Nfl(_) => "NFL".into(),
            Employer::HighSchool(_) => "HS".into(),
            Employer::Other(_) => continue,
        };
        let role = match j.role {
            Role::HeadCoach => "HC",
            Role::OffensiveCoordinator => "OC",
            Role::DefensiveCoordinator => "DC",
            Role::PositionCoach(_) => "PC",
        };
        let e = now.entry((level, role)).or_default();
        e.0 += usize::from(black(c));
        e.1 += usize::from(color(c));
        e.2 += 1;
    }
    println!("  now (level role: Black, of color, n):");
    for ((l, r), (b, k, n)) in &now {
        println!("    {l} {r}: {:.1}%, {:.1}%, {n}", pct(*b, *n), pct(*k, *n));
    }
    nfl_by_season(w);
    staff_hires(w);
    high_schools(w);
}

/// Bug 244: NFL head coaches of color in each season (of 32), against real
/// counted by name (from memory, as in `afm/race-check/released-worlds.md`).
fn nfl_by_season(w: &World) {
    let real: BTreeMap<i32, usize> = [
        (2005, 6),
        (2006, 7),
        (2007, 6),
        (2009, 6),
        (2011, 8),
        (2013, 4),
        (2015, 6),
        (2016, 6),
        (2017, 8),
        (2018, 8),
        (2019, 4),
        (2021, 5),
        (2022, 6),
        (2023, 6),
        (2024, 9),
        (2025, 7),
    ]
    .into_iter()
    .collect();
    let mut by: BTreeMap<i32, usize> = BTreeMap::new();
    for s in &w.nfl_seasons {
        if let Some(c) = s.coach {
            *by.entry(s.year).or_default() += usize::from(w.coaches[c].ethnicity != Ethnicity::White);
        }
    }
    println!("\n244 NFL head coaches of color each season (world / real)");
    let row: Vec<String> = by
        .iter()
        .filter(|(y, _)| **y >= 2000)
        .map(|(y, k)| match real.get(y) {
            Some(r) => format!("{y} {k}/{r}"),
            None => format!("{y} {k}"),
        })
        .collect();
    println!("  {}", row.join(", "));
    let avg = |a: i32, b: i32| {
        let v: Vec<usize> = by.range(a..=b).map(|(_, k)| *k).collect();
        v.iter().sum::<usize>() as f64 / v.len().max(1) as f64
    };
    println!("  2005-2018 average {:.1} (real 6), 2019-2025 {:.1} (real 6.1)", avg(2005, 2018), avg(2019, 2025));
    // The head coaches' pool: NFL coordinators of color in each season (of 64).
    let mut coords: BTreeMap<i32, (usize, usize)> = BTreeMap::new();
    for co in &w.coaches {
        let color = co.ethnicity != Ethnicity::White;
        let jobs = co
            .history
            .iter()
            .map(|h| (h.employer, h.role, h.from, h.to))
            .chain(co.job.map(|j| (j.employer, j.role, j.since, w.year)));
        for (e, r, from, to) in jobs {
            if matches!(e, Employer::Nfl(_)) && matches!(r, Role::OffensiveCoordinator | Role::DefensiveCoordinator) {
                for y in (from + 1).max(2000)..=to {
                    let k = coords.entry(y).or_default();
                    k.0 += usize::from(color && r == Role::OffensiveCoordinator);
                    k.1 += usize::from(color && r == Role::DefensiveCoordinator);
                }
            }
        }
    }
    let row: Vec<String> = coords.iter().map(|(y, (o, d))| format!("{y} {o}+{d}")).collect();
    println!("  NFL coordinators of color each season (OC+DC): {}", row.join(", "));
}

/// Bugs 161 and 243: who took each FBS staff job, by ten seasons (the
/// coaches' histories hold every stint; a stint's first season is the hire).
fn staff_hires(w: &World) {
    let mut hires: BTreeMap<(i32, &str), (usize, usize, usize)> = BTreeMap::new();
    for co in &w.coaches {
        let jobs = co
            .history
            .iter()
            .map(|h| (h.employer, h.role, h.from))
            .chain(co.job.map(|j| (j.employer, j.role, j.since)));
        for (e, r, from) in jobs {
            let Employer::College(p) = e else { continue };
            if division(w, p, from) != Some(Division::FBS) {
                continue;
            }
            let role = match r {
                Role::HeadCoach => "HC",
                Role::OffensiveCoordinator => "OC",
                Role::DefensiveCoordinator => "DC",
                Role::PositionCoach(_) => "PC",
            };
            let k = hires.entry((from / 10 * 10, role)).or_default();
            k.0 += usize::from(co.ethnicity == Ethnicity::Black);
            k.1 += usize::from(co.ethnicity != Ethnicity::White);
            k.2 += 1;
        }
    }
    // NFL head and coordinator hires by five seasons (real head coach hires
    // of color: about 7% in 1996-2002, about 18% in 2003-2012).
    let mut nfl: BTreeMap<(i32, &str), (usize, usize)> = BTreeMap::new();
    for co in &w.coaches {
        let jobs = co
            .history
            .iter()
            .map(|h| (h.employer, h.role, h.from))
            .chain(co.job.map(|j| (j.employer, j.role, j.since)));
        for (e, r, from) in jobs {
            if !matches!(e, Employer::Nfl(_)) || from < 1996 {
                continue;
            }
            let role = match r {
                Role::HeadCoach => "HC",
                Role::OffensiveCoordinator => "OC",
                Role::DefensiveCoordinator => "DC",
                Role::PositionCoach(_) => "PC",
            };
            let k = nfl.entry((from / 5 * 5, role)).or_default();
            k.0 += usize::from(co.ethnicity != Ethnicity::White);
            k.1 += 1;
        }
    }
    println!("\n244 NFL hires of color by five seasons (of color, n)");
    for ((d, r), (k, n)) in &nfl {
        println!("    {d} {r}: {:.0}%, {n}", pct(*k, *n));
    }
    // FBS position coaches now, by group, and where this decade's came from.
    let mut by_group: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut from: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for co in &w.coaches {
        let Some(j) = co.job else { continue };
        let (Employer::College(p), Role::PositionCoach(g)) = (j.employer, j.role) else { continue };
        if w.programs[p].division != Division::FBS {
            continue;
        }
        let k = by_group.entry(format!("{g:?}")).or_default();
        k.0 += usize::from(co.ethnicity == Ethnicity::Black);
        k.1 += 1;
        if j.since >= 2016 {
            let prev = co.history.iter().rev().find(|h| h.to <= j.since);
            let src = match prev.map(|h| h.employer) {
                None => "new to coaching",
                Some(Employer::HighSchool(_)) => "high school",
                Some(Employer::College(q)) if w.programs[q].division == Division::FBS => "FBS",
                Some(Employer::College(_)) => "smaller college",
                Some(_) => "pro",
            };
            let k = from.entry(src).or_default();
            k.0 += usize::from(co.ethnicity == Ethnicity::Black);
            k.1 += 1;
        }
    }
    let row: Vec<String> = by_group.iter().map(|(g, (b, n))| format!("{g} {:.0}% of {n}", pct(*b, *n))).collect();
    println!("  FBS position coaches Black by group: {}", row.join(", "));
    let row: Vec<String> = from.iter().map(|(g, (b, n))| format!("{g} {n} ({:.0}% Black)", pct(*b, *n))).collect();
    println!("  FBS position coaches hired since 2016 came from: {}", row.join(", "));
    println!(
        "\n161/243 FBS hires by ten seasons (Black, of color, n); real 2022: OC 7-9% Black, DC just over 20%, PC 44% Black and 53% of color"
    );
    for ((d, r), (b, k, n)) in &hires {
        if *d >= 1990 {
            println!("    {d}s {r}: {:.1}%, {:.1}%, {n}", pct(*b, *n), pct(*k, *n));
        }
    }
}

/// Bug 242: high school head coaches by state (Texas 6A 2018: 18% Black,
/// 11% Hispanic, 71% white, Dave Campbell's Texas Football).
fn high_schools(w: &World) {
    let state = |code: &str| afm_engine::world::geo::state_index(code);
    let mut rows: Vec<(String, Vec<usize>)> = Vec::new();
    let heads =
        |hs: &mut dyn Iterator<Item = usize>| -> Vec<usize> { hs.filter_map(|h| w.high_schools[h].coach).collect() };
    rows.push(("all".into(), heads(&mut (0..w.high_schools.len()))));
    for code in ["TX", "CA", "FL", "GA", "OH", "PA", "AL", "LA", "MS", "IL", "MI", "NY", "AZ", "NM", "UT", "MN"] {
        let Some(s) = state(code) else { continue };
        let mut list: Vec<usize> = (0..w.high_schools.len()).filter(|&h| w.high_schools[h].state == s).collect();
        rows.push((code.into(), heads(&mut list.iter().copied())));
        if code == "TX" {
            // Texas's biggest fifth by enrollment (6A is about the biggest 250 of 1,200).
            list.sort_by_key(|&h| std::cmp::Reverse(w.high_schools[h].enrollment));
            let top = list.len().div_ceil(5);
            rows.push(("TX big fifth".into(), heads(&mut list.into_iter().take(top))));
        }
    }
    // Where the head coaches come from: the world's own former players
    // (college, and pros), and the made-up men who never played in college.
    let all: Vec<usize> = w.high_schools.iter().filter_map(|h| h.coach).collect();
    let former: Vec<usize> =
        all.iter().copied().filter(|&c| w.coaches[c].played.is_some_and(|p| p.former.is_some())).collect();
    let pros: Vec<usize> = all.iter().copied().filter(|&c| w.coaches[c].played.is_some_and(|p| p.nfl())).collect();
    let mut came: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for &c in &all {
        let co = &w.coaches[c];
        let src = match co.history.last().map(|h| (h.employer, h.role)) {
            None => "first job",
            Some((Employer::HighSchool(_), Role::HeadCoach)) => "high school head",
            Some((Employer::HighSchool(_), _)) => "high school coordinator",
            Some((Employer::College(_), _)) => "college",
            Some(_) => "pro",
        };
        let k = came.entry(src).or_default();
        k.0 += usize::from(co.ethnicity == Ethnicity::Black);
        k.1 += 1;
    }
    let row: Vec<String> = came.iter().map(|(g, (b, n))| format!("{g} {n} ({:.0}% Black)", pct(*b, *n))).collect();
    println!("  high school head coaches' last job: {}", row.join(", "));
    rows.push(("former players".into(), former));
    rows.push(("ex-pros".into(), pros));
    let coords: Vec<usize> = w.high_schools.iter().flat_map(|h| [h.oc, h.dc]).flatten().collect();
    rows.push(("all coordinators".into(), coords));
    println!("\n242 high school head coaches by state (white, Black, Hispanic, other; n)");
    for (name, cs) in rows {
        let n = cs.len();
        let of = |e: Ethnicity| cs.iter().filter(|&&c| w.coaches[c].ethnicity == e).count();
        let (wh, b, h) = (of(Ethnicity::White), of(Ethnicity::Black), of(Ethnicity::Hispanic));
        println!(
            "    {name}: {:.0}%, {:.0}%, {:.0}%, {:.0}%; {n}",
            pct(wh, n),
            pct(b, n),
            pct(h, n),
            pct(n - wh - b - h, n)
        );
    }
}

fn overlaps(w: &World) {
    let kind = |e: Employer| match e {
        Employer::College(_) => "college",
        Employer::HighSchool(_) => "high school",
        Employer::Nfl(_) => "NFL",
        Employer::Other(_) => "other",
    };
    let mut pairs: BTreeMap<String, usize> = BTreeMap::new();
    let mut heads = 0;
    let mut example = None;
    for co in &w.coaches {
        let h: Vec<_> = co.history.iter().filter(|s| s.role == Role::HeadCoach).collect();
        heads += h.len();
        for (i, a) in h.iter().enumerate() {
            for b in &h[i + 1..] {
                if b.from < a.to {
                    *pairs.entry(format!("{} to {}", kind(a.employer), kind(b.employer))).or_default() += 1;
                    example.get_or_insert(format!("{}: {}-{} then {}-{}", co.name, a.from, a.to, b.from, b.to));
                }
            }
        }
    }
    // Any job (not only head jobs) closed after the next one began, by more than the move's season.
    let mut any = 0;
    for co in &w.coaches {
        for (a, b) in co.history.iter().zip(co.history.iter().skip(1)) {
            any += usize::from(b.from < a.to);
        }
    }
    println!("\n128 head jobs that overlap (the book's S7.no_overlap; {heads} head stints)");
    println!("  pairs: {}, {pairs:?}", pairs.values().sum::<usize>());
    println!("  any two jobs in a row: {any}");
    if let Some(e) = example {
        println!("  e.g. {e}");
    }
}

fn champions(w: &World) {
    let (mut after, mut after_new, mut other, mut other_new) = (0, 0, 0, 0);
    let mut gone: BTreeMap<String, usize> = BTreeMap::new();
    let mut rows: Vec<(usize, u32, u32, f64)> = Vec::new();
    let mut since06 = 0;
    for (h, hs) in w.high_schools.iter().enumerate() {
        let (mut changes, mut titles) = (0, 0);
        for (a, b) in hs.seasons.iter().zip(hs.seasons.iter().skip(1)) {
            let (Some(x), Some(y)) = (a.coach, b.coach) else { continue };
            let new = x != y;
            if a.state_title {
                after += 1;
                after_new += usize::from(new);
                if new {
                    // Why the champion's coach left, and for what.
                    let co = &w.coaches[x];
                    let k = co.history.iter().position(|s| s.employer == Employer::HighSchool(h) && s.to >= a.year);
                    let ended = k.map_or("still there?", |k| &*co.history[k].ended).to_string();
                    let to = k
                        .and_then(|k| co.history.get(k + 1).map(|s| (s.employer, s.role)))
                        .or(co.job.filter(|j| j.employer != Employer::HighSchool(h)).map(|j| (j.employer, j.role)));
                    let to = match to {
                        None => String::new(),
                        Some((Employer::College(q), r)) => {
                            let what = if r == Role::HeadCoach { "head" } else { "staff" };
                            format!(" to {:?} {what}", w.programs[q].division)
                        }
                        Some((Employer::HighSchool(_), _)) => " to another high school".into(),
                        Some((Employer::Nfl(_), _)) => " to the NFL".into(),
                        Some(_) => " elsewhere".into(),
                    };
                    *gone.entry(format!("{ended}{to}")).or_default() += 1;
                }
            } else {
                other += 1;
                other_new += usize::from(new);
            }
            if b.year >= 2006 {
                changes += u32::from(new);
                titles += u32::from(b.state_title);
            }
            since06 = since06.max(b.year);
        }
        if titles >= 8 {
            rows.push((h, changes, titles, hs.prestige));
        }
    }
    rows.sort_by_key(|x| std::cmp::Reverse(x.2));
    println!("\n129 high school champions' head coaches");
    println!(
        "  a new head coach the season after a state title: {:.1}% (n {after}); after no title {:.1}% (n {other})",
        pct(after_new, after),
        pct(other_new, other)
    );
    let mut gone: Vec<_> = gone.into_iter().collect();
    gone.sort_by_key(|x| std::cmp::Reverse(x.1));
    println!("  why: {}", gone.iter().take(10).map(|(k, n)| format!("{k} {n}")).collect::<Vec<_>>().join(", "));
    let n = rows.len();
    let changes: u32 = rows.iter().map(|r| r.1).sum();
    println!("  schools with 8+ titles since 2006: {n}, {:.1} changes each", changes as f64 / n.max(1) as f64);
    for (h, c, t, p) in rows.iter().take(8) {
        println!("    {} ({p:.2}): {t} titles, {c} changes", w.high_schools[*h].name);
    }
}

/// Every coach in a job, one row each, for reading offline (`AFM_COACH_CSV`).
fn coach_csv(w: &World, path: &str) {
    use std::fmt::Write as _;
    let mut out =
        String::from("level,role,group,eth,came_up,played,former,ex_pro,age,since,prev,prev_role,state,private\n");
    for co in &w.coaches {
        let Some(j) = co.job else { continue };
        let (level, state, private) = match j.employer {
            Employer::College(p) => (format!("{:?}", w.programs[p].division), w.programs[p].state as i64, false),
            Employer::Nfl(_) => ("NFL".into(), -1, false),
            Employer::HighSchool(h) => ("HS".into(), w.high_schools[h].state as i64, w.high_schools[h].private),
            Employer::Other(_) => ("Other".into(), -1, false),
        };
        let (role, group) = match j.role {
            Role::HeadCoach => ("HC", String::new()),
            Role::OffensiveCoordinator => ("OC", String::new()),
            Role::DefensiveCoordinator => ("DC", String::new()),
            Role::PositionCoach(g) => ("PC", format!("{g:?}")),
        };
        let played = co.played.map(|p| format!("{:?}", p.group())).unwrap_or_default();
        let former = co.played.is_some_and(|p| p.former.is_some());
        let prev = co.history.last();
        let prev_kind = match prev.map(|h| h.employer) {
            None => "none".to_string(),
            Some(Employer::College(q)) => format!("{:?}", w.programs[q].division),
            Some(Employer::HighSchool(_)) => "HS".into(),
            Some(Employer::Nfl(_)) => "NFL".into(),
            Some(Employer::Other(_)) => "Other".into(),
        };
        let prev_role = prev.map_or("", |h| match h.role {
            Role::HeadCoach => "HC",
            Role::OffensiveCoordinator => "OC",
            Role::DefensiveCoordinator => "DC",
            Role::PositionCoach(_) => "PC",
        });
        let _ = writeln!(
            out,
            "{level},{role},{group},{:?},{:?},{played},{former},{},{},{},{prev_kind},{prev_role},{state},{private}",
            co.ethnicity, co.came_up, co.ex_pro, co.age, j.since
        );
    }
    std::fs::write(path, out).expect("coach csv");
}
