//! The analysts' models behind the Data section's game pages and decisions
//! (Data overhaul items 24-27, lane NB8): the call each 4th down and each
//! try after a touchdown should have been, post-game win expectancy,
//! garbage time, and the concept and coverage of a snap.
//!
//! The 4th-down model works as nfl4th and the public 4th-down bots do: the
//! win chance after each choice from the game's win-chance model
//! (`game::win_chance`, whose spreads are fitted on the engine's games),
//! with how often a try converts, how far a converted try gets, where a
//! punt leaves the other side, how often a kick is good and how often a
//! two-point try scores all fitted on the engine's own plays at each level
//! by `decide_fit`, as `xyards_fit` fits rush yards over expected. The
//! coaches never read it: the engine's own read (`calling::fourth_down`,
//! `live::win_prob`) makes their calls, so it only watches.
//!
//! Post-game win expectancy is Bill Connelly's: from a game's five factors
//! (success rate, explosiveness, field position, finishing drives and
//! turnovers), how often a side with that box wins. A logistic model
//! fitted on the engine's games at each level.
//!
//! Proposed, not confirmed: the bands, a converted try's spot, the six
//! seconds each choice takes, overtime in college and high school left
//! unweighed, a clear call as one point of win chance.

use crate::game::playbook::{PASS_CONCEPTS, RUN_CONCEPTS};
use crate::game::stats::{PlayRecord, PlayType, TeamStats};
use crate::game::win_chance::{Params, State, chance};
use crate::player::Level;

use super::games::Adv;

/// The levels in each table's order (as `XCOMP`'s): high school, FBS, FCS,
/// small colleges, the NFL.
pub fn row(level: Level) -> usize {
    level as usize
}

/// Yards to go bands: 1, 2, 3, 4, 5, 6-7, 8-10, 11-15, 16 and more.
pub const TOGO_BANDS: usize = 9;

pub fn togo_band(togo: f64) -> usize {
    let t = togo.max(0.0).ceil();
    match t {
        t if t <= 1.0 => 0,
        t if t <= 2.0 => 1,
        t if t <= 3.0 => 2,
        t if t <= 4.0 => 3,
        t if t <= 5.0 => 4,
        t if t <= 7.0 => 5,
        t if t <= 10.0 => 6,
        t if t <= 15.0 => 7,
        _ => 8,
    }
}

/// Punt spots: yards to the goal in bands of five from 30 (30-34, 35-39,
/// ... 95-99); a punt from inside the 30 reads the first.
pub const PUNT_BANDS: usize = 14;

pub fn punt_band(yl: f64) -> usize {
    (((yl - 30.0) / 5.0).floor().max(0.0) as usize).min(PUNT_BANDS - 1)
}

/// Field goal distances (the kick's, the snap 7 yards behind the line and
/// the posts 10 past the goal): under 30, 30-34, 35-39, 40-44, 45-49, 50
/// and more.
pub const FG_BANDS: usize = 6;

pub fn fg_band(dist: f64) -> usize {
    match dist {
        d if d < 30.0 => 0,
        d if d < 35.0 => 1,
        d if d < 40.0 => 2,
        d if d < 45.0 => 3,
        d if d < 50.0 => 4,
        _ => 5,
    }
}

/// How often a 3rd or 4th down play gets the first down (or the score),
/// by yards to go, away from the goal line. The tables below are from
/// `decide_fit -- 1500 1` (2026-11-04, after lanes NB4-NB6; refitted
/// 2026-11-07 after minor bugs batch 9): 1,500 engine games a level on
/// synthetic leagues, garbage time and overtime left out.
pub const CONV: [[f64; TOGO_BANDS]; 5] = [
    // High school.
    [0.825, 0.671, 0.611, 0.496, 0.445, 0.378, 0.264, 0.156, 0.062],
    // FBS.
    [0.787, 0.639, 0.586, 0.507, 0.479, 0.408, 0.293, 0.189, 0.090],
    // FCS.
    [0.807, 0.674, 0.572, 0.510, 0.463, 0.419, 0.289, 0.193, 0.092],
    // Small colleges.
    [0.819, 0.668, 0.572, 0.523, 0.461, 0.402, 0.291, 0.171, 0.081],
    // NFL.
    [0.757, 0.604, 0.581, 0.544, 0.512, 0.433, 0.312, 0.219, 0.106],
];

/// The same with the goal line as the line to gain (goal to go): a
/// conversion is a touchdown.
pub const CONV_GOAL: [[f64; TOGO_BANDS]; 5] = [
    // High school.
    [0.692, 0.531, 0.551, 0.426, 0.442, 0.371, 0.194, 0.121, 0.095],
    // FBS.
    [0.621, 0.548, 0.444, 0.454, 0.413, 0.419, 0.312, 0.196, 0.093],
    // FCS.
    [0.662, 0.548, 0.385, 0.435, 0.533, 0.409, 0.257, 0.188, 0.073],
    // Small colleges.
    [0.654, 0.537, 0.469, 0.488, 0.426, 0.462, 0.320, 0.149, 0.089],
    // NFL.
    [0.618, 0.500, 0.427, 0.485, 0.419, 0.378, 0.282, 0.204, 0.000],
];

/// Yards a converted play gains from the line of scrimmage, by yards to
/// go (at least the distance itself).
pub const GAIN: [[f64; TOGO_BANDS]; 5] = [
    // High school.
    [5.9, 8.0, 9.4, 10.8, 11.6, 13.0, 15.4, 19.7, 26.0],
    // FBS.
    [5.6, 8.0, 9.5, 11.1, 12.4, 13.6, 17.1, 20.9, 28.2],
    // FCS.
    [6.2, 8.1, 9.4, 11.6, 12.2, 13.7, 16.5, 20.2, 27.6],
    // Small colleges.
    [6.0, 8.2, 9.6, 11.3, 12.4, 13.7, 16.4, 20.0, 27.8],
    // NFL.
    [5.6, 9.2, 9.7, 11.7, 12.4, 14.0, 17.4, 21.9, 28.0],
];

/// Where a punt leaves the other side: its yards to the goal it attacks at
/// its next snap, by the punt's spot (`punt_band`).
pub const PUNT: [[f64; PUNT_BANDS]; 5] = [
    // High school.
    [84.4, 84.0, 85.1, 82.3, 78.5, 73.0, 69.2, 64.8, 59.5, 54.1, 49.4, 44.1, 39.4, 32.9],
    // FBS.
    [85.9, 85.9, 87.0, 86.5, 84.5, 80.0, 76.2, 72.0, 67.0, 62.5, 57.0, 52.0, 47.4, 43.4],
    // FCS.
    [86.7, 86.1, 86.1, 85.7, 82.5, 77.8, 74.0, 69.3, 64.4, 60.3, 55.0, 49.7, 44.8, 39.5],
    // Small colleges.
    [85.6, 85.5, 85.8, 85.1, 81.0, 76.5, 72.7, 68.3, 63.4, 58.7, 53.2, 48.5, 44.4, 37.7],
    // NFL.
    [87.8, 87.8, 87.8, 87.2, 86.6, 85.3, 81.6, 77.6, 72.2, 67.3, 62.0, 56.7, 52.5, 47.2],
];

/// Field goals made, by distance (`fg_band`).
pub const FG: [[f64; FG_BANDS]; 5] = [
    // High school.
    [0.930, 0.826, 0.782, 0.695, 0.612, 0.612],
    // FBS.
    [0.967, 0.906, 0.855, 0.789, 0.704, 0.668],
    // FCS.
    [0.963, 0.891, 0.827, 0.762, 0.705, 0.651],
    // Small colleges.
    [0.960, 0.896, 0.799, 0.725, 0.655, 0.584],
    // NFL.
    [0.992, 0.965, 0.924, 0.880, 0.804, 0.747],
];

/// Extra points made and two-point tries converted.
pub const XP: [f64; 5] = [0.980, 0.990, 0.987, 0.983, 0.968];
pub const TWO: [f64; 5] = [0.462, 0.402, 0.380, 0.417, 0.468];

/// How many terms post-game win expectancy reads.
pub const PGWE_TERMS: usize = 6;

/// What post-game win expectancy reads of a game, the home side less the
/// away side: a constant (the home field), success rate (points of 100),
/// explosiveness (EPA a successful play), field position (the average
/// drive start, yards from its own goal), finishing drives (points a trip
/// inside the 40, a touchdown 7 and a field goal 3) and turnovers (the
/// away side's less the home side's).
pub fn pgwe_features(a: &[Adv; 2], b: &[TeamStats; 2]) -> [f64; PGWE_TERMS] {
    let f = |k: usize| five(&a[k]);
    let (h, w) = (f(0), f(1));
    [1.0, h[0] - w[0], h[1] - w[1], h[2] - w[2], h[3] - w[3], b[1].turnovers as f64 - b[0].turnovers as f64]
}

/// A side's first four factors: success rate (0-100), explosiveness (EPA a
/// successful play), field position (yards from its own goal) and points a
/// trip inside the 40.
pub fn five(a: &Adv) -> [f64; 4] {
    let plays = (a.runs + a.dropbacks) as f64;
    let succ = (a.run_success + a.pass_success) as f64;
    [
        if plays > 0.0 { 100.0 * succ / plays } else { 0.0 },
        if succ > 0.0 { a.success_epa as f64 / succ } else { 0.0 },
        if a.drives > 0 { a.start_sum as f64 / a.drives as f64 } else { 0.0 },
        if a.trips40 > 0 { (7.0 * a.trip_td as f64 + 3.0 * a.trip_fg as f64) / a.trips40 as f64 } else { 0.0 },
    ]
}

/// Post-game win expectancy's weights by level, from `decide_fit`.
pub const PGWE: [[f64; PGWE_TERMS]; 5] = [
    // High school.
    [0.0354, 0.3427, 5.9515, 0.1983, 0.8238, 0.3517],
    // FBS.
    [0.0031, 0.3827, 7.1211, 0.1862, 0.7156, 0.4742],
    // FCS.
    [0.0845, 0.3825, 7.0080, 0.2167, 0.7153, 0.6692],
    // Small colleges.
    [0.2241, 0.3907, 7.7099, 0.1943, 0.6556, 0.6081],
    // NFL.
    [0.0149, 0.3281, 6.5872, 0.1205, 0.6859, 0.7615],
];

/// The home side's post-game win expectancy (0-1); a neutral site has no
/// home field.
pub fn pgwe(level: Level, a: &[Adv; 2], b: &[TeamStats; 2], neutral: bool) -> f64 {
    let mut x = pgwe_features(a, b);
    if neutral {
        x[0] = 0.0;
    }
    let z: f64 = PGWE[row(level)].iter().zip(x).map(|(w, x)| w * x).sum();
    1.0 / (1.0 + (-z).exp())
}

/// Garbage time, as the public sites leave it out: in college and high
/// school Bill Connelly's margins (collegefootballdata.com's), never in
/// the 1st quarter, more than 38 points in the 2nd, 28 in the 3rd and 22
/// in the 4th; in the NFL the offense's win chance outside 10-90%
/// (rbsdm.com's default filter). Overtime never is.
pub fn garbage(level: Level, r: &PlayRecord) -> bool {
    if r.quarter > 4 {
        return false;
    }
    if level == Level::NFL {
        return r.wp_before < 0.10 || r.wp_before > 0.90;
    }
    let m = (r.score[0] - r.score[1]).abs();
    match r.quarter {
        2 => m > 38,
        3 => m > 28,
        4 => m > 22,
        _ => false,
    }
}

/// A snap's play concept, from the call's words: the run concept on a
/// handoff (an RPO's give too), the pass concept on a dropback (an RPO's
/// throw too). A designed play reads as the library concept it looks most
/// like, as the other side's film files it. None off a scrimmage snap.
pub fn concept_of(r: &PlayRecord) -> Option<&'static str> {
    let call = r.call.strip_suffix(" (audible)").unwrap_or(&r.call);
    match r.kind {
        PlayType::Run => {
            let run = call.split(" with ").next().unwrap_or(call);
            let run = run.strip_suffix(" left").or_else(|| run.strip_suffix(" right")).unwrap_or(run);
            RUN_CONCEPTS
                .iter()
                .map(|c| c.name())
                .filter(|n| run.ends_with(n))
                .max_by_key(|n| n.len())
                .or(Some("other run"))
        }
        PlayType::Pass | PlayType::Sack | PlayType::Scramble => {
            let pass = call.rsplit(" with ").next().unwrap_or(call);
            let lib = PASS_CONCEPTS.iter().map(|c| c.name).filter(|n| pass.ends_with(n)).max_by_key(|n| n.len());
            lib.or_else(|| {
                let i = crate::game::playbook::designed_named(pass)?;
                Some(PASS_CONCEPTS[crate::game::playbook::film_concept(i)].name)
            })
            .or(Some("other pass"))
        }
        _ => None,
    }
}

/// The coverage the defense played on a snap (the call's first words).
pub fn coverage_of(r: &PlayRecord) -> Option<&str> {
    let c = r.defense_call.split(", ").next()?;
    (!c.is_empty()).then_some(c)
}

/// What a 4th down was weighed at: the offense's win chance after going,
/// punting and kicking (None where it is no choice: a punt inside the 30,
/// a kick out of range), and the chance to convert.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Weigh {
    pub go: f64,
    pub punt: Option<f64>,
    pub kick: Option<f64>,
    pub conv: f64,
}

impl Weigh {
    /// The best choice (0 go, 1 punt, 2 kick) and its win chance.
    pub fn best(&self) -> (u8, f64) {
        let mut b = (0u8, self.go);
        for (k, v) in [(1u8, self.punt), (2u8, self.kick)] {
            if let Some(v) = v
                && v > b.1
            {
                b = (k, v);
            }
        }
        b
    }

    /// The win chance of a choice (0 go, 1 punt, 2 kick).
    pub fn of(&self, did: u8) -> Option<f64> {
        match did {
            0 => Some(self.go),
            1 => self.punt,
            _ => self.kick,
        }
    }
}

/// Seconds each choice takes off the clock (a snap and its whistle).
const PLAY_SECS: f64 = 6.0;

/// The analysts' model for one game: the level's win-chance numbers and
/// what the home side was expected to win by.
#[derive(Clone, Copy, Debug)]
pub struct Model {
    pub p: Params,
    pub level: Level,
    /// The home side's expected margin (the closing line turned round; the
    /// home field without one; 0 at a neutral site).
    pub spread: f64,
}

impl Model {
    /// A game of `level` in `year`; `spread` is the home side's expected
    /// margin from the books, None where none hang a line (high school).
    pub fn new(level: Level, year: i32, spread: Option<f64>, neutral: bool) -> Model {
        let p = Params::for_year(level, year);
        match spread {
            Some(s) => Model { p, level, spread: s },
            None => {
                let home = if neutral {
                    0.0
                } else if level == Level::NFL {
                    1.7
                } else {
                    2.5
                };
                Model { p: p.without_line(), level, spread: home }
            }
        }
    }

    fn r(&self) -> usize {
        row(self.level)
    }

    /// The game at a record, from its offense's side.
    pub fn state(&self, r: &PlayRecord) -> State {
        let to = r.timeouts.unwrap_or([2, 2]);
        let o = r.offense.min(1);
        let sign = if o == 0 { 1.0 } else { -1.0 };
        State {
            quarter: r.quarter,
            clock: r.clock.max(0.0),
            lead: sign * (r.score[0] - r.score[1]) as f64,
            down: r.down.max(1),
            togo: if r.togo > 0.0 { r.togo } else { 10.0 },
            yl: r.yardline_100,
            timeouts: [to[o], to[1 - o]],
            spread: sign * self.spread,
            answering: false,
        }
    }

    /// College and high school overtime (possessions from a spot): left
    /// unweighed.
    pub fn untimed(&self, r: &PlayRecord) -> bool {
        r.quarter > 4 && self.p.ot_spot.is_some()
    }

    /// The side with the ball's chance after it gives the ball up with
    /// `lead` (its own) and the other side starts at `yl` (yards to the
    /// goal it attacks).
    fn hand_over(&self, s: &State, lead: f64, yl: f64) -> f64 {
        let o = State {
            lead: -lead,
            spread: -s.spread,
            timeouts: [s.timeouts[1], s.timeouts[0]],
            down: 1,
            togo: 10.0_f64.min(yl),
            yl: yl.clamp(1.0, 99.0),
            answering: false,
            ..ticked(s)
        };
        1.0 - chance(&self.p, &o)
    }

    /// After a touchdown worth `lead` (its try still to come): the try at
    /// the level's make, then the kickoff.
    fn after_td(&self, s: &State, lead: f64) -> f64 {
        let xp = XP[self.r()];
        xp * self.hand_over(s, lead + 1.0, self.p.start) + (1.0 - xp) * self.hand_over(s, lead, self.p.start)
    }

    /// A kick from `yl` yards to the goal: made, by distance (nothing past
    /// the longest kick a coach tries).
    pub fn fg_make(&self, yl: f64) -> f64 {
        let d = yl + 17.0;
        if d > self.p.fg_max {
            return 0.0;
        }
        // Straight lines between the bands' middles.
        let mids = [25.0, 32.0, 37.0, 42.0, 47.0, 53.0];
        let f = &FG[self.r()];
        if d <= mids[0] {
            return f[0];
        }
        for k in 0..FG_BANDS - 1 {
            if d <= mids[k + 1] {
                let s = (d - mids[k]) / (mids[k + 1] - mids[k]);
                return f[k] + (f[k + 1] - f[k]) * s;
            }
        }
        // Past the last middle the make falls away to nothing at the
        // longest kick.
        let s = ((d - mids[FG_BANDS - 1]) / (self.p.fg_max - mids[FG_BANDS - 1]).max(1.0)).clamp(0.0, 1.0);
        f[FG_BANDS - 1] * (1.0 - s * s)
    }

    /// A 4th down weighed from the game at its snap (None in college and
    /// high school overtime).
    pub fn fourth(&self, r: &PlayRecord) -> Option<Weigh> {
        if self.untimed(r) {
            return None;
        }
        Some(self.fourth_at(&self.state(r)))
    }

    /// A 4th down weighed from a state (the offense's side).
    pub fn fourth_at(&self, s: &State) -> Weigh {
        let (yl, togo) = (s.yl, s.togo.max(0.5));
        let b = togo_band(togo);
        let goal = togo >= yl - 0.01;
        let conv = if goal { CONV_GOAL[self.r()][b] } else { CONV[self.r()][b] };
        let made = if goal || GAIN[self.r()][b].max(togo) >= yl {
            self.after_td(s, s.lead + 6.0)
        } else {
            let ny = yl - GAIN[self.r()][b].max(togo);
            let n = State { down: 1, togo: 10.0_f64.min(ny), yl: ny, answering: false, ..ticked(s) };
            chance(&self.p, &n)
        };
        let stopped = self.hand_over(s, s.lead, 100.0 - yl);
        let go = conv * made + (1.0 - conv) * stopped;
        let punt = (yl >= 30.0).then(|| self.hand_over(s, s.lead, PUNT[self.r()][punt_band(yl)]));
        let make = self.fg_make(yl);
        let kick = (make > 0.0).then(|| {
            let miss_at = (100.0 - (yl + 7.0)).min(80.0);
            make * self.hand_over(s, s.lead + 3.0, self.p.start) + (1.0 - make) * self.hand_over(s, s.lead, miss_at)
        });
        Weigh { go, punt, kick, conv }
    }

    /// A try after a touchdown, weighed: the scoring side's win chance
    /// kicking and going for two (None in college and high school
    /// overtime). The record's score counts the touchdown.
    pub fn try_after(&self, r: &PlayRecord) -> Option<[f64; 2]> {
        if self.untimed(r) {
            return None;
        }
        let s = State { clock: r.clock.max(0.0) + PLAY_SECS, ..self.state(r) };
        let (xp, two) = (XP[self.r()], TWO[self.r()]);
        let kick =
            xp * self.hand_over(&s, s.lead + 1.0, self.p.start) + (1.0 - xp) * self.hand_over(&s, s.lead, self.p.start);
        let go = two * self.hand_over(&s, s.lead + 2.0, self.p.start)
            + (1.0 - two) * self.hand_over(&s, s.lead, self.p.start);
        Some([kick, go])
    }
}

/// The game a snap later.
fn ticked(s: &State) -> State {
    State { clock: (s.clock - PLAY_SECS).max(0.0), ..*s }
}
