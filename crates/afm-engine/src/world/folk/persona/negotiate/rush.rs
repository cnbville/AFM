//! A prospect left in the undrafted rush (6.11): you sell your roster at
//! his spot and the money, and he weighs your call the way the rush does.
//! `pro::scouts::rush` is played read-only with your priority list as it
//! would be with his row at your terms: the same paths, cash, ties, home
//! and luck, so a yes on the phone is the rush's own yes. It's binding
//! only once he's on your list and the rush runs.

use super::super::terms::{Counter, Give, Offer, Weighed};
use super::super::wants::{Want, WantKind};
use super::contract::amount;
use crate::world::World;
use crate::world::folk::claims::words::spot;
use crate::world::folk::scene::Scene;
use crate::world::front_office as fo;
use crate::world::pro::scouts::{self, Offer as RushOffer};
use crate::world::scouting::live::{LiveDraft, default_pfa};

/// Steps in the search for the bonus that lands him.
const SEARCH: usize = 9;

fn live(w: &World) -> Option<&LiveDraft> {
    w.scouting.live.as_ref()
}

/// Whether you can call him: the war room is on its last night or the
/// rush is open, and he's still on the board.
pub fn callable(w: &World, i: usize) -> Result<(), String> {
    let l = live(w).ok_or("The draft isn't live.")?;
    if !l.rush && l.night < 2 {
        return Err("Clubs call undrafted players from the last night of the draft.".into());
    }
    if !l.run.pool.iter().any(|x| x.0 == i) {
        return Err("He isn't left on the board.".into());
    }
    Ok(())
}

/// Your priority list as the rush reads it, with his row at these terms
/// (in its place, or first when he wasn't on it).
fn list_with(w: &World, l: &LiveDraft, id: u32, bonus: f64, gtd: f64) -> Vec<(u32, u32, u32)> {
    let mut pfa = w.scouting.board.pfa.clone();
    if pfa.is_empty() {
        pfa = default_pfa(w, l);
    }
    let row = (id, bonus.round().max(0.0) as u32, gtd.round().max(0.0) as u32);
    match pfa.iter().position(|x| x.0 == id) {
        Some(k) => pfa[k] = row,
        None => pfa.insert(0, row),
    }
    pfa
}

/// The pool your list spends before his row.
fn spent_before(w: &World, l: &LiveDraft, id: u32) -> f64 {
    let pfa = list_with(w, l, id, 0.0, 0.0);
    pfa.iter().take_while(|x| x.0 != id).map(|x| x.1 as f64).sum()
}

/// The most you can offer him: the pool your list leaves (sandbox mode's
/// money switch lifts it).
pub fn most(w: &World, i: usize) -> f64 {
    let Some(l) = live(w) else { return 0.0 };
    let pool = scouts::udfa_pool(l.run.year);
    if w.sandbox(crate::world::sandbox::Switch::Money) {
        return pool * 3.0;
    }
    (pool - spent_before(w, l, w.people[i].player.id)).max(0.0)
}

/// Where he signs if the rush ran now with your call at these terms: the
/// club, or none.
pub fn signs_with(w: &World, i: usize, bonus: f64, gtd: f64) -> Option<usize> {
    let l = live(w)?;
    let year = l.run.year;
    let team = l.team;
    let left: Vec<usize> = l.run.pool.iter().map(|x| x.0).collect();
    let limit = w.rules.nfl_offseason as usize;
    let mut room: Vec<usize> = (0..w.nfl.len())
        .map(|t| {
            if t < w.clubs.len() && crate::world::franchise::active_in(w, t, year) {
                let have =
                    w.clubs[t].roster.len() + l.run.picked.iter().filter(|x| x.2 == t && x.1 > l.run.committed).count();
                limit.saturating_sub(have)
            } else {
                0
            }
        })
        .collect();
    let at: std::collections::HashMap<u32, usize> = l.run.pool.iter().map(|x| (w.people[x.0].player.id, x.0)).collect();
    let pool = scouts::udfa_pool(year);
    let free = w.sandbox(crate::world::sandbox::Switch::Money);
    let mut spent = 0.0;
    let mut mine: Vec<(usize, RushOffer)> = Vec::new();
    for (id, b, g) in list_with(w, l, w.people[i].player.id, bonus, gtd) {
        let Some(&k) = at.get(&id) else { continue };
        let b = if free { b as f64 } else { (b as f64).min((pool - spent).max(0.0)) };
        spent += b;
        mine.push((k, RushOffer { team, bonus: b, guarantee: g as f64 }));
    }
    if fo::roster_lifted(w, team) {
        room[team] = room[team].max(mine.len());
    }
    let seat = (team, mine);
    let signed = scouts::rush(w, &l.run.boards, &left, &mut room, Some(&seat));
    signed.iter().find(|x| x.0 == i).map(|x| x.1.team)
}

/// The terms in an offer: the bonus (or money named), the guarantee.
fn terms(w: &World, i: usize, offer: &Offer) -> (f64, f64) {
    let id = w.people[i].player.id;
    let row = w.scouting.board.pfa.iter().find(|x| x.0 == id);
    let mut bonus = row.map_or(0.0, |x| x.1 as f64);
    let mut gtd = row.map_or(0.0, |x| x.2 as f64);
    for g in &offer.gives {
        match *g {
            Give::Bonus(x) | Give::Money(x) => bonus = amount(x),
            Give::Guaranteed(x) => gtd = amount(x),
            _ => {}
        }
    }
    (bonus, gtd)
}

/// The smallest bonus (to the nearest $1,000) that lands him with you at
/// this guarantee, if any you can pay does.
pub fn price(w: &World, i: usize, from: f64, gtd: f64) -> Option<f64> {
    let team = live(w)?.team;
    let hi = most(w, i);
    if signs_with(w, i, hi, gtd) != Some(team) {
        return None;
    }
    let (mut lo, mut hi) = (from.max(0.0), hi);
    for _ in 0..SEARCH {
        let mid = (lo + hi) / 2.0;
        if signs_with(w, i, mid, gtd) == Some(team) {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    Some((hi / 1000.0).ceil() * 1000.0)
}

/// How he takes your call, and where he goes if not to you.
pub fn weigh(w: &World, i: usize, offer: &Offer) -> (Weighed, Option<usize>) {
    if callable(w, i).is_err() {
        return (Weighed::No, None);
    }
    let Some(team) = live(w).map(|l| l.team) else { return (Weighed::No, None) };
    let (bonus, gtd) = terms(w, i, offer);
    let goes = signs_with(w, i, bonus, gtd);
    if goes == Some(team) {
        return (Weighed::Yes, goes);
    }
    match price(w, i, bonus, gtd) {
        Some(x) => (
            Weighed::Counter(Counter {
                asks: vec![format!("He'd sign for a {} bonus.", fo::dollars(x))],
                gives: vec![Give::Bonus(x), Give::Guaranteed(gtd)],
                terms: Vec::new(),
                why: Some(Want { kind: WantKind::Money, weight: 1.0, met: false }),
            }),
            goes,
        ),
        None => (Weighed::No, goes),
    }
}

/// His answer in words for the voice, and your staff's note.
pub fn words(w: &World, i: usize, o: &Offer, notes: &mut Vec<String>) -> Vec<String> {
    let (wd, goes) = weigh(w, i, o);
    let team = live(w).map(|l| l.team);
    match wd {
        Weighed::Yes => {
            notes.push("Put him on your list at those terms and he signs with you when the rush runs.".into());
            vec!["\"That works. We'd love to be there.\"".into()]
        }
        Weighed::Counter(_) => vec!["\"Other clubs are calling too.\"".into()],
        _ => match goes {
            Some(t) if Some(t) != team => {
                vec![format!("\"We're leaning to the {}. He sees a better shot at their roster.\"", w.nfl[t].name)]
            }
            _ => {
                notes.push("You have no room left on your offseason roster.".into());
                vec!["\"I don't see where he fits on your roster.\"".into()]
            }
        },
    }
}

/// His own ask as a real thing: the bonus that lands him.
pub fn asks_for(w: &World, i: usize, kind: WantKind) -> Vec<Give> {
    if !matches!(kind, WantKind::Money | WantKind::Fee | WantKind::Security) {
        return Vec::new();
    }
    let gtd = w.scouting.board.pfa.iter().find(|x| x.0 == w.people[i].player.id).map_or(0.0, |x| x.2 as f64);
    price(w, i, 0.0, gtd).map(|x| vec![Give::Bonus(x)]).unwrap_or_default()
}

/// Puts him on your priority list at the agreed terms (your scouts' list
/// is filled in first when you have none, so their other calls stand).
pub fn put_on_list(w: &mut World, i: usize, offer: &Offer) -> Vec<String> {
    let Some(l) = live(w) else { return vec!["The draft is over.".into()] };
    let id = w.people[i].player.id;
    let (bonus, gtd) = terms(w, i, offer);
    let list = list_with(w, l, id, bonus, gtd);
    w.scouting.board.pfa = list;
    vec![format!(
        "{} is on your priority list: a {} bonus, {} guaranteed.",
        w.people[i].name,
        fo::dollars(bonus),
        fo::dollars(gtd)
    )]
}

/// Your roster at his spot, best first (up to four).
pub fn at_his_spot(w: &World, i: usize) -> Vec<usize> {
    let Some(t) = live(w).map(|l| l.team) else { return Vec::new() };
    let pos = w.people[i].player.position;
    w.clubs.get(t).and_then(|c| c.depth.get(pos.index())).map_or_else(Vec::new, |d| d.iter().take(4).copied().collect())
}

/// The call's first lines: who's on the phone, your room at his spot and
/// the pool left.
pub fn lede(w: &World, s: &mut Scene, i: usize) {
    let name = w.people[i].name.clone();
    match super::agent_of(w, i) {
        Some(a) => {
            s.narrate(format!("You get {name} on the phone, his agent {} on the line too.", w.market.agents[a].name))
        }
        None => s.narrate(format!("You get {name} on the phone.")),
    }
    let pos = w.people[i].player.position;
    let n = live(w).and_then(|l| w.clubs.get(l.team)).and_then(|c| c.depth.get(pos.index())).map_or(0, |d| d.len());
    s.narrate(format!("You have {n} at {} on your roster.", spot(pos)));
    s.narrate(format!("Your bonus pool has {} left for him.", fo::dollars(most(w, i))));
}
