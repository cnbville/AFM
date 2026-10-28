//! Your own read beside your staff's (scouting update SK23, SK24, SK27):
//! every player has two reads, your staff's from all of its looks and
//! yours from your own looks only.
//!
//! Until you have seen him yourself, your read is a first impression from
//! what reaches your desk (your staff's word on him, his stars, a clip):
//! wrong in its centre by a lasting misjudgment of yours, not just wide.
//! Each look of your own narrows it and pulls it toward what you saw, by
//! your evaluation eye: a practice or a game in person most, film next, a
//! camp for what it measures, a visit only the talk attributes (the base
//! misreads of `file::sees`). Your own players are in your practices every
//! week (`readers::own_roster`), so your read of them is your own.

use super::file::{self, FileRead, Link};
use super::readers::{self, Staff};
use crate::rng::clamp;
use crate::world::World;
use crate::world::season::key_normal;

/// A first impression's misjudgment for an average eye, rating points of
/// overall (proposed, not confirmed: sd 3.5, so up to about 7 points off).
pub const IMPRESSION_SD: f64 = 3.5;

/// Your own read of a player.
#[derive(Clone, Debug)]
pub struct OwnRead {
    /// His overall now: centre and sd.
    pub now: (f64, f64),
    /// Each attribute your own looks read: centre and sd.
    pub attrs: Vec<Option<(f64, f64)>>,
    /// Your own looks written up (0: a first impression only).
    pub looks: usize,
}

impl OwnRead {
    /// The overall as a range on 1-99, the way the file shows one.
    pub fn range(&self) -> (u8, u8) {
        range(self.now)
    }
}

/// A (centre, sd) as a 1-99 range, the file's way (`AttrRead::range`).
pub fn range(r: (f64, f64)) -> (u8, u8) {
    let half = r.1.max(1.0);
    let lo = (r.0 - half).floor().clamp(1.0, 99.0) as u8;
    let hi = (r.0 + half).ceil().clamp(1.0, 99.0) as u8;
    (lo, hi.max(lo))
}

/// Your evaluation eye (1-20), or None without a career.
pub fn your_eye(w: &World) -> Option<f64> {
    let c = readers::you(w)?;
    w.coaches.get(c).map(|c| c.evaluation())
}

/// Your first impression of a player from your staff's read of him: their
/// centre, moved by your own lasting misjudgment (keyed by you and him, so
/// it holds), and wider than theirs.
pub fn impression(w: &World, id: u32, staff: &FileRead) -> Option<(f64, f64)> {
    let you = readers::you(w)?;
    let eye = your_eye(w)?;
    let miss = IMPRESSION_SD * file::sharp(eye);
    let lean = key_normal(0x1F57_1A55 ^ (w.coaches[you].id as u64) << 24, id as u64);
    let centre = clamp(staff.now.0 + miss * lean, 1.0, 99.0);
    Some((centre, (staff.now.1 * staff.now.1 + miss * miss).sqrt().max(3.0)))
}

/// Your own read: your looks only, read as your file reads them, combined
/// with your first impression where your staff has one. None while
/// nothing of him has reached you.
pub fn own_read(w: &World, staff: Staff, id: u32, staff_read: Option<&FileRead>) -> Option<OwnRead> {
    let i = super::place(w, id)?;
    readers::you(w)?;
    let (rds, looks, _) = readers::evidence(w, staff, id);
    let me = rds.iter().position(|r| r.link == Link::You);
    let mine = me.and_then(|r| {
        let mut rd = rds[r].clone();
        // Alone in the room, your say is your weight.
        rd.say = 1.0;
        let lk: Vec<file::Evidence> = looks
            .iter()
            .filter(|l| l.reader == r)
            .filter(|l| l.date.add(rd.lag(l.kind, id, l.date)) <= w.today)
            .map(|l| file::Evidence { reader: 0, ..*l })
            .collect();
        if lk.is_empty() {
            return None;
        }
        let n = lk.len();
        let f = file::read(
            w,
            i,
            std::slice::from_ref(&rd),
            &lk,
            &readers::norms(w, i),
            readers::medical_rating(w, staff),
            Some(w.today),
        );
        // Only what your own eye saw counts as yours: a shared measurement
        // isn't in your looks.
        let attrs = f.per_reader.first().cloned().unwrap_or_default();
        Some((f.now, attrs, n))
    });
    let first = staff_read.and_then(|f| impression(w, id, f));
    match (mine, first) {
        (Some((now, attrs, n)), Some(imp)) => {
            let (a, b) = (1.0 / (imp.1 * imp.1), 1.0 / (now.1 * now.1).max(0.25));
            Some(OwnRead { now: ((imp.0 * a + now.0 * b) / (a + b), (1.0 / (a + b)).sqrt()), attrs, looks: n })
        }
        (Some((now, attrs, n)), None) => Some(OwnRead { now, attrs, looks: n }),
        (None, Some(imp)) => {
            Some(OwnRead { now: imp, attrs: vec![None; crate::attributes::ATTRIBUTE_COUNT], looks: 0 })
        }
        (None, None) => None,
    }
}
