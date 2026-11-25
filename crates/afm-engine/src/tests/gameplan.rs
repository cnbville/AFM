//! The deeper game plan (update 2, lane 5): the head coach's guidelines
//! lean his coordinators by their firmness and never bind them.

use crate::game::calling::{DefCtx, Memory, Situation, call_defense};
use crate::game::guide::{BaseFront, Firm, Want, expected_man, habit_man};
use crate::game::playbook::{Backfield, DefPersonnel, Formation, Personnel};
use crate::game::{OffScheme, Shell, Staff};
use crate::rng::Rng;

fn sit(down: u8, togo: f64, yl: f64) -> Situation {
    Situation {
        down,
        togo,
        yardline_100: yl,
        score_diff: 0,
        quarter: 2,
        half_left: 600.0,
        game_left: 2400.0,
        timeouts: 3,
        opp_timeouts: 3,
        ease: 0.0,
        concede: false,
        def_ease: 0.0,
        def_concede: false,
        college: false,
        lean: 0.0,
        weather: 0.0,
        half_go: false,
        fourth: Default::default(),
        trying: false,
    }
}

/// The share of man coverage and of nickel over a spread of normal downs.
fn calls(staff: &Staff, personnel: Personnel, seed: u32) -> (f64, f64) {
    let mut rng = Rng::new(seed);
    let mem = Memory::default();
    let off = Formation::plain(personnel, Backfield::Shotgun, 1.0);
    let downs = [(1, 10.0, 70.0), (2, 7.0, 55.0), (2, 3.0, 45.0), (3, 6.0, 40.0), (1, 10.0, 35.0), (3, 2.0, 60.0)];
    let (mut man, mut nickel, mut n) = (0usize, 0usize, 0usize);
    for i in 0..6000 {
        let (d, t, y) = downs[i % downs.len()];
        let c = call_defense(staff, &mem, &sit(d, t, y), &off, &DefCtx::default(), &mut rng);
        n += 1;
        man += usize::from(c.coverage.is_man());
        nickel += usize::from(c.personnel == DefPersonnel::Nickel);
    }
    (man as f64 / n as f64, nickel as f64 / n as f64)
}

#[test]
fn a_man_guideline_pulls_by_its_firmness() {
    // A Cover 3 coordinator left alone plays mostly zone.
    let mut staff = Staff::average(OffScheme::WestCoast);
    staff.shell = Shell::Cover3;
    let (own, _) = calls(&staff, Personnel::P11, 7);
    assert!(own < 0.4, "a Cover 3 staff plays mostly zone: {own:.2}");
    let mut last = own;
    for firm in [Firm::Theirs, Firm::Lean, Firm::Firm] {
        staff.guide.man = Want::new(0.7, firm);
        let (m, _) = calls(&staff, Personnel::P11, 7);
        println!("{firm:?}: man {m:.2} (expected {:.2})", expected_man(staff.shell, &staff.guide));
        assert!(m > last + 0.02, "{firm:?} pulls further than the one before: {m:.2} after {last:.2}");
        last = m;
    }
    // A firm guideline from a roster built for man: mostly man, not
    // mostly zone drops.
    assert!(last > 0.5, "a firm man guideline ends in mostly man: {last:.2}");
    // A lean lands about halfway between the habit and the guideline.
    staff.guide.man = Want::new(0.7, Firm::Lean);
    let (m, _) = calls(&staff, Personnel::P11, 8);
    let half = habit_man(Shell::Cover3) + 0.55 * (0.7 - habit_man(Shell::Cover3));
    assert!((m - half).abs() < 0.12, "a lean lands near halfway: {m:.2} against {half:.2}");
}

#[test]
fn a_four_three_team_still_goes_to_nickel_against_three_receivers() {
    let mut staff = Staff::average(OffScheme::WestCoast);
    let (_, own) = calls(&staff, Personnel::P11, 9);
    staff.guide.front = Want::new(BaseFront::FourThree, Firm::Firm);
    let (_, firm) = calls(&staff, Personnel::P11, 9);
    assert!(firm > 0.6, "a firm 4-3 still plays nickel against 11 personnel: {firm:.2}");
    assert!(firm < own, "but a little less than his own habit: {firm:.2} against {own:.2}");
    // A nickel base plays nickel against two receivers too.
    staff.guide.front = Want::new(BaseFront::FourTwoFive, Firm::Firm);
    let (_, two) = calls(&staff, Personnel::P12, 9);
    assert!(two > 0.5, "a 4-2-5 base is in nickel against 12 personnel: {two:.2}");
    // With no front guideline a staff answers two receivers as real NFL
    // staffs do (realism audit item 3, `PACKAGES`: a nickel on 44% of early
    // downs, more on third and medium), well under a nickel base.
    staff.guide.front = Want::default();
    let (_, base) = calls(&staff, Personnel::P12, 9);
    assert!((0.35..0.6).contains(&base), "a staff with no front guideline plays the real mix against 12: {base:.2}");
    assert!(base < two - 0.2, "and well under a 4-2-5 base: {base:.2} against {two:.2}");
}

/// Every staff drafts a call sheet (Cain's rule: the same rules for every
/// team): two college or NFL teams with no week's plan both draft one at
/// kickoff, and both offenses run their opening script and call from their
/// boxes. High school AI staffs don't (`sheet::KICKOFF`, for time).
#[test]
fn every_team_calls_from_a_sheet() {
    use crate::game::quick_game;
    use crate::game::synthetic::synthetic_league;
    use crate::player::Level;
    for level in [Level::HS, Level::FBS, Level::NFL] {
        let teams = synthetic_league(level, 8, 21);
        // Per side: script calls and box calls.
        let mut on = [[0usize; 2]; 2];
        for g in 0..4 {
            let res = quick_game(&teams[g * 2], &teams[g * 2 + 1], level, 4_400_011 + g as u64);
            for r in &res.stats.plays {
                if r.call_why == "the opening script" {
                    on[r.offense][0] += 1;
                }
                if r.call_why == "on the call sheet" {
                    on[r.offense][1] += 1;
                }
            }
        }
        for (side, n) in on.iter().enumerate() {
            println!("{level:?} side {side}: {} script calls, {} box calls in 4 games", n[0], n[1]);
            if level == Level::HS {
                assert_eq!(*n, [0, 0], "a high school AI staff drafts no sheet at kickoff");
            } else {
                assert!(n[0] >= 8 && n[1] >= 40, "{level:?} side {side} calls from its sheet: {n:?}");
            }
        }
    }
}

/// The backs a staff plays follow the real NFL's table by the receivers and
/// the down (realism audit item 3, `PACKAGES`): against three receivers a
/// nickel on early downs and a dime on 45% of third and longs, against four
/// a nickel mostly (it used to be a dime always).
#[test]
fn the_backs_follow_the_receivers_and_the_down() {
    let staff = Staff::average(OffScheme::WestCoast);
    let mem = Memory::default();
    let share = |personnel: Personnel, down: u8, togo: f64, seed: u32| {
        let mut rng = Rng::new(seed);
        let off = Formation::plain(personnel, Backfield::Shotgun, 1.0);
        let mut n = [0usize; 3];
        for _ in 0..4000 {
            let c = call_defense(&staff, &mem, &sit(down, togo, 60.0), &off, &DefCtx::default(), &mut rng);
            match c.personnel {
                DefPersonnel::Dime => n[2] += 1,
                DefPersonnel::Nickel => n[1] += 1,
                _ => n[0] += 1,
            }
        }
        n.map(|x| x as f64 / 4000.0)
    };
    let early = share(Personnel::P11, 1, 10.0, 3);
    println!("11 personnel, 1st and 10: {early:.3?}");
    assert!(early[1] > 0.8 && early[2] < 0.15, "a nickel on early downs against three: {early:.3?}");
    let long = share(Personnel::P11, 3, 9.0, 4);
    println!("11 personnel, 3rd and 9: {long:.3?}");
    assert!((0.35..0.55).contains(&long[2]), "a dime on 45% of third and longs: {long:.3?}");
    let four = share(Personnel::P10, 1, 10.0, 5);
    println!("10 personnel, 1st and 10: {four:.3?}");
    assert!(four[1] > 0.75, "a nickel mostly against four receivers: {four:.3?}");
}
