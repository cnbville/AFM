//! Time zones and the six-part day (player immersion I1,
//! `docs/design/day-sections.md`).
//!
//! The world's moments stay Eastern (`date::Moment` is Eastern wall-clock
//! time). The world's day runs from 5:00 AM to 5:00 AM Eastern, so a "day
//! minute" is minutes after 5:00 AM Eastern of the world's day: 0 is 5:00
//! AM, 660 is 4:00 PM, 1140 is midnight and 1439 is 4:59 AM the next
//! calendar day.
//!
//! A seat lives on its own local time: its six sections start at 5 AM,
//! 8 AM, 1 PM, 4 PM, 7 PM and 10 PM where it is (Cain, 2026-10-30 and
//! 2026-11-04), and every time on its screens shows local. A Seattle
//! coach's 5 AM stop is 8 AM Eastern; Arizona keeps standard time all year,
//! three hours behind Eastern in summer and two in winter.

use super::date::{Date, Moment, Weekday};

/// Minutes in the world's day (5 AM to 5 AM).
pub const DAY: u16 = 24 * 60;

/// The six sections' first hours, local (S1-S6).
pub const SECTION_HOURS: [u16; 6] = [5, 8, 13, 16, 19, 22];

/// A day minute for an Eastern clock time (hour 0-23): 5:00 AM is 0, 4:00
/// PM is 660, midnight is 1140, 4:59 AM is 1439.
pub const fn et(hour: u16, minute: u16) -> u16 {
    ((hour + 19) % 24) * 60 + minute
}

/// The Eastern moment of day minute `m` of world day `d` (past midnight it
/// is the next calendar date).
pub fn moment(d: Date, m: u16) -> Moment {
    Moment { date: d, minute: 5 * 60 }.add_minutes(m as i32)
}

/// Where an Eastern moment falls in the world's day `d`: its day minute,
/// or None when it is outside that day.
pub fn day_minute(d: Date, at: Moment) -> Option<u16> {
    let start = Moment { date: d, minute: 5 * 60 };
    let mins = minutes_between(start, at);
    (0..DAY as i32).contains(&mins).then_some(mins as u16)
}

/// The world day an Eastern moment belongs to (before 5 AM it is the day
/// before) and its day minute.
pub fn world_day(at: Moment) -> (Date, u16) {
    if at.minute < 5 * 60 { (at.date.add(-1), at.minute + 19 * 60) } else { (at.date, at.minute - 5 * 60) }
}

/// Minutes from `a` to `b`.
pub fn minutes_between(a: Moment, b: Moment) -> i32 {
    (b.date.0 - a.date.0) * 24 * 60 + b.minute as i32 - a.minute as i32
}

/// Whether US daylight saving time is in effect on `d` (its daytime): the
/// second Sunday of March to the first Sunday of November from 2007, the
/// first Sunday of April to the last Sunday of October 1987-2006, the last
/// Sunday of April before.
pub fn daylight(d: Date) -> bool {
    let y = d.year();
    let (from, to) = if y >= 2007 {
        (Date::nth_weekday(y, 3, Weekday::Sun, 2), Date::nth_weekday(y, 11, Weekday::Sun, 1))
    } else if y >= 1987 {
        (Date::nth_weekday(y, 4, Weekday::Sun, 1), Date::last_weekday(y, 10, Weekday::Sun))
    } else {
        (Date::last_weekday(y, 4, Weekday::Sun), Date::last_weekday(y, 10, Weekday::Sun))
    };
    d >= from && d < to
}

/// A US time zone as a seat lives it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Zone {
    #[default]
    Eastern,
    Central,
    Mountain,
    /// Arizona: Mountain standard time all year (no daylight time).
    Arizona,
    Pacific,
    Alaska,
    /// Hawaii: standard time all year.
    Hawaii,
}

impl Zone {
    /// Hours behind Eastern on day `d`.
    pub fn behind(self, d: Date) -> i32 {
        match self {
            Zone::Eastern => 0,
            Zone::Central => 1,
            Zone::Mountain => 2,
            Zone::Arizona => {
                if daylight(d) {
                    3
                } else {
                    2
                }
            }
            Zone::Pacific => 3,
            Zone::Alaska => 4,
            Zone::Hawaii => {
                if daylight(d) {
                    6
                } else {
                    5
                }
            }
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Zone::Eastern => "Eastern",
            Zone::Central => "Central",
            Zone::Mountain => "Mountain",
            Zone::Arizona => "Arizona",
            Zone::Pacific => "Pacific",
            Zone::Alaska => "Alaska",
            Zone::Hawaii => "Hawaii",
        }
    }

    /// A state's main zone.
    pub fn of_state(code: &str) -> Zone {
        match code {
            "AL" | "AR" | "IL" | "IA" | "KS" | "LA" | "MN" | "MS" | "MO" | "NE" | "ND" | "OK" | "SD" | "TX" | "WI"
            | "TN" => Zone::Central,
            "CO" | "ID" | "MT" | "NM" | "UT" | "WY" => Zone::Mountain,
            "AZ" => Zone::Arizona,
            "CA" | "NV" | "OR" | "WA" => Zone::Pacific,
            "AK" => Zone::Alaska,
            "HI" => Zone::Hawaii,
            _ => Zone::Eastern,
        }
    }

    /// A place's zone: its state's, with the split states read by town
    /// (East Tennessee, the Florida panhandle, western Kentucky, the corners
    /// of Indiana, the Upper Peninsula's west, El Paso, the western Plains,
    /// the Idaho panhandle, eastern Oregon), by its place when known.
    pub fn of_place(code: &str, town: &str, place: Option<(f64, f64)>) -> Zone {
        let base = Zone::of_state(code);
        if let Some(z) = TOWNS.iter().find(|t| t.0 == code && t.1.eq_ignore_ascii_case(town.trim())).map(|t| t.2) {
            return z;
        }
        let Some((lat, lon)) = place.filter(|p| p.0 != 0.0 || p.1 != 0.0) else { return base };
        match code {
            "TN" if lon > -85.0 || (lon > -85.45 && lat < 35.25) => Zone::Eastern,
            "FL" if lon < -85.0 => Zone::Central,
            "KY" if lon < -85.95 => Zone::Central,
            "IN" if (lat > 41.2 && lon < -86.8) || (lat < 38.4 && lon < -86.9) => Zone::Central,
            "MI" if lon < -87.6 && lat > 45.3 => Zone::Central,
            "TX" if lon < -104.9 => Zone::Mountain,
            "KS" if lon < -101.5 => Zone::Mountain,
            "NE" if lon < -101.0 => Zone::Mountain,
            "ND" if lon < -101.0 && lat < 47.3 => Zone::Mountain,
            "SD" if lon < -100.6 => Zone::Mountain,
            "ID" if lat > 45.6 => Zone::Pacific,
            "OR" if lon > -117.6 && lat < 44.6 => Zone::Mountain,
            _ => base,
        }
    }

    /// An Eastern moment on this zone's clock (local wall-clock time).
    pub fn local(self, at: Moment) -> Moment {
        at.add_minutes(-60 * self.behind(at.date))
    }

    /// A local wall-clock moment on Eastern clocks.
    pub fn to_eastern(self, local: Moment) -> Moment {
        local.add_minutes(60 * self.behind(local.date))
    }

    /// The day minutes of the six section starts of local date `d`, on
    /// world day `d` (every US zone's six fall inside it).
    pub fn stops(self, d: Date) -> [u16; 6] {
        let b = self.behind(d) as u16;
        SECTION_HOURS.map(|h| (h + b - 5) * 60)
    }

    /// Which section (0-5) a local clock minute (after midnight) is in.
    pub fn section_of(local_minute: u16) -> usize {
        let h = local_minute / 60;
        match h {
            5..=7 => 0,
            8..=12 => 1,
            13..=15 => 2,
            16..=18 => 3,
            19..=21 => 4,
            _ => 5,
        }
    }
}

/// Places in split states that a town's position would read wrongly, and
/// the college and pro towns there.
const TOWNS: &[(&str, &str, Zone)] = &[
    ("TN", "Knoxville", Zone::Eastern),
    ("TN", "Chattanooga", Zone::Eastern),
    ("TN", "Johnson City", Zone::Eastern),
    ("TN", "Jefferson City", Zone::Eastern),
    ("TN", "Maryville", Zone::Eastern),
    ("TN", "Harrogate", Zone::Eastern),
    ("TN", "Greeneville", Zone::Eastern),
    ("TN", "Tusculum", Zone::Eastern),
    ("TN", "Cleveland", Zone::Eastern),
    ("TN", "Athens", Zone::Eastern),
    ("TN", "Kingsport", Zone::Eastern),
    ("TN", "Bristol", Zone::Eastern),
    ("FL", "Pensacola", Zone::Central),
    ("FL", "Panama City", Zone::Central),
    ("KY", "Bowling Green", Zone::Central),
    ("KY", "Murray", Zone::Central),
    ("KY", "Owensboro", Zone::Central),
    ("KY", "Paducah", Zone::Central),
    ("KY", "Hopkinsville", Zone::Central),
    ("IN", "Evansville", Zone::Central),
    ("IN", "Valparaiso", Zone::Central),
    ("IN", "Hammond", Zone::Central),
    ("IN", "Gary", Zone::Central),
    ("TX", "El Paso", Zone::Mountain),
    ("ND", "Dickinson", Zone::Mountain),
    ("SD", "Rapid City", Zone::Mountain),
    ("SD", "Spearfish", Zone::Mountain),
    ("NE", "Chadron", Zone::Mountain),
    ("NE", "Scottsbluff", Zone::Mountain),
    ("ID", "Moscow", Zone::Pacific),
    ("ID", "Lewiston", Zone::Pacific),
    ("ID", "Coeur d'Alene", Zone::Pacific),
    ("OR", "Ontario", Zone::Mountain),
];

/// "4:00 PM" for a clock minute after midnight.
pub fn clock(minute: u16) -> String {
    let h = minute / 60 % 24;
    let m = minute % 60;
    let (h12, ap) = match h {
        0 => (12, "AM"),
        1..=11 => (h, "AM"),
        12 => (12, "PM"),
        _ => (h - 12, "PM"),
    };
    format!("{h12}:{m:02} {ap}")
}

/// "4 PM" for a section's first hour, "4:30 PM" off the hour.
pub fn short_clock(minute: u16) -> String {
    if minute.is_multiple_of(60) { clock(minute).replace(":00", "") } else { clock(minute) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_minutes_run_five_to_five() {
        assert_eq!(et(5, 0), 0);
        assert_eq!(et(16, 0), 660);
        assert_eq!(et(0, 0), 1140);
        assert_eq!(et(4, 59), 1439);
        let d = Date::ymd(2026, 10, 30);
        assert_eq!(moment(d, 1200), Moment::at(d.add(1), 1, 0));
        assert_eq!(day_minute(d, Moment::at(d.add(1), 1, 0)), Some(1200));
        assert_eq!(day_minute(d, Moment::at(d, 4, 0)), None);
        assert_eq!(world_day(Moment::at(d.add(1), 1, 0)), (d, 1200));
    }

    #[test]
    fn zones_follow_daylight_time() {
        let summer = Date::ymd(2026, 7, 1);
        let winter = Date::ymd(2026, 12, 1);
        assert_eq!(Zone::Pacific.behind(summer), 3);
        assert_eq!(Zone::Pacific.behind(winter), 3);
        assert_eq!(Zone::Arizona.behind(summer), 3);
        assert_eq!(Zone::Arizona.behind(winter), 2);
        // 2026: 8 March to 1 November.
        assert!(!daylight(Date::ymd(2026, 3, 7)));
        assert!(daylight(Date::ymd(2026, 3, 8)));
        assert!(daylight(Date::ymd(2026, 10, 31)));
        assert!(!daylight(Date::ymd(2026, 11, 1)));
        // A Pacific seat's stops: 8 AM, 11 AM, 4 PM, 7 PM, 10 PM and 1 AM Eastern.
        assert_eq!(Zone::Pacific.stops(summer), [180, 360, 660, 840, 1020, 1200]);
        assert_eq!(Zone::Eastern.stops(summer), [0, 180, 480, 660, 840, 1020]);
        assert_eq!(Zone::Hawaii.stops(summer)[5], 1380);
        assert_eq!(Zone::of_place("TN", "Knoxville", None), Zone::Eastern);
        assert_eq!(Zone::of_place("TN", "Nashville", None), Zone::Central);
        assert_eq!(Zone::of_place("FL", "Pensacola", None), Zone::Central);
        assert_eq!(Zone::of_place("FL", "Tallahassee", Some((30.44, -84.28))), Zone::Eastern);
        assert_eq!(Zone::of_place("TN", "Chattanooga", Some((35.05, -85.31))), Zone::Eastern);
        assert_eq!(clock(16 * 60), "4:00 PM");
        assert_eq!(short_clock(19 * 60), "7 PM");
        assert_eq!(Zone::section_of(16 * 60), 3);
        assert_eq!(Zone::section_of(2 * 60), 5);
    }
}
