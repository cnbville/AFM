//! The world log on disk (world reader, lanes R1 and R2): what the world
//! works out and then throws away, written beside the world file as it
//! happens, so memory does not grow. Its own folder holds `meta.json`
//! (the format, the world's options, what each stream holds) and one
//! folder per season with one file per stream:
//! `<season>/<stream>.jsonl.zst`, zstd-compressed JSON lines. A season's
//! files close once the football year after it starts.
//!
//! Rows are compact: a field that is zero, false, empty or missing is left
//! out (read it as zero), and decimals are rounded (one place from 10 up,
//! three below). Every row carries ids, so every name links to the world
//! file and the other streams.
//!
//! Logging only watches: nothing here draws a number or changes the world,
//! and rows are written in the world's own order, so a seed gives the same
//! world with the log on or off.

use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::thread::JoinHandle;

use super::game_log::{GameSitu, Situ, bump};

/// The log's format; a reader refuses a newer one.
pub const FORMAT: u32 = 1;

/// zstd's level: level 8 writes about a fifth less than level 3 for
/// about 6 MB a stream, on the writer's own thread.
const LEVEL: i32 = 8;

/// High school situation keys summed for the whole country, not by state:
/// the play names (thousands of calls and fronts in every state).
const HS_NATIONAL: &[&str] = &["call|", "defense|"];

type Enc = zstd::stream::write::Encoder<'static, BufWriter<File>>;

pub struct WorldLog {
    dir: PathBuf,
    /// The writer's thread compresses and writes the lines in the order
    /// they are sent; the world never waits on the disk unless it is
    /// `QUEUE` lines ahead.
    tx: Option<SyncSender<Job>>,
    writer: Option<JoinHandle<Files>>,
    /// Games written per (season, level): each game's id.
    ids: HashMap<(i32, &'static str), u32>,
    /// League situation totals by (season, level, group) then key, and
    /// each team's by (season, level, team) then key, written when the
    /// season closes.
    situ: HashMap<(i32, &'static str, String), HashMap<String, Situ>>,
    team_situ: HashMap<(i32, &'static str, usize), HashMap<String, Situ>>,
}

/// Lines the world may run ahead of the writer.
const QUEUE: usize = 256;

enum Job {
    Line(i32, &'static str, Vec<u8>),
    /// Finish the files of every season before this one.
    CloseBefore(i32),
}

/// The writer's side: the open files and what went into them.
struct Files {
    dir: PathBuf,
    files: BTreeMap<(i32, &'static str), Enc>,
    /// Bytes handed to each (season, stream) before compression.
    raw: BTreeMap<(i32, &'static str), u64>,
    /// The first error; the log stops writing after it.
    error: Option<String>,
}

/// What each stream holds, for `meta.json`.
pub const STREAMS: &[(&str, &str)] = &[
    (
        "games-college",
        "every college game: kind, date, site, weather, line, score by quarter, box, player lines, drives, scoring, injuries, officiating",
    ),
    (
        "plays-college",
        "every college game's plays, snap by snap: call, battles, result, why, expected points and win chance before and after",
    ),
    ("games-nfl", "every NFL game, preseason and playoffs too, as games-college"),
    ("plays-nfl", "every NFL game's plays, as plays-college"),
    ("games-other", "every game of the other pro leagues, as games-college"),
    ("plays-other", "every other-league game's plays, as plays-college"),
    ("games-hs", "every high school game: engine games as games-college; stand-in games with the winner only"),
    (
        "situations",
        "each level's plays summed by situation key (college by division, the pro leagues by league, high school by state with the play names for the whole country)",
    ),
    (
        "team-situations",
        "each college and pro team's plays summed by its own short list of keys, o| with the ball and d| defending",
    ),
];

impl Files {
    fn enc(&mut self, season: i32, stream: &'static str) -> Option<&mut Enc> {
        if self.error.is_some() {
            return None;
        }
        if !self.files.contains_key(&(season, stream)) {
            let open = || -> std::io::Result<Enc> {
                let dir = self.dir.join(season.to_string());
                std::fs::create_dir_all(&dir)?;
                // Appending: a stream opened again (a save going on from a
                // world's log) adds a frame, which readers read through.
                let f = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(dir.join(format!("{stream}.jsonl.zst")))?;
                zstd::stream::write::Encoder::new(BufWriter::with_capacity(1 << 20, f), LEVEL)
            };
            match open() {
                Ok(e) => {
                    self.files.insert((season, stream), e);
                }
                Err(e) => {
                    self.error = Some(format!("{season}/{stream}: {e}"));
                    return None;
                }
            }
        }
        self.files.get_mut(&(season, stream))
    }

    fn line(&mut self, season: i32, stream: &'static str, json: &[u8]) {
        let Some(e) = self.enc(season, stream) else { return };
        let r = e.write_all(json).and_then(|_| e.write_all(b"\n"));
        match r {
            Ok(()) => *self.raw.entry((season, stream)).or_insert(0) += json.len() as u64 + 1,
            Err(err) => self.error = Some(format!("{season}/{stream}: {err}")),
        }
    }

    /// Finishes every file of the seasons before `season`.
    fn close_before(&mut self, season: i32) {
        let keys: Vec<(i32, &'static str)> = self.files.keys().filter(|k| k.0 < season).copied().collect();
        for k in keys {
            if let Some(e) = self.files.remove(&k)
                && let Err(err) = e.finish().and_then(|mut w| w.flush())
            {
                self.error.get_or_insert(format!("{}/{}: {err}", k.0, k.1));
            }
        }
    }
}

/// The writer's thread: lines in the order sent, then every file closed.
fn write(mut f: Files, rx: Receiver<Job>) -> Files {
    for job in rx {
        match job {
            Job::Line(season, stream, json) => f.line(season, stream, &json),
            Job::CloseBefore(season) => f.close_before(season),
        }
    }
    f.close_before(i32::MAX);
    f
}

impl WorldLog {
    /// Opens a log folder (made if missing), writes its `meta.json` and
    /// starts the writer's thread.
    pub fn create(dir: &Path, meta: serde_json::Value) -> std::io::Result<WorldLog> {
        std::fs::create_dir_all(dir)?;
        let mut m = match meta {
            serde_json::Value::Object(m) => m,
            _ => serde_json::Map::new(),
        };
        m.insert("format".into(), FORMAT.into());
        let streams: serde_json::Map<String, serde_json::Value> =
            STREAMS.iter().map(|(k, v)| (k.to_string(), serde_json::Value::from(*v))).collect();
        m.entry("streams").or_insert(serde_json::Value::Object(streams));
        std::fs::write(dir.join("meta.json"), serde_json::to_vec_pretty(&m)?)?;
        let files = Files { dir: dir.to_path_buf(), files: BTreeMap::new(), raw: BTreeMap::new(), error: None };
        let (tx, rx) = sync_channel(QUEUE);
        let writer = std::thread::Builder::new().name("world log".into()).spawn(move || write(files, rx))?;
        Ok(WorldLog {
            dir: dir.to_path_buf(),
            tx: Some(tx),
            writer: Some(writer),
            ids: HashMap::new(),
            situ: HashMap::new(),
            team_situ: HashMap::new(),
        })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The next game id for a season and level (0, 1, 2... in the order
    /// the world plays them).
    pub fn next_id(&mut self, season: i32, level: &'static str) -> u32 {
        let n = self.ids.entry((season, level)).or_insert(0);
        *n += 1;
        *n - 1
    }

    fn send(&mut self, job: Job) {
        // A writer that stopped (it panicked) has its error kept for
        // `finish`; the world goes on without the log.
        if let Some(tx) = &self.tx
            && tx.send(job).is_err()
        {
            self.tx = None;
        }
    }

    /// Writes one JSON line (without its newline).
    pub fn line(&mut self, season: i32, stream: &'static str, json: Vec<u8>) {
        self.send(Job::Line(season, stream, json));
    }

    /// Writes one row as a JSON line.
    pub fn row<T: serde::Serialize>(&mut self, season: i32, stream: &'static str, row: &T) {
        if let Ok(v) = serde_json::to_vec(row) {
            self.line(season, stream, v);
        }
    }

    /// Adds one game's situation totals: the league keys under its level
    /// and group (a division, a state, a league), and each side's keys
    /// under its team. High school keeps no team keys, and its play names
    /// go under the group "all" (`HS_NATIONAL`).
    pub fn add_situations(&mut self, season: i32, level: &'static str, group: &str, teams: [usize; 2], part: GameSitu) {
        let hs = level == "hs";
        let mut national = HashMap::new();
        let m = self.situ.entry((season, level, group.to_string())).or_default();
        for (k, v) in part.league {
            if hs && HS_NATIONAL.iter().any(|p| k.starts_with(p)) {
                national.insert(k, v);
            } else {
                bump(m.entry(k).or_default(), &v);
            }
        }
        if !national.is_empty() {
            let m = self.situ.entry((season, level, "all".to_string())).or_default();
            for (k, v) in national {
                bump(m.entry(k).or_default(), &v);
            }
        }
        if hs {
            return;
        }
        for (side, keys) in part.team.into_iter().enumerate() {
            let m = self.team_situ.entry((season, level, teams[side])).or_default();
            for (k, v) in keys {
                bump(m.entry(k).or_default(), &v);
            }
        }
    }

    /// Writes a season's situation totals.
    fn write_situations(&mut self, season: i32) {
        let mut groups: Vec<_> = self.situ.keys().filter(|k| k.0 == season).cloned().collect();
        groups.sort();
        for g in groups {
            let mut keys: Vec<_> = self.situ.remove(&g).unwrap_or_default().into_iter().collect();
            keys.sort_by(|a, b| a.0.cmp(&b.0));
            for (k, v) in keys {
                let row = serde_json::json!({ "level": g.1, "group": g.2, "key": k, "v": round_situ(&v) });
                self.row(season, "situations", &row);
            }
        }
        let mut teams: Vec<_> = self.team_situ.keys().filter(|k| k.0 == season).copied().collect();
        teams.sort();
        for t in teams {
            let mut keys: Vec<_> = self.team_situ.remove(&t).unwrap_or_default().into_iter().collect();
            keys.sort_by(|a, b| a.0.cmp(&b.0));
            for (k, v) in keys {
                let row = serde_json::json!({ "level": t.1, "team": t.2, "key": k, "v": round_situ(&v) });
                self.row(season, "team-situations", &row);
            }
        }
    }

    /// Closes every season before `season` (a new football year started):
    /// writes their situation totals and finishes their files.
    pub fn close_before(&mut self, season: i32) {
        let mut open: Vec<i32> = self.situ.keys().map(|k| k.0).chain(self.team_situ.keys().map(|k| k.0)).collect();
        open.sort();
        open.dedup();
        for s in open.into_iter().filter(|&s| s < season) {
            self.write_situations(s);
        }
        // A season's totals are gone from memory once written.
        self.situ.shrink_to_fit();
        self.team_situ.shrink_to_fit();
        self.send(Job::CloseBefore(season));
    }

    /// Waits for the writer and returns its files' record.
    fn join(&mut self) -> Option<Files> {
        self.tx = None;
        self.writer.take().and_then(|h| h.join().ok())
    }

    /// Closes everything and adds each file's size to `meta.json`. Returns
    /// the bytes on disk, or the first error.
    pub fn finish(mut self) -> Result<u64, String> {
        self.close_before(i32::MAX);
        let Some(mut files) = self.join() else {
            return Err("the log's writer stopped".into());
        };
        if let Some(e) = files.error.take() {
            return Err(e);
        }
        let mut sizes = serde_json::Map::new();
        let mut total = 0;
        for (&(season, stream), &raw) in &files.raw {
            let disk = std::fs::metadata(self.dir.join(season.to_string()).join(format!("{stream}.jsonl.zst")))
                .map_or(0, |m| m.len());
            total += disk;
            sizes.insert(format!("{season}/{stream}"), serde_json::json!({ "bytes": disk, "raw": raw }));
        }
        let path = self.dir.join("meta.json");
        let mut meta: serde_json::Value = std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_else(|| serde_json::json!({}));
        if let Some(m) = meta.as_object_mut() {
            m.insert("files".into(), serde_json::Value::Object(sizes));
            m.insert("bytes".into(), total.into());
        }
        std::fs::write(&path, serde_json::to_vec_pretty(&meta).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        Ok(total)
    }
}

impl Drop for WorldLog {
    /// A log dropped unfinished (a stopped build) still closes its frames.
    fn drop(&mut self) {
        self.join();
    }
}

fn round_situ(v: &Situ) -> Vec<f64> {
    v.iter().map(|x| (x * 100.0).round() / 100.0).collect()
}

/// Rounds a decimal the log's way: one place from 10 up, three below; a
/// whole number stays whole.
pub fn round(x: f64) -> f64 {
    if x.fract() == 0.0 || !x.is_finite() {
        x
    } else if x.abs() >= 10.0 {
        (x * 10.0).round() / 10.0
    } else {
        (x * 1000.0).round() / 1000.0
    }
}

/// Makes a row compact: drops every field that is null, false, zero, ""
/// or empty (an array keeps its places), rounds decimals and writes whole
/// decimals as whole numbers.
pub fn compact(v: &mut serde_json::Value) {
    use serde_json::Value;
    match v {
        Value::Object(m) => {
            m.retain(|_, x| {
                compact(x);
                !empty(x)
            });
        }
        Value::Array(a) => a.iter_mut().for_each(compact),
        Value::Number(n) => {
            if let Some(f) = n.as_f64()
                && !n.is_i64()
                && !n.is_u64()
            {
                let r = round(f);
                *v = if r.fract() == 0.0 && r.abs() < 9.0e15 {
                    Value::from(r as i64)
                } else {
                    serde_json::Number::from_f64(r).map_or(Value::Null, Value::Number)
                };
            }
        }
        _ => {}
    }
}

fn empty(v: &serde_json::Value) -> bool {
    use serde_json::Value;
    match v {
        Value::Null => true,
        Value::Bool(b) => !b,
        Value::Number(n) => n.as_f64() == Some(0.0),
        Value::String(s) => s.is_empty(),
        Value::Array(a) => a.is_empty(),
        Value::Object(m) => m.is_empty(),
    }
}

/// A row's JSON with its id first: `{"id":7,...}`.
pub fn with_id(id: u32, body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(body.len() + 16);
    out.extend_from_slice(format!("{{\"id\":{id}").as_bytes());
    match body {
        [b'{', b'}'] | [] => out.push(b'}'),
        [b'{', rest @ ..] => {
            out.push(b',');
            out.extend_from_slice(rest);
        }
        _ => out.push(b'}'),
    }
    out
}
