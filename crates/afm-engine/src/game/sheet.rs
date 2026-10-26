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
//! The sheet is the week's menu, not a script for every snap: the play
//! caller reads the snap as he always does and calls his play from the
//! box when it is there; else he thinks the snap over until his call is
//! on it; else he takes the box's play most like his (the same kind of
//! throw or run), the coordinator's order breaking ties. He still calls
//! off the card when he reads it differently (`Card::stick`).
//!
//! Every staff drafts one: your coordinators on plan day, every other
//! team's at kickoff from the same film and the same draft
//! (`world::plans::kickoff_card`). The sizes and counts are proposed, not
//! confirmed.

use std::sync::atomic::{AtomicU8, Ordering};

use super::calling::Situation;
use super::playbook::{Coverage, DefCall, DefPersonnel, OffCall, PlayKind, Pressure, Stunt, pass_concept};
use super::roster::{DefSheetCall, Prep, SheetCall};
use crate::rng::Rng;

/// Which teams draft a sheet at kickoff when the week gave them none: 3
/// college and NFL teams (the game: a high school staff's draft for every
/// high school game cost a season on the engine 43% more time), 1 every
/// team, 0 none, 2 the home team only (engine checks: `AFM_SHEET` in
/// `match_check` and `memory_check`).
pub static KICKOFF: AtomicU8 = AtomicU8::new(3);

/// Whether the team on this side (0 home) drafts at kickoff.
pub fn drafts_at_kickoff(side: usize, level: crate::player::Level) -> bool {
    match KICKOFF.load(Ordering::Relaxed) {
        0 => false,
        2 => side == 0,
        3 => level != crate::player::Level::HS,
        _ => true,
    }
}

/// A side's call sheets for one game, offense and defense, dry and wet,
/// and how often each play caller sticks to the card.
#[derive(Clone, Debug, Default)]
pub struct Card {
    pub sheet: Vec<(u8, Vec<SheetCall>)>,
    pub wet: Vec<(u8, Vec<SheetCall>)>,
    pub stick: f64,
    pub dsheet: Vec<(u8, Vec<DefSheetCall>)>,
    pub dwet: Vec<(u8, Vec<DefSheetCall>)>,
    pub dstick: f64,
}

impl Card {
    /// The sheets your week put on the game team.
    pub fn of(prep: &Prep) -> Card {
        Card {
            sheet: prep.sheet.clone(),
            wet: prep.wet.clone(),
            stick: prep.stick,
            dsheet: prep.dsheet.clone(),
            dwet: prep.dwet.clone(),
            dstick: prep.dstick,
        }
    }
    pub fn is_empty(&self) -> bool {
        self.sheet.is_empty() && self.dsheet.is_empty()
    }
}

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

/// The coordinator's order in a box: each next play about 0.85 as likely
/// as the one above it among the plays that fit the snap (proposed, not
/// confirmed).
const ORDER: f64 = 0.85;

/// A pick down a list by the coordinator's order alone.
fn ordered<T>(list: &[T], rng: &mut Rng) -> usize {
    let w: Vec<(usize, f64)> = (0..list.len()).map(|i| (i, ORDER.powi(i as i32))).collect();
    rng.weighted(&w)
}

/// A pick among the plays closest to the play caller's own call: the
/// highest `level` any play reaches decides, the order within it.
fn closest<T>(list: &[T], level: impl Fn(&T) -> u8, rng: &mut Rng) -> usize {
    let best = list.iter().map(&level).max().unwrap_or(0);
    let w: Vec<(usize, f64)> = list
        .iter()
        .enumerate()
        .filter(|(_, x)| level(x) == best)
        .enumerate()
        .map(|(rank, (i, _))| (i, ORDER.powi(rank as i32)))
        .collect();
    rng.weighted(&w)
}

/// A run, a run with a throw tagged on (an RPO) or a throw.
fn runs(k: PlayKind) -> bool {
    matches!(k, PlayKind::Run(_) | PlayKind::Rpo(..))
}

/// How close a sheet play is to the play caller's own call: 3 the same
/// play, 2 the same kind of play (a throw at the same depth, with or
/// without play action; a run of the same family), 1 a run for a run or a
/// throw for a throw, 0 anything else.
fn likeness(x: &SheetCall, own: &OffCall) -> u8 {
    match (x.3, own.kind) {
        (PlayKind::Pass(a), PlayKind::Pass(b)) => {
            let (ca, cb) = (pass_concept(a), pass_concept(b));
            if a == b {
                3
            } else if ca.depth == cb.depth && ca.play_action == cb.play_action {
                2
            } else {
                1
            }
        }
        (PlayKind::Run(a), PlayKind::Run(b) | PlayKind::Rpo(b, _)) => {
            if a == b {
                3
            } else if a.family() == b.family() {
                2
            } else {
                1
            }
        }
        _ => 0,
    }
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

/// How many times a play caller thinks the snap over before he settles
/// for the menu's play most like his own (proposed, not confirmed).
const REREAD: usize = 8;

/// Box calls by how they were found (engine checks, `match_check`): his
/// own play on the box, a second thought on it, the closest play, for the
/// offense then the defense.
pub static FOUND: [std::sync::atomic::AtomicU64; 6] = [const { std::sync::atomic::AtomicU64::new(0) }; 6];

fn found(k: usize) {
    FOUND[k].fetch_add(1, Ordering::Relaxed);
}

/// The sheet's play is this play: the same pass concept, or the same run
/// (with or without a throw tagged on).
fn same_play(x: PlayKind, k: PlayKind) -> bool {
    match (x, k) {
        (PlayKind::Pass(a), PlayKind::Pass(b)) => a == b,
        (PlayKind::Run(a), PlayKind::Run(b) | PlayKind::Rpo(b, _)) => a == b,
        _ => false,
    }
}

fn to_call(own: &OffCall, x: SheetCall, why: &'static str) -> OffCall {
    let (pers, back, set, kind) = x;
    let mut f = own.formation;
    f.personnel = pers;
    f.backfield = back;
    f.set = set;
    // His own run with a throw tagged on stays one when the sheet's run is
    // the same run.
    let kind = match (kind, own.kind) {
        (PlayKind::Run(a), PlayKind::Rpo(b, c)) if a == b => PlayKind::Rpo(b, c),
        _ => kind,
    };
    let mut c = OffCall::new(f, kind, own.dir, own.tempo);
    // The quarterback's kill call stays with the call: the other kind of
    // play, his to pick at the line.
    c.alt = own.alt.filter(|a| runs(*a) != runs(kind));
    c.why = why;
    c
}

/// The offense's call from the sheet for this snap (None: the play
/// caller's own call stands), and whether it was the script's. `redraw`
/// is the play caller thinking the snap over (his own call again).
pub fn off_call(
    card: &Card,
    sit: &Situation,
    own: &OffCall,
    ctx: Ctx,
    rng: &mut Rng,
    mut redraw: impl FnMut(&mut Rng) -> OffCall,
) -> Option<(OffCall, bool)> {
    let sheet = if ctx.wet && !card.wet.is_empty() { &card.wet } else { &card.sheet };
    if sheet.is_empty() || !rng.chance(card.stick) {
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
        if p > 0.0 && !runs(own.kind) && rng.chance(p) {
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
        let whole = k == TWO_POINT && !ctx.forced;
        let fit: Vec<SheetCall> = list.iter().copied().filter(|x| runs(x.3) == runs(own.kind)).collect();
        let pool = if whole { list } else { fit };
        if pool.is_empty() {
            return None;
        }
        // The menu, his read: his own play when it is on the box; else he
        // thinks the snap over the way he calls any snap until his call is
        // on it; else the box's play most like his own, the order counting
        // within the closest.
        let on = |c: &OffCall| (whole || runs(c.kind) == runs(own.kind)) && pool.iter().any(|x| same_play(x.3, c.kind));
        let mine = |mut c: OffCall| {
            c.why = "on the call sheet";
            Some((c, false))
        };
        if on(own) {
            found(0);
            return mine(*own);
        }
        for _ in 0..REREAD {
            let c = redraw(rng);
            if on(&c) {
                found(1);
                return mine(c);
            }
        }
        found(2);
        let i = closest(&pool, |x| likeness(x, own), rng);
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
/// decision: his own call when the box has it; else he thinks the snap
/// over (`redraw`) until his call is on it; else the box's call of his
/// kind (man or zone, a rush of four or more) closest to his, and a box
/// without his kind leaves his call standing. His pressure package comes
/// out on passing downs when he was sending pressure anyway. A decided
/// game or a tipped call is his own. Returns the call and whether it was
/// the script's.
pub fn def_call(
    card: &Card,
    sit: &Situation,
    own: &DefCall,
    ctx: DefCtx,
    rng: &mut Rng,
    mut redraw: impl FnMut(&mut Rng) -> DefCall,
) -> Option<(DefCall, bool)> {
    let sheet = if ctx.wet && !card.dwet.is_empty() { &card.dwet } else { &card.dsheet };
    if sheet.is_empty() || ctx.tipped || sit.def_ease > 0.0 || !rng.chance(card.dstick) {
        return None;
    }
    // The defense's opening script (G11): its first calls in order on
    // normal early downs, each testing something on their offense.
    let script = box_list(sheet, SCRIPT, 0);
    let normal = sit.down <= 2 && sit.yardline_100 > 20.0 && sit.yardline_100 < 90.0 && !ctx.two_point;
    let passing = (sit.down == 3 && sit.togo >= 4.0) || (sit.down == 2 && sit.togo >= 7.0);
    let blitz = own.pressure.extra() > 0;
    let like = |x: &DefSheetCall| x.0.is_man() == own.coverage.is_man() && (x.1.extra() > 0) == blitz;
    // His own call when the box has it, else the same coverage, else the
    // same shell.
    let near = |x: &DefSheetCall| {
        if x.0 == own.coverage && x.1 == own.pressure {
            3
        } else if x.0 == own.coverage {
            2
        } else if x.0.high() == own.coverage.high() {
            1
        } else {
            0
        }
    };
    let (pick, scripted): (Option<DefSheetCall>, bool) = if normal && ctx.scripted < script.len() && ctx.scripted < 12 {
        (Some(script[ctx.scripted]), true)
    } else {
        let pressures: Vec<DefSheetCall> = box_list(sheet, SHOTS, 0).into_iter().filter(|x| x.1.extra() > 0).collect();
        let package: Vec<DefSheetCall> =
            pressures.iter().copied().filter(|x| x.0.is_man() == own.coverage.is_man()).collect();
        let package = if package.is_empty() { pressures } else { package };
        let mut pool: Vec<DefSheetCall> = if passing && blitz && !package.is_empty() {
            package.clone()
        } else {
            boxes(sit, ctx.two_point).into_iter().map(|k| box_list(sheet, k, 0)).find(|l| !l.is_empty())?
        };
        // His pressures are on the sheet for every down: a box lists his
        // base calls, the package his pressures.
        if blitz {
            pool.extend(package.iter().copied().filter(|x| !pool.contains(x)).collect::<Vec<_>>());
        }
        // The menu, his read (as the offense's).
        let on = |c: &DefCall| pool.contains(&(c.coverage, c.pressure));
        if on(own) {
            found(3);
            return Some((*own, false));
        }
        for _ in 0..REREAD {
            let c = redraw(rng);
            if (c.pressure.extra() > 0) == blitz && on(&c) {
                found(4);
                return Some((c, false));
            }
        }
        found(5);
        // The box answers the snap; his read decides the kind.
        let fit: Vec<DefSheetCall> = pool.into_iter().filter(like).collect();
        ((!fit.is_empty()).then(|| fit[closest(&fit, near, rng)]), false)
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
