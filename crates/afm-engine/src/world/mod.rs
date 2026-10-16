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
mod clock;
pub mod coach;
pub(crate) mod college;
pub mod data;
pub mod ethnicity;
pub mod game_log;
pub mod geo;
pub mod high_school;
mod hs_year;
pub mod names;
pub mod ncaa;
pub mod news;
mod nfl_year;
pub mod program;
mod recruiting;
pub mod schedule;
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
    /// The first season whose college games run on the match engine (item 12).
    /// None, the game's setting, plays every college game on the engine,
    /// history included, so the history comes from the same football as the
    /// save (Raichel, 2026-10-14). A year is for quick checks only.
    pub engine_from: Option<i32>,
    /// Trial tooling: keep every college game's box score totals and each
    /// player's season totals in `World::game_log` (history years only).
    pub keep_game_log: bool,
}

impl Default for WorldOptions {
    fn default() -> Self {
        WorldOptions {
            seed: 2026,
            scale: 1.0,
            history_years: 30,
            start_year: 2026,
            names: NameMode::Real,
            engine_from: None,
            keep_game_log: false,
        }
    }
}

/// Where a player is in his football life.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    HighSchool,
    /// A year at a prep school to fix grades or get bigger (topic 2, 1.6).
    PrepSchool,
    College,
    /// In the NFL draft pool: an underclassman who declared, or a senior whose
    /// college career is over, off the college roster until the draft (T3.1).
    Declared,
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
    pub ethnicity: ethnicity::Ethnicity,
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
    /// Offensive and defensive snaps in his last college season and in his
    /// whole college career (dynasty item 3: backups' garbage-time snaps
    /// count as game experience).
    pub last_snaps: u32,
    pub college_snaps: u32,
    pub last_performance: Option<f64>,
    pub transfers: u32,
    pub juco: bool,
    /// Every college he played for, in order.
    pub path: Vec<usize>,
    /// A dismissal or suspension at his current school.
    pub on_notice: bool,
    /// How his grades move each high school year (hidden), GPA points.
    pub(crate) gpa_trend: f64,
    /// Core GPA when he signed out of high school (note 6).
    pub gpa_at_signing: Option<f64>,
    /// Signed as an academic redshirt: practices but can't play his first year (note 6).
    pub academic_redshirt: bool,
    /// On the roster without a scholarship (fix list 1.6: specialist walk-ons).
    pub walk_on: bool,
    /// His role before he transferred, until his first season at the new school is logged.
    pub(crate) transfer_from: Option<PlayingTime>,
    /// His college coaches each season: (year, program, head coach, position coach).
    pub college_staff: Vec<(i32, usize, Option<usize>, Option<usize>)>,
    /// Off-field incidents this season (for the locker room).
    pub incident: bool,
    pub notes: Vec<String>,
    /// What the dated systems know about him this year (section 4).
    pub cal: PersonCal,
}

/// The draft staffs' read of a player in December (T3.1, T3.2): his consensus
/// grade, his board score, and the round it projects to (8 = undrafted).
#[derive(Clone, Copy, Debug)]
pub struct DraftRead {
    pub grade: f64,
    pub score: f64,
    pub round: u32,
}

/// When a recruit makes up his mind (T5.2): a commitment wave before the
/// early period (0 late April, 1 late June, 2 late August, 3 late October),
/// at the early signing period, or late (February signing day).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CommitTiming {
    Summer(u8),
    Early,
    Late,
}

/// A signed recruit before he enrolls: the program, a walk-on spot or not,
/// the day he signed, and whether he enrolls in January.
#[derive(Clone, Copy, Debug)]
pub struct Signed {
    pub program: usize,
    pub walk_on: bool,
    pub date: crate::calendar::Date,
    pub early_enrollee: bool,
}

/// A portal entrant's dates (T3): the day he entered and the day his
/// decision is due.
#[derive(Clone, Copy, Debug)]
pub struct PortalEntry {
    pub entered: crate::calendar::Date,
    pub decide_by: crate::calendar::Date,
    pub why: &'static str,
}

/// A person's calendar state (section 4). Per-season flags reset at his
/// season's start.
#[derive(Clone, Debug, Default)]
pub struct PersonCal {
    /// This season counts against his eligibility unless he plays four games
    /// or fewer (2018 rule); settled January 2.
    pub count_pending: bool,
    /// Academically ineligible this season.
    pub academic_sit: bool,
    /// Suspended or dismissed this season.
    pub trouble: bool,
    /// Failed the spring term's eligibility check: out this fall (A23).
    pub spring_ineligible: bool,
    /// Lean toward leaving through the portal, as a chance (T3), and what moved it.
    pub lean: f64,
    pub lean_events: Vec<(crate::calendar::Date, &'static str, f64)>,
    /// A whisper about his leaving is out (T3).
    pub whispered: bool,
    pub draft_read: Option<DraftRead>,
    pub opted_out: bool,
    pub redshirt_trick: bool,
    /// A verbal commitment: program and day (T5.1).
    pub commit: Option<(usize, crate::calendar::Date)>,
    pub signed: Option<Signed>,
    pub commit_timing: Option<CommitTiming>,
    /// A spring practice notice is out (T4.1): a breakout may land by the spring game.
    pub spring_notice: bool,
    /// Invited to the NFL combine (A10) or a college all-star game (A9).
    pub combine_invite: bool,
    pub all_star: Option<&'static str>,
    /// Best SAT-scale test score so far (A30).
    pub test_score: Option<u32>,
    /// Must sit next season under the transfer rules of the day.
    pub transfer_sit: bool,
    /// Sitting out this season as a transfer (practice only).
    pub sitting_transfer: bool,
    pub portal: Option<PortalEntry>,
    /// High school games missed this season for grades (A29).
    pub hs_grade_out: u32,
    /// Plans to enter the draft if the reads hold (drawn in December).
    pub declare_plan: Option<bool>,
    /// Graduated (a December graduate can transfer without sitting).
    pub graduated: bool,
    /// All-state selections in high school (A32): 1 = honorable mention or
    /// second team, 2 = first team.
    pub all_state: u8,
    /// Programs that offered him at the last recruiting stage, best first (T5).
    pub offers: Vec<usize>,
    /// Dated recruiting contact (T5.3, T5.4): program, day and kind.
    pub visits: Vec<(usize, crate::calendar::Date, Visit)>,
}

/// A recruiting action on a day the NCAA periods allow (T5.3, T5.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visit {
    /// Paid by the school, on campus, at most five.
    Official,
    /// Paid by the family: a home game on a Saturday.
    Unofficial,
    /// A staff member watches him at his high school in an evaluation week.
    DropIn,
}

impl PersonCal {
    /// A new season's start: last season's flags go.
    pub fn season_reset(&mut self) {
        self.academic_sit = false;
        self.sitting_transfer = false;
        self.trouble = false;
        self.opted_out = false;
        self.redshirt_trick = false;
        self.whispered = false;
        self.lean = 0.0;
        self.lean_events.clear();
        self.hs_grade_out = 0;
        self.spring_notice = false;
    }
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
    pub ethnicity: ethnicity::Ethnicity,
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
    /// The NFL team that drafted him.
    pub nfl_team: Option<&'static str>,
    pub gpa_at_signing: Option<f64>,
    /// His college coaches each season: (year, program, head coach, position coach).
    pub college_staff: Vec<(i32, usize, Option<usize>, Option<usize>)>,
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
            ethnicity: p.ethnicity,
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
            nfl_team: None,
            gpa_at_signing: p.gpa_at_signing,
            college_staff: p.college_staff.clone(),
            player: (pick.is_some() || p.stars >= 4).then(|| Box::new(p.player.clone())),
        }
    }
}

/// One portal entrant (recorded history only).
#[derive(Clone, Debug)]
pub struct PortalRecord {
    /// The offseason he entered (the season just played).
    pub year: i32,
    /// His role in that season.
    pub role: Option<PlayingTime>,
    /// Why he entered: playing time, coaching change, steered, move up, dismissed or money.
    pub why: &'static str,
    pub from: Division,
    /// Where he landed, or None if he found no school.
    pub to: Option<Division>,
}

/// Recruiting and portal records for trials (dynasty item 11 checks).
#[derive(Clone, Debug, Default)]
pub struct RecruitingLog {
    /// (signing class, program) for each high school 5-star (not a specialist) who signed with an FBS program.
    pub five_stars: Vec<(i32, usize)>,
    pub portal: Vec<PortalRecord>,
    /// A transfer's first season at his new school: (season, role before, role now, division).
    pub transfer_seasons: Vec<(i32, PlayingTime, PlayingTime, Division)>,
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
    /// Recruiting areas (state, town), indexed by `HighSchool::area`.
    pub areas: Vec<(usize, String)>,
    /// How much talent each state has beyond what its own Power 4 schools
    /// sign, 0-1: where Group of 5 and FCS staffs go recruiting (note 4).
    pub overflow: [f64; 51],
    /// The 32 NFL teams: front offices and depth charts, for draft needs (note 7).
    pub nfl: Vec<crate::league::Team>,
    /// Overall of the 64th and 138th best players taken in the last draft:
    /// what staffs read early NFL entrants against (fix list 1.2).
    pub(crate) draft_bars: Option<(f64, f64)>,
    pub(crate) rng: Rng,
    /// False during the spin-up before recorded history, while rosters fill.
    pub(crate) recording: bool,
    /// Trial tooling, see `WorldOptions::keep_game_log`.
    pub game_log: game_log::GameLog,
    /// Recruiting and portal records for trials (dynasty item 11 checks).
    pub recruiting_log: RecruitingLog,
    next_person: u32,
    /// Section 4: today on the clock, this football year's real dates (from
    /// the day after one Super Bowl to the next) and last year's (for days
    /// that fall after its Super Bowl, like a 1990s signing day).
    pub today: crate::calendar::Date,
    pub dates: crate::calendar::YearDates,
    pub(crate) prev_dates: Option<crate::calendar::YearDates>,
    /// Every system's events, with their loudness (T8).
    pub calendar: crate::calendar::Calendar,
    /// The 12 agreed off-field changes as they happen (T11).
    pub changes: crate::calendar::ChangeState,
    /// This football year's rule book.
    pub rules: crate::calendar::RuleBook,
    pub(crate) college: Option<college::CollegeSeason>,
    pub(crate) college_prev: Option<college::CollegeSeason>,
    pub(crate) hs_season: Option<hs_year::HsSeasonState>,
    pub(crate) nfl_season: Option<nfl_year::NflSeason>,
    /// A head-coach-change transfer window waiting to open (T3): opens, closes.
    pub(crate) coach_window: Option<(crate::calendar::Date, crate::calendar::Date)>,
    /// Counters for trials: this football year's, and every earlier one.
    pub log: CalendarLog,
    pub log_history: Vec<CalendarLog>,
    /// Trial tooling: each year's medium and louder events.
    pub event_history: Vec<(i32, Vec<crate::calendar::Event>)>,
    /// Portal entrants still deciding (person indices; resolved before the
    /// year-end book compacts the people list).
    pub(crate) portal_pending: Vec<usize>,
    /// Verbal commitments decided at a commitment wave and waiting for the
    /// recruit's own announcement day (T5.2): day, person index, player id,
    /// program.
    pub(crate) announcements: Vec<(crate::calendar::Date, usize, u32, usize)>,
    /// The portal market of the open window (kept while entrants decide).
    pub(crate) portal_market: Option<recruiting::PortalMarket>,
    /// Super Bowl champions by season.
    pub nfl_champions: Vec<(i32, usize)>,
    /// Conference moves announced, to take effect next season (program, conference).
    pub(crate) realign: Vec<(usize, String)>,
}

/// What the dated systems did in one football year, for trials.
#[derive(Clone, Debug, Default)]
pub struct CalendarLog {
    pub year: i32,
    pub games: u32,
    pub snow_games: u32,
    pub rain_games: u32,
    pub freezing_games: u32,
    pub lightning_delays: u32,
    pub weather_moves: u32,
    pub weather_cancels: u32,
    pub opt_outs: u32,
    pub bowl_practice_gain: f64,
    pub four_game_redshirts: u32,
    pub redshirt_tricks: u32,
    pub spring_notices: u32,
    pub spring_breakouts: u32,
    pub spring_fizzles: u32,
    pub hs_games: u32,
    pub hs_forfeits: u32,
    pub hs_no_contests: u32,
    pub hs_grade_outs: u32,
    pub hs_homecomings: u32,
    pub hs_senior_nights: u32,
    pub hs_all_state: u32,
    pub heat_limited_days: u32,
    pub commits: u32,
    pub commits_by_month: [u32; 12],
    pub decommits: u32,
    pub flips: u32,
    pub signed_early: u32,
    pub signed_late: u32,
    /// Signees by division (FBS, FCS, small college, JUCO): [before February, February].
    pub signed_by_division: [[u32; 2]; 4],
    /// Dated recruiting actions (T5.3, T5.4): official visits, game-day
    /// unofficial visits, staff drop-ins at high schools.
    pub official_visits: u32,
    pub unofficial_visits: u32,
    pub drop_ins: u32,
    pub early_enrollees: u32,
    pub declared: u32,
    pub withdrew: u32,
    pub portal_entries: u32,
    pub portal_by_day: Vec<(crate::calendar::Date, u32)>,
    pub portal_landed: u32,
    pub portal_returned: u32,
    pub whispers: u32,
    pub sit_out_transfers: u32,
    pub spring_ineligible: u32,
    pub fall_ineligible: u32,
    pub combine_invites: u32,
    pub all_star_invites: u32,
    pub huge_by_team: Vec<(usize, u32)>,
    pub rule_changes: Vec<String>,
    pub nfl_games: u32,
    /// Huge games per NFL team in the season (the stakes test).
    pub nfl_huge_by_team: Vec<u32>,
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
        let mut areas: Vec<(usize, String)> = Vec::new();
        for s in high_schools.iter_mut() {
            let key = (s.state, s.town.clone());
            s.area = match areas.iter().position(|a| *a == key) {
                Some(k) => k,
                None => {
                    areas.push(key);
                    areas.len() - 1
                }
            };
        }
        let overflow = overflow_by_state(&programs);
        let nfl = crate::league::make_league(&mut rng);
        let changes = crate::calendar::ChangeState::default();
        let dates = crate::calendar::YearDates::new(year);
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
            areas,
            overflow,
            nfl,
            draft_bars: None,
            rng,
            recording: false,
            game_log: game_log::GameLog::default(),
            recruiting_log: RecruitingLog::default(),
            next_person: 1,
            today: dates.start,
            rules: changes.rules(year),
            dates,
            prev_dates: None,
            calendar: crate::calendar::Calendar::default(),
            changes,
            college: None,
            college_prev: None,
            hs_season: None,
            nfl_season: None,
            coach_window: None,
            log: CalendarLog { year, ..CalendarLog::default() },
            log_history: Vec::new(),
            event_history: Vec::new(),
            portal_pending: Vec::new(),
            announcements: Vec::new(),
            portal_market: None,
            nfl_champions: Vec::new(),
            realign: Vec::new(),
        };
        staff::staff_everyone(&mut w);
        for d in [Division::FBS, Division::FCS, Division::D2, Division::JUCO] {
            let top = w.programs.iter().filter(|p| p.division == d).map(|p| p.prestige).fold(0.01, f64::max);
            for p in w.programs.iter_mut().filter(|p| p.division == d) {
                p.prestige_rel = p.prestige / top;
            }
        }
        for p in w.programs.iter_mut() {
            program::rebuild_footprint(p, &w.overflow);
        }
        w
    }

    /// One football year on the daily clock (section 4): from the day after
    /// one Super Bowl to the day after the next. Every system acts on its own
    /// real days (see `clock`).
    pub fn advance_year(&mut self) {
        clock::advance_year(self);
    }

    /// One day on the clock.
    pub fn advance_day(&mut self) {
        clock::advance_day(self);
    }

    /// Continue (T10): skips quiet days and stops on the first day with an
    /// event that needs a decision or is huge. Returns that day.
    pub fn continue_until_needed(&mut self, max_days: u32) -> crate::calendar::Date {
        clock::continue_until_needed(self, max_days)
    }

    /// Holiday mode (T10): staff run things until `until`; it always stops
    /// for a huge moment. Returns the day it stopped on.
    pub fn holiday_to(&mut self, until: crate::calendar::Date) -> crate::calendar::Date {
        clock::holiday_to(self, until)
    }

    /// The college season's ranking releases so far: (day, poll, top 25).
    pub fn polls(&self) -> Vec<(crate::calendar::Date, &'static str, Vec<usize>)> {
        self.college.iter().flat_map(|c| c.polls.iter().map(|p| (p.date, p.kind.name(), p.top.clone()))).collect()
    }

    /// A state's high school season this year: first game, playoffs, finals.
    pub fn hs_state_dates(&self, code: &str) -> (crate::calendar::Date, crate::calendar::Date, crate::calendar::Date) {
        hs_year::state_season_dates(code, self.year)
    }

    /// This NFL season's playoff field: (seed, team), AFC and NFC mixed.
    pub fn nfl_playoff_field(&self) -> Vec<(usize, usize)> {
        self.nfl_season.as_ref().map_or(Vec::new(), nfl_year::playoff_field)
    }
}

impl World {
    /// A program as the match engine sees it today (trial tooling only).
    #[cfg(feature = "debug")]
    pub fn debug_game_team(&self, prog: usize) -> Option<crate::game::GameTeam> {
        season::game_team(self, &self.programs[prog])
    }

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
                let ethnicity = ethnicity::player_ethnicity(s, z, &mut rng);
                // Grades drift through high school: effort pushes them up.
                let gpa_trend = rng.normal(0.0, 0.07)
                    + 0.012 * (player.personality.work_ethic - 10.0)
                    + 0.04 * (bg.home_stability - 0.5);
                Person {
                    name: names::person_name(ethnicity, year - age as i32, &mut rng),
                    ethnicity,
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
                    last_snaps: 0,
                    college_snaps: 0,
                    last_performance: None,
                    transfers: 0,
                    juco: false,
                    path: Vec::new(),
                    on_notice: false,
                    gpa_trend,
                    gpa_at_signing: None,
                    academic_redshirt: false,
                    walk_on: false,
                    transfer_from: None,
                    college_staff: Vec::new(),
                    incident: false,
                    notes: Vec::new(),
                    cal: PersonCal::default(),
                }
            })
            .collect();
        let start = self.people.len();
        self.people.extend(new);
        self.place_new_class(start);
    }

    /// Athlete first (topic 5.2): each high school's head coach places the
    /// boys who just arrived, by what he sees and what his roster needs.
    fn place_new_class(&mut self, start: usize) {
        let mut by_school: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
        for i in start..self.people.len() {
            by_school.entry(self.people[i].hs).or_default().push(i);
        }
        let mut groups: Vec<(usize, Vec<usize>)> = by_school.into_iter().collect();
        groups.sort_unstable_by_key(|g| g.0);
        for (hs, members) in groups {
            let eye = self.hs_eye(hs);
            let mut players: Vec<&mut Player> = Vec::with_capacity(members.len());
            // Members are in increasing order, so split the slice one person at a time.
            let mut rest: &mut [Person] = &mut self.people[start..];
            let mut offset = start;
            for &i in &members {
                let (_, tail) = rest.split_at_mut(i - offset);
                let (one, after) = tail.split_at_mut(1);
                players.push(&mut one[0].player);
                rest = after;
                offset = i + 1;
            }
            crate::athlete::place_class(&mut players, eye);
        }
    }

    /// How sharply a high school's head coach reads where a boy fits.
    pub(crate) fn hs_eye(&self, hs: usize) -> crate::athlete::Eye {
        let school = &self.high_schools[hs];
        let skill = school.coach.map_or(7.0, |c| self.coaches[c].ability().head_coach_value());
        crate::athlete::Eye::coach(skill).staff(0x4853_0000 ^ school.coach.unwrap_or(hs) as u32)
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
                if p.stage == Stage::HighSchool {
                    p.bg.gpa = clamp(p.bg.gpa + p.gpa_trend + p.rng.normal(0.0, 0.05), 0.0, 4.0);
                }
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
        self.people
            .retain(|p| matches!(p.stage, Stage::HighSchool | Stage::College | Stage::PrepSchool | Stage::Declared));
        self.announcements.clear();
    }

    /// First-year dip for a step up (shared with the career model).
    pub(crate) fn adjustment(p: &mut Person, step: Step) -> f64 {
        first_year_adjustment(&p.player, step, &mut p.rng).dip
    }
}

/// Talent each state produces beyond what its own Power 4 programs sign, 0-1.
fn overflow_by_state(programs: &[Program]) -> [f64; 51] {
    let mut out = [0.0; 51];
    for row in data::STATES.iter() {
        let Some(s) = geo::state_index(&row.code) else { continue };
        let p4 = programs.iter().filter(|p| p.state == s && p.power_four()).count() as f64;
        // A Power 4 program signs about half of its class from its own state.
        out[s] = (row.fbs_signees_per_year - 14.0 * p4).max(0.0);
    }
    let top = out.iter().copied().fold(1.0, f64::max);
    for v in out.iter_mut() {
        *v = (*v / top).sqrt();
    }
    out
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
