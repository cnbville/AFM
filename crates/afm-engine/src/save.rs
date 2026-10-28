//! The world as a file (section 8, save and load): the whole `World`, every
//! random stream included, so a world read back plays on exactly as the one
//! written. The game imports a library world this way.
//!
//! Format version 1 (agreed with the section 8 build):
//! - 8 bytes `AFMWORLD`, then the format version as a little-endian u32;
//! - a little-endian u32 length, then that many bytes of UTF-8 JSON: seed,
//!   history years, start year, git commit, date written, world date;
//! - the world as MessagePack with named fields, streamed through zstd
//!   level 3. Named fields let a newer game read an older file: a new field
//!   takes `#[serde(default)]`, a removed one is ignored.
//!
//! Static text (`&'static str` fields: award names, reasons, tags) is
//! written as text and interned on load (`leaked`): each distinct string is
//! kept once for the life of the program.
//!
//! Not written, rebuilt or left empty on load: the trial game log
//! (`World::game_log`, trial tooling only).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::Hash;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Mutex;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::world::World;

pub const MAGIC: &[u8; 8] = b"AFMWORLD";
pub const FORMAT_VERSION: u32 = 1;

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
}

/// Writes the world to `path` (see the module notes for the format), with
/// the game's card for it in the header.
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
    };
    let file = std::fs::File::create(path)?;
    let mut out = std::io::BufWriter::new(file);
    out.write_all(MAGIC)?;
    out.write_all(&FORMAT_VERSION.to_le_bytes())?;
    let json = serde_json::to_vec(&meta).map_err(std::io::Error::other)?;
    out.write_all(&(json.len() as u32).to_le_bytes())?;
    out.write_all(&json)?;
    let mut z = zstd::Encoder::new(out, 3)?;
    rmp_serde::encode::write_named(&mut z, w).map_err(std::io::Error::other)?;
    z.finish()?.flush()
}

/// Reads a world file's header.
pub fn read_meta(path: &Path) -> std::io::Result<WorldMeta> {
    let mut f = std::io::BufReader::new(std::fs::File::open(path)?);
    header(&mut f)
}

fn header(f: &mut impl Read) -> std::io::Result<WorldMeta> {
    let mut magic = [0u8; 8];
    f.read_exact(&mut magic)?;
    if &magic != MAGIC {
        return Err(std::io::Error::other("not an AFM world file"));
    }
    let mut n = [0u8; 4];
    f.read_exact(&mut n)?;
    let version = u32::from_le_bytes(n);
    if version != FORMAT_VERSION {
        return Err(std::io::Error::other(format!("world file format {version}, this game reads {FORMAT_VERSION}")));
    }
    f.read_exact(&mut n)?;
    let mut json = vec![0u8; u32::from_le_bytes(n) as usize];
    f.read_exact(&mut json)?;
    serde_json::from_slice(&json).map_err(std::io::Error::other)
}

/// Reads a world written by `write_world`.
pub fn read_world(path: &Path) -> std::io::Result<World> {
    let mut f = std::io::BufReader::new(std::fs::File::open(path)?);
    header(&mut f)?;
    let z = zstd::Decoder::new(f)?;
    let mut w: World = rmp_serde::decode::from_read(z).map_err(std::io::Error::other)?;
    // The game keeps no trial game log.
    w.options.keep_game_log = false;
    // A file from before the contract books (update 2, lane 6): fill them.
    crate::world::deals::ensure(&mut w);
    // A file from before lane 18: real junior posts and high school
    // coordinators.
    crate::world::staff::ensure_lane18(&mut w);
    Ok(w)
}

/// Static text read back from a file: each distinct string kept once.
fn intern(s: String) -> &'static str {
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
