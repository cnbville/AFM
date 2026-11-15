//! Flaws and tools (individual practice part 2,
//! `docs/reference/how-it-works/flaws-and-tools.md`): their size in the
//! engine, the stress return, the learning dip, where they come from and
//! how a staff changes them, and the level-centred read.

use crate::craft::{Craft, Crafts, FULL, Worn};
use crate::game::roster::OffScheme;
use crate::game::synthetic::{synthetic_team, typical_craft};
use crate::player::{Level, ProspectOptions, generate_prospect};
use crate::{Position, Rng};

fn raw(n: u32, pos: Position) -> Vec<crate::Player> {
    let mut rng = Rng::new(41);
    (0..n)
        .map(|i| {
            generate_prospect(
                &mut rng,
                ProspectOptions { id: Some(700_000 + i), position: Some(pos), age: Some(14), talent_z: Some(1.2) },
            )
        })
        .collect()
}

#[test]
fn a_full_flaw_costs_five_points_and_comes_back_a_third_under_stress() {
    let mut c = Crafts::default();
    c.set(Craft::Oversets, 1.0);
    let w = c.worn();
    assert!((w.pts(Craft::Oversets, 0.0) + FULL).abs() < 0.05, "{}", w.pts(Craft::Oversets, 0.0));
    assert_eq!(w.pts(Craft::Spin, 0.0), 0.0, "one he doesn't carry reads nothing");
    // Fixed down to 0.4 from 1.0: under full stress a third of the way back.
    let mut w = Worn::default();
    w.code[0] = Craft::Oversets as u8 + 1;
    w.now[0] = -2.0;
    w.was[0] = -5.0;
    assert!((w.pts(Craft::Oversets, 1.0) + 3.0).abs() < 1e-6);
    assert!((w.pts(Craft::Oversets, 0.0) + 2.0).abs() < 1e-6);
    // A tool adds.
    let mut t = Crafts::default();
    t.set(Craft::LongArm, 0.5);
    assert!((t.worn().pts(Craft::LongArm, 0.0) - 2.5).abs() < 0.05);
}

#[test]
fn a_raw_freshman_carries_three_to_five_flaws_and_few_tools() {
    let mut flaws = 0.0;
    let mut tools = 0.0;
    let mut n = 0.0;
    for pos in [Position::QB, Position::WR, Position::OT, Position::EDGE, Position::LB, Position::CB] {
        for p in raw(120, pos) {
            assert!(p.craft.iter().all(|(c, _)| c.fits(pos)), "only his position's");
            flaws += p.craft.flaws() as f64;
            tools += p.craft.tools() as f64;
            n += 1.0;
        }
    }
    println!("at 14: {:.2} flaws, {:.2} tools", flaws / n, tools / n);
    assert!((2.8..=5.0).contains(&(flaws / n)), "{}", flaws / n);
    assert!(tools / n < 0.3, "{}", tools / n);
}

#[test]
fn a_good_staff_fixes_flaws_and_a_poor_one_lets_them_grow() {
    let size = |coaching: f64| {
        let mut total = 0.0;
        for mut p in raw(150, Position::CB) {
            for a in 14..22 {
                p.age = a;
                crate::craft::year(&mut p, coaching, if a < 18 { Level::HS } else { Level::FBS }, 1.0);
            }
            total += p.craft.iter().filter(|(c, _)| c.is_flaw()).map(|(_, it)| it.sev()).sum::<f64>();
        }
        total / 150.0
    };
    let (poor, mid, good) = (size(5.0), size(11.0), size(16.0));
    println!("flaw size held at 22: staff 5 {poor:.2}, 11 {mid:.2}, 16 {good:.2}");
    assert!(good < mid && mid < poor, "{good} {mid} {poor}");
    assert!(good > 0.1, "even a great staff leaves some habits: {good}");
}

#[test]
fn the_engine_reads_them_against_the_level_so_a_level_plays_as_it_did() {
    // The typical starter's own points in a flaw sit near what the level's
    // men carry: their mean read against it is about 0.
    for level in [Level::HS, Level::NFL] {
        let mut sum = 0.0;
        let mut n = 0.0;
        for k in 0..16u32 {
            let t = synthetic_team("T", level, 0.0, OffScheme::WestCoast, 4100 + k);
            for p in t.players.iter().filter(|p| p.position == Position::CB) {
                sum += p.craft_vs(Craft::Peeks, level, 0.0);
                n += 1.0;
            }
        }
        let typ = typical_craft(level, Position::CB, Craft::Peeks);
        println!("{level:?}: typical Peeks {typ:.2}, mean against it {:.2}", sum / n);
        assert!(typ < 0.0, "corners carry it: {typ}");
        assert!((sum / n).abs() < 0.6, "{}", sum / n);
    }
}
