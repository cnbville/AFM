//! Coaches (topic 3): a hidden ability split into parts, a public reputation
//! that lags behind it, a personality, a career arc and a coaching tree.

use crate::positions::Position;
use crate::rng::{Rng, clamp};

/// Position groups a coach teaches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
            Position::K | Position::P => Group::Specialists,
        }
    }

    pub fn offense(self) -> bool {
        matches!(self, Group::QB | Group::RB | Group::Receivers | Group::OL)
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
#[derive(Clone, Copy, Debug, PartialEq)]
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

/// Personality, 1-20 each.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CoachPersonality {
    /// Wants to move up.
    pub ambition: f64,
    /// Stays at one school.
    pub loyalty: f64,
    pub temper: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    HeadCoach,
    OffensiveCoordinator,
    DefensiveCoordinator,
}

impl Role {
    pub fn name(self) -> &'static str {
        match self {
            Role::HeadCoach => "head coach",
            Role::OffensiveCoordinator => "offensive coordinator",
            Role::DefensiveCoordinator => "defensive coordinator",
        }
    }
}

/// Where a coach works.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Employer {
    HighSchool(usize),
    College(usize),
    Nfl(usize),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Job {
    pub employer: Employer,
    pub role: Role,
    pub since: i32,
}

/// One finished job, for the coach's record.
#[derive(Clone, Debug)]
pub struct Stint {
    pub employer: Employer,
    pub role: Role,
    pub from: i32,
    pub to: i32,
    pub wins: u32,
    pub losses: u32,
    pub titles: u32,
    /// Why it ended, in plain words.
    pub ended: &'static str,
}

#[derive(Clone, Debug)]
pub struct Coach {
    pub id: usize,
    pub name: String,
    pub age: u32,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CoachTier {
    HighSchool,
    SmallCollege,
    Fcs,
    Fbs,
    Nfl,
}

pub fn generate_coach(id: usize, name: String, tier: CoachTier, age: u32, rng: &mut Rng) -> Coach {
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
    let ex_pro = rng.chance(match tier {
        CoachTier::HighSchool => 0.04,
        CoachTier::Nfl => 0.35,
        _ => 0.12,
    });
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
    Coach {
        id,
        name,
        age,
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

    /// One year of a coach's career: young coaches learn fast; old ones fall
    /// behind on schemes. Working under a better head coach rubs off (his tree).
    pub fn age_one_year(&mut self, mentor: Option<&Ability>, rng: &mut Rng) {
        self.age += 1;
        let age = self.age as f64;
        let learn = if age < 35.0 {
            0.45
        } else if age < 48.0 {
            0.2
        } else if age < 58.0 {
            0.05
        } else {
            -0.1
        };
        let ceiling = self.learning_ceiling;
        for v in self.ability.parts_mut() {
            let room = (ceiling - *v).max(0.0);
            *v = clamp(*v + learn * (room / 6.0).min(1.0) + rng.normal(0.0, 0.15), 1.0, 20.0);
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
        self.recent.push(vs_expectation);
        if self.recent.len() > 6 {
            self.recent.remove(0);
        }
        let speed = if head { 1.0 } else { 0.5 };
        // What the results say he is.
        let shown = self.ability.head_coach_value() + 12.0 * vs_expectation;
        self.reputation += speed * 0.3 * (shown - self.reputation);
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
        self.reputation -= speed * 0.6 * locker_room;
        self.reputation = clamp(self.reputation, 1.0, 20.0);
    }

    /// What a hiring committee sees: reputation, bent by recency bias. The last
    /// season counts double and the one before it once; anything older is forgotten.
    pub fn hiring_buzz(&self) -> f64 {
        let n = self.recent.len();
        let last = if n >= 1 { self.recent[n - 1] } else { 0.0 };
        let before = if n >= 2 { self.recent[n - 2] } else { 0.0 };
        self.reputation + 10.0 * last + 4.0 * before
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TitleKind {
    None,
    State,
    Conference,
    National,
}

/// What a school wants from a hire, 0-1 weights (1.5, topic 3). Small schools pay
/// for network because it gets their kids seen; powerhouses are their own
/// exposure and pay for teaching and scheme.
#[derive(Clone, Copy, Debug)]
pub struct HiringNeeds {
    pub network: f64,
    pub teaching: f64,
    pub recruiting: f64,
}

pub fn hiring_needs(prestige: f64, is_high_school: bool) -> HiringNeeds {
    let p = clamp(prestige, 0.0, 1.0);
    if is_high_school {
        HiringNeeds { network: 0.6 * (1.0 - p) + 0.05, teaching: 0.3 + 0.6 * p, recruiting: 0.0 }
    } else {
        HiringNeeds { network: 0.25 * (1.0 - p), teaching: 0.35 + 0.25 * p, recruiting: 0.4 - 0.1 * p }
    }
}

/// How a search committee scores a candidate. They see reputation (with recency
/// bias) and rough reads of his parts; an ex-pro's name opens doors at small schools.
pub fn candidate_score(c: &Coach, needs: HiringNeeds, rng: &mut Rng) -> f64 {
    let a = &c.ability;
    let teach: f64 = a.teaching.iter().sum::<f64>() / GROUP_COUNT as f64;
    let read = |v: f64, rng: &mut Rng| v + rng.normal(0.0, 2.5);
    c.hiring_buzz()
        + needs.network * (read(a.network, rng) - 10.0) * 1.2
        + needs.teaching * (read(teach, rng) - 10.0) * 0.6
        + needs.recruiting * (read(a.recruiting, rng) - 10.0) * 0.6
        + if c.ex_pro { 2.0 * needs.network } else { 0.0 }
        + rng.normal(0.0, 1.0)
}
