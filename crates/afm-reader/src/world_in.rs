//! The world file into the book, read straight from its MessagePack
//! (`save.rs`: a header, then the `World` with named fields through zstd;
//! in format 2 the world without its big lists, then each list in pieces
//! of its own, read here in order as if the list were whole).
//! Nothing is loaded whole: each store's records stream into their tables
//! one at a time, so a world of any size reads in about the game's import
//! time with a few hundred MB of memory, and a world from any build reads
//! (a field the engine has since renamed is simply its old name).
//!
//! Players and alumni also get what the engine works out and never stores
//! (true overall, true potential, age), through the engine's own types.

use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use rayon::prelude::*;
use rmpv::Value as V;
use rusqlite::types::Value as S;

use crate::json;
use crate::sink::{self, Writer};

/// What the world file says about itself.
#[derive(Clone, Debug, Default)]
pub struct WorldInfo {
    pub header: serde_json::Value,
    /// The file's format (`save.rs`, `FORMAT_VERSION`).
    pub format: u32,
    pub year: i32,
    pub today: String,
    pub stores: Vec<(String, u64)>,
}

/// Stores read through this many levels: a list at any of them streams its
/// records one by one; deeper values are read whole.
const STREAM_DEPTH: usize = 2;
/// Records converted together (the engine's types in parallel).
const BATCH: usize = 256;
/// The newest world file format the reader knows.
const FORMAT: u32 = 2;

/// A format 2 file's lists kept as parts of their own: the table each
/// streams into and where its pieces are, in order (`save.rs`, `Kind`).
#[derive(Default)]
struct Pieces {
    path: PathBuf,
    lists: Vec<(&'static str, Vec<(u64, u64)>)>,
}

impl Pieces {
    fn of(&self, table: &str) -> Option<&[(u64, u64)]> {
        self.lists.iter().find(|(t, _)| *t == table).map(|(_, p)| p.as_slice())
    }
}

/// One part's zstd stream, on its own handle to the file.
fn open_part(path: &Path, offset: u64, len: u64) -> std::io::Result<impl Read> {
    let mut f = std::fs::File::open(path)?;
    f.seek(SeekFrom::Start(offset))?;
    Ok(BufReader::with_capacity(1 << 22, zstd::Decoder::new(f.take(len))?.single_frame()))
}

pub fn read(path: &Path, w: &mut Writer, log: &mut dyn FnMut(&str)) -> std::io::Result<WorldInfo> {
    let mut f = BufReader::with_capacity(1 << 20, std::fs::File::open(path)?);
    let mut magic = [0u8; 8];
    f.read_exact(&mut magic)?;
    if &magic != afm_engine::save::MAGIC {
        return Err(std::io::Error::other("not an AFM world or save file"));
    }
    let mut b4 = [0u8; 4];
    f.read_exact(&mut b4)?;
    let format = u32::from_le_bytes(b4);
    if !(1..=FORMAT).contains(&format) {
        return Err(std::io::Error::other(format!(
            "world file format {format}, this reader reads up to {FORMAT}: update the reader"
        )));
    }
    f.read_exact(&mut b4)?;
    let mut head = vec![0u8; u32::from_le_bytes(b4) as usize];
    f.read_exact(&mut head)?;
    let header: serde_json::Value = serde_json::from_slice(&head).map_err(std::io::Error::other)?;
    let mut pieces = Pieces { path: path.to_path_buf(), ..Default::default() };
    let mut r: Box<dyn Read> = if format == 1 {
        Box::new(BufReader::with_capacity(1 << 22, zstd::Decoder::new(f)?))
    } else {
        // The table of parts: an 8-byte name, the item count, the offset
        // and the length (`save.rs`, `ENTRY`).
        f.read_exact(&mut b4)?;
        let mut table = vec![0u8; u32::from_le_bytes(b4) as usize * 32];
        f.read_exact(&mut table)?;
        let mut rest = None;
        for e in table.as_chunks::<32>().0 {
            let at = |i: usize| u64::from_le_bytes(e[8 + 8 * i..16 + 8 * i].try_into().unwrap_or_default());
            let spot = (at(1), at(2));
            let list = match std::str::from_utf8(&e[..8]).unwrap_or("").trim_end_matches('\0') {
                "rest" => {
                    rest = Some(spot);
                    continue;
                }
                "alumni" => "alumni",
                "news" => "news",
                "seasons" => "stats__seasons",
                "people" => "people",
                other => return Err(std::io::Error::other(format!("a part this reader doesn't know: {other}"))),
            };
            match pieces.lists.iter_mut().find(|(t, _)| *t == list) {
                Some((_, p)) => p.push(spot),
                None => pieces.lists.push((list, vec![spot])),
            }
        }
        let (offset, len) = rest.ok_or_else(|| std::io::Error::other("the world file has no rest part"))?;
        Box::new(open_part(path, offset, len)?)
    };
    let r = &mut r;
    let mut info = WorldInfo { header, format, ..Default::default() };
    w.source = "world file".into();
    let n = match container(r)? {
        Head::Map(n) => n,
        _ => return Err(std::io::Error::other("the world is not a map")),
    };
    let mut top: Vec<(V, V)> = Vec::new();
    for _ in 0..n {
        let k = rmpv::decode::read_value(r).map_err(std::io::Error::other)?;
        let name = json::key(&k);
        let t = std::time::Instant::now();
        let before: u64 = w.tables.values().map(|t| t.rows).sum();
        if let Some(v) = stream(r, w, &name, 1, &pieces)? {
            match (name.as_str(), &v) {
                ("year", _) => info.year = json::as_i64(&v).unwrap_or(0) as i32,
                ("today", _) => info.today = date_text(&v),
                _ => {}
            }
            top.push((k, v));
        }
        let rows: u64 = w.tables.values().map(|t| t.rows).sum::<u64>() - before;
        if rows > 0 {
            log(&format!("  {name}: {rows} rows in {:.1}s", t.elapsed().as_secs_f64()));
        }
        info.stores.push((name, rows));
    }
    // The world's own scalars and small stores in one row.
    w.put("world", vec![("_row".into(), S::Integer(0))], &V::Map(top)).map_err(std::io::Error::other)?;
    w.flush().map_err(std::io::Error::other)?;
    Ok(info)
}

/// A date as `YYYY-MM-DD` from however the engine wrote it.
pub fn date_text(v: &V) -> String {
    match v {
        V::String(s) => s.as_str().unwrap_or("").to_string(),
        // A day number since 1970 (`calendar::Date`).
        V::Integer(i) => afm_engine::calendar::Date(i.as_i64().unwrap_or(0) as i32).to_string(),
        V::Map(_) => {
            let g = |k| json::get(v, k).and_then(json::as_i64).unwrap_or(0);
            format!("{:04}-{:02}-{:02}", g("year"), g("month"), g("day"))
        }
        V::Array(a) if a.len() == 3 => {
            let g = |i: usize| json::as_i64(&a[i]).unwrap_or(0);
            format!("{:04}-{:02}-{:02}", g(0), g(1), g(2))
        }
        other => json::to_json(other),
    }
}

enum Head {
    Array(u32),
    Map(u32),
    Other(u8),
}

fn container<R: Read>(r: &mut R) -> std::io::Result<Head> {
    let mut b = [0u8; 1];
    r.read_exact(&mut b)?;
    let m = b[0];
    let mut len = |n: usize| -> std::io::Result<u32> {
        let mut x = [0u8; 4];
        r.read_exact(&mut x[4 - n..])?;
        Ok(u32::from_be_bytes(x))
    };
    Ok(match m {
        0x90..=0x9f => Head::Array((m & 0x0f) as u32),
        0xdc => Head::Array(len(2)?),
        0xdd => Head::Array(len(4)?),
        0x80..=0x8f => Head::Map((m & 0x0f) as u32),
        0xde => Head::Map(len(2)?),
        0xdf => Head::Map(len(4)?),
        other => Head::Other(other),
    })
}

/// Reads one value at `table`: a list's records become rows as they come;
/// a struct's lists become their own tables (`table__field`). Returns the
/// value when it is small enough to keep in its parent's row.
fn stream<R: Read>(
    r: &mut R,
    w: &mut Writer,
    table: &str,
    depth: usize,
    pieces: &Pieces,
) -> std::io::Result<Option<V>> {
    match container(r)? {
        Head::Array(n) => {
            let mut batch: Vec<(usize, V)> = Vec::with_capacity(BATCH.min(n as usize));
            let mut scalars: Vec<V> = Vec::new();
            items(r, n as usize, 0, w, table, &mut batch, &mut scalars)?;
            // A format 2 list is empty here: its records are in pieces.
            if let (0, Some(spots)) = (n, pieces.of(table)) {
                let mut at = 0;
                for &(offset, len) in spots {
                    let mut p = open_part(&pieces.path, offset, len)?;
                    let Head::Array(m) = container(&mut p)? else {
                        return Err(std::io::Error::other(format!("a piece of {table} is not a list")));
                    };
                    items(&mut p, m as usize, at, w, table, &mut batch, &mut scalars)?;
                    at += m as usize;
                }
            }
            if !batch.is_empty() {
                records(w, table, batch)?;
            }
            Ok((!scalars.is_empty()).then_some(V::Array(scalars)))
        }
        Head::Map(n) if depth <= STREAM_DEPTH => {
            let mut keep: Vec<(V, V)> = Vec::new();

            for _ in 0..n {
                let k = rmpv::decode::read_value(r).map_err(std::io::Error::other)?;
                let is_field = matches!(&k, V::String(s) if s.as_str().is_some_and(|t| t.parse::<f64>().is_err() && !t.starts_with('[')));
                if is_field {
                    let child = format!("{table}__{}", json::key(&k));
                    if let Some(v) = stream(r, w, &child, depth + 1, pieces)? {
                        keep.push((k, v));
                    }
                } else {
                    // An id-keyed map: each entry a row with `_key`.
                    let v = rmpv::decode::read_value(r).map_err(std::io::Error::other)?;
                    let fixed = vec![("_key".to_string(), key_value(&k))];
                    w.put(table, fixed, &v).map_err(std::io::Error::other)?;
                }
            }
            if keep.is_empty() {
                return Ok(None);
            }
            if depth == 1 {
                // A store that is a struct: its small fields in one row.
                w.put(table, vec![("_row".into(), S::Integer(0))], &V::Map(keep)).map_err(std::io::Error::other)?;
                return Ok(None);
            }
            Ok(Some(V::Map(keep)))
        }
        Head::Map(n) => {
            let mut m = Vec::with_capacity(n as usize);
            for _ in 0..n {
                let k = rmpv::decode::read_value(r).map_err(std::io::Error::other)?;
                let v = rmpv::decode::read_value(r).map_err(std::io::Error::other)?;
                m.push((k, v));
            }
            Ok(Some(V::Map(m)))
        }
        Head::Other(b) => {
            let mut c = std::io::Cursor::new([b]).chain(r);
            let v = rmpv::decode::read_value(&mut c).map_err(std::io::Error::other)?;
            Ok(Some(v))
        }
    }
}

/// `n` records of a list into `table`, numbered from `at`, in batches;
/// plain values go to `scalars`.
fn items<R: Read>(
    r: &mut R,
    n: usize,
    at: usize,
    w: &mut Writer,
    table: &str,
    batch: &mut Vec<(usize, V)>,
    scalars: &mut Vec<V>,
) -> std::io::Result<()> {
    for i in at..at + n {
        let v = rmpv::decode::read_value(r).map_err(std::io::Error::other)?;
        if matches!(v, V::Map(_) | V::Array(_)) {
            batch.push((i, v));
            if batch.len() >= BATCH {
                records(w, table, std::mem::take(batch))?;
            }
        } else {
            scalars.push(v);
        }
    }
    Ok(())
}

fn key_value(k: &V) -> S {
    match k {
        V::Integer(i) => i.as_i64().map_or_else(|| S::Text(json::key(k)), S::Integer),
        V::String(s) => {
            let t = s.as_str().unwrap_or("");
            t.parse::<i64>().map_or_else(|_| S::Text(t.to_string()), S::Integer)
        }
        other => S::Text(json::key(other)),
    }
}

/// A batch of a list's records: the engine's own reads where it has them,
/// then each record into the book.
fn records(w: &mut Writer, table: &str, batch: Vec<(usize, V)>) -> std::io::Result<()> {
    let extra: Vec<Vec<(String, S)>> = match table {
        "people" => batch.par_iter().map(|(_, v)| person_extra(v)).collect(),
        "alumni" => batch.par_iter().map(|(_, v)| alumnus_extra(v)).collect(),
        _ => vec![Vec::new(); batch.len()],
    };
    for ((i, v), mut e) in batch.into_iter().zip(extra) {
        let mut fixed = vec![("_row".to_string(), S::Integer(i as i64))];
        fixed.append(&mut e);
        w.put(table, fixed, &v).map_err(std::io::Error::other)?;
    }
    Ok(())
}

/// What the engine works out for a world player: true overall and true
/// potential (debug), the position's name and the attributes by name.
/// People whose player part the engine could not read back (a world from
/// another build): they get no overall or potential.
pub static PEOPLE_UNREAD: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// A record read back as the engine's own type, the way the world file
/// is read (unit enum variants are names).
fn typed<T: serde::de::DeserializeOwned>(v: &V) -> Result<T, String> {
    let mut raw = Vec::new();
    rmpv::encode::write_value(&mut raw, v).map_err(|e| e.to_string())?;
    rmp_serde::from_slice(&raw).map_err(|e| e.to_string())
}

/// Only the player part is read: the whole person would keep its text for
/// good (`save::leaked`).
fn person_extra(v: &V) -> Vec<(String, S)> {
    match json::get(v, "player").map(typed::<afm_engine::player::Player>) {
        Some(Ok(p)) => player_cols(&p, Some(afm_engine::debug::potential(&p))),
        _ => {
            PEOPLE_UNREAD.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Vec::new()
        }
    }
}

/// An alumnus keeps his potential as it stood when he left
/// (`debug::alumnus_potential` reads the same field).
fn alumnus_extra(v: &V) -> Vec<(String, S)> {
    json::get(v, "potential")
        .and_then(json::as_f64)
        .map(|p| vec![("x_potential".into(), S::Real(round(p)))])
        .unwrap_or_default()
}

fn player_cols(p: &afm_engine::player::Player, potential: Option<f64>) -> Vec<(String, S)> {
    let mut out = vec![("x_overall".to_string(), S::Real(round(p.overall())))];
    if let Some(pot) = potential {
        out.push(("x_potential".into(), S::Real(round(pot))));
    }
    // His Dynamic Links today, 0 to 1 in `engine_links` order: derived from
    // his body and ratings, stored nowhere (`links::links_of`).
    let links: Vec<f64> =
        afm_engine::links::links_of(p).0.iter().map(|x| (f64::from(*x) * 100.0).round() / 100.0).collect();
    out.push(("x_links".into(), S::Text(serde_json::to_string(&links).unwrap_or_default())));
    out
}

fn round(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}

/// The engine's answers that no file stores, for each season the world
/// covers: the rule book (`RuleBook::era`) and the year's dates
/// (`YearDates::new`); the attributes' and Dynamic Links' names.
pub fn engine_tables(w: &mut Writer, first: i32, last: i32) -> std::io::Result<()> {
    w.source = "engine".into();
    // From 1984 so every era's rule book and dates sit beside the world's (R8).
    for season in first.min(1984)..=last {
        let fixed = || vec![("season".to_string(), S::Integer(season as i64))];
        let rules = afm_engine::calendar::RuleBook::era(season);
        w.put("engine_rules", fixed(), &to_value(&rules)?).map_err(std::io::Error::other)?;
        let dates = afm_engine::calendar::YearDates::new(season);
        w.put("engine_dates", fixed(), &to_value(&dates)?).map_err(std::io::Error::other)?;
    }
    for (i, a) in afm_engine::attributes::Attr::ALL.iter().enumerate() {
        let row = vec![
            ("_row".to_string(), S::Integer(i as i64)),
            ("name".into(), S::Text(format!("{a:?}"))),
            ("label".into(), S::Text(a.label().to_string())),
        ];
        w.insert("engine_attributes", None, row).map_err(std::io::Error::other)?;
    }
    for (i, l) in afm_engine::links::Link::ALL.iter().enumerate() {
        let parts: Vec<String> = l.parts().iter().map(|p| format!("{p:?}")).collect();
        let row = vec![
            ("_row".to_string(), S::Integer(i as i64)),
            ("name".into(), S::Text(l.name().to_string())),
            ("parts".into(), S::Text(parts.join(", "))),
            ("edge".into(), S::Text(l.edge().to_string())),
        ];
        w.insert("engine_links", None, row).map_err(std::io::Error::other)?;
    }
    // World reader R8: each franchise's eras (name, home, division, roof
    // on their real dates) and the stat book's stats by place, which the
    // world file keeps as plain lists.
    use afm_engine::world::franchise;
    let mut k = 0;
    for team in 0..afm_engine::league::NFL_TEAM_NAMES.len() {
        let mut seen = std::collections::HashSet::new();
        for year in 1920..=last.max(2026) + 1 {
            let Some(e) = franchise::era(team, year) else { continue };
            if !seen.insert(e.from) {
                continue;
            }
            let row = vec![
                ("_row".to_string(), S::Integer(k)),
                ("team".into(), S::Integer(team as i64)),
                ("from".into(), S::Integer(e.from.max(1920) as i64)),
                ("to".into(), S::Integer(e.to.min(9999) as i64)),
                ("name".into(), S::Text(e.name.to_string())),
                ("fictional".into(), S::Text(franchise::fictional_name(team, year).to_string())),
                ("city".into(), S::Text(e.city.to_string())),
                ("state".into(), S::Text(e.state.to_string())),
                ("conference".into(), S::Text(if e.div < 4 { "AFC" } else { "NFC" }.into())),
                ("div".into(), S::Integer(e.div as i64)),
                ("roof".into(), S::Text(format!("{:?}", e.roof))),
            ];
            w.insert("engine_franchises", None, row).map_err(std::io::Error::other)?;
            k += 1;
        }
    }
    for (i, s) in afm_engine::world::records::Stat::ALL.iter().enumerate() {
        let row = vec![
            ("_row".to_string(), S::Integer(i as i64)),
            ("key".into(), S::Text(s.key().to_string())),
            ("name".into(), S::Text(s.name().to_string())),
        ];
        w.insert("engine_stats", None, row).map_err(std::io::Error::other)?;
    }
    for (i, m) in afm_engine::world::records::FOOTPRINT.iter().enumerate() {
        let row = vec![("_row".to_string(), S::Integer(i as i64)), ("name".into(), S::Text(m.name().to_string()))];
        w.insert("engine_footprints", None, row).map_err(std::io::Error::other)?;
    }
    // The other pro leagues by number, with their real years (R8).
    for (i, l) in afm_engine::world::pro::leagues::LEAGUES.iter().enumerate() {
        let row = vec![
            ("_row".to_string(), S::Integer(i as i64)),
            ("name".into(), S::Text(l.name.to_string())),
            ("from".into(), S::Integer(l.from as i64)),
            ("to".into(), S::Integer(l.to as i64)),
        ];
        w.insert("engine_leagues", None, row).map_err(std::io::Error::other)?;
    }
    // The storyline kinds, so a story's kind number reads as its name (R8).
    for (i, d) in afm_engine::world::storylines::catalogue::CATALOGUE.iter().enumerate() {
        let row = vec![
            ("_row".to_string(), S::Integer(i as i64)),
            ("key".into(), S::Text(d.key.to_string())),
            ("family".into(), S::Text(format!("{:?}", d.family))),
            ("name".into(), S::Text(d.name.to_string())),
            ("stakes".into(), S::Integer(d.stakes as i64)),
            ("tone".into(), S::Integer(d.tone as i64)),
            ("public".into(), S::Integer(d.public as i64)),
        ];
        w.insert("engine_story_kinds", None, row).map_err(std::io::Error::other)?;
        // Each ending and how it reads for its team (good above 0).
        for (j, (name, reads)) in d.endings.iter().enumerate() {
            let row = vec![
                ("kind".to_string(), S::Integer(i as i64)),
                ("i".into(), S::Integer(j as i64)),
                ("name".into(), S::Text(name.to_string())),
                ("reads".into(), S::Real(f64::from(*reads))),
            ];
            w.insert("engine_story_endings", None, row).map_err(std::io::Error::other)?;
        }
    }
    // The finance books' line names and the real anchors the money follows (R8).
    use afm_engine::world::finance::{college, data, hs, nfl};
    let levels: [(&str, &[afm_engine::world::finance::Line]); 3] =
        [("nfl", &nfl::LINES), ("college", &college::LINES), ("hs", &hs::LINES)];
    let mut k = 0;
    for (level, lines) in levels {
        for (i, l) in lines.iter().enumerate() {
            let row = vec![
                ("_row".to_string(), S::Integer(k)),
                ("level".into(), S::Text(level.to_string())),
                ("i".into(), S::Integer(i as i64)),
                ("name".into(), S::Text(l.name.to_string())),
                ("income".into(), S::Integer(l.income as i64)),
            ];
            w.insert("engine_finance_lines", None, row).map_err(std::io::Error::other)?;
            k += 1;
        }
    }
    let anchors: [(&str, &[(i32, f64)]); 4] = [
        ("league revenue ($B)", &data::LEAGUE_REVENUE),
        ("national share", &data::NATIONAL_SHARE),
        ("average ticket ($)", &data::TICKET_PRICE),
        ("value multiple", &data::VALUE_MULTIPLE),
    ];
    let mut k = 0;
    for (kind, xs) in anchors {
        for (year, v) in xs {
            let row = vec![
                ("_row".to_string(), S::Integer(k)),
                ("kind".into(), S::Text(kind.to_string())),
                ("year".into(), S::Integer(*year as i64)),
                ("value".into(), S::Real(*v)),
            ];
            w.insert("engine_finance_anchors", None, row).map_err(std::io::Error::other)?;
            k += 1;
        }
    }
    for (i, part) in data::CARD_PARTS.iter().enumerate() {
        let row = vec![("_row".to_string(), S::Integer(i as i64)), ("name".into(), S::Text(part.to_string()))];
        w.insert("engine_card_parts", None, row).map_err(std::io::Error::other)?;
    }
    for (i, g) in data::GRADES.iter().enumerate() {
        let row = vec![("_row".to_string(), S::Integer(i as i64)), ("grade".into(), S::Text(g.to_string()))];
        w.insert("engine_grades", None, row).map_err(std::io::Error::other)?;
    }
    w.flush().map_err(std::io::Error::other)
}

/// Any engine value as MessagePack, the way the world file writes it.
pub fn to_value<T: serde::Serialize>(x: &T) -> std::io::Result<V> {
    let bytes = rmp_serde::to_vec_named(x).map_err(std::io::Error::other)?;
    rmpv::decode::read_value(&mut &bytes[..]).map_err(std::io::Error::other)
}

pub use sink::scalar;
