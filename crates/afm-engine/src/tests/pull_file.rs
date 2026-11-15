//! "What pulls him" in words (talk facts lane L7; `world::folk::pulls`):
//! the file's order marks where his words and your read part, the lines
//! for it read the way section 7 of the design says ("says development,
//! lights up at money"), and no line carries a number.

use crate::calendar::Date;
use crate::world::folk::claims::Group;
use crate::world::folk::pulls::{self, Ev, Pull, Pulls, Where};

fn row(group: Group, ev: Ev, on: Date) -> Pull {
    Pull { who: 7, group: group as u8, ev, from: Where::Call, on, by: None, decider: false }
}

#[test]
fn the_file_marks_a_kid_who_dresses_up_the_money() {
    let d = Date::ymd(2037, 3, 3);
    let mut f = Pulls::default();
    // He talks up development and plays the money down; your read catches
    // the money reaching him.
    f.note(d, row(Group::League, Ev::ToldFirst, d));
    f.note(d, row(Group::League, Ev::TookWell, d));
    f.note(d, row(Group::Money, Ev::Cool, d));
    f.note(d, row(Group::Money, Ev::CaughtLit, d));
    let o = f.order(7, Some(false));
    let money = o.iter().find(|r| r.group == Group::Money).unwrap();
    assert!(money.under && money.conflict && !money.over && !money.talked);
    let league = o.iter().find(|r| r.group == Group::League).unwrap();
    assert!(league.talked && !league.conflict);
    assert_eq!(pulls::flags(&o), vec!["Says development, lights up at money.".to_string()]);
    // Nothing he talks up: the money line stands alone.
    let mut g = Pulls::default();
    g.note(d, row(Group::Money, Ev::Cool, d));
    g.note(d, row(Group::Money, Ev::CaughtLit, d));
    assert_eq!(pulls::flags(&g.order(7, None)), vec!["Money matters more to him than he lets on.".to_string()]);
    // He says playing time, and your read caught it falling flat.
    let mut h = Pulls::default();
    h.note(d, row(Group::PlayingTime, Ev::TookWell, d));
    h.note(d, row(Group::PlayingTime, Ev::CaughtFlat, d));
    let o = h.order(7, None);
    assert!(o.iter().any(|r| r.group == Group::PlayingTime && r.over));
    assert_eq!(pulls::flags(&o), vec!["Says playing time, but it doesn't seem to reach him.".to_string()]);
    // A straight kid: nothing flagged.
    let mut s = Pulls::default();
    s.note(d, row(Group::Home, Ev::ToldFirst, d));
    s.note(d, row(Group::Home, Ev::CaughtLit, d));
    assert!(pulls::flags(&s.order(7, None)).is_empty());
}

#[test]
fn every_row_says_what_he_did_in_words() {
    let d = Date::ymd(2037, 3, 3);
    let evs = [
        Ev::BroughtUp,
        Ev::AskedLater,
        Ev::TookWell,
        Ev::Polite,
        Ev::Cool,
        Ev::Doubted,
        Ev::CaughtLit,
        Ev::CaughtFlat,
        Ev::ToldFirst,
        Ev::ToldSecond,
        Ev::Guess,
    ];
    for g in Group::ALL {
        for ev in evs {
            let s = row(g, ev, d).says();
            assert!(s.contains(pulls::word(g)), "{s}");
            assert!(!s.chars().any(|c| c.is_ascii_digit()), "no number on the file: {s}");
        }
    }
    for code in 0..4u8 {
        let r = Pull { who: 7, group: code, ev: Ev::Decides, from: Where::Home, on: d, by: None, decider: true };
        assert!(r.says().starts_with("Told you"), "{}", r.says());
        assert_eq!(pulls::decider_of(code) as u8, code);
    }
    assert_eq!(row(Group::League, Ev::BroughtUp, d).says(), "Asked about development first");
}
