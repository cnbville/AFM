//! Your week on Ground (lane G3: GR12-GR15, GR47-GR51, GR53, GR54,
//! GR56-GR58; `docs/design/ground.md`, part E).
//!
//! - **The meeting at the map** (GR47): every Monday your recruiting
//!   coordinator drafts the week (`plan`): each man's days and area, your
//!   calls, what you host and the week's scenes. Your scouting assignments,
//!   your asks for a live look and the film queue are rows in it. You change
//!   any of it; what you leave, he plays.
//! - **Road days** (GR48): you spend your staff's recruiting-person days
//!   yourself (`road`), by each year's rule book: 140 in the spring window
//!   and 33 in the fall from 2023 (`scouting::dept::eval_limits`), up to the
//!   off-campus recruiters at once, the FBS head coach off the spring road
//!   from 2008. This replaces the built pace of 14 every spring Thursday.
//! - **Trips** (GR49): each trip comes out of the recruiting line
//!   (`finance::college`), shown as trips: drives cheap, a flight reaches a
//!   far area in a day, your AD's raise buys more flights.
//! - **One ledger** (GR50): your calls, your road days and what you host go
//!   on the same weekly ledger as film (`scouting::film::time`) with its
//!   tiredness.
//! - **Scenes as slots** (GR51): three to five a week come to you with the
//!   coordinator's reason, the rest he plays; until lane G5 makes them
//!   playable, each is its one-line result.
//! - **The day** (GR12-GR15, `phone`): the morning map's overnight list, the
//!   in-flight list, the night calls from the road, and calls that stop the
//!   day at their minute.
//! - **The beat** (GR56, `beat`), the May list (GR57) and July's week off
//!   (GR58).
//!
//! Only your college seat has a week. Nothing here is read by any other
//! program, so the world plays the same on every seed.

pub mod beat;
pub mod phone;
pub mod plan;
pub mod road;

use super::people::Who;
use crate::calendar::{Date, Weekday};
use crate::world::World;
use crate::world::scouting::readers::{self, Staff};

/// One man's road this week (GR47, GR48).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Road {
    /// The coach (index in the world's coaches).
    pub man: u32,
    /// The ground area (`ground::areas`).
    pub area: u16,
    /// Bit k is the Monday plus k days.
    pub days: u8,
    pub flight: bool,
    /// The coordinator's reason, or yours.
    pub why: String,
    /// You set this row (the coordinator leaves it alone).
    pub yours: bool,
    /// Schools he goes to first (high school ids): an assignment's player,
    /// a position group's kids, the cross-checks, your ask.
    #[serde(default)]
    pub focus: Vec<u32>,
}

/// A call on your list this week (GR53).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Planned {
    pub who: Who,
    pub why: String,
    /// 0 to make, 1 made by you, 2 made by your coordinator.
    pub state: u8,
}

/// What kind of scene a slot is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SlotKind {
    Meeting,
    SchoolVisit,
    CoachCall,
    NightCall,
    JuniorDay,
    Unofficial,
    Official,
    Camp,
    Film,
    /// Lane G5: your clinic, open practice, the spring game, the convention.
    Clinic,
    OpenPractice,
    SpringGame,
    Convention,
    /// Capital lane C4 (CP2, CP13): a lunch or dinner with a donor, and a
    /// room of donors (the suite before a home game, the spring caravan's
    /// dinner, the summer golf outing). Capital plays them
    /// (`capital::scenes`), all year.
    Donor,
    DonorRoom,
}

impl SlotKind {
    pub fn key(self) -> &'static str {
        match self {
            SlotKind::Meeting => "meeting",
            SlotKind::SchoolVisit => "school_visit",
            SlotKind::CoachCall => "coach_call",
            SlotKind::NightCall => "night_call",
            SlotKind::JuniorDay => "junior_day",
            SlotKind::Unofficial => "unofficial",
            SlotKind::Official => "official",
            SlotKind::Camp => "camp",
            SlotKind::Film => "film",
            SlotKind::Clinic => "clinic",
            SlotKind::OpenPractice => "open_practice",
            SlotKind::SpringGame => "spring_game",
            SlotKind::Convention => "convention",
            SlotKind::Donor => "donor",
            SlotKind::DonorRoom => "donor_room",
        }
    }
}

/// One of the week's scenes (GR51): at its hour, with the coordinator's
/// reason; `yours` comes to you, the rest he plays and you get the line.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Slot {
    pub date: Date,
    /// Local minutes after midnight.
    pub minute: u16,
    pub kind: SlotKind,
    pub title: String,
    pub reason: String,
    pub yours: bool,
    /// The man it's about, a person, a school.
    pub man: Option<u32>,
    pub who: Option<Who>,
    pub school: Option<u32>,
    /// Its one-line result once it has happened.
    pub line: Option<String>,
    /// The donor it is with (Capital lane C4's donor slots).
    #[serde(default)]
    pub donor: Option<u32>,
}

/// What kind of thing happened.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DoneKind {
    /// A man's school visit.
    Visit,
    /// A man's night call from the road (GR13).
    NightCall,
    /// A call you made, or your coordinator made for you.
    Call,
    /// A coach calling back (GR14).
    CallBack,
    /// A call you missed.
    Missed,
    /// Something you hosted.
    Hosted,
}

/// One thing that happened this week or the last few (kept five weeks).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Done {
    #[serde(default)]
    pub id: u32,
    pub date: Date,
    /// Local minutes after midnight.
    pub minute: u16,
    pub kind: DoneKind,
    pub man: Option<u32>,
    pub area: Option<u16>,
    pub school: Option<u32>,
    pub who: Option<Who>,
    /// Recruits it touched (player ids).
    pub ids: Vec<u32>,
    pub text: String,
    /// It stops the day at its minute (a live moment, GR14).
    pub matters: bool,
    /// You took it or read it.
    pub heard: bool,
    /// A level someone reached with it (0 none).
    pub level: u8,
}

/// A coach calling you back at his hour (GR12, GR14).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CallBack {
    pub id: u32,
    pub who: Who,
    pub date: Date,
    /// Local minutes after midnight.
    pub minute: u16,
    pub matters: bool,
    /// 0 waiting, 1 taken by you, 2 taken by your coordinator, 3 missed.
    pub state: u8,
    pub tries: u8,
}

/// Hours on your weekly ledger (GR50).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Hours {
    pub date: Date,
    pub what: String,
    pub hours: f32,
    /// You chose it (a call): it comes out of the week's free hours, the
    /// same ones film uses. Booked hours (what you host, your road day)
    /// count for tiredness and wear only.
    pub chosen: bool,
}

/// The May list (GR57).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MayList {
    pub date: Option<Date>,
    /// Each man's list from the spring road (coach, player ids).
    pub lists: Vec<(u32, Vec<u32>)>,
    /// The coordinator's ranking.
    pub ranked: Vec<u32>,
    /// Your open official visit spots, May to June.
    pub spots: u32,
    /// The ones invited.
    pub invited: Vec<u32>,
}

/// Your week's store, on `Ground.week`.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Week {
    /// The Monday the plan is for.
    pub monday: Option<Date>,
    pub roads: Vec<Road>,
    pub calls: Vec<Planned>,
    pub slots: Vec<Slot>,
    /// You handed the week to your coordinator: only live moments stop you.
    pub handed: bool,
    pub log: Vec<Done>,
    pub phone: Vec<CallBack>,
    pub ledger: Vec<Hours>,
    /// A week or two off in July's dead period (GR58): first and last day.
    pub off: Option<(Date, Date)>,
    /// Each Monday's letter (the newest six).
    pub letters: Vec<(Date, Vec<String>)>,
    /// The board's player ids at the last Monday (new names).
    pub board: Vec<u32>,
    /// Each person's level this morning (the morning map).
    pub levels: Vec<(Who, u8)>,
    /// The morning map's overnight lines and their day.
    pub overnight: Vec<String>,
    pub overnight_day: Option<Date>,
    pub may: MayList,
    pub(crate) next_id: u32,
}

/// The Monday of a day's week.
pub fn monday_of(d: Date) -> Date {
    d.on_or_before(Weekday::Mon)
}

/// Your college program, when Ground's week runs (a college seat).
pub fn seat_program(w: &World) -> Option<usize> {
    w.career.as_ref()?;
    match readers::seat_staff(w) {
        Some(Staff::Program(p)) => Some(p),
        _ => None,
    }
}

/// You're on your week off today.
pub fn off_now(w: &World) -> bool {
    w.ground.week.off.is_some_and(|(a, b)| w.today >= a && w.today <= b)
}

/// A man's name and job ("Coach Davis, defensive line").
pub fn man_words(w: &World, c: u32) -> String {
    let name = w.coaches.get(c as usize).map_or_else(|| "A coach".into(), |x| x.name.clone());
    let role = crate::world::folk::facts::coach_role(w, c as usize);
    format!("{name}, {}", role.to_lowercase())
}

/// A man's last name ("Davis").
pub fn last_name(w: &World, c: u32) -> String {
    let n = w.coaches.get(c as usize).map_or_else(|| "Coach".into(), |x| x.name.clone());
    n.rsplit(' ').next().unwrap_or(&n).to_string()
}

/// Your recruiting coordinator (the best recruiter on your staff).
pub fn coordinator(w: &World, prog: usize) -> Option<usize> {
    crate::world::front_office::college::coordinator(w, prog)
}

impl Week {
    pub(crate) fn next(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id
    }

    /// Hours on the ledger in the week of `monday` (chosen only, or all).
    pub fn hours(&self, monday: Date, chosen_only: bool) -> f32 {
        self.ledger.iter().filter(|h| monday_of(h.date) == monday && (h.chosen || !chosen_only)).map(|h| h.hours).sum()
    }

    /// Hours on the ledger on one day.
    pub fn hours_on(&self, d: Date) -> f32 {
        self.ledger.iter().filter(|h| h.date == d).map(|h| h.hours).sum()
    }

    /// Books hours once (by day and what).
    pub(crate) fn book(&mut self, d: Date, what: &str, hours: f32, chosen: bool) {
        if !chosen && self.ledger.iter().any(|h| h.date == d && h.what == what) {
            return;
        }
        self.ledger.push(Hours { date: d, what: what.to_string(), hours, chosen });
    }
}

/// Booked ground hours past what a week carries anyway (your Saturday's
/// junior day is your work week too, the second big weekend isn't): what
/// the film room's wear counts on top of the hours you work (proposed, not
/// confirmed).
pub const BOOKED_FREE: f32 = 12.0;

/// Your ground hours this week that come out of film's free hours (calls
/// you make): `scouting::film::time::used` adds them.
pub fn chosen_hours(w: &World) -> f32 {
    if seat_program(w).is_none() {
        return 0.0;
    }
    w.ground.week.hours(monday_of(w.today), true)
}

/// Your ground hours today: `scouting::film::time::tiredness` counts them.
pub fn hours_today(w: &World) -> f32 {
    if seat_program(w).is_none() {
        return 0.0;
    }
    w.ground.week.hours_on(w.today)
}

/// The booked hours past `BOOKED_FREE` in the week of `monday` (the film
/// room's wear).
pub fn booked_over(w: &World, monday: Date) -> f32 {
    if seat_program(w).is_none() {
        return 0.0;
    }
    let all = w.ground.week.hours(monday, false);
    let chosen = w.ground.week.hours(monday, true);
    (all - chosen - BOOKED_FREE).max(0.0)
}

/// Your seat's week each day, before the department's work: the morning
/// map, the week's turn on Mondays, what you host on the ledger, calls
/// that lapsed, the May list, your week off.
pub(crate) fn day(w: &mut World, d: Date) {
    let Some(prog) = seat_program(w) else { return };
    // Your first season from a Jan 1 start: Ground waits until the
    // January window has closed (GR31, lane G6).
    if super::campaign::waits(w, d) {
        return;
    }
    phone::morning(w, prog, d);
    if d.weekday() == Weekday::Mon || w.ground.week.monday.is_none_or(|m| m != monday_of(d)) {
        plan::turn(w, prog, d);
    }
    phone::lapse(w, prog, d);
    hosted(w, prog, d);
    if d.weekday() == Weekday::Sat {
        plan::coordinator_calls(w, prog, d);
    }
    if d == beat::may_list_day(d.year()) {
        plan::may_list(w, prog, d);
    }
    if let Some((a, _)) = w.ground.week.off
        && a == d
    {
        let fr = &mut w.scouting.film;
        fr.wear = 0.0;
        fr.today = None;
    }
    // Keep five weeks.
    let cut = d.add(-35);
    let wk = &mut w.ground.week;
    wk.log.retain(|x| x.date >= cut);
    wk.ledger.retain(|x| x.date >= cut);
    wk.phone.retain(|x| x.date >= cut);
}

/// What you host goes on the ledger at its real time (GR50): a junior day
/// your Saturday, an official visit weekend Friday evening to Sunday
/// breakfast, an unofficial visit 2 to 3 hours, a home visit two, your camp
/// a day (sizes from `ground/week.md`).
fn hosted(w: &mut World, prog: usize, d: Date) {
    use crate::world::Visit;
    let (mut off, mut un, mut home) = (0usize, 0usize, 0usize);
    for q in
        w.people.iter().filter(|q| matches!(q.stage, crate::world::Stage::HighSchool | crate::world::Stage::PrepSchool))
    {
        for v in q.cal.visits.iter().filter(|v| v.0 == prog && v.1 == d) {
            match v.2 {
                Visit::Official => off += 1,
                Visit::Unofficial | Visit::DropIn => un += 1,
                Visit::Home => home += 1,
            }
        }
    }
    let wd = d.weekday();
    let junior = wd == Weekday::Sat && d >= w.dates.junior_days.0 && d <= w.dates.junior_days.1 && un > 0;
    let camp = beat::camp_days(w.dates.season).contains(&d);
    let wk = &mut w.ground.week;
    if junior {
        wk.book(d, &format!("Junior day: {un} on campus"), 8.0, false);
    } else if un > 0 {
        wk.book(d, &format!("Unofficial visits: {un}"), (2.5 * un as f32).min(6.0), false);
    }
    if off > 0 {
        let h = match wd {
            Weekday::Fri => 3.0,
            Weekday::Sat => 8.0,
            Weekday::Sun => 2.0,
            _ => 4.0,
        };
        wk.book(d, &format!("Official visits: {off}"), h, false);
    }
    if home > 0 {
        wk.book(d, &format!("Home visits: {home}"), (2.0 * home as f32).min(8.0), false);
    }
    if camp {
        wk.book(d, "Your camp", 8.0, false);
    }
}

/// Takes a week or two off in July's dead period (GR58): your tiredness
/// goes back to zero, your family gets the time and your coordinator holds
/// the phones (a call he takes counts as his).
pub fn take_off(w: &mut World, from: Date, weeks: u32) -> Result<String, String> {
    seat_program(w).ok_or("Ground's week is for a college head coach's seat.")?;
    if !(1..=2).contains(&weeks) {
        return Err("A week or two.".into());
    }
    if from < w.today {
        return Err("That week has gone.".into());
    }
    let to = from.add(7 * weeks as i32 - 1);
    let y = beat::dates_for(w, from);
    for k in 0..(7 * weeks as i32) {
        let d = from.add(k);
        if crate::calendar::recruiting_period(&y, d) != crate::calendar::Period::Dead || !matches!(d.month(), 6 | 7) {
            return Err(format!("Only inside the summer dead period: {} isn't.", beat::short(d)));
        }
    }
    w.ground.week.off = Some((from, to));
    Ok(format!(
        "You're off {} to {}. Your coordinator holds the phones; your family has you for the time.",
        beat::short(from),
        beat::short(to)
    ))
}
