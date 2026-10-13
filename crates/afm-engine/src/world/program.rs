//! College programs: prestige (topic 7), EA-style program grades (1.6, topic 7),
//! NIL collectives (topic 8), recruiting footprints and pipelines (topic 4).

use super::data::{AcademicTier, CollegeRow, Division};
use super::geo;
use crate::rng::{Rng, clamp};

/// The program grade categories (1.6). NIL and brand exposure are graded
/// separately because one swings fast and the other moves steadily (topic 7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum GradeCat {
    Academics,
    ProPipeline,
    PlayingTime,
    Tradition,
    TitleChances,
    CoachPrestige,
    Facilities,
    Nil,
    BrandExposure,
    ConferenceStrength,
    StadiumAtmosphere,
    CampusLife,
}

pub const GRADE_COUNT: usize = 12;

impl GradeCat {
    pub const ALL: [GradeCat; GRADE_COUNT] = [
        GradeCat::Academics,
        GradeCat::ProPipeline,
        GradeCat::PlayingTime,
        GradeCat::Tradition,
        GradeCat::TitleChances,
        GradeCat::CoachPrestige,
        GradeCat::Facilities,
        GradeCat::Nil,
        GradeCat::BrandExposure,
        GradeCat::ConferenceStrength,
        GradeCat::StadiumAtmosphere,
        GradeCat::CampusLife,
    ];

    pub fn name(self) -> &'static str {
        [
            "academics",
            "pro pipeline",
            "playing time",
            "tradition",
            "title chances",
            "coach prestige",
            "facilities",
            "NIL",
            "brand exposure",
            "conference strength",
            "stadium atmosphere",
            "campus life",
        ][self as usize]
    }

    /// How a grade moves each year (topic 7, confirmed split).
    pub fn speed(self) -> GradeSpeed {
        match self {
            GradeCat::TitleChances
            | GradeCat::PlayingTime
            | GradeCat::CoachPrestige
            | GradeCat::Nil
            | GradeCat::ProPipeline => GradeSpeed::Fast,
            GradeCat::ConferenceStrength => GradeSpeed::Realignment,
            _ => GradeSpeed::Steady,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GradeSpeed {
    /// A little each year.
    Steady,
    /// Follows what just happened: two straight conference titles take title
    /// chances from C- to B+ or higher; a coach leaving drops coach prestige overnight.
    Fast,
    /// Only on realignment, or when several member schools rise or fall.
    Realignment,
}

/// A grade's score 0-100 as an EA-style letter.
pub fn letter(score: f64) -> &'static str {
    const STEPS: [(f64, &str); 12] = [
        (97.0, "A+"),
        (93.0, "A"),
        (90.0, "A-"),
        (87.0, "B+"),
        (83.0, "B"),
        (80.0, "B-"),
        (77.0, "C+"),
        (73.0, "C"),
        (70.0, "C-"),
        (67.0, "D+"),
        (63.0, "D"),
        (60.0, "D-"),
    ];
    STEPS.iter().find(|(min, _)| score >= *min).map_or("F", |s| s.1)
}

/// Moves a grade toward its target for the year.
pub fn move_grade(current: f64, target: f64, speed: GradeSpeed, realigned: bool) -> f64 {
    let share = match speed {
        GradeSpeed::Steady => 0.15,
        GradeSpeed::Fast => 0.8,
        GradeSpeed::Realignment => {
            if realigned {
                1.0
            } else {
                0.05
            }
        }
    };
    current + share * (target - current)
}

/// Why a collective's money is moving, for warnings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NilTrend {
    Growing,
    Steady,
    /// Donations are falling after losing seasons.
    Falling,
    /// A big booster walked away.
    BoosterLeft,
}

/// A school's NIL collective (topic 8). Budgets are modeled estimates: no
/// public source gives NIL dollars per school.
#[derive(Clone, Debug)]
pub struct Collective {
    /// Yearly budget in dollars.
    pub budget: f64,
    /// Steady money from the wider booster base.
    pub base: f64,
    /// One big booster's yearly money, if the school has one.
    pub big_booster: Option<f64>,
    pub trend: NilTrend,
    /// How many years in a row it has been drying up (drives warning strength).
    pub drying_years: u32,
}

/// Season record for history.
#[derive(Clone, Debug)]
pub struct ProgramSeason {
    pub year: i32,
    pub wins: u32,
    pub losses: u32,
    pub conference_title: bool,
    pub national_title: bool,
    pub coach: Option<usize>,
    pub strength: f64,
}

#[derive(Clone, Debug)]
pub struct Program {
    pub id: usize,
    pub real_name: String,
    pub fictional_name: String,
    pub division: Division,
    pub conference: String,
    pub city: String,
    pub state: usize,
    pub enrollment: u32,
    pub academic_tier: AcademicTier,
    pub stadium: u32,
    /// Prestige parts, each 0-1.
    pub tradition: f64,
    pub recent: f64,
    pub facilities: f64,
    pub fans: f64,
    /// Overall prestige 0-1, rebuilt yearly from its parts.
    pub prestige: f64,
    /// Prestige when recorded history began.
    pub prestige_start: f64,
    /// Prestige against the best program in its division, 0-1: how big a name
    /// it is at its own level (NDSU in FCS, Alabama in FBS).
    pub prestige_rel: f64,
    pub grades: [f64; GRADE_COUNT],
    /// Last year's grades, for the trend arrows.
    pub last_grades: [f64; GRADE_COUNT],
    /// Grades at the end of each recorded year.
    pub grade_history: Vec<(i32, [f64; GRADE_COUNT])>,
    /// Prestige at the end of each recorded year.
    pub prestige_history: Vec<(i32, f64)>,
    pub collective: Collective,
    /// How well the staff knows each state, 0-1 (topic 4).
    pub footprint: [f64; 51],
    /// Pipelines per state, 0-1: grown only by impact, fading slowly.
    pub pipeline: [f64; 51],
    /// Last year a signee from each state made an impact.
    pub pipeline_last_impact: [i32; 51],
    pub head_coach: Option<usize>,
    pub oc: Option<usize>,
    pub dc: Option<usize>,
    pub roster: Vec<usize>,
    pub seasons: Vec<ProgramSeason>,
    pub titles: u32,
    pub conference_titles: u32,
    /// Draft picks by year (round of each pick).
    pub draft_picks: Vec<(i32, u32)>,
    /// Scholarships a year.
    pub class_size: usize,
}

impl Program {
    pub fn name(&self, mode: super::names::NameMode) -> &str {
        match mode {
            super::names::NameMode::Real => &self.real_name,
            super::names::NameMode::Fictional => &self.fictional_name,
        }
    }

    /// Power 4 or not: decides TV money and NIL scale.
    pub fn power_four(&self) -> bool {
        matches!(self.conference.as_str(), "SEC" | "Big Ten" | "Big 12" | "ACC") || self.real_name == "Notre Dame"
    }
}

fn division_scale(d: Division) -> f64 {
    match d {
        Division::FBS => 1.0,
        Division::FCS => 0.45,
        Division::D2 => 0.25,
        Division::JUCO => 0.15,
    }
}

/// Builds a program from its real row. Prestige parts are seeded from real
/// history: titles, 2021-25 record, stadium size, conference and academics.
pub fn program_from_row(id: usize, row: &CollegeRow, seed: u32, rng: &mut Rng) -> Program {
    let state = geo::state_index(&row.state).unwrap_or(0);
    let stadium = row.stadium_capacity.unwrap_or(match row.division {
        Division::FBS => 30000,
        Division::FCS => 12000,
        _ => 4000,
    });
    let scale = division_scale(row.division);
    let p4 = matches!(row.conference.as_str(), "SEC" | "Big Ten" | "Big 12" | "ACC") || row.name == "Notre Dame";
    let titles = row.national_titles as f64;
    let tradition =
        clamp(scale * (0.25 + 0.12 * titles.sqrt() + if p4 { 0.25 } else { 0.0 }) + rng.normal(0.0, 0.05), 0.02, 1.0);
    let win = row.last_5yr_win_pct.unwrap_or(0.45);
    let recent = clamp(scale * (0.2 + 0.8 * win) * if p4 { 1.0 } else { 0.8 }, 0.02, 1.0);
    let stadium_share = clamp((stadium as f64 / 110_000.0).sqrt(), 0.05, 1.0);
    let facilities =
        clamp(scale * (0.3 + 0.5 * stadium_share + if p4 { 0.2 } else { 0.0 }) + rng.normal(0.0, 0.05), 0.02, 1.0);
    let fans = clamp(0.2 * scale + 0.8 * stadium_share * scale.sqrt(), 0.02, 1.0);
    // Modeled NIL: P4 budgets run to tens of millions, Group of 5 to a few
    // million, FCS and D2 far less. A minority of schools have one big booster.
    let base = nil_target(row.division, p4, fans, recent) * rng.log_normal(0.25);
    let big_booster = if rng.chance(if p4 { 0.3 } else { 0.08 }) { Some(base * rng.range(0.2, 0.8)) } else { None };
    let class_size = match row.division {
        Division::FBS => 28,
        Division::FCS => 24,
        Division::D2 => 30,
        Division::JUCO => 40,
    };
    let mut p = Program {
        id,
        real_name: row.name.clone(),
        fictional_name: super::names::fictional_program_name(&row.city, id, seed),
        division: row.division,
        conference: row.conference.clone(),
        city: row.city.clone(),
        state,
        enrollment: row.enrollment,
        academic_tier: row.academic_tier,
        stadium,
        tradition,
        recent,
        facilities,
        fans,
        prestige: 0.0,
        prestige_start: 0.0,
        prestige_rel: 0.5,
        grades: [70.0; GRADE_COUNT],
        last_grades: [70.0; GRADE_COUNT],
        grade_history: Vec::new(),
        prestige_history: Vec::new(),
        collective: Collective {
            budget: base + big_booster.unwrap_or(0.0),
            base,
            big_booster,
            trend: NilTrend::Steady,
            drying_years: 0,
        },
        footprint: [0.0; 51],
        pipeline: [0.0; 51],
        pipeline_last_impact: [i32::MIN / 2; 51],
        head_coach: None,
        oc: None,
        dc: None,
        roster: Vec::new(),
        seasons: Vec::new(),
        titles: row.national_titles,
        conference_titles: 0,
        draft_picks: Vec::new(),
        class_size,
    };
    p.prestige = prestige_of(&p, 10.0, 0.5);
    p
}

/// What a booster base can raise a year (modeled; no public per-school figures).
/// Power 4 budgets run to tens of millions, Group of 5 to a few million, FCS
/// and D2 far less.
pub fn nil_target(d: Division, p4: bool, fans: f64, recent: f64) -> f64 {
    match d {
        Division::FBS if p4 => 4.0e6 + 14.0e6 * fans * recent.max(0.3),
        Division::FBS => 0.6e6 + 2.5e6 * fans,
        Division::FCS => 0.08e6 + 0.5e6 * fans,
        _ => 0.02e6 + 0.1e6 * fans,
    }
}

/// Overall prestige from its parts (topic 7). Recent results count most.
pub fn prestige_of(p: &Program, coach_rep: f64, conference: f64) -> f64 {
    let nil_share = clamp((p.collective.budget / 25.0e6).sqrt(), 0.0, 1.0);
    clamp(
        0.15 * p.tradition
            + 0.3 * p.recent
            + 0.12 * clamp(coach_rep / 20.0, 0.0, 1.0) * division_scale(p.division).sqrt()
            + 0.12 * p.facilities
            + 0.11 * nil_share
            + 0.12 * p.fans
            + 0.08 * conference,
        0.0,
        1.0,
    )
}

/// Rebuilds the footprint: the home region plus wider reach for bigger brands, plus pipelines.
pub fn rebuild_footprint(p: &mut Program) {
    // A national brand recruits everywhere; a small school knows its own area.
    // Reach follows how big a name it is at its own level.
    let level_reach = match p.division {
        Division::FBS => 1.0,
        Division::FCS => 0.55,
        Division::D2 => 0.3,
        Division::JUCO => 0.6,
    };
    let rel = p.prestige_rel;
    let reach_miles = 150.0 + 1100.0 * level_reach * rel * rel + 250.0 * p.fans;
    for s in 0..51 {
        let d = geo::miles(p.state, s);
        let local = (1.0 - d / reach_miles).max(0.0);
        let national = 0.35 * level_reach * rel.powi(3);
        p.footprint[s] = clamp(local.max(national) + 0.6 * p.pipeline[s], 0.0, 1.0);
    }
}

/// A signee from `state` made an impact: led in stats, won an award or went in
/// the first three rounds. Only impact grows a pipeline (topic 4).
pub fn record_impact(p: &mut Program, state: usize, weight: f64, year: i32) {
    p.pipeline[state] = clamp(p.pipeline[state] + 0.12 * weight * (1.0 - p.pipeline[state]), 0.0, 1.0);
    p.pipeline_last_impact[state] = year;
}

/// Pipelines fade slowly after several years with no impact.
pub fn fade_pipelines(p: &mut Program, year: i32) {
    for s in 0..51 {
        if year - p.pipeline_last_impact[s] > 3 {
            p.pipeline[s] = (p.pipeline[s] - 0.03).max(0.0);
        }
    }
}

/// One year of a collective's money. Losing seasons shrink donations; a big
/// booster can walk away. Returns the warning level (0 = none).
pub fn update_collective(p: &mut Program, win_pct: f64, rng: &mut Rng) -> u32 {
    let before = p.collective.budget;
    let mood = (win_pct - 0.5) * 0.18 + (p.prestige - 0.5) * 0.04;
    // Donations follow results, and drift back toward what the brand can raise.
    let target = nil_target(p.division, p.power_four(), p.fans, p.recent);
    let c = &mut p.collective;
    c.base = (c.base * (1.0 + mood + rng.normal(0.0, 0.04))).max(5_000.0);
    c.base += 0.08 * (target - c.base);
    let mut booster_left = false;
    if let Some(b) = c.big_booster {
        let leave = 0.025 + if win_pct < 0.4 { 0.04 } else { 0.0 };
        if rng.chance(leave) {
            c.big_booster = None;
            booster_left = true;
        } else {
            c.big_booster = Some(b * (1.0 + rng.normal(0.02, 0.08)));
        }
    } else if rng.chance(0.015 + 0.02 * p.prestige) {
        // A new big booster arrives: one rich booster can change a program.
        c.big_booster = Some(c.base * rng.range(0.2, 0.8));
    }
    c.budget = c.base + c.big_booster.unwrap_or(0.0);
    let change = c.budget / before.max(1.0) - 1.0;
    c.trend = if booster_left {
        NilTrend::BoosterLeft
    } else if change < -0.12 {
        NilTrend::Falling
    } else if change > 0.08 {
        NilTrend::Growing
    } else {
        NilTrend::Steady
    };
    if matches!(c.trend, NilTrend::BoosterLeft | NilTrend::Falling) {
        c.drying_years += 1;
    } else {
        c.drying_years = 0;
    }
    c.drying_years
}
