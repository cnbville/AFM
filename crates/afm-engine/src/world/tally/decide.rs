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

// Fitted tables: a refit can land a value near a named constant.
#![allow(clippy::approx_constant)]

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
/// 2026-11-07 after minor bugs batch 9 and 2026-11-19 after engine gaps
/// lane EG5, the rule books and flags, over scheme-fit lane SF2, minor
/// bugs 409-412 and engine gaps lane EG2, ball security): 1,500 engine
/// games a level on synthetic leagues, garbage time and overtime left out.
pub const CONV: [[f64; TOGO_BANDS]; 5] = [
    // High school.
    [0.842, 0.707, 0.618, 0.517, 0.475, 0.405, 0.279, 0.166, 0.072],
    // FBS.
    [0.796, 0.682, 0.594, 0.552, 0.479, 0.432, 0.312, 0.207, 0.095],
    // FCS.
    [0.822, 0.681, 0.601, 0.545, 0.493, 0.428, 0.310, 0.205, 0.094],
    // Small colleges.
    [0.819, 0.691, 0.615, 0.543, 0.487, 0.443, 0.313, 0.213, 0.096],
    // NFL.
    [0.766, 0.656, 0.609, 0.574, 0.535, 0.459, 0.342, 0.241, 0.100],
];

/// The same with the goal line as the line to gain (goal to go): a
/// conversion is a touchdown.
pub const CONV_GOAL: [[f64; TOGO_BANDS]; 5] = [
    // High school.
    [0.722, 0.629, 0.490, 0.563, 0.365, 0.357, 0.308, 0.164, 0.098],
    // FBS.
    [0.641, 0.529, 0.448, 0.476, 0.455, 0.391, 0.332, 0.234, 0.100],
    // FCS.
    [0.701, 0.588, 0.533, 0.551, 0.461, 0.433, 0.300, 0.186, 0.095],
    // Small colleges.
    [0.652, 0.554, 0.475, 0.521, 0.404, 0.395, 0.297, 0.208, 0.067],
    // NFL.
    [0.635, 0.502, 0.500, 0.516, 0.466, 0.422, 0.332, 0.212, 0.062],
];

/// Yards a converted play gains from the line of scrimmage, by yards to
/// go (at least the distance itself).
pub const GAIN: [[f64; TOGO_BANDS]; 5] = [
    // High school.
    [5.9, 7.5, 9.0, 11.3, 11.6, 13.0, 15.1, 19.5, 27.0],
    // FBS.
    [5.5, 7.9, 9.3, 11.5, 12.2, 13.9, 16.7, 20.3, 28.1],
    // FCS.
    [5.8, 8.0, 9.3, 11.7, 12.1, 13.7, 16.5, 19.8, 26.5],
    // Small colleges.
    [5.8, 7.9, 9.3, 11.5, 11.9, 13.7, 16.3, 19.8, 26.3],
    // NFL.
    [5.6, 8.5, 9.6, 11.3, 11.8, 14.1, 17.1, 21.3, 26.9],
];

/// Where a punt leaves the other side: its yards to the goal it attacks at
/// its next snap, by the punt's spot (`punt_band`). This table, `FG` and
/// `XP` were refitted again on 2026-11-19 after engine gaps lane EG3 (long
/// kicks, blocked kicks and returns), over lane EG5, `decide_fit -- 1500 1`.
pub const PUNT: [[f64; PUNT_BANDS]; 5] = [
    // High school.
    [83.3, 84.2, 84.2, 82.0, 78.2, 73.1, 69.1, 64.3, 58.9, 53.7, 48.9, 44.1, 39.6, 33.1],
    // FBS.
    [87.4, 86.0, 86.2, 85.8, 83.8, 80.4, 76.5, 71.7, 67.5, 62.7, 57.1, 53.5, 48.0, 42.5],
    // FCS.
    [86.2, 85.5, 85.9, 85.1, 82.3, 78.1, 74.6, 69.8, 65.0, 60.3, 55.9, 50.1, 46.1, 41.2],
    // Small colleges.
    [85.6, 85.5, 85.0, 84.6, 81.0, 77.1, 72.8, 68.9, 63.3, 58.8, 53.5, 49.2, 44.2, 39.4],
    // NFL.
    [88.7, 88.7, 86.2, 87.0, 86.4, 84.6, 81.3, 77.0, 72.4, 67.3, 62.2, 58.3, 53.0, 49.7],
];

/// Field goals made, by distance (`fg_band`).
pub const FG: [[f64; FG_BANDS]; 5] = [
    // High school.
    [0.872, 0.746, 0.687, 0.529, 0.452, 0.452],
    // FBS.
    [0.932, 0.844, 0.745, 0.653, 0.565, 0.569],
    // FCS.
    [0.877, 0.774, 0.685, 0.588, 0.600, 0.500],
    // Small colleges.
    [0.864, 0.748, 0.676, 0.566, 0.480, 0.480],
    // NFL.
    [0.981, 0.954, 0.909, 0.834, 0.769, 0.663],
];

/// Extra points made and two-point tries converted.
pub const XP: [f64; 5] = [0.952, 0.966, 0.946, 0.927, 0.955];
pub const TWO: [f64; 5] = [0.443, 0.378, 0.416, 0.416, 0.469];

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
    [0.1822, 0.2812, 5.6801, 0.1847, 0.5734, 0.6409],
    // FBS.
    [0.1390, 0.4123, 7.6307, 0.1540, 0.6135, 0.6956],
    // FCS.
    [0.0703, 0.3829, 7.4952, 0.2065, 0.5214, 0.5955],
    // Small colleges.
    [-0.2198, 0.4138, 8.6701, 0.1684, 0.5568, 0.8445],
    // NFL.
    [0.0907, 0.3258, 6.9714, 0.0628, 0.4170, 0.8497],
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
