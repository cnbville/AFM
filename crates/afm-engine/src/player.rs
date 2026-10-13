//! Players, their hidden traits and season history, and prospect generation.

use std::sync::atomic::{AtomicU32, Ordering};

use crate::attributes::{ATTRIBUTE_PROFILES, Attr, Attributes, MAX_RATING, MIN_RATING};
use crate::positions::{Position, elite_shape_overall, elite_shape_value, overall_for};
use crate::rng::{Rng, clamp, round_half_up};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Level {
    HS,
    FBS,
    FCS,
    SmallCollege,
    NFL,
}

impl Level {
    pub fn is_college(self) -> bool {
        matches!(self, Level::FBS | Level::FCS | Level::SmallCollege)
    }
    pub fn name(self) -> &'static str {
        match self {
            Level::HS => "HS",
            Level::FBS => "FBS",
            Level::FCS => "FCS",
            Level::SmallCollege => "SmallCollege",
            Level::NFL => "NFL",
        }
    }
}

/// Hidden personality traits on Football Manager's 1-20 scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Personality {
    /// Effort in training: speeds growth, slows decline, pushes potential up.
    pub work_ethic: f64,
    /// How well instruction sticks: amplifies good coaching and protects against bad.
    pub coachability: f64,
    /// Drive to beat the man across from him: helps breakouts and protects against stagnation.
    pub competitiveness: f64,
    /// Love of the game and durability of mind: some keep playing through the grind, others walk away early.
    pub love_of_game: f64,
}

/// Practice squad players train all year without playing; an unsigned free
/// agent trains on his own while he waits for a call.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlayingTime {
    Starter,
    Rotation,
    Backup,
    Redshirt,
    PracticeSquad,
    Unsigned,
}

impl PlayingTime {
    pub fn name(self) -> &'static str {
        match self {
            PlayingTime::Starter => "starter",
            PlayingTime::Rotation => "rotation",
            PlayingTime::Backup => "backup",
            PlayingTime::Redshirt => "redshirt",
            PlayingTime::PracticeSquad => "practiceSquad",
            PlayingTime::Unsigned => "unsigned",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DevelopmentEvent {
    GrowthSpurt,
    /// His body stopped developing: physical ceilings settle where they are and don't rise again.
    BodyStalled,
    Breakout,
    Stagnation,
    LostSeason,
    /// Suspended or academically ineligible for the season.
    Suspended,
    /// Dismissed from his college team.
    Dismissed,
    Slump,
    MinorInjury,
    ModerateInjury,
    MajorInjury,
}

impl DevelopmentEvent {
    pub const ALL: [DevelopmentEvent; 11] = [
        DevelopmentEvent::GrowthSpurt,
        DevelopmentEvent::BodyStalled,
        DevelopmentEvent::Breakout,
        DevelopmentEvent::Stagnation,
        DevelopmentEvent::LostSeason,
        DevelopmentEvent::Suspended,
        DevelopmentEvent::Dismissed,
        DevelopmentEvent::Slump,
        DevelopmentEvent::MinorInjury,
        DevelopmentEvent::ModerateInjury,
        DevelopmentEvent::MajorInjury,
    ];

    pub fn name(self) -> &'static str {
        match self {
            DevelopmentEvent::GrowthSpurt => "growthSpurt",
            DevelopmentEvent::BodyStalled => "bodyStalled",
            DevelopmentEvent::Breakout => "breakout",
            DevelopmentEvent::Stagnation => "stagnation",
            DevelopmentEvent::LostSeason => "lostSeason",
            DevelopmentEvent::Suspended => "suspended",
            DevelopmentEvent::Dismissed => "dismissed",
            DevelopmentEvent::Slump => "slump",
            DevelopmentEvent::MinorInjury => "minorInjury",
            DevelopmentEvent::ModerateInjury => "moderateInjury",
            DevelopmentEvent::MajorInjury => "majorInjury",
        }
    }
}

/// A season's events, as a small bit set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Events(u16);

impl Events {
    #[inline]
    pub fn has(self, e: DevelopmentEvent) -> bool {
        self.0 & (1 << e as u16) != 0
    }
    #[inline]
    pub fn add(&mut self, e: DevelopmentEvent) {
        self.0 |= 1 << e as u16;
    }
    #[inline]
    pub fn remove(&mut self, e: DevelopmentEvent) {
        self.0 &= !(1 << e as u16);
    }
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
    pub fn iter(self) -> impl Iterator<Item = DevelopmentEvent> {
        DevelopmentEvent::ALL.into_iter().filter(move |e| self.has(*e))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SeasonRecord {
    pub age: u32,
    pub level: Level,
    /// Where he played that season.
    pub position: Position,
    pub playing_time: PlayingTime,
    /// Rating going into the season: what he played the season at, and what his role was picked from.
    pub overall_start: f64,
    /// Rating at the end of the year, after the season and the offseason that follows.
    pub overall: f64,
    /// True potential at the end of the year. Hidden: see `debug`.
    pub(crate) potential: f64,
    /// On-field level shown that season: his rating during the season plus form,
    /// slumps and breakouts. None for a season without games (a redshirt year or
    /// one lost to suspension or dismissal).
    pub performance: Option<f64>,
    /// 1-99 for the rating he played the season at, placed against his own
    /// position and then ranked nationally: high schoolers against everyone their
    /// age, college players against all FBS players, pros simply their overall
    /// (filled in by a cohort simulation).
    pub level_rating: Option<u8>,
    /// Where his rating at the end of the year ranks against everyone else's at the end of theirs.
    pub level_rating_end: Option<u8>,
    /// Quality of the coaching he had that season, 1-20.
    pub coaching: f64,
    pub events: Events,
    /// Games missed through injury.
    pub games_missed: u32,
    /// Size of his first-year dip after a step up (0 = ready at once, 1 = a very hard first year).
    pub adjustment: Option<f64>,
    /// Pro team he spent the season with (index into the league).
    pub team: Option<usize>,
    /// Depth chart spot, e.g. "WR2", "practice squad" or "free agent".
    pub depth: Option<String>,
    /// How his pro surroundings helped (+) or hurt (-) him that season, -1 to 1.
    pub situation: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Player {
    pub id: u32,
    pub position: Position,
    pub age: u32,
    /// Current ability, visible on the field.
    pub attributes: Attributes,
    /// Hidden ceiling per attribute. Moves over time; never shown directly.
    pub(crate) ceilings: Attributes,
    /// Potential when the player was generated. Hidden: see `debug`.
    pub(crate) original_potential: f64,
    /// Years the player's growth window is shifted: negative = early bloomer, positive = late bloomer.
    pub bloom: f64,
    /// Years added to (or removed from) every decline age.
    pub longevity: f64,
    /// 1-20, higher means more injuries.
    pub injury_proneness: f64,
    /// Wear on his body from heavy workloads and major injuries; speeds physical decline.
    pub wear: f64,
    pub personality: Personality,
    /// The adult body he grows into: height, weight, arms, hands (note 8).
    pub body: crate::body::Body,
    /// Every position he has been placed at, with his age then (topic 5.2).
    pub position_history: Vec<(u32, Position)>,
    pub history: Vec<SeasonRecord>,
}

impl Player {
    /// Current overall rating at the player's position.
    #[inline]
    pub fn overall(&self) -> f64 {
        overall_for(self.position, &self.attributes)
    }

    /// True potential: the overall the player would have if every attribute
    /// reached its current ceiling. Hidden from the user, who only sees scouted
    /// estimates; exposed outside the engine only through `debug`.
    #[inline]
    pub(crate) fn potential(&self) -> f64 {
        overall_for(self.position, &self.ceilings)
    }

    pub fn has_had(&self, e: DevelopmentEvent) -> bool {
        self.history.iter().any(|h| h.events.has(e))
    }
}

/// Talent at maturity for a random high school player. A normal core with a
/// stretched upper tail: most kids top out as decent high school players, a few
/// per thousand have a college ceiling, and a handful per national class have
/// an elite pro ceiling.
pub fn talent_from_z(z: f64) -> f64 {
    let tail = (z - 1.6).max(0.0);
    45.0 + 6.5 * z + 2.75 * tail * tail
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ProspectOptions {
    pub id: Option<u32>,
    pub position: Option<Position>,
    pub age: Option<u32>,
    /// Override the latent talent draw (standard normal z-score).
    pub talent_z: Option<f64>,
}

static NEXT_ID: AtomicU32 = AtomicU32::new(1);

fn trait_1_20(rng: &mut Rng) -> f64 {
    clamp(round_half_up(rng.normal(10.0, 3.5)), 1.0, 20.0)
}

pub fn generate_prospect(rng: &mut Rng, opts: ProspectOptions) -> Player {
    let age = opts.age.unwrap_or(14);
    let z = match opts.talent_z {
        Some(z) => z,
        None => rng.normal(0.0, 1.0),
    };
    let talent = clamp(talent_from_z(z), 20.0, 99.0);
    let id = opts.id.unwrap_or_else(|| NEXT_ID.fetch_add(1, Ordering::Relaxed));

    // Athlete first (topic 5.2): body, athletic traits and skill leanings come
    // before any position. A fixed position (tests, tools) builds the player
    // around that position's elite shape instead.
    let (position, ceilings, body) = match opts.position {
        None => {
            let (ceilings, body, natural) = crate::athlete::create(rng, talent);
            (natural, ceilings, body)
        }
        Some(position) => {
            let scale = talent / elite_shape_overall(position);
            let mut ceilings = Attributes::default();
            for key in Attr::ALL {
                ceilings[key] =
                    clamp(elite_shape_value(position, key) * scale + rng.normal(0.0, 4.0), MIN_RATING, MAX_RATING);
            }
            let potential = overall_for(position, &ceilings);
            // His build: how his strength and speed ceilings sit against his position's norm.
            let dev = |a: Attr| (ceilings[a] - elite_shape_value(position, a) * scale) / 4.0;
            let body = crate::body::adult_body(id, position, potential, dev(Attr::Strength), dev(Attr::Speed));
            (position, ceilings, body)
        }
    };

    let bloom = clamp(rng.normal(0.0, 1.1), -2.5, 3.0);
    let mut attributes = Attributes::default();
    for key in Attr::ALL {
        let ceiling = ceilings[key];
        let profile = &ATTRIBUTE_PROFILES[key.index()];
        // Early bloomers arrive more physically mature; late bloomers less so.
        let maturity = clamp(
            profile.maturity_at_14 - 0.035 * bloom + 0.03 * (age as f64 - 14.0) + rng.normal(0.0, 0.05),
            0.2,
            0.98,
        );
        // Even the most gifted 14-year-old is still a boy: soften the top end.
        let raw = ceiling * maturity;
        attributes[key] = clamp(if raw > 40.0 { 40.0 + (raw - 40.0) * 0.5 } else { raw }, MIN_RATING, ceiling);
    }
    // Talent is what he can become at the spot that suits him best.
    let original_potential = overall_for(position, &ceilings);
    let longevity = clamp(rng.normal(0.0, 1.3), -3.0, 3.5);
    let injury_proneness = trait_1_20(rng);
    let personality = Personality {
        work_ethic: trait_1_20(rng),
        coachability: trait_1_20(rng),
        competitiveness: trait_1_20(rng),
        love_of_game: trait_1_20(rng),
    };
    Player {
        id,
        position,
        age,
        attributes,
        ceilings,
        original_potential,
        bloom,
        longevity,
        injury_proneness,
        wear: 0.0,
        personality,
        body,
        position_history: vec![(age, position)],
        history: Vec::new(),
    }
}
