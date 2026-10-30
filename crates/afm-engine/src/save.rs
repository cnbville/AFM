//! The world as a file (section 8, save and load): the whole `World`, every
//! random stream included, so a world read back plays on exactly as the one
//! written. The game imports a library world this way.
//!
//! Every file starts the same way:
//! - 8 bytes `AFMWORLD`, then the format version as a little-endian u32;
//! - a little-endian u32 length, then that many bytes of UTF-8 JSON: seed,
//!   history years, start year, git commit, date written, world date.
//!
//! Format 1 (agreed with the section 8 build; every file before update 3):
//! then the world as MessagePack with named fields, as one zstd level 3
//! stream. Still read.
//!
//! Format 2 (update 3, faster saves): then a table of parts, a
//! little-endian u32 count and per part 32 bytes (an 8-byte name, the item
//! count, the offset in the file and the length, each a little-endian u64),
//! then the parts, each its own zstd level 3 stream of named MessagePack:
//! - `rest`: the world without the lists below (written empty);
//! - `alumni`, `news`: those lists whole;
//! - `seasons`: the stat book's player seasons, in pieces;
//! - `people`: the people, in pieces, in order.
//!
//! The parts are packed and unpacked on every core, and the pieces of a
//! list are read straight into one list. Named fields let a newer game read
//! an older file: a new field takes `#[serde(default)]`, a removed one is
//! ignored.
//!
//! Static text (`&'static str` fields: award names, reasons, tags) is
//! written as text and interned on load (`leaked`): each distinct string is
//! kept once for the life of the program.
//!
//! Not written, rebuilt or left empty on load: the trial game log
//! (`World::game_log`, trial tooling only).

use std::cell::Cell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::Hash;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::mem::MaybeUninit;
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use serde::de::{DeserializeOwned, DeserializeSeed, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::world::World;

pub const MAGIC: &[u8; 8] = b"AFMWORLD";
/// The format this game writes. It reads this one and format 1.
pub const FORMAT_VERSION: u32 = 2;

/// The buffer between MessagePack and zstd, both ways: without it every
/// small value is its own call into zstd.
const BUFFER: usize = 1 << 20;
/// zstd's level: level 1 saves little time for 3% bigger files.
const LEVEL: i32 = 3;
/// Pieces per list: enough to keep every core busy to the end, small
/// enough that the packed pieces waiting to be written stay small.
const PEOPLE_PIECES: usize = 128;
const SEASON_PIECES: usize = 16;
/// Bytes per entry in the table of parts.
const ENTRY: usize = 32;

/// What the file says about its world, before the world itself.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldMeta {
    pub seed: u32,
    pub history_years: u32,
    pub start_year: i32,
    pub commit: String,
    pub written: String,
    pub world_date: String,
    /// A save's career: you and your team ("Pat Tester, Allen"); none in a
    /// world file.
    #[serde(default)]
    pub career: Option<String>,
    /// What the game's opening and library pages show about the file
    /// without loading it (the world's story, your team, the headlines),
    /// written by the game.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card: Option<serde_json::Value>,
    /// A career that ever used the no-fire override (Settings, a test
    /// switch): the day it was first turned on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no_fire: Option<String>,
}

thread_local! {
    /// Set while this thread writes the `rest` part.
    static IN_PARTS: Cell<bool> = const { Cell::new(false) };
}

/// `#[serde(serialize_with = "crate::save::in_parts")]` on a list written
/// as its own part in format 2: empty inside the `rest` part.
pub fn in_parts<T: Serialize, S: Serializer>(v: &[T], s: S) -> Result<S::Ok, S::Error> {
    if IN_PARTS.with(Cell::get) { s.collect_seq(std::iter::empty::<&T>()) } else { v.serialize(s) }
}

/// One part of a format 2 file, as its table entry names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Rest,
    Alumni,
    News,
    Seasons,
    People,
}

impl Kind {
    const ALL: [Kind; 5] = [Kind::Rest, Kind::Alumni, Kind::News, Kind::Seasons, Kind::People];

    fn name(self) -> [u8; 8] {
        let s: &[u8] = match self {
            Kind::Rest => b"rest",
            Kind::Alumni => b"alumni",
            Kind::News => b"news",
            Kind::Seasons => b"seasons",
            Kind::People => b"people",
        };
        let mut n = [0u8; 8];
        n[..s.len()].copy_from_slice(s);
        n
    }
}

#[derive(Clone, Copy, Debug)]
struct Part {
    kind: Kind,
    /// Items in a list's piece; 1 for the rest.
    count: u64,
    offset: u64,
    len: u64,
}

/// A list cut into `pieces` runs of about the same length, in order.
fn cut<T>(v: &[T], pieces: usize) -> Vec<&[T]> {
    if v.is_empty() {
        return vec![v];
    }
    v.chunks(v.len().div_ceil(pieces)).collect()
}

/// Runs `job(i)` for every `i` in `order` on every core, the first in
/// `order` first. Stops at the first error and returns it.
fn on_every_core(order: &[usize], job: impl Fn(usize) -> std::io::Result<()> + Sync) -> std::io::Result<()> {
    let next = AtomicUsize::new(0);
    let stop = AtomicBool::new(false);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).min(order.len()).max(1);
    let errors: Vec<std::io::Error> = std::thread::scope(|s| {
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                s.spawn(|| -> std::io::Result<()> {
                    while !stop.load(Ordering::Relaxed) {
                        let Some(&i) = order.get(next.fetch_add(1, Ordering::Relaxed)) else { break };
                        if let Err(e) = job(i) {
                            stop.store(true, Ordering::Relaxed);
                            return Err(e);
                        }
                    }
                    Ok(())
                })
            })
            .collect();
        workers.into_iter().filter_map(|t| t.join().unwrap_or_else(|p| std::panic::resume_unwind(p)).err()).collect()
    });
    errors.into_iter().next().map_or(Ok(()), Err)
}

/// Writes the world to `path` in format 2 (see the module notes), with the
/// game's card for it in the header.
pub fn write_world(
    w: &World,
    path: &Path,
    commit: &str,
    written: &str,
    card: Option<serde_json::Value>,
) -> std::io::Result<()> {
    let meta = WorldMeta {
        seed: w.options.seed,
        history_years: w.options.history_years,
        start_year: w.options.start_year,
        commit: commit.to_string(),
        written: written.to_string(),
        world_date: w.today.to_string(),
        career: w.career.as_ref().map(|c| {
            let team = w.seat.and_then(|s| match s.level {
                crate::calendar::Level::Nfl => s.nfl_team.map(|t| w.nfl_team_name(t)),
                crate::calendar::Level::College => s.program.map(|p| w.program_name(p).to_string()),
                crate::calendar::Level::HighSchool => s.program.map(|h| w.school_name(h).to_string()),
                _ => None,
            });
            team.map_or(c.you.name.clone(), |t| format!("{}, {t}", c.you.name))
        }),
        card,
        no_fire: w.career.as_ref().and_then(|c| c.no_fire_since).map(|d| d.to_string()),
    };
    // The parts in table order; the biggest single parts come first, so
    // they start first.
    let seasons = cut(&w.stats.seasons, SEASON_PIECES);
    let people = cut(&w.people, PEOPLE_PIECES);
    let mut parts = vec![
        Part { kind: Kind::Rest, count: 1, offset: 0, len: 0 },
        Part { kind: Kind::Alumni, count: w.alumni.len() as u64, offset: 0, len: 0 },
        Part { kind: Kind::News, count: w.news.len() as u64, offset: 0, len: 0 },
    ];
    parts.extend(seasons.iter().map(|s| Part { kind: Kind::Seasons, count: s.len() as u64, offset: 0, len: 0 }));
    parts.extend(people.iter().map(|s| Part { kind: Kind::People, count: s.len() as u64, offset: 0, len: 0 }));

    let mut head = Vec::new();
    head.extend_from_slice(MAGIC);
    head.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    let json = serde_json::to_vec(&meta).map_err(std::io::Error::other)?;
    head.extend_from_slice(&(json.len() as u32).to_le_bytes());
    head.extend_from_slice(&json);
    head.extend_from_slice(&(parts.len() as u32).to_le_bytes());
    let table_at = head.len() as u64;
    head.resize(head.len() + parts.len() * ENTRY, 0);
    let mut file = std::fs::File::create(path)?;
    file.write_all(&head)?;

    // Each part's named MessagePack, through the buffer.
    let encode = |i: usize, out: &mut dyn Write| -> std::io::Result<()> {
        let mut b = BufWriter::with_capacity(BUFFER, out);
        let r = match parts[i].kind {
            Kind::Rest => {
                IN_PARTS.with(|p| p.set(true));
                let r = rmp_serde::encode::write_named(&mut b, w);
                IN_PARTS.with(|p| p.set(false));
                r
            }
            Kind::Alumni => rmp_serde::encode::write_named(&mut b, &w.alumni),
            Kind::News => rmp_serde::encode::write_named(&mut b, &w.news),
            Kind::Seasons => rmp_serde::encode::write_named(&mut b, seasons[i - 3]),
            Kind::People => rmp_serde::encode::write_named(&mut b, people[i - 3 - seasons.len()]),
        };
        r.map_err(std::io::Error::other)?;
        b.flush()
    };
    // Each part is packed on its own core; one more thread writes each
    // packed part whole wherever the file has got to, and the table says
    // where. The queue to the writer is short, so few packed parts wait in
    // memory.
    let mut spots = vec![(0u64, 0u64); parts.len()];
    let (tx, rx) = std::sync::mpsc::sync_channel::<(usize, Vec<u8>)>(2);
    let order: Vec<usize> = (0..parts.len()).collect();
    let mut file = std::thread::scope(|s| -> std::io::Result<std::fs::File> {
        let spots = &mut spots;
        let mut at = head.len() as u64;
        let writer = s.spawn(move || -> std::io::Result<std::fs::File> {
            for (i, bytes) in rx {
                file.write_all(&bytes)?;
                spots[i] = (at, bytes.len() as u64);
                at += bytes.len() as u64;
            }
            Ok(file)
        });
        let packed = on_every_core(&order, |i| {
            let mut z = zstd::Encoder::new(Vec::new(), LEVEL)?;
            encode(i, &mut z)?;
            tx.send((i, z.finish()?)).map_err(|_| std::io::Error::other("the save's writer stopped"))
        });
        drop(tx);
        // The writer's own error first: it is why the packers stopped.
        let file = writer.join().unwrap_or_else(|p| std::panic::resume_unwind(p))?;
        packed.map(|_| file)
    })?;
    for (p, &(offset, len)) in parts.iter_mut().zip(&spots) {
        (p.offset, p.len) = (offset, len);
    }
    let mut table = Vec::with_capacity(parts.len() * ENTRY);
    for p in &parts {
        table.extend_from_slice(&p.kind.name());
        for x in [p.count, p.offset, p.len] {
            table.extend_from_slice(&x.to_le_bytes());
        }
    }
    file.seek(SeekFrom::Start(table_at))?;
    file.write_all(&table)?;
    file.flush()
}

/// Reads a world file's header.
pub fn read_meta(path: &Path) -> std::io::Result<WorldMeta> {
    let mut f = BufReader::new(std::fs::File::open(path)?);
    header(&mut f).map(|h| h.0)
}

/// The header's JSON block and the format version.
fn header(f: &mut impl Read) -> std::io::Result<(WorldMeta, u32)> {
    let mut magic = [0u8; 8];
    f.read_exact(&mut magic)?;
    if &magic != MAGIC {
        return Err(std::io::Error::other("not an AFM world file"));
    }
    let mut n = [0u8; 4];
    f.read_exact(&mut n)?;
    let version = u32::from_le_bytes(n);
    if !(1..=FORMAT_VERSION).contains(&version) {
        return Err(std::io::Error::other(format!(
            "world file format {version}, this game reads up to {FORMAT_VERSION}: update the game"
        )));
    }
    f.read_exact(&mut n)?;
    let mut json = vec![0u8; u32::from_le_bytes(n) as usize];
    f.read_exact(&mut json)?;
    serde_json::from_slice(&json).map(|m| (m, version)).map_err(std::io::Error::other)
}

/// Reads a world written by `write_world`, in either format.
pub fn read_world(path: &Path) -> std::io::Result<World> {
    let mut f = BufReader::new(std::fs::File::open(path)?);
    let (_, version) = header(&mut f)?;
    let mut w = if version == 1 {
        let z = BufReader::with_capacity(BUFFER, zstd::Decoder::new(f)?);
        rmp_serde::decode::from_read(z).map_err(std::io::Error::other)?
    } else {
        read_parts(path, &mut f)?
    };
    // The game keeps no trial game log.
    w.options.keep_game_log = false;
    // A file from before the contract books (update 2, lane 6): fill them.
    crate::world::deals::ensure(&mut w);
    // A file from before lane 18: real junior posts and high school
    // coordinators.
    crate::world::staff::ensure_lane18(&mut w);
    Ok(w)
}

fn bad(why: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, format!("damaged world file: {why}"))
}

/// One part's zstd stream, opened on its own handle to the file.
fn open_part(path: &Path, p: &Part) -> std::io::Result<impl Read> {
    let mut f = std::fs::File::open(path)?;
    f.seek(SeekFrom::Start(p.offset))?;
    let z = zstd::Decoder::new(f.take(p.len))?.single_frame();
    Ok(BufReader::with_capacity(BUFFER, z))
}

fn unpack<T: DeserializeOwned>(path: &Path, p: &Part) -> std::io::Result<T> {
    rmp_serde::decode::from_read(open_part(path, p)?).map_err(std::io::Error::other)
}

/// A format 2 world: the table, then every part on every core.
fn read_parts(path: &Path, f: &mut impl Read) -> std::io::Result<World> {
    let mut n = [0u8; 4];
    f.read_exact(&mut n)?;
    let mut table = vec![0u8; u32::from_le_bytes(n) as usize * ENTRY];
    f.read_exact(&mut table)?;
    let mut parts = Vec::new();
    for e in table.chunks(ENTRY) {
        let word = |k: usize| u64::from_le_bytes(e[8 + 8 * k..16 + 8 * k].try_into().unwrap());
        let kind = Kind::ALL.into_iter().find(|k| k.name() == e[..8]).ok_or_else(|| {
            std::io::Error::other("this world file has parts this game does not know: update the game")
        })?;
        parts.push(Part { kind, count: word(0), offset: word(1), len: word(2) });
    }
    let one = |k: Kind| -> std::io::Result<Part> {
        let mut it = parts.iter().filter(|p| p.kind == k);
        match (it.next(), it.next()) {
            (Some(p), None) => Ok(*p),
            _ => Err(bad("the table of parts")),
        }
    };
    let (rest, alumni, news) = (one(Kind::Rest)?, one(Kind::Alumni)?, one(Kind::News)?);
    let pieces = |k: Kind| -> Vec<usize> { parts.iter().filter(|p| p.kind == k).map(|p| p.count as usize).collect() };
    let (n_people, n_seasons) = (pieces(Kind::People).iter().sum(), pieces(Kind::Seasons).iter().sum());
    let mut people = Vec::with_capacity(n_people);
    let mut seasons = Vec::with_capacity(n_seasons);
    let (world, alumni_v, news_v) = {
        let people_in = Fill::split(&mut people, &pieces(Kind::People));
        let seasons_in = Fill::split(&mut seasons, &pieces(Kind::Seasons));
        let world = Mutex::new(None);
        let alumni_v = Mutex::new(None);
        let news_v = Mutex::new(None);
        // Each piece's place in its list.
        let mut place = vec![0; parts.len()];
        let mut seen = [0usize; 2];
        for (i, p) in parts.iter().enumerate() {
            if let k @ (Kind::People | Kind::Seasons) = p.kind {
                let s = &mut seen[(k == Kind::People) as usize];
                place[i] = *s;
                *s += 1;
            }
        }
        // The longest parts first.
        let mut order: Vec<usize> = (0..parts.len()).collect();
        order.sort_by_key(|&i| std::cmp::Reverse(parts[i].len));
        on_every_core(&order, |i| {
            let p = &parts[i];
            match p.kind {
                Kind::Rest => *world.lock().unwrap() = Some(unpack::<World>(path, p)?),
                Kind::Alumni => *alumni_v.lock().unwrap() = Some(unpack::<Vec<_>>(path, p)?),
                Kind::News => *news_v.lock().unwrap() = Some(unpack::<Vec<_>>(path, p)?),
                Kind::People => people_in[place[i]].lock().unwrap().read(open_part(path, p)?)?,
                Kind::Seasons => seasons_in[place[i]].lock().unwrap().read(open_part(path, p)?)?,
            }
            Ok(())
        })?;
        let world: World = world.into_inner().unwrap().ok_or_else(|| bad("no rest"))?;
        let alumni_v: Vec<_> = alumni_v.into_inner().unwrap().ok_or_else(|| bad("no alumni"))?;
        let news_v: Vec<_> = news_v.into_inner().unwrap().ok_or_else(|| bad("no news"))?;
        if alumni_v.len() as u64 != alumni.count || news_v.len() as u64 != news.count || rest.count != 1 {
            return Err(bad("a list's length"));
        }
        let people_in = Fill::full(people_in)?;
        let seasons_in = Fill::full(seasons_in)?;
        // Both lists are whole: their items stay in them.
        people_in.into_iter().for_each(std::mem::forget);
        seasons_in.into_iter().for_each(std::mem::forget);
        (world, alumni_v, news_v)
    };
    // Every slot of both lists holds an item (`Fill::full`).
    unsafe {
        people.set_len(n_people);
        seasons.set_len(n_seasons);
    }
    let mut w = world;
    w.people = people;
    w.alumni = alumni_v;
    w.news = news_v;
    w.stats.seasons = seasons;
    Ok(w)
}

/// One piece of a list read in place: its run of the list's spare room and
/// how many items are in it so far. Dropped before it is kept, it drops the
/// items it holds.
pub(crate) struct Fill<'a, T> {
    slots: &'a mut [MaybeUninit<T>],
    n: usize,
}

impl<'a, T> Fill<'a, T> {
    /// The spare room of `v` (with capacity for every piece) cut into runs.
    pub(crate) fn split(v: &'a mut Vec<T>, counts: &[usize]) -> Vec<Mutex<Fill<'a, T>>> {
        let mut room = v.spare_capacity_mut();
        counts
            .iter()
            .map(|&c| {
                let (a, b) = std::mem::take(&mut room).split_at_mut(c);
                room = b;
                Mutex::new(Fill { slots: a, n: 0 })
            })
            .collect()
    }

    /// The runs, once every one is full.
    pub(crate) fn full(fills: Vec<Mutex<Fill<'a, T>>>) -> std::io::Result<Vec<Fill<'a, T>>> {
        let fills: Vec<Fill<T>> = fills.into_iter().map(|f| f.into_inner().unwrap()).collect();
        if fills.iter().any(|f| f.n != f.slots.len()) {
            return Err(bad("a piece shorter than the table says"));
        }
        Ok(fills)
    }

    pub(crate) fn read(&mut self, r: impl Read) -> std::io::Result<()>
    where
        T: DeserializeOwned,
    {
        let mut de = rmp_serde::Deserializer::new(r);
        DeserializeSeed::deserialize(&mut *self, &mut de).map_err(std::io::Error::other)
    }
}

impl<T> Drop for Fill<'_, T> {
    fn drop(&mut self) {
        for s in &mut self.slots[..self.n] {
            // The first `n` slots hold items.
            unsafe { s.assume_init_drop() }
        }
    }
}

impl<'de, T: DeserializeOwned> DeserializeSeed<'de> for &mut Fill<'_, T> {
    type Value = ();
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<(), D::Error> {
        d.deserialize_seq(self)
    }
}

impl<'de, T: DeserializeOwned> Visitor<'de> for &mut Fill<'_, T> {
    type Value = ();
    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("a list")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        while let Some(x) = seq.next_element::<T>()? {
            let Some(slot) = self.slots.get_mut(self.n) else {
                return Err(serde::de::Error::custom("a piece longer than the table says"));
            };
            slot.write(x);
            self.n += 1;
        }
        Ok(())
    }
}

/// Static text read back from a file: each distinct string kept once. Each
/// thread reading a file keeps the strings it has met, so the threads
/// reading the parts rarely wait on each other for the shared pool.
fn intern(s: String) -> &'static str {
    thread_local! {
        static MET: std::cell::RefCell<HashSet<&'static str>> = std::cell::RefCell::new(HashSet::new());
    }
    if let Some(k) = MET.with(|m| m.borrow().get(s.as_str()).copied()) {
        return k;
    }
    let k = pooled(s);
    MET.with(|m| m.borrow_mut().insert(k));
    k
}

fn pooled(s: String) -> &'static str {
    static POOL: Mutex<Option<HashSet<&'static str>>> = Mutex::new(None);
    let mut pool = POOL.lock().unwrap();
    let pool = pool.get_or_insert_with(HashSet::new);
    if let Some(&k) = pool.get(s.as_str()) {
        return k;
    }
    let k: &'static str = Box::leak(s.into_boxed_str());
    pool.insert(k);
    k
}

/// A type that holds static text, read through its owned twin.
pub trait FromOwned: Sized {
    type Owned: for<'de> Deserialize<'de>;
    fn leak(o: Self::Owned) -> Self;
}

impl FromOwned for &'static str {
    type Owned = String;
    fn leak(o: String) -> Self {
        intern(o)
    }
}

macro_rules! plain {
    ($($t:ty),*) => {$(
        impl FromOwned for $t {
            type Owned = $t;
            fn leak(o: $t) -> Self { o }
        }
    )*};
}
plain!(i32, u32, u64, usize, f64, f32, bool, String, crate::calendar::Date);

impl<T: FromOwned> FromOwned for Vec<T> {
    type Owned = Vec<T::Owned>;
    fn leak(o: Self::Owned) -> Self {
        o.into_iter().map(T::leak).collect()
    }
}

impl<T: FromOwned> FromOwned for Option<T> {
    type Owned = Option<T::Owned>;
    fn leak(o: Self::Owned) -> Self {
        o.map(T::leak)
    }
}

impl<T: FromOwned, const N: usize> FromOwned for [T; N]
where
    [T::Owned; N]: for<'de> Deserialize<'de>,
{
    type Owned = [T::Owned; N];
    fn leak(o: Self::Owned) -> Self {
        o.map(T::leak)
    }
}

impl<K: FromOwned + Eq + Hash, V: FromOwned> FromOwned for HashMap<K, V>
where
    K::Owned: Eq + Hash,
{
    type Owned = HashMap<K::Owned, V::Owned>;
    fn leak(o: Self::Owned) -> Self {
        o.into_iter().map(|(k, v)| (K::leak(k), V::leak(v))).collect()
    }
}

impl<K: FromOwned + Ord, V: FromOwned> FromOwned for BTreeMap<K, V>
where
    K::Owned: Ord,
{
    type Owned = BTreeMap<K::Owned, V::Owned>;
    fn leak(o: Self::Owned) -> Self {
        o.into_iter().map(|(k, v)| (K::leak(k), V::leak(v))).collect()
    }
}

macro_rules! tuple {
    ($($n:ident),*) => {
        #[allow(non_snake_case)]
        impl<$($n: FromOwned),*> FromOwned for ($($n,)*) {
            type Owned = ($($n::Owned,)*);
            fn leak(o: Self::Owned) -> Self {
                let ($($n,)*) = o;
                ($($n::leak($n),)*)
            }
        }
    };
}
tuple!(A, B);
tuple!(A, B, C);
tuple!(A, B, C, D);
tuple!(A, B, C, D, E);

/// `#[serde(deserialize_with = "crate::save::leaked")]` on a field holding
/// static text.
pub fn leaked<'de, D: Deserializer<'de>, T: FromOwned>(d: D) -> Result<T, D::Error> {
    T::Owned::deserialize(d).map(T::leak)
}

/// A map whose values are arrays longer than serde's 32, written as pairs.
pub mod big_values {
    use super::*;

    pub fn serialize<K: Serialize, T: Serialize, const N: usize, S: Serializer>(
        m: &HashMap<K, [T; N]>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        s.collect_seq(m.iter().map(|(k, v)| (k, v.as_slice())))
    }

    pub fn deserialize<'de, K, T, const N: usize, D>(d: D) -> Result<HashMap<K, [T; N]>, D::Error>
    where
        K: Deserialize<'de> + Eq + Hash,
        T: Deserialize<'de> + Copy + Default,
        D: Deserializer<'de>,
    {
        let pairs: Vec<(K, Vec<T>)> = Vec::deserialize(d)?;
        Ok(pairs
            .into_iter()
            .map(|(k, v)| {
                let mut a = [T::default(); N];
                for (x, y) in a.iter_mut().zip(v) {
                    *x = y;
                }
                (k, a)
            })
            .collect())
    }
}

/// Static text in a saved field. The alias keeps serde from borrowing the
/// field from the file (it borrows a field written as `&str`), so `leaked`
/// can intern it instead.
pub type Text = &'static str;
