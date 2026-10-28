//! The search index and the hover-card definitions.
//! - `search`: every person, alumnus, coach, program, high school and NFL
//!   club by name, with the table and row that hold it.
//! - `defs`: what each number in the book is (checks, distributions,
//!   inventory areas, attributes, knobs, tables, real figures), with where
//!   it comes from and its real source and years.

use crate::book::Book;

pub fn write(b: &Book) -> rusqlite::Result<()> {
    b.conn.execute_batch(
        "CREATE TABLE search(kind TEXT, id TEXT, name TEXT, detail TEXT, first_year INTEGER, last_year INTEGER, tbl TEXT, row INTEGER);
         CREATE TABLE defs(key TEXT PRIMARY KEY, kind TEXT, title TEXT, what TEXT, file TEXT, function TEXT, unit TEXT,
            real_source TEXT, real_years TEXT);",
    )?;
    let has = |t: &str, cs: &[&str]| {
        let have = b.columns(t);
        !have.is_empty() && cs.iter().all(|c| have.iter().any(|h| h == c))
    };
    // (kind, table, needed columns, id, name, detail, first year, last year)
    let parts: &[(&str, &str, &[&str], &str, &str, &str, &str, &str)] = &[
        (
            "person",
            "people",
            &["player__id", "name", "player__position", "stage", "class_of"],
            "player__id",
            "name",
            "player__position || ', ' || stage || ', class of ' || class_of",
            "class_of - 4",
            "NULL",
        ),
        (
            "alumnus",
            "alumni",
            &["name", "position", "year", "stars", "pick"],
            "COALESCE(player__id, 'a' || _row)",
            "name",
            "position || ', ' || stars || ' stars, ' || CASE WHEN pick IS NULL THEN 'undrafted' ELSE 'pick ' || pick END || ' in ' || year",
            "year - 4",
            "year",
        ),
        ("coach", "coaches", &["id", "name"], "id", "name", "COALESCE(job__role, 'no job')", "NULL", "NULL"),
        (
            "program",
            "programs",
            &["id", "real_name", "division", "conference"],
            "id",
            "real_name",
            "division || ', ' || conference || COALESCE(' (' || fictional_name || ')', '')",
            "NULL",
            "NULL",
        ),
        (
            "school",
            "high_schools",
            &["id", "name", "town"],
            "id",
            "name",
            "town || ', ' || COALESCE(county, '') || ' county'",
            "NULL",
            "NULL",
        ),
        ("club", "clubs", &["name"], "_row", "name", "'NFL club'", "NULL", "NULL"),
    ];
    for (kind, t, need, id, name, detail, first, last) in parts {
        if !has(t, need) {
            continue;
        }
        // An optional column the world file lacks is left out of the text.
        let fix = |s: &str| {
            let mut s = s.to_string();
            for (c, extra, alt) in [
                ("player__id", "alumni", "NULL"),
                ("job__role", "coaches", "NULL"),
                ("fictional_name", "programs", "NULL"),
                ("county", "high_schools", "NULL"),
            ] {
                if extra == *t && !has(t, &[c]) {
                    s = s.replace(c, alt);
                }
            }
            s
        };
        let sql = format!(
            "INSERT INTO search SELECT '{kind}', {}, {name}, {}, {}, {}, '{t}', _row FROM \"{t}\"",
            fix(id),
            fix(detail),
            first,
            last
        );
        b.conn.execute_batch(&sql)?;
    }
    b.conn.execute_batch(
        "CREATE INDEX ix_search_name ON search(name COLLATE NOCASE); CREATE INDEX ix_search_id ON search(kind, id);",
    )?;

    // Hover cards.
    let inserts = [
        "INSERT OR IGNORE INTO defs SELECT id, 'check', title, how || CASE WHEN needs != 'world file' THEN ' Needs the ' || needs || '.' ELSE '' END,
            'crates/afm-reader/src/defs_list.rs', id, unit, real_source, real_years FROM checks",
        "INSERT OR IGNORE INTO defs SELECT id, 'distribution', title, 'Spread of ' || title || ' by year and group (' || rows || ' values).',
            'crates/afm-reader/src/defs_list.rs', id, unit, NULL, NULL FROM metrics",
        "INSERT OR IGNORE INTO defs SELECT id, 'consistency', title, why, 'crates/afm-reader/src/defs_list.rs', id, NULL, NULL, NULL FROM consistency",
        "INSERT OR IGNORE INTO defs SELECT code, 'area', title, shows, NULL, NULL, NULL, NULL, NULL FROM coverage",
        "INSERT OR IGNORE INTO defs SELECT 'attr:' || name, 'attribute', label, label, 'crates/afm-engine/src/attributes.rs', 'Attr::' || name,
            NULL, NULL, NULL FROM engine_attributes",
        "INSERT OR IGNORE INTO defs SELECT 'knob:' || name, 'knob', name, value || CASE WHEN note != '' THEN ' (' || note || ')' ELSE '' END ||
            ', ' || status, file, name, NULL, NULL, NULL FROM knobs",
        "INSERT OR IGNORE INTO defs SELECT 'table:' || tbl, 'table', tbl, what, NULL, NULL, NULL, source, NULL FROM catalog",
        "INSERT OR IGNORE INTO defs SELECT 'issue:' || id, 'known issue', title, live_why, location, NULL, NULL, NULL, NULL FROM known_issues",
    ];
    for q in inserts {
        // A list a book lacks leaves its cards out.
        let _ = b.conn.execute_batch(q);
    }
    if has("real_figures", &["key", "file"]) {
        let cols = b.columns("real_figures");
        let c = |n: &str| if cols.iter().any(|x| x == n) { n.to_string() } else { "NULL".into() };
        let q = format!(
            "INSERT OR IGNORE INTO defs SELECT 'real:' || key, 'real figure', COALESCE({label}, key), COALESCE({note}, ''),
                'data/real-world/' || file, NULL, {unit}, COALESCE({source}, file), {years} FROM real_figures",
            label = c("label"),
            note = c("note"),
            unit = c("unit"),
            source = c("source"),
            years = c("years"),
        );
        let _ = b.conn.execute_batch(&q);
    }
    Ok(())
}
