//! Your athletic director's yes to a bigger recruiting budget (people topic
//! 12; `folk::staffing::recruit_bonus`, missed feature 5): it is your
//! program's alone, grows the athletic budget's recruiting line, hosts your
//! official visits better and buys your staff more flights on the road
//! (Ground lane G3, GR49). Nothing moves without a
//! raise, and a new job starts without one.

use super::legacy::seat;
use crate::world::finance::college::{RECRUITING, raise_recruiting};
use crate::world::folk::staffing::recruit_bonus;
use crate::world::{Stage, Visit, World, recruiting};

fn college() -> (World, usize) {
    let w = seat("college");
    let mine = w.seat.and_then(|s| s.program).unwrap();
    (w, mine)
}

#[test]
fn the_raise_is_your_programs_alone_and_a_new_job_starts_without_it() {
    let (mut w, mine) = college();
    assert_eq!(w.folk.staff.recruit_bonus, 0.0, "a new job starts with no raise");
    w.folk.staff.recruit_bonus = 0.2;
    assert!((recruit_bonus(&w, mine) - 0.2).abs() < 1e-6);
    let other = (0..w.programs.len()).find(|&p| p != mine).unwrap();
    assert_eq!(recruit_bonus(&w, other), 0.0, "another program gets none of it");
}

#[test]
fn a_yes_grows_this_years_recruiting_line_by_a_tenth_each_time() {
    let (mut w, mine) = college();
    if w.finance.college.get(mine).is_none() {
        return;
    }
    let usual = w.finance.college[mine].budget.l[RECRUITING] as f64;
    assert!(usual > 0.0, "an FBS program has a recruiting line");
    for k in 1..=2 {
        w.folk.staff.recruit_bonus += 0.1;
        raise_recruiting(&mut w, mine, 0.1);
        let now = w.finance.college[mine].budget.l[RECRUITING] as f64;
        let want = usual * (1.0 + 0.1 * k as f64);
        assert!((now - want).abs() < 1.0, "after {k} yes: {now} against {want}");
    }
}

#[test]
fn a_raise_hosts_your_official_visits_better_and_no_one_elses() {
    let (mut w, mine) = college();
    w.add_class_aged(17);
    let i = (0..w.people.len()).find(|&i| w.people[i].stage == Stage::HighSchool).unwrap();
    let other = (0..w.programs.len()).find(|&p| p != mine).unwrap();
    let today = w.today;
    for j in [mine, other] {
        w.people[i].cal.visits.push((j, today, Visit::Official));
    }
    let before = (recruiting::appeal(&w, i, mine, Some(0.0)), recruiting::appeal(&w, i, other, Some(0.0)));
    w.folk.staff.recruit_bonus = 0.3;
    let after = (recruiting::appeal(&w, i, mine, Some(0.0)), recruiting::appeal(&w, i, other, Some(0.0)));
    assert!((after.0 - before.0 - 0.02 * 0.3).abs() < 1e-9, "your weekend: {} to {}", before.0, after.0);
    assert_eq!(after.1, before.1, "another school's weekend is its own");
    // Without an official visit the raise adds nothing.
    w.people[i].cal.visits.retain(|v| v.0 != mine);
    let none = recruiting::appeal(&w, i, mine, Some(0.0));
    w.folk.staff.recruit_bonus = 0.0;
    assert_eq!(recruiting::appeal(&w, i, mine, Some(0.0)), none);
}

#[test]
fn travel_money_buys_your_staff_more_flights() {
    // Ground lane G3 (GR49): the raise grows the recruiting line, and the
    // week's road money with it, so more of your men can fly.
    let (mut w, mine) = college();
    if w.finance.college.get(mine).is_none() {
        return;
    }
    let before = crate::world::ground::week::road::purse(&w, mine, &[]);
    assert!(!before.raised, "no raise yet");
    w.folk.staff.recruit_bonus = 1.0;
    raise_recruiting(&mut w, mine, 1.0);
    let after = crate::world::ground::week::road::purse(&w, mine, &[]);
    assert!(after.raised, "the raise is in the week's money");
    assert!(after.flights > before.flights, "flights {} to {}", before.flights, after.flights);
}

#[test]
fn each_raise_he_already_gave_halves_his_next_yes_and_a_new_job_starts_fresh() {
    use crate::world::folk::boss::{AskKind, given_cut};
    let (mut w, _) = college();
    assert_eq!(given_cut(&w, AskKind::RecruitingMoney), 1.0, "no raise yet, his full chance");
    w.folk.staff.recruit_bonus = 0.1;
    assert!((given_cut(&w, AskKind::RecruitingMoney) - 0.5).abs() < 1e-9, "one raise given: half");
    w.folk.staff.recruit_bonus = 0.3;
    assert!((given_cut(&w, AskKind::RecruitingMoney) - 0.125).abs() < 1e-9, "three given: an eighth");
    assert_eq!(given_cut(&w, AskKind::StaffMoney), 1.0, "the staff pool counts its own raises");
    w.folk.staff.pool_bonus = 0.2;
    assert!((given_cut(&w, AskKind::StaffMoney) - 0.25).abs() < 1e-9, "two staff raises: a quarter");
    assert_eq!(given_cut(&w, AskKind::Facility), 1.0, "other asks are untouched");
}

// --- Your boss's yes to more money for the staff pool (people topic 12) ---

#[test]
fn a_staff_pool_yes_grows_the_support_staff_line_and_the_level_and_no_other_programs() {
    use crate::world::finance::college::{STAFF, open_year, raise_staff};
    use crate::world::folk::Team;
    use crate::world::folk::staffing::staff_bonus;
    let (mut w, mine) = college();
    if w.finance.college.get(mine).is_none() || w.finance.college[mine].budget.year == 0 {
        return;
    }
    let other = (0..w.programs.len()).find(|&p| p != mine && w.finance.college[p].budget.year != 0).unwrap();
    assert_eq!(staff_bonus(&w, Team::College(mine)), 0.0, "a new job starts with no raise");
    let usual = w.finance.college[mine].budget.l[STAFF] as f64;
    let level = w.finance.college[mine].level as f64;
    assert!(usual > 0.0, "an FBS program has a support staff line");
    for k in 1..=2 {
        w.folk.staff.pool_bonus += 0.1;
        raise_staff(&mut w, mine, 0.1);
        let now = w.finance.college[mine].budget.l[STAFF] as f64;
        let want = usual * (1.0 + 0.1 * k as f64);
        assert!((now - want).abs() < 1.0, "after {k} yes: {now} against {want}");
    }
    assert!(w.finance.college[mine].level as f64 >= level, "the level never falls with a raise");
    assert_eq!(staff_bonus(&w, Team::College(other)), 0.0, "another program gets none of it");
    // The next year's budget carries the raise, through the same line as the base pool.
    let y = w.year;
    let raised = w.finance.college[mine].budget.l[STAFF] as f64;
    open_year(&mut w, mine, y);
    assert!((w.finance.college[mine].budget.l[STAFF] as f64 - raised).abs() < 1.0, "open_year reads the raise");
    w.folk.staff.pool_bonus = 0.0;
    open_year(&mut w, mine, y);
    assert!((w.finance.college[mine].budget.l[STAFF] as f64 - usual).abs() < 1.0, "no raise, the usual line");
    // Every AI program's line is the usual one.
    let before = w.finance.college[other].budget.l[STAFF];
    w.folk.staff.pool_bonus = 0.5;
    open_year(&mut w, other, y);
    assert_eq!(w.finance.college[other].budget.l[STAFF], before, "only your seat's program reads the raise");
}
