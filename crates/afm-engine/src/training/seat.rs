//! Your team's training (topics 2-9 and 13): your week templates and this
//! week's edits, each player's plan (focus, habit, body, position move,
//! extra work, rest), the practice log, grades from practice battles on
//! the engine, the injury report, your staff's notes and block reports,
//! the installs, the week's game plan and call sheet, your designed plays
//! and your packages. Everything here is your team's only (memory gate).

use super::plan::{Kind, Period, SLOTS, Slot, Template, Week, fit, neutral, styled, week_from};
use super::rules::{BLOCKS, Block, Book, Breach, Counts, Frame, Tier, block_limits, check_week, weekly_base};
use super::week::{Frames, block_of_week, books};
use super::work::{Role, absorb_factor, ratio, share, week_load, week_work};
use super::{FAMILIES, Family, cover, family_of, land};
use crate::attributes::Attr;
use crate::calendar::{Date, Level as Cal};
use crate::game::playbook::{Backfield, Personnel, PlayKind, RunConcept, Set};
use crate::game::sim::InjuryKind;
use crate::positions::Position;
use crate::rng::{Rng, clamp};
use crate::world::{Stage, World};
use std::collections::HashMap;

/// A player's plan beyond his focus (the focus stays in `seat_focus`).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Plan {
    pub id: u32,
    /// A habit coached toward a target: tendency index and target (1-20).
    pub habit: Option<(u8, u8)>,
    /// A weight target in pounds and the strength staff's emphasis (0
    /// strength, 1 speed, 2 lean).
    pub body: Option<(u16, u8)>,
    /// A position he is learning, and how far along he is (0-100).
    pub moving: Option<(u8, u8)>,
    /// Extra work on his own: 1 film, 2 extra throws or catches after
    /// practice, 4 extra lifting.
    pub extra: u8,
    /// Practice days a week he sits to rest (a veteran's day off).
    pub rest: u8,
}

/// Someone hurt in practice (or in a game, for the report).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Hurt {
    pub id: u32,
    /// None: off the field for something that isn't a football injury
    /// (the non-football injury list after a failed conditioning test,
    /// heat illness); `how` says what.
    pub kind: Option<InjuryKind>,
    pub from: Date,
    pub until: Date,
    /// What it came from: "practice" with the period, or "game".
    pub how: String,
}

/// A staff note (topic 13): who wrote it and about whom.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Note {
    pub date: Date,
    /// The player, or 0 for the team.
    pub id: u32,
    /// "position", "strength", "trainers", "coordinator".
    pub dept: String,
    pub text: String,
}

/// One week as it was practiced.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WeekLog {
    pub monday: Date,
    pub block: Option<Block>,
    pub week: Week,
    pub snaps: u32,
    pub injuries: u32,
    /// Changes the staff made to keep the week legal.
    pub fitted: Vec<String>,
    /// The best and worst practice graders this week: (id, grade).
    pub best: Vec<(u32, u8)>,
    pub worst: Vec<(u32, u8)>,
    /// One-on-one battles: (winner id, loser id, wins, losses).
    pub battles: Vec<(u32, u32, u8, u8)>,
}

/// A practice case: a week outside the rules that was found out (topic 3).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Case {
    pub date: Date,
    /// The week it was about, and what broke the rules.
    pub week: Date,
    pub what: Vec<String>,
    /// How it came out, and what it cost.
    pub how: String,
    pub penalty: String,
}

/// A player's time on his own (topic 9): what he really did, week by week,
/// and his weight when it began, for report day.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Away {
    pub id: u32,
    pub effort: f32,
    pub weeks: u8,
    pub start_wt: u16,
}

/// A department head's end-of-block report.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Report {
    pub date: Date,
    pub block: Block,
    pub lines: Vec<(String, String)>,
}

/// How far a part of the playbook is installed (plays and tactics topic 4).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum InstallKey {
    /// A concept family of the playbook (`playbook::Family`).
    Family(u8),
    /// One of your designed plays (index into `designs`).
    Design(u16),
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Install {
    pub key: InstallKey,
    /// 0-1: a third is knowing the job, all of it mastery.
    pub level: f32,
    pub reps: u32,
}

/// A play on the call sheet.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SheetPlay {
    pub personnel: Personnel,
    pub backfield: Backfield,
    pub set: Set,
    pub play: SheetKind,
    /// The hash it is called from (update 2, G10): 0 either, 1 left, 2 right.
    #[serde(default)]
    pub hash: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SheetKind {
    Run(RunConcept),
    /// A library pass concept by name index (`PASS_CONCEPTS`).
    Pass(u16),
    /// One of your designs.
    Design(u16),
}

/// The call sheet's situations (`game::sheet` has their keys; update 2,
/// G9 added the five after the two-minute drill).
pub const SHEET: [&str; 15] = [
    "Opening script",
    "First and 10",
    "Second and long",
    "Second and short",
    "Third and short",
    "Third and medium",
    "Third and long",
    "Red zone",
    "Goal line",
    "Two-minute",
    "Second and medium",
    "Backed up",
    "Four-minute",
    "Shot plays",
    "Two-point",
];

/// The week's game plan (calendar topic 6, plays and tactics topic 5):
/// the game it is for, the call sheet by situation, and the plan's
/// emphasis. Game day (`docs/design/game-day.md`) reads it.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GamePlan {
    pub game: Option<Date>,
    pub sheet: Vec<(u8, Vec<SheetPlay>)>,
    /// Run share leaning, -0.2 to 0.2 against the coordinator's own.
    pub lean: f32,
    /// How closely the coordinator sticks to the sheet, 0-1 (0.85 a normal staff).
    pub stick: f32,
}

/// A saved design: the play and whether it is in the playbook.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SavedDesign {
    pub design: crate::game::designer::Design,
    pub saved: Date,
    /// The coordinator's review when saved.
    pub review: Vec<String>,
    /// The practice read so far: snaps, yards a snap, completions.
    pub read: Option<crate::game::designer::PracticeRead>,
    /// In the install this week.
    pub installing: bool,
}

/// The boards of the depth chart (packages).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Board {
    Nickel,
    Dime,
    GoalLineD,
    GoalLineO,
    KickReturn,
    KickCover,
    PuntReturn,
    PuntCover,
    FieldGoal,
    /// Three tight ends (13 personnel) outside the goal line (update 2,
    /// G15).
    Jumbo,
}

impl Board {
    pub const ALL: [Board; 10] = [
        Board::Nickel,
        Board::Dime,
        Board::GoalLineD,
        Board::GoalLineO,
        Board::KickReturn,
        Board::KickCover,
        Board::PuntReturn,
        Board::PuntCover,
        Board::FieldGoal,
        Board::Jumbo,
    ];
    pub fn name(self) -> &'static str {
        [
            "nickel",
            "dime",
            "goal_line_d",
            "goal_line_o",
            "kick_return",
            "kick_cover",
            "punt_return",
            "punt_cover",
            "field_goal",
            "jumbo",
        ][self as usize]
    }
    pub fn label(self) -> &'static str {
        [
            "Nickel",
            "Dime",
            "Goal line defense",
            "Goal line offense",
            "Kickoff return",
            "Kickoff coverage",
            "Punt return",
            "Punt coverage",
            "Field goal and extra point",
            "Jumbo",
        ][self as usize]
    }
    pub fn parse(s: &str) -> Option<Board> {
        Board::ALL.iter().copied().find(|b| b.name() == s)
    }
    /// The positions that fill it, by slot: (label, position).
    pub fn slots(self) -> &'static [(&'static str, Position)] {
        use Position::*;
        match self {
            Board::Nickel => &[
                ("LE", EDGE),
                ("DT", DT),
                ("DT", DT),
                ("RE", EDGE),
                ("MIKE", LB),
                ("WILL", LB),
                ("CB", CB),
                ("NICKEL", CB),
                ("CB", CB),
                ("FS", S),
                ("SS", S),
            ],
            Board::Dime => &[
                ("LE", EDGE),
                ("DT", DT),
                ("DT", DT),
                ("RE", EDGE),
                ("MIKE", LB),
                ("CB", CB),
                ("NICKEL", CB),
                ("DIME", CB),
                ("CB", CB),
                ("FS", S),
                ("SS", S),
            ],
            Board::GoalLineD => &[
                ("LE", EDGE),
                ("DT", DT),
                ("NT", DT),
                ("DT", DT),
                ("RE", EDGE),
                ("SAM", LB),
                ("MIKE", LB),
                ("WILL", LB),
                ("CB", CB),
                ("CB", CB),
                ("SS", S),
            ],
            Board::Jumbo => &[
                ("QB", QB),
                ("LT", OT),
                ("LG", IOL),
                ("C", IOL),
                ("RG", IOL),
                ("RT", OT),
                ("TE", TE),
                ("TE", TE),
                ("TE", TE),
                ("RB", RB),
            ],
            Board::GoalLineO => &[
                ("QB", QB),
                ("LT", OT),
                ("LG", IOL),
                ("C", IOL),
                ("RG", IOL),
                ("RT", OT),
                ("TE", TE),
                ("TE", TE),
                ("TE", TE),
                ("FB", RB),
                ("RB", RB),
            ],
            Board::KickReturn => &[
                ("KR", WR),
                ("UP", RB),
                ("WEDGE", TE),
                ("WEDGE", LB),
                ("WEDGE", TE),
                ("FRONT", LB),
                ("FRONT", LB),
                ("FRONT", S),
                ("FRONT", LB),
                ("FRONT", RB),
                ("FRONT", S),
            ],
            Board::KickCover => &[
                ("K", K),
                ("L5", LB),
                ("L4", S),
                ("L3", CB),
                ("L2", WR),
                ("L1", LB),
                ("R1", LB),
                ("R2", S),
                ("R3", CB),
                ("R4", WR),
                ("R5", LB),
            ],
            Board::PuntReturn => &[
                ("PR", WR),
                ("JAM", CB),
                ("JAM", CB),
                ("RUSH", EDGE),
                ("RUSH", LB),
                ("RUSH", DT),
                ("RUSH", DT),
                ("RUSH", LB),
                ("RUSH", EDGE),
                ("HOLD", LB),
                ("HOLD", S),
            ],
            Board::PuntCover => &[
                ("P", P),
                ("LS", LS),
                ("GUNNER", CB),
                ("GUNNER", WR),
                ("PP", S),
                ("WING", TE),
                ("WING", LB),
                ("GUARD", LB),
                ("GUARD", LB),
                ("TACKLE", TE),
                ("TACKLE", LB),
            ],
            Board::FieldGoal => &[
                ("K", K),
                ("H", P),
                ("LS", LS),
                ("LT", OT),
                ("LG", IOL),
                ("RG", IOL),
                ("RT", OT),
                ("TE", TE),
                ("TE", TE),
                ("WING", TE),
                ("WING", LB),
            ],
        }
    }
}

/// Your team's training store.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SeatTraining {
    /// Your templates by slot; a slot you never set runs your staff's
    /// default (the AI's, by your style).
    pub templates: Vec<(Slot, Template)>,
    /// This week's own edits: the week's Monday and its days.
    pub this_week: Option<(Date, Week)>,
    pub plans: Vec<Plan>,
    pub hurt: Vec<Hurt>,
    pub notes: Vec<Note>,
    pub log: Vec<WeekLog>,
    pub reports: Vec<Report>,
    /// The block's counts so far: the block, its season and the counts.
    pub counts: Option<(Block, i32, Counts)>,
    /// Practice grades this block: id, grade sum, reps.
    pub grades: Vec<(u32, f32, u16)>,
    pub installs: Vec<Install>,
    pub plan: GamePlan,
    pub designs: Vec<SavedDesign>,
    /// Concept numbers of the designs in this session (registered on load).
    #[serde(skip)]
    pub design_ids: Vec<usize>,
    pub packages: Vec<(Board, Vec<u32>)>,
    /// This block's growth read by your staff, per player: overall points
    /// (the staff see it through their eye; never shown as a number).
    #[serde(skip)]
    pub moved: Vec<(u32, f32)>,
    /// The NFL clubs' practice risk this week (all clubs).
    #[serde(default)]
    pub club_risk: Vec<f32>,
    /// Players whose position you set: the staff leave them there.
    #[serde(default)]
    pub yours_moved: Vec<u32>,
    /// This week's plan pushed past the rules on purpose (topic 3: real
    /// teams do it; it is found out and punished as in real life).
    #[serde(default)]
    pub pushed: Option<Date>,
    #[serde(default)]
    pub cases: Vec<Case>,
    /// Practices lost to a case: the block, its season and how many.
    #[serde(default)]
    pub lost: Vec<(Block, i32, u16)>,
    /// Your players' time on their own since it began (topic 9).
    #[serde(default)]
    pub away: Vec<Away>,
    /// When each of your players arrived (TR8: new arrivals).
    #[serde(default)]
    pub arrived: Vec<(u32, Date)>,
    /// Routes on air beyond an average staff's (TR20): the quarterback's
    /// timing with his receivers, in chemistry (`chemistry::apply`).
    #[serde(default)]
    pub air: f32,
    /// Outings and the leadership council beyond an average staff's since
    /// camp (TR32, TR33): friendships form a little more or less easily.
    #[serde(default)]
    pub bonds: f32,
    /// The strength staff's testing days, the last three (TR5).
    #[serde(default)]
    pub tests: Vec<TestDay>,
    /// Your NFL club's practice risk from your own week, set by your week
    /// and read by the clubs' practice injuries (0 when there's none).
    #[serde(skip)]
    pub my_risk: f32,
}

/// A testing day's results (TR5).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TestDay {
    pub date: Date,
    pub rows: Vec<TestRow>,
    /// Your staff's baseline in its first week, not a testing day.
    #[serde(default)]
    pub baseline: bool,
}

/// One player's numbers: bench, squat and clean in pounds, his reps at 225
/// (the NFL's bench), the 40 and its 10-yard split and the short shuttle in
/// hundredths of a second, the vertical in half inches, his weight.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TestRow {
    pub id: u32,
    pub bench: u16,
    pub reps: u8,
    pub squat: u16,
    pub clean: u16,
    pub forty: u16,
    pub split: u16,
    pub shuttle: u16,
    pub vertical: u8,
    pub weight: u16,
}

/// A player's testing numbers today: the combine's expected results for his
/// ratings and body with a test day's noise (keyed), the lifts from his
/// bench max (his strength at 220 lb scaled by his weight to the two-thirds
/// power; squat about 1.45 times it, the power clean about 0.88; proposed,
/// not confirmed: strength staffs' usual ratios).
pub fn test_row(p: &crate::player::Player, key: u64) -> TestRow {
    let c = crate::body::Drills::of(p).expected();
    let wt = crate::body::body_now(p).weight;
    let n = |k: u64, sd: f64| crate::world::season::key_normal(key ^ k, p.id as u64) * sd;
    let reps = (c.bench + n(1, 1.5)).round().clamp(0.0, 50.0);
    // His max: the combine's reps are fitted at draft weights, so read his
    // strength at 220 lb and scale it by his weight to the two-thirds power
    // (the way strength goes with size; a 160 lb receiver doesn't press
    // what the fit gives a 200 lb one).
    let at_220 = crate::body::Drills { weight: 220.0, ..crate::body::Drills::of(p) }.expected().bench + n(1, 1.5);
    let strength = p.attributes.get(Attr::Strength);
    let one = if at_220 >= 1.0 {
        225.0 * (1.0 + at_220 / 30.0) * (wt.max(100.0) / 220.0).powf(0.67)
    } else {
        (wt * clamp(0.55 + 0.011 * (strength - 25.0), 0.45, 1.6)).min(220.0)
    };
    let five = |x: f64| ((x / 5.0).round() * 5.0).clamp(0.0, 900.0) as u16;
    let bench = one * (1.0 + n(2, 0.02));
    let forty = (c.forty + n(5, 0.03)).clamp(4.2, 6.5);
    TestRow {
        id: p.id,
        bench: five(bench),
        reps: reps as u8,
        squat: five(bench * 1.45 * (1.0 + n(3, 0.04))),
        clean: five(bench * 0.88 * (1.0 + n(4, 0.04))),
        forty: (forty * 100.0).round() as u16,
        split: ((0.375 * forty - 0.15 + n(6, 0.015)) * 100.0).round() as u16,
        shuttle: ((c.shuttle + n(7, 0.04)).clamp(3.8, 6.0) * 100.0).round() as u16,
        vertical: ((c.vertical + n(8, 0.8)).clamp(14.0, 46.0) * 2.0).round() as u8,
        weight: wt.round().clamp(0.0, 500.0) as u16,
    }
}

/// A practice day's weather and where the team went (TR15).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DayWeather {
    /// Degrees Fahrenheit at practice time.
    pub temp: i16,
    /// "outside", "inside" (moved to the indoor field), "delayed" (a
    /// lightning hold took half an hour) or "lost" (the field work lost).
    pub place: &'static str,
    /// What moved it: "rain", "snow", "cold", "wind", "lightning",
    /// "severe weather", or "" for a normal day.
    pub why: &'static str,
}

/// Whether your team has an indoor field (proposed, not confirmed: every
/// NFL club; FBS programs past a modest facilities grade, FCS programs
/// with good ones, the best-funded high schools).
pub fn indoor_field(w: &World, st: &SeatTeam) -> bool {
    let Some(seat) = w.seat else { return false };
    match st.tier {
        Tier::Pro => true,
        Tier::College => seat.program.and_then(|p| w.programs.get(p)).is_some_and(|p| {
            p.facilities
                > match p.division {
                    crate::world::data::Division::FBS => 0.35,
                    crate::world::data::Division::FCS => 0.7,
                    _ => 0.9,
                }
        }),
        Tier::High => seat.program.and_then(|h| w.high_schools.get(h)).is_some_and(|h| h.resources > 0.85),
    }
}

/// Your team's town for its practice weather: state index, code and town.
fn practice_town(w: &World, st: &SeatTeam) -> Option<(usize, String, String)> {
    let seat = w.seat?;
    match st.tier {
        Tier::Pro => {
            let (town, code) = crate::world::finance::nfl::city(w, seat.nfl_team?, w.year);
            let code = if code.is_empty() { "NY".to_string() } else { code };
            Some((crate::world::geo::state_index(&code)?, code, town))
        }
        Tier::College => {
            let p = w.programs.get(seat.program?)?;
            Some((p.state, w.states[p.state].code.clone(), p.city.clone()))
        }
        Tier::High => {
            let h = w.high_schools.get(seat.program?)?;
            Some((h.state, w.states[h.state].code.clone(), h.town.clone()))
        }
    }
}

/// The week's practice weather for your team (TR15; proposed, not
/// confirmed): severe weather and lightning send a team with an indoor
/// field inside and cost one without it the day's field work or half an
/// hour; snow, bitter cold and high wind send it inside when it can; rain
/// does about half the time. Days with no field work have none.
pub fn practice_weather(w: &World, st: &SeatTeam, monday: Date, wk: &Week) -> [Option<DayWeather>; 7] {
    let Some((state, code, town)) = practice_town(w, st) else { return [None; 7] };
    let inside = indoor_field(w, st);
    let seed = w.options.seed as u64;
    std::array::from_fn(|k| {
        let day = &wk.days[k];
        if day.game || !day.periods.iter().any(|p| p.kind.on_field() && !p.own()) {
            return None;
        }
        let d = monday.add(k as i32);
        let hour = match st.tier {
            Tier::Pro => 11,
            _ if st.frame.block(d) == Block::Camp => 9,
            _ => 15,
        };
        let wx = crate::calendar::weather::weather_in(seed, state, &code, &town, d, hour);
        let temp = wx.temperature.round() as i16;
        let (place, why) = if wx.severe.is_some() {
            (if inside { "inside" } else { "lost" }, "severe weather")
        } else if wx.lightning {
            (if inside { "inside" } else { "delayed" }, "lightning")
        } else if wx.snow {
            (if inside { "inside" } else { "outside" }, "snow")
        } else if wx.temperature < 20.0 {
            (if inside { "inside" } else { "outside" }, "cold")
        } else if wx.wind > 30.0 {
            (if inside { "inside" } else { "outside" }, "wind")
        } else if wx.rain {
            let moved = inside && crate::world::season::key_uniform(seed ^ 0x2A1, d.0 as u64) < 0.5;
            (if moved { "inside" } else { "outside" }, "rain")
        } else {
            ("outside", "")
        };
        Some(DayWeather { temp, place, why })
    })
}

/// What the weather takes from a week: a lost day's field work, half an
/// hour of a delayed one's.
pub fn weather_cuts(wk: &mut Week, wx: &[Option<DayWeather>; 7]) {
    for (d, x) in wk.days.iter_mut().zip(wx.iter()) {
        match x.map(|x| x.place) {
            Some("lost") => d.periods.retain(|p| !p.kind.on_field() || p.own()),
            Some("delayed") => {
                let mut cut = 30u8;
                while cut > 0 {
                    let Some(p) = d.periods.iter_mut().filter(|p| p.kind.on_field() && p.min > 5).max_by_key(|p| p.min)
                    else {
                        break;
                    };
                    p.min -= 5;
                    cut -= 5;
                }
            }
            _ => {}
        }
    }
}

impl SeatTraining {
    pub fn plan_of(&self, id: u32) -> Plan {
        self.plans.iter().find(|p| p.id == id).cloned().unwrap_or(Plan { id, ..Default::default() })
    }

    pub fn set_plan(&mut self, p: Plan) {
        self.plans.retain(|x| x.id != p.id);
        if p != (Plan { id: p.id, ..Default::default() }) {
            self.plans.push(p);
        }
    }

    /// Whether you set his position (a move learned or under way).
    pub fn position_set(&self, id: u32) -> bool {
        self.yours_moved.contains(&id) || self.plans.iter().any(|p| p.id == id && p.moving.is_some())
    }

    pub fn package(&self, b: Board) -> &[u32] {
        self.packages.iter().find(|(x, _)| *x == b).map_or(&[], |(_, v)| v)
    }

    pub fn hurt_on(&self, id: u32, d: Date) -> Option<&Hurt> {
        self.hurt.iter().find(|h| h.id == id && h.until > d)
    }

    /// Out of a game on day `d`: hurt in practice (or on the non-football
    /// list) and not back yet. Only your team's players are kept here.
    pub fn hurt_out(&self, id: u32, d: Date) -> bool {
        self.hurt.iter().any(|h| h.id == id && h.from <= d && h.until > d)
    }

    /// Arrived in the last two weeks (a save from before reads nobody as new).
    pub fn is_new(&self, id: u32, d: Date) -> bool {
        !self.arrived.is_empty() && self.arrived.iter().find(|a| a.0 == id).is_none_or(|a| a.1 > d.add(-14))
    }

    pub fn install_level(&self, key: InstallKey) -> f32 {
        self.installs.iter().find(|i| i.key == key).map_or(0.0, |i| i.level)
    }

    fn note(&mut self, date: Date, id: u32, dept: &str, text: String) {
        self.notes.push(Note { date, id, dept: dept.to_string(), text });
        if self.notes.len() > 400 {
            self.notes.drain(..self.notes.len() - 400);
        }
    }

    /// Registers the saved designs as concepts (after a load).
    pub fn register_designs(&mut self) {
        self.design_ids = self
            .designs
            .iter()
            .map(|d| crate::game::playbook::register_concept(crate::game::designer::to_concept(&d.design)))
            .collect();
    }

    pub fn design_concept(&mut self, k: usize) -> Option<usize> {
        if self.design_ids.len() != self.designs.len() {
            self.register_designs();
        }
        self.design_ids.get(k).copied()
    }
}

/// Your team as training sees it.
#[derive(Clone, Debug)]
pub struct SeatTeam {
    pub tier: Tier,
    pub level: crate::player::Level,
    pub frame: Frame,
    pub weeks: [u8; BLOCKS],
    pub state: &'static str,
    pub heat: bool,
    /// The people on your roster (indices).
    pub roster: Vec<usize>,
    pub style: crate::tendencies::Style,
    /// Your team's games in the coming days.
    pub games: Vec<Date>,
}

/// Your team now, with its frame for the football year of `y`.
pub fn seat_team(w: &World, frames: &Frames) -> Option<SeatTeam> {
    let seat = w.seat?;
    let style = w.career.as_ref().map_or_else(Default::default, |c| w.coaches[c.coach].style());
    let (tier, level, frame, weeks, state, heat, roster, games) = match seat.level {
        Cal::Nfl => {
            let t = seat.nfl_team?;
            let roster: Vec<usize> = w.clubs.get(t)?.roster.clone();
            let games = w.nfl_fixtures(Some(t)).iter().filter_map(|f| f.when).collect();
            (Tier::Pro, crate::player::Level::NFL, frames.pro, frames.pro_weeks, "", false, roster, games)
        }
        Cal::HighSchool => {
            let h = seat.program?;
            let s = w.high_schools.get(h)?.state;
            let (f, wk, code, heat) = frames.states.get(s)?;
            let games = w.hs_fixtures(h).iter().map(|f| f.date).collect();
            let roster = w.high_schools[h].roster.clone();
            (Tier::High, crate::player::Level::HS, *f, *wk, *code, *heat, roster, games)
        }
        _ => {
            let c = seat.program?;
            let prog = w.programs.get(c)?;
            let (f, wk) = frames.division(prog.division);
            let games = w.college_fixtures(Some(c)).iter().map(|f| f.date).collect();
            let level = crate::world::division_level(prog.division);
            (Tier::College, level, *f, *wk, "", false, prog.roster.clone(), games)
        }
    };
    let roster = roster
        .into_iter()
        .filter(|&i| {
            let p = &w.people[i];
            matches!(p.stage, Stage::HighSchool | Stage::College | Stage::Pro)
        })
        .collect();
    Some(SeatTeam { tier, level, frame, weeks, state, heat, roster, style, games })
}

/// The books of a slot for checking a template (a mid-block week).
pub fn slot_books(st: &SeatTeam, slot: Slot) -> [Book; 7] {
    let b = match slot {
        Slot::Game | Slot::Bye => Block::Season,
        Slot::Camp => Block::Camp,
        Slot::Spring => Block::Spring,
        Slot::Winter => Block::Winter,
        Slot::Summer => Block::Summer,
        Slot::Bowl => Block::Postseason,
        Slot::Phase1 => Block::Phase1,
        Slot::Phase2 => Block::Phase2,
        Slot::Otas => Block::Otas,
        Slot::Own => Block::Quiet,
    };
    std::array::from_fn(|_| Book {
        big: st.frame.big,
        ..super::rules::book(st.tier, b, st.frame.season, 30, st.state, st.heat)
    })
}

/// The week planned from Monday `monday`: its block and slot, the books,
/// the week as your staff will run it (your edits, your template or your
/// staff's default, kept inside the rules), the neutral week, and what the
/// staff had to change.
pub struct PlannedWeek {
    pub monday: Date,
    /// The practice days' weather and where the team went (TR15).
    pub weather: [Option<DayWeather>; 7],
    pub block: Block,
    pub slot: Slot,
    pub books: [Book; 7],
    pub week: Week,
    pub neutral: Week,
    pub fitted: Vec<String>,
    pub games: Vec<Date>,
    pub yours: bool,
    /// Run as you wrote it, past the rules (topic 3).
    pub pushed: bool,
}

pub fn planned(w: &World, st: &SeatTeam, monday: Date) -> PlannedWeek {
    let block = block_of_week(&st.frame, monday);
    let bk = books(&st.frame, monday, st.state, st.heat);
    let games: Vec<Date> = st.games.iter().copied().filter(|&g| g >= monday && g < monday.add(7)).collect();
    let bowl = block == Block::Postseason && st.games.iter().any(|&g| g >= monday);
    let slot = Slot::of(block, !games.is_empty(), bowl);
    let t = &w.training;
    let tpl = |s: Slot| super::week::template_of(&t.templates, st.tier, s, &st.style);
    let pushed = t.pushed == Some(monday) && t.this_week.as_ref().is_some_and(|(m, _)| *m == monday);
    let (mut wk, yours) = match &t.this_week {
        Some((m, wk)) if *m == monday => (wk.clone(), true),
        _ => {
            let mut wk = week_from(slot, monday, &games, &tpl(slot), &tpl(Slot::Bye));
            super::plan::extras(&st.frame, monday, &mut wk);
            (wk, t.templates.iter().any(|(s, _)| *s == slot))
        }
    };
    let before = wk.clone();
    if !pushed {
        fit(&mut wk, &bk);
    }
    // What is left of the block's counts (spring's 15 practices, the NFL's
    // 14 padded practices in season).
    let mut lim = block_limits(st.tier, block, st.frame.season);
    // Practices a case took away from this block.
    let lost: u16 = t.lost.iter().filter(|l| l.0 == block && l.1 == st.frame.season).map(|l| l.2).sum();
    if lost > 0 {
        let base = if lim.practices == u16::MAX { st.weeks[block as usize] as u16 * 5 } else { lim.practices };
        lim.practices = base.saturating_sub(lost);
    }
    let used = match t.counts {
        Some((b, s, c)) if b == block && s == st.frame.season && !t.log.last().is_some_and(|l| l.monday == monday) => c,
        Some((b, s, mut c)) if b == block && s == st.frame.season => {
            // Re-planning the week already counted: take it back out.
            if let Some(l) = t.log.last() {
                let mut back = Counts::default();
                back.add_week(&l.week);
                c.practices = c.practices.saturating_sub(back.practices);
                c.contact = c.contact.saturating_sub(back.contact);
                c.padded = c.padded.saturating_sub(back.padded);
                c.scrimmages = c.scrimmages.saturating_sub(back.scrimmages);
            }
            c
        }
        _ => Counts::default(),
    };
    if !pushed {
        super::plan::fit_block(&mut wk, &used, &lim);
    }
    let mut fitted = Vec::new();
    if wk != before {
        for (k, (a, b)) in before.days.iter().zip(wk.days.iter()).enumerate() {
            if a != b {
                let day = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"][k];
                fitted.push(format!("{day}: toned down to stay within the rules ({}).", bk[k].rule));
            }
        }
    }
    let nk =
        super::week::staff_week(st.tier, &st.frame, monday, &games, bowl, st.state, st.heat, &|s| neutral(st.tier, s));
    let weather = practice_weather(w, st, monday, &wk);
    weather_cuts(&mut wk, &weather);
    PlannedWeek { monday, weather, block, slot, books: bk, week: wk, neutral: nk, fitted, games, yours, pushed }
}

/// Saves a template for a slot after checking it against the slot's rules
/// (a period the rules forbid can't be saved, topic 1).
pub fn save_template(w: &mut World, slot: Slot, t: Template) -> Result<(), Vec<Breach>> {
    let y = w.dates.clone();
    let frames = Frames::new(w, &y);
    let st = seat_team(w, &frames).ok_or_else(|| vec![Breach { day: None, text: "You have no team.".into() }])?;
    let bk = slot_books(&st, slot);
    let mut probe = Week { days: t.clone() };
    if slot == Slot::Game {
        probe.days[6].game = true;
    }
    let breaches = check_week(&bk, &probe);
    if !breaches.is_empty() {
        return Err(breaches);
    }
    w.training.templates.retain(|(s, _)| *s != slot);
    w.training.templates.push((slot, t));
    Ok(())
}

/// Clears a slot back to your staff's default.
pub fn reset_template(w: &mut World, slot: Slot) {
    w.training.templates.retain(|(s, _)| *s != slot);
}

/// Saves this week's own plan after checking it against the week's books.
/// With `push` a plan past the rules is kept as written (topic 3: real
/// staffs do it), and it may be found out.
pub fn save_this_week(w: &mut World, monday: Date, wk: Week, push: bool) -> Result<(), Vec<Breach>> {
    let y = w.dates.clone();
    let frames = Frames::new(w, &y);
    let st = seat_team(w, &frames).ok_or_else(|| vec![Breach { day: None, text: "You have no team.".into() }])?;
    let bk = books(&st.frame, monday, st.state, st.heat);
    let mut breaches = check_week(&bk, &wk);
    let block = block_of_week(&st.frame, monday);
    if let Some((b, s, used)) = w.training.counts
        && b == block
        && s == st.frame.season
    {
        let mut c = used;
        c.add_week(&wk);
        breaches.extend(c.over(&block_limits(st.tier, block, s)).into_iter().map(|t| Breach { day: None, text: t }));
    }
    if !breaches.is_empty() && !push {
        return Err(breaches);
    }
    w.training.pushed = (push && !breaches.is_empty()).then_some(monday);
    w.training.this_week = Some((monday, wk));
    Ok(())
}

/// Each player's role this week: his place on your depth chart.
pub fn roles(w: &World, st: &SeatTeam) -> HashMap<usize, Role> {
    let mut out = HashMap::new();
    let mut by_pos: HashMap<Position, Vec<usize>> = HashMap::new();
    for &i in &st.roster {
        by_pos.entry(w.people[i].player.position).or_default().push(i);
    }
    for (pos, mut list) in by_pos {
        let order = w.seat_depth.iter().find(|(p, _)| *p == pos).map(|(_, ids)| ids.clone()).unwrap_or_default();
        list.sort_by(|&a, &b| {
            let ka = order.iter().position(|&id| id == w.people[a].player.id).unwrap_or(usize::MAX);
            let kb = order.iter().position(|&id| id == w.people[b].player.id).unwrap_or(usize::MAX);
            ka.cmp(&kb).then(w.people[b].player.overall().total_cmp(&w.people[a].player.overall()))
        });
        let s = crate::league::depth_slots(pos);
        for (k, i) in list.into_iter().enumerate() {
            let role = if pos.is_specialist() {
                Role::Specialist
            } else if k < s.starters {
                Role::Starter
            } else if k < s.starters + s.rotation.max(1) {
                Role::Backup
            } else {
                Role::Young
            };
            out.insert(i, role);
        }
    }
    out
}

/// A player's own week: his plan's extra work added, rest days and an
/// injury taken off.
pub fn own_week(base: &Week, plan: &Plan, hurt: bool, does_extra: bool) -> Week {
    let mut wk = base.clone();
    if hurt {
        for d in wk.days.iter_mut() {
            // The classroom and the weight room he can do; no field, no mats.
            d.periods.retain(|p| !p.kind.on_field() && !matches!(p.kind, Kind::MatDrills | Kind::Testing));
            if !d.periods.is_empty() {
                d.periods.push(Period::new(Kind::Recovery, 45, super::plan::Gear::None, super::plan::Contact::Air));
            }
        }
        return wk;
    }
    let mut rested = 0;
    for d in wk.days.iter_mut() {
        if rested < plan.rest && d.periods.iter().any(|p| p.kind.on_field()) && !d.game {
            d.periods.retain(|p| !p.kind.on_field());
            rested += 1;
        }
    }
    if does_extra && plan.extra > 0 {
        let g = super::plan::Gear::None;
        let a = super::plan::Contact::Air;
        for d in wk.days.iter_mut().filter(|d| d.periods.iter().any(|p| p.kind.on_field())) {
            if plan.extra & 1 != 0 {
                d.periods.push(Period::new(Kind::Film, 20, g, a));
            }
            if plan.extra & 2 != 0 {
                d.periods.push(Period::new(Kind::Individual, 15, g, a));
            }
            if plan.extra & 4 != 0 {
                d.periods.push(Period::new(Kind::WeightRoom, 20, g, a));
            }
        }
    }
    wk
}

/// What a week's non-contact periods do beyond growth and prep (proposed,
/// not confirmed): a third morning of mat drills costs mood (TR6); routes
/// on air warm the quarterback's timing with his receivers (TR20); outings
/// and the leadership council lift the room and help friendships form
/// (TR32, TR33); study hall moves a flagged player's grades (TR14: under a
/// 2.3 or in his first semester).
fn team_effects(w: &mut World, st: &SeatTeam, pw: &PlannedWeek) {
    let on_air = |p: &Period| p.kind == Kind::OnAir;
    let air = minutes(&pw.week, on_air) - minutes(&pw.neutral, on_air);
    let social = |p: &Period| matches!(p.kind, Kind::TeamBuilding | Kind::Council);
    let tb = minutes(&pw.week, social) - minutes(&pw.neutral, social);
    let mats = pw.week.days.iter().filter(|x| x.periods.iter().any(|p| p.kind == Kind::MatDrills)).count();
    let halls = |wk: &Week| wk.days.iter().flat_map(|x| &x.periods).filter(|p| p.kind == Kind::StudyHall).count();
    let hall = halls(&pw.week) as f64 - halls(&pw.neutral) as f64;
    let t = &mut w.training;
    t.air = (t.air * 0.97 + (0.001 * air / 30.0) as f32).clamp(-0.05, 0.1);
    t.bonds = (t.bonds + (0.01 * tb / 60.0) as f32).clamp(-0.15, 0.25);
    let room = clamp(0.004 * tb / 60.0, -0.01, 0.015) as f32;
    let grind = if mats > 2 { -0.02 * (mats - 2) as f32 } else { 0.0 };
    for &i in &st.roster {
        let id = w.people[i].player.id;
        if grind < 0.0 {
            crate::world::folk::talks::mood_event(w, id, crate::world::folk::talks::Reason::Practice, grind);
        }
        let p = &mut w.people[i];
        p.mood = (p.mood + room).clamp(-1.0, 1.0);
        let flagged = p.bg.gpa < 2.3 || (st.tier == Tier::College && p.college_seasons == 0);
        if hall != 0.0 && st.tier != Tier::Pro && flagged {
            p.bg.gpa = clamp(p.bg.gpa + 0.003 * hall, 0.0, 4.0);
        }
    }
}

/// A week's minutes in the periods `f` picks.
fn minutes(wk: &Week, f: impl Fn(&Period) -> bool) -> f64 {
    wk.days.iter().flat_map(|x| &x.periods).filter(|p| f(p)).map(|p| p.min as f64).sum()
}

/// Mobility against the neutral week (TR23; proposed, not confirmed):
/// looser soft tissue, up to a tenth fewer hamstrings, a tenth more when
/// it's cut.
fn limber(pw: &PlannedWeek) -> f64 {
    let mob = |wk: &Week| minutes(wk, |p| p.kind == Kind::Mobility);
    let (m, n) = (mob(&pw.week), mob(&pw.neutral));
    clamp(1.0 - 0.1 * (m - n) / (n + 15.0), 0.9, 1.1)
}

/// Your team's week seed (the same in the Monday pass and the screens).
fn seat_seed(w: &World, d: Date) -> u64 {
    w.options.seed as u64 ^ (d.0 as u64) << 20 ^ 0x5EA7
}

/// Whether he really does the extra work he says he'll do this week.
fn does_extra(p: &crate::world::Person, seed: u64) -> bool {
    let t = &p.player.personality;
    crate::world::season::key_uniform(seed ^ 0xE7, p.player.id as u64)
        < clamp(0.35 + 0.04 * t.work_ethic + 0.1 * t.traits.z(crate::player::Trait::Professionalism), 0.1, 0.98)
}

/// A player's prep this week on your team (TR36): the AI's read, from his
/// own week (his rest days, his rehab, his extra film) against your
/// staff's neutral week. The screens read the week ahead with it before
/// Monday's pass, and the pass records the same word.
pub fn player_prep(w: &World, st: &SeatTeam, pw: &PlannedWeek, i: usize, role: Role) -> super::prep::Prep {
    prep_detail(w, st, pw, i, role, None).0
}

/// A player's prep with its parts: the minutes of his own week and his
/// role's minutes in the neutral week (a screen's "x of y"). `out`: whether
/// he was out hurt at the week's pass, for a page after it (None: as the
/// pass reads him now).
pub fn prep_detail(
    w: &World,
    st: &SeatTeam,
    pw: &PlannedWeek,
    i: usize,
    role: Role,
    out: Option<bool>,
) -> (super::prep::Prep, super::prep::Parts, super::prep::Parts) {
    use super::prep::{Parts, Prep, Status, against, out_this_week, parts, read};
    let d = pw.monday;
    let p = &w.people[i];
    let id = p.player.id;
    let plan = w.training.plan_of(id);
    let injury = w.training.hurt_on(id, d);
    let out = out.unwrap_or_else(|| p.pro.as_ref().is_some_and(|c| out_this_week(c.out_weeks)));
    let hurt = injury.is_some() || out;
    let key = super::prep::key(w.options.seed as u64, d);
    let practice = pw.week.days.iter().any(|x| x.periods.iter().any(|q| q.kind.on_field()));
    let status = if hurt {
        if injury.is_some_and(|h| h.until <= d.add(3)) { Status::Limited } else { Status::Out }
    } else if pw.books.iter().any(|b| b.voluntary) && !super::week::attends(&p.player, key) {
        Status::Absent
    } else if st.tier == Tier::High && super::offseason::sport_ratio(&p.player, pw.block, d).is_some() {
        Status::Away
    } else if w.training.is_new(id, d) {
        Status::New
    } else if plan.rest > 0 && practice {
        Status::Rested
    } else {
        Status::Full
    };
    if pw.books.iter().all(|b| !b.coaches) {
        return (Prep::none(status), Parts::default(), Parts::default());
    }
    let camp = pw.block == Block::Camp;
    let mine = own_week(&pw.week, &plan, status == Status::Out, does_extra(p, seat_seed(w, d)));
    let x = parts(&mine, role, camp);
    let n = parts(&pw.neutral, role, camp);
    let r = against(&x, &n, &parts(&pw.neutral, Role::Starter, camp));
    let years = match st.tier {
        Tier::High => p.player.age.saturating_sub(14),
        Tier::College => p.college_seasons,
        Tier::Pro => p.pro.as_ref().map_or(0, |c| c.seasons.len() as u32),
    };
    (read(&p.player, &r, x.game, x.test, x.days, status, st.tier, st.frame.season, years, key), x, n)
}

/// Words for how a family is coming along, from your staff's read.
fn family_words(f: Family, up: bool) -> &'static str {
    match (f, up) {
        (Family::Strength, true) => "is stronger at the point of attack",
        (Family::Strength, false) => "hasn't added strength",
        (Family::Speed, true) => "looks quicker in his drills",
        (Family::Speed, false) => "hasn't found any extra burst",
        (Family::Conditioning, true) => "is finishing the conditioning periods strong",
        (Family::Conditioning, false) => "is fading late in practice",
        (Family::Technique, true) => "is cleaner with his technique",
        (Family::Technique, false) => "is still sloppy with his technique",
        (Family::Contact, true) => "is winning more of the physical reps",
        (Family::Contact, false) => "is losing the physical reps",
        (Family::Kicking, true) => "is striking the ball better",
        (Family::Kicking, false) => "is inconsistent striking the ball",
        (Family::Mental, true) => "is playing faster, he sees it now",
        (Family::Mental, false) => "is thinking too much, he's not getting it",
    }
}

/// Your team's week: growth by your plan, habits, body plans, position
/// moves, load, practice injuries, practice battles on the engine, notes,
/// and the block report when a block ends. Returns your players (the
/// world's pass skips them).
pub(crate) fn week(w: &mut World, d: Date, frames: &Frames) -> Vec<usize> {
    w.training.my_risk = 0.0;
    let Some(st) = seat_team(w, frames) else { return Vec::new() };
    if w.design_ids_stale() {
        w.training.register_designs();
    }
    let pw = planned(w, &st, d);
    let roles = roles(w, &st);
    let focus = w.focus_by_person();
    let seed = seat_seed(w, d);
    let camp = pw.block == Block::Camp;
    let base = weekly_base(&st.frame, &st.weeks, pw.block);
    let mut moved: Vec<(u32, f32)> = std::mem::take(&mut w.training.moved);
    let mut injuries = 0;
    let mut notes: Vec<(u32, &'static str, String)> = Vec::new();
    // Block counts.
    let season = st.frame.season;
    match w.training.counts {
        Some((b, s, _)) if b == pw.block && s == season => {}
        _ => w.training.counts = Some((pw.block, season, Counts::default())),
    }
    if let Some((_, _, c)) = w.training.counts.as_mut() {
        c.add_week(&pw.week);
    }
    let own_time = pw.books.iter().all(|b| !b.coaches);
    // Who arrived when (TR8): a player new in the last two weeks reads as
    // new; a save from before this keeps everyone as he was.
    let ids: Vec<u32> = st.roster.iter().map(|&i| w.people[i].player.id).collect();
    let first = w.training.arrived.is_empty();
    w.training.arrived.retain(|a| ids.contains(&a.0));
    for id in ids {
        if !w.training.arrived.iter().any(|a| a.0 == id) {
            w.training.arrived.push((id, if first { d.add(-365) } else { d }));
        }
    }
    let mut away: Vec<(usize, f64)> = Vec::new();
    let (_, full_week, field_week) = week_load(&pw.week, Role::Starter, camp);
    let (_, neutral_full, neutral_field) = week_load(&pw.neutral, Role::Starter, camp);
    // Your NFL club's practice risk is your own week's (the clubs' rule:
    // padded and full-speed work against an average staff's), with the
    // mobility work's looser soft tissue.
    w.training.my_risk = if st.tier == Tier::Pro && !own_time && neutral_field > 0.0 {
        let risk = ((full_week + 10.0) / (neutral_full + 10.0)).sqrt()
            * ((field_week + 30.0) / (neutral_field + 30.0)).powf(0.3);
        (clamp(risk, 0.5, 2.0) * (0.77 + 0.23 * limber(&pw))) as f32
    } else {
        0.0
    };
    // Nutrition sessions feed the body plans (TR31; proposed, not
    // confirmed: a session a week the neutral week lacks moves a body plan
    // about a seventh faster).
    let nut = |wk: &Week| minutes(wk, |p| p.kind == Kind::Nutrition);
    let fuel = clamp(1.0 + 0.15 * (nut(&pw.week) - nut(&pw.neutral)) / (nut(&pw.neutral) + 30.0), 0.9, 1.15);
    for &i in &st.roster {
        let id = w.people[i].player.id;
        let role = *roles.get(&i).unwrap_or(&Role::Young);
        let plan = w.training.plan_of(id);
        let hurt = w.training.hurt_on(id, d).is_some()
            || w.people[i].pro.as_ref().is_some_and(|c| super::prep::out_this_week(c.out_weeks));
        // The week's prep (TR36): read before anything moves him.
        let pr = player_prep(w, &st, &pw, i, role);
        let absent = pr.status == super::prep::Status::Absent;
        let p = &w.people[i];
        let absorb = absorb_factor(&p.player);
        // Extra work he says he'll do: whether he really does it.
        let does = does_extra(p, seed);
        let mine = own_week(&pw.week, &plan, hurt, does);
        let r = if absent {
            // Not at a voluntary week: his own work, as on his own time.
            [super::offseason::own_effort(&p.player, p.pro.as_ref().is_some_and(|c| c.paid), false); FAMILIES]
        } else if own_time {
            // His time on his own (topic 9): what he really does.
            let paid = p.pro.as_ref().is_some_and(|c| c.paid);
            let last_year = p
                .pro
                .as_ref()
                .and_then(|c| c.contract.as_ref())
                .is_some_and(|k| k.signed + k.years.len() as i32 - 1 <= w.year);
            let effort = super::offseason::own_effort(&p.player, paid, last_year)
                * if plan.extra > 0 && does { 1.08 } else { 1.0 };
            away.push((i, effort));
            [effort; FAMILIES]
        } else {
            ratio(&week_work(&mine, role, camp, absorb), &week_work(&pw.neutral, role, camp, absorb))
        };
        let mut r = r;
        // College and high school summers are the staff's: the strength
        // staff see who comes and how hard he works (topic 9).
        if !own_time && pw.block == Block::Summer && st.tier != Tier::Pro {
            away.push((i, super::offseason::own_effort(&p.player, false, false)));
        }
        // A high schooler's other sport takes its share of the week.
        if st.tier == Tier::High
            && let Some((_, sr)) = super::offseason::sport_ratio(&p.player, pw.block, d)
        {
            for f in 0..FAMILIES {
                r[f] *= sr[f];
            }
        }
        // The body plan's emphasis moves the weight room's work.
        if let Some((_, emph)) = plan.body {
            let (s, sp) = match emph {
                0 => (1.08, 0.95),
                1 => (0.95, 1.08),
                _ => (0.98, 1.02),
            };
            r[Family::Strength as usize] *= s;
            r[Family::Speed as usize] *= sp;
        }
        let load = week_load(&mine, role, camp).0;
        let p = &mut w.people[i];
        p.player.train.add_load(load);
        let prev = super::prep::latest(&p.player.train);
        super::prep::record(&mut p.player, pr.word, pr.mental);
        if pr.status == super::prep::Status::Out {
            super::prep::mark_out(&mut p.player.train);
        }
        if (1..=2).contains(&pr.word) && (1..=2).contains(&prev) {
            let word = super::prep::WORDS[pr.word as usize].to_lowercase();
            notes.push((id, "position", format!("has been {word} two weeks running: {}", pr.why())));
        }
        let spike = p.player.train.spike();
        if spike > 1.45 && !hurt {
            notes.push((
                id,
                "trainers",
                format!("load is up sharply this week ({:.1} times his usual): watch his reps", spike),
            ));
        }
        if p.player.train.pending() {
            let b = cover(&mut p.player.train, &base);
            let mut s = share(&b, &r, 1.0);
            // A run of poor prep slows the season's growth (TR40).
            super::prep::slow(&p.player.train, &mut s);
            let due = p.player.train.due.map(|v| v as f64 / 2000.0);
            p.player.train.due = [0; FAMILIES];
            for f in 0..FAMILIES {
                s[f] += due[f];
            }
            let before = p.player.attributes;
            let gain = land(&mut p.player, &s, focus.get(&i).copied(), seed);
            if let Some(m) = moved.iter_mut().find(|m| m.0 == id) {
                m.1 += gain as f32;
            } else {
                moved.push((id, gain as f32));
            }
            // His position coach's read of the week (noisy): a note on the
            // family that moved most, now and then.
            let mut by_f = [0.0; FAMILIES];
            for a in Attr::ALL {
                by_f[family_of(a) as usize] +=
                    (p.player.attributes.get(a) - before.get(a)) * crate::positions::weight(p.player.position, a);
            }
            let (fbest, vbest) =
                by_f.iter().enumerate().fold((0, 0.0), |acc, (f, &v)| if v > acc.1 { (f, v) } else { acc });
            let noise = crate::world::season::key_normal(seed ^ 0x40, id as u64);
            if vbest + 0.04 * noise > 0.12 && crate::world::season::key_uniform(seed ^ 0x41, id as u64) < 0.35 {
                notes.push((id, "position", family_words(Family::ALL[fbest], true).to_string()));
            } else if gain < 0.01
                && role != Role::Starter
                && crate::world::season::key_uniform(seed ^ 0x42, id as u64) < 0.05
            {
                notes.push((id, "position", family_words(Family::ALL[fbest], false).to_string()));
            }
        }
        // A habit coached toward its target (topic 5): the built coaching
        // pull, spread over the practice weeks, as far as he obeys.
        if let Some((k, target)) = plan.habit
            && !own_time
            && !hurt
        {
            let tend = crate::tendencies::Tend::ALL[k as usize % crate::tendencies::Tend::ALL.len()];
            let teach = staff_teaching(w, &st, i);
            let p = &mut w.people[i];
            let per_week = habit_rate(teach, p.player.personality.coachability, p.player.age) / 30.0;
            let disc = p.player.attributes.get(Attr::Discipline) / 5.0;
            let obey = crate::tendencies::obey_chance(disc, p.player.personality.coachability, 0.5, 10.0);
            let v = p.player.tend.get(tend);
            let gap = target as f64 - v;
            let step = gap.signum() * gap.abs().min(per_week * obey);
            p.player.tend.set(tend, v + step);
            if (v + step - target as f64).abs() < 0.5 && gap.abs() >= 0.5 {
                notes.push((id, "position", format!("has the habit where we want it ({})", tend.label())));
            }
        }
        // The body plan: a real rate toward the weight target.
        if let Some((target, _)) = plan.body {
            let p = &mut w.people[i];
            let now = crate::body::body_now(&p.player).weight;
            let gap = target as f64 - now;
            let rate = if gap > 0.0 { 1.5 } else { 1.2 }
                * clamp(0.7 + 0.03 * p.player.personality.work_ethic, 0.7, 1.3)
                * fuel;
            let step = gap.signum() * gap.abs().min(rate);
            if step.abs() > 0.05 {
                body_change(&mut p.player, step);
            }
        }
        // A position move learned over practice time.
        if let Some((to, prog)) = plan.moving
            && pw.books.iter().any(|b| b.field)
            && !hurt
        {
            let iq = w.people[i].player.attributes.get(Attr::FootballIq);
            let add = clamp(9.0 + 0.1 * (iq - 60.0), 5.0, 14.0) as u8;
            let next = prog.saturating_add(add).min(100);
            let mut plan2 = plan.clone();
            if next >= 100 {
                let pos = Position::ALL[to as usize % Position::ALL.len()];
                crate::athlete::set_position(&mut w.people[i].player, pos);
                plan2.moving = None;
                if !w.training.yours_moved.contains(&id) {
                    w.training.yours_moved.push(id);
                }
                notes.push((id, "position", format!("is ready to play {} now", pos.name())));
            } else {
                plan2.moving = Some((to, next));
            }
            w.training.set_plan(plan2);
        }
        let _ = role;
    }
    // Practice injuries (college and high school; the NFL's come from its
    // practice day with your plan's risk).
    if st.tier != Tier::Pro && !own_time {
        injuries += practice_injuries(w, &st, d, &pw, full_week, neutral_full, &roles);
    }
    // Practice battles on the engine (choice 3).
    let mut log = WeekLog {
        monday: d,
        block: Some(pw.block),
        week: pw.week.clone(),
        snaps: 0,
        injuries,
        fitted: pw.fitted.clone(),
        best: Vec::new(),
        worst: Vec::new(),
        battles: Vec::new(),
    };
    if !own_time {
        super::practice::seat_battles(w, &st, &pw, &roles, seed, &mut log);
    }
    if !own_time {
        team_effects(w, &st, &pw);
        // The strength staff's testing day (TR5), and a new staff's
        // baseline in its first week.
        let testing = super::plan::testing_day(&st.frame, d);
        if testing.is_some() || w.training.tests.is_empty() {
            let key = seed ^ 0x7E57;
            let rows: Vec<TestRow> = st.roster.iter().map(|&i| test_row(&w.people[i].player, key)).collect();
            let day = d.add(testing.unwrap_or(0) as i32);
            w.training.tests.push(TestDay { date: day, rows, baseline: testing.is_none() });
            if w.training.tests.len() > 3 {
                w.training.tests.remove(0);
            }
        }
    }
    // Installs: this week's team and situation periods put in the plan's
    // families and your designs (plays and tactics topic 4).
    install_week(w, &pw);
    for (id, dept, text) in notes {
        let name = w.people.iter().find(|p| p.player.id == id).map_or(String::new(), |p| p.name.clone());
        w.training.note(d, id, dept, format!("[[p:{id}|{name}]] {text}."));
    }
    offseason_week(w, &st, d, &away, seed);
    report_day(w, &st, d, seed);
    if st.tier == Tier::High && !own_time {
        log.injuries += heat_week(w, &st, d, &pw, seed);
    }
    if pw.pushed {
        practice_case(w, &st, d, &pw, log.injuries > 0, seed);
    }
    if pw.block == Block::Camp {
        camp_news(w, &st, d, &log, &roles);
    }
    w.training.moved = moved;
    w.training.log.push(log);
    if w.training.log.len() > 16 {
        w.training.log.remove(0);
    }
    // End of a block: the department heads report.
    if st.frame.block(d.add(7)) != pw.block {
        block_report(w, &st, d, pw.block);
    }
    w.training.this_week = w.training.this_week.take().filter(|(m, _)| *m > d);
    let _ = SLOTS;
    st.roster
}

/// Your players' time on their own (topic 9): each week's real effort is
/// kept for report day, and a few signs reach your staff, each only as
/// true as whoever gives it.
fn offseason_week(w: &mut World, st: &SeatTeam, d: Date, away: &[(usize, f64)], seed: u64) {
    if away.is_empty() {
        return;
    }
    let mut shown = 0;
    for &(i, effort) in away {
        let p = &w.people[i];
        let id = p.player.id;
        let wt = crate::body::body_now(&p.player).weight.round().clamp(0.0, 999.0) as u16;
        match w.training.away.iter_mut().find(|a| a.id == id) {
            Some(a) => {
                a.effort += effort as f32;
                a.weeks = a.weeks.saturating_add(1);
            }
            None => w.training.away.push(Away { id, effort: effort as f32, weeks: 1, start_wt: wt }),
        }
        // About one sign a player every four weeks, at most six a week.
        let u = crate::world::season::key_uniform(seed ^ 0x51C, id as u64);
        if u > 0.25 || shown >= 6 {
            continue;
        }
        let noise = crate::world::season::key_uniform(seed ^ 0x51D, id as u64) * 2.0 - 1.0;
        let pick = crate::world::season::key_uniform(seed ^ 0x51E, id as u64);
        if let Some(sg) = super::offseason::sign(st.tier, &p.player, &p.name.clone(), effort, noise, pick) {
            w.training.note(d, id, sg.dept, sg.text);
            shown += 1;
        }
    }
}

/// Report day (topic 9): the weigh-in against his target and the
/// conditioning test, the week camp opens. Failing the test puts him on
/// the non-football injury list until he passes.
fn report_day(w: &mut World, st: &SeatTeam, d: Date, seed: u64) {
    let report = st.frame.report.max(st.frame.camp);
    if !(report >= d && report < d.add(7)) || w.training.away.is_empty() {
        return;
    }
    let away = std::mem::take(&mut w.training.away);
    let (mut tested, mut failed, mut heavy) = (0, Vec::new(), Vec::new());
    for a in away {
        let Some(i) = st.roster.iter().copied().find(|&i| w.people[i].player.id == a.id) else { continue };
        let effort = a.effort as f64 / a.weeks.max(1) as f64;
        let p = &w.people[i];
        let stamina = p.player.attributes.get(Attr::Stamina);
        tested += 1;
        let chance = super::offseason::fail_chance(effort, stamina, st.tier);
        let name = p.name.clone();
        if crate::world::season::key_uniform(seed ^ 0x7E57, a.id as u64) < chance {
            let days = 3 + (crate::world::season::key_uniform(seed ^ 0x7E58, a.id as u64) * 8.0) as i32;
            w.training.hurt.push(Hurt {
                id: a.id,
                kind: None,
                from: report,
                until: report.add(days),
                how: "failed the conditioning test: on the non-football injury list until he passes".into(),
            });
            failed.push(format!("[[p:{}|{name}]]", a.id));
        }
        let now = crate::body::body_now(&w.people[i].player).weight;
        let target = w.training.plan_of(a.id).body.map(|b| b.0 as f64);
        let over = match target {
            Some(t) => now - t,
            None => now - a.start_wt as f64 - 6.0,
        };
        if over >= 7.0 {
            heavy.push(format!("[[p:{}|{name}]] ({:.0} lb over)", a.id, over));
        }
        let text = if effort > 1.1 {
            "reported in great shape: the summer's work shows"
        } else if effort < 0.85 {
            "reported out of shape: a lost summer"
        } else {
            "reported in good shape"
        };
        if !(0.85..=1.1).contains(&effort) {
            w.training.note(report, a.id, "strength", format!("[[p:{}|{name}]] {text}.", a.id));
        }
    }
    let mut line = format!("Report day: {} of {tested} passed the conditioning test.", tested - failed.len());
    if !failed.is_empty() {
        line += &format!(" Failed: {}.", failed.join(", "));
    }
    if !heavy.is_empty() {
        line += &format!(" Over their weight: {}.", heavy.join(", "));
    }
    w.training.note(report, 0, "strength", line);
}

/// High school heat (topic 10): in August a practice day with a heat index
/// of 104 F or more at 3pm. In a state with a heat rule the practice is cut
/// to an hour and moved indoors or to the evening (minutes lost); in one
/// without, it goes ahead and each player risks heat illness (proposed,
/// not confirmed: about 1 in 300 a hot practice, from the NCAA and NFHS
/// exertional heat illness counts, which peak in the first August weeks).
fn heat_week(w: &mut World, st: &SeatTeam, d: Date, pw: &PlannedWeek, seed: u64) -> u32 {
    let Some(h) = w.seat_school() else { return 0 };
    let state = w.high_schools[h].state;
    let code = st.state;
    let mut n = 0;
    for (k, day) in pw.week.days.iter().enumerate() {
        let date = d.add(k as i32);
        let (_, m, _) = date.parts();
        if !(7..=9).contains(&m) || !day.periods.iter().any(|p| p.kind.practice()) {
            continue;
        }
        let hi = crate::calendar::weather::weather_at(w.options.seed as u64, state, code, date, 15).heat_index();
        if hi < 104.0 {
            continue;
        }
        if st.heat {
            w.training.note(date, 0, "trainers", format!("Heat index {hi:.0} F at 3pm: practice cut to an hour and moved to the evening, by the state's heat rule."));
            continue;
        }
        let mut hurt_today = Vec::new();
        for &i in &st.roster {
            let p = &w.people[i];
            let id = p.player.id;
            if w.training.hurt_on(id, date).is_some() {
                continue;
            }
            let heavy = if p.player.body.weight >= 260.0 { 1.8 } else { 1.0 };
            let fit = (1.2 - 0.004 * p.player.attributes.get(Attr::Stamina)).max(0.6);
            let chance = 0.0033 * heavy * fit * (1.0 + (hi - 104.0) / 10.0);
            if crate::world::season::key_uniform(seed ^ 0x4EA7 ^ k as u64, id as u64) < chance {
                hurt_today.push((id, p.name.clone()));
            }
        }
        for (id, name) in hurt_today {
            w.training.hurt.push(Hurt {
                id,
                kind: None,
                from: date,
                until: date.add(3),
                how: "heat illness at practice".into(),
            });
            w.training.note(
                date,
                id,
                "trainers",
                format!(
                    "[[p:{id}|{name}]] went down with heat illness at practice (heat index {hi:.0} F); out three days."
                ),
            );
            n += 1;
        }
    }
    n
}

/// A week practised past the rules (topic 3): found out the real ways, and
/// punished as real leagues punish it.
fn practice_case(w: &mut World, st: &SeatTeam, d: Date, pw: &PlannedWeek, hurt: bool, seed: u64) {
    let mut what: Vec<String> = check_week(&pw.books, &pw.week).into_iter().map(|b| b.text).collect();
    if let Some((b, s, c)) = w.training.counts
        && b == pw.block
        && s == st.frame.season
    {
        what.extend(c.over(&block_limits(st.tier, pw.block, s)));
    }
    if what.is_empty() {
        return;
    }
    let chance = super::offseason::found_out(st.tier, what.len(), hurt);
    let u = crate::world::season::key_uniform(seed ^ 0xCA5E, d.0 as u64);
    if u >= chance {
        w.training.note(
            d,
            0,
            "coordinator",
            "We went past the rules this week. Nobody has said anything yet.".to_string(),
        );
        return;
    }
    let how = match (st.tier, (u / chance * 3.0) as u32) {
        (Tier::Pro, 0) => "a player called the union",
        (Tier::Pro, 1) => "a video from practice reached the league office",
        (Tier::College, 0) => "a player's parent called the compliance office",
        (Tier::College, 1) => "a former graduate assistant told the NCAA",
        (Tier::High, 0) => "a parent called the state association",
        (Tier::High, 1) => "a rival coach saw the practice and reported it",
        _ => "an assistant who left took it to the league",
    };
    let next_block = match (st.tier, pw.block) {
        (Tier::Pro, Block::Phase2 | Block::Otas) => Block::Otas,
        (_, b) => b,
    };
    let season = if next_block == pw.block && st.tier == Tier::Pro { st.frame.season + 1 } else { st.frame.season };
    let (pen, lost) = super::offseason::penalty(st.tier, pw.block, w.training.cases.len());
    w.training.lost.push((next_block, season, lost));
    w.training.cases.push(Case { date: d, week: d, what: what.clone(), how: how.to_string(), penalty: pen.clone() });
    let team = match w.seat {
        Some(s) if s.level == crate::calendar::Level::Nfl => s.nfl_team.map(|t| w.nfl_team_name(t)),
        Some(s) if s.level == crate::calendar::Level::HighSchool => s.program.map(|h| w.school_name(h).to_string()),
        Some(s) => s.program.map(|c| w.program_name(c).to_string()),
        None => None,
    }
    .unwrap_or_default();
    w.training.note(d, 0, "coordinator", format!("Found out: {how}. {}", upper_first(&pen)));
    w.news.push(crate::world::news::NewsItem {
        date: Some(d),
        year: w.year,
        kind: crate::world::news::NewsKind::PracticeCase,
        channel: crate::world::news::Channel::News,
        program: w
            .seat
            .filter(|s| s.level != crate::calendar::Level::Nfl && s.level != crate::calendar::Level::HighSchool)
            .and_then(|s| s.program),
        text: format!(
            "{team} broke the practice rules ({}): {pen}.",
            what.first().cloned().unwrap_or_default().trim_end_matches('.')
        ),
        refs: Vec::new(),
    });
}

fn upper_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str() + ".",
        None => String::new(),
    }
}

/// Camp as news (topic 8): at an NFL club or a big program the beat
/// writers report a backup who keeps winning and a starter losing reps.
fn camp_news(w: &mut World, st: &SeatTeam, d: Date, log: &WeekLog, roles: &HashMap<usize, Role>) {
    let big = match st.tier {
        Tier::Pro => true,
        Tier::College => st.level == crate::player::Level::FBS,
        Tier::High => false,
    };
    if !big {
        return;
    }
    let by_id: HashMap<u32, usize> = st.roster.iter().map(|&i| (w.people[i].player.id, i)).collect();
    let riser = log.best.iter().find(|(id, g)| {
        *g >= 70 && by_id.get(id).is_some_and(|i| roles.get(i).is_some_and(|r| matches!(r, Role::Backup | Role::Young)))
    });
    let slider = log
        .worst
        .iter()
        .find(|(id, g)| *g <= 50 && by_id.get(id).is_some_and(|i| roles.get(i) == Some(&Role::Starter)));
    let team = match st.tier {
        Tier::Pro => w.seat.and_then(|s| s.nfl_team).map(|t| w.nfl_team_name(t)),
        _ => w.seat.and_then(|s| s.program).map(|c| w.program_name(c).to_string()),
    }
    .unwrap_or_default();
    let mut lines = Vec::new();
    if let Some(&(id, _)) = riser {
        let p = &w.people[by_id[&id]];
        lines.push((id, p.name.clone(), format!("Camp notes: {} {} has been the story of {team}'s week, winning his reps and getting more of them with the ones.", p.player.position.name(), p.name)));
    }
    if let Some(&(id, _)) = slider {
        let p = &w.people[by_id[&id]];
        lines.push((
            id,
            p.name.clone(),
            format!(
                "Camp notes: {team}'s starting {} {} had a rough week; the staff gave more of his reps to the backups.",
                p.player.position.name(),
                p.name
            ),
        ));
    }
    for (id, name, text) in lines {
        w.news.push(
            crate::world::news::NewsItem {
                date: Some(d),
                year: w.year,
                kind: crate::world::news::NewsKind::Camp,
                channel: crate::world::news::Channel::News,
                program: w.seat.filter(|s| s.level != crate::calendar::Level::Nfl).and_then(|s| s.program),
                text,
                refs: Vec::new(),
            }
            .link(crate::calendar::Link::Person(id), &name),
        );
    }
}

/// The coaching of a habit (tendencies topic 3.4): about 3 a year for a
/// strong coach with a coachable player, 1.5 for an average pair.
pub fn habit_rate(teaching: f64, coachability: f64, age: u32) -> f64 {
    let settle = clamp((26.5 - age as f64) / 4.5, 0.12, 1.0);
    clamp(0.375 * (teaching - 8.0) * (0.45 + 0.037 * coachability), 0.0, 3.4) * settle.max(0.35)
}

/// His position coach's teaching.
pub fn staff_teaching(w: &World, st: &SeatTeam, i: usize) -> f64 {
    let p = &w.people[i];
    let g = crate::world::coach::Group::of(p.player.position);
    match st.tier {
        Tier::High => w.seat_school().and_then(|h| w.hs_staff_teaching(h, g)).unwrap_or(9.0),
        Tier::College => p
            .college
            .and_then(|c| w.programs[c].position_coaches[g as usize].or(w.programs[c].head_coach))
            .map_or(9.0, |c| w.coaches[c].teaching(g)),
        Tier::Pro => p
            .pro
            .as_ref()
            .and_then(|c| c.team)
            .and_then(|t| w.nfl[t].head_coach)
            .map_or(10.0, |c| w.coaches[c].teaching(g)),
    }
}

/// Weight changes the body as built: heavier is stronger and slower
/// (proposed, not confirmed: per pound, strength and anchor up a little,
/// speed, quickness and stamina down a little; ceilings move with them).
pub fn body_change(p: &mut crate::player::Player, lbs: f64) {
    let now = crate::body::body_now(p).weight.max(100.0);
    let k = p.body.weight / now;
    p.body.weight = clamp(p.body.weight + lbs * k, 120.0, 400.0);
    for (a, per) in [
        (Attr::Strength, 0.05),
        (Attr::Anchor, 0.04),
        (Attr::Speed, -0.03),
        (Attr::Acceleration, -0.02),
        (Attr::Agility, -0.03),
        (Attr::Stamina, -0.02),
    ] {
        let dv = per * lbs;
        p.attributes.add(a, dv);
        p.ceilings.add(a, dv);
    }
}

/// A time-loss injury's chance per practice for an average player
/// (proposed, not confirmed: set so a college season's practice share of
/// injuries lands on the NCAA Injury Surveillance Program's split, about
/// 55-62% of all injuries in practice by count with the most in preseason
/// camp, on the engine's game injury rate; camp about three times the
/// season's rate per practice, spring about twice; high school about 40%
/// of college).
pub fn practice_rate(tier: Tier, block: Block) -> f64 {
    let per = match block {
        Block::Camp => 0.0037,
        Block::Spring => 0.0023,
        Block::Season | Block::Postseason => 0.00115,
        _ => 0.0008,
    };
    per * if tier == Tier::High { 0.4 } else { 1.0 }
}

/// Soft-tissue risk from a load spike (acute against chronic above 1.3).
pub fn soft_tissue(spike: f64) -> f64 {
    if spike > 1.3 { 1.0 + 1.5 * (spike - 1.3) } else { 1.0 }
}

/// His chance of a practice injury over a week's `practices`: the block's
/// rate, the week's contact against the neutral week's, his proneness
/// (1-20), his exposure (a specialist's quarter) and his load spike.
pub fn injury_chance(
    tier: Tier,
    block: Block,
    practices: f64,
    contact: f64,
    proneness: f64,
    exposure: f64,
    spike: f64,
) -> f64 {
    let prone = 0.42 + proneness / 18.0;
    1.0 - (1.0 - practice_rate(tier, block) * contact * prone * exposure * soft_tissue(spike).sqrt()).powf(practices)
}

/// Practice injuries on your team (topic 7): every rep carries risk from
/// its contact, gear and tempo, his load against his usual, and his body.
/// The neutral week lands on the NCAA's practice rates (proposed, not
/// confirmed: camp about 9.7 time-loss injuries per 1000 practices, the
/// season about 3, spring about 6; high school about a third of college).
fn practice_injuries(
    w: &mut World,
    st: &SeatTeam,
    d: Date,
    pw: &PlannedWeek,
    full: f64,
    neutral_full: f64,
    roles: &HashMap<usize, Role>,
) -> u32 {
    let practices = pw.week.days.iter().filter(|x| x.periods.iter().any(|p| p.kind.on_field())).count() as f64;
    if practices == 0.0 {
        return 0;
    }
    let contact = ((full + 10.0) / (neutral_full + 10.0)).powf(0.7);
    let limber = limber(pw);
    let med = 1.0;
    const MIX: [(InjuryKind, f64); 12] = [
        (InjuryKind::Hamstring, 22.0),
        (InjuryKind::AnkleSprain, 15.0),
        (InjuryKind::Knee, 14.0),
        (InjuryKind::Bruise, 10.0),
        (InjuryKind::Shoulder, 7.0),
        (InjuryKind::Back, 6.0),
        (InjuryKind::Hand, 6.0),
        (InjuryKind::HighAnkleSprain, 4.0),
        (InjuryKind::Concussion, 4.0),
        (InjuryKind::Acl, 2.5),
        (InjuryKind::Ribs, 2.0),
        (InjuryKind::Fracture, 2.0),
    ];
    let seed = w.options.seed ^ d.0 as u32 ^ 0x1A7D;
    let mut n = 0;
    for &i in &st.roster {
        let p = &w.people[i];
        let id = p.player.id;
        if w.training.hurt_on(id, d).is_some() {
            continue;
        }
        let role = *roles.get(&i).unwrap_or(&Role::Young);
        let exposure = if role == Role::Specialist { 0.25 } else { 1.0 };
        let spike = p.player.train.spike();
        let soft = soft_tissue(spike) * limber;
        let chance = injury_chance(st.tier, pw.block, practices, contact, p.player.injury_proneness, exposure, spike)
            * (0.77 + 0.23 * limber);
        let mut rng = Rng::new(seed ^ id.wrapping_mul(0x9E37_79B1));
        if !rng.chance(chance / med) {
            continue;
        }
        let mix: Vec<(InjuryKind, f64)> = MIX
            .iter()
            .map(|&(k, wgt)| {
                let m = match k {
                    InjuryKind::Hamstring => soft,
                    InjuryKind::Concussion | InjuryKind::Shoulder => contact,
                    _ => 1.0,
                };
                (k, wgt * m)
            })
            .collect();
        let kind = rng.weighted(&mix);
        let weeks = crate::game::sim::injury_weeks(kind, &mut rng) * (1.3 - 0.03 * p.player.game.recovery);
        let days = (weeks * 7.0).round().max(1.0) as i32;
        let day = rng.int(0, 5);
        let name = p.name.clone();
        w.training.hurt.push(Hurt {
            id,
            kind: Some(kind),
            from: d.add(day),
            until: d.add(day + days),
            how: "practice".into(),
        });
        w.training.note(
            d.add(day),
            id,
            "trainers",
            format!(
                "{name} hurt in practice: {}, out about {} day{}.",
                kind.name(),
                days,
                if days == 1 { "" } else { "s" }
            ),
        );
        n += 1;
    }
    w.training.hurt.retain(|h| h.until > d.add(-60));
    n
}

/// This week's installs (plays and tactics topic 4): the team and
/// situation periods put the plan in.
fn install_week(w: &mut World, pw: &PlannedWeek) {
    let mins: f64 = pw
        .week
        .days
        .iter()
        .flat_map(|x| x.periods.iter())
        .filter(|p| matches!(p.kind, Kind::Team | Kind::Situations | Kind::SevenOnSeven | Kind::Walkthrough))
        .map(|p| p.min as f64)
        .sum();
    if mins <= 0.0 {
        return;
    }
    let mut fams: Vec<u8> = Vec::new();
    for (_, plays) in &w.training.plan.sheet {
        for sp in plays {
            let f = match sp.play {
                SheetKind::Run(r) => r.family() as u8,
                SheetKind::Pass(k) => {
                    crate::game::playbook::pass_family(crate::game::playbook::pass_concept(k as usize)) as u8
                }
                SheetKind::Design(_) => continue,
            };
            if !fams.contains(&f) {
                fams.push(f);
            }
        }
    }
    let designs: Vec<u16> =
        w.training.designs.iter().enumerate().filter(|(_, x)| x.installing).map(|(k, _)| k as u16).collect();
    let keys: Vec<InstallKey> =
        fams.into_iter().map(InstallKey::Family).chain(designs.into_iter().map(InstallKey::Design)).collect();
    if keys.is_empty() {
        return;
    }
    // A play takes about 60 team reps to master (proposed, not confirmed).
    let reps = (mins * 1.5 / keys.len() as f64).round() as u32;
    for k in keys {
        match w.training.installs.iter_mut().find(|i| i.key == k) {
            Some(i) => {
                i.reps += reps;
                i.level = (1.0 - (-(i.reps as f64) / 60.0).exp()) as f32;
            }
            None => {
                w.training.installs.push(Install { key: k, reps, level: (1.0 - (-(reps as f64) / 60.0).exp()) as f32 })
            }
        }
    }
}

/// A block's end: the department heads' short reports (topic 13).
fn block_report(w: &mut World, st: &SeatTeam, d: Date, block: Block) {
    let mut lines = Vec::new();
    let logs: Vec<&WeekLog> = w.training.log.iter().filter(|l| l.block == Some(block)).collect();
    let practices: usize =
        logs.iter().map(|l| l.week.days.iter().filter(|x| x.periods.iter().any(|p| p.kind.on_field())).count()).sum();
    let hurt: u32 = logs.iter().map(|l| l.injuries).sum();
    lines.push((
        "Head coach".to_string(),
        format!("{} weeks, {practices} practices, {} practice injuries.", logs.len(), hurt),
    ));
    let mut moved = w.training.moved.clone();
    moved.sort_by(|a, b| b.1.total_cmp(&a.1));
    let name = |id: u32| w.people.iter().find(|p| p.player.id == id).map_or(String::new(), |p| p.name.clone());
    let up: Vec<String> = moved.iter().take(3).filter(|m| m.1 > 0.3).map(|m| name(m.0)).collect();
    if !up.is_empty() {
        lines.push(("Position coaches".to_string(), format!("Coming on: {}.", up.join(", "))));
    }
    let behind: Vec<String> = moved
        .iter()
        .rev()
        .filter(|m| m.1 < 0.05 && w.people.iter().any(|p| p.player.id == m.0 && p.player.age <= 21))
        .take(3)
        .map(|m| name(m.0))
        .collect();
    if !behind.is_empty() {
        lines.push(("Position coaches".to_string(), format!("Not getting it yet: {}.", behind.join(", "))));
    }
    let flags = st.roster.iter().filter(|&&i| w.people[i].player.train.spike() > 1.4).count();
    lines.push((
        "Trainers".to_string(),
        format!(
            "{flags} players with a load spike; {} on the injury report.",
            w.training.hurt.iter().filter(|h| h.until > d).count()
        ),
    ));
    let inst: Vec<String> = w
        .training
        .installs
        .iter()
        .filter(|i| matches!(i.key, InstallKey::Family(_)))
        .map(|i| match i.key {
            InstallKey::Family(f) => format!(
                "{} {:.0}%",
                crate::game::playbook::Family::ALL[f as usize % crate::game::playbook::FAMILY_COUNT].name(),
                i.level * 100.0
            ),
            _ => String::new(),
        })
        .collect();
    if !inst.is_empty() {
        lines.push(("Coordinators".to_string(), format!("Installed: {}.", inst.join(", "))));
    }
    w.training.reports.push(Report { date: d, block, lines });
    if w.training.reports.len() > 12 {
        w.training.reports.remove(0);
    }
    w.training.moved.clear();
    w.training.grades.clear();
}

/// Your players hurt in practice (or on the non-football list) sit today's
/// game: taken off a game team built for the day. The college team kept
/// for the season keeps them; its game day takes them off.
pub(crate) fn sit_hurt(w: &World, team: &mut crate::game::GameTeam) {
    let (t, d) = (&w.training, w.today);
    team.players.retain(|p| !t.hurt_out(p.id, d));
}

/// Your team's game team takes the week: the late week's load sets kickoff
/// energy, your packages and call sheet go with it, and the install
/// follows the week's prep. Safe to run again on the same team (the
/// college team kept for the season is refreshed after every change): it
/// sets from the staff's own install, never from the last result.
pub(crate) fn prepare(w: &World, team: &mut crate::game::GameTeam) {
    let t = &w.training;
    let base_install = *team.prep.install_base.get_or_insert(team.staff.install);
    team.staff.install = base_install;
    let prep = &mut team.prep;
    prep.packages = t.packages.iter().map(|(b, ids)| (*b as u8, ids.clone())).collect();
    // Each box splits by hash (update 2, G10): the key carries it.
    prep.sheet = t
        .plan
        .sheet
        .iter()
        .flat_map(|(sit, plays)| {
            let mut hs: Vec<u8> = plays.iter().map(|p| p.hash.min(2)).collect();
            hs.sort_unstable();
            hs.dedup();
            hs.into_iter().map(move |h| (*sit, h, plays.iter().filter(move |p| p.hash.min(2) == h)))
        })
        .map(|(sit, h, plays)| {
            (
                crate::game::sheet::key(sit, h),
                plays
                    .filter_map(|sp| {
                        let kind = match sp.play {
                            SheetKind::Run(r) => PlayKind::Run(r),
                            SheetKind::Pass(k) => PlayKind::Pass(k as usize),
                            SheetKind::Design(k) => {
                                let level = t.install_level(InstallKey::Design(k));
                                if level < 0.35 {
                                    return None;
                                }
                                PlayKind::Pass(*t.design_ids.get(k as usize)?)
                            }
                        };
                        Some((sp.personnel, sp.backfield, sp.set, kind))
                    })
                    .collect(),
            )
        })
        .collect();
    prep.stick = if t.plan.sheet.is_empty() { 0.0 } else { t.plan.stick.clamp(0.0, 1.0) as f64 };
    // Freshness from the last practice week.
    if let Some(l) = t.log.last() {
        let gd = l.week.days.iter().position(|x| x.game).unwrap_or(6);
        let late = super::work::late_load(&l.week, gd, Role::Starter);
        let tier = match w.seat.map(|s| s.level) {
            Some(Cal::Nfl | Cal::Other) => Tier::Pro,
            Some(Cal::HighSchool) => Tier::High,
            _ => Tier::College,
        };
        let neutral_wk = week_from(
            Slot::Game,
            l.monday,
            &[l.monday.add(gd as i32)],
            &neutral(tier, Slot::Game),
            &neutral(tier, Slot::Bye),
        );
        let base = super::work::late_load(&neutral_wk, gd, Role::Starter);
        let fresh = super::work::freshness(late, base);
        prep.fresh = fresh;
        // The install: this week's prep against an average staff's week.
        let mine = super::work::prep_minutes(&l.week);
        let theirs = super::work::prep_minutes(&neutral_wk);
        if theirs > 0.0 {
            team.staff.install = clamp(base_install * (mine / theirs).sqrt().clamp(0.75, 1.15), 0.3, 1.0);
        }
        let (noise, rules) = week_habits(&l.week, &neutral_wk);
        prep.noise = noise;
        prep.rules = rules;
    }
    // Your guidelines and the deeper game plan (update 2, lane 5).
    crate::world::guide::seat_apply(w, team);
    crate::world::plans::prepare(w, team);
}

/// What a week's crowd-noise periods and rules clinic send into the game
/// against an average staff's week (TR28, TR29; proposed, not confirmed):
/// each loud day short of the usual lets a road crowd get to the offense
/// 7.5% more, each extra one 7.5% less (0.9 to 1.15); the officials' clinic
/// takes up to a tenth off the pre-snap fouls, and skipping it adds a
/// little.
pub fn week_habits(wk: &Week, neutral: &Week) -> (f64, f64) {
    let loud = |w: &Week| w.days.iter().filter(|d| d.periods.iter().any(|p| p.noise)).count() as f64;
    let clinic = |w: &Week| {
        w.days
            .iter()
            .flat_map(|d| &d.periods)
            .filter(|p| p.kind == Kind::RulesClinic)
            .map(|p| p.min as f64)
            .sum::<f64>()
    };
    let noise = clamp(1.0 + 0.075 * (loud(neutral) - loud(wk)), 0.9, 1.15);
    let (a, b) = (clinic(wk), clinic(neutral));
    let rules = clamp(1.0 - 0.1 * (a - b) / (b + 10.0), 0.9, 1.08);
    (noise, rules)
}

impl World {
    fn design_ids_stale(&self) -> bool {
        self.training.design_ids.len() != self.training.designs.len()
    }

    /// Your college team kept for the season takes your latest depth
    /// chart, packages and plan (after you change them).
    pub fn refresh_seat_team(&mut self) {
        let Some(c) = self.seat.filter(|s| s.level == Cal::College).and_then(|s| s.program) else { return };
        if self.training.design_ids.len() != self.training.designs.len() {
            self.training.register_designs();
        }
        let order = self.seat_order();
        let mut team = match self.college.as_mut().and_then(|cs| cs.teams.get_mut(c)).and_then(|t| t.take()) {
            Some(t) => t,
            None => return,
        };
        team.order = order;
        prepare(self, &mut team);
        if let Some(slot) = self.college.as_mut().and_then(|cs| cs.teams.get_mut(c)) {
            *slot = Some(team);
        }
    }

    /// The default plan your position coach would set (his read of the
    /// player's weakest skill that matters at his position).
    pub fn staff_focus(&self, id: u32) -> Option<Attr> {
        let p = self.people.iter().find(|p| p.player.id == id)?;
        let pos = p.player.position;
        let mut best: Option<(Attr, f64)> = None;
        for a in Attr::ALL {
            let wgt = crate::positions::weight(pos, a);
            if wgt < 0.04 {
                continue;
            }
            let score = -p.player.attributes.get(a) + 200.0 * wgt;
            if best.is_none_or(|b| score > b.1) {
                best = Some((a, score));
            }
        }
        best.map(|b| b.0)
    }

    /// The football year's frames (for the screens).
    pub fn training_frames(&self) -> Frames {
        let y = self.dates.clone();
        Frames::new(self, &y)
    }

    /// Your team as training sees it.
    pub fn training_team(&self) -> Option<SeatTeam> {
        seat_team(self, &self.training_frames())
    }
}

/// Slots each level keeps a template for.
pub fn slots_for(tier: Tier) -> Vec<Slot> {
    match tier {
        Tier::Pro => vec![Slot::Game, Slot::Bye, Slot::Camp, Slot::Phase1, Slot::Phase2, Slot::Otas],
        Tier::College => vec![Slot::Game, Slot::Bye, Slot::Camp, Slot::Spring, Slot::Winter, Slot::Summer, Slot::Bowl],
        Tier::High => vec![Slot::Game, Slot::Bye, Slot::Camp, Slot::Spring, Slot::Winter, Slot::Summer],
    }
}

/// The staff's default template for a slot (the AI's, by your style).
pub fn default_template(st: &SeatTeam, slot: Slot) -> Template {
    styled(st.tier, slot, &st.style)
}

/// The block counts against the block's limits, in words.
pub fn counts_text(st: &SeatTeam, block: Block, c: &Counts) -> Vec<String> {
    let lim = block_limits(st.tier, block, st.frame.season);
    let mut out = Vec::new();
    let mut add = |used: u16, max: u16, what: &str| {
        if max != u16::MAX {
            out.push(format!("{used} of {max} {what}"));
        }
    };
    add(c.practices, lim.practices, "practices");
    add(c.contact, lim.contact, "contact practices");
    add(c.padded, lim.padded, "padded practices");
    add(c.scrimmages, lim.scrimmages, "scrimmages");
    out
}
