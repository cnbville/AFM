//! The world log (world reader lane R2): what the world works out and then
//! throws away, overwrites or deletes, written to disk as the days go by, so
//! the world reader can show every year and every why, not only the moment
//! a world file holds.
//!
//! Logging only watches. Nothing here draws a random number, changes the
//! world or changes the order anything runs in, so a world plays the same
//! with the log on or off (`tests/world_log.rs` compares every person each
//! day).
//!
//! On disk a log is a folder (`name.afmlog`):
//! - `meta.json`: the format, each session's commit and start, the world
//!   options, the experiment switches, the class pipeline's knobs and what
//!   each kind holds (`KINDS`);
//! - one folder per kind, holding segments `YYYY-NNNN.mpz`. A segment is a
//!   run of zstd frames, each frame a run of records as named MessagePack
//!   (`read.rs` reads them back). A segment is never written again once it
//!   is closed: a new one opens each football year and at each save, so a
//!   save keeps the log as it stood by hard links (`save_to`).
//!
//! Records are packed and compressed a frame (1 MB) at a time by the thread
//! that writes them (at the year-end book, every core) and appended to disk
//! by a writer thread, so the game's memory never grows with the log.
//! Every `f64` is written as an `f32` (`narrow.rs`): seven significant
//! digits, about half the disk.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::thread::JoinHandle;

use serde::{Deserialize, Serialize};

pub(crate) mod fp;
pub(crate) mod games;
mod narrow;
pub mod read;
pub(crate) mod rows;
mod snap;
pub mod why;

pub use why::Why;

/// The log's format version.
pub const FORMAT: u32 = 1;
/// A log folder's extension.
pub const EXT: &str = "afmlog";
/// Packed records per frame before it goes to the writer thread.
const FRAME: usize = 1 << 20;
/// zstd level: 3 packs a world's rows 5% bigger than 6 in less than half
/// the time (measured on a year's player rows).
const LEVEL: i32 = 3;

/// What each kind of record holds (written into `meta.json`).
pub const KINDS: &[(&str, &str)] = &[
    (
        "people",
        "What never changes about a person, once, at the first book he is in: name, heritage, birth date, home state, first high school and class, family, the team he grew up cheering for and his face.",
    ),
    (
        "players",
        "One row per person in football each year at the year-end book (before leavers are dropped): every rating and ceiling (in tenths), hidden trait, tendency (in tenths), body, background, mood, fame, money, stars, visibility, schooling, the year's recruiting and calendar state, training state, the latest season row and the pro career's year, with his true overall and potential.",
    ),
    (
        "growth",
        "The why of each yearly development step (high school and college at their checkpoints, pros at camp): the context the world gave (coaching, role, snaps, situation, morale, support, focus) and every term the step drew (injury, form, the year's event and its size, potential drive and its parts, veteran learning, growth rates, decline, fades).",
    ),
    (
        "coaches",
        "Every coach each year at the book: the whole coach (ability parts, reputation, job, contract, systems, ties, unit records, stints).",
    ),
    (
        "programs",
        "Every college program each year at the book: the whole program (prestige and its parts, grades, conference, staff seats, roster, collective, pipelines, seasons) and its season's result.",
    ),
    ("schools", "Every high school each year at the book: the whole school and its seasons."),
    (
        "clubs",
        "Every NFL club at the end of each football year: the club (roster, depth chart, books, office, scouts and files), its finances and its season row.",
    ),
    (
        "exits",
        "Everyone who leaves football, instead of that year's player row, written before the book drops him: the person as in `players`, every note he has, and why he left.",
    ),
    (
        "events",
        "Every calendar event of the year at every loudness (the date, level, kind, rule, team and the linked names), written when the year closes, and every answer the calendar resolved.",
    ),
    ("news", "Every news item, with its date and linked names, written when the year closes."),
    (
        "snapshots",
        "Whole stores on key dates: the college, high school and NFL seasons (schedules, results, polls, brackets, awards, team sheets), clubs and the league, the draft desks and boards, the market, voters, finance, people around the teams, stories, charts, other leagues and the world's fitted numbers.",
    ),
    (
        "whys",
        "Dated events with their why: each recruit's commitment and signing with the schools that pulled him and the seven reasons, portal entries with the lean behind them, position moves with both fits, coaching reviews (the read, its parts, the bar, the chance) and searches (the candidates called and their scores), the combine, each free agent's market by day, every pro transaction with its contract, transfers' first roles, the recruiting log's rows and the alumni written each year.",
    ),
    (
        "dropped",
        "What a store drops to stay small, as it drops it: life events after 150 days, health episodes after 400, closed court cases past the newest 120, the people log past 600.",
    ),
    (
        "world",
        "One row per year: the calendar's counters, the year's rule book and changes, signing classes, champions, the fitted numbers and a fingerprint of each store.",
    ),
    (
        "games-college",
        "Every college game (lane R1): kind, date, week, site, kickoff, line, weather, crowd, the toss, score by quarter, both team boxes, every player's line and snaps, drives, scoring, injuries, penalties, officiating. Stand-in games: the facts and the winner.",
    ),
    (
        "plays-college",
        "Every college engine game's snaps (lane R1): the call and its why, the defense, every block, rush win, tackle and miss, the result and its why, expected points and win chance before and after; named by level, season, date and teams.",
    ),
    ("games-nfl", "Every NFL game, preseason and playoffs too, as games-college (lane R1)."),
    ("plays-nfl", "Every NFL game's snaps, as plays-college (lane R1)."),
    ("games-other", "Every game of the other pro leagues, as games-college (lane R1)."),
    ("plays-other", "Every other-league game's snaps, as plays-college (lane R1)."),
    (
        "games-hs",
        "Every high school game (lane R1): engine games as games-college (no snaps); stand-in games and forfeits with the winner.",
    ),
    (
        "situations",
        "Each level's plays summed by situation key (lane R1): college by division, the pro leagues by league, high school by state with its play names under all. A season's totals are the sum of its records (one at the year's close, more when a save cuts it).",
    ),
    (
        "team-situations",
        "Each college and pro team's plays summed by its own keys, o| with the ball and d| defending (lane R1), summed like situations.",
    ),
];

/// What `meta.json` says.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Meta {
    pub format: u32,
    /// The world's options as built (seed, scale, history, engine years).
    pub options: crate::world::WorldOptions,
    /// Each run that wrote to this log: the first is the world's build; a
    /// career adds one each time a save opens.
    pub sessions: Vec<Session>,
    /// The experiment switches (`game::experiment`) and their state.
    pub switches: Vec<(String, bool)>,
    /// The class pipeline's knobs (`career::PIPELINE`), as text.
    pub knobs: String,
    /// What each kind holds.
    pub kinds: Vec<(String, String)>,
    /// The first person id whose `people` row is not written yet: ids only
    /// grow, so everyone from here on is new at the next book.
    #[serde(default)]
    pub people_mark: u32,
}

/// One run that wrote to a log.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub commit: String,
    /// The world date it started on.
    pub from: String,
    /// The first segment number it wrote.
    pub first_segment: u32,
    /// Where its history came from (a library world's name), if anywhere.
    #[serde(default)]
    pub history: Option<String>,
}

enum Job {
    /// A compressed frame for a segment (or why it could not be compressed).
    Frame(PathBuf, Result<Vec<u8>, String>),
    Sync(SyncSender<std::io::Result<()>>),
}

/// The log of one world, open for writing.
pub struct WorldLog {
    dir: PathBuf,
    seq: u32,
    year: i32,
    bufs: Mutex<BTreeMap<&'static str, Vec<u8>>>,
    counts: Mutex<BTreeMap<&'static str, u64>>,
    tx: Option<SyncSender<Job>>,
    worker: Option<JoinHandle<()>>,
    /// How far the append-only lists have been written (news, alumni).
    pub(crate) marks: Mutex<Marks>,
    /// The game log's situation totals not written yet (`games.rs`).
    totals: games::Totals,
}

/// How far each append-only list has been written.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Marks {
    pub news: usize,
    pub alumni: usize,
    pub portal: usize,
    pub five_stars: usize,
    pub transfer_seasons: usize,
    /// See `Meta::people_mark`.
    pub people: u32,
}

impl WorldLog {
    /// A new log in `dir` (created; an old log there is replaced).
    /// `from` is the world date it starts on (empty for a world's build).
    pub fn create(
        dir: &Path,
        options: &crate::world::WorldOptions,
        commit: &str,
        from: &str,
    ) -> std::io::Result<WorldLog> {
        if dir.exists() {
            std::fs::remove_dir_all(dir)?;
        }
        std::fs::create_dir_all(dir)?;
        let meta = Meta {
            format: FORMAT,
            options: options.clone(),
            sessions: vec![Session {
                commit: commit.to_string(),
                from: from.to_string(),
                first_segment: 0,
                history: None,
            }],
            switches: crate::game::experiment::switches().iter().map(|(n, on)| (n.to_string(), *on)).collect(),
            knobs: format!("{:#?}", crate::career::PIPELINE),
            kinds: KINDS.iter().map(|(k, w)| (k.to_string(), w.to_string())).collect(),
            people_mark: 0,
        };
        write_meta(dir, &meta)?;
        Ok(WorldLog::start(dir.to_path_buf(), 0, 0))
    }

    /// Opens a log that already has segments (a save's, linked into a live
    /// folder) to write on after them: a new session in its meta.
    pub fn resume(dir: &Path, commit: &str, from: &str, history: Option<String>) -> std::io::Result<WorldLog> {
        let mut meta = read::meta(dir)?;
        let seq = read::segments(dir)?.iter().map(|s| s.seq + 1).max().unwrap_or(0);
        meta.sessions.push(Session { commit: commit.to_string(), from: from.to_string(), first_segment: seq, history });
        meta.switches = crate::game::experiment::switches().iter().map(|(n, on)| (n.to_string(), *on)).collect();
        meta.kinds = KINDS.iter().map(|(k, w)| (k.to_string(), w.to_string())).collect();
        write_meta(dir, &meta)?;
        Ok(WorldLog::start(dir.to_path_buf(), seq, meta.people_mark))
    }

    fn start(dir: PathBuf, seq: u32, people: u32) -> WorldLog {
        // A few frames may wait for the writer; more and the world waits.
        let (tx, rx) = sync_channel::<Job>(4);
        let worker = std::thread::Builder::new()
            .name("afm-world-log".into())
            .spawn(move || writer(rx))
            .expect("the world log's writer thread");
        WorldLog {
            dir,
            seq,
            year: 0,
            bufs: Mutex::new(BTreeMap::new()),
            counts: Mutex::new(BTreeMap::new()),
            tx: Some(tx),
            worker: Some(worker),
            marks: Mutex::new(Marks { people, ..Marks::default() }),
            totals: games::Totals::default(),
        }
    }

    /// The log's folder.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Records written so far this run, by kind.
    pub fn counts(&self) -> BTreeMap<&'static str, u64> {
        self.counts.lock().unwrap().clone()
    }

    /// Adds one record of `kind`. Safe from many threads at once (the
    /// growth step runs on all cores); the order within a frame then
    /// follows the threads, so readers key records by their ids.
    pub(crate) fn put<T: Serialize + ?Sized>(&self, kind: &'static str, rec: &T) {
        // Packed before the lock, so the cores don't wait on each other.
        let mut one = Vec::with_capacity(256);
        if rmp_serde::encode::write_named(&mut one, &narrow::Narrow(rec)).is_err() {
            return;
        }
        // A record of a frame or more (a whole season's store) goes as its
        // own frame, after what was waiting, without being copied.
        let (full, own) = {
            let mut bufs = self.bufs.lock().unwrap();
            let buf = bufs.entry(kind).or_default();
            if one.len() >= FRAME {
                ((!buf.is_empty()).then(|| std::mem::take(buf)), Some(one))
            } else {
                buf.extend_from_slice(&one);
                ((buf.len() >= FRAME).then(|| std::mem::take(buf)), None)
            }
        };
        *self.counts.lock().unwrap().entry(kind).or_default() += 1;
        for bytes in [full, own].into_iter().flatten() {
            self.send(kind, bytes);
        }
    }

    fn segment(&self, kind: &str) -> PathBuf {
        self.dir.join(kind).join(format!("{}-{:04}.mpz", self.year, self.seq))
    }

    /// Compresses a frame on the calling thread (at the book that is every
    /// core) and hands it to the writer thread to append.
    fn send(&self, kind: &'static str, bytes: Vec<u8>) {
        if let Some(tx) = &self.tx {
            let z = zstd::bulk::compress(&bytes, LEVEL).map_err(|e| e.to_string());
            let _ = tx.send(Job::Frame(self.segment(kind), z));
        }
    }

    /// Sends every waiting record to disk and waits until it is written.
    pub fn flush(&self) -> std::io::Result<()> {
        let bufs = std::mem::take(&mut *self.bufs.lock().unwrap());
        for (kind, bytes) in bufs {
            if !bytes.is_empty() {
                self.send(kind, bytes);
            }
        }
        let Some(tx) = &self.tx else { return Ok(()) };
        let (ack, done) = sync_channel(1);
        tx.send(Job::Sync(ack)).map_err(|_| std::io::Error::other("the world log's writer stopped"))?;
        done.recv().map_err(|_| std::io::Error::other("the world log's writer stopped"))?
    }

    /// Closes the open segments: later records go into new ones.
    pub fn rotate(&mut self) -> std::io::Result<()> {
        self.flush()?;
        self.seq += 1;
        Ok(())
    }

    /// Everyone below `next` has his `people` row (kept in the meta, so a
    /// save's log resumes from it).
    pub(crate) fn set_people_mark(&self, next: u32) {
        self.marks.lock().unwrap().people = next;
        if let Ok(mut meta) = read::meta(&self.dir) {
            meta.people_mark = next;
            let _ = write_meta(&self.dir, &meta);
        }
    }

    /// The football year the next records are filed under.
    pub(crate) fn set_year(&mut self, year: i32) {
        if self.year != year {
            let _ = self.flush();
            self.year = year;
        }
    }

    /// Writes everything and stops the writer.
    pub fn close(mut self) -> std::io::Result<()> {
        self.write_totals();
        let r = self.flush();
        self.stop();
        r
    }

    fn stop(&mut self) {
        self.tx = None;
        if let Some(h) = self.worker.take() {
            let _ = h.join();
        }
    }

    /// Keeps the log as it stands beside a save: closes the open segments
    /// and mirrors the folder into `to` (hard links where the disk allows,
    /// copies where it doesn't), replacing what was there.
    pub fn save_to(&mut self, to: &Path) -> std::io::Result<()> {
        self.write_totals();
        self.rotate()?;
        mirror(&self.dir, to)
    }
}

impl Drop for WorldLog {
    fn drop(&mut self) {
        let _ = self.flush();
        self.stop();
    }
}

/// Mirrors a log folder into `to`, through `to.partial`, so an old copy is
/// only replaced by a whole one.
pub fn mirror(from: &Path, to: &Path) -> std::io::Result<()> {
    let tmp = to.with_extension(format!("{EXT}.partial"));
    if tmp.exists() {
        std::fs::remove_dir_all(&tmp)?;
    }
    copy_tree(from, &tmp)?;
    if to.exists() {
        std::fs::remove_dir_all(to)?;
    }
    std::fs::rename(&tmp, to)
}

fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)? {
        let e = e?;
        let (src, dst) = (e.path(), to.join(e.file_name()));
        if e.file_type()?.is_dir() {
            copy_tree(&src, &dst)?;
        } else if src.extension().is_some_and(|x| x == "json") || std::fs::hard_link(&src, &dst).is_err() {
            // The meta is rewritten by each session, so it is copied.
            std::fs::copy(&src, &dst)?;
        }
    }
    Ok(())
}

fn write_meta(dir: &Path, meta: &Meta) -> std::io::Result<()> {
    let json = serde_json::to_vec_pretty(meta).map_err(std::io::Error::other)?;
    let tmp = dir.join("meta.json.partial");
    std::fs::write(&tmp, json)?;
    std::fs::rename(tmp, dir.join("meta.json"))
}

/// The writer thread: appends each compressed frame to its segment.
fn writer(rx: Receiver<Job>) {
    let mut error: Option<std::io::Error> = None;
    for job in rx {
        match job {
            Job::Frame(path, z) => {
                if error.is_none()
                    && let Err(e) = z.map_err(std::io::Error::other).and_then(|z| append_frame(&path, &z))
                {
                    error = Some(e);
                }
            }
            Job::Sync(ack) => {
                let _ = ack.send(match error.take() {
                    Some(e) => Err(e),
                    None => Ok(()),
                });
            }
        }
    }
}

fn append_frame(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
    f.write_all(bytes)
}

/// The world's hooks: each does nothing without a log, and only reads the world.
impl crate::world::World {
    /// Starts logging into `log` from today on (a world being built, or a
    /// save opened in the game).
    pub fn attach_log(&mut self, mut log: WorldLog) {
        log.set_year(self.year);
        let mut m = log.marks.lock().unwrap();
        m.news = self.news.len();
        m.alumni = self.alumni.len();
        m.portal = self.recruiting_log.portal.len();
        m.five_stars = self.recruiting_log.five_stars.len();
        m.transfer_seasons = self.recruiting_log.transfer_seasons.len();
        drop(m);
        self.wlog = Some(Box::new(log));
    }

    /// Stops logging and hands the log back.
    pub fn detach_log(&mut self) -> Option<WorldLog> {
        self.wlog.take().map(|b| *b)
    }

    /// The log, if one is attached.
    pub fn world_log(&mut self) -> Option<&mut WorldLog> {
        self.wlog.as_deref_mut()
    }

    /// Whether this world writes a log (and history is being recorded).
    pub(crate) fn logging(&self) -> bool {
        self.wlog.is_some() && self.recording
    }
}

/// Writes the year-end book's rows and the year's close for the world as
/// it stands now (`afm_log book`: what a full-size book costs, in time and
/// disk, on a world file). Only reads the world, like every hook.
#[cfg(feature = "debug")]
pub fn book_now(w: &crate::world::World) {
    if w.wlog.is_none() {
        return;
    }
    rows::book(w, w.year);
    rows::year_end(w);
}

/// The day's hook, run first thing each day (before any system acts):
/// snapshots for the key dates.
pub(crate) fn day(w: &mut crate::world::World, d: crate::calendar::Date) {
    if !w.logging() {
        return;
    }
    let year = w.year;
    if let Some(l) = w.wlog.as_deref_mut() {
        l.set_year(year);
    }
    snap::day(w, d);
}

/// The year-end book's hook, before leavers are dropped: the yearly rows
/// and the exits.
pub(crate) fn book(w: &mut crate::world::World, season: i32) {
    if !w.logging() {
        return;
    }
    rows::book(w, season);
}

/// The football year closes (after the Super Bowl, before the calendar
/// drops the year's events): events, news, season stores, clubs and the
/// world's yearly row. The next records go into the next year's segments.
pub(crate) fn year_end(w: &mut crate::world::World) {
    if !w.logging() {
        return;
    }
    if let Some(l) = w.wlog.as_deref_mut() {
        l.write_totals();
    }
    rows::year_end(w);
}
