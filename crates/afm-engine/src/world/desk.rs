//! Your desk (section 10, topics 1, 3, 5 and 8; the final build's inbox
//! 10a-10i and "my players"): the mail you keep for 30 days, what you read,
//! saved and answered, your notes on anything with the page each was
//! written on, your "my players" list with the first look your staff took,
//! and searches you saved as watches.
//!
//! The screens write it; nothing in the world reads it. An answer you pick
//! acts through the world's own fields (a player's mood and trust), and the
//! game crate says in words what it did.

use crate::calendar::Date;

/// The letterhead a mail is printed on (10a).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Head {
    /// Your university's athletic department (or a high school district).
    School,
    /// A conference or state association office.
    Conference,
    /// The NCAA or the NFL's league office.
    League,
    /// Your NFL club.
    Club,
    /// A player agency.
    Agency,
    /// A newspaper's masthead: the morning clips.
    Paper,
    /// A person's own plain email.
    Email,
    /// An internal staff memo.
    Memo,
}

/// One mail. Text fields carry the link markup the screens draw
/// (`[[p:12|Name]]` and the like, see the game crate's `lane_core::links`).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Mail {
    /// Stable id: the same mail is never filed twice.
    pub id: u64,
    pub date: Date,
    /// Minutes after midnight it arrived.
    pub minute: u16,
    pub head: Head,
    /// The letterhead's organisation and its second line (an address, a
    /// department, a section).
    pub org: String,
    pub org_line: String,
    /// Sender's name and role, and who it is to.
    pub from: String,
    pub role: String,
    pub to: String,
    pub subject: String,
    pub body: Vec<String>,
    /// A table in the letter: column names, then rows.
    #[serde(default)]
    pub table: Vec<Vec<String>>,
    #[serde(default)]
    pub attach: Vec<String>,
    pub sign: String,
    /// The keywords (10e) as bits, see the game crate's `KEYWORDS`.
    pub keys: u16,
    /// The page the mail is about (opened beside it): kind and id.
    #[serde(default)]
    pub about: Option<(u8, u32)>,
    /// Picked answers (10i): two or three, with what each does.
    #[serde(default)]
    pub answers: Vec<Answer>,
    #[serde(default)]
    pub answered: Option<u8>,
    #[serde(default)]
    pub answer_by: Option<Date>,
    pub read: bool,
    pub saved: bool,
    /// How loud the moment behind it was (0 small to 3 huge).
    pub loud: u8,
    /// The players it names (an answer may act on all of them).
    #[serde(default)]
    pub ids: Vec<u32>,
    /// The team whose colours print the letterhead (the game crate's code).
    #[serde(default)]
    pub org_team: Option<u32>,
    /// Why it happened, as your staff sees it (update 3, H6): the two or
    /// three biggest reasons, linked text.
    #[serde(default)]
    pub why: Vec<String>,
    /// How many entries a compound message holds (the list shows it as a
    /// badge when it is more than one): the day's 212 portal entrants, of
    /// which the table names the few that matter.
    #[serde(default)]
    pub count: u32,
    /// The analyst's weekly leads (live talks lane 4): 0 for any other mail,
    /// 1 when the writer's part is wanted (the game's own table stands until
    /// you open it with a key), 2 once it is written.
    #[serde(default)]
    pub leads: u8,
}

/// One picked answer: its words, what it will do, and the act behind it.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Answer {
    pub label: String,
    /// What it does, in words, shown before you pick and kept after.
    pub does: String,
    /// The act (see the game crate's `mail::Act`) and its target id.
    pub act: u8,
    pub target: u32,
}

/// A note you wrote about anything but a single player (players' notes stay
/// on `seat::Career::notes`): a coach, a team, a game, a mail, or nothing.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct DeskNote {
    pub date: Date,
    pub text: String,
    /// The page it was written on ("Inbox · Spring practice plan").
    pub page: String,
    /// What it is about: kind (see the game crate) and id, with its name.
    pub about: Option<(u8, u32)>,
    pub about_name: String,
}

/// A player on your "my players" list (scouting topic 18), with what your
/// staff and the public thought of him the day he went on it.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Listed {
    pub id: u32,
    pub since: Date,
    /// Your staff's read of him then (low, high), 0 when not seen.
    pub read: (u8, u8),
    /// His public stars then.
    pub stars: u8,
    /// How he got on it: 0 you added him, 1 a watch found him, 2 an answer.
    pub how: u8,
}

/// A saved search that watches for new players (screens topic 3).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Watch {
    pub name: String,
    /// The search as the screens wrote it (JSON).
    pub query: String,
    pub since: Date,
    /// Players already matched and told.
    pub seen: Vec<u32>,
    /// The players its last mail named, until you open the search (a
    /// saved search of the find pages, update 2 lane 7).
    #[serde(default)]
    pub fresh: Vec<u32>,
}

/// Contacts you made from the inbox (congratulations, calls), for the
/// people who keep track of ties: who, what and when.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Contact {
    /// 0 a player (id), 1 a coach (index).
    pub kind: u8,
    pub id: u32,
    pub date: Date,
    pub what: String,
}

/// A reveal waiting to be shown (screens topic 9). The first nine fields
/// are the original pop-up; the rest dress the big-moment show (the hat
/// table, the quiet reveals, the facts under the name) and are empty in a
/// save from before it.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Reveal {
    pub date: Date,
    /// commit, signing, miss, hire, fired, title, record, hall, first.
    pub kind: String,
    pub kicker: String,
    pub number: String,
    pub name: String,
    pub sub: String,
    pub verdict: String,
    /// The page it opens: kind and id.
    pub about: Option<(u8, u32)>,
    pub colors: [String; 2],
    /// The short token in the verdict box ("QB1", "1st", "18 yrs"); empty
    /// for none. Always words or a rank, never a potential number.
    #[serde(default)]
    pub token: String,
    /// A quiet reveal (a miss, a firing): dark room, no rays.
    #[serde(default)]
    pub quiet: bool,
    /// The small facts under the verdict: label, then the value in bold.
    #[serde(default)]
    pub facts: Vec<(String, String)>,
    /// Cards under the verdict: a heading and its lines (a why, a quote
    /// with its speaker as the heading).
    #[serde(default)]
    pub cards: Vec<(String, Vec<String>)>,
    /// The hat table: a recruit's finalists (name and colours); `chosen`
    /// is the cap he picks up. Empty: no ceremony.
    #[serde(default)]
    pub hats: Vec<(String, [String; 2])>,
    #[serde(default)]
    pub chosen: usize,
    /// The room the ceremony is held in (his high school's gym).
    #[serde(default)]
    pub room: String,
    /// A record's old and new marks: label, value text, share of the new.
    #[serde(default)]
    pub bars: Vec<(String, String, f32)>,
    /// A career laid out in steps: age or year, then what happened.
    #[serde(default)]
    pub steps: Vec<(String, String)>,
    /// The other page it can open, with the label on its button.
    #[serde(default)]
    pub also: Option<(String, u8, u32)>,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Desk {
    /// Kept mail, oldest first.
    pub mails: Vec<Mail>,
    /// Mail you deleted: its id and date, forgotten once it would have cleared.
    pub gone: Vec<(u64, Date)>,
    /// The day the inbox was last filed through.
    pub through: Option<Date>,
    pub notes: Vec<DeskNote>,
    pub listed: Vec<Listed>,
    /// Players the list adds by itself that you took off it.
    pub unlisted: Vec<u32>,
    pub watches: Vec<Watch>,
    pub contacts: Vec<Contact>,
    /// Reveals still to show.
    pub reveals: Vec<Reveal>,
    /// Reveals already made (ids), so none is shown twice.
    pub revealed: Vec<u64>,
    /// The reveals already shown, newest last (capped), so one can be
    /// played again from the page that holds it.
    #[serde(default)]
    pub shown: Vec<Reveal>,
    /// The seat the "You're hired" reveal last played for (a team code),
    /// so a new job gets its own.
    #[serde(default)]
    pub hired_for: Option<u32>,
    /// Players whose first NFL start the desk already told (a player you
    /// wrote a note on at 14 to 16).
    #[serde(default)]
    pub first_told: Vec<u32>,
    /// Your former players whose landing a "Portal landings" message
    /// told, and the day it was filed, so none is told twice.
    #[serde(default)]
    pub landed: Vec<(u32, Date)>,
}

/// Days mail is kept (10d): it clears the morning it turns 31 days old.
pub const KEEP_DAYS: i32 = 30;

impl Desk {
    /// Clears mail older than 30 days (saved mail stays) and forgets old
    /// deletions.
    pub fn clear_old(&mut self, today: Date) {
        self.mails.retain(|m| m.saved || m.date.days_to(today) <= KEEP_DAYS);
        self.gone.retain(|g| g.1.days_to(today) <= KEEP_DAYS + 1);
        if self.revealed.len() > 400 {
            let cut = self.revealed.len() - 400;
            self.revealed.drain(..cut);
        }
        if self.landed.len() > 600 {
            let cut = self.landed.len() - 600;
            self.landed.drain(..cut);
        }
        if self.contacts.len() > 2000 {
            let cut = self.contacts.len() - 2000;
            self.contacts.drain(..cut);
        }
    }

    /// Whether a mail id is filed or was deleted.
    pub fn has(&self, id: u64) -> bool {
        self.mails.iter().any(|m| m.id == id) || self.gone.iter().any(|g| g.0 == id)
    }

    pub fn unread(&self) -> usize {
        self.mails.iter().filter(|m| !m.read).count()
    }

    pub fn is_listed(&self, id: u32) -> bool {
        self.listed.iter().any(|l| l.id == id)
    }
}
