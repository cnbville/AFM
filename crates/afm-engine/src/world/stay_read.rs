//! The day's read (playtest bug 15, Cain 2026-11-13): on the day a college
//! player decides on the portal (a window's day, and at your program the
//! day after his talk with you), he reads his whole situation again, not
//! only the lean his fall camp role set. Cain: "it should look at
//! everything ... it's highly illogical that a young player who's in a
//! starting position and promised playing time decides to leave."
//!
//! The parts, each agreed with Cain on 2026-11-13 (sizes proposed, not
//! confirmed):
//! - his own guess of his place next fall: who ahead of him at his spot
//!   is leaving, against his view of himself (never the depth chart);
//! - after a head coach change, the old staff's role counts half and his
//!   own guess fills the other half (he has had no practice under the new
//!   man before a January window);
//! - your word on his role (an open playing-time promise) replaces both,
//!   by how much he believes it ([`promise_belief`], the hook the promises
//!   lane P1 fills), and holds him on its own;
//! - a young starter with two seasons or more left;
//! - a promise his head coach broke;
//! - trust in his current head coach, his mood today;
//! - his NIL pay against the going rate at his level: an underpaid man
//!   leans out, a well-paid one who wants out may stay for the money.
//!
//! The read is a factor on his lean, held to [`READ_MIN`]-[`READ_MAX`]: a
//! strong hold, never a lock. It replaces the previous day's read, so it
//! counts once.

use std::collections::HashMap;

use crate::calendar::Date;
use crate::player::PlayingTime;

use super::data::Division;
use super::folk::talks::{PromiseKind, PromiseTo};
use super::{Stage, World};

/// The read's floor and ceiling (agreed: a strong hold, never a lock).
pub(crate) const READ_MIN: f64 = 0.35;
pub(crate) const READ_MAX: f64 = 2.0;

/// The lean event that takes the previous day's read back out, so a new
/// read replaces it. Screens skip it ([`is_read`]).
pub const READ_UNDO: &str = "his earlier read, replaced";

const OLD_ROLE_HALF: &str = "the old staff's role counts half under a new head coach";
const YOUR_WORD: &str = "his head coach's word on his role";
const SEES_START: &str = "sees himself starting next fall";
const SEES_NEXT: &str = "sees himself next in line next fall";
const YOUNG_STARTER: &str = "a young starter with seasons left";
const PROMISE_OPEN: &str = "a promise from his head coach";
const PROMISE_BROKEN: &str = "his head coach broke a promise";
const TRUSTS: &str = "trusts his head coach";
const DOUBTS: &str = "doesn't trust his head coach";
const HAPPY: &str = "happy where he is";
const UNHAPPY: &str = "unhappy where he is";
const PAID: &str = "well paid where he is";
const UNDERPAID: &str = "underpaid where he is";
const LIMIT: &str = "the day's read held at its limit";
pub(crate) const LEVEL: &str = "against every player's read at his level";

/// Every lean event the day's read writes.
const READS: [&str; 16] = [
    READ_UNDO,
    OLD_ROLE_HALF,
    YOUR_WORD,
    SEES_START,
    SEES_NEXT,
    YOUNG_STARTER,
    PROMISE_OPEN,
    PROMISE_BROKEN,
    TRUSTS,
    DOUBTS,
    HAPPY,
    UNHAPPY,
    PAID,
    UNDERPAID,
    LIMIT,
    LEVEL,
];

/// Whether a lean event belongs to the day's read.
pub(crate) fn is_read(why: &str) -> bool {
    READS.contains(&why)
}

/// His own guess (3a): sees himself starting next fall ×0.6, next in line
/// ×0.85, with no head coach change (with one, the guess fills the open
/// half of his role instead).
const START_HOLD: f64 = 0.6;
const NEXT_HOLD: f64 = 0.85;
/// A young starter (item 4): two seasons or more left.
const YOUNG_HOLD: f64 = 0.7;
/// An open playing-time promise from his head coach (item 5), times his belief.
const PROMISE_HOLD: f64 = 0.4;
/// A playing-time promise his head coach broke (item 5).
const BROKEN_PUSH: f64 = 1.4;
/// NIL (item 10): his money want, ×6 (six wants sum to 1), held to 0-2.
const PAID_HOLD: f64 = 0.15;
const UNDERPAID_PUSH: f64 = 0.2;

/// His place next fall as he guesses it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Next {
    Start,
    InLine,
    Behind,
}

impl Next {
    fn role(self) -> PlayingTime {
        match self {
            Next::Start => PlayingTime::Starter,
            Next::InLine => PlayingTime::Rotation,
            Next::Behind => PlayingTime::Backup,
        }
    }
}

/// What the read needs about the world, worked out once a day.
pub(crate) struct Ctx {
    /// The season the decision belongs to.
    label: i32,
    /// Players coming back next fall, by (program, position): (id, overall).
    returning: HashMap<(usize, usize), Vec<(u32, f64)>>,
    /// The median NIL pay by (level, position, role): the going rate.
    nil_median: HashMap<(u8, usize, u8), f64>,
    nil_era: bool,
    /// Your program, when you are its head coach: the promises in
    /// `folk.promises` are yours there.
    mine: Option<usize>,
    /// The read decides who leaves, not how many (proposed, not
    /// confirmed): each division's reads are divided by their mean,
    /// weighted by the lean they act on.
    level: HashMap<Division, f64>,
}

/// The money level a program pays at: Power 4, the rest of FBS, FCS, below.
fn money_level(w: &World, c: usize, year: i32) -> u8 {
    let prog = &w.programs[c];
    match prog.division {
        Division::FBS if prog.power_conference(year) => 0,
        Division::FBS => 1,
        Division::FCS => 2,
        _ => 3,
    }
}

fn role_key(r: Option<PlayingTime>) -> u8 {
    r.map_or(9, |x| x as u8)
}

impl Ctx {
    pub(crate) fn new(w: &World, label: i32) -> Ctx {
        let mut returning: HashMap<(usize, usize), Vec<(u32, f64)>> = HashMap::new();
        let mut pay: HashMap<(u8, usize, u8), Vec<f64>> = HashMap::new();
        let nil_era = super::ncaa::nil_era(label);
        for p in w.people.iter() {
            if p.stage != Stage::College {
                continue;
            }
            let Some(c) = p.college else { continue };
            let div = w.programs[c].division;
            let pos = p.player.position.index();
            if nil_era {
                pay.entry((money_level(w, c, label), pos, role_key(p.last_role))).or_default().push(p.nil_income);
            }
            if p.cal.portal.is_some()
                || p.cal.declare_plan == Some(true)
                || super::college::no_season_left(p, div, label)
            {
                continue;
            }
            returning.entry((c, pos)).or_default().push((p.player.id, p.player.overall()));
        }
        for v in returning.values_mut() {
            v.sort_by(|a, b| b.1.total_cmp(&a.1));
        }
        let nil_median = pay
            .into_iter()
            .map(|(k, mut v)| {
                v.sort_by(|a, b| a.total_cmp(b));
                (k, v[v.len() / 2])
            })
            .collect();
        let mine = super::front_office::program(w);
        let mut ctx = Ctx { label, returning, nil_median, nil_era, mine, level: HashMap::new() };
        let mut sums: HashMap<Division, (f64, f64)> = HashMap::new();
        for i in readable(w) {
            let p = &w.people[i];
            let prev: f64 = p.cal.lean_events.iter().filter(|e| is_read(e.1)).map(|e| e.2).product();
            let lean = p.cal.lean / prev;
            let f: f64 = raw(w, &ctx, i).iter().map(|x| x.1).product();
            let s = sums.entry(w.programs[p.college.unwrap()].division).or_default();
            s.0 += lean * f;
            s.1 += lean;
        }
        ctx.level = sums.into_iter().filter(|x| x.1.0 > 0.0).map(|(d, (a, b))| (d, b / a)).collect();
        ctx
    }
}

/// The college players the read reads: a lean, not in the portal, not at
/// a junior college.
fn readable(w: &World) -> impl Iterator<Item = usize> + '_ {
    (0..w.people.len()).filter(|&i| {
        let p = &w.people[i];
        p.stage == Stage::College
            && p.cal.lean > 0.0
            && p.cal.portal.is_none()
            && p.college.is_some_and(|c| w.programs[c].division != Division::JUCO)
    })
}

/// The lean base of a role (the fall camp lean's own: `college::lean_checkpoint`).
fn base(role: PlayingTime) -> f64 {
    let rates = crate::career::PIPELINE.portal_entry;
    match role {
        PlayingTime::Starter => rates.rotation / 3.0,
        PlayingTime::Rotation => rates.rotation,
        _ => rates.backup,
    }
}

/// What seeing himself in a role next fall holds him by (3a).
fn hold(role: PlayingTime) -> f64 {
    match role {
        PlayingTime::Starter => START_HOLD,
        PlayingTime::Rotation => NEXT_HOLD,
        _ => 1.0,
    }
}

/// His guess at his place next fall (3a): the returning men at his spot
/// he thinks he is better than, by his own view of himself against what he
/// sees of them every day. Never the depth chart.
pub(crate) fn next_fall(w: &World, ctx: &Ctx, i: usize) -> Next {
    let p = &w.people[i];
    let Some(c) = p.college else { return Next::Behind };
    let pos = p.player.position;
    let view = super::people::self_view(p);
    let ahead = ctx
        .returning
        .get(&(c, pos.index()))
        .map_or(0, |v| v.iter().filter(|x| x.0 != p.player.id && x.1 > view).count());
    let slots = crate::league::depth_slots(pos);
    if ahead < slots.starters {
        Next::Start
    } else if ahead < slots.starters + slots.rotation.max(1) {
        Next::InLine
    } else {
        Next::Behind
    }
}

/// How much he believes his head coach's promise, 0.2-1: the promise's
/// own belief (the promises design, section 4; lane P1), which moves each
/// week with what he sees and what you say. A promise not heard yet is
/// heard now. Held to 0.2-1 as before, since the day's read multiplies
/// by it.
pub(crate) fn promise_belief(w: &World, _ctx: &Ctx, i: usize, pr: &super::folk::talks::Promise) -> f64 {
    let b = if pr.belief > 0.0 {
        pr.belief as f64
    } else {
        super::folk::promise::belief_own(w, i, pr.kind, &pr.terms, pr.backed)
    };
    b.clamp(0.2, 1.0)
}

/// The role a playing-time promise gives him.
fn promised_role(kind: PromiseKind) -> Option<PlayingTime> {
    match kind {
        PromiseKind::Start | PromiseKind::Join(0) => Some(PlayingTime::Starter),
        PromiseKind::Snaps | PromiseKind::Join(_) => Some(PlayingTime::Rotation),
        PromiseKind::Place(r) => Some(match r {
            0 => PlayingTime::Starter,
            1 => PlayingTime::Rotation,
            _ => PlayingTime::Backup,
        }),
        _ => None,
    }
}

/// The current head coach's playing-time promises to him: (the best open
/// one, any broken). Only yours exist (AI staffs make none yet, lane P2),
/// and only the current head coach's count (Cain, item 5): a new job
/// clears the old ones (`takeover::begin`).
fn promises<'a>(w: &'a World, ctx: &Ctx, i: usize) -> (Option<&'a super::folk::talks::Promise>, bool) {
    let p = &w.people[i];
    if ctx.mine.is_none() || p.college != ctx.mine {
        return (None, false);
    }
    let id = p.player.id;
    let mut open: Option<&super::folk::talks::Promise> = None;
    let mut broken = false;
    for pr in &w.folk.promises {
        if pr.to != PromiseTo::Player(id) {
            continue;
        }
        let Some(role) = promised_role(pr.kind) else { continue };
        match pr.kept {
            None => {
                if open.and_then(|o| promised_role(o.kind)).is_none_or(|r| (role as u8) < (r as u8)) {
                    open = Some(pr);
                }
            }
            Some(false) => broken = true,
            Some(true) => {}
        }
    }
    (open, broken)
}

/// The day's read on player `i`: the parts and their product, levelled
/// against his division's and held to the floor and ceiling. Empty when
/// he has no lean to read.
pub(crate) fn read(w: &World, ctx: &Ctx, i: usize) -> Vec<(&'static str, f64)> {
    let mut parts = raw(w, ctx, i);
    if parts.is_empty() {
        return parts;
    }
    let div = w.programs[w.people[i].college.unwrap()].division;
    if let Some(&k) = ctx.level.get(&div)
        && (k - 1.0).abs() > 0.01
    {
        parts.push((LEVEL, k));
    }
    let total: f64 = parts.iter().map(|x| x.1).product();
    let held = total.clamp(READ_MIN, READ_MAX);
    if (held - total).abs() > 1e-9 {
        parts.push((LIMIT, held / total));
    }
    parts
}

/// The read's own parts, before levelling and the limit.
fn raw(w: &World, ctx: &Ctx, i: usize) -> Vec<(&'static str, f64)> {
    let p = &w.people[i];
    let mut parts: Vec<(&'static str, f64)> = Vec::new();
    if p.stage != Stage::College || p.cal.lean <= 0.0 {
        return parts;
    }
    let Some(c) = p.college else { return parts };
    // Your word decided it (no future here, play elsewhere, last choice):
    // nothing more to read this year (`folk::future`).
    if super::folk::future::decided(p).is_some() {
        return parts;
    }
    let div = w.programs[c].division;
    // The fall camp base: the first event when it is a role.
    let camp = p.cal.lean_events.first().filter(|e| super::college::CAMP_ROLES.contains(&e.1) && e.2 > 0.0);
    let next = next_fall(w, ctx, i);
    let changed = p.cal.lean_events.iter().any(|e| e.1 == super::college::COACH_GONE);
    let (open, broken) = promises(w, ctx, i);
    let word = open.and_then(|pr| promised_role(pr.kind).map(|r| (r, promise_belief(w, ctx, i, pr))));
    if let Some(b0) = camp.map(|e| e.2) {
        // 2a: a new head coach hasn't seen him practice; the old staff's
        // role counts half, his own guess the other half.
        let own = if changed { 0.5 * b0 + 0.5 * base(next.role()) } else { b0 };
        if changed && (own - b0).abs() > 1e-9 {
            parts.push((OLD_ROLE_HALF, own / b0));
        }
        // 3a: with no change, his own guess of next fall holds him.
        let mut own = own;
        if !changed {
            match next {
                Next::Start => parts.push((SEES_START, START_HOLD)),
                Next::InLine => parts.push((SEES_NEXT, NEXT_HOLD)),
                Next::Behind => {}
            }
            own *= hold(next.role());
        }
        // 2b: your word on his place next fall wins where you said it, by
        // how much he believes it; it agrees with his guess when they match.
        if let Some((role, belief)) = word {
            let now = belief * base(role) * hold(role) + (1.0 - belief) * own;
            if (now / own - 1.0).abs() > 1e-9 {
                parts.push((YOUR_WORD, now / own));
            }
        }
    }
    // 4: a young starter, now or as he reads next fall (or as you told him).
    let starts = p.last_role == Some(PlayingTime::Starter)
        || next == Next::Start
        || word.is_some_and(|x| x.0 == PlayingTime::Starter);
    if starts && super::eligibility::seasons_left(p, div, ctx.label + 1) >= 2 {
        parts.push((YOUNG_STARTER, YOUNG_HOLD));
    }
    // 5: the promise holds him on its own; a broken one pushes him.
    if let Some((_, belief)) = word {
        parts.push((PROMISE_OPEN, 1.0 - PROMISE_HOLD * belief));
    }
    if broken {
        parts.push((PROMISE_BROKEN, BROKEN_PUSH));
    }
    // 6: trust in his current head coach only.
    if w.programs[c].head_coach.is_some_and(|h| h as u32 == p.trust_in) {
        let f = (1.0 - (p.trust as f64 - 0.5)).clamp(0.7, 1.3);
        if (f - 1.0).abs() > 0.01 {
            parts.push((if f < 1.0 { TRUSTS } else { DOUBTS }, f));
        }
    }
    // 8: his mood today.
    let f = (1.0 - 0.5 * p.mood as f64).clamp(0.75, 1.4);
    if (f - 1.0).abs() > 0.01 {
        parts.push((if f < 1.0 { HAPPY } else { UNHAPPY }, f));
    }
    // 10: NIL against the going rate at his level. A well-paid man stays
    // for the money only if he wants out for other reasons (Cain: "if they
    // have reasons to stay, don't add more NIL on top of it").
    if ctx.nil_era
        && let Some(&rate) =
            ctx.nil_median.get(&(money_level(w, c, ctx.label), p.player.position.index(), role_key(p.last_role)))
        && rate > 0.0
    {
        let m = (6.0 * p.family.own_wants.money).clamp(0.0, 2.0);
        let wants_out = parts.iter().map(|x| x.1).product::<f64>() > 1.0 || p.mood < -0.15;
        if p.nil_income < 0.5 * rate {
            parts.push((UNDERPAID, 1.0 + UNDERPAID_PUSH * m));
        } else if p.nil_income >= rate && wants_out && m > 0.0 {
            parts.push((PAID, 1.0 - PAID_HOLD * m));
        }
    }
    parts
}

/// Writes the day's read into his lean, taking the previous read back out
/// first, so the read counts once and `lean_on` still sees each day's.
pub(crate) fn apply(p: &mut super::Person, date: Date, parts: &[(&'static str, f64)]) {
    let prev: f64 = p.cal.lean_events.iter().filter(|e| is_read(e.1)).map(|e| e.2).product();
    let now: f64 = parts.iter().map(|x| x.1).product();
    if (now / prev - 1.0).abs() < 1e-9 {
        return;
    }
    if (prev - 1.0).abs() > 1e-9 {
        p.cal.lean *= 1.0 / prev;
        p.cal.lean_events.push((date, READ_UNDO, 1.0 / prev));
    }
    for &(why, f) in parts {
        p.cal.lean *= f;
        p.cal.lean_events.push((date, why, f));
    }
}

/// Reads every college player with a lean today (a window's day).
pub(crate) fn read_all(w: &mut World, label: i32) {
    let ctx = Ctx::new(w, label);
    let today = w.today;
    let reads: Vec<(usize, Vec<(&'static str, f64)>)> = readable(w).map(|i| (i, read(w, &ctx, i))).collect();
    for (i, parts) in reads {
        apply(&mut w.people[i], today, &parts);
    }
}

/// Reads one player again on his own day (your player, the day after his
/// talk with you).
pub(crate) fn read_one(w: &mut World, ctx: &Ctx, i: usize) {
    let today = w.today;
    let parts = read(w, ctx, i);
    apply(&mut w.people[i], today, &parts);
}
