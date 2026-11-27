//! How a player sees his own game (talk replies audit, 2026-11-27): Cain
//! told a tackle with Communication 8 his communication wasn't up to par
//! and the voice had him answer "I've been calling the protections since I
//! got here, that's news to me". The card now carries his own read of his
//! game, so a hole he can't miss is named as one and a strength as one.

use super::legacy::seat;
use crate::attributes::Attr;
use crate::positions::Position;
use crate::world::folk::persona::Them;
use crate::world::folk::persona::talk::own_game;
use crate::world::{Stage, World};

/// A college world with one tackle on your roster, his communication and
/// his hands set.
fn tackle(communication: f64, strength: f64) -> (World, usize, u32) {
    let mut w = seat("college");
    w.add_class_aged(19);
    let p = w.seat.and_then(|s| s.program).unwrap();
    let year = w.year;
    let i = (0..w.people.len()).find(|&i| w.people[i].stage == Stage::HighSchool).unwrap();
    let q = &mut w.people[i];
    q.stage = Stage::College;
    q.college = Some(p);
    q.class_of = year - 1;
    q.college_seasons = 2;
    q.player.position = Position::OT;
    q.player.attributes.set(Attr::Communication, communication);
    q.player.attributes.set(Attr::Strength, strength);
    // A sharp, even-egoed man reads himself closely: the read is within 4
    // points of the gap, whatever the draw.
    q.player.attributes.set(Attr::FootballIq, 90.0);
    q.player.personality.traits.set(crate::player::Trait::Ego, 10.0);
    let id = q.player.id;
    w.programs[p].roster.push(i);
    (w, i, id)
}

#[test]
fn a_tackle_with_communication_8_knows_it_is_a_hole() {
    let (w, i, id) = tackle(8.0, 80.0);
    let words = own_game::words(&w, Them::Player(id)).join(" | ");
    let weak = words.split(" | ").find(|l| l.starts_with("where you know you're weaker")).unwrap_or("");
    assert!(weak.contains("communication"), "{words}");
    // A gap past BIG (as he reads it) is a hole he has heard about before.
    let seen = own_game::view(&w, i).into_iter().find(|(a, _)| *a == Attr::Communication).unwrap().1;
    assert_eq!(weak.contains("a hole you know about"), seen <= -own_game::BIG, "{words} ({seen})");
    // His strength is a strength, never a weak spot.
    assert!(!weak.contains("strength"), "{words}");
}

#[test]
fn a_man_who_calls_the_protections_does_not_list_communication_as_weak() {
    let (w, _, id) = tackle(90.0, 40.0);
    let words = own_game::words(&w, Them::Player(id)).join(" | ");
    let weak = words.split(" | ").find(|l| l.starts_with("where you know you're weaker")).unwrap_or("");
    assert!(!weak.contains("communication"), "{words}");
    let good = words.split(" | ").find(|l| l.starts_with("what you know you do well")).unwrap_or("");
    assert!(good.contains("communication"), "{words}");
}

#[test]
fn the_read_is_the_same_every_talk_and_has_no_numbers() {
    let (w, _, id) = tackle(8.0, 50.0);
    let a = own_game::words(&w, Them::Player(id));
    assert_eq!(a, own_game::words(&w, Them::Player(id)));
    assert!(a.iter().all(|l| !l.chars().any(|c| c.is_ascii_digit())), "{a:?}");
    // A coach or someone who isn't a player has none.
    assert!(own_game::words(&w, Them::Coach(0)).is_empty());
}

#[test]
fn a_proud_blurry_man_reads_himself_off_but_never_by_more_than_the_error_allows() {
    let (mut w, i, _) = tackle(8.0, 60.0);
    let sharp = own_game::view(&w, i).into_iter().find(|(a, _)| *a == Attr::Communication).unwrap().1;
    w.people[i].player.attributes.set(Attr::FootballIq, 10.0);
    w.people[i].player.personality.traits.set(crate::player::Trait::Ego, 20.0);
    let blurry = own_game::view(&w, i).into_iter().find(|(a, _)| *a == Attr::Communication).unwrap().1;
    let most = 2.0 * own_game::BLUR_MAX + own_game::EGO_BIAS + 2.0 * own_game::BLUR_MIN;
    assert!((blurry - sharp).abs() <= most + 1e-9, "sharp {sharp}, blurry {blurry}");
    // The same man reads himself the same way again.
    let again = own_game::view(&w, i).into_iter().find(|(a, _)| *a == Attr::Communication).unwrap().1;
    assert_eq!(blurry, again);
}
