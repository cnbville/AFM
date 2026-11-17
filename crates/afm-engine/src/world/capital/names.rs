//! Names on buildings, and a named donor's scandal (Capital lane C4, CP25).
//!
//! - **Names:** when a drive breaks ground, the donor who gave a quarter of
//!   its goal names the building, and each who gave a twentieth names the
//!   room he gave to (real naming gifts: the building's lead gift, then the
//!   rooms). The donors who already have their names up when the world
//!   hands over come from the store: the richest old donors with a room
//!   they'd give to, about half of them (`seed`). Names are words only; for
//!   every program, so they change nothing in the world.
//! - **Scandal** (your program only): a named donor's fraud case, a slur on
//!   tape, a suit; about 1 in 100 a year for each name up. The athletic
//!   director brings you the call: take it down (his circle cools, his open
//!   pledges may go with him, the press approves) or keep it (he and his
//!   friends remember, the questions keep coming for six months and the AD
//!   gets nervous). Left a week, the university decides by how bad it is.
//!   Louisville took Papa John's name off its stadium in 2018.
//!
//! Sizes are Claude's (proposed, not confirmed).

use super::scenes::{self, Kind, NAME_DOWN, NAME_KEEP, NAME_LEAVE};
use crate::calendar::{Date, Level, Loudness};
use crate::world::World;
use crate::world::folk::Team;
use crate::world::folk::scene::{self as fscene, Kind as SceneKind, Scene};

/// A name on a building or a room.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Name {
    pub id: u32,
    pub donor: u32,
    pub program: u32,
    /// The drive whose building it is on (0: up before the world's drives).
    pub drive: u32,
    /// The building: a hall on the campus map, or a drive's building.
    pub hall: String,
    /// The room it names (`Room as u8`); None names the whole building.
    pub room: Option<u8>,
    pub since: i32,
    /// The day it came down.
    pub down: Option<Date>,
}

/// A named donor's scandal at your program.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Scandal {
    pub id: u32,
    pub name: u32,
    pub donor: u32,
    pub program: u32,
    pub day: Date,
    /// What came out ("is charged with fraud in his business").
    pub what: String,
    /// How bad it is, 0-1.
    pub severity: f32,
    /// The AD's call scene.
    pub scene: u32,
    /// Taken down (true) or kept (false), once decided.
    pub down: Option<bool>,
    /// You made the call (false: the university did).
    pub yours: bool,
    /// Kept: the questions come monthly until this day.
    pub questions_until: Option<Date>,
}

/// A building's naming gift, and a room's, as shares of the drive's goal.
pub const BUILDING_SHARE: f64 = 0.25;
pub const ROOM_SHARE: f64 = 0.05;
/// The names up when the world hands over: donors who could name a room
/// (in 2026 dollars), the richest three a program, half of them.
pub const SEED_FLOOR: f64 = 2.0e6;
pub const SEED_MOST: usize = 3;
pub const SEED_CHANCE: f64 = 0.5;
/// A scandal's chance a year for each name up at your program.
pub const SCANDAL_YEAR: f64 = 0.01;
/// The university takes a name down on its own from this severity.
pub const UNIVERSITY_DOWN: f32 = 0.6;
/// Taken down: his warmth toward you falls to at most this, his open
/// pledges go with him at this chance, the AD's trust (the press approves).
pub const DOWN_WARMTH: f32 = 0.1;
pub const DOWN_PULLS: f64 = 0.6;
pub const DOWN_TRUST: f32 = 0.02;
/// Kept: his warmth, his old-money friends', the AD's trust now and each
/// month of questions (times the severity), for six months.
pub const KEEP_WARMTH: f32 = 0.1;
pub const KEEP_CIRCLE: f32 = 0.03;
pub const KEEP_TRUST: f32 = -0.02;
pub const QUESTIONS_TRUST: f32 = -0.025;
pub const QUESTIONS_MONTHS: i32 = 6;

/// What can come out about a donor, and how bad it is.
const SCANDALS: [(&str, f32); 6] = [
    ("is caught on tape using a racial slur", 0.9),
    ("is named in a sexual harassment suit by former employees", 0.8),
    ("is charged with fraud in his business", 0.7),
    ("is accused of hiding losses from his investors", 0.6),
    ("faces a federal tax case", 0.5),
    ("is arrested for driving drunk", 0.35),
];

fn title(s: &str) -> String {
    s.split(' ')
        .map(|x| {
            if x == "and" {
                x.to_string()
            } else {
                let mut c = x.chars();
                c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

impl Name {
    /// "the Hale Football Building", "the Hale Weight Room".
    pub fn words(&self, w: &World) -> String {
        let last = w
            .capital
            .donor(self.donor)
            .map_or_else(String::new, |d| crate::world::names::surname_of(&d.name).to_string());
        match self.room.and_then(super::rooms::Room::from_index) {
            Some(r) => format!("the {last} {}", title(r.words())),
            None => format!("the {last} {}", title(&self.hall)),
        }
    }
    pub fn up(&self) -> bool {
        self.down.is_none()
    }
}

/// The names up at a program.
pub fn of(w: &World, p: usize) -> Vec<&Name> {
    w.capital.names.iter().filter(|n| n.program == p as u32 && n.up()).collect()
}

/// Names each day: the store's names once the drives live, a name for each
/// building that broke ground, and at your program a scandal's roll and the
/// questions of a name kept.
pub(crate) fn day(w: &mut World, d: Date) {
    if !super::live(w) {
        return;
    }
    if !w.capital.seeded {
        seed(w);
        w.capital.seeded = true;
    }
    name_buildings(w);
    if let Some(p) = super::week::your_program(w) {
        roll(w, p, d);
        questions(w, p, d);
    }
}

/// The names already up when the world hands over.
pub(crate) fn seed(w: &mut World) {
    let year = w.year;
    let floor = SEED_FLOOR * super::donors::dollars(year);
    let mut out: Vec<Name> = Vec::new();
    for p in 0..w.programs.len() {
        let prog = &w.programs[p];
        if !prog.campus.set {
            continue;
        }
        let mut v: Vec<&super::Donor> = w
            .capital
            .donors
            .iter()
            .filter(|d| d.alive() && d.home == p as u32 && d.pro.is_none() && d.capacity >= floor)
            .collect();
        v.sort_by(|a, b| b.capacity.total_cmp(&a.capacity).then(a.id.cmp(&b.id)));
        let mut rooms_named: Vec<u8> = Vec::new();
        for d in v.into_iter().take(SEED_MOST) {
            if super::draw(w, 50, d.id as u64) >= SEED_CHANCE {
                continue;
            }
            let Some(r) = d
                .rooms
                .iter()
                .copied()
                .filter_map(super::rooms::Room::from_index)
                .find(|&r| prog.campus.level(r) >= 0.5 && !rooms_named.contains(&(r as u8)))
            else {
                continue;
            };
            let hall = prog.campus.halls.iter().find(|h| h.holds(r)).map_or("football offices", |h| h.name.as_str());
            let years = (1.0 + 14.0 * super::draw(w, 51, d.id as u64)) as i32;
            rooms_named.push(r as u8);
            out.push(Name {
                id: 0,
                donor: d.id,
                program: p as u32,
                drive: 0,
                hall: hall.to_lowercase(),
                room: Some(r as u8),
                since: (year - years).max(d.joined),
                down: None,
            });
        }
    }
    for mut n in out {
        n.id = w.capital.id();
        w.capital.names.push(n);
    }
}

/// Each building that broke ground gets its names: a quarter of the goal
/// names the building, a twentieth a room.
pub(crate) fn name_buildings(w: &mut World) {
    let todo: Vec<usize> = (0..w.capital.drives.len())
        .filter(|&k| {
            let x = &w.capital.drives[k];
            x.ground_day.is_some() && !x.shelved && !w.capital.named.contains(&x.id)
        })
        .collect();
    for k in todo {
        let x = w.capital.drives[k].clone();
        let mut by: Vec<(u32, f64, Option<u8>)> = Vec::new();
        for y in w.capital.pledges.iter().filter(|y| y.drive == x.id && y.pulled.is_none()) {
            match by.iter_mut().find(|b| b.0 == y.donor) {
                Some(b) => {
                    b.1 += y.total;
                    b.2 = b.2.or(y.room);
                }
                None => by.push((y.donor, y.total, y.room)),
            }
        }
        by.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        let year = x.ground.unwrap_or(w.year);
        let mut named_rooms: Vec<u8> = Vec::new();
        let mut building = false;
        for (n, &(donor, total, room)) in by.iter().enumerate() {
            if n == 0 && total >= BUILDING_SHARE * x.goal {
                building = true;
                let id = w.capital.id();
                w.capital.names.push(Name {
                    id,
                    donor,
                    program: x.program,
                    drive: x.id,
                    hall: x.kind.clone(),
                    room: None,
                    since: year,
                    down: None,
                });
                continue;
            }
            if total < ROOM_SHARE * x.goal {
                break;
            }
            let wants = w.capital.donor(donor).map_or_else(Vec::new, |d| d.rooms.clone());
            let Some(r) =
                room.into_iter().chain(wants).find(|&r| x.rooms & (1u16 << r) != 0 && !named_rooms.contains(&r))
            else {
                continue;
            };
            named_rooms.push(r);
            let id = w.capital.id();
            w.capital.names.push(Name {
                id,
                donor,
                program: x.program,
                drive: x.id,
                hall: x.kind.clone(),
                room: Some(r),
                since: year,
                down: None,
            });
        }
        let _ = building;
        w.capital.named.push(x.id);
    }
}

/// A scandal's roll each day for each name up at your program (one open at
/// a time).
pub(crate) fn roll(w: &mut World, p: usize, d: Date) {
    if w.capital.scandals.iter().any(|s| s.program == p as u32 && s.down.is_none()) {
        return;
    }
    let names: Vec<(u32, u32)> = w
        .capital
        .names
        .iter()
        .filter(|n| n.program == p as u32 && n.up())
        .filter(|n| w.capital.donor(n.donor).is_some_and(|x| x.alive()))
        .map(|n| (n.id, n.donor))
        .collect();
    for (name, donor) in names {
        let key = (name as u64) << 20 ^ d.0 as u64;
        if super::draw(w, 52, key) >= SCANDAL_YEAR / 365.0 {
            continue;
        }
        if w.capital.scandals.iter().any(|s| s.donor == donor) {
            continue;
        }
        let k = ((super::draw(w, 53, key) * SCANDALS.len() as f64) as usize).min(SCANDALS.len() - 1);
        start(w, p, name, donor, SCANDALS[k].0, SCANDALS[k].1, d);
        return;
    }
}

/// A scandal breaks: the news, and the AD's call on your desk.
pub fn start(w: &mut World, p: usize, name: u32, donor: u32, what: &str, severity: f32, d: Date) -> u32 {
    let id = w.capital.id();
    let dname = w.capital.donor(donor).map_or_else(String::new, |x| x.name.clone());
    let named = w.capital.names.iter().find(|n| n.id == name).map_or_else(String::new, |n| n.words(w));
    let text = format!("{dname} {what}. His name is on {named}.");
    let e = crate::world::finance::story(w, Level::College, Loudness::Big, text).program(p);
    crate::world::finance::post(w, e);
    let mut s = Scene::new(SceneKind::Capital, d, format!("The name on {named}"));
    s.b = Kind::Name.code();
    s.a = id as u64;
    // It waits for you a week, then the university decides.
    s.stops = false;
    s.place = "The athletic director's office".into();
    let ad = crate::world::folk::boss::boss(w, Team::College(p));
    s.who = vec![fscene::Who::Cast(ad.r.token())];
    let scene = fscene::add(w, s);
    w.capital.scandals.push(Scandal {
        id,
        name,
        donor,
        program: p as u32,
        day: d,
        what: what.to_string(),
        severity,
        scene,
        down: None,
        yours: false,
        questions_until: None,
    });
    id
}

pub(crate) fn open(w: &mut World, _prog: usize, s: &mut Scene) {
    let Some(x) = w.capital.scandals.iter().find(|x| x.id == s.a as u32).cloned() else {
        s.end();
        return;
    };
    let dname = w.capital.donor(x.donor).map_or_else(String::new, |d| d.name.clone());
    let named = w.capital.names.iter().find(|n| n.id == x.name).map_or_else(String::new, |n| n.words(w));
    let open: f64 = w
        .capital
        .pledges
        .iter()
        .filter(|y| y.donor == x.donor && y.program == x.program && y.open())
        .map(|y| y.left())
        .sum();
    let lean = if x.severity >= UNIVERSITY_DOWN {
        "The president's office wants it down by the end of the week."
    } else {
        "The president's office would rather ride it out."
    };
    s.narrate(format!("{dname} {}. His name is on {named}. {lean}", x.what));
    if open > 0.0 {
        s.narrate("He still owes on his pledge. Take his name down and that money may go with him.");
    }
    let circle = w.capital.donor(x.donor).and_then(|d| d.in_circle()).and_then(|c| w.capital.circle(c));
    if let Some(c) = circle {
        s.narrate(format!("His friends from {} are watching what you do.", c.name));
    }
    s.picks.clear();
    scenes::final_pick(s, NAME_DOWN, "Take his name down", "His circle cools; the press approves");
    scenes::final_pick(s, NAME_KEEP, "Keep his name up", "He remembers; the questions keep coming");
    s.pick(NAME_LEAVE, "Leave it to the university", "Defer", "", "The president's office decides");
}

pub(crate) fn answer(w: &mut World, _prog: usize, s: &mut Scene, code: u32) {
    let id = s.a as u32;
    let line = match code {
        NAME_DOWN => decide(w, id, true, true),
        NAME_KEEP => decide(w, id, false, true),
        _ => university(w, id),
    };
    s.outcome = vec![line];
    s.end();
}

pub(crate) fn hand(w: &mut World, s: &mut Scene) {
    let line = university(w, s.a as u32);
    s.outcome = vec![line];
}

/// The university's own call: down from `UNIVERSITY_DOWN`.
fn university(w: &mut World, id: u32) -> String {
    let Some(x) = w.capital.scandals.iter().find(|x| x.id == id) else { return String::new() };
    if x.down.is_some() {
        return String::new();
    }
    let down = x.severity >= UNIVERSITY_DOWN;
    decide(w, id, down, false)
}

/// Your call from the screen (the AD's scene in your inbox closes with it).
pub fn call(w: &mut World, id: u32, down: bool) -> Result<String, String> {
    let p = super::week::your_program(w).ok_or("Donors are the head coach's to court.")?;
    let x = w.capital.scandals.iter().find(|x| x.id == id && x.program == p as u32).cloned().ok_or("No such call.")?;
    if x.down.is_some() {
        return Err("That call is made.".into());
    }
    let line = decide(w, id, down, true);
    let said = line.clone();
    fscene::leave(w, x.scene, move |_, s| s.outcome = vec![line]);
    Ok(said)
}

/// The name comes down or stays.
pub fn decide(w: &mut World, id: u32, down: bool, yours: bool) -> String {
    let Some(k) = w.capital.scandals.iter().position(|x| x.id == id) else { return "No such call.".into() };
    if w.capital.scandals[k].down.is_some() {
        return "That call is made.".into();
    }
    let x = w.capital.scandals[k].clone();
    let p = x.program as usize;
    let today = w.today;
    let me = crate::world::scouting::readers::you(w).map(|c| c as u32);
    let dname = w.capital.donor(x.donor).map_or_else(String::new, |d| d.name.clone());
    let named = w.capital.names.iter().find(|n| n.id == x.name).map_or_else(String::new, |n| n.words(w));
    let ad = crate::world::folk::boss::boss(w, Team::College(p)).key();
    let pname = w.programs[p].name(w.options.names).to_string();
    let who = if yours { pname.clone() } else { format!("{pname}'s president") };
    {
        let s = &mut w.capital.scandals[k];
        s.down = Some(down);
        s.yours = yours;
    }
    let line = if down {
        if let Some(n) = w.capital.names.iter_mut().find(|n| n.id == x.name) {
            n.down = Some(today);
        }
        // His circle hears it as a betrayal of one of their own (CP4).
        if let Some(c) = me {
            super::donors::word(w, x.donor, c, false);
            if let Some(d) = w.capital.donor_mut(x.donor) {
                let b = d.bond_mut(c);
                b.warmth = b.warmth.min(DOWN_WARMTH);
            }
        }
        // His open pledges may go with him.
        let mut pulled = false;
        if super::draw(w, 54, x.id as u64) < DOWN_PULLS {
            for y in w.capital.pledges.iter_mut().filter(|y| y.donor == x.donor && y.program == x.program && y.open()) {
                y.pulled = Some(today);
                pulled = true;
            }
        }
        w.folk.mine.nudge(ad, DOWN_TRUST);
        let more = if pulled { " He pulls what was left of his pledge." } else { "" };
        format!("{who} takes {dname}'s name off {named}. The press approves.{more}")
    } else {
        if let Some(c) = me {
            let circle = w.capital.donor(x.donor).and_then(|d| d.in_circle());
            scenes::warm(w, x.donor, c, KEEP_WARMTH);
            if let Some(cid) = circle {
                let friends: Vec<u32> = w
                    .capital
                    .donors
                    .iter()
                    .filter(|m| m.id != x.donor && m.alive() && m.in_circle() == Some(cid))
                    .filter(|m| m.money == super::Money::Old)
                    .map(|m| m.id)
                    .collect();
                for f in friends {
                    scenes::warm(w, f, c, KEEP_CIRCLE);
                }
            }
        }
        w.folk.mine.nudge(ad, KEEP_TRUST);
        w.capital.scandals[k].questions_until = Some(today.add((30.44 * QUESTIONS_MONTHS as f64) as i32));
        format!("{who} keeps {dname}'s name on {named}. The questions won't stop soon.")
    };
    let e = crate::world::finance::story(w, Level::College, Loudness::Big, line.clone()).program(p);
    crate::world::finance::post(w, e);
    line
}

/// A name kept: the first of each month for six months, the questions and
/// the AD's nerves.
pub(crate) fn questions(w: &mut World, p: usize, d: Date) {
    if d.day() != 1 {
        return;
    }
    let open: Vec<(u32, u32, f32)> = w
        .capital
        .scandals
        .iter()
        .filter(|x| x.program == p as u32 && x.down == Some(false) && x.questions_until.is_some_and(|u| d <= u))
        .map(|x| (x.name, x.donor, x.severity))
        .collect();
    for (name, donor, severity) in open {
        let dname = w.capital.donor(donor).map_or_else(String::new, |x| x.name.clone());
        let named = w.capital.names.iter().find(|n| n.id == name).map_or_else(String::new, |n| n.words(w));
        let text = format!("The questions about {dname}'s name on {named} come up again.");
        let e = crate::world::finance::story(w, Level::College, Loudness::Small, text).program(p);
        crate::world::finance::post(w, e);
        let ad = crate::world::folk::boss::boss(w, Team::College(p)).key();
        w.folk.mine.nudge(ad, QUESTIONS_TRUST * severity);
    }
}
