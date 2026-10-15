//! Garbage time (dynasty item 3, trial 4 topic 2): a head coach decides,
//! series by series, whether the game is won. From that read his starters
//! come out in the real order, the leader plays to end the game, and a
//! beaten team stops fighting. Nothing eases up by a flat factor: the
//! backups play with their own ratings and the calls change.
//!
//! The read is the comeback chance a coach sees: the lead against how much
//! scoring can still swing in the time left. His game management sets how
//! well he reads it, his hidden sportsmanship how soon he acts on it, and
//! the situation pushes both. Every number here is calibrated against real
//! play-by-play (`data/real-world/garbage_time.json`) and is proposed, not
//! confirmed.

use crate::player::Level;
use crate::positions::Position;
use crate::rng::{Rng, clamp};

use super::roster::Depth;

/// Swing of the final margin over a whole game, and the part of it no clock
/// removes (a late score, an onside kick), in points. Fitted to the real
/// NFL win-probability table (`nfl_win_prob.csv`, leads 8-28 with 5-45
/// minutes left): lead 14 with 7:30 left comes back about 1 in 70.
const SWING: f64 = 10.0;

/// Behind, a coach stops trying to win before he takes his starters out:
/// in many games he plays it out with them whatever the score, running the
/// ball and the clock (real FBS, down 28+ after three quarters: 51% passes
/// and 27.7 seconds a play against 62% and 22.1 in close games, yet QB1
/// plays in the 4th quarter in 66% of those games and in 52% of games down
/// 42+, against 13% up 42+). Fitted to the real shares by deficit; the
/// level's share of games he plays out is in `Habits`.
const FIGHT_SD: f64 = 0.8;
const PLAY_OUT_FIGHT: f64 = 4.5;

/// Beaten, how often he goes for it instead of kicking a field goal that
/// changes nothing (real FBS, down 28+ in the 4th quarter: 83% of 4th downs
/// in field goal range go).
pub const CONCEDED_GO: f64 = 0.7;
const FLOOR: f64 = 3.25;

/// How many standard deviations of the swing still left a lead is: about 3.1
/// is a 1-in-1,000 comeback.
pub fn comeback_z(lead: i32, game_left: f64, game_len: f64) -> f64 {
    lead as f64 / (SWING * (game_left.max(0.0) / game_len).sqrt() + FLOOR)
}

/// Odds against a comeback at `z`, for the why lines.
pub fn comeback_odds(z: f64) -> f64 {
    let p = 1.0 - crate::athlete::phi(z.abs());
    1.0 / p.max(1e-7)
}

/// What the situation does to one coach's read of one game (proposed, not
/// confirmed). Only pushes the world has data for exist; rivals, award
/// races and senior day wait for the calendar, awards and walk-ons.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Context {
    /// A contender late in the season: he believes margins impress the
    /// voters, so he keeps his starters in longer when ahead.
    pub race: bool,
    /// The other side is the program that fired him.
    pub grudge: bool,
    /// His job is on the line: behind, he keeps fighting longer.
    pub hot_seat: bool,
    /// One or two wins short of a winning season late in the year: behind,
    /// he keeps fighting longer.
    pub bowl_push: bool,
}

/// One level's habits: when the first read comes and the read each step
/// needs (comeback z). Starters come out in the real order (real
/// play-by-play, FBS 2022-24 and NFL 2018-25): the quarterback and the skill
/// players first, the lead back usually before the quarterback; with them
/// the back seven on defense; the lines last; at the end of a rout the
/// second string too.
#[derive(Clone, Copy, Debug)]
pub struct Habits {
    pub first_quarter: u8,
    pub steps: [f64; 3],
    pub max_step: u8,
    /// The read where a leader starts playing to end it, and where he is
    /// fully doing so (real: FBS leaders up 21-27 entering the 4th run as
    /// much as those up 28+, those up 15-20 partly).
    pub ease: (f64, f64),
    /// The read where a beaten team stops trying to win.
    pub concede: f64,
    /// How often a beaten coach plays it out with his starters.
    pub play_out: f64,
    /// Ahead with his backups in, how often he goes for it on 4th down
    /// rather than kick a field goal (a good sport more often).
    pub fg_go: f64,
}

pub fn habits(level: Level) -> Habits {
    match level {
        // High school pulls earliest: a 35-0 halftime score ends it.
        Level::HS => Habits {
            first_quarter: 2,
            steps: [3.4, 3.9, 4.7],
            max_step: 3,
            ease: (1.2, 2.4),
            concede: 3.0,
            play_out: 0.35,
            fg_go: 0.6,
        },
        // The NFL keeps starters longer and rarely pulls the lines; a
        // beaten pro team keeps playing (real NFL, down 28+ after three
        // quarters: 66% passes, 24.4 seconds a play, QB1 plays in 83%).
        Level::NFL => Habits {
            first_quarter: 3,
            steps: [4.4, 5.4, 99.0],
            max_step: 2,
            ease: (1.4, 2.8),
            concede: 5.2,
            play_out: 0.6,
            fg_go: 0.25,
        },
        _ => Habits {
            first_quarter: 3,
            steps: [4.3, 4.8, 5.7],
            max_step: 3,
            ease: (1.2, 2.4),
            concede: 3.4,
            play_out: 0.45,
            fg_go: 0.55,
        },
    }
}

/// The back-off step to undo a step: a coach who pulled his starters puts
/// them back only when the game is clearly alive again.
const HOLD: f64 = 0.8;

/// How much sooner than the first step the lead back gets his rest (real:
/// the FBS lead back's share of 4th-quarter carries falls from 42% in close
/// games to 27% up 21-27, while the quarterback still plays).
const REST_BACK: f64 = 1.2;

/// A coach's read of his side's game at one review.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Read {
    /// 0 = everyone in; 1 = skill players and back seven out; 2 = the whole
    /// first team out; 3 = the second string out too.
    pub step: u8,
    /// Leading: how much he plays to end the game, 0-1.
    pub ease: f64,
    /// Behind: he has given up on this one.
    pub concede: bool,
    /// Before the first step: the lead back already sits (real: his share of
    /// the 4th-quarter carries falls by about a third up or down 21-27).
    pub rest_back: bool,
    /// The comeback z as he sees it (signed: positive when ahead).
    pub seen: f64,
}

/// Per-coach, per-game state of the read.
#[derive(Clone, Debug, Default)]
pub struct Coach {
    pub read: Read,
    /// How far off this coach's read is in this game (z units).
    bias: f64,
    /// Game management, 1-20.
    gm: f64,
    sportsmanship: f64,
    ctx: Context,
    /// Behind, how long this coach keeps fighting in this game (z units):
    /// some keep the starters in to the end of any rout.
    fight: f64,
}

impl Coach {
    pub fn new(gm: f64, sportsmanship: f64, ctx: Context, h: &Habits, rng: &mut Rng) -> Coach {
        let bias = rng.normal(0.0, 0.03 * (21.0 - gm).max(0.0));
        let fight = if rng.chance(h.play_out) { PLAY_OUT_FIGHT } else { rng.normal(0.0, FIGHT_SD) };
        Coach { read: Read::default(), bias, gm, sportsmanship, ctx, fight }
    }

    /// Thresholds for each step, moved by who he is and the situation.
    fn steps(&self, h: &Habits, leading: bool) -> [f64; 3] {
        let mut s = h.steps;
        let add = self.shift(leading) + if leading { 0.0 } else { self.fight };
        for x in s.iter_mut() {
            *x += add;
        }
        s
    }

    fn shift(&self, leading: bool) -> f64 {
        if leading {
            // A bad sport runs it up; a good one pulls early. Pushes are in
            // sportsmanship points.
            let push = if self.ctx.race { -2.0 } else { 0.0 } + if self.ctx.grudge { -4.0 } else { 0.0 };
            0.15 * (10.0 - (self.sportsmanship + push))
        } else {
            // His job or a bowl on the line, he keeps trying longer.
            (if self.ctx.hot_seat { 0.8 } else { 0.0 }) + if self.ctx.bowl_push { 0.5 } else { 0.0 }
        }
    }

    /// Review the game at a drive start or a new quarter.
    pub fn review(&mut self, z: f64, quarter: u8, h: &Habits, rng: &mut Rng) -> Read {
        if quarter < h.first_quarter {
            return self.read;
        }
        let seen = z + self.bias + rng.normal(0.0, 0.015 * (21.0 - self.gm).max(0.0));
        let leading = seen > 0.0;
        let th = self.steps(h, leading);
        let a = seen.abs();
        let want = (0..h.max_step as usize).filter(|&k| a >= th[k]).count() as u8;
        let cur = self.read.step;
        let flipped = cur > 0 && leading != (self.read.seen > 0.0);
        // The step only falls back when the game is alive again: a coach
        // who leads by about the same score doesn't go back and forth.
        let step = if flipped || want >= cur {
            want
        } else {
            let keep = (0..cur as usize).filter(|&k| a >= th[k] - HOLD).count() as u8;
            want.max(keep)
        };
        let ease = if leading {
            let (from, full) = (h.ease.0 + self.shift(true), h.ease.1 + self.shift(true));
            clamp((a - from) / (full - from), 0.0, 1.0)
        } else {
            0.0
        };
        let give_up = h.concede + self.shift(false);
        let concede = !leading && (step > 0 || a >= give_up || (self.read.concede && a >= give_up - HOLD));
        let back_at = th[0] - REST_BACK;
        let rest_back = a >= back_at || (self.read.rest_back && a >= back_at - HOLD);
        self.read = Read { step, ease, concede, rest_back, seen };
        self.read
    }
}

/// Who sits at each step: the starters by the depth chart, in the real
/// order. Kickers, punters and the long snapper stay.
pub fn benched(depth: &Depth, n: usize, read: &Read) -> Vec<bool> {
    let step = read.step;
    let mut out = vec![false; n];
    let mut sit = |pos: Position, from: usize, count: usize| {
        for &i in depth.list(pos).iter().skip(from).take(count) {
            if i < n {
                out[i] = true;
            }
        }
    };
    if read.rest_back {
        sit(Position::RB, 0, 1);
    }
    if step >= 1 {
        for (pos, k) in [
            (Position::QB, 1),
            (Position::RB, 1),
            (Position::WR, 3),
            (Position::LB, 2),
            (Position::CB, 3),
            (Position::S, 2),
        ] {
            sit(pos, 0, k);
        }
    }
    if step >= 2 {
        for (pos, k) in
            [(Position::TE, 1), (Position::OT, 2), (Position::IOL, 3), (Position::EDGE, 2), (Position::DT, 2)]
        {
            sit(pos, 0, k);
        }
    }
    if step >= 3 {
        for (pos, from, k) in [
            (Position::QB, 1, 1),
            (Position::RB, 1, 1),
            (Position::WR, 3, 3),
            (Position::TE, 1, 1),
            (Position::OT, 2, 2),
            (Position::IOL, 3, 3),
            (Position::EDGE, 2, 2),
            (Position::DT, 2, 2),
            (Position::LB, 2, 2),
            (Position::CB, 3, 2),
            (Position::S, 2, 2),
        ] {
            sit(pos, from, k);
        }
    }
    for pos in [Position::K, Position::P] {
        for &i in depth.list(pos).iter().take(1) {
            if i < n {
                out[i] = false;
            }
        }
    }
    if depth.snapper < n {
        out[depth.snapper] = false;
    }
    out
}

/// Plain words for a step, for the why lines.
pub fn step_name(step: u8) -> &'static str {
    match step {
        0 => "puts his starters back in",
        1 => "pulls his quarterback, skill players and back seven",
        2 => "pulls the rest of his first team",
        _ => "empties the bench",
    }
}
