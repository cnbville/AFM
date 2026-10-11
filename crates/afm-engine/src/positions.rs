//! Positions, what each one needs, and how a player's overall at a position is computed.

use std::sync::LazyLock;

use crate::attributes::{ATTRIBUTE_COUNT, Attr, Attributes};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum Position {
    QB,
    RB,
    WR,
    TE,
    OT,
    IOL,
    EDGE,
    DT,
    LB,
    CB,
    S,
    K,
    P,
}

pub const POSITION_COUNT: usize = 13;

impl Position {
    pub const ALL: [Position; POSITION_COUNT] = [
        Position::QB,
        Position::RB,
        Position::WR,
        Position::TE,
        Position::OT,
        Position::IOL,
        Position::EDGE,
        Position::DT,
        Position::LB,
        Position::CB,
        Position::S,
        Position::K,
        Position::P,
    ];

    #[inline]
    pub fn index(self) -> usize {
        self as usize
    }

    pub fn name(self) -> &'static str {
        ["QB", "RB", "WR", "TE", "OT", "IOL", "EDGE", "DT", "LB", "CB", "S", "K", "P"][self as usize]
    }

    pub fn profile(self) -> &'static PositionProfile {
        &POSITION_PROFILES[self as usize]
    }

    /// Kickers and punters.
    #[inline]
    pub fn is_specialist(self) -> bool {
        matches!(self, Position::K | Position::P)
    }
}

impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

pub struct PositionProfile {
    /// Relative importance of each attribute to the position's overall rating.
    pub weights: &'static [(Attr, f64)],
    /// Attribute shape of an elite player at the position. Ceilings are drawn by
    /// scaling this shape to the player's talent, so a great guard is strong and
    /// slow while a great corner is fast and light. Unlisted attributes are 45.
    pub elite_shape: &'static [(Attr, f64)],
    /// Years added to every attribute's decline age (QBs and specialists last
    /// longer). Physical attributes take at most 1.5 of it.
    pub aging_offset: f64,
    /// Relative chance of injury per season (1 = league average).
    pub injury_risk: f64,
    /// Share of high school players listed at the position.
    pub share: f64,
}

const DEFAULT_SHAPE: f64 = 45.0;

use Attr::*;

/// Indexed by `Position`.
pub const POSITION_PROFILES: [PositionProfile; POSITION_COUNT] = [
    PositionProfile {
        weights: &[
            (ThrowAccuracy, 5.0),
            (ThrowPower, 3.0),
            (Awareness, 4.0),
            (Poise, 3.0),
            (Vision, 1.0),
            (Agility, 0.5),
            (Speed, 0.5),
        ],
        elite_shape: &[
            (ThrowAccuracy, 96.0),
            (ThrowPower, 95.0),
            (Awareness, 96.0),
            (Poise, 95.0),
            (Vision, 85.0),
            (Speed, 72.0),
            (Acceleration, 74.0),
            (Agility, 75.0),
            (Strength, 60.0),
            (Stamina, 80.0),
            (Carrying, 65.0),
            (Jumping, 60.0),
        ],
        aging_offset: 3.0,
        injury_risk: 0.7,
        share: 0.06,
    },
    PositionProfile {
        weights: &[
            (Speed, 3.0),
            (Acceleration, 3.0),
            (Agility, 3.0),
            (Elusiveness, 3.0),
            (Vision, 3.0),
            (Carrying, 2.0),
            (Strength, 1.5),
            (Catching, 1.0),
            (PassBlocking, 0.5),
            (Awareness, 1.0),
        ],
        elite_shape: &[
            (Speed, 94.0),
            (Acceleration, 95.0),
            (Agility, 95.0),
            (Elusiveness, 95.0),
            (Vision, 94.0),
            (Carrying, 92.0),
            (Strength, 80.0),
            (Catching, 78.0),
            (PassBlocking, 65.0),
            (Awareness, 85.0),
            (Jumping, 80.0),
            (Stamina, 85.0),
            (RouteRunning, 65.0),
        ],
        aging_offset: -2.5,
        injury_risk: 1.4,
        share: 0.09,
    },
    PositionProfile {
        weights: &[
            (Catching, 4.0),
            (RouteRunning, 4.0),
            (Speed, 3.0),
            (Acceleration, 2.0),
            (Agility, 1.5),
            (Jumping, 1.0),
            (Elusiveness, 1.0),
            (Awareness, 1.5),
        ],
        elite_shape: &[
            (Catching, 95.0),
            (RouteRunning, 95.0),
            (Speed, 95.0),
            (Acceleration, 94.0),
            (Agility, 93.0),
            (Jumping, 90.0),
            (Elusiveness, 88.0),
            (Awareness, 88.0),
            (Strength, 62.0),
            (Stamina, 85.0),
            (Carrying, 75.0),
            (Vision, 80.0),
        ],
        aging_offset: 0.0,
        injury_risk: 1.0,
        share: 0.15,
    },
    PositionProfile {
        weights: &[
            (Catching, 3.0),
            (RouteRunning, 2.0),
            (RunBlocking, 2.5),
            (PassBlocking, 1.0),
            (Strength, 2.0),
            (Speed, 1.5),
            (Awareness, 1.5),
        ],
        elite_shape: &[
            (Catching, 92.0),
            (RouteRunning, 88.0),
            (RunBlocking, 88.0),
            (PassBlocking, 82.0),
            (Strength, 86.0),
            (Speed, 84.0),
            (Acceleration, 84.0),
            (Agility, 80.0),
            (Awareness, 90.0),
            (Jumping, 82.0),
            (Stamina, 85.0),
            (Carrying, 75.0),
        ],
        aging_offset: 0.5,
        injury_risk: 1.1,
        share: 0.06,
    },
    PositionProfile {
        weights: &[(PassBlocking, 5.0), (RunBlocking, 3.0), (Strength, 3.0), (Agility, 1.5), (Awareness, 2.0)],
        elite_shape: &[
            (PassBlocking, 96.0),
            (RunBlocking, 92.0),
            (Strength, 94.0),
            (Agility, 75.0),
            (Awareness, 92.0),
            (Acceleration, 70.0),
            (Speed, 58.0),
            (Stamina, 85.0),
        ],
        aging_offset: 1.5,
        injury_risk: 0.9,
        share: 0.09,
    },
    PositionProfile {
        weights: &[(RunBlocking, 5.0), (PassBlocking, 3.0), (Strength, 4.0), (Awareness, 2.0), (Agility, 0.5)],
        elite_shape: &[
            (RunBlocking, 96.0),
            (PassBlocking, 92.0),
            (Strength, 96.0),
            (Awareness, 92.0),
            (Agility, 68.0),
            (Acceleration, 66.0),
            (Speed, 55.0),
            (Stamina, 85.0),
        ],
        aging_offset: 1.5,
        injury_risk: 0.9,
        share: 0.13,
    },
    PositionProfile {
        weights: &[
            (PassRush, 5.0),
            (BlockShedding, 3.0),
            (Acceleration, 2.5),
            (Speed, 2.0),
            (Strength, 2.0),
            (Tackling, 1.5),
            (PlayRecognition, 1.5),
        ],
        elite_shape: &[
            (PassRush, 96.0),
            (BlockShedding, 92.0),
            (Acceleration, 92.0),
            (Speed, 86.0),
            (Strength, 88.0),
            (Tackling, 85.0),
            (PlayRecognition, 88.0),
            (Agility, 82.0),
            (Stamina, 82.0),
            (Awareness, 82.0),
        ],
        aging_offset: 0.0,
        injury_risk: 1.0,
        share: 0.08,
    },
    PositionProfile {
        weights: &[
            (BlockShedding, 4.0),
            (Strength, 4.5),
            (PassRush, 2.5),
            (Tackling, 2.0),
            (PlayRecognition, 1.5),
            (Acceleration, 1.0),
        ],
        elite_shape: &[
            (BlockShedding, 95.0),
            (Strength, 97.0),
            (PassRush, 90.0),
            (Tackling, 86.0),
            (PlayRecognition, 88.0),
            (Acceleration, 80.0),
            (Speed, 66.0),
            (Agility, 66.0),
            (Stamina, 78.0),
            (Awareness, 84.0),
        ],
        aging_offset: 0.5,
        injury_risk: 1.0,
        share: 0.08,
    },
    PositionProfile {
        weights: &[
            (Tackling, 4.0),
            (PlayRecognition, 4.0),
            (ZoneCoverage, 2.0),
            (Speed, 2.0),
            (Awareness, 2.0),
            (BlockShedding, 1.5),
            (Strength, 1.0),
        ],
        elite_shape: &[
            (Tackling, 95.0),
            (PlayRecognition, 95.0),
            (ZoneCoverage, 86.0),
            (Speed, 88.0),
            (Awareness, 92.0),
            (BlockShedding, 85.0),
            (Strength, 80.0),
            (Acceleration, 88.0),
            (Agility, 84.0),
            (ManCoverage, 72.0),
            (PassRush, 75.0),
            (Stamina, 88.0),
        ],
        aging_offset: 0.0,
        injury_risk: 1.1,
        share: 0.1,
    },
    PositionProfile {
        weights: &[
            (ManCoverage, 4.5),
            (ZoneCoverage, 3.0),
            (Speed, 3.5),
            (Acceleration, 2.0),
            (Agility, 2.0),
            (PlayRecognition, 2.0),
            (Jumping, 1.0),
            (Catching, 0.5),
        ],
        elite_shape: &[
            (ManCoverage, 96.0),
            (ZoneCoverage, 92.0),
            (Speed, 96.0),
            (Acceleration, 95.0),
            (Agility, 95.0),
            (PlayRecognition, 90.0),
            (Jumping, 90.0),
            (Catching, 75.0),
            (Tackling, 70.0),
            (Strength, 60.0),
            (Stamina, 88.0),
            (Awareness, 86.0),
        ],
        aging_offset: -0.5,
        injury_risk: 1.0,
        share: 0.1,
    },
    PositionProfile {
        weights: &[
            (ZoneCoverage, 4.0),
            (PlayRecognition, 3.5),
            (Tackling, 2.5),
            (Speed, 2.5),
            (ManCoverage, 1.5),
            (Awareness, 1.5),
        ],
        elite_shape: &[
            (ZoneCoverage, 95.0),
            (PlayRecognition, 95.0),
            (Tackling, 88.0),
            (Speed, 92.0),
            (ManCoverage, 85.0),
            (Awareness, 92.0),
            (Acceleration, 92.0),
            (Agility, 88.0),
            (Jumping, 86.0),
            (Catching, 72.0),
            (Strength, 70.0),
            (Stamina, 88.0),
        ],
        aging_offset: -0.5,
        injury_risk: 1.0,
        share: 0.08,
    },
    PositionProfile {
        weights: &[(KickAccuracy, 5.0), (KickPower, 4.0), (Poise, 1.0)],
        elite_shape: &[
            (KickAccuracy, 97.0),
            (KickPower, 96.0),
            (Poise, 90.0),
            (Awareness, 60.0),
            (Speed, 55.0),
            (Strength, 45.0),
        ],
        // Kick power and accuracy already decline late (from 36 and 37), so specialists need little extra.
        aging_offset: 2.0,
        injury_risk: 0.25,
        share: 0.02,
    },
    PositionProfile {
        weights: &[(KickPower, 5.0), (KickAccuracy, 3.0), (Poise, 1.0)],
        elite_shape: &[
            (KickPower, 97.0),
            (KickAccuracy, 93.0),
            (Poise, 88.0),
            (Awareness, 60.0),
            (Speed, 55.0),
            (Strength, 45.0),
        ],
        aging_offset: 2.0,
        injury_risk: 0.25,
        share: 0.02,
    },
];

struct Derived {
    /// Normalised weights per position, dense over every attribute.
    weights: [[f64; ATTRIBUTE_COUNT]; POSITION_COUNT],
    elite_shape: [Attributes; POSITION_COUNT],
    elite_shape_overall: [f64; POSITION_COUNT],
}

// Precomputed because overall ratings are evaluated millions of times in a cohort simulation.
static DERIVED: LazyLock<Derived> = LazyLock::new(|| {
    let mut weights = [[0.0; ATTRIBUTE_COUNT]; POSITION_COUNT];
    let mut elite_shape = [Attributes::filled(DEFAULT_SHAPE); POSITION_COUNT];
    let mut elite_shape_overall = [0.0; POSITION_COUNT];
    for pos in Position::ALL {
        let p = pos.profile();
        let total: f64 = p.weights.iter().map(|(_, w)| w).sum();
        for &(a, w) in p.weights {
            weights[pos.index()][a.index()] = w / total;
        }
        for &(a, v) in p.elite_shape {
            elite_shape[pos.index()][a] = v;
        }
    }
    for pos in Position::ALL {
        elite_shape_overall[pos.index()] = dot(&weights[pos.index()], &elite_shape[pos.index()]);
    }
    Derived { weights, elite_shape, elite_shape_overall }
});

#[inline]
fn dot(w: &[f64; ATTRIBUTE_COUNT], a: &Attributes) -> f64 {
    let mut sum = 0.0;
    for i in 0..ATTRIBUTE_COUNT {
        sum += w[i] * a.0[i];
    }
    sum
}

/// Position overall: weighted mean of the attributes that matter at the position.
#[inline]
pub fn overall_for(position: Position, attrs: &Attributes) -> f64 {
    dot(&DERIVED.weights[position.index()], attrs)
}

/// Normalised weight of an attribute at a position (0 if it doesn't count).
#[inline]
pub fn weight(position: Position, attr: Attr) -> f64 {
    DERIVED.weights[position.index()][attr.index()]
}

pub fn elite_shape_value(position: Position, attr: Attr) -> f64 {
    DERIVED.elite_shape[position.index()][attr]
}

/// Overall an "elite shape" player would have; used to scale talent into ceilings.
pub fn elite_shape_overall(position: Position) -> f64 {
    DERIVED.elite_shape_overall[position.index()]
}

/// Weight of an attribute as listed in the position profile, before normalising (0 if unlisted).
pub fn raw_weight(position: Position, attr: Attr) -> f64 {
    position.profile().weights.iter().find(|(a, _)| *a == attr).map_or(0.0, |(_, w)| *w)
}
