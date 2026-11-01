//! The people around a team (people topics 9, 10 and 12; takeover items 12,
//! 13 and 19): bosses, boosters, reporters and the support staff, as
//! persons with names, ages, traits and a term in the job.
//!
//! Nobody here is stored. Each person comes from a stable key (the team,
//! the role, a slot and the term he serves), so the same principal has the
//! same name, age and temper every time a screen or a review asks, and a
//! new one takes over when his term ends. People the world already keeps
//! (a college's athletic director and support heads, an NFL club's general
//! manager, owner traits and scouting director) are read from the world.
//!
//! Real shares behind the draws (sizes not published are proposed, not
//! confirmed): public school principals about 55% women (NCES 2020-21),
//! superintendents about 27% (AASA 2020), Division I athletic directors
//! about 15%, sports reporters about 15%, college athletic trainers about
//! half; a high school's athletic trainer is full time at 37% of public
//! schools, part time at 32% and absent at 31% (CATCH ON, 2015-18).

use super::Team;
use crate::rng::{Rng, clamp};
use crate::world::World;
use crate::world::data::Division;
use crate::world::ethnicity::Ethnicity;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Role {
    /// College athletic director (kept by the world).
    AthleticDirector,
    /// The university president (college).
    President,
    /// A high school's principal.
    Principal,
    /// A high school's athletic director (often the head coach in Texas;
    /// here the campus AD above you).
    SchoolAd,
    /// The district's superintendent.
    Superintendent,
    /// The NFL owner (traits kept by the world, the person drawn here).
    Owner,
    /// The owner's family: the heir in the building, a partner.
    OwnerKin,
    /// An NFL club's team president.
    TeamPresident,
    /// An NFL club's general manager (kept by the world).
    Gm,
    /// A big booster (college) or a local business owner (high school).
    Booster,
    /// A high school's booster club president.
    ClubPresident,
    /// A reporter who covers the team.
    Reporter,
    /// The communications director (sports information at a college).
    Comms,
    /// The director of football operations (college).
    Ops,
    /// Academic support: a college's director, a high school's counselor.
    Academic,
    /// The head athletic trainer.
    Trainer,
    /// The head strength and conditioning coach.
    Strength,
    /// An NFL club's cap manager (vice president of football administration).
    CapManager,
    /// An NFL club's director of college scouting, or a college's director
    /// of player personnel.
    Personnel,
    /// A feeder middle school's coach (high school).
    Feeder,
    /// The youth league's president (high school).
    Youth,
    /// Your chief of staff (college and NFL).
    ChiefOfStaff,
}

impl Role {
    pub fn name(self) -> &'static str {
        match self {
            Role::AthleticDirector => "athletic director",
            Role::President => "university president",
            Role::Principal => "principal",
            Role::SchoolAd => "athletic director",
            Role::Superintendent => "superintendent",
            Role::Owner => "owner",
            Role::OwnerKin => "owner's family",
            Role::TeamPresident => "team president",
            Role::Gm => "general manager",
            Role::Booster => "booster",
            Role::ClubPresident => "booster club president",
            Role::Reporter => "reporter",
            Role::Comms => "communications director",
            Role::Ops => "director of football operations",
            Role::Academic => "academic director",
            Role::Trainer => "head athletic trainer",
            Role::Strength => "head strength coach",
            Role::CapManager => "cap manager",
            Role::Personnel => "personnel director",
            Role::Feeder => "middle school coach",
            Role::Youth => "youth league president",
            Role::ChiefOfStaff => "chief of staff",
        }
    }

    fn code(self) -> u64 {
        self as u64 + 1
    }
}

/// Which person: a team, his role there and his place among several.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CastRef {
    pub team: Team,
    pub role: Role,
    pub slot: u8,
}

impl CastRef {
    pub fn new(team: Team, role: Role, slot: u8) -> CastRef {
        CastRef { team, role, slot }
    }
    /// His key, for trust and line memory: the job and the term he serves.
    pub fn key(self, term: u16) -> u64 {
        self.team.key() << 20 ^ self.role.code() << 12 ^ (self.slot as u64) << 8 ^ (term as u64) << 40
    }
    /// The link token for a cast person.
    pub fn token(self) -> String {
        let (k, i) = self.team.parts();
        format!("{k}:{i}:{:?}:{}", self.role, self.slot)
    }
}

/// A want a booster brings (takeover item 14), from the real kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Want {
    /// Beat the rival.
    BeatRival,
    /// Access: the sideline, the team plane, the coaches' box.
    Access,
    /// A say in hires.
    SayInHires,
    /// A say on a player (playing time for a favourite, a local kid).
    SayOnPlayer,
    /// NIL deals for his company's ads (2021 on).
    NilDeals,
    /// A clean program: no NCAA cases, no arrests.
    Clean,
    /// His name on a building.
    Building,
    /// His old high school's players recruited.
    OldSchool,
    /// Win now.
    WinNow,
}

impl Want {
    pub fn words(self) -> &'static str {
        match self {
            Want::BeatRival => "beating the rival, every year",
            Want::Access => "access: the sideline on Saturdays, a seat on the team plane, the coaches' box",
            Want::SayInHires => "a say in who you hire",
            Want::SayOnPlayer => "a say on players he likes",
            Want::NilDeals => "NIL deals that put players in his company's ads",
            Want::Clean => "a clean program: no NCAA trouble, nobody in the paper for the wrong reasons",
            Want::Building => "his name on the next building",
            Want::OldSchool => "his old high school's best players recruited",
            Want::WinNow => "winning now, not in three years",
        }
    }
    pub fn short(self) -> &'static str {
        match self {
            Want::BeatRival => "beat the rival",
            Want::Access => "access",
            Want::SayInHires => "a say in hires",
            Want::SayOnPlayer => "a say on players",
            Want::NilDeals => "NIL deals for his company",
            Want::Clean => "a clean program",
            Want::Building => "his name on a building",
            Want::OldSchool => "his old school recruited",
            Want::WinNow => "win now",
        }
    }
}

/// A booster's story.
#[derive(Clone, Debug, Default)]
pub struct BoosterInfo {
    /// His business ("car dealerships across north Alabama").
    pub business: String,
    /// What he has given ("the indoor practice facility that carries his name").
    pub gift: String,
    /// What he gives a year now, in dollars (0 when unknown).
    pub yearly: f64,
    pub wants: Vec<Want>,
    /// His old high school (an index), for the old-school want.
    pub old_school: Option<usize>,
    /// Close to the old coach: 0 none to 1 a loyalist (takeover item 15).
    pub loyal_old: f64,
    /// Who he is close to, in words ("the AD", "the president", "the board").
    pub close_to: &'static str,
    /// The big booster the world counts (the money that can walk away).
    pub big: bool,
}

/// A reporter's beat and habits (people topic 9).
#[derive(Clone, Debug, Default)]
pub struct ReporterInfo {
    pub outlet: String,
    /// "beat writer", "recruiting insider", "radio host" ...
    pub beat: &'static str,
    /// -1 a critic to 1 a homer.
    pub lean: f64,
    /// 0-1: how often his reports hold up.
    pub accuracy: f64,
    /// Who talks to him.
    pub sources: Vec<&'static str>,
    /// Leaks what he hears from inside the building.
    pub leaks: bool,
    /// Writes for a national outlet.
    pub national: bool,
}

/// A person around a team.
#[derive(Clone, Debug)]
pub struct Cast {
    pub r: CastRef,
    /// His term in the job (a new one is a new person).
    pub term: u16,
    pub name: String,
    pub she: bool,
    pub eth: Ethnicity,
    pub age: u32,
    pub since: i32,
    /// His title in full ("Athletic director", "Sports editor, the Ames Tribune").
    pub title: String,
    /// Traits 1-20: ego, temper, patience, judgement, meddling, warmth,
    /// honesty and how tight he keeps the money.
    pub ego: f64,
    pub temper: f64,
    pub patience: f64,
    pub judgement: f64,
    pub meddling: f64,
    pub warmth: f64,
    pub honesty: f64,
    pub tight: f64,
    pub booster: Option<BoosterInfo>,
    pub reporter: Option<ReporterInfo>,
}

impl Cast {
    pub fn key(&self) -> u64 {
        self.r.key(self.term)
    }
    pub fn link(&self) -> String {
        format!("[[s:people_cast?tok={}|{}]]", self.r.token(), self.name)
    }
    /// "the athletic director, [[x..|Mike Bobinski]]"
    pub fn first(&self) -> &str {
        self.name.split_whitespace().next().unwrap_or(&self.name)
    }
    pub fn last(&self) -> &str {
        let parts: Vec<&str> = self.name.split_whitespace().collect();
        match parts.as_slice() {
            [.., l, s] if matches!(*s, "Jr." | "II" | "III" | "IV") => l,
            [.., l] => l,
            _ => &self.name,
        }
    }
    pub fn he(&self) -> &'static str {
        if self.she { "she" } else { "he" }
    }
    pub fn him(&self) -> &'static str {
        if self.she { "her" } else { "him" }
    }
    pub fn his(&self) -> &'static str {
        if self.she { "her" } else { "his" }
    }
    /// -1 to 1 around 10.
    pub fn z(v: f64) -> f64 {
        (v - 10.0) / 10.0
    }

    /// What people around the building say about him: words for his
    /// strongest traits, never numbers (the screens show these).
    pub fn reputation(&self) -> Vec<&'static str> {
        let mut v: Vec<(f64, &'static str)> = Vec::new();
        let mut put = |x: f64, hi: &'static str, lo: &'static str| {
            if x >= 14.0 {
                v.push((x - 10.0, hi));
            } else if x <= 7.0 && !lo.is_empty() {
                v.push((10.0 - x, lo));
            }
        };
        put(self.ego, "Likes to be seen with the program", "Never makes it about himself");
        put(self.temper, "Has a temper the staff works around", "Hard to rattle");
        put(self.patience, "Gives a plan time to work", "Not a patient man");
        put(self.judgement, "Shrewd; people trust his reads", "Has backed the wrong people before");
        put(self.meddling, "Wants a say in football decisions", "Leaves the football to the football people");
        put(self.warmth, "Easy to like", "Keeps people at arm's length");
        put(self.honesty, "Tells you where you stand", "Says one thing in the room and another outside it");
        put(self.tight, "Watches every dollar", "Spends when he believes in something");
        v.sort_by(|a, b| b.0.total_cmp(&a.0));
        let mut out: Vec<&'static str> = v.into_iter().take(4).map(|x| x.1).collect();
        if self.she {
            for s in out.iter_mut() {
                *s = match *s {
                    "Never makes it about himself" => "Never makes it about herself",
                    "Not a patient man" => "Not a patient woman",
                    x => x,
                };
            }
        }
        if out.is_empty() {
            out.push("No strong reputation either way");
        }
        out
    }
}

/// A term's number and first year for a job that turns over every `lo` to
/// `hi` years, keyed so it never moves between runs.
fn term_at(base: u32, key: u64, year: i32, first: i32, lo: i32, hi: i32) -> (u16, i32) {
    let k = base as u64 ^ key.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    let span = (hi - lo + 1).max(1) as f64;
    let mut start = first - (crate::world::season::key_uniform(k, 0) * hi as f64) as i32;
    let mut n: u16 = 0;
    loop {
        let len = lo + (crate::world::season::key_uniform(k, n as u64 + 1) * span) as i32;
        if start + len > year || n > 400 {
            return (n, start);
        }
        start += len;
        n += 1;
    }
}

const WOMEN_OLD: [&str; 24] = [
    "Mary", "Linda", "Patricia", "Susan", "Deborah", "Karen", "Barbara", "Donna", "Sharon", "Cynthia", "Pamela",
    "Teresa", "Brenda", "Denise", "Laura", "Kimberly", "Lisa", "Michelle", "Angela", "Diane", "Carol", "Janet",
    "Julie", "Kathy",
];
const WOMEN_MID: [&str; 24] = [
    "Jennifer",
    "Amy",
    "Melissa",
    "Heather",
    "Stephanie",
    "Nicole",
    "Elizabeth",
    "Rebecca",
    "Kelly",
    "Christina",
    "Amanda",
    "Jessica",
    "Sarah",
    "Erin",
    "Shannon",
    "Kristen",
    "Rachel",
    "Megan",
    "Andrea",
    "Emily",
    "Katie",
    "Allison",
    "Lauren",
    "Courtney",
];
const WOMEN_YOUNG: [&str; 16] = [
    "Ashley", "Brittany", "Samantha", "Hannah", "Taylor", "Kayla", "Alexis", "Morgan", "Madison", "Haley", "Abby",
    "Caroline", "Olivia", "Grace", "Paige", "Natalie",
];
const WOMEN_BLACK: [&str; 16] = [
    "Tamika", "Keisha", "LaToya", "Monique", "Ebony", "Aisha", "Tasha", "Yolanda", "Crystal", "Erica", "Jasmine",
    "Kendra", "Tiffany", "Shonda", "Brandi", "Danielle",
];
const WOMEN_HISPANIC: [&str; 12] = [
    "Maria", "Rosa", "Ana", "Veronica", "Adriana", "Gabriela", "Elena", "Monica", "Claudia", "Yesenia", "Lucia",
    "Sofia",
];

/// A name: a man's from the world's name pools, a woman's from a short
/// list by era and heritage with a surname from the same pools.
pub(crate) fn person_name(eth: Ethnicity, born: i32, she: bool, rng: &mut Rng) -> String {
    let full = crate::world::names::person_name(eth, born, rng);
    if !she {
        return full;
    }
    let mut parts: Vec<&str> = full.split_whitespace().collect();
    if parts.len() > 2 && matches!(*parts.last().unwrap(), "Jr." | "II" | "III" | "IV") {
        parts.pop();
    }
    let last = parts.last().copied().unwrap_or("Smith").to_string();
    let first = match eth {
        Ethnicity::Black if rng.chance(0.5) => *rng.pick(&WOMEN_BLACK),
        Ethnicity::Hispanic if rng.chance(0.6) => *rng.pick(&WOMEN_HISPANIC),
        _ if born < 1966 => *rng.pick(&WOMEN_OLD),
        _ if born < 1988 => *rng.pick(&WOMEN_MID),
        _ => *rng.pick(&WOMEN_YOUNG),
    };
    format!("{first} {last}")
}

/// Heritage by role (sizes from the shares in the module comment and the
/// real counts of each job; proposed, not confirmed where unpublished).
fn heritage(role: Role, rng: &mut Rng) -> Ethnicity {
    let (w, b, h, a) = match role {
        Role::Principal | Role::SchoolAd | Role::Superintendent | Role::Academic | Role::Feeder | Role::Youth => {
            (0.74, 0.12, 0.11, 0.03)
        }
        Role::Owner | Role::OwnerKin | Role::Booster | Role::ClubPresident | Role::TeamPresident => {
            (0.9, 0.05, 0.03, 0.02)
        }
        Role::Reporter | Role::Comms => (0.82, 0.1, 0.05, 0.03),
        _ => (0.72, 0.2, 0.05, 0.03),
    };
    // The Asian share split into its three groups at the end of the list
    // (faces F4): one draw picks every other heritage as before.
    rng.weighted(&super::super::ethnicity::with_asian_split([w, b, h, 0.0, a]))
}

fn women_share(role: Role) -> f64 {
    match role {
        Role::Principal => 0.55,
        Role::Academic => 0.62,
        Role::Superintendent => 0.27,
        Role::SchoolAd => 0.2,
        Role::AthleticDirector => 0.15,
        Role::Reporter => 0.15,
        Role::Comms => 0.35,
        Role::Trainer => 0.3,
        Role::OwnerKin => 0.45,
        Role::President => 0.3,
        Role::ClubPresident => 0.4,
        Role::TeamPresident => 0.1,
        Role::Ops | Role::ChiefOfStaff => 0.2,
        Role::CapManager => 0.15,
        _ => 0.0,
    }
}

/// How many of a role a team has (slots), by level, size and era.
pub fn slots(w: &World, team: Team, role: Role) -> u8 {
    let year = w.year;
    match (team, role) {
        (Team::College(p), Role::Booster) => {
            let prog = &w.programs[p];
            match prog.division {
                Division::FBS if prog.power_four() => {
                    3 + (prog.prestige > 0.6) as u8 + prog.collective.big_booster.is_some() as u8
                }
                Division::FBS => 2 + (prog.prestige > 0.4) as u8,
                Division::FCS => 1 + (prog.prestige > 0.5) as u8,
                _ => 1,
            }
        }
        (Team::High(h), Role::Booster) => 1 + (w.high_schools[h].resources > 0.55) as u8,
        (Team::Nfl(_), Role::OwnerKin) => 2,
        (Team::College(p), Role::Reporter) => match w.programs[p].division {
            Division::FBS => 4 + (w.programs[p].prestige > 0.5) as u8 + (year >= 2016) as u8,
            Division::FCS => 2,
            _ => 1,
        },
        (Team::Nfl(_), Role::Reporter) => 5 + (year >= 2016) as u8,
        (Team::High(h), Role::Reporter) => {
            1 + (w.high_schools[h].prestige > 0.6 || w.high_schools[h].enrollment > 1800) as u8
        }
        // One feeder middle school per 600 students, up to four: the same
        // schools the school and feeders page lists (`front_office::school::feeders`).
        (Team::High(h), Role::Feeder) => (1 + w.high_schools[h].enrollment / 600).clamp(1, 4) as u8,
        _ => 1,
    }
}

/// Whether a team has this role at all (a small college has no
/// operations director, a high school no chief of staff).
pub fn has(w: &World, team: Team, role: Role) -> bool {
    use Role::*;
    match team {
        Team::College(p) => {
            let d = w.programs[p].division;
            match role {
                AthleticDirector | President | Booster | Reporter | Comms | Academic | Trainer | Strength => true,
                Ops => matches!(d, Division::FBS | Division::FCS),
                Personnel => d == Division::FBS && w.year >= 2012,
                ChiefOfStaff => d == Division::FBS && w.year >= 2014 && w.programs[p].prestige > 0.4,
                _ => false,
            }
        }
        // An other-league club: an owner, a general manager, a writer and the
        // trainers, no cap office or personnel department.
        Team::Other(_) => matches!(role, Owner | Gm | Reporter | Comms | Trainer | Strength),
        Team::Nfl(_) => matches!(
            role,
            Owner
                | OwnerKin
                | TeamPresident
                | Gm
                | Reporter
                | Comms
                | Trainer
                | Strength
                | CapManager
                | Personnel
                | ChiefOfStaff
        ),
        Team::High(h) => match role {
            Principal | SchoolAd | Superintendent | Booster | ClubPresident | Reporter | Academic | Feeder | Youth => {
                true
            }
            Trainer => hs_trainer(w, h) > 0,
            _ => false,
        },
    }
}

/// A high school's athletic trainer: 2 full time, 1 part time, 0 none
/// (CATCH ON shares, private schools less often), keyed by the school.
pub fn hs_trainer(w: &World, h: usize) -> u8 {
    let s = &w.high_schools[h];
    let u = super::roll(w, 0x7A1E, h as u64) - 0.25 * (s.resources - 0.5);
    let (full, part) = if s.private { (0.27, 0.55) } else { (0.37, 0.69) };
    if u < full {
        2
    } else if u < part {
        1
    } else {
        0
    }
}

/// The person in a job this year.
pub fn get(w: &World, r: CastRef) -> Cast {
    let base = w.folk.base;
    let year = w.year;
    let first = w.first_year() - 10;
    let (lo, hi) = match r.role {
        Role::President => (4, 12),
        Role::Principal => (2, 8),
        Role::SchoolAd => (3, 12),
        Role::Superintendent => (3, 9),
        Role::Owner => (14, 40),
        Role::OwnerKin => (10, 30),
        Role::TeamPresident => (5, 15),
        Role::Booster => (8, 30),
        Role::ClubPresident => (2, 6),
        Role::Reporter => (3, 15),
        Role::Comms | Role::Ops => (3, 11),
        Role::CapManager => (4, 12),
        Role::Personnel => (3, 9),
        Role::Feeder => (3, 20),
        Role::Youth => (2, 12),
        Role::ChiefOfStaff => (2, 8),
        Role::Academic | Role::Trainer | Role::Strength => (4, 14),
        Role::AthleticDirector | Role::Gm => (5, 12),
    };
    let jobkey = r.key(0);
    let (mut term, mut since) = term_at(base, jobkey, year, first, lo, hi);
    let mut rng = Rng::new(base ^ (jobkey ^ (term as u64) << 33).wrapping_mul(0x2545_F491_4F6C_DD1D) as u32);
    let she = rng.chance(women_share(r.role));
    let mut eth = heritage(r.role, &mut rng);
    let age_lo = match r.role {
        Role::Reporter | Role::Comms => 26,
        Role::Feeder | Role::Youth => 28,
        Role::OwnerKin => 30,
        Role::Owner | Role::Booster => 48,
        Role::President | Role::Superintendent => 45,
        _ => 34,
    };
    let mut age = (age_lo + rng.int(0, 22)) as u32 + (year - since).max(0) as u32 / 2;
    let born = year - age as i32;
    let mut name = person_name(eth, born, she, &mut rng);
    let mut she = she;
    // What the world keeps wins over the draw.
    let kept = |n: &str, s: i32, name: &mut String, since: &mut i32, term: &mut u16| {
        *name = n.to_string();
        *since = s;
        *term = (s - first).max(0) as u16;
    };
    match (r.team, r.role) {
        (Team::College(p), Role::AthleticDirector) => {
            if let Some(ad) = &w.programs[p].ad {
                kept(&ad.name, ad.since, &mut name, &mut since, &mut term);
                she = false;
            }
        }
        (Team::College(p), Role::Academic | Role::Trainer | Role::Strength) => {
            let s = &w.programs[p].support;
            let st = match r.role {
                Role::Academic => &s.academic,
                Role::Trainer => &s.medical,
                _ => &s.strength,
            };
            if !st.name.is_empty() {
                kept(&st.name, st.since, &mut name, &mut since, &mut term);
                she = false;
            }
        }
        (Team::Nfl(t), Role::Gm) => {
            if let Some(c) = w.clubs.get(t) {
                kept(&c.office.gm.name, c.office.gm.since, &mut name, &mut since, &mut term);
                she = false;
            }
        }
        (Team::Nfl(t), Role::Trainer | Role::Strength) => {
            if let Some(c) = w.clubs.get(t) {
                let st = if r.role == Role::Trainer { &c.support.medical } else { &c.support.strength };
                if !st.name.is_empty() {
                    kept(&st.name, st.since, &mut name, &mut since, &mut term);
                    she = false;
                }
            }
        }
        (Team::Nfl(t), Role::Personnel) => {
            if let Some(d) = w.clubs.get(t).and_then(|c| c.scouts.first()) {
                kept(&d.name, d.since, &mut name, &mut since, &mut term);
                she = false;
                age = (year - d.born).max(30) as u32;
            }
        }
        // The heir carries the family name; a partner is his own man.
        (Team::Nfl(t), Role::OwnerKin) if r.slot == 0 => {
            let owner = get(w, CastRef::new(Team::Nfl(t), Role::Owner, 0));
            let first_name = name.split_whitespace().next().unwrap_or("Sam").to_string();
            name = format!("{first_name} {}", owner.last());
            eth = owner.eth;
            age = (owner.age as i32 - 28 + rng.int(0, 6)).max(28) as u32;
        }
        _ => {}
    }
    let t = |rng: &mut Rng, m: f64| clamp(rng.normal(m, 3.5), 1.0, 20.0);
    let mut c = Cast {
        r,
        term,
        name,
        she,
        eth,
        age,
        since,
        title: String::new(),
        ego: t(&mut rng, 10.0),
        temper: t(&mut rng, 9.0),
        patience: t(&mut rng, 10.0),
        judgement: t(&mut rng, 10.0),
        meddling: t(&mut rng, 9.0),
        warmth: t(&mut rng, 10.5),
        honesty: t(&mut rng, 11.0),
        tight: t(&mut rng, 10.0),
        booster: None,
        reporter: None,
    };
    // Kept traits.
    match (r.team, r.role) {
        (Team::College(p), Role::AthleticDirector) => {
            if let Some(ad) = &w.programs[p].ad {
                c.judgement = ad.judgement;
                c.patience = ad.patience;
            }
            c.tight = clamp(14.0 - 8.0 * w.programs[p].prestige + rng.normal(0.0, 2.0), 1.0, 20.0);
        }
        (Team::Nfl(t), Role::Owner) => {
            if let Some(cl) = w.clubs.get(t) {
                let o = &cl.office.owner;
                c.meddling = clamp(1.0 + 19.0 * o.meddling, 1.0, 20.0);
                c.patience = clamp(1.0 + 19.0 * o.patience, 1.0, 20.0);
                c.ego = clamp(4.0 + 14.0 * o.pr + rng.normal(0.0, 1.5), 1.0, 20.0);
                c.tight = clamp(20.0 - 18.0 * o.cash_will, 1.0, 20.0);
            }
        }
        (Team::Nfl(t), Role::Gm) => {
            if let Some(cl) = w.clubs.get(t) {
                let g = &cl.office.gm;
                c.judgement = g.skill;
                c.ego = clamp(4.0 + 14.0 * g.overconfidence, 1.0, 20.0);
                c.patience = clamp(20.0 - 18.0 * g.impatience, 1.0, 20.0);
            }
        }
        _ => {}
    }
    c.title = title(w, &c);
    match r.role {
        Role::Booster | Role::ClubPresident | Role::OwnerKin => c.booster = Some(booster_info(w, &c, &mut rng)),
        Role::Reporter => c.reporter = Some(reporter_info(w, &c, &mut rng)),
        _ => {}
    }
    if let Some(rep) = &c.reporter {
        c.title = format!("{}, {}", cap(rep.beat), rep.outlet);
    }
    c
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn title(w: &World, c: &Cast) -> String {
    let college_sid = w.year < 2008;
    match (c.r.team, c.r.role) {
        (Team::College(_), Role::Comms) if college_sid => "Sports information director".into(),
        (Team::College(_), Role::Comms) => "Associate AD for communications".into(),
        (Team::Nfl(_), Role::Comms) => "Vice president of communications".into(),
        (Team::College(_), Role::Academic) => "Director of academic support".into(),
        (Team::High(_), Role::Academic) => "School counselor".into(),
        (Team::High(h), Role::Trainer) => {
            if hs_trainer(w, h) == 2 {
                "Athletic trainer".into()
            } else {
                "Athletic trainer (part time)".into()
            }
        }
        (Team::College(_), Role::Personnel) => "Director of player personnel".into(),
        (Team::Nfl(_), Role::Personnel) => "Director of college scouting".into(),
        (Team::Nfl(_), Role::CapManager) => "Vice president of football administration".into(),
        (Team::Nfl(_), Role::OwnerKin) if c.r.slot == 0 => "Executive vice president (the owner's family)".into(),
        (Team::Nfl(_), Role::OwnerKin) => "Limited partner".into(),
        (Team::High(_), Role::Booster) => "Local business owner".into(),
        (Team::High(_), Role::Feeder) => "Middle school coach".into(),
        (_, r) => cap(r.name()),
    }
}

const BUSINESS: [&str; 24] = [
    "car dealerships across the state",
    "a regional bank",
    "a trucking company",
    "a chain of steakhouses",
    "commercial real estate",
    "a personal-injury law firm",
    "a medical device company",
    "a homebuilding company",
    "farm equipment dealerships",
    "a private-equity fund",
    "an oil and gas company",
    "a construction company",
    "a beer distributorship",
    "a chain of convenience stores",
    "an insurance agency",
    "a software company he sold in 2019",
    "a timber company",
    "a family grocery chain",
    "an engineering firm",
    "a hotel group",
    "a sporting goods chain",
    "a heating and air company",
    "a funeral home business",
    "a regional airline he founded",
];

const HS_BUSINESS: [&str; 14] = [
    "the Ford dealership out on the highway",
    "an insurance agency on Main Street",
    "the feed and seed store",
    "a dental practice",
    "an auto body shop",
    "the barbecue place everybody eats at after games",
    "a landscaping company",
    "a heating and air company",
    "the local bank branch",
    "a real estate office",
    "a plumbing company",
    "the pharmacy downtown",
    "a roofing company",
    "a pizza place two blocks from the school",
];

fn booster_info(w: &World, c: &Cast, rng: &mut Rng) -> BoosterInfo {
    let year = w.year;
    let nil = year >= 2021;
    let mut b = BoosterInfo::default();
    match c.r.team {
        Team::College(p) => {
            let prog = &w.programs[p];
            b.business = rng.pick(&BUSINESS).to_string();
            let big = c.r.slot == 0 && prog.collective.big_booster.is_some();
            b.big = big;
            let yearly = if big {
                prog.collective.big_booster.unwrap_or(0.0)
            } else {
                prog.collective.base * (0.08 + 0.1 * rng.next()) / (1.0 + c.r.slot as f64 * 0.3)
            };
            b.yearly = yearly.max(0.0);
            let gifts: &[&str] = if big || c.r.slot == 0 {
                &[
                    "the indoor practice facility that carries his name",
                    "the founding money for the NIL collective",
                    "most of the last head coach's buyout",
                    "the new football operations building",
                    "the video board in the south end zone",
                    "the weight room renovation",
                ]
            } else {
                &[
                    "two endowed scholarships",
                    "the players' lounge",
                    "the team's charter upgrades",
                    "the recruiting lounge in the stadium",
                    "a big share of the collective's money",
                    "the nutrition station",
                ]
            };
            b.gift = rng.pick(gifts).to_string();
            if !nil && b.gift.contains("collective") {
                b.gift = "the money behind the last stadium renovation".into();
            }
            b.close_to =
                *rng.pick(&["the athletic director", "the president", "the board of trustees", "the governor"]);
        }
        Team::High(_) => {
            b.business = rng.pick(&HS_BUSINESS).to_string();
            b.yearly = 3_000.0 + 25_000.0 * rng.next();
            b.gift = rng
                .pick(&[
                    "the new scoreboard",
                    "the team's pregame meals all last season",
                    "the turf field fund",
                    "the weight room equipment",
                    "the team's new helmets",
                    "the bus to the playoff game in Waco",
                ])
                .to_string();
            if c.r.role == Role::ClubPresident {
                b.business = rng
                    .pick(&[
                        "a parent with a senior and a sophomore on the team",
                        "a parent and the school's 1998 quarterback",
                        "a parent who runs the concession stand",
                        "the father of the starting center",
                    ])
                    .to_string();
                b.gift = "the booster club's money: meals, travel and equipment".into();
            }
            b.close_to =
                *rng.pick(&["the principal", "the superintendent", "the school board", "the athletic director"]);
        }
        Team::Nfl(_) | Team::Other(_) => {
            b.business =
                if c.r.slot == 0 { "the family business and the club".into() } else { rng.pick(&BUSINESS).to_string() };
            b.gift = if c.r.slot == 0 {
                "years in the building at his father's side".into()
            } else {
                "a share of the club".into()
            };
            b.close_to = "the owner";
        }
    }
    // Wants: one or two, by what he is (takeover item 14).
    let mut pool: Vec<(Want, f64)> = vec![
        (Want::BeatRival, 1.0),
        (Want::Access, 1.0),
        (Want::SayInHires, 0.6 + 0.04 * c.meddling),
        (Want::SayOnPlayer, 0.3 + 0.04 * c.meddling),
        (Want::Clean, 0.5 + 0.03 * c.honesty),
        (Want::Building, if matches!(c.r.team, Team::College(_)) { 0.6 } else { 0.1 }),
        (Want::OldSchool, if matches!(c.r.team, Team::College(_)) { 0.5 } else { 0.0 }),
        (Want::WinNow, 0.4 + 0.04 * (20.0 - c.patience)),
    ];
    if nil && !matches!(c.r.team, Team::Nfl(_)) {
        pool.push((Want::NilDeals, 0.8));
    }
    if matches!(c.r.team, Team::Nfl(_)) {
        pool.retain(|x| matches!(x.0, Want::WinNow | Want::SayInHires | Want::SayOnPlayer | Want::Clean));
    }
    let n = if rng.chance(0.55) { 2 } else { 1 };
    for _ in 0..n {
        if pool.is_empty() {
            break;
        }
        let pick = rng.weighted(&pool);
        b.wants.push(pick);
        pool.retain(|x| x.0 != pick);
    }
    if b.wants.contains(&Want::OldSchool)
        && let Some(st) = c.r.team.state(w)
    {
        let schools = &w.states[st].schools;
        if !schools.is_empty() {
            let best = schools
                .iter()
                .copied()
                .max_by(|&a, &b| {
                    let fa = w.high_schools[a].prestige + 0.3 * crate::world::season::key_uniform(c.key(), a as u64);
                    let fb = w.high_schools[b].prestige + 0.3 * crate::world::season::key_uniform(c.key(), b as u64);
                    fa.total_cmp(&fb)
                })
                .unwrap();
            b.old_school = Some(best);
        }
    }
    // A loyalist of the old coach: the longer the old coach was here and
    // the more this man gave during his years, the closer (item 15).
    b.loyal_old = clamp(0.25 + 0.5 * rng.next() + 0.15 * Cast::z(c.warmth), 0.0, 1.0);
    b
}

fn reporter_info(w: &World, c: &Cast, rng: &mut Rng) -> ReporterInfo {
    let year = w.year;
    let town = c.r.team.town(w);
    let paper = |rng: &mut Rng| {
        let kind = *rng.pick(&[
            "Gazette", "Herald", "Courier", "Tribune", "Journal", "Times", "Register", "Star", "Press", "News",
        ]);
        if town.is_empty() { format!("the {kind}") } else { format!("the {town} {kind}") }
    };
    let k = c.r.slot;
    let mut r = ReporterInfo {
        lean: clamp(rng.normal(0.1, 0.45), -1.0, 1.0),
        accuracy: clamp(rng.normal(0.72, 0.12), 0.35, 0.97),
        ..Default::default()
    };
    r.leaks = rng.chance(0.25);
    let src_all = [
        "agents",
        "boosters",
        "parents",
        "staff",
        "former players",
        "the AD's office",
        "high school coaches",
        "scouts",
    ];
    let n = 2 + rng.int(0, 1) as usize;
    let mut srcs: Vec<&'static str> = Vec::new();
    while srcs.len() < n {
        let s = *rng.pick(&src_all);
        if !srcs.contains(&s) {
            srcs.push(s);
        }
    }
    r.sources = srcs;
    match c.r.team {
        Team::College(p) => {
            let school = w.program_name(p).to_string();
            let (outlet, beat, national) = match k {
                0 => (paper(rng), "beat writer", false),
                1 => (
                    if year >= 2021 {
                        format!("On3's {school} site")
                    } else if year >= 2011 {
                        format!("247Sports' {school} site")
                    } else if year >= 1999 {
                        format!("Rivals' {school} site")
                    } else {
                        format!("the {school} recruiting newsletter")
                    },
                    "recruiting insider",
                    false,
                ),
                2 => (format!("sports radio in {town}"), "radio host", false),
                3 => (format!("the six o'clock news in {town}"), "TV sports anchor", false),
                4 => ("ESPN".into(), "national college football writer", true),
                _ => {
                    if year >= 2016 {
                        ("The Athletic".into(), "beat writer", false)
                    } else {
                        ("the Associated Press".into(), "regional writer", true)
                    }
                }
            };
            r.outlet = outlet;
            r.beat = beat;
            r.national = national;
            if beat == "radio host" {
                r.lean = clamp(r.lean - 0.3, -1.0, 1.0);
            }
        }
        Team::Nfl(_) => {
            let (outlet, beat, national) = match k {
                0 => (paper(rng), "beat writer", false),
                1 => (paper(rng), "columnist", false),
                2 => ("the club's own website".into(), "team reporter", false),
                3 => (if year >= 2003 { "NFL Network".into() } else { "ESPN".into() }, "national insider", true),
                4 => (format!("sports radio in {town}"), "radio host", false),
                _ => ("The Athletic".into(), "beat writer", false),
            };
            r.outlet = outlet;
            r.beat = beat;
            r.national = national;
            if beat == "team reporter" {
                r.lean = 0.9;
                r.leaks = false;
            }
        }
        Team::Other(_) => {
            let (outlet, beat) = match k {
                0 | 1 => (paper(rng), "beat writer"),
                _ => (format!("sports radio in {town}"), "radio host"),
            };
            r.outlet = outlet;
            r.beat = beat;
        }
        Team::High(_) => {
            let (outlet, beat) = match k {
                0 => (paper(rng), "sports editor"),
                _ => (
                    if year >= 2004 {
                        "MaxPreps and the Friday night radio show".into()
                    } else {
                        format!("the Friday night radio show in {town}")
                    },
                    "Friday night voice",
                ),
            };
            r.outlet = outlet;
            r.beat = beat;
            r.lean = clamp(r.lean + 0.3, -1.0, 1.0);
        }
    }
    r
}

/// Everyone around a team in a role, this year.
pub fn all(w: &World, team: Team, role: Role) -> Vec<Cast> {
    if !has(w, team, role) {
        return Vec::new();
    }
    (0..slots(w, team, role)).map(|s| get(w, CastRef::new(team, role, s))).collect()
}

/// The boss who hires and fires the head coach: the athletic director, the
/// owner, or the principal.
pub fn boss_role(team: Team) -> Role {
    match team {
        Team::College(_) => Role::AthleticDirector,
        Team::Nfl(_) | Team::Other(_) => Role::Owner,
        Team::High(_) => Role::Principal,
    }
}

/// A cast person from a link token ("college:12:Booster:0").
pub fn from_token(w: &World, tok: &str) -> Option<Cast> {
    let mut it = tok.split(':');
    let kind = it.next()?;
    let id: usize = it.next()?.parse().ok()?;
    let role = it.next()?;
    let slot: u8 = it.next()?.parse().ok()?;
    let team = match kind {
        "college" if id < w.programs.len() => Team::College(id),
        "nfl" if id < w.nfl.len() => Team::Nfl(id),
        "hs" if id < w.high_schools.len() => Team::High(id),
        _ => return None,
    };
    use Role::*;
    let role = [
        AthleticDirector,
        President,
        Principal,
        SchoolAd,
        Superintendent,
        Owner,
        OwnerKin,
        TeamPresident,
        Gm,
        Booster,
        ClubPresident,
        Reporter,
        Comms,
        Ops,
        Academic,
        Trainer,
        Strength,
        CapManager,
        Personnel,
        Feeder,
        Youth,
        ChiefOfStaff,
    ]
    .into_iter()
    .find(|r| format!("{r:?}") == role)?;
    Some(get(w, CastRef::new(team, role, slot)))
}
