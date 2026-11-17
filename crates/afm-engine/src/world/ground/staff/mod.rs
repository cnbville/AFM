//! Your staff's ground and the assistant seat (lane G10: GR68, GR69;
//! `docs/design/ground.md`, part K; the round file `ground/staff.md`).
//!
//! - **Areas on the map** (`beats`): each on-field assistant works one or
//!   more of Ground's areas (GR1) plus his position group. Your recruiting
//!   coordinator proposes the split at the February meeting by each man's
//!   own ground (home ground first), and you move any of it any Monday. A
//!   big area can take two men, an empty one shows up as misses, and a young
//!   coach can ride along with a veteran and build ground of his own.
//! - **Each man's ground** (`men`): his people and his standing with them,
//!   his ties, his record and the kids credited to him, his style, his week
//!   and his mood.
//! - **Losing a man** (`carousel`): when another school's search reaches
//!   one of your men and he means to go, you get the warning first and a few
//!   days for a counter (a raise, a title or a promise, tracked). When he
//!   goes, his own ties go with him, his commits waver, some follow him, and
//!   he joins your coaching tree or runs you down.
//! - **Hiring for ground** (`men::signs`): a candidate's ground reads only
//!   in signs, fogged by your network.
//! - **The assistant seat** (`desk`): your area and your group, your head
//!   coach's road days, your list of two a week on the offer chain from the
//!   other side, his verdict and his trust in your eye, your finds, his
//!   memory of you, your goals and his August verdict.
//!
//! Only your seat's program keeps any of it, so a world with no career
//! plays the same on every seed. Every size here is the build's pick
//! unless the spec gave it, proposed, not confirmed.

pub mod beats;
pub mod carousel;
pub mod desk;
pub mod men;

use crate::calendar::{Date, Weekday};
use crate::world::World;
use crate::world::coach::{Employer, Role};

/// One man on one area (GR68).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Beat {
    pub man: u32,
    /// Ground's area (`ground::areas`).
    pub area: u16,
    pub since: Date,
    /// You set it (the coordinator's next proposal leaves it alone until
    /// the February meeting).
    pub yours: bool,
    /// The coordinator's reason, or yours.
    pub why: String,
}

/// The coordinator's proposal (GR68: at the February meeting, and on the
/// first day of a job).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Proposal {
    pub date: Option<Date>,
    pub rows: Vec<Beat>,
    /// His letter: one line per man, then the areas left empty.
    pub letter: Vec<String>,
    /// Areas that matter to your program with nobody on them (GR67: their
    /// misses go in the ground report).
    pub empty: Vec<u16>,
}

/// A young coach riding along with a veteran (GR68).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Ride {
    pub young: u32,
    pub veteran: u32,
    pub from: Date,
    pub to: Date,
    /// Days ridden so far.
    pub days: u16,
}

/// A signee credited to the man who worked his area.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Credit {
    pub id: u32,
    pub man: u32,
    pub date: Date,
    pub stars: u8,
    pub area: u16,
}

/// A kid from one of your areas with nobody of yours on it who signed
/// elsewhere (GR67's misses, for the ground report).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Miss {
    pub id: u32,
    pub area: u16,
    pub to: u32,
    pub stars: u8,
    pub date: Date,
}

/// Another school wants one of your men (GR76).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Courted {
    pub man: u32,
    pub employer: Employer,
    pub role: Role,
    pub prestige: f64,
    /// Their offer, dollars a year.
    pub pay: f64,
    /// Why he'd go, in words (the carousel's own reason).
    pub why: String,
    pub made: Date,
    /// His answer is due.
    pub until: Date,
    /// What pulls him, the build's own size (prestige points).
    pub pull: f32,
    /// Your counter (`carousel::Counter` as u8), once made.
    pub counter: Option<u8>,
    /// 0 waiting, 1 he stays, 2 he goes.
    pub state: u8,
    /// What he said in the hallway.
    pub line: String,
}

/// A promise to one of your men (GR76: tracked like hiring promises).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Promise {
    pub man: u32,
    /// 1: your next coordinator job.
    pub kind: u8,
    pub made: Date,
    pub kept: Option<bool>,
}

/// A commit shaken by his recruiter's leaving (GR68).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Waver {
    pub id: u32,
    pub man: u32,
    pub made: Date,
    pub until: Date,
    /// You called him (the scene's one-line stand-in until lane G5).
    pub called: bool,
    /// 0 held, 1 reopened, 2 followed his recruiter.
    pub done: Option<u8>,
    pub line: Option<String>,
}

/// A man who left your staff.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Left {
    pub man: u32,
    pub date: Date,
    /// 0 left on good terms (your tree), 1 grateful (Home in your tree),
    /// 2 angry (he runs you down), 3 neither.
    pub terms: u8,
    pub to: Option<Employer>,
    pub why: String,
}

/// The store on `Ground.staff`.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct StaffGround {
    /// The program it belongs to: a new job starts it again.
    pub program: Option<u32>,
    pub beats: Vec<Beat>,
    pub proposal: Proposal,
    pub rides: Vec<Ride>,
    pub credits: Vec<Credit>,
    pub misses: Vec<Miss>,
    /// Each man's road days by season: (man, season, spring, fall).
    pub out: Vec<(u32, i32, u16, u16)>,
    /// Men passed over for a coordinator job on your staff: (man, day).
    pub passed_over: Vec<(u32, Date)>,
    pub courted: Vec<Courted>,
    pub promises: Vec<Promise>,
    pub wavers: Vec<Waver>,
    pub left: Vec<Left>,
    /// Your staff yesterday (on-field assistants), and the coordinators.
    pub roster: Vec<u32>,
    pub coords: (Option<u32>, Option<u32>),
    /// The last day the store looked at signees (credits and misses).
    pub looked: Option<Date>,
    /// The assistant seat.
    pub desk: desk::Desk,
}

/// Your program, when your seat is a college staff (head coach or
/// assistant).
pub fn seat_program(w: &World) -> Option<usize> {
    super::standing::seat_program(w)
}

/// Whether you hold the head job there.
pub fn you_are_head(w: &World) -> bool {
    w.seat_head()
}

/// Your on-field assistants (coordinators and position coaches, not the
/// head coach), in staff order.
pub fn men(w: &World, prog: usize) -> Vec<usize> {
    let p = &w.programs[prog];
    let mut v: Vec<usize> = Vec::new();
    for c in [p.oc, p.dc].into_iter().chain(p.position_coaches.iter().copied()).flatten() {
        if !v.contains(&c) && Some(c) != p.head_coach {
            v.push(c);
        }
    }
    v
}

/// The February meeting (GR30, GR68): the Monday after signing day.
pub fn february_meeting(w: &World, d: Date) -> Date {
    let y = super::week::beat::dates_for(w, d);
    y.signing_day.add(1).on_or_after(Weekday::Mon)
}

/// Your seat's staff ground each day (after standing and the week).
pub(crate) fn day(w: &mut World, d: Date) {
    let Some(prog) = seat_program(w) else { return };
    fresh(w, prog);
    carousel::roster(w, prog, d);
    beats::day(w, prog, d);
    carousel::day(w, prog, d);
    desk::day(w, prog, d);
}

/// A new job starts a new store: your men as they stand today (never you).
fn fresh(w: &mut World, prog: usize) {
    if w.ground.staff.program == Some(prog as u32) {
        return;
    }
    let desk = std::mem::take(&mut w.ground.staff.desk);
    w.ground.staff = StaffGround { program: Some(prog as u32), ..Default::default() };
    w.ground.staff.desk = desk;
    let you = w.career.as_ref().map(|c| c.coach);
    w.ground.staff.roster = men(w, prog).into_iter().filter(|&c| Some(c) != you).map(|c| c as u32).collect();
    let p = &w.programs[prog];
    w.ground.staff.coords = (p.oc.map(|c| c as u32), p.dc.map(|c| c as u32));
}

/// The staff's areas are in place for your seat before the first day plays
/// (the screens open on a new job): the coordinator's first proposal.
pub fn ready(w: &mut World) {
    let Some(prog) = seat_program(w) else { return };
    if w.ground.off {
        return;
    }
    fresh(w, prog);
    let d = w.today;
    beats::ensure(w, prog, d);
}

/// A man's mood moves how he weighs another job (`staff::weigh_offer`):
/// a happy man stays, an unhappy one listens. Only your own staff's men,
/// so the world plays the same without a career.
pub(crate) fn offer_pull(w: &World, c: usize) -> Option<(f64, String)> {
    let prog = seat_program(w)?;
    if !you_are_head(w) || w.ground.staff.program != Some(prog as u32) || !men(w, prog).contains(&c) {
        return None;
    }
    let m = men::mood(w, prog, c);
    // Half a point of mood either way is worth about three hundredths of
    // prestige (proposed, not confirmed).
    let by = 0.06 * (0.5 - m.value as f64);
    let why = if by > 0.0 { m.reasons.first().cloned().unwrap_or_default() } else { String::new() };
    Some((by, why))
}

/// A memo to your inbox from a person on your staff (or your boss).
pub(crate) fn memo(w: &mut World, d: Date, minute: u16, from: usize, subject: String, body: Vec<String>, loud: u8) {
    use crate::world::desk::{Head, Mail};
    let you = crate::world::folk::scene::your_name(w);
    let title = match w.career.as_ref().and_then(|c| c.role) {
        Some(r) if r != Role::HeadCoach => {
            let s = r.name();
            let mut c = s.chars();
            c.next().map_or_else(String::new, |f| f.to_uppercase().collect::<String>() + c.as_str())
        }
        _ => "Head Coach".to_string(),
    };
    let text: Vec<u64> = subject.bytes().map(u64::from).collect();
    let id = super::key(&[0x57AF_F00D, d.0 as u64, from as u64, super::key(&text)]);
    let name = w.coaches.get(from).map_or_else(|| "Your staff".to_string(), |c| c.name.clone());
    let role = crate::world::folk::facts::coach_role(w, from);
    let role = {
        let mut c = role.chars();
        c.next().map_or_else(String::new, |f| f.to_uppercase().collect::<String>() + c.as_str())
    };
    let Some(c) = w.career.as_mut() else { return };
    if c.desk.has(id) {
        return;
    }
    c.desk.mails.push(Mail {
        id,
        date: d,
        minute,
        head: Head::Memo,
        org: String::new(),
        org_line: "Football office".into(),
        from: name.clone(),
        role,
        to: format!("{you}, {title}"),
        subject,
        body,
        table: Vec::new(),
        attach: Vec::new(),
        sign: name,
        keys: (1 << 2) | (1 << 9),
        about: None,
        answers: Vec::new(),
        answered: None,
        answer_by: None,
        read: false,
        saved: false,
        loud,
        ids: Vec::new(),
        org_team: None,
        why: Vec::new(),
        count: 0,
        leads: 0,
    });
    c.desk.mails.sort_by_key(|m| (m.date, m.minute));
}

/// A man's last name ("Davis").
pub fn last(w: &World, c: usize) -> String {
    super::week::last_name(w, c as u32)
}

/// "1 kid", "3 kids".
pub(crate) fn count(n: usize, one: &str, many: &str) -> String {
    if n == 1 { format!("1 {one}") } else { format!("{n} {many}") }
}
