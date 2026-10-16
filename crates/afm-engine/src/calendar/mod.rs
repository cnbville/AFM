//! Section 4: the season calendar and pacing. Spec: `docs/design/calendar.md`
//! (the 11 agreed topics), `docs/design/calendar-additions.md` (A1-A46) and
//! the build plan `docs/design/calendar-build-plan.md`.
//!
//! This module holds the pure calendar: dates and real anchors per year, the
//! rule books by era and the 12 agreed off-field changes, events with their
//! loudness, pending answers, weather from climate, and the recruiting
//! periods. The world (`world::clock`) runs on it one day at a time.

pub mod anchors;
pub mod data;
pub mod date;
pub mod events;
pub mod rules;
pub mod weather;

pub use anchors::{CfbPostseason, YearDates};
pub use date::{Date, Moment, Weekday};
pub use events::{Calendar, Event, Kind, Level, Loudness, Pending};
pub use rules::{Change, ChangeState, RuleBook};

/// The NCAA's FBS recruiting periods (topic 5): what college coaches may do
/// with a recruit on a given day.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Period {
    /// In-person contact allowed, on and off campus.
    Contact,
    /// Coaches may watch (practices, games) but not talk off campus.
    Evaluation,
    /// Contact only on the college campus.
    Quiet,
    /// No in-person contact at all.
    Dead,
}

impl Period {
    pub fn name(self) -> &'static str {
        match self {
            Period::Contact => "contact period",
            Period::Evaluation => "evaluation period",
            Period::Quiet => "quiet period",
            Period::Dead => "dead period",
        }
    }
    /// Unofficial visits (paid by the family) happen outside dead periods.
    pub fn unofficial_visits(self) -> bool {
        self != Period::Dead
    }
    /// Official visits (paid by the school): contact or quiet periods.
    pub fn official_visits(self) -> bool {
        matches!(self, Period::Contact | Period::Quiet)
    }
}

/// The FBS recruiting period on a date of football year `y` (modelled on the
/// 2025-26 calendar: quiet Mar 2-Apr 14, contact Apr 15-30, quiet in May,
/// dead May 24-27, quiet May 28-Jun 21, dead Jun 22-Jul 31; the fall
/// evaluation period, contact from late November, dead around both signing
/// days and over the holidays).
pub fn recruiting_period(y: &YearDates, d: Date) -> Period {
    let (yr, m, day) = d.parts();
    let s = y.season;
    let in_dead = |a: Date, b: Date| d >= a && d <= b;
    // Dead around signing days.
    let nsd_prev = Date::nth_weekday(s, 2, Weekday::Wed, 1);
    if in_dead(nsd_prev.add(-3), nsd_prev) || in_dead(y.signing_day.add(-3), y.signing_day) {
        return Period::Dead;
    }
    if let Some(esp) = y.early_signing
        && in_dead(esp.add(-2), esp.add(2))
    {
        return Period::Dead;
    }
    if yr == s {
        match (m, day) {
            (2, _) | (3, _) => Period::Quiet,
            (4, 1..=14) => Period::Quiet,
            (4, _) => Period::Contact,
            (5, 24..=27) => Period::Dead,
            (5, _) => Period::Quiet,
            (6, 1..=21) => Period::Quiet,
            (6, _) | (7, _) => Period::Dead,
            (8, _) => Period::Quiet,
            (9, _) | (10, _) => Period::Evaluation,
            (11, 1..=29) => Period::Evaluation,
            (11, _) => Period::Contact,
            (12, 1..=14) => Period::Contact,
            (12, _) => Period::Dead,
            _ => Period::Quiet,
        }
    } else {
        match (m, day) {
            (1, 1) => Period::Dead,
            (1, _) => Period::Contact,
            _ => Period::Quiet,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recruiting_periods_follow_the_2025_26_calendar() {
        let y = YearDates::new(2026);
        assert_eq!(recruiting_period(&y, Date::ymd(2026, 3, 10)), Period::Quiet);
        assert_eq!(recruiting_period(&y, Date::ymd(2026, 4, 20)), Period::Contact);
        assert_eq!(recruiting_period(&y, Date::ymd(2026, 5, 25)), Period::Dead);
        assert_eq!(recruiting_period(&y, Date::ymd(2026, 6, 10)), Period::Quiet);
        assert_eq!(recruiting_period(&y, Date::ymd(2026, 7, 4)), Period::Dead);
        assert_eq!(recruiting_period(&y, Date::ymd(2026, 10, 2)), Period::Evaluation);
        assert_eq!(recruiting_period(&y, Date::ymd(2026, 12, 2)), Period::Dead, "early signing");
        assert_eq!(recruiting_period(&y, Date::ymd(2026, 12, 25)), Period::Dead);
        assert_eq!(recruiting_period(&y, Date::ymd(2027, 1, 12)), Period::Contact);
        assert_eq!(recruiting_period(&y, Date::ymd(2027, 2, 3)), Period::Dead, "signing day");
    }
}
