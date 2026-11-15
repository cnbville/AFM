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
                crate::craft::year(&mut p, coaching, if a < 18 { Level::HS } else { Level::FBS }, 1.0, Some(10.0));
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

/// A 20-year-old tackle with a typical overset (0.5), coachability 16 and a
/// flaw whose ease for him is about average.
fn tackle(coachability: f64, age: u32) -> crate::Player {
    let mut p = raw(400, Position::OT)
        .into_iter()
        .find(|p| (0.95..1.05).contains(&crate::craft::ease(p, Craft::Oversets)))
        .expect("an average ease");
    p.age = age;
    p.personality.coachability = coachability;
    p.craft = Crafts { drawn: true, ..Crafts::default() };
    p.craft.set(Craft::Oversets, 0.5);
    p
}

/// Weeks of work until it holds in games (`None`: not in two years).
fn weeks_to_hold(p: &mut crate::Player, t: &crate::craft::Teach) -> Option<u32> {
    (1..=104).find(|_| crate::craft::work_week(p, Craft::Oversets, t).is_some_and(|s| s.1 == Stage::Games))
}

use crate::craft::{Stage, Teach};

#[test]
fn a_coachable_twenty_year_old_with_a_strong_teacher_fixes_a_flaw_in_an_offseason() {
    let strong = Teach { teaching: 15.0, minutes: 30.0, buy_in: 1.0 };
    let mut p = tackle(16.0, 20);
    let mut saw_practice_first = false;
    let mut dip = false;
    for wk in 1..=40 {
        let (_, after) = crate::craft::work_week(&mut p, Craft::Oversets, &strong).unwrap();
        let it = *p.craft.get(Craft::Oversets).unwrap();
        // Practice moves first and games follow.
        saw_practice_first |= it.prac() + 0.05 < it.sev();
        // The learning dip: the first weeks cost a quarter more in games.
        if wk == 2 {
            dip = p.craft.worn().pts(Craft::Oversets, 0.0) < -crate::craft::FULL * it.sev() * 1.2;
        }
        if after == Stage::Games {
            println!("holds in games after {wk} weeks");
            assert!((11..=17).contains(&wk), "{wk} weeks (sizes item 2: 12 to 16)");
            break;
        }
        assert!(wk < 40, "never held");
    }
    assert!(saw_practice_first && dip);
    // A poor teacher or a low coachability: a year or more.
    let poor = Teach { teaching: 7.0, ..strong };
    let slow = weeks_to_hold(&mut tackle(16.0, 20), &poor);
    assert!(slow.is_none_or(|w| w >= 40), "poor teacher {slow:?}");
    let low = weeks_to_hold(&mut tackle(6.0, 20), &strong);
    assert!(low.is_none_or(|w| w >= 40), "low coachability {low:?}");
    // A 30-year-old takes about twice as long.
    let old = weeks_to_hold(&mut tackle(16.0, 30), &strong).unwrap();
    let young = weeks_to_hold(&mut tackle(16.0, 20), &strong).unwrap();
    println!("20: {young} weeks, 30: {old}");
    assert!((1.6..=2.4).contains(&(old as f64 / young as f64)), "{old} vs {young}");
    // More minutes, sooner.
    let more = weeks_to_hold(&mut tackle(16.0, 20), &Teach { minutes: 60.0, ..strong }).unwrap();
    assert!(more < young);
}

#[test]
fn the_wrong_flaw_fixes_nothing_and_a_new_tool_starts_from_nothing() {
    let t = Teach { teaching: 14.0, minutes: 30.0, buy_in: 1.0 };
    let mut p = tackle(14.0, 21);
    let before = p.craft;
    // He doesn't carry it: the staff named the wrong one.
    assert!(crate::craft::work_week(&mut p, Craft::CatchesPunch, &t).is_none());
    assert_eq!(p.craft, before);
    // A tool: learned in practice first, then in games.
    for _ in 0..30 {
        crate::craft::work_week(&mut p, Craft::RefitsHands, &t);
    }
    let it = *p.craft.get(Craft::RefitsHands).expect("learning it");
    assert!(it.prac() > it.sev() && it.sev() > 0.1, "{} {}", it.prac(), it.sev());
    assert!(p.craft.worn().pts(Craft::RefitsHands, 0.0) > 0.0);
    // Under stress the new move fades back toward where it began.
    assert!(p.craft.worn().pts(Craft::RefitsHands, 1.0) < p.craft.worn().pts(Craft::RefitsHands, 0.0));
}

#[test]
fn the_year_leaves_a_fix_your_plan_worked_week_by_week() {
    let t = Teach { teaching: 12.0, minutes: 30.0, buy_in: 0.8 };
    let mut p = tackle(12.0, 20);
    p.craft.set(Craft::HighPadBlock, 0.5);
    for _ in 0..8 {
        crate::craft::work_week(&mut p, Craft::Oversets, &t);
    }
    let worked = *p.craft.get(Craft::Oversets).unwrap();
    crate::craft::year(&mut p, 12.0, Level::FBS, 1.0, None);
    let after = *p.craft.get(Craft::Oversets).unwrap();
    assert_eq!(after.sev, worked.sev, "the weeks moved it, not the year");
    assert!(!after.worked() && after.weeks() == worked.weeks(), "no second dip next year");
    // The one nobody worked moved by the year's slow background only.
    assert!(p.craft.sev(Craft::HighPadBlock) <= 0.5);
}

#[test]
fn a_point_is_one_byte_and_two_points_share_the_minutes() {
    use crate::attributes::Attr;
    use crate::craft::Point;
    for pt in [Point::Rating(Attr::PassBlocking), Point::Craft(Craft::Oversets), Point::Craft(Craft::CarriesLoose)] {
        assert_eq!(Point::from_code(pt.code()), Some(pt));
    }
    assert_eq!(Point::Craft(Craft::Oversets).rating(), Craft::Oversets.owner());
    let f = crate::development::Focus::two(Attr::PassBlocking, Some(Attr::RunBlocking), 40.0);
    assert_eq!(f.minutes_on(Attr::PassBlocking), 20.0);
    assert_eq!(f.minutes_on(Attr::Speed), 0.0);
    let same = crate::development::Focus::two(Attr::PassBlocking, Some(Attr::PassBlocking), 40.0);
    assert_eq!(same.minutes_on(Attr::PassBlocking), 40.0);
}

#[test]
fn a_moulding_staff_works_its_system_and_an_adapting_one_his_biggest() {
    let mut c = Crafts { drawn: true, ..Crafts::default() };
    // A big habit in a rating that matters less at tackle, and a smaller
    // one in pass blocking.
    c.set(Craft::HeavyClimb, 0.7);
    c.set(Craft::Oversets, 0.5);
    c.set(Craft::HighPadBlock, 0.3);
    let adapt = crate::craft::staff_points(&c, Position::OT, 1.0);
    assert_eq!(adapt, [Some(Craft::HeavyClimb), Some(Craft::Oversets)]);
    let mould = crate::craft::staff_points(&c, Position::OT, 20.0);
    println!("moulding staff: {mould:?}");
    assert!(mould.contains(&Some(Craft::Oversets)));
}
