//! Live talks v3, lane N5e: scouting and the draft
//! (`docs/design/live-talks-persona.md`, sections 6.7, 6.11 and question
//! 10). Prospects, the people around them and your own scouting people,
//! played as themselves on lane N4's conversation, and the systems that
//! question 10 brought back, each giving real information or a real action:
//!
//! - `room`: the combine interview, the top-30 visit and the official
//!   visit's sit-down play as v3 talks; each typed question spends the
//!   room's clock on the topic it is about, so the file reads it as the
//!   picked topic would.
//! - `whiteboard`: he explains a play at his spot; it reads his Football IQ
//!   and his play recognition into your file.
//! - `coached`: agents and training camps prep prospects for the stock
//!   questions; a coached answer reads better than the truth until you
//!   press past the script.
//! - `remember`: a prospect remembers how you treated him: in his next
//!   room, in the undrafted rush, and when he joins you.
//! - `facet`: who you talk to on the draft side (his family, your
//!   director of security, your scouts, your pro scouts, another club's
//!   GM) and what each knows (`facet::Facet`, over lane N1's people).
//! - `scouts`: sending a scout, your security man or a pro scout to find
//!   something out; his answer comes back as a letter and in your file.
//! - `debate`: on the clock, your scouts argue for their men from their
//!   own reads.
//! - `smoke`: reading other clubs' smoke from what your staff can see, and
//!   planting your own on calls.
//! - `scene`: the talks without a key (their pick lists) and how each
//!   opens.
//!
//! Every size here is Claude's pick, proposed, not confirmed. Only your
//! seat changes: an AI world with no talks plays the same.

pub mod coached;
pub mod debate;
pub mod facet;
pub mod remember;
pub mod room;
pub mod scene;
pub mod scouts;
pub mod smoke;
pub mod whiteboard;

use super::Them;
use super::wants::Kind as PersonKind;
use crate::calendar::Date;
use crate::world::World;

/// A draft-side talk (`scene::Kind::DraftTalk`): who it is with. Kept in
/// the scene's `b` (the kind) and `c` (the index); `a` is the player it is
/// about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sub {
    /// The prospect's family, by phone (his decider first).
    Family,
    /// Your club's director of security.
    Security,
    /// One of your club's college scouts (his place in the department).
    Scout(usize),
    /// One of your pro scouts (his place in the pro department), about
    /// another club's player.
    ProScout(usize),
    /// Your draft room on the clock.
    Room,
    /// Another club's general manager on the phone (the club).
    Gm(usize),
    /// The draft-day call to a man: the one you took, or one you passed
    /// on who thought you'd take him.
    Call,
}

impl Sub {
    pub fn code(self) -> (u64, i64) {
        match self {
            Sub::Family => (0, 0),
            Sub::Security => (1, 0),
            Sub::Scout(k) => (2, k as i64),
            Sub::ProScout(k) => (3, k as i64),
            Sub::Room => (4, 0),
            Sub::Gm(t) => (5, t as i64),
            Sub::Call => (6, 0),
        }
    }
    pub fn of(b: u64, c: i64) -> Option<Sub> {
        let c = c.max(0) as usize;
        Some(match b & 0xF {
            0 => Sub::Family,
            1 => Sub::Security,
            2 => Sub::Scout(c),
            3 => Sub::ProScout(c),
            4 => Sub::Room,
            5 => Sub::Gm(c),
            6 => Sub::Call,
            _ => return None,
        })
    }
}

/// Keys for the people the world doesn't store as people (`Them::Drawn`).
pub const SECURITY_KEY: u64 = 0x5EC0_0000;
pub const PRO_KEY: u64 = 0x9205_0000;

/// Your club's director of security.
pub fn security(t: usize) -> Them {
    Them::Drawn(PersonKind::Staff, SECURITY_KEY | t as u64)
}

/// One of your pro scouts (his place in `scouting.pro.scouts`).
pub fn pro_scout(k: usize) -> Them {
    Them::Drawn(PersonKind::Staff, PRO_KEY | k as u64)
}

/// One of your club's college scouts (his place in its department).
pub fn scout(t: usize, k: usize) -> Them {
    Them::ProScout(super::makeup::pro_scout_key(t, k))
}

/// Another club's general manager.
pub fn gm(w: &World, t: usize) -> Them {
    use crate::world::folk::Team;
    use crate::world::folk::cast::{self, CastRef, Role};
    let r = CastRef::new(Team::Nfl(t), Role::Gm, 0);
    Them::Cast(r, cast::get(w, r).term)
}

/// Which of yours a drawn person is: your security director, or your pro
/// scout k.
pub fn drawn_of(him: Them) -> Option<(bool, usize)> {
    let Them::Drawn(PersonKind::Staff, k) = him else { return None };
    if k & !0xFFFF == SECURITY_KEY {
        Some((true, (k & 0xFFFF) as usize))
    } else if k & !0xFFFF == PRO_KEY {
        Some((false, (k & 0xFFFF) as usize))
    } else {
        None
    }
}

/// Your NFL club, when your seat is one.
pub fn club(w: &World) -> Option<usize> {
    w.seat.filter(|s| s.level == crate::calendar::Level::Nfl).and_then(|s| s.nfl_team)
}

/// What the draft side keeps (in `Folk.live.persona.draft`). Empty and
/// unwritten until used, so old saves load and write the same.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Book {
    /// Questions you sent someone to find out (`scouts`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub questions: Vec<scouts::Question>,
    /// Whiteboard sessions in a typed room (`whiteboard`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub boards: Vec<whiteboard::Board>,
    /// How far you pressed past his script on a topic (`coached`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pressed: Vec<coached::Pressed>,
    /// What clubs told you on calls (`smoke`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub told: Vec<smoke::Told>,
    /// Smoke you planted on a call (`smoke`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub planted: Vec<smoke::Planted>,
    /// What your staff saw of each club's leaks once the draft showed it:
    /// (club, year, leaks shown real, shown smoke).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub record: Vec<(usize, i32, u8, u8)>,
    /// Men you passed on who thought you'd take him (`remember`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub passed: Vec<(u32, Date)>,
    /// Men whose word of you was carried into their trust when they joined
    /// (`remember`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub joined: Vec<u32>,
}

impl Book {
    pub fn is_empty(&self) -> bool {
        self.questions.is_empty()
            && self.boards.is_empty()
            && self.pressed.is_empty()
            && self.told.is_empty()
            && self.planted.is_empty()
            && self.record.is_empty()
            && self.passed.is_empty()
            && self.joined.is_empty()
    }
}

/// The most rows each list keeps (the oldest go first).
pub const KEEP: usize = 400;

fn trim<T>(v: &mut Vec<T>) {
    if v.len() > KEEP {
        let cut = v.len() - KEEP;
        v.drain(..cut);
    }
}

/// The draft side's day, for your seat (after the letters): questions
/// coming back, men who joined you.
pub(crate) fn day(w: &mut World, d: Date) {
    scouts::day(w, d);
    if d.weekday() == crate::calendar::Weekday::Mon {
        remember::week(w);
    }
    let b = &mut w.folk.live.persona.draft;
    trim(&mut b.questions);
    trim(&mut b.boards);
    trim(&mut b.pressed);
    trim(&mut b.told);
    trim(&mut b.planted);
    trim(&mut b.record);
    trim(&mut b.passed);
    trim(&mut b.joined);
}

/// After the draft, before the season's pre-draft store is cleared: what
/// the picks showed of each club's leaks and your smoke, and the men you
/// passed on.
pub(crate) fn after_draft(w: &mut World) {
    if w.seat.is_none() {
        return;
    }
    smoke::settle(w);
    remember::passed_on(w);
}

/// After each line of a v3 talk the game played (`talk::Conversation::apply`):
/// an interview's clock.
pub(crate) fn after_line(w: &mut World, id: u32, said: &super::talk::Said, t: &super::ear::Turn) {
    room::count(w, id, said, t);
    scene::after_line(w, id, said);
}

/// When a v3 talk ends on one of this lane's scenes: whether it was ours
/// (the room is filed, the scene closed).
pub(crate) fn end(w: &mut World, s: &mut crate::world::folk::scene::Scene) -> bool {
    use crate::world::folk::scene::Kind;
    match s.kind {
        Kind::CombineTalk => {
            crate::world::folk::dialogue::combine_close(w, s);
            true
        }
        Kind::DraftTalk => {
            scene::close(w, s);
            true
        }
        _ => false,
    }
}
