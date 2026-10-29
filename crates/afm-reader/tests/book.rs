//! A small world with its world log, written as a world file, then its
//! book with the log and without it: every check, distribution and
//! consistency check's SQL runs on the engine's own field names, every
//! page prints, and a book without a log says what it lacks.
//! `AFM_BOOK_TEST_DIR=dir` keeps the world, the log and the books.

use afm_engine::world::wlog::WorldLog;
use afm_engine::world::{World, WorldOptions};
use afm_reader::{Book, Filters, make};

#[test]
fn a_book_from_a_logged_world() {
    let keep = std::env::var("AFM_BOOK_TEST_DIR").ok();
    let dir =
        keep.clone().map_or_else(|| std::env::temp_dir().join(format!("afm-book-{}", std::process::id())), Into::into);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let scale = std::env::var("AFM_BOOK_TEST_SCALE").ok().and_then(|s| s.parse().ok()).unwrap_or(0.002);
    let opts = WorldOptions { scale, history_years: 2, ..Default::default() };
    let world = dir.join("w.afmworld");
    let log = dir.join("w.afmlog");
    let mut w =
        World::new_logged(opts.clone(), Some(WorldLog::create(&log, &opts, "test", "").unwrap()), |_, _| true).unwrap();
    w.detach_log().unwrap().close().unwrap();
    afm_engine::save::write_world(&w, &world, "test", "2026-10-28", None).unwrap();
    let first = w.people[0].name.clone();
    drop(w);

    // With the log beside it.
    let with = dir.join("with.afmbook");
    let s = make::build(&world, make::log_beside(&world).as_deref(), &with, &mut |_| {}).unwrap();
    assert!(s.has_log && s.checks > 30 && s.tables > 100, "{s:?}");
    assert!(s.missing.is_empty(), "the game log and the world log are both in: {:?}", s.missing);
    let b = Book::open(&with).unwrap();
    for t in [
        "people",
        "programs",
        "coaches",
        "alumni",
        "log_players",
        "log_growth",
        "log_games",
        "log_games__box",
        "log_plays",
        "log_situations",
        "checks",
        "dist",
        "odd",
        "coverage",
        "knobs",
        "known_issues",
        "search",
        "defs",
    ] {
        assert!(b.num(&format!("SELECT COUNT(*) FROM \"{t}\"")).unwrap_or(0.0) > 0.0, "{t} is empty");
    }
    // Every piece of SQL runs: a failure here is a field the engine renamed.
    let (_, bad) = b
        .rows(
            "SELECT 'check', id, detail FROM checks WHERE detail LIKE 'Its SQL failed%'
             UNION ALL SELECT 'distribution', id, note FROM metrics WHERE note LIKE 'SQL error%'
             UNION ALL SELECT 'consistency', id, why FROM consistency
             WHERE status = 'error' OR (status = 'no data' AND needs != 'game log')",
        )
        .unwrap();
    assert!(bad.is_empty(), "SQL that no longer reads the world: {bad:?}");
    // Every page prints, and a person is found by name.
    let f = Filters::default();
    for p in afm_reader::pages::PAGES {
        let text = b.text(p.id, &f);
        assert!(text.starts_with("# "), "{}: {text}", p.id);
        assert!(b.page(p.id, &f)["sections"].as_array().is_some_and(|s| !s.is_empty()), "{}", p.id);
    }
    for p in [
        "checks",
        "odd",
        "issues",
        "knobs",
        "coverage",
        "tables",
        "table people",
        "check G1.fbs_points",
        "dist program_wins",
    ] {
        assert!(b.text(p, &f).len() > 50, "{p}");
    }
    let found = b.text(&format!("search {first}"), &f);
    assert!(found.contains("person"), "{found}");
    let id = b
        .text_of(&format!("SELECT id FROM search WHERE kind = 'person' AND name = '{}'", first.replace('\'', "''")))
        .unwrap();
    assert!(b.text(&format!("person {id}"), &f).contains(&first));
    drop(b);

    // From the world file alone.
    let without = dir.join("without.afmbook");
    let s = make::build(&world, None, &without, &mut |_| {}).unwrap();
    assert!(!s.has_log && s.missing.len() == 2, "{:?}", s.missing);
    let b = Book::open(&without).unwrap();
    let text = b.text("overview", &f);
    assert!(text.contains("Not in this book: world log"), "{text}");
    assert!(b.num("SELECT COUNT(*) FROM checks WHERE mark = 'no data' AND needs = 'world log'").is_some());
    drop(b);
    if keep.is_none() {
        let _ = std::fs::remove_dir_all(&dir);
    }
}
