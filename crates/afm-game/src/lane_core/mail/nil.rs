//! "NIL answers": how the people you put a number to took it, as one memo
//! from the collective for each time a batch of answers reaches you, instead
//! of one row each on the Continue card (the inbox lane, row 34 of the event
//! table). A recruit's promise, a transfer's offer and a roster player's deal
//! each leave one answer (`front_office::college::Reaction`); it reaches you
//! at the next section stop (`due`). Nothing here raises a world event.

use std::collections::BTreeMap;

use afm_engine::calendar::{Date, Moment};
use afm_engine::world::front_office::college::{MoneyKind, Mood, Reaction};
use afm_engine::world::front_office::dollars;

use super::super::links;
use super::super::people::Job;
use super::{MONEY, PLAYERS, PORTAL, Pen, RECRUITING};
use crate::views::{self, TeamRef};

/// How many rows a memo shows; the rest wait on the NIL desk.
const ROWS: usize = 30;

/// The worst first: who took it badly leads.
fn rank(m: Mood) -> u8 {
    match m {
        Mood::Insulted => 0,
        Mood::WantsMore => 1,
        Mood::Fine => 2,
        Mood::Pleased => 3,
        Mood::Thrilled => 4,
    }
}

fn what_for(k: MoneyKind) -> &'static str {
    match k {
        MoneyKind::Promise => "Recruit promise",
        MoneyKind::Portal => "Transfer offer",
        MoneyKind::Deal => "Your player's deal",
    }
}

/// One memo for each section stop whose answers have come in `from..=to`
/// (your local days).
pub(super) fn answers(pen: &mut Pen, from: Date, to: Date) {
    let w = pen.w;
    if !matches!(pen.me, TeamRef::College(_)) {
        return;
    }
    // By the local moment the answer reaches you; one that is not due yet
    // waits for its section.
    let mut groups: BTreeMap<Moment, Vec<&Reaction>> = BTreeMap::new();
    for r in &w.front_office.college.reactions {
        let Some(due) = r.due.filter(|&m| w.has_come(m)) else { continue };
        let at = w.local(due);
        if at.date >= from && at.date <= to {
            groups.entry(at).or_default().push(r);
        }
    }
    for (at, mut rs) in groups {
        rs.retain(|r| views::person_index(w, r.id).is_some());
        if rs.is_empty() {
            continue;
        }
        rs.sort_by_key(|r| (rank(r.mood), views::person_index(w, r.id).map(|k| w.people[k].name.clone())));
        let want_more = rs.iter().filter(|r| matches!(r.mood, Mood::Insulted | Mood::WantsMore)).count();
        let subject = match (rs.len(), want_more) {
            (1, _) => {
                let k = views::person_index(w, rs[0].id).unwrap_or_default();
                format!("NIL answer: {}", w.people[k].name)
            }
            (n, 0) => format!("NIL answers: {n} back"),
            (n, m) => format!("NIL answers: {n} back, {m} want more"),
        };
        let mut m = pen.memo(&format!("nilanswers|{}", at.minute), at.date, at.minute, Job::Collective, subject);
        m.body = vec![format!("{}, here is how they took the numbers you set:", pen.dear)];
        m.table = vec![vec![
            "Player".into(),
            "Pos".into(),
            "For".into(),
            "Your number".into(),
            "He says".into(),
            "Why".into(),
        ]];
        for r in rs.iter().take(ROWS) {
            let Some(k) = views::person_index(w, r.id) else { continue };
            let p = &w.people[k];
            m.table.push(vec![
                links::player(p.player.id, &p.name),
                p.player.position.name().into(),
                what_for(r.kind).into(),
                dollars(r.money),
                r.mood.word().into(),
                r.text.clone(),
            ]);
        }
        if rs.len() > ROWS {
            m.body.push(format!(
                "{} more answers are on [[s:nil_desk|the NIL desk]], which is also where you change a number.",
                rs.len() - ROWS
            ));
        } else {
            m.body.push("[[s:nil_desk|The NIL desk]] is where you change a number.".into());
        }
        m.ids = rs.iter().map(|r| r.id).collect();
        m.count = rs.len() as u32;
        m.keys = MONEY | PLAYERS;
        if rs.iter().any(|r| r.kind == MoneyKind::Promise) {
            m.keys |= RECRUITING;
        }
        if rs.iter().any(|r| r.kind == MoneyKind::Portal) {
            m.keys |= PORTAL;
        }
        m.loud = 1;
        pen.out.push(m);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_worst_answer_leads() {
        let mut moods = [Mood::Thrilled, Mood::Fine, Mood::Insulted, Mood::Pleased, Mood::WantsMore];
        moods.sort_by_key(|&m| rank(m));
        assert_eq!(moods, [Mood::Insulted, Mood::WantsMore, Mood::Fine, Mood::Pleased, Mood::Thrilled]);
    }
}
