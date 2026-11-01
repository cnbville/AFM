//! The film room and being there (update 3, lane 9; scouting update
//! SK32-SK34, SK36-SK39, SK43-SK63).
//!
//! All film is cut-ups (SK45): a player's tape is played on the match
//! engine from his real team against his real opponents (`tape`), and a
//! session watches one cut-up snap by snap (`session`). What your eye
//! catches is kept per attribute (`Catch`) and moves your own read
//! (`sharpen`); the session becomes your report in his file. Film time is
//! one weekly ledger on the real calendar, set by how hard you work
//! (`time`); a kid nobody recruits can pop off another kid's tape
//! (`finds`); your staff send you a queue of cut-ups each week (`queue`);
//! and the days you are there in person are scenes (`there`).
//!
//! Every number no source gives is Claude's, proposed, not confirmed.

pub mod finds;
pub mod opp;
pub mod queue;
pub mod session;
pub mod tape;
pub mod there;
pub mod time;
pub mod words;

use crate::attributes::{ATTRIBUTE_COUNT, Attr};
use crate::calendar::{Date, Weekday};
use crate::positions::{Position, weight};
use crate::world::World;

/// What a session watches: a player, or a team (your next opponent).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Target {
    Player(u32),
    Team(u16),
}

/// One thing your eye (or your analyst beside you) caught on a snap: the
/// attribute, what it looked like and how sure (rating points).
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Catch {
    pub id: u32,
    pub attr: u8,
    pub v: f32,
    pub sd: f32,
    pub date: Date,
    /// 0 your eye, 1 your analyst's point.
    pub by: u8,
    /// Which snap it was read on (0 for a look in person): the same snap
    /// watched again adds nothing new.
    #[serde(default)]
    pub key: u64,
}

/// Same snap, same man, same thing, same eyes.
fn same(a: &Catch, b: &Catch) -> bool {
    a.key != 0 && a.key == b.key && a.id == b.id && a.attr == b.attr && a.by == b.by
}

/// Adds a session's catches: a snap seen before keeps the tighter read of
/// it (a better eye sees more in the same snap), never both.
pub(crate) fn add_catches(all: &mut Vec<Catch>, new: Vec<Catch>) {
    for c in new {
        match all.iter().position(|o| same(o, &c)) {
            Some(k) if c.sd < all[k].sd => all[k] = c,
            Some(_) => {}
            None => all.push(c),
        }
    }
}

/// A tag carried forward (SK44): "ask him about this" to the interview
/// room, "show the staff" to the board meetings.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Tag {
    pub id: u32,
    pub date: Date,
    /// 2 ask him, 3 show the staff (`session::TAGS`).
    pub kind: u8,
    pub text: String,
    #[serde(default)]
    pub used: bool,
}

/// Everything the film room keeps (on `Scouting::film`; every field fills
/// in empty on an older file).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct FilmRoom {
    /// The session in progress.
    #[serde(default)]
    pub open: Option<session::Session>,
    #[serde(default)]
    pub reports: Vec<session::Report>,
    #[serde(default)]
    pub catches: Vec<Catch>,
    #[serde(default)]
    pub tags: Vec<Tag>,
    /// Your staff's queue (SK55).
    #[serde(default)]
    pub queue: Vec<queue::Queued>,
    /// Film hours today (the day, hours): your eye tires (SK59).
    #[serde(default)]
    pub today: Option<(Date, f32)>,
    /// How hard you work this week (SK61).
    #[serde(default)]
    pub work: time::Work,
    /// The weeks you worked: Monday, level, film hours, the budget.
    #[serde(default)]
    pub weeks: Vec<time::Week>,
    /// Wear from long weeks (0 fresh, 1 worn out), and the rest you were
    /// sent home for.
    #[serde(default)]
    pub wear: f32,
    #[serde(default)]
    pub rest_until: Option<Date>,
    /// Film hours by position (SK63), by `Position::index`.
    #[serde(default)]
    pub pos_hours: Vec<f32>,
    /// Film hours by year (your coach profile, SK63).
    #[serde(default)]
    pub years: Vec<(i32, f32)>,
    /// Kids who popped off another kid's tape (SK34).
    #[serde(default)]
    pub finds: Vec<finds::Find>,
    /// Your work ethic (SK62), 1-20, once set or moved; None reads 12.
    #[serde(default)]
    pub ethic: Option<f32>,
    /// The days you were there in person (SK32-SK39).
    #[serde(default)]
    pub there: there::There,
    /// When the word "first one in the building" last got around.
    #[serde(default)]
    pub word: Option<Date>,
    /// Bumped on every change that moves a read (the reads cache keys on it).
    #[serde(default)]
    pub version: u32,
    /// The hardest work level set this week, from its Monday (SK61).
    #[serde(default)]
    pub week_work: Option<(Date, time::Work)>,
    /// The year you last set your work ethic (SK62): once a year.
    #[serde(default)]
    pub ethic_year: Option<i32>,
}

/// How far a catch drifts from him as he changes: rating points a year
/// (a young player grows, an old one fades).
const DRIFT_A_YEAR: f64 = 3.0;

/// Your film catches of a player, attribute by attribute: each one
/// widened by its age, combined by inverse variance. Exact (sd 0, his
/// real number today) once it is tighter than half a point on fresh tape:
/// "100%" for what the tape shows (SK57).
pub fn catches_of(w: &World, id: u32) -> Vec<Option<(f64, f64)>> {
    catches_with(w, id, &[])
}

/// The same with catches not yet saved (a session in progress).
pub fn catches_with(w: &World, id: u32, extra: &[Catch]) -> Vec<Option<(f64, f64)>> {
    let mut acc = vec![(0.0f64, 0.0f64, i32::MAX); ATTRIBUTE_COUNT];
    let today = w.today;
    let stored = &w.scouting.film.catches;
    let fresh = extra.iter().filter(|c| !stored.iter().any(|o| same(o, c)));
    for c in stored.iter().chain(fresh).filter(|c| c.id == id) {
        let k = c.attr as usize;
        if k >= ATTRIBUTE_COUNT {
            continue;
        }
        let age = c.date.days_to(today).max(0) as f64 / 365.0;
        let sd = ((c.sd as f64).powi(2) + (DRIFT_A_YEAR * age).powi(2)).sqrt().max(0.05);
        let wt = 1.0 / (sd * sd);
        acc[k].0 += wt * c.v as f64;
        acc[k].1 += wt;
        acc[k].2 = acc[k].2.min(c.date.days_to(today));
    }
    let i = super::place(w, id);
    acc.iter()
        .enumerate()
        .map(|(k, &(s, wt, fresh))| {
            if wt <= 0.0 {
                return None;
            }
            let sd = (1.0 / wt).sqrt();
            if sd <= 0.5
                && fresh <= 30
                && let Some(i) = i
            {
                return Some((w.people[i].player.attributes.get(Attr::ALL[k]), 0.0));
            }
            Some((s / wt, sd))
        })
        .collect()
}

/// Your film catches folded into your own read (SK43): each attribute by
/// inverse variance with what your looks read, and the overall narrowed
/// by how much of his position's weight the tape has pinned down.
pub fn sharpen(w: &World, id: u32, pos: Position, now: &mut (f64, f64), attrs: &mut Vec<Option<(f64, f64)>>) {
    sharpen_with(w, id, pos, now, attrs, &[]);
}

/// The same with catches not yet saved.
pub fn sharpen_with(
    w: &World,
    id: u32,
    pos: Position,
    now: &mut (f64, f64),
    attrs: &mut Vec<Option<(f64, f64)>>,
    extra: &[Catch],
) {
    let film = catches_with(w, id, extra);
    if film.iter().all(Option::is_none) {
        return;
    }
    attrs.resize(ATTRIBUTE_COUNT, None);
    let (mut shift, mut cov, mut total) = (0.0, 0.0, 0.0);
    for a in Attr::ALL {
        let wt = weight(pos, a);
        total += wt;
        let k = a.index();
        let Some(f) = film[k] else { continue };
        let before = attrs[k];
        let after = match before {
            _ if f.1 == 0.0 => f,
            Some(b) => {
                let (x, y) = (1.0 / (b.1 * b.1).max(0.01), 1.0 / (f.1 * f.1));
                ((b.0 * x + f.0 * y) / (x + y), (1.0 / (x + y)).sqrt())
            }
            None => f,
        };
        if let Some(b) = before {
            shift += wt * (after.0 - b.0);
            cov += wt * (1.0 - (after.1 * after.1) / (b.1 * b.1).max(0.01)).clamp(0.0, 1.0);
        } else {
            // Read for the first time: how much of a wide guess (9 points)
            // the tape replaced.
            cov += wt * (1.0 - (after.1 / 9.0).powi(2)).clamp(0.0, 1.0);
        }
        attrs[k] = Some(after);
    }
    if total > 0.0 {
        now.0 = (now.0 + shift / total).clamp(1.0, 99.0);
        now.1 = (now.1 * (1.0 - cov / total).max(0.0).sqrt()).max(if cov >= total * 0.999 { 0.0 } else { 0.5 });
    }
}

/// The film room's day: the week turns over on Monday (your hours logged,
/// wear, your ethic, the staff mirroring you, the word around the
/// building, a new queue), the look-backs settle, and the days you are
/// there come round.
pub(crate) fn day(w: &mut World, d: Date) {
    if d.weekday() == Weekday::Mon {
        time::week(w, d);
        queue::week(w, d);
    }
    // A visit ends with the day.
    if w.scouting.film.there.visit.as_ref().is_some_and(|v| v.date < d) {
        let _ = there::visit_close(w);
    }
    settle(w, d);
}

/// The look-back (SK58): three years on, each verdict you gave on a
/// player's question is checked against what he is now.
fn settle(w: &mut World, d: Date) {
    let due: Vec<usize> = (0..w.scouting.film.reports.len())
        .filter(|&k| {
            let r = &w.scouting.film.reports[k];
            r.settled.is_none() && matches!(r.verdict, Some(0 | 1)) && r.date.days_to(d) >= 1095
        })
        .collect();
    for k in due {
        let r = &w.scouting.film.reports[k];
        let (Target::Player(id), Some(q)) = (r.target, r.ask.clone()) else { continue };
        let Some(i) = super::place(w, id) else { continue };
        let p = &w.people[i];
        let m: f64 = q.attrs.iter().map(|&a| p.player.attributes.get(Attr::ALL[a as usize])).sum::<f64>()
            / q.attrs.len().max(1) as f64;
        let yes = m >= q.bar as f64;
        let right = (r.verdict == Some(0)) == yes;
        w.scouting.film.reports[k].settled = Some(right);
    }
}

/// Your tags waiting for a room: "ask him about this" for the interview
/// room, "show the staff" for the board meeting.
pub fn tags_for(w: &World, id: u32, kind: u8) -> Vec<&Tag> {
    w.scouting.film.tags.iter().filter(|t| t.id == id && t.kind == kind && !t.used).collect()
}

/// A tag used in its room.
pub fn use_tags(w: &mut World, id: u32, kind: u8) {
    for t in w.scouting.film.tags.iter_mut().filter(|t| t.id == id && t.kind == kind) {
        t.used = true;
    }
}
