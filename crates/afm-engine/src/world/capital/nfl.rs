//! Capital lane C6, the NFL (`docs/design/capital.md`, CP35-CP36).
//!
//! Every club's training building has eight of the college rooms
//! (`ROOMS_NFL`): the locker room, the weight room, the sports science lab,
//! the training room and recovery, the cafeteria and the nutritionist, the
//! film rooms, the indoor field and the family room. The stadium stays as
//! the books build it (`finance::nfl`). There are no donors: the owner
//! pays.
//!
//! Each room moves its own line of the players' report card (CP35: the
//! locker room the locker room's grade, the weight room the weight room's,
//! the training room and the lab the training room's, the cafeteria food
//! and the nutritionist, the family room the treatment of families) and
//! its tool, read around the league's typical room: the body's coaching,
//! days back from injury, soft-tissue injuries, body plans, film study, the
//! room's mood.
//!
//! You ask the owner for one room a league year (CP35); winning, the
//! playoffs and other teams wanting you make the ask hard to refuse, a
//! losing season a long shot (CP36). Owners renovate on their own on the
//! same rule for every club (CP27), and a poor report card sends a proud
//! owner to fix the rooms it names.
//!
//! Before the rooms live (`capital::live`) every room reads the club's one
//! facility number, so the world's past stays the same per seed. Sizes are
//! Claude's (proposed, not confirmed).

use serde::{Deserialize, Serialize};

use super::rooms::{AGE_A_YEAR, ROOMS, Room};
use crate::calendar::{Date, Level, Loudness};
use crate::rng::clamp;
use crate::world::World;
use crate::world::folk::Team;

/// The rooms a training building holds.
pub const ROOMS_NFL: [Room; 8] = [
    Room::Locker,
    Room::Weight,
    Room::SportsScience,
    Room::Recovery,
    Room::Nutrition,
    Room::Film,
    Room::IndoorField,
    Room::PlayersLounge,
];

/// A room's share of the building (proposed, not confirmed; they sum to 1).
pub fn weight(r: Room) -> f64 {
    match r {
        Room::Locker => 0.16,
        Room::Weight => 0.18,
        Room::SportsScience => 0.10,
        Room::Recovery => 0.14,
        Room::Nutrition => 0.10,
        Room::Film => 0.14,
        Room::IndoorField => 0.12,
        Room::PlayersLounge => 0.06,
        _ => 0.0,
    }
}

/// The room's name in a training building.
pub fn words(r: Room) -> &'static str {
    match r {
        Room::Recovery => "training room",
        Room::Nutrition => "cafeteria and nutrition",
        Room::PlayersLounge => "family room and players' lounge",
        Room::Film => "film rooms",
        r => r.words(),
    }
}

/// What the room does at a club.
pub fn unlocks(r: Room) -> &'static str {
    match r {
        Room::Locker => "The locker room's grade; the team's mood",
        Room::Weight => "The weight room's grade; strength and body growth",
        Room::SportsScience => "The training room's grade; fewer soft-tissue injuries",
        Room::Recovery => "The training room's grade; hurt players back sooner",
        Room::Nutrition => "The food and nutritionist grades; body plans move faster",
        Room::Film => "Film study",
        Room::IndoorField => "Practice in bad weather",
        Room::PlayersLounge => "The families' grade; the team's mood",
        _ => "",
    }
}

/// A key for the calls: the room's index in `Room`.
pub fn from_key(k: &str) -> Option<Room> {
    ROOMS_NFL.into_iter().find(|r| key(*r) == k)
}

pub fn key(r: Room) -> &'static str {
    match r {
        Room::Locker => "locker",
        Room::Weight => "weight",
        Room::SportsScience => "science",
        Room::Recovery => "training",
        Room::Nutrition => "nutrition",
        Room::Film => "film",
        Room::IndoorField => "indoor",
        Room::PlayersLounge => "family",
        _ => "",
    }
}

/// One club's rooms (on `Capital.clubs`, by team).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ClubRooms {
    pub level: [f32; ROOMS],
    pub since: [i32; ROOMS],
    pub set: bool,
}

impl ClubRooms {
    pub fn level(&self, r: Room) -> f64 {
        self.level[r as usize] as f64
    }
    pub fn value(&self, r: Room) -> f64 {
        (self.level(r) / 5.0).clamp(0.0, 1.0)
    }
    pub fn shown(&self, r: Room) -> u8 {
        let l = self.level[r as usize];
        if l <= 0.0 { 0 } else { l.round().clamp(1.0, 5.0) as u8 }
    }
    /// The building's one number, from its rooms.
    pub fn number(&self) -> f64 {
        ROOMS_NFL.iter().map(|&r| weight(r) * self.value(r)).sum::<f64>().clamp(0.0, 1.0)
    }
}

/// A room renovation the owner pays for.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClubBuild {
    pub team: u8,
    /// The rooms it lifts, a bit per `Room`.
    pub rooms: u16,
    /// Dollars of the day (the books only).
    pub cost: f64,
    pub started: Date,
    pub opens: Date,
    /// You asked the owner for it.
    pub asked: bool,
    pub done: bool,
}

/// Your asks of the owner: (league year, room, his yes).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoomAsk {
    pub year: i32,
    pub room: Room,
    pub yes: bool,
}

// ---------------------------------------------------------------------------
// Reading the rooms
// ---------------------------------------------------------------------------

/// The club's rooms, once they live.
pub fn rooms_of(cap: &super::Capital, t: usize) -> Option<&ClubRooms> {
    cap.clubs.get(t).filter(|c| c.set && !cap.nfl_off)
}

/// A room's value at a club (0-1): the room once it lives, else the club's
/// one facility number.
pub fn value(w: &World, t: usize, r: Room) -> f64 {
    match rooms_of(&w.capital, t) {
        Some(c) => c.value(r),
        None => w.finance.nfl.get(t).map_or(0.6, |c| c.facility as f64),
    }
}

/// A room against the league's typical one (0 before the rooms live).
pub fn over(cap: &super::Capital, t: usize, r: Room) -> f64 {
    match rooms_of(cap, t) {
        Some(c) => clamp(c.value(r) - cap.nfl_typical[r as usize] as f64, -0.6, 0.6),
        None => 0.0,
    }
}

/// The body's rooms as one value (the college mix: the weight room,
/// nutrition, the lab, recovery).
fn physical_of(v: impl Fn(Room) -> f64) -> f64 {
    0.55 * v(Room::Weight) + 0.2 * v(Room::Nutrition) + 0.15 * v(Room::SportsScience) + 0.1 * v(Room::Recovery)
}

/// What the rooms add to the body's coaching, in coaching points (the
/// college slope, 1.8 a unit of the body's rooms, read around the league's
/// typical).
pub fn physical_lift(cap: &super::Capital, t: usize) -> f64 {
    let Some(c) = rooms_of(cap, t) else { return 0.0 };
    let typ = physical_of(|r| cap.nfl_typical[r as usize] as f64);
    clamp(1.8 * (physical_of(|r| c.value(r)) - typ), -0.9, 0.9)
}

/// The training room's days back, in the medical staff's units (the
/// college recovery center's slope).
pub fn recovery_units(cap: &super::Capital, t: usize) -> f64 {
    clamp(2.0 * over(cap, t, Room::Recovery), -1.0, 1.0)
}

/// The lab's guard on soft-tissue injuries (the college lab's slope).
pub fn soft_tissue_guard(cap: &super::Capital, t: usize) -> f64 {
    clamp(1.0 - 0.3 * over(cap, t, Room::SportsScience), 0.85, 1.15)
}

/// The film rooms' part of film study, in points.
pub fn film_lift(cap: &super::Capital, t: usize) -> f64 {
    clamp(7.0 * over(cap, t, Room::Film), -2.0, 2.0)
}

/// The locker room and the family room in the room's mood.
pub fn mood_lift(cap: &super::Capital, t: usize) -> f64 {
    clamp(0.12 * over(cap, t, Room::Locker) + 0.08 * over(cap, t, Room::PlayersLounge), -0.1, 0.1)
}

/// The cafeteria and nutrition on a body plan's weekly rate.
pub fn body_plan_rate(cap: &super::Capital, t: usize) -> f64 {
    clamp(1.0 + 0.4 * over(cap, t, Room::Nutrition), 0.85, 1.2)
}

/// Whether the club practices indoors in bad weather (every club has an
/// indoor field by 2026).
pub fn has_indoor(cap: &super::Capital, t: usize) -> bool {
    rooms_of(cap, t).is_none_or(|c| c.level(Room::IndoorField) > 0.0)
}

/// The report card's lines the rooms move (CP35), as their raw 0-1 values
/// before the survey's noise, from the club's old inputs. `newness` is the
/// stadium's part; `med` the medical staff's; `am` the owner's amenities.
/// Returns (families, food, nutritionist, locker, training room, weight
/// room) in place of the old reads, which they equal before the rooms live.
pub fn card_lines(w: &World, t: usize, base: CardBase) -> [f64; 6] {
    let CardBase { family, am, strength, med, newness, facility } = base;
    let old = [
        0.6 * family + 0.4 * am,
        am,
        0.6 * am + 0.4 * strength,
        facility * 0.7 + 0.3 * newness,
        0.6 * (facility * 0.7 + 0.3 * newness) + 0.4 * med,
        facility * 0.7 + 0.3 * newness,
    ];
    let cap = &w.capital;
    let Some(c) = rooms_of(cap, t) else { return old };
    let training = 0.5 * c.value(Room::Recovery) + 0.5 * c.value(Room::SportsScience);
    [
        old[0] + 0.3 * over(cap, t, Room::PlayersLounge),
        old[1] + 0.3 * over(cap, t, Room::Nutrition),
        old[2] + 0.3 * over(cap, t, Room::Nutrition),
        c.value(Room::Locker) * 0.7 + 0.3 * newness,
        0.6 * (training * 0.7 + 0.3 * newness) + 0.4 * med,
        c.value(Room::Weight) * 0.7 + 0.3 * newness,
    ]
}

/// What the old card read for a club.
#[derive(Clone, Copy, Debug)]
pub struct CardBase {
    pub family: f64,
    pub am: f64,
    pub strength: f64,
    pub med: f64,
    pub newness: f64,
    pub facility: f64,
}

/// The card line each room moves (`finance::data::CARD_PARTS`).
pub fn card_line(r: Room) -> Option<usize> {
    match r {
        Room::PlayersLounge => Some(0),
        Room::Nutrition => Some(1),
        Room::Locker => Some(3),
        Room::Recovery | Room::SportsScience => Some(4),
        Room::Weight => Some(6),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// The year
// ---------------------------------------------------------------------------

/// A renovation's cost: the room's share of a new training building, at
/// half (proposed, not confirmed: the building $160M in 2024).
pub fn renovation_cost(w: &World, r: Room) -> f64 {
    use crate::world::finance::data;
    let grow = data::league_revenue(w.year) / data::league_revenue(2024);
    160.0e6 * grow * weight(r) * 0.5
}

/// Where an owner keeps his building, as a level (proposed, not
/// confirmed: a spender's at the top, a cheap one's near the middle).
pub fn owner_target(w: &World, t: usize) -> f64 {
    5.0 * (0.55 + 0.45 * crate::world::finance::nfl::spend_level(w, t))
}

fn typical(w: &mut World) {
    let n = w.capital.clubs.iter().filter(|c| c.set).count().max(1) as f64;
    let mut out = [0f32; ROOMS];
    for r in ROOMS_NFL {
        out[r as usize] = (w.capital.clubs.iter().filter(|c| c.set).map(|c| c.value(r)).sum::<f64>() / n) as f32;
    }
    w.capital.nfl_typical = out;
}

/// The first live league year: each club's rooms start at its building's
/// number, a keyed half level either way (clubs renovate room by room).
fn fill(w: &mut World) {
    let n = w.finance.nfl.len().min(w.nfl.len());
    if w.capital.clubs.len() < n {
        w.capital.clubs.resize(n, ClubRooms::default());
    }
    let year = w.year;
    for t in 0..n {
        if w.capital.clubs[t].set {
            continue;
        }
        let f = w.finance.nfl[t].facility as f64;
        let since = w.finance.nfl[t].facility_year;
        let mut c = ClubRooms { set: true, ..Default::default() };
        for r in ROOMS_NFL {
            let u = super::draw(w, 80 + r as u64, t as u64);
            let l = clamp(5.0 * f + 0.8 * (u - 0.5), 0.5, 5.0);
            c.level[r as usize] = l as f32;
            c.since[r as usize] = since.min(year);
        }
        w.capital.clubs[t] = c;
    }
}

fn busy(w: &World, t: usize) -> bool {
    use crate::world::finance::{Owner, ProjectKind};
    w.capital.club_builds.iter().any(|b| b.team as usize == t && !b.done)
        || w.finance.projects.iter().any(|p| p.owner == Owner::Nfl(t) && !p.done && p.kind == ProjectKind::Facility)
}

fn build(w: &mut World, t: usize, rooms: u16, asked: bool) -> Date {
    let today = w.today;
    // An offseason job: asked by June it is done for training camp, later
    // for next year's.
    let y = if today.month() < 6 { today.year() } else { today.year() + 1 };
    let opens = Date::ymd(y, 7, 20);
    let cost = ROOMS_NFL.iter().filter(|r| rooms & r.bit() != 0).map(|&r| renovation_cost(w, r)).sum();
    w.capital.club_builds.push(ClubBuild { team: t as u8, rooms, cost, started: today, opens, asked, done: false });
    opens
}

/// The room furthest behind an owner's target, weighted by its share.
fn most_behind(c: &ClubRooms, target: f64) -> Option<(Room, f64)> {
    ROOMS_NFL
        .into_iter()
        .map(|r| (r, target - c.level(r)))
        .filter(|x| x.1 > 0.0)
        .max_by(|a, b| (a.1 * weight(a.0)).total_cmp(&(b.1 * weight(b.0))).then(b.0.cmp(&a.0)))
}

/// The day after the Super Bowl, once the rooms live (called by the books
/// after the projects open): the rooms are set the first year, then age a
/// tenth of a level a year (CP22); each owner renovates the room furthest
/// behind on the same rule for every club (CP27); the typical room is read
/// and each club's one number follows its rooms.
pub(crate) fn year(w: &mut World) {
    if !super::live(w) || w.capital.nfl_off || w.capital.nfl_year >= w.year {
        return;
    }
    let first = w.capital.clubs.iter().all(|c| !c.set);
    fill(w);
    w.capital.nfl_year = w.year;
    let year = w.year;
    if !first {
        for c in w.capital.clubs.iter_mut().filter(|c| c.set) {
            for r in ROOMS_NFL {
                let l = &mut c.level[r as usize];
                if *l > 0.0 {
                    *l = (*l - AGE_A_YEAR).max(0.1);
                }
            }
        }
    }
    let n = w.capital.clubs.len().min(w.finance.nfl.len());
    for t in 0..n {
        if !crate::world::franchise::active_in(w, t, year) || busy(w, t) {
            continue;
        }
        let Some((r, behind)) = most_behind(&w.capital.clubs[t], owner_target(w, t)) else { continue };
        let profit = w.finance.nfl[t]
            .years
            .last()
            .map_or(0.0, |b| crate::world::finance::nfl::Club::revenue(b) - crate::world::finance::nfl::Club::costs(b));
        let chance = super::rooms::renovation_chance(behind, profit);
        if super::draw(w, 88, (t as u64) << 16 ^ year as u64) < chance {
            build(w, t, r.bit(), false);
        }
    }
    typical(w);
    settle_numbers(w);
}

/// Each club's one number follows its rooms.
fn settle_numbers(w: &mut World) {
    let n = w.capital.clubs.len().min(w.finance.nfl.len());
    for t in 0..n {
        if w.capital.clubs[t].set {
            w.finance.nfl[t].facility = w.capital.clubs[t].number() as f32;
        }
    }
}

/// A new training building opens (`finance::nfl`): every room is new.
pub(crate) fn opened_building(w: &mut World, t: usize) {
    let year = w.year;
    let Some(c) = w.capital.clubs.get_mut(t).filter(|c| c.set) else { return };
    for r in ROOMS_NFL {
        c.level[r as usize] = 5.0;
        c.since[r as usize] = year;
    }
    typical(w);
    settle_numbers(w);
}

/// A poor report card names the rooms (`finance::nfl`, the owner shamed):
/// with one or two of the building's lines poor he renovates those rooms;
/// all three poor and he builds new (the old step, returned false).
pub(crate) fn card_push(w: &mut World, t: usize, grades: &[u8; 11]) -> bool {
    if rooms_of(&w.capital, t).is_none() {
        return false;
    }
    let poor = |k: usize| grades[k] <= 4;
    let n = [3, 4, 6].iter().filter(|&&k| poor(k)).count();
    if busy(w, t) {
        return true;
    }
    if n == 3 {
        return false;
    }
    let mut rooms = 0u16;
    if poor(3) {
        rooms |= Room::Locker.bit();
    }
    if poor(4) {
        rooms |= Room::Recovery.bit() | Room::SportsScience.bit();
    }
    if poor(6) {
        rooms |= Room::Weight.bit();
    }
    if rooms == 0 {
        return true;
    }
    build(w, t, rooms, false);
    let name = w.nfl[t].name;
    let list: Vec<&str> = ROOMS_NFL.iter().filter(|r| rooms & r.bit() != 0).map(|&r| words(r)).collect();
    let text =
        format!("The {name} will redo their {} this offseason, after their players' poor marks.", list.join(" and "));
    let e = crate::world::finance::story(w, Level::Nfl, Loudness::Medium, text).team(t);
    crate::world::finance::post(w, e);
    true
}

/// Every day once the rooms live: renovations done open for camp.
pub(crate) fn day(w: &mut World, d: Date) {
    if w.capital.club_builds.iter().all(|b| b.done) {
        return;
    }
    let year = w.year;
    let mut any = false;
    for k in 0..w.capital.club_builds.len() {
        let b = w.capital.club_builds[k].clone();
        if b.done || b.opens > d {
            continue;
        }
        w.capital.club_builds[k].done = true;
        any = true;
        let t = b.team as usize;
        if let Some(c) = w.capital.clubs.get_mut(t) {
            for r in ROOMS_NFL.iter().filter(|r| b.rooms & r.bit() != 0) {
                c.level[*r as usize] = 5.0;
                c.since[*r as usize] = year;
            }
        }
        if w.seat_club(t) {
            let list: Vec<&str> = ROOMS_NFL.iter().filter(|r| b.rooms & r.bit() != 0).map(|&r| words(r)).collect();
            let text = format!("The new {} is ready for camp.", list.join(" and "));
            let e = crate::world::finance::story(w, Level::Nfl, Loudness::Big, text).team(t);
            crate::world::finance::post(w, e);
        }
    }
    if any {
        typical(w);
        settle_numbers(w);
    }
    let cut = year - 6;
    w.capital.club_builds.retain(|b| !b.done || b.opens.year() > cut);
}

// ---------------------------------------------------------------------------
// Your asks (CP35) and your leverage (CP36)
// ---------------------------------------------------------------------------

/// Your leverage with the owner (CP36): last season's record, the
/// playoffs, and another team wanting you this past year, as a factor on
/// his yes and in words (proposed, not confirmed: 0.35 after a winless
/// year to 2.0 for a champion with offers).
pub fn leverage(w: &World, t: usize) -> (f64, &'static str) {
    let last = crate::world::folk::facts::seasons(w, Team::Nfl(t), 1);
    let (ws, playoffs) =
        last.last().map_or((0.5, false), |s| (s.wins as f64 / (s.wins + s.losses).max(1) as f64, !s.note.is_empty()));
    let wanted = wanted(w);
    let mut f = 0.5 + ws;
    if ws < 0.5 {
        f *= 0.7;
    }
    if playoffs {
        f += 0.15;
    }
    if wanted {
        f += 0.35;
    }
    let words = if f >= 1.3 {
        "Hard to refuse"
    } else if f >= 0.9 {
        "A fair hearing"
    } else {
        "A long shot"
    };
    (f, words)
}

/// Another team wanted you this past year: an offer made, or one you
/// turned down.
pub fn wanted(w: &World) -> bool {
    let Some(c) = w.career.as_ref() else { return false };
    let today = w.today;
    c.market.offers.iter().any(|o| o.made.days_to(today) < 365)
        || c.market.passed.iter().any(|p| p.2.days_to(today) < 365)
}

/// His yes chance on a room: his own (`folk::boss::chance` on a facility:
/// his money, his trust in you, the goals you met, his nature) times your
/// leverage, a little more for a room far behind.
pub fn ask_chance(w: &World, t: usize, r: Room) -> f64 {
    let base = crate::world::folk::boss::chance(w, Team::Nfl(t), crate::world::folk::boss::AskKind::Facility);
    let (lev, _) = leverage(w, t);
    let behind = rooms_of(&w.capital, t).map_or(0.0, |c| (owner_target(w, t) - c.level(r)).max(0.0));
    clamp(base * lev + 0.06 * behind, 0.02, 0.95)
}

/// You ask the owner for one room (CP35), once a league year. A yes is an
/// offseason renovation the owner pays for: done for camp if asked by
/// June, else for next year's. Returns his yes and his words.
pub fn ask_room(w: &mut World, r: Room) -> Result<(bool, String), String> {
    let t =
        w.seat.and_then(|s| s.nfl_team).filter(|&t| w.seat_club(t)).ok_or("Only an NFL head coach asks the owner.")?;
    if !ROOMS_NFL.contains(&r) {
        return Err(format!("A club has no {}.", r.words()));
    }
    if !super::live(w) {
        return Err("Not yet.".into());
    }
    fill(w);
    let year = w.year;
    if w.capital.room_asks.iter().any(|a| a.year == year) {
        return Err("You asked him for a room this league year already.".into());
    }
    if busy(w, t) {
        return Err("A renovation or a new building is already under way.".into());
    }
    if w.capital.clubs[t].level(r) >= 4.95 {
        return Err(format!("The {} is as good as they come.", words(r)));
    }
    let chance = ask_chance(w, t, r);
    let yes = super::draw(w, 89, (t as u64) << 20 ^ (year as u64) << 4 ^ r as u64) < chance;
    w.capital.room_asks.push(RoomAsk { year, room: r, yes });
    let team = Team::Nfl(t);
    let b = crate::world::folk::boss::boss(w, team);
    w.folk.mine.nudge(b.key(), if yes { -0.01 } else { -0.005 });
    let words_said = if yes {
        let opens = build(w, t, r.bit(), true);
        if opens.year() == w.today.year() {
            format!("\"Do it. Have it ready for camp.\" The new {} opens in July.", words(r))
        } else {
            format!("\"Fine. We'll do it next offseason.\" The new {} opens for camp next July.", words(r))
        }
    } else {
        let (lev, _) = leverage(w, t);
        if lev < 0.9 {
            "\"Win first. Then we'll talk about rooms.\"".to_string()
        } else {
            "\"Not this year. The money's spoken for.\"".to_string()
        }
    };
    w.folk.asks.push(crate::world::folk::boss::Ask {
        kind: crate::world::folk::boss::AskKind::Facility,
        date: w.today,
        granted: yes,
        words: words_said.clone(),
    });
    if w.folk.asks.len() > 40 {
        w.folk.asks.remove(0);
    }
    Ok((yes, words_said))
}

/// The rooms the owner meeting offers to ask for (CP35): the three
/// furthest behind the best, once the rooms live and no ask or build is
/// in the way this league year.
pub fn rooms_to_ask(w: &World, t: usize) -> Option<Vec<Room>> {
    let c = rooms_of(&w.capital, t)?;
    if !super::live(w) || w.capital.room_asks.iter().any(|a| a.year == w.year) || busy(w, t) {
        return None;
    }
    let mut v: Vec<(Room, f64)> = ROOMS_NFL.into_iter().map(|r| (r, c.level(r))).filter(|x| x.1 < 4.95).collect();
    v.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    let out: Vec<Room> = v.into_iter().take(3).map(|x| x.0).collect();
    (!out.is_empty()).then_some(out)
}

/// The owner's yes to a facility in words, not by room (a typed ask): he
/// redoes the room furthest behind where he keeps them.
pub(crate) fn grant_room(w: &mut World, t: usize) -> bool {
    if !super::live(w) {
        return false;
    }
    fill(w);
    if busy(w, t) {
        return false;
    }
    let target = owner_target(w, t).max(5.0);
    let Some((r, _)) = most_behind(&w.capital.clubs[t], target) else { return false };
    build(w, t, r.bit(), true);
    true
}

/// Your club's rooms for the screens (lane CS6): each room's level, what
/// it moves, its card line's latest grade, your leverage.
pub fn seat_json(w: &World) -> Option<serde_json::Value> {
    let t = w.seat.and_then(|s| s.nfl_team)?;
    let c = rooms_of(&w.capital, t)?;
    let card = crate::world::finance::nfl::card(w, t);
    let rooms: Vec<serde_json::Value> = ROOMS_NFL
        .iter()
        .map(|&r| {
            let grade =
                card_line(r).and_then(|k| card.map(|c| crate::world::finance::data::GRADES[c.grades[k] as usize]));
            serde_json::json!({
                "key": key(r),
                "name": words(r),
                "level": c.shown(r),
                "unlocks": unlocks(r),
                "card_line": card_line(r).map(|k| crate::world::finance::data::CARD_PARTS[k]),
                "grade": grade,
                "since": c.since[r as usize],
            })
        })
        .collect();
    let (_, lev) = leverage(w, t);
    let build = w.capital.club_builds.iter().rev().find(|b| b.team as usize == t && !b.done).map(|b| {
        serde_json::json!({
            "rooms": ROOMS_NFL.iter().filter(|r| b.rooms & r.bit() != 0).map(|&r| words(r)).collect::<Vec<_>>(),
            "opens": b.opens.pretty(),
            "asked": b.asked,
        })
    });
    Some(serde_json::json!({
        "rooms": rooms,
        "leverage": lev,
        "asked": w.capital.room_asks.iter().any(|a| a.year == w.year),
        "build": build,
        "owner": crate::world::finance::nfl::owner_words(w, t),
    }))
}
