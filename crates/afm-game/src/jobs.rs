//! Long jobs beside the game: building a world on this machine, and
//! downloading one from the worlds page (Cain 2026-10-21: both). Each job runs
//! on its own thread, never holding the game, and writes its progress where
//! the screens read it. One job at a time.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use afm_engine::world::names::NameMode;
use afm_engine::world::{SPIN_UP_YEARS, World, WorldOptions};
use serde::Serialize;
use serde_json::{Value, json};

use crate::files;

/// The public repository whose releases are the downloadable worlds. It holds
/// world files only, no game code; `.github/workflows/build-world.yml` builds
/// and publishes them.
pub const WORLDS_REPO: &str = "cnbville/afm-worlds";

/// What a job has done so far.
#[derive(Clone, Default, Serialize)]
pub struct Job {
    /// "build" or "download".
    pub kind: String,
    /// The world's name in the library.
    pub name: String,
    /// One plain line for the screen.
    pub text: String,
    /// Share done, 0 to 1.
    pub done: f64,
    pub seconds: f64,
    pub finished: bool,
    pub error: Option<String>,
    /// The world file, once it is in the library.
    pub path: Option<String>,
    /// Stop was pressed: the job ends at its next step (a build after the
    /// season it is on, a download within a second).
    pub stopping: bool,
}

const STOPPED: &str = "Stopped. Nothing was added to your library.";

fn stopping(shared: &Shared) -> bool {
    shared.lock().ok().and_then(|g| g.as_ref().map(|j| j.stopping)).unwrap_or(false)
}

/// Asks the running job to stop.
pub fn stop(shared: &Shared) -> Value {
    set(shared, |j| {
        if !j.finished {
            j.stopping = true;
            j.text =
                if j.kind == "build" { "Stopping after the season it is on…".into() } else { "Stopping…".into() };
        }
    });
    status(shared)
}

pub type Shared = Arc<Mutex<Option<Job>>>;

fn set(shared: &Shared, f: impl FnOnce(&mut Job)) {
    if let Ok(mut g) = shared.lock()
        && let Some(j) = g.as_mut()
    {
        f(j);
    }
}

fn start(shared: &Shared, kind: &str, name: &str, text: &str) -> Result<(), String> {
    let mut g = shared.lock().map_err(|_| "The job list is broken.")?;
    if g.as_ref().is_some_and(|j| !j.finished) {
        return Err("Another world is still being built or downloaded.".into());
    }
    *g = Some(Job { kind: kind.into(), name: name.into(), text: text.into(), ..Default::default() });
    Ok(())
}

fn finish(shared: &Shared, t: Instant, r: Result<PathBuf, String>) {
    set(shared, |j| {
        j.seconds = t.elapsed().as_secs_f64();
        j.finished = true;
        match r {
            Ok(p) => {
                j.done = 1.0;
                j.text = "In your library.".into();
                j.path = Some(p.to_string_lossy().into_owned());
            }
            Err(e) => j.error = Some(e),
        }
    });
}

/// The job now, for the screens.
pub fn status(shared: &Shared) -> Value {
    shared.lock().ok().and_then(|g| g.clone()).map_or(Value::Null, |j| json!(j))
}

/// A built world's name in the library.
pub fn world_name(seed: u32, years: u32) -> String {
    let start = WorldOptions::default().start_year;
    if years == 30 { format!("world-{start}-seed{seed}") } else { format!("world-{start}-seed{seed}-{years}y") }
}

/// Builds a full world and writes it into `worlds`, calling `each` after
/// every season (the 12 spin-up seasons first, then the history) with the
/// seasons done and the season's year, then once
/// with 0 when it starts writing the file. The game's
/// "Build a world" button and the worlds page's builder both use it.
pub fn build_world_file(
    worlds: &Path,
    seed: u32,
    years: u32,
    scale: f64,
    each: impl FnMut(u32, i32) -> bool,
) -> Result<PathBuf, String> {
    build_world_file_named(worlds, seed, years, scale, false, each)
}

/// The same, with every name fictional when `fictional` (lane 10): the
/// colleges, the high schools, the NFL's nicknames and the conferences.
pub fn build_world_file_named(
    worlds: &Path,
    seed: u32,
    years: u32,
    scale: f64,
    fictional: bool,
    mut each: impl FnMut(u32, i32) -> bool,
) -> Result<PathBuf, String> {
    let name = world_name(seed, years) + if fictional { "-fictional" } else { "" };
    let to = worlds.join(format!("{name}.afmworld"));
    let mut seasons = 0;
    let names = if fictional { NameMode::Fictional } else { NameMode::Real };
    let opts =
        WorldOptions { seed, history_years: years, scale, names, fictional_league: fictional, ..Default::default() };
    let (log_part, data_part, mut w) = begin(worlds, &name, opts)?;
    let done = w.play_history(|w, _| {
        seasons += 1;
        each(seasons, w.year - 1)
    });
    if !done {
        drop(w);
        let _ = std::fs::remove_dir_all(&log_part);
        let _ = std::fs::remove_dir_all(&data_part);
        return Err(STOPPED.into());
    }
    // Seasons done: 0 says the world is being written.
    each(0, 0);
    write_built(w, &to, &log_part, &data_part)
}

/// The world before its history plays, with its log (world reader lane
/// R2) and its tallies (lane NB1) written beside it as the history plays:
/// the log's and the tallies' part folders, and the world.
fn begin(worlds: &Path, name: &str, opts: WorldOptions) -> Result<(PathBuf, PathBuf, World), String> {
    let log_part = worlds.join(format!("{name}.{}.part", afm_engine::world::wlog::EXT));
    let log = if crate::world_log::on() {
        let log = afm_engine::world::wlog::WorldLog::create(&log_part, &opts, crate::world_log::commit(), "");
        Some(log.map_err(|e| format!("Could not start the world log: {e}"))?)
    } else {
        None
    };
    let data_part = worlds.join(format!("{name}.{}.part", afm_engine::world::tally::EXT));
    if data_part.exists() {
        let _ = std::fs::remove_dir_all(&data_part);
    }
    let w = World::begin_recorded(opts, log, Some(&data_part));
    Ok((log_part, data_part, w))
}

/// What [`build_world_in_parts`] and [`resume_world_file`] left in the
/// folder.
pub enum Part {
    /// The finished world file, its tallies beside it.
    Done(PathBuf),
    /// A world stopped between two history seasons (`name.afmworld.half`,
    /// its tallies so far in `name.afmdata.part`), for
    /// [`resume_world_file`] to carry on, on this machine or another.
    Half(PathBuf),
}

/// The ending of a half-built world's file.
pub const HALF: &str = "afmworld.half";

/// `build_world_file` for a build too long for one machine's turn (NB13:
/// a GitHub job may run six hours): after each season, `pause` says
/// whether to stop there and write the world so far as a half file (it is
/// asked after every season, the last too, so it can print progress). A
/// world with its log never pauses (the log is written as the history
/// plays and cannot move machines).
pub fn build_world_in_parts(
    worlds: &Path,
    seed: u32,
    years: u32,
    scale: f64,
    mut pause: impl FnMut(&World) -> bool,
) -> Result<Part, String> {
    let name = world_name(seed, years);
    let to = worlds.join(format!("{name}.afmworld"));
    let opts = WorldOptions { seed, history_years: years, scale, ..Default::default() };
    let logged = crate::world_log::on();
    let (log_part, data_part, mut w) = begin(worlds, &name, opts)?;
    if !w.play_history(|w, _| !(pause(w) && !logged && w.history_left() > 0)) {
        return pause_half(&w, &worlds.join(format!("{name}.{HALF}"))).map(Part::Half);
    }
    write_built(w, &to, &log_part, &data_part).map(Part::Done)
}

/// Carries on a half-built world (`name.afmworld.half`, its tallies so far
/// in `name.afmdata.part` beside it), pausing again where `pause` says, and
/// writes the world file beside it once its history is done. The seasons
/// it plays come out as in a build that never paused (`tests/split_build.rs`).
pub fn resume_world_file(half: &Path, mut pause: impl FnMut(&World) -> bool) -> Result<Part, String> {
    let file = half.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    let name = file.strip_suffix(&format!(".{HALF}")).ok_or("Not a half-built world (name.afmworld.half).")?;
    let dir = half.parent().filter(|d| !d.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let to = dir.join(format!("{name}.afmworld"));
    let log_part = dir.join(format!("{name}.{}.part", afm_engine::world::wlog::EXT));
    let data_part = dir.join(format!("{name}.{}.part", afm_engine::world::tally::EXT));
    let mut w = afm_engine::save::read_world(half).map_err(|e| format!("Could not read {}: {e}", half.display()))?;
    w.attach_tallies(&data_part).map_err(|e| format!("Could not open the tallies: {e}"))?;
    if !w.play_history(|w, _| !(pause(w) && w.history_left() > 0)) {
        return pause_half(&w, half).map(Part::Half);
    }
    std::fs::remove_file(half).map_err(|e| format!("Could not remove {}: {e}", half.display()))?;
    write_built(w, &to, &log_part, &data_part).map(Part::Done)
}

/// Writes a world stopped part way through its history to `half`, through
/// a part file, so a write cut off leaves no half file.
fn pause_half(w: &World, half: &Path) -> Result<PathBuf, String> {
    let part = half.with_extension("half.part");
    afm_engine::save::write_world(w, &part, crate::world_log::commit(), &files::now_utc(), None)
        .and_then(|_| std::fs::rename(&part, half))
        .map_err(|e| format!("Could not write the half-built world: {e}"))?;
    Ok(half.to_path_buf())
}

/// The world's history is done: its log and tallies go beside `to`, then
/// the world file itself.
fn write_built(mut w: World, to: &Path, log_part: &Path, data_part: &Path) -> Result<PathBuf, String> {
    if let Some(log) = w.detach_log() {
        let to_log = crate::world_log::beside(to);
        log.close()
            .and_then(|_| if to_log.exists() { std::fs::remove_dir_all(&to_log) } else { Ok(()) })
            .and_then(|_| std::fs::rename(log_part, &to_log))
            .map_err(|e| format!("Could not write the world log: {e}"))?;
    }
    w.detach_tallies();
    let to_data = crate::tallies::beside(to);
    if to_data.exists() {
        std::fs::remove_dir_all(&to_data).map_err(|e| format!("Could not replace the tallies: {e}"))?;
    }
    std::fs::rename(data_part, &to_data).map_err(|e| format!("Could not write the tallies: {e}"))?;
    let part = to.with_extension("afmworld.part");
    let commit = crate::world_log::commit();
    afm_engine::save::write_world(&w, &part, commit, &files::now_utc(), Some(crate::cards::card(&w)))
        .and_then(|_| std::fs::rename(&part, to))
        .map_err(|e| format!("Could not write the world: {e}"))?;
    Ok(to.to_path_buf())
}

/// Builds a full world on this machine, beside the game, into the library.
/// A 30-year world takes hours and about 6.5 GB of memory at its peak.
pub fn build(shared: &Shared, worlds: &Path, seed: u32, years: u32, scale: f64) -> Result<String, String> {
    build_named(shared, worlds, seed, years, scale, false)
}

/// `build`, with every name fictional when `fictional`.
pub fn build_named(
    shared: &Shared,
    worlds: &Path,
    seed: u32,
    years: u32,
    scale: f64,
    fictional: bool,
) -> Result<String, String> {
    let name = world_name(seed, years) + if fictional { "-fictional" } else { "" };
    start(shared, "build", &name, "Starting the history…")?;
    let (shared, worlds) = (shared.clone(), worlds.to_path_buf());
    std::thread::spawn(move || {
        let t = Instant::now();
        // The last 12 seasons play every high school game on the engine and
        // take about six times as long as the others.
        let all = SPIN_UP_YEARS + years;
        let slow = years.min(12);
        let weight = |n: u32| n.min(all - slow) as f64 + 6.0 * n.saturating_sub(all - slow) as f64;
        let r = build_world_file_named(&worlds, seed, years, scale, fictional, |n, year| {
            set(&shared, |j| {
                j.seconds = t.elapsed().as_secs_f64();
                if n == 0 {
                    j.done = 0.98;
                    j.text = "Writing the world file…".into();
                } else if !j.stopping {
                    j.done = 0.95 * weight(n) / weight(all);
                    j.text = format!("Season {year} done ({n} of {all})");
                }
            });
            !stopping(&shared)
        });
        finish(&shared, t, r);
    });
    Ok(name)
}

fn get(url: &str) -> Result<ureq::Response, String> {
    ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(10))
        .timeout_read(std::time::Duration::from_secs(60))
        .build()
        .get(url)
        .set("User-Agent", "AFM")
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(404, _) => "The worlds page isn't set up yet.".to_string(),
            ureq::Error::Status(code, _) => format!("The worlds page answered {code}."),
            ureq::Error::Transport(t) => format!("Could not reach the worlds page ({t})."),
        })
}

/// The world files of one release, in order (a whole file, or its parts).
fn world_assets(release: &Value) -> Vec<(String, u64, String)> {
    let mut a: Vec<(String, u64, String)> = release["assets"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|x| {
            let name = x["name"].as_str()?;
            let part = name.rsplit_once(".afmworld").is_some_and(|(_, rest)| {
                rest.is_empty() || (rest.len() == 4 && rest[1..].bytes().all(|b| b.is_ascii_digit()))
            });
            let url = x["browser_download_url"].as_str()?;
            part.then(|| (name.to_string(), x["size"].as_u64().unwrap_or(0), url.to_string()))
        })
        .collect();
    a.sort();
    a
}

/// A release's tallies (lane NB1): `name.afmdata.tar`, or its parts in order.
fn data_assets(release: &Value) -> Vec<(String, u64, String)> {
    let mut a: Vec<(String, u64, String)> = release["assets"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|x| {
            let name = x["name"].as_str()?;
            let part = name.rsplit_once(".afmdata.tar").is_some_and(|(_, rest)| {
                rest.is_empty() || (rest.len() == 4 && rest[1..].bytes().all(|b| b.is_ascii_digit()))
            });
            let url = x["browser_download_url"].as_str()?;
            part.then(|| (name.to_string(), x["size"].as_u64().unwrap_or(0), url.to_string()))
        })
        .collect();
    a.sort();
    a
}

/// The worlds on the worlds page, newest first, each marked when it is
/// already in the library. A world is published again under the same name
/// when a newer game builds it, so only its newest build is listed, and it is
/// marked `newer` when the library holds an older build it would replace.
pub fn online(worlds: &Path) -> Result<Value, String> {
    let list: Value = get(&format!("https://api.github.com/repos/{WORLDS_REPO}/releases?per_page=30"))?
        .into_json()
        .map_err(|e| e.to_string())?;
    let mut seen = std::collections::HashSet::new();
    let rows: Vec<Value> = newest_first(&list)
        .into_iter()
        .filter_map(|r| {
            let assets = world_assets(r);
            let first = assets.first()?.0.clone();
            let name = first.strip_suffix(".001").unwrap_or(&first);
            let name = name.strip_suffix(".afmworld").unwrap_or(name).to_string();
            if !seen.insert(name.clone()) {
                return None;
            }
            let mb = assets.iter().chain(&data_assets(r)).map(|a| a.1).sum::<u64>() as f64 / 1e6;
            // A library copy this game can't read the header of is replaced too.
            let local = worlds.join(format!("{name}.afmworld"));
            let (tag, date) = (r["tag_name"].as_str().unwrap_or(""), r["published_at"].as_str().unwrap_or(""));
            let have = local.exists();
            let newer = have && files::read_meta(&local).map_or(true, |m| newer_build(&m, tag, date));
            Some(json!({
                "tag": r["tag_name"], "title": r["name"], "notes": r["body"], "date": r["published_at"],
                "name": name, "mb": mb.round(), "have": have && !newer, "newer": newer,
            }))
        })
        .collect();
    Ok(json!({ "repo": WORLDS_REPO, "worlds": rows }))
}

/// The worlds page's releases, newest published first. GitHub lists them by
/// the date of the commit each tag points at, and every world's tag points at
/// the worlds page's one commit, so its own order is no order at all
/// (2026-11-02: seed 11 came 7th of 9, and the 21 October seed 2026 build
/// was kept as the newest).
pub fn newest_first(list: &Value) -> Vec<&Value> {
    let mut v: Vec<&Value> = list.as_array().into_iter().flatten().collect();
    v.sort_by(|a, b| b["published_at"].as_str().unwrap_or("").cmp(a["published_at"].as_str().unwrap_or("")));
    v
}

/// Whether a release (tag `name-<commit>`, published at `date`) is a newer
/// build of the library's world with header `local`: another game's commit,
/// published after the library's copy was written.
fn newer_build(local: &Value, tag: &str, date: &str) -> bool {
    let build = tag.rsplit_once('-').map_or("", |(_, b)| b);
    let commit = local["commit"].as_str().unwrap_or("");
    if !build.is_empty() && commit.starts_with(build) {
        return false;
    }
    // "2026-10-23 20:55 UTC" against "2026-10-23T20:57:13Z", to the minute.
    let written = local["written"].as_str().unwrap_or("").replacen(' ', "T", 1);
    date.get(..16).unwrap_or(date) > written.get(..16).unwrap_or(&written)
}

/// Downloads one world from the worlds page into the library.
pub fn download(shared: &Shared, worlds: &Path, tag: &str) -> Result<(), String> {
    let release: Value = get(&format!("https://api.github.com/repos/{WORLDS_REPO}/releases/tags/{tag}"))?
        .into_json()
        .map_err(|e| e.to_string())?;
    let mut assets = world_assets(&release);
    let first = assets.first().ok_or("That release has no world file.")?.0.clone();
    // Its tallies come along and are unpacked beside it (lane NB1).
    assets.extend(data_assets(&release));
    start(shared, "download", &first, "Starting the download…")?;
    let total: u64 = assets.iter().map(|a| a.1).sum::<u64>().max(1);
    let (shared, worlds) = (shared.clone(), worlds.to_path_buf());
    std::thread::spawn(move || {
        let t = Instant::now();
        let dir = worlds.join(".download");
        let r = (|| -> Result<PathBuf, String> {
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let mut got = 0u64;
            let mut buf = vec![0u8; 1 << 20];
            for (name, _, url) in &assets {
                let mut from = get(url)?.into_reader();
                let mut out = std::fs::File::create(dir.join(name)).map_err(|e| e.to_string())?;
                loop {
                    if stopping(&shared) {
                        return Err(STOPPED.into());
                    }
                    let n = from.read(&mut buf).map_err(|e| format!("The download broke off: {e}"))?;
                    if n == 0 {
                        break;
                    }
                    out.write_all(&buf[..n]).map_err(|e| e.to_string())?;
                    got += n as u64;
                    let secs = t.elapsed().as_secs_f64();
                    set(&shared, |j| {
                        j.done = 0.97 * got as f64 / total as f64;
                        j.seconds = secs;
                        j.text = format!(
                            "{:.0} of {:.0} MB, {:.0} MB a second",
                            got as f64 / 1e6,
                            total as f64 / 1e6,
                            got as f64 / 1e6 / secs.max(0.1)
                        );
                    });
                }
            }
            set(&shared, |j| j.text = "Adding it to your library…".into());
            files::import_world(&dir.join(&first), &worlds)
        })();
        let _ = std::fs::remove_dir_all(&dir);
        finish(&shared, t, r);
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{newer_build, newest_first};
    use serde_json::json;

    #[test]
    fn the_worlds_page_lists_newest_first() {
        let list = json!([
            { "tag_name": "world-2026-seed2026-fbd9131", "published_at": "2026-10-21T19:51:58Z" },
            { "tag_name": "world-2026-seed11-e581227", "published_at": "2026-11-01T19:27:02Z" },
            { "tag_name": "draft", "published_at": null },
            { "tag_name": "world-2026-seed2026-0d6b38c", "published_at": "2026-11-01T10:42:19Z" },
        ]);
        let tags: Vec<&str> = newest_first(&list).iter().map(|r| r["tag_name"].as_str().unwrap()).collect();
        assert_eq!(
            tags,
            ["world-2026-seed11-e581227", "world-2026-seed2026-0d6b38c", "world-2026-seed2026-fbd9131", "draft"]
        );
    }

    #[test]
    fn a_newer_build_of_a_library_world() {
        let old = json!({ "commit": "fbd9131abf232070464d2b6c1e109a24e74cfebc", "written": "2026-10-21 19:50 UTC" });
        let new = json!({ "commit": "ec7d3d9f3bc3ac277fbb932f313b66fc75e81cba", "written": "2026-10-23 20:55 UTC" });
        // The library holds the 21 October build; the 23 October one replaces it.
        assert!(newer_build(&old, "world-2026-seed2026-ec7d3d9", "2026-10-23T20:57:13Z"));
        // The same build is already there.
        assert!(!newer_build(&new, "world-2026-seed2026-ec7d3d9", "2026-10-23T20:57:13Z"));
        // A world built on this machine later than the release is kept.
        let mine = json!({ "commit": "0.2.0", "written": "2026-10-24 09:00 UTC" });
        assert!(!newer_build(&mine, "world-2026-seed2026-ec7d3d9", "2026-10-23T20:57:13Z"));
    }
}
