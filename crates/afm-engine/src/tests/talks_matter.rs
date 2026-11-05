//! Talks that matter (new bugs, Cain 2026-11-03: "if i have player with
//! worries, i want to be able to promise him... a WR1 at a D1 school won't
//! ruin his career because a friend left... talks should matter, cause and
//! consequence"). A friend who leaves becomes his subject for four weeks,
//! with its own lines and a promise; every exchange says how it went over
//! and moves the bars; a starter weighs his role before following a friend
//! to the portal; "sir" follows your coach's he or she.

use super::legacy::seat;
use crate::player::PlayingTime;
use crate::rng::Rng;
use crate::world::folk::scene::{self, Kind};
use crate::world::folk::talks::{self, PromiseKind, PromiseTo, Subject};
use crate::world::folk::{ties, voice};
use crate::world::pronoun::Pro;
use crate::world::{PortalEntry, Stage, World};

/// Your college program with `n` players on it (sophomores), by index.
fn room(n: usize) -> (World, usize, Vec<usize>) {
    let mut w = seat("college");
    let p = w.seat.and_then(|s| s.program).unwrap();
    w.add_class_aged(19);
    let year = w.year;
    let v: Vec<usize> = (0..w.people.len()).filter(|&i| w.people[i].stage == Stage::HighSchool).take(n).collect();
    for &i in &v {
        let q = &mut w.people[i];
        q.stage = Stage::College;
        q.college = Some(p);
        q.class_of = year - 1;
        q.college_seasons = 1;
        q.last_role = Some(PlayingTime::Backup);
        w.programs[p].roster.push(i);
    }
    (w, p, v)
}

fn id(w: &World, i: usize) -> u32 {
    w.people[i].player.id
}

/// The next Monday on or after today (your roster's weekly look at ties).
fn monday(w: &World) -> crate::calendar::Date {
    let mut d = w.today;
    while d.weekday() != crate::calendar::Weekday::Mon {
        d = d.add(1);
    }
    d
}

#[test]
fn a_friend_leaving_is_his_subject_and_takes_a_promise() {
    let (mut w, p, v) = room(3);
    let (a, b) = (v[0], v[1]);
    w.people[a].last_role = Some(PlayingTime::Starter);
    let (ia, ib) = (id(&w, a), id(&w, b));
    ties::bond(&mut w, ia, ib, 55);
    // A Monday with both on the roster, then he leaves and the next Monday
    // sees it.
    let d = monday(&w);
    w.today = d;
    ties::day(&mut w, d);
    assert!(talks::friend_gone(&w, ia).is_none(), "nobody has left yet");
    w.programs[p].roster.retain(|&j| j != b);
    let d = d.add(7);
    w.today = d;
    ties::day(&mut w, d);
    assert_eq!(talks::friend_gone(&w, ia), Some(ib), "his friend's leaving weighs on him");
    let (sub, words) = talks::concern(&w, a);
    assert_eq!(sub, Subject::Friend, "{words}");
    let gone = w.people[b].name.split_whitespace().next().unwrap().to_string();
    assert!(words.starts_with(&gone), "{words}");
    let chips = talks::subjects_for(&w, ia);
    assert_eq!(chips[0], ("friend", format!("{gone} leaving")), "the friend's chip comes first");
    assert_eq!(chips.len(), Subject::ALL.len() + 1);

    // The talk: it opens on him, the bars know where he came in, and a
    // promise is on the table.
    let trust = w.people[a].trust;
    let sid = talks::start_talk(&mut w, ia, Subject::Friend).unwrap();
    let s = scene::open(&mut w, sid).unwrap();
    assert_eq!(s.kind, Kind::OneOnOne);
    assert_eq!(Subject::from_u64(s.b), Subject::Friend);
    assert!(s.c & 2 != 0 && (s.f[2] - trust).abs() < 1e-6, "the bars' marks are set");
    let first = s.lines.iter().find(|l| l.by == 2).expect("he speaks").text.clone();
    assert!(!first.contains(['{', '}']), "{first}");
    let promise = s.picks.iter().find(|x| x.mv == "Promise").expect("a promise on a friend leaving").clone();
    assert!(promise.label.contains(&gone), "{}", promise.label);
    assert!(s.picks.iter().any(|x| x.mv.starts_with("Tell")), "tell him straight he's your starter");
    assert!(s.picks.iter().any(|x| x.mv.starts_with("Hand")), "ask a teammate to look out for him");

    let s = scene::answer(&mut w, sid, promise.code).unwrap();
    let said = s.lines.iter().rev().find(|l| l.by == 2).unwrap();
    assert!(said.went.as_deref().is_some_and(|x| x.ends_with('.')), "how it went over: {:?}", said.went);
    let made = w.folk.promises.iter().find(|x| matches!(x.to, PromiseTo::Player(t) if t == ia)).expect("logged");
    assert_eq!(made.kind, PromiseKind::Place(0), "a starter is promised his starting job");
    assert_eq!(made.kind.words(), "that he'd keep his starting job");
    let made = made.clone();

    // Kept while he stays your starter; broken once he's gone.
    assert!(talks::kept(&w, &made), "still on the team, still the starter");
    w.people[a].last_role = Some(PlayingTime::Backup);
    w.seat_depth.clear();
    assert!(!talks::kept(&w, &made), "benched breaks it");
    w.people[a].last_role = Some(PlayingTime::Starter);
    w.programs[p].roster.retain(|&j| j != a);
    assert!(!talks::kept(&w, &made), "gone from the team breaks it");
}

#[test]
fn every_worry_takes_a_promise() {
    let (mut w, _, v) = room(1);
    let a = v[0];
    let ia = id(&w, a);
    for sub in [Subject::Behavior, Subject::Life, Subject::Team] {
        let sid = talks::start_talk(&mut w, ia, sub).unwrap();
        let s = scene::open(&mut w, sid).unwrap();
        let promise = s.picks.iter().find(|x| x.mv == "Promise");
        let promise = promise.unwrap_or_else(|| panic!("{sub:?}: no promise")).code;
        scene::answer(&mut w, sid, promise).unwrap();
        let k = w.folk.promises.last().unwrap().kind;
        assert_eq!(k, PromiseKind::Place(PlayingTime::Backup as u8), "{sub:?}: a backup keeps his place");
        w.folk.promises.clear();
        w.folk.scenes.retain(|s| s.id != sid);
    }
}

#[test]
fn a_starter_weighs_his_role_before_following_a_friend() {
    let (mut w, _, v) = room(3);
    let (left, starter, backup) = (v[0], v[1], v[2]);
    let d = w.dates.portal_open.add(1);
    w.people[left].cal.portal =
        Some(PortalEntry { entered: d.add(-1), decide_by: d.add(30), why: "a friend left", held: false });
    w.people[starter].last_role = Some(PlayingTime::Starter);
    w.people[backup].last_role = Some(PlayingTime::Backup);
    let il = id(&w, left);
    for &i in &[starter, backup] {
        let o = id(&w, i);
        ties::bond(&mut w, il, o, 55);
        w.people[i].cal.lean = 0.5;
        w.people[i].player.personality.traits.set(crate::player::Trait::Loyalty, 10.0);
    }
    ties::day(&mut w, d);
    let pull = |i: usize| {
        w.people[i].cal.lean_events.iter().find(|e| e.1 == "a close friend entered the portal").map(|e| e.2).unwrap()
    };
    let (s, b) = (pull(starter), pull(backup));
    assert!(s > 1.0 && b > s, "starter x{s:.3}, backup x{b:.3}");
    assert!(((s - 1.0) / (b - 1.0) - 0.4).abs() < 1e-6, "a starter feels 0.4 of a backup's pull");
}

#[test]
fn sir_follows_your_coachs_he_or_she() {
    let f = voice::Facts::new();
    let mut r = Rng::new(7u32);
    assert_eq!(voice::fill("Yes {coach_sir}.", &f, &mut r, Pro::HE), "Yes sir.");
    assert_eq!(voice::fill("Yes {coach_sir}.", &f, &mut r, Pro::SHE), "Yes ma'am.");
    assert_eq!(voice::fill("{coach_sir}. You wanted me?", &f, &mut r, Pro::SHE), "Ma'am. You wanted me?");
    // No line said to you calls you sir outright any more.
    let b = voice::bank();
    for (key, _) in b.keys() {
        if !key.starts_with("talk.") {
            continue;
        }
        for v in b.versions(key) {
            let text = v.0.replace("{coach_sir}", "");
            let words = text.split(|c: char| !c.is_alphabetic()).collect::<Vec<_>>();
            assert!(!words.iter().any(|x| x.eq_ignore_ascii_case("sir")), "{key}: {}", v.0);
        }
    }
}
