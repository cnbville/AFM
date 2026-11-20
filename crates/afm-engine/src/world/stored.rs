//! Parts of a record kept only once they hold something (memory diet,
//! lane M1, `docs/design/memory-diet.md` items 2 and 4).
//!
//! Most of the world's 1.16 million people are high schoolers who never
//! touch their college-and-later fields, and two in three of them have a
//! blank recruiting calendar. `Stored<T>` keeps such a part behind a box
//! that exists only once something is written to it: reading an empty one
//! gives the part's blank value (`T::default()`), writing to it makes the
//! box first. Every value a reader sees is the same as before, so results
//! don't change; only where the bytes live does.
//!
//! - Read through `Deref` (`p.cal.lean`): an empty part reads as blank.
//! - Write through `DerefMut` (`p.cal.lean = 1.2`): the box is made then.
//!   A loop that writes blank values to everyone would make every box, so
//!   such writers go through `stored_mut` (only parts that exist) instead.
//! - `tidy` drops a part that is blank again (at the season's reset and on
//!   load), so a world read from a file keeps only the parts in use.
//!
//! The parts are written and read in world files exactly as before (each
//! record's fields flat, by name): see `Person`'s own `Serialize`.

use std::ops::{Deref, DerefMut};

/// A part that has a blank value every empty part reads as.
pub trait Blank: Default + PartialEq + Sized + 'static {
    /// The blank value, made once for the life of the program.
    fn blank() -> &'static Self;
}

/// Gives a type its blank value (`Default`, made once).
macro_rules! blank {
    ($t:ty) => {
        impl $crate::world::stored::Blank for $t {
            fn blank() -> &'static Self {
                static BLANK: std::sync::LazyLock<$t> = std::sync::LazyLock::new(<$t>::default);
                &BLANK
            }
        }
    };
}
pub(crate) use blank;

/// A part kept only once it holds something.
#[derive(Clone, PartialEq)]
pub struct Stored<T: Blank>(Option<Box<T>>);

impl<T: Blank> Default for Stored<T> {
    fn default() -> Self {
        Stored(None)
    }
}

impl<T: Blank> Stored<T> {
    /// A part with this value: kept only if it isn't blank.
    pub fn new(v: T) -> Self {
        // Checked before it is boxed: a load reads a million blank parts,
        // and boxing each to drop it would leave the allocator's pages full
        // of holes.
        if v == *T::blank() { Stored(None) } else { Stored(Some(Box::new(v))) }
    }
    /// The part, if it holds something (the rest read as blank).
    pub fn stored(&self) -> Option<&T> {
        self.0.as_deref()
    }
    /// The part to change, if it holds something: for writers that only
    /// clear or reset, so they never make a box for a blank part.
    pub fn stored_mut(&mut self) -> Option<&mut T> {
        self.0.as_deref_mut()
    }
    /// Drops the part if it is blank again.
    pub fn tidy(&mut self) {
        if self.0.as_deref().is_some_and(|v| v == T::blank()) {
            self.0 = None;
        }
    }
}

impl<T: Blank> Deref for Stored<T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        match &self.0 {
            Some(b) => b,
            None => T::blank(),
        }
    }
}

impl<T: Blank> DerefMut for Stored<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        self.0.get_or_insert_with(Box::default)
    }
}

impl<T: Blank + std::fmt::Debug> std::fmt::Debug for Stored<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (**self).fmt(f)
    }
}

/// Written as the part itself (blank when empty), read back the same way.
impl<T: Blank + serde::Serialize> serde::Serialize for Stored<T> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        (**self).serialize(s)
    }
}

impl<'de, T: Blank + serde::Deserialize<'de>> serde::Deserialize<'de> for Stored<T> {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        T::deserialize(d).map(Stored::new)
    }
}

/// Trial tooling (memory diet, lane M2): with `AFM_TRIPWIRE` set, every
/// read or write of a dropped part (a retired coach's working part, a part
/// of a man who left football) prints where it came from, once a place, so
/// a check run lists every reader of what was dropped. Off, it costs a
/// flag read on a path that is rare by design.
pub(crate) fn tripwire(what: &'static str) {
    static ON: std::sync::LazyLock<bool> = std::sync::LazyLock::new(|| std::env::var_os("AFM_TRIPWIRE").is_some());
    if *ON {
        tripped(what);
    }
}

#[cold]
#[inline(never)]
fn tripped(what: &'static str) {
    static SEEN: std::sync::Mutex<Option<std::collections::HashSet<String>>> = std::sync::Mutex::new(None);
    let bt = std::backtrace::Backtrace::force_capture().to_string();
    let frames: Vec<String> = bt
        .lines()
        .map(str::trim)
        .filter(|l| l.contains("afm_") && !l.contains("tripwire") && !l.contains("tripped") && !l.contains("Deref"))
        .filter_map(|l| l.split_once(": ").map(|x| x.1.to_string()))
        .take(5)
        .collect();
    let key = format!("{what}: {}", frames.join(" < "));
    let mut seen = SEEN.lock().unwrap_or_else(|e| e.into_inner());
    if seen.get_or_insert_with(Default::default).insert(key.clone()) {
        eprintln!("tripwire: {key}");
    }
}

/// A player kept for the record (memory diet item 3): an alumnus's full
/// player as he left, packed (MessagePack in a zstd frame), with his
/// id at hand. Most readers want only the id; a page that reads his
/// ratings, body or season rows unpacks him (`get`), a few hundredths of a
/// millisecond each. Packed as MessagePack by position (the field names
/// stay out; only this program reads it back), written, read and printed
/// as the player itself, so files and the world log's prints are the same.
#[derive(Clone, PartialEq)]
pub struct KeptPlayer {
    pub id: u32,
    /// His position as he left (pages filter on it before unpacking).
    pub position: crate::positions::Position,
    /// His unpacked length (the decompressor's room).
    raw: u32,
    bytes: Box<[u8]>,
}

thread_local! {
    // One zstd context a thread, reused: a fresh one each call costs more
    // than the unpacking itself.
    static PACK: std::cell::RefCell<zstd::bulk::Compressor<'static>> =
        std::cell::RefCell::new(zstd::bulk::Compressor::new(3).expect("a zstd context"));
    static UNPACK: std::cell::RefCell<zstd::bulk::Decompressor<'static>> =
        std::cell::RefCell::new(zstd::bulk::Decompressor::new().expect("a zstd context"));
}

impl KeptPlayer {
    pub fn new(p: &crate::player::Player) -> KeptPlayer {
        let raw = rmp_serde::to_vec(p).expect("a player packs");
        let bytes = PACK.with(|c| c.borrow_mut().compress(&raw)).expect("a player packs").into_boxed_slice();
        KeptPlayer { id: p.id, position: p.position, raw: raw.len() as u32, bytes }
    }

    /// The player, unpacked.
    pub fn get(&self) -> crate::player::Player {
        let raw =
            UNPACK.with(|c| c.borrow_mut().decompress(&self.bytes, self.raw as usize)).expect("a kept player unpacks");
        rmp_serde::from_slice(&raw).expect("a kept player reads back")
    }

    /// Bytes held (memory checks).
    pub fn packed_len(&self) -> usize {
        self.bytes.len()
    }
}

impl std::fmt::Debug for KeptPlayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.get().fmt(f)
    }
}

impl serde::Serialize for KeptPlayer {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.get().serialize(s)
    }
}

impl<'de> serde::Deserialize<'de> for KeptPlayer {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        crate::player::Player::deserialize(d).map(|p| KeptPlayer::new(&p))
    }
}

/// Trial tooling (memory diet, lane M2): `AFM_M2_KEEP=coaches,leavers,tallies`
/// keeps those parts whole (retired coaches' working part, the leavers'
/// lists, the open tallies' idle rows), so a check can tell which item
/// moved a result. Unset, everything is kept compact.
pub(crate) fn kept_whole(item: &str) -> bool {
    static KEEP: std::sync::LazyLock<String> =
        std::sync::LazyLock::new(|| std::env::var("AFM_M2_KEEP").unwrap_or_default());
    !KEEP.is_empty() && KEEP.split(',').any(|k| k.trim() == item)
}
