//! The captain promise (2026-11-18, GAPS "You'll be a captain"): telling
//! your player he'll be a captain is a promise the game holds you to.
//!
//! Real captains are named before the season, at college usually by a
//! team vote (some head coaches pick their own), and in the NFL by the
//! players' vote at camp. So the promise is judged the day your room names
//! its captains (`people::settle_room`: college fall camp, the NFL's
//! veterans report), never on games.
//!
//! - The room votes as it always does. Voted in, the promise is kept.
//! - Not voted in, you keep your word: a head coach has the last say on
//!   his captains, so you name him one more captain beside the men the
//!   room voted. The room sees it: your name with the room drops by how
//!   far down its vote he was, and the men it put ahead of him who aren't
//!   captains feel passed over. A coach who picks his own captains anyway
//!   pays half: his room expects his picks.
//! - Turned off (the agenda's plan setting, `usage::set_off`), you let the
//!   vote stand: voted in he has it, else you broke it.
//! - A condition he missed ("if you keep your grades up") releases you: you
//!   don't name him over the vote.
//! - Promised where the room can hear (he tells his friends, a loud
//!   parent, a press conference), the room learns you promised a captaincy
//!   before the vote, and your name with the room drops a little.
//!
//! What he believes of it reads the vote as he sees it: his own place in
//! the room (ego lifts it), against how many captains it names; under a
//! coach who picks his own, the coach can deliver it.
//!
//! Every size here is Claude's (proposed, not confirmed).

use super::talks::{PromiseKind, PromiseTo, Reason};
use super::{Name, Team};
use crate::calendar::Date;
use crate::player::Trait;
use crate::world::World;

/// Days past the naming before a captain promise that never met a vote is
/// judged on the captains as they stand (he left, the room never voted).
pub const GRACE: i32 = 14;

/// Your name with the room for each man the room put ahead of him that you
/// passed to name him (at most `PASSED_MAX`). Proposed, not confirmed.
pub const OVER_ROOM: f32 = 0.02;

/// The most men passed that count against you.
pub const PASSED_MAX: usize = 6;

/// The men passed over who feel it: the first few ahead of him.
pub const FELT: usize = 3;

/// What a passed-over man loses: trust in you and mood.
pub const PASSED_TRUST: f32 = 0.02;
pub const PASSED_MOOD: f32 = -0.03;

/// Your name with the room when it hears you promised a captaincy before
/// the vote.
pub const HEARD_ROOM: f32 = 0.03;

/// The day your room next names its captains: college fall camp, the NFL's
/// veterans report (next year's once this year's is past). None where the
/// world names no captains (a high school, another league).
pub fn next_naming(w: &World) -> Option<Date> {
    let d = match Team::seat(w)? {
        Team::College(_) => w.dates.fall_camp,
        Team::Nfl(_) => w.dates.veterans_report,
        _ => return None,
    };
    Some(if d > w.today { d } else { d.add(364) })
}

/// Whether you can promise him a captaincy: your college or NFL team, and
/// he's on it.
pub fn open(w: &World, i: usize) -> bool {
    next_naming(w).is_some() && Team::seat(w).is_some_and(|t| t.roster(w).contains(&i))
}

/// How likely it looks to him: within the captains his read of the vote
/// names (his own score lifted by his ego), 0.9; each place beyond them
/// 0.1 less, to 0.35. A man the room can't vote yet (a college first-year)
/// reads it at the floor. Under a coach who picks his own captains, it's
/// the coach's to give: 0.9.
pub fn likely(w: &World, i: usize) -> f64 {
    let Some(team) = Team::seat(w) else { return 0.5 };
    let Some(crate::world::people::RoomRead { votes, strict, n }) = crate::world::people::room_read(w, team) else {
        return 0.5;
    };
    if strict {
        return 0.9;
    }
    let Some(mine) = votes.iter().find(|v| v.0 == i).map(|v| v.1) else { return 0.35 };
    let ego = w.people[i].player.personality.traits.z(Trait::Ego).max(0.0);
    let me = mine + 1.5 * ego;
    let ahead = votes.iter().filter(|v| v.0 != i && v.1 > me).count();
    if ahead < n { 0.9 } else { (0.9 - 0.1 * (ahead + 1 - n) as f64).max(0.35) }
}

/// Your open captain promises to men on `team`'s roster: (promise index,
/// his index).
fn open_promises(w: &World, team: Team) -> Vec<(usize, usize)> {
    let roster = team.roster(w);
    (0..w.folk.promises.len())
        .filter_map(|k| {
            let p = &w.folk.promises[k];
            let PromiseTo::Player(id) = p.to else { return None };
            if p.kind != PromiseKind::Captain || p.kept.is_some() || p.made > w.today {
                return None;
            }
            let i = super::person(w, id)?;
            roster.contains(&i).then_some((k, i))
        })
        .collect()
}

/// When your room names its captains: each man you promised a captaincy
/// whom the vote left out, you name beside them, unless you turned the
/// promise's plan setting off or he missed its condition. `votes` is the
/// room's vote, best first; `captains` the men it named, to which he is
/// added. The room's costs land here.
pub(crate) fn name(w: &mut World, team: Team, votes: &[(usize, f64)], strict: bool, captains: &mut Vec<usize>) {
    let n = captains.len();
    for (k, i) in open_promises(w, team) {
        if captains.contains(&i) {
            continue;
        }
        let p = w.folk.promises[k].clone();
        if p.terms.off || super::promise::cond_now(w, i, &p) == Some(false) {
            continue;
        }
        // The men the room put ahead of him who aren't captains: all of the
        // rest for a man it couldn't vote (a first-year).
        let rank = votes.iter().position(|v| v.0 == i).unwrap_or(votes.len());
        let passed: Vec<usize> =
            votes[n.min(rank)..rank].iter().map(|v| v.0).filter(|j| !captains.contains(j)).collect();
        let over = if rank >= votes.len() { PASSED_MAX } else { passed.len().min(PASSED_MAX) };
        let half = if strict { 0.5 } else { 1.0 };
        w.folk.mine.word(Name::Room, -OVER_ROOM * over as f32 * half);
        for &j in passed.iter().take(FELT) {
            let q = &mut w.people[j];
            q.trust = (q.trust - PASSED_TRUST * half).max(0.0);
            let id = q.player.id;
            super::talks::mood_event(w, id, Reason::Room, PASSED_MOOD * half);
        }
        captains.push(i);
        let who = super::player_link(w, i);
        let how = if strict {
            "You named your captains, and he is one of them, as you told him.".to_string()
        } else if over == 0 {
            "The room's vote left him just outside; you named him a captain beside the men it voted, as you told him."
                .to_string()
        } else {
            let first = passed.first().map(|&j| super::player_link(w, j));
            match first {
                Some(f) if passed.len() > 1 => format!(
                    "The room's vote left him out; you named him a captain beside the men it voted, as you told him. The room noticed: {f} and others got more votes."
                ),
                Some(f) => format!(
                    "The room's vote left him out; you named him a captain beside the men it voted, as you told him. The room noticed: {f} got more votes."
                ),
                None => "The room couldn't vote him yet; you named him a captain yourself, as you told him. The room noticed."
                    .to_string(),
            }
        };
        super::note(w, "talk", "Your promises".into(), format!("{who}: {how}"), None);
    }
}

/// After your room named its captains: every open captain promise to a man
/// on it is judged today (`promise::judge`, `talks::check_promises`).
pub(crate) fn named(w: &mut World, team: Team) {
    let today = w.today;
    let due: Vec<usize> = open_promises(w, team).into_iter().map(|x| x.0).collect();
    if due.is_empty() {
        return;
    }
    for k in due {
        w.folk.promises[k].due = today;
    }
    super::talks::check_promises(w, today);
}

/// How a captain promise came out, from the captains your room has now:
/// named, kept; else released when he missed its condition; else broken.
pub fn verdict(w: &World, p: &super::talks::Promise, i: usize) -> super::promise::Verdict {
    use super::promise::Verdict;
    let named =
        Team::seat(w).is_some_and(|t| t.roster(w).contains(&i) && super::takeover::captains_of(w, t).contains(&i));
    if named {
        Verdict::Kept
    } else if super::promise::cond_now(w, i, p) == Some(false) {
        Verdict::Released(0.0)
    } else {
        Verdict::Broken(1.0)
    }
}

/// The room hears you promised a captaincy before the vote (the promised
/// man told his friends, a loud parent, a press conference): your name with
/// the room drops. Returns what happened, in words.
pub(crate) fn heard(w: &mut World, name: &str) -> String {
    w.folk.mine.word(Name::Room, -HEARD_ROOM);
    format!("Word got around the room that you promised {name} a captaincy before the vote.")
}
