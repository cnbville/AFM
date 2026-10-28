//! The world library: books on the worlds page (cnbville/afm-worlds), one
//! click to download; books on this Mac; and the game's own world files and
//! saves, which open once a book is built from them.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde_json::{Value, json};

pub const WORLDS_REPO: &str = "cnbville/afm-worlds";
/// Book files: `name.afmbook`, or parts `name.afmbook.001`, `.002`, ...
pub const BOOK_EXT: &str = "afmbook";

/// A long job (a download or a book build) and how far it is.
#[derive(Default, Clone)]
pub struct Job {
    pub kind: String,
    pub name: String,
    pub done: f64,
    pub text: String,
    pub seconds: f64,
    pub running: bool,
    pub error: Option<String>,
    /// The file it made, once finished.
    pub made: Option<PathBuf>,
}

pub type Shared = Arc<Mutex<Job>>;

pub fn job_json(j: &Shared) -> Value {
    let j = j.lock().map(|j| j.clone()).unwrap_or_default();
    json!({
        "kind": j.kind, "name": j.name, "done": j.done, "text": j.text, "seconds": j.seconds,
        "running": j.running, "error": j.error, "made": j.made.map(|p| p.to_string_lossy().into_owned()),
    })
}

fn set(j: &Shared, f: impl FnOnce(&mut Job)) {
    if let Ok(mut g) = j.lock() {
        f(&mut g);
    }
}

/// The reader's own folder: `books/` and `settings.json`. On a Mac it is
/// `~/Library/Application Support/AFM Reader`, elsewhere `~/.afm-reader`;
/// `AFM_READER_HOME` overrides it.
pub fn default_home() -> PathBuf {
    if let Ok(h) = std::env::var("AFM_READER_HOME") {
        return PathBuf::from(h);
    }
    let home = std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("."));
    if cfg!(target_os = "macos") {
        home.join("Library/Application Support/AFM Reader")
    } else {
        home.join(".afm-reader")
    }
}

/// The game's folder, for its library worlds and saves (`AFM_HOME` as the
/// game reads it).
pub fn game_home() -> PathBuf {
    if let Ok(h) = std::env::var("AFM_HOME") {
        return PathBuf::from(h);
    }
    let home = std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("."));
    if cfg!(target_os = "macos") { home.join("Library/Application Support/AFM") } else { home.join(".afm") }
}

fn files_in(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some(ext))
        .collect();
    v.sort();
    v
}

fn card(p: &Path) -> Value {
    let md = std::fs::metadata(p).ok();
    let modified = md
        .as_ref()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    json!({
        "path": p.to_string_lossy(),
        "name": p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
        "gb": md.as_ref().map(|m| (m.len() as f64 / 1e7).round() / 100.0).unwrap_or(0.0),
        "mb": md.map(|m| (m.len() as f64 / 1e5).round() / 10.0).unwrap_or(0.0),
        "modified": modified,
    })
}

/// Everything on this Mac: books, and world files and saves (with whether a
/// book for each already exists).
pub fn local(home: &Path, game: &Path) -> Value {
    let books: Vec<Value> = files_in(&home.join("books"), BOOK_EXT).iter().map(|p| card(p)).collect();
    let has_book = |p: &Path| {
        let stem = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        home.join("books").join(format!("{stem}.{BOOK_EXT}")).exists()
    };
    let worlds: Vec<Value> = files_in(&game.join("worlds"), "afmworld")
        .iter()
        .map(|p| {
            let mut c = card(p);
            c["book"] = json!(has_book(p));
            c["kind"] = json!("world");
            if let Ok(m) = afm_engine::save::read_meta(p) {
                c["seed"] = json!(m.seed);
                c["years"] = json!(m.history_years);
                c["commit"] = json!(m.commit);
                c["written"] = json!(m.written);
            }
            c
        })
        .collect();
    let saves: Vec<Value> = files_in(&game.join("saves"), "afmsave")
        .iter()
        .map(|p| {
            let mut c = card(p);
            c["book"] = json!(has_book(p));
            c["kind"] = json!("save");
            if let Ok(m) = afm_engine::save::read_meta(p) {
                c["career"] = json!(m.career);
                c["date"] = json!(m.world_date);
            }
            c
        })
        .collect();
    json!({ "books": books, "worlds": worlds, "saves": saves, "folder": home.join("books").to_string_lossy() })
}

fn get(url: &str) -> Result<ureq::Response, String> {
    ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(10))
        .timeout_read(std::time::Duration::from_secs(60))
        .build()
        .get(url)
        .set("User-Agent", "AFM-Reader")
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(404, _) => "The worlds page isn't set up yet.".to_string(),
            ureq::Error::Status(code, _) => format!("The worlds page answered {code}."),
            ureq::Error::Transport(t) => format!("Could not reach the worlds page ({t})."),
        })
}

/// The book files of one release, in order (a whole file, or its parts).
fn book_assets(release: &Value) -> Vec<(String, u64, String)> {
    let mut a: Vec<(String, u64, String)> = release["assets"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|x| {
            let name = x["name"].as_str()?;
            let ok = name.rsplit_once(&format!(".{BOOK_EXT}")).is_some_and(|(_, rest)| {
                rest.is_empty() || (rest.len() == 4 && rest[1..].bytes().all(|b| b.is_ascii_digit()))
            });
            ok.then(|| {
                (
                    name.to_string(),
                    x["size"].as_u64().unwrap_or(0),
                    x["browser_download_url"].as_str().unwrap_or("").to_string(),
                )
            })
        })
        .collect();
    a.sort();
    a
}

/// The worlds on the worlds page that carry a book, newest first.
pub fn online(home: &Path) -> Result<Value, String> {
    let list: Value = get(&format!("https://api.github.com/repos/{WORLDS_REPO}/releases?per_page=40"))?
        .into_json()
        .map_err(|e| e.to_string())?;
    let rows: Vec<Value> = list
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| {
            let assets = book_assets(r);
            let first = assets.first()?.0.clone();
            let name = first.strip_suffix(".001").unwrap_or(&first);
            let name = name.strip_suffix(&format!(".{BOOK_EXT}")).unwrap_or(name).to_string();
            let mb = assets.iter().map(|a| a.1).sum::<u64>() as f64 / 1e6;
            let have = home.join("books").join(format!("{name}.{BOOK_EXT}")).exists();
            Some(json!({ "tag": r["tag_name"], "title": r["name"], "notes": r["body"], "date": r["published_at"], "name": name, "mb": mb.round(), "have": have }))
        })
        .collect();
    Ok(json!({ "repo": WORLDS_REPO, "books": rows }))
}

/// Starts downloading one release's book into `books/`, joining its parts.
pub fn download(job: &Shared, home: &Path, tag: &str) -> Result<(), String> {
    if job.lock().map(|j| j.running).unwrap_or(false) {
        return Err("Another download or build is running.".into());
    }
    let release: Value = get(&format!("https://api.github.com/repos/{WORLDS_REPO}/releases/tags/{tag}"))?
        .into_json()
        .map_err(|e| e.to_string())?;
    let assets = book_assets(&release);
    let first = assets.first().ok_or("That world has no book yet.")?.0.clone();
    let name = first.strip_suffix(".001").unwrap_or(&first).to_string();
    set(job, |j| {
        *j = Job {
            kind: "download".into(),
            name: name.clone(),
            running: true,
            text: "Starting the download…".into(),
            ..Job::default()
        }
    });
    let (job, books) = (job.clone(), home.join("books"));
    std::thread::spawn(move || {
        let t = Instant::now();
        let total: u64 = assets.iter().map(|a| a.1).sum::<u64>().max(1);
        let r = (|| -> Result<PathBuf, String> {
            std::fs::create_dir_all(&books).map_err(|e| e.to_string())?;
            let part = books.join(format!("{name}.part"));
            let mut out = std::fs::File::create(&part).map_err(|e| e.to_string())?;
            let (mut got, mut buf) = (0u64, vec![0u8; 1 << 20]);
            for (_, _, url) in &assets {
                let mut from = get(url)?.into_reader();
                loop {
                    let n = from.read(&mut buf).map_err(|e| format!("The download broke off: {e}"))?;
                    if n == 0 {
                        break;
                    }
                    out.write_all(&buf[..n]).map_err(|e| e.to_string())?;
                    got += n as u64;
                    let secs = t.elapsed().as_secs_f64();
                    set(&job, |j| {
                        j.done = got as f64 / total as f64;
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
            drop(out);
            let to = books.join(&name);
            std::fs::rename(&part, &to).map_err(|e| e.to_string())?;
            Ok(to)
        })();
        let secs = t.elapsed().as_secs_f64();
        set(&job, |j| {
            j.running = false;
            j.seconds = secs;
            match r {
                Ok(p) => {
                    j.done = 1.0;
                    j.text = format!("Downloaded in {secs:.0} s.");
                    j.made = Some(p);
                }
                Err(e) => j.error = Some(e),
            }
        });
    });
    Ok(())
}

/// Copies a book chosen from disk into `books/` (a whole file or the `.001`
/// part of a split one), so it is listed from then on.
pub fn import(home: &Path, from: &Path) -> Result<PathBuf, String> {
    let books = home.join("books");
    std::fs::create_dir_all(&books).map_err(|e| e.to_string())?;
    let s = from.to_string_lossy().into_owned();
    if let Some(base) = s.strip_suffix(".001") {
        let base = PathBuf::from(base);
        let to = books.join(base.file_name().ok_or("No file name.")?);
        let mut out = std::fs::File::create(&to).map_err(|e| e.to_string())?;
        for k in 1.. {
            let p = PathBuf::from(format!("{}.{k:03}", base.to_string_lossy()));
            if !p.exists() {
                break;
            }
            let mut f = std::fs::File::open(&p).map_err(|e| e.to_string())?;
            std::io::copy(&mut f, &mut out).map_err(|e| e.to_string())?;
        }
        return Ok(to);
    }
    if from.starts_with(&books) {
        return Ok(from.to_path_buf());
    }
    Ok(from.to_path_buf())
}
