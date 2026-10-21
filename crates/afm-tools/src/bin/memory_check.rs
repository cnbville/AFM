//! Memory check (trial 9 finding 12): a full-size world with the game log
//! off, the way the game will run it. Prints resident memory and the live
//! heap after the history and after each future season, then weighs each
//! of the world's stores by dropping it, biggest first.
//! Usage: memory_check [seed] [history_years] [future_years] [scale]
//!
//! In a season (final build, the 4 GB budget): `memory_check file.afmworld
//! [month-day]` opens a world file as the game does, plays day by day to
//! that day (default 09-06, the first college Saturday's week), prints the
//! memory every two weeks, then weighs the stores, the season's own first.
//! `AFM_SEASON_TO=09-06` does the same after a world built here (a small
//! one for quick before-and-after checks).

use std::time::Instant;

use afm_engine::world::{World, WorldOptions};

fn main() {
    afm_tools::count_heap();
    let args: Vec<String> = std::env::args().collect();
    if let Some(path) = args.get(1).filter(|a| a.ends_with(".afmworld")) {
        return in_season(path, args.get(2).map(String::as_str).unwrap_or("09-06"));
    }
    let seed: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(2026);
    let years: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(30);
    let future: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);
    // A share of the real size, for quick checks only.
    let scale: f64 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(1.0);
    // AFM_ENGINE_FROM=year plays the history's games on the stand-in before
    // that year: quick checks only, the people stores are the same size.
    let engine_from = std::env::var("AFM_ENGINE_FROM").ok().and_then(|v| v.parse().ok());
    println!(
        "sizes: person {} B, player {} B, attributes {} B, season record {} B",
        std::mem::size_of::<afm_engine::world::Person>(),
        std::mem::size_of::<afm_engine::player::Player>(),
        std::mem::size_of::<afm_engine::attributes::Attributes>(),
        std::mem::size_of::<afm_engine::player::SeasonRecord>()
    );
    let t = Instant::now();
    let opts =
        WorldOptions { seed, history_years: years, keep_game_log: false, scale, engine_from, ..Default::default() };
    let mut w = World::new_watched(opts, year_line);
    println!(
        "history of {years} years built in {:.0}s: heap {:.2} GB, resident {}",
        t.elapsed().as_secs_f64(),
        afm_tools::heap_gb(),
        afm_tools::memory_text()
    );
    for _ in 0..future {
        let s = Instant::now();
        w.advance_year();
        year_line(&w, s.elapsed().as_secs_f64());
    }
    if let Ok(until) = std::env::var("AFM_SEASON_TO") {
        into_season(&mut w, &until);
    }
    weigh_all(w, t);
}

/// A world file opened as the game opens it, then played day by day into
/// the season: memory every two weeks, then each store weighed.
fn in_season(path: &str, until: &str) {
    let t = Instant::now();
    let mut w = afm_engine::save::read_world(std::path::Path::new(path)).expect("read the world");
    w.after_load();
    println!(
        "opened {path} in {:.0}s on {}: heap {:.2} GB, resident {}",
        t.elapsed().as_secs_f64(),
        w.today,
        afm_tools::heap_gb(),
        afm_tools::memory_text()
    );
    into_season(&mut w, until);
    weigh_all(w, t);
}

/// Plays day by day to `until` (month-day) in the world's season, then
/// weighs the season's own stores.
fn into_season(w: &mut World, until: &str) {
    let (m, d) = until.split_once('-').map_or((9, 6), |(m, d)| (m.parse().unwrap_or(9), d.parse().unwrap_or(6)));
    let stop = afm_engine::calendar::Date::ymd(w.year, m, d);
    let s = Instant::now();
    let mut k = 0;
    while w.today < stop {
        w.advance_day();
        k += 1;
        if k % 14 == 0 || w.today >= stop {
            println!(
                "{}: heap {:.2} GB, resident {}, events this year {}",
                w.today,
                afm_tools::heap_gb(),
                afm_tools::memory_text(),
                w.calendar.events.values().map(Vec::len).sum::<usize>()
            );
        }
    }
    println!("played {k} days in {:.0}s", s.elapsed().as_secs_f64());
    let mut rows: Vec<(String, f64)> = Vec::new();
    for store in [
        "college season",
        "last college season",
        "high school season",
        "NFL season",
        "portal market",
        "calendar: this year's events",
        "calendar: past years' moments",
        "your games",
        "people: high school lines",
    ] {
        let before = afm_tools::heap_gb();
        w.debug_drop_season_store(store);
        rows.push((store.to_string(), before - afm_tools::heap_gb()));
    }
    rows.sort_by(|a, b| b.1.total_cmp(&a.1));
    println!("\n| the season's stores | GB |\n|---|---|");
    for (name, gb) in &rows {
        println!("| {name} | {gb:.3} |");
    }
}

/// Each store weighed by dropping it, biggest first.
fn weigh_all(mut w: World, t: Instant) {
    println!(
        "people {}, alumni {}, coaches {}, high schools {}, programs {}, news {}",
        w.people.len(),
        w.alumni.len(),
        w.coaches.len(),
        w.high_schools.len(),
        w.programs.len(),
        w.news.len()
    );
    // Parts first (inside the stores), then whole stores.
    let mut rows: Vec<(String, f64)> = Vec::new();
    let mut weigh = |name: &str, w: &mut World, f: &dyn Fn(&mut World)| {
        let before = afm_tools::heap_gb();
        f(w);
        rows.push((name.to_string(), before - afm_tools::heap_gb()));
    };
    let (len, cap) = w.debug_people_capacity();
    println!("people list: {len} people, room for {cap}");
    for part in ["story", "name", "position history", "college staff", "path", "calendar lists"] {
        weigh(&format!("people: {part}"), &mut w, &|w| w.debug_drop_people_part(part));
    }
    weigh("people: notes", &mut w, &|w| w.people.iter_mut().for_each(|p| p.notes = Vec::new()));
    weigh("people: season records", &mut w, &|w| w.people.iter_mut().for_each(|p| p.player.history = Vec::new()));
    weigh("people: pro careers", &mut w, &|w| w.people.iter_mut().for_each(|p| p.pro = None));
    weigh("alumni: notes", &mut w, &|w| w.alumni.iter_mut().for_each(|a| a.notes = Vec::new()));
    weigh("alumni: players", &mut w, &|w| w.alumni.iter_mut().for_each(|a| a.player = None));
    weigh("alumni: pro careers", &mut w, &|w| w.alumni.iter_mut().for_each(|a| a.pro = None));
    weigh("people (the rest)", &mut w, &|w| w.people = Vec::new());
    weigh("alumni (the rest)", &mut w, &|w| w.alumni = Vec::new());
    weigh("coaches", &mut w, &|w| w.coaches = Vec::new());
    weigh("high schools", &mut w, &|w| w.high_schools = Vec::new());
    weigh("programs", &mut w, &|w| w.programs = Vec::new());
    weigh("news", &mut w, &|w| w.news = Vec::new());
    weigh("stat book", &mut w, &|w| w.stats = Default::default());
    weigh("game log", &mut w, &|w| w.game_log = Default::default());
    weigh("recruiting log", &mut w, &|w| w.recruiting_log = Default::default());
    weigh("NFL market", &mut w, &|w| w.market = Default::default());
    weigh("NFL clubs", &mut w, &|w| w.clubs = Vec::new());
    weigh("NFL book", &mut w, &|w| w.nfl_book = Default::default());
    weigh("calendar logs", &mut w, &|w| w.log_history = Vec::new());
    weigh("event history", &mut w, &|w| w.event_history = Vec::new());
    weigh("calendar", &mut w, &|w| w.calendar = Default::default());
    let rest = afm_tools::heap_gb();
    rows.sort_by(|a, b| b.1.total_cmp(&a.1));
    println!("\n| store | GB |\n|---|---|");
    for (name, gb) in &rows {
        println!("| {name} | {gb:.3} |");
    }
    println!("| everything else | {rest:.3} |");
    println!("\nall in {:.0}s", t.elapsed().as_secs_f64());
}

/// One season's line: its time, memory, and its high school games on the
/// engine with their per-team averages.
fn year_line(w: &World, secs: f64) {
    let (games, hs) = w.log_history.last().map_or((0, Default::default()), |l| (l.hs_engine_games, l.hs_totals));
    let per = |x: f64| if hs.games > 0 { x / hs.games as f64 } else { 0.0 };
    println!(
        "season {}: {secs:.0}s, heap {:.2} GB, resident {}, people {}, HS engine games {games} (a team game: {:.1} points, {:.1} plays, {:.0} yards)",
        w.year - 1,
        afm_tools::heap_gb(),
        afm_tools::memory_text(),
        w.people.len(),
        per(hs.points_for as f64),
        per(hs.plays as f64),
        per(hs.yards),
    );
}
