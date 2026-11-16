//! One memory per person and holder (GR62): what is between a person and
//! a program or a coach. It keeps standing and its why, the contacts that
//! built it, how his kids fared with you, your word on promises and pulled
//! offers, up to five moments in his words (lane G1b), and (lane G7) the
//! favors either way in words, his word's record with you and yours with
//! him, a staff's credit for a high school coach's word, what reached him
//! through others and where he worked when you last looked. The other
//! stores that tracked what a person thinks
//! of you read through it (`stores`).

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
    /// Favors either way (GR63, in words on every screen): + he owes you.
    /// For a ground person it is the network's balance (`stores`).
    #[serde(default)]
    pub owes: i8,
    /// The favors behind it, newest last (`FAVORS` at most).
    #[serde(default)]
    pub favors: Vec<Favor>,
    /// His word with you: tips settled, and how many held up ("4 of his 6
    /// kids held up").
    #[serde(default)]
    pub his_word: (u8, u8),
    /// Your word with him: your calls on kids settled, and how many held up.
    #[serde(default)]
    pub your_word: (u8, u8),
    /// The holder's credit for his word, 0-1: a college staff's read of a
    /// high school coach who calls about his players (`scouting/hs.rs`).
    #[serde(default)]
    pub credit: Option<f32>,
    /// Where he worked when you last looked (`people::post_key`), so a move
    /// shows (0 not looked yet).
    #[serde(default)]
    pub at: u32,
    /// How he came to know of you when nobody called: "Coach Hill told him
    /// about you", "he heard what you did for Reyes".
    #[serde(default)]
    pub heard: Option<String>,
    /// The network's values for him the last time they were folded in
    /// (`stores::sync`): warmth, favors, his record.
    #[serde(default)]
    pub(crate) seen: Option<(f32, i8, (u8, u8))>,
}

/// Favors a memory keeps in words.
pub const FAVORS: usize = 6;

/// One favor, in words.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Favor {
    pub date: Date,
    pub words: String,
    /// He did it for you (you owe him), or you for him.
    pub his: bool,
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
#[serde(default)]
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
    /// A family that knew you through an older brother, or a cousin's.
    pub family: bool,
    /// Your coaching tree: your mentor, or a man who learned under you.
    pub tree: bool,
    /// A former player of yours, or an old teammate.
    pub old_player: bool,
    /// Your own network (a tie from before Ground).
    pub network: bool,
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
            owes: 0,
            favors: Vec::new(),
            his_word: (0, 0),
            your_word: (0, 0),
            credit: None,
            at: 0,
            heard: None,
            seen: None,
        }
    }

    /// A favor either way (+1 he owes you one, -1 you owe him), held to five
    /// either way like the network's balance.
    pub fn favor(&mut self, by: i8, words: String, d: Date) {
        self.owes = self.owes.saturating_add(by).clamp(-5, 5);
        if by != 0 && !words.is_empty() {
            self.favors.push(Favor { date: d, words, his: by < 0 });
            if self.favors.len() > FAVORS {
                self.favors.remove(0);
            }
        }
    }

    /// The favors in words, never numbers (GR63).
    pub fn favor_words(&self) -> Option<String> {
        match self.owes {
            0 => None,
            1 => Some("he owes you one".into()),
            2..=3 => Some("he owes you a big one".into()),
            x if x > 3 => Some("he owes you more than he can repay".into()),
            -1 => Some("you owe him one".into()),
            -3..=-2 => Some("you owe him a big one".into()),
            _ => Some("you owe him more than you can repay".into()),
        }
    }

    /// His record with you in words ("4 of his 6 tips held up").
    pub fn his_word_words(&self) -> Option<String> {
        let (n, held) = self.his_word;
        (n > 0).then(|| format!("{held} of his {n} {} held up", if n == 1 { "tip" } else { "tips" }))
    }

    /// Your record with him in words ("4 of 5 held up").
    pub fn your_word_words(&self) -> Option<String> {
        let (n, held) = self.your_word;
        (n > 0).then(|| format!("your word held up {held} of {n} {}", if n == 1 { "time" } else { "times" }))
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
