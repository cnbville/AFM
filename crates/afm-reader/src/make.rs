//! Building a book: the world file, its log, the engine's answers and the
//! real index into tables, then everything worked out over them.

use std::path::{Path, PathBuf};
use std::time::Instant;

use rusqlite::params;

use crate::book::Book;
use crate::sink::Writer;
use crate::{log_in, real_in, world_in};

/// The book's format, bumped when a table changes shape.
pub const FORMAT: u32 = 1;

#[derive(Clone, Debug, Default)]
pub struct Summary {
    pub tables: usize,
    pub rows: u64,
    pub bytes: u64,
    pub seconds: f64,
    pub peak_mb: f64,
    pub checks: usize,
    pub has_log: bool,
    pub missing: Vec<String>,
}

/// The log folder beside a world file or save (`name.afmlog`), if any.
pub fn log_beside(world: &Path) -> Option<PathBuf> {
    let p = world.with_extension("afmlog");
    p.is_dir().then_some(p)
}

/// Builds the book for `world` at `out`, reading `log` when given.
pub fn build(world: &Path, log: Option<&Path>, out: &Path, say: &mut dyn FnMut(&str)) -> std::io::Result<Summary> {
    let t = Instant::now();
    // Each step says when it started and the memory peak so far.
    let say = &mut |s: &str| say(&format!("{s} [{:.0}s, peak {:.0} MB]", t.elapsed().as_secs_f64(), peak_mb()));
    let e = std::io::Error::other;
    let tmp = out.with_extension("part");
    let mut w = Writer::create(&tmp).map_err(e)?;
    say(&format!("world file {}", world.display()));
    let info = world_in::read(world, &mut w, say)?;
    let head = |k: &str| info.header.get(k).cloned().unwrap_or_default();
    let start = head("start_year").as_i64().unwrap_or(info.year as i64) as i32;
    let years = head("history_years").as_i64().unwrap_or(0) as i32;
    let first = start - years;
    let last = info.year.max(start);
    let mut log_info = None;
    if let Some(dir) = log {
        say(&format!("world log {}", dir.display()));
        log_info = Some(log_in::read(dir, &mut w, say)?);
    }
    say("engine tables and the real index");
    world_in::engine_tables(&mut w, first, last)?;
    let index = real_in::index();
    real_in::read(&mut w, &index).map_err(e)?;
    // The catalog of tables, before the worked-out ones are added.
    let mut cat: Vec<(String, String, Option<String>, u64, usize, String)> = w
        .tables
        .iter()
        .map(|(n, t)| {
            (
                n.clone(),
                t.source.clone(),
                t.parent.clone(),
                t.rows,
                t.cols.len(),
                serde_json::to_string(&t.cols).unwrap_or_default(),
            )
        })
        .collect();
    cat.sort();
    w.flush().map_err(e)?;
    let conn = std::mem::replace(&mut w.conn, rusqlite::Connection::open_in_memory().map_err(e)?);
    drop(w);
    let b = Book::with(conn).map_err(e)?;
    b.conn
        .execute_batch("CREATE TABLE catalog(tbl TEXT PRIMARY KEY, source TEXT, parent TEXT, rows INTEGER, ncols INTEGER, cols TEXT, what TEXT);")
        .map_err(e)?;
    for (n, s, p, r, nc, cols) in &cat {
        b.conn
            .execute(
                "INSERT INTO catalog VALUES(?1,?2,?3,?4,?5,?6,?7)",
                params![n, s, p, *r as i64, *nc as i64, cols, crate::coverage::what(n)],
            )
            .map_err(e)?;
    }
    say("indexes");
    indexes(&b).map_err(e)?;
    for (t, c) in crate::defs::columns() {
        let cols = b.columns(t);
        if cols.is_empty() && crate::defs::makes_missing(t) {
            b.conn.execute_batch(&format!("CREATE TABLE \"{t}\" (_row INTEGER, \"{c}\"{})", ty(c))).map_err(e)?;
        } else if !cols.is_empty() && !cols.iter().any(|x| x == c) {
            b.conn.execute_batch(&format!("ALTER TABLE \"{t}\" ADD COLUMN \"{c}\"{}", ty(c))).map_err(e)?;
        }
    }
    // What the book has, for the checks that need a log.
    let streams: Vec<String> = log_info.as_ref().map(|l| l.streams.keys().cloned().collect()).unwrap_or_default();
    let has_game_log = streams.iter().any(|s| s.starts_with("games-"));
    let has_world_log = streams.iter().any(|s| s == "players" || s == "growth" || s == "whys");
    let have = |needs: &str| match needs {
        "game log" => has_game_log,
        "world log" => has_world_log,
        _ => true,
    };
    let mut missing = Vec::new();
    if !has_game_log {
        missing.push(
            "game log (lane R1): no game, box score, player game line, drive or play; game-level checks have no data"
                .to_string(),
        );
    }
    if !has_world_log {
        missing.push("world log (lane R2): no yearly player rows, growth whys, events with whys, snapshots or exits; only the world file's moment and the records it keeps".to_string());
    }
    let unread = crate::world_in::PEOPLE_UNREAD.swap(0, std::sync::atomic::Ordering::Relaxed);
    if unread > 0 {
        missing.push(format!(
            "true overall and potential for {unread} people: the world file is from another build, so the reader's engine could not read their player part"
        ));
    }
    let world_commit = head("commit").as_str().unwrap_or("").to_string();
    let written = head("written").as_str().unwrap_or("").to_string();
    say("known issues, knobs, coverage");
    let issues = crate::issues::write(&b, &world_commit, &written).map_err(e)?;
    crate::knobs::write(&b).map_err(e)?;
    say("distributions");
    crate::dist::write_all(&b, &have).map_err(e)?;
    say("checks against real");
    let checks = crate::checks::write_all(&b, &index, &have, &|id| issues.touching(id)).map_err(e)?;
    say("panels");
    crate::panels::write_all(&b, &have).map_err(e)?;
    say("consistency checks and odd things");
    crate::odd::write_all(&b, &have, &issues).map_err(e)?;
    crate::coverage::write(&b, &streams, &log_info).map_err(e)?;
    say("search index and hover cards");
    crate::search::write(&b).map_err(e)?;
    // The book's own facts.
    b.conn.execute_batch("CREATE TABLE book_meta(key TEXT PRIMARY KEY, value TEXT);").map_err(e)?;
    let mut meta: Vec<(&str, String)> = vec![
        ("format", FORMAT.to_string()),
        ("reader_commit", crate::knobs::commit()),
        ("world_file", world.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default()),
        ("world_commit", world_commit),
        ("world_written", written),
        ("world_date", info.today.clone()),
        ("world_year", info.year.to_string()),
        ("seed", head("seed").to_string()),
        ("history_years", years.to_string()),
        ("start_year", start.to_string()),
        ("first_season", first.to_string()),
        ("last_season", last.to_string()),
        ("career", head("career").as_str().unwrap_or("").to_string()),
        ("log", log.map(|l| l.display().to_string()).unwrap_or_default()),
        ("log_streams", serde_json::to_string(&streams).unwrap_or_default()),
        ("log_meta", log_info.as_ref().map(|l| l.meta.to_string()).unwrap_or_default()),
        ("missing", serde_json::to_string(&missing).unwrap_or_default()),
        ("header", info.header.to_string()),
        ("world_format", info.format.to_string()),
    ];
    let rows: u64 = cat.iter().map(|c| c.3).sum();
    meta.push(("records", rows.to_string()));
    meta.push(("seconds", format!("{:.0}", t.elapsed().as_secs_f64())));
    meta.push(("peak_mb", format!("{:.0}", peak_mb())));
    for (k, v) in &meta {
        b.conn.execute("INSERT INTO book_meta VALUES(?1,?2)", params![k, v]).map_err(e)?;
    }
    b.conn.execute_batch("ANALYZE; COMMIT;").map_err(e)?;
    drop(b);
    std::fs::rename(&tmp, out)?;
    let bytes = std::fs::metadata(out).map_or(0, |m| m.len());
    Ok(Summary {
        tables: cat.len(),
        rows,
        bytes,
        seconds: t.elapsed().as_secs_f64(),
        peak_mb: peak_mb(),
        checks,
        has_log: log_info.is_some(),
        missing,
    })
}

/// Works out a built book's distributions, checks, panels and consistency
/// checks again with this reader's definitions, without reading the world
/// again (`afm_read refresh`): for a thread that adds or fixes one. Returns
/// how many checks and panels.
pub fn refresh(book: &Path, say: &mut dyn FnMut(&str)) -> rusqlite::Result<(usize, usize)> {
    let b = Book::with(rusqlite::Connection::open(book)?)?;
    let streams: Vec<String> = b.meta("log_streams").and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    let has_game_log = streams.iter().any(|s| s.starts_with("games-"));
    let has_world_log = streams.iter().any(|s| s == "players" || s == "growth" || s == "whys");
    let have = |needs: &str| match needs {
        "game log" => has_game_log,
        "world log" => has_world_log,
        _ => true,
    };
    for (t, c) in crate::defs::columns() {
        let cols = b.columns(t);
        if cols.is_empty() && crate::defs::makes_missing(t) {
            b.conn.execute_batch(&format!("CREATE TABLE \"{t}\" (_row INTEGER, \"{c}\"{})", ty(c)))?;
        } else if !cols.is_empty() && !cols.iter().any(|x| x == c) {
            b.conn.execute_batch(&format!("ALTER TABLE \"{t}\" ADD COLUMN \"{c}\"{}", ty(c)))?;
        }
    }
    // The live known issues, as the build marked them.
    let mut live: Vec<(String, Vec<String>)> = Vec::new();
    b.each("SELECT id, touches FROM known_issues WHERE live = 1", |r| {
        let touches: String = r.get::<_, Option<String>>(1)?.unwrap_or_default();
        live.push((r.get(0)?, touches.split_whitespace().map(str::to_string).collect()));
        Ok(())
    })
    .ok();
    let issues = crate::issues::Issues(
        live.into_iter()
            .map(|(id, touches)| crate::issues::Issue { id, live: true, touches, ..Default::default() })
            .collect(),
    );
    // Every table these steps make is made again, so a definition this
    // reader dropped leaves nothing behind.
    b.conn.execute_batch(
        "BEGIN; DROP TABLE IF EXISTS checks; DROP TABLE IF EXISTS check_years; DROP TABLE IF EXISTS dist;
         DROP TABLE IF EXISTS hist; DROP TABLE IF EXISTS metrics; DROP TABLE IF EXISTS odd; DROP TABLE IF EXISTS consistency;",
    )?;
    say("distributions");
    crate::dist::write_all(&b, &have)?;
    say("checks against real");
    let checks = crate::checks::write_all(&b, &crate::real_in::index(), &have, &|id| issues.touching(id))?;
    say("panels");
    let panels = crate::panels::write_all(&b, &have)?;
    say("consistency checks and odd things");
    crate::odd::write_all(&b, &have, &issues)?;
    b.conn.execute_batch("COMMIT;")?;
    Ok((checks, panels))
}

/// A column's declared type: `_parent` is an integer, as the sink makes it.
fn ty(c: &str) -> &'static str {
    if c == "_parent" { " INTEGER" } else { "" }
}

fn indexes(b: &Book) -> rusqlite::Result<()> {
    for t in b.tables() {
        let cols = b.columns(&t);
        let has = |c: &str| cols.iter().any(|x| x == c);
        let q = |c: &str| format!("\"{c}\"");
        let mut idx: Vec<Vec<&str>> = Vec::new();
        if has("_parent") {
            idx.push(vec!["_parent"]);
        }
        if t.starts_with("log_") {
            for k in [["season", "stream", "id"].as_slice(), &["id", "season"], &["home"], &["away"]] {
                if k.iter().all(|c| has(c)) {
                    idx.push(k.to_vec());
                }
            }
        }
        for k in ["player__id", "id"] {
            if has(k) && !t.starts_with("log_") {
                idx.push(vec![k]);
            }
        }
        // A school's and a college's players, a coach's protégés, the news
        // that names a program, coach or club (the entity pages, R7).
        let more: &[&str] = match t.as_str() {
            "people" => &["hs", "college"],
            "coaches" => &["mentor"],
            "news__refs" => &["to__Program", "to__Coach", "to__Club"],
            _ => &[],
        };
        for k in more {
            if has(k) {
                idx.push(vec![k]);
            }
        }
        for (i, cols) in idx.iter().enumerate() {
            let list: Vec<String> = cols.iter().map(|c| q(c)).collect();
            b.conn.execute_batch(&format!("CREATE INDEX \"ix_{t}_{i}\" ON \"{t}\"({})", list.join(",")))?;
        }
    }
    Ok(())
}

/// The process's peak memory in MB (Linux; 0 elsewhere).
pub fn peak_mb() -> f64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines().find(|l| l.starts_with("VmHWM:")).and_then(|l| l.split_whitespace().nth(1)?.parse::<f64>().ok())
        })
        .map_or(0.0, |kb| kb / 1024.0)
}
