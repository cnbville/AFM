//! World and save files (section 8, topic 3). The format is the engine's
//! (`afm_engine::save`, agreed with the world-building thread): an
//! `AFMWORLD` header, a format version, a JSON block about the world, then
//! the world as named MessagePack through zstd, in parts packed and read on
//! every core (format 2, update 3) or as one stream (format 1, still read).
//! Named fields let an older file load into a newer game: what a newer game
//! added fills in at load (`World::after_load`).

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
    let card = Some(crate::cards::card(w));
    afm_engine::save::write_world(w, &tmp, env!("CARGO_PKG_VERSION"), &written, card).map_err(|e| e.to_string())?;
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
    // Written beside the library first and moved in whole, so a world it
    // replaces (a newer build of the same world) is never left half written.
    let whole = name.strip_suffix(".001").unwrap_or(name);
    let to = worlds.join(whole);
    if to == path {
        return read_meta(path).map(|_| to);
    }
    let part = to.with_extension("afmworld.part");
    let made = (|| -> Result<(), String> {
        if whole == name {
            read_meta(path)?;
            std::fs::copy(path, &part).map_err(|e| format!("Could not copy the world: {e}"))?;
            return Ok(());
        }
        let mut out = std::fs::File::create(&part).map_err(|e| format!("Could not write the world: {e}"))?;
        for i in 1.. {
            let piece = path.with_file_name(format!("{whole}.{i:03}"));
            if !piece.exists() {
                break;
            }
            let mut f = std::fs::File::open(&piece).map_err(|e| format!("Could not read {}: {e}", piece.display()))?;
            std::io::copy(&mut f, &mut out).map_err(|e| format!("Could not join the world: {e}"))?;
        }
        drop(out);
        read_meta(&part).map(|_| ())
    })()
    .and_then(|_| std::fs::rename(&part, &to).map_err(|e| format!("Could not add the world: {e}")));
    if let Err(e) = made {
        let _ = std::fs::remove_file(&part);
        return Err(e);
    }
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

#[cfg(test)]
mod tests {
    use super::{import_world, read_meta};
    use std::path::Path;

    /// A world file's header with `commit`, and some body after it.
    fn fake(path: &Path, commit: &str, body: &[u8]) {
        let meta = serde_json::json!({ "seed": 2026, "history_years": 30, "start_year": 2026, "commit": commit,
            "written": "2026-10-23 20:55 UTC", "world_date": "2026-02-09" })
        .to_string();
        let mut b = afm_engine::save::MAGIC.to_vec();
        b.extend(afm_engine::save::FORMAT_VERSION.to_le_bytes());
        b.extend((meta.len() as u32).to_le_bytes());
        b.extend(meta.as_bytes());
        b.extend(body);
        std::fs::write(path, b).unwrap();
    }

    #[test]
    fn a_newer_build_replaces_the_library_world_whole() {
        let dir = std::env::temp_dir().join(format!("afm-import-{}", std::process::id()));
        let (from, worlds) = (dir.join("download"), dir.join("worlds"));
        std::fs::create_dir_all(&from).unwrap();
        std::fs::create_dir_all(&worlds).unwrap();
        fake(&worlds.join("w.afmworld"), "old", b"old body");
        // A single file replaces the library's copy.
        fake(&from.join("w.afmworld"), "new", b"new body");
        let to = import_world(&from.join("w.afmworld"), &worlds).unwrap();
        assert_eq!(to, worlds.join("w.afmworld"));
        assert_eq!(read_meta(&to).unwrap()["commit"], "new");
        // A world in parts is joined, then replaces it.
        fake(&dir.join("whole"), "parts", &[7u8; 5000]);
        let whole = std::fs::read(dir.join("whole")).unwrap();
        let (a, b) = whole.split_at(1000);
        std::fs::write(from.join("w.afmworld.001"), a).unwrap();
        std::fs::write(from.join("w.afmworld.002"), b).unwrap();
        import_world(&from.join("w.afmworld.001"), &worlds).unwrap();
        assert_eq!(std::fs::read(worlds.join("w.afmworld")).unwrap(), whole);
        // Something that isn't a world leaves the library's copy as it was.
        std::fs::write(from.join("x.afmworld"), b"not a world").unwrap();
        std::fs::copy(from.join("x.afmworld"), from.join("w.afmworld")).unwrap();
        assert!(import_world(&from.join("w.afmworld"), &worlds).is_err());
        assert_eq!(std::fs::read(worlds.join("w.afmworld")).unwrap(), whole);
        assert!(!worlds.join("w.afmworld.part").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
