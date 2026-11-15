//! College money and the support staff market (realism audit items 9 and
//! 10): football's staff and recruiting lines come from what the
//! department earns, with no tier top; support heads are people on a
//! market, paid by what their post can offer, and a new head coach brings
//! his own strength coach.

use super::legacy::seat;
use crate::world::World;
use crate::world::data::Division;
use crate::world::finance::college::{self, RECRUITING, STAFF};
use crate::world::people::{Dept, staff_support, support_bosses};

/// A college seat with its books open.
fn world() -> World {
    let mut w = seat("college");
    crate::world::finance::ensure(&mut w);
    w
}

fn fbs(w: &World) -> Vec<usize> {
    (0..w.programs.len())
        .filter(|&p| w.programs[p].plays(w.year) && w.programs[p].division == Division::FBS)
        .filter(|&p| w.finance.college.get(p).is_some_and(|d| d.budget.year != 0))
        .collect()
}

#[test]
fn the_staff_line_is_a_share_of_what_the_department_earns() {
    let mut w = world();
    let year = w.year;
    let mine = w.seat.and_then(|s| s.program).unwrap();
    let list: Vec<usize> = fbs(&w).into_iter().filter(|&p| p != mine).collect();
    assert!(list.len() > 20, "FBS programs with books");
    let mut ratios = Vec::new();
    let mut lines = Vec::new();
    for &p in &list {
        college::open_year(&mut w, p, year);
        let d = &w.finance.college[p];
        let earned = college::earned(&d.budget, d);
        let push = college::football_push(w.programs[p].prestige, d.reserves as f64, earned);
        let t = college::tier(&w, p, year);
        let staff = d.budget.l[STAFF] as f64;
        ratios.push((t, staff / (earned * push), d.budget.l[RECRUITING] as f64 / (earned * push)));
        lines.push((earned, staff));
    }
    // One share per tier: the same for every program of it.
    for t in 0..2u8 {
        let mine: Vec<&(u8, f64, f64)> = ratios.iter().filter(|r| r.0 == t).collect();
        if mine.is_empty() {
            continue;
        }
        let (s0, r0) = (mine[0].1, mine[0].2);
        assert!(mine.iter().all(|r| (r.1 - s0).abs() < 1e-3 && (r.2 - r0).abs() < 1e-3), "tier {t}: one share");
    }
    // The richest department spends the most: no tier top flattens it.
    lines.sort_by(|a, b| b.0.total_cmp(&a.0));
    let top = lines[0].1;
    let mid = lines[lines.len() / 2].1;
    assert!(top > 1.3 * mid, "the richest spends well above the middle: {top:.0} against {mid:.0}");
}

#[test]
fn heads_are_paid_by_their_post_and_the_richer_posts_pay_more() {
    let mut w = world();
    staff_support(&mut w);
    let list = fbs(&w);
    for &p in &list {
        for d in Dept::ALL {
            let s = d.post(&w.programs[p].support);
            assert!(s.rating > 0 && s.pay > 0.0 && s.born > 0, "every post filled and paid");
        }
    }
    let mut rows: Vec<(f64, f64)> = list
        .iter()
        .map(|&p| (w.finance.college[p].budget.l[STAFF] as f64, w.programs[p].support.strength.pay as f64))
        .collect();
    rows.sort_by(|a, b| b.0.total_cmp(&a.0));
    let half = rows.len() / 2;
    let rich = rows[..half].iter().map(|r| r.1).sum::<f64>() / half as f64;
    let poor = rows[half..].iter().map(|r| r.1).sum::<f64>() / (rows.len() - half) as f64;
    assert!(rich > 1.3 * poor, "the richer half pays its strength heads more: {rich:.0} against {poor:.0}");
    // The NFL's posts are filled too.
    let t = (0..w.clubs.len()).find(|&t| crate::world::franchise::active_in(&w, t, w.year)).unwrap();
    assert!(w.clubs[t].support.strength.pay > 0.0 && w.clubs[t].support.academic.rating == 0);
}

#[test]
fn a_new_head_coach_brings_his_own_strength_coach() {
    let mut w = world();
    let mine = w.seat.and_then(|s| s.program).unwrap();
    staff_support(&mut w);
    support_bosses(&mut w);
    let list: Vec<usize> =
        fbs(&w).into_iter().filter(|&p| p != mine && w.programs[p].head_coach.is_some()).take(24).collect();
    assert!(list.len() >= 24);
    // Twelve head coaches move: the first of each pair takes the second's job.
    let mut moves = Vec::new();
    for pair in list.chunks(2) {
        let (from, to) = (pair[0], pair[1]);
        let hc = w.programs[from].head_coach.unwrap();
        let his = w.programs[from].support.strength.clone();
        assert_eq!(his.coach, Some(hc), "the strength head is his head coach's man");
        w.programs[from].head_coach = None;
        w.programs[to].head_coach = Some(hc);
        moves.push((from, to, hc, his));
    }
    let before = w.support_market.years.last().copied().unwrap_or_default();
    support_bosses(&mut w);
    let after = *w.support_market.years.last().unwrap();
    let mut brought = 0;
    for (from, to, hc, his) in moves {
        let now = &w.programs[to].support.strength;
        assert!(now.rating > 0, "the post is filled the same day");
        assert_eq!(now.coach, Some(hc), "whoever has it is the new head coach's man");
        if now.name == his.name && now.born == his.born {
            brought += 1;
            assert!(now.pay >= his.pay, "he follows for at least what he had");
            assert!(w.programs[from].support.strength.rating > 0, "his old post is filled again");
            assert_ne!(w.programs[from].support.strength.name, his.name);
        }
    }
    assert!(brought >= 8, "most new head coaches bring their man: {brought} of 12");
    assert_eq!((after.brought - before.brought) as usize, brought);
    // A program with no head coach yet (an interim) changes nothing.
    let quiet = w.support_market.years.last().copied().unwrap();
    support_bosses(&mut w);
    assert_eq!(w.support_market.years.last().unwrap().brought, quiet.brought);
}
