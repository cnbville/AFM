//! Lane NB10 (Data overhaul item 29, board 58): each team's season as the
//! Trends page reads it, so the league's 30 years and a team's ten come
//! from one small file a season instead of the teams file's heavy cells.
//!
//! One row a team a season at every level:
//! - `style`: how it played, counted off the teams tally's cells (college
//!   and the NFL): snaps, runs, dropbacks, aimed throws, deep throws (20+
//!   air yards), throws behind the line, play action, the backfield, motion,
//!   no huddle, RPOs, personnel, the blitz, man coverage, pressure, nickel
//!   and dime, and runs into eight or more in the box. `STYLE` names each,
//!   with the ball first and then without it (what was done to it).
//! - `pass`: its passers' dropbacks, attempts, aimed throws, air yards and
//!   time to throw, from the players tally's depth counts (college and the
//!   NFL), for the depth of target and time to throw.
//! - `sizes`: the height and weight of the men who started its games, by
//!   position, each weighed by his starts: what a roster's listed sizes
//!   say (every level, high school too). Written when the season closes, as
//!   listed then: a body grows and shrinks, so a past season's sizes can
//!   only be read off at the time.
//!
//! Only reads the open stores and the people: no draws, nothing in the
//! world reads it (the same-seed test, `tests/tallies.rs`).

use std::collections::HashMap;

use super::depth::{self, Dc};
use super::{Level, teams};
use crate::world::World;

/// The style counts, each side's in this order (with the ball, then
/// without it).
pub const STYLE: [&str; 22] = [
    "snaps",
    "runs",
    "dropbacks",
    "aimed",
    "deep",
    "behind",
    "play_action",
    "gun",
    "pistol",
    "under_center",
    "motion",
    "no_huddle",
    "rpo",
    "p11",
    "p12",
    "p21",
    "blitz",
    "man",
    "pressured",
    "nickel",
    "dime",
    "box8",
];

/// The passers' counts, summed by the team he last played for that season.
pub const PASS: [&str; 5] = ["db", "att", "aimed", "air", "ttt"];

/// One team's season.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Season {
    /// 0 college, 1 NFL, 2 high school (`Level`).
    pub level: u8,
    pub team: u32,
    pub games: u32,
    /// `STYLE` with the ball, then `STYLE` without it; empty in high school.
    pub style: Vec<f32>,
    /// `PASS`; empty in high school.
    pub pass: Vec<f32>,
    /// (position code, [starts, inches x starts, pounds x starts]), by code.
    pub sizes: Vec<(u8, [f32; 3])>,
}

impl Season {
    /// A style count: `side` 0 with the ball, 1 without it.
    pub fn style_of(&self, side: usize, name: &str) -> f64 {
        let Some(k) = STYLE.iter().position(|s| *s == name) else { return 0.0 };
        self.style.get(side * STYLE.len() + k).copied().unwrap_or(0.0) as f64
    }

    pub fn pass_of(&self, name: &str) -> f64 {
        PASS.iter().position(|s| *s == name).and_then(|k| self.pass.get(k)).copied().unwrap_or(0.0) as f64
    }

    /// Its starters' (starts, average inches, average pounds) at some
    /// positions (by code).
    pub fn size_of(&self, positions: &[u8]) -> Option<(f64, f64, f64)> {
        let (mut n, mut h, mut w) = (0.0, 0.0, 0.0);
        for (p, v) in &self.sizes {
            if positions.contains(p) {
                n += v[0] as f64;
                h += v[1] as f64;
                w += v[2] as f64;
            }
        }
        (n > 0.0).then(|| (n, h / n, w / n))
    }
}

pub fn key(level: Level, team: u32) -> u64 {
    teams::key(level, team)
}

/// One side's style counts from a team's cells (`p`: "o|" or "d|").
fn style_side(t: &teams::Season, p: &str) -> [f32; STYLE.len()] {
    let mut out = [0f32; STYLE.len()];
    let mut add = |k: usize, v: f32| out[k] += v;
    for (key, c) in t.cells.range(p.to_string()..) {
        let Some(rest) = key.strip_prefix(p) else { break };
        let plays = c[0];
        let parts: Vec<&str> = rest.split('|').collect();
        match parts.as_slice() {
            ["kind", "run"] => {
                add(0, plays);
                add(1, plays);
            }
            ["kind", "pass"] => {
                add(0, plays);
                add(2, plays);
            }
            ["depth", band] => {
                add(3, plays);
                if band.starts_with("deep") {
                    add(4, plays);
                } else if *band == "behind" {
                    add(5, plays);
                }
            }
            ["play action", "true"] => add(6, plays),
            ["backfield", "gun", _] => add(7, plays),
            ["backfield", "pistol", _] => add(8, plays),
            ["backfield", "under center", _] => add(9, plays),
            ["motion", m, _] if *m != "none" => add(10, plays),
            ["tempo", "no huddle", _] => add(11, plays),
            ["rpo", _] => add(12, plays),
            ["personnel", "11", _] => add(13, plays),
            ["personnel", "12", _] => add(14, plays),
            ["personnel", "21", _] => add(15, plays),
            ["blitz", _, "true"] => add(16, plays),
            ["coverage", "man"] => add(17, plays),
            ["pressured", "true"] => add(18, plays),
            ["package", "nickel", _] => add(19, plays),
            ["package", "dime", _] => add(20, plays),
            ["box", n, "run"] if n.parse::<u32>().is_ok_and(|n| n >= 8) => add(21, plays),
            _ => {}
        }
    }
    out
}

/// Every team's row of the season being counted, read off the open stores
/// and the people as they are now (the season closes on these).
pub fn rows(w: &World) -> Vec<(u64, Season)> {
    let t = &w.tallies;
    let mut out: HashMap<u64, Season> = HashMap::new();
    // Style: college and the NFL keep cells.
    for (k, s) in t.teams.rows() {
        let e = out.entry(k).or_insert_with(|| Season { level: s.level, team: s.team, ..Default::default() });
        e.games = s.games;
        let mut v = style_side(s, "o|").to_vec();
        v.extend(style_side(s, "d|"));
        e.style = v;
    }
    // Passing, from each passer's depth counts.
    for (_, p) in t.players.rows() {
        let d = |c: Dc| depth::get(&p.depth, c as u16) as f32;
        if d(Dc::Db) <= 0.0 {
            continue;
        }
        let level = if p.level == Level::Nfl as u8 { Level::Nfl } else { Level::College };
        let e = out.entry(key(level, p.team)).or_insert_with(|| Season {
            level: level as u8,
            team: p.team,
            ..Default::default()
        });
        if e.pass.len() < PASS.len() {
            e.pass.resize(PASS.len(), 0.0);
        }
        for (k, c) in [Dc::Db, Dc::Att, Dc::Aimed, Dc::Air, Dc::Ttt].into_iter().enumerate() {
            e.pass[k] += d(c);
        }
    }
    // Sizes: every man who started a game, by his listed position now.
    let by_id: HashMap<u32, usize> = w.people.iter().enumerate().map(|(k, p)| (p.player.id, k)).collect();
    for (k, s) in t.squads.rows() {
        let e = out.entry(k).or_insert_with(|| Season { level: s.level, team: s.team, ..Default::default() });
        e.games = e.games.max(s.games);
        let mut sizes: Vec<(u8, [f32; 3])> = Vec::new();
        for &(id, o, d) in &s.starters {
            let Some(&p) = by_id.get(&id) else { continue };
            let pl = &w.people[p].player;
            let starts = (o as f32) + (d as f32);
            if starts <= 0.0 {
                continue;
            }
            let code = pl.position as u8;
            let v = match sizes.binary_search_by_key(&code, |x| x.0) {
                Ok(i) => &mut sizes[i].1,
                Err(i) => {
                    sizes.insert(i, (code, [0.0; 3]));
                    &mut sizes[i].1
                }
            };
            v[0] += starts;
            v[1] += starts * pl.body.height as f32;
            v[2] += starts * pl.body.weight as f32;
        }
        e.sizes = sizes;
    }
    let mut v: Vec<(u64, Season)> = out.into_iter().collect();
    v.sort_by_key(|x| x.0);
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_teams_cells_read_as_its_style() {
        let mut t = teams::Season { level: 0, team: 3, games: 2, ..Default::default() };
        let mut put = |k: &str, n: f32| {
            let mut c = teams::Situ::default();
            c[0] = n;
            t.cells.insert(k.to_string(), c);
        };
        put("o|kind|run", 60.0);
        put("o|kind|pass", 70.0);
        put("o|depth|deep (20+)", 8.0);
        put("o|depth|short (0-9)", 40.0);
        put("o|depth|behind", 6.0);
        put("o|backfield|gun|pass", 50.0);
        put("o|backfield|gun|run", 20.0);
        put("o|motion|none|run", 30.0);
        put("o|motion|jet motion|run", 12.0);
        put("o|box|8|run", 9.0);
        put("o|box|7|run", 30.0);
        put("d|kind|pass", 66.0);
        put("d|blitz|3|true", 11.0);
        put("d|blitz|3|false", 20.0);
        put("d|coverage|man", 25.0);
        let o = style_side(&t, "o|");
        let d = style_side(&t, "d|");
        let at = |n: &str| STYLE.iter().position(|s| *s == n).unwrap();
        assert_eq!(o[at("snaps")], 130.0);
        assert_eq!(o[at("aimed")], 54.0);
        assert_eq!(o[at("deep")], 8.0);
        assert_eq!(o[at("behind")], 6.0);
        assert_eq!(o[at("gun")], 70.0);
        assert_eq!(o[at("motion")], 12.0);
        assert_eq!(o[at("box8")], 9.0);
        assert_eq!(o[at("blitz")], 0.0);
        assert_eq!(d[at("dropbacks")], 66.0);
        assert_eq!(d[at("blitz")], 11.0);
        assert_eq!(d[at("man")], 25.0);
        let mut s = Season { style: o.to_vec(), ..Default::default() };
        s.style.extend(d);
        assert_eq!(s.style_of(1, "blitz"), 11.0);
        assert_eq!(s.style_of(0, "deep"), 8.0);
        s.sizes = vec![(4, [10.0, 770.0, 3100.0]), (5, [30.0, 2280.0, 9300.0])];
        let (n, h, w) = s.size_of(&[4, 5]).unwrap();
        assert_eq!(n, 40.0);
        assert!((h - 76.25).abs() < 1e-9 && (w - 310.0).abs() < 1e-9);
        assert!(s.size_of(&[0]).is_none());
    }
}
