//! Long jobs beside the game: building a world on this machine, and
//! downloading one from the worlds page (Cain 2026-10-21: both). Each job runs
//! on its own thread, never holding the game, and writes its progress where
//! the screens read it. One job at a time.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

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
    mut each: impl FnMut(u32, i32) -> bool,
) -> Result<PathBuf, String> {
    let to = worlds.join(format!("{}.afmworld", world_name(seed, years)));
    let mut seasons = 0;
    let w = World::new_stoppable(WorldOptions { seed, history_years: years, scale, ..Default::default() }, |w, _| {
        seasons += 1;
        each(seasons, w.year - 1)
    })
    .ok_or(STOPPED)?;
    // Seasons done: 0 says the world is being written.
    each(0, 0);
    let part = to.with_extension("afmworld.part");
    let commit = option_env!("AFM_COMMIT").unwrap_or(env!("CARGO_PKG_VERSION"));
    afm_engine::save::write_world(&w, &part, commit, &files::now_utc())
        .and_then(|_| std::fs::rename(&part, &to))
        .map_err(|e| format!("Could not write the world: {e}"))?;
    Ok(to)
}

/// Builds a full world on this machine, beside the game, into the library.
/// A 30-year world takes hours and about 6.5 GB of memory at its peak.
pub fn build(shared: &Shared, worlds: &Path, seed: u32, years: u32, scale: f64) -> Result<String, String> {
    let name = world_name(seed, years);
    start(shared, "build", &name, "Starting the history…")?;
    let (shared, worlds) = (shared.clone(), worlds.to_path_buf());
    std::thread::spawn(move || {
        let t = Instant::now();
        // The last 12 seasons play every high school game on the engine and
        // take about six times as long as the others.
        let all = SPIN_UP_YEARS + years;
        let slow = years.min(12);
        let weight = |n: u32| n.min(all - slow) as f64 + 6.0 * n.saturating_sub(all - slow) as f64;
        let r = build_world_file(&worlds, seed, years, scale, |n, year| {
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

/// The worlds on the worlds page, newest first, each marked when it is
/// already in the library.
pub fn online(worlds: &Path) -> Result<Value, String> {
    let list: Value = get(&format!("https://api.github.com/repos/{WORLDS_REPO}/releases?per_page=30"))?
        .into_json()
        .map_err(|e| e.to_string())?;
    let rows: Vec<Value> = list
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| {
            let assets = world_assets(r);
            let first = assets.first()?.0.clone();
            let name = first.strip_suffix(".001").unwrap_or(&first);
            let name = name.strip_suffix(".afmworld").unwrap_or(name).to_string();
            let mb = assets.iter().map(|a| a.1).sum::<u64>() as f64 / 1e6;
            Some(json!({
                "tag": r["tag_name"], "title": r["name"], "notes": r["body"], "date": r["published_at"],
                "name": name, "mb": mb.round(), "have": worlds.join(format!("{name}.afmworld")).exists(),
            }))
        })
        .collect();
    Ok(json!({ "repo": WORLDS_REPO, "worlds": rows }))
}

/// Downloads one world from the worlds page into the library.
pub fn download(shared: &Shared, worlds: &Path, tag: &str) -> Result<(), String> {
    let release: Value = get(&format!("https://api.github.com/repos/{WORLDS_REPO}/releases/tags/{tag}"))?
        .into_json()
        .map_err(|e| e.to_string())?;
    let assets = world_assets(&release);
    let first = assets.first().ok_or("That release has no world file.")?.0.clone();
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
