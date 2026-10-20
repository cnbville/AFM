//! Calendar dates and times of day (US Eastern, the league's clock).
//!
//! A date is a day count from 1970-01-01, so adding days and comparing dates
//! is plain arithmetic. Times of day are minutes after midnight Eastern: the
//! NFL sets its deadlines in Eastern time (4pm ET for the new league year,
//! the trade deadline and cutdown day).

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub struct Date(pub i32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub enum Weekday {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

impl Weekday {
    pub const ALL: [Weekday; 7] =
        [Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri, Weekday::Sat, Weekday::Sun];
    pub fn index(self) -> i32 {
        self as i32
    }
    pub fn short(self) -> &'static str {
        ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"][self as usize]
    }
}

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

impl Date {
    /// The date for a year, month (1-12) and day (1-31).
    pub fn ymd(y: i32, m: u32, d: u32) -> Date {
        // Howard Hinnant's days_from_civil.
        let y = if m <= 2 { y - 1 } else { y };
        let era = if y >= 0 { y } else { y - 399 } / 400;
        let yoe = y - era * 400;
        let m = m as i32;
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d as i32 - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        Date(era * 146_097 + doe - 719_468)
    }

    /// (year, month 1-12, day 1-31).
    pub fn parts(self) -> (i32, u32, u32) {
        let z = self.0 + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        (if m <= 2 { y + 1 } else { y }, m, d)
    }

    pub fn year(self) -> i32 {
        self.parts().0
    }
    pub fn month(self) -> u32 {
        self.parts().1
    }
    pub fn day(self) -> u32 {
        self.parts().2
    }

    pub fn weekday(self) -> Weekday {
        // 1970-01-01 was a Thursday.
        Weekday::ALL[((self.0 + 3).rem_euclid(7)) as usize]
    }

    pub fn add(self, days: i32) -> Date {
        Date(self.0 + days)
    }

    /// Days from `self` to `other` (positive when `other` is later).
    pub fn days_to(self, other: Date) -> i32 {
        other.0 - self.0
    }

    /// The next `wd` on or after this date.
    pub fn on_or_after(self, wd: Weekday) -> Date {
        let diff = (wd.index() - self.weekday().index()).rem_euclid(7);
        self.add(diff)
    }

    /// The last `wd` on or before this date.
    pub fn on_or_before(self, wd: Weekday) -> Date {
        let diff = (self.weekday().index() - wd.index()).rem_euclid(7);
        self.add(-diff)
    }

    /// The `n`th (1-based) weekday `wd` of a month.
    pub fn nth_weekday(y: i32, m: u32, wd: Weekday, n: u32) -> Date {
        Date::ymd(y, m, 1).on_or_after(wd).add(7 * (n as i32 - 1))
    }

    /// The last weekday `wd` of a month.
    pub fn last_weekday(y: i32, m: u32, wd: Weekday) -> Date {
        let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
        Date::ymd(ny, nm, 1).add(-1).on_or_before(wd)
    }

    /// Day of the year, 0-365, for climate lookups.
    pub fn day_of_year(self) -> u32 {
        (self.0 - Date::ymd(self.year(), 1, 1).0) as u32
    }

    /// "Sat Sep 5, 2026".
    pub fn pretty(self) -> String {
        let (y, m, d) = self.parts();
        format!("{} {} {d}, {y}", self.weekday().short(), MONTHS[m as usize - 1])
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (y, m, d) = self.parts();
        write!(f, "{y:04}-{m:02}-{d:02}")
    }
}

/// A moment: a date and minutes after midnight Eastern.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub struct Moment {
    pub date: Date,
    pub minute: u16,
}

impl Moment {
    pub fn at(date: Date, hour: u16, minute: u16) -> Moment {
        Moment { date, minute: hour * 60 + minute }
    }
    pub fn add_minutes(self, mins: i32) -> Moment {
        let total = self.minute as i32 + mins;
        let days = total.div_euclid(24 * 60);
        Moment { date: self.date.add(days), minute: total.rem_euclid(24 * 60) as u16 }
    }
    /// "4:00pm ET".
    pub fn clock(self) -> String {
        let h = self.minute / 60;
        let m = self.minute % 60;
        let (h12, ap) = match h {
            0 => (12, "am"),
            1..=11 => (h, "am"),
            12 => (12, "pm"),
            _ => (h - 12, "pm"),
        };
        format!("{h12}:{m:02}{ap} ET")
    }
}

impl fmt::Display for Moment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.date, self.clock())
    }
}
