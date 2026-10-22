//! Birth dates (final build A1, A2). Every person has a date of birth, and
//! every age the game shows is counted from it on today's date.
//!
//! A school class runs from September to August birthdays: the class of
//! 2030 is the children born 1 September 2011 to 31 August 2012 (the most
//! common US kindergarten cutoff). The engine still keeps a whole-year
//! `age` for the yearly development step; it is the age a person carries
//! through the season of a football year, so a person whose engine age is
//! `a` in the season of year `y` is born between 1 September `y - a - 1`
//! and 31 August `y - a` (he is `a` on 31 August, as the season starts).
//!
//! About 1 in 10 boys is a year older than his class (held back a grade, or
//! started kindergarten a year late), and about 1 in 100 a year younger
//! (skipped a grade or started early). Summer birthdays are held back most,
//! fall birthdays least (NCES, "academic redshirting"; Deming and Dynarski
//! 2008: about 9-10% of boys enter kindergarten a year late, mostly summer
//! births, and about 10% of high school students have repeated a grade).
//!
//! Every draw here is keyed (no random stream is consumed), so adding birth
//! dates moved nothing else in the world.

use super::date::Date;

/// Relative births per day by month, January first (CDC natality,
/// 2015-2023: a late-summer peak, a spring low).
const MONTH_RATE: [f64; 12] = [0.97, 0.98, 0.98, 0.96, 0.98, 1.01, 1.05, 1.06, 1.05, 1.00, 0.97, 0.99];

/// Chance a boy born in each month is a year older than his class (held
/// back), January first. Averages about 10% over the year.
const HELD_BACK: [f64; 12] = [0.08, 0.08, 0.08, 0.09, 0.10, 0.15, 0.17, 0.19, 0.04, 0.05, 0.06, 0.07];

/// Chance a boy born in each month is a year younger than his class.
const AHEAD: [f64; 12] = [0.005, 0.004, 0.004, 0.004, 0.004, 0.003, 0.003, 0.003, 0.022, 0.018, 0.014, 0.008];

/// A uniform draw in [0, 1) from two keys (no stream consumed).
fn key01(a: u64, b: u64) -> f64 {
    let mut x = a.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ b.wrapping_add(0xD1B5_4A32_D192_ED03);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    ((x ^ (x >> 31)) >> 11) as f64 / (1u64 << 53) as f64
}

/// A day in the class year that starts 1 September `y`, weighted by the
/// real births per month.
fn day_in_class_year(y: i32, u: f64) -> Date {
    let start = Date::ymd(y, 9, 1);
    let days = start.days_to(Date::ymd(y + 1, 9, 1));
    let weights: Vec<f64> = (0..days).map(|k| MONTH_RATE[start.add(k).month() as usize - 1]).collect();
    let total: f64 = weights.iter().sum();
    let mut left = u * total;
    for (k, w) in weights.iter().enumerate() {
        left -= w;
        if left < 0.0 {
            return start.add(k as i32);
        }
    }
    start.add(days - 1)
}

/// A birth date for someone whose engine age in the season of `season` is
/// `age`: born between 1 September `season - age - 1` and 31 August
/// `season - age`. `key` keys the draw (a player's or coach's id).
pub fn birth_date(key: u64, age: u32, season: i32) -> Date {
    day_in_class_year(season - age as i32 - 1, key01(key, 0xB127_0D47))
}

/// A birth date inside the class year of a high school class (born 1
/// September `class_of - 19` to 31 August `class_of - 18`): on time for
/// that class. Old world files fill their people's birthdays with it.
pub fn for_class(key: u64, class_of: i32) -> Date {
    day_in_class_year(class_of - 19, key01(key, 0xB127_0D47))
}

/// A new high schooler's birth date and how many years older than his class
/// he is (+1 held back, -1 a year young, 0 on time), for a class whose
/// on-time age in the season of `season` is `age`. His engine age is
/// `age + years`.
pub fn schoolboy(key: u64, age: u32, season: i32) -> (Date, i32) {
    let on_time = birth_date(key, age, season);
    let m = on_time.month() as usize - 1;
    let u = key01(key, 0x4E1D_BAC5);
    let years = if u < HELD_BACK[m] {
        1
    } else if u > 1.0 - AHEAD[m] {
        -1
    } else {
        0
    };
    (shift_years(on_time, -years), years)
}

/// The same day `years` later (29 February falls back to the 28th).
pub fn shift_years(d: Date, years: i32) -> Date {
    let (y, m, day) = d.parts();
    let day = if m == 2 && day == 29 { 28 } else { day };
    Date::ymd(y + years, m, day)
}

/// The high school class (spring of senior year) a birth date falls in when
/// on time: September to August birthdays.
pub fn class_of_birth(born: Date) -> i32 {
    let (y, m, _) = born.parts();
    if m >= 9 { y + 19 } else { y + 18 }
}

/// Whole years from `born` to `on`.
pub fn age_on(born: Date, on: Date) -> u32 {
    let (by, bm, bd) = born.parts();
    let (y, m, d) = on.parts();
    (y - by - i32::from((m, d) < (bm, bd))).max(0) as u32
}

/// Someone's age on a day from his birth date, or `fallback` (the engine's
/// whole-year age) while the date is unknown.
pub fn age_or(born: Date, on: Date, fallback: u32) -> u32 {
    if born == UNKNOWN { fallback } else { age_on(born, on) }
}

/// Marks a birth date not yet filled (a person from an older world file).
pub const UNKNOWN: Date = Date(i32::MIN);

/// serde default for a birth date field.
pub fn unknown() -> Date {
    UNKNOWN
}
