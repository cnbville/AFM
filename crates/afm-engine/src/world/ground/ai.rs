//! Lane G9: the AI works its ground (GR65, with GR64 for the AI).
//!
//! Every program the game plays works the same ground by the same rules as
//! your staff: its men have areas (the way staffs split states), they go on
//! the road in the same windows with the same day limits (`week::road`),
//! the staff makes its five calls a week, each contact builds standing with
//! a high school head coach the way yours does (`standing::contact`'s
//! sizes and gates), standing fades from 60 days without contact, and his
//! kids who did well pay it. Its board reads what its people know
//! (`board`): a coach who trusts it calls, a coach who knows it gives his
//! list, and the services' lists for the rest. That replaces the board
//! from reach, pipelines, footprint and stars in `world/recruiting.rs`.
//!
//! The store is small: one compact tie per coach and holder that someone
//! has worked (the program, and the man who made the contact: his own ties
//! go with him). A tie that was never worked is read from the world's 30
//! years (`standing::derived`), and the work sits on top of that history
//! (the build's pick: the AI's standing never fades below its day one,
//! which the world itself moves as pipelines and men's ties change).
//! Nothing here draws on the world's random stream.

use std::collections::{BTreeMap, HashMap};

use super::people::{self, Holder, Who};
use super::standing::{self, Level};
use super::week::plan::{CALLS, DAYS_PER_MAN};
use super::week::road::{self, Window};
use crate::calendar::{Date, Weekday};
use crate::world::World;
use crate::world::coach::{Employer, Role};
use crate::world::data::Division;

/// A contact this recent gets a Known coach's real list (GR19), the
/// build's pick: about one recruiting cycle.
pub const LIST_DAYS: i32 = 400;
/// A man doesn't go back to a school his staff saw this recently in the
/// same window (proposed, not confirmed).
pub const REVISIT_DAYS: i32 = 28;
/// A call goes to a coach the staff hasn't talked to this long.
pub const CALL_GAP: i32 = 90;
/// The share of a man's days a staff below Division I can pay for on the
/// road (FBS, FCS, Division II, NAIA, Division III, junior colleges;
/// proposed, not confirmed: smaller staffs, smaller budgets, and more of
/// their recruits find them).
pub const ROAD_SHARE: [f64; 6] = [1.0, 1.0, 0.5, 0.35, 0.35, 0.35];
/// An area a staff works is worth at least this share of its best one.
pub const AREA_FLOOR: f64 = 0.1;
/// A staff sends men only where its name draws at least this much (its
/// footprint in the state, 0-1): real staffs work their region and the
/// talent states they can win in, not every state with talent (minor bug
/// 246's lesson). Proposed, not confirmed.
pub const REACH_FLOOR: f64 = 0.15;
/// The tape a kid (and his coach) sends to the schools near him, read by
/// a staff with this chance times his exposure and how near it is (its
/// region in his state), by talent class (FBS, FCS, Division II and the
/// NAIA, Division III and junior colleges). Most small-college rosters are
/// built from kids who reach out; an FBS staff watches little of it.
/// Proposed, not confirmed.
pub const SENT_TAPE: [f64; 4] = [0.3, 0.6, 1.2, 1.6];
/// A second man goes into an area worth this many times the last one
/// picked (the AI's answer to an area that matters: GR65).
pub const SECOND_MAN: f64 = 3.0;
/// A tie with fewer points than this and no contact in two years is
/// dropped (the world's history still reads).
pub const PRUNE: f32 = 1.0;

/// What each star in a school's last four classes is worth to a level's
/// staffs (FBS; FCS; Division II and the NAIA; Division III and junior
/// colleges), no stars to five (proposed, not confirmed). FBS is Ground's
/// talent weighting (`standing::talents`).
const TALENT: [[f32; 6]; 4] = [
    [0.2, 0.3, 1.0, 3.0, 8.0, 20.0],
    [0.4, 0.8, 2.0, 3.0, 1.0, 0.2],
    [0.8, 1.5, 2.0, 0.8, 0.1, 0.0],
    [1.2, 1.5, 1.2, 0.3, 0.0, 0.0],
];

/// A division's talent class (`TALENT`'s row).
pub fn class_of(d: Division) -> usize {
    match d {
        Division::FBS => 0,
        Division::FCS => 1,
        Division::D2 | Division::NAIA => 2,
        Division::D3 | Division::JUCO => 3,
    }
}

/// One worked tie: a high school coach and a holder (a program or a man).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Tie {
    pub points: f32,
    /// When they met, and when it reached Known.
    pub met: Date,
    pub known: Option<Date>,
    pub last: Date,
    /// The last visit (a call isn't one): the sign-in sheet's date.
    pub seen: Date,
    pub contacts: u16,
    /// Contacts this season (the season, the count).
    pub season: i16,
    pub visits: u8,
    /// A kid of his did well with the program (Home's gate, GR28).
    pub did_well: bool,
}

/// A tie as saved: a tuple, so the file doesn't repeat field names.
type Row = (u64, f32, i32, i32, i32, i32, u16, i16, u8, bool);

/// The worked ties, by (coach, holder).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ties(pub BTreeMap<u64, Tie>);

impl serde::Serialize for Ties {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let rows: Vec<Row> = self
            .0
            .iter()
            .map(|(&k, t)| {
                (
                    k,
                    t.points,
                    t.met.0,
                    t.known.map_or(i32::MIN, |d| d.0),
                    t.last.0,
                    t.seen.0,
                    t.contacts,
                    t.season,
                    t.visits,
                    t.did_well,
                )
            })
            .collect();
        rows.serialize(s)
    }
}

impl<'de> serde::Deserialize<'de> for Ties {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let rows: Vec<Row> = Vec::deserialize(d)?;
        Ok(Ties(
            rows.into_iter()
                .map(|r| {
                    let t = Tie {
                        points: r.1,
                        met: Date(r.2),
                        known: (r.3 != i32::MIN).then_some(Date(r.3)),
                        last: Date(r.4),
                        seen: Date(r.5),
                        contacts: r.6,
                        season: r.7,
                        visits: r.8,
                        did_well: r.9,
                    };
                    (r.0, t)
                })
                .collect(),
        ))
    }
}

/// A man's areas this season.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Beat {
    pub prog: u32,
    pub man: u32,
    pub areas: Vec<u16>,
}

/// The AI's ground on `Ground.ai`.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Ai {
    pub ties: Ties,
    /// Each program's men and their areas, sorted by program.
    pub beats: Vec<Beat>,
    /// The season the areas were planned for.
    pub planned: i32,
    /// Road days each program used this season: (fall, spring), by program.
    pub used: Vec<(u16, u16)>,
    pub used_season: i32,
    /// Each talent class's schools in each area, most talent first:
    /// `order[class][area]`. Set when the areas are planned, so a loaded
    /// file goes on the same way.
    pub order: Vec<Vec<Vec<u32>>>,
}

/// A holder as a key's low half.
fn holder_code(h: Holder) -> u64 {
    match h {
        Holder::Program(p) => p as u64,
        Holder::Coach(c) => 0x8000_0000 | c as u64,
    }
}

fn holder_of(code: u64) -> Holder {
    let c = (code & 0xFFFF_FFFF) as u32;
    if c & 0x8000_0000 != 0 { Holder::Coach(c & 0x7FFF_FFFF) } else { Holder::Program(c) }
}

/// The key for a coach and a holder.
pub fn tie_key(coach: u32, holder: Holder) -> u64 {
    ((coach as u64) << 32) | holder_code(holder)
}

/// The coach and holder of a key.
pub fn key_parts(k: u64) -> (u32, Holder) {
    ((k >> 32) as u32, holder_of(k))
}

impl Ai {
    /// A worked tie with a person (only high school and junior college
    /// coaches are worked by the AI).
    pub fn get(&self, who: Who, holder: Holder) -> Option<&Tie> {
        match who {
            Who::Coach(c) => self.ties.0.get(&tie_key(c, holder)),
            _ => None,
        }
    }

    /// Every worked tie with one coach.
    pub fn with(&self, coach: u32) -> impl Iterator<Item = (Holder, &Tie)> {
        let a = (coach as u64) << 32;
        self.ties.0.range(a..a + (1u64 << 32)).map(|(&k, t)| (holder_of(k), t))
    }

    pub fn len(&self) -> usize {
        self.ties.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ties.0.is_empty()
    }
}

/// The highest points the gates allow (GR28), as `standing::cap`.
pub fn cap(t: &Tie, d: Date) -> f32 {
    if t.contacts < 2 {
        return 24.9;
    }
    if !t.known.is_some_and(|k| k.days_to(d) >= 365) {
        return 49.9;
    }
    let years = t.met.days_to(d) as f32 / 365.25;
    if (years >= 3.0 && t.did_well) || years >= 5.0 { 100.0 } else { 74.9 }
}

fn gain(t: &mut Tie, by: f32, d: Date) {
    if by >= 0.0 {
        let top = cap(t, d).max(t.points);
        t.points = (t.points + by).min(top);
    } else {
        t.points += by;
    }
    t.points = t.points.clamp(0.0, 100.0);
    if t.points >= 25.0 && t.known.is_none() {
        t.known = Some(d);
    }
}

/// A tie made from day one: they have known each other for years
/// (`standing::day_one`).
fn new_tie(start: f32, d: Date) -> Tie {
    let lv = Level::of(start);
    let mut t = Tie { points: start, met: d, last: d, ..Default::default() };
    if start > 0.0 {
        t.met = d.add(-5 * 365);
        t.contacts = match lv {
            Level::Stranger => 1,
            Level::Known => 3,
            _ => 6,
        };
        if lv >= Level::Known {
            t.known = Some(if lv >= Level::Trusted { d.add(-2 * 365) } else { d.add(-180) });
        }
    }
    t
}

/// Whether the program is the AI's to play: it plays this year and isn't
/// your seat's.
pub fn ai_program(w: &World, prog: usize) -> bool {
    w.programs.get(prog).is_some_and(|p| p.plays(w.year)) && seat_college(w) != Some(prog)
}

/// Your college seat's program, whether or not the seat's ground is on
/// (your staff works by your plan either way).
pub fn seat_college(w: &World) -> Option<usize> {
    w.career.as_ref()?;
    match crate::world::scouting::readers::seat_staff(w) {
        Some(crate::world::scouting::readers::Staff::Program(p)) => Some(p),
        _ => None,
    }
}

/// One contact by an AI staff with a high school coach: the man who made
/// it builds his own tie in full and the program a third, with the seat's
/// sizes (`standing::contact`): 6 points a visit × his people skill × the
/// coach's warmth × the program's pull, the head coach's name on his own
/// visits and calls, a call a third of a visit, a second contact the same
/// week half, one a day.
pub fn contact(w: &mut World, prog: usize, man: usize, coach: u32, call: bool, d: Date) {
    let who = Who::Coach(coach);
    let mk = people::makeup(w, who);
    let prestige = w.programs.get(prog).map_or(0.5, |p| p.prestige) as f32;
    let pull = 0.9 + 0.2 * prestige;
    let hc = w.programs.get(prog).and_then(|p| p.head_coach) == Some(man);
    let name = 0.8 + 1.3 * prestige * prestige;
    let skill = people::skill(w, Holder::Coach(man as u32));
    let base = standing::VISIT * skill * mk.warmth * pull;
    let size = match (call, hc) {
        (_, true) => base * name,
        (false, false) => base,
        (true, false) => base * standing::CALL_SHARE,
    };
    let season = w.dates.season as i16;
    let week_start = d.add(-(d.weekday().index()));
    for (holder, share) in [(Holder::Coach(man as u32), 1.0), (Holder::Program(prog as u32), standing::PROGRAM_SHARE)] {
        let k = tie_key(coach, holder);
        let start = match w.ground.ai.ties.0.get(&k) {
            Some(_) => 0.0,
            None => standing::derived(w, who, holder),
        };
        let t = w.ground.ai.ties.0.entry(k).or_insert_with(|| new_tie(start, d));
        if t.last == d && t.contacts > 0 && t.season == season {
            continue;
        }
        // A second contact the same week buys half (GR54).
        let mut by = size * share;
        if t.last >= week_start && t.last < d {
            by *= 0.5;
        }
        if t.season != season {
            t.season = season;
            t.visits = 0;
        }
        if !call {
            t.visits = t.visits.saturating_add(1);
            t.seen = d;
        }
        t.contacts = t.contacts.saturating_add(1);
        t.last = d;
        gain(t, by, d);
    }
}

// --- The areas (GR68's rules for the AI) ----------------------------------------------------

/// Each school's recruits by stars over the last four classes.
fn star_counts(w: &World) -> Vec<[u16; 6]> {
    let mut t = vec![[0u16; 6]; w.high_schools.len()];
    let first = w.year - 3;
    for q in &w.people {
        if q.class_of >= first && q.class_of <= w.year && q.hs < t.len() {
            let s = (q.stars as usize).min(5);
            t[q.hs][s] = t[q.hs][s].saturating_add(1);
        }
    }
    for a in w.alumni.iter().rev().take_while(|a| a.class_of >= first - 4) {
        if a.class_of >= first && a.class_of <= w.year && a.high_school < t.len() {
            let s = (a.stars as usize).min(5);
            t[a.high_school][s] = t[a.high_school][s].saturating_add(1);
        }
    }
    t
}

fn talent(counts: &[u16; 6], class: usize) -> f32 {
    1.0 + counts.iter().zip(TALENT[class].iter()).map(|(&n, &v)| n as f32 * v).sum::<f32>()
}

/// The men who work areas: the on-field assistants, then the head coach
/// (`week::road::men`).
fn men(w: &World, prog: usize) -> Vec<usize> {
    road::men(w, prog)
}

/// Plans every AI staff's areas for the season: each area is worth the
/// talent its schools control at the staff's level times where the
/// program's name draws (its footprint in the state: where it can win
/// kids, from its region, its national reach and its pipelines). Each man
/// takes up to three areas (`staff::beats::PER_MAN`), the one he has the
/// most ties in first; an area worth `SECOND_MAN` times the last one picked
/// gets a second man.
pub(crate) fn plan(w: &mut World) {
    let counts = star_counts(w);
    let by_area = super::areas::schools_by_area(w);
    let n_areas = by_area.len();
    let mut area_talent: Vec<[f32; 4]> = vec![[0.0; 4]; n_areas];
    let mut order: Vec<Vec<Vec<u32>>> = vec![vec![Vec::new(); n_areas]; 4];
    for (a, schools) in by_area.iter().enumerate() {
        for class in 0..4 {
            let mut v: Vec<(f32, u32)> = schools.iter().map(|&h| (talent(&counts[h], class), h as u32)).collect();
            area_talent[a][class] = v.iter().map(|x| x.0).sum();
            v.sort_by(|x, y| y.0.total_cmp(&x.0).then(x.1.cmp(&y.1)));
            order[class][a] = v.into_iter().map(|x| x.1).collect();
        }
    }
    let table = super::areas::table();
    let mut beats: Vec<Beat> = Vec::new();
    for prog in 0..w.programs.len() {
        if !ai_program(w, prog) {
            continue;
        }
        let p = &w.programs[prog];
        let class = class_of(p.division);
        let mut val: Vec<(f64, u16)> = (0..n_areas)
            .map(|a| {
                let st = table[a].state.min(50);
                let f = p.footprint[st];
                let v = if f >= REACH_FLOOR { area_talent[a][class] as f64 * f * f } else { 0.0 };
                (v, a as u16)
            })
            .filter(|x| x.0 > 0.0)
            .collect();
        val.sort_by(|x, y| y.0.total_cmp(&x.0).then(x.1.cmp(&y.1)));
        let top = val.first().map_or(0.0, |x| x.0);
        val.retain(|x| x.0 >= AREA_FLOOR * top);
        let ms = men(w, prog);
        if ms.is_empty() {
            continue;
        }
        let slots = ms.len() * super::staff::beats::PER_MAN;
        let mut picks: Vec<u16> = val.iter().take(slots).map(|x| x.1).collect();
        // A second man where an area matters most.
        let last = val.get(picks.len().saturating_sub(1)).map_or(0.0, |x| x.0);
        let extra: Vec<u16> = val.iter().take_while(|x| x.0 >= SECOND_MAN * last && last > 0.0).map(|x| x.1).collect();
        for a in extra {
            if picks.len() < slots + ms.len() {
                picks.push(a);
            }
        }
        // Each area to the man with the most ties in it and room left.
        let mut rows: Vec<Beat> =
            ms.iter().map(|&m| Beat { prog: prog as u32, man: m as u32, areas: Vec::new() }).collect();
        for a in picks {
            let schools = &by_area[a as usize];
            let mut best: Option<(f64, usize)> = None;
            for (k, &m) in ms.iter().enumerate() {
                if rows[k].areas.len() > super::staff::beats::PER_MAN || rows[k].areas.contains(&a) {
                    continue;
                }
                let ties: f64 =
                    w.coaches[m].ties.iter().filter(|(h, _)| schools.binary_search(h).is_ok()).map(|(_, t)| *t).sum();
                let s = ties - 0.4 * rows[k].areas.len() as f64 - if Some(m) == p.head_coach { 1.0 } else { 0.0 };
                if best.is_none_or(|b| s > b.0) {
                    best = Some((s, k));
                }
            }
            if let Some((_, k)) = best {
                rows[k].areas.push(a);
            }
        }
        beats.extend(rows.into_iter().filter(|r| !r.areas.is_empty()));
    }
    w.ground.ai.beats = beats;
    w.ground.ai.order = order;
    w.ground.ai.planned = w.dates.season;
}

/// A program's beats.
pub fn beats_of(w: &World, prog: usize) -> &[Beat] {
    let b = &w.ground.ai.beats;
    let a = b.partition_point(|x| (x.prog as usize) < prog);
    let z = b.partition_point(|x| (x.prog as usize) <= prog);
    &b[a..z]
}

/// The states a staff works (its men's areas), for the services' lists.
/// Your seat's are your men's areas (lane G10), or before you set any the
/// states your department covers by name (footprint 0.3 or more).
pub fn work_states(w: &World, prog: usize) -> [bool; 51] {
    let mut v = [false; 51];
    let table = super::areas::table();
    for b in beats_of(w, prog) {
        for &a in &b.areas {
            v[table[a as usize].state.min(50)] = true;
        }
    }
    if seat_college(w) == Some(prog) {
        for b in &w.ground.staff.beats {
            if let Some(a) = table.get(b.area as usize) {
                v[a.state.min(50)] = true;
            }
        }
        if w.ground.staff.beats.is_empty()
            && let Some(p) = w.programs.get(prog)
        {
            for (st, &f) in p.footprint.iter().enumerate().take(51) {
                v[st] |= f >= 0.3;
            }
        }
    }
    if let Some(p) = w.programs.get(prog) {
        v[p.state.min(50)] = true;
    }
    v
}

// --- The day ---------------------------------------------------------------------------------

/// The AI's ground each evening: the areas at the February meeting, the
/// road on road days, the calls on Mondays, the fade on the first, and his
/// kids who did well on February 1 and May 20.
pub(crate) fn day(w: &mut World, d: Date) {
    if w.programs.is_empty() || w.high_schools.is_empty() {
        return;
    }
    if w.ground.ai.planned != w.dates.season
        && (d >= super::staff::february_meeting(w, d) || w.ground.ai.beats.is_empty())
    {
        plan(w);
    }
    if w.ground.ai.used_season != w.dates.season {
        w.ground.ai.used = vec![(0, 0); w.programs.len()];
        w.ground.ai.used_season = w.dates.season;
    }
    if w.ground.ai.used.len() < w.programs.len() {
        w.ground.ai.used.resize(w.programs.len(), (0, 0));
    }
    if let Some(win) = road::window(w, d)
        && road::day_kind(win, d)
    {
        road_day(w, win, d);
    }
    if d.weekday() == Weekday::Mon {
        calls(w, d);
    }
    let (_, m, day) = d.parts();
    if day == 1 {
        month(w, d);
    }
    if (m, day) == (2, 1) || (m, day) == (5, 20) {
        kids(w, d);
    }
}

/// Whether a man works this day: three of five school days in the spring
/// and the winter, each by his place on the staff; the fall's Thursday and
/// Friday; a staff below Division I only as its budget allows.
fn works_today(w: &World, prog: usize, k: usize, man: usize, win: Window, d: Date) -> bool {
    let wd = d.weekday().index() as usize;
    let on = match win {
        Window::Fall => true,
        _ => (wd + k) % 5 < DAYS_PER_MAN[0] as usize,
    };
    if !on {
        return false;
    }
    let div = w.programs[prog].division as usize;
    let share = ROAD_SHARE[div.min(5)];
    share >= 1.0 || super::unit(w.options.seed, &[0xA1_0AD, prog as u64, man as u64, d.0 as u64]) < share
}

/// A road day for every AI staff: each man out goes to his areas' schools
/// with the most talent at his level that his staff hasn't seen lately,
/// as many as the area allows in a day (GR4), inside the window's days.
fn road_day(w: &mut World, win: Window, d: Date) {
    if w.ground.ai.order.is_empty() {
        return;
    }
    let season = w.dates.season;
    let beats = w.ground.ai.beats.clone();
    let mut k_of_prog: HashMap<u32, usize> = HashMap::new();
    for b in beats {
        let prog = b.prog as usize;
        if !ai_program(w, prog) {
            continue;
        }
        let man = b.man as usize;
        // He is still on the staff and the rules let him out.
        if !road::men(w, prog).contains(&man) || road::may_go(w, prog, man, d).is_err() {
            continue;
        }
        let k = {
            let e = k_of_prog.entry(b.prog).or_insert(0);
            *e += 1;
            *e - 1
        };
        if !works_today(w, prog, k, man, win, d) {
            continue;
        }
        // The window's day limit (FBS and FCS; none in the winter).
        let limited = matches!(w.programs[prog].division, Division::FBS | Division::FCS);
        if limited && win != Window::Winter {
            let academy = crate::world::scouting::dept::service_academy(&w.programs[prog]);
            let (fall, spring) = crate::world::scouting::dept::eval_limits(season, academy);
            let used = w.ground.ai.used[prog];
            let (have, cap) = if win == Window::Fall { (used.0, fall) } else { (used.1, spring) };
            if have >= cap {
                continue;
            }
            let u = &mut w.ground.ai.used[prog];
            if win == Window::Fall {
                u.0 += 1;
            } else {
                u.1 += 1;
            }
        }
        if b.areas.is_empty() {
            continue;
        }
        let week = d.0.div_euclid(7) as usize;
        let area = b.areas[(week + k) % b.areas.len()];
        let n = super::areas::schools_a_day(area) as usize;
        let class = class_of(w.programs[prog].division);
        let list = w.ground.ai.order.get(class).and_then(|o| o.get(area as usize)).cloned().unwrap_or_default();
        let mut went = 0;
        for h in list {
            if went >= n {
                break;
            }
            let Some(Who::Coach(c)) = people::head_coach_of(w, h as usize) else { continue };
            let seen = w.ground.ai.ties.0.get(&tie_key(c, Holder::Program(prog as u32))).map(|t| t.last);
            if seen.is_some_and(|l| l.days_to(d) < REVISIT_DAYS) {
                continue;
            }
            contact(w, prog, man, c, false, d);
            went += 1;
        }
    }
}

/// Mondays: each AI staff's five calls (the seat's `week::plan::CALLS`):
/// a coach one contact from Known first, then the Known coaches in its
/// areas it hasn't talked to in `CALL_GAP` days, most talent first. The
/// area's man makes the call.
fn calls(w: &mut World, d: Date) {
    if w.ground.ai.order.is_empty() {
        return;
    }
    let beats = w.ground.ai.beats.clone();
    let mut i = 0;
    while i < beats.len() {
        let prog = beats[i].prog as usize;
        let mut z = i;
        while z < beats.len() && beats[z].prog == beats[i].prog {
            z += 1;
        }
        if ai_program(w, prog) {
            let class = class_of(w.programs[prog].division);
            let mut want: Vec<(u8, u32, usize)> = Vec::new();
            for b in &beats[i..z] {
                for &a in &b.areas {
                    let Some(list) = w.ground.ai.order.get(class).and_then(|o| o.get(a as usize)) else { continue };
                    for &h in list.iter().take(40) {
                        let Some(Who::Coach(c)) = people::head_coach_of(w, h as usize) else { continue };
                        let t = w.ground.ai.ties.0.get(&tie_key(c, Holder::Program(prog as u32)));
                        let rank = match t {
                            Some(t) if t.contacts == 1 => 0,
                            Some(t) if t.points >= 25.0 && t.last.days_to(d) >= CALL_GAP => 1,
                            None if standing::derived(w, Who::Coach(c), Holder::Program(prog as u32)) >= 25.0 => 2,
                            _ => continue,
                        };
                        want.push((rank, c, b.man as usize));
                    }
                }
            }
            want.sort_by_key(|x| x.0);
            want.dedup_by_key(|x| x.1);
            for (_, c, man) in want.into_iter().take(CALLS) {
                if road::men(w, prog).contains(&man) {
                    contact(w, prog, man, c, true, d);
                }
            }
        }
        i = z;
    }
}

/// The first of each month: a tie fades from 60 days without contact, a
/// point a month and half a point more for each rival staff that has been
/// with him in the last 30 days (GR24). A cold tie that holds nothing goes.
fn month(w: &mut World, d: Date) {
    let recent = d.add(-30);
    let old = d.add(-standing::FADE_AFTER);
    let gone = d.add(-730);
    let ties = &mut w.ground.ai.ties.0;
    let keys: Vec<u64> = ties.keys().copied().collect();
    let mut i = 0;
    while i < keys.len() {
        let coach = keys[i] >> 32;
        let mut z = i;
        while z < keys.len() && keys[z] >> 32 == coach {
            z += 1;
        }
        let busy = keys[i..z]
            .iter()
            .filter(|&&k| matches!(holder_of(k), Holder::Program(_)) && ties[&k].last >= recent)
            .count() as f32;
        for &k in &keys[i..z] {
            let t = ties.get_mut(&k).expect("key");
            if t.last < old {
                let rivals = if t.last >= recent { busy - 1.0 } else { busy };
                t.points = (t.points - standing::FADE - standing::FADE_RIVAL * rivals.max(0.0)).max(0.0);
            }
            if t.points < PRUNE && t.last < gone {
                ties.remove(&k);
            }
        }
        i = z;
    }
}

/// His kids who did well pay the staff that had them (GR23, the seat's
/// sizes): a starter last season on February 1, a graduate and a draft
/// pick on May 20, to the program and its head coach, once each.
fn kids(w: &mut World, d: Date) {
    let (y, m, _) = d.parts();
    let mut pay: Vec<(usize, usize, f32)> = Vec::new();
    for q in w.people.iter().filter(|q| q.stage == crate::world::Stage::College) {
        let Some(prog) = q.college else { continue };
        if m == 2 && q.last_role == Some(crate::player::PlayingTime::Starter) {
            pay.push((prog, q.hs, standing::STARTED));
        }
        if m == 5 && (q.cal.graduated || q.cal.may_graduate) {
            pay.push((prog, q.hs, standing::GRADUATED));
        }
    }
    if m == 5 {
        for a in w.alumni.iter().rev().take_while(|a| a.year >= y - 1) {
            if a.year == y
                && a.pick.is_some()
                && let Some(&prog) = a.colleges.last()
            {
                pay.push((prog, a.high_school, standing::DRAFTED));
            }
        }
    }
    for (prog, hs, by) in pay {
        if !ai_program(w, prog) {
            continue;
        }
        let Some(Who::Coach(c)) = people::head_coach_of(w, hs) else { continue };
        let mut holders = vec![Holder::Program(prog as u32)];
        if let Some(hc) = w.programs[prog].head_coach {
            holders.push(Holder::Coach(hc as u32));
        }
        for holder in holders {
            let k = tie_key(c, holder);
            let start = match w.ground.ai.ties.0.get(&k) {
                Some(_) => 0.0,
                None => standing::derived(w, Who::Coach(c), holder),
            };
            let t = w.ground.ai.ties.0.entry(k).or_insert_with(|| new_tie(start, d));
            t.did_well = true;
            gain(t, by, d);
        }
    }
}

// --- What the board reads ------------------------------------------------------------------

/// Each program's worked standing with each high school's head coach, the
/// best of the program's and its current men's ties, and the last contact:
/// built once for a signing stage.
#[derive(Default)]
pub struct Index {
    pub by_prog: Vec<HashMap<u32, (f32, Date)>>,
}

impl Index {
    pub fn get(&self, prog: usize, school: usize) -> Option<(f32, Date)> {
        self.by_prog.get(prog).and_then(|m| m.get(&(school as u32)).copied())
    }
}

/// The index over the worked ties (the world's history is read beside it,
/// `standing::derived`). Your seat's program reads your own memories.
pub fn index(w: &World) -> Index {
    let mut by_prog: Vec<HashMap<u32, (f32, Date)>> = vec![HashMap::new(); w.programs.len()];
    let mut put = |p: usize, h: usize, pts: f32, last: Date| {
        if let Some(m) = by_prog.get_mut(p) {
            let e = m.entry(h as u32).or_insert((0.0, Date(i32::MIN)));
            e.0 = e.0.max(pts);
            e.1 = e.1.max(last);
        }
    };
    for (&k, t) in &w.ground.ai.ties.0 {
        let (coach, holder) = key_parts(k);
        let Some((h, Role::HeadCoach)) = people::coach_school(w, coach as usize) else { continue };
        let prog = match holder {
            Holder::Program(p) => Some(p as usize),
            Holder::Coach(m) => match w.coaches.get(m as usize).and_then(|c| c.job) {
                Some(j) => match j.employer {
                    Employer::College(p) => Some(p),
                    _ => None,
                },
                None => None,
            },
        };
        if let Some(p) = prog {
            put(p, h, t.points, t.last);
        }
    }
    // Your seat: your staff's own memories with head coaches.
    if let Some(seat) = standing::seat_program(w) {
        let holders = standing::holders(w, seat);
        for m in &w.ground.memories {
            if !holders.contains(&m.holder) {
                continue;
            }
            if let Who::Coach(c) = m.who
                && let Some((h, Role::HeadCoach)) = people::coach_school(w, c as usize)
            {
                put(seat, h, m.points, m.last);
            }
        }
    }
    Index { by_prog }
}

/// A staff's standing with a school's head coach, day one included, the
/// index's worked ties on top.
pub fn school_points(w: &World, ix: &Index, prog: usize, school: usize) -> f32 {
    let tie = w.programs.get(prog).map_or(0.0, |p| {
        [p.head_coach, p.oc, p.dc]
            .into_iter()
            .chain(p.position_coaches.iter().copied())
            .flatten()
            .filter_map(|c| w.coaches.get(c).and_then(|c| c.ties.get(&school)).copied())
            .fold(0.0f64, f64::max)
    });
    let day_one = standing::program_part(w, prog, school).max(standing::TIE_POINTS * tie.clamp(0.0, 1.0) as f32);
    ix.get(prog, school).map_or(day_one, |x| x.0.max(day_one))
}

/// How well a staff knows a kid's coach, 0-1: Known is half, Trusted all
/// (what an early offer and a booster's pick lean on).
pub fn knows(w: &World, ix: &Index, prog: usize, school: usize) -> f64 {
    (school_points(w, ix, prog, school) as f64 / 50.0).clamp(0.0, 1.0)
}

/// How a staff hears of a recruit through his high school coach (GR19):
/// a coach who trusts it calls (Trusted and Home), a Known coach gives his
/// list when it has been in touch this cycle and calls with his public
/// news otherwise, and a coach it has only met gives the names everyone
/// has. `pts` is the staff's standing with him, `last` its last contact.
pub fn told(pts: f32, last: Option<Date>, public: bool, today: Date) -> bool {
    let contacted = last.is_some_and(|l| l.days_to(today) <= LIST_DAYS);
    pts >= 50.0 || (pts >= 25.0 && (contacted || public)) || (contacted && public)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_round_trip() {
        for h in [Holder::Program(0), Holder::Program(812), Holder::Coach(0), Holder::Coach(123_456)] {
            let k = tie_key(77, h);
            assert_eq!(key_parts(k), (77, h));
        }
        assert!(tie_key(3, Holder::Coach(1)) < tie_key(4, Holder::Program(0)));
    }

    #[test]
    fn gates_hold_like_the_seat() {
        let d = Date::ymd(2026, 4, 20);
        let mut t = new_tie(0.0, d);
        t.contacts = 1;
        gain(&mut t, 40.0, d);
        assert!(t.points < 25.0, "a Stranger after one contact");
        t.contacts = 2;
        gain(&mut t, 40.0, d);
        assert!(t.points < 50.0 && t.points >= 25.0, "Known, never Trusted in a day");
        assert_eq!(t.known, Some(d));
        let later = d.add(400);
        gain(&mut t, 40.0, later);
        assert!(t.points >= 50.0 && t.points < 75.0, "Trusted a year on, not Home");
    }

    #[test]
    fn told_reads_the_ladder() {
        let d = Date::ymd(2026, 12, 1);
        assert!(told(80.0, None, false, d), "Home calls first");
        assert!(told(55.0, None, false, d), "Trusted sends tape");
        assert!(told(30.0, Some(d.add(-100)), false, d), "Known gives his list");
        assert!(!told(30.0, None, false, d), "Known without contact: public news only");
        assert!(told(30.0, None, true, d));
        assert!(told(10.0, Some(d.add(-10)), true, d), "a Stranger met gives the public names");
        assert!(!told(10.0, Some(d.add(-10)), false, d));
        assert!(!told(10.0, Some(d.add(-500)), true, d), "a visit two years ago is cold");
    }

    #[test]
    fn ties_save_compactly() {
        let mut t = Ties::default();
        t.0.insert(tie_key(5, Holder::Program(9)), new_tie(30.0, Date::ymd(2026, 5, 1)));
        let b = rmp_serde::to_vec_named(&t).expect("save");
        let back: Ties = rmp_serde::from_slice(&b).expect("load");
        assert_eq!(back, t);
    }
}
