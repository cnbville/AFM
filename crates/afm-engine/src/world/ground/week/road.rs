//! Road days (GR48) and the money for trips (GR49), and a man's day on the
//! road: his schools, his looks, the coach he sees, his night call.
//!
//! The rule book is the built one: the spring window's recruiting-person
//! days and the fall's evaluation days (`scouting::dept::eval_limits`: 140
//! and 33 from 2023, 168 and 42 before), up to the off-campus recruiters at
//! once (`off_campus_recruiters`), the FBS head coach off the spring road
//! from 2008. January's and December's contact periods have no day limit.
//! If the 2027 model passes (one pool of 300 from January 1 to May 31), it
//! comes in here; the game's rule book doesn't have it yet.

use super::{Done, DoneKind, Road, seat_program};
use crate::calendar::{Date, Period, Weekday, recruiting_period};
use crate::world::ground::areas::{self, Trip};
use crate::world::ground::people::{self, Holder, Who};
use crate::world::ground::standing::{self, Contact, Level};
use crate::world::scouting::readers::COACH;
use crate::world::scouting::{Kind, dept};
use crate::world::{Stage, World};

/// Which road window a day is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum Window {
    /// The spring contact (or, before 2024, evaluation) period.
    Spring,
    /// The fall evaluation period, September to November.
    Fall,
    /// January's and December's contact periods (no day limit).
    Winter,
}

impl Window {
    pub fn key(self) -> &'static str {
        match self {
            Window::Spring => "spring",
            Window::Fall => "fall",
            Window::Winter => "winter",
        }
    }
}

/// The road window on a day, if the rules let your staff out at all.
pub fn window(w: &World, d: Date) -> Option<Window> {
    let y = super::beat::dates_for(w, d);
    let m = d.month();
    match recruiting_period(&y, d) {
        Period::Contact | Period::Evaluation if matches!(m, 4..=6) => Some(Window::Spring),
        Period::Evaluation if matches!(m, 9..=11) => Some(Window::Fall),
        Period::Contact => Some(Window::Winter),
        _ => None,
    }
}

/// The days a window holds this season (None: no limit) and the days used.
pub fn pool(w: &World, prog: usize, win: Window) -> Option<(u16, u16)> {
    if w.sandbox(crate::world::sandbox::Switch::StaffTime) {
        return None;
    }
    let season = w.dates.season;
    let (fall, spring) = dept::eval_limits(season, dept::service_academy(&w.programs[prog]));
    let (s, used_f, used_s) = w.scouting.dept.eval_days;
    let (used_f, used_s) = if s == season { (used_f, used_s) } else { (0, 0) };
    match win {
        Window::Spring => Some((spring, used_s)),
        Window::Fall => Some((fall, used_f)),
        Window::Winter => None,
    }
}

/// The road days left in a window (None: no limit).
pub fn left(w: &World, prog: usize, win: Window) -> Option<u16> {
    pool(w, prog, win).map(|(a, u)| a.saturating_sub(u))
}

/// The open weekdays left in a window from `from` on (school days: Monday
/// to Friday; the fall's Thursdays and Fridays).
pub fn open_days_from(w: &World, win: Window, from: Date) -> u32 {
    let mut n = 0;
    let mut d = from;
    for _ in 0..120 {
        if window(w, d) == Some(win) {
            if day_kind(win, d) {
                n += 1;
            }
        } else if d > from.add(3)
            && window(w, d).is_none()
            && window(w, d.add(1)).is_none()
            && window(w, d.add(2)).is_none()
        {
            break;
        }
        d = d.add(1);
    }
    n
}

/// The days of the week a window's road runs on: school days in the spring
/// and winter, the fall's Thursday practices and Friday games.
pub fn day_kind(win: Window, d: Date) -> bool {
    let wd = d.weekday();
    match win {
        Window::Fall => matches!(wd, Weekday::Thu | Weekday::Fri),
        _ => !matches!(wd, Weekday::Sat | Weekday::Sun),
    }
}

/// Whether a man may go on the road on a day: on your staff, an on-field
/// coach, and not the FBS head coach in the spring window (from 2008).
pub fn may_go(w: &World, prog: usize, man: usize, d: Date) -> Result<(), String> {
    let p = &w.programs[prog];
    let on =
        p.head_coach == Some(man) || p.oc == Some(man) || p.dc == Some(man) || p.position_coaches.contains(&Some(man));
    if !on {
        return Err("He isn't on your staff.".into());
    }
    if p.head_coach == Some(man)
        && p.division == crate::world::data::Division::FBS
        && w.year >= 2008
        && window(w, d) == Some(Window::Spring)
    {
        return Err("The rules keep an FBS head coach off the road in the spring (since 2008).".into());
    }
    Ok(())
}

/// The men who can go out: your on-field staff, the head coach last.
pub fn men(w: &World, prog: usize) -> Vec<usize> {
    let p = &w.programs[prog];
    let mut v: Vec<usize> = Vec::new();
    for c in [p.oc, p.dc].into_iter().chain(p.position_coaches.iter().copied()).flatten() {
        if !v.contains(&c) && Some(c) != p.head_coach {
            v.push(c);
        }
    }
    if let Some(h) = p.head_coach {
        v.push(h);
    }
    v
}

// --- The money (GR49) ---------------------------------------------------------------------

/// The share of the recruiting line that pays for the staff's road (the
/// rest is official visits, camps, mail and the rest), and the weeks it is
/// spread over: about 6 spring weeks, 12 in the fall, 4 in January and 2 in
/// December (proposed, not confirmed).
pub const ROAD_SHARE: f64 = 0.12;
pub const ROAD_WEEKS: f64 = 24.0;
/// A day on the road for one man: hotel and meals (dollars, 2026; proposed,
/// not confirmed).
pub const DAY_COST: f64 = 180.0;
/// A mile driven, the IRS rate's neighborhood (2026).
pub const MILE_COST: f64 = 0.70;
/// A flight: the fare and the rental car (2026; proposed, not confirmed).
pub const FLIGHT_COST: f64 = 700.0;

/// This week's money for the road, shown as trips.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub struct Purse {
    /// Flights the week's money covers after the drives planned.
    pub flights: u32,
    /// Flights planned.
    pub flown: u32,
    /// The AD's raise is in it.
    pub raised: bool,
}

/// The week's road money (dollars, kept inside: the screens show trips).
fn week_money(w: &World, prog: usize) -> f64 {
    let line =
        w.finance.college.get(prog).map_or(0.0, |d| d.budget.l[crate::world::finance::college::RECRUITING] as f64);
    let line = if line > 0.0 {
        line
    } else {
        // No books yet: the tier's usual line.
        match crate::world::finance::college::tier(w, prog, w.year) {
            0 => 5.0e6,
            1 => 0.8e6,
            2 => 0.3e6,
            _ => 0.05e6,
        }
    };
    let idx = crate::world::finance::data::hs_index(w.year) / crate::world::finance::data::hs_index(2026);
    line * ROAD_SHARE / ROAD_WEEKS / idx.max(0.05)
}

/// What a trip costs (dollars at 2026 prices).
pub fn trip_cost(t: Trip, days: u32) -> f64 {
    match t {
        Trip::Drive { miles, .. } => 2.0 * miles * MILE_COST + DAY_COST * days as f64,
        Trip::Fly { .. } => FLIGHT_COST + DAY_COST * days as f64,
    }
}

/// The week's purse for a set of rows.
pub fn purse(w: &World, prog: usize, rows: &[Road]) -> Purse {
    let money = week_money(w, prog);
    let mut spent = 0.0;
    let mut flown = 0;
    for r in rows {
        let t = areas::trip_to_area(w, prog, r.area);
        let days = r.days.count_ones();
        if t.flight() {
            flown += 1;
        }
        if !t.flight() {
            spent += trip_cost(t, days);
        }
    }
    let typical = trip_cost(Trip::Fly { miles: 0.0, hours: 0.0 }, 2);
    let left = (money - spent).max(0.0);
    let flights = (left / typical).floor() as u32;
    Purse { flights, flown, raised: crate::world::folk::staffing::recruit_bonus(w, prog) > 0.0 }
}

/// "1 flight", "3 flights", "no flights".
pub fn flights_words(n: u32) -> String {
    match n {
        0 => "no flights".into(),
        1 => "1 flight".into(),
        n => format!("{n} flights"),
    }
}

// --- A day on the road --------------------------------------------------------------------

/// A recruit's weight on the road: by stars, for picking schools.
const WEIGHT: [f64; 6] = [0.5, 0.5, 1.0, 2.0, 4.0, 8.0];

/// Seen in person by your staff this recruiting year.
pub fn seen(w: &World, i: usize, since: Date) -> bool {
    w.scouting.dept.looks.get(&w.people[i].player.id).is_some_and(|v| {
        v.iter().any(|l| matches!(l.kind, Kind::Practice | Kind::Game | Kind::Camp) && l.date >= since)
    })
}

/// The recruiting year a day belongs to starts on September 1.
pub fn recruiting_year(d: Date) -> Date {
    let (y, m, _) = d.parts();
    Date::ymd(if m >= 9 { y } else { y - 1 }, 9, 1)
}

/// The board's high school recruits by school, for the day.
pub(crate) fn board_by_school(w: &World, board: &[usize]) -> std::collections::HashMap<usize, Vec<usize>> {
    let mut m: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
    for &i in board {
        let q = &w.people[i];
        if matches!(q.stage, Stage::HighSchool | Stage::PrepSchool) && q.hs < w.high_schools.len() {
            m.entry(q.hs).or_default().push(i);
        }
    }
    m
}

/// A school's worth to a man's day: its board recruits (more for the ones
/// nobody of yours has seen in person this year).
pub(crate) fn school_need(w: &World, kids: &[usize], since: Date) -> f64 {
    kids.iter()
        .map(|&i| {
            let s = WEIGHT[(w.people[i].stars as usize).min(5)];
            if seen(w, i, since) { 0.25 * s } else { s }
        })
        .sum()
}

/// The day's road for your seat (from `scouting::dept`, in place of the
/// built Thursday pace): each man out today sees his area's schools, the
/// recruits on your board there and the coaches who hold them, and calls
/// in tonight. Returns whether the road ran today.
pub(crate) fn day(w: &mut World, prog: usize, d: Date) -> bool {
    let Some(win) = window(w, d) else { return false };
    if seat_program(w) != Some(prog) {
        return false;
    }
    super::plan::ensure(w, prog, d);
    let k = w.ground.week.monday.map_or(0, |m| m.days_to(d));
    if !(0..7).contains(&k) {
        return false;
    }
    let rows: Vec<Road> = w.ground.week.roads.iter().filter(|r| r.days & (1 << k) != 0).cloned().collect();
    if rows.is_empty() {
        return true;
    }
    let cap = dept::off_campus_recruiters(&w.programs[prog], d).unwrap_or(0) as usize;
    let mut left = left(w, prog, win);
    let board = dept::watch(w, prog);
    let by_school = board_by_school(w, &board);
    let by_area = areas::schools_by_area(w);
    let talent = standing::talents(w);
    let since = recruiting_year(d);
    let mut out = 0usize;
    let mut night = 0u16;
    for r in rows {
        if out >= cap {
            break;
        }
        if left == Some(0) {
            break;
        }
        let man = r.man as usize;
        if may_go(w, prog, man, d).is_err() {
            continue;
        }
        // A flight's first day loses half the schools to the airport.
        let first = (0..k).all(|j| r.days & (1 << j) == 0);
        let mut n = areas::schools_a_day(r.area) as usize;
        if win == Window::Fall && d.weekday() == Weekday::Fri {
            n = 1;
        } else if r.flight && first {
            n = n.div_ceil(2).max(1);
        }
        let schools = pick_schools(w, man, r.area, n, &r.focus, &by_area, &by_school, &talent, since, d);
        if schools.is_empty() {
            continue;
        }
        out += 1;
        if let Some(l) = left.as_mut() {
            *l -= 1;
        }
        // One man out one day is one day, however many schools (NCAA).
        match win {
            Window::Spring => w.scouting.dept.eval_days.2 += 1,
            Window::Fall => w.scouting.dept.eval_days.1 += 1,
            Window::Winter => {}
        }
        let mut lines: Vec<Visited> = Vec::new();
        for h in schools {
            lines.push(visit(w, prog, man, h, by_school.get(&h).map(|v| v.as_slice()).unwrap_or(&[]), d));
        }
        if w.programs[prog].head_coach == Some(man) {
            w.ground.week.book(d, &format!("On the road: {}", areas::area(r.area).name), 10.0, false);
        }
        super::phone::night_call(w, man, r.area, &lines, night, d);
        night += 1;
        // Lane G10: his road days count for his mood, and a young man riding
        // along meets the same coaches (both men's days).
        let spring = win == Window::Spring;
        crate::world::ground::staff::beats::count_day(w, man, spring);
        if let Some(y) = crate::world::ground::staff::beats::rider(w, man, d)
            && out < cap
            && left != Some(0)
            && !w.ground.week.roads.iter().any(|x| x.man == y as u32 && x.days & (1 << k) != 0)
            && may_go(w, prog, y, d).is_ok()
        {
            out += 1;
            if let Some(l) = left.as_mut() {
                *l -= 1;
            }
            match win {
                Window::Spring => w.scouting.dept.eval_days.2 += 1,
                Window::Fall => w.scouting.dept.eval_days.1 += 1,
                Window::Winter => {}
            }
            crate::world::ground::staff::beats::count_day(w, y, spring);
            let visited: Vec<usize> = lines.iter().map(|v| v.school).collect();
            crate::world::ground::staff::beats::ride_day(w, prog, y, man, &visited, d);
        }
    }
    true
}

/// One school on a man's day.
pub(crate) struct Visited {
    pub school: usize,
    pub seen: Vec<u32>,
    pub coach: Option<Who>,
    pub before: Level,
    pub after: Level,
}

#[allow(clippy::too_many_arguments)]
fn pick_schools(
    w: &World,
    man: usize,
    area: u16,
    n: usize,
    focus: &[u32],
    by_area: &[Vec<usize>],
    by_school: &std::collections::HashMap<usize, Vec<usize>>,
    talent: &[f32],
    since: Date,
    d: Date,
) -> Vec<usize> {
    let Some(list) = by_area.get(area as usize) else { return Vec::new() };
    let week = super::monday_of(d);
    let been = |h: usize| {
        w.ground.week.log.iter().any(|x| {
            x.kind == DoneKind::Visit && x.school == Some(h as u32) && x.man == Some(man as u32) && x.date >= week
        })
    };
    // Schools with your board's recruits first (the ones nobody has seen,
    // the best), then the area's schools by the talent they hold where your
    // man stands lowest with the coach.
    let mut scored: Vec<(f64, usize)> = list
        .iter()
        .map(|&h| {
            let kids = by_school.get(&h).map(|v| v.as_slice()).unwrap_or(&[]);
            let need = school_need(w, kids, since);
            let t = talent.get(h).copied().unwrap_or(1.0) as f64;
            let stand = people::head_coach_of(w, h)
                .map_or(0.0, |who| standing::points(w, who, Holder::Coach(man as u32)) as f64);
            let first = if focus.contains(&(h as u32)) { 1.0e4 } else { 0.0 };
            let s = first + 10.0 * need + 0.05 * t * (1.0 - stand / 100.0) - if been(h) { 1.0e3 } else { 0.0 };
            (s, h)
        })
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    scored.into_iter().filter(|x| x.0 > -500.0).take(n).map(|x| x.1).collect()
}

/// A man's visit to one school: a look at each of your board's recruits
/// there (an evaluation day; on a fall Friday, the game) and the coach's
/// time; with none on the board, the coach's time alone (his coordinators
/// sit in).
fn visit(w: &mut World, prog: usize, man: usize, h: usize, kids: &[usize], d: Date) -> Visited {
    let coach = people::head_coach_of(w, h);
    let before = coach.map_or(Level::Stranger, |c| standing::level(w, prog, c));
    let fall_game = window(w, d) == Some(Window::Fall) && d.weekday() == Weekday::Fri;
    let kind = if fall_game { Kind::Game } else { Kind::Practice };
    let mut seen_ids = Vec::new();
    for &i in kids {
        let id = w.people[i].player.id;
        if dept::add(w, id, COACH + man as u32, kind, d) {
            w.scouting.dept.days_log.push((d, COACH + man as u32, id));
        }
        w.scouting.dept.asks.retain(|a| a.0 != id || a.1 != Kind::Practice);
        seen_ids.push(id);
    }
    if kids.is_empty() && !w.ground.off {
        if let Some(c) = coach {
            standing::contact(w, prog, Some(man), c, Contact::Visit, 1.0, d);
        }
        let (oc, dc) = w.high_schools.get(h).map_or((None, None), |s| (s.oc, s.dc));
        for x in [oc, dc].into_iter().flatten() {
            standing::contact(w, prog, Some(man), Who::Coach(x as u32), Contact::Visit, standing::SAT_IN, d);
        }
    }
    let after = coach.map_or(Level::Stranger, |c| standing::level(w, prog, c));
    let text = visit_line(w, man, h, &seen_ids, coach, after);
    let id = w.ground.week.next();
    w.ground.week.log.push(Done {
        id,
        date: d,
        minute: 15 * 60,
        kind: DoneKind::Visit,
        man: Some(man as u32),
        area: w.high_schools.get(h).map(|s| s.ground_area),
        school: Some(h as u32),
        who: coach,
        ids: seen_ids.clone(),
        text,
        matters: false,
        heard: false,
        level: if after > before { after as u8 } else { 0 },
    });
    Visited { school: h, seen: seen_ids, coach, before, after }
}

/// "Carver (Columbus): saw Reyes and Banks at spring practice; Coach Hill,
/// Known."
fn visit_line(w: &World, man: usize, h: usize, ids: &[u32], coach: Option<Who>, lv: Level) -> String {
    let school = w.school_name(h).to_string();
    let saw = names(w, ids);
    let who = coach.map_or_else(String::new, |c| format!("; {}, {}", people::name(w, c), lv.name()));
    let man = super::last_name(w, man as u32);
    if saw.is_empty() {
        format!("{man} at {school}: the coach's time{who}")
    } else {
        format!("{man} at {school}: saw {saw}{who}")
    }
}

/// "Reyes", "Reyes and Banks", "Reyes, Banks and Cole".
pub(crate) fn names(w: &World, ids: &[u32]) -> String {
    let v: Vec<String> = ids
        .iter()
        .filter_map(|&id| crate::world::scouting::place(w, id))
        .map(|i| {
            let n = &w.people[i].name;
            n.rsplit(' ').next().unwrap_or(n).to_string()
        })
        .collect();
    match v.len() {
        0 => String::new(),
        1 => v[0].clone(),
        n => format!("{} and {}", v[..n - 1].join(", "), v[n - 1]),
    }
}
