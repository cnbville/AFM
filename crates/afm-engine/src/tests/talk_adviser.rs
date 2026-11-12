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
