//! The precomputed distributions: every metric in `defs_list::METRICS` by
//! year and by group (level, division, conference, position, stars, state,
//! era, age), with percentiles, and a histogram of each group over all
//! years.

use std::collections::BTreeMap;

use rusqlite::params;
use rusqlite::types::Value as S;

use crate::book::Book;
use crate::stats;

pub fn create(b: &Book) -> rusqlite::Result<()> {
    b.conn.execute_batch(
        "CREATE TABLE dist(metric TEXT, page TEXT, areas TEXT, title TEXT, unit TEXT, year INTEGER, grp_kind TEXT, grp TEXT,
            n INTEGER, mean REAL, sd REAL, min REAL, p1 REAL, p5 REAL, p10 REAL, p25 REAL, p50 REAL, p75 REAL, p90 REAL,
            p95 REAL, p99 REAL, max REAL);
         CREATE TABLE hist(metric TEXT, grp_kind TEXT, grp TEXT, lo REAL, hi REAL, n INTEGER);
         CREATE TABLE metrics(id TEXT PRIMARY KEY, page TEXT, areas TEXT, title TEXT, unit TEXT, needs TEXT, rows INTEGER, sql TEXT, note TEXT);",
    )
}

fn f(v: &S) -> Option<f64> {
    match v {
        S::Integer(i) => Some(*i as f64),
        S::Real(x) => Some(*x),
        S::Text(t) => t.parse().ok(),
        _ => None,
    }
}

fn t(v: &S) -> String {
    match v {
        S::Integer(i) => i.to_string(),
        S::Real(x) => x.to_string(),
        S::Text(s) => s.clone(),
        _ => String::new(),
    }
}

pub fn write_all(b: &Book, have: &dyn Fn(&str) -> bool) -> rusqlite::Result<()> {
    create(b)?;
    for m in crate::defs::metrics() {
        if !have(m.needs) {
            b.conn.execute(
                "INSERT INTO metrics VALUES(?1,?2,?3,?4,?5,?6,0,?7,?8)",
                params![
                    m.id,
                    m.page,
                    m.areas,
                    m.title,
                    m.unit,
                    m.needs,
                    m.sql,
                    format!("not in this book: needs the {}", m.needs)
                ],
            )?;
            continue;
        }
        // (year or none, group kind, group) -> values; "all" groups too.
        // Rows are read one at a time: a metric over every person is millions.
        let mut groups: BTreeMap<(Option<i64>, String, String), Vec<f64>> = BTreeMap::new();
        let mut n_rows = 0i64;
        let mut first_kind: Option<String> = None;
        let read = (|| -> rusqlite::Result<()> {
            let mut st = b.conn.prepare(m.sql)?;
            let mut rs = st.query([])?;
            while let Some(r) = rs.next()? {
                let Some(v) = f(&r.get::<_, S>(3)?) else { continue };
                n_rows += 1;
                let year = match r.get::<_, S>(0)? {
                    S::Integer(y) => Some(y),
                    S::Real(y) => Some(y as i64),
                    _ => None,
                };
                let (gk, g) = (t(&r.get::<_, S>(1)?), t(&r.get::<_, S>(2)?));
                let gk = if gk.is_empty() { "all".to_string() } else { gk };
                let g = if g.is_empty() { "all".to_string() } else { g };
                groups.entry((year, gk.clone(), g.clone())).or_default().push(v);
                if year.is_some() {
                    groups.entry((None, gk.clone(), g.clone())).or_default().push(v);
                }
                // The "all" group counts each thing once: from the first
                // group kind only (a metric may list it by several kinds).
                let first = first_kind.get_or_insert_with(|| gk.clone());
                if gk != "all" && *first == gk {
                    groups.entry((year, "all".into(), "all".into())).or_default().push(v);
                    if year.is_some() {
                        groups.entry((None, "all".into(), "all".into())).or_default().push(v);
                    }
                }
            }
            Ok(())
        })();
        let note = read.err().map(|e| format!("SQL error: {e}")).unwrap_or_default();
        let mut st = b.conn.prepare_cached(
            "INSERT INTO dist VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22)",
        )?;
        for ((year, gk, g), mut xs) in groups {
            let d = stats::dist(&mut xs);
            st.execute(params![
                m.id, m.page, m.areas, m.title, m.unit, year, gk, g, d.n as i64, d.mean, d.sd, d.min, d.p[0], d.p[1],
                d.p[2], d.p[3], d.p[4], d.p[5], d.p[6], d.p[7], d.p[8], d.max
            ])?;
            if year.is_none() {
                for (lo, hi, n) in stats::histogram(&xs, 20) {
                    b.conn.execute(
                        "INSERT INTO hist VALUES(?1,?2,?3,?4,?5,?6)",
                        params![m.id, gk, g, lo, hi, n as i64],
                    )?;
                }
            }
        }
        b.conn.execute(
            "INSERT INTO metrics VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![m.id, m.page, m.areas, m.title, m.unit, m.needs, n_rows, m.sql, note],
        )?;
    }
    Ok(())
}
