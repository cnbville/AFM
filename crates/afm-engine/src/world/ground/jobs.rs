//! Jobs carry standing (GR62). Coaches move on the built calendar: high
//! school jobs change on January 20 (`clock.rs`, `hs_carousel_day`), the
//! college carousel after rivalry week (`world/staff.rs`), a five-star's
//! coach to a college staff in December. A memory is keyed by the man, so
//! what you built with him goes with him: his new job is new ground for
//! you, and a high school coach who joins a college staff becomes one that
//! can take the kids you pass on.
//!
//! The new man at his old school starts from his own memory of you: a
//! Stranger, or Known if he sat in on your visits there or the man before
//! him told him about you (Trusted or better with your staff). A school's
//! counselor retires or moves on at the end of a tenure (`people::counselor`):
//! the new one starts the same way.

use super::Holder;
use super::people::{self, COLLEGE, HS_STAFF, Who};
use super::standing::{self, Level};
use crate::calendar::Date;
use crate::world::World;
use crate::world::desk::Head;

/// How long a head coaching job may stay open before the successor rule
/// lapses (the build's pick, proposed, not confirmed).
pub const VACANT_DAYS: i32 = 150;

/// Each day: notice who moved since you last looked.
pub(crate) fn day(w: &mut World, d: Date) {
    let Some(prog) = standing::seat_program(w) else { return };
    let you = w.career.as_ref().map(|c| c.coach as u32);
    let mut whos: Vec<Who> = w
        .ground
        .memories
        .iter()
        .map(|m| m.who)
        .filter(|x| matches!(x, Who::Coach(_) | Who::Counselor(_)) && Some(*x) != you.map(Who::Coach))
        .collect();
    whos.dedup();
    for who in whos {
        let now = people::post_key(w, who);
        let was = w.ground.with(who).iter().map(|m| m.at).find(|&a| a != 0);
        let Some(was) = was else {
            set_at(w, who, now);
            continue;
        };
        if was == now {
            continue;
        }
        set_at(w, who, now);
        match who {
            Who::Coach(_) => moved(w, prog, who, was, now, d),
            Who::Counselor(h) => counselor_changed(w, prog, h, d),
            _ => {}
        }
    }
    successors(w, prog, d);
}

fn set_at(w: &mut World, who: Who, at: u32) {
    let a = w.ground.memories.partition_point(|m| m.who < who);
    for m in w.ground.memories[a..].iter_mut().take_while(|m| m.who == who) {
        m.at = at;
    }
}

/// Where a post key was, in words.
fn place(w: &World, key: u32) -> Option<String> {
    match key {
        k if (1..HS_STAFF).contains(&k) => Some(w.school_name((k - 1) as usize).to_string()),
        k if (HS_STAFF..COLLEGE).contains(&k) => Some(w.school_name((k - HS_STAFF) as usize).to_string()),
        k if (COLLEGE..0x7FFF_FFFE).contains(&k) => Some(w.program_name((k - COLLEGE) as usize).to_string()),
        _ => None,
    }
}

fn moved(w: &mut World, prog: usize, who: Who, was: u32, now: u32, d: Date) {
    let lv = standing::level(w, prog, who);
    // A head coach who left his school: the job's next man gets the
    // successor rule once it is filled.
    if (1..HS_STAFF).contains(&was) {
        w.ground.vacated.push(((was - 1), who, d));
    }
    if lv < Level::Known {
        return;
    }
    let name = people::name(w, who);
    let from = place(w, was).unwrap_or_else(|| "his old job".into());
    let text = match place(w, now) {
        Some(to) if now >= COLLEGE => format!(
            "{name} has left {from} for {to}'s staff. What you built with him goes with him: he can take the kids you pass on now."
        ),
        Some(to) => format!("{name} has left {from} for {to}. What you built with him goes with him."),
        None => format!("{name} is out at {from}. What you built with him stays with the man."),
    };
    standing::post(w, who, d, Head::Memo, format!("{name} moves on"), vec![text], 1);
    // The memory remembers the move.
    let a = w.ground.memories.partition_point(|m| m.who < who);
    let words = format!("followed him from {from}");
    for m in w.ground.memories[a..].iter_mut().take_while(|m| m.who == who) {
        if m.points >= Level::Known.floor() {
            m.heard = Some(words.clone());
        }
    }
}

/// The new head coach at a school whose man moved on: Known if he sat in
/// on your visits or heard about you from the man before him.
fn successors(w: &mut World, prog: usize, d: Date) {
    if w.ground.vacated.is_empty() {
        return;
    }
    let list = std::mem::take(&mut w.ground.vacated);
    let mut keep = Vec::new();
    for (h, before, since) in list {
        let Some(new) = people::head_coach_of(w, h as usize) else {
            if since.days_to(d) <= VACANT_DAYS {
                keep.push((h, before, since));
            }
            continue;
        };
        if new == before {
            continue;
        }
        let school = w.school_name(h as usize).to_string();
        let before_name = people::name(w, before);
        let lv0 = standing::level(w, prog, new);
        let at = people::post_key(w, new);
        for holder in standing::holders(w, prog) {
            let pred = w.ground.get(before, holder).map_or(0.0, |m| m.points);
            let sat_in = w.ground.get(new, holder).is_some_and(|m| m.contacts > 0);
            let told = pred >= Level::Trusted.floor();
            if !(sat_in || told) {
                continue;
            }
            let start = standing::base(w, new, holder);
            let m = standing::entry(&mut w.ground, start, new, holder, d);
            if m.points < Level::Known.floor() {
                m.points = Level::Known.floor();
                m.contacts = m.contacts.max(2);
                m.known_since.get_or_insert(d);
            }
            m.heard = Some(if sat_in {
                "he sat in on your visits".into()
            } else {
                format!("{before_name} told him about you")
            });
            m.at = at;
        }
        let lv = standing::level(w, prog, new);
        if lv > lv0 || lv >= Level::Known {
            let name = people::name(w, new);
            let how = if w.ground.with(new).iter().any(|m| m.heard.as_deref() == Some("he sat in on your visits")) {
                "He sat in on your visits there, so he knows you."
            } else {
                "The man before him told him about you."
            };
            let text = format!("{name} is the new head coach at {school}. {how}");
            standing::post(w, new, d, Head::Memo, format!("New coach at {school}"), vec![text], 1);
            w.ground.set_told(new, lv as u8);
        }
    }
    w.ground.vacated = keep;
}

/// A counselor's tenure ended: what you built goes with that person, and
/// the new one starts from what the school says of you.
fn counselor_changed(w: &mut World, prog: usize, h: u32, d: Date) {
    let who = Who::Counselor(h);
    let had = standing::level(w, prog, who);
    let a = w.ground.memories.partition_point(|m| m.who < who);
    let b = w.ground.memories.partition_point(|m| m.who <= who);
    w.ground.memories.drain(a..b);
    w.ground.set_told(who, 0);
    if had >= Level::Trusted {
        let holder = Holder::Program(prog as u32);
        let at = people::post_key(w, who);
        let m = standing::entry(&mut w.ground, 0.0, who, holder, d);
        m.points = Level::Known.floor();
        m.contacts = 2;
        m.known_since = Some(d);
        m.heard = Some("the school vouched for you".into());
        m.at = at;
        w.ground.set_told(who, Level::Known as u8);
    }
    if had >= Level::Known {
        let school = w.school_name(h as usize).to_string();
        let name = people::name(w, who);
        let text = format!("{school} has a new counselor, {name}. You start over with the new one.");
        standing::post(w, who, d, Head::Memo, format!("New counselor at {school}"), vec![text], 1);
    }
}
