//! "Around the trail": the Monday recruiting digest also tells what happened
//! to the recruits you are watching at other schools: official visits,
//! commitments and signings, for the players on your board and every
//! 5-star (the inbox lane, Cain 2026-11-12; rows 18 and 22 of the event
//! table). Nothing here raises a world event: each row is read from the
//! recruit's own dated record (`cal.visits`, `cal.commit`, `cal.signed`).
//! The other schools' lines never reached the Continue card or the clips,
//! so nothing leaves them.

use std::collections::HashSet;

use afm_engine::calendar::Date;
use afm_engine::world::{PersonCal, Visit, World};

use super::super::links;
use crate::views::TeamRef;

/// How many rows each part of the table shows; the rest are a count.
const BOARD_ROWS: usize = 20;
const STAR_ROWS: usize = 10;

/// One step of a recruit's way at a school that is not yours.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Step {
    /// An official visit to the program.
    Visit(usize),
    /// A commitment to the program.
    Commit(usize),
    /// A signing with the program (`true`: he flipped to get there).
    Signed(usize, bool),
}

impl Step {
    /// Signings first, then commitments, then visits.
    fn rank(self) -> u8 {
        match self {
            Step::Signed(..) => 0,
            Step::Commit(_) => 1,
            Step::Visit(_) => 2,
        }
    }
}

/// The recruits on your board (the ones you have not pulled).
pub(super) fn board(w: &World, pi: usize) -> HashSet<u32> {
    let f = &w.front_office.college;
    if f.program != Some(pi) {
        return HashSet::new();
    }
    f.board.iter().filter(|t| t.pulled.is_none()).map(|t| t.id).collect()
}

/// A recruit's steps at other schools inside `inside`: official visits, the
/// commitment he holds (a later flip leaves only the last one), and his
/// signing. Walk-ons are not the story.
pub(super) fn steps(cal: &PersonCal, pi: usize, inside: impl Fn(Date) -> bool) -> Vec<(Date, Step)> {
    let mut out = Vec::new();
    for &(j, d, v) in &cal.visits {
        if j != pi && v == Visit::Official && inside(d) {
            out.push((d, Step::Visit(j)));
        }
    }
    if let Some((j, d, walk)) = cal.commit
        && j != pi
        && !walk
        && inside(d)
    {
        out.push((d, Step::Commit(j)));
    }
    if let Some(s) = cal.signed
        && s.program != pi
        && !s.walk_on
        && inside(s.date)
    {
        out.push((s.date, Step::Signed(s.program, s.flipped)));
    }
    out
}

/// What a step reads as, with the school linked.
fn what(w: &World, s: Step) -> String {
    let team = |j: usize| links::team(w, TeamRef::College(j));
    match s {
        Step::Visit(j) => format!("Official visit to {}", team(j)),
        Step::Commit(j) => format!("Commits to {}", team(j)),
        Step::Signed(j, false) => format!("Signs with {}", team(j)),
        Step::Signed(j, true) => format!("Signs with {} (flipped)", team(j)),
    }
}

/// The digest's trail part for one week: section rows (`["## ..."]`) and
/// player rows, capped, how many steps there were, and how many are left
/// out of the table.
pub(super) struct Part {
    pub table: Vec<Vec<String>>,
    pub total: u32,
    pub more: u32,
}

/// `list`: (day, person index, step) for the week; `board`: who is on it.
pub(super) fn part(w: &World, mut list: Vec<(Date, usize, Step)>, board: &HashSet<u32>) -> Part {
    // Newest signings first, then by stars, so the cap keeps what matters.
    list.sort_by_key(|&(d, k, s)| {
        (s.rank(), std::cmp::Reverse(w.people[k].stars), std::cmp::Reverse(d), w.people[k].player.id)
    });
    let (mine, stars): (Vec<_>, Vec<_>) = list.iter().partition(|(_, k, _)| board.contains(&w.people[*k].player.id));
    let mut table = Vec::new();
    let mut shown = 0usize;
    for (title, rows, cap) in [("our board", &mine, BOARD_ROWS), ("other 5-stars", &stars, STAR_ROWS)] {
        if rows.is_empty() {
            continue;
        }
        let n = rows.len().min(cap);
        table.push(vec![format!("## Around the trail: {title} · {n} of {}", rows.len())]);
        for &&(_, k, s) in rows.iter().take(cap) {
            let p = &w.people[k];
            table.push(vec![
                links::player(p.player.id, &p.name),
                p.player.position.name().into(),
                if p.stars > 0 { format!("{}★", p.stars) } else { "–".into() },
                links::team(w, TeamRef::HighSchool(p.hs)),
                what(w, s),
            ]);
        }
        shown += n;
    }
    let total = list.len() as u32;
    Part { table, total, more: total.saturating_sub(shown as u32) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use afm_engine::world::Signed;

    fn cal() -> PersonCal {
        PersonCal::default()
    }

    #[test]
    fn a_recruits_steps_elsewhere_are_read_from_his_dated_record() {
        let all = |_: Date| true;
        let d = Date::ymd(2026, 12, 5);
        let mut c = cal();
        c.visits.push((4, d, Visit::Official));
        c.visits.push((4, d, Visit::Unofficial));
        c.visits.push((9, d, Visit::Official));
        c.commit = Some((4, d, false));
        c.signed =
            Some(Signed { program: 4, walk_on: false, date: d, early_enrollee: false, flipped: true, committed: None });
        let got = steps(&c, 9, all);
        // His visit to us (9) is ours to tell, the unofficial one is no news.
        assert_eq!(got, [(d, Step::Visit(4)), (d, Step::Commit(4)), (d, Step::Signed(4, true))]);
        // Walk-ons and our own school are not the story; neither is a day outside.
        c.commit = Some((4, d, true));
        c.signed = Some(Signed {
            program: 9,
            walk_on: false,
            date: d,
            early_enrollee: false,
            flipped: false,
            committed: None,
        });
        assert_eq!(steps(&c, 9, all), [(d, Step::Visit(4))]);
        assert!(steps(&c, 9, |_| false).is_empty());
    }
}
