//! His own way of talking (talk replies item 8, agreed by Cain
//! 2026-11-19): habits drawn once from his makeup, age and what he is, and
//! kept with him (the same keyed draw every time, so the same man talks
//! the same way in every talk). How he addresses you, how long he talks, a
//! way or two of putting things, and a phrase of his own. Before this 16
//! of 17 replies opened "Coach,", four said "I appreciate" and four echoed
//! your words back as a question: nothing gave a person his own voice.
//!
//! These are habits for the voice to play, never lines to paste. Every
//! size and pool here is Claude's, proposed, not confirmed.

use super::Them;
use super::makeup::{self, Makeup};
use super::wants::Kind;
use crate::calendar::born;
use crate::world::World;

/// His habits in words for the voice's card, in the second person.
pub fn habits(w: &World, him: Them, coach: &str) -> Vec<String> {
    let kind = makeup::kind(w, him);
    let m = makeup::makeup(w, him);
    let age = age(w, him);
    let key = makeup::key(him);
    let roll = |salt: u64| crate::world::folk::roll(w, key ^ 0x5EEC_4000, salt);
    let z = Makeup::z;
    let mut out = Vec::new();
    out.push(address(kind, &m, age, coach, roll(1)));
    out.push(length(&m, roll(2)).into());
    // One or two ways of putting things, by his makeup, each drawn.
    let mut ways: Vec<(&str, f64)> = vec![
        ("you open straight on the point, with no warm-up", 0.3 + 0.4 * z(m.temper) - 0.2 * z(m.warmth)),
        ("you think out loud a little before you land on what you mean", 0.25 + 0.3 * z(m.iq) - 0.2 * z(m.temper)),
        ("you use plain, short words", 0.3 - 0.3 * z(m.professionalism) - 0.2 * z(m.iq)),
        ("you talk football the way coaches do, in its own words", 0.15 + 0.5 * z(m.iq)),
        ("you have a dry sense of humour and let it show now and then", 0.2 + 0.3 * z(m.warmth) + 0.2 * z(m.ego)),
        ("you call things as you see them, bluntly", 0.2 + 0.4 * z(m.temper) - 0.3 * z(m.professionalism)),
        ("you soften a hard thing before you say it", 0.2 + 0.5 * z(m.warmth) - 0.3 * z(m.temper)),
        ("you bring things back to the people around you", 0.2 + 0.4 * z(m.warmth) + 0.2 * z(m.loyalty)),
        ("you speak in specifics more than feelings", 0.2 + 0.4 * z(m.professionalism)),
        ("you talk about yourself and what you've done more than most", 0.1 + 0.6 * z(m.ego)),
        ("you keep your cards close and say less than you know", 0.15 - 0.4 * z(m.honesty) - 0.2 * z(m.warmth)),
    ];
    for (k, x) in ways.iter_mut().enumerate() {
        x.1 = x.1.max(0.02) * (0.4 + roll(10 + k as u64));
    }
    ways.sort_by(|a, b| b.1.total_cmp(&a.1));
    let n = if roll(3) < 0.5 { 1 } else { 2 };
    out.extend(ways.iter().take(n).map(|x| x.0.to_string()));
    if let Some(p) = phrase(kind, &m, age, roll(4), roll(5)) {
        out.push(format!("a phrase of your own you use now and then, never twice in a row: \"{p}\""));
    }
    out
}

/// How old he is, when the world knows it.
fn age(w: &World, him: Them) -> u32 {
    match him {
        Them::Player(id) => {
            crate::world::folk::person(w, id).map_or(19, |i| born::age_or(w.people[i].born, w.today, 19))
        }
        Them::Coach(c) => w.coaches.get(c).map_or(45, |co| born::age_or(co.born, w.today, co.age)),
        Them::Kin(..) => 45,
        _ => 50,
    }
}

/// How he addresses you: Coach, Coach and your name, sir, or nothing at all
/// (most of the time he just talks).
fn address(kind: Kind, m: &Makeup, age: u32, coach: &str, r: f64) -> String {
    let z = Makeup::z;
    let surname = coach.split_whitespace().last().unwrap_or(coach);
    // "Coach Coach" for a coach whose last name is Coach (the test coach).
    let surname = if surname.eq_ignore_ascii_case("coach") { "" } else { surname };
    // Shares of each way, by who he is: (Coach, Coach + name, sir, nothing).
    let (a, b, c, d) = match kind {
        Kind::Player | Kind::Recruit | Kind::Transfer | Kind::Prospect => {
            let formal = 0.15 + 0.25 * z(m.professionalism).max(0.0);
            (0.45, 0.1, formal, 0.3)
        }
        Kind::Family => (0.4, 0.3, 0.05, 0.25),
        Kind::Staff | Kind::OtherCoach | Kind::HsCoach => {
            if age >= 50 {
                (0.25, 0.1, 0.0, 0.65)
            } else {
                (0.35, 0.15, 0.05, 0.45)
            }
        }
        Kind::Boss | Kind::Gm | Kind::Agent | Kind::Booster => (0.15, 0.1, 0.0, 0.75),
        _ => (0.3, 0.15, 0.05, 0.5),
    };
    let t = r * (a + b + c + d);
    let way = if t < a || (t < a + b && surname.is_empty()) {
        "\"Coach\"".to_string()
    } else if t < a + b {
        format!("\"Coach {surname}\"")
    } else if t < a + b + c {
        "\"sir\"".to_string()
    } else {
        return "you address the coach by no title at all: you just talk to him".into();
    };
    format!("you call the coach {way}, but only now and then, never at the start of every line")
}

/// How long he talks.
fn length(m: &Makeup, r: f64) -> &'static str {
    let z = Makeup::z;
    let x = 0.5 * z(m.warmth) + 0.4 * z(m.ego) - 0.4 * z(m.temper) + (r - 0.5);
    if x < -0.3 {
        "you're a man of few words: short answers, nothing extra"
    } else if x > 0.35 {
        "you talk freely and at some length when something matters to you"
    } else {
        "you say what you need to and stop"
    }
}

/// A phrase of his own, by his age and what he is, or none.
fn phrase(kind: Kind, m: &Makeup, age: u32, r: f64, pick: f64) -> Option<&'static str> {
    if r < 0.3 {
        return None;
    }
    let young = matches!(kind, Kind::Player | Kind::Recruit | Kind::Transfer | Kind::Prospect) && age < 24;
    let pool: &[&str] = if young {
        &["for real", "straight up", "I'm locked in", "on everything", "no doubt", "that's facts"]
    } else if Makeup::z(m.professionalism) > 0.3 {
        &["bottom line", "the way I see it", "here's the thing", "to be fair to everybody"]
    } else {
        &["look", "I'll tell you what", "at the end of the day", "I'll be honest with you", "here's where I'm at"]
    };
    Some(pool[((pick * pool.len() as f64) as usize).min(pool.len() - 1)])
}
