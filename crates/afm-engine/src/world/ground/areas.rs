//! Ground's areas and the map's data (Ground lane G1a; spec
//! `docs/design/ground.md`, part A: GR1, GR3, GR4, GR6).
//!
//! Each state splits into the areas real staffs use, with their real names,
//! nicknames and area codes ("the 757", "the Valley", "the DMV"). An area is a
//! list of counties, and every high school's area is its county's. The table
//! is `data/real-world/ground_areas.json`, written by `tools/ground/areas.py`
//! from the hand-written seeds and the Census county shapes; it is one shared
//! table, never saved. `HighSchool::ground_area` holds each school's area,
//! filled when a world is built and when one is read.
//!
//! Also here: travel from a campus (drive or flight, GR4), how many schools a
//! man sees in a day in each area (GR4), each area's character in words from
//! the world's own data (GR3) and the map's history pins (GR6). The built
//! `World.areas` (one per county, for area pipelines) is a different thing and
//! stays as it is.

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::positions::Position;
use crate::world::data::Division;
use crate::world::high_school::HighSchool;
use crate::world::{Stage, World, geo};

const AREAS_JSON: &str = include_str!("../../../../../data/real-world/ground_areas.json");

/// One area, as staffs split their states.
#[derive(Clone, Debug)]
pub struct Area {
    /// The area's state (index into `geo::STATE_CENTERS`).
    pub state: usize,
    /// Its short key, unique in its state ("757", "valley").
    pub key: &'static str,
    /// Its plain name ("Hampton Roads").
    pub name: &'static str,
    /// What staffs call it ("the 757").
    pub nickname: &'static str,
    pub codes: Vec<&'static str>,
    /// Its counties as the atlas spells them; another state's county carries
    /// its state ("AL:Russell" in Georgia's Columbus and the Valley).
    pub counties: Vec<&'static str>,
    /// Its center and a label point inside it (latitude, longitude).
    pub center: (f64, f64),
    pub label: (f64, f64),
    pub sq_miles: f64,
    /// The real high schools in it (the town table's places).
    pub schools: u32,
    /// How close together its schools sit: each school's own county's schools
    /// per 100 square miles, averaged over its schools.
    pub near: f64,
}

#[derive(serde::Deserialize)]
struct Row {
    state: String,
    key: String,
    name: String,
    nickname: String,
    codes: Vec<String>,
    counties: Vec<String>,
    center: [f64; 2],
    label: [f64; 2],
    sq_miles: f64,
    schools: u32,
    near: f64,
}

#[derive(serde::Deserialize)]
struct Alias {
    state: String,
    county: String,
    area: String,
}

#[derive(serde::Deserialize)]
struct File {
    areas: Vec<Row>,
    aliases: Vec<Alias>,
}

struct Table {
    areas: Vec<Area>,
    /// (state, county as the town table spells it) to its area.
    by_county: HashMap<(usize, String), u16>,
    /// Each state's areas.
    by_state: Vec<Vec<u16>>,
}

fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

static TABLE: LazyLock<Table> = LazyLock::new(|| {
    let file: File = serde_json::from_str(AREAS_JSON).expect("ground_areas.json");
    let mut areas = Vec::with_capacity(file.areas.len());
    let mut by_county = HashMap::new();
    let mut by_state = vec![Vec::new(); geo::STATE_CENTERS.len()];
    for (k, r) in file.areas.into_iter().enumerate() {
        let state = geo::state_index(&r.state).expect("an area's state");
        let id = k as u16;
        by_state[state].push(id);
        for c in &r.counties {
            let (st, name) = match c.split_once(':') {
                Some((st, name)) => (geo::state_index(st).expect("a county's state"), name),
                None => (state, c.as_str()),
            };
            by_county.insert((st, name.to_string()), id);
        }
        areas.push(Area {
            state,
            key: leak(r.key),
            name: leak(r.name),
            nickname: leak(r.nickname),
            codes: r.codes.into_iter().map(leak).collect(),
            counties: r.counties.into_iter().map(leak).collect(),
            center: (r.center[0], r.center[1]),
            label: (r.label[0], r.label[1]),
            sq_miles: r.sq_miles,
            schools: r.schools,
            near: r.near,
        });
    }
    for a in file.aliases {
        let state = geo::state_index(&a.state).expect("an alias's state");
        let id = by_state[state].iter().copied().find(|&i| areas[i as usize].key == a.area).expect("an alias's area");
        by_county.insert((state, a.county), id);
    }
    Table { areas, by_county, by_state }
});

/// Every area, in the data's order (an area's number is its place here).
pub fn table() -> &'static [Area] {
    &TABLE.areas
}

/// One area.
pub fn area(id: u16) -> &'static Area {
    &TABLE.areas[id as usize]
}

/// An area's id for the map art and the screens: its state and key ("GA-404").
pub fn code(id: u16) -> String {
    let a = area(id);
    format!("{}-{}", geo::STATE_CENTERS[a.state].0, a.key)
}

/// The area with this code ("GA-404").
pub fn find(code: &str) -> Option<u16> {
    let (st, key) = code.split_once('-')?;
    let s = geo::state_index(st)?;
    TABLE.by_state[s].iter().copied().find(|&i| area(i).key == key)
}

/// A state's areas.
pub fn in_state(state: usize) -> &'static [u16] {
    TABLE.by_state.get(state).map_or(&[], |v| v.as_slice())
}

/// The area a school sits in: its county's, or (a county the table doesn't
/// know, or none) the state's area whose center is nearest the town.
pub fn area_for(state: usize, county: &str, place: (f64, f64)) -> u16 {
    if let Some(&id) = TABLE.by_county.get(&(state, county.to_string())) {
        return id;
    }
    let own = in_state(state);
    own.iter()
        .copied()
        .min_by(|&a, &b| {
            geo::miles_between(place, area(a).center).total_cmp(&geo::miles_between(place, area(b).center))
        })
        .unwrap_or(0)
}

/// Fills every school's ground area (a world being built, or one read).
pub fn place_schools(schools: &mut [HighSchool]) {
    for s in schools {
        s.ground_area = area_for(s.state, &s.county, geo::place(s.state, &s.town));
    }
}

/// How many schools one man sees in a day in an area (GR4): two to five in a
/// city, one or two in the country, by how close its schools sit. Proposed,
/// not confirmed: the cut points.
pub fn schools_a_day(id: u16) -> u32 {
    match area(id).near {
        n if n >= 6.0 => 5,
        n if n >= 3.0 => 4,
        n if n >= 1.5 => 3,
        n if n >= 0.75 => 2,
        _ => 1,
    }
}

/// Road miles are about 1.25 straight-line miles; the road is 55 an hour.
pub const ROAD_FACTOR: f64 = 1.25;
pub const ROAD_MPH: f64 = 55.0;
/// Over five hours on the road, a man flies.
pub const FLY_OVER_HOURS: f64 = 5.0;
/// A flight's door-to-door hours on top of the air: to the airport and
/// through it, then the rental car to the school (proposed, not confirmed).
pub const FLIGHT_GROUND_HOURS: f64 = 2.5;
pub const FLIGHT_MPH: f64 = 450.0;

/// How a man gets from one place to another (GR4).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Trip {
    /// Road miles and hours behind the wheel.
    Drive { miles: f64, hours: f64 },
    /// Straight-line miles and door-to-door hours.
    Fly { miles: f64, hours: f64 },
}

impl Trip {
    pub fn hours(self) -> f64 {
        match self {
            Trip::Drive { hours, .. } | Trip::Fly { hours, .. } => hours,
        }
    }

    pub fn flight(self) -> bool {
        matches!(self, Trip::Fly { .. })
    }

    /// "a 2-hour drive", "a flight, about 4 hours door to door".
    pub fn words(self) -> String {
        match self {
            Trip::Drive { hours, .. } if hours < 0.75 => {
                format!("{} minutes away", ((hours * 60.0 / 5.0).round() * 5.0).max(5.0))
            }
            Trip::Drive { hours, .. } => format!("a {}-hour drive", half_hours(hours)),
            Trip::Fly { hours, .. } => format!("a flight, about {} hours door to door", half_hours(hours)),
        }
    }
}

fn half_hours(h: f64) -> String {
    let r = (h * 2.0).round() / 2.0;
    if r.fract() == 0.0 { format!("{}", r as i64) } else { format!("{r:.1}") }
}

/// The trip between two places (latitude, longitude).
pub fn trip(from: (f64, f64), to: (f64, f64)) -> Trip {
    let straight = geo::miles_between(from, to);
    let miles = straight * ROAD_FACTOR;
    let hours = miles / ROAD_MPH;
    if hours > FLY_OVER_HOURS {
        Trip::Fly { miles: straight, hours: FLIGHT_GROUND_HOURS + straight / FLIGHT_MPH }
    } else {
        Trip::Drive { miles, hours }
    }
}

/// A program's campus.
pub fn campus(w: &World, program: usize) -> (f64, f64) {
    let p = &w.programs[program];
    geo::campus(p.state, &p.city)
}

/// A high school's town.
pub fn school_place(w: &World, school: usize) -> (f64, f64) {
    let s = &w.high_schools[school];
    geo::place(s.state, &s.town)
}

/// From a program's campus to a high school.
pub fn trip_to_school(w: &World, program: usize, school: usize) -> Trip {
    trip(campus(w, program), school_place(w, school))
}

/// From a program's campus to an area's center.
pub fn trip_to_area(w: &World, program: usize, id: u16) -> Trip {
    trip(campus(w, program), area(id).center)
}

/// The world's high schools in each area, by area.
pub fn schools_by_area(w: &World) -> Vec<Vec<usize>> {
    let mut out = vec![Vec::new(); table().len()];
    for s in &w.high_schools {
        out[s.ground_area as usize].push(s.id);
    }
    out
}

/// An area in words (GR3): the public line everyone sees, and the words
/// your staff learn as they work there (the screens show them once you are
/// Known in the area).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Character {
    pub public: String,
    pub words: Vec<String>,
}

/// What the words are read from, per area: the last four classes.
#[derive(Clone, Debug, Default)]
pub struct Facts {
    pub schools: u32,
    /// Schools in the state's biggest classification, private schools.
    pub big: u32,
    pub private: u32,
    pub enrollment: f64,
    pub money: f64,
    /// Three-star and better recruits, FBS signees and the programs that
    /// signed them.
    pub recruits: u32,
    pub signees: u32,
    pub programs: u32,
    /// Of the three-star and better: skill-speed spots (RB, WR, CB, S) and
    /// the big men (OT, IOL, DT, EDGE).
    pub speed: u32,
    pub line: u32,
}

const SPEED: [Position; 4] = [Position::RB, Position::WR, Position::CB, Position::S];
const LINE: [Position; 4] = [Position::OT, Position::IOL, Position::DT, Position::EDGE];

/// Every area's facts from the world, in one pass over its people and
/// alumni.
pub fn facts(w: &World) -> Vec<Facts> {
    let n = table().len();
    let mut f = vec![Facts::default(); n];
    for s in &w.high_schools {
        let a = &mut f[s.ground_area as usize];
        a.schools += 1;
        a.big += u32::from(s.classification == 0);
        a.private += u32::from(s.private);
        a.enrollment += s.enrollment as f64;
        a.money += s.area_money;
    }
    for a in &mut f {
        if a.schools > 0 {
            a.enrollment /= a.schools as f64;
            a.money /= a.schools as f64;
        }
    }
    let mut signed: Vec<Vec<usize>> = vec![Vec::new(); n];
    let recent = |class_of: i32| class_of > w.year - 4 && class_of <= w.year;
    let mut count = |hs: usize, stars: u8, pos: Position, first: Option<usize>| {
        let Some(s) = w.high_schools.get(hs) else { return };
        let k = s.ground_area as usize;
        let a = &mut f[k];
        if stars >= 3 {
            a.recruits += 1;
            a.speed += u32::from(SPEED.contains(&pos));
            a.line += u32::from(LINE.contains(&pos));
        }
        if let Some(c) = first.filter(|&c| w.programs.get(c).is_some_and(|p| p.division == Division::FBS)) {
            a.signees += 1;
            if !signed[k].contains(&c) {
                signed[k].push(c);
            }
        }
    };
    for p in &w.people {
        if recent(p.class_of) && p.stage != Stage::HighSchool {
            count(p.hs, p.stars, p.player.position, p.path.first().copied());
        }
    }
    for a in &w.alumni {
        if recent(a.class_of) {
            count(a.high_school, a.stars, a.position, a.colleges.first().copied());
        }
    }
    for (a, s) in f.iter_mut().zip(signed) {
        a.programs = s.len() as u32;
    }
    f
}

/// Every area's character (GR3), from the world's own data: talent against
/// the nation, how many programs work it, what kind of players it grows,
/// school sizes, money and the state's football culture. Each word is
/// relative to the whole country, so it reads the same at any world scale.
pub fn characters(w: &World) -> Vec<Character> {
    let f = facts(w);
    let all_schools: f64 = f.iter().map(|a| a.schools as f64).sum::<f64>().max(1.0);
    let all_recruits: f64 = f.iter().map(|a| a.recruits as f64).sum();
    let per_school = all_recruits / all_schools;
    let speed_all = f.iter().map(|a| a.speed as f64).sum::<f64>() / all_recruits.max(1.0);
    let line_all = f.iter().map(|a| a.line as f64).sum::<f64>() / all_recruits.max(1.0);
    // The states' football culture, ranked.
    let mut cultures: Vec<f64> = w.states.iter().map(|s| s.culture).collect();
    cultures.sort_by(f64::total_cmp);
    let top = cultures.get(cultures.len().saturating_sub(10)).copied().unwrap_or(f64::MAX);
    let low = cultures.get(9).copied().unwrap_or(f64::MIN);
    f.iter()
        .enumerate()
        .map(|(k, a)| {
            let public = format!(
                "{} {}, {} in the biggest class. {} FBS {} in the last four classes.",
                a.schools,
                if a.schools == 1 { "school" } else { "schools" },
                a.big,
                a.signees,
                if a.signees == 1 { "signee" } else { "signees" }
            );
            let mut words: Vec<(f64, String)> = Vec::new();
            let talent =
                if per_school > 0.0 && a.schools > 0 { a.recruits as f64 / a.schools as f64 / per_school } else { 1.0 };
            if a.schools > 0 && talent >= 1.8 {
                words.push((talent, "talent everywhere: real players at most schools, every year".into()));
            } else if a.schools > 0 && talent <= 0.4 {
                words.push((
                    1.0 / talent.max(0.1),
                    "thin: a real player a year, if that, and you find him yourself".into(),
                ));
            }
            if a.programs >= 25 {
                words.push((a.programs as f64 / 12.0, "every school is recruited by everyone".into()));
            } else if a.signees >= 3 && a.programs <= 4 {
                words.push((2.0, "a few programs have it to themselves".into()));
            }
            if a.recruits >= 5 {
                let speed = a.speed as f64 / a.recruits as f64 - speed_all;
                let line = a.line as f64 / a.recruits as f64 - line_all;
                if speed >= 0.08 && speed >= line {
                    words.push((1.0 + speed * 10.0, "speed everywhere".into()));
                } else if line >= 0.08 {
                    words.push((1.0 + line * 10.0, "big linemen".into()));
                }
            }
            if a.schools > 0 && a.enrollment < 500.0 {
                words.push((1.6, "small towns and small schools, where the coach decides who gets seen".into()));
            } else if a.enrollment > 1800.0 {
                words.push((1.4, "big schools with big staffs and film on everyone".into()));
            }
            if a.schools >= 3 && a.money >= 0.62 && a.private as f64 >= 0.2 * a.schools as f64 {
                words.push((1.3, "money and private schools: families with a say".into()));
            } else if a.schools >= 3 && a.money <= 0.35 {
                words.push((1.3, "working towns, where a scholarship changes a family".into()));
            }
            let culture = w.states.get(area(k as u16).state).map_or(0.0, |s| s.culture);
            if culture >= top {
                words.push((1.2, "a football state: Friday night is everything".into()));
            } else if culture <= low {
                words.push((1.2, "football isn't the first sport here".into()));
            }
            words.sort_by(|x, y| y.0.total_cmp(&x.0));
            Character { public, words: words.into_iter().take(3).map(|(_, s)| s).collect() }
        })
        .collect()
}

/// One pin on the map (GR6).
#[derive(Clone, Debug, PartialEq)]
pub struct Pin {
    pub school: usize,
    pub name: String,
    /// His high school class.
    pub class_of: i32,
    pub position: Position,
    pub stars: u8,
    /// What he did, in words ("round 1, 2019, Atlanta"; "All-American 2021").
    pub what: String,
    /// The player's id, for links, when known.
    pub player: Option<u32>,
}

/// A rival's pipeline into an area, drawn as a line to its campus (GR6).
#[derive(Clone, Debug, PartialEq)]
pub struct Pipeline {
    pub program: usize,
    /// 0-1: the strongest of its pipelines to the area's counties and schools.
    pub strength: f64,
    pub campus: (f64, f64),
}

/// An area's history pins (GR6). Your finds join them when finds are built
/// (lane G4).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pins {
    /// Kids from the area who signed with the program asked about, newest first.
    pub signees: Vec<Pin>,
    /// The area's legends from the world's years: high draft picks,
    /// All-Americans and the Hall of Fame, the most famous first (up to 12).
    pub legends: Vec<Pin>,
    /// The programs with the strongest pipelines into it (up to 6).
    pub pipelines: Vec<Pipeline>,
}

/// How many legends and pipelines an area shows.
pub const LEGENDS: usize = 12;
pub const PIPELINES: usize = 6;

/// Every area's pins, for `program`'s signees (none for no program).
pub fn pins(w: &World, program: Option<usize>) -> Vec<Pins> {
    let n = table().len();
    let mut out = vec![Pins::default(); n];
    let area_of = |hs: usize| w.high_schools.get(hs).map(|s| s.ground_area as usize);
    // Signees: people still in the world and those who have left it.
    if let Some(prog) = program {
        for p in &w.people {
            if p.stage != Stage::HighSchool && p.path.first() == Some(&prog) {
                if let Some(k) = area_of(p.hs) {
                    out[k].signees.push(Pin {
                        school: p.hs,
                        name: p.name.clone(),
                        class_of: p.class_of,
                        position: p.player.position,
                        stars: p.stars,
                        what: format!("signed {}", p.class_of),
                        player: Some(p.player.id),
                    });
                }
            }
        }
        for a in &w.alumni {
            if a.colleges.first() == Some(&prog) {
                if let Some(k) = area_of(a.high_school) {
                    out[k].signees.push(Pin {
                        school: a.high_school,
                        name: a.name.clone(),
                        class_of: a.class_of,
                        position: a.position,
                        stars: a.stars,
                        what: format!("signed {}", a.class_of),
                        player: a.player.as_ref().map(|p| p.id),
                    });
                }
            }
        }
    }
    // Legends: draft picks in the first three rounds, All-Americans and the
    // Hall, each with a fame score.
    let honors = &w.stats.honors;
    let mut legends: Vec<Vec<(f64, Pin)>> = vec![Vec::new(); n];
    for a in &w.alumni {
        let Some(k) = area_of(a.high_school) else { continue };
        let id = a.player.as_ref().map(|p| p.id);
        let h = id.and_then(|i| honors.get(&i));
        let aa = h.map_or(0, |h| h.all_america.len());
        let hall = h.and_then(|h| h.inducted);
        let round = a.pick.map(|p| (p.saturating_sub(1) / 32 + 1).min(7));
        let high = round.is_some_and(|r| r <= 3);
        if !high && aa == 0 && hall.is_none() {
            continue;
        }
        let mut what = Vec::new();
        let mut fame = 0.0;
        if let (Some(r), Some(pick)) = (round, a.pick) {
            let team = a.nfl_team.as_deref().map(|t| format!(", {t}")).unwrap_or_default();
            what.push(format!("round {r}, {}{team}", a.year));
            fame += (8 - r) as f64 + if pick <= 10 { 3.0 } else { 0.0 };
        }
        if aa > 0 {
            what.push(if aa == 1 { "All-American".to_string() } else { format!("All-American {aa} times") });
            fame += 3.0 * aa as f64;
        }
        if let Some(y) = hall {
            what.push(format!("Hall of Fame {y}"));
            fame += 12.0;
        }
        legends[k].push((
            fame,
            Pin {
                school: a.high_school,
                name: a.name.clone(),
                class_of: a.class_of,
                position: a.position,
                stars: a.stars,
                what: what.join("; "),
                player: id,
            },
        ));
    }
    // Pipelines: each program's strongest tie to the area's counties and
    // schools.
    let mut county_area: HashMap<usize, usize> = HashMap::new();
    for s in &w.high_schools {
        county_area.insert(s.area, s.ground_area as usize);
    }
    let mut lines: Vec<HashMap<usize, f64>> = vec![HashMap::new(); n];
    for (pid, p) in w.programs.iter().enumerate() {
        let ties = p
            .area_pipeline
            .iter()
            .filter_map(|(c, v)| county_area.get(c).map(|&k| (k, v.0)))
            .chain(p.hs_pipeline.iter().filter_map(|(h, v)| area_of(*h).map(|k| (k, v.0))));
        for (k, v) in ties {
            let e = lines[k].entry(pid).or_insert(0.0);
            *e = e.max(v);
        }
    }
    for (k, o) in out.iter_mut().enumerate() {
        o.signees.sort_by(|a, b| b.class_of.cmp(&a.class_of).then(a.name.cmp(&b.name)));
        let mut l = std::mem::take(&mut legends[k]);
        l.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.class_of.cmp(&b.1.class_of)).then(a.1.name.cmp(&b.1.name)));
        o.legends = l.into_iter().take(LEGENDS).map(|(_, p)| p).collect();
        let mut p: Vec<(usize, f64)> = lines[k].iter().map(|(&a, &b)| (a, b)).filter(|x| x.1 >= 0.05).collect();
        p.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        o.pipelines = p
            .into_iter()
            .take(PIPELINES)
            .map(|(program, strength)| Pipeline { program, strength, campus: campus(w, program) })
            .collect();
    }
    out
}
