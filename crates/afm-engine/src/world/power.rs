//! The Data section's power rating (Data overhaul item 23, lane NB7): the
//! world's own rating of every team from its results so far, a preseason
//! projection, each coming game's win chance, and playoff and bowl odds.
//!
//! Like SP+ and FPI it reads only what is public: scores, the schedule,
//! last season's ratings, the production coming back and the recruiting
//! stars. Never a hidden rating.
//!
//! - The rating: each team's offense (points it scores over an average
//!   team's) and defense (points it keeps off the board), worked out
//!   together from every game's score with the opponent and home field
//!   taken out (least squares, as Massey and SRS do), and pulled toward its
//!   preseason projection by `PRIOR_GAMES` games' worth (as SP+ and FPI
//!   blend their preseason number in until the season's own games outweigh
//!   it). The overall rating is the two added: the points it would beat an
//!   average team by on a neutral field. Home field is fitted on the
//!   games themselves.
//! - The preseason projection: last season's and the season before's
//!   ratings, the share of production coming back and the roster's stars,
//!   with their weights fitted on the world's own past seasons (`project`).
//! - The win chance: the rating gap plus home field against the spread of
//!   real results around the ratings, fitted on the season before's games
//!   game day by game day, each read with only the games before it (`sigma`).
//! - The odds: the rest of the season played 2,000 times with each team's
//!   rating drawn around its number, then each level's own selection: bowl
//!   eligibility at six wins, the College Football Playoff's field by era,
//!   the other divisions' brackets, the NFL's division winners and wild
//!   cards (`odds`).
//!
//! Only reads the world: nothing in the simulation reads it, and the odds
//! draw from their own stream, never the world's.
//!
//! Proposed, not confirmed: `PRIOR_GAMES`, the score a stand-in result is
//! read as (`STAND_IN`), the fallback weights, the committee's order in the
//! odds (`resume`) and the rating spread the odds draw from.

use std::collections::HashMap;

use super::World;
use super::data::Division;
use super::schedule::GameKind;
use super::tally::{self, Level};
use crate::calendar::anchors::CfbPostseason;
use crate::rng::Rng;

/// How many games' worth the preseason projection counts in the rating.
pub const PRIOR_GAMES: f64 = 3.0;
/// The points a result kept without its score is read as (winner, loser).
pub const STAND_IN: [i32; 2] = [24, 14];
/// How many seasons the odds play.
pub const SIMS: usize = 2000;

/// One game of a season: played or still to come.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SGame {
    pub date: i32,
    /// Home, away (program ids, NFL club indices, high school ids).
    pub teams: [u32; 2],
    pub neutral: bool,
    /// A conference, division or district game.
    pub league: bool,
    /// A bowl, playoff or title game (a conference title game counts too).
    pub post: bool,
    pub score: Option<[i32; 2]>,
    /// Played: 0 the home side won, 1 the away side, 2 a tie.
    pub won: Option<u8>,
}

impl SGame {
    pub fn played(&self) -> bool {
        self.won.is_some()
    }

    /// The points as the rating reads them (a result without its score
    /// reads as `STAND_IN`).
    pub fn points(&self) -> Option<[f64; 2]> {
        if let Some(s) = self.score {
            return Some([s[0] as f64, s[1] as f64]);
        }
        match self.won? {
            0 => Some([STAND_IN[0] as f64, STAND_IN[1] as f64]),
            1 => Some([STAND_IN[1] as f64, STAND_IN[0] as f64]),
            _ => Some([17.0, 17.0]),
        }
    }
}

/// A team's rating: points scored over an average team's, points kept off
/// the board, and the games behind it.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Rating {
    pub off: f64,
    pub def: f64,
    pub games: u32,
}

impl Rating {
    /// The points it would beat an average team by on a neutral field.
    pub fn overall(&self) -> f64 {
        self.off + self.def
    }
}

/// A season's ratings.
#[derive(Clone, Debug, Default)]
pub struct Fit {
    pub teams: HashMap<u32, Rating>,
    /// An average team's points a game.
    pub mean: f64,
    /// Home field, in points.
    pub home: f64,
}

impl Fit {
    pub fn get(&self, t: u32) -> Rating {
        self.teams.get(&t).copied().unwrap_or_default()
    }

    /// The home side's expected margin.
    pub fn margin(&self, home: u32, away: u32, neutral: bool) -> f64 {
        self.get(home).overall() - self.get(away).overall() + if neutral { 0.0 } else { self.home }
    }
}

/// The ratings from the season's played games, each team pulled toward its
/// prior (`prior`, an average team when it has none) by `weight` games.
pub fn solve(games: &[SGame], prior: &HashMap<u32, Rating>, weight: f64) -> Fit {
    let rows: Vec<(usize, [f64; 2])> =
        games.iter().enumerate().filter_map(|(k, g)| g.points().filter(|_| g.played()).map(|p| (k, p))).collect();
    let mut ids: Vec<u32> = rows.iter().flat_map(|&(k, _)| games[k].teams).collect();
    ids.extend(prior.keys());
    ids.sort_unstable();
    ids.dedup();
    let at: HashMap<u32, usize> = ids.iter().enumerate().map(|(k, &t)| (t, k)).collect();
    let n = ids.len();
    let p0: Vec<Rating> = ids.iter().map(|t| prior.get(t).copied().unwrap_or_default()).collect();
    let mut off: Vec<f64> = p0.iter().map(|r| r.off).collect();
    let mut def: Vec<f64> = p0.iter().map(|r| r.def).collect();
    let mut played = vec![0u32; n];
    // Each side's line: (team, opponent, points, home sign).
    let mut sides: Vec<(usize, usize, f64, f64)> = Vec::with_capacity(rows.len() * 2);
    for &(k, pts) in &rows {
        let g = &games[k];
        let (h, a) = (at[&g.teams[0]], at[&g.teams[1]]);
        let s = if g.neutral { 0.0 } else { 1.0 };
        sides.push((h, a, pts[0], s));
        sides.push((a, h, pts[1], -s));
        played[h] += 1;
        played[a] += 1;
    }
    let mut mean = if sides.is_empty() { 0.0 } else { sides.iter().map(|x| x.2).sum::<f64>() / sides.len() as f64 };
    let mut home = 0.0;
    for _ in 0..60 {
        // An average team's points.
        if !sides.is_empty() {
            mean = sides.iter().map(|&(t, o, p, s)| p - off[t] + def[o] - s * home / 2.0).sum::<f64>()
                / sides.len() as f64;
        }
        // Home field: the home sides' points over the away sides'.
        let hs: Vec<f64> =
            sides.iter().filter(|x| x.3 != 0.0).map(|&(t, o, p, s)| s * (p - mean - off[t] + def[o])).collect();
        if !hs.is_empty() {
            home = (2.0 * hs.iter().sum::<f64>() / hs.len() as f64).clamp(-10.0, 10.0);
        }
        let mut so = vec![0.0; n];
        let mut sd = vec![0.0; n];
        for &(t, o, p, s) in &sides {
            so[t] += p - mean - s * home / 2.0 + def[o];
            sd[o] += mean + off[t] + s * home / 2.0 - p;
        }
        for k in 0..n {
            let w = weight.max(0.01);
            off[k] = (so[k] + w * p0[k].off) / (played[k] as f64 + w);
            def[k] = (sd[k] + w * p0[k].def) / (played[k] as f64 + w);
        }
    }
    let teams =
        ids.iter().enumerate().map(|(k, &t)| (t, Rating { off: off[k], def: def[k], games: played[k] })).collect();
    Fit { teams, mean, home }
}

/// The normal distribution's chance below `z` (Abramowitz and Stegun 7.1.26).
pub fn phi(z: f64) -> f64 {
    let x = z.abs() / std::f64::consts::SQRT_2;
    let t = 1.0 / (1.0 + 0.327_591_1 * x);
    let y = 1.0
        - (((((1.061_405_429 * t - 1.453_152_027) * t) + 1.421_413_741) * t - 0.284_496_736) * t + 0.254_829_592)
            * t
            * (-x * x).exp();
    if z >= 0.0 { 0.5 * (1.0 + y) } else { 0.5 * (1.0 - y) }
}

/// How far results fall from the ratings: each game day's games read with
/// ratings from the days before only (the way a forecast meets them), the
/// root mean square of margin less forecast, and how many games.
pub fn sigma(games: &[SGame], prior: &HashMap<u32, Rating>, weight: f64) -> (f64, usize) {
    let mut g: Vec<SGame> = games.iter().copied().filter(|g| g.played() && g.points().is_some()).collect();
    g.sort_by_key(|x| x.date);
    let mut sq = 0.0;
    let mut n = 0usize;
    let mut k = 0;
    while k < g.len() {
        let d = g[k].date;
        let end = g[k..].iter().position(|x| x.date != d).map_or(g.len(), |e| k + e);
        if k > 0 {
            let fit = solve(&g[..k], prior, weight);
            for x in &g[k..end] {
                let p = x.points().unwrap();
                let e = (p[0] - p[1]) - fit.margin(x.teams[0], x.teams[1], x.neutral);
                sq += e * e;
                n += 1;
            }
        }
        k = end;
    }
    if n == 0 { (14.0, 0) } else { ((sq / n as f64).sqrt(), n) }
}

/// The home side's chance to win, from its expected margin.
pub fn win_chance(margin: f64, sigma: f64) -> f64 {
    phi(margin / sigma.max(1.0))
}

// ---- The seasons' games. ----

/// A game record of the games tally, as the ratings read it.
#[derive(serde::Deserialize, Default)]
#[serde(default)]
struct Row {
    level: u8,
    date: i32,
    kind: String,
    teams: [u32; 2],
    group: String,
    neutral: bool,
    conference: bool,
    score: [i32; 2],
}

fn row_game(r: &Row) -> SGame {
    let s = r.score;
    SGame {
        date: r.date,
        teams: r.teams,
        neutral: r.neutral,
        league: r.conference,
        post: r.kind != "regular",
        score: Some(s),
        won: Some(if s[0] > s[1] {
            0
        } else if s[1] > s[0] {
            1
        } else {
            2
        }),
    }
}

/// A season's engine games from its tallies, by level (and a high school's
/// state code in `group`): (level, group, game).
pub fn tallied_games(w: &World, season: i32) -> Vec<(u8, String, SGame)> {
    let mut out = Vec::new();
    w.tally_games_as::<Row>(season, |_, r| out.push((r.level, r.group.clone(), row_game(&r))));
    out
}

/// The same, from a season's packed rows (the close, before the file).
pub(crate) fn games_of_frames(f: &tally::file::Frames) -> Vec<(u8, String, SGame)> {
    let mut out = Vec::new();
    f.each(|_, r: &[u8]| {
        if let Some(x) = tally::file::decode::<Row>(r) {
            out.push((x.level, x.group.clone(), row_game(&x)));
        }
    });
    out
}

/// This season's schedule at a level, played and to come: college (every
/// division), the NFL, or one state's high schools. Scores come from the
/// season itself, or the tallies for a high school's engine games.
pub fn this_season(w: &World, level: Level, state: Option<usize>) -> (i32, Vec<SGame>) {
    match level {
        Level::College => {
            let Some(cs) = w.college.as_ref() else { return (w.year, Vec::new()) };
            let games = cs
                .games
                .iter()
                .filter(|g| !g.played.is_some_and(|p| p.cancelled))
                .map(|g| {
                    let p = g.played;
                    SGame {
                        date: g.date.0,
                        teams: [g.home as u32, g.away as u32],
                        neutral: g.site.is_some(),
                        league: g.conference,
                        post: g.kind != GameKind::Regular,
                        score: p.map(|p| [p.score[0] as i32, p.score[1] as i32]),
                        won: p.map(|p| {
                            if p.score[0] == p.score[1] && p.score[0] > 0 {
                                2
                            } else if p.home_won {
                                0
                            } else {
                                1
                            }
                        }),
                    }
                })
                .collect();
            (cs.year, games)
        }
        Level::Nfl => {
            let Some(s) = w.nfl_season.as_ref() else { return (w.year, Vec::new()) };
            let results: HashMap<(u8, u8), i8> = s.results.iter().map(|&(h, a, r)| ((h, a), r)).collect();
            let mut games = Vec::new();
            for (k, week) in s.weeks.iter().enumerate() {
                for (g, &(h, a)) in week.iter().enumerate() {
                    let Some(sl) = s.slots.get(k).and_then(|v| v.get(g)) else { continue };
                    let res = results.get(&(h as u8, a as u8)).copied();
                    let won = if sl.played {
                        match (sl.score, res) {
                            (Some([x, y]), _) if x > y => Some(0),
                            (Some([x, y]), _) if y > x => Some(1),
                            (Some(_), _) => Some(2),
                            (None, Some(1)) => Some(0),
                            (None, Some(-1)) => Some(1),
                            (None, Some(_)) => Some(2),
                            (None, None) => None,
                        }
                    } else {
                        None
                    };
                    games.push(SGame {
                        date: sl.at.date.0,
                        teams: [h as u32, a as u32],
                        neutral: sl.site.is_some(),
                        league: s.div.get(h).is_some() && s.div.get(h) == s.div.get(a),
                        post: false,
                        score: sl.score.filter(|_| sl.played).map(|x| [x[0] as i32, x[1] as i32]),
                        won,
                    });
                }
            }
            (s.year, games)
        }
        Level::Hs => {
            let Some(hs) = w.hs_season.as_ref() else { return (w.year, Vec::new()) };
            let Some(st) = state else { return (w.year, Vec::new()) };
            let Some(ss) = hs.states.get(st) else { return (w.year, Vec::new()) };
            let year = ss.start.year();
            let won = hs.winners(&|x| w.high_schools.get(x).is_some_and(|s| s.state == st));
            // The engine games' scores, from the open season's tallies.
            let mut scores: HashMap<(u32, u32), [i32; 2]> = HashMap::new();
            if w.tally_open_season() == year || w.tally_seasons().contains(&year) {
                let code = &w.states[st].code;
                w.tally_games_as::<Row>(year, |_, r| {
                    if r.level == Level::Hs as u8 && &r.group == code {
                        scores.insert((r.teams[0], r.teams[1]), r.score);
                    }
                });
            }
            let mut seen = std::collections::HashSet::new();
            let mut games = Vec::new();
            let fixtures = hs.fixtures_by_school(st);
            let mut schools: Vec<&usize> = fixtures.keys().collect();
            schools.sort();
            for s in schools {
                for (k, &(h, a)) in fixtures[s].iter().enumerate() {
                    if !seen.insert((h, a, k)) {
                        continue;
                    }
                    let (x, y) = (h as u32, a as u32);
                    let date = ss.weeks.get(k).map_or(ss.start.0 + 7 * k as i32, |d| d.0);
                    let w0 = if won.contains(&(x, y)) {
                        Some(0)
                    } else if won.contains(&(y, x)) {
                        Some(1)
                    } else {
                        None
                    };
                    games.push(SGame {
                        date,
                        teams: [x, y],
                        neutral: false,
                        league: false,
                        post: false,
                        score: scores.get(&(x, y)).copied().filter(|_| w0.is_some()),
                        won: w0,
                    });
                }
            }
            // Each pair once a week (both schools list it).
            games.sort_by_key(|g| (g.date, g.teams));
            games.dedup_by_key(|g| (g.date, g.teams));
            (year, games)
        }
    }
}

// ---- A season's closing ratings (the tallies keep them). ----

/// Each team's closing rating from a season's games, by level and team:
/// the results alone (an average team as the only prior, one game's worth).
pub(crate) fn closing(games: &[(u8, String, SGame)]) -> HashMap<(u8, u32), Rating> {
    let mut out = HashMap::new();
    for level in 0..3u8 {
        let g: Vec<SGame> = games.iter().filter(|x| x.0 == level).map(|x| x.2).collect();
        if g.is_empty() {
            continue;
        }
        let fit = solve(&g, &HashMap::new(), 1.0);
        for (t, r) in fit.teams {
            out.insert((level, t), r);
        }
    }
    out
}

/// A season's ratings at a level: the tallies' closing ratings, else (a
/// world without tallies that season) the points scored and allowed a game
/// over the level's average, not adjusted for the schedule.
pub fn season_ratings(w: &World, level: Level, season: i32) -> HashMap<u32, Rating> {
    #[derive(serde::Deserialize, Default)]
    #[serde(default)]
    struct T {
        level: u8,
        team: u32,
        rating: Vec<f32>,
    }
    let mut out = HashMap::new();
    if w.tally_seasons().contains(&season) && season != w.tally_open_season() {
        let _ = w.tally_each("squads", season, |_, r| {
            if let Some(t) = tally::file::decode::<T>(r)
                && t.level == level as u8
                && t.rating.len() >= 3
            {
                out.insert(
                    t.team,
                    Rating { off: t.rating[0] as f64, def: t.rating[1] as f64, games: t.rating[2] as u32 },
                );
            }
        });
        if !out.is_empty() {
            return out;
        }
    }
    // Points a game, from the books every level keeps.
    use super::folk::Team;
    let lines: Vec<(u32, f64, f64, u32)> = match level {
        Level::College => (0..w.programs.len())
            .filter_map(|p| super::units::season(w, Team::College(p), season).map(|s| (p as u32, s)))
            .map(|(p, s)| (p, s.points_for as f64, s.points_against as f64, s.games))
            .collect(),
        Level::Nfl => (0..w.nfl.len())
            .filter_map(|t| super::units::season(w, Team::Nfl(t), season).map(|s| (t as u32, s)))
            .map(|(t, s)| (t, s.points_for as f64, s.points_against as f64, s.games))
            .collect(),
        Level::Hs => w
            .high_schools
            .iter()
            .enumerate()
            .filter_map(|(h, x)| {
                x.seasons.iter().find(|s| s.year == season && s.wins + s.losses > 0).map(|s| {
                    let g = (s.wins + s.losses).max(1);
                    (h as u32, s.points[0] as f64, s.points[1] as f64, g)
                })
            })
            .collect(),
    };
    let lines: Vec<_> = lines.into_iter().filter(|x| x.3 > 0).collect();
    if lines.is_empty() {
        return out;
    }
    let mean = lines.iter().map(|x| (x.1 + x.2) / (2.0 * x.3 as f64)).sum::<f64>() / lines.len() as f64;
    for (t, pf, pa, g) in lines {
        let g1 = g as f64;
        out.insert(t, Rating { off: pf / g1 - mean, def: mean - pa / g1, games: g });
    }
    out
}

// ---- The preseason projection. ----

/// What a team brings into a season: last season's rating, the one before,
/// the share of its offense's and defense's production back (0-1) and its
/// offense's and defense's stars against the level's average.
#[derive(Clone, Copy, Debug, Default)]
pub struct Inputs {
    pub last: Option<Rating>,
    pub before: Option<Rating>,
    pub back: Option<[f64; 2]>,
    pub stars: Option<[f64; 2]>,
}

/// The projection's weights for one side: a constant, last season, the
/// season before, the share back less a half, the stars against average.
pub type Weights = [f64; 5];

/// The weights when the world's past is too short to fit (proposed, not
/// confirmed): most of last season, a little of the one before.
pub const FALLBACK: Weights = [0.0, 0.6, 0.15, 0.0, 0.0];

fn features(x: &Inputs, side: usize) -> [f64; 5] {
    let pick = |r: Option<Rating>| r.map_or(0.0, |r| if side == 0 { r.off } else { r.def });
    [
        1.0,
        pick(x.last),
        pick(x.before.or(x.last)),
        x.back.map_or(0.0, |b| b[side] - 0.5),
        x.stars.map_or(0.0, |s| s[side]),
    ]
}

/// Fits the weights on past seasons: (inputs, the season's closing rating).
/// Needs 40 seasons of teams at least, else `FALLBACK`.
pub fn fit_projection(rows: &[(Inputs, Rating)]) -> [Weights; 2] {
    let mut out = [FALLBACK; 2];
    if rows.len() < 40 {
        return out;
    }
    for (side, w) in out.iter_mut().enumerate() {
        let mut h = [[0.0; 5]; 5];
        let mut g = [0.0; 5];
        for (x, y) in rows {
            let f = features(x, side);
            let t = if side == 0 { y.off } else { y.def };
            for i in 0..5 {
                g[i] += f[i] * t;
                for j in 0..5 {
                    h[i][j] += f[i] * f[j];
                }
            }
        }
        // A little ridge, so a column the world never fills (no stars kept)
        // reads as zero, not a wild number.
        for (i, row) in h.iter_mut().enumerate() {
            row[i] += 1.0;
        }
        *w = solve_small(h, g);
    }
    out
}

#[allow(clippy::needless_range_loop)]
fn solve_small(mut h: [[f64; 5]; 5], mut g: [f64; 5]) -> [f64; 5] {
    const N: usize = 5;
    for c in 0..N {
        let p = (c..N).max_by(|&a, &b| h[a][c].abs().total_cmp(&h[b][c].abs())).unwrap();
        h.swap(c, p);
        g.swap(c, p);
        if h[c][c].abs() < 1e-12 {
            continue;
        }
        for r in c + 1..N {
            let f = h[r][c] / h[c][c];
            for k in c..N {
                h[r][k] -= f * h[c][k];
            }
            g[r] -= f * g[c];
        }
    }
    let mut x = [0.0; N];
    for c in (0..N).rev() {
        if h[c][c].abs() < 1e-12 {
            continue;
        }
        x[c] = (g[c] - (c + 1..N).map(|k| h[c][k] * x[k]).sum::<f64>()) / h[c][c];
    }
    x
}

/// A team's projection from its inputs.
pub fn project(w: &[Weights; 2], x: &Inputs) -> Rating {
    let side = |s: usize| w[s].iter().zip(features(x, s)).map(|(b, f)| b * f).sum::<f64>();
    Rating { off: side(0), def: side(1), games: 0 }
}

// ---- The odds. ----

/// A team's odds from the rest of its season played `SIMS` times.
#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub struct Odds {
    /// Wins and losses expected at the regular season's end (played ones
    /// included).
    pub wins: f64,
    pub losses: f64,
    /// Six wins or more (bowl eligible; college).
    pub bowl: f64,
    /// Its conference's or division's best record.
    pub league: f64,
    /// A place in its level's playoff field (the BCS era's title game).
    pub playoff: f64,
    /// A first-round bye (the NFL's top seed; the twelve-team playoff's top
    /// four).
    pub bye: f64,
    /// A winning record or better (high school).
    pub winning: f64,
}

/// Who plays for what: each team's group for the league race (its
/// conference or division) and the field's rule.
#[derive(Clone, Debug)]
pub enum Field {
    /// College: each team's division and conference; the era's rules.
    College { division: HashMap<u32, Division>, conference: HashMap<u32, String> },
    /// The NFL: each club's conference (0 AFC, 1 NFC) and division, and the
    /// playoff teams in all.
    Nfl { conf: HashMap<u32, usize>, div: HashMap<u32, usize>, teams: usize },
    /// High school: no field read (the states' own formulas pick it).
    None,
}

/// The committee's order in a played-out season (proposed, not confirmed):
/// fewest losses first, then wins, then the drawn strength.
fn resume(w: u32, l: u32, r: f64) -> f64 {
    w as f64 - 1.25 * l as f64 + r / 7.0
}

/// The odds for every team in `fit`: the season's games still to play
/// (regular season only) drawn from each side's rating, spread by `sigma`
/// a game and by each rating's own doubt.
#[allow(clippy::too_many_arguments)]
pub fn odds(
    games: &[SGame],
    fit: &Fit,
    sigma: f64,
    field: &Field,
    season: i32,
    rules: &crate::calendar::rules::RuleBook,
    seed: u32,
) -> HashMap<u32, Odds> {
    let reg: Vec<&SGame> = games.iter().filter(|g| !g.post).collect();
    let mut ids: Vec<u32> = reg.iter().flat_map(|g| g.teams).collect();
    ids.sort_unstable();
    ids.dedup();
    let at: HashMap<u32, usize> = ids.iter().enumerate().map(|(k, &t)| (t, k)).collect();
    let n = ids.len();
    let base: Vec<Rating> = ids.iter().map(|&t| fit.get(t)).collect();
    // The doubt in a rating: a game's spread over the games behind it.
    let doubt: Vec<f64> = base.iter().map(|r| sigma / ((r.games as f64 + PRIOR_GAMES).sqrt() * 1.6)).collect();
    let mut wins0 = vec![0u32; n];
    let mut loss0 = vec![0u32; n];
    let mut lw0 = vec![0u32; n];
    let mut ll0 = vec![0u32; n];
    let mut todo: Vec<(usize, usize, bool, bool)> = Vec::new();
    for g in &reg {
        let (h, a) = (at[&g.teams[0]], at[&g.teams[1]]);
        match g.won {
            Some(r) => {
                let (w, l) = match r {
                    0 => (h, a),
                    1 => (a, h),
                    _ => continue,
                };
                wins0[w] += 1;
                loss0[l] += 1;
                if g.league {
                    lw0[w] += 1;
                    ll0[l] += 1;
                }
            }
            None => todo.push((h, a, g.neutral, g.league)),
        }
    }
    let mut rng = Rng::new(crate::rng::Seed::Number(seed ^ 0x504F_5752));
    let mut acc = vec![Odds::default(); n];
    let groups: Vec<Option<String>> = ids
        .iter()
        .map(|t| match field {
            Field::College { conference, division } => {
                conference.get(t).filter(|c| !c.is_empty()).map(|c| format!("{:?}|{c}", division.get(t)))
            }
            Field::Nfl { div, .. } => div.get(t).map(|d| d.to_string()),
            Field::None => None,
        })
        .collect();
    for _ in 0..SIMS {
        let r: Vec<f64> = (0..n).map(|k| base[k].overall() + rng.normal(0.0, doubt[k])).collect();
        let (mut wins, mut loss, mut lw, mut ll) = (wins0.clone(), loss0.clone(), lw0.clone(), ll0.clone());
        for &(h, a, neutral, league) in &todo {
            let m = r[h] - r[a] + if neutral { 0.0 } else { fit.home };
            let home_won = rng.normal(m, sigma) > 0.0;
            let (x, y) = if home_won { (h, a) } else { (a, h) };
            wins[x] += 1;
            loss[y] += 1;
            if league {
                lw[x] += 1;
                ll[y] += 1;
            }
        }
        // The league race: best league record, the drawn strength breaking ties.
        let mut best: HashMap<&str, (usize, f64)> = HashMap::new();
        for k in 0..n {
            let Some(gname) = &groups[k] else { continue };
            let pct = (lw[k] as f64 + 0.5) / ((lw[k] + ll[k]) as f64 + 1.0);
            let score = pct * 1000.0 + r[k] / 100.0;
            let e = best.entry(gname.as_str()).or_insert((k, f64::MIN));
            if score > e.1 {
                *e = (k, score);
            }
        }
        let champs: std::collections::HashSet<usize> = best.values().map(|x| x.0).collect();
        for k in 0..n {
            let a = &mut acc[k];
            a.wins += wins[k] as f64;
            a.losses += loss[k] as f64;
            a.bowl += (wins[k] >= 6) as u32 as f64;
            a.winning += (wins[k] >= loss[k]) as u32 as f64;
            a.league += champs.contains(&k) as u32 as f64;
        }
        match field {
            Field::College { division, conference } => {
                for d in Division::ALL {
                    let mut pool: Vec<usize> = (0..n).filter(|&k| division.get(&ids[k]) == Some(&d)).collect();
                    if pool.is_empty() {
                        continue;
                    }
                    pool.sort_by(|&a, &b| resume(wins[b], loss[b], r[b]).total_cmp(&resume(wins[a], loss[a], r[a])));
                    let (picked, byes) = college_field(d, &pool, &champs, conference, &ids, season, rules);
                    for k in picked {
                        acc[k].playoff += 1.0;
                    }
                    for k in byes {
                        acc[k].bye += 1.0;
                    }
                }
            }
            Field::Nfl { conf, div, teams } => {
                let per = (*teams / 2).max(1);
                for c in 0..2 {
                    let pool: Vec<usize> = (0..n).filter(|&k| conf.get(&ids[k]) == Some(&c)).collect();
                    let pct =
                        |k: usize| (wins[k] as f64 + 0.5) / ((wins[k] + loss[k]) as f64 + 1.0) * 1000.0 + r[k] / 100.0;
                    let mut winners: Vec<usize> = Vec::new();
                    let mut by_div: HashMap<usize, usize> = HashMap::new();
                    for &k in &pool {
                        let d = div.get(&ids[k]).copied().unwrap_or(99);
                        let e = by_div.entry(d).or_insert(k);
                        if pct(k) > pct(*e) {
                            *e = k;
                        }
                    }
                    winners.extend(by_div.values());
                    winners.sort_by(|&a, &b| pct(b).total_cmp(&pct(a)));
                    let mut rest: Vec<usize> = pool.iter().copied().filter(|k| !winners.contains(k)).collect();
                    rest.sort_by(|&a, &b| pct(b).total_cmp(&pct(a)));
                    let mut field: Vec<usize> = winners.clone();
                    field.extend(rest.into_iter().take(per.saturating_sub(winners.len())));
                    for &k in &field {
                        acc[k].playoff += 1.0;
                    }
                    // The top seed's bye (14 teams from 2020; 12 gave two).
                    let byes = if *teams >= 14 {
                        1
                    } else if *teams >= 12 {
                        2
                    } else {
                        0
                    };
                    for &k in winners.iter().take(byes) {
                        acc[k].bye += 1.0;
                    }
                }
            }
            Field::None => {}
        }
    }
    let s = SIMS as f64;
    ids.iter()
        .enumerate()
        .map(|(k, &t)| {
            let a = acc[k];
            (
                t,
                Odds {
                    wins: a.wins / s,
                    losses: a.losses / s,
                    bowl: a.bowl / s,
                    league: a.league / s,
                    playoff: a.playoff / s,
                    bye: a.bye / s,
                    winning: a.winning / s,
                },
            )
        })
        .collect()
}

/// A division's playoff field in one played-out season (the world's own
/// sizes, `college::pick_division_playoff` and `selection_day`), and the
/// teams with a first-round bye. `pool` is in the committee's order.
fn college_field(
    d: Division,
    pool: &[usize],
    champs: &std::collections::HashSet<usize>,
    conference: &HashMap<u32, String>,
    ids: &[u32],
    season: i32,
    rules: &crate::calendar::rules::RuleBook,
) -> (Vec<usize>, Vec<usize>) {
    let conf = |k: usize| conference.get(&ids[k]).map_or("", |c| c.as_str());
    match d {
        Division::FBS => {
            let n = match rules.cfb_postseason {
                CfbPostseason::PollsOnly => 0,
                CfbPostseason::Bcs => 2,
                _ => rules.cfp_teams.max(4) as usize,
            };
            let pool: Vec<usize> = if rules.breakaway {
                pool.iter().copied().filter(|&k| matches!(conf(k), "SEC" | "Big Ten")).collect()
            } else {
                pool.to_vec()
            };
            if n < 12 {
                return (pool.iter().copied().take(n).collect(), Vec::new());
            }
            let champs5: Vec<usize> = pool.iter().copied().filter(|k| champs.contains(k)).take(5).collect();
            let mut field = champs5.clone();
            for &k in &pool {
                if field.len() >= n {
                    break;
                }
                if !field.contains(&k) {
                    field.push(k);
                }
            }
            // 2024: the four best champions had the byes; from 2025 the top four seeds.
            let byes: Vec<usize> = if season == 2024 {
                champs5.iter().copied().take(4).collect()
            } else {
                let mut f = field.clone();
                let pos: HashMap<usize, usize> = pool.iter().enumerate().map(|(i, &k)| (k, i)).collect();
                f.sort_by_key(|k| pos[k]);
                f.into_iter().take(4).collect()
            };
            (field, byes)
        }
        _ => {
            let n = match d {
                Division::FCS => rules.fcs_playoff as usize,
                Division::D2 => rules.d2_playoff as usize,
                Division::JUCO => {
                    if season >= 2021 {
                        4
                    } else {
                        2
                    }
                }
                Division::D3 => match season {
                    ..=1984 => 8,
                    1985..=1998 => 16,
                    1999..=2004 => 28,
                    _ => 32,
                },
                Division::NAIA => match season {
                    ..=1986 => 8,
                    1987..=2021 => 16,
                    _ => 20,
                },
                Division::FBS => 0,
            };
            let out = |k: usize| d == Division::FCS && matches!(conf(k), "SWAC" | "MEAC" | "Ivy");
            let mut field: Vec<usize> = Vec::new();
            if d == Division::FCS {
                field.extend(pool.iter().copied().filter(|&k| champs.contains(&k) && !out(k)).take(n));
            }
            for &k in pool {
                if field.len() >= n {
                    break;
                }
                if !out(k) && !field.contains(&k) {
                    field.push(k);
                }
            }
            (field, Vec::new())
        }
    }
}

/// The field each team plays for at a level today.
pub fn field_of(w: &World, level: Level) -> Field {
    match level {
        Level::College => Field::College {
            division: w.programs.iter().map(|p| (p.id as u32, p.division)).collect(),
            conference: w.programs.iter().map(|p| (p.id as u32, p.conference.clone())).collect(),
        },
        Level::Nfl => match w.nfl_season.as_ref() {
            Some(s) => Field::Nfl {
                conf: s.conf.iter().enumerate().map(|(t, &c)| (t as u32, c)).collect(),
                div: s.div.iter().enumerate().map(|(t, &d)| (t as u32, d)).collect(),
                teams: w.rules.nfl_playoff_teams as usize,
            },
            None => Field::None,
        },
        Level::Hs => Field::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(date: i32, h: u32, a: u32, s: [i32; 2]) -> SGame {
        SGame { date, teams: [h, a], score: Some(s), won: Some(if s[0] > s[1] { 0 } else { 1 }), ..Default::default() }
    }

    /// Three teams, each a touchdown better than the next, with home field
    /// worth three: the fit finds both.
    #[test]
    fn the_rating_finds_strength_and_home_field() {
        let mut games = Vec::new();
        let base = [7.0, 0.0, -7.0];
        let mut d = 0;
        for _ in 0..6 {
            for h in 0..3u32 {
                for a in 0..3u32 {
                    if h == a {
                        continue;
                    }
                    let m = base[h as usize] - base[a as usize] + 3.0;
                    let hp: f64 = 24.0 + m / 2.0;
                    let ap: f64 = 24.0 - m / 2.0;
                    games.push(g(d, h, a, [hp.round() as i32, ap.round() as i32]));
                    d += 1;
                }
            }
        }
        let fit = solve(&games, &HashMap::new(), 0.5);
        assert!((fit.home - 3.0).abs() < 0.6, "home {}", fit.home);
        let r = |t: u32| fit.get(t).overall();
        assert!((r(0) - r(1) - 7.0).abs() < 0.8 && (r(1) - r(2) - 7.0).abs() < 0.8, "{} {} {}", r(0), r(1), r(2));
        assert!(win_chance(fit.margin(0, 2, true), 14.0) > 0.8);
        assert!((phi(0.0) - 0.5).abs() < 1e-6 && (phi(1.96) - 0.975).abs() < 1e-3);
    }

    /// The prior counts like a few games: with no games a team is its
    /// prior, and the season's results take over as they come.
    #[test]
    fn the_projection_fades_as_games_come() {
        let prior: HashMap<u32, Rating> =
            [(0, Rating { off: 5.0, def: 5.0, games: 0 }), (1, Rating::default())].into_iter().collect();
        let fit = solve(&[], &prior, PRIOR_GAMES);
        assert!((fit.get(0).overall() - 10.0).abs() < 1e-9);
        // Home and away in turn, so home field cannot take the credit.
        let games: Vec<SGame> =
            (0..12).map(|d| if d % 2 == 0 { g(d, 1, 0, [30, 20]) } else { g(d, 0, 1, [20, 30]) }).collect();
        let fit = solve(&games, &prior, PRIOR_GAMES);
        assert!(fit.get(1).overall() > fit.get(0).overall(), "the results win out");
    }

    /// Projection weights fitted on rows where this season is last season's
    /// rating halved come out near a half.
    #[test]
    fn the_projection_is_fitted_on_the_past() {
        let rows: Vec<(Inputs, Rating)> = (0..200)
            .map(|k| {
                let x = (k as f64 - 100.0) / 10.0;
                let last = Rating { off: x, def: -x / 2.0, games: 12 };
                (Inputs { last: Some(last), ..Default::default() }, Rating { off: x / 2.0, def: -x / 4.0, games: 12 })
            })
            .collect();
        let w = fit_projection(&rows);
        let p = project(&w, &Inputs { last: Some(Rating { off: 8.0, def: -4.0, games: 12 }), ..Default::default() });
        assert!((p.off - 4.0).abs() < 0.3 && (p.def + 2.0).abs() < 0.3, "{p:?}");
    }

    /// The odds add up: a team that has won every game and plays nobody
    /// good is a near-lock for six wins.
    #[test]
    fn the_odds_play_the_season_out() {
        let mut games: Vec<SGame> = (0..5).map(|d| g(d, 0, 1 + d as u32, [40, 10])).collect();
        for d in 5..12 {
            games.push(SGame { date: d, teams: [0, 10 + d as u32], ..Default::default() });
        }
        let fit = solve(&games, &HashMap::new(), PRIOR_GAMES);
        let rules = crate::calendar::rules::RuleBook::era(2025);
        let o = odds(&games, &fit, 14.0, &Field::None, 2025, &rules, 7);
        let me = o[&0];
        assert!(me.bowl > 0.9 && me.wins > 9.0 && me.wins <= 12.0, "{me:?}");
        assert!((me.wins + me.losses - 12.0).abs() < 1e-9);
    }
}
