//! A prospect remembers you (question 10: "everyone remembers now"). How
//! you treated him in a room (respect, a cheap shot, a talk that went hot)
//! moves his trust in you, kept by his key like anyone you talk to who
//! isn't yours (lane N1's `Mine.trust`). It follows him:
//!
//! - into his next room with you (lane N1: patience after a hot talk, and
//!   his trust as the talk opens);
//! - into the undrafted rush, where your club's call weighs his word of you
//!   beside the path to the roster, the money and a coach who knew him
//!   (`rush_pull`);
//! - onto your roster: once he's yours, his trust in you as his head coach
//!   starts halfway from where his word of you stood (`week`);
//! - and a man you had in for a visit or a workout who thought you'd take
//!   him, and watched you pass with the pick in hand, thinks a little less
//!   of you (`passed_on`), unless you call him.
//!
//! Sizes are Claude's picks, proposed, not confirmed.

use super::super::Them;
use super::super::makeup;
use crate::player::Trait;
use crate::world::World;

/// How much his word of you weighs in the rush, in roster places of path
/// per point of trust over 0.5 (a coach who knew him from college counts
/// 0.8). Proposed, not confirmed.
pub const RUSH: f64 = 1.0;
/// How much of the way his trust in you as his coach starts toward his
/// word of you. Proposed, not confirmed.
pub const CARRY: f32 = 0.5;
/// What passing on a man who thought you'd take him costs your word, plus
/// his ego. Proposed, not confirmed.
pub const PASSED: f32 = 0.03;
/// How far past your pick he still thought it was you, in picks.
pub const NEAR: u32 = 40;

/// His word of you, when you've talked (None when you never have).
pub fn word(w: &World, id: u32) -> Option<f32> {
    w.folk.mine.trust_of(makeup::key(Them::Player(id)))
}

/// Your club's call in the rush, by his word of you (0 for every other
/// club, and for a man you never talked to: the AI world is unchanged).
pub fn rush_pull(w: &World, i: usize, team: usize) -> f64 {
    if super::club(w) != Some(team) {
        return 0.0;
    }
    word(w, w.people[i].player.id).map_or(0.0, |t| RUSH * (t as f64 - 0.5))
}

/// Mondays: a man who has joined you carries his word of you into his
/// trust in you as his head coach, once.
pub(crate) fn week(w: &mut World) {
    let Some(t) = super::club(w) else { return };
    let Some(you) = crate::world::scouting::readers::you(w) else { return };
    let Some(club) = w.clubs.get(t) else { return };
    let roster: Vec<usize> = club.roster.clone();
    for i in roster {
        let id = w.people[i].player.id;
        if w.people[i].trust_in != you as u32 || w.folk.live.persona.draft.joined.contains(&id) {
            continue;
        }
        let Some(word) = word(w, id) else { continue };
        let p = &mut w.people[i];
        p.trust = (p.trust + CARRY * (word - p.trust)).clamp(0.0, 1.0);
        w.folk.live.persona.draft.joined.push(id);
    }
}

/// After the draft: each man you had in for a visit or a private workout
/// this spring who went within `NEAR` picks after one of
/// yours to someone else (or went undrafted) thinks you passed on him.
pub(crate) fn passed_on(w: &mut World) {
    use crate::world::scouting::predraft::{LIST_VISIT, LIST_WORKOUT};
    let Some(mine) = super::club(w) else { return };
    let year = w.year;
    let mut ids: Vec<u32> =
        w.scouting.predraft.lists.iter().filter(|x| x.1 == LIST_VISIT || x.1 == LIST_WORKOUT).map(|x| x.0).collect();
    ids.sort();
    ids.dedup();
    let picks: Vec<u32> = w
        .people
        .iter()
        .filter_map(|p| {
            let (y, _, pick, t) = p.pro.as_ref()?.draft?;
            (y == year && t == mine).then_some(pick)
        })
        .collect();
    if picks.is_empty() {
        return;
    }
    let today = w.today;
    for id in ids {
        let Some(i) = crate::world::scouting::place(w, id) else { continue };
        let went = w.people[i].pro.as_ref().and_then(|c| c.draft).filter(|d| d.0 == year);
        let thought = match went {
            Some((_, _, _, t)) if t == mine => false,
            Some((_, _, pk, _)) => picks.iter().any(|&m| m < pk && pk <= m + NEAR),
            None => true,
        };
        if !thought || w.folk.live.persona.draft.passed.iter().any(|x| x.0 == id && x.1.year() == year) {
            continue;
        }
        let ego = w.people[i].player.personality.traits.z(Trait::Ego).max(0.0) as f32;
        let key = makeup::key(Them::Player(id));
        let now = makeup::trust(w, Them::Player(id)) as f32;
        w.folk.mine.set_trust(key, (now - PASSED * (1.0 + ego)).clamp(0.0, 1.0));
        w.folk.live.persona.draft.passed.push((id, today));
    }
}
