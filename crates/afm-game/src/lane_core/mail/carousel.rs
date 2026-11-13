//! "Coaching carousel": the Monday memo from the chief of staff on the
//! head-coaching moves around your division in the past week: firings,
//! departures, new hires and extensions (the inbox lane, rows 64-66 of the
//! event table). Nothing here raises a world event: each row is a news item
//! the staff market already wrote (`w.news`: `CoachFired`, `CoachLeft`,
//! `CoachHired`, `CoachExtended`), linked from the ids it stores.

use afm_engine::calendar::Date;
use afm_engine::world::World;
use afm_engine::world::news::{NewsItem, NewsKind};

use super::super::links::{self, Linker};
use super::super::people::Job;
use super::{Pen, STAFF};
use crate::views::TeamRef;

/// Rows a memo shows; the rest are a count.
const ROWS: usize = 12;

/// A move that is not the head coach's (an assistant hired from a school).
fn assistant(n: &NewsItem) -> bool {
    n.kind == NewsKind::CoachHired && n.text.contains(" as ")
}

/// Which row of the table a news item is: fired, left, hired.
fn rank(k: NewsKind) -> u8 {
    match k {
        NewsKind::CoachFired => 0,
        NewsKind::CoachLeft => 1,
        _ => 2,
    }
}

fn move_word(k: NewsKind) -> &'static str {
    match k {
        NewsKind::CoachFired => "Fired",
        NewsKind::CoachLeft => "Leaves",
        _ => "Hired",
    }
}

/// The moves of the week before Monday `d` (the seven days up to Sunday),
/// in your division, your own program left out (its staff moves come as
/// their own letters). Read from the news store's tail: the carousel step
/// pushes hundreds in a day, so the scan goes back by date, not by count.
fn week(w: &World, pi: usize, d: Date) -> Vec<&NewsItem> {
    let lo = d.add(-7);
    let div = w.programs[pi].division;
    let mut out = Vec::new();
    for (n, nd) in super::news_since(w, lo) {
        if nd >= d || n.program.is_none_or(|p| p == pi || p >= w.programs.len()) {
            continue;
        }
        if !matches!(
            n.kind,
            NewsKind::CoachFired | NewsKind::CoachLeft | NewsKind::CoachHired | NewsKind::CoachExtended
        ) {
            continue;
        }
        if n.program.is_some_and(|p| w.programs[p].division == div) {
            out.push(n);
        }
    }
    out.reverse();
    out
}

/// The memo of Monday `d`.
pub(super) fn monday(pen: &mut Pen, d: Date) {
    let w = pen.w;
    let TeamRef::College(pi) = pen.me else { return };
    let items = week(w, pi, d);
    if items.is_empty() {
        return;
    }
    let mut rows: Vec<&NewsItem> = items
        .iter()
        .copied()
        .filter(|n| matches!(n.kind, NewsKind::CoachFired | NewsKind::CoachLeft | NewsKind::CoachHired))
        .filter(|n| !assistant(n))
        .collect();
    let extended = items.iter().filter(|n| n.kind == NewsKind::CoachExtended).count();
    let assistants = items.iter().filter(|n| assistant(n)).count();
    if rows.is_empty() && extended == 0 {
        return;
    }
    // Firings first, then departures, then hires; each in the order told.
    rows.sort_by_key(|n| rank(n.kind));
    let subject = match rows.as_slice() {
        [] => format!("Coaching carousel: {extended} extensions"),
        [one] if one.kind == NewsKind::CoachHired => {
            let p = one.program.map_or(String::new(), |p| w.program_name(p).to_string());
            format!("Coaching carousel: {p} hires a head coach")
        }
        [one] => {
            let p = one.program.map_or(String::new(), |p| w.program_name(p).to_string());
            format!(
                "Coaching carousel: {p} {}",
                if one.kind == NewsKind::CoachFired { "fires its coach" } else { "loses its coach" }
            )
        }
        many => format!("Coaching carousel: {} head-coaching moves", many.len()),
    };
    let lines: Vec<&str> = rows.iter().take(ROWS).map(|n| n.text.as_str()).collect();
    let linker = Linker::new(w, &lines, false);
    let mut m = pen.memo(&format!("carousel|{}", d.0), d, 8 * 60, Job::ChiefOfStaff, subject);
    m.body = vec![format!(
        "{}, the head-coaching moves around {} this past week:",
        pen.dear,
        w.programs[pi].division.name()
    )];
    if !rows.is_empty() {
        m.table = vec![vec!["Program".into(), "Move".into(), "What happened".into()]];
        for n in rows.iter().take(ROWS) {
            let p = n.program.unwrap_or(pi);
            m.table.push(vec![
                links::team(w, TeamRef::College(p)),
                move_word(n.kind).into(),
                linker.link_refs(&n.text, &n.refs, &[TeamRef::College(p)]),
            ]);
        }
    }
    if rows.len() > ROWS {
        m.body.push(format!(
            "{} more moves are on [[s:history?tab=Coaching carousel|the coaching carousel]].",
            rows.len() - ROWS
        ));
    }
    if extended > 0 {
        m.body.push(format!(
            "{} {} signed {} extension{}.",
            extended,
            if extended == 1 { "coach" } else { "coaches" },
            if extended == 1 { "an" } else { "" },
            if extended == 1 { "" } else { "s" }
        ));
    }
    if assistants > 0 {
        m.body.push(format!(
            "{assistants} assistant{} left a school staff for a college job. The market has the names.",
            if assistants == 1 { "" } else { "s" }
        ));
    }
    m.count = rows.len() as u32;
    m.keys = STAFF;
    m.loud = if rows.is_empty() { 0 } else { 1 };
    pen.out.push(m);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn firings_lead_and_hires_close() {
        let mut v = [NewsKind::CoachHired, NewsKind::CoachLeft, NewsKind::CoachFired];
        v.sort_by_key(|&k| rank(k));
        assert_eq!(v, [NewsKind::CoachFired, NewsKind::CoachLeft, NewsKind::CoachHired]);
    }

    #[test]
    fn an_assistant_hire_is_told_by_his_role() {
        let mk = |text: &str| NewsItem {
            date: None,
            at: None,
            year: 2026,
            kind: NewsKind::CoachHired,
            channel: afm_engine::world::news::Channel::News,
            program: Some(1),
            text: text.into(),
            refs: Vec::new(),
        };
        assert!(assistant(&mk("Ohio State hires Sam Lee as offensive coordinator (from a high school job).")));
        assert!(!assistant(&mk("Ohio State hires Sam Lee (from Iowa).")));
    }
}
