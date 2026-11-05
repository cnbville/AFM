//! Talks that matter (new bugs, Cain 2026-11-03: "if i have player with
//! worries, i want to be able to promise him... a WR1 at a D1 school won't
//! ruin his career because a friend left... talks should matter, cause and
//! consequence"). A friend who leaves becomes his subject for four weeks,
//! with its own lines and a promise; every exchange says how it went over
//! and moves the bars; a starter weighs his role before following a friend
//! to the portal; "sir" follows your coach's he or she; and only a brash
//! player talks back in your office (Cain 2026-11-05, "Yes, tag them").

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
    assert!(!talks::lapsed(&w, &made), "dismissed: you broke it");
    // Left on his own, it lapses quietly: drafted, or at another school.
    w.people[a].stage = Stage::Pro;
    assert!(talks::lapsed(&w, &made), "drafted");
    w.people[a].stage = Stage::College;
    w.people[a].college = Some(if p == 0 { 1 } else { 0 });
    assert!(talks::lapsed(&w, &made), "transferred");
}

#[test]
fn what_you_said_lands_when_you_hand_the_talk_over() {
    let (mut w, _, v) = room(1);
    let a = v[0];
    let ia = id(&w, a);
    let sid = talks::start_talk(&mut w, ia, Subject::Role).unwrap();
    let s = scene::open(&mut w, sid).unwrap();
    let praise = s.picks.iter().find(|x| x.mv == "Praise").or(s.picks.first()).unwrap().code;
    let s = scene::answer(&mut w, sid, praise).unwrap();
    let said = (s.f[1] * 100.0).round() as i8;
    assert!(!s.done && said != 0, "one line in, mood so far {}", s.f[1]);
    scene::hand(&mut w, sid).unwrap();
    let notes = &w.folk.moods.iter().find(|x| x.0 == ia).unwrap().1;
    assert!(
        notes.iter().any(|&(_, why, v)| why == talks::Reason::Talk as u8 && v == said),
        "the talk so far lands: {notes:?}, {said}"
    );
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

#[test]
fn only_a_brash_player_talks_back_in_your_office() {
    use crate::player::Trait;
    let (mut w, _, v) = room(1);
    let a = v[0];
    let set = |w: &mut World, ego: f64, temper: f64, trust: f32| {
        let p = &mut w.people[a];
        p.player.personality.traits.set(Trait::Ego, ego);
        p.player.personality.traits.set(Trait::Temper, temper);
        p.trust = trust;
    };
    let brash = |w: &World| voice::Speaker::player(w, a).has("brash");
    // College: a big ego, a hot temper and little trust in you, all three.
    set(&mut w, 16.0, 16.0, 0.3);
    assert!(brash(&w), "cocky, hot-headed and wary of you");
    set(&mut w, 16.0, 16.0, 0.6);
    assert!(!brash(&w), "he trusts you");
    set(&mut w, 16.0, 9.0, 0.3);
    assert!(!brash(&w), "an ego alone");
    set(&mut w, 9.0, 16.0, 0.3);
    assert!(!brash(&w), "a temper alone");
    // High school: never.
    set(&mut w, 18.0, 18.0, 0.1);
    w.people[a].stage = Stage::HighSchool;
    assert!(!brash(&w), "a high school player never talks back");
    // The NFL: an ego or a temper keeps its edge.
    w.people[a].stage = Stage::Pro;
    set(&mut w, 16.0, 9.0, 0.9);
    assert!(brash(&w), "an NFL ego");
    set(&mut w, 9.0, 9.0, 0.1);
    assert!(!brash(&w), "a level-headed pro");
}

#[test]
fn a_school_player_keeps_it_respectful_in_your_office() {
    // Lines that talk back are tagged brash (Cain 2026-11-05): a high school
    // or college player with every trait but brash can't say them, and
    // every talk part keeps plenty he can say.
    let b = voice::bank();
    let traits = ["young", "ego", "hot", "loose", "greedy", "competitor", "shaky", "social", "modern", "nil", "south"];
    let mouthy = ["Whatever you say, Coach.", "Are we done?", "Sit me, then.", "This better be good, Coach."];
    for level in ["hs", "college"] {
        let mut who: Vec<&str> = vec!["player", level];
        who.extend(traits);
        let can = |need: &[String], not: &[String]| {
            need.iter().all(|t| who.contains(&t.as_str())) && !not.iter().any(|t| who.contains(&t.as_str()))
        };
        let mut seen = 0;
        for (key, _) in b.keys() {
            if !key.starts_with("talk.") {
                continue;
            }
            let vs = b.versions(key);
            for v in &vs {
                if mouthy.contains(&v.0) {
                    seen += 1;
                    assert!(!can(v.1, v.2), "{level} can say {key}: {}", v.0);
                }
            }
            let said = vs.iter().filter(|v| can(v.1, v.2)).count();
            let brash = vs.iter().filter(|v| v.1.iter().any(|t| t == "brash")).count();
            assert!(brash == 0 || said >= 5, "{level}: {key} keeps {said} lines");
        }
        assert!(seen >= mouthy.len(), "the mouthy lines are in the bank");
    }
}
