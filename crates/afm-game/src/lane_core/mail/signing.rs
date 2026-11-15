//! The signing dates as messages (the inbox lane, rows 24 and 25 of the
//! event table): a short notice when the early signing period opens and on
//! national signing day, with how many places of your class are filled and,
//! once the services have ranked the classes, where yours stands. Nothing
//! here raises a world event: the dates are the calendar's own lines, the
//! count is read from the recruits' records, and the rank from each
//! program's class score (`Program::class_scores`).

use afm_engine::calendar::{Date, Event, Kind, Level as CalLevel, Loudness};
use afm_engine::world::data::Division;
use afm_engine::world::front_office::college::{self as desk, Standing};
use afm_engine::world::{Stage, World};

use super::super::people::Job;
use super::{PLAYERS, Pen, RECRUITING};
use crate::lane_squadboards::ordinal;
use crate::views::{self, TeamRef};

/// Which of the two dates a calendar line is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Day {
    EarlyPeriod,
    SigningDay,
}

/// The calendar line that opens the early period or is signing day.
pub(super) fn day_of(e: &Event) -> Option<Day> {
    if e.level != CalLevel::Recruiting {
        return None;
    }
    if e.kind == Kind::Window && e.rule == "NCAA: early signing" {
        return Some(Day::EarlyPeriod);
    }
    (e.kind == Kind::Date && e.text.starts_with("National signing day,")).then_some(Day::SigningDay)
}

/// The services' ranking line ("...: the services' top recruiting classes are 1. ...").
pub(super) fn ranking_line(e: &Event) -> bool {
    e.kind == Kind::Signing
        && e.loud == Loudness::Big
        && e.level == CalLevel::Recruiting
        && e.text.contains("top recruiting classes")
}

/// Lines the notice carries, so the Continue card leaves them out (college
/// seats): the ranking, which reached the card only for the top class.
pub(super) fn covered(e: &Event) -> bool {
    ranking_line(e)
}

/// The class the day signs: December's early period signs the class of the
/// coming year.
fn sign_year(d: Date) -> i32 {
    if d.month() == 12 { d.year() + 1 } else { d.year() }
}

/// Where the services place your program's class (FBS only, as they rank),
/// by the engine's own order: score, then program number.
fn rank(w: &World, pi: usize, year: i32) -> Option<(usize, usize)> {
    if w.programs[pi].division != Division::FBS {
        return None;
    }
    let score = |j: usize| w.programs[j].class_scores.iter().rev().find(|x| x.0 == year).map(|x| x.1);
    let mine = score(pi)?;
    let (mut better, mut of) = (0, 0);
    for (j, p) in w.programs.iter().enumerate() {
        if p.division != Division::FBS {
            continue;
        }
        let Some(s) = score(j) else { continue };
        of += 1;
        if s > mine || (s == mine && j < pi) {
            better += 1;
        }
    }
    Some((better + 1, of))
}

/// A college class's places: committed or signed to us, and the places there are.
fn college_class(w: &World, pi: usize, year: i32) -> (usize, Option<usize>) {
    let (mut filled, mut signed) = (0usize, 0usize);
    for p in w.people.iter().filter(|p| p.class_of == year) {
        match desk::standing(p, pi) {
            Standing::CommittedHere => filled += 1,
            Standing::SignedHere => {
                filled += 1;
                signed += 1;
            }
            _ => {}
        }
    }
    let o = desk::outlook(w, pi);
    // The plan's slots are what is left after the signees.
    (filled, (o.sign_year == year).then_some(o.slots + signed))
}

/// The notice for the day `d` (`ranked`: the services' line, linked, once it
/// is out). Written for a college seat or a high school seat; the early
/// period is a college seat's.
pub(super) fn notice(pen: &mut Pen, d: Date, day: Day, ranked: Option<&str>) {
    let w = pen.w;
    let year = sign_year(d);
    let (tag, job) = match (day, pen.me) {
        (Day::EarlyPeriod, TeamRef::College(_)) => ("signing|early", Job::Personnel),
        (Day::SigningDay, TeamRef::College(_)) => ("signing|day", Job::Personnel),
        (Day::SigningDay, TeamRef::HighSchool(_)) => ("signing|day", Job::Operations),
        _ => return,
    };
    let mut subject = match day {
        Day::EarlyPeriod => "Early signing period opens today".to_string(),
        Day::SigningDay => "National signing day is today".to_string(),
    };
    let mut body = vec![match day {
        Day::EarlyPeriod => format!(
            "{}, the early signing period opens today. Recruits who are ready can sign now; everyone else has until national signing day.",
            pen.dear
        ),
        Day::SigningDay => format!(
            "{}, national signing day is today. Anyone still unsigned decides today; the late signing days follow each Wednesday until summer enrollment.",
            pen.dear
        ),
    }];
    match pen.me {
        TeamRef::College(pi) => {
            let (n, slots) = college_class(w, pi, year);
            body.push(match slots {
                Some(s) => format!("{n} of the {s} places in the class of {year} are committed or signed to us."),
                None => format!("{n} recruits in the class of {year} are committed or signed to us."),
            });
            // December's score is overwritten in February: the early notice
            // keeps it only while its day is fresh.
            let fresh = day == Day::SigningDay || w.today <= d.add(1);
            if let (true, Some(line), Some((r, of))) = (fresh, ranked, rank(w, pi, year)) {
                subject = format!("{subject}: our class ranks {}", ordinal(r as u32));
                body.push(format!("The services rank our class {} of {of} FBS classes.", ordinal(r as u32)));
                body.push(line.to_string());
            }
        }
        TeamRef::HighSchool(_) => {
            let seniors: Vec<usize> =
                views::roster_people(w, pen.me).into_iter().filter(|&k| w.people[k].class_of == year).collect();
            let offered = seniors
                .iter()
                .filter(|&&k| !w.people[k].cal.offers.is_empty() || !w.people[k].cal.early_offers.is_empty())
                .count();
            let decided = seniors
                .iter()
                .filter(|&&k| {
                    let p = &w.people[k];
                    p.stage != Stage::HighSchool || p.cal.commit.is_some() || p.cal.signed.is_some()
                })
                .count();
            body.push(format!(
                "Of our {} seniors, {offered} hold offers and {decided} are committed or have signed.",
                seniors.len()
            ));
        }
        _ => return,
    }
    let mut m = pen.memo(tag, d, 7 * 60, job, subject);
    m.body = body;
    m.keys = RECRUITING | PLAYERS;
    m.loud = 2;
    pen.out.push(m);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(kind: Kind, level: CalLevel, loud: Loudness, rule: &'static str, text: &str) -> Event {
        let mut e = Event::new(Date::ymd(2026, 12, 3), level, kind, loud, text);
        e.rule = rule;
        e
    }

    #[test]
    fn the_signing_dates_are_read_from_the_calendars_own_lines() {
        let early = line(
            Kind::Window,
            CalLevel::Recruiting,
            Loudness::Big,
            "NCAA: early signing",
            "The early signing period opens Wed 3 Dec.",
        );
        let day = line(Kind::Date, CalLevel::Recruiting, Loudness::Big, "", "National signing day, Wed 4 Feb.");
        let other = line(Kind::Window, CalLevel::Recruiting, Loudness::Big, "NCAA: contact", "A contact period opens.");
        assert_eq!(day_of(&early), Some(Day::EarlyPeriod));
        assert_eq!(day_of(&day), Some(Day::SigningDay));
        assert_eq!(day_of(&other), None);
        let rank = line(
            Kind::Signing,
            CalLevel::Recruiting,
            Loudness::Big,
            "",
            "National signing day: the services' top recruiting classes are 1. A (20 signees), 2. B (19 signees), 3. C (18 signees).",
        );
        assert!(ranking_line(&rank) && covered(&rank));
        assert!(!ranking_line(&day));
    }

    #[test]
    fn december_signs_next_years_class() {
        assert_eq!(sign_year(Date::ymd(2026, 12, 3)), 2027);
        assert_eq!(sign_year(Date::ymd(2027, 2, 4)), 2027);
    }
}
