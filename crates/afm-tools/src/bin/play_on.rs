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
    for d in 0..days {
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
    if days > 0 {
        afm_engine::save::write_world(&w, Path::new(&args[3]), "play_on", "play_on", None).expect("write the world");
    }
    eprintln!("wrote {:?}, {:?}, {:.0} s", args[3], w.today, t.elapsed().as_secs_f64());
}

/// The league's last two market years, dead money and the Hall's last classes.
fn summary(w: &afm_engine::world::World) {
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
