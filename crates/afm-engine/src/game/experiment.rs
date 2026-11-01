//! Prototype switches for the dynasty proof (`dynasty_proof`), off by
//! default so every result stays the same. Rough stand-ins for proposals
//! Raichel hasn't answered yet; nothing here is a design decision.

use super::roster::{GameTeam, OffScheme, Staff};
use crate::attributes::ATTRIBUTE_COUNT;
use crate::positions::Position;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

static POSITION_VALUES: AtomicBool = AtomicBool::new(false);
static SAME_SCHEMES: AtomicBool = AtomicBool::new(false);

/// Rough position values (trial 4 proposal 1): in each game, every player's
/// attributes move toward the two teams' average at his position, so a
/// rating point there is worth the middle of the proposed band instead of
/// what the engine pays now. Lines and quarterbacks are left alone.
pub fn set_position_values(on: bool) {
    POSITION_VALUES.store(on, Ordering::Relaxed);
}

/// Rough "no scheme wins by itself" (trial 4 proposal 2, at its limit):
/// every team plays the same schemes.
pub fn set_same_schemes(on: bool) {
    SAME_SCHEMES.store(on, Ordering::Relaxed);
}

/// Every switch and whether it is on (the world log's meta, R2).
pub fn switches() -> [(&'static str, bool); 2] {
    [
        ("position_values", POSITION_VALUES.load(Ordering::Relaxed)),
        ("same_schemes", SAME_SCHEMES.load(Ordering::Relaxed)),
    ]
}

/// Share of the quarterback's value per rating point the engine pays now
/// (`docs/match-engine.md`) against the middle of the proposed band.
pub fn shrink(pos: Position) -> f64 {
    let (now, band): (f64, f64) = match pos {
        Position::RB => (0.74, 0.10),
        Position::EDGE => (0.60, 0.225),
        Position::S => (0.45, 0.13),
        Position::WR => (0.44, 0.225),
        Position::LB => (0.42, 0.10),
        Position::DT => (0.41, 0.175),
        Position::TE => (0.39, 0.125),
        Position::CB => (0.37, 0.185),
        _ => (1.0, 1.0),
    };
    band / now
}

/// The two teams as the switches change them, or `None` when all are off.
pub fn adjust(home: &GameTeam, away: &GameTeam) -> Option<(GameTeam, GameTeam)> {
    let pos = POSITION_VALUES.load(Ordering::Relaxed);
    let same = SAME_SCHEMES.load(Ordering::Relaxed);
    if !pos && !same {
        return None;
    }
    let mut t = [home.clone(), away.clone()];
    if pos {
        let mut sum: HashMap<Position, ([f64; ATTRIBUTE_COUNT], f64)> = HashMap::new();
        for p in t.iter().flat_map(|t| &t.players) {
            let e = sum.entry(p.position).or_insert(([0.0; ATTRIBUTE_COUNT], 0.0));
            for k in 0..ATTRIBUTE_COUNT {
                e.0[k] += p.attrs.at(k);
            }
            e.1 += 1.0;
        }
        for p in t.iter_mut().flat_map(|t| &mut t.players) {
            let f = shrink(p.position);
            let (s, n) = sum[&p.position];
            for k in 0..ATTRIBUTE_COUNT {
                let m = s[k] / n;
                p.attrs.set_at(k, m + f * (p.attrs.at(k) - m));
            }
        }
    }
    if same {
        let n = Staff::average(OffScheme::WestCoast);
        for x in &mut t {
            x.staff.off_scheme = n.off_scheme;
            x.staff.front = n.front;
            x.staff.shell = n.shell;
        }
    }
    let [h, a] = t;
    Some((h, a))
}
