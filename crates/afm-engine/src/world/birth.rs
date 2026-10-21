//! Birth dates and school classes (final build A1, A2).
//!
//! Every person has a birth date. A school class runs from a September
//! birthday to the next August's, so the class of 2030 was born from
//! September 1, 2011 to August 31, 2012, and on August 31 before each
//! school year every on-time boy in it has the same age: 14 before his
//! freshman year, 17 before his senior year. That age is his "season age",
//! the age the engine's yearly steps and his season rows carry. Every age a
//! screen shows comes from the birth date and the day it is shown.
//!
//! About one boy in ten is a year older than his class (A2): he started
//! school a year late (a "redshirted" kindergartner, mostly summer
//! birthdays), repeated a grade before high school, or his family held him
//! back in eighth grade to gain a year for football. So a senior can be 19.
//!
//! Every draw here is keyed (seed, person), so it moves no other stream.

use crate::calendar::Date;

use super::season::key_uniform;

/// US births by month, share of the year (CDC/NCHS natality, 2010-2019
/// average): fewest in February, most in July to September.
pub const MONTH_SHARE: [f64; 12] = [0.081, 0.075, 0.083, 0.080, 0.084, 0.083, 0.088, 0.089, 0.086, 0.084, 0.079, 0.083];

/// Why a boy sits in the class he does (A2). Stored as a byte on the person.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum ClassPlace {
    #[default]
    OnTime = 0,
    /// Started kindergarten a year late, usually a summer birthday.
    LateStart = 1,
    /// Repeated a grade in elementary or middle school.
    Repeated = 2,
    /// Held back in eighth grade to gain a year for football.
    HeldForFootball = 3,
    /// Reclassified in high school to graduate a year early (built before).
    ReclassifiedUp = 4,
    /// Reclassified in high school to repeat a year (built before).
    ReclassifiedDown = 5,
}

impl ClassPlace {
    pub fn from_u8(v: u8) -> ClassPlace {
        match v {
            1 => ClassPlace::LateStart,
            2 => ClassPlace::Repeated,
            3 => ClassPlace::HeldForFootball,
            4 => ClassPlace::ReclassifiedUp,
            5 => ClassPlace::ReclassifiedDown,
            _ => ClassPlace::OnTime,
        }
    }

    /// The line his page gives it, or None for an on-time boy.
    pub fn words(self) -> Option<&'static str> {
        match self {
            ClassPlace::OnTime => None,
            ClassPlace::LateStart => Some("A year older than his class: he started school a year late."),
            ClassPlace::Repeated => Some("A year older than his class: he repeated a grade before high school."),
            ClassPlace::HeldForFootball => {
                Some("A year older than his class: his family held him back in eighth grade to gain a year for football.")
            }
            ClassPlace::ReclassifiedUp => Some("Reclassified in high school to graduate a year early."),
            ClassPlace::ReclassifiedDown => Some("Reclassified in high school to repeat a year."),
        }
    }
}

/// Days in a month.
fn month_days(y: i32, m: u32) -> i32 {
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    Date::ymd(y, m, 1).days_to(Date::ymd(ny, nm, 1))
}

/// The birthday of someone who is `age` on August 31 of `season`: a day
/// from September 1 of `season - age - 1` to August 31 of `season - age`,
/// months weighted by real births times `lean` (more summer birthdays among
/// late starters). `u` is a uniform draw in [0, 1).
pub fn born_in_class(season: i32, age: u32, u: f64, lean: &[f64; 12]) -> Date {
    let first = season - age as i32 - 1;
    // The window's months in order, September first.
    let months: [(i32, u32); 12] = std::array::from_fn(|k| {
        let m = (8 + k as u32) % 12 + 1;
        (if m >= 9 { first } else { first + 1 }, m)
    });
    let weight = |&(_, m): &(i32, u32)| MONTH_SHARE[m as usize - 1] * lean[m as usize - 1];
    let total: f64 = months.iter().map(weight).sum();
    let mut left = u.clamp(0.0, 0.999_999) * total;
    for ym in &months {
        let wt = weight(ym);
        if left < wt {
            let (y, m) = *ym;
            let days = month_days(y, m);
            let d = ((left / wt) * days as f64) as i32;
            return Date::ymd(y, m, 1).add(d.clamp(0, days - 1));
        }
        left -= wt;
    }
    Date::ymd(first + 1, 8, 31)
}

/// No lean: births as they fall.
pub const EVEN: [f64; 12] = [1.0; 12];

/// Late starters are mostly summer birthdays (kindergarten "redshirting":
/// boys born June to August are several times as likely to wait a year).
const LATE_LEAN: [f64; 12] = [0.5, 0.5, 0.55, 0.7, 1.4, 2.8, 3.4, 3.6, 0.45, 0.45, 0.45, 0.5];

/// Families who hold a boy back for football favour the young ones too.
const FOOTBALL_LEAN: [f64; 12] = [0.8, 0.8, 0.9, 1.0, 1.3, 1.7, 1.9, 2.0, 0.7, 0.7, 0.75, 0.8];

/// Whether a boy entering high school is a year older than his class, and
/// why (A2). About 1 in 10 in all (Claude's pick, final build): late
/// school starts about 5% of boys (NCES: about 6% of kindergartners wait a
/// year, more of them boys with summer birthdays, more in better-off
/// areas), a repeated grade before high school about 4.5% (more in poorer
/// areas), and a football hold-back about 1% (more for the best athletes).
/// `area_money` is his school's area, 0-1; `talent_z` his talent draw.
pub fn older_for_class(seed: u64, id: u32, area_money: f64, talent_z: f64) -> ClassPlace {
    let u = key_uniform(seed ^ 0xB1B7_0A2E, id as u64);
    let late = 0.05 * (0.75 + 0.5 * area_money);
    let repeated = 0.045 * (1.45 - 0.9 * area_money);
    let football = 0.004 + 0.006 * area_money + 0.01 * (talent_z - 1.0).clamp(0.0, 2.0);
    if u < late {
        ClassPlace::LateStart
    } else if u < late + repeated {
        ClassPlace::Repeated
    } else if u < late + repeated + football {
        ClassPlace::HeldForFootball
    } else {
        ClassPlace::OnTime
    }
}

/// A boy's birthday in his class: `class_age` is the season age of an
/// on-time classmate in `season`; a boy a year older was born in the
/// window before.
pub fn birthday(seed: u64, id: u32, season: i32, class_age: u32, place: ClassPlace) -> Date {
    let u = key_uniform(seed ^ 0xB1B7_0DA7, id as u64);
    let (older, lean) = match place {
        ClassPlace::LateStart => (1, &LATE_LEAN),
        ClassPlace::Repeated => (1, &EVEN),
        ClassPlace::HeldForFootball => (1, &FOOTBALL_LEAN),
        _ => (0, &EVEN),
    };
    born_in_class(season, class_age + older, u, lean)
}

/// A birthday for someone `age` (his season age) in `season`, with no
/// class to place him in: a coach, a generated veteran, a staff member.
pub fn birthday_at(seed: u64, key: u64, season: i32, age: u32) -> Date {
    born_in_class(season, age, key_uniform(seed ^ 0xB1B7_C0AC, key), &EVEN)
}

/// His season age in `season`: his age on August 31, the eve of the school
/// and football year.
pub fn season_age(born: Date, season: i32) -> u32 {
    born.years_to(Date::ymd(season, 8, 31))
}

/// An older world file (written before the final build) has no birth dates:
/// each person, alumnus, coach and staff member gets one on load, the same
/// one every time (keyed draws). A boy the A2 draw puts a year older than
/// his class becomes so for good: his age and his rows' ages move up a
/// year, his class stays. High school reclassifications made before (their
/// notes say so) keep their class and get the matching birthday.
pub(crate) fn fill_missing(w: &mut super::World) {
    use rayon::prelude::*;
    let seed = w.options.seed as u64;
    let schools = &w.high_schools;
    w.people.par_iter_mut().filter(|p| !p.born.is_set()).for_each(|p| {
        let id = p.player.id;
        let money = schools.get(p.hs).map_or(0.5, |s| s.area_money);
        let mut place = older_for_class(seed, id, money, 0.0);
        let mut older = i32::from(place != ClassPlace::OnTime);
        let up = p.notes.iter().any(|n| n.starts_with("Reclassified to graduate a year early"));
        let down = p.notes.iter().any(|n| n.starts_with("Reclassified to repeat a year"));
        if older > 0 {
            p.player.age += 1;
            for h in p.player.history.iter_mut() {
                h.age += 1;
            }
            for h in p.player.position_history.iter_mut() {
                h.0 += 1;
            }
        }
        // A reclassification already moved his class, not his age.
        if up {
            older -= 1;
            place = ClassPlace::ReclassifiedUp;
        } else if down {
            older += 1;
            place = ClassPlace::ReclassifiedDown;
        }
        let u = key_uniform(seed ^ 0xB1B7_0DA7, id as u64);
        let lean = if place == ClassPlace::LateStart { &LATE_LEAN } else { &EVEN };
        p.born = born_in_class(p.class_of - 4, (14 + older).max(10) as u32, u, lean);
        p.old_for_class = older as i8;
        p.class_place = place as u8;
    });
    w.alumni.par_iter_mut().filter(|a| !a.born.is_set()).for_each(|a| {
        let id = a.player.as_ref().map_or(a.class_of as u32 ^ a.name.len() as u32, |p| p.id);
        let money = schools.get(a.high_school).map_or(0.5, |s| s.area_money);
        let place = older_for_class(seed, id, money, 0.0);
        a.born = birthday(seed, id, a.class_of - 4, 14, place);
    });
    let year = w.year;
    let you = w.career.as_ref().and_then(|c| c.you.born_date().map(|b| (c.coach, b)));
    for c in w.coaches.iter_mut().filter(|c| !c.born.is_set()) {
        c.born = match you {
            Some((mine, b)) if mine == c.id => b,
            _ => birthday_at(seed ^ 0xC0AC, c.id as u64, year, c.age),
        };
    }
    if let Some(career) = w.career.as_mut() {
        let key = seed ^ 0x45AF_0B0D ^ (career.coach as u64) << 20;
        for (k, a) in career.hs_staff.iter_mut().enumerate().filter(|(_, a)| !a.born.is_set()) {
            a.born = birthday_at(key, k as u64, year, a.age);
        }
    }
}
