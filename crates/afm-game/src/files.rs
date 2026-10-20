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
