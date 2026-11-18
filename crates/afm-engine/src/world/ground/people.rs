//! Who is a ground person, and his makeup (GR62, GR63, part B). At college
//! the people on your ground are:
//! - high school head coaches and coordinators, junior college coaches and
//!   other college staffs' coaches (built coaches, `world/coach.rs`);
//! - private trainers and 7v7 directors (former players, `world/former.rs`,
//!   `Life::Trainer` and `Life::Camp`), each with his clients or his team;
//! - each school's counselor (a person per school and tenure);
//! - a recruit's family, which keeps one memory for every brother;
//! - prep school coaches (the real postgraduate programs).
//!
//! People are made when they matter, from what the world already has (a
//! coach's loyalty and recruiting eye, a former player's eye and how
//! straight he talks) and the world's seed, never from its random stream,
//! so a person is the same every time. Nobody sees the hidden numbers: you
//! learn them from his record and your staff's read.

use crate::calendar::Date;
use crate::player::Trait;
use crate::world::World;
use crate::world::coach::{Employer, Role};
use crate::world::data::Division;
use crate::world::ethnicity::Ethnicity;
use crate::world::former::Life;
use crate::world::{Stage, folk};

/// A person on someone's ground. New kinds go at the end, so stored lists
/// stay sorted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub enum Who {
    /// A coach, by coach id: a high school head coach or coordinator, a
    /// junior college coach, another college staff's man.
    Coach(u32),
    /// A private trainer (`Life::Trainer`), by former player id.
    Trainer(u32),
    /// A 7v7 director and camp man (`Life::Camp`), by former player id.
    SevenOnSeven(u32),
    /// A school's counselor, by high school id: the person in the job now
    /// (`counselor`).
    Counselor(u32),
    /// A recruit's family, by the player id of the oldest brother the world
    /// knows (`family_of`): the younger brother reads the same memory.
    Family(u32),
    /// A prep school's head coach, by the school's place in `PREPS`.
    Prep(u16),
}

/// Who holds a memory (GR29): a program (its ties stay when its men go)
/// or a coach (his own ties, which go with him).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub enum Holder {
    Program(u32),
    Coach(u32),
}

/// What kind of person he is on your ground.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Kind {
    HsHead,
    HsCoordinator,
    /// A high school assistant or a coach between jobs.
    HsCoach,
    Juco,
    /// Another college staff's man (a four-year program); a pro coach or a
    /// man between jobs is a colleague too, never ground.
    College,
    Trainer,
    SevenOnSeven,
    Counselor,
    Family,
    Prep,
}

impl Kind {
    pub fn words(self) -> &'static str {
        match self {
            Kind::HsHead => "high school head coach",
            Kind::HsCoordinator => "high school coordinator",
            Kind::HsCoach => "high school coach",
            Kind::Juco => "junior college coach",
            Kind::College => "college coach",
            Kind::Trainer => "private trainer",
            Kind::SevenOnSeven => "7v7 director",
            Kind::Counselor => "counselor",
            Kind::Family => "family",
            Kind::Prep => "prep school coach",
        }
    }
}

/// What he pushes (people.md, "What each person has").
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Agenda {
    /// A coach protects his kid.
    HisKids,
    /// A trainer his paying clients.
    Clients,
    /// A 7v7 director his team's exposure.
    Exposure,
    /// A counselor a kid who'll graduate.
    Graduates,
    /// A family what the family wants.
    Family,
    /// A junior college or prep coach homes for his kids moving up.
    MovingUp,
    /// Another college staff: places for its leavers and its no's.
    Leavers,
}

impl Agenda {
    pub fn words(self) -> &'static str {
        match self {
            Agenda::HisKids => "protects his kids",
            Agenda::Clients => "pushes his paying clients",
            Agenda::Exposure => "wants his team seen",
            Agenda::Graduates => "wants each kid placed where he'll graduate",
            Agenda::Family => "wants what the family wants",
            Agenda::MovingUp => "needs homes for his kids moving up",
            Agenda::Leavers => "needs homes for his leavers and his no's",
        }
    }
}

/// What a person is like with staffs (hidden).
#[derive(Clone, Copy, Debug)]
pub struct Makeup {
    /// How warmly he takes visitors, 0.7-1.3 (by his nature).
    pub warmth: f32,
    /// Loyal (1) or self-serving (0).
    pub loyal: f32,
    /// How many years a big moment holds at full weight, 3-5.
    pub grudge: f32,
    /// His hour: when he writes or calls (local minutes after midnight).
    pub hour: u16,
    /// His eye for a player, 1-20: how good his word on a kid is.
    pub eye: f32,
    /// How straight he talks, 1-20 (low oversells).
    pub straight: f32,
    pub agenda: Agenda,
}

fn salt(who: Who) -> [u64; 2] {
    match who {
        Who::Coach(c) => [1, c as u64],
        Who::Trainer(f) => [2, f as u64],
        Who::SevenOnSeven(f) => [3, f as u64],
        Who::Counselor(h) => [4, h as u64],
        Who::Family(k) => [5, k as u64],
        Who::Prep(k) => [6, k as u64],
    }
}

/// A number in [0, 1) for this person, the same every time.
pub(crate) fn u(w: &World, who: Who, k: u64) -> f64 {
    let s = salt(who);
    super::unit(w.options.seed, &[0x6E0D, s[0], s[1], k])
}

/// His makeup.
pub fn makeup(w: &World, who: Who) -> Makeup {
    let u = |k: u64| u(w, who, k);
    // A coach's loyalty is built (1-20); a family's leans on the kid's own
    // loyalty; everyone else's comes from the seed.
    let loyal = match who {
        Who::Coach(c) => w.coaches.get(c as usize).map_or(0.5, |c| ((c.personality.loyalty - 1.0) / 19.0) as f32),
        Who::Family(k) => folk::person(w, k).map_or(u(1) as f32, |i| {
            let t = w.people[i].player.personality.traits.get(Trait::Loyalty);
            (0.5 * (t - 1.0) / 19.0 + 0.5 * u(1)) as f32
        }),
        _ => u(1) as f32,
    };
    // Warmth leans on loyalty: a loyal man warms to the staff that shows up.
    let warmth = (0.7 + 0.6 * (0.6 * u(2) as f32 + 0.4 * loyal)).clamp(0.7, 1.3);
    let grudge = 3.0 + 2.0 * u(3) as f32;
    // A high school coach writes in his planning period after lunch, a
    // counselor in the school morning, a college man between meetings,
    // trainers, 7v7 men and families in the evening.
    let hour = match who {
        Who::Coach(c) => match employer(w, c as usize) {
            Some(Employer::College(_)) => 11 * 60 + (u(4) * 60.0) as u16,
            _ => 13 * 60 + (u(4) * 50.0) as u16,
        },
        Who::Counselor(_) => 10 * 60 + (u(4) * 60.0) as u16,
        Who::Prep(_) => 15 * 60 + (u(4) * 60.0) as u16,
        Who::Trainer(_) | Who::SevenOnSeven(_) | Who::Family(_) => 19 * 60 + (u(4) * 90.0) as u16,
    };
    // His eye and how straight he talks: a coach's recruiting rating and
    // his own straightness (the tips' reading, `folk/tips.rs`), a former
    // player's own; a counselor reads grades true (a teacher as a source,
    // eye 9); a family loves its son.
    let (eye, straight) = match who {
        Who::Coach(c) => (
            w.coaches.get(c as usize).map_or(10.0, |c| c.ability.recruiting as f32),
            (6.0 + 12.0 * folk::roll(w, 0x57A1, c as u64)) as f32,
        ),
        Who::Trainer(f) | Who::SevenOnSeven(f) => {
            w.formers.get(f).map_or((10.0, 10.0), |x| (x.eye as f32, x.straight as f32))
        }
        Who::Counselor(_) => (8.0 + 3.0 * u(5) as f32, 14.0 + 5.0 * u(6) as f32),
        Who::Family(_) => (4.0 + 6.0 * u(5) as f32, 6.0 + 10.0 * u(6) as f32),
        Who::Prep(_) => (9.0 + 6.0 * u(5) as f32, 6.0 + 10.0 * u(6) as f32),
    };
    let agenda = match kind(w, who) {
        Kind::HsHead | Kind::HsCoordinator | Kind::HsCoach => Agenda::HisKids,
        Kind::Juco | Kind::Prep => Agenda::MovingUp,
        Kind::College => Agenda::Leavers,
        Kind::Trainer => Agenda::Clients,
        Kind::SevenOnSeven => Agenda::Exposure,
        Kind::Counselor => Agenda::Graduates,
        Kind::Family => Agenda::Family,
    };
    Makeup { warmth, loyal, grudge, hour, eye: eye.clamp(1.0, 20.0), straight: straight.clamp(1.0, 20.0), agenda }
}

/// A coach's employer now.
pub fn employer(w: &World, c: usize) -> Option<Employer> {
    w.coaches.get(c)?.job.map(|j| j.employer)
}

/// The high school a coach works at now, and his role there.
pub fn coach_school(w: &World, c: usize) -> Option<(usize, Role)> {
    let j = w.coaches.get(c)?.job?;
    match j.employer {
        Employer::HighSchool(h) => Some((h, j.role)),
        _ => None,
    }
}

/// His kind.
pub fn kind(w: &World, who: Who) -> Kind {
    match who {
        Who::Coach(c) => match w.coaches.get(c as usize).and_then(|c| c.job) {
            Some(j) => match j.employer {
                Employer::HighSchool(_) => match j.role {
                    Role::HeadCoach => Kind::HsHead,
                    Role::OffensiveCoordinator | Role::DefensiveCoordinator => Kind::HsCoordinator,
                    _ => Kind::HsCoach,
                },
                Employer::College(p) if w.programs.get(p).is_some_and(|p| p.division == Division::JUCO) => Kind::Juco,
                Employer::College(_) => Kind::College,
                // A pro coach is a colleague, never ground.
                _ => Kind::College,
            },
            None => Kind::College,
        },
        Who::Trainer(_) => Kind::Trainer,
        Who::SevenOnSeven(_) => Kind::SevenOnSeven,
        Who::Counselor(_) => Kind::Counselor,
        Who::Family(_) => Kind::Family,
        Who::Prep(_) => Kind::Prep,
    }
}

/// Whether he is on Ground's ladder as a person you work (a coach at a
/// four-year college is a colleague first: his standing is your network's
/// tie to him, `standing::derived`).
pub fn on_ground(w: &World, who: Who) -> bool {
    kind(w, who) != Kind::College
}

/// The school a person works at now.
pub fn school(w: &World, who: Who) -> Option<usize> {
    match who {
        Who::Coach(c) => coach_school(w, c as usize).map(|s| s.0),
        Who::Counselor(h) => Some(h as usize),
        Who::Family(k) => folk::person(w, k).map(|i| w.people[i].hs),
        Who::Trainer(_) | Who::SevenOnSeven(_) | Who::Prep(_) => None,
    }
}

/// The state he works in.
pub fn state(w: &World, who: Who) -> Option<usize> {
    match who {
        Who::Trainer(f) | Who::SevenOnSeven(f) => w.formers.get(f).map(|x| x.area as usize),
        Who::Prep(k) => PREPS.get(k as usize).and_then(|p| state_index(p.2)),
        Who::Coach(c) => match employer(w, c as usize)? {
            Employer::College(p) => w.programs.get(p).map(|p| p.state),
            Employer::HighSchool(h) => w.high_schools.get(h).map(|s| s.state),
            _ => None,
        },
        _ => school(w, who).and_then(|h| w.high_schools.get(h)).map(|s| s.state),
    }
}

/// A school's head coach, the man who holds its kids (GR27).
pub fn head_coach_of(w: &World, hs: usize) -> Option<Who> {
    w.high_schools.get(hs)?.coach.map(|c| Who::Coach(c as u32))
}

/// Everyone at a school a college staff deals with: the head coach, the
/// coordinators who sit in on visits, the counselor.
pub fn at_school(w: &World, hs: usize) -> Vec<Who> {
    let Some(h) = w.high_schools.get(hs) else { return Vec::new() };
    let mut v: Vec<Who> = [h.coach, h.oc, h.dc].into_iter().flatten().map(|c| Who::Coach(c as u32)).collect();
    v.push(Who::Counselor(hs as u32));
    v
}

// ---------------------------------------------------------------------------
// Counselors.

/// A counselor's tenure at a school: (its first year, the person's key).
/// Each school's counselors serve 4 to 14 years in turn (the build's pick,
/// proposed, not confirmed), the same every run.
pub fn counselor(w: &World, hs: u32, year: i32) -> (i32, u64) {
    let base = super::unit(w.options.seed, &[0xC0E5, hs as u64]);
    let len = 4 + (super::unit(w.options.seed, &[0xC0E6, hs as u64]) * 11.0) as i32;
    let offset = (base * len as f64) as i32;
    let t = (year + offset).div_euclid(len);
    (t * len - offset, super::key(&[0xC0E7, hs as u64, t as u64]))
}

/// A made-up person's name from a key: a surname from the real lists by a
/// heritage drawn at the national shares of the job (counselors: BLS
/// 2023, about 64% white, 20% Black, 12% Hispanic, 4% Asian), and a title.
fn made_name(w: &World, key: u64, women: f64) -> (String, Ethnicity) {
    let r = super::unit(w.options.seed, &[0x4A3E, key]);
    let eth = if r < 0.64 {
        Ethnicity::White
    } else if r < 0.84 {
        Ethnicity::Black
    } else if r < 0.96 {
        Ethnicity::Hispanic
    } else {
        Ethnicity::EastAsian
    };
    let mut rng = crate::rng::Rng::new((key as u32) ^ (key >> 32) as u32 ^ w.options.seed.rotate_left(17));
    let full = crate::world::names::person_name(eth, 1975, &mut rng);
    let last = crate::world::names::surname_of(&full).to_string();
    let title = if super::unit(w.options.seed, &[0x4A3F, key]) < women { "Ms." } else { "Mr." };
    (format!("{title} {last}"), eth)
}

// ---------------------------------------------------------------------------
// Families.

/// A kid's family (GR62): one memory for every brother. A younger kid at
/// the same high school with the same surname, one to six classes later,
/// whose family has an older brother in college (`older_brother_at`) or a
/// family tie, is his younger brother: he reads the oldest brother's
/// memory (the build's pick, proposed, not confirmed).
pub fn family_of(w: &World, person: usize) -> Who {
    let mut i = person;
    for _ in 0..4 {
        match older_brother(w, i) {
            Some(j) => i = j,
            None => break,
        }
    }
    Who::Family(w.people[i].player.id)
}

fn older_brother(w: &World, i: usize) -> Option<usize> {
    let p = w.people.get(i)?;
    let id = p.player.id;
    let last = crate::world::names::surname_of(&p.name);
    // A family tie to an older player at the same high school.
    for t in w.folk.ties.of_id(id) {
        if t.kind == crate::world::folk::ties::Kind::Family
            && t.b == id
            && let Some(j) = folk::person(w, t.a)
            && w.people[j].hs == p.hs
            && w.people[j].class_of < p.class_of
        {
            return Some(j);
        }
    }
    p.family.older_brother_at?;
    w.people.iter().position(|q| {
        q.hs == p.hs
            && q.class_of < p.class_of
            && q.class_of >= p.class_of - 6
            && q.player.id != id
            && crate::world::names::surname_of(&q.name) == last
    })
}

/// A cousin's family (a family tie to a kid at another high school): the
/// family he starts from, a level lower.
pub fn cousin_of(w: &World, person: usize) -> Option<Who> {
    let p = w.people.get(person)?;
    let id = p.player.id;
    w.folk
        .ties
        .of_id(id)
        .filter(|t| t.kind == crate::world::folk::ties::Kind::Family)
        .filter_map(|t| folk::person(w, if t.a == id { t.b } else { t.a }))
        .find(|&j| w.people[j].hs != p.hs)
        .map(|j| family_of(w, j))
}

/// Who speaks for a family: the decider, in words ("his mother").
pub fn family_voice(w: &World, who: Who) -> String {
    use crate::world::background::Decider;
    let Who::Family(k) = who else { return String::new() };
    let Some(i) = folk::person(w, k) else { return "his family".into() };
    let mother = u(w, who, 7) < 0.7;
    match w.people[i].family.decider {
        Decider::Parent if mother => "his mother".into(),
        Decider::Parent => "his father".into(),
        Decider::Grandparent => "his grandmother".into(),
        Decider::Handler => "the man who advises the family".into(),
        Decider::Himself => "his parents".into(),
    }
}

// ---------------------------------------------------------------------------
// Prep schools.

/// The real postgraduate programs a prep year runs through: name, town,
/// state.
pub const PREPS: [(&str, &str, &str); 11] = [
    ("Fork Union Military Academy", "Fork Union", "VA"),
    ("Hargrave Military Academy", "Chatham", "VA"),
    ("Milford Academy", "New Berlin", "NY"),
    ("Bridgton Academy", "North Bridgton", "ME"),
    ("Avon Old Farms", "Avon", "CT"),
    ("Cheshire Academy", "Cheshire", "CT"),
    ("Kimball Union Academy", "Meriden", "NH"),
    ("Tilton School", "Tilton", "NH"),
    ("Worcester Academy", "Worcester", "MA"),
    ("Valley Forge Military Academy", "Wayne", "PA"),
    ("IMG Academy", "Bradenton", "FL"),
];

fn state_index(code: &str) -> Option<usize> {
    crate::world::data::STATES.iter().position(|s| s.code == code)
}

/// A prep year's school: the program in his home state, else one keyed to
/// him (the build's pick, proposed, not confirmed: most go east).
pub fn prep_of(w: &World, person: usize) -> Option<Who> {
    let p = w.people.get(person)?;
    if p.stage != Stage::PrepSchool {
        return None;
    }
    let st = w.high_schools.get(p.hs).map(|h| h.state);
    let here: Vec<usize> = (0..PREPS.len()).filter(|&k| state_index(PREPS[k].2) == st).collect();
    let pick = if here.is_empty() {
        (super::unit(w.options.seed, &[0x93E9, p.player.id as u64]) * PREPS.len() as f64) as usize
    } else {
        here[(super::unit(w.options.seed, &[0x93EA, p.player.id as u64]) * here.len() as f64) as usize]
    };
    Some(Who::Prep(pick.min(PREPS.len() - 1) as u16))
}

// ---------------------------------------------------------------------------
// Trainers and 7v7 directors: their clients and their teams.

/// The private trainer who works a kid (GR62): a trainer in his state, the
/// same one every time, for a kid with two stars or more (the build's
/// pick, proposed, not confirmed).
pub fn trainer_of(w: &World, person: usize) -> Option<Who> {
    let p = w.people.get(person)?;
    if p.stars < 2 || !matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) {
        return None;
    }
    let st = w.high_schools.get(p.hs)?.state;
    pick_former(w, st, Life::Trainer, p.player.id).map(Who::Trainer)
}

/// The 7v7 team a skill player runs with in the spring and summer.
pub fn seven_of(w: &World, person: usize) -> Option<Who> {
    use crate::positions::Position as P;
    let p = w.people.get(person)?;
    if p.stars < 2 || !matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) {
        return None;
    }
    if !matches!(p.player.position, P::QB | P::RB | P::WR | P::TE | P::LB | P::CB | P::S) {
        return None;
    }
    let st = w.high_schools.get(p.hs)?.state;
    pick_former(w, st, Life::Camp, p.player.id).map(Who::SevenOnSeven)
}

fn pick_former(w: &World, state: usize, life: Life, kid: u32) -> Option<u32> {
    let pool: Vec<u32> =
        w.formers.list.iter().filter(|f| f.life == life && f.area as usize == state).map(|f| f.id).collect();
    if pool.is_empty() {
        return None;
    }
    let k = (super::unit(w.options.seed, &[0x7A1E, life as u64, kid as u64]) * pool.len() as f64) as usize;
    Some(pool[k.min(pool.len() - 1)])
}

/// A trainer's or 7v7 director's kids today: the recruits in his state he
/// works (at most `n`, the best first).
pub fn clients(w: &World, who: Who, n: usize) -> Vec<usize> {
    let Some(st) = state(w, who) else { return Vec::new() };
    let mut v: Vec<usize> = (0..w.people.len())
        .filter(|&i| {
            let q = &w.people[i];
            matches!(q.stage, Stage::HighSchool | Stage::PrepSchool)
                && q.stars >= 2
                && w.high_schools.get(q.hs).is_some_and(|h| h.state == st)
        })
        .filter(|&i| match who {
            Who::Trainer(_) => trainer_of(w, i) == Some(who),
            Who::SevenOnSeven(_) => seven_of(w, i) == Some(who),
            _ => false,
        })
        .collect();
    v.sort_by_key(|&i| (std::cmp::Reverse(w.people[i].stars), w.people[i].player.id));
    v.truncate(n);
    v
}

// ---------------------------------------------------------------------------
// Names, roles, ties.

/// His name.
pub fn name(w: &World, who: Who) -> String {
    match who {
        Who::Coach(c) => w.coaches.get(c as usize).map_or_else(|| "A coach".into(), |c| c.name.clone()),
        Who::Trainer(f) | Who::SevenOnSeven(f) => {
            w.formers.get(f).map_or_else(|| "A trainer".into(), |x| x.name.clone())
        }
        Who::Counselor(h) => made_name(w, counselor(w, h, w.year).1, 0.75).0,
        Who::Family(k) => match folk::person(w, k) {
            Some(i) => format!("The {} family", crate::world::names::surname_of(&w.people[i].name)),
            None => "A family".into(),
        },
        Who::Prep(k) => made_name(w, super::key(&[0x93EB, k as u64, (w.year / 6) as u64]), 0.05)
            .0
            .replacen("Ms. ", "Coach ", 1)
            .replacen("Mr. ", "Coach ", 1),
    }
}

/// His heritage, for the face the screens draw.
pub fn heritage(w: &World, who: Who) -> Ethnicity {
    match who {
        Who::Coach(c) => w.coaches.get(c as usize).map_or(Ethnicity::White, |c| c.ethnicity),
        Who::Trainer(f) | Who::SevenOnSeven(f) => w.formers.get(f).map_or(Ethnicity::White, |x| x.ethnicity),
        Who::Counselor(h) => made_name(w, counselor(w, h, w.year).1, 0.75).1,
        Who::Family(k) => folk::person(w, k).map_or(Ethnicity::White, |i| w.people[i].ethnicity),
        Who::Prep(k) => made_name(w, super::key(&[0x93EB, k as u64, (w.year / 6) as u64]), 0.05).1,
    }
}

/// The face recipe the screens draw (`looks.rs`): a coach's own; everyone
/// else's the same keyed recipe from who he is.
pub fn looks(w: &World, who: Who) -> crate::world::looks::Looks {
    use crate::world::looks::Looks;
    match who {
        Who::Coach(c) => Looks::coach(w.options.seed, c as usize),
        Who::Trainer(f) | Who::SevenOnSeven(f) => Looks::new(w.options.seed, f),
        _ => Looks((super::key(&[0x10C5, salt(who)[0], salt(who)[1]]) as u32).max(1)),
    }
}

/// What he is, in words ("head coach at Destrehan").
pub fn role(w: &World, who: Who) -> String {
    match who {
        Who::Coach(c) => match w.coaches.get(c as usize).and_then(|c| c.job) {
            Some(j) => match j.employer {
                Employer::HighSchool(h) => format!("{} at {}", j.role.name(), w.school_name(h)),
                Employer::College(p) => format!("{} at {}", j.role.name(), w.program_name(p)),
                _ => "coach".into(),
            },
            None => "between jobs".into(),
        },
        Who::Trainer(_) => "private trainer".into(),
        Who::SevenOnSeven(_) => "7v7 director".into(),
        Who::Counselor(h) => format!("counselor at {}", w.school_name(h as usize)),
        Who::Family(_) => format!("{} decides", family_voice(w, who)),
        Who::Prep(k) => format!("head coach at {}", PREPS.get(k as usize).map_or("a prep school", |p| p.0)),
    }
}

/// His close ties on your ground (GR62): the people around his kids, whom
/// word reaches first.
pub fn ties(w: &World, who: Who) -> Vec<Who> {
    let mut v: Vec<Who> = Vec::new();
    match (who, kind(w, who)) {
        (Who::Coach(c), Kind::HsHead) => {
            if let Some((h, _)) = coach_school(w, c as usize) {
                v.extend(at_school(w, h).into_iter().filter(|x| *x != who));
                // The trainers and 7v7 men who work his best kids.
                for i in kids_at(w, h, 4) {
                    v.extend(trainer_of(w, i));
                    v.extend(seven_of(w, i));
                    v.push(family_of(w, i));
                }
            }
        }
        (Who::Coach(c), Kind::HsCoordinator | Kind::HsCoach) => {
            if let Some((h, _)) = coach_school(w, c as usize) {
                v.extend(head_coach_of(w, h));
            }
        }
        (Who::Counselor(h), _) => v.extend(head_coach_of(w, h as usize)),
        (Who::Family(k), _) => {
            if let Some(i) = folk::person(w, k) {
                v.extend(head_coach_of(w, w.people[i].hs));
                v.extend(trainer_of(w, i));
            }
        }
        (Who::Trainer(_) | Who::SevenOnSeven(_), _) => {
            for i in clients(w, who, 4) {
                v.extend(head_coach_of(w, w.people[i].hs));
            }
        }
        _ => {}
    }
    v.sort_unstable();
    v.dedup();
    v.retain(|x| *x != who);
    v
}

/// A school's recruits now, the best first.
pub fn kids_at(w: &World, hs: usize, n: usize) -> Vec<usize> {
    let mut v: Vec<usize> = w
        .high_schools
        .get(hs)
        .map(|h| h.roster.clone())
        .unwrap_or_default()
        .into_iter()
        .filter(|&i| w.people.get(i).is_some_and(|q| matches!(q.stage, Stage::HighSchool | Stage::PrepSchool)))
        .collect();
    v.sort_by_key(|&i| (std::cmp::Reverse(w.people[i].stars), w.people[i].player.id));
    v.truncate(n);
    v
}

/// What he wants from you, in words (people.md's table).
pub fn wants(w: &World, who: Who) -> Vec<&'static str> {
    match kind(w, who) {
        Kind::HsHead => {
            let mut v = vec!["to be asked about his kids", "the truth about his kids", "help placing his lesser kids"];
            v.push("a clinic talk");
            if let Who::Coach(c) = who
                && w.coaches.get(c as usize).is_some_and(|c| c.personality.ambition >= 13.0 && c.age < 50)
            {
                v.push("a college job someday");
            }
            v
        }
        Kind::HsCoordinator | Kind::HsCoach => vec!["to be noticed", "a step up the ladder"],
        Kind::Juco | Kind::Prep => vec!["homes for his kids moving up", "your no's to develop"],
        Kind::College => vec!["looks at his leavers", "homes for his kids"],
        Kind::Trainer => vec!["his clients seen", "his business known"],
        Kind::SevenOnSeven => vec!["his team seen", "offers for his players"],
        Kind::Counselor => vec!["a kid placed where he'll graduate"],
        Kind::Family => family_wants(w, who),
    }
}

fn family_wants(w: &World, who: Who) -> Vec<&'static str> {
    let Who::Family(k) = who else { return Vec::new() };
    let Some(i) = folk::person(w, k) else { return vec!["the truth"] };
    let d = &w.people[i].family.decider_wants;
    let mut v = [
        (d.academics, "a degree"),
        (d.playing_time, "playing time"),
        (d.close_to_home, "home close by"),
        (d.money, "money for the family"),
        (d.development, "a path to the NFL"),
        (d.prestige, "a winner"),
    ];
    v.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut out = vec!["the truth"];
    out.extend(v.iter().take(2).map(|x| x.1));
    out
}

/// His life now, in words (people.md: his job and how safe it is, his
/// season, what he's after next).
pub fn life(w: &World, who: Who) -> Vec<String> {
    let mut v = Vec::new();
    match who {
        Who::Coach(c) => {
            let Some(co) = w.coaches.get(c as usize) else { return v };
            if let Some(j) = co.job {
                let years = (w.dates.season - j.first_season()).max(0) + 1;
                v.push(format!("{years} year{} in the job", if years == 1 { "" } else { "s" }));
                if let Employer::HighSchool(h) = j.employer
                    && let Some(s) = w.high_schools.get(h).and_then(|s| s.seasons.last())
                {
                    let title = if s.state_title { ", state champions" } else { "" };
                    v.push(format!("{}-{} last season{title}", s.wins, s.losses));
                    let bad = w.high_schools[h].seasons.iter().rev().take(3).filter(|s| s.wins < s.losses).count();
                    if j.role == Role::HeadCoach && bad >= 3 {
                        v.push("on the hot seat".into());
                    }
                }
            }
            if co.age >= 62 {
                v.push("near retirement".into());
            } else if co.personality.ambition >= 13.0
                && matches!(co.job.map(|j| j.employer), Some(Employer::HighSchool(_)))
            {
                v.push("wants a college job".into());
            }
        }
        Who::Trainer(f) | Who::SevenOnSeven(f) => {
            if let Some(x) = w.formers.get(f) {
                let since = (w.year - x.since as i32).max(0);
                v.push(format!("{} since {}", x.life.name(), x.since));
                if since <= 2 {
                    v.push("building his business".into());
                }
            }
        }
        Who::Counselor(h) => {
            let (from, _) = counselor(w, h, w.year);
            v.push(format!("at the school since {from}"));
        }
        Who::Family(k) => {
            if let Some(i) = folk::person(w, k) {
                v.push(w.people[i].family.structure.phrase().to_string());
            }
        }
        Who::Prep(k) => {
            if let Some(p) = PREPS.get(k as usize) {
                v.push(format!("{}, {}", p.1, p.2));
            }
        }
    }
    v
}

/// A holder's people skill (GR23): 0.6 + 0.04 × his relationships (1-20);
/// a program on its own (its personnel staff) is an average man.
pub fn skill(w: &World, holder: Holder) -> f32 {
    match holder {
        Holder::Coach(c) => w.coaches.get(c as usize).map_or(1.0, |c| (0.6 + 0.04 * c.ability.relationships) as f32),
        Holder::Program(_) => 1.0,
    }
}

/// `post_key`'s bands: a high school head coach is 1 + the school, a high
/// school assistant `HS_STAFF` + the school, a college man `COLLEGE` + the
/// program.
pub const HS_STAFF: u32 = 0x2000_0000;
pub const COLLEGE: u32 = 0x4000_0000;

/// Where a coach is now, as a key the memory keeps to notice a move
/// (0 none): a high school, a college, anything else.
pub fn post_key(w: &World, who: Who) -> u32 {
    match who {
        Who::Coach(c) => match w.coaches.get(c as usize).and_then(|c| c.job) {
            Some(j) => match j.employer {
                Employer::HighSchool(h) if j.role == Role::HeadCoach => 1 + h as u32,
                Employer::HighSchool(h) => HS_STAFF + h as u32,
                Employer::College(p) => COLLEGE + p as u32,
                _ => 0x7FFF_FFFE,
            },
            None => 0x7FFF_FFFF,
        },
        // A counselor's key is his tenure's first year.
        Who::Counselor(h) => counselor(w, h, w.year).0 as u32,
        _ => 0,
    }
}

/// A short date in words ("March 2027").
pub fn month_year(d: Date) -> String {
    const M: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    let (y, m, _) = d.parts();
    format!("{} {y}", M[m as usize - 1])
}
