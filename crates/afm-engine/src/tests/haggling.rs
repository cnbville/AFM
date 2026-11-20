//! Real haggling (talk replies item 15, agreed by Cain 2026-11-19): a coach
//! you're hiring opens at his ask and gives part of the gap each round,
//! less each time, never under his walk-away, and an offer at his number
//! is a yes.

use super::legacy::seat;
use crate::world::folk::hiring::{self, Terms};
use crate::world::folk::{Team, staffing};

#[test]
fn a_coach_you_hire_gives_less_each_round_down_to_his_walk_away() {
    let mut w = seat("college");
    if let Some(Team::College(p)) = Team::seat(&w) {
        w.programs[p].oc = None;
    }
    let role = *staffing::openings(&w).first().expect("an opening");
    let team = Team::seat(&w).unwrap();
    let mut seen = 0;
    for c in 0..w.coaches.len() {
        if w.coaches[c].retired || w.coaches[c].job.is_none_or(|j| Team::from_employer(j.employer) == team) {
            continue;
        }
        let v = &mut w;
        if staffing::start_interview(v, c, role).is_err() {
            continue;
        }
        let t = |salary: f64| Terms { salary, years: 3, promises: 0, clauses: Vec::new() };
        let Some(h) = hiring::haggle(v, c, role, &t(0.0)) else { continue };
        if h.now < h.walk * 1.15 {
            continue;
        }
        // Round by round at his walk-away: his number comes down, by less
        // each time, and never under it.
        let mut nums = vec![h.now];
        for _ in 0..4 {
            let Ok(text) = hiring::offer(v, c, role, t(h.walk * 1.002)) else { break };
            if !text.contains("wants more") {
                break;
            }
            let n = hiring::talk(v, c).and_then(|k| k.haggle).map(|x| x.0).unwrap();
            assert!(n >= h.walk - 1.0 && n <= *nums.last().unwrap(), "{nums:?} then {n}");
            nums.push(n);
        }
        if !staffing::openings(v).contains(&role) {
            break;
        }
        if nums.len() < 3 {
            continue;
        }
        let gaps: Vec<f64> = nums.windows(2).map(|x| x[0] - x[1]).collect();
        assert!(gaps[1] <= gaps[0] + 5_000.0, "he gives less each round: {nums:?}");
        // His own number is a yes.
        let last = *nums.last().unwrap();
        let text = hiring::offer(v, c, role, t(last)).unwrap();
        assert!(!text.contains("wants more"), "{text}");
        seen += 1;
        break;
    }
    assert!(seen > 0, "no coach haggled");
}

#[test]
fn a_coach_in_a_talk_comes_down_round_by_round() {
    use crate::world::folk::persona::terms::{DealKind, Give, Offer, Weighed};
    use crate::world::folk::persona::{Them, deals, office};
    let mut w = seat("college");
    if let Some(Team::College(p)) = Team::seat(&w) {
        w.programs[p].oc = None;
    }
    let role = *staffing::openings(&w).first().expect("an opening");
    let team = Team::seat(&w).unwrap();
    let c = (0..w.coaches.len())
        .find(|&c| !w.coaches[c].retired && w.coaches[c].job.is_some_and(|j| Team::from_employer(j.employer) != team))
        .unwrap();
    staffing::start_interview(&mut w, c, role).unwrap();
    let ask = hiring::ask(&w, c, role);
    let him = Them::Coach(c);
    let money = |x: f64| Offer { kind: DealKind::Staff, gives: vec![Give::Money(x)], ..Default::default() };
    let number = |v: &Weighed| match v {
        Weighed::Counter(k) => k.gives.iter().find_map(|g| if let Give::Money(x) = g { Some(*x) } else { None }),
        _ => None,
    };
    let mut seen = Vec::new();
    for share in [0.75, 0.85, 0.88, 0.9] {
        let o = money((ask * share / 10_000.0).round() * 10_000.0);
        let v = office::deals::weigh(&w, him, &o).unwrap();
        deals::offered(&mut w, him, &o, &v);
        println!("ask {ask:.0}: offer {share} -> {v:?}; talk {:?}", hiring::talk(&w, c).map(|k| (k.haggle, k.gave)));
        match number(&v) {
            Some(n) => seen.push(n),
            None => break,
        }
    }
    assert!(seen.len() >= 2, "{seen:?}");
    assert!(seen.windows(2).all(|x| x[1] <= x[0]), "his number never goes up: {seen:?}");
    assert!(seen.windows(2).any(|x| x[1] < x[0]), "it comes down: {seen:?}");
}
