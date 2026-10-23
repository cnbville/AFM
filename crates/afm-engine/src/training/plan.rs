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
}

pub const KINDS: usize = 17;

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
        ][self as usize]
    }

    pub fn parse(s: &str) -> Option<Kind> {
        Kind::ALL.iter().copied().find(|k| k.name() == s)
    }

    /// The colour group the planner shows it in.
    pub fn group(self) -> &'static str {
        match self {
            Kind::WarmUp | Kind::Conditioning | Kind::Speed | Kind::WeightRoom | Kind::Recovery => "body",
            Kind::Individual | Kind::OneOnOnes | Kind::Young => "technique",
            Kind::InsideRun | Kind::SevenOnSeven | Kind::Team | Kind::Scrimmage => "team",
            Kind::Situations | Kind::Walkthrough => "situations",
            Kind::SpecialTeams => "special",
            Kind::Meeting | Kind::Film => "classroom",
        }
    }

    /// On the practice field (with coaches and a ball).
    pub fn on_field(self) -> bool {
        !matches!(self, Kind::WeightRoom | Kind::Meeting | Kind::Film | Kind::Recovery)
    }

    /// A practice in the rule books' count: a ball and coaches on the
    /// field, not a conditioning or speed session.
    pub fn practice(self) -> bool {
        self.on_field() && !matches!(self, Kind::Conditioning | Kind::Speed | Kind::WarmUp)
    }

    /// Offense against defense.
    pub fn offense_against_defense(self) -> bool {
        matches!(
            self,
            Kind::InsideRun | Kind::SevenOnSeven | Kind::OneOnOnes | Kind::Team | Kind::Scrimmage | Kind::Situations
        )
    }

    pub fn film(self) -> bool {
        matches!(self, Kind::Meeting | Kind::Film | Kind::Walkthrough)
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
    /// opponent's looks.
    #[serde(default)]
    pub what: u8,
}

impl Period {
    pub fn new(kind: Kind, min: u8, gear: Gear, contact: Contact) -> Period {
        let who = match kind {
            Kind::Team | Kind::Scrimmage | Kind::Situations | Kind::InsideRun | Kind::SevenOnSeven => Who::Ones,
            Kind::Young => Who::Young,
            _ => Who::All,
        };
        let tempo = if kind == Kind::Walkthrough { Tempo::Walk } else { Tempo::Normal };
        Period { kind, min, gear, contact, tempo, who, what: 0 }
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
    /// A day with nothing required (recovery may still happen).
    pub fn rest(&self) -> bool {
        !self.game && self.periods.iter().all(|p| p.kind == Kind::Recovery)
    }

    /// NCAA countable minutes: everything but treatment; a game counts
    /// three hours.
    pub fn countable(&self) -> u32 {
        self.periods.iter().filter(|p| p.kind != Kind::Recovery).map(|p| p.min as u32).sum::<u32>()
            + if self.game { 180 } else { 0 }
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

fn day(ps: &[(Kind, u8, Gear, Contact)]) -> Day {
    Day { periods: ps.iter().map(|&(k, m, g, c)| Period::new(k, m, g, c)).collect(), game: false, second: false }
}

fn off() -> Day {
    Day::default()
}

use Contact::{Air, Bags, Control, Thud};
use Gear::{Helmets, Pads, Shells};
use Kind::*;

/// The neutral staff's template for a slot at a level: what an average
/// staff of the era runs (proposed, not confirmed: built from the real
/// weeks of calendar topic 6 and the period lengths of real practice
/// scripts).
pub fn neutral(tier: Tier, slot: Slot) -> Template {
    let g = Gear::None;
    match (tier, slot) {
        (_, Slot::Own) => Default::default(),
        (Tier::College, Slot::Game) => [
            // Sunday after the game: treatment, film, the young players' practice.
            day(&[(Recovery, 30, g, Air), (Film, 45, g, Air), (Young, 50, Shells, Control)]),
            off(),
            day(&[
                (Meeting, 45, g, Air),
                (Film, 30, g, Air),
                (WarmUp, 15, Pads, Air),
                (Individual, 20, Pads, Bags),
                (SpecialTeams, 15, Pads, Control),
                (InsideRun, 15, Pads, Thud),
                (OneOnOnes, 10, Pads, Thud),
                (SevenOnSeven, 15, Pads, Air),
                (Team, 30, Pads, Thud),
                (Conditioning, 10, Pads, Air),
                (WeightRoom, 30, g, Air),
            ]),
            day(&[
                (Meeting, 45, g, Air),
                (Film, 30, g, Air),
                (WarmUp, 15, Pads, Air),
                (Individual, 20, Pads, Bags),
                (SpecialTeams, 15, Pads, Control),
                (Situations, 20, Pads, Thud),
                (SevenOnSeven, 15, Pads, Air),
                (Team, 30, Pads, Control),
                (WeightRoom, 25, g, Air),
            ]),
            day(&[
                (Meeting, 40, g, Air),
                (Film, 30, g, Air),
                (WarmUp, 10, Shells, Air),
                (Individual, 15, Shells, Bags),
                (SpecialTeams, 15, Shells, Air),
                (Situations, 20, Shells, Control),
                (Team, 20, Shells, Control),
            ]),
            day(&[(Walkthrough, 40, Helmets, Air), (Meeting, 30, g, Air), (Film, 20, g, Air)]),
            Day { game: true, ..off() },
        ],
        (Tier::College, Slot::Bye) | (Tier::College, Slot::Bowl) => [
            off(),
            day(&[(WeightRoom, 45, g, Air), (Film, 30, g, Air), (Recovery, 30, g, Air)]),
            day(&[
                (Meeting, 30, g, Air),
                (WarmUp, 15, Pads, Air),
                (Individual, 30, Pads, Bags),
                (OneOnOnes, 15, Pads, Thud),
                (Young, 30, Pads, Thud),
                (Team, 25, Pads, Thud),
                (Conditioning, 15, Pads, Air),
                (WeightRoom, 30, g, Air),
            ]),
            day(&[
                (Meeting, 30, g, Air),
                (WarmUp, 15, Shells, Air),
                (Individual, 30, Shells, Bags),
                (SevenOnSeven, 20, Shells, Air),
                (Young, 30, Shells, Control),
                (Situations, 20, Shells, Control),
            ]),
            day(&[(WeightRoom, 45, g, Air), (Film, 30, g, Air)]),
            day(&[
                (Meeting, 30, g, Air),
                (WarmUp, 15, Pads, Air),
                (Individual, 25, Pads, Bags),
                (Young, 30, Pads, Thud),
                (Team, 30, Pads, Thud),
                (SpecialTeams, 15, Pads, Control),
            ]),
            off(),
        ],
        (Tier::College, Slot::Camp) => {
            let practice = |gear: Gear, c: Contact| {
                day(&[
                    (Meeting, 60, Gear::None, Air),
                    (Film, 30, Gear::None, Air),
                    (WarmUp, 15, gear, Air),
                    (Individual, 25, gear, Bags),
                    (SpecialTeams, 15, gear, Control),
                    (InsideRun, 15, gear, c),
                    (OneOnOnes, 15, gear, c),
                    (SevenOnSeven, 15, gear, Air),
                    (Team, 30, gear, c),
                    (Situations, 15, gear, Control),
                    (Conditioning, 15, gear, Air),
                    (WeightRoom, 30, Gear::None, Air),
                ])
            };
            [
                practice(Shells, Control),
                practice(Pads, Thud),
                practice(Shells, Control),
                practice(Pads, Thud),
                practice(Shells, Control),
                {
                    let mut d = practice(Pads, Thud);
                    d.periods.retain(|p| !matches!(p.kind, Team | InsideRun));
                    d.periods.insert(4, Period::new(Scrimmage, 50, Pads, Contact::Live));
                    d
                },
                day(&[(Recovery, 30, g, Air)]),
            ]
        }
        (Tier::College, Slot::Spring) => [
            day(&[(WeightRoom, 45, g, Air), (Film, 30, g, Air)]),
            day(&[
                (Meeting, 45, g, Air),
                (Film, 30, g, Air),
                (WarmUp, 15, Shells, Air),
                (Individual, 25, Shells, Bags),
                (OneOnOnes, 15, Shells, Control),
                (SevenOnSeven, 15, Shells, Air),
                (Team, 30, Shells, Control),
                (SpecialTeams, 10, Shells, Air),
                (Conditioning, 10, Shells, Air),
            ]),
            day(&[(WeightRoom, 45, g, Air), (Film, 30, g, Air)]),
            day(&[
                (Meeting, 45, g, Air),
                (WarmUp, 15, Pads, Air),
                (Individual, 25, Pads, Bags),
                (InsideRun, 15, Pads, Thud),
                (OneOnOnes, 15, Pads, Thud),
                (SevenOnSeven, 15, Pads, Air),
                (Team, 30, Pads, Thud),
                (Situations, 15, Pads, Control),
            ]),
            day(&[(WeightRoom, 45, g, Air), (Film, 30, g, Air)]),
            day(&[
                (Meeting, 30, g, Air),
                (WarmUp, 15, Pads, Air),
                (Individual, 15, Pads, Bags),
                (Scrimmage, 60, Pads, Contact::Live),
                (SpecialTeams, 10, Pads, Control),
            ]),
            off(),
        ],
        (Tier::College, Slot::Winter) => {
            let lift = |film: bool| {
                let mut d = day(&[(WeightRoom, 60, g, Air), (Conditioning, 25, g, Air), (Speed, 20, g, Air)]);
                if film {
                    d.periods.push(Period::new(Film, 30, g, Air));
                }
                d
            };
            [lift(true), lift(false), lift(true), lift(false), off(), off(), off()]
        }
        (Tier::College, Slot::Summer) => {
            let lift = |film: bool| {
                let mut d = day(&[(WeightRoom, 60, g, Air), (Speed, 30, g, Air), (Conditioning, 15, g, Air)]);
                if film {
                    d.periods.push(Period::new(Film, 30, g, Air));
                }
                d
            };
            [lift(true), lift(false), lift(true), lift(false), off(), off(), off()]
        }
        (Tier::Pro, Slot::Game) => [
            day(&[
                (Recovery, 45, g, Air),
                (Film, 60, g, Air),
                (Meeting, 30, g, Air),
                (WeightRoom, 30, g, Air),
                (Walkthrough, 20, Helmets, Air),
            ]),
            off(),
            day(&[
                (Meeting, 90, g, Air),
                (Film, 45, g, Air),
                (WarmUp, 15, Pads, Air),
                (Individual, 20, Pads, Bags),
                (SpecialTeams, 15, Pads, Control),
                (SevenOnSeven, 10, Pads, Air),
                (Team, 35, Pads, Thud),
                (WeightRoom, 30, g, Air),
            ]),
            day(&[
                (Meeting, 75, g, Air),
                (Film, 45, g, Air),
                (WarmUp, 15, Shells, Air),
                (Individual, 20, Shells, Bags),
                (SpecialTeams, 15, Shells, Air),
                (Situations, 30, Shells, Control),
                (Team, 20, Shells, Control),
                (WeightRoom, 20, g, Air),
            ]),
            day(&[
                (Meeting, 60, g, Air),
                (Film, 30, g, Air),
                (WarmUp, 10, Helmets, Air),
                (Individual, 10, Helmets, Air),
                (SpecialTeams, 10, Helmets, Air),
                (Situations, 30, Helmets, Air),
                (Team, 15, Helmets, Air),
            ]),
            day(&[(Walkthrough, 45, Helmets, Air), (Meeting, 45, g, Air)]),
            Day { game: true, ..off() },
        ],
        (Tier::Pro, Slot::Bye) | (Tier::Pro, Slot::Bowl) => [
            day(&[(Film, 60, g, Air), (Meeting, 30, g, Air), (WeightRoom, 30, g, Air)]),
            day(&[
                (Meeting, 60, g, Air),
                (WarmUp, 15, Shells, Air),
                (Individual, 30, Shells, Bags),
                (Young, 30, Shells, Control),
                (Team, 25, Shells, Control),
                (WeightRoom, 30, g, Air),
            ]),
            day(&[(Meeting, 45, g, Air), (Film, 30, g, Air), (Walkthrough, 30, Helmets, Air)]),
            off(),
            off(),
            off(),
            off(),
        ],
        (Tier::Pro, Slot::Camp) => {
            let practice = |gear: Gear, c: Contact| {
                day(&[
                    (Meeting, 90, Gear::None, Air),
                    (Film, 45, Gear::None, Air),
                    (WarmUp, 15, gear, Air),
                    (Individual, 25, gear, Bags),
                    (SpecialTeams, 15, gear, Control),
                    (InsideRun, 15, gear, c),
                    (OneOnOnes, 15, gear, c),
                    (SevenOnSeven, 15, gear, Air),
                    (Team, 30, gear, c),
                    (Situations, 15, gear, Control),
                    (Conditioning, 10, gear, Air),
                    (WeightRoom, 30, Gear::None, Air),
                ])
            };
            [
                practice(Pads, Thud),
                practice(Pads, Thud),
                practice(Shells, Control),
                practice(Pads, Thud),
                practice(Shells, Control),
                practice(Shells, Control),
                day(&[(Recovery, 30, g, Air)]),
            ]
        }
        (Tier::Pro, Slot::Phase1) => {
            let d = day(&[
                (Meeting, 30, g, Air),
                (Film, 30, g, Air),
                (WeightRoom, 75, g, Air),
                (Conditioning, 30, g, Air),
                (Speed, 20, g, Air),
            ]);
            [d.clone(), d.clone(), off(), d.clone(), d, off(), off()]
        }
        (Tier::Pro, Slot::Phase2) => {
            let d = day(&[
                (Meeting, 60, g, Air),
                (Film, 30, g, Air),
                (WarmUp, 10, g, Air),
                (Individual, 40, g, Air),
                (Walkthrough, 20, g, Air),
                (WeightRoom, 60, g, Air),
                (Conditioning, 20, g, Air),
            ]);
            [d.clone(), d.clone(), off(), d.clone(), d, off(), off()]
        }
        (Tier::Pro, Slot::Otas) => {
            let d = day(&[
                (Meeting, 60, g, Air),
                (Film, 30, g, Air),
                (WarmUp, 10, Helmets, Air),
                (Individual, 30, Helmets, Air),
                (SevenOnSeven, 20, Helmets, Air),
                (Team, 30, Helmets, Air),
                (SpecialTeams, 10, Helmets, Air),
                (WeightRoom, 45, g, Air),
            ]);
            [d.clone(), d.clone(), off(), d.clone(), d, off(), off()]
        }
        (Tier::Pro, Slot::Winter | Slot::Summer | Slot::Spring) => Default::default(),
        (Tier::High, Slot::Game) => [
            // Saturday after Friday's game: film, lift, treatment.
            day(&[(Film, 60, g, Air), (WeightRoom, 45, g, Air), (Recovery, 20, g, Air)]),
            off(),
            day(&[
                (Film, 30, g, Air),
                (WeightRoom, 30, g, Air),
                (WarmUp, 10, Shells, Air),
                (Individual, 20, Shells, Bags),
                (Walkthrough, 20, Helmets, Air),
                (SevenOnSeven, 20, Shells, Air),
                (SpecialTeams, 15, Shells, Air),
                (Conditioning, 15, Shells, Air),
            ]),
            day(&[
                (WarmUp, 10, Pads, Air),
                (Individual, 20, Pads, Bags),
                (InsideRun, 15, Pads, Thud),
                (OneOnOnes, 10, Pads, Thud),
                (SevenOnSeven, 15, Pads, Air),
                (Team, 25, Pads, Thud),
                (SpecialTeams, 15, Pads, Control),
                (Conditioning, 10, Pads, Air),
            ]),
            day(&[
                (WarmUp, 10, Pads, Air),
                (Individual, 20, Pads, Bags),
                (Situations, 20, Pads, Control),
                (SevenOnSeven, 15, Pads, Air),
                (Team, 25, Pads, Thud),
                (SpecialTeams, 15, Pads, Control),
                (Conditioning, 10, Pads, Air),
            ]),
            day(&[(Walkthrough, 40, Helmets, Air), (SpecialTeams, 15, Helmets, Air), (Film, 20, g, Air)]),
            Day { game: true, ..off() },
        ],
        (Tier::High, Slot::Bye) | (Tier::High, Slot::Bowl) => {
            let d = day(&[
                (WarmUp, 10, Shells, Air),
                (Individual, 30, Shells, Bags),
                (Young, 30, Shells, Control),
                (SevenOnSeven, 20, Shells, Air),
                (Conditioning, 15, Shells, Air),
                (WeightRoom, 30, g, Air),
            ]);
            [d.clone(), d.clone(), d, day(&[(Film, 30, g, Air)]), off(), off(), off()]
        }
        (Tier::High, Slot::Camp) => {
            let practice = |gear: Gear, c: Contact| {
                day(&[
                    (Film, 20, Gear::None, Air),
                    (WarmUp, 15, gear, Air),
                    (Individual, 30, gear, Bags),
                    (InsideRun, 15, gear, c),
                    (OneOnOnes, 15, gear, c),
                    (SevenOnSeven, 20, gear, Air),
                    (Team, 30, gear, c),
                    (SpecialTeams, 15, gear, Air),
                    (Conditioning, 20, gear, Air),
                ])
            };
            [
                practice(Pads, Thud),
                practice(Pads, Thud),
                practice(Shells, Control),
                practice(Pads, Thud),
                practice(Shells, Control),
                {
                    let mut d = practice(Pads, Thud);
                    d.periods.retain(|p| !matches!(p.kind, Team | InsideRun));
                    d.periods.insert(3, Period::new(Scrimmage, 45, Pads, Contact::Live));
                    d
                },
                off(),
            ]
        }
        (Tier::High, Slot::Spring) => {
            let d = day(&[
                (WarmUp, 10, Pads, Air),
                (Individual, 25, Pads, Bags),
                (InsideRun, 10, Pads, Thud),
                (OneOnOnes, 10, Pads, Thud),
                (SevenOnSeven, 15, Pads, Air),
                (Team, 25, Pads, Thud),
                (SpecialTeams, 10, Pads, Air),
                (Conditioning, 15, Pads, Air),
            ]);
            [d.clone(), d.clone(), off(), d.clone(), d, off(), off()]
        }
        (Tier::High, Slot::Winter) => {
            let d = day(&[(WeightRoom, 60, g, Air), (Speed, 20, g, Air)]);
            [d.clone(), d.clone(), d.clone(), d, off(), off(), off()]
        }
        (Tier::High, Slot::Summer) => {
            let d = day(&[
                (WeightRoom, 60, g, Air),
                (Speed, 30, g, Air),
                (Conditioning, 20, g, Air),
                (SevenOnSeven, 30, Helmets, Air),
            ]);
            [d.clone(), d.clone(), d.clone(), d, off(), off(), off()]
        }
        (_, _) => Default::default(),
    }
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
        // A game on its day, whatever the template says.
        if games.contains(&d) {
            w.days[k] = Day { game: true, ..Default::default() };
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
        if !k.coaches {
            d.periods.clear();
            continue;
        }
        if !k.two_a_days {
            d.second = false;
        }
        d.periods.retain(|p| {
            (k.field || !p.kind.on_field() || matches!(p.kind, Kind::Conditioning | Kind::Speed))
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
            let scale = m as f64 / total as f64;
            for d in w.days.iter_mut() {
                for p in d.periods.iter_mut().filter(|p| p.kind != Kind::Recovery) {
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
                    .flat_map(|d| d.periods.iter_mut())
                    .filter(|p| p.kind != Kind::Recovery && p.min > 5)
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
        w.days[i].periods.retain(|p| p.kind == Kind::Recovery);
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
        if d.rest() || !d.periods.iter().any(|p| p.kind.practice()) {
            continue;
        }
        let room = lim.practices.saturating_sub(c.practices);
        if room == 0 {
            d.periods.retain(|p| !p.kind.practice());
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
    while measure(d) > max {
        let Some(p) = d
            .periods
            .iter_mut()
            .filter(|p| p.kind != Kind::Recovery && p.min > 5 && (!field || p.kind.on_field()))
            .max_by_key(|p| p.min)
        else {
            d.periods.retain(|p| p.kind == Kind::Recovery || (field && !p.kind.on_field()));
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
