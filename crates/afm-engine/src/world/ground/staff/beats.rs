//! Your men's areas on the map (GR68): the coordinator's proposal by each
//! man's own ground, your changes any Monday, two men on a big area, the
//! areas left empty and their misses, the ride-along, and the kids credited
//! to the man whose area they came from.

use std::collections::HashMap;

use super::{Beat, Credit, Miss, Proposal, Ride, count, last, memo, men};
use crate::calendar::{Date, Weekday};
use crate::world::World;
use crate::world::ground::areas;
use crate::world::ground::people::{self, Holder, Who};
use crate::world::ground::standing::{self, Level};
use crate::world::ground::week::road;

/// The most areas the coordinator gives one man (proposed, not confirmed).
pub const PER_MAN: usize = 3;
/// A man on his home state's ground starts ahead: how much it adds to his
/// claim on an area, in standing points (proposed, not confirmed).
pub const HOME_STATE: f32 = 12.0;
/// The longest ride-along (weeks).
pub const RIDE_WEEKS: u32 = 8;

/// A holder's standing over a set of schools: each school's head coach,
/// weighted by the talent it controls (GR27's weighting).
pub fn holder_points(w: &World, holder: Holder, schools: &[usize], talent: &[f32]) -> f32 {
    let (mut sum, mut weight) = (0.0f32, 0.0f32);
    for &h in schools {
        let t = talent.get(h).copied().unwrap_or(1.0);
        let p = people::head_coach_of(w, h).map_or(0.0, |who| standing::points(w, who, holder));
        sum += t * p;
        weight += t;
    }
    if weight > 0.0 { sum / weight } else { 0.0 }
}

/// How many of a set of schools' head coaches stand at each level with a
/// holder: (Known, Trusted, Home).
pub fn holder_levels(w: &World, holder: Holder, schools: &[usize]) -> (usize, usize, usize) {
    let (mut k, mut t, mut h) = (0, 0, 0);
    for &s in schools {
        let Some(who) = people::head_coach_of(w, s) else { continue };
        match Level::of(standing::points(w, who, holder)) {
            Level::Home => h += 1,
            Level::Trusted => t += 1,
            Level::Known => k += 1,
            Level::Stranger => {}
        }
    }
    (k, t, h)
}

/// The areas a man works, when the map's areas are set (else None: the
/// plan falls back on the built state split).
pub fn his_areas(w: &World, prog: usize, man: usize) -> Option<Vec<u16>> {
    let s = &w.ground.staff;
    if s.program != Some(prog as u32) || s.beats.is_empty() {
        return None;
    }
    Some(s.beats.iter().filter(|b| b.man == man as u32).map(|b| b.area).collect())
}

/// The men on an area, oldest first.
pub fn on_area(w: &World, prog: usize, area: u16) -> Vec<usize> {
    let s = &w.ground.staff;
    if s.program != Some(prog as u32) {
        return Vec::new();
    }
    let mut v: Vec<&Beat> = s.beats.iter().filter(|b| b.area == area).collect();
    v.sort_by_key(|b| (b.since, b.man));
    v.into_iter().map(|b| b.man as usize).collect()
}

/// The man who recruits a school for your program on the map's areas: the
/// one on its area who stands best with its head coach (None when the
/// areas aren't set, or nobody is on it).
pub fn recruiter_for_school(w: &World, prog: usize, h: usize) -> Option<usize> {
    let area = w.high_schools.get(h)?.ground_area;
    let on = on_area(w, prog, area);
    if on.len() <= 1 {
        return on.first().copied();
    }
    let coach = people::head_coach_of(w, h);
    on.into_iter().max_by(|&a, &b| {
        let pa = coach.map_or(0.0, |c| standing::points(w, c, Holder::Coach(a as u32)));
        let pb = coach.map_or(0.0, |c| standing::points(w, c, Holder::Coach(b as u32)));
        pa.total_cmp(&pb).then(b.cmp(&a))
    })
}

/// The man who holds most of a state's areas (the department's state
/// screen reads the map through it).
pub fn state_man(w: &World, prog: usize, st: usize) -> Option<usize> {
    let s = &w.ground.staff;
    if s.program != Some(prog as u32) || s.beats.is_empty() {
        return None;
    }
    let mut n: HashMap<u32, usize> = HashMap::new();
    for b in s.beats.iter().filter(|b| areas::area(b.area).state == st) {
        *n.entry(b.man).or_default() += 1;
    }
    n.into_iter().max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0))).map(|x| x.0 as usize)
}

/// What an area holds for a program: its recruits on your board (the
/// week's need), and the talent its schools control times how far the
/// program reaches there (its name in the state, the drive).
fn worth(w: &World, prog: usize, by_area: &[Vec<usize>], talent: &[f32], need: &[f64]) -> Vec<f64> {
    let p = &w.programs[prog];
    (0..areas::table().len())
        .map(|a| {
            let ar = areas::area(a as u16);
            let t: f32 = by_area[a].iter().map(|&h| talent.get(h).copied().unwrap_or(1.0)).sum();
            let foot = p.footprint.get(ar.state).copied().unwrap_or(0.0);
            let near = if areas::trip_to_area(w, prog, a as u16).flight() { 0.25 } else { 1.0 };
            let home = if ar.state == p.state { 0.6 } else { 0.0 };
            3.0 * need[a] + (t as f64 / 60.0) * (foot + home) * near
        })
        .collect()
}

/// A man's claim on an area: his own standing with its head coaches, and
/// his home state.
fn claim(w: &World, man: usize, area: u16, schools: &[usize], talent: &[f32]) -> f32 {
    let own = holder_points(w, Holder::Coach(man as u32), schools, talent);
    let home = w.coaches.get(man).and_then(|c| c.home_state) == Some(areas::area(area).state);
    own + if home { HOME_STATE } else { 0.0 }
}

/// The why of a man on an area, in words.
pub fn why_words(w: &World, man: usize, area: u16, schools: &[usize]) -> String {
    let ar = areas::area(area);
    let (k, t, h) = holder_levels(w, Holder::Coach(man as u32), schools);
    let home_state = w.coaches.get(man).and_then(|c| c.home_state) == Some(ar.state);
    let heads = schools.iter().filter(|&&s| people::head_coach_of(w, s).is_some()).count();
    if h > 0 {
        format!("his home ground: Home with {} of its {heads} head coaches", h)
    } else if t > 0 {
        format!("ground he knows: Trusted with {}", count(t, "head coach", "head coaches"))
    } else if k > 0 {
        format!("he's Known with {} of its {heads} head coaches", k)
    } else if home_state {
        "his home state, but new ground: he knows none of its head coaches yet".into()
    } else {
        "new ground: he starts a Stranger there".into()
    }
}

/// The coordinator's proposal (GR68): the areas that matter most to your
/// program, each to the man with the best claim on it (his own ground,
/// his home state), up to three a man; a man left without one goes as a
/// second man on the biggest; the rest are left empty.
pub fn propose(w: &World, prog: usize, d: Date) -> Proposal {
    let staff = men(w, prog);
    let by_area = areas::schools_by_area(w);
    let talent = standing::talents(w);
    let board = crate::world::scouting::dept::watch(w, prog);
    let by_school = road::board_by_school(w, &board);
    let since = road::recruiting_year(d);
    let mut need = vec![0.0f64; areas::table().len()];
    for (&h, kids) in &by_school {
        need[w.high_schools[h].ground_area as usize] += road::school_need(w, kids, since);
    }
    let value = worth(w, prog, &by_area, &talent, &need);
    let mut order: Vec<u16> = (0..value.len() as u16).filter(|&a| value[a as usize] > 0.05).collect();
    order.sort_by(|a, b| value[*b as usize].total_cmp(&value[*a as usize]).then(a.cmp(b)));
    order.truncate((staff.len() * 2).max(4));
    let mut load: HashMap<usize, usize> = HashMap::new();
    let mut rows: Vec<Beat> = Vec::new();
    let mut empty: Vec<u16> = Vec::new();
    for &a in &order {
        let schools = &by_area[a as usize];
        let pick = staff
            .iter()
            .copied()
            .filter(|m| load.get(m).copied().unwrap_or(0) < PER_MAN)
            .map(|m| (claim(w, m, a, schools, &talent) - 8.0 * load.get(&m).copied().unwrap_or(0) as f32, m))
            .max_by(|x, y| x.0.total_cmp(&y.0).then(y.1.cmp(&x.1)));
        match pick {
            Some((_, m)) => {
                *load.entry(m).or_default() += 1;
                rows.push(Beat { man: m as u32, area: a, since: d, yours: false, why: why_words(w, m, a, schools) });
            }
            None => empty.push(a),
        }
    }
    // A man with no area goes as a second man on the biggest one.
    for &m in &staff {
        if load.get(&m).copied().unwrap_or(0) > 0 {
            continue;
        }
        // The area with the fewest men, the biggest first among equals.
        let men_on = |a: u16| rows.iter().filter(|r| r.area == a).count();
        if let Some(&a) = order.iter().filter(|&&a| men_on(a) >= 1).min_by_key(|&&a| men_on(a)) {
            let first = rows.iter().find(|r| r.area == a).map_or(0, |r| r.man as usize);
            rows.push(Beat {
                man: m as u32,
                area: a,
                since: d,
                yours: false,
                why: format!("a second man on {}, alongside {}", areas::area(a).nickname, last(w, first)),
            });
            *load.entry(m).or_default() += 1;
        }
    }
    let letter = letter(w, &rows, &empty);
    Proposal { date: Some(d), rows, letter, empty }
}

fn letter(w: &World, rows: &[Beat], empty: &[u16]) -> Vec<String> {
    let mut by_man: Vec<(u32, Vec<&Beat>)> = Vec::new();
    for r in rows {
        match by_man.iter_mut().find(|x| x.0 == r.man) {
            Some(x) => x.1.push(r),
            None => by_man.push((r.man, vec![r])),
        }
    }
    let mut out = Vec::new();
    for (m, list) in by_man {
        let areas_txt: Vec<String> =
            list.iter().map(|b| format!("{} ({})", areas::area(b.area).nickname, b.why)).collect();
        out.push(format!("{}: {}.", super::super::week::man_words(w, m), areas_txt.join("; ")));
    }
    if !empty.is_empty() {
        let names: Vec<&str> = empty.iter().take(4).map(|&a| areas::area(a).nickname).collect();
        out.push(format!("Nobody on {}: what comes out of there, we'll hear about late.", names.join(", ")));
    }
    out
}

/// Puts the coordinator's proposal in place (your own rows go too: the
/// February meeting sets the year), with his memo.
pub(crate) fn apply(w: &mut World, prog: usize, d: Date, why: &str) {
    let p = propose(w, prog, d);
    w.ground.staff.beats = p.rows.clone();
    let lines = p.letter.clone();
    w.ground.staff.proposal = p;
    if let Some(c) = super::super::week::coordinator(w, prog) {
        memo(w, d, 7 * 60 + 30, c, format!("{why}: who works where"), lines, 1);
    }
}

/// You set a man's areas (GR68: any Monday's meeting at the map). Your
/// coordinator plans him there from the next meeting.
pub fn set_areas(w: &mut World, man: usize, list: &[u16]) -> Result<String, String> {
    let prog = super::seat_program(w).ok_or("Your staff's areas are for a college seat.")?;
    if !super::you_are_head(w) {
        return Err("Your head coach sets the areas.".into());
    }
    if !men(w, prog).contains(&man) {
        return Err("He isn't one of your on-field assistants.".into());
    }
    if list.iter().any(|&a| a as usize >= areas::table().len()) {
        return Err("No such area.".into());
    }
    if list.len() > PER_MAN + 1 {
        return Err(format!("{} areas is more than one man can work.", list.len()));
    }
    ensure(w, prog, w.today);
    let d = w.today;
    let by_area = areas::schools_by_area(w);
    w.ground.staff.beats.retain(|b| b.man != man as u32);
    for &a in list {
        let why = format!("your call: {}", why_words(w, man, a, &by_area[a as usize]));
        w.ground.staff.beats.push(Beat { man: man as u32, area: a, since: d, yours: true, why });
    }
    let when = if d.weekday() == Weekday::Mon { "this week's plan on" } else { "next Monday's plan on" };
    let names: Vec<&str> = list.iter().map(|&a| areas::area(a).nickname).collect();
    Ok(if list.is_empty() {
        format!("{} has no area now: your coordinator sends him where the board needs him, from {when}.", last(w, man))
    } else {
        format!("{} works {} from {when}.", last(w, man), names.join(", "))
    })
}

/// The beats exist for your program (the coordinator's first proposal on
/// the first day of a job, or a file from before lane G10).
pub(crate) fn ensure(w: &mut World, prog: usize, d: Date) {
    if w.ground.staff.program == Some(prog as u32) && w.ground.staff.beats.is_empty() && !men(w, prog).is_empty() {
        apply(w, prog, d, "Your staff's areas");
    }
}

/// A young coach rides along with a veteran (GR68): the coaches the
/// veteran visits meet him too, and he keeps them when the veteran leaves.
/// It costs both men's road days.
pub fn ride(w: &mut World, young: usize, veteran: usize, weeks: u32) -> Result<String, String> {
    let prog = super::seat_program(w).ok_or("Ride-alongs are for a college seat.")?;
    if !super::you_are_head(w) {
        return Err("Your head coach decides who rides along.".into());
    }
    let staff = men(w, prog);
    if !staff.contains(&young) || !staff.contains(&veteran) || young == veteran {
        return Err("Two different men on your staff.".into());
    }
    if !(1..=RIDE_WEEKS).contains(&weeks) {
        return Err(format!("One to {RIDE_WEEKS} weeks."));
    }
    if w.coaches[young].age >= w.coaches[veteran].age {
        return Err(format!("{} is the younger man: he rides with {}.", last(w, veteran), last(w, young)));
    }
    let d = w.today;
    let s = &mut w.ground.staff;
    s.rides.retain(|r| r.young != young as u32);
    s.rides.push(Ride {
        young: young as u32,
        veteran: veteran as u32,
        from: d,
        to: d.add(7 * weeks as i32 - 1),
        days: 0,
    });
    Ok(format!(
        "{} rides along with {} for {}: each day costs both men's road days, and the coaches they see know {} on his own.",
        last(w, young),
        last(w, veteran),
        count(weeks as usize, "week", "weeks"),
        last(w, young)
    ))
}

/// Ends a ride-along.
pub fn end_ride(w: &mut World, young: usize) -> Result<String, String> {
    let before = w.ground.staff.rides.len();
    w.ground.staff.rides.retain(|r| r.young != young as u32);
    if w.ground.staff.rides.len() == before {
        return Err("He isn't riding along with anyone.".into());
    }
    Ok(format!("{} works on his own again.", last(w, young)))
}

/// The young man riding with a veteran today, if any.
pub(crate) fn rider(w: &World, veteran: usize, d: Date) -> Option<usize> {
    w.ground
        .staff
        .rides
        .iter()
        .find(|r| r.veteran == veteran as u32 && r.from <= d && d <= r.to)
        .map(|r| r.young as usize)
}

/// A rider's day: the coaches at the veteran's schools meet him too.
pub(crate) fn ride_day(w: &mut World, prog: usize, young: usize, veteran: usize, schools: &[usize], d: Date) {
    let mut met = 0;
    for &h in schools {
        if let Some(c) = people::head_coach_of(w, h) {
            standing::contact(w, prog, Some(young), c, standing::Contact::Visit, 1.0, d);
            met += 1;
        }
        let (oc, dc) = w.high_schools.get(h).map_or((None, None), |s| (s.oc, s.dc));
        for x in [oc, dc].into_iter().flatten() {
            standing::contact(
                w,
                prog,
                Some(young),
                Who::Coach(x as u32),
                standing::Contact::Visit,
                standing::SAT_IN,
                d,
            );
        }
    }
    if let Some(r) = w.ground.staff.rides.iter_mut().find(|r| r.young == young as u32) {
        r.days += 1;
    }
    let area = schools.first().and_then(|&h| w.high_schools.get(h)).map(|s| s.ground_area);
    let text = format!(
        "{} rode along with {}{}: met {}.",
        last(w, young),
        last(w, veteran),
        area.map_or(String::new(), |a| format!(" in {}", areas::area(a).nickname)),
        count(met, "head coach", "head coaches")
    );
    let id = w.ground.week.next();
    w.ground.week.log.push(super::super::week::Done {
        id,
        date: d,
        minute: 16 * 60,
        kind: super::super::week::DoneKind::Visit,
        man: Some(young as u32),
        area,
        school: None,
        who: None,
        ids: Vec::new(),
        text,
        matters: false,
        heard: false,
        level: 0,
    });
}

/// One road day for a man, counted for his mood (a long spring away).
pub(crate) fn count_day(w: &mut World, man: usize, spring: bool) {
    let season = w.dates.season;
    let s = &mut w.ground.staff;
    match s.out.iter_mut().find(|x| x.0 == man as u32 && x.1 == season) {
        Some(x) => {
            if spring {
                x.2 += 1
            } else {
                x.3 += 1
            }
        }
        None => s.out.push((man as u32, season, spring as u16, (!spring) as u16)),
    }
    s.out.retain(|x| x.1 >= season - 2);
}

/// A man's road days this season: (spring, fall).
pub fn days_out(w: &World, man: usize) -> (u16, u16) {
    let season = w.dates.season;
    w.ground.staff.out.iter().find(|x| x.0 == man as u32 && x.1 == season).map_or((0, 0), |x| (x.2, x.3))
}

/// Each day: the first proposal, the February meeting, Monday's orphans and
/// rides, and the week's signees (credits and misses).
pub(crate) fn day(w: &mut World, prog: usize, d: Date) {
    ensure(w, prog, d);
    if d == super::february_meeting(w, d) {
        apply(w, prog, d, "The February meeting");
    }
    if d.weekday() == Weekday::Mon {
        orphans(w, prog, d);
        w.ground.staff.rides.retain(|r| r.to >= d);
    }
    let signing = w.dates.early_signing.is_some_and(|e| d == e.add(1)) || d == w.dates.signing_day.add(1);
    if d.weekday() == Weekday::Mon || signing {
        signees(w, prog, d);
    }
}

/// Areas whose man left: the coordinator gives each to the man with the
/// fewest, until you or the February meeting say otherwise.
fn orphans(w: &mut World, prog: usize, d: Date) {
    let staff = men(w, prog);
    if staff.is_empty() {
        return;
    }
    w.ground.staff.beats.retain(|b| staff.contains(&(b.man as usize)));
    let held: Vec<u16> = w.ground.staff.beats.iter().map(|b| b.area).collect();
    let lost: Vec<u16> = w
        .ground
        .staff
        .proposal
        .rows
        .iter()
        .map(|b| b.area)
        .filter(|a| !held.contains(a))
        .collect::<std::collections::BTreeSet<u16>>()
        .into_iter()
        .collect();
    if lost.is_empty() {
        return;
    }
    let by_area = areas::schools_by_area(w);
    let mut lines = Vec::new();
    for a in lost {
        let load = |m: usize, s: &[Beat]| s.iter().filter(|b| b.man == m as u32).count();
        let Some(&m) = staff.iter().min_by_key(|&&m| (load(m, &w.ground.staff.beats), m)) else { continue };
        if load(m, &w.ground.staff.beats) >= PER_MAN + 1 {
            continue;
        }
        let why = format!("covering until a new man climbs: {}", why_words(w, m, a, &by_area[a as usize]));
        lines.push(format!("{} takes {} ({why}).", last(w, m), areas::area(a).nickname));
        w.ground.staff.beats.push(Beat { man: m as u32, area: a, since: d, yours: false, why });
    }
    if !lines.is_empty()
        && let Some(c) = super::super::week::coordinator(w, prog)
    {
        memo(w, d, 7 * 60 + 30, c, "Who covers the open areas".into(), lines, 1);
    }
}

/// The week's signees: each of yours is credited to the man on his area;
/// a 3-star or better from an area nobody of yours works who signed
/// elsewhere is a miss (GR67).
fn signees(w: &mut World, prog: usize, d: Date) {
    let since = w.ground.staff.looked.unwrap_or(d.add(-7));
    w.ground.staff.looked = Some(d);
    let home = w.programs[prog].state;
    let empty: Vec<u16> = {
        let s = &w.ground.staff;
        let mut v: Vec<u16> = s.proposal.empty.clone();
        for &a in areas::in_state(home) {
            if !s.beats.iter().any(|b| b.area == a) {
                v.push(a);
            }
        }
        v
    };
    let mut credits = Vec::new();
    let mut misses = Vec::new();
    for q in &w.people {
        let Some(s) = q.cal.signed else { continue };
        if s.walk_on || s.date < since || s.date > d {
            continue;
        }
        let Some(hs) = w.high_schools.get(q.hs) else { continue };
        let area = hs.ground_area;
        let stars = q.stars;
        if s.program == prog {
            if w.ground.staff.credits.iter().any(|c| c.id == q.player.id) {
                continue;
            }
            if let Some(m) = recruiter_for_school(w, prog, q.hs) {
                credits.push(Credit { id: q.player.id, man: m as u32, date: s.date, stars, area });
            }
        } else if stars >= 3 && empty.contains(&area) && !w.ground.staff.misses.iter().any(|m| m.id == q.player.id) {
            misses.push(Miss { id: q.player.id, area, to: s.program as u32, stars, date: s.date });
        }
    }
    let st = &mut w.ground.staff;
    st.credits.extend(credits);
    st.misses.extend(misses);
    let cut = d.add(-6 * 365);
    st.credits.retain(|c| c.date >= cut);
    st.misses.retain(|m| m.date >= d.add(-2 * 365));
}

/// The department's state command on the map's areas: every area of the
/// state to one man (yours), or back to the coordinator.
pub(crate) fn set_state(w: &mut World, st: usize, coach: Option<usize>) {
    let Some(prog) = super::seat_program(w) else { return };
    if w.ground.staff.program != Some(prog as u32) || w.ground.staff.beats.is_empty() {
        return;
    }
    let list = areas::in_state(st);
    w.ground.staff.beats.retain(|b| !list.contains(&b.area));
    if let Some(c) = coach {
        let d = w.today;
        for &a in list {
            w.ground.staff.beats.push(Beat {
                man: c as u32,
                area: a,
                since: d,
                yours: true,
                why: "your call: the whole state".into(),
            });
        }
    }
}
