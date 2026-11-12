//! How much NIL money moves a recruit's and a transfer's choice (playtest
//! item F3, Cain 2026-11-12: an SEC school offering double the field's
//! money wasn't in a recruit's first ten). For every recruit of the class
//! signing next and every power-conference school on his list, the school's
//! money is set to 0, 1, 1.5, 2 and 3 times the best money any other school
//! on his list offers, and its place on his list is read again by the same
//! appeal his race and his signing day weigh (`recruiting::appeal`). The
//! same for portal entrants in an open window (`recruiting::portal_race`).
//! Results by what he weighs: money first of his six reasons, among his
//! first three, or lower.
//!
//! Usage: nil_pull play <world> <out dir> [recruit date] [portal date]
//!          plays the world on (games on the stand-in) and saves it on each
//!          date (`recruits.afmworld`, `portal.afmworld`), so a change is
//!          measured on the very same recruits and entrants.
//!        nil_pull recruits <world> [conference]
//!        nil_pull portal <world> [conference] [most entrants]
//! Dates are yyyy-mm-dd. The conference (default SEC) names the schools
//! whose money is moved; "power" takes every power-conference school.

use afm_engine::calendar::Date;
use afm_engine::debug;
use afm_engine::world::front_office::college as desk;
use afm_engine::world::{Stage, World};
use std::path::Path;

const TIMES: [f64; 5] = [0.0, 1.0, 1.5, 2.0, 3.0];

fn date(s: &str) -> Date {
    let v: Vec<i32> = s.split('-').map(|x| x.parse().expect("yyyy-mm-dd")).collect();
    Date::ymd(v[0], v[1] as u32, v[2] as u32)
}

fn read(path: &str) -> World {
    let t = std::time::Instant::now();
    let w = afm_engine::save::read_world(Path::new(path)).expect("read the world");
    eprintln!("read {path} in {:.0}s: {}", t.elapsed().as_secs_f64(), w.today.pretty());
    w
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("play") => {
            let mut w = read(&args[2]);
            w.options.engine_from = Some(2100);
            let out = Path::new(&args[3]);
            std::fs::create_dir_all(out).expect("the folder");
            let season = w.today.year();
            let first = args.get(4).map_or(Date::ymd(season, 11, 15), |s| date(s));
            let second = args.get(5).map_or(Date::ymd(season + 1, 1, 8), |s| date(s));
            let t = std::time::Instant::now();
            for (day, name) in [(first, "recruits"), (second, "portal")] {
                while w.today < day {
                    w.advance_day();
                }
                let to = out.join(format!("{name}.afmworld"));
                afm_engine::save::write_world(&w, &to, "nil_pull", "nil_pull", None).expect("write");
                eprintln!("{} saved on {} at {:.0}s", to.display(), w.today.pretty(), t.elapsed().as_secs_f64());
            }
        }
        Some("recruits") => recruits(&read(&args[2]), args.get(3).map_or("SEC", String::as_str)),
        Some("portal") => portal(
            &read(&args[2]),
            args.get(3).map_or("SEC", String::as_str),
            args.get(4).and_then(|s| s.parse().ok()).unwrap_or(usize::MAX),
        ),
        _ => eprintln!("usage: nil_pull play <world> <out dir> | recruits <world> [conference] | portal <world> [conference]"),
    }
}

/// Whether a program is one of the schools whose money the check moves.
fn picked(w: &World, j: usize, conference: &str) -> bool {
    let p = &w.programs[j];
    if conference == "power" { p.power_conference(w.year) } else { p.conference == conference }
}

/// One (player, school) pair: what he weighs, and the school's place on his
/// list at each multiple of the field's best money.
struct Pair {
    /// Money's place among his six reasons (0: first).
    money_rank: usize,
    /// Money's own weight in his choice.
    money_w: f64,
    stars: u8,
    committed: bool,
    list: usize,
    /// The field's best money, and the school's own today.
    field: f64,
    own: f64,
    /// The school's place today, at its own money.
    place: usize,
    /// Its place at each of `TIMES`.
    at: [usize; 5],
    /// Its appeal less the best other school's, at each of `TIMES`.
    gap: [f64; 5],
    /// The spread (standard deviation) of the list's appeals.
    spread: f64,
}

fn money_place(weights: &[f64; 6]) -> usize {
    weights.iter().filter(|&&x| x > weights[2]).count()
}

fn sd(v: &[f64]) -> f64 {
    let n = v.len().max(1) as f64;
    let m = v.iter().sum::<f64>() / n;
    (v.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / n).sqrt()
}

fn recruits(w: &World, conference: &str) {
    let class = desk::next_class(w);
    println!("# Recruits of the class of {class} on {} ({conference} schools' money moved)", w.today.pretty());
    let mut pairs: Vec<Pair> = Vec::new();
    let (mut lists, mut sizes) = (0usize, 0usize);
    for (i, p) in w.people.iter().enumerate() {
        if p.stage != Stage::HighSchool || p.class_of != class || p.cal.signed.is_some() {
            continue;
        }
        let mut schools: Vec<usize> =
            p.cal.offers.iter().copied().chain(p.cal.early_offers.iter().map(|o| o.0)).collect();
        if let Some(c) = p.cal.commit {
            schools.push(c.0);
        }
        schools.sort_unstable();
        schools.dedup();
        if schools.len() < 5 {
            continue;
        }
        lists += 1;
        sizes += schools.len();
        let base: Vec<(usize, f64, f64)> =
            schools.iter().map(|&j| (j, debug::recruit_appeal(w, i, j, None), debug::recruit_money(w, j, i))).collect();
        let weights = desk::reason_weights(w, i);
        let spread = sd(&base.iter().map(|b| b.1).collect::<Vec<_>>());
        for &(j, appeal, own) in &base {
            if !picked(w, j, conference) || p.cal.commit.is_some_and(|c| c.0 == j) {
                continue;
            }
            let others = || base.iter().filter(move |b| b.0 != j);
            let field = others().map(|b| b.2).fold(0.0, f64::max);
            if field < 1_000.0 {
                continue;
            }
            let best = others().map(|b| b.1).fold(f64::NEG_INFINITY, f64::max);
            let place_of = |a: f64| 1 + others().filter(|b| b.1 > a).count();
            let mut at = [0; 5];
            let mut gap = [0.0; 5];
            for (k, t) in TIMES.iter().enumerate() {
                let a = debug::recruit_appeal(w, i, j, Some(t * field));
                at[k] = place_of(a);
                gap[k] = a - best;
            }
            pairs.push(Pair {
                money_rank: money_place(&weights),
                money_w: weights[2],
                stars: p.stars,
                committed: p.cal.commit.is_some(),
                list: schools.len(),
                field,
                own,
                place: place_of(appeal),
                at,
                gap,
                spread,
            });
        }
    }
    println!(
        "{lists} recruits with five or more schools after them (a list of {:.1} on average); {} pairs with a {conference} school",
        sizes as f64 / lists.max(1) as f64,
        pairs.len()
    );
    report(&pairs);
}

fn portal(w: &World, conference: &str, most: usize) {
    println!("# Portal entrants on {} ({conference} schools' money moved)", w.today.pretty());
    let entrants: Vec<usize> = w
        .people
        .iter()
        .enumerate()
        .filter(|(_, p)| p.stage == Stage::College && p.cal.portal.is_some_and(|e| !e.held && e.entered <= w.today))
        .map(|(i, _)| i)
        .collect();
    let mut pairs: Vec<Pair> = Vec::new();
    let (mut lists, mut sizes, mut seen) = (0usize, 0usize, 0usize);
    for &i in &entrants {
        let base = debug::portal_race_with(w, i, None);
        if base.len() < 3 {
            continue;
        }
        lists += 1;
        sizes += base.len();
        if !base.iter().any(|b| picked(w, b.0, conference)) {
            continue;
        }
        seen += 1;
        if seen > most {
            break;
        }
        let p = &w.people[i];
        let weights = debug::transfer_weights(w, i);
        let spread = sd(&base.iter().map(|b| b.1).collect::<Vec<_>>());
        for (n, &(j, _, own)) in base.iter().enumerate() {
            if !picked(w, j, conference) {
                continue;
            }
            let field = base.iter().filter(|b| b.0 != j).map(|b| b.2).fold(0.0, f64::max);
            if field < 1_000.0 {
                continue;
            }
            let mut at = [0; 5];
            let mut gap = [0.0; 5];
            for (k, t) in TIMES.iter().enumerate() {
                let race = debug::portal_race_with(w, i, Some((j, t * field)));
                let at_j = race.iter().position(|r| r.0 == j).expect("still after him");
                at[k] = at_j + 1;
                let best = race.iter().filter(|r| r.0 != j).map(|r| r.1).fold(f64::NEG_INFINITY, f64::max);
                gap[k] = race[at_j].1 - best;
            }
            pairs.push(Pair {
                money_rank: money_place(&weights),
                money_w: weights[2],
                stars: p.stars,
                committed: false,
                list: base.len(),
                field,
                own,
                place: n + 1,
                at,
                gap,
                spread,
            });
        }
    }
    println!(
        "{} entrants shopping, {lists} with three or more programs after them (a list of {:.1} on average); {} pairs with a {conference} program",
        entrants.len(),
        sizes as f64 / lists.max(1) as f64,
        pairs.len()
    );
    report(&pairs);
}

fn median(v: &mut [f64]) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    v[v.len() / 2]
}

fn mean(v: impl Iterator<Item = f64>) -> f64 {
    let (mut s, mut n) = (0.0, 0usize);
    for x in v {
        s += x;
        n += 1;
    }
    if n == 0 { f64::NAN } else { s / n as f64 }
}

fn table(name: &str, v: &[&Pair]) {
    if v.is_empty() {
        println!("\n## {name}: none");
        return;
    }
    let n = v.len();
    println!(
        "\n## {name}: {n} pairs; list {:.1}; the field's best money ${:.0}K (median), the school's own ${:.0}K; money's weight {:.3}; the list's spread {:.3}",
        mean(v.iter().map(|p| p.list as f64)),
        median(&mut v.iter().map(|p| p.field / 1e3).collect::<Vec<_>>()),
        median(&mut v.iter().map(|p| p.own / 1e3).collect::<Vec<_>>()),
        mean(v.iter().map(|p| p.money_w)),
        mean(v.iter().map(|p| p.spread)),
    );
    println!("| money | median place | mean place | first | top 3 | top 5 | top 10 | behind the leader |");
    println!("|---|---|---|---|---|---|---|---|");
    let share = |f: &dyn Fn(&Pair) -> bool| 100.0 * v.iter().filter(|p| f(p)).count() as f64 / n as f64;
    let row = |label: &str, at: &dyn Fn(&Pair) -> usize, gap: Option<usize>| {
        println!(
            "| {label} | {:.0} | {:.1} | {:.0}% | {:.0}% | {:.0}% | {:.0}% | {} |",
            median(&mut v.iter().map(|p| at(p) as f64).collect::<Vec<_>>()),
            mean(v.iter().map(|p| at(p) as f64)),
            share(&|p| at(p) == 1),
            share(&|p| at(p) <= 3),
            share(&|p| at(p) <= 5),
            share(&|p| at(p) <= 10),
            gap.map_or("".into(), |k| format!("{:+.3}", mean(v.iter().map(|p| p.gap[k])))),
        );
    };
    row("its own today", &|p| p.place, None);
    for (k, t) in TIMES.iter().enumerate() {
        row(&format!("{t}x the field"), &|p| p.at[k], Some(k));
    }
}

fn report(pairs: &[Pair]) {
    let all: Vec<&Pair> = pairs.iter().collect();
    table("Everyone", &all);
    let open: Vec<&Pair> = pairs.iter().filter(|p| !p.committed).collect();
    if open.len() != all.len() {
        table("Not committed anywhere", &open);
        table("Committed to another school", &pairs.iter().filter(|p| p.committed).collect::<Vec<_>>());
    }
    println!("\n# By what he weighs (not committed anywhere)");
    table("Money is his first reason", &open.iter().copied().filter(|p| p.money_rank == 0).collect::<Vec<_>>());
    table(
        "Money is his second or third reason",
        &open.iter().copied().filter(|p| (1..=2).contains(&p.money_rank)).collect::<Vec<_>>(),
    );
    table("Money is fourth or lower", &open.iter().copied().filter(|p| p.money_rank >= 3).collect::<Vec<_>>());
    println!("\n# By stars (not committed anywhere)");
    for s in [5u8, 4, 3] {
        table(&format!("{s}-stars"), &open.iter().copied().filter(|p| p.stars == s).collect::<Vec<_>>());
    }
    table("2-stars and lower", &open.iter().copied().filter(|p| p.stars <= 2).collect::<Vec<_>>());
    // Cain's case: a recruit with many schools after him.
    table("Lists of 15 schools or more", &open.iter().copied().filter(|p| p.list >= 15).collect::<Vec<_>>());
}
