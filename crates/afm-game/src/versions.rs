//! Going back to an older version of the app (Settings, "Other versions").
//! The app side installs the older build (`app/src-tauri`, `install_version`);
//! this is the game's side of it: what an older build can't open, a copy of
//! your saves kept beside them first, and the pause on updates.
//!
//! - `versions_check`: which saves and library worlds were written by a game
//!   newer than the build you are going back to (the writer's code time in the
//!   file's header against that build's commit time). An older game says "made
//!   by a newer version" and leaves such a file alone, but it can still open
//!   one that has only picked up small additions and then save it over, which
//!   drops what the newer game added. So the screens say how many there are.
//! - `versions_backup`: before the app is replaced, every file in the saves
//!   folder is kept in `saves-backup/<label>-<time>/`. A save is only ever
//!   written to a temporary file and renamed over the old one (`files::
//!   write_world`, the autosave), so the old file stays whole under a second
//!   name: the big saves are hard links (no space, no wait), the small files
//!   beside them (notebooks, plans) are copied, because those are rewritten
//!   in place. Nothing in your saves folder is moved or changed.
//! - The pause (`update-pin.json` in the game folder): while it stands the
//!   app doesn't update by itself, so a build you went back to stays. The
//!   Settings button "Go back to the latest" clears it. A build from before
//!   the pause existed can't honour it (it never reads the file).

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::Game;

const PIN: &str = "update-pin.json";
/// A file beside a save that is copied, not linked, is copied up to here.
const COPY_LIMIT: u64 = 64 * 1024 * 1024;

fn pin_path(home: &Path) -> PathBuf {
    home.join(PIN)
}

/// "YYYY-MM-DD HH:MM UTC": the form a world file's header writes, which
/// compares as text.
fn is_time(t: &str) -> bool {
    t.len() == 20 && t.ends_with(" UTC") && t.as_bytes()[4] == b'-'
}

/// The pause on updates, or null when there is none.
pub fn pin_get(g: &Game) -> Value {
    std::fs::read_to_string(pin_path(&g.home))
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .filter(|v| v.get("version").is_some_and(Value::is_string))
        .unwrap_or(Value::Null)
}

/// Pauses updates on the build you went back to (`version`, "0.2.492").
pub fn pin_set(g: &Game, args: &Value) -> Result<Value, String> {
    let version = args.get("version").and_then(Value::as_str).filter(|v| !v.is_empty()).ok_or("Missing version.")?;
    let pin = json!({
        "version": version,
        "from": args.get("from").and_then(Value::as_str).unwrap_or(""),
        "at": crate::files::now_utc(),
    });
    std::fs::create_dir_all(&g.home).map_err(|e| e.to_string())?;
    // Written whole beside it and renamed, like every file here.
    let tmp = pin_path(&g.home).with_extension("partial");
    std::fs::write(&tmp, pin.to_string()).map_err(|e| format!("Could not pause updates: {e}"))?;
    std::fs::rename(&tmp, pin_path(&g.home)).map_err(|e| format!("Could not pause updates: {e}"))?;
    Ok(pin)
}

/// Lets the app update by itself again.
pub fn pin_clear(g: &Game) -> Result<Value, String> {
    match std::fs::remove_file(pin_path(&g.home)) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("Could not lift the pause on updates: {e}")),
    }
    Ok(Value::Null)
}

/// The files of one folder with one extension, for the check.
fn world_files(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == ext))
        .collect();
    v.sort();
    v
}

/// Which saves and library worlds a build of `before` (its commit time,
/// "YYYY-MM-DD HH:MM UTC") may not read: those whose writer's code is later.
/// A file from before the writer's stamp (2026-11-15) is older than any
/// build you can go back to, and one with no stamp at all is left out.
pub fn check(g: &Game, args: &Value) -> Result<Value, String> {
    let before = args.get("before").and_then(Value::as_str).unwrap_or("");
    let known = is_time(before);
    let mut out = Vec::new();
    let mut total = [0usize; 2];
    for (i, (dir, ext, kind)) in [("saves", "afmsave", "save"), ("worlds", "afmworld", "world")].into_iter().enumerate()
    {
        for path in world_files(&g.home.join(dir), ext) {
            total[i] += 1;
            let Ok(meta) = crate::files::read_meta(&path) else { continue };
            let code = meta.get("code").and_then(Value::as_str).unwrap_or("");
            if known && is_time(code) && code > before {
                out.push(json!({
                    "kind": kind,
                    "name": path.file_stem().map(|s| s.to_string_lossy().to_string()),
                    "code": code,
                    "written": meta.get("written").cloned().unwrap_or(Value::Null),
                    "career": meta.get("career").cloned().unwrap_or(Value::Null),
                }));
            }
        }
    }
    Ok(json!({ "known": known, "before": before, "saves": total[0], "worlds": total[1], "newer": out }))
}

/// A folder name from a label: letters, digits, dots and dashes.
fn label_safe(s: &str) -> String {
    let t: String =
        s.chars().map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' { c } else { '-' }).collect();
    let t = t.trim_matches('-');
    if t.is_empty() { "backup".into() } else { t.to_string() }
}

/// Keeps every file of the saves folder whole in `saves-backup/` before the
/// app is replaced (see the top of this file for how).
pub fn backup(g: &Game, args: &Value) -> Result<Value, String> {
    let label = label_safe(args.get("label").and_then(Value::as_str).unwrap_or("before-going-back"));
    let saves = g.home.join("saves");
    let stamp: String = crate::files::now_utc().chars().filter(char::is_ascii_digit).collect();
    let root = g.home.join("saves-backup");
    let mut dir = root.join(format!("{label}-{stamp}"));
    let mut n = 1;
    while dir.exists() {
        n += 1;
        dir = root.join(format!("{label}-{stamp}-{n}"));
    }
    let entries: Vec<PathBuf> = std::fs::read_dir(&saves)
        .map(|r| r.flatten().map(|e| e.path()).filter(|p| p.is_file()).collect())
        .unwrap_or_default();
    // Nothing to keep: no folder is made.
    if entries.is_empty() {
        return Ok(json!({ "folder": Value::Null, "kept": 0, "linked": 0, "copied": 0, "left": [] }));
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("Could not make the backup folder: {e}"))?;
    let (mut linked, mut copied) = (0usize, 0usize);
    let mut left = Vec::new();
    for from in entries {
        let Some(name) = from.file_name() else { continue };
        let to = dir.join(name);
        let big = from.extension().is_some_and(|x| x == "afmsave");
        let size = from.metadata().map_or(0, |m| m.len());
        let ok = if big {
            // Never rewritten in place, so the link keeps the old bytes. Where
            // the disk won't link, the save is copied, whatever its size: a
            // save is what this backup is for.
            std::fs::hard_link(&from, &to)
                .map(|()| linked += 1)
                .or_else(|_| std::fs::copy(&from, &to).map(|_| copied += 1))
                .is_ok()
        } else if size <= COPY_LIMIT {
            std::fs::copy(&from, &to).map(|_| copied += 1).is_ok()
        } else {
            false
        };
        if !ok {
            left.push(name.to_string_lossy().to_string());
        }
    }
    Ok(
        json!({ "folder": dir.display().to_string(), "kept": linked + copied, "linked": linked, "copied": copied, "left": left }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A save file: the header with the writer's code time and a body.
    fn fake(path: &Path, code: Option<&str>, body: &[u8]) {
        let mut meta = json!({ "seed": 2026, "history_years": 30, "start_year": 2026, "commit": "x",
            "written": "2026-11-15 20:00 UTC", "world_date": "2026-02-09" });
        if let Some(c) = code {
            meta["code"] = json!(c);
        }
        let meta = meta.to_string();
        let mut b = afm_engine::save::MAGIC.to_vec();
        b.extend(afm_engine::save::FORMAT_VERSION.to_le_bytes());
        b.extend((meta.len() as u32).to_le_bytes());
        b.extend(meta.as_bytes());
        b.extend(body);
        std::fs::write(path, b).unwrap();
    }

    fn home(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("afm-versions-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("saves")).unwrap();
        std::fs::create_dir_all(d.join("worlds")).unwrap();
        d
    }

    #[test]
    fn the_check_names_what_a_build_cannot_read() {
        let h = home("check");
        let g = Game::new(h.clone());
        fake(&h.join("saves/old.afmsave"), Some("2026-11-10 09:00 UTC"), b"a");
        fake(&h.join("saves/new.afmsave"), Some("2026-11-15 21:00 UTC"), b"b");
        fake(&h.join("saves/unstamped.afmsave"), None, b"c");
        fake(&h.join("worlds/w.afmworld"), Some("2026-11-15 22:00 UTC"), b"d");
        let r = check(&g, &json!({ "before": "2026-11-15 13:00 UTC" })).unwrap();
        assert_eq!((r["saves"].as_u64(), r["worlds"].as_u64()), (Some(3), Some(1)));
        let names: Vec<String> =
            r["newer"].as_array().unwrap().iter().map(|x| x["name"].as_str().unwrap().to_string()).collect();
        assert_eq!(names, ["new", "w"], "{r}");
        // A build whose time isn't known claims nothing.
        let r = check(&g, &json!({ "before": "someday" })).unwrap();
        assert!(r["newer"].as_array().unwrap().is_empty() && r["known"] == false, "{r}");
        let _ = std::fs::remove_dir_all(&h);
    }

    #[test]
    fn the_backup_keeps_a_save_whole_when_a_newer_one_is_written_over_it() {
        let h = home("backup");
        let g = Game::new(h.clone());
        let save = h.join("saves/career.afmsave");
        fake(&save, Some("2026-11-15 21:00 UTC"), b"the 0.2.540 career");
        std::fs::write(h.join("saves/career.afmnotes"), b"my notes").unwrap();
        let r = backup(&g, &json!({ "label": "before 0.2.492" })).unwrap();
        assert_eq!((r["linked"].as_u64(), r["copied"].as_u64()), (Some(1), Some(1)), "{r}");
        let folder = PathBuf::from(r["folder"].as_str().unwrap());
        assert!(folder.starts_with(h.join("saves-backup")) && folder.to_string_lossy().contains("before-0.2.492-"));
        let kept = std::fs::read(folder.join("career.afmsave")).unwrap();
        // The way a save is written: a temporary file renamed over the old one.
        let tmp = save.with_extension("partial");
        fake(&tmp, Some("2026-11-14 01:00 UTC"), b"the 0.2.492 career, saved over it");
        std::fs::rename(&tmp, &save).unwrap();
        assert_eq!(std::fs::read(folder.join("career.afmsave")).unwrap(), kept, "the backup changed");
        assert_ne!(std::fs::read(&save).unwrap(), kept);
        // The small file beside it is its own copy, so a rewrite in place leaves the backup alone.
        std::fs::write(h.join("saves/career.afmnotes"), b"changed").unwrap();
        assert_eq!(std::fs::read(folder.join("career.afmnotes")).unwrap(), b"my notes");
        // A second backup never reuses the first one's folder.
        let again = backup(&g, &json!({ "label": "before 0.2.492" })).unwrap();
        assert_ne!(again["folder"], r["folder"]);
        let _ = std::fs::remove_dir_all(&h);
    }

    #[test]
    fn no_saves_make_no_backup_folder() {
        let h = home("none");
        let g = Game::new(h.clone());
        let r = backup(&g, &json!({})).unwrap();
        assert!(r["folder"].is_null() && !h.join("saves-backup").exists(), "{r}");
        let _ = std::fs::remove_dir_all(&h);
    }

    #[test]
    fn the_pause_on_updates_is_kept_and_lifted() {
        let h = home("pin");
        let g = Game::new(h.clone());
        assert!(pin_get(&g).is_null());
        pin_set(&g, &json!({ "version": "0.2.492", "from": "0.2.540" })).unwrap();
        let p = pin_get(&g);
        assert_eq!((p["version"].as_str(), p["from"].as_str()), (Some("0.2.492"), Some("0.2.540")), "{p}");
        assert!(pin_set(&g, &json!({})).is_err());
        pin_clear(&g).unwrap();
        pin_clear(&g).unwrap();
        assert!(pin_get(&g).is_null());
        // A file that isn't a pause is no pause.
        std::fs::write(h.join(PIN), "not json").unwrap();
        assert!(pin_get(&g).is_null());
        let _ = std::fs::remove_dir_all(&h);
    }
}
