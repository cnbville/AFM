//! AFM on the desktop (section 8, topic 9): one window with the screens in
//! `app/ui`, and one command, `call`, that hands a screen's request to the
//! game. The game runs on its own thread, so a long Continue never freezes
//! the window.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, Mutex};

use afm_game::Game;
use serde_json::Value;

struct Engine(Arc<Mutex<Game>>);

#[tauri::command]
async fn call(engine: tauri::State<'_, Engine>, cmd: String, args: Value) -> Result<Value, String> {
    let game = engine.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut g = game.lock().map_err(|_| "the game stopped after an error; restart AFM".to_string())?;
        g.call(&cmd, &args)
    })
    .await
    .map_err(|e| e.to_string())?
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
