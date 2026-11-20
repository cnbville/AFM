//! Capital lane C6, high school (`docs/design/capital.md`, CP32-CP34).
//!
//! Every high school has five rooms (`HsRoom`): the weight room, the film
//! room, the locker room, the turf practice field and the stadium, each
//! with a level from 0 (none) to 5 that ages a tenth of a level a year
//! (CP22). Each carries its own tool, read around the high schools'
//! typical room so the level's totals stay where they were: the weight
//! room the body's coaching, the film room film study, the locker room the
//! room's mood, the turf field practice in the rain and the field's
//! upkeep, the stadium the crowd.
//!
//! The booster club pays for the shorter room list (CP33): racks and
//! platforms for the weight room, a locker room renovation, a meeting room
//! with film screens, a field house, a turf practice field. Your club works
//! from your wish list (`front_office::school`); an AI school's club takes
//! the room furthest behind on the same costs.
//!
//! The stadium is the district's (CP32): the school board calls a bond
//! election (in February for the first Saturday in May, in August for
//! November's general election, as Texas districts vote), and the town
//! votes. The season, the crowds, the tax bill and your work in town (a
//! lunch talk at the Rotary club, a night at the booster club's phone
//! bank, the district's community forum, each a slot in your week) move
//! the yes share; an AI coach does his own work on the same rule. A
//! district stadium is shared by every public school in the district, so
//! your crosstown rival gets it too (CP34).
//!
//! Nothing here runs before the rooms live (`capital::live`), so the
//! world's past stays the same per seed. Sizes are Claude's (proposed, not
//! confirmed).

use serde::{Deserialize, Serialize};

use crate::calendar::{Date, Event, Kind, Level, Loudness, Weekday};
use crate::rng::clamp;
use crate::world::World;
use crate::world::finance::data::hs_index;

/// How many rooms a high school has.
pub const HS_ROOMS: usize = 5;

/// A tenth of a level a year (CP22).
pub const AGE_A_YEAR: f32 = 0.1;

/// The high school rooms (CP33's list and the stadium).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum HsRoom {
    Weight,
    Film,
    Locker,
    Turf,
    Stadium,
}

impl HsRoom {
    pub const ALL: [HsRoom; HS_ROOMS] = [HsRoom::Weight, HsRoom::Film, HsRoom::Locker, HsRoom::Turf, HsRoom::Stadium];

    pub fn from_index(k: u8) -> Option<HsRoom> {
        HsRoom::ALL.get(k as usize).copied()
    }

    pub fn words(self) -> &'static str {
        match self {
            HsRoom::Weight => "weight room",
            HsRoom::Film => "film room",
            HsRoom::Locker => "locker room",
            HsRoom::Turf => "turf practice field",
            HsRoom::Stadium => "stadium",
        }
    }

    pub fn unlocks(self) -> &'static str {
        match self {
            HsRoom::Weight => "Faster strength and body growth",
            HsRoom::Film => "Better film study and game plans",
            HsRoom::Locker => "The team's mood lifts",
            HsRoom::Turf => "Practice through the rain, a cheaper field to keep",
            HsRoom::Stadium => "Bigger crowds and a bigger gate",
        }
    }

    /// What it means by key on the screens and in the calls.
    pub fn key(self) -> &'static str {
        match self {
            HsRoom::Weight => "weight",
            HsRoom::Film => "film",
            HsRoom::Locker => "locker",
            HsRoom::Turf => "turf",
            HsRoom::Stadium => "stadium",
        }
    }
}

/// One school's rooms (on `Capital.hs`, by school).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HsCampus {
    pub level: [f32; HS_ROOMS],
    /// The season each was last built or renovated (0: before the records).
    pub since: [i32; HS_ROOMS],
}

impl HsCampus {
    pub fn level(&self, r: HsRoom) -> f64 {
        self.level[r as usize] as f64
    }
    /// A room's value, 0-1.
    pub fn value(&self, r: HsRoom) -> f64 {
        (self.level(r) / 5.0).clamp(0.0, 1.0)
    }
    /// The level shown, 1-5 (0: none).
    pub fn shown(&self, r: HsRoom) -> u8 {
        let l = self.level[r as usize];
        if l <= 0.0 { 0 } else { l.round().clamp(1.0, 5.0) as u8 }
    }
}

// ---------------------------------------------------------------------------
// The rooms' tools, read around the typical room
// ---------------------------------------------------------------------------

/// A school's room against the high schools' typical one, as a value
/// (0 before the rooms live).
pub fn over(w: &World, h: usize, r: HsRoom) -> f64 {
    match w.capital.hs.get(h) {
        Some(c) if !w.capital.hs_off => clamp(c.value(r) - w.capital.hs_typical[r as usize] as f64, -0.6, 0.6),
        _ => 0.0,
    }
}

/// The same from the store alone (for the development step's split borrows).
pub fn over_in(cap: &super::Capital, h: usize, r: HsRoom) -> f64 {
    match cap.hs.get(h) {
        Some(c) if !cap.hs_off => clamp(c.value(r) - cap.hs_typical[r as usize] as f64, -0.6, 0.6),
        _ => 0.0,
    }
}

/// What the weight room adds to the body's coaching, in coaching points
/// (proposed, not confirmed: about 0.4 a level against the typical room,
/// the college weight room's slope).
pub const WEIGHT_COACHING: f64 = 1.8;

/// What the film room adds to film study, in points (proposed, not
/// confirmed: half the school budget's slope).
pub const FILM_STUDY: f64 = 3.5;

/// What the locker room adds to the room's mood (the college locker room's
/// slope).
pub const LOCKER_MOOD: f64 = 0.12;

/// The stadium's pull on the crowd (proposed, not confirmed: about 6% a
/// level against the typical stadium, 0.8-1.25).
pub const STADIUM_CROWD: f64 = 0.3;

/// What turf saves of the field's upkeep (proposed, not confirmed: no
/// mowing, watering or painting; fields still need grooming and
/// replacing).
pub const TURF_UPKEEP: f64 = 0.25;

/// The body's coaching at a school, around the typical weight room.
pub fn physical_lift(cap: &super::Capital, h: usize) -> f64 {
    WEIGHT_COACHING * over_in(cap, h, HsRoom::Weight)
}

/// Film study at a school, around the typical film room.
pub fn film_lift(w: &World, h: usize) -> f64 {
    FILM_STUDY * over(w, h, HsRoom::Film)
}

/// The room's mood from the locker room.
pub fn mood_lift(cap: &super::Capital, h: usize) -> f64 {
    clamp(LOCKER_MOOD * over_in(cap, h, HsRoom::Locker), -0.08, 0.08)
}

/// A factor on a school's home crowd from its stadium.
pub fn crowd_factor(w: &World, h: usize) -> f64 {
    clamp(1.0 + STADIUM_CROWD * over(w, h, HsRoom::Stadium), 0.8, 1.25)
}

/// Whether the school practices on turf (a turf field: the rain doesn't
/// close it).
pub fn has_turf(w: &World, h: usize) -> bool {
    !w.capital.hs_off && w.capital.hs.get(h).is_some_and(|c| c.level(HsRoom::Turf) > 0.0)
}

/// A factor on the field's upkeep line.
pub fn upkeep_factor(w: &World, h: usize) -> f64 {
    if has_turf(w, h) { 1.0 - TURF_UPKEEP } else { 1.0 }
}

// ---------------------------------------------------------------------------
// Starting levels
// ---------------------------------------------------------------------------

/// The usual crowd without the stadium's pull (the finance books' own).
fn base_crowd(w: &World, h: usize) -> f64 {
    let s = &w.high_schools[h];
    let culture = w.states[s.state].culture;
    let base = 350.0 + 0.9 * s.enrollment as f64;
    (base * (0.45 + 1.3 * s.prestige) * (1.0 + 0.35 * culture.clamp(-1.0, 1.5))).clamp(150.0, 14_000.0)
}

/// Where a school's money keeps its rooms: the level its booster club and
/// district aim for (proposed, not confirmed).
pub fn target(w: &World, h: usize, r: HsRoom) -> f64 {
    let s = &w.high_schools[h];
    match r {
        HsRoom::Stadium => {
            let c = base_crowd(w, h);
            clamp(1.0 + 4.0 * (c / 200.0).ln() / (12_000.0f64 / 200.0).ln(), 0.8, 5.0)
        }
        HsRoom::Turf => clamp(1.5 + 3.0 * s.resources, 0.0, 5.0),
        HsRoom::Locker => clamp(0.8 + 3.2 * s.resources + 0.4 * s.prestige, 0.5, 5.0),
        _ => clamp(0.8 + 3.4 * s.resources, 0.5, 5.0),
    }
}

/// The share of schools with a turf field, by the school (proposed, not
/// confirmed: about half of all high schools, nearly every big Texas
/// school, few small rural ones).
pub fn turf_chance(w: &World, h: usize) -> f64 {
    let s = &w.high_schools[h];
    let culture = w.states[s.state].culture.clamp(-1.0, 1.5);
    clamp(
        0.02 + 0.4 * s.resources + 0.3 * (s.enrollment as f64 / 2500.0).min(1.0) + 0.1 * culture + 0.1 * s.area_money
            - 0.1,
        0.03,
        0.95,
    )
}

fn draw_campus(w: &World, h: usize) -> HsCampus {
    let mut c = HsCampus::default();
    for r in HsRoom::ALL {
        let u = super::draw(w, 60 + r as u64, h as u64);
        let t = target(w, h, r);
        c.level[r as usize] = match r {
            HsRoom::Turf => {
                if u < turf_chance(w, h) {
                    // The carpet's age: a field lasts about ten years.
                    (1.0 + 4.0 * super::draw(w, 66, h as u64)) as f32
                } else {
                    0.0
                }
            }
            _ => clamp(t + 1.2 * (u - 0.5), 0.5, 5.0) as f32,
        };
    }
    c
}

/// The typical room this year: the mean value over every school.
fn typical(w: &mut World) {
    let n = w.capital.hs.len().max(1) as f64;
    let mut t = [0f32; HS_ROOMS];
    for r in HsRoom::ALL {
        t[r as usize] = (w.capital.hs.iter().map(|c| c.value(r)).sum::<f64>() / n) as f32;
    }
    w.capital.hs_typical = t;
}

/// Builds the rooms of every school not yet in the store (the first live
/// July, an older file, a school that opened since).
fn fill(w: &mut World) {
    let n = w.high_schools.len();
    let have = w.capital.hs.len();
    if have >= n {
        return;
    }
    let add: Vec<HsCampus> = (have..n).map(|h| draw_campus(w, h)).collect();
    w.capital.hs.extend(add);
    typical(w);
}

// ---------------------------------------------------------------------------
// The booster club's room list (CP33)
// ---------------------------------------------------------------------------

/// What a booster project does to the rooms: (room, levels; 5.0 with
/// `to` builds it to that level). The index is the school desk's
/// `PROJECTS` list; equipment (the speed gear, the video tower) is no
/// room.
pub fn project_rooms(k: usize) -> &'static [(HsRoom, f32, bool)] {
    match k {
        2 => &[(HsRoom::Weight, 1.0, false)],
        3 => &[(HsRoom::Locker, 1.0, false)],
        4 => &[(HsRoom::Film, 1.0, false)],
        5 => &[(HsRoom::Weight, 0.6, false), (HsRoom::Locker, 0.6, false)],
        6 => &[(HsRoom::Turf, 5.0, true)],
        _ => &[],
    }
}

/// The booster project for a room, by the desk's index.
fn project_for(r: HsRoom) -> usize {
    match r {
        HsRoom::Weight => 2,
        HsRoom::Locker => 3,
        HsRoom::Film => 4,
        _ => 6,
    }
}

/// A booster project done at a school: its rooms rise. Returns whether it
/// was a room (equipment is the school's resources, as before).
pub(crate) fn booster_project(w: &mut World, h: usize, k: usize) -> bool {
    let lifts = project_rooms(k);
    if lifts.is_empty() || !super::live(w) {
        return false;
    }
    fill(w);
    let year = w.year;
    let Some(c) = w.capital.hs.get_mut(h) else { return false };
    for &(r, x, to) in lifts {
        let l = &mut c.level[r as usize];
        *l = if to { l.max(x) } else { (*l + x).min(5.0) };
        c.since[r as usize] = year;
    }
    true
}

/// Each July, an AI school's booster club pays for the room furthest
/// behind where its money keeps it, on your costs (CP27): the chance grows
/// with how far behind and with what the club has to spend against the
/// project's price (proposed, not confirmed: two years' purchases buy it
/// for sure).
fn boosters(w: &mut World) {
    use crate::world::front_office::school::PROJECTS;
    let year = w.year;
    let seat = w.seat_school();
    let n = w.capital.hs.len().min(w.high_schools.len()).min(w.finance.hs.len());
    for h in 0..n {
        if seat == Some(h) {
            continue;
        }
        let c = &w.capital.hs[h];
        let Some((r, behind)) = [HsRoom::Weight, HsRoom::Film, HsRoom::Locker, HsRoom::Turf]
            .into_iter()
            .map(|r| (r, target(w, h, r) - c.level(r)))
            .filter(|x| x.1 >= 0.5)
            .max_by(|a, b| a.1.total_cmp(&b.1))
        else {
            continue;
        };
        let k = project_for(r);
        let cost = PROJECTS[k].1 * 1.03f64.powi(year - 2026);
        let money = w.finance.hs[h].budget.l[crate::world::finance::hs::PURCHASES_LINE] as f64;
        let chance = clamp(0.25 * behind, 0.0, 0.6) * clamp(2.0 * money / cost.max(1.0), 0.0, 1.0);
        if super::draw(w, 67, (h as u64) << 16 ^ year as u64) < chance {
            booster_project(w, h, k);
        }
    }
}

// ---------------------------------------------------------------------------
// Districts and bond night (CP32, CP34)
// ---------------------------------------------------------------------------

/// The most schools one district stadium serves (proposed, not confirmed:
/// Katy ISD's Legacy Stadium hosts nine; a big city's schools are split
/// among several districts and stadiums).
pub const DISTRICT_MAX: usize = 6;

/// A school's district: the public schools of its town in its state, a
/// big town's split into groups of `DISTRICT_MAX` by id. Private schools
/// are their own (no bonds; boosters only).
pub fn district(w: &World, h: usize) -> Vec<usize> {
    let s = &w.high_schools[h];
    if s.private {
        return vec![h];
    }
    let mut town: Vec<usize> = w.states[s.state]
        .schools
        .iter()
        .copied()
        .filter(|&o| o < w.high_schools.len())
        .filter(|&o| !w.high_schools[o].private && w.high_schools[o].town == s.town)
        .collect();
    town.sort_unstable();
    let at = town.iter().position(|&o| o == h).unwrap_or(0);
    let start = at / DISTRICT_MAX * DISTRICT_MAX;
    town[start..(start + DISTRICT_MAX).min(town.len())].to_vec()
}

/// Every district once, by its first school.
fn districts(w: &World) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    let mut seen = vec![false; w.high_schools.len()];
    for h in 0..w.high_schools.len() {
        if seen[h] || w.high_schools[h].private {
            continue;
        }
        let d = district(w, h);
        for &o in &d {
            seen[o] = true;
        }
        out.push(d);
    }
    out
}

/// What a bond builds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Package {
    /// A new stadium with a turf field and a field house under the stands.
    New,
    /// New stands, lights and the turf replaced.
    Renovation,
}

/// A district's bond election for its stadium.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bond {
    pub id: u32,
    /// The district's public schools (CP34: all of them share it).
    pub schools: Vec<u32>,
    pub package: Package,
    pub name: String,
    pub seats: u32,
    /// Dollars of the day (the books only; screens read it in words).
    pub cost: f64,
    pub called: Date,
    /// Election day.
    pub day: Date,
    /// The yes share the state's law asks.
    pub needs: f32,
    /// You asked the board for it.
    pub asked: bool,
    /// Your work in town (CP32): yes share points.
    pub work: f32,
    /// The yes share on election night.
    pub yes: Option<f32>,
    pub passed: bool,
    /// The season it opens, once it passed.
    pub opens: Option<Date>,
    pub opened: bool,
}

impl Bond {
    pub fn holds(&self, h: usize) -> bool {
        self.schools.contains(&(h as u32))
    }
    pub fn pending(&self) -> bool {
        self.yes.is_none()
    }
    pub fn building(&self) -> bool {
        self.passed && !self.opened
    }
}

/// One piece of your work in town before a vote (CP32).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Work {
    /// A lunch talk at the Rotary, Lions or Kiwanis club.
    Rotary,
    /// A night at the booster club's phone bank.
    PhoneBank,
    /// The district's community forum (informational: a district employee
    /// may explain a bond but not campaign for it on the job).
    Forum,
}

impl Work {
    pub fn from_key(k: &str) -> Option<Work> {
        match k {
            "rotary" => Some(Work::Rotary),
            "phone_bank" => Some(Work::PhoneBank),
            "forum" => Some(Work::Forum),
            _ => None,
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Work::Rotary => "rotary",
            Work::PhoneBank => "phone_bank",
            Work::Forum => "forum",
        }
    }
    pub fn words(self) -> &'static str {
        match self {
            Work::Rotary => "a lunch talk at the Rotary club",
            Work::PhoneBank => "a night at the booster club's phone bank",
            Work::Forum => "the district's community forum",
        }
    }
    /// Hours of your week, the yes share it adds, and how many times it
    /// counts (proposed, not confirmed: all of it about seven points).
    pub fn size(self) -> (f32, f32, u8) {
        match self {
            Work::Rotary => (2.0, 0.010, 3),
            Work::PhoneBank => (3.0, 0.008, 4),
            Work::Forum => (2.0, 0.012, 1),
        }
    }
}

/// The yes share the law asks (proposed, not confirmed: California 55%
/// since Proposition 39 in 2000, Idaho two-thirds, Oklahoma, Iowa and
/// Washington 60%, Missouri four-sevenths in a general election, else a
/// simple majority).
pub fn needs(code: &str) -> f32 {
    match code {
        "CA" => 0.55,
        "ID" => 0.667,
        "OK" | "IA" | "WA" => 0.6,
        "MO" => 0.5714,
        _ => 0.5,
    }
}

/// States whose districts don't vote on school bonds (Hawaii's schools are
/// one state district).
fn votes_in(code: &str) -> bool {
    code != "HI"
}

/// The weeks before the vote your work counts.
pub const WORK_DAYS: i32 = 70;

/// The board orders the election at least 78 days before (Texas Election
/// Code 3.005).
pub const ORDER_DAYS: i32 = 78;

/// Election day: the first Saturday in May, or the first Tuesday after the
/// first Monday in November.
pub fn election_days(year: i32) -> [Date; 2] {
    let may = Date::nth_weekday(year, 5, Weekday::Sat, 1);
    let nov = Date::nth_weekday(year, 11, Weekday::Mon, 1).add(1);
    [may, nov]
}

/// The next election day at least the order's days out.
pub fn next_election(today: Date) -> Date {
    let y = today.year();
    election_days(y)
        .into_iter()
        .chain(election_days(y + 1))
        .find(|d| today.days_to(*d) >= ORDER_DAYS)
        .unwrap_or_else(|| election_days(y + 1)[0])
}

/// A district's need for its stadium: how far its stadium trails the
/// typical one, as a value (0 or more).
fn stadium_need(w: &World, d: &[usize]) -> f64 {
    let t = w.capital.hs_typical[HsRoom::Stadium as usize] as f64;
    let mean = d.iter().filter_map(|&h| w.capital.hs.get(h)).map(|c| c.value(HsRoom::Stadium)).sum::<f64>()
        / d.len().max(1) as f64;
    let best = d.iter().map(|&h| target(w, h, HsRoom::Stadium) / 5.0).fold(0.0, f64::max);
    (0.5 * (t - mean) + 0.5 * (best - mean)).max(0.0)
}

/// A board's chance to call a bond in one window (proposed, not
/// confirmed: about 1-2% of districts a window, most of them renovations;
/// real districts rebuild a stadium about every 40 years and renovate it
/// between).
pub fn call_chance(w: &World, d: &[usize]) -> f64 {
    let need = stadium_need(w, d);
    let culture = w.states[w.high_schools[d[0]].state].culture.clamp(-1.0, 1.5);
    clamp(0.002 + 0.25 * need + 0.004 * culture, 0.0, 0.2)
}

/// What the district would ask the town for.
fn package_for(w: &World, d: &[usize]) -> (Package, u32, f64, String) {
    let year = w.year;
    let lead = d[0];
    let s = &w.high_schools[lead];
    let need = stadium_need(w, d);
    let enrollment: u32 = d.iter().map(|&h| w.high_schools[h].enrollment).sum();
    let crowd = d.iter().map(|&h| base_crowd(w, h)).fold(0.0, f64::max);
    let culture = w.states[s.state].culture;
    let new = need >= 0.25 && (enrollment >= 1500 || culture >= 0.5);
    let seats = (crowd * 1.5).clamp(2_000.0, 18_000.0);
    // About $3,300 a seat in 2012 (Allen ISD's 18,000-seat stadium for
    // $60M), growing with the high school index.
    let per = 3_300.0 * hs_index(year) / hs_index(2012).max(1e-9);
    let full = (seats * per).max(3.0e6 * hs_index(year) / hs_index(2012).max(1e-9));
    let code = &w.states[s.state].code;
    let name = if d.len() > 1 {
        if code == "TX" { format!("{} ISD Stadium", s.town) } else { format!("{} District Stadium", s.town) }
    } else {
        format!("{} Stadium", w.school_name(lead))
    };
    if new { (Package::New, seats as u32, full, name) } else { (Package::Renovation, seats as u32, 0.25 * full, name) }
}

fn busy(w: &World, d: &[usize]) -> bool {
    let year = w.year;
    w.capital.bonds.iter().any(|b| {
        d.iter().any(|&h| b.holds(h))
            && (b.pending()
                || b.building()
                || (b.passed && b.day.year() + 8 > year)
                || (!b.passed && b.day.year() + 1 > year))
    })
}

/// The board calls a bond: its election day is the next one at least 78
/// days out.
fn call(w: &mut World, d: &[usize], asked: bool) -> Option<u32> {
    let s = &w.high_schools[d[0]];
    let code = w.states[s.state].code.clone();
    if !votes_in(&code) {
        return None;
    }
    let (package, seats, cost, name) = package_for(w, d);
    let id = w.capital.id();
    let today = w.today;
    let day = next_election(today);
    let bond = Bond {
        id,
        schools: d.iter().map(|&h| h as u32).collect(),
        package,
        name,
        seats,
        cost,
        called: today,
        day,
        needs: needs(&code),
        asked,
        work: 0.0,
        yes: None,
        passed: false,
        opens: None,
        opened: false,
    };
    let text = match package {
        Package::New => format!(
            "The school board calls a bond election for {}: a new {}-seat stadium{}.",
            day.pretty(),
            seats,
            shared_words(w, d)
        ),
        Package::Renovation => format!(
            "The school board calls a bond election for {}: new stands, lights and turf for the stadium{}.",
            day.pretty(),
            shared_words(w, d)
        ),
    };
    w.capital.bonds.push(bond);
    say(w, d, Loudness::Big, text);
    Some(id)
}

/// ", shared with ..." for a district of several schools, as your school
/// reads it.
fn shared_words(w: &World, d: &[usize]) -> String {
    if d.len() < 2 {
        return String::new();
    }
    let me = w.seat_school();
    let others: Vec<&str> = d.iter().filter(|&&h| Some(h) != me).map(|&h| w.school_name(h)).collect();
    format!(", shared with {}", list(&others))
}

fn list(v: &[&str]) -> String {
    match v.len() {
        0 => String::new(),
        1 => v[0].to_string(),
        2 => format!("{} and {}", v[0], v[1]),
        n => format!("{}, and {}", v[..n - 1].join(", "), v[n - 1]),
    }
}

/// A news line for your district only.
fn say(w: &mut World, d: &[usize], loud: Loudness, text: String) {
    let Some(me) = w.seat_school() else { return };
    if !d.contains(&me) {
        return;
    }
    let e = Event::new(w.today, Level::HighSchool, Kind::News, loud, text).rule("capital").program(me);
    w.calendar.post(e);
}

/// The boards' windows (1 February for May, 1 August for November): each
/// district with no bond under way weighs one on its need.
fn calls(w: &mut World) {
    let year = w.year;
    for d in districts(w) {
        // Your district's board weighs its own need on the same rule as
        // every other (CP27); your ask comes on top.
        if busy(w, &d) {
            continue;
        }
        let chance = call_chance(w, &d);
        let key = (d[0] as u64) << 20 ^ (year as u64) << 4 ^ w.today.month() as u64;
        if super::draw(w, 68, key) < chance {
            call(w, &d, false);
        }
    }
}

/// An AI head coach's own work in town, in yes share points (CP27: on the
/// rule your work runs on; proposed, not confirmed: a sociable coach puts
/// in about what you can, a cold one little).
fn coach_work(w: &World, h: usize) -> f64 {
    let Some(c) = w.high_schools[h].coach.and_then(|c| w.coaches.get(c)) else { return 0.0 };
    let rel = c.ability().relationships;
    let u = super::draw(w, 69, h as u64 ^ (w.year as u64) << 20);
    0.07 * clamp((rel - 5.0) / 15.0, 0.0, 1.0) * (0.4 + 0.6 * u)
}

/// Where a district's expected yes share comes from, in shares (lane CS6
/// shows them as points): the town's starting mood, your seasons, the
/// crowds, the town's money and football, the tax bill and your work.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct YesParts {
    pub mood: f64,
    pub season: f64,
    pub crowds: f64,
    pub town: f64,
    pub tax: f64,
    pub work: f64,
    /// The district's win share and games behind `season`.
    pub win: f64,
    pub games: u32,
    pub title: bool,
}

impl YesParts {
    pub fn total(&self) -> f64 {
        self.mood + self.season + self.crowds + self.town + self.tax + self.work
    }
}

/// The yes share a district's vote expects before election night's luck
/// (CP32: the season, the crowds, the tax bill, the town's money and
/// football, the coach's work).
pub fn expected_yes(w: &World, b: &Bond) -> f64 {
    yes_parts(w, b).total()
}

/// `expected_yes` in its parts.
pub fn yes_parts(w: &World, b: &Bond) -> YesParts {
    let d: Vec<usize> = b.schools.iter().map(|&h| h as usize).collect();
    let lead = d[0];
    let s = &w.high_schools[lead];
    // November votes in the season: this season's record so far; May
    // votes: last season's.
    let record = |h: usize| -> (u32, u32, bool) {
        let now = w.hs_season.as_ref().and_then(|x| x.results.get(h)).filter(|r| r.wins + r.losses > 0);
        match now {
            Some(r) if b.day.month() >= 9 => (r.wins, r.losses, r.state_title),
            _ => w.high_schools[h].seasons.last().map_or((5, 5, false), |x| (x.wins, x.losses, x.state_title)),
        }
    };
    let (mut wins, mut games, mut title) = (0u32, 0u32, false);
    for &h in &d {
        let (wn, l, t) = record(h);
        wins += wn;
        games += wn + l;
        title |= t;
    }
    let win = if games == 0 { 0.5 } else { wins as f64 / games as f64 };
    let crowd = d.iter().map(|&h| crate::world::finance::hs::usual_crowd(w, h) as f64).sum::<f64>();
    let size = d.iter().map(|&h| 350.0 + 0.9 * w.high_schools[h].enrollment as f64).sum::<f64>();
    let crowds = clamp(0.02 * (crowd / size.max(1.0)).ln(), -0.03, 0.03);
    let enrollment: f64 = d.iter().map(|&h| w.high_schools[h].enrollment as f64).sum();
    let burden = b.cost / (enrollment.max(100.0) * 25_000.0 * hs_index(b.day.year()) / hs_index(2026).max(1e-9));
    let culture = w.states[s.state].culture.clamp(-1.0, 1.5);
    let me = w.seat_school().filter(|h| d.contains(h));
    let work = match me {
        Some(_) => b.work as f64,
        None => coach_work(w, lead),
    };
    YesParts {
        mood: 0.535,
        season: 0.1 * (win - 0.5) + if title { 0.03 } else { 0.0 },
        crowds,
        town: 0.04 * culture + 0.06 * (s.area_money - 0.5),
        tax: -0.05 * (burden - 1.0).clamp(-1.0, 3.0),
        work,
        win,
        games,
        title,
    }
}

/// Election night's spread around the expected share (proposed, not
/// confirmed: about two in three bonds pass, as Ballotpedia counts).
pub const NIGHT_SD: f64 = 0.06;

/// Election night: the share, the result, the building.
fn election(w: &mut World, k: usize) {
    let b = w.capital.bonds[k].clone();
    let expected = expected_yes(w, &b);
    let z = crate::world::season::key_normal(w.options.seed as u64 ^ 0xC6_E1, b.id as u64);
    let yes = clamp(expected + NIGHT_SD * z, 0.15, 0.9) as f32;
    let passed = yes >= b.needs;
    let d: Vec<usize> = b.schools.iter().map(|&h| h as usize).collect();
    let opens = if passed {
        let y = b.day.year() + if b.package == Package::New { 2 } else { 1 };
        Some(Date::ymd(y, 8, 1))
    } else {
        None
    };
    {
        let x = &mut w.capital.bonds[k];
        x.yes = Some(yes);
        x.passed = passed;
        x.opens = opens;
    }
    let pct = (yes * 100.0).round() as u32;
    let text = if passed {
        let when = opens.map_or(String::new(), |o| format!(" It opens for the {} season.", o.year()));
        format!("The stadium bond passes with {pct}% of the vote.{when}")
    } else {
        format!(
            "The stadium bond fails: {pct}% yes, {}% needed. The board can try again next year.",
            (b.needs * 100.0).round() as u32
        )
    };
    say(w, &d, Loudness::Big, text);
    // Your work and the result in the boss's book: the principal and the
    // superintendent remember a bond you asked for.
    if b.asked
        && let Some(me) = w.seat_school().filter(|h| d.contains(h))
    {
        let team = crate::world::folk::Team::High(me);
        let boss = crate::world::folk::boss::boss(w, team);
        w.folk.mine.nudge(boss.key(), if passed { 0.04 } else { -0.03 });
    }
}

/// A passed bond's stadium opens (CP34: every school in the district gets
/// it, your crosstown rival too).
fn open(w: &mut World, k: usize) {
    let b = w.capital.bonds[k].clone();
    w.capital.bonds[k].opened = true;
    let year = w.year;
    for &h in &b.schools {
        let Some(c) = w.capital.hs.get_mut(h as usize) else { continue };
        let st = HsRoom::Stadium as usize;
        let tf = HsRoom::Turf as usize;
        match b.package {
            Package::New => {
                c.level[st] = 5.0;
                c.since[st] = year;
            }
            Package::Renovation => {
                c.level[st] = (c.level[st] + 1.5).min(5.0);
                c.since[st] = year;
            }
        }
        // The stadium's field is turf: the schools without a practice
        // field of their own practice on it.
        if c.level[tf] < 4.5 {
            c.level[tf] = 4.5;
            c.since[tf] = year;
        }
    }
    // A new stadium's field house under the stands holds the lead school's
    // locker room.
    if b.package == Package::New
        && let Some(c) = w.capital.hs.get_mut(b.schools[0] as usize)
    {
        let lk = HsRoom::Locker as usize;
        c.level[lk] = c.level[lk].max(4.5);
        c.since[lk] = year;
    }
    let d: Vec<usize> = b.schools.iter().map(|&h| h as usize).collect();
    let text = if d.len() > 1 {
        format!(
            "{} opens: {} seats, home to {}.",
            b.name,
            b.seats,
            list(&d.iter().map(|&h| w.school_name(h)).collect::<Vec<_>>())
        )
    } else {
        format!("{} opens: {} seats.", b.name, b.seats)
    };
    say(w, &d, Loudness::Big, text);
}

// ---------------------------------------------------------------------------
// The year and the day
// ---------------------------------------------------------------------------

/// 1 July once the rooms live: every room ages (CP22), new schools get
/// rooms, the AI booster clubs pay for a room, the typical room is read.
pub(crate) fn year(w: &mut World) {
    if w.capital.hs_year >= w.year {
        return;
    }
    let first = w.capital.hs.is_empty();
    fill(w);
    w.capital.hs_year = w.year;
    if !first {
        for c in w.capital.hs.iter_mut() {
            for l in c.level.iter_mut() {
                if *l > 0.0 {
                    *l = (*l - AGE_A_YEAR).max(0.1);
                }
            }
        }
    }
    boosters(w);
    typical(w);
}

/// Every day once the rooms live: the boards' windows, election nights
/// and openings.
pub(crate) fn day(w: &mut World, d: Date) {
    if w.capital.hs.is_empty() {
        return;
    }
    if (d.month(), d.day()) == (2, 1) || (d.month(), d.day()) == (8, 1) {
        calls(w);
    }
    let due: Vec<usize> =
        (0..w.capital.bonds.len()).filter(|&k| w.capital.bonds[k].pending() && w.capital.bonds[k].day <= d).collect();
    for k in due {
        election(w, k);
    }
    let open_now: Vec<usize> = (0..w.capital.bonds.len())
        .filter(|&k| w.capital.bonds[k].building() && w.capital.bonds[k].opens.is_some_and(|o| o <= d))
        .collect();
    for &k in &open_now {
        open(w, k);
    }
    if !open_now.is_empty() {
        typical(w);
    }
    prune(w);
}

/// Old bonds go after twelve years.
fn prune(w: &mut World) {
    let year = w.year;
    w.capital.bonds.retain(|b| b.pending() || b.building() || b.day.year() + 12 > year);
}

// ---------------------------------------------------------------------------
// Your calls (CP32)
// ---------------------------------------------------------------------------

fn your_school(w: &World) -> Result<usize, String> {
    w.seat_school().ok_or_else(|| "Only a high school head coach works a bond.".to_string())
}

/// Your district's bond under way, if any.
pub fn your_bond(w: &World) -> Option<&Bond> {
    let h = w.seat_school()?;
    w.capital.bonds.iter().rev().find(|b| b.holds(h) && (b.pending() || b.building()))
}

/// The board's chance to call a bond on your ask: the stadium's need, the
/// principal's trust in you, your record (proposed, not confirmed).
pub fn ask_chance(w: &World, h: usize) -> f64 {
    let d = district(w, h);
    let team = crate::world::folk::Team::High(h);
    let trust = crate::world::folk::boss::trust(w, team) as f64;
    let win = w.high_schools[h].seasons.last().map_or(0.5, |x| x.wins as f64 / (x.wins + x.losses).max(1) as f64);
    clamp(0.08 + 1.2 * stadium_need(w, &d) + 0.5 * (trust - 0.5) + 0.2 * (win - 0.5), 0.03, 0.85)
}

/// The board puts a bond on the ballot on your ask, if it is free to:
/// the boss meeting's yes on a facility (`folk::boss`) lands here.
/// Returns the election day.
pub(crate) fn call_for(w: &mut World, h: usize) -> Result<Date, String> {
    if !super::live(w) {
        return Err("Not yet.".into());
    }
    fill(w);
    if w.high_schools[h].private {
        return Err("A private school has no bond: its foundation and booster club pay.".into());
    }
    let code = w.states[w.high_schools[h].state].code.clone();
    if !votes_in(&code) {
        return Err("The state builds its schools' stadiums; there is no local vote.".into());
    }
    let d = district(w, h);
    if busy(w, &d) {
        return Err("A bond is already on the ballot or being built, or the town voted on one lately.".into());
    }
    let id = call(w, &d, true).ok_or("No vote.")?;
    Ok(w.capital.bonds.iter().find(|b| b.id == id).map_or(w.today, |b| b.day))
}

/// The money year today falls in, by the year it starts: July to June.
fn money_year(w: &World) -> i32 {
    if w.today < w.dates.money_year { w.year - 1 } else { w.year }
}

/// Why you can't ask the board for a stadium bond now, if you can't (lane
/// CS6 reads it for the button; `ask_bond` answers with it).
pub fn bond_block(w: &World, h: usize) -> Option<String> {
    if w.capital.bond_asked == money_year(w) {
        return Some("You asked the board this year already; the answer stands until next summer.".into());
    }
    if !super::live(w) {
        return Some("Not yet.".into());
    }
    if w.high_schools[h].private {
        return Some("A private school has no bond: its foundation and booster club pay.".into());
    }
    let code = &w.states[w.high_schools[h].state].code;
    if !votes_in(code) {
        return Some("The state builds its schools' stadiums; there is no local vote.".into());
    }
    if busy(w, &district(w, h)) {
        return Some("A bond is already on the ballot or being built, or the town voted on one lately.".into());
    }
    None
}

/// You ask the superintendent and the board for a stadium bond (once a
/// money year): their yes puts it on the next ballot at least 78 days out.
pub fn ask_bond(w: &mut World) -> Result<String, String> {
    let h = your_school(w)?;
    let my = money_year(w);
    if super::live(w) {
        fill(w);
    }
    if let Some(why) = bond_block(w, h) {
        return Err(why);
    }
    let d = district(w, h);
    w.capital.bond_asked = my;
    let chance = ask_chance(w, h);
    if super::draw(w, 70, (h as u64) << 16 ^ my as u64) >= chance {
        let why = if stadium_need(w, &d) < 0.05 {
            "the stadium is in good shape, and the board has roofs to fix"
        } else {
            "the board doesn't think the town will vote for it this year"
        };
        return Ok(format!("The superintendent says no: {why}."));
    }
    let day = call_for(w, h)?;
    Ok(format!("The board puts a stadium bond on the ballot for {}.", day.pretty()))
}

/// Your work in town before a vote (CP32): each takes hours of your week,
/// one a week, within ten weeks of election day, each kind counted only so
/// many times.
pub fn town_work(w: &mut World, key: &str) -> Result<String, String> {
    let work = Work::from_key(key).ok_or("No such work.")?;
    let h = your_school(w)?;
    let today = w.today;
    let k = w
        .capital
        .bonds
        .iter()
        .rposition(|b| b.holds(h) && b.pending())
        .ok_or("There is no bond on the ballot in your district.")?;
    let day = w.capital.bonds[k].day;
    let left = today.days_to(day);
    if left > WORK_DAYS {
        return Err(format!("It's early: the campaign starts ten weeks before election day ({}).", day.pretty()));
    }
    if left < 0 {
        return Err("The vote is in.".into());
    }
    let id = w.capital.bonds[k].id;
    let week = today.add(-(today.weekday().index()));
    if w.capital.town.iter().any(|t| t.bond == id && t.date.add(-(t.date.weekday().index())) == week) {
        return Err("You already gave the campaign your slot this week.".into());
    }
    let (hours, lift, times) = work.size();
    let done = w.capital.town.iter().filter(|t| t.bond == id && t.work == work).count() as u8;
    if done >= times {
        return Err(format!("You've done {} as often as it helps.", work.words()));
    }
    if !crate::world::folk::talks::spend(w, hours) {
        return Err("No hours left this week.".into());
    }
    w.capital.town.push(TownWork { bond: id, date: today, work });
    w.capital.bonds[k].work += lift;
    Ok(format!("You did {}. Election day is {}.", work.words(), day.pretty()))
}

/// One piece of your work in town.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TownWork {
    pub bond: u32,
    pub date: Date,
    pub work: Work,
}

/// How the vote looks, in words (no numbers: the screens read it).
pub fn outlook(w: &World, b: &Bond) -> &'static str {
    let gap = expected_yes(w, b) - b.needs as f64;
    if gap > 0.08 {
        "The polls say it passes comfortably."
    } else if gap > 0.02 {
        "It leans yes."
    } else if gap > -0.02 {
        "A coin flip."
    } else if gap > -0.08 {
        "It leans no."
    } else {
        "The town is against it."
    }
}

/// Your school's rooms and bond for the screens (lane CS6).
pub fn seat_json(w: &World) -> Option<serde_json::Value> {
    let h = w.seat_school()?;
    let c = w.capital.hs.get(h)?;
    let rooms: Vec<serde_json::Value> = HsRoom::ALL
        .iter()
        .map(|&r| {
            serde_json::json!({
                "key": r.key(),
                "name": r.words(),
                "level": c.shown(r),
                "unlocks": r.unlocks(),
                "since": c.since[r as usize],
            })
        })
        .collect();
    let bond = your_bond(w).map(|b| {
        let mine: Vec<serde_json::Value> = w
            .capital
            .town
            .iter()
            .filter(|t| t.bond == b.id)
            .map(|t| {
                let (_, lift, _) = t.work.size();
                serde_json::json!({
                    "date": t.date.pretty(),
                    "work": t.work.key(),
                    "words": t.work.words(),
                    "points": (lift as f64 * 1000.0).round() / 10.0,
                })
            })
            .collect();
        let p = yes_parts(w, b);
        let pts = |x: f64| (x * 1000.0).round() / 10.0;
        serde_json::json!({
            "name": b.name,
            "package": match b.package { Package::New => "new", Package::Renovation => "renovation" },
            "seats": b.seats,
            "day": b.day.pretty(),
            "shared_with": b.schools.iter().filter(|&&o| o as usize != h).map(|&o| w.school_name(o as usize)).collect::<Vec<_>>(),
            "outlook": if b.pending() { outlook(w, b) } else { "" },
            "passed": b.passed,
            "yes": b.yes.map(|y| (y as f64 * 100.0).round() as i64),
            "opens": b.opens.map(|o| o.year()),
            "work": mine,
            "poll": (p.total() * 100.0).round() as i64,
            "needs": (b.needs as f64 * 100.0).round() as i64,
            "moved": {
                "season": pts(p.season), "crowds": pts(p.crowds), "town": pts(p.town), "tax": pts(p.tax),
                "work": pts(p.work), "win": p.win, "games": p.games, "title": p.title,
            },
        })
    });
    // The last vote in your district, for the line under the bar.
    let last = w
        .capital
        .bonds
        .iter()
        .filter(|b| b.holds(h) && !b.pending() && Some(b.id) != your_bond(w).map(|y| y.id))
        .max_by_key(|b| b.day)
        .map(|b| serde_json::json!({ "year": b.day.year(), "passed": b.passed, "yes": b.yes.map(|y| (y as f64 * 100.0).round() as i64) }));
    let block = bond_block(w, h);
    Some(serde_json::json!({
        "rooms": rooms,
        "bond": bond,
        "last": last,
        "asked": w.capital.bond_asked == money_year(w),
        "can_ask": block.is_none() && bond.is_none(),
        "why_not": block,
        "odds": super::odds_words(ask_chance(w, h)),
        "district": district(w, h).into_iter().map(|o| w.school_name(o).to_string()).collect::<Vec<_>>(),
    }))
}
