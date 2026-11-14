//! The public bar you set (missed features item 4): `Mine.expectation` is
//! held against your seasons by the people who review you, for a while.

use crate::world::folk::boss::{PUBLIC_BAR, public_bar};
use crate::world::{World, WorldOptions};

fn tiny() -> World {
    World::empty(WorldOptions { scale: 0.001, history_years: 0, start_year: 2037, ..Default::default() })
}

#[test]
fn a_raised_bar_weighs_three_seasons_then_fades() {
    let mut w = tiny();
    w.folk.mine.expectation = 0.5;
    let full = PUBLIC_BAR * 0.5;
    for t in 0..=3 {
        assert!((public_bar(&w, t) - full).abs() < 1e-6, "season {t}");
    }
    for t in 4..=5 {
        assert!((public_bar(&w, t) - 0.5 * full).abs() < 1e-6, "season {t}");
    }
    assert_eq!(public_bar(&w, 6), 0.0, "then your record is the bar");
}

#[test]
fn a_lowered_bar_buys_only_the_first_season() {
    let mut w = tiny();
    w.folk.mine.expectation = -0.15;
    assert!(public_bar(&w, 1) < 0.0, "the first review reads kinder");
    assert_eq!(public_bar(&w, 2), 0.0);
}

#[test]
fn no_words_no_bar() {
    let w = tiny();
    for t in 0..8 {
        assert_eq!(public_bar(&w, t), 0.0);
    }
}
