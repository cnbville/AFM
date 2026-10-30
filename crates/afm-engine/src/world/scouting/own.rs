//! Your own side of the scouting fresh look (update 2, SK7-SK22): the May
//! review of your scouts, your college personnel hires, your own days out
//! and what the press makes of them, your eye's own record, the interview
//! room, board meetings, hard minimums, the scouting services, the NFL
//! scouts on your campus and your word to them, your pro day, the NFL's
//! advisory grades and your stay-or-go talks, and the athletes in your
//! high school's building who don't play football yet.
//!
//! Everything the screens show is your staff's read: ranges, words and
//! counts, never a true rating. The rules are the real ones; the sizes are
//! Claude's, proposed, not confirmed (listed in `docs/game.md`, lane 8).

use std::collections::HashMap;

use super::Kind;
use super::readers::{self, COACH, Staff};
use crate::attributes::Attr;
use crate::calendar::{Date, Event, Kind as CalKind, Level, Loudness, Period, Weekday};
use crate::positions::{Position, overall_for};
use crate::rng::{Rng, clamp};
use crate::world::pro::scouts::{self, Role, Scout};
use crate::world::season::{key_normal, key_uniform};
use crate::world::{Stage, World};

/// Everything your seat keeps for lane 8 (stored on `Scouting::own`; every
/// field fills in empty on an older file).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Own {
    /// SK11: this year's May review while it is open.
    #[serde(default)]
    pub review: Option<Review>,
    /// SK7 at a college: your personnel staff once you have hired, by post
    /// (an empty post is None). Empty: the program's staff as it was.
    #[serde(default)]
    pub personnel: Vec<Option<Hire>>,
    /// SK12-SK13: your own days out.
    #[serde(default)]
    pub trips: Vec<Trip>,
    /// Bug 82: your own film hours this week (its Monday, hours spent).
    #[serde(default)]
    pub film_week: Option<(Date, f32)>,
    /// SK14: the looks of yours already credited to your eye.
    #[serde(default)]
    pub eye_credited: Vec<u32>,
    /// What your eye has gained from your own looks (rating points).
    #[serde(default)]
    pub eye_gain: f32,
    /// SK15: the interviews you ran.
    #[serde(default)]
    pub interviews: Vec<Interview>,
    /// SK16: board meetings held.
    #[serde(default)]
    pub meetings: Vec<Meeting>,
    /// SK17: your department's hard minimums.
    #[serde(default)]
    pub minimums: Vec<Minimum>,
    /// SK18: your club's scouting service, and a change you made that
    /// takes effect after the draft.
    #[serde(default)]
    pub service: Service,
    #[serde(default)]
    pub service_next: Option<Service>,
    /// SK19: how open your practices are to NFL scouts (0 closed, 1 open,
    /// 2 open with the film room), your word on your players and each NFL
    /// club's credit for it.
    #[serde(default = "one")]
    pub openness: u8,
    #[serde(default)]
    pub words: Vec<Word>,
    #[serde(default)]
    pub cred: HashMap<u8, f32>,
    /// SK21: your advisory requests (player id, the December they were
    /// for) and your stay-or-go talks (player id, year, 0 stay, 1 your
    /// call, 2 go).
    #[serde(default)]
    pub advisory: Vec<(u32, i32)>,
    #[serde(default)]
    pub talks: Vec<(u32, i32, u8)>,
    /// SK22: athletes in your building you talked to, by key.
    #[serde(default)]
    pub athletes: Vec<AthleteTalk>,
}

fn one() -> u8 {
    1
}

// --- SK11: the May review ------------------------------------------------------------

/// Your call on one scout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Call {
    #[default]
    Undecided,
    Renew,
    Raise,
    Promote,
    LetGo,
}

impl Call {
    pub fn key(self) -> &'static str {
        match self {
            Call::Undecided => "undecided",
            Call::Renew => "renew",
            Call::Raise => "raise",
            Call::Promote => "promote",
            Call::LetGo => "let_go",
        }
    }
    pub fn from_key(k: &str) -> Option<Call> {
        [Call::Undecided, Call::Renew, Call::Raise, Call::Promote, Call::LetGo].into_iter().find(|c| c.key() == k)
    }
    pub fn words(self) -> &'static str {
        match self {
            Call::Undecided => "Not decided",
            Call::Renew => "Renew",
            Call::Raise => "Renew with a raise",
            Call::Promote => "Promote",
            Call::LetGo => "Let go",
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Review {
    pub year: i32,
    /// When the contracts end (June 1): undecided calls are renewals.
    pub due: Date,
    /// Your call on each scout, by his place in the department.
    pub calls: Vec<(u8, Call)>,
    /// Scouts you signed from the market: the place they take (a scout
    /// you let go, or a new post at 255), and the man.
    pub hires: Vec<(u8, Scout)>,
}

/// Right after the draft, when scouting contracts end: the review opens
/// with every scout's look-backs in front of you (SK11).
pub(crate) fn open_review(w: &mut World) {
    let Some(t) = w.seat.and_then(|s| s.nfl_team) else { return };
    let year = w.year;
    if w.scouting.own.review.as_ref().is_some_and(|r| r.year == year) {
        return;
    }
    let n = w.clubs[t].scouts.len();
    let due = Date::ymd(year, 6, 1).max(w.today.add(10));
    w.scouting.own.review =
        Some(Review { year, due, calls: (0..n).map(|k| (k as u8, Call::Undecided)).collect(), hires: Vec::new() });
    let text = format!(
        "The May review: your {n} scouts' contracts end on {}. Renew, raise, promote or let go, with each man's look-backs in front of you.",
        due.pretty()
    );
    w.calendar.post(
        Event::new(w.today, Level::Nfl, CalKind::Coaching, Loudness::Medium, text)
            .team(t)
            .decision()
            .rule("The May review"),
    );
}

/// Your call on one scout in the review.
pub fn review_call(w: &mut World, place: u8, call: Call) -> Result<String, String> {
    let t = w.seat.and_then(|s| s.nfl_team).ok_or("Only an NFL club holds a May review.")?;
    let r = w.scouting.own.review.as_ref().ok_or("The May review opens after the draft.")?;
    let s = w.clubs[t].scouts.get(place as usize).ok_or("No such scout.")?;
    if call == Call::Promote && s.role == Role::Director {
        return Err("He is already your director.".into());
    }
    if call == Call::Promote && s.role == Role::National {
        let dir = w.clubs[t].scouts.iter().position(|x| x.role == Role::Director).unwrap_or(0) as u8;
        let dir_out = r.calls.iter().any(|&(k, c)| k == dir && c == Call::LetGo);
        if !dir_out {
            return Err("A national scout steps up to director only when you let your director go.".into());
        }
    }
    let name = s.name.clone();
    let r = w.scouting.own.review.as_mut().unwrap();
    match r.calls.iter_mut().find(|x| x.0 == place) {
        Some(x) => x.1 = call,
        None => r.calls.push((place, call)),
    }
    Ok(format!("{name}: {}.", call.words().to_lowercase()))
}

/// The market (SK7): every scout without a club. The store keeps the AI's
/// order; the screen sorts by what a club can see.
pub fn market(w: &World) -> &[Scout] {
    &w.scouting.market
}

/// You sign a scout from the market into a place (a scout you let go) or
/// a new post (SK7).
pub fn sign(w: &mut World, k: usize, place: Option<u8>) -> Result<String, String> {
    let t = w.seat.and_then(|s| s.nfl_team).ok_or("Only an NFL club hires scouts.")?;
    let r = w.scouting.own.review.as_ref().ok_or("Scouts sign after the draft, in the May review.")?;
    if k >= w.scouting.market.len() {
        return Err("He has signed elsewhere.".into());
    }
    let slot = match place {
        Some(p) => {
            if !r.calls.iter().any(|&(q, c)| q == p && c == Call::LetGo) {
                return Err("Let a scout go first: his post is the one you fill.".into());
            }
            if r.hires.iter().any(|h| h.0 == p) {
                return Err("You have already filled that post.".into());
            }
            p
        }
        None => {
            let (areas, nats) = scout_sizes(w, t);
            let extra = r.hires.iter().filter(|h| h.0 == 255).count();
            if w.clubs[t].scouts.len() + extra >= areas + nats + 2 {
                return Err("Your owner won't pay for another post.".into());
            }
            255
        }
    };
    let mut s = w.scouting.market.remove(k);
    let year = w.year;
    s.since = year;
    s.pay = s.market_pay(year).max(s.pay);
    let name = s.name.clone();
    w.scouting.own.review.as_mut().unwrap().hires.push((slot, s));
    Ok(format!("{name} signs with you from June 1."))
}

fn scout_sizes(w: &World, t: usize) -> (usize, usize) {
    let areas = w.clubs[t].scouts.iter().filter(|s| s.role == Role::Area).count();
    let nats = w.clubs[t].scouts.iter().filter(|s| s.role == Role::National).count() + 1;
    (areas, nats)
}

/// The review closes (June 1, or when you say so): your calls take hold,
/// the good men you didn't raise may take another club's offer, empty
/// posts are filled from the market, and each man's year on shows (SK8,
/// SK9).
pub fn close_review(w: &mut World) -> Result<String, String> {
    let t = w.seat.and_then(|s| s.nfl_team).ok_or("Only an NFL club holds a May review.")?;
    let r = w.scouting.own.review.take().ok_or("No review is open.")?;
    let year = w.year;
    let mut rng = Rng::new(w.options.seed ^ 0x5E71_0000 ^ (year as u32).wrapping_mul(7_919));
    let budget = crate::world::finance::nfl::spend_level(w, t);
    let mut scouts = std::mem::take(&mut w.clubs[t].scouts);
    let call_of = |k: usize| r.calls.iter().find(|x| x.0 as usize == k).map_or(Call::Renew, |x| x.1);
    let mut changed: Vec<u8> = Vec::new();
    let mut left: Vec<String> = Vec::new();
    let mut out = Vec::new();
    // His record from your look-backs, kept with him.
    for (k, s) in scouts.iter_mut().enumerate() {
        let (held, high, low) = super::lookback::record_of(&w.scouting.records, k as u32);
        s.held = s.held.max(held as u16);
        s.missed = s.missed.max((high + low) as u16);
    }
    let promoted_dir = r
        .calls
        .iter()
        .any(|&(k, c)| c == Call::Promote && scouts.get(k as usize).is_some_and(|s| s.role == Role::National));
    for (k, mut s) in scouts.into_iter().enumerate() {
        let call = call_of(k);
        match call {
            Call::LetGo => {
                s.clubs.push(t as u8);
                left.push(s.name.clone());
                w.scouting.market.push(s.clone());
                changed.push(k as u8);
                // A placeholder until the post is filled below.
                s.name.clear();
            }
            Call::Raise => s.pay = (s.pay as f64 * 1.12).round() as u32,
            Call::Promote => {
                s.role = match s.role {
                    Role::Area => Role::National,
                    _ => Role::Director,
                };
                s.pay = s.market_pay(year).max(s.pay);
            }
            _ => {}
        }
        if !s.name.is_empty() {
            // The AI clubs' rule (`scouts::staff`): one in ten moves on, a
            // sharp man at a thrifty club more often (a raise keeps most of
            // them), the old retire.
            let age = s.age(year);
            let sharp = if s.judge >= 15 { 0.12 * (1.0 - budget) } else { 0.0 };
            let sharp = if call == Call::Raise { 0.4 * sharp } else { sharp };
            let leave = 0.1 + sharp + if age >= 64 { 0.5 } else { 0.0 };
            if rng.chance(leave) {
                left.push(format!("{} ({})", s.name, if age >= 64 { "retired" } else { "took another job" }));
                if age < 64 {
                    s.clubs.push(t as u8);
                    w.scouting.market.push(s.clone());
                }
                changed.push(k as u8);
                s.name.clear();
            }
        }
        if !s.name.is_empty() {
            scouts::grow(&mut s, year, &mut rng);
        }
        out.push(s);
    }
    // A director let go without a national promoted: the post is filled.
    let _ = promoted_dir;
    for (slot, s) in r.hires {
        if slot != 255 && (slot as usize) < out.len() {
            out[slot as usize] = s;
        } else {
            out.push(s);
        }
    }
    for k in 0..out.len() {
        if out[k].name.is_empty() {
            let role = out[k].role;
            let region = out[k].region;
            let mut s = scouts::sign(w, &mut rng, t, role, budget, true);
            s.region = region;
            out[k] = s;
        }
    }
    // One director.
    let mut seen_dir = false;
    for s in out.iter_mut() {
        if s.role == Role::Director {
            if seen_dir {
                s.role = Role::National;
            }
            seen_dir = true;
        }
    }
    if !seen_dir && let Some(s) = out.iter_mut().find(|s| s.role == Role::National) {
        s.role = Role::Director;
    }
    let areas = out.iter().filter(|s| s.role == Role::Area).count();
    let mut area: Vec<Scout> = Vec::new();
    let mut idx: Vec<usize> = Vec::new();
    for (k, s) in out.iter().enumerate() {
        if s.role == Role::Area {
            area.push(s.clone());
            idx.push(k);
        }
    }
    scouts::place_areas(&mut area, areas);
    for (s, k) in area.into_iter().zip(idx) {
        out[k] = s;
    }
    w.clubs[t].scouts = out;
    // A new man doesn't carry the old one's look-backs.
    for g in w.scouting.records.drafts.iter_mut().chain(w.scouting.records.classes.iter_mut()) {
        if g.lead.is_some_and(|l| changed.contains(&(l as u8)) && l < 250) {
            g.lead = None;
        }
    }
    let text = if left.is_empty() {
        "The May review is done: your department stays together.".to_string()
    } else {
        format!("The May review is done. Leaving: {}.", left.join(", "))
    };
    w.calendar.post(Event::new(w.today, Level::Nfl, CalKind::Coaching, Loudness::Small, text.clone()).team(t));
    Ok(text)
}

// --- SK7 at a college: your personnel staff -----------------------------------------------

/// A personnel staffer you hired (SK7): his ratings are hidden; the
/// screens show his record and his work.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Hire {
    pub name: String,
    pub job: u8,
    pub judge: u8,
    pub potential: u8,
    pub athletic: u8,
    pub character: u8,
    pub network: u8,
    pub work: u8,
    pub born: i32,
    pub since: i32,
    pub key: u64,
    /// Programs he worked for, oldest first.
    pub past: Vec<u16>,
    pub held: u16,
    pub missed: u16,
}

/// The posts a college personnel staff can hold.
pub const PERSONNEL_JOBS: [&str; 6] = [
    "General manager",
    "Director of player personnel",
    "Director of recruiting",
    "Assistant director of personnel",
    "Recruiting analyst",
    "On-campus recruiting coordinator",
];

pub fn job_index(job: &str) -> u8 {
    PERSONNEL_JOBS.iter().position(|j| *j == job).unwrap_or(4) as u8
}

/// This year's personnel candidates for your program (SK7): men off other
/// staffs, a few climbing from analyst jobs, each with his past programs
/// and his look-backs there. Drawn by key, so the list holds all year.
pub fn personnel_market(w: &World) -> Vec<Hire> {
    let Some(prog) = readers::seat_staff(w).and_then(|s| if let Staff::Program(p) = s { Some(p) } else { None }) else {
        return Vec::new();
    };
    let year = w.year;
    let mut rng = Rng::new(w.options.seed ^ 0x9E57_0000 ^ (year as u32).wrapping_mul(31) ^ prog as u32);
    let money = clamp(0.4 * w.programs[prog].facilities / 20.0 + 0.6 * w.programs[prog].prestige / 100.0, 0.0, 1.0);
    let n = 12;
    let others: Vec<u16> = (0..w.programs.len())
        .filter(|&q| q != prog && w.programs[q].plays(year) && w.programs[q].division == w.programs[prog].division)
        .map(|q| q as u16)
        .collect();
    (0..n)
        .map(|k| {
            let mean = 7.5 + 3.0 * money + rng.normal(0.0, 1.2);
            let mut r = || clamp(rng.normal(mean, 2.8), 1.0, 20.0).round() as u8;
            let (judge, potential, athletic, character, network, work) = (r(), r(), r(), r(), r(), r());
            let age = rng.int(24, 58);
            let eth = scouts::staff_ethnicity(&mut rng, year);
            let name = crate::world::names::person_name(eth, year - age, &mut rng);
            let years = (age - 22).clamp(1, 20);
            let stops = (years / 4).clamp(1, 4);
            let past = (0..stops)
                .filter_map(|_| others.get(rng.int(0, others.len().max(1) as i32 - 1) as usize).copied())
                .collect();
            let p = clamp(0.35 + 0.025 * (judge as f64 - 10.0), 0.1, 0.7);
            let reads = (years * 4).min(60) as u16;
            let held = (0..reads).filter(|_| rng.chance(p)).count() as u16;
            let job = match k % 6 {
                0 => 1,
                1 => 2,
                2 | 3 => 4,
                4 => 3,
                _ => 5,
            };
            Hire {
                name,
                job,
                judge,
                potential,
                athletic,
                character,
                network,
                work,
                born: year - age,
                since: year,
                key: (prog as u64) << 32 ^ (year as u64) << 8 ^ k as u64 ^ 0x9E51,
                past,
                held,
                missed: reads - held,
            }
        })
        .filter(|h| !w.scouting.own.personnel.iter().flatten().any(|x| x.key == h.key))
        .collect()
}

/// Your posts: the program's staff as it stands when you first change it.
fn take_personnel(w: &mut World, prog: usize) {
    if !w.scouting.own.personnel.is_empty() {
        return;
    }
    let year = w.year;
    let staff = readers::keyed_personnel(w, prog);
    w.scouting.own.personnel = staff
        .into_iter()
        .map(|p| {
            Some(Hire {
                name: p.name,
                job: job_index(p.job),
                judge: p.judge as u8,
                potential: p.potential as u8,
                athletic: p.athletic as u8,
                character: p.character as u8,
                network: p.network as u8,
                work: p.work as u8,
                born: year - 35,
                since: p.since,
                key: p.key,
                past: Vec::new(),
                held: 0,
                missed: 0,
            })
        })
        .collect();
}

/// You hire a personnel candidate into a post (an empty one, or a new one
/// your budget allows).
pub fn hire_personnel(w: &mut World, key: u64) -> Result<String, String> {
    let Some(Staff::Program(prog)) = readers::seat_staff(w) else {
        return Err("Only a college hires personnel staff.".into());
    };
    let h = personnel_market(w).into_iter().find(|h| h.key == key).ok_or("He has taken another job.")?;
    take_personnel(w, prog);
    let cap = readers::keyed_personnel(w, prog).len().max(1) + 1;
    let own = &mut w.scouting.own.personnel;
    let name = h.name.clone();
    let job = PERSONNEL_JOBS[h.job as usize];
    if let Some(slot) = own.iter_mut().find(|x| x.is_none()) {
        *slot = Some(h);
    } else if own.len() < cap {
        own.push(Some(h));
    } else {
        return Err("Every post you can pay for is filled: let someone go first.".into());
    }
    Ok(format!("{name} joins your staff as {}.", job.to_lowercase()))
}

/// You let a personnel staffer go.
pub fn let_go_personnel(w: &mut World, post: usize) -> Result<String, String> {
    let Some(Staff::Program(prog)) = readers::seat_staff(w) else {
        return Err("Only a college has personnel staff.".into());
    };
    take_personnel(w, prog);
    let slot = w.scouting.own.personnel.get_mut(post).ok_or("No such post.")?;
    let h = slot.take().ok_or("That post is empty.")?;
    Ok(format!("{} leaves your staff.", h.name))
}

/// A year on for your personnel staff (SK8), on July 1.
fn grow_personnel(w: &mut World) {
    let year = w.year;
    let mut rng = Rng::new(w.options.seed ^ 0x9E58_0000 ^ year as u32);
    for h in w.scouting.own.personnel.iter_mut().flatten() {
        let age = year - h.born;
        let youth = match age {
            ..38 => 1.0,
            38..48 => 0.6,
            48..58 => 0.3,
            _ => 0.1,
        };
        let n = (h.held + h.missed) as f64;
        let honest = if n > 0.0 { h.held as f64 / n } else { 0.5 };
        let p = 0.32 * youth * (0.5 + honest);
        if rng.chance(p) && h.judge < 20 {
            h.judge += 1;
        }
        if rng.chance(0.8 * p) && h.potential < 20 {
            h.potential += 1;
        }
        if age >= 56 && rng.chance(0.35) && h.work > 1 {
            h.work -= 1;
        }
    }
}

// --- SK12-SK14: your own days out -----------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TripKind {
    /// His college or high school game.
    Game,
    /// His practice (college evaluation days).
    Practice,
    /// His pro day.
    ProDay,
    /// A private workout at his school.
    Workout,
    /// All-star week.
    AllStar,
    /// A move-in's old film (high school).
    OldFilm,
    /// The middle school's game: next year's freshmen (high school).
    MiddleSchool,
}

impl TripKind {
    pub fn key(self) -> &'static str {
        match self {
            TripKind::Game => "game",
            TripKind::Practice => "practice",
            TripKind::ProDay => "pro_day",
            TripKind::Workout => "workout",
            TripKind::AllStar => "all_star",
            TripKind::OldFilm => "old_film",
            TripKind::MiddleSchool => "middle_school",
        }
    }
    pub fn from_key(k: &str) -> Option<TripKind> {
        use TripKind::*;
        [Game, Practice, ProDay, Workout, AllStar, OldFilm, MiddleSchool].into_iter().find(|t| t.key() == k)
    }
    pub fn words(self) -> &'static str {
        match self {
            TripKind::Game => "His game",
            TripKind::Practice => "His practice",
            TripKind::ProDay => "His pro day",
            TripKind::Workout => "A private workout",
            TripKind::AllStar => "All-star week",
            TripKind::OldFilm => "His old school's film",
            TripKind::MiddleSchool => "The middle school's game",
        }
    }
    fn look(self) -> Kind {
        match self {
            TripKind::Game | TripKind::MiddleSchool => Kind::Game,
            TripKind::Practice => Kind::Practice,
            TripKind::ProDay => Kind::ProDay,
            TripKind::Workout => Kind::Workout,
            TripKind::AllStar => Kind::AllStar,
            TripKind::OldFilm => Kind::HsFilm,
        }
    }
}

/// One of your days out.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Trip {
    pub date: Date,
    pub id: u32,
    pub kind: TripKind,
    /// Taken (the look is in the file).
    pub done: bool,
    /// The press or agents reported you there (SK13).
    pub seen: bool,
    /// Who went in your place (a reader code), if you sent someone.
    pub sent: Option<u32>,
}

/// Your own coach's reader code.
fn me(w: &World) -> Option<u32> {
    readers::you(w).map(|c| COACH + c as u32)
}

/// The trips a seat makes.
pub fn trip_kinds(staff: Staff) -> &'static [TripKind] {
    match staff {
        Staff::Club(_) => &[TripKind::Game, TripKind::ProDay, TripKind::Workout, TripKind::AllStar],
        Staff::Program(_) => &[TripKind::Game, TripKind::Practice],
        Staff::School(_) => &[TripKind::OldFilm, TripKind::MiddleSchool],
    }
}

/// The day a trip can be taken, or why not (SK12): the NCAA's evaluation
/// periods and the head coach's off-campus limits at a college; the
/// season's Saturdays, pro days, the spring's workouts and all-star week
/// at an NFL club; your own building at a high school.
pub fn trip_day(w: &World, id: u32, kind: TripKind) -> Result<Date, String> {
    let i = super::place(w, id).ok_or("No such player.")?;
    let p = &w.people[i];
    let today = w.today;
    let staff = readers::seat_staff(w).ok_or("Start a career first.")?;
    let (_, m, _) = today.parts();
    match (staff, kind) {
        (Staff::Club(_), TripKind::Game) => {
            if !matches!(p.stage, Stage::College) {
                return Err("He isn't playing college games.".into());
            }
            if !matches!(m, 9..=11) {
                return Err("College games are on Saturdays from September to November.".into());
            }
            let sat = today.add(1).on_or_after(Weekday::Sat);
            if w.career.as_ref().and_then(|c| c.role) == Some(crate::world::coach::Role::HeadCoach) {
                return Err("A head coach in season is with his own team on weekends: send a scout.".into());
            }
            Ok(sat)
        }
        (Staff::Club(_), TripKind::ProDay) => {
            if !matches!(p.stage, Stage::College | Stage::Declared) {
                return Err("Pro days are for this year's draft prospects.".into());
            }
            let c = p.college.ok_or("He has no school to hold a pro day.")?;
            let d = w
                .nfl_season
                .as_ref()
                .and_then(|s| s.pro_days.iter().find(|x| x.0 == c).map(|x| x.1))
                .ok_or("His school's pro day isn't set yet: they are set for March and April.")?;
            // Last spring's dates stand until the new ones are set.
            if d < today && d.year() < super::public::draft_year(w) {
                return Err("His school's pro day isn't set yet: they are set for March and April.".into());
            }
            if d < today {
                return Err("His school's pro day has been held.".into());
            }
            Ok(d)
        }
        (Staff::Club(_), TripKind::Workout) => {
            if p.stage != Stage::Declared {
                return Err("Private workouts are for declared prospects, after the combine.".into());
            }
            if !matches!(m, 3 | 4) {
                return Err("Private workouts run from March to the draft.".into());
            }
            Ok(today.add(2))
        }
        (Staff::Club(_), TripKind::AllStar) => {
            if !(m == 1 && p.stage == Stage::Declared) && !(m == 1 && p.stage == Stage::College) {
                return Err("All-star week is in late January.".into());
            }
            Ok(today.add(1))
        }
        (Staff::Program(_), TripKind::Game | TripKind::Practice) => {
            if !matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) {
                return Err("Your evaluation days are for high school recruits.".into());
            }
            let season = w.dates.season;
            let (lf, ls) = super::dept::eval_limits(season);
            let (_, uf, us) = w.scouting.dept.eval_days;
            let wanted = if kind == TripKind::Game { Weekday::Fri } else { Weekday::Thu };
            for k in 1..=21 {
                let d = today.add(k);
                if d.weekday() != wanted {
                    continue;
                }
                let period = crate::calendar::recruiting_period(&w.dates, d);
                let fall = period == Period::Evaluation && matches!(d.parts().1, 9..=11);
                let spring = (period == Period::Contact && d.parts().1 == 4)
                    || (d.parts().1 == 5 && period == Period::Quiet && d.parts().2 <= 23);
                if fall && uf < lf {
                    return Ok(d);
                }
                if spring && kind == TripKind::Practice && us < ls {
                    // The 2008 rule: head coaches stay home in the spring
                    // evaluation period.
                    if is_head(w) && d.year() >= 2008 {
                        return Err(
                            "Since 2008 a head coach can't go out in the spring evaluation period: send an assistant."
                                .into(),
                        );
                    }
                    return Ok(d);
                }
            }
            Err("No evaluation day is open in the next three weeks (or the staff's days are spent).".into())
        }
        (Staff::School(h), TripKind::OldFilm) => {
            if p.hs != h || !p.hs_lines.iter().any(|l| l.school as usize != h) {
                return Err("Only a move-in to your school has an old school's film.".into());
            }
            Ok(today)
        }
        (Staff::School(h), TripKind::MiddleSchool) => {
            let next = w.dates.season + 4;
            if p.hs != h || p.class_of < next {
                return Err("The middle school's games show next year's freshmen.".into());
            }
            if !matches!(m, 9..=11) {
                return Err("Middle school games are in the fall.".into());
            }
            Ok(today.add(1).on_or_after(Weekday::Thu))
        }
        _ => Err("That isn't a trip your seat makes.".into()),
    }
}

fn is_head(w: &World) -> bool {
    w.career.as_ref().and_then(|c| c.role).is_none_or(|r| r == crate::world::coach::Role::HeadCoach)
}

/// You go yourself, or send someone in your place (SK12, SK13).
pub fn go(w: &mut World, id: u32, kind: TripKind, send: Option<u32>) -> Result<String, String> {
    let d = trip_day(w, id, kind)?;
    let you = me(w).ok_or("Start a career first.")?;
    if w.scouting.own.trips.iter().any(|t| t.date == d && t.sent.is_none() && send.is_none() && !t.done) {
        return Err("You already have a trip that day.".into());
    }
    let i = super::place(w, id).ok_or("No such player.")?;
    let name = w.people[i].name.clone();
    w.scouting.own.trips.push(Trip { date: d, id, kind, done: false, seen: false, sent: send.filter(|&s| s != you) });
    if d == w.today {
        take_trips(w, d);
    }
    let who = if send.is_some() { "Sent" } else { "You go" };
    Ok(format!("{who}: {} for {name}, {}.", kind.words().to_lowercase(), d.pretty()))
}

// --- Bug 82: watching a player's film yourself -------------------------------------------

/// Your own film hours a week (lane 9: SK60, SK61): the real calendar at
/// the hours you choose to work, game prep included in season. One
/// ledger for all your film (`film::time`).
pub fn film_budget(w: &World) -> f32 {
    super::film::time::budget(w)
}

fn monday(d: Date) -> Date {
    super::film::time::monday(d)
}

/// Your film hours left this week.
pub fn film_left(w: &World) -> f32 {
    let used = w.scouting.own.film_week.filter(|x| x.0 == monday(w.today)).map_or(0.0, |x| x.1);
    (film_budget(w) - used).max(0.0)
}

/// What watching him takes, and the look it is: a recruit's reel and a
/// full game, about an hour; a college player's three games, every snap
/// of his, two and a half; a pro's cut-ups, two.
pub fn film_cost(w: &World, id: u32) -> Result<(Kind, f32, &'static str), String> {
    let i = super::place(w, id).ok_or("No such player.")?;
    let p = &w.people[i];
    let f = super::footage::footage(w, i);
    match p.stage {
        Stage::HighSchool | Stage::PrepSchool if f.snaps > 0.0 || f.reels => {
            Ok((Kind::HsFilm, QUICK, "a cut-up of his snaps"))
        }
        Stage::HighSchool | Stage::PrepSchool => {
            Err("There's no film of him yet: he hasn't played a high school game.".into())
        }
        Stage::College | Stage::Declared if f.snaps > 0.0 => Ok((Kind::CollegeFilm, QUICK, "a cut-up of his snaps")),
        Stage::College | Stage::Declared => Err("He hasn't played a college snap: there's nothing to watch.".into()),
        Stage::Pro
            if p.pro.as_ref().and_then(|c| c.team.or(c.teams.last().copied())).is_some_and(|t| t < w.nfl.len()) =>
        {
            Ok((Kind::ProFilm, QUICK, "a cut-up of his snaps"))
        }
        Stage::Pro => Err("He hasn't been with an NFL club: there's no pro tape of him.".into()),
        _ => Err("There's no film of him.".into()),
    }
}

/// A quick cut-up's hours (lane 9, SK53): twenty minutes.
pub const QUICK: f32 = super::film::session::QUICK_MIN / 60.0;

/// Why you can't watch his film yourself now, if you can't: you watched
/// him today, a cut-up is open in the film room, you're home resting, or
/// the week's film hours are spent.
pub fn film_blocked(w: &World, id: u32) -> Option<String> {
    let you = me(w)?;
    let (kind, _, _) = film_cost(w, id).ok()?;
    let d = w.today;
    if w.scouting.dept.looks.get(&id).is_some_and(|v| v.iter().any(|l| l.who == you && l.date == d && l.kind == kind)) {
        return Some("You watched him today already: the next look is another day's tape.".into());
    }
    if w.scouting.film.open.is_some() {
        return Some("Finish the cut-up you have open in the film room first.".into());
    }
    super::film::time::can_watch(w, super::film::session::Mode::Quick).err()
}

/// You watch his film yourself, today, on your own hours (bug 82): "Watch
/// it quick" in the film room (lane 9, SK53), a cut-up of his snaps for
/// twenty minutes. The look goes in his file under your name, read by your
/// own eye, and what your eye caught moves your own read. The full
/// session, snap by snap, opens from his file in the film room.
pub fn watch_film(w: &mut World, id: u32) -> Result<String, String> {
    readers::seat_staff(w).ok_or("Start a career first.")?;
    film_cost(w, id)?;
    if let Some(why) = film_blocked(w, id) {
        return Err(why);
    }
    let text = super::film::session::open_player(
        w,
        id,
        super::film::tape::Cut::BestOpponent,
        super::film::session::Mode::Quick,
        None,
    )?;
    Ok(format!("{text} {:.1} film hours left this week.", film_left(w)))
}

/// The day's trips: the look goes in the file under the name of who went,
/// by his own eye. A head coach or GM turning up is news (SK13): the press
/// and agents report it, so the interest leaks; a man sent in your place
/// isn't.
fn take_trips(w: &mut World, d: Date) {
    let Some(you) = me(w) else { return };
    let staff = readers::seat_staff(w);
    let todo: Vec<usize> = (0..w.scouting.own.trips.len())
        .filter(|&k| !w.scouting.own.trips[k].done && w.scouting.own.trips[k].date == d)
        .collect();
    for k in todo {
        let t = w.scouting.own.trips[k].clone();
        let who = t.sent.unwrap_or(you);
        let look = t.kind.look();
        let Some(i) = super::place(w, t.id) else {
            w.scouting.own.trips[k].done = true;
            continue;
        };
        super::dept::add(w, t.id, who, look, d);
        // An NFL club's board reads its own file.
        if let Some(Staff::Club(c)) = staff
            && who == you
            && w.nfl.get(c).and_then(|x| x.head_coach) == readers::you(w)
        {
            let lk = match look {
                Kind::ProDay | Kind::Workout => scouts::LookKind::Coach,
                Kind::AllStar => scouts::LookKind::AllStar,
                _ => scouts::LookKind::Coach,
            };
            super::dept::club_look(w, c, t.id, scouts::HC, lk, d);
        }
        if let Some(Staff::Program(_)) = staff
            && matches!(t.kind, TripKind::Game | TripKind::Practice)
        {
            let fall = matches!(d.parts().1, 9..=11);
            if fall {
                w.scouting.dept.eval_days.1 += 1;
            } else {
                w.scouting.dept.eval_days.2 += 1;
            }
            w.scouting.dept.days_log.push((d, who, t.id));
        }
        let mut seen = false;
        if who == you && !matches!(staff, Some(Staff::School(_))) {
            // How visible the trip is: a pro day draws cameras, a Friday
            // night game the local paper, a workout only his agent.
            let p = match t.kind {
                TripKind::ProDay => 0.6,
                TripKind::AllStar => 0.5,
                TripKind::Game => 0.3,
                TripKind::Practice => 0.15,
                TripKind::Workout => 0.35,
                _ => 0.0,
            };
            seen = key_uniform(w.options.seed as u64 ^ 0x5EE7 ^ (d.0 as u64) << 24, t.id as u64) < p;
        }
        w.scouting.own.trips[k].done = true;
        w.scouting.own.trips[k].seen = seen;
        if seen {
            leak(w, i, t.kind, d);
        }
    }
}

/// The press reports you there (SK13). At an NFL club the interest leaks
/// into the other clubs' reads of the market; at a college his stock rises
/// with every staff in the land.
fn leak(w: &mut World, i: usize, kind: TripKind, d: Date) {
    let you = w.career.as_ref().map_or("The coach".to_string(), |c| c.you.name.clone());
    let p = &w.people[i];
    let (pname, id) = (p.name.clone(), p.player.id);
    let place = match kind {
        TripKind::ProDay => "at his pro day",
        TripKind::Workout => "at a private workout for him",
        TripKind::AllStar => "watching him all week at the all-star game",
        TripKind::Practice => "at his practice",
        _ => "at his game",
    };
    match readers::seat_staff(w) {
        Some(Staff::Club(t)) => {
            let text = format!("{you} of the {} was {place}: {pname} ({}).", w.nfl[t].name, p.player.position);
            w.calendar
                .post(Event::new(d, Level::Nfl, CalKind::Whisper, Loudness::Small, text).team(t).person(id, &pname));
            w.scouting.predraft.leaks.push(super::predraft::Leak {
                team: t,
                player: id,
                date: d,
                real: true,
                yours: false,
            });
        }
        Some(Staff::Program(prog)) => {
            let text = format!("{you} of {} was {place}: {pname} ({}).", w.program_name(prog), p.player.position);
            w.calendar.post(
                Event::new(d, Level::Recruiting, CalKind::Whisper, Loudness::Small, text)
                    .program(prog)
                    .person(id, &pname),
            );
        }
        _ => {}
    }
}

/// How much your trips that the press reported raise a recruit's
/// visibility with every staff (SK13): a head coach turning up gets others
/// looking, 0.05 a trip, at most 0.1 (proposed, not confirmed).
pub fn seen_lift(w: &World) -> HashMap<u32, f64> {
    let mut out: HashMap<u32, f64> = HashMap::new();
    let since = w.today.add(-365);
    for t in &w.scouting.own.trips {
        if t.seen && t.date >= since {
            let e = out.entry(t.id).or_default();
            *e = (*e + 0.05).min(0.1);
        }
    }
    out
}

/// SK14: your eye grows from your own looks and how they turned out.
/// Each settled look-back you took part in teaches you something: a
/// read that held up a little, a miss you face up to a little more. Up
/// to 3 points over a career (proposed, not confirmed).
fn credit_eye(w: &mut World) {
    let Some(you) = me(w) else { return };
    let mut gain = 0.0f32;
    let mut credited = Vec::new();
    for g in w.scouting.records.drafts.iter().chain(w.scouting.records.classes.iter()) {
        if g.outcome.is_none() || w.scouting.own.eye_credited.contains(&g.id) {
            continue;
        }
        let mine = w.scouting.dept.looks.get(&g.id).is_some_and(|v| v.iter().any(|l| l.who == you));
        if !mine {
            continue;
        }
        credited.push(g.id);
        gain += match super::lookback::verdict(g) {
            super::lookback::Verdict::Held => 0.04,
            super::lookback::Verdict::TooHigh | super::lookback::Verdict::TooLow => 0.06,
            _ => 0.0,
        };
    }
    if credited.is_empty() {
        return;
    }
    let room = 3.0 - w.scouting.own.eye_gain;
    let gain = gain.min(room.max(0.0));
    w.scouting.own.eye_gain += gain;
    w.scouting.own.eye_credited.extend(credited);
    if let Some(c) = readers::you(w)
        && let Some(y) = w.coaches[c].yours.as_mut()
    {
        y.evaluation = (y.evaluation + gain as f64).min(20.0);
    }
}

// --- SK15: the interview room ----------------------------------------------------------------

/// What you spend an interview's minutes on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Topic {
    /// Teach him a play on the whiteboard, quiz him on it later.
    Whiteboard,
    /// Show him his worst tape.
    WorstTape,
    /// Ask him about an incident.
    Incident,
    /// His family and where he comes from.
    Family,
    /// What football means to him.
    Football,
}

pub const TOPICS: [Topic; 5] = [Topic::Whiteboard, Topic::WorstTape, Topic::Incident, Topic::Family, Topic::Football];

impl Topic {
    pub fn key(self) -> &'static str {
        match self {
            Topic::Whiteboard => "whiteboard",
            Topic::WorstTape => "worst_tape",
            Topic::Incident => "incident",
            Topic::Family => "family",
            Topic::Football => "football",
        }
    }
    pub fn words(self) -> &'static str {
        match self {
            Topic::Whiteboard => "Teach a play on the whiteboard",
            Topic::WorstTape => "Show him his worst tape",
            Topic::Incident => "Ask about an incident",
            Topic::Family => "His family",
            Topic::Football => "What football means to him",
        }
    }
    /// The traits (file numbering) and attributes it reads.
    pub fn reads(self) -> (&'static [usize], &'static [Attr]) {
        match self {
            Topic::Whiteboard => (&[1, 11], &[Attr::FootballIq, Attr::PlayRecognition, Attr::Communication]),
            Topic::WorstTape => (&[1, 8, 12], &[Attr::Composure]),
            Topic::Incident => (&[7, 10], &[]),
            Topic::Family => (&[5, 6, 4], &[]),
            Topic::Football => (&[0, 2, 3, 9], &[Attr::Competitiveness]),
        }
    }
}

/// An interview you ran.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Interview {
    pub id: u32,
    pub date: Date,
    /// The room: "Combine interview", "Top-30 visit", "Official visit".
    pub room: String,
    pub minutes: u8,
    /// Minutes by topic.
    pub spent: Vec<(Topic, u8)>,
    /// His answers, one a topic, in his own voice.
    pub answers: Vec<(Topic, String)>,
    /// The whiteboard quiz later: assignments he drew back of those taught.
    pub quiz: Option<(u8, u8)>,
}

/// The rooms open to you with this player now, and their minutes (SK15):
/// a combine interview (15 minutes, 18 from 2020), a top-30 visit's hour
/// (from the pro days to the visits' deadline), an official visit's
/// sit-down.
pub fn rooms(w: &World, id: u32) -> Vec<(&'static str, u8)> {
    let mut v = Vec::new();
    let Some(i) = super::place(w, id) else { return v };
    let p = &w.people[i];
    match readers::seat_staff(w) {
        Some(Staff::Club(t)) => {
            let at_combine = w.today >= w.dates.combine_start && w.today <= w.dates.combine_end;
            let listed = super::predraft::seat_list(w, t, super::predraft::LIST_INTERVIEW).contains(&id);
            if at_combine && listed && p.stage == Stage::Declared {
                v.push(("Combine interview", if w.year >= 2020 { 18 } else { 15 }));
            }
            // Top-30 visits run from the pro days to their deadline.
            let visit = super::predraft::seat_list(w, t, super::predraft::LIST_VISIT).contains(&id);
            let open = w.today >= w.dates.pro_days.0 && w.today <= w.dates.top30_deadline;
            if visit && p.stage == Stage::Declared && open {
                v.push(("Top-30 visit", 60));
            }
        }
        Some(Staff::Program(prog)) => {
            let on_visit =
                p.cal.visits.iter().any(|x| x.0 == prog && x.2 == crate::world::Visit::Official && x.1 == w.today);
            if on_visit {
                v.push(("Official visit sit-down", 45));
            }
        }
        _ => {}
    }
    v.retain(|r| {
        !w.scouting.own.interviews.iter().any(|x| x.id == id && x.room == r.0 && x.date.year() == w.today.year())
    });
    v
}

/// You run an interview, spending its minutes as you choose (SK15). Each
/// choice reads different things; his answers come in his own voice from
/// the line bank, shaded the way he tells it.
pub fn interview(w: &mut World, id: u32, room: &str, spent: Vec<(Topic, u8)>) -> Result<String, String> {
    let open = rooms(w, id);
    let (room, minutes) = open.into_iter().find(|r| r.0 == room).ok_or("That room isn't open with him today.")?;
    let total: u32 = spent.iter().map(|x| x.1 as u32).sum();
    if total == 0 || total > minutes as u32 {
        return Err(format!("Spend up to {minutes} minutes."));
    }
    let i = super::place(w, id).ok_or("No such player.")?;
    let you = me(w).ok_or("Start a career first.")?;
    let p = &w.people[i];
    let mut answers = Vec::new();
    for &(t, m) in &spent {
        if m == 0 {
            continue;
        }
        answers.push((t, answer_line(w, i, t)));
    }
    // The whiteboard quiz comes at the end of the day: how many of six
    // assignments he draws back, from his football IQ and recognition.
    let quiz = spent.iter().find(|x| x.0 == Topic::Whiteboard && x.1 >= 3).map(|_| {
        let a = &p.player.attributes;
        let skill = 0.6 * a.get(Attr::FootballIq) + 0.4 * a.get(Attr::PlayRecognition);
        let x = (skill - 40.0) / 50.0 + 0.12 * key_normal(0x0B0A ^ w.today.0 as u64, id as u64);
        ((6.0 * clamp(x, 0.0, 1.0)).round() as u8, 6u8)
    });
    let d = w.today;
    super::dept::add(w, id, you, Kind::Interview, d);
    w.scouting.own.interviews.push(Interview { id, date: d, room: room.to_string(), minutes, spent, answers, quiz });
    Ok(format!("{room} with {} is done.", w.people[i].name))
}

/// His answer on a topic, in his own voice from the line bank (SK15; the
/// timed combine talk, lane 14 D8, asks the same way).
pub(crate) fn answer_line(w: &World, i: usize, t: Topic) -> String {
    let p = &w.people[i];
    let trouble = p.player.history.iter().any(|h| {
        h.events.has(crate::player::DevelopmentEvent::Suspended)
            || h.events.has(crate::player::DevelopmentEvent::Dismissed)
    }) || p.incident;
    if t == Topic::Incident && !trouble {
        return "There's nothing on my record, coach. Ask anybody back home.".to_string();
    }
    super::lines::interview_answer(t.key(), answer_band(p, t), p.player.id as u64 ^ (w.today.0 as u64) << 20, t as u64)
}

/// Where his answer on a topic falls (0 worrying, 1 fine, 2 strong), from
/// what is true of him, shaded the way he tells it: a big ego and a good
/// talker polish their answers.
/// How well he tells it: a confident talker who speaks well answers
/// better than he is, never worse (trait points).
fn polish(p: &crate::world::Person) -> f64 {
    (0.15 * (p.player.personality.traits.get(crate::player::Trait::Ego) - 10.0)
        + 0.05 * (p.player.attributes.get(Attr::Communication) - 50.0) / 5.0)
        .max(0.0)
}

fn answer_band(p: &crate::world::Person, t: Topic) -> u8 {
    let (traits, _) = t.reads();
    let mut v = 0.0;
    for &k in traits {
        let x = super::file::trait_value(p, k);
        v += if super::file::trait_good_high(k) { x } else { 21.0 - x };
    }
    v /= traits.len().max(1) as f64;
    let v = v + polish(p);
    if v >= 13.0 {
        2
    } else if v >= 8.5 {
        1
    } else {
        0
    }
}

/// The interview's evidence on his traits for your file (SK15): each topic
/// reads its traits, sharper the more minutes it got and the better your
/// own read of people. Applied after the file's own looks.
pub(crate) fn interview_traits(w: &World, id: u32, traits: &mut [super::file::TraitRead]) {
    let Some(i) = super::place(w, id) else { return };
    let p = &w.people[i];
    let rel = readers::you(w).map_or(10.0, |c| w.coaches[c].ability().relationships);
    for iv in w.scouting.own.interviews.iter().filter(|x| x.id == id) {
        for &(t, m) in &iv.spent {
            if m == 0 {
                continue;
            }
            for &k in t.reads().0 {
                let Some(tr) = traits.get_mut(k) else { continue };
                // He tells it well: the room reads him a little better
                // than he is.
                let shade = if super::file::trait_good_high(k) { polish(p) } else { -polish(p) };
                let truth = super::file::trait_value(p, k) + 0.6 * shade;
                let sd = 2.4 * (6.0 / m as f64).sqrt() * super::file::sharp(rel);
                let v = sd * key_normal(0x1A7E ^ (k as u64) << 40 ^ iv.date.0 as u64, id as u64);
                let (acc, prec) =
                    if tr.seen { ((tr.centre - truth) / (tr.sd * tr.sd), 1.0 / (tr.sd * tr.sd)) } else { (0.0, 0.0) };
                let acc = acc + v / (sd * sd);
                let prec = prec + 1.0 / (sd * sd);
                tr.centre = clamp(truth + acc / prec, 1.0, 20.0);
                tr.sd = (1.0 / prec).sqrt();
                tr.seen = true;
            }
        }
    }
}

// --- SK16: board meetings --------------------------------------------------------------------

/// One player in a board meeting: who presented him, how strongly, the
/// high and low men, and how you settled it.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Item {
    pub id: u32,
    pub name: String,
    pub pos: Position,
    /// The man who presented him (reader code) and his conviction (1-20;
    /// 0 when his staff keeps none).
    pub by: u32,
    pub by_name: String,
    pub conviction: u8,
    /// The high and low men's reads (staff ranges: low, high on the
    /// position scale) and who they are.
    pub high: (u32, String, u8, u8),
    pub low: (u32, String, u8, u8),
    /// 0 not settled, 1 with the high man, 2 with the low man, 3 between.
    pub settled: u8,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Meeting {
    pub date: Date,
    pub kind: String,
    pub items: Vec<Item>,
}

/// The meetings' dates: the NFL's February meetings after the all-star
/// games and April's final stack; a college staff's board meetings after
/// the spring evaluation period, before the season and before the early
/// signing period.
fn meeting_today(w: &World, d: Date) -> Option<&'static str> {
    let (y, m, day) = d.parts();
    match readers::seat_staff(w)? {
        Staff::Club(_) => {
            if d == Date::nth_weekday(y, 2, Weekday::Mon, 1) {
                Some("February draft meetings")
            } else if m == 4 && day == 15 {
                Some("April's final stack")
            } else {
                None
            }
        }
        Staff::Program(_) => {
            if (m == 6 && day == 1) || (m == 8 && day == 1) || (m == 12 && day == 1) {
                Some("Recruiting board meeting")
            } else {
                None
            }
        }
        Staff::School(_) => None,
    }
}

/// A reader's overall read of a player, from his own reads of each
/// attribute and the staff's where he saw nothing.
pub(crate) fn reader_overall(w: &World, i: usize, f: &super::file::FileRead, r: usize) -> Option<(f64, f64)> {
    let p = &w.people[i];
    let pos = p.player.position;
    let row = f.per_reader.get(r)?;
    if row.iter().filter(|x| x.is_some()).count() < 6 {
        return None;
    }
    let mut c = p.player.attributes;
    let mut sd2 = 0.0;
    for a in Attr::ALL {
        let k = a.index();
        let (v, sd) = row[k].unwrap_or((f.attrs[k].centre, f.attrs[k].sd.max(4.0)));
        c.set(a, v);
        let wt = crate::positions::weight(pos, a);
        sd2 += wt * wt * sd * sd;
    }
    Some((overall_for(pos, &c), sd2.sqrt().max(1.5)))
}

/// The installed systems of the staff you lead, when it has any.
fn seat_systems(w: &World) -> Option<&crate::world::systems::Installed> {
    match readers::seat_staff(w)? {
        Staff::Club(t) => w.clubs.get(t).map(|c| &c.installed),
        Staff::Program(p) => w.programs.get(p).map(|c| &c.installed),
        Staff::School(_) => None,
    }
}

/// What one reader says about a player in your room (bug 18), from his
/// own read: where he has him against the room, his looks, the tier and
/// how he fits what you run. None when he read too little of him.
pub fn voice(
    w: &World,
    i: usize,
    f: &super::file::FileRead,
    rds: &[super::file::Reader],
    looks: &[super::file::Evidence],
    r: usize,
    speak: super::file::Speak,
) -> Option<String> {
    let (c, sd) = reader_overall(w, i, f, r)?;
    let p = &w.people[i];
    let row = &f.per_reader[r];
    let n = looks.iter().filter(|l| l.reader == r).count();
    let numbers = looks.iter().filter(|l| l.reader == r).all(|l| l.kind == Kind::Numbers);
    let lo = (c - sd).floor().clamp(1.0, 99.0) as u8;
    let hi = (c + sd).ceil().clamp(1.0, 99.0) as u8;
    let tier = super::file::tier_words(speak, f.ceiling.0 + (c - f.now.0));
    // Fit: his read of what the system asks against his read of the rest
    // that matters at the spot.
    let fit = seat_systems(w).and_then(|inst| {
        let asks = super::footage::scheme_asks(inst, p.player.position);
        let mine: Vec<f64> = asks.iter().filter_map(|a| row[a.index()].map(|x| x.0)).collect();
        if mine.is_empty() {
            return None;
        }
        let rest: Vec<f64> = Attr::ALL
            .iter()
            .filter(|a| !asks.contains(a) && super::file::matters(p.player.position, **a) > 0.4)
            .filter_map(|a| row[a.index()].map(|x| x.0))
            .collect();
        let avg = |v: &[f64]| v.iter().sum::<f64>() / v.len().max(1) as f64;
        (!rest.is_empty()).then(|| avg(&mine) - avg(&rest))
    });
    let v = super::lines::Voice { name: &p.name, read: (lo, hi), room: f.now.0, looks: n, tier, fit, numbers };
    Some(super::lines::voice(&v, rds[r].key, p.player.id as u64))
}

/// Holds the day's meeting (SK16): the top of your board, each player
/// presented by the man who saw him most, with the high and low men where
/// your room disagrees by a tier or more.
fn hold_meeting(w: &mut World, d: Date, kind: &str) {
    let Some(staff) = readers::seat_staff(w) else { return };
    let ids: Vec<u32> = match staff {
        Staff::Club(_) => {
            let g = super::board::seat_grades(w);
            super::board::ordered(w, &g).into_iter().take(60).collect()
        }
        Staff::Program(prog) => {
            let mut v: Vec<u32> =
                w.front_office.college.board.iter().filter(|t| t.pulled.is_none()).map(|t| t.id).collect();
            if v.is_empty() {
                v = super::dept::watch(w, prog).into_iter().take(60).map(|i| w.people[i].player.id).collect();
            }
            v.truncate(60);
            v
        }
        Staff::School(_) => return,
    };
    let mut items = Vec::new();
    for id in ids {
        let Some((rds, looks, f)) = readers::read(w, staff, id) else { continue };
        let Some(i) = super::place(w, id) else { continue };
        let mut per: Vec<(usize, f64, f64, usize)> = Vec::new();
        for r in 0..rds.len() {
            if rds[r].link == super::file::Link::Event {
                continue;
            }
            let n = looks.iter().filter(|l| l.reader == r).count();
            if n < 2 {
                continue;
            }
            if let Some((c, sd)) = reader_overall(w, i, &f, r) {
                per.push((r, c, sd, n));
            }
        }
        if per.len() < 2 {
            continue;
        }
        let hi = per.iter().copied().max_by(|a, b| a.1.total_cmp(&b.1)).unwrap();
        let lo = per.iter().copied().min_by(|a, b| a.1.total_cmp(&b.1)).unwrap();
        if hi.1 - lo.1 < 4.0 {
            continue;
        }
        let by = per.iter().copied().max_by_key(|x| x.3).unwrap();
        let codes = readers::evidence(w, staff, id).2;
        let code = |r: usize| codes.get(r).copied().unwrap_or(0);
        let conviction = match staff {
            Staff::Club(t) => w.clubs[t].scouts.get(code(by.0) as usize).map_or(0, |s| s.conviction),
            _ => 0,
        };
        let range =
            |c: f64, sd: f64| (((c - sd).floor()).clamp(1.0, 99.0) as u8, ((c + sd).ceil()).clamp(1.0, 99.0) as u8);
        let (hl, hh) = range(hi.1, hi.2);
        let (ll, lh) = range(lo.1, lo.2);
        let p = &w.people[i];
        items.push(Item {
            id,
            name: p.name.clone(),
            pos: p.player.position,
            by: code(by.0),
            by_name: rds[by.0].name.clone(),
            conviction,
            high: (code(hi.0), rds[hi.0].name.clone(), hl, hh),
            low: (code(lo.0), rds[lo.0].name.clone(), ll, lh),
            settled: 0,
        });
        // An NFL room goes down its board; a college staff's board isn't
        // ranked, so it takes the twelve it's highest on.
        if matches!(staff, Staff::Club(_)) && items.len() >= 12 {
            break;
        }
    }
    if !matches!(staff, Staff::Club(_)) {
        items.sort_by(|a, b| (b.high.3, b.high.2, a.id).cmp(&(a.high.3, a.high.2, b.id)));
        items.truncate(12);
    }
    let n = items.len();
    w.scouting.own.meetings.push(Meeting { date: d, kind: kind.to_string(), items });
    if w.scouting.own.meetings.len() > 12 {
        w.scouting.own.meetings.remove(0);
    }
    let text = if n > 0 {
        let who = if n == 1 { "one player".to_string() } else { format!("{n} players") };
        format!("{kind}: your room splits on {who}. Settle them under Evaluation, Meetings.")
    } else {
        format!("{kind}: your room agrees on the top of the board.")
    };
    let ev = Event::new(d, Level::Nfl, CalKind::DraftChoice, Loudness::Small, text).rule("Board meeting");
    let ev = match staff {
        Staff::Club(t) => ev.team(t).decision(),
        Staff::Program(p) => Event { level: Level::Recruiting, ..ev }.program(p).decision(),
        _ => ev,
    };
    w.calendar.post(ev);
}

/// You settle a disagreement (SK16): with the high man, the low man, or
/// between. Your board follows: the player moves up or down by how far
/// apart they were.
pub fn settle(w: &mut World, meeting: usize, id: u32, side: u8) -> Result<String, String> {
    if !(1..=3).contains(&side) {
        return Err("Side with the high man, the low man, or settle between.".into());
    }
    let staff = readers::seat_staff(w).ok_or("Start a career first.")?;
    let m = w.scouting.own.meetings.get_mut(meeting).ok_or("No such meeting.")?;
    let it = m.items.iter_mut().find(|x| x.id == id).ok_or("He wasn't discussed.")?;
    if it.settled != 0 {
        return Err("Already settled.".into());
    }
    it.settled = side;
    let gap = (it.high.2 as f64 + it.high.3 as f64 - it.low.2 as f64 - it.low.3 as f64) / 2.0;
    let name = it.name.clone();
    // Places to move: about three a point of the gap on a draft board,
    // two on a recruiting board.
    let shift = match side {
        1 => gap / 2.0,
        2 => -gap / 2.0,
        _ => 0.0,
    };
    match staff {
        Staff::Club(_) => {
            let g = super::board::seat_grades(w);
            let mut order = super::board::ordered(w, &g);
            if let Some(k) = order.iter().position(|&x| x == id) {
                order.remove(k);
                let to = (k as f64 - 3.0 * shift).round().clamp(0.0, order.len() as f64) as usize;
                order.insert(to, id);
                let year = super::public::draft_year(w);
                w.scouting.board.for_year(year);
                w.scouting.board.order = order;
            }
        }
        Staff::Program(_) => {
            if let Some(k) = w.front_office.college.board.iter().position(|t| t.id == id) {
                let to = (k as f64 - 2.0 * shift).round().max(0.0) as usize;
                let _ = crate::world::front_office::college::reorder(w, id, to);
            }
        }
        _ => {}
    }
    Ok(format!(
        "{name}: {}.",
        match side {
            1 => "you side with the high man",
            2 => "you side with the low man",
            _ => "you settle between them",
        }
    ))
}

/// The record of who was right in your meetings (SK16), once each class
/// has its look-back: (reader code) -> (right, wrong).
pub fn meeting_record(w: &World) -> HashMap<u32, (u32, u32)> {
    let mut out: HashMap<u32, (u32, u32)> = HashMap::new();
    for m in &w.scouting.own.meetings {
        for it in &m.items {
            let g = w.scouting.records.drafts.iter().chain(w.scouting.records.classes.iter()).find(|g| g.id == it.id);
            let Some(g) = g else { continue };
            let v = super::lookback::verdict(g);
            let (hi_right, lo_right) = match v {
                super::lookback::Verdict::TooLow => (true, false),
                super::lookback::Verdict::TooHigh => (false, true),
                super::lookback::Verdict::Held => (false, false),
                _ => continue,
            };
            let e = out.entry(it.high.0).or_default();
            if hi_right {
                e.0 += 1
            } else {
                e.1 += 1
            }
            let e = out.entry(it.low.0).or_default();
            if lo_right { e.0 += 1 } else { e.1 += 1 }
        }
    }
    out
}

// --- SK17: hard minimums ----------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Measure {
    Height,
    Weight,
    Arm,
    Hand,
    Forty,
}

pub const MEASURES: [Measure; 5] = [Measure::Height, Measure::Weight, Measure::Arm, Measure::Hand, Measure::Forty];

impl Measure {
    pub fn key(self) -> &'static str {
        match self {
            Measure::Height => "height",
            Measure::Weight => "weight",
            Measure::Arm => "arm",
            Measure::Hand => "hand",
            Measure::Forty => "forty",
        }
    }
    pub fn words(self) -> &'static str {
        match self {
            Measure::Height => "Height (in)",
            Measure::Weight => "Weight (lb)",
            Measure::Arm => "Arm length (in)",
            Measure::Hand => "Hand size (in)",
            Measure::Forty => "40-yard dash (s, at most)",
        }
    }
    pub fn from_key(k: &str) -> Option<Measure> {
        MEASURES.into_iter().find(|m| m.key() == k)
    }
}

/// One hard minimum: a position, a measure and the line; drop takes him
/// off the board, else he's flagged.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Minimum {
    pub pos: Position,
    pub measure: Measure,
    pub value: f64,
    pub drop: bool,
}

/// What your file has on a measure: measured at the combine, a pro day or
/// your camp, else his school's listing for height and weight, else
/// nothing yet.
pub fn measured(w: &World, i: usize, m: Measure) -> Option<(f64, bool)> {
    let p = &w.people[i];
    let id = p.player.id;
    let looks = w.scouting.dept.looks.get(&id);
    let tested = matches!(p.stage, Stage::Declared | Stage::Pro)
        || looks.is_some_and(|v| {
            v.iter().any(|l| matches!(l.kind, Kind::Combine | Kind::ProDay | Kind::Camp | Kind::Workout))
        });
    let b = &p.player.body;
    match m {
        Measure::Height => Some(if tested { (b.height, true) } else { (super::file::listed(&p.player).0, false) }),
        Measure::Weight => Some(if tested { (b.weight, true) } else { (super::file::listed(&p.player).1, false) }),
        Measure::Arm => tested.then_some((b.arm, true)),
        Measure::Hand => tested.then_some((b.hand, true)),
        Measure::Forty => tested.then(|| (super::file::tests(&p.player, Kind::Combine, w.year).forty, true)),
    }
}

/// Whether he falls under a minimum, and which.
pub fn under(w: &World, i: usize, mins: &[Minimum]) -> Option<Minimum> {
    let pos = w.people[i].player.position;
    mins.iter()
        .filter(|m| m.pos == pos)
        .find(|m| match measured(w, i, m.measure) {
            Some((v, _)) if m.measure == Measure::Forty => v > m.value,
            Some((v, _)) => v < m.value,
            None => false,
        })
        .copied()
}

/// You set one of your minimums (SK17), replacing one on the same spot
/// and measure; `value` None takes it off.
pub fn set_minimum(
    w: &mut World,
    pos: Position,
    measure: Measure,
    value: Option<f64>,
    drop: bool,
) -> Result<String, String> {
    readers::seat_staff(w)
        .filter(|s| !matches!(s, Staff::School(_)))
        .ok_or("Minimums are for an NFL or college staff.")?;
    let mins = &mut w.scouting.own.minimums;
    mins.retain(|m| !(m.pos == pos && m.measure == measure));
    let Some(v) = value else { return Ok(format!("No {} line at {}.", measure.words().to_lowercase(), pos.name())) };
    let (lo, hi) = match measure {
        Measure::Height => (66.0, 82.0),
        Measure::Weight => (150.0, 360.0),
        Measure::Arm => (28.0, 37.0),
        Measure::Hand => (8.0, 11.5),
        Measure::Forty => (4.2, 5.6),
    };
    if !(lo..=hi).contains(&v) {
        return Err(format!("Set a line between {lo} and {hi}."));
    }
    mins.push(Minimum { pos, measure, value: v, drop });
    Ok(format!(
        "{} at {}: {v}, {}.",
        measure.words(),
        pos.name(),
        if drop { "off the board under it" } else { "flagged under it" }
    ))
}

/// An AI GM's or head coach's own minimums (SK17), by key: about a third
/// hold one or two (Seattle's 32-inch arms at corner; a 6-foot-2 line at
/// quarterback; 33-inch arms at tackle; proposed, not confirmed).
pub fn ai_minimums(key: u64) -> Vec<Minimum> {
    let mut v = Vec::new();
    let pool = [
        Minimum { pos: Position::CB, measure: Measure::Arm, value: 32.0, drop: true },
        Minimum { pos: Position::QB, measure: Measure::Height, value: 74.0, drop: true },
        Minimum { pos: Position::OT, measure: Measure::Arm, value: 33.0, drop: true },
        Minimum { pos: Position::EDGE, measure: Measure::Arm, value: 33.0, drop: true },
        Minimum { pos: Position::QB, measure: Measure::Hand, value: 9.0, drop: true },
        Minimum { pos: Position::WR, measure: Measure::Forty, value: 4.6, drop: true },
    ];
    let u = key_uniform(key ^ 0x3171, 1);
    if u < 0.34 {
        v.push(pool[(key_uniform(key ^ 0x3172, 2) * pool.len() as f64) as usize % pool.len()]);
        if u < 0.12 {
            let b = pool[(key_uniform(key ^ 0x3173, 3) * pool.len() as f64) as usize % pool.len()];
            if !v.iter().any(|m| m.pos == b.pos && m.measure == b.measure) {
                v.push(b);
            }
        }
    }
    v
}

// --- SK18: the scouting services --------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Service {
    /// Not decided: the club's long-standing membership.
    #[default]
    Unset,
    None,
    National,
    Blesto,
}

impl Service {
    pub fn key(self) -> &'static str {
        match self {
            Service::Unset => "unset",
            Service::None => "none",
            Service::National => "national",
            Service::Blesto => "blesto",
        }
    }
    pub fn words(self) -> &'static str {
        match self {
            Service::Unset | Service::None => "No service",
            Service::National => "National Football Scouting",
            Service::Blesto => "BLESTO",
        }
    }
    pub fn from_key(k: &str) -> Option<Service> {
        [Service::None, Service::National, Service::Blesto].into_iter().find(|s| s.key() == k)
    }
}

/// A club's service (SK18): about two-thirds of clubs belong to National or
/// BLESTO, a club's membership holding for decades (keyed by club and
/// decade). Yours is your call once you've made it.
pub fn service_of(w: &World, t: usize) -> Service {
    if w.seat.and_then(|s| s.nfl_team) == Some(t) && w.scouting.own.service != Service::Unset {
        return w.scouting.own.service;
    }
    let u = key_uniform(w.options.seed as u64 ^ 0x5E4C ^ ((w.year / 10) as u64) << 8, t as u64);
    if u < 0.36 {
        Service::National
    } else if u < 0.66 {
        Service::Blesto
    } else {
        Service::None
    }
}

/// The yearly fee, in dollars (real: over $100,000 in the 2010s).
pub fn service_fee(year: i32) -> f64 {
    120_000.0 * clamp(0.4 + 0.6 * (year - 1985) as f64 / 35.0, 0.4, 1.3)
}

/// A service's spring grade of a rising senior (SK18), on its scale of 1.0
/// to 9.0 (7.0 and up about round one): its own scouts' read of his tape,
/// with their own lasting lean and the tape's shared lean.
pub fn service_grade(w: &World, s: Service, i: usize) -> Option<f64> {
    let p = &w.people[i];
    if p.stage != Stage::College || s == Service::None || s == Service::Unset {
        return None;
    }
    let pos = p.cal.pro_position.unwrap_or(p.player.position);
    let key = match s {
        Service::National => 0x4A71u64,
        _ => 0xB1E5,
    };
    let now = p.player.overall();
    let ceiling = overall_for(pos, &p.player.ceilings).max(now);
    let read = now + 0.3 * (ceiling - now) + 4.0 * key_normal(key ^ w.options.seed as u64, p.player.id as u64);
    let f = super::footage::footage(w, i);
    let read = read + super::footage::scout_tilt(&f, 11.0, false, false);
    let disc = match p.college.map(|c| w.programs[c].division) {
        Some(crate::world::data::Division::FCS) => 1.0,
        Some(crate::world::data::Division::FBS) | None => 0.0,
        _ => 5.0,
    };
    Some((clamp(7.0 + (read - disc - 78.0) / 4.0, 1.0, 9.0) * 10.0).round() / 10.0)
}

/// You choose a service; it takes effect after this draft (SK18).
pub fn set_service(w: &mut World, s: Service) -> Result<String, String> {
    w.seat.and_then(|x| x.nfl_team).ok_or("Only NFL clubs belong to the services.")?;
    w.scouting.own.service_next = Some(s);
    Ok(format!("From after the draft: {}.", s.words()))
}

// --- SK19: NFL scouts on your campus -----------------------------------------------------------

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Word {
    pub club: u8,
    pub id: u32,
    /// 0 worth a look, 1 can play for you, 2 a sure thing.
    pub strength: u8,
    pub date: Date,
    /// Settled after his draft: your word held up.
    pub held: Option<bool>,
}

/// An NFL club's credit for your word (0-1): your record with it, or where
/// your network puts you.
pub fn cred(w: &World, club: usize) -> f64 {
    if let Some(&c) = w.scouting.own.cred.get(&(club as u8)) {
        return c as f64;
    }
    let net = readers::you(w).map_or(10.0, |c| w.coaches[c].ability().network);
    (0.35 + 0.02 * (net - 10.0)).clamp(0.1, 0.7)
}

/// You tell an NFL club about one of your players (SK19).
pub fn tell(w: &mut World, club: usize, id: u32, strength: u8) -> Result<String, String> {
    let Some(Staff::Program(prog)) = readers::seat_staff(w) else {
        return Err("Only a college coach talks to NFL scouts about his players.".into());
    };
    let i = super::place(w, id).ok_or("No such player.")?;
    if w.people[i].college != Some(prog) || !matches!(w.people[i].stage, Stage::College | Stage::Declared) {
        return Err("Only your own players.".into());
    }
    if club >= w.nfl.len() {
        return Err("No such club.".into());
    }
    let year = w.year;
    if w.scouting.own.words.iter().any(|x| x.club as usize == club && x.id == id && x.date.year() == year) {
        return Err("You have already given that club your word on him this year.".into());
    }
    let strength = strength.min(2);
    let d = w.today;
    w.scouting.own.words.push(Word { club: club as u8, id, strength, date: d, held: None });
    let words = ["worth a look", "can play for you", "a sure thing"][strength as usize];
    Ok(format!("You told the {}: {} is {words}.", w.nfl[club].name, w.people[i].name))
}

/// How much your word moves a club's read of a player (SK19), in grade
/// points: up to about two for a sure thing from a coach it trusts.
pub fn word_lift(w: &World, club: usize, id: u32) -> f64 {
    let since = w.today.add(-400);
    w.scouting
        .own
        .words
        .iter()
        .filter(|x| x.club as usize == club && x.id == id && x.date >= since && x.held.is_none())
        .map(|x| (0.5 + 0.6 * x.strength as f64) * cred(w, club))
        .fold(0.0, f64::max)
}

/// After the draft: each word you gave is settled by where he went (SK19).
/// A sure thing held if he went in the first two rounds, "can play" in the
/// first four, "worth a look" if he was drafted or signed.
pub(crate) fn settle_words(w: &mut World) {
    let year = w.year;
    let mut moves: Vec<(u8, f64)> = Vec::new();
    for k in 0..w.scouting.own.words.len() {
        let x = w.scouting.own.words[k].clone();
        if x.held.is_some() {
            continue;
        }
        let Some(i) = super::place(w, x.id) else { continue };
        let p = &w.people[i];
        if p.stage != Stage::Pro && p.stage != Stage::Done {
            continue;
        }
        let round = p.pro.as_ref().and_then(|c| c.draft).filter(|d| d.0 == year).map(|d| d.1);
        let signed = p.stage == Stage::Pro;
        let held = match x.strength {
            2 => round.is_some_and(|r| r <= 2),
            1 => round.is_some_and(|r| r <= 4),
            _ => signed,
        };
        let size = 0.04 + 0.04 * x.strength as f64;
        moves.push((x.club, if held { size } else { -size - 0.03 * x.strength as f64 }));
        w.scouting.own.words[k].held = Some(held);
    }
    for (club, d) in moves {
        let c = cred(w, club as usize);
        w.scouting.own.cred.insert(club, clamp(c + d, 0.05, 0.95) as f32);
    }
}

/// You set how open your practices are to NFL scouts (SK19).
pub fn set_openness(w: &mut World, level: u8) -> Result<String, String> {
    let Some(Staff::Program(_)) = readers::seat_staff(w) else {
        return Err("Only a college opens its practices to NFL scouts.".into());
    };
    let level = level.min(2);
    w.scouting.own.openness = level;
    Ok([
        "Practice is closed to NFL scouts.",
        "Practice is open to NFL scouts.",
        "Practice and the film room are open to NFL scouts.",
    ][level as usize]
        .into())
}

/// How open your practices are moves what a school call sees (SK19):
/// closed practice hides more, an open film room shows more.
pub fn openness_factor(w: &World, prog: Option<usize>) -> f64 {
    let mine = readers::seat_staff(w).and_then(|s| if let Staff::Program(p) = s { Some(p) } else { None });
    if prog.is_none() || prog != mine {
        return 1.0;
    }
    match w.scouting.own.openness {
        0 => 1.15,
        2 => 0.9,
        _ => 1.0,
    }
}

// --- SK20: pro days ---------------------------------------------------------------------------

/// Every program's pro day as it happened (SK20): the year, how many clubs
/// sent a scout, and how many sent a head coach, coordinator or GM. Its
/// pull in recruiting reads it.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ProDayCount {
    /// The program's id.
    pub program: u16,
    pub year: i16,
    pub clubs: u8,
    pub heads: u8,
    pub tested: u8,
    /// At your own pro day: the clubs that sent a head coach, coordinator
    /// or GM, and the players who worked out (ids).
    #[serde(default)]
    pub head_clubs: Vec<u8>,
    #[serde(default)]
    pub players: Vec<u32>,
}

/// A pro day's pull with recruits (SK20): a program whose pro day draws
/// the league's head men sells it, up to 0.03 appeal points for one that
/// drew twenty or more (proposed, not confirmed). Last two springs.
pub fn pro_day_pull(w: &World, prog: usize) -> f64 {
    let y = w.year as i16;
    w.scouting
        .pro_days
        .iter()
        .filter(|x| x.program as usize == prog && x.year >= y - 2)
        .map(|x| 0.03 * clamp((x.clubs as f64 + 2.0 * x.heads as f64) / 40.0, 0.0, 1.0))
        .fold(0.0, f64::max)
}

// --- SK21: the advisory grades and your stay-or-go talks ---------------------------------------

/// The advisory committee's rules by era (SK21): from 2014 five requests a
/// school (more only by exception, for a likely first-rounder) and three
/// answers; before that any underclassman could ask and got a round.
pub fn advisory_limit(year: i32) -> Option<usize> {
    (year >= 2014).then_some(5)
}

/// You send a junior's name to the committee, or take it back.
pub fn request_advisory(w: &mut World, id: u32, on: bool) -> Result<String, String> {
    let Some(Staff::Program(prog)) = readers::seat_staff(w) else {
        return Err("Only a college coach sends names to the committee.".into());
    };
    let i = super::place(w, id).ok_or("No such player.")?;
    let p = &w.people[i];
    if p.college != Some(prog) || p.stage != Stage::College {
        return Err("Only your own underclassmen.".into());
    }
    let year = w.year;
    if w.today > Date::ymd(year, 12, 10) && w.today.parts().1 == 12 {
        return Err("The committee's requests closed on December 10.".into());
    }
    let name = p.name.clone();
    let exception = p.cal.draft_read.is_some_and(|r| r.round <= 1);
    let list = &mut w.scouting.own.advisory;
    list.retain(|x| x.1 == year || x.1 == year - 1);
    if !on {
        list.retain(|x| !(x.0 == id && x.1 == year));
        return Ok(format!("{name} comes off your advisory list."));
    }
    if list.iter().any(|x| x.0 == id && x.1 == year) {
        return Ok(format!("{name} is on your list."));
    }
    let n = list.iter().filter(|x| x.1 == year).count();
    if let Some(cap) = advisory_limit(year)
        && n >= cap
        && !exception
    {
        return Err(format!(
            "Five names a school since 2014: more only by exception, for a likely first-rounder ({name} isn't one)."
        ));
    }
    list.push((id, year));
    Ok(format!("{name} goes to the committee."))
}

/// Your stay-or-go talk with a junior (SK21): stay a year, your call, or go.
pub fn stay_or_go(w: &mut World, id: u32, say: u8) -> Result<String, String> {
    let Some(Staff::Program(prog)) = readers::seat_staff(w) else {
        return Err("Only his college coach has this talk.".into());
    };
    let i = super::place(w, id).ok_or("No such player.")?;
    let p = &w.people[i];
    if p.college != Some(prog) || p.stage != Stage::College || p.cal.draft_read.is_none() {
        return Err("Only your underclassmen with an NFL read.".into());
    }
    let year = w.year;
    let say = say.min(2);
    w.scouting.own.talks.retain(|x| !(x.0 == id && x.1 == year));
    w.scouting.own.talks.push((id, year, say));
    // A plan made after the committee's grades is weighed again now.
    let factor = talk_factor(w, i, say);
    let p = &mut w.people[i];
    let name = p.name.clone();
    if let Some(plan) = p.cal.declare_plan {
        let flip = match (plan, say) {
            (true, 0) => p.rng.chance(1.0 - factor),
            (false, 2) => p.rng.chance((factor - 1.0).max(0.0)),
            _ => false,
        };
        if flip {
            p.cal.declare_plan = Some(!plan);
        }
    }
    Ok(match say {
        0 => format!("You told {name} another year would make him."),
        1 => format!("You told {name} it's his call and you'll back him."),
        _ => format!("You told {name} he's ready."),
    })
}

/// How your talk moves his chance to declare: staying sways him by how
/// far he trusts you (your relationships, his coachability and loyalty);
/// "go" pushes a little (proposed, not confirmed: at most a 45% cut).
pub fn talk_factor(w: &World, i: usize, say: u8) -> f64 {
    let p = &w.people[i];
    let rel = readers::you(w).map_or(10.0, |c| w.coaches[c].ability().relationships);
    let per = &p.player.personality;
    let trust = clamp(
        0.35 + 0.02 * (rel - 10.0)
            + 0.015 * (per.coachability - 10.0)
            + 0.015 * (per.traits.get(crate::player::Trait::Loyalty) - 10.0),
        0.05,
        0.75,
    );
    match say {
        0 => 1.0 - 0.6 * trust,
        2 => 1.0 + 0.2 * trust,
        _ => 1.0,
    }
}

/// Your talk with a junior this year, if any.
pub fn talk_of(w: &World, id: u32) -> Option<u8> {
    let year = w.year;
    w.scouting.own.talks.iter().find(|x| x.0 == id && x.1 == year).map(|x| x.2)
}

// --- SK22: the athletes in your building ----------------------------------------------------

/// A sport and the football spot it feeds.
pub const SPORTS: [(&str, Position, &[Attr]); 6] = [
    ("Soccer", Position::K, &[Attr::KickPower, Attr::ShortKickAccuracy, Attr::Stamina]),
    ("Basketball", Position::TE, &[Attr::Jumping, Attr::BodyControl, Attr::Catching]),
    ("Wrestling", Position::IOL, &[Attr::Strength, Attr::Balance, Attr::HandUse]),
    ("Track (sprints)", Position::CB, &[Attr::Speed, Attr::Acceleration]),
    ("Baseball", Position::QB, &[Attr::ArmStrength, Attr::QuickRelease, Attr::BallTracking]),
    ("Track (throws)", Position::DT, &[Attr::Strength, Attr::Flexibility]),
];

/// An athlete in your building who doesn't play football: drawn by key from
/// your school and the year, never stored until you talk to him.
#[derive(Clone, Debug)]
pub struct Athlete {
    pub key: u64,
    pub name: String,
    pub sport: usize,
    pub pos: Position,
    pub class_of: i32,
    pub ethnicity: crate::world::ethnicity::Ethnicity,
    /// The athlete he truly is (hidden; the screens show only `tools`).
    pub truth: f64,
    /// Your staff's read of the athlete he is (a range on the 1-20 tools
    /// scale) and how much he loves his own sport (words).
    pub tools: (u8, u8),
    pub spotted_by: String,
    /// He's open to it: his own pull to his sport against football.
    pub open: f64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AthleteTalk {
    pub key: u64,
    pub date: Date,
    pub yes: bool,
    /// The player he became, if he said yes.
    pub id: Option<u32>,
    /// His name and his sport, for a boy who stayed with it.
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub sport: String,
}

/// The athletes your staff has spotted this year (SK22): each school has a
/// handful by its size; your staff spots the better ones, more with a
/// sharper eye.
pub fn athletes(w: &World) -> Vec<Athlete> {
    let Some(Staff::School(h)) = readers::seat_staff(w) else { return Vec::new() };
    let eye = readers::you(w).map_or(10.0, |c| w.coaches[c].evaluation());
    let by = readers::you(w).map_or("You".into(), |c| w.coaches[c].name.clone());
    spotted(w, h, eye, &by).into_iter().filter(|a| !w.scouting.own.athletes.iter().any(|t| t.key == a.key)).collect()
}

/// The athletes a school's staff spots this year, by key.
fn spotted(w: &World, h: usize, eye: f64, by: &str) -> Vec<Athlete> {
    let school = &w.high_schools[h];
    let season = w.dates.season;
    let n = (2.0 + school.enrollment as f64 / 450.0).round().clamp(2.0, 7.0) as usize;
    let mut out = Vec::new();
    for k in 0..n {
        let key = (h as u64) << 32 ^ (season as u64) << 8 ^ k as u64 ^ 0xA7F1;
        let u = |s: u64| key_uniform(key ^ s, 7);
        let tools_true = 8.0 + 9.0 * u(1);
        // The staff spots him if his tools show (better ones more often).
        if u(2) > clamp(0.25 + 0.03 * (eye - 10.0) + 0.04 * (tools_true - 10.0), 0.05, 0.9) {
            continue;
        }
        let sport = (u(3) * SPORTS.len() as f64) as usize % SPORTS.len();
        let mut rng = Rng::new(key as u32 ^ (key >> 32) as u32);
        let eth = crate::world::ethnicity::player_ethnicity(school.state, 0.0, None, &mut rng);
        let grade = (u(4) * 3.0) as i32; // 0 sophomore .. 2 senior
        let class_of = season + 3 - grade;
        let name = crate::world::names::person_name(eth, season - 15 - grade, &mut rng);
        let half = 1.0 + (20.0 - eye) * 0.12;
        let c = tools_true + half * 0.6 * key_normal(key ^ 0x3E, 1);
        out.push(Athlete {
            key,
            name,
            sport,
            pos: SPORTS[sport].1,
            class_of,
            ethnicity: eth,
            truth: tools_true,
            tools: (clamp(c - half, 1.0, 20.0).floor() as u8, clamp(c + half, 1.0, 20.0).ceil() as u8),
            spotted_by: by.to_string(),
            open: clamp(0.2 + 0.5 * u(5), 0.0, 1.0),
        });
    }
    out
}

/// Every other school's coach works his own hallways the same way (SK22):
/// each August 1 an AI head coach talks to the best athlete his staff
/// spots when he would start (tools 14 and up), on the same keyed draws
/// and the same yes as yours. Scaled with the world, like the classes.
pub(crate) fn ai_athletes(w: &mut World) {
    let season = w.dates.season;
    let mine = match readers::seat_staff(w) {
        Some(Staff::School(h)) => Some(h),
        _ => None,
    };
    let scale = w.options.scale;
    let mut adds: Vec<(usize, Athlete)> = Vec::new();
    for h in 0..w.high_schools.len() {
        if Some(h) == mine || key_uniform(0xA7F0 ^ (h as u64) << 20, season as u64) >= scale {
            continue;
        }
        let eye = w.high_schools[h].coach.map_or(8.0, |c| w.coaches[c].evaluation());
        let best = spotted(w, h, eye, "Your staff")
            .into_iter()
            .filter(|a| a.truth >= 14.0)
            .max_by(|a, b| a.truth.total_cmp(&b.truth));
        let Some(a) = best else { continue };
        let rel = w.high_schools[h].coach.map_or(10.0, |c| w.coaches[c].ability().relationships);
        if key_uniform(a.key ^ 0x5A7, season as u64) < yes_chance(&a, rel, season) {
            adds.push((h, a));
        }
    }
    for (h, a) in adds {
        w.add_athlete(h, &a.name, a.ethnicity, a.class_of, a.key, a.pos, SPORTS[a.sport].2, (a.truth - 10.0) / 3.0);
    }
}

/// The chance he says yes: his own openness, the coach's way with people,
/// a senior rarely.
fn yes_chance(a: &Athlete, rel: f64, season: i32) -> f64 {
    let senior = a.class_of == season + 1;
    clamp(a.open + 0.025 * (rel - 10.0) - if senior { 0.1 } else { 0.0 }, 0.05, 0.9)
}

/// You talk an athlete into coming out (SK22). Whether he says yes: his
/// own pull to his sport, your way with people, and a senior's last year.
pub fn recruit_athlete(w: &mut World, key: u64) -> Result<String, String> {
    let Some(Staff::School(h)) = readers::seat_staff(w) else {
        return Err("Only a high school coach recruits his own building.".into());
    };
    let a = athletes(w).into_iter().find(|a| a.key == key).ok_or("You've already talked to him this year.")?;
    let rel = readers::you(w).map_or(10.0, |c| w.coaches[c].ability().relationships);
    let yes = key_uniform(key ^ 0x5A7, w.dates.season as u64) < yes_chance(&a, rel, w.dates.season);
    let d = w.today;
    if !yes {
        let (name, sport) = (a.name.clone(), SPORTS[a.sport].0.to_string());
        w.scouting.own.athletes.push(AthleteTalk { key, date: d, yes: false, id: None, name, sport });
        return Ok(format!("{} thanks you and sticks with {}.", a.name, SPORTS[a.sport].0.to_lowercase()));
    }
    let i = w.add_athlete(h, &a.name, a.ethnicity, a.class_of, key, a.pos, SPORTS[a.sport].2, (a.truth - 10.0) / 3.0);
    let id = w.people[i].player.id;
    let (name, sport) = (a.name.clone(), SPORTS[a.sport].0.to_string());
    w.scouting.own.athletes.push(AthleteTalk { key, date: d, yes: true, id: Some(id), name, sport });
    Ok(format!("{} comes out for football: he starts at {}.", a.name, a.pos.name()))
}

// --- The day ----------------------------------------------------------------------------------

/// Lane 8's daily work for your seat.
pub(crate) fn day(w: &mut World, d: Date) {
    take_trips(w, d);
    if w.scouting.own.review.as_ref().is_some_and(|r| d >= r.due) {
        let _ = close_review(w);
    }
    if let Some(kind) = meeting_today(w, d) {
        hold_meeting(w, d, kind);
    }
    let (_, m, day) = d.parts();
    if m == 9 && day == 2 {
        credit_eye(w);
    }
    if m == 7 && day == 1 && !w.scouting.own.personnel.is_empty() {
        grow_personnel(w);
    }
    // Old trips and interviews drop off after three years.
    let keep = d.add(-3 * 365);
    w.scouting.own.trips.retain(|t| t.date >= keep || !t.done);
    w.scouting.own.interviews.retain(|t| t.date >= keep);
}

/// After the draft: your words settle, a new service starts.
pub(crate) fn after_draft(w: &mut World) {
    settle_words(w);
    if let Some(s) = w.scouting.own.service_next.take() {
        w.scouting.own.service = s;
    }
}
