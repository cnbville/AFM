//! Your seat's flaws and tools (individual practice part 3,
//! `world/craft_work.rs`): points of emphasis, the staff's eye, the tape
//! book and a week's work on a player of yours.

use super::legacy::seat;
use crate::attributes::Attr;
use crate::craft::{Craft, Crafts, Point};
use crate::player::PlayingTime;
use crate::positions::Position;
use crate::world::{Stage, World};
use std::collections::HashMap;

/// Your college program with one sophomore tackle on it who oversets.
fn tackle() -> (World, usize) {
    let mut w = seat("college");
    let p = w.seat.and_then(|s| s.program).unwrap();
    w.add_class_aged(19);
    let year = w.year;
    let i = (0..w.people.len()).find(|&i| w.people[i].stage == Stage::HighSchool).unwrap();
    let q = &mut w.people[i];
    q.stage = Stage::College;
    q.college = Some(p);
    q.class_of = year - 1;
    q.college_seasons = 1;
    q.last_role = Some(PlayingTime::Starter);
    q.trust = 0.6;
    q.player.position = Position::OT;
    q.player.age = 20;
    q.player.personality.coachability = 15.0;
    q.player.craft = Crafts { drawn: true, ..Crafts::default() };
    q.player.craft.set(Craft::Oversets, 0.6);
    w.programs[p].roster.push(i);
    (w, i)
}

#[test]
fn points_keep_their_order_and_the_rating_focus_keeps_a_second_point() {
    let (mut w, i) = tackle();
    let id = w.people[i].player.id;
    assert_eq!(w.set_points(id, &[Point::Craft(Craft::Oversets), Point::Rating(Attr::RunBlocking)]), None);
    assert_eq!(w.points_of(id), vec![Point::Craft(Craft::Oversets), Point::Rating(Attr::RunBlocking)]);
    // His first point grows its rating: the flaw's owner.
    assert_eq!(w.focus_of(id), Some(Craft::Oversets.owner()));
    // The old one-rating focus replaces the first point and keeps the second.
    w.set_focus(id, Some(Attr::Footwork));
    assert_eq!(w.points_of(id), vec![Point::Rating(Attr::Footwork), Point::Rating(Attr::RunBlocking)]);
    let by = w.focus_by_person();
    assert_eq!(by.get(&i), Some(&(Attr::Footwork, Some(Attr::RunBlocking))));
    w.set_points(id, &[]);
    assert!(w.points_of(id).is_empty() && w.focus_of(id).is_none());
}

#[test]
fn a_week_of_work_fixes_his_point_and_the_staff_sees_what_he_carries() {
    let (mut w, i) = tackle();
    let id = w.people[i].player.id;
    w.set_points(id, &[Point::Craft(Craft::Oversets)]);
    let minutes: HashMap<usize, (f64, f64)> = [(i, (30.0, 20.0))].into_iter().collect();
    let roster = vec![i];
    let mut notes = Vec::new();
    let start = w.today;
    for wk in 0..40 {
        w.today = start.add(7 * wk);
        notes.extend(crate::world::craft_work::week(&mut w, &roster, &minutes));
    }
    let it = *w.people[i].player.craft.get(Craft::Oversets).expect("kept for its stress return");
    println!("after 40 weeks: games {:.2}, practice {:.2}, notes {notes:?}", it.sev(), it.prac());
    assert!(it.sev() < 0.3, "the work moved it: {}", it.sev());
    assert!(it.worked(), "the year's step leaves it alone");
    assert!(!notes.is_empty(), "he says so when it lands");
    // Forty weeks of looks name something (the flaw, or a neighbour by a
    // weak eye).
    assert!(!w.seen_of(id).is_empty());
}

#[test]
fn with_no_points_of_yours_the_staff_works_what_it_has_named() {
    let (mut w, i) = tackle();
    let id = w.people[i].player.id;
    assert!(w.worked_on(id).is_empty(), "nothing named, nothing worked");
    w.seat_seen.push(crate::world::craft_work::Seen { id, code: Craft::Oversets as u8, day: w.today });
    assert_eq!(w.worked_on(id), vec![Craft::Oversets]);
    // Yours win: a plain rating point works no flaw.
    w.set_points(id, &[Point::Rating(Attr::PassBlocking)]);
    assert!(w.worked_on(id).is_empty());
}

#[test]
fn the_tape_book_keeps_your_players_reps() {
    let (mut w, i) = tackle();
    let id = w.people[i].player.id;
    let teams = crate::game::synthetic::synthetic_league(crate::player::Level::FBS, 2, 3);
    let res = crate::game::quick_game(&teams[0], &teams[1], crate::player::Level::FBS, 5);
    let mut r = res.stats.plays.iter().find(|p| !p.on_field[0].is_empty()).unwrap().clone();
    r.on_field[if r.offense == 0 { 0 } else { 1 }][0] = id;
    r.craft = vec![(id, Craft::Oversets as u8, -1), (9_999_999, Craft::Spin as u8, 1)];
    crate::world::craft_work::book_plays(&mut w, &[r.clone(), r], false);
    let rows: Vec<_> = w.tags_of(id).collect();
    assert_eq!(rows.len(), 1, "one row a game");
    assert_eq!((rows[0].shows, rows[0].lost, rows[0].practice), (2, 2, false));
    assert_eq!(w.seat_book.len(), 1, "only your players");
}
