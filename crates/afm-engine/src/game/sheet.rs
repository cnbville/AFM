//! The call sheet on game day (section 6b; update 2, lane 5: G7-G13). The
//! sheet's boxes are the situations a play caller plans for: the opening
//! script, the downs and distances, the red zone and goal line, backed up,
//! the four-minute offense, the two-minute drill, shot plays and the
//! two-point list. Each box lists its plays in order of preference, and a
//! play can be marked for the left or the right hash. The script is called
//! in its order on early downs; in bad weather the bad-weather version is
//! called. The defense has its own sheet: coverage and pressure by
//! situation.
//!
//! The play caller still calls off the card when he reads it differently
//! (`Prep::stick`). Sizes are Claude's picks (proposed, not confirmed).

use super::calling::Situation;
use super::playbook::{Coverage, DefCall, DefPersonnel, OffCall, Pressure, Stunt};
use super::roster::{DefSheetCall, Prep, SheetCall};
use crate::rng::Rng;

/// The boxes, in the order of `training::seat::SHEET`.
pub const SCRIPT: u8 = 0;
pub const FIRST: u8 = 1;
pub const SECOND_LONG: u8 = 2;
pub const SECOND_SHORT: u8 = 3;
pub const THIRD_SHORT: u8 = 4;
pub const THIRD_MEDIUM: u8 = 5;
pub const THIRD_LONG: u8 = 6;
pub const RED_ZONE: u8 = 7;
pub const GOAL_LINE: u8 = 8;
pub const TWO_MINUTE: u8 = 9;
pub const SECOND_MEDIUM: u8 = 10;
pub const BACKED_UP: u8 = 11;
pub const FOUR_MINUTE: u8 = 12;
pub const SHOTS: u8 = 13;
pub const TWO_POINT: u8 = 14;
pub const BOXES: u8 = 15;

/// A sheet key carries its hash in the top bits: 0 either hash, 1 the left
/// hash, 2 the right.
pub fn key(k: u8, hash: u8) -> u8 {
    k | (hash.min(2) << 5)
}

pub fn split(key: u8) -> (u8, u8) {
    (key & 31, key >> 5)
}

/// The hash the ball sits on as the offense looks downfield: 1 left, 2
/// right, 0 in the middle.
pub fn hash_of(ball_y: f64) -> u8 {
    let mid = super::geom::mid();
    if (ball_y - mid).abs() < 1.5 {
        0
    } else if ball_y > mid {
        1
    } else {
        2
    }
}

/// The four-minute offense: ahead by one to two scores with about four
/// minutes or less to play.
pub fn four_minute(sit: &Situation) -> bool {
    sit.quarter >= 4 && (1..=16).contains(&sit.score_diff) && sit.game_left <= 300.0 && sit.half_left > 120.0
}

/// The boxes for a snap, best first.
pub fn boxes(sit: &Situation, two_point: bool) -> Vec<u8> {
    if two_point {
        return vec![TWO_POINT, GOAL_LINE];
    }
    let mut v = Vec::new();
    if sit.half_left <= 120.0 {
        v.push(TWO_MINUTE);
    }
    if four_minute(sit) {
        v.push(FOUR_MINUTE);
    }
    if sit.yardline_100 >= 90.0 {
        v.push(BACKED_UP);
    }
    if sit.yardline_100 <= 3.0 {
        v.push(GOAL_LINE);
    }
    if sit.yardline_100 <= 20.0 {
        v.push(RED_ZONE);
    }
    match sit.down {
        1 => v.push(FIRST),
        2 if sit.togo >= 7.0 => v.push(SECOND_LONG),
        // An older sheet has one "second and short" box for 1-6 yards.
        2 if sit.togo >= 4.0 => v.extend([SECOND_MEDIUM, SECOND_SHORT]),
        2 => v.push(SECOND_SHORT),
        _ if sit.togo <= 3.0 => v.push(THIRD_SHORT),
        _ if sit.togo <= 6.0 => v.push(THIRD_MEDIUM),
        _ => v.push(THIRD_LONG),
    }
    v
}

/// What the offense's sheet call needs to know beyond the situation.
#[derive(Clone, Copy, Debug, Default)]
pub struct Ctx {
    /// Script plays already called this game.
    pub scripted: usize,
    pub hash: u8,
    /// Rain, snow or a strong wind: the bad-weather sheet.
    pub wet: bool,
    pub two_point: bool,
    pub hurry: bool,
}

/// A pick down a box's list: the first choice most often (each next one
/// about 0.6 as often).
fn ordered<T>(list: &[T], rng: &mut Rng) -> usize {
    let w: Vec<(usize, f64)> = (0..list.len()).map(|i| (i, 0.6f64.powi(i as i32))).collect();
    rng.weighted(&w)
}

/// The plays of a box for this hash: the hash's own list first, then the
/// box's plays for either hash.
fn box_list<T: Copy>(sheet: &[(u8, Vec<T>)], k: u8, hash: u8) -> Vec<T> {
    let mut out: Vec<T> = Vec::new();
    if hash > 0 {
        out.extend(sheet.iter().filter(|(s, _)| *s == key(k, hash)).flat_map(|(_, l)| l.iter().copied()));
    }
    out.extend(sheet.iter().filter(|(s, _)| *s == k).flat_map(|(_, l)| l.iter().copied()));
    out
}

fn to_call(own: &OffCall, x: SheetCall, why: &'static str) -> OffCall {
    let (pers, back, set, kind) = x;
    let mut f = own.formation;
    f.personnel = pers;
    f.backfield = back;
    f.set = set;
    let mut c = OffCall::new(f, kind, own.dir, own.tempo);
    c.why = why;
    c
}

/// The offense's call from the sheet for this snap (None: the play
/// caller's own call stands), and whether it was the script's.
pub fn off_call(prep: &Prep, sit: &Situation, own: &OffCall, ctx: Ctx, rng: &mut Rng) -> Option<(OffCall, bool)> {
    let sheet = if ctx.wet && !prep.wet.is_empty() { &prep.wet } else { &prep.sheet };
    if sheet.is_empty() || !rng.chance(prep.stick) {
        return None;
    }
    // The opening script (G11), in its order, on normal early downs.
    let script = box_list(sheet, SCRIPT, 0);
    let normal = sit.down <= 2 && sit.yardline_100 > 20.0 && sit.yardline_100 < 90.0 && !ctx.hurry && !ctx.two_point;
    if normal && ctx.scripted < script.len() && ctx.scripted < 20 {
        return Some((to_call(own, script[ctx.scripted], "the opening script"), true));
    }
    // A shot play (G9): on second and short in the middle of the field, now
    // and then on first down there.
    let shots = box_list(sheet, SHOTS, ctx.hash);
    if !shots.is_empty() && !ctx.hurry && !ctx.two_point && (35.0..=65.0).contains(&sit.yardline_100) {
        let p = match sit.down {
            1 => 0.08,
            2 if sit.togo <= 3.0 => 0.35,
            _ => 0.0,
        };
        if p > 0.0 && rng.chance(p) {
            let i = ordered(&shots, rng);
            return Some((to_call(own, shots[i], "a shot play off the call sheet"), false));
        }
    }
    for k in boxes(sit, ctx.two_point) {
        let list = box_list(sheet, k, ctx.hash);
        if list.is_empty() {
            continue;
        }
        // The kind the play caller read the snap as, if the box has it.
        let fit: Vec<SheetCall> = list.iter().copied().filter(|x| x.3.is_run() == own.kind.is_run()).collect();
        let pool = if fit.is_empty() || k == TWO_POINT { list } else { fit };
        let i = ordered(&pool, rng);
        return Some((to_call(own, pool[i], "on the call sheet"), false));
    }
    None
}

/// The defense's call from its sheet (G8): the coverage and pressure for
/// the situation, the front and personnel as the coordinator set them.
#[allow(clippy::too_many_arguments)]
pub fn def_call(
    prep: &Prep,
    sit: &Situation,
    own: &DefCall,
    wet: bool,
    two_point: bool,
    scripted: usize,
    rng: &mut Rng,
) -> Option<DefCall> {
    let sheet = if wet && !prep.dwet.is_empty() { &prep.dwet } else { &prep.dsheet };
    if sheet.is_empty() || !rng.chance(prep.dstick) {
        return None;
    }
    // The defense's opening script (G11): its first calls in order on
    // normal early downs, each testing something on their offense.
    let script = box_list(sheet, SCRIPT, 0);
    let normal = sit.down <= 2 && sit.yardline_100 > 20.0 && sit.yardline_100 < 90.0 && !two_point;
    // The pressure package on passing downs.
    let passing = (sit.down == 3 && sit.togo >= 4.0) || (sit.down == 2 && sit.togo >= 7.0);
    let pressures = box_list(sheet, SHOTS, 0);
    let pick: Option<DefSheetCall> = if normal && scripted < script.len() && scripted < 12 {
        Some(script[scripted])
    } else if passing && !pressures.is_empty() && rng.chance(0.3) {
        Some(pressures[ordered(&pressures, rng)])
    } else {
        boxes(sit, two_point).into_iter().find_map(|k| {
            let list = box_list(sheet, k, 0);
            (!list.is_empty()).then(|| list[ordered(&list, rng)])
        })
    };
    let (coverage, pressure) = pick?;
    let mut c = *own;
    c.coverage = coverage;
    c.pressure = if coverage == Coverage::Cover0 { Pressure::DoubleA } else { pressure };
    if c.personnel == DefPersonnel::Dime && c.pressure == Pressure::DoubleA {
        c.pressure = Pressure::DbBlitz;
    }
    if c.pressure.extra() > 0 {
        c.stunt = Stunt::None;
    }
    if c.coverage.is_man() {
        c.spy = false;
    }
    Some(c)
}
