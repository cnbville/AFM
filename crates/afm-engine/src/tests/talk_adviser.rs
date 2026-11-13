//! Who tells you how a talk will go (Cain 2026-11-12: "when i talk to a
//! player there is a position coach that tells me if the talk is gonna go
//! well, but that position is always out of a job and not the current
//! coach"). The adviser was read from the player's `college_staff` record
//! of past seasons, so he was last season's coach, who may have left since.
//! He is now the man at the position coach's seat today; with the seat
//! empty it is the coordinator on his side, then the head coach, and never
//! you; with none, "your staff".

use super::legacy::seat;
use crate::player::PlayingTime;
use crate::world::coach::{Group, Role};
use crate::world::folk::scene::{self, Kind, Who};
use crate::world::folk::talks::{self, Subject};
use crate::world::folk::{coach_link, facts};
use crate::world::{Stage, World};

/// Your program with one sophomore on it, a coach who coached him last
/// season and has no job now, and a coach for the seat today.
fn room() -> (World, usize, usize, usize, usize) {
    let mut w = seat("college");
    let p = w.seat.and_then(|s| s.program).unwrap();
    w.add_class_aged(19);
    let year = w.year;
    let i = (0..w.people.len()).find(|&i| w.people[i].stage == Stage::HighSchool).unwrap();
    {
        let q = &mut w.people[i];
        q.stage = Stage::College;
        q.college = Some(p);
        q.class_of = year - 1;
        q.college_seasons = 1;
        q.last_role = Some(PlayingTime::Backup);
    }
    w.programs[p].roster.push(i);
    let group = Group::of(w.people[i].player.position);
    let you = w.career.as_ref().unwrap().coach;
    let mut others = (0..w.coaches.len()).filter(|&c| c != you);
    let (old, now) = (others.next().unwrap(), others.next().unwrap());
    // Last season's coach, out of coaching since.
    w.coaches[old].job = None;
    w.people[i].college_staff.clear();
    w.people[i].college_staff.push((year - 1, p, w.programs[p].head_coach, Some(old)));
    // Today's: at the seat.
    let mut job = w.coaches[you].job.unwrap();
    job.role = Role::PositionCoach(group);
    w.coaches[now].job = Some(job);
    w.programs[p].position_coaches[group as usize] = Some(now);
    (w, p, i, old, now)
}

/// What the talk page shows for a move: (what they expect, how sure, who).
fn forecast(w: &mut World, i: usize) -> (String, String, String) {
    let id = w.people[i].player.id;
    let sid = talks::start_talk(w, id, Subject::Role).unwrap();
    let s = w.folk.scenes.iter().find(|s| s.id == sid).unwrap();
    talks::expect(w, s, 110).expect("a one-on-one has a forecast")
}

#[test]
fn the_man_who_reads_a_talk_holds_the_seat_today() {
    let (mut w, _, i, old, now) = room();
    let (_, _, who) = forecast(&mut w, i);
    assert!(who.contains(&coach_link(&w, now)), "the seat's coach reads him: {who}");
    assert!(!who.contains(&coach_link(&w, old)), "last season's coach is gone: {who}");
    assert!(!who.contains("out of coaching"), "{who}");
    assert!(who.contains(&facts::coach_role(&w, now)), "{who}");
}

#[test]
fn an_empty_seat_goes_to_his_coordinator_then_the_head_coach_never_you() {
    let (mut w, p, i, _, now) = room();
    let group = Group::of(w.people[i].player.position);
    let you = w.career.as_ref().unwrap().coach;
    w.programs[p].position_coaches[group as usize] = None;
    // The coordinator on his side.
    if group.offense() {
        w.programs[p].oc = Some(now);
    } else {
        w.programs[p].dc = Some(now);
    }
    assert_eq!(talks::position_coach_now(&w, i), Some(now), "the coordinator covers the room");
    // No coordinator: the head coach reads him, unless that is you.
    if group.offense() {
        w.programs[p].oc = None;
    } else {
        w.programs[p].dc = None;
    }
    w.programs[p].head_coach = Some(now);
    assert_eq!(talks::position_coach_now(&w, i), Some(now), "the head coach covers it");
    w.programs[p].head_coach = Some(you);
    assert_eq!(talks::position_coach_now(&w, i), None, "you don't advise yourself");
    let (_, _, who) = forecast(&mut w, i);
    assert_eq!(who, "Your staff");
    // You in the seat yourself: still nobody to name.
    w.programs[p].position_coaches[group as usize] = Some(you);
    assert_eq!(talks::position_coach_now(&w, i), None);
}

#[test]
fn the_hallway_word_comes_from_the_seat_too() {
    let (mut w, _, i, old, now) = room();
    let id = w.people[i].player.id;
    let mut sc = scene::Scene::new(Kind::StaffLine, w.today, "He is slipping");
    sc.a = id as u64;
    let sid = scene::add(&mut w, sc);
    let s = scene::open(&mut w, sid).unwrap();
    assert!(s.who.contains(&Who::Coach(now)), "{:?}", s.who);
    assert!(!s.who.contains(&Who::Coach(old)), "{:?}", s.who);
    let said = s.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>().join(" ");
    assert!(said.contains(&coach_link(&w, now)) && !said.contains(&coach_link(&w, old)), "{said}");
    assert!(s.lines.iter().any(|l| l.name == "Position coach"), "the seat's own coach speaks as the position coach");
}

// The staff effects themselves (minor bug 377's follow-up, Cain 2026-11-13):
// a talk you hand him, your weekly line, "stay on him" and a word to stay
// are sized by the relationships rating of the man at the seat today, not
// last season's coach.

/// The room with last season's coach a poor people person and today's a
/// fine one.
fn rated_room(old_rel: f64, now_rel: f64) -> (World, usize, usize, usize, usize) {
    let (mut w, p, i, old, now) = room();
    w.coaches[old].ability.relationships = old_rel;
    w.coaches[now].ability.relationships = now_rel;
    (w, p, i, old, now)
}

#[test]
fn a_talk_you_hand_over_is_had_by_the_seats_coach() {
    let (mut w, _, i, _, _) = rated_room(1.0, 19.0);
    let id = w.people[i].player.id;
    let sid = talks::start_talk(&mut w, id, Subject::Role).unwrap();
    scene::open(&mut w, sid).unwrap();
    let (trust, mood) = (w.people[i].trust, w.people[i].mood);
    let s = scene::hand(&mut w, sid).unwrap();
    // 0.02 x (19 - 9) / 5 = 0.04: half of it to his trust, all to his mood.
    // Last season's coach (1) would have cost him 0.032.
    assert!((w.people[i].trust - trust - 0.02).abs() < 1e-4, "{} -> {}", trust, w.people[i].trust);
    assert!((w.people[i].mood - mood - 0.04).abs() < 1e-4, "{} -> {}", mood, w.people[i].mood);
    assert_eq!(s.handed.as_deref(), Some("his position coach"));
}

#[test]
fn an_empty_seat_hands_the_talk_to_the_coordinator_by_name() {
    let (mut w, p, i, _, now) = rated_room(1.0, 19.0);
    let group = Group::of(w.people[i].player.position);
    w.programs[p].position_coaches[group as usize] = None;
    let mut job = w.coaches[now].job.unwrap();
    if group.offense() {
        w.programs[p].oc = Some(now);
        job.role = Role::OffensiveCoordinator;
    } else {
        w.programs[p].dc = Some(now);
        job.role = Role::DefensiveCoordinator;
    }
    w.coaches[now].job = Some(job);
    let id = w.people[i].player.id;
    let sid = talks::start_talk(&mut w, id, Subject::Role).unwrap();
    let s = scene::hand(&mut w, sid).unwrap();
    let side = if group.offense() { "offensive" } else { "defensive" };
    assert_eq!(s.handed, Some(format!("his {side} coordinator")));
    assert!(s.outcome.iter().any(|o| o.starts_with(&format!("His {side} coordinator "))), "{:?}", s.outcome);
}

#[test]
fn your_weekly_line_is_carried_by_the_seats_coach() {
    // "Keep him level": his mood eases by 0.15 x (rating - 6) / 10 a week.
    // Today's coach (20) takes 21% off; last season's (4) would take none.
    let (mut w, _, i, _, _) = rated_room(4.0, 20.0);
    let id = w.people[i].player.id;
    talks::set_line(&mut w, id, Some(talks::Line::Level));
    w.people[i].mood = -0.6;
    let mut d = w.today;
    while d.weekday() != crate::calendar::Weekday::Mon {
        d = d.add(1);
    }
    talks::day(&mut w, d);
    let want = -0.6 * (1.0 - 0.15 * 1.4);
    assert!((w.people[i].mood - want).abs() < 1e-4, "{} against {want}", w.people[i].mood);
}

#[test]
fn stay_on_him_is_the_seats_coachs_word() {
    // "Stay on him": 0.03 x (rating - 8) / 8 to his mood. Today's coach (16)
    // lifts him 0.03; last season's (2) would have cost him 0.0225.
    let (mut w, _, i, _, _) = rated_room(2.0, 16.0);
    let id = w.people[i].player.id;
    let mut sc = scene::Scene::new(Kind::StaffLine, w.today, "He is slipping");
    sc.a = id as u64;
    let sid = scene::add(&mut w, sc);
    scene::open(&mut w, sid).unwrap();
    let mood = w.people[i].mood;
    let s = scene::answer(&mut w, sid, 401).unwrap();
    assert!((w.people[i].mood - mood - 0.03).abs() < 1e-4, "{mood} -> {}", w.people[i].mood);
    assert!(s.outcome.iter().any(|o| o == "His position coach stays on him."), "{:?}", s.outcome);
}

#[test]
fn a_word_to_stay_comes_from_the_seats_coach() {
    // His lean x (1 - 0.02 x (rating - 8)), held to 0.85-1.05: today's coach
    // (18) takes it to 0.85 of itself; last season's (3) would have raised it.
    let (mut w, p, i, _, now) = rated_room(3.0, 18.0);
    w.people[i].cal.lean = 0.5;
    crate::world::folk::takeover::position_coach_word(&mut w, i);
    assert!((w.people[i].cal.lean - 0.5 * 0.85).abs() < 1e-9, "{}", w.people[i].cal.lean);
    assert_eq!(w.people[i].cal.lean_events.last().map(|e| e.1), Some("his position coach asked him to stay"));
    // With the seat empty, the coordinator on his side asks him.
    let group = Group::of(w.people[i].player.position);
    w.programs[p].position_coaches[group as usize] = None;
    let mut job = w.coaches[now].job.unwrap();
    if group.offense() {
        w.programs[p].oc = Some(now);
        job.role = Role::OffensiveCoordinator;
    } else {
        w.programs[p].dc = Some(now);
        job.role = Role::DefensiveCoordinator;
    }
    w.coaches[now].job = Some(job);
    crate::world::folk::takeover::position_coach_word(&mut w, i);
    let side = if group.offense() { "offensive" } else { "defensive" };
    assert_eq!(
        w.people[i].cal.lean_events.last().map(|e| e.1.to_string()),
        Some(format!("his {side} coordinator asked him to stay"))
    );
}
