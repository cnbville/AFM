//! AFM on the desktop (section 8, topic 9): one window with the screens in
//! `app/ui`, and one command, `call`, that hands a screen's request to the
//! game. The game runs on its own thread, so a long Continue never freezes
//! the window.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use afm_game::Game;
use serde_json::{Value, json};

mod keychain;

struct Engine(Arc<Mutex<Game>>);

#[tauri::command]
async fn call(engine: tauri::State<'_, Engine>, cmd: String, args: Value) -> Result<Value, String> {
    let game = engine.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let asked = Instant::now();
        let mut g = game.lock().map_err(|_| "the game stopped after an error; restart AFM".to_string())?;
        let waited = asked.elapsed();
        let r = g.call(&cmd, &args);
        slow_call(&g.home, &cmd, waited, asked.elapsed());
        if !(matches!(cmd.as_str(), "ai_key_set" | "ai_key_clear") && r.is_ok()) {
            return r;
        }
        // The Keychain may stop to ask the person (a new build is signed
        // afresh), so it runs with the game unlocked: the screens must not
        // freeze behind a question.
        drop(g);
        let saved = follow_keychain(&cmd, &args);
        let mut g = game.lock().map_err(|_| "the game stopped after an error; restart AFM".to_string())?;
        Ok(keychain_status(&mut g, &cmd, &args, saved))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// The live talks key follows the Keychain (live talks lane 1): a key pasted
/// into settings is saved there, and a key cleared is removed. The game has
/// already checked and kept it in memory. An error is the plain note for the
/// screen (the key then lasts until AFM quits). The key is never logged.
fn follow_keychain(cmd: &str, args: &Value) -> Result<(), String> {
    if cmd == "ai_key_set" {
        let key = args.get("key").and_then(Value::as_str).unwrap_or("").trim();
        keychain::set(key)
            .map_err(|e| format!("The key could not be saved in the Keychain, so it lasts until you quit AFM. ({e})"))
    } else {
        keychain::clear().map_err(|e| format!("The saved key could not be removed from the Keychain. ({e})"))
    }
}

/// The answer to `ai_key_set` and `ai_key_clear`: the game's status, with
/// where the key is kept and, if the Keychain would not take it, its note.
/// A new key reads as kept in the Keychain only when it was saved there and
/// is still the key the game holds.
fn keychain_status(g: &mut Game, cmd: &str, args: &Value, saved: Result<(), String>) -> Value {
    if cmd == "ai_key_set" && saved.is_ok() {
        let key = args.get("key").and_then(Value::as_str).unwrap_or("").trim();
        if g.ai.key_tail().as_deref() == key.get(key.len().saturating_sub(4)..) {
            g.ai.set_kept("keychain");
        }
    }
    let mut status = g.ai.status();
    if let Err(note) = saved {
        status["keychain_note"] = json!(note);
    }
    status
}

/// A call that took two seconds or more goes into `logs/slow-calls.log` in
/// the game folder, with how long it waited for the game and how long in
/// all, so a freeze on a player's Mac can be read afterwards (2026-10-30).
fn slow_call(home: &Path, cmd: &str, waited: Duration, took: Duration) {
    if took < Duration::from_secs(2) {
        return;
    }
    let dir = home.join("logs");
    let path = dir.join("slow-calls.log");
    // Kept small: past 256 KB the old lines move to slow-calls.old.log.
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > 256 * 1024) {
        let _ = std::fs::rename(&path, dir.join("slow-calls.old.log"));
    }
    let at = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(
            f,
            "{at} {cmd} waited {:.1} s, took {:.1} s in all (app {})",
            waited.as_secs_f64(),
            took.as_secs_f64(),
            env!("CARGO_PKG_VERSION")
        );
    }
}

fn main() {
    let mut game = Game::new(afm_game::default_home());
    // The live talks key, saved in the Keychain, is handed to the game once.
    match keychain::get() {
        Ok(Some(k)) => {
            if game.ai.set_key(&k).is_ok() {
                game.ai.set_kept("keychain");
            }
        }
        Ok(None) => {}
        Err(e) => eprintln!("AFM: the Keychain would not give up the live talks key. {e}"),
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        // Updates itself from the public downloads page (`plugins.updater` in
        // tauri.conf.json; the screens check at start, app.js `checkUpdate`).
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(Engine(Arc::new(Mutex::new(game))))
        .invoke_handler(tauri::generate_handler![call])
        .run(tauri::generate_context!())
        .expect("AFM could not start");
}
