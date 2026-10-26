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
//! shrinks with the square root of the time left (Stern 1991), a little
//! wider a minute in the last ten. A leader who can kneel out the clock
//! against the other side's timeouts has won. Late, the habits blend in
//! with the clock rather than switch on at one moment: the leader burns
//! the clock, the side behind picks its pace and anyone short of time
//! plays for the kick, which it must first get in range for. Overtime
//! plays by its level's rule, and in college and high school the side
//! that answers knows what it needs. Each level's spread is fitted to its
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
    /// How far apart two of the level's sides are, as the spread of the
    /// lines books hang (points over a game): what a game with no line
    /// leaves unknown.
    pub gap_sd: f64,
    /// No line was hung: the score so far also hints which side is better.
    pub no_line: bool,
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
                gap_sd: 6.0,
                no_line: false,
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
                gap_sd: 14.0,
                no_line: false,
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
                gap_sd: 14.0,
                no_line: false,
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
                gap_sd: 17.0,
                no_line: false,
            },
        }
    }

    /// The same level for a game with no line (high school's always).
    pub fn without_line(self) -> Params {
        Params { no_line: true, ..self }
    }

    fn length(&self) -> f64 {
        4.0 * self.quarter
    }

    /// The chance a kick from `yl` yards out (the snap 7 behind the line,
    /// the posts 10 past the goal line) is good.
    pub fn fg_make(&self, yl: f64) -> f64 {
        let d = yl + 17.0;
        if d >= self.fg_max {
            return 0.0;
        }
        // Over the last seven yards to the longest kick the make falls
        // away to nothing, with no cliff at the end.
        let s = (d - (self.fg_max - 7.0)).max(0.0) / 7.0;
        self.fg_line_make(d) * (1.0 - s * s)
    }

    /// The make by distance before the taper.
    fn fg_line_make(&self, d: f64) -> f64 {
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
        f[3]
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
    /// College and high school overtime: the other side has had its
    /// possession in this round, so this one answers it.
    pub answering: bool,
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

/// How a side runs its drive late in the game: at its usual pace, in a
/// hurry, or slowly to leave the other side little time after it scores.
/// The side behind in the fourth picks whichever gives it the best chance.
#[derive(Clone, Copy, PartialEq)]
enum Pace {
    Usual,
    Hurry,
    Slow,
}

/// The late habits, in seconds left: a side level or ahead starts burning
/// the clock from 7:00 of the fourth and does it fully from 3:00; a side
/// short of time plays for the kick from 6:00 of the half, fully from
/// 4:00.
const MILK_FROM: f64 = 420.0;
const MILK_RAMP: f64 = 240.0;
const KICK_FROM: f64 = 360.0;
const KICK_RAMP: f64 = 120.0;
/// The chance of getting from outside range into it falls off about this
/// many yards past the edge (and a fourth down out there is one snap).
const REACH: f64 = 25.0;
const FOURTH_REACH: f64 = 0.45;
/// The two-minute warning eases in over its last half minute: a snap
/// just before it runs into it.
const WARN_RAMP: f64 = 30.0;
/// The margin moves more a minute in the last ten: the side behind
/// hurries, goes for it and kicks onside.
const LATE_SPREAD: f64 = 600.0;
/// A drive's length varies: it is read at seven lengths around its usual
/// one, weighted like a bell (sixteenths).
const LENGTHS: [(f64, f64); 7] =
    [(0.55, 1.0), (0.7, 2.0), (0.85, 3.0), (1.0, 4.0), (1.15, 3.0), (1.3, 2.0), (1.45, 1.0)];

fn ramp(x: f64, r: f64) -> f64 {
    clamp(x / r, 0.0, 1.0)
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

/// The two-minute warning as a stop still to come.
fn warning_stop(p: &Params, half_left: f64) -> f64 {
    if p.warning { ramp(half_left - 120.0, WARN_RAMP) } else { 0.0 }
}

/// The chances of the drive on the field (`half_left` and `game_left`
/// seconds left). The split of its points comes from `probe` (the spot or
/// one further back, see `chance`), the clock from the real spot.
fn drive(p: &Params, s: &State, half_left: f64, game_left: f64, probe: f64, pace: Pace) -> Drive {
    let yl = clamp(probe, 0.5, 99.5);
    let real = clamp(s.yl, 0.5, 99.5);
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
    let (td, fg) = (td0 * lam, fg0 * lam);
    // The late habits: the side level or ahead burns the clock more as
    // the end nears; the side behind in the fourth picks its pace; anyone
    // short of time plays for the kick.
    let mut hurry = if pace == Pace::Hurry { 1.0 } else { 0.0 };
    let mut milk = if pace == Pace::Slow { 1.0 } else { 0.0 };
    if s.quarter >= 4 && s.lead >= 0.0 {
        hurry = 0.0;
        milk = ramp(MILK_FROM - game_left, MILK_RAMP);
    }
    let kickw = ramp(KICK_FROM - half_left, KICK_RAMP);
    // The clock, from the real spot. A two-minute drive gains about six
    // yards a snap; a snap takes about six seconds of play and, unless a
    // timeout stops the clock, about sixteen more before the next one. A
    // snap that starts in time is played out, so the last needs no clock.
    let to = s.timeouts[0] as f64 + warning_stop(p, half_left);
    let snaps_td = (real / 6.0).ceil().max(1.0);
    let snaps_fg = ((real - range) / 6.0).max(0.0).ceil() + 1.0;
    let gap = |n: f64| 6.0 * (n - 1.0) + 16.0 * (n - 1.0 - to).max(0.0);
    let ft = if snaps_td > 1.0 { in_time(half_left, gap(snaps_td)) } else { 1.0 };
    let ff = if snaps_fg > 1.0 { in_time(half_left, gap(snaps_fg)) } else { 1.0 };
    // A shot at the end zone (a Hail Mary from midfield lands about one
    // time in twenty).
    let shot = if half_left > 0.0 { 0.55 * (-real / 14.0).exp() } else { 0.0 };
    let td_c = (td * ft).max(shot.min(td));
    // Given no touchdown, a side short of time kicks once it gets to
    // range: from outside it has to get there first.
    let beyond = (real - range).max(0.0);
    let reach = (-beyond / REACH).exp() * if s.down >= 4 && beyond > 0.0 { FOURTH_REACH } else { 1.0 };
    let fg_late = ((1.0 - td_c) * reach * ff * make).max(fg * ff);
    let fg_c = (1.0 - kickw) * fg * ff + kickw * fg_late;
    // The half's last snap: a kick from the real spot (nothing past the
    // longest kick), or a shot at the end zone, whichever is worth more.
    let more = in_time(half_left, gap(2.0));
    let last = 1.0 - more;
    let kick_now = p.fg_make(real);
    let (td_l, fg_l) = if s.quarter >= 4 {
        if s.lead > 0.0 {
            (0.0, 0.0)
        } else if s.lead >= -3.0 {
            // A kick that only ties is worth half a win.
            let k = kick_now * if s.lead <= -2.5 { 0.5 } else { 1.0 };
            if k >= shot { (0.0, kick_now) } else { (shot, 0.0) }
        } else {
            (shot, 0.0)
        }
    } else if 3.0 * kick_now >= (6.0 + p.try_pts) * shot {
        (0.0, kick_now)
    } else {
        (shot, 0.0)
    };
    let td = last * td_l + more * td_c;
    let fg = (last * fg_l + more * fg_c).min(1.0 - td - def_td);
    // How long it takes, from the real spot: a scoring drive runs the
    // field at about eight yards a snap, a kick needs only the ground to
    // range, and a drive that stalls lasts about four and a half snaps.
    // About 27 seconds a snap at the usual pace and 14 in a hurry; slowly,
    // 36 less about 30 for each timeout the other side spends. A side
    // burning the clock runs its four downs before it scores. A side in a
    // hurry scores when it scores: the short clock's own time. The side
    // behind in the fourth can take as long as a drive from the probe spot
    // (it never has to score sooner than from further back), and slowly it
    // can run the play clock down to its last two seconds every snap, less
    // what the other side's timeouts save.
    let floor = 4.0 * milk;
    let behind = s.quarter >= 4 && s.lead < 0.0;
    let span = if behind { yl } else { real };
    let score = (1.0 + span / 8.0).max(floor);
    let kick = (2.0 + (span - (range - 8.0)).max(0.0) / 8.0).max(floor).min(score);
    let (per_snap, per_timeout) = if behind { (p.play_clock + 4.0, p.play_clock - 2.0) } else { (36.0, 30.0) };
    let secs = |n: f64| {
        let slow = n * per_snap - per_timeout * (s.timeouts[1] as f64).min((n - 1.0).max(0.0));
        let t = (1.0 - hurry - milk) * n * 27.0 + hurry * n * 14.0 + milk * slow;
        t.min(half_left)
    };
    let (t_td, t_fg) = if hurry > 0.0 {
        (time_to_score(half_left, gap(snaps_td) + 6.0), time_to_score(half_left, gap(snaps_fg) + 6.0))
    } else {
        (secs(score), secs(kick))
    };
    Drive { td, fg, def_td, opp_after, secs: [t_td.min(half_left), t_fg.min(half_left), secs(4.5)] }
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
    let stops = (s.timeouts[1] as f64 + warning_stop(p, game_left)).min(n - 1.0);
    n * 1.5 + (n - 1.0 - stops) * (p.play_clock - 1.0)
}

/// The chance of a final margin over 0 (a tie counts half) from a mean
/// and a spread.
fn over(mean: f64, sd: f64) -> f64 {
    let sd = sd.max(0.25);
    0.5 * (phi((mean - 0.5) / sd) + phi((mean + 0.5) / sd))
}

/// The rest of the game's spread with `after` seconds left: about the
/// square root of the time, and more a minute in the last ten.
fn rest_sd(p: &Params, after: f64) -> f64 {
    let g = p.length();
    let boost = ramp(LATE_SPREAD - after, LATE_SPREAD);
    p.sigma * (after * (1.0 + boost) / g).sqrt()
}

/// The offense's chance to win from the snap. The drive's split of the
/// spot's points into touchdowns, kicks and stops has small kinks where
/// kicks give way to punts, so the split is read as never worth less than
/// from a spot up to 25 yards further back (the clock still runs from the
/// real spot). The side behind in the fourth picks its pace.
pub fn chance(p: &Params, s: &State) -> f64 {
    if s.quarter > 4 {
        return overtime(p, s);
    }
    let paces: &[Pace] =
        if s.quarter >= 4 && s.lead < 0.0 { &[Pace::Usual, Pace::Hurry, Pace::Slow] } else { &[Pace::Usual] };
    let mut best = 0.0f64;
    for k in 0..=25 {
        let probe = (s.yl + k as f64).min(99.0).max(s.yl);
        for &pace in paces {
            best = best.max(from_spot(p, s, probe, pace));
        }
    }
    best
}

/// The chance with the drive's split read from `probe` at one pace.
fn from_spot(p: &Params, s: &State, probe: f64, pace: Pace) -> f64 {
    let q = s.quarter.clamp(1, 4) as f64;
    let half_left = s.clock + if s.quarter % 2 == 1 { p.quarter } else { 0.0 };
    let game_left = s.clock + (4.0 - q) * p.quarter;
    // The leader kneels it out.
    if s.lead > 0.0 && s.quarter == 4 && s.yl > 2.0 && game_left <= kneel_burn(p, s, game_left) {
        return 0.999;
    }
    let d = drive(p, s, half_left, game_left, probe, pace);
    // After the drive: the rest of the game. The other side gets the ball
    // on a kickoff after points; a drive that ends the half leaves the next
    // half even. The other side's next drive needs time too.
    // With no line, the gap between the sides is unknown: the lead beyond
    // what the line said so far hints at it, as a normal prior updated by
    // the score (the gap's spread from the books' lines, the play's from
    // `sigma`).
    let len = p.length();
    let played = (1.0 - game_left / len).clamp(0.0, 1.0);
    let tau2 = if p.no_line { p.gap_sd * p.gap_sd } else { 0.0 };
    let sig2 = p.sigma * p.sigma;
    let hint = tau2 / (tau2 * played + sig2);
    let gap_mean = (s.lead - s.spread * played) * hint;
    let gap_var = tau2 * sig2 / (tau2 * played + sig2);
    let drift = (s.spread + gap_mean) * game_left / len;
    let base = s.lead + drift;
    let rest = |secs: f64, pts: f64, opp_ep: f64| {
        let mut tot = 0.0;
        for (f, wt) in LENGTHS {
            let t = (secs * f).min(half_left);
            let after = (game_left - t).max(0.0);
            let half_after = (half_left - t).max(0.0);
            let opp = opp_ep * in_time(half_after, 110.0);
            let sd = (rest_sd(p, after).powi(2) + (after / len).powi(2) * gap_var).sqrt();
            tot += wt * over(base + pts - opp, sd);
        }
        tot / 16.0
    };
    let kick_back = ep1(p.start);
    let none = 1.0 - d.td - d.fg - d.def_td;
    let w = d.td * rest(d.secs[0], 6.0 + p.try_pts, kick_back)
        + d.fg * rest(d.secs[1], 3.0, kick_back)
        + d.def_td * rest(d.secs[2], -6.0 - p.try_pts, -kick_back)
        + none.max(0.0) * rest(d.secs[2], 0.0, d.opp_after);
    clamp(w, 0.001, 0.999)
}

/// A round of college or high school overtime settled on a margin: a tie
/// goes to another round, even.
fn settle(m: f64) -> f64 {
    if m > 0.5 {
        1.0
    } else if m < -0.5 {
        0.0
    } else {
        0.5
    }
}

/// Overtime: the NFL's timed period plays like the end of a game with a
/// tie as half; college and high school go possession for possession from
/// a spot, the side that answers knowing what it needs.
fn overtime(p: &Params, s: &State) -> f64 {
    let Some(spot) = p.ot_spot else {
        let s2 = State { quarter: 4, clock: s.clock.min(600.0), ..*s };
        return chance(p, &s2);
    };
    let lead = s.lead.round();
    let probe = State { quarter: 1, clock: 900.0, ..*s };
    let d = drive(p, &probe, 900.0, 3600.0, probe.yl, Pace::Usual);
    let outs = |d: &Drive| {
        let none = (1.0 - d.td - d.fg).max(0.0);
        [(6.0 + p.try_pts, d.td), (3.0, d.fg), (0.0, none)]
    };
    if lead == 0.0 && !s.answering {
        // The first possession: the other side answers from the spot.
        let fresh_s = State { down: 1, togo: 10.0_f64.min(spot), yl: spot, lead: 0.0, ..probe };
        let fresh = drive(p, &fresh_s, 900.0, 3600.0, spot, Pace::Usual);
        let mut w = 0.0;
        for (pts, pr) in outs(&d) {
            let ans: f64 = outs(&fresh).iter().map(|&(op, opr)| opr * settle(pts - op)).sum();
            w += pr * ans;
        }
        clamp(w, 0.001, 0.999)
    } else {
        // The answer: the other side already had its possession (level, a
        // score wins it).
        let w: f64 = outs(&d).iter().map(|&(pts, pr)| pr * settle(lead + pts)).sum();
        clamp(w, 0.001, 0.999)
    }
}

/// The home team's chance before a play in the log, with the home side's
/// expected margin before the game (`home_spread`, 0 with no line). A
/// play in college or high school overtime needs to know whether its side
/// is answering: `home_series` follows that through a game.
pub fn home_before(p: &Params, r: &PlayRecord, home_spread: f64) -> f64 {
    home_before_in(p, r, home_spread, false)
}

/// The game at a record, from its offense's side.
fn state_of(r: &PlayRecord, home_spread: f64, answering: bool) -> State {
    let to = r.timeouts.unwrap_or([2, 2]);
    let home_lead = (r.score[0] - r.score[1]) as f64;
    let o = r.offense.min(1);
    let sign = if o == 0 { 1.0 } else { -1.0 };
    State {
        quarter: r.quarter,
        clock: r.clock.max(0.0),
        lead: sign * home_lead,
        down: r.down.max(1),
        togo: if r.togo > 0.0 { r.togo } else { 10.0 },
        yl: r.yardline_100,
        timeouts: [to[o], to[1 - o]],
        spread: sign * home_spread,
        answering,
    }
}

/// The other side with the ball after this side's try or kick (`lead`
/// is this side's), at `yl`.
fn other(s: &State, lead: f64, yl: f64, answering: bool) -> State {
    State {
        lead: -lead,
        spread: -s.spread,
        timeouts: [s.timeouts[1], s.timeouts[0]],
        down: 1,
        togo: 10.0_f64.min(yl),
        yl,
        answering,
        ..*s
    }
}

/// The kicking side's chance before a kickoff: the other side takes it
/// at about its usual start.
fn kickoff_value(p: &Params, s: &State) -> f64 {
    1.0 - chance(p, &other(s, s.lead, p.start, false))
}

/// The scoring side's chance before its try (`s.lead` counts the
/// touchdown).
fn try_value(p: &Params, s: &State, two: bool) -> f64 {
    let (pts, make) = if two { (2.0, 0.475) } else { (1.0, p.try_pts) };
    match p.ot_spot.filter(|_| s.quarter > 4) {
        // College and high school overtime: the try ends the possession.
        // Answering, it settles the round; else the other side answers
        // from the spot, with no kickoff.
        Some(spot) => {
            if s.answering {
                make * settle(s.lead + pts) + (1.0 - make) * settle(s.lead)
            } else {
                make * (1.0 - chance(p, &other(s, s.lead + pts, spot, true)))
                    + (1.0 - make) * (1.0 - chance(p, &other(s, s.lead, spot, true)))
            }
        }
        // The try's worth, then the kickoff.
        None => 1.0 - chance(p, &other(s, s.lead + pts * make, p.start, false)),
    }
}

fn home_before_in(p: &Params, r: &PlayRecord, home_spread: f64, answering: bool) -> f64 {
    let s = state_of(r, home_spread, answering);
    let off = match r.kind {
        // The kicking team is the offense on a kickoff.
        PlayType::Kickoff => kickoff_value(p, &s),
        PlayType::ExtraPoint | PlayType::TwoPoint => try_value(p, &s, r.kind == PlayType::TwoPoint),
        _ => chance(p, &s),
    };
    if r.offense.min(1) == 0 { off } else { 1.0 - off }
}

/// Who has had the ball in a round of college or high school overtime. A
/// possession ends with its try, its kick or a turnover, or when the
/// other side has the ball; a side with the ball again starts a round.
#[derive(Clone, Copy, Default)]
struct Rounds {
    had: [bool; 2],
    cur: Option<usize>,
    round: u32,
}

impl Rounds {
    fn start(&mut self, o: usize) {
        if self.cur != Some(o) {
            if let Some(c) = self.cur {
                self.had[c] = true;
            }
            if self.had[o] || self.round == 0 {
                self.had = [false; 2];
                self.round += 1;
            }
            self.cur = Some(o);
        }
    }

    /// Counts a record; whether its side was answering.
    fn step(&mut self, r: &PlayRecord) -> bool {
        let o = r.offense.min(1);
        self.start(o);
        let answering = self.had[1 - o];
        let ends = matches!(r.kind, PlayType::ExtraPoint | PlayType::TwoPoint | PlayType::FieldGoal) || r.turnover;
        if ends && !r.nullified {
            self.had[o] = true;
            self.cur = None;
        }
        answering
    }

    /// Whether side `o` with the ball now would be answering.
    fn answering(&self, o: usize) -> bool {
        let mut x = *self;
        x.start(o);
        x.had[1 - o]
    }
}

/// Follows a log's untimed overtime: the rounds so far.
fn rounds_of(p: &Params, plays: &[PlayRecord]) -> Rounds {
    let mut k = Rounds::default();
    for r in plays.iter().filter(|r| r.offense < 2 && r.quarter > 4 && p.ot_spot.is_some()) {
        k.step(r);
    }
    k
}

/// The home team's chance before each play of a game's log, one for each
/// record (0.5 for a record off the scrimmage sides). In college and high
/// school overtime it follows each round's possessions, so the side that
/// answers knows what it needs.
pub fn home_series(p: &Params, plays: &[PlayRecord], home_spread: f64) -> Vec<f64> {
    let mut k = Rounds::default();
    plays
        .iter()
        .map(|r| {
            if r.offense > 1 {
                return 0.5;
            }
            let answering = r.quarter > 4 && p.ot_spot.is_some() && k.step(r);
            home_before_in(p, r, home_spread, answering)
        })
        .collect()
}

/// Each record's overtime round in college and high school overtime
/// (0 in regulation, in a timed overtime and off the scrimmage sides).
pub fn ot_rounds(plays: &[PlayRecord]) -> Vec<u32> {
    let mut k = Rounds::default();
    plays
        .iter()
        .map(|r| {
            if r.offense > 1 || r.quarter <= 4 || r.clock < 1.0e6 {
                return 0;
            }
            k.step(r);
            k.round
        })
        .collect()
}

/// What comes next in a game under way.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Next {
    Snap,
    Try { two: bool },
    Kickoff,
}

/// The home side's chance in a game under way, before what comes next:
/// `s` is the game from the side with the ball (the kicking side before a
/// kickoff, the scoring side before its try), without the line and
/// overtime's answering, which come from `home_spread` and the log so far.
pub fn home_now(p: &Params, plays: &[PlayRecord], next: Next, offense: usize, s: State, home_spread: f64) -> f64 {
    let o = offense.min(1);
    let sign = if o == 0 { 1.0 } else { -1.0 };
    let answering = s.quarter > 4 && p.ot_spot.is_some() && rounds_of(p, plays).answering(o);
    let s = State { spread: sign * home_spread, answering, ..s };
    let off = match next {
        Next::Snap => chance(p, &s),
        Next::Try { two } => try_value(p, &s, two),
        Next::Kickoff => kickoff_value(p, &s),
    };
    if o == 0 { off } else { 1.0 - off }
}
