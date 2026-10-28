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
    ] {
        let n = count(&saved, kind);
        assert!(n > 0, "no {kind} records");
        assert_eq!(n, count(&dir, kind), "{kind} differs in the save");
    }
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
