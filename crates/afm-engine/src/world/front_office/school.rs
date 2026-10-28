//! Your high school (lane 10; first-playable topic 7, people topics 10, 12
//! and 13): varsity, JV and freshman squads with their own games, the
//! principal, the athletic director, the superintendent and the booster
//! club as people with goals and money, your teaching job and its hours,
//! and the school's feeder area.
//!
//! Everything here is your school's alone: AI schools keep the built model
//! (their freshmen play freshman ball off the books, their whole roster
//! develops by the staff's depth chart). Draws are keyed by the career
//! (`key`), so no other draw in the world moves.

use crate::calendar::{Date, Event, Kind, Level, Loudness, Weekday};
use crate::player::PlayingTime;
use crate::positions::Position;
use crate::rng::{Rng, clamp};
use crate::world::hs_year::HS_STAT_COUNT;
use crate::world::{Stage, World};

use super::{Duty, Owner, key, key_n};

// ---------------------------------------------------------------------------
// The records
// ---------------------------------------------------------------------------

/// Which team a high schooler plays on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Squad {
    #[default]
    Varsity,
    Jv,
    Freshman,
}

impl Squad {
    pub const ALL: [Squad; 3] = [Squad::Varsity, Squad::Jv, Squad::Freshman];

    pub fn name(self) -> &'static str {
        match self {
            Squad::Varsity => "Varsity",
            Squad::Jv => "JV",
            Squad::Freshman => "Freshman",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Squad::Varsity => "varsity",
            Squad::Jv => "jv",
            Squad::Freshman => "freshman",
        }
    }

    pub fn from_key(k: &str) -> Option<Squad> {
        Squad::ALL.into_iter().find(|s| s.key() == k)
    }
}

/// One of the people your job answers to (people topic 12): a name, a
/// title, how long he has been there, what he weighs (wins, grades, the
/// numbers coming out for football, behavior), how patient he is, and his
/// trust in you, which moves with the goals and is shown only in words.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SchoolPerson {
    pub name: String,
    pub title: String,
    pub age: u32,
    pub since: i32,
    /// Weights on wins, grades, numbers and behavior (they sum to 1).
    pub wants: [f32; 4],
    /// 0-1: how long he gives a coach.
    pub patience: f32,
    /// 0-1, hidden: his trust in you.
    pub trust: f32,
    /// A plain line about him ("A former math teacher, 14 years in the district").
    pub about: String,
    /// The player on your roster he is the parent of, if any (booster presidents).
    pub child: Option<u32>,
    /// A woman (the people around the team are drawn as persons in `folk::cast`).
    pub she: bool,
    /// The person's key in `folk::cast` (the job and the term he serves):
    /// his name and trust are the ones the meeting room and his page show.
    pub cast: u64,
}

/// What each `wants` weight is about.
pub const WANTS: [&str; 4] = ["wins", "grades", "numbers", "behavior"];

/// One goal for the school year, set at the principal's start-of-year
/// meeting (or at your first meeting, the week you take the job).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Goal {
    pub kind: GoalKind,
    pub target: u32,
    /// Who set it: 0 the principal, 1 the athletic director, 2 the boosters.
    pub by: u8,
    /// Met or missed once the season is judged.
    pub met: Option<bool>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum GoalKind {
    /// At least `target` wins, playoffs included.
    Wins,
    /// Reach the playoffs (target 0), or win `target` playoff games.
    Playoffs,
    /// At most `target` players ruled ineligible for grades.
    Grades,
    /// At least `target` players out for football.
    Numbers,
    /// At most `target` players suspended or in trouble.
    Behavior,
}

impl GoalKind {
    /// Which of the four wants it serves.
    pub fn want(self) -> usize {
        match self {
            GoalKind::Wins | GoalKind::Playoffs => 0,
            GoalKind::Grades => 1,
            GoalKind::Numbers => 2,
            GoalKind::Behavior => 3,
        }
    }
}

/// The booster club (people topic 12): its president, members, the money
/// it raised and spent this football year, what it holds, and the projects
/// it has paid for. The coach's wish list says what it funds next.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Boosters {
    pub president: SchoolPerson,
    pub members: u32,
    pub balance: f64,
    /// This football year's money in: membership, the golf tournament,
    /// program ads and banners, concessions, the banquet.
    pub raised: [f64; 5],
    /// This football year's team fundraisers (the discount cards, the car
    /// wash, the lift-a-thon you run from the finances screen).
    pub fundraised: f64,
    /// This football year's money out: equipment, meals, travel, projects.
    pub spent: [f64; 4],
    /// Each line of the year's books: date, what, amount (+ in, - out).
    pub ledger: Vec<(Date, String, f64)>,
    /// Projects done: year, project index.
    pub done: Vec<(i32, u8)>,
    /// Your wish list, by project index, first wanted first.
    pub wish: Vec<u8>,
}

/// What the boosters can pay for beyond the running costs, with its price
/// (2026 dollars) and how much it adds to the school's resources (weight
/// room, film, staff help: the number every high school's development reads).
/// Proposed, not confirmed: the prices are typical quotes, the gains small.
pub const PROJECTS: [(&str, f64, f64); 7] = [
    ("Speed and agility equipment (sleds, ladders, a timing system)", 7_500.0, 0.006),
    ("A video tower and a drone for practice film", 9_000.0, 0.008),
    ("New racks, platforms and bars for the weight room", 38_000.0, 0.015),
    ("A locker room renovation", 45_000.0, 0.010),
    ("A team meeting room with film screens", 60_000.0, 0.012),
    ("A field house renovation", 180_000.0, 0.020),
    ("An artificial turf practice field", 450_000.0, 0.025),
];

/// Your teaching job (people topic 13; first-playable topic 7): most high
/// school head coaches teach, and in Texas the head coach of a big school
/// runs the campus's athletics instead of a classroom.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Teaching {
    pub subject: String,
    /// Class periods you teach, of the school day's periods.
    pub periods: u8,
    pub of_periods: u8,
    /// Campus athletic coordinator (Texas): athletics periods, no classroom.
    pub coordinator: bool,
    /// Teaching salary and the coaching stipend on top, a year.
    pub salary: f64,
    pub stipend: f64,
}

/// A JV or freshman game (the Thursday before the varsity's Friday).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SubGame {
    pub date: Date,
    pub squad: Squad,
    pub opp: usize,
    pub home: bool,
    /// Your points, theirs; None when no game was played.
    pub score: Option<[u16; 2]>,
    /// The game's best lines: player id and the line in words.
    pub top: Vec<(u32, String)>,
    /// Why there was no game.
    pub note: Option<String>,
}

/// A player's JV or freshman season so far.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SubLine {
    pub id: u32,
    pub squad: Squad,
    pub games: u8,
    pub s: [i32; HS_STAT_COUNT],
}

/// A staff suggestion about a player (move him up, call him up for the playoffs).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Suggestion {
    pub id: u32,
    pub date: Date,
    pub to: Squad,
    pub why: String,
}

/// How a season was judged: the year, each goal met or not, and each
/// person's words.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Review {
    pub year: i32,
    pub met: u32,
    pub of: u32,
    pub words: Vec<(String, String)>,
}

/// The school desk.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SchoolDesk {
    /// The school these records are for (the desk starts over at a new school).
    pub school: Option<usize>,
    /// The football year the season's lists are for.
    pub year: i32,
    /// Your squad choices this season, by player id.
    pub moves: Vec<(u32, Squad)>,
    pub principal: SchoolPerson,
    pub ad: SchoolPerson,
    pub superintendent: SchoolPerson,
    pub boosters: Boosters,
    pub teaching: Teaching,
    pub goals: Vec<Goal>,
    pub goals_year: i32,
    pub games: Vec<SubGame>,
    pub lines: Vec<SubLine>,
    pub suggestions: Vec<Suggestion>,
    /// Wins at the end of the regular season (playoff wins are counted from it).
    pub regular_wins: Option<u32>,
    /// Your school made this season's playoff field (set when it is seeded).
    pub in_playoffs: bool,
    pub reviews: Vec<Review>,
    /// The school's mascot (its youth teams carry it too).
    pub mascot: String,
    /// The day your squads were last set by you (for the screen's line).
    pub set_on: Option<Date>,
    /// Reminders already sent this season: squads, call-ups, review.
    pub sent: Vec<(i32, u8)>,
}

const SALT: u64 = 0x5C40_0000;

// ---------------------------------------------------------------------------
// Setting the desk up
// ---------------------------------------------------------------------------

const MASCOTS: [&str; 30] = [
    "Panthers",
    "Eagles",
    "Wildcats",
    "Tigers",
    "Bulldogs",
    "Cougars",
    "Mustangs",
    "Falcons",
    "Hornets",
    "Rams",
    "Raiders",
    "Warriors",
    "Trojans",
    "Spartans",
    "Knights",
    "Patriots",
    "Lions",
    "Bears",
    "Bobcats",
    "Cardinals",
    "Chargers",
    "Hawks",
    "Titans",
    "Pirates",
    "Rebels",
    "Vikings",
    "Wolverines",
    "Yellow Jackets",
    "Broncos",
    "Golden Eagles",
];

/// About what a teacher earns in each state, 2023-24 (NEA, "Rankings and
/// Estimates", 2024, rounded to the thousand). Grows 3% a year from there.
const TEACHER_PAY: [(&str, f64); 51] = [
    ("AK", 75.0),
    ("AL", 61.0),
    ("AR", 55.0),
    ("AZ", 59.0),
    ("CA", 101.0),
    ("CO", 67.0),
    ("CT", 86.0),
    ("DC", 92.0),
    ("DE", 72.0),
    ("FL", 54.0),
    ("GA", 67.0),
    ("HI", 75.0),
    ("IA", 63.0),
    ("ID", 59.0),
    ("IL", 74.0),
    ("IN", 60.0),
    ("KS", 57.0),
    ("KY", 57.0),
    ("LA", 57.0),
    ("MA", 96.0),
    ("MD", 79.0),
    ("ME", 62.0),
    ("MI", 69.0),
    ("MN", 70.0),
    ("MO", 55.0),
    ("MS", 53.0),
    ("MT", 59.0),
    ("NC", 59.0),
    ("ND", 61.0),
    ("NE", 61.0),
    ("NH", 66.0),
    ("NJ", 83.0),
    ("NM", 64.0),
    ("NV", 64.0),
    ("NY", 95.0),
    ("OH", 67.0),
    ("OK", 57.0),
    ("OR", 74.0),
    ("PA", 74.0),
    ("RI", 81.0),
    ("SC", 61.0),
    ("SD", 54.0),
    ("TN", 58.0),
    ("TX", 62.0),
    ("UT", 63.0),
    ("VA", 66.0),
    ("VT", 67.0),
    ("WA", 84.0),
    ("WI", 64.0),
    ("WV", 54.0),
    ("WY", 64.0),
];

/// A head football coach's stipend at a big school in the state, before
/// the school's size (proposed, not confirmed: from posted district
/// stipend schedules, which run from about $3,000 to $15,000).
fn stipend_base(code: &str) -> f64 {
    match code {
        "TX" => 12_000.0,
        "GA" | "AL" => 9_000.0,
        "PA" | "NJ" => 7_500.0,
        "OH" | "MI" | "LA" | "MS" | "SC" => 6_500.0,
        "FL" | "NC" | "TN" => 4_500.0,
        _ => 5_500.0,
    }
}

/// What a high school head job pays a year as an offer reads it: the
/// teaching salary and the coaching stipend (a big Texas school's
/// coordinator salary covers both).
pub(crate) fn offer_pay(w: &World, h: usize) -> f64 {
    let school = &w.high_schools[h];
    let code = w.states.get(school.state).map_or(String::new(), |s| s.code.to_string());
    let pay = TEACHER_PAY.iter().find(|x| x.0 == code).map_or(62.0, |x| x.1) * 1000.0;
    let raise = 1.03f64.powi(w.year - 2024);
    let size = (school.enrollment as f64 / 2000.0).min(1.2);
    if code == "TX" && school.enrollment >= 1500 && !school.private {
        (pay + 25_000.0 + 30_000.0 * size + 15_000.0 * school.prestige) * raise
    } else {
        (pay * 1.06 + stipend_base(&code) * (0.55 + 0.6 * size) * (1.0 + 0.4 * school.prestige)) * raise
    }
}

fn draw_name(w: &World, rng: &mut Rng, age: u32) -> String {
    let eth = crate::world::ethnicity::coach_ethnicity(crate::world::ethnicity::CoachPool::HighSchool, w.year, rng);
    crate::world::names::person_name(eth, w.year - age as i32, rng)
}

fn rng_for(w: &World, h: usize, salt: u64) -> Rng {
    Rng::new((key(w, SALT ^ salt, h as u64) * 4.0e9) as u32)
}

fn wants(rng: &mut Rng, base: [f64; 4]) -> [f32; 4] {
    let v: Vec<f64> = base.iter().map(|b| (b * (1.0 + rng.normal(0.0, 0.25))).max(0.02)).collect();
    let s: f64 = v.iter().sum();
    [(v[0] / s) as f32, (v[1] / s) as f32, (v[2] / s) as f32, (v[3] / s) as f32]
}

/// Your first trust with the school's people: your name, a local man
/// (grew up in the town, went to the school) and your presence.
fn first_trust(w: &World, h: usize, rng: &mut Rng) -> f32 {
    // A small school's people read your name as much as your standing (C10).
    let rep = crate::world::folk::facts::read_by(w, crate::world::folk::Team::High(h));
    let you = w.career.as_ref().map(|c| &c.you);
    let school = &w.high_schools[h];
    let local = you.is_some_and(|y| {
        y.profile.hometown.trim().eq_ignore_ascii_case(&school.town)
            || y.profile.high_school.trim().eq_ignore_ascii_case(w.school_name(h))
    });
    let aura = you.map_or(10.0, |y| y.aura());
    // The man they just hired starts with the benefit of the doubt: between
    // "Watching how it goes" and "Pleased with you", never on thin ice
    // before a game is played.
    clamp(
        0.55 + 0.012 * (rep - 6.0) + if local { 0.07 } else { 0.0 } + 0.008 * (aura - 10.0) + rng.normal(0.0, 0.04),
        0.46,
        0.74,
    ) as f32
}

/// Sets the desk up for your school: its people, your teaching job, the
/// booster club and the mascot. Run when you take the job (and after a
/// world file from before this build opens).
pub(crate) fn open(w: &mut World, h: usize) {
    let year = w.year;
    let school = w.high_schools[h].clone();
    let code = w.states[school.state].code.clone();
    let texas_big = code == "TX" && school.enrollment >= 1500 && !school.private;
    let mut r = rng_for(w, h, 1);
    let principal = {
        let age = r.int(38, 62) as u32;
        let name = draw_name(w, &mut r, age);
        let doctor = r.chance(0.3);
        let since = year - r.int(0, 9);
        let before = r.pick(&BEFORE);
        SchoolPerson {
            name: if doctor { format!("Dr. {name}") } else { name },
            title: "Principal".into(),
            age,
            since,
            wants: wants(&mut r, [0.2, 0.35, 0.15, 0.3]),
            patience: r.range(0.35, 0.9) as f32,
            trust: first_trust(w, h, &mut r),
            about: format!("Principal since {since}, before that {before}."),
            child: None,
            ..Default::default()
        }
    };
    let ad = {
        let age = r.int(34, 63) as u32;
        let name = draw_name(w, &mut r, age);
        let since = year - r.int(0, 12);
        let (title, about) = if code == "TX" {
            (
                "District athletic director",
                format!("Runs athletics for the whole district since {since}; a former head coach."),
            )
        } else if school.enrollment < 450 {
            ("Athletic director", format!("Athletic director since {since}, and the school's boys' basketball coach."))
        } else {
            (
                "Athletic director",
                format!("Athletic director since {since}; he coached football and track before that."),
            )
        };
        SchoolPerson {
            name,
            title: title.into(),
            age,
            since,
            wants: wants(&mut r, [0.42, 0.13, 0.25, 0.2]),
            patience: r.range(0.3, 0.85) as f32,
            trust: first_trust(w, h, &mut r),
            about,
            child: None,
            ..Default::default()
        }
    };
    let superintendent = {
        let age = r.int(45, 66) as u32;
        let name = draw_name(w, &mut r, age);
        let since = year - r.int(0, 8);
        SchoolPerson {
            name: format!("Dr. {name}"),
            title: if school.private { "Head of school".into() } else { "Superintendent".into() },
            age,
            since,
            wants: wants(&mut r, [0.25, 0.4, 0.1, 0.25]),
            patience: r.range(0.45, 0.9) as f32,
            trust: first_trust(w, h, &mut r),
            about: if school.private {
                format!("Head of school since {since}; answers to the board of trustees.")
            } else {
                format!("Superintendent since {since}; the school board hires and fires on his word.")
            },
            child: None,
            ..Default::default()
        }
    };
    // The booster club's president is a parent: a junior's or senior's father.
    let parents: Vec<u32> = school
        .roster
        .iter()
        .filter(|&&i| w.people[i].stage == Stage::HighSchool && w.people[i].school_age() >= 16)
        .map(|&i| w.people[i].player.id)
        .collect();
    let child = (!parents.is_empty()).then(|| parents[r.int(0, parents.len() as i32 - 1) as usize]);
    let president = {
        let age = r.int(40, 58) as u32;
        let first = draw_name(w, &mut r, age);
        let first = first.split_whitespace().next().unwrap_or("Mike").to_string();
        let last = child
            .and_then(|id| super::index_of(w, id))
            .and_then(|i| w.people[i].name.split_whitespace().last().map(str::to_string))
            .unwrap_or_else(|| draw_name(w, &mut r, age).split_whitespace().last().unwrap_or("Miller").to_string());
        let job = r.pick(&JOBS);
        SchoolPerson {
            name: format!("{first} {last}"),
            title: "Booster club president".into(),
            age,
            since: year - r.int(0, 3),
            wants: wants(&mut r, [0.7, 0.05, 0.1, 0.15]),
            patience: r.range(0.2, 0.7) as f32,
            trust: first_trust(w, h, &mut r),
            about: format!("A parent who {job}."),
            child,
            ..Default::default()
        }
    };
    let base = booster_base(w, h);
    let members = (40.0 + base / 260.0) as u32;
    // Your teaching job: your degree decides your subject.
    let degree = w.career.as_ref().map_or(String::new(), |c| c.you.profile.degree.to_lowercase());
    let subject = subject_for(&degree);
    let pay = TEACHER_PAY.iter().find(|x| x.0 == code).map_or(62.0, |x| x.1) * 1000.0;
    let raise = 1.03f64.powi(year - 2024);
    let size = (school.enrollment as f64 / 2000.0).min(1.2);
    let teaching = if texas_big {
        // Head coach and campus athletic coordinator: the district's own
        // pay scale for the job, about $100-150K at a big school.
        Teaching {
            subject: "Athletics".into(),
            periods: 2,
            of_periods: 8,
            coordinator: true,
            salary: (pay + 25_000.0 + 30_000.0 * size + 15_000.0 * school.prestige) * raise,
            stipend: 0.0,
        }
    } else {
        Teaching {
            subject,
            periods: if school.enrollment >= 1500 { 4 } else { 5 },
            of_periods: 7,
            coordinator: false,
            salary: pay * (1.0 + 0.12 * r.next()) * raise,
            stipend: stipend_base(&code) * (0.55 + 0.6 * size) * (1.0 + 0.4 * school.prestige) * raise,
        }
    };
    let mascot = MASCOTS[(key(w, SALT ^ 7, h as u64) * MASCOTS.len() as f64) as usize % MASCOTS.len()].to_string();
    let desk = &mut w.front_office.school;
    *desk = SchoolDesk {
        school: Some(h),
        year,
        principal,
        ad,
        superintendent,
        boosters: Boosters {
            president,
            members,
            balance: (0.25 + 0.3 * r.next()) * base,
            wish: vec![2, 1, 0, 3, 4, 5, 6],
            ..Default::default()
        },
        teaching,
        mascot,
        ..Default::default()
    };
    sync_people(w, h);
    first_goals(w, h);
}

const BEFORE: [&str; 6] = [
    "an English teacher",
    "a math teacher",
    "an assistant principal",
    "a science teacher",
    "a school counselor",
    "a history teacher",
];

const JOBS: [&str; 7] = [
    "owns a car dealership",
    "runs an insurance agency",
    "is a dentist",
    "owns a construction company",
    "manages the bank's branch",
    "farms and runs a feed store",
    "is a pharmacist",
];

/// The people you answer to are the persons the rest of the game knows
/// (`folk::cast`: the meeting room, their pages, the letters): the same
/// name, age, years in the job and temper, and one trust in you that the
/// meetings and this desk both move. Run when the desk opens, at each new
/// football year (a term can end and a new person take over) and after a
/// load.
pub(crate) fn sync_people(w: &mut World, h: usize) {
    use crate::world::folk::Team;
    use crate::world::folk::cast::{self, CastRef, Role};
    crate::world::folk::ensure_base(w);
    let school = &w.high_schools[h];
    let (private, small) = (school.private, school.enrollment < 450);
    let texas = w.states[school.state].code == "TX";
    let roster: Vec<usize> = school
        .roster
        .iter()
        .copied()
        .filter(|&i| w.people[i].stage == Stage::HighSchool && w.people[i].school_age() >= 16)
        .collect();
    for (slot, role) in
        [Role::Principal, Role::SchoolAd, Role::Superintendent, Role::ClubPresident].into_iter().enumerate()
    {
        let c = cast::get(w, CastRef::new(Team::High(h), role, 0));
        let k = c.key();
        let desk = &w.front_office.school;
        let old = match slot {
            0 => &desk.principal,
            1 => &desk.ad,
            2 => &desk.superintendent,
            _ => &desk.boosters.president,
        };
        if old.cast == k && old.name == c.name {
            continue;
        }
        let mut p = old.clone();
        let pick = |salt: u64, n: usize| (key(w, SALT ^ salt, k) * n as f64) as usize % n;
        let (he, his, since) = (c.he(), c.his(), c.since);
        p.name = c.name.clone();
        p.she = c.she;
        p.age = c.age;
        p.since = since;
        // His patience is the person's own (1-20), read as how long he gives a coach.
        p.patience = clamp(0.15 + 0.04 * c.patience, 0.2, 0.95) as f32;
        p.about = match role {
            Role::Principal => format!("Principal since {since}, before that {}.", BEFORE[pick(0xB1, BEFORE.len())]),
            Role::SchoolAd if texas => {
                format!("Runs athletics for the whole district since {since}; a former head coach.")
            }
            Role::SchoolAd if small => format!(
                "Athletic director since {since}, and the school's {} basketball coach.",
                if c.she { "girls'" } else { "boys'" }
            ),
            Role::SchoolAd => format!(
                "Athletic director since {since}; {he} coached {} and track before that.",
                if c.she { "volleyball" } else { "football" }
            ),
            Role::Superintendent if private => {
                format!("Head of school since {since}; answers to the board of trustees.")
            }
            Role::Superintendent => {
                format!("Superintendent since {since}; the school board hires and fires on {his} word.")
            }
            _ => {
                // The club's president is a parent when a player on your
                // roster carries the name; otherwise a booster whose own kids
                // came through the program.
                p.child = roster
                    .iter()
                    .find(|&&i| w.people[i].name.split_whitespace().any(|x| x == c.last()))
                    .map(|&i| w.people[i].player.id);
                let job = JOBS[pick(0xB2, JOBS.len())];
                if p.child.is_some() {
                    format!("A parent who {job}.")
                } else {
                    let mut t = format!("{he} {job}; {his} own kids came through the program.");
                    t[..1].make_ascii_uppercase();
                    t
                }
            }
        };
        p.trust = match w.folk.mine.trust_of(k) {
            Some(t) => t,
            None => {
                // The first time: the desk's own first read of you, or a new
                // person's open mind.
                let t = if old.cast == 0 && !old.name.is_empty() { old.trust } else { 0.52 };
                w.folk.mine.set_trust(k, t);
                t
            }
        };
        p.cast = k;
        let desk = &mut w.front_office.school;
        match slot {
            0 => desk.principal = p,
            1 => desk.ad = p,
            2 => desk.superintendent = p,
            _ => desk.boosters.president = p,
        }
    }
}

/// His trust in you, from the one store the meetings move too.
pub fn trust_of(w: &World, p: &SchoolPerson) -> f32 {
    if p.cast != 0 { w.folk.mine.trust_of(p.cast).unwrap_or(p.trust) } else { p.trust }
}

fn set_trust(w: &mut World, which: u8, t: f32) {
    let t = t.clamp(0.02, 0.98);
    let desk = &mut w.front_office.school;
    let p = match which {
        0 => &mut desk.principal,
        1 => &mut desk.ad,
        2 => &mut desk.superintendent,
        _ => &mut desk.boosters.president,
    };
    p.trust = t;
    if p.cast != 0 {
        let k = p.cast;
        w.folk.mine.set_trust(k, t);
    }
}

fn subject_for(degree: &str) -> String {
    let has = |k: &[&str]| k.iter().any(|x| degree.contains(x));
    if has(&["history", "social", "government", "political"]) {
        "U.S. history".into()
    } else if has(&["math", "statistic", "engineer"]) {
        "Algebra".into()
    } else if has(&["english", "journalism", "communication"]) {
        "English".into()
    } else if has(&["biology", "chemistry", "science", "physics"]) {
        "Biology".into()
    } else if has(&["business", "econom", "finance", "account", "management"]) {
        "Business".into()
    } else if has(&["health", "nutrition"]) {
        "Health".into()
    } else if has(&["special"]) {
        "Special education".into()
    } else {
        // Kinesiology, exercise science, physical education and the rest.
        "Physical education".into()
    }
}

/// The money a school's booster club raises in a normal year (proposed,
/// not confirmed: about $5K at a small rural school, $60-120K at a big
/// suburban one, several hundred thousand at a powerhouse), with the
/// last two seasons' winning.
pub fn booster_base(w: &World, h: usize) -> f64 {
    let s = &w.high_schools[h];
    let recent: Vec<&crate::world::high_school::HsSeason> = s.seasons.iter().rev().take(2).collect();
    let pct = if recent.is_empty() {
        0.5
    } else {
        recent.iter().map(|x| x.wins as f64 / (x.wins + x.losses).max(1) as f64).sum::<f64>() / recent.len() as f64
    };
    let win = 0.8 + 0.45 * pct;
    let power = if s.prestige >= 0.9 {
        2.2
    } else if s.prestige >= 0.75 {
        1.5
    } else {
        1.0
    };
    (4_000.0
        + 60_000.0 * s.area_money.powf(1.3) * (s.enrollment as f64 / 1500.0).powf(0.6) * (0.5 + s.prestige) * power)
        * win
        * if s.private { 1.3 } else { 1.0 }
}

// ---------------------------------------------------------------------------
// Squads
// ---------------------------------------------------------------------------

/// Your school, if your seat is one.
pub fn your_school(w: &World) -> Option<usize> {
    w.career.as_ref()?;
    w.seat_school()
}

/// His grade, 9-12, today.
pub fn grade(w: &World, class_of: i32) -> u32 {
    let seniors = if w.today < w.dates.hs_school_start { w.year } else { w.year + 1 };
    (12 - (class_of - seniors).clamp(0, 3)) as u32
}

/// Which teams a school fields (proposed, not confirmed: JV from about 450
/// students, a freshman team from about 1,100, each only with 22 players
/// to dress; small schools play everyone on varsity).
pub fn fields(w: &World, h: usize) -> (bool, bool) {
    let s = &w.high_schools[h];
    (s.enrollment >= 450, s.enrollment >= 1100)
}

/// His staff's read of a player (his true level with the staff's error,
/// the same eye for a whole season): what the staff's squads go by.
fn staff_read(w: &World, i: usize) -> f64 {
    let p = &w.people[i];
    p.player.overall() + 1.6 * key_n(w, SALT ^ 0x11 ^ (w.year as u64) << 20, p.player.id as u64)
}

/// The staff's squads at a school: every senior and junior on varsity
/// (a junior below the three-deep goes to JV), a sophomore in the two-deep
/// and a freshman who starts move up, the rest play JV and freshman ball,
/// varsity at most 72. A freshman team short of 22 isn't fielded and its
/// players go up a team; a short JV takes the freshmen in, or goes up too.
/// Players who left football are left out.
pub fn staff_squads(w: &World, h: usize) -> Vec<(usize, Squad)> {
    let roster: Vec<usize> =
        w.high_schools[h].roster.iter().copied().filter(|&i| w.people[i].stage == Stage::HighSchool).collect();
    let (jv, fr) = fields(w, h);
    if !jv && !fr {
        return roster.into_iter().map(|i| (i, Squad::Varsity)).collect();
    }
    let read: Vec<f64> = roster.iter().map(|&i| staff_read(w, i)).collect();
    // Depth at each position, best first.
    let mut rank = vec![0usize; roster.len()];
    for pos in Position::ALL {
        let mut at: Vec<usize> = (0..roster.len()).filter(|&k| w.people[roster[k]].player.position == pos).collect();
        at.sort_by(|&a, &b| read[b].total_cmp(&read[a]));
        for (r, &k) in at.iter().enumerate() {
            rank[k] = r;
        }
    }
    let mut out: Vec<(usize, Squad)> = roster
        .iter()
        .enumerate()
        .map(|(k, &i)| {
            let p = &w.people[i];
            let need = crate::game::roster::starters(p.player.position).max(1);
            let up = match grade(w, p.class_of) {
                12 => true,
                11 => rank[k] < 3 * need,
                10 => rank[k] < 2 * need,
                _ => rank[k] < need,
            };
            let squad = if up {
                Squad::Varsity
            } else if fr && grade(w, p.class_of) == 9 {
                Squad::Freshman
            } else if jv {
                Squad::Jv
            } else {
                Squad::Varsity
            };
            (i, squad)
        })
        .collect();
    // Varsity at most 72: the lowest reads below seniors go down a team.
    let mut v: Vec<usize> = (0..out.len()).filter(|&k| out[k].1 == Squad::Varsity).collect();
    if v.len() > 72 && jv {
        v.sort_by(|&a, &b| read[a].total_cmp(&read[b]));
        let mut extra = v.len() - 72;
        for k in v {
            if extra == 0 {
                break;
            }
            let p = &w.people[out[k].0];
            if grade(w, p.class_of) < 12 && rank[k] >= crate::game::roster::starters(p.player.position) {
                out[k].1 = Squad::Jv;
                extra -= 1;
            }
        }
    }
    settle(w, &mut out);
    out
}

/// A team too small to dress (under 22) is folded into the team above it.
/// A short JV first takes the freshmen in, one sub-varsity team as most
/// schools of that size run, rather than swelling the varsity past its 72.
fn settle(w: &World, out: &mut [(usize, Squad)]) {
    let n = |out: &[(usize, Squad)], s: Squad| out.iter().filter(|x| x.1 == s).count();
    let Some(first) = out.first() else { return };
    let jv = fields(w, w.people[first.0].hs).0;
    if n(out, Squad::Freshman) > 0 && n(out, Squad::Freshman) < 22 {
        let to = if jv { Squad::Jv } else { Squad::Varsity };
        out.iter_mut().filter(|x| x.1 == Squad::Freshman).for_each(|x| x.1 = to);
    }
    if jv
        && (1..22).contains(&n(out, Squad::Jv))
        && n(out, Squad::Freshman) > 0
        && n(out, Squad::Jv) + n(out, Squad::Freshman) >= 22
    {
        out.iter_mut().filter(|x| x.1 == Squad::Freshman).for_each(|x| x.1 = Squad::Jv);
    }
    if n(out, Squad::Jv) > 0 && n(out, Squad::Jv) < 22 {
        out.iter_mut().filter(|x| x.1 == Squad::Jv).for_each(|x| x.1 = Squad::Varsity);
    }
}

/// Your school's squads today: the staff's, with your moves on top (a team
/// your moves leave under 22 is still played as you set it).
pub fn squads(w: &World) -> Vec<(usize, Squad)> {
    let Some(h) = your_school(w) else { return Vec::new() };
    let mut out = staff_squads(w, h);
    let moves = &w.front_office.school.moves;
    if !moves.is_empty() {
        for x in out.iter_mut() {
            if let Some(&(_, s)) = moves.iter().find(|m| m.0 == w.people[x.0].player.id) {
                x.1 = s;
            }
        }
    }
    out
}

/// Your varsity's players, for the varsity game team (None at any other school).
pub(crate) fn varsity_filter(w: &World, h: usize) -> Option<Vec<usize>> {
    if your_school(w) != Some(h) {
        return None;
    }
    Some(squads(w).into_iter().filter(|x| x.1 == Squad::Varsity).map(|x| x.0).collect())
}

/// Development reps for your squads (the year's step lands at the state's
/// first game): a player your squads keep off varsity who would start on
/// it gets a JV starter's reps, not a varsity starter's. With the staff's
/// squads nobody changes, so your school develops as any other.
pub(crate) fn squad_roles(w: &World, roles: &mut [(usize, PlayingTime)]) {
    let Some(h) = your_school(w) else { return };
    let sq = squads(w);
    let roster = &w.high_schools[h].roster;
    for (i, role) in roles.iter_mut() {
        if *role == PlayingTime::Starter && roster.contains(i) && sq.iter().any(|x| x.0 == *i && x.1 != Squad::Varsity)
        {
            *role = PlayingTime::Rotation;
        }
    }
}

/// Moves a player to a squad (your choice). Seniors don't play JV (most
/// state associations and leagues bar them), and only freshmen play on
/// the freshman team.
pub fn set_squad(w: &mut World, id: u32, to: Squad) -> Result<String, String> {
    let h = your_school(w).ok_or("You don't coach a high school.")?;
    if w.front_office.owner(Duty::Squads) == Owner::Staff {
        return Err("Your staff has the squads. Take them back first.".into());
    }
    let i =
        super::index_of(w, id).filter(|i| w.high_schools[h].roster.contains(i)).ok_or("He isn't on your roster.")?;
    let p = &w.people[i];
    let g = grade(w, p.class_of);
    let (jv, fr) = fields(w, h);
    let name = p.name.clone();
    match to {
        Squad::Jv if !jv => return Err(format!("{} doesn't field a JV team.", w.school_name(h))),
        Squad::Freshman if !fr => return Err(format!("{} doesn't field a freshman team.", w.school_name(h))),
        Squad::Jv if g == 12 => return Err("Seniors don't play JV.".into()),
        Squad::Freshman if g != 9 => return Err("Only freshmen play on the freshman team.".into()),
        _ => {}
    }
    let today = w.today;
    let desk = &mut w.front_office.school;
    desk.moves.retain(|m| m.0 != id);
    desk.moves.push((id, to));
    desk.set_on = Some(today);
    desk.suggestions.retain(|s| s.id != id);
    super::note(w, format!("Moved {name} to {}.", to.name().to_lowercase()), Some(id));
    Ok(format!("{name} plays {} now.", to.name()))
}

/// Gives every squad back to the staff's lists.
pub fn reset_squads(w: &mut World) {
    let desk = &mut w.front_office.school;
    desk.moves.clear();
    desk.set_on = None;
}

// ---------------------------------------------------------------------------
// JV and freshman games
// ---------------------------------------------------------------------------

/// A squad's game team at a school (its players who may play that week).
fn sub_team(w: &World, h: usize, squad: Squad, week: usize) -> Option<(crate::game::GameTeam, Vec<(u32, usize)>)> {
    let list: Vec<(usize, Squad)> = if your_school(w) == Some(h) { squads(w) } else { staff_squads(w, h) };
    let dressed: Vec<usize> = list
        .into_iter()
        .filter(|x| x.1 == squad && crate::world::hs_year::can_play(w, x.0, week))
        .map(|x| x.0)
        .collect();
    let (mut t, ids) = crate::world::hs_year::squad_team(w, h, dressed, false)?;
    t.name = format!("{} {}", w.school_name(h), squad.name());
    Some((t, ids))
}

/// The best lines of a sub-varsity game, in words.
fn top_lines(stats: &crate::game::stats::GameStats, side: usize) -> Vec<(u32, String)> {
    let lines = &stats.players[side];
    let mut out = Vec::new();
    let best = |f: &dyn Fn(&crate::game::stats::StatLine) -> f64| {
        lines.iter().max_by(|a, b| f(a.1).total_cmp(&f(b.1))).filter(|x| f(x.1) > 0.0).map(|x| (*x.0, x.1.clone()))
    };
    if let Some((id, l)) = best(&|l| l.pass_yds) {
        out.push((
            id,
            format!(
                "{}-{}, {:.0} yds, {} TD{}",
                l.cmp,
                l.att,
                l.pass_yds,
                l.pass_td,
                if l.int > 0 { format!(", {} INT", l.int) } else { String::new() }
            ),
        ));
    }
    if let Some((id, l)) = best(&|l| l.rush_yds) {
        out.push((
            id,
            format!(
                "{} {}, {:.0} yds{}",
                l.rush,
                crate::world::folk::noun(l.rush, "carry"),
                l.rush_yds,
                if l.rush_td > 0 { format!(", {} TD", l.rush_td) } else { String::new() }
            ),
        ));
    }
    if let Some((id, l)) = best(&|l| l.rec_yds) {
        out.push((
            id,
            format!(
                "{} {}, {:.0} yds{}",
                l.rec,
                crate::world::folk::noun(l.rec, "catch"),
                l.rec_yds,
                if l.rec_td > 0 { format!(", {} TD", l.rec_td) } else { String::new() }
            ),
        ));
    }
    if let Some((id, l)) = best(&|l| (l.tackles + l.assists) as f64 + 2.0 * l.sacks + 3.0 * l.ints as f64) {
        let mut t = format!("{} {}", l.tackles + l.assists, crate::world::folk::noun(l.tackles + l.assists, "tackle"));
        if l.sacks > 0.0 {
            t += &format!(", {} sack{}", l.sacks, if l.sacks == 1.0 { "" } else { "s" });
        }
        if l.ints > 0 {
            t += &format!(", {} INT", l.ints);
        }
        out.push((id, t));
    }
    out
}

/// Thursday's JV and freshman games, the night before the varsity's
/// Friday, against the same school (each plays at the other site, as most
/// districts schedule them). Sub-varsity games run on the engine with
/// 10-minute quarters and the NFHS running clock. They are your school's
/// alone: nothing else in the world reads them.
fn sub_games(w: &mut World, h: usize, week: usize, friday: Date, d: Date) {
    let Some(f) = w.hs_fixtures(h).into_iter().find(|f| f.date == friday) else { return };
    let opp = if f.home == h { f.away } else { f.home };
    let home = f.home != h;
    let (jv, fr) = fields(w, h);
    let (ojv, ofr) = fields(w, opp);
    let state = w.high_schools[h].state;
    let code = w.states[state].code.clone();
    let wx = crate::calendar::weather::weather_at(w.options.seed as u64, state, &code, d, 18);
    for (squad, ours, theirs) in [(Squad::Jv, jv, ojv), (Squad::Freshman, fr, ofr)] {
        if !ours {
            continue;
        }
        let our_n = squads(w).iter().filter(|x| x.1 == squad).count();
        if our_n == 0 {
            continue;
        }
        let mine = sub_team(w, h, squad, week);
        let other = if theirs { sub_team(w, opp, squad, week) } else { None };
        let (Some((a, aid)), Some((b, bid))) = (mine, other) else {
            let note = if !theirs {
                format!("No {} game: {} doesn't field a {} team.", squad.name(), w.school_name(opp), squad.name())
            } else {
                format!("No {} game: one side couldn't dress 18 players.", squad.name())
            };
            w.front_office.school.games.push(SubGame {
                date: d,
                squad,
                opp,
                home,
                score: None,
                top: Vec::new(),
                note: Some(note),
            });
            continue;
        };
        let (home_t, away_t) = if home { (&a, &b) } else { (&b, &a) };
        let seed = (key(w, SALT ^ 0x6A, (w.year as u64) << 16 | (week as u64) << 2 | squad as u64) * 1.8e19) as u64;
        let mut setup = crate::game::GameSetup::new(home_t, away_t, crate::player::Level::HS, seed);
        setup.rules = if matches!(code.as_str(), "TX" | "MA") {
            crate::game::Rules::high_school_on_college_rules()
        } else {
            crate::game::Rules::for_level(crate::player::Level::HS)
        };
        setup.rules.quarter = 600.0;
        setup.cond = crate::game::Conditions {
            temperature: wx.temperature,
            wind: wx.wind,
            rain: wx.rain,
            snow: wx.snow,
            crowd: 0.08,
            ..Default::default()
        };
        let r = crate::game::play_game(&setup);
        let side = if home { 0 } else { 1 };
        let [hp, ap] = r.stats.score;
        let (us, them) = if home { (hp, ap) } else { (ap, hp) };
        let top = top_lines(&r.stats, side);
        let _ = bid;
        // Each of your players' lines for the season.
        let desk = &mut w.front_office.school;
        for &(id, _) in &aid {
            if let Some(l) = r.stats.players[side].get(&id) {
                let c = crate::world::hs_year::hs_counts(l);
                let on = l.snaps_off + l.snaps_def + l.snaps_st > 0;
                match desk.lines.iter_mut().find(|x| x.id == id && x.squad == squad) {
                    Some(x) => {
                        x.games += on as u8;
                        for (a, b) in x.s.iter_mut().zip(c) {
                            *a += b;
                        }
                    }
                    None => desk.lines.push(SubLine { id, squad, games: on as u8, s: c }),
                }
            }
        }
        let score = [us.max(0) as u16, them.max(0) as u16];
        desk.games.push(SubGame { date: d, squad, opp, home, score: Some(score), top: top.clone(), note: None });
        let won = score[0] > score[1];
        let star = top
            .first()
            .and_then(|(id, line)| super::index_of(w, *id).map(|i| format!(" {}: {line}.", w.people[i].name)));
        let text = format!(
            "{} {} {} {} {}-{}.{}",
            w.school_name(h),
            squad.name(),
            if won {
                "beat"
            } else if score[0] == score[1] {
                "tied"
            } else {
                "lost to"
            },
            w.school_name(opp),
            score[0].max(score[1]),
            score[0].min(score[1]),
            star.unwrap_or_default()
        );
        post(w, d, text, false);
    }
}

/// Your staff watches the JV and freshman tape: a player whose read is
/// at least the varsity's last man in the two-deep at his spot, after two
/// games, is ready to move up. With the squads in your hands it is a
/// suggestion; with your staff's, they move him.
fn ready_to_move_up(w: &mut World, h: usize, d: Date) {
    let sq = squads(w);
    let lines = w.front_office.school.lines.clone();
    let mut ups: Vec<(f64, Position, u32, Squad, String)> = Vec::new();
    for l in lines.iter().filter(|l| l.games >= 2) {
        let Some(i) = super::index_of(w, l.id) else { continue };
        let Some(&(_, now)) = sq.iter().find(|x| x.0 == i) else { continue };
        if now != l.squad || now == Squad::Varsity {
            continue;
        }
        if w.front_office.school.suggestions.iter().any(|s| s.id == l.id)
            || w.front_office.school.moves.iter().any(|m| m.0 == l.id)
        {
            continue;
        }
        let pos = w.people[i].player.position;
        let need = 2 * crate::game::roster::starters(pos).max(1);
        let mut v: Vec<f64> = sq
            .iter()
            .filter(|x| x.1 == Squad::Varsity && w.people[x.0].player.position == pos)
            .map(|x| staff_read(w, x.0))
            .collect();
        v.sort_by(|a, b| b.total_cmp(a));
        let bar = v.get(need - 1).copied().unwrap_or(f64::MIN);
        if staff_read(w, i) >= bar {
            let to =
                if now == Squad::Freshman && fields(w, h).0 && grade(w, w.people[i].class_of) == 9 && v.len() >= need {
                    Squad::Jv
                } else {
                    Squad::Varsity
                };
            let why = format!(
                "{} games on the {} tape: he is as good as the {} in the varsity's two-deep at {}.",
                l.games,
                l.squad.name(),
                if v.len() >= need { "last man" } else { "players" },
                pos.name()
            );
            ups.push((staff_read(w, i), pos, l.id, to, why));
        }
    }
    // The best one at each spot: one place opens, one player is named.
    ups.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut seen: Vec<Position> = w
        .front_office
        .school
        .suggestions
        .iter()
        .filter(|s| s.to == Squad::Varsity)
        .filter_map(|s| super::index_of(w, s.id).map(|i| w.people[i].player.position))
        .collect();
    ups.retain(|u| {
        let new = !seen.contains(&u.1);
        seen.push(u.1);
        new
    });
    let staff = w.front_office.owner(Duty::Squads) == Owner::Staff;
    for (_, _, id, to, why) in ups.into_iter().take(3) {
        let Some(i) = super::index_of(w, id) else { continue };
        let name = w.people[i].name.clone();
        let pos = w.people[i].player.position.name();
        if staff {
            w.front_office.school.moves.retain(|m| m.0 != id);
            w.front_office.school.moves.push((id, to));
            super::note(w, format!("Your staff moved {name} up to {}.", to.name().to_lowercase()), Some(id));
            post(w, d, format!("Your staff moved {pos} {name} up to {}: {why}", to.name().to_lowercase()), false);
        } else {
            w.front_office.school.suggestions.push(Suggestion { id, date: d, to, why: why.clone() });
            post(
                w,
                d,
                format!("Your staff thinks {pos} {name} is ready for {}: {why}", to.name().to_lowercase()),
                false,
            );
        }
    }
}

/// A call-up's reason (kept until the season ends).
const CALL_UP: &str = "The JV season is over: call him up for the last game and the playoffs.";

/// The JV season is over after the last regular-season Thursday: the best
/// JV players (sophomores and juniors) dress for the varsity's last game
/// and the playoffs, as real staffs bring them up.
fn call_ups(w: &mut World, d: Date) {
    let sq = squads(w);
    let mut jv: Vec<(f64, usize)> = sq
        .iter()
        .filter(|x| x.1 == Squad::Jv && grade(w, w.people[x.0].class_of) >= 10)
        .map(|x| (staff_read(w, x.0), x.0))
        .collect();
    if jv.is_empty() {
        return;
    }
    jv.sort_by(|a, b| b.0.total_cmp(&a.0));
    let picks: Vec<usize> = jv.iter().take(6).map(|x| x.1).collect();
    let names: Vec<String> =
        picks.iter().map(|&i| format!("{} {}", w.people[i].player.position.name(), w.people[i].name)).collect();
    if w.front_office.owner(Duty::Squads) == Owner::Staff {
        for &i in &picks {
            let id = w.people[i].player.id;
            w.front_office.school.moves.retain(|m| m.0 != id);
            w.front_office.school.moves.push((id, Squad::Varsity));
        }
        post(
            w,
            d,
            format!(
                "The JV season is over. Your staff called up {} for the rest of the varsity's season.",
                list(&names)
            ),
            false,
        );
    } else {
        for &i in &picks {
            let id = w.people[i].player.id;
            if !w.front_office.school.suggestions.iter().any(|s| s.id == id) {
                w.front_office.school.suggestions.push(Suggestion {
                    id,
                    date: d,
                    to: Squad::Varsity,
                    why: CALL_UP.into(),
                });
            }
        }
        post(
            w,
            d,
            format!(
                "The JV season is over. Your staff suggests calling up {} for the varsity's last game and the playoffs.",
                list(&names)
            ),
            true,
        );
    }
}

fn list(v: &[String]) -> String {
    match v.len() {
        0 => String::new(),
        1 => v[0].clone(),
        n => format!("{} and {}", v[..n - 1].join(", "), v[n - 1]),
    }
}

// ---------------------------------------------------------------------------
// Goals, the season's review and the school's verdict
// ---------------------------------------------------------------------------

/// Regular-season games your school plays this year.
/// The program's own goals beside the principal's (the principal sets
/// hers or his in the start-of-year meeting, `folk::boss`): the athletic
/// director's numbers out for football and, at a school that expects deep
/// runs, the booster club's playoff wins. Proposed, not confirmed: the
/// targets below, from the roster and the school's prestige.
fn set_goals(w: &mut World, h: usize, first: bool) {
    let school = &w.high_schools[h];
    let roster = school.roster.iter().filter(|&&i| w.people[i].stage == Stage::HighSchool).count() as u32;
    let numbers = (roster.saturating_sub(roster / 12) / 5 * 5).max(25);
    let mut goals = vec![Goal { kind: GoalKind::Numbers, target: numbers, by: 1, met: None }];
    if school.prestige >= 0.9 && school.state_titles >= 3 {
        goals.push(Goal { kind: GoalKind::Playoffs, target: 4, by: 2, met: None });
    } else if school.prestige >= 0.75 {
        goals.push(Goal { kind: GoalKind::Playoffs, target: 2, by: 2, met: None });
    }
    let year = w.year;
    let desk = &mut w.front_office.school;
    desk.goals = goals;
    desk.goals_year = year;
    if first {
        return;
    }
    let mut text = format!(
        "{}, the athletic director, wants at least {numbers} players out for football this year: numbers keep a program alive.",
        desk.ad.name
    );
    if let Some(g) = desk.goals.iter().find(|g| g.kind == GoalKind::Playoffs) {
        text += &format!(" The booster club's goal: {}.", goal_text(g).to_lowercase());
    }
    post(w, w.today, text, false);
}

fn first_goals(w: &mut World, h: usize) {
    set_goals(w, h, true);
}

/// The principal's goals for the season on the page: the ones set in the
/// meeting (`folk::boss`), next season's once they are set.
pub fn principal_goals(w: &World) -> (i32, Vec<crate::world::folk::boss::Goal>) {
    let season = if w.folk.goals.iter().any(|g| g.season == w.year + 1) { w.year + 1 } else { w.year };
    (season, w.folk.goals.iter().filter(|g| g.season == season).cloned().collect())
}

/// Where one of the principal's goals stands now: (in words, on track).
pub fn principal_goal_now(w: &World, h: usize, g: &crate::world::folk::boss::Goal) -> (String, bool) {
    use crate::world::folk::boss::GoalKind as K;
    let (wins, _) = if g.season == w.year { w.hs_record(h).unwrap_or((0, 0)) } else { (0, 0) };
    let roster: Vec<usize> =
        w.high_schools[h].roster.iter().copied().filter(|&i| w.people[i].stage == Stage::HighSchool).collect();
    match g.kind {
        K::Wins(n) => (format!("{wins} of {n}"), wins >= n as u32),
        K::Postseason => {
            let made = g.season == w.year && w.front_office.school.in_playoffs;
            (if made { "In the field".into() } else { String::new() }, made)
        }
        K::Grades => {
            let n = roster
                .iter()
                .filter(|&&i| {
                    let c = &w.people[i].cal;
                    c.hs_grade_out > 0 || c.academic_sit || c.fall_ineligible
                })
                .count();
            (format!("{n} ruled out, 2 at most"), n <= 2)
        }
        K::Clean => {
            let ids: Vec<u32> = roster.iter().map(|&i| w.people[i].player.id).collect();
            let n = w
                .folk
                .courts
                .closed
                .iter()
                .chain(w.folk.courts.open.iter())
                .filter(|c| c.opened.year() == g.season && ids.contains(&c.id))
                .count();
            (if n == 0 { "No cases".into() } else { format!("{n} case{}", if n == 1 { "" } else { "s" }) }, n == 0)
        }
        K::BeatRival => {
            let team = crate::world::folk::Team::High(h);
            let Some((rival, game, ..)) = crate::world::folk::facts::rival(w, team) else {
                return (String::new(), true);
            };
            let (_, r) = rival.parts();
            let played = w
                .seat_games
                .iter()
                .rev()
                .find(|x| x.date.year() == g.season && x.teams.contains(&r) && x.teams.contains(&h));
            match played {
                Some(x) => {
                    let side = if x.teams[0] == h { 0 } else { 1 };
                    let won = x.stats.score[side] > x.stats.score[1 - side];
                    (
                        format!(
                            "{} {}-{}",
                            if won { "Won" } else { "Lost" },
                            x.stats.score[side],
                            x.stats.score[1 - side]
                        ),
                        won,
                    )
                }
                None => (game.to_string(), false),
            }
        }
        _ => {
            (String::new(), crate::world::folk::boss::goal_met(w, crate::world::folk::Team::High(h), g.kind, g.season))
        }
    }
}

/// Which of the four wants a principal's goal serves.
fn principal_want(k: crate::world::folk::boss::GoalKind) -> usize {
    use crate::world::folk::boss::GoalKind as K;
    match k {
        K::Grades => 1,
        K::Clean => 3,
        _ => 0,
    }
}

/// A goal's state now: (the number it counts, whether it is met so far,
/// whether it can still be met).
pub fn goal_now(w: &World, h: usize, g: &Goal) -> (u32, bool) {
    let roster: Vec<usize> =
        w.high_schools[h].roster.iter().copied().filter(|&i| w.people[i].stage == Stage::HighSchool).collect();
    let (wins, _) = w.hs_record(h).unwrap_or((0, 0));
    match g.kind {
        GoalKind::Wins => (wins, wins >= g.target),
        GoalKind::Playoffs => {
            let made = w.front_office.school.in_playoffs;
            let pw = w.front_office.school.regular_wins.map_or(0, |r| wins.saturating_sub(r));
            if g.target == 0 { (made as u32, made) } else { (pw, pw >= g.target) }
        }
        GoalKind::Grades => {
            let n = roster.iter().filter(|&&i| w.people[i].cal.hs_grade_out > 0).count() as u32;
            (n, n <= g.target)
        }
        GoalKind::Numbers => (roster.len() as u32, roster.len() as u32 >= g.target),
        GoalKind::Behavior => {
            let n = roster.iter().filter(|&&i| w.people[i].cal.trouble || w.people[i].incident).count() as u32;
            (n, n <= g.target)
        }
    }
}

/// A goal in words.
pub fn goal_text(g: &Goal) -> String {
    match g.kind {
        GoalKind::Wins => format!("Win at least {} {}", g.target, crate::world::folk::noun(g.target, "game")),
        GoalKind::Playoffs if g.target == 0 => "Make the playoffs".into(),
        GoalKind::Playoffs if g.target >= 4 => "Win the state title".into(),
        GoalKind::Playoffs => format!("Win {} {}", g.target, crate::world::folk::noun(g.target, "playoff game")),
        GoalKind::Grades => format!(
            "No more than {} {} ruled ineligible for grades",
            g.target,
            crate::world::folk::noun(g.target, "player")
        ),
        GoalKind::Numbers => format!("Keep at least {} players out for football", g.target),
        GoalKind::Behavior => format!(
            "No more than {} {} suspended or in trouble",
            g.target,
            crate::world::folk::noun(g.target, "player")
        ),
    }
}

/// A goal's progress in words ("31 of 35", "1 now, 2 at most").
pub fn goal_progress(g: &Goal, now: u32) -> String {
    match g.kind {
        GoalKind::Wins | GoalKind::Numbers => format!("{now} of {}", g.target),
        GoalKind::Playoffs if g.target == 0 => {
            if now > 0 {
                "In the field".into()
            } else {
                String::new()
            }
        }
        GoalKind::Playoffs => format!("{now} of {} playoff wins", g.target),
        GoalKind::Grades | GoalKind::Behavior => format!("{now} now, {} at most", g.target),
    }
}

/// Trust in words: no meter, the way a coach reads his bosses.
pub fn trust_words(t: f32) -> &'static str {
    match t {
        t if t >= 0.75 => "Firmly behind you",
        t if t >= 0.6 => "Pleased with you",
        t if t >= 0.45 => "Watching how it goes",
        t if t >= 0.3 => "Losing patience",
        _ => "Wants a change",
    }
}

/// The season is judged after your last game (your state's finals): the
/// athletic director, the booster club and the superintendent read every
/// goal, the principal's and the program's, by what each cares about,
/// through his patience. The principal gives hers or his in the season
/// review meeting (`folk::boss`), which moves her or his trust.
fn review(w: &mut World, h: usize) {
    let year = w.year;
    // (text, the want it serves, met)
    let mut state: Vec<(String, usize, bool)> = Vec::new();
    let (season, pg) = principal_goals(w);
    if season == year {
        for g in &pg {
            let ok = principal_goal_now(w, h, g).1;
            state.push((g.kind.words(Level::HighSchool), principal_want(g.kind), ok));
        }
    }
    let own: Vec<(Goal, bool)> = w.front_office.school.goals.iter().map(|g| (g.clone(), goal_now(w, h, g).1)).collect();
    for (g, ok) in &own {
        state.push((goal_text(g).to_lowercase(), g.kind.want(), *ok));
    }
    if state.is_empty() {
        return;
    }
    for (g, (_, m)) in w.front_office.school.goals.iter_mut().zip(&own) {
        g.met = Some(*m);
    }
    let met = state.iter().filter(|x| x.2).count() as u32;
    let of = state.len() as u32;
    let missed: Vec<String> = state.iter().filter(|x| !x.2).map(|x| x.0.clone()).collect();
    let goals = |n: usize| if n == 1 { "a goal".to_string() } else { format!("{n} goals") };
    let mut words = Vec::new();
    for which in [1u8, 3] {
        let p = if which == 1 { &w.front_office.school.ad } else { &w.front_office.school.boosters.president };
        let (name, wants, patience) = (p.name.clone(), p.wants, p.patience);
        let mut d = 0.0f32;
        for (_, want, m) in &state {
            d += wants[*want] * if *m { 0.14 } else { -0.16 / (0.5 + patience) };
        }
        let t = (trust_of(w, p) + d).clamp(0.02, 0.98);
        set_trust(w, which, t);
        let line = match (d >= 0.02, missed.is_empty()) {
            (_, true) => "Every goal met. Exactly what this school wanted.".to_string(),
            (true, false) => format!("A good year, but we missed {}: {}.", goals(missed.len()), missed.join("; ")),
            (false, false) if t < 0.3 => {
                format!(
                    "We missed {}: {}. We can't keep having this conversation.",
                    goals(missed.len()),
                    missed.join("; ")
                )
            }
            (false, false) => format!("We fell short on {}: {}.", goals(missed.len()), missed.join("; ")),
        };
        words.push((name, line));
    }
    // The superintendent follows the principal's read of you.
    let (p, s) = (trust_of(w, &w.front_office.school.principal), trust_of(w, &w.front_office.school.superintendent));
    set_trust(w, 2, s + 0.5 * (p - s) * 0.3);
    let ad_title = w.front_office.school.ad.title.clone();
    w.front_office.school.reviews.push(Review { year, met, of, words: words.clone() });
    let text = format!("The season's review: {met} of {of} goals met. {} ({ad_title}): \"{}\"", words[0].0, words[0].1);
    post(w, w.today, text, false);
}

/// The school's verdict on its coach in January (the carousel's day): a
/// non-renewal of the coaching stipend, as high schools do it, when both
/// the principal and the athletic director have lost faith, after two
/// seasons' grace (proposed, not confirmed). You keep teaching. Replaces
/// the AI's review for your seat.
pub(crate) fn verdict(w: &World, tenure: i32) -> bool {
    let d = &w.front_office.school;
    let (p, a) = (trust_of(w, &d.principal), trust_of(w, &d.ad));
    (tenure >= 2 && p < 0.25 && a < 0.35) || (tenure >= 1 && p < 0.12 && a < 0.15)
}

// ---------------------------------------------------------------------------
// The booster club's year
// ---------------------------------------------------------------------------

fn money_in(w: &mut World, d: Date, k: usize, what: String, x: f64) {
    let b = &mut w.front_office.school.boosters;
    b.raised[k] += x;
    b.balance += x;
    b.ledger.push((d, what, x));
}

/// A team fundraiser's take, paid into the booster club's account
/// (`finance::hs`): on its ledger with the year's other money in.
pub(crate) fn fundraiser_in(w: &mut World, what: &str, x: f64) {
    let d = w.today;
    let b = &mut w.front_office.school.boosters;
    b.fundraised += x;
    b.balance += x;
    b.ledger.push((d, what.to_string(), x));
}

fn money_out(w: &mut World, d: Date, k: usize, what: String, x: f64) {
    let b = &mut w.front_office.school.boosters;
    b.spent[k] += x;
    b.balance -= x;
    b.ledger.push((d, what, -x));
}

/// The year's booster money (proposed, not confirmed: the shares of a
/// typical club's year): membership in the spring drive (20%), the golf
/// tournament in June (20%), program ads and stadium banners in August
/// (25%), concessions at each home game (25%), the banquet in December
/// (10%). Out: pregame meals every game, charter buses for long road
/// trips and every playoff trip, helmet reconditioning in May, new gear
/// in July, the film subscription, and a project off your wish list
/// when the money is there at the end of June.
fn booster_day(w: &mut World, h: usize, d: Date) {
    let (y, m, day) = d.parts();
    let base = booster_base(w, h) * (1.0 + 0.1 * key_n(w, SALT ^ 0xB0, (y as u64) << 8));
    let roster = w.high_schools[h].roster.len() as f64;
    if (m, day) == (4, 15) {
        let n = (w.front_office.school.boosters.members as f64 * (0.95 + 0.1 * key(w, SALT ^ 0xB1, y as u64))) as u32;
        w.front_office.school.boosters.members = n.max(20);
        money_in(w, d, 0, format!("Spring membership drive ({n} members)"), 0.2 * base);
    }
    if d == Date::nth_weekday(y, 6, Weekday::Sat, 1) {
        money_in(w, d, 1, "The booster golf tournament".into(), 0.2 * base);
    }
    if (m, day) == (8, 1) {
        money_in(w, d, 2, "Program ads and stadium banners".into(), 0.25 * base);
    }
    if (m, day) == (5, 15) {
        money_out(w, d, 0, format!("Helmet reconditioning ({} helmets)", roster as u32), 38.0 * roster);
    }
    if (m, day) == (7, 15) {
        let n = (0.12 * roster).ceil();
        money_out(w, d, 0, format!("New helmets ({n}), shoulder pads and practice gear"), 420.0 * n + 25.0 * roster);
        money_out(w, d, 0, "The year's film subscription".into(), 3_300.0);
    }
    if d == Date::nth_weekday(y, 12, Weekday::Sat, 2) {
        money_in(w, d, 4, "The football banquet".into(), 0.1 * base);
        money_out(w, d, 1, "The banquet dinner".into(), 32.0 * roster);
    }
    // A home game's concessions; every game's pregame meal; long trips.
    if let Some(f) = w.hs_fixtures(h).into_iter().find(|f| f.date == d) {
        let n_home = w.hs_fixtures(h).iter().filter(|f| f.home == h).count().max(1) as f64;
        let varsity = squads(w).iter().filter(|x| x.1 == Squad::Varsity).count() as f64;
        money_out(w, d, 1, "Pregame meal".into(), 11.0 * varsity.max(30.0));
        let opp = if f.home == h { f.away } else { f.home };
        if f.home == h {
            money_in(w, d, 3, format!("Concessions, home game vs {}", w.school_name(opp)), 0.25 * base / n_home);
        } else if w.high_schools[opp].area != w.high_schools[h].area {
            money_out(w, d, 2, format!("Charter buses to {}", w.school_name(opp)), 1_900.0);
        }
    } else if w.hs_alive(h) && w.hs_weeks(h).is_some_and(|(_, rounds, _)| rounds.contains(&d)) {
        money_out(w, d, 2, "Charter buses and a meal on the road, playoffs".into(), 2_600.0);
    }
    // The end of June: a project off your wish list, if the money is there.
    if (m, day) == (6, 30) {
        let reserve = 0.3 * base;
        let b = &w.front_office.school.boosters;
        let next = b.wish.iter().copied().find(|k| !b.done.iter().any(|x| x.1 == *k));
        if let Some(k) = next {
            let (name, cost, gain) = PROJECTS[k as usize];
            let cost = cost * 1.03f64.powi(y - 2026);
            if b.balance - cost >= reserve {
                money_out(w, d, 3, name.to_string(), cost);
                w.front_office.school.boosters.done.push((y, k));
                let s = &mut w.high_schools[h];
                s.resources = (s.resources + gain).min(1.0);
                post(w, d, format!("The booster club voted to pay for {}.", name.to_lowercase()), false);
            } else {
                let short = cost + reserve - b.balance;
                post(
                    w,
                    d,
                    format!(
                        "The booster club can't fund {} yet: it is about {} short.",
                        name.to_lowercase(),
                        super::dollars(short)
                    ),
                    false,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Your hours
// ---------------------------------------------------------------------------

/// Your week's hours on a date: (teaching, coaching), and what the week is.
/// A school week is about 38 hours in the building plus an hour a day for
/// each class you plan and grade; coaching runs from a dozen hours in the
/// offseason program to 35 or more in season (proposed, not confirmed,
/// from coaches' own accounts).
pub fn hours(w: &World, d: Date) -> (f64, f64, &'static str) {
    let Some(h) = your_school(w) else { return (0.0, 0.0, "") };
    let t = &w.front_office.school.teaching;
    let (y, m, _) = d.parts();
    let school_in = !(m == 6 && d.day() > 5 || m == 7 || m == 8 && d < w.dates.hs_school_start);
    let teach = if !school_in {
        0.0
    } else if t.coordinator {
        44.0
    } else {
        38.0 + 1.0 * t.periods as f64
    };
    let weeks = w.hs_weeks(h);
    let (season_start, season_end) = weeks
        .as_ref()
        .and_then(|(wk, rounds, _)| Some((*wk.first()?, *rounds.last().or(wk.last())?)))
        .unwrap_or((Date::ymd(y, 8, 28), Date::ymd(y, 11, 15)));
    let alive = w.hs_alive(h) || d <= weeks.as_ref().and_then(|x| x.0.last().copied()).unwrap_or(season_end);
    let (coach, what) = if d >= season_start.add(-24) && d < season_start {
        (45.0, "Fall camp: two-a-days before school starts")
    } else if d >= season_start && d <= season_end && alive {
        (34.0, "In season: practice, Friday nights, Saturday film, Sunday game plans")
    } else if (3..=5).contains(&m) {
        (14.0, "The offseason program and spring ball")
    } else if m == 6 || m == 7 {
        (11.0, "Summer: weights, 7-on-7, camps")
    } else if m == 12 || m <= 2 {
        (10.0, "The offseason: weights, the banquet, college coaches visiting")
    } else {
        (12.0, "The offseason program")
    };
    (teach, coach, what)
}

// ---------------------------------------------------------------------------
// Feeder area
// ---------------------------------------------------------------------------

/// A feeder school or youth team.
#[derive(Clone, Debug)]
pub struct Feeder {
    pub name: String,
    pub youth: bool,
    pub coach: String,
    /// The coach's (or the youth league president's) page token, when the
    /// game knows him as a person (`folk::cast`).
    pub tok: Option<String>,
    /// Boys in its football program.
    pub boys: u32,
    /// Its share of your freshmen.
    pub share: f64,
    /// Last fall, in words.
    pub last: String,
}

const MIDDLE_NAMES: [&str; 10] = [
    "Lincoln",
    "Washington",
    "Jefferson",
    "Roosevelt",
    "Kennedy",
    "Franklin",
    "Jackson",
    "Wilson",
    "Madison",
    "Monroe",
];

/// Your school's feeder middle schools and youth teams (generated from the
/// school's size and town, the same every time it is asked).
pub fn feeders(w: &World, h: usize) -> Vec<Feeder> {
    use crate::world::folk::Team;
    use crate::world::folk::cast::{self, CastRef, Role};
    let s = &w.high_schools[h];
    let code = &w.states[s.state].code;
    let junior_high = matches!(code.as_str(), "TX" | "UT" | "ID" | "OK" | "AR" | "MS" | "AL");
    let n = (1 + s.enrollment / 600).clamp(1, 4) as usize;
    let mut r = rng_for(w, h, 9);
    let mut out = Vec::new();
    let mut weights: Vec<f64> = (0..n).map(|_| r.range(0.6, 1.4)).collect();
    if s.private {
        weights = vec![1.0; n];
    }
    let total: f64 = weights.iter().sum::<f64>() * 1.18;
    let class = (s.enrollment as f64 / 4.0 * 0.11).round().max(8.0);
    let year = w.year;
    for k in 0..n {
        let name = if s.private {
            format!("{} Middle School", ["St. Mary's", "Holy Cross", "Trinity", "St. Joseph's"][k % 4])
        } else if k == 0 {
            format!("{} {}", s.town, if junior_high { "Junior High" } else { "Middle School" })
        } else {
            format!(
                "{} {}",
                MIDDLE_NAMES[(r.int(0, 9) as usize + k) % 10],
                if junior_high { "Junior High" } else { "Middle School" }
            )
        };
        let age = r.int(26, 60) as u32;
        let drawn = draw_name(w, &mut r, age);
        // Its coach is the person the meeting room knows (`folk::cast`).
        let (coach, tok) = if k < cast::slots(w, Team::High(h), Role::Feeder) as usize {
            let c = cast::get(w, CastRef::new(Team::High(h), Role::Feeder, k as u8));
            (c.name, Some(c.r.token()))
        } else {
            (drawn, None)
        };
        let (wn, l) = {
            let wn = r.int(1, 8);
            (wn, 8 - wn)
        };
        out.push(Feeder {
            name,
            youth: false,
            coach,
            tok,
            boys: (2.2 * class * weights[k] / total * n as f64 / n as f64 * 1.0).round().max(12.0) as u32,
            share: weights[k] / total,
            last: format!("{wn}-{l} in the eighth-grade season, fall {}", year - 1),
        });
    }
    let mascot = &w.front_office.school.mascot;
    let mascot = if mascot.is_empty() { "Panthers" } else { mascot.as_str() };
    let age = r.int(30, 55) as u32;
    let _ = draw_name(w, &mut r, age);
    // The youth league's president runs it (`folk::cast`).
    let youth = cast::get(w, CastRef::new(Team::High(h), Role::Youth, 0));
    let (wn, l) = {
        let wn = r.int(2, 10);
        (wn, 10 - wn)
    };
    out.push(Feeder {
        name: format!("{} Jr. {mascot}", s.town),
        youth: true,
        coach: youth.name.clone(),
        tok: Some(youth.r.token()),
        boys: (class * 1.4).round() as u32,
        share: 0.18 / 1.18,
        last: format!("Seventh and eighth grade youth tackle, {wn}-{l} last fall"),
    });
    out
}

/// Where a player came up: a feeder (its index), or None for one who
/// moved in or transferred.
pub fn feeder_of(w: &World, h: usize, i: usize) -> Option<usize> {
    feeder_in(&feeders(w, h), w, h, i)
}

/// `feeder_of` with the school's feeders already drawn (a whole roster
/// at once).
pub fn feeder_in(f: &[Feeder], w: &World, h: usize, i: usize) -> Option<usize> {
    let p = &w.people[i];
    if p.transfers > 0 || p.hs != h {
        return None;
    }
    let u = key(w, SALT ^ 0xFE, p.player.id as u64);
    if u > 0.93 {
        return None;
    }
    let mut acc = 0.0;
    let mid: Vec<&Feeder> = f.iter().filter(|x| !x.youth).collect();
    let s: f64 = mid.iter().map(|x| x.share).sum();
    for (k, x) in mid.iter().enumerate() {
        acc += x.share / s * 0.93;
        if u < acc {
            return Some(k);
        }
    }
    Some(mid.len().saturating_sub(1))
}

/// Whether he played youth tackle for the town's youth team before middle school.
pub fn youth_player(w: &World, i: usize) -> bool {
    key(w, SALT ^ 0xFD, w.people[i].player.id as u64) < 0.55
}

// ---------------------------------------------------------------------------
// The day
// ---------------------------------------------------------------------------

fn post(w: &mut World, d: Date, text: String, decision: bool) {
    let mut e = Event::new(
        d,
        Level::HighSchool,
        if decision { Kind::Deadline } else { Kind::News },
        if decision { Loudness::Medium } else { Loudness::Small },
        text,
    )
    .rule("Your school");
    if decision {
        e = e.decision();
    }
    w.calendar.post(e);
}

fn sent(w: &mut World, k: u8) -> bool {
    let y = w.year;
    let s = &mut w.front_office.school.sent;
    if s.contains(&(y, k)) {
        return true;
    }
    s.push((y, k));
    false
}

/// The school desk's day.
pub(crate) fn day(w: &mut World, d: Date) {
    let Some(h) = your_school(w) else { return };
    if w.front_office.school.school != Some(h) {
        open(w, h);
    }
    // A new football year: last season's sub-varsity books close.
    if w.front_office.school.year != w.year {
        let year = w.year;
        let desk = &mut w.front_office.school;
        desk.year = year;
        desk.games.clear();
        desk.lines.clear();
        desk.suggestions.clear();
        desk.moves.clear();
        desk.set_on = None;
        desk.regular_wins = None;
        desk.in_playoffs = false;
        desk.sent.retain(|x| x.0 >= year - 1);
        let b = &mut desk.boosters;
        b.raised = [0.0; 5];
        b.fundraised = 0.0;
        b.spent = [0.0; 4];
        b.ledger.clear();
        sync_people(w, h);
    }
    // School starts: the athletic director's and the boosters' goals for
    // the program (the principal's come from the start-of-year meeting).
    if d == w.dates.hs_school_start && w.front_office.school.goals_year != w.year {
        set_goals(w, h, false);
    }
    booster_day(w, h, d);
    let Some((weeks, rounds, done)) = w.hs_weeks(h) else { return };
    if weeks.is_empty() {
        return;
    }
    // Suggestions go stale: a move-up after two weeks (the newer tape has
    // its own), a call-up when the season ends, all of them once it has.
    let over = done && rounds.last().is_some_and(|&f| d > f);
    w.front_office.school.suggestions.retain(|s| !over && (s.why == CALL_UP || s.date.add(14) >= d));
    // A week before the first game: the squads are set.
    if d == weeks[0].add(-8) && !sent(w, 1) {
        let sq = squads(w);
        let n = |s: Squad| sq.iter().filter(|x| x.1 == s).count();
        let (jv, fr) = fields(w, h);
        let mut text = format!("Your staff's squads for the season: {} varsity", n(Squad::Varsity));
        if jv {
            text += &format!(", {} JV", n(Squad::Jv));
        }
        if fr {
            text += &format!(", {} freshmen", n(Squad::Freshman));
        }
        let yours = w.front_office.owner(Duty::Squads) == Owner::You && (jv || fr);
        if yours {
            text += ". Move anyone before the first game: the year's growth lands with it, and a player who would start on varsity gets only a JV starter's reps on JV.";
        } else {
            text += ".";
        }
        post(w, d, text, yours);
    }
    // Thursday: JV and freshman games.
    if let Some(k) = weeks.iter().position(|&f| f.add(-1) == d) {
        sub_games(w, h, k, weeks[k], d);
        if k >= 1 {
            ready_to_move_up(w, h, d);
        }
    }
    // The regular season's wins, for the playoff goals.
    if d == weeks[weeks.len() - 1].add(1) {
        let wins = w.hs_record(h).map_or(0, |r| r.0);
        w.front_office.school.regular_wins = Some(wins);
        if !sent(w, 2) {
            call_ups(w, d);
        }
    }
    // The season is judged once your state's season is over.
    if done && !sent(w, 3) && rounds.last().is_some_and(|&f| d > f) {
        review(w, h);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subjects_follow_the_degree() {
        assert_eq!(subject_for("b.s. kinesiology"), "Physical education");
        assert_eq!(subject_for("history"), "U.S. history");
        assert_eq!(subject_for("mathematics"), "Algebra");
        assert_eq!(subject_for("business administration"), "Business");
    }

    #[test]
    fn trust_reads_in_words() {
        assert_eq!(trust_words(0.8), "Firmly behind you");
        assert_eq!(trust_words(0.5), "Watching how it goes");
        assert_eq!(trust_words(0.1), "Wants a change");
    }

    #[test]
    fn every_state_has_a_teacher_pay() {
        let codes: std::collections::HashSet<&str> = TEACHER_PAY.iter().map(|x| x.0).collect();
        assert_eq!(codes.len(), 51);
        assert!(TEACHER_PAY.iter().all(|x| (40.0..120.0).contains(&x.1)));
    }

    #[test]
    fn goals_read_plainly() {
        let g = Goal { kind: GoalKind::Playoffs, target: 0, by: 1, met: None };
        assert_eq!(goal_text(&g), "Make the playoffs");
        let g = Goal { kind: GoalKind::Wins, target: 7, by: 1, met: None };
        assert_eq!(goal_text(&g), "Win at least 7 games");
    }

    #[test]
    fn squads_have_keys() {
        for s in Squad::ALL {
            assert_eq!(Squad::from_key(s.key()), Some(s));
        }
    }
}
