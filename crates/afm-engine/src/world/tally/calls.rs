//! The calls tally (`calls/<season>.tally`): each team's season of game
//! management (Data overhaul item 26, lane NB8): every 4th down and every
//! try after a touchdown against the analysts' model (`decide.rs`), the
//! timeouts it spent and the challenges it threw, one record a team a
//! season at every level.
//!
//! College and NFL teams keep each 4th down, try and challenge as a row
//! (the Decisions page's lists and a game page's table); high school
//! teams keep the sums and the grid only, since a high school staff has
//! only its own film (your own games' rows come from the games you keep).
//! The grid counts each team's 4th downs by yards to go and field zone:
//! how often it went and how often the model said go.

use std::collections::{BTreeMap, HashMap};

use crate::game::live::{CloseCall, Replay, Reviewer};
use crate::game::stats::{GameStats, PlayRecord, PlayType};
use crate::player::Level as PLevel;

use super::Level;
use super::decide::{Model, Weigh};

/// A choice worth this much win chance (0-1) over the next is a clear
/// call; within it the coach agrees with the model either way (proposed,
/// not confirmed).
pub const CLEAR: f64 = 0.01;

/// One 4th down.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Fourth {
    /// The game's key in the season's games file.
    pub game: u64,
    pub date: i32,
    pub opp: u32,
    /// 0 at home, 1 away, 2 a neutral site.
    pub at: u8,
    pub q: u8,
    /// Seconds left in the quarter.
    pub clock: u16,
    /// The team's lead at the snap.
    pub lead: i16,
    pub togo: f32,
    /// Yards to the goal.
    pub yl: u8,
    /// 0 went for it, 1 punted, 2 kicked.
    pub did: u8,
    /// The model's win chance going, punting and kicking, 0-100 (-1 where it
    /// was no choice), and the chance to convert (0-100). All -1 where the
    /// model doesn't weigh it (college and high school overtime).
    pub wp: [f32; 3],
    pub conv: f32,
    /// The play: 0 run, 1 pass, 2 sack, 3 scramble, 4 punt, 5 field goal.
    pub kind: u8,
    /// Going: 0 stopped, 1 a first down, 2 a touchdown, 3 a turnover. A
    /// kick: 1 good, 0 missed, 4 blocked. A punt: 0, 4 blocked.
    pub res: u8,
    /// Yards gained going; a kick's distance; after a punt, where the other
    /// side began (yards from its own goal).
    pub yds: f32,
    /// Who carried, caught (else threw) or kicked.
    pub who: u32,
    /// The final score, the team's and the other's.
    pub fin: [i16; 2],
}

impl Fourth {
    /// The model's weights back (None where unweighed).
    pub fn weigh(&self) -> Option<Weigh> {
        (self.wp[0] >= 0.0).then(|| Weigh {
            go: self.wp[0] as f64 / 100.0,
            punt: (self.wp[1] >= 0.0).then_some(self.wp[1] as f64 / 100.0),
            kick: (self.wp[2] >= 0.0).then_some(self.wp[2] as f64 / 100.0),
            conv: self.conv as f64 / 100.0,
        })
    }
}

/// The model's verdict on one 4th down: its best choice (0 go, 1 punt, 2
/// kick), whether going was the clear call, whether the coach's choice
/// was within `CLEAR` of the best and the win chance (0-1) it gave up.
/// `even`: every choice there was within `CLEAR` of the best (a game
/// already decided, or a toss-up), so the model has no call.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Judge {
    pub best: u8,
    pub said_go: bool,
    pub agreed: bool,
    pub lost: f64,
    pub even: bool,
}

pub fn judge(w: &Weigh, did: u8) -> Judge {
    let (b, best) = w.best();
    let next = [Some(w.go), w.punt, w.kick]
        .iter()
        .enumerate()
        .filter(|(k, _)| *k as u8 != b)
        .filter_map(|(_, v)| *v)
        .fold(f64::MIN, f64::max);
    let chosen = w.of(did).unwrap_or(w.go);
    let lowest = [Some(w.go), w.punt, w.kick].iter().filter_map(|v| *v).fold(f64::MAX, f64::min);
    Judge {
        best: b,
        said_go: b == 0 && best >= next + CLEAR,
        agreed: chosen >= best - CLEAR,
        lost: (best - chosen).max(0.0),
        even: best - lowest < CLEAR,
    }
}

/// The model's verdict on a try (win chance kicking and going for two):
/// whether two was the clear call, whether the choice agreed and what it
/// gave up.
pub fn judge_try(kick: f64, go: f64, two: bool) -> (bool, bool, f64) {
    let best = kick.max(go);
    let chosen = if two { go } else { kick };
    (go >= kick + CLEAR, chosen >= best - CLEAR, (best - chosen).max(0.0))
}

impl Try {
    /// The model's verdict (None where unweighed).
    pub fn judge(&self) -> Option<(bool, bool, f64)> {
        (self.wp[0] >= 0.0).then(|| judge_try(self.wp[0] as f64 / 100.0, self.wp[1] as f64 / 100.0, self.two))
    }
}

/// The 4th-down and try sums of a season's rows (college and the NFL keep
/// rows): what a page reads when it leaves out the games your staff
/// hasn't charted yet. Timeouts and challenges stay with the season's sums.
pub fn sums_of(fourths: &[&Fourth], tries: &[&Try]) -> (Sums, BTreeMap<u16, [u32; 3]>) {
    let mut s = Sums::default();
    let mut grid: BTreeMap<u16, [u32; 3]> = BTreeMap::new();
    for f in fourths {
        s.fourths += 1;
        s.went += (f.did == 0) as u32;
        s.converted += (f.did == 0 && matches!(f.res, 1 | 2)) as u32;
        let cell = grid.entry(grid_key(f.togo as f64, f.yl as f64)).or_default();
        cell[0] += 1;
        cell[1] += (f.did == 0) as u32;
        if let Some(w) = f.weigh() {
            let j = judge(&w, f.did);
            s.weighed += 1;
            if j.said_go {
                s.said_go += 1;
                s.went_said_go += (f.did == 0) as u32;
                cell[2] += 1;
            }
            s.agreed += j.agreed as u32;
            s.lost += j.lost as f32;
        }
    }
    for t in tries {
        s.tries += 1;
        s.twos += t.two as u32;
        s.twos_made += (t.two && t.good) as u32;
        if let Some((said_two, agreed, lost)) = t.judge() {
            s.tries_weighed += 1;
            s.said_two += said_two as u32;
            s.two_said_two += (said_two && t.two) as u32;
            s.try_agreed += agreed as u32;
            s.try_lost += lost as f32;
        }
    }
    (s, grid)
}

/// One try after a touchdown.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Try {
    pub game: u64,
    pub date: i32,
    pub opp: u32,
    pub at: u8,
    pub q: u8,
    pub clock: u16,
    /// The team's lead with the touchdown, before the try.
    pub lead: i16,
    pub two: bool,
    pub good: bool,
    /// The model's win chance kicking and going for two, 0-100 (-1 where
    /// unweighed).
    pub wp: [f32; 2],
    pub fin: [i16; 2],
}

/// One challenge thrown.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Chal {
    pub game: u64,
    pub date: i32,
    pub opp: u32,
    pub q: u8,
    pub clock: u16,
    /// 0 a catch, 1 a fumble, 2 the goal line, 3 a spot, 4 an interception.
    pub call: u8,
    pub won: bool,
}

/// A team's sums (every level).
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Sums {
    pub games: u32,
    /// 4th downs (kneels left out), the weighed ones, went for it and
    /// converted.
    pub fourths: u32,
    pub weighed: u32,
    pub went: u32,
    pub converted: u32,
    /// The model said go clearly, and the team went then.
    pub said_go: u32,
    pub went_said_go: u32,
    /// Did the model's best, or something within a point of it.
    pub agreed: u32,
    /// Win chance given up against the model's best, summed (0-1 a play).
    pub lost: f32,
    /// Tries after touchdowns, weighed, two-point tries and made, and the
    /// times the model said two clearly (and the team went then).
    pub tries: u32,
    pub tries_weighed: u32,
    pub twos: u32,
    pub twos_made: u32,
    pub said_two: u32,
    pub two_said_two: u32,
    pub try_agreed: u32,
    pub try_lost: f32,
    /// Timeouts taken, in the last two minutes of a half, and left over at
    /// the end of losses by eight or less (and how many such losses).
    pub timeouts: u32,
    pub to_late: u32,
    pub to_left: u32,
    pub close_losses: u32,
    /// Challenges thrown and won, and wrong rulings against the team it
    /// could have challenged and didn't.
    pub challenges: u32,
    pub won: u32,
    pub missed: u32,
}

impl Sums {
    pub fn add(&mut self, b: &Sums) {
        self.games += b.games;
        self.fourths += b.fourths;
        self.weighed += b.weighed;
        self.went += b.went;
        self.converted += b.converted;
        self.said_go += b.said_go;
        self.went_said_go += b.went_said_go;
        self.agreed += b.agreed;
        self.lost += b.lost;
        self.tries += b.tries;
        self.tries_weighed += b.tries_weighed;
        self.twos += b.twos;
        self.twos_made += b.twos_made;
        self.said_two += b.said_two;
        self.two_said_two += b.two_said_two;
        self.try_agreed += b.try_agreed;
        self.try_lost += b.try_lost;
        self.timeouts += b.timeouts;
        self.to_late += b.to_late;
        self.to_left += b.to_left;
        self.close_losses += b.close_losses;
        self.challenges += b.challenges;
        self.won += b.won;
        self.missed += b.missed;
    }
}

/// The grid's rows by yards to go: 1, 2, 3, 4-5, 6-10, 11 and more.
pub const GRID_ROWS: [&str; 6] = ["4th & 1", "4th & 2", "4th & 3", "4th & 4-5", "4th & 6-10", "4th & 11+"];
/// The grid's field zones, from the team's own goal: own 1-19, own 20-39,
/// own 40-49, the opponent's 50-35, 34-20 and 19-1.
pub const GRID_ZONES: [&str; 6] = ["own 1-19", "own 20-39", "own 40-49", "opp 50-35", "opp 34-20", "opp 19-1"];
/// Each row's and zone's middle, for the model's own grid.
pub const GRID_TOGO: [f64; 6] = [1.0, 2.0, 3.0, 4.5, 8.0, 13.0];
pub const GRID_YL: [f64; 6] = [90.0, 70.0, 55.0, 42.0, 27.0, 10.0];

pub fn grid_row(togo: f64) -> usize {
    match togo.max(0.0).ceil() {
        t if t <= 1.0 => 0,
        t if t <= 2.0 => 1,
        t if t <= 3.0 => 2,
        t if t <= 5.0 => 3,
        t if t <= 10.0 => 4,
        _ => 5,
    }
}

pub fn grid_zone(yl: f64) -> usize {
    match yl.round() {
        y if y >= 81.0 => 0,
        y if y >= 61.0 => 1,
        y if y >= 51.0 => 2,
        y if y >= 35.0 => 3,
        y if y >= 20.0 => 4,
        _ => 5,
    }
}

pub fn grid_key(togo: f64, yl: f64) -> u16 {
    (grid_row(togo) * 8 + grid_zone(yl)) as u16
}

/// One side's game.
#[derive(Clone, Debug, Default)]
pub struct Game {
    pub sum: Sums,
    /// Grid cell -> (4th downs, went, the model said go).
    pub grid: HashMap<u16, [u32; 3]>,
    pub fourths: Vec<Fourth>,
    pub tries: Vec<Try>,
    pub challenges: Vec<Chal>,
}

/// A team's season.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Season {
    /// 0 college, 1 NFL, 2 high school.
    pub level: u8,
    pub team: u32,
    pub sum: Sums,
    pub grid: BTreeMap<u16, [u32; 3]>,
    pub fourths: Vec<Fourth>,
    pub tries: Vec<Try>,
    pub challenges: Vec<Chal>,
}

pub fn key(level: Level, team: u32) -> u64 {
    super::teams::key(level, team)
}

/// The open season's teams.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Open {
    pub seasons: HashMap<u64, Season>,
}

impl Open {
    /// A side's game, its rows marked with the game's key.
    pub fn add(&mut self, level: Level, team: u32, game: u64, g: Game) {
        let s = self.seasons.entry(key(level, team)).or_insert_with(|| Season {
            level: level as u8,
            team,
            ..Default::default()
        });
        s.sum.add(&g.sum);
        for (k, v) in g.grid {
            let e = s.grid.entry(k).or_default();
            for i in 0..3 {
                e[i] += v[i];
            }
        }
        s.fourths.extend(g.fourths.into_iter().map(|mut f| {
            f.game = game;
            f
        }));
        s.tries.extend(g.tries.into_iter().map(|mut t| {
            t.game = game;
            t
        }));
        s.challenges.extend(g.challenges.into_iter().map(|mut c| {
            c.game = game;
            c
        }));
    }

    pub fn rows(&self) -> Vec<(u64, &Season)> {
        let mut v: Vec<(u64, &Season)> = self.seasons.iter().map(|(&k, s)| (k, s)).collect();
        v.sort_by_key(|x| x.0);
        v
    }
}

/// What the game knows about itself for the rows: its date, the two teams
/// (home, away), a neutral site and the season.
#[derive(Clone, Copy, Debug, Default)]
pub struct About {
    pub date: i32,
    pub teams: [u32; 2],
    pub neutral: bool,
    pub season: i32,
}

fn pct(x: f64) -> f32 {
    ((x * 1000.0).round() / 10.0) as f32
}

/// A win chance (0-1) as a row keeps it.
fn kept(x: f64) -> f64 {
    pct(x) as f64 / 100.0
}

fn kind_code(k: PlayType) -> u8 {
    match k {
        PlayType::Run => 0,
        PlayType::Pass => 1,
        PlayType::Sack => 2,
        PlayType::Scramble => 3,
        PlayType::Punt => 4,
        _ => 5,
    }
}

fn call_code(c: CloseCall) -> u8 {
    match c {
        CloseCall::Catch => 0,
        CloseCall::Fumble => 1,
        CloseCall::GoalLine => 2,
        CloseCall::Spot => 3,
        CloseCall::Interception => 4,
    }
}

/// The last two minutes of a half (or overtime).
fn late(r: &PlayRecord) -> bool {
    r.quarter > 4 || (matches!(r.quarter, 2 | 4) && r.clock <= 120.0)
}

/// Where the other side began after a punt: its next snap with the ball,
/// in yards from its own goal (None if the half ended first or the
/// punting side had the ball next).
pub fn punt_to(plays: &[PlayRecord], i: usize) -> Option<f32> {
    let p0 = &plays[i];
    let half = |q: u8| {
        if q <= 2 {
            1
        } else if q <= 4 {
            2
        } else {
            q
        }
    };
    let next = plays[i + 1..].iter().find(|p| {
        p.offense < 2
            && !p.nullified
            && matches!(
                p.kind,
                PlayType::Run
                    | PlayType::Pass
                    | PlayType::Sack
                    | PlayType::Scramble
                    | PlayType::Punt
                    | PlayType::FieldGoal
                    | PlayType::Kneel
                    | PlayType::Spike
            )
    })?;
    (next.offense.min(1) != p0.offense.min(1) && half(next.quarter) == half(p0.quarter))
        .then_some((100.0 - next.yardline_100) as f32)
}

/// Both sides' game: every 4th down and try weighed, the timeouts and the
/// challenges. `rows`: keep each one as a row (college and the NFL).
pub fn game(g: &GameStats, m: &Model, about: About, rows: bool) -> [Game; 2] {
    let mut out: [Game; 2] = Default::default();
    let plays = &g.plays;
    let fin = |k: usize| [g.score[k] as i16, g.score[1 - k] as i16];
    let at = |k: usize| if about.neutral { 2 } else { k as u8 };
    let clock = |r: &PlayRecord| r.clock.clamp(0.0, 65_000.0) as u16;
    for k in 0..2 {
        out[k].sum.games = 1;
    }
    let replay = Replay::of(m.level, about.season);
    let mut prev_to: Option<(u8, [u8; 2])> = None;
    for (i, r) in plays.iter().enumerate() {
        if r.offense > 1 {
            continue;
        }
        let o = r.offense.min(1);
        // Timeouts: a drop in a side's count from one snap to the next in
        // the same half.
        if let Some(to) = r.timeouts {
            let half = if r.quarter <= 2 {
                1
            } else if r.quarter <= 4 {
                2
            } else {
                r.quarter
            };
            if let Some((h, before)) = prev_to
                && h == half
            {
                for k in 0..2 {
                    let used = before[k].saturating_sub(to[k]) as u32;
                    out[k].sum.timeouts += used;
                    if late(r) {
                        out[k].sum.to_late += used;
                    }
                }
            }
            prev_to = Some((half, to));
        }
        // Challenges, and wrong rulings a coach could have challenged.
        if let Some(of) = &r.officiating {
            if let Reviewer::Challenge(t) = of.reviewer {
                let t = t.min(1);
                out[t].sum.challenges += 1;
                out[t].sum.won += of.overturned as u32;
                if rows {
                    out[t].challenges.push(Chal {
                        date: about.date,
                        opp: about.teams[1 - t],
                        q: r.quarter,
                        clock: clock(r),
                        call: call_code(of.call),
                        won: of.overturned,
                        ..Default::default()
                    });
                }
            } else if of.wrong && of.reviewer == Reviewer::None && replay.challenges > 0 && !replay.booth_all {
                let booth = (replay.booth_late && late(r))
                    || (replay.booth_scores && r.touchdown)
                    || (replay.booth_turnovers && r.turnover);
                if !booth {
                    out[of.against.min(1)].sum.missed += 1;
                }
            }
        }
        if r.nullified {
            continue;
        }
        // Tries after a touchdown.
        if matches!(r.kind, PlayType::ExtraPoint | PlayType::TwoPoint) {
            // College and high school overtime's tries (two-point tries
            // forced from the third period) are left out.
            if m.untimed(r) {
                continue;
            }
            let two = r.kind == PlayType::TwoPoint;
            let good = if two { r.touchdown && !r.turnover } else { r.kick_good };
            let s = &mut out[o].sum;
            s.tries += 1;
            s.twos += two as u32;
            s.twos_made += (two && good) as u32;
            let w = m.try_after(r).map(|x| x.map(kept));
            if let Some([kick, go]) = w {
                let (said_two, agreed, lost) = judge_try(kick, go, two);
                s.tries_weighed += 1;
                s.said_two += said_two as u32;
                s.two_said_two += (said_two && two) as u32;
                s.try_agreed += agreed as u32;
                s.try_lost += lost as f32;
            }
            if rows {
                let lead = if o == 0 { r.score[0] - r.score[1] } else { r.score[1] - r.score[0] };
                out[o].tries.push(Try {
                    date: about.date,
                    opp: about.teams[1 - o],
                    at: at(o),
                    q: r.quarter,
                    clock: clock(r),
                    lead: lead as i16,
                    two,
                    good,
                    wp: w.map_or([-1.0; 2], |x| x.map(pct)),
                    fin: fin(o),
                    ..Default::default()
                });
            }
            continue;
        }
        // 4th downs: went for it, punted or kicked (kneels and spikes left
        // out).
        let did = match r.kind {
            PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble => 0u8,
            PlayType::Punt => 1,
            PlayType::FieldGoal => 2,
            _ => continue,
        };
        if r.down != 4 {
            continue;
        }
        // Weighed at the precision the row keeps, so a page that reads the
        // rows judges every 4th down as the sums did.
        let w = m.fourth(r).map(|w| Weigh {
            go: kept(w.go),
            punt: w.punt.map(kept),
            kick: w.kick.map(kept),
            conv: kept(w.conv),
        });
        let x = &mut out[o];
        x.sum.fourths += 1;
        x.sum.went += (did == 0) as u32;
        let converted = did == 0 && (r.first_down || (r.touchdown && !r.defense_td));
        x.sum.converted += converted as u32;
        let cell = x.grid.entry(grid_key(r.togo, r.yardline_100)).or_default();
        cell[0] += 1;
        cell[1] += (did == 0) as u32;
        if let Some(w) = w {
            let j = judge(&w, did);
            x.sum.weighed += 1;
            if j.said_go {
                x.sum.said_go += 1;
                x.sum.went_said_go += (did == 0) as u32;
                cell[2] += 1;
            }
            x.sum.agreed += j.agreed as u32;
            x.sum.lost += j.lost as f32;
        }
        if rows {
            let lead = if o == 0 { r.score[0] - r.score[1] } else { r.score[1] - r.score[0] };
            let res = match did {
                0 if r.turnover && !r.first_down && !r.touchdown => 3,
                0 if r.touchdown && !r.defense_td => 2,
                0 => r.first_down as u8,
                1 => 4 * r.blocked as u8,
                _ if r.blocked => 4,
                _ => r.kick_good as u8,
            };
            let yds = match did {
                0 => r.yards as f32,
                1 => punt_to(plays, i).unwrap_or(-1.0),
                _ => r.kick_distance as f32,
            };
            let who = match r.kind {
                PlayType::Run | PlayType::Scramble => r.carrier.or(r.passer),
                PlayType::Pass if r.complete => r.target.or(r.passer),
                PlayType::Pass | PlayType::Sack => r.passer,
                _ => r.kicker,
            };
            x.fourths.push(Fourth {
                date: about.date,
                opp: about.teams[1 - o],
                at: at(o),
                q: r.quarter,
                clock: clock(r),
                lead: lead as i16,
                togo: r.togo as f32,
                yl: r.yardline_100.round().clamp(0.0, 100.0) as u8,
                did,
                wp: w.map_or([-1.0; 3], |w| [pct(w.go), w.punt.map_or(-1.0, pct), w.kick.map_or(-1.0, pct)]),
                conv: w.map_or(-1.0, |w| pct(w.conv)),
                kind: kind_code(r.kind),
                res,
                yds,
                who: who.unwrap_or(0),
                fin: fin(o),
                ..Default::default()
            });
        }
    }
    // Timeouts left at the end of a loss by eight or less.
    if let Some(last) = plays.iter().rev().find(|r| r.offense < 2 && r.timeouts.is_some() && r.quarter <= 4) {
        let to = last.timeouts.unwrap_or_default();
        for k in 0..2 {
            let m = g.score[k] - g.score[1 - k];
            if (-8..0).contains(&m) {
                out[k].sum.close_losses += 1;
                out[k].sum.to_left += to[k] as u32;
            }
        }
    }
    out
}

/// The model's level for a game's tally (`PLevel`), from the tally's level
/// and the division where it is college.
pub fn model_level(level: Level, college: Option<PLevel>) -> PLevel {
    match level {
        Level::Nfl => PLevel::NFL,
        Level::Hs => PLevel::HS,
        Level::College => college.unwrap_or(PLevel::FBS),
    }
}
