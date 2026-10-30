//! What your staff knows (scouting topic 1, screens topic 4): every rating
//! the screens show is a range your staff reads from its looks, never the
//! true number (minor bug 54: the old simple read, the truth with a blur,
//! is retired). The read is the scouting file's (`world/scouting/file.rs`,
//! `read`) on every screen, so the roster, the player page, the board and
//! the file never disagree; your own players are in your coaches' practice
//! and game looks every week (`readers::own_roster`). A player nobody on
//! your staff has looked at is "Not seen", and a better staff reads truer,
//! not just narrower: the truth can sit outside a range.
//!
//! Beside it, your own read (SK23, SK24, SK27): your looks only, a first
//! impression until you have seen him yourself
//! (`world/scouting/yours.rs`).
//!
//! The reads are kept for the day (until the day, your seat or the looks
//! change), so a roster of a hundred reads each file once.
//!
//! The one exception: a seat in another league has no scouting staff of
//! its own in the game yet, so its own players keep the coaches' simple
//! read (`simple`) and everyone else is "Not seen".

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use afm_engine::attributes::{ATTRIBUTE_COUNT, Attr};
use afm_engine::player::Player;
use afm_engine::world::scouting::readers::{self, Staff};
use afm_engine::world::scouting::{file, yours};
use afm_engine::world::{Stage, World};

/// A read of one rating: low and high, or not seen.
#[derive(Clone, Copy, Debug)]
pub enum Read {
    Range(u8, u8),
    NotSeen,
}

impl Read {
    pub fn json(self) -> serde_json::Value {
        match self {
            Read::Range(lo, hi) => serde_json::json!([lo, hi]),
            Read::NotSeen => serde_json::Value::Null,
        }
    }
    fn of(r: Option<(u8, u8)>) -> Read {
        r.map_or(Read::NotSeen, |(lo, hi)| Read::Range(lo, hi))
    }
    /// The middle of the range, for sorting (None while not seen).
    pub fn mid(self) -> Option<f64> {
        match self {
            Read::Range(lo, hi) => Some((lo as f64 + hi as f64) / 2.0),
            Read::NotSeen => None,
        }
    }
}

/// One player's reads as your seat has them today.
#[derive(Clone, Debug, Default)]
pub struct Seen {
    /// Your staff's read of each attribute (None: nobody saw it).
    pub attrs: Vec<Option<(u8, u8)>>,
    /// Your staff's overall now: centre and sd (None: not seen).
    pub now: Option<(f64, f64)>,
    /// Where your staff thinks he could get to, in words.
    pub tier: Option<&'static str>,
    pub sure: Option<&'static str>,
    pub hints: Vec<String>,
    /// Looks in his file.
    pub looks: usize,
    /// Your staff's read of him at the four positions it reads him best
    /// at, best first: position and range.
    pub positions: Vec<(afm_engine::positions::Position, (u8, u8))>,
    /// Your own read (SK23): overall now, each attribute you saw, and your
    /// own looks behind it (0: a first impression).
    pub mine: Option<(f64, f64)>,
    pub mine_attrs: Vec<Option<(u8, u8)>>,
    pub mine_looks: usize,
}

impl Seen {
    pub fn overall(&self) -> Read {
        Read::of(self.now.map(yours::range))
    }
    pub fn mine(&self) -> Read {
        Read::of(self.mine.map(yours::range))
    }
}

type Key = (i32, u64, usize, u64);
type Kept = (Key, HashMap<u32, Arc<Seen>>);
static CACHE: Mutex<Option<Kept>> = Mutex::new(None);

fn speak(stage: Stage) -> file::Speak {
    match stage {
        Stage::HighSchool | Stage::PrepSchool => file::Speak::Recruit,
        Stage::Pro => file::Speak::Pro,
        _ => file::Speak::Prospect,
    }
}

fn read_one(w: &World, staff: Staff, k: usize) -> Seen {
    let p = &w.people[k];
    let id = p.player.id;
    let staff_read = readers::read(w, staff, id);
    let mut s =
        Seen { attrs: vec![None; ATTRIBUTE_COUNT], mine_attrs: vec![None; ATTRIBUTE_COUNT], ..Default::default() };
    if let Some((rds, looks, f)) = &staff_read {
        s.attrs = f.attrs.iter().map(|a| a.range()).collect();
        s.now = Some(f.now);
        s.tier = Some(file::tier_words(speak(p.stage), f.ceiling.0));
        s.sure = Some(file::sure_words(f.ceiling.1));
        s.hints = file::hints(w, k, rds, looks).into_iter().map(|h| h.0).collect();
        s.looks = looks.len();
        let mut all: Vec<(afm_engine::positions::Position, f64)> = afm_engine::positions::Position::ALL
            .iter()
            .map(|&pos| (pos, afm_engine::positions::overall_for(pos, &f.centres)))
            .collect();
        all.sort_by(|a, b| b.1.total_cmp(&a.1));
        s.positions = all.into_iter().take(4).map(|(pos, c)| (pos, yours::range((c, f.now.1 + 1.0)))).collect();
    }
    if let Some(o) = yours::own_read(w, staff, id, staff_read.as_ref().map(|x| &x.2)) {
        s.mine = Some(o.now);
        s.mine_attrs = o.attrs.iter().map(|a| a.map(yours::range)).collect();
        s.mine_looks = o.looks;
    }
    s
}

/// A seat in another league: its own players read by their coaches.
fn other_seat(w: &World, k: usize) -> Seen {
    let p = &w.people[k];
    let mut s =
        Seen { attrs: vec![None; ATTRIBUTE_COUNT], mine_attrs: vec![None; ATTRIBUTE_COUNT], ..Default::default() };
    let Some(t) = crate::views::mine(w) else { return s };
    if crate::views::team_of(w, p) != Some(t) {
        return s;
    }
    let eye = crate::views::eye_for(w, t, p.player.position);
    let key = crate::views::reader_key(w);
    for (a, r) in simple(&p.player, eye, key) {
        if let Read::Range(lo, hi) = r {
            s.attrs[a.index()] = Some((lo, hi));
        }
    }
    let half = 2.0 + (20.0 - eye.clamp(1.0, 20.0)) * 0.12;
    if let Read::Range(lo, hi) = range(p.player.overall(), half, key, p.player.id, 99) {
        s.now = Some(((lo as f64 + hi as f64) / 2.0, (hi as f64 - lo as f64) / 2.0));
    }
    s
}

/// Your seat's reads of a player (by his place in the people list), kept
/// for the day.
pub fn seen(w: &World, k: usize) -> Arc<Seen> {
    let Some(p) = w.people.get(k) else { return Arc::new(Seen::default()) };
    let id = p.player.id;
    let Some(staff) = readers::seat_staff(w) else { return Arc::new(other_seat(w, k)) };
    let key = crate::lane_find::staff::cache_key(w, staff);
    {
        let guard = CACHE.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((kk, m)) = guard.as_ref()
            && *kk == key
            && let Some(s) = m.get(&id)
        {
            return s.clone();
        }
    }
    let s = Arc::new(read_one(w, staff, k));
    let mut guard = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if guard.as_ref().is_none_or(|c| c.0 != key) {
        *guard = Some((key, HashMap::new()));
    }
    if let Some((_, m)) = guard.as_mut() {
        m.insert(id, s.clone());
    }
    s
}

/// Your staff's read of each attribute.
pub fn attributes(w: &World, k: usize) -> Vec<(Attr, Read)> {
    let s = seen(w, k);
    Attr::ALL.iter().map(|&a| (a, Read::of(s.attrs.get(a.index()).copied().flatten()))).collect()
}

/// Your own read of each attribute (SK23).
pub fn mine_attributes(w: &World, k: usize) -> Vec<(Attr, Read)> {
    let s = seen(w, k);
    Attr::ALL.iter().map(|&a| (a, Read::of(s.mine_attrs.get(a.index()).copied().flatten()))).collect()
}

/// Your staff's read of his overall now.
pub fn overall(w: &World, k: usize) -> Read {
    seen(w, k).overall()
}

/// Your own read of his overall now (SK23).
pub fn mine(w: &World, k: usize) -> Read {
    seen(w, k).mine()
}

/// Where your staff puts him now, for sorting (None: not seen).
pub fn centre(w: &World, k: usize) -> Option<f64> {
    seen(w, k).now.map(|n| n.0)
}

/// Potential in words only (scouting topic 7): the tier your staff's
/// ceiling read lands in, how sure it is, and the hints its best eye for
/// potential picked up. Null while not seen.
pub fn potential_words(w: &World, k: usize) -> serde_json::Value {
    let s = seen(w, k);
    match s.tier {
        Some(tier) => serde_json::json!({ "tier": tier, "sure": s.sure.unwrap_or("unsure"), "hints": s.hints }),
        None => serde_json::Value::Null,
    }
}

/// The public read of another team's player nobody of yours has looked at,
/// for comparing your team with its rivals: what every staff sees of its
/// opponents on game film and television. It carries the public's own
/// lasting misread (keyed by the player, the same for every seat; wider
/// for high schoolers and small schools, narrower for pros), so the truth
/// can sit outside it (proposed, not confirmed: sd 4 points at FBS,
/// x the film's visibility, shown +-5).
pub fn public(w: &World, k: usize) -> Read {
    let Some(p) = w.people.get(k) else { return Read::NotSeen };
    let vis = file::visibility(w, k);
    let miss = 4.0 * vis * afm_engine::world::key_normal(0x009A_B11C, p.player.id as u64);
    let c = p.player.overall() + miss;
    Read::of(Some(yours::range((c, 5.0 * vis))))
}

/// Forgets the kept reads (a test's fresh world).
#[cfg(test)]
pub fn forget() {
    *CACHE.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

/// A fixed lean in -1..1 for a reader, player and attribute.
fn lean(reader: u64, player: u32, attr: usize) -> f64 {
    let mut z = reader ^ (player as u64) << 20 ^ (attr as u64) << 52 ^ 0xA5F0_1C3D;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
}

/// A range around a true value, `half` wide each side, leaning by the reader.
pub fn range(truth: f64, half: f64, reader: u64, player: u32, attr: usize) -> Read {
    let centre = truth + lean(reader, player, attr) * half * 0.6;
    let lo = (centre - half).round().clamp(1.0, 99.0) as u8;
    let hi = (centre + half).round().clamp(1.0, 99.0) as u8;
    Read::Range(lo, hi.max(lo))
}

/// Whether your staff has measured a high schooler (a camp or a showcase look).
pub fn hs_tested(w: &afm_engine::world::World, id: u32) -> bool {
    use afm_engine::world::scouting::Kind;
    w.scouting.dept.looks.get(&id).is_some_and(|v| v.iter().any(|l| matches!(l.kind, Kind::Camp | Kind::Showcase)))
}

/// Your staff's read of his Dynamic Links (`docs/design/dynamic-links.md`):
/// each link it sees, how strong it looks and the numbers it is named by.
/// Your own players are measured by your staff; a pro or a combine invite
/// has public numbers; anyone else is a guess from film, which can miss a
/// link or see one that isn't there. A better athletic eye reads truer.
pub fn links(p: &Player, own: bool, measured: bool, eye: f64, reader: u64) -> Vec<serde_json::Value> {
    use afm_engine::links::{Link, links_of, numbers, projected, strength_words};
    // A boy still growing is read for what his frame and testing project
    // to, in words only (his grown numbers would give his ceiling away).
    let young = p.age < 21 && !measured;
    let truth = if young { projected(p) } else { links_of(p) };
    let body = afm_engine::body::body_now(p);
    let noise = if young {
        0.3 - if own { 0.008 * eye.clamp(1.0, 20.0) } else { 0.0 }
    } else if own || measured {
        0.06 + (20.0 - eye.clamp(1.0, 20.0)) * 0.006
    } else {
        0.25
    };
    let mut out: Vec<(f64, serde_json::Value)> = Link::ALL
        .iter()
        .filter_map(|&l| {
            let seen = (truth.get(l) + noise * lean(reader, p.id, 300 + l.index())).clamp(0.0, 1.0);
            (seen >= 0.2).then(|| {
                let shown = if young {
                    format!("Projects: {}", l.film())
                } else if own || measured {
                    numbers(l, &body, &p.attributes)
                } else {
                    format!("From film: {}", l.film())
                };
                (
                    seen,
                    serde_json::json!({
                        "name": l.name(),
                        "strength": strength_words(seen),
                        "numbers": shown,
                        "edge": l.edge(),
                        "measured": !young && (own || measured),
                    }),
                )
            })
        })
        .collect();
    out.sort_by(|a, b| b.0.total_cmp(&a.0));
    out.into_iter().map(|x| x.1).collect()
}

/// The coaches' simple read of their own player, kept only for a seat in
/// another league (see the top): each attribute within 2 to 4.3 points by
/// the coach's eye.
fn simple(p: &Player, eye: f64, reader: u64) -> Vec<(Attr, Read)> {
    let half = 2.0 + (20.0 - eye.clamp(1.0, 20.0)) * 0.12;
    Attr::ALL.iter().map(|&a| (a, range(p.attributes.get(a), half, reader, p.id, a.index()))).collect()
}
