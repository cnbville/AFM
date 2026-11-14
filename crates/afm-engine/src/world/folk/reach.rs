//! The recruit's other talks (talk facts lane L6; `afm/talk-facts/proposal.md`
//! section 6): your calls and messages to a high school recruit on each
//! era's contact rules (item 9), his family's call when something changes
//! that touches what they weigh (item 28), and your case at the end of his
//! official visit (item 29). Each is the recruit conversation of lane L5
//! (`pitch`) in its own setting, and adds to the one case per school
//! (`case`): more scenes never raise the ceiling.
//!
//! - **A call** is said to him, three exchanges at half a sit-down; **a
//!   message** is one exchange at a quarter: you name one thing (his game,
//!   your win, one point of yours) and he answers. The NCAA's rules of the
//!   day decide whether you may (`calendar::contact`); the game's own pace
//!   is one call and one message to a recruit a day. Each counts as a
//!   contact for your staff's read of his interest (`Desk.reach`).
//! - **His family calls you** (whoever decides for him) when an event on
//!   his fact sheet touches a group they weigh at `FAMILY_CARE` or more:
//!   his position coach or your head coach gone, a commitment, signee or
//!   transfer at his spot, your job in the news, another offer from a school
//!   ahead of you. One or two exchanges, their question first. No change,
//!   no call.
//! - **At the end of his official visit** you sit down with him and whoever
//!   decides, in your office: a sit-down like the living room.
//!
//! AI staffs run the same code (rule 8): each week a staff's position coach
//! calls the recruits it offered when the rules allow it, and every head
//! coach sits down with a recruit at the end of his official visit, each a
//! case in one step (`case::one_step`). Every size is Claude's (proposed,
//! not confirmed).

use rayon::prelude::*;

use super::case::{self, CALL_EXCHANGES, Case, PHONE, SIT_DOWN_EXCHANGES};
use super::claims::{self, Ear, Family, Group, Kind as Point};
use super::scene::{self, Kind, Scene, Who};
use crate::calendar::contact::{self, CallLimit, Medium};
use crate::calendar::{Date, Weekday};
use crate::world::front_office::college as fc;
use crate::world::season::key_uniform;
use crate::world::{Stage, World};

/// His family calls about a change only in a group they weigh this much
/// (1.0 is a want of average weight). Proposed, not confirmed.
pub const FAMILY_CARE: f32 = 1.0;
/// At most one call from his family in this many days. Proposed, not
/// confirmed.
pub const FAMILY_GAP: i32 = 21;
/// How often in a week an AI staff's position coach has a real talk with a
/// recruit it offered, when the rules allow a call. Proposed, not
/// confirmed: staffs call their targets every week, and only the best
/// point in each group counts, so this sets how soon the case fills, not
/// how big it gets.
pub const AI_CALL_WEEK: f64 = 0.5;
/// The weekday AI staffs' calls are counted on.
pub const AI_CALL_DAY: Weekday = Weekday::Wed;

/// What you did: a call, a message, or his family's call to you.
pub const CALL: u8 = 0;
pub const MESSAGE: u8 = 1;
pub const FAMILY: u8 = 2;

/// One of your calls or messages to a recruit, or his family's to you.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Reach {
    pub id: u32,
    pub on: Date,
    pub kind: u8,
}

/// Your calls and messages kept: a year and a half covers every rule's
/// window.
const KEEP_DAYS: i32 = 550;

fn you(w: &World) -> Result<usize, String> {
    crate::world::front_office::program(w).ok_or_else(|| "You have no college job.".to_string())
}

fn yours(w: &World, id: u32, kind: u8) -> Vec<Date> {
    w.front_office.college.reach.iter().filter(|r| r.id == id && r.kind == kind).map(|r| r.on).collect()
}

/// Your calls and messages to him so far (what your staff's read of his
/// interest counts as contact).
pub fn contacts(w: &World, id: u32) -> usize {
    w.front_office.college.reach.iter().filter(|r| r.id == id && r.kind != FAMILY).count()
}

/// Whether a recruit can be reached at all: a recruit of yours to win, not
/// signed.
fn reachable(w: &World, i: usize) -> Result<(), String> {
    let p = &w.people[i];
    fc::recruitable(w, i)?;
    if p.cal.signed.is_some() {
        return Err("He has signed: there is nothing left to talk about.".into());
    }
    if p.cal.portal.is_some() {
        return Err("He's in the portal: call him from his portal file.".into());
    }
    Ok(())
}

fn in_high_school(w: &World, i: usize) -> bool {
    w.people[i].stage == Stage::HighSchool
}

/// Whether you may call him today: Ok with how often the rule of the day
/// lets you, Err with why not.
pub fn call_rule(w: &World, id: u32) -> Result<CallLimit, String> {
    you(w)?;
    let i = super::person(w, id).ok_or("No such player.")?;
    reachable(w, i)?;
    let calls = yours(w, id, CALL);
    if calls.contains(&w.today) {
        return Err("You've called him today. Give him a day.".into());
    }
    let p = &w.people[i];
    contact::call_rule(&w.dates, w.today, p.class_of, in_high_school(w, i), &calls)
}

/// Whether you may message him today: Ok with what a message is in the era.
pub fn message_rule(w: &World, id: u32) -> Result<Medium, String> {
    you(w)?;
    let i = super::person(w, id).ok_or("No such player.")?;
    reachable(w, i)?;
    if yours(w, id, MESSAGE).contains(&w.today) {
        return Err("You've written to him today.".into());
    }
    let p = &w.people[i];
    contact::message_rule(w.today, p.class_of, in_high_school(w, i))
}

fn log(w: &mut World, id: u32, kind: u8) {
    let today = w.today;
    let r = &mut w.front_office.college.reach;
    r.retain(|x| x.on.days_to(today) <= KEEP_DAYS);
    r.push(Reach { id, on: today, kind });
}

fn first_of(n: &str) -> &str {
    n.split_whitespace().next().unwrap_or(n)
}

/// You call a recruit: the call is a scene, opened on its page.
pub fn call(w: &mut World, id: u32) -> Result<u32, String> {
    call_rule(w, id)?;
    let j = you(w)?;
    let i = super::person(w, id).ok_or("No such player.")?;
    log(w, id, CALL);
    let name = w.people[i].name.clone();
    let mut s = Scene::new(Kind::RecruitCall, w.today, format!("A call to {name}"));
    s.place = format!("On the phone with {}", first_of(&name));
    s.a = id as u64;
    s.b = j as u64;
    s.who = vec![Who::Player(id)];
    Ok(scene::add(w, s))
}

/// You message a recruit: one exchange, a scene opened on its page.
pub fn message(w: &mut World, id: u32) -> Result<u32, String> {
    let medium = message_rule(w, id)?;
    let j = you(w)?;
    let i = super::person(w, id).ok_or("No such player.")?;
    log(w, id, MESSAGE);
    let name = w.people[i].name.clone();
    let noun = medium.noun();
    let mut s = Scene::new(Kind::RecruitMessage, w.today, format!("A {noun} to {name}"));
    s.place = format!("A {noun} to {}", first_of(&name));
    s.a = id as u64;
    s.b = j as u64;
    s.who = vec![Who::Player(id)];
    Ok(scene::add(w, s))
}

/// Your sit-down at the end of his official visit to you: a scene that
/// waits for you that day.
pub(crate) fn official_talk(w: &mut World, id: u32, j: usize, d: Date) {
    let Some(i) = super::person(w, id) else { return };
    let name = w.people[i].name.clone();
    let mut s = Scene::new(Kind::OfficialTalk, d, format!("Official visit: {name}"));
    s.place = format!("Your office, the end of {}'s official visit", first_of(&name));
    s.a = id as u64;
    s.b = j as u64;
    s.stops = true;
    s.who = vec![Who::Player(id), Who::Group(super::dialogue::decider_words(&w.people[i]).to_string())];
    scene::add(w, s);
}

/// The group an event on his sheet touches for his family, if it is one
/// they call about: the man who'd coach him or the head coach gone, a
/// player at his spot, your job in the news, another offer.
fn touches(k: Point, care: &[f32; 7]) -> Option<Group> {
    match k {
        Point::CoachLeft | Point::YourJob => Some(Group::People),
        Point::CommitAtSpot | Point::SigneeAtSpot | Point::TransferIn => Some(Group::PlayingTime),
        // Another offer: they ask about what they weigh most.
        Point::NewOffer => Group::ALL.into_iter().max_by(|a, b| care[*a as usize].total_cmp(&care[*b as usize])),
        _ => None,
    }
}

/// The event his family would call about today, with the group it
/// touches: the freshest on his sheet they haven't talked through with you.
pub(crate) fn family_reason(w: &World, i: usize, j: usize) -> Option<(Point, Group)> {
    let p = &w.people[i];
    let care = claims::group_cares(p, Ear::Decider);
    let sheet = claims::recruit(w, i, j, Ear::Decider);
    let today = w.today;
    let race = fc::race_at(w, i);
    let mine = race.iter().position(|r| r.0 == j);
    sheet
        .points
        .iter()
        .filter(|pt| pt.known && pt.kind.family() == Family::Event)
        .filter(|pt| !w.folk.talked.remembers(today, p.player.id, pt))
        .filter(|pt| pt.kind != Point::CoachLeft || pt.get("coach").is_some())
        .filter(|pt| {
            // Another offer matters from a school ahead of you on his list.
            pt.kind != Point::NewOffer
                || pt.get("school").is_some_and(|name| {
                    race.iter().position(|r| w.program_name(r.0) == name).is_some_and(|k| mine.is_none_or(|m| k < m))
                })
        })
        .filter_map(|pt| touches(pt.kind, &care).filter(|g| care[*g as usize] >= FAMILY_CARE).map(|g| (pt, g)))
        .max_by(|a, b| a.0.mind.total_cmp(&b.0.mind).then(b.0.kind.cmp(&a.0.kind)))
        .map(|(pt, g)| (pt.kind, g))
}

/// Each day at your desk: the promises of recruits who signed with you
/// (`pledge::log_signed`), and the families who call (a seventh of your
/// board each day, so each recruit is looked at once a week).
pub(crate) fn day(w: &mut World, j: usize, d: Date) {
    super::pledge::log_signed(w, j);
    let ids: Vec<u32> = w
        .front_office
        .college
        .board
        .iter()
        .filter(|t| t.offered.is_some() && t.pulled.is_none())
        .map(|t| t.id)
        .filter(|id| (*id as i32 + d.0).rem_euclid(7) == 0)
        .collect();
    for id in ids {
        let Some(i) = super::person(w, id) else { continue };
        let p = &w.people[i];
        if !matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) || p.cal.signed.is_some() {
            continue;
        }
        let gap =
            w.front_office.college.reach.iter().any(|r| r.id == id && r.kind == FAMILY && r.on.days_to(d) < FAMILY_GAP);
        if gap || w.folk.scenes.iter().any(|s| s.a == id as u64 && s.kind.is_pitch() && !s.done) {
            continue;
        }
        let Some((ev, g)) = family_reason(w, i, j) else { continue };
        log(w, id, FAMILY);
        let name = w.people[i].name.clone();
        let who = super::dialogue::decider_words(&w.people[i]).to_string();
        let first = first_of(&name).to_string();
        let caller =
            if who == "He decides himself" { name.clone() } else { who.replacen("His ", &format!("{first}'s "), 1) };
        let mut s = Scene::new(Kind::FamilyCall, d, format!("{caller} calls"));
        s.place = format!("A call from {caller}");
        s.a = id as u64;
        s.b = j as u64;
        s.f[2] = Point::ALL.iter().position(|&k| k == ev).unwrap_or(0) as f32;
        s.f[3] = g as u8 as f32;
        s.who = vec![Who::Player(id), Who::Group(who)];
        scene::add(w, s);
    }
}

/// The event and group a family call is about (`day`).
pub(crate) fn family_about(s: &Scene) -> Option<(Point, Group)> {
    if s.kind != Kind::FamilyCall {
        return None;
    }
    let k = Point::ALL.get(s.f[2] as usize).copied()?;
    let g = Group::ALL.get(s.f[3] as usize).copied()?;
    Some((k, g))
}

// ---------------------------------------------------------------------------
// AI staffs: the same talks in one step.

/// An AI head coach's sit-down at the end of a recruit's official visit:
/// his case in one step, a sit-down's points, folded into the school's
/// case (each group at its best, under the one cap).
pub(crate) fn ai_official_talk(w: &mut World, i: usize, j: usize, d: Date) {
    let hc = w.programs[j].head_coach;
    let seed = w.options.seed as u64 ^ (d.0 as u64) << 20 ^ (j as u64) << 40 ^ w.people[i].player.id as u64 ^ 0x0FF1;
    let back = case::backing(w, i, j);
    let c = case::one_step(w, i, j, hc, Ear::Decider, back, SIT_DOWN_EXCHANGES, 1.0, seed);
    case::case_mut(w, i, j).merge(&c);
}

/// Each week, every staff's calls to the recruits it offered in the
/// classes it may call (the era's rules; yours are your own): a call's
/// points in one step by his position coach, said to him.
pub(crate) fn ai_calls(w: &mut World, d: Date) {
    if d.weekday() != AI_CALL_DAY {
        return;
    }
    let mine = fc::visits_program(w);
    let sign_year = crate::world::recruiting::next_class(w);
    let world: &World = w;
    let made: Vec<(usize, Case)> = (0..w.people.len())
        .into_par_iter()
        .with_min_len(4096)
        .flat_map_iter(|i| {
            let p = &world.people[i];
            let out: Vec<(usize, Case)> = if p.stage != Stage::HighSchool
                || p.cal.signed.is_some()
                || p.class_of > sign_year + 1
                || p.class_of < sign_year
                || contact::call_rule(&world.dates, d, p.class_of, true, &[]).is_err()
            {
                Vec::new()
            } else {
                let id = p.player.id as u64;
                let mut schools: Vec<usize> = p.cal.offers.clone();
                schools.extend(p.cal.early_offers.iter().map(|o| o.0));
                schools.sort_unstable();
                schools.dedup();
                schools
                    .into_iter()
                    .filter(|&j| Some(j) != mine && j < world.programs.len())
                    .filter(|&j| {
                        key_uniform(world.options.seed as u64 ^ (d.0 as u64) << 20 ^ (j as u64) << 40 ^ 0xCA11, id)
                            < AI_CALL_WEEK
                    })
                    .map(|j| {
                        let caller = super::dialogue::caller(world, i, j);
                        let back = case::backing(world, i, j);
                        let seed = world.options.seed as u64 ^ (d.0 as u64) << 18 ^ (j as u64) << 42 ^ id ^ 0xCA12;
                        (i, case::one_step(world, i, j, caller, Ear::Himself, back, CALL_EXCHANGES, PHONE, seed))
                    })
                    .collect()
            };
            out
        })
        .collect();
    for (i, c) in made {
        case::case_mut(w, i, c.prog as usize).merge(&c);
    }
}
