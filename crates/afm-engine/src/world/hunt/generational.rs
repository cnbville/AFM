//! The generational prospect (SK67): once or twice a decade a prospect
//! stands above every class before him, and the whole world chases him.
//!
//! The public keeps a record of each class's best: the services' read of
//! the class going into its junior year (14 July), and the pro scouts'
//! December read of each draft's best. A prospect who beats the best of
//! each of the nine classes (or drafts) before his is generational: "the
//! best the services have seen since ...". The label is the public's word
//! from its own reads, so it can be wrong (potential stays hidden): a
//! label is a hint, not a promise. Every power program then hunts him
//! (`ai::GEN_PRESTIGE`); in his draft year the five clubs with the worst
//! records do.
//!
//! Proposed, not confirmed: nine classes of record, the summer read, the
//! December read, the five worst clubs.

use crate::calendar::{Date, Event, Kind as CalKind, Level, Loudness};
use crate::positions::Position;
use crate::world::scouting as sc;
use crate::world::{Stage, World};

/// Classes (or drafts) of record a prospect must beat.
pub const RECORD: i32 = 9;

/// A class's best by the public's read.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Top {
    /// His class (high school) or the draft (pro scouts).
    pub year: i32,
    pub draft: bool,
    pub id: u32,
    pub name: String,
    pub read: f32,
}

/// A generational prospect.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Gen {
    pub id: u32,
    pub name: String,
    pub pos: Position,
    /// His class (high school label) or his draft (pro label).
    pub year: i32,
    pub draft: bool,
    pub date: Date,
    /// The last one the public read higher, before the record: (year, name).
    pub since: Option<(i32, String)>,
}

pub(crate) fn day(w: &mut World, d: Date) {
    let (_, m, dd) = d.parts();
    if (m, dd) == (7, 14) {
        class(w, d);
    }
    if d == w.dates.fbs_regular_end.add(1) {
        draft(w, d);
    }
}

/// The record so far, the newest `RECORD` before `year`.
pub(crate) fn beats_record(w: &World, draft: bool, year: i32, read: f32) -> Option<Option<(i32, String)>> {
    let prev: Vec<&Top> =
        w.hunt.tops.iter().filter(|t| t.draft == draft && t.year < year && t.year >= year - RECORD).collect();
    if (prev.len() as i32) < RECORD || prev.iter().any(|t| t.read >= read) {
        return None;
    }
    let since = w
        .hunt
        .tops
        .iter()
        .filter(|t| t.draft == draft && t.year < year && t.read >= read)
        .max_by_key(|t| t.year)
        .map(|t| (t.year, t.name.clone()));
    Some(since)
}

fn keep(w: &mut World, t: Top) {
    w.hunt.tops.retain(|x| !(x.draft == t.draft && x.year == t.year));
    w.hunt.tops.push(t);
    let cut = w.year - 60;
    w.hunt.tops.retain(|x| x.year >= cut);
}

/// 14 July: the class going into its junior year.
fn class(w: &mut World, d: Date) {
    let class_of = d.year() + 2;
    let best = w
        .people
        .iter()
        .filter(|p| matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) && p.class_of == class_of && p.stars >= 4)
        .filter_map(|p| p.cal.service_read.map(|r| (r, p.player.id)))
        .max_by(|a, b| a.0.total_cmp(&b.0).then(b.1.cmp(&a.1)));
    let Some((read, id)) = best else { return };
    let Some(i) = sc::place(w, id) else { return };
    let (name, pos, hs) = (w.people[i].name.clone(), w.people[i].player.position, w.people[i].hs);
    let read = read as f32;
    let label = beats_record(w, false, class_of, read);
    keep(w, Top { year: class_of, draft: false, id, name: name.clone(), read });
    let Some(since) = label else { return };
    let when = match &since {
        Some((y, n)) => format!("the best prospect the services have seen since {n} (class of {y})"),
        None => "the best prospect the services have ever ranked".into(),
    };
    let school = w.school_name(hs).to_string();
    let text =
        format!("{name}, a {pos} at {school} in the class of {class_of}: {when}. A generational talent, they say.");
    w.calendar.post(
        Event::new(d, Level::Recruiting, CalKind::News, Loudness::Huge, text)
            .person(id, &name)
            .hs_link(hs, &school)
            .memorable(),
    );
    w.hunt.generational.push(Gen { id, name, pos, year: class_of, draft: false, date: d, since });
}

/// December (the pro scouts' read): the coming draft's best.
fn draft(w: &mut World, d: Date) {
    let dy = w.year + 1;
    let best = w
        .people
        .iter()
        .filter(|p| p.stage == Stage::College && dy - p.class_of >= 3 && p.pro.is_none())
        .filter_map(|p| p.cal.draft_read.map(|r| (r.score, p.player.id)))
        .max_by(|a, b| a.0.total_cmp(&b.0).then(b.1.cmp(&a.1)));
    let Some((read, id)) = best else { return };
    let Some(i) = sc::place(w, id) else { return };
    let (name, pos) = (w.people[i].name.clone(), w.people[i].player.position);
    let read = read as f32;
    let label = beats_record(w, true, dy, read);
    keep(w, Top { year: dy, draft: true, id, name: name.clone(), read });
    let Some(since) = label else { return };
    let when = match &since {
        Some((y, n)) => format!("the best prospect the scouts have read since {n} ({y} draft)"),
        None => "the best prospect the scouts have ever read".into(),
    };
    let school = w.people[i].college.map_or_else(String::new, |c| format!(" at {}", w.program_name(c)));
    let text = format!("{name}, a {pos}{school}: {when}. The bottom of the league starts counting losses.");
    w.calendar.post(Event::new(d, Level::Nfl, CalKind::News, Loudness::Huge, text).person(id, &name).memorable());
    w.hunt.generational.push(Gen { id, name: name.clone(), pos, year: dy, draft: true, date: d, since });
    // The five worst records hunt him now, whatever they planned.
    let order = crate::world::nfl_year::mock_order(w).unwrap_or_default();
    let mine = super::seat(w).filter(|x| x.0 == super::Seat::Club).map(|x| x.1);
    let worst: Vec<usize> = order.into_iter().filter(|&t| t < w.clubs.len() && Some(t) != mine).take(5).collect();
    for &t in &worst {
        for x in w
            .hunt
            .ai
            .iter_mut()
            .filter(|x| x.level == super::ai::CLUB && x.team == t as u32 && x.year == dy && x.open())
        {
            x.end = Some((d, false));
        }
        w.hunt.ai.push(super::ai::AiHunt {
            level: super::ai::CLUB,
            team: t as u32,
            id,
            year: dy,
            since: d,
            why: super::ai::GEN,
            end: None,
        });
    }
}

/// The generational prospect of a class (the high school label).
pub fn of_class(w: &World, class_of: i32) -> Option<u32> {
    w.hunt.generational.iter().find(|g| !g.draft && g.year == class_of).map(|g| g.id)
}

/// The generational prospect of a draft (the pro label, else the high
/// school label of a man in that draft's pool).
pub fn of_draft(w: &World, dy: i32) -> Option<u32> {
    w.hunt.generational.iter().find(|g| g.draft && g.year == dy).map(|g| g.id)
}

/// A player's label, if he carries one (the newest).
pub fn of(w: &World, id: u32) -> Option<&Gen> {
    w.hunt.generational.iter().rev().find(|g| g.id == id)
}
