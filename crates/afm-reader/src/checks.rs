//! Every check against real (world reader inventory, each area's checks):
//! the world's value, the real value with its source and years, the mark
//! and a "what it says" line in plain words. The definitions are in
//! `defs.rs`; each world number is SQL over the book's tables, so a thread
//! can rerun any of them by hand (`afm_read <book> check <id> sql`).

use rusqlite::params;

use crate::book::Book;
use crate::defs::{Check, Real};
use crate::marks::{self, Tol};

/// One check's result.
#[derive(Clone, Debug, Default)]
pub struct Result {
    pub world: Option<f64>,
    pub real: Option<f64>,
    pub mark: String,
    pub source: String,
    pub years: String,
    pub status: String,
    pub says: String,
    pub detail: Option<String>,
    pub series: Vec<(i64, f64)>,
}

pub fn create(b: &Book) -> rusqlite::Result<()> {
    b.conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS checks(id TEXT PRIMARY KEY, page TEXT, areas TEXT, title TEXT, unit TEXT,
            world REAL, real REAL, gap REAL, mark TEXT, tolerance TEXT, real_key TEXT, real_source TEXT,
            real_years TEXT, real_status TEXT, says TEXT, detail TEXT, needs TEXT, how TEXT, issues TEXT,
            sql TEXT, years_sql TEXT);
         CREATE TABLE IF NOT EXISTS check_years(check_id TEXT, year INTEGER, world REAL);",
    )
}

/// Runs one check on a book.
pub fn run(b: &Book, c: &Check, index: &serde_json::Value, have: &dyn Fn(&str) -> bool) -> Result {
    let mut r = Result::default();
    let (real, source, years, status) = real_of(&c.real, index);
    r.real = real;
    r.source = source;
    r.years = years;
    r.status = status;
    if !have(c.needs) {
        r.mark = "no data".into();
        r.says = format!("{}: not in this book; it needs the {}.", c.title, c.needs);
        return r;
    }
    let mut error = None;
    r.world = match b.try_num(c.world) {
        Ok(v) => v.map(|x| x * c.scale),
        Err(e) => {
            error = Some(e);
            None
        }
    };
    if let Some(ys) = c.years
        && let Ok((_, rows)) = b.rows(ys)
    {
        for row in rows {
            let y = match row.first() {
                Some(rusqlite::types::Value::Integer(y)) => *y,
                _ => continue,
            };
            let v = match row.get(1) {
                Some(rusqlite::types::Value::Integer(i)) => *i as f64,
                Some(rusqlite::types::Value::Real(f)) => *f,
                _ => continue,
            };
            r.series.push((y, v * c.scale));
        }
    }
    r.detail = match error {
        Some(e) => Some(format!("Its SQL failed: {e}.")),
        None => c.detail.and_then(|d| b.text_of(d)),
    };
    let tol = Tol { near: c.tol.0, off: c.tol.1, floor: c.tol.2 };
    r.mark = match (r.world, r.real) {
        (None, _) => "no data".into(),
        (Some(_), None) => "no real".into(),
        (Some(w), Some(re)) => marks::mark(w, re, tol).into(),
    };
    r.says = says(c, &r);
    r
}

fn says(c: &Check, r: &Result) -> String {
    let w = r.world.map_or("no value".to_string(), |w| marks::with_unit(w, c.unit));
    let detail = r.detail.as_ref().map_or(String::new(), |d| format!(" {d}"));
    match (r.mark.as_str(), r.real) {
        ("no data", _) => format!("{}: the book has no rows for it.{detail}", c.title),
        ("no real", _) => {
            format!("{}: {w}. No real figure to set it against ({}).{detail}", c.title, r.source)
        }
        (m, Some(re)) => {
            let from = if r.source.is_empty() { String::new() } else { format!(" ({}, {})", r.source, r.years) };
            let memo = if r.status == "from memory" { " The real figure is from memory." } else { "" };
            format!("{} is {m}: {w} against real {}{from}.{detail}{memo}", c.title, marks::with_unit(re, c.unit))
        }
        _ => c.title.to_string(),
    }
}

/// A check's real figure, source, years and status from the index.
pub fn real_of(real: &Real, index: &serde_json::Value) -> (Option<f64>, String, String, String) {
    match real {
        Real::Key(key, scale) => {
            let Some((v, fig)) = crate::real_in::figure(index, key) else {
                return (None, format!("{key} is not in the index"), String::new(), "missing".into());
            };
            let file = fig.get("file").and_then(|f| f.as_str()).unwrap_or("");
            let fileinfo = index.get("files").and_then(|f| f.get(file));
            let pick = |k: &str| {
                fig.get(k)
                    .or_else(|| fileinfo.and_then(|f| f.get(k)))
                    .map(|x| x.as_str().map_or_else(|| x.to_string(), str::to_string))
                    .unwrap_or_default()
            };
            let num = v.as_f64().or_else(|| v.get("value").and_then(|x| x.as_f64()));
            (num.map(|x| x * scale), pick("source"), pick("years"), pick("status"))
        }
        Real::Fixed(v, source, years) => (Some(*v), source.to_string(), years.to_string(), "sourced".into()),
        Real::Memory(v, note) => (Some(*v), note.to_string(), String::new(), "from memory".into()),
        Real::None(why) => (None, why.to_string(), String::new(), "none".into()),
    }
}

/// Runs every check and writes them into the book.
pub fn write_all(
    b: &Book,
    index: &serde_json::Value,
    have: &dyn Fn(&str) -> bool,
    issues: &dyn Fn(&str) -> String,
) -> rusqlite::Result<usize> {
    create(b)?;
    let mut n = 0;
    for c in crate::defs::CHECKS {
        let r = run(b, c, index, have);
        let tol = Tol { near: c.tol.0, off: c.tol.1, floor: c.tol.2 };
        let gap = match (r.world, r.real) {
            (Some(w), Some(re)) => Some(marks::gap(w, re, tol)),
            _ => None,
        };
        let key = match &c.real {
            Real::Key(k, _) => k.to_string(),
            _ => String::new(),
        };
        b.conn.execute(
            "INSERT OR REPLACE INTO checks VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21)",
            params![
                c.id, c.page, c.areas, c.title, c.unit, r.world, r.real, gap, r.mark, marks::words(tol), key, r.source,
                r.years, r.status, r.says, r.detail, c.needs, c.how, issues(c.id), c.world, c.years
            ],
        )?;
        for (y, v) in &r.series {
            b.conn.execute("INSERT INTO check_years VALUES(?1,?2,?3)", params![c.id, y, v])?;
        }
        n += 1;
    }
    Ok(n)
}
