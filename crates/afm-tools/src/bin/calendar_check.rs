//! Section 4 checks: builds a world on the daily clock and prints each
//! year's calendar log, the loud events and the key dates, so the calendar
//! can be judged against the real one.
//! Usage: calendar_check [scale] [history_years] [seed] [engine_from]

use afm_engine::calendar::Loudness;
use afm_engine::world::{CalendarLog, World, WorldOptions};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let scale: f64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.1);
    let years: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(2);
    let seed: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(2026);
    let engine_from: Option<i32> = args.get(4).and_then(|s| s.parse().ok());
    let t = std::time::Instant::now();
    let w = World::new(WorldOptions {
        seed,
        scale,
        history_years: years,
        engine_from,
        keep_game_log: true,
        ..Default::default()
    });
    println!("built in {:.0}s", t.elapsed().as_secs_f64());
    for log in w.log_history.iter().rev().take(years as usize).rev() {
        println!("--- {} ---", log.year);
        print_log(log);
    }
    for (year, events) in w.event_history.iter().rev().take(1) {
        println!("=== loud events {year} ===");
        for e in events.iter().filter(|e| e.loud >= Loudness::Big) {
            println!("{} {:<6} {:<5} {}", e.when.date.pretty(), e.level.name(), e.loud.name(), e.text);
        }
    }
    println!("NFL champions: {:?}", w.nfl_champions);
    hs_dynasties(&w);
    for (k, n) in w.calendar.counts.iter() {
        println!("count {k:?}: {n}");
    }
}

/// One line per number; lists are summed or counted.
fn print_log(l: &CalendarLog) {
    let rows: Vec<(&str, String)> = vec![
        ("college games", l.games.to_string()),
        ("snow / rain / freezing", format!("{} / {} / {}", l.snow_games, l.rain_games, l.freezing_games)),
        (
            "lightning delays / moved / cancelled",
            format!("{} / {} / {}", l.lightning_delays, l.weather_moves, l.weather_cancels),
        ),
        ("bowl opt-outs", l.opt_outs.to_string()),
        ("bowl practice gain (sum)", format!("{:.0}", l.bowl_practice_gain)),
        ("four-game redshirts / tricks", format!("{} / {}", l.four_game_redshirts, l.redshirt_tricks)),
        (
            "system switches (FBS WC AR WZ PG SO / 43 34 / C3 C4 man C2)",
            format!("{} ({:?})", l.system_switches, l.fbs_systems),
        ),
        (
            "spring notices / breakouts / fizzles",
            format!("{} / {} / {}", l.spring_notices, l.spring_breakouts, l.spring_fizzles),
        ),
        ("hs games / forfeits / no contests", format!("{} / {} / {}", l.hs_games, l.hs_forfeits, l.hs_no_contests)),
        ("hs grade-outs", l.hs_grade_outs.to_string()),
        ("hs homecomings / senior nights", format!("{} / {}", l.hs_homecomings, l.hs_senior_nights)),
        ("hs all-state", l.hs_all_state.to_string()),
        ("heat-limited days", l.heat_limited_days.to_string()),
        ("commits / decommits / flips", format!("{} / {} / {}", l.commits, l.decommits, l.flips)),
        ("commits by month", format!("{:?}", l.commits_by_month)),
        ("signed early / February", format!("{} / {}", l.signed_early, l.signed_late)),
        ("signed by division [early, Feb]", format!("{:?}", l.signed_by_division)),
        ("early enrollees", l.early_enrollees.to_string()),
        (
            "official / unofficial visits / drop-ins",
            format!("{} / {} / {}", l.official_visits, l.unofficial_visits, l.drop_ins),
        ),
        ("declared early / withdrew", format!("{} / {}", l.declared, l.withdrew)),
        (
            "portal entries / landed / returned",
            format!("{} / {} / {}", l.portal_entries, l.portal_landed, l.portal_returned),
        ),
        ("portal entries by division (FBS FCS D2 NAIA D3 JUCO)", format!("{:?}", l.portal_by_division)),
        ("portal entry days", l.portal_by_day.len().to_string()),
        ("portal busiest days", {
            let mut v = l.portal_by_day.clone();
            v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            v.iter().take(4).map(|(d, n)| format!("{} {n}", d.pretty())).collect::<Vec<_>>().join(", ")
        }),
        ("whispers", l.whispers.to_string()),
        ("sit-out transfers", l.sit_out_transfers.to_string()),
        ("spring / fall ineligible", format!("{} / {}", l.spring_ineligible, l.fall_ineligible)),
        (
            "combine invites / all-star invites / declined",
            format!("{} / {} / {}", l.combine_invites, l.all_star_invites, l.all_star_declined),
        ),
        ("walk-ons put on scholarship / tryouts", format!("{} / {}", l.walk_on_scholarships, l.tryouts)),
        ("college teams with a huge game", l.huge_by_team.len().to_string()),
        ("college huge games (sum)", l.huge_by_team.iter().map(|h| h.1).sum::<u32>().to_string()),
        ("rule changes", format!("{:?}", l.rule_changes)),
        ("nfl games", l.nfl_games.to_string()),
        ("nfl huge games by team", format!("{:?}", l.nfl_huge_by_team)),
    ];
    for (k, v) in rows {
        println!("  {k:<40} {v}");
    }
}

/// High school dynasties (A11): the longest winning streaks across the
/// states (real record: De La Salle's 151, 1992-2004) and the best programs'
/// records over the history.
fn hs_dynasties(w: &World) {
    use afm_engine::world::records::{Book, Scope};
    let mut hs: Vec<_> = w
        .stats
        .books
        .iter()
        .filter(|(k, _)| matches!(k.0, Scope::State(_)) && k.1 == Book::WinStreak)
        .flat_map(|(k, v)| v.iter().map(move |e| (k.0.clone(), e)))
        .collect();
    hs.sort_by(|a, b| b.1.value.total_cmp(&a.1.value).then_with(|| a.1.name.cmp(&b.1.name)));
    println!("=== longest high school winning streaks ===");
    for (sc, e) in hs.iter().take(10) {
        let st = if let Scope::State(s) = sc { afm_engine::world::geo::STATE_CENTERS[*s].0 } else { "" };
        println!("{:>4.0} {} {st} {}-{}{}", e.value, e.name, e.from, e.year, if e.live { " running" } else { "" });
    }
    println!("=== most state titles ===");
    let mut by: Vec<_> = w.high_schools.iter().filter(|h| !h.seasons.is_empty()).collect();
    by.sort_by_key(|h| std::cmp::Reverse(h.seasons.iter().filter(|s| s.state_title).count()));
    for h in by.iter().take(10) {
        let (wn, l): (u32, u32) = h.seasons.iter().fold((0, 0), |a, s| (a.0 + s.wins, a.1 + s.losses));
        let titles = h.seasons.iter().filter(|s| s.state_title).count();
        let st = afm_engine::world::geo::STATE_CENTERS[h.state].0;
        println!("{titles:>3} titles {wn}-{l} {} ({}) {st} class {}", h.name, h.town, h.classification + 1);
    }
}
