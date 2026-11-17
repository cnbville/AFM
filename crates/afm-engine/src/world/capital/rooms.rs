//! Capital lane C2: the rooms and the campus (`docs/design/capital.md`,
//! items CP17-CP19, CP21-CP23 and CP26, and campus life in recruiting).
//!
//! Every college program has thirteen rooms (`Room`, the spec's CP17
//! table in order), each with a level from 0 (none) to 5. A room's level
//! is what it was built to, less a tenth of a level every year since
//! (CP22: about a level every ten years unless renovated). Each room
//! carries its own tool, and the rooms share out what the one facilities
//! number did (CP19): growth in the weight room, nutrition, sports science
//! and recovery for the body, film and the indoor field for the rest;
//! the academic center for grades; the film rooms for film study. The old
//! number stays as the trade's read, worked out from the rooms (`number`).
//!
//! Through the world's past the number is the one the facilities lane
//! built (real buildings for FBS, the seat-based model for the rest), and
//! the rooms follow it: their shape comes from what each real building
//! held (its kind and the data's note), scaled so the rooms give the same
//! number (`fit`). From the season the world hands over, the rooms live:
//! they age, projects raise them (`open`), the ADs renovate the room
//! furthest behind (`renovations`, CP18), and the number is read from them.
//!
//! What a recruit sees of a room is read against the best in his
//! conference (CP23, `Campus::read`); what a body gets from it is the room
//! itself. Each tool that is new with this lane (visits, injuries, the
//! days back, the install, NIL, the room's mood, nights out, the pipeline
//! pitch, the atmosphere, body plans) is read around the division's
//! typical room, so the level's totals stay where they were. Every size
//! here is proposed, not confirmed, unless the spec gives it.

use serde::{Deserialize, Serialize};

use crate::rng::clamp;
use crate::world::World;
use crate::world::buildings::{self, Building, Kind, TeamRooms};
use crate::world::data::Division;
use crate::world::program::Program;
use crate::world::season::key_uniform;

/// How many rooms a program has.
pub const ROOMS: usize = 13;

/// A tenth of a level a year (CP22: a level about every ten years).
pub const AGE_A_YEAR: f32 = 0.1;

/// The rooms, in the spec's CP17 order (lane C1's donors store a room
/// want as this index).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum Room {
    Locker,
    Weight,
    SportsScience,
    Recovery,
    Nutrition,
    Film,
    IndoorField,
    RecruitingLounge,
    PlayersLounge,
    Academic,
    ContentStudio,
    TrophyHall,
    Stadium,
}

impl Room {
    pub const ALL: [Room; ROOMS] = [
        Room::Locker,
        Room::Weight,
        Room::SportsScience,
        Room::Recovery,
        Room::Nutrition,
        Room::Film,
        Room::IndoorField,
        Room::RecruitingLounge,
        Room::PlayersLounge,
        Room::Academic,
        Room::ContentStudio,
        Room::TrophyHall,
        Room::Stadium,
    ];

    pub fn from_index(k: u8) -> Option<Room> {
        Room::ALL.get(k as usize).copied()
    }

    fn bit(self) -> u16 {
        1 << self as u16
    }

    /// The room's name on screen.
    pub fn words(self) -> &'static str {
        match self {
            Room::Locker => "locker room",
            Room::Weight => "weight room",
            Room::SportsScience => "sports science lab",
            Room::Recovery => "recovery center",
            Room::Nutrition => "nutrition center",
            Room::Film => "film theater and position rooms",
            Room::IndoorField => "indoor field",
            Room::RecruitingLounge => "recruiting lounge",
            Room::PlayersLounge => "players' lounge",
            Room::Academic => "academic center",
            Room::ContentStudio => "content studio",
            Room::TrophyHall => "trophy hall and NFL wall",
            Room::Stadium => "stadium",
        }
    }

    /// What the room unlocks (CP17).
    pub fn unlocks(self) -> &'static str {
        match self {
            Room::Locker => "Visits count for more, the team's mood lifts",
            Room::Weight => "Faster strength and body growth",
            Room::SportsScience => "Your staff sees each player's load and injury risk, fewer soft-tissue injuries",
            Room::Recovery => "Hurt players back a few days sooner",
            Room::Nutrition => "Body plans move weight faster",
            Room::Film => "Faster tape tags, playbooks learned faster",
            Room::IndoorField => "Practice in bad weather",
            Room::RecruitingLounge => "Game-day visits count more",
            Room::PlayersLounge => "Better mood, fewer late nights out",
            Room::Academic => "Better grades and eligibility",
            Room::ContentStudio => "Better NIL deals for your players",
            Room::TrophyHall => "A stronger pro pipeline and tradition pitch; a plaque for each first-round pick",
            Room::Stadium => "Atmosphere and the crowd",
        }
    }

    /// The room's share of the old facilities number (CP19: the rooms
    /// share it out; proposed, not confirmed). They sum to 1.
    pub fn weight(self) -> f64 {
        match self {
            Room::Locker => 0.10,
            Room::Weight => 0.14,
            Room::SportsScience => 0.06,
            Room::Recovery => 0.07,
            Room::Nutrition => 0.06,
            Room::Film => 0.12,
            Room::IndoorField => 0.10,
            Room::RecruitingLounge => 0.06,
            Room::PlayersLounge => 0.05,
            Room::Academic => 0.08,
            Room::ContentStudio => 0.03,
            Room::TrophyHall => 0.05,
            Room::Stadium => 0.08,
        }
    }

    /// Every program has one (a locker room, a weight room, meeting rooms,
    /// a training room, academic support, a stadium); the rest a program
    /// may not have at all.
    pub fn everyday(self) -> bool {
        matches!(self, Room::Locker | Room::Weight | Room::Recovery | Room::Film | Room::Academic | Room::Stadium)
    }
}

/// A building on the campus map (CP21: a stylized layout, not real
/// geography).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Hall {
    pub name: String,
    pub opened: i32,
    /// The rooms it holds, a bit per `Room`.
    pub rooms: u16,
}

impl Hall {
    pub fn holds(&self, r: Room) -> bool {
        self.rooms & r.bit() != 0
    }
}

/// A program's rooms and campus (on `Program.campus`; old saves fill it
/// on load, `after_load`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Campus {
    /// Each room's level as it stands, 0 (none) to 5.
    #[serde(default)]
    pub level: [f32; ROOMS],
    /// The season each room was last built or renovated (0: before the
    /// world's records).
    #[serde(default)]
    pub since: [i32; ROOMS],
    /// Each room's level read against the best in the conference (CP23):
    /// the best reads 5 and every room a level behind it reads a level
    /// less, never under 1 (0: no room).
    #[serde(default)]
    pub read: [f32; ROOMS],
    /// The division's typical room this year, as a value (level / 5) and
    /// as a read: the new tools are read around it.
    #[serde(default)]
    pub typical: [f32; ROOMS],
    #[serde(default)]
    pub typical_read: [f32; ROOMS],
    /// Rooms the team works without while they are rebuilt (CP26), a bit
    /// per `Room`.
    #[serde(default)]
    pub temporary: u16,
    /// The rooms run on their own (from the season the world hands over).
    #[serde(default)]
    pub live: bool,
    /// Whether the rooms have been set at all.
    #[serde(default)]
    pub set: bool,
    /// The campus map: its buildings and its open plots.
    #[serde(default)]
    pub halls: Vec<Hall>,
    #[serde(default)]
    pub open_plots: u8,
}

impl Campus {
    /// A room's level as it stands.
    pub fn level(&self, r: Room) -> f64 {
        self.level[r as usize] as f64
    }

    /// The level shown (1-5, 0 for none): the read against the conference.
    pub fn shown(&self, r: Room) -> u8 {
        let x = self.read[r as usize];
        if self.level[r as usize] <= 0.0 { 0 } else { x.round().clamp(1.0, 5.0) as u8 }
    }

    /// The old facilities number from the rooms (CP19).
    pub fn number(&self) -> f64 {
        clamp(Room::ALL.iter().map(|&r| r.weight() * self.level(r) / 5.0).sum::<f64>(), 0.02, 1.0)
    }

    /// What a room gives its tool: its level as a value (0-1, on the old
    /// number's scale), a level less while the team works in temporary
    /// space (CP26).
    pub fn value(&self, r: Room) -> f64 {
        let l = self.level(r);
        let temp = if self.temporary & r.bit() != 0 { 1.0 } else { 0.0 };
        ((l - temp).max(0.0) / 5.0).min(1.0)
    }

    /// A room's value against its division's typical one.
    fn over(&self, r: Room) -> f64 {
        self.value(r) - self.typical[r as usize] as f64
    }

    /// A room's read (0-5) against the typical read, in levels.
    fn read_over(&self, r: Room) -> f64 {
        let temp = if self.temporary & r.bit() != 0 { 1.0 } else { 0.0 };
        (self.read[r as usize] as f64 - temp).max(0.0) - self.typical_read[r as usize] as f64
    }

    /// Whether the team works in temporary space this year (CP26).
    pub fn in_temporary_space(&self) -> bool {
        self.temporary != 0
    }
}

// ---------------------------------------------------------------------------
// The rooms' share of today's effects (CP19)
// ---------------------------------------------------------------------------

/// The body's part of college coaching (the weight room, nutrition,
/// sports science and recovery), on the old number's 0-1 scale; it is the
/// number itself when every room stands at it.
pub fn physical(p: &Program) -> f64 {
    let c = &p.campus;
    0.55 * c.value(Room::Weight)
        + 0.2 * c.value(Room::Nutrition)
        + 0.15 * c.value(Room::SportsScience)
        + 0.1 * c.value(Room::Recovery)
}

/// The rest of college coaching: the film theater and position rooms, the
/// indoor field's year of reps and the recovery center's days back.
pub fn teaching(p: &Program) -> f64 {
    let c = &p.campus;
    0.5 * c.value(Room::Film) + 0.3 * c.value(Room::IndoorField) + 0.2 * c.value(Room::Recovery)
}

/// The academic center, where the number stood for academic support.
pub fn academic(p: &Program) -> f64 {
    p.campus.value(Room::Academic)
}

/// The film rooms, where the number stood for film study.
pub fn film(p: &Program) -> f64 {
    p.campus.value(Room::Film)
}

// ---------------------------------------------------------------------------
// The new tools (CP17), read around the division's typical room
// ---------------------------------------------------------------------------

/// How much more an official visit counts (CP17, the locker room, read
/// against the conference): about a fifth more for the conference's best
/// against a typical one (proposed, not confirmed).
pub fn visit_lift(p: &Program) -> f64 {
    clamp(0.1 * p.campus.read_over(Room::Locker), -0.25, 0.25)
}

/// How much more a game-day visit counts (CP17, the recruiting lounge,
/// read against the conference; proposed, not confirmed).
pub fn game_day_lift(p: &Program) -> f64 {
    clamp(0.15 * p.campus.read_over(Room::RecruitingLounge), -0.35, 0.35)
}

/// The sports science lab's guard on soft-tissue injuries, a factor on
/// their share of the game's injury mix like the strength staff's
/// (`game::sim::strength_guard`): about 11% fewer for the best lab against
/// a typical one (proposed, not confirmed).
pub fn soft_tissue_guard(p: &Program) -> f64 {
    clamp(1.0 - 0.3 * p.campus.over(Room::SportsScience), 0.85, 1.15)
}

/// Whether your staff sees each player's load and injury risk (CP17: a
/// sports science lab at level 3 or more; proposed, not confirmed).
pub fn sees_load(p: &Program) -> bool {
    p.campus.level(Room::SportsScience) >= 2.5
}

/// The recovery center's days back after a game injury, in the medical
/// staff's units (`training::seat::game_injuries`, where each unit is 12%
/// of the time out): about 9% sooner for the best center against a typical
/// one, "a few days" on a three-week injury (proposed, not confirmed).
pub fn recovery_days(p: &Program) -> f64 {
    clamp(2.0 * p.campus.over(Room::Recovery), -1.0, 1.0)
}

/// The film rooms' part of the install: a weekly plan a little fuller for
/// the best rooms (CP17, playbooks learned faster; proposed, not
/// confirmed).
pub fn install_lift(p: &Program) -> f64 {
    clamp(0.06 * p.campus.over(Room::Film), -0.03, 0.03)
}

/// A factor on a player's brand money (CP17, the content studio: about a
/// fifth more for the best studio against none; proposed, not confirmed).
pub fn nil_lift(p: &Program) -> f64 {
    clamp(1.0 + 0.25 * p.campus.over(Room::ContentStudio), 0.85, 1.25)
}

/// What the locker room and the players' lounge add to the room's mood
/// (CP17), less the build year's temporary space (CP26; proposed, not
/// confirmed).
pub fn mood_lift(p: &Program) -> f64 {
    let c = &p.campus;
    let home = 0.12 * c.over(Room::Locker) + 0.08 * c.over(Room::PlayersLounge);
    let temp = if c.in_temporary_space() { TEMPORARY_MOOD } else { 0.0 };
    clamp(home - temp, -0.15, 0.1)
}

/// The build year's cost to the room's mood (CP26; proposed, not
/// confirmed).
pub const TEMPORARY_MOOD: f64 = 0.04;

/// A factor on the chance of off-field trouble: campus life adds a little
/// (also in scope, CP1) and the players' lounge keeps men in at night
/// (CP17). `campus_life` is the program's campus life grade, 0-1 (55-99
/// within the division); proposed, not confirmed: about 5% either way
/// for each.
pub fn trouble_factor(p: &Program, campus_life: f64) -> f64 {
    let life = 0.25 * (campus_life - 0.77);
    let lounge = -0.2 * p.campus.over(Room::PlayersLounge);
    clamp(1.0 + life + lounge, 0.85, 1.15)
}

/// A factor on the pro pipeline and tradition reads behind the recruit's
/// grades (CP17, the trophy hall and NFL wall, read against the
/// conference; proposed, not confirmed).
pub fn pitch_lift(p: &Program) -> f64 {
    clamp(1.0 + 0.03 * p.campus.read_over(Room::TrophyHall), 0.9, 1.1)
}

/// A factor on the stadium atmosphere read behind the recruit's grade
/// (CP17, the stadium, read against the conference; proposed, not
/// confirmed).
pub fn atmosphere_lift(p: &Program) -> f64 {
    clamp(1.0 + 0.03 * p.campus.read_over(Room::Stadium), 0.9, 1.1)
}

/// A factor on a body plan's weekly rate toward its target (CP17, the
/// nutrition center; proposed, not confirmed).
pub fn body_plan_rate(p: &Program) -> f64 {
    clamp(1.0 + 0.4 * p.campus.over(Room::Nutrition), 0.85, 1.2)
}

/// Whether the program has an indoor field (CP17; built as
/// `training::seat::indoor_field`).
pub fn has_indoor(p: &Program) -> bool {
    p.campus.level(Room::IndoorField) > 0.0
}

/// How much a recruit weighs campus life (also in scope, CP1): most a
/// little, some a lot, the less professional kid more. A keyed draw on his
/// own stream, so nothing else in the world moves (proposed, not
/// confirmed: 0.02 to about 0.2, a median near 0.04, against a want's
/// usual 0.17).
pub fn campus_life_want(p: &crate::world::Person) -> f64 {
    let u = key_uniform(0xCA4F_11FE, p.player.id as u64);
    let pro = p.player.personality.traits.z(crate::player::Trait::Professionalism);
    clamp(0.02 + 0.14 * u.powi(3) - 0.02 * pro, 0.01, 0.22)
}

/// The NFL wall (CP17): every first-round pick the program sent to the
/// draft, newest first, as (draft year, player id, name), once it has a
/// trophy hall.
pub fn nfl_wall(w: &World, prog: usize) -> Vec<(i32, Option<u32>, String)> {
    let Some(p) = w.programs.get(prog) else { return Vec::new() };
    if p.campus.level(Room::TrophyHall) <= 0.0 {
        return Vec::new();
    }
    let ix = crate::world::folk::claims::Alumni::get(w);
    ix.of(prog).filter(|a| a.round == 1).map(|a| (a.year, a.id, a.name.clone())).collect()
}

// ---------------------------------------------------------------------------
// What a building holds
// ---------------------------------------------------------------------------

fn has(text: &str, words: &[&str]) -> bool {
    words.iter().any(|w| text.contains(w))
}

/// The rooms a real project built: its kind's usual rooms and what its
/// name and the data's note say it held (proposed, not confirmed: the
/// usual rooms by kind and era).
pub fn rooms_of(b: &Building) -> u16 {
    let text = format!("{} {}", b.project, b.note).to_lowercase();
    let mut m = 0u16;
    let era = b.opened;
    match b.kind {
        Kind::FootballBuilding => {
            for r in [Room::Locker, Room::Weight, Room::Recovery, Room::Film, Room::PlayersLounge, Room::TrophyHall] {
                m |= r.bit();
            }
            if era >= 2000 {
                m |= Room::RecruitingLounge.bit();
            }
            if era >= 2012 {
                m |= Room::Nutrition.bit();
            }
            if era >= 2015 {
                m |= Room::SportsScience.bit();
            }
        }
        Kind::IndoorPractice => m |= Room::IndoorField.bit(),
        Kind::SharedAthletics => {
            m |= Room::Weight.bit();
            m |= Room::Recovery.bit();
        }
        Kind::Stadium => {
            m |= Room::Stadium.bit();
            match b.rooms {
                TeamRooms::Home => {
                    for r in [Room::Locker, Room::Weight, Room::Film, Room::Recovery] {
                        m |= r.bit();
                    }
                }
                TeamRooms::Rooms => m |= Room::Locker.bit(),
                TeamRooms::None => return m,
            }
        }
        Kind::Renovation | Kind::OutdoorPractice => {}
    }
    let keys: [(Room, &[&str]); 11] = [
        (Room::Locker, &["locker"]),
        (Room::Weight, &["weight room", "weight-room", "strength and conditioning", "strength center"]),
        (
            Room::SportsScience,
            &["sports science", "sport science", "performance science", "biomechan", "motion capture"],
        ),
        (
            Room::Recovery,
            &["hydrotherapy", "recovery", "training room", "athletic training", "sports medicine", "cryo", "rehab"],
        ),
        (Room::Nutrition, &["nutrition", "fueling", "dining", "cafeteria", "kitchen"]),
        (Room::Film, &["meeting room", "film", "theater", "theatre", "auditorium", "position room", "team room"]),
        (Room::RecruitingLounge, &["recruit"]),
        (Room::PlayersLounge, &["lounge", "barber", "game room"]),
        (Room::Academic, &["academic", "study hall", "tutor"]),
        (Room::ContentStudio, &["studio", "content creation", "broadcast"]),
        (Room::TrophyHall, &["hall of fame", "trophy", "heritage", "museum"]),
    ];
    for (r, words) in keys {
        if has(&text, words) {
            m |= r.bit();
        }
    }
    if b.kind != Kind::IndoorPractice && has(&text, &["indoor practice", "indoor field"]) && b.kind != Kind::Stadium {
        m |= Room::IndoorField.bit();
    }
    if m == 0 && b.kind == Kind::Renovation {
        m |= Room::Locker.bit();
        m |= Room::Weight.bit();
    }
    m
}

/// The level a project's rooms open at, from what each room got of its
/// money in 2024 dollars (proposed, not confirmed: $4M a room reads 3, each
/// doubling about another 0.8 of a level, 5 at about $25M a room).
pub fn built_level(per_room_2024: f64) -> f32 {
    clamp(3.0 + (per_room_2024.max(0.1) / 4.0e6).log2() / 1.2, 1.5, 5.0) as f32
}

fn spread(mask: u16, money_2024: f64) -> [f32; ROOMS] {
    let total: f64 = Room::ALL.iter().filter(|r| mask & r.bit() != 0).map(|r| r.weight()).sum();
    let mut out = [0.0f32; ROOMS];
    for r in Room::ALL {
        if mask & r.bit() != 0 && total > 0.0 {
            out[r as usize] = built_level(money_2024 * r.weight() / total);
        }
    }
    out
}

/// A real project's rooms as they open, in 2024 dollars.
fn real_levels(b: &Building) -> [f32; ROOMS] {
    let mask = rooms_of(b);
    let team = b.value_2024 * 1.0e6 * buildings::share(b.kind, b.rooms).max(0.15);
    let mut out = spread(mask & !Room::Stadium.bit(), team);
    if mask & Room::Stadium.bit() != 0 {
        out[Room::Stadium as usize] = built_level(b.value_2024 * 1.0e6 / 4.0);
    }
    out
}

/// The shape of a program's rooms in `season` (each room's level before
/// the fit to the number) and its buildings for the map.
fn shape(p: &Program, season: i32) -> ([f32; ROOMS], Vec<Hall>) {
    let mut lv = [0.0f32; ROOMS];
    for r in Room::ALL.iter().filter(|r| r.everyday()) {
        lv[*r as usize] = 1.5;
    }
    let mut halls = Vec::new();
    let real = buildings::of(&p.real_name);
    if real.is_empty() {
        // No buildings in the data: the everyday rooms, and the rest by
        // the level's usual share of programs that have one (proposed, not
        // confirmed), on its own keyed draw.
        let share = optional_share(p.division);
        for r in Room::ALL.iter().filter(|r| !r.everyday()) {
            if key_uniform(0xC2_0F7 ^ (*r as u64) << 8, p.id as u64) < share(*r) {
                lv[*r as usize] = 1.5;
            }
        }
        halls.push(Hall { name: "Stadium".into(), opened: 0, rooms: Room::Stadium.bit() });
        let mut office = 0u16;
        for r in Room::ALL {
            if lv[r as usize] > 0.0 && !matches!(r, Room::Stadium | Room::IndoorField) {
                office |= r.bit();
            }
        }
        halls.push(Hall { name: "Football offices".into(), opened: 0, rooms: office });
        if lv[Room::IndoorField as usize] > 0.0 {
            halls.push(Hall { name: "Indoor practice facility".into(), opened: 0, rooms: Room::IndoorField.bit() });
        }
        return (lv, halls);
    }
    for b in real.iter().filter(|b| b.opened <= season) {
        let age = (season - b.opened) as f32 * AGE_A_YEAR;
        let at = real_levels(b);
        for k in 0..ROOMS {
            if at[k] > 0.0 {
                lv[k] = lv[k].max((at[k] - age).max(1.0));
            }
        }
        if b.kind != Kind::Renovation {
            halls.push(Hall { name: b.project.clone(), opened: b.opened, rooms: rooms_of(b) });
        }
    }
    if !halls.iter().any(|h| h.holds(Room::Stadium)) {
        halls.insert(0, Hall { name: "Stadium".into(), opened: 0, rooms: Room::Stadium.bit() });
    }
    (lv, halls)
}

/// The share of programs at a level with a room beyond the everyday ones,
/// where the data has no buildings (proposed, not confirmed).
fn optional_share(d: Division) -> impl Fn(Room) -> f64 {
    let scale = match d {
        Division::FBS => 1.0,
        Division::FCS => 0.5,
        Division::D2 => 0.3,
        Division::NAIA => 0.2,
        Division::D3 | Division::JUCO => 0.15,
    };
    move |r| {
        scale
            * match r {
                Room::IndoorField => 0.5,
                Room::PlayersLounge => 0.6,
                Room::RecruitingLounge => 0.5,
                Room::TrophyHall => 0.6,
                Room::Nutrition => 0.4,
                Room::SportsScience => 0.2,
                Room::ContentStudio => 0.1,
                _ => 1.0,
            }
    }
}

/// Open plots on the map by level (CP21; proposed, not confirmed).
fn plots(p: &Program, season: i32) -> u8 {
    match p.division {
        Division::FBS if p.power_conference(season) => 3,
        Division::FBS | Division::FCS => 2,
        _ => 1,
    }
}

/// Scales a shape so the rooms give `number` (each room at most level 5),
/// keeping which rooms exist; when even every room at 5 falls short, the
/// missing rooms come in.
fn scale_to(shape: &[f32; ROOMS], number: f64) -> [f32; ROOMS] {
    let total = |c: f64, s: &[f32; ROOMS]| -> f64 {
        Room::ALL.iter().map(|&r| r.weight() * (c * s[r as usize] as f64).min(5.0) / 5.0).sum()
    };
    let mut s = *shape;
    if total(1.0e6, &s) < number - 1e-9 {
        for v in s.iter_mut().filter(|v| **v <= 0.0) {
            *v = 1.0;
        }
    }
    let (mut lo, mut hi) = (0.0f64, 1.0f64);
    while total(hi, &s) < number && hi < 1.0e6 {
        hi *= 2.0;
    }
    for _ in 0..60 {
        let mid = 0.5 * (lo + hi);
        if total(mid, &s) < number { lo = mid } else { hi = mid }
    }
    let c = 0.5 * (lo + hi);
    s.map(|v| (c * v as f64).min(5.0) as f32)
}

/// Sets a program's rooms to follow its facilities number in `season`
/// (the world's past): the shape from its buildings, scaled to the number.
pub fn fit(p: &mut Program, season: i32) {
    let (lv, halls) = shape(p, season);
    p.campus.level = scale_to(&lv, p.facilities);
    let real = buildings::of(&p.real_name);
    for r in Room::ALL {
        let k = r as usize;
        p.campus.since[k] = real
            .iter()
            .filter(|b| b.opened <= season && rooms_of(b) & r.bit() != 0)
            .map(|b| b.opened)
            .max()
            .unwrap_or(0);
    }
    p.campus.halls = halls;
    p.campus.open_plots = plots(p, season);
    p.campus.set = true;
}

/// Keeps the rooms and the number together. In the past the rooms follow
/// the number (`fit`); once the rooms live, a number someone moved by hand
/// (a tool, a test) moves every room in step, and otherwise the number is
/// read from the rooms.
pub fn sync_program(p: &mut Program, season: i32) {
    if !p.campus.set || !p.campus.live && (p.campus.number() - p.facilities).abs() > 1e-6 {
        let live = p.campus.live;
        fit(p, season);
        p.campus.live = live;
        if live {
            p.facilities = p.campus.number();
        }
        return;
    }
    if p.campus.live && (p.campus.number() - p.facilities).abs() > 1e-6 {
        let target = p.facilities;
        p.campus.level = scale_to(&p.campus.level, target);
        p.facilities = p.campus.number();
    }
}

/// Every program's rooms with its number.
pub fn sync(w: &mut World) {
    let season = w.year;
    for p in w.programs.iter_mut() {
        sync_program(p, season);
    }
}

/// Reads every program's rooms against its conference (CP23) and its
/// division's typical room, and marks the rooms being rebuilt (CP26).
pub fn read_all(w: &mut World) {
    let year = w.year;
    let n = w.programs.len();
    let mut best: std::collections::BTreeMap<(Division, String), [f32; ROOMS]> = Default::default();
    for p in w.programs.iter().filter(|p| p.plays(year)) {
        let b = best.entry((p.division, p.conference.clone())).or_insert([0.0; ROOMS]);
        for k in 0..ROOMS {
            b[k] = b[k].max(p.campus.level[k]);
        }
    }
    let mut temporary = vec![0u16; n];
    for x in w.finance.projects.iter() {
        if let crate::world::finance::Owner::College(p) = x.owner
            && p < n
            && !x.done
            && x.kind == crate::world::finance::ProjectKind::Facility
            && x.opens == year + 1
            && w.programs[p].campus.live
            && renovation_room(&x.name).is_none()
        {
            // The build year, the season before it opens: the team works
            // without the rooms it already has that the building moves
            // (a summer's renovation is done while the team is away).
            let mask = project_rooms(&w.programs[p], &x.name);
            temporary[p] |= mask & !(Room::Stadium.bit() | Room::IndoorField.bit()) & has_mask(&w.programs[p].campus);
        }
    }
    for (i, p) in w.programs.iter_mut().enumerate() {
        let b = best.get(&(p.division, p.conference.clone())).copied().unwrap_or([0.0; ROOMS]);
        for k in 0..ROOMS {
            let l = p.campus.level[k];
            p.campus.read[k] = if l <= 0.0 { 0.0 } else { (5.0 - (b[k] - l)).clamp(1.0, 5.0) };
        }
        p.campus.temporary = temporary[i];
    }
    let mut sum = [([0.0f64; ROOMS], [0.0f64; ROOMS], 0.0f64); crate::world::data::DIVISION_COUNT];
    for p in w.programs.iter().filter(|p| p.plays(year)) {
        let s = &mut sum[p.division as usize];
        for k in 0..ROOMS {
            s.0[k] += (p.campus.level[k] / 5.0) as f64;
            s.1[k] += p.campus.read[k] as f64;
        }
        s.2 += 1.0;
    }
    for p in w.programs.iter_mut() {
        let (v, r, c) = sum[p.division as usize];
        if c > 0.0 {
            p.campus.typical = v.map(|x| (x / c) as f32);
            p.campus.typical_read = r.map(|x| (x / c) as f32);
        }
    }
}

fn has_mask(c: &Campus) -> u16 {
    Room::ALL.iter().filter(|r| c.level[**r as usize] > 0.0).fold(0, |m, r| m | r.bit())
}

/// The rooms a project of the world's own builds, from its name: a real
/// project under way when the world hands over by what it held, the ADs'
/// kinds (`finance::college`, `facility_decisions`) by what such a building
/// holds today, and a renovation by its room.
pub fn project_rooms(p: &Program, name: &str) -> u16 {
    if let Some(b) = buildings::of(&p.real_name).iter().find(|b| b.project == name) {
        return rooms_of(b);
    }
    if let Some(r) = renovation_room(name) {
        return r.bit();
    }
    let n = name.to_lowercase();
    let set = |rs: &[Room]| rs.iter().fold(0u16, |m, r| m | r.bit());
    if n.contains("football building") {
        set(&[
            Room::Locker,
            Room::Weight,
            Room::SportsScience,
            Room::Recovery,
            Room::Nutrition,
            Room::Film,
            Room::RecruitingLounge,
            Room::PlayersLounge,
            Room::ContentStudio,
            Room::TrophyHall,
        ])
    } else if n.contains("operations center") {
        set(&[Room::Locker, Room::Weight, Room::Recovery, Room::Nutrition, Room::Film, Room::PlayersLounge])
    } else if n.contains("indoor") {
        Room::IndoorField.bit()
    } else if n.contains("weight room and locker room") {
        set(&[Room::Weight, Room::Locker])
    } else {
        set(&[Room::Locker, Room::Weight])
    }
}

/// A renovation's name ("weight room renovation").
pub fn renovation_name(r: Room) -> String {
    format!("{} renovation", r.words())
}

fn renovation_room(name: &str) -> Option<Room> {
    let base = name.strip_suffix(" renovation")?;
    Room::ALL.into_iter().find(|r| r.words() == base)
}

/// A project of the world's own opens (CP18): a renovation lifts its room
/// a level; a new building lifts its rooms to 4 or 5 by its money (each
/// room's share in 2024 dollars, `built_level`), and goes on the map.
pub fn open(w: &mut World, p: usize, name: &str, cost: f64, year: i32) {
    if !w.programs[p].campus.live {
        return;
    }
    let mask = project_rooms(&w.programs[p], name);
    let money = cost / crate::world::finance::data::college_index(year).max(0.05);
    let prog = &mut w.programs[p];
    let c = &mut prog.campus;
    if let Some(r) = renovation_room(name) {
        let k = r as usize;
        c.level[k] = (c.level[k].max(1.0) + 1.0).min(5.0);
        c.since[k] = year;
    } else {
        let levels = spread(mask, money);
        for r in Room::ALL.iter().filter(|r| mask & r.bit() != 0) {
            let k = *r as usize;
            c.level[k] = c.level[k].max(levels[k].clamp(4.0, 5.0));
            c.since[k] = year;
        }
        c.halls.push(Hall { name: name.to_string(), opened: year, rooms: mask });
        c.open_plots = c.open_plots.saturating_sub(1);
    }
    c.temporary &= !mask;
    prog.facilities = prog.campus.number();
}

/// What a project would add to the number when it opens (its `step` in
/// the books once the rooms live).
pub fn rise(p: &Program, name: &str, cost: f64, year: i32) -> f64 {
    let mut q = p.campus.clone();
    let mask = project_rooms(p, name);
    if let Some(r) = renovation_room(name) {
        let k = r as usize;
        q.level[k] = (q.level[k].max(1.0) + 1.0).min(5.0);
    } else {
        let levels = spread(mask, cost / crate::world::finance::data::college_index(year).max(0.05));
        for r in Room::ALL.iter().filter(|r| mask & r.bit() != 0) {
            let k = *r as usize;
            q.level[k] = q.level[k].max(levels[k].clamp(4.0, 5.0));
        }
    }
    (q.number() - p.campus.number()).max(0.0)
}

/// What a renovation costs (CP18: a fraction of a new building; proposed,
/// not confirmed: the room's share of its tier's building at two fifths,
/// so a Power 4 weight room about $9M in 2024 dollars).
pub fn renovation_cost(tier: u8, r: Room, year: i32) -> f64 {
    let base = match tier {
        0 => 140.0e6,
        1 => 40.0e6,
        2 => 14.0e6,
        _ => 3.0e6,
    };
    base * r.weight() / 0.85 * 0.4 * crate::world::finance::data::college_index(year)
}

/// The room a department would renovate first: the one furthest behind
/// where its money and fans would keep it (the ADs' target,
/// `finance::college`, as a level), weighted by the room's share.
pub fn most_behind(p: &Program, target: f64) -> Option<(Room, f64)> {
    let goal = 5.0 * target;
    Room::ALL
        .into_iter()
        .filter(|&r| p.campus.level(r) > 0.0)
        .map(|r| (r, goal - p.campus.level(r)))
        .filter(|x| x.1 > 0.0)
        .max_by(|a, b| (a.1 * a.0.weight()).total_cmp(&(b.1 * b.0.weight())).then(b.0.cmp(&a.0)))
}

/// The ADs' renovation chance for the room furthest behind, by how far
/// behind it is in levels (proposed, not confirmed: 15% at half a level,
/// 55% at two levels, at most 70%).
pub fn renovation_chance(behind: f64, reserves: f64) -> f64 {
    if behind < 0.5 {
        return 0.0;
    }
    let money = if reserves < 0.0 { 0.4 } else { 1.0 };
    clamp(0.02 + 0.265 * behind, 0.0, 0.7) * money
}

/// The start of July for the rooms: from the season the world hands over
/// they live, and every year after that each room ages a tenth of a level
/// (CP22), never below the level's everyday rooms, and the number is read
/// from them. In the world's past nothing happens here: the rooms follow
/// the number (`settle`).
pub fn age(w: &mut World) {
    let season = w.year;
    if season < w.options.start_year {
        return;
    }
    for p in w.programs.iter_mut() {
        if !p.campus.set {
            fit(p, season);
        }
        if !p.campus.live {
            p.campus.live = true;
            continue;
        }
        let base = (5.0 * crate::world::program::division_scale(p.division) * buildings::BASE) as f32;
        for k in 0..ROOMS {
            let l = p.campus.level[k];
            if l > 0.0 {
                p.campus.level[k] = (l - AGE_A_YEAR).max(base.min(l)).max(0.1);
            }
        }
        p.facilities = p.campus.number();
    }
}

/// The end of July, after the projects opened and the ADs decided: the
/// renovations once the rooms live (CP18), the rooms kept with the
/// number, and every room read against its conference (CP23).
pub fn settle(w: &mut World) {
    if w.year >= w.options.start_year {
        renovations(w);
    }
    sync(w);
    read_all(w);
}

/// Where a department's money and fans would keep its facilities, as the
/// ADs read it (`finance::college`, `facility_decisions`).
pub fn ad_target(p: &Program) -> f64 {
    let nil_share = clamp((p.collective.boosters() / 25.0e6).sqrt(), 0.0, 1.0);
    let scale = match p.division {
        Division::FBS => 1.0,
        Division::FCS => 0.55,
        Division::D2 => 0.35,
        _ => 0.25,
    };
    clamp(scale * (0.3 + 0.4 * p.fans) + 0.3 * nil_share, 0.0, 1.0)
}

/// After a load: an old save's rooms follow its numbers, then are read.
pub fn after_load(w: &mut World) {
    let need = w.programs.iter().any(|p| !p.campus.set);
    if need {
        let season = w.year;
        let start = w.options.start_year;
        for p in w.programs.iter_mut().filter(|p| !p.campus.set) {
            fit(p, season);
            p.campus.live = season >= start;
        }
        read_all(w);
    }
}

/// Your program's athletic director on renovating one room (CP18): he reads
/// the department's money and how far the room trails; yes means it is
/// done over the summer, a level up when next season opens.
pub fn ask_renovation(w: &mut World, p: usize, r: Room) -> Result<String, String> {
    use crate::world::finance::{Owner, Project, ProjectKind};
    let year = w.year;
    if p >= w.programs.len() || p >= w.finance.college.len() {
        return Err("No program.".into());
    }
    if w.finance.projects.iter().any(|x| x.owner == Owner::College(p) && !x.done) {
        return Err("A project is already under way.".into());
    }
    let prog = &w.programs[p];
    if prog.campus.level(r) <= 0.0 {
        return Err(format!("There is no {} to renovate: it comes with a new building.", r.words()));
    }
    if prog.campus.level(r) >= 4.95 {
        return Err(format!("The {} is as good as they come.", r.words()));
    }
    let renovated = w
        .finance
        .projects
        .iter()
        .any(|x| x.owner == Owner::College(p) && x.started == year && x.name.ends_with(" renovation"));
    if renovated {
        return Err("You renovated a room this year already.".into());
    }
    let tier = crate::world::finance::college::tier(w, p, year);
    let cost = renovation_cost(tier, r, year);
    let reserves = w.finance.college[p].reserves as f64;
    let behind = (prog.campus.read[r as usize] as f64 - 5.0).abs();
    let yes = clamp(0.35 + 0.15 * behind + clamp(reserves / cost.max(1.0) * 0.1, -0.3, 0.3), 0.05, 0.9);
    let ad = prog.ad.as_ref().map_or("The athletic director".to_string(), |a| a.name.clone());
    let draw = crate::world::finance::draw(w, 0xC2_A5, p as u64 ^ (r as u64) << 16, year);
    if draw >= yes {
        return Ok(format!("{ad}: \"Not this year.\" The department can't fit the {} into the budget.", r.words()));
    }
    let name = renovation_name(r);
    let step = rise(prog, &name, cost, year);
    w.finance.projects.push(Project {
        owner: Owner::College(p),
        kind: ProjectKind::Facility,
        name: name.clone(),
        cost: cost as f32,
        public: 0.0,
        pledged: 0.0,
        started: year,
        opens: year + 1,
        capacity: 0,
        step: step as f32,
        done: false,
        asked: true,
    });
    read_all(w);
    Ok(format!("{ad}: \"We'll do the {} this summer.\" It opens a level better next season.", r.words()))
}

/// The ADs' renovations each July once the rooms live (CP18, on the same
/// rules for every program): a department with no project under way does
/// over the room furthest behind, by how far behind it is and its money.
pub fn renovations(w: &mut World) {
    use crate::world::finance::{Owner, Project, ProjectKind};
    let year = w.year;
    let n = w.finance.college.len().min(w.programs.len());
    let busy: std::collections::HashSet<usize> = w
        .finance
        .projects
        .iter()
        .filter(|x| !x.done)
        .filter_map(|x| if let Owner::College(p) = x.owner { Some(p) } else { None })
        .collect();
    for p in 0..n {
        let prog = &w.programs[p];
        if busy.contains(&p) || !prog.plays(year) || !prog.campus.live {
            continue;
        }
        let Some((r, behind)) = most_behind(prog, ad_target(prog)) else { continue };
        let reserves = w.finance.college[p].reserves as f64;
        if crate::world::finance::draw(w, 0xC2_4E, p as u64, year) >= renovation_chance(behind, reserves) {
            continue;
        }
        let tier = crate::world::finance::college::tier(w, p, year);
        let cost = renovation_cost(tier, r, year);
        let name = renovation_name(r);
        let step = rise(prog, &name, cost, year);
        w.finance.projects.push(Project {
            owner: Owner::College(p),
            kind: ProjectKind::Facility,
            name,
            cost: cost as f32,
            public: 0.0,
            pledged: 0.0,
            started: year,
            opens: year + 1,
            capacity: 0,
            step: step as f32,
            done: false,
            asked: false,
        });
    }
}
