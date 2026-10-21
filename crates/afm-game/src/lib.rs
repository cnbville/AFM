//! The AFM game (section 8): your career on the engine's world, as one
//! command interface. The desktop app (Tauri) and the browser test server
//! both call `Game::call` with a command name and JSON arguments and get
//! JSON back, so every screen works the same in both.
//!
//! Nothing here shows a hidden number: attributes come as your staff's
//! ranges, potential as words, coaches as reputation (scouting topic 1,
//! Q46). The engine's `debug` feature is never on in this crate.

pub mod colors;
pub mod files;
pub mod jobs;
pub mod reads;
pub mod setup;
pub mod views;

use std::path::PathBuf;

use afm_engine::world::{World, WorldOptions};
use serde_json::{Value, json};

/// A running game: the world (once loaded) and where its files live.
pub struct Game {
    pub world: Option<World>,
    /// Where worlds, saves and bug packs are kept.
    pub home: PathBuf,
    /// The save file this career was last saved to or loaded from.
    pub save_path: Option<PathBuf>,
    /// A world being built or downloaded beside the game.
    pub job: jobs::Shared,
}

pub type Reply = Result<Value, String>;

impl Game {
    pub fn new(home: PathBuf) -> Game {
        for d in ["worlds", "saves", "bugs"] {
            let _ = std::fs::create_dir_all(home.join(d));
        }
        Game { world: None, home, save_path: None, job: Default::default() }
    }

    fn w(&self) -> Result<&World, String> {
        self.world.as_ref().ok_or_else(|| "No world is loaded.".to_string())
    }

    fn wm(&mut self) -> Result<&mut World, String> {
        self.world.as_mut().ok_or_else(|| "No world is loaded.".to_string())
    }

    /// Runs one command. Unknown commands and bad arguments come back as
    /// errors in plain words.
    pub fn call(&mut self, cmd: &str, args: &Value) -> Reply {
        let num = |k: &str| args.get(k).and_then(Value::as_u64);
        let id = || num("id").map(|v| v as u32).ok_or_else(|| "Missing id.".to_string());
        match cmd {
            "hello" => {
                Ok(json!({ "game": "AFM", "version": env!("CARGO_PKG_VERSION"), "loaded": self.world.is_some() }))
            }
            "library" => Ok(setup::library(self)),
            "build_world" => {
                let seed = num("seed").unwrap_or(2026) as u32;
                let years = num("history_years").unwrap_or(30) as u32;
                let scale = args.get("scale").and_then(Value::as_f64).unwrap_or(1.0);
                let engine_from = num("engine_from").map(|y| y as i32);
                self.world = Some(World::new(WorldOptions {
                    seed,
                    history_years: years,
                    scale,
                    engine_from,
                    ..Default::default()
                }));
                self.save_path = None;
                Ok(json!({ "built": true }))
            }
            // A full world built on this machine, into the library (hours).
            "build_world_file" => {
                let seed = num("seed").unwrap_or(2026) as u32;
                let years = num("history_years").unwrap_or(30).clamp(1, 60) as u32;
                let scale = args.get("scale").and_then(Value::as_f64).unwrap_or(1.0);
                Ok(json!({ "name": jobs::build(&self.job, &self.home.join("worlds"), seed, years, scale)? }))
            }
            "online_worlds" => jobs::online(&self.home.join("worlds")),
            "download_world" => {
                let tag = args.get("tag").and_then(Value::as_str).ok_or("Missing tag.")?;
                jobs::download(&self.job, &self.home.join("worlds"), tag)?;
                Ok(json!({ "started": true }))
            }
            "job" => Ok(jobs::status(&self.job)),
            "import_world" => {
                // Copies a world file into the library, where it stays for every new career.
                let path = PathBuf::from(args.get("path").and_then(Value::as_str).ok_or("Missing path.")?);
                let to = files::import_world(&path, &self.home.join("worlds"))?;
                Ok(serde_json::json!({ "path": to.to_string_lossy() }))
            }
            "open_world" | "load" => {
                let path = args.get("path").and_then(Value::as_str).ok_or("Missing path.")?;
                self.world = Some(setup::read(path)?);
                self.save_path = (cmd == "load").then(|| PathBuf::from(path));
                Ok(views::status(self.w()?))
            }
            "save" => {
                let name = args.get("name").and_then(Value::as_str);
                let path = match name {
                    Some(n) => self.home.join("saves").join(format!("{}.afmsave", setup::file_safe(n))),
                    None => self.save_path.clone().ok_or("Name the save first.")?,
                };
                setup::write(self.w()?, &path)?;
                self.save_path = Some(path.clone());
                Ok(json!({ "saved": path.display().to_string() }))
            }
            "setup" => Ok(setup::options(self.w()?)),
            "jobs" => Ok(setup::jobs(self.w()?, args.get("level").and_then(Value::as_str).unwrap_or("college"))),
            "start_career" => {
                let w = self.wm()?;
                setup::start(w, args)?;
                Ok(views::status(w))
            }
            "status" => Ok(views::status(self.w()?)),
            "continue" => {
                let max = num("max_days").unwrap_or(400) as u32;
                let w = self.wm()?;
                let from = w.today;
                let stopped = w.continue_until_needed(max);
                Ok(views::stopped(w, from, stopped))
            }
            "day" => {
                let w = self.wm()?;
                let from = w.today;
                w.advance_day();
                Ok(views::stopped(w, from, from))
            }
            "holiday" => {
                let days = num("days").unwrap_or(7) as i32;
                let w = self.wm()?;
                let from = w.today;
                let stopped = w.holiday_to(from.add(days));
                Ok(views::stopped(w, from, stopped))
            }
            "home" => Ok(views::home(self.w()?)),
            "inbox" => Ok(views::inbox(self.w()?, num("days").unwrap_or(60) as i32)),
            "news" => Ok(views::news(self.w()?, args)),
            "calendar" => Ok(views::calendar(self.w()?)),
            "team" => Ok(views::team(self.w()?, views::TeamRef::from_args(self.w()?, args)?)),
            "roster" => Ok(views::roster(self.w()?, views::TeamRef::from_args(self.w()?, args)?)),
            "schedule" => Ok(views::schedule(self.w()?, views::TeamRef::from_args(self.w()?, args)?)),
            "standings" => Ok(views::standings(self.w()?, args)),
            "staff" => Ok(views::staff(self.w()?, views::TeamRef::from_args(self.w()?, args)?)),
            "depth" => Ok(views::depth(self.w()?)),
            "set_depth" => views::set_depth(self.wm()?, args),
            "training" => Ok(views::training(self.w()?)),
            "recruiting" => Ok(views::recruiting(self.w()?)),
            "contracts" => Ok(views::contracts(self.w()?)),
            "set_focus" => views::set_focus(self.wm()?, args),
            "player" => views::player(self.w()?, id()?),
            "coach" => views::coach(self.w()?, num("id").ok_or("Missing id.")? as usize),
            "games" => Ok(views::my_games(self.w()?)),
            "game" => views::game(self.w()?, num("index").ok_or("Missing index.")? as usize),
            "search" => Ok(views::search(self.w()?, args.get("q").and_then(Value::as_str).unwrap_or(""))),
            "you" => Ok(views::you(self.w()?)),
            "note_add" => {
                let text = args.get("text").and_then(Value::as_str).ok_or("Missing text.")?.to_string();
                let pid = id()?;
                let w = self.wm()?;
                w.add_note(pid, text);
                views::player(w, pid)
            }
            "note_delete" => {
                let pid = id()?;
                let k = num("note").ok_or("Missing note.")? as usize;
                let w = self.wm()?;
                w.delete_note(pid, k);
                views::player(w, pid)
            }
            "bug" => setup::bug_pack(self, args),
            _ => Err(format!("Unknown command: {cmd}")),
        }
    }
}

/// Where the game keeps its files: `AFM_HOME`, else the Mac's Application
/// Support folder, else `~/.afm`.
pub fn default_home() -> PathBuf {
    if let Ok(h) = std::env::var("AFM_HOME") {
        return PathBuf::from(h);
    }
    let home = std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("."));
    if cfg!(target_os = "macos") { home.join("Library/Application Support/AFM") } else { home.join(".afm") }
}
