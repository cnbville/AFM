//! Synthetic teams for tools and the development checks (topic 10): a
//! roster at a level's typical strength, built from the same player model.

use super::roster::*;
use crate::attributes::{Attr, MAX_RATING, MIN_RATING};
use crate::player::{Level, ProspectOptions, generate_prospect, talent_from_z};
use crate::positions::Position;
use crate::rng::{Rng, clamp};

/// Typical overall of a starter and of a backup at each level, from the
/// player model's own calibration (`calibrate`: NFL seasons average about
/// 76; high school seniors about 32, the best few hundred about 50).
pub fn level_strength(level: Level) -> (f64, f64) {
    match level {
        Level::NFL => (77.0, 69.0),
        Level::FBS => (63.0, 54.0),
        Level::FCS => (55.0, 47.0),
        Level::HS => (42.0, 33.0),
        _ => (50.0, 42.0),
    }
}

fn z_for(talent: f64) -> f64 {
    let (mut lo, mut hi) = (-4.0, 6.0);
    for _ in 0..50 {
        let m = 0.5 * (lo + hi);
        if talent_from_z(m) < talent { lo = m } else { hi = m }
    }
    0.5 * (lo + hi)
}

/// How many of each position a roster carries.
pub fn roster_counts(level: Level) -> [(Position, usize); 13] {
    let big = level != Level::HS;
    [
        (Position::QB, if big { 3 } else { 2 }),
        (Position::RB, if big { 4 } else { 3 }),
        (Position::WR, if big { 6 } else { 4 }),
        (Position::TE, if big { 3 } else { 2 }),
        (Position::OT, if big { 4 } else { 3 }),
        (Position::IOL, if big { 5 } else { 4 }),
        (Position::EDGE, if big { 4 } else { 3 }),
        (Position::DT, if big { 4 } else { 3 }),
        (Position::LB, if big { 5 } else { 3 }),
        (Position::CB, if big { 5 } else { 3 }),
        (Position::S, if big { 4 } else { 2 }),
        (Position::K, 1),
        (Position::P, 1),
    ]
}

/// A synthetic team. `quality` shifts every player's rating (a good team +3,
/// a bad one -3).
pub fn synthetic_team(name: &str, level: Level, quality: f64, scheme: OffScheme, seed: u32) -> GameTeam {
    let mut rng = Rng::new(seed);
    let (starter, backup) = level_strength(level);
    let mut players = Vec::new();
    let mut id = seed.wrapping_mul(1000) & 0x7FFF_FFFF;
    for (pos, n) in roster_counts(level) {
        let starters = starters(pos);
        for k in 0..n {
            let base = if k < starters { starter } else { backup - 2.0 * (k - starters) as f64 };
            let target = clamp(base + quality + rng.normal(0.0, 4.5), 20.0, 97.0);
            id += 1;
            let mut p = generate_prospect(
                &mut rng,
                ProspectOptions { id: Some(id), position: Some(pos), age: Some(14), talent_z: Some(z_for(target)) },
            );
            // Grown into his body and skills for the level.
            let age = match level {
                Level::HS => 16 + (k % 2) as u32,
                Level::NFL => 24 + (k % 6) as u32,
                _ => 19 + (k % 4) as u32,
            };
            p.age = age;
            let share = match level {
                Level::HS => 0.86,
                Level::NFL => 1.0,
                _ => 0.95,
            };
            for a in Attr::ALL {
                p.attributes[a] = clamp(p.ceilings[a] * share, MIN_RATING, MAX_RATING);
            }
            let exp = match level {
                Level::NFL => 2.0 + (k % 6) as f64,
                _ => 1.0 + (k % 3) as f64,
            };
            let know = match level {
                Level::NFL => 0.92,
                Level::HS => 0.8,
                _ => 0.86,
            };
            players.push(GamePlayer::from_player(&p, know, exp));
        }
    }
    let staff = Staff {
        offense: Caller {
            skill: clamp(rng.normal(10.0, 3.0), 1.0, 20.0),
            adaptation: clamp(rng.normal(10.0, 3.0), 1.0, 20.0),
            aggressiveness: clamp(rng.normal(10.0, 3.0), 1.0, 20.0),
        },
        defense: Caller {
            skill: clamp(rng.normal(10.0, 3.0), 1.0, 20.0),
            adaptation: clamp(rng.normal(10.0, 3.0), 1.0, 20.0),
            aggressiveness: clamp(rng.normal(10.0, 3.0), 1.0, 20.0),
        },
        game_management: clamp(rng.normal(10.0, 3.0), 1.0, 20.0),
        strictness: clamp(rng.normal(10.0, 3.0), 1.0, 20.0),
        off_scheme: scheme,
        front: if rng.chance(0.35) { Front::ThreeFour } else { Front::FourThree },
        shell: *rng.pick(&[Shell::Cover3, Shell::Quarters, Shell::ManPressure, Shell::Cover2]),
        playbook: 1.0,
        // From its own stream so the draws above keep their values.
        sportsmanship: clamp(Rng::new(seed ^ 0x5BD1_E995).normal(10.0, 3.5), 1.0, 20.0),
    };
    GameTeam { name: name.to_string(), players, staff }
}

/// A league's worth of synthetic teams with a spread of quality and schemes.
pub fn synthetic_league(level: Level, n: usize, seed: u32) -> Vec<GameTeam> {
    let mut rng = Rng::new(seed ^ 0xA5A5);
    let schemes: &[OffScheme] = match level {
        Level::HS => {
            &[OffScheme::SpreadOption, OffScheme::PowerGap, OffScheme::WingT, OffScheme::AirRaid, OffScheme::WestCoast]
        }
        Level::NFL => {
            &[OffScheme::WestCoast, OffScheme::WideZone, OffScheme::PowerGap, OffScheme::AirRaid, OffScheme::WestCoast]
        }
        _ => &[
            OffScheme::SpreadOption,
            OffScheme::AirRaid,
            OffScheme::WestCoast,
            OffScheme::PowerGap,
            OffScheme::WideZone,
        ],
    };
    (0..n)
        .map(|k| {
            let q = rng.normal(0.0, 2.5);
            let s = *rng.pick(schemes);
            synthetic_team(&format!("Team {}", k + 1), level, q, s, seed.wrapping_add(k as u32 * 7919))
        })
        .collect()
}
