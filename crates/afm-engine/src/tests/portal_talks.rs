//! Your players talk to you before they enter the portal (thread "Fix mass
//! transfers after coaching change", 2026-10-28): a window doesn't take them
//! on its first day; each asks to see you, and on his day he enters only if
//! his lean is still above the bar his draw set.

use super::legacy::seat;
use crate::player::PlayingTime;
use crate::world::clock::PortalWindow;
use crate::world::recruiting;
use crate::world::{Stage, World};

/// Your program with `n` sophomore backups who all want out.
fn leaning_room(n: usize) -> (World, usize, Vec<usize>) {
    let mut w = seat("college");
    let p = w.seat.and_then(|s| s.program).unwrap();
    w.add_class_aged(19);
    let year = w.year;
    let room: Vec<usize> = (0..w.people.len()).filter(|&i| w.people[i].stage == Stage::HighSchool).take(n).collect();
    for &i in &room {
        let q = &mut w.people[i];
        q.stage = Stage::College;
        q.college = Some(p);
        q.class_of = year - 1;
        q.college_seasons = 1;
        q.last_role = Some(PlayingTime::Backup);
        q.cal.lean = 0.9;
        w.programs[p].roster.push(i);
    }
    (w, p, room)
}

#[test]
fn your_players_ask_to_see_you_and_the_talk_decides_who_goes() {
    let (mut w, p, room) = leaning_room(6);
    let close = w.today.add(15);
    recruiting::portal_open(&mut w, PortalWindow::Winter, close);
    // Nobody is in yet: each asked to see you.
    for &i in &room {
        assert!(w.people[i].cal.portal.is_none(), "{} entered before his talk", w.people[i].name);
        let id = w.people[i].player.id as u64;
        let talk = w.folk.scenes.iter().find(|s| s.a == id && !s.done).expect("a talk on your calendar");
        assert!(talk.title.ends_with("wants to talk about his future"), "{}", talk.title);
    }
    assert_eq!(w.portal_intents.len(), room.len());
    // Three of them heard what they needed; three didn't.
    for &i in &room[..3] {
        w.people[i].cal.lean = 0.0;
    }
    w.today = close.add(1);
    recruiting::intents_day(&mut w);
    assert!(w.portal_intents.is_empty());
    for &i in &room[..3] {
        let q = &w.people[i];
        assert!(q.cal.portal.is_none() && q.college == Some(p), "{} left after a good talk", q.name);
        assert!(q.cal.lean_events.iter().any(|e| e.1 == "decided to stay after talking it over"));
    }
    for &i in &room[3..] {
        assert!(w.people[i].cal.portal.is_some(), "{} still wanted out and stayed", w.people[i].name);
    }
    // Once they decide where to go, their reasons to leave are spent.
    recruiting::portal_day(&mut w, true);
    for &i in &room[3..] {
        assert!(w.people[i].cal.portal.is_none());
        assert_eq!(w.people[i].cal.lean, 0.0, "{} could be drawn again", w.people[i].name);
    }
}

#[test]
fn a_talk_you_hand_to_staff_is_his_position_coachs_word() {
    let (mut w, _, room) = leaning_room(1);
    let close = w.today.add(15);
    recruiting::portal_open(&mut w, PortalWindow::Winter, close);
    let i = room[0];
    let id = w.people[i].player.id as u64;
    let talk = w.folk.scenes.iter().find(|s| s.a == id && !s.done).map(|s| s.id).unwrap();
    crate::world::folk::scene::hand(&mut w, talk).unwrap();
    assert!(w.people[i].cal.lean_events.iter().any(|e| e.1 == "his position coach asked him to stay"));
}
