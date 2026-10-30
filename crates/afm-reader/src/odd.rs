//! The odd-things list, at three levels (spec section 6):
//! - league: checks off or far off against real;
//! - one: a consistency check that fails (with the facts) or a rare
//!   outlier (with its percentile against the world);
//! - data: what the book doesn't count (a missing log) and the known
//!   issues still live in this world.
//!
//! Each row names its page, the record it is about and the facts.

use rusqlite::params;
use rusqlite::types::Value as S;

use crate::book::Book;
use crate::issues::Issues;
use crate::text::cell;

pub fn create(b: &Book) -> rusqlite::Result<()> {
    b.conn.execute_batch(
        "CREATE TABLE odd(level TEXT, kind TEXT, page TEXT, source TEXT, what TEXT, entity_id TEXT, year INTEGER,
            says TEXT, facts TEXT, pct REAL);
         CREATE TABLE consistency(id TEXT PRIMARY KEY, areas TEXT, title TEXT, status TEXT, checked INTEGER,
            failures INTEGER, why TEXT, needs TEXT, sql TEXT);",
    )
}

/// Failures and outliers kept per check.
const KEEP: usize = 200;

pub fn write_all(b: &Book, have: &dyn Fn(&str) -> bool, issues: &Issues) -> rusqlite::Result<()> {
    create(b)?;
    let ins = |level: &str,
               kind: &str,
               page: &str,
               source: &str,
               what: &str,
               id: &str,
               year: Option<i64>,
               says: &str,
               facts: &str,
               pct: Option<f64>| {
        b.conn.execute(
            "INSERT INTO odd VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![level, kind, page, source, what, id, year, says, facts, pct],
        )
    };
    // League: checks away from real.
    if let Ok((_, rows)) = b.rows("SELECT id, page, mark, says, world, real, issues FROM checks WHERE mark IN ('far off','off') ORDER BY mark = 'far off' DESC, gap DESC") {
        for r in rows {
            let issues = cell(&r[6]);
            let facts = if issues.is_empty() { String::new() } else { format!("known issues: {issues}") };
            ins("league", &cell(&r[2]), &cell(&r[1]), &cell(&r[0]), "check", &cell(&r[0]), None, &cell(&r[3]), &facts, None)?;
        }
    }
    // One record: consistency checks.
    for rule in crate::defs::rules() {
        if !have(rule.needs) {
            b.conn.execute(
                "INSERT INTO consistency VALUES(?1,?2,?3,'no data',0,0,?4,?5,?6)",
                params![
                    rule.id,
                    rule.areas,
                    rule.title,
                    format!("not in this book: needs the {}", rule.needs),
                    rule.needs,
                    rule.failures
                ],
            )?;
            continue;
        }
        let checked = b.num(rule.checked).unwrap_or(0.0) as i64;
        // Failures are counted one at a time; the first KEEP are kept.
        let mut rows: Vec<Vec<S>> = Vec::new();
        let mut n = 0i64;
        let read = b.each(rule.failures, |r| {
            n += 1;
            if rows.len() < KEEP {
                rows.push((0..4).map(|i| r.get::<_, S>(i)).collect::<rusqlite::Result<_>>()?);
            }
            Ok(())
        });
        let (status, why) = match read {
            Ok(()) => (if n == 0 { "ok" } else { "failed" }, rule.why.to_string()),
            Err(e) => (if e.to_string().contains("no such") { "no data" } else { "error" }, e.to_string()),
        };
        b.conn.execute(
            "INSERT INTO consistency VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![rule.id, rule.areas, rule.title, status, checked, n, why, rule.needs, rule.failures],
        )?;
        for r in rows.iter().take(KEEP) {
            let year = match r.get(2) {
                Some(S::Integer(y)) => Some(*y),
                _ => None,
            };
            let says = format!("{}: {}", rule.title, r.get(3).map(cell).unwrap_or_default());
            ins(
                "one",
                "contradiction",
                crate::pages::page_of(rule.areas.split_whitespace().next().unwrap_or("T11")),
                rule.id,
                &r.first().map(cell).unwrap_or_default(),
                &r.get(1).map(cell).unwrap_or_default(),
                year,
                &says,
                &r.get(3).map(cell).unwrap_or_default(),
                None,
            )?;
        }
    }
    // One record: rare values, with their percentile against the world.
    for o in crate::defs::outliers() {
        if !have(o.needs) {
            continue;
        }
        // Two reads: every value for the percentiles, then only the rare rows.
        let mut vals: Vec<f64> = Vec::new();
        if b.each(o.sql, |r| {
            vals.extend(num(Some(&r.get::<_, S>(3)?)));
            Ok(())
        })
        .is_err()
        {
            continue;
        }
        vals.sort_by(|a, b| a.total_cmp(b));
        let mut picked: Vec<(f64, Vec<S>)> = Vec::new();
        b.each(o.sql, |r| {
            let Some(v) = num(Some(&r.get::<_, S>(3)?)) else { return Ok(()) };
            let p = crate::stats::rank(&vals, v);
            if if o.low { p <= 1.0 - o.above } else { p >= o.above } {
                picked.push((p, (0..5).map(|i| r.get::<_, S>(i)).collect::<rusqlite::Result<_>>()?));
            }
            Ok(())
        })?;
        picked.sort_by(|a, b| if o.low { a.0.total_cmp(&b.0) } else { b.0.total_cmp(&a.0) });
        for (p, r) in picked.into_iter().take(KEEP / 4) {
            let year = match r.get(2) {
                Some(S::Integer(y)) => Some(*y),
                _ => None,
            };
            let share = if o.low { (1.0 - p) * 100.0 } else { p * 100.0 };
            let words = r.get(4).map(cell).unwrap_or_default();
            let says = format!(
                "{}: {} ({} than {:.1}% of the {} measured)",
                o.title,
                words,
                if o.low { "lower" } else { "more" },
                share,
                vals.len()
            );
            ins(
                "one",
                "outlier",
                o.page,
                o.id,
                &r.first().map(cell).unwrap_or_default(),
                &r.get(1).map(cell).unwrap_or_default(),
                year,
                &says,
                &words,
                Some(p),
            )?;
        }
    }
    // Data: live known issues.
    for i in issues.0.iter().filter(|i| i.live) {
        let says = format!("{} ({})", i.title, i.live_why);
        ins("data", "known issue", "tools", &i.id, "issue", &i.id, None, &says, &i.touches.join(" "), None)?;
    }
    if let Some(m) = b.meta("missing") {
        for x in serde_json::from_str::<Vec<String>>(&m).unwrap_or_default() {
            ins("data", "not recorded", "tools", "book", "log", "", None, &x, "", None)?;
        }
    }
    Ok(())
}

fn num(v: Option<&S>) -> Option<f64> {
    match v? {
        S::Integer(i) => Some(*i as f64),
        S::Real(f) => Some(*f),
        _ => None,
    }
}
