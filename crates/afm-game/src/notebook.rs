//! The notebook (Cain, 2026-11-12, playtest bug 5): free notes in the top
//! bar's panel, sorted in categories. "No DBs below 47 man coverage" stays
//! written down where you can read it while you scout.
//!
//! A career's notebook is one small file beside its save, `name.afmnotes`
//! next to `name.afmsave` (JSON). Before the first save it sits in
//! `notebook/unsaved.afmnotes` in the game folder and moves beside the save
//! when you save. The save itself is untouched, so every old save loads as
//! it did. A note is written to the file the moment you make it, not only
//! when you save, so a note is never lost with the days you didn't save.
//!
//! These are plain notes. The notes on players, coaches and letters (the
//! inbox's "Notes", `lane_core/desk.rs`) stay in the save as they were.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{Game, Reply};

/// The file beside a save.
pub const EXT: &str = "afmnotes";
const VERSION: u32 = 1;
/// Caps, so one stuck key can't fill the file.
const MAX_NOTES: usize = 2000;
const MAX_TEXT: usize = 4000;
const MAX_CATEGORIES: usize = 40;
const MAX_CATEGORY_LEN: usize = 32;
/// The category of a note with none. Never a name you can give.
const GENERAL: &str = "General";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Note {
    pub id: u32,
    /// Empty: General.
    #[serde(default)]
    pub cat: String,
    pub text: String,
    /// The game date it was written, as the screens show dates.
    #[serde(default)]
    pub made: String,
    /// The game date of the last edit; empty when never edited.
    #[serde(default)]
    pub edited: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Book {
    #[serde(default)]
    pub version: u32,
    /// The next note's number: ids are never reused.
    #[serde(default)]
    pub next: u32,
    /// Your categories, in the order you made them (General is always there and not listed).
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub notes: Vec<Note>,
}

fn is_general(name: &str) -> bool {
    name.trim().eq_ignore_ascii_case(GENERAL)
}

impl Book {
    /// A file read from outside: every note's category is one the book lists.
    fn tidy(&mut self) {
        self.version = VERSION;
        let max = self.notes.iter().map(|n| n.id + 1).max().unwrap_or(0);
        self.next = self.next.max(max);
        for n in self.notes.iter_mut() {
            if is_general(&n.cat) {
                n.cat.clear();
            } else if !n.cat.is_empty() && !self.categories.contains(&n.cat) {
                self.categories.push(n.cat.clone());
            }
        }
    }

    /// A category name as typed: trimmed, one space between words, "" for General.
    fn clean(name: &str) -> Result<String, String> {
        let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
        if is_general(&name) {
            return Ok(String::new());
        }
        if name.chars().count() > MAX_CATEGORY_LEN {
            return Err(format!("A category name is {MAX_CATEGORY_LEN} letters at most."));
        }
        Ok(name)
    }

    fn existing(&self, name: &str) -> Option<String> {
        self.categories.iter().find(|k| k.eq_ignore_ascii_case(name)).cloned()
    }

    /// The category a name means, made when it is new. "baselines" finds "Baselines".
    fn resolve(&mut self, name: &str) -> Result<String, String> {
        let name = Self::clean(name)?;
        if name.is_empty() {
            return Ok(name);
        }
        if let Some(k) = self.existing(&name) {
            return Ok(k);
        }
        if self.categories.len() >= MAX_CATEGORIES {
            return Err(format!("{MAX_CATEGORIES} categories is the most. Delete one you don't use."));
        }
        self.categories.push(name.clone());
        Ok(name)
    }

    pub fn add(&mut self, text: &str, cat: &str, today: &str) -> Result<u32, String> {
        let text = text.trim();
        if text.is_empty() {
            return Err("The note is empty.".into());
        }
        if text.chars().count() > MAX_TEXT {
            return Err(format!("A note is {MAX_TEXT} letters at most. Split it in two."));
        }
        if self.notes.len() >= MAX_NOTES {
            return Err(format!("{MAX_NOTES} notes is the most. Delete some you don't need."));
        }
        let cat = self.resolve(cat)?;
        let id = self.next;
        self.next += 1;
        self.notes.push(Note { id, cat, text: text.to_string(), made: today.to_string(), edited: String::new() });
        Ok(id)
    }

    /// Changes a note's words or category, or deletes it.
    pub fn edit(
        &mut self,
        id: u32,
        text: Option<&str>,
        cat: Option<&str>,
        delete: bool,
        today: &str,
    ) -> Result<(), String> {
        let at = self.notes.iter().position(|n| n.id == id).ok_or("That note is gone.")?;
        if delete {
            self.notes.remove(at);
            return Ok(());
        }
        let text = match text {
            Some(t) => {
                let t = t.trim();
                if t.is_empty() {
                    return Err("The note is empty. Delete it instead.".into());
                }
                if t.chars().count() > MAX_TEXT {
                    return Err(format!("A note is {MAX_TEXT} letters at most. Split it in two."));
                }
                Some(t.to_string())
            }
            None => None,
        };
        let cat = match cat {
            Some(c) => Some(self.resolve(c)?),
            None => None,
        };
        let n = &mut self.notes[at];
        let mut changed = false;
        if let Some(t) = text
            && t != n.text
        {
            n.text = t;
            changed = true;
        }
        if let Some(c) = cat
            && c != n.cat
        {
            n.cat = c;
            changed = true;
        }
        if changed {
            n.edited = today.to_string();
        }
        Ok(())
    }

    /// Makes, renames or deletes a category. Deleting never deletes notes:
    /// they go to General. Renaming to a name you already have joins the two.
    pub fn category(&mut self, op: &str, name: &str, to: &str) -> Result<(), String> {
        let name = Self::clean(name)?;
        if name.is_empty() {
            return Err("General can't be changed.".into());
        }
        match op {
            "add" => {
                self.resolve(&name)?;
            }
            "rename" => {
                let from = self.existing(&name).ok_or("That category is gone.")?;
                let to = Self::clean(to)?;
                if to.is_empty() {
                    return Err("Give the category a name other than General.".into());
                }
                let into = self.existing(&to).filter(|k| *k != from);
                let new = into.clone().unwrap_or(to);
                for n in self.notes.iter_mut().filter(|n| n.cat == from) {
                    n.cat = new.clone();
                }
                let at = self.categories.iter().position(|k| *k == from).unwrap_or(0);
                if into.is_some() {
                    self.categories.remove(at);
                } else {
                    self.categories[at] = new;
                }
            }
            "delete" => {
                let from = self.existing(&name).ok_or("That category is gone.")?;
                for n in self.notes.iter_mut().filter(|n| n.cat == from) {
                    n.cat.clear();
                }
                self.categories.retain(|k| *k != from);
            }
            _ => return Err("Unknown category change.".into()),
        }
        Ok(())
    }

    /// What the screen draws: notes newest first, each category with its count.
    pub fn view(&self) -> Value {
        let mut notes: Vec<&Note> = self.notes.iter().collect();
        notes.sort_by_key(|n| std::cmp::Reverse(n.id));
        json!({
            "total": self.notes.len(),
            "general": self.notes.iter().filter(|n| n.cat.is_empty()).count(),
            "categories": self.categories.iter().map(|k| json!({ "name": k, "count": self.notes.iter().filter(|n| n.cat == *k).count() })).collect::<Vec<_>>(),
            "notes": notes.iter().map(|n| json!({ "id": n.id, "cat": n.cat, "text": n.text, "made": n.made, "edited": n.edited })).collect::<Vec<_>>(),
        })
    }
}

// --- The file ----------------------------------------------------------------------------------

/// The notebook beside a save.
pub fn beside(save: &Path) -> PathBuf {
    save.with_extension(EXT)
}

fn unsaved(home: &Path) -> PathBuf {
    home.join("notebook").join(format!("unsaved.{EXT}"))
}

/// Where this career's notebook is now.
fn path_for(g: &Game) -> PathBuf {
    match &g.save_path {
        Some(p) => beside(p),
        None => unsaved(&g.home),
    }
}

/// The notebook at a path; none there is an empty one. A file that can't be
/// read is put aside as `.broken` (never overwritten) and the notebook starts
/// empty.
fn read(path: &Path) -> Book {
    let Ok(bytes) = std::fs::read(path) else { return Book::default() };
    match serde_json::from_slice::<Book>(&bytes) {
        Ok(mut b) => {
            b.tidy();
            b
        }
        Err(e) => {
            eprintln!("notebook: {} could not be read ({e}); kept as .broken", path.display());
            let _ = std::fs::rename(path, path.with_extension(format!("{EXT}.broken")));
            Book::default()
        }
    }
}

/// Written whole to `name.partial` and renamed over the old file, so a
/// stopped game never leaves half a notebook.
fn write(path: &Path, book: &Book) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("Could not write your notebook: {e}"))?;
    }
    let mut b = book.clone();
    b.version = VERSION;
    let bytes = serde_json::to_vec_pretty(&b).map_err(|e| e.to_string())?;
    let part = path.with_extension(format!("{EXT}.partial"));
    std::fs::write(&part, bytes).map_err(|e| format!("Could not write your notebook: {e}"))?;
    std::fs::rename(&part, path).map_err(|e| format!("Could not write your notebook: {e}"))
}

fn same_file(a: &Path, b: &Path) -> bool {
    matches!((a.canonicalize(), b.canonicalize()), (Ok(x), Ok(y)) if x == y)
}

// --- Following the save --------------------------------------------------------------------------

/// A new world or a loaded save: the notebook of a career never saved goes.
pub fn on_open(home: &Path) {
    let _ = std::fs::remove_file(unsaved(home));
}

/// After a save to `to`: the notebook goes with it. A first save moves the
/// unsaved one; a "save as" copies the old save's. A save replaced by a
/// career that has no notes loses the old save's notes with it.
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

/// A save renamed: its notebook too.
pub fn on_rename(from: &Path, to: &Path) {
    let (a, b) = (beside(from), beside(to));
    if a.exists() && a != b {
        let _ = std::fs::rename(a, b);
    }
}

/// A save deleted: its notebook too.
pub fn on_delete(save: &Path) {
    let _ = std::fs::remove_file(beside(save));
}

// --- Commands ------------------------------------------------------------------------------------

pub fn call(g: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    let text = |k: &str| args.get(k).and_then(Value::as_str);
    let id = || args.get("id").and_then(Value::as_u64).map(|v| v as u32).ok_or_else(|| "Missing id.".to_string());
    Some(match cmd {
        "notebook" => Ok(read(&path_for(g)).view()),
        "notebook_add" => change(g, |b, today| {
            b.add(text("text").ok_or("Missing text.")?, text("cat").unwrap_or(""), today)?;
            Ok(())
        }),
        "notebook_edit" => change(g, |b, today| {
            let delete = args.get("delete").and_then(Value::as_bool).unwrap_or(false);
            b.edit(id()?, text("text"), text("cat"), delete, today)
        }),
        "notebook_category" => change(g, |b, _| {
            b.category(text("op").ok_or("Missing op.")?, text("name").ok_or("Missing name.")?, text("to").unwrap_or(""))
        }),
        _ => return None,
    })
}

/// Reads the notebook, changes it, writes it back and answers with the new one.
fn change(g: &Game, f: impl FnOnce(&mut Book, &str) -> Result<(), String>) -> Reply {
    let path = path_for(g);
    let today = g.world.as_ref().map(|w| crate::views::date_text(w.today)).unwrap_or_default();
    let mut book = read(&path);
    f(&mut book, &today)?;
    write(&path, &book)?;
    Ok(book.view())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::*;

    const TODAY: &str = "Thu 12 Nov 2026";

    fn temp() -> PathBuf {
        static N: AtomicU32 = AtomicU32::new(0);
        let d = std::env::temp_dir().join(format!(
            "afm-notebook-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn texts(b: &Book) -> Vec<String> {
        b.view()["notes"].as_array().unwrap().iter().map(|n| n["text"].as_str().unwrap().to_string()).collect()
    }

    #[test]
    fn notes_are_added_edited_and_deleted() {
        let mut b = Book::default();
        let a = b.add("  No DBs below 47 man coverage  ", "Baselines", TODAY).unwrap();
        let c = b.add("Watch the Texas safeties", "", TODAY).unwrap();
        // Newest first, text trimmed, General for no category.
        assert_eq!(texts(&b), ["Watch the Texas safeties", "No DBs below 47 man coverage"]);
        assert_eq!(b.notes[0].cat, "Baselines");
        assert_eq!(b.notes[1].cat, "");
        assert!(b.add("   ", "", TODAY).is_err(), "an empty note is refused");
        // An edit that changes nothing leaves the edited date empty.
        b.edit(a, Some("No DBs below 47 man coverage"), None, false, "Fri 13 Nov 2026").unwrap();
        assert_eq!(b.notes[0].edited, "");
        b.edit(a, Some("No DBs below 50 man coverage"), Some("baselines"), false, "Fri 13 Nov 2026").unwrap();
        assert_eq!(b.notes[0].text, "No DBs below 50 man coverage");
        assert_eq!(b.notes[0].edited, "Fri 13 Nov 2026");
        assert_eq!(b.categories, ["Baselines"], "a category's spelling is kept, not doubled");
        assert!(b.edit(a, Some(" "), None, false, TODAY).is_err(), "an edit can't empty a note");
        b.edit(c, None, None, true, TODAY).unwrap();
        assert!(b.edit(c, None, None, true, TODAY).is_err(), "a deleted note is gone");
        // Ids are never reused.
        let d = b.add("New", "", TODAY).unwrap();
        assert!(d > c, "{d} {c}");
    }

    #[test]
    fn categories_join_rename_and_delete_without_losing_notes() {
        let mut b = Book::default();
        b.add("one", "Baselines", TODAY).unwrap();
        b.add("two", "  scouting   notes ", TODAY).unwrap();
        b.add("three", "General", TODAY).unwrap();
        assert_eq!(b.categories, ["Baselines", "scouting notes"]);
        assert_eq!(b.notes[2].cat, "", "General is no category");
        assert!(b.category("add", "general", "").is_err(), "General can't be made or changed");
        assert!(b.category("add", &"x".repeat(40), "").is_err(), "a long name is refused");
        b.category("add", "Recruiting", "").unwrap();
        b.category("add", "RECRUITING", "").unwrap();
        assert_eq!(b.categories.len(), 3, "the same name in other letters is the same category");
        b.category("rename", "scouting notes", "Scouting").unwrap();
        assert_eq!(b.notes[1].cat, "Scouting");
        // Renaming onto another category joins the two.
        b.category("rename", "Scouting", "baselines").unwrap();
        assert_eq!(b.categories, ["Baselines", "Recruiting"]);
        assert!(b.notes.iter().filter(|n| n.cat == "Baselines").count() == 2);
        // Deleting a category keeps its notes, in General.
        b.category("delete", "Baselines", "").unwrap();
        assert_eq!(b.notes.len(), 3);
        assert!(b.notes.iter().all(|n| n.cat.is_empty()));
        assert_eq!(b.view()["general"], 3);
        assert!(b.category("delete", "Baselines", "").is_err());
    }

    #[test]
    fn limits_hold() {
        let mut b = Book::default();
        assert!(b.add(&"x".repeat(MAX_TEXT + 1), "", TODAY).is_err());
        assert!(b.add(&"x".repeat(MAX_TEXT), "", TODAY).is_ok());
        for i in 0..MAX_CATEGORIES {
            b.category("add", &format!("c{i}"), "").unwrap();
        }
        assert!(b.category("add", "one more", "").is_err());
    }

    #[test]
    fn the_file_round_trips_and_a_broken_one_is_kept_aside() {
        let d = temp();
        let p = d.join("a.afmnotes");
        assert!(read(&p).notes.is_empty(), "no file is an empty notebook");
        let mut b = Book::default();
        b.add("keep me", "Baselines", TODAY).unwrap();
        write(&p, &b).unwrap();
        let back = read(&p);
        assert_eq!(texts(&back), ["keep me"]);
        assert_eq!(back.categories, ["Baselines"]);
        // A note whose category the file forgot to list gets it listed.
        std::fs::write(&p, r#"{"notes":[{"id":4,"cat":"Scouting","text":"x"},{"id":2,"cat":"general","text":"y"}]}"#)
            .unwrap();
        let odd = read(&p);
        assert_eq!(odd.categories, ["Scouting"]);
        assert_eq!(odd.notes[1].cat, "");
        assert_eq!(odd.next, 5, "ids go on after the highest");
        // A file that isn't a notebook is not overwritten.
        std::fs::write(&p, "not json").unwrap();
        assert!(read(&p).notes.is_empty());
        assert!(d.join("a.afmnotes.broken").exists());
        assert!(!p.exists());
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn the_commands_work_before_any_save_and_the_notebook_follows_the_save() {
        let d = temp();
        let mut g = Game::new(d.clone());
        let r = g.call("notebook_add", &json!({ "text": "No DBs below 47 man coverage", "cat": "Baselines" })).unwrap();
        assert_eq!(r["total"], 1);
        assert_eq!(r["categories"][0]["name"], "Baselines");
        let id = r["notes"][0]["id"].as_u64().unwrap();
        assert!(d.join("notebook/unsaved.afmnotes").exists(), "an unsaved career keeps it in the game folder");

        // The first save moves it beside the save.
        let save = d.join("saves").join("first.afmsave");
        std::fs::write(&save, "save").unwrap();
        on_save(&d, g.save_path.as_deref(), &save);
        g.save_path = Some(save.clone());
        assert!(d.join("saves/first.afmnotes").exists());
        assert!(!d.join("notebook/unsaved.afmnotes").exists());
        let r = g.call("notebook", &json!({})).unwrap();
        assert_eq!(r["notes"][0]["text"], "No DBs below 47 man coverage");

        // Edits go to the file beside the save.
        let r = g
            .call("notebook_edit", &json!({ "id": id, "text": "No DBs below 50 man coverage", "cat": "Scouting" }))
            .unwrap();
        assert_eq!(r["notes"][0]["cat"], "Scouting");
        assert!(g.call("notebook_edit", &json!({ "id": 99, "delete": true })).unwrap_err().contains("gone"));
        let r =
            g.call("notebook_category", &json!({ "op": "rename", "name": "Scouting", "to": "Man coverage" })).unwrap();
        assert_eq!(r["categories"][1]["name"], "Man coverage");

        // Saving again to the same save leaves it alone.
        on_save(&d, g.save_path.as_deref(), &save);
        assert_eq!(read(&beside(&save)).notes.len(), 1);

        // Save as: a copy, and the first save keeps its own.
        let second = d.join("saves").join("second.afmsave");
        on_save(&d, g.save_path.as_deref(), &second);
        assert!(beside(&second).exists() && beside(&save).exists());
        g.call("notebook_add", &json!({ "text": "Only in the first" })).unwrap();
        assert_eq!(read(&beside(&second)).notes.len(), 1, "the copy does not follow");

        // Rename and delete follow the save.
        let third = d.join("saves").join("third.afmsave");
        on_rename(&second, &third);
        assert!(beside(&third).exists() && !beside(&second).exists());
        on_delete(&third);
        assert!(!beside(&third).exists());

        // A save replaced by a career with no notes loses the old notes.
        let mut fresh = Game::new(d.clone());
        on_open(&d);
        on_save(&d, fresh.save_path.as_deref(), &save);
        assert!(!beside(&save).exists());
        // Opening another career clears the unsaved one.
        fresh.call("notebook_add", &json!({ "text": "scratch" })).unwrap();
        on_open(&d);
        assert_eq!(fresh.call("notebook", &json!({})).unwrap()["total"], 0);
        let _ = std::fs::remove_dir_all(d);
    }

    /// Cain, 2026-11-13: the notebook is per career, never shared across careers.
    #[test]
    fn each_career_has_its_own_notebook() {
        let d = temp();
        let a = d.join("saves").join("a.afmsave");
        let b = d.join("saves").join("b.afmsave");
        std::fs::create_dir_all(d.join("saves")).unwrap();
        std::fs::write(&a, "save").unwrap();
        std::fs::write(&b, "save").unwrap();
        let note =
            |g: &mut Game, text: &str| g.call("notebook_add", &json!({ "text": text, "cat": "Baselines" })).unwrap();

        // Career A is saved with its own note.
        let mut g = Game::new(d.clone());
        note(&mut g, "A's note");
        on_save(&d, g.save_path.as_deref(), &a);
        g.save_path = Some(a.clone());

        // A new world is a new career: it starts with an empty notebook, and A's file is untouched.
        on_open(&d);
        g.save_path = None;
        let r = g.call("notebook", &json!({})).unwrap();
        assert_eq!(r["total"], 0, "a new career starts with no notes");
        assert_eq!(r["categories"].as_array().unwrap().len(), 0, "and none of the old categories");
        note(&mut g, "B's note");
        on_save(&d, g.save_path.as_deref(), &b);
        g.save_path = Some(b.clone());
        assert_eq!(texts(&read(&beside(&a))), ["A's note"]);
        assert_eq!(texts(&read(&beside(&b))), ["B's note"]);

        // Loading A shows A's notes only; a note written there stays out of B.
        on_open(&d);
        g.save_path = Some(a.clone());
        assert_eq!(g.call("notebook", &json!({})).unwrap()["notes"][0]["text"], "A's note");
        note(&mut g, "More for A");
        assert_eq!(texts(&read(&beside(&b))), ["B's note"], "a note in one career never reaches another");

        // And back to B.
        on_open(&d);
        g.save_path = Some(b.clone());
        let r = g.call("notebook", &json!({})).unwrap();
        assert_eq!(r["total"], 1);
        assert_eq!(r["notes"][0]["text"], "B's note");
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn a_save_over_itself_under_another_spelling_keeps_its_notes() {
        let d = temp();
        let save = d.join("s.afmsave");
        std::fs::write(&save, "save").unwrap();
        let mut b = Book::default();
        b.add("keep", "", TODAY).unwrap();
        write(&beside(&save), &b).unwrap();
        // The same file named another way is not a "save as".
        std::fs::create_dir_all(d.join("sub")).unwrap();
        let other = d.join("sub").join("..").join("s.afmsave");
        assert_ne!(other, save);
        on_save(&d, Some(&save), &other);
        assert_eq!(texts(&read(&beside(&save))), ["keep"]);
        let _ = std::fs::remove_dir_all(d);
    }
}
