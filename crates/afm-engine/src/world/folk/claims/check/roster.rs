//! Facts about your own players in a typed line (live talks lane 9, LT1):
//! each looked up on the sheet or on what the room sees, plus what the
//! sheet lacks (a teammate's snap share and touches, from your games).
//!
//! Everything here is something your player can check as you say it: his
//! own body and his own case, a teammate's injury, case or place (the room
//! sees who works with the ones and who plays the snaps, never your chart;
//! `folk::seen`), and your record. So a false one is caught on the spot and
//! costs what a move against the facts costs (`talks::typed_against`); none
//! is saved as a lie. A claim about where he himself stands never reaches
//! this file: it is a promise about what he'll see (`check::fact`). A stat,
//! a money or NIL number and another club's interest read unknown: the
//! game doesn't check them, so they count for nothing either way.

use crate::world::World;
use crate::world::folk::claims::Kind as K;
use crate::world::folk::claims::truth::{Stated, Truth, your_player};
use crate::world::folk::scene::Scene;
use crate::world::folk::{Team, facts, person, promise, seen};

/// A snap share said in so many words, held against what he played: within
/// this much is true. Proposed, not confirmed.
pub const SHARE_SLACK: f64 = 0.2;
/// Touches a game said in so many words: within this many, or this share
/// of what he gets, is true. Proposed, not confirmed.
pub const TOUCH_SLACK: (f64, f64) = (3.0, 0.3);
/// The games a teammate's touches are read over (the same window as a
/// touches promise). Proposed, not confirmed.
pub const TOUCH_GAMES: usize = 4;

pub fn check(w: &World, s: &Scene, stated: &Stated) -> Truth {
    let Some(him) = your_player(w, s) else { return Truth::Unknown };
    let v = stated.value.to_lowercase();
    if stated.kind == "record" {
        return Team::seat(w).map_or(Truth::Unknown, |t| record(facts::record_now(w, t), &v));
    }
    let about = stated.about.unwrap_or(him);
    let Some(j) = person(w, about) else { return Truth::Unknown };
    if super::super::team_of(w, j) != Team::seat(w) {
        return Truth::Unknown;
    }
    let hurt = seen::hurt_on(w, about, w.today);
    match stated.kind.as_str() {
        "injury" => said(!healthy(&v), hurt),
        "status" => status(w, j, &v, hurt),
        "off_field" => {
            let sheet = super::super::player(w, j);
            let case = [K::Arrest, K::CaseClosed, K::Suspended, K::RuleBreak].into_iter().any(|k| sheet.has(k));
            said(!denies(&v), case)
        }
        "depth_rank" | "role" if about != him => {
            place(&v).map_or(Truth::Unknown, |first| said(first, seen::starter(w, j)))
        }
        // A specialist's share is the kicking game's, not a side's.
        "snaps_share" if about != him && !w.people[j].player.position.is_specialist() => {
            share(&v).map_or(Truth::Unknown, |x| said(true, (x - promise::share_now(w, j)).abs() <= SHARE_SLACK))
        }
        "touches" if about != him => match (number(&v), touches(w, about)) {
            (Some(n), Some(t)) => said(true, (n - t).abs() <= TOUCH_SLACK.0.max(TOUCH_SLACK.1 * t)),
            _ => Truth::Unknown,
        },
        _ => Truth::Unknown,
    }
}

/// True when what was said matches what is so.
fn said(claim: bool, so: bool) -> Truth {
    if claim == so { Truth::True } else { Truth::False }
}

fn has(v: &str, words: &[&str]) -> bool {
    words.iter().any(|x| v.contains(x))
}

/// The words say he is fine, not hurt.
fn healthy(v: &str) -> bool {
    has(v, &["healthy", "cleared", "not hurt", "fine", "full go", "100", "no injur", "isn't hurt", "is not hurt"])
}

/// The words deny a case.
fn denies(v: &str) -> bool {
    has(v, &["clean", "no case", "never", "nothing", "no trouble", "no record", "not in trouble"])
}

/// A teammate's status: hurt, suspended, or healthy. Anything else the game
/// doesn't check.
fn status(w: &World, j: usize, v: &str, hurt: bool) -> Truth {
    if has(v, &["suspend"]) {
        let out = super::super::player(w, j).has(K::Suspended);
        return said(!has(v, &["not suspend", "no longer suspend"]), out);
    }
    if healthy(v) || has(v, &["back"]) && !has(v, &["back injur", "back spasm", "his back"]) {
        return said(false, hurt);
    }
    if has(v, &["hurt", "injur", "out for", "sidelined", "banged up", "in a boot", "can't go", "won't play"]) {
        return said(true, hurt);
    }
    Truth::Unknown
}

/// A place in the words: Some(true) a starter, Some(false) not one.
fn place(v: &str) -> Option<bool> {
    if has(v, &["backup", "second", "2nd", "twos", "bench", "third", "3rd", "scout", "not start", "isn't start"]) {
        return Some(false);
    }
    has(v, &["start", "first", "1st", "the ones", "number one", "no. 1", "our guy", "the guy", "the man"])
        .then_some(true)
}

/// A snap share in the words: a percent, or "all", "most", "half", "few".
fn share(v: &str) -> Option<f64> {
    if let Some(n) = number(v)
        && v.contains('%')
    {
        return Some((n / 100.0).clamp(0.0, 1.0));
    }
    [
        (&["every snap", "all the snaps", "all of them"][..], 0.95),
        (&["most"][..], 0.7),
        (&["half"][..], 0.5),
        (&["barely", "hardly", "a few", "no snaps", "doesn't play"][..], 0.05),
    ]
    .into_iter()
    .find(|(w, _)| has(v, w))
    .map(|x| x.1)
}

/// The first number in the words, in digits or as a word up to twenty.
fn number(v: &str) -> Option<f64> {
    const WORDS: [&str; 21] = [
        "zero",
        "one",
        "two",
        "three",
        "four",
        "five",
        "six",
        "seven",
        "eight",
        "nine",
        "ten",
        "eleven",
        "twelve",
        "thirteen",
        "fourteen",
        "fifteen",
        "sixteen",
        "seventeen",
        "eighteen",
        "nineteen",
        "twenty",
    ];
    for t in v.split(|c: char| !(c.is_ascii_alphanumeric() || c == '.')) {
        if let Ok(n) = t.trim_end_matches('.').parse::<f64>() {
            return Some(n);
        }
        if let Some(n) = WORDS.iter().position(|x| *x == t) {
            return Some(n as f64);
        }
    }
    None
}

/// Touches a game (carries and targets) over his last healthy games with
/// you; None before he has played.
fn touches(w: &World, id: u32) -> Option<f64> {
    let gs: Vec<seen::Game> = seen::games(w, id, w.today.add(-seen::SEASON_DAYS), w.today, None);
    let last: Vec<&seen::Game> = gs.iter().rev().filter(|g| !g.hurt && g.line.is_some()).take(TOUCH_GAMES).collect();
    (!last.is_empty()).then(|| last.iter().map(|g| g.touches() as f64).sum::<f64>() / last.len() as f64)
}

/// Your record as said ("6-2", "six and two", "undefeated", "winless")
/// against your record this season.
fn record((won, lost): (u32, u32), v: &str) -> Truth {
    if has(v, &["undefeated", "unbeaten", "perfect"]) {
        return said(true, lost == 0 && won > 0);
    }
    if has(v, &["winless", "haven't won", "no wins"]) {
        return said(true, won == 0 && lost > 0);
    }
    let nums: Vec<f64> = v.split(|c: char| !c.is_ascii_alphanumeric()).filter_map(number).take(2).collect();
    match nums[..] {
        [a, b] => said(true, a as u32 == won && b as u32 == lost),
        _ => Truth::Unknown,
    }
}
