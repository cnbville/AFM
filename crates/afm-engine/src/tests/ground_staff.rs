//! Ground lane G10: your staff's ground and the assistant seat
//! (`world/ground/staff/`): the coordinator's areas by each man's ground,
//! your changes, the ride-along, a man courted and your counter, losing a
//! man (his ties, his commits, your tree), hiring signs, and the desk.

use crate::calendar::{Date, Weekday};
use crate::world::coach::{Employer, Group, Role};
use crate::world::ground::people::{self, Holder, Who};
use crate::world::ground::staff::{self, beats, carousel, desk, men};
use crate::world::ground::{areas, standing, week};
use crate::world::scouting::dept;
use crate::world::seat::{Job, Start};
use crate::world::{Stage, World};

/// Words a screen must never show.
fn whole(s: &str) -> bool {
    !s.is_empty() && !s.contains(['{', '}']) && !s.contains("NaN") && !s.contains("inf ")
}

/// Kids at the schools of the seat's own state: pros made into high school
/// juniors and seniors, so the board and the areas have recruits.
fn kids(w: &mut World, st: usize) {
    let schools: Vec<usize> = (0..w.high_schools.len())
        .filter(|&h| w.high_schools[h].state == st && w.high_schools[h].coach.is_some())
        .collect();
    assert!(!schools.is_empty(), "coached schools in the state");
    let season = w.dates.season;
    for (k, p) in w.people.iter_mut().filter(|p| p.stage == Stage::Pro).take(240).enumerate() {
        p.stage = Stage::HighSchool;
        p.pro = None;
        p.class_of = season + 1 + (k % 2) as i32;
        p.player.age = 17;
        p.hs = schools[k % schools.len()];
        p.home_state = st;
        p.stars = [2, 3, 3, 4][k % 4];
    }
}

/// The tiny college seat (head coach) with recruits in its state.
fn head() -> (World, usize) {
    let mut w = super::legacy::seat("college");
    let prog = w.seat.and_then(|s| s.program).unwrap();
    let st = w.programs[prog].state;
    kids(&mut w, st);
    (w, prog)
}

/// One day of Ground for your seat, the way the clock plays it.
fn run(w: &mut World, from: Date, days: i32) {
    for k in 0..days {
        let d = from.add(k);
        w.today = d;
        w.played = 0;
        week::day(w, d);
        crate::world::ground::day(w, d);
        dept::day(w, d);
    }
}

#[test]
fn the_coordinator_gives_each_man_areas_by_his_own_ground() {
    let (mut w, prog) = head();
    let staff = staff::men(&w, prog);
    assert!(staff.len() >= 3, "a staff: {}", staff.len());
    // One man has his own strong ties at the schools of one area.
    let by_area = areas::schools_by_area(&w);
    let st = w.programs[prog].state;
    let a = *areas::in_state(st).iter().max_by_key(|&&a| by_area[a as usize].len()).unwrap();
    let m = staff[1];
    for &h in by_area[a as usize].iter().take(8) {
        w.coaches[m].ties.insert(h, 1.0);
    }
    let d = w.today;
    run(&mut w, d, 1);
    let s = &w.ground.staff;
    assert_eq!(s.program, Some(prog as u32));
    assert!(!s.beats.is_empty(), "the coordinator's first proposal on the first day");
    assert!(s.beats.iter().any(|b| b.man == m as u32 && b.area == a), "his own ground is his: {:?}", s.beats);
    let mine = s.beats.iter().find(|b| b.man == m as u32 && b.area == a).unwrap();
    assert!(mine.why.contains("Home") || mine.why.contains("Trusted"), "{}", mine.why);
    for &c in &staff {
        let n = s.beats.iter().filter(|b| b.man == c as u32).count();
        assert!(n >= 1 && n <= beats::PER_MAN, "every man works one to three areas: {n}");
    }
    assert!(s.proposal.letter.iter().all(|l| whole(l)), "{:?}", s.proposal.letter);
    // The man on a school's area recruits it.
    let h = by_area[a as usize][0];
    assert_eq!(beats::recruiter_for_school(&w, prog, h), Some(m));
    // His page reads his ground in words.
    let g = men::ground(&w, prog, m).unwrap();
    assert!(g.areas.iter().any(|x| x.area == a && (x.level == "Home" || x.level == "Trusted")), "{:?}", g.areas);
    assert!(g.levels.1 + g.levels.2 >= 1);
    assert!(whole(g.mood.words) && whole(g.style));
}

#[test]
fn you_move_a_man_any_monday_and_the_week_sends_him_there() {
    let (mut w, prog) = head();
    let d = w.today;
    run(&mut w, d, 1);
    let staff = staff::men(&w, prog);
    let m = staff[0];
    let st = w.programs[prog].state;
    let a = areas::in_state(st)[0];
    let said = beats::set_areas(&mut w, m, &[a]).unwrap();
    assert!(said.contains(areas::area(a).nickname), "{said}");
    assert_eq!(beats::his_areas(&w, prog, m), Some(vec![a]));
    assert!(w.ground.staff.beats.iter().any(|b| b.man == m as u32 && b.yours));
    assert!(beats::set_areas(&mut w, m, &[0, 1, 2, 3, 4]).is_err(), "five areas is too many for one man");
    // The department's state command moves the whole state.
    let other = staff[1];
    dept::set_area(&mut w, st as u8, Some(other));
    assert!(areas::in_state(st).iter().all(|&x| beats::on_area(&w, prog, x) == vec![other]));
    assert_eq!(dept::area_coach(&w, prog, st as u8), Some(other));
}

#[test]
fn a_young_coach_riding_along_meets_the_veterans_coaches() {
    let (mut w, prog) = head();
    let (a, _) = week::beat::spring_road(&w.dates).expect("a spring road");
    let mon = a.on_or_after(Weekday::Mon);
    run(&mut w, mon, 1);
    let mut staff = staff::men(&w, prog);
    staff.sort_by_key(|&c| w.coaches[c].age);
    let (young, vet) = (staff[0], *staff.last().unwrap());
    assert!(w.coaches[young].age < w.coaches[vet].age);
    beats::ride(&mut w, young, vet, 2).unwrap();
    assert!(beats::ride(&mut w, vet, young, 2).is_err(), "the younger man rides with the older");
    let st = w.programs[prog].state;
    let area = *areas::in_state(st).first().unwrap();
    week::plan::clear_road(&mut w, young as u32).ok();
    week::plan::set_road(&mut w, vet as u32, area, &[1]).unwrap();
    let before = w.scouting.dept.eval_days.2;
    run(&mut w, mon.add(1), 1);
    assert!(w.scouting.dept.eval_days.2 - before >= 2, "a ride-along costs both men's day");
    assert_eq!(beats::days_out(&w, vet).0, 1);
    let holder = Holder::Coach(young as u32);
    let met = w.ground.memories.iter().filter(|m| m.holder == holder && m.contacts > 0).count();
    assert!(met > 0, "the coaches the veteran saw know the young man on his own");
    assert!(w.ground.week.log.iter().any(|x| x.man == Some(young as u32) && x.text.contains("rode along")));
    assert_eq!(beats::days_out(&w, young).0, 1);
}

#[test]
fn a_man_another_school_wants_waits_for_your_counter() {
    let (mut w, prog) = head();
    let d = w.today;
    run(&mut w, d, 1);
    let m = staff::men(&w, prog)[2];
    let other = (0..w.programs.len()).find(|&p| p != prog && w.programs[p].head_coach.is_some()).unwrap();
    let e = Employer::College(other);
    assert!(carousel::courting(
        &mut w,
        m,
        e,
        Role::OffensiveCoordinator,
        0.6,
        "a step up to offensive coordinator",
        900_000.0
    ));
    assert!(!carousel::courting(&mut w, m, e, Role::OffensiveCoordinator, 0.6, "again", 900_000.0), "once");
    let x = w.ground.staff.courted.last().unwrap().clone();
    assert_eq!(x.state, 0);
    assert!(x.until > d && whole(&x.line));
    let inbox = &w.career.as_ref().unwrap().desk.mails;
    assert!(inbox.iter().any(|m| m.subject.contains("is after")), "the warning");
    for k in [carousel::Counter::Raise, carousel::Counter::Title, carousel::Counter::Promise] {
        let r = carousel::reach(&w, m, k).unwrap();
        assert!((0.05..=0.95).contains(&r));
        assert!(whole(carousel::reach_words(r)) && whole(&carousel::side_effect(&w, m, k)));
    }
    // Asked again before his answer, he says nothing yet.
    assert!(carousel::courted_answer(&mut w, 999_999, e, Role::OffensiveCoordinator).is_none());
    let said = carousel::counter(&mut w, m, carousel::Counter::Title).unwrap();
    assert!(whole(&said));
    assert_eq!(crate::world::folk::hiring::title_of(&w, m), Some(2), "the title is his either way");
    let st = w.ground.staff.courted.last().unwrap().state;
    assert!(st == 1 || st == 2);
    let ans = carousel::courted_answer(&mut w, m, e, Role::OffensiveCoordinator).unwrap();
    assert_eq!(ans.is_some(), st == 2, "the search reads his answer");
    assert!(carousel::counter(&mut w, m, carousel::Counter::Raise).is_err(), "one counter");
}

#[test]
fn when_a_man_goes_his_ties_go_his_commits_waver_and_he_joins_your_tree() {
    let (mut w, prog) = head();
    let d = w.today;
    run(&mut w, d, 1);
    let m = *staff::men(&w, prog).iter().find(|&&c| !w.ground.staff.beats.iter().all(|b| b.man != c as u32)).unwrap();
    let area = w.ground.staff.beats.iter().find(|b| b.man == m as u32).unwrap().area;
    // His own tie to a school there, and a kid of that school committed to you.
    let h =
        (0..w.high_schools.len()).find(|&h| w.high_schools[h].ground_area == area && w.high_schools[h].coach.is_some());
    let Some(h) = h else { return };
    w.coaches[m].ties.insert(h, 1.0);
    let coach = people::head_coach_of(&w, h).unwrap();
    let with_him = standing::staff(&w, prog, coach).0;
    let i = (0..w.people.len()).find(|&i| w.people[i].stage == Stage::HighSchool).unwrap();
    w.people[i].hs = h;
    w.people[i].cal.commit = Some((prog, d, false));
    w.people[i].cal.signed = None;
    let id = w.people[i].player.id;
    assert_eq!(beats::recruiter_for_school(&w, prog, h), Some(m));
    // He leaves for another school's staff.
    let other = (0..w.programs.len()).find(|&p| p != prog).unwrap();
    crate::world::staff::leave(&mut w, m, "left for another job");
    crate::world::staff::hire(&mut w, m, Employer::College(other), Role::PositionCoach(Group::DL));
    run(&mut w, d.add(1), 1);
    let s = &w.ground.staff;
    let left = s.left.last().expect("he's on the list of men who left");
    assert_eq!(left.man, m as u32);
    assert!(!s.beats.iter().any(|b| b.man == m as u32), "his areas wait for the coordinator");
    assert!(standing::staff(&w, prog, coach).0 <= with_him, "his tie went with him");
    assert!(s.wavers.iter().any(|x| x.id == id), "his commit wavers");
    if left.terms <= 1 {
        let you = w.career.as_ref().unwrap().coach as u32;
        let mem = w.ground.get(Who::Coach(m as u32), Holder::Coach(you)).expect("your tree");
        assert!(mem.roots.tree && mem.points >= standing::TREE);
    }
    let said = carousel::call_commit(&mut w, id).unwrap();
    assert!(whole(&said), "{said}");
    assert!(w.ground.staff.wavers.iter().any(|x| x.id == id && x.done.is_some()));
    // A ride-along's young man keeps what he built: the memory is his.
}

#[test]
fn a_candidates_ground_reads_only_in_signs() {
    let (mut w, prog) = head();
    let d = w.today;
    run(&mut w, d, 1);
    let mine = staff::men(&w, prog);
    let c = (0..w.coaches.len())
        .find(|&c| w.coaches[c].job.is_some_and(|j| j.employer != Employer::College(prog)) && !mine.contains(&c))
        .expect("a coach elsewhere");
    let st = w.programs[prog].state;
    let theirs: Vec<usize> = (0..w.high_schools.len()).filter(|&h| w.high_schools[h].state == st).take(3).collect();
    for h in theirs {
        w.coaches[c].ties.insert(h, 1.0);
    }
    let signs = men::signs(&w, prog, c);
    assert!(!signs.is_empty());
    for s in &signs {
        assert!(whole(s), "{s}");
        assert!(!s.contains('.') || !s.chars().any(|ch| ch == '%'), "no numbers of his: {s}");
    }
    assert!(signs[0].starts_with("Where he's coached"), "{signs:?}");
}

#[test]
fn the_assistant_brings_two_kids_a_week_and_the_head_coach_decides() {
    let mut w = super::legacy::seat("college");
    let prog = w.seat.and_then(|s| s.program).unwrap();
    // Start again as his defensive line coach under the AI head coach.
    let mut w2 = crate::world::World::empty(crate::world::WorldOptions {
        scale: 0.001,
        history_years: 0,
        start_year: 2037,
        ..Default::default()
    });
    w2.start_career_as(&super::legacy::you(), Start::Assistant(Job::College(prog), Role::PositionCoach(Group::DL)), 7);
    w = w2;
    let st = w.programs[prog].state;
    kids(&mut w, st);
    assert!(!w.seat_head());
    let d = w.today;
    run(&mut w, d, 1);
    let (p, you, hc) = desk::seat(&w).expect("the assistant seat");
    assert_eq!(p, prog);
    assert_ne!(you, hc);
    assert!(w.ground.staff.left.is_empty(), "you never leave your own staff on day one");
    assert!(!w.ground.staff.roster.contains(&(you as u32)));
    assert!(beats::set_areas(&mut w, you, &[0]).is_err(), "your head coach sets the areas");
    assert!(!w.ground.staff.desk.goals.is_empty(), "his goals for your area and your group");
    assert!(!w.ground.staff.desk.choices.is_empty(), "ground goals to pick from");
    desk::pick_goal(&mut w, 0).unwrap();
    assert!(desk::pick_goal(&mut w, 0).is_err(), "one ground goal a year");
    // Your list: kids from your area or your group.
    let mine = beats::his_areas(&w, prog, you).unwrap_or_default();
    let kidlist: Vec<u32> = w
        .people
        .iter()
        .filter(|q| q.stage == Stage::HighSchool && q.cal.signed.is_none() && q.cal.commit.is_none())
        .filter(|q| {
            w.high_schools.get(q.hs).is_some_and(|s| mine.contains(&s.ground_area))
                || Group::of(q.player.position) == Group::DL
        })
        .filter(|q| !q.cal.offers.contains(&prog) && !q.cal.early_offers.iter().any(|o| o.0 == prog))
        .map(|q| q.player.id)
        .take(3)
        .collect();
    assert!(kidlist.len() >= 3, "kids for your list");
    desk::push(&mut w, kidlist[0], "Long arms, plays hard.").unwrap();
    desk::push(&mut w, kidlist[1], "").unwrap();
    assert!(desk::push(&mut w, kidlist[2], "").is_err(), "two a week");
    let t = w.today;
    run(&mut w, t.add(1), 3);
    let pushes = w.ground.staff.desk.pushes.clone();
    assert!(pushes.iter().all(|p| p.verdict != 0), "his word once the film is in: {pushes:?}");
    for p in &pushes {
        assert!(whole(&p.reason) && p.film_word.as_deref().is_some_and(whole));
        let i = crate::world::scouting::place(&w, p.id).unwrap();
        let offered = w.people[i].cal.offers.contains(&prog);
        assert_eq!(offered, p.verdict == 1, "a signed push is an offer from your program");
    }
    if let Some(p) = pushes.iter().find(|p| p.verdict == 2) {
        let said = desk::push_back(&mut w, p.id).unwrap();
        assert!(whole(&said));
        assert!(desk::push_back(&mut w, p.id).is_err(), "once");
    }
    let v = desk::view(&w).unwrap();
    assert!(!v.memory.is_empty(), "his memory of you");
    assert!(whole(v.trust));
    // Only your own days, on your area.
    let other = staff::men(&w, prog).into_iter().find(|&c| c != you).unwrap();
    assert!(desk::check_road(&w, other as u32, 0, 1).is_err());
}

#[test]
fn no_career_no_staff_ground() {
    let mut w = crate::world::World::empty(crate::world::WorldOptions {
        scale: 0.001,
        history_years: 0,
        start_year: 2037,
        ..Default::default()
    });
    let d = w.today;
    crate::world::ground::day(&mut w, d);
    assert!(w.ground.staff.program.is_none() && w.ground.staff.beats.is_empty());
    assert!(staff::offer_pull(&w, 0).is_none());
    assert_eq!(desk::review_factor(&w, 0), 1.0);
}
