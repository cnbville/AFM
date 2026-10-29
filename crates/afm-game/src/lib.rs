//! The AFM game (section 8): your career on the engine's world, as one
//! command interface. The desktop app (Tauri) and the browser test server
//! both call `Game::call` with a command name and JSON arguments and get
//! JSON back, so every screen works the same in both.
//!
//! Nothing here shows a hidden number: attributes come as your staff's
//! ranges, potential as words, coaches as reputation (scouting topic 1,
//! Q46). The engine's `debug` feature is never on in this crate.

pub mod cards;
pub mod colors;
pub mod files;
pub mod jobs;
pub mod lane_boards;
pub mod lane_calendar;
pub mod lane_coaching;
pub mod lane_core;
pub mod lane_draftroom;
pub mod lane_evaluate;
pub mod lane_faces;
pub mod lane_film;
pub mod lane_find;
pub mod lane_frontoffice;
pub mod lane_gameday;
pub mod lane_gameplan;
pub mod lane_handbook;
pub mod lane_help;
pub mod lane_hiring;
pub mod lane_legacy;
pub mod lane_logos;
pub mod lane_money;
pub mod lane_paths;
pub mod lane_people;
pub mod lane_ratings;
pub mod lane_schedule;
pub mod lane_scouting;
pub mod lane_squadboards;
pub mod lane_stories;
pub mod lane_storylines;
pub mod lane_training;
pub mod lane_views;
pub mod memory;
pub mod reads;
pub mod setup;
pub mod views;
pub mod world_log;

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
        let r = self.run(cmd, args);
        // The guided week's tasks tick when their change is made (H12).
        if r.is_ok() {
            lane_handbook::after_call(self, cmd);
        }
        r
    }

    fn run(&mut self, cmd: &str, args: &Value) -> Reply {
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
                // Fictional names (lane 10): colleges, conferences and NFL nicknames; towns stay real.
                let fictional = args.get("names").and_then(Value::as_str) == Some("fictional");
                let d = WorldOptions::default();
                // How many history seasons play high school on the engine
                // (quick checks pass 0; live seasons always do).
                let hs_engine_years = num("hs_engine_years").map_or(d.hs_engine_years, |n| Some(n as u32));
                self.world = Some(World::new(WorldOptions {
                    seed,
                    history_years: years,
                    scale,
                    engine_from,
                    names: if fictional {
                        afm_engine::world::names::NameMode::Fictional
                    } else {
                        afm_engine::world::names::NameMode::Real
                    },
                    fictional_league: fictional,
                    hs_engine_years,
                    ..d
                }));
                self.save_path = None;
                if let Some(w) = self.world.as_mut() {
                    world_log::on_open(&self.home, w, None);
                }
                lane_faces::on_open(self);
                Ok(json!({ "built": true }))
            }
            // A full world built on this machine, into the library (hours).
            "build_world_file" => {
                let seed = num("seed").unwrap_or(2026) as u32;
                let years = num("history_years").unwrap_or(30).clamp(1, 60) as u32;
                let scale = args.get("scale").and_then(Value::as_f64).unwrap_or(1.0);
                let fictional = args.get("names").and_then(Value::as_str) == Some("fictional");
                let worlds = self.home.join("worlds");
                Ok(json!({ "name": jobs::build_named(&self.job, &worlds, seed, years, scale, fictional)? }))
            }
            "online_worlds" => jobs::online(&self.home.join("worlds")),
            "download_world" => {
                let tag = args.get("tag").and_then(Value::as_str).ok_or("Missing tag.")?;
                jobs::download(&self.job, &self.home.join("worlds"), tag)?;
                Ok(json!({ "started": true }))
            }
            "job" => Ok(jobs::status(&self.job)),
            "job_stop" => Ok(jobs::stop(&self.job)),
            "import_world" => {
                // Copies a world file into the library, where it stays for every new career.
                let path = PathBuf::from(args.get("path").and_then(Value::as_str).ok_or("Missing path.")?);
                let to = files::import_world(&path, &self.home.join("worlds"))?;
                Ok(serde_json::json!({ "path": to.to_string_lossy() }))
            }
            "open_world" | "load" => {
                let path = args.get("path").and_then(Value::as_str).ok_or("Missing path.")?;
                let w = setup::read(path)?;
                // The last world goes first, with its log's writer.
                self.world = None;
                self.world = Some(w);
                self.save_path = (cmd == "load").then(|| PathBuf::from(path));
                if let Some(w) = self.world.as_mut() {
                    world_log::on_open(&self.home, w, Some(std::path::Path::new(path)));
                }
                // "Previously" shows each time a save opens, even one saved
                // the day it was last shown.
                self.wm()?.storylines.previously_seen = None;
                lane_core::after_days(self.wm()?);
                memory::hand_back();
                lane_faces::on_open(self);
                Ok(views::status(self.w()?))
            }
            "save" => {
                let name = args.get("name").and_then(Value::as_str);
                let path = match name {
                    Some(n) => self.home.join("saves").join(format!("{}.afmsave", setup::file_safe(n))),
                    None => self.save_path.clone().ok_or("Name the save first.")?,
                };
                setup::write(self.w()?, &path)?;
                world_log::on_save(self.wm()?, &path);
                lane_faces::on_save(&self.home, self.save_path.as_deref(), &path);
                self.save_path = Some(path.clone());
                Ok(json!({ "saved": path.display().to_string() }))
            }
            "rename_save" => setup::rename_save(self, args),
            "delete_save" => setup::delete_save(self, args),
            "reveal" => setup::reveal(self, args),
            "setup" => Ok(setup::options(self.w()?)),
            "similar" => Ok(setup::similar(self.w()?, args)),
            "towns" => Ok(setup::towns(self.w()?, args.get("state").and_then(Value::as_str).unwrap_or("TX"))),
            "jobs" => Ok(setup::jobs(self.w()?, args.get("level").and_then(Value::as_str).unwrap_or("college"))),
            "start_career" => {
                let w = self.wm()?;
                setup::start(w, args)?;
                lane_handbook::on_start(w, args);
                lane_core::after_days(w);
                Ok(views::status(w))
            }
            "status" => Ok(views::status(self.w()?)),
            "card" => Ok(cards::card(self.w()?)),
            "continue" => {
                let max = num("max_days").unwrap_or(400) as u32;
                let w = self.wm()?;
                let from = w.today;
                let stopped = w.continue_until_needed(max);
                lane_core::after_days(w);
                memory::hand_back();
                Ok(views::stopped(w, from, stopped))
            }
            "day" => {
                let w = self.wm()?;
                let from = w.today;
                w.advance_day();
                lane_core::after_days(w);
                Ok(views::stopped(w, from, from))
            }
            "holiday" => {
                let days = num("days").unwrap_or(7) as i32;
                let w = self.wm()?;
                let from = w.today;
                let stopped = w.holiday_to(from.add(days));
                lane_core::after_days(w);
                Ok(views::stopped(w, from, stopped))
            }
            "home" => Ok(views::home(self.w()?)),
            "inbox" => Ok(views::inbox(self.w()?, num("days").unwrap_or(60) as i32)),
            "news" => Ok(views::news(self.w()?, args)),
            "calendar" => Ok(views::calendar(self.w()?)),
            "team" => Ok(views::team(self.w()?, views::TeamRef::from_args(self.w()?, args)?)),
            "roster" => Ok(views::roster(self.w()?, views::TeamRef::from_args(self.w()?, args)?)),
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
            _ => {
                // The final build's lanes each answer their own commands.
                type Lane = fn(&mut Game, &str, &Value) -> Option<Reply>;
                const LANES: [Lane; 28] = [
                    lane_core::call,
                    lane_gameday::call,
                    lane_money::call,
                    lane_people::call,
                    lane_scouting::call,
                    lane_stories::call,
                    lane_storylines::call,
                    lane_legacy::call,
                    lane_frontoffice::call,
                    lane_training::call,
                    lane_faces::call,
                    lane_views::call,
                    lane_ratings::call,
                    lane_boards::call,
                    lane_find::call,
                    lane_gameplan::call,
                    lane_schedule::call,
                    lane_squadboards::call,
                    lane_draftroom::call,
                    lane_evaluate::call,
                    lane_calendar::call,
                    lane_coaching::call,
                    lane_hiring::call,
                    lane_logos::call,
                    lane_help::call,
                    lane_paths::call,
                    lane_film::call,
                    lane_handbook::call,
                ];
                for lane in LANES {
                    if let Some(r) = lane(self, cmd, args) {
                        return r;
                    }
                }
                Err(format!("Unknown command: {cmd}"))
            }
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
