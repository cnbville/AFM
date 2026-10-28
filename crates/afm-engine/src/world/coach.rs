//! Coaches (topic 3): a hidden ability split into parts, a public reputation
//! that lags behind it, a personality, a career arc and a coaching tree.

use std::collections::HashMap;

use super::ethnicity::Ethnicity;
use crate::positions::Position;
use crate::rng::{Rng, clamp};

/// Position groups a coach teaches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum Group {
    QB,
    RB,
    Receivers,
    OL,
    DL,
    LB,
    DB,
    Specialists,
}

pub const GROUP_COUNT: usize = 8;

impl Group {
    pub const ALL: [Group; GROUP_COUNT] =
        [Group::QB, Group::RB, Group::Receivers, Group::OL, Group::DL, Group::LB, Group::DB, Group::Specialists];

    pub fn of(pos: Position) -> Group {
        match pos {
            Position::QB => Group::QB,
            Position::RB => Group::RB,
            Position::WR | Position::TE => Group::Receivers,
            Position::OT | Position::IOL => Group::OL,
            Position::EDGE | Position::DT => Group::DL,
            Position::LB => Group::LB,
            Position::CB | Position::S => Group::DB,
            Position::K | Position::P | Position::LS => Group::Specialists,
        }
    }

    pub fn offense(self) -> bool {
        matches!(self, Group::QB | Group::RB | Group::Receivers | Group::OL)
    }

    /// The title of the position coach for this group.
    pub fn coach_title(self) -> &'static str {
        [
            "quarterbacks coach",
            "running backs coach",
            "wide receivers coach",
            "offensive line coach",
            "defensive line coach",
            "linebackers coach",
            "defensive backs coach",
            "special teams coordinator",
        ][self as usize]
    }

    pub fn name(self) -> &'static str {
        [
            "quarterbacks",
            "running backs",
            "receivers",
            "offensive line",
            "defensive line",
            "linebackers",
            "defensive backs",
            "specialists",
        ][self as usize]
    }
}

/// Hidden ability, every part 1-20 (topic 3).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Ability {
    pub teaching: [f64; GROUP_COUNT],
    pub scheme: f64,
    pub game_management: f64,
    pub motivation: f64,
    pub discipline: f64,
    pub recruiting: f64,
    /// Connections to other coaches and programs: exposure for his players (1.5).
    pub network: f64,
    pub relationships: f64,
}

impl Ability {
    /// Overall coaching strength as a head coach would show it on Saturdays.
    pub fn head_coach_value(&self) -> f64 {
        let teach: f64 = self.teaching.iter().sum::<f64>() / GROUP_COUNT as f64;
        0.3 * teach
            + 0.2 * self.scheme
            + 0.15 * self.game_management
            + 0.15 * self.motivation
            + 0.1 * self.discipline
            + 0.1 * self.relationships
    }

    fn parts_mut(&mut self) -> impl Iterator<Item = &mut f64> {
        self.teaching.iter_mut().chain([
            &mut self.scheme,
            &mut self.game_management,
            &mut self.motivation,
            &mut self.discipline,
            &mut self.recruiting,
            &mut self.network,
            &mut self.relationships,
        ])
    }
}

/// The four ratings only your own coach has as numbers (people topic 15):
/// play calling and in-game adaptation, evaluation and negotiating, 1-20.
/// An AI coach's come from his other ratings: calling from scheme and game
/// management, his eye from recruiting and teaching, and his club's GM does
/// the haggling.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Yours {
    pub play_calling: f64,
    pub adaptation: f64,
    pub evaluation: f64,
    pub negotiating: f64,
    /// You call your offense's or your defense's plays yourself as head
    /// coach (plays and tactics topic 6.1); otherwise the coordinator does.
    #[serde(default)]
    pub calls_offense: bool,
    #[serde(default)]
    pub calls_defense: bool,
}

impl Default for Yours {
    fn default() -> Yours {
        Yours {
            play_calling: 10.0,
            adaptation: 10.0,
            evaluation: 10.0,
            negotiating: 10.0,
            calls_offense: false,
            calls_defense: false,
        }
    }
}

/// Personality, 1-20 each.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CoachPersonality {
    /// Wants to move up.
    pub ambition: f64,
    /// Stays at one school.
    pub loyalty: f64,
    pub temper: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Role {
    HeadCoach,
    OffensiveCoordinator,
    DefensiveCoordinator,
    /// A college position coach (note 5): the first rung for many coaches.
    PositionCoach(Group),
}

impl Role {
    pub fn name(self) -> &'static str {
        match self {
            Role::HeadCoach => "head coach",
            Role::OffensiveCoordinator => "offensive coordinator",
            Role::DefensiveCoordinator => "defensive coordinator",
            Role::PositionCoach(g) => g.coach_title(),
        }
    }

    /// Offense, defense, or neither (special teams), for who credits a record.
    pub fn offense(self) -> Option<bool> {
        match self {
            Role::HeadCoach => None,
            Role::OffensiveCoordinator => Some(true),
            Role::DefensiveCoordinator => Some(false),
            Role::PositionCoach(Group::Specialists) => None,
            Role::PositionCoach(g) => Some(g.offense()),
        }
    }
}

/// Where a coach works.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Employer {
    HighSchool(usize),
    College(usize),
    Nfl(usize),
    /// A team in another pro league (the CFL, arena football, the spring
    /// leagues; final build X7), by its franchise in `world::pro::other`.
    Other(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Job {
    pub employer: Employer,
    pub role: Role,
    pub since: i32,
}

/// A junior post (C36): the first rung below the coaching staff, held by
/// real people with hidden ratings and a learning ceiling. A college
/// graduate assistant or analyst, an NFL quality-control coach.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Post {
    GradAssistant,
    Analyst,
    QualityControl,
}

impl Post {
    pub fn name(self) -> &'static str {
        match self {
            Post::GradAssistant => "graduate assistant",
            Post::Analyst => "analyst",
            Post::QualityControl => "quality-control coach",
        }
    }
}

/// A junior post a coach holds or held (C36): the room he works with, and
/// the season it ended (None while he holds it).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Junior {
    pub employer: Employer,
    pub post: Post,
    pub group: Group,
    pub since: i32,
    pub until: Option<i32>,
}

/// One finished job, for the coach's record.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Stint {
    pub employer: Employer,
    pub role: Role,
    pub from: i32,
    pub to: i32,
    pub wins: u32,
    pub losses: u32,
    pub titles: u32,
    /// Why it ended, in plain words.
    #[serde(deserialize_with = "crate::save::leaked")]
    pub ended: crate::save::Text,
}

/// What schools can see of his teaching at one position group (note 15B):
/// how his players grew against others at their level and age, and how many
/// moved up (college offers, honors, the draft). Old seasons fade.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UnitRecord {
    pub growth: f64,
    pub seasons: f64,
    pub moved_up: f64,
}

impl UnitRecord {
    /// Rating points per player-season above the norm, shrunk toward zero
    /// when he has coached few players there.
    pub fn growth_read(&self) -> f64 {
        self.growth / (self.seasons + 8.0)
    }

    pub fn fade(&mut self, k: f64) {
        self.growth *= k;
        self.seasons *= k;
        self.moved_up *= k;
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Coach {
    pub id: usize,
    pub name: String,
    pub ethnicity: Ethnicity,
    pub age: u32,
    /// His date of birth (A1): the age the game shows counts from it.
    #[serde(default = "crate::calendar::born::unknown")]
    pub born: crate::calendar::Date,
    /// Hidden. The game shows reputation, never this.
    pub(crate) ability: Ability,
    /// How far his ability can still grow (hidden), 1-20.
    pub(crate) learning_ceiling: f64,
    /// Public reputation, 1-20: what schools and the media think of him.
    pub reputation: f64,
    pub personality: CoachPersonality,
    /// The head coach he learned under longest: his coaching tree.
    pub mentor: Option<usize>,
    pub ex_pro: bool,
    pub job: Option<Job>,
    pub history: Vec<Stint>,
    /// Results against expectation, most recent last (+1 = a season one win-share above).
    pub recent: Vec<f64>,
    /// Current stint record.
    pub wins: u32,
    pub losses: u32,
    pub titles: u32,
    /// State or conference or national titles across his career.
    pub career_titles: u32,
    pub retired: bool,
    /// The year a school gave him a "prove it" extension instead of firing him (note 1).
    pub prove_it: Option<i32>,
    /// Recruiting ties to high schools, 0-1, built from impact players he
    /// coached. They travel with him from job to job (note 4).
    pub ties: HashMap<usize, f64>,
    /// The group he came up coaching (a DB coach, an offensive line coach):
    /// public, part of his story.
    pub came_up: Group,
    /// His visible teaching record by group.
    pub units: [UnitRecord; GROUP_COUNT],
    /// Hidden, 1-20: how soon he takes his starters out of a decided game
    /// (dynasty item 3). Not part of his reputation.
    pub(crate) sportsmanship: f64,
    /// Hidden: the state he calls home, where his family roots are (11.13).
    /// Set with his first job.
    pub(crate) home_state: Option<usize>,
    /// Hidden: the age his sharpness starts to fade, and how fast (11.14).
    pub(crate) decline_from: f64,
    pub(crate) decline_rate: f64,
    /// The last season an NCAA show-cause order keeps him out of college
    /// coaching (dynasty item 11.6).
    pub show_cause: Option<i32>,
    /// A college head coach's contract (trial 4 topic 4).
    pub contract: Option<Contract>,
    /// Hidden: what he knows of each system (S1-S4).
    pub(crate) systems: super::systems::Systems,
    /// Buzz from his units' numbers (world item 5), fading by half a season.
    pub stat_buzz: f64,
    /// The style set in his coach profile (the user's coach); AI coaches
    /// leave it empty and coach to the style their nature gives them.
    pub style_set: Option<crate::tendencies::Style>,
    /// The average discipline of the head coaches at his level, less the
    /// middle (`set_freedom_norms`): freedom is read against it.
    pub freedom_norm: f64,
    /// Your own coach's four extra ratings; None for every AI coach.
    #[serde(default)]
    pub(crate) yours: Option<Yours>,
    /// What he played and how good he was (C11); None if he never played
    /// past high school.
    #[serde(default)]
    pub played: Option<super::former::Played>,
    /// His name (C10), 1-20: what fans, boosters, recruits and small
    /// schools know of him. `reputation` is his standing: what owners,
    /// general managers, big programs' athletic directors and other coaches
    /// think of his coaching. 0 in a world written before the split.
    #[serde(default)]
    pub renown: f64,
    /// The part of his name that comes from his playing days, fading.
    #[serde(default)]
    pub fame: f64,
    /// His junior post (C36): a graduate assistant, analyst or
    /// quality-control coach, held now (`until` None) or before his first
    /// full-time job.
    #[serde(default)]
    pub junior: Option<Junior>,
}

/// A college head coach's contract (trial 4 topic 4): the last season it
/// covers, what it pays a year, and the share of the unpaid rest the
/// school owes if it fires him.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Contract {
    pub through: i32,
    pub salary: f64,
    pub buyout_share: f64,
}

impl Contract {
    /// What firing him after `season` would cost.
    pub fn buyout(&self, season: i32) -> f64 {
        (self.through - season).max(0) as f64 * self.salary * self.buyout_share
    }
}

/// Typical ability by level: (mean, spread).
pub fn level_ability(level: CoachTier) -> (f64, f64) {
    match level {
        CoachTier::HighSchool => (8.0, 2.6),
        CoachTier::SmallCollege => (9.5, 2.3),
        CoachTier::Fcs => (10.5, 2.2),
        CoachTier::Fbs => (12.0, 2.3),
        CoachTier::Nfl => (13.5, 2.0),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum CoachTier {
    HighSchool,
    SmallCollege,
    Fcs,
    Fbs,
    Nfl,
}

pub fn generate_coach(
    id: usize,
    name: String,
    ethnicity: Ethnicity,
    tier: CoachTier,
    age: u32,
    rng: &mut Rng,
) -> Coach {
    generate_coach_as(id, name, ethnicity, tier, age, None, rng)
}

/// The same, with whether he played in the NFL already known (a former
/// player of the world's own, C11).
pub fn generate_coach_as(
    id: usize,
    name: String,
    ethnicity: Ethnicity,
    tier: CoachTier,
    age: u32,
    pro: Option<bool>,
    rng: &mut Rng,
) -> Coach {
    let (mean, sd) = level_ability(tier);
    let core = rng.normal(mean, sd);
    let part = |rng: &mut Rng, spread: f64| clamp(core + rng.normal(0.0, spread), 1.0, 20.0);
    let mut teaching = [0.0; GROUP_COUNT];
    // Most coaches came up on one side of the ball and teach it better.
    let offense = rng.chance(0.5);
    for g in Group::ALL {
        let side = if g.offense() == offense { 1.0 } else { -1.0 };
        teaching[g as usize] = clamp(part(rng, 2.2) + side, 1.0, 20.0);
    }
    // He came up coaching his best group on his own side.
    let came_up = Group::ALL
        .iter()
        .copied()
        .filter(|g| g.offense() == offense && *g != Group::Specialists)
        .max_by(|a, b| teaching[*a as usize].total_cmp(&teaching[*b as usize]))
        .unwrap_or(Group::ALL[0]);
    let drawn = rng.chance(match tier {
        CoachTier::HighSchool => 0.04,
        CoachTier::Nfl => 0.35,
        _ => 0.12,
    });
    let ex_pro = pro.unwrap_or(drawn);
    let mut ability = Ability {
        teaching,
        scheme: part(rng, 2.0),
        game_management: part(rng, 2.0),
        motivation: part(rng, 2.5),
        discipline: part(rng, 2.5),
        recruiting: part(rng, 3.0),
        network: clamp(core - 1.0 + rng.normal(0.0, 3.0), 1.0, 20.0),
        relationships: part(rng, 2.5),
    };
    if ex_pro {
        // A former pro knows people and the game, but teaching is its own skill.
        ability.network = clamp(ability.network + rng.range(3.0, 6.0), 1.0, 20.0);
        for t in ability.teaching.iter_mut() {
            *t = clamp(*t - rng.range(0.0, 1.5), 1.0, 20.0);
        }
    }
    let personality = CoachPersonality {
        ambition: clamp(rng.normal(10.0, 3.5) + if ex_pro { 2.0 } else { 0.0 }, 1.0, 20.0),
        loyalty: clamp(rng.normal(10.0, 3.5), 1.0, 20.0),
        temper: clamp(rng.normal(10.0, 3.5), 1.0, 20.0),
    };
    let learning_ceiling = clamp(core + rng.normal(2.0, 2.0) - (age as f64 - 30.0).max(0.0) * 0.05, 1.0, 20.0);
    // Reputation starts near what he looks like on paper, with plenty of error.
    let reputation =
        clamp(ability.head_coach_value() + rng.normal(0.0, 2.0) + if ex_pro { 1.0 } else { 0.0 }, 1.0, 20.0);
    // Sharpness fades from the late 50s, earlier for some and later for
    // others, at his own pace (11.14; proposed, not confirmed: onset about
    // 59 give or take 3.5 years, 0.08-0.35 points a year to start).
    let decline_from = clamp(rng.normal(59.0, 3.5), 52.0, 68.0);
    let decline_rate = rng.range(0.08, 0.35);
    Coach {
        id,
        name,
        ethnicity,
        age,
        born: crate::calendar::born::UNKNOWN,
        ability,
        learning_ceiling,
        reputation,
        personality,
        mentor: None,
        ex_pro,
        job: None,
        history: Vec::new(),
        recent: Vec::new(),
        wins: 0,
        losses: 0,
        titles: 0,
        career_titles: 0,
        retired: false,
        prove_it: None,
        ties: HashMap::new(),
        came_up,
        units: [UnitRecord::default(); GROUP_COUNT],
        sportsmanship: 10.0,
        home_state: None,
        decline_from,
        decline_rate,
        show_cause: None,
        contract: None,
        systems: Default::default(),
        stat_buzz: 0.0,
        style_set: None,
        freedom_norm: 0.0,
        yours: None,
        played: None,
        renown: 0.0,
        fame: 0.0,
        junior: None,
    }
}

impl Coach {
    /// Development quality he gives one position group, 1-20, as a head coach
    /// who also teaches it himself (high school) or with coordinators.
    pub(crate) fn teaching(&self, g: Group) -> f64 {
        self.ability.teaching[g as usize]
    }

    pub(crate) fn ability(&self) -> &Ability {
        &self.ability
    }

    /// Whether he is the user's own coach.
    pub fn is_yours(&self) -> bool {
        self.yours.is_some()
    }

    /// Hidden, 1-20 (C38): how fast he learns a group he didn't come up
    /// in, and how willing he is to take a job outside it. Keyed by the
    /// coach, so no draw moves (spread like his personality; proposed, not
    /// confirmed).
    pub fn adaptability(&self) -> f64 {
        let k = self.name.bytes().fold(0xADA9_7AB1u64, |h, b| h.rotate_left(5) ^ b as u64);
        clamp(10.0 + 3.5 * super::season::key_normal(k ^ 0xADA7, self.id as u64), 1.0, 20.0)
    }

    /// His play calling, 1-20: your own coach's rating, an AI coach's from
    /// his scheme and game management (`world/season.rs`, `coach_caller`).
    pub fn calling(&self) -> f64 {
        match self.yours {
            Some(y) => y.play_calling,
            None => 0.6 * self.ability.scheme + 0.4 * self.ability.game_management,
        }
    }

    /// The junior post he holds now (C36).
    pub fn junior_now(&self) -> Option<Junior> {
        self.junior.filter(|j| j.until.is_none())
    }

    /// His hidden learning ceiling, for your read of his potential (C35)
    /// and debug tooling; never shown as a number.
    pub fn ceiling(&self) -> f64 {
        self.learning_ceiling
    }

    /// What he knows of each system, 0-1 (hidden: screens show it in
    /// words through your staff's read, never as a number).
    pub fn system_know(&self) -> [f64; 11] {
        self.systems.know
    }

    /// His eye for players (section 7, topic 14): how well he judges a
    /// recruit, 1-20 (hidden). Keyed by the coach, so no draw moves; it
    /// leans on his recruiting and his teaching (proposed, not confirmed).
    pub fn evaluation(&self) -> f64 {
        if let Some(y) = self.yours {
            return y.evaluation;
        }
        let a = &self.ability;
        let teach = a.teaching.iter().sum::<f64>() / GROUP_COUNT as f64;
        let own = super::season::key_normal(0xE7A1, self.id as u64);
        crate::rng::clamp(10.0 + 0.3 * (a.recruiting - 10.0) + 0.3 * (teach - 10.0) + 3.0 * own, 1.0, 20.0)
    }

    /// His coaching style (tendencies, "Your coaching style"). An AI coach's
    /// comes from his personality and ability, so programs get identities:
    /// risk is the aggressiveness his play calling already uses (ambition),
    /// structure his discipline, physical his temper, and the rest his own
    /// lean (proposed, not confirmed). Never drawn from a stream, so no
    /// other draw moves.
    pub fn style(&self) -> crate::tendencies::Style {
        if let Some(s) = self.style_set {
            return s;
        }
        let id = self.id as u64;
        let n = |k: u64| super::season::key_normal(k, id);
        crate::tendencies::Style {
            risk: self.personality.ambition,
            // Read against the head coaches at his level (trial 8 finding
            // 4: top jobs go to disciplined coaches, so nearly every FBS and
            // NFL staff gave little freedom, about 6).
            freedom: clamp(21.0 - self.ability.discipline + self.freedom_norm + 1.5 * n(0xF4EE), 1.0, 20.0),
            physical: clamp(10.5 + 0.4 * (self.personality.temper - 10.0) + 2.5 * n(0x9A11), 1.0, 20.0),
            ball_hunting: clamp(10.5 + 3.0 * n(0xBA11), 1.0, 20.0),
            mould: clamp(10.5 - 0.25 * (self.ability.relationships - 10.0) + 2.5 * n(0x40D1), 1.0, 20.0),
        }
    }

    /// The level of the job he holds, if any.
    pub fn job_level(&self, programs: &[super::program::Program]) -> Option<crate::player::Level> {
        use crate::player::Level;
        self.job.map(|j| match j.employer {
            Employer::HighSchool(_) => Level::HS,
            Employer::College(p) => super::division_level(programs[p].division),
            Employer::Nfl(_) | Employer::Other(_) => Level::NFL,
        })
    }

    /// One year of a coach's career: young coaches learn fast; old ones fall
    /// behind on schemes and lose sharpness. Working under a better head coach
    /// rubs off (his tree).
    /// His age on a day, from his date of birth (A1).
    pub fn age_on(&self, d: crate::calendar::Date) -> u32 {
        crate::calendar::born::age_or(self.born, d, self.age)
    }

    /// How fast each ability part grows in his job (C59, C61), in the
    /// order of `Ability::parts_mut`: teaching by group, then scheme, game
    /// management, motivation, discipline, recruiting, network,
    /// relationships. A position coach learns his own group at full pace
    /// and the others at a third; a coordinator every group on his side,
    /// the group he coaches himself half again faster (C60), the other side
    /// a third; a head coach two thirds everywhere. Adaptability (C38)
    /// speeds a group he didn't come up in: ×0.6 at 1 to ×1.4 at 20.
    /// Scheme and game management grow at full pace for coordinators and
    /// head coaches (an analyst's scheme too), half for the rest; recruiting
    /// at full pace for anyone with a recruiting area (a college staff), a
    /// third elsewhere. Out of work, a third (proposed, not confirmed: the
    /// out-of-work, junior and recruiting paces).
    pub(crate) fn pace(&self) -> [f64; GROUP_COUNT + 7] {
        let mut p = [1.0; GROUP_COUNT + 7];
        let third = 1.0 / 3.0;
        let job = self
            .job
            .map(|j| (j.employer, Some(j.role), None))
            .or_else(|| self.junior_now().map(|j| (j.employer, Some(Role::PositionCoach(j.group)), Some(j.post))));
        let adapt = 0.6 + 0.8 * (self.adaptability() - 1.0) / 19.0;
        let up = self.came_up;
        for g in Group::ALL {
            let base = match job.and_then(|x| x.1) {
                None => third,
                Some(Role::HeadCoach) => 2.0 / 3.0,
                Some(Role::PositionCoach(own)) => {
                    if g == own {
                        1.0
                    } else {
                        third
                    }
                }
                Some(r @ (Role::OffensiveCoordinator | Role::DefensiveCoordinator)) => {
                    let off = r == Role::OffensiveCoordinator;
                    if g == up && g.offense() == off {
                        1.5
                    } else if g.offense() == off && g != Group::Specialists {
                        1.0
                    } else {
                        third
                    }
                }
            };
            p[g as usize] = base * if g == up { 1.0 } else { adapt };
        }
        let big = matches!(
            job.and_then(|x| x.1),
            Some(Role::HeadCoach | Role::OffensiveCoordinator | Role::DefensiveCoordinator)
        );
        let analyst = matches!(job.and_then(|x| x.2), Some(Post::Analyst | Post::QualityControl));
        let n = GROUP_COUNT;
        p[n] = if big || analyst { 1.0 } else { 0.5 };
        p[n + 1] = if big { 1.0 } else { 0.5 };
        p[n + 4] = if matches!(job.map(|x| x.0), Some(Employer::College(_))) { 1.0 } else { third };
        p
    }

    pub fn age_one_year(&mut self, mentor: Option<&Ability>, rng: &mut Rng) {
        self.age += 1;
        let age = self.age as f64;
        // Learning stops in the late 50s; what comes after is decline, below.
        let learn = if age < 35.0 {
            0.45
        } else if age < 48.0 {
            0.2
        } else if age < 58.0 {
            0.05
        } else {
            0.0
        };
        let ceiling = self.learning_ceiling;
        let pace = self.pace();
        for (k, v) in self.ability.parts_mut().enumerate() {
            let room = (ceiling - *v).max(0.0);
            *v = clamp(*v + pace[k] * learn * (room / 6.0).min(1.0) + rng.normal(0.0, 0.15), 1.0, 20.0);
        }
        // Your own four grow the same way, without a draw, so no other
        // coach's stream moves.
        if let Some(y) = self.yours.as_mut() {
            for v in [&mut y.play_calling, &mut y.adaptation, &mut y.evaluation, &mut y.negotiating] {
                let room = (ceiling - *v).max(0.0);
                *v = clamp(*v + learn * (room / 6.0).min(1.0), 1.0, 20.0);
            }
        }
        // Age takes the edge off every coach, at his ceiling or not (11.14):
        // the energy for the road and for teenagers goes first, then the
        // sideline sharpness; what he knows about teaching holds longest, and
        // his network stays. It speeds up a little each year (proposed, not
        // confirmed: weights 1.2 recruiting, 1.0 motivation, 0.8 game
        // management, 0.6 relationships, 0.5 discipline, 0.4 teaching; +15% a year).
        let past = age - self.decline_from;
        if past > 0.0 {
            let d = self.decline_rate * (1.0 + 0.15 * past);
            let a = &mut self.ability;
            for (v, k) in [
                (&mut a.recruiting, 1.2),
                (&mut a.motivation, 1.0),
                (&mut a.game_management, 0.8),
                (&mut a.relationships, 0.6),
                (&mut a.discipline, 0.5),
            ] {
                *v = clamp(*v - k * d, 1.0, 20.0);
            }
            for t in a.teaching.iter_mut() {
                *t = clamp(*t - 0.4 * d, 1.0, 20.0);
            }
            // Sideline sharpness goes with game management.
            if let Some(y) = self.yours.as_mut() {
                y.adaptation = clamp(y.adaptation - 0.8 * d, 1.0, 20.0);
                y.play_calling = clamp(y.play_calling - 0.4 * d, 1.0, 20.0);
            }
        }
        if age >= 58.0 {
            // Schemes move on; an old coach who doesn't adapt falls behind.
            self.ability.scheme = clamp(self.ability.scheme - rng.range(0.1, 0.5), 1.0, 20.0);
        }
        if let Some(m) = mentor {
            // Picks up part of his head coach's skills and network: noteworthy, not a driver.
            for g in 0..GROUP_COUNT {
                let gap = m.teaching[g] - self.ability.teaching[g];
                if gap > 0.0 {
                    self.ability.teaching[g] += 0.06 * gap;
                }
            }
            for (mine, theirs) in [(&mut self.ability.scheme, m.scheme), (&mut self.ability.network, m.network)] {
                if theirs > *mine {
                    *mine += 0.06 * (theirs - *mine);
                }
            }
        }
        // Each season in a job widens his network a little.
        if self.job.is_some() {
            self.ability.network = clamp(self.ability.network + 0.08, 1.0, 20.0);
        }
    }

    /// Updates reputation after a season.
    ///
    /// `vs_expectation`: wins above what his roster should have won, as a share
    /// of the season (about -0.4 to +0.4). `title`: a state, conference or
    /// national title. `locker_room`: incidents and bad signings this year, 0-1.
    ///
    /// Reputation catches up with what results show over about three seasons;
    /// big results give a massive boost (Cignetti at JMU); a bad locker room
    /// and bad signings chip at it slowly. Assistants move at half speed.
    /// `stage`: how big the program is at its level, 0-1. A title at a big
    /// program moves a reputation more, and the top of the scale is hard to reach.
    pub fn update_reputation(
        &mut self,
        vs_expectation: f64,
        title: TitleKind,
        locker_room: f64,
        head: bool,
        stage: f64,
    ) {
        self.season_reputation(vs_expectation, locker_room, head);
        self.title_reputation(title, head, stage);
    }

    /// The season's record against expectation and the locker room, judged
    /// when the regular season ends (section 4: firings come before the
    /// title games).
    pub fn season_reputation(&mut self, vs_expectation: f64, locker_room: f64, head: bool) {
        self.recent.push(vs_expectation);
        if self.recent.len() > 6 {
            self.recent.remove(0);
        }
        let speed = if head { 1.0 } else { 0.5 };
        // What the results say he is.
        let shown = self.ability.head_coach_value() + 12.0 * vs_expectation;
        self.reputation += speed * 0.3 * (shown - self.reputation);
        self.reputation -= speed * 0.6 * locker_room;
        self.reputation = clamp(self.reputation, 1.0, 20.0);
        // His name (C10) follows what he did as a player, fading, or what
        // his coaching shows, whichever is bigger: a head coach's work is
        // seen more than an assistant's (proposed, not confirmed: 0.75 and
        // 0.5 of his standing, the roads' agreed name and standing pairs).
        self.fame *= 0.99;
        let seen = if head { 0.75 } else { 0.5 } * self.reputation;
        let target = self.fame.max(seen);
        let now = self.name_value();
        self.renown = clamp(now + speed * 0.3 * (target - now), 1.0, 20.0);
    }

    /// His name (C10), 1-20.
    pub fn name_value(&self) -> f64 {
        if self.renown > 0.0 { self.renown } else { clamp(self.fame.max(0.5 * self.reputation), 1.0, 20.0) }
    }

    /// A title's boost, when it is won.
    pub fn title_reputation(&mut self, title: TitleKind, head: bool, stage: f64) {
        let speed = if head { 1.0 } else { 0.5 };
        let headroom = ((20.0 - self.reputation) / 8.0).min(1.0);
        self.reputation += speed
            * headroom
            * (0.5 + 0.5 * stage.clamp(0.0, 1.0))
            * match title {
                TitleKind::None => 0.0,
                TitleKind::Conference => 3.0,
                TitleKind::State => 3.0,
                TitleKind::National => 5.0,
            };
        self.reputation = clamp(self.reputation, 1.0, 20.0);
        // A title makes a name as much as a standing (proposed, not
        // confirmed: the same boost).
        let name = self.name_value();
        let room = ((20.0 - name) / 8.0).min(1.0);
        let boost = match title {
            TitleKind::None => 0.0,
            TitleKind::Conference | TitleKind::State => 3.0,
            TitleKind::National => 5.0,
        };
        self.renown = clamp(name + speed * room * (0.5 + 0.5 * stage.clamp(0.0, 1.0)) * boost, 1.0, 20.0);
    }

    /// What a hiring committee sees: reputation, bent by recency bias. The last
    /// season counts double and the one before it once; anything older is forgotten.
    pub fn hiring_buzz(&self) -> f64 {
        self.hiring_buzz_for(0.0)
    }

    /// The same for a search that reads his name as well as his standing
    /// (C10): `name` is the name's share, 0-1.
    pub fn hiring_buzz_for(&self, name: f64) -> f64 {
        let n = self.recent.len();
        let last = if n >= 1 { self.recent[n - 1] } else { 0.0 };
        let before = if n >= 2 { self.recent[n - 2] } else { 0.0 };
        let base = (1.0 - name) * self.reputation + name * self.name_value();
        base + 10.0 * last + 4.0 * before + self.stat_buzz
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TitleKind {
    None,
    State,
    Conference,
    National,
}

/// What a school wants from a hire, 0-1 weights (1.5, topic 3). Small schools pay
/// for network because it gets their kids seen; powerhouses are their own
/// exposure and pay for teaching and scheme.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct HiringNeeds {
    pub network: f64,
    pub teaching: f64,
    pub recruiting: f64,
    /// How much the search reads his name rather than his standing (C10).
    #[serde(default)]
    pub name: f64,
}

pub fn hiring_needs(prestige: f64, is_high_school: bool) -> HiringNeeds {
    let p = clamp(prestige, 0.0, 1.0);
    if is_high_school {
        HiringNeeds { network: 0.6 * (1.0 - p) + 0.05, teaching: 0.3 + 0.6 * p, recruiting: 0.0, name: 0.0 }
    } else {
        HiringNeeds { network: 0.25 * (1.0 - p), teaching: 0.35 + 0.25 * p, recruiting: 0.4 - 0.1 * p, name: 0.0 }
    }
}

/// How a search committee scores a candidate. They see reputation (with recency
/// bias) and rough reads of his parts; an ex-pro's name opens doors at small schools.
pub fn candidate_score(c: &Coach, needs: HiringNeeds, rng: &mut Rng) -> f64 {
    let a = &c.ability;
    let teach: f64 = a.teaching.iter().sum::<f64>() / GROUP_COUNT as f64;
    let read = |v: f64, rng: &mut Rng| v + rng.normal(0.0, 2.5);
    c.hiring_buzz_for(needs.name)
        + needs.network * (read(a.network, rng) - 10.0) * 1.2
        + needs.teaching * (read(teach, rng) - 10.0) * 0.6
        + needs.recruiting * (read(a.recruiting, rng) - 10.0) * 0.6
        + if c.ex_pro { 2.0 * needs.network } else { 0.0 }
        + rng.normal(0.0, 1.0)
}

/// Each coach's freedom is read against the head coaches at his level: the
/// level's average discipline less the middle, so each level has strict and
/// free staffs around the middle (trial 8 finding 4).
pub fn set_freedom_norms(w: &mut super::World) {
    let mut sum = [0.0f64; 5];
    let mut n = [0.0f64; 5];
    use crate::player::Level;
    let idx = |l: Level| match l {
        Level::HS => 0,
        Level::FBS => 1,
        Level::FCS => 2,
        Level::SmallCollege => 3,
        Level::NFL => 4,
    };
    for c in &w.coaches {
        if let (Some(j), Some(l)) = (c.job, c.job_level(&w.programs))
            && j.role == Role::HeadCoach
            && !c.retired
        {
            sum[idx(l)] += c.ability.discipline;
            n[idx(l)] += 1.0;
        }
    }
    for i in 0..w.coaches.len() {
        if let Some(l) = w.coaches[i].job_level(&w.programs)
            && n[idx(l)] > 0.0
        {
            w.coaches[i].freedom_norm = sum[idx(l)] / n[idx(l)] - 10.5;
        }
    }
}
