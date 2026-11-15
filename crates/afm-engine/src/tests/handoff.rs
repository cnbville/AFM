//! Hand-off projections (Cain 2026-11-15): before you hand a meeting to
//! your staff the game shows what each choice would do, side by side, from
//! the same code that does it. The projections only read the world, name
//! what happens on the button, and work a man's result at both ends of
//! *your read* of his ratings, never his true ones.

use super::legacy::seat;
use crate::player::PlayingTime;
use crate::world::coach::{Group, Role};
use crate::world::folk::handoff::{self, Tone};
use crate::world::folk::scene::{self, Kind};
use crate::world::folk::talks::{self, Subject, word_of};
use crate::world::{Stage, World};

/// Your program with one sophomore on it and a position coach at his seat
/// whose Relationships is `rel`.
fn room(rel: f64) -> (World, usize, usize) {
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
    let now = (0..w.coaches.len()).find(|&c| c != you).unwrap();
    let mut job = w.coaches[you].job.unwrap();
    job.role = Role::PositionCoach(group);
    w.coaches[now].job = Some(job);
    w.programs[p].position_coaches[group as usize] = Some(now);
    w.coaches[now].ability.relationships = rel;
    (w, i, now)
}

#[test]
fn a_one_on_one_shows_both_sides_through_your_read_of_the_man() {
    for rel in [3.0, 9.0, 14.0, 19.0] {
        let (mut w, i, coach) = room(rel);
        let id = w.people[i].player.id;
        let sid = talks::start_talk(&mut w, id, Subject::Role).unwrap();
        let s = w.folk.scenes.iter().find(|s| s.id == sid).unwrap().clone();
        // The button names the man.
        let (tone, label) = handoff::label(&w, &s);
        assert_eq!(tone, Tone::Man);
        let last = w.coaches[coach].name.split_whitespace().last().unwrap().to_string();
        assert!(label.starts_with("Hand to ") && label.ends_with(&last), "{label}");
        let h = handoff::preview(&w, sid).unwrap();
        assert_eq!(h.label, label);
        // Your read of his Relationships holds his true rating and is a range.
        let r = &h.hand.reads[0];
        assert_eq!(r.name, "Relationships");
        assert!(r.lo as f64 <= rel && rel <= r.hi as f64 || rel < 1.0, "{rel} not in {}-{}", r.lo, r.hi);
        assert!(!h.hand.word.is_empty() && !h.hold.word.is_empty());
        assert!(h.hand.list.iter().any(|x| x.contains("promise")), "the one thing only you can do: {:?}", h.hand.list);
        assert!(!h.hold.list.is_empty() && h.hold.bars.len() == 2 && h.hand.bars.len() == 2);
        // It reads only: the world is as it was.
        let (trust, mood) = (w.people[i].trust, w.people[i].mood);
        let again = handoff::preview(&w, sid).unwrap();
        assert_eq!(serde_json::to_string(&again).unwrap(), serde_json::to_string(&h).unwrap());
        assert_eq!((w.people[i].trust, w.people[i].mood), (trust, mood));
        // What the hand does lands inside the projection's words.
        scene::open(&mut w, sid).unwrap();
        let (trust, mood) = (w.people[i].trust, w.people[i].mood);
        scene::hand(&mut w, sid).unwrap();
        let moved = (w.people[i].trust - trust) + (w.people[i].mood - mood);
        let word = word_of(moved);
        assert!(
            h.hand.word.to_lowercase().contains(&word.to_lowercase()),
            "rel {rel}: {} vs {word} ({moved})",
            h.hand.word
        );
    }
}

/// Projection against what the hand does, over every rating. The
/// projection gives your read's range as words ("should land to barely
/// moves him" when your read of his rating spans two), so the check is the
/// handed word being one of the words shown; the exact one-word matches are
/// printed beside it.
#[test]
fn the_handed_word_is_inside_the_projected_words_nine_times_in_ten() {
    let (mut ok, mut inside, mut n) = (0, 0, 0);
    for rel in 1..=20 {
        for _ in 0..2 {
            let (mut w, i, _) = room(rel as f64);
            let id = w.people[i].player.id;
            let sid = talks::start_talk(&mut w, id, Subject::Role).unwrap();
            let h = handoff::preview(&w, sid).unwrap();
            scene::open(&mut w, sid).unwrap();
            let (trust, mood) = (w.people[i].trust, w.people[i].mood);
            scene::hand(&mut w, sid).unwrap();
            let word = word_of((w.people[i].trust - trust) + (w.people[i].mood - mood)).to_lowercase();
            let said = h.hand.word.to_lowercase();
            n += 1;
            if said == word {
                ok += 1;
            }
            if said.contains(&word) {
                inside += 1;
            }
        }
    }
    println!("the projected word is the handed word {ok} of {n}; inside the projected words {inside} of {n}");
    assert!(inside * 10 >= n * 9, "the real result is inside the words your read gives: {inside} of {n}");
}

#[test]
fn every_kind_names_its_button_and_shows_what_it_costs() {
    let mut w = seat("college");
    let today = w.today;
    let mut seen = std::collections::BTreeMap::new();
    let kinds = [
        Kind::TeamMeeting,
        Kind::PressIntro,
        Kind::BossFirst,
        Kind::Holdover,
        Kind::StaffReport,
        Kind::Recruiting,
        Kind::Academic,
        Kind::Trainer,
        Kind::Booster,
        Kind::Media,
        Kind::Tour,
        Kind::Plan,
        Kind::Parents,
        Kind::Feeder,
        Kind::HireReaction,
        Kind::OldCoach,
        Kind::Press,
        Kind::TeamTalk,
        Kind::BossMeeting,
        Kind::StaffLine,
        Kind::Interview,
        Kind::Firing,
        Kind::Discipline,
        Kind::ReachOut,
        Kind::Trip,
        Kind::StarPower,
        Kind::SecondOpinion,
        Kind::Ritual,
        Kind::ParentCall,
        Kind::Proposal,
        Kind::JobCall,
        Kind::Leaders,
        Kind::FlightRisks,
        Kind::Gm,
        Kind::Family,
        Kind::Backchannel,
        Kind::Negotiation,
        Kind::CollegeCall,
        Kind::JobInterview,
        Kind::StaffMeeting,
        Kind::CombineTalk,
        Kind::SigneeRelease,
        Kind::Council,
        Kind::DraftTalk,
        Kind::Sitdown,
        Kind::Reference,
    ];
    for k in kinds {
        let sid = scene::add(&mut w, scene::Scene::new(k, today, format!("A {}", k.name())));
        let s = w.folk.scenes.iter().find(|s| s.id == sid).unwrap().clone();
        let (tone, label) = handoff::label(&w, &s);
        assert!(!label.is_empty(), "{k:?}");
        let h = handoff::preview(&w, sid).unwrap_or_else(|e| panic!("{k:?}: {e}"));
        assert_eq!((h.tone, &h.label), (tone, &label), "{k:?}");
        for side in [&h.hand, &h.hold] {
            for t in side.list.iter().chain([&side.word, &side.text, &side.title]) {
                assert!(!t.contains(['{', '}']), "{k:?}: {t}");
            }
        }
        assert!(!h.hold.list.is_empty(), "{k:?}: what holding it gets you");
        seen.insert(format!("{k:?}"), (tone, label));
    }
    // A skip says so on the button; a standard says what it applies.
    assert_eq!(seen["BossMeeting"].0, Tone::Skip);
    assert_eq!(seen["Discipline"].1, "Let the staff rule");
    assert_eq!(seen["SecondOpinion"].1, "Let the doctors arrange it");
    assert!(seen.values().all(|(_, l)| l != "Hand to staff"), "no kind keeps the one old button");
    for (k, (t, l)) in &seen {
        println!("{k:<14} {t:?} {l}");
    }
}

#[test]
fn a_holiday_shows_each_meeting_inside_it_with_its_line() {
    let (mut w, i, _) = room(12.0);
    let id = w.people[i].player.id;
    let sid = talks::start_talk(&mut w, id, Subject::Role).unwrap();
    // A skip further out than the holiday stays out of it.
    let far = w.today.add(30);
    let out = scene::add(&mut w, scene::Scene::new(Kind::BossMeeting, far, "The AD, later"));
    let near = w.today.add(3);
    let boss = scene::add(&mut w, scene::Scene::new(Kind::BossMeeting, near, "The AD"));
    let p = handoff::holiday(&w, w.today.add(14));
    assert!(p.rows.iter().any(|r| r.id == sid), "today's talk falls inside");
    assert!(p.rows.iter().any(|r| r.id == boss && r.tone == Tone::Skip));
    assert!(!p.rows.iter().any(|r| r.id == out));
    assert!(p.rows.windows(2).all(|x| x[0].date <= x[1].date));
    assert!(p.rows.iter().all(|r| !r.line.is_empty()));
    assert!(!p.unknown.is_empty());
    // A meeting dated the day it comes back isn't handed.
    let back = w.today.add(14);
    let on = scene::add(&mut w, scene::Scene::new(Kind::BossMeeting, back, "The AD, on the day"));
    assert!(!handoff::holiday(&w, back).rows.iter().any(|r| r.id == on));
}
