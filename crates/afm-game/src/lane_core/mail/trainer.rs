//! The trainers' memo for a college or high school seat (the inbox lane,
//! row 49 of the event table): who is hurt, for how long, the morning after
//! each game and on Wednesdays while the season runs. It reads the training
//! desk's own record of who is out (`SeatTraining::hurt`: the day he was
//! hurt, the day he is back, what it was), so nothing here raises a world
//! event. An NFL seat has its own injury report (`mail::injuries`).

use std::collections::HashSet;

use afm_engine::calendar::Date;
use afm_engine::training::seat::Hurt;
use afm_engine::world::World;

use super::super::links;
use super::super::people::Job;
use super::{ABOUT_GAME, INJURIES, PLAYERS, Pen, act, answer};
use crate::views::{self, TeamRef};

/// Rows a memo shows.
const ROWS: usize = 25;

/// Off the field on day `d` after being hurt before it, for a man on my roster.
fn out_before<'a>(w: &'a World, roster: &HashSet<u32>, d: Date, before: Date) -> Vec<&'a Hurt> {
    let mut rows: Vec<&Hurt> =
        w.training.hurt.iter().filter(|h| roster.contains(&h.id) && h.from < before && h.until > d).collect();
    // A man with two rows keeps the longest.
    rows.sort_by_key(|h| (h.id, std::cmp::Reverse(h.until)));
    rows.dedup_by_key(|h| h.id);
    rows
}

/// Hurt in the game of day `g`, still out on `d`.
fn from_game<'a>(w: &'a World, roster: &HashSet<u32>, g: Date, d: Date) -> Vec<&'a Hurt> {
    let mut rows: Vec<&Hurt> = w
        .training
        .hurt
        .iter()
        .filter(|h| roster.contains(&h.id) && h.how == "game" && h.from == g && h.until > d)
        .collect();
    rows.sort_by_key(|h| (h.id, std::cmp::Reverse(h.until)));
    rows.dedup_by_key(|h| h.id);
    rows
}

/// Whether anyone was hurt in a game on `g` (the morning after has a memo).
pub(super) fn hurt_in_game(w: &World, g: Date) -> bool {
    w.training.hurt.iter().any(|h| h.how == "game" && h.from == g)
}

/// "3 wk" or "5 d".
fn out_words(days: i32) -> String {
    if days <= 7 { format!("{} d", days.max(1)) } else { format!("{} wk", (days + 6) / 7) }
}

fn what(h: &Hurt) -> String {
    match h.kind {
        Some(k) => k.name().to_string(),
        None if h.how.contains("heat") => "Heat illness".into(),
        None => "Non-football injury list".into(),
    }
}

/// The memo of day `d`: `game` is the day of the game it follows (the
/// morning after), none for Wednesday's.
pub(super) fn report(pen: &mut Pen, d: Date, game: Option<Date>) {
    let w = pen.w;
    if !matches!(pen.me, TeamRef::College(_) | TeamRef::HighSchool(_)) {
        return;
    }
    let roster: HashSet<u32> = views::roster_people(w, pen.me).into_iter().map(|k| w.people[k].player.id).collect();
    let (fresh, older): (Vec<&Hurt>, Vec<&Hurt>) = match game {
        Some(g) => {
            let fresh = from_game(w, &roster, g, d);
            let ids: HashSet<u32> = fresh.iter().map(|h| h.id).collect();
            (fresh, out_before(w, &roster, d, g).into_iter().filter(|h| !ids.contains(&h.id)).collect())
        }
        None => (Vec::new(), out_before(w, &roster, d, d)),
    };
    if fresh.is_empty() && older.is_empty() {
        return;
    }
    // Off the season only the long ones are worth a Wednesday.
    if game.is_none() {
        let in_season = w.seat_games().iter().any(|g| g.date < d && g.date >= d.add(-10));
        if !in_season && !older.iter().any(|h| d.days_to(h.until) >= 14) {
            return;
        }
    }
    let opp = game.and_then(|g| {
        let me = match pen.me {
            TeamRef::College(i) | TeamRef::HighSchool(i) | TeamRef::Nfl(i) | TeamRef::Other(i) => i,
        };
        w.seat_games().iter().find(|x| x.date == g).map(|x| x.names[usize::from(x.teams[0] == me)].clone())
    });
    let n = fresh.len() + older.len();
    let subject = match (&game, &opp) {
        (Some(_), Some(o)) if !fresh.is_empty() => format!("Trainers: {} hurt in the {o} game", fresh.len()),
        (Some(_), None) if !fresh.is_empty() => format!("Trainers: {} hurt in the game", fresh.len()),
        (Some(_), _) => format!("Trainers: {n} still out after the game"),
        (None, _) => format!("Trainers: {n} out as of Wednesday"),
    };
    let tag = match game {
        Some(g) => format!("hurtgame|{}", g.0),
        None => format!("hurtwed|{}", d.0),
    };
    let mut m = pen.memo(&tag, d, 8 * 60, Job::Trainer, subject);
    m.body = vec![format!("{}, how we stand:", pen.dear)];
    m.table = vec![vec!["Player".into(), "Pos".into(), "Role".into(), "Injury".into(), "Out".into(), "Back".into()]];
    let mut shown = 0;
    let mut ids: Vec<u32> = Vec::new();
    let mut starter_out = false;
    for (title, rows) in [("From the game, worst first", fresh), ("Still out, longest first", older)] {
        if rows.is_empty() {
            continue;
        }
        let mut rows = rows;
        rows.sort_by_key(|h| std::cmp::Reverse(h.until));
        if game.is_some() || shown > 0 {
            m.table.push(vec![format!("## {title}")]);
        }
        for h in rows {
            ids.push(h.id);
            let Some(k) = views::person_index(w, h.id) else { continue };
            let p = &w.people[k];
            let days = d.days_to(h.until);
            starter_out |= days >= 28 && p.last_role == Some(afm_engine::player::PlayingTime::Starter);
            if shown < ROWS {
                m.table.push(vec![
                    links::player(p.player.id, &p.name),
                    p.player.position.name().into(),
                    p.last_role.map_or("", views::role_words).into(),
                    what(h),
                    out_words(days),
                    h.until.pretty(),
                ]);
                shown += 1;
            }
        }
    }
    if ids.len() > shown {
        m.body.push(format!("{} more are on the [[s:training|training desk]]'s list.", ids.len() - shown));
    }
    m.count = ids.len() as u32;
    m.ids = ids;
    m.keys = INJURIES | PLAYERS;
    m.loud = if starter_out { 2 } else { 1 };
    if let Some(g) = game {
        if let Some(k) = w.seat_games().iter().position(|x| x.date == g) {
            m.about = Some((ABOUT_GAME, k as u32));
        }
        m.answers = vec![
            answer(
                "Check in on each of them",
                "You call each of them. Each one's mood rises and the calls are logged.",
                act::CALL_ALL,
                0,
            ),
            answer("Leave it to the trainers", "Nothing more is done.", act::ACK, 0),
        ];
    }
    pen.out.push(m);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn out_reads_in_days_then_weeks() {
        assert_eq!(out_words(0), "1 d");
        assert_eq!(out_words(5), "5 d");
        assert_eq!(out_words(7), "7 d");
        assert_eq!(out_words(8), "2 wk");
        assert_eq!(out_words(21), "3 wk");
        assert_eq!(out_words(22), "4 wk");
    }
}
