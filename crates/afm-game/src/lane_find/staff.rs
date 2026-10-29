//! Your staff's reads for the search, kept for the day (update 2, lane 7).
//!
//! A search filters on what your files say. Each player your staff has
//! looked at is read once (`readers::read`, the same read his scouting
//! file shows) into a few compact numbers: his overall and ceiling, each
//! attribute's centre and spread, the character traits, the medical read
//! and how many looks are behind it. The reads are kept until the day, your
//! seat or the looks change, so a search over a whole class reads numbers,
//! not files.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use afm_engine::world::World;
use afm_engine::world::scouting::Kind;
use afm_engine::world::scouting::readers::{self, Staff};
use rayon::prelude::*;

/// One player's read as your staff has it.
#[derive(Clone, Debug)]
pub struct Read {
    /// Overall at his position now, and its spread.
    pub now: (f32, f32),
    /// Where he could get to, and its spread.
    pub ceiling: (f32, f32),
    /// Each attribute: centre and spread (rounded), spread 0 when unseen.
    pub attrs: Box<[(u8, u8)]>,
    /// Each character trait: centre and spread, spread 0 when unread.
    pub traits: Box<[(f32, f32)]>,
    /// 0 cleared, 1 minor, 2 major concern, 3 failed; and whether a doctor
    /// of yours examined him.
    pub medical: (u8, bool),
    /// Looks in the file, the film and live ones among them, and the best
    /// judge among the readers (1-20).
    pub looks: u16,
    pub film: u16,
    pub judge: f32,
    /// Your staff's own looks by kind that the search asks about.
    pub camp: bool,
    pub transcript: bool,
    pub contact: u16,
}

impl Read {
    /// An attribute's read: centre and spread, or None while unseen.
    pub fn attr(&self, k: usize) -> Option<(f64, f64)> {
        self.attrs.get(k).filter(|a| a.1 > 0).map(|a| (a.0 as f64, a.1 as f64))
    }
}

struct Cache {
    key: (i32, u64, usize, u64),
    reads: Arc<HashMap<u32, Read>>,
}

static CACHE: Mutex<Option<Cache>> = Mutex::new(None);

/// What changes a staff's reads: the day, whose staff, how many looks its
/// files hold, and who reads them (a club's head coach and coordinators are
/// read by role, so a hire the same day re-reads; the doctors' rating and
/// the world's seed, so another file never borrows these reads).
pub(crate) fn cache_key(w: &World, staff: Staff) -> (i32, u64, usize, u64) {
    let mut looks: usize = w.scouting.dept.looks.values().map(|v| v.len()).sum();
    if let Staff::Club(t) = staff
        && let Some(c) = w.clubs.get(t)
    {
        looks += c.files.values().map(|v| v.len()).sum::<usize>();
    }
    let who = match staff {
        Staff::Club(t) => w.nfl.get(t).map_or(0, |tm| {
            let id = |c: Option<usize>| c.map_or(0xFFFF, |c| c as u64 & 0xFFFF);
            id(tm.head_coach) | id(tm.oc) << 16 | id(tm.dc) << 32
        }),
        Staff::Program(p) => w.programs.get(p).map_or(0, |pr| pr.head_coach.map_or(0xFFFF, |c| c as u64 & 0xFFFF)),
        Staff::School(_) => 0,
    };
    let medical = readers::medical_rating(w, staff) as u64;
    let seat = who ^ medical << 48 ^ (w.options.seed as u64).rotate_left(24);
    (w.today.0, readers::staff_key(staff) ^ (w.people.len() as u64) << 32, looks, seat)
}

/// Whether your staff has a file on a player (a look by anyone of yours).
pub fn has_file(w: &World, staff: Staff, id: u32) -> bool {
    w.scouting.dept.looks.get(&id).is_some_and(|v| !v.is_empty())
        || matches!(staff, Staff::Club(t) if w.clubs.get(t).is_some_and(|c| c.files.get(&id).is_some_and(|v| !v.is_empty())))
}

/// Your staff's contacts with a player: calls, home visits, his visits
/// and your camps.
pub fn contacts(w: &World, id: u32) -> usize {
    w.scouting.dept.looks.get(&id).map_or(0, |v| {
        v.iter()
            .filter(|l| matches!(l.kind, Kind::Call | Kind::HomeVisit | Kind::Official | Kind::Unofficial | Kind::Camp))
            .count()
    })
}

/// Whether your staff has been in touch with a player (what "Who decides"
/// and the race's family words wait for).
pub fn contacted(w: &World, id: u32) -> bool {
    contacts(w, id) > 0
}

/// The reads of these players (those with a file), read once a day.
pub fn reads(w: &World, staff: Staff, ids: &[u32]) -> Arc<HashMap<u32, Read>> {
    let key = cache_key(w, staff);
    let mut guard = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if guard.as_ref().is_none_or(|c| c.key != key) {
        *guard = Some(Cache { key, reads: Arc::new(HashMap::new()) });
    }
    let have = guard.as_ref().map(|c| c.reads.clone()).unwrap_or_default();
    let missing: Vec<u32> =
        ids.iter().copied().filter(|id| !have.contains_key(id) && has_file(w, staff, *id)).collect();
    if missing.is_empty() {
        return have;
    }
    let fresh: Vec<(u32, Read)> =
        missing.par_iter().filter_map(|&id| read_one(w, staff, id).map(|r| (id, r))).collect();
    let mut all: HashMap<u32, Read> = (*have).clone();
    all.extend(fresh);
    let all = Arc::new(all);
    if let Some(c) = guard.as_mut() {
        c.reads = all.clone();
    }
    all
}

fn read_one(w: &World, staff: Staff, id: u32) -> Option<Read> {
    let (rds, looks, f) = readers::read(w, staff, id)?;
    let attrs: Box<[(u8, u8)]> = f
        .attrs
        .iter()
        .map(|a| {
            if a.seen { (a.centre.round().clamp(1.0, 99.0) as u8, a.sd.round().clamp(1.0, 60.0) as u8) } else { (0, 0) }
        })
        .collect();
    let traits: Box<[(f32, f32)]> =
        f.traits.iter().map(|t| if t.seen { (t.centre as f32, t.sd.max(0.1) as f32) } else { (0.0, 0.0) }).collect();
    let film = looks.iter().filter(|l| matches!(l.kind.family(), "Film" | "Live")).count() as u16;
    let judge = rds.iter().map(|r| r.judge).fold(0.0f64, f64::max) as f32;
    let own = w.scouting.dept.looks.get(&id);
    let has = |k: &[Kind]| own.is_some_and(|v| v.iter().any(|l| k.contains(&l.kind)));
    let contact = contacts(w, id) as u16;
    let seen_now = f.now.1 > 0.0 || f.now.0 > 0.0;
    Some(Read {
        now: if seen_now { (f.now.0 as f32, f.now.1 as f32) } else { (0.0, 0.0) },
        ceiling: (f.ceiling.0 as f32, f.ceiling.1 as f32),
        attrs,
        traits,
        medical: (f.medical.grade, f.medical.examined),
        looks: looks.len() as u16,
        film,
        judge,
        camp: has(&[Kind::Camp]),
        transcript: has(&[Kind::Transcript]),
        contact,
    })
}

/// Forgets the kept reads (a test's fresh world).
#[cfg(test)]
pub fn forget() {
    *CACHE.lock().unwrap_or_else(|e| e.into_inner()) = None;
}
