//! Section 7 for your seat and the build 9 leftovers (`docs/design/scouting.md`;
//! the final build's lane 7): the scouting file every staff keeps, read
//! attribute by attribute from the looks it took; the public evaluators
//! (recruiting services and draft analysts) with their rankings, big
//! boards and mock drafts; your own department (assignments, evaluation
//! days, camps, calls, visits, interviews, look-backs); the draft board and
//! the live draft room; prospects' own pre-draft choices, smokescreens and
//! the supplemental draft; pro scouting; the high school coach's side;
//! social media with NIL from followers.
//!
//! Memory (the build's gate): AI clubs keep only their looks (the built
//! `pro::Club::files`, eight bytes a look, cleared after each draft); your
//! own seat keeps its looks the same way, and every read is rebuilt from
//! them when a screen asks. Everything here is kept by player id, never by
//! place in the people list, which the year-end book compacts.
//!
//! The real rules and rhythms are researched; the sizes and weights are
//! Claude's, proposed, not confirmed (logged in `docs/world-model.md`).

pub mod board;
pub mod college;
pub mod dept;
pub mod file;
pub mod hs;
pub mod lines;
pub mod live;
pub mod lookback;
pub mod predraft;
pub mod pro;
pub mod public;
pub mod social;
pub mod supp;

use std::collections::HashMap;

use crate::calendar::Date;

/// Every kind of look a staff can take (topic 4), at every level. The NFL
/// club files use the built `LookKind`; your college and high school files
/// use these, and the file reader maps both onto them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum Kind {
    /// An NFL area scout's fall school call: film, practice and the people
    /// around him (the strength coach, the trainer, the equipment man).
    School,
    /// Film from the office.
    Film,
    CrossCheck,
    Director,
    Gm,
    /// The head coach or a coordinator: spring film, pro day drills.
    Coach,
    AllStar,
    Combine,
    Interview,
    ProDay,
    /// A top-30 visit: interviews, a physical and written tests, no drills.
    Visit,
    /// A private workout at his school: one club, a coach's full look.
    Workout,
    /// The combine's medical exam or the April recheck.
    Medical,
    /// Club security and NFL Security: the people around him, his record,
    /// his social media.
    Background,
    /// A local day: a player from the club's area, testing allowed.
    LocalDay,
    /// A college coach watching him practice in an evaluation period.
    Practice,
    /// A college coach at his game.
    Game,
    /// Your one-day camp: laser times, a day of coaching him.
    Camp,
    /// A national showcase or high school all-star game.
    Showcase,
    JuniorDay,
    Official,
    Unofficial,
    /// The head coach in the living room (contact period).
    HomeVisit,
    /// Calls to his high school coach, teachers and counselor.
    Call,
    /// The transcript through the Eligibility Center, read by your
    /// academic staff.
    Transcript,
    /// His social media and what the local police and reporters know.
    Social,
    /// His high school's game film (Hudl from about 2008).
    HsFilm,
    /// A portal or junior college player's college film and stat book.
    CollegeFilm,
    /// A call to the staff he left (it runs cold).
    OldStaff,
    /// A workout of a street free agent (Tuesdays in season).
    Tryout,
    /// Pro game film (pro scouting).
    ProFilm,
}

impl Kind {
    pub const ALL: [Kind; 31] = [
        Kind::School,
        Kind::Film,
        Kind::CrossCheck,
        Kind::Director,
        Kind::Gm,
        Kind::Coach,
        Kind::AllStar,
        Kind::Combine,
        Kind::Interview,
        Kind::ProDay,
        Kind::Visit,
        Kind::Workout,
        Kind::Medical,
        Kind::Background,
        Kind::LocalDay,
        Kind::Practice,
        Kind::Game,
        Kind::Camp,
        Kind::Showcase,
        Kind::JuniorDay,
        Kind::Official,
        Kind::Unofficial,
        Kind::HomeVisit,
        Kind::Call,
        Kind::Transcript,
        Kind::Social,
        Kind::HsFilm,
        Kind::CollegeFilm,
        Kind::OldStaff,
        Kind::Tryout,
        Kind::ProFilm,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Kind::School => "School call",
            Kind::Film => "Film study",
            Kind::CrossCheck => "Cross-check",
            Kind::Director => "Director's look",
            Kind::Gm => "GM's look",
            Kind::Coach => "Coach's film and drills",
            Kind::AllStar => "All-star week",
            Kind::Combine => "Combine",
            Kind::Interview => "Interview",
            Kind::ProDay => "Pro day",
            Kind::Visit => "Top-30 visit",
            Kind::Workout => "Private workout",
            Kind::Medical => "Medical exam",
            Kind::Background => "Background check",
            Kind::LocalDay => "Local day",
            Kind::Practice => "Practice evaluation",
            Kind::Game => "Game evaluation",
            Kind::Camp => "Your camp",
            Kind::Showcase => "Showcase",
            Kind::JuniorDay => "Junior day",
            Kind::Official => "Official visit",
            Kind::Unofficial => "Unofficial visit",
            Kind::HomeVisit => "Home visit",
            Kind::Call => "Calls",
            Kind::Transcript => "Transcript",
            Kind::Social => "Social media and record",
            Kind::HsFilm => "Game film",
            Kind::CollegeFilm => "College film",
            Kind::OldStaff => "His old staff",
            Kind::Tryout => "Tryout",
            Kind::ProFilm => "Pro film",
        }
    }

    /// The evidence chip on a range (the mock-up's F, L, C, I): film, live,
    /// camp or measured, interview.
    pub fn chip(self) -> u8 {
        match self {
            Kind::Film | Kind::CrossCheck | Kind::Director | Kind::Gm | Kind::HsFilm | Kind::CollegeFilm | Kind::ProFilm => {
                CHIP_FILM
            }
            Kind::School | Kind::Practice | Kind::Game | Kind::AllStar | Kind::Coach => CHIP_LIVE,
            Kind::Combine | Kind::ProDay | Kind::Camp | Kind::Showcase | Kind::Workout | Kind::LocalDay | Kind::Tryout => {
                CHIP_CAMP
            }
            Kind::Interview
            | Kind::Visit
            | Kind::JuniorDay
            | Kind::Official
            | Kind::Unofficial
            | Kind::HomeVisit
            | Kind::Call
            | Kind::OldStaff
            | Kind::Background
            | Kind::Social
            | Kind::Transcript
            | Kind::Medical => CHIP_TALK,
        }
    }

    /// The looks log's filters (the mock-up): live and school, film and
    /// charting, camp and campus, calls and records, heard and online.
    pub fn family(self) -> &'static str {
        match self {
            Kind::School | Kind::Practice | Kind::Game | Kind::AllStar | Kind::Coach | Kind::Workout | Kind::Tryout => {
                "Live and school"
            }
            Kind::Film | Kind::CrossCheck | Kind::Director | Kind::Gm | Kind::HsFilm | Kind::CollegeFilm | Kind::ProFilm => {
                "Film and charting"
            }
            Kind::Combine
            | Kind::ProDay
            | Kind::Camp
            | Kind::Showcase
            | Kind::Visit
            | Kind::LocalDay
            | Kind::JuniorDay
            | Kind::Official
            | Kind::Unofficial
            | Kind::Medical => "Camp and campus",
            Kind::Interview | Kind::HomeVisit | Kind::Call | Kind::Transcript | Kind::OldStaff => "Calls and records",
            Kind::Background | Kind::Social => "Heard and online",
        }
    }

    /// Measurements everyone shares the moment they're taken (no report to
    /// write).
    pub fn instant(self) -> bool {
        matches!(self, Kind::Combine | Kind::ProDay | Kind::Showcase)
    }

    pub fn from_nfl(k: crate::world::pro::scouts::LookKind) -> Kind {
        use crate::world::pro::scouts::LookKind as L;
        match k {
            L::School => Kind::School,
            L::Film => Kind::Film,
            L::CrossCheck => Kind::CrossCheck,
            L::Director => Kind::Director,
            L::Gm => Kind::Gm,
            L::Coach => Kind::Coach,
            L::AllStar => Kind::AllStar,
            L::Combine => Kind::Combine,
            L::Interview => Kind::Interview,
            L::ProDay => Kind::ProDay,
            L::Visit => Kind::Visit,
            L::Workout => Kind::Workout,
            L::Medical => Kind::Medical,
            L::Background => Kind::Background,
            L::LocalDay => Kind::LocalDay,
            L::Tryout => Kind::Tryout,
            L::ProFilm => Kind::ProFilm,
        }
    }
}

pub const CHIP_FILM: u8 = 1;
pub const CHIP_LIVE: u8 = 2;
pub const CHIP_CAMP: u8 = 4;
pub const CHIP_TALK: u8 = 8;

/// One look in your college or high school seat's file: who (a coach id,
/// or one of the codes in `dept`), what kind, when. Twelve bytes.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SeatLook {
    pub who: u32,
    pub kind: Kind,
    pub date: Date,
}

/// Everything section 7 keeps beside the built clubs' files (stored on the
/// world as `World::scouting`; every field fills in empty on an older file).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Scouting {
    /// The public evaluators: recruiting services and draft analysts.
    #[serde(default)]
    pub desks: Vec<public::Desk>,
    /// This draft season's big boards and mock drafts, and last year's
    /// scored (the media scores them the morning after).
    #[serde(default)]
    pub mocks: Vec<public::Mock>,
    #[serde(default)]
    pub mock_scores: Vec<public::MockScore>,
    /// Each prospect's best spot in this season's mocks and boards (player
    /// id, pick): a player who falls far below it carries it.
    #[serde(default)]
    pub mocked: HashMap<u32, u16>,
    /// Your department: assignments, instructions, your seat's looks.
    #[serde(default)]
    pub dept: dept::Dept,
    /// Your draft board as you set it.
    #[serde(default)]
    pub board: board::SeatBoard,
    /// The draft while it is live (your seat only).
    #[serde(default)]
    pub live: Option<live::LiveDraft>,
    /// The war room's last draft, for its page and the look-back.
    #[serde(default)]
    pub last_draft: Option<live::DraftRecord>,
    /// Prospects' own choices this pre-draft season (player id).
    #[serde(default)]
    pub choices: HashMap<u32, predraft::Choice>,
    /// Interest that leaked this season, true or planted.
    #[serde(default)]
    pub leaks: Vec<predraft::Leak>,
    /// Undrafted players left for rookie minicamp tryouts: (player id,
    /// clubs that will try him out).
    #[serde(default)]
    pub tryouts: Vec<(u32, Vec<u8>)>,
    /// Supplemental drafts held (through 2019).
    #[serde(default)]
    pub supplemental: Vec<supp::SuppPick>,
    /// The grades kept for look-backs.
    #[serde(default)]
    pub records: lookback::Records,
    /// Each college program's personnel staff (topic 2).
    #[serde(default)]
    pub college: Vec<college::Personnel>,
    /// Each NFL club's pro personnel department (topic 13).
    #[serde(default)]
    pub pro: Vec<pro::ProDept>,
    /// Social media accounts and followers (people topic 9).
    #[serde(default)]
    pub social: social::Social,
    /// Your side as a high school coach (topic 17).
    #[serde(default)]
    pub hs: hs::HsSide,
}

/// The daily scouting work (called from the clock each day, after the
/// day's other systems): your department's calls and evaluation days, the
/// public desks' boards and mocks on their dates, prospects' choices, the
/// supplemental draft, pro scouting's rhythm, and followers.
pub(crate) fn day(w: &mut crate::world::World, d: Date) {
    if w.scouting.desks.is_empty() {
        public::staff_desks(w);
    }
    public::day(w, d);
    college::day(w, d);
    dept::day(w, d);
    predraft::day(w, d);
    supp::day(w, d);
    pro::day(w, d);
    hs::day(w, d);
    social::day(w, d);
    lookback::day(w, d);
}

/// After a file loads: what an older world didn't keep starts empty and
/// fills itself on its first day; a live draft saved mid-draft carries on.
pub(crate) fn after_load(w: &mut crate::world::World) {
    if w.scouting.desks.is_empty() {
        public::staff_desks(w);
    }
}

/// The people list's place for each player id, for code that walks many
/// players by id.
pub(crate) fn places(w: &crate::world::World) -> HashMap<u32, usize> {
    w.people.iter().enumerate().map(|(i, p)| (p.player.id, i)).collect()
}

/// One player's place by id.
pub fn place(w: &crate::world::World, id: u32) -> Option<usize> {
    w.people.iter().position(|p| p.player.id == id)
}
