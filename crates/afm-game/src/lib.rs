//! The AFM game (section 8): your career on the engine's world, as one
//! command interface. The desktop app (Tauri) and the browser test server
//! both call `Game::call` with a command name and JSON arguments and get
//! JSON back, so every screen works the same in both.
//!
//! Nothing here shows a hidden number: attributes come as your staff's
//! ranges, potential as words, coaches as reputation (scouting topic 1,
//! Q46). The engine's `debug` feature is never on in this crate.

pub mod autosave;
pub mod cards;
pub mod colors;
pub mod depth_plan;
pub mod files;
pub mod jobs;
pub mod joining;
pub mod lane_boards;
pub mod lane_calendar;
pub mod lane_campaign;
pub mod lane_capital;
pub mod lane_capital_campus;
pub mod lane_capital_drive;
pub mod lane_capital_pecking;
pub mod lane_capital_project;
pub mod lane_cfl;
pub mod lane_coaching;
pub mod lane_core;
pub mod lane_data;
pub mod lane_draftroom;
pub mod lane_evaluate;
pub mod lane_faces;
pub mod lane_favors;
pub mod lane_film;
pub mod lane_find;
pub mod lane_frontoffice;
pub mod lane_gameday;
pub mod lane_gameplan;
pub mod lane_ground;
pub mod lane_ground_people;
pub mod lane_groundstaff;
pub mod lane_groundweek;
pub mod lane_groundzero;
pub mod lane_handbook;
pub mod lane_help;
pub mod lane_hiring;
pub mod lane_hunt;
pub mod lane_leads;
pub mod lane_legacy;
pub mod lane_logos;
pub mod lane_money;
pub mod lane_paths;
pub mod lane_people;
pub mod lane_race;
pub mod lane_ratings;
pub mod lane_schedule;
pub mod lane_scouting;
pub mod lane_squadboards;
pub mod lane_stories;
pub mod lane_storylines;
pub mod lane_training;
pub mod lane_views;
pub mod live_talk;
pub mod memory;
pub mod notebook;
pub mod quick_filter;
pub mod reads;
pub mod road;
pub mod setup;
pub mod tallies;
pub mod versions;
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
    /// Live talks: the API key (in memory only), the client, this month's
    /// spend and the calls in flight (`live_talk`).
    pub ai: live_talk::Ai,
    /// The autosave every 10 minutes (`autosave.rs`).
    pub autosave: autosave::Autosave,
}

pub type Reply = Result<Value, String>;

impl Game {
    pub fn new(home: PathBuf) -> Game {
        for d in ["worlds", "saves", "bugs"] {
            let _ = std::fs::create_dir_all(home.join(d));
        }
        let ai = live_talk::Ai::new(&home);
        Game { world: None, home, save_path: None, job: Default::default(), ai, autosave: Default::default() }
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
        self.autosave.touch(cmd);
        // Days never play while an autosave still shares the world's pages
        // (autosave.rs, module notes). Continue waits in its own arm, once
        // no meeting stops it.
        if autosave::plays_days(cmd) && cmd != "continue" {
            self.autosave.finish_first();
        }
        let mut r = self.run(cmd, args);
        // The lines a screen showed you are heard: the same moment keeps
        // its words all day, and the next game day reaches for new ones
        // (minor bug 314).
        if let Some(w) = self.world.as_mut() {
            afm_engine::world::folk::voice::remember_shown(w);
        }
        // The guided week's tasks tick when their change is made (H12).
        if r.is_ok() {
            lane_handbook::after_call(self, cmd);
            // Live talks: a box on a reply (the core fills this in).
            live_talk::after(self, cmd, args, &mut r);
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
                lane_core::links::forget_names();
                let _ = jobs::take_handoff(std::path::Path::new(""));
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
                self.autosave.reset();
                if let Some(w) = self.world.as_mut() {
                    world_log::on_open(&self.home, w, None);
                    tallies::on_open(&self.home, w, None);
                }
                lane_faces::on_open(self);
                notebook::on_open(&self.home);
                depth_plan::on_open(&self.home);
                Ok(json!({ "built": true }))
            }
            // A full world built on this machine, into the library (hours).
            "build_world_file" => {
                let seed = num("seed").unwrap_or(2026) as u32;
                let years = num("history_years").unwrap_or(30).clamp(1, 60) as u32;
                let scale = args.get("scale").and_then(Value::as_f64).unwrap_or(1.0);
                let fictional = args.get("names").and_then(Value::as_str) == Some("fictional");
                // Where the world stops (the Jan 1 start): Jan 1 unless asked for the morning after the Super Bowl.
                let jan1 = args.get("stop").and_then(Value::as_str) != Some("feb9");
                let worlds = self.home.join("worlds");
                // A played-on world still kept in memory goes first.
                let _ = jobs::take_handoff(std::path::Path::new(""));
                Ok(json!({ "name": jobs::build_named(&self.job, &worlds, seed, years, scale, fictional, jan1)? }))
            }
            // The Jan 1 start: a library world plays on to Jan 1 once and
            // stays in the library as its own world (minutes, beside the game).
            "play_on" => {
                let path = PathBuf::from(args.get("path").and_then(Value::as_str).ok_or("Missing path.")?);
                // A played-on world still kept in memory goes first.
                let _ = jobs::take_handoff(std::path::Path::new(""));
                let name = jobs::play_on(&self.job, &self.home.join("worlds"), &path)?;
                // Only one full world in memory: the open one goes once the
                // play-on has started (the screens closed its career first).
                self.autosave.reset();
                self.world = None;
                self.save_path = None;
                Ok(json!({ "name": name }))
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
                // A world the play-on just wrote is still in memory.
                let handed = jobs::take_handoff(std::path::Path::new(path));
                // The last world goes first, with its log's writer (and any
                // autosave still writing it), before the new one is read: a
                // 30-year world holds about 6 GB, and reading a save while
                // the career or world you had open was still in memory held
                // two, 12 GB, which an 8 GB Mac can't give (the save "won't
                // load"). The screens ask you to close the open career first
                // (`leaveCareer`), and a file that can't be read leaves no
                // world open, which they show as the start page.
                self.autosave.reset();
                self.world = None;
                self.save_path = None;
                lane_core::links::forget_names();
                memory::hand_back();
                let w = match handed {
                    Some(w) => w,
                    None => setup::read(path)?,
                };
                self.world = Some(w);
                self.save_path = (cmd == "load").then(|| PathBuf::from(path));
                if let Some(w) = self.world.as_mut() {
                    world_log::on_open(&self.home, w, Some(std::path::Path::new(path)));
                    tallies::on_open(&self.home, w, Some(std::path::Path::new(path)));
                }
                // "Previously" shows each time a save opens, even one saved
                // the day it was last shown.
                self.wm()?.storylines.previously_seen = None;
                lane_core::after_days(self.wm()?);
                memory::hand_back();
                lane_faces::on_open(self);
                notebook::on_open(&self.home);
                depth_plan::on_open(&self.home);
                Ok(views::status(self.w()?))
            }
            "save" => {
                let name = args.get("name").and_then(Value::as_str);
                let path = match name {
                    Some(n) => self.home.join("saves").join(format!("{}.afmsave", setup::file_safe(n))),
                    None => self.save_path.clone().ok_or("Name the save first.")?,
                };
                // A save by hand ends an autosave still writing: never two at once.
                self.autosave.cancel();
                setup::write(self.w()?, &path)?;
                self.autosave.saved_by_hand();
                world_log::on_save(self.wm()?, &path);
                tallies::on_save(self.wm()?, &path);
                lane_faces::on_save(&self.home, self.save_path.as_deref(), &path);
                notebook::on_save(&self.home, self.save_path.as_deref(), &path);
                depth_plan::on_save(&self.home, self.save_path.as_deref(), &path);
                self.save_path = Some(path.clone());
                Ok(json!({ "saved": path.display().to_string() }))
            }
            "rename_save" => setup::rename_save(self, args),
            "delete_save" => setup::delete_save(self, args),
            "delete_world" => setup::delete_world(self, args),
            "reveal" => setup::reveal(self, args),
            // Going back to an older version (Settings): what it can't open, a
            // copy of your saves, and the pause on updates.
            "versions_check" => versions::check(self, args),
            "versions_backup" => versions::backup(self, args),
            "update_pin_get" => Ok(versions::pin_get(self)),
            "update_pin_set" => versions::pin_set(self, args),
            "update_pin_clear" => versions::pin_clear(self),
            "setup" => Ok(setup::options(self.w()?)),
            "similar" => Ok(setup::similar(self.w()?, args)),
            "towns" => Ok(setup::towns(self.w()?, args.get("state").and_then(Value::as_str).unwrap_or("TX"))),
            "jobs" => Ok(setup::jobs(self.w()?, args.get("level").and_then(Value::as_str).unwrap_or("college"))),
            "start_career" => {
                let w = self.wm()?;
                setup::start(w, args)?;
                lane_frontoffice::college::start_clean(w, args);
                lane_handbook::on_start(w, args);
                lane_core::after_days(w);
                let r = Ok(views::status(w));
                self.autosave.reset();
                r
            }
            "status" => Ok(views::status(self.w()?)),
            "card" => Ok(cards::card(self.w()?)),
            "continue" => {
                let w = self.wm()?;
                // A meeting that waits for you stops the clock until you hold
                // it or hand it to your staff. `staff: true` (the tools and
                // tests) hands every one on, as the day's end always did.
                let waiting: Vec<u32> = afm_engine::world::folk::scene::waiting(w).iter().map(|s| s.id).collect();
                if !waiting.is_empty() {
                    if args.get("staff").and_then(Value::as_bool) == Some(true) {
                        for id in waiting {
                            afm_engine::world::folk::scene::hand(w, id)?;
                        }
                    } else {
                        let first = afm_engine::world::folk::scene::waiting(w)[0];
                        let text = lane_people::plain(&first.title);
                        let more = waiting.len() - 1;
                        return Ok(json!({
                            "status": views::status(w),
                            "blocked": { "meeting": first.id, "text": text, "more": more },
                        }));
                    }
                }
                // Only once the days will play: a meeting's answer above
                // never waits for the autosave.
                self.autosave.finish_first();
                let w = self.wm()?;
                let (from, filed) = (w.now(), views::desk_ids(w));
                w.continue_until_needed();
                lane_core::after_days(w);
                memory::hand_back();
                Ok(views::stopped(w, from, &filed))
            }
            "day" => {
                // A whole day (the debug tools): the rest of today, then to
                // your next stop.
                let w = self.wm()?;
                let (from, filed) = (w.now(), views::desk_ids(w));
                w.next_day();
                lane_core::after_days(w);
                Ok(views::stopped(w, from, &filed))
            }
            "holiday" => {
                let days = num("days").unwrap_or(7) as i32;
                let w = self.wm()?;
                let (from, filed) = (w.now(), views::desk_ids(w));
                let today = w.today;
                // A meeting you picked to stop for brings you back the
                // morning it is held (the holiday preview's stops).
                let stops: Vec<u32> = args
                    .get("stops")
                    .and_then(Value::as_array)
                    .map(|a| a.iter().filter_map(Value::as_u64).map(|x| x as u32).collect())
                    .unwrap_or_default();
                let until = w
                    .folk
                    .scenes
                    .iter()
                    .filter(|s| !s.done && stops.contains(&s.id) && s.date > today)
                    .map(|s| s.date)
                    .min()
                    .map_or(today.add(days), |d| d.min(today.add(days)));
                // The inbox is filed at each new day on the way: an entrant
                // who decides inside the stretch is gone from the world by
                // its end, and his day's message is written from the day.
                w.holiday_to_with(until, &mut |w| lane_core::mail::after_days(w));
                lane_core::after_days(w);
                Ok(views::stopped(w, from, &filed))
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
            "bug" => {
                self.autosave.cancel();
                setup::bug_pack(self, args)
            }
            "autosave_tick" => autosave::tick(self),
            "autosave_status" => Ok(self.autosave.status()),
            _ => {
                // The final build's lanes each answer their own commands.
                type Lane = fn(&mut Game, &str, &Value) -> Option<Reply>;
                const LANES: &[Lane] = &[
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
                    lane_cfl::call,
                    lane_evaluate::call,
                    lane_calendar::call,
                    lane_coaching::call,
                    lane_hiring::call,
                    lane_logos::call,
                    lane_help::call,
                    lane_paths::call,
                    lane_film::call,
                    lane_handbook::call,
                    lane_groundzero::call,
                    lane_ground_people::call,
                    lane_favors::call,
                    lane_ground::call,
                    lane_groundweek::call,
                    lane_groundstaff::call,
                    lane_race::call,
                    lane_hunt::call,
                    lane_leads::call,
                    lane_campaign::call,
                    lane_capital::call,
                    lane_capital_drive::call,
                    lane_capital_project::call,
                    lane_capital_pecking::call,
                    lane_capital_campus::call,
                    lane_data::call,
                    notebook::call,
                    live_talk::call,
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

/// The screens share one page: a top-level name declared in two scripts
/// lets the later one silently replace the earlier (bug 251, and the film
/// stills' drawer that broke Find's board button, 2026-11-04).
#[cfg(test)]
mod screen_names {
    #[test]
    fn no_two_screens_declare_the_same_name() {
        let ui = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../app/ui");
        let index = std::fs::read_to_string(ui.join("index.html")).unwrap();
        let mut seen: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        let mut clashes = Vec::new();
        for src in index.split("<script src=\"").skip(1).filter_map(|s| s.split('"').next()) {
            let Ok(text) = std::fs::read_to_string(ui.join(src)) else { continue };
            for line in text.lines() {
                let rest = ["async function ", "function ", "const ", "let ", "var ", "class "]
                    .iter()
                    .find_map(|k| line.strip_prefix(k));
                let Some(rest) = rest else { continue };
                let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '$').collect();
                if name.is_empty() {
                    continue;
                }
                if let Some(first) = seen.insert(name.clone(), src.to_string())
                    && first != src
                {
                    clashes.push(format!("{name}: {first} and {src}"));
                }
            }
        }
        assert!(clashes.is_empty(), "names declared in two screens: {clashes:?}");
    }
}

/// Opening a world or a save closes the one you had open before it reads
/// the file, so two 30-year worlds (about 6 GB each) are never in memory at
/// once (Cain's seed 181 saves that "won't load" on an 8 GB Mac,
/// 2026-11-17).
#[cfg(test)]
mod open_order {
    use super::Game;
    use afm_engine::world::{World, WorldOptions};
    use serde_json::json;

    #[test]
    fn the_open_world_goes_before_the_next_is_read() {
        let home = std::env::temp_dir().join(format!("afm-open-order-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        let mut g = Game::new(home.clone());
        let w = World::empty(WorldOptions { scale: 0.001, history_years: 0, start_year: 2037, ..Default::default() });
        let good = home.join("saves").join("small.afmsave");
        crate::setup::write(&w, &good).unwrap();
        let bad = home.join("saves").join("broken.afmsave");
        std::fs::write(&bad, b"AFMWORLD not a world").unwrap();

        g.world = Some(w);
        // A file that can't be read: the open world went first, so none is left.
        assert!(g.call("load", &json!({ "path": bad.display().to_string() })).is_err());
        assert!(g.world.is_none(), "the open world is dropped before the next file is read");
        assert_eq!(g.call("hello", &json!({})).unwrap()["loaded"], false);
        // A good file opens as before.
        g.call("load", &json!({ "path": good.display().to_string() })).unwrap();
        assert_eq!(g.call("hello", &json!({})).unwrap()["loaded"], true);
        let _ = std::fs::remove_dir_all(&home);
    }
}
