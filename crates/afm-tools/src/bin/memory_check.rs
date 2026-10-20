//! Memory check (trial 9 finding 12): a full-size world with the game log
//! off, the way the game will run it. Prints resident memory and the live
//! heap after the history and after each future season, then weighs each
//! of the world's stores by dropping it, biggest first.
//! Usage: memory_check [seed] [history_years] [future_years]

use std::time::Instant;

use afm_engine::world::{World, WorldOptions};

fn main() {
    afm_tools::count_heap();
    let args: Vec<String> = std::env::args().collect();
    let seed: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(2026);
    let years: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(30);
    let future: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);
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
    let mut w = World::new(WorldOptions {
        seed,
        history_years: years,
        keep_game_log: false,
        engine_from,
        ..Default::default()
    });
    println!(
        "history of {years} years built in {:.0}s: heap {:.2} GB, resident {}",
        t.elapsed().as_secs_f64(),
        afm_tools::heap_gb(),
        afm_tools::memory_text()
    );
    for _ in 0..future {
        let s = Instant::now();
        w.advance_year();
        println!(
            "season to {}: {:.0}s, heap {:.2} GB, resident {}",
            w.year,
            s.elapsed().as_secs_f64(),
            afm_tools::heap_gb(),
            afm_tools::memory_text()
        );
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
