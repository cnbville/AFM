//! NFL head coaches' hot seats and firings during the season (`staff::midseason`):
//! builds a small world on the stand-in, plays it on and prints every
//! seat that went hot, every head coach fired in season with his season in
//! the job, the games played and the days of warning he had, and the
//! totals a season against the real ones (2008-24: 36 in 17 seasons, 3 in
//! a first season, 6 in a second, 27 later; 9 after four or five games).
//! Usage: seat_check [scale] [history_years] [seed] [seasons]
//! `AFM_SEAT_ROWS=file` writes every owner's read on every Monday (tab
//! separated) for sizing the knobs away from the world.

use afm_engine::calendar::Weekday;
use afm_engine::world::news::NewsKind;
use afm_engine::world::{World, WorldOptions};
use std::collections::BTreeMap;
use std::io::Write;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let scale: f64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.02);
    let years: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(2);
    let seed: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(391);
    let seasons: i32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(20);
    let t0 = std::time::Instant::now();
    let mut w =
        World::new(WorldOptions { seed, scale, history_years: years, engine_from: Some(2100), ..Default::default() });
    let first = w.year;
    println!(
        "world: scale {scale}, {years} years of history, seed {seed}, built in {:.0} s",
        t0.elapsed().as_secs_f64()
    );
    let mut rows = std::env::var("AFM_SEAT_ROWS").ok().and_then(|p| std::fs::File::create(p).ok());
    if let Some(f) = rows.as_mut() {
        let _ = writeln!(f, "season\tday\tteam\tcoach\tnth\twins\tlosses\tties\texpect\tlast\thurry\tbad\traw\thot");
    }
    // Head coach -> the day his seat went hot (a club can move and change
    // its name while he sits on it).
    let mut hot: BTreeMap<String, afm_engine::calendar::Date> = BTreeMap::new();
    // Seats carried in before the first day read here.
    for x in &w.folk.staff.seats {
        hot.insert(w.coaches[x.coach as usize].name.clone(), x.since);
    }
    let mut seen = w.news.len();
    let mut fired: Vec<(i32, u32, u32, i64, String)> = Vec::new();
    let (mut warmed, mut carried, mut cooled, mut coords) = (0u32, 0u32, 0u32, 0u32);
    let end = first + seasons;
    while w.year < end {
        w.advance_day();
        if w.news.len() < seen {
            seen = 0;
        }
        let today = w.today;
        if today.weekday() == Weekday::Mon
            && let Some(f) = rows.as_mut()
        {
            for r in w.nfl_seat_reads() {
                let _ = writeln!(
                    f,
                    "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{}",
                    w.dates.season,
                    today,
                    w.nfl[r.team].name,
                    w.coaches[r.coach].name,
                    r.nth,
                    r.wins,
                    r.losses,
                    r.ties,
                    r.expect,
                    r.last,
                    r.hurry,
                    r.bad,
                    r.raw,
                    r.hot.is_some() as u8
                );
            }
        }
        for n in &w.news[seen..] {
            let text = &n.text;
            let coach_before = |word: &str| {
                let name = text.split("head coach ").nth(1).and_then(|x| x.split(word).next()).unwrap_or("");
                name.to_string()
            };
            match n.kind {
                NewsKind::CoachSeat => {
                    println!("  {today}  {text}");
                    if text.contains("on the hot seat in his") {
                        warmed += 1;
                        hot.insert(coach_before(" is on the hot seat"), today);
                    } else if text.contains("starts it on the hot seat") {
                        carried += 1;
                        hot.insert(coach_before(" for a "), today);
                    } else {
                        cooled += 1;
                        hot.remove(&coach_before("'s job has quieted"));
                    }
                }
                NewsKind::CoachFired if text.contains(" fire head coach ") && text.contains("games into his") => {
                    println!("  {today}  {text}");
                    let num = |after: &str| -> u32 {
                        text.split(after)
                            .next()
                            .and_then(|x| x.split_whitespace().last())
                            .and_then(|x| x.trim_end_matches(|c: char| c.is_alphabetic()).parse().ok())
                            .unwrap_or(0)
                    };
                    let games = num(" games into his");
                    let nth = text
                        .split(" games into his ")
                        .nth(1)
                        .and_then(|x| x.split_whitespace().next())
                        .and_then(|x| x.trim_end_matches(|c: char| c.is_alphabetic()).parse().ok())
                        .unwrap_or(0);
                    let name = coach_before(" games into his");
                    let name = name.rsplit_once(' ').map_or("", |x| x.0);
                    let warned = hot.remove(name).map_or(-1, |d| d.days_to(today) as i64);
                    fired.push((w.dates.season, games, nth, warned, text.clone()));
                }
                NewsKind::CoachFired if text.contains(" fire ") && text.contains("games into the season") => {
                    coords += 1;
                }
                _ => {}
            }
        }
        seen = w.news.len();
    }
    let n = seasons as f64;
    println!("\n{seasons} seasons from {first}, in {:.0} s", t0.elapsed().as_secs_f64());
    println!(
        "head coaches fired in season: {} ({:.2} a season; real 2008-24: 2.1)",
        fired.len(),
        fired.len() as f64 / n
    );
    let by = |f: &dyn Fn(u32) -> bool| fired.iter().filter(|x| f(x.2)).count();
    let all = fired.len().max(1) as f64;
    println!(
        "  by his season in the job: first {} ({:.0}%; real 8%), second {} ({:.0}%; real 17%), later {} ({:.0}%; real 75%)",
        by(&|x| x <= 1),
        100.0 * by(&|x| x <= 1) as f64 / all,
        by(&|x| x == 2),
        100.0 * by(&|x| x == 2) as f64 / all,
        by(&|x| x >= 3),
        100.0 * by(&|x| x >= 3) as f64 / all
    );
    let g = |lo: u32, hi: u32| fired.iter().filter(|x| x.1 >= lo && x.1 <= hi).count();
    println!(
        "  by games played: 4-5 {} ({:.0}%; real 25%), 6-9 {} ({:.0}%; real 22%), 10-13 {} ({:.0}%; real 33%), 14+ {} ({:.0}%; real 20%)",
        g(4, 5),
        100.0 * g(4, 5) as f64 / all,
        g(6, 9),
        100.0 * g(6, 9) as f64 / all,
        g(10, 13),
        100.0 * g(10, 13) as f64 / all,
        g(14, 99),
        100.0 * g(14, 99) as f64 / all
    );
    let first_early = fired.iter().filter(|x| x.2 <= 1 && x.1 < 10).count();
    println!("  in a first season before game 10: {first_early} (real 0)");
    let unwarned = fired.iter().filter(|x| x.3 < 7).count();
    let days: Vec<i64> = fired.iter().filter(|x| x.3 >= 0).map(|x| x.3).collect();
    println!(
        "  with under a week on a public hot seat: {unwarned}; days of warning: shortest {}, median {}",
        days.iter().min().copied().unwrap_or(0),
        {
            let mut d = days.clone();
            d.sort();
            d.get(d.len() / 2).copied().unwrap_or(0)
        }
    );
    println!(
        "seats: {warmed} went hot in season ({:.1} a season), {carried} carried in from Black Monday ({:.1} a season), {cooled} cooled",
        warmed as f64 / n,
        carried as f64 / n
    );
    println!("coordinators fired in season: {coords} ({:.2} a season; real about 2)", coords as f64 / n);
}
