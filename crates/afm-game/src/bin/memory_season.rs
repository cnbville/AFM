//! Memory in season (final build, lane 1): opens a world file the way the
//! game does, starts a career, and presses Continue to the first game,
//! printing the live heap and resident memory at every stop. At the end it
//! weighs the world's stores by dropping them, biggest first. With the
//! fingerprint it prints the world log's print of every store (`wlog::why::
//! fingerprints`, each store's whole serde form): two builds that print the
//! same lines played the same season, field for field.
//! Usage: memory_season <world file> [college|nfl|hs] [team id] [weigh: open|end|none|monthN|days]
//! (`days` steps a day at a time and prints each day that raised the peak;
//! `AFM_TRIM=1` also prints resident memory after the allocator lets go).
//! The game's own allocator (the system one) is kept; a thin counter
//! around it gives the live heap.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use afm_engine::world::World;
use afm_game::Game;
use serde_json::json;

struct Counting;
static LIVE: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        LIVE.fetch_add(l.size(), Ordering::Relaxed);
        unsafe { System.alloc(l) }
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        LIVE.fetch_add(l.size(), Ordering::Relaxed);
        unsafe { System.alloc_zeroed(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        LIVE.fetch_sub(l.size(), Ordering::Relaxed);
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, new: usize) -> *mut u8 {
        LIVE.fetch_add(new, Ordering::Relaxed);
        LIVE.fetch_sub(l.size(), Ordering::Relaxed);
        unsafe { System.realloc(p, l, new) }
    }
}

#[global_allocator]
static A: Counting = Counting;

fn heap_gb() -> f64 {
    LIVE.load(Ordering::Relaxed) as f64 / 1e9
}

/// Resident now and at its peak, from /proc (Linux) or `ps` elsewhere.
fn resident() -> String {
    if let Ok(s) = std::fs::read_to_string("/proc/self/status") {
        let kb = |key: &str| {
            s.lines()
                .find(|l| l.starts_with(key))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|v| v.parse::<f64>().ok())
                .map_or(0.0, |k| k / 1e6)
        };
        return format!("resident {:.2} GB, peak {:.2} GB", kb("VmRSS:"), kb("VmHWM:"));
    }
    let out = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<f64>().ok())
        .map_or(0.0, |k| k / 1e6);
    format!("resident {out:.2} GB")
}

/// Resident memory at its peak so far (Linux only; 0 elsewhere).
fn peak_gb() -> f64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines().find(|l| l.starts_with("VmHWM:")).and_then(|l| l.split_whitespace().nth(1)?.parse().ok())
        })
        .map_or(0.0, |k: f64| k / 1e6)
}

fn line(what: &str, t: &Instant) {
    println!("{what:<40} {:>6.0}s  heap {:.2} GB, {}", t.elapsed().as_secs_f64(), heap_gb(), resident());
    // AFM_TRIM=1: what resident memory is once the allocator hands back
    // the pages it holds free (glibc only), to tell live data from slack.
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    if std::env::var("AFM_TRIM").is_ok() {
        unsafe extern "C" {
            fn malloc_trim(pad: usize) -> i32;
        }
        unsafe { malloc_trim(0) };
        println!(
            "{:<40} {:>6.0}s  heap {:.2} GB, {}",
            "  after trim",
            t.elapsed().as_secs_f64(),
            heap_gb(),
            resident()
        );
    }
}

/// The draft live from your chair (lane 7): the staff picks through the
/// night, then the rush on the last.
fn war_room(g: &mut Game) {
    for _ in 0..400 {
        let room = g.call("draft_live", &json!({})).expect("draft_live");
        if !room["live"].as_bool().unwrap_or(false) || room["waiting"] == json!(true) {
            break;
        }
        if room["rush"].as_bool().unwrap_or(false) {
            g.call("draft_rush", &json!({})).expect("draft_rush");
            break;
        }
        let cmd = if room["on_clock"].as_bool().unwrap_or(false) { "draft_auto" } else { "draft_go" };
        g.call(cmd, &json!({ "until": "yours" })).expect(cmd);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).expect("usage: memory_season <world file> [kind] [id] [weigh]");
    let kind = args.get(2).map_or("college", |s| s.as_str());
    let weigh_at = args.get(4).map_or("end", |s| s.as_str());
    let t = Instant::now();
    let home = std::env::temp_dir().join("afm-memory-season");
    let mut g = Game::new(home);
    g.call("open_world", &json!({ "path": path })).expect("open");
    line("opened", &t);
    println!(
        "  sizes: person {} B, player {} B, season record {} B",
        std::mem::size_of::<afm_engine::world::Person>(),
        std::mem::size_of::<afm_engine::player::Player>(),
        std::mem::size_of::<afm_engine::player::SeasonRecord>()
    );
    let id: u64 = match args.get(3).and_then(|s| s.parse().ok()) {
        Some(i) => i,
        None => {
            let w = g.world.as_ref().expect("world");
            (0..w.programs.len()).find(|&i| w.program_name(i) == "Iowa").unwrap_or(0) as u64
        }
    };
    if weigh_at == "open" {
        weigh(g.world.as_mut().expect("world"));
        return;
    }
    if weigh_at == "parts" {
        parts(g.world.as_mut().expect("world"));
        return;
    }
    g.call("start_career", &json!({ "job": { "kind": kind, "id": id }, "seed": 7 })).expect("start");
    line("career started", &t);
    let year = g.world.as_ref().expect("world").today.parts().0;
    // weigh "days": one day at a time to the first game, handing freed
    // pages back after each, printing each day that raised the peak: what
    // a single day's work costs at its height.
    if weigh_at == "days" {
        let mut peak = 0.0;
        loop {
            let r = g.call("day", &json!({})).expect("day");
            // The war room holds the clock: the staff makes the night's picks.
            if r["studio"]["screen"] == json!("warroom") {
                war_room(&mut g);
            }
            afm_game::memory::hand_back();
            let w = g.world.as_ref().expect("world");
            let (y, m, d) = w.today.parts();
            let now = peak_gb();
            if now > peak + 0.005 {
                line(&format!("peak rose by {:.2} GB to {y}-{m:02}-{d:02}", now - peak), &t);
                peak = now;
            }
            if w.seat_games.iter().any(|g| g.date.parts().0 == year && g.date.parts().1 >= 8) || y > year {
                return;
            }
        }
    }
    loop {
        let r = g.call("continue", &json!({ "staff": true })).expect("continue");
        // The draft live from your chair (lane 7): the staff picks, then the rush.
        if r["studio"]["screen"] == json!("warroom") {
            line("war room open", &t);
            war_room(&mut g);
            line("draft done", &t);
        }
        let w = g.world.as_ref().expect("world");
        let (y, m, d) = w.today.parts();
        let _ = r;
        line(&format!("stopped {y}-{m:02}-{d:02}"), &t);
        println!(
            "  people {} (room {}), alumni {}, news {}, records {}",
            w.people.len(),
            w.people.capacity(),
            w.alumni.len(),
            w.news.len(),
            w.people.iter().map(|p| p.player.history.len()).sum::<usize>()
        );
        if let Some(month) = weigh_at.strip_prefix("month").and_then(|x| x.parse::<u32>().ok())
            && m >= month
        {
            weigh(g.world.as_mut().expect("world"));
            return;
        }
        let played = w.seat_games.iter().any(|g| g.date.parts().0 == year && g.date.parts().1 >= 8);
        if played || (y > year) {
            break;
        }
    }
    // AFM_UNTIL=YYYY-MM-DD (memory diet lane M2's check): Continue on past
    // the first game to that day, through the year's cleanup, so the prints
    // cover a whole football year.
    if let Some(until) = std::env::var("AFM_UNTIL").ok().and_then(|s| {
        let v: Vec<u32> = s.split('-').filter_map(|x| x.parse().ok()).collect();
        (v.len() == 3).then(|| afm_engine::calendar::Date::ymd(v[0] as i32, v[1], v[2]))
    }) {
        for _ in 0..2000 {
            if g.world.as_ref().expect("world").today >= until {
                break;
            }
            let r = g.call("continue", &json!({ "staff": true })).expect("continue");
            if r["studio"]["screen"] == json!("warroom") {
                war_room(&mut g);
            }
        }
        let (y, m, d) = g.world.as_ref().expect("world").today.parts();
        line(&format!("on to {y}-{m:02}-{d:02}"), &t);
    }
    // The film room (lane 13): what watching your last game back adds.
    if let Some(k) = g.world.as_ref().and_then(|w| w.seat_games.len().checked_sub(1)) {
        let before = heap_gb();
        let t1 = Instant::now();
        match g.call("film", &json!({ "index": k })) {
            Ok(_) => {
                let secs = t1.elapsed().as_secs_f64();
                println!("  the film of your game: {:.0} MB, played back in {secs:.1}s", (heap_gb() - before) * 1000.0);
                line("film open", &t);
            }
            Err(e) => println!("  film: {e}"),
        }
        let _ = g.call("film_close", &json!({}));
    }
    println!("fingerprint {:016x}", fingerprint(g.world.as_ref().expect("world")));
    for (store, print) in afm_engine::world::wlog::why::fingerprints(g.world.as_ref().expect("world")) {
        println!("  store print {store:<14} {print}");
    }
    kept_parts(g.world.as_ref().expect("world"));
    if weigh_at == "end" {
        weigh(g.world.as_mut().expect("world"));
    }
    if weigh_at == "endparts" {
        parts(g.world.as_mut().expect("world"));
    }
}

/// The memory diet's lane M2 parts (items 3, 6, 9, 10), each weighed by
/// dropping it: men who left football, retired coaches, the alumni's parts
/// and the tallies' open stores.
fn parts(w: &mut World) {
    use afm_engine::world::Stage;
    let done = w.people.iter().filter(|p| p.stage == Stage::Done).count();
    let retired = w.coaches.iter().filter(|c| c.retired).count();
    println!(
        "  left football {done} ({} B inline each), retired coaches {retired} of {} ({} B inline each), alumni {} ({} B inline), pro seasons {} B",
        std::mem::size_of::<afm_engine::world::Person>(),
        w.coaches.len(),
        std::mem::size_of::<afm_engine::world::coach::Coach>(),
        w.alumni.len(),
        std::mem::size_of::<afm_engine::world::Alumnus>(),
        std::mem::size_of::<afm_engine::world::pro::ProSeason>(),
    );
    let mut rows: Vec<(String, f64)> = Vec::new();
    let mut take = |name: &str, w: &mut World, f: &dyn Fn(&mut World)| {
        let before = heap_gb();
        f(w);
        rows.push((name.to_string(), before - heap_gb()));
    };
    let gone = |p: &afm_engine::world::Person| p.stage == Stage::Done;
    take("left: season rows", w, &|w| {
        w.people.iter_mut().filter(|p| gone(p)).for_each(|p| p.player.history = Vec::new())
    });
    take("left: hs lines", w, &|w| w.people.iter_mut().filter(|p| gone(p)).for_each(|p| p.hs_lines = Vec::new()));
    take("left: notes", w, &|w| w.people.iter_mut().filter(|p| gone(p)).for_each(|p| p.notes = Vec::new()));
    take("left: college part", w, &|w| {
        w.people.iter_mut().filter(|p| gone(p)).for_each(|p| p.later = Default::default())
    });
    take("left: calendar rest", w, &|w| {
        w.people.iter_mut().filter(|p| gone(p)).for_each(|p| p.cal.more = Default::default())
    });
    take("left: pro careers", w, &|w| w.people.iter_mut().filter(|p| gone(p)).for_each(|p| p.pro = None));
    take("left: tests", w, &|w| w.people.iter_mut().filter(|p| gone(p)).for_each(|p| p.tests = Default::default()));
    take("retired coaches: history", w, &|w| {
        w.coaches.iter_mut().filter(|c| c.retired).for_each(|c| c.history = Vec::new())
    });
    take("retired coaches: ties", w, &|w| {
        w.coaches.iter_mut().filter(|c| c.retired).for_each(|c| c.ties = Default::default())
    });
    take("retired coaches: recent", w, &|w| {
        w.coaches.iter_mut().filter(|c| c.retired).for_each(|c| c.recent = Vec::new())
    });
    take("alumni: pro playoffs", w, &|w| {
        w.alumni
            .iter_mut()
            .filter_map(|a| a.pro.as_mut())
            .for_each(|c| c.seasons.iter_mut().for_each(|s| s.playoffs = None))
    });
    take("alumni: pro careers", w, &|w| w.alumni.iter_mut().for_each(|a| a.pro = None));
    take("alumni: players", w, &|w| w.alumni.iter_mut().for_each(|a| a.player = None));
    take("alumni: notes and summaries", w, &|w| {
        w.alumni.iter_mut().for_each(|a| {
            a.notes = Vec::new();
            a.background_summary = String::new();
        })
    });
    take("tallies: players", w, &|w| w.tallies.players = Default::default());
    take("tallies: teams", w, &|w| w.tallies.teams = Default::default());
    take("tallies: squads", w, &|w| w.tallies.squads = Default::default());
    take("tallies: calls", w, &|w| w.tallies.calls = Default::default());
    take("tallies: prospects", w, &|w| w.tallies.prospects = Default::default());
    take("tallies: classes", w, &|w| w.tallies.classes = Default::default());
    take("tallies: moves", w, &|w| w.tallies.moves = Default::default());
    take("tallies: games", w, &|w| w.tallies.games = Default::default());
    println!("\n| part | MB |\n|---|---|");
    for (name, gb) in &rows {
        println!("| {name} | {:.1} |", gb * 1000.0);
    }
}

/// A hash of what the season produced: every person's season records and
/// ratings, every coach's record, the news, your games and the calendar.
fn fingerprint(w: &World) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    format!("{:?}", w.today).hash(&mut h);
    for p in &w.people {
        p.name.hash(&mut h);
        format!("{:?}", p.player.history).hash(&mut h);
        format!("{:?}{:?}{:?}", p.stage, p.college, p.player.position).hash(&mut h);
        format!("{:?}", p.player.attributes).hash(&mut h);
    }
    for n in &w.news {
        format!("{n:?}").hash(&mut h);
    }
    // The box score keeps player lines in a hash map, whose print order
    // changes run to run, so the lines go in by player id.
    for g in &w.seat_games {
        format!("{:?}{:?}{:?}{:?}", g.date, g.teams, g.stats.score, g.stats.quarters).hash(&mut h);
        format!("{:?}{:?}", g.stats.team, g.stats.plays).hash(&mut h);
        for side in &g.stats.players {
            let mut lines: Vec<_> = side.iter().collect();
            lines.sort_by_key(|(id, _)| **id);
            format!("{lines:?}").hash(&mut h);
        }
    }
    format!("{:?}", w.seat_depth).hash(&mut h);
    h.finish()
}

fn weigh(w: &mut World) {
    let mut rows: Vec<(String, f64)> = Vec::new();
    let mut take = |name: &str, w: &mut World, f: &dyn Fn(&mut World)| {
        let before = heap_gb();
        f(w);
        rows.push((name.to_string(), before - heap_gb()));
    };
    take("people: season records", w, &|w| w.people.iter_mut().for_each(|p| p.player.history = Vec::new()));
    take("people: notes", w, &|w| w.people.iter_mut().for_each(|p| p.notes = Vec::new()));
    take("people: hs lines", w, &|w| w.people.iter_mut().for_each(|p| p.hs_lines = Vec::new()));
    take("people: pro careers", w, &|w| w.people.iter_mut().for_each(|p| p.pro = None));
    for s in [
        "college season",
        "last college season",
        "high school season",
        "NFL season",
        "portal market",
        "announcements",
        "searches",
    ] {
        take(s, w, &|w| w.memory_drop_store(s));
    }
    take("people (the rest)", w, &|w| w.people = Vec::new());
    take("alumni", w, &|w| w.alumni = Vec::new());
    take("coaches", w, &|w| w.coaches = Vec::new());
    take("high schools", w, &|w| w.high_schools = Vec::new());
    take("programs", w, &|w| w.programs = Vec::new());
    take("news", w, &|w| w.news = Vec::new());
    take("stories", w, &|w| w.stories = Default::default());
    take("stat book", w, &|w| w.stats = Default::default());
    take("game log", w, &|w| w.game_log = Default::default());
    take("recruiting log", w, &|w| w.recruiting_log = Default::default());
    take("NFL market", w, &|w| w.market = Default::default());
    take("NFL clubs", w, &|w| w.clubs = Vec::new());
    take("NFL teams", w, &|w| w.nfl = Vec::new());
    take("NFL book", w, &|w| w.nfl_book = Default::default());
    take("NFL seasons", w, &|w| w.nfl_seasons = Vec::new());
    take("calendar logs", w, &|w| w.log_history = Vec::new());
    take("event history", w, &|w| w.event_history = Vec::new());
    take("calendar", w, &|w| w.calendar = Default::default());
    take("seat games", w, &|w| w.seat_games = Vec::new());
    take("career", w, &|w| w.career = None);
    let rest = heap_gb();
    rows.sort_by(|a, b| b.1.total_cmp(&a.1));
    println!("\n| store | GB |\n|---|---|");
    for (name, gb) in &rows {
        println!("| {name} | {gb:.3} |");
    }
    println!("| everything else | {rest:.3} |");
}

/// How many people keep their calendar's rest and their college part
/// (memory diet, lane M1: kept only once used), and how many of them are
/// high schoolers.
fn kept_parts(w: &World) {
    use afm_engine::world::Stage;
    let count = |f: &dyn Fn(&afm_engine::world::Person) -> bool| {
        let all = w.people.iter().filter(|p| f(p)).count();
        let hs = w.people.iter().filter(|p| p.stage == Stage::HighSchool && f(p)).count();
        (all, hs)
    };
    let (cal, cal_hs) = count(&|p| p.cal.more.stored().is_some());
    let (later, later_hs) = count(&|p| p.later.stored().is_some());
    println!(
        "  kept parts of {} people: calendar rest {cal} ({cal_hs} high schoolers), college part {later} ({later_hs} high schoolers)",
        w.people.len()
    );
    // Lane M2 (items 3, 9, 10): what is packed or dropped now.
    let retired = w.coaches.iter().filter(|c| c.retired).count();
    let kept: Vec<usize> = w.alumni.iter().filter_map(|a| a.player.as_ref().map(|k| k.packed_len())).collect();
    let (p_hot, p_cold, p_bytes) = w.tallies.players.held();
    let (t_hot, t_cold, t_bytes) = w.tallies.teams.held();
    println!(
        "  retired coaches {retired} of {}; alumni players packed {} ({:.1} MB); open tallies: players {p_hot} rows, {p_cold} packed ({:.1} MB), teams {t_hot} rows, {t_cold} packed ({:.1} MB)",
        w.coaches.len(),
        kept.len(),
        kept.iter().sum::<usize>() as f64 / 1e6,
        p_bytes as f64 / 1e6,
        t_bytes as f64 / 1e6
    );
    // A Data page's read of every open row (packed rows unpacked).
    let t = std::time::Instant::now();
    let (mut n, mut games) = (0usize, 0u64);
    w.tally_players(w.tally_open_season(), |s| {
        n += 1;
        games += s.games as u64;
    });
    let mut teams = 0usize;
    w.tally_teams(w.tally_open_season(), |_| teams += 1);
    println!(
        "  reading every open row: {n} players ({games} games) and {teams} teams in {:.1} ms",
        t.elapsed().as_secs_f64() * 1e3
    );
    let t = std::time::Instant::now();
    let mut last = 0usize;
    w.tally_players(w.tally_open_season() - 1, |_| last += 1);
    println!("  reading last season's closed file: {last} players in {:.1} ms", t.elapsed().as_secs_f64() * 1e3);
    // The squad boards' worst case: every kept alumni player unpacked once.
    let t = std::time::Instant::now();
    let ages: u64 = w.alumni.iter().filter_map(|a| a.player.as_ref()).map(|k| k.get().age as u64).sum();
    let one = t.elapsed().as_secs_f64() * 1e3;
    let t = std::time::Instant::now();
    use rayon::prelude::*;
    let par: u64 = w.alumni.par_iter().filter_map(|a| a.player.as_ref()).map(|k| k.get().age as u64).sum();
    let most = afm_engine::positions::Position::ALL
        .iter()
        .map(|&p| w.alumni.iter().filter(|a| a.position == p && a.player.is_some()).count())
        .max()
        .unwrap_or(0);
    println!(
        "  unpacking all {} alumni players: {one:.1} ms on one core, {:.1} ms on all (ages {ages} {par}); the biggest position has {most}",
        kept.len(),
        t.elapsed().as_secs_f64() * 1e3
    );
}
