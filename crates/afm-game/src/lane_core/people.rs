//! The people and names the screens need that the world doesn't keep as
//! people yet: your off-field staff (chief of staff, trainer, the
//! communications director), the boss's name at a high school, the town's
//! paper, a state's high school association; and jersey numbers. Each name
//! is drawn once from a stream fixed by the team and the job, so it never
//! changes between screens or saves.

use afm_engine::rng::Rng;
use afm_engine::world::World;
use afm_engine::world::ethnicity::{CoachPool, coach_ethnicity};
use afm_engine::world::names::person_name;

use crate::views::TeamRef;

/// Off-field jobs on a staff (people topic 10).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Job {
    ChiefOfStaff,
    Communications,
    Trainer,
    Strength,
    Personnel,
    Operations,
    Academics,
    Compliance,
    Collective,
    CapManager,
    Principal,
    AthleticDirector,
    Statistician,
}

impl Job {
    fn salt(self) -> u32 {
        self as u32 * 0x9E37 + 0x51
    }

    /// The job's title at a level.
    pub fn title(self, t: TeamRef) -> &'static str {
        let hs = matches!(t, TeamRef::HighSchool(_));
        let nfl = matches!(t, TeamRef::Nfl(_));
        match self {
            Job::ChiefOfStaff => "Chief of staff",
            Job::Communications if nfl => "Vice president of communications",
            Job::Communications => "Director of football communications",
            Job::Trainer => "Head athletic trainer",
            Job::Strength if nfl => "Head strength and conditioning coach",
            Job::Strength => "Director of strength and conditioning",
            Job::Personnel if nfl => "Director of player personnel",
            Job::Personnel => "Director of player personnel",
            Job::Operations if hs => "Football office",
            Job::Operations => "Director of football operations",
            Job::Academics => "Director of academic services",
            Job::Compliance => "Director of compliance",
            Job::Collective => "Executive director",
            Job::CapManager => "Vice president of football administration",
            Job::Principal => "Principal",
            Job::AthleticDirector if hs => "Campus athletic coordinator",
            Job::AthleticDirector => "Director of athletics",
            Job::Statistician => "Team statistician",
        }
    }
}

fn team_seed(t: TeamRef) -> u32 {
    match t {
        TeamRef::Nfl(i) => 0x00A1_0000 ^ i as u32,
        TeamRef::College(i) => 0x00B2_0000 ^ i as u32,
        TeamRef::HighSchool(i) => 0x00C3_0000 ^ i as u32,
    }
}

/// The name of the person holding an off-field job at a team. The world
/// seed keeps it the same for a career; another world gets other people.
pub fn staff_name(w: &World, t: TeamRef, job: Job) -> String {
    // A college's athletic director is a person in the world already.
    if job == Job::AthleticDirector
        && let TeamRef::College(p) = t
        && let Some(ad) = &w.programs[p].ad
    {
        return ad.name.clone();
    }
    let mut rng = Rng::new(w.options.seed.rotate_left(7) ^ team_seed(t).wrapping_mul(0x2545_F491) ^ job.salt());
    let pool =
        if matches!(t, TeamRef::HighSchool(_)) { CoachPool::HighSchool } else { CoachPool::CollegeAssistant1990 };
    let eth = coach_ethnicity(pool, w.year, &mut rng);
    let age = 28 + (rng.next() * 30.0) as i32;
    person_name(eth, w.year - age, &mut rng)
}

/// A person outside any team (a commissioner, a league office's vice
/// president, an association's director), fixed by a key.
pub fn named(w: &World, key: &str) -> String {
    let h = key.bytes().fold(0x811C_9DC5u32, |h, b| (h ^ b as u32).wrapping_mul(0x0100_0193));
    let mut rng = Rng::new(w.options.seed.rotate_left(11) ^ h);
    let eth = coach_ethnicity(CoachPool::CollegeAssistant1990, w.year, &mut rng);
    let age = 45 + (rng.next() * 20.0) as i32;
    person_name(eth, w.year - age, &mut rng)
}

/// Your boss: the athletic director, the general manager, or at a high
/// school the principal (the campus athletic coordinator runs the sport,
/// the principal hires and fires).
pub fn boss(w: &World, t: TeamRef) -> (String, &'static str) {
    match t {
        TeamRef::College(_) => (staff_name(w, t, Job::AthleticDirector), "Director of athletics"),
        TeamRef::Nfl(i) => {
            let gm = w.clubs.get(i).map_or_else(|| staff_name(w, t, Job::CapManager), |c| c.office.gm.name.clone());
            (gm, "General manager")
        }
        TeamRef::HighSchool(_) => (staff_name(w, t, Job::Principal), "Principal"),
    }
}

/// The town's paper, whose clips your communications staff sends each
/// morning: a made-up masthead per city (real ones are trademarks).
pub fn paper(w: &World, t: TeamRef) -> (String, String) {
    let city = match t {
        TeamRef::College(i) => w.programs[i].city.clone(),
        TeamRef::HighSchool(i) => w.high_schools[i].town.clone(),
        TeamRef::Nfl(i) => nfl_city(&w.nfl_team_name(i)),
    };
    const NAMES: [&str; 10] =
        ["Herald", "Courier", "Gazette", "Register", "Tribune", "Journal", "Ledger", "Star", "Post", "Press"];
    let k = city.bytes().fold(7u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32)) as usize % NAMES.len();
    let state = match t {
        TeamRef::College(i) => w.states[w.programs[i].state].code.clone(),
        TeamRef::HighSchool(i) => w.states[w.high_schools[i].state].code.clone(),
        TeamRef::Nfl(_) => String::new(),
    };
    let line = if state.is_empty() { "Sports".to_string() } else { format!("Sports · {city}, {state}") };
    (format!("The {city} {}", NAMES[k]), line)
}

/// The national paper whose front page carries the game's biggest news.
pub const NATIONAL_PAPER: &str = "The National Football Daily";

/// A club's city from its name ("Kansas City Chiefs" -> "Kansas City").
pub fn nfl_city(name: &str) -> String {
    let words: Vec<&str> = name.split_whitespace().collect();
    if words.len() <= 1 { name.to_string() } else { words[..words.len() - 1].join(" ") }
}

/// A state's high school association by its real name where we have it.
pub fn association(code: &str, state: &str) -> String {
    let known = [
        ("TX", "University Interscholastic League"),
        ("CA", "California Interscholastic Federation"),
        ("FL", "Florida High School Athletic Association"),
        ("GA", "Georgia High School Association"),
        ("OH", "Ohio High School Athletic Association"),
        ("PA", "Pennsylvania Interscholastic Athletic Association"),
        ("AL", "Alabama High School Athletic Association"),
        ("IL", "Illinois High School Association"),
        ("NJ", "New Jersey State Interscholastic Athletic Association"),
        ("MI", "Michigan High School Athletic Association"),
        ("NC", "North Carolina High School Athletic Association"),
        ("VA", "Virginia High School League"),
        ("LA", "Louisiana High School Athletic Association"),
        ("TN", "Tennessee Secondary School Athletic Association"),
        ("SC", "South Carolina High School League"),
        ("MN", "Minnesota State High School League"),
    ];
    known
        .iter()
        .find(|k| k.0 == code)
        .map_or_else(|| format!("{state} High School Athletic Association"), |k| k.1.to_string())
}

/// A high school's district in its state's words.
pub fn district(w: &World, h: usize) -> String {
    let s = &w.high_schools[h];
    if s.private {
        return format!("{} · Office of the President", w.school_name(h));
    }
    let code = w.states[s.state].code.as_str();
    if code == "TX" { format!("{} Independent School District", s.town) } else { format!("{} Public Schools", s.town) }
}

/// Each player's jersey number on a team, by the era's numbering rules
/// with retired numbers out of circulation (the stories keep both:
/// `afm_engine::world::stories::numbers`). Returns (person index, number).
pub fn jerseys(w: &World, t: TeamRef) -> Vec<(usize, u8)> {
    use afm_engine::world::stories::{TeamKey, numbers};
    let key = match t {
        TeamRef::Nfl(i) => TeamKey::Nfl(i as u16),
        TeamRef::College(i) => TeamKey::College(i as u32),
        TeamRef::HighSchool(i) => TeamKey::Hs(i as u32),
    };
    let nums = numbers::numbers(w, key);
    crate::views::roster_people(w, t)
        .into_iter()
        .filter_map(|k| nums.get(&w.people[k].player.id).map(|&n| (k, n)))
        .collect()
}

/// One player's number on his team.
pub fn jersey(w: &World, t: TeamRef, k: usize) -> Option<u8> {
    jerseys(w, t).into_iter().find(|x| x.0 == k).map(|x| x.1)
}
