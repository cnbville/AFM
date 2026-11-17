//! The big moments' data (Capital lane C4, CP38): the groundbreaking, the
//! reveal video and the ribbon cutting at your program, for the full-screen
//! moments (lane CS7) with the donors, the recruits there and their
//! reactions, and the next day's headline.
//!
//! - **The groundbreaking:** the first Saturday within two weeks of the day
//!   ground breaks with recruits on campus (real programs set it on a big
//!   visit weekend), else the Saturday after; the recruits on campus that
//!   weekend are there.
//! - **The reveal video:** the day the drive goes public (the renderings);
//!   the recruits it reaches are the ones with your offer, the best first.
//! - **The ribbon cutting:** the Friday before your first home game of the
//!   season the building opens, with that weekend's visitors.
//!
//! Each recruit's reaction reads the building's rooms against what he
//! wants on a tour (`tour::wants`). Words and data only: the building's
//! pull on recruits is the rooms' and the groundbreaking's, as built. Only
//! your seat keeps them. Sizes are Claude's (proposed, not confirmed).

use super::rooms::Room;
use crate::calendar::{Date, Level, Loudness, Weekday};
use crate::world::{Stage, World};

/// The three moments.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Rite {
    #[default]
    Groundbreaking,
    Reveal,
    Ribbon,
}

impl Rite {
    pub fn words(self) -> &'static str {
        match self {
            Rite::Groundbreaking => "the groundbreaking",
            Rite::Reveal => "the reveal video",
            Rite::Ribbon => "the ribbon cutting",
        }
    }
}

/// A recruit there and how he took it.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Reaction {
    pub kid: u32,
    /// The room in the building he cared about most (`Room as u8`).
    pub room: u8,
    /// How much the building's rooms mean to him, 0-1.
    pub strength: f32,
    pub words: String,
}

/// One of your program's big moments.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Ceremony {
    pub id: u32,
    pub program: u32,
    pub drive: u32,
    pub rite: Rite,
    /// The day it is held.
    pub day: Date,
    pub held: bool,
    /// The building ("the Hale Football Building" once named).
    pub building: String,
    pub rooms: u16,
    /// The donors on the stage, the biggest gift first (the building's
    /// namer first when it has one), with their pledged totals.
    pub donors: Vec<(u32, f64)>,
    pub kids: Vec<Reaction>,
    /// The next day's headline.
    pub headline: String,
}

/// Days a groundbreaking waits for a weekend with recruits on campus.
pub const WAIT_FOR_VISITORS: i32 = 14;
/// Donors on the stage, and recruits a reveal video counts.
pub const STAGE: usize = 6;
pub const REVEAL_KIDS: usize = 8;

/// Your program's moments each morning: set the day of each new one, and
/// hold today's.
pub(crate) fn day(w: &mut World, d: Date) {
    let Some(p) = crate::world::ground::week::seat_program(w) else { return };
    if w.capital.off {
        return;
    }
    schedule(w, p, d);
    let today: Vec<usize> = (0..w.capital.ceremonies.len())
        .filter(|&k| !w.capital.ceremonies[k].held && w.capital.ceremonies[k].day == d)
        .collect();
    for k in today {
        hold(w, k);
    }
}

fn has(w: &World, drive: u32, rite: Rite) -> bool {
    w.capital.ceremonies.iter().any(|c| c.drive == drive && c.rite == rite)
}

/// Visitors on campus on a day (any visit).
fn visitors(w: &World, p: usize, from: Date, to: Date) -> Vec<usize> {
    let mut v: Vec<usize> = w
        .people
        .iter()
        .enumerate()
        .filter(|(_, q)| matches!(q.stage, Stage::HighSchool | Stage::PrepSchool))
        .filter(|(_, q)| q.cal.visits.iter().any(|x| x.0 == p && x.1 >= from && x.1 <= to))
        .map(|(i, _)| i)
        .collect();
    v.sort_by_key(|&i| (std::cmp::Reverse(w.people[i].stars), w.people[i].player.id));
    v
}

pub(crate) fn schedule(w: &mut World, p: usize, d: Date) {
    let recent = |x: Option<Date>| x.is_some_and(|x| x <= d && x.add(3) >= d);
    let drives: Vec<super::Drive> = w.capital.drives.iter().filter(|x| x.program == p as u32).cloned().collect();
    for x in drives {
        if recent(x.ground_day) && !has(w, x.id, Rite::Groundbreaking) {
            let g = x.ground_day.unwrap_or(d);
            let first_sat = g.add(1).on_or_after(Weekday::Sat);
            let mut day = first_sat;
            let mut s = first_sat;
            while g.days_to(s) <= WAIT_FOR_VISITORS {
                if !visitors(w, p, s.add(-1), s.add(1)).is_empty() {
                    day = s;
                    break;
                }
                s = s.add(7);
            }
            add(w, &x, Rite::Groundbreaking, day.max(d));
        }
        if recent(x.public_on) && !has(w, x.id, Rite::Reveal) {
            add(w, &x, Rite::Reveal, d);
        }
        let opened = x.opens.is_some_and(|y| y == w.year) && !x.shelved && d >= w.dates.money_year;
        if opened && x.ground.is_some() && !has(w, x.id, Rite::Ribbon) {
            let first = crate::world::front_office::college::home_games(w, p)
                .into_iter()
                .map(|g| g.0)
                .find(|&g| g > d)
                .map_or(Date::ymd(w.year, 8, 15), |g| g.add(-1));
            add(w, &x, Rite::Ribbon, first.max(d));
        }
    }
}

fn add(w: &mut World, x: &super::Drive, rite: Rite, day: Date) {
    let id = w.capital.id();
    w.capital.ceremonies.push(Ceremony {
        id,
        program: x.program,
        drive: x.id,
        rite,
        day,
        held: false,
        building: x.kind.clone(),
        rooms: x.rooms,
        donors: Vec::new(),
        kids: Vec::new(),
        headline: String::new(),
    });
}

/// How a recruit takes the building: what its rooms mean to him.
pub fn reaction(w: &World, i: usize, rooms: u16) -> Reaction {
    let p = &w.people[i];
    let wants = super::tour::wants(p);
    let first = p.name.split_whitespace().next().unwrap_or(&p.name).to_string();
    let inside: Vec<Room> = Room::ALL.into_iter().filter(|r| rooms & r.bit() != 0).collect();
    let strength: f64 = inside.iter().map(|&r| wants[r as usize]).sum();
    let best = inside.iter().copied().max_by(|&a, &b| wants[a as usize].total_cmp(&wants[b as usize]).then(b.cmp(&a)));
    let top = super::tour::top(&wants);
    let room = best.unwrap_or(top);
    let words = if strength >= 0.45 {
        format!("{first} couldn't stop talking about the new {}.", room.words())
    } else if strength >= 0.25 {
        format!("{first} liked what he saw in the {}.", room.words())
    } else {
        format!("{first} was polite; he cares more about the {}.", top.words())
    };
    Reaction { kid: p.player.id, room: room as u8, strength: strength as f32, words }
}

pub(crate) fn hold(w: &mut World, k: usize) {
    let c = w.capital.ceremonies[k].clone();
    let p = c.program as usize;
    // The donors on the stage.
    let mut by: Vec<(u32, f64)> = Vec::new();
    for y in w.capital.pledges.iter().filter(|y| y.drive == c.drive && y.pulled.is_none()) {
        match by.iter_mut().find(|b| b.0 == y.donor) {
            Some(b) => b.1 += y.total,
            None => by.push((y.donor, y.total)),
        }
    }
    by.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    let namer = w.capital.names.iter().find(|n| n.drive == c.drive && n.room.is_none() && n.up()).cloned();
    if let Some(n) = &namer
        && let Some(k) = by.iter().position(|b| b.0 == n.donor)
    {
        let x = by.remove(k);
        by.insert(0, x);
    }
    by.truncate(STAGE);
    let building = namer.as_ref().map_or_else(|| format!("the new {}", c.building), |n| n.words(w));
    // The recruits there.
    let kids: Vec<usize> = match c.rite {
        Rite::Reveal => {
            let mut v: Vec<usize> = w
                .people
                .iter()
                .enumerate()
                .filter(|(_, q)| matches!(q.stage, Stage::HighSchool | Stage::PrepSchool) && q.cal.offers.contains(&p))
                .filter(|(_, q)| q.cal.signed.is_none())
                .map(|(i, _)| i)
                .collect();
            v.sort_by_key(|&i| (std::cmp::Reverse(w.people[i].stars), w.people[i].player.id));
            v.truncate(REVEAL_KIDS);
            v
        }
        _ => visitors(w, p, c.day.add(-1), c.day.add(1)),
    };
    let reactions: Vec<Reaction> = kids.iter().map(|&i| reaction(w, i, c.rooms)).collect();
    let pname = w.programs[p].name(w.options.names).to_string();
    let lead = by.first().and_then(|b| w.capital.donor(b.0)).map(|d| d.name.clone());
    let n = reactions.len();
    let headline = match c.rite {
        Rite::Groundbreaking => match &lead {
            Some(l) => format!("{pname} breaks ground on {building}; {l} holds a gold shovel{}.", kids_words(n)),
            None => format!("{pname} breaks ground on {building}{}.", kids_words(n)),
        },
        Rite::Reveal => {
            format!("{pname}'s reveal video for {building} lands with recruits{}.", reach_words(&reactions))
        }
        Rite::Ribbon => format!("{pname} cuts the ribbon on {building}{}.", kids_words(n)),
    };
    {
        let x = &mut w.capital.ceremonies[k];
        x.held = true;
        x.donors = by;
        x.kids = reactions;
        x.building = building;
        x.headline = headline.clone();
    }
    // The next day's paper.
    let e =
        crate::calendar::Event::new(c.day.add(1), Level::College, crate::calendar::Kind::News, Loudness::Big, headline)
            .rule("money")
            .program(p);
    crate::world::finance::post(w, e);
}

fn kids_words(n: usize) -> String {
    match n {
        0 => String::new(),
        1 => ", with a recruit on campus watching".into(),
        n => format!(", with {n} recruits on campus watching"),
    }
}

fn reach_words(r: &[Reaction]) -> String {
    let big = r.iter().filter(|x| x.strength >= 0.45).count();
    if big == 0 { String::new() } else { format!(": {big} of your top targets share it within the hour") }
}
