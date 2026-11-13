//! "Where we stand": the Monday memo from communications while the college
//! season runs: where the weekend's polls put you, your record and the next
//! game with the other side's rank and record (the inbox lane, row 50 of the
//! event table). Nothing here raises a world event: the polls are the
//! season's own releases (`World::poll_weeks`) and the record is counted
//! from the games dated before the Monday, so a later stop gives the same
//! memo.

use afm_engine::calendar::{Date, Moment};
use afm_engine::world::seat::Fixture;

use super::super::links;
use super::super::people::Job;
use super::Pen;
use crate::views::TeamRef;

/// The polls a memo reports, as `PollWeek::kind` names them.
const POLLS: [&str; 4] = ["Coaches Poll", "AP poll", "BCS standings", "CFP rankings"];

/// A record counted from the games played before `d`: wins, losses, and the
/// conference's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Tally {
    w: u32,
    l: u32,
    cw: u32,
    cl: u32,
}

fn tally(games: &[Fixture], team: usize, d: Date) -> Tally {
    let mut t = Tally::default();
    for g in games.iter().filter(|g| g.date < d && !g.cancelled) {
        let Some([h, a]) = g.score else { continue };
        let (mine, theirs) = if g.home == team { (h, a) } else { (a, h) };
        if mine == theirs {
            continue;
        }
        let won = mine > theirs;
        t.w += u32::from(won);
        t.l += u32::from(!won);
        if g.conference {
            t.cw += u32::from(won);
            t.cl += u32::from(!won);
        }
    }
    t
}

fn record(t: Tally) -> String {
    format!("{}-{}", t.w, t.l)
}

/// "No. 12" or "unranked", from a poll's top list.
fn rank_words(top: &[usize], p: usize) -> String {
    match top.iter().position(|&x| x == p) {
        Some(i) => format!("No. {}", i + 1),
        None => "unranked".into(),
    }
}

/// The memo of Monday `d`.
pub(super) fn where_we_stand(pen: &mut Pen, d: Date) {
    let w = pen.w;
    let TeamRef::College(pi) = pen.me else { return };
    let mine = w.college_fixtures_in(Some(pi), false);
    // The season is open: a game played this past week or a poll out in it.
    let weeks: Vec<_> = w
        .poll_weeks(false)
        .into_iter()
        .filter(|p| p.date < d && p.date >= d.add(-7) && POLLS.contains(&p.kind))
        .collect();
    let played_week = mine.iter().any(|g| g.date < d && g.date >= d.add(-7) && g.score.is_some());
    if weeks.is_empty() && !played_week {
        return;
    }
    let t = tally(&mine, pi, d);
    if t.w + t.l == 0 {
        return;
    }
    // The headline poll: the AP's, else the first of the week.
    let lead = weeks.iter().find(|p| p.kind == "AP poll").or(weeks.first());
    let ranked = lead.and_then(|p| p.top.iter().position(|&x| x == pi)).map(|i| i + 1);
    let subject = match (ranked, lead) {
        (Some(r), Some(p)) => format!("Where we stand: No. {r} in the {}, {}", p.kind, record(t)),
        _ if weeks.is_empty() => format!("Where we stand: {}", record(t)),
        _ => format!("Where we stand: unranked, {}", record(t)),
    };
    let mut m = pen.memo(&format!("stand|{}", d.0), d, 8 * 60 + 30, Job::Communications, subject);
    m.body = vec![format!("{}, after the weekend:", pen.dear)];
    if !weeks.is_empty() {
        m.table = vec![vec!["Poll".into(), "Rank".into(), "Points".into()]];
        for p in &weeks {
            let at = p.top.iter().position(|&x| x == pi);
            m.table.push(vec![
                p.kind.to_string(),
                rank_words(&p.top, pi),
                at.and_then(|i| p.points.get(i)).map_or(String::new(), |x| x.to_string()),
            ]);
        }
    }
    m.body.push(if t.cw + t.cl > 0 {
        format!("Record: {} ({}-{} in the conference).", record(t), t.cw, t.cl)
    } else {
        format!("Record: {}.", record(t))
    });
    // The next game, with the other side's rank in the lead poll and record.
    let next = mine.iter().filter(|g| g.date >= d && !g.cancelled).min_by_key(|g| g.date);
    let mut loud = u8::from(ranked.is_some());
    if let Some(g) = next {
        let (opp, away) = if g.home == pi { (g.away, false) } else { (g.home, true) };
        let theirs = tally(&w.college_fixtures_in(Some(opp), false), opp, d);
        let their_rank = lead.and_then(|p| p.top.iter().position(|&x| x == opp)).map(|i| format!("No. {} ", i + 1));
        let when = w.when_text(Moment::at(g.date, g.kickoff.0, g.kickoff.1));
        m.body.push(format!(
            "{}Next: {}{}{} ({}), {when}.",
            if g.date > d.add(6) { "Open date. " } else { "" },
            if away { "at " } else { "" },
            their_rank.unwrap_or_default(),
            links::team(w, TeamRef::College(opp)),
            record(theirs),
        ));
        if ranked.is_some() && lead.is_some_and(|p| p.top.contains(&opp)) {
            loud = 2;
        }
    }
    m.count = m.table.len().saturating_sub(1) as u32;
    m.loud = loud;
    pen.out.push(m);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(date: Date, home: usize, away: usize, score: Option<[u32; 2]>, conference: bool) -> Fixture {
        Fixture {
            date,
            kickoff: (15, 30),
            home,
            away,
            name: None,
            conference,
            neutral: false,
            why: Vec::new(),
            score,
            cancelled: false,
            line: None,
            attendance: 0,
        }
    }

    #[test]
    fn the_record_is_counted_from_the_games_before_the_monday() {
        let d = Date::ymd(2026, 10, 12);
        let games = [
            game(Date::ymd(2026, 9, 5), 1, 2, Some([30, 10]), false),
            game(Date::ymd(2026, 9, 12), 3, 1, Some([24, 20]), true),
            game(Date::ymd(2026, 10, 3), 1, 4, Some([17, 17]), true),
            // After the Monday: not counted.
            game(Date::ymd(2026, 10, 17), 1, 5, Some([50, 0]), false),
        ];
        let t = tally(&games, 1, d);
        assert_eq!(t, Tally { w: 1, l: 1, cw: 0, cl: 1 });
        assert_eq!(record(t), "1-1");
    }

    #[test]
    fn rank_reads_from_the_top_list() {
        assert_eq!(rank_words(&[7, 3, 9], 3), "No. 2");
        assert_eq!(rank_words(&[7, 3, 9], 4), "unranked");
    }
}
