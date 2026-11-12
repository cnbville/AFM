//! Plays a world file on for a number of days and writes it out, for
//! measuring a fix on a real world's league (L12's fix list): the same
//! code paths as the game, with every game on the stand-in unless
//! `engine` is given (quick checks only).
//! Usage: play_on <world file> <days> <out file> [engine]

use std::path::Path;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let t = Instant::now();
    let mut w = afm_engine::save::read_world(Path::new(&args[1])).expect("read the world");
    eprintln!("read {:?} in {:.0} s, {:?}", args[1], t.elapsed().as_secs_f64(), w.today);
    if args.get(4).map(String::as_str) != Some("engine") {
        w.options.engine_from = Some(2100);
    }
    let days: u32 = args[2].parse().expect("days");
    let t = Instant::now();
    // Player trades by month (minor bug 87): real 2021-2025 has about 14 in
    // March and 19 in August (trades.csv).
    let mut months: std::collections::BTreeMap<String, u32> = Default::default();
    let traded = |w: &afm_engine::world::World| w.market.years.iter().map(|m| m.trades).sum::<u32>();
    // Minor bug 290: each year's combine edges and tackles by arm length,
    // then where they went in the draft.
    let mut invited: Vec<(usize, u32, f64)> = Vec::new();
    let mut arm_rows: Vec<(String, f64, Option<u32>)> = Vec::new();
    for d in 0..days {
        if w.today == w.dates.combine_end {
            invited = w
                .people
                .iter()
                .enumerate()
                .filter(|(_, p)| {
                    p.cal.combine_invite
                        && matches!(
                            p.player.position,
                            afm_engine::positions::Position::EDGE | afm_engine::positions::Position::OT
                        )
                })
                .map(|(i, p)| (i, p.player.position.index() as u32, p.player.body.arm))
                .collect();
        }
        if w.today == w.dates.udfa_rush.date && !invited.is_empty() {
            for &(i, pos, arm) in &invited {
                let pick = w.people[i].pro.as_ref().and_then(|c| c.draft).filter(|d| d.0 == w.year).map(|d| d.2);
                arm_rows.push((afm_engine::positions::Position::ALL[pos as usize].to_string(), arm, pick));
            }
            invited.clear();
        }
        let before = traded(&w);
        if w.today == afm_engine::world::scouting::supp::date(w.year) {
            eprintln!("supplemental entrants this July: {}", afm_engine::world::scouting::supp::eligible(&w).len());
        }
        w.advance_day();
        let n = traded(&w) - before;
        if n > 0 {
            *months.entry(format!("{}-{:02}", w.today.year(), w.today.month())).or_default() += n;
        }
        // The rosters in the regular season's last week, before Black
        // Monday's futures deals (minor bug 122): rostered, practice squad
        // and reserve players a club by position.
        if w.dates.nfl_weeks.last().is_some_and(|&s| w.today == s) {
            let mut by: std::collections::BTreeMap<String, [f64; 3]> = Default::default();
            for p in w.people.iter().filter(|p| p.stage == afm_engine::world::Stage::Pro) {
                let Some(c) = p.pro.as_ref().filter(|c| c.team.is_some()) else { continue };
                let k = match c.status {
                    afm_engine::world::pro::Status::Roster => 0,
                    afm_engine::world::pro::Status::PracticeSquad => 1,
                    afm_engine::world::pro::Status::Reserve => 2,
                    _ => continue,
                };
                by.entry(p.player.position.to_string()).or_default()[k] += 1.0 / 32.0;
            }
            for (pos, v) in &by {
                eprintln!("ROSTER {pos} {:.2} {:.2} {:.2}", v[0], v[1], v[2]);
            }
        }
        // Cap room in camp (minor bug 87's depth trades): clubs by room.
        if w.today == w.dates.veterans_report.add(7) {
            let mut room: Vec<f64> =
                (0..w.clubs.len()).map(|t| afm_engine::world::pro::space(&w, t, w.year) / 1e6).collect();
            room.sort_by(f64::total_cmp);
            eprintln!("CAMP ROOM $M {:?}", room.iter().map(|x| x.round()).collect::<Vec<_>>());
        }
        // Who is still unsigned at cutdown (minor bug 95): good veterans.
        if w.today == w.dates.cutdown.date {
            let fa: Vec<String> = w
                .people
                .iter()
                .filter(|p| p.stage == afm_engine::world::Stage::Pro)
                .filter(|p| p.pro.as_ref().is_some_and(|c| c.status == afm_engine::world::pro::Status::FreeAgent))
                .filter(|p| p.player.overall() >= 78.0)
                .map(|p| format!("{} {} {:.0}", p.player.position, p.player.age, p.player.overall()))
                .collect();
            eprintln!("UNSIGNED 78+ at cutdown: {} {:?}", fa.len(), fa);
        }
        if d % 30 == 29 {
            eprintln!("day {} {:?} {:.0} s", d + 1, w.today, t.elapsed().as_secs_f64());
        }
    }
    eprintln!("TRADES BY MONTH {months:?}");
    summary(&w);
    arm_report(&arm_rows);
    if days > 0 {
        afm_engine::save::write_world(&w, Path::new(&args[3]), "play_on", "play_on", None).expect("write the world");
    }
    eprintln!("wrote {:?}, {:?}, {:.0} s", args[3], w.today, t.elapsed().as_secs_f64());
}

/// The league's last two market years, dead money and the Hall's last classes.
fn summary(w: &afm_engine::world::World) {
    // NFL backs' yards a carry, 120+ carries (minor bug 136): the spread
    // across backs and season to season (real 2021-24: sd 0.573, 0.29).
    let mut by: std::collections::BTreeMap<i32, Vec<(u32, f64)>> = Default::default();
    for p in w.people.iter().filter(|p| p.player.position == afm_engine::positions::Position::RB) {
        let Some(c) = p.pro.as_ref() else { continue };
        let mut seen: std::collections::BTreeMap<i32, (u32, f64)> = Default::default();
        for s in &c.seasons {
            let e = seen.entry(s.year).or_default();
            e.0 += s.stats.rush;
            e.1 += s.stats.rush_yds;
        }
        for (y, (n, yds)) in seen {
            if n >= 120 {
                by.entry(y).or_default().push((p.player.id, yds / n as f64));
            }
        }
    }
    for (y, v) in by.iter().rev().take(12) {
        let m = v.iter().map(|x| x.1).sum::<f64>() / v.len() as f64;
        let sd = (v.iter().map(|x| (x.1 - m).powi(2)).sum::<f64>() / (v.len() as f64 - 1.0).max(1.0)).sqrt();
        let (mut a, mut b) = (Vec::new(), Vec::new());
        if let Some(nx) = by.get(&(y + 1)) {
            for (id, x) in v {
                if let Some((_, z)) = nx.iter().find(|q| q.0 == *id) {
                    a.push(*x);
                    b.push(*z);
                }
            }
        }
        let corr = if a.len() > 3 {
            let (ma, mb) = (a.iter().sum::<f64>() / a.len() as f64, b.iter().sum::<f64>() / b.len() as f64);
            let c: f64 = a.iter().zip(&b).map(|(x, z)| (x - ma) * (z - mb)).sum();
            let va: f64 = a.iter().map(|x| (x - ma).powi(2)).sum();
            let vb: f64 = b.iter().map(|z| (z - mb).powi(2)).sum();
            c / (va * vb).sqrt()
        } else {
            f64::NAN
        };
        eprintln!(
            "RB YPC {y}: {} backs 120+ carries, mean {m:.2} sd {sd:.3}, next season corr {corr:.2} ({} pairs)",
            v.len(),
            a.len()
        );
    }
    // The NFL's points a team game and yards an attempt and a carry by year.
    let mut lg: std::collections::BTreeMap<i32, [f64; 6]> = Default::default();
    for x in &w.nfl_units {
        let e = lg.entry(x.0).or_default();
        e[0] += x.2 as f64;
        e[1] += x.3 as f64;
    }
    for p in w.people.iter() {
        let Some(c) = p.pro.as_ref() else { continue };
        for s in &c.seasons {
            let e = lg.entry(s.year).or_default();
            e[2] += s.stats.att as f64;
            e[3] += s.stats.pass_yds;
            e[4] += s.stats.rush as f64;
            e[5] += s.stats.rush_yds;
        }
    }
    for (y, e) in lg.iter().rev().take(4) {
        eprintln!(
            "NFL {y}: points a team game {:.1}, yards an attempt {:.2}, yards a carry {:.2}",
            e[1] / e[0].max(1.0),
            e[3] / e[2].max(1.0),
            e[5] / e[4].max(1.0)
        );
    }
    for m in w.market.years.iter().rev().take(2) {
        eprintln!(
            "MARKET {} trades {} draft {} season {} deadline {} funnel {:?} ufa {} resigned {} comp {} formula {} supp {} dead {:?}",
            m.year,
            m.trades,
            m.draft_trades,
            m.season_trades,
            m.deadline_trades,
            m.trade_funnel,
            m.ufa,
            m.resigned,
            m.comp_picks,
            m.comp_formula,
            m.comp_supplemental,
            m.dead
        );
    }
    for y in [w.year - 1, w.year] {
        let mut dead: Vec<f64> = w
            .clubs
            .iter()
            .map(|c| c.dead.iter().filter(|x| x.year == y).map(|x| x.amount).sum::<f64>() / 1e6)
            .collect();
        dead.sort_by(f64::total_cmp);
        eprintln!("DEAD {y} median ${:.1}M max ${:.1}M", dead[dead.len() / 2], dead[dead.len() - 1]);
    }
    for h in w.nfl_book.hall.iter().filter(|h| h.0 >= w.year - 1) {
        eprintln!("HALL {} {} {} {}", h.0, h.2, h.3, h.4);
    }
    for h in w.nfl_book.hall_coaches.iter() {
        eprintln!("HALL COACH {} {} {}", h.0, h.2, h.3);
    }
}

/// Drafted share and log pick against arm length by position (minor bug
/// 290; real edges: +6.7 pts drafted an inch, log pick -0.128 an inch).
fn arm_report(rows: &[(String, f64, Option<u32>)]) {
    for pos in ["EDGE", "OT"] {
        let r: Vec<_> = rows.iter().filter(|x| x.0 == pos).collect();
        if r.len() < 20 {
            continue;
        }
        let n = r.len() as f64;
        let ma = r.iter().map(|x| x.1).sum::<f64>() / n;
        let sa = (r.iter().map(|x| (x.1 - ma).powi(2)).sum::<f64>() / n).sqrt();
        let slope = |pts: &[(f64, f64)]| {
            let k = pts.len() as f64;
            let mx = pts.iter().map(|p| p.0).sum::<f64>() / k;
            let my = pts.iter().map(|p| p.1).sum::<f64>() / k;
            let sxy: f64 = pts.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum();
            let sxx: f64 = pts.iter().map(|p| (p.0 - mx).powi(2)).sum();
            (sxy / sxx.max(1e-9), mx, my)
        };
        let drafted: Vec<(f64, f64)> = r.iter().map(|x| (x.1, if x.2.is_some() { 1.0 } else { 0.0 })).collect();
        let picks: Vec<(f64, f64)> = r.iter().filter_map(|x| x.2.map(|p| (x.1, (p as f64).ln()))).collect();
        let r1: Vec<(f64, f64)> =
            r.iter().filter_map(|x| x.2.map(|p| (x.1, if p <= 32 { 1.0 } else { 0.0 }))).collect();
        let (sd, _, md) = slope(&drafted);
        let (sp, _, _) = slope(&picks);
        let (s1, _, _) = slope(&r1);
        eprintln!(
            "ARMS {pos}: {} invited, arm {ma:.2} sd {sa:.2}, drafted {:.1}%; per inch: drafted {:+.1} pts, log pick {:+.3}, round one among drafted {:+.1} pts",
            r.len(),
            100.0 * md,
            100.0 * sd,
            sp,
            100.0 * s1
        );
        for (lo, hi) in [(-9.0, -1.0), (-1.0, 0.0), (0.0, 0.95), (0.95, 9.0)] {
            let b: Vec<_> = r.iter().filter(|x| (x.1 - ma) / sa >= lo && (x.1 - ma) / sa < hi).collect();
            let dr: Vec<u32> = b.iter().filter_map(|x| x.2).collect();
            let mut sorted = dr.clone();
            sorted.sort();
            let med = sorted.get(sorted.len() / 2).copied().unwrap_or(0);
            eprintln!(
                "ARMS {pos} z {lo:+.2}..{hi:+.2}: n {}, drafted {:.0}%, round one {:.0}%, median pick {med}",
                b.len(),
                100.0 * dr.len() as f64 / b.len().max(1) as f64,
                100.0 * dr.iter().filter(|&&p| p <= 32).count() as f64 / b.len().max(1) as f64
            );
        }
    }
}
