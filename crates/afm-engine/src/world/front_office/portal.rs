//! Portal recruiting for your program (Cain 2026-11-07: "there is no
//! reason why I shouldn't have the same options with a portal player that
//! I have with an HS player"): calls, a campus visit and a home visit with
//! their talks, a promise of a role, and the satisfaction bar ("when it's
//! met it's decided immediately"), on the portal's own clock. Every AI
//! staff calls the entrants it wants and hosts him when it is one of his
//! finalists, by the same code (`recruiting::suitors`, `finalist_visits`);
//! these are yours. Sizes are Claude's picks (proposed, not confirmed).

use super::college::{Plan, PortalTalk, VisitKind, VisitState};
use super::{index_of, note, program, tell};
use crate::calendar::{Date, recruiting_period};
use crate::player::PlayingTime;
use crate::world::background::Decider;
use crate::world::folk::dialogue::{self, AT_HOME, ON_CAMPUS, ON_PHONE};
use crate::world::recruiting as rec;
use crate::world::season::key_normal;
use crate::world::{Stage, World};

/// An official visit's pull, as a recruit's (0.004 for the contact, 0.02
/// for the visit; proposed, not confirmed).
pub(crate) const VISIT_PULL: f64 = 0.024;

/// How far a campus can surprise a transfer, in appeal points: half a
/// recruit's (`FEEL_SD` 0.07), since he has lived a college program
/// already (proposed, not confirmed).
const FEEL_SD: f64 = 0.035;

/// A middle transfer's satisfaction bar, in appeal points: how far his
/// best option must lead the next before he stops shopping (proposed, not
/// confirmed: about a sold official visit and a good talk together, or a
/// third more money than the next school's for a player who weighs money
/// in the middle).
const BAR: f64 = 0.06;

/// With nobody else after him, he weighs your offer against waiting for
/// one (proposed, not confirmed).
const ALONE: f64 = 0.08;

/// How a campus felt to him on his visit, in appeal points (his own draw
/// for each school, keyed by the day he entered).
pub(crate) fn feel(w: &World, i: usize, j: usize, entered: Date) -> f64 {
    let p = &w.people[i];
    let seed = w.options.seed as u64 ^ (entered.0 as u64) << 20 ^ 0x9F;
    FEEL_SD * key_normal(seed ^ j as u64, p.player.id as u64)
}

fn feel_words(f: f64) -> &'static str {
    match f / FEEL_SD {
        z if z > 1.0 => "He came away sold on the place.",
        z if z > 0.3 => "He liked what he saw.",
        z if z > -0.3 => "He was polite, hard to read.",
        z if z > -1.0 => "He left lukewarm.",
        _ => "It didn't click for him.",
    }
}

/// His own satisfaction bar (Cain: "I'm not saying that every player
/// should have the same or a set list"): higher with ego (he enjoys being
/// courted), ambition (he wants to see every door) and a handler shopping
/// him, lower with loyalty (once he likes a place, he's in) and love of
/// the game (he wants to play, not shop), and a draw of his own (spread
/// about a third either way). It falls to half by his decision day: the
/// longer he waits, the less he needs. Real entrants commit anywhere from
/// the day they enter to the window's last week; the fast ones have a
/// clear first choice. Proposed, not confirmed.
pub fn bar(w: &World, i: usize) -> f64 {
    let p = &w.people[i];
    let own = own_bar(w, i, BAR);
    match p.cal.portal {
        Some(e) => {
            let span = e.entered.days_to(e.decide_by).max(1) as f64;
            let left = w.today.days_to(e.decide_by).max(0) as f64;
            own * (0.5 + 0.5 * (left / span).min(1.0))
        }
        None => own,
    }
}

/// A player's own bar on `base`: his traits and his draw (`bar`).
pub(crate) fn own_bar(w: &World, i: usize, base: f64) -> f64 {
    let p = &w.people[i];
    let t = &p.player.personality.traits;
    let f = |x: u8| x as f64 - 10.0;
    let z = 0.04 * f(t.ego) + 0.03 * f(t.ambition)
        - 0.04 * f(t.loyalty)
        - 0.03 * (p.player.personality.love_of_game - 10.0)
        + if p.family.decider == Decider::Handler { 0.3 } else { 0.0 }
        + 0.25 * key_normal(w.options.seed as u64 ^ 0xBA2, p.player.id as u64);
    base * z.clamp(-1.2, 1.2).exp()
}

/// Your lead over his next option, in appeal points (None: you aren't
/// first), from the race.
pub fn lead(race: &[(usize, f64)], j: usize, alone: f64) -> Option<f64> {
    let first = race.first()?;
    if first.0 != j {
        return None;
    }
    Some(race.get(1).map_or(alone, |s| first.1 - s.1))
}

/// Your staff's read of where you stand with him against his bar: 0 far
/// off, 1 there (the screens' meter; the numbers stay in the engine).
pub fn meter(w: &World, i: usize) -> Option<f64> {
    let j = program(w)?;
    let race = rec::portal_race(w, i);
    let k = race.iter().position(|r| r.0 == j)?;
    let b = bar(w, i).max(1e-6);
    let gap = match lead(&race, j, ALONE) {
        Some(l) => l,
        None => race[k].1 - race[0].1,
    };
    Some(((gap + b) / (2.0 * b)).clamp(0.0, 1.0))
}

/// The race as your staff reads it: every program after him, best first.
pub fn race(w: &World, i: usize) -> Vec<(usize, f64)> {
    rec::portal_race(w, i)
}

fn target_mut(w: &mut World, id: u32) -> Option<&mut super::college::PortalTarget> {
    w.front_office.college.portal.iter_mut().find(|t| t.id == id && !t.settled)
}

fn target(w: &World, id: u32) -> Option<&super::college::PortalTarget> {
    w.front_office.college.portal.iter().find(|t| t.id == id && !t.settled)
}

/// He is shopping now: in the portal, his window open.
fn shopping(w: &World, i: usize) -> bool {
    let p = &w.people[i];
    p.stage == Stage::College && p.cal.portal.is_some_and(|e| !e.held && e.entered <= w.today)
}

fn first(w: &World, i: usize) -> String {
    w.people[i].name.split_whitespace().next().unwrap_or("He").to_string()
}

/// The place he'd have with you: his role and the money.
pub(crate) fn place(w: &World, i: usize, j: usize) -> (PlayingTime, f64) {
    rec::portal_place(w, i, j).map_or((PlayingTime::Backup, 0.0), |p| (p.0, p.1))
}

/// A talk's pull, kept: the latest at each place (a call, his home, your
/// campus), as a living room's is.
pub(crate) fn talked(w: &mut World, id: u32, at: f32, lean: f32, pitch: &str) {
    let Some(i) = index_of(w, id) else { return };
    if !shopping(w, i) {
        return;
    }
    let today = w.today;
    let Some(t) = target_mut(w, id) else { return };
    let at = at as u8;
    t.talks.retain(|x| x.at != at);
    t.talks.push(PortalTalk { date: today, at, lean, pitch: pitch.to_string() });
}

/// Your promise of a role is on his file (0 a starting job, 1 a real role).
pub(crate) fn promised(w: &World, id: u32) -> Option<(u8, Date)> {
    target(w, id).and_then(|t| t.promise)
}

/// You promise him a role: (its pull, how far he believes it, the role he'd
/// have with you anyway). Kept on his file; logged as your promise the day
/// he lands with you.
pub(crate) fn promise(w: &mut World, id: u32, kind: u8) -> Option<(f64, f64, PlayingTime)> {
    let i = index_of(w, id)?;
    let j = program(w)?;
    let (role, _, perf, bar) = rec::portal_place(w, i, j)?;
    let (pull, belief) = dialogue::promise_pull(w, i, kind, role, perf, bar);
    let today = w.today;
    let t = target_mut(w, id)?;
    t.promise = Some((kind, today));
    let name = w.people[i].name.clone();
    let what = if kind == 0 { "a starting job" } else { "a real role" };
    note(w, format!("Promised portal entrant {name} {what}."), Some(id));
    Some((pull, belief, role))
}

/// After a talk: he may have heard enough.
pub(crate) fn after_talk(w: &mut World, id: u32) -> Option<String> {
    check(w, id)
}

/// Your offer clears his bar: he tells you yes today, and is yours
/// tonight, when the portal's decisions are made. Some words for you.
pub(crate) fn check(w: &mut World, id: u32) -> Option<String> {
    let j = program(w)?;
    let i = index_of(w, id)?;
    let t = target(w, id)?;
    if t.offered.is_none() || t.committed.is_some() || !shopping(w, i) {
        return None;
    }
    let race = rec::portal_race(w, i);
    let l = lead(&race, j, ALONE)?;
    if l < bar(w, i) {
        return None;
    }
    let today = w.today;
    if let Some(t) = target_mut(w, id) {
        t.committed = Some(today);
    }
    if let Some(e) = w.people[i].cal.portal.as_mut() {
        e.decide_by = today;
    }
    close_visits(w, id, "He told you yes before the day came.");
    let name = w.people[i].name.clone();
    let pos = w.people[i].player.position;
    let text = format!(
        "{name} ({pos}) commits to you: your offer gave him what he needed, and he's done shopping. \
         He signs tonight."
    );
    note(w, format!("{name} committed from the portal."), Some(id));
    tell(w, today, text.clone(), false);
    Some(text)
}

/// Each morning at the desk: the day's visits, then anyone your offer now
/// satisfies (others' moves and his clock change the race too).
pub(crate) fn day(w: &mut World, j: usize, d: Date) {
    run_visits(w, j, d);
    let ids: Vec<u32> = w
        .front_office
        .college
        .portal
        .iter()
        .filter(|t| !t.settled && t.offered.is_some() && t.committed.is_none())
        .map(|t| t.id)
        .collect();
    for id in ids {
        check(w, id);
    }
}

// --- Calls -----------------------------------------------------------------------------------

/// You call an entrant (or your position coach for his spot does): open to
/// any staff from the hour he enters, in any period (calls aren't
/// in-person contact), one a day. Returns the call's scene.
pub fn call(w: &mut World, id: u32) -> Result<u32, String> {
    let j = program(w).ok_or("You have no college job.")?;
    let i = index_of(w, id).ok_or("No such player.")?;
    if w.people[i].college == Some(j) {
        return Err("He plays for you: sit down with him instead.".into());
    }
    if !shopping(w, i) {
        return Err(if w.people[i].cal.portal.is_some() {
            "He isn't in the portal yet: his team is still playing.".into()
        } else {
            "He isn't in the portal: a call now is tampering. Reach out quietly instead.".into()
        });
    }
    let today = w.today;
    super::college::portal_target(w, id)?;
    let t = target_mut(w, id).ok_or("No such player.")?;
    if t.called == Some(today) {
        return Err("You've called him today. Give him a day.".into());
    }
    if t.committed.is_some() {
        return Err("He's told you yes. He signs tonight.".into());
    }
    t.called = Some(today);
    Ok(dialogue::transfer_talk(w, id, j, today, ON_PHONE))
}

// --- Visits ----------------------------------------------------------------------------------

/// Why a visit can't be set on a day, or None: a campus visit (24 to 48
/// hours, the school pays) on any day of his window outside a dead period;
/// a home visit in a contact period. One of each while he shops, from
/// tomorrow to his decision day.
pub fn visit_rule(w: &World, i: usize, kind: VisitKind, d: Date) -> Option<String> {
    let p = &w.people[i];
    let Some(e) = p.cal.portal.filter(|e| !e.held && e.entered <= w.today) else {
        return Some("He isn't shopping in the portal.".into());
    };
    if !matches!(kind, VisitKind::Official | VisitKind::Home) {
        return Some("A transfer takes an official visit or a home visit.".into());
    }
    if d <= w.today {
        return Some("A visit is set from tomorrow.".into());
    }
    if d > e.decide_by {
        return Some(format!("He decides by {}.", e.decide_by.pretty()));
    }
    let period = recruiting_period(&w.dates, d);
    if !period.unofficial_visits() {
        return Some("A dead period: no visits.".into());
    }
    if kind == VisitKind::Home && period != crate::calendar::Period::Contact {
        return Some("A home visit is for a contact period.".into());
    }
    None
}

/// The days you may set a visit on, with the period's name.
pub fn visit_dates(w: &World, id: u32, kind: VisitKind) -> Vec<(Date, String)> {
    let Some(i) = index_of(w, id) else { return Vec::new() };
    let Some(e) = w.people[i].cal.portal else { return Vec::new() };
    let mut v = Vec::new();
    let mut d = w.today.add(1);
    while d <= e.decide_by && v.len() < 30 {
        if visit_rule(w, i, kind, d).is_none() {
            v.push((d, recruiting_period(&w.dates, d).name().to_string()));
        }
        d = d.add(1);
    }
    v
}

/// Whether he says yes: you need an offer out, and a campus visit goes to
/// his finalists (he takes about four); a home visit he grants anyone near
/// the front.
fn answer(w: &World, i: usize, j: usize, kind: VisitKind, d: Date) -> Result<String, String> {
    let name = first(w, i);
    let race = rec::portal_race(w, i);
    let Some(k) = race.iter().position(|r| r.0 == j) else {
        return Err(format!("{name} doesn't see a place for him with you."));
    };
    let draw = super::key(w, 0x7015 ^ kind as u64, w.people[i].player.id as u64 ^ (d.0 as u64) << 32);
    let ahead: Vec<String> = race.iter().take(k.min(2)).map(|r| w.program_name(r.0).to_string()).collect();
    match kind {
        VisitKind::Official if k < 4 || (k < 6 && draw < 0.3) => {
            Ok(format!("{name} accepted: he'll be on campus on {}.", d.pretty()))
        }
        VisitKind::Official => Err(format!("{name} said no: he's saving his trips for {}.", ahead.join(" and "))),
        _ if k < 6 || draw < 0.2 => Ok(format!("{name} will see you at home on {}.", d.pretty())),
        _ => Err(format!("{name} thanked you, but he's down to {}.", ahead.join(" and "))),
    }
}

/// Sets a visit, if the rules allow it and he says yes.
pub fn set_visit(w: &mut World, id: u32, kind: VisitKind, d: Date) -> Result<String, String> {
    let j = program(w).ok_or("You have no college job.")?;
    let i = index_of(w, id).ok_or("No such player.")?;
    if let Some(why) = visit_rule(w, i, kind, d) {
        return Err(why);
    }
    let t = target(w, id).ok_or("Offer him first: a transfer visits the schools that want him.")?;
    if t.offered.is_none() {
        return Err("Offer him first: a transfer visits the schools that want him.".into());
    }
    if t.committed.is_some() {
        return Err("He's told you yes already.".into());
    }
    if t.visits.iter().any(|v| v.kind == kind && matches!(v.state, VisitState::Set | VisitState::Done)) {
        return Err(format!("You have his {} already.", kind.name().to_lowercase()));
    }
    if t.visits.iter().any(|v| v.date == d && v.state == VisitState::Set) {
        return Err(format!("You see him on {} already.", d.pretty()));
    }
    let reply = answer(w, i, j, kind, d);
    let (state, text) = match &reply {
        Ok(t) => (VisitState::Set, t.clone()),
        Err(t) => (VisitState::Declined, t.clone()),
    };
    if let Some(t) = target_mut(w, id) {
        t.visits.retain(|v| !(v.kind == kind && v.state == VisitState::Declined));
        t.visits.push(Plan { id, kind, date: d, state, note: text.clone() });
    }
    if state == VisitState::Set {
        dialogue::transfer_talk(w, id, j, d, at_of(kind));
    }
    let name = w.people[i].name.clone();
    note(w, format!("{} for portal entrant {name} on {}: {text}", kind.name(), d.pretty()), Some(id));
    reply
}

pub fn cancel_visit(w: &mut World, id: u32, kind: VisitKind) -> Result<String, String> {
    let t = target_mut(w, id).ok_or("No such visit is set.")?;
    let v =
        t.visits.iter_mut().find(|v| v.kind == kind && v.state == VisitState::Set).ok_or("No such visit is set.")?;
    v.state = VisitState::Off;
    v.note = "You called it off.".into();
    let d = v.date;
    dialogue::transfer_visit_off(w, id, d, at_of(kind), "You called it off.");
    Ok(format!("{} called off.", kind.name()))
}

/// Where a visit's talk happens.
fn at_of(kind: VisitKind) -> f32 {
    if kind == VisitKind::Official { ON_CAMPUS } else { AT_HOME }
}

/// His visits still set are off (he decided, or told you yes), each with
/// its talk.
pub(crate) fn close_visits(w: &mut World, id: u32, why: &str) {
    let mut off = Vec::new();
    for t in w.front_office.college.portal.iter_mut().filter(|t| t.id == id) {
        for v in t.visits.iter_mut().filter(|v| v.state == VisitState::Set) {
            v.state = VisitState::Off;
            v.note = why.to_string();
            off.push((v.date, at_of(v.kind)));
        }
    }
    for (d, at) in off {
        dialogue::transfer_visit_off(w, id, d, at, why);
    }
}

/// The day's visits happen (the talk at their end is its own scene).
fn run_visits(w: &mut World, j: usize, d: Date) {
    let due: Vec<(u32, VisitKind)> = w
        .front_office
        .college
        .portal
        .iter()
        .filter(|t| !t.settled)
        .flat_map(|t| t.visits.iter().filter(|v| v.state == VisitState::Set && v.date == d).map(|v| (v.id, v.kind)))
        .collect();
    for (id, kind) in due {
        let Some(i) = index_of(w, id) else { continue };
        let off = if !shopping(w, i) {
            Some("He made his choice before the day came.")
        } else if !recruiting_period(&w.dates, d).unofficial_visits() {
            Some("A dead period closed it.")
        } else {
            None
        };
        let entered = w.people[i].cal.portal.map_or(d, |e| e.entered);
        let f = feel(w, i, j, entered);
        let p = &mut w.people[i];
        let (name, pos, age) = (p.name.clone(), p.player.position, p.row_age());
        let text = match off {
            Some(why) => why.to_string(),
            None if kind == VisitKind::Official => {
                p.note(format!(
                    "At {age} he took an official visit to {} from the portal.",
                    w.programs[j].name(w.options.names)
                ));
                format!("{name} ({pos}) spent his visit with you. {}", feel_words(f))
            }
            None => format!("You sat with {name} at home."),
        };
        if off.is_some() {
            dialogue::transfer_visit_off(w, id, d, at_of(kind), &text);
        } else {
            tell(w, d, text.clone(), false);
            if kind == VisitKind::Official {
                w.log.official_visits += 1;
            }
        }
        if let Some(t) = target_mut(w, id) {
            if off.is_none() && kind == VisitKind::Official {
                t.feel = Some(f as f32);
            }
            if let Some(v) = t.visits.iter_mut().find(|v| v.kind == kind && v.state == VisitState::Set) {
                v.state = if off.is_some() { VisitState::Off } else { VisitState::Done };
                v.note = text;
            }
        }
    }
}

/// What your work with him adds at his decision: the quiet word before
/// he entered, your talks (the latest at each place) and how your campus
/// felt to him with the visit's own pull.
pub(crate) fn effort(t: &super::college::PortalTarget) -> f64 {
    let talks: f64 = t.talks.iter().map(|x| x.lean as f64).sum();
    let visit = t.feel.map_or(0.0, |f| VISIT_PULL + f as f64);
    talks + visit
}

/// Whether your official visit with him happened.
pub(crate) fn visited(t: &super::college::PortalTarget) -> bool {
    t.visits.iter().any(|v| v.kind == VisitKind::Official && v.state == VisitState::Done)
}
