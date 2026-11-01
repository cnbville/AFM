//! Values and rows as short text (pages are in `view.rs`).

use std::fmt::Write;

use rusqlite::types::Value as S;

use crate::book::Book;
use crate::marks;

/// A value as short text.
pub fn cell(v: &S) -> String {
    match v {
        S::Null => String::new(),
        S::Integer(i) => i.to_string(),
        S::Real(f) => marks::num(*f),
        S::Text(t) => {
            if t.chars().count() > 80 {
                let s: String = t.chars().take(77).collect();
                format!("{s}...")
            } else {
                t.clone()
            }
        }
        S::Blob(b) => format!("<{} bytes>", b.len()),
    }
}

/// Rows as an aligned table.
pub fn table(names: &[String], rows: &[Vec<S>]) -> String {
    let cells: Vec<Vec<String>> = rows.iter().map(|r| r.iter().map(cell).collect()).collect();
    let mut w: Vec<usize> = names.iter().map(|n| n.chars().count()).collect();
    for r in &cells {
        for (i, c) in r.iter().enumerate() {
            w[i] = w[i].max(c.chars().count());
        }
    }
    let mut out = String::new();
    let line = |vals: &[String], out: &mut String| {
        let parts: Vec<String> = vals.iter().enumerate().map(|(i, v)| format!("{v:<width$}", width = w[i])).collect();
        let _ = writeln!(out, "{}", parts.join("  ").trim_end());
    };
    line(names, &mut out);
    for r in &cells {
        line(r, &mut out);
    }
    out
}

/// Runs SQL and prints its rows (at most `limit`).
pub fn sql(b: &Book, q: &str, limit: usize) -> String {
    match b.rows(q) {
        Ok((names, rows)) => {
            let n = rows.len();
            let mut out = table(&names, &rows[..n.min(limit)]);
            if n > limit {
                let _ = writeln!(out, "... {} more rows (limit={limit})", n - limit);
            }
            out
        }
        Err(e) => format!("SQL error: {e}\n"),
    }
}
