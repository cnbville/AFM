//! Synthetic teams for tools and the development checks (topic 10): a
//! roster at a level's typical strength, built from the same player model.

use super::roster::*;
use crate::attributes::{Attr, MAX_RATING, MIN_RATING};
use crate::player::{Level, ProspectOptions, generate_prospect, talent_from_z};
use crate::positions::{POSITION_COUNT, Position, overall_for};
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

/// Where the world's NFL starters sit against `level_strength` by position:
/// trial 8's depth charts in its last year (2045, the 67 attributes), each
/// position's starters' average over FBS starters' (trial 8 finding 6; it
/// was trial 7's 2025 charts moved by the class checks, with the offensive
/// line well above the world's). The engine is tuned on rosters like the
/// world's (trial 7 finding 1). Proposed, not confirmed.
fn nfl_offset(pos: Position) -> f64 {
    match pos {
        Position::QB => 6.9,
        Position::RB => 3.1,
        Position::WR => 3.2,
        Position::TE => 2.7,
        Position::OT => 5.3,
        Position::IOL => 2.9,
        Position::EDGE => 3.8,
        Position::DT => 1.9,
        Position::LB => 3.2,
        Position::CB => 3.6,
        Position::S => 2.9,
        Position::K => 5.1,
        Position::P => 0.8,
        Position::LS => 0.0,
    }
}

/// The talent draw (z) that gives a player this overall ceiling.
pub fn z_for(talent: f64) -> f64 {
    let (mut lo, mut hi) = (-4.0, 6.0);
    for _ in 0..50 {
        let m = 0.5 * (lo + hi);
        if talent_from_z(m) < talent { lo = m } else { hi = m }
    }
    0.5 * (lo + hi)
}

/// How many of each position a roster carries. Above high school a team
/// dresses a long snapper (fix list 1.7); a high school team snaps with a
/// player from another position.
pub fn roster_counts(level: Level) -> [(Position, usize); POSITION_COUNT] {
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
        (Position::LS, if big { 1 } else { 0 }),
    ]
}

/// A synthetic team. `quality` shifts every player's rating (a good team +3,
/// a bad one -3).
pub fn synthetic_team(name: &str, level: Level, quality: f64, scheme: OffScheme, seed: u32) -> GameTeam {
    let mut rng = Rng::new(seed);
    // The long snapper draws from his own stream, so the rest of the team
    // is the same as before he was added.
    let mut ls_rng = Rng::new(seed ^ 0x4C53_0000);
    let (starter, backup) = level_strength(level);
    let mut raw = Vec::new();
    let mut id = seed.wrapping_mul(1000) & 0x7FFF_FFFF;
    for (pos, n) in roster_counts(level) {
        let starters = starters(pos);
        for k in 0..n {
            let r = if pos == Position::LS { &mut ls_rng } else { &mut rng };
            let shift = if level == Level::NFL { nfl_offset(pos) } else { 0.0 };
            let base = shift + if k < starters { starter } else { backup - 2.0 * (k - starters) as f64 };
            let target = clamp(base + quality + r.normal(0.0, 4.5), 20.0, 97.0);
            id += 1;
            let mut p = generate_prospect(
                r,
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
                p.attributes.set(a, clamp(p.ceilings.get(a) * share, MIN_RATING, MAX_RATING));
            }
            // A snapper is as good as his snap: set it so his rating fits the level.
            if pos == Position::LS {
                let w = crate::positions::weight(Position::LS, Attr::LongSnapping);
                let rest = overall_for(Position::LS, &p.attributes) - w * p.attributes.get(Attr::LongSnapping);
                p.attributes.set(Attr::LongSnapping, clamp((target - rest) / w, MIN_RATING, MAX_RATING));
            }
            // Habits at the level's spread (tendencies topic 3.2).
            p.tend = crate::tendencies::Tendencies::at_level(
                id,
                level,
                &p.ceilings,
                p.body.weight,
                p.body.height,
                &p.personality,
            );
            let exp = match level {
                Level::NFL => 2.0 + (k % 6) as f64,
                _ => 1.0 + (k % 3) as f64,
            };
            let know = match level {
                Level::NFL => 0.92,
                Level::HS => 0.8,
                _ => 0.86,
            };
            raw.push((p, know, exp));
        }
    }
    // The staff sets its spots (fix list topic 3).
    finish_team(name, level, raw, &mut rng, seed, scheme)
}

/// Depth spots, game players and a staff for a team's players.
fn finish_team(
    name: &str,
    level: Level,
    mut raw: Vec<(crate::player::Player, f64, f64)>,
    rng: &mut Rng,
    seed: u32,
    scheme: OffScheme,
) -> GameTeam {
    let spots = crate::spots::choose_spots(&raw.iter().map(|r| &r.0).collect::<Vec<_>>());
    for (r, s) in raw.iter_mut().zip(spots) {
        crate::spots::set_spot(&mut r.0, s);
    }
    let players: Vec<GamePlayer> = raw.iter().map(|(p, know, exp)| GamePlayer::from_player(p, *know, *exp)).collect();
    // The staff sizes its weekly plan to its players (T6.2).
    let aware = players.iter().map(|p| p.a(Attr::FootballIq)).sum::<f64>() / players.len().max(1) as f64;
    let staff = Staff {
        offense: Caller::new(
            clamp(rng.normal(10.0, 3.0), 1.0, 20.0),
            clamp(rng.normal(10.0, 3.0), 1.0, 20.0),
            clamp(rng.normal(10.0, 3.0), 1.0, 20.0),
        ),
        defense: Caller::new(
            clamp(rng.normal(10.0, 3.0), 1.0, 20.0),
            clamp(rng.normal(10.0, 3.0), 1.0, 20.0),
            clamp(rng.normal(10.0, 3.0), 1.0, 20.0),
        ),
        game_management: clamp(rng.normal(10.0, 3.0), 1.0, 20.0),
        strictness: clamp(rng.normal(10.0, 3.0), 1.0, 20.0),
        off_scheme: scheme,
        front: if rng.chance(0.35) { Front::ThreeFour } else { Front::FourThree },
        shell: *rng.pick(&[Shell::Cover3, Shell::Quarters, Shell::ManPressure, Shell::Cover2]),
        playbook: 1.0,
        // From its own stream so the draws above keep their values.
        sportsmanship: clamp(Rng::new(seed ^ 0x5BD1_E995).normal(10.0, 3.5), 1.0, 20.0),
        nickel: clamp(Rng::new(seed ^ 0x2545_F491).normal(0.88, 0.08), 0.55, 0.99),
        year: 2026,
        film_study: 10.0,
        install: absorb(aware).clamp(0.35, 1.0),
        style: Default::default(),
        guide: Default::default(),
        // Own streams, so the draws above keep their values.
        motivation: clamp(Rng::new(seed ^ 0x40C1_7A11).normal(10.0, 3.0), 1.0, 20.0),
        temper: clamp(Rng::new(seed ^ 0x7E3B_0D21).normal(10.0, 3.5), 1.0, 20.0),
    };
    // The head coach's guidelines (update 2, G2), from their own stream so
    // the draws above keep their values.
    let mut staff = staff;
    staff.guide = super::guide::draw(
        &mut Rng::new(seed ^ 0x6A1D_E5E7),
        level != Level::NFL,
        staff.off_scheme,
        staff.shell,
        staff.defense.aggressiveness,
    );
    GameTeam::new(name.to_string(), players, staff)
}

/// A player read from a world trial's export (`tools/rosters/`): his
/// position, ratings and tendencies. What the export leaves out (his body
/// and hidden traits) comes from a prospect drawn at that position.
pub struct RatedPlayer {
    pub position: Position,
    pub attrs: crate::attributes::Attributes,
    pub tend: [f64; crate::tendencies::TEND_COUNT],
}

/// A team built from exported players, for checking the engine on a
/// world's own rosters without running the world (trial tooling).
pub fn team_from_ratings(name: &str, level: Level, list: &[RatedPlayer], scheme: OffScheme, seed: u32) -> GameTeam {
    let mut rng = Rng::new(seed);
    let mut id = seed.wrapping_mul(1000) & 0x7FFF_FFFF;
    let mut raw = Vec::new();
    for (k, rp) in list.iter().enumerate() {
        id += 1;
        let target = overall_for(rp.position, &rp.attrs);
        let mut p = generate_prospect(
            &mut rng,
            ProspectOptions { id: Some(id), position: Some(rp.position), age: Some(14), talent_z: Some(z_for(target)) },
        );
        p.age = 19 + (k % 4) as u32;
        p.attributes = rp.attrs;
        p.ceilings = rp.attrs;
        p.tend.v = rp.tend.map(|x| x as f32);
        p.tend.was = rp.tend.map(|x| x as f32);
        raw.push((p, 0.86, 1.0 + (k % 3) as f64));
    }
    finish_team(name, level, raw, &mut rng, seed, scheme)
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
            // High school teams differ far more than college or pro ones (a
            // powerhouse against a small town's team): real high school
            // winners average a 26-point margin (MaxPreps, 5.2 million
            // games). A team's quality spreads 6 points there (A7).
            let q = rng.normal(0.0, if level == Level::HS { 6.0 } else { 2.5 });
            let s = *rng.pick(schemes);
            synthetic_team(&format!("Team {}", k + 1), level, q, s, seed.wrapping_add(k as u32 * 7919))
        })
        .collect()
}
