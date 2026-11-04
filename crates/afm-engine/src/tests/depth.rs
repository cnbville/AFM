//! The players tally's depth counts (lane NB4): the spot codes match the
//! engine's, the sparse helpers add and take away, the facet grades add up
//! to the overall grade, and a game's counts agree with its stat book.

use crate::game::playbook::ORole;
use crate::game::quick_game;
use crate::game::scrimmage::DSlot;
use crate::game::stats::{FACETS, PlayType};
use crate::game::synthetic::synthetic_league;
use crate::player::Level;
use crate::world::analytics::{self as an, Cm};
use crate::world::tally::depth::{self as dp, Dc, dslot, orole};
use crate::world::tally::expect;

#[test]
fn spot_codes_match_the_engine() {
    let o = [
        (ORole::QB, orole::QB),
        (ORole::RB, orole::RB),
        (ORole::FB, orole::FB),
        (ORole::X, orole::X),
        (ORole::Z, orole::Z),
        (ORole::Slot, orole::SLOT),
        (ORole::Slot2, orole::SLOT2),
        (ORole::TE, orole::TE),
        (ORole::TE2, orole::TE2),
        (ORole::TE3, orole::TE3),
        (ORole::LT, orole::LT),
        (ORole::LG, orole::LG),
        (ORole::C, orole::C),
        (ORole::RG, orole::RG),
        (ORole::RT, orole::RT),
        (ORole::Slot3, orole::SLOT3),
    ];
    for (r, c) in o {
        assert_eq!(r as u8, c, "{r:?}");
        assert!((c as usize) < orole::N);
    }
    let d = [
        (DSlot::LE, dslot::LE),
        (DSlot::RE, dslot::RE),
        (DSlot::DTL, dslot::DTL),
        (DSlot::DTR, dslot::DTR),
        (DSlot::NT, dslot::NT),
        (DSlot::Sam, dslot::SAM),
        (DSlot::Mike, dslot::MIKE),
        (DSlot::Will, dslot::WILL),
        (DSlot::CBL, dslot::CBL),
        (DSlot::CBR, dslot::CBR),
        (DSlot::NCB, dslot::NCB),
        (DSlot::DCB, dslot::DCB),
        (DSlot::FS, dslot::FS),
        (DSlot::SS, dslot::SS),
    ];
    for (s, c) in d {
        assert_eq!(s as u8, c, "{s:?}");
        assert!((c as usize) < dslot::N);
    }
    // The codes don't run into each other.
    assert!(Dc::ALL.len() <= dp::THROW as usize);
    const { assert!(dp::THROW + 3 * 48 <= dp::GAP && dp::GAP + 30 <= dp::ROUTE) };
    assert!(dp::ROUTE + 5 * dp::ROUTES.len() as u16 <= dp::PMAP && dp::PMAP + 3 * 8 <= dp::FACET);
    assert!(dp::FACET + 2 * FACETS.len() as u16 <= dp::ALIGN_O && dp::ALIGN_O + orole::N as u16 <= dp::ALIGN_D);
    assert!(dp::ALIGN_D + dslot::N as u16 <= dp::FG && dp::FG as usize + 8 <= dp::LEN);
}

#[test]
fn sparse_counts_add_and_take_away() {
    let mut a: dp::Depth = vec![(1, 2.0), (5, 1.0)];
    dp::add(&mut a, &[(0, 1.0), (5, 2.0), (9, 4.0)]);
    assert_eq!(a, vec![(0, 1.0), (1, 2.0), (5, 3.0), (9, 4.0)]);
    assert_eq!(dp::get(&a, 5), 3.0);
    assert_eq!(dp::get(&a, 7), 0.0);
    dp::sub(&mut a, &[(5, 3.0), (9, 1.0)]);
    assert_eq!(dp::get(&a, 9), 3.0);
    assert_eq!(dp::get(&a, 5), 0.0);
    // A longest return keeps the longest across games, and stays when
    // the last days' games come off.
    let long = Dc::KrLong as u16;
    let mut s: dp::Depth = Vec::new();
    dp::add_game(&mut s, &[(long, 40.0)]);
    dp::add_game(&mut s, &[(long, 25.0)]);
    assert_eq!(dp::get(&s, long), 40.0);
    dp::sub(&mut s, &[(long, 25.0)]);
    assert_eq!(dp::get(&s, long), 40.0);
}

#[test]
fn a_games_depth_counts_agree_with_its_book() {
    for level in [Level::FBS, Level::NFL] {
        let teams = synthetic_league(level, 32, 11);
        let mut runs = 0.0;
        let mut ryoe = 0.0;
        for g in 0..16 {
            let (h, a) = ((g * 7) % 32, (g * 13 + 5) % 32);
            let a = if a == h { (a + 1) % 32 } else { a };
            let res = quick_game(&teams[h], &teams[a], level, 9_100_003 + g as u64);
            let s = &res.stats;
            let d = dp::game(s);
            for side in 0..2 {
                for (id, l) in &s.players[side] {
                    let Some(x) = d.get(&(side, *id)) else { continue };
                    let c = |k: Dc| dp::get(x, k as u16);
                    assert_eq!(c(Dc::Att), l.att as f64, "{level:?} {id} attempts");
                    assert_eq!(c(Dc::Cmp), l.cmp as f64, "{level:?} {id} completions");
                    assert_eq!(c(Dc::Tgt), l.targets as f64, "{level:?} {id} targets");
                    assert_eq!(c(Dc::Rec), l.rec as f64, "{level:?} {id} catches");
                    // The facet grades add up to the overall grade.
                    let plays: f64 = (0..FACETS.len()).map(|f| dp::get(x, dp::FACET + 2 * f as u16 + 1)).sum();
                    let sum: f64 = (0..FACETS.len()).map(|f| dp::get(x, dp::FACET + 2 * f as u16)).sum();
                    assert_eq!(plays, l.graded as f64, "{level:?} {id} graded plays");
                    assert!((sum - l.grade_sum).abs() < 0.01, "{level:?} {id} grade {sum} against {}", l.grade_sum);
                    // The throw grid holds every aimed throw.
                    let grid: f64 = (0..12).map(|k| dp::get(x, dp::throw_code(0, k / 3, k % 3, 0))).sum();
                    assert_eq!(grid, c(Dc::Aimed), "{level:?} {id} throw grid");
                    let gaps: f64 = (0..5).map(|k| dp::get(x, dp::gap_code(k, 0))).sum();
                    assert_eq!(gaps, c(Dc::RAtt), "{level:?} {id} run paths");
                    runs += c(Dc::RAtt);
                    ryoe += c(Dc::RYds) - c(Dc::RXYds);
                }
            }
            // Every scrimmage snap puts 11 men somewhere on each side.
            let snaps = s
                .plays
                .iter()
                .filter(|r| {
                    !r.nullified
                        && matches!(r.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble)
                })
                .filter(|r| r.roles[0].len() == r.on_field[0].len())
                .count() as f64;
            let off: f64 =
                d.values().map(|x| (0..orole::N as u16).map(|k| dp::get(x, dp::ALIGN_O + k)).sum::<f64>()).sum();
            assert!(snaps > 60.0 && off >= 10.0 * snaps, "{level:?} game {g}: {off} spots on {snaps} snaps");
        }
        // The expected rush model is the engine's own: its average back
        // sits near zero over expected.
        assert!(runs > 300.0 && (ryoe / runs).abs() < 0.8, "{level:?}: {:.2} RYOE a carry", ryoe / runs);
    }
}

#[test]
fn every_view_reads_its_numbers() {
    for g in 0..an::GROUPS.len() {
        let v = an::views(g);
        assert!(!v.is_empty() && v[0].1 == an::charted(g), "group {g}");
        for (name, card) in v {
            assert!(!card.is_empty() && card.len() <= an::CARD_MAX, "{name}: {} numbers", card.len());
        }
    }
    // A deep number reads the depth counts and only them.
    assert!(Cm::Ryoe.deep() && !Cm::Ypc.deep() && !Cm::Returns.deep());
    assert_eq!(Cm::Ryoe.value(&|_| 1.0, 1.0), None);
    let d = |c: u16| match c {
        c if c == Dc::RAtt as u16 => 10.0,
        c if c == Dc::RYds as u16 => 50.0,
        c if c == Dc::RXYds as u16 => 42.0,
        _ => 0.0,
    };
    assert!((Cm::Ryoe.value_d(&d).unwrap() - 0.8).abs() < 1e-9);
    // Features have their models' lengths.
    assert_eq!(expect::xrush_features(7, true, 50.0, 10.0).len(), expect::XRUSH_TERMS);
    assert_eq!(expect::run_gap("Inside zone left"), 1);
    assert_eq!(expect::run_gap("Toss right"), 4);
    assert_eq!(expect::run_gap("QB draw"), 2);
}
