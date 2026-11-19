//! The open season's idle rows, packed (memory diet item 10).
//!
//! A player's or a team's season is a running sum that every game adds
//! to, and the Data pages read it while the season runs. A row with no
//! game through two weekly sweeps (its season is over, or he is out) is
//! packed here, many rows to a zstd frame as in a closed season's file, and
//! taken out again when his next game comes. A page reading every row
//! unpacks each frame once, so the open season reads at about the cost of
//! a closed one. The stores that use it (`players`, `teams`) read a packed
//! row back whole and write every row as before, so pages, saves and
//! season files see the same values.

use std::collections::{HashMap, HashSet};

/// Days between sweeps.
pub const SWEEP_DAYS: i32 = 7;
/// A frame holds this many bytes of rows before compression (as a closed
/// season's file does, `file::FRAME_RAW`).
const FRAME_RAW: usize = super::file::FRAME_RAW;

/// One packed frame: rows one after another (the key as a u64, the length
/// as a u32, the row as named MessagePack), in a zstd frame.
#[derive(Clone, Default)]
struct Frame {
    bytes: Box<[u8]>,
    raw: u32,
    rows: u32,
    live: u32,
}

#[derive(Clone, Default)]
pub struct Cold {
    frames: Vec<Frame>,
    /// Each packed row's frame.
    at: HashMap<u64, u32>,
    /// Keys that had a game since the last sweep, and in the sweep before.
    touched: [HashSet<u64>; 2],
    /// The day of the next sweep.
    next: i32,
}

impl std::fmt::Debug for Cold {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cold({} rows)", self.at.len())
    }
}

fn unpack(f: &Frame) -> Vec<u8> {
    zstd::bulk::decompress(&f.bytes, f.raw as usize).unwrap_or_default()
}

/// The rows of an unpacked frame, by key.
fn walk(raw: &[u8], mut f: impl FnMut(u64, &[u8])) {
    let mut at = 0;
    while at + 12 <= raw.len() {
        let key = u64::from_le_bytes(raw[at..at + 8].try_into().unwrap());
        let len = u32::from_le_bytes(raw[at + 8..at + 12].try_into().unwrap()) as usize;
        at += 12;
        if at + len > raw.len() {
            break;
        }
        f(key, &raw[at..at + len]);
        at += len;
    }
}

fn decode<V: serde::de::DeserializeOwned + Default>(b: &[u8]) -> V {
    super::file::decode(b).unwrap_or_default()
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
        self.take(key)
    }

    /// Whether a row went through two sweeps without a game.
    pub fn idle(&self, key: u64) -> bool {
        !self.touched[0].contains(&key) && !self.touched[1].contains(&key)
    }

    /// Packs these rows into new frames (on all cores), with the rows still
    /// held in frames that have lost half their rows, and starts the next
    /// sweep's count.
    pub fn sweep<V: serde::Serialize + Sync>(&mut self, rows: &[(u64, V)]) {
        use rayon::prelude::*;
        let mut out: Vec<(u64, Vec<u8>)> = rows.par_iter().map(|(k, v)| (*k, super::file::encode(v))).collect();
        for i in 0..self.frames.len() {
            let f = &self.frames[i];
            if f.live == 0 || f.live * 2 > f.rows {
                continue;
            }
            let raw = unpack(f);
            walk(&raw, |k, b| {
                if self.at.get(&k) == Some(&(i as u32)) {
                    out.push((k, b.to_vec()));
                }
            });
            self.frames[i] = Frame::default();
        }
        if out.is_empty() {
            return;
        }
        let mut chunks: Vec<Vec<(u64, Vec<u8>)>> = vec![Vec::new()];
        let mut size = 0;
        for r in out {
            if size >= FRAME_RAW {
                chunks.push(Vec::new());
                size = 0;
            }
            size += 12 + r.1.len();
            chunks.last_mut().expect("a chunk").push(r);
        }
        let packed: Vec<(Frame, Vec<u64>)> = chunks
            .into_par_iter()
            .map(|c| {
                let mut raw = Vec::with_capacity(c.iter().map(|r| 12 + r.1.len()).sum());
                for (k, b) in &c {
                    raw.extend_from_slice(&k.to_le_bytes());
                    raw.extend_from_slice(&(b.len() as u32).to_le_bytes());
                    raw.extend_from_slice(b);
                }
                let bytes = zstd::bulk::compress(&raw, super::file::LEVEL_OPEN).unwrap_or_default().into_boxed_slice();
                let n = c.len() as u32;
                (Frame { bytes, raw: raw.len() as u32, rows: n, live: n }, c.into_iter().map(|r| r.0).collect())
            })
            .collect();
        for (f, keys) in packed {
            // An emptied slot is used again before the list grows.
            let i = match self.frames.iter().position(|f| f.rows == 0) {
                Some(i) => {
                    self.frames[i] = f;
                    i
                }
                None => {
                    self.frames.push(f);
                    self.frames.len() - 1
                }
            };
            for k in keys {
                self.at.insert(k, i as u32);
            }
        }
    }

    /// The sweep is done: this week's games count as the week before.
    pub fn turn(&mut self) {
        self.touched.swap(0, 1);
        self.touched[0].clear();
    }

    /// A packed row, unpacked (its frame is unpacked to find it).
    pub fn get<V: serde::de::DeserializeOwned + Default>(&self, key: u64) -> Option<V> {
        let i = *self.at.get(&key)?;
        let mut out = None;
        walk(&unpack(&self.frames[i as usize]), |k, b| {
            if k == key && out.is_none() {
                out = Some(decode(b));
            }
        });
        out
    }

    /// A packed row taken out. Its frame lets its bytes go once no row in
    /// it is live; a frame down to half is packed again at the next sweep.
    pub fn take<V: serde::de::DeserializeOwned + Default>(&mut self, key: u64) -> Option<V> {
        let v = self.get(key)?;
        let i = self.at.remove(&key).expect("a packed row") as usize;
        let f = &mut self.frames[i];
        f.live -= 1;
        if f.live == 0 {
            *f = Frame::default();
        }
        Some(v)
    }

    /// Every packed row, unpacked, by key (each frame once, on all cores).
    pub fn all<V: serde::de::DeserializeOwned + Default + Send>(&self) -> Vec<(u64, V)> {
        use rayon::prelude::*;
        let per: Vec<Vec<(u64, V)>> = self
            .frames
            .par_iter()
            .enumerate()
            .filter(|(_, f)| f.live > 0)
            .map(|(i, f)| {
                let mut out = Vec::with_capacity(f.live as usize);
                walk(&unpack(f), |k, b| {
                    if self.at.get(&k) == Some(&(i as u32)) {
                        out.push((k, decode(b)));
                    }
                });
                out
            })
            .collect();
        per.into_iter().flatten().collect()
    }

    /// Packed rows and their bytes (memory checks).
    pub fn held(&self) -> (usize, usize) {
        (self.at.len(), self.frames.iter().map(|f| f.bytes.len()).sum())
    }
}

/// Hands a map's room back once a sweep has emptied half of it.
pub fn shrink<K: std::hash::Hash + Eq, V>(m: &mut HashMap<K, V>) {
    if m.len() < m.capacity() / 2 {
        m.shrink_to_fit();
    }
}
