//! World and save files (section 8, topic 3). The format is the engine's
//! (`afm_engine::save`, agreed with the world-building thread): an
//! `AFMWORLD` header, a format version, a JSON block about the world, then
//! the whole world as named MessagePack through zstd. Named fields let an
//! older file load into a newer game: what a newer game added fills in at
//! load (`World::after_load`).

use std::path::{Path, PathBuf};

use afm_engine::world::World;
use serde_json::Value;

pub fn read_world(path: &Path) -> Result<World, String> {
    let mut w = afm_engine::save::read_world(path).map_err(|e| e.to_string())?;
    w.after_load();
    Ok(w)
}

pub fn write_world(w: &World, path: &Path) -> Result<(), String> {
    let written = now_utc();
    // Written to a temporary file first, so a failed save never breaks the old one.
    let tmp = path.with_extension("partial");
    afm_engine::save::write_world(w, &tmp, env!("CARGO_PKG_VERSION"), &written).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

/// The JSON block at the front of a world file, read without loading it.
pub fn read_meta(path: &Path) -> Result<Value, String> {
    let mut m = afm_engine::save::read_meta(path).map_err(|e| e.to_string())?;
    // Files written before the readable date carry the date's raw day count.
    if let Some(n) = m.world_date.strip_prefix("Date(").and_then(|x| x.strip_suffix(')')).and_then(|x| x.parse().ok()) {
        m.world_date = afm_engine::calendar::Date(n).to_string();
    }
    serde_json::to_value(m).map_err(|e| e.to_string())
}

/// Copies a world file into the library folder and checks it opens. A world
/// split into parts (`name.afmworld.001`, `.002`, ...) is joined back into
/// `name.afmworld` when the first part, or the folder holding them, is given.
pub fn import_world(path: &Path, worlds: &Path) -> Result<PathBuf, String> {
    // A folder holding the world (whole or in parts) imports as its world file.
    if path.is_dir() {
        let mut found: Vec<PathBuf> = std::fs::read_dir(path)
            .map_err(|e| format!("Could not read the folder: {e}"))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.to_str().is_some_and(|s| s.ends_with(".afmworld") || s.ends_with(".afmworld.001")))
            .collect();
        found.sort();
        let file = found.into_iter().next().ok_or("No world file in that folder.")?;
        return import_world(&file, worlds);
    }
    let name = path.file_name().and_then(|n| n.to_str()).ok_or("Not a file.")?;
    let to = match name.strip_suffix(".001") {
        Some(whole) => {
            let to = worlds.join(whole);
            let mut out = std::fs::File::create(&to).map_err(|e| format!("Could not write the world: {e}"))?;
            for i in 1.. {
                let part = path.with_file_name(format!("{whole}.{i:03}"));
                if !part.exists() {
                    break;
                }
                let mut f =
                    std::fs::File::open(&part).map_err(|e| format!("Could not read {}: {e}", part.display()))?;
                std::io::copy(&mut f, &mut out).map_err(|e| format!("Could not join the world: {e}"))?;
            }
            drop(out);
            if let Err(e) = read_meta(&to) {
                let _ = std::fs::remove_file(&to);
                return Err(e);
            }
            to
        }
        None => {
            read_meta(path)?;
            let to = worlds.join(name);
            if to != path {
                std::fs::copy(path, &to).map_err(|e| format!("Could not copy the world: {e}"))?;
            }
            to
        }
    };
    Ok(to)
}

/// The time now as `YYYY-MM-DD HH:MM UTC`.
pub fn now_utc() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs()) as i64;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Civil date from days since 1970 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + (m <= 2) as i64;
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02} UTC", rem / 3600, rem % 3600 / 60)
}
