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

    pub fn from_u8(k: u8) -> Level {
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
// Lane G7's day one (the build's picks, proposed, not confirmed).
/// A family whose older son plays for the program.
pub const BROTHER: f32 = 40.0;
/// A former player and his old school.
pub const ALUM: f32 = 30.0;
/// A former player and a man who coached him (C13: Known).
pub const OLD_PLAYER: f32 = 40.0;
/// Your coaching tree (people.md: Trusted, or Home on good terms).
pub const TREE: f32 = 55.0;
pub const TREE_HOME: f32 = 78.0;
/// A cousin's family starts a level below the family it knows (GR62).
pub const COUSIN_DROP: f32 = 25.0;

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
pub(crate) fn program_part(w: &World, p: usize, h: usize) -> f32 {
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
        (Who::Coach(c), Holder::Coach(k)) => coach_part(w, c, k),
        // A counselor knows the programs his kids went to and stayed in.
        (Who::Counselor(h), Holder::Program(p)) => w
            .programs
            .get(p as usize)
            .and_then(|q| q.hs_pipeline.get(&(h as usize)))
            .map_or(0.0, |x| (60.0 * x.0.clamp(0.0, 1.0)) as f32),
        // A family whose older son plays for the program knows it (lane G7).
        (Who::Family(k), Holder::Program(p)) => crate::world::folk::person(w, k)
            .filter(|&i| w.people[i].family.older_brother_at == Some(p as usize))
            .map_or(0.0, |_| BROTHER),
        // A former player knows his old school, and the men who coached him
        // (C13: Known; one coaching high school Trusted).
        (Who::Trainer(f) | Who::SevenOnSeven(f), Holder::Program(p)) => {
            w.formers.get(f).filter(|x| x.colleges.iter().any(|c| c.0 as u32 == p)).map_or(0.0, |_| ALUM)
        }
        (Who::Trainer(f) | Who::SevenOnSeven(f), Holder::Coach(k)) => {
            w.formers.get(f).filter(|x| x.coached_by.contains(&k)).map_or(0.0, |_| OLD_PLAYER)
        }
        _ => 0.0,
    }
}

/// Day one for a coach and one of your men: his own recruiting tie to the
/// coach's school, and (lane G7) a four-year college's man is your
/// network's tie to him (other college staffs get standing like everyone,
/// GR63). Your coaching tree and your old players are stored when your job
/// starts (`seed`).
fn coach_part(w: &World, c: u32, k: u32) -> f32 {
    let mut v: f32 = 0.0;
    let Some(man) = w.coaches.get(k as usize) else { return 0.0 };
    if let Some((h, Role::HeadCoach)) = people::coach_school(w, c as usize) {
        v = v.max(man.ties.get(&h).map_or(0.0, |&t| TIE_POINTS * t.clamp(0.0, 1.0) as f32));
    }
    let you = w.career.as_ref().map(|x| x.coach as u32);
    if Some(k) == you && people::kind(w, Who::Coach(c)) == people::Kind::College {
        v = v.max(100.0 * crate::world::folk::network::warmth(w, c as usize));
    }
    v
}

/// A holder's points with a person: his memory, or (lane G9) the AI's
/// worked tie on top of day one, or day one.
pub fn points(w: &World, who: Who, holder: Holder) -> f32 {
    w.ground.get(who, holder).map_or_else(|| base(w, who, holder), |m| m.points)
}

/// What a holder has with a person before your seat's memory: the AI's
/// worked tie (a man who joins your staff brings it), never below day one
/// (lane G9).
pub fn base(w: &World, who: Who, holder: Holder) -> f32 {
    let d = derived(w, who, holder);
    w.ground.ai.get(who, holder).map_or(d, |t| t.points.max(d))
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
pub(crate) fn gain(m: &mut Memory, by: f32, d: Date) {
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
pub(crate) fn entry(g: &mut Ground, w_derived: f32, who: Who, holder: Holder, d: Date) -> &mut Memory {
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
pub(crate) fn head_holder(w: &World, prog: usize) -> Option<Holder> {
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
        let one = VISIT * skill * mk.warmth * pull;
        let size = match kind {
            Contact::Visit if hc => one * name,
            Contact::Visit => one,
            Contact::Call | Contact::Campus if hc => one * name,
            Contact::Call | Contact::Campus => one * CALL_SHARE,
            Contact::Event(fit) => EVENT * fit * mk.warmth,
        } * share
            * scale;
        let start = base(w, who, holder);
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

/// Your college seat's program, when the seat keeps ground (a college
/// staff with a career, the ground on).
pub fn seat_program(w: &World) -> Option<usize> {
    if w.ground.off || w.career.is_none() {
        return None;
    }
    match readers::seat_staff(w) {
        Some(readers::Staff::Program(p)) => Some(p),
        _ => None,
    }
}

/// A cousin's family starts from the family it knows, a level lower
/// (GR62): its points less one level's width, as old acquaintances.
pub(crate) fn from_cousin(w: &mut World, prog: usize, fam: Who, cousin: Who, d: Date) {
    let hs: Vec<Holder> = holders(w, prog);
    for holder in hs {
        let Some(c) = w.ground.get(cousin, holder) else { continue };
        let pts = (c.points - COUSIN_DROP).max(0.0);
        // A family you broke your word with knows you for itself.
        if pts <= 0.0 || w.ground.get(fam, holder).is_some_and(|m| m.broken.is_some()) {
            continue;
        }
        let met = c.met;
        let m = w.ground.entry(fam, holder, || Memory::new(fam, holder, 0.0, d));
        m.points = m.points.max(pts);
        m.met = m.met.min(met);
        m.contacts = m.contacts.max(2);
        if m.points >= 25.0 && m.known_since.is_none() {
            m.known_since = Some(met.max(d.add(-365)));
        }
        m.roots.family = true;
        m.heard = Some("his cousins' family told them about you".into());
    }
}

/// Your staff's look with a recruit (`scouting::dept::add`) as contacts
/// with the people around him: an evaluation day is a visit with his head
/// coach (his coordinators sit in), a call reaches his coach and his
/// counselor, your camp is an event with his coach, a visit to campus is
/// the host's call with his coach. Only a college seat keeps ground.
pub(crate) fn on_look(w: &mut World, id: u32, code: u32, kind: Kind, d: Date) {
    let Some(prog) = seat_program(w) else { return };
    let c = match kind {
        Kind::Practice | Kind::Game => Contact::Visit,
        Kind::Call => Contact::Call,
        Kind::Camp => Contact::Event(1.0),
        Kind::Official | Kind::Unofficial | Kind::JuniorDay | Kind::HomeVisit => Contact::Campus,
        _ => return,
    };
    let Some(i) = crate::world::scouting::place(w, id) else { return };
    let (stage, college, hs) = (w.people[i].stage, w.people[i].college, w.people[i].hs);
    let man = (code >= readers::COACH).then(|| (code - readers::COACH) as usize).filter(|&c| c < w.coaches.len());
    // A junior college kid's coach is his head coach there (lane G7).
    if stage == Stage::College {
        let juco = college.and_then(|p| w.programs.get(p)).filter(|p| p.division == crate::world::Division::JUCO);
        if let Some(hc) = juco.and_then(|p| p.head_coach) {
            contact(w, prog, man, Who::Coach(hc as u32), c, 1.0, d);
        }
        return;
    }
    if !matches!(stage, Stage::HighSchool | Stage::PrepSchool) {
        return;
    }
    // His family hears from you the way he does: calls, home visits and his
    // visits to campus (the recruiting calendar's contact rules, GR62).
    if matches!(c, Contact::Call | Contact::Campus) {
        let fam = people::family_of(w, i);
        if w.ground.get(fam, Holder::Program(prog as u32)).is_none()
            && let Some(cousin) = people::cousin_of(w, i)
        {
            from_cousin(w, prog, fam, cousin, d);
        }
        contact(w, prog, man, fam, c, 1.0, d);
    }
    // A prep year's kid is at his prep school: its coach takes the visit.
    if stage == Stage::PrepSchool
        && let Some(prep) = people::prep_of(w, i)
    {
        if c != Contact::Event(1.0) {
            contact(w, prog, man, prep, c, 1.0, d);
        }
        if c == Contact::Visit {
            return;
        }
    }
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
    let Some(prog) = seat_program(w) else { return };
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
    // Your own word's credit with each college staff (you as the person,
    // `scouting/hs.rs`) stays wherever you go.
    let me = Who::Coach(you as u32);
    w.ground.memories.retain(|m| keep.contains(&m.holder) || m.holder == Holder::Coach(you as u32) || m.who == me);
    w.ground.told.clear();
    let ph = Holder::Program(prog as u32);
    for h in 0..w.high_schools.len() {
        for who in [people::head_coach_of(w, h), Some(Who::Counselor(h as u32))].into_iter().flatten() {
            let v = base(w, who, ph);
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
            let v = base(w, who, yh);
            if v >= 25.0 {
                entry(&mut w.ground, v, who, yh, d).did_well = true;
            }
        }
    }
    for (h, kind) in roots(w, you) {
        let Some(who) = people::head_coach_of(w, h) else { continue };
        let town = w.high_schools[h].town.clone();
        let start = base(w, who, yh);
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
    tree(w, you, d);
    let mut whos: Vec<Who> = w.ground.memories.iter().map(|m| m.who).collect();
    whos.dedup();
    for who in whos {
        let lv = level(w, prog, who) as u8;
        w.ground.set_told(who, lv);
    }
}

/// Your coaching tree and your old players (lane G7, people.md): your
/// mentor and the men who learned under you start Trusted, Home when he
/// is a loyal man; a former player of yours coaching high school starts
/// Trusted, one coaching elsewhere or training kids Known.
fn tree(w: &mut World, you: usize, d: Date) {
    let yh = Holder::Coach(you as u32);
    let mut v: Vec<(Who, f32, &str)> = Vec::new();
    for (c, co) in w.coaches.iter().enumerate() {
        if c == you || co.retired || co.job.is_none() {
            continue;
        }
        let tie = co.mentor == Some(you) || w.coaches[you].mentor == Some(c);
        if tie {
            let pts = if co.personality.loyalty >= 14.0 { TREE_HOME } else { TREE };
            let words =
                if co.mentor == Some(you) { "I learned this job from you." } else { "You were always one of mine." };
            v.push((Who::Coach(c as u32), pts, words));
        }
    }
    for f in w.formers.list.iter().filter(|f| f.coached_by.contains(&(you as u32))) {
        match (f.coach, f.life) {
            (Some(c), _) => {
                let hs = people::coach_school(w, c as usize).is_some();
                v.push((Who::Coach(c), if hs { TREE } else { OLD_PLAYER }, "You coached me. I don't forget that."));
            }
            (None, crate::world::former::Life::Trainer) => {
                v.push((Who::Trainer(f.id), OLD_PLAYER, "You coached me. I don't forget that."))
            }
            (None, crate::world::former::Life::Camp) => {
                v.push((Who::SevenOnSeven(f.id), OLD_PLAYER, "You coached me. I don't forget that."))
            }
            _ => {}
        }
    }
    for (who, pts, words) in v {
        let m = w.ground.entry(who, yh, || Memory::new(who, yh, 0.0, d));
        if m.points < pts {
            m.points = pts;
        }
        m.met = m.met.min(d.add(-6 * 365));
        m.contacts = m.contacts.max(6);
        let ks = d.add(-4 * 365);
        m.known_since = Some(m.known_since.map_or(ks, |k| k.min(ks)));
        if pts >= TREE_HOME {
            m.did_well = true;
        }
        if words.starts_with("You coached") {
            m.roots.old_player = true;
        } else {
            m.roots.tree = true;
        }
        if !m.moments.iter().any(|x| x.words == words) {
            m.keep(Moment { date: d, words: words.into(), good: true, big: false, holds_to: None });
        }
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
pub(crate) fn pay(
    w: &mut World,
    prog: usize,
    who: Who,
    by: f32,
    d: Date,
    kid: Option<(u32, &str, Fared)>,
    moment: Option<(String, bool, bool)>,
) {
    let mut hs = vec![Holder::Program(prog as u32)];
    hs.extend(head_holder(w, prog));
    pay_to(w, prog, &hs, who, by, d, kid, moment);
}

/// `pay` into these holders only. Word spreads from a coach through your
/// program's area (`word::spread`) only when your program is paid.
#[allow(clippy::too_many_arguments)]
fn pay_to(
    w: &mut World,
    prog: usize,
    hs: &[Holder],
    who: Who,
    by: f32,
    d: Date,
    kid: Option<(u32, &str, Fared)>,
    moment: Option<(String, bool, bool)>,
) {
    let before = level(w, prog, who);
    let mk = people::makeup(w, who);
    for &holder in hs {
        let start = base(w, who, holder);
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
    // People talk (GR62): it reaches a coach's close ties and his area.
    // (A family and a counselor are paid with him, so they don't pass it
    // on again.)
    if let Some((_, good, big)) = &moment
        && matches!(who, Who::Coach(_))
        && hs.contains(&Holder::Program(prog as u32))
    {
        let about = kid.map(|k| k.1.to_string());
        super::word::spread(w, prog, who, by, *good, *big, about, d);
    }
}

/// Points from a scene (lane G5): what you said and did with him in the
/// room, into the memories of your program and its head coach, inside the
/// gates, with a moment in his words when it's one he keeps. A loss here is
/// a talk gone badly, not your word broken.
pub(crate) fn grant(w: &mut World, prog: usize, who: Who, by: f32, words: Option<String>, d: Date) {
    if w.ground.off {
        return;
    }
    let mut hs = vec![Holder::Program(prog as u32)];
    hs.extend(head_holder(w, prog));
    for holder in hs {
        let start = base(w, who, holder);
        let m = entry(&mut w.ground, start, who, holder, d);
        gain(m, by, d);
        if let Some(words) = &words
            && !m.moments.iter().any(|x| &x.words == words)
        {
            m.keep(Moment { date: d, words: words.clone(), good: by >= 0.0, big: false, holds_to: None });
        }
    }
}

/// A son who graduated with you or was drafted from your program makes
/// his family Home (people.md, "Families").
fn family_home(w: &mut World, prog: usize, fam: Who, d: Date, id: u32, last: &str, how: Fared) {
    let before = level(w, prog, fam);
    let mut hs = vec![Holder::Program(prog as u32)];
    hs.extend(head_holder(w, prog));
    let words = match how {
        Fared::Drafted => "You got our son to the league.".to_string(),
        _ => "He came home with a degree. That was the promise.".to_string(),
    };
    for holder in hs {
        let start = base(w, fam, holder);
        let m = entry(&mut w.ground, start, fam, holder, d);
        m.kid(id, last, how);
        m.did_well = true;
        m.points = m.points.max(Level::Home.floor());
        m.contacts = m.contacts.max(2);
        m.known_since.get_or_insert(d);
        if !m.moments.iter().any(|x| x.words == words) {
            m.keep(Moment { date: d, words: words.clone(), good: true, big: true, holds_to: None });
        }
    }
    announce(w, prog, fam, before, d);
}

/// Promises judged and offers pulled reach the kid's high school coach
/// (GR24): a promise kept is your word held; one broken, or an offer
/// pulled, is a big moment he holds for years.
///
/// A promise made at your program now pays it, its head coach and you (an
/// assistant's own word too). One made at another program you coached
/// (`Promise::team`; judged when you left it, or a promise that you'd stay
/// broken when you started here) pays only your own word, which goes with
/// you (`seed` keeps it), not this program, which never gave it. One made
/// at a job outside college football is no word to a high school coach;
/// his family still remembers it, on your own word.
fn word(w: &mut World, prog: usize, d: Date) {
    use crate::world::folk::Team;
    let you = Holder::Coach(w.career.as_ref().map_or(0, |c| c.coach) as u32);
    // The program and its head coach; a promise you made here reaches your
    // own word too (an assistant's).
    let here = vec![Holder::Program(prog as u32)].into_iter().chain(head_holder(w, prog)).collect::<Vec<_>>();
    let mut yours = here.clone();
    if !yours.contains(&you) {
        yours.push(you);
    }
    /// One word owed: its key, the kid, kept or not, what it was, who
    /// holds it, whether his high school coach hears it, and the day it
    /// was judged (a word from a job away from college waits for your next
    /// college seat, and is dated when it happened).
    struct Owed {
        k: u64,
        id: u32,
        kept: bool,
        what: String,
        to: Vec<Holder>,
        coach: bool,
        on: Date,
    }
    let mut todo: Vec<Owed> = Vec::new();
    // A promise closed with nothing to judge (it lapsed, or closed when you
    // left the job) is no word held or broken.
    for p in w.folk.promises.iter().filter(|p| !p.quiet()) {
        if let (Some(kept), crate::world::folk::talks::PromiseTo::Player(id)) = (p.kept, p.to) {
            let (to, coach) = match p.team {
                None => (yours.clone(), true),
                Some(Team::College(c)) if c == prog => (yours.clone(), true),
                Some(Team::College(_)) => (vec![you], true),
                Some(_) => (vec![you], false),
            };
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
                todo.push(Owed { k, id, kept, what: what.into(), to, coach, on: p.due.min(d) });
            }
        }
    }
    for t in &w.front_office.college.board {
        if let Some(pd) = t.pulled {
            let k = key(&[0x9B11, t.id as u64, pd.0 as u64]);
            if w.ground.done.binary_search(&k).is_err() {
                todo.push(Owed {
                    k,
                    id: t.id,
                    kept: false,
                    what: "pulled".into(),
                    to: here.clone(),
                    coach: true,
                    on: pd.min(d),
                });
            }
        }
    }
    for Owed { k, id, kept, what, to: hs_to, coach, on: d } in todo {
        w.ground.once(k);
        let Some(i) = crate::world::scouting::place(w, id) else { continue };
        let hs = w.people[i].hs;
        let last = crate::world::names::surname_of(&w.people[i].name).to_string();
        // His family remembers too, for every brother (GR62): lie to them
        // and the whole family remembers.
        let fam = people::family_of(w, i);
        let to = hs_to.as_slice();
        match (kept, what.as_str()) {
            (true, _) => pay_to(
                w,
                prog,
                to,
                fam,
                WORD_HELD,
                d,
                None,
                Some(("You did right by our son, like you said.".into(), true, false)),
            ),
            (false, "pulled") => {
                let words = format!("You pulled his offer in {}.", MONTH[d.parts().1 as usize - 1]);
                pay_to(w, prog, to, fam, PROMISE_BROKEN, d, None, Some((words, false, true)));
            }
            (false, _) => {
                pay_to(w, prog, to, fam, PROMISE_BROKEN, d, None, Some(("You lied to us.".into(), false, true)))
            }
        }
        let Some(who) = people::head_coach_of(w, hs).filter(|_| coach) else { continue };
        match (kept, what.as_str()) {
            (true, _) => pay_to(
                w,
                prog,
                to,
                who,
                WORD_HELD,
                d,
                None,
                Some((format!("You did right by {last}, like you said."), true, false)),
            ),
            (false, "pulled") => {
                let words = format!("You pulled {last}'s offer in {}.", MONTH[d.parts().1 as usize - 1]);
                pay_to(w, prog, to, who, PROMISE_BROKEN, d, None, Some((words, false, true)));
            }
            (false, "play") => {
                let words = format!("You told me {last} would play.");
                pay_to(
                    w,
                    prog,
                    to,
                    who,
                    PROMISE_BROKEN,
                    d,
                    Some((id, &last, Fared::Buried)),
                    Some((words, false, true)),
                );
            }
            (false, _) => {
                let words = format!("You made {last} a promise and didn't keep it.");
                pay_to(w, prog, to, who, PROMISE_BROKEN, d, None, Some((words, false, true)));
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
        // A son who played for you and did well makes his family Home.
        if let Some(i) = crate::world::folk::person(w, id) {
            let fam = people::family_of(w, i);
            match how {
                Fared::Started => pay(
                    w,
                    prog,
                    fam,
                    by,
                    d,
                    Some((id, &last, how)),
                    Some(("Our boy started for you.".into(), true, false)),
                ),
                _ => family_home(w, prog, fam, d, id, &last, how),
            }
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
    // Lane G9: the rival staffs' own road and calls with his head coach.
    for (&k, t) in &w.ground.ai.ties.0 {
        let (coach, holder) = super::ai::key_parts(k);
        if let Holder::Program(p) = holder
            && p as usize != prog
            && t.last > d.add(-30)
            && t.last <= d
            && let Some((h, Role::HeadCoach)) = people::coach_school(w, coach as usize)
        {
            seen.push((h, p as usize, t.last.0));
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
    let me = w.career.as_ref().map(|c| Who::Coach(c.coach as u32));
    w.ground.memories.retain(|m| keep.contains(&m.holder) || Some(m.holder) == you || Some(m.who) == me);
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
        // A family keeps its memory for the younger brother (GR62).
        if m.last.days_to(d) > FADE_AFTER && !matches!(m.who, Who::Family(_)) {
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
        if lv == Level::Stranger || m.warned == lv as u8 + 1 || matches!(who, Who::Family(_)) {
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
pub(crate) fn announce(w: &mut World, prog: usize, who: Who, before: Level, d: Date) {
    let after = level(w, prog, who);
    let told = Level::from_u8(w.ground.told(who));
    if after <= before.max(told) {
        if after < told {
            w.ground.set_told(who, after as u8);
        }
        return;
    }
    w.ground.set_told(who, after as u8);
    w.ground.ups.push((d, who, after as u8));
    let line = up_words(w, who, after);
    let (_, holder) = staff(w, prog, who);
    if let Some(m) = w.ground.get_mut(who, holder) {
        m.keep(Moment { date: d, words: line.clone(), good: true, big: after == Level::Home, holds_to: None });
    }
    let subject = match after {
        Level::Known => "Call me any time",
        Level::Trusted => "Between us",
        _ => "You'll hear first",
    };
    post(w, who, d, Head::Email, subject.into(), vec!["Coach,".into(), line], if after == Level::Home { 2 } else { 1 });
}

/// A new level in his own words (GR26), by who he is and his nature: a
/// loyal man says it plainly, a self-serving one with his price on it.
fn up_words(w: &World, who: Who, lv: Level) -> String {
    use people::Kind as K;
    let loyal = people::makeup(w, who).loyal >= 0.5;
    let i = (lv as usize).clamp(1, 3) - 1;
    let pick = |x: [&str; 3]| x[i].to_string();
    match people::kind(w, who) {
        K::HsHead if loyal => pick([
            "You've been by enough. Call me any time and I'll give you my list.",
            "I'll tell you what I really think of my kids.",
            "From now on you hear about my guys first.",
        ]),
        K::HsHead => pick([
            "You keep showing up, so I'll give you my list. Call me any time.",
            "You've been straight with me, so I'll tell you what I really think of my kids. Look after my guys.",
            "From now on you hear about my guys first. Don't make me regret it.",
        ]),
        K::HsCoordinator | K::HsCoach => pick([
            "Coach says you're good people. Ask me about the weight room any time.",
            "I'll tell you who really works and who's hurting.",
            "Anything moves in our building, you'll know.",
        ]),
        K::Counselor => pick([
            "Send me what you need on any of our seniors and I'll get it to you.",
            "I'll tell you where his grades really stand, not what the transcript says.",
            "When one of ours is ready, you'll hear it from me first.",
        ]),
        K::Juco => pick([
            "Call me when you need a body. I'll tell you who's real.",
            "I'll tell you which of mine can handle the jump, and which can't.",
            "My best ones, you see them before anybody.",
        ]),
        K::Prep => pick([
            "Our guys are here for one year. Call me and I'll tell you where they stand.",
            "I'll tell you which ones have the grades now and which never will.",
            "When one of mine is ready, you're my first call.",
        ]),
        K::Trainer if loyal => pick([
            "Come by a session. I'll show you what my guys really run.",
            "I'll give you the real times, not the ones on the internet.",
            "Anybody new comes through my gym, you hear about him first.",
        ]),
        K::Trainer => pick([
            "Come by a session. My guys are the best in the state, you'll see.",
            "I'll give you the real times. Just remember who sent them.",
            "Anybody new comes through my gym, you hear about him first. Take care of my clients.",
        ]),
        K::SevenOnSeven => pick([
            "Come out when we're in town. You'll see my guys.",
            "I'll tell you who's rising on the circuit before the camps do.",
            "Somebody pops this spring, you'll hear it from me first.",
        ]),
        K::Family => pick([
            "Coach, thank you for calling. We're listening.",
            "We'll tell you what he really wants.",
            "Whatever comes next in this family, you'll hear it from us first.",
        ]),
        K::College => pick([
            "Call me when you're looking. I'll tell you who's leaving.",
            "I'll tell you about my leavers before they hit the portal.",
            "You get my first call on anybody I can't keep.",
        ]),
    }
}

/// A letter or memo to your inbox at the person's hour.
pub(crate) fn post(w: &mut World, who: Who, d: Date, head: Head, subject: String, body: Vec<String>, loud: u8) {
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

/// A staff memo about a person (lane G4: an oversold kid), at his hour.
pub(crate) fn post_memo(w: &mut World, who: Who, d: Date, subject: String, body: Vec<String>, loud: u8) {
    post(w, who, d, Head::Memo, subject, body, loud);
}

/// A person's own call or letter (lane G4: news, a bloom), at his hour.
pub(crate) fn post_call(w: &mut World, who: Who, d: Date, subject: String, body: Vec<String>, loud: u8) {
    post(w, who, d, Head::Email, subject, body, loud);
}

/// A bloom pays the person who told you (lane G4, GR67): points inside the
/// gates with your program and your head coach, and the moment in his words.
pub(crate) fn bloom(w: &mut World, prog: usize, who: Who, by: f32, words: String, big: bool, d: Date) {
    let before = level(w, prog, who);
    let mut targets = vec![Holder::Program(prog as u32)];
    if let Some(h) = head_holder(w, prog) {
        targets.push(h);
    }
    for holder in targets {
        let start = base(w, who, holder);
        let m = entry(&mut w.ground, start, who, holder, d);
        gain(m, by, d);
        if !m.moments.iter().any(|x| x.words == words) {
            m.keep(Moment { date: d, words: words.clone(), good: true, big, holds_to: None });
        }
    }
    announce(w, prog, who, before, d);
}

fn who_key(who: Who) -> u64 {
    match who {
        Who::Coach(c) => (1 << 40) | c as u64,
        Who::Trainer(f) => (2 << 40) | f as u64,
        Who::SevenOnSeven(f) => (3 << 40) | f as u64,
        Who::Counselor(h) => (4 << 40) | h as u64,
        Who::Family(k) => (5 << 40) | k as u64,
        Who::Prep(k) => (6 << 40) | k as u64,
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
        // Lane G7: the rest of the memory, in words.
        if m.roots.tree {
            parts.push("your coaching tree".into());
        }
        if m.roots.old_player {
            parts.push("you coached him".into());
        }
        if let Some(h) = &m.heard {
            parts.push(h.clone());
        }
        if let Some(f) = m.favor_words() {
            parts.push(f);
        }
        if let Some(x) = m.your_word_words() {
            parts.push(x);
        }
        if let Some(x) = m.his_word_words() {
            parts.push(x);
        }
        if let Some(g) = m.moments.iter().filter(|x| x.good && x.big).max_by_key(|x| x.date)
            && !matches!(m.roots, r if r.own_school)
        {
            parts.push(format!("he remembers \"{}\"", g.words));
        }
        if m.contacts > 0 && m.last.days_to(d) > FADE_AFTER {
            parts.push(format!("no visit since {}", MONTH[m.last.parts().1 as usize - 1]));
        }
    }
    let from_day_one = w.ground.get(who, holder).is_none_or(|m| m.roots.program || m.roots.tie);
    if from_day_one && pts >= 1.0 && w.ground.get(who, holder).is_none() && !matches!(who, Who::Coach(_)) {
        // Lane G7's people from the world's own facts.
        let words = match (who, holder) {
            (Who::Family(_), Holder::Program(p)) => format!("his brother plays for {}", w.program_name(p as usize)),
            (Who::Trainer(_) | Who::SevenOnSeven(_), Holder::Program(p)) => {
                format!("he played at {}", w.program_name(p as usize))
            }
            (Who::Trainer(_) | Who::SevenOnSeven(_), Holder::Coach(c)) => format!(
                "{} coached him",
                if Some(c) == you {
                    "you".to_string()
                } else {
                    w.coaches.get(c as usize).map_or("a coach".into(), |c| c.name.clone())
                }
            ),
            _ => String::new(),
        };
        if !words.is_empty() {
            parts.push(words);
        }
    } else if from_day_one && pts >= 1.0 {
        match holder {
            Holder::Coach(c) if Some(c) != you => parts.push(format!(
                "{}'s own tie to his school",
                w.coaches.get(c as usize).map_or("a coach", |c| c.name.as_str())
            )),
            Holder::Coach(_) if people::kind(w, who) == people::Kind::College => {
                parts.push("your own tie to him".into())
            }
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
/// coach's standing, weighted by the talent the school controls; any list
/// of schools (`area_level` for one of lane G1a's areas).
pub fn area_points(w: &World, prog: usize, schools: &[usize], talent: &[f32]) -> f32 {
    // A weighted mean of 0-100 points, summed in f64: in f32 the sum and the
    // divide round apart and an all-Home area read 100.0000076.
    let (mut sum, mut weight) = (0.0f64, 0.0f64);
    for &h in schools {
        let t = f64::from(talent.get(h).copied().unwrap_or(1.0));
        let p = people::head_coach_of(w, h).map_or(0.0, |who| staff(w, prog, who).0);
        sum += t * f64::from(p);
        weight += t;
    }
    if weight > 0.0 { (sum / weight) as f32 } else { 0.0 }
}

/// A program's standing in one of Ground's areas (GR27, lane G1a's
/// `areas`): its schools' head coaches weighted by talent.
pub fn area_level(w: &World, prog: usize, area: u16, talent: &[f32]) -> f32 {
    let schools: Vec<usize> = (0..w.high_schools.len()).filter(|&h| w.high_schools[h].ground_area == area).collect();
    area_points(w, prog, &schools, talent)
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
