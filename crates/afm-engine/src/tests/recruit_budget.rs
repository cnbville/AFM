//! Your athletic director's yes to a bigger recruiting budget (people topic
//! 12; `folk::staffing::recruit_bonus`, missed feature 5): it is your
//! program's alone, grows the athletic budget's recruiting line, hosts your
//! official visits better and puts a coach out on a Thursday on to a second
//! school, without spending an NCAA evaluation day. Nothing moves without a
//! raise, and a new job starts without one.

use super::legacy::seat;
use crate::calendar::{Date, Weekday};
use crate::world::finance::college::{RECRUITING, raise_recruiting};
use crate::world::folk::staffing::recruit_bonus;
use crate::world::scouting::dept::second_schools;
use crate::world::scouting::readers::COACH;
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
fn travel_money_sends_a_coach_on_to_a_second_school_on_thursdays_only() {
    let (mut w, mine) = college();
    w.add_class_aged(17);
    // Ten kids from one state on the board; two coaches out on the first two.
    let st = w.people.iter().find(|p| p.stage == Stage::HighSchool).unwrap().home_state;
    let board: Vec<usize> = (0..w.people.len())
        .filter(|&i| w.people[i].stage == Stage::HighSchool && w.people[i].home_state == st)
        .take(10)
        .collect();
    assert!(board.len() >= 4, "enough kids in one state");
    let picks = vec![(COACH + 1, board[0]), (COACH + 2, board[1])];
    let unseen = |_: &World, _: usize| false;
    let mut thu = Date::ymd(w.year, 9, 1);
    while thu.weekday() != Weekday::Thu {
        thu = thu.add(1);
    }
    assert!(second_schools(&w, mine, thu, &board, &picks, &unseen).is_empty(), "no raise, no second school");
    // A raise of the whole budget: every coach out goes on.
    w.folk.staff.recruit_bonus = 1.0;
    let extra = second_schools(&w, mine, thu, &board, &picks, &unseen);
    assert_eq!(extra, vec![(COACH + 1, board[2]), (COACH + 2, board[3])], "the next best unseen near the first");
    // Friday's games kick off at once: no second school.
    assert!(second_schools(&w, mine, thu.add(1), &board, &picks, &unseen).is_empty());
    // Someone already seen is skipped.
    let seen = |_: &World, k: usize| k == board[2];
    let extra = second_schools(&w, mine, thu, &board, &picks, &seen);
    assert_eq!(extra, vec![(COACH + 1, board[3]), (COACH + 2, board[4])]);
    // At a tenth more money about a tenth of coach days go on.
    w.folk.staff.recruit_bonus = 0.1;
    let mut days = 0;
    let mut on = 0;
    let mut d = thu;
    for _ in 0..400 {
        days += picks.len();
        on += second_schools(&w, mine, d, &board, &picks, &unseen).len();
        d = d.add(7);
    }
    let share = on as f64 / days as f64;
    assert!((0.07..0.13).contains(&share), "share of coach days with a second school: {share}");
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
