//! Memory check (trial 9 finding 12): a full-size world with the game log
//! off, the way the game will run it. Prints resident memory and the live
//! heap after the history and after each future season, then weighs each
//! of the world's stores by dropping it, biggest first.
//! Usage: memory_check [seed] [history_years] [future_years] [scale]
//! (`AFM_LOG=folder` writes the world log there; `AFM_HS_YEARS=n` sets the
//! history's high school years on the engine.)

use std::time::Instant;

use afm_engine::world::{World, WorldOptions};

fn main() {
    afm_tools::count_heap();
    let args: Vec<String> = std::env::args().collect();
    let seed: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(2026);
    let years: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(30);
    let future: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);
    // A share of the real size, for quick checks only.
    let scale: f64 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(1.0);
    // AFM_ENGINE_FROM=year plays the history's games on the stand-in before
    // that year: quick checks only, the people stores are the same size.
    let engine_from = std::env::var("AFM_ENGINE_FROM").ok().and_then(|v| v.parse().ok());
    // AFM_SHEET=none: no staff drafts a call sheet at kickoff, to time what
    // the drafts cost the history (every staff drafts one in the game).
    // AFM_SHEET=every: high school staffs too (the game: college and NFL).
    match std::env::var("AFM_SHEET").as_deref() {
        Ok("none") => afm_engine::game::sheet::KICKOFF.store(0, std::sync::atomic::Ordering::Relaxed),
        Ok("every") => afm_engine::game::sheet::KICKOFF.store(1, std::sync::atomic::Ordering::Relaxed),
        _ => {}
    }
    println!(
        "sizes: person {} B, player {} B, attributes {} B, season record {} B",
        std::mem::size_of::<afm_engine::world::Person>(),
        std::mem::size_of::<afm_engine::player::Player>(),
        std::mem::size_of::<afm_engine::attributes::Attributes>(),
        std::mem::size_of::<afm_engine::player::SeasonRecord>()
    );
    let t = Instant::now();
    // AFM_HS_YEARS=n: the history's last n years of high school games on
    // the engine (the world build: 12).
    let hs_engine_years = std::env::var("AFM_HS_YEARS").ok().and_then(|v| v.parse().ok());
    // AFM_LOG=folder: write the world log there, as the world build does.
    let log_dir = std::env::var("AFM_LOG").ok().map(std::path::PathBuf::from);
    let opts = WorldOptions {
        seed,
        history_years: years,
        keep_game_log: false,
        scale,
        engine_from,
        log_dir,
        hs_engine_years: hs_engine_years.or(WorldOptions::default().hs_engine_years),
        ..Default::default()
    };
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
    println!(
        "people {}, alumni {}, coaches {}, high schools {}, programs {}, news {}",
        w.people.len(),
        w.alumni.len(),
        w.coaches.len(),
        w.high_schools.len(),
        w.programs.len(),
        w.news.len()
    );
    // The same seed must give the same world whatever the stores' layout:
    // a hash of what the years produced (compare it across builds).
    println!("fingerprint {:016x}", fingerprint(&w));
    match w.finish_log() {
        Some(Ok(bytes)) => println!("world log: {:.3} GB on disk", bytes as f64 / 1e9),
        Some(Err(e)) => println!("world log failed: {e}"),
        None => {}
    }
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
    weigh("stories", &mut w, &|w| w.stories = Default::default());
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

/// A hash of what the world's years produced: every person's season rows,
/// ratings, stage and school, every alumnus, coach and news line.
fn fingerprint(w: &World) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for p in &w.people {
        p.name.hash(&mut h);
        format!("{:?}", p.player.history).hash(&mut h);
        format!("{:?}{:?}{:?}{}", p.stage, p.college, p.player.position, p.hs).hash(&mut h);
        format!("{:?}", p.player.attributes).hash(&mut h);
    }
    for a in &w.alumni {
        a.name.hash(&mut h);
    }
    for c in &w.coaches {
        format!("{}{:?}", c.name, c.history.len()).hash(&mut h);
    }
    for n in &w.news {
        n.text.hash(&mut h);
    }
    h.finish()
}
