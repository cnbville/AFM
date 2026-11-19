//! The open season's idle rows, packed (memory diet item 10).
//!
//! A player's or a team's season is a running sum that every game adds
//! to, and the Data pages read it while the season runs. A row with no
//! game through two weekly sweeps (its season is over, or he is out) is
//! packed here (named MessagePack in a zstd frame) and unpacked again when
//! the next game comes. The stores that use it (`players`, `teams`) read a
//! packed row back whole, and write every row as before, so pages, saves
//! and season files see the same values.

use std::collections::{HashMap, HashSet};

/// Days between sweeps.
pub const SWEEP_DAYS: i32 = 7;

#[derive(Clone, Default)]
pub struct Cold {
    rows: HashMap<u64, Box<[u8]>>,
    /// Keys that had a game since the last sweep, and in the sweep before.
    touched: [HashSet<u64>; 2],
    /// The day of the next sweep.
    next: i32,
}

impl std::fmt::Debug for Cold {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cold({} rows)", self.rows.len())
    }
}

fn pack<V: serde::Serialize>(v: &V) -> Box<[u8]> {
    zstd::bulk::compress(&super::file::encode(v), 3).unwrap_or_default().into_boxed_slice()
}

fn unpack<V: serde::de::DeserializeOwned + Default>(b: &[u8]) -> V {
    zstd::stream::decode_all(b).ok().and_then(|raw| super::file::decode(&raw)).unwrap_or_default()
}

impl Cold {
    /// A game on `date` for row `key`: whether a sweep is due first.
    pub fn due(&mut self, date: i32) -> bool {
        if date < self.next || crate::world::stored::kept_whole("tallies") {
            return false;
        }
        self.next = date + SWEEP_DAYS;
        true
    }

    /// The row had a game; a packed one comes back to be added to.
    pub fn touch<V: serde::de::DeserializeOwned + Default>(&mut self, key: u64) -> Option<V> {
        self.touched[0].insert(key);
        self.rows.remove(&key).map(|b| unpack(&b))
    }

    /// Whether a row went through two sweeps without a game.
    pub fn idle(&self, key: u64) -> bool {
        !self.touched[0].contains(&key) && !self.touched[1].contains(&key)
    }

    /// Packs these rows (on all cores) and starts the next sweep's count.
    pub fn sweep<V: serde::Serialize + Sync>(&mut self, rows: &[(u64, V)]) {
        use rayon::prelude::*;
        let packed: Vec<(u64, Box<[u8]>)> = rows.par_iter().map(|(k, v)| (*k, pack(v))).collect();
        self.rows.extend(packed);
    }

    /// The sweep is done: this week's games count as the week before.
    pub fn turn(&mut self) {
        self.touched.swap(0, 1);
        self.touched[0].clear();
    }

    /// A packed row, unpacked.
    pub fn get<V: serde::de::DeserializeOwned + Default>(&self, key: u64) -> Option<V> {
        self.rows.get(&key).map(|b| unpack(b))
    }

    /// A packed row taken out.
    pub fn take<V: serde::de::DeserializeOwned + Default>(&mut self, key: u64) -> Option<V> {
        self.rows.remove(&key).map(|b| unpack(&b))
    }

    /// Every packed row, unpacked, by key.
    pub fn all<V: serde::de::DeserializeOwned + Default>(&self) -> Vec<(u64, V)> {
        self.rows.iter().map(|(k, b)| (*k, unpack(b))).collect()
    }

    /// Packed rows and their bytes (memory checks).
    pub fn held(&self) -> (usize, usize) {
        (self.rows.len(), self.rows.values().map(|b| b.len()).sum())
    }
}

/// Hands a map's room back once a sweep has emptied half of it.
pub fn shrink<K: std::hash::Hash + Eq, V>(m: &mut HashMap<K, V>) {
    if m.len() < m.capacity() / 2 {
        m.shrink_to_fit();
    }
}
