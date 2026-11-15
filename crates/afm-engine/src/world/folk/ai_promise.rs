//! AI staffs make and break promises (the promises design,
//! `docs/design/promises.md` section 10, lane P2; Cain agreed 2026-11-13,
//! question 12: "AI staffs make and keep promises by the same rules").
//! Before this lane only you promised, so a rival's word read neutral and
//! no player was ever burned elsewhere.
//!
//! - **Who promises**: an AI program's head coach, at three moments that
//!   are yours too: the sit-down with a high school recruit (his home
//!   visit and the end of an official visit), a transfer he wants (the
//!   staff's call), and a player of his own who might leave (the portal
//!   window's stay meeting). Never your program.
//! - **What his roster allows**: he reads the room at the recruit's spot
//!   (who stays, plus the starts and roles he has already promised this
//!   cycle at that spot), a transfer's role there, or where his own man
//!   ranks among the men coming back. He uses a promise at all as readily
//!   as he sells (`sells`, his recruiting), and then gives the strongest
//!   kind his read says is sure, or a stronger one that reads as a stretch
//!   as far as he isn't careful (`careful`, his relationships; `pick`). His read strays by his recruiting
//!   rating (`case::read_noise`), so a poor reader over-promises without
//!   meaning to. His depth chart never bends to a promise: his games keep
//!   or break it.
//! - **Belief** is P1's six parts with the coach's own name on the trail
//!   in place of yours (`belief`), and pulls as yours does: a recruit's on
//!   the school's case beside its cap (`pledge::PULL`), a transfer's in his
//!   choice (`dialogue::transfer_pull`), a player's in the day's read
//!   (`stay_read`, as your word).
//! - **Judged** once a season, three days after the FBS regular season
//!   ends, on the season's own record (`day`): a start kept in 3 of every
//!   4 games he was healthy for (from week 4 for a freshman), a role as 8
//!   games or 3 starts in a 12-game season, his position, an early chance
//!   (a game), no redshirt (past four games from 2018). A coach gone, a
//!   player gone or hurt most of the term: it lapses.
//! - **What it costs and pays**: P1's sizes. Kept: his trust in the coach
//!   +0.10 x (2 - belief), mood +0.14; broken: trust -0.22 x (0.4 + 0.6 x
//!   belief) x the share missed, mood 1.4x that, his close friends' trust
//!   in the coach -0.04 x it, and he is burned (`burned`): every later
//!   promise from anyone is believed a little less (`promise::past`). A
//!   recruit's or a transfer's promise moves the coach's trail name, +0.04
//!   kept and -0.08 x the broken scale, fading a quarter a year; the name
//!   travels with him. The day's read pushes a player whose current head
//!   coach broke one (`stay_read`, as yours).
//!
//! Every size here is Claude's, proposed, not confirmed, except the ones
//! that are P1's (confirmed 2026-11-14).

use std::collections::{BTreeMap, HashMap};

use super::case;
use super::claims::{Ear, Group};
use super::pledge::{self, Kind};
use super::promise::{self, Parts};
use super::{Team, person};
use crate::calendar::Date;
use crate::game::talk::normal;
use crate::player::PlayingTime;
use crate::world::season::key_uniform;
use crate::world::{Stage, World};

/// Where an AI promise was made.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum From {
    /// To a high school recruit at a sit-down.
    #[default]
    Recruit,
    /// To a transfer the staff wanted.
    Transfer,
    /// To one of his own players in the portal window: a start next fall.
    Stay,
}

/// One AI head coach's promise to a player who came (or stayed).
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct AiPromise {
    /// The player's id.
    pub to: u32,
    /// The program.
    pub prog: u16,
    /// The head coach who gave his word (a coach index).
    pub coach: u32,
    pub from: From,
    /// What he promised, `pledge::Kind as u8`.
    pub kind: u8,
    /// His position when he heard it (`Position as u8`).
    pub pos: u8,
    pub made: Date,
    /// The season whose games judge it.
    pub season: i32,
    /// How far he believed it when he heard it.
    pub belief: f32,
    /// None while open; Some(true) kept, Some(false) broken.
    pub kept: Option<bool>,
    /// The share he missed, when broken.
    pub missed: f32,
    /// Nothing to judge: the coach or the player gone, or hurt all its term.
    pub lapsed: bool,
}

/// The AI staffs' promises (one per world).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Book {
    /// Logged promises, open and judged in the last two seasons.
    pub promises: Vec<AiPromise>,
    /// Each AI head coach's name on the recruiting trail for keeping his
    /// word, -1 to 1 (0 neutral), by coach index. It travels with him.
    pub trail: BTreeMap<u32, f32>,
    /// Players any staff broke a promise to (sorted ids): burned.
    pub burned: Vec<u32>,
    /// The starts (1 each) and roles (a half each) a program's head coach
    /// promised recruits at a spot this cycle: `prog << 8 | pos`.
    pub pending: BTreeMap<u32, f32>,
    /// The season `pending` counts.
    pub pending_season: i32,
    /// The last season judged (and the names faded).
    pub judged: i32,
}

// ---------------------------------------------------------------------------
// Sizes (proposed, not confirmed).

/// A read of a promise's chances from here is sure: he promises as readily
/// as he sells.
pub const SURE: f64 = 0.7;
/// From here to `SURE` it's a stretch: he promises only as far as he isn't
/// careful.
pub const STRETCH: f64 = 0.5;
/// How far his read of the room strays: this x `case::read_noise` (0.15
/// room at recruiting 10, 0.04 at 20, 0.23 at 1).
pub const ROOM_NOISE: f64 = 0.25;
/// His read of his own men coming back strays this many rating points x
/// `case::read_noise` / 0.6 (2 at recruiting 10): he sees them every day.
pub const OWN_NOISE: f64 = 2.0;
/// What each pending promise at a spot takes from the room he reads: a
/// start a man, a role half a man, an early chance a quarter.
pub const PENDING: [f32; 5] = [1.0, 0.5, 0.0, 0.25, 0.0];
/// A burned man believes every later promise this much less (x).
pub const BURNED: f64 = 0.88;
/// The trail name: kept +0.04, broken -0.08 x the broken scale (P1's).
pub const KEPT_NAME: f32 = 0.04;
pub const BROKEN_NAME: f32 = 0.08;
/// The name fades a quarter of the way to neutral a year (P1's).
pub const NAME_FADE: f32 = 0.75;
/// A start a freshman was promised runs from his fourth game.
pub const FRESH_START_FROM: u32 = 3;
/// A role is 8 games or 3 starts in a 12-game season (P1's recruit rule),
/// scaled to the games he was healthy for.
pub const ROLE_GAMES: f64 = 8.0 / 12.0;
pub const ROLE_STARTS: f64 = 3.0 / 12.0;

// ---------------------------------------------------------------------------
// The coach.

/// His name on the trail, -1 to 1.
pub fn trail(w: &World, coach: usize) -> f64 {
    w.folk.ai_promises.trail.get(&(coach as u32)).copied().unwrap_or(0.0) as f64
}

/// How readily he uses a promise to land or keep a man: his recruiting,
/// 0.1-0.9 (0.5 at 10).
pub fn sells(w: &World, coach: usize) -> f64 {
    (0.5 + 0.04 * (w.coaches[coach].ability().recruiting - 10.0)).clamp(0.1, 0.9)
}

/// How careful he is to promise only what his roster allows: his
/// relationships, 0.1-0.9 (0.5 at 10).
pub fn careful(w: &World, coach: usize) -> f64 {
    (0.5 + 0.04 * (w.coaches[coach].ability().relationships - 10.0)).clamp(0.1, 0.9)
}

/// What he promises, if anything, from his reads of each kind, strongest
/// first: he uses a promise at all as readily as he sells (`sell`, a
/// roll); then he takes the strongest kind his read says is sure, unless
/// a stronger one reads as a stretch and he isn't careful enough to pass
/// it up (`dare`, a roll for each stretch). None when nothing reads sure
/// or taken.
fn pick(w: &World, coach: usize, reads: &[(Kind, f64)], sell: f64, dare: impl Fn(usize) -> f64) -> Option<Kind> {
    if sell >= sells(w, coach) {
        return None;
    }
    for (n, &(k, r)) in reads.iter().enumerate() {
        if r >= SURE {
            return Some(k);
        }
        if r >= STRETCH && dare(n) >= careful(w, coach) {
            return Some(k);
        }
    }
    None
}

/// A program's AI head coach (None for your program or a program without one).
fn ai_coach(w: &World, j: usize) -> Option<usize> {
    if Team::seat(w) == Some(Team::College(j)) {
        return None;
    }
    w.programs[j].head_coach.filter(|&c| w.career.as_ref().is_none_or(|y| y.coach != c))
}

/// Whether a player was burned: a staff broke a promise to him.
pub fn burned(w: &World, id: u32) -> bool {
    w.folk.ai_promises.burned.binary_search(&id).is_ok()
}

/// His belief in a promise from AI head coach `coach`: P1's six parts with
/// the coach's trail name in place of yours. `own` reads his trust in his
/// own head coach (a stay); out of the building the trail, the coach's way
/// with people and who decides in his family.
pub fn belief(w: &World, i: usize, coach: usize, ear: Ear, likely: f64, own: bool) -> f64 {
    let p = &w.people[i];
    let name = trail(w, coach);
    let rel = w.coaches[coach].ability().relationships;
    let trust = if own {
        if p.trust_in == coach as u32 { p.trust as f64 } else { 0.5 }
    } else {
        let family = if ear == Ear::Decider { -0.03 } else { 0.0 };
        (0.5 + 0.25 * name + 0.01 * (rel - 10.0) + family).clamp(0.1, 0.9)
    };
    Parts {
        trust,
        record: (0.5 + 0.25 * name).clamp(0.0, 1.0),
        likely: likely.clamp(0.3, 1.0),
        past: promise::past(w, i),
        ear: promise::ear(w, i),
        shape: 1.0,
    }
    .belief()
}

/// The season a promise made today is judged on: the next fall.
fn next_season(w: &World) -> i32 {
    if w.today <= w.dates.fall_camp { w.dates.season } else { w.dates.season + 1 }
}

// ---------------------------------------------------------------------------
// A high school recruit (at a sit-down).

fn spot_key(j: usize, pos: u8) -> u32 {
    (j as u32) << 8 | pos as u32
}

/// The room at his spot as the head coach reads it, 0 crowded to 1 open:
/// the men staying at the spot and the promises he already made there
/// this cycle (the fact sheet's room, `claims::program::room`, plus his
/// own word).
pub fn room(w: &World, i: usize, j: usize) -> f64 {
    let pos = w.people[i].player.position;
    let staying = w.programs[j]
        .roster
        .iter()
        .filter(|&&k| {
            k != i
                && w.people[k].player.position == pos
                && w.people[k].stage == Stage::College
                && !super::facts::leaving(w, k)
        })
        .count() as f64;
    let b = &w.folk.ai_promises;
    let pending = if b.pending_season == w.dates.season {
        b.pending.get(&spot_key(j, pos as u8)).copied().unwrap_or(0.0) as f64
    } else {
        0.0
    };
    let slots = crate::game::roster::starters(pos).max(1) as f64;
    (1.0 - (staying + pending) / (3.0 * slots)).clamp(0.0, 1.0)
}

/// The head coach of AI program `j` at a sit-down with recruit `i` (said to
/// `ear`): he reads the room and, when a start, a real role or an early
/// chance reads sure (or a stretch he takes), promises the first one he
/// will (one a school, on its case). Returns the kind.
pub(crate) fn recruit(w: &mut World, i: usize, j: usize, ear: Ear, seed: u64) -> Option<Kind> {
    let c = ai_coach(w, j)?;
    if case::of(&w.people[i], j).is_some_and(|x| x.promise != 0) {
        return None;
    }
    let p = &w.people[i];
    let (stars, id, pos) = (p.stars, p.player.id as u64, p.player.position as u8);
    let truth = room(w, i, j);
    let read = (truth + ROOM_NOISE * case::read_noise(w, Some(c)) * normal(seed ^ 0x9A00, id)).clamp(0.0, 1.0);
    let reads: Vec<(Kind, f64)> =
        [Kind::Start, Kind::Role, Kind::Early].into_iter().map(|k| (k, pledge::likely(k, read, stars))).collect();
    let k = pick(w, c, &reads, key_uniform(seed ^ 0x9A10, id), |n| key_uniform(seed ^ 0x9A20 ^ n as u64, id))?;
    let care = super::claims::group_cares(&w.people[i], ear)[Group::PlayingTime as usize] as f64;
    let b = belief(w, i, c, ear, pledge::likely(k, truth, stars), false);
    let pull = pledge::PULL[k as usize] * care * pledge::ADDS[k as usize] * b;
    let today = w.today;
    let x = case::case_mut(w, i, j);
    x.promise = k as u8 + 1;
    x.pledge = pull as f32;
    x.pos = pos;
    x.promised = today;
    x.belief = b as f32;
    let season = w.dates.season;
    let book = &mut w.folk.ai_promises;
    if book.pending_season != season {
        book.pending.clear();
        book.pending_season = season;
    }
    *book.pending.entry(spot_key(j, pos)).or_insert(0.0) += PENDING[k as usize];
    Some(k)
}

/// He enrolls at an AI program that promised him something: the promise
/// goes in the book, judged on his first season there.
pub(crate) fn log_recruit(w: &mut World, i: usize, j: usize) {
    let Some(c) = ai_coach(w, j) else { return };
    let Some(x) = case::of(&w.people[i], j).filter(|x| x.promise != 0 && !x.logged).copied() else { return };
    let season = next_season(w);
    w.folk.ai_promises.promises.push(AiPromise {
        to: w.people[i].player.id,
        prog: j as u16,
        coach: c as u32,
        from: From::Recruit,
        kind: x.promise - 1,
        pos: x.pos,
        made: x.promised,
        season,
        belief: if x.belief > 0.0 { x.belief } else { 0.5 },
        ..Default::default()
    });
}

// ---------------------------------------------------------------------------
// A transfer (the staff's call).

/// The likely look of a promise to a transfer from the role he'd have:
/// a start a starter's 0.9, a rotation man's 0.65, a backup's 0.45; a role
/// 0.95, a backup's 0.65.
fn transfer_likely(k: Kind, role: PlayingTime) -> f64 {
    match (k, role) {
        (Kind::Start, PlayingTime::Starter) => 0.9,
        (Kind::Start, PlayingTime::Rotation) => 0.65,
        (Kind::Start, _) => 0.45,
        (_, PlayingTime::Starter | PlayingTime::Rotation) => 0.95,
        _ => 0.65,
    }
}

/// What the head coach of AI program `j` promises transfer `i` with the
/// role he'd have there: a start (sure for a starter, a stretch for a
/// rotation man) or else a real role (sure for a rotation man, a stretch
/// for a backup). Pure, so the portal reads it while it ranks his schools
/// and again the day he lands. Returns (kind, pull, belief).
pub(crate) fn transfer(w: &World, i: usize, j: usize, role: PlayingTime) -> Option<(Kind, f64, f64)> {
    let c = ai_coach(w, j)?;
    let id = w.people[i].player.id as u64;
    let seed = w.options.seed as u64 ^ (j as u64) << 40 ^ (w.dates.season as u64) << 24 ^ 0x7A11;
    let reads = |k: Kind| match (k, role) {
        (Kind::Start, PlayingTime::Starter) | (Kind::Role, PlayingTime::Rotation | PlayingTime::Starter) => SURE,
        (Kind::Start, PlayingTime::Rotation) | (Kind::Role, _) => STRETCH,
        _ => 0.0,
    };
    let reads = [(Kind::Start, reads(Kind::Start)), (Kind::Role, reads(Kind::Role))];
    let k = pick(w, c, &reads, key_uniform(seed, id), |n| key_uniform(seed ^ 0x10 ^ n as u64, id))?;
    let b = belief(w, i, c, Ear::Himself, transfer_likely(k, role), false);
    let pull = super::dialogue::transfer_pull(w, i, k as u8, role, b);
    Some((k, pull, b))
}

/// He lands at an AI program: what its head coach promised him goes in
/// the book, judged on his first season there.
pub(crate) fn log_transfer(w: &mut World, i: usize, j: usize, role: PlayingTime) {
    let Some((k, _, b)) = transfer(w, i, j, role) else { return };
    let Some(c) = ai_coach(w, j) else { return };
    let season = next_season(w);
    let p = &w.people[i];
    w.folk.ai_promises.promises.push(AiPromise {
        to: p.player.id,
        prog: j as u16,
        coach: c as u32,
        from: From::Transfer,
        kind: k as u8,
        pos: p.player.position as u8,
        made: w.today,
        season,
        belief: b as f32,
        ..Default::default()
    });
}

// ---------------------------------------------------------------------------
// One of his own (the portal window's stay meeting).

/// The players with an open AI promise (the stay meeting gives none to a
/// man who holds one).
pub fn open_ids(w: &World) -> std::collections::HashSet<u32> {
    w.folk.ai_promises.promises.iter().filter(|x| x.kept.is_none() && !x.lapsed).map(|x| x.to).collect()
}

/// The head coach of AI program `j` and one of his non-starters who might
/// leave, with the men coming back at the spot (`others`: their overalls,
/// him left out): he reads where the man ranks next fall (his overall with
/// the coach's stray against theirs) and promises him a start when the
/// read is sure (he is among the starters) or, as a stretch, next in line.
/// Logs it and returns its belief. The caller skips a man who holds an
/// open promise already (`open_ids`).
pub(crate) fn stay(w: &mut World, i: usize, j: usize, others: &[f64]) -> Option<f64> {
    let c = ai_coach(w, j)?;
    let p = &w.people[i];
    let id = p.player.id;
    let pos = p.player.position;
    let seed = w.options.seed as u64 ^ (w.dates.season as u64) << 24 ^ 0x57A1;
    let sd = OWN_NOISE * case::read_noise(w, Some(c)) / 0.6;
    let read = p.player.overall() + sd * normal(seed, id as u64);
    let ahead = others.iter().filter(|&&o| o > read).count();
    let starters = crate::league::depth_slots(pos).starters.max(1);
    let r = if ahead < starters {
        SURE
    } else if ahead == starters {
        STRETCH
    } else {
        0.0
    };
    pick(w, c, &[(Kind::Start, r)], key_uniform(seed ^ 1, id as u64), |_| key_uniform(seed ^ 2, id as u64))?;
    // How likely it looks to him: the man he must pass (the last starter
    // he'd beat) against his own view of himself.
    let mut sorted: Vec<f64> = others.to_vec();
    sorted.sort_by(|a, b| b.total_cmp(a));
    let bar = sorted.get(starters - 1).copied();
    let me = crate::world::people::self_view(p);
    let likely = bar.map_or(1.0, |b| 0.65 + 0.06 * (me - b - 1.0));
    let b = belief(w, i, c, Ear::Himself, likely, true);
    let season = next_season(w);
    w.folk.ai_promises.promises.push(AiPromise {
        to: id,
        prog: j as u16,
        coach: c as u32,
        from: From::Stay,
        kind: Kind::Start as u8,
        pos: pos as u8,
        made: w.today,
        season,
        belief: b as f32,
        ..Default::default()
    });
    Some(b)
}

/// The day's read (`stay_read`): each player's open playing-time promise
/// from his current head coach at an AI program (the role and his belief),
/// and whether that coach broke one to him.
pub(crate) fn index(w: &World) -> HashMap<u32, (Option<(PlayingTime, f64)>, bool)> {
    let mut m: HashMap<u32, (Option<(PlayingTime, f64)>, bool)> = HashMap::new();
    for x in &w.folk.ai_promises.promises {
        if x.lapsed || w.programs.get(x.prog as usize).and_then(|p| p.head_coach) != Some(x.coach as usize) {
            continue;
        }
        let role = match Kind::of(x.kind) {
            Some(Kind::Start) => PlayingTime::Starter,
            Some(Kind::Role) => PlayingTime::Rotation,
            _ => continue,
        };
        let e = m.entry(x.to).or_default();
        match x.kept {
            None => {
                if e.0.is_none_or(|o| (role as u8) < (o.0 as u8)) {
                    e.0 = Some((role, x.belief as f64));
                }
            }
            Some(false) => e.1 = true,
            Some(true) => {}
        }
    }
    m
}

// ---------------------------------------------------------------------------
// Judging (once a season).

/// What a promise came to over the season's record.
fn verdict(w: &World, x: &AiPromise) -> promise::Verdict {
    use promise::Verdict as V;
    let Some(i) = person(w, x.to) else { return V::Lapsed };
    let p = &w.people[i];
    let j = x.prog as usize;
    // He left on his own (or never came), or the man who said it is gone:
    // the new head coach isn't bound.
    if p.stage != Stage::College || p.college != Some(j) || w.programs[j].head_coach != Some(x.coach as usize) {
        return V::Lapsed;
    }
    let Some(cs) = w.college.as_ref().filter(|c| c.on_engine && c.year == x.season) else { return V::Lapsed };
    let id = p.player.id;
    let team = cs.results.get(j).map_or(0, |r| r.games());
    let missed = p.player.history.last().filter(|h| h.level.is_college()).map_or(0, |h| h.games_missed);
    let healthy = team.saturating_sub(missed);
    if team == 0 || 3 * healthy < team || healthy < 2 {
        return V::Lapsed;
    }
    let played = cs.snaps.get(&id).map_or(0, |s| s.1);
    let starts = cs.starts.get(&id).copied().unwrap_or(0);
    let kind = Kind::of(x.kind);
    let r = match kind {
        Some(Kind::Start) => {
            let from = if x.from == From::Recruit { FRESH_START_FROM } else { 0 };
            let open = healthy.saturating_sub(from).max(1);
            (starts.min(open) as f64 / open as f64) / promise::KEPT_SHARE
        }
        Some(Kind::Role) => {
            let h = healthy as f64;
            (played as f64 / (ROLE_GAMES * h)).max(starts as f64 / (ROLE_STARTS * h))
        }
        Some(Kind::Position) => f64::from(p.player.position as u8 == x.pos),
        Some(Kind::Early) => f64::from(played >= 1),
        Some(Kind::NoRedshirt) => {
            f64::from(if w.changes.rules(x.season).four_game_redshirt { played > 4 } else { played >= 1 })
        }
        None => return V::Lapsed,
    };
    if r >= 1.0 { V::Kept } else { V::Broken((1.0 - r).clamp(0.0, 1.0)) }
}

/// Three days after the FBS regular season ends: every open promise for
/// this season is judged and paid out, the names fade a quarter, and
/// promises judged more than two seasons ago go.
pub(crate) fn day(w: &mut World, d: Date) {
    if d != w.dates.fbs_regular_end.add(3) || w.folk.ai_promises.judged >= w.dates.season {
        return;
    }
    judge(w);
}

/// The season's judging (`day`).
pub(crate) fn judge(w: &mut World) {
    let season = w.dates.season;
    w.folk.ai_promises.judged = season;
    // The names fade a quarter of the way to neutral a year.
    let trail = &mut w.folk.ai_promises.trail;
    for v in trail.values_mut() {
        *v *= NAME_FADE;
    }
    trail.retain(|_, v| v.abs() >= 0.005);
    let open: Vec<usize> = (0..w.folk.ai_promises.promises.len())
        .filter(|&k| {
            let x = &w.folk.ai_promises.promises[k];
            x.kept.is_none() && !x.lapsed && x.season <= season
        })
        .collect();
    let verdicts: Vec<(usize, promise::Verdict)> =
        open.into_iter().map(|k| (k, verdict(w, &w.folk.ai_promises.promises[k]))).collect();
    for (k, v) in verdicts {
        settle(w, k, v);
    }
    // Two seasons of judged promises are kept (the record a recruit hears);
    // the burned are kept while they are still in football.
    w.folk.ai_promises.promises.retain(|x| (x.kept.is_none() && !x.lapsed) || x.season + 2 > season);
    let burned = std::mem::take(&mut w.folk.ai_promises.burned);
    w.folk.ai_promises.burned =
        burned.into_iter().filter(|&id| person(w, id).is_some_and(|i| w.people[i].stage != Stage::Done)).collect();
}

/// Pays a verdict out: his trust and mood, his friends', the coach's name
/// and the burned list.
fn settle(w: &mut World, k: usize, v: promise::Verdict) {
    use promise::Verdict as V;
    let x = w.folk.ai_promises.promises[k];
    let trail_heard = matches!(x.from, From::Recruit | From::Transfer);
    let Some(i) = person(w, x.to) else {
        w.folk.ai_promises.promises[k].lapsed = true;
        return;
    };
    match v {
        V::Lapsed | V::Released(_) => w.folk.ai_promises.promises[k].lapsed = true,
        V::Kept => {
            w.folk.ai_promises.promises[k].kept = Some(true);
            let p = &mut w.people[i];
            if p.trust_in == x.coach {
                p.trust = (p.trust + promise::kept_trust(x.belief)).clamp(0.0, 1.0);
            }
            p.mood = (p.mood + 0.1 * 1.4).clamp(-1.0, 1.0);
            if trail_heard {
                let t = w.folk.ai_promises.trail.entry(x.coach).or_insert(0.0);
                *t = (*t + KEPT_NAME).clamp(-1.0, 1.0);
            }
        }
        V::Broken(share) => {
            let scale = promise::broken_scale(x.belief, share);
            {
                let e = &mut w.folk.ai_promises.promises[k];
                e.kept = Some(false);
                e.missed = share as f32;
            }
            let dt = -0.22 * scale;
            let prog = w.program_name(x.prog as usize).to_string();
            let p_pos = w.people[i].player.position;
            let what = Kind::of(x.kind).map_or_else(String::new, |kind| {
                kind.words(crate::positions::Position::ALL.get(x.pos as usize).copied().unwrap_or(p_pos))
            });
            let p = &mut w.people[i];
            if p.trust_in == x.coach {
                p.trust = (p.trust + dt).clamp(0.0, 1.0);
            }
            p.mood = (p.mood + 1.4 * dt).clamp(-1.0, 1.0);
            let age = p.row_age();
            p.note(format!("At {age} he felt let down at {prog}: his head coach had promised him {what}."));
            let id = p.player.id;
            for f in super::ties::friends(w, id) {
                if let Some(j) = person(w, f)
                    && w.people[j].trust_in == x.coach
                {
                    w.people[j].trust = (w.people[j].trust - 0.04 * scale).max(0.0);
                }
            }
            if trail_heard {
                let t = w.folk.ai_promises.trail.entry(x.coach).or_insert(0.0);
                *t = (*t - BROKEN_NAME * scale).clamp(-1.0, 1.0);
            }
            let b = &mut w.folk.ai_promises.burned;
            if let Err(at) = b.binary_search(&id) {
                b.insert(at, id);
            }
        }
    }
}
