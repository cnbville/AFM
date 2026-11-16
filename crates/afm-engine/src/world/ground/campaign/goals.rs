//! The goal list (GR36): the 63 recruiting goals of `ground/goals.md` in
//! eight groups, each a measure the campaign, your AD's word and a staff
//! deal can name (`Aim`), read from the world's own record (`read`): the
//! class's commitments and signings, the services' stars and ranks, the
//! state's best (`legacy::college::state_top`), followers (`fame`), honors,
//! the draft, your finds (lane G4) and your standing on the map (lane G1b).
//!
//! Every goal ends as a share (GR39): Done (all of it), Mostly (two-thirds
//! or more), Partly (a third or more), Missed. A near miss counts half on a
//! goal with one target (the state's #2 when the goal was the #1).
//!
//! A class here is a recruiting class by its high school year (`class_of`):
//! the February meeting's class signs the next December and February, so
//! the August verdict reads it as a pace from the commitments. Readers that
//! need what isn't kept (a commitment's news, a collective's deals) read the
//! nearest thing that is, said so on each row. Every size is the build's
//! pick (proposed, not confirmed).

use crate::positions::Position;
use crate::world::data::Division;
use crate::world::ground::{areas, standing};
use crate::world::{Stage, World};

/// A goal's group in the list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum Group {
    Class,
    Home,
    Names,
    Positions,
    Battles,
    Eye,
    Program,
    Ground,
}

impl Group {
    pub fn words(self) -> &'static str {
        match self {
            Group::Class => "The class",
            Group::Home => "Home and the state",
            Group::Names => "The big names",
            Group::Positions => "Positions",
            Group::Battles => "Battles",
            Group::Eye => "Your eye",
            Group::Program => "The program",
            Group::Ground => "Ground and pipelines",
        }
    }
}

/// What a goal is about besides its number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum Subj {
    None,
    /// A kid, by player id (the AD's named kid, the hype kid).
    Kid,
    /// A ground area.
    Area,
    /// The rival program.
    Rival,
    /// A position (index in `Position::ALL`).
    Position,
    /// A high school.
    School,
    /// A state.
    State,
}

/// Who in the world pitches a goal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum Raiser {
    Ad,
    President,
    Donors,
    Collective,
    Fans,
    Coordinator,
    You,
}

impl Raiser {
    pub fn words(self) -> &'static str {
        match self {
            Raiser::Ad => "your AD",
            Raiser::President => "the president",
            Raiser::Donors => "the donors",
            Raiser::Collective => "the collective",
            Raiser::Fans => "the fans",
            Raiser::Coordinator => "your coordinator",
            Raiser::You => "you",
        }
    }
}

/// One goal on the list.
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct Def {
    pub id: u8,
    pub key: &'static str,
    pub group: Group,
    /// "A top-{n} national class".
    pub name: &'static str,
    /// Classes it runs (GR38).
    pub classes: u8,
    /// Judged after the class plays: its slot comes back once the class
    /// signs, and the result comes back later.
    pub later: bool,
    pub judged: &'static str,
    pub raised: &'static [Raiser],
    /// The number it names by default (0 none).
    pub n: u32,
    pub subj: Subj,
    /// One target (a rank, one kid): a near miss counts half (GR39).
    pub single: bool,
}

use Group as G;
use Raiser as R;
use Subj as S;

macro_rules! def {
    ($id:expr, $key:expr, $g:expr, $name:expr, $cl:expr, $later:expr, $judged:expr, [$($r:expr),*], $n:expr, $s:expr, $single:expr) => {
        Def { id: $id, key: $key, group: $g, name: $name, classes: $cl, later: $later, judged: $judged, raised: &[$($r),*], n: $n, subj: $s, single: $single }
    };
}

/// The 63 goals (`ground/goals.md`), in its order.
pub const GOALS: [Def; 63] = [
    def!(
        1,
        "national_rank",
        G::Class,
        "A top-{n} national class",
        1,
        false,
        "The national class rank",
        [R::Ad, R::President],
        25,
        S::None,
        true
    ),
    def!(
        2,
        "conference_rank",
        G::Class,
        "A top-{n} class in the conference",
        1,
        false,
        "The class rank in the conference",
        [R::Ad],
        6,
        S::None,
        true
    ),
    def!(
        3,
        "beat_rival_class",
        G::Class,
        "Beat the rival's class",
        1,
        false,
        "Your class ranked above his",
        [R::Ad, R::Donors],
        0,
        S::Rival,
        true
    ),
    def!(
        4,
        "state_best_class",
        G::Class,
        "The best class in the state",
        1,
        false,
        "Above every program in your state",
        [R::Ad, R::President],
        0,
        S::None,
        true
    ),
    def!(
        5,
        "blue_chips",
        G::Class,
        "{n} blue-chips in one class",
        1,
        false,
        "Four- and five-stars signed",
        [R::Ad, R::You],
        4,
        S::None,
        false
    ),
    def!(
        6,
        "blue_chip_roster",
        G::Class,
        "A blue-chip roster",
        3,
        false,
        "Over your last four classes, more four- and five-stars than two- and three-stars",
        [R::You, R::Ad],
        0,
        S::None,
        false
    ),
    def!(
        7,
        "national_rank_run",
        G::Class,
        "A top-{n} class three years running",
        3,
        false,
        "Each year's rank",
        [R::Ad],
        25,
        S::None,
        true
    ),
    def!(
        8,
        "hold_class",
        G::Class,
        "Hold the class",
        1,
        false,
        "No commitment lost before signing day",
        [R::You],
        0,
        S::None,
        true
    ),
    def!(
        9,
        "early_class",
        G::Class,
        "An early class: {n} commitments by the verdict",
        1,
        false,
        "Commitments by the verdict",
        [R::Coordinator],
        12,
        S::None,
        false
    ),
    def!(
        10,
        "class_fits",
        G::Class,
        "A class that fits",
        1,
        false,
        "Every signee graded a fit at his spot by your staff",
        [R::You],
        0,
        S::None,
        false
    ),
    def!(
        11,
        "state_one",
        G::Home,
        "The state's #1",
        1,
        false,
        "Sign your state's top-ranked prospect",
        [R::Ad, R::President, R::Donors],
        0,
        S::None,
        true
    ),
    def!(
        12,
        "state_one_two",
        G::Home,
        "The state's #1, two years running",
        2,
        false,
        "Each year's #1",
        [R::Ad],
        0,
        S::None,
        true
    ),
    def!(
        13,
        "state_one_three",
        G::Home,
        "The state's #1, three years running",
        3,
        false,
        "Each year's #1",
        [R::Ad, R::You],
        0,
        S::None,
        true
    ),
    def!(
        14,
        "fence",
        G::Home,
        "Keep {n} of the state's top 10 home",
        1,
        false,
        "Your fence, counted at the top 10",
        [R::Ad, R::President],
        6,
        S::None,
        false
    ),
    def!(
        15,
        "fence_run",
        G::Home,
        "Keep {n} of the top 10 home, three years running",
        3,
        false,
        "{n} of the top 10 each year",
        [R::Ad],
        6,
        S::None,
        false
    ),
    def!(
        16,
        "all_state",
        G::Home,
        "{n} first-team all-state signees",
        1,
        false,
        "First-team all-state signees",
        [R::Ad],
        3,
        S::None,
        false
    ),
    def!(
        17,
        "city_best",
        G::Home,
        "{n} of the city's top 5",
        1,
        false,
        "Your metro's top 5",
        [R::President, R::Donors],
        2,
        S::None,
        false
    ),
    def!(
        18,
        "hometown_kid",
        G::Home,
        "The hometown kid",
        1,
        false,
        "The best prospect from your school's own county",
        [R::Fans, R::President],
        0,
        S::None,
        true
    ),
    def!(
        19,
        "win_back_state",
        G::Home,
        "Win back the state",
        1,
        false,
        "More in-state signees than last year's leader",
        [R::Ad, R::Donors],
        0,
        S::None,
        true
    ),
    def!(
        20,
        "half_home",
        G::Home,
        "Half the class from home",
        1,
        false,
        "Your in-state share",
        [R::President],
        0,
        S::None,
        false
    ),
    def!(21, "five_star", G::Names, "A five-star", 1, false, "Sign one", [R::Ad, R::Donors], 0, S::None, true),
    def!(
        22,
        "generational",
        G::Names,
        "The generational one",
        1,
        false,
        "The best prospect your state has had in ten years",
        [R::Ad, R::Donors, R::President],
        0,
        S::Kid,
        true
    ),
    def!(
        23,
        "hype_kid",
        G::Names,
        "The hype kid",
        1,
        false,
        "The recruit with the class's biggest following",
        [R::Ad, R::President, R::Collective],
        0,
        S::Kid,
        true
    ),
    def!(
        24,
        "famous_name",
        G::Names,
        "A famous name",
        1,
        false,
        "A recruit from a football family",
        [R::Ad, R::Fans],
        0,
        S::Kid,
        true
    ),
    def!(
        25,
        "national_one",
        G::Names,
        "The national #1",
        1,
        false,
        "The country's top-ranked prospect",
        [R::Ad],
        0,
        S::Kid,
        true
    ),
    def!(
        26,
        "two_five_stars",
        G::Names,
        "Two five-stars in one class",
        1,
        false,
        "Sign two",
        [R::Ad],
        2,
        S::None,
        false
    ),
    def!(
        27,
        "loud_commit",
        G::Names,
        "A commitment the country hears",
        1,
        false,
        "His commitment is big or huge news",
        [R::Ad, R::President],
        0,
        S::None,
        true
    ),
    def!(
        28,
        "qb_future",
        G::Positions,
        "A quarterback of the future",
        1,
        false,
        "A top-10 quarterback in the class",
        [R::Ad, R::You],
        10,
        S::None,
        true
    ),
    def!(
        29,
        "qb_every_class",
        G::Positions,
        "A quarterback every class",
        3,
        false,
        "One each year your staff grades a future starter",
        [R::You],
        0,
        S::None,
        true
    ),
    def!(
        30,
        "elite_tackle",
        G::Positions,
        "An elite offensive tackle",
        1,
        false,
        "A top-10 offensive tackle",
        [R::You],
        10,
        S::None,
        true
    ),
    def!(
        31,
        "rebuild_line",
        G::Positions,
        "Rebuild the line",
        2,
        false,
        "A tackle, a guard and a center your staff grades future starters",
        [R::Coordinator],
        0,
        S::None,
        false
    ),
    def!(
        32,
        "secondary",
        G::Positions,
        "A secondary class",
        1,
        false,
        "Two corners and a safety graded starters",
        [R::Coordinator],
        0,
        S::None,
        false
    ),
    def!(
        33,
        "biggest_need",
        G::Positions,
        "The best at your biggest need",
        1,
        false,
        "The best player at your weakest spot on your staff's board",
        [R::Coordinator],
        0,
        S::Kid,
        true
    ),
    def!(
        34,
        "freshman_specialist",
        G::Positions,
        "A freshman specialist",
        1,
        true,
        "A kicker or punter who starts as a freshman",
        [R::Coordinator],
        0,
        S::None,
        true
    ),
    def!(
        35,
        "own_position",
        G::Positions,
        "Own a position in the state",
        1,
        false,
        "Your state's top 3 at one position",
        [R::You],
        3,
        S::Position,
        false
    ),
    def!(
        36,
        "plays_now",
        G::Positions,
        "An athlete who plays now",
        1,
        true,
        "A skill athlete who plays as a freshman",
        [R::You],
        0,
        S::None,
        true
    ),
    def!(
        37,
        "size_inside",
        G::Positions,
        "Size inside: {n} big defensive tackles",
        1,
        false,
        "Defensive tackles at 300 pounds or more",
        [R::Coordinator],
        2,
        S::None,
        false
    ),
    def!(
        38,
        "head_to_head",
        G::Battles,
        "Head to head with the rival: win {n}",
        1,
        false,
        "Recruits you both offered",
        [R::Ad, R::Donors],
        3,
        S::Rival,
        false
    ),
    def!(
        39,
        "flip_one",
        G::Battles,
        "Flip one",
        1,
        false,
        "A recruit committed elsewhere signs with you",
        [R::You],
        0,
        S::None,
        true
    ),
    def!(
        40,
        "flip_blue_blood",
        G::Battles,
        "Flip a blue blood",
        1,
        false,
        "A blue-chip committed to a top-10 program signs with you",
        [R::Ad, R::Donors],
        0,
        S::None,
        true
    ),
    def!(
        41,
        "backyard",
        G::Battles,
        "Into his backyard: {n} from the rival's state",
        1,
        false,
        "Signees from the rival's state",
        [R::You],
        2,
        S::Rival,
        false
    ),
    def!(
        42,
        "nobody_flips",
        G::Battles,
        "Nobody flips",
        1,
        false,
        "No commitment lost to a rival",
        [R::Ad],
        0,
        S::None,
        true
    ),
    def!(
        43,
        "beat_champion",
        G::Battles,
        "Out-recruit the champion",
        1,
        false,
        "A class ranked above the conference champion's",
        [R::Ad],
        0,
        S::None,
        true
    ),
    def!(
        44,
        "hunt",
        G::Eye,
        "The hunt",
        1,
        false,
        "A find of yours committed to you or rising with the services",
        [R::You],
        0,
        S::None,
        true
    ),
    def!(
        45,
        "find_starts",
        G::Eye,
        "A find who starts",
        1,
        true,
        "An unranked signee who starts as a freshman",
        [R::You],
        0,
        S::None,
        true
    ),
    def!(
        46,
        "two_star_all_conf",
        G::Eye,
        "A two-star all-conference",
        1,
        true,
        "A two-star signee named all-conference in his first three seasons",
        [R::You],
        0,
        S::None,
        true
    ),
    def!(47, "unranked_drafted", G::Eye, "An unranked kid drafted", 1, true, "Drafted", [R::You], 0, S::None, true),
    def!(
        48,
        "outplays_stars",
        G::Eye,
        "A class that outplays its stars: {n} starting by year three",
        1,
        true,
        "Signees starting by their third year",
        [R::You],
        5,
        S::None,
        false
    ),
    def!(
        49,
        "ahead_of_services",
        G::Eye,
        "Ahead of the services: {n} who end four-stars",
        1,
        false,
        "Kids you offered before the services rated them who end four-stars",
        [R::You],
        1,
        S::None,
        false
    ),
    def!(
        50,
        "reach",
        G::Program,
        "A class with reach",
        1,
        false,
        "The class's combined following",
        [R::President, R::Collective],
        250_000,
        S::None,
        false
    ),
    def!(
        51,
        "qualifies",
        G::Program,
        "Every signee qualifies",
        1,
        false,
        "No academic casualty",
        [R::President, R::Ad],
        0,
        S::None,
        false
    ),
    def!(
        52,
        "graduates",
        G::Program,
        "A class that graduates",
        1,
        true,
        "The class's graduation rate",
        [R::President],
        0,
        S::None,
        false
    ),
    def!(
        53,
        "clean_class",
        G::Program,
        "A clean class",
        1,
        false,
        "No signee in trouble with the law or his school",
        [R::Ad, R::President],
        0,
        S::None,
        false
    ),
    def!(
        54,
        "inside_pot",
        G::Program,
        "Inside the collective's pot",
        1,
        false,
        "No deal past what the collective can pay",
        [R::Ad, R::Collective],
        0,
        S::None,
        false
    ),
    def!(
        55,
        "donors_kid",
        G::Program,
        "The donor's kid",
        1,
        false,
        "The recruit a big donor asks for",
        [R::Donors],
        0,
        S::Kid,
        true
    ),
    def!(
        56,
        "collectives_man",
        G::Program,
        "The collective's man",
        1,
        false,
        "The recruit the collective puts its money on",
        [R::Collective],
        0,
        S::Kid,
        true
    ),
    def!(
        57,
        "pipeline",
        G::Ground,
        "Build a pipeline: {n} from one high school",
        3,
        false,
        "Signees from one high school",
        [R::You],
        3,
        S::School,
        false
    ),
    def!(
        58,
        "new_state",
        G::Ground,
        "Open a new state: {n} signees",
        2,
        false,
        "Signees from a state you haven't signed from in five years",
        [R::You],
        2,
        S::State,
        false
    ),
    def!(59, "take_area", G::Ground, "Take an area", 1, false, "Become its leader", [R::You], 0, S::Area, true),
    def!(60, "hold_area", G::Ground, "Hold an area", 1, false, "Stay its leader", [R::You], 0, S::Area, true),
    def!(61, "open_area", G::Ground, "Open an area", 1, false, "From Stranger to Known", [R::You], 0, S::Area, true),
    def!(
        62,
        "juco_pipeline",
        G::Ground,
        "A junior college pipeline: {n} who start",
        2,
        true,
        "Junior college signees who start",
        [R::You],
        2,
        S::None,
        false
    ),
    def!(
        63,
        "every_coach",
        G::Ground,
        "Every coach signs one",
        1,
        false,
        "Each assistant signs a recruit from his own area",
        [R::Coordinator],
        0,
        S::None,
        false
    ),
];

/// A goal by its number.
pub fn def(id: u8) -> &'static Def {
    &GOALS[(id.clamp(1, 63) - 1) as usize]
}

/// A goal by its key.
pub fn by_key(key: &str) -> Option<&'static Def> {
    GOALS.iter().find(|d| d.key == key)
}

/// One goal set for a class: its number, its number to reach, what it's
/// about and its first class. The same aim names a campaign goal, your
/// AD's goal (`folk::boss::GoalKind::Campaign`) and a staff deal's term.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Aim {
    pub goal: u8,
    pub n: u32,
    /// A player id, area, program, position index, school or state, by the
    /// goal's subject (0 when none).
    pub of: u32,
    /// The first class it is for (high school class year).
    pub class: i32,
}

impl Aim {
    pub fn def(&self) -> &'static Def {
        def(self.goal)
    }

    /// The classes it runs.
    pub fn classes(&self) -> std::ops::RangeInclusive<i32> {
        self.class..=self.class + self.def().classes as i32 - 1
    }

    /// The last class it is for.
    pub fn last(&self) -> i32 {
        self.class + self.def().classes as i32 - 1
    }
}

/// How a goal ended (GR39).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, serde::Serialize, serde::Deserialize)]
pub enum Verdict {
    Missed,
    Partly,
    Mostly,
    Done,
}

impl Verdict {
    pub fn of(share: f32) -> Verdict {
        if share >= 0.999 {
            Verdict::Done
        } else if share >= 2.0 / 3.0 - 1e-4 {
            Verdict::Mostly
        } else if share >= 1.0 / 3.0 - 1e-4 {
            Verdict::Partly
        } else {
            Verdict::Missed
        }
    }

    pub fn words(self) -> &'static str {
        match self {
            Verdict::Done => "Done",
            Verdict::Mostly => "Mostly",
            Verdict::Partly => "Partly",
            Verdict::Missed => "Missed",
        }
    }

    /// What the share moves (GR39): Done all of it, Mostly most, Partly a
    /// little against, Missed against.
    pub fn score(self) -> f32 {
        match self {
            Verdict::Done => 1.0,
            Verdict::Mostly => 0.5,
            Verdict::Partly => -0.25,
            Verdict::Missed => -1.0,
        }
    }
}

/// A goal's reading for one class.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Read {
    /// 0-1 (a near miss on one target is 0.5).
    pub share: f32,
    /// What there is, in words ("2 of 3 tackles", "21st nationally").
    pub words: String,
    /// It can't be judged yet (a later goal before the class plays).
    pub waiting: bool,
}

impl Read {
    fn of(share: f32, words: impl Into<String>) -> Read {
        Read { share: share.clamp(0.0, 1.0), words: words.into(), waiting: false }
    }
    fn wait(words: impl Into<String>) -> Read {
        Read { share: 0.0, words: words.into(), waiting: true }
    }
}

/// The goal's words with its number and its subject's name: "A top-6
/// class in the conference", "The hype kid: Malik Ross", "Open Columbus".
pub fn words(w: &World, a: &Aim) -> String {
    let d = a.def();
    let mut s = d.name.replace("{n}", &a.n.to_string());
    let subject = subject_name(w, a);
    match d.id {
        59..=61 => {
            let verb = match d.id {
                59 => "Take",
                60 => "Hold",
                _ => "Open",
            };
            s = format!("{verb} {}", subject.unwrap_or_else(|| "an area".into()));
        }
        _ => {
            if let Some(n) = subject {
                s = format!("{s}: {n}");
            }
        }
    }
    let classes = d.classes as i32;
    if classes > 1 {
        s.push_str(&format!(" (the classes of {} to {})", a.class, a.class + classes - 1));
    }
    s
}

/// The name of what a goal is about.
pub fn subject_name(w: &World, a: &Aim) -> Option<String> {
    let d = a.def();
    match d.subj {
        Subj::None => None,
        Subj::Kid => {
            let i = crate::world::folk::person(w, a.of)?;
            let p = &w.people[i];
            let school = w.high_schools.get(p.hs).map_or(String::new(), |h| format!(" ({})", h.name));
            Some(format!("{}{school}", p.name))
        }
        Subj::Area => areas::table().get(a.of as usize).map(|x| x.name.to_string()),
        Subj::Rival => w.programs.get(a.of as usize).map(|_| w.program_name(a.of as usize).to_string()),
        Subj::Position => Position::ALL.get(a.of as usize).map(|p| p.name().to_string()),
        Subj::School => w.high_schools.get(a.of as usize).map(|h| h.name.clone()),
        Subj::State => crate::world::geo::STATE_CENTERS
            .get(a.of as usize)
            .map(|s| crate::world::folk::facts::state_name(s.0).to_string()),
    }
}

// --- Who's in a class --------------------------------------------------------------------------

/// The program a person's class went to: his signing, else his
/// commitment (no walk-ons), else his first college once enrolled.
pub(crate) fn went(p: &crate::world::Person) -> Option<usize> {
    match p.stage {
        Stage::HighSchool | Stage::PrepSchool => {
            if let Some(s) = p.cal.signed.as_ref() {
                return (!s.walk_on).then_some(s.program);
            }
            p.cal.commit.filter(|c| !c.2).map(|c| c.0)
        }
        _ => p.path.first().copied().or(p.college),
    }
}

/// Your commitments and signees in a class (people indices).
pub fn class_of(w: &World, prog: usize, class: i32) -> Vec<usize> {
    (0..w.people.len()).filter(|&i| w.people[i].class_of == class && went(&w.people[i]) == Some(prog)).collect()
}

/// A star's composite rating on the services' 100 scale.
fn rating(stars: u8) -> f64 {
    match stars {
        5 => 99.0,
        4 => 92.0,
        3 => 85.0,
        2 => 78.0,
        _ => 70.0,
    }
}

/// Every program's class score for a class (the 247 shape the world uses,
/// `recruiting::class_rankings`): each signee's stars on the services'
/// points, the deeper ones counting less. Best first.
pub fn class_scores(w: &World, class: i32) -> Vec<(f64, usize)> {
    let mut by: Vec<Vec<f64>> = vec![Vec::new(); w.programs.len()];
    for p in w.people.iter().filter(|p| p.class_of == class) {
        if let Some(j) = went(p)
            && j < by.len()
        {
            by[j].push(rating(p.stars));
        }
    }
    let mut v: Vec<(f64, usize)> = by
        .into_iter()
        .enumerate()
        .filter(|(_, v)| !v.is_empty())
        .map(|(j, mut v)| {
            v.sort_by(|a, b| b.total_cmp(a));
            (v.iter().enumerate().map(|(n, r)| r * (-((n as f64).powi(2)) / 200.0).exp()).sum(), j)
        })
        .collect();
    v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    v
}

/// A program's place among some programs' classes (1 the best), None with
/// no class.
fn place(scores: &[(f64, usize)], prog: usize, among: impl Fn(usize) -> bool) -> Option<u32> {
    let mut r = 0u32;
    for &(_, j) in scores {
        if !among(j) {
            continue;
        }
        r += 1;
        if j == prog {
            return Some(r);
        }
    }
    None
}

/// The class's top prospects in a set: stars, then the services' read.
fn ranked(w: &World, class: i32, keep: impl Fn(&crate::world::Person) -> bool) -> Vec<usize> {
    let mut v: Vec<(u8, f64, u32, usize)> = w
        .people
        .iter()
        .enumerate()
        .filter(|(_, p)| p.class_of == class && keep(p))
        .map(|(i, p)| (p.stars, p.cal.service_read.unwrap_or(0.0), p.player.id, i))
        .collect();
    v.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.total_cmp(&a.1)).then(a.2.cmp(&b.2)));
    v.into_iter().map(|x| x.3).collect()
}

fn ordinal(n: u32) -> String {
    let suf = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suf}")
}

/// A rank goal: within `n` Done, a near miss (one place out, or a fifth of
/// `n` past it) half.
fn rank_read(rank: Option<u32>, n: u32, where_: &str) -> Read {
    let Some(r) = rank else { return Read::of(0.0, format!("no class yet {where_}")) };
    let near = n + (n / 5).max(1);
    let share = if r <= n {
        1.0
    } else if r <= near {
        0.5
    } else {
        0.0
    };
    Read::of(share, format!("{} {where_}", ordinal(r)))
}

/// A count goal: have of n.
fn count_read(have: usize, n: u32, what: &str) -> Read {
    let n = n.max(1);
    Read::of(have as f32 / n as f32, format!("{have} of {n} {what}"))
}

/// Your staff grades him a future starter at your level: its file's
/// ceiling read at your level's starter line, else (no report yet) the
/// services' stars at the program's usual line.
pub fn future_starter(w: &World, prog: usize, i: usize) -> bool {
    let bar = starter_bar(w, prog);
    let id = w.people[i].player.id;
    match crate::world::scouting::readers::read(w, crate::world::scouting::readers::Staff::Program(prog), id) {
        Some((_, _, f)) => f.ceiling.0 >= bar,
        None => w.people[i].stars >= star_line(w, prog),
    }
}

/// Graded a fit at his spot: a contributor's ceiling, four under the
/// starter line.
pub fn fits(w: &World, prog: usize, i: usize) -> bool {
    let bar = starter_bar(w, prog) - 4.0;
    let id = w.people[i].player.id;
    match crate::world::scouting::readers::read(w, crate::world::scouting::readers::Staff::Program(prog), id) {
        Some((_, _, f)) => f.ceiling.0 >= bar,
        None => w.people[i].stars + 1 >= star_line(w, prog),
    }
}

/// The ceiling a starter at your level needs (the file's tier words:
/// Power 68, FBS 61, FCS 54, below 48).
fn starter_bar(w: &World, prog: usize) -> f64 {
    let p = &w.programs[prog];
    match p.division {
        Division::FBS if p.power_conference(w.year) => 68.0,
        Division::FBS => 61.0,
        Division::FCS => 54.0,
        _ => 48.0,
    }
}

/// The stars a program usually signs: four at a power program over 0.7
/// prestige, three elsewhere in FBS, two below.
pub fn star_line(w: &World, prog: usize) -> u8 {
    let p = &w.programs[prog];
    match p.division {
        Division::FBS if p.power_conference(w.year) && p.prestige >= 0.7 => 4,
        Division::FBS => 3,
        _ => 2,
    }
}

/// His first college seasons (college levels only), oldest first.
fn college_years(p: &crate::world::Person) -> Vec<&crate::player::SeasonRecord> {
    p.player
        .history
        .iter()
        .filter(|r| !matches!(r.level, crate::player::Level::HS | crate::player::Level::NFL))
        .collect()
}

/// Seasons a class has played in college (0 before it enrolls): it
/// enrolls the fall of its class year; a season counts once its title game
/// is played.
fn played(w: &World, class: i32) -> i32 {
    let done = if w.today > w.dates.title_game { w.year } else { w.year - 1 };
    (done - class + 1).max(0)
}

/// The program's campus area (the area whose center is nearest its campus
/// in its state).
pub fn campus_area(w: &World, prog: usize) -> Option<u16> {
    let c = areas::campus(w, prog);
    let st = w.programs[prog].state;
    areas::table()
        .iter()
        .enumerate()
        .filter(|(_, a)| a.state == st)
        .min_by(|a, b| {
            crate::world::geo::miles_between(c, a.1.center).total_cmp(&crate::world::geo::miles_between(c, b.1.center))
        })
        .map(|(k, _)| k as u16)
}

/// The rival program, when your team has one.
pub fn rival(w: &World, prog: usize) -> Option<usize> {
    let (t, ..) = crate::world::folk::facts::rival(w, crate::world::folk::Team::College(prog))?;
    match t {
        crate::world::folk::Team::College(r) => Some(r),
        _ => None,
    }
}

/// The ten programs with the most prestige (the blue bloods' line).
fn top_ten(w: &World) -> Vec<usize> {
    let mut v: Vec<(f64, usize)> = w
        .programs
        .iter()
        .enumerate()
        .filter(|(_, p)| p.division == Division::FBS)
        .map(|(j, p)| (p.prestige, j))
        .collect();
    v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    v.into_iter().take(10).map(|x| x.1).collect()
}

/// Last season's conference champion (else its best record).
fn champion(w: &World, prog: usize) -> Option<usize> {
    let conf = &w.programs[prog].conference;
    let season = if w.today < w.dates.fall_camp { w.year - 1 } else { w.year };
    let mut best: Option<(bool, f64, usize)> = None;
    for (j, q) in w.programs.iter().enumerate().filter(|(j, q)| &q.conference == conf && *j != prog) {
        let Some(s) = q.seasons.iter().find(|s| s.year == season).or(q.seasons.last()) else { continue };
        let wp = s.wins as f64 / (s.wins + s.losses).max(1) as f64;
        let k = (s.conference_title, wp, j);
        if best.is_none_or(|b| (k.0, k.1) > (b.0, b.1)) {
            best = Some(k);
        }
    }
    best.map(|b| b.2)
}

/// A kid is with you (committed or signed), in words for a named-kid goal.
fn kid_read(w: &World, prog: usize, id: u32) -> Read {
    let Some(i) = crate::world::folk::person(w, id) else { return Read::of(0.0, "gone from the world") };
    let p = &w.people[i];
    match went(p) {
        Some(j) if j == prog => {
            Read::of(1.0, if p.cal.signed.is_some() { "signed with you" } else { "committed to you" })
        }
        Some(j) => Read::of(0.0, format!("committed to {}", w.program_name(j))),
        None => Read::of(0.0, "undecided"),
    }
}

/// One kid among a ranked list: signing the target is Done, the next one
/// down (the state's #2 when the goal was the #1) half.
fn target_read(w: &World, prog: usize, list: &[usize], what: &str) -> Read {
    let Some(&first) = list.first() else { return Read::of(0.0, format!("no {what} this class")) };
    if went(&w.people[first]) == Some(prog) {
        return Read::of(1.0, format!("{} is yours", w.people[first].name));
    }
    if let Some(&second) = list.get(1)
        && went(&w.people[second]) == Some(prog)
    {
        return Read::of(0.5, format!("the next one down, {}, is yours", w.people[second].name));
    }
    let where_ = went(&w.people[first]).map_or("undecided".to_string(), |j| format!("with {}", w.program_name(j)));
    Read::of(0.0, format!("{} is {where_}", w.people[first].name))
}

/// The campaign's tracked flips for a class (`Campaign.flips`): flips in,
/// commitments lost (and to whom).
type Flips = (Vec<(u32, u32)>, Vec<(u32, Option<u32>)>);

fn flips(w: &World, class: i32) -> Flips {
    let c = &w.ground.campaign;
    let ins = c.flipped_in.iter().filter(|x| x.0 == class).map(|x| (x.1, x.2)).collect();
    let lost = c.lost.iter().filter(|x| x.0 == class).map(|x| (x.1, x.2)).collect();
    (ins, lost)
}

/// Reads a goal for one class.
pub fn read(w: &World, prog: usize, a: &Aim, class: i32) -> Read {
    let d = a.def();
    let n = a.n;
    let mine = class_of(w, prog, class);
    let st = w.programs[prog].state;
    let count = |f: &dyn Fn(&crate::world::Person) -> bool| mine.iter().filter(|&&i| f(&w.people[i])).count();
    match d.id {
        1 | 7 => {
            let scores = class_scores(w, class);
            let div = w.programs[prog].division;
            rank_read(place(&scores, prog, |j| w.programs[j].division == div), n, "nationally")
        }
        2 => {
            let scores = class_scores(w, class);
            let conf = &w.programs[prog].conference;
            rank_read(place(&scores, prog, |j| &w.programs[j].conference == conf), n, "in the conference")
        }
        3 => {
            let Some(r) = (a.of as usize).lt(&w.programs.len()).then_some(a.of as usize).or(rival(w, prog)) else {
                return Read::of(0.0, "no rival");
            };
            let scores = class_scores(w, class);
            let s = |j: usize| scores.iter().find(|x| x.1 == j).map_or(0.0, |x| x.0);
            let (me, him) = (s(prog), s(r));
            let share = if me > him {
                1.0
            } else if him > 0.0 && me >= 0.95 * him {
                0.5
            } else {
                0.0
            };
            Read::of(share, format!("{:.0} to {:.0} for {}", me, him, w.program_name(r)))
        }
        4 => {
            let scores = class_scores(w, class);
            let r = place(&scores, prog, |j| {
                w.programs[j].state == st && matches!(w.programs[j].division, Division::FBS | Division::FCS)
            });
            rank_read(r, 1, "in the state")
        }
        5 => count_read(count(&|p| p.stars >= 4), n, "blue-chips"),
        6 => {
            let (mut blue, mut rest) = (0usize, 0usize);
            for c in class - 3..=class {
                for i in class_of(w, prog, c) {
                    match w.people[i].stars {
                        4 | 5 => blue += 1,
                        2 | 3 => rest += 1,
                        _ => {}
                    }
                }
            }
            let ratio = blue as f32 / (blue + rest).max(1) as f32;
            Read::of(ratio / 0.5, format!("{blue} blue-chips to {rest} over four classes"))
        }
        8 => {
            let (_, lost) = flips(w, class);
            let k = lost.len();
            Read::of(
                match k {
                    0 => 1.0,
                    1 => 0.5,
                    _ => 0.0,
                },
                if k == 0 { "no commitment lost".to_string() } else { format!("{k} lost") },
            )
        }
        9 => count_read(mine.len(), n, "commitments"),
        10 => {
            let fit = mine.iter().filter(|&&i| fits(w, prog, i)).count();
            Read::of(
                if mine.is_empty() { 0.0 } else { fit as f32 / mine.len() as f32 },
                format!("{fit} of {} graded fits", mine.len()),
            )
        }
        11..=13 => {
            let top = crate::world::legacy::college::state_top(w, st, class, 2);
            target_read(w, prog, &top, "state #1")
        }
        14 | 15 => {
            let top = crate::world::legacy::college::state_top(w, st, class, 10);
            let ours = top.iter().filter(|&&i| went(&w.people[i]) == Some(prog)).count();
            count_read(ours, n, "of the top 10 home")
        }
        16 => count_read(count(&|p| p.cal.all_state >= 2), n, "first-team all-state"),
        17 => {
            let Some(area) = campus_area(w, prog) else { return Read::of(0.0, "no metro") };
            let top = ranked(w, class, |p| w.high_schools.get(p.hs).is_some_and(|h| h.ground_area == area));
            let ours = top.iter().take(5).filter(|&&i| went(&w.people[i]) == Some(prog)).count();
            count_read(ours, n, "of the city's top 5")
        }
        18 => {
            let c = areas::campus(w, prog);
            let near = (0..w.high_schools.len()).filter(|&h| w.high_schools[h].state == st).min_by(|&a, &b| {
                crate::world::geo::miles_between(c, areas::school_place(w, a))
                    .total_cmp(&crate::world::geo::miles_between(c, areas::school_place(w, b)))
            });
            let Some(h0) = near else { return Read::of(0.0, "no county") };
            let county = w.high_schools[h0].county.clone();
            let top =
                ranked(w, class, |p| w.high_schools.get(p.hs).is_some_and(|h| h.state == st && h.county == county));
            target_read(w, prog, &top, "hometown kid")
        }
        19 => {
            let leader = last_state_leader(w, st, class - 1);
            let instate = |j: usize| class_of(w, j, class).iter().filter(|&&i| w.people[i].home_state == st).count();
            let me = instate(prog);
            match leader {
                Some(l) if l != prog => {
                    let him = instate(l);
                    let share = if me > him {
                        1.0
                    } else if me + 1 >= him && me > 0 {
                        0.5
                    } else {
                        0.0
                    };
                    Read::of(share, format!("{me} in-state to {him} for {}", w.program_name(l)))
                }
                _ => Read::of(1.0, format!("{me} in-state: you led the state last year")),
            }
        }
        20 => {
            let home = count(&|p| p.home_state == st);
            let share = if mine.is_empty() { 0.0 } else { home as f32 / mine.len() as f32 };
            Read::of(share / 0.5, format!("{home} of {} from home", mine.len()))
        }
        21 => {
            let k = count(&|p| p.stars >= 5);
            Read::of((k >= 1) as u8 as f32, format!("{k} five-stars"))
        }
        22..=25 | 33 | 55 | 56 => kid_read(w, prog, a.of),
        26 => count_read(count(&|p| p.stars >= 5), n.max(2), "five-stars"),
        27 => {
            // G8 (not built yet) makes the class's ten biggest followings big
            // news; until then a commitment the country hears is one of them
            // or a national top-10 prospect.
            let loud = ranked(w, class, |_| true).into_iter().take(10).collect::<Vec<_>>();
            let mut fol: Vec<(u32, usize)> = w
                .people
                .iter()
                .enumerate()
                .filter(|(_, p)| p.class_of == class)
                .map(|(i, p)| (p.followers, i))
                .collect();
            fol.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
            let hype: Vec<usize> = fol.into_iter().take(10).map(|x| x.1).collect();
            let hit = mine.iter().find(|i| loud.contains(i) || hype.contains(i));
            match hit {
                Some(&i) => Read::of(1.0, format!("{}'s commitment", w.people[i].name)),
                None => Read::of(0.0, "no commitment the country heard"),
            }
        }
        28 | 30 => {
            let pos = if d.id == 28 { Position::QB } else { Position::OT };
            let list = ranked(w, class, |p| p.player.position == pos);
            let best = mine.iter().filter_map(|i| list.iter().position(|x| x == i)).min();
            rank_read(best.map(|r| r as u32 + 1), n.max(1), &format!("at {} in the class", pos.name()))
        }
        29 => {
            let k = mine
                .iter()
                .filter(|&&i| w.people[i].player.position == Position::QB && future_starter(w, prog, i))
                .count();
            Read::of((k >= 1) as u8 as f32, format!("{k} quarterback graded a future starter"))
        }
        31 => {
            let mut all = Vec::new();
            for c in a.class..=class {
                all.extend(class_of(w, prog, c));
            }
            let at = |pos: Position| {
                all.iter().filter(|&&i| w.people[i].player.position == pos && future_starter(w, prog, i)).count()
            };
            let (t, g) = (at(Position::OT).min(1), at(Position::IOL).min(2));
            Read::of((t + g) as f32 / 3.0, format!("{t} tackle and {g} interior linemen graded starters"))
        }
        32 => {
            let at = |pos: Position| {
                mine.iter().filter(|&&i| w.people[i].player.position == pos && future_starter(w, prog, i)).count()
            };
            let (c, s) = (at(Position::CB).min(2), at(Position::S).min(1));
            Read::of((c + s) as f32 / 3.0, format!("{c} corners and {s} safety graded starters"))
        }
        34 | 36 | 45 => {
            if played(w, class) < 1 {
                return Read::wait("judged after his first season");
            }
            let first_start = |p: &crate::world::Person| {
                college_years(p).first().is_some_and(|r| r.playing_time == crate::player::PlayingTime::Starter)
            };
            let plays = |p: &crate::world::Person| {
                college_years(p).first().is_some_and(|r| {
                    matches!(r.playing_time, crate::player::PlayingTime::Starter | crate::player::PlayingTime::Rotation)
                })
            };
            let k = match d.id {
                34 => count(&|p| matches!(p.player.position, Position::K | Position::P) && first_start(p)),
                36 => count(&|p| {
                    matches!(p.player.position, Position::RB | Position::WR | Position::CB | Position::S) && plays(p)
                }),
                _ => count(&|p| p.stars <= 2 && first_start(p)),
            };
            Read::of((k >= 1) as u8 as f32, format!("{k} as a freshman"))
        }
        35 => {
            let pos = Position::ALL.get(a.of as usize).copied().unwrap_or(Position::QB);
            let top = ranked(w, class, |p| p.home_state == st && p.player.position == pos);
            let ours = top.iter().take(3).filter(|&&i| went(&w.people[i]) == Some(prog)).count();
            count_read(ours, n.max(1), &format!("of the state's top 3 at {}", pos.name()))
        }
        37 => count_read(
            count(&|p| p.player.position == Position::DT && p.player.body.weight >= 300.0),
            n,
            "defensive tackles at 300 pounds",
        ),
        38 => {
            let Some(r) = (a.of as usize).lt(&w.programs.len()).then_some(a.of as usize).or(rival(w, prog)) else {
                return Read::of(0.0, "no rival");
            };
            let both = |p: &crate::world::Person| {
                let has = |j: usize| p.cal.offers.contains(&j) || p.cal.early_offers.iter().any(|x| x.0 == j);
                has(prog) && has(r)
            };
            let won = w.people.iter().filter(|p| p.class_of == class && both(p) && went(p) == Some(prog)).count();
            count_read(won, n, &format!("won against {}", w.program_name(r)))
        }
        39 | 40 => {
            let (ins, _) = flips(w, class);
            let blue = top_ten(w);
            let k = ins
                .iter()
                .filter(|(kid, from)| {
                    d.id == 39
                        || (blue.contains(&(*from as usize))
                            && crate::world::folk::person(w, *kid).is_some_and(|i| w.people[i].stars >= 4))
                })
                .filter(|(kid, _)| {
                    crate::world::folk::person(w, *kid).is_some_and(|i| went(&w.people[i]) == Some(prog))
                })
                .count();
            Read::of((k >= 1) as u8 as f32, format!("{k} flipped to you"))
        }
        41 => {
            let Some(r) = (a.of as usize).lt(&w.programs.len()).then_some(a.of as usize).or(rival(w, prog)) else {
                return Read::of(0.0, "no rival");
            };
            let rs = w.programs[r].state;
            count_read(count(&|p| p.home_state == rs), n, "from his state")
        }
        42 => {
            let (_, lost) = flips(w, class);
            let k = lost.iter().filter(|x| x.1.is_some()).count();
            Read::of(
                match k {
                    0 => 1.0,
                    1 => 0.5,
                    _ => 0.0,
                },
                if k == 0 { "nobody flipped".to_string() } else { format!("{k} flipped away") },
            )
        }
        43 => {
            let Some(c) = champion(w, prog) else { return Read::of(0.0, "no champion") };
            let scores = class_scores(w, class);
            let s = |j: usize| scores.iter().find(|x| x.1 == j).map_or(0.0, |x| x.0);
            let (me, him) = (s(prog), s(c));
            let share = if me > him {
                1.0
            } else if him > 0.0 && me >= 0.95 * him {
                0.5
            } else {
                0.0
            };
            Read::of(share, format!("{:.0} to {:.0} for {}", me, him, w.program_name(c)))
        }
        44 => {
            let finds: Vec<&crate::world::ground::finds::Find> =
                w.ground.finds.iter().filter(|f| f.prog as usize == prog && f.class_of == class).collect();
            let hit = finds.iter().find(|f| {
                crate::world::folk::person(w, f.kid)
                    .is_some_and(|i| went(&w.people[i]) == Some(prog) || w.people[i].stars > f.stars)
            });
            match hit {
                Some(f) => Read::of(1.0, format!("{}, your find", f.name)),
                None if finds.is_empty() => Read::of(0.0, "no find yet"),
                None => Read::of(0.0, format!("{} finds, none committed or rising yet", finds.len())),
            }
        }
        46 => {
            if played(w, class) < 1 {
                return Read::wait("judged over his first three seasons");
            }
            let k = count(&|p| p.stars <= 2 && w.stats.honors.get(&p.player.id).is_some_and(|h| h.all_conference > 0));
            if k == 0 && played(w, class) < 3 {
                return Read::wait("no two-star all-conference yet");
            }
            Read::of((k >= 1) as u8 as f32, format!("{k} two-star all-conference"))
        }
        47 => {
            let drafted_here = w
                .alumni
                .iter()
                .filter(|al| {
                    al.class_of == class && al.colleges.first() == Some(&prog) && al.stars <= 2 && al.pick.is_some()
                })
                .count();
            let drafted_now = count(&|p| p.stars <= 2 && p.pro.as_ref().is_some_and(|x| x.draft.is_some()));
            let k = drafted_here + drafted_now;
            if k == 0 && played(w, class) < 4 {
                return Read::wait("judged at his draft");
            }
            Read::of((k >= 1) as u8 as f32, format!("{k} unranked kids drafted"))
        }
        48 => {
            if played(w, class) < 3 {
                return Read::wait("judged after the class's third season");
            }
            let k = count(&|p| {
                college_years(p).iter().take(3).any(|r| r.playing_time == crate::player::PlayingTime::Starter)
            });
            count_read(k, n, "starting by year three")
        }
        49 => {
            let k = w
                .ground
                .finds
                .iter()
                .filter(|f| f.prog as usize == prog && f.class_of == class && f.stars <= 3)
                .filter(|f| crate::world::folk::person(w, f.kid).is_some_and(|i| w.people[i].stars >= 4))
                .count();
            count_read(k, n, "who became four-stars")
        }
        50 => {
            let sum: u64 = mine.iter().map(|&i| w.people[i].followers as u64).sum();
            Read::of(sum as f32 / n.max(1) as f32, format!("{} followers", thousands(sum)))
        }
        51 => {
            let div = w.programs[prog].division;
            let ok = mine
                .iter()
                .filter(|&&i| {
                    let p = &w.people[i];
                    let g = crate::world::recruiting::eligibility_gpa(
                        crate::world::scouting::file::gpa(p),
                        p.cal.test_score,
                        class,
                    );
                    crate::world::recruiting::ncaa_tier(div, g) == crate::world::recruiting::Eligibility::Qualifier
                })
                .count();
            let out = mine.len() - ok;
            Read::of(
                if out == 0 { 1.0 } else { ok as f32 / mine.len().max(1) as f32 * 0.6 },
                if out == 0 { "every one qualifies".to_string() } else { format!("{out} short of the academic line") },
            )
        }
        52 => {
            if played(w, class) < 5 {
                return Read::wait("judged after five years");
            }
            let grad = count(&|p| p.cal.graduated || p.cal.may_graduate);
            Read::of(grad as f32 / mine.len().max(1) as f32 / 0.75, format!("{grad} of {} graduated", mine.len()))
        }
        53 => {
            let ids: Vec<u32> = mine.iter().map(|&i| w.people[i].player.id).collect();
            let courts = &w.folk.courts;
            let bad = mine.iter().filter(|&&i| w.people[i].on_notice).count()
                + courts.open.iter().chain(courts.closed.iter()).filter(|c| ids.contains(&c.id)).count();
            Read::of(
                if bad == 0 { 1.0 } else { (1.0 - bad as f32 / mine.len().max(1) as f32).min(0.5) },
                if bad == 0 { "nobody in trouble".to_string() } else { format!("{bad} in trouble") },
            )
        }
        54 => {
            let pot = w.programs[prog].collective.budget;
            let paid: f64 = mine.iter().map(|&i| w.people[i].nil_income).sum();
            if pot <= 0.0 || paid <= pot {
                Read::of(1.0, "inside the pot")
            } else {
                Read::of((pot / paid) as f32 * 0.6, "past the pot")
            }
        }
        57 => {
            let h = a.of as usize;
            let mut k = 0;
            for c in a.class..=class {
                k += class_of(w, prog, c).iter().filter(|&&i| w.people[i].hs == h).count();
            }
            count_read(k, n, "from the school")
        }
        58 => {
            let s = a.of as usize;
            let mut k = 0;
            for c in a.class..=class {
                k += class_of(w, prog, c).iter().filter(|&&i| w.people[i].home_state == s).count();
            }
            count_read(k, n, "from the state")
        }
        59..=61 => area_read(w, prog, a),
        62 => {
            if played(w, class) < 1 {
                return Read::wait("judged after their first season with you");
            }
            let k = w
                .people
                .iter()
                .filter(|p| p.juco && p.path.iter().position(|&j| j == prog).is_some_and(|k| k > 0))
                .filter(|p| p.class_of >= class - 2 && p.class_of <= class)
                .filter(|p| p.last_role == Some(crate::player::PlayingTime::Starter))
                .count();
            count_read(k, n, "junior college signees starting")
        }
        63 => {
            let men: Vec<usize> = crate::world::ground::week::road::men(w, prog)
                .into_iter()
                .filter(|&m| Some(m) != w.programs[prog].head_coach)
                .collect();
            if men.is_empty() {
                return Read::of(0.0, "no assistants");
            }
            let signed = men
                .iter()
                .filter(|&&m| {
                    mine.iter().any(|&i| {
                        crate::world::scouting::dept::area_coach(w, prog, w.people[i].home_state as u8) == Some(m)
                    })
                })
                .count();
            Read::of(signed as f32 / men.len() as f32, format!("{signed} of {} coaches", men.len()))
        }
        _ => Read::of(0.0, ""),
    }
}

fn thousands(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1} million", n as f64 / 1e6)
    } else if n >= 1000 {
        format!("{}K", n / 1000)
    } else {
        n.to_string()
    }
}

/// The program with the most in-state signees in a class.
fn last_state_leader(w: &World, st: usize, class: i32) -> Option<usize> {
    let mut by = vec![0u32; w.programs.len()];
    for p in w.people.iter().filter(|p| p.class_of == class && p.home_state == st) {
        if let Some(j) = went(p) {
            by[j] += 1;
        }
    }
    for al in w.alumni.iter().filter(|a| a.class_of == class && a.home_state == st) {
        if let Some(&j) = al.colleges.first() {
            by[j] += 1;
        }
    }
    by.iter().enumerate().filter(|x| *x.1 > 0).max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(&a.0))).map(|x| x.0)
}

/// An area goal now: open (Known, 25 points), take or hold (its leader).
/// A near miss on the one target counts half: 20 points toward Known, or
/// within 5 points of the leader.
pub fn area_read(w: &World, prog: usize, a: &Aim) -> Read {
    let id = a.of as u16;
    let Some(area) = areas::table().get(id as usize) else { return Read::of(0.0, "no area") };
    let talent = standing::talents(w);
    let schools: Vec<usize> = (0..w.high_schools.len()).filter(|&h| w.high_schools[h].ground_area == id).collect();
    let me = standing::area_points(w, prog, &schools, &talent);
    let level = standing::Level::of(me);
    if a.goal == 61 {
        let share = if me >= 25.0 {
            1.0
        } else if me >= 20.0 {
            0.5
        } else {
            0.0
        };
        return Read::of(share, format!("{} {}", area.name, level.name()));
    }
    match standing::leader(w, &schools, &talent) {
        Some((l, _)) if l == prog => Read::of(1.0, format!("you lead {} ({})", area.name, level.name())),
        Some((l, pts)) => {
            let share = if me + 5.0 >= pts { 0.5 } else { 0.0 };
            Read::of(share, format!("{} leads {}, you're {}", w.program_name(l), area.name, level.name()))
        }
        None => Read::of(0.0, format!("nobody leads {}", area.name)),
    }
}

/// A goal's share over its classes so far: each class part read now and
/// averaged over every class it runs (two of three state #1s is Mostly).
/// Parts still to come count nothing yet.
pub fn share_over(parts: &[f32], classes: u8) -> f32 {
    parts.iter().sum::<f32>() / classes.max(1) as f32
}

/// How likely a goal looks today for its class (GR41), 0.05-0.95: the
/// same goal read on the program's last class (the way `terms::likely`
/// reads last season), blended toward today's reading as the spring goes
/// by (`t`: 0 at the February meeting, 1 at the verdict). A named kid
/// reads where you stand in his race; an area its gap to the leader.
pub fn likely(w: &World, prog: usize, a: &Aim, t: f32) -> f32 {
    let d = a.def();
    let now = if d.later { 0.0 } else { read(w, prog, a, a.class).share };
    let prior = match d.subj {
        Subj::Kid => {
            let race = crate::world::front_office::college::race(w, a.of);
            match race.iter().position(|x| x.0 == prog) {
                _ if now >= 1.0 => 0.7,
                Some(0) => 0.55,
                Some(1) => 0.3,
                Some(_) => 0.15,
                None => 0.08,
            }
        }
        Subj::Area => {
            let r = area_read(w, prog, a);
            0.15 + 0.6 * r.share
        }
        _ => {
            let last = Aim { class: a.class - 1, ..*a };
            let r = read(w, prog, &last, last.class);
            if r.waiting { 0.4 } else { 0.15 + 0.7 * r.share }
        }
    };
    let t = t.clamp(0.0, 1.0);
    (prior * (1.0 - t) + now * t).clamp(0.05, 0.95)
}
