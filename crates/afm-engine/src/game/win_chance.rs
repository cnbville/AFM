//! The win chance before a snap (update 2, lane 9): the model the game
//! flow board, the game page's chart and its swing plays read. It sits
//! beside the engine's own quick read (`live::win_prob`, what the coaches
//! weigh), which it leaves alone so games play out the same.
//!
//! How it works, the way the public models do (Pro Football Reference's
//! and nflfastR's): the drive on the field first, then the rest of the
//! game. The drive's touchdown and field goal chances come from the spot,
//! are scaled so the drive is worth the real expected points of the down,
//! distance and spot, and shrink when the half's clock is too short to get
//! there. After the drive, the rest of the game's margin is about normal,
//! with the pregame line spread over the time left and a spread that
//! shrinks with the square root of the time left (Stern 1991). A leader who
//! can kneel out the clock against the other side's timeouts has won.
//! Overtime plays by its level's rule. Each level's spread is fitted to its
//! own games on the engine (`tests/win_chance.rs`).

use super::stats::{PlayRecord, PlayType};
use crate::athlete::phi;
use crate::player::Level;
use crate::rng::clamp;

/// What a level's games are like, for the model.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Params {
    pub level: Level,
    /// Seconds in a quarter.
    pub quarter: f64,
    /// The final margin's spread over a whole game, points.
    pub sigma: f64,
    /// Seconds the clock runs between snaps when a team milks it.
    pub play_clock: f64,
    /// The two-minute warning stops the clock (the NFL).
    pub warning: bool,
    /// Field goal makes by distance (under 30, 30-39, 40-49, 50 and over)
    /// and the longest kick a coach tries.
    pub fg: [f64; 4],
    pub fg_max: f64,
    /// The extra point's worth after a touchdown (kicks, with the odd two).
    pub try_pts: f64,
    /// Where the receiving team starts after a kickoff (yards from the
    /// goal it attacks).
    pub start: f64,
    /// Overtime: the NFL's timed period, or college's alternating
    /// possessions from a spot (the 25; high school's Kansas plan, the 10).
    pub ot_spot: Option<f64>,
}

impl Params {
    /// Field goal makes: the NFL and FBS from the real 2021-25 data
    /// (`nfl_game_targets.json`, `college_game_targets.json`), FCS and
    /// small colleges a step below FBS, high school well below (no
    /// national data; proposed, not confirmed). The spreads were fitted to
    /// each level's games on the engine.
    pub fn for_level(level: Level) -> Params {
        match level {
            Level::NFL => Params {
                level,
                quarter: 900.0,
                sigma: 13.5,
                play_clock: 40.0,
                warning: true,
                fg: [0.98, 0.934, 0.798, 0.686],
                fg_max: 62.0,
                try_pts: 0.955,
                start: 71.0,
                ot_spot: None,
            },
            Level::FBS => Params {
                level,
                quarter: 900.0,
                sigma: 16.0,
                play_clock: 40.0,
                warning: false,
                fg: [0.921, 0.807, 0.652, 0.506],
                fg_max: 57.0,
                try_pts: 0.97,
                start: 73.0,
                ot_spot: Some(25.0),
            },
            Level::FCS | Level::SmallCollege => Params {
                level,
                quarter: 900.0,
                sigma: 16.5,
                play_clock: 40.0,
                warning: false,
                fg: [0.88, 0.76, 0.6, 0.45],
                fg_max: 55.0,
                try_pts: 0.95,
                start: 73.0,
                ot_spot: Some(25.0),
            },
            Level::HS => Params {
                level,
                quarter: 720.0,
                sigma: 18.0,
                play_clock: 40.0,
                warning: false,
                fg: [0.8, 0.6, 0.36, 0.15],
                fg_max: 47.0,
                try_pts: 0.88,
                start: 68.0,
                ot_spot: Some(10.0),
            },
        }
    }

    fn length(&self) -> f64 {
        4.0 * self.quarter
    }

    /// The chance a kick from `yl` yards out (the snap 7 behind the line,
    /// the posts 10 past the goal line) is good.
    pub fn fg_make(&self, yl: f64) -> f64 {
        let d = yl + 17.0;
        if d > self.fg_max {
            return 0.0;
        }
        // Straight lines between the bins' middles.
        let mids = [25.0, 35.0, 45.0, 55.0];
        let f = &self.fg;
        if d <= mids[0] {
            return clamp(f[0] + 0.004 * (mids[0] - d), 0.0, 0.995);
        }
        for k in 0..3 {
            if d <= mids[k + 1] {
                let s = (d - mids[k]) / 10.0;
                return f[k] + (f[k + 1] - f[k]) * s;
            }
        }
        let s = (d - mids[3]) / (self.fg_max - mids[3]).max(1.0);
        (f[3] * (1.0 - 0.7 * s)).max(0.05)
    }

    /// The spot a coach still kicks from: the longest try, a few yards
    /// short of it when the game doesn't make him.
    fn fg_line(&self) -> f64 {
        self.fg_max - 17.0 - 4.0
    }
}

/// The game at a snap, from the side of the team with the ball.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    pub quarter: u8,
    /// Seconds left in the quarter.
    pub clock: f64,
    /// The offense's lead.
    pub lead: f64,
    pub down: u8,
    pub togo: f64,
    /// Yards from the goal it attacks.
    pub yl: f64,
    /// Timeouts left: the offense's, the defense's.
    pub timeouts: [u8; 2],
    /// What the offense was expected to win by before the game (the
    /// closing line's spread turned round; 0 with no line).
    pub spread: f64,
}

/// The current drive's outcomes.
struct Drive {
    td: f64,
    fg: f64,
    /// The other side scores on the drive (a pick six, a fumble return).
    def_td: f64,
    /// What the other side expects from where it gets the ball when the
    /// drive ends without points.
    opp_after: f64,
    /// Seconds the drive takes when it ends in a touchdown, a field goal
    /// and anything else.
    secs: [f64; 3],
}

/// Expected points at a spot, read between the table's 5-yard rows (each
/// row taken as its middle yard) so a yard closer never drops a step.
fn ep_at(down: u8, togo: f64, yl: f64) -> f64 {
    let yl = clamp(yl, 1.0, 99.0);
    let lo = ((yl - 2.5) / 5.0).floor() * 5.0;
    let f = (yl - (lo + 2.5)) / 5.0;
    let row = |y: f64| super::tables::expected_points(down, togo, clamp(y, 1.0, 99.0));
    (1.0 - f) * row(lo) + f * row(lo + 5.0)
}

/// The other side's expected points from its first snap at `yl`.
fn ep1(yl: f64) -> f64 {
    ep_at(1, 10.0, yl)
}

/// How much of a drive's chances survive a short clock: `avail` seconds
/// for a drive that needs `need`.
fn in_time(avail: f64, need: f64) -> f64 {
    if need <= 0.0 {
        return 1.0;
    }
    1.0 - (-1.6 * avail.max(0.0) / need).exp()
}

/// The chances of the drive on the field (`half_left` seconds left in the
/// half).
fn drive(p: &Params, s: &State, half_left: f64, late: bool) -> Drive {
    let yl = clamp(s.yl, 0.5, 99.5);
    // From the spot, before down and distance: touchdowns about as real
    // NFL drives score them (a fifth from the 25, three in five from the
    // 10), field goals as often as drives stall in range.
    let td0 = 0.15 + 0.65 * (-yl / 28.0).exp();
    let range = p.fg_line();
    let attempt = 0.2 + 0.4 / (1.0 + ((yl - (range - 8.0)) / 6.0).exp());
    let make = p.fg_make(yl.min(range).max(2.0));
    let fg0 = (1.0 - td0) * attempt * make;
    let def_td = 0.012;
    // Where the other side starts when the drive stalls: a punt from the
    // offense's own end (net 42, a touchback at its 20), the spot of the
    // kick or the turnover in range. Drives stall at a spread of spots, so
    // near the edge of range it is part punt, part missed kick.
    let end = (yl - 6.0).max(1.0);
    let punt = 100.0 - (end - 42.0).max(20.0);
    let miss = 100.0 - (end + 7.0).min(99.0);
    let w = clamp((end - (range - 6.0)) / 12.0, 0.0, 1.0);
    let opp_yl = w * punt + (1.0 - w) * miss;
    let opp_after = ep1(opp_yl);
    // Scale the scoring chances so the drive is worth the table's expected
    // points for this down and distance (a 3rd and 15 is worth less than a
    // 1st and 10 from the same spot).
    let ep = ep_at(s.down.max(1), s.togo.max(0.5), yl);
    let kick_back = ep1(p.start);
    let gain = td0 * (6.0 + p.try_pts - kick_back) + fg0 * (3.0 - kick_back);
    let lam = (ep + 7.0 * def_td + (1.0 - def_td) * opp_after) / (gain + (td0 + fg0) * opp_after).max(0.05);
    let lam = clamp(lam, 0.02, 0.97 / (td0 + fg0));
    let (mut td, mut fg) = (td0 * lam, fg0 * lam);
    // A short clock: the touchdown needs the field, a field goal only the
    // ground to range. A two-minute drive gains about six yards a snap; a
    // snap takes about six seconds of play and, unless a timeout stops
    // the clock, about sixteen more before the next one.
    let to = s.timeouts[0] as f64 + if p.warning && half_left > 120.0 { 1.0 } else { 0.0 };
    let snaps_td = (yl / 6.0).ceil().max(1.0);
    let snaps_fg = ((yl - range) / 6.0).max(0.0).ceil() + 1.0;
    let need = |snaps: f64| (snaps * 6.0 + 16.0 * (snaps - 1.0 - to.min(snaps - 1.0))).max(4.0);
    let ft = in_time(half_left, need(snaps_td));
    let ff = in_time(half_left, need(snaps_fg));
    // One snap left: a shot at the end zone (a Hail Mary from midfield
    // lands about one time in twenty).
    let shot = if half_left >= 3.0 { 0.55 * (-yl / 14.0).exp() } else { 0.0 };
    let td_t = (td * ft).max(shot.min(td));
    // A team that runs short of time plays for the kick when it is close
    // enough: given no touchdown, it kicks whenever it gets to range. On
    // fourth down outside range it has no snaps left to get there.
    let fg_t = if late && yl <= range + 12.0 && (s.down < 4 || yl <= range) {
        let given = (fg / (1.0 - td).max(0.05)).max(0.9 * make * ff);
        ((1.0 - td_t) * given.min(make)).max(fg * ff)
    } else {
        fg * ff
    };
    td = td_t;
    fg = fg_t.min(1.0 - td - def_td);
    // How long the drive takes: a scoring drive runs the field at about
    // eight yards a snap, a kick needs only the ground to range, and a
    // drive that stalls lasts about four and a half snaps wherever it
    // starts. About 27 seconds a snap at the usual pace, 14 in a hurry,
    // and 36 for a side level or ahead burning the clock late in the
    // fourth, less about 30 for each timeout the other side spends; that
    // side runs its four downs before it scores, wherever it is. A side
    // in a hurry scores when it scores: the time it takes is the short
    // clock's own, so a score from the 1 leaves more time behind it than
    // one from the 18, not less.
    let hurry = late && s.lead < 0.0;
    let milk = late && s.lead >= 0.0 && s.quarter >= 4;
    let floor = if milk { 4.0 } else { 0.0 };
    let score = (1.0 + yl / 8.0).max(floor);
    let kick = (2.0 + (yl - (range - 8.0)).max(0.0) / 8.0).max(floor).min(score);
    let secs = |n: f64| {
        let t = if hurry {
            n * 14.0
        } else if milk {
            n * 36.0 - 30.0 * (s.timeouts[1] as f64).min(n - 1.0)
        } else {
            n * 27.0
        };
        t.min(half_left)
    };
    let (t_td, t_fg) = if hurry {
        (time_to_score(half_left, need(snaps_td)), time_to_score(half_left, need(snaps_fg)))
    } else {
        (secs(score), secs(kick))
    };
    Drive { td, fg, def_td, opp_after, secs: [t_td, t_fg, secs(4.5)] }
}

/// The seconds a score takes when it comes in time: `in_time` reads the
/// time to it as about exponential, so this is that time's mean below
/// `avail`.
fn time_to_score(avail: f64, need: f64) -> f64 {
    let a = avail.max(0.0);
    let m = need.max(1.0) / 1.6;
    let e = (-a / m).exp();
    if 1.0 - e < 1e-9 {
        return a / 2.0;
    }
    (m - a * e / (1.0 - e)).clamp(0.0, a)
}

/// Seconds a leader can run off by kneeling from this down, against the
/// other side's timeouts (and the two-minute warning when it hasn't come).
fn kneel_burn(p: &Params, s: &State, game_left: f64) -> f64 {
    let n = (5 - s.down.clamp(1, 4)) as f64;
    let stops = (s.timeouts[1] as f64 + if p.warning && game_left > 120.0 { 1.0 } else { 0.0 }).min(n - 1.0);
    n * 1.5 + (n - 1.0 - stops) * (p.play_clock - 1.0)
}

/// The chance of a final margin over 0 (a tie counts half) from a mean
/// and a spread.
fn over(mean: f64, sd: f64) -> f64 {
    let sd = sd.max(0.25);
    0.5 * (phi((mean - 0.5) / sd) + phi((mean + 0.5) / sd))
}

/// The offense's chance to win from the snap. The drive's split of the
/// spot's points into touchdowns, kicks and stops has small kinks where
/// kicks give way to punts and the clock gets short, so a spot is read as
/// never worth less than one up to 25 yards further back.
pub fn chance(p: &Params, s: &State) -> f64 {
    if s.quarter > 4 {
        return overtime(p, s);
    }
    (0..=25).map(|k| from_spot(p, &State { yl: (s.yl + k as f64).min(99.0).max(s.yl), ..*s })).fold(0.0, f64::max)
}

/// The chance from this exact spot.
fn from_spot(p: &Params, s: &State) -> f64 {
    let q = s.quarter.clamp(1, 4) as f64;
    let half_left = s.clock + if s.quarter % 2 == 1 { p.quarter } else { 0.0 };
    let game_left = s.clock + (4.0 - q) * p.quarter;
    // The leader kneels it out.
    if s.lead > 0.0 && s.quarter == 4 && s.yl > 2.0 && game_left <= kneel_burn(p, s, game_left) {
        return 0.999;
    }
    let late = half_left < 300.0;
    let d = drive(p, s, half_left, late);
    // After the drive: the rest of the game. The other side gets the ball
    // on a kickoff after points; a drive that ends the half leaves the next
    // half even. The other side's next drive needs time too (the end of
    // the half).
    let g = p.length();
    let drift = s.spread * game_left / g;
    let base = s.lead + drift;
    let rest = |secs: f64, pts: f64, opp_ep: f64| {
        let after = (game_left - secs).max(0.0);
        let half_after = (half_left - secs).max(0.0);
        let sd = p.sigma * (after / g).sqrt();
        let opp = if half_after <= 1.0 { 0.0 } else { opp_ep * in_time(half_after, 110.0) };
        over(base + pts - opp, sd)
    };
    let kick_back = ep1(p.start);
    let none = 1.0 - d.td - d.fg - d.def_td;
    let w = d.td * rest(d.secs[0], 6.0 + p.try_pts, kick_back)
        + d.fg * rest(d.secs[1], 3.0, kick_back)
        + d.def_td * rest(d.secs[2], -6.0 - p.try_pts, -kick_back)
        + none.max(0.0) * rest(d.secs[2], 0.0, d.opp_after);
    clamp(w, 0.001, 0.999)
}

/// Overtime: the NFL's timed period plays like the end of a game with a
/// tie as half; college and high school go possession for possession from
/// a spot, the team that answers knowing what it needs.
fn overtime(p: &Params, s: &State) -> f64 {
    let Some(spot) = p.ot_spot else {
        let s2 = State { quarter: 4, clock: s.clock.min(600.0), ..*s };
        return chance(p, &s2);
    };
    // A possession's points from the spot: the offense here, then the
    // answer.
    let lead = s.lead.round();
    let probe = State { quarter: 1, clock: 900.0, ..*s };
    let d = drive(p, &probe, 900.0, false);
    let fresh = drive(p, &State { down: 1, togo: 10.0_f64.min(spot), yl: spot, lead: 0.0, ..probe }, 900.0, false);
    let outs = |d: &Drive| {
        let none = (1.0 - d.td - d.fg).max(0.0);
        [(6.0 + p.try_pts, d.td), (3.0, d.fg), (0.0, none)]
    };
    // A tie after both possessions: another round, even.
    let settle = |m: f64| {
        if m > 0.5 {
            1.0
        } else if m < -0.5 {
            0.0
        } else {
            0.5
        }
    };
    if lead == 0.0 {
        // The first possession: the other side answers from the spot.
        let mut w = 0.0;
        for (pts, pr) in outs(&d) {
            let mut ans = 0.0;
            for (op, opr) in outs(&fresh) {
                ans += opr * settle(pts - op);
            }
            w += pr * ans;
        }
        w
    } else {
        // The answer: the leader already had his.
        let mut w = 0.0;
        for (pts, pr) in outs(&d) {
            w += pr * settle(lead + pts);
        }
        clamp(w, 0.001, 0.999)
    }
}

/// The home team's chance before a play in the log, with the home side's
/// expected margin before the game (`home_spread`, 0 with no line).
pub fn home_before(p: &Params, r: &PlayRecord, home_spread: f64) -> f64 {
    let to = r.timeouts.unwrap_or([2, 2]);
    let home_lead = (r.score[0] - r.score[1]) as f64;
    let o = r.offense.min(1);
    let sign = if o == 0 { 1.0 } else { -1.0 };
    let mut s = State {
        quarter: r.quarter,
        clock: r.clock.max(0.0),
        lead: sign * home_lead,
        down: r.down.max(1),
        togo: if r.togo > 0.0 { r.togo } else { 10.0 },
        yl: r.yardline_100,
        timeouts: [to[o], to[1 - o]],
        spread: sign * home_spread,
    };
    let off = match r.kind {
        // The kicking team is the offense on a kickoff: the other side
        // takes it at about its usual start.
        PlayType::Kickoff => {
            s = State { lead: -s.lead, spread: -s.spread, timeouts: [s.timeouts[1], s.timeouts[0]], ..s };
            s.down = 1;
            s.togo = 10.0;
            s.yl = p.start;
            1.0 - chance(p, &s)
        }
        // The try: its worth, then the kickoff.
        PlayType::ExtraPoint | PlayType::TwoPoint => {
            let worth = if r.kind == PlayType::TwoPoint { 0.95 } else { p.try_pts };
            let k = State {
                lead: -(s.lead + worth),
                spread: -s.spread,
                timeouts: [s.timeouts[1], s.timeouts[0]],
                down: 1,
                togo: 10.0,
                yl: p.start,
                ..s
            };
            1.0 - chance(p, &k)
        }
        _ => chance(p, &s),
    };
    if o == 0 { off } else { 1.0 - off }
}
