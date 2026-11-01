//! Game day control (section 9, `docs/design/game-day.md`): a game you
//! play snap by snap, the officials' real mistakes with replay challenges
//! and booth reviews, and the coin toss as a real choice.
//!
//! A live game is the one-loop game (`flow::play_game`) with a `Control`:
//! at each point where you may decide (a snap, a two-point try, an onside
//! kick, the toss, a challenge, halftime) the game asks it. Your answer is
//! kept with the number of the point it answered, so the whole game is its
//! seed plus your answers (a few kilobytes): every step plays the game
//! again from the kickoff with your answers and stops at the next point
//! that waits for you. A game where you hand every call to your staff
//! plays exactly as `play_game` does (`a_delegated_live_game_matches_the_one_loop_game`).

use super::calling::FourthDown;
use super::flow::{GameResult, GameSetup};
use super::garbage;
use super::playbook::{DefCall, OffCall, Personnel};
use super::roster::GameTeam;
use super::rules::Rules;
use super::sim::Conditions;
use crate::player::Level;
use crate::rng::{Rng, clamp};

// ------------------------------------------------------------ settings

/// Which snaps stop for you (game-day.md question 4, Cain: a choice).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum StopMode {
    /// The situations you pick stop; your staff calls the rest.
    #[default]
    Situations,
    /// Every snap stops: you call the game and learn.
    EverySnap,
    /// Nothing stops: the staff plays the rest of the game.
    Off,
}

/// The situations that stop for you in `StopMode::Situations`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Situations {
    pub third_down: bool,
    pub fourth_down: bool,
    pub red_zone: bool,
    pub goal_line: bool,
    pub two_minute: bool,
    /// Two-point tries, onside kicks and challenges.
    pub decisions: bool,
}

impl Default for Situations {
    fn default() -> Situations {
        Situations {
            third_down: true,
            fourth_down: true,
            red_zone: true,
            goal_line: true,
            two_minute: true,
            decisions: true,
        }
    }
}

/// The time you get for a call (question 5, Cain: a choice). Realism: 25
/// seconds, the time a real play caller has between the 40-second play
/// clock starting and the helmet radio cutting out at 15; when it runs out
/// the play caller's call goes in. Learning: the game waits. The clock runs
/// on the screen; the engine only ever sees your answer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CallClock {
    Realism,
    #[default]
    Learning,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    pub stop: StopMode,
    pub situations: Situations,
    pub clock: CallClock,
}

/// Your offense's tempo between plays (topic 5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Tempo {
    /// The play caller's own tempo.
    #[default]
    Normal,
    /// No huddle, to the line fast: less time for the defense to
    /// substitute and disguise, more wear on both sides.
    Hurry,
    /// Huddle and snap with the play clock nearly out.
    Milk,
}

/// The toss winner's choice (topic 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TossChoice {
    Receive,
    /// Defer: the winner takes the choice in the second half (allowed in
    /// the NFL since 2008, in college and high school always).
    Defer,
    Kick,
    /// Pick the end: the wind at your back in the first and third quarters.
    Goal,
    /// Overtime in college and high school: play defense first.
    DefenseFirst,
    OffenseFirst,
}

/// A change to the matchup plan between plays or at halftime (topic 5, a
/// word to a coordinator; topic 9). Player indexes are into the other
/// team's players, as in `calling::Plan`.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PlanChange {
    /// Attack this corner (Some(None) stops attacking anyone).
    pub target_cb: Option<Option<usize>>,
    /// Bracket this receiver (Some(None) stops the bracket).
    pub bracket: Option<Option<usize>>,
    pub spy: Option<bool>,
    /// Kick away from their returner: short of the goal line, or squib.
    pub kick_away: Option<bool>,
    /// Draws and screens against an edge who rushes upfield.
    #[serde(default)]
    pub draws: Option<bool>,
}

/// Your 4th-down chart (topic 2, your game rules): the staff's own
/// judgment, or a rule from the win-probability numbers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FourthChart {
    #[default]
    Staff,
    /// Go whenever going is worth the most.
    Numbers,
    /// Go when going is within two points of win chance of the best kick.
    Aggressive,
    /// Kick unless going is clearly better (four points of win chance).
    Conservative,
}

/// Your two-point chart (topic 2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TwoChart {
    #[default]
    Staff,
    /// The classic chart in the fourth quarter (Dick Vermeil's): go for two
    /// up 1, 4, 5, 12, 15 or 19 and down 2, 5, 9, 10, 13, 17 or 18 after
    /// the touchdown.
    Chart,
    /// Always kick unless the rules force a try.
    Kick,
}

/// Your timeout rule (update 2, G4): how many to keep for the end of each
/// half. Your staff spends the rest to beat the play clock instead of
/// taking a delay of game.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TimeoutRule {
    #[default]
    Staff,
    KeepTwo,
    KeepOne,
    Free,
}

/// Your four-minute plan (update 2, G4): ahead late, run it out (runs and
/// the play clock), keep attacking (your usual mix and tempo), or the
/// staff's call.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FourMinute {
    #[default]
    Staff,
    RunItOut,
    KeepAttacking,
}

/// The 4th-down chart's rows (to go) and columns (field position), for
/// your own cells (update 2, G4).
pub const CHART_ROWS: [&str; 7] =
    ["4th & 1", "4th & 2", "4th & 3", "4th & 4-5", "4th & 6-7", "4th & 8-10", "4th & 11+"];
pub const CHART_COLS: [&str; 9] =
    ["Own 1-20", "Own 21-30", "Own 31-40", "Own 41-50", "Opp 49-40", "Opp 39-30", "Opp 29-20", "Opp 19-10", "Opp 9-1"];

/// A snap's cell on the chart: (row, column).
pub fn chart_at(togo: f64, yardline_100: f64) -> (usize, usize) {
    let row = match togo {
        t if t <= 1.0 => 0,
        t if t <= 2.0 => 1,
        t if t <= 3.0 => 2,
        t if t <= 5.0 => 3,
        t if t <= 7.0 => 4,
        t if t <= 10.0 => 5,
        _ => 6,
    };
    let y = yardline_100;
    let col = if y >= 80.0 {
        0
    } else if y >= 70.0 {
        1
    } else if y >= 60.0 {
        2
    } else if y >= 50.0 {
        3
    } else if y >= 40.0 {
        4
    } else if y >= 30.0 {
        5
    } else if y >= 20.0 {
        6
    } else if y >= 10.0 {
        7
    } else {
        8
    };
    (row, col)
}

/// Your own call in a cell, if you set one: two bits a column, 1 go, 2
/// field goal, 3 punt.
pub fn chart_cell(cells: &[u32; 7], togo: f64, yardline_100: f64) -> Option<super::calling::FourthDown> {
    let (r, c) = chart_at(togo, yardline_100);
    match (cells[r] >> (2 * c)) & 3 {
        1 => Some(super::calling::FourthDown::Go),
        2 => Some(super::calling::FourthDown::FieldGoal),
        3 => Some(super::calling::FourthDown::Punt),
        _ => None,
    }
}

/// The classic two-point chart (`TwoChart::Chart`), by the margin after
/// the touchdown.
pub fn chart_says_two(diff_after_td: i32) -> bool {
    matches!(diff_after_td, 1 | 4 | 5 | 12 | 15 | 19 | -2 | -5 | -9 | -10 | -13 | -17 | -18)
}

/// Your week's work brought to the game (topics 1 and 2): the matchup plan,
/// the opponent film you studied with your own eyes, your charts and your
/// offense's tempo. Applied at kickoff; empty, it changes nothing.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Prep {
    pub plan: PlanChange,
    /// The opponent's film you studied: your staff reads its habits as
    /// they really were, on top of its own breakdown.
    pub study: Option<super::calling::Film>,
    pub fourth: FourthChart,
    pub two: TwoChart,
    pub tempo: Tempo,
    /// Your standing rules beyond the charts (update 2, G4): timeouts, the
    /// four-minute plan and your own cells on the 4th-down chart.
    #[serde(default)]
    pub timeouts: TimeoutRule,
    #[serde(default)]
    pub four: FourMinute,
    #[serde(default)]
    pub cells: [u32; 7],
    /// Your pregame talk (dialogue D4); None: your own coach profile's.
    #[serde(default)]
    pub talk: Option<super::talk::Pick>,
}

impl Prep {
    pub fn is_empty(&self) -> bool {
        self.talk.is_none()
            && self.plan == PlanChange::default()
            && self.study.is_none()
            && self.fourth == FourthChart::Staff
            && self.two == TwoChart::Staff
            && self.tempo == Tempo::Normal
            && self.timeouts == TimeoutRule::Staff
            && self.four == FourMinute::Staff
            && self.cells == [0; 7]
    }
}

/// Your answer at one point.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Input {
    /// The staff's choice stands (accept the call).
    Staff,
    /// Take a timeout before this snap; the snap then asks again.
    Timeout,
    /// Change the plan; the snap then asks again.
    Plan(PlanChange),
    /// Change your tempo; the snap then asks again.
    Tempo(Tempo),
    /// Your call on a snap: on 4th down punt, kick or go; the play.
    Call {
        fourth: Option<FourthDown>,
        offense: Option<OffCall>,
        defense: Option<DefCall>,
    },
    /// After your touchdown: go for two (with your call), or kick.
    Pat {
        two: bool,
        offense: Option<OffCall>,
    },
    Onside(bool),
    Toss(TossChoice),
    Challenge(bool),
    /// Halftime: back out for the second half, with any change to the plan.
    Halftime(PlanChange),
    /// Your halftime talk (dialogue D5); the halftime stop then asks again.
    Talk(super::talk::Pick),
    /// A word on the sideline to one of your players after his mistake
    /// (dialogue D6); the snap then asks again.
    Word {
        id: u32,
        pick: super::talk::Pick,
    },
}

/// The kinds of points where you may decide.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Ask {
    /// Your offense's snap: 4th-down choice and the call.
    Snap,
    /// Your defense's call.
    Defense,
    Pat,
    Onside,
    Toss,
    Challenge,
    Halftime,
}

/// What the game knows when it stops, for the screen: the scoreboard,
/// the ball and the clock.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct View {
    pub quarter: u8,
    pub clock: f64,
    pub overtime: bool,
    pub poss: usize,
    /// Yards from the offense's own goal line.
    pub x: f64,
    pub down: u8,
    pub togo: f64,
    pub ball_y: f64,
    pub score: [i32; 2],
    pub timeouts: [u8; 2],
    pub challenges: [u8; 2],
    pub running: bool,
    /// Home win probability (a smooth model, `win_prob`).
    pub wp_home: f64,
    pub plays: u32,
    /// Your tempo.
    pub tempo: Tempo,
    /// The wind at the back of the home team this quarter (mph, + helps
    /// the home team kick).
    pub wind_home: f64,
}

/// A decision waiting for you, with what your staff would do and why.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Pending {
    pub ask: Ask,
    pub view: View,
    /// The staff's choices.
    pub staff_fourth: Option<FourthDown>,
    pub staff_off: Option<OffCall>,
    pub staff_def: Option<DefCall>,
    pub staff_two: bool,
    pub staff_onside: bool,
    pub staff_toss: Option<TossChoice>,
    pub staff_challenge: bool,
    /// The play caller's next choices from the same box (send it back).
    pub off_options: Vec<OffCall>,
    pub def_options: Vec<DefCall>,
    /// The personnel the other offense sent on (when you are on defense).
    pub their_personnel: Option<Personnel>,
    /// Advice lines (the analytics staff, the replay assistant).
    pub advice: Vec<String>,
    /// For a challenge: what was ruled, and the play.
    pub ruling: Option<String>,
    /// For a 4th down: (go, punt, field goal) win probability, when the
    /// staff has an analyst.
    pub fourth_wp: Option<[f64; 3]>,
    /// The kicker's make chance from here.
    pub fg_make: Option<f64>,
    /// Your players waiting for a word after a costly mistake or a third
    /// small one (dialogue D6), with the word your staff would give.
    #[serde(default)]
    pub words: Vec<WordOffer>,
    /// At halftime: the talk you gave, once you gave it (dialogue D5).
    #[serde(default)]
    pub talked: Option<super::talk::Pick>,
}

/// A player waiting for a word (dialogue D6).
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct WordOffer {
    pub id: u32,
    pub mistake: super::talk::Mistake,
    /// The play it came on.
    pub play: u32,
    /// The word your staff would give him.
    pub staff: super::talk::Pick,
}

/// The game's link to you. Kept in the save with the game: your settings
/// and every answer, by the number of the point it answered.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Control {
    /// Your team in this game: 0 home, 1 away.
    pub team: usize,
    pub settings: Settings,
    /// (point number, answer), in order.
    pub inputs: Vec<(u32, Input)>,
    /// Points before this one never stop again (settings changed later
    /// don't move an answered game).
    pub frontier: u32,
    /// Stop at the next snap whatever the settings (the take-over key).
    pub take_next: bool,
    /// Keep the tracking of plays from this number on (the 2D view of the
    /// snap you just played).
    pub record_from: u32,
    /// Your week's work, applied at kickoff.
    #[serde(default)]
    pub prep: Prep,
    /// Set by a run.
    #[serde(skip)]
    pub asked: u32,
    #[serde(skip)]
    pub pending: Option<Pending>,
    #[serde(skip)]
    pub pause_at: Option<u32>,
}

/// Where a snap stands, for the stop rule.
pub struct StopCtx {
    pub down: u8,
    pub yardline_100: f64,
    pub half_left: f64,
    pub quarter: u8,
    pub goal_to_go: bool,
    /// Your team trails.
    pub trailing: bool,
    /// One of your players is waiting for a word (dialogue D6).
    pub word: bool,
}

impl Control {
    pub fn new(team: usize, settings: Settings) -> Control {
        Control { team, settings, ..Control::default() }
    }

    pub(crate) fn input_at(&self, a: u32) -> Option<&Input> {
        self.inputs.iter().rev().find(|x| x.0 == a).map(|x| &x.1)
    }

    /// Does this point stop for you (only at or past the frontier)?
    pub(crate) fn stops(&self, ask: Ask, c: &StopCtx) -> bool {
        let s = &self.settings;
        if s.stop == StopMode::Off {
            return false;
        }
        match ask {
            Ask::Toss | Ask::Halftime => true,
            // Two-point tries in the second half, onside kicks when you
            // trail late, and every challenge you could throw; all of them
            // on every-snap.
            Ask::Challenge => s.stop == StopMode::EverySnap || s.situations.decisions,
            Ask::Pat => s.stop == StopMode::EverySnap || (s.situations.decisions && c.quarter >= 3),
            Ask::Onside => s.stop == StopMode::EverySnap || (s.situations.decisions && c.trailing && c.quarter >= 4),
            Ask::Snap | Ask::Defense => {
                if s.stop == StopMode::EverySnap || self.take_next {
                    return true;
                }
                // A player waiting for a word stops it when you take the
                // decisions (dialogue D6).
                if c.word && s.situations.decisions {
                    return true;
                }
                let t = &s.situations;
                (t.third_down && c.down == 3)
                    || (t.fourth_down && c.down == 4)
                    || (t.red_zone && c.yardline_100 <= 20.0)
                    || (t.goal_line && c.goal_to_go)
                    || (t.two_minute && (c.quarter == 2 || c.quarter >= 4) && c.half_left <= 120.0)
            }
        }
    }

    /// Answer the point the game stopped at.
    pub fn answer(&mut self, input: Input) -> bool {
        let Some(a) = self.pause_at.take() else { return false };
        self.inputs.push((a, input));
        self.frontier = a + 1;
        self.take_next = false;
        self.pending = None;
        true
    }
}

/// Win probability for a team leading by `lead` points with `ep` expected
/// points on its current possession (negative when the other team has the
/// ball), `left` seconds of a `length`-second game left. The final margin
/// from here is about normal with a spread that shrinks with the square
/// root of the time left (Stern 1991: about 13.9 points over a whole NFL
/// game). Overtime and the last snap are handled by the spread's floor.
pub fn win_prob(lead: f64, ep: f64, left: f64, length: f64) -> f64 {
    let frac = clamp(left / length.max(1.0), 0.0, 1.0);
    let sd = 13.9 * frac.sqrt() + 0.6;
    let z = (lead + ep) / sd;
    clamp(crate::athlete::phi(z), 0.001, 0.999)
}

// ------------------------------------------------------------ officials

/// The officiating crew's eye (topic 6): how often it gets a close call
/// wrong against an average crew's 1 (0.7 sharp, 1.4 poor). Drawn on the officials' own stream, so a
/// game's other draws don't move.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Eye {
    pub miss: f64,
}

/// What a close call was about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CloseCall {
    /// A catch, often at the sideline (feet down, control through the ground).
    Catch,
    /// A fumble or a man down by contact first.
    Fumble,
    /// The ball across the goal line.
    GoalLine,
    /// The spot against the line to gain.
    Spot,
    /// An interception or a trapped ball.
    Interception,
}

impl CloseCall {
    pub fn name(self) -> &'static str {
        match self {
            CloseCall::Catch => "catch",
            CloseCall::Fumble => "fumble",
            CloseCall::GoalLine => "goal line",
            CloseCall::Spot => "spot",
            CloseCall::Interception => "interception",
        }
    }
}

/// Who looked at a close call again, and what came of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Reviewer {
    /// Nobody: the ruling stood unseen.
    None,
    /// The booth (NFL scoring plays, turnovers and the last two minutes;
    /// college any play).
    Booth,
    /// The NFL's replay assist (2023 on): fixed in the booth without a stop.
    Assist,
    /// A coach's challenge, by team.
    Challenge(usize),
}

/// The officiating on one play, kept on its record.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Officiating {
    pub call: CloseCall,
    /// The ruling on the field was wrong.
    pub wrong: bool,
    pub reviewer: Reviewer,
    /// The review changed the ruling.
    pub overturned: bool,
    /// The team the ruling on the field went against.
    pub against: usize,
    /// Plain words, for the play by play.
    pub note: String,
}

/// The replay rules of a level and era (topic 6): who may review what.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Replay {
    /// Challenges a coach starts the game with (NFL 2, college 1, high
    /// school 0).
    pub challenges: u8,
    /// A third challenge when the first two both win (NFL, 2004 on).
    pub third: bool,
    /// The booth reviews scoring plays (NFL 2011 on).
    pub booth_scores: bool,
    /// The booth reviews turnovers (NFL 2012 on).
    pub booth_turnovers: bool,
    /// The booth reviews any play (college, 2006 on).
    pub booth_all: bool,
    /// Replay assist (NFL 2023 on).
    pub assist: bool,
    /// Booth only inside the last two minutes of each half and overtime.
    pub booth_late: bool,
}

impl Replay {
    pub fn of(level: Level, year: i32) -> Replay {
        let none = Replay {
            challenges: 0,
            third: false,
            booth_scores: false,
            booth_turnovers: false,
            booth_all: false,
            assist: false,
            booth_late: false,
        };
        match level {
            // The NFL's replay returned in 1999 with the coach's challenge.
            Level::NFL if year >= 1999 => Replay {
                challenges: 2,
                third: year >= 2004,
                booth_scores: year >= 2011,
                booth_turnovers: year >= 2012,
                booth_all: false,
                assist: year >= 2023,
                booth_late: true,
            },
            Level::NFL => none,
            // NCAA instant replay from 2006 (the Big Ten tried it in 2004):
            // the booth may stop any play, and each coach has one challenge.
            Level::HS => none,
            _ if year >= 2006 => Replay {
                challenges: 1,
                third: false,
                booth_scores: true,
                booth_turnovers: true,
                booth_all: true,
                assist: false,
                booth_late: false,
            },
            _ => none,
        }
    }
}

/// How often a crew of average eye rules a close call wrong (proposed, not
/// confirmed). With the close-call chances in `flow.rs` an NFL game has
/// about 0.9 wrong rulings a review could change, 0.38 challenges a game
/// won 38% of the time (real 2024: 133 challenges in 272 games, 50 won,
/// ESPN) and 0.85 booth reviews, 56% reversed (Football Zebras 2020: 103
/// of 179); `match_check` prints them (AFM_REFS=1 by kind of call).
pub const WRONG_SHARE: f64 = 0.16;

/// The chance the replay shows enough to change a wrong ruling
/// ("indisputable visual evidence"; proposed, not confirmed).
pub const EVIDENCE: f64 = 0.78;

pub(crate) fn ref_seed(seed: u64) -> u32 {
    ((seed >> 13) as u32) ^ (seed as u32).rotate_left(19) ^ 0x5851_F42D
}

pub(crate) fn toss_seed(seed: u64) -> u32 {
    ((seed >> 7) as u32) ^ (seed as u32).rotate_left(23) ^ 0x2545_F491
}

pub(crate) fn draw_eye(rng: &mut Rng) -> Eye {
    Eye { miss: clamp(rng.normal(1.0, 0.15), 0.7, 1.4) }
}

/// The toss: the AI winner's habit. Since the NFL allowed deferring in
/// 2008 most winners defer (82%, the level's typical head coach; minor bug
/// 140: read against 10, the NFL's deferred 90%); college and high school
/// coaches defer a little less often (proposed, not confirmed).
pub(crate) fn ai_toss(level: Level, year: i32, gm: f64, rng: &mut Rng) -> TossChoice {
    let defer = match level {
        Level::NFL if year < 2008 => 0.0,
        Level::NFL => clamp(0.82 + 0.008 * (gm - super::rules::coach_norm(level)), 0.6, 0.97),
        Level::HS => 0.6,
        _ => 0.78,
    };
    if rng.chance(defer) { TossChoice::Defer } else { TossChoice::Receive }
}

/// Overtime toss habit: NFL winners receive; college and high school
/// winners take defense first, to know what they need (about nine in ten).
pub(crate) fn ai_ot_toss(level: Level, rng: &mut Rng) -> TossChoice {
    match level {
        Level::NFL => {
            if rng.chance(0.88) {
                TossChoice::Receive
            } else {
                TossChoice::Kick
            }
        }
        _ => {
            if rng.chance(0.9) {
                TossChoice::DefenseFirst
            } else {
                TossChoice::OffenseFirst
            }
        }
    }
}

// ------------------------------------------------------------ a live game

/// Everything a game needs apart from the teams: kept with a live game.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Params {
    pub rules: Rules,
    pub cond: Conditions,
    pub seed: u64,
    pub neutral: bool,
    pub playoff: bool,
    pub two_way_margin: [f64; 2],
    pub context: [garbage::Context; 2],
    pub wear: [f64; 2],
    pub cold_unused: [f64; 2],
    /// For the views only: the local kickoff hour and a roof over the
    /// field (later extras topic 2).
    #[serde(default)]
    pub kickoff: f32,
    #[serde(default)]
    pub indoor: bool,
}

impl Params {
    pub fn of(s: &GameSetup) -> Params {
        Params {
            rules: s.rules,
            cond: s.cond,
            seed: s.seed,
            neutral: s.neutral,
            playoff: s.playoff,
            two_way_margin: s.two_way_margin,
            context: s.context,
            wear: s.wear,
            cold_unused: s.cold_unused,
            kickoff: s.kickoff,
            indoor: s.indoor,
        }
    }
    pub fn setup<'a>(&self, home: &'a GameTeam, away: &'a GameTeam) -> GameSetup<'a> {
        let mut s = GameSetup::new(home, away, self.rules.level, self.seed);
        s.rules = self.rules;
        s.cond = self.cond;
        s.neutral = self.neutral;
        s.playoff = self.playoff;
        s.two_way_margin = self.two_way_margin;
        s.context = self.context;
        s.wear = self.wear;
        s.cold_unused = self.cold_unused;
        s.kickoff = self.kickoff;
        s.indoor = self.indoor;
        s
    }
}

/// A game you are playing: both teams as they took the field, the game's
/// settings and your answers. Plays again from the kickoff on each step.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct LiveGame {
    pub home: GameTeam,
    pub away: GameTeam,
    pub params: Params,
    pub ctl: Control,
}

/// Where a live game stands after a step.
pub struct Step {
    pub result: GameResult,
    /// The decision waiting for you (None: the game is over).
    pub pending: Option<Pending>,
    pub view: View,
}

impl LiveGame {
    pub fn new(home: GameTeam, away: GameTeam, params: Params, team: usize, settings: Settings) -> LiveGame {
        LiveGame { home, away, params, ctl: Control::new(team, settings) }
    }

    /// Play from the kickoff with every answer so far, to the next point
    /// that waits for you (or the end).
    pub fn run(&mut self) -> Step {
        self.run_keep(false)
    }

    /// A run; `keep` keeps the game's setup and your answers in the result
    /// for its film (the finished game).
    fn run_keep(&mut self, keep: bool) -> Step {
        let mut setup = self.params.setup(&self.home, &self.away);
        setup.keep = keep;
        let ctl = std::mem::take(&mut self.ctl);
        let (result, ctl, view) = super::flow::play_game_ctl(&setup, ctl);
        self.ctl = ctl;
        Step { result, pending: self.ctl.pending.clone(), view }
    }

    /// Answer the waiting decision and play on to the next one.
    pub fn answer(&mut self, input: Input) -> Step {
        let first = self.run();
        if first.pending.is_none() {
            return first;
        }
        self.ctl.answer(input);
        self.ctl.record_from = first.view.plays + 1;
        self.run()
    }

    pub fn over(&mut self) -> bool {
        self.run().pending.is_none()
    }

    /// Hand the rest of the game to the staff and play it out.
    pub fn finish(&mut self) -> GameResult {
        self.ctl.settings.stop = StopMode::Off;
        self.ctl.take_next = false;
        self.run_keep(true).result
    }
}
