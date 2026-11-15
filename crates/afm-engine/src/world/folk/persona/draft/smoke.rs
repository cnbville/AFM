//! Smoke (question 10: "reading smoke on calls", "planting smoke"). Every
//! spring clubs leak interest, some of it real and some of it planted to
//! move other clubs (`predraft::leaks`), and a GM on the phone with a rival
//! rarely says who he really likes.
//!
//! Reading it: your staff never knows which is which. It weighs what it
//! can see: how that club's leaks held up in the drafts you watched (each
//! draft shows it: a club that took the man it leaked was real, one that
//! passed on him with the pick in hand was smoke), whether your people saw
//! the club do real work on him (a visit, a private workout), whether he
//! fills a hole on their roster, and whether he fits where they pick. Each
//! is seen only as far as your scouting director's network reaches.
//!
//! Planting it: on a call you tell a GM you love a man (only his club
//! hears it), or you tell an agent (it spreads to every club, as a press
//! leak does). Other clubs read it the way they read any leak
//! (`predraft::leak_pull`). A GM you fed smoke sees it on draft day when
//! you pass on the man with the pick in hand, and trusts your word a
//! little less.
//!
//! Sizes are Claude's picks, proposed, not confirmed.

use super::super::Them;
use crate::calendar::Date;
use crate::world::World;
use crate::world::scouting::predraft::Leak;
use crate::world::season::key_uniform;

/// What a club told you on a call.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Told {
    pub team: usize,
    pub player: u32,
    pub date: Date,
    /// What he said was his club's real interest (the game knows; your
    /// staff only reads it).
    pub real: bool,
}

/// Smoke you planted on a call: with a GM (his club only) or an agent
/// (everyone).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Planted {
    pub to: Option<usize>,
    pub player: u32,
    pub date: Date,
    /// What the draft showed: you took him (real), you passed on him with
    /// the pick in hand (smoke), or not yet.
    pub shown: Option<bool>,
}

/// The base rates your staff starts from: how much press interest is real
/// in a spring, and how much a rival GM says on the phone. Proposed, not
/// confirmed (the built leaks plant 15-35% of the time).
pub const PRESS_PRIOR: f64 = 0.75;
pub const CALL_PRIOR: f64 = 0.55;
/// How much one club's past record weighs against the prior, in leaks.
pub const RECORD_WEIGHT: f64 = 4.0;
/// What a GM you fed smoke takes off your word. Proposed, not confirmed.
pub const SMOKE_COST: f32 = 0.03;

/// A club's GM on the phone: who he names when you ask who he likes, and
/// whether it's his real interest. His real interest is a man his club did
/// real work on (a visit); smoke is a top prospect he didn't. He lies more
/// the more he likes games (his overconfidence) and the less he trusts
/// you. Keyed to the club and the week, so asking again the same week gets
/// the same answer.
pub fn gm_says(w: &World, t: usize, about: Option<u32>) -> Option<(u32, bool)> {
    let club = w.clubs.get(t)?;
    let g = &club.office.gm;
    let trust = super::super::makeup::trust(w, super::gm(w, t));
    let lie = (0.25 + 0.2 * g.overconfidence.min(1.0) - 0.25 * (trust - 0.5)).clamp(0.1, 0.7);
    let week = (w.today.0 / 7) as u64;
    let lies = key_uniform(0x5A0E ^ (t as u64) << 20 ^ week << 40, about.unwrap_or(0) as u64) < lie;
    let visited: Vec<u32> = {
        let mut v: Vec<u32> = club
            .files
            .iter()
            .filter(|(_, v)| v.iter().any(|l| l.kind == crate::world::pro::scouts::LookKind::Visit))
            .map(|(&id, _)| id)
            .collect();
        v.sort();
        v
    };
    if let Some(id) = about {
        // "Do you like him?": a yes he means, or the opposite.
        let real = visited.contains(&id);
        return Some((id, real != lies));
    }
    let pool: Vec<u32> = if lies || visited.is_empty() {
        let mut v: Vec<u32> = crate::world::scouting::public::consensus(w).unwrap_or_default();
        v.retain(|x| !visited.contains(x));
        v
    } else {
        visited
    };
    if pool.is_empty() {
        return None;
    }
    let k = (key_uniform(0xC0D1 ^ t as u64, week) * pool.len() as f64) as usize;
    let id = pool[k.min(pool.len() - 1)];
    Some((id, w_visited(club, id)))
}

fn w_visited(club: &crate::world::pro::Club, id: u32) -> bool {
    club.files.get(&id).is_some_and(|v| v.iter().any(|l| l.kind == crate::world::pro::scouts::LookKind::Visit))
}

/// Your staff's read of one club's interest in a man: the odds it's real,
/// a band and the reasons it can see.
#[derive(Clone, Debug, PartialEq)]
pub struct Read {
    pub odds: f64,
    pub band: &'static str,
    pub why: Vec<String>,
}

/// How far your people see what other clubs do: your director of college
/// scouting's network, else 10.
fn reach(w: &World) -> f64 {
    let Some(t) = super::club(w) else { return 0.35 };
    let net = w
        .clubs
        .get(t)
        .and_then(|c| c.scouts.iter().find(|s| s.role == crate::world::pro::scouts::Role::Director))
        .map_or(10.0, |s| s.network as f64);
    0.35 + 0.4 * net / 20.0
}

/// Your staff's read of a club's interest in a man, from a leak in the
/// press (`call` false) or a GM on the phone (`call` true).
pub fn read(w: &World, team: usize, player: u32, call: bool) -> Read {
    let mut why = Vec::new();
    let prior = if call { CALL_PRIOR } else { PRESS_PRIOR };
    // Their record: what the drafts you watched showed of their leaks.
    let year = w.year;
    let (r, s) = w
        .folk
        .live
        .persona
        .draft
        .record
        .iter()
        .filter(|x| x.0 == team && year - x.1 <= 6)
        .fold((0.0, 0.0), |a, x| (a.0 + x.2 as f64, a.1 + x.3 as f64));
    let mut p = (prior * RECORD_WEIGHT + r) / (RECORD_WEIGHT + r + s);
    if r + s > 0.0 {
        why.push(format!(
            "Their leaks have held up {} of {} times in the drafts we watched.",
            r as u32,
            (r + s) as u32
        ));
    }
    let mut odds = p / (1.0 - p);
    let name = w.nfl.get(team).map_or("They", |c| c.name);
    // Real work on him, as far as your people see it.
    if let Some(club) = w.clubs.get(team) {
        let seen = key_uniform(0x5EE5 ^ (team as u64) << 24 ^ year as u64, player as u64) < reach(w);
        if seen {
            if w_visited(club, player) {
                odds *= 3.0;
                why.push(format!("Our people say the {name} had him in for a visit."));
            } else {
                odds *= 0.5;
                why.push(format!("Nobody saw the {name} do real work on him."));
            }
        }
    }
    if let Some(i) = crate::world::scouting::place(w, player) {
        let pos = w.people[i].player.position;
        // A hole on their roster at his spot.
        let top =
            |t: usize| w.nfl.get(t).map_or(0.0, |c| c.depth[pos.index()].iter().map(|m| m.rating).fold(0.0, f64::max));
        let mut tops: Vec<f64> = crate::world::franchise::active_teams_in(w, year)
            .into_iter()
            .filter(|&t| t < w.nfl.len())
            .map(top)
            .collect();
        tops.sort_by(|a, b| a.total_cmp(b));
        if !tops.is_empty() {
            let median = tops[tops.len() / 2];
            if top(team) < median - 2.0 {
                odds *= 1.3;
                why.push(format!("They need a {pos}."));
            } else if top(team) > median + 4.0 {
                odds *= 0.8;
                why.push(format!("They're set at {pos}."));
            }
        }
        // Where they pick against where he goes.
        let slot = crate::world::nfl_year::draft_order(w).and_then(|o| o.iter().position(|&x| x == team));
        let rank = crate::world::scouting::public::consensus(w).and_then(|v| v.iter().position(|&x| x == player));
        if let (Some(slot), Some(rank)) = (slot, rank) {
            let gap = rank as i64 - slot as i64;
            if gap.abs() <= 10 {
                odds *= 1.2;
                why.push("He fits where they pick.".into());
            } else if gap < -15 {
                odds *= 0.6;
                why.push("He'll be gone before they pick.".into());
            } else if gap > 25 {
                odds *= 0.7;
                why.push("He'd be a reach where they pick.".into());
            }
        }
    }
    p = odds / (1.0 + odds);
    let band = if p >= 0.65 {
        "Looks real"
    } else if p <= 0.4 {
        "Smells like smoke"
    } else {
        "Could be either"
    };
    Read { odds: p, band, why }
}

/// You plant smoke on a call (`Request::Plant`): with a GM, only his club
/// hears it; with an agent, it gets around the league. Returns what
/// happens, in words.
pub fn plant(w: &mut World, him: Them, player: u32) -> Option<String> {
    let mine = super::club(w)?;
    let i = crate::world::folk::person(w, player)?;
    let name = w.people[i].name.clone();
    let to = match him {
        Them::Cast(r, _) if r.role == crate::world::folk::cast::Role::Gm => match r.team {
            crate::world::folk::Team::Nfl(t) if t != mine => Some(t),
            _ => return None,
        },
        Them::Agent(_) => None,
        _ => return None,
    };
    let d = w.today;
    w.scouting.predraft.leaks.push(Leak { team: mine, player, date: d, real: false, yours: true, to });
    w.folk.live.persona.draft.planted.push(Planted { to, player, date: d, shown: None });
    Some(match to {
        Some(t) => format!("The {} now think you love {name}.", w.nfl.get(t).map_or("other club", |c| c.name)),
        None => format!("Word that you love {name} gets around the league."),
    })
}

/// A GM's answer on a call goes in the book, for your staff's read.
pub fn tell(w: &mut World, team: usize, player: u32, real: bool) {
    let d = w.today;
    let b = &mut w.folk.live.persona.draft;
    if !b.told.iter().any(|x| x.team == team && x.player == player && x.date.year() == d.year()) {
        b.told.push(Told { team, player, date: d, real });
    }
}

/// After the draft: what the picks showed of each club's leaks and of your
/// planted smoke. A club that took the man it leaked was real; one that
/// passed on him with its pick in hand was smoke; the rest show nothing.
pub(crate) fn settle(w: &mut World) {
    let year = w.year;
    let taken: std::collections::HashMap<u32, (usize, u32)> = w
        .people
        .iter()
        .filter_map(|p| {
            let (y, _, pick, team) = p.pro.as_ref()?.draft?;
            (y == year).then_some((p.player.id, (team, pick)))
        })
        .collect();
    let mut first: std::collections::HashMap<usize, u32> = std::collections::HashMap::new();
    for &(t, pk) in taken.values() {
        let e = first.entry(t).or_insert(pk);
        *e = (*e).min(pk);
    }
    // Did the club pass on him with its pick in hand?
    let shown = |team: usize, player: u32| -> Option<bool> {
        let f = first.get(&team).copied()?;
        match taken.get(&player) {
            Some(&(t, _)) if t == team => Some(true),
            Some(&(_, pk)) if pk > f => Some(false),
            Some(_) => None,
            // Undrafted: everyone passed on him.
            None => Some(false),
        }
    };
    let mut rec: std::collections::HashMap<usize, (u8, u8)> = std::collections::HashMap::new();
    let mine = super::club(w);
    for l in &w.scouting.predraft.leaks {
        if l.yours || Some(l.team) == mine {
            continue;
        }
        if let Some(real) = shown(l.team, l.player) {
            let e = rec.entry(l.team).or_default();
            if real {
                e.0 = e.0.saturating_add(1);
            } else {
                e.1 = e.1.saturating_add(1);
            }
        }
    }
    for x in &w.folk.live.persona.draft.told {
        if x.date.year() != year {
            continue;
        }
        if let Some(real) = shown(x.team, x.player) {
            let e = rec.entry(x.team).or_default();
            if real {
                e.0 = e.0.saturating_add(1);
            } else {
                e.1 = e.1.saturating_add(1);
            }
        }
    }
    let mut rows: Vec<(usize, i32, u8, u8)> = rec.into_iter().map(|(t, (r, s))| (t, year, r, s)).collect();
    rows.sort();
    w.folk.live.persona.draft.record.extend(rows);
    // Your planted smoke: a GM who heard it sees what you did.
    let Some(mine) = mine else { return };
    let planted: Vec<(usize, Option<usize>, u32)> = w
        .folk
        .live
        .persona
        .draft
        .planted
        .iter()
        .enumerate()
        .filter(|(_, p)| p.shown.is_none() && p.date.year() == year)
        .map(|(k, p)| (k, p.to, p.player))
        .collect();
    for (k, to, player) in planted {
        let s = shown(mine, player);
        w.folk.live.persona.draft.planted[k].shown = s;
        if s == Some(false)
            && let Some(t) = to
        {
            let key = super::super::makeup::key(super::gm(w, t));
            let now = super::super::makeup::trust(w, super::gm(w, t)) as f32;
            w.folk.mine.set_trust(key, (now - SMOKE_COST).clamp(0.0, 1.0));
        }
    }
}
