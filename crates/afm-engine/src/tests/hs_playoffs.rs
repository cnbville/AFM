//! U-d (each state's real high school playoff format) and A11 (the
//! dynasty leveler: strong programs schedule up).

use crate::calendar::data::hs_calendar;
use crate::world::data::STATES;
use crate::world::hs_playoffs::{
    Bracket, Brackets, NONE, PrivateRule, Qualify, bracket_order, format, real_private, rounds_for,
};

/// The state's playoff rounds, as `hs_year` counts them (one a week, then
/// the final on its own day).
fn calendar_rounds(code: &str) -> usize {
    let c = hs_calendar(code);
    let days = crate::calendar::Date::ymd(2026, c.playoffs.0, c.playoffs.1)
        .days_to(crate::calendar::Date::ymd(2026, c.finals.0, c.finals.1));
    ((days + 3) / 7).max(1) as usize + 1
}

#[test]
fn every_state_has_a_real_format() {
    for st in STATES.iter() {
        let f = format(&st.code);
        assert_eq!(f.code, st.code, "{} has no format row", st.code);
        assert!(!f.classes.is_empty() && !f.fields.is_empty() && !f.qualify.is_empty(), "{}", st.code);
        assert!(f.sizes.is_empty() || f.sizes.len() == f.classes.len(), "{} sizes", st.code);
    }
}

#[test]
fn real_fields_fit_each_states_calendar() {
    // Every full-size bracket fits the weeks between the state's first round
    // and its final, so no real field is cut. (Where everyone enters, the
    // calendar is the cap: the weakest miss out, as a ninth team in a
    // Minnesota section does.)
    let mut bad = Vec::new();
    for st in STATES.iter() {
        let f = format(&st.code);
        let rounds = calendar_rounds(&st.code);
        for i in 0..f.bracket_labels().len() {
            let field = f.field_of(i) as usize;
            if f.qualify_of(i) != Qualify::All && rounds_for(field) > rounds {
                bad.push(format!("{} bracket {i}: {field} teams, {rounds} rounds", st.code));
            }
        }
        if let Some(o) = &f.open {
            let need = if o.pools { 5 } else { rounds_for(o.teams as usize) };
            if need > rounds {
                bad.push(format!("{} Open", st.code));
            }
        }
    }
    assert!(bad.is_empty(), "{bad:?}");
}

#[test]
fn field_sizes_and_private_rules_per_state() {
    let nc = format("NC");
    assert_eq!((nc.field_of(0), nc.field_of(1), nc.field_of(7)), (24, 48, 24));
    let tx = format("TX");
    assert_eq!(tx.qualify_of(0), Qualify::District(4));
    assert_eq!(tx.qualify_of(10), Qualify::District(2), "six-man takes the top 2 per district");
    assert_eq!(tx.bracket_labels().len(), 15, "12 UIL brackets and 3 TAPPS");
    let la = format("LA");
    assert_eq!(la.private, PrivateRule::Select);
    let labels = la.bracket_labels();
    assert_eq!(labels[0], "Non-Select Division I");
    assert_eq!(labels[4], "Select Division I");
    assert_eq!((la.field_of(0), la.field_of(4)), (28, 24));
    let il = format("IL");
    assert_eq!(il.private, PrivateRule::Multiplier(1.65));
    assert_eq!(il.brackets, Brackets::Enrollment(8));
    assert_eq!(il.field_of(3), 48);
    assert!(matches!(format("CA").brackets, Brackets::Equity(ref l) if l.len() == 15 && l[0] == "Open"));
    assert_eq!(format("IN").qualify_of(0), Qualify::All);
    assert_eq!(format("MN").qualify_of(2), Qualify::All);
    assert_eq!(format("CT").private, PrivateRule::PlayUp);
    assert_eq!(format("WI").private, PrivateRule::Tpf);
    assert!(matches!(format("NJ").private, PrivateRule::NonPublic(ref l) if l.len() == 2));
    assert_eq!(format("AL").private_classes.len(), 2);
    assert_eq!(format("TN").field_of(8), 13);
    let fl = format("FL").open.clone().unwrap();
    assert!(fl.pools && fl.teams == 8 && fl.from.is_none());
    assert_eq!(format("AZ").open.clone().unwrap().from, Some(0));
    assert_eq!(format("UT").qualify_of(0), Qualify::All);
    assert_eq!(format("UT").qualify_of(1), Qualify::Top);
}

#[test]
fn real_powerhouses_are_private_by_name() {
    for n in ["Mater Dei", "St. John Bosco", "IMG Academy", "Bishop Gorman", "De La Salle", "Brother Martin", "Trinity"] {
        assert!(real_private(n), "{n}");
    }
    for n in ["Duncanville", "Buford", "Mater Academy East Las Vegas", "Edna Karr", "Imhotep Charter", "Allen"] {
        assert!(!real_private(n), "{n}");
    }
}

/// Plays a bracket out, the better seed always winning, and returns the
/// games per round.
fn play_out(b: &mut Bracket) -> Vec<usize> {
    let mut per = Vec::new();
    while !b.done() {
        let g = b.games();
        per.push(g.len());
        let res: Vec<(u32, u32)> = g.iter().map(|&(a, h)| (a, h)).collect();
        b.advance(&res);
        assert!(per.len() < 12);
    }
    per
}

#[test]
fn byes_go_to_the_top_seeds() {
    // North Carolina: 48 in a 64 bracket, the top 16 on a bye.
    assert_eq!(bracket_order(4), vec![0, 3, 1, 2]);
    let mut b = Bracket::new(0, "7A".into(), (0..48).collect(), 6, false);
    assert_eq!(b.start, 0);
    let first = b.games();
    assert_eq!(first.len(), 16);
    assert!(first.iter().all(|&(a, h)| a >= 16 && h >= 16), "no top-16 seed plays the first round");
    assert_eq!(play_out(&mut b), vec![16, 16, 8, 4, 2, 1]);
    assert_eq!(b.champion, 0);
    assert!(b.semi[0] && b.semi[3] && !b.semi[4]);
    assert_eq!(b.wins[0], 5, "a bye is not a win");
    // Arizona 6A: 12 teams with five rounds on the calendar start a round
    // late, the top 4 on a bye.
    let mut b = Bracket::new(0, "6A".into(), (0..12).collect(), 5, false);
    assert_eq!((b.start, b.round), (1, 1));
    assert_eq!(play_out(&mut b), vec![4, 4, 2, 1]);
    // A field bigger than the calendar allows keeps its best.
    let b = Bracket::new(0, "x".into(), (0..40).collect(), 5, false);
    assert_eq!(b.seeds.len(), 32);
    // A class of one is crowned without a game.
    let b = Bracket::new(0, "x".into(), vec![7], 5, false);
    assert_eq!(b.champion, 7);
}

#[test]
fn florida_open_plays_pools_then_semifinals() {
    let mut b = Bracket::new(0, "Open".into(), (0..8).collect(), 5, true);
    assert_eq!(b.start, 0);
    assert_eq!(play_out(&mut b), vec![4, 4, 4, 2, 1]);
    assert_eq!(b.champion, 0);
    // Seeds 1 and 2 top their pools; 4 and 3 come second.
    assert!(b.semi[0] && b.semi[1] && b.semi[2] && b.semi[3] && !b.semi[4]);
    assert_eq!(b.slots.len(), 1);
    assert_ne!(b.champion, NONE);
}

#[test]
fn strong_programs_schedule_up() {
    // A state of two classes of 40; four private powerhouses (two in each
    // class) and a few strong public programs.
    let ids: Vec<usize> = (0..80).collect();
    let classes: Vec<(u32, &[usize])> = vec![(0, &ids[..40]), (1, &ids[40..])];
    let area = |id: usize| id / 8;
    let powerhouses = [3usize, 17, 44, 61];
    let strong = [9usize, 25, 50, 70];
    let power = |id: usize| {
        if powerhouses.contains(&id) {
            (0.9, true)
        } else if strong.contains(&id) {
            (0.65, false)
        } else {
            (0.3, false)
        }
    };
    let (weeks, districts) = crate::world::hs_year::state_schedule(&classes, &area, &power, 10, 7);
    // The private powerhouses play one league, across classes.
    let league = districts.iter().find(|d| d.contains(&3)).unwrap();
    for p in powerhouses {
        assert!(league.contains(&p), "{p} is in the powerhouse league");
    }
    // Each powerhouse meets the other three and the strong programs.
    let met = |a: usize, b: usize| weeks.iter().flatten().any(|&(h, x)| (h, x) == (a, b) || (h, x) == (b, a));
    for &p in &powerhouses {
        for &q in &powerhouses {
            assert!(p == q || met(p, q), "{p} and {q} meet in the league");
        }
        let big = strong.iter().filter(|&&q| met(p, q)).count();
        assert!(big >= 2, "{p} schedules up: {big} strong programs");
    }
    // Every school still plays most weeks, and no pair meets twice.
    let mut seen = std::collections::HashSet::new();
    for &(h, a) in weeks.iter().flatten() {
        assert!(seen.insert((h.min(a), h.max(a))), "{h} and {a} meet twice");
    }
}
