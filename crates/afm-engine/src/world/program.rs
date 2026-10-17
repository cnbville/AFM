//! College programs: prestige (topic 7), EA-style program grades (1.6, topic 7),
//! NIL collectives (topic 8), recruiting footprints and pipelines (topic 4).

use std::collections::HashMap;

use super::data::{self, AcademicTier, CollegeRow, Division};
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

/// A school's booster base and NIL collective (topic 8). Budgets are modeled
/// estimates: no public source gives NIL dollars per school. The boosters
/// give in every era (facilities, the program's standing); their money
/// reaches players as NIL only from July 2021 (dynasty item 11.5).
#[derive(Clone, Debug)]
pub struct Collective {
    /// NIL money the program can pay players this offseason, in dollars:
    /// zero before July 2021 (`ncaa::nil_budget`).
    pub budget: f64,
    /// Steady money from the wider booster base.
    pub base: f64,
    /// One big booster's yearly money, if the school has one.
    pub big_booster: Option<f64>,
    pub trend: NilTrend,
    /// How many years in a row it has been drying up (drives warning strength).
    pub drying_years: u32,
}

impl Collective {
    /// What the booster base gives a year, within the ceiling of the biggest
    /// real collectives (dynasty item 11.7).
    pub fn boosters(&self) -> f64 {
        (self.base + self.big_booster.unwrap_or(0.0)).min(super::ncaa::COLLECTIVE_CEILING)
    }
}

/// The athletic director (trial 4 topic 4): a person who reads the coach
/// against the roster he was given. Judgement sharpens the read, patience
/// sets how long a bad stretch is tolerated, both 1-20.
#[derive(Clone, Debug)]
pub struct AthleticDirector {
    pub name: String,
    pub judgement: f64,
    pub patience: f64,
    pub since: i32,
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
    /// Team totals from the match engine; zero games when the stand-in played.
    pub totals: SeasonTotals,
}

/// A team's season totals from its engine games.
#[derive(Clone, Copy, Debug, Default)]
pub struct SeasonTotals {
    pub games: u32,
    pub points_for: i32,
    pub points_against: i32,
    pub plays: u32,
    pub yards: f64,
    pub pass_yards: f64,
    pub rush_yards: f64,
    pub yards_allowed: f64,
    pub att: u32,
    pub cmp: u32,
    pub carries: u32,
    pub turnovers: u32,
    pub takeaways: u32,
    pub sacks: u32,
    pub sacks_allowed: u32,
}

impl SeasonTotals {
    pub fn add(&mut self, own: &crate::game::stats::TeamStats, opp: &crate::game::stats::TeamStats) {
        self.games += 1;
        self.points_for += own.points;
        self.points_against += opp.points;
        self.plays += own.plays;
        self.yards += own.yards;
        self.pass_yards += own.pass_yards;
        self.rush_yards += own.rush_yards;
        self.yards_allowed += opp.yards;
        self.att += own.att;
        self.cmp += own.cmp;
        self.carries += own.carries;
        self.turnovers += own.turnovers;
        self.takeaways += opp.turnovers;
        // A team's own line counts the sacks it gave up.
        self.sacks += opp.sacks;
        self.sacks_allowed += own.sacks;
    }
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
    /// Why the program moved: facility projects, boosters, hires (note 15C).
    pub story: Vec<(i32, String)>,
    pub fans: f64,
    /// Overall prestige 0-1, rebuilt yearly from its parts.
    pub prestige: f64,
    /// Prestige when recorded history began.
    pub prestige_start: f64,
    /// Prestige against the best program in its division, 0-1: how big a name
    /// it is at its own level (NDSU in FCS, Alabama in FBS).
    pub prestige_rel: f64,
    /// Seasons with no team (not started yet, dropped or suspended), from
    /// the real history (`college_history.csv`).
    pub dark: Vec<(i32, i32)>,
    /// Its real past where it differs from today (1984-2025, college
    /// history): first and last season, division (None: no team) and conference.
    pub history: Vec<(i32, i32, Option<Division>, String)>,
    /// Its 2026 division and conference; None for a program that has since
    /// dropped football.
    pub base: Option<(Division, String)>,
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
    /// Pipelines to single high schools and to recruiting areas (a Detroit
    /// high school, the Tampa area), 0-1, with the last impact year (note 4).
    pub hs_pipeline: HashMap<usize, (f64, i32)>,
    pub area_pipeline: HashMap<usize, (f64, i32)>,
    pub head_coach: Option<usize>,
    /// The athletic director, who judges and fires the head coach (trial 4 topic 4).
    pub ad: Option<AthleticDirector>,
    /// Who coaches the team's remaining games when that is not the head
    /// coach: an interim after a firing or departure, or a retiring coach
    /// who stays through his team's last game (part B 38, 45).
    pub sideline: Option<usize>,
    /// The season the head coach last changed (fired, retired or left).
    pub hc_changed: i32,
    /// Recruiting class scores by signing year (0-100 points, top class 100).
    pub class_scores: Vec<(i32, f64)>,
    pub oc: Option<usize>,
    pub dc: Option<usize>,
    /// Position coaches by group (note 5).
    pub position_coaches: [Option<usize>; super::coach::GROUP_COUNT],
    pub roster: Vec<usize>,
    pub seasons: Vec<ProgramSeason>,
    pub titles: u32,
    pub conference_titles: u32,
    /// Draft picks by year (round of each pick).
    pub draft_picks: Vec<(i32, u32)>,
    /// The school's own admissions bar, a core GPA, on top of the NCAA floor (note 6).
    pub admissions_bar: f64,
    /// Exceptions a strict school can make per class for a player it believes
    /// will grow out of his grades (note 6).
    pub special_admits: u32,
    /// Era rules and the NCAA: a rogue booster, penalties, the yearly record
    /// (dynasty items 11.4-11.6).
    pub era: super::ncaa::ProgramEra,
    /// School single-season records (A40): passing, rushing and receiving
    /// yards and sacks, with the holder's name and year, from the games played
    /// on the engine.
    pub season_records: [Option<SeasonRecord>; 4],
}

/// A school single-season record (A40).
#[derive(Clone, Debug)]
pub struct SeasonRecord {
    pub value: f64,
    pub holder: String,
    pub year: i32,
}

/// The stats a school keeps single-season records for (A40), and the least a
/// mark must be to count as a record worth chasing (proposed, not confirmed).
pub const RECORD_STATS: [(&str, f64); 4] =
    [("passing yards", 2500.0), ("rushing yards", 1000.0), ("receiving yards", 900.0), ("sacks", 8.0)];

/// Admissions bars of academically strict programs (proposed, not confirmed:
/// schools don't publish football admission floors; these follow their
/// reputations). Every other school sits at the NCAA floor, or a little above
/// it for AAU research universities.
const STRICT_ADMISSIONS: [(&str, f64); 26] = [
    ("Stanford", 3.6),
    ("Northwestern", 3.5),
    ("Duke", 3.4),
    ("Vanderbilt", 3.4),
    ("Rice", 3.4),
    ("Notre Dame", 3.4),
    ("California", 3.2),
    ("UCLA", 3.2),
    ("Tulane", 3.2),
    ("Virginia", 3.1),
    ("Wake Forest", 3.1),
    ("Michigan", 3.0),
    ("Georgia Tech", 3.0),
    ("Boston College", 3.0),
    ("USC", 3.0),
    ("North Carolina", 2.9),
    ("SMU", 2.9),
    ("Air Force", 3.0),
    ("Army", 3.0),
    ("Navy", 3.0),
    ("William & Mary", 3.2),
    ("Richmond", 3.0),
    ("Colgate", 3.2),
    ("Lehigh", 3.2),
    ("Bucknell", 3.3),
    ("Georgetown", 3.3),
];

fn admissions(row: &CollegeRow) -> (f64, u32) {
    if row.conference == "Ivy" {
        return (3.5, 2);
    }
    if let Some(&(_, bar)) = STRICT_ADMISSIONS.iter().find(|(n, _)| *n == row.name) {
        return (bar, 3);
    }
    (0.0, 0)
}

impl Program {
    /// Whether the program fields a team in `year`: a program that had not
    /// started football yet, or had dropped or suspended it, does not.
    pub fn plays(&self, year: i32) -> bool {
        !self.dark.iter().any(|&(a, b)| (a..=b).contains(&year))
    }

    /// Its real division and conference in a season (college history):
    /// None when it fielded no team. Seasons before the history starts read
    /// as its first year; from 2026 on its own row stands, and the world's
    /// own changes take over after that.
    pub fn real_status(&self, season: i32) -> Option<(Division, String)> {
        let s = season.max(data::HISTORY_FIRST);
        match self.history.iter().find(|h| (h.0..=h.1).contains(&s)) {
            Some(h) => h.2.map(|d| (d, h.3.clone())),
            None => self.base.clone(),
        }
    }

    /// Fills its real past from the college history and the seasons it
    /// fielded no team.
    pub fn set_history(&mut self, dropped: bool) {
        self.history = data::COLLEGE_HISTORY
            .iter()
            .filter(|h| h.name == self.real_name)
            .map(|h| (h.first, h.last, h.division, h.conference.clone()))
            .collect();
        if dropped {
            self.base = None;
        }
        self.dark.clear();
        let mut open: Option<i32> = None;
        for y in data::HISTORY_FIRST..=data::HISTORY_LAST + 1 {
            let dark = self.real_status(y).is_none();
            match (dark, open) {
                (true, None) => open = Some(if y == data::HISTORY_FIRST { i32::MIN } else { y }),
                (false, Some(a)) => {
                    self.dark.push((a, y - 1));
                    open = None;
                }
                _ => {}
            }
        }
        if let Some(a) = open {
            self.dark.push((a, i32::MAX));
        }
    }

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
        Division::NAIA => 0.2,
        Division::D3 => 0.15,
        Division::JUCO => 0.15,
    }
}

/// Builds a program from its real row. Prestige parts are seeded from real
/// history: titles, 2021-25 record, stadium size, conference and academics.
pub fn program_from_row(id: usize, row: &CollegeRow, seed: u32, rng: &mut Rng) -> Program {
    let state = geo::state_index(&row.state).unwrap_or(0);
    // A shared or oversized stadium (Winston-Salem State plays in a 22,000-seat
    // city stadium) says little about a small program's crowds: cap by level.
    let stadium = row
        .stadium_capacity
        .unwrap_or(match row.division {
            Division::FBS => 30000,
            Division::FCS => 12000,
            _ => 4000,
        })
        .min(match row.division {
            Division::FBS => 110_000,
            Division::FCS => 25_000,
            _ => 10_000,
        });
    let scale = division_scale(row.division);
    let p4 = matches!(row.conference.as_str(), "SEC" | "Big Ten" | "Big 12" | "ACC") || row.name == "Notre Dame";
    let titles = row.national_titles as f64;
    let tradition =
        clamp(scale * (0.25 + 0.12 * titles.sqrt() + if p4 { 0.25 } else { 0.0 }) + rng.normal(0.0, 0.05), 0.02, 1.0);
    let win = row.last_5yr_win_pct.unwrap_or(0.45);
    // The same scale the yearly update uses, so nobody drifts just from the start.
    let recent = clamp(scale * (0.2 + 0.8 * win), 0.02, 1.0);
    let stadium_share = clamp((stadium as f64 / 110_000.0).sqrt(), 0.05, 1.0);
    let facilities =
        clamp(scale * (0.3 + 0.5 * stadium_share + if p4 { 0.2 } else { 0.0 }) + rng.normal(0.0, 0.05), 0.02, 1.0);
    let fans = clamp(0.2 * scale + 0.8 * stadium_share * scale.sqrt(), 0.02, 1.0);
    // Modeled NIL: P4 budgets run to tens of millions, Group of 5 to a few
    // million, FCS and D2 far less. A minority of schools have one big booster.
    let base = nil_target(row.division, p4, fans, recent) * rng.log_normal(0.25);
    let big_booster = if rng.chance(if p4 { 0.3 } else { 0.08 }) { Some(base * rng.range(0.2, 0.8)) } else { None };
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
        story: Vec::new(),
        fans,
        prestige: 0.0,
        prestige_start: 0.0,
        prestige_rel: 0.5,
        dark: Vec::new(),
        history: Vec::new(),
        base: Some((row.division, row.conference.clone())),
        season_records: [None, None, None, None],
        grades: [70.0; GRADE_COUNT],
        last_grades: [70.0; GRADE_COUNT],
        grade_history: Vec::new(),
        prestige_history: Vec::new(),
        collective: Collective {
            // Set each year from the era (`ncaa::nil_budget`); the world starts before NIL.
            budget: 0.0,
            base,
            big_booster,
            trend: NilTrend::Steady,
            drying_years: 0,
        },
        footprint: [0.0; 51],
        pipeline: [0.0; 51],
        pipeline_last_impact: [i32::MIN / 2; 51],
        hs_pipeline: HashMap::new(),
        area_pipeline: HashMap::new(),
        head_coach: None,
        ad: None,
        sideline: None,
        hc_changed: i32::MIN,
        class_scores: Vec::new(),
        oc: None,
        dc: None,
        position_coaches: [None; super::coach::GROUP_COUNT],
        roster: Vec::new(),
        seasons: Vec::new(),
        titles: row.national_titles,
        conference_titles: 0,
        draft_picks: Vec::new(),
        admissions_bar: admissions(row).0,
        special_admits: admissions(row).1,
        era: Default::default(),
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
    // Booster money counts in every era: before NIL it built facilities and
    // paid the staff; revenue sharing, about equal across a level, doesn't.
    let nil_share = clamp((p.collective.boosters() / 25.0e6).sqrt(), 0.0, 1.0);
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

/// Rebuilds the footprint: the home region plus wider reach for bigger
/// brands, plus pipelines, plus overflow regions: Group of 5 and FCS staffs
/// recruit talent-rich states where the big schools can't take everyone
/// (the MAC in Florida and Georgia, note 4).
pub fn rebuild_footprint(p: &mut Program, overflow: &[f64; 51]) {
    // A national brand recruits everywhere; a small school knows its own area.
    // Reach follows how big a name it is at its own level.
    let level_reach = match p.division {
        Division::FBS => 1.0,
        Division::FCS => 0.55,
        Division::D2 => 0.3,
        Division::NAIA => 0.3,
        Division::D3 => 0.25,
        Division::JUCO => 0.6,
    };
    let overflow_reach = match p.division {
        Division::FBS if !p.power_four() => 0.45,
        Division::FCS => 0.3,
        _ => 0.0,
    };
    let rel = p.prestige_rel;
    let reach_miles = 150.0 + 1100.0 * level_reach * rel * rel + 250.0 * p.fans;
    for s in 0..51 {
        let d = geo::miles(p.state, s);
        let local = (1.0 - d / reach_miles).max(0.0);
        let national = 0.35 * level_reach * rel.powi(3);
        let spill = overflow_reach * overflow[s] * (1.0 - d / 2200.0).max(0.0);
        p.footprint[s] = clamp(local.max(national).max(spill) + 0.6 * p.pipeline[s], 0.0, 1.0);
    }
}

/// A signee made an impact: all-conference or better, or drafted, or signed
/// by a pro team (note 4). Only impact grows a pipeline: to his state, his
/// high school and his area.
pub fn record_impact(p: &mut Program, state: usize, hs: usize, area: usize, weight: f64, year: i32) {
    p.pipeline[state] = clamp(p.pipeline[state] + 0.12 * weight * (1.0 - p.pipeline[state]), 0.0, 1.0);
    p.pipeline_last_impact[state] = year;
    for (map, key, k) in [(&mut p.hs_pipeline, hs, 0.25), (&mut p.area_pipeline, area, 0.12)] {
        let e = map.entry(key).or_insert((0.0, year));
        e.0 = clamp(e.0 + k * weight * (1.0 - e.0), 0.0, 1.0);
        e.1 = year;
    }
}

/// Pipelines fade slowly after several years with no impact.
pub fn fade_pipelines(p: &mut Program, year: i32) {
    for s in 0..51 {
        if year - p.pipeline_last_impact[s] > 3 {
            p.pipeline[s] = (p.pipeline[s] - 0.03).max(0.0);
        }
    }
    for map in [&mut p.hs_pipeline, &mut p.area_pipeline] {
        map.retain(|_, (v, last)| {
            if year - *last > 3 {
                *v -= 0.03;
            }
            *v > 0.005
        });
    }
}

/// One year of a collective's money. Losing seasons shrink donations; a big
/// booster can walk away. `next` is the offseason the new NIL budget pays
/// for. Returns the warning level (0 = none).
pub fn update_collective(p: &mut Program, win_pct: f64, next: i32, rng: &mut Rng) -> u32 {
    let before = p.collective.boosters();
    let mood = (win_pct - 0.5) * 0.18 + (p.prestige - 0.5) * 0.04;
    // Donations follow results, and drift back toward what the brand can raise.
    let target = nil_target(p.division, p.power_four(), p.fans, p.recent);
    let c = &mut p.collective;
    // New money gets harder to find as the boosters near what the biggest
    // real collectives raise, and they never pass it (11.7). Losses aren't
    // slowed.
    let room = (1.0 - c.boosters() / super::ncaa::COLLECTIVE_CEILING).max(0.0);
    let grow = |g: f64| if g > 0.0 { g * room } else { g };
    c.base = (c.base * (1.0 + grow(mood + rng.normal(0.0, 0.04)))).max(5_000.0);
    c.base += 0.08 * (target - c.base);
    let mut booster_left = false;
    if let Some(b) = c.big_booster {
        let leave = 0.025 + if win_pct < 0.4 { 0.04 } else { 0.0 };
        if rng.chance(leave) {
            c.big_booster = None;
            booster_left = true;
        } else {
            c.big_booster = Some(b * (1.0 + grow(rng.normal(0.02, 0.08))));
        }
    } else if rng.chance(0.015 + 0.02 * p.prestige) {
        // A new big booster arrives: one rich booster can change a program.
        c.big_booster = Some(c.base * rng.range(0.2, 0.8));
    }
    let change = c.boosters() / before.max(1.0) - 1.0;
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
    let level = c.drying_years;
    p.collective.budget = super::ncaa::nil_budget(p, next);
    level
}

/// A school accepts a stronger conference's invitation (topic 11, change 7):
/// one of the top Group of Five programs joins the power conference with the
/// most members in its state (the smallest one on a tie). The move is
/// announced now and takes effect next season. Proposed, not confirmed.
pub(crate) fn realign_one(w: &mut super::World, rng: &mut Rng) {
    let p4 = |c: &str| matches!(c, "SEC" | "Big Ten" | "Big 12" | "ACC");
    let mut g5: Vec<(f64, usize)> = w
        .programs
        .iter()
        .filter(|p| p.division == Division::FBS && !p4(&p.conference) && !p.conference.contains("Independent"))
        .filter(|p| !w.realign.iter().any(|r| r.0 == p.id))
        .map(|p| (p.prestige + rng.normal(0.0, 0.05), p.id))
        .collect();
    g5.sort_by(|a, b| b.0.total_cmp(&a.0));
    let Some(&(_, pick)) = g5.iter().take(5).nth(rng.int(0, 4.min(g5.len() as i32 - 1)) as usize) else { return };
    let state = w.programs[pick].state;
    let mut best: Option<(i32, i32, String)> = None;
    for conf in ["SEC", "Big Ten", "Big 12", "ACC"] {
        let members: Vec<&Program> = w.programs.iter().filter(|p| p.conference == conf).collect();
        let here = members.iter().filter(|p| p.state == state).count() as i32;
        let key = (here, -(members.len() as i32), conf.to_string());
        if best.as_ref().is_none_or(|b| (key.0, key.1) > (b.0, b.1)) {
            best = Some(key);
        }
    }
    let Some((_, _, conf)) = best else { return };
    let season = w.year;
    let from = w.programs[pick].conference.clone();
    let name = w.programs[pick].real_name.clone();
    w.realign.push((pick, conf.clone()));
    let text = format!("{name} accepts an invitation to leave the {from} for the {conf}, from next season.");
    w.programs[pick].story.push((season, text.clone()));
    w.log.rule_changes.push(text.clone());
    w.changes.applied.push(crate::calendar::rules::AppliedChange {
        change: crate::calendar::Change::ConferenceSwitch,
        season,
        detail: format!("{name}: {from} to {conf}"),
    });
    let today = w.today;
    w.calendar.post(
        crate::calendar::Event::new(
            today,
            crate::calendar::Level::College,
            crate::calendar::Kind::RuleChange,
            crate::calendar::Loudness::Big,
            text,
        )
        .program(pick),
    );
}

/// A level's name in a season's own words: Division I-A and I-AA until the
/// 2006 renaming to FBS and FCS.
pub fn division_era_name(d: Division, season: i32) -> &'static str {
    match d {
        Division::FBS if season < 2006 => "Division I-A",
        Division::FCS if season < 2006 => "Division I-AA",
        Division::FBS => "FBS",
        Division::FCS => "FCS",
        Division::D2 => "Division II",
        Division::NAIA => "the NAIA",
        Division::D3 => "Division III",
        Division::JUCO => "junior college football",
    }
}

fn conference_phrase(c: &str) -> String {
    if c.contains("Independent") { "as an independent".to_string() } else { format!("in the {c}") }
}

/// The season's real moves, at the start of the football year (college
/// history, through 2026): a program moving between levels or conferences,
/// starting football or coming back to it, and the staff of a program that
/// dropped it let go. Each is news; after 2026 the world's own realignment
/// takes over.
pub(crate) fn apply_history(w: &mut super::World) {
    use crate::calendar::{Event, Kind, Level, Loudness};
    let season = w.year;
    if season > data::HISTORY_LAST + 1 {
        return;
    }
    let today = w.today;
    for i in 0..w.programs.len() {
        let p = &w.programs[i];
        let (was, now) = (p.plays(season - 1), p.plays(season));
        if was && !now {
            // Dropped or paused after last season: the staff goes.
            let staff: Vec<usize> =
                [p.head_coach, p.oc, p.dc].into_iter().chain(p.position_coaches.iter().copied()).flatten().collect();
            for c in staff {
                super::staff::leave(w, c, "his school dropped football");
            }
            continue;
        }
        let Some((d, c)) = p.real_status(season) else { continue };
        let name = p.real_name.clone();
        let (old_d, old_c) = (p.division, p.conference.clone());
        let text = if !was {
            let before = (data::HISTORY_FIRST..season).any(|y| p.plays(y) && y >= w.first_year());
            if before {
                format!("{name} brings football back, in {} {}.", division_era_name(d, season), conference_phrase(&c))
            } else {
                format!(
                    "{name} plays its first season of football, in {} {}.",
                    division_era_name(d, season),
                    conference_phrase(&c)
                )
            }
        } else if d != old_d {
            let dir = if (d as usize) < old_d as usize { "moves up" } else { "moves down" };
            format!(
                "{name} {dir} from {} to {} and plays {}.",
                division_era_name(old_d, season),
                division_era_name(d, season),
                conference_phrase(&c)
            )
        } else if c != old_c {
            if c.contains("Independent") {
                format!("{name} leaves the {old_c} to play as an independent.")
            } else if old_c.contains("Independent") {
                format!("{name} ends its years as an independent and joins the {c}.")
            } else {
                format!("{name} leaves the {old_c} for the {c}.")
            }
        } else {
            continue;
        };
        let p = &mut w.programs[i];
        if was && (d as usize) < old_d as usize {
            // A school moves up after building for it: facilities and crowds
            // take a step toward the new level (proposed, not confirmed).
            let step = (division_scale(d) / division_scale(old_d)).sqrt();
            p.facilities = clamp(p.facilities * step, 0.0, 1.0);
            p.fans = clamp(p.fans * step, 0.0, 1.0);
        }
        p.division = d;
        p.conference = c;
        p.story.push((season, text.clone()));
        let loud = match d.min(old_d) {
            Division::FBS => Loudness::Big,
            Division::FCS => Loudness::Medium,
            _ => Loudness::Small,
        };
        w.calendar.post(Event::new(today, Level::College, Kind::RuleChange, loud, text).program(i));
    }
}

/// The schools dropping or pausing football after this season announce it
/// when their regular season ends (college history); every player with
/// eligibility left is free to transfer in the winter.
pub(crate) fn announce_drops(w: &mut super::World) {
    use crate::calendar::{Event, Kind, Level, Loudness};
    let season = w.year;
    let today = w.today;
    for i in 0..w.programs.len() {
        let p = &w.programs[i];
        if !p.plays(season) || p.plays(season + 1) || p.roster.is_empty() && p.head_coach.is_none() {
            continue;
        }
        // Nobody knows yet whether it comes back (UAB did, two years later).
        let text = format!("{} drops football after this season. Its players are free to transfer.", p.real_name);
        let loud = if p.division == Division::FBS { Loudness::Big } else { Loudness::Medium };
        w.programs[i].story.push((season, text.clone()));
        w.calendar.post(Event::new(today, Level::College, Kind::RuleChange, loud, text).program(i));
    }
}
