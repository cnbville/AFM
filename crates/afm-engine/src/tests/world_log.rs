//! The world reader's game log (lane R1): logging only watches. A world
//! built with the log on stays the same as one with it off, and the log
//! holds every game at every level.

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::io::BufRead;

use crate::world::{World, WorldOptions};

/// What a day leaves in the world: every person, program and coach, the
/// day and the year's counters.
fn day_print(w: &World) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    w.today.hash(&mut h);
    for p in &w.people {
        (p.player.id, p.player.overall().to_bits(), p.player.age, p.college, p.hs).hash(&mut h);
        format!("{:?}{:?}", p.stage, p.player.position).hash(&mut h);
    }
    for p in &w.programs {
        (p.prestige.to_bits(), p.head_coach, p.roster.len()).hash(&mut h);
    }
    for c in &w.coaches {
        (c.wins, c.losses).hash(&mut h);
    }
    format!("{:?}", w.log).hash(&mut h);
    w.rng.clone().next().to_bits().hash(&mut h);
    h.finish()
}

/// Everything the world's years produced (`memory_check`'s fingerprint).
fn full_print(w: &World) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for p in &w.people {
        p.name.hash(&mut h);
        format!("{:?}{:?}{:?}{:?}", p.player.history, p.stage, p.college, p.player.attributes).hash(&mut h);
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
    format!("{:?}", w.log_history).hash(&mut h);
    h.finish()
}

fn rows(path: &std::path::Path) -> Vec<serde_json::Value> {
    let f = std::fs::File::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let r = std::io::BufReader::new(zstd::stream::read::Decoder::new(f).unwrap());
    r.lines().map(|l| serde_json::from_str(&l.unwrap()).unwrap()).collect()
}

#[test]
fn the_game_log_only_watches_and_keeps_every_game() {
    let dir = std::env::temp_dir().join(format!("afm-world-log-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let opts = WorldOptions {
        seed: 21,
        scale: 0.05,
        history_years: 1,
        engine_from: Some(2025),
        hs_engine_years: Some(1),
        ..Default::default()
    };
    // The history plays its year day by day (`clock::advance_year`), every
    // game on the engine, the same path as a save's days.
    let off = World::new(opts.clone());
    let mut on = World::new(WorldOptions { log_dir: Some(dir.clone()), ..opts });
    assert_eq!(full_print(&off), full_print(&on), "the history differs with the log on");
    assert_eq!(day_print(&off), day_print(&on), "the last day differs with the log on");
    let season = 2025;
    let played = off
        .college_prev
        .iter()
        .chain(off.college.iter())
        .filter(|c| c.year == season)
        .flat_map(|c| c.games.iter())
        .filter(|g| g.played.is_some_and(|p| !p.cancelled))
        .count();
    let bytes = on.finish_log().expect("a log").expect("the log closes");
    assert!(bytes > 0);

    let meta: serde_json::Value = serde_json::from_slice(&std::fs::read(dir.join("meta.json")).unwrap()).unwrap();
    assert_eq!(meta["format"], 1);
    assert_eq!(meta["bytes"].as_u64(), Some(bytes));
    let at = |s: &str| dir.join(season.to_string()).join(format!("{s}.jsonl.zst"));
    // College: every game played that season, each with its snaps.
    let games = rows(&at("games-college"));
    let plays = rows(&at("plays-college"));
    assert_eq!(games.len(), played, "college games logged against played");
    let engine: Vec<&serde_json::Value> = games.iter().filter(|g| g.get("standin").is_none()).collect();
    assert_eq!(engine.len(), plays.len(), "every engine game has its snaps");
    for (k, g) in games.iter().enumerate() {
        assert_eq!(g["id"].as_u64(), Some(k as u64), "ids run in order");
    }
    let kinds: BTreeMap<String, usize> = games.iter().fold(BTreeMap::new(), |mut m, g| {
        *m.entry(g["kind"].as_str().unwrap_or("?").to_string()).or_default() += 1;
        m
    });
    assert!(kinds.contains_key("regular") && kinds.len() >= 3, "{kinds:?}");
    let g = engine[0];
    for k in ["date", "week", "teams", "score", "quarters", "box", "players", "drives", "scoring", "conditions"] {
        assert!(g.get(k).is_some(), "a college game row lacks {k}: {g}");
    }
    let trips: u64 = engine.iter().flat_map(|g| g["box"].as_array().unwrap()).filter_map(|b| b["red_zone_trips"].as_u64()).sum();
    let tds: u64 = engine.iter().flat_map(|g| g["box"].as_array().unwrap()).filter_map(|b| b["red_zone_tds"].as_u64()).sum();
    let per = trips as f64 / (2.0 * engine.len() as f64);
    assert!((2.0..6.5).contains(&per) && tds < trips, "{per:.2} red zone trips a team game, {tds} of {trips} TDs");
    let held: f64 = engine
        .iter()
        .flat_map(|g| g["box"].as_array().unwrap())
        .map(|b| b["clock_possession"].as_f64().unwrap_or(0.0))
        .sum::<f64>()
        / engine.len() as f64;
    assert!((3300.0..3800.0).contains(&held), "{held:.0} clock seconds a game");
    let p = &plays[0]["plays"][5];
    for k in ["n", "quarter", "clock", "kind", "wp_before", "wp_after", "ep_before", "ep_after", "on_field"] {
        assert!(p.get(k).is_some(), "a play lacks {k}: {p}");
    }
    // The NFL: preseason, regular season and playoffs, with snaps.
    let nfl = rows(&at("games-nfl"));
    let nfl_kinds: BTreeMap<String, usize> = nfl.iter().fold(BTreeMap::new(), |mut m, g| {
        *m.entry(g["kind"].as_str().unwrap_or("?").to_string()).or_default() += 1;
        m
    });
    assert_eq!(nfl_kinds.get("super bowl"), Some(&1), "{nfl_kinds:?}");
    assert!(nfl_kinds.contains_key("preseason") && nfl_kinds.get("regular") >= Some(&250), "{nfl_kinds:?}");
    assert_eq!(rows(&at("plays-nfl")).len(), nfl.len());
    // High school: every game, engine games with their boxes and lines.
    let hs = rows(&at("games-hs"));
    let hs_engine = hs.iter().filter(|g| g.get("standin").is_none()).count();
    assert!(hs_engine > 1000 && hs.iter().all(|g| g["state"].is_string()), "{hs_engine} of {}", hs.len());
    // Situation rows for every level.
    let situ = rows(&at("situations"));
    for level in ["college", "nfl", "hs"] {
        assert!(situ.iter().any(|r| r["level"] == level), "no {level} situation rows");
    }
    for key in ["run gap|", "real pass rate|", "garbage|", "backfield|", "kick result|"] {
        assert!(situ.iter().any(|r| r["key"].as_str().is_some_and(|k| k.starts_with(key))), "no {key} keys");
    }
    let _ = std::fs::remove_dir_all(&dir);
}
