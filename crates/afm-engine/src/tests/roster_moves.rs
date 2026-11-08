//! Spring and summer roster moves (lane B of the Jan 1 start): your
//! walk-ons go on scholarship by your word, your open tryout brings students
//! out and you keep the ones you want, an unsigned senior who is still
//! looking stays recruitable after signing day, and the portal's entrants
//! follow their places when the people list is compacted.

use super::legacy::seat;
use crate::player::PlayingTime;
use crate::world::front_office::{college as desk, walk_ons};
use crate::world::{Stage, World};

/// Your program and `n` high school kids of the world, by index.
fn kids(w: &World, n: usize) -> Vec<usize> {
    (0..w.people.len()).filter(|&i| w.people[i].stage == Stage::HighSchool).take(n).collect()
}

fn your_program(w: &World) -> usize {
    w.seat.and_then(|s| s.program).unwrap()
}

#[test]
fn a_walk_on_goes_on_scholarship_by_your_word() {
    let mut w = seat("college");
    let p = your_program(&w);
    w.add_class_aged(19);
    let i = kids(&w, 1)[0];
    let year = w.year;
    {
        let q = &mut w.people[i];
        q.stage = Stage::College;
        q.college = Some(p);
        q.class_of = year - 1;
        q.walk_on = true;
        q.last_role = Some(PlayingTime::Starter);
    }
    w.programs[p].roster.push(i);
    let id = w.people[i].player.id;
    let words = walk_ons::scholarship_words(&w, id).expect("a walk-on on your roster can go on aid");
    assert!(!words.is_empty());
    let before = w.log.walk_on_scholarships;
    walk_ons::award(&mut w, id).unwrap();
    assert!(!w.people[i].walk_on);
    assert_eq!(w.log.walk_on_scholarships, before + 1);
    assert!(walk_ons::award(&mut w, id).is_err(), "only once");
    // Not one of yours: no call.
    let other = kids(&w, 1)[0];
    assert!(walk_ons::scholarship_words(&w, w.people[other].player.id).is_err());
}

#[test]
fn your_tryout_brings_students_out_once_a_season() {
    let mut w = seat("college");
    let p = your_program(&w);
    // Outside the window: no tryout.
    let (from, to) = walk_ons::window(&w);
    if w.today < from || w.today > to {
        assert!(walk_ons::tryout_rule(&w).is_err());
    }
    // New graduates nobody signed, at home in your state.
    w.add_class_aged(18);
    let season = w.dates.season;
    let state = w.programs[p].state;
    let hs = w.high_schools.iter().position(|h| h.state == state);
    for i in kids(&w, 80) {
        let q = &mut w.people[i];
        q.stage = Stage::Done;
        q.class_of = season;
        q.path.clear();
        q.college = None;
        q.home_state = state;
        if let Some(h) = hs {
            q.hs = h;
        }
    }
    w.today = from.add(3);
    walk_ons::tryout_rule(&w).expect("open in the window");
    walk_ons::hold_tryout(&mut w).unwrap();
    let t = w.front_office.college.tryout.clone().unwrap();
    assert!(!t.came.is_empty() && t.came.len() <= walk_ons::TRYOUT_LOOKS, "{} came", t.came.len());
    for r in &t.came {
        assert!(r.low < r.high, "a range");
        assert!((3.8..7.5).contains(&r.forty), "a forty of {}", r.forty);
    }
    assert!(walk_ons::hold_tryout(&mut w).is_err(), "once a season");
    // Keep the first: he joins your roster as a walk-on.
    let id = t.came[0].id;
    let before = w.log.tryouts;
    walk_ons::keep(&mut w, id).unwrap();
    let i = w.people.iter().position(|q| q.player.id == id).unwrap();
    assert_eq!(w.people[i].stage, Stage::College);
    assert_eq!(w.people[i].college, Some(p));
    assert!(w.people[i].walk_on);
    assert!(w.programs[p].roster.contains(&i));
    assert_eq!(w.log.tryouts, before + 1);
    assert!(walk_ons::keep(&mut w, id).is_err(), "kept already");
}

#[test]
fn an_unsigned_senior_still_looking_stays_recruitable() {
    let mut w = seat("college");
    w.add_class_aged(18);
    let i = kids(&w, 1)[0];
    let sign_year = crate::world::recruiting::next_class(&w);
    {
        let q = &mut w.people[i];
        q.class_of = sign_year - 1;
        q.cal.signed = None;
    }
    assert!(desk::recruitable(&w, i).is_err(), "his class has signed");
    w.people[i].cal.late = true;
    assert!(desk::late_recruit(&w.people[i]));
    assert!(desk::recruitable(&w, i).is_ok(), "a late recruit");
}

#[test]
fn portal_entrants_follow_their_places_through_the_compact() {
    let mut w = seat("college");
    w.portal_pending = vec![2, 5, 9];
    // 2 is dropped, 5 moves to 4, 9 to 7.
    let mut map: Vec<Option<usize>> = (0..12).map(Some).collect();
    map[2] = None;
    map[5] = Some(4);
    map[9] = Some(7);
    crate::world::recruiting::remap_people(&mut w, &map);
    assert_eq!(w.portal_pending, vec![4, 7]);
}

#[test]
fn a_walk_on_role_counts_only_from_a_season_here() {
    let mut w = seat("college");
    let p = your_program(&w);
    w.add_class_aged(19);
    let i = kids(&w, 1)[0];
    let year = w.year;
    {
        let q = &mut w.people[i];
        q.stage = Stage::College;
        q.college = Some(p);
        q.walk_on = true;
        // A high school starter who walked on in June.
        q.last_role = Some(PlayingTime::Starter);
        q.college_staff.clear();
    }
    w.programs[p].roster.push(i);
    assert!(!walk_ons::in_two_deep(&w, i, p), "his high school role doesn't count");
    let id = w.programs[p].id;
    w.people[i].college_staff.push((year - 1, id, None, None));
    assert!(walk_ons::in_two_deep(&w, i, p), "a season here does");
}
