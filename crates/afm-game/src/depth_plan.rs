//! Plan ahead on the depth chart (Cain, 2026-11-13): your own private chart
//! for each of the next three falls, a reminder of who you have starting
//! when you recruit. Nothing in the game reads it: no player, recruit, talk,
//! promise or AI team ever sees it, and it is not part of the world or the
//! save file. It is one small file beside the save, `name.afmplan`, the way
//! the notebook is `name.afmnotes`. Before a career's first save it sits in
//! `depthplan/unsaved.afmplan` in the game folder and moves beside the save
//! when you save. A "save as" copies it, a rename and a delete follow the
//! save, and a new world or a loaded save clears the unsaved one. Every
//! change is written at once, so a plan is never lost with the days you
//! didn't save.
//!
//! What it holds: for a fall and a position, the men in the position's order
//! (every spot's starter, then every spot's second man, then every spot's
//! third, as the live depth list reads), with a hole where nobody is
//! planned. A position you haven't touched in that fall has no entry and
//! follows your chart now (`lane_squadboards/plan.rs`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::Game;

/// The file beside a save.
pub const EXT: &str = "afmplan";
const VERSION: u32 = 1;
const DIR: &str = "depthplan";

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Plan {
    #[serde(default)]
    pub version: u32,
    /// Fall -> position name -> the position's cells (a player id, or none).
    #[serde(default)]
    pub years: BTreeMap<String, BTreeMap<String, Vec<Option<u32>>>>,
}

impl Plan {
    pub fn get(&self, fall: i32, pos: &str) -> Option<&Vec<Option<u32>>> {
        self.years.get(&fall.to_string())?.get(pos)
    }

    pub fn set(&mut self, fall: i32, pos: &str, cells: Vec<Option<u32>>) {
        self.years.entry(fall.to_string()).or_default().insert(pos.to_string(), cells);
    }

    /// Gives positions back to your chart now in one fall.
    pub fn clear(&mut self, fall: i32, positions: &[&str]) {
        let key = fall.to_string();
        if let Some(y) = self.years.get_mut(&key) {
            for p in positions {
                y.remove(*p);
            }
            if y.is_empty() {
                self.years.remove(&key);
            }
        }
    }

    /// Whether any of these positions is yours in this fall.
    pub fn has_any(&self, fall: i32, positions: &[&str]) -> bool {
        self.years.get(&fall.to_string()).is_some_and(|y| positions.iter().any(|p| y.contains_key(*p)))
    }

    /// Falls that have come (the live chart takes over) are dropped.
    pub fn prune(&mut self, current_fall: i32) {
        self.years.retain(|k, _| k.parse::<i32>().is_ok_and(|f| f > current_fall));
    }
}

// --- The file ----------------------------------------------------------------------------------

/// The plan beside a save.
pub fn beside(save: &Path) -> PathBuf {
    save.with_extension(EXT)
}

fn unsaved(home: &Path) -> PathBuf {
    home.join(DIR).join(format!("unsaved.{EXT}"))
}

/// Where this career's plan is now.
pub fn path_for(g: &Game) -> PathBuf {
    match &g.save_path {
        Some(p) => beside(p),
        None => unsaved(&g.home),
    }
}

/// The plan at a path; none there is an empty one. A file that can't be
/// read is put aside as `.broken` (never overwritten) and the plan starts
/// empty.
pub fn read(path: &Path) -> Plan {
    let Ok(bytes) = std::fs::read(path) else { return Plan::default() };
    match serde_json::from_slice::<Plan>(&bytes) {
        Ok(mut p) => {
            p.version = VERSION;
            p
        }
        Err(e) => {
            eprintln!("depth plan: {} could not be read ({e}); kept as .broken", path.display());
            let _ = std::fs::rename(path, path.with_extension(format!("{EXT}.broken")));
            Plan::default()
        }
    }
}

/// Written whole to `name.partial` and renamed over the old file, so a
/// stopped game never leaves half a plan. An empty plan removes the file.
pub fn write(path: &Path, plan: &Plan) -> Result<(), String> {
    if plan.years.is_empty() {
        let _ = std::fs::remove_file(path);
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("Could not write your plan: {e}"))?;
    }
    let mut p = plan.clone();
    p.version = VERSION;
    let bytes = serde_json::to_vec_pretty(&p).map_err(|e| e.to_string())?;
    let part = path.with_extension(format!("{EXT}.partial"));
    std::fs::write(&part, bytes).map_err(|e| format!("Could not write your plan: {e}"))?;
    std::fs::rename(&part, path).map_err(|e| format!("Could not write your plan: {e}"))
}

fn same_file(a: &Path, b: &Path) -> bool {
    matches!((a.canonicalize(), b.canonicalize()), (Ok(x), Ok(y)) if x == y)
}

// --- Following the save --------------------------------------------------------------------------

/// A new world or a loaded save: the plan of a career never saved goes.
pub fn on_open(home: &Path) {
    let _ = std::fs::remove_file(unsaved(home));
}

/// After a save to `to`: the plan goes with it. A first save moves the
/// unsaved one; a "save as" copies the old save's. A save replaced by a
/// career that has no plan loses the old save's plan with it.
pub fn on_save(home: &Path, from: Option<&Path>, to: &Path) {
    let dest = beside(to);
    let src = match from {
        None => unsaved(home),
        Some(f) if f != to => beside(f),
        _ => return,
    };
    if same_file(&src, &dest) {
        return;
    }
    if !src.exists() {
        let _ = std::fs::remove_file(&dest);
        return;
    }
    let _ = std::fs::remove_file(&dest);
    if from.is_none() && std::fs::rename(&src, &dest).is_ok() {
        return;
    }
    if std::fs::copy(&src, &dest).is_ok() && from.is_none() {
        let _ = std::fs::remove_file(&src);
    }
}

/// A save renamed: its plan too.
pub fn on_rename(from: &Path, to: &Path) {
    let (a, b) = (beside(from), beside(to));
    if a.exists() && a != b {
        let _ = std::fs::rename(a, b);
    }
}

/// A save deleted: its plan too.
pub fn on_delete(save: &Path) {
    let _ = std::fs::remove_file(beside(save));
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::*;

    fn temp() -> PathBuf {
        static N: AtomicU32 = AtomicU32::new(0);
        let d = std::env::temp_dir().join(format!(
            "afm-depthplan-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn cells_keep_their_holes_and_come_back() {
        let d = temp();
        let p = d.join("a.afmplan");
        assert_eq!(read(&p), Plan::default(), "no file is an empty plan");
        let mut plan = Plan::default();
        plan.set(2028, "QB", vec![Some(7), None, Some(9)]);
        plan.set(2028, "RB", vec![None]);
        plan.set(2029, "QB", vec![Some(1), Some(2), Some(3)]);
        write(&p, &plan).unwrap();
        let back = read(&p);
        assert_eq!(back.get(2028, "QB"), Some(&vec![Some(7), None, Some(9)]));
        assert!(back.has_any(2029, &["QB"]) && !back.has_any(2029, &["RB"]));
        // Giving positions back removes them, and the empty year with them.
        let mut b = back;
        b.clear(2028, &["QB", "RB"]);
        assert!(b.get(2028, "QB").is_none() && !b.years.contains_key("2028"));
        // A fall that has come is dropped.
        b.prune(2029);
        assert!(b.years.is_empty());
        write(&p, &b).unwrap();
        assert!(!p.exists(), "an empty plan leaves no file");
    }

    #[test]
    fn a_broken_file_is_kept_and_the_plan_starts_empty() {
        let d = temp();
        let p = d.join("a.afmplan");
        std::fs::write(&p, "not json").unwrap();
        assert!(read(&p).years.is_empty());
        assert!(d.join("a.afmplan.broken").exists() && !p.exists());
    }

    #[test]
    fn the_plan_follows_the_save() {
        let d = temp();
        let mut plan = Plan::default();
        plan.set(2028, "QB", vec![Some(5)]);
        // An unsaved career keeps it in the game folder.
        write(&unsaved(&d), &plan).unwrap();
        let save = d.join("saves").join("one.afmsave");
        std::fs::create_dir_all(save.parent().unwrap()).unwrap();
        std::fs::write(&save, "save").unwrap();
        // The first save moves it beside the save.
        on_save(&d, None, &save);
        assert!(beside(&save).exists() && !unsaved(&d).exists());
        // A save as copies it; the copy does not follow later changes.
        let two = d.join("saves").join("two.afmsave");
        std::fs::write(&two, "save").unwrap();
        on_save(&d, Some(&save), &two);
        assert!(beside(&two).exists() && beside(&save).exists());
        // Rename and delete follow the save.
        let three = d.join("saves").join("three.afmsave");
        on_rename(&two, &three);
        assert!(beside(&three).exists() && !beside(&two).exists());
        on_delete(&three);
        assert!(!beside(&three).exists());
        // A save over a career with no plan loses the old save's plan.
        on_save(&d, None, &save);
        assert!(!beside(&save).exists());
        // Opening another career clears the unsaved one.
        write(&unsaved(&d), &plan).unwrap();
        on_open(&d);
        assert!(!unsaved(&d).exists());
    }
}
