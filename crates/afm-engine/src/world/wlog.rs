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

use super::game_log::{GameSitu, Situ, bump};

/// The log's format; a reader refuses a newer one.
pub const FORMAT: u32 = 1;

type Enc = zstd::stream::write::Encoder<'static, BufWriter<File>>;

pub struct WorldLog {
    dir: PathBuf,
    files: BTreeMap<(i32, &'static str), Enc>,
    /// Games written per (season, level): each game's id.
    ids: HashMap<(i32, &'static str), u32>,
    /// League situation totals by (season, level, group, key) and each
    /// team's by (season, level, team, key), written when the season closes.
    situ: HashMap<(i32, &'static str, String, String), Situ>,
    team_situ: HashMap<(i32, &'static str, usize, String), Situ>,
    /// Bytes handed to each (season, stream) before compression.
    raw: BTreeMap<(i32, &'static str), u64>,
    /// The first error; the log stops writing after it.
    pub error: Option<String>,
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
    ("situations", "each level's plays summed by situation key (college by division, high school by state)"),
    ("team-situations", "each team's plays summed by its own short list of keys, o| with the ball and d| defending"),
];

impl WorldLog {
    /// Opens a log folder (made if missing) and writes its `meta.json`.
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
        Ok(WorldLog {
            dir: dir.to_path_buf(),
            files: BTreeMap::new(),
            ids: HashMap::new(),
            situ: HashMap::new(),
            team_situ: HashMap::new(),
            raw: BTreeMap::new(),
            error: None,
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
                zstd::stream::write::Encoder::new(BufWriter::with_capacity(1 << 20, f), 3)
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

    /// Writes one JSON line (without its newline).
    pub fn line(&mut self, season: i32, stream: &'static str, json: &[u8]) {
        let Some(e) = self.enc(season, stream) else { return };
        let r = e.write_all(json).and_then(|_| e.write_all(b"\n"));
        match r {
            Ok(()) => *self.raw.entry((season, stream)).or_insert(0) += json.len() as u64 + 1,
            Err(err) => self.error = Some(format!("{season}/{stream}: {err}")),
        }
    }

    /// Writes one row as a JSON line.
    pub fn row<T: serde::Serialize>(&mut self, season: i32, stream: &'static str, row: &T) {
        match serde_json::to_vec(row) {
            Ok(v) => self.line(season, stream, &v),
            Err(err) => self.error = Some(format!("{season}/{stream}: {err}")),
        }
    }

    /// Adds one game's situation totals: the league keys under its level
    /// and group (a division, a state, a league), and each side's keys
    /// under its team.
    pub fn add_situations(&mut self, season: i32, level: &'static str, group: &str, teams: [usize; 2], part: GameSitu) {
        for (k, v) in part.league {
            bump(self.situ.entry((season, level, group.to_string(), k)).or_default(), &v);
        }
        for (side, m) in part.team.into_iter().enumerate() {
            for (k, v) in m {
                bump(self.team_situ.entry((season, level, teams[side], k)).or_default(), &v);
            }
        }
    }

    /// Writes a season's situation totals and closes its files.
    pub fn close_season(&mut self, season: i32) {
        let mut rows: Vec<_> = self.situ.keys().filter(|k| k.0 == season).cloned().collect();
        rows.sort();
        for k in rows {
            let v = self.situ.remove(&k).unwrap_or_default();
            let row = serde_json::json!({ "level": k.1, "group": k.2, "key": k.3, "v": round_situ(&v) });
            self.row(season, "situations", &row);
        }
        let mut rows: Vec<_> = self.team_situ.keys().filter(|k| k.0 == season).cloned().collect();
        rows.sort();
        for k in rows {
            let v = self.team_situ.remove(&k).unwrap_or_default();
            let row = serde_json::json!({ "level": k.1, "team": k.2, "key": k.3, "v": round_situ(&v) });
            self.row(season, "team-situations", &row);
        }
        let keys: Vec<(i32, &'static str)> = self.files.keys().filter(|k| k.0 == season).copied().collect();
        for k in keys {
            if let Some(e) = self.files.remove(&k)
                && let Err(err) = e.finish().and_then(|mut w| w.flush())
            {
                self.error.get_or_insert(format!("{}/{}: {err}", k.0, k.1));
            }
        }
    }

    /// Closes every season before `season` (a new football year started).
    pub fn close_before(&mut self, season: i32) {
        let mut open: Vec<i32> = self.files.keys().map(|k| k.0).collect();
        open.extend(self.situ.keys().map(|k| k.0));
        open.extend(self.team_situ.keys().map(|k| k.0));
        open.sort();
        open.dedup();
        for s in open.into_iter().filter(|&s| s < season) {
            self.close_season(s);
        }
    }

    /// Closes everything and adds each file's size to `meta.json`. Returns
    /// the bytes on disk, or the first error.
    pub fn finish(mut self) -> Result<u64, String> {
        self.close_before(i32::MAX);
        if let Some(e) = self.error.take() {
            return Err(e);
        }
        let mut sizes = serde_json::Map::new();
        let mut total = 0;
        for (&(season, stream), &raw) in &self.raw {
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
        for (_, e) in std::mem::take(&mut self.files) {
            let _ = e.finish().and_then(|mut w| w.flush());
        }
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
