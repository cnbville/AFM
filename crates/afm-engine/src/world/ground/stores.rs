//! The older stores read through the one memory (GR62, lane G7). Before
//! Ground, four places kept their own track of what a person thinks of
//! you. They read through the memory now:
//!
//! - **Your network** (`folk/network.rs`): for a person on your ground (a
//!   high school or junior college coach, a trainer, a 7v7 director) at a
//!   college seat, his warmth is your own standing with him, his favor
//!   balance and his tips' record live in the memory. The contact keeps
//!   them as a copy the screens and older readers see, refreshed each day
//!   (`sync`); what the network's own code changes (a call, a favor asked
//!   or done, a tip settled) is folded into the memory first. A coach at a
//!   four-year college stays a colleague: his standing reads your network
//!   (`standing::derived`).
//! - **The high school coach's credit** with each college staff
//!   (`scouting/hs.rs`): kept as the staff's memory of you (you are the
//!   person, the college the holder), the same numbers as before.
//! - **Promises' trust**: a recruit's family reads you from its own memory
//!   once it knows you; your trail name stays what a stranger reads before
//!   he knows you (C10; `folk/promise.rs`, `trust_part`). Live talks read
//!   the family's and a coach's memory for their trust in you
//!   (`persona/makeup.rs`, `trust`).
//! - **Standing** is the memory itself (lane G1b).
//!
//! Old saves load: the old fields stay, and `after_load` moves the credit
//! map into the memory.

use super::people::{self, Holder, Kind, Who};
use super::standing::{self, Level};
use crate::calendar::Date;
use crate::world::World;
use crate::world::folk::network::{Contact, Other};

/// The person on your ground a network contact is, when his tie lives in
/// the memory: a college seat with the ground on, and a high school or
/// junior college coach, a trainer or a 7v7 director.
pub fn ground_who(w: &World, x: &Contact) -> Option<Who> {
    standing::seat_program(w)?;
    let who = match (x.other, x.coach_id()) {
        (None, Some(c)) => Who::Coach(c as u32),
        (Some(Other::Former(f)), _) => match w.formers.get(f).map(|f| f.life) {
            Some(crate::world::former::Life::Trainer) => Who::Trainer(f),
            Some(crate::world::former::Life::Camp) => Who::SevenOnSeven(f),
            _ => return None,
        },
        _ => return None,
    };
    matches!(
        people::kind(w, who),
        Kind::HsHead | Kind::HsCoordinator | Kind::HsCoach | Kind::Juco | Kind::Trainer | Kind::SevenOnSeven
    )
    .then_some(who)
}

/// Each day: what the network changed is folded into the memory, and the
/// contact shows the memory again.
pub(crate) fn sync(w: &mut World) {
    let Some(you) = w.career.as_ref().map(|c| c.coach) else { return };
    let yh = Holder::Coach(you as u32);
    let d = w.today;
    for k in 0..w.folk.network.contacts.len() {
        let Some(who) = ground_who(w, &w.folk.network.contacts[k]) else { continue };
        let x = w.folk.network.contacts[k].clone();
        let words = new_favors(w, &x);
        let start = standing::derived(w, who, yh);
        let m = standing::entry(&mut w.ground, start, who, yh, d);
        match m.seen {
            None => {
                // An old acquaintance from before Ground: the network's tie
                // as it stood.
                m.points = m.points.max(100.0 * x.warmth);
                m.met = m.met.min(x.met);
                if m.points >= Level::Known.floor() {
                    m.contacts = m.contacts.max(2);
                    m.known_since.get_or_insert(x.met.max(d.add(-365)));
                }
                m.owes = (m.owes as i16 + x.balance as i16).clamp(-5, 5) as i8;
                m.his_word = (m.his_word.0.max(x.record.0), m.his_word.1.max(x.record.1));
                m.roots.network = true;
                if m.heard.is_none() && !x.how.is_empty() {
                    m.heard = Some(x.how.clone());
                }
            }
            Some((sw, sb, sr)) => {
                let dw = x.warmth - sw;
                if dw.abs() > 1e-5 {
                    standing::gain(m, 100.0 * dw, d);
                    if dw > 0.0 {
                        m.last = m.last.max(x.last.unwrap_or(d));
                    }
                }
                let db = x.balance as i16 - sb as i16;
                if db != 0 {
                    m.owes = (m.owes as i16 + db).clamp(-5, 5) as i8;
                }
                m.his_word.0 = m.his_word.0.saturating_add(x.record.0.saturating_sub(sr.0));
                m.his_word.1 = m.his_word.1.saturating_add(x.record.1.saturating_sub(sr.1));
            }
        }
        for (date, text, his) in words {
            if !m.favors.iter().any(|f| f.date == date && f.words == text) {
                m.favors.push(super::memory::Favor { date, words: text, his });
                if m.favors.len() > super::memory::FAVORS {
                    m.favors.remove(0);
                }
            }
        }
        let (pts, owes, his) = (m.points, m.owes, m.his_word);
        let c = &mut w.folk.network.contacts[k];
        c.warmth = (pts / 100.0).clamp(0.0, 1.0);
        c.balance = owes;
        c.record = his;
        let seen = (c.warmth, c.balance, c.record);
        if let Some(m) = w.ground.get_mut(who, yh) {
            m.seen = Some(seen);
        }
    }
}

/// The ledger's favors with a contact, in words: (day, words, he did it
/// for you).
fn new_favors(w: &World, x: &Contact) -> Vec<(Date, String, bool)> {
    w.folk
        .network
        .ledger
        .iter()
        .filter(|f| match x.other {
            Some(o) => f.other == Some(o),
            None => f.other.is_none() && f.coach == x.coach,
        })
        .rev()
        .take(super::memory::FAVORS)
        .map(|f| {
            let words = if f.asked {
                format!("he did you a favor: {}", f.words)
            } else {
                format!("you did him a favor: {}", f.words)
            };
            (f.date, words, f.asked)
        })
        .collect()
}

/// Whether the network's own monthly cooling skips this contact: a person
/// on your ground fades in the memory instead (GR24).
pub fn fades_in_memory(w: &World, x: &Contact) -> bool {
    ground_who(w, x).is_some()
}

// ---------------------------------------------------------------------------
// The high school coach's credit (`scouting/hs.rs`).

/// A college staff's credit for your word as a high school coach, from its
/// memory of you.
pub fn credit(w: &World, prog: usize) -> Option<f32> {
    let Some(you) = w.career.as_ref().map(|c| c.coach as u32) else {
        return w.scouting.hs.credibility.get(&prog).copied();
    };
    w.ground.get(Who::Coach(you), Holder::Program(prog as u32)).and_then(|m| m.credit)
}

/// Sets a staff's credit for your word.
pub fn set_credit(w: &mut World, prog: usize, v: f32) {
    let Some(you) = w.career.as_ref().map(|c| c.coach as u32) else {
        w.scouting.hs.credibility.insert(prog, v);
        return;
    };
    let (who, holder, d) = (Who::Coach(you), Holder::Program(prog as u32), w.today);
    let m = w.ground.entry(who, holder, || super::Memory::new(who, holder, 0.0, d));
    m.credit = Some(v);
}

/// Every staff's credit for your word, by program.
pub fn credits(w: &World) -> Vec<(usize, f32)> {
    let Some(you) = w.career.as_ref().map(|c| c.coach as u32) else {
        let mut v: Vec<(usize, f32)> = w.scouting.hs.credibility.iter().map(|(&k, &c)| (k, c)).collect();
        v.sort_by_key(|x| x.0);
        return v;
    };
    w.ground
        .with(Who::Coach(you))
        .iter()
        .filter_map(|m| match (m.holder, m.credit) {
            (Holder::Program(p), Some(c)) => Some((p as usize, c)),
            _ => None,
        })
        .collect()
}

/// A file from before lane G7: the credit map moves into the memory.
pub(crate) fn after_load(w: &mut World) {
    if w.career.is_none() || w.scouting.hs.credibility.is_empty() {
        return;
    }
    let mut old: Vec<(usize, f32)> = w.scouting.hs.credibility.drain().collect();
    old.sort_by_key(|x| x.0);
    for (p, c) in old {
        if credit(w, p).is_none() {
            set_credit(w, p, c);
        }
    }
}

// ---------------------------------------------------------------------------
// Trust, for promises and live talks.

/// Your staff's standing with a recruit's family, when it knows you: the
/// points, and whether it holds a lie or a broken word against you.
fn family_points(w: &World, person: usize) -> Option<(f32, bool)> {
    let prog = standing::seat_program(w)?;
    let fam = people::family_of(w, person);
    let any = w.ground.with(fam).iter().any(|m| standing::holders(w, prog).contains(&m.holder));
    if !any {
        return None;
    }
    let (pts, holder) = standing::staff(w, prog, fam);
    let held = w.ground.get(fam, holder).is_some_and(|m| m.holding(w.today).is_some())
        || w.ground.get(fam, Holder::Program(prog as u32)).is_some_and(|m| m.holding(w.today).is_some());
    Some((pts, held))
}

/// The family's read of you in the trail name's units (-1 to 1), once it
/// knows you (Known or better), or holds a broken word against you; the
/// trail's name until then (promises, `trust_part`).
pub fn family_name(w: &World, person: usize) -> Option<f32> {
    let (pts, held) = family_points(w, person)?;
    if held {
        return Some(-1.0);
    }
    (pts >= Level::Known.floor()).then(|| ((pts - 25.0) / 50.0).clamp(-1.0, 1.0))
}

/// A family's trust in you for a live talk (0-1), from its memory.
pub fn family_trust(w: &World, person: usize) -> Option<f64> {
    let (pts, held) = family_points(w, person)?;
    Some(if held { 0.15 } else { (0.25 + 0.6 * pts as f64 / 100.0).clamp(0.15, 0.85) })
}

/// A ground coach's trust in you for a live talk, from your staff's
/// standing with him (the network's warmth reads the same memory).
pub fn coach_trust(w: &World, c: usize) -> Option<f64> {
    let prog = standing::seat_program(w)?;
    let who = Who::Coach(c as u32);
    if !people::on_ground(w, who) || w.ground.with(who).is_empty() {
        return None;
    }
    let (pts, _) = standing::staff(w, prog, who);
    Some((0.35 + 0.5 * pts as f64 / 100.0).clamp(0.1, 0.95))
}
