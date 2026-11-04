//! The squads tally (`squads/<season>.tally`, lane NB7, Data overhaul
//! items 16-23): one small row per team a season at every level, high
//! school too, with what the team pages, leaderboards and the power rating
//! read across many seasons without opening the heavy situation cells of
//! the teams tally:
//! - both sides' box scores summed (`BOX`) and the advanced numbers for and
//!   against (`games::Adv`);
//! - the fumbles and passes defensed that expected turnovers read (`LUCK`);
//! - college and the NFL: the offense's snaps, dropbacks and expected
//!   dropbacks by situation (`TEND`, pass rate over expected);
//! - its kicks and returns (`ST`);
//! - who started each game on each side (returning starters);
//! - set when the season closes: its closing power rating
//!   (`world::power`) and what its roster brought (`TALENT_COLLEGE`,
//!   `TALENT_HS`).

use std::collections::{BTreeMap, HashMap};

use crate::game::stats::{GameStats, PlayRecord, PlayType, TeamStats};

use super::Level;
use super::games::Adv;
use super::teams::add_adv;

/// A team box's numbers, in order (append only: files keep them by place).
pub const BOX: [&str; 34] = [
    "points",
    "plays",
    "yards",
    "pass_yards",
    "rush_yards",
    "first_downs",
    "third_att",
    "third_conv",
    "fourth_att",
    "fourth_conv",
    "penalties",
    "penalty_yds",
    "turnovers",
    "time_of_possession",
    "sacks",
    "red_zone_trips",
    "red_zone_tds",
    "epa",
    "successes",
    "explosive",
    "dropbacks",
    "pressures",
    "drives",
    "start_sum",
    "punts",
    "fg_att",
    "fg_made",
    "att",
    "cmp",
    "carries",
    "ints",
    "fumbles_lost",
    "clock_possession",
    "games",
];

/// A box as `BOX`'s numbers (one game).
pub fn box_values(t: &TeamStats) -> Vec<f32> {
    vec![
        t.points as f32,
        t.plays as f32,
        t.yards as f32,
        t.pass_yards as f32,
        t.rush_yards as f32,
        t.first_downs as f32,
        t.third_att as f32,
        t.third_conv as f32,
        t.fourth_att as f32,
        t.fourth_conv as f32,
        t.penalties as f32,
        t.penalty_yds as f32,
        t.turnovers as f32,
        t.time_of_possession as f32,
        t.sacks as f32,
        t.red_zone_trips as f32,
        t.red_zone_tds as f32,
        t.epa as f32,
        t.successes as f32,
        t.explosive as f32,
        t.dropbacks as f32,
        t.pressures as f32,
        t.drives as f32,
        t.start_sum as f32,
        t.punts as f32,
        t.fg_att as f32,
        t.fg_made as f32,
        t.att as f32,
        t.cmp as f32,
        t.carries as f32,
        t.ints as f32,
        t.fumbles_lost as f32,
        t.clock_possession as f32,
        1.0,
    ]
}

/// A summed box's number by name (zero when the file is older).
pub fn box_get(b: &[f32], name: &str) -> f64 {
    BOX.iter().position(|&n| n == name).and_then(|k| b.get(k)).map_or(0.0, |&v| v as f64)
}

fn add_vec(into: &mut Vec<f32>, from: &[f32]) {
    if into.len() < from.len() {
        into.resize(from.len(), 0.0);
    }
    for (a, b) in into.iter_mut().zip(from) {
        *a += b;
    }
}

/// The counts expected turnovers read (Bill Connelly's turnover luck:
/// about half of all fumbles go to either side, and a level's own share of
/// passes defensed are picked off): its fumbles and those it lost, the
/// other side's fumbles and those it lost, passes its defense defensed
/// (picks and breakups) and its picks, passes defensed against its offense
/// and the picks it threw.
pub const LUCK: [&str; 8] =
    ["fumbles", "fumbles_lost", "opp_fumbles", "opp_fumbles_lost", "pd", "picks", "pd_against", "picks_thrown"];

/// A side's kicks and returns: its punts (gross and net yards, inside the
/// 20), kickoffs and touchbacks, its kick and punt returns and their
/// yards, field goals and extra points tried and made, and the kicks it
/// blocked.
pub const ST: [&str; 15] = [
    "punts",
    "punt_gross",
    "punt_net",
    "punts_in20",
    "kickoffs",
    "kickoff_tb",
    "kr",
    "kr_yds",
    "pr",
    "pr_yds",
    "fg_att",
    "fg_made",
    "xp_att",
    "xp_made",
    "blocked",
];

fn kicks(g: &GameStats, side: usize) -> Vec<f32> {
    let mut v = vec![0f32; ST.len()];
    for p in g.plays.iter().filter(|p| !p.nullified) {
        let kicking = p.offense.min(1) == side;
        match p.kind {
            PlayType::Punt if kicking && !p.blocked => {
                v[0] += 1.0;
                v[1] += p.kick_distance as f32;
                v[2] += (p.kick_distance - p.return_yards - if p.touchback { 20.0 } else { 0.0 }) as f32;
                v[3] += p.inside20 as u8 as f32;
            }
            PlayType::Punt if !kicking && !p.blocked && p.returner.is_some() && !p.fair_catch && !p.touchback => {
                v[8] += 1.0;
                v[9] += p.return_yards as f32;
            }
            PlayType::Kickoff if kicking => {
                v[4] += 1.0;
                v[5] += p.touchback as u8 as f32;
            }
            PlayType::Kickoff if p.returner.is_some() && !p.fair_catch && !p.touchback => {
                v[6] += 1.0;
                v[7] += p.return_yards as f32;
            }
            PlayType::FieldGoal if kicking => {
                v[10] += 1.0;
                v[11] += p.kick_good as u8 as f32;
            }
            PlayType::ExtraPoint if kicking => {
                v[12] += 1.0;
                v[13] += p.kick_good as u8 as f32;
            }
            _ => {}
        }
        if !kicking && p.blocked && matches!(p.kind, PlayType::Punt | PlayType::FieldGoal | PlayType::ExtraPoint) {
            v[14] += 1.0;
        }
    }
    v
}

/// The situations the offense's calls are read in, by code (append only).
/// Each keeps scrimmage snaps, dropbacks and the expected dropbacks
/// (`expect::expected_pass`). Code 1 is nflfastR's neutral early downs:
/// first and second down, a win chance from 20 to 80%, outside the last
/// two minutes of a half.
pub const TEND: [&str; 21] = [
    "every snap",
    "early downs, neutral",
    "1st and 10",
    "2nd and 7+",
    "2nd and 4-6",
    "2nd and 1-3",
    "3rd and 7+",
    "3rd and 4-6",
    "3rd and 1-3",
    "4th down",
    "red zone",
    "goal to go",
    "ahead by 9+",
    "within 8",
    "behind by 9+",
    "last two minutes of a half",
    "1st quarter",
    "2nd quarter",
    "3rd quarter",
    "4th quarter",
    "overtime",
];

fn scrimmage(p: &PlayRecord) -> bool {
    !p.nullified && matches!(p.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble)
}

/// The `TEND` codes a scrimmage snap counts under.
pub fn tend_codes(p: &PlayRecord) -> Vec<u8> {
    let mut c = vec![0u8];
    let late = matches!(p.quarter, 2 | 4) && p.clock <= 120.0;
    if (1..=2).contains(&p.down) && (0.2..=0.8).contains(&p.wp_before) && !late {
        c.push(1);
    }
    let t = p.togo;
    let band = |hi: u8| {
        if t >= 7.0 {
            hi
        } else if t >= 4.0 {
            hi + 1
        } else {
            hi + 2
        }
    };
    match p.down {
        1 if (t - 10.0).abs() < 0.5 => c.push(2),
        2 => c.push(band(3)),
        3 => c.push(band(6)),
        4 => c.push(9),
        _ => {}
    }
    if p.yardline_100 <= 20.0 {
        c.push(10);
    }
    if p.yardline_100 <= 10.0 && p.togo >= p.yardline_100 - 0.01 {
        c.push(11);
    }
    let o = p.offense.min(1);
    let m = p.score[o] - p.score[1 - o];
    c.push(if m >= 9 {
        12
    } else if m <= -9 {
        14
    } else {
        13
    });
    if late {
        c.push(15);
    }
    c.push(if p.quarter >= 5 { 20 } else { 15 + p.quarter.max(1) });
    c
}

/// Each side's starters on each unit: on the field for its first snap with
/// the ball (offense) and its first snap without it (defense), by id.
pub fn starters_by_unit(g: &GameStats) -> [[Vec<u32>; 2]; 2] {
    let mut out: [[Vec<u32>; 2]; 2] = Default::default();
    let mut seen = [[false; 2]; 2];
    for p in g.plays.iter().filter(|p| scrimmage(p)) {
        let o = p.offense.min(1);
        for (unit, side) in [(0usize, o), (1usize, 1 - o)] {
            if !seen[side][unit] {
                seen[side][unit] = true;
                let mut ids = p.on_field[unit].clone();
                ids.sort_unstable();
                out[side][unit] = ids;
            }
        }
        if seen.iter().all(|s| s[0] && s[1]) {
            break;
        }
    }
    out
}

/// One side's game.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Game {
    /// Its box and the other side's (`BOX`).
    pub boxes: [Vec<f32>; 2],
    /// `LUCK`.
    pub luck: Vec<f32>,
    /// Its offense's calls against the expected pass rate (`TEND`; college
    /// and the NFL).
    pub tend: Vec<(u8, [f32; 3])>,
    /// Who started on offense and on defense.
    pub starters: [Vec<u32>; 2],
    /// `ST`.
    pub st: Vec<f32>,
}

/// A side's game. `tend` false (high school) leaves out the calls by
/// situation.
pub fn game(g: &GameStats, side: usize, tend: bool) -> Game {
    let level = g.level.unwrap_or(crate::player::Level::NFL);
    let mut luck = vec![0f32; LUCK.len()];
    let mut cells: BTreeMap<u8, [f32; 3]> = BTreeMap::new();
    for p in g.plays.iter().filter(|p| scrimmage(p)) {
        let mine = p.offense.min(1) == side;
        if p.fumbled.is_some() {
            luck[if mine { 0 } else { 2 }] += 1.0;
            if p.fumble_lost {
                luck[if mine { 1 } else { 3 }] += 1.0;
            }
        }
        if p.kind == PlayType::Pass && (p.interception || p.pbu.is_some()) {
            luck[if mine { 6 } else { 4 }] += 1.0;
            if p.interception {
                luck[if mine { 7 } else { 5 }] += 1.0;
            }
        }
        if tend && mine {
            let xp = super::expect::expected_pass(level, p) as f32;
            let db = (p.kind != PlayType::Run) as u8 as f32;
            for c in tend_codes(p) {
                let e = cells.entry(c).or_default();
                e[0] += 1.0;
                e[1] += db;
                e[2] += xp;
            }
        }
    }
    let [a, b] = starters_by_unit(g);
    Game {
        boxes: [box_values(&g.team[side]), box_values(&g.team[1 - side])],
        luck,
        tend: cells.into_iter().collect(),
        starters: if side == 0 { a } else { b },
        st: kicks(g, side),
    }
}

/// A college season's talent numbers: the recruiting services' average
/// stars of the men who played, on the whole, on offense and on defense,
/// how many of each, and the share of them rated four or five stars (Bud
/// Elliott's blue-chip ratio).
pub const TALENT_COLLEGE: [&str; 7] = ["stars", "stars_off", "stars_def", "men", "men_off", "men_def", "blue_chips"];
/// A high school season's: its regular starters (half its games or more)
/// on offense and on defense, and the seniors among them.
pub const TALENT_HS: [&str; 4] = ["starters_off", "starters_def", "seniors_off", "seniors_def"];

/// A regular starter: started half his team's games or more on that side.
pub fn regular(starts: u8, games: u32) -> bool {
    starts as u32 * 2 >= games.max(1)
}

/// One team's season.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Season {
    /// 0 college, 1 NFL, 2 high school.
    pub level: u8,
    pub team: u32,
    pub games: u32,
    pub wins: u32,
    pub losses: u32,
    pub points_for: u32,
    pub points_against: u32,
    /// Its box and its opponents' summed (`BOX`).
    pub boxes: [Vec<f32>; 2],
    /// The advanced numbers for and against (`games::Adv`).
    pub adv: Adv,
    pub adv_against: Adv,
    /// `LUCK`, summed.
    pub luck: Vec<f32>,
    /// `ST`, summed.
    pub st: Vec<f32>,
    /// Its offense's snaps, dropbacks and expected dropbacks by situation
    /// (`TEND`).
    pub tend: Vec<(u8, [f32; 3])>,
    /// Every man who started a game: (id, games started on offense, on
    /// defense), by id.
    pub starters: Vec<(u32, u8, u8)>,
    /// The season's closing power rating (`world::power::closing`):
    /// offense, defense, games rated.
    pub rating: Vec<f32>,
    /// What the roster brought (`TALENT_COLLEGE`, `TALENT_HS`; the NFL
    /// keeps none).
    pub talent: Vec<f32>,
}

impl Season {
    /// A number of its own box (side 0) or its opponents' (side 1).
    pub fn box_of(&self, side: usize, name: &str) -> f64 {
        box_get(&self.boxes[side.min(1)], name)
    }

    /// A `LUCK` count.
    pub fn luck_of(&self, name: &str) -> f64 {
        LUCK.iter().position(|&n| n == name).and_then(|k| self.luck.get(k)).map_or(0.0, |&v| v as f64)
    }

    /// An `ST` count.
    pub fn st_of(&self, name: &str) -> f64 {
        ST.iter().position(|&n| n == name).and_then(|k| self.st.get(k)).map_or(0.0, |&v| v as f64)
    }

    /// A `TEND` situation's snaps, dropbacks and expected dropbacks.
    pub fn tend_of(&self, code: u8) -> [f32; 3] {
        self.tend.iter().find(|x| x.0 == code).map_or([0.0; 3], |x| x.1)
    }

    /// A talent number by name (`TALENT_COLLEGE` or `TALENT_HS`).
    pub fn talent_of(&self, name: &str) -> Option<f64> {
        let names: &[&str] = if self.level == Level::Hs as u8 { &TALENT_HS } else { &TALENT_COLLEGE };
        names.iter().position(|&n| n == name).and_then(|k| self.talent.get(k)).map(|&v| v as f64)
    }

    /// The men who started half its games or more on a side (0 offense, 1
    /// defense).
    pub fn regulars(&self, unit: usize) -> Vec<u32> {
        self.starters.iter().filter(|s| regular(if unit == 0 { s.1 } else { s.2 }, self.games)).map(|s| s.0).collect()
    }
}

fn add_starters(into: &mut Vec<(u32, u8, u8)>, ids: &[Vec<u32>; 2]) {
    for (unit, list) in ids.iter().enumerate() {
        for &id in list {
            let k = match into.binary_search_by_key(&id, |x| x.0) {
                Ok(k) => k,
                Err(k) => {
                    into.insert(k, (id, 0, 0));
                    k
                }
            };
            let e = &mut into[k];
            if unit == 0 {
                e.1 = e.1.saturating_add(1);
            } else {
                e.2 = e.2.saturating_add(1);
            }
        }
    }
}

pub fn key(level: Level, team: u32) -> u64 {
    super::teams::key(level, team)
}

/// The open season's squads.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Open {
    pub seasons: HashMap<u64, Season>,
}

impl Open {
    /// A side's game, its and the other side's advanced numbers and the
    /// score (its, theirs).
    pub fn add(&mut self, level: Level, team: u32, g: Game, adv: [&Adv; 2], score: [i32; 2]) {
        let s = self.seasons.entry(key(level, team)).or_insert_with(|| Season {
            level: level as u8,
            team,
            ..Default::default()
        });
        s.games += 1;
        s.wins += (score[0] > score[1]) as u32;
        s.losses += (score[0] < score[1]) as u32;
        s.points_for += score[0].max(0) as u32;
        s.points_against += score[1].max(0) as u32;
        add_adv(&mut s.adv, adv[0]);
        add_adv(&mut s.adv_against, adv[1]);
        let [mine, theirs] = g.boxes;
        add_vec(&mut s.boxes[0], &mine);
        add_vec(&mut s.boxes[1], &theirs);
        add_vec(&mut s.luck, &g.luck);
        add_vec(&mut s.st, &g.st);
        for (c, v) in g.tend {
            match s.tend.binary_search_by_key(&c, |x| x.0) {
                Ok(k) => {
                    for i in 0..3 {
                        s.tend[k].1[i] += v[i];
                    }
                }
                Err(k) => s.tend.insert(k, (c, v)),
            }
        }
        add_starters(&mut s.starters, &g.starters);
    }

    pub fn rows(&self) -> Vec<(u64, &Season)> {
        let mut v: Vec<(u64, &Season)> = self.seasons.iter().map(|(&k, s)| (k, s)).collect();
        v.sort_by_key(|x| x.0);
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_box_reads_back_by_name() {
        let t = TeamStats { points: 31, plays: 70, ints: 2, ..Default::default() };
        let v = box_values(&t);
        assert_eq!(v.len(), BOX.len());
        assert_eq!(box_get(&v, "points"), 31.0);
        assert_eq!(box_get(&v, "plays"), 70.0);
        assert_eq!(box_get(&v, "ints"), 2.0);
        assert_eq!(box_get(&v, "games"), 1.0);
        assert_eq!(box_get(&v[..3], "games"), 0.0);
    }

    #[test]
    fn a_season_sums_its_games() {
        let mut o = Open::default();
        let g = |pts: i32, ids: [Vec<u32>; 2]| Game {
            boxes: [box_values(&TeamStats { points: pts, ..Default::default() }), box_values(&TeamStats::default())],
            luck: vec![1.0; LUCK.len()],
            tend: vec![(0, [60.0, 30.0, 28.5]), (2, [20.0, 9.0, 9.5])],
            starters: ids,
            st: vec![2.0; ST.len()],
        };
        let a = Adv::default();
        o.add(Level::Hs, 7, g(28, [vec![1, 2], vec![3]]), [&a, &a], [28, 14]);
        o.add(Level::Hs, 7, g(10, [vec![2, 4], vec![3]]), [&a, &a], [10, 17]);
        let s = &o.seasons[&key(Level::Hs, 7)];
        assert_eq!((s.games, s.wins, s.losses, s.points_for), (2, 1, 1, 38));
        assert_eq!(s.box_of(0, "points"), 38.0);
        assert_eq!(s.box_of(0, "games"), 2.0);
        assert_eq!(s.luck_of("picks"), 2.0);
        assert_eq!(s.st_of("punts"), 4.0);
        assert_eq!(s.tend_of(0), [120.0, 60.0, 57.0]);
        assert_eq!(s.starters, vec![(1, 1, 0), (2, 2, 0), (3, 0, 2), (4, 1, 0)]);
        assert_eq!(s.regulars(0), vec![1, 2, 4]);
        assert_eq!(s.regulars(1), vec![3]);
    }

    #[test]
    fn a_snap_counts_under_its_situations() {
        let mut p = PlayRecord::new(1, 4, 90.0, 0, PlayType::Pass);
        p.down = 3;
        p.togo = 8.0;
        p.yardline_100 = 15.0;
        p.score = [10, 24];
        p.wp_before = 0.1;
        let c = tend_codes(&p);
        assert_eq!(c, vec![0, 6, 10, 14, 15, 19]);
    }
}
