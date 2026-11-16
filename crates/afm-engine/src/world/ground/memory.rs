//! One memory per person and holder (GR62): what is between a person and
//! a program or a coach. Lane G1b keeps standing and its why, the contacts
//! that built it, how his kids fared with you, your word on promises and
//! pulled offers, and up to five moments in his words. Lane G7 moves the
//! other stores that track what a person thinks of you (contacts' warmth
//! and favors, the high school coach's credit, promises' trust) onto it.

use super::people::{Holder, Who};
use crate::calendar::Date;

/// Moments a person keeps (GR62: about five).
pub const MOMENTS: usize = 5;

/// One memory.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Memory {
    pub who: Who,
    pub holder: Holder,
    /// Standing points, 0-100 (Stranger 0-24, Known 25-49, Trusted 50-74,
    /// Home 75-100).
    pub points: f32,
    /// The day they first dealt with each other.
    pub met: Date,
    /// The day standing first reached Known (the Trusted gate counts from it).
    pub known_since: Option<Date>,
    /// The last contact (fade counts from it) and the last few, newest last.
    pub last: Date,
    pub recent: Vec<Date>,
    /// Contacts in all (the Known gate needs two).
    pub contacts: u16,
    /// This calendar year's visits and calls, for the why: (year, visits, calls).
    pub year: (i32, u16, u16),
    /// Contacts this week (a second visit buys half): (week, how many).
    pub week: (i32, u8),
    /// The last time your word broke with him (a promise, a pulled offer):
    /// Trusted needs a year since Known without one.
    pub broken: Option<Date>,
    /// A kid of his did well with you (started, graduated, drafted): Home
    /// in three years instead of five.
    pub did_well: bool,
    /// How his kids fared with you: (player id, surname, how), newest last.
    pub kids: Vec<(u32, String, Fared)>,
    /// Where it started, for the why.
    pub roots: Roots,
    /// The moments he keeps, in his words.
    pub moments: Vec<Moment>,
    /// The level last warned about (0 none): one warning per slide.
    pub warned: u8,
}

/// How one of his kids fared with you.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Fared {
    Started,
    Graduated,
    Drafted,
    /// Buried after you said he'd play (a playing-time promise broken).
    Buried,
}

impl Fared {
    pub fn words(self) -> &'static str {
        match self {
            Fared::Started => "started for you",
            Fared::Graduated => "graduated with you",
            Fared::Drafted => "was drafted from your program",
            Fared::Buried => "was buried after you said he'd play",
        }
    }
}

/// Where a memory started (several can hold).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Roots {
    /// Day one from the program's pipelines and name in his area.
    pub program: bool,
    /// Day one from the coach's own recruiting tie to his school.
    pub tie: bool,
    /// Your hometown (GR7).
    pub hometown: bool,
    /// Your own high school (GR7).
    pub own_school: bool,
    /// A school you coached at (GR7).
    pub old_job: bool,
}

/// A moment he keeps.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Moment {
    pub date: Date,
    /// His own words.
    pub words: String,
    pub good: bool,
    /// A big one holds at full weight for years (GR62).
    pub big: bool,
    /// While a big bad moment holds, being there buys a quarter.
    pub holds_to: Option<Date>,
}

impl Memory {
    /// A first memory, from a starting number of points.
    pub fn new(who: Who, holder: Holder, points: f32, met: Date) -> Memory {
        Memory {
            who,
            holder,
            points,
            met,
            known_since: None,
            last: met,
            recent: Vec::new(),
            contacts: 0,
            year: (met.parts().0, 0, 0),
            week: (met.0.div_euclid(7), 0),
            broken: None,
            did_well: false,
            kids: Vec::new(),
            roots: Roots::default(),
            moments: Vec::new(),
            warned: 0,
        }
    }

    /// Keeps a moment: the five he holds are the big ones first, then the
    /// newest.
    pub fn keep(&mut self, m: Moment) {
        self.moments.push(m);
        while self.moments.len() > MOMENTS {
            let drop =
                self.moments.iter().enumerate().min_by_key(|(_, x)| (x.big, x.date)).map(|(i, _)| i).unwrap_or(0);
            self.moments.remove(drop);
        }
    }

    /// A big bad moment he still holds on this day.
    pub fn holding(&self, d: Date) -> Option<&Moment> {
        self.moments.iter().filter(|m| m.big && !m.good && m.holds_to.is_some_and(|t| t > d)).max_by_key(|m| m.date)
    }

    /// Contacts in the 30 days before this one.
    pub fn month_contacts(&self, d: Date) -> usize {
        self.recent.iter().filter(|&&x| x > d.add(-30) && x <= d).count()
    }

    pub fn kid(&mut self, id: u32, name: &str, how: Fared) {
        if self.kids.iter().any(|k| k.0 == id && k.2 == how) {
            return;
        }
        self.kids.push((id, name.to_string(), how));
        if self.kids.len() > 12 {
            self.kids.remove(0);
        }
    }
}
