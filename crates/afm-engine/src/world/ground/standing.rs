//! Standing (part C, GR17-GR29, roots GR7): Stranger, Known, Trusted and
//! Home with each person, with a why in words. The sizes are the spec's
//! starting sizes (`docs/design/ground.md`, "Starting sizes"; proposed,
//! not confirmed, tuned in GR61's trial and in play).
//!
//! - Every program has ground from day one, read from the world's 30
//!   years (`derived`): its pipelines to the school, the area and the
//!   state, its footprint, and each coach's own recruiting ties (a tie to
//!   a school is a bond with its head coach today). Until lane G9 the AI's
//!   ground is read from those whenever it is asked, so the world's
//!   recruiting is unchanged.
//! - Your seat (a college staff) keeps memories (`Ground`): your program's,
//!   your own and your men's. Your staff's looks with recruits are its
//!   contacts today (`on_look`; lane G3 replaces how the days are spent):
//!   an evaluation day at his school is a visit, a call is a call, your
//!   June camp an event, a visit to campus the host's call. Standing rises
//!   inside the gates (GR28), fades without contact (GR24), warns before a
//!   level drops (GR25) and announces each new level in the person's own
//!   words at his hour (GR26).
//! - A staff's standing with a person is the higher of its program's and
//!   its best current man's own (GR29); an area's is its people together,
//!   weighted by the talent each school sends out (GR27).

use super::memory::{Fared, Memory, Moment};
use super::people::{self, Holder, Who};
use super::{Ground, key};
use crate::calendar::Date;
use crate::world::coach::{Employer, Role};
use crate::world::desk::{Head, Mail};
use crate::world::scouting::{Kind, readers};
use crate::world::{Stage, World};

/// The four levels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub enum Level {
    Stranger,
    Known,
    Trusted,
    Home,
}

impl Level {
    pub const ALL: [Level; 4] = [Level::Stranger, Level::Known, Level::Trusted, Level::Home];

    pub fn of(points: f32) -> Level {
        if points >= 75.0 {
            Level::Home
        } else if points >= 50.0 {
            Level::Trusted
        } else if points >= 25.0 {
            Level::Known
        } else {
            Level::Stranger
        }
    }

    /// The lowest points of the level.
    pub fn floor(self) -> f32 {
        [0.0, 25.0, 50.0, 75.0][self as usize]
    }

    pub fn name(self) -> &'static str {
        ["Stranger", "Known", "Trusted", "Home"][self as usize]
    }

    fn from_u8(k: u8) -> Level {
        Level::ALL[(k as usize).min(3)]
    }
}

// The starting sizes (spec, "Starting sizes"; proposed, not confirmed).
/// A school visit, before his people skill, the person's warmth and the pull.
pub const VISIT: f32 = 6.0;
/// An assistant's call is a third of a visit.
pub const CALL_SHARE: f32 = 1.0 / 3.0;
/// Your clinic, your camp, the convention: with each coach there.
pub const EVENT: f32 = 3.0;
pub const WORD_HELD: f32 = 5.0;
pub const WORD_BROKEN: f32 = -12.0;
/// A promise broken: an offer pulled, a kid buried after you said he'd play.
pub const PROMISE_BROKEN: f32 = -20.0;
pub const STARTED: f32 = 6.0;
pub const GRADUATED: f32 = 4.0;
pub const DRAFTED: f32 = 12.0;
/// Days without contact before standing fades.
pub const FADE_AFTER: i32 = 60;
/// A month's fade, and the more for each rival staff there more than you.
pub const FADE: f32 = 1.0;
pub const FADE_RIVAL: f32 = 0.5;

// The build's own picks (proposed, not confirmed).
/// The program's share of a contact one of its men made: the man keeps
/// the rest as his own tie, which goes with him (GR24, GR29).
pub const PROGRAM_SHARE: f32 = 1.0 / 3.0;
/// A coordinator sits in on your visit to his school at half its weight.
pub const SAT_IN: f32 = 0.5;
/// While a big bad moment holds, being there buys a quarter.
pub const HELD: f32 = 0.25;
/// Warnings in your inbox a month, the schools with the most talent first.
pub const WARN_MAX: usize = 5;
/// Day one: points for a coach's recruiting tie to a school (0-1).
pub const TIE_POINTS: f32 = 85.0;

/// A contact: what someone on your staff did with a person.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Contact {
    /// An evaluation day at his school (practice or the game).
    Visit,
    /// A call to him.
    Call,
    /// He came to your camp or clinic: the scheme fit (1.0-1.5).
    Event(f32),
    /// His kid's visit to your campus, hosted by the man who made it.
    Campus,
}

/// A program's day-one part with a school's people, 0-100: its pipelines
/// to the school (0.8), the area (0.45) and the state (0.25) and its
/// footprint in the state (0.2), put together as chances; Home only where
/// the school itself is a pipeline (a kid of his did well there).
fn program_part(w: &World, p: usize, h: usize) -> f32 {
    let (Some(prog), Some(s)) = (w.programs.get(p), w.high_schools.get(h)) else { return 0.0 };
    let school = prog.hs_pipeline.get(&h).map_or(0.0, |x| x.0);
    let area = prog.area_pipeline.get(&s.area).map_or(0.0, |x| x.0);
    let st = s.state.min(50);
    let parts = [0.8 * school, 0.45 * area, 0.25 * prog.pipeline[st], 0.2 * prog.footprint[st]];
    let v = 100.0 * (1.0 - parts.iter().map(|x| 1.0 - x.clamp(0.0, 1.0)).product::<f64>()) as f32;
    if school < 0.05 { v.min(74.0) } else { v }
}

/// Day one, read from the world's 30 years (and re-read whenever asked,
/// so it follows the world's pipelines and ties as they change).
pub fn derived(w: &World, who: Who, holder: Holder) -> f32 {
    match (who, holder) {
        (Who::Coach(c), Holder::Program(p)) => match people::coach_school(w, c as usize) {
            Some((h, Role::HeadCoach)) => program_part(w, p as usize, h),
            Some((h, _)) => SAT_IN * program_part(w, p as usize, h),
            None => 0.0,
        },
        (Who::Coach(c), Holder::Coach(k)) => match people::coach_school(w, c as usize) {
            Some((h, Role::HeadCoach)) => w
                .coaches
                .get(k as usize)
                .and_then(|k| k.ties.get(&h))
                .map_or(0.0, |&t| TIE_POINTS * t.clamp(0.0, 1.0) as f32),
            _ => 0.0,
        },
        // A counselor knows the programs his kids went to and stayed in.
        (Who::Counselor(h), Holder::Program(p)) => w
            .programs
            .get(p as usize)
            .and_then(|q| q.hs_pipeline.get(&(h as usize)))
            .map_or(0.0, |x| (60.0 * x.0.clamp(0.0, 1.0)) as f32),
        _ => 0.0,
    }
}

/// A holder's points with a person: his memory, or day one.
pub fn points(w: &World, who: Who, holder: Holder) -> f32 {
    w.ground.get(who, holder).map_or_else(|| derived(w, who, holder), |m| m.points)
}

/// A program and its current men: who holds its staff's ground.
pub fn holders(w: &World, prog: usize) -> Vec<Holder> {
    let mut v = vec![Holder::Program(prog as u32)];
    if let Some(p) = w.programs.get(prog) {
        for c in [p.head_coach, p.oc, p.dc].into_iter().chain(p.position_coaches.iter().copied()).flatten() {
            let h = Holder::Coach(c as u32);
            if !v.contains(&h) {
                v.push(h);
            }
        }
    }
    v
}

/// A staff's standing with a person (GR29): the higher of its program's
/// and its best current man's own, and who holds it.
pub fn staff(w: &World, prog: usize, who: Who) -> (f32, Holder) {
    holders(w, prog)
        .into_iter()
        .map(|h| (points(w, who, h), h))
        .fold((0.0, Holder::Program(prog as u32)), |a, b| if b.0 > a.0 { b } else { a })
}

pub fn level(w: &World, prog: usize, who: Who) -> Level {
    Level::of(staff(w, prog, who).0)
}

/// The highest points the gates allow on this day (GR28): Known after two
/// contacts; Trusted a year after Known with no broken word in that year;
/// Home three years after you met and a kid of his who did well with you,
/// or five years.
pub fn cap(m: &Memory, d: Date) -> f32 {
    if m.contacts < 2 {
        return 24.9;
    }
    let trusted = m.known_since.is_some_and(|k| {
        let from = m.broken.map_or(k, |b| b.max(k));
        from.days_to(d) >= 365
    });
    if !trusted {
        return 49.9;
    }
    let years = m.met.days_to(d) as f32 / 365.25;
    if (years >= 3.0 && m.did_well) || years >= 5.0 { 100.0 } else { 74.9 }
}

/// Adds points inside the gates (a loss is never gated) and notes Known.
fn gain(m: &mut Memory, by: f32, d: Date) {
    if by >= 0.0 {
        let top = cap(m, d).max(m.points);
        m.points = (m.points + by).min(top);
    } else {
        m.points += by;
    }
    m.points = m.points.clamp(0.0, 100.0);
    if m.points >= 25.0 && m.known_since.is_none() {
        m.known_since = Some(d);
    }
}

/// The memory, made from day one the first time.
fn entry(g: &mut Ground, w_derived: f32, who: Who, holder: Holder, d: Date) -> &mut Memory {
    g.entry(who, holder, || {
        let mut m = Memory::new(who, holder, w_derived, d);
        if w_derived > 0.0 {
            day_one(&mut m, d);
        }
        m
    })
}

/// A memory read from day one: they have known each other for years.
fn day_one(m: &mut Memory, d: Date) {
    let lv = Level::of(m.points);
    m.met = d.add(-5 * 365);
    m.last = d;
    m.contacts = m.contacts.max(match lv {
        Level::Stranger => 1,
        Level::Known => 3,
        _ => 6,
    });
    if lv >= Level::Known {
        m.known_since = Some(if lv >= Level::Trusted { d.add(-2 * 365) } else { d.add(-180) });
    }
    m.roots.program = matches!(m.holder, Holder::Program(_));
    m.roots.tie = matches!(m.holder, Holder::Coach(_));
}

/// The head coach of a program, as a holder.
fn head_holder(w: &World, prog: usize) -> Option<Holder> {
    w.programs.get(prog)?.head_coach.map(|c| Holder::Coach(c as u32))
}

/// One contact by your staff with a person: the man who made it builds
/// his own tie and the program a third (a contact with nobody's name on
/// it, the program's in full). `scale` sizes it down (a coordinator who
/// sat in).
pub fn contact(w: &mut World, prog: usize, man: Option<usize>, who: Who, kind: Contact, scale: f32, d: Date) {
    let before = level(w, prog, who);
    let mk = people::makeup(w, who);
    let prestige = w.programs.get(prog).map_or(0.5, |p| p.prestige) as f32;
    let pull = 0.9 + 0.2 * prestige;
    let hc = man.is_some() && w.programs.get(prog).and_then(|p| p.head_coach) == man;
    // Your head coach's name (GR53): about 2 at LSU, about 1 at Georgia State.
    let name = 0.8 + 1.3 * prestige * prestige;
    let mut targets: Vec<(Holder, f32)> = Vec::new();
    match man {
        Some(c) => {
            targets.push((Holder::Coach(c as u32), 1.0));
            targets.push((Holder::Program(prog as u32), PROGRAM_SHARE));
        }
        None => targets.push((Holder::Program(prog as u32), 1.0)),
    }
    let week = d.0.div_euclid(7);
    for (holder, share) in targets {
        let skill = people::skill(w, holder_man(holder, man));
        let base = VISIT * skill * mk.warmth * pull;
        let size = match kind {
            Contact::Visit if hc => base * name,
            Contact::Visit => base,
            Contact::Call | Contact::Campus if hc => base * name,
            Contact::Call | Contact::Campus => base * CALL_SHARE,
            Contact::Event(fit) => EVENT * fit * mk.warmth,
        } * share
            * scale;
        let start = derived(w, who, holder);
        let m = entry(&mut w.ground, start, who, holder, d);
        // One contact a day: three of his kids seen on one Thursday are one
        // visit with him.
        if m.recent.last() == Some(&d) {
            continue;
        }
        if m.week.0 != week {
            m.week = (week, 0);
        }
        let (y, _, _) = d.parts();
        if m.year.0 != y {
            m.year = (y, 0, 0);
        }
        let mut by = size;
        if !matches!(kind, Contact::Event(_)) {
            // A second visit the same week buys half of the first (GR54).
            by *= 0.5f32.powi(m.week.1.min(4) as i32);
            m.week.1 = m.week.1.saturating_add(1);
        }
        if m.holding(d).is_some() {
            by *= HELD;
        }
        m.contacts = m.contacts.saturating_add(1);
        m.last = d;
        m.recent.push(d);
        if m.recent.len() > 8 {
            m.recent.remove(0);
        }
        match kind {
            Contact::Visit => m.year.1 += 1,
            Contact::Call | Contact::Campus => m.year.2 += 1,
            Contact::Event(_) => {}
        }
        m.warned = 0;
        gain(m, by, d);
    }
    announce(w, prog, who, before, d);
}

/// The skill that counts for a holder: the man's own (the program's share
/// of his contact is his work too).
fn holder_man(holder: Holder, man: Option<usize>) -> Holder {
    match (holder, man) {
        (Holder::Program(_), Some(c)) => Holder::Coach(c as u32),
        _ => holder,
    }
}

/// Your staff's look with a recruit (`scouting::dept::add`) as contacts
/// with the people around him: an evaluation day is a visit with his head
/// coach (his coordinators sit in), a call reaches his coach and his
/// counselor, your camp is an event with his coach, a visit to campus is
/// the host's call with his coach. Only a college seat keeps ground.
pub(crate) fn on_look(w: &mut World, id: u32, code: u32, kind: Kind, d: Date) {
    if w.ground.off || w.career.is_none() {
        return;
    }
    let Some(readers::Staff::Program(prog)) = readers::seat_staff(w) else { return };
    let c = match kind {
        Kind::Practice | Kind::Game => Contact::Visit,
        Kind::Call => Contact::Call,
        Kind::Camp => Contact::Event(1.0),
        Kind::Official | Kind::Unofficial | Kind::JuniorDay | Kind::HomeVisit => Contact::Campus,
        _ => return,
    };
    let Some(i) = crate::world::scouting::place(w, id) else { return };
    let q = &w.people[i];
    if !matches!(q.stage, Stage::HighSchool | Stage::PrepSchool) {
        return;
    }
    let hs = q.hs;
    let man = (code >= readers::COACH).then(|| (code - readers::COACH) as usize).filter(|&c| c < w.coaches.len());
    let Some(h) = w.high_schools.get(hs) else { return };
    let (head, oc, dc) = (h.coach, h.oc, h.dc);
    if let Some(hc) = head {
        contact(w, prog, man, Who::Coach(hc as u32), c, 1.0, d);
    }
    match c {
        Contact::Visit => {
            for x in [oc, dc].into_iter().flatten() {
                contact(w, prog, man, Who::Coach(x as u32), c, SAT_IN, d);
            }
        }
        Contact::Call => contact(w, prog, man, Who::Counselor(hs as u32), c, 1.0, d),
        _ => {}
    }
}

/// Your seat's ground each day: day one on a new job, promises and pulled
/// offers as they happen, his kids' seasons on their dates, and on the
/// first of each month the fade and the warnings.
pub(crate) fn day(w: &mut World, d: Date) {
    if w.ground.off || w.career.is_none() {
        return;
    }
    let Some(readers::Staff::Program(prog)) = readers::seat_staff(w) else { return };
    let you = w.career.as_ref().map_or(0, |c| c.coach);
    if w.ground.seeded != Some((you as u32, prog as u32)) {
        seed(w, prog, you, d);
    }
    word(w, prog, d);
    kids(w, prog, d);
    if d.parts().2 == 1 {
        month(w, prog, d);
    }
}

/// Day one for your seat: your program's ground from the world's 30 years,
/// your own ties, and your roots (GR7): your own high school's coach
/// Trusted, your hometown's coaches and your old schools' coaches Known.
/// Only people at Known or better are kept; the rest stay day one until
/// someone deals with them.
fn seed(w: &mut World, prog: usize, you: usize, d: Date) {
    w.ground.seeded = Some((you as u32, prog as u32));
    // The program's ties stay with it and your own go with you: drop
    // whatever another program held.
    let keep = holders(w, prog);
    w.ground.memories.retain(|m| keep.contains(&m.holder) || m.holder == Holder::Coach(you as u32));
    w.ground.told.clear();
    let ph = Holder::Program(prog as u32);
    for h in 0..w.high_schools.len() {
        for who in [people::head_coach_of(w, h), Some(Who::Counselor(h as u32))].into_iter().flatten() {
            let v = derived(w, who, ph);
            if v >= 25.0 {
                let did = w.programs[prog].hs_pipeline.get(&h).is_some_and(|x| x.0 >= 0.05);
                let m = entry(&mut w.ground, v, who, ph, d);
                m.did_well |= did;
            }
        }
    }
    let yh = Holder::Coach(you as u32);
    let ties: Vec<usize> = w.coaches[you].ties.keys().copied().collect();
    for h in ties {
        if let Some(who) = people::head_coach_of(w, h) {
            let v = derived(w, who, yh);
            if v >= 25.0 {
                entry(&mut w.ground, v, who, yh, d).did_well = true;
            }
        }
    }
    for (h, kind) in roots(w, you) {
        let Some(who) = people::head_coach_of(w, h) else { continue };
        let town = w.high_schools[h].town.clone();
        let start = derived(w, who, yh);
        let m = entry(&mut w.ground, start, who, yh, d);
        let (pts, words, since) = match kind {
            Root::OwnSchool => {
                m.roots.own_school = true;
                (55.0, "You're one of ours.".to_string(), 10)
            }
            Root::OldJob => {
                m.roots.old_job = true;
                (35.0, "You coached here. You know how it is.".to_string(), 6)
            }
            Root::Hometown => {
                m.roots.hometown = true;
                (30.0, format!("You're a {town} guy."), 10)
            }
        };
        if m.points < pts {
            m.points = pts;
        }
        m.met = m.met.min(d.add(-since * 365));
        m.contacts = m.contacts.max(4);
        let ks = if pts >= 50.0 { d.add(-5 * 365) } else { d.add(-365) };
        m.known_since = Some(m.known_since.map_or(ks, |k| k.min(ks)));
        if !m.moments.iter().any(|x| x.words == words) {
            m.keep(Moment { date: d, words, good: true, big: kind == Root::OwnSchool, holds_to: None });
        }
    }
    let mut whos: Vec<Who> = w.ground.memories.iter().map(|m| m.who).collect();
    whos.dedup();
    for who in whos {
        let lv = level(w, prog, who) as u8;
        w.ground.set_told(who, lv);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Root {
    OwnSchool,
    Hometown,
    OldJob,
}

/// Your roots (GR7): the high school you played for, the others in your
/// hometown, and the high schools you coached at.
fn roots(w: &World, you: usize) -> Vec<(usize, Root)> {
    let mut out: Vec<(usize, Root)> = Vec::new();
    let Some(c) = w.career.as_ref() else { return out };
    let own = c.past.as_ref().and_then(|p| p.high_school).map(|h| h.0);
    if let Some(h) = own.filter(|&h| h < w.high_schools.len()) {
        out.push((h, Root::OwnSchool));
    }
    if let Some((state, town)) = hometown(w) {
        for (h, s) in w.high_schools.iter().enumerate() {
            if s.state == state && s.town.eq_ignore_ascii_case(&town) && Some(h) != own {
                out.push((h, Root::Hometown));
            }
        }
    }
    let mut jobs: Vec<usize> = c
        .past
        .iter()
        .flat_map(|p| p.stops.iter())
        .filter_map(|s| match s.0 {
            Employer::HighSchool(h) => Some(h),
            _ => None,
        })
        .collect();
    jobs.extend(w.coaches[you].history.iter().filter_map(|s| match s.employer {
        Employer::HighSchool(h) => Some(h),
        _ => None,
    }));
    for h in jobs {
        if h < w.high_schools.len() && !out.iter().any(|x| x.0 == h) {
            out.push((h, Root::OldJob));
        }
    }
    out
}

/// Your hometown: your high school's town, or the town on your profile in
/// your home state.
pub fn hometown(w: &World) -> Option<(usize, String)> {
    let c = w.career.as_ref()?;
    if let Some(h) = c.past.as_ref().and_then(|p| p.high_school).and_then(|h| w.high_schools.get(h.0)) {
        return Some((h.state, h.town.clone()));
    }
    let t = c.you.profile.hometown.split(',').next().unwrap_or("").trim();
    (!t.is_empty()).then(|| (c.you.home_state, t.to_string()))
}

/// A kid from your hometown knows your name (GR7).
pub fn knows_your_name(w: &World, person: usize) -> bool {
    let (Some((state, town)), Some(q)) = (hometown(w), w.people.get(person)) else { return false };
    w.high_schools.get(q.hs).is_some_and(|h| h.state == state && h.town.eq_ignore_ascii_case(&town))
}

/// Pays one thing into the memories of your program and its head coach
/// with a person, with a moment in his words.
#[allow(clippy::too_many_arguments)]
fn pay(
    w: &mut World,
    prog: usize,
    who: Who,
    by: f32,
    d: Date,
    kid: Option<(u32, &str, Fared)>,
    moment: Option<(String, bool, bool)>,
) {
    let before = level(w, prog, who);
    let mk = people::makeup(w, who);
    let mut hs = vec![Holder::Program(prog as u32)];
    hs.extend(head_holder(w, prog));
    for holder in hs {
        let start = derived(w, who, holder);
        let m = entry(&mut w.ground, start, who, holder, d);
        if let Some((id, name, how)) = kid {
            m.kid(id, name, how);
            if matches!(how, Fared::Started | Fared::Graduated | Fared::Drafted) {
                m.did_well = true;
            }
        }
        if by < 0.0 {
            m.broken = Some(d);
        }
        gain(m, by, d);
        if let Some((words, good, big)) = &moment {
            let holds_to = (*big && !*good).then(|| d.add((mk.grudge * 365.25) as i32));
            m.keep(Moment { date: d, words: words.clone(), good: *good, big: *big, holds_to });
        }
    }
    announce(w, prog, who, before, d);
}

/// Promises judged and offers pulled reach the kid's high school coach
/// (GR24): a promise kept is your word held; one broken, or an offer
/// pulled, is a big moment he holds for years.
fn word(w: &mut World, prog: usize, d: Date) {
    let mut todo: Vec<(u64, u32, bool, String)> = Vec::new();
    for p in &w.folk.promises {
        if let (Some(kept), crate::world::folk::talks::PromiseTo::Player(id)) = (p.kept, p.to) {
            let k = key(&[0x9A0B, id as u64, p.made.0 as u64, p.due.0 as u64]);
            if w.ground.done.binary_search(&k).is_err() {
                let what = if matches!(
                    p.kind,
                    crate::world::folk::talks::PromiseKind::Start | crate::world::folk::talks::PromiseKind::Snaps
                ) {
                    "play"
                } else {
                    "promise"
                };
                todo.push((k, id, kept, what.into()));
            }
        }
    }
    for t in &w.front_office.college.board {
        if let Some(pd) = t.pulled {
            let k = key(&[0x9B11, t.id as u64, pd.0 as u64]);
            if w.ground.done.binary_search(&k).is_err() {
                todo.push((k, t.id, false, "pulled".into()));
            }
        }
    }
    for (k, id, kept, what) in todo {
        w.ground.once(k);
        let Some(i) = crate::world::scouting::place(w, id) else { continue };
        let q = &w.people[i];
        let Some(who) = people::head_coach_of(w, q.hs) else { continue };
        let last = crate::world::names::surname_of(&q.name).to_string();
        match (kept, what.as_str()) {
            (true, _) => pay(
                w,
                prog,
                who,
                WORD_HELD,
                d,
                None,
                Some((format!("You did right by {last}, like you said."), true, false)),
            ),
            (false, "pulled") => {
                let words = format!("You pulled {last}'s offer in {}.", MONTH[d.parts().1 as usize - 1]);
                pay(w, prog, who, PROMISE_BROKEN, d, None, Some((words, false, true)));
            }
            (false, "play") => {
                let words = format!("You told me {last} would play.");
                pay(w, prog, who, PROMISE_BROKEN, d, Some((id, &last, Fared::Buried)), Some((words, false, true)));
            }
            (false, _) => {
                let words = format!("You made {last} a promise and didn't keep it.");
                pay(w, prog, who, PROMISE_BROKEN, d, None, Some((words, false, true)));
            }
        }
    }
}

const MONTH: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// How his kids fared with you (GR23), on their dates: who started last
/// season (February 1), who was drafted and who graduated (May 20).
fn kids(w: &mut World, prog: usize, d: Date) {
    let (y, m, day) = d.parts();
    let mut todo: Vec<(usize, u32, String, Fared)> = Vec::new();
    if (m, day) == (2, 1) {
        for q in w.people.iter().filter(|q| q.college == Some(prog) && q.stage == Stage::College) {
            if q.last_role == Some(crate::player::PlayingTime::Starter) {
                todo.push((q.hs, q.player.id, crate::world::names::surname_of(&q.name).into(), Fared::Started));
            }
        }
    }
    if (m, day) == (5, 20) {
        for q in w.people.iter().filter(|q| q.college == Some(prog) && q.stage == Stage::College) {
            if q.cal.graduated || q.cal.may_graduate {
                todo.push((q.hs, q.player.id, crate::world::names::surname_of(&q.name).into(), Fared::Graduated));
            }
        }
        for a in w.alumni.iter().rev().take_while(|a| a.year >= y - 1) {
            if a.year == y && a.pick.is_some() && a.colleges.last() == Some(&prog) {
                let id =
                    a.player.as_ref().map_or_else(|| key(&[a.class_of as u64, a.high_school as u64]) as u32, |p| p.id);
                todo.push((a.high_school, id, crate::world::names::surname_of(&a.name).into(), Fared::Drafted));
            }
        }
    }
    for (hs, id, last, how) in todo {
        if !w.ground.once(key(&[0x4D1C, id as u64, how as u64])) {
            continue;
        }
        let (by, moment) = match how {
            Fared::Started => (STARTED, (format!("My {last} started for you."), true, false)),
            Fared::Graduated => (GRADUATED, (format!("{last} got his degree with you."), true, false)),
            Fared::Drafted => (DRAFTED, (format!("You got my {last} drafted."), true, true)),
            Fared::Buried => continue,
        };
        if let Some(who) = people::head_coach_of(w, hs) {
            pay(w, prog, who, by, d, Some((id, &last, how)), Some(moment));
        }
        // A counselor wants a kid placed where he'll graduate.
        if how == Fared::Graduated {
            pay(w, prog, Who::Counselor(hs as u32), by, d, Some((id, &last, how)), None);
        }
    }
}

/// Rival staffs with each school's kids in the last 30 days: (school,
/// program, days), from the kids' dated contacts.
fn rivals(w: &World, prog: usize, d: Date) -> std::collections::HashMap<usize, Vec<(usize, u16)>> {
    let mut out: std::collections::HashMap<usize, Vec<(usize, u16)>> = std::collections::HashMap::new();
    let mut seen: Vec<(usize, usize, i32)> = Vec::new();
    for q in w.people.iter().filter(|q| matches!(q.stage, Stage::HighSchool | Stage::PrepSchool)) {
        for v in &q.cal.visits {
            if v.0 != prog && v.1 > d.add(-30) && v.1 <= d {
                seen.push((q.hs, v.0, v.1.0));
            }
        }
    }
    seen.sort_unstable();
    seen.dedup();
    for (h, p, _) in seen {
        let e = out.entry(h).or_default();
        match e.iter_mut().find(|x| x.0 == p) {
            Some(x) => x.1 += 1,
            None => e.push((p, 1)),
        }
    }
    out
}

/// The first of each month: standing fades without contact (GR24), and
/// you hear before a level drops (GR25).
fn month(w: &mut World, prog: usize, d: Date) {
    // A man who left your staff took his own ties with him.
    let keep = holders(w, prog);
    let you = w.career.as_ref().map(|c| Holder::Coach(c.coach as u32));
    w.ground.memories.retain(|m| keep.contains(&m.holder) || Some(m.holder) == you);
    let riv = rivals(w, prog, d);
    let at: Vec<Option<usize>> = w.ground.memories.iter().map(|m| people::school(w, m.who)).collect();
    let more = |g: &Ground, i: usize| -> f32 {
        let m = &g.memories[i];
        let mine = m.month_contacts(d) as u16;
        at[i].and_then(|h| riv.get(&h)).map_or(0, |v| v.iter().filter(|x| x.1 > mine).count()) as f32
    };
    for i in 0..w.ground.memories.len() {
        let r = more(&w.ground, i);
        let m = &mut w.ground.memories[i];
        if m.last.days_to(d) > FADE_AFTER {
            m.points = (m.points - FADE - FADE_RIVAL * r).max(0.0);
        }
    }
    // Levels after the fade, and the warnings for the month ahead.
    let mut whos: Vec<Who> = w.ground.memories.iter().map(|m| m.who).collect();
    whos.dedup();
    let talent = talents(w);
    let mut warn: Vec<(f32, Who, Holder, String)> = Vec::new();
    for who in whos {
        let (pts, holder) = staff(w, prog, who);
        let lv = Level::of(pts);
        if (lv as u8) < w.ground.told(who) {
            w.ground.set_told(who, lv as u8);
        }
        let Ok(i) = w.ground.memories.binary_search_by(|m| (m.who, m.holder).cmp(&(who, holder))) else { continue };
        let m = &w.ground.memories[i];
        if lv == Level::Stranger || m.warned == lv as u8 + 1 {
            continue;
        }
        let r = more(&w.ground, i);
        let rate = if m.last.days_to(d) + 30 > FADE_AFTER { FADE + FADE_RIVAL * r } else { 0.0 };
        if rate > 0.0 && pts - rate < lv.floor() {
            let t = people::school(w, who).map_or(1.0, |h| talent.get(h).copied().unwrap_or(1.0));
            let text = slipping(w, who, m, at[i].and_then(|h| riv.get(&h)), lv);
            warn.push((t, who, holder, text));
        }
    }
    warn.sort_by(|a, b| b.0.total_cmp(&a.0));
    for (_, who, holder, text) in warn.into_iter().take(WARN_MAX) {
        let lv = level(w, prog, who);
        if let Some(m) = w.ground.get_mut(who, holder) {
            m.warned = lv as u8 + 1;
        }
        let place = people::school(w, who).map_or_else(|| people::name(w, who), |h| w.high_schools[h].town.clone());
        let subject = format!("{place} is slipping");
        post(w, who, d, Head::Memo, subject, vec![text], 1);
    }
}

/// "Columbus is slipping: no visit since March, Georgia was there twice.
/// A visit this month holds it."
fn slipping(w: &World, who: Who, m: &Memory, riv: Option<&Vec<(usize, u16)>>, lv: Level) -> String {
    let place = people::school(w, who).map_or_else(|| people::name(w, who), |h| w.high_schools[h].town.clone());
    let mut why = vec![format!("no visit since {}", MONTH[m.last.parts().1 as usize - 1])];
    let mine = m.month_contacts(w.today) as u16;
    if let Some(&(p, n)) = riv.and_then(|v| v.iter().filter(|x| x.1 > mine).max_by_key(|x| x.1)) {
        why.push(format!("{} was there {}", w.program_name(p), times(n)));
    }
    let to = Level::ALL[(lv as usize).saturating_sub(1)];
    format!(
        "{place} is slipping: {}. {} ({}) drops to {} within the month. A visit this month holds it.",
        why.join(", "),
        people::name(w, who),
        people::role(w, who),
        to.name()
    )
}

fn times(n: u16) -> String {
    match n {
        1 => "once".into(),
        2 => "twice".into(),
        n => format!("{} times", words(n as usize)),
    }
}

fn words(n: usize) -> String {
    const W: [&str; 11] = ["no", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten"];
    W.get(n).map_or_else(|| n.to_string(), |s| s.to_string())
}

/// Moving up is a moment (GR26): a new level comes in his own words, at
/// his hour, and he keeps it.
fn announce(w: &mut World, prog: usize, who: Who, before: Level, d: Date) {
    let after = level(w, prog, who);
    let told = Level::from_u8(w.ground.told(who));
    if after <= before.max(told) {
        if after < told {
            w.ground.set_told(who, after as u8);
        }
        return;
    }
    w.ground.set_told(who, after as u8);
    let line = up_words(who, after);
    let (_, holder) = staff(w, prog, who);
    if let Some(m) = w.ground.get_mut(who, holder) {
        m.keep(Moment { date: d, words: line.to_string(), good: true, big: after == Level::Home, holds_to: None });
    }
    let subject = match after {
        Level::Known => "Call me any time",
        Level::Trusted => "Between us",
        _ => "You'll hear first",
    };
    post(
        w,
        who,
        d,
        Head::Email,
        subject.into(),
        vec!["Coach,".into(), line.into()],
        if after == Level::Home { 2 } else { 1 },
    );
}

fn up_words(who: Who, lv: Level) -> &'static str {
    match (who, lv) {
        (Who::Counselor(_), Level::Known) => "Send me what you need on any of our seniors and I'll get it to you.",
        (Who::Counselor(_), Level::Trusted) => {
            "I'll tell you where his grades really stand, not what the transcript says."
        }
        (Who::Counselor(_), _) => "When one of ours is ready, you'll hear it from me first.",
        (_, Level::Known) => "You've been by enough. Call me any time and I'll give you my list.",
        (_, Level::Trusted) => "I'll tell you what I really think of my kids.",
        (_, _) => "From now on you hear about my guys first.",
    }
}

/// A letter or memo to your inbox at the person's hour.
fn post(w: &mut World, who: Who, d: Date, head: Head, subject: String, body: Vec<String>, loud: u8) {
    let mk = people::makeup(w, who);
    let from = people::name(w, who);
    let role = people::role(w, who);
    let school = people::school(w, who);
    let org = school.map_or_else(String::new, |h| w.school_name(h).to_string());
    let you = crate::world::folk::scene::your_name(w);
    let (from, role, org, org_line, sign) = match head {
        Head::Memo => (
            "Recruiting office".to_string(),
            "Your staff".to_string(),
            String::new(),
            "Recruiting".to_string(),
            "Recruiting office".to_string(),
        ),
        _ => (from.clone(), role, org, "Football office".to_string(), from),
    };
    let text: Vec<u64> = subject.bytes().map(u64::from).collect();
    let id = key(&[0x6A0D, d.0 as u64, who_key(who), key(&text)]);
    let Some(c) = w.career.as_mut() else { return };
    if c.desk.has(id) {
        return;
    }
    c.desk.mails.push(Mail {
        id,
        date: d,
        minute: mk.hour,
        head,
        org,
        org_line,
        from,
        role,
        to: format!("{you}, Head Coach"),
        subject,
        body,
        table: Vec::new(),
        attach: Vec::new(),
        sign,
        keys: (1 << 2) | (1 << 9),
        about: None,
        answers: Vec::new(),
        answered: None,
        answer_by: None,
        read: false,
        saved: false,
        loud,
        ids: Vec::new(),
        org_team: school.map(|h| 1_000_000 + h as u32),
        why: Vec::new(),
        count: 0,
        leads: 0,
    });
    c.desk.mails.sort_by_key(|m| (m.date, m.minute));
}

fn who_key(who: Who) -> u64 {
    match who {
        Who::Coach(c) => (1 << 40) | c as u64,
        Who::Trainer(f) => (2 << 40) | f as u64,
        Who::SevenOnSeven(f) => (3 << 40) | f as u64,
        Who::Counselor(h) => (4 << 40) | h as u64,
    }
}

/// The why in words (GR18): "Trusted: four visits this year, his Reyes
/// started for you".
pub fn why(w: &World, prog: usize, who: Who) -> String {
    let (pts, holder) = staff(w, prog, who);
    let lv = Level::of(pts);
    let mut parts: Vec<String> = Vec::new();
    let d = w.today;
    let you = w.career.as_ref().map(|c| c.coach as u32);
    if let Some(m) = w.ground.get(who, holder) {
        if let Some(h) = m.holding(d) {
            parts.push(format!("he hasn't forgotten \"{}\"", h.words));
        }
        let (y, v, c) = m.year;
        if y == d.parts().0 {
            if v > 0 {
                parts.push(format!("{} {} this year", words(v as usize), if v == 1 { "visit" } else { "visits" }));
            }
            if c > 0 {
                parts.push(format!("{} {} this year", words(c as usize), if c == 1 { "call" } else { "calls" }));
            }
        }
        for (_, name, how) in m.kids.iter().rev().take(2) {
            parts.push(format!("his {name} {}", how.words()));
        }
        if m.roots.own_school {
            parts.push("you played for his school".into());
        }
        if m.roots.hometown {
            parts.push("you're from his town".into());
        }
        if m.roots.old_job {
            parts.push("you coached at his school".into());
        }
        if m.contacts > 0 && m.last.days_to(d) > FADE_AFTER {
            parts.push(format!("no visit since {}", MONTH[m.last.parts().1 as usize - 1]));
        }
    }
    let from_day_one = w.ground.get(who, holder).is_none_or(|m| m.roots.program || m.roots.tie);
    if from_day_one && pts >= 1.0 {
        match holder {
            Holder::Coach(c) if Some(c) != you => parts.push(format!(
                "{}'s own tie to his school",
                w.coaches.get(c as usize).map_or("a coach", |c| c.name.as_str())
            )),
            Holder::Coach(_) => parts.push("your own tie to his school".into()),
            Holder::Program(p) => {
                let school = people::school(w, who).unwrap_or(0);
                let prog = &w.programs[p as usize];
                if prog.hs_pipeline.get(&school).is_some_and(|x| x.0 >= 0.05) {
                    parts.push(format!("his kids have done well at {}", w.program_name(p as usize)));
                } else {
                    parts.push(format!("{}'s name in his area", w.program_name(p as usize)));
                }
            }
        }
    }
    if parts.is_empty() {
        parts.push("no dealings yet".into());
    }
    format!("{}: {}", lv.name(), parts.join(", "))
}

/// Star weights for the talent a school sends out (proposed, not
/// confirmed): no stars 0.2, one 0.3, two 1, three 3, four 8, five 20.
const STAR: [f32; 6] = [0.2, 0.3, 1.0, 3.0, 8.0, 20.0];

/// The talent each high school controls (GR27): its recruits by stars
/// over the last four classes, plus one so every school counts a little.
pub fn talents(w: &World) -> Vec<f32> {
    let mut t = vec![1.0f32; w.high_schools.len()];
    let first = w.year - 3;
    for q in &w.people {
        if q.class_of >= first && q.class_of <= w.year && q.hs < t.len() {
            t[q.hs] += STAR[(q.stars as usize).min(5)];
        }
    }
    for a in w.alumni.iter().rev().take_while(|a| a.class_of >= first - 4) {
        if a.class_of >= first && a.class_of <= w.year && a.high_school < t.len() {
            t[a.high_school] += STAR[(a.stars as usize).min(5)];
        }
    }
    t
}

/// A program's standing over a set of schools (GR27): each school's head
/// coach's standing, weighted by the talent the school controls. Lane G1a's
/// areas give the sets; until then a caller passes any list of schools.
pub fn area_points(w: &World, prog: usize, schools: &[usize], talent: &[f32]) -> f32 {
    let (mut sum, mut weight) = (0.0f32, 0.0f32);
    for &h in schools {
        let t = talent.get(h).copied().unwrap_or(1.0);
        let p = people::head_coach_of(w, h).map_or(0.0, |who| staff(w, prog, who).0);
        sum += t * p;
        weight += t;
    }
    if weight > 0.0 { sum / weight } else { 0.0 }
}

/// The program that leads a set of schools, with its points: what the
/// map's owners' colors read (GR8). Every program counts, so it is a
/// whole-world pass: call it for an area, not per frame.
pub fn leader(w: &World, schools: &[usize], talent: &[f32]) -> Option<(usize, f32)> {
    (0..w.programs.len())
        .map(|p| (p, area_points(w, p, schools, talent)))
        .filter(|x| x.1 > 0.0)
        .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0)))
}
