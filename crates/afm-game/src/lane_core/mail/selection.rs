//! Selection day as a letter to a college seat (the inbox lane, row 51 of
//! the event table): the bowl or playoff place the committee gave you, or
//! the word that there is none. Nothing here raises a world event: the day
//! is the calendar's own line ("Selection day: ..." for the FBS, "The FCS
//! playoff field is set ..." below it), the place is read from your
//! schedule's games dated after it, and the seed from the order of the
//! schools that line links.

use afm_engine::calendar::{Date, Event, Kind, Link, Moment};
use afm_engine::world::World;

use super::super::links;
use super::super::people::Job;
use super::{ABOUT_TEAM, Pen, team_code};
use crate::views::TeamRef;

/// Whether this calendar line is the day's announcement for a program of
/// `division` (the FBS has its own line; the lower divisions each have one).
pub(super) fn is_day(e: &Event, division: afm_engine::world::data::Division) -> bool {
    use afm_engine::world::data::Division;
    if e.kind != Kind::Date {
        return false;
    }
    if division == Division::FBS {
        return e.text.starts_with("Selection day:");
    }
    e.text.starts_with(&format!("The {} playoff field is set", division.name()))
}

/// The seed a top-four team holds: the order the line links the schools in.
fn seed_of(e: &Event, pi: usize) -> Option<usize> {
    let mut seen = 0;
    for r in &e.refs {
        if let Link::Program(p) = r.to {
            seen += 1;
            if p as usize == pi {
                return Some(seen);
            }
        }
    }
    None
}

/// "No. 5 Ohio State" or the school alone, from the latest poll out by `d`.
fn ranked(w: &World, d: Date, t: usize) -> String {
    let polls = w.poll_weeks(false);
    let poll = polls
        .iter()
        .rev()
        .filter(|p| p.date <= d)
        .find(|p| p.kind == "CFP rankings")
        .or_else(|| polls.iter().rev().filter(|p| p.date <= d).find(|p| p.kind == "AP poll"));
    let name = links::team(w, TeamRef::College(t));
    match poll.and_then(|p| p.top.iter().position(|&x| x == t)) {
        Some(i) => format!("No. {} {name}", i + 1),
        None => name,
    }
}

/// The letter of selection day `d` for the announcement `e`.
pub(super) fn notice(pen: &mut Pen, d: Date, e: &Event) {
    let w = pen.w;
    let TeamRef::College(pi) = pen.me else { return };
    let division = w.programs[pi].division;
    // Filed at 1 PM Eastern, after the committee's noon step.
    let at = w.local(Moment::at(d, 13, 0));
    let seed = seed_of(e, pi);
    let game = w
        .college_fixtures_in(Some(pi), false)
        .into_iter()
        .filter(|g| g.date > d && !g.cancelled)
        .min_by_key(|g| g.date);
    let banned = w.programs[pi].era.ban_until >= w.dates.season;
    let tag = format!("selection|{}", d.0);
    let dear = pen.dear.clone();
    let mut m;
    match (game, seed, banned) {
        (Some(g), _, _) => {
            let (opp, away) = if g.home == pi { (g.away, false) } else { (g.home, true) };
            let place = g.name.map_or_else(|| format!("the {} playoff", division.name()), |n| format!("the {n}"));
            let subject = match seed {
                Some(s) => format!("Selection day: seed No. {s} in the playoff"),
                None => format!("Selection day: we are in {place}"),
            };
            m = pen.boss_letter(&tag, at.date, at.minute, subject);
            m.body = vec![format!("{dear},")];
            m.body.push(match seed {
                Some(s) => format!("The committee made us the No. {s} seed. We play {place}."),
                None => format!("We have a place in {place}. The committee called this morning."),
            });
            m.body.push(format!(
                "{} {}, {}. We go in as {}.",
                if away { "We play at" } else { "We host" },
                ranked(w, d, opp),
                w.when_text(Moment::at(g.date, g.kickoff.0, g.kickoff.1)),
                ranked(w, d, pi),
            ));
            m.body.push("Practice time for the postseason is yours to plan. Enjoy the week.".into());
            m.loud = if seed.is_some() { 3 } else { 2 };
        }
        (None, _, true) => {
            m = pen.boss_letter(&tag, at.date, at.minute, "Selection day: a postseason ban stands".into());
            m.body = vec![
                format!("{dear},"),
                "Our postseason ban stands, so there is no bowl or playoff place for us this year. Our compliance office has the terms.".into(),
            ];
            m.loud = 1;
        }
        (None, Some(s), false) => {
            m = pen.boss_letter(&tag, at.date, at.minute, format!("Selection day: seed No. {s} in the playoff"));
            m.body = vec![
                format!("{dear},"),
                format!(
                    "The committee made us the No. {s} seed. We have a first-round bye and open the playoff in the second round."
                ),
            ];
            m.loud = 3;
        }
        // A lower division's bye cannot be told from a miss: say nothing.
        (None, None, false) if division != afm_engine::world::data::Division::FBS => return,
        (None, None, false) => {
            m = pen.memo(&tag, at.date, at.minute, Job::Operations, "Selection day: no postseason this year".into());
            m.body = vec![
                format!("{dear}, the committees have picked their fields and we are not in one."),
                "The staff turns to the portal and recruiting.".into(),
            ];
            m.loud = 1;
        }
    }
    m.about = Some((ABOUT_TEAM, team_code(pen.me)));
    m.count = 1;
    pen.out.push(m);
}

#[cfg(test)]
mod tests {
    use super::*;
    use afm_engine::calendar::Level as CalLevel;
    use afm_engine::calendar::Loudness;
    use afm_engine::world::data::Division;

    #[test]
    fn the_seed_is_the_order_the_line_links_the_schools_in() {
        let text = "Selection day: 40 bowls filled. The playoff field is set (12 teams; top seeds Ohio State, Texas, Oregon, Georgia).";
        let mut e = Event::new(Date::ymd(2026, 12, 6), CalLevel::College, Kind::Date, Loudness::Big, text);
        for (p, n) in [(11, "Ohio State"), (22, "Texas"), (33, "Oregon"), (44, "Georgia")] {
            e = e.school_link(p, n);
        }
        assert_eq!(seed_of(&e, 33), Some(3));
        assert_eq!(seed_of(&e, 99), None);
        assert!(is_day(&e, Division::FBS));
        assert!(!is_day(&e, Division::FCS));
    }

    #[test]
    fn a_lower_division_has_its_own_line() {
        let e = Event::new(
            Date::ymd(2026, 11, 22),
            CalLevel::College,
            Kind::Date,
            Loudness::Small,
            "The FCS playoff field is set: 24 teams, top seed North Dakota State.",
        );
        assert!(is_day(&e, Division::FCS));
        assert!(!is_day(&e, Division::D2));
    }
}
