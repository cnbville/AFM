//! Spring and summer roster moves (lane B of the Jan 1 start): builds a
//! world, plays it on day by day and prints, for each offseason, the portal
//! by the month entrants landed, who was still looking at fall camp, the
//! late signings of seniors and junior college players after February's
//! signing day, walk-on tryouts and scholarships, and every level's squads
//! at fall camp against real, then the log's own lane B counters.
//! Usage: roster_moves [scale] [history_years] [seed] [seasons] [engine_from]

use afm_engine::calendar::{Date, YearDates};
use afm_engine::world::data::Division;
use afm_engine::world::{Stage, World, WorldOptions};

const LEVELS: [Division; 6] =
    [Division::FBS, Division::FCS, Division::D2, Division::NAIA, Division::D3, Division::JUCO];

/// A real full squad at each level, walk-ons included (the engine's own
/// `level_roster`; FBS has the House limit of 105 from 2025).
fn real_squad(d: Division, season: i32) -> f64 {
    match d {
        Division::FBS if season >= 2025 => 105.0,
        Division::FBS => 119.7,
        Division::FCS => 108.0,
        Division::D2 => 115.0,
        Division::D3 => 107.0,
        Division::NAIA => 100.0,
        Division::JUCO => 80.0,
    }
}

#[derive(Default)]
struct Offseason {
    /// Seniors still unsigned and in high school the day after signing day.
    late_pool: usize,
    /// Signings after signing day, by level of the school: [high school, JUCO mover].
    late: [[u32; 2]; 6],
    late_by_month: [u32; 12],
    /// Portal entrants still looking at fall camp.
    pending_at_camp: usize,
    squads: Vec<(Division, usize, usize)>,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let scale: f64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.25);
    let years: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(3);
    let seed: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(74);
    let seasons: i32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(1);
    let engine_from: i32 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(2100);
    let t = std::time::Instant::now();
    let mut w = World::new(WorldOptions {
        seed,
        scale,
        history_years: years,
        engine_from: Some(engine_from),
        hs_engine_years: None,
        ..Default::default()
    });
    println!("built in {:.0}s; today {} (world year {})", t.elapsed().as_secs_f64(), w.today.pretty(), w.year);
    let first = w.year;
    let mut seen: Vec<(i32, Offseason)> = Vec::new();
    let mut cur = Offseason::default();
    let end = Date::ymd(first + seasons, 2, 9);
    let mut counted_late: std::collections::HashSet<u32> = std::collections::HashSet::new();
    let start = w.today;
    while w.today < end {
        let d = w.today;
        let y = w.dates.clone();
        w.advance_day();
        // The evening after each step.
        if d == y.signing_day.add(1) || d == start {
            let class = if d.month() <= 2 { d.year() } else { d.year() + 1 };
            cur.late_pool = w
                .people
                .iter()
                .filter(|p| p.stage == Stage::HighSchool && p.class_of == class && p.cal.signed.is_none())
                .count();
        }
        // Late signings: a signature dated after February's signing day.
        for p in w.people.iter() {
            if let Some(s) = p.cal.signed
                && s.date > YearDates::new(s.date.year() - 1).signing_day
                && s.date.month() <= 8
                && counted_late.insert(p.player.id)
            {
                let lvl = LEVELS.iter().position(|&l| l == w.programs[s.program].division).unwrap();
                let juco = p.stage == Stage::College;
                cur.late[lvl][juco as usize] += 1;
                cur.late_by_month[s.date.month() as usize - 1] += 1;
            }
        }
        if d == y.fall_camp {
            cur.pending_at_camp = w.people.iter().filter(|p| p.stage == Stage::College && p.cal.portal.is_some()).count();
            cur.squads = w
                .programs
                .iter()
                .enumerate()
                .filter(|(_, p)| p.plays(w.year))
                .map(|(j, prog)| {
                    let mut n = 0;
                    let mut walk = 0;
                    for &i in &prog.roster {
                        let p = &w.people[i];
                        if p.stage == Stage::College && p.college == Some(j) {
                            n += 1;
                            walk += p.walk_on as usize;
                        }
                    }
                    (prog.division, n, walk)
                })
                .collect();
            seen.push((w.year, std::mem::take(&mut cur)));
        }
    }
    println!("played on to {} in {:.0}s", w.today.pretty(), t.elapsed().as_secs_f64());
    for (season, o) in &seen {
        report(&w, *season, o);
    }
}

fn report(w: &World, season: i32, o: &Offseason) {
    let label = season - 1;
    println!("=== offseason before the {season} season ===");
    // The portal cycle that feeds this season.
    let recs: Vec<_> = w.recruiting_log.portal.iter().filter(|r| r.year == label).collect();
    let mut by_month = [0u32; 12];
    let (mut landed, mut returned, mut left) = (0, 0, 0);
    let mut by_from = [[0u32; 3]; 6];
    let mut fbs_to = [0u32; 7];
    for r in &recs {
        let f = LEVELS.iter().position(|&l| l == r.from).unwrap();
        match (r.to, r.returned) {
            (_, true) => {
                returned += 1;
                by_from[f][1] += 1;
            }
            (Some(_), false) => {
                landed += 1;
                by_from[f][0] += 1;
                by_month[r.decided.month() as usize - 1] += 1;
            }
            (None, false) => {
                left += 1;
                by_from[f][2] += 1;
            }
        }
        if r.from == Division::FBS {
            let k = match (r.to, r.returned) {
                (_, true) => 6,
                (Some(d), _) => LEVELS.iter().position(|&l| l == d).unwrap(),
                (None, _) => 6,
            };
            fbs_to[k] += 1;
        }
    }
    let n = recs.len().max(1) as f64;
    println!(
        "portal {label}: {} decisions, landed {landed} ({:.0}%), returned {returned} ({:.0}%), left football {left} ({:.0}%)",
        recs.len(),
        100.0 * landed as f64 / n,
        100.0 * returned as f64 / n,
        100.0 * left as f64 / n
    );
    let months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let row: Vec<String> = months.iter().zip(by_month).filter(|x| x.1 > 0).map(|(m, k)| format!("{m} {k}")).collect();
    println!("  landed by month: {}", row.join(", "));
    for (k, l) in LEVELS.iter().enumerate() {
        let [a, b, c] = by_from[k];
        let t = (a + b + c).max(1) as f64;
        if a + b + c > 0 {
            println!(
                "  from {:<5} {:>5} entrants: landed {:.0}%, returned {:.0}%, left {:.0}%",
                l.name(),
                a + b + c,
                100.0 * a as f64 / t,
                100.0 * b as f64 / t,
                100.0 * c as f64 / t
            );
        }
    }
    let ft = fbs_to.iter().sum::<u32>().max(1) as f64;
    println!(
        "  FBS entrants to: FBS {:.0}%, FCS {:.0}%, D2 {:.0}%, NAIA {:.0}%, D3 {:.0}%, JUCO {:.0}%, none or back {:.0}%",
        100.0 * fbs_to[0] as f64 / ft,
        100.0 * fbs_to[1] as f64 / ft,
        100.0 * fbs_to[2] as f64 / ft,
        100.0 * fbs_to[3] as f64 / ft,
        100.0 * fbs_to[4] as f64 / ft,
        100.0 * fbs_to[5] as f64 / ft,
        100.0 * fbs_to[6] as f64 / ft
    );
    println!("  still in the portal at fall camp: {}", o.pending_at_camp);
    println!("late pool (unsigned seniors still in school after signing day): {}", o.late_pool);
    let total: u32 = o.late.iter().map(|x| x[0] + x[1]).sum();
    let row: Vec<String> = LEVELS
        .iter()
        .zip(o.late)
        .filter(|x| x.1[0] + x.1[1] > 0)
        .map(|(l, [h, j])| format!("{} {h}+{j}", l.name()))
        .collect();
    println!("late signings after signing day: {total} (high school + JUCO): {}", row.join(", "));
    let row: Vec<String> =
        months.iter().zip(o.late_by_month).filter(|x| x.1 > 0).map(|(m, k)| format!("{m} {k}")).collect();
    println!("  by month: {}", row.join(", "));
    if let Some(l) = w.log_history.iter().find(|l| l.year == season).or((w.log.year == season).then_some(&w.log)) {
        println!("walk-ons put on scholarship {} / open tryouts {}", l.walk_on_scholarships, l.tryouts);
        println!(
            "log: late pool {}, signed after signing day {:?}, portal landed after signing day {}, unplaced at fall camp {}",
            l.late_pool, l.signed_after, l.portal_landed_after, l.portal_unplaced_at_camp
        );
    }
    println!("squads at fall camp (mean, p10-p90, walk-ons, real):");
    for l in LEVELS {
        let mut v: Vec<&(Division, usize, usize)> = o.squads.iter().filter(|s| s.0 == l).collect();
        if v.is_empty() {
            continue;
        }
        v.sort_by_key(|s| s.1);
        let mean = v.iter().map(|s| s.1).sum::<usize>() as f64 / v.len() as f64;
        let walk = v.iter().map(|s| s.2).sum::<usize>() as f64 / v.len() as f64;
        let pct = |q: f64| v[((v.len() - 1) as f64 * q) as usize].1;
        println!(
            "  {:<5} {:>4} teams: {:>6.1} ({}-{}), walk-ons {:>5.1}, real {:.0}",
            l.name(),
            v.len(),
            mean,
            pct(0.1),
            pct(0.9),
            walk,
            real_squad(l, season)
        );
    }
}
