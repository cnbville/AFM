//! The tour on an official visit (Capital lane C4, CP20).
//!
//! On an official visit weekend you choose the three rooms a recruit
//! tours. His wants decide which matter: a lineman the weight room and the
//! NFL wall, a kid whose family wants a degree the academic center, a kid
//! after the money the content studio. He notices the room you skipped.
//!
//! Left to your staff, the tour is the usual one: the three rooms that read
//! best against the conference (the program's pride, whatever the kid
//! wants). That tour is the built official visit's pull; a tour you choose
//! moves the weekend's pull up or down against it, by how much better (or
//! worse) it fits the kid (`lift`). Only your seat chooses tours, so the
//! world plays the same on every seed. Sizes are Claude's (proposed, not
//! confirmed).

use super::rooms::{ROOMS, Room};
use crate::positions::Position;
use crate::world::program::Program;
use crate::world::{Person, Visit, World};

/// Rooms on a tour.
pub const STOPS: usize = 3;
/// The tour's lift on the official weekend's pull: this much a point of fit
/// over your staff's usual tour, at most `MAX` either way (the locker room's
/// own lift on the weekend is ±25%).
pub const SLOPE: f64 = 1.0;
pub const MAX: f64 = 0.25;
/// What skipping the room he cares about most costs the tour's fit, as a
/// share of that room's weight: he asks where it was.
pub const SKIP: f64 = 0.5;
/// A tour is set from three weeks before the weekend to two days after its
/// Friday (the Sunday).
pub const AHEAD: i32 = 21;
pub const AFTER: i32 = 2;
/// Tours kept (two years).
pub const KEEP_DAYS: i32 = 730;

/// A tour you chose.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Tour {
    /// The recruit (player id).
    pub kid: u32,
    pub program: u32,
    /// The day of his official visit.
    pub day: crate::calendar::Date,
    /// The three rooms (`Room as u8`).
    pub rooms: Vec<u8>,
    /// The weekend's pull against your staff's usual tour (`lift`).
    pub lift: f32,
    /// How he took it, in words.
    pub words: String,
}

/// How much a recruit cares about each room on a tour, summing to 1: from
/// what his family weighs (development, academics, money, the name), his
/// position and how much campus life means to him.
pub fn wants(p: &Person) -> [f64; ROOMS] {
    let w = &p.family.decider_wants;
    let pos = p.player.position;
    let big = matches!(pos, Position::OT | Position::IOL | Position::DT | Position::EDGE | Position::TE);
    let thinker = matches!(pos, Position::QB | Position::LB | Position::S);
    let mut v = [0.0f64; ROOMS];
    let mut set = |r: Room, x: f64| v[r as usize] += x.max(0.0);
    // The wow room every kid walks into, the name's own room.
    set(Room::Locker, 0.06 + 0.25 * w.prestige);
    set(Room::Weight, w.development * if big { 0.55 } else { 0.3 });
    set(Room::Nutrition, w.development * if big { 0.2 } else { 0.06 });
    set(Room::SportsScience, 0.12 * w.development);
    set(Room::Recovery, 0.1 * w.development);
    set(Room::Film, w.development * if thinker { 0.4 } else { 0.12 });
    set(Room::IndoorField, 0.02);
    set(Room::RecruitingLounge, 0.01);
    set(Room::PlayersLounge, 0.5 * super::rooms::campus_life_want(p));
    set(Room::Academic, 0.9 * w.academics);
    set(Room::ContentStudio, 0.6 * w.money);
    set(Room::TrophyHall, 0.3 * w.development + 0.25 * w.prestige);
    set(Room::Stadium, 0.25 * w.prestige + 0.04);
    let s: f64 = v.iter().sum::<f64>().max(1e-9);
    for x in v.iter_mut() {
        *x /= s;
    }
    v
}

/// The room he cares about most.
pub fn top(wants: &[f64; ROOMS]) -> Room {
    let k = (0..ROOMS).max_by(|&a, &b| wants[a].total_cmp(&wants[b]).then(b.cmp(&a))).unwrap_or(0);
    Room::ALL[k]
}

/// How a room shows on a tour: its read against the conference over the
/// division's typical room, -1 to 1 (two levels either way is the end).
fn shows(prog: &Program, r: Room) -> f64 {
    let c = &prog.campus;
    let k = r as usize;
    ((c.read[k] - c.typical_read[k]) as f64 / 2.0).clamp(-1.0, 1.0)
}

/// Rooms a program can tour (it has them).
pub fn tourable(prog: &Program) -> Vec<Room> {
    Room::ALL.into_iter().filter(|&r| prog.campus.level(r) >= 0.5).collect()
}

/// Your staff's usual tour: the three rooms that read best (ties in the
/// spec's order).
pub fn usual(prog: &Program) -> Vec<Room> {
    let mut v = tourable(prog);
    v.sort_by(|&a, &b| prog.campus.read[b as usize].total_cmp(&prog.campus.read[a as usize]).then(a.cmp(&b)));
    v.truncate(STOPS);
    v
}

/// How well a tour fits him: each toured room's weight times how it shows,
/// less what skipping his first room costs.
pub fn fit(prog: &Program, wants: &[f64; ROOMS], rooms: &[Room]) -> f64 {
    let shown: f64 = rooms.iter().map(|&r| wants[r as usize] * shows(prog, r)).sum();
    let t = top(wants);
    let skipped = if rooms.contains(&t) { 0.0 } else { SKIP * wants[t as usize] };
    shown - skipped
}

/// The tour's lift on the weekend's pull against your staff's usual tour.
pub fn lift_of(prog: &Program, wants: &[f64; ROOMS], rooms: &[Room]) -> f64 {
    (SLOPE * (fit(prog, wants, rooms) - fit(prog, wants, &usual(prog)))).clamp(-MAX, MAX)
}

/// The tour's lift for a kid's official visit at program `j` (0 when you
/// chose none: the built weekend). Read by the recruit's appeal.
pub fn lift(w: &World, kid: u32, j: usize) -> f64 {
    let t = &w.capital.tours;
    if t.is_empty() {
        return 0.0;
    }
    let lo = t.partition_point(|x| x.kid < kid);
    t[lo..].iter().take_while(|x| x.kid == kid).find(|x| x.program == j as u32).map_or(0.0, |x| x.lift as f64)
}

/// His official visit to `prog` that a tour can be set for (from three
/// weeks ahead to the weekend's Sunday), if any.
pub fn visit_day(w: &World, i: usize, prog: usize) -> Option<crate::calendar::Date> {
    let today = w.today;
    w.people[i]
        .cal
        .visits
        .iter()
        .filter(|v| v.0 == prog && v.2 == Visit::Official)
        .map(|v| v.1)
        .find(|&d| d.add(AFTER) >= today && today.add(AHEAD) >= d)
}

/// You set a recruit's tour (three rooms your program has) for his official
/// visit: his take on it in words.
pub fn set(w: &mut World, kid: u32, rooms: &[Room]) -> Result<String, String> {
    let prog = super::week::your_program(w).ok_or("Tours are a college head coach's call.")?;
    let i = crate::world::folk::person(w, kid).ok_or("No such recruit.")?;
    let day = visit_day(w, i, prog).ok_or("He has no official visit with you to tour.")?;
    if rooms.len() != STOPS {
        return Err("A tour is three rooms.".into());
    }
    let have = tourable(&w.programs[prog]);
    let mut seen: Vec<Room> = Vec::new();
    for &r in rooms {
        if !have.contains(&r) {
            return Err(format!("You have no {} to show him.", r.words()));
        }
        if seen.contains(&r) {
            return Err("Three different rooms.".into());
        }
        seen.push(r);
    }
    let p = &w.people[i];
    let wants = wants(p);
    let pr = &w.programs[prog];
    let lift = lift_of(pr, &wants, rooms);
    let words = reaction(pr, p, &wants, rooms);
    let t = &mut w.capital.tours;
    let cut = w.today.add(-KEEP_DAYS);
    t.retain(|x| x.day >= cut && !(x.kid == kid && x.program == prog as u32));
    let at = t.partition_point(|x| x.kid <= kid);
    t.insert(
        at,
        Tour {
            kid,
            program: prog as u32,
            day,
            rooms: rooms.iter().map(|&r| r as u8).collect(),
            lift: lift as f32,
            words: words.clone(),
        },
    );
    Ok(words)
}

/// What he made of a tour: the room he lit up in, the one that left him
/// cold, and the one he asked about that you skipped.
pub fn reaction(prog: &Program, p: &Person, wants: &[f64; ROOMS], rooms: &[Room]) -> String {
    let first = p.name.split_whitespace().next().unwrap_or(&p.name).to_string();
    let mut best: Option<(Room, f64)> = None;
    let mut worst: Option<(Room, f64)> = None;
    for &r in rooms {
        let x = wants[r as usize] * shows(prog, r);
        if best.is_none_or(|b| x > b.1) {
            best = Some((r, x));
        }
        if worst.is_none_or(|b| x < b.1) {
            worst = Some((r, x));
        }
    }
    let mut parts: Vec<String> = Vec::new();
    if let Some((r, x)) = best.filter(|b| b.1 > 0.02) {
        parts.push(if x > 0.08 {
            format!("{first} lit up in the {}", r.words())
        } else {
            format!("{first} liked the {}", r.words())
        });
    }
    if let Some((r, _)) = worst.filter(|b| b.1 < -0.02) {
        parts.push(format!("the {} left him cold", r.words()));
    }
    let t = top(wants);
    if !rooms.contains(&t) {
        parts.push(format!("he asked where the {} was", t.words()));
    }
    if parts.is_empty() {
        return format!("{first} was polite about all of it.");
    }
    let mut s = parts.join("; ");
    if let Some(c) = s.get_mut(0..1) {
        c.make_ascii_uppercase();
    }
    s.push('.');
    s
}

/// Your staff's read on what a recruit will want to see, in words: by his
/// position, and his family's first want once you have sat with them.
pub fn staff_read(p: &Person, family_heard: Option<&str>) -> String {
    let first = p.name.split_whitespace().next().unwrap_or(&p.name).to_string();
    let pos = p.player.position;
    let guess = match pos {
        Position::OT | Position::IOL | Position::DT | Position::EDGE | Position::TE => {
            "a big man usually wants the weight room and the NFL wall"
        }
        Position::QB => "a quarterback usually wants the film rooms and the NFL wall",
        Position::LB | Position::S => "his position usually wants the film rooms and the weight room",
        _ => "his position usually wants the locker room and the NFL wall",
    };
    match family_heard {
        Some(top) => format!("{first}: {guess}; his family told you they want {top} most."),
        None => format!("{first}: {guess}."),
    }
}
