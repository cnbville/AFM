//! Another club's GM (6.7): a trade in words, weighed on his own chart
//! (`front_office::deals::evaluate`): players, picks and a conditional
//! pick. His counter is the picks he'd want added; a yes waits on your
//! confirm and trades through `deals::propose`, both caps and his yes
//! checked again.

use super::super::terms::{Counter, Give, Offer, PickIf, Weighed};
use super::super::wants::{Want, WantKind};
use crate::league::depth_slots;
use crate::world::World;
use crate::world::folk::scene::Scene;
use crate::world::folk::terms as season;
use crate::world::front_office::{self as fo, deals};
use crate::world::pro::office::Mode;

/// The club a player is on, by id.
fn club_of(w: &World, id: u32) -> Option<usize> {
    let i = crate::world::folk::person(w, id)?;
    w.people[i].pro.as_ref().and_then(|c| c.team)
}

/// One of a club's picks by draft and round: its own pick first, then
/// another club's, then a compensatory one, leaving out those taken.
fn pick(w: &World, t: usize, year: i32, round: u32, taken: &[deals::Pick]) -> Option<deals::Pick> {
    deals::picks_of(w, t)
        .into_iter()
        .filter(|p| p.year == year && p.round == round && !taken.contains(p))
        .min_by_key(|p| (p.comp, p.orig != t))
}

/// A pick's condition on the man it reads: a snap share is lane N2's
/// season term (`folk::terms`), judged on his season's snaps.
fn condition(w: &World, c: PickIf, pid: u32) -> Option<deals::Condition> {
    Some(match c {
        PickIf::Games(n) => deals::Condition::Games(n as u32),
        PickIf::Starts(n) => deals::Condition::Starts(n as u32),
        PickIf::ProBowl => deals::Condition::ProBowl,
        PickIf::Snaps(n) => {
            let pos = crate::world::folk::person(w, pid).map(|i| w.people[i].player.position)?;
            deals::Condition::Term(season::read(Some(pos), "snaps", Some(n.clamp(1, 100) as f32), false)?)
        }
    })
}

/// The trade an offer in words makes: your men and picks go, his come; a
/// condition sits on your first pick past round 1 and reads the first man
/// you get. None when it names nothing.
pub fn proposal(w: &World, club: usize, offer: &Offer) -> Option<deals::Proposal> {
    let me = fo::club(w)?;
    let year = offer.gives.iter().find_map(|g| if let Give::Draft(y) = g { Some(*y) } else { None });
    let year = year.unwrap_or_else(|| deals::first_draft(w));
    let mut give = deals::Side::default();
    let mut get = deals::Side::default();
    let mut cond = None;
    for g in &offer.gives {
        match *g {
            Give::Player(id) => match club_of(w, id) {
                Some(t) if t == me && !give.players.contains(&id) => give.players.push(id),
                Some(t) if t == club && !get.players.contains(&id) => get.players.push(id),
                _ => {}
            },
            Give::Pick(r) => {
                if let Some(p) = pick(w, me, year, r as u32, &give.picks) {
                    give.picks.push(p);
                }
            }
            Give::TheirPick(r) => {
                if let Some(p) = pick(w, club, year, r as u32, &get.picks) {
                    get.picks.push(p);
                }
            }
            Give::PickIf(c) => cond = Some(c),
            _ => {}
        }
    }
    if give.players.is_empty() && give.picks.is_empty() && get.players.is_empty() && get.picks.is_empty() {
        return None;
    }
    // The condition: a named one, else a season term said with the offer
    // ("if he gets 8 sacks").
    let conditional = match (give.picks.iter().find(|p| p.round >= 2), get.players.first()) {
        (Some(p), Some(&pid)) => cond
            .and_then(|c| condition(w, c, pid))
            .or_else(|| offer.terms.iter().find(|u| u.season).map(|u| deals::Condition::Term(*u)))
            .map(|c| (*p, c, pid)),
        _ => None,
    };
    Some(deals::Proposal { other: club, give, get, conditional })
}

/// His words without the "name, the club GM:" that the desk puts first.
fn quote(text: &str) -> String {
    text.find(": \"").map_or_else(|| text.to_string(), |k| text[k + 2..].to_string())
}

/// How he takes the trade on his chart, and his answer.
pub fn weigh(w: &World, club: usize, offer: &Offer) -> (Weighed, Option<deals::Answer>) {
    let Some(p) = proposal(w, club, offer) else { return (Weighed::No, None) };
    let Ok(a) = deals::evaluate(w, &p) else { return (Weighed::No, None) };
    let wd = if a.accept {
        Weighed::Yes
    } else if let Some(extra) = &a.counter {
        let mut gives: Vec<Give> = extra.iter().map(|pk| Give::Pick(pk.round as u8)).collect();
        if let Some(y) = extra.first().map(|pk| pk.year).filter(|&y| y != deals::first_draft(w)) {
            gives.push(Give::Draft(y));
        }
        Weighed::Counter(Counter {
            asks: vec![quote(&a.text)],
            gives,
            terms: Vec::new(),
            why: Some(Want { kind: WantKind::Club, weight: 1.0, met: false }),
        })
    } else if a.far {
        Weighed::Lowball
    } else {
        Weighed::No
    };
    (wd, Some(a))
}

/// His answer in words for the voice (a counter's own words go with the
/// counter), and your chart's read as your staff's note.
pub fn words(w: &World, club: usize, o: &Offer, notes: &mut Vec<String>) -> Vec<String> {
    let Some(p) = proposal(w, club, o) else { return vec!["\"Who are we talking about here?\"".into()] };
    match deals::evaluate(w, &p) {
        Err(e) => {
            notes.push(format!("Your staff: {e}"));
            vec!["\"I don't think that's a deal either of us can make right now.\"".into()]
        }
        Ok(a) => {
            notes.push(format!("On your chart you give {:.0} points and get {:.0}.", a.points.0, a.points.1));
            if a.counter.is_some() { Vec::new() } else { vec![quote(&a.text)] }
        }
    }
}

/// A trade in words for your screen: "You send ...; you get ...".
pub fn describe(w: &World, p: &deals::Proposal) -> String {
    let side = |s: &deals::Side| {
        let mut v: Vec<String> = s
            .players
            .iter()
            .filter_map(|&id| crate::world::folk::person(w, id).map(|i| w.people[i].name.clone()))
            .collect();
        v.extend(s.picks.iter().map(|pk| deals::pick_words(w, pk)));
        if v.is_empty() { "nothing".to_string() } else { v.join(", ") }
    };
    let mut s = format!("You send {}; you get {}", side(&p.give), side(&p.get));
    if let Some((pk, c, _)) = p.conditional {
        s += &format!(" ({} moves up a round {})", deals::pick_words(w, &pk), c.words());
    }
    s + "."
}

/// Makes the trade you confirmed, through the desk: his yes and both caps
/// are checked again.
pub fn make(w: &mut World, club: usize, offer: &Offer) -> Vec<String> {
    let Some(p) = proposal(w, club, offer) else { return vec!["There's no trade to make.".into()] };
    let said = describe(w, &p);
    match deals::propose(w, &p) {
        Ok(a) if a.accept => vec![format!("Trade made with the {}. {said}", w.nfl[club].name)],
        Ok(a) => vec![a.text],
        Err(e) => vec![e],
    }
}

/// Why a club would move one of its men, by the AI's own trade rule (the
/// seller's surplus in `trade_window`): none when it wouldn't.
pub fn why_move(w: &World, club: usize, i: usize) -> Vec<&'static str> {
    let Some(c) = w.people[i].pro.as_ref() else { return Vec::new() };
    if c.team != Some(club) {
        return Vec::new();
    }
    let pos = w.people[i].player.position;
    let age = w.people[i].player.age;
    let rank = w.clubs[club].depth.get(pos.index()).and_then(|d| d.iter().position(|&j| j == i)).unwrap_or(9);
    let mut v = Vec::new();
    if w.clubs[club].office.mode == Mode::Rebuild && age >= 26 {
        v.push("they're rebuilding and he's past the age they're building around");
    }
    if rank >= depth_slots(pos).starters {
        v.push("he isn't a starter for them");
    }
    if c.wants_out {
        v.push("he's asked out");
    }
    if deals::player_points(w, club, i) < 0.0 && age >= 27 {
        v.push("he costs them more than he's worth to them");
    }
    v
}

/// Who the ear's card may name on a GM call: his men he'd move, then your
/// men on your block.
pub fn named(w: &World, club: usize, me: Option<usize>) -> Vec<(usize, String)> {
    let mut v: Vec<(usize, String)> = Vec::new();
    for d in &w.clubs[club].depth {
        for &i in d {
            if v.len() < 5 && !v.iter().any(|x| x.0 == i) && !why_move(w, club, i).is_empty() {
                v.push((i, format!("on the {}", w.nfl[club].name)));
            }
        }
    }
    if let Some(me) = me {
        for &id in &w.front_office.trades.block {
            if let Some(i) = crate::world::folk::person(w, id)
                && w.people[i].pro.as_ref().and_then(|c| c.team) == Some(me)
                && v.len() < 8
            {
                v.push((i, "on your trading block".into()));
            }
        }
    }
    v
}

/// The call's first lines: who picked up, where his club stands, the man
/// it's about, and the window.
pub fn lede(w: &World, s: &mut Scene, club: usize, about: Option<usize>) {
    let gm = &w.clubs[club].office.gm;
    s.narrate(format!("{} of the {} picks up.", gm.name, w.nfl[club].name));
    if let Some(i) = about {
        s.narrate(format!("The call is about {}.", w.people[i].name));
    }
    if !deals::window_open(w) {
        s.narrate("Trades are closed until the new league year.");
    }
}
