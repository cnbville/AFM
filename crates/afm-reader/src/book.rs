//! Reading a book: open it, run SQL on it, read a packed record back.
//!
//! Every book connection has the SQL function `more(table, _more)`: a
//! packed row's whole record as JSON text, so a query can reach anything
//! in it with `json_extract(more('log_players', _more), '$.person.mood')`.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::functions::FunctionFlags;
use rusqlite::types::{Value as S, ValueRef};
use rusqlite::{Connection, OpenFlags};

use crate::json;

pub struct Book {
    pub conn: Connection,
    dicts: Arc<Mutex<HashMap<String, Option<Vec<u8>>>>>,
}

impl Book {
    pub fn open(path: &Path) -> rusqlite::Result<Book> {
        let conn =
            Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
        Book::with(conn)
    }

    /// A book connection (also the one the builder writes through).
    pub fn with(conn: Connection) -> rusqlite::Result<Book> {
        let dicts: Arc<Mutex<HashMap<String, Option<Vec<u8>>>>> = Arc::default();
        conn.execute_batch("PRAGMA cache_size=-131072; PRAGMA temp_store=MEMORY;")?;
        let d = dicts.clone();
        // `more` reads the dictionaries through its own handle on the file.
        let path = conn.path().map(str::to_string);
        conn.create_scalar_function(
            "more",
            2,
            FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
            move |ctx| {
                let table: String = ctx.get(0)?;
                let blob = match ctx.get_raw(1) {
                    ValueRef::Blob(b) => b.to_vec(),
                    _ => return Ok(None),
                };
                let dict = {
                    let mut m = d.lock().unwrap();
                    if !m.contains_key(&table) {
                        let got = path.as_deref().and_then(|p| dict_of(p, &table));
                        m.insert(table.clone(), got);
                    }
                    m[&table].clone()
                };
                Ok(unpack(&blob, dict.as_deref()).map(|v| json::to_json(&v)))
            },
        )?;
        // Older SQLite builds lack the math functions the checks use.
        for (name, f) in [("sqrt", f64::sqrt as fn(f64) -> f64), ("ln", f64::ln), ("exp", f64::exp)] {
            conn.create_scalar_function(
                name,
                1,
                FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
                move |ctx| Ok(ctx.get::<Option<f64>>(0)?.map(f).filter(|x| x.is_finite())),
            )?;
        }
        Ok(Book { conn, dicts })
    }

    /// The tables, from the catalog when the book has one.
    pub fn tables(&self) -> Vec<String> {
        let mut st = match self.conn.prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name") {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        st.query_map([], |r| r.get::<_, String>(0)).map(|it| it.flatten().collect()).unwrap_or_default()
    }

    pub fn has(&self, table: &str) -> bool {
        self.conn.query_row("SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1", [table], |_| Ok(())).is_ok()
    }

    pub fn columns(&self, table: &str) -> Vec<String> {
        let sql = format!("PRAGMA table_info(\"{}\")", table.replace('"', ""));
        let Ok(mut st) = self.conn.prepare(&sql) else { return Vec::new() };
        st.query_map([], |r| r.get::<_, String>(1)).map(|it| it.flatten().collect()).unwrap_or_default()
    }

    /// Runs a query; each row as column values.
    pub fn rows(&self, sql: &str) -> rusqlite::Result<(Vec<String>, Vec<Vec<S>>)> {
        let mut st = self.conn.prepare(sql)?;
        let names: Vec<String> = st.column_names().iter().map(|s| s.to_string()).collect();
        let n = names.len();
        let rows = st
            .query_map([], |r| (0..n).map(|i| r.get::<_, S>(i)).collect::<rusqlite::Result<Vec<S>>>())?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok((names, rows))
    }

    /// Runs a query and hands each row to `f`, one at a time.
    pub fn each(&self, sql: &str, mut f: impl FnMut(&rusqlite::Row) -> rusqlite::Result<()>) -> rusqlite::Result<()> {
        let mut st = self.conn.prepare(sql)?;
        let mut rs = st.query([])?;
        while let Some(r) = rs.next()? {
            f(r)?;
        }
        Ok(())
    }

    /// One number from a query, or why the query failed.
    pub fn try_num(&self, sql: &str) -> Result<Option<f64>, String> {
        match self.conn.query_row(sql, [], |r| r.get::<_, S>(0)) {
            Ok(S::Integer(i)) => Ok(Some(i as f64)),
            Ok(S::Real(f)) => Ok(Some(f)),
            Ok(S::Text(t)) => Ok(t.parse().ok()),
            Ok(_) | Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    /// One number from a query (none when it has no rows or no number).
    pub fn num(&self, sql: &str) -> Option<f64> {
        self.conn.query_row(sql, [], |r| r.get::<_, S>(0)).ok().and_then(|v| match v {
            S::Integer(i) => Some(i as f64),
            S::Real(f) => Some(f),
            S::Text(t) => t.parse().ok(),
            _ => None,
        })
    }

    pub fn text_of(&self, sql: &str) -> Option<String> {
        self.conn.query_row(sql, [], |r| r.get::<_, S>(0)).ok().and_then(|v| match v {
            S::Text(t) => Some(t),
            S::Integer(i) => Some(i.to_string()),
            S::Real(f) => Some(f.to_string()),
            _ => None,
        })
    }

    /// A book's own fact (`book_meta`).
    pub fn meta(&self, key: &str) -> Option<String> {
        self.conn.query_row("SELECT value FROM book_meta WHERE key=?1", [key], |r| r.get(0)).ok()
    }

    /// A row as JSON: its columns, its packed record merged in, and its
    /// child tables' rows under their names.
    pub fn record(&self, table: &str, row: i64) -> Option<serde_json::Value> {
        let sql = format!("SELECT * FROM \"{}\" WHERE _row={row}", table.replace('"', ""));
        let (names, rows) = self.rows(&sql).ok()?;
        let vals = rows.into_iter().next()?;
        let mut o = serde_json::Map::new();
        for (n, v) in names.iter().zip(&vals) {
            if n == "_more" {
                if let S::Blob(b) = v {
                    let dict = self.dict(table);
                    if let Some(rec) = unpack(b, dict.as_deref()) {
                        o.insert("record".into(), json::to_value(&rec));
                    }
                }
                continue;
            }
            if !matches!(v, S::Null) {
                o.insert(n.clone(), crate::sink::sql_json(v));
            }
        }
        for child in self
            .tables()
            .into_iter()
            .filter(|t| t.strip_prefix(table).is_some_and(|r| r.starts_with("__") && !r[2..].contains("__")))
        {
            let sql = format!("SELECT _row FROM \"{child}\" WHERE _parent={row} ORDER BY _row LIMIT 500");
            if let Ok((_, kids)) = self.rows(&sql) {
                let list: Vec<serde_json::Value> = kids
                    .iter()
                    .filter_map(|k| match k[0] {
                        S::Integer(r) => self.record(&child, r),
                        _ => None,
                    })
                    .collect();
                if !list.is_empty() {
                    o.insert(child[table.len() + 2..].to_string(), serde_json::Value::Array(list));
                }
            }
        }
        Some(serde_json::Value::Object(o))
    }

    fn dict(&self, table: &str) -> Option<Vec<u8>> {
        let mut m = self.dicts.lock().unwrap();
        m.entry(table.to_string())
            .or_insert_with(|| {
                self.conn
                    .query_row("SELECT dict FROM dicts WHERE tbl=?1", [table], |r| r.get::<_, Option<Vec<u8>>>(0))
                    .ok()
                    .flatten()
            })
            .clone()
    }
}

fn dict_of(path: &str, table: &str) -> Option<Vec<u8>> {
    if path.is_empty() {
        return None;
    }
    let c =
        Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX).ok()?;
    c.query_row("SELECT dict FROM dicts WHERE tbl=?1", [table], |r| r.get::<_, Option<Vec<u8>>>(0)).ok().flatten()
}

/// A packed record back as a MessagePack value.
pub fn unpack(blob: &[u8], dict: Option<&[u8]>) -> Option<rmpv::Value> {
    let raw = match dict {
        Some(d) => {
            let mut dec = zstd::bulk::Decompressor::with_dictionary(d).ok()?;
            dec.decompress(blob, 64 << 20).ok()?
        }
        None => zstd::bulk::decompress(blob, 64 << 20).ok()?,
    };
    rmpv::decode::read_value(&mut &raw[..]).ok()
}
