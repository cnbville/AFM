//! Every team's testing records (Cain 2026-11-15): the book a player keeps
//! at his team, what a new team does to it, and how it is written.

use crate::calendar::Date;
use crate::training::rules::Tier;
use crate::training::seat::TestRow;
use crate::training::testing::{Entry, KEEP, TestBook, team_key};

fn row(id: u32, forty: u16) -> TestRow {
    TestRow {
        id,
        bench: 315,
        reps: 22,
        squat: 455,
        clean: 280,
        forty,
        split: 159,
        shuttle: 431,
        vertical: 67,
        weight: 247,
    }
}

#[test]
fn a_row_packs_to_twenty_bytes_and_back() {
    assert_eq!(std::mem::size_of::<Entry>(), 20);
    let r = row(7, 462);
    let e = Entry::new(Date::ymd(2026, 1, 12), team_key(Tier::College, 41), &r, true);
    assert!(e.baseline());
    assert_eq!(e.row(7), r);
    let e = Entry::new(Date::ymd(2026, 1, 12), team_key(Tier::College, 41), &r, false);
    assert!(!e.baseline());
    assert_eq!(e.row(7), r);
}

#[test]
fn the_records_stay_with_the_team() {
    let a = team_key(Tier::High, 900);
    let b = team_key(Tier::College, 900);
    assert_ne!(a, b);
    let mut book = TestBook::default();
    let mut d = Date::ymd(2020, 4, 20);
    for k in 0..20 {
        book.record(Entry::new(d, a, &row(1, 480 - k), false));
        d = d.add(180);
    }
    // His last KEEP days at the team, oldest first.
    let kept: Vec<&Entry> = book.at(a).collect();
    assert_eq!(kept.len(), KEEP);
    assert!(kept.windows(2).all(|x| x[0].date < x[1].date));
    assert_eq!(kept.last().unwrap().row(1).forty, 461);
    // Nothing of his old team's shows at the new one before it tests him.
    assert_eq!(book.at(b).count(), 0);
    // Its first testing day drops the old team's rows.
    book.record(Entry::new(d, b, &row(1, 455), false));
    assert_eq!(book.at(a).count(), 0);
    assert_eq!(book.at(b).count(), 1);
    assert_eq!(book.len(), 1);
    // The same day twice keeps the later numbers once.
    book.record(Entry::new(d, b, &row(1, 452), false));
    assert_eq!(book.len(), 1);
    assert_eq!(book.at(b).next().unwrap().row(1).forty, 452);
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
struct Holder {
    #[serde(default)]
    tests: TestBook,
    other: u8,
}

#[test]
fn a_book_is_written_as_bytes_and_read_back() {
    let mut book = TestBook::default();
    let t = team_key(Tier::Pro, 31);
    book.record(Entry::new(Date::ymd(2026, 4, 20), t, &row(5, 451), false));
    book.record(Entry::new(Date::ymd(2026, 7, 21), t, &row(5, 449), true));
    let h = Holder { tests: book, other: 3 };
    let bytes = rmp_serde::to_vec_named(&h).unwrap();
    // Two rows of 20 bytes, with little around them.
    assert!(bytes.len() < 40 + 24, "{} bytes", bytes.len());
    let back: Holder = rmp_serde::from_slice(&bytes).unwrap();
    assert_eq!(back, h);
    let json = serde_json::to_string(&h).unwrap();
    let back: Holder = serde_json::from_str(&json).unwrap();
    assert_eq!(back, h);
    // Older files have no book.
    #[derive(serde::Serialize)]
    struct Old {
        other: u8,
    }
    let old: Holder = rmp_serde::from_slice(&rmp_serde::to_vec_named(&Old { other: 3 }).unwrap()).unwrap();
    assert!(old.tests.is_empty());
}
