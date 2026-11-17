//! Capital's week (lane C4: CP2, CP13; `docs/design/capital.md`, part 3).
//!
//! Meeting a donor takes a slot in your week, the same week Ground's
//! meeting at the map lays out (`ground::week::plan`, GR51): a lunch or a
//! dinner, ranked with the visits, the calls and your road day for the five
//! scenes that come to you. A slot you don't hold is the development
//! office's: its man has the lunch, and the gift that needed the head coach
//! in the room doesn't come of it.
//!
//! - **The office's list** (January to August): one donor a week, two while
//!   the drive is asking and a lead donor waits for you; the lead donors
//!   first, then the richest the office reads that you haven't seen in four
//!   months (`suggest`).
//! - **Your bookings** (`book`): any donor of yours, a weekday lunch or
//!   dinner up to eight weeks out. In this week it takes a slot from the
//!   scenes already yours (the one that matters least goes to your
//!   coordinator).
//! - **The year's rooms of donors:** the booster club's dinner on the spring
//!   caravan (the second Tuesday of May), the golf outing (the last Friday
//!   of June), and the suite before each home game.
//! - **In season** (September to December, CP13): your only donor slot is
//!   the hour in a suite before a home game.
//!
//! Only your seat has a week, and only its head coach courts donors, so the
//! world plays the same on every seed. Sizes are Claude's (proposed, not
//! confirmed).

use super::scenes;
use crate::calendar::{Date, Moment, Weekday};
use crate::world::World;
use crate::world::ground::week::{self as gweek, Slot, SlotKind};

/// A donor meeting you booked.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Booked {
    pub donor: u32,
    pub date: Date,
    /// Local minutes after midnight.
    pub minute: u16,
}

/// A lunch and a dinner (local minutes).
pub const LUNCH: u16 = 12 * 60;
pub const DINNER: u16 = 18 * 60 + 30;
/// How far ahead you can book.
pub const BOOK_AHEAD: i32 = 56;
/// The office's donors a week, and two while a lead donor waits.
pub const OFFICE_WEEK: usize = 1;
pub const OFFICE_LEADS: usize = 2;
/// The office puts a donor up again after this many days unseen.
pub const UNSEEN: i32 = 120;
/// The ranks for the meeting at the map (`ground::week::plan`, GR51: the
/// meeting 9, officials 8, junior day 7, a road day 6, unofficials 4): your
/// booking 8, the suite 7, a lead donor 6, the caravan and the golf 5,
/// another donor 4.
pub const RANK_BOOKED: u8 = 8;
pub const RANK_SUITE: u8 = 7;
pub const RANK_LEAD: u8 = 6;
pub const RANK_EVENT: u8 = 5;
pub const RANK_DONOR: u8 = 4;
/// Donors in a room: the suite, the caravan's dinner, the golf outing.
pub const SUITE: usize = 6;
pub const CARAVAN: usize = 10;
pub const GOLF: usize = 12;

/// Your college program when you are its head coach (only the head coach
/// closes the big gifts).
pub fn your_program(w: &World) -> Option<usize> {
    let p = gweek::seat_program(w)?;
    let you = crate::world::scouting::readers::you(w)?;
    (w.programs[p].head_coach == Some(you)).then_some(p)
}

/// In season (September to December): only the suite (CP13).
pub fn in_season(d: Date) -> bool {
    d.month() >= 9
}

/// The second Tuesday of May (the spring caravan's booster dinner) and the
/// last Friday of June (the golf outing).
pub fn caravan_day(year: i32) -> Date {
    Date::nth_weekday(year, 5, Weekday::Tue, 2)
}
pub fn golf_day(year: i32) -> Date {
    Date::ymd(year, 6, 30).on_or_before(Weekday::Fri)
}

/// What kind of room of donors a slot is, from its title.
pub fn room_kind(s: &Slot) -> scenes::RoomKind {
    if s.title.starts_with("The suite") {
        scenes::RoomKind::Suite
    } else if s.title.starts_with("The caravan") {
        scenes::RoomKind::Caravan
    } else {
        scenes::RoomKind::Golf
    }
}

fn slot(date: Date, minute: u16, kind: SlotKind, title: String, reason: &str, donor: Option<u32>) -> Slot {
    Slot {
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
        donor,
    }
}

/// "Lunch" or "Dinner".
pub fn meal(minute: u16) -> &'static str {
    if minute < 15 * 60 { "Lunch" } else { "Dinner" }
}

/// Your donor slots for the meeting at the map in the week of `monday`,
/// ranked (`ground::week::plan::draft_slots` reads them).
pub(crate) fn slots(w: &World, prog: usize, monday: Date) -> Vec<(u8, Slot)> {
    let mut v: Vec<(u8, Slot)> = Vec::new();
    if your_program(w) != Some(prog) || w.capital.off {
        return v;
    }
    let end = monday.add(7);
    if in_season(monday) || in_season(end.add(-1)) {
        // The suite before each home game this week.
        for (d, opp) in crate::world::front_office::college::home_games(w, prog) {
            if d < monday || d >= end || !in_season(d) {
                continue;
            }
            let minute = suite_minute(w, prog, d);
            let name = w.programs[opp].name(w.options.names).to_string();
            v.push((
                RANK_SUITE,
                slot(
                    d,
                    minute,
                    SlotKind::DonorRoom,
                    format!("The suite before kickoff: {name}"),
                    "Your only donor hour in season",
                    None,
                ),
            ));
        }
        if in_season(monday) {
            return v;
        }
    }
    let off = |d: Date| w.ground.week.off.is_some_and(|(a, b)| d >= a && d <= b);
    let c = caravan_day(monday.year());
    if c >= monday && c < end && !off(c) {
        v.push((
            RANK_EVENT,
            slot(
                c,
                DINNER,
                SlotKind::DonorRoom,
                "The caravan's booster dinner".into(),
                "The spring caravan's stop",
                None,
            ),
        ));
    }
    let g = golf_day(monday.year());
    if g >= monday && g < end && !off(g) {
        v.push((RANK_EVENT, slot(g, 8 * 60, SlotKind::DonorRoom, "The golf outing".into(), "Summer golf", None)));
    }
    let mut taken: Vec<Date> = Vec::new();
    for b in w.capital.booked.iter().filter(|b| b.date >= monday && b.date < end && !in_season(b.date)) {
        let Some(d) = w.capital.donor(b.donor).filter(|d| d.alive()) else { continue };
        taken.push(b.date);
        v.push((
            RANK_BOOKED,
            slot(
                b.date,
                b.minute,
                SlotKind::Donor,
                format!("{} with {}", meal(b.minute), d.name),
                "You asked for this one",
                Some(b.donor),
            ),
        ));
    }
    // The office's list on the days left: Thursday's dinner, then Tuesday's
    // lunch, then Wednesday's dinner.
    let from = monday.max(w.today);
    let days: Vec<(Date, u16)> = [(3, DINNER), (1, LUNCH), (2, DINNER)]
        .into_iter()
        .map(|(k, m)| (monday.add(k), m))
        .filter(|(d, _)| *d >= from && !taken.contains(d) && !off(*d) && !in_season(*d))
        .collect();
    let booked: Vec<u32> =
        w.capital.booked.iter().filter(|b| b.date >= monday && b.date < end).map(|b| b.donor).collect();
    let list = suggest(w, prog, monday);
    let leads = list.iter().filter(|x| x.1).count();
    let n = if leads > 0 { OFFICE_LEADS } else { OFFICE_WEEK };
    for ((id, lead, why), (d, m)) in list.into_iter().filter(|x| !booked.contains(&x.0)).take(n).zip(days) {
        let Some(x) = w.capital.donor(id) else { continue };
        v.push((
            if lead { RANK_LEAD } else { RANK_DONOR },
            slot(d, m, SlotKind::Donor, format!("{} with {}", meal(m), x.name), &why, Some(id)),
        ));
    }
    v
}

/// Two hours before your kickoff, on your clock (8 AM at the earliest).
fn suite_minute(w: &World, prog: usize, d: Date) -> u16 {
    let k = w.college.as_ref().and_then(|c| c.games.iter().find(|g| g.date == d && g.home == prog)).map(|g| g.kickoff);
    let Some((h, m)) = k else { return 10 * 60 };
    let local = w.seat_zone().local(Moment::at(d, h, m));
    let minute = if local.date == d { local.minute as i32 } else { 12 * 60 };
    (minute - 120).max(8 * 60) as u16
}

/// The development office's list for the week: (donor, a lead donor of the
/// drive waiting for you, the office's reason), the most worth your time
/// first. A lead donor of the drive while it asks, not asked in this
/// stage, comes first; then the richest the office reads that you haven't
/// seen in four months, the warmest first among equals. Nobody seen in the
/// last month.
pub fn suggest(w: &World, prog: usize, monday: Date) -> Vec<(u32, bool, String)> {
    let Some(you) = crate::world::scouting::readers::you(w).map(|c| c as u32) else { return Vec::new() };
    let open = super::drive::open_drive(&w.capital, prog).map(|k| &w.capital.drives[k]);
    let asking = open.filter(|x| matches!(x.stage, super::Stage::Quiet | super::Stage::Public));
    let mut v: Vec<(f64, u32, bool, String)> = Vec::new();
    for k in super::donors::of(w, prog) {
        let d = &w.capital.donors[k];
        let b = d.bond(you);
        let last = b.and_then(|b| b.last);
        let since = last.map_or(i32::MAX, |l| l.days_to(monday));
        if since < 30 {
            continue;
        }
        let lead = asking.is_some_and(|x| {
            x.lead.contains(&d.id)
                && !x.asked.iter().any(|a| a.0 == d.id && (a.1 == x.stage as u8 || a.1 == super::drive::GAVE))
        });
        let read = super::screen::read(w, prog, d, b.map_or(0, |b| b.met));
        if !lead && since < UNSEEN {
            continue;
        }
        let score =
            if lead { 100.0 } else { 0.0 } + 10.0 * (read.hi as f64 + read.lo as f64) / 2.0 + d.warmth_to(you) as f64;
        let why = if lead {
            format!("A lead donor for the drive: {}", read.words.trim_end_matches('.').to_lowercase())
        } else if last.is_none() {
            format!("You haven't met him. The office: {}", read.words.trim_end_matches('.').to_lowercase())
        } else {
            format!(
                "Unseen since {}. The office: {}",
                last.unwrap().pretty(),
                read.words.trim_end_matches('.').to_lowercase()
            )
        };
        v.push((score, d.id, lead, why));
    }
    v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    v.into_iter().map(|x| (x.1, x.2, x.3)).collect()
}

/// A scene's rank at the map by its kind, for the slot a booking takes
/// this week (the plan's own ranks, `ground::week::plan`).
fn rank_of(s: &Slot) -> u8 {
    match s.kind {
        SlotKind::Meeting => 9,
        SlotKind::Official => 8,
        SlotKind::JuniorDay | SlotKind::Camp | SlotKind::Clinic | SlotKind::SpringGame => 7,
        SlotKind::SchoolVisit => 6,
        SlotKind::CoachCall if s.reason.starts_with("Slipping") => 6,
        SlotKind::Convention => 5,
        SlotKind::Unofficial => 4,
        SlotKind::CoachCall => 3,
        SlotKind::OpenPractice => 2,
        SlotKind::Donor if s.reason == "You asked for this one" => RANK_BOOKED,
        SlotKind::Donor if s.reason.starts_with("A lead donor") => RANK_LEAD,
        SlotKind::Donor => RANK_DONOR,
        SlotKind::DonorRoom if s.title.starts_with("The suite") => RANK_SUITE,
        SlotKind::DonorRoom => RANK_EVENT,
        SlotKind::NightCall | SlotKind::Film => 1,
    }
}

/// You book a lunch or a dinner with a donor (CP2): a weekday from
/// tomorrow to eight weeks out, January to August (CP13), one donor a day.
/// In this week it takes one of the scenes that are yours; the one that
/// matters least goes to your coordinator.
pub fn book(w: &mut World, donor: u32, date: Date, dinner: bool) -> Result<String, String> {
    let prog = your_program(w).ok_or("Donors are the head coach's to court.")?;
    let d = w.capital.donor(donor).filter(|d| d.alive()).cloned().ok_or("No such donor.")?;
    if d.home != prog as u32 && d.also != Some(prog as u32) {
        return Err("He gives to another school.".into());
    }
    if date <= w.today {
        return Err("That day has gone.".into());
    }
    if w.today.days_to(date) > BOOK_AHEAD {
        return Err("The office books up to eight weeks out.".into());
    }
    if in_season(date) {
        return Err("From September to December your only donor time is the suite before a home game.".into());
    }
    if matches!(date.weekday(), Weekday::Sat | Weekday::Sun) {
        return Err("Donors get a weekday lunch or dinner; weekends are for the visits.".into());
    }
    if w.ground.week.off.is_some_and(|(a, b)| date >= a && date <= b) {
        return Err("That's your week off.".into());
    }
    if w.capital.booked.iter().any(|b| b.date == date) {
        return Err("You have a donor that day already.".into());
    }
    if w.capital.booked.iter().any(|b| b.donor == donor && b.date.days_to(date).abs() < 30) {
        return Err("He's on your calendar within the month already.".into());
    }
    let monday = gweek::monday_of(date);
    let hc = w.programs[prog].head_coach.map(|c| c as u32);
    let k = monday.days_to(date);
    if w.ground.week.monday == Some(monday)
        && w.ground.week.roads.iter().any(|r| Some(r.man) == hc && r.days & (1 << k) != 0)
    {
        return Err("You're on the road that day.".into());
    }
    let minute = if dinner { DINNER } else { LUNCH };
    w.capital.booked.push(Booked { donor, date, minute });
    let cut = w.today.add(-14);
    w.capital.booked.retain(|b| b.date >= cut);
    let mut said = format!("{} with {} on {}.", meal(minute), d.name, date.pretty());
    // This week's plan: the slot is yours now, and one goes to your
    // coordinator if you're at five.
    if w.ground.week.monday == Some(monday) {
        let wk = &mut w.ground.week;
        // The office's slot that day, and his own if the office had him on
        // the week already.
        wk.slots.retain(|s| {
            !(s.kind == SlotKind::Donor && s.line.is_none() && (s.date == date && !s.yours || s.donor == Some(donor)))
        });
        let mut s = slot(
            date,
            minute,
            SlotKind::Donor,
            format!("{} with {}", meal(minute), d.name),
            "You asked for this one",
            Some(donor),
        );
        s.yours = true;
        let today = w.today;
        let yours = wk.slots.iter().filter(|x| x.yours).count();
        if yours >= crate::world::ground::week::plan::YOURS.1 {
            let drop = wk
                .slots
                .iter()
                .enumerate()
                .filter(|(_, x)| x.yours && x.kind != SlotKind::Meeting && x.line.is_none() && x.date > today)
                .min_by_key(|(_, x)| (rank_of(x), std::cmp::Reverse(x.date)))
                .map(|(k, _)| k);
            if let Some(k) = drop {
                wk.slots[k].yours = false;
                said.push_str(&format!(" Your coordinator takes {} instead.", wk.slots[k].title.to_lowercase()));
            }
        }
        wk.slots.push(s);
        wk.slots.sort_by_key(|s| (s.date, s.minute));
    }
    Ok(said)
}

/// You call off a booking (the office tells him politely).
pub fn unbook(w: &mut World, donor: u32, date: Date) -> Result<String, String> {
    your_program(w).ok_or("Donors are the head coach's to court.")?;
    let n = w.capital.booked.len();
    w.capital.booked.retain(|b| !(b.donor == donor && b.date == date));
    if w.capital.booked.len() == n {
        return Err("No such booking.".into());
    }
    let today = w.today;
    w.ground
        .week
        .slots
        .retain(|s| !(s.kind == SlotKind::Donor && s.donor == Some(donor) && s.date == date && s.date > today));
    Ok("Called off. The office sends his assistant a note.".into())
}

/// Your donor slots each morning (5 AM, after Ground's): today's that are
/// yours become scenes at their hour, all year; yesterday's that were the
/// development office's get their line.
pub(crate) fn dawn(w: &mut World, d: Date) {
    let Some(prog) = your_program(w) else { return };
    if w.ground.off || w.capital.off || crate::world::ground::campaign::waits(w, d) {
        return;
    }
    crate::world::ground::week::plan::ensure(w, prog, d);
    let slots: Vec<Slot> = w
        .ground
        .week
        .slots
        .iter()
        .filter(|s| matches!(s.kind, SlotKind::Donor | SlotKind::DonorRoom))
        .cloned()
        .collect();
    for s in slots.iter().filter(|s| s.date == d && s.yours) {
        if crate::world::ground::scenes::made(w, s).is_some() {
            continue;
        }
        if let Some(id) = scenes::start(w, prog, s) {
            w.ground.scenes.made.push((s.date, s.minute, s.kind.key().to_string(), id));
        }
    }
    let y = d.add(-1);
    for s in slots.iter().filter(|s| s.date == y && !s.yours) {
        if crate::world::ground::scenes::made(w, s).is_some() {
            continue;
        }
        let line = scenes::office_line(w, s);
        w.ground.scenes.made.push((s.date, s.minute, s.kind.key().to_string(), 0));
        set_line(w, s, &line);
    }
    let cut = d.add(-14);
    w.capital.booked.retain(|b| b.date >= cut);
}

/// A slot's line.
pub(crate) fn set_line(w: &mut World, s: &Slot, line: &str) {
    for x in w.ground.week.slots.iter_mut() {
        if x.date == s.date && x.minute == s.minute && x.kind == s.kind {
            x.line = Some(line.to_string());
        }
    }
}
