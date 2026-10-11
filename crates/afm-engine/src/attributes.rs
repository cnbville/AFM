//! Player attributes on a 1-99 scale. Each attribute has a development profile
//! describing how mature it is in a 14-year-old, how quickly it grows, what
//! drives that growth, and when age starts to take it away.

use std::ops::{Index, IndexMut};

pub const MIN_RATING: f64 = 1.0;
pub const MAX_RATING: f64 = 99.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum Attr {
    // physical
    Speed,
    Acceleration,
    Agility,
    Strength,
    Jumping,
    Stamina,
    // technical
    ThrowPower,
    ThrowAccuracy,
    Carrying,
    Catching,
    RouteRunning,
    Elusiveness,
    RunBlocking,
    PassBlocking,
    Tackling,
    BlockShedding,
    PassRush,
    ManCoverage,
    ZoneCoverage,
    KickPower,
    KickAccuracy,
    // mental
    Awareness,
    PlayRecognition,
    Vision,
    Poise,
}

pub const ATTRIBUTE_COUNT: usize = 25;

impl Attr {
    pub const ALL: [Attr; ATTRIBUTE_COUNT] = [
        Attr::Speed,
        Attr::Acceleration,
        Attr::Agility,
        Attr::Strength,
        Attr::Jumping,
        Attr::Stamina,
        Attr::ThrowPower,
        Attr::ThrowAccuracy,
        Attr::Carrying,
        Attr::Catching,
        Attr::RouteRunning,
        Attr::Elusiveness,
        Attr::RunBlocking,
        Attr::PassBlocking,
        Attr::Tackling,
        Attr::BlockShedding,
        Attr::PassRush,
        Attr::ManCoverage,
        Attr::ZoneCoverage,
        Attr::KickPower,
        Attr::KickAccuracy,
        Attr::Awareness,
        Attr::PlayRecognition,
        Attr::Vision,
        Attr::Poise,
    ];

    #[inline]
    pub fn index(self) -> usize {
        self as usize
    }

    pub fn key(self) -> &'static str {
        ATTRIBUTE_KEYS[self as usize]
    }

    #[inline]
    pub fn profile(self) -> &'static AttributeProfile {
        &ATTRIBUTE_PROFILES[self as usize]
    }
}

/// camelCase names, as in the TypeScript model and the trial files.
pub const ATTRIBUTE_KEYS: [&str; ATTRIBUTE_COUNT] = [
    "speed",
    "acceleration",
    "agility",
    "strength",
    "jumping",
    "stamina",
    "throwPower",
    "throwAccuracy",
    "carrying",
    "catching",
    "routeRunning",
    "elusiveness",
    "runBlocking",
    "passBlocking",
    "tackling",
    "blockShedding",
    "passRush",
    "manCoverage",
    "zoneCoverage",
    "kickPower",
    "kickAccuracy",
    "awareness",
    "playRecognition",
    "vision",
    "poise",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AttributeGroup {
    Physical,
    Technical,
    Mental,
}

#[derive(Clone, Copy, Debug)]
pub struct AttributeProfile {
    pub group: AttributeGroup,
    /// Share of the eventual ceiling a typical 14-year-old already has.
    pub maturity_at_14: f64,
    /// Fraction of the remaining gap to the ceiling closed in a normal year at prime development age.
    pub growth_rate: f64,
    /// Ages (inclusive) over which growth runs at full rate before tapering.
    pub growth_window: (f64, f64),
    /// How much growth depends on coaching and training (0 = innate, 1 = fully coached).
    pub coachability: f64,
    /// How much growth depends on game snaps (0 = not at all, 1 = only learned by playing).
    pub experience: f64,
    /// Typical age decline begins, before player and position adjustments.
    pub decline_age: f64,
    /// Points lost per year one year past decline_age; grows linearly each year after.
    pub decline_rate: f64,
    /// Whether a major injury can permanently cost this attribute.
    pub injury_sensitive: bool,
}

#[allow(clippy::too_many_arguments)]
const fn physical(m: f64, g: f64, w: (f64, f64), c: f64, da: f64, dr: f64, inj: bool) -> AttributeProfile {
    AttributeProfile {
        group: AttributeGroup::Physical,
        maturity_at_14: m,
        growth_rate: g,
        growth_window: w,
        coachability: c,
        experience: 0.1,
        decline_age: da,
        decline_rate: dr,
        injury_sensitive: inj,
    }
}

const fn technical(m: f64, g: f64, w: (f64, f64), c: f64, e: f64, da: f64, dr: f64) -> AttributeProfile {
    AttributeProfile {
        group: AttributeGroup::Technical,
        maturity_at_14: m,
        growth_rate: g,
        growth_window: w,
        coachability: c,
        experience: e,
        decline_age: da,
        decline_rate: dr,
        injury_sensitive: false,
    }
}

const fn mental(m: f64, g: f64, w: (f64, f64), e: f64, da: f64, dr: f64) -> AttributeProfile {
    AttributeProfile {
        group: AttributeGroup::Mental,
        maturity_at_14: m,
        growth_rate: g,
        growth_window: w,
        coachability: 0.4,
        experience: e,
        decline_age: da,
        decline_rate: dr,
        injury_sensitive: false,
    }
}

/// Indexed by `Attr`.
pub const ATTRIBUTE_PROFILES: [AttributeProfile; ATTRIBUTE_COUNT] = [
    // Speed is largely genetic: high at 14, modest gains, first thing to go.
    physical(0.8, 0.24, (14.0, 20.0), 0.25, 27.0, 0.9, true), // speed
    physical(0.76, 0.24, (14.0, 21.0), 0.35, 27.0, 0.8, true), // acceleration
    physical(0.74, 0.24, (14.0, 21.0), 0.35, 27.0, 0.8, true), // agility
    // Strength is built in the weight room through the college years.
    physical(0.48, 0.2, (15.0, 24.0), 0.8, 31.0, 0.6, false), // strength
    physical(0.74, 0.22, (14.0, 21.0), 0.3, 28.0, 0.8, true), // jumping
    physical(0.62, 0.22, (14.0, 24.0), 0.7, 30.0, 0.6, false), // stamina
    technical(0.58, 0.2, (14.0, 22.0), 0.5, 0.2, 34.0, 0.6),  // throwPower
    technical(0.48, 0.15, (15.0, 27.0), 0.8, 0.6, 35.0, 0.6), // throwAccuracy
    technical(0.58, 0.18, (14.0, 24.0), 0.7, 0.4, 32.0, 0.5), // carrying
    technical(0.56, 0.17, (14.0, 24.0), 0.6, 0.4, 32.0, 0.5), // catching
    technical(0.46, 0.15, (15.0, 26.0), 0.8, 0.6, 32.0, 0.5), // routeRunning
    technical(0.64, 0.18, (14.0, 22.0), 0.3, 0.4, 27.0, 0.8), // elusiveness
    technical(0.46, 0.16, (15.0, 26.0), 0.8, 0.6, 32.0, 0.5), // runBlocking
    technical(0.42, 0.15, (15.0, 27.0), 0.8, 0.7, 32.0, 0.5), // passBlocking
    technical(0.52, 0.17, (14.0, 24.0), 0.7, 0.5, 31.0, 0.6), // tackling
    technical(0.46, 0.16, (15.0, 26.0), 0.7, 0.6, 31.0, 0.6), // blockShedding
    technical(0.46, 0.16, (15.0, 26.0), 0.7, 0.6, 30.0, 0.7), // passRush
    technical(0.48, 0.16, (15.0, 25.0), 0.7, 0.6, 29.0, 0.8), // manCoverage
    technical(0.46, 0.15, (15.0, 27.0), 0.7, 0.7, 32.0, 0.5), // zoneCoverage
    technical(0.62, 0.2, (14.0, 23.0), 0.5, 0.1, 36.0, 0.5),  // kickPower
    technical(0.52, 0.16, (14.0, 28.0), 0.8, 0.4, 37.0, 0.4), // kickAccuracy
    // Mental attributes are learned by playing and keep improving into the 30s.
    mental(0.4, 0.13, (16.0, 30.0), 0.8, 35.0, 0.5),  // awareness
    mental(0.4, 0.13, (16.0, 30.0), 0.8, 34.0, 0.5),  // playRecognition
    mental(0.55, 0.14, (14.0, 26.0), 0.6, 31.0, 0.6), // vision
    mental(0.42, 0.12, (16.0, 31.0), 0.8, 36.0, 0.4), // poise
];

/// One value per attribute.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Attributes(pub [f64; ATTRIBUTE_COUNT]);

impl Attributes {
    pub const fn filled(value: f64) -> Self {
        Attributes([value; ATTRIBUTE_COUNT])
    }
}

impl Default for Attributes {
    fn default() -> Self {
        Attributes::filled(MIN_RATING)
    }
}

impl Index<Attr> for Attributes {
    type Output = f64;
    #[inline]
    fn index(&self, a: Attr) -> &f64 {
        &self.0[a as usize]
    }
}

impl IndexMut<Attr> for Attributes {
    #[inline]
    fn index_mut(&mut self, a: Attr) -> &mut f64 {
        &mut self.0[a as usize]
    }
}
