//! Words for the values a point carries: a day, a stat line, a mistake.
//! Everything here is something a person would say out loud and could
//! check: a box score, a weekday, a count.

use crate::calendar::Date;
use crate::game::stats::StatLine;
use crate::positions::Position;
use crate::world::folk::{count_words, noun};

const WEEKDAYS: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// "March 3".
pub fn date_words(d: Date) -> String {
    let (_, m, day) = d.parts();
    format!("{} {day}", MONTHS[m as usize - 1])
}

/// When a past day was, as someone would say it today: "today",
/// "yesterday", "on Saturday" (inside a week), "last week", then "on
/// March 3". A day ahead reads "on Saturday" inside a week, else its date.
pub fn when_words(d: Date, today: Date) -> String {
    let age = d.days_to(today);
    match age {
        0 => "today".into(),
        1 => "yesterday".into(),
        -1 => "tomorrow".into(),
        2..=6 | -6..=-2 => format!("on {}", WEEKDAYS[d.weekday().index() as usize]),
        7..=13 => "last week".into(),
        _ => format!("on {}", date_words(d)),
    }
}

/// A position as people say it.
pub fn spot(p: Position) -> &'static str {
    use Position as P;
    match p {
        P::QB => "quarterback",
        P::RB => "running back",
        P::WR => "receiver",
        P::TE => "tight end",
        P::OT => "tackle",
        P::IOL => "guard",
        P::EDGE => "defensive end",
        P::DT => "defensive tackle",
        P::LB => "linebacker",
        P::CB => "corner",
        P::S => "safety",
        P::K => "kicker",
        P::P => "punter",
        P::LS => "long snapper",
    }
}

/// Whether a position plays on offense.
pub fn on_offense(p: Position) -> bool {
    matches!(p, Position::QB | Position::RB | Position::WR | Position::TE | Position::OT | Position::IOL)
}

/// A first name.
pub fn first(full: &str) -> String {
    full.split_whitespace().next().unwrap_or(full).to_string()
}

/// "Ann", "Ann and Bo", "Ann, Bo and Cy".
pub fn list(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [a] => a.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// A count with its noun in words up to twelve: "one game", "three
/// games", "14 games".
pub fn counted(n: u32, one: &str) -> String {
    format!("{} {}", count_words(n as usize), noun(n, one))
}

/// "a touchdown", "two touchdowns".
fn some(n: u32, one: &str) -> String {
    if n == 1 { format!("{} {one}", crate::world::folk::a_an(one)) } else { counted(n, one) }
}

fn yards(y: f64) -> String {
    let y = y.round() as i64;
    if y == 1 { "1 yard".into() } else { format!("{y} yards") }
}

/// His game as the box score has it, by what his position does: "22 of
/// 31 for 284 yards and three touchdowns", "132 yards on 21 carries", "7
/// catches for 112 yards", "9 tackles and two sacks". None when the box
/// says nothing worth naming (a lineman, a specialist, a man who barely
/// played).
pub fn stat_line(pos: Position, s: &StatLine) -> Option<String> {
    use Position as P;
    let mut out = match pos {
        P::QB if s.att >= 5 => format!("{} of {} for {}", s.cmp, s.att, yards(s.pass_yds)),
        P::QB | P::RB if s.rush >= 5 => format!("{} on {}", yards(s.rush_yds), counted(s.rush, "carry")),
        P::WR | P::TE | P::RB if s.rec >= 2 => format!("{} for {}", counted(s.rec, "catch"), yards(s.rec_yds)),
        P::EDGE | P::DT | P::LB | P::CB | P::S if s.tackles + s.assists >= 2 => {
            counted(s.tackles + s.assists, "tackle")
        }
        _ => return None,
    };
    let mut extra: Vec<String> = Vec::new();
    let td = match pos {
        P::QB => s.pass_td + s.rush_td,
        _ => s.rush_td + s.rec_td,
    };
    if td > 0 && !matches!(pos, P::EDGE | P::DT | P::LB | P::CB | P::S) {
        extra.push(some(td, "touchdown"));
    }
    if matches!(pos, P::EDGE | P::DT | P::LB | P::CB | P::S) {
        let sacks = s.sacks.floor() as u32;
        if sacks > 0 {
            extra.push(some(sacks, "sack"));
        }
        if s.ints > 0 {
            extra.push(some(s.ints, "interception"));
        }
    }
    if !extra.is_empty() {
        out.push_str(" and ");
        out.push_str(&extra.join(" and "));
    }
    Some(out)
}

/// The costly part of his game, when the box has one: "two
/// interceptions", "a lost fumble", "three drops".
pub fn mistake(s: &StatLine) -> Option<String> {
    if s.int > 0 {
        Some(some(s.int, "interception"))
    } else if s.fumbles_lost > 0 {
        Some(if s.fumbles_lost == 1 { "a lost fumble".into() } else { counted(s.fumbles_lost, "lost fumble") })
    } else if s.drops >= 2 {
        Some(counted(s.drops, "drop"))
    } else if s.penalties >= 2 {
        Some(counted(s.penalties, "penalty"))
    } else if s.missed >= 3 {
        Some(counted(s.missed, "missed tackle"))
    } else {
        None
    }
}

/// How many giveaways and flags the box charges him with.
pub fn mistakes(s: &StatLine) -> u32 {
    s.int + s.fumbles_lost
}

/// A draft round as a word: "first", "second" up to "seventh".
pub fn round_word(round: u32) -> &'static str {
    ["first", "second", "third", "fourth", "fifth", "sixth", "seventh"]
        .get(round.saturating_sub(1) as usize)
        .copied()
        .unwrap_or("late")
}

/// The round of an overall pick (1-257), by 32 a round.
pub fn round_of(pick: u32) -> u32 {
    ((pick.max(1) - 1) / 32 + 1).min(7)
}
