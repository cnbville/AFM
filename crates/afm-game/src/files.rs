//! World and save files (section 8, topic 3). The format is the engine's
//! (`afm_engine::save`, agreed with the world-building thread): an
//! `AFMWORLD` header, a format version, a JSON block about the world, then
//! the whole world as named MessagePack through zstd. Named fields let an
//! older file load into a newer game: what a newer game added fills in at
//! load (`World::after_load`).

use std::path::Path;

use afm_engine::world::World;
use serde_json::Value;

pub fn read_world(path: &Path) -> Result<World, String> {
    let mut w = afm_engine::save::read_world(path).map_err(|e| e.to_string())?;
    w.after_load();
    Ok(w)
}

pub fn write_world(w: &World, path: &Path) -> Result<(), String> {
    let written = format!("{:?}", std::time::SystemTime::now());
    // Written to a temporary file first, so a failed save never breaks the old one.
    let tmp = path.with_extension("partial");
    afm_engine::save::write_world(w, &tmp, env!("CARGO_PKG_VERSION"), &written).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

/// The JSON block at the front of a world file, read without loading it.
pub fn read_meta(path: &Path) -> Result<Value, String> {
    let m = afm_engine::save::read_meta(path).map_err(|e| e.to_string())?;
    serde_json::to_value(m).map_err(|e| e.to_string())
}
