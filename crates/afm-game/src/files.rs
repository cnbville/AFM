//! World and save files (section 8, topic 3). The format is the engine's
//! (`afm_engine::save`, agreed with the world-building thread): an
//! `AFMWORLD` header, a format version, a JSON block about the world, then
//! the whole world as named MessagePack through zstd.

use std::path::Path;

use afm_engine::world::World;
use serde_json::Value;

pub fn read_world(path: &Path) -> Result<World, String> {
    let _ = path;
    Err("save files arrive with the engine's save module".into())
}

pub fn write_world(w: &World, path: &Path) -> Result<(), String> {
    let _ = (w, path);
    Err("save files arrive with the engine's save module".into())
}

/// The JSON block at the front of a world file, read without loading it.
pub fn read_meta(path: &Path) -> Result<Value, String> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut head = [0u8; 16];
    f.read_exact(&mut head).map_err(|e| e.to_string())?;
    if &head[..8] != b"AFMWORLD" {
        return Err("not an AFM world file".into());
    }
    let version = u32::from_le_bytes(head[8..12].try_into().unwrap_or_default());
    let len = u32::from_le_bytes(head[12..16].try_into().unwrap_or_default()) as usize;
    let mut meta = vec![0u8; len.min(1 << 20)];
    f.read_exact(&mut meta).map_err(|e| e.to_string())?;
    let mut v: Value = serde_json::from_slice(&meta).map_err(|e| e.to_string())?;
    v["format"] = Value::from(version);
    Ok(v)
}
