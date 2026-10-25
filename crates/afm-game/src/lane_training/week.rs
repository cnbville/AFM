//! The training page's week (update 2, lane 4, built to the agreed mock-up):
//! the rules this week against the hours, contact, game prep and effort;
//! each day's clock and who runs each period (TR2, TR3); each player's prep
//! (TR36-TR43); your staff's reasons for this block and this opponent (T1);
//! the work by family; lift groups and body targets (TR4, TR12); the
//! strength coach's numbers (TR5, T3); who's where (TR7-TR9, TR14); and the
//! league (TR42). Words and reads only: a player's prep word is your
//! staff's read of it, and nothing here shows a ceiling.

use std::collections::HashMap;

use afm_engine::calendar::Date;
use afm_engine::positions::Position;
use afm_engine::training::plan::{Day, Kind, Week, testing_day, weekday_index};
use afm_engine::training::prep::{self, Parts, Prep, Status};
use afm_engine::training::rules::{Block, Book, Tier};
use afm_engine::training::seat::{self as ts, PlannedWeek, SeatTeam};
use afm_engine::training::week::block_of_week;
use afm_engine::training::work::Role;
use afm_engine::world::World;
use afm_engine::world::coach::{Group, Role as CRole};
use afm_engine::world::scouting::file;
use afm_engine::world::staff_read::{self, Job, Member};
use serde_json::{Map, Value, json};

use crate::views::date_text;

const WEEKDAYS: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

/// Hours and minutes: "3 h 10", "45 min".
pub(super) fn hm(m: u32) -> String {
    let (h, r) = (m / 60, m % 60);
    match (h, r) {
        (0, r) => format!("{r} min"),
        (h, 0) => format!("{h} h"),
        (h, r) => format!("{h} h {r}"),
    }
}

/// A clock time from minutes after midnight: "6:35 am".
pub(super) fn clock_text(m: u32) -> String {
    let (h, mi) = ((m / 60) % 24, m % 60);
    let (h12, ap) = match h {
        0 => (12, "am"),
        1..=11 => (h, "am"),
        12 => (12, "pm"),
        _ => (h - 12, "pm"),
    };
    format!("{h12}:{mi:02} {ap}")
}

fn short_date(d: Date) -> String {
    let t = date_text(d);
    // "Mon 9 Feb 2026" → "Mon 9 Feb".
    t.rsplit_once(' ').map_or(t.clone(), |(a, _)| a.to_string())
}

// ---------- who runs each period (TR3) ----------

/// Who runs a kind of period (proposed, not confirmed): the head coach runs
/// practice and the team meeting, the coordinators the installs, the plan
/// and the opponent's film, the position coaches their rooms, the strength
/// staff their own work, the trainers treatment, the academic staff study
/// hall, and the players their own throwing.
pub(super) fn runs(kind: Kind, what: u8) -> &'static str {
    match kind {
        Kind::Meeting => ["hc", "coords", "pos"][(what as usize).min(2)],
        Kind::Install | Kind::PlanTest | Kind::HomeStudy => "coords",
        Kind::Film => match what {
            2 | 5 => "pos",
            _ => "coords",
        },
        Kind::Walkthrough => {
            if what == 0 {
                "coords"
            } else {
                "hc"
            }
        }
        Kind::WarmUp | Kind::Mobility | Kind::Vision => "str2",
        Kind::Conditioning | Kind::Speed | Kind::WeightRoom | Kind::MatDrills | Kind::Testing => "str",
        Kind::Individual | Kind::OneOnOnes | Kind::BallSecurity => "pos",
        Kind::Young => "young",
        Kind::InsideRun | Kind::SevenOnSeven | Kind::Team | Kind::Scrimmage | Kind::Situations => "hc",
        Kind::SpecialTeams | Kind::KickStations => "st",
        Kind::Recovery => "trn",
        Kind::StudyHall => "acad",
        Kind::OnAir => {
            if what == 1 {
                "players"
            } else {
                "qb"
            }
        }
        Kind::VrReps => "qb",
        Kind::MentalSkills => "psy",
        Kind::RulesClinic => "ref",
        Kind::Nutrition => "diet",
        Kind::Council => "cap",
        Kind::TeamBuilding => "hc",
        Kind::Signals => "dc",
    }
}

/// When in the day a kind of period goes (TR2): the strength staff's work
/// early (before classes, on a day without football), the dietitian at
/// lunch, study hall, the team dinners and the players' own study in the
/// evening (the page puts community service and an outing in the day), the
/// rest in the day's order.
pub(super) fn slot_of(kind: Kind) -> &'static str {
    match kind {
        Kind::WeightRoom | Kind::MatDrills | Kind::Speed | Kind::Conditioning | Kind::Mobility | Kind::Testing => {
            "early"
        }
        Kind::Vision => "early",
        Kind::Nutrition => "lunch",
        Kind::StudyHall | Kind::TeamBuilding | Kind::HomeStudy => "evening",
        _ => "day",
    }
}

/// Your read of what a staff member does best for the job (TR3): a
/// coach's teaching, a support head's job, as a range on 1-20.
fn read_of(w: &World, m: &Member) -> Option<(u8, u8)> {
    let f = m
        .facets
        .iter()
        .position(|f| f.key.starts_with("teach:") && f.seen())
        .or_else(|| m.facets.iter().position(|f| f.group == "The job" && f.seen()))?;
    staff_read::now(w, m, f)
}

fn last_name(n: &str) -> &str {
    n.split_whitespace().rfind(|x| !matches!(*x, "Jr." | "Sr." | "II" | "III" | "IV")).unwrap_or(n)
}

/// Your staff for the week's periods, by the keys `runs` gives: each with
/// his name, page, title and your read of him in a word.
pub(super) fn staff(w: &World, tier: Tier) -> Value {
    let ms = staff_read::members(w);
    let you = w.career.as_ref().map(|c| c.coach);
    let my_role = you.and_then(|c| w.coaches[c].job).map(|j| j.role);
    let entry = |m: &Member| {
        let read = read_of(w, m);
        json!({
            "name": m.name,
            "id": m.coach,
            "key": m.key.to_string(),
            "title": m.title,
            "words": read.map(|(lo, hi)| staff_read::read_words(lo, hi)),
            "sure": read.map(|(lo, hi)| staff_read::sure_words(lo, hi)),
            "range": read.map(|(lo, hi)| [lo, hi]),
            "teach": m.facets.iter().any(|f| f.key.starts_with("teach:") && f.seen()),
            "you": false,
        })
    };
    let you_as = |title: &str| json!({ "name": "You", "id": you, "title": title, "you": true });
    let find = |f: &dyn Fn(&Member) -> bool| ms.iter().find(|m| f(m)).map(entry);
    let mut map = Map::new();
    let head =
        if w.seat_head() { Some(you_as("Head coach")) } else { find(&|m| m.job == Job::Coach(CRole::HeadCoach)) };
    let head = head.unwrap_or_else(|| json!({ "name": "The head coach", "title": "Head coach" }));
    map.insert("hc".into(), head.clone());
    let coord = |r: CRole, title: &str| {
        if my_role == Some(r) { Some(you_as(title)) } else { find(&|m| m.job == Job::Coach(r)) }
    };
    let oc = coord(CRole::OffensiveCoordinator, "Offensive coordinator");
    let dc = coord(CRole::DefensiveCoordinator, "Defensive coordinator");
    // A room's coach: a college position coach, an NFL room or a high
    // school assistant on that group.
    let room = |g: Group| -> Option<Value> {
        if my_role == Some(CRole::PositionCoach(g)) {
            return Some(you_as(g.coach_title()));
        }
        ms.iter()
            .find(|m| {
                m.group == Some(g) && matches!(m.job, Job::Coach(CRole::PositionCoach(_)) | Job::Room(_) | Job::HsCoach)
            })
            .map(entry)
    };
    let name_of = |v: &Value| v["name"].as_str().unwrap_or("").to_string();
    let coords = match (&oc, &dc) {
        (Some(o), Some(d)) => json!({
            "name": format!("{} and {}", last_name(&name_of(o)), last_name(&name_of(d))),
            "title": "The coordinators, offense and defense",
            "group": [o.clone(), d.clone()],
        }),
        (Some(o), None) => o.clone(),
        (None, Some(d)) => d.clone(),
        (None, None) => head.clone(),
    };
    map.insert("oc".into(), oc.clone().unwrap_or_else(|| head.clone()));
    map.insert("dc".into(), dc.clone().unwrap_or_else(|| head.clone()));
    map.insert("coords".into(), coords);
    let rooms: Vec<Value> = Group::ALL.iter().filter_map(|&g| room(g)).collect();
    map.insert(
        "pos".into(),
        json!({
            "name": "Position coaches",
            "title": if rooms.is_empty() { "The head coach and his assistants" } else { "Each room with its coach" },
            "group": rooms,
        }),
    );
    map.insert("qb".into(), room(Group::QB).or_else(|| oc.clone()).unwrap_or_else(|| head.clone()));
    map.insert("st".into(), room(Group::Specialists).unwrap_or_else(|| head.clone()));
    let support = |j: Job| find(&|m| m.job == j);
    let strength = support(Job::StrengthHead).unwrap_or_else(|| match tier {
        Tier::High => head.clone(),
        _ => json!({ "name": "The strength staff", "title": "Strength and conditioning" }),
    });
    map.insert("str".into(), strength.clone());
    map.insert("str2".into(), support(Job::StrengthAssistant).unwrap_or_else(|| strength.clone()));
    map.insert(
        "trn".into(),
        support(Job::Trainer).unwrap_or_else(|| json!({ "name": "The athletic trainer", "title": "Medical" })),
    );
    map.insert(
        "diet".into(),
        support(Job::Dietitian).unwrap_or_else(|| {
            json!({ "name": "A visiting dietitian", "title": if tier == Tier::High { "From the district" } else { "Nutrition" } })
        }),
    );
    map.insert(
        "acad".into(),
        support(Job::Academic).unwrap_or_else(|| {
            json!({ "name": if tier == Tier::High { "The school's teachers" } else { "The academic staff" }, "title": "Academics" })
        }),
    );
    map.insert(
        "psy".into(),
        json!({
            "name": "The sports psychologist",
            "title": match tier { Tier::College => "Hired by the athletic department", _ => "On the club's medical staff" },
        }),
    );
    map.insert(
        "ref".into(),
        json!({
            "name": "Visiting officials",
            "title": match tier { Tier::High => "The state association's crew", Tier::College => "The conference's officials", Tier::Pro => "The league's officiating department" },
        }),
    );
    map.insert("cap".into(), json!({ "name": "Leadership council", "title": "Elected players; the captains run it" }));
    map.insert(
        "players".into(),
        json!({ "name": "The players", "title": "The quarterback runs it; no staff allowed" }),
    );
    map.insert(
        "young".into(),
        support(Job::GradAssistant).unwrap_or_else(|| match tier {
            Tier::High => json!({ "name": "The JV coaches", "title": "The young players' own period" }),
            _ => json!({ "name": "The assistants", "title": "The young players' own period" }),
        }),
    );
    Value::Object(map)
}

// ---------- the day's clock (TR2) ----------

/// Classes on a day (TR2): a college's semesters and summer session, a
/// high school's year (proposed, not confirmed: spring term 12 January to
/// 5 May, fall term 24 August to 10 December, summer session June and July
/// Monday to Thursday; a high school year 12 August to 19 December and
/// 5 January to 28 May). Returns the hours and the term's name.
fn classes(tier: Tier, d: Date) -> Option<(u32, u32, &'static str)> {
    let wd = weekday_index(d);
    if wd >= 5 {
        return None;
    }
    let (_, m, day) = d.parts();
    let md = m * 100 + day;
    match tier {
        Tier::College => {
            if (112..=505).contains(&md) {
                Some((510, 830, "Spring semester"))
            } else if (824..=1210).contains(&md) {
                Some((480, 780, "Fall semester"))
            } else if (601..=731).contains(&md) && wd < 4 {
                Some((540, 690, "Summer session"))
            } else {
                None
            }
        }
        Tier::High => {
            if (812..=1219).contains(&md) || (105..=528).contains(&md) {
                Some((465, 900, "School"))
            } else {
                None
            }
        }
        Tier::Pro => None,
    }
}

/// A day's clock: when the early work starts, classes, the day's start and
/// the evening, and a game day's kickoff, in minutes after midnight
/// (proposed, not confirmed: mat drills at 5:45 and lifting at 6:35 before
/// a college's winter classes, 6:45 in season; meetings at 3:15 after
/// winter classes and 1:30 in season; a high school lifts at 6:30 before
/// school and practises at 3:15; an NFL day starts with meetings at 8:00).
pub(super) fn day_clock(tier: Tier, block: Block, date: Date, d: &Day, kick: Option<u32>) -> Value {
    let cls = classes(tier, date);
    let mats = d.periods.iter().any(|p| p.kind == Kind::MatDrills);
    let (early, day): (Option<u32>, u32) = match tier {
        Tier::College => match block {
            Block::Season | Block::Postseason => (Some(405), 810),
            Block::Camp => (Some(390), 480),
            Block::Summer => (Some(360), if cls.is_some() { 750 } else { 540 }),
            _ => (Some(if mats { 345 } else { 395 }), if cls.is_some() { 915 } else { 540 }),
        },
        Tier::High => match cls {
            Some(_) => (Some(390), 915),
            None => (Some(420), 540),
        },
        Tier::Pro => (None, if block == Block::Phase1 { 540 } else { 480 }),
    };
    json!({
        "early": early,
        "classes": cls.map(|(a, b, n)| json!({ "from": a, "to": b, "name": n, "text": format!("{} – {}", clock_text(a), clock_text(b)) })),
        "day": day,
        "evening": 1140,
        "kick": kick,
    })
}

// ---------- where the year is (TR10) ----------

fn short_block(b: Block, tier: Tier) -> &'static str {
    match (b, tier) {
        (Block::Winter, Tier::High) => "Offseason",
        (Block::Winter, Tier::Pro) => "On their own",
        (Block::Winter, _) => "Winter",
        (Block::Spring, Tier::High) => "Spring ball",
        (Block::Spring, _) => "Spring",
        (Block::Quiet, _) => "Quiet weeks",
        (Block::Summer, _) => "Summer",
        (Block::Camp, Tier::High) => "Preseason",
        (Block::Camp, _) => "Camp",
        (Block::Season, _) => "Season",
        (Block::Postseason, Tier::College) => "Bowls",
        (Block::Postseason, _) => "Playoffs",
        // The phases are two and three weeks: short labels, the long names
        // on hover.
        (Block::Phase1, _) => "P1",
        (Block::Phase2, _) => "P2",
        (Block::Otas, _) => "OTAs",
        (Block::Break, _) => "Break",
    }
}

fn monday_on_or_before(d: Date) -> Date {
    d.add(-(weekday_index(d) as i32))
}

/// The year's blocks with this week's place in them, the next block's
/// countdown, the next testing day, and in season the next weeks' games.
pub(super) fn year(st: &SeatTeam, monday: Date, today: Date, next_games: &[(Date, String)]) -> Value {
    let f = &st.frame;
    let mut spans: Vec<(Block, Date, Date)> = Vec::new();
    let mut m = monday_on_or_before(f.start.add(1));
    while m < f.end.add(7) {
        let b = block_of_week(f, m);
        match spans.last_mut() {
            Some(s) if s.0 == b => s.2 = m.add(6),
            _ => spans.push((b, m, m.add(6))),
        }
        m = m.add(7);
    }
    let now = spans.iter().position(|s| monday >= s.1 && monday <= s.2);
    let phases: Vec<Value> = spans
        .iter()
        .enumerate()
        .map(|(k, s)| {
            let weeks = ((s.1.days_to(s.2) + 1) as f64 / 7.0).round().max(1.0) as u32;
            let p = if Some(k) == now {
                (s.1.days_to(today) as f64 / (s.1.days_to(s.2) + 1) as f64).clamp(0.0, 1.0)
            } else {
                0.0
            };
            json!({
                "label": short_block(s.0, st.tier),
                "long": s.0.label(st.tier),
                "weeks": weeks,
                "done": s.2 < monday,
                "now": Some(k) == now,
                "p": (p * 100.0).round(),
                "from": short_date(s.1),
                "to": short_date(s.2),
            })
        })
        .collect();
    let mut say: Vec<String> = Vec::new();
    if let Some(k) = now {
        let (b, from, to) = spans[k];
        let week = from.days_to(monday) / 7 + 1;
        let weeks = (from.days_to(to) + 1) / 7;
        if weeks > 1 {
            say.push(format!("{}: week {week} of {weeks}.", b.label(st.tier)));
        }
        if let Some(&(nb, nfrom, nto)) = spans.get(k + 1) {
            let days = today.days_to(nfrom).max(0);
            say.push(format!(
                "{} in {} day{}: {} – {}.",
                nb.label(st.tier),
                days,
                if days == 1 { "" } else { "s" },
                short_date(nfrom),
                short_date(nto)
            ));
        }
    }
    // The next testing day in the coming ten weeks (TR5).
    let mut m = monday;
    for _ in 0..10 {
        if let Some(k) = testing_day(f, m) {
            let d = m.add(k as i32);
            if d >= today {
                say.push(format!("Testing day {}.", short_date(d)));
                break;
            }
        }
        m = m.add(7);
    }
    // In season: next week's game, or the bye.
    if now.is_some_and(|k| matches!(spans[k].0, Block::Season | Block::Postseason)) {
        let next_week: Vec<&(Date, String)> =
            next_games.iter().filter(|g| g.0 >= monday.add(7) && g.0 < monday.add(14)).collect();
        match next_week.first() {
            Some(g) => say.push(format!("Next week: {}.", g.1)),
            None if next_games.iter().any(|g| g.0 >= monday.add(14)) => say.push("Bye next week.".to_string()),
            None => {}
        }
    }
    json!({ "phases": phases, "say": say.join(" ") })
}

// ---------- the summary cards (TR1, TR11, TR13; T2) ----------

/// The book that sets the week's limits: the first day the staff may run.
fn main_book(pw: &PlannedWeek) -> &Book {
    pw.books.iter().find(|b| b.coaches).unwrap_or(&pw.books[0])
}

fn kinds_min(wk: &Week, f: impl Fn(&afm_engine::training::plan::Period) -> bool) -> u32 {
    wk.days.iter().flat_map(|d| &d.periods).filter(|p| f(p)).map(|p| p.min as u32).sum()
}

/// The hours, contact and game prep cards.
pub(super) fn cards(pw: &PlannedWeek, tier: Tier, counts: &[String], opener: Option<String>) -> Value {
    let wk = &pw.week;
    let k = main_book(pw);
    let used: u32 = wk.days.iter().map(|d| d.countable()).sum();
    let film: u32 = kinds_min(wk, |p| p.kind.film() && p.counted());
    let strength: u32 = kinds_min(wk, |p| p.counted() && (p.kind.strength_staff() || p.kind == Kind::WeightRoom));
    let longest = wk.days.iter().map(|d| d.countable()).max().unwrap_or(0);
    let off = wk.days.iter().filter(|d| d.rest()).count();
    let off_days: Vec<&str> =
        wk.days.iter().enumerate().filter(|(_, d)| d.rest()).map(|(i, _)| &WEEKDAYS[i][..3]).collect();
    let game = wk.days.iter().any(|d| d.game);
    let hours = json!({
        "used": used,
        "used_text": hm(used),
        "cap": k.week_min,
        "cap_text": k.week_min.map(|m| hm(m as u32)),
        "film": film,
        "film_text": hm(film),
        "film_cap": k.film_week,
        "film_cap_text": k.film_week.map(|m| hm(m as u32)),
        "strength_text": hm(strength),
        "longest_text": hm(longest),
        "day_cap_text": k.day_min.map(|m| hm(m as u32)),
        "field_cap_text": k.field_min.map(|m| hm(m as u32)),
        "game": game,
        "off": off,
        "off_need": k.days_off,
        "off_text": format!(
            "{} required{} · {} kept",
            k.days_off,
            if off_days.is_empty() { String::new() } else { format!(" ({})", off_days.join(", ")) },
            off
        ),
        "left_text": k.week_min.map(|m| hm((m as u32).saturating_sub(used))),
        "label": match (k.week_min, k.film_week, k.day_min) {
            (Some(m), Some(fm), _) => format!("{} a week · film up to {}", hm(m as u32), hm(fm as u32)),
            (Some(m), None, Some(dm)) => format!("{} a week · {} a day", hm(m as u32), hm(dm as u32)),
            (Some(m), None, None) => format!("{} a week", hm(m as u32)),
            _ => match tier {
                Tier::Pro => "The CBA's phases and limits".to_string(),
                Tier::College => "NCAA: nothing required".to_string(),
                Tier::High => "The state's limits".to_string(),
            },
        },
        "counts": counts,
        "practice_days": k.practice_days,
    });
    // Contact (the week's full-contact minutes, yours against the staff's).
    let full = |wk: &Week| kinds_min(wk, |p| p.contact.full());
    let field_allowed = pw.books.iter().any(|b| b.field);
    let per_day: Vec<Value> = wk
        .days
        .iter()
        .enumerate()
        .filter(|(_, d)| !d.game && d.periods.iter().any(|p| p.kind.on_field()))
        .take(4)
        .map(|(i, d)| {
            let m: u32 = d.periods.iter().filter(|p| p.contact.full()).map(|p| p.min as u32).sum();
            let what: Vec<String> = d
                .periods
                .iter()
                .filter(|p| p.contact.full())
                .map(|p| p.kind.label().to_lowercase())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();
            let gear = d.periods.iter().map(|p| p.gear).max().map(|g| g.label()).unwrap_or("");
            json!([
                WEEKDAYS[i],
                if m > 0 { format!("{m} ({})", what.join(", ")) } else { format!("0 · {}", gear.to_lowercase()) }
            ])
        })
        .collect();
    let contact = json!({
        "allowed": field_allowed,
        "mine": full(wk),
        "staff": full(&pw.neutral),
        "pads": k.max_gear.label(),
        "max_contact": k.max_contact.label(),
        "days": per_day,
        "week_cap": k.full_week,
        "days_cap": k.full_days,
    });
    // Game prep (T2: winter film is never game prep).
    let mine = afm_engine::training::work::prep_minutes(wk);
    let staff = afm_engine::training::work::prep_minutes(&pw.neutral);
    let install: u32 = kinds_min(wk, |p| p.kind == Kind::Install && p.what != 4);
    let opp_film: u32 = kinds_min(wk, |p| p.kind == Kind::Film && matches!(p.what, 0 | 3));
    let system: u32 = kinds_min(wk, |p| (p.kind == Kind::Film || p.kind == Kind::Install) && p.what == 4);
    let all_film: u32 = kinds_min(wk, |p| p.kind == Kind::Film);
    let gp = json!({
        "game": game,
        "mine": mine.round(),
        "staff": staff.round(),
        // About one new call for every two minutes of install (proposed,
        // not confirmed).
        "calls": (install as f64 * 0.55).round(),
        "opp_film_text": hm(opp_film),
        "film_text": hm(all_film),
        "system_text": hm(system),
        "opener": opener,
    });
    json!({ "hours": hours, "contact": contact, "gp": gp })
}

// ---------- each player's prep (TR36-TR43) ----------

/// About how many practice snaps a minute of team work gives (proposed,
/// not confirmed: a 24-play team period in 15 minutes).
const REPS_PER_MIN: f64 = 1.6;

fn of(a: f64, b: f64, unit: &str) -> String {
    let (a, b) = (a.round() as u32, b.round() as u32);
    if b == 0 && a == 0 {
        "—".to_string()
    } else if unit == "min" {
        format!("{a} of {b} min")
    } else {
        format!("{a} of {b}{unit}")
    }
}

fn status_text(s: Status, why: &str) -> String {
    let base = match s {
        Status::Full => "Full",
        Status::Limited => "Limited",
        Status::Rested => "Rested",
        Status::Out => "Rehab",
        Status::Away => "Away part-time",
        Status::Absent => "Absent",
        Status::New => "Full · new",
    };
    if why.is_empty() { base.to_string() } else { format!("{base} ({why})") }
}

fn ordinal(n: usize) -> &'static str {
    ["", "First", "Second", "Third", "Fourth", "Fifth"][n.min(5)]
}

/// What your staff says about his week (TR43): the poor run with its
/// reason, the rust and the slowed growth; a word for the mental reps of a
/// player who couldn't practise (TR37); a newcomer catching up (TR8).
fn says(p: &Prep, word: u8, words: &[u8; 5], rust: (f64, f64), growth: f64) -> String {
    let poor = |w: u8| w == 1 || w == 2;
    if poor(word) {
        let run = words.iter().rev().take_while(|&&w| poor(w)).count().max(1);
        let mut s = if run >= 2 {
            format!("{} poor week running: {}.", ordinal(run), p.why())
        } else {
            format!("A poor week: {}.", p.why())
        };
        if rust.0 + rust.1 > 0.0 {
            s.push_str(" He's rusty now; two or three good weeks win it back.");
        } else {
            s.push_str(" One more like it and the rust starts.");
        }
        if growth < 0.95 {
            s.push_str(" His growth has slowed.");
        }
        return s;
    }
    if word == 3 {
        let mut s = format!("Patchy: {}.", p.why());
        if rust.0 + rust.1 > 0.0 {
            s.push_str(" Still shaking off the rust.");
        }
        return s;
    }
    if word >= 4 {
        match p.status {
            Status::Out | Status::Limited | Status::Rested => {
                return "Out of some practice, but at every meeting and on film: the mental reps keep him prepared."
                    .to_string();
            }
            Status::New => return "New, and catching up on the system fast.".to_string(),
            _ => {}
        }
        if rust.0 + rust.1 > 0.0 {
            return "Back on track; the rust is nearly gone.".to_string();
        }
        if word == 5 && p.test && p.game {
            return "Knows the plan cold: near perfect on the plan test.".to_string();
        }
    }
    String::new()
}

/// A player's prep for the page: his status, his week's parts against his
/// role's usual, the staff's word, five weeks of words (oldest first), his
/// rust and what your staff says. `live`: Monday, before the week's pass
/// records the word.
#[allow(clippy::too_many_arguments)]
pub(super) fn prep_json(
    w: &World,
    st: &SeatTeam,
    pw: &PlannedWeek,
    i: usize,
    role: Role,
    live: bool,
    why: &str,
) -> (Value, u8, Prep) {
    let p = &w.people[i];
    let id = p.player.id;
    let hist = prep::words(&p.player.train);
    // The word this week: the live read before Monday's pass, the recorded
    // one after it. A week with nothing recorded (a save from before prep
    // was kept, or he joined after Monday) shows the live read too.
    let fresh = live || hist[0] == 0;
    // After the pass his week reads as the pass saw it: out hurt or not,
    // whatever Tuesday's healing or a game has done since.
    let out = if live {
        None
    } else if hist[0] == 0 {
        Some(p.pro.as_ref().is_some_and(|c| c.out_weeks > 0))
    } else {
        Some(prep::was_out(&p.player.train))
    };
    let (pr, x, n): (Prep, Parts, Parts) = ts::prep_detail(w, st, pw, i, role, out);
    let true_word = if fresh { pr.word } else { hist[0] };
    let key = prep::key(w.options.seed as u64, pw.monday);
    let word = prep::staff_read(true_word, pr.test, key, id);
    let mut five: [u8; 5] = if fresh {
        [hist[3], hist[2], hist[1], hist[0], pr.word]
    } else {
        [hist[4], hist[3], hist[2], hist[1], hist[0]]
    };
    // The bars show your staff's reads too: this week's as the chip says.
    five[4] = word;
    let rust = prep::rust(&p.player.train);
    let growth = prep::growth_factor(&p.player.train);
    let kept = if pr.days == 0 { 1.0 } else { 1.0 - pr.missed as f64 / pr.days as f64 };
    let own_word = if pr.own >= 1.0 {
        "Driven"
    } else if pr.own >= 0.85 {
        "Steady"
    } else if pr.own >= 0.7 {
        "Casual"
    } else {
        "Lazy"
    };
    let home: u32 = kinds_min(&pw.week, |q| q.kind == Kind::HomeStudy);
    let device = match st.tier {
        Tier::High if st.frame.season >= 2012 => "Hudl",
        Tier::College | Tier::Pro if st.frame.season >= 2011 => "Tablet",
        _ => "Playbook",
    };
    let vr: u32 = kinds_min(&pw.week, |q| q.kind == Kind::VrReps);
    let own_text = if home > 0 {
        format!("{device} {} · {own_word}", hm((home as f64 * pr.own).round() as u32))
    } else {
        format!("{device} · {own_word}")
    };
    let game = pr.game;
    // At another sport by agreement: the classroom at home, the work on his
    // off days (the same reading as his word).
    let away = pr.status == Status::Away;
    let room = if away { 1.0 } else { pr.status.there().1 };
    let count = |wk: &Week, k: Kind| wk.days.iter().filter(|d| d.periods.iter().any(|q| q.kind == k)).count() as f64;
    let (work_text, meet_text, walks_text) = if game {
        (
            if role == Role::Young {
                format!("Scout team: {} cards", (x.reps * REPS_PER_MIN * kept).round())
            } else {
                of(x.reps * REPS_PER_MIN * kept, n.reps * REPS_PER_MIN, "")
            },
            of(count(&pw.week, Kind::Install) * kept * room, count(&pw.neutral, Kind::Install), ""),
            of(count(&pw.week, Kind::Walkthrough) * kept * room, count(&pw.neutral, Kind::Walkthrough), ""),
        )
    } else {
        (
            if away { "Off days only".to_string() } else { of(x.work * kept, n.work, "min") },
            of(x.install * kept, n.install, "min"),
            of(x.walks * kept, n.walks, "min"),
        )
    };
    let film_text = {
        let base = of(x.film * kept * room, n.film, "min");
        if vr > 0 && matches!(p.player.position, Position::QB) { format!("{base} · +{vr}\u{a0}VR") } else { base }
    };
    let rust_text = if rust.0 + rust.1 > 0.0 {
        let mut parts = Vec::new();
        if rust.0 >= 0.5 {
            parts.push(format!("−{:.0} reading", rust.0.round().max(1.0)));
        }
        if rust.1 >= 0.5 {
            parts.push(format!("−{:.0} technique", rust.1.round().max(1.0)));
        }
        if parts.is_empty() { "A little".to_string() } else { parts.join(" · ") }
    } else {
        String::new()
    };
    let says = says(&pr, word, &five, rust, growth);
    let v = json!({
        "word": word,
        "word_text": prep::WORDS[word as usize],
        "status": pr.status.name(),
        "status_text": status_text(pr.status, why),
        "limited": !matches!(pr.status, Status::Full | Status::New),
        "work": work_text,
        "meet": meet_text,
        "film": film_text,
        "walks": walks_text,
        "own": own_text,
        "five": five,
        "rust": rust_text,
        "says": says,
        "warn": word > 0 && word <= 2,
        "missed": pr.missed,
        "took": (pr.took * 100.0).round(),
        "game": game,
        "test": pr.test,
    });
    (v, word, pr)
}

/// The effort cell's word from the roster's mean prep score.
pub(super) fn effort_word(mean: f64) -> &'static str {
    if mean >= 0.92 {
        "High"
    } else if mean >= 0.82 {
        "Good"
    } else if mean >= 0.7 {
        "Fair"
    } else {
        "Poor"
    }
}

// ---------- your staff's reasons (T1) ----------

/// Why your staff planned this week (T1): the block first, then what this
/// week does (mat drills, the hours, the install's pieces, crowd noise,
/// pads by day, the weather's moves, the newcomers, a limited starter, the
/// bye ahead), then the staff's style.
#[allow(clippy::too_many_arguments)]
pub(super) fn reasons(
    w: &World,
    st: &SeatTeam,
    pw: &PlannedWeek,
    style_lines: Vec<String>,
    game: Option<(Date, String, bool)>,
    next_week_game: bool,
    limited: Option<String>,
    first_year: bool,
) -> Vec<String> {
    let wk = &pw.week;
    let k = main_book(pw);
    let days_with = |kind: Kind| wk.days.iter().filter(|d| d.periods.iter().any(|p| p.kind == kind)).count();
    let mut v: Vec<String> = Vec::new();
    let field = pw.books.iter().any(|b| b.field);
    v.push(match (pw.block, st.tier) {
        (Block::Winter, Tier::College) => {
            "It's winter: no football on the field until spring practice, so the week is strength, speed and the playbook on film.".to_string()
        }
        (Block::Winter, Tier::High) => {
            "The offseason program: the weight room before school and speed after it; football waits for spring ball.".to_string()
        }
        (Block::Winter, Tier::Pro) | (Block::Quiet, _) | (Block::Break, _) => {
            "The players' own time: the rules keep the staff from requiring anything, so the strength staff checks in and the veterans run their own work.".to_string()
        }
        (Block::Spring, _) => {
            "Spring practice: the install goes in and the depth chart is open, with pads after the first days.".to_string()
        }
        (Block::Summer, Tier::College) => {
            "Summer: the strength staff's hours, film for part of them, and the quarterbacks' own throwing with the receivers.".to_string()
        }
        (Block::Summer, _) => "Summer workouts: lifting and speed in the mornings, 7-on-7 when the rules allow it.".to_string(),
        (Block::Camp, _) => "Camp: acclimatisation first, then pads; every practice counts toward the depth chart.".to_string(),
        (Block::Season | Block::Postseason, _) => match &game {
            Some((d, opp, home)) => format!(
                "A {} game against {opp} on {}: the install goes in by Thursday and the legs stay fresh for game day.",
                if *home { "home" } else { "road" },
                WEEKDAYS[weekday_index(*d)]
            ),
            None if pw.block == Block::Postseason => {
                "Bowl practice: extra reps, most of them early for the young players.".to_string()
            }
            None => "No game this week: lighter, a self-scout, and more reps for the young players.".to_string(),
        },
        (Block::Phase1, _) => "Phase 1: strength and conditioning only; no footballs with the coaches yet.".to_string(),
        (Block::Phase2, _) => "Phase 2: drills on the field, but no offense against defense.".to_string(),
        (Block::Otas, _) => {
            "OTAs: team practice without pads, voluntary; the minicamp is the one that's required.".to_string()
        }
    });
    let mats = days_with(Kind::MatDrills);
    if mats > 0 {
        let tone =
            if first_year && pw.block == Block::Winter { " A new staff's first winter sets the tone." } else { "" };
        v.push(match mats {
            1 => format!("Mat drills once this week.{tone}"),
            2 => format!("Mat drills twice: a third morning would start to cost mood.{tone}"),
            n => format!("Mat drills {n} mornings: it hardens them, but past two it costs mood.{tone}"),
        });
    }
    if let Some(cap) = k.week_min {
        let used: u32 = wk.days.iter().map(|d| d.countable()).sum();
        let film: u32 = kinds_min(wk, |p| p.kind.film() && p.counted());
        let left = (cap as u32).saturating_sub(used);
        let film_part = match k.film_week {
            Some(fm) if film > 0 => format!(", with {} of the {} film hours", hm(film), hm(fm as u32)),
            _ => String::new(),
        };
        if left == 0 {
            v.push(format!("The week uses all {}{film_part}.", hm(cap as u32)));
        } else {
            v.push(format!("We're {} inside the {}{film_part}.", hm(left), hm(cap as u32)));
        }
    }
    let installs: Vec<String> = wk
        .days
        .iter()
        .flat_map(|d| &d.periods)
        .filter(|p| p.kind == Kind::Install && p.what < 4)
        .map(|p| p.name().trim_start_matches("Install: ").to_string())
        .collect();
    if installs.len() >= 2 {
        v.push(format!(
            "The install goes in {} pieces ({}) so nobody gets every new call on one day.",
            installs.len(),
            installs.join(", ")
        ));
    }
    if wk.days.iter().flat_map(|d| &d.periods).any(|p| p.noise) {
        v.push("A loud stadium ahead: crowd noise on in the team periods, and a silent count.".to_string());
    }
    if field {
        let gear_days = |g: afm_engine::training::plan::Gear| -> Vec<&str> {
            wk.days
                .iter()
                .enumerate()
                .filter(|(_, d)| !d.game && d.periods.iter().any(|p| p.kind.on_field()))
                .filter(|(_, d)| d.periods.iter().map(|p| p.gear).max() == Some(g))
                .map(|(i, _)| WEEKDAYS[i])
                .collect()
        };
        let pads = gear_days(afm_engine::training::plan::Gear::Pads);
        let shells = gear_days(afm_engine::training::plan::Gear::Shells);
        if !pads.is_empty() {
            let mut s = format!("Pads {}", list(&pads));
            if !shells.is_empty() {
                s.push_str(&format!(", shells {}", list(&shells)));
            }
            s.push('.');
            v.push(s);
        }
    }
    for (i, wx) in pw.weather.iter().enumerate() {
        let Some(wx) = wx else { continue };
        let what = |why: &str| match why {
            "rain" => "rain",
            "snow" => "snow",
            "cold" => "the cold",
            "wind" => "the wind",
            "lightning" => "lightning",
            _ => "severe weather",
        };
        match wx.place {
            "inside" => {
                v.push(format!("{}'s field work moved indoors: {}° and {}.", WEEKDAYS[i], wx.temp, what(wx.why)))
            }
            "lost" => v.push(format!("{}'s field work was lost to {}: no indoor field.", WEEKDAYS[i], what(wx.why))),
            "delayed" => v.push(format!("A lightning hold cost {} half an hour on the field.", WEEKDAYS[i])),
            _ => {}
        }
    }
    let new = st.roster.iter().filter(|&&i| w.training.is_new(w.people[i].player.id, pw.monday)).count();
    if new >= 3 && !matches!(pw.block, Block::Season | Block::Postseason) {
        v.push(format!(
            "The {new} newcomers lift as their own group so the strength staff can teach them the lifts from scratch."
        ));
    }
    if let Some(l) = limited {
        v.push(l);
    }
    if matches!(pw.block, Block::Season) && game.is_some() && !next_week_game {
        v.push("Bye next week: no extra conditioning this week.".to_string());
    }
    if field {
        v.extend(style_lines);
    }
    v
}

fn list(xs: &[&str]) -> String {
    match xs.len() {
        0 => String::new(),
        1 => xs[0].to_string(),
        n => format!("{} and {}", xs[..n - 1].join(", "), xs[n - 1]),
    }
}

// ---------- the week's work by family ----------

pub(super) const SHOWN: [&str; 9] =
    ["Strength", "Speed and movement", "Conditioning", "Technique", "Contact", "Kicking", "Mental", "Recovery", "Team"];

/// Minutes by family, yours against the staff's, with what each grows.
pub(super) fn families(pw: &PlannedWeek) -> Vec<Value> {
    let by = |wk: &Week| {
        let mut m: HashMap<&'static str, u32> = HashMap::new();
        for p in wk.days.iter().flat_map(|d| &d.periods) {
            if let Some(f) = p.kind.shown_family(p.contact) {
                *m.entry(f).or_default() += p.min as u32;
            }
        }
        m
    };
    let (a, b) = (by(&pw.week), by(&pw.neutral));
    SHOWN
        .iter()
        .map(|&f| {
            let (x, y) = (a.get(f).copied().unwrap_or(0), b.get(f).copied().unwrap_or(0));
            json!({ "family": f, "mine": x, "staff": y, "text": hm(x), "grows": grows(f) })
        })
        .collect()
}

/// What each family grows (the mock-up's words).
pub(super) fn grows(f: &str) -> &'static str {
    match f {
        "Strength" => "Strength and power; the body target",
        "Speed and movement" => "Acceleration, agility, change of direction",
        "Conditioning" => "Stamina and freshness late in games; toughness from mat drills",
        "Technique" => "Each position's technique ratings",
        "Contact" => "Tackling, block shedding and toughness, with injury risk",
        "Kicking" => "Kicking, punting, snapping, holding",
        "Mental" => "Knowledge of the system, reading, composure; the week's prep",
        "Recovery" => "Freshness and a little less soft-tissue risk",
        "Team" => "Bonds, leadership and mood",
        _ => "",
    }
}

// ---------- lift groups and body targets (TR4, TR12) ----------

fn lineman(p: Position) -> bool {
    matches!(p, Position::OT | Position::IOL | Position::DT | Position::EDGE)
}

/// The lift groups (TR4): out of season four groups by unit, the newcomers
/// and rehab on their own; in season linemen, skill groups, rehab and the
/// scout team (proposed, not confirmed). Times come from the page's clock.
pub(super) fn lift_groups(w: &World, st: &SeatTeam, pw: &PlannedWeek, roles: &HashMap<usize, Role>) -> Vec<Value> {
    let d = pw.monday;
    let hurt = |i: usize| {
        let p = &w.people[i];
        w.training.hurt_on(p.player.id, d).is_some() || p.pro.as_ref().is_some_and(|c| c.out_weeks > 0)
    };
    let new = |i: usize| w.training.is_new(w.people[i].player.id, d);
    let pos = |i: usize| w.people[i].player.position;
    let season = matches!(pw.block, Block::Season | Block::Postseason | Block::Camp);
    let n = |f: &dyn Fn(usize) -> bool| st.roster.iter().filter(|&&i| f(i)).count();
    if season || st.tier == Tier::Pro {
        let mut v = vec![
            json!({ "name": "Linemen", "count": n(&|i| !hurt(i) && (lineman(pos(i)) || pos(i) == Position::TE)), "by": "str", "at": 0 }),
            json!({ "name": "Skill groups", "count": n(&|i| !hurt(i) && !lineman(pos(i)) && pos(i) != Position::TE), "by": "str2", "at": 1 }),
            json!({ "name": "Rehab and limited", "count": n(&|i| hurt(i)), "by": "trn", "at": -1 }),
        ];
        if st.tier != Tier::Pro && season {
            let young = n(&|i| roles.get(&i) == Some(&Role::Young) && !hurt(i));
            v.push(json!({ "name": "Scout team", "count": young, "by": "young", "at": -2 }));
        }
        v
    } else {
        let g1 = |i: usize| lineman(pos(i));
        let g2 = |i: usize| matches!(pos(i), Position::LB | Position::TE | Position::RB) || pos(i).is_specialist();
        let apart = |i: usize| new(i) || hurt(i);
        vec![
            json!({ "name": "G1 · Offensive and defensive line", "count": n(&|i| !apart(i) && g1(i)), "by": "str", "at": 0 }),
            json!({ "name": "G2 · Linebackers, tight ends, backs, specialists", "count": n(&|i| !apart(i) && g2(i)), "by": "str2", "at": 1 }),
            json!({ "name": "G3 · Quarterbacks, receivers, defensive backs", "count": n(&|i| !apart(i) && !g1(i) && !g2(i)), "by": "str", "at": 2 }),
            json!({ "name": "G4 · Newcomers and rehab", "count": n(&|i| apart(i)), "by": "str2", "at": 3 }),
        ]
    }
}

/// Body targets by group (TR12): the group's weight now against its
/// targets (your plan's, else the weight he is growing into this year).
pub(super) fn body_targets(w: &World, st: &SeatTeam) -> Vec<Value> {
    let groups: [(&str, &dyn Fn(Position) -> bool); 7] = [
        ("Offensive line", &|p| matches!(p, Position::OT | Position::IOL)),
        ("Defensive line", &|p| matches!(p, Position::DT | Position::EDGE)),
        ("Linebackers", &|p| p == Position::LB),
        ("Tight ends and backs", &|p| matches!(p, Position::TE | Position::RB)),
        ("Receivers and DBs", &|p| matches!(p, Position::WR | Position::CB | Position::S)),
        ("Quarterbacks", &|p| p == Position::QB),
        ("Specialists", &|p| p.is_specialist()),
    ];
    let mut out = Vec::new();
    for (name, f) in groups {
        let mut now = Vec::new();
        let mut target = Vec::new();
        let mut planned = 0;
        for &i in &st.roster {
            let p = &w.people[i];
            if !f(p.player.position) {
                continue;
            }
            let wt = afm_engine::body::body_now(&p.player).weight;
            now.push(wt);
            match w.training.plan_of(p.player.id).body {
                Some((t, _)) => {
                    target.push(t as f64);
                    planned += 1;
                }
                None => {
                    let mut older = p.player.clone();
                    older.age += 1;
                    target.push(afm_engine::body::body_now(&older).weight.max(wt));
                }
            }
        }
        if now.is_empty() {
            continue;
        }
        let avg = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
        let (a, t) = (avg(&now).round(), avg(&target).round());
        let diff = t - a;
        let status = if diff.abs() <= 3.0 {
            "On track".to_string()
        } else if diff > 0.0 {
            format!("{diff:.0} lb to add")
        } else {
            format!("{:.0} lb to lose", -diff)
        };
        out.push(json!({
            "group": name,
            "now": format!("avg {a:.0} lb"),
            "target": format!("target {t:.0}"),
            "status": status,
            "warn": diff.abs() > 6.0,
            "planned": planned,
            "count": now.len(),
        }));
    }
    out
}

// ---------- the strength coach's numbers (TR5, T3) ----------

/// The last testing day's numbers for your roster, the next testing day,
/// and each player's weight against his target.
pub(super) fn numbers(w: &World, st: &SeatTeam, monday: Date, today: Date) -> Value {
    let pro = st.tier == Tier::Pro;
    let next = {
        let mut m = monday;
        let mut found = None;
        for _ in 0..30 {
            if let Some(k) = testing_day(&st.frame, m) {
                let d = m.add(k as i32);
                if d > today {
                    found = Some(d);
                    break;
                }
            }
            m = m.add(7);
        }
        found
    };
    let Some(day) = w.training.tests.iter().rev().find(|t| t.date <= today) else {
        return json!({
            "date": null,
            "next": next.map(date_text),
            "rows": [],
            "pro": pro,
        });
    };
    let mut rows: Vec<(usize, Value)> = day
        .rows
        .iter()
        .filter_map(|r| {
            let i = st.roster.iter().copied().find(|&i| w.people[i].player.id == r.id)?;
            let p = &w.people[i];
            let now = afm_engine::body::body_now(&p.player).weight.round();
            let target = w.training.plan_of(r.id).body.map(|(t, _)| t as f64);
            let hurt = w.training.hurt_on(r.id, day.date).is_some();
            let bench = if pro { format!("{} reps", r.reps) } else { format!("{} lb", r.bench) };
            Some((
                p.player.position.index(),
                json!({
                    "id": r.id,
                    "name": p.name,
                    "pos": p.player.position.name(),
                    "bench": if hurt { "rehab".to_string() } else { bench },
                    "squat": if hurt || pro { "—".to_string() } else { format!("{} lb", r.squat) },
                    "clean": if hurt || pro { "—".to_string() } else { format!("{} lb", r.clean) },
                    "speed": if hurt { "—".to_string() } else if pro { format!("{:.2}", r.split as f64 / 100.0) } else { format!("{:.2}", r.forty as f64 / 100.0) },
                    "shuttle": if hurt { "—".to_string() } else { format!("{:.2}", r.shuttle as f64 / 100.0) },
                    "vertical": if hurt { "—".to_string() } else { format!("{:.1}\"", r.vertical as f64 / 2.0) },
                    "tested": r.weight,
                    "weight": now,
                    "target": target,
                    "diff": target.map(|t| now - t),
                }),
            ))
        })
        .collect();
    rows.sort_by_key(|r| r.0);
    json!({
        "date": date_text(day.date),
        "baseline": day.baseline,
        "next": next.map(date_text),
        "rows": rows.into_iter().map(|r| r.1).collect::<Vec<_>>(),
        "pro": pro,
        "older": w.training.tests.iter().rev().skip(1).map(|t| date_text(t.date)).collect::<Vec<_>>(),
    })
}

// ---------- who's where (TR7, TR8, TR9, TR14) ----------

/// The trainers' return-to-play steps (TR7): rest and treatment, range of
/// motion, strength back, running straight lines, drills without contact,
/// full practice (proposed, not confirmed), by how far along the injury is.
fn rehab_step(from: Date, until: Date, d: Date) -> (usize, usize, &'static str) {
    const STEPS: [&str; 6] = [
        "rest and treatment",
        "range of motion",
        "strength back",
        "running straight lines",
        "drills, no contact",
        "full practice",
    ];
    let total = from.days_to(until).max(1) as f64;
    let done = (from.days_to(d).max(0) as f64 / total).clamp(0.0, 0.999);
    let n = if total <= 14.0 {
        3
    } else if total <= 42.0 {
        5
    } else {
        6
    };
    let k = (done * n as f64) as usize;
    let steps: Vec<&str> = match n {
        3 => vec![STEPS[0], STEPS[3], STEPS[5]],
        5 => vec![STEPS[0], STEPS[1], STEPS[2], STEPS[3], STEPS[4]],
        _ => STEPS.to_vec(),
    };
    (k + 1, n, steps[k.min(n - 1)])
}

pub(super) fn wheres(w: &World, st: &SeatTeam, pw: &PlannedWeek) -> Value {
    let d = pw.monday;
    let link = |i: usize| {
        let p = &w.people[i];
        json!({ "id": p.player.id, "name": p.name, "pos": p.player.position.name() })
    };
    let by_id: HashMap<u32, usize> = st.roster.iter().map(|&i| (w.people[i].player.id, i)).collect();
    // Rehab.
    let mut rehab: Vec<Value> = w
        .training
        .hurt
        .iter()
        .filter(|h| h.until > d && h.from <= d.add(6))
        .filter_map(|h| {
            let &i = by_id.get(&h.id)?;
            let (k, n, step) = rehab_step(h.from, h.until, d.max(h.from));
            Some(json!({
                "p": link(i),
                "what": h.kind.map_or(h.how.clone(), |k| k.name().to_string()),
                "since": short_date(h.from),
                "step": format!("Step {k} of {n}: {step}"),
                "back": format!("Back about {}", short_date(h.until)),
            }))
        })
        .collect();
    for &i in &st.roster {
        let p = &w.people[i];
        if let Some(c) = p.pro.as_ref().filter(|c| c.out_weeks > 0)
            && !w.training.hurt.iter().any(|h| h.id == p.player.id && h.until > d)
        {
            rehab.push(json!({
                "p": link(i),
                "what": crate::views::status_text(w, p),
                "since": "",
                "step": "With the trainers",
                "back": format!("Out about {} more week{}", c.out_weeks, if c.out_weeks == 1 { "" } else { "s" }),
            }));
        }
    }
    // New arrivals (TR8).
    let mut arrivals: Vec<(Date, Value)> = w
        .training
        .arrived
        .iter()
        .filter(|a| a.1 >= d.add(-28))
        .filter_map(|&(id, at)| {
            let &i = by_id.get(&id)?;
            let p = &w.people[i];
            let how = match st.tier {
                Tier::College if p.college_seasons == 0 => "Freshman",
                Tier::College => "Transfer",
                Tier::Pro if p.pro.as_ref().is_some_and(|c| c.seasons.is_empty()) => "Rookie",
                Tier::Pro => "Signed",
                Tier::High => "New to the school",
            };
            Some((at, json!({ "p": link(i), "how": how, "at": short_date(at) })))
        })
        .collect();
    arrivals.sort_by_key(|a| std::cmp::Reverse(a.0));
    // At another sport (TR9).
    let away: Vec<Value> = if st.tier == Tier::High {
        st.roster
            .iter()
            .filter_map(|&i| {
                let p = &w.people[i];
                let sport = afm_engine::training::offseason::other_sport(&p.player)?;
                let now = afm_engine::training::offseason::sport_ratio(&p.player, pw.block, d).is_some();
                Some(json!({
                    "p": link(i),
                    "what": format!("{}{}", sport.name(), if now { ": in season now, lifts when he can" } else { ": out of season" }),
                    "now": now,
                }))
            })
            .collect()
    } else {
        Vec::new()
    };
    // Study hall (TR14): under a 2.3, or in his first semester; a high
    // school's eligibility line at 2.0 (proposed, not confirmed).
    let line = if st.tier == Tier::High { 2.0 } else { 2.3 };
    let mut study: Vec<(f64, Value)> = if st.tier == Tier::Pro {
        Vec::new()
    } else {
        st.roster
            .iter()
            .filter_map(|&i| {
                let p = &w.people[i];
                let first = st.tier == Tier::College && p.college_seasons == 0;
                let gpa = file::gpa(p);
                let low = gpa < line;
                (first || low).then(|| {
                    (
                        gpa,
                        json!({
                            "p": link(i),
                            "why": if low { format!("{gpa:.2} GPA") } else { "First semester".to_string() },
                            "risk": gpa < line - 0.3,
                        }),
                    )
                })
            })
            .collect()
    };
    study.sort_by(|a, b| a.0.total_cmp(&b.0));
    let flagged = study.len();
    json!({
        "rehab": rehab,
        "arrivals": arrivals.into_iter().map(|a| a.1).take(12).collect::<Vec<_>>(),
        "away": away,
        "study": study.into_iter().map(|s| s.1).take(10).collect::<Vec<_>>(),
        "flagged": flagged,
        "line": line,
        "tier": match st.tier { Tier::High => "hs", Tier::College => "college", Tier::Pro => "nfl" },
    })
}

// ---------- across the league (TR42) ----------

/// Across the league: the same rules for every team; your staff's read of
/// the next opponent's rusty starters from film; the voluntary sessions'
/// absences in the news.
pub(super) fn league(w: &World, opp_roster: Option<(&str, &[usize])>, news: Vec<(String, String)>) -> Vec<Value> {
    let mut v = vec![json!({
        "head": "The same rules for every team.",
        "line": "Other staffs plan their weeks with the same tools, and their players drift the same way: a lazy veteran there gets rusty too.",
    })];
    if let Some((name, roster)) = opp_roster {
        let mut rusty: Vec<(f64, usize)> = roster
            .iter()
            .copied()
            .filter(|&i| w.people[i].last_role == Some(afm_engine::player::PlayingTime::Starter))
            .filter_map(|i| {
                let r = prep::rust(&w.people[i].player.train);
                (r.0 + r.1 >= 1.5).then_some((r.0 + r.1, i))
            })
            .collect();
        rusty.sort_by(|a, b| b.0.total_cmp(&a.0));
        for (_, i) in rusty.into_iter().take(2) {
            let p = &w.people[i];
            v.push(json!({
                "head": name,
                "line": format!(
                    "Your staff's read from film: their {} {} has looked a step slow for weeks. Rusty, or hurt.",
                    p.player.position.name(),
                    p.name
                ),
                "id": p.player.id,
            }));
        }
    }
    for (head, line) in news.into_iter().take(3) {
        v.push(json!({ "head": head, "line": line }));
    }
    v
}
