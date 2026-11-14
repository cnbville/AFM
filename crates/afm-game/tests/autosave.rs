//! The autosave (2026-11-14): a forked copy of the game writes the save while
//! the game goes on. The checks that matter: the file is the world as it stood
//! at the moment of the tick even though the game plays on at once, a running
//! autosave is ended by a save by hand, a killed one leaves the old file as it
//! was, nothing half-written is left in the saves folder, and nothing is saved
//! when nothing was done.

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::time::Duration;

use afm_game::Game;
use serde_json::{Value, json};

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    match g.call(cmd, &args) {
        Ok(v) => v,
        Err(e) => panic!("{cmd}: {e}"),
    }
}

/// Waits for a running autosave to end and returns the answer.
fn wait(g: &mut Game) -> Value {
    for _ in 0..480 {
        let s = ok(g, "autosave_status", json!({}));
        if s["state"] != "saving" {
            return s;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    panic!("the autosave did not end");
}

fn world_date(path: &Path) -> String {
    afm_game::files::read_meta(path).unwrap()["world_date"].as_str().unwrap().to_string()
}

fn partials(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir.join("saves"))
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "partial"))
        .collect()
}

#[test]
fn the_autosave_is_the_world_at_the_tick_and_never_in_the_way() {
    let dir = std::env::temp_dir().join(format!("afm-autosave-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut g = Game::new(dir.clone());
    ok(&mut g, "build_world", json!({ "seed": 7, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    let setup = ok(&mut g, "setup", json!({}));
    let ratings: Vec<f64> = setup["backgrounds"].as_array().unwrap()[0]["ratings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect();
    let job = ok(&mut g, "jobs", json!({ "level": "college" }))["jobs"][0]["ref"].clone();
    ok(
        &mut g,
        "start_career",
        json!({ "name": "Test Coach", "age": 41, "background": "college_assistant", "job": job, "seed": 99,
                "ratings": ratings, "home_state": "TX", "calls": "defense" }),
    );
    let auto = dir.join("saves").join("Autosave - Test Coach.afmsave");

    // Not yet ten minutes: nothing happens, however much is done.
    ok(&mut g, "home", json!({}));
    assert_eq!(ok(&mut g, "autosave_tick", json!({}))["state"], "idle");
    assert!(!auto.exists());

    // Ten minutes (shortened): the tick forks and answers at once.
    g.autosave.every = Duration::ZERO;
    ok(&mut g, "home", json!({}));
    let at_tick = g.world.as_ref().unwrap().today.to_string();
    let t = std::time::Instant::now();
    let s = ok(&mut g, "autosave_tick", json!({}));
    assert!(t.elapsed() < Duration::from_secs(2), "the tick answered in {:?}", t.elapsed());
    assert_eq!(s["state"], "saving", "{s}");
    // A second tick while it writes starts nothing more.
    assert_eq!(ok(&mut g, "autosave_tick", json!({}))["state"], "saving");
    // The game plays on at once; the file must still be the world at the tick.
    for _ in 0..3 {
        ok(&mut g, "day", json!({}));
    }
    let after = g.world.as_ref().unwrap().today.to_string();
    assert_ne!(after, at_tick, "the game moved on");
    let s = wait(&mut g);
    assert_eq!(s["state"], "saved", "{s}");
    assert_eq!(world_date(&auto), at_tick, "the autosave is the world as it stood at the tick");
    assert!(partials(&dir).is_empty());
    // It loads like any save, and plays on.
    let path = auto.display().to_string();
    ok(&mut g, "load", json!({ "path": path }));
    assert_eq!(g.world.as_ref().unwrap().today.to_string(), at_tick);

    // Nothing done since the last save: nothing is saved, even when due.
    g.autosave.every = Duration::ZERO;
    assert_eq!(ok(&mut g, "autosave_tick", json!({}))["state"], "idle");

    // A save by hand ends a running autosave: never two at once.
    ok(&mut g, "day", json!({}));
    let moved = g.world.as_ref().unwrap().today.to_string();
    assert_eq!(ok(&mut g, "autosave_tick", json!({}))["state"], "saving");
    let hand = ok(&mut g, "save", json!({ "name": "by hand" }))["saved"].as_str().unwrap().to_string();
    assert_eq!(ok(&mut g, "autosave_status", json!({}))["state"], "idle");
    assert_eq!(world_date(Path::new(&hand)), moved);
    std::thread::sleep(Duration::from_secs(1));
    assert!(partials(&dir).is_empty(), "{:?}", partials(&dir));
    // The autosave on disk is still whole, from the earlier moment.
    assert_eq!(world_date(&auto), at_tick);

    // An autosave killed while it writes leaves the old file as it was.
    ok(&mut g, "day", json!({}));
    assert_eq!(ok(&mut g, "autosave_tick", json!({}))["state"], "saving");
    g.autosave.cancel();
    std::thread::sleep(Duration::from_secs(1));
    assert!(partials(&dir).is_empty());
    assert_eq!(world_date(&auto), at_tick);

    // Opening a world ends it too, and the next career starts its clock over.
    g.autosave.every = Duration::ZERO;
    ok(&mut g, "day", json!({}));
    assert_eq!(ok(&mut g, "autosave_tick", json!({}))["state"], "saving");
    ok(&mut g, "load", json!({ "path": hand }));
    assert_eq!(ok(&mut g, "autosave_status", json!({}))["state"], "idle");
    let _ = std::fs::remove_dir_all(&dir);
}
