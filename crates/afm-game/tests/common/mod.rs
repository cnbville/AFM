//! Helpers shared by the game's test files (live talks, seam 7). Each file
//! under `tests/` is its own crate and uses only some of these, so unused
//! code is allowed here.
#![allow(dead_code)]

use serde_json::Value;

/// Keys that would carry something the game hides: a true potential or
/// ceiling, a true rating. The draft room's and the squad boards' lists,
/// merged.
pub const HIDDEN_KEYS: [&str; 6] = ["potential", "ceiling", "truth", "overall_true", "true", "pot"];

/// No hidden number anywhere in an answer: no hidden key at any depth, and
/// every staff read a range at least 2 wide. `cmd` and `path` name the place
/// in the panic.
pub fn nothing_hidden(cmd: &str, v: &Value, path: &str) {
    match v {
        Value::Object(o) => {
            for (k, x) in o {
                let p = format!("{path}.{k}");
                assert!(!HIDDEN_KEYS.contains(&k.as_str()), "{cmd}{p}");
                if k == "read"
                    && let Some(a) = x.as_array()
                    && let (Some(lo), Some(hi)) = (a.first().and_then(Value::as_f64), a.get(1).and_then(Value::as_f64))
                {
                    assert!(hi - lo >= 2.0, "{cmd}{p}: {x} is too sure to be a staff's read");
                }
                nothing_hidden(cmd, x, &p);
            }
        }
        Value::Array(a) => a.iter().enumerate().for_each(|(i, x)| nothing_hidden(cmd, x, &format!("{path}[{i}]"))),
        _ => {}
    }
}

/// `build_world` for a test. Each test file is its own program, and most
/// build one of the same few small worlds, four to eight minutes each, so
/// the screen tests spent most of their hours building the same worlds
/// again. With `AFM_TEST_WORLDS` set to a folder (Full tests sets one that
/// is new to each run), the first test to ask for a world builds it and
/// writes it there, and every test opens that file the way the library
/// opens a world, the builder included, so each test sees the same world
/// whichever file runs first. Without it the world is built as before.
pub fn build_world(g: &mut afm_game::Game, args: Value) {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    static KEYS: Mutex<Option<HashMap<String, Arc<Mutex<()>>>>> = Mutex::new(None);
    let Some(dir) = std::env::var_os("AFM_TEST_WORLDS").map(std::path::PathBuf::from) else {
        if let Err(e) = g.call("build_world", &args) {
            panic!("build_world: {e}");
        }
        return;
    };
    let key: String = args.to_string().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
    let kept = dir.join(format!("{key}.afmsave"));
    // Tests in one file run side by side: one builds, the others wait.
    let lock = KEYS.lock().unwrap().get_or_insert_with(HashMap::new).entry(key.clone()).or_default().clone();
    {
        let _held = lock.lock().unwrap_or_else(|e| e.into_inner());
        if !kept.exists() {
            if let Err(e) = g.call("build_world", &args) {
                panic!("build_world: {e}");
            }
            std::fs::create_dir_all(&dir).unwrap();
            // Written to a temporary file and renamed, so a file that is
            // there is whole.
            afm_game::setup::write(g.world.as_ref().unwrap(), &kept).unwrap();
        }
    }
    // Each test opens its own copy, so nothing written beside the world
    // reaches another test, and drops it once open (the world is in memory).
    static OPENED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = OPENED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mine = std::env::temp_dir().join(format!("afm-test-world-{}-{n}-{key}.afmsave", std::process::id()));
    std::fs::copy(&kept, &mine).unwrap();
    let opened = g.call("open_world", &serde_json::json!({ "path": mine.display().to_string() }));
    let _ = std::fs::remove_file(&mine);
    if let Err(e) = opened {
        panic!("open_world: {e}");
    }
}
