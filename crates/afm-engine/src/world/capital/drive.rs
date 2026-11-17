//! The drive (Capital lane C3; `docs/design/capital.md`, CP11, CP12,
//! CP14-CP16, CP24, CP27): a real capital campaign for one building.
//!
//! The athletic director starts one in July when the department's money
//! and fans would keep its facilities above where they stand, or the fever
//! is up (the same rule at every program, CP27; it replaces the old July
//! decisions once the rooms live). It runs in its real stages:
//!
//! - **The study** (five months): the development office reads what the
//!   donors would give (`gifts::expected`) and says what size of building
//!   it supports (the full complex, mid-size or smaller), and who the lead
//!   donors are. You set the goal with the AD (CP15): the study's, or a
//!   bolder one.
//! - **The quiet phase**: the lead gifts (they need the head coach in the
//!   room, `gifts`) and the top half of the list. It goes public at half
//!   the goal (real campaigns wait for about that); you can go public
//!   earlier (CP16), and if the next season is a losing one the drive
//!   stalls in public.
//! - **The public phase**: everyone else and the many small gifts, to the
//!   goal or two years. A goal hit lifts the AD's trust in you; one missed
//!   by 15% or more is in the papers and costs it, more for a bold goal.
//! - **The building**: ground breaks at four fifths of the goal; you can
//!   push the AD to borrow to break ground at half, or to build bigger, at a
//!   later cost to your staff pool (CP11). A cost overrun comes up half a
//!   year in, and you choose: cut rooms, wait a year to raise more, or
//!   borrow (CP24).
//!
//! You can leave it to the development office (CP14): it asks every donor
//! on the same odds as an AI program's, but a lead donor gives it only a
//! courtesy gift; the lead gift that needed you in the room never comes.
//! On screen it is "the drive" (CP12). Sizes are Claude's (proposed, not
//! confirmed) unless the spec gives them.

use super::Capital;
use super::gifts;
use super::rooms::{self, Room};
use crate::calendar::{Date, Level, Loudness};
use crate::rng::clamp;
use crate::world::World;
use crate::world::finance::{self, Owner, Project, ProjectKind, dollars};
use crate::world::folk::Team;

/// A drive's stage.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum Stage {
    #[default]
    Study,
    Quiet,
    Public,
    /// The campaign is over (the building may still be going up).
    Closed,
}

impl Stage {
    pub fn words(self) -> &'static str {
        match self {
            Stage::Study => "the feasibility study",
            Stage::Quiet => "the quiet phase",
            Stage::Public => "the public phase",
            Stage::Closed => "closed",
        }
    }
}

/// The building's size (CP15's goal).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum Size {
    Small,
    #[default]
    Mid,
    Full,
}

impl Size {
    pub const ALL: [Size; 3] = [Size::Small, Size::Mid, Size::Full];
    /// Its share of the full building's cost and of its rooms' weight.
    pub fn share(self) -> f64 {
        match self {
            Size::Small => 0.5,
            Size::Mid => 0.75,
            Size::Full => 1.0,
        }
    }
    pub fn words(self) -> &'static str {
        match self {
            Size::Small => "a smaller building",
            Size::Mid => "a mid-size building",
            Size::Full => "the full complex",
        }
    }
    pub fn down(self) -> Option<Size> {
        match self {
            Size::Full => Some(Size::Mid),
            Size::Mid => Some(Size::Small),
            Size::Small => None,
        }
    }
    pub fn up(self) -> Option<Size> {
        match self {
            Size::Small => Some(Size::Mid),
            Size::Mid => Some(Size::Full),
            Size::Full => None,
        }
    }
}

/// Your answer to a cost overrun (CP24).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Fix {
    /// Drop rooms until the cost fits.
    CutRooms,
    /// Open a year later and raise the rest.
    Wait,
    /// Take bonds for it: a smaller staff pool for years.
    Borrow,
}

/// A cost overrun on the building (CP24).
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Overrun {
    /// Over the cost by this share.
    pub share: f32,
    /// The day it came up.
    pub found: Date,
    pub fix: Option<Fix>,
    /// Shown to you (your program's): the AD waits `ANSWER_DAYS` for you.
    pub told: bool,
}

/// A capital campaign for one building.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Drive {
    pub id: u32,
    pub program: u32,
    /// The building ("football building", "indoor practice facility").
    pub kind: String,
    /// The full building's cost when the study opened, in dollars of the day.
    pub full_cost: f64,
    pub size: Size,
    /// The study's size (None: it found the donors weren't there).
    pub advised: Option<Size>,
    /// The study's read of what donors would give the advised size.
    pub expected: f64,
    /// The goal: what the drive means to raise from donors.
    pub goal: f64,
    /// You set the goal (CP15).
    pub set_by_you: bool,
    pub stage: Stage,
    pub opened: Option<Date>,
    /// The day the stage began.
    pub since: Option<Date>,
    pub public_on: Option<Date>,
    /// Went public before half the goal was in (CP16).
    pub early: bool,
    /// Your program's drive with you working the lead donors yourself (off:
    /// the development office runs it, CP14).
    pub hands_on: bool,
    /// The many small gifts, in dollars.
    pub crowd: f64,
    /// Donors asked, with the stage they were asked in.
    pub asked: Vec<(u32, u8)>,
    /// The lead donors (their gift needs the head coach in the room).
    pub lead: Vec<u32>,
    /// Bonds pushed for past the AD's line (CP11, CP24's borrowing), in
    /// dollars: a part of their cost comes out of the staff pool.
    pub borrowed: f64,
    /// The money year ground broke and the day.
    pub ground: Option<i32>,
    pub ground_day: Option<Date>,
    /// The money year the building opens.
    pub opens: Option<i32>,
    /// The rooms it holds (after any cut).
    pub rooms: u16,
    pub overrun: Option<Overrun>,
    /// A drive that went public early and then lost: gifts come at half the
    /// rate until this day (CP16).
    pub stalled_until: Option<Date>,
    pub stall_checked: bool,
    /// A pulled pledge set the building back a year (CP5).
    pub set_back: bool,
    /// What was raised the day ground broke.
    pub broke_with: f64,
    pub closed: Option<Date>,
    /// The goal was hit (judged at the close).
    pub hit: Option<bool>,
    /// The drive ended without a building.
    pub shelved: bool,
    /// The last money year you pushed the AD to borrow (one ask a year).
    pub pushed: i32,
}

impl Drive {
    /// Months since a day.
    fn months(since: Option<Date>, today: Date) -> f64 {
        since.map_or(0.0, |s| s.days_to(today) as f64 / 30.44)
    }
    /// A bolder goal than the study's (CP15).
    pub fn bold(&self) -> bool {
        self.advised.is_none_or(|a| self.size > a)
    }
}

/// The mark in `Drive::asked` of a donor who has given to the drive (he is
/// not asked again).
pub const GAVE: u8 = 9;
/// The study's months (July to December).
pub const STUDY_MONTHS: f64 = 5.0;
/// The quiet phase goes public at this share of the goal (CP16: real
/// campaigns wait for about half).
pub const LAUNCH: f64 = 0.5;
/// Ground breaks at this share of the goal; pushing the AD to borrow
/// breaks it at `PUSH_BREAK` (CP11).
pub const BREAK: f64 = 0.8;
pub const PUSH_BREAK: f64 = 0.5;
/// The longest quiet and public phases, in months.
pub const QUIET_MAX: f64 = 24.0;
pub const PUBLIC_MAX: f64 = 24.0;
/// The study advises the biggest size whose goal its read covers this far.
pub const ADVISE: f64 = 0.9;
/// A lead donor: one of the three biggest whose gift would be this share of
/// the goal.
pub const LEAD_SHARE: f64 = 0.08;
pub const LEADS: usize = 3;
/// Each month the office asks each donor of the stage's list with this
/// chance; an AI head coach meets each lead donor he hasn't with `AI_MEET`.
pub const OFFICE_PACE: f64 = 0.25;
pub const AI_MEET: f64 = 0.2;
/// The many small gifts: this share of the goal, half at the launch and
/// half at the close, times (1 + fever) and (0.7 + 0.6 x the win share).
pub const CROWD: f64 = 0.08;
/// Gifts' chance while a drive is stalled in public (CP16).
pub const STALL: f64 = 0.5;
/// The share of a building's cost the drive raises from donors, by tier
/// (Power 4, the rest of the FBS, FCS, below): the department's bonds
/// and the university carry the rest (smaller programs lean on them more).
pub const FUND_SHARE: [f64; 4] = [0.75, 0.4, 0.12, 0.06];
/// A cost overrun: this chance at the groundbreaking, 5% to 30% over.
pub const OVERRUN: f64 = 0.35;
/// The bonds' yearly cost (as the books' debt line) and its years, and the
/// share of a pushed bond's cost that comes out of football's staff pool,
/// at most a quarter of it (CP11: about a tenth for a full building's
/// bonds at the Power 4).
pub const BOND_RATE: f64 = 0.075;
pub const BOND_YEARS: i32 = 22;
pub const STAFF_SHARE: f64 = 0.5;
pub const STAFF_CUT_MAX: f64 = 0.25;
/// Days the AD waits for your answer to an overrun.
pub const ANSWER_DAYS: i32 = 30;
/// The AD's trust (CP15): a goal hit, a bold one hit, a miss of 15% or
/// more, a bold one missed; and a drive stalled in public (CP16).
pub const TRUST_HIT: f32 = 0.06;
pub const TRUST_BOLD_HIT: f32 = 0.04;
pub const TRUST_MISS: f32 = -0.05;
pub const TRUST_BOLD_MISS: f32 = -0.05;
pub const TRUST_STALL: f32 = -0.04;
pub const MISS: f64 = 0.15;
/// A drive starts in July: 4% plus 0.7 a point of the AD's target over the
/// facilities number, plus 0.2 x the fever, plus the department's money
/// and a big booster, at most 45%.
pub const START_BASE: f64 = 0.04;
pub const START_GAP: f64 = 0.7;
pub const START_FEVER: f64 = 0.2;
/// Years a program waits after a shelved drive.
pub const SHELVED_WAIT: i32 = 3;

/// The building a drive raises for, by tier, and its full cost in 2024
/// dollars (the old July decisions' buildings).
pub fn kind_for(tier: u8) -> (&'static str, f64) {
    match tier {
        0 => ("football building", 140.0e6),
        1 => ("football operations center", 40.0e6),
        2 => ("indoor practice facility", 14.0e6),
        _ => ("weight room and locker room", 3.0e6),
    }
}

/// The rooms a building of `kind` holds at a size: the full building's
/// rooms, the biggest first, to the size's share of their weight (at least
/// one).
pub fn rooms_for(p: &crate::world::program::Program, kind: &str, size: Size) -> u16 {
    let mask = rooms::project_rooms(p, kind);
    let mut have: Vec<Room> = Room::ALL.into_iter().filter(|r| mask & r.bit() != 0).collect();
    have.sort_by(|a, b| b.weight().total_cmp(&a.weight()).then(a.cmp(b)));
    let total: f64 = have.iter().map(|r| r.weight()).sum();
    let mut out = 0u16;
    let mut kept = 0.0;
    for r in have {
        if out != 0 && kept >= size.share() * total - 1e-9 {
            break;
        }
        out |= r.bit();
        kept += r.weight();
    }
    out
}

fn tier(w: &World, p: usize) -> u8 {
    finance::college::tier(w, p, w.year)
}

/// The goal for a size.
pub fn goal_for(w: &World, d: &Drive, size: Size) -> f64 {
    d.full_cost * size.share() * FUND_SHARE[tier(w, d.program as usize).min(3) as usize]
}

/// The program's drive whose campaign is open (not closed), if any.
pub fn open_drive(cap: &Capital, p: usize) -> Option<usize> {
    cap.drives.iter().rposition(|d| d.program == p as u32 && d.stage != Stage::Closed)
}

/// The program's latest drive, open or not.
pub fn latest(cap: &Capital, p: usize) -> Option<&Drive> {
    cap.drives.iter().rev().find(|d| d.program == p as u32)
}

/// What a drive has raised: every pledge to it (a pulled one only what it
/// paid) and the small gifts.
pub fn raised(cap: &Capital, d: &Drive) -> f64 {
    cap.pledges.iter().filter(|x| x.drive == d.id).map(|x| x.paid + x.left()).sum::<f64>() + d.crowd
}

fn busy(w: &World, p: usize) -> bool {
    w.finance.projects.iter().any(|x| x.owner == Owner::College(p) && !x.done)
}

/// A drive begins: the study opens (for you, the development office runs
/// it until you take it over, CP14).
pub(crate) fn start(w: &mut World, p: usize) -> Option<u32> {
    if open_drive(&w.capital, p).is_some() || busy(w, p) {
        return None;
    }
    let year = w.year;
    let (kind, base) = kind_for(tier(w, p));
    let noise = 0.85 + 0.3 * super::draw(w, 30, (p as u64) << 16 ^ year as u64);
    let full_cost = base * finance::data::college_index(year) * noise;
    let id = w.capital.id();
    let today = w.today;
    let rooms = rooms_for(&w.programs[p], kind, Size::Mid);
    let mut d = Drive {
        id,
        program: p as u32,
        kind: kind.to_string(),
        full_cost,
        size: Size::Mid,
        stage: Stage::Study,
        opened: Some(today),
        since: Some(today),
        rooms,
        ..Default::default()
    };
    d.goal = goal_for(w, &d, Size::Mid);
    w.capital.drives.push(d);
    if w.seat_program(p) {
        let name = w.programs[p].name(w.options.names).to_string();
        let text = format!(
            "{name} opens a feasibility study for a new {kind}: the development office reads what the donors would give before the drive is set."
        );
        say(w, p, Loudness::Medium, None, text);
    }
    Some(id)
}

/// A news line: your program's at `mine`, a Power 4 program's at `p4`.
fn say(w: &mut World, p: usize, mine: Loudness, p4: Option<Loudness>, text: String) {
    let prog = &w.programs[p];
    let loud = if w.seat_program(p) {
        mine
    } else if let Some(l) =
        p4.filter(|_| prog.division == crate::world::data::Division::FBS && prog.power_conference(w.year))
    {
        l
    } else {
        return;
    };
    let e = finance::story(w, Level::College, loud, text).program(p);
    finance::post(w, e);
}

/// Each July, once the rooms live (CP27, in place of the old decisions):
/// pledges pay their year, and each program's AD weighs a new drive on the
/// same rule; an AI drive weighs pushing to borrow.
pub(crate) fn year(w: &mut World) {
    gifts::pay(&mut w.capital);
    let year = w.year;
    let fevers = super::fever::all(w);
    let n = w.finance.college.len().min(w.programs.len());
    for p in 0..n {
        let prog = &w.programs[p];
        if !prog.plays(year) || !prog.campus.live || prog.division == crate::world::data::Division::JUCO {
            continue;
        }
        // An AI drive that has waited a year with half its goal in, and the
        // fever up or money in the bank, pushes to break ground (CP11).
        if let Some(k) = open_drive(&w.capital, p)
            && !w.seat_program(p)
        {
            let d = &w.capital.drives[k];
            let reserves = w.finance.college.get(p).map_or(0.0, |c| c.reserves as f64);
            if d.ground.is_none()
                && d.stage >= Stage::Quiet
                && Drive::months(d.opened, w.today) >= 12.0
                && raised(&w.capital, d) >= PUSH_BREAK * d.goal
                && (fevers[p] >= 0.3 || reserves > 0.0)
                && super::draw(w, 31, (d.id as u64) << 16 ^ year as u64) < 0.3
            {
                break_ground(w, k, true);
            }
            continue;
        }
        if busy(w, p) || open_drive(&w.capital, p).is_some() {
            continue;
        }
        // A shelved drive waits three years for another try.
        if latest(&w.capital, p).is_some_and(|d| d.shelved && d.closed.is_some_and(|c| c.year() + SHELVED_WAIT > year))
        {
            continue;
        }
        let prog = &w.programs[p];
        let gap = rooms::ad_target(prog) - prog.facilities;
        let fever = fevers[p];
        if gap <= 0.02 && fever < 0.3 {
            continue;
        }
        let reserves = w.finance.college.get(p).map_or(0.0, |c| c.reserves as f64);
        let big = prog.collective.big_booster.is_some();
        let chance = clamp(
            START_BASE
                + START_GAP * gap.max(0.0)
                + START_FEVER * fever
                + if big { 0.05 } else { 0.0 }
                + clamp(reserves / 40.0e6, -0.05, 0.1),
            0.0,
            0.45,
        );
        if super::draw(w, 32, (p as u64) << 16 ^ year as u64) < chance {
            start(w, p);
        }
    }
}

/// The first of each month: every open drive takes its month's step, and
/// every building under way is watched.
pub(crate) fn month(w: &mut World, d: Date) {
    gifts::estates(w);
    let fevers = super::fever::all(w);
    let by_program = donors_by_program(w);
    for k in 0..w.capital.drives.len() {
        let x = &w.capital.drives[k];
        let p = x.program as usize;
        // A closed drive whose building waits on a project under way still
        // breaks ground when it can.
        if x.stage != Stage::Closed || x.ground.is_none() && !x.shelved {
            step(w, k, d, fevers[p], &by_program[p]);
        }
        let x = &w.capital.drives[k];
        if x.ground.is_some() && !x.shelved && x.opens.is_some_and(|o| o >= w.year) {
            building(w, k, d, fevers[p]);
        }
    }
    prune(w);
}

/// Every program's living donors (indexes in the store), its own and those
/// with ties to it.
fn donors_by_program(w: &World) -> Vec<Vec<usize>> {
    let mut v: Vec<Vec<usize>> = vec![Vec::new(); w.programs.len()];
    for (k, d) in w.capital.donors.iter().enumerate().filter(|(_, d)| d.alive()) {
        if let Some(x) = v.get_mut(d.home as usize) {
            x.push(k);
        }
        if let Some(q) = d.also
            && let Some(x) = v.get_mut(q as usize)
        {
            x.push(k);
        }
    }
    v
}

/// One drive's month.
fn step(w: &mut World, k: usize, d: Date, fever: f64, list: &[usize]) {
    match w.capital.drives[k].stage {
        Stage::Study => {
            if Drive::months(w.capital.drives[k].since, d) >= STUDY_MONTHS {
                study(w, k, fever, list);
            }
        }
        Stage::Quiet | Stage::Public => {
            asks(w, k, d, fever, list);
            stage_checks(w, k, d, fever);
        }
        Stage::Closed => {}
    }
    let x = &w.capital.drives[k];
    if x.ground.is_none() && x.stage >= Stage::Quiet && !x.shelved && raised(&w.capital, x) >= BREAK * x.goal {
        break_ground(w, k, false);
    }
}

/// The study's report (CP15): what the donors would give each size, the
/// advised size and the lead donors. Pledges and estates left from an
/// earlier drive join this one.
fn study(w: &mut World, k: usize, fever: f64, list: &[usize]) {
    let today = w.today;
    let (p, id) = (w.capital.drives[k].program as usize, w.capital.drives[k].id);
    // Money left from a drive that never built (drive 0) joins this one.
    for x in w.capital.pledges.iter_mut().filter(|x| x.program == p as u32 && x.drive == 0 && x.pulled.is_none()) {
        x.drive = id;
    }
    let coach = w.programs[p].head_coach.map(|c| c as u32);
    let carried: f64 = raised(&w.capital, &w.capital.drives[k]);
    let wp = gifts::wins(w, p);
    let read = |w: &World, size: Size| -> f64 {
        let d = &w.capital.drives[k];
        let rooms = rooms_for(&w.programs[p], &d.kind, size);
        let donors: f64 = list.iter().map(|&j| gifts::expected(w, p, &w.capital.donors[j], rooms, coach, fever)).sum();
        donors + carried + CROWD * goal_for(w, d, size) * (1.0 + fever) * (0.7 + 0.6 * wp)
    };
    let mut advised = None;
    let mut expected = 0.0;
    for s in Size::ALL.into_iter().rev() {
        let e = read(w, s);
        if e >= ADVISE * goal_for(w, &w.capital.drives[k], s) {
            advised = Some(s);
            expected = e;
            break;
        }
        expected = e;
    }
    let mine = w.seat_program(p);
    let set = w.capital.drives[k].set_by_you;
    let mut size = if set { w.capital.drives[k].size } else { advised.unwrap_or(Size::Small) };
    // An AI program on a fever sometimes goes bolder than the study (CP15,
    // the same choice you have).
    if !mine
        && !set
        && fever >= 0.5
        && let Some(up) = size.up()
        && super::draw(w, 33, id as u64) < 0.3
    {
        size = up;
    }
    // The lead donors: the three biggest gifts the study sees, if each is a
    // real share of the goal.
    let rooms = rooms_for(&w.programs[p], &w.capital.drives[k].kind, size);
    let goal = goal_for(w, &w.capital.drives[k], size);
    let mut by: Vec<(f64, u32)> = list
        .iter()
        .map(|&j| {
            let o = gifts::odds(w, p, &w.capital.donors[j], rooms, coach, fever);
            (o.share * w.capital.donors[j].capacity, w.capital.donors[j].id)
        })
        .filter(|x| x.0 >= LEAD_SHARE * goal)
        .collect();
    by.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let lead: Vec<u32> = by.into_iter().take(LEADS).map(|x| x.1).collect();
    let shelve = advised.is_none() && !mine && !set;
    let dr = &mut w.capital.drives[k];
    dr.advised = advised;
    dr.expected = expected;
    dr.size = size;
    dr.goal = goal;
    dr.rooms = rooms;
    dr.lead = lead;
    if shelve {
        // The study says not now: an AI program shelves it.
        dr.stage = Stage::Closed;
        dr.closed = Some(today);
        dr.shelved = true;
        return;
    }
    dr.stage = Stage::Quiet;
    dr.since = Some(today);
    if mine {
        let name = w.programs[p].name(w.options.names).to_string();
        let study = match advised {
            Some(a) => format!("the study says the donors are there for {}", a.words()),
            None => "the study says the donors aren't there yet".to_string(),
        };
        let text = format!(
            "{name}'s drive goes into its quiet phase: {study}. The goal is {} for {}.",
            dollars(goal),
            size.words()
        );
        say(w, p, Loudness::Medium, None, text);
    }
}

/// The month's asks: the development office works the stage's list; an AI
/// head coach meets his lead donors; yours wait for you unless the office
/// runs the drive.
fn asks(w: &mut World, k: usize, d: Date, fever: f64, list: &[usize]) {
    let (p, id, stage, rooms) = {
        let x = &w.capital.drives[k];
        (x.program as usize, x.id, x.stage, x.rooms)
    };
    let mark = stage as u8;
    let mine = w.seat_program(p);
    let hands_on = mine && w.capital.drives[k].hands_on;
    let stalled = w.capital.drives[k].stalled_until.is_some_and(|s| d < s);
    let coach = w.programs[p].head_coach.map(|c| c as u32);
    // The quiet phase works the top half of the list.
    let mut caps: Vec<f64> = list.iter().map(|&j| w.capital.donors[j].capacity).collect();
    caps.sort_by(f64::total_cmp);
    let median = caps.get(caps.len() / 2).copied().unwrap_or(0.0);
    let month = (d.year() as u64) << 4 ^ d.month() as u64;
    let mut gifts_now: Vec<(u32, f64, Option<u32>, Option<u8>)> = Vec::new();
    for &j in list {
        let don = &w.capital.donors[j];
        let did = don.id;
        let asked = &w.capital.drives[k].asked;
        if asked.contains(&(did, mark)) || asked.contains(&(did, GAVE)) {
            continue;
        }
        if stage == Stage::Quiet && don.capacity < median {
            continue;
        }
        let lead = w.capital.drives[k].lead.contains(&did);
        let key = (did as u64) << 24 ^ (id as u64) << 4 ^ mark as u64;
        let (who, courtesy) = if lead {
            if mine {
                // Your lead donors wait for you; the office alone gets a
                // courtesy gift (CP14).
                if hands_on {
                    continue;
                }
                (None, true)
            } else {
                // An AI head coach meets them through the drive.
                if super::draw(w, 34, key ^ month << 40) >= AI_MEET {
                    continue;
                }
                (coach, false)
            }
        } else {
            if super::draw(w, 35, key ^ month << 40) >= OFFICE_PACE {
                continue;
            }
            (None, false)
        };
        let o = gifts::odds(w, p, don, rooms, who, fever);
        let chance = o.chance * if stalled { STALL } else { 1.0 };
        let yes = super::draw(w, 36, key) < chance;
        let room = don.rooms.iter().copied().find(|&r| rooms & (1u16 << r) != 0);
        let amount = o.share * don.capacity * if courtesy { gifts::COURTESY } else { 1.0 };
        w.capital.drives[k].asked.push((did, mark));
        if yes {
            gifts_now.push((did, amount, who, room));
        }
    }
    for (did, amount, who, room) in gifts_now {
        if let Some(c) = who {
            super::donors::met(w, did, c, d);
        }
        gifts::give(w, id, p, did, amount, who, false, room);
    }
}

/// The stage's turns: going public, the quiet phase running long, the
/// close, and an early launch that stalls (CP16).
fn stage_checks(w: &mut World, k: usize, d: Date, fever: f64) {
    let (p, id) = (w.capital.drives[k].program as usize, w.capital.drives[k].id);
    let mine = w.seat_program(p);
    let raised_now = raised(&w.capital, &w.capital.drives[k]);
    let x = w.capital.drives[k].clone();
    let months = Drive::months(x.since, d);
    match x.stage {
        Stage::Quiet => {
            let ready = raised_now >= LAUNCH * x.goal;
            // An AI program launches on the fever with a quarter in, half
            // the time (the same choice you have).
            let early = !mine && fever >= 0.5 && raised_now >= 0.25 * x.goal && super::draw(w, 37, id as u64) < 0.5;
            if ready || early {
                launch(w, k, !ready);
            } else if months >= QUIET_MAX {
                if raised_now >= 0.35 * x.goal {
                    launch(w, k, true);
                } else if let Some(s) = x.size.down() {
                    let p_ = &w.programs[p];
                    let rooms = rooms_for(p_, &x.kind, s);
                    let goal = goal_for(w, &x, s);
                    let dr = &mut w.capital.drives[k];
                    dr.size = s;
                    dr.rooms = rooms;
                    dr.goal = goal;
                    dr.since = Some(d.add(-365));
                } else {
                    shelve(w, k);
                }
            }
        }
        Stage::Public => {
            if !x.stall_checked && x.early {
                stall_check(w, k);
            }
            if raised_now >= x.goal || months >= PUBLIC_MAX {
                close(w, k, fever);
            }
        }
        _ => {}
    }
}

/// The public launch (CP16: `early` before half the goal was in).
fn launch(w: &mut World, k: usize, early: bool) {
    let today = w.today;
    let p = w.capital.drives[k].program as usize;
    let fever = super::fever::fever(w, p);
    let wp = gifts::wins(w, p);
    let x = &mut w.capital.drives[k];
    x.stage = Stage::Public;
    x.since = Some(today);
    x.public_on = Some(today);
    x.early = early;
    x.crowd += 0.5 * CROWD * x.goal * (1.0 + fever) * (0.7 + 0.6 * wp);
    let (goal, size, kind) = (x.goal, x.size, x.kind.clone());
    let raised_now = raised(&w.capital, &w.capital.drives[k]);
    let name = w.programs[p].name(w.options.names).to_string();
    let text = format!(
        "{name} launches its drive in public: {} for {} ({kind}), with {:.0}% already pledged.",
        dollars(goal),
        size.words(),
        100.0 * raised_now / goal.max(1.0)
    );
    say(w, p, Loudness::Big, Some(Loudness::Small), text);
}

/// A drive that went public early: if the next season is a losing one, it
/// stalls in public (CP16).
fn stall_check(w: &mut World, k: usize) {
    let p = w.capital.drives[k].program as usize;
    let Some(on) = w.capital.drives[k].public_on else { return };
    // The season played after the launch.
    let season = if on.month() >= 9 { on.year() + 1 } else { on.year() };
    let Some(s) = w.programs[p].seasons.iter().find(|s| s.year == season) else { return };
    let wp = s.wins as f64 / (s.wins + s.losses).max(1) as f64;
    w.capital.drives[k].stall_checked = true;
    if wp >= 0.5 {
        return;
    }
    let today = w.today;
    w.capital.drives[k].stalled_until = Some(today.add(365));
    let name = w.programs[p].name(w.options.names).to_string();
    let text = format!(
        "{name}'s drive stalls in public: it launched before the lead gifts were in, and a {}-{} season has donors waiting.",
        s.wins, s.losses
    );
    say(w, p, Loudness::Big, Some(Loudness::Small), text);
    if w.seat_program(p) {
        let b = crate::world::folk::boss::boss(w, Team::College(p));
        w.folk.mine.nudge(b.key(), TRUST_STALL);
    }
}

/// The close (CP15): the goal hit or missed, the AD's trust and the press;
/// a drive short of its building's line steps down to the size it can
/// build, or is shelved.
fn close(w: &mut World, k: usize, fever: f64) {
    let today = w.today;
    let p = w.capital.drives[k].program as usize;
    let wp = gifts::wins(w, p);
    {
        let x = &mut w.capital.drives[k];
        x.crowd += 0.5 * CROWD * x.goal * (1.0 + fever) * (0.7 + 0.6 * wp);
        x.stage = Stage::Closed;
        x.closed = Some(today);
    }
    let x = w.capital.drives[k].clone();
    let got = raised(&w.capital, &x);
    let hit = got >= x.goal;
    let short = 1.0 - got / x.goal.max(1.0);
    w.capital.drives[k].hit = Some(hit);
    let name = w.programs[p].name(w.options.names).to_string();
    let mine = w.seat_program(p);
    if hit {
        let text = format!("{name}'s drive closes over its goal: {} raised for {}.", dollars(got), x.size.words());
        say(w, p, Loudness::Big, None, text);
    } else if short >= MISS {
        let text =
            format!("{name}'s drive closes {:.0}% short of the {} it announced.", 100.0 * short, dollars(x.goal));
        say(w, p, Loudness::Big, Some(Loudness::Small), text);
    } else if mine {
        let text = format!("{name}'s drive closes just short: {} of {}.", dollars(got), dollars(x.goal));
        say(w, p, Loudness::Medium, None, text);
    }
    if mine {
        let by = if hit {
            TRUST_HIT + if x.bold() { TRUST_BOLD_HIT } else { 0.0 }
        } else if short >= MISS {
            TRUST_MISS + if x.bold() { TRUST_BOLD_MISS } else { 0.0 }
        } else {
            0.0
        };
        if by != 0.0 {
            let b = crate::world::folk::boss::boss(w, Team::College(p));
            w.folk.mine.nudge(b.key(), by);
        }
    }
    if x.ground.is_some() {
        return;
    }
    // Short of the line for its size: the biggest size the money builds.
    let mut size = Some(x.size);
    while let Some(s) = size {
        if got >= BREAK * goal_for(w, &x, s) {
            let rooms = rooms_for(&w.programs[p], &x.kind, s);
            let goal = goal_for(w, &x, s);
            let dr = &mut w.capital.drives[k];
            dr.size = s;
            dr.rooms = rooms;
            dr.goal = goal;
            break_ground(w, k, false);
            return;
        }
        size = s.down();
    }
    shelve(w, k);
}

/// A drive ends without a building: what was pledged waits for the next
/// one (drive 0).
fn shelve(w: &mut World, k: usize) {
    let today = w.today;
    let (id, p) = (w.capital.drives[k].id, w.capital.drives[k].program as usize);
    let x = &mut w.capital.drives[k];
    x.stage = Stage::Closed;
    x.closed.get_or_insert(today);
    x.shelved = true;
    for y in w.capital.pledges.iter_mut().filter(|y| y.drive == id) {
        y.drive = 0;
    }
    if w.seat_program(p) {
        let name = w.programs[p].name(w.options.names).to_string();
        let text = format!("{name}'s drive is shelved: the pledges it raised wait for the next one.");
        say(w, p, Loudness::Big, None, text);
    }
}

/// The months a building takes, by kind.
pub fn build_days(kind: &str) -> i32 {
    match kind {
        "indoor practice facility" => 450,
        "weight room and locker room" => 330,
        _ => 730,
    }
}

/// Ground breaks: the project goes in the books with what is pledged, the
/// rest in bonds; `pushed` breaks it before the AD's line, the bonds past
/// it costing the staff pool (CP11). A cost overrun is drawn (CP24).
pub(crate) fn break_ground(w: &mut World, k: usize, pushed: bool) {
    let today = w.today;
    let year = w.year;
    let x = w.capital.drives[k].clone();
    let p = x.program as usize;
    if busy(w, p) {
        return;
    }
    let cost = x.full_cost * x.size.share();
    let got = raised(&w.capital, &x);
    let borrowed = if pushed { (BREAK * x.goal - got).max(0.0) } else { 0.0 };
    let done = today.add(build_days(&x.kind));
    let opens = done.year() + if done.month() > 7 { 1 } else { 0 };
    let step = rooms::rise_rooms(&w.programs[p], &x.kind, cost, year, x.rooms);
    w.finance.projects.push(Project {
        owner: Owner::College(p),
        kind: ProjectKind::Facility,
        name: x.kind.clone(),
        cost: cost as f32,
        public: 0.0,
        pledged: got.min(cost) as f32,
        started: year,
        opens,
        capacity: 0,
        step: step as f32,
        done: false,
        asked: w.seat_program(p) && x.set_by_you,
    });
    let over = super::draw(w, 38, x.id as u64) < OVERRUN;
    let share = (0.05 + 0.25 * super::draw(w, 39, x.id as u64).powi(2)) as f32;
    {
        let dr = &mut w.capital.drives[k];
        dr.ground = Some(year);
        dr.ground_day = Some(today);
        dr.broke_with = got;
        dr.opens = Some(opens);
        dr.borrowed += borrowed;
        dr.overrun = over.then_some(Overrun { share, found: today.add(180), fix: None, told: false });
    }
    let name = w.programs[p].name(w.options.names).to_string();
    let how = if pushed { ", the athletic department borrowing the rest" } else { "" };
    let text = format!(
        "{name} breaks ground on {} ({}), opening in {opens}: donors have pledged {}{how}.",
        x.size.words(),
        x.kind,
        dollars(got)
    );
    say(w, p, Loudness::Big, Some(Loudness::Small), text);
    // Recruits see the building from the day ground breaks.
    crate::world::staff::facilities_grade_now(w, p);
}

/// The project in the books for a drive's building.
pub fn project_of(w: &World, x: &Drive) -> Option<usize> {
    let g = x.ground?;
    w.finance.projects.iter().rposition(|y| {
        y.owner == Owner::College(x.program as usize)
            && y.name == x.kind
            && y.started == g
            && y.kind == ProjectKind::Facility
    })
}

/// A building under way, each month: the books follow the pledges (a pulled
/// one is bonds now, and one that leaves it well short sets it back a
/// year, CP5), and a cost overrun comes up and is answered (CP24).
fn building(w: &mut World, k: usize, d: Date, fever: f64) {
    let x = w.capital.drives[k].clone();
    let p = x.program as usize;
    let Some(j) = project_of(w, &x) else { return };
    if w.finance.projects[j].done {
        return;
    }
    let got = raised(&w.capital, &x);
    let cost = w.finance.projects[j].cost as f64;
    w.finance.projects[j].pledged = got.min(cost) as f32;
    if !x.set_back && got < 0.8 * x.broke_with {
        w.capital.drives[k].set_back = true;
        w.finance.projects[j].opens += 1;
        w.capital.drives[k].opens = Some(w.finance.projects[j].opens);
        let name = w.programs[p].name(w.options.names).to_string();
        let text = format!("{name}'s {} stalls: pledges were pulled, and it opens a year later.", x.kind);
        say(w, p, Loudness::Big, Some(Loudness::Small), text);
    }
    let Some(o) = x.overrun.filter(|o| o.fix.is_none() && d >= o.found) else { return };
    let mine = w.seat_program(p);
    if mine && !o.told {
        if let Some(r) = w.capital.drives[k].overrun.as_mut() {
            r.told = true;
        }
        let name = w.programs[p].name(w.options.names).to_string();
        let text = format!(
            "{name}'s {} is running {:.0}% over: cut rooms, wait a year to raise more, or borrow. The AD waits on your call.",
            x.kind,
            100.0 * o.share
        );
        say(w, p, Loudness::Big, None, text);
        return;
    }
    if mine && o.found.days_to(d) < ANSWER_DAYS {
        return;
    }
    // The AD's own call (yours if you didn't make one): wait.
    let fix = if mine {
        Fix::Wait
    } else {
        let several = x.rooms.count_ones() > 1;
        let reserves = w.finance.college.get(p).map_or(0.0, |c| c.reserves as f64);
        if o.share >= 0.15 && several {
            Fix::CutRooms
        } else if reserves > 0.0 || fever >= 0.3 {
            Fix::Borrow
        } else {
            Fix::Wait
        }
    };
    apply_fix(w, k, fix);
}

/// The rooms an overrun of `share` would cut from a drive's building (the
/// smallest first, until the cut covers it, at least one room kept): what is
/// left and what goes. The screen shows it before you choose (CS4).
pub fn cut_preview(x: &Drive, share: f32) -> (u16, Vec<Room>) {
    let mut have: Vec<Room> = Room::ALL.into_iter().filter(|r| x.rooms & r.bit() != 0).collect();
    have.sort_by(|a, b| a.weight().total_cmp(&b.weight()).then(b.cmp(a)));
    let total: f64 = have.iter().map(|r| r.weight()).sum();
    let mut rooms = x.rooms;
    let mut cut = 0.0;
    let mut dropped: Vec<Room> = Vec::new();
    for r in have {
        if cut >= share as f64 * total || rooms.count_ones() <= 1 {
            break;
        }
        rooms &= !r.bit();
        cut += r.weight();
        dropped.push(r);
    }
    (rooms, dropped)
}

fn apply_fix(w: &mut World, k: usize, fix: Fix) -> String {
    let x = w.capital.drives[k].clone();
    let p = x.program as usize;
    let Some(o) = x.overrun else { return String::new() };
    let Some(j) = project_of(w, &x) else { return String::new() };
    let over = w.finance.projects[j].cost as f64 * o.share as f64;
    let name = w.programs[p].name(w.options.names).to_string();
    let text = match fix {
        Fix::CutRooms => {
            let (rooms, dropped) = cut_preview(&x, o.share);
            let dropped: Vec<&str> = dropped.iter().map(|r| r.words()).collect();
            w.capital.drives[k].rooms = rooms;
            let year = w.year;
            let cost = w.finance.projects[j].cost as f64;
            w.finance.projects[j].step = rooms::rise_rooms(&w.programs[p], &x.kind, cost, year, rooms) as f32;
            format!("{name} cuts the {} from its {} to stay on budget.", dropped.join(" and the "), x.kind)
        }
        Fix::Wait => {
            w.finance.projects[j].cost += over as f32;
            w.finance.projects[j].opens += 1;
            let opens = w.finance.projects[j].opens;
            let today = w.today;
            let dr = &mut w.capital.drives[k];
            dr.opens = Some(opens);
            dr.goal += over;
            // The campaign reopens for a year to raise it.
            if dr.stage == Stage::Closed {
                dr.stage = Stage::Public;
                dr.since = Some(today.add(-(30.44 * (PUBLIC_MAX - 12.0)) as i32));
                dr.closed = None;
                dr.hit = None;
            }
            format!("{name}'s {} will open a year later, in {opens}, while the drive raises the overrun.", x.kind)
        }
        Fix::Borrow => {
            w.finance.projects[j].cost += over as f32;
            w.capital.drives[k].borrowed += over;
            format!("{name} borrows to cover the {}'s overrun; the staff pool will feel it for years.", x.kind)
        }
    };
    if let Some(r) = w.capital.drives[k].overrun.as_mut() {
        r.fix = Some(fix);
    }
    say(w, p, Loudness::Big, Some(Loudness::Small), text.clone());
    text
}

/// What borrowing costs football's staff pool this money year (CP11): a
/// share of the pushed bonds' yearly cost against the pool's line, at most
/// a quarter of it.
pub fn staff_cut(w: &World, p: usize, year: i32, staff_line: f64) -> f64 {
    let service: f64 = w
        .capital
        .drives
        .iter()
        .filter(|d| d.program == p as u32 && d.borrowed > 0.0)
        .filter(|d| d.ground.is_some_and(|g| g <= year && g + BOND_YEARS > year))
        .map(|d| d.borrowed * BOND_RATE)
        .sum();
    if service <= 0.0 || staff_line <= 0.0 { 0.0 } else { (STAFF_SHARE * service / staff_line).min(STAFF_CUT_MAX) }
}

/// Old drives, pledges and calls go once nothing needs them.
fn prune(w: &mut World) {
    let year = w.year;
    w.capital.drives.retain(|d| {
        d.stage != Stage::Closed
            || d.closed.is_none_or(|c| c.year() + 3 > year)
            || d.ground.is_some_and(|g| g + BOND_YEARS > year)
    });
    let keep: std::collections::HashSet<u32> = w.capital.drives.iter().map(|d| d.id).collect();
    w.capital
        .pledges
        .retain(|x| x.open() || x.drive == 0 || keep.contains(&x.drive) || x.made.is_some_and(|m| m.year() + 6 > year));
    w.capital.catches.retain(|c| c.answer.is_none() || c.called.is_some_and(|x| x.year() + 2 > year));
}

// ---------------------------------------------------------------------------
// Your calls
// ---------------------------------------------------------------------------

fn yours(w: &World, p: usize) -> Result<usize, String> {
    if !w.seat_program(p) {
        return Err("Not your program.".into());
    }
    open_drive(&w.capital, p).ok_or_else(|| "There's no drive under way.".to_string())
}

/// You set the drive's goal with the AD (CP15): any size, before it goes
/// public. Bolder than the study's is a public bet.
pub fn set_goal(w: &mut World, p: usize, size: Size) -> Result<String, String> {
    let k = yours(w, p)?;
    let x = w.capital.drives[k].clone();
    if x.stage >= Stage::Public {
        return Err("The goal is public already.".into());
    }
    let rooms = rooms_for(&w.programs[p], &x.kind, size);
    let goal = goal_for(w, &x, size);
    let dr = &mut w.capital.drives[k];
    dr.size = size;
    dr.rooms = rooms;
    dr.goal = goal;
    dr.set_by_you = true;
    let bold = dr.advised.is_some_and(|a| size > a) || dr.stage == Stage::Quiet && dr.advised.is_none();
    let warn = if bold { " It's more than the study says the donors are there for." } else { "" };
    Ok(format!("The goal is {} for {}.{warn}", dollars(goal), size.words()))
}

/// You take the drive public now (CP16): before half the goal is in it is
/// early, and a losing season after it stalls the drive in public.
pub fn go_public(w: &mut World, p: usize) -> Result<String, String> {
    let k = yours(w, p)?;
    if w.capital.drives[k].stage != Stage::Quiet {
        return Err("The drive isn't in its quiet phase.".into());
    }
    let got = raised(&w.capital, &w.capital.drives[k]);
    let early = got < LAUNCH * w.capital.drives[k].goal;
    launch(w, k, early);
    Ok(if early {
        "The drive is public. The lead gifts aren't all in: a bad season now would show.".into()
    } else {
        "The drive is public.".into()
    })
}

/// You work the lead donors yourself, or leave the drive to the development
/// office (CP14).
pub fn hands_on(w: &mut World, p: usize, on: bool) -> Result<String, String> {
    let k = yours(w, p)?;
    w.capital.drives[k].hands_on = on;
    Ok(if on {
        "You'll work the lead donors yourself.".into()
    } else {
        "The development office runs the drive. The lead donors get a call from the office, not from you.".into()
    })
}

/// You push the AD to borrow (CP11): to break ground now at half the goal,
/// or to build a size bigger. Bonds past the AD's line cost the staff pool
/// for years. One ask a money year.
pub fn push_borrow(w: &mut World, p: usize, bigger: bool) -> Result<String, String> {
    let k = yours(w, p)?;
    let x = w.capital.drives[k].clone();
    if x.ground.is_some() {
        return Err("Ground has broken already.".into());
    }
    if x.stage < Stage::Quiet {
        return Err("Wait for the study.".into());
    }
    if x.pushed == w.year {
        return Err("You pushed the AD this year already.".into());
    }
    if busy(w, p) {
        return Err("A project is already under way.".into());
    }
    let size = if bigger { x.size.up().ok_or_else(|| "It's the full complex already.".to_string())? } else { x.size };
    let goal = goal_for(w, &x, size);
    let got = raised(&w.capital, &x);
    if got < PUSH_BREAK * goal {
        return Err("Not even the AD will borrow with less than half the goal pledged.".into());
    }
    w.capital.drives[k].pushed = w.year;
    let trust = crate::world::folk::boss::trust(w, Team::College(p)) as f64;
    let reserves = w.finance.college.get(p).map_or(0.0, |c| c.reserves as f64);
    let fever = super::fever::fever(w, p);
    let yes = clamp(0.35 + 0.8 * (trust - 0.5) + clamp(reserves / 40.0e6, -0.2, 0.2) + 0.2 * fever, 0.05, 0.9);
    let ad = w.programs[p].ad.as_ref().map_or("The athletic director".to_string(), |a| a.name.clone());
    if super::draw(w, 40, (x.id as u64) << 16 ^ w.year as u64) >= yes {
        return Ok(format!("{ad}: \"Not on these numbers. Bring me more pledges.\""));
    }
    let rooms = rooms_for(&w.programs[p], &x.kind, size);
    {
        let dr = &mut w.capital.drives[k];
        dr.size = size;
        dr.rooms = rooms;
        dr.goal = goal;
    }
    break_ground(w, k, true);
    Ok(format!(
        "{ad}: \"We'll borrow it.\" Ground breaks on {}. The bonds come out of the staff pool for years.",
        size.words()
    ))
}

/// Your answer to a cost overrun (CP24). Cutting rooms needs a building
/// with more than one.
pub fn fix_overrun(w: &mut World, p: usize, fix: Fix) -> Result<String, String> {
    if !w.seat_program(p) {
        return Err("Not your program.".into());
    }
    let Some(k) = w
        .capital
        .drives
        .iter()
        .rposition(|d| d.program == p as u32 && d.overrun.is_some_and(|o| o.fix.is_none() && o.found <= w.today))
    else {
        return Err("There's no overrun to answer.".into());
    };
    if fix == Fix::CutRooms && w.capital.drives[k].rooms.count_ones() <= 1 {
        return Err("There's only one room to build: wait or borrow.".into());
    }
    Ok(apply_fix(w, k, fix))
}

/// Asking your AD for a facility (people topic 12) once the rooms live: a
/// yes opens a drive's study.
pub(crate) fn ask_drive(w: &mut World, p: usize) -> Result<Option<u32>, String> {
    if open_drive(&w.capital, p).is_some() {
        return Err("A drive is already under way.".into());
    }
    if busy(w, p) {
        return Err("A project is already under way.".into());
    }
    Ok(start(w, p))
}

/// The rooms a project of the books opens: a drive's building with the
/// rooms it kept (CP24), any other by its name (`rooms::project_rooms`).
pub fn rooms_of_project(w: &World, p: usize, x: &Project) -> u16 {
    w.capital
        .drives
        .iter()
        .rev()
        .find(|d| d.program == p as u32 && d.kind == x.name && d.ground == Some(x.started))
        .map_or_else(|| rooms::project_rooms(&w.programs[p], &x.name), |d| d.rooms)
}
