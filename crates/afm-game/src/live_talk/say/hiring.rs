//! LT18, interviews when you hire staff (live talks, the core's proof). A
//! typed question lands on one of the interview's four questions (60 to 63)
//! or on "Thank him and wrap it up" (69), through the core's generic reader.
//! His answer is written from what the game released for that question:
//! today's answer (the line the game wrote), and the reads that question
//! puts on his page, in words. No number reaches the writer.

use afm_engine::world::World;
use afm_engine::world::coach::{Group, Role};
use afm_engine::world::folk::coach_look::{self, AMBITION, FACETS};
use afm_engine::world::folk::scene::{Kind, Scene};
use afm_engine::world::folk::staffing;

use crate::live_talk::card::Card;

/// The reads each question puts on his page: the same facets as
/// `staffing::interview_facets` (60, his scheme and his teaching for the
/// role; 61, relationships and discipline; 62, recruiting and network; 63,
/// ambition). A test holds the two together.
pub fn facets(code: u32, role: Role) -> Vec<usize> {
    match code {
        60 => {
            let mut v = vec![8];
            match role {
                Role::PositionCoach(g) => v.push(g as usize),
                Role::OffensiveCoordinator | Role::DefensiveCoordinator => {
                    let off = role == Role::OffensiveCoordinator;
                    v.extend(
                        Group::ALL
                            .iter()
                            .filter(|g| g.offense() == off && **g != Group::Specialists)
                            .map(|g| *g as usize),
                    );
                }
                Role::HeadCoach => {}
            }
            v
        }
        61 => vec![13, 12],
        62 => vec![14, 15],
        63 => vec![AMBITION],
        _ => Vec::new(),
    }
}

/// What your read of him shows for one facet, in words: the page's range,
/// told the way a staff member would say it. Proposed, not confirmed: a
/// range 9 or wider is "still hard to read"; its middle 16 and up is "a
/// real strength", 13 "good", 9 "about average", 6 "below average", lower
/// "a weak spot". Ambition: 14 "wants more, and soon", 9 "some ambition",
/// lower "content where he is".
pub fn read_words(w: &World, c: usize, f: usize) -> String {
    let (lo, hi) = coach_look::read(w, c, f);
    let mid = (f32::from(lo) + f32::from(hi)) / 2.0;
    let name = if f == AMBITION { "Ambition" } else { FACETS.get(f).map_or("His work", |x| x.1) };
    let words = if hi.saturating_sub(lo) >= 9 {
        "still hard to read"
    } else if f == AMBITION {
        if mid >= 14.0 {
            "wants more, and soon"
        } else if mid >= 9.0 {
            "some ambition"
        } else {
            "content where he is"
        }
    } else if mid >= 16.0 {
        "a real strength"
    } else if mid >= 13.0 {
        "good"
    } else if mid >= 9.0 {
        "about average"
    } else if mid >= 6.0 {
        "below average"
    } else {
        "a weak spot"
    };
    format!("{name}: {words}")
}

/// The writer's hook for a staff interview: the reads the question put on
/// his page, in words.
pub fn writer(w: &World, s: &Scene, code: u32, card: &mut Card) {
    if s.kind != Kind::Interview {
        return;
    }
    let c = s.a as usize;
    if c >= w.coaches.len() {
        return;
    }
    for f in facets(code, staffing::role_from(s.b)) {
        card.released.push(read_words(w, c, f));
    }
}
