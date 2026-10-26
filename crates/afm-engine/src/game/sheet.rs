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
/// minutes or less to play, the last two included (one window for the
/// tempo and the calls).
pub fn four_minute(sit: &Situation) -> bool {
    sit.quarter >= 4 && (1..=16).contains(&sit.score_diff) && sit.game_left <= 300.0
}

/// The boxes for a snap, best first: the two-minute drill when the offense
/// must hurry, the four-minute offense when it leads late.
pub fn boxes(sit: &Situation, two_point: bool) -> Vec<u8> {
    if two_point {
        return vec![TWO_POINT, GOAL_LINE];
    }
    let mut v = Vec::new();
    if sit.hurry() {
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
    /// The snap's kind is set before the sheet (your four-minute plan, a
    /// practice period): the sheet calls only that kind.
    pub forced: bool,
}

/// A pick down a box's list: the first choice most often (each next one
/// about 0.6 as often).
fn ordered<T>(list: &[T], rng: &mut Rng) -> usize {
    let w: Vec<(usize, f64)> = (0..list.len()).map(|i| (i, 0.6f64.powi(i as i32))).collect();
    rng.weighted(&w)
}

/// The plays of a box for this hash: the hash's own list first, then the
/// box's plays for either hash. A ball in the middle of the field takes
/// both hashes' lists, first choices first, then the plays for either.
fn box_list<T: Copy + PartialEq>(sheet: &[(u8, Vec<T>)], k: u8, hash: u8) -> Vec<T> {
    let of =
        |key: u8| -> Vec<T> { sheet.iter().filter(|(s, _)| *s == key).flat_map(|(_, l)| l.iter().copied()).collect() };
    let mut out: Vec<T> = Vec::new();
    let mut push = |x: T| {
        if !out.contains(&x) {
            out.push(x);
        }
    };
    if hash > 0 {
        of(key(k, hash)).into_iter().for_each(&mut push);
    } else {
        let (a, b) = (of(key(k, 1)), of(key(k, 2)));
        for i in 0..a.len().max(b.len()) {
            a.get(i).into_iter().chain(b.get(i)).for_each(|&x| push(x));
        }
    }
    of(k).into_iter().for_each(&mut push);
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
    let normal = sit.down <= 2
        && sit.yardline_100 > 20.0
        && sit.yardline_100 < 90.0
        && !ctx.hurry
        && !ctx.two_point
        && !ctx.forced;
    if normal && ctx.scripted < script.len() && ctx.scripted < 20 {
        return Some((to_call(own, script[ctx.scripted], "the opening script"), true));
    }
    // A shot play (G9): on second and short in the middle of the field, now
    // and then on first down there; never when the clock or the score says
    // run it (the four-minute offense, a decided game, a run called first).
    let shots = box_list(sheet, SHOTS, ctx.hash);
    let quiet = ctx.hurry || ctx.two_point || ctx.forced || four_minute(sit) || sit.ease > 0.0;
    if !shots.is_empty() && !quiet && (35.0..=65.0).contains(&sit.yardline_100) {
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
        // The kind the play caller read the snap as: a box without it
        // leaves his own call standing (the two-point list is called
        // whole).
        let fit: Vec<SheetCall> = list.iter().copied().filter(|x| x.3.is_run() == own.kind.is_run()).collect();
        let pool = if k == TWO_POINT && !ctx.forced { list } else { fit };
        if pool.is_empty() {
            return None;
        }
        let i = ordered(&pool, rng);
        return Some((to_call(own, pool[i], "on the call sheet"), false));
    }
    None
}

/// What the defense's sheet call needs to know beyond the situation.
#[derive(Clone, Copy, Debug, Default)]
pub struct DefCtx {
    pub wet: bool,
    pub two_point: bool,
    /// Scripted calls already used this game.
    pub scripted: usize,
    /// The coordinator read the call off a tip (a former player with our
    /// words): his own call stands.
    pub tipped: bool,
}

/// The defense's call from its sheet (G8), inside the coordinator's own
/// decision: he has read the snap (man or zone, a rush of four or more)
/// and the sheet says which of the week's calls of that kind; a box
/// without that kind leaves his call standing. His pressure package comes
/// out on passing downs when he was sending pressure anyway. A decided
/// game or a tipped call is his own. Returns the call and whether it was
/// the script's.
pub fn def_call(prep: &Prep, sit: &Situation, own: &DefCall, ctx: DefCtx, rng: &mut Rng) -> Option<(DefCall, bool)> {
    let sheet = if ctx.wet && !prep.dwet.is_empty() { &prep.dwet } else { &prep.dsheet };
    if sheet.is_empty() || ctx.tipped || sit.def_ease > 0.0 || !rng.chance(prep.dstick) {
        return None;
    }
    // The defense's opening script (G11): its first calls in order on
    // normal early downs, each testing something on their offense.
    let script = box_list(sheet, SCRIPT, 0);
    let normal = sit.down <= 2 && sit.yardline_100 > 20.0 && sit.yardline_100 < 90.0 && !ctx.two_point;
    let passing = (sit.down == 3 && sit.togo >= 4.0) || (sit.down == 2 && sit.togo >= 7.0);
    let blitz = own.pressure.extra() > 0;
    let like = |x: &DefSheetCall| x.0.is_man() == own.coverage.is_man() && (x.1.extra() > 0) == blitz;
    let (pick, scripted): (Option<DefSheetCall>, bool) = if normal && ctx.scripted < script.len() && ctx.scripted < 12 {
        (Some(script[ctx.scripted]), true)
    } else {
        let pressures: Vec<DefSheetCall> = box_list(sheet, SHOTS, 0).into_iter().filter(|x| x.1.extra() > 0).collect();
        let package: Vec<DefSheetCall> =
            pressures.iter().copied().filter(|x| x.0.is_man() == own.coverage.is_man()).collect();
        let package = if package.is_empty() { pressures } else { package };
        if passing && blitz && !package.is_empty() {
            (Some(package[ordered(&package, rng)]), false)
        } else {
            let found = boxes(sit, ctx.two_point).into_iter().find_map(|k| {
                let list = box_list(sheet, k, 0);
                if list.is_empty() {
                    return None;
                }
                // The box answers the snap; his read decides the kind.
                let fit: Vec<DefSheetCall> = list.into_iter().filter(like).collect();
                Some((!fit.is_empty()).then(|| fit[ordered(&fit, rng)]))
            });
            (found.flatten(), false)
        }
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
    Some((c, scripted))
}
