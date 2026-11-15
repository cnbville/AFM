//! Officials on game day (6.12: "ask why, argue, and pay for it with a
//! flag. You can't talk a call back, only challenge it"). Between plays you
//! can go to the official about the last flag or close call against you.
//! He tells you what he called, never that he missed it. How you went at
//! him and how tight his crew calls personal fouls (`game::sim::Crew`,
//! `personal`, about 0.6 to 1.5) decide whether he warns you or flags your
//! sideline: an unsportsmanlike foul, 15 yards (`game::live::Input::
//! BenchFlag`). An official warns once before he flags an argument; abuse
//! he flags at once, more readily after a warning.
//!
//! Every size is proposed, not confirmed.

use super::super::ear::{Act, How, Reading};

/// How you went at him.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Went {
    /// You asked what he called, or said nothing hard.
    Ask,
    /// You argued the call: he heard doubt or a sting.
    Argue,
    /// You insulted or threatened him.
    Abuse,
}

/// How a line went at him, from the ear's reading.
pub fn went(r: &Reading) -> Went {
    if r.hits.iter().any(|h| matches!(h.how, How::Insulted | How::Threatened))
        || r.acts.iter().any(|a| matches!(a, Act::Threat { .. }))
    {
        Went::Abuse
    } else if r.hits.iter().any(|h| matches!(h.how, How::Doubted | How::Stung)) {
        Went::Argue
    } else {
        Went::Ask
    }
}

/// What he does about it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ruling {
    /// He answers and walks off.
    Answers,
    /// "That's enough, Coach."
    Warns,
    /// The flag comes out: 15 yards on your sideline.
    Flags,
}

/// The odds he flags you: an argument after a warning 0.25, abuse 0.55
/// (0.8 after a warning), each times his crew's strictness on personal
/// fouls.
pub fn flag_odds(went: Went, warned: u8, strict: f64) -> f64 {
    let base = match (went, warned) {
        (Went::Ask, _) => 0.0,
        (Went::Argue, 0) => 0.0,
        (Went::Argue, _) => 0.25,
        (Went::Abuse, 0) => 0.55,
        (Went::Abuse, _) => 0.8,
    };
    (base * strict.clamp(0.5, 1.6)).clamp(0.0, 0.95)
}

/// What he does, with `u` a keyed draw (0-1) for this stop.
pub fn rule(went: Went, warned: u8, strict: f64, u: f64) -> Ruling {
    if u < flag_odds(went, warned, strict) {
        Ruling::Flags
    } else if went != Went::Ask {
        Ruling::Warns
    } else {
        Ruling::Answers
    }
}

/// His words for what he does, for the voice.
pub fn words(r: Ruling) -> &'static str {
    match r {
        Ruling::Answers => "He answers and goes back to his spot.",
        Ruling::Warns => "He warns you: that's enough, and the next one is a flag.",
        Ruling::Flags => "He throws his flag: unsportsmanlike conduct on your sideline, 15 yards.",
    }
}
