//! Reading a world log back: its meta, its segments and their records,
//! typed or as JSON (for the book builder, `afm_log` and the tests).

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde::de::{DeserializeOwned, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value};

use super::Meta;

/// One segment file: a kind's records from one football year and session.
#[derive(Clone, Debug)]
pub struct Segment {
    pub kind: String,
    pub year: i32,
    pub seq: u32,
    pub path: PathBuf,
}

/// A log's `meta.json`.
pub fn meta(dir: &Path) -> std::io::Result<Meta> {
    let bytes = std::fs::read(dir.join("meta.json"))?;
    serde_json::from_slice(&bytes).map_err(std::io::Error::other)
}

/// Every segment in a log, by kind, year and session order.
pub fn segments(dir: &Path) -> std::io::Result<Vec<Segment>> {
    let mut out = Vec::new();
    for k in std::fs::read_dir(dir)? {
        let k = k?;
        if !k.file_type()?.is_dir() {
            continue;
        }
        let kind = k.file_name().to_string_lossy().to_string();
        for f in std::fs::read_dir(k.path())? {
            let path = f?.path();
            let Some(stem) = path.file_name().and_then(|n| n.to_str()).and_then(|n| n.strip_suffix(".mpz")) else {
                continue;
            };
            let Some((y, s)) = stem.rsplit_once('-') else { continue };
            let (Ok(year), Ok(seq)) = (y.parse(), s.parse()) else { continue };
            out.push(Segment { kind: kind.clone(), year, seq, path });
        }
    }
    out.sort_by(|a, b| (&a.kind, a.year, a.seq).cmp(&(&b.kind, b.year, b.seq)));
    Ok(out)
}

/// The segments of one kind.
pub fn kind(dir: &Path, kind: &str) -> std::io::Result<Vec<Segment>> {
    Ok(segments(dir)?.into_iter().filter(|s| s.kind == kind).collect())
}

/// Calls `f` with each record of a segment, read as `T`, in order. Stops at
/// the first record that does not read (a segment cut short by a crash).
pub fn each<T: DeserializeOwned>(path: &Path, mut f: impl FnMut(T)) -> std::io::Result<u64> {
    let file = std::fs::File::open(path)?;
    let mut r = BufReader::with_capacity(1 << 16, zstd::Decoder::new(file)?);
    let mut n = 0;
    loop {
        match r.fill_buf() {
            Ok([]) => return Ok(n),
            Ok(_) => {}
            Err(e) => return if n > 0 { Ok(n) } else { Err(e) },
        }
        match rmp_serde::from_read::<_, T>(&mut r) {
            Ok(rec) => {
                f(rec);
                n += 1;
            }
            Err(e) => return if n > 0 { Ok(n) } else { Err(std::io::Error::other(e)) },
        }
    }
}

/// Calls `f` with each record of a segment as JSON. Map keys that are not
/// text (an id, a pair of ids) become their JSON text.
pub fn each_json(path: &Path, mut f: impl FnMut(Value)) -> std::io::Result<u64> {
    each::<Any>(path, |a| f(a.0))
}

/// Any MessagePack value, as JSON.
pub struct Any(pub Value);

impl<'de> Deserialize<'de> for Any {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Any, D::Error> {
        d.deserialize_any(AnyVisitor).map(Any)
    }
}

struct AnyVisitor;

impl<'de> Visitor<'de> for AnyVisitor {
    type Value = Value;
    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("any value")
    }
    fn visit_bool<E>(self, v: bool) -> Result<Value, E> {
        Ok(Value::Bool(v))
    }
    fn visit_i64<E>(self, v: i64) -> Result<Value, E> {
        Ok(Value::from(v))
    }
    fn visit_u64<E>(self, v: u64) -> Result<Value, E> {
        Ok(Value::from(v))
    }
    fn visit_f32<E>(self, v: f32) -> Result<Value, E> {
        // An f32 shown with the digits it really has.
        Ok(v.to_string().parse::<f64>().map_or(Value::Null, Value::from))
    }
    fn visit_f64<E>(self, v: f64) -> Result<Value, E> {
        Ok(Value::from(v))
    }
    fn visit_str<E>(self, v: &str) -> Result<Value, E> {
        Ok(Value::String(v.to_string()))
    }
    fn visit_string<E>(self, v: String) -> Result<Value, E> {
        Ok(Value::String(v))
    }
    fn visit_bytes<E>(self, v: &[u8]) -> Result<Value, E> {
        Ok(Value::Array(v.iter().map(|&b| Value::from(b)).collect()))
    }
    fn visit_none<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Value, D::Error> {
        d.deserialize_any(AnyVisitor)
    }
    fn visit_newtype_struct<D: Deserializer<'de>>(self, d: D) -> Result<Value, D::Error> {
        d.deserialize_any(AnyVisitor)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut s: A) -> Result<Value, A::Error> {
        let mut v = Vec::new();
        while let Some(Any(x)) = s.next_element()? {
            v.push(x);
        }
        Ok(Value::Array(v))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut m: A) -> Result<Value, A::Error> {
        let mut out = Map::new();
        while let Some((Any(k), Any(v))) = m.next_entry()? {
            let k = match k {
                Value::String(s) => s,
                other => other.to_string(),
            };
            out.insert(k, v);
        }
        Ok(Value::Object(out))
    }
}
