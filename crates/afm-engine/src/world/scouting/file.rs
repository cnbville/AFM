//! The file (topic 1): a staff's whole knowledge of one player, rebuilt
//! from the looks it took. Each person who looked keeps his own lasting
//! lean on each attribute (about 60% of his misread, the rest the noise of
//! the look, which shrinks with more looks by him); different people's
//! leans average out, weighted by their skill. Each kind of look shows
//! some attributes and not others (topic 4); tests measure the test, read
//! by how well the staff reads tests; a staff's biases tilt it by up to a
//! third of a misread (topic 2). Ranges come out about 6-8 points wide on
//! a well-scouted senior and 15 or more on a lightly seen small-school
//! player (proposed, not confirmed).
//!
//! The per-attribute leans share the category lean the AI clubs' boards
//! use (`pro::scouts::read`), so what your file shows and how your board
//! grades the player agree.
//!
//! The same file holds the character read (topic 5), the medical read
//! (topic 6) and the potential read in words with hints and comparisons
//! (topic 7). Nothing here leaves the engine as a true number: ranges,
//! words and how sure.

use super::Kind;
use crate::attributes::{ATTRIBUTE_COUNT, Attr, Category};
use crate::calendar::Date;
use crate::positions::{Position, overall_for, weight};
use crate::rng::clamp;
use crate::world::World;
use crate::world::pro::scouts::Bias;
use crate::world::season::key_normal;

const LEAN: f64 = 0.6;
const NOISE: f64 = 0.4;
const MIX_SD: f64 = 0.7211;

/// How sharp a rating makes a read (10 is 1.0, 20 is 0.6, 1 is 1.36).
pub fn sharp(r: f64) -> f64 {
    1.4 - 0.04 * r
}

/// Who a reader is, for the screens' links.
#[derive(Clone, Debug, PartialEq)]
pub enum Link {
    /// A scout's place in an NFL club's department.
    Scout(u8),
    /// A coach (index into the world's coaches).
    Coach(usize),
    /// An NFL club's GM.
    Gm,
    /// You.
    You,
    /// A support or personnel staffer (named, no page).
    Staff,
    /// A shared measurement (the combine, a pro day, a showcase).
    Event,
}

/// A person who reads players for a staff, with the ratings that matter
/// for reading (hidden; only his work shows them).
#[derive(Clone, Debug)]
pub struct Reader {
    pub key: u64,
    pub name: String,
    pub job: String,
    pub link: Link,
    pub judge: f64,
    pub potential: f64,
    pub athletic: f64,
    pub character: f64,
    pub network: f64,
    pub work: f64,
    /// His weight in the room.
    pub say: f64,
    pub bias: Option<Bias>,
    pub home: u8,
    /// An area scout's contacts (SK9): his region and how deep they run
    /// (0-1): coaches, trainers and people around the programs who tell
    /// him what they know.
    pub contacts: Option<(u8, f64)>,
}

impl Reader {
    pub fn event(name: &str) -> Reader {
        Reader {
            key: 0x3EA5,
            name: name.to_string(),
            job: "Measured".to_string(),
            link: Link::Event,
            judge: 10.0,
            potential: 10.0,
            athletic: 10.0,
            character: 10.0,
            network: 10.0,
            work: 10.0,
            say: 1.0,
            bias: None,
            home: 99,
            contacts: None,
        }
    }

    /// How many days after the look his report is written (1 to 3 weeks,
    /// faster with a hard worker; measurements at once).
    pub fn lag(&self, kind: Kind, id: u32, date: Date) -> i32 {
        if kind.instant() || self.link == Link::Event {
            return 0;
        }
        let base = 7.0 + 0.7 * (20.0 - self.work.clamp(1.0, 20.0));
        let jitter = 3.0 * key_normal(self.key ^ 0x1A6, id as u64 ^ (date.0 as u64) << 32);
        clamp(base + jitter, 4.0, 21.0).round() as i32
    }
}

/// One look in a file.
#[derive(Clone, Copy, Debug)]
pub struct Evidence {
    /// Index into the file's readers.
    pub reader: usize,
    pub kind: Kind,
    pub date: Date,
}

// --- What each look shows (topic 4) -------------------------------------------------------

fn measured_attr(a: Attr) -> bool {
    matches!(a, Attr::Speed | Attr::Acceleration | Attr::Agility | Attr::Strength | Attr::Jumping)
}

/// Technique a drill shows: footwork, hands, throws, kicks.
fn drill_attr(a: Attr) -> bool {
    matches!(
        a,
        Attr::Footwork
            | Attr::QuickRelease
            | Attr::ArmStrength
            | Attr::ShortAccuracy
            | Attr::MediumAccuracy
            | Attr::DeepAccuracy
            | Attr::Touch
            | Attr::ThrowOnMove
            | Attr::BallHandling
            | Attr::RouteRunning
            | Attr::Release
            | Attr::Catching
            | Attr::BallTracking
            | Attr::BodyControl
            | Attr::HandUse
            | Attr::Press
            | Attr::BallSecurity
            | Attr::Flexibility
            | Attr::Balance
    ) || a.category() == Category::KickingAndSnapping
}

/// The mental side film shows: what he sees, how hard he plays.
fn film_mind(a: Attr) -> bool {
    matches!(
        a,
        Attr::PlayRecognition
            | Attr::Anticipation
            | Attr::Decisions
            | Attr::Discipline
            | Attr::Motor
            | Attr::Competitiveness
            | Attr::Physicality
            | Attr::Toughness
            | Attr::Concentration
    )
}

/// The mental side a conversation shows.
fn talk_mind(a: Attr) -> bool {
    matches!(a, Attr::FootballIq | Attr::Communication | Attr::Composure)
}

/// The base misread (rating points, an average eye at FBS level) of an
/// attribute on a kind of look, or None if it can't see it.
pub fn sees(kind: Kind, a: Attr) -> Option<f64> {
    let cat = a.category();
    let phys = cat == Category::Physical;
    let mental = cat == Category::Mental;
    let film = |p: f64, t: f64, fm: f64| {
        if phys {
            Some(if measured_attr(a) { p } else { p - 0.5 })
        } else if mental {
            if film_mind(a) {
                Some(fm)
            } else if a == Attr::Composure {
                Some(fm + 0.5)
            } else if a == Attr::FootballIq {
                Some(fm + 2.5)
            } else {
                None
            }
        } else {
            Some(t)
        }
    };
    match kind {
        Kind::Film
        | Kind::HsFilm
        | Kind::CollegeFilm
        | Kind::ProFilm
        | Kind::CrossCheck
        | Kind::Director
        | Kind::Gm => film(8.0, 7.0, 7.5),
        Kind::Coach => film(8.0, 6.5, 7.5),
        Kind::School => {
            if mental {
                Some(if talk_mind(a) { 9.0 } else { 7.0 })
            } else {
                film(6.5, 6.5, 7.0)
            }
        }
        Kind::AllStar => {
            if mental {
                Some(if talk_mind(a) { 8.5 } else { 7.0 })
            } else {
                film(6.0, 6.0, 7.0)
            }
        }
        Kind::Practice | Kind::Game => {
            if phys {
                Some(if measured_attr(a) { 5.5 } else { 6.0 })
            } else if mental {
                Some(match a {
                    Attr::Motor | Attr::Competitiveness | Attr::Toughness | Attr::Physicality => 6.5,
                    Attr::Communication | Attr::Composure => 7.5,
                    Attr::FootballIq => 9.5,
                    _ => 7.5,
                })
            } else {
                Some(7.5)
            }
        }
        Kind::Camp | Kind::Tryout | Kind::Workout | Kind::LocalDay => {
            if phys {
                Some(if measured_attr(a) { 4.5 } else { 6.0 })
            } else if mental {
                match a {
                    Attr::Competitiveness | Attr::Motor => Some(7.0),
                    Attr::FootballIq | Attr::Communication if kind != Kind::Camp => Some(8.0),
                    _ => None,
                }
            } else if drill_attr(a) {
                Some(if kind == Kind::Workout { 5.5 } else { 6.0 })
            } else {
                Some(8.5)
            }
        }
        Kind::Showcase => {
            if phys {
                Some(if measured_attr(a) { 5.0 } else { 6.0 })
            } else if mental {
                film_mind(a).then_some(7.5)
            } else {
                Some(6.5)
            }
        }
        Kind::Combine | Kind::ProDay => {
            if drill_attr(a) && !phys {
                Some(7.5)
            } else if kind == Kind::Combine && matches!(a, Attr::FootballIq | Attr::PlayRecognition) {
                // The cognitive test (the Wonderlic until 2022, then clubs' own).
                Some(9.0)
            } else {
                None
            }
        }
        Kind::Interview => match a {
            Attr::FootballIq | Attr::Communication => Some(7.0),
            Attr::Composure => Some(8.0),
            Attr::PlayRecognition => Some(8.5),
            Attr::Decisions => Some(9.5),
            _ => None,
        },
        Kind::Visit | Kind::JuniorDay | Kind::Official | Kind::Unofficial | Kind::HomeVisit => match a {
            Attr::FootballIq | Attr::Communication => Some(if kind == Kind::Visit { 7.5 } else { 8.5 }),
            Attr::Composure => Some(8.5),
            _ => None,
        },
        Kind::Call => match a {
            Attr::Motor | Attr::Competitiveness | Attr::Concentration | Attr::Discipline => Some(9.0),
            Attr::FootballIq => Some(9.5),
            _ => None,
        },
        Kind::OldStaff => match a {
            Attr::Motor | Attr::Discipline | Attr::Concentration => Some(8.5),
            Attr::FootballIq => Some(9.0),
            _ => None,
        },
        // The numbers' reach depends on his snaps (`footage::numbers_see`).
        Kind::Medical | Kind::Background | Kind::Social | Kind::Transcript | Kind::Numbers => None,
    }
}

/// A shared measurement's error on an attribute (rating points): the
/// combine's electronic times, a pro day's hand times, a camp's lasers.
pub fn measures(kind: Kind, a: Attr) -> Option<f64> {
    let base = match a {
        Attr::Speed => 1.5,
        Attr::Acceleration => 2.0,
        Attr::Jumping => 2.0,
        Attr::Agility => 2.5,
        Attr::Strength => 2.5,
        _ => return None,
    };
    match kind {
        Kind::Combine => Some(base),
        Kind::ProDay => Some(base + 1.0),
        // Camps time the 40, the shuttle and the jumps; few bench.
        Kind::Camp | Kind::Showcase if a != Attr::Strength => Some(base + 1.5),
        Kind::LocalDay | Kind::Workout | Kind::Tryout => Some(base + 2.0),
        _ => None,
    }
}

// --- The thirteen traits (section 6) ----------------------------------------------------

pub const TRAITS: usize = 13;
pub const TRAIT_NAMES: [&str; TRAITS] = [
    "Work ethic",
    "Coachability",
    "Competitiveness",
    "Love of the game",
    "Ambition",
    "Loyalty",
    "Money",
    "Temper",
    "Ego",
    "Leadership",
    "Professionalism",
    "Adaptability",
    "Handling setbacks",
];

/// A trait's true value, 1-20 (temper, ego and money: higher is more).
pub(crate) fn trait_value(p: &crate::world::Person, k: usize) -> f64 {
    use crate::player::Trait;
    let per = &p.player.personality;
    let t = &per.traits;
    match k {
        0 => per.work_ethic,
        1 => per.coachability,
        2 => per.competitiveness,
        3 => per.love_of_game,
        4 => t.get(Trait::Ambition),
        5 => t.get(Trait::Loyalty),
        6 => t.get(Trait::Greed),
        7 => t.get(Trait::Temper),
        8 => t.get(Trait::Ego),
        9 => t.get(Trait::Leadership),
        10 => t.get(Trait::Professionalism),
        11 => t.get(Trait::Adaptability),
        _ => p.bg.stability,
    }
}

/// Which traits a kind of look shows, and its base error in trait points
/// (1-20 scale).
fn trait_sees(kind: Kind) -> (&'static [usize], f64) {
    match kind {
        Kind::School => (&[0, 1, 10, 9, 7, 3, 2], 3.0),
        Kind::Interview => (&[8, 9, 12, 4, 11, 3, 6, 1], 2.6),
        Kind::Visit | Kind::LocalDay => (&[8, 9, 12, 4, 11, 3, 6], 2.4),
        Kind::Background => (&[7, 10, 12, 5, 6], 2.2),
        Kind::Social => (&[8, 7, 6, 10], 3.0),
        Kind::Practice | Kind::Game => (&[2, 1, 0, 7, 9], 3.0),
        Kind::Camp | Kind::Workout | Kind::Tryout => (&[1, 2, 0], 3.2),
        Kind::JuniorDay | Kind::Official | Kind::Unofficial => (&[8, 5, 4, 6, 12, 11], 2.8),
        Kind::HomeVisit => (&[5, 4, 6, 12, 8, 10], 2.4),
        Kind::Call => (&[0, 1, 10, 9, 2], 3.0),
        Kind::OldStaff => (&[0, 1, 10, 5, 7], 3.0),
        Kind::AllStar => (&[2, 1, 9, 0], 3.0),
        Kind::Director | Kind::Gm => (&[0, 2, 3, 1], 3.2),
        Kind::Coach => (&[2, 3, 1], 3.4),
        Kind::Film | Kind::HsFilm | Kind::CollegeFilm | Kind::ProFilm | Kind::CrossCheck => (&[2, 3], 4.0),
        Kind::Transcript => (&[10], 4.0),
        Kind::Combine | Kind::ProDay | Kind::Medical | Kind::Showcase | Kind::Numbers => (&[], 9.0),
    }
}

/// How a source shades what it says (topic 5): the position coach runs
/// warm, the trainer and equipment man true, a parent defends, a rival
/// school's staff or one he left runs cold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shade {
    Warm,
    True,
    Defends,
    Cold,
}

impl Shade {
    pub fn words(self) -> &'static str {
        match self {
            Shade::Warm => "runs warm",
            Shade::True => "runs true",
            Shade::Defends => "defends him",
            Shade::Cold => "runs cold",
        }
    }
    /// The tilt toward the good side, in trait points.
    fn tilt(self) -> f64 {
        match self {
            Shade::Warm => 1.5,
            Shade::True => 0.0,
            Shade::Defends => 2.0,
            Shade::Cold => -1.6,
        }
    }
}

/// Whether a trait reads better high (work ethic) or low (temper, ego, money).
pub fn trait_good_high(k: usize) -> bool {
    !matches!(k, 6..=8)
}

fn kind_shade(kind: Kind) -> Shade {
    match kind {
        Kind::Call | Kind::Practice | Kind::Game => Shade::Warm,
        Kind::HomeVisit | Kind::Official | Kind::Unofficial | Kind::JuniorDay => Shade::Defends,
        Kind::OldStaff => Shade::Cold,
        _ => Shade::True,
    }
}

// --- The read ---------------------------------------------------------------------------

/// One attribute's read: where the staff puts him and how sure (rating
/// points), with the kinds of looks behind it.
#[derive(Clone, Copy, Debug, Default)]
pub struct AttrRead {
    pub centre: f64,
    pub sd: f64,
    pub seen: bool,
    pub chips: u8,
    pub readers: u8,
}

impl AttrRead {
    /// The range on the 1-99 scale.
    pub fn range(&self) -> Option<(u8, u8)> {
        if !self.seen {
            return None;
        }
        let half = self.sd.max(1.0);
        let lo = (self.centre - half).floor().clamp(1.0, 99.0) as u8;
        let hi = (self.centre + half).ceil().clamp(1.0, 99.0) as u8;
        Some((lo, hi.max(lo)))
    }
}

/// A trait read: the staff's centre and how sure, the source it leans on
/// most, and the readers who saw it.
#[derive(Clone, Copy, Debug, Default)]
pub struct TraitRead {
    pub centre: f64,
    pub sd: f64,
    pub seen: bool,
    /// The look (index into the evidence) that weighs most.
    pub main: usize,
}

/// The medical read (topic 6).
#[derive(Clone, Debug, Default)]
pub struct MedicalRead {
    /// 0 cleared, 1 minor concern, 2 major concern, 3 failed.
    pub grade: u8,
    pub examined: bool,
    pub concern: f64,
    pub notes: Vec<String>,
}

/// Context a read needs about the class (for the biases).
#[derive(Clone, Copy, Debug, Default)]
pub struct Norms {
    pub z_size: f64,
    pub z_speed: f64,
    pub small_school: bool,
    pub region: u8,
    pub home: u8,
    pub conf_key: u64,
}

/// What a file's looks add to the tape (SK4, SK5): whether he has been
/// seen against good players (an all-star week, the combine, a showcase),
/// and whether all the staff has is his highlight reel.
pub fn tape_flags(looks: &[Evidence]) -> (bool, bool) {
    let seen_vs_good = looks.iter().any(|l| matches!(l.kind, Kind::AllStar | Kind::Combine | Kind::Showcase));
    let reel_only = !looks.iter().any(|l| {
        matches!(l.kind, Kind::Game | Kind::Practice | Kind::Call | Kind::Camp | Kind::Showcase | Kind::Workout)
    }) && looks.iter().filter(|l| l.kind == Kind::HsFilm).count() < 3;
    (seen_vs_good, reel_only)
}

/// The whole read of a file.
#[derive(Clone, Debug)]
pub struct FileRead {
    pub attrs: Vec<AttrRead>,
    pub traits: Vec<TraitRead>,
    /// Now (overall at his position) and its sd.
    pub now: (f64, f64),
    /// Where he could get to (overall at his position), and its sd.
    pub ceiling: (f64, f64),
    pub medical: MedicalRead,
    /// Each reader's own read of each attribute: (centre, sd) or None.
    pub per_reader: Vec<Vec<Option<(f64, f64)>>>,
    /// Every attribute's centre the overall is read from (an unseen one at
    /// the wide prior), for reading him at another position.
    pub centres: crate::attributes::Attributes,
}

/// The year a look's noise belongs to: a new football year, a new draw.
fn noise_year(d: Date) -> u64 {
    d.year() as u64
}

/// How well a level's film shows a player (the built level visibility):
/// FBS and pros clearly, smaller schools and high schools less, and
/// small-school high school tape thin before Hudl (about 2008).
pub fn visibility(w: &World, i: usize) -> f64 {
    use crate::world::{Stage, data::Division};
    let p = &w.people[i];
    match p.stage {
        Stage::Pro => 0.7,
        Stage::College | Stage::Declared => match p.college.map(|c| w.programs[c].division) {
            Some(Division::FBS) | None => 1.0,
            Some(Division::FCS) => 1.2,
            _ => 1.5,
        },
        _ => {
            let small = w.high_schools.get(p.hs).is_some_and(|h| h.classification <= 2);
            if w.year < 2008 && small { 1.55 } else { 1.25 }
        }
    }
}

/// Reads a file: each attribute, each trait, the overall now and where he
/// could get to, the medical read. `upto` limits the read to the first
/// looks (for "how this range narrowed"). `today` drops looks whose
/// reports aren't written yet.
pub fn read(
    w: &World,
    i: usize,
    readers: &[Reader],
    looks: &[Evidence],
    norms: &Norms,
    medical_rating: f64,
    today: Option<Date>,
) -> FileRead {
    let p = &w.people[i];
    let id = p.player.id as u64;
    let pos = p.player.position;
    let vis = visibility(w, i);
    let looks: Vec<Evidence> = looks
        .iter()
        .copied()
        .filter(|l| today.is_none_or(|t| l.date.add(readers[l.reader].lag(l.kind, p.player.id, l.date)) <= t))
        .collect();
    // What his tape holds (SK1-SK6): how far film narrows him, the skills
    // his role never tested, and the shared leans every staff reads.
    let foot = super::footage::footage(w, i);
    let film_f = super::footage::film_factor(&foot);
    let (seen_vs_good, reel_only) = tape_flags(&looks);
    // Each reader's looks that see each attribute: count and best base sd.
    let n_r = readers.len();
    let mut count = vec![[0u8; ATTRIBUTE_COUNT]; n_r];
    let mut best = vec![[f64::MAX; ATTRIBUTE_COUNT]; n_r];
    let mut chips = [0u8; ATTRIBUTE_COUNT];
    let mut meas: [Option<(f64, Kind)>; ATTRIBUTE_COUNT] = [None; ATTRIBUTE_COUNT];
    let mut year = [0u64; 64];
    let (mut numbers, mut tape, mut all) = ([0u16; 64], [0u16; 64], [0u16; 64]);
    for l in &looks {
        if l.reader < 64 {
            year[l.reader] = year[l.reader].max(noise_year(l.date));
        }
        for a in Attr::ALL {
            let k = a.index();
            if let Some(sd) = measures(l.kind, a)
                && (readers[l.reader].link == Link::Event
                    || matches!(l.kind, Kind::Camp | Kind::Workout | Kind::LocalDay | Kind::Tryout))
            {
                if meas[k].is_none_or(|m| sd < m.0) {
                    meas[k] = Some((sd, l.kind));
                }
                chips[k] |= l.kind.chip();
            }
            if readers[l.reader].link == Link::Event {
                continue;
            }
            let seen = if l.kind == Kind::Numbers {
                super::footage::numbers_see(&foot, a, pos).then(|| super::footage::analytics_sd(&foot))
            } else if l.kind.tape() && !super::footage::role_seen(&foot, a, pos) {
                None
            } else {
                sees(l.kind, a).map(|sd| if l.kind.tape() && l.kind != Kind::Game { sd * film_f } else { sd })
            };
            if let Some(sd) = seen {
                count[l.reader][k] = count[l.reader][k].saturating_add(1);
                best[l.reader][k] = best[l.reader][k].min(sd);
                chips[k] |= l.kind.chip();
            }
        }
        if l.reader < 64 {
            if l.kind == Kind::Numbers {
                numbers[l.reader] += 1;
            } else if l.kind.tape() {
                tape[l.reader] += 1;
            }
            all[l.reader] += 1;
        }
    }
    // Each reader's shared lean (SK2, SK4, SK5; SK6 for the numbers): by
    // how much of his looks were tape.
    let lean_of: Vec<f64> = readers
        .iter()
        .enumerate()
        .map(|(r, rd)| {
            if r >= 64 || all[r] == 0 || rd.link == Link::Event {
                0.0
            } else if numbers[r] == all[r] {
                super::footage::analytics_tilt(&foot)
            } else {
                tape[r] as f64 / all[r] as f64 * super::footage::scout_tilt(&foot, rd.judge, seen_vs_good, reel_only)
            }
        })
        .collect();
    // The staff's best test reader discounts hand times and testing camps.
    let test_eye = readers.iter().filter(|r| r.link != Link::Event).map(|r| r.athletic).fold(1.0f64, f64::max);
    let discount = clamp((test_eye - 6.0) / 12.0, 0.0, 1.0);
    // Pre-draft training (topic 4): a hard worker at a testing camp runs up
    // to about 0.05 s faster in the 40 without being a faster player.
    // F19: only the prospects who chose a testing camp this winter.
    let training = if matches!(p.stage, crate::world::Stage::Declared)
        && w.scouting.predraft.has(p.player.id, super::predraft::TESTING_CAMP)
    {
        2.0 * clamp((p.player.personality.work_ethic - 6.0) / 12.0, 0.0, 1.0) * (0.5 + 0.5 * p.bg.money.min(1.0))
    } else {
        0.0
    };
    let mut attrs = vec![AttrRead::default(); ATTRIBUTE_COUNT];
    let mut per_reader = vec![vec![None; ATTRIBUTE_COUNT]; n_r];
    for a in Attr::ALL {
        let k = a.index();
        let cat = a.category();
        let truth = p.player.attributes.get(a);
        let mut acc = 0.0;
        let mut prec = 0.0;
        let mut plain_prec = 0.0;
        let mut nread = 0u8;
        for (r, rd) in readers.iter().enumerate() {
            let n = count[r][k];
            if n == 0 {
                continue;
            }
            nread += 1;
            let skill = match cat {
                Category::Physical => rd.athletic,
                Category::Mental => 0.6 * rd.judge + 0.4 * rd.character,
                _ => rd.judge,
            };
            let sd = sharp(skill) * best[r][k] * vis;
            let c = cat_index(cat) as u64;
            let lean = 0.7 * key_normal(rd.key ^ 0x1EA4 ^ c << 56, id)
                + 0.714 * key_normal(rd.key ^ 0xA77 ^ (k as u64) << 50, id);
            let yr = if r < 64 { year[r] } else { w.year as u64 };
            let noise = key_normal(rd.key ^ 0x0153 ^ (k as u64) << 50 ^ yr << 40, id);
            let nn = n as f64;
            let mut v = sd * (LEAN * lean + NOISE * noise / nn.sqrt()) / MIX_SD;
            v += lean_of[r] * if cat == Category::Physical { 0.5 } else { 1.0 };
            let tilt = 0.33 * sd;
            v += match rd.bias {
                Some(Bias::Size) => tilt * norms.z_size,
                Some(Bias::SmallSchools) if norms.small_school => -tilt,
                Some(Bias::Testing) if cat != Category::Physical => tilt * norms.z_speed,
                Some(Bias::Tape) if cat == Category::Physical => -tilt * norms.z_speed,
                Some(Bias::Conference) if norms.conf_key != 0 && norms.conf_key % 12 == rd.key % 12 => -tilt,
                Some(Bias::Region) if norms.region == rd.home || norms.home == rd.home => tilt,
                Some(Bias::Speed) => tilt * norms.z_speed,
                _ => 0.0,
            };
            let var = sd * sd * (LEAN * LEAN + NOISE * NOISE / nn) / (MIX_SD * MIX_SD);
            per_reader[r][k] = Some((truth + v, var.sqrt()));
            acc += rd.say * v / var;
            prec += rd.say / var;
            plain_prec += 1.0 / var;
        }
        if let Some((sd, kind)) = meas[k] {
            let salt = match kind {
                Kind::Combine => 0xC0B1u64,
                Kind::ProDay => 0x9D4A,
                _ => 0xCA4F,
            };
            let mut v = sd * key_normal(0x3EA5 ^ salt ^ (k as u64) << 50, id);
            // Hand times run fast, and testing camps teach the test.
            if matches!(a, Attr::Speed | Attr::Acceleration | Attr::Agility) {
                let hand = if kind == Kind::ProDay { 2.5 } else { 0.0 };
                v += (hand + training) * (1.0 - discount);
            }
            acc += v / (sd * sd);
            prec += 1.0 / (sd * sd);
            plain_prec += 1.0 / (sd * sd);
        }
        if prec > 0.0 {
            attrs[k] = AttrRead {
                centre: clamp(truth + acc / prec, 1.0, 99.0),
                sd: (1.0 / plain_prec).sqrt(),
                seen: true,
                chips: chips[k],
                readers: nread,
            };
        }
    }
    // The overall now: what nobody saw comes from the services' word only
    // (wide), as the AI clubs' boards do.
    let prior = 9.0 * vis;
    let mut centres = p.player.attributes;
    let mut by_cat = [0.0f64; 8];
    let mut var_sum = 0.0;
    for a in Attr::ALL {
        let k = a.index();
        let wgt = weight(pos, a);
        let (c, sd) = if attrs[k].seen {
            (attrs[k].centre, attrs[k].sd)
        } else {
            let v = prior * key_normal(0x9A10 ^ (cat_index(a.category()) as u64) << 40, id);
            (p.player.attributes.get(a) + v, prior)
        };
        centres.set(a, c);
        by_cat[cat_index(a.category())] += wgt * sd;
        var_sum += wgt * wgt * sd * sd;
    }
    let corr: f64 = by_cat.iter().map(|x| x * x).sum::<f64>();
    let now_sd = (0.49 * corr + 0.51 * var_sum).sqrt();
    let now = overall_for(pos, &centres);
    // Where he could get to (topic 7): each reader's read of his ceiling,
    // sharper with his eye for potential, wider for the young.
    let plast = crate::development::potential_plasticity(p.player.age);
    let truth_pot = overall_for(pos, &p.player.ceilings).max(p.player.overall());
    let mut pacc = 0.0;
    let mut pprec = 0.0;
    for (r, rd) in readers.iter().enumerate() {
        if rd.link == Link::Event {
            continue;
        }
        let n = looks.iter().filter(|l| l.reader == r && !matches!(l.kind, Kind::Medical | Kind::Transcript)).count();
        if n == 0 {
            continue;
        }
        let talk = looks.iter().any(|l| l.reader == r && l.kind.chip() == super::CHIP_TALK);
        let sd = (3.0 + 7.0 * plast) * sharp(rd.potential) * vis * if talk { 1.0 } else { 1.15 };
        let lean = key_normal(rd.key ^ 0x9071, id);
        let yr = if r < 64 { year[r] } else { w.year as u64 };
        let noise = key_normal(rd.key ^ 0x9072 ^ yr << 40, id);
        let nn = n as f64;
        let v = sd * (LEAN * lean + NOISE * noise / nn.sqrt()) / MIX_SD;
        let var = sd * sd * (LEAN * LEAN + NOISE * NOISE / nn) / (MIX_SD * MIX_SD);
        pacc += rd.say * v / var;
        pprec += rd.say / var;
    }
    let ceiling = if pprec > 0.0 {
        let sd = (1.0 / pprec).sqrt();
        ((truth_pot + pacc / pprec).max(now), sd)
    } else {
        let sd = (3.0 + 7.0 * plast) * 1.6 * vis;
        ((truth_pot + sd * key_normal(0x9A11, id)).max(now), sd)
    };
    let traits = trait_reads(w, i, readers, &looks);
    let medical = medical_read(w, i, readers, &looks, medical_rating);
    FileRead { attrs, traits, now: (now, now_sd), ceiling, medical, per_reader, centres }
}

pub fn cat_index(c: Category) -> usize {
    match c {
        Category::Physical => 0,
        Category::Passing => 1,
        Category::Running => 2,
        Category::Receiving => 3,
        Category::LinePlay => 4,
        Category::Defense => 5,
        Category::KickingAndSnapping => 6,
        Category::Mental => 7,
    }
}

/// The trait reads (topic 5): each reader who could see a trait writes
/// his read of it, shaded by the source he heard it from; a scout with a
/// strong character read and network sees through more of the shading.
fn trait_reads(w: &World, i: usize, readers: &[Reader], looks: &[Evidence]) -> Vec<TraitRead> {
    let p = &w.people[i];
    let region = p.college.map(|c| crate::world::pro::scouts::region_of(w.programs[c].state));
    let id = p.player.id as u64;
    let mut out = vec![TraitRead::default(); TRAITS];
    for (t, out_t) in out.iter_mut().enumerate() {
        let truth = trait_value(p, t);
        let good = if trait_good_high(t) { 1.0 } else { -1.0 };
        let mut acc = 0.0;
        let mut prec = 0.0;
        let mut best_w = 0.0;
        for (li, l) in looks.iter().enumerate() {
            let (shows, base) = trait_sees(l.kind);
            if !shows.contains(&t) {
                continue;
            }
            let rd = &readers[l.reader];
            // SK9: in his own region an area scout's people talk to him.
            let known = match rd.contacts {
                Some((r, c))
                    if Some(r) == region
                        && matches!(l.kind, Kind::Background | Kind::Call | Kind::School | Kind::OldStaff) =>
                {
                    c
                }
                _ => 0.0,
            };
            let sd = base * sharp(rd.character) * (1.0 - 0.35 * known);
            let lean = key_normal(rd.key ^ 0xC4A2 ^ (t as u64) << 48, id);
            let noise = key_normal(rd.key ^ 0xC4A3 ^ (t as u64) << 48 ^ (l.date.0 as u64) << 20, id);
            let see_through = clamp((rd.character + rd.network - 16.0) / 24.0 + 0.2 * known, 0.0, 0.85);
            let shade = kind_shade(l.kind).tilt() * (1.0 - see_through) * good;
            let v = sd * (LEAN * lean + NOISE * noise) / MIX_SD + shade;
            let var = sd * sd;
            acc += v / var;
            prec += 1.0 / var;
            if 1.0 / var > best_w {
                best_w = 1.0 / var;
                out_t.main = li;
            }
        }
        if prec > 0.0 {
            out_t.centre = clamp(truth + acc / prec, 1.0, 20.0);
            out_t.sd = (1.0 / prec).sqrt();
            out_t.seen = true;
        }
    }
    out
}

/// The medical read (topic 6): each past injury, a hint of hidden injury
/// proneness and how long he might last, and a grade, through the medical
/// staff's diagnosis rating and what they were allowed to examine (the
/// combine and recheck for invitees, a top-30 or local visit; otherwise
/// only records and film).
fn medical_read(w: &World, i: usize, readers: &[Reader], looks: &[Evidence], rating: f64) -> MedicalRead {
    use crate::player::DevelopmentEvent as E;
    let p = &w.people[i];
    let id = p.player.id as u64;
    let examined = looks.iter().any(|l| matches!(l.kind, Kind::Medical | Kind::Visit | Kind::LocalDay));
    let records = looks.iter().any(|l| {
        matches!(l.kind, Kind::School | Kind::Call | Kind::OldStaff | Kind::Background | Kind::Official | Kind::Combine)
    });
    let mut notes = Vec::new();
    let mut minors = 0u32;
    for h in &p.player.history {
        let what = if h.events.has(E::MajorInjury) {
            Some("a major injury")
        } else if h.events.has(E::ModerateInjury) {
            Some("an injury")
        } else if h.events.has(E::MinorInjury) {
            minors += 1;
            None
        } else {
            None
        };
        if let Some(what) = what {
            let lasting = if h.events.has(E::LostBurst) { "; he never got all of his burst back" } else { "" };
            let missed = if h.games_missed > 0 {
                format!(", missed {} {}", h.games_missed, crate::world::folk::noun(h.games_missed, "game"))
            } else {
                String::new()
            };
            notes.push(format!("At {}: {what}{missed}{lasting}.", h.age));
        }
    }
    if minors > 0 {
        notes.push(format!("{minors} minor knocks on record, nothing that cost him a season."));
    }
    let seen_any = examined || records || !looks.is_empty();
    let sd = if examined { 0.35 } else { 0.8 } * sharp(rating);
    let lean = key_normal(0x3ED1 ^ (rating as u64) << 8 ^ readers.len() as u64, id);
    let prone = (p.player.injury_proneness - 10.0) / 5.0;
    let long = p.player.longevity / 2.0;
    let concern = medical_truth(p) + sd * lean;
    if examined {
        if prone + 0.6 * lean >= 0.8 {
            let part = match p.player.position {
                Position::OT | Position::IOL | Position::DT | Position::EDGE => "his knees and back will need managing",
                Position::QB => "his throwing shoulder will need watching",
                _ => "soft-tissue history: his hamstrings will need managing",
            };
            notes.push(format!("The doctors' note: {part}."));
        } else if prone + 0.6 * lean <= -0.8 {
            notes.push("The doctors' note: a clean, durable frame.".to_string());
        }
        if long + 0.6 * lean >= 1.0 {
            notes.push("Built to last: the joints of a younger man.".to_string());
        } else if long + 0.6 * lean <= -1.0 {
            notes.push("Wear on the joints already: a shorter career is likely.".to_string());
        }
    } else if seen_any {
        notes.push("Records and film only: our doctors haven't examined him.".to_string());
    }
    let grade = if !seen_any {
        0
    } else if concern >= 2.4 {
        3
    } else if concern >= 1.4 {
        2
    } else if concern >= 0.6 {
        1
    } else {
        0
    };
    MedicalRead { grade, examined, concern, notes }
}

/// What a doctor would find: proneness, the injuries on record, his
/// longevity and the wear already on him, on the medical grades' scale
/// (0.6 a minor concern, 1.4 major, 2.4 a failed physical). Hidden: every
/// read of it carries its doctors' misread.
pub fn medical_truth(p: &crate::world::Person) -> f64 {
    use crate::player::DevelopmentEvent as E;
    let (mut majors, mut moderates) = (0u32, 0u32);
    for h in &p.player.history {
        if h.events.has(E::MajorInjury) {
            majors += 1;
        } else if h.events.has(E::ModerateInjury) {
            moderates += 1;
        }
    }
    let prone = (p.player.injury_proneness - 10.0) / 5.0;
    let long = p.player.longevity / 2.0;
    0.45 * prone + 0.8 * majors as f64 + 0.25 * moderates as f64 - 0.2 * long + 0.3 * p.player.wear
}

pub fn medical_words(g: u8) -> &'static str {
    match g {
        0 => "Cleared",
        1 => "Minor concern",
        2 => "Major concern",
        _ => "Failed",
    }
}

// --- Potential in words (topic 7) ---------------------------------------------------------

/// The level a potential read speaks in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Speak {
    /// A high school recruit: college levels.
    Recruit,
    /// A college player or draft prospect: NFL roles.
    Prospect,
    /// A pro.
    Pro,
}

/// The role tier for a ceiling read, in the staff's own words.
pub fn tier_words(speak: Speak, ceiling: f64) -> &'static str {
    use crate::career::NFL_LEVELS as L;
    match speak {
        Speak::Recruit => {
            if ceiling >= L.starter {
                "a future NFL player"
            } else if ceiling >= 68.0 {
                "a Power Four starter"
            } else if ceiling >= 61.0 {
                "an FBS starter"
            } else if ceiling >= 54.0 {
                "an FCS player"
            } else {
                "a small-college player"
            }
        }
        Speak::Prospect | Speak::Pro => {
            if ceiling >= L.elite {
                "one of the best in the league"
            } else if ceiling >= L.pro_bowl {
                "a Pro Bowl player"
            } else if ceiling >= L.starter {
                "an NFL starter"
            } else if ceiling >= L.roster {
                "an NFL backup"
            } else {
                "a camp body"
            }
        }
    }
}

/// A floor and ceiling in words (topic 7).
pub fn floor_words(now: f64, ceiling: f64, sd: f64, age: u32, physical: f64) -> &'static str {
    let gap = ceiling - now;
    if sd >= 7.5 && gap >= 8.0 {
        "boom or bust"
    } else if gap >= 10.0 && physical >= 70.0 {
        "raw, but the tools are there"
    } else if gap <= 2.5 && sd <= 4.5 {
        "what you see is what you get"
    } else if gap <= 5.0 && now >= 62.0 && age >= 21 {
        "high floor, limited ceiling"
    } else if gap >= 6.0 && age <= 20 {
        "a year or two away; he hasn't scratched the surface"
    } else if now >= 64.0 && age >= 22 {
        "pro-ready"
    } else {
        "a steady climb ahead"
    }
}

/// How sure the staff is, from the ceiling read's spread.
pub fn sure_words(sd: f64) -> &'static str {
    if sd <= 3.5 {
        "sure"
    } else if sd <= 6.0 {
        "fairly sure"
    } else {
        "unsure"
    }
}

/// A hint (topic 7): a real observation tied to a true cause in the
/// player, picked up only by good evaluators and sometimes wrong. Hints
/// start at judging potential 13, the hit rate rising from about 60% at 13
/// to about 90% at 20.
pub fn hints(w: &World, i: usize, readers: &[Reader], looks: &[Evidence]) -> Vec<(String, usize)> {
    use crate::player::DevelopmentEvent as E;
    let p = &w.people[i];
    let id = p.player.id as u64;
    let mut out: Vec<(String, usize)> = Vec::new();
    let young = p.player.age <= 21;
    let per = &p.player.personality;
    let body_gap = p.player.body.weight - crate::body::body_now(&p.player).weight;
    let tend = &p.player.tend;
    let size = crate::body::combine_size(p.player.position);
    let small = p.player.body.weight < size.2 - size.3;
    let mut facts: Vec<(bool, &'static str, &'static str, u64)> = vec![
        (young && p.player.bloom >= 1.0, "still growing into his body", "looks physically mature already", 1),
        (
            young && p.player.bloom <= -1.0,
            "an early bloomer: physically mature already",
            "still growing into his body",
            2,
        ),
        (young && p.player.has_had(E::BodyStalled), "his body may be done growing", "still growing into his body", 3),
        (body_gap >= 15.0, "a frame to add 20 pounds", "maxed out on his frame", 4),
        (per.work_ethic >= 15.0, "a tireless worker", "questions about his work ethic", 5),
        (per.work_ethic <= 5.0, "questions about his work ethic", "a tireless worker", 6),
        (per.coachability >= 15.0, "soaks up coaching", "doesn't take coaching well", 7),
        (per.love_of_game <= 5.0, "questions about his love of the game", "lives for football", 8),
        (per.love_of_game >= 16.0, "lives for football", "questions about his love of the game", 9),
        (
            p.player.age <= 20 && matches!(p.stage, crate::world::Stage::College | crate::world::Stage::Declared),
            "young for his class",
            "old for his class",
            10,
        ),
        (
            p.player.age >= 23 && matches!(p.stage, crate::world::Stage::College | crate::world::Stage::Declared),
            "old for his class: what you see is close to what you get",
            "young for his class",
            11,
        ),
        (
            matches!(p.player.position, Position::CB | Position::S)
                && tend.get(crate::tendencies::Tend::JumpRoutes) >= 14.0,
            "gambles in coverage: jumps routes",
            "plays it safe in coverage",
            12,
        ),
        (
            matches!(p.player.position, Position::EDGE | Position::LB)
                && small
                && p.player.attributes.get(Attr::FinesseRush) >= 65.0,
            "wins with bend and speed for his size: production should carry up",
            "a tweener without a position",
            13,
        ),
        (
            p.player.position == Position::QB
                && p.player.attributes.get(Attr::Speed) >= 80.0
                && p.player.attributes.get(Attr::ShortAccuracy) < 55.0,
            "a safety's body playing quarterback",
            "a pocket passer",
            14,
        ),
    ];
    // The best potential reader on the staff decides what is noticed.
    let Some((ri, eye)) = readers
        .iter()
        .enumerate()
        .filter(|(r, rd)| rd.link != Link::Event && looks.iter().any(|l| l.reader == *r))
        .map(|(r, rd)| (r, rd.potential))
        .max_by(|a, b| a.1.total_cmp(&b.1))
    else {
        return out;
    };
    if eye < crate::scouting::HINT_MIN_ACCURACY {
        return out;
    }
    let notice = clamp(0.3 + (eye - 13.0) * 0.08, 0.0, 0.85);
    let hit = clamp(0.6 + (eye - 13.0) * 0.043, 0.6, 0.9);
    let key = readers[ri].key;
    facts.retain(|f| f.0);
    for (_, right, wrong, salt) in facts {
        if crate::world::season::key_uniform(key ^ 0x4171 ^ salt, id) < notice {
            let correct = crate::world::season::key_uniform(key ^ 0x4172 ^ salt, id) < hit;
            out.push(((if correct { right } else { wrong }).to_string(), ri));
        }
    }
    out
}

/// "Plays like a young...": a player from the world's history, past or
/// present, matched by position, body and style (the shape of his
/// attributes and tendencies), never by potential. A better evaluator
/// matches more closely. Returns the comparison's name and where he
/// played, and his player id when he is still in the world.
pub fn comparison(w: &World, i: usize, eye: f64, file: &FileRead) -> Option<(String, String, Option<u32>)> {
    let p = &w.people[i];
    let pos = p.player.position;
    let id = p.player.id as u64;
    // His style as the staff reads it: each seen attribute's distance from
    // his own average, by position weight.
    let mean_seen = {
        let v: Vec<f64> = Attr::ALL
            .iter()
            .filter(|a| file.attrs[a.index()].seen && weight(pos, **a) > 0.0)
            .map(|a| file.attrs[a.index()].centre)
            .collect();
        if v.len() < 6 {
            return None;
        }
        v.iter().sum::<f64>() / v.len() as f64
    };
    let shape = |get: &dyn Fn(Attr) -> f64, mean: f64| -> Vec<f64> {
        Attr::ALL.iter().map(|&a| if weight(pos, a) > 0.0 { get(a) - mean } else { 0.0 }).collect()
    };
    let mine =
        shape(&|a: Attr| if file.attrs[a.index()].seen { file.attrs[a.index()].centre } else { mean_seen }, mean_seen);
    let h = p.player.body.height;
    let wt = p.player.body.weight;
    let noise_sd = 0.6 * sharp(eye);
    let mut best: Option<(f64, String, String, Option<u32>)> = None;
    let mut consider = |pl: &crate::player::Player, name: &str, where_: String, live: Option<u32>| {
        if pl.id == p.player.id || pl.position != pos {
            return;
        }
        let m: f64 = {
            let v: Vec<f64> =
                Attr::ALL.iter().filter(|&&a| weight(pos, a) > 0.0).map(|&a| pl.attributes.get(a)).collect();
            v.iter().sum::<f64>() / v.len().max(1) as f64
        };
        let theirs = shape(&|a: Attr| pl.attributes.get(a), m);
        let style: f64 = mine.iter().zip(theirs.iter()).map(|(a, b)| (a - b).powi(2)).sum::<f64>().sqrt() / 10.0;
        let body = ((pl.body.height - h) / 1.5).powi(2) + ((pl.body.weight - wt) / 12.0).powi(2);
        let fuzz = noise_sd * key_normal(0xC03A ^ pl.id as u64, id).abs();
        let d = style + 0.6 * body.sqrt() + fuzz;
        if best.as_ref().is_none_or(|b| d < b.0) {
            best = Some((d, name.to_string(), where_, live));
        }
    };
    // Proven players only: pros with a few seasons, and retired players
    // from the record.
    for q in w.people.iter() {
        if q.stage == crate::world::Stage::Pro && q.pro.as_ref().is_some_and(|c| c.accrued >= 3) {
            let team =
                q.pro.as_ref().and_then(|c| c.team).map_or_else(|| "the NFL".to_string(), |t| w.nfl_team_name(t));
            consider(&q.player, &q.name, team, Some(q.player.id));
        }
    }
    for a in w.alumni.iter().rev().take(4000) {
        if let (Some(pl), Some(_)) = (a.player.as_ref(), a.pick) {
            let team = a.nfl_team.map_or_else(|| "the NFL".to_string(), |t| t.to_string());
            consider(pl, &a.name, format!("{team}, {} draft", a.year), None);
        }
    }
    best.map(|b| (b.1, b.2, b.3))
}

/// Exact test numbers where the staff measured them (S2): a seeded test
/// day, so every club sees the same combine and each pro day its own.
pub fn tests(p: &crate::player::Player, kind: Kind, year: i32) -> crate::body::Combine {
    let salt = match kind {
        Kind::Combine => 0xC0B1u32,
        Kind::ProDay => 0x9D4A,
        Kind::Camp => 0xCA4F,
        Kind::Showcase => 0x5A0C,
        _ => 0x7E57,
    };
    let mut rng = crate::rng::Rng::new(p.id ^ salt ^ (year as u32).wrapping_mul(2654435761));
    let mut c = crate::body::combine(p, &mut rng);
    // Hand times at a pro day on a fast track run about 0.05 s quick.
    if kind == Kind::ProDay {
        c.forty = ((c.forty - 0.05) * 100.0).round() / 100.0;
    }
    // A camp's laser 40 for a high schooler on grass.
    if kind == Kind::Camp {
        c.forty = ((c.forty + 0.04) * 100.0).round() / 100.0;
    }
    c
}

/// What a school lists him at: high school and college listings run tall
/// and heavy (a real habit), about an inch and a few pounds.
pub fn listed(p: &crate::player::Player) -> (f64, f64) {
    let b = crate::body::body_now(p);
    let up = crate::world::season::key_uniform(0x1157, p.id as u64);
    ((b.height + 0.25 + up).round(), (b.weight + 3.0 + 6.0 * up).round())
}

/// A trait read in words, with how sure.
pub fn trait_word(k: usize, centre: f64) -> &'static str {
    super::lines::trait_word(k, centre)
}

/// Each attribute's position weight share (for picking what a look showed).
pub fn matters(pos: Position, a: Attr) -> f64 {
    weight(pos, a)
}

/// His grade point average (the transcript's number; screens show it as a
/// range unless the staff has his transcript).
pub fn gpa(p: &crate::world::Person) -> f64 {
    p.bg.gpa
}
