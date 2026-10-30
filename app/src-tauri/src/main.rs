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
use serde_json::Value;

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
        r
    })
    .await
    .map_err(|e| e.to_string())?
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
    let game = Game::new(afm_game::default_home());
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
