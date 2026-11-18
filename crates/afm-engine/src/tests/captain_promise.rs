//! The captain promise (`world/folk/captain.rs`, 2026-11-18): "you'll be a
//! captain" is a promise judged the day your room names its captains. The
//! vote keeps it, or you name him beside the vote and the room pays for it;
//! with its setting off you let the vote stand and break it; a missed
//! condition releases you; a coach who picks his own pays half; and what he
//! believes reads the vote as he sees it.

use super::legacy::seat;
use crate::player::{PlayingTime, Trait};
use crate::positions::Position;
use crate::world::folk::promise::{Cond, Terms};
use crate::world::folk::talks::{self, PromiseKind, PromiseTo};
use crate::world::folk::{Name, Team, captain, scene, takeover};
use crate::world::{Stage, World};

/// Your college program with a room of `n` linebackers in their second
/// year, all starters: man 0 the room's natural leader down to man n-1,
/// the last. Your coach lets the room vote unless `strict`.
fn room(n: usize, strict: bool) -> (World, usize, Vec<usize>) {
    let mut w = seat("college");
    let p = w.seat.and_then(|s| s.program).unwrap();
    w.add_class_aged(19);
    let year = w.year;
    let v: Vec<usize> = (0..w.people.len()).filter(|&i| w.people[i].stage == Stage::HighSchool).take(n).collect();
    let you = w.career.as_ref().map(|c| c.coach).unwrap();
    for (k, &i) in v.iter().enumerate() {
        let q = &mut w.people[i];
        q.stage = Stage::College;
        q.college = Some(p);
        q.class_of = year - 1;
        q.college_seasons = 1;
        q.last_role = Some(PlayingTime::Starter);
        q.player.position = Position::LB;
        // Leadership from 20 down, ego and professionalism level: the vote
        // follows leadership (its noise is 1.5 a man, the steps 2).
        let t = &mut q.player.personality.traits;
        t.set(Trait::Leadership, 20.0 - 2.0 * k as f64);
        t.set(Trait::Ego, 10.0);
        t.set(Trait::Professionalism, 10.0);
        q.trust = 0.5;
        q.trust_in = you as u32;
        q.mood = 0.0;
    }
    w.programs[p].roster = v.clone();
    w.coaches[you].ability.discipline = if strict { 18.0 } else { 8.0 };
    (w, p, v)
}

fn id(w: &World, i: usize) -> u32 {
    w.people[i].player.id
}

/// Promise man `i` a captaincy today; its index.
fn promise(w: &mut World, i: usize) -> usize {
    let him = id(w, i);
    talks::promise(w, PromiseTo::Player(him), PromiseKind::Captain, 0);
    w.folk.promises.len() - 1
}

/// Fall camp: the staff sets everyone a starter and the room votes.
fn camp(w: &mut World, v: &[usize]) {
    w.today = w.dates.fall_camp;
    let roles: Vec<(usize, PlayingTime)> = v.iter().map(|&i| (i, PlayingTime::Starter)).collect();
    crate::world::people::college_camp_mood(w, &roles);
}

fn room_name(w: &World) -> f32 {
    w.folk.mine.name(Name::Room)
}

#[test]
fn a_captaincy_is_due_when_the_room_votes() {
    let (mut w, p, v) = room(12, false);
    w.today = w.dates.fall_camp.add(-120);
    let k = promise(&mut w, v[10]);
    let pr = &w.folk.promises[k];
    assert_eq!(pr.words(), "that he'd be a captain");
    assert_eq!(pr.due, w.dates.fall_camp.add(captain::GRACE), "judged at fall camp, with grace");
    assert!(captain::open(&w, v[0]));
    // Not a man on another team.
    let other = (0..w.people.len()).find(|&i| w.people[i].stage == Stage::HighSchool).unwrap();
    assert!(!captain::open(&w, other));
    let _ = p;
}

#[test]
fn the_vote_keeps_it_for_the_room_leader_at_no_cost() {
    let (mut w, p, v) = room(12, false);
    w.today = w.dates.fall_camp.add(-60);
    let k = promise(&mut w, v[0]);
    let (trust, name) = (w.people[v[0]].trust, room_name(&w));
    camp(&mut w, &v);
    let caps = takeover::captains_of(&w, Team::College(p));
    assert_eq!(caps.len(), 4, "the vote's four, no one named beside them");
    assert!(caps.contains(&v[0]));
    assert_eq!(w.folk.promises[k].kept, Some(true));
    assert!(w.people[v[0]].trust > trust, "a kept promise lifts his trust");
    assert!(room_name(&w) >= name, "the room voted him: nothing to resent");
}

#[test]
fn you_name_a_man_the_vote_left_out_and_the_room_notices() {
    // The same camp without the promise, to read what it cost.
    let (mut base, _, v0) = room(12, false);
    base.today = base.dates.fall_camp.add(-60);
    camp(&mut base, &v0);
    let (mut w, p, v) = room(12, false);
    w.today = w.dates.fall_camp.add(-60);
    let last = v[11];
    let k = promise(&mut w, last);
    let name = room_name(&w);
    camp(&mut w, &v);
    let caps = takeover::captains_of(&w, Team::College(p));
    assert_eq!(caps.len(), 5, "the vote's four and the man you promised");
    assert!(caps.contains(&last));
    assert_eq!(w.folk.promises[k].kept, Some(true), "your word kept");
    assert!(room_name(&w) < name - 0.05, "the room saw you pass seven men: {} -> {}", name, room_name(&w));
    // The men the room put ahead of him feel passed over.
    let base_caps = takeover::captains_of(&base, Team::College(p));
    let passed: Vec<usize> = v.iter().copied().filter(|j| !base_caps.contains(j) && *j != last).collect();
    let lost = passed.iter().filter(|&&j| w.people[j].trust < base.people[j].trust - 0.01).count();
    assert!(lost >= 2, "{lost} passed men lost trust");
    assert!(w.folk.log.iter().any(|n| n.text.contains("named him a captain")));
}

#[test]
fn a_coach_who_picks_his_own_pays_half() {
    let cost = |strict: bool| {
        let (mut w, _, v) = room(12, strict);
        w.today = w.dates.fall_camp.add(-60);
        promise(&mut w, v[11]);
        let name = room_name(&w);
        camp(&mut w, &v);
        name - room_name(&w)
    };
    let (vote, pick) = (cost(false), cost(true));
    assert!(pick > 0.0 && pick < 0.75 * vote, "picked {pick} against voted {vote}");
}

#[test]
fn with_its_setting_off_the_vote_stands_and_breaks_it() {
    let (mut w, p, v) = room(12, false);
    w.today = w.dates.fall_camp.add(-60);
    let last = v[11];
    let k = promise(&mut w, last);
    crate::world::folk::usage::set_off(&mut w, k, true).unwrap();
    let trust = w.people[last].trust;
    camp(&mut w, &v);
    assert!(!takeover::captains_of(&w, Team::College(p)).contains(&last));
    assert_eq!(w.folk.promises[k].kept, Some(false), "you broke it");
    assert!(w.people[last].trust < trust);
}

#[test]
fn a_missed_condition_releases_you() {
    let (mut w, p, v) = room(12, false);
    w.today = w.dates.fall_camp.add(-60);
    let last = v[11];
    let k = promise(&mut w, last);
    talks::set_terms(&mut w, k, Terms { cond: Cond::Grades, ..Terms::default() });
    w.people[last].cal.fall_ineligible = true;
    let trust = w.people[last].trust;
    camp(&mut w, &v);
    assert!(!takeover::captains_of(&w, Team::College(p)).contains(&last), "not named over the vote");
    assert_eq!(w.folk.promises[k].kept, Some(true));
    assert!(w.folk.promises[k].released);
    assert!(w.people[last].trust >= trust, "no cost for his own miss");
}

#[test]
fn he_believes_it_by_where_he_stands_in_the_room() {
    let (mut w, _, v) = room(12, false);
    let lead = captain::likely(&w, v[0]);
    let last = captain::likely(&w, v[11]);
    assert!(lead >= 0.9 && last <= 0.5, "leader {lead}, last {last}");
    // A first-year can't be voted at college.
    w.people[v[5]].college_seasons = 0;
    assert!(captain::likely(&w, v[5]) <= 0.35 + 1e-9);
    // Under a coach who picks his own, the coach can give it.
    let (w, _, v) = room(12, true);
    assert!(captain::likely(&w, v[11]) >= 0.9);
}

#[test]
fn a_typed_captain_promise_lands_as_one() {
    let (mut w, _, v) = room(12, false);
    let him = id(&w, v[3]);
    let read = scene::PromiseRead { kind: "captain".into(), ..Default::default() };
    talks::typed_promise(&mut w, 0, him, &read);
    let p = w.folk.promises.last().unwrap();
    assert_eq!((p.to, p.kind), (PromiseTo::Player(him), PromiseKind::Captain));
    assert!(p.belief > 0.0);
}
