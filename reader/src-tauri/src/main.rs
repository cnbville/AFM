//! The AFM World Reader on the desktop (lane R5, spec section 5): one window
//! with the screens in `reader/ui`, and one command, `call`, that hands a
//! screen's request to the reader. The reader runs on its own thread, so a
//! long read never freezes the window.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, Mutex};

use afm_reader_app::Reader;
use serde_json::Value;

struct Shared(Arc<Mutex<Reader>>);

#[tauri::command]
async fn call(reader: tauri::State<'_, Shared>, cmd: String, args: Value) -> Result<Value, String> {
    let r = reader.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut r = r.lock().map_err(|_| "the reader stopped after an error; restart it".to_string())?;
        r.call(&cmd, &args)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn main() {
    let reader = Reader::new(afm_reader_app::default_home(), afm_reader_app::game_home());
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        // Updates itself from the public downloads page (`plugins.updater` in
        // tauri.conf.json; the screens check at start, reader.js `checkUpdate`).
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(Shared(Arc::new(Mutex::new(reader))))
        .invoke_handler(tauri::generate_handler![call])
        .run(tauri::generate_context!())
        .expect("the AFM World Reader could not start");
}
