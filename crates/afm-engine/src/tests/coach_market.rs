//! The coaching market (realism audit item 6): a coach's pay is bid up by
//! the market and his results, not set by the job alone.

use crate::world::coach::{Employer, Role};
use crate::world::data::Division;
use crate::world::folk::hiring::pay_now;
use crate::world::staff::{hire, leave, pay, salary};
use crate::world::{World, WorldOptions};

fn tiny() -> World {
    World::empty(WorldOptions { scale: 0.001, history_years: 0, start_year: 2037, ..Default::default() })
}

fn fbs(w: &World) -> Vec<usize> {
    (0..w.programs.len())
        .filter(|&p| w.programs[p].division == Division::FBS && w.programs[p].head_coach.is_some())
        .collect()
}

#[test]
fn the_hottest_head_coach_is_read_over_the_richest_line() {
    let mut w = tiny();
    let progs = fbs(&w);
    let top_line = progs.iter().map(|&p| salary(&w, Employer::College(p), Role::HeadCoach)).fold(0.0, f64::max);
    // One head coach made the hottest name in the country, the rest cold.
    for &q in &progs {
        let h = w.programs[q].head_coach.unwrap();
        w.coaches[h].reputation = 4.0;
        w.coaches[h].recent = vec![-0.2, -0.2, -0.2];
    }
    let p = progs[progs.len() / 2];
    let c = w.programs[p].head_coach.unwrap();
    w.coaches[c].reputation = 20.0;
    w.coaches[c].recent = vec![0.3, 0.3, 0.3];
    pay::refresh(&mut w);
    let read = pay::read(&w, c).expect("a read");
    assert!(read >= top_line * 1.2, "the top name is bid over the top line: {read} vs {top_line}");
    // His school can't pay all of it: a re-signing stops at its top.
    let now = pay_now(&w, c);
    let renewed = pay::renewed(&w, c, now, 1.05);
    assert!(renewed >= now * 1.05 - 1.0);
    assert!(renewed <= pay::cap(&w, Employer::College(p), Role::HeadCoach).max(now) + 1.0);
}

#[test]
fn an_offer_beats_his_pay_and_a_counter_bids_it_up() {
    let mut w = tiny();
    let progs = fbs(&w);
    pay::refresh(&mut w);
    let mut countered = 0;
    for (k, &p) in progs.iter().enumerate() {
        let Some(c) = w.programs[p].oc else { continue };
        w.coaches[c].reputation = 16.0;
        let q = progs[(k + 1) % progs.len()];
        let now = pay_now(&w, c);
        let offer = pay::offer(&w, c, Employer::College(q), Role::OffensiveCoordinator, 0);
        let top = pay::cap(&w, Employer::College(q), Role::OffensiveCoordinator);
        assert!(offer >= (now * 1.15).min(top) - 1.0, "a move across pays: {offer} vs {now}");
        if let Some((last, stay)) = pay::auction(&w, c, offer, Employer::College(q), Role::OffensiveCoordinator) {
            countered += 1;
            assert!(last >= offer - 1.0, "the suitor's last bid is no lower than its first");
            assert!(stay > offer, "his school went past the first offer");
        }
    }
    assert!(countered > 0, "some schools counter");
}

#[test]
fn an_nfl_head_coach_signs_a_deal_owed_in_full() {
    let mut w = tiny();
    let t = (0..w.nfl.len()).find(|&t| w.nfl[t].head_coach.is_some()).unwrap();
    let old = w.nfl[t].head_coach.unwrap();
    let c = w.nfl[t].oc.expect("an offensive coordinator");
    leave(&mut w, old, "fired");
    leave(&mut w, c, "left for another job");
    hire(&mut w, c, Employer::Nfl(t), Role::HeadCoach);
    let k = w.coaches[c].contract.expect("a contract");
    assert_eq!(k.buyout_share, 1.0);
    assert!(k.through >= w.year + 3);
    assert!(k.salary > 0.0);
}

/// Realism audit item 9: a college's pay line reads what its department
/// earns, not its prestige, so a richer school's line is never below a
/// poorer one's at its level, a Power 4 school pays several times a Group
/// of 5 school, and the most it pays one man is its line stretched.
#[test]
fn the_pay_line_follows_what_the_department_earns() {
    use crate::world::finance::{college, data};
    let mut w = World::empty(WorldOptions { scale: 0.01, history_years: 0, start_year: 2026, ..Default::default() });
    college::open(&mut w);
    let mut rows: Vec<(f64, f64, u8)> = (0..w.programs.len())
        .filter(|&p| w.programs[p].division == Division::FBS)
        .map(|p| {
            let d = &w.finance.college[p];
            let earned = college::earned(&d.budget, d) / data::college_index(d.budget.year);
            (earned, salary(&w, Employer::College(p), Role::HeadCoach), college::tier(&w, p, w.year))
        })
        .collect();
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    for x in rows.windows(2) {
        assert!(x[1].1 >= x[0].1, "more money, no less pay: {:?} then {:?}", x[0], x[1]);
    }
    let mean = |t: u8| {
        let v: Vec<f64> = rows.iter().filter(|r| r.2 == t).map(|r| r.1).collect();
        v.iter().sum::<f64>() / v.len() as f64
    };
    assert!(mean(0) > 3.0 * mean(1), "Power 4 {} against Group of 5 {}", mean(0), mean(1));
    let p = (0..w.programs.len()).find(|&p| w.programs[p].division == Division::FBS).unwrap();
    let e = Employer::College(p);
    assert!((pay::cap(&w, e, Role::HeadCoach) - 1.4 * salary(&w, e, Role::HeadCoach)).abs() < 1.0);
}
