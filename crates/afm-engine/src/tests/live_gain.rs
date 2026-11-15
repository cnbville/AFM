//! Live talks lane 6, rooms that gain something (LT11, LT12, LT17): the
//! interview rooms (a typed question equal to the clicked one, minutes by
//! its length, a flagged worry asked straight out read closer), asking a
//! source (only what his look shows, half an hour, once a topic a year,
//! into your file), and a case of your own at the staff meeting (the plan
//! moves your way, checked after the next game both ways).

use super::legacy::seat;
use crate::world::folk::Team;
use crate::world::folk::dialogue::{self, Case};
use crate::world::folk::scene::{self, Heard, Kind, Scene, Who};
use crate::world::scouting::file::{self, TRAIT_NAMES, TraitRead};
use crate::world::scouting::own::{self, Interview, Topic};
use crate::world::scouting::readers::{self, COACH};
use crate::world::scouting::{Kind as LookKind, dept};
use crate::world::{Stage, Visit, World};

fn typed(code: u32, words: &str) -> Heard {
    Heard { pick: code, tone: "calm".into(), words: words.into(), ..Heard::default() }
}

fn bytes(w: &World) -> Vec<u8> {
    rmp_serde::to_vec(w).unwrap()
}

fn untyped(w: &mut World) {
    for s in &mut w.folk.scenes {
        for l in &mut s.lines {
            l.typed = None;
        }
    }
}

fn prog(w: &World) -> usize {
    match Team::seat(w).unwrap() {
        Team::College(p) => p,
        _ => panic!("a college seat"),
    }
}

/// A high school recruit on an official visit to your program today, with
/// a hard worker's habits read low by your own call a week ago: his work
/// ethic and coachability come out as worries.
fn recruit(w: &mut World) -> u32 {
    let p = prog(w);
    if !w.people.iter().any(|q| q.stage == Stage::HighSchool) {
        w.add_class_aged(17);
    }
    let i = w.people.iter().position(|q| q.stage == Stage::HighSchool && q.player.age >= 16).expect("a recruit");
    let d = w.today;
    let q = &mut w.people[i];
    q.player.personality.work_ethic = 1.5;
    q.player.personality.coachability = 2.0;
    q.cal.visits.push((p, d, Visit::Official));
    let id = q.player.id;
    let you = COACH + w.career.as_ref().unwrap().coach as u32;
    dept::add(w, id, you, LookKind::Call, d.add(-7));
    id
}

fn room(w: &mut World, id: u32) -> u32 {
    let s = dialogue::start_combine_talk(w, id, "Official visit sit-down").unwrap();
    scene::open(w, s).unwrap();
    s
}

fn codes(s: &Scene) -> Vec<u32> {
    s.picks.iter().map(|p| p.code).collect()
}

#[test]
fn a_typed_interview_question_is_the_clicked_one() {
    let (mut a, mut b) = (seat("college"), seat("college"));
    let (ia, ib) = (recruit(&mut a), recruit(&mut b));
    let (sa, sb) = (room(&mut a, ia), room(&mut b, ib));
    let clicked = scene::answer(&mut a, sa, 965).unwrap();
    let said = scene::say(&mut b, sb, typed(965, "What does football mean to you?"), false).unwrap();
    assert_eq!(clicked.c, said.c, "a short question takes the topic's minutes");
    assert_eq!(codes(&clicked), codes(&said));
    untyped(&mut b);
    assert!(bytes(&a) == bytes(&b), "the world after a short typed question is the chip's");
}

#[test]
fn a_long_question_takes_minutes_off_the_clock_and_adds_none_to_the_topic() {
    let (mut a, mut b) = (seat("college"), seat("college"));
    let (ia, ib) = (recruit(&mut a), recruit(&mut b));
    let (sa, sb) = (room(&mut a, ia), room(&mut b, ib));
    let clicked = scene::answer(&mut a, sa, 965).unwrap();
    let long = "Tell me what this game means to you, ".repeat(9);
    assert!((300..=400).contains(&long.len()));
    let said = scene::say(&mut b, sb, typed(965, &long), false).unwrap();
    assert_eq!(clicked.c - 2, said.c, "300 to 400 characters take two more minutes");
    assert_eq!(clicked.b & 0x3FFF_FFFF, said.b & 0x3FFF_FFFF, "the topic's minutes are the chip's");
    // Both rooms wrap up: the long question bought no read.
    scene::answer(&mut a, sa, 969).unwrap();
    scene::answer(&mut b, sb, 969).unwrap();
    let (iva, ivb) = (&a.scouting.own.interviews[0], &b.scouting.own.interviews[0]);
    assert_eq!(iva.spent, ivb.spent);
}

#[test]
fn the_worries_your_scouts_flagged_are_offered_and_read_closer() {
    let mut w = seat("college");
    let id = recruit(&mut w);
    let flagged = own::flagged_worries(&w, id);
    assert!(flagged.contains(&0) || flagged.contains(&1), "the low reads are flagged: {flagged:?}");
    assert!(flagged.len() <= own::WORRIES);
    let s = room(&mut w, id);
    let open = w.folk.scenes.iter().find(|x| x.id == s).unwrap().clone();
    for k in &flagged {
        assert!(codes(&open).contains(&(970 + *k as u32)), "worry {k} offered");
    }
    let k = flagged[0];
    let after = scene::answer(&mut w, s, 970 + k as u32).unwrap();
    assert!(!codes(&after).contains(&(970 + k as u32)), "asked once");
    assert!(codes(&after).contains(&967), "a follow-up is open after a worry");
    let mine = after.lines.iter().rev().find(|l| l.by == 1).unwrap();
    assert_eq!(mine.text, own::worry_question(k));
    scene::answer(&mut w, s, 969).unwrap();
    let iv = w.scouting.own.interviews.last().unwrap();
    assert_eq!(iv.worries, vec![(k as u8, 8)], "an official visit's 45 minutes make a worry 8");
    assert!(iv.answers.iter().any(|a| a.0 == own::worry_topic(k)), "his answer is his topic's");
}

/// The spread of the room's read of trait k around the truth over the
/// world's recruits: one topic question of `m` minutes, or the same minutes
/// on the trait as a flagged worry.
fn spread(w: &mut World, k: usize, worry: bool) -> f64 {
    let ids: Vec<u32> =
        w.people.iter().filter(|q| q.stage == Stage::HighSchool).map(|q| q.player.id).take(400).collect();
    let topic = own::worry_topic(k);
    let d = w.today;
    w.scouting.own.interviews = ids
        .iter()
        .map(|&id| Interview {
            id,
            date: d,
            room: "Official visit sit-down".into(),
            minutes: 45,
            spent: if worry { Vec::new() } else { vec![(topic, 8)] },
            answers: Vec::new(),
            quiz: None,
            worries: if worry { vec![(k as u8, 8)] } else { Vec::new() },
        })
        .collect();
    let mut sq = 0.0;
    for &id in &ids {
        let mut t = vec![TraitRead::default(); file::TRAITS];
        own::interview_traits(w, id, &mut t);
        let i = crate::world::scouting::place(w, id).unwrap();
        assert!(t[k].seen);
        sq += (t[k].centre - file::trait_value(&w.people[i], k)).powi(2);
    }
    (sq / ids.len() as f64).sqrt()
}

#[test]
fn a_worry_asked_straight_out_reads_tighter_than_todays_topic() {
    let mut w = seat("college");
    w.add_class_aged(17);
    for k in [0usize, 7] {
        let topic = spread(&mut w, k, false);
        let worry = spread(&mut w, k, true);
        println!("{}: the topic's spread {topic:.2}, the worry's {worry:.2}", TRAIT_NAMES[k]);
        assert!(worry < topic, "{}: {worry} against {topic}", TRAIT_NAMES[k]);
        assert!(worry > 0.5 * topic, "{}: tighter, not exact", TRAIT_NAMES[k]);
    }
}

fn source_of(w: &World, id: u32, code: u8) -> own::Source {
    own::sources(w, id).into_iter().find(|s| s.code == code).unwrap_or_else(|| panic!("source {code}"))
}

#[test]
fn a_source_never_tells_more_than_his_look_shows() {
    let mut w = seat("college");
    let id = recruit(&mut w);
    let list = own::sources(&w, id);
    assert!(list.iter().any(|s| s.code == 1) && list.iter().any(|s| s.code == 3), "a teacher and a teammate");
    // A teacher (a call) sees work ethic, coachability, professionalism,
    // leadership, temper, love and competitiveness: nothing of what drives him.
    let (_, _, a) = own::source_answer(&w, id, 1, 3).unwrap();
    assert!(a.said.is_empty() && a.unknown.len() == 3, "{a:?}");
    assert!(a.text.contains("not something I'd know"));
    // A teammate (practice) speaks to temper and leadership, not
    // professionalism or ego.
    let (_, _, a) = own::source_answer(&w, id, 3, 2).unwrap();
    let names: Vec<&str> = a.said.iter().map(|x| x.0).collect();
    assert_eq!(names, vec!["Temper", "Leadership"]);
    assert_eq!(a.unknown, vec!["Professionalism", "Ego"]);
    // Every source on every topic: only traits his look shows.
    for s in &list {
        for t in 0..own::SOURCE_TOPICS.len() as u8 {
            let (_, _, a) = own::source_answer(&w, id, s.code, t).unwrap();
            assert_eq!(a.said.len() + a.unknown.len(), own::SOURCE_TOPICS[t as usize].1.len());
        }
    }
    // The same source says the same thing all year.
    let (_, _, x) = own::source_answer(&w, id, 3, 2).unwrap();
    let (_, _, y) = own::source_answer(&w, id, 3, 2).unwrap();
    assert_eq!(x.text, y.text);
    let _ = source_of(&w, id, 3);
    // Nothing for your own players.
    let mine = w.programs[prog(&w)].roster.first().copied();
    if let Some(i) = mine {
        let pid = w.people[i].player.id;
        assert!(own::sources(&w, pid).is_empty());
    }
}

#[test]
fn a_shade_tilts_what_he_says_and_a_warm_tie_softens_it() {
    let mut w = seat("college");
    w.add_class_aged(17);
    let i = w.people.iter().position(|q| q.stage == Stage::HighSchool).unwrap();
    let p = &w.people[i];
    let key = 0x77;
    let warm = file::source_says(p, LookKind::Call, file::Shade::Warm, 10.0, 0.0, key, &[0]);
    let flat = file::source_says(p, LookKind::Call, file::Shade::True, 10.0, 0.0, key, &[0]);
    let friend = file::source_says(p, LookKind::Call, file::Shade::Warm, 10.0, 1.0, key, &[0]);
    assert!(warm[0].tilt > friend[0].tilt && friend[0].tilt > flat[0].tilt);
    let sharp = file::source_says(p, LookKind::Call, file::Shade::True, 18.0, 0.0, key, &[0]);
    assert!(sharp[0].sd < flat[0].sd, "a better eye misreads less");
}

#[test]
fn asking_a_source_costs_half_an_hour_once_a_topic_and_feeds_your_file() {
    let mut w = seat("college");
    let id = recruit(&mut w);
    let staff = readers::seat_staff(&w).unwrap();
    let before = readers::read(&w, staff, id).unwrap().2.traits;
    let hours = w.folk.mine.hours;
    let a = own::ask_source(&mut w, id, 3, 2).unwrap();
    assert!(!a.said.is_empty());
    assert!((w.folk.mine.hours - hours - own::SOURCE_HOURS).abs() < 1e-4, "half an hour of your week");
    assert_eq!(own::ask_source(&mut w, id, 3, 2).unwrap_err(), "You asked him that already this year.");
    let after = readers::read(&w, staff, id).unwrap().2.traits;
    for k in [7usize, 9] {
        assert!(after[k].sd < before[k].sd || !before[k].seen, "{}: the file gained", TRAIT_NAMES[k]);
    }
    for k in [8usize, 10] {
        assert_eq!(after[k].sd, before[k].sd, "{}: nothing he couldn't see", TRAIT_NAMES[k]);
    }
    // A full week refuses the call.
    w.folk.mine.hours = 1000.0;
    assert_eq!(own::ask_source(&mut w, id, 1, 0).unwrap_err(), "Your week is full.");
}

/// A staff meeting where your coordinator (DC for kinds 0 and 1) argues
/// his case.
fn meeting(w: &mut World, kind: u8) -> u32 {
    let [oc, dc] = crate::world::folk::gametalk::coordinators(w, Team::seat(w).unwrap());
    let c = if kind <= 1 { dc } else { oc }.expect("a coordinator");
    let mut s = Scene::new(Kind::StaffMeeting, w.today, "Staff meeting: your DC wants a change");
    s.a = c as u64;
    s.b = kind as u64;
    s.c = 31;
    s.who = vec![Who::Coach(c)];
    let id = scene::add(w, s);
    scene::open(w, id).unwrap();
    id
}

#[test]
fn your_own_case_moves_the_plan_your_way() {
    let mut w = seat("college");
    let id = meeting(&mut w, 0);
    let open = w.folk.scenes.iter().find(|x| x.id == id).unwrap().clone();
    let own: Vec<u32> = dialogue::own_moves(0).into_iter().map(|m| 954 + m as u32).collect();
    assert_eq!(own, vec![955, 958, 959], "the other moves on his side of the ball");
    for c in &own {
        assert!(codes(&open).contains(c));
    }
    let before = w.plans.guide.pressure.to.unwrap_or(0.3);
    scene::answer(&mut w, id, 958).unwrap();
    let after = w.plans.guide.pressure.to.unwrap();
    assert!((before - 0.08 - after).abs() < 1e-6, "less pressure, one full step: {before} to {after}");
    let case = w.folk.dialogue.case.unwrap();
    assert_eq!((case.kind, case.said, case.mine), (0, 2, Some(4)));
    // An offense case offers the other way only.
    let mut w = seat("college");
    let id = meeting(&mut w, 2);
    let open = w.folk.scenes.iter().find(|x| x.id == id).unwrap().clone();
    assert!(codes(&open).contains(&957) && !codes(&open).contains(&954) && !codes(&open).contains(&958));
}

#[test]
fn a_typed_case_of_your_own_is_the_clicked_one() {
    let (mut a, mut b) = (seat("college"), seat("college"));
    let (ia, ib) = (meeting(&mut a, 0), meeting(&mut b, 0));
    scene::answer(&mut a, ia, 959).unwrap();
    scene::say(&mut b, ib, typed(959, "No, I want more man coverage this week."), false).unwrap();
    untyped(&mut b);
    assert!(bytes(&a) == bytes(&b));
}

#[test]
fn a_case_of_yours_is_checked_both_ways_at_todays_sizes() {
    let d = crate::calendar::Date(0);
    let case = |said, mine, kind| Case { coach: 0, kind, said, date: d, before: 31, mine };
    // Defense: 28 or more allowed went wrong, 17 or fewer went well.
    assert_eq!(dialogue::case_trust(&case(2, Some(4), 0), 31), -0.06);
    assert_eq!(dialogue::case_trust(&case(2, Some(4), 0), 10), 0.03);
    assert_eq!(dialogue::case_trust(&case(2, Some(4), 0), 21), 0.0);
    // Offense: under 17 scored went wrong, 28 or more went well.
    assert_eq!(dialogue::case_trust(&case(2, Some(3), 2), 10), -0.06);
    assert_eq!(dialogue::case_trust(&case(2, Some(3), 2), 35), 0.03);
    // Today's cases keep today's numbers.
    assert_eq!(dialogue::case_trust(&case(2, None, 0), 31), -0.06);
    assert_eq!(dialogue::case_trust(&case(2, None, 0), 10), 0.01);
    assert_eq!(dialogue::case_trust(&case(0, None, 0), 10), 0.03);
    assert_eq!(dialogue::case_trust(&case(0, None, 0), 31), -0.01);
    assert_eq!(dialogue::case_trust(&case(1, None, 0), 31), 0.0);
}

#[test]
fn an_old_case_reads_with_no_case_of_yours() {
    let old = r#"{"coach":3,"kind":1,"said":0,"date":12000,"before":24}"#;
    let c: Case = serde_json::from_str(old).unwrap();
    assert_eq!(c.mine, None);
    let iv = r#"{"id":5,"date":12000,"room":"Combine interview","minutes":18,"spent":[["Football",3]],"answers":[],"quiz":null}"#;
    let iv: Interview = serde_json::from_str(iv).unwrap();
    assert!(iv.worries.is_empty());
    assert_eq!(iv.spent[0].0, Topic::Football);
}
