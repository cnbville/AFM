//! Rule trouble as letters (the inbox lane, rows 61 and 62 of the event
//! table): the NCAA's penalty letter from your athletic director and the
//! compliance office's notice when your own practice week was found out.
//! Nothing here raises a world event: the penalty is a news item the NCAA
//! step already wrote (`NewsKind::NcaaPenalty`) with its terms on the
//! program's own record (`ProgramEra`); the practice case is read from the
//! training desk's list (`SeatTraining::cases`).

use afm_engine::calendar::{Date, Link, Moment};
use afm_engine::world::news::NewsItem;

use super::super::links::Linker;
use super::{ABOUT_TEAM, MONEY, Pen, team_code};
use crate::views::TeamRef;

/// The NCAA's letter about a case against your program, filed at 2 PM
/// Eastern on `d`, after the NCAA's 1 PM step.
pub(super) fn ncaa(pen: &mut Pen, d: Date, n: &NewsItem) {
    let w = pen.w;
    let TeamRef::College(pi) = pen.me else { return };
    let era = &w.programs[pi].era;
    let at = w.local(Moment::at(d, 14, 0));
    let linker = Linker::new(w, &[n.text.as_str()], false);
    let text = linker.link_refs(&n.text, &n.refs, &[TeamRef::College(pi)]);
    let show_cause = n.text.contains("show-cause");
    let coach = n.refs.iter().find_map(|r| match r.to {
        Link::Coach(c) if (c as usize) < w.coaches.len() => Some(c as usize),
        _ => None,
    });
    // The terms are read against the coming season: a case raised this year
    // starts next season.
    let banned = era.ban_until >= w.dates.season;
    let mut m = pen.boss_letter(&format!("ncaa|{pi}"), at.date, at.minute, "NCAA penalties".into());
    m.body = vec![format!("{},", pen.dear), text];
    if era.cut > 0 && era.cut_until >= w.dates.season {
        m.body.push(format!("The scholarship cut runs through the {} season.", era.cut_until));
    }
    if banned {
        m.body.push(format!("The football program is out of the postseason through the {} season.", era.ban_until));
    }
    if show_cause && let Some(c) = coach {
        m.body.push(format!("The show-cause order covers {}.", super::super::links::coach(c, &w.coaches[c].name)));
    }
    m.body.push("We will comply and appeal where we can. Let's talk this week.".into());
    m.keys |= MONEY;
    m.loud = if banned || show_cause { 3 } else { 2 };
    m.about = Some((ABOUT_TEAM, team_code(pen.me)));
    m.count = 1;
    pen.out.push(m);
}

/// A notice for each practice case dated in `from..=to`: how it came out
/// and what it cost. Filed at 1 PM Eastern, since the desk keeps the date
/// only.
pub(super) fn practice_cases(pen: &mut Pen, from: Date, to: Date) {
    let w = pen.w;
    for (k, c) in w.training.cases.iter().enumerate().rev() {
        if c.date < from {
            // Cases are kept in the order they came.
            break;
        }
        if c.date > to {
            continue;
        }
        let short = c.what.first().map_or("", |s| s.trim_end_matches('.'));
        let subject =
            if short.is_empty() { "Practice rules: found out".to_string() } else { format!("Practice rules: {short}") };
        let mut m = pen.notice(&format!("practicecase|{k}"), c.date, subject);
        m.minute = w.local(Moment::at(c.date, 13, 0)).minute;
        m.body = vec![format!("{},", pen.dear), format!("We were found out: {}.", c.how.trim_end_matches('.'))];
        m.body.extend(c.what.iter().map(|x| format!("{}.", x.trim_end_matches('.'))));
        m.body.push(format!("The ruling: {}.", c.penalty.trim_end_matches('.')));
        m.loud = 3;
        m.count = 1;
        pen.out.push(m);
    }
}
