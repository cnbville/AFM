//! The knob table (inventory T2): every tuning constant the how-it-works
//! pages name, its value as written, its file and whether it is confirmed,
//! read when the reader was built (`build.rs`). A world built at another
//! commit may have had other values: `book_meta` keeps both commits, and
//! the world log's `meta.json` keeps the class pipeline's knobs as built.

use rusqlite::params;

use crate::book::Book;

const KNOBS: &str = include_str!(concat!(env!("OUT_DIR"), "/knobs.json"));

fn doc() -> serde_json::Value {
    serde_json::from_str(KNOBS).unwrap_or_default()
}

/// The commit the reader was built at.
pub fn commit() -> String {
    doc().get("commit").and_then(|c| c.as_str()).unwrap_or("").to_string()
}

pub fn write(b: &Book) -> rusqlite::Result<usize> {
    b.conn.execute_batch(
        "CREATE TABLE knobs(name TEXT, value TEXT, file TEXT, system TEXT, status TEXT, note TEXT, read_at TEXT);",
    )?;
    let d = doc();
    let at = commit();
    let mut n = 0;
    for k in d.get("knobs").and_then(|k| k.as_array()).into_iter().flatten() {
        let s = |f: &str| k.get(f).and_then(|x| x.as_str()).unwrap_or("").to_string();
        b.conn.execute(
            "INSERT INTO knobs VALUES(?1,?2,?3,?4,?5,?6,?7)",
            params![s("name"), s("value"), s("file"), s("system"), s("status"), s("note"), at],
        )?;
        n += 1;
    }
    Ok(n)
}
