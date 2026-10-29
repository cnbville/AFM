//! The world log (world reader lane R2) only watches: a world with the log
//! on plays exactly as one with it off, every person on every day, and the
//! log reads back whole, through a save and a reopen.

use crate::world::wlog::{WorldLog, fp, read};
use crate::world::{SPIN_UP_YEARS, World, WorldOptions};

/// Every person's print, and the world's other stores and random stream.
fn prints(w: &World) -> (Vec<u64>, [u64; 6]) {
    (
        w.people.iter().map(fp::of).collect(),
        [fp::of(&w.rng), fp::of(&w.coaches), fp::of(&w.programs), fp::of(&w.clubs), fp::of(&w.alumni), fp::of(&w.news)],
    )
}

fn count(dir: &std::path::Path, kind: &str) -> u64 {
    read::kind(dir, kind).unwrap().iter().map(|s| read::each_json(&s.path, |_| {}).unwrap()).sum()
}

#[test]
fn the_world_log_only_watches_and_reads_back() {
    let opts = WorldOptions { scale: 0.002, history_years: 2, ..Default::default() };
    let dir = std::env::temp_dir().join(format!("afm-world-log-{}", std::process::id()));
    // One world through its spin-up, then two copies of it read back the
    // way the game opens a world: one plays with the log, one without.
    let mut w = World::empty(opts.clone());
    for age in [17, 16, 15] {
        w.add_class_aged(age);
    }
    for _ in 0..SPIN_UP_YEARS {
        w.advance_year();
    }
    w.start_recording();
    let file = dir.with_extension("afmworld");
    crate::save::write_world(&w, &file, "test", "", None).unwrap();
    drop(w);
    let open = || {
        let mut w = crate::save::read_world(&file).unwrap();
        w.after_load();
        w
    };
    let (mut a, mut b) = (open(), open());
    let _ = std::fs::remove_file(&file);
    b.attach_log(WorldLog::create(&dir, &opts, "test", "").unwrap());
    let end = a.year + opts.history_years as i32;
    let mut days = 0;
    while a.year < end {
        a.advance_day();
        b.advance_day();
        days += 1;
        let (pa, sa) = prints(&a);
        let (pb, sb) = prints(&b);
        if let Some(k) = (0..pa.len().max(pb.len())).find(|&k| pa.get(k) != pb.get(k)) {
            panic!("{}: person {k} ({}) differs with the log on", a.today, a.people[k].name);
        }
        assert_eq!(sa, sb, "{}: a store differs with the log on", a.today);
    }
    assert!(days > 700);
    // Every field a person has is in his yearly row or his `people` row.
    let missing = crate::world::wlog::rows::missing_fields(&b.people[0]);
    assert!(missing.is_empty(), "not in the log: {missing:?}");
    // A save keeps the log as it stood; the live log goes on after it.
    let mut log = b.detach_log().unwrap();
    let saved = dir.with_extension("saved");
    log.save_to(&saved).unwrap();
    log.close().unwrap();
    for kind in [
        "people",
        "players",
        "growth",
        "coaches",
        "programs",
        "schools",
        "clubs",
        "exits",
        "events",
        "news",
        "snapshots",
        "whys",
        "world",
        "games-college",
        "plays-college",
        "games-nfl",
        "plays-nfl",
        "games-other",
        "plays-other",
        "games-hs",
        "situations",
        "team-situations",
    ] {
        let n = count(&saved, kind);
        assert!(n > 0, "no {kind} records");
        assert_eq!(n, count(&dir, kind), "{kind} differs in the save");
    }
    games(&saved, opts.history_years);
    // One player row per person at each book, and a growth row per step.
    let rows = count(&saved, "players");
    assert!(rows as usize >= b.people.len(), "{rows} rows for {} people", b.people.len());
    // Typed reading: the world rows carry the fingerprints.
    let mut years = Vec::new();
    for s in read::kind(&saved, "world").unwrap() {
        read::each_json(&s.path, |v| years.push(v["season"].as_i64().unwrap())).unwrap();
    }
    assert_eq!(years.len(), opts.history_years as usize);
    // Reopened for a career: new segments follow the save's.
    let more = WorldLog::resume(&saved, "test", &b.today.to_string(), None).unwrap();
    b.attach_log(more);
    for _ in 0..3 {
        b.advance_day();
    }
    b.detach_log().unwrap().close().unwrap();
    assert_eq!(read::meta(&saved).unwrap().sessions.len(), 2);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&saved);
}

fn all(dir: &std::path::Path, kind: &str) -> Vec<serde_json::Value> {
    let mut out = Vec::new();
    for s in read::kind(dir, kind).unwrap() {
        read::each_json(&s.path, |v| out.push(v)).unwrap();
    }
    out
}

/// The game log (lane R1): every game with its facts, box, lines and
/// drives, every engine game's snaps, and the season's situation totals.
fn games(dir: &std::path::Path, years: u32) {
    let college = all(dir, "games-college");
    let engine: Vec<&serde_json::Value> = college.iter().filter(|g| g.get("standin").is_none()).collect();
    assert!(engine.len() > 100, "{} college engine games", engine.len());
    for k in ["level", "season", "date", "week", "kind", "teams", "score", "quarters", "box", "players", "drives"] {
        assert!(engine[0].get(k).is_some(), "a college game row lacks {k}: {}", engine[0]);
    }
    assert!(college.iter().any(|g| g["kind"] == "bowl"), "no bowl games");
    let boxes = || engine.iter().flat_map(|g| g["box"].as_array().unwrap());
    let trips: f64 = boxes().filter_map(|b| b["red_zone_trips"].as_f64()).sum();
    let tds: f64 = boxes().filter_map(|b| b["red_zone_tds"].as_f64()).sum();
    assert!(trips > engine.len() as f64 && tds < trips, "{trips} red zone trips, {tds} TDs in {} games", engine.len());
    let held: f64 = boxes().filter_map(|b| b["clock_possession"].as_f64()).sum::<f64>() / engine.len() as f64;
    assert!((3300.0..3800.0).contains(&held), "{held:.0} clock seconds a game");
    // Each engine game's snaps, named like its row.
    let plays = all(dir, "plays-college");
    assert_eq!(plays.len(), engine.len(), "every college engine game has its snaps");
    let p = &plays[0]["plays"][5];
    for k in ["n", "quarter", "clock", "kind", "wp_before", "wp_after", "ep_before", "ep_after", "on_field"] {
        assert!(p.get(k).is_some(), "a snap lacks {k}: {p}");
    }
    assert!(engine.iter().any(|g| g["date"] == plays[0]["date"] && g["teams"] == plays[0]["teams"]));
    // The NFL from preseason to the Super Bowl, each game with its snaps.
    let nfl = all(dir, "games-nfl");
    let kind = |k: &str| nfl.iter().filter(|g| g["kind"] == k).count();
    assert_eq!(kind("super bowl"), years as usize);
    assert!(kind("preseason") > 0 && kind("wild card") > 0 && kind("regular") >= 250 * years as usize);
    assert_eq!(all(dir, "plays-nfl").len(), nfl.len());
    // High school: engine games with their boxes and lines, by state.
    let hs = all(dir, "games-hs");
    assert!(hs.iter().any(|g| g.get("standin").is_none() && g.get("players").is_some()));
    assert!(hs.iter().all(|g| g["state"].is_string()));
    // Situation totals for every level, high school by state.
    let situ = all(dir, "situations");
    for level in ["college", "nfl", "other", "hs"] {
        assert!(situ.iter().any(|r| r["level"] == level), "no {level} situation totals");
    }
    let keys: Vec<&str> = situ.iter().map(|r| r["key"].as_str().unwrap()).collect();
    assert!(situ.iter().all(|r| r["v"].as_array().is_some_and(|v| v.len() == 8)));
    for key in ["run gap|", "real pass rate|", "garbage|", "backfield|", "kick result|", "call|"] {
        assert!(keys.iter().any(|k| k.starts_with(key)), "no {key} keys");
    }
    assert!(
        situ.iter()
            .filter(|r| r["level"] == "hs")
            .all(|r| r["group"] == "all" || r["group"].as_str().unwrap().len() == 2)
    );
    assert!(all(dir, "team-situations").iter().all(|r| r["level"] != "hs"));
}

/// The game log builds its rows lean (`wlog/lean.rs`): one pass that keeps
/// only the fields that are not zero, false or empty, rounded the log's
/// way. It has to hold the same as the full value compacted afterwards.
#[test]
fn lean_rows_match_the_full_value_compacted() {
    use crate::game::drives::drives;
    use crate::game::quick_game;
    use crate::game::synthetic::synthetic_league;
    use crate::player::Level;
    use crate::world::wlog::games::compact;
    use crate::world::wlog::lean::lean;
    // Lean drops every empty field as it goes; only a variant keeps its
    // name over an empty value (`{"Challenge": 0}`: the home side
    // challenged), which compacting afterwards lost.
    fn no_empty(v: &serde_json::Value, what: &str) {
        match v {
            serde_json::Value::Object(m) => {
                for (k, x) in m {
                    let variant = k.starts_with(|c: char| c.is_ascii_uppercase());
                    assert!(variant || !crate::world::wlog::games::empty(x), "{what} kept {k}: {x}");
                    no_empty(x, what);
                }
            }
            serde_json::Value::Array(a) => a.iter().for_each(|x| no_empty(x, what)),
            _ => {}
        }
    }
    let same = |what: &str, full: serde_json::Value, lean: serde_json::Value| {
        no_empty(&lean, what);
        let (mut a, mut b) = (full, lean);
        compact(&mut a);
        compact(&mut b);
        assert_eq!(a, b, "{what}");
    };
    for level in [Level::HS, Level::FBS, Level::NFL] {
        let teams = synthetic_league(level, 4, 11);
        for g in 0..3u64 {
            let r = quick_game(&teams[g as usize], &teams[3 - g as usize], level, 9_100 + g);
            let s = &r.stats;
            for side in 0..2 {
                same("team box", serde_json::to_value(&s.team[side]).unwrap(), lean(&s.team[side]));
                for (id, l) in &s.players[side] {
                    same(&format!("player {id}"), serde_json::to_value(l).unwrap(), lean(l));
                }
            }
            for p in &s.plays {
                same(&format!("{level:?} snap {}", p.n), serde_json::to_value(p).unwrap(), lean(p));
            }
            same("drives", serde_json::to_value(drives(s)).unwrap(), lean(&drives(s)));
        }
    }
}
