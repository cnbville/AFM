//! Writing the book: any record (a MessagePack value) into SQLite tables.
//!
//! Two ways a record lands:
//! - **Spread** (`put`): every scalar a column, nested structs flattened
//!   with `__` between the names (`player__position`), lists of records
//!   into their own child table (`people__player__history`, with `_parent`
//!   the parent's `_row` and `_i` its place), maps keyed by ids into a
//!   child table with `_key`. Lists of plain numbers stay JSON text.
//! - **Packed** (`put_packed`): the record's top-level scalars and a few
//!   named fields as columns, the whole record in `_more` as MessagePack,
//!   zstd-compressed with a dictionary trained on the table's first rows.
//!   For the log's big yearly rows and the plays (`Book::record` and the
//!   SQL function `more(table, _more)` read them back).
//!
//! Columns appear as records bring them (`ALTER TABLE ADD COLUMN`); a value
//! is stored with the type it has. Every table has `_row`, its row id.

use std::collections::HashMap;

use rmpv::Value as V;
use rusqlite::Connection;
use rusqlite::types::Value as S;

use crate::json;

/// Columns a table may have before the rest of a record goes into `_extra`.
const MAX_COLS: usize = 1500;
/// Child tables below a store before the rest is kept as JSON text.
const MAX_DEPTH: usize = 4;
/// Struct levels flattened into one row's columns.
const MAX_FLAT: usize = 5;
/// Rows a packed table keeps to train its dictionary.
const SAMPLES: usize = 3000;
/// Bytes a packed table keeps to train its dictionary (a whole game's
/// plays is a big record: training time and memory grow with the bytes).
const SAMPLE_BYTES: usize = 24 << 20;
/// Rows between commits.
const COMMIT_EVERY: u64 = 400_000;

pub struct Table {
    pub cols: Vec<String>,
    index: HashMap<String, usize>,
    pub rows: u64,
    sql: String,
    pub source: String,
    pub parent: Option<String>,
    packed: Option<Packed>,
}

/// A row's columns by name.
type Row = Vec<(String, S)>;

struct Packed {
    keys: Vec<String>,
    buffer: Vec<(Row, Vec<u8>)>,
    zip: Option<zstd::bulk::Compressor<'static>>,
    trained: bool,
    bytes: usize,
}

pub struct Writer {
    pub conn: Connection,
    pub tables: HashMap<String, Table>,
    since_commit: u64,
    /// Where the records being written come from (`world file`, `game log`...).
    pub source: String,
}

fn q(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

impl Writer {
    pub fn create(path: &std::path::Path) -> rusqlite::Result<Writer> {
        let _ = std::fs::remove_file(path);
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA page_size=16384; PRAGMA journal_mode=OFF; PRAGMA synchronous=OFF;
             PRAGMA locking_mode=EXCLUSIVE; PRAGMA temp_store=MEMORY; PRAGMA cache_size=-262144;
             CREATE TABLE dicts(tbl TEXT PRIMARY KEY, dict BLOB);
             BEGIN;",
        )?;
        conn.set_prepared_statement_cache_capacity(512);
        Ok(Writer { conn, tables: HashMap::new(), since_commit: 0, source: String::new() })
    }

    fn table(&mut self, name: &str, parent: Option<&str>) -> rusqlite::Result<&mut Table> {
        if !self.tables.contains_key(name) {
            self.conn.execute_batch(&format!("CREATE TABLE {}(_row INTEGER PRIMARY KEY)", q(name)))?;
            self.tables.insert(
                name.to_string(),
                Table {
                    cols: vec!["_row".into()],
                    index: HashMap::from([("_row".to_string(), 0)]),
                    rows: 0,
                    sql: String::new(),
                    source: self.source.clone(),
                    parent: parent.map(str::to_string),
                    packed: None,
                },
            );
        }
        Ok(self.tables.get_mut(name).unwrap())
    }

    /// Inserts one row of columns; returns its `_row`.
    pub fn insert(&mut self, name: &str, parent: Option<&str>, cols: Vec<(String, S)>) -> rusqlite::Result<i64> {
        self.table(name, parent)?;
        let t = self.tables.get_mut(name).unwrap();
        let mut extra: Vec<(String, S)> = Vec::new();
        let mut changed = t.sql.is_empty();
        for (c, _) in &cols {
            if t.index.contains_key(c) {
                continue;
            }
            if t.cols.len() >= MAX_COLS {
                if !t.index.contains_key("_extra") {
                    self.conn.execute_batch(&format!("ALTER TABLE {} ADD COLUMN _extra", q(name)))?;
                    t.index.insert("_extra".into(), t.cols.len());
                    t.cols.push("_extra".into());
                    changed = true;
                }
                continue;
            }
            self.conn.execute_batch(&format!("ALTER TABLE {} ADD COLUMN {}", q(name), q(c)))?;
            t.index.insert(c.clone(), t.cols.len());
            t.cols.push(c.clone());
            changed = true;
        }
        if changed {
            let names: Vec<String> = t.cols.iter().map(|c| q(c)).collect();
            let marks: Vec<String> = (1..=t.cols.len()).map(|i| format!("?{i}")).collect();
            t.sql = format!("INSERT INTO {}({}) VALUES({})", q(name), names.join(","), marks.join(","));
        }
        let mut vals = vec![S::Null; t.cols.len()];
        for (c, v) in cols {
            match t.index.get(&c) {
                Some(&i) => vals[i] = v,
                None => extra.push((c, v)),
            }
        }
        if !extra.is_empty() {
            let mut o = serde_json::Map::new();
            for (c, v) in extra {
                o.insert(c, sql_json(&v));
            }
            vals[t.index["_extra"]] = S::Text(serde_json::Value::Object(o).to_string());
        }
        t.rows += 1;
        let sql = t.sql.clone();
        self.conn.prepare_cached(&sql)?.execute(rusqlite::params_from_iter(vals.iter()))?;
        let row = self.conn.last_insert_rowid();
        self.since_commit += 1;
        if self.since_commit >= COMMIT_EVERY {
            self.conn.execute_batch("COMMIT; BEGIN;")?;
            self.since_commit = 0;
        }
        Ok(row)
    }

    /// Spreads one record into `table` (and its child tables); `fixed`
    /// columns go first (`_row`, `_parent`, `_i`, a season). Returns `_row`.
    pub fn put(&mut self, table: &str, fixed: Vec<(String, S)>, v: &V) -> rusqlite::Result<i64> {
        self.put_at(table, None, fixed, v, depth_of(table))
    }

    fn put_at(
        &mut self,
        table: &str,
        parent: Option<&str>,
        fixed: Vec<(String, S)>,
        v: &V,
        depth: usize,
    ) -> rusqlite::Result<i64> {
        let mut cols = fixed;
        let mut kids: Vec<(String, &V)> = Vec::new();
        match v {
            V::Map(m) if !dynamic(m) => flatten(m, "", 0, depth, &mut cols, &mut kids),
            V::Map(_) => kids.push(("entries".into(), v)),
            V::Array(a) => {
                for (i, x) in a.iter().enumerate() {
                    let name = format!("_{i}");
                    match x {
                        V::Map(m) if !dynamic(m) => {
                            flatten(m, &format!("{name}__"), 1, depth, &mut cols, &mut kids);
                        }
                        V::Map(_) | V::Array(_) => cols.push((name, S::Text(json::to_json(x)))),
                        _ => {
                            if let Some(s) = scalar(x) {
                                cols.push((name, s));
                            }
                        }
                    }
                }
            }
            other => {
                if let Some(s) = scalar(other) {
                    cols.push(("_v".into(), s));
                }
            }
        }
        let row = self.insert(table, parent, cols)?;
        for (name, kid) in kids {
            let child = format!("{table}__{name}");
            self.children(&child, table, row, kid, depth + 1)?;
        }
        Ok(row)
    }

    /// The rows of a list or id-keyed map under one parent row.
    pub fn children(&mut self, child: &str, parent: &str, row: i64, kid: &V, depth: usize) -> rusqlite::Result<()> {
        match kid {
            V::Array(a) => {
                for (i, x) in a.iter().enumerate() {
                    let base = vec![("_parent".to_string(), S::Integer(row)), ("_i".to_string(), S::Integer(i as i64))];
                    match x {
                        V::Array(inner) if !inner.is_empty() && inner.iter().all(|e| matches!(e, V::Map(_))) => {
                            for (j, e) in inner.iter().enumerate() {
                                let mut f = base.clone();
                                f.push(("_j".into(), S::Integer(j as i64)));
                                self.put_at(child, Some(parent), f, e, depth)?;
                            }
                        }
                        _ => {
                            self.put_at(child, Some(parent), base, x, depth)?;
                        }
                    }
                }
            }
            V::Map(m) => {
                for (k, x) in m {
                    let base = vec![("_parent".to_string(), S::Integer(row)), ("_key".to_string(), key_sql(k))];
                    self.put_at(child, Some(parent), base, x, depth)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Marks a table as packed: `keys` are dotted paths pulled out as
    /// columns besides the record's top-level scalars.
    pub fn packed(&mut self, table: &str, keys: &[&str]) -> rusqlite::Result<()> {
        let t = self.table(table, None)?;
        if t.packed.is_none() {
            t.packed = Some(Packed {
                keys: keys.iter().map(|k| k.to_string()).collect(),
                buffer: Vec::new(),
                zip: None,
                trained: false,
                bytes: 0,
            });
        }
        Ok(())
    }

    /// One packed record (see `packed`).
    pub fn put_packed(&mut self, table: &str, fixed: Vec<(String, S)>, v: &V) -> rusqlite::Result<()> {
        if self.tables.get(table).and_then(|t| t.packed.as_ref()).is_none() {
            self.packed(table, &[])?;
        }
        let mut cols = fixed;
        let keys = self.tables[table].packed.as_ref().unwrap().keys.clone();
        if let V::Map(m) = v {
            for (k, x) in m {
                if let (Some(name), Some(s)) = (k.as_str(), scalar(x)) {
                    cols.push((name.to_string(), s));
                }
            }
        }
        for k in &keys {
            if let Some(s) = json::path(v, k).and_then(scalar) {
                cols.push((k.replace('.', "__"), s));
            }
        }
        let mut raw = Vec::new();
        rmpv::encode::write_value(&mut raw, v).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        let p = self.tables.get_mut(table).unwrap().packed.as_mut().unwrap();
        if !p.trained {
            p.bytes += raw.len();
            p.buffer.push((cols, raw));
            if p.buffer.len() >= SAMPLES || p.bytes >= SAMPLE_BYTES {
                self.train(table)?;
            }
            return Ok(());
        }
        let blob = compress(p, &raw);
        cols.push(("_more".into(), S::Blob(blob)));
        self.insert(table, None, cols)?;
        Ok(())
    }

    fn train(&mut self, table: &str) -> rusqlite::Result<()> {
        let p = self.tables.get_mut(table).unwrap().packed.as_mut().unwrap();
        let buffer = std::mem::take(&mut p.buffer);
        p.trained = true;
        let dict = if buffer.len() >= 64 {
            let samples: Vec<&[u8]> = buffer.iter().map(|(_, r)| &r[..]).collect();
            zstd::dict::from_samples(&samples, 112 * 1024).ok()
        } else {
            None
        };
        p.zip = Some(
            match &dict {
                Some(d) => zstd::bulk::Compressor::with_dictionary(3, d),
                None => zstd::bulk::Compressor::new(3),
            }
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?,
        );
        self.conn.execute("INSERT OR REPLACE INTO dicts(tbl, dict) VALUES(?1, ?2)", rusqlite::params![table, dict])?;
        for (mut cols, raw) in buffer {
            let p = self.tables.get_mut(table).unwrap().packed.as_mut().unwrap();
            let blob = compress(p, &raw);
            cols.push(("_more".into(), S::Blob(blob)));
            self.insert(table, None, cols)?;
        }
        Ok(())
    }

    /// Writes what packed tables still hold and commits.
    pub fn flush(&mut self) -> rusqlite::Result<()> {
        let open: Vec<String> = self
            .tables
            .iter()
            .filter(|(_, t)| t.packed.as_ref().is_some_and(|p| !p.trained))
            .map(|(n, _)| n.clone())
            .collect();
        for t in open {
            self.train(&t)?;
        }
        self.conn.execute_batch("COMMIT; BEGIN;")?;
        self.since_commit = 0;
        Ok(())
    }
}

fn compress(p: &mut Packed, raw: &[u8]) -> Vec<u8> {
    p.zip.as_mut().and_then(|z| z.compress(raw).ok()).unwrap_or_default()
}

fn depth_of(table: &str) -> usize {
    table.matches("__").count()
}

/// A map keyed by ids (numbers, pairs) rather than field names.
pub fn dynamic(m: &[(V, V)]) -> bool {
    !m.is_empty()
        && m.iter().all(|(k, _)| match k {
            V::String(s) => s.as_str().is_none_or(|t| {
                t.parse::<i64>().is_ok() || t.starts_with('[') || t.starts_with('{') || t.parse::<f64>().is_ok()
            }),
            _ => true,
        })
}

fn key_sql(k: &V) -> S {
    match k {
        V::Integer(i) => i.as_i64().map_or(S::Text(json::key(k)), S::Integer),
        V::String(s) => {
            let t = s.as_str().unwrap_or("");
            t.parse::<i64>().map_or_else(|_| S::Text(t.to_string()), S::Integer)
        }
        other => S::Text(json::key(other)),
    }
}

/// A scalar as a column value (none for lists and structs).
pub fn scalar(v: &V) -> Option<S> {
    Some(match v {
        V::Nil => return None,
        V::Boolean(b) => S::Integer(*b as i64),
        V::Integer(i) => match i.as_i64() {
            Some(x) => S::Integer(x),
            None => S::Real(i.as_f64()?),
        },
        V::F32(f) if f.is_finite() => S::Real(f.to_string().parse().unwrap_or(*f as f64)),
        V::F64(f) if f.is_finite() => S::Real(*f),
        V::F32(_) | V::F64(_) => return None,
        V::String(s) => S::Text(s.as_str().unwrap_or("").to_string()),
        V::Binary(b) => S::Blob(b.clone()),
        // A unit enum variant written as a one-entry map.
        V::Map(m) if m.len() == 1 && matches!(m[0].1, V::Nil) => S::Text(json::key(&m[0].0)),
        _ => return None,
    })
}

fn flatten<'a>(
    m: &'a [(V, V)],
    prefix: &str,
    level: usize,
    depth: usize,
    cols: &mut Vec<(String, S)>,
    kids: &mut Vec<(String, &'a V)>,
) {
    for (k, x) in m {
        let name = format!("{prefix}{}", json::key(k));
        match x {
            V::Nil => {}
            V::Map(inner) if inner.len() == 1 && matches!(inner[0].1, V::Nil) => {
                cols.push((name, S::Text(json::key(&inner[0].0))));
            }
            V::Map(inner) if inner.is_empty() => {}
            V::Map(inner) if dynamic(inner) => {
                if depth < MAX_DEPTH {
                    kids.push((name, x));
                } else {
                    cols.push((name, S::Text(json::to_json(x))));
                }
            }
            V::Map(inner) => {
                if level < MAX_FLAT {
                    flatten(inner, &format!("{name}__"), level + 1, depth, cols, kids);
                } else {
                    cols.push((name, S::Text(json::to_json(x))));
                }
            }
            V::Array(a) if a.is_empty() => {}
            V::Array(a) => {
                let records = a.iter().any(|e| match e {
                    V::Map(m) => !(m.len() == 1 && matches!(m[0].1, V::Nil)),
                    V::Array(inner) => inner.iter().any(|e| matches!(e, V::Map(_) | V::Array(_))) || inner.len() > 1,
                    _ => false,
                });
                if records && depth < MAX_DEPTH {
                    kids.push((name, x));
                } else {
                    cols.push((name, S::Text(json::to_json(x))));
                }
            }
            other => {
                if let Some(s) = scalar(other) {
                    cols.push((name, s));
                }
            }
        }
    }
}

/// A column value as JSON.
pub fn sql_json(v: &S) -> serde_json::Value {
    match v {
        S::Null => serde_json::Value::Null,
        S::Integer(i) => (*i).into(),
        S::Real(f) => serde_json::Number::from_f64(*f).map_or(serde_json::Value::Null, serde_json::Value::Number),
        S::Text(t) => t.clone().into(),
        S::Blob(b) => format!("<{} bytes>", b.len()).into(),
    }
}
