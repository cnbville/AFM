//! The precomputed panels (world reader lanes R6-R8): each inventory
//! area's own tables and charts, worked out once when the book is built so
//! a page opens in under a second on a full-size world. A panel is SQL over
//! the book's tables giving rows with named columns; the app draws it by
//! its `kind`, and `afm_read <book> area <code>` prints it.
//!
//! Column names the app reads, by kind (any other column shows on hover):
//! - `bars`: `label`, `world`, optional `real`, `n`; world against real per row.
//! - `hist`: `x` (the bin), `world`, optional `real`; shares or counts.
//! - `line`: `year`, then one column per series; `real_lo` and `real_hi`
//!   shade the real band, `real` draws the real line.
//! - `matrix`: `row`, `col`, `value`, optional `real`; a heat grid.
//! - `funnel`: `stage`, `n`, optional `share` and `real` (shares of the start).
//! - `map`: `state` (two letters), `world`, optional `real`, `n`.
//! - `table`: anything; an `id` column opens the panel's `open` page.
//!
//! A `grp` column splits a panel into groups the reader picks between (a
//! level, a position); a `year` column lets the year slider pick a year.

use rusqlite::params;
use rusqlite::types::Value as S;

use crate::book::Book;

/// One panel.
pub struct Panel {
    pub id: &'static str,
    /// The topic page (`pages::PAGES`).
    pub page: &'static str,
    /// The inventory area it belongs to.
    pub area: &'static str,
    pub title: &'static str,
    /// bars, hist, line, matrix, funnel, map, table
    pub kind: &'static str,
    /// What it shows, where the real figures come from, what it leaves out.
    pub note: &'static str,
    /// SQL giving the rows.
    pub sql: &'static str,
    /// SQL giving one line of plain words about it (the area's verdict).
    pub says: Option<&'static str>,
    /// What the book needs for it: "world file", "game log" or "world log".
    pub needs: &'static str,
    /// The page a row opens, with `{col}` filled from the row
    /// (`e/people/{id}`, `class/{class}`, `state/{state}`); empty for none.
    pub open: &'static str,
}

/// Rows kept per panel: a top list or a full spread, never millions.
pub const MAX_ROWS: usize = 5000;

pub fn create(b: &Book) -> rusqlite::Result<()> {
    b.conn.execute_batch(
        "DROP TABLE IF EXISTS panels;
         CREATE TABLE panels(id TEXT PRIMARY KEY, page TEXT, area TEXT, title TEXT, kind TEXT, note TEXT,
            needs TEXT, open TEXT, cols TEXT, rows TEXT, n INTEGER, more INTEGER, says TEXT, status TEXT,
            sql TEXT, seconds REAL);",
    )
}

fn json_of(v: &S) -> serde_json::Value {
    match v {
        S::Null => serde_json::Value::Null,
        S::Integer(i) => (*i).into(),
        // Seven significant digits, as the book keeps floats.
        S::Real(x) => {
            let r = if *x == 0.0 || !x.is_finite() {
                *x
            } else {
                let m = 10f64.powi(6 - x.abs().log10().floor() as i32);
                (x * m).round() / m
            };
            serde_json::Number::from_f64(r).map_or(serde_json::Value::Null, serde_json::Value::Number)
        }
        S::Text(t) => t.clone().into(),
        S::Blob(_) => serde_json::Value::Null,
    }
}

/// Runs one panel: its columns, rows (at most `MAX_ROWS`), how many more,
/// its line of words and its status.
pub fn run(b: &Book, p: &Panel) -> (Vec<String>, Vec<Vec<serde_json::Value>>, usize, Option<String>, String) {
    let mut st = match b.conn.prepare(p.sql) {
        Ok(st) => st,
        // A child table that a small or early world never filled is not
        // in its book: no data, not a broken panel.
        Err(e) if e.to_string().contains("no such table") => {
            return (Vec::new(), Vec::new(), 0, None, format!("no data: {e}"));
        }
        Err(e) => return (Vec::new(), Vec::new(), 0, None, format!("SQL error: {e}")),
    };
    let cols: Vec<String> = st.column_names().iter().map(|c| c.to_string()).collect();
    let n = cols.len();
    let mut rows = Vec::new();
    let mut more = 0;
    let res = st.query([]).and_then(|mut it| {
        while let Some(r) = it.next()? {
            if rows.len() >= MAX_ROWS {
                more += 1;
                continue;
            }
            let mut row = Vec::with_capacity(n);
            for i in 0..n {
                row.push(json_of(&r.get::<_, S>(i)?));
            }
            rows.push(row);
        }
        Ok(())
    });
    if let Err(e) = res {
        return (cols, Vec::new(), 0, None, format!("SQL error: {e}"));
    }
    let says = p.says.and_then(|s| b.text_of(s));
    let status = if rows.is_empty() { "no rows" } else { "ok" };
    (cols, rows, more, says, status.into())
}

/// Small lookup tables the panels join: a state's two letters by the
/// engine's index (the row order of `hs_states.csv`), and a season row's
/// event bits by name (`player::DevelopmentEvent`, in order).
const LOOKUPS: &str = "
    DROP TABLE IF EXISTS state_codes;
    CREATE TABLE state_codes(idx INTEGER PRIMARY KEY, code TEXT);
    INSERT INTO state_codes SELECT rowid - 1, state FROM real_hs_states ORDER BY rowid;
    DROP TABLE IF EXISTS event_bits;
    CREATE TABLE event_bits(bit INTEGER PRIMARY KEY, name TEXT, words TEXT);
    INSERT INTO event_bits VALUES (0, 'GrowthSpurt', 'a growth spurt'), (1, 'BodyStalled', 'his body stalled'),
        (2, 'Breakout', 'a breakout'), (3, 'Stagnation', 'stagnation'), (4, 'LostSeason', 'a lost season'),
        (5, 'Suspended', 'suspended or ineligible'), (6, 'Dismissed', 'dismissed'), (7, 'Slump', 'a slump'),
        (8, 'MinorInjury', 'a minor injury'), (9, 'ModerateInjury', 'a moderate injury'),
        (10, 'MajorInjury', 'a major injury'), (11, 'FirstYear', 'the first-year dip'),
        (12, 'SpringBreakout', 'a spring breakout'), (13, 'LostBurst', 'lost burst for good'),
        (14, 'WeightGain', 'bad weight'), (15, 'Complacent', 'complacency after a payday'),
        (16, 'PoorCoaching', 'poor coaching');";

/// Every panel into the book.
pub fn write_all(b: &Book, have: &dyn Fn(&str) -> bool) -> rusqlite::Result<usize> {
    if b.has("real_hs_states") {
        b.conn.execute_batch(LOOKUPS)?;
    }
    // Most people panels read one stage (college and pro players are a tenth
    // of the table): an index on it turns their full scans into lookups.
    if b.columns("people").iter().any(|c| c == "stage") {
        b.conn.execute_batch("CREATE INDEX IF NOT EXISTS ix_people_stage ON people(stage)")?;
    }
    create(b)?;
    let mut n = 0;
    for p in all() {
        let t = std::time::Instant::now();
        let (cols, rows, more, says, status) = if have(p.needs) {
            run(b, p)
        } else {
            (Vec::new(), Vec::new(), 0, None, format!("not in this book: needs the {}", p.needs))
        };
        b.conn.execute(
            "INSERT OR REPLACE INTO panels VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
            params![
                p.id,
                p.page,
                p.area,
                p.title,
                p.kind,
                p.note,
                p.needs,
                p.open,
                serde_json::to_string(&cols).unwrap_or_default(),
                serde_json::to_string(&rows).unwrap_or_default(),
                rows.len() as i64,
                more as i64,
                says,
                status,
                p.sql,
                t.elapsed().as_secs_f64()
            ],
        )?;
        n += 1;
    }
    Ok(n)
}

/// Every panel the reader knows, page by page.
pub fn all() -> impl Iterator<Item = &'static Panel> {
    crate::defs_people::PANELS.iter().chain(crate::defs_programs::PANELS)
}
