//! Calls and messages to recruits on each era's NCAA football rules (talk
//! facts lane L6, item 9; scouting topic 15's "call or message (by the
//! era's rules)"). The rules and their sources are in
//! `data/real-world/ncaa_contact_rules.csv`:
//!
//! - **Calls, to July 31, 2023**: one call to a junior from April 15
//!   through May 31; to a senior once a week from September 1, unlimited in
//!   a contact period and from the Sunday before a signing date through 48
//!   hours after it. In 2021-22 only, juniors could be called from January
//!   16, 2022: one a week in January, March, June and July, none in
//!   February, at the school's discretion in April and May.
//! - **Calls from August 1, 2023** (Proposal 2023-14): from June 15 after
//!   the sophomore year, at the school's discretion.
//! - **Messages**: from September 1 of the junior year, by email from
//!   August 1, 2007 (the text ban) and every kind again from August 1, 2016;
//!   from June 15 after the sophomore year from August 1, 2023. No limit.
//! - **Junior college and prep school players**: once a week, unlimited in
//!   a contact period (the two-year college rule; a prep school player is
//!   read the same way, proposed, not confirmed).
//!
//! The NCAA limits only what a school starts: a recruit may call a coach
//! any time. The game's own pace (one call and one message to a recruit a
//! day) is the desk's (`world::front_office::college`), not a rule here.

use super::{Date, Period, Weekday, YearDates};

/// The first day of the 2023 football recruiting model.
pub fn model_2023() -> Date {
    Date::ymd(2023, 8, 1)
}

/// The text ban, and the day every kind of message came back.
fn text_ban() -> (Date, Date) {
    (Date::ymd(2007, 8, 1), Date::ymd(2016, 8, 1))
}

/// How often you may call him under the rule of the day.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallLimit {
    /// As often as you like (a contact period, around a signing date, or
    /// the 2023 model).
    Open,
    /// One call a week (Sunday to Saturday).
    Weekly,
    /// One call in a window (a junior's April 15 to May 31).
    Once,
}

impl CallLimit {
    pub fn words(self) -> &'static str {
        match self {
            CallLimit::Open => "Calls are open: no limit.",
            CallLimit::Weekly => "One call a week.",
            CallLimit::Once => "One call from April 15 to May 31 of his junior year.",
        }
    }
}

/// What a message is in the era: a text (or a direct message), or an
/// email while texts were banned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Medium {
    Text,
    Email,
}

impl Medium {
    pub fn of(d: Date) -> Medium {
        let (ban, back) = text_ban();
        if d >= ban && d < back { Medium::Email } else { Medium::Text }
    }
    pub fn noun(self) -> &'static str {
        match self {
            Medium::Text => "text",
            Medium::Email => "email",
        }
    }
}

/// The Sunday on or before a day.
fn sunday_of(d: Date) -> Date {
    let back = match d.weekday() {
        Weekday::Sun => 0,
        Weekday::Mon => 1,
        Weekday::Tue => 2,
        Weekday::Wed => 3,
        Weekday::Thu => 4,
        Weekday::Fri => 5,
        Weekday::Sat => 6,
    };
    d.add(-back)
}

/// From the Sunday before a signing date through 48 hours after it.
fn near_signing(y: &YearDates, d: Date) -> bool {
    let near = |s: Date| d >= sunday_of(s.add(-1)) && d <= s.add(2);
    y.early_signing.is_some_and(near) || near(y.signing_day)
}

/// Whether a school may call him on day `d`, and how often: his class
/// (the year he finishes high school), whether he is still in high school
/// (a junior college or prep school player is read as one), the calls it
/// has made to him (their days) and the football year's dates (`y`, for
/// the contact periods and signing dates). Err says why not, in words.
pub fn call_rule(
    y: &YearDates,
    d: Date,
    class_of: i32,
    high_school: bool,
    calls: &[Date],
) -> Result<CallLimit, String> {
    let open = super::recruiting_period(y, d) == Period::Contact || near_signing(y, d);
    let this_week = |from: Date| calls.iter().any(|&c| c >= from && c <= d);
    let weekly = || {
        if this_week(sunday_of(d)) {
            Err("One call a week, and you've called him this week.".to_string())
        } else {
            Ok(CallLimit::Weekly)
        }
    };
    if !high_school {
        return if open { Ok(CallLimit::Open) } else { weekly() };
    }
    if d >= model_2023() {
        let opens = Date::ymd(class_of - 2, 6, 15);
        return if d >= opens {
            Ok(CallLimit::Open)
        } else {
            Err(format!("Calls open June 15 after his sophomore year ({}).", opens.pretty()))
        };
    }
    let senior = Date::ymd(class_of - 1, 9, 1);
    if d >= senior {
        return if open { Ok(CallLimit::Open) } else { weekly() };
    }
    // 2021-22 only: the juniors of the class of 2023 from January 16, 2022.
    let (y0, y1) = (Date::ymd(2022, 1, 16), Date::ymd(2022, 7, 31));
    if class_of == 2023 && d >= y0 && d <= y1 {
        return match d.month() {
            2 => Err("No calls to juniors in February this year.".into()),
            4 | 5 => Ok(CallLimit::Open),
            _ => weekly(),
        };
    }
    let (a, b) = (Date::ymd(class_of - 1, 4, 15), Date::ymd(class_of - 1, 5, 31));
    if d >= a && d <= b {
        return if calls.iter().any(|&c| c >= a && c <= b) {
            Err("One call to a junior from April 15 to May 31, and you've made it.".into())
        } else {
            Ok(CallLimit::Once)
        };
    }
    Err(if d < a {
        format!("You may call a junior once from April 15 ({}).", a.pretty())
    } else {
        format!("Calls open again on September 1 of his senior year ({}).", senior.pretty())
    })
}

/// Whether a school may send him a message on day `d`: Ok with what it is
/// in the era, Err with why not. No limit on how many.
pub fn message_rule(d: Date, class_of: i32, high_school: bool) -> Result<Medium, String> {
    let medium = Medium::of(d);
    if !high_school {
        return Ok(medium);
    }
    let (opens, words) = if d >= model_2023() {
        (Date::ymd(class_of - 2, 6, 15), "June 15 after his sophomore year")
    } else {
        (Date::ymd(class_of - 2, 9, 1), "September 1 of his junior year")
    };
    if d >= opens { Ok(medium) } else { Err(format!("Messages open {words} ({}).", opens.pretty())) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_2023_model_opens_calls_after_the_sophomore_year() {
        let y = YearDates::new(2026);
        // The class of 2029 finishes its sophomore year in June 2027.
        assert!(call_rule(&y, Date::ymd(2026, 9, 10), 2029, true, &[]).is_err());
        let y27 = YearDates::new(2027);
        assert_eq!(call_rule(&y27, Date::ymd(2027, 6, 15), 2029, true, &[]), Ok(CallLimit::Open));
        let many = [Date::ymd(2027, 6, 15), Date::ymd(2027, 6, 16)];
        assert_eq!(call_rule(&y27, Date::ymd(2027, 6, 17), 2029, true, &many), Ok(CallLimit::Open));
        assert!(message_rule(Date::ymd(2027, 6, 14), 2029, true).is_err());
        assert_eq!(message_rule(Date::ymd(2027, 6, 15), 2029, true), Ok(Medium::Text));
    }

    #[test]
    fn before_2023_a_junior_gets_one_spring_call_and_a_senior_one_a_week() {
        let y = YearDates::new(2018);
        // The class of 2019: a junior in the spring of 2018.
        assert!(call_rule(&y, Date::ymd(2018, 3, 20), 2019, true, &[]).is_err());
        assert_eq!(call_rule(&y, Date::ymd(2018, 4, 20), 2019, true, &[]), Ok(CallLimit::Once));
        assert!(call_rule(&y, Date::ymd(2018, 5, 20), 2019, true, &[Date::ymd(2018, 4, 20)]).is_err());
        assert!(call_rule(&y, Date::ymd(2018, 8, 20), 2019, true, &[]).is_err());
        // A senior from September 1: once a week in the fall's evaluation
        // period, as often as you like in the contact period.
        let tue = Date::ymd(2018, 9, 11);
        assert_eq!(call_rule(&y, tue, 2019, true, &[]), Ok(CallLimit::Weekly));
        assert!(call_rule(&y, tue.add(2), 2019, true, &[tue]).is_err());
        assert_eq!(call_rule(&y, tue.add(6), 2019, true, &[tue]), Ok(CallLimit::Weekly), "a new week");
        let contact = Date::ymd(2018, 12, 3);
        assert_eq!(call_rule(&y, contact, 2019, true, &[contact.add(-1)]), Ok(CallLimit::Open));
    }

    #[test]
    fn juniors_in_2021_22_and_the_text_ban() {
        let y = YearDates::new(2021);
        assert!(call_rule(&y, Date::ymd(2022, 2, 8), 2023, true, &[]).is_err());
        assert_eq!(call_rule(&y, Date::ymd(2022, 4, 5), 2023, true, &[Date::ymd(2022, 4, 4)]), Ok(CallLimit::Open));
        assert_eq!(message_rule(Date::ymd(2010, 10, 1), 2012, true), Ok(Medium::Email));
        assert_eq!(message_rule(Date::ymd(2017, 10, 1), 2019, true), Ok(Medium::Text));
        assert!(message_rule(Date::ymd(2017, 8, 20), 2019, true).is_err(), "before September 1 of the junior year");
    }
}
