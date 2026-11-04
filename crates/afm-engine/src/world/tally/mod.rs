//! Running tallies (Data overhaul item 1, lane NB1): the counts the Data
//! section's pages read, kept by the world as its games are played,
//! instead of from the world reader's game log (a 30-year world's log is
//! about 40 GB, far too big to ship).
//!
//! Five kinds, one file each per season, so later data lanes can each grow
//! their own (`KINDS`):
//! - `players`: each college and NFL player's season: games, starts, snaps,
//!   the games he played and his numbers split by situation (`players.rs`).
//! - `teams`: each program's and club's season of situation and tendency
//!   counts, both sides of the ball (`teams.rs`).
//! - `games`: every engine game at every level: facts and boxes, and for
//!   college and the NFL the advanced numbers, the win chance before every
//!   snap and every player's line (`games.rs`).
//! - `classes`: every draft class and Division I recruiting class with what
//!   became of each member so far (`classes.rs`).
//! - `people`: each coach's unit results with their ranks (`people.rs`).
//! - `calls`: each team's 4th downs and tries against the analysts' model,
//!   its timeouts and challenges (lane NB8, `calls.rs`; the models in
//!   `decide.rs`).
//!
//! The season being played is in the world, so a save carries it. When the
//! season closes (the day after the Super Bowl, `clock.rs`, `rollover`)
//! each kind is written to `<dir>/<kind>/<season>.tally` beside the world
//! file or save (`name.afmdata`, see `crates/afm-game/src/tallies.rs`), and
//! the open store starts over, so memory stays flat. Without a folder
//! attached the closed seasons wait in memory until one is.
//!
//! Tallies only watch: no random draws, nothing in the world reads them
//! (`tests/tallies.rs` plays a world with them on and off). History counts
//! from the first recorded season, as the record books do.

pub mod calls;
pub mod classes;
pub mod decide;
pub mod depth;
pub mod expect;
pub mod file;
pub mod games;
pub mod people;
pub mod players;
pub mod teams;

use std::path::{Path, PathBuf};

use super::World;

/// The folder beside a world or save: `name.afmdata`.
pub const EXT: &str = "afmdata";
/// The kinds, each its own folder of season files.
pub const KINDS: [&str; 6] = ["players", "teams", "games", "classes", "people", "calls"];

/// The level a tally belongs to (its code in files).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Level {
    #[default]
    College = 0,
    Nfl = 1,
    Hs = 2,
}

/// The world's running tallies.
#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Tallies {
    /// Nothing is counted (`AFM_TALLIES=off`; the same-seed test's other
    /// world).
    #[serde(default)]
    pub off: bool,
    /// The season being counted.
    #[serde(default)]
    pub season: i32,
    #[serde(default)]
    pub games: games::Open,
    #[serde(default)]
    pub players: players::Open,
    #[serde(default)]
    pub teams: teams::Open,
    #[serde(default)]
    pub classes: classes::Store,
    #[serde(default)]
    pub calls: calls::Open,
    /// Seasons closed, oldest first.
    #[serde(default)]
    pub closed: Vec<i32>,
    /// Where closed seasons are written.
    #[serde(skip)]
    dir: Option<PathBuf>,
    /// Closed seasons not written yet (no folder attached): (kind, season,
    /// the file's bytes).
    #[serde(skip)]
    held: Vec<(&'static str, i32, Vec<u8>)>,
}

/// The path of a kind's season file in a folder.
pub fn path(dir: &Path, kind: &str, season: i32) -> PathBuf {
    dir.join(kind).join(format!("{season}.tally"))
}

impl Tallies {
    pub fn new() -> Tallies {
        Tallies { off: std::env::var("AFM_TALLIES").is_ok_and(|v| v == "off"), ..Default::default() }
    }

    /// Bytes of the season files waiting for a folder.
    pub fn held_bytes(&self) -> usize {
        self.held.iter().map(|h| h.2.len()).sum()
    }

    /// The open season's size as a save packs it, before compression
    /// (`memory_check` weighs it through the season).
    pub fn open_bytes(&self) -> usize {
        rmp_serde::to_vec_named(self).map_or(0, |v| v.len())
    }
}

impl World {
    /// Whether this world counts its games: recorded history, unless off.
    pub(crate) fn tallying(&self) -> bool {
        self.recording && !self.tallies.off
    }

    /// Turns the tallies off or on (a world off counts nothing new).
    pub fn set_tallies_off(&mut self, off: bool) {
        self.tallies.off = off;
    }

    /// Closed seasons go into `dir` from now on; any held in memory are
    /// written there first.
    pub fn attach_tallies(&mut self, dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        for (kind, season, bytes) in std::mem::take(&mut self.tallies.held) {
            file::write(&path(dir, kind, season), &bytes)?;
        }
        self.tallies.dir = Some(dir.to_path_buf());
        Ok(())
    }

    /// The folder closed seasons go to.
    pub fn tally_dir(&self) -> Option<&Path> {
        self.tallies.dir.as_deref()
    }

    /// Stops writing to the folder (closed seasons wait in memory again).
    pub fn detach_tallies(&mut self) -> Option<PathBuf> {
        self.tallies.dir.take()
    }

    /// Adds a game played (its tallies built where it was played).
    pub(crate) fn tally_game(&mut self, part: games::Part) {
        if !self.tallying() {
            return;
        }
        let t = &mut self.tallies;
        let key = t.games.add(part.level, part.date, &part.row);
        let [c0, c1] = part.calls;
        t.calls.add(part.level, part.teams[0], key, c0);
        t.calls.add(part.level, part.teams[1], key, c1);
        for g in part.players {
            let team = part.teams[g.side as usize & 1];
            t.players.add(part.level, part.date, team, key, g);
        }
        if let Some(sides) = part.sides {
            let [a, b] = sides;
            let d = part.date.0;
            t.teams.add(part.level, part.teams[0], d, a, [&part.adv[0], &part.adv[1]], part.score);
            t.teams.add(part.level, part.teams[1], d, b, [&part.adv[1], &part.adv[0]], [part.score[1], part.score[0]]);
        }
    }

    /// The draft's picks: (person index, overall pick, round, club).
    pub(crate) fn tally_draft(&mut self, year: i32, picks: &[(usize, u32, u32, usize)]) {
        if !self.tallying() {
            return;
        }
        let rows: Vec<classes::Pick> = picks
            .iter()
            .filter_map(|&(i, pick, round, club)| {
                let p = self.people.get(i)?;
                let id = p.player.id;
                // His college numbers: his class's when he signed with a
                // Division I class we counted, else the stat book's seasons.
                let mut college = classes::Career::default();
                for r in self.stats.career(id) {
                    college.seasons += 1;
                    college.games += r.games as u16;
                    college.snaps +=
                        (r.get(super::records::Stat::SnapsOff) + r.get(super::records::Stat::SnapsDef)) as u32;
                    college.last = college.last.max(r.year as i32);
                }
                Some(classes::Pick {
                    year,
                    round: round as u8,
                    pick: pick as u16,
                    club: club as u16,
                    id,
                    pos: Some(p.player.position),
                    program: p.college.or(p.path.last().copied()).map(|x| x as u32),
                    out: classes::Outcome {
                        college,
                        transfers: p.path.len().saturating_sub(1) as u8,
                        through: year - 1,
                        ..Default::default()
                    },
                })
            })
            .collect();
        let mut rows = rows;
        for r in rows.iter_mut() {
            if let Some(s) = self.tallies.classes.signee(r.id) {
                r.out.college = s.out.college;
            }
        }
        self.tallies.classes.draft(rows);
    }

    /// The football year is over: the season's tallies are written (or held)
    /// and the open stores start over. Only reads the world.
    pub(crate) fn tally_close(&mut self) {
        let season = self.year;
        if self.tallies.off || !self.recording {
            self.tallies.season = season + 1;
            return;
        }
        let mut classes = std::mem::take(&mut self.tallies.classes);
        classes.close(self, season, &self.tallies.players);
        self.tallies.classes = classes;
        let mut files: Vec<(&'static str, Vec<u8>)> = Vec::new();
        let mut f = file::Frames::new(file::LEVEL_CLOSE);
        for (k, s) in self.tallies.players.rows() {
            f.push(k, &file::encode(s));
        }
        files.push(("players", f.file_bytes("players", season)));
        // The latest games kept for the charting delay are the open
        // season's only.
        for s in self.tallies.teams.seasons.values_mut() {
            s.last = None;
        }
        let teams = self.tallies.teams.rows();
        let mut f = file::Frames::new(file::LEVEL_CLOSE);
        for (k, s) in &teams {
            f.push(*k, &file::encode(s));
        }
        files.push(("teams", f.file_bytes("teams", season)));
        let mut f = file::Frames::new(file::LEVEL_CLOSE);
        for r in people::rows(self, season, &teams) {
            f.push(people::key(&r), &file::encode(&r));
        }
        files.push(("people", f.file_bytes("people", season)));
        let mut f = file::Frames::new(file::LEVEL_CLOSE);
        let c = &self.tallies.classes;
        let mut members: Vec<(u64, classes::Member)> = c
            .picks
            .iter()
            .map(|p| (classes::pick_key(p), classes::Member::Pick(p.clone())))
            .chain(c.signees.iter().map(|s| (classes::signee_key(s), classes::Member::Signee(s.clone()))))
            .collect();
        members.sort_by_key(|m| m.0);
        for (k, m) in &members {
            f.push(*k, &file::encode(m));
        }
        files.push(("classes", f.file_bytes("classes", season)));
        let mut f = file::Frames::new(file::LEVEL_CLOSE);
        for (k, s) in self.tallies.calls.rows() {
            f.push(k, &file::encode(s));
        }
        files.push(("calls", f.file_bytes("calls", season)));
        let g = std::mem::take(&mut self.tallies.games);
        files.push(("games", g.frames.file_bytes("games", season)));
        let t = &mut self.tallies;
        t.players = Default::default();
        t.teams = Default::default();
        t.calls = Default::default();
        for (kind, bytes) in files {
            let written = t.dir.as_ref().map(|d| file::write(&path(d, kind, season), &bytes));
            match written {
                Some(Ok(())) => {}
                Some(Err(e)) => {
                    eprintln!("tallies: {kind} {season} not written ({e}), kept in memory");
                    t.held.push((kind, season, bytes));
                }
                None => t.held.push((kind, season, bytes)),
            }
        }
        if !t.closed.contains(&season) {
            t.closed.push(season);
        }
        t.season = season + 1;
    }

    /// The seasons with tallies, oldest first (the open one last).
    pub fn tally_seasons(&self) -> Vec<i32> {
        let mut v = self.tallies.closed.clone();
        if self.tallying() && !v.contains(&self.tallies.season) {
            v.push(self.tallies.season);
        }
        v
    }

    /// The season being counted.
    pub fn tally_open_season(&self) -> i32 {
        self.tallies.season
    }

    /// Every record of a closed season's file, from the folder or memory.
    pub fn tally_each(&self, kind: &str, season: i32, mut f: impl FnMut(u64, &[u8])) -> std::io::Result<bool> {
        if let Some((_, _, bytes)) = self.tallies.held.iter().find(|h| h.0 == kind && h.1 == season) {
            file::each_in(bytes, &mut f)?;
            return Ok(true);
        }
        let Some(dir) = &self.tallies.dir else { return Ok(false) };
        let p = path(dir, kind, season);
        if !p.exists() {
            return Ok(false);
        }
        file::open(&p)?.each(f)?;
        Ok(true)
    }

    /// Some records of a closed season's file.
    pub fn tally_get(
        &self,
        kind: &str,
        season: i32,
        keys: &[u64],
        mut f: impl FnMut(u64, &[u8]),
    ) -> std::io::Result<bool> {
        if self.tallies.held.iter().any(|h| h.0 == kind && h.1 == season) {
            let want: std::collections::HashSet<u64> = keys.iter().copied().collect();
            return self.tally_each(kind, season, |k, r| {
                if want.contains(&k) {
                    f(k, r)
                }
            });
        }
        let Some(dir) = &self.tallies.dir else { return Ok(false) };
        let p = path(dir, kind, season);
        if !p.exists() {
            return Ok(false);
        }
        file::open(&p)?.get_many(keys, f)?;
        Ok(true)
    }

    /// A player's season at a level.
    pub fn tally_player(&self, level: Level, season: i32, id: u32) -> Option<players::Season> {
        if season == self.tallies.season {
            return self.tallies.players.get(level, id).cloned();
        }
        let mut out = None;
        let _ = self.tally_get("players", season, &[players::key(level, id)], |_, r| out = file::decode(r));
        out
    }

    /// Every player's season (`f` sees each).
    pub fn tally_players(&self, season: i32, mut f: impl FnMut(&players::Season)) {
        if season == self.tallies.season {
            for (_, s) in self.tallies.players.rows() {
                f(s);
            }
            return;
        }
        let _ = self.tally_each("players", season, |_, r| {
            if let Some(s) = file::decode::<players::Season>(r) {
                f(&s)
            }
        });
    }

    /// A team's season.
    pub fn tally_team(&self, level: Level, season: i32, team: u32) -> Option<teams::Season> {
        let k = teams::key(level, team);
        if season == self.tallies.season {
            return self.tallies.teams.seasons.get(&k).cloned();
        }
        let mut out = None;
        let _ = self.tally_get("teams", season, &[k], |_, r| out = file::decode(r));
        out
    }

    /// Every team's season.
    pub fn tally_teams(&self, season: i32, mut f: impl FnMut(&teams::Season)) {
        if season == self.tallies.season {
            for (_, s) in self.tallies.teams.rows() {
                f(s);
            }
            return;
        }
        let _ = self.tally_each("teams", season, |_, r| {
            if let Some(s) = file::decode::<teams::Season>(r) {
                f(&s)
            }
        });
    }

    /// A season's games (`keys`: only these), in the order played.
    pub fn tally_games(&self, season: i32, keys: Option<&[u64]>, mut f: impl FnMut(u64, games::GameRow)) {
        let mut each = |k: u64, r: &[u8]| {
            if let Some(g) = file::decode::<games::GameRow>(r) {
                f(k, g)
            }
        };
        if season == self.tallies.season {
            match keys {
                Some(keys) => self.tallies.games.frames.get_many(keys, &mut each),
                None => self.tallies.games.frames.each(&mut each),
            }
            return;
        }
        let _ = match keys {
            Some(keys) => self.tally_get("games", season, keys, &mut each),
            None => self.tally_each("games", season, &mut each),
        };
    }

    /// A season's games read as any lighter view of `games::GameRow` (its
    /// named fields; the rest are skipped, not built).
    pub fn tally_games_as<T: serde::de::DeserializeOwned>(&self, season: i32, mut f: impl FnMut(u64, T)) {
        let mut each = |k: u64, r: &[u8]| {
            if let Some(g) = file::decode::<T>(r) {
                f(k, g)
            }
        };
        if season == self.tallies.season {
            self.tallies.games.frames.each(&mut each);
            return;
        }
        let _ = self.tally_each("games", season, &mut each);
    }

    /// A closed season's players read as any lighter view of
    /// `players::Season` (the open season is `tallies.players`).
    pub fn tally_players_as<T: serde::de::DeserializeOwned>(&self, season: i32, mut f: impl FnMut(u64, T)) {
        let _ = self.tally_each("players", season, |k, r| {
            if let Some(s) = file::decode::<T>(r) {
                f(k, s)
            }
        });
    }

    /// The open season's college or NFL games played on or after `since`.
    pub fn tally_games_since(&self, level: Level, since: crate::calendar::Date) -> Vec<games::GameRow> {
        self.tallies.games.since(level, since)
    }

    /// Each player's depth counts from this season's games on or after
    /// `since` (the charting delay leaves them out).
    pub fn tally_depth_since(
        &self,
        level: Level,
        since: crate::calendar::Date,
    ) -> std::collections::HashMap<u32, depth::Depth> {
        self.tallies.players.recent_since(level, since)
    }

    /// A season's coach unit rows (the open season's worked out now).
    pub fn tally_units(&self, season: i32, mut f: impl FnMut(&people::UnitRow)) {
        if season == self.tallies.season {
            for r in people::rows(self, season, &self.tallies.teams.rows()) {
                f(&r);
            }
            return;
        }
        let _ = self.tally_each("people", season, |_, r| {
            if let Some(u) = file::decode::<people::UnitRow>(r) {
                f(&u)
            }
        });
    }

    /// A team's season of calls.
    pub fn tally_call(&self, level: Level, season: i32, team: u32) -> Option<calls::Season> {
        let k = calls::key(level, team);
        if season == self.tallies.season {
            return self.tallies.calls.seasons.get(&k).cloned();
        }
        let mut out = None;
        let _ = self.tally_get("calls", season, &[k], |_, r| out = file::decode(r));
        out
    }

    /// Every team's season of calls.
    pub fn tally_calls(&self, season: i32, mut f: impl FnMut(&calls::Season)) {
        if season == self.tallies.season {
            for (_, s) in self.tallies.calls.rows() {
                f(s);
            }
            return;
        }
        let _ = self.tally_each("calls", season, |_, r| {
            if let Some(s) = file::decode::<calls::Season>(r) {
                f(&s)
            }
        });
    }

    /// Every team's season of calls read as any lighter view of
    /// `calls::Season` (the sums and grid without the rows).
    pub fn tally_calls_as<T: serde::de::DeserializeOwned>(&self, season: i32, mut f: impl FnMut(u64, T)) {
        if season == self.tallies.season {
            for (k, s) in self.tallies.calls.rows() {
                if let Some(t) = file::decode::<T>(&file::encode(s)) {
                    f(k, t)
                }
            }
            return;
        }
        let _ = self.tally_each("calls", season, |k, r| {
            if let Some(s) = file::decode::<T>(r) {
                f(k, s)
            }
        });
    }

    /// One game's record by its key.
    pub fn tally_game_row(&self, season: i32, key: u64) -> Option<games::GameRow> {
        let mut out = None;
        self.tally_games(season, Some(&[key]), |_, g| out = Some(g));
        out
    }

    /// The classes as they stand.
    pub fn tally_classes(&self) -> &classes::Store {
        &self.tallies.classes
    }
}
