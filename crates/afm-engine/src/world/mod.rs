//! Section 2: the world. Real colleges and conferences, real states with
//! generated high schools (real powerhouses by name), coaches, programs with
//! prestige and grades, NIL collectives, and every player's hidden background
//! and family. The world runs year by year, and builds 30 years of hidden
//! history before the 2026 save starts.
//!
//! Spec: `docs/design/world.md`. Hidden values (a player's background, a
//! coach's true ability) stay inside the engine; the game sees reputations,
//! grades, character reports and news.
//!
//! Game results here are a stand-in until the match engine (section 3): each
//! game is decided from the two teams' starters, with home field and luck.

pub mod background;
pub mod coach;
pub mod data;
pub mod geo;
pub mod high_school;
pub mod names;
pub mod news;
pub mod program;
mod recruiting;
mod season;
mod staff;

use rayon::prelude::*;

use crate::career::{Step, first_year_adjustment};
use crate::player::{Level, Player, PlayingTime, ProspectOptions, generate_prospect};
use crate::rng::{Rng, clamp};
use background::{Background, BackgroundSeed, Family, apply_background, generate_background, generate_family};
use coach::Coach;
use data::Division;
use high_school::{HighSchool, TransferRule};
use names::NameMode;
use news::NewsItem;
use program::Program;

#[derive(Clone, Debug)]
pub struct WorldOptions {
    pub seed: u32,
    /// 1.0 = a real-size world: every high school program and a national class
    /// of about 250,000. Tests use a small share.
    pub scale: f64,
    /// Hidden history before the save starts (confirmed: 30 years).
    pub history_years: u32,
    /// The year the save starts.
    pub start_year: i32,
    pub names: NameMode,
}

impl Default for WorldOptions {
    fn default() -> Self {
        WorldOptions { seed: 2026, scale: 1.0, history_years: 30, start_year: 2026, names: NameMode::Real }
    }
}

/// Where a player is in his football life.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    HighSchool,
    /// A year at a prep school to fix grades or get bigger (topic 2, 1.6).
    PrepSchool,
    College,
    /// Left for the pros (drafted or undrafted); archived.
    Pro,
    /// Left football.
    Done,
}

/// One state's football setting.
#[derive(Clone, Debug)]
pub struct StateInfo {
    pub code: String,
    pub transfer: TransferRule,
    /// Whether high school players may earn NIL money (topic 8; filled from data).
    pub hs_nil: bool,
    /// Football culture: participation, spring ball, 7v7, coaching depth. A
    /// shift in the talent draw, calibrated so FBS signees per state match the
    /// real 2022-26 counts.
    pub culture: f64,
    /// Share of the national class growing up here.
    pub class_share: f64,
    pub schools: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct Person {
    pub name: String,
    pub player: Player,
    pub(crate) rng: Rng,
    pub(crate) bg: Background,
    pub(crate) family: Family,
    pub home_state: usize,
    pub hs: usize,
    pub stage: Stage,
    pub college: Option<usize>,
    /// Year of his last high school season's spring (his class).
    pub class_of: i32,
    pub stars: u8,
    /// Hidden visibility to college recruiters, 0-1 (topic 4).
    pub(crate) visibility: f64,
    pub college_seasons: u32,
    pub redshirt_used: bool,
    /// Share of next season he must sit after a high school transfer.
    pub sit_out: f64,
    /// Signed where his parent or handler wanted, not where he wanted (topic 5).
    pub steered: bool,
    pub nil_income: f64,
    pub pending_dip: Option<f64>,
    pub last_role: Option<PlayingTime>,
    pub last_performance: Option<f64>,
    pub transfers: u32,
    pub juco: bool,
    /// Every college he played for, in order.
    pub path: Vec<usize>,
    /// A dismissal or suspension at his current school.
    pub on_notice: bool,
    /// Off-field incidents this season (for the locker room).
    pub incident: bool,
    pub notes: Vec<String>,
}

impl Person {
    pub fn note(&mut self, s: impl Into<String>) {
        self.notes.push(s.into());
    }
    /// What a scout with judging skill `accuracy` (1-20) learns about his
    /// background and family: the only view of them the game gets (1.7).
    pub fn character_report(&mut self, accuracy: f64) -> Vec<String> {
        background::character_report(&self.player, &self.bg, &self.family, accuracy, &mut self.rng)
    }
    pub fn level(&self, programs: &[Program]) -> Level {
        match (self.stage, self.college) {
            (Stage::College, Some(c)) => division_level(programs[c].division),
            _ => Level::HS,
        }
    }
}

pub fn division_level(d: Division) -> Level {
    match d {
        Division::FBS => Level::FBS,
        Division::FCS => Level::FCS,
        Division::D2 | Division::JUCO => Level::SmallCollege,
    }
}

/// A player who left the world: drafted, undrafted, or a blue-chip who left
/// football. Kept for history, pipelines and trials.
#[derive(Clone, Debug)]
pub struct Alumnus {
    pub name: String,
    pub home_state: usize,
    pub high_school: usize,
    pub colleges: Vec<usize>,
    pub stars: u8,
    pub year: i32,
    /// Draft pick (1-257), or None for an undrafted free agent.
    pub pick: Option<u32>,
    pub position: crate::positions::Position,
    pub overall: f64,
    pub(crate) potential: f64,
    pub juco: bool,
    pub left_football: bool,
    /// His high school class (year of his last high school spring).
    pub class_of: i32,
    pub(crate) visibility: f64,
    pub steered: bool,
    pub background_summary: String,
    pub notes: Vec<String>,
    /// The full player as he left, kept for drafted players and blue-chips (trial player pages).
    pub player: Option<Box<crate::player::Player>>,
}

impl Alumnus {
    pub(crate) fn from_person(p: &Person, year: i32, pick: Option<u32>, left_football: bool) -> Alumnus {
        let bg = &p.bg;
        let background_summary = format!(
            "{}; money {:.2}, safety {:.2}, home stability {:.2}, school resources {:.2}, GPA {:.1}, mental stability {:.0}, grit {:.0}, mentor {}",
            p.family.structure.phrase(),
            bg.money,
            bg.safety,
            bg.home_stability,
            bg.school_resources,
            bg.gpa,
            bg.stability,
            bg.grit,
            if bg.mentor { "yes" } else { "no" }
        );
        Alumnus {
            name: p.name.clone(),
            home_state: p.home_state,
            high_school: p.hs,
            colleges: p.path.clone(),
            stars: p.stars,
            year,
            pick,
            position: p.player.position,
            overall: p.player.overall(),
            potential: p.player.potential(),
            juco: p.juco,
            left_football,
            class_of: p.class_of,
            visibility: p.visibility,
            steered: p.notes.iter().any(|n| n.contains("more than his own")),
            background_summary,
            notes: p.notes.clone(),
            player: (pick.is_some() || p.stars >= 4).then(|| Box::new(p.player.clone())),
        }
    }
}

pub struct World {
    pub options: WorldOptions,
    /// The next season to be played.
    pub year: i32,
    pub states: Vec<StateInfo>,
    pub high_schools: Vec<HighSchool>,
    pub programs: Vec<Program>,
    pub coaches: Vec<Coach>,
    pub people: Vec<Person>,
    pub alumni: Vec<Alumnus>,
    pub news: Vec<NewsItem>,
    /// Star counts per signing class (index = stars), for calibration.
    pub signing_classes: Vec<(i32, [u32; 6])>,
    /// High school players signed by FBS programs per home state, over recorded history.
    pub fbs_signees_by_state: [u32; 51],
    pub(crate) rng: Rng,
    /// False during the spin-up before recorded history, while rosters fill.
    pub(crate) recording: bool,
    next_person: u32,
}

/// Years before recorded history, used to fill high school and college rosters.
const SPIN_UP_YEARS: u32 = 6;

impl World {
    /// Builds the world and its hidden history. The save then starts at
    /// `options.start_year`.
    pub fn new(options: WorldOptions) -> World {
        let mut w = World::empty(options);
        // Older high school classes already in school when the clock starts.
        for age in [17, 16, 15] {
            w.add_class_aged(age);
        }
        for _ in 0..SPIN_UP_YEARS {
            w.advance_year();
        }
        w.recording = true;
        for p in w.programs.iter_mut() {
            p.prestige_start = p.prestige;
        }
        for _ in 0..w.options.history_years {
            w.advance_year();
        }
        w
    }

    /// The world with programs, schools and staffs but no players or history yet.
    pub fn empty(options: WorldOptions) -> World {
        let mut rng = Rng::new(options.seed);
        let seed = options.seed;
        let mut programs: Vec<Program> = Vec::new();
        for (i, row) in data::COLLEGES.iter().enumerate() {
            programs.push(program::program_from_row(i, row, seed, &mut rng));
        }
        for row in recruiting::juco_rows() {
            let id = programs.len();
            programs.push(program::program_from_row(id, &row, seed, &mut rng));
        }
        let mut high_schools = Vec::new();
        let mut states = Vec::new();
        let total_programs: f64 = data::STATES.iter().map(|s| s.programs as f64).sum();
        for row in data::STATES.iter() {
            let idx = geo::state_index(&row.code).unwrap_or(0);
            let schools = high_school::generate_state_schools(idx, row, options.scale, high_schools.len(), &mut rng);
            let ids: Vec<usize> = schools.iter().map(|s| s.id).collect();
            high_schools.extend(schools);
            states.push((idx, row, ids));
        }
        let mut infos: Vec<StateInfo> = (0..51)
            .map(|i| StateInfo {
                code: geo::STATE_CENTERS[i].0.to_string(),
                transfer: TransferRule { sit_out: 0.5, free_once: false },
                hs_nil: false,
                culture: 0.0,
                class_share: 0.0,
                schools: Vec::new(),
            })
            .collect();
        let total_signees: f64 = data::STATES.iter().map(|s| s.fbs_signees_per_year).sum();
        for (idx, row, ids) in states {
            let info = &mut infos[idx];
            info.transfer = high_school::transfer_rule(&row.transfer_rule);
            info.hs_nil = recruiting::hs_nil_allowed(&row.code);
            info.class_share = row.programs as f64 / total_programs;
            info.culture = culture_shift(row.fbs_signees_per_year / total_signees, info.class_share);
            info.schools = ids;
        }
        let year = options.start_year - options.history_years as i32 - SPIN_UP_YEARS as i32;
        let mut w = World {
            options,
            year,
            states: infos,
            high_schools,
            programs,
            coaches: Vec::new(),
            people: Vec::new(),
            alumni: Vec::new(),
            news: Vec::new(),
            signing_classes: Vec::new(),
            fbs_signees_by_state: [0; 51],
            rng,
            recording: false,
            next_person: 1,
        };
        staff::staff_everyone(&mut w);
        for d in [Division::FBS, Division::FCS, Division::D2, Division::JUCO] {
            let top = w.programs.iter().filter(|p| p.division == d).map(|p| p.prestige).fold(0.01, f64::max);
            for p in w.programs.iter_mut().filter(|p| p.division == d) {
                p.prestige_rel = p.prestige / top;
            }
        }
        for p in w.programs.iter_mut() {
            program::rebuild_footprint(p);
        }
        w
    }

    /// One full year: a new class arrives, high school transfers, the seasons,
    /// titles, the draft, signing day, the portal, the coaching carousel, and
    /// program prestige, grades and NIL.
    pub fn advance_year(&mut self) {
        let debug = std::env::var("AFM_DEBUG").is_ok();
        let mut t = std::time::Instant::now();
        let mut lap = |name: &str| {
            if debug {
                eprintln!("  {name}: {:.2}s", t.elapsed().as_secs_f64());
            }
            t = std::time::Instant::now();
        };
        self.add_class();
        lap("new class");
        recruiting::high_school_transfers(self);
        self.rebuild_rosters();
        lap("hs transfers");
        let results = season::play_seasons(self);
        lap("seasons");
        staff::after_season(self, &results);
        staff::carousel(self);
        lap("carousel");
        recruiting::college_exits(self, &results);
        lap("draft");
        recruiting::portal(self, &results);
        lap("portal");
        signing_day_and_rosters(self);
        lap("signing day");
        staff::update_programs(self, &results);
        self.life_events();
        self.compact();
        lap("programs and life");
        self.year += 1;
    }
}

fn signing_day_and_rosters(w: &mut World) {
    recruiting::signing_day(w);
    w.rebuild_rosters();
}

impl World {
    pub fn program_name(&self, id: usize) -> &str {
        self.programs[id].name(self.options.names)
    }

    pub fn school_name(&self, id: usize) -> &str {
        let s = &self.high_schools[id];
        match self.options.names {
            NameMode::Real => &s.name,
            NameMode::Fictional => &s.fictional_name,
        }
    }

    fn add_class(&mut self) {
        self.add_class_aged(14);
    }

    fn add_class_aged(&mut self, age: u32) {
        let size = (250_000.0 * self.options.scale).round() as usize;
        let year = self.year;
        let mut kids: Vec<(usize, usize, u32)> = Vec::with_capacity(size);
        // Which state and school each kid grows up at, drawn on the world stream.
        let state_weights: Vec<(usize, f64)> = (0..51).map(|s| (s, self.states[s].class_share)).collect();
        for _ in 0..size {
            let s = self.rng.weighted(&state_weights);
            let schools = &self.states[s].schools;
            if schools.is_empty() {
                continue;
            }
            // Bigger schools field bigger rosters.
            let mut best = schools[self.rng.int(0, schools.len() as i32 - 1) as usize];
            let other = schools[self.rng.int(0, schools.len() as i32 - 1) as usize];
            if self.high_schools[other].enrollment > self.high_schools[best].enrollment && self.rng.chance(0.5) {
                best = other;
            }
            kids.push((s, best, self.rng.next_u32()));
        }
        let first_id = self.next_person;
        self.next_person += kids.len() as u32;
        let schools = &self.high_schools;
        let states = &self.states;
        let new: Vec<Person> = kids
            .par_iter()
            .enumerate()
            .map(|(k, &(s, hs, seed))| {
                let mut rng = Rng::new(seed);
                let culture = states[s].culture;
                let z = rng.normal(culture, 1.0);
                let mut player = generate_prospect(
                    &mut rng,
                    ProspectOptions {
                        id: Some(first_id + k as u32),
                        talent_z: Some(z),
                        age: Some(age),
                        ..Default::default()
                    },
                );
                let school = &schools[hs];
                let (mut bg, structure) = generate_background(
                    BackgroundSeed {
                        area_money: school.area_money,
                        area_safety: school.area_safety,
                        school_resources: school.resources,
                    },
                    &mut rng,
                );
                let family = generate_family(&bg, Some(structure), &mut rng);
                apply_background(&mut player, &mut bg, &family, &mut rng);
                Person {
                    name: names::person_name(&mut rng),
                    player,
                    rng,
                    bg,
                    family,
                    home_state: s,
                    hs,
                    stage: Stage::HighSchool,
                    college: None,
                    class_of: year + 18 - age as i32,
                    stars: 0,
                    visibility: 0.0,
                    college_seasons: 0,
                    redshirt_used: false,
                    sit_out: 0.0,
                    steered: false,
                    nil_income: 0.0,
                    pending_dip: None,
                    last_role: None,
                    last_performance: None,
                    transfers: 0,
                    juco: false,
                    path: Vec::new(),
                    on_notice: false,
                    incident: false,
                    notes: Vec::new(),
                }
            })
            .collect();
        self.people.extend(new);
    }

    pub(crate) fn rebuild_rosters(&mut self) {
        for s in self.high_schools.iter_mut() {
            s.roster.clear();
        }
        for p in self.programs.iter_mut() {
            p.roster.clear();
        }
        for (i, p) in self.people.iter().enumerate() {
            match p.stage {
                Stage::HighSchool => self.high_schools[p.hs].roster.push(i),
                Stage::College => {
                    if let Some(c) = p.college {
                        self.programs[c].roster.push(i);
                    }
                }
                _ => {}
            }
        }
    }

    fn life_events(&mut self) {
        let programs = &self.programs;
        self.people
            .par_iter_mut()
            .filter(|p| matches!(p.stage, Stage::HighSchool | Stage::College | Stage::PrepSchool))
            .for_each(|p| {
                let age = p.player.age;
                let ev = background::roll_life_event(&mut p.bg, age, p.nil_income, &mut p.rng);
                if let Some(background::LifeEvent::FamilyMove) = ev {
                    // A family move changes his high school's area for good.
                    p.bg.money = clamp(p.bg.money + p.rng.normal(0.0, 0.1), 0.0, 1.0);
                }
                // A good coach relationship and team staff help him recover.
                let support = match p.college {
                    Some(c) if p.stage == Stage::College => programs[c].facilities,
                    _ => p.bg.school_resources * 0.6,
                };
                background::recover_stability(&mut p.bg, support);
            });
    }

    /// Drops everyone who left football or turned pro (pros are archived).
    fn compact(&mut self) {
        // Blue-chips who left football stay on record: busts are part of history.
        if self.recording {
            let year = self.year + 1;
            let gone: Vec<Alumnus> = self
                .people
                .iter()
                .filter(|p| p.stage == Stage::Done && p.stars >= 4)
                .map(|p| Alumnus::from_person(p, year, None, true))
                .collect();
            self.alumni.extend(gone);
        }
        self.people.retain(|p| matches!(p.stage, Stage::HighSchool | Stage::College | Stage::PrepSchool));
    }

    /// First-year dip for a step up (shared with the career model).
    pub(crate) fn adjustment(p: &mut Person, step: Step) -> f64 {
        first_year_adjustment(&p.player, step, &mut p.rng).dip
    }
}

/// The culture shift that makes a state's share of FBS signees match the data,
/// given its share of the class. FBS signees come from roughly the top 1.3% of
/// a class, so the shift moves that tail.
fn culture_shift(signee_share: f64, class_share: f64) -> f64 {
    if class_share <= 0.0 {
        return 0.0;
    }
    let ratio = (signee_share / class_share).max(0.02);
    // Tail mass above t=2.23 shifts by about exp(2.4 * shift) for small shifts.
    clamp(ratio.ln() / 2.6, -0.6, 0.5)
}
