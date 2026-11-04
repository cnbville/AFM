//! A season's tally file (`<kind>/<season>.tally`): records by key, packed
//! in zstd frames so a page reads one record (a player, a team, a game)
//! without unpacking the season.
//!
//! Layout: `AFMTALLY`, the format as a little-endian u32, a u32 length and
//! that much JSON (`Header`), a u32 frame count and the table (per frame:
//! first key, last key, record count, offset and length, little-endian
//! u64s but the count, a u32), then the frames. A frame unpacks to records
//! one after another: the key (u64), the length (u32) and the record as
//! named MessagePack, so a newer game reads an older file by field name
//! (every field a later lane adds takes `#[serde(default)]`).

use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

pub const MAGIC: &[u8; 8] = b"AFMTALLY";
pub const FORMAT: u32 = 1;
/// A frame is sealed at this many bytes of records (before compression).
pub const FRAME_RAW: usize = 256 * 1024;
/// zstd level for frames sealed while a season is played, and for the
/// season's close (written once, read many times).
pub const LEVEL_OPEN: i32 = 3;
pub const LEVEL_CLOSE: i32 = 9;
const ENTRY: usize = 8 + 8 + 4 + 8 + 8;

/// What a file says about itself.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Header {
    pub kind: String,
    pub season: i32,
    pub records: u64,
}

/// One sealed frame.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Frame {
    pub first: u64,
    pub last: u64,
    pub count: u32,
    pub bytes: Blob,
}

/// Bytes written as one MessagePack binary (not an array of numbers).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Blob(pub Vec<u8>);

impl serde::Serialize for Blob {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(&self.0)
    }
}

impl<'de> serde::Deserialize<'de> for Blob {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Blob, D::Error> {
        struct V;
        impl<'de> serde::de::Visitor<'de> for V {
            type Value = Blob;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("bytes")
            }
            fn visit_bytes<E: serde::de::Error>(self, v: &[u8]) -> Result<Blob, E> {
                Ok(Blob(v.to_vec()))
            }
            fn visit_byte_buf<E: serde::de::Error>(self, v: Vec<u8>) -> Result<Blob, E> {
                Ok(Blob(v))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut a: A) -> Result<Blob, A::Error> {
                let mut v = Vec::new();
                while let Some(b) = a.next_element::<u8>()? {
                    v.push(b);
                }
                Ok(Blob(v))
            }
        }
        d.deserialize_bytes(V)
    }
}

/// Records gathered into frames: pushed in key order, sealed every
/// `FRAME_RAW` bytes. What a season's open store keeps in memory and what a
/// close writes.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Frames {
    pub frames: Vec<Frame>,
    #[serde(default)]
    pub raw: Blob,
    #[serde(default)]
    first: u64,
    #[serde(default)]
    last: u64,
    #[serde(default)]
    count: u32,
    #[serde(default)]
    level: i32,
}

impl Frames {
    pub fn new(level: i32) -> Frames {
        Frames { level, ..Default::default() }
    }

    /// Adds a record (`rec`: named MessagePack) under `key`.
    pub fn push(&mut self, key: u64, rec: &[u8]) {
        if self.count == 0 {
            self.first = key;
        }
        self.last = key;
        self.count += 1;
        let raw = &mut self.raw.0;
        raw.extend_from_slice(&key.to_le_bytes());
        raw.extend_from_slice(&(rec.len() as u32).to_le_bytes());
        raw.extend_from_slice(rec);
        if raw.len() >= FRAME_RAW {
            self.seal();
        }
    }

    /// Packs the records waiting into a frame.
    pub fn seal(&mut self) {
        if self.count == 0 {
            return;
        }
        let level = if self.level == 0 { LEVEL_OPEN } else { self.level };
        let bytes = zstd::bulk::compress(&self.raw.0, level).unwrap_or_default();
        self.frames.push(Frame { first: self.first, last: self.last, count: self.count, bytes: Blob(bytes) });
        self.raw.0.clear();
        self.raw.0.shrink_to_fit();
        self.count = 0;
    }

    pub fn records(&self) -> u64 {
        self.frames.iter().map(|f| f.count as u64).sum::<u64>() + self.count as u64
    }

    pub fn is_empty(&self) -> bool {
        self.records() == 0
    }

    /// Every record, in order, sealed or not.
    pub fn each(&self, mut f: impl FnMut(u64, &[u8])) {
        for fr in &self.frames {
            if let Ok(raw) = unpack(&fr.bytes.0) {
                walk(&raw, &mut f);
            }
        }
        walk(&self.raw.0, &mut f);
    }

    /// The records under these keys (sorted or not), from the frames that
    /// can hold them.
    pub fn get_many(&self, keys: &[u64], mut f: impl FnMut(u64, &[u8])) {
        let want: std::collections::HashSet<u64> = keys.iter().copied().collect();
        for fr in &self.frames {
            if keys.iter().any(|&k| fr.first <= k && k <= fr.last)
                && let Ok(raw) = unpack(&fr.bytes.0)
            {
                walk(&raw, &mut |k, r| {
                    if want.contains(&k) {
                        f(k, r)
                    }
                });
            }
        }
        walk(&self.raw.0, &mut |k, r| {
            if want.contains(&k) {
                f(k, r)
            }
        });
    }

    /// The whole file's bytes.
    pub fn file_bytes(mut self, kind: &str, season: i32) -> Vec<u8> {
        self.seal();
        let head = serde_json::to_vec(&Header { kind: kind.into(), season, records: self.records() }).unwrap();
        let mut out = Vec::new();
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&FORMAT.to_le_bytes());
        out.extend_from_slice(&(head.len() as u32).to_le_bytes());
        out.extend_from_slice(&head);
        out.extend_from_slice(&(self.frames.len() as u32).to_le_bytes());
        let mut offset = (out.len() + self.frames.len() * ENTRY) as u64;
        for fr in &self.frames {
            out.extend_from_slice(&fr.first.to_le_bytes());
            out.extend_from_slice(&fr.last.to_le_bytes());
            out.extend_from_slice(&fr.count.to_le_bytes());
            out.extend_from_slice(&offset.to_le_bytes());
            out.extend_from_slice(&(fr.bytes.0.len() as u64).to_le_bytes());
            offset += fr.bytes.0.len() as u64;
        }
        for fr in &self.frames {
            out.extend_from_slice(&fr.bytes.0);
        }
        out
    }
}

fn unpack(z: &[u8]) -> std::io::Result<Vec<u8>> {
    zstd::stream::decode_all(z)
}

fn walk(raw: &[u8], f: &mut dyn FnMut(u64, &[u8])) {
    let mut at = 0;
    while at + 12 <= raw.len() {
        let key = u64::from_le_bytes(raw[at..at + 8].try_into().unwrap());
        let len = u32::from_le_bytes(raw[at + 8..at + 12].try_into().unwrap()) as usize;
        at += 12;
        if at + len > raw.len() {
            return;
        }
        f(key, &raw[at..at + len]);
        at += len;
    }
}

/// A record as it is written.
pub fn encode<T: serde::Serialize>(t: &T) -> Vec<u8> {
    rmp_serde::to_vec_named(t).unwrap_or_default()
}

/// A record read back (None when it doesn't read).
pub fn decode<T: serde::de::DeserializeOwned>(b: &[u8]) -> Option<T> {
    rmp_serde::from_slice(b).ok()
}

/// Writes a file whole: beside it first, then moved over (so a save that
/// hard-links the old file keeps the old one).
pub fn write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp: PathBuf = path.with_extension("tally.partial");
    {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all().ok();
    }
    std::fs::rename(&tmp, path)
}

/// A file's header and frame table.
pub struct Opened {
    pub header: Header,
    file: std::fs::File,
    table: Vec<(u64, u64, u32, u64, u64)>,
}

/// Opens a season's file for reading.
pub fn open(path: &Path) -> std::io::Result<Opened> {
    let mut file = std::fs::File::open(path)?;
    let mut b4 = [0u8; 4];
    let mut magic = [0u8; 8];
    file.read_exact(&mut magic)?;
    if &magic != MAGIC {
        return Err(std::io::Error::other("not a tally file"));
    }
    file.read_exact(&mut b4)?;
    if u32::from_le_bytes(b4) > FORMAT {
        return Err(std::io::Error::other("a tally file from a newer game"));
    }
    file.read_exact(&mut b4)?;
    let mut head = vec![0u8; u32::from_le_bytes(b4) as usize];
    file.read_exact(&mut head)?;
    let header: Header = serde_json::from_slice(&head).map_err(std::io::Error::other)?;
    file.read_exact(&mut b4)?;
    let n = u32::from_le_bytes(b4) as usize;
    let mut t = vec![0u8; n * ENTRY];
    file.read_exact(&mut t)?;
    let u = |b: &[u8]| u64::from_le_bytes(b.try_into().unwrap());
    let table = t
        .chunks(ENTRY)
        .map(|e| {
            (u(&e[0..8]), u(&e[8..16]), u32::from_le_bytes(e[16..20].try_into().unwrap()), u(&e[20..28]), u(&e[28..36]))
        })
        .collect();
    Ok(Opened { header, file, table })
}

impl Opened {
    fn frame(&mut self, k: usize) -> std::io::Result<Vec<u8>> {
        let (_, _, _, off, len) = self.table[k];
        self.file.seek(SeekFrom::Start(off))?;
        let mut z = vec![0u8; len as usize];
        self.file.read_exact(&mut z)?;
        unpack(&z)
    }

    /// Every record in key order.
    pub fn each(&mut self, mut f: impl FnMut(u64, &[u8])) -> std::io::Result<()> {
        for k in 0..self.table.len() {
            let raw = self.frame(k)?;
            walk(&raw, &mut f);
        }
        Ok(())
    }

    /// The records under these keys.
    pub fn get_many(&mut self, keys: &[u64], mut f: impl FnMut(u64, &[u8])) -> std::io::Result<()> {
        let want: std::collections::HashSet<u64> = keys.iter().copied().collect();
        for k in 0..self.table.len() {
            let (first, last, ..) = self.table[k];
            if !keys.iter().any(|&x| first <= x && x <= last) {
                continue;
            }
            let raw = self.frame(k)?;
            walk(&raw, &mut |key, r| {
                if want.contains(&key) {
                    f(key, r)
                }
            });
        }
        Ok(())
    }

    /// One record.
    pub fn get(&mut self, key: u64) -> std::io::Result<Option<Vec<u8>>> {
        let mut out = None;
        self.get_many(&[key], |_, r| out = Some(r.to_vec()))?;
        Ok(out)
    }
}

/// Every record of a file's bytes held in memory (a season not on disk).
pub fn each_in(bytes: &[u8], mut f: impl FnMut(u64, &[u8])) -> std::io::Result<Header> {
    let tmp = std::io::Cursor::new(bytes);
    let mut r = std::io::BufReader::new(tmp);
    let mut magic = [0u8; 8];
    let mut b4 = [0u8; 4];
    r.read_exact(&mut magic)?;
    r.read_exact(&mut b4)?;
    r.read_exact(&mut b4)?;
    let mut head = vec![0u8; u32::from_le_bytes(b4) as usize];
    r.read_exact(&mut head)?;
    let header: Header = serde_json::from_slice(&head).map_err(std::io::Error::other)?;
    r.read_exact(&mut b4)?;
    let n = u32::from_le_bytes(b4) as usize;
    let mut t = vec![0u8; n * ENTRY];
    r.read_exact(&mut t)?;
    for e in t.chunks(ENTRY) {
        let off = u64::from_le_bytes(e[20..28].try_into().unwrap()) as usize;
        let len = u64::from_le_bytes(e[28..36].try_into().unwrap()) as usize;
        let raw = unpack(&bytes[off..off + len])?;
        walk(&raw, &mut f);
    }
    Ok(header)
}
