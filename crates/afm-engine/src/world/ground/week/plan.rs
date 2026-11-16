//! The meeting at the map (GR47): your coordinator's draft of the week and
//! your changes to it, the week's calls for you (GR53), the week's scenes
//! with his reasons (GR51), the Monday letter (GR56), and the May list
//! (GR57).

use super::road::{self, Window};
use super::{
    Done, DoneKind, MayList, Planned, Road, Slot, SlotKind, beat, coordinator, last_name, monday_of, seat_program,
};
use crate::calendar::{Date, Weekday};
use crate::world::ground::areas;
use crate::world::ground::people::{self, Who};
use crate::world::ground::standing::{self, Contact, Level};
use crate::world::scouting::dept::{self, Target};
use crate::world::scouting::readers::COACH;
use crate::world::scouting::{Kind, place};
use crate::world::{Stage, World};

/// The most days a man spends in one area in a week: three school days in
/// the spring and January, the fall's Thursday and Friday (proposed, not
/// confirmed).
pub const DAYS_PER_MAN: [u32; 3] = [3, 2, 3];
/// The calls the coordinator lists for you a week.
pub const CALLS: usize = 5;
/// The scenes a week that come to you (GR51), the Monday meeting included.
pub const YOURS: (usize, usize) = (3, 5);
/// Official visitors a weekend after the road, by division (FBS, FCS,
/// below; proposed, not confirmed).
pub const PER_WEEKEND: [u32; 3] = [10, 6, 4];

fn win_index(w: Window) -> usize {
    match w {
        Window::Spring => 0,
        Window::Fall => 1,
        Window::Winter => 2,
    }
}

/// The plan for this day's week exists (drafted the first time it's
/// asked for).
pub(crate) fn ensure(w: &mut World, prog: usize, d: Date) {
    if w.ground.week.monday != Some(monday_of(d)) {
        turn(w, prog, d);
    }
}

/// The plan for today's week (the screens ask for it any day).
pub fn ensure_now(w: &mut World, prog: usize) {
    let d = w.today;
    ensure(w, prog, d);
}

/// The week turns: last week's letter written from what it bought, the
/// board's names kept, and the coordinator's draft for this one.
pub(crate) fn turn(w: &mut World, prog: usize, d: Date) {
    let monday = monday_of(d);
    if w.ground.week.monday == Some(monday) {
        return;
    }
    let had = w.ground.week.monday.is_some();
    w.ground.week.monday = Some(monday);
    w.ground.week.handed = false;
    draft(w, prog, monday, d);
    let board: Vec<u32> = {
        let mut v: Vec<u32> = dept::watch(w, prog).into_iter().map(|i| w.people[i].player.id).collect();
        v.sort_unstable();
        v
    };
    let lines = letter(w, prog, monday, if had { Some(&w.ground.week.board) } else { None }, &board);
    let wk = &mut w.ground.week;
    wk.board = board;
    wk.letters.retain(|x| x.0 != monday);
    wk.letters.push((monday, lines));
    if wk.letters.len() > 6 {
        wk.letters.remove(0);
    }
}

/// The open road days of a week from `from` (the days a window lets your
/// staff out on), with the window.
fn open_days(w: &World, monday: Date, from: Date) -> Vec<(u32, Window)> {
    (0..7)
        .filter_map(|k| {
            let d = monday.add(k);
            let win = road::window(w, d)?;
            (d >= from && road::day_kind(win, d)).then_some((k as u32, win))
        })
        .collect()
}

/// The coordinator's order of the week's days: Tuesday first (Monday is
/// the meeting), then on through the week; the fall's Thursday, Friday.
fn day_order(open: &[(u32, Window)]) -> Vec<u32> {
    let pref = [1u32, 2, 3, 0, 4];
    let mut v: Vec<u32> = pref.iter().copied().filter(|k| open.iter().any(|o| o.0 == *k)).collect();
    v.extend(open.iter().map(|o| o.0).filter(|k| !pref.contains(k)));
    v
}

/// What each area holds for your board this week, and which of its
/// recruits nobody of yours has seen in person.
fn area_need(w: &World, prog: usize, d: Date) -> (Vec<f64>, std::collections::HashMap<usize, Vec<usize>>) {
    let board = dept::watch(w, prog);
    let by_school = road::board_by_school(w, &board);
    let since = road::recruiting_year(d);
    let mut need = vec![0.0f64; areas::table().len()];
    for (&h, kids) in &by_school {
        let a = w.high_schools[h].ground_area as usize;
        need[a] += road::school_need(w, kids, since);
    }
    (need, by_school)
}

/// The best area for a set of kids (an assignment): where most of them are.
fn area_of_kids(w: &World, kids: &[usize]) -> Option<u16> {
    let mut count: std::collections::HashMap<u16, f64> = std::collections::HashMap::new();
    let since = road::recruiting_year(w.today);
    for (k, &i) in kids.iter().enumerate() {
        let q = &w.people[i];
        if let Some(s) = w.high_schools.get(q.hs) {
            let unseen = if road::seen(w, i, since) { 0.2 } else { 1.0 };
            *count.entry(s.ground_area).or_default() += unseen / (1.0 + k as f64 * 0.05);
        }
    }
    count.into_iter().max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0))).map(|x| x.0)
}

/// The states a man recruits: the areas you set, else the staff's split.
fn his_states(w: &World, prog: usize, man: usize) -> Vec<u8> {
    if let Some((_, v)) = w.scouting.dept.areas.iter().find(|a| a.0 == man) {
        return v.clone();
    }
    (0..w.states.len().min(51) as u8).filter(|&st| dept::area_coach(w, prog, st) == Some(man)).collect()
}

/// The coordinator drafts the week: your assignments first (a coach sent
/// to a player, a state, a position group or cross-checks), your asks for
/// a live look, then each man to the area of his states that needs him
/// most, the days spread to keep the window's pace, flights while the
/// week's money lasts. Your calls and the week's scenes come with it.
pub(crate) fn draft(w: &mut World, prog: usize, monday: Date, from: Date) {
    let roads = draft_roads(w, prog, monday, from);
    w.ground.week.roads = roads;
    w.ground.week.calls = draft_calls(w, prog, monday);
    w.ground.week.slots = draft_slots(w, prog, monday);
}

fn draft_roads(w: &World, prog: usize, monday: Date, from: Date) -> Vec<Road> {
    let open = open_days(w, monday, from);
    if open.is_empty() {
        return Vec::new();
    }
    let win = open[0].1;
    let cap = dept::off_campus_recruiters(&w.programs[prog], monday.add(open[0].0 as i32)).unwrap_or(0) as usize;
    let mut men: Vec<usize> = road::men(w, prog)
        .into_iter()
        .filter(|&m| road::may_go(w, prog, m, monday.add(open[0].0 as i32)).is_ok())
        .collect();
    // The head coach is on the road only where the coordinator would send
    // him: January's contact period, and the spring at FCS.
    let hc = w.programs[prog].head_coach;
    let fbs = w.programs[prog].division == crate::world::data::Division::FBS;
    if win == Window::Fall || (win == Window::Spring && fbs) {
        men.retain(|&m| Some(m) != hc);
    }
    if men.is_empty() || cap == 0 {
        return Vec::new();
    }
    let per_man = DAYS_PER_MAN[win_index(win)].min(open.len() as u32);
    let budget = match road::left(w, prog, win) {
        Some(left) => {
            let ahead = road::open_days_from(w, win, monday.add(open[0].0 as i32)).max(1);
            let pace = (left as f64 * open.len() as f64 / ahead as f64).ceil() as u32;
            pace.min(left as u32)
        }
        None => (men.len().min(cap) as u32) * per_man,
    };
    if budget == 0 {
        return Vec::new();
    }
    let (need, by_school) = area_need(w, prog, monday);
    let order = day_order(&open);
    // Each man's area and reason, in priority.
    let mut picks: Vec<(usize, u16, f64, String, Vec<u32>)> = Vec::new();
    let taken = |picks: &[(usize, u16, f64, String, Vec<u32>)], a: u16| picks.iter().filter(|p| p.1 == a).count();
    // Your assignments first (minor bug 265 kept: the man goes where you sent him).
    for a in &w.scouting.dept.assignments {
        if a.who < COACH {
            continue;
        }
        let man = (a.who - COACH) as usize;
        if !men.contains(&man) || picks.iter().any(|p| p.0 == man) {
            continue;
        }
        let schools_of = |kids: &[usize]| -> Vec<u32> {
            let mut v: Vec<u32> = kids.iter().map(|&i| w.people[i].hs as u32).collect();
            v.dedup();
            v
        };
        let (area, why, focus) = match a.target {
            Target::Player(id) => match place(w, id).and_then(|i| w.high_schools.get(w.people[i].hs).map(|s| (i, s))) {
                Some((i, s)) => (
                    Some(s.ground_area),
                    format!("Your assignment: see {}", w.people[i].name),
                    vec![w.people[i].hs as u32],
                ),
                None => (None, String::new(), Vec::new()),
            },
            Target::Region(st) => {
                let best = areas::in_state(st as usize)
                    .iter()
                    .copied()
                    .max_by(|x, y| need[*x as usize].total_cmp(&need[*y as usize]).then(y.cmp(x)));
                (
                    best,
                    format!(
                        "Your assignment: {}",
                        w.states
                            .get(st as usize)
                            .map_or("his state", |s| crate::world::folk::facts::state_name(&s.code))
                    ),
                    Vec::new(),
                )
            }
            Target::Group(g) => {
                let mut kids: Vec<usize> = by_school
                    .values()
                    .flatten()
                    .copied()
                    .filter(|&i| crate::world::coach::Group::of(w.people[i].player.position) as u8 == g)
                    .collect();
                kids.sort_by_key(|&i| (std::cmp::Reverse(w.people[i].stars), i));
                (
                    area_of_kids(w, &kids),
                    format!(
                        "Your assignment: the {}",
                        crate::world::coach::Group::ALL[(g as usize).min(7)].name().to_lowercase()
                    ),
                    schools_of(&kids),
                )
            }
            Target::CrossCheck => {
                let board = dept::watch(w, prog);
                let list = dept::cross_check_list(w, &board, a.who, Some(road::recruiting_year(monday)));
                (area_of_kids(w, &list), "Your assignment: cross-checks".to_string(), schools_of(&list))
            }
        };
        if let Some(area) = area {
            let focus = focus
                .into_iter()
                .filter(|&h| w.high_schools.get(h as usize).is_some_and(|s| s.ground_area == area))
                .collect();
            picks.push((man, area, f64::MAX, why, focus));
        }
    }
    // Your asks for a live look go to the kid's position coach.
    for a in w.scouting.dept.asks.iter().filter(|a| a.1 == Kind::Practice) {
        let Some(i) = place(w, a.0) else { continue };
        let Some(man) = dept::evaluator_for(w, prog, i) else { continue };
        if !men.contains(&man) || picks.iter().any(|p| p.0 == man) {
            continue;
        }
        if let Some(s) = w.high_schools.get(w.people[i].hs) {
            picks.push((
                man,
                s.ground_area,
                f64::MAX / 2.0,
                format!("Your ask: a live look at {}", w.people[i].name),
                vec![w.people[i].hs as u32],
            ));
        }
    }
    // Everyone else to the area of his states that needs him most.
    let mut rest: Vec<(usize, Vec<u16>)> = men
        .iter()
        .copied()
        .filter(|m| !picks.iter().any(|p| p.0 == *m))
        .map(|m| {
            let mut v: Vec<u16> = his_states(w, prog, m)
                .into_iter()
                .flat_map(|st| areas::in_state(st as usize).iter().copied())
                .collect();
            v.sort_by(|a, b| need[*b as usize].total_cmp(&need[*a as usize]).then(a.cmp(b)));
            (m, v)
        })
        .collect();
    rest.sort_by(|a, b| {
        let na = a.1.first().map_or(0.0, |&x| need[x as usize]);
        let nb = b.1.first().map_or(0.0, |&x| need[x as usize]);
        nb.total_cmp(&na).then(a.0.cmp(&b.0))
    });
    let mut all: Vec<u16> = (0..need.len() as u16).filter(|&a| need[a as usize] > 0.0).collect();
    all.sort_by(|a, b| need[*b as usize].total_cmp(&need[*a as usize]).then(a.cmp(b)));
    for (man, mine) in rest {
        let free = |a: u16, picks: &[(usize, u16, f64, String, Vec<u32>)]| {
            need[a as usize] > 0.0 && (taken(picks, a) == 0 || taken(picks, a) == 1 && need[a as usize] > 12.0)
        };
        let home = w.coaches.get(man).and_then(|c| c.home_state);
        let pick = mine
            .iter()
            .copied()
            .find(|&a| free(a, &picks))
            .map(|a| (a, true))
            .or_else(|| all.iter().copied().find(|&a| free(a, &picks)).map(|a| (a, false)));
        if let Some((a, his)) = pick {
            let ar = areas::area(a);
            let kids: Vec<usize> = by_school
                .iter()
                .filter(|(h, _)| w.high_schools[**h].ground_area == a)
                .flat_map(|(_, v)| v.iter().copied())
                .collect();
            let since = road::recruiting_year(monday);
            let unseen = kids.iter().filter(|&&i| !road::seen(w, i, since)).count();
            let mut why = format!("{} on the board in {}, {} nobody's seen in person", kids.len(), ar.nickname, unseen);
            if home == Some(ar.state) {
                why.push_str("; his home state");
            } else if his {
                why.push_str("; his area");
            }
            picks.push((man, a, need[a as usize], why, Vec::new()));
        }
    }
    // Days, in priority, while the budget and the money last.
    picks.sort_by(|a, b| b.2.total_cmp(&a.2).then(a.0.cmp(&b.0)));
    let mut rows: Vec<Road> = Vec::new();
    let mut left = budget;
    let mut per_day = [0usize; 7];
    for (man, area, _, why, focus) in picks {
        if left == 0 {
            break;
        }
        let t = areas::trip_to_area(w, prog, area);
        if t.flight() {
            let p = road::purse(w, prog, &rows);
            if p.flights <= p.flown {
                continue;
            }
        }
        let mut days = 0u8;
        let mut n = 0;
        for &k in &order {
            if n >= per_man || left == 0 {
                break;
            }
            if per_day[k as usize] >= cap {
                continue;
            }
            days |= 1 << k;
            per_day[k as usize] += 1;
            n += 1;
            left -= 1;
        }
        if days != 0 {
            rows.push(Road { man: man as u32, area, days, flight: t.flight(), why, yours: false, focus });
        }
    }
    rows
}

/// The week's calls for you (GR53: the phone is an FBS head coach's spring
/// road): coaches slipping, coaches one contact from Known with a kid on
/// your board, and the ones in areas nobody of yours reaches this week.
fn draft_calls(w: &World, prog: usize, monday: Date) -> Vec<Planned> {
    let (_, by_school) = area_need(w, prog, monday);
    let covered: Vec<u16> = w.ground.week.roads.iter().map(|r| r.area).collect();
    let talent = standing::talents(w);
    let mut cand: Vec<(f64, Who, String)> = Vec::new();
    let add = |score: f64, who: Who, why: String, cand: &mut Vec<(f64, Who, String)>| match cand
        .iter_mut()
        .find(|c| c.1 == who)
    {
        Some(c) => {
            if score > c.0 {
                *c = (score, who, why);
            }
        }
        None => cand.push((score, who, why)),
    };
    for m in &w.ground.memories {
        if m.warned > 0
            && let Who::Coach(_) = m.who
            && let Some(h) = people::school(w, m.who)
        {
            let t = talent.get(h).copied().unwrap_or(1.0) as f64;
            add(3.0 + t.ln_1p(), m.who, format!("Slipping: no contact since {}", beat::short(m.last)), &mut cand);
        }
    }
    let since = road::recruiting_year(monday);
    for (&h, kids) in &by_school {
        let Some(who) = people::head_coach_of(w, h) else { continue };
        let best = kids.iter().copied().max_by_key(|&i| (w.people[i].stars, std::cmp::Reverse(i)));
        let Some(best) = best else { continue };
        let q = &w.people[best];
        let lv = standing::level(w, prog, who);
        let t = talent.get(h).copied().unwrap_or(1.0) as f64;
        let mem_contacts = standing::holders(w, prog)
            .into_iter()
            .filter_map(|x| w.ground.get(who, x))
            .map(|m| m.contacts)
            .max()
            .unwrap_or(0);
        let stars = if q.stars > 0 { format!(" ({}★)", q.stars) } else { String::new() };
        if lv == Level::Stranger && mem_contacts == 1 {
            add(
                2.0 + t.ln_1p(),
                who,
                format!("One more contact and he gives you his list: his {}{stars}", q.name),
                &mut cand,
            );
        } else if !covered.contains(&w.high_schools[h].ground_area) && lv < Level::Trusted && q.stars >= 3 {
            let seen = if road::seen(w, best, since) { "" } else { ", nobody's seen him in person" };
            add(
                1.0 + t.ln_1p(),
                who,
                format!("Nobody of yours is in his area this week: {}{stars}{seen}", q.name),
                &mut cand,
            );
        }
    }
    cand.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    cand.into_iter().take(CALLS).map(|(_, who, why)| Planned { who, why, state: 0 }).collect()
}

/// The week's scenes (GR51), each with the coordinator's reason: the
/// meeting, what you host, your calls, your own days on the road. Three to
/// five come to you, by what matters most; he plays the rest.
fn draft_slots(w: &World, prog: usize, monday: Date) -> Vec<Slot> {
    use crate::world::Visit;
    let mut v: Vec<(u8, Slot)> = Vec::new();
    let slot = |date: Date, minute: u16, kind: SlotKind, title: String, reason: &str| Slot {
        date,
        minute,
        kind,
        title,
        reason: reason.to_string(),
        yours: false,
        man: None,
        who: None,
        school: None,
        line: None,
    };
    v.push((9, slot(monday, 8 * 60, SlotKind::Meeting, "The meeting at the map".into(), "The week's one planner")));
    let mut days: Vec<(Date, usize, usize)> = (0..7).map(|k| (monday.add(k), 0, 0)).collect();
    for q in w.people.iter().filter(|q| matches!(q.stage, Stage::HighSchool | Stage::PrepSchool)) {
        for vi in q.cal.visits.iter().filter(|vi| vi.0 == prog && vi.1 >= monday && vi.1 < monday.add(7)) {
            let k = monday.days_to(vi.1) as usize;
            match vi.2 {
                Visit::Official => days[k].1 += 1,
                Visit::Unofficial | Visit::DropIn => days[k].2 += 1,
                Visit::Home => {}
            }
        }
    }
    let jd = w.dates.junior_days;
    for (d, off, un) in days {
        if off > 0 && d.weekday() == Weekday::Sat {
            v.push((
                8,
                slot(
                    d,
                    10 * 60,
                    SlotKind::Official,
                    format!("Official visit weekend: {off} on campus"),
                    "The biggest pull there is",
                ),
            ));
        }
        if un > 0 && d.weekday() == Weekday::Sat && d >= jd.0 && d <= jd.1 {
            v.push((
                7,
                slot(
                    d,
                    9 * 60,
                    SlotKind::JuniorDay,
                    format!("Junior day: {un} juniors"),
                    "The families' first look at you",
                ),
            ));
        } else if un > 0 {
            v.push((
                4,
                slot(d, 10 * 60, SlotKind::Unofficial, format!("Unofficial visits: {un}"), "Families on campus"),
            ));
        }
    }
    for d in beat::camp_days(w.dates.season) {
        if d >= monday && d < monday.add(7) {
            v.push((
                7,
                slot(d, 8 * 60, SlotKind::Camp, "Your camp".into(), "Times, names, and the coaches who brought them"),
            ));
        }
    }
    for c in &w.ground.week.calls {
        let mut s =
            slot(monday.add(1), 13 * 60, SlotKind::CoachCall, format!("Call {}", people::name(w, c.who)), &c.why);
        s.who = Some(c.who);
        s.school = people::school(w, c.who).map(|h| h as u32);
        v.push((if c.why.starts_with("Slipping") { 6 } else { 3 }, s));
    }
    let hc = w.programs[prog].head_coach.map(|c| c as u32);
    for r in w.ground.week.roads.iter().filter(|r| Some(r.man) == hc) {
        let first = (0..7).find(|k| r.days & (1 << k) != 0).unwrap_or(1);
        let mut s = slot(
            monday.add(first),
            9 * 60,
            SlotKind::SchoolVisit,
            format!("On the road: {}", areas::area(r.area).name),
            "You're the one in the room",
        );
        s.man = Some(r.man);
        v.push((6, s));
    }
    // Lane G5: your clinic, open practices, the spring game and the
    // convention. The spring game holds that Saturday's unofficial visits.
    let coaches = super::super::scenes::coach_slots(w, prog, monday);
    let game: Vec<Date> = coaches.iter().filter(|x| x.1.kind == SlotKind::SpringGame).map(|x| x.1.date).collect();
    v.retain(|x| !(x.1.kind == SlotKind::Unofficial && game.contains(&x.1.date)));
    v.extend(coaches);
    v.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.date.cmp(&b.1.date)));
    let n = v.len().min(YOURS.1);
    for (k, (_, s)) in v.iter_mut().enumerate() {
        s.yours = k < n;
    }
    let mut out: Vec<Slot> = v.into_iter().map(|x| x.1).collect();
    out.sort_by_key(|s| (s.date, s.minute));
    out
}

// --- Your changes at the map ----------------------------------------------------------------

fn weekday_bits(days: &[u32]) -> u8 {
    days.iter().filter(|&&k| k < 7).fold(0u8, |b, &k| b | (1 << k))
}

/// You send a man to an area on days of this week (0 Monday to 6
/// Sunday), in place of his row: the rules (the window, the FBS head
/// coach's spring, the off-campus limit) and the week's flights are
/// checked here.
pub fn set_road(w: &mut World, man: u32, area: u16, days: &[u32]) -> Result<String, String> {
    let prog = seat_program(w).ok_or("Ground's week is for a college seat.")?;
    ensure(w, prog, w.today);
    let monday = w.ground.week.monday.unwrap_or(monday_of(w.today));
    if area as usize >= areas::table().len() {
        return Err("No such area.".into());
    }
    let bits = weekday_bits(days);
    if bits == 0 {
        return Err("Pick his days.".into());
    }
    for k in 0..7 {
        if bits & (1 << k) == 0 {
            continue;
        }
        let d = monday.add(k);
        if d < w.today {
            return Err(format!("{} has gone.", beat::short(d)));
        }
        let win = road::window(w, d)
            .ok_or(format!("{} isn't a road day: the rules keep your staff home.", beat::short(d)))?;
        if !road::day_kind(win, d) {
            return Err(format!("{}: the schools aren't open to you that day.", beat::short(d)));
        }
        road::may_go(w, prog, man as usize, d)?;
        let cap = dept::off_campus_recruiters(&w.programs[prog], d).unwrap_or(0) as usize;
        let out = w.ground.week.roads.iter().filter(|r| r.man != man && r.days & (1 << k) != 0).count();
        if out >= cap {
            return Err(format!("{} men are out {} already, the most the rules allow.", cap, beat::short(d)));
        }
    }
    let t = areas::trip_to_area(w, prog, area);
    let others: Vec<Road> = w.ground.week.roads.iter().filter(|r| r.man != man).cloned().collect();
    if t.flight() {
        let p = road::purse(w, prog, &others);
        if p.flights <= p.flown {
            return Err(format!(
                "That's a flight, and this week's money covers {}{}.",
                road::flights_words(p.flights),
                if p.flown > 0 { " already taken" } else { "" }
            ));
        }
    }
    let ar = areas::area(area);
    let row =
        Road { man, area, days: bits, flight: t.flight(), why: "Your call".into(), yours: true, focus: Vec::new() };
    w.ground.week.roads.retain(|r| r.man != man);
    w.ground.week.roads.push(row);
    let n = bits.count_ones();
    Ok(format!(
        "{} to {} ({}), {} {}: {}.",
        last_name(w, man),
        ar.name,
        ar.nickname,
        n,
        if n == 1 { "day" } else { "days" },
        t.words()
    ))
}

/// You keep a man home this week.
pub fn clear_road(w: &mut World, man: u32) -> Result<String, String> {
    let prog = seat_program(w).ok_or("Ground's week is for a college seat.")?;
    ensure(w, prog, w.today);
    let before = w.ground.week.roads.len();
    w.ground.week.roads.retain(|r| r.man != man);
    if w.ground.week.roads.len() == before {
        return Err("He isn't on the road this week.".into());
    }
    Ok(format!("{} stays home this week.", last_name(w, man)))
}

/// You hand the week to your coordinator (GR51): he plays your scenes, and
/// only live moments stop you.
pub fn hand(w: &mut World, on: bool) -> Result<String, String> {
    let prog = seat_program(w).ok_or("Ground's week is for a college seat.")?;
    ensure(w, prog, w.today);
    w.ground.week.handed = on;
    for s in w.ground.week.slots.iter_mut().filter(|s| s.kind != SlotKind::Meeting) {
        if on {
            s.yours = false;
        }
    }
    if !on {
        w.ground.week.slots = draft_slots(w, prog, w.ground.week.monday.unwrap_or(monday_of(w.today)));
    }
    Ok(if on {
        "Your coordinator has the week. You'll read the Monday letter, and only a call that matters stops you.".into()
    } else {
        "The week is yours again.".into()
    })
}

/// Saturday: your coordinator makes the calls on your list you didn't get
/// to, as his own (a third of a visit at most, GR53).
pub(crate) fn coordinator_calls(w: &mut World, prog: usize, d: Date) {
    let Some(coord) = coordinator(w, prog) else { return };
    let todo: Vec<Who> = w.ground.week.calls.iter().filter(|c| c.state == 0).map(|c| c.who).collect();
    for who in todo {
        if !w.ground.off {
            standing::contact(w, prog, Some(coord), who, Contact::Call, 1.0, d);
        }
        if let Some(c) = w.ground.week.calls.iter_mut().find(|c| c.who == who) {
            c.state = 2;
        }
        let id = w.ground.week.next();
        let text = format!("{} called {} for you.", last_name(w, coord as u32), people::name(w, who));
        w.ground.week.log.push(Done {
            id,
            date: d,
            minute: 10 * 60,
            kind: DoneKind::Call,
            man: Some(coord as u32),
            area: people::school(w, who).and_then(|h| w.high_schools.get(h)).map(|s| s.ground_area),
            school: people::school(w, who).map(|h| h as u32),
            who: Some(who),
            ids: Vec::new(),
            text,
            matters: false,
            heard: false,
            level: 0,
        });
    }
}

// --- The Monday letter (GR56) ---------------------------------------------------------------

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// Monday's letter from your coordinator: the week's beat first, then what
/// last week bought, the road days and the money, the plan, your scenes and
/// your calls.
fn letter(w: &World, prog: usize, monday: Date, old_board: Option<&Vec<u32>>, board: &[u32]) -> Vec<String> {
    let b = beat::beat(w, monday);
    let mut lines = vec![b.line()];
    // Last week.
    let last = monday.add(-7);
    let log: Vec<&Done> = w.ground.week.log.iter().filter(|x| x.date >= last && x.date < monday).collect();
    let mut bought: Vec<String> = Vec::new();
    if let Some(old) = old_board {
        let new = board.iter().filter(|id| old.binary_search(id).is_err()).count();
        if new > 0 {
            bought.push(plural(new, "new name on the board", "new names on the board"));
        }
    }
    let ups: Vec<String> = log
        .iter()
        .filter(|x| x.level > 0 && matches!(x.kind, DoneKind::Visit | DoneKind::Call | DoneKind::CallBack))
        .filter_map(|x| {
            let who = x.who?;
            Some(format!("{} {}", people::name(w, who), Level::ALL[(x.level as usize).min(3)].name()))
        })
        .collect();
    if !ups.is_empty() {
        let n = ups.len();
        let shown: Vec<String> = ups.into_iter().take(3).collect();
        bought.push(if n > 3 {
            format!("{} and {} more a level up", shown.join(", "), n - 3)
        } else {
            shown.join(", ")
        });
    }
    let visits: Vec<&&Done> = log.iter().filter(|x| x.kind == DoneKind::Visit).collect();
    if !visits.is_empty() {
        let mut men: Vec<u32> = visits.iter().filter_map(|x| x.man).collect();
        men.sort_unstable();
        men.dedup();
        let mut seen: Vec<u32> = visits.iter().flat_map(|x| x.ids.iter().copied()).collect();
        seen.sort_unstable();
        seen.dedup();
        bought.push(format!(
            "{} by {}, {} seen",
            plural(visits.len(), "school visit", "school visits"),
            plural(men.len(), "man", "men"),
            plural(seen.len(), "recruit", "recruits")
        ));
    }
    let mine = log.iter().filter(|x| matches!(x.kind, DoneKind::Call | DoneKind::CallBack) && x.man.is_none()).count();
    if mine > 0 {
        bought.push(plural(mine, "call of yours", "calls of yours"));
    }
    if old_board.is_some() {
        lines.push(if bought.is_empty() {
            "Last week was quiet: no road, no new names.".into()
        } else {
            format!("Last week bought: {}.", bought.join("; "))
        });
    }
    // The road and the money.
    let open = open_days(w, monday, monday);
    match open.first().map(|o| o.1) {
        Some(win) => {
            match road::pool(w, prog, win) {
                Some((pool, used)) => {
                    let ahead = road::open_days_from(w, win, monday.add(open[0].0 as i32));
                    let weeks = ahead.div_ceil(if win == Window::Fall { 2 } else { 5 });
                    lines.push(format!(
                        "{} of {} road days left this {}, {} to go.",
                        pool.saturating_sub(used),
                        pool,
                        if win == Window::Fall { "fall" } else { "spring" },
                        plural(weeks as usize, "week", "weeks")
                    ));
                }
                None => {
                    let cap = dept::off_campus_recruiters(&w.programs[prog], monday).unwrap_or(0);
                    lines.push(format!("A contact period: no limit on road days, up to {cap} men out at once."));
                }
            }
            let p = road::purse(w, prog, &w.ground.week.roads);
            lines.push(format!(
                "This week's money covers {}; driving for everyone else.{}",
                road::flights_words(p.flights),
                if p.raised { " Your AD's raise is in it." } else { "" }
            ));
        }
        None => lines.push(format!("No road this week: {}.", b.period.to_lowercase())),
    }
    let rows: Vec<String> = w
        .ground
        .week
        .roads
        .iter()
        .map(|r| {
            format!(
                "{} to {}{} ({})",
                last_name(w, r.man),
                areas::area(r.area).name,
                if r.flight { " by air" } else { "" },
                days_words(r.days)
            )
        })
        .collect();
    if !rows.is_empty() {
        lines.push(format!("The plan: {}.", rows.join(", ")));
    }
    let yours: Vec<String> = w
        .ground
        .week
        .slots
        .iter()
        .filter(|s| s.yours && s.kind != SlotKind::Meeting)
        .map(|s| format!("{} ({})", s.title, s.reason.to_lowercase()))
        .collect();
    if !yours.is_empty() {
        lines.push(format!("For you this week: {}. I'll take the rest.", yours.join("; ")));
    }
    let calls: Vec<String> = w
        .ground
        .week
        .calls
        .iter()
        .map(|c| format!("{} ({})", people::name(w, c.who), super::phone::where_words(w, c.who)))
        .collect();
    if !calls.is_empty() {
        lines.push(format!("Calls for you: {}.", calls.join(", ")));
    }
    lines.push(format!("At stake: {}.", b.stake.to_lowercase()));
    lines
}

/// "Tue-Thu", "Thu, Fri", "Mon".
pub fn days_words(bits: u8) -> String {
    const D: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let ks: Vec<usize> = (0..7).filter(|k| bits & (1 << k) != 0).collect();
    if ks.len() >= 3 && ks.windows(2).all(|p| p[1] == p[0] + 1) {
        return format!("{}-{}", D[ks[0]], D[*ks.last().unwrap()]);
    }
    ks.iter().map(|&k| D[k]).collect::<Vec<_>>().join(", ")
}

/// The letter for a Monday, if it was written.
pub fn letter_for(w: &World, monday: Date) -> Option<&Vec<String>> {
    w.ground.week.letters.iter().find(|x| x.0 == monday).map(|x| &x.1)
}

// --- The May list (GR57) --------------------------------------------------------------------

/// The day after Memorial Day: each man's list from the spring road (the
/// rising seniors he saw in person), ranked by your coordinator (your
/// offers first, then the services' word, then how often your staff saw
/// him; lane G6 brings your goals into it), and your open official visit
/// spots from this weekend to the dead period.
pub(crate) fn may_list(w: &mut World, prog: usize, d: Date) {
    if w.ground.week.may.date == Some(d) {
        return;
    }
    let season = w.dates.season;
    let from = Date::ymd(season, 4, 1);
    let men = road::men(w, prog);
    let mut lists: Vec<(u32, Vec<u32>)> = Vec::new();
    let mut count: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();
    for &m in &men {
        let code = COACH + m as u32;
        let mut ids: Vec<u32> = w
            .scouting
            .dept
            .looks
            .iter()
            .filter(|(_, v)| {
                v.iter().any(|l| {
                    l.who == code && matches!(l.kind, Kind::Practice | Kind::Game) && l.date >= from && l.date < d
                })
            })
            .map(|(&id, _)| id)
            .filter(|&id| {
                place(w, id).is_some_and(|i| {
                    let q = &w.people[i];
                    q.stage == Stage::HighSchool
                        && q.class_of == season + 1
                        && q.cal.signed.is_none()
                        && q.cal.commit.is_none_or(|c| c.0 == prog)
                })
            })
            .collect();
        if ids.is_empty() {
            continue;
        }
        ids.sort_unstable();
        for &id in &ids {
            *count.entry(id).or_default() += 1;
        }
        lists.push((m as u32, ids));
    }
    let mut all: Vec<u32> = count.keys().copied().collect();
    let key = |id: u32| {
        let i = place(w, id).unwrap_or(0);
        let q = &w.people[i];
        (q.cal.offers.contains(&prog), q.stars, count.get(&id).copied().unwrap_or(0))
    };
    all.sort_by(|a, b| key(*b).cmp(&key(*a)).then(a.cmp(b)));
    // Your officials still open: the weekends from Friday to the dead
    // period, less the ones set.
    let div = match w.programs[prog].division {
        crate::world::data::Division::FBS => 0,
        crate::world::data::Division::FCS => 1,
        _ => 2,
    };
    let y = beat::dates_for(w, d);
    let mut weekends = 0;
    let mut f = d.on_or_after(Weekday::Fri);
    while weekends < 8 && crate::calendar::recruiting_period(&y, f) != crate::calendar::Period::Dead && f.month() <= 6 {
        weekends += 1;
        f = f.add(7);
    }
    let set = w
        .front_office
        .college
        .visits
        .iter()
        .filter(|v| v.kind == crate::world::front_office::college::VisitKind::Official && v.date >= d)
        .count() as u32;
    let spots = (weekends * PER_WEEKEND[div]).saturating_sub(set);
    all.retain(|&id| {
        place(w, id).is_some_and(|i| {
            !w.people[i].cal.visits.iter().any(|v| v.0 == prog && v.2 == crate::world::Visit::Official)
        })
    });
    let top: Vec<String> = all
        .iter()
        .take(spots.min(8) as usize)
        .filter_map(|&id| place(w, id))
        .map(|i| {
            let q = &w.people[i];
            format!(
                "{} ({}, {})",
                q.name,
                q.player.position.name(),
                if q.stars > 0 { format!("{}★", q.stars) } else { "unrated".into() }
            )
        })
        .collect();
    let men_n = lists.len();
    w.ground.week.may = MayList { date: Some(d), lists, ranked: all.clone(), spots, invited: Vec::new() };
    let coord =
        coordinator(w, prog).map_or_else(|| "Your recruiting coordinator".into(), |c| super::man_words(w, c as u32));
    let mut body = vec![format!(
        "The staff met at the map with their lists from the spring road: {} from {}.",
        plural(all.len(), "rising senior", "rising seniors"),
        plural(men_n, "man", "men")
    )];
    body.push(format!("You have {} open official visit spots from this weekend to the dead period.", spots));
    if !top.is_empty() {
        body.push(format!("The top of my list for them: {}.", top.join(", ")));
    }
    memo(w, d, 8 * 60, &coord, "The May list".into(), body);
}

/// You invite the top of the May list to your open official visit spots:
/// each gets the first weekend he can take, and says yes or no as the
/// built visit rules have him (`front_office::college::set_visit`).
pub fn may_invite(w: &mut World, n: usize) -> Result<Vec<String>, String> {
    seat_program(w).ok_or("Ground's week is for a college seat.")?;
    if w.ground.week.may.date.is_none() {
        return Err("The May list comes the day after Memorial Day.".into());
    }
    let spots = w.ground.week.may.spots as usize;
    let take = n.min(spots.saturating_sub(w.ground.week.may.invited.len()));
    let ids: Vec<u32> = w
        .ground
        .week
        .may
        .ranked
        .iter()
        .copied()
        .filter(|id| !w.ground.week.may.invited.contains(id))
        .take(take)
        .collect();
    let mut out = Vec::new();
    for id in ids {
        use crate::world::front_office::college::{VisitKind, set_visit, visit_dates};
        let Some(d) = visit_dates(w, id, VisitKind::Official).first().map(|x| x.0) else {
            out.push(format!(
                "{}: no weekend open to him.",
                place(w, id).map_or(String::new(), |i| w.people[i].name.clone())
            ));
            continue;
        };
        match set_visit(w, id, VisitKind::Official, d) {
            Ok(s) => {
                w.ground.week.may.invited.push(id);
                out.push(s);
            }
            Err(e) => out.push(e),
        }
    }
    Ok(out)
}

/// A memo to your inbox from your staff.
fn memo(w: &mut World, d: Date, minute: u16, from: &str, subject: String, body: Vec<String>) {
    use crate::world::desk::{Head, Mail};
    let you = crate::world::folk::scene::your_name(w);
    let text: Vec<u64> = subject.bytes().map(u64::from).collect();
    let id = crate::world::ground::key(&[0x3EE7, d.0 as u64, crate::world::ground::key(&text)]);
    let Some(c) = w.career.as_mut() else { return };
    if c.desk.has(id) {
        return;
    }
    c.desk.mails.push(Mail {
        id,
        date: d,
        minute,
        head: Head::Memo,
        org: String::new(),
        org_line: "Recruiting".into(),
        from: from.to_string(),
        role: "Recruiting coordinator".into(),
        to: format!("{you}, Head Coach"),
        subject,
        body,
        table: Vec::new(),
        attach: Vec::new(),
        sign: from.to_string(),
        keys: (1 << 2) | (1 << 9),
        about: None,
        answers: Vec::new(),
        answered: None,
        answer_by: None,
        read: false,
        saved: false,
        loud: 1,
        ids: Vec::new(),
        org_team: None,
        why: Vec::new(),
        count: 0,
        leads: 0,
    });
    c.desk.mails.sort_by_key(|m| (m.date, m.minute));
}
