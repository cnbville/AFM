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
pub mod broadcast;
pub mod charts;
pub(crate) mod chemistry;
pub(crate) mod clock;
pub mod coach;
pub(crate) mod college;
pub mod data;
pub mod deals;
pub mod desk;
pub mod eligibility;
pub mod ethnicity;
pub mod fame;
pub mod finance;
pub mod folk;
pub mod former;
pub mod franchise;
pub mod front_office;
pub mod game_log;
pub mod geo;
pub mod guide;
pub mod high_school;
pub mod hs_playoffs;
pub mod hs_year;
pub mod legacy;
pub mod live_seat;
pub mod looks;
pub mod names;
pub mod ncaa;
pub mod news;
pub(crate) mod nfl_year;
pub mod norms;
pub use nfl_year::NflTeamSeason;
pub use recruiting::DraftRun;
pub use season::{key_normal, key_uniform};

/// Last season's draft order once the season has ended (for the coming draft).
pub(crate) fn nfl_year_order(w: &World) -> Option<Vec<usize>> {
    nfl_year::draft_order(w)
}
pub mod people;
pub mod plans;
pub(crate) mod polls;
pub mod pro;
pub mod program;
pub mod records;
pub(crate) mod recruiting;
pub mod road_marks;
pub mod schedule;
pub mod scouting;
pub(crate) mod season;
pub mod season_page;
pub mod seat;
pub mod seat_names;
pub mod staff;
pub mod staff_read;
pub mod stories;
pub mod storylines;
pub mod systems;
pub mod tape;
pub mod trends;
pub mod voters;
pub mod wlog;

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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
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
    /// A fictional league as well (lane 10, the setup's fictional option):
    /// NFL nicknames and conference names on the screens are fictional too,
    /// over the fictional college and high school names `names` gives.
    #[serde(default)]
    pub fictional_league: bool,
    /// The first season whose college games run on the match engine (item 12).
    /// None, the game's setting, plays every college game on the engine,
    /// history included, so the history comes from the same football as the
    /// save (Raichel, 2026-10-14). A year is for quick checks only.
    pub engine_from: Option<i32>,
    /// How many of the history's last seasons play their high school games
    /// on the match engine (section 8, choice 2; Cain 2026-10-20: the last
    /// 12, "for now"). Every live season does too, whatever this is (final
    /// build: every high school game in a live save). Earlier history
    /// seasons, and every history season with this None, use the stand-in.
    /// A season before `engine_from` stays on the stand-in as well.
    pub hs_engine_years: Option<u32>,
    /// Trial tooling: keep every college game's box score totals and each
    /// player's season totals in `World::game_log` (history years only).
    pub keep_game_log: bool,
    /// Trial tooling: the old yearly growth (everything at the
    /// checkpoint) instead of growth landing week by week (section 6b,
    /// choice 1), to measure one against the other.
    #[serde(default)]
    pub yearly_growth: bool,
}

impl Default for WorldOptions {
    fn default() -> Self {
        WorldOptions {
            seed: 2026,
            scale: 1.0,
            history_years: 30,
            start_year: 2026,
            names: NameMode::Real,
            fictional_league: false,
            engine_from: None,
            hs_engine_years: Some(12),
            keep_game_log: false,
            yearly_growth: false,
        }
    }
}

/// Where a player is in his football life.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Stage {
    HighSchool,
    /// A year at a prep school to fix grades or get bigger (topic 2, 1.6).
    PrepSchool,
    College,
    /// In the NFL draft pool: an underclassman who declared, or a senior whose
    /// college career is over, off the college roster until the draft (T3.1).
    Declared,
    /// In pro football (drafted, undrafted or a veteran at the start of
    /// history): his career is in `Person::pro` (section 5).
    Pro,
    /// Left football.
    Done,
    /// Away from football for a while with his clock stopped: a church
    /// mission or a military term (A3, final build). `Person::roads.away`
    /// says which, when he comes back and to which program.
    Away,
}

/// One state's football setting.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
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
    /// His date of birth (A1): every age the game shows counts from it. A
    /// class runs from September to August birthdays; a boy born a year
    /// before his class was held back (A2, `calendar::born`).
    #[serde(default = "crate::calendar::born::unknown")]
    pub born: crate::calendar::Date,
    /// Social media followers, all platforms (`fame`): public, they grow
    /// with fame and Aura from the late 2000s.
    #[serde(default)]
    pub followers: u32,
    /// His yearly money from ads, brand NIL deals and endorsements, in
    /// dollars (`fame::brand_money`), last set at the year's book.
    #[serde(default)]
    pub brand: f32,
    /// The recruiting services' stars (their latest rating). They rate
    /// kickers, punters and long snappers 3 stars at most.
    pub stars: u8,
    /// The kicking and snapping camps' own stars for a kicker, punter or
    /// long snapper (fix list 1.5): what staffs recruit specialists on.
    pub camp_stars: u8,
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
    /// His high school seasons' stat lines, from the seasons played on the
    /// match engine (section 8, choice 2).
    pub hs_lines: Vec<hs_year::HsLine>,
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
    /// How much he wants to stay in his own state (fix list 2.2): family,
    /// local fame. Added to every school in it.
    pub stay_home: f64,
    /// The school he grew up cheering for (fix list 2.3), usually in his state.
    pub childhood_team: Option<usize>,
    pub cal: PersonCal,
    /// His pro career (section 5): None until he joins the league.
    pub pro: Option<Box<pro::ProCareer>>,
    /// Section 6 topic 3: his mood, -1 to 1 (0 is his own baseline), and
    /// the main reason behind it (`people::Why`).
    pub mood: f32,
    pub mood_why: u8,
    /// His trust in his head coach, 0-1, and which coach it is (a new head
    /// coach starts it fresh).
    pub trust: f32,
    pub trust_in: u32,
    /// His road through college (A3, S11): years away, years added to his
    /// clock, the COVID year, seasons lost to injury, a mission or a
    /// military term in progress.
    #[serde(default)]
    pub roads: eligibility::Roads,
    /// His face recipe (faces F3): one number his genes, looks and
    /// headshots are read from (`looks.rs`). 0 in a world written before
    /// faces, until `World::faces_after_load` sets it.
    #[serde(default)]
    pub looks: looks::Looks,
}

/// The draft staffs' read of a player in December (T3.1, T3.2): his consensus
/// grade, his board score, and the round it projects to (8 = undrafted).
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct DraftRead {
    pub grade: f64,
    pub score: f64,
    pub round: u32,
}

/// When a recruit makes up his mind (T5.2): a commitment wave before the
/// early period (0 after junior days in March, 1 late April, 2 mid-June, 3
/// late August, 4 late October), at the early signing period, or late
/// (February signing day).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum CommitTiming {
    Summer(u8),
    Early,
    Late,
}

/// A signed recruit before he enrolls: the program, a walk-on spot or not,
/// the day he signed, and whether he enrolls in January.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Signed {
    pub program: usize,
    pub walk_on: bool,
    pub date: crate::calendar::Date,
    pub early_enrollee: bool,
}

/// A portal entrant's dates (T3): the day he entered and the day his
/// decision is due.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct PortalEntry {
    pub entered: crate::calendar::Date,
    pub decide_by: crate::calendar::Date,
    #[serde(deserialize_with = "crate::save::leaked")]
    pub why: crate::save::Text,
    /// Held until his team's playoff run ended (T3).
    pub held: bool,
}

/// A person's calendar state (section 4). Per-season flags reset at his
/// season's start.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
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
    /// Failed the fall term's check: out of the postseason and spring
    /// practice, until the spring term's grades (A23).
    pub fall_ineligible: bool,
    /// Lean toward leaving through the portal, as a chance (T3), and what moved it.
    pub lean: f64,
    #[serde(deserialize_with = "crate::save::leaked")]
    pub lean_events: Vec<(crate::calendar::Date, &'static str, f64)>,
    /// A whisper about his leaving is out (T3).
    pub whispered: bool,
    pub draft_read: Option<DraftRead>,
    /// Where December's pro scouts project him, when it isn't his college
    /// position: he moves there when he leaves for the pros, not before.
    pub pro_position: Option<crate::positions::Position>,
    pub opted_out: bool,
    /// The day his opt-out is announced (part D 16): his own day before his bowl.
    pub opt_out_on: Option<crate::calendar::Date>,
    pub redshirt_trick: bool,
    /// A verbal commitment: program, day (T5.1), and whether it was to a
    /// walk-on place.
    pub commit: Option<(usize, crate::calendar::Date, bool)>,
    pub signed: Option<Signed>,
    pub commit_timing: Option<CommitTiming>,
    /// A spring practice notice is out (T4.1): a breakout may land by the spring game.
    pub spring_notice: bool,
    /// An early enrollee's college spring step, taken before his first
    /// college row: that row opens with it at fall camp (F2).
    pub spring_gain_ahead: Option<f64>,
    /// Invited to the NFL combine (A10) or a college all-star game (A9).
    pub combine_invite: bool,
    #[serde(deserialize_with = "crate::save::leaked")]
    pub all_star: Option<crate::save::Text>,
    /// The high school all-star game he played in (January of his senior
    /// year), kept apart from the college invite.
    #[serde(deserialize_with = "crate::save::leaked")]
    pub hs_all_star: Option<crate::save::Text>,
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
    /// The services' running read of his upside: each new look moves it
    /// halfway toward what they saw (part B 6).
    pub service_read: Option<f64>,
    /// Scholarship offers made before his class's first commitment wave,
    /// with their days (part B 34): from his sophomore year for the best.
    pub early_offers: Vec<(usize, crate::calendar::Date)>,
    /// The NFL advisory committee's grade (part B 37): the round, 8 for
    /// "return to school".
    pub advisory: Option<u8>,
    /// Graduated at this May's commencement: the graduate-transfer choice
    /// is open (part B 36).
    pub may_graduate: bool,
    /// A big NIL deal came this year (fix list 4.5): read by the season's
    /// development step, then cleared.
    pub payday: bool,
}

/// A recruiting action on a day the NCAA periods allow (T5.3, T5.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Visit {
    /// Paid by the school, on campus, at most five.
    Official,
    /// Paid by the family: a home game on a Saturday.
    Unofficial,
    /// A staff member watches him at his high school in an evaluation week.
    DropIn,
    /// The head coach in his family's living room, in a contact period
    /// (lane 10: your own home visits).
    Home,
}

impl PersonCal {
    /// A new season's start: last season's flags go.
    pub fn season_reset(&mut self) {
        self.academic_sit = false;
        self.sitting_transfer = false;
        self.trouble = false;
        self.opted_out = false;
        self.opt_out_on = None;
        self.redshirt_trick = false;
        self.whispered = false;
        self.lean = 0.0;
        self.lean_events.clear();
        self.hs_grade_out = 0;
        self.spring_notice = false;
        self.may_graduate = false;
    }
}

impl Person {
    pub fn note(&mut self, s: impl Into<String>) {
        // Exact room, for the note and the list (memory: notes are kept
        // for every person, and a formatted line carries spare room).
        let mut s: String = s.into();
        s.shrink_to_fit();
        self.notes.reserve_exact(1);
        self.notes.push(s);
    }
    /// His family's football ties (a brother's program, a bloodline): the
    /// public record a recruiting search reads.
    pub fn family(&self) -> &background::Family {
        &self.family
    }
    /// How well off his family is, 0 to 1: what pays for camps and travel.
    pub fn family_money(&self) -> f64 {
        self.bg.money
    }
    /// The age on his latest season row. A row covers the season and the
    /// offseason after it, so every note about either gives this age and a
    /// page reads one age per season (D5, F15). Steps toward a season not
    /// yet started (a camp move, a summer commitment or transfer) give his
    /// age now, the age that season's row will carry. So does an early or
    /// summer enrollee on campus before his first college row (F2): his
    /// spring, spring term, NIL and portal notes belong to that row.
    pub fn row_age(&self) -> u32 {
        match self.player.history.last() {
            Some(h) if self.stage != Stage::College || h.level.is_college() => h.age,
            _ => self.player.age,
        }
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

/// The key a person's birthday is drawn from: his world's seed and his id.
pub(crate) fn born_key(seed: u32, id: u32) -> u64 {
    ((seed as u64) << 32 | id as u64) ^ 0xB0E5_DA7E_0000_0000
}

impl Person {
    /// His age on a day, from his date of birth (A1). A person from an
    /// older world file without one yet uses the engine's age.
    pub fn age_on(&self, d: crate::calendar::Date) -> u32 {
        if self.born == crate::calendar::born::UNKNOWN {
            self.player.age
        } else {
            crate::calendar::born::age_on(self.born, d)
        }
    }

    /// Years older than his class (A2): 1 when he was held back or started
    /// school late, -1 when he is a year young, 0 on time. Reclassifying
    /// moves his class and so this too.
    pub fn years_off_class(&self) -> i32 {
        if self.born == crate::calendar::born::UNKNOWN {
            0
        } else {
            self.class_of - crate::calendar::born::class_of_birth(self.born)
        }
    }

    /// His age as his grade counts it: the engine age of an on-time
    /// classmate (14 a freshman, 17 a senior through the season). High
    /// school steps that go by grade read this, not his real age.
    pub fn school_age(&self) -> u32 {
        (self.player.age as i32 - self.years_off_class()).max(0) as u32
    }
}

pub fn division_level(d: Division) -> Level {
    match d {
        Division::FBS => Level::FBS,
        Division::FCS => Level::FCS,
        Division::D2 | Division::NAIA | Division::D3 | Division::JUCO => Level::SmallCollege,
    }
}

/// A player who left the world: drafted, undrafted, or a blue-chip who left
/// football. Kept for history, pipelines and trials.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Alumnus {
    pub name: String,
    pub ethnicity: ethnicity::Ethnicity,
    pub home_state: usize,
    pub high_school: usize,
    pub colleges: Vec<usize>,
    /// The recruiting services' stars.
    pub stars: u8,
    /// The specialist camps' stars (kickers, punters, long snappers).
    pub camp_stars: u8,
    pub year: i32,
    /// Draft pick (1-257), or None for an undrafted free agent.
    pub pick: Option<u32>,
    pub position: crate::positions::Position,
    pub overall: f64,
    #[cfg_attr(not(feature = "debug"), allow(dead_code))]
    pub(crate) potential: f64,
    pub juco: bool,
    pub left_football: bool,
    /// His high school class (year of his last high school spring).
    pub class_of: i32,
    /// His date of birth (A1).
    #[serde(default = "crate::calendar::born::unknown")]
    pub born: crate::calendar::Date,
    #[cfg_attr(not(feature = "debug"), allow(dead_code))]
    pub(crate) visibility: f64,
    pub steered: bool,
    pub background_summary: String,
    pub notes: Vec<String>,
    /// The NFL team that drafted him.
    #[serde(deserialize_with = "crate::save::leaked")]
    pub nfl_team: Option<crate::save::Text>,
    pub gpa_at_signing: Option<f64>,
    /// His college coaches each season: (year, program, head coach, position coach).
    pub college_staff: Vec<(i32, usize, Option<usize>, Option<usize>)>,
    /// The full player as he left, kept for drafted players and blue-chips (trial player pages).
    pub player: Option<Box<crate::player::Player>>,
    /// His pro career, written when he retires (section 5).
    pub pro: Option<Box<pro::ProCareer>>,
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
            camp_stars: p.camp_stars,
            year,
            pick,
            position: p.player.position,
            overall: p.player.overall(),
            potential: p.player.potential(),
            juco: p.juco,
            left_football,
            class_of: p.class_of,
            born: p.born,
            visibility: p.visibility,
            steered: p.notes.iter().any(|n| n.contains("more than his own")),
            background_summary,
            notes: p.notes.clone(),
            nfl_team: None,
            gpa_at_signing: p.gpa_at_signing,
            college_staff: p.college_staff.clone(),
            player: (pick.is_some() || p.stars >= 4).then(|| Box::new(p.player.clone())),
            pro: None,
        }
    }
}

/// One portal entrant (recorded history only).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PortalRecord {
    /// The offseason he entered (the season just played).
    pub year: i32,
    /// His role in that season.
    pub role: Option<PlayingTime>,
    /// Why he entered: playing time, coaching change, steered, move up, dismissed or money.
    #[serde(deserialize_with = "crate::save::leaked")]
    pub why: crate::save::Text,
    pub from: Division,
    /// Where he landed, or None if he found no school.
    pub to: Option<Division>,
    /// For the raw export (part B 19): who, the days he entered and
    /// decided, the programs, whether he was held through a playoff run,
    /// whether his first choice fell through and whether he went back.
    pub id: u32,
    pub entered: crate::calendar::Date,
    pub decided: crate::calendar::Date,
    pub from_program: usize,
    pub to_program: Option<usize>,
    pub held: bool,
    pub fell_through: bool,
    pub returned: bool,
}

/// Recruiting and portal records for trials (dynasty item 11 checks).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RecruitingLog {
    /// (signing class, program) for each high school 5-star (not a specialist) who signed with an FBS program.
    pub five_stars: Vec<(i32, usize)>,
    pub portal: Vec<PortalRecord>,
    /// A transfer's first season at his new school: (season, role before, role now, division).
    pub transfer_seasons: Vec<(i32, PlayingTime, PlayingTime, Division)>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct World {
    pub options: WorldOptions,
    /// The next season to be played.
    pub year: i32,
    pub states: Vec<StateInfo>,
    pub high_schools: Vec<HighSchool>,
    pub programs: Vec<Program>,
    pub coaches: Vec<Coach>,
    /// People, alumni and news are their own parts in a save (`save.rs`).
    #[serde(serialize_with = "crate::save::in_parts")]
    pub people: Vec<Person>,
    #[serde(serialize_with = "crate::save::in_parts")]
    pub alumni: Vec<Alumnus>,
    #[serde(serialize_with = "crate::save::in_parts")]
    pub news: Vec<NewsItem>,
    /// Star counts per signing class (index = stars), for calibration.
    pub signing_classes: Vec<(i32, [u32; 6])>,
    /// High school players signed by FBS programs per home state, over recorded history.
    #[serde(with = "serde_big_array::BigArray")]
    pub fbs_signees_by_state: [u32; 51],
    /// Recruiting areas (state, town), indexed by `HighSchool::area`.
    pub areas: Vec<(usize, String)>,
    /// How much talent each state has beyond what its own Power 4 schools
    /// sign, 0-1: where Group of 5 and FCS staffs go recruiting (note 4).
    #[serde(with = "serde_big_array::BigArray")]
    pub overflow: [f64; 51],
    /// The 32 NFL teams: front offices and depth charts, for draft needs (note 7).
    pub nfl: Vec<crate::league::Team>,
    /// Each franchise's books and depth chart of people (section 5).
    pub clubs: Vec<pro::Club>,
    /// The NFL's honors, record book and Hall of Fame (section 5).
    pub nfl_book: pro::honors::NflBook,
    /// Overall of the 64th and 138th best players taken in the last draft:
    /// what staffs read early NFL entrants against (fix list 1.2).
    pub(crate) draft_bars: Option<(f64, f64)>,
    pub(crate) rng: Rng,
    /// False during the spin-up before recorded history, while rosters fill.
    pub(crate) recording: bool,
    /// The last history season keeps every NFL and college team's last
    /// tapes, so a new career has film of its first opponents (tape.rs).
    #[serde(skip)]
    pub(crate) world_tapes: bool,
    /// Trial tooling, see `WorldOptions::keep_game_log`.
    /// Trial tooling only: not saved (`save.rs`).
    #[serde(skip)]
    pub game_log: game_log::GameLog,
    /// The world log (world reader lane R2): on disk, never saved in the
    /// world file. None unless a build or the game attaches one.
    #[serde(skip)]
    pub(crate) wlog: Option<Box<wlog::WorldLog>>,
    /// The career stats record, record books and Hall of Fame (section 3,
    /// topic 8): always kept.
    pub stats: records::StatBook,
    /// Each position's typical score in each coaching system (S3).
    pub(crate) system_norms: systems::SystemNorms,
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
    /// The NFL's market: agents, deals waiting, picks and trades (section 5).
    pub market: pro::market::Market,
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
    /// program, and whether it is a walk-on place.
    pub(crate) announcements: Vec<(crate::calendar::Date, usize, u32, usize, bool)>,
    /// The portal market of the open window (kept while entrants decide).
    pub(crate) portal_market: Option<recruiting::PortalMarket>,
    /// Super Bowl champions by season.
    pub nfl_champions: Vec<(i32, usize)>,
    /// Every NFL team's season: record, head coach and how far it went
    /// (kept from the build that added it; older files start empty).
    #[serde(default)]
    pub nfl_seasons: Vec<nfl_year::NflTeamSeason>,
    /// Conference moves announced, to take effect next season (program, conference).
    pub(crate) realign: Vec<(usize, String)>,
    /// College coaching searches under way, each deciding on its own day (part D 18).
    pub(crate) searches: Vec<staff::Search>,
    /// Coaches who took a job in this college carousel cycle: none moves twice.
    pub(crate) moved_cycle: std::collections::HashSet<usize>,
    /// The college win curve's spread fitted to last season's games: what
    /// a roster's expected wins are read with.
    pub(crate) college_spread: f64,
    /// The manager's seat (N58): Continue stops on Big moments about his
    /// team or his level, and on Huge ones anywhere. None in trials, where
    /// every Big moment stops it.
    pub seat: Option<Seat>,
    /// Each division's concept families by season (plays and tactics topic
    /// 9.9), and how new each family is to its defenses now (9.5).
    pub trends: Vec<trends::Trend>,
    pub(crate) novelty: [[f64; crate::game::playbook::FAMILY_COUNT]; data::DIVISION_COUNT],
    /// Your team's games kept whole for their game pages (section 8).
    #[serde(default)]
    pub seat_games: Vec<seat::SeatGame>,
    /// Your depth chart where you set one (section 8, topic 6).
    #[serde(default)]
    pub seat_depth: Vec<(crate::positions::Position, Vec<u32>)>,
    /// Your players' plan focus (section 6b topic 5): player id and
    /// attribute index.
    #[serde(default)]
    pub seat_focus: Vec<(u32, u8)>,
    /// Your career once it has started: you, your coach id, your notes.
    #[serde(default)]
    pub career: Option<seat::Career>,
    /// Section 6's people built in the final build (lane 6): the people
    /// around a team, your meetings and talks, ties, lives, courts, staff
    /// contracts and your network (`folk`).
    #[serde(default)]
    pub folk: folk::Folk,
    /// Your game being played live today (section 9): the day books it
    /// when it runs.
    #[serde(default)]
    pub live: Option<live_seat::LiveSeat>,
    /// Game day (section 9): your settings, the tapes of your coming
    /// opponents, what you have studied.
    #[serde(default)]
    pub gameday: live_seat::GameDay,
    /// The game boards' level numbers and your state's polls (update 2,
    /// lane 9).
    #[serde(default)]
    pub charts: charts::Charts,
    /// Every club's books, projects, report cards and lines (later extras topic 1).
    #[serde(default)]
    pub finance: finance::Finance,
    /// Scouting and the draft (section 7, final build lane 7): public
    /// boards and mocks, your department's looks, your draft board, the live
    /// draft, the pre-draft season, pro scouting, look-backs, followers.
    #[serde(default)]
    pub scouting: scouting::Scouting,
    /// Former players whose lives stay in football (coaching update C11,
    /// C13): coaches come from them, and your old teammates are among them.
    #[serde(default)]
    pub formers: former::Formers,
    /// The poll and award voters (section 6 topic 9), each with his
    /// latest ballot; older files build them on load.
    #[serde(default)]
    pub voters: voters::Voters,
    /// Your front office: who has each duty and each level's desk (final
    /// build, lane 10).
    #[serde(default)]
    pub front_office: front_office::FrontOffice,
    /// Your team's training (section 6b): plans, logs, notes, installs,
    /// the game plan, designs and packages.
    #[serde(default)]
    pub training: crate::training::seat::SeatTraining,
    /// The stories around the football (final build lane 8): traditions,
    /// media, nicknames, your decisions graded, your career's story.
    #[serde(default)]
    pub stories: stories::Stories,
    /// The other pro leagues as full leagues (final build X7).
    #[serde(default)]
    pub others: pro::other::Others,
    /// 1 once the world has faces (looks numbers and the three Asian
    /// groups); 0 in an older file until `faces_after_load` runs.
    #[serde(default)]
    pub faces_version: u8,
    /// Each high school age and college's level-scale anchors over the last
    /// five seasons (Cain, 2026-10-24). An older file starts empty and
    /// pools the seasons it plays from then on.
    #[serde(default)]
    pub level_scales: Vec<crate::level_rating::ScaleHistory>,
    /// The storyline engine (update 2, S1-S10): stories that last, with a
    /// cast, a question, heat and an ending.
    #[serde(default)]
    pub storylines: storylines::Storylines,
    /// The systems to chase (update 2, lane 2): ambitions, your tree, your
    /// scheme, your guys, the Hall of Fame race and each level's five.
    #[serde(default)]
    pub legacy: legacy::Legacy,
    /// The deeper game plan (update 2, lane 5): your identity, standing
    /// game rules, call words, the staff's drafted call sheet and the
    /// week's plan beyond the matchups.
    #[serde(default)]
    pub plans: plans::SeatPlans,
    /// The schedule page (update 2, lane 6): non-conference games under
    /// contract for the seasons ahead, last season's polls and NFL season,
    /// and your own team's past seasons kept small.
    #[serde(default)]
    pub(crate) deals: Vec<deals::Deal>,
    #[serde(default)]
    pub(crate) prev_polls: Vec<college::Poll>,
    #[serde(default)]
    pub(crate) nfl_prev: Option<nfl_year::NflSeason>,
    #[serde(default)]
    pub(crate) kept_schedules: Vec<season_page::KeptSeason>,
}

/// What the dated systems did in one football year, for trials.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CalendarLog {
    pub year: i32,
    /// Spring system changes by college staffs (S3), and the FBS programs
    /// running each system after spring (offenses, fronts, shells).
    pub system_switches: u32,
    pub fbs_systems: [u32; 11],
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
    /// High school games played on the match engine, and their team totals
    /// (each game counted once per team).
    pub hs_engine_games: u32,
    pub hs_totals: program::SeasonTotals,
    pub hs_forfeits: u32,
    pub hs_no_contests: u32,
    pub hs_grade_outs: u32,
    pub hs_homecomings: u32,
    pub hs_senior_nights: u32,
    pub hs_all_state: u32,
    pub heat_limited_days: u32,
    /// Recruiting counters, commitments through signings, belong to the
    /// season whose class signs: a February signing day that falls after
    /// the Super Bowl still counts in its season's log.
    pub commits: u32,
    pub commits_by_month: [u32; 12],
    pub decommits: u32,
    /// Walk-ons put on scholarship in fall camp (topic 5.4).
    pub walk_on_scholarships: u32,
    /// Students who made a team at an open tryout (topic 5.5).
    pub tryouts: u32,
    pub graduates: u32,
    pub graduate_transfers: u32,
    pub flips: u32,
    pub signed_early: u32,
    pub signed_late: u32,
    /// Scholarship signees by division (FBS, FCS, small college, JUCO): [before February, February].
    pub signed_by_division: [[u32; 2]; data::DIVISION_COUNT],
    /// Dated recruiting actions (T5.3, T5.4): official visits, game-day
    /// unofficial visits, staff drop-ins at high schools.
    pub official_visits: u32,
    pub unofficial_visits: u32,
    pub drop_ins: u32,
    pub early_enrollees: u32,
    pub declared: u32,
    pub withdrew: u32,
    /// Declarations and withdrawals by day (part B 20): the title game's
    /// players declare and withdraw late.
    pub declared_by_day: Vec<(crate::calendar::Date, u32)>,
    pub withdrew_by_day: Vec<(crate::calendar::Date, u32)>,
    pub portal_entries: u32,
    /// Portal entrants by the division they left (part B 10).
    pub portal_by_division: [u32; data::DIVISION_COUNT],
    pub portal_by_day: Vec<(crate::calendar::Date, u32)>,
    pub portal_landed: u32,
    pub portal_returned: u32,
    pub whispers: u32,
    pub sit_out_transfers: u32,
    pub spring_ineligible: u32,
    pub fall_ineligible: u32,
    pub combine_invites: u32,
    pub all_star_invites: u32,
    /// College all-star invitations turned down (part C 5).
    pub all_star_declined: u32,
    pub huge_by_team: Vec<(usize, u32)>,
    pub rule_changes: Vec<String>,
    pub nfl_games: u32,
    pub nfl_preseason_games: u32,
    /// Huge games per NFL team in the season (the stakes test).
    pub nfl_huge_by_team: Vec<u32>,
    /// The long roads (A3, final build): medical hardship waivers, sixth
    /// years, COVID-year players who came back and who didn't, missions and
    /// military terms begun, players back from either, and players who moved
    /// up two levels or from NAIA and Division III.
    #[serde(default)]
    pub medical_hardships: u32,
    #[serde(default)]
    pub sixth_years: u32,
    #[serde(default)]
    pub covid_returns: u32,
    #[serde(default)]
    pub covid_declined: u32,
    #[serde(default)]
    pub missions: u32,
    #[serde(default)]
    pub enlisted: u32,
    #[serde(default)]
    pub returned: u32,
    #[serde(default)]
    pub moved_up_far: u32,
}

/// Where the manager works (N58): his level, and his NFL team, college
/// program or high school, or his club in another league.
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Seat {
    pub level: crate::calendar::Level,
    pub nfl_team: Option<usize>,
    pub program: Option<usize>,
    /// A club in another pro league (`Level::Other`; later extras topic 5),
    /// by its franchise in `world::pro::other`.
    #[serde(default)]
    pub other: Option<usize>,
}

impl Seat {
    /// Whether a moment matters to him: his own team, or his level. An
    /// event's program is a college program, so it is his own only when he
    /// coaches in college (a high school seat keeps its school there).
    pub fn cares(&self, e: &crate::calendar::Event) -> bool {
        e.level == self.level
            || (self.level == crate::calendar::Level::College && e.program.is_some() && e.program == self.program)
            || (e.nfl_team.is_some() && e.nfl_team == self.nfl_team)
    }
}

/// Years before recorded history, used to fill high school and college rosters.
pub const SPIN_UP_YEARS: u32 = 12;

impl World {
    /// What it took last draft to go in rounds 1-2 and in rounds 3-4, as
    /// overall bars (a public fact: who went where and how they played).
    pub fn draft_bars(&self) -> Option<(f64, f64)> {
        self.draft_bars
    }

    /// The world's first season: the spin-up years before recorded history.
    pub fn first_year(&self) -> i32 {
        self.options.start_year - self.options.history_years as i32 - SPIN_UP_YEARS as i32
    }

    /// Builds the world and its hidden history. The save then starts at
    /// `options.start_year`.
    pub fn new(options: WorldOptions) -> World {
        World::new_watched(options, |_, _| {})
    }

    /// `new`, calling `each` after every season with the world and the
    /// season's seconds (tools time the build year by year).
    pub fn new_watched(options: WorldOptions, mut each: impl FnMut(&World, f64)) -> World {
        World::new_stoppable(options, |w, s| {
            each(w, s);
            true
        })
        .expect("never stopped")
    }

    /// `new_watched`, where `each` says whether to go on: false stops the
    /// build after that season and gives nothing (the game's Stop button).
    pub fn new_stoppable(options: WorldOptions, each: impl FnMut(&World, f64) -> bool) -> Option<World> {
        World::new_logged(options, None, each)
    }

    /// `new_stoppable`, writing the world log into `log` as the history is
    /// played (world reader lane R2). The log stays attached to the world.
    pub fn new_logged(
        options: WorldOptions,
        log: Option<wlog::WorldLog>,
        mut each: impl FnMut(&World, f64) -> bool,
    ) -> Option<World> {
        let mut w = World::empty(options);
        if let Some(log) = log {
            w.attach_log(log);
        }
        // Older high school classes already in school when the clock starts.
        for age in [17, 16, 15] {
            w.add_class_aged(age);
        }
        for _ in 0..SPIN_UP_YEARS {
            let t = std::time::Instant::now();
            w.advance_year();
            if !each(&w, t.elapsed().as_secs_f64()) {
                return None;
            }
        }
        w.start_recording();
        for k in 0..w.options.history_years {
            let t = std::time::Instant::now();
            w.world_tapes = k + 1 == w.options.history_years;
            w.advance_year();
            w.world_tapes = false;
            if !each(&w, t.elapsed().as_secs_f64()) {
                return None;
            }
        }
        Some(w)
    }

    /// The spin-up is over: recorded history starts.
    pub(crate) fn start_recording(&mut self) {
        self.recording = true;
        for p in self.programs.iter_mut() {
            p.prestige_start = p.prestige;
        }
        program::seed_pipelines(self);
    }

    /// The world with programs, schools and staffs but no players or history yet.
    pub fn empty(options: WorldOptions) -> World {
        let mut rng = Rng::new(options.seed);
        let seed = options.seed;
        let mut programs: Vec<Program> = Vec::new();
        // Every program starts as it really was in the world's first season
        // (college history), or as it was when it first fielded a team, so
        // Boise State starts in Division I-AA and grows. Programs that
        // dropped football since 1984 play their real years.
        let first = options.start_year - options.history_years as i32 - SPIN_UP_YEARS as i32;
        let rows = data::COLLEGES.iter().map(|r| (r, false)).chain(data::DROPPED_COLLEGES.iter().map(|r| (r, true)));
        for (row, dropped) in rows {
            let id = programs.len();
            let mut probe = program::program_from_row(id, row, seed, &mut Rng::new(0));
            probe.set_history(dropped);
            let status = (first.max(data::HISTORY_FIRST)..=data::HISTORY_LAST + 1).find_map(|y| probe.real_status(y));
            let mut start = row.clone();
            if let Some((d, c)) = status {
                start.division = d;
                start.conference = c;
            }
            let mut p = program::program_from_row(id, &start, seed, &mut rng);
            p.base = probe.base.clone();
            p.set_history(dropped);
            programs.push(p);
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
            // Schools in one county share a recruiting area (final build).
            let key = (s.state, if s.county.is_empty() { s.town.clone() } else { format!("{} County", s.county) });
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
            clubs: Vec::new(),
            nfl_book: pro::honors::NflBook::new(),
            draft_bars: None,
            rng,
            recording: false,
            world_tapes: false,
            game_log: game_log::GameLog::default(),
            wlog: None,
            stats: records::StatBook::default(),
            system_norms: [[0.0; 11]; crate::positions::POSITION_COUNT],
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
            voters: Default::default(),
            hs_season: None,
            nfl_season: None,
            market: Default::default(),
            coach_window: None,
            log: CalendarLog { year, ..CalendarLog::default() },
            log_history: Vec::new(),
            event_history: Vec::new(),
            portal_pending: Vec::new(),
            announcements: Vec::new(),
            portal_market: None,
            nfl_champions: Vec::new(),
            nfl_seasons: Vec::new(),
            realign: Vec::new(),
            searches: Vec::new(),
            moved_cycle: std::collections::HashSet::new(),
            college_spread: season::COLLEGE_SPREAD,
            seat: None,
            folk: Default::default(),
            trends: Vec::new(),
            novelty: [[0.0; crate::game::playbook::FAMILY_COUNT]; data::DIVISION_COUNT],
            seat_games: Vec::new(),
            seat_depth: Vec::new(),
            seat_focus: Vec::new(),
            training: Default::default(),
            career: None,
            live: None,
            gameday: Default::default(),
            charts: Default::default(),
            finance: finance::Finance::default(),
            scouting: Default::default(),
            formers: Default::default(),
            front_office: Default::default(),
            stories: stories::Stories::default(),
            others: Default::default(),
            faces_version: 1,
            level_scales: Vec::new(),
            storylines: Default::default(),
            legacy: Default::default(),
            plans: Default::default(),
            deals: Vec::new(),
            prev_polls: Vec::new(),
            nfl_prev: None,
            kept_schedules: Vec::new(),
        };
        staff::staff_everyone(&mut w);
        pro::fill_league(&mut w);
        systems::spring_install(&mut w);
        systems::set_spots(&mut w);
        for d in Division::ALL {
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
    /// event that needs a decision, or a big or huge moment (T8). Returns
    /// that day.
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
    /// Memory tooling: the people list's spare capacity, then drops one
    /// part of every person by name (`memory_check`).
    #[cfg(feature = "debug")]
    pub fn debug_people_capacity(&self) -> (usize, usize) {
        (self.people.len(), self.people.capacity())
    }

    #[cfg(feature = "debug")]
    pub fn debug_drop_people_part(&mut self, part: &str) {
        for p in self.people.iter_mut() {
            match part {
                "story" => p.bg.story = Vec::new(),
                "name" => p.name = String::new(),
                "position history" => p.player.position_history = Vec::new(),
                "college staff" => p.college_staff = Vec::new(),
                "path" => p.path = Vec::new(),
                "calendar lists" => {
                    p.cal.lean_events = Vec::new();
                    p.cal.offers = Vec::new();
                    p.cal.visits = Vec::new();
                    p.cal.early_offers = Vec::new();
                }
                _ => {}
            }
        }
    }

    /// Memory tooling (`memory_season`): drops one of the world's inner
    /// stores by name, so a tool can weigh it. Never called by the game.
    #[doc(hidden)]
    pub fn memory_drop_store(&mut self, name: &str) {
        match name {
            "college season" => self.college = None,
            "last college season" => self.college_prev = None,
            "high school season" => self.hs_season = None,
            "NFL season" => self.nfl_season = None,
            "last NFL season" => self.nfl_prev = None,
            "deals" => self.deals = Vec::new(),
            "kept schedules" => self.kept_schedules = Vec::new(),
            "portal market" => self.portal_market = None,
            "announcements" => self.announcements = Vec::new(),
            "searches" => self.searches = Vec::new(),
            _ => {}
        }
    }

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

    pub fn add_class_aged(&mut self, age: u32) {
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
        let seed_world = self.options.seed;
        let schools = &self.high_schools;
        let states = &self.states;
        let start = self.people.len();
        // Grow by exactly the class (section 8, memory): doubling a list of
        // 850,000 people reserved room for two million. The class is built
        // straight into that room, in order, not in a list of its own first
        // (final build: 0.45 GB less at the peak of the summer).
        self.people.reserve_exact(kids.len());
        self.people.par_extend(kids.par_iter().enumerate().map(|(k, &(s, hs, seed))| {
            new_kid(seed_world, &states[s], s, &schools[hs], hs, seed, first_id + k as u32, age, year, None)
        }));
        self.draw_home_ties(start);
        self.place_new_class(start);
    }

    /// A boy out of another sport who comes out for football at your
    /// school (SK22): made like any new boy, keyed, with the tools his sport
    /// built and the football skills of someone who never played. Ids rise
    /// along the list, so he goes on the end. Returns his place.
    #[allow(clippy::too_many_arguments)]
    pub fn add_athlete(
        &mut self,
        hs: usize,
        name: &str,
        ethnicity: ethnicity::Ethnicity,
        class_of: i32,
        key: u64,
        pos: crate::positions::Position,
        built: &[crate::attributes::Attr],
        z: f64,
    ) -> usize {
        use crate::attributes::{Attr, Category};
        let s = self.high_schools[hs].state;
        let id = self.next_person;
        self.next_person += 1;
        let age = (self.year + 18 - class_of).clamp(14, 18) as u32;
        let seed = (key ^ (key >> 32)) as u32 ^ 0x5A7E;
        let mut p = new_kid(
            self.options.seed,
            &self.states[s],
            s,
            &self.high_schools[hs],
            hs,
            seed,
            id,
            age,
            self.year,
            Some(z),
        );
        p.name = name.to_string();
        p.ethnicity = ethnicity;
        p.class_of = class_of;
        let a = &mut p.player.attributes;
        for x in Attr::ALL {
            let v = a.get(x);
            if built.contains(&x) {
                a.set(x, (v + 6.0).min(99.0));
            } else if !matches!(x.category(), Category::Physical | Category::Mental) {
                // Football skills he never learned.
                a.set(x, (v * 0.8).max(1.0));
            }
        }
        p.player.position = pos;
        let start = self.people.len();
        self.people.push(p);
        self.draw_home_ties(start);
        start
    }

    /// Each new boy's pull to stay in his state and the team he grew up
    /// cheering for (fix list 2.2, 2.3), on keyed draws. Proposed, not
    /// confirmed: a quarter feel no pull, half a little (0.02-0.08), a
    /// quarter a lot (0.10-0.25). Four in five grew up with a team in their
    /// own state, picked by fan base and tradition; the rest with a national
    /// brand (and every boy in a state without one).
    fn draw_home_ties(&mut self, start: usize) {
        let seed = self.options.seed as u64;
        let year = self.year;
        let mut by_state: Vec<Vec<(usize, f64)>> = vec![Vec::new(); 51];
        let mut national: Vec<(usize, f64)> = Vec::new();
        for p in &self.programs {
            if !matches!(p.division, Division::FBS | Division::FCS) || !p.plays(year) {
                continue;
            }
            let pull = p.fans * (0.3 + p.grades[program::GradeCat::Tradition as usize] / 100.0);
            by_state[p.state].push((p.id, pull));
            if p.division == Division::FBS {
                national.push((p.id, pull * p.prestige_rel * p.prestige_rel));
            }
        }
        let pick = |list: &[(usize, f64)], u: f64| -> Option<usize> {
            let total: f64 = list.iter().map(|x| x.1).sum();
            let mut left = u * total;
            for &(id, wgt) in list {
                if left < wgt {
                    return Some(id);
                }
                left -= wgt;
            }
            list.last().map(|x| x.0)
        };
        for person in &mut self.people[start..] {
            let id = person.player.id as u64;
            let u = season::key_uniform(seed ^ 0x57A1, id);
            // Sized against the real home-state shares (fix list 2.8: FBS
            // median 37%, p10 15%, p90 65%); a first pass at 0-0.25 gave a
            // median of 20% (proposed, not confirmed).
            person.stay_home = if u < 0.2 {
                0.0
            } else if u < 0.65 {
                0.04 + 0.10 * season::key_uniform(seed ^ 0x57A2, id)
            } else {
                0.15 + 0.25 * season::key_uniform(seed ^ 0x57A3, id)
            };
            let local = &by_state[person.home_state];
            let list = if local.is_empty() || season::key_uniform(seed ^ 0x57A4, id) < 0.2 { &national } else { local };
            person.childhood_team = pick(list, season::key_uniform(seed ^ 0x57A5, id));
        }
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

    /// Drops everyone who left football (a retired pro's career is already in his alumni record).
    fn compact(&mut self) {
        // The ones whose lives stay in football are kept (C11, C13).
        former::year_end(self);
        former::intake(self);
        // Blue-chips who left football stay on record: busts are part of history.
        if self.recording {
            let year = self.year + 1;
            let gone: Vec<Alumnus> = self
                .people
                .iter()
                .filter(|p| p.stage == Stage::Done && p.stars >= 4 && p.pro.is_none())
                .map(|p| Alumnus::from_person(p, year, None, true))
                .collect();
            self.alumni.extend(gone);
        }
        // Where each person who stays lands, for the lists that hold people
        // by place (trial 7 finding 7: the comp picks' free agents).
        let mut next = 0;
        let map: Vec<Option<usize>> = self
            .people
            .iter()
            .map(|p| {
                let stays = matches!(
                    p.stage,
                    Stage::HighSchool | Stage::College | Stage::PrepSchool | Stage::Declared | Stage::Pro | Stage::Away
                );
                stays.then(|| {
                    next += 1;
                    next - 1
                })
            })
            .collect();
        // The stories they are in keep their names, and so do your games.
        storylines::keep_names(self, &map);
        seat_names::keep(self, &map);
        let mut stays = map.iter();
        self.people.retain(|_| stays.next().is_some_and(|m| m.is_some()));
        // Hand back the room of everyone who left, keeping a third more for
        // next year's class and the year's pros, so the list never doubles
        // (section 8, memory: it held room for 2.0 million people).
        let room = self.people.len() + self.people.len() / 3;
        self.people.shrink_to(room);
        self.announcements.clear();
        pro::remap_people(self, &map);
        pro::rebuild(self);
    }

    /// First-year dip for a step up (shared with the career model).
    pub(crate) fn adjustment(p: &mut Person, step: Step) -> f64 {
        first_year_adjustment(&p.player, step, &mut p.rng).dip
    }
}

/// Talent each state produces beyond what its own Power 4 programs sign, 0-1.
/// Mutable references to the items at `idx` (ascending, no repeats), so
/// per-person work on part of the world can run on all cores.
pub(crate) fn pick_mut<T>(v: &mut [T], idx: impl IntoIterator<Item = usize>) -> Vec<&mut T> {
    let mut out = Vec::new();
    let mut rest = v;
    let mut base = 0;
    for i in idx {
        let (_, tail) = std::mem::take(&mut rest).split_at_mut(i - base);
        let (x, tail) = tail.split_first_mut().expect("ascending indices");
        out.push(x);
        rest = tail;
        base = i + 1;
    }
    out
}

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

/// One new boy at a high school: born, grown up, his family and his
/// talent (`z` set for a boy whose talent is already known, such as an
/// athlete out of another sport).
#[allow(clippy::too_many_arguments)]
fn new_kid(
    seed_world: u32,
    state: &StateInfo,
    s: usize,
    school: &HighSchool,
    hs: usize,
    seed: u32,
    id: u32,
    age: u32,
    year: i32,
    z: Option<f64>,
) -> Person {
    let mut rng = Rng::new(seed);
    let culture = state.culture;
    let z = z.unwrap_or_else(|| rng.normal(culture, 1.0));
    // His birthday, and whether he is a year older or younger
    // than his class (A1, A2): keyed, so no draw moves.
    let (born, years) = crate::calendar::born::schoolboy(born_key(seed_world, id), age, year);
    let age = (age as i32 + years).max(13) as u32;
    let mut player = generate_prospect(
        &mut rng,
        ProspectOptions { id: Some(id), talent_z: Some(z), age: Some(age), ..Default::default() },
    );
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
    people::sync_new(&mut player, &bg, &family);
    let ethnicity = ethnicity::player_ethnicity(s, z, &mut rng);
    // Grades drift through high school: effort pushes them up.
    let gpa_trend =
        rng.normal(0.0, 0.07) + 0.012 * (player.personality.work_ethic - 10.0) + 0.04 * (bg.home_stability - 0.5);
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
        class_of: year + 18 - age as i32 + years,
        born,
        followers: 0,
        brand: 0.0,
        stars: 0,
        camp_stars: 0,
        visibility: 0.0,
        college_seasons: 0,
        redshirt_used: false,
        sit_out: 0.0,
        steered: false,
        nil_income: 0.0,
        pending_dip: None,
        last_role: None,
        hs_lines: Vec::new(),
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
        stay_home: 0.0,
        childhood_team: None,
        cal: PersonCal::default(),
        pro: None,
        mood: 0.0,
        mood_why: 0,
        trust: 0.6,
        trust_in: u32::MAX,
        roads: eligibility::Roads::default(),
        looks: looks::Looks::new(seed_world, id),
    }
}
