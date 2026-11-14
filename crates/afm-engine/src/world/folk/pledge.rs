//! Promises to high school recruits (talk facts lane L6, item 10; the
//! promises design, `afm/promises/proposal.md` section 9). Coaches promise
//! recruits five things (talk facts design, section 6, "Promises"): a
//! start, a real role, his position (a quarterback who won't be moved to
//! safety), an early chance to play, and no redshirt.
//!
//! - **What it pulls** is a transfer's promise read for a recruit: its
//!   size, times how much playing time means to whoever is listening, times
//!   what it adds, times how far they believe it (`make`). Belief is your
//!   name for keeping your word, your read of people and the answers you
//!   gave straight (as `dialogue::promise_pull`), times how likely it looks
//!   from the public roster at his spot (the fact sheet's room: who stays
//!   and who leaves). The promises design's belief, read from your name on
//!   the trail, his high school coach and his family, is its lane P1, which
//!   replaces this read.
//! - **It is kept on his case** with your school (`Case.promise`) and adds
//!   beside the case and its cap, as a transfer's does. One a school.
//! - **The day he signs with you it becomes your promise**
//!   (`talks::PromiseKind::Recruit`), checked on what he sees from his
//!   first practice with you and his first games, never your chart: a start
//!   or an early chance after his fourth game (a start in his games for
//!   you, or with the ones in his last two practice weeks; any game for an
//!   early chance), a real role, his position and no redshirt at the
//!   season's end (8 games or 3 starts; the position he was promised; more
//!   than four games from 2018, else any game). It lapses if he never comes
//!   or leaves on his own.
//!
//! AI staffs make no promises yet (the promises design, section 10, its
//! lane P2). Every size here is Claude's (proposed, not confirmed).

use super::case;
use super::claims::{Ear, Group};
use crate::calendar::Date;
use crate::positions::Position;
use crate::world::World;

/// A promise to a recruit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Start = 0,
    Role,
    Position,
    Early,
    NoRedshirt,
}

impl Kind {
    pub const ALL: [Kind; 5] = [Kind::Start, Kind::Role, Kind::Position, Kind::Early, Kind::NoRedshirt];

    pub fn of(k: u8) -> Option<Kind> {
        Kind::ALL.get(k as usize).copied()
    }

    /// Its key in a line bank part (`recruit.promise.{key}.{believes|doubts}`).
    pub fn key(self) -> &'static str {
        match self {
            Kind::Start => "start",
            Kind::Role => "role",
            Kind::Position => "position",
            Kind::Early => "early",
            Kind::NoRedshirt => "no_redshirt",
        }
    }

    /// What you promised, in words.
    pub fn words(self, pos: Position) -> String {
        match self {
            Kind::Start => "a start in his first season".into(),
            Kind::Role => "a real role in his first season".into(),
            Kind::Position => format!("that he'd stay at {}", super::claims::words::spot(pos)),
            Kind::Early => "an early chance to play".into(),
            Kind::NoRedshirt => "no redshirt year".into(),
        }
    }

    /// Your words when you make it.
    pub fn label(self, pos: Position) -> String {
        match self {
            Kind::Start => "\"You'll start for us as a freshman. You have my word.\"".into(),
            Kind::Role => "\"You'll have a real role from your first year.\"".into(),
            Kind::Position => {
                format!("\"You'll play {} here. Nobody moves you.\"", super::claims::words::spot(pos))
            }
            Kind::Early => "\"You'll get your chance early. You won't wait your turn.\"".into(),
            Kind::NoRedshirt => "\"You won't redshirt. You'll play as a freshman.\"".into(),
        }
    }

    /// When and how it is checked, for the pick's hint.
    pub fn hint(self) -> &'static str {
        match self {
            Kind::Start => "Checked after his fourth game with you: a start, or work with the ones.",
            Kind::Role => "Checked at the end of his first season: 8 games or 3 starts.",
            Kind::Position => "Checked at the end of his first season: still at his position.",
            Kind::Early => "Checked after his fourth game with you: he has played.",
            Kind::NoRedshirt => "Checked at the end of his first season: he played past a redshirt.",
        }
    }
}

/// Each promise's pull at its fullest, in appeal points (a starting job and
/// a real role as a transfer's, `dialogue::PROMISE_PULL`). Proposed, not
/// confirmed.
pub const PULL: [f64; 5] = [0.06, 0.035, 0.02, 0.03, 0.025];

/// What each promise adds to what a freshman expects anyway (he rarely
/// starts, often waits). Proposed, not confirmed.
pub const ADDS: [f64; 5] = [1.0, 0.6, 0.5, 0.5, 0.4];

/// He believes it from this much (as a transfer's "believes").
pub const BELIEVES: f64 = 0.6;

/// How likely the promise looks from the room at his spot (0 crowded to 1
/// open, the fact sheet's `Room`) and his stars.
fn likely(k: Kind, room: f64, stars: u8) -> f64 {
    let room = room.clamp(0.0, 1.0);
    match k {
        Kind::Start => (0.35 + 0.65 * room) * (0.7 + 0.075 * stars as f64).min(1.0),
        Kind::Position => 1.0,
        Kind::NoRedshirt => 0.6 + 0.4 * room,
        Kind::Role | Kind::Early => 0.5 + 0.5 * room,
    }
}

/// How far he believes a promise from you (the promises design, sections
/// 4 and 9; lane P1): the same six parts as your own player's, with the
/// trail's name, the answers you gave him straight and his family in place
/// of trust, and how likely it looks from the public roster at his spot.
pub fn belief(w: &World, k: Kind, room: f64, stars: u8, i: usize, j: usize, ear: Ear) -> f64 {
    super::promise::belief_out(w, i, j, ear, likely(k, room, stars))
}

/// You promise recruit `i` something for school `j`, said to `ear`: the
/// promise goes on his case and pulls by how much playing time means to
/// the listener, what it adds and how far they believe it. Returns (the
/// pull, the belief), or None when a promise is already on his case.
pub fn make(w: &mut World, i: usize, j: usize, k: Kind, ear: Ear, room: f64) -> Option<(f64, f64)> {
    let p = &w.people[i];
    if case::of(p, j).is_some_and(|c| c.promise != 0) {
        return None;
    }
    let care = super::claims::group_cares(p, ear)[Group::PlayingTime as usize] as f64;
    let b = belief(w, k, room, p.stars, i, j, ear);
    let pull = PULL[k as usize] * care * ADDS[k as usize] * b;
    let pos = p.player.position as u8;
    let today = w.today;
    let c = case::case_mut(w, i, j);
    c.promise = k as u8 + 1;
    c.pledge = pull as f32;
    c.pos = pos;
    c.promised = today;
    Some((pull, b))
}

/// The promise on his case with school `j`, if any.
pub fn on_case(w: &World, i: usize, j: usize) -> Option<Kind> {
    case::of(&w.people[i], j).and_then(|c| c.promise.checked_sub(1)).and_then(Kind::of)
}

/// Each day at your desk: a recruit who signed with you brings the
/// promise you made him with him (`talks::PromiseKind::Recruit`).
pub(crate) fn log_signed(w: &mut World, j: usize) {
    let ids: Vec<u32> = w.front_office.college.board.iter().map(|t| t.id).collect();
    for id in ids {
        let Some(i) = super::person(w, id) else { continue };
        if w.people[i].cal.signed.as_ref().is_none_or(|s| s.program != j) {
            continue;
        }
        let Some(c) = case::of(&w.people[i], j).filter(|c| c.promise != 0 && !c.logged).copied() else { continue };
        case::case_mut(w, i, j).logged = true;
        let kind = super::talks::PromiseKind::Recruit(c.promise - 1, c.pos);
        super::talks::promise(w, super::talks::PromiseTo::Player(id), kind, 0);
        let k = w.folk.promises.len() - 1;
        // A start is his from week 4 of his first season, judged over the
        // games to its end (section 7); a role from the opener.
        if Kind::of(c.promise - 1) == Some(Kind::Start) {
            let t = super::promise::Terms { by: super::promise::By::Week4, dated: true, ..Default::default() };
            super::talks::set_terms(w, k, t);
        }
        w.folk.promises[k].made = c.promised;
    }
}

/// When a recruit's promise is due, from the day it is logged: after his
/// fourth game with you for a start or an early chance, else the season's
/// end.
pub(crate) fn due(w: &World, k: u8) -> Date {
    let today = w.today;
    match Kind::of(k) {
        Some(Kind::Early) => {
            let fourth = w.dates.cfb_weeks.get(3).copied().unwrap_or(w.dates.fall_camp.add(50)).add(3);
            if fourth > today { fourth } else { fourth.add(365) }
        }
        _ => {
            let e = w.dates.fbs_regular_end.add(3);
            if e > today.add(30) { e } else { e.add(365) }
        }
    }
}

/// Whether he got what you promised, from what he saw: his games and starts
/// for you this season, his practice group, his position.
pub(crate) fn kept(w: &World, i: usize, k: u8, pos: u8, games: u32, starts: u32) -> bool {
    match Kind::of(k) {
        Some(Kind::Start) => starts > 0 || with_the_ones(w, w.people[i].player.id),
        Some(Kind::Role) => games >= 8 || starts >= 3,
        Some(Kind::Position) => w.people[i].player.position as u8 == pos,
        Some(Kind::Early) => games >= 1,
        Some(Kind::NoRedshirt) => {
            if w.changes.rules(w.dates.season).four_game_redshirt {
                games > 4
            } else {
                games >= 1
            }
        }
        None => true,
    }
}

/// He worked with the ones in each of his last two practice weeks (what he
/// sees, the promises design, section 2).
fn with_the_ones(w: &World, id: u32) -> bool {
    let weeks: Vec<u8> = w
        .training
        .groups
        .iter()
        .rev()
        .filter(|x| x.0 <= w.today)
        .filter_map(|x| x.1.iter().find(|y| y.0 == id).map(|y| y.1))
        .take(2)
        .collect();
    weeks.len() == 2 && weeks.iter().all(|&g| g == crate::training::seat::GROUP_ONES)
}
