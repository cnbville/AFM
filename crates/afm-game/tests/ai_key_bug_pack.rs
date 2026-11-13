//! Live talks lane 1: the API key reaches no file of the game folder, not
//! even a bug pack. `bug` fails with no world, so this one needs a small world
//! loaded (the quick check's version of the test runs the commands that work
//! with none: `live_talk::tests`).
//!
//! The world file inside a bug pack is compressed, so a scan of its bytes
//! proves little by itself. The guarantee is structural: the key is a field of
//! `Game.ai`, not of the `World`, and `save` and `bug` write the world only.
//! The scan catches the other ways a key could get out: a note, a log, the
//! spend book, a copy in the folder.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use afm_ai::Fake;
use afm_game::Game;
use serde_json::{Value, json};

const MARKER: &str = "sk-ant-api03-MARKERzyxwvutsrqponmlkjihgfedcba9876543210-abcd";

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    match g.call(cmd, &args) {
        Ok(v) => v,
        Err(e) => panic!("{cmd}: {e}"),
    }
}

/// Every file under `dir` that holds `needle`.
fn files_holding(dir: &Path, needle: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut todo = vec![dir.to_path_buf()];
    while let Some(d) = todo.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                todo.push(p);
            } else if std::fs::read(&p).is_ok_and(|b| b.windows(needle.len()).any(|w| w == needle.as_bytes())) {
                found.push(p);
            }
        }
    }
    found
}

#[test]
fn the_key_is_in_no_file_after_a_save_and_a_bug_pack() {
    let dir = std::env::temp_dir().join(format!("afm-ai-key-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut g = Game::new(dir.clone());
    ok(
        &mut g,
        "build_world",
        json!({ "seed": 17, "history_years": 1, "scale": 0.04, "engine_from": 2026, "hs_engine_years": 0 }),
    );
    ok(&mut g, "ai_key_set", json!({ "key": MARKER }));
    // A test call on the fake, so the spend book is written too.
    g.ai.use_fake(Arc::new(Fake::new()));
    let id = ok(&mut g, "ai_test", json!({}))["ticket"].as_u64().unwrap();
    for _ in 0..200 {
        if ok(&mut g, "ai_poll", json!({ "ticket": id }))["state"] != "wait" {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let saved = ok(&mut g, "save", json!({ "name": "keytest" }));
    assert!(saved["saved"].as_str().is_some_and(|p| Path::new(p).exists()));
    let pack = ok(&mut g, "bug", json!({ "note": "the key must not be here", "screen": "settings" }));
    assert!(Path::new(pack["folder"].as_str().unwrap()).join("world.afmsave").exists());
    assert_eq!(files_holding(&dir, "MARKER"), Vec::<PathBuf>::new());
    assert_eq!(files_holding(&dir, MARKER), Vec::<PathBuf>::new());
    // The spend book is there and holds money, not a key.
    let book = std::fs::read_to_string(dir.join("ai-spend-fake.json")).unwrap();
    assert!(book.contains("\"calls\""));
    let _ = std::fs::remove_dir_all(&dir);
}
