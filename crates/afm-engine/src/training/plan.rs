//! The practice day and the week (topics 1, 2 and 11): a practice is a
//! list of periods, each with its length, kind, who is in it, gear,
//! contact and tempo; a week is seven days made from templates by day
//! type, the game week counted from the game so the same template follows
//! a Saturday game or a Thursday one. AI staffs build theirs from their
//! head coach's styles under the same rules (`fit`); your staff's default
//! is the same AI.

use super::rules::{Block, Book, Tier};
use crate::calendar::{Date, Weekday};
use crate::tendencies::Style;

/// What a period is (topic 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum Kind {
    WarmUp,
    /// Each position group with its coach: technique drills.
    Individual,
    /// The run game against the front seven.
    InsideRun,
    /// The passing game: quarterbacks, receivers, backs and the back seven
    /// (the line works pass protection and rush at the same time).
    SevenOnSeven,
    /// Receivers against corners, linemen against rushers.
    OneOnOnes,
    /// Eleven on eleven.
    Team,
    /// A game-like live period.
    Scrimmage,
    /// Third down, red zone, two-minute, goal line, backed up, four-minute.
    Situations,
    SpecialTeams,
    Walkthrough,
    Conditioning,
    /// Speed and agility work.
    Speed,
    WeightRoom,
    Meeting,
    Film,
    /// The young players' developmental period.
    Young,
    /// Treatment, rehab and recovery: not countable.
    Recovery,
    // The training week (update 2, lane 4: TR5-TR35). New kinds go at the
    // end so older saves read the same.
    /// Early-morning team competitions on the mats (TR6).
    MatDrills,
    /// The strength staff's testing: lifts, the 40, the shuttle, the jump (TR5).
    Testing,
    /// The week's new plays on the board before they're repped (TR17).
    Install,
    /// Flagged players at the academic center: academic, not countable (TR14).
    StudyHall,
    /// Quarterbacks and receivers with no defense (TR20); `what` 1 is the
    /// players' own session with no staff.
    OnAir,
    /// Catching and ball security stations: the JUGS machine, tennis
    /// balls, the gauntlet (TR21).
    BallSecurity,
    /// Snap-hold-kick, punt catching, directional punting (TR22).
    KickStations,
    /// Mobility, yoga and pliability (TR23).
    Mobility,
    /// Vision and reaction training: strobe glasses, light boards (TR25).
    Vision,
    /// Virtual reality reps for quarterbacks (TR26).
    VrReps,
    /// Mental skills with a sports psychologist (TR27).
    MentalSkills,
    /// Officials at practice on the year's rules (TR29).
    RulesClinic,
    /// A written quiz on the game plan (TR30).
    PlanTest,
    /// Nutrition with the dietitian (TR31).
    Nutrition,
    /// The leadership council and player-led meetings (TR32).
    Council,
    /// Away from the field: community service, team dinners (TR33).
    TeamBuilding,
    /// The secondary's calls, the line's protection calls, the sideline
    /// signals (TR34).
    Signals,
    /// Home study on the tablet or the playbook (TR35).
    HomeStudy,
}

pub const KINDS: usize = 35;

impl Kind {
    pub const ALL: [Kind; KINDS] = [
        Kind::WarmUp,
        Kind::Individual,
        Kind::InsideRun,
        Kind::SevenOnSeven,
        Kind::OneOnOnes,
        Kind::Team,
        Kind::Scrimmage,
        Kind::Situations,
        Kind::SpecialTeams,
        Kind::Walkthrough,
        Kind::Conditioning,
        Kind::Speed,
        Kind::WeightRoom,
        Kind::Meeting,
        Kind::Film,
        Kind::Young,
        Kind::Recovery,
        Kind::MatDrills,
        Kind::Testing,
        Kind::Install,
        Kind::StudyHall,
        Kind::OnAir,
        Kind::BallSecurity,
        Kind::KickStations,
        Kind::Mobility,
        Kind::Vision,
        Kind::VrReps,
        Kind::MentalSkills,
        Kind::RulesClinic,
        Kind::PlanTest,
        Kind::Nutrition,
        Kind::Council,
        Kind::TeamBuilding,
        Kind::Signals,
        Kind::HomeStudy,
    ];

    pub fn name(self) -> &'static str {
        [
            "warmup",
            "individual",
            "inside_run",
            "seven_on_seven",
            "one_on_ones",
            "team",
            "scrimmage",
            "situations",
            "special_teams",
            "walkthrough",
            "conditioning",
            "speed",
            "weight_room",
            "meeting",
            "film",
            "young",
            "recovery",
            "mat_drills",
            "testing",
            "install",
            "study_hall",
            "on_air",
            "ball_security",
            "kick_stations",
            "mobility",
            "vision",
            "vr_reps",
            "mental_skills",
            "rules_clinic",
            "plan_test",
            "nutrition",
            "council",
            "team_building",
            "signals",
            "home_study",
        ][self as usize]
    }

    pub fn label(self) -> &'static str {
        [
            "Warm-up",
            "Individual",
            "Inside run",
            "7-on-7",
            "One-on-ones",
            "Team",
            "Scrimmage",
            "Situations",
            "Special teams",
            "Walkthrough",
            "Conditioning",
            "Speed and agility",
            "Weight room",
            "Meetings",
            "Film",
            "Young players",
            "Treatment and recovery",
            "Mat drills",
            "Testing",
            "Install",
            "Study hall",
            "Routes on air",
            "Ball security",
            "Kicking stations",
            "Mobility and yoga",
            "Vision and reaction",
            "VR reps",
            "Mental skills",
            "Rules clinic",
            "Game plan test",
            "Nutrition",
            "Leadership council",
            "Team building",
            "Signals and communication",
            "Home study",
        ][self as usize]
    }

    pub fn parse(s: &str) -> Option<Kind> {
        Kind::ALL.iter().copied().find(|k| k.name() == s)
    }

    /// The colour group the planner shows it in.
    pub fn group(self) -> &'static str {
        match self {
            Kind::WarmUp
            | Kind::Conditioning
            | Kind::Speed
            | Kind::WeightRoom
            | Kind::Recovery
            | Kind::MatDrills
            | Kind::Testing
            | Kind::Mobility
            | Kind::Nutrition => "body",
            Kind::Individual | Kind::OneOnOnes | Kind::Young | Kind::OnAir | Kind::BallSecurity => "technique",
            Kind::InsideRun | Kind::SevenOnSeven | Kind::Team | Kind::Scrimmage => "team",
            Kind::Situations | Kind::Walkthrough => "situations",
            Kind::SpecialTeams | Kind::KickStations => "special",
            Kind::Meeting
            | Kind::Film
            | Kind::Install
            | Kind::Vision
            | Kind::VrReps
            | Kind::MentalSkills
            | Kind::RulesClinic
            | Kind::PlanTest
            | Kind::Signals
            | Kind::HomeStudy
            | Kind::StudyHall => "classroom",
            Kind::Council | Kind::TeamBuilding => "people",
        }
    }

    /// The family a screen files it under (the training week's nine: the
    /// seven that grow, recovery and the team), by its contact for the
    /// periods that can go either way.
    pub fn shown_family(self, contact: Contact) -> Option<&'static str> {
        Some(match self {
            Kind::WeightRoom | Kind::Testing | Kind::Nutrition => "Strength",
            Kind::Speed | Kind::WarmUp => "Speed and movement",
            Kind::Conditioning | Kind::MatDrills => "Conditioning",
            Kind::InsideRun | Kind::Team | Kind::Scrimmage | Kind::OneOnOnes | Kind::Situations | Kind::Young
                if contact.full() =>
            {
                "Contact"
            }
            Kind::Individual
            | Kind::OneOnOnes
            | Kind::Young
            | Kind::InsideRun
            | Kind::SevenOnSeven
            | Kind::Team
            | Kind::Scrimmage
            | Kind::Situations
            | Kind::OnAir
            | Kind::BallSecurity => "Technique",
            Kind::SpecialTeams | Kind::KickStations => "Kicking",
            Kind::Meeting
            | Kind::Film
            | Kind::Walkthrough
            | Kind::Install
            | Kind::Vision
            | Kind::VrReps
            | Kind::MentalSkills
            | Kind::RulesClinic
            | Kind::PlanTest
            | Kind::Signals
            | Kind::HomeStudy => "Mental",
            Kind::Recovery | Kind::Mobility => "Recovery",
            Kind::Council | Kind::TeamBuilding => "Team",
            Kind::StudyHall => return None,
        })
    }

    /// On the practice field (with coaches and a ball).
    pub fn on_field(self) -> bool {
        !matches!(
            self,
            Kind::WeightRoom
                | Kind::Meeting
                | Kind::Film
                | Kind::Recovery
                | Kind::MatDrills
                | Kind::Testing
                | Kind::Install
                | Kind::StudyHall
                | Kind::Mobility
                | Kind::Vision
                | Kind::VrReps
                | Kind::MentalSkills
                | Kind::RulesClinic
                | Kind::PlanTest
                | Kind::Nutrition
                | Kind::Council
                | Kind::TeamBuilding
                | Kind::Signals
                | Kind::HomeStudy
        )
    }

    /// The strength staff's own work: allowed when football isn't (the
    /// NCAA's winter, the NFL's phase 1).
    pub fn strength_staff(self) -> bool {
        matches!(self, Kind::Conditioning | Kind::Speed | Kind::MatDrills | Kind::Testing | Kind::Mobility)
    }

    /// A practice in the rule books' count: a ball and coaches on the
    /// field, not a conditioning or speed session.
    pub fn practice(self) -> bool {
        self.on_field() && !matches!(self, Kind::Conditioning | Kind::Speed | Kind::WarmUp)
    }

    /// Counted against the rules (the NCAA's countable athletically related
    /// activities): not treatment, academics, the dietitian, voluntary
    /// meetings and outings, a psychologist's opt-in sessions or the
    /// players' own study (proposed, not confirmed: how the NCAA's
    /// countable hours read each).
    pub fn counted(self) -> bool {
        !matches!(
            self,
            Kind::Recovery
                | Kind::StudyHall
                | Kind::Nutrition
                | Kind::Council
                | Kind::TeamBuilding
                | Kind::MentalSkills
                | Kind::HomeStudy
        )
    }

    /// Offense against defense.
    pub fn offense_against_defense(self) -> bool {
        matches!(
            self,
            Kind::InsideRun | Kind::SevenOnSeven | Kind::OneOnOnes | Kind::Team | Kind::Scrimmage | Kind::Situations
        )
    }

    /// Film and meetings in the rules' count (the NCAA's two hours out of
    /// season).
    pub fn film(self) -> bool {
        matches!(
            self,
            Kind::Meeting
                | Kind::Film
                | Kind::Walkthrough
                | Kind::Install
                | Kind::VrReps
                | Kind::RulesClinic
                | Kind::PlanTest
                | Kind::Signals
        )
    }

    /// Classroom, mind or body work that readies a player for the week's
    /// plan (TR36): meetings, installs, film, walkthroughs, tests.
    pub fn study(self) -> bool {
        matches!(
            self,
            Kind::Meeting
                | Kind::Film
                | Kind::Walkthrough
                | Kind::Install
                | Kind::VrReps
                | Kind::PlanTest
                | Kind::Signals
                | Kind::HomeStudy
        )
    }

    /// Practice snaps a minute at normal tempo (a rep every 30-45 seconds
    /// in team periods, faster in one-on-ones).
    pub fn snaps_per_min(self) -> f64 {
        match self {
            Kind::InsideRun | Kind::Team | Kind::Situations => 1.5,
            Kind::SevenOnSeven => 1.8,
            Kind::OneOnOnes => 2.5,
            Kind::Scrimmage => 1.1,
            Kind::Young => 1.2,
            _ => 0.0,
        }
    }
}

/// Gear (topic 1): what is worn sets what contact is possible.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum Gear {
    None,
    Helmets,
    Shells,
    Pads,
}

impl Gear {
    pub const ALL: [Gear; 4] = [Gear::None, Gear::Helmets, Gear::Shells, Gear::Pads];
    pub fn name(self) -> &'static str {
        ["none", "helmets", "shells", "pads"][self as usize]
    }
    pub fn label(self) -> &'static str {
        ["No gear", "Helmets", "Shells", "Full pads"][self as usize]
    }
    pub fn parse(s: &str) -> Option<Gear> {
        Gear::ALL.iter().copied().find(|g| g.name() == s)
    }
    /// The most contact this gear allows.
    pub fn most_contact(self) -> Contact {
        match self {
            Gear::None | Gear::Helmets => Contact::Bags,
            Gear::Shells => Contact::Thud,
            Gear::Pads => Contact::Live,
        }
    }
}

/// Contact levels, in USA Football's and the NFL's words.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum Contact {
    Air,
    Bags,
    Control,
    Thud,
    Live,
}

impl Contact {
    pub const ALL: [Contact; 5] = [Contact::Air, Contact::Bags, Contact::Control, Contact::Thud, Contact::Live];
    pub fn name(self) -> &'static str {
        ["air", "bags", "control", "thud", "live"][self as usize]
    }
    pub fn label(self) -> &'static str {
        ["Air", "Bags", "Control", "Thud", "Live"][self as usize]
    }
    pub fn parse(s: &str) -> Option<Contact> {
        Contact::ALL.iter().copied().find(|c| c.name() == s)
    }
    /// Full contact: thud and live.
    pub fn full(self) -> bool {
        self >= Contact::Thud
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum Tempo {
    Walk,
    Normal,
    Fast,
}

impl Tempo {
    pub const ALL: [Tempo; 3] = [Tempo::Walk, Tempo::Normal, Tempo::Fast];
    pub fn name(self) -> &'static str {
        ["walk", "normal", "fast"][self as usize]
    }
    pub fn parse(s: &str) -> Option<Tempo> {
        Tempo::ALL.iter().copied().find(|t| t.name() == s)
    }
}

/// Who is in a period.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum Who {
    /// Everyone, each at his own station.
    All,
    /// The two-deep: the starters take most reps, the twos the rest, the
    /// others run the scout team.
    Ones,
    /// The players outside the two-deep.
    Young,
}

impl Who {
    pub const ALL: [Who; 3] = [Who::All, Who::Ones, Who::Young];
    pub fn name(self) -> &'static str {
        ["all", "ones", "young"][self as usize]
    }
    pub fn parse(s: &str) -> Option<Who> {
        Who::ALL.iter().copied().find(|t| t.name() == s)
    }
}

/// What a team or situation period works on.
pub const SITUATIONS: [&str; 7] =
    ["Base", "Third down", "Red zone", "Two-minute", "Goal line", "Backed up", "Four-minute"];

/// One period (7 bytes; memory gate).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Period {
    pub kind: Kind,
    pub min: u8,
    pub gear: Gear,
    pub contact: Contact,
    pub tempo: Tempo,
    pub who: Who,
    /// The situation for team and situation periods (`SITUATIONS`); for a
    /// team period 0 is base work and 1+ this week's install against the
    /// opponent's looks. For the kinds with sub-kinds it is the sub-kind
    /// (`Period::name`): which meeting, which film, which walkthrough,
    /// which install, which outing; routes on air 1 is the players' own.
    #[serde(default)]
    pub what: u8,
    /// Crowd noise on (TR28): speakers before a loud road game.
    #[serde(default)]
    pub noise: bool,
}

impl Period {
    pub fn new(kind: Kind, min: u8, gear: Gear, contact: Contact) -> Period {
        let who = match kind {
            Kind::Team | Kind::Scrimmage | Kind::Situations | Kind::InsideRun | Kind::SevenOnSeven => Who::Ones,
            Kind::Young => Who::Young,
            _ => Who::All,
        };
        let tempo = if kind == Kind::Walkthrough { Tempo::Walk } else { Tempo::Normal };
        Period { kind, min, gear, contact, tempo, who, what: 0, noise: false }
    }

    /// The players' own session with no staff (routes on air, `what` 1):
    /// not counted, allowed when the staff may run nothing.
    pub fn own(&self) -> bool {
        self.kind == Kind::OnAir && self.what == 1
    }

    /// Counted against the rules' hours (the NCAA's countable activities).
    pub fn counted(&self) -> bool {
        self.kind.counted() && !self.own()
    }

    /// A practice in the rule books' count.
    pub fn practice(&self) -> bool {
        self.kind.practice() && !self.own()
    }

    /// What happens when the staff may run nothing: the players' own
    /// sessions and study hall (academic) stay.
    pub fn players_own(&self) -> bool {
        self.own() || self.kind == Kind::StudyHall
    }

    /// The period's name on a screen, with its sub-kind.
    pub fn name(&self) -> String {
        let pick = |l: &[&str]| l[(self.what as usize).min(l.len() - 1)].to_string();
        match self.kind {
            Kind::Meeting => pick(&MEETINGS),
            Kind::Film => pick(&FILMS),
            Kind::Walkthrough => pick(&WALKS),
            Kind::Install => pick(&INSTALLS),
            Kind::TeamBuilding => pick(&OUTINGS),
            Kind::Recovery => pick(&RECOVERY),
            Kind::OnAir if self.own() => "Throwing on air (players' own)".to_string(),
            Kind::WeightRoom if self.what == 1 => "Weight room (in season)".to_string(),
            _ => self.kind.label().to_string(),
        }
    }

    fn on(mut self, gear: Gear, contact: Contact) -> Period {
        self.gear = gear;
        self.contact = contact;
        self
    }

    fn w(mut self, what: u8) -> Period {
        self.what = what;
        self
    }

    fn loud(mut self) -> Period {
        self.noise = true;
        self
    }

    fn at(mut self, t: Tempo) -> Period {
        self.tempo = t;
        self
    }
}

/// The sub-kinds of the kinds that have them (`Period::what`), for the
/// screens: meetings (TR16), film (TR18), walkthroughs (TR19), installs
/// (TR17), outings (TR33) and recovery (TR7, TR24).
pub const MEETINGS: [&str; 3] = ["Team meeting", "Unit meetings", "Position meetings"];
pub const FILMS: [&str; 6] = [
    "Film: the opponent",
    "Film: self-scout",
    "Film: practice tape",
    "Film: situation cut-ups",
    "Film: the new system",
    "Film: the game, graded",
];
pub const WALKS: [&str; 3] = ["Walkthrough: jog-through", "Mock game", "Hotel walkthrough"];
pub const INSTALLS: [&str; 5] = [
    "Install: base downs",
    "Install: third down",
    "Install: red zone and goal line",
    "Install: two-minute and situations",
    "Install: the new system",
];
pub const OUTINGS: [&str; 4] = ["Team dinner", "Community service", "Pasta dinner", "Team outing"];
pub const RECOVERY: [&str; 2] = ["Treatment and recovery", "Rehab group"];

impl Kind {
    /// The sub-kinds a screen offers for this kind (empty: none).
    pub fn whats(self) -> &'static [&'static str] {
        match self {
            Kind::Meeting => &MEETINGS,
            Kind::Film => &FILMS,
            Kind::Walkthrough => &WALKS,
            Kind::Install => &INSTALLS,
            Kind::TeamBuilding => &OUTINGS,
            Kind::Recovery => &RECOVERY,
            Kind::OnAir => &["With the staff", "The players' own"],
            Kind::WeightRoom => &["Weight room", "In season"],
            Kind::Team | Kind::Situations => &SITUATIONS,
            _ => &[],
        }
    }
}

/// Whether a kind of period exists at a level in an era (proposed, not
/// confirmed): virtual reality headsets from 2015 and vision boards from
/// 2010, at FBS programs and NFL clubs; a sports psychologist's sessions
/// from 2005 at the top, 2015 elsewhere in college, not at a high school;
/// a dietitian from 2000 in the NFL, 2005 at the top of college, 2015
/// elsewhere, a high school's visiting dietitian from 2010; mobility and
/// yoga from 2000 in the NFL, 2005 in college, 2012 at a high school.
pub fn available(kind: Kind, tier: Tier, season: i32, big: bool) -> bool {
    match kind {
        Kind::VrReps => big && tier != Tier::High && season >= 2015,
        Kind::Vision => big && tier != Tier::High && season >= 2010,
        Kind::MentalSkills => match tier {
            Tier::Pro => season >= 2005,
            Tier::College => season >= if big { 2005 } else { 2015 },
            Tier::High => false,
        },
        Kind::Nutrition => {
            season
                >= match tier {
                    Tier::Pro => 2000,
                    Tier::College if big => 2005,
                    Tier::College => 2015,
                    Tier::High => 2010,
                }
        }
        Kind::Mobility => {
            season
                >= match tier {
                    Tier::Pro => 2000,
                    Tier::College => 2005,
                    Tier::High => 2012,
                }
        }
        _ => true,
    }
}

/// One day: its periods, and whether it is a game day or a second
/// practice (two-a-days).
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Day {
    pub periods: Vec<Period>,
    #[serde(default)]
    pub game: bool,
    #[serde(default)]
    pub second: bool,
}

impl Day {
    /// A day with nothing required (treatment, academics, voluntary
    /// outings and the players' own sessions may still happen).
    pub fn rest(&self) -> bool {
        !self.game && self.periods.iter().all(|p| !p.counted())
    }

    /// NCAA countable minutes: everything counted (`Period::counted`); a
    /// game day counts three hours flat, its walkthrough and meetings
    /// inside them.
    pub fn countable(&self) -> u32 {
        if self.game {
            return 180;
        }
        self.periods.iter().filter(|p| p.counted()).map(|p| p.min as u32).sum::<u32>()
    }

    /// Minutes on the practice field.
    pub fn field(&self) -> u32 {
        self.periods.iter().filter(|p| p.kind.on_field()).map(|p| p.min as u32).sum()
    }

    pub fn minutes(&self) -> u32 {
        self.periods.iter().map(|p| p.min as u32).sum()
    }

    pub fn padded(&self) -> bool {
        self.periods.iter().any(|p| p.gear == Gear::Pads)
    }
}

/// A week, Monday first.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Week {
    pub days: [Day; 7],
}

/// The kinds of week a staff keeps a template for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum Slot {
    /// A game week, its days counted back from the game (`GameWeek`).
    Game,
    /// A week with no game in season.
    Bye,
    Camp,
    Spring,
    Winter,
    Summer,
    /// Bowl practice.
    Bowl,
    Phase1,
    Phase2,
    Otas,
    /// The players' own time: nothing the staff may require.
    Own,
}

pub const SLOTS: usize = 11;

impl Slot {
    pub const ALL: [Slot; SLOTS] = [
        Slot::Game,
        Slot::Bye,
        Slot::Camp,
        Slot::Spring,
        Slot::Winter,
        Slot::Summer,
        Slot::Bowl,
        Slot::Phase1,
        Slot::Phase2,
        Slot::Otas,
        Slot::Own,
    ];
    pub fn name(self) -> &'static str {
        ["game", "bye", "camp", "spring", "winter", "summer", "bowl", "phase1", "phase2", "otas", "own"][self as usize]
    }
    pub fn label(self) -> &'static str {
        [
            "Game week",
            "Bye week",
            "Camp week",
            "Spring week",
            "Offseason week",
            "Summer week",
            "Bowl practice week",
            "Phase 1 week",
            "Phase 2 week",
            "OTA week",
            "Players' own time",
        ][self as usize]
    }
    pub fn parse(s: &str) -> Option<Slot> {
        Slot::ALL.iter().copied().find(|t| t.name() == s)
    }

    /// The template a block uses (a season week with no game is a bye).
    pub fn of(b: Block, game_week: bool, bowl: bool) -> Slot {
        match b {
            Block::Season if game_week => Slot::Game,
            Block::Season => Slot::Bye,
            Block::Camp => Slot::Camp,
            Block::Spring => Slot::Spring,
            Block::Winter => Slot::Winter,
            Block::Summer => Slot::Summer,
            Block::Postseason if bowl => Slot::Bowl,
            // No bowl: December is finals and the players' own lifting.
            Block::Postseason => Slot::Own,
            Block::Phase1 => Slot::Phase1,
            Block::Phase2 => Slot::Phase2,
            Block::Otas => Slot::Otas,
            Block::Quiet | Block::Break => Slot::Own,
        }
    }
}

/// A template: seven days. For a game week, day 0 is six days before the
/// game and day 6 the game; for every other slot, Monday to Sunday.
pub type Template = [Day; 7];

fn d(ps: Vec<Period>) -> Day {
    Day { periods: ps, game: false, second: false }
}

fn off() -> Day {
    Day::default()
}

/// A game day with its own periods (the hotel walkthrough, the captains'
/// meeting): inside the game day's three hours.
fn game_day(ps: Vec<Period>) -> Day {
    Day { periods: ps, game: true, second: false }
}

/// A period with no gear and no contact.
fn p(k: Kind, min: u8) -> Period {
    Period::new(k, min, Gear::None, Contact::Air)
}

use Contact::{Air, Bags, Control, Live, Thud};
use Gear::{Helmets, Pads, Shells};
use Kind::*;

/// The neutral staff's template for a slot at a level: what an average
/// staff of the era runs (proposed, not confirmed: built from the real
/// weeks of calendar topic 6, the period lengths of real practice scripts
/// and update 2's training week: mat drills, testing, installs in three
/// pieces, film by type, walkthroughs, stations, recovery, study hall,
/// meetings by room, the players' own throwing). Kinds an era or level
/// doesn't have drop out in `fit` (`available`).
pub fn neutral(tier: Tier, slot: Slot) -> Template {
    match (tier, slot) {
        (Tier::College, Slot::Own) => {
            [off(), d(vec![p(OnAir, 60).w(1)]), off(), d(vec![p(OnAir, 60).w(1)]), off(), off(), off()]
        }
        (_, Slot::Own) => Default::default(),
        (Tier::College, Slot::Game) => [
            // Sunday after the game: treatment, a recovery lift, the game
            // graded on film, the young players' practice, the weigh-in.
            d(vec![
                p(Recovery, 30),
                p(WeightRoom, 30).w(1),
                p(Film, 45).w(5),
                p(Mobility, 20),
                p(Young, 50).on(Shells, Control),
                p(Nutrition, 10),
            ]),
            // Monday: the required day off; treatment, and the plan on the
            // tablets in the evening.
            d(vec![p(Recovery, 60), p(HomeStudy, 30)]),
            // Tuesday: base downs installed, then a padded practice.
            d(vec![
                p(WeightRoom, 30).w(1),
                p(Meeting, 10),
                p(Install, 30),
                p(Meeting, 30).w(2),
                p(Walkthrough, 15).on(Helmets, Air),
                p(WarmUp, 10).on(Pads, Air),
                p(KickStations, 15).on(Pads, Air),
                p(Individual, 15).on(Pads, Bags),
                p(BallSecurity, 10).on(Pads, Bags),
                p(InsideRun, 15).on(Pads, Thud),
                p(OneOnOnes, 10).on(Pads, Thud),
                p(SevenOnSeven, 15).on(Pads, Air).w(1),
                p(Team, 25).on(Pads, Thud).w(1).loud(),
                p(Conditioning, 10).on(Pads, Air),
                p(Recovery, 20),
            ]),
            // Wednesday: third down.
            d(vec![
                p(WeightRoom, 25).w(1),
                p(Install, 30).w(1),
                p(Film, 30),
                p(Signals, 15),
                p(WarmUp, 10).on(Pads, Air),
                p(KickStations, 15).on(Pads, Air),
                p(Individual, 20).on(Pads, Bags),
                p(Situations, 25).on(Pads, Thud).w(1).loud(),
                p(SevenOnSeven, 15).on(Pads, Air).w(1),
                p(Team, 20).on(Pads, Control).w(1).loud(),
                p(Recovery, 20),
            ]),
            // Thursday: red zone, the plan test, the mock game in shells.
            d(vec![
                p(Install, 25).w(2),
                p(PlanTest, 15),
                p(Film, 20).w(2),
                p(RulesClinic, 10),
                p(WarmUp, 10).on(Shells, Air),
                p(KickStations, 15).on(Shells, Air),
                p(Individual, 15).on(Shells, Bags),
                p(Walkthrough, 35).on(Shells, Air).w(1).at(Tempo::Normal),
                p(Situations, 15).on(Shells, Control).w(3),
                p(MentalSkills, 15),
            ]),
            // Friday: meetings, cut-ups, the walkthrough, the team dinner.
            d(vec![
                p(Meeting, 30).w(2),
                p(Film, 20).w(3),
                p(Walkthrough, 20).on(Helmets, Air),
                p(VrReps, 15),
                p(TeamBuilding, 45),
                p(Meeting, 30).w(1),
            ]),
            game_day(vec![p(Walkthrough, 15).w(2), p(Council, 15)]),
        ],
        (Tier::College, Slot::Bye) | (Tier::College, Slot::Bowl) => [
            d(vec![p(WeightRoom, 45), p(Film, 30).w(1), p(Recovery, 30), p(Mobility, 20)]),
            d(vec![
                p(Meeting, 30).w(2),
                p(WarmUp, 15).on(Pads, Air),
                p(Individual, 30).on(Pads, Bags),
                p(BallSecurity, 10).on(Pads, Bags),
                p(OneOnOnes, 15).on(Pads, Thud),
                p(Young, 30).on(Pads, Thud),
                p(Team, 25).on(Pads, Thud),
                p(Conditioning, 15).on(Pads, Air),
                p(WeightRoom, 30),
            ]),
            d(vec![
                p(Meeting, 30).w(1),
                p(Film, 30).w(1),
                p(WarmUp, 15).on(Shells, Air),
                p(KickStations, 15).on(Shells, Air),
                p(Individual, 30).on(Shells, Bags),
                p(SevenOnSeven, 20).on(Shells, Air),
                p(Young, 30).on(Shells, Control),
                p(Situations, 20).on(Shells, Control),
            ]),
            d(vec![p(WeightRoom, 45), p(Film, 30).w(2), p(Council, 30), p(MentalSkills, 20)]),
            d(vec![
                p(Meeting, 30).w(2),
                p(WarmUp, 15).on(Pads, Air),
                p(Individual, 25).on(Pads, Bags),
                p(Young, 30).on(Pads, Thud),
                p(Team, 30).on(Pads, Thud),
                p(SpecialTeams, 15).on(Pads, Control),
                p(TeamBuilding, 60).w(3),
            ]),
            off(),
            off(),
        ],
        (Tier::College, Slot::Camp) => {
            let practice = |gear: Gear, c: Contact| {
                d(vec![
                    p(Install, 45),
                    p(Meeting, 30).w(2),
                    p(Film, 20).w(2),
                    p(WarmUp, 15).on(gear, Air),
                    p(KickStations, 10).on(gear, Air),
                    p(Individual, 25).on(gear, Bags),
                    p(SpecialTeams, 15).on(gear, Control),
                    p(InsideRun, 15).on(gear, c),
                    p(OneOnOnes, 15).on(gear, c),
                    p(SevenOnSeven, 15).on(gear, Air),
                    p(Team, 30).on(gear, c),
                    p(Situations, 15).on(gear, Control),
                    p(Conditioning, 15).on(gear, Air),
                    p(WeightRoom, 30),
                    p(Recovery, 20),
                ])
            };
            let mut clinic = practice(Shells, Control);
            clinic.periods.insert(3, p(RulesClinic, 30));
            let mut outing = practice(Shells, Control);
            outing.periods.push(p(TeamBuilding, 60).w(3));
            [
                practice(Shells, Control),
                practice(Pads, Thud),
                clinic,
                practice(Pads, Thud),
                outing,
                {
                    let mut x = practice(Pads, Thud);
                    x.periods.retain(|q| !matches!(q.kind, Team | InsideRun));
                    x.periods.insert(7, p(Scrimmage, 50).on(Pads, Live));
                    x
                },
                d(vec![p(Recovery, 30), p(Mobility, 20)]),
            ]
        }
        (Tier::College, Slot::Spring) => [
            d(vec![p(WeightRoom, 45), p(Film, 30).w(2), p(Mobility, 15)]),
            d(vec![
                p(Install, 30).w(4),
                p(Meeting, 30).w(2),
                p(Film, 15).w(2),
                p(WarmUp, 15).on(Shells, Air),
                p(KickStations, 10).on(Shells, Air),
                p(Individual, 25).on(Shells, Bags),
                p(OneOnOnes, 15).on(Shells, Control),
                p(SevenOnSeven, 15).on(Shells, Air),
                p(Team, 30).on(Shells, Control),
                p(Conditioning, 10).on(Shells, Air),
            ]),
            d(vec![p(WeightRoom, 45), p(Film, 30).w(2), p(StudyHall, 90)]),
            d(vec![
                p(Install, 30).w(4),
                p(Meeting, 30).w(2),
                p(WarmUp, 15).on(Pads, Air),
                p(Individual, 25).on(Pads, Bags),
                p(BallSecurity, 10).on(Pads, Bags),
                p(InsideRun, 15).on(Pads, Thud),
                p(OneOnOnes, 15).on(Pads, Thud),
                p(SevenOnSeven, 15).on(Pads, Air),
                p(Team, 30).on(Pads, Thud),
                p(Situations, 15).on(Pads, Control).w(1),
            ]),
            d(vec![p(WeightRoom, 45), p(Film, 30).w(2), p(Council, 30)]),
            d(vec![
                p(Meeting, 30).w(1),
                p(WarmUp, 15).on(Pads, Air),
                p(Individual, 15).on(Pads, Bags),
                p(Scrimmage, 60).on(Pads, Live),
                p(SpecialTeams, 10).on(Pads, Control),
                p(Recovery, 20),
            ]),
            off(),
        ],
        (Tier::College, Slot::Winter) => [
            // Winter conditioning (TR6): mat drills twice a week at 5:45, the
            // lift groups, film of the system, study hall at night.
            d(vec![p(MatDrills, 40), p(WeightRoom, 55), p(Meeting, 15), p(Film, 30).w(4), p(StudyHall, 90)]),
            d(vec![
                p(WeightRoom, 55),
                p(Speed, 25),
                p(Mobility, 15),
                p(Nutrition, 20),
                p(Recovery, 45).w(1),
                p(OnAir, 45).w(1),
            ]),
            d(vec![p(WeightRoom, 55), p(Film, 25).w(1), p(Council, 30), p(StudyHall, 90)]),
            d(vec![p(WeightRoom, 55), p(Conditioning, 25), p(Vision, 15), p(VrReps, 15), p(MentalSkills, 30)]),
            d(vec![p(MatDrills, 40), p(Recovery, 30), p(TeamBuilding, 90)]),
            d(vec![p(TeamBuilding, 120).w(1)]),
            off(),
        ],
        (Tier::College, Slot::Summer) => [
            d(vec![p(WeightRoom, 60), p(Speed, 30), p(Film, 30).w(4), p(StudyHall, 60)]),
            d(vec![p(WeightRoom, 60), p(Conditioning, 20), p(Mobility, 15), p(OnAir, 60).w(1)]),
            d(vec![p(WeightRoom, 60), p(Speed, 30), p(Film, 30).w(1), p(Nutrition, 20)]),
            d(vec![p(WeightRoom, 60), p(Conditioning, 20), p(OnAir, 60).w(1)]),
            d(vec![p(TeamBuilding, 90).w(1)]),
            off(),
            off(),
        ],
        (Tier::Pro, Slot::Game) => [
            // Monday: the game graded, a recovery lift.
            d(vec![p(Meeting, 20), p(Film, 60).w(5), p(WeightRoom, 40).w(1), p(Mobility, 20), p(Recovery, 45)]),
            // Tuesday: the players' day off; the plan reaches the tablets.
            d(vec![p(Recovery, 60), p(TeamBuilding, 120).w(1), p(HomeStudy, 60)]),
            // Wednesday: first and second down, the padded day.
            d(vec![
                p(Meeting, 15),
                p(Install, 45),
                p(Meeting, 40).w(2),
                p(Walkthrough, 20).on(Helmets, Air),
                p(KickStations, 15).on(Pads, Air),
                p(Individual, 20).on(Pads, Bags),
                p(BallSecurity, 10).on(Pads, Bags),
                p(Team, 35).on(Pads, Thud).w(1).loud(),
                p(SevenOnSeven, 15).on(Pads, Air).w(1),
                p(WeightRoom, 30),
                p(Recovery, 20),
            ]),
            // Thursday: third down.
            d(vec![
                p(Install, 40).w(1),
                p(Film, 40),
                p(Signals, 15),
                p(PlanTest, 15),
                p(WarmUp, 15).on(Shells, Air),
                p(Individual, 20).on(Shells, Bags),
                p(SpecialTeams, 15).on(Shells, Air),
                p(Situations, 30).on(Shells, Control).w(1).loud(),
                p(Team, 20).on(Shells, Control).w(1),
                p(WeightRoom, 20),
            ]),
            // Friday: red zone and two-minute.
            d(vec![
                p(Install, 30).w(2),
                p(MentalSkills, 20),
                p(Vision, 10),
                p(WarmUp, 10).on(Helmets, Air),
                p(Individual, 10).on(Helmets, Air),
                p(KickStations, 10).on(Helmets, Air),
                p(Situations, 30).on(Helmets, Air).w(2),
                p(Team, 15).on(Helmets, Air).w(1),
            ]),
            // Saturday: the mock game, the hotel meetings and walkthrough.
            d(vec![
                p(Walkthrough, 30).on(Helmets, Air).w(1),
                p(Meeting, 45).w(1),
                p(Walkthrough, 15).w(2),
                p(VrReps, 15),
            ]),
            game_day(vec![]),
        ],
        (Tier::Pro, Slot::Bye) | (Tier::Pro, Slot::Bowl) => [
            d(vec![p(Film, 60).w(1), p(Meeting, 30), p(WeightRoom, 30), p(Mobility, 20)]),
            d(vec![
                p(Meeting, 60).w(2),
                p(WarmUp, 15).on(Shells, Air),
                p(Individual, 30).on(Shells, Bags),
                p(Young, 30).on(Shells, Control),
                p(Team, 25).on(Shells, Control),
                p(WeightRoom, 30),
            ]),
            d(vec![p(Meeting, 45).w(1), p(Film, 30).w(1), p(Walkthrough, 30).on(Helmets, Air)]),
            off(),
            off(),
            off(),
            off(),
        ],
        (Tier::Pro, Slot::Camp) => {
            let practice = |gear: Gear, c: Contact| {
                d(vec![
                    p(Install, 45),
                    p(Meeting, 45).w(2),
                    p(Film, 45).w(2),
                    p(WarmUp, 15).on(gear, Air),
                    p(KickStations, 10).on(gear, Air),
                    p(Individual, 25).on(gear, Bags),
                    p(SpecialTeams, 15).on(gear, Control),
                    p(InsideRun, 15).on(gear, c),
                    p(OneOnOnes, 15).on(gear, c),
                    p(SevenOnSeven, 15).on(gear, Air),
                    p(Team, 30).on(gear, c),
                    p(Situations, 15).on(gear, Control),
                    p(Conditioning, 10).on(gear, Air),
                    p(WeightRoom, 30),
                    p(Recovery, 20),
                ])
            };
            let mut clinic = practice(Shells, Control);
            clinic.periods.insert(3, p(RulesClinic, 30));
            [
                practice(Pads, Thud),
                practice(Pads, Thud),
                clinic,
                practice(Pads, Thud),
                practice(Shells, Control),
                practice(Shells, Control),
                d(vec![p(Recovery, 30), p(Mobility, 20)]),
            ]
        }
        (Tier::Pro, Slot::Phase1) => [
            // Phase 1: the classroom, the weight room and rehab only.
            d(vec![p(Meeting, 30), p(Install, 60).w(4), p(WeightRoom, 60), p(Nutrition, 20), p(Recovery, 60).w(1)]),
            d(vec![
                p(Meeting, 60).w(1),
                p(WeightRoom, 60),
                p(Conditioning, 30),
                p(Vision, 15),
                p(VrReps, 30),
                p(Recovery, 30),
            ]),
            d(vec![p(Meeting, 60).w(2), p(WeightRoom, 60), p(Mobility, 30), p(MentalSkills, 30), p(Council, 20)]),
            d(vec![p(Install, 45).w(3), p(Signals, 20), p(WeightRoom, 60), p(Speed, 30), p(HomeStudy, 60)]),
            d(vec![p(TeamBuilding, 180).w(1)]),
            off(),
            off(),
        ],
        (Tier::Pro, Slot::Phase2) => {
            let x = d(vec![
                p(Install, 40).w(4),
                p(Meeting, 30).w(2),
                p(Film, 20).w(4),
                p(WarmUp, 10),
                p(Individual, 35),
                p(KickStations, 10),
                p(Walkthrough, 20),
                p(WeightRoom, 45),
                p(Conditioning, 15),
                p(Mobility, 15),
            ]);
            [x.clone(), x.clone(), off(), x.clone(), x, off(), off()]
        }
        (Tier::Pro, Slot::Otas) => {
            let x = d(vec![
                p(Install, 40),
                p(Meeting, 30).w(2),
                p(Film, 20).w(2),
                p(WarmUp, 10).on(Helmets, Air),
                p(Individual, 25).on(Helmets, Air),
                p(SevenOnSeven, 20).on(Helmets, Air),
                p(Team, 30).on(Helmets, Air),
                p(KickStations, 10).on(Helmets, Air),
                p(WeightRoom, 40),
                p(Recovery, 20),
            ]);
            [x.clone(), x.clone(), off(), x.clone(), x, off(), off()]
        }
        (Tier::Pro, Slot::Winter | Slot::Summer | Slot::Spring) => Default::default(),
        (Tier::High, Slot::Game) => [
            // Saturday after Friday's game: the film graded, a lift.
            d(vec![p(Film, 60).w(5), p(WeightRoom, 45).w(1), p(Mobility, 20), p(Recovery, 20)]),
            off(),
            // Monday: the opponent, base downs installed, helmets.
            d(vec![
                p(Film, 30),
                p(WeightRoom, 25).w(1),
                p(Install, 20),
                p(WarmUp, 10).on(Helmets, Air),
                p(Individual, 20).on(Helmets, Bags),
                p(KickStations, 10).on(Helmets, Air),
                p(SevenOnSeven, 20).on(Helmets, Air).w(1),
                p(Walkthrough, 15).on(Helmets, Air),
                p(Conditioning, 10).on(Helmets, Air),
            ]),
            // Tuesday: the padded day.
            d(vec![
                p(Film, 30),
                p(Meeting, 25).w(2),
                p(WarmUp, 10).on(Pads, Air),
                p(Individual, 20).on(Pads, Bags),
                p(BallSecurity, 10).on(Pads, Bags),
                p(InsideRun, 15).on(Pads, Thud),
                p(OneOnOnes, 10).on(Pads, Thud),
                p(Team, 25).on(Pads, Thud).w(1),
                p(SpecialTeams, 15).on(Pads, Air),
                p(Conditioning, 10).on(Pads, Air),
            ]),
            // Wednesday: third down and red zone, the quiz.
            d(vec![
                p(Install, 30).w(1),
                p(PlanTest, 15),
                p(KickStations, 15).on(Pads, Air),
                p(WarmUp, 10).on(Pads, Air),
                p(Individual, 15).on(Pads, Bags),
                p(SevenOnSeven, 20).on(Pads, Air).w(1),
                p(Team, 25).on(Pads, Thud).w(1),
                p(Situations, 10).on(Pads, Control).w(3),
                p(Conditioning, 10).on(Pads, Air),
            ]),
            // Thursday: cut-ups, the mock game in shells, the pasta dinner.
            d(vec![
                p(Film, 25).w(3),
                p(RulesClinic, 10),
                p(Walkthrough, 40).on(Shells, Air).w(1).at(Tempo::Normal),
                p(SpecialTeams, 15).on(Shells, Air),
                p(TeamBuilding, 60).w(2),
            ]),
            game_day(vec![p(Walkthrough, 25).on(Helmets, Air)]),
        ],
        (Tier::High, Slot::Bye) | (Tier::High, Slot::Bowl) => {
            let x = d(vec![
                p(WarmUp, 10).on(Shells, Air),
                p(Individual, 30).on(Shells, Bags),
                p(KickStations, 10).on(Shells, Air),
                p(Young, 30).on(Shells, Control),
                p(SevenOnSeven, 20).on(Shells, Air),
                p(Conditioning, 15).on(Shells, Air),
                p(WeightRoom, 30),
            ]);
            [x.clone(), x.clone(), x, d(vec![p(Film, 30).w(1), p(Council, 30)]), off(), off(), off()]
        }
        (Tier::High, Slot::Camp) => {
            let practice = |gear: Gear, c: Contact| {
                d(vec![
                    p(Install, 20),
                    p(Film, 15).w(2),
                    p(WarmUp, 15).on(gear, Air),
                    p(KickStations, 10).on(gear, Air),
                    p(Individual, 30).on(gear, Bags),
                    p(InsideRun, 15).on(gear, c),
                    p(OneOnOnes, 15).on(gear, c),
                    p(SevenOnSeven, 20).on(gear, Air),
                    p(Team, 30).on(gear, c),
                    p(SpecialTeams, 15).on(gear, Air),
                    p(Conditioning, 15).on(gear, Air),
                ])
            };
            let mut clinic = practice(Shells, Control);
            clinic.periods.insert(2, p(RulesClinic, 20));
            [
                practice(Pads, Thud),
                practice(Pads, Thud),
                clinic,
                practice(Pads, Thud),
                practice(Shells, Control),
                {
                    let mut x = practice(Pads, Thud);
                    x.periods.retain(|q| !matches!(q.kind, Team | InsideRun));
                    x.periods.insert(4, p(Scrimmage, 45).on(Pads, Live));
                    x.periods.push(p(TeamBuilding, 60).w(2));
                    x
                },
                off(),
            ]
        }
        (Tier::High, Slot::Spring) => {
            let x = d(vec![
                p(Install, 20).w(4),
                p(WarmUp, 10).on(Pads, Air),
                p(Individual, 25).on(Pads, Bags),
                p(InsideRun, 10).on(Pads, Thud),
                p(OneOnOnes, 10).on(Pads, Thud),
                p(SevenOnSeven, 15).on(Pads, Air),
                p(Team, 25).on(Pads, Thud),
                p(SpecialTeams, 10).on(Pads, Air),
                p(Conditioning, 15).on(Pads, Air),
            ]);
            [x.clone(), x.clone(), off(), x.clone(), x, off(), off()]
        }
        (Tier::High, Slot::Winter) => [
            // The athletics period (TR2): the lift, speed, mat drills on
            // Friday, study hall before the bell, the players' own throwing.
            d(vec![p(StudyHall, 30), p(WeightRoom, 55)]),
            d(vec![p(Speed, 30), p(Mobility, 25), p(OnAir, 45).w(1)]),
            d(vec![p(WeightRoom, 55), p(Nutrition, 30)]),
            d(vec![p(StudyHall, 30), p(Film, 25).w(1), p(Council, 30)]),
            d(vec![p(MatDrills, 40), p(HomeStudy, 15)]),
            d(vec![p(TeamBuilding, 120).w(1)]),
            off(),
        ],
        (Tier::High, Slot::Summer) => {
            let x = d(vec![p(WeightRoom, 60), p(Speed, 30), p(Conditioning, 20), p(SevenOnSeven, 30).on(Helmets, Air)]);
            let mut throws = x.clone();
            throws.periods.push(p(OnAir, 45));
            let mut kicks = x.clone();
            kicks.periods.push(p(KickStations, 20).on(Helmets, Air));
            let mut mob = x;
            mob.periods.push(p(Mobility, 15));
            [throws.clone(), kicks, throws, mob, d(vec![p(TeamBuilding, 60).w(3)]), off(), off()]
        }
        (_, _) => Default::default(),
    }
}

/// The week's dated extras on top of a template (update 2, lane 4): the
/// strength staff's testing days (TR5: college in the second week of the
/// winter term, a week before spring practice and the summer's last week;
/// the NFL's first program day and report day; a high school's first May
/// week and late July) replace the day's lift, and an NFL quarterback's
/// own throwing sessions fill the summer break (TR20).
pub fn extras(f: &super::rules::Frame, monday: Date, wk: &mut Week) {
    if let Some(k) = testing_day(f, monday) {
        let d = &mut wk.days[k];
        if !d.game {
            d.periods.retain(|q| q.kind != WeightRoom);
            d.periods.insert(0, p(Testing, 60));
        }
    }
    if f.tier == Tier::Pro && f.block(monday.add(2)) == super::rules::Block::Break {
        for k in [1, 3] {
            wk.days[k].periods.push(p(OnAir, 90).w(1));
        }
    }
}

/// The weekday (Monday 0) of a testing day in the week from `monday`.
pub fn testing_day(f: &super::rules::Frame, monday: Date) -> Option<usize> {
    let in_week = |d: Date| (d >= monday && d < monday.add(7)).then(|| monday.days_to(d) as usize);
    let first_monday = |d: Date| {
        let mut m = d;
        while weekday_index(m) != 0 {
            m = m.add(1);
        }
        m
    };
    let s = f.season;
    let days: Vec<Date> = match f.tier {
        Tier::College => {
            let mut v = vec![first_monday(f.winter.add(7)), first_monday(f.camp.add(-10))];
            if let Some((a, _)) = f.spring {
                v.push(first_monday(a.add(-12)));
            }
            v
        }
        Tier::Pro => vec![f.phase1, f.report],
        Tier::High => vec![first_monday(Date::ymd(s, 5, 1)).add(-7), first_monday(f.camp.add(-10))],
    };
    days.into_iter().find_map(in_week)
}

/// An AI staff's template (topic 11): the neutral week moved by the head
/// coach's styles. A physical staff goes thud to live and adds contact
/// minutes; a protective one tackles less and moves thud to control; a
/// structured staff spends more on individual work and walkthroughs, a
/// free one more on team periods; an aggressive one practises fast.
pub fn styled(tier: Tier, slot: Slot, st: &Style) -> Template {
    let mut t = neutral(tier, slot);
    let x = |v: f64| ((v - 10.5) / 9.5).clamp(-1.0, 1.0);
    let (phys, free, risk) = (x(st.physical), x(st.freedom), x(st.risk));
    for d in t.iter_mut() {
        for p in d.periods.iter_mut() {
            let adj = |m: u8, k: f64| ((m as f64) * (1.0 + k)).round().clamp(5.0, 120.0) as u8;
            match p.kind {
                Kind::Individual | Kind::Walkthrough => p.min = adj(p.min, -0.2 * free),
                Kind::Team | Kind::SevenOnSeven => p.min = adj(p.min, 0.2 * free),
                Kind::InsideRun | Kind::OneOnOnes => p.min = adj(p.min, 0.25 * phys),
                _ => {}
            }
            if p.contact == Contact::Thud && phys < -0.4 {
                p.contact = Contact::Control;
            } else if p.contact == Contact::Thud && phys > 0.55 && p.kind == Kind::InsideRun {
                p.contact = Contact::Live;
            }
            if risk > 0.4 && matches!(p.kind, Kind::Team | Kind::SevenOnSeven | Kind::Situations) {
                p.tempo = Tempo::Fast;
            }
        }
    }
    t
}

/// The days of a Monday-first week from a template: a game week's days by
/// their distance to the next game (a long week beyond six days takes the
/// bye template's days), every other slot by weekday.
pub fn week_from(slot: Slot, monday: Date, games: &[Date], t: &Template, bye: &Template) -> Week {
    let mut w = Week::default();
    for k in 0..7 {
        let d = monday.add(k as i32);
        w.days[k] = if matches!(slot, Slot::Game) {
            match games.iter().find(|&&g| g >= d) {
                Some(&g) if d.days_to(g) <= 6 => t[6 - d.days_to(g) as usize].clone(),
                _ => bye[k].clone(),
            }
        } else {
            t[k].clone()
        };
        // A game on its day, whatever the template says; the template's
        // own game day keeps its periods (the hotel walkthrough, the
        // captains' meeting).
        if games.contains(&d) {
            let ps = if w.days[k].game { std::mem::take(&mut w.days[k].periods) } else { Vec::new() };
            w.days[k] = Day { periods: ps, game: true, second: false };
        }
    }
    w
}

/// Makes a plan legal for the week's books (the AI never breaks a rule;
/// topic 14, proof 6): what isn't allowed is dropped or toned down, and
/// days and weeks over their minutes are trimmed.
pub fn fit(w: &mut Week, books: &[Book; 7]) {
    for (i, d) in w.days.iter_mut().enumerate() {
        let k = &books[i];
        // What the level and era don't have (no VR headsets in 2005).
        d.periods.retain(|p| available(p.kind, k.tier, k.season, k.big));
        if !k.coaches {
            // The players' own time: their own throwing and study hall stay.
            d.periods.retain(|p| p.players_own());
            d.second = false;
            continue;
        }
        if !k.two_a_days {
            d.second = false;
        }
        d.periods.retain(|p| {
            (k.field || !p.kind.on_field() || p.kind.strength_staff() || p.own())
                && (k.team || !p.kind.offense_against_defense())
        });
        for p in d.periods.iter_mut() {
            p.gear = p.gear.min(k.max_gear);
            p.contact = p.contact.min(k.max_contact).min(p.gear.most_contact());
        }
        if let Some(m) = k.full_session {
            let mut used = 0u32;
            for p in d.periods.iter_mut().filter(|p| p.contact.full() && p.kind != Kind::Scrimmage) {
                if used + p.min as u32 > m as u32 {
                    p.contact = Contact::Control;
                } else {
                    used += p.min as u32;
                }
            }
        }
        if let Some(m) = k.day_min {
            trim(d, m as u32, false);
        }
        if let Some(m) = k.field_min {
            trim(d, m as u32, true);
        }
    }
    // Practice days a week, then full contact a week and on days in a row,
    // then the week's minutes and film.
    let k = *books.iter().find(|b| b.coaches).unwrap_or(&books[0]);
    if let Some(n) = k.practice_days {
        let mut seen = 0;
        for d in w.days.iter_mut() {
            if !d.rest() {
                seen += 1;
                if seen > n {
                    d.periods.clear();
                }
            }
        }
    }
    let mut full_days = 0u8;
    let mut full_week = 0u32;
    let mut prev_full = false;
    for d in w.days.iter_mut() {
        let mut today = 0u32;
        let room_days = k.full_days.is_none_or(|n| full_days < n);
        let room_row = k.back_to_back || !prev_full;
        for p in d.periods.iter_mut().filter(|p| p.contact.full()) {
            let room_week = k.full_week.is_none_or(|m| full_week + today + p.min as u32 <= m as u32);
            if room_days && room_row && room_week {
                today += p.min as u32;
            } else {
                p.contact = Contact::Control;
            }
        }
        if today > 0 {
            full_days += 1;
        }
        full_week += today;
        prev_full = today > 0;
    }
    if let Some(m) = k.film_week {
        let mut film = 0u32;
        for d in w.days.iter_mut() {
            d.periods.retain(|p| {
                if p.kind.film() {
                    film += p.min as u32;
                    film <= m as u32
                } else {
                    true
                }
            });
        }
    }
    if let Some(m) = k.week_min {
        let total: u32 = w.days.iter().map(|d| d.countable()).sum();
        if total > m as u32 {
            // A game day counts three hours flat: only practice days scale.
            let games: u32 = w.days.iter().filter(|d| d.game).map(|d| d.countable()).sum();
            let scale = (m as f64 - games as f64).max(0.0) / (total - games).max(1) as f64;
            for d in w.days.iter_mut().filter(|d| !d.game) {
                for p in d.periods.iter_mut().filter(|p| p.counted()) {
                    p.min = ((p.min as f64 * scale).floor() as u8).max(5);
                }
                d.periods.retain(|p| p.min >= 5);
            }
            // Rounding up to five minutes can still leave it over: cut the
            // longest periods until it fits.
            while w.days.iter().map(|d| d.countable()).sum::<u32>() > m as u32 {
                let Some(p) = w
                    .days
                    .iter_mut()
                    .filter(|d| !d.game)
                    .flat_map(|d| d.periods.iter_mut())
                    .filter(|p| p.counted() && p.min > 5)
                    .max_by_key(|p| p.min)
                else {
                    break;
                };
                p.min -= 5;
            }
        }
    }
    // Days off the rules require: the lightest days rest.
    let need = k.days_off as usize;
    while w.days.iter().filter(|d| d.rest()).count() < need {
        let Some(i) = (0..7).filter(|&i| !w.days[i].rest() && !w.days[i].game).min_by_key(|&i| w.days[i].minutes())
        else {
            break;
        };
        w.days[i].periods.retain(|p| !p.counted());
    }
}

/// Keeps a week inside what is left of its block's counts (topic 14.6):
/// once the block's practices are used, a day keeps only its weight room,
/// meetings and conditioning; once its scrimmages, contact or padded
/// practices are used, those tone down (a scrimmage becomes a team
/// period, live and thud become control, pads become shells).
pub fn fit_block(w: &mut Week, used: &super::rules::Counts, lim: &super::rules::Counts) {
    let mut c = *used;
    for d in w.days.iter_mut() {
        if d.rest() || !d.periods.iter().any(|p| p.practice()) {
            continue;
        }
        let room = lim.practices.saturating_sub(c.practices);
        if room == 0 {
            d.periods.retain(|p| !p.practice());
            d.second = false;
            continue;
        }
        if d.second && room < 2 {
            d.second = false;
        }
        c.practices += 1 + d.second as u16;
        if d.periods.iter().any(|p| p.kind == Kind::Scrimmage) {
            if c.scrimmages >= lim.scrimmages {
                for p in d.periods.iter_mut().filter(|p| p.kind == Kind::Scrimmage) {
                    p.kind = Kind::Team;
                }
            } else {
                c.scrimmages += 1;
            }
        }
        if d.periods.iter().any(|p| p.gear == Gear::Pads) {
            if c.padded >= lim.padded {
                for p in d.periods.iter_mut().filter(|p| p.gear == Gear::Pads) {
                    p.gear = Gear::Shells;
                    p.contact = p.contact.min(Gear::Shells.most_contact());
                }
            } else {
                c.padded += 1;
            }
        }
        if d.periods.iter().any(|p| p.contact >= Contact::Thud) {
            if c.contact >= lim.contact {
                for p in d.periods.iter_mut().filter(|p| p.contact >= Contact::Thud) {
                    p.contact = Contact::Control;
                }
            } else {
                c.contact += 1;
            }
        }
    }
}

/// Cuts a day to `max` countable minutes (or on-field minutes with
/// `field`), shortening the longest periods first.
fn trim(d: &mut Day, max: u32, field: bool) {
    let measure = |d: &Day| if field { d.field() } else { d.countable() };
    let counts = |p: &Period| if field { p.kind.on_field() } else { p.counted() };
    while measure(d) > max {
        let Some(p) = d.periods.iter_mut().filter(|p| counts(p) && p.min > 5).max_by_key(|p| p.min) else {
            d.periods.retain(|p| !counts(p));
            return;
        };
        p.min -= 5;
    }
}

/// The weekday of a date, Monday 0.
pub fn weekday_index(d: Date) -> usize {
    match d.weekday() {
        Weekday::Mon => 0,
        Weekday::Tue => 1,
        Weekday::Wed => 2,
        Weekday::Thu => 3,
        Weekday::Fri => 4,
        Weekday::Sat => 5,
        Weekday::Sun => 6,
    }
}
