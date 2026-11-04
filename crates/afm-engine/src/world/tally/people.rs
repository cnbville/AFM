//! The people tally (`people/<season>.tally`): each coach's unit results
//! for the season (item 39, judging coaches): the head coach's team, the
//! offensive coordinator's offense, the defensive coordinator's defense,
//! at every college division and the NFL, each number with its rank at
//! the level. Worked out when the season closes, from the teams tally and
//! who held each job that season (`units::Charge`).

use std::collections::HashMap;

use crate::world::World;
use crate::world::coach::Role;
use crate::world::folk::Team;
use crate::world::units::Charge;

use super::Level;
use super::teams::{Season as TeamSeason, Situ};

/// The numbers a unit is judged by, in order. Append only.
pub const STATS: [&str; 10] = [
    "points a game",
    "yards a play",
    "EPA a play",
    "success rate",
    "explosive play rate",
    "turnovers a game",
    "third down rate",
    "points a scoring chance",
    "havoc rate",
    "three and outs a game",
];

/// Whether more is better for the offense (a defense is judged the other way).
pub const UP: [bool; 10] = [true, true, true, true, true, false, true, true, false, false];

/// One coach's unit season.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct UnitRow {
    pub coach: u32,
    pub season: i32,
    /// 0 college, 1 NFL.
    pub level: u8,
    pub team: u32,
    /// The college division or "NFL": who the ranks are among.
    pub group: String,
    pub role: String,
    /// 0 offense, 1 defense, 2 the whole team (a head coach).
    pub unit: u8,
    pub games: u32,
    pub wins: u32,
    pub losses: u32,
    /// `STATS` for the unit (a defense's are what it allowed; a head
    /// coach's are his offense's less his defense's).
    pub vals: Vec<f32>,
    /// Each stat's rank at the level (1 best) and of how many teams.
    pub ranks: Vec<(u16, u16)>,
}

fn sum(s: &TeamSeason, side: &str, keys: &[&str]) -> Situ {
    let mut out = [0f32; 8];
    for k in keys {
        let v = s.get(&format!("{side}|{k}"));
        for i in 0..8 {
            out[i] += v[i];
        }
    }
    out
}

/// A side's numbers (`side`: "o" its offense, "d" what its defense allowed).
pub fn unit_stats(s: &TeamSeason, side: &str) -> Vec<f32> {
    let g = s.games.max(1) as f32;
    let snaps = sum(s, side, &["kind|run", "kind|pass"]);
    let third = sum(
        s,
        side,
        &[
            "third|1|run",
            "third|1|pass",
            "third|2-3|run",
            "third|2-3|pass",
            "third|4-6|run",
            "third|4-6|pass",
            "third|7-10|run",
            "third|7-10|pass",
            "third|11+|run",
            "third|11+|pass",
        ],
    );
    let explosive = sum(s, side, &["explosive|run", "explosive|pass"]);
    let (adv, points) = if side == "o" { (&s.adv, s.points_for) } else { (&s.adv_against, s.points_against) };
    let other = if side == "o" { &s.adv_against } else { &s.adv };
    let plays = snaps[0].max(1.0);
    let trips = adv.trips40.max(1) as f32;
    vec![
        points as f32 / g,
        snaps[1] / plays,
        snaps[2] / plays,
        snaps[3] / plays,
        explosive[0] / plays,
        snaps[6] / g,
        (third[4] + third[5]) / third[0].max(1.0),
        (7 * adv.trip_td + 3 * adv.trip_fg) as f32 / trips,
        // Havoc a side's offense suffered is what the other side's defense made.
        other.havoc as f32 / plays,
        adv.three_outs as f32 / g,
    ]
}

/// Ranks each value among the pool (1 best).
fn ranks(pool: &[Vec<f32>], mine: &[f32], up: &[bool]) -> Vec<(u16, u16)> {
    (0..mine.len())
        .map(|k| {
            let better = pool.iter().filter(|v| if up[k] { v[k] > mine[k] } else { v[k] < mine[k] }).count();
            ((better + 1) as u16, pool.len() as u16)
        })
        .collect()
}

/// The group a team's season is ranked in.
fn group(w: &World, s: &TeamSeason) -> String {
    match s.level {
        1 => "NFL".into(),
        _ => w.programs.get(s.team as usize).map_or(String::new(), |p| p.division.name().to_string()),
    }
}

/// The season's rows, from its teams.
pub fn rows(w: &World, season: i32, teams: &[(u64, &TeamSeason)]) -> Vec<UnitRow> {
    let charge = Charge::new(w, season);
    // Each group's teams: (row, [offense, defense, margins]).
    type Pool = Vec<(usize, [Vec<f32>; 3])>;
    let mut pools: HashMap<String, Pool> = HashMap::new();
    for (k, (_, s)) in teams.iter().enumerate() {
        let o = unit_stats(s, "o");
        let d = unit_stats(s, "d");
        let team: Vec<f32> = o.iter().zip(&d).map(|(a, b)| a - b).collect();
        pools.entry(group(w, s)).or_default().push((k, [o, d, team]));
    }
    let up_o = UP.to_vec();
    let up_d: Vec<bool> = UP.iter().map(|u| !u).collect();
    // A head coach's margins (his offense's less what his defense allowed)
    // read the offense's way: fewer turnovers, havoc and three and outs
    // given than taken is better.
    let up_t = UP.to_vec();
    let mut out = Vec::new();
    for (g, list) in &pools {
        for (unit, role, up) in [
            (0usize, Role::OffensiveCoordinator, &up_o),
            (1, Role::DefensiveCoordinator, &up_d),
            (2, Role::HeadCoach, &up_t),
        ] {
            let pool: Vec<Vec<f32>> = list.iter().map(|x| x.1[unit].clone()).collect();
            for (k, vals) in list {
                let s = teams[*k].1;
                let t = match s.level {
                    1 => Team::Nfl(s.team as usize),
                    _ => Team::College(s.team as usize),
                };
                let Some(coach) = charge.get(t, role) else { continue };
                let mine = &vals[unit];
                out.push(UnitRow {
                    coach: coach as u32,
                    season,
                    level: s.level,
                    team: s.team,
                    group: g.clone(),
                    role: role.name().to_string(),
                    unit: unit as u8,
                    games: s.games,
                    wins: s.wins,
                    losses: s.losses,
                    vals: mine.clone(),
                    ranks: ranks(&pool, mine, up),
                });
            }
        }
    }
    out.sort_by_key(|r| (r.coach, r.unit, r.team));
    out
}

/// A row's key: the coach, then the unit and the team (a coach who held two
/// jobs in a season has a row for each).
pub fn key(r: &UnitRow) -> u64 {
    (r.coach as u64) << 32 | (r.unit as u64) << 28 | (r.level as u64) << 24 | (r.team as u64 & 0xFF_FFFF)
}

/// The level a row's team is at.
pub fn level(r: &UnitRow) -> Level {
    if r.level == 1 { Level::Nfl } else { Level::College }
}
