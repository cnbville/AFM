//! The world log into the book. Two layouts share a log folder
//! (`docs/world-log.md`, `docs/reference/how-it-works/world-log.md`):
//! - the game log (lane R1): `<season>/<stream>.jsonl.zst`, JSON lines;
//! - the world log (lane R2): `<kind>/YYYY-NNNN.mpz`, MessagePack records
//!   in zstd frames.
//!
//! Either name lands in the same table whichever layout holds it, so a log
//! that moves a stream from one layout to the other still reads.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

use rmpv::Value as V;
use rusqlite::types::Value as S;

use crate::json;
use crate::sink::Writer;

#[derive(Clone, Debug, Default)]
pub struct LogInfo {
    /// `meta.json` as written.
    pub meta: serde_json::Value,
    /// Each stream or kind: records, seasons and the table it went to.
    pub streams: BTreeMap<String, Stream>,
    pub commit: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Stream {
    pub records: u64,
    pub seasons: std::collections::BTreeSet<i32>,
    pub table: String,
    pub bytes: u64,
}

/// The fields a packed table keeps as columns besides its top-level ones.
fn packed_keys(kind: &str) -> &'static [&'static str] {
    match kind {
        "players" | "exits" => &[
            "person.stage",
            "person.stars",
            "person.hs",
            "person.college",
            "person.class_of",
            "person.player.position",
            "person.player.age",
            "why",
        ],
        // A coach's year: his job, reputation and hidden parts (the Coaches page's lines by year).
        "coaches" => &[
            "coach.name",
            "coach.age",
            "coach.ethnicity",
            "coach.retired",
            "coach.reputation",
            "coach.stat_buzz",
            "coach.learning_ceiling",
            "coach.mentor",
            "coach.wins",
            "coach.losses",
            "coach.career_titles",
            "coach.prove_it",
            "coach.contract.salary",
            "coach.job.role",
            "coach.job.role.PositionCoach",
            "coach.job.since",
            "coach.job.employer.College",
            "coach.job.employer.HighSchool",
            "coach.job.employer.Nfl",
            "coach.job.employer.Other",
            "coach.ability.teaching.0",
            "coach.ability.teaching.1",
            "coach.ability.teaching.2",
            "coach.ability.teaching.3",
            "coach.ability.teaching.4",
            "coach.ability.teaching.5",
            "coach.ability.teaching.6",
            "coach.ability.teaching.7",
            "coach.ability.scheme",
            "coach.ability.game_management",
            "coach.ability.motivation",
            "coach.ability.discipline",
            "coach.ability.recruiting",
            "coach.ability.network",
            "coach.ability.relationships",
        ],
        "programs" => &[
            "program.real_name",
            "program.fictional_name",
            "program.division",
            "program.conference",
            "program.prestige",
            "program.head_coach",
            "result.expected",
            "result.wins",
            "result.losses",
        ],
        "schools" => &[
            "school.name",
            "school.state",
            "school.classification",
            "school.enrollment",
            "school.private",
            "school.prestige",
            "school.coach",
            "result.wins",
            "result.losses",
            "result.state_title",
        ],
        _ => &[],
    }
}

const SITUATION: [&str; 8] = ["plays", "yards", "epa", "successes", "firsts", "tds", "turnovers", "extra"];

pub fn read(dir: &Path, w: &mut Writer, log: &mut dyn FnMut(&str)) -> std::io::Result<LogInfo> {
    let mut info = LogInfo::default();
    if let Ok(b) = std::fs::read(dir.join("meta.json")) {
        info.meta = serde_json::from_slice(&b).unwrap_or_default();
        info.commit = info
            .meta
            .get("commit")
            .and_then(|c| c.as_str())
            .map(str::to_string)
            .or_else(|| info.meta.pointer("/sessions/0/commit").and_then(|c| c.as_str()).map(str::to_string));
    }
    w.source = "world log".into();
    let mut dirs: Vec<_> = std::fs::read_dir(dir)?.filter_map(|e| e.ok()).filter(|e| e.path().is_dir()).collect();
    dirs.sort_by_key(|e| e.file_name());
    for d in dirs {
        let name = d.file_name().to_string_lossy().to_string();
        let mut files: Vec<_> = std::fs::read_dir(d.path())?.filter_map(|e| e.ok()).map(|e| e.path()).collect();
        files.sort();
        if let Ok(season) = name.parse::<i32>() {
            // The game log: one file per stream.
            for f in files {
                let fname = f.file_name().unwrap_or_default().to_string_lossy().to_string();
                let Some(stream) = fname.strip_suffix(".jsonl.zst").or_else(|| fname.strip_suffix(".jsonl")) else {
                    continue;
                };
                let t = std::time::Instant::now();
                let bytes = f.metadata().map_or(0, |m| m.len());
                let mut n = 0u64;
                let file = std::fs::File::open(&f)?;
                let rd: Box<dyn Read> =
                    if fname.ends_with(".zst") { Box::new(zstd::Decoder::new(file)?) } else { Box::new(file) };
                for line in BufReader::with_capacity(1 << 20, rd).lines() {
                    let line = line?;
                    if line.trim().is_empty() {
                        continue;
                    }
                    let Ok(j) = serde_json::from_str::<serde_json::Value>(&line) else { continue };
                    let table = route(w, stream, season, 0, &json::from_json(&j))?;
                    let s = info.streams.entry(stream.to_string()).or_default();
                    s.table = table;
                    n += 1;
                }
                let s = info.streams.entry(stream.to_string()).or_default();
                s.records += n;
                s.seasons.insert(season);
                s.bytes += bytes;
                log(&format!("  {season} {stream}: {n} in {:.1}s", t.elapsed().as_secs_f64()));
            }
        } else {
            // The world log: segments of one kind.
            let t = std::time::Instant::now();
            let mut n = 0u64;
            for f in files {
                let Some(stem) = f.file_name().and_then(|x| x.to_str()).and_then(|x| x.strip_suffix(".mpz")) else {
                    continue;
                };
                let Some((y, sq)) = stem.rsplit_once('-') else { continue };
                let (Ok(year), Ok(seq)) = (y.parse::<i32>(), sq.parse::<i32>()) else { continue };
                let bytes = f.metadata().map_or(0, |m| m.len());
                let mut r = BufReader::with_capacity(1 << 20, zstd::Decoder::new(std::fs::File::open(&f)?)?);
                loop {
                    match r.fill_buf() {
                        Ok([]) | Err(_) => break,
                        Ok(_) => {}
                    }
                    // A segment cut short by a crash ends at its last whole record.
                    let Ok(v) = rmpv::decode::read_value(&mut r) else { break };
                    let table = route(w, &name, year, seq, &v)?;
                    let s = info.streams.entry(name.clone()).or_default();
                    s.table = table;
                    s.records += 1;
                    n += 1;
                }
                let s = info.streams.entry(name.clone()).or_default();
                s.seasons.insert(year);
                s.bytes += bytes;
            }
            log(&format!("  {name}: {n} in {:.1}s", t.elapsed().as_secs_f64()));
        }
    }
    w.flush().map_err(std::io::Error::other)?;
    Ok(info)
}

/// One record into its table; returns the table's name.
fn route(w: &mut Writer, stream: &str, season: i32, seq: i32, v: &V) -> std::io::Result<String> {
    let e = std::io::Error::other;
    let level = stream.split_once('-').map_or("", |(_, l)| l);
    let mut fixed = vec![("season".to_string(), S::Integer(season as i64))];
    if seq > 0 {
        fixed.push(("segment".into(), S::Integer(seq as i64)));
    }
    let table = match stream {
        s if s.starts_with("games-") => {
            fixed.push(("stream".into(), S::Text(level.to_string())));
            let pair = |k: &str| match json::get(v, k) {
                Some(V::Array(a)) if a.len() == 2 => Some((a[0].clone(), a[1].clone())),
                _ => None,
            };
            if let Some((h, a)) = pair("teams") {
                fixed.extend([("home".into(), crate::sink::scalar(&h).unwrap_or(S::Null))]);
                fixed.push(("away".into(), crate::sink::scalar(&a).unwrap_or(S::Null)));
            }
            if let Some((h, a)) = pair("score") {
                fixed.push(("home_pts".into(), crate::sink::scalar(&h).unwrap_or(S::Null)));
                fixed.push(("away_pts".into(), crate::sink::scalar(&a).unwrap_or(S::Null)));
            }
            w.put("log_games", fixed, v).map_err(e)?;
            "log_games"
        }
        s if s.starts_with("plays-") => {
            fixed.push(("stream".into(), S::Text(level.to_string())));
            if let Some(V::Array(p)) = json::get(v, "plays") {
                fixed.push(("n".into(), S::Integer(p.len() as i64)));
            }
            // A game's snaps join its row by season, level, date and teams (R8).
            if let Some(V::Array(t)) = json::get(v, "teams")
                && t.len() == 2
            {
                fixed.push(("home".into(), crate::sink::scalar(&t[0]).unwrap_or(S::Null)));
                fixed.push(("away".into(), crate::sink::scalar(&t[1]).unwrap_or(S::Null)));
            }
            w.put_packed("log_plays", fixed, v).map_err(e)?;
            "log_plays"
        }
        "situations" | "team-situations" => {
            if let Some(V::Array(a)) = json::get(v, "v") {
                for (i, x) in a.iter().enumerate().take(SITUATION.len()) {
                    if let Some(s) = crate::sink::scalar(x) {
                        fixed.push((SITUATION[i].to_string(), s));
                    }
                }
            }
            let t = if stream == "situations" { "log_situations" } else { "log_team_situations" };
            w.put(t, fixed, v).map_err(e)?;
            t
        }
        "people" | "events" | "news" | "whys" | "dropped" => {
            let t = format!("log_{stream}");
            w.put(&t, fixed, v).map_err(e)?;
            return Ok(t);
        }
        "players" | "exits" | "growth" | "coaches" | "programs" | "schools" | "clubs" | "snapshots" | "world" => {
            let t = format!("log_{stream}");
            w.packed(&t, packed_keys(stream)).map_err(e)?;
            w.put_packed(&t, fixed, v).map_err(e)?;
            return Ok(t);
        }
        other => {
            let t = format!("log_{}", other.replace(|c: char| !c.is_ascii_alphanumeric(), "_"));
            w.put(&t, fixed, v).map_err(e)?;
            return Ok(t);
        }
    };
    Ok(table.to_string())
}
