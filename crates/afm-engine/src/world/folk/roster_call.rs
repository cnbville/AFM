//! Staff urgency: when to move on from a player (`docs/design/staff-urgency.md`,
//! 2026-11-21, Cain yes to all three questions).
//!
//! One read and one bar, shared by your talks (`persona::evaluate`), the
//! staff's lists in your meetings and the AI's own staffs:
//!
//! - **The read** (the same for every coach): the staff's read of the man
//!   placed against the level's marks at his spot (`norms::marks`), carried
//!   to his last season by the world's growth (`growth`, the Needs map's
//!   carry) in the middle and in a good case; the clock (the next portal
//!   window, free agency and the draft and the cutdown, a high school
//!   season); the portal's market at his spot as the world's past windows
//!   showed it (`Market`); what he costs to keep (his place under the
//!   limit) and the man himself (a leader keeps his place in the room).
//! - **The bar** (who the coach is): the level's starter mark for a
//!   starter's job, the backup mark for a depth job, moved by the coach's
//!   makeup and situation, never the read: up to `LOYAL_MOST` under it for
//!   the most patient, up to `RUTHLESS_MOST` over it for the most ruthless
//!   when the portal clearly has better.
//! - **The move**: move on now, bring a man in over him, or keep and
//!   develop, each with its reason in words.
//!
//! The AI's staffs act on the same calls (`told_for_window`, `within_flow`,
//! `flow_factors`, `camp_trim`): the men they move on from are told before
//! the winter window, at most a quarter of the program's expected entrants
//! and the men furthest under their bar first, and most enter it, inside
//! the world's portal total (the program's expected entrants stay the
//! same; the told men take the place of draws).
//!
//! Sizes are proposed, not confirmed.

use std::collections::{BTreeMap, HashSet};

use super::Team;
use super::bonds::{self, Seat};
use super::scene::{self, Doc, DocRow, Kind, Scene, Who};
use crate::calendar::Date;
use crate::player::Trait;
use crate::positions::{POSITION_COUNT, Position, overall_for};
use crate::world::data::Division;
use crate::world::norms::{self, Pool, Rated};
use crate::world::season::{key_normal, key_uniform};
use crate::world::{Stage, World, growth};

/// The most patient coach keeps a man projected up to this far under the
/// level's mark (Cain "sure", 2026-11-21; proposed, not confirmed).
pub const LOYAL_MOST: f64 = 4.0;
/// The most ruthless coach moves on from a man projected up to this far
/// over it, when the portal clearly has better (proposed, not confirmed).
pub const RUTHLESS_MOST: f64 = 2.0;
/// Under half a man a window at his spot that the program could land, the
/// market is thin: the portal rarely has better (proposed, not confirmed).
pub const THIN: f64 = 0.5;
/// The portal clearly has better: a man a window or more (proposed, not
/// confirmed).
pub const CLEAR: f64 = 1.0;
/// A man told he isn't in the plans enters the window; the rest stay and
/// fight for a place (proposed, not confirmed).
pub const TOLD_GO: f64 = 0.85;
/// The most of a program's expected entrants its staff pushes out in a
/// window: the men furthest under their bar first (proposed, not
/// confirmed: reporting says some entrants are encouraged to leave, no
/// study counts them).
pub const TOLD_SHARE: f64 = 0.25;
/// The AI staff's read error, for an average evaluator (points; proposed,
/// not confirmed).
pub const AI_READ_SD: f64 = 2.0;
/// The windows the market keeps.
const MARKET_WINDOWS: usize = 3;
/// A told man's word lasts this long (a meeting before the window).
const TOLD_DAYS: i32 = 75;

/// What the coach tells you to do with the man.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    /// He won't reach the bar in his time and the window gives a better chance.
    MoveOn,
    /// He won't reach it, but he is worth his place as depth.
    Over,
    /// He reaches it in time, or the market is too thin to do better.
    Keep,
}

/// The job he is judged for: his place in the room today.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Job {
    Starter,
    Depth,
}

/// The reason behind the move, for the words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    /// He gets there on the usual growth.
    InTime,
    /// A good year or two gets him there.
    YearAway,
    /// He won't, but the portal rarely has better at his spot.
    Thin,
    /// He won't, and the window has better.
    OutOfTime,
    /// He won't, but he holds the room together.
    Culture,
    /// He won't, but he takes no place a newcomer needs.
    Room,
}

/// One coach's call on one man.
#[derive(Clone, Debug)]
pub struct Call {
    pub i: usize,
    pub id: u32,
    /// The coach who makes the call (his position coach, else his
    /// coordinator, else the head coach).
    pub coach: Option<usize>,
    pub spot: Position,
    pub job: Job,
    /// His place in the room by the staff's reads (1 the best), of how many.
    pub place: (usize, usize),
    /// The staff's read now, and carried to his last season in the middle
    /// and in a good case.
    pub now: f64,
    pub last: f64,
    pub good: f64,
    /// Years of growth before his last season (seasons, contract years).
    pub years: u32,
    /// The level's mark for his job, and the coach's bar for it.
    pub line: f64,
    pub bar: f64,
    /// Men a window better than him at his job the program could land.
    pub market: f64,
    pub mv: Move,
    pub why: Why,
    /// The move the average coach would make (bar at the line): where it
    /// differs, the coach's own makeup decided, and he says so.
    pub plain: Move,
    /// He recruited the man himself.
    pub own: bool,
}

// --- The market ------------------------------------------------------------

/// One window's portal at each level and spot, as the world saw it.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Window {
    /// The offseason it belongs to (the season just played).
    pub label: i32,
    /// Each tier's starter and backup marks at each spot (tier × spots).
    pub marks: Vec<[f32; 2]>,
    /// Entrants at or above them.
    pub supply: Vec<[u16; 2]>,
    /// Transfers landed at or above the landing tier's starter mark, by
    /// tier and the landing program's fifth by prestige in its tier.
    pub landed: [[u16; 5]; 3],
    /// Programs in each tier and fifth.
    pub programs: [[u16; 5]; 3],
}

/// The portal's market over the last windows (`MARKET_WINDOWS`).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Market {
    pub windows: Vec<Window>,
}

impl Market {
    pub fn is_empty(&self) -> bool {
        self.windows.is_empty()
    }

    /// Men a window at or above the tier's mark at a spot (0 starter, 1
    /// backup), on average.
    fn supply(&self, tier: u8, pos: Position, which: usize) -> Option<f64> {
        let k = tier as usize * POSITION_COUNT + pos.index();
        let v: Vec<f64> = self.windows.iter().filter_map(|x| x.supply.get(k).map(|s| s[which] as f64)).collect();
        (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64)
    }

    /// The good transfers a program in this tier and fifth lands a window,
    /// over the tier's whole supply of good men: its share of the market.
    fn share(&self, tier: u8, fifth: usize) -> Option<f64> {
        let t = tier as usize;
        let (mut landed, mut programs, mut supply) = (0.0, 0.0, 0.0);
        for x in &self.windows {
            landed += x.landed[t][fifth] as f64;
            programs += x.programs[t][fifth] as f64;
            supply += (0..POSITION_COUNT)
                .filter_map(|k| x.supply.get(t * POSITION_COUNT + k).map(|s| s[0] as f64))
                .sum::<f64>();
        }
        (programs > 0.0 && supply > 0.0).then(|| landed / programs / (supply / self.windows.len() as f64))
    }
}

/// Each tier's starter and backup marks at every spot, read once.
pub struct LevelMarks {
    marks: Vec<Option<[f64; 3]>>,
    /// Each program's places taken next season: the men who return and
    /// the recruits signed for it (`recruiting::counted_rosters`).
    places: Vec<usize>,
}

impl LevelMarks {
    pub fn of(w: &World) -> LevelMarks {
        let mut marks = Vec::with_capacity(3 * POSITION_COUNT);
        for t in 0..3u8 {
            for pos in Position::ALL {
                marks.push(
                    norms::marks(w, Pool::College(t), pos, Rated::Overall).map(|m| [m.backup, m.starter, m.elite]),
                );
            }
        }
        LevelMarks { marks, places: crate::world::recruiting::counted_rosters(w, next_season(w)) }
    }

    fn get(&self, tier: u8, pos: Position) -> Option<[f64; 3]> {
        self.marks.get(tier as usize * POSITION_COUNT + pos.index()).copied().flatten()
    }
}

/// A program's fifth by prestige among the programs in its tier (0 the
/// lowest).
fn fifth(w: &World, j: usize) -> usize {
    let t = norms::tier(w.programs[j].division);
    let me = w.programs[j].prestige;
    let (mut below, mut all) = (0usize, 0usize);
    for p in w.programs.iter().filter(|p| norms::tier(p.division) == t && p.plays(w.year)) {
        all += 1;
        if p.prestige < me {
            below += 1;
        }
    }
    (5 * below / all.max(1)).min(4)
}

/// The window's entrants at each level's marks (`recruiting::portal_open`):
/// a new window in the market, or more for the cycle's window already in it.
pub(crate) fn record_supply(w: &mut World, label: i32, marks: &LevelMarks, entrants: &[usize]) {
    let mut win = Window {
        label,
        marks: (0..3u8)
            .flat_map(|t| Position::ALL.map(|pos| marks.get(t, pos).map_or([99.0; 2], |m| [m[1] as f32, m[0] as f32])))
            .collect(),
        supply: vec![[0; 2]; 3 * POSITION_COUNT],
        landed: [[0; 5]; 3],
        programs: [[0; 5]; 3],
    };
    for &i in entrants {
        let p = &w.people[i];
        let pos = p.player.position;
        let v = overall_for(pos, &p.player.attributes) as f32;
        for t in 0..3 {
            let k = t * POSITION_COUNT + pos.index();
            for which in 0..2 {
                if v >= win.marks[k][which] {
                    win.supply[k][which] += 1;
                }
            }
        }
    }
    for j in 0..w.programs.len() {
        if w.programs[j].plays(label + 1) {
            win.programs[norms::tier(w.programs[j].division) as usize][fifth(w, j)] += 1;
        }
    }
    let m = &mut w.folk.market;
    match m.windows.last_mut() {
        Some(last) if last.label == label => {
            for (a, b) in last.supply.iter_mut().zip(&win.supply) {
                a[0] += b[0];
                a[1] += b[1];
            }
        }
        _ => {
            m.windows.push(win);
            if m.windows.len() > MARKET_WINDOWS {
                m.windows.remove(0);
            }
        }
    }
}

/// A transfer landed at program `j`: counted for its tier and fifth when he
/// is at or above that tier's starter mark at his spot.
pub(crate) fn landed(w: &mut World, i: usize, j: usize, label: i32) {
    let t = norms::tier(w.programs[j].division) as usize;
    let p = &w.people[i];
    let pos = p.player.position;
    let v = overall_for(pos, &p.player.attributes) as f32;
    let f = fifth(w, j);
    if let Some(win) = w.folk.market.windows.iter_mut().rev().find(|x| x.label == label)
        && win.marks.get(t * POSITION_COUNT + pos.index()).is_some_and(|m| v >= m[0])
    {
        win.landed[t][f] += 1;
    }
}

// --- The clock ---------------------------------------------------------------

/// The next point at which the man can be replaced, at his level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clock {
    /// A portal window: open now (until `close`), or opening on `open`.
    Window { open: Date, close: Date },
    /// The NFL's next market: free agency, the draft, the cutdown.
    Nfl { day: Date, what: &'static str },
    /// High school: the next season.
    Season { day: Date },
}

/// The next college portal window still to close.
pub fn next_window(w: &World) -> Option<(Date, Date)> {
    let label = if w.today >= w.dates.fall_camp { w.year } else { w.year - 1 };
    for s in [label, label + 1] {
        let rule = w.changes.rules(s).transfer;
        for (open, close, which) in crate::world::clock::portal_windows(s, rule) {
            if which != crate::world::clock::PortalWindow::Graduate && close >= w.today {
                return Some((open, close));
            }
        }
    }
    None
}

fn clock(w: &World, team: Team) -> Option<Clock> {
    let today = w.today;
    match team {
        Team::College(_) => next_window(w).map(|(open, close)| Clock::Window { open, close }),
        Team::Nfl(_) | Team::Other(_) => {
            let y = &w.dates;
            [(y.league_year.date, "free agency"), (y.draft[0].date, "the draft"), (y.cutdown.date, "the cutdown to 53")]
                .into_iter()
                .find(|x| x.0 >= today)
                .map(|(day, what)| Clock::Nfl { day, what })
        }
        Team::High(_) => Some(Clock::Season {
            day: if today <= w.dates.fall_camp {
                w.dates.fall_camp
            } else {
                crate::calendar::YearDates::new(w.year + 1).fall_camp
            },
        }),
    }
}

const MONTHS: [&str; 12] = [
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

fn ordinal(d: u32) -> String {
    let s = match (d % 10, d % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{d}{s}")
}

/// "the 2nd" within the month, "January 2" further off.
fn day_words(today: Date, d: Date) -> String {
    let (_, m0, _) = today.parts();
    let (_, m, day) = d.parts();
    if m == m0 && today.days_to(d) < 31 {
        format!("the {}", ordinal(day))
    } else {
        format!("{} {}", MONTHS[m as usize - 1], ordinal(day))
    }
}

fn clock_words(w: &World, c: Option<Clock>) -> String {
    let today = w.today;
    match c {
        Some(Clock::Window { open, close }) if open <= today => {
            format!("The window is open until {}.", day_words(today, close))
        }
        Some(Clock::Window { open, .. }) if today.days_to(open) <= 45 => {
            format!("The window opens on {}.", day_words(today, open))
        }
        Some(Clock::Window { open, .. }) => {
            format!("The next window is in {}.", MONTHS[open.parts().1 as usize - 1])
        }
        Some(Clock::Nfl { day, what }) => format!("{} is on {}.", capital(what), day_words(today, day)),
        Some(Clock::Season { .. }) => "The season is the clock: he plays this fall or he doesn't.".into(),
        None => String::new(),
    }
}

fn capital(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

// --- The read ----------------------------------------------------------------

/// The pool a team is read against, and the growth table's tier for it.
fn pool(w: &World, team: Team) -> Option<(Pool, u8)> {
    match team {
        Team::College(j) => Some((Pool::College(norms::tier(w.programs.get(j)?.division)), 1)),
        Team::Nfl(_) | Team::Other(_) => Some((Pool::Nfl, 2)),
        Team::High(h) => {
            let s = w.high_schools.get(h)?;
            Some((Pool::Hs { state: Some(s.state), class: Some(s.classification), grade: 0 }, 0))
        }
    }
}

/// The next season he has not started yet.
fn next_season(w: &World) -> i32 {
    if w.today > w.dates.fall_camp { w.year + 1 } else { w.year }
}

/// Years of growth before his last season: his college seasons left from
/// the next one, his contract's years, his high school falls. None when
/// he has none left here.
fn years_left(w: &World, i: usize) -> Option<u32> {
    let p = &w.people[i];
    let next = next_season(w);
    let n = match p.stage {
        Stage::College => {
            // The season in play counts until January 2 settles it (the
            // portal's own check, `college::no_season_left`): on December 31
            // a redshirt junior's season just played is already his.
            let div = w.programs.get(p.college?)?.division;
            let el = crate::world::eligibility::seasons_left(p, div, next);
            let pending = (p.cal.count_pending && next > w.year) as u32;
            el.saturating_sub(pending)
        }
        Stage::Pro => {
            let last = p.pro.as_ref()?.contract.as_ref().map_or(next - 1, |k| k.last_year());
            (last - next + 1).max(0) as u32
        }
        Stage::HighSchool => (p.class_of - next).max(0) as u32,
        _ => 0,
    };
    (n > 0).then_some(n)
}

/// The coach who makes the call on him: his position coach, else his
/// coordinator, else his head coach.
fn his_coach(w: &World, i: usize) -> Option<usize> {
    let cs = bonds::coaches_of(w, i);
    for want in [Seat::Position, Seat::HsHead, Seat::Coordinator, Seat::HsCoordinator, Seat::Head] {
        if let Some(c) = cs.iter().find(|x| x.1 == want) {
            return Some(c.0);
        }
    }
    None
}

/// An AI staff's read of a man: his level with the staff's error, the
/// same error for a season, smaller for a better evaluator.
pub fn ai_read(w: &World, i: usize) -> f64 {
    let p = &w.people[i];
    let pos = p.player.position;
    let eye = his_coach(w, i).map_or(10.0, |c| w.coaches[c].evaluation());
    let sd = (AI_READ_SD * (1.0 - 0.05 * (eye - 10.0))).clamp(0.8, 3.5);
    overall_for(pos, &p.player.attributes) + sd * key_normal(0x0B5E_55ED ^ ((w.year as u64) << 20), p.player.id as u64)
}

/// Your staff's read of one of your men: the coach's own looks when he
/// has him in his room, else the staff's file.
pub fn your_read(w: &World, i: usize, coach: Option<usize>) -> Option<(f64, bool)> {
    use crate::world::scouting::file::Link;
    use crate::world::scouting::{own, readers};
    let staff = readers::seat_staff(w)?;
    let id = w.people[i].player.id;
    let (rds, _, f) = readers::read(w, staff, id)?;
    let mine = coach
        .and_then(|c| rds.iter().position(|r| r.link == Link::Coach(c)))
        .and_then(|r| own::reader_overall(w, i, &f, r));
    Some((mine.map_or(f.now.0, |m| m.0), mine.is_some()))
}

/// The coach's lean on the bar from his makeup and situation, -1 the most
/// patient to +1 the most ruthless (proposed, not confirmed): temper,
/// ambition, a hot seat, a contender, a strong room against loyalty, his
/// teaching at the spot, his bond with the man, his own recruit, a
/// rebuilding program and a thin room.
fn lean(w: &World, team: Team, coach: Option<usize>, i: usize, strong: bool, thin_room: bool) -> (f64, bool) {
    let p = &w.people[i];
    let id = p.player.id;
    let Some(c) = coach else { return (0.0, false) };
    let k = &w.coaches[c];
    let pers = &k.personality;
    let unit = |x: f64| ((x - 10.0) / 10.0).clamp(-1.0, 1.0);
    let group = crate::world::coach::Group::of(p.player.position) as usize;
    let teach = k.ability().teaching.get(group).copied().unwrap_or(10.0);
    let bond = bonds::bond(w, id, c);
    let close = bond.map_or(0.0, |b| b.close as f64);
    let own = bond.is_some_and(|b| b.met == bonds::Met::Recruited);
    let (head, contender, rebuild) = match team {
        Team::College(j) => {
            let prog = &w.programs[j];
            let last = prog.seasons.last();
            let rebuild = last.is_some_and(|s| s.wins < s.losses);
            (prog.head_coach, prog.prestige >= 0.75, rebuild)
        }
        Team::Nfl(t) => (w.nfl.get(t).and_then(|c| c.head_coach), false, false),
        _ => (None, false, false),
    };
    let hot = head.is_some_and(|h| super::claims::hot_seat(w, h));
    let ruthless = 0.25 * unit(pers.temper)
        + 0.25 * unit(pers.ambition)
        + if hot { 0.35 } else { 0.0 }
        + if contender { 0.15 } else { 0.0 }
        + if strong { 0.15 } else { 0.0 };
    let patient = 0.3 * unit(pers.loyalty)
        + 0.2 * unit(teach)
        + 0.4 * close
        + if own { 0.15 } else { 0.0 }
        + if rebuild { 0.15 } else { 0.0 }
        + if thin_room { 0.15 } else { 0.0 };
    ((ruthless - patient).clamp(-1.0, 1.0), own)
}

/// The move for a projection against a bar.
#[allow(clippy::too_many_arguments)]
fn decide(
    job: Job,
    last: f64,
    good: f64,
    years: u32,
    bar: f64,
    depth_bar: f64,
    market: f64,
    culture: bool,
    room: bool,
) -> (Move, Why) {
    if last >= bar {
        return (Move::Keep, Why::InTime);
    }
    if good >= bar && years >= 2 {
        return (Move::Keep, Why::YearAway);
    }
    if market < THIN {
        return (Move::Keep, Why::Thin);
    }
    let depth = match job {
        // A starter who won't start can still be a backup.
        Job::Starter => last >= depth_bar,
        // A backup who won't hold his place is depth only where he costs
        // nobody's.
        Job::Depth => false,
    };
    if culture {
        return (Move::Over, Why::Culture);
    }
    if depth || room {
        return (Move::Over, Why::Room);
    }
    (Move::MoveOn, Why::OutOfTime)
}

/// Every call a team's staff makes on its roster today, read with `read`
/// (your staff's reads at your seat, the AI's own at an AI program).
pub fn calls(
    w: &World,
    team: Team,
    table: &growth::Table,
    read: &dyn Fn(usize, Option<usize>) -> Option<f64>,
) -> Vec<Call> {
    calls_by(w, team, table, read, &|i| his_coach(w, i), None)
}

/// An AI program's calls on its own reads, with the levels' marks read once.
fn ai_calls(w: &World, j: usize, table: &growth::Table, marks: &LevelMarks) -> Vec<Call> {
    calls_by(w, Team::College(j), table, &|i, _| Some(ai_read(w, i)), &|i| his_coach(w, i), Some(marks))
}

/// The calls with the coach who makes each one named (a talk asks one
/// coach about one man: he makes that call).
pub fn calls_by(
    w: &World,
    team: Team,
    table: &growth::Table,
    read: &dyn Fn(usize, Option<usize>) -> Option<f64>,
    coach_of: &dyn Fn(usize) -> Option<usize>,
    level_marks: Option<&LevelMarks>,
) -> Vec<Call> {
    let Some((pool, gtier)) = pool(w, team) else { return Vec::new() };
    let roster: Vec<usize> = team
        .roster(w)
        .into_iter()
        .filter(|&i| {
            let p = &w.people[i];
            p.cal.portal.is_none() && p.cut_here().is_none() && p.cal.declare_plan != Some(true)
        })
        .collect();
    let next = next_season(w);
    // The limit: the program's places, and how many take one.
    let room_left = match team {
        Team::College(j) => {
            let prog = &w.programs[j];
            let whole = crate::world::ncaa::whole_roster_limit(prog, next);
            let limit = crate::world::ncaa::roster_limits(prog, next).0;
            // The places as a staff counts them: the roster it has now
            // under next season's rules (a program that filled its places
            // will fill them again), or the men who return with the class
            // it signed, whichever is more. December's leavers and February's
            // class both count.
            let now =
                roster.iter().filter(|&&i| crate::world::recruiting::takes_a_place(&w.people[i], whole, next)).count();
            let signed = match level_marks {
                Some(lm) => lm.places.get(j).copied().unwrap_or(0),
                None => crate::world::recruiting::counted_rosters(w, next).get(j).copied().unwrap_or(0),
            };
            let used = now.max(signed);
            Some((limit.saturating_sub(used), whole))
        }
        _ => None,
    };
    let tier = match pool {
        Pool::College(t) => Some(t),
        _ => None,
    };
    let fifth = match team {
        Team::College(j) => fifth(w, j),
        _ => 2,
    };
    let captains: HashSet<usize> = match team {
        Team::College(j) => w.programs[j].captains.iter().copied().collect(),
        Team::Nfl(t) => w.clubs.get(t).map(|c| c.captains.iter().copied().collect()).unwrap_or_default(),
        _ => HashSet::new(),
    };
    let mut out = Vec::new();
    for pos in Position::ALL {
        let men: Vec<usize> = roster.iter().copied().filter(|&i| w.people[i].player.position == pos).collect();
        if men.is_empty() {
            continue;
        }
        let m = match (level_marks, pool) {
            (Some(lm), Pool::College(t)) => lm.get(t, pos),
            _ => norms::marks(w, pool, pos, Rated::Overall).map(|m| [m.backup, m.starter, m.elite]),
        };
        let Some([backup, starter, _]) = m else { continue };
        let coached: Vec<(usize, Option<usize>)> = men.iter().map(|&i| (i, coach_of(i))).collect();
        let reads: Vec<(usize, Option<usize>, f64)> =
            coached.iter().filter_map(|&(i, c)| read(i, c).map(|r| (i, c, r))).collect();
        let starters = crate::game::roster::starters(pos).max(1);
        let strong = reads.iter().filter(|x| x.2 >= backup).count() >= 2 * starters;
        let thin_room = men.len() <= 2 * starters;
        for &(i, coach, now) in &reads {
            let Some(years) = years_left(w, i) else { continue };
            let p = &w.people[i];
            let place = 1 + reads.iter().filter(|x| x.0 != i && x.2 > now).count();
            let job = if place <= starters { Job::Starter } else { Job::Depth };
            let line = match job {
                Job::Starter => starter,
                Job::Depth => backup,
            };
            let tiers = vec![gtier; years as usize];
            let last = *growth::carry(table, now, p.player.age, &tiers, growth::MID).last().unwrap_or(&now);
            let good = *growth::carry(table, now, p.player.age, &tiers, growth::GOOD).last().unwrap_or(&now);
            // The market at his job: the men a window at or above the mark
            // the program could land. Without a market read yet (an old
            // save before its first window, the NFL, high school) it is
            // neither thin nor clear.
            let market = match tier {
                Some(t) => {
                    let which = if job == Job::Starter { 0 } else { 1 };
                    match (w.folk.market.supply(t, pos, which), w.folk.market.share(t, fifth)) {
                        // A staff aims its landings at the spot it needs:
                        // the program's rate on an average spot's men,
                        // never more than all of them.
                        (Some(s), Some(sh)) => s * (sh * POSITION_COUNT as f64).min(1.0),
                        _ => CLEAR,
                    }
                }
                None => CLEAR,
            };
            let (s, own) = lean(w, team, coach, i, strong, thin_room);
            let shift = if s >= 0.0 { if market >= CLEAR { RUTHLESS_MOST * s } else { 0.0 } } else { LOYAL_MOST * s };
            let bar = line + shift;
            let culture = captains.contains(&i) || p.player.personality.traits.get(Trait::Leadership) >= 16.0;
            // Room: a walk-on outside a scholarship limit takes no place;
            // under a whole-roster limit only open places count.
            let room = match room_left {
                Some((left, whole)) => (!whole && p.walk_on) || left >= 3,
                None => false,
            };
            let (mv, why) = decide(job, last, good, years, bar, backup + shift, market, culture, room);
            let (plain, _) = decide(job, last, good, years, line, backup, market, culture, room);
            out.push(Call {
                i,
                id: p.player.id,
                coach,
                spot: pos,
                job,
                place: (place, reads.len()),
                now,
                last,
                good,
                years,
                line,
                bar,
                market,
                mv,
                why,
                plain,
                own,
            });
        }
    }
    out
}

/// Your staff's calls on your roster today.
pub fn seat_calls(w: &World) -> Vec<Call> {
    let Some(team) = Team::seat(w) else { return Vec::new() };
    let table = growth::cached(w);
    calls(w, team, &table, &|i, c| your_read(w, i, c).map(|r| r.0))
}

/// One coach's call on one of your men (his own when he coaches him).
pub fn seat_call(w: &World, i: usize, coach: usize) -> Option<Call> {
    let team = Team::seat(w)?;
    if !team.roster(w).contains(&i) {
        return None;
    }
    let table = growth::cached(w);
    calls_by(
        w,
        team,
        &table,
        &|k, c| your_read(w, k, c).map(|r| r.0),
        &|k| {
            if k == i { Some(coach) } else { his_coach(w, k) }
        },
        None,
    )
    .into_iter()
    .find(|c| c.i == i)
}

// --- The words -----------------------------------------------------------------

fn spots(pos: Position) -> String {
    let s = super::claims::words::spot(pos);
    match s.strip_suffix('y') {
        Some(stem) => format!("{stem}ies"),
        None => format!("{s}s"),
    }
}

fn how_many(x: f64) -> &'static str {
    match x.round() as i64 {
        ..=0 => "hardly ever one",
        1 => "one",
        2 => "two",
        3 => "three",
        _ => "a few",
    }
}

/// The level in words ("an FBS starter").
fn level_words(w: &World, team: Team, job: Job) -> String {
    let what = match job {
        Job::Starter => "starter",
        Job::Depth => "two-deep player",
    };
    match team {
        Team::College(j) => match w.programs[j].division {
            Division::FBS => format!("an FBS {what}"),
            Division::FCS => format!("an FCS {what}"),
            _ => format!("a {what} at this level"),
        },
        Team::Nfl(_) | Team::Other(_) => format!("an NFL {what}"),
        Team::High(_) => format!("a varsity {what}"),
    }
}

/// The move in words for a seat: what you'd do.
fn act_words(team: Team, mv: Move, senior: bool) -> &'static str {
    match (team, mv) {
        (_, Move::Keep) => "Keep him.",
        (Team::College(_), Move::MoveOn) => "Move on now.",
        (Team::College(_), Move::Over) => "Bring a man in over him and keep him.",
        (Team::Nfl(_) | Team::Other(_), Move::MoveOn) => "Release him.",
        (Team::Nfl(_) | Team::Other(_), Move::Over) => "Sign or draft a man over him and keep him as depth.",
        (Team::High(_), Move::MoveOn) if senior => "Play the younger men and tell him where he can play next.",
        (Team::High(_), Move::MoveOn) => "Move him down to JV or play him less.",
        (Team::High(_), Move::Over) => "Start the man behind him and keep him in the rotation.",
    }
}

/// The call in words: what he tells you and why (no number but a date).
pub fn words(w: &World, c: &Call) -> String {
    let Some(team) = Team::seat(w).or_else(|| w.people[c.i].college.map(Team::College)) else { return String::new() };
    let spot = spots(c.spot);
    let level = level_words(w, team, c.job);
    let senior = c.years <= 1;
    let out_of_years = match team {
        Team::Nfl(_) | Team::Other(_) => "before his deal is up",
        _ => "before he's out of years",
    };
    let market = match team {
        Team::College(_) if c.market < THIN => format!("The portal rarely has better {spot} than him."),
        Team::College(_) => {
            format!("We usually find {} {spot} better than him there.", how_many(c.market))
        }
        _ => String::new(),
    };
    let clock = clock_words(w, clock(w, team));
    let mut v: Vec<String> = Vec::new();
    match c.why {
        Why::InTime => v.push(match c.job {
            Job::Starter => format!("He's on track to be {level} before he's done."),
            Job::Depth => format!("He's on track to be {level}."),
        }),
        Why::YearAway => v.push("He's a year away, and he has the time.".to_string()),
        Why::Thin => {
            v.push(format!("He won't be {level} {out_of_years}."));
            v.push(market);
        }
        Why::OutOfTime => {
            v.push(format!("He won't be {level} {out_of_years}."));
            if !clock.is_empty() {
                v.push(clock);
            }
            if !market.is_empty() {
                v.push(market);
            }
        }
        Why::Culture => {
            v.push(format!("He won't be {level} {out_of_years}, but he holds the room together."));
        }
        Why::Room => v.push(match c.job {
            Job::Starter => format!("He won't be {level} {out_of_years}, but he's worth his place as a backup."),
            Job::Depth => format!("He won't be {level} {out_of_years}, but he takes no place we need."),
        }),
    }
    // He can be wrong, and says why: his makeup made the call.
    if c.mv != c.plain {
        v.push(if c.mv == Move::Keep || (c.mv == Move::Over && c.plain == Move::MoveOn) {
            if c.own { "I recruited him, I believe in him." } else { "I believe in him." }.to_string()
        } else {
            "We can do better, and we can't wait on him.".to_string()
        });
    }
    v.push(act_words(team, c.mv, senior).to_string());
    v.into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" ")
}

/// The move in a word, for the lists.
fn move_cell(team: Team, mv: Move) -> &'static str {
    match (team, mv) {
        (_, Move::Keep) => "Keep",
        (Team::Nfl(_) | Team::Other(_), Move::MoveOn) => "Release",
        (Team::High(_), Move::MoveOn) => "Less time",
        (_, Move::MoveOn) => "Move on",
        (Team::High(_), Move::Over) => "Start the next man",
        (_, Move::Over) => "Bring in a man over him",
    }
}

// --- Your meetings ---------------------------------------------------------------

/// Marks a staff report as a roster meeting (`Scene.a`), with its occasion
/// in the low bits.
pub const ROSTER: u64 = 1 << 40;

/// The decision points where your staff brings its list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u64)]
pub enum Occasion {
    /// The first meetings of a new job (in the state of the team).
    Takeover = 0,
    WinterWindow = 1,
    SpringWindow = 2,
    FallCamp = 3,
    FreeAgency = 4,
    Draft = 5,
    Cutdown = 6,
    HsSeason = 7,
    HsAfter = 8,
}

impl Occasion {
    fn of(code: u64) -> Occasion {
        match code & 0xF {
            1 => Occasion::WinterWindow,
            2 => Occasion::SpringWindow,
            3 => Occasion::FallCamp,
            4 => Occasion::FreeAgency,
            5 => Occasion::Draft,
            6 => Occasion::Cutdown,
            7 => Occasion::HsSeason,
            8 => Occasion::HsAfter,
            _ => Occasion::Takeover,
        }
    }

    fn title(self) -> &'static str {
        match self {
            Occasion::Takeover => "The state of the team",
            Occasion::WinterWindow => "The roster meeting before the window",
            Occasion::SpringWindow => "The roster meeting before the spring window",
            Occasion::FallCamp => "The roster meeting before fall camp",
            Occasion::FreeAgency => "The roster meeting before free agency",
            Occasion::Draft => "The roster meeting before the draft",
            Occasion::Cutdown => "The roster meeting before the cutdown",
            Occasion::HsSeason => "The roster meeting before the season",
            Occasion::HsAfter => "The roster meeting after the season",
        }
    }
}

/// This football year's meeting days for your seat.
fn meeting_days(w: &World, team: Team) -> Vec<(Date, Occasion)> {
    let y = &w.dates;
    let mut v = Vec::new();
    match team {
        Team::College(j) => {
            let s = w.year;
            let rule = w.changes.rules(s).transfer;
            for (open, _, which) in crate::world::clock::portal_windows(s, rule) {
                if which == crate::world::clock::PortalWindow::Winter {
                    // After the last regular-season game, before the window.
                    let day = y.conference_championships.add(1).min(open.add(-1));
                    v.push((day, Occasion::WinterWindow));
                }
            }
            let rule = w.changes.rules(s - 1).transfer;
            for (open, _, which) in crate::world::clock::portal_windows(s - 1, rule) {
                if which == crate::world::clock::PortalWindow::Spring
                    && rule == crate::calendar::rules::TransferRule::Windows
                {
                    v.push((open.add(-7), Occasion::SpringWindow));
                }
            }
            if crate::world::ncaa::whole_roster_limit(&w.programs[j], s) {
                v.push((y.fall_camp.add(-7), Occasion::FallCamp));
            }
        }
        Team::Nfl(_) | Team::Other(_) => {
            v.push((y.league_year.date.add(-7), Occasion::FreeAgency));
            v.push((y.draft[0].date.add(-10), Occasion::Draft));
            v.push((y.cutdown.date.add(-4), Occasion::Cutdown));
        }
        Team::High(_) => {
            v.push((y.fall_camp.add(-3), Occasion::HsSeason));
            v.push((y.december_graduation, Occasion::HsAfter));
        }
    }
    v
}

/// Each day: your staff's roster meeting on its day (Cain: "whenever it
/// makes sense, like the first meetings, before portals etc").
pub(crate) fn day(w: &mut World, d: Date) {
    let Some(team) = Team::seat(w) else { return };
    for (day, occ) in meeting_days(w, team) {
        if day != d {
            continue;
        }
        let tag = ROSTER | occ as u64;
        if w.folk.scenes.iter().any(|s| s.kind == Kind::StaffReport && s.a == tag && s.date.days_to(d).abs() < 30) {
            continue;
        }
        let mut s = Scene::new(Kind::StaffReport, d, occ.title());
        s.a = tag;
        s.place = match team {
            Team::Nfl(_) | Team::Other(_) => "The draft room, the board on the wall".into(),
            _ => "The staff room, the depth chart on the board".into(),
        };
        scene::add(w, s);
    }
}

/// The staff's calls as a list beside a meeting: each coach's men he would
/// move on from or bring a man in over, first, then the keeps worth
/// saying.
pub(crate) fn list_doc(w: &World, team: Team, calls: &[Call]) -> Doc {
    let mut rows: Vec<&Call> = calls.iter().filter(|c| c.mv != Move::Keep || c.plain != Move::Keep).collect();
    rows.sort_by_key(|c| (c.mv != Move::MoveOn, c.mv != Move::Over, c.spot.index(), c.place.0));
    Doc {
        title: "Your coaches' calls".into(),
        note: "Each coach's call on the men he coaches: against the level, where they'll be when they're out of time, \
               the clock and the portal at their spot."
            .into(),
        cols: vec!["Pos".into(), "Player".into(), "Coach".into(), "Call".into(), "Why".into()],
        rows: rows
            .into_iter()
            .map(|c| DocRow {
                pid: Some(c.id),
                cells: vec![
                    c.spot.name().into(),
                    super::player_link(w, c.i),
                    c.coach.map(|k| super::coach_link(w, k)).unwrap_or_default(),
                    move_cell(team, c.mv).into(),
                    words(w, c),
                ],
            })
            .collect(),
    }
}

/// The meeting opens: the list and the picks.
pub(crate) fn open(w: &mut World, s: &mut Scene, team: Team) {
    let occ = Occasion::of(s.a);
    let calls = seat_calls(w);
    let on = calls.iter().filter(|c| c.mv == Move::MoveOn).count();
    let over = calls.iter().filter(|c| c.mv == Move::Over).count();
    s.who.push(Who::Group("Your coordinators and position coaches".into()));
    let clock = clock_words(w, clock(w, team));
    s.narrate(format!(
        "Your coaches go through the roster spot by spot, each with his own men. {clock} {}",
        match (on, over) {
            (0, 0) => "Nobody they'd move on from.".to_string(),
            (on, 0) => format!("They'd move on from {}.", count_men(on)),
            (0, over) => format!("They'd bring a man in over {}.", count_men(over)),
            (on, over) => format!("They'd move on from {} and bring a man in over {}.", count_men(on), count_men(over)),
        }
    ));
    if occ == Occasion::FallCamp
        && let Team::College(j) = team
    {
        let over_by = over_limit(w, j);
        if over_by > 0 {
            s.narrate(format!("You're {} over the limit before camp.", count_men(over_by)));
            s.pick(
                44,
                "Trim to the limit, weakest calls first.",
                "Hand him something",
                "firm",
                "Cut to the limit by their list.",
            );
        }
    }
    s.docs.push(list_doc(w, team, &calls));
    if matches!(occ, Occasion::WinterWindow | Occasion::SpringWindow) && on > 0 {
        s.pick(
            42,
            "Tell the men we're moving on from, before the window.",
            "Hand him something",
            "firm",
            "Most of them will enter the window; the room will see it.",
        );
    }
    s.pick(43, "I'll make each call myself.", "Ask", "calm", "The list is filed; nothing changes until you act.");
}

fn count_men(n: usize) -> String {
    match n {
        1 => "one man".into(),
        2 => "two men".into(),
        3 => "three men".into(),
        n => format!("{n} men"),
    }
}

/// Your answer to the meeting.
pub(crate) fn answer(w: &mut World, s: &mut Scene, _team: Team, code: u32) {
    match code {
        42 => {
            let calls = seat_calls(w);
            let today = w.today;
            let mut n = 0;
            for c in calls.iter().filter(|c| c.mv == Move::MoveOn) {
                tell(w, c.i, today);
                n += 1;
            }
            s.outcome.push(format!("Your coaches told {} they aren't in the plans.", count_men(n)));
        }
        44 => {
            let n = seat_trim(w);
            s.outcome.push(format!("Cut {} to reach the limit.", count_men(n)));
        }
        _ => s.outcome.push("The list is filed. The calls are yours.".into()),
    }
    s.end();
}

/// He is told he isn't in the plans: his word for the window.
fn tell(w: &mut World, i: usize, today: Date) {
    let id = w.people[i].player.id;
    w.folk.told.insert(id, today);
    let p = &mut w.people[i];
    p.trust = (p.trust - 0.1).max(0.0);
    p.cal.lean_events.push((today, "his coaches told him he isn't in the plans", 1.0));
}

fn over_limit(w: &World, j: usize) -> usize {
    let prog = &w.programs[j];
    let s = w.year;
    let whole = crate::world::ncaa::whole_roster_limit(prog, s);
    let limit = crate::world::ncaa::roster_limits(prog, s).0;
    let used = prog
        .roster
        .iter()
        .filter(|&&i| w.people[i].stage == Stage::College)
        .filter(|&&i| crate::world::recruiting::takes_a_place(&w.people[i], whole, s))
        .count();
    used.saturating_sub(limit)
}

/// The order a trim cuts in: the men furthest under their bar first.
fn trim_order(w: &World, j: usize, calls: &[Call]) -> Vec<usize> {
    let s = w.year;
    let whole = crate::world::ncaa::whole_roster_limit(&w.programs[j], s);
    let mut v: Vec<&Call> = calls
        .iter()
        .filter(|c| c.mv != Move::Keep && crate::world::recruiting::takes_a_place(&w.people[c.i], whole, s))
        .collect();
    v.sort_by(|a, b| (a.last - a.bar).total_cmp(&(b.last - b.bar)).then(a.id.cmp(&b.id)));
    v.into_iter().map(|c| c.i).collect()
}

/// Your trim to the limit: your cuts, by the staff's list.
fn seat_trim(w: &mut World) -> usize {
    let Some(Team::College(j)) = Team::seat(w) else { return 0 };
    let n = over_limit(w, j);
    let order = trim_order(w, j, &seat_calls(w));
    let mut done = 0;
    for i in order.into_iter().take(n) {
        let id = w.people[i].player.id;
        if crate::world::front_office::college::cut(w, id).is_ok() {
            done += 1;
        }
    }
    done
}

// --- The AI's staffs -------------------------------------------------------------

/// The measure's switch: `AFM_URGENCY=off` leaves the AI's staffs as they
/// were (the before arm of the world measure).
fn ai_on() -> bool {
    std::env::var("AFM_URGENCY").map_or(true, |v| v != "off")
}

/// The programs whose staffs act on their calls: the scholarship
/// divisions (proposed, not confirmed: Division III, the NAIA and junior
/// colleges rarely run a man off).
fn acts(d: Division) -> bool {
    matches!(d, Division::FBS | Division::FCS | Division::D2)
}

/// A program's men told and the spots it wants a man over.
type ProgramCalls = (usize, Vec<(usize, f64)>, Vec<(u8, u8)>);

/// Before the winter window: the men each staff tells it is moving on
/// from (the AI's own programs on their calls, and yours by your word in
/// the roster meeting), and the spots its staff wants a man over the one
/// it has (`Folk.over`, read by the portal market).
pub(crate) fn told_for_window(w: &mut World) -> (HashSet<usize>, Vec<(usize, f64)>) {
    let mut told = HashSet::new();
    let seat = match Team::seat(w) {
        Some(Team::College(j)) => Some(j),
        _ => None,
    };
    // Yours: your word, given in this cycle's meeting.
    let today = w.today;
    if let Some(j) = seat {
        for &i in &w.programs[j].roster {
            let id = w.people[i].player.id;
            if w.folk.told.get(&id).is_some_and(|d| d.days_to(today) <= TOLD_DAYS) {
                told.insert(i);
            }
        }
    }
    w.folk.told.clear();
    w.folk.over.clear();
    if !ai_on() {
        return (told, Vec::new());
    }
    let table = growth::table(w);
    let marks = LevelMarks::of(w);
    let progs: Vec<usize> = (0..w.programs.len())
        .filter(|&j| Some(j) != seat && acts(w.programs[j].division) && w.programs[j].plays(w.year + 1))
        .collect();
    let wr: &World = w;
    use rayon::prelude::*;
    let found: Vec<ProgramCalls> = progs
        .par_iter()
        .map(|&j| {
            let cs = ai_calls(wr, j, &table, &marks);
            let on: Vec<(usize, f64)> =
                cs.iter().filter(|c| c.mv == Move::MoveOn).map(|c| (c.i, c.last - c.bar)).collect();
            let over: Vec<(u8, u8)> = cs
                .iter()
                .filter(|c| c.mv == Move::Over && c.job == Job::Starter)
                .map(|c| (c.spot.index() as u8, 1))
                .collect();
            (j, on, over)
        })
        .collect();
    if std::env::var("AFM_URGENCY_TRACE").is_ok() {
        trace(wr, &progs, &table, &marks);
    }
    let mut cands = Vec::new();
    for (j, on, over) in found {
        cands.extend(on);
        for (k, n) in over {
            *w.folk.over.entry((j as u16, k)).or_insert(0) += n;
        }
    }
    (told, cands)
}

/// The AI's men each staff tells, within its flow: at most `TOLD_SHARE` of
/// the program's expected entrants (its lean draws, `eligible` saying who
/// reaches them), the men furthest under their bar first. The rest stay on
/// the list and fight for their place.
pub(crate) fn within_flow(w: &World, cands: &[(usize, f64)], era: f64, eligible: &dyn Fn(usize) -> bool) -> Vec<usize> {
    let pn = w.programs.len();
    let mut flow = vec![0.0f64; pn];
    for i in 0..w.people.len() {
        let p = &w.people[i];
        let Some(c) = p.college.filter(|_| p.stage == Stage::College) else { continue };
        if eligible(i) {
            flow[c] += (p.cal.lean * era).clamp(0.0, 1.0);
        }
    }
    let mut by: Vec<Vec<(usize, f64)>> = vec![Vec::new(); pn];
    for &(i, m) in cands {
        if let Some(c) = w.people[i].college
            && eligible(i)
        {
            by[c].push((i, m));
        }
    }
    let mut out = Vec::new();
    for (c, mut v) in by.into_iter().enumerate() {
        v.sort_by(|a, b| a.1.total_cmp(&b.1));
        let cap = (TOLD_SHARE * flow[c] / TOLD_GO).round() as usize;
        out.extend(v.into_iter().take(cap).map(|x| x.0));
    }
    out
}

/// The men the AI's staffs told, as each hears it.
pub(crate) fn mark_told(w: &mut World, told: &[usize]) {
    let today = w.today;
    for &i in told {
        w.people[i].cal.lean_events.push((today, "his coaches told him he isn't in the plans", 1.0));
    }
}

/// The window's calls by level and reason, with the market's spread
/// (`AFM_URGENCY_TRACE`, for `urgency_check`).
pub fn trace(w: &World, progs: &[usize], table: &growth::Table, marks: &LevelMarks) {
    let mut by: BTreeMap<(&'static str, String), usize> = BTreeMap::new();
    let mut markets: BTreeMap<&'static str, Vec<f64>> = BTreeMap::new();
    for &j in progs {
        let d = match w.programs[j].division {
            Division::FBS => "FBS",
            Division::FCS => "FCS",
            _ => "D2",
        };
        for c in ai_calls(w, j, table, marks) {
            *by.entry((d, format!("{:?} {:?}", c.mv, c.why))).or_default() += 1;
            markets.entry(d).or_default().push(c.market);
        }
    }
    eprintln!("urgency trace {}: {by:?}", w.today);
    for (d, mut v) in markets {
        let q: Vec<String> = [0.1, 0.5, 0.9].iter().map(|&x| format!("{:.2}", growth::quantile(&mut v, x))).collect();
        eprintln!("urgency trace {} {d} market p10/p50/p90: {}", w.today, q.join(" "));
    }
}

/// The spots a program's staff wants a man over the one it has (an AI
/// staff's "bring a man in over him"), added to its open starting spots in
/// the portal market.
pub(crate) fn over_at(w: &World, j: usize, k: usize) -> usize {
    w.folk.over.get(&(j as u16, k as u8)).copied().unwrap_or(0) as usize
}

/// Each program's factor on the window's lean draws, so its expected
/// entrants stay what they were: a told man's place in the flow replaces
/// draws, never adds to them (spec 5: "part of the real flow ... not added
/// on top"). `eligible` says who reaches the lean draw.
pub(crate) fn flow_factors(w: &World, told: &HashSet<usize>, era: f64, eligible: &dyn Fn(usize) -> bool) -> Vec<f64> {
    let pn = w.programs.len();
    let mut rest = vec![0.0f64; pn];
    let mut extra = vec![0.0f64; pn];
    for i in 0..w.people.len() {
        let p = &w.people[i];
        let Some(c) = p.college.filter(|_| p.stage == Stage::College) else { continue };
        if !eligible(i) {
            continue;
        }
        let prob = (p.cal.lean * era).clamp(0.0, 1.0);
        if told.contains(&i) {
            extra[c] += TOLD_GO - prob;
        } else {
            rest[c] += prob;
        }
    }
    (0..pn).map(|c| if rest[c] > 0.0 { ((rest[c] - extra[c]) / rest[c]).clamp(0.0, 1.0) } else { 1.0 }).collect()
}

/// A told man's draw: he enters the window, or stays to fight for a place.
pub(crate) fn told_goes(seed: u64, id: u32) -> bool {
    key_uniform(seed ^ 0x7011, id as u64) < TOLD_GO
}

/// Before fall camp: an AI program over its limit trims to it, the men
/// furthest under their bar first (spec 5).
pub(crate) fn camp_trim(w: &mut World) {
    if !ai_on() {
        return;
    }
    let seat = match Team::seat(w) {
        Some(Team::College(j)) => Some(j),
        _ => None,
    };
    let mut tables: Option<(growth::Table, LevelMarks)> = None;
    let today = w.today;
    let s = w.year;
    for j in 0..w.programs.len() {
        if Some(j) == seat || !w.programs[j].plays(s) {
            continue;
        }
        let n = over_limit(w, j);
        if n == 0 {
            continue;
        }
        let order = {
            if tables.is_none() {
                tables = Some((growth::table(w), LevelMarks::of(w)));
            }
            let (table, marks) = tables.as_ref().unwrap();
            let cs = ai_calls(w, j, table, marks);
            trim_order(w, j, &cs)
        };
        let aid_kept = crate::world::ncaa::aid_kept(&w.programs[j], s);
        let school = w.program_name(j).to_string();
        for i in order.into_iter().take(n) {
            let p = &mut w.people[i];
            p.cal.cut =
                Some(crate::world::CollegeCut { program: j as u16, date: today, aid_kept: aid_kept && !p.walk_on });
            p.cal.lean_events.push((today, "cut from the team", 1.0));
            let age = p.row_age();
            p.note(format!("Cut from {school} at {age}, to reach the roster limit before camp."));
            w.programs[j].roster.retain(|&x| x != i);
        }
    }
}

/// The calls a team's staff makes, counted by move, for the measure
/// (`urgency_check`).
pub fn tally(w: &World, j: usize, table: &growth::Table, marks: &LevelMarks) -> [usize; 3] {
    let cs = ai_calls(w, j, table, marks);
    let mut t = [0; 3];
    for c in &cs {
        t[match c.mv {
            Move::MoveOn => 0,
            Move::Over => 1,
            Move::Keep => 2,
        }] += 1;
    }
    t
}

/// What the stores keep between days: your word to the men you moved on
/// from (by id, the day), and the spots the AI's staffs want a man over.
pub type Told = BTreeMap<u32, Date>;
pub type Over = BTreeMap<(u16, u8), u8>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_man_out_of_time_with_better_in_the_portal_is_moved_on() {
        // A RS junior starter projected under the starter mark, one season
        // left, a market of two better men: move on.
        let (mv, why) = decide(Job::Starter, 60.0, 61.0, 1, 66.0, 58.0, 2.0, false, false);
        assert_eq!((mv, why), (Move::Over, Why::Room), "worth a backup's place");
        let (mv, why) = decide(Job::Starter, 55.0, 56.0, 1, 66.0, 58.0, 2.0, false, false);
        assert_eq!((mv, why), (Move::MoveOn, Why::OutOfTime));
        // The same man with a thin market: keep.
        let (mv, why) = decide(Job::Starter, 55.0, 56.0, 1, 66.0, 58.0, 0.2, false, false);
        assert_eq!((mv, why), (Move::Keep, Why::Thin));
        // A young man whose good case gets there: a year away.
        let (mv, why) = decide(Job::Starter, 63.0, 67.0, 3, 66.0, 58.0, 2.0, false, false);
        assert_eq!((mv, why), (Move::Keep, Why::YearAway));
        // A captain who won't start: bring a man in over him.
        let (mv, _) = decide(Job::Depth, 50.0, 51.0, 1, 58.0, 58.0, 2.0, true, false);
        assert_eq!(mv, Move::Over);
    }

    #[test]
    fn day_words_say_the_day() {
        let today = Date::ymd(2025, 12, 31);
        assert_eq!(day_words(today, Date::ymd(2026, 1, 2)), "January 2nd");
        assert_eq!(day_words(Date::ymd(2026, 1, 1), Date::ymd(2026, 1, 2)), "the 2nd");
    }
}
