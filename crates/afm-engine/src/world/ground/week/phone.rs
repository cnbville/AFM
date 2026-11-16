//! The day on Ground (GR12-GR15): the morning map's overnight list, the
//! in-flight list of everything you're waiting on, your calls and the
//! coaches calling back at their hour, the night calls from the road, and
//! the calls that stop the day at their minute (live moments).

use super::road::{Visited, names};
use super::{CallBack, Done, DoneKind, monday_of, seat_program};
use crate::calendar::{Date, Moment, Weekday};
use crate::world::World;
use crate::world::ground::people::{self, Who};
use crate::world::ground::standing::{self, Contact, Level};
use crate::world::scouting::Kind;

/// A call's time on the ledger (GR50: 10 to 20 minutes).
pub const CALL_HOURS: f32 = 0.25;
/// How long a coach who called back waits on the line for you, minutes:
/// past it he hangs up (proposed, not confirmed).
pub const WAITS: i32 = 60;

/// A local minute's clock ("1:20 PM").
pub fn clock(minute: u16) -> String {
    crate::calendar::zone::clock(minute)
}

fn eastern(w: &World, d: Date, minute: u16) -> Moment {
    w.seat_zone().to_eastern(Moment::at(d, minute / 60, minute % 60))
}

/// Your coach index (the head coach at a college seat).
fn you(w: &World) -> Option<usize> {
    w.career.as_ref().map(|c| c.coach)
}

/// A person's area in words ("Carver, Columbus").
pub fn where_words(w: &World, who: Who) -> String {
    match people::school(w, who) {
        Some(h) => {
            let a = w.high_schools.get(h).map(|s| crate::world::ground::areas::area(s.ground_area).name);
            match a {
                Some(a) => format!("{}, {a}", w.school_name(h)),
                None => w.school_name(h).to_string(),
            }
        }
        None => people::role(w, who),
    }
}

// --- The morning map (GR15) ---------------------------------------------------------------

/// Each person's level with your staff, for everyone your seat keeps a
/// memory with.
fn levels(w: &World, prog: usize) -> Vec<(Who, u8)> {
    let mut v: Vec<(Who, u8)> = Vec::new();
    for m in &w.ground.memories {
        if v.last().is_some_and(|x| x.0 == m.who) {
            continue;
        }
        v.push((m.who, standing::level(w, prog, m.who) as u8));
    }
    v
}

/// The 5 AM stop: what changed overnight. Levels reached and lost since
/// yesterday's morning, yesterday's road by man, calls missed.
pub(crate) fn morning(w: &mut World, prog: usize, d: Date) {
    if w.ground.week.overnight_day == Some(d) {
        return;
    }
    let now = levels(w, prog);
    let old = std::mem::take(&mut w.ground.week.levels);
    let mut lines: Vec<String> = Vec::new();
    if !old.is_empty() {
        for &(who, lv) in &now {
            let before = old.binary_search_by(|x| x.0.cmp(&who)).map_or(0, |i| old[i].1);
            if lv != before {
                let name = people::name(w, who);
                let at = where_words(w, who);
                let l = Level::ALL[lv.min(3) as usize];
                lines.push(if lv > before {
                    format!("{name} ({at}): {}", l.name())
                } else {
                    format!("{name} ({at}) slipped to {}", l.name())
                });
            }
        }
    }
    let y = d.add(-1);
    let mut by_man: Vec<(u32, u16, usize, usize)> = Vec::new();
    for x in w.ground.week.log.iter().filter(|x| x.date == y && x.kind == DoneKind::Visit) {
        let Some(m) = x.man else { continue };
        match by_man.iter_mut().find(|b| b.0 == m) {
            Some(b) => {
                b.2 += 1;
                b.3 += x.ids.len();
            }
            None => by_man.push((m, x.area.unwrap_or(0), 1, x.ids.len())),
        }
    }
    for (m, a, n, k) in by_man {
        lines.push(format!(
            "{} in {}: {} {}, {} {} seen",
            super::last_name(w, m),
            crate::world::ground::areas::area(a).name,
            n,
            if n == 1 { "school" } else { "schools" },
            k,
            if k == 1 { "recruit" } else { "recruits" }
        ));
    }
    for x in w.ground.week.log.iter().filter(|x| x.date == y && x.kind == DoneKind::Missed) {
        lines.push(x.text.clone());
    }
    let wk = &mut w.ground.week;
    wk.levels = now;
    wk.overnight = lines;
    wk.overnight_day = Some(d);
}

// --- Your calls (GR53) and calls back (GR12, GR14) ---------------------------------------

/// Whether a person picks up at a local minute: in his hour (a coach's
/// planning period, a counselor's school morning, a trainer's evening) or,
/// for a coach, in the evening after practice.
pub fn picks_up(w: &World, who: Who, minute: u16) -> bool {
    let h = people::makeup(w, who).hour;
    let in_hour = minute >= h && minute < h + 50;
    let evening = matches!(who, Who::Coach(_)) && (19 * 60..21 * 60 + 30).contains(&minute);
    in_hour || evening
}

/// His next hour from a local minute on a day: today if it's still ahead,
/// else the next school day (a summer day too: he keeps his hours).
fn next_hour(w: &World, who: Who, d: Date, minute: u16) -> (Date, u16) {
    let h = people::makeup(w, who).hour;
    let mut day = if minute < h { d } else { d.add(1) };
    while matches!(day.weekday(), Weekday::Sat | Weekday::Sun) {
        day = day.add(1);
    }
    (day, h)
}

/// Whether a call with him matters enough to stop your day: his school
/// has a kid on your board, or he's Trusted or better with you.
fn matters(w: &World, prog: usize, who: Who) -> bool {
    if standing::level(w, prog, who) >= Level::Trusted {
        return true;
    }
    let Some(h) = people::school(w, who) else { return false };
    let season = w.dates.season;
    w.people.iter().any(|q| {
        q.hs == h
            && matches!(q.stage, crate::world::Stage::HighSchool)
            && q.class_of > season
            && q.class_of <= season + 2
            && (q.cal.offers.contains(&prog) || q.stars >= 3)
    })
}

/// You call a high school coach (GR53): it counts by your name. In his hour
/// (or in the evening) he picks up; otherwise you leave a message and he
/// calls back at his next hour. 15 minutes on your ledger.
pub fn call(w: &mut World, who: Who) -> Result<String, String> {
    let prog = seat_program(w).ok_or("Ground's week is for a college seat.")?;
    if super::off_now(w) {
        return Err("You're off this week: your coordinator holds the phones.".into());
    }
    if people::school(w, who).is_none() && !matches!(who, Who::Trainer(_) | Who::SevenOnSeven(_)) {
        return Err("He isn't at a school now.".into());
    }
    let me = you(w).ok_or("Start a career first.")?;
    let d = w.today;
    let minute = w.local_now().minute.max(5 * 60);
    let name = people::name(w, who);
    let at = where_words(w, who);
    w.ground.week.book(d, &format!("Call: {name}"), CALL_HOURS, true);
    if let Some(p) = w.ground.week.calls.iter_mut().find(|p| p.who == who && p.state == 0) {
        p.state = 1;
    }
    if picks_up(w, who, minute) {
        let before = standing::level(w, prog, who);
        if !w.ground.off {
            standing::contact(w, prog, Some(me), who, Contact::Call, 1.0, d);
        }
        let after = standing::level(w, prog, who);
        let text = if after > before {
            format!("You called {name} ({at}): {}.", after.name())
        } else {
            format!("You called {name} ({at}).")
        };
        log(w, d, minute, DoneKind::Call, Some(who), text.clone(), false, if after > before { after as u8 } else { 0 });
        return Ok(text);
    }
    let (day, hour) = next_hour(w, who, d, minute);
    let m = matters(w, prog, who);
    if !m {
        // He calls back between things: you talk at his hour.
        if !w.ground.off {
            standing::contact(w, prog, Some(me), who, Contact::Call, 1.0, d);
        }
        let text = format!("{name} ({at}) called back at {}.", clock(hour));
        log(w, day, hour, DoneKind::CallBack, Some(who), text, false, 0);
        return Ok(format!("{name} is teaching: he'll call back at his {} planning period.", clock(hour)));
    }
    let id = w.ground.week.next();
    w.ground.week.phone.push(CallBack { id, who, date: day, minute: hour, matters: true, state: 0, tries: 0 });
    Ok(format!(
        "{name} is teaching. You left a message: he calls back {} at {}.",
        if day == d { "today".to_string() } else { super::beat::short(day) },
        clock(hour)
    ))
}

#[allow(clippy::too_many_arguments)]
fn log(w: &mut World, d: Date, minute: u16, kind: DoneKind, who: Option<Who>, text: String, matters: bool, level: u8) {
    let id = w.ground.week.next();
    let area = who.and_then(|x| people::school(w, x)).and_then(|h| w.high_schools.get(h)).map(|s| s.ground_area);
    let school = who.and_then(|x| people::school(w, x)).map(|h| h as u32);
    w.ground.week.log.push(Done {
        id,
        date: d,
        minute,
        kind,
        man: None,
        area,
        school,
        who,
        ids: Vec::new(),
        text,
        matters,
        heard: false,
        level,
    });
}

/// You take a coach's call back (GR14): now, while he's on the line.
pub fn take(w: &mut World, id: u32) -> Result<String, String> {
    let prog = seat_program(w).ok_or("Ground's week is for a college seat.")?;
    let me = you(w).ok_or("Start a career first.")?;
    let k = w.ground.week.phone.iter().position(|c| c.id == id && c.state == 0).ok_or("No such call waiting.")?;
    let c = w.ground.week.phone[k].clone();
    let at = eastern(w, c.date, c.minute);
    let now = w.now();
    if now < at.add_minutes(-1) {
        return Err(format!("He calls at {}.", clock(c.minute)));
    }
    if now > at.add_minutes(WAITS) {
        return Err("He's off the line: the moment passed.".into());
    }
    let name = people::name(w, c.who);
    let place = where_words(w, c.who);
    let before = standing::level(w, prog, c.who);
    if !w.ground.off {
        standing::contact(w, prog, Some(me), c.who, Contact::Call, 1.0, w.today);
    }
    let after = standing::level(w, prog, c.who);
    w.ground.week.phone[k].state = 1;
    let d = w.today;
    w.ground.week.book(d, &format!("Call back: {name}"), CALL_HOURS, true);
    let text = if after > before {
        format!("{name} ({place}) called back and you took it: {}.", after.name())
    } else {
        format!("{name} ({place}) called back and you took it.")
    };
    log(
        w,
        d,
        c.minute,
        DoneKind::CallBack,
        Some(c.who),
        text.clone(),
        true,
        if after > before { after as u8 } else { 0 },
    );
    Ok(text)
}

/// The day's calls back that went by: your coordinator takes them on your
/// week off (his call, GR58); otherwise a coach who barely knows you
/// doesn't call again (GR14), and a Trusted one tries once more.
pub(crate) fn lapse(w: &mut World, prog: usize, d: Date) {
    let now = w.now();
    let off = super::off_now(w);
    let list: Vec<CallBack> = w.ground.week.phone.iter().filter(|c| c.state == 0).cloned().collect();
    for c in list {
        let at = eastern(w, c.date, c.minute);
        let gone = c.date < d || now > at.add_minutes(WAITS);
        let due = c.date <= d;
        let name = people::name(w, c.who);
        let place = where_words(w, c.who);
        if off && due {
            if let Some(coord) = super::coordinator(w, prog)
                && !w.ground.off
            {
                standing::contact(w, prog, Some(coord), c.who, Contact::Call, 1.0, d);
            }
            set_state(w, c.id, 2);
            let who = super::coordinator(w, prog)
                .map_or_else(|| "Your coordinator".into(), |x| super::last_name(w, x as u32));
            log(
                w,
                d,
                c.minute,
                DoneKind::CallBack,
                Some(c.who),
                format!("{who} took {name}'s call ({place})."),
                false,
                0,
            );
            continue;
        }
        if !gone {
            continue;
        }
        let lv = standing::level(w, prog, c.who);
        if lv >= Level::Trusted && c.tries == 0 {
            let (day, hour) = next_hour(w, c.who, d.add(-1).max(c.date), c.minute.saturating_add(1));
            if let Some(x) = w.ground.week.phone.iter_mut().find(|x| x.id == c.id) {
                x.date = day.max(d);
                x.minute = hour;
                x.tries = 1;
            }
            continue;
        }
        set_state(w, c.id, 3);
        log(
            w,
            c.date,
            c.minute,
            DoneKind::Missed,
            Some(c.who),
            format!("{name} ({place}) called back and you missed him; he didn't call again."),
            false,
            0,
        );
    }
}

fn set_state(w: &mut World, id: u32, s: u8) {
    if let Some(x) = w.ground.week.phone.iter_mut().find(|x| x.id == id) {
        x.state = s;
    }
}

// --- The night calls (GR13) ---------------------------------------------------------------

const OPENERS: [&str; 6] = [
    "Long day in {area}.",
    "{area} today.",
    "Just got back to the hotel.",
    "Checking in from {area}.",
    "Good day on the road.",
    "Quick one from {area}.",
];

/// A man's night call from the road, 7 to 10 PM, in his own voice: his
/// schools, who he saw, where he stands with the coaches. It stops your
/// evening (GR14) when a coach moved up a level with him, or he saw a
/// 4-star or better nobody of yours had seen in person.
pub(crate) fn night_call(w: &mut World, man: usize, area: u16, lines: &[Visited], k: u16, d: Date) {
    let a = crate::world::ground::areas::area(area);
    let open = OPENERS[(crate::world::ground::key(&[0x41A7, man as u64]) % OPENERS.len() as u64) as usize]
        .replace("{area}", a.name);
    let mut parts: Vec<String> = vec![open];
    let mut matters = false;
    let mut all: Vec<u32> = Vec::new();
    let mut level = 0u8;
    let since = super::road::recruiting_year(d);
    for v in lines {
        let school = w.school_name(v.school).to_string();
        if v.seen.is_empty() {
            parts.push(format!("Sat with the coach at {school}."));
        } else {
            parts.push(format!("Saw {} at {school}.", names(w, &v.seen)));
        }
        if let Some(c) = v.coach
            && v.after > v.before
        {
            parts.push(format!("{} {}.", people::name(w, c), up_words(v.after)));
            if v.after >= Level::Known {
                matters = true;
                level = level.max(v.after as u8);
            }
        }
        for &id in &v.seen {
            let Some(i) = crate::world::scouting::place(w, id) else { continue };
            let first = !w.scouting.dept.looks.get(&id).is_some_and(|l| {
                l.iter().any(|l| {
                    matches!(l.kind, Kind::Practice | Kind::Game | Kind::Camp) && l.date >= since && l.date < d
                })
            });
            if first && w.people[i].stars >= 4 {
                matters = true;
            }
        }
        all.extend(v.seen.iter().copied());
    }
    let minute = (19 * 60 + 25 * k).min(21 * 60 + 50);
    let text = format!("{}: \"{}\"", super::man_words(w, man as u32), parts.join(" "));
    let id = w.ground.week.next();
    w.ground.week.log.push(Done {
        id,
        date: d,
        minute,
        kind: DoneKind::NightCall,
        man: Some(man as u32),
        area: Some(area),
        school: None,
        who: None,
        ids: all,
        text,
        matters,
        heard: false,
        level,
    });
}

fn up_words(lv: Level) -> &'static str {
    match lv {
        Level::Stranger => "knows our name now",
        Level::Known => "knows us now: he'll give us his list",
        Level::Trusted => "trusts us now",
        Level::Home => "says we hear about his guys first from now on",
    }
}

/// You hear a night call (or read it after).
pub fn hear(w: &mut World, id: u32) -> Result<String, String> {
    seat_program(w).ok_or("Ground's week is for a college seat.")?;
    let x = w.ground.week.log.iter_mut().find(|x| x.id == id).ok_or("No such call.")?;
    x.heard = true;
    Ok(x.text.clone())
}

/// The live moments Ground puts in your day (GR14): coaches calling back
/// who matter, and night calls that do, at their minute (Eastern).
pub fn moments(w: &World, d: Date) -> Vec<(Moment, String)> {
    if seat_program(w).is_none() {
        return Vec::new();
    }
    let mut v = Vec::new();
    let off = super::off_now(w);
    for c in w.ground.week.phone.iter().filter(|c| c.state == 0 && c.matters && c.date == d && !off) {
        v.push((eastern(w, c.date, c.minute), format!("Call: {} is calling back", people::name(w, c.who))));
    }
    for x in w.ground.week.log.iter().filter(|x| x.kind == DoneKind::NightCall && x.date == d && x.matters && !x.heard)
    {
        let man = x.man.map_or_else(|| "Your man".into(), |m| super::last_name(w, m));
        v.push((eastern(w, x.date, x.minute), format!("Night call: {man} from the road")));
    }
    v
}

// --- The in-flight list (GR12) ------------------------------------------------------------

/// Something you're waiting on, with when it lands.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Flight {
    pub what: String,
    pub lands: String,
    pub date: Date,
    /// Local minutes, when it has an hour.
    pub minute: Option<u16>,
}

/// Everything you're waiting on, soonest first: your men on the road
/// (their night calls), coaches calling back, your staff's film, your asks
/// for a live look, what you host this week, official visits set, the May
/// list.
pub fn in_flight(w: &World) -> Vec<Flight> {
    let Some(prog) = seat_program(w) else { return Vec::new() };
    let d = w.today;
    let mut v: Vec<Flight> = Vec::new();
    let week = &w.ground.week;
    if let Some(m) = week.monday {
        let k = m.days_to(d);
        if (0..7).contains(&k) {
            for r in week.roads.iter().filter(|r| r.days & (1 << k) != 0) {
                v.push(Flight {
                    what: format!(
                        "{} in {}",
                        super::last_name(w, r.man),
                        crate::world::ground::areas::area(r.area).name
                    ),
                    lands: "his night call, 7 to 10 PM".into(),
                    date: d,
                    minute: Some(19 * 60),
                });
            }
        }
    }
    for c in week.phone.iter().filter(|c| c.state == 0) {
        v.push(Flight {
            what: format!("{} ({}) calling back", people::name(w, c.who), where_words(w, c.who)),
            lands: format!(
                "{}by his {} planning period",
                if c.date == d { String::new() } else { format!("{}, ", super::beat::short(c.date)) },
                clock(c.minute)
            ),
            date: c.date,
            minute: Some(c.minute),
        });
    }
    for x in week.log.iter().filter(|x| x.kind == DoneKind::CallBack && !w.has_come(eastern(w, x.date, x.minute))) {
        let who = x.who.map_or_else(String::new, |p| people::name(w, p));
        v.push(Flight {
            what: format!("{who} calling back"),
            lands: clock(x.minute),
            date: x.date,
            minute: Some(x.minute),
        });
    }
    for q in w.scouting.film.queue.iter().filter(|q| q.state == crate::world::scouting::film::queue::WAITING) {
        let name = crate::world::scouting::place(w, q.id).map_or_else(String::new, |i| w.people[i].name.clone());
        v.push(Flight {
            what: format!("{name}'s tape from {}", q.from),
            lands: format!("by {}", super::beat::short(q.by)),
            date: q.by,
            minute: None,
        });
    }
    for a in w.scouting.dept.asks.iter().filter(|a| a.1 == Kind::Practice) {
        let Some(i) = crate::world::scouting::place(w, a.0) else { continue };
        let area = w.high_schools.get(w.people[i].hs).map(|s| crate::world::ground::areas::area(s.ground_area).name);
        v.push(Flight {
            what: format!("A live look at {}", w.people[i].name),
            lands: format!("the next road day in {}", area.unwrap_or("his area")),
            date: d,
            minute: None,
        });
    }
    let sun = monday_of(d).add(6);
    let mut hosted: Vec<(Date, usize, &'static str)> = Vec::new();
    for q in
        w.people.iter().filter(|q| matches!(q.stage, crate::world::Stage::HighSchool | crate::world::Stage::PrepSchool))
    {
        for vi in q.cal.visits.iter().filter(|vi| vi.0 == prog && vi.1 > d && vi.1 <= sun) {
            let k = match vi.2 {
                crate::world::Visit::Official => "official visit",
                crate::world::Visit::Home => "home visit",
                _ => "unofficial visit",
            };
            match hosted.iter_mut().find(|h| h.0 == vi.1 && h.2 == k) {
                Some(h) => h.1 += 1,
                None => hosted.push((vi.1, 1, k)),
            }
        }
    }
    for (day, n, k) in hosted {
        v.push(Flight {
            what: format!("{n} {}{} on your campus", k, if n == 1 { "" } else { "s" }),
            lands: super::beat::short(day),
            date: day,
            minute: None,
        });
    }
    let may = super::beat::may_list_day(d.year());
    if may > d && may.days_to(d) > -8 {
        v.push(Flight { what: "The May list".into(), lands: super::beat::short(may), date: may, minute: Some(8 * 60) });
    }
    v.sort_by_key(|f| (f.date, f.minute.unwrap_or(0)));
    v
}
