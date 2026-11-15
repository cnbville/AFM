//! The older meetings on the conversation (thread "Move old meetings off
//! chips"): a typed line in discipline, a parent's call, the captains'
//! room, a staff proposal or your job interview plays as a v3 talk with
//! the person across, never onto one of the meeting's chips. The meeting's
//! own picks stay to finish it, a meeting with nobody across takes no
//! typed line, and with no key the chips play as before.

use afm_engine::world::folk::Team;
use afm_engine::world::folk::persona::talk::{self as ptalk, store};
use afm_engine::world::folk::persona::{Them, makeup, meetings};
use afm_engine::world::folk::scene::{self, Kind, Scene, Who};
use serde_json::{Value, json};

use super::super::{cards, ear::EAR, voice::VOICE};
use super::tests::{act, ear, finish, game, ok, scene_of, trust, voice};
use crate::Game;
use crate::live_talk::say::READER;

/// A meeting of this kind with your player, opened through the screen.
fn meeting(g: &mut Game, kind: Kind, him: u32, b: u64) -> u32 {
    let w = g.world.as_mut().unwrap();
    let mut s = Scene::new(kind, w.today, "A meeting");
    s.a = him as u64;
    s.b = b;
    s.stops = false;
    s.place = if kind == Kind::ParentCall { "On the phone".into() } else { "Your office".into() };
    let sid = scene::add(w, s);
    ok(g, "people_scene", json!({ "id": sid }));
    sid
}

/// The tag of the first item on the ear's card for this meeting whose
/// words start so, and the card's text.
fn tag(g: &Game, id: u32, starts: &str) -> (String, String) {
    let w = g.world.as_ref().unwrap();
    let s = scene_of(g, id);
    let room = super::room_of(w, &s);
    let (c, _) = cards::ear(w, &s, &room, &super::parts_of(s.kind), "");
    let t = c.items.iter().find(|i| i.text.starts_with(starts)).map(|i| i.tag.clone());
    (t.unwrap_or_else(|| panic!("{c:?}")), c.them.clone())
}

fn say(g: &mut Game, id: u32, text: &str) -> Value {
    let t = ok(g, "people_say", json!({ "id": id, "text": text }))["ticket"].as_u64().unwrap();
    finish(g, t)
}

#[test]
fn a_typed_line_in_discipline_is_a_talk_with_him_not_a_chip() {
    let (mut g, f, him) = game("old-discipline", true);
    let f = f.unwrap();
    let id = meeting(&mut g, Kind::Discipline, him, 0);
    let before = scene_of(&g, id);
    assert!(!before.picks.is_empty(), "the meeting has its picks");
    let (pride, them) = tag(&g, id, "his pride");
    assert!(!them.is_empty(), "he is across: {them}");
    f.push(
        EAR,
        Ok(ear(vec![act("question", "", "Mood", "")], vec![json!({"on": "", "item": pride, "how": "stung"})], false)),
    );
    f.push(VOICE, Ok(voice("I know, Coach. It won't happen again.", "")));
    let t0 = trust(&g, him);
    let v = say(&mut g, id, "What were you thinking? You let the whole team down.");
    assert_eq!(v["state"], "done", "{v}");
    let s = scene_of(&g, id);
    // No chip was read from the line: the ear and the voice played it, the
    // meeting's picks are still there to finish it, and nothing ended it.
    let asked = f.asked();
    assert!(asked.iter().any(|r| r.label == EAR) && !asked.iter().any(|r| r.label == READER), "{asked:?}");
    assert!(!s.done && s.step == before.step && s.picks.len() == before.picks.len(), "{s:?}");
    assert_eq!(
        s.lines.iter().rev().find(|l| l.by == 1).and_then(|l| l.typed.as_deref()),
        Some("What were you thinking? You let the whole team down.")
    );
    assert_eq!(s.lines.last().unwrap().text, "I know, Coach. It won't happen again.");
    assert!(trust(&g, him) < t0, "a sting on his pride costs his trust");
    assert_eq!(store::open(g.world.as_ref().unwrap(), id).unwrap().lines, 1, "the talk is open");
}

#[test]
fn a_parents_call_is_a_talk_with_the_parent_whose_trust_comes_home() {
    let (mut g, f, him) = game("old-parent", true);
    let f = f.unwrap();
    let id = meeting(&mut g, Kind::ParentCall, him, 0);
    let w = g.world.as_ref().unwrap();
    let room = super::room_of(w, &scene_of(&g, id));
    let parent = meetings::parent(w, him);
    assert_eq!(ptalk::across(&room), Some(parent), "the parent is across");
    let (pride, them) = tag(&g, id, "his pride");
    assert!(them.contains("parent") || them.contains("family"), "{them}");
    f.push(EAR, Ok(ear(vec![], vec![json!({"on": "", "item": pride, "how": "respected"})], false)));
    f.push(VOICE, Ok(voice("Thank you for hearing me out.", "")));
    let p0 = makeup::trust(g.world.as_ref().unwrap(), parent) as f32;
    let s0 = trust(&g, him);
    assert_eq!(say(&mut g, id, "I hear you. You know your son, and I respect that.")["state"], "done");
    let w = g.world.as_ref().unwrap();
    let up = makeup::trust(w, parent) as f32 - p0;
    assert!(up > 0.0 && up <= meetings::PARENT_UP + 1e-4, "parent up {up}");
    assert!((trust(&g, him) - s0 - up).abs() < 1e-4, "his son hears it at home");
    assert!(!scene_of(&g, id).done, "the call's own picks finish it");
}

#[test]
fn a_line_to_the_captains_reaches_every_one_of_them() {
    let (mut g, f, him) = game("old-leaders", true);
    let f = f.unwrap();
    let w = g.world.as_mut().unwrap();
    let mut s = Scene::new(Kind::Leaders, w.today, "The captains and the room");
    s.stops = false;
    s.place = "Your office, one at a time".into();
    let id = scene::add(w, s);
    ok(&mut g, "people_scene", json!({ "id": id }));
    let w = g.world.as_ref().unwrap();
    let room = super::room_of(w, &scene_of(&g, id));
    assert!(room.present.iter().any(|p| p.who == Them::Player(him)), "the captains are in the room");
    f.push(EAR, Ok(ear(vec![], vec![json!({"on": "p0", "item": "c0", "how": "respected"})], false)));
    f.push(VOICE, Ok(voice("We've got the room, Coach.", "")));
    let read = |g: &Game| -> (f32, f32) {
        let w = g.world.as_ref().unwrap();
        let i = afm_engine::world::folk::person(w, him).unwrap();
        (w.people[i].trust, w.people[i].mood)
    };
    let before = read(&g);
    let v = say(&mut g, id, "You lead this team. I need you to carry the message.");
    assert_eq!(v["state"], "done", "{v}");
    // Each captain heard it as his own (lane N4's sizes, its small noise
    // included): his trust and mood moved at once.
    assert_ne!(read(&g), before, "a line to the room lands on each captain");
    assert!(!scene_of(&g, id).done, "the meeting's own picks finish it");
}

#[test]
fn a_job_interviews_talk_counts_in_its_score_inside_one_answer() {
    let (mut g, f, _) = game("old-interview", true);
    let f = f.unwrap();
    let w = g.world.as_mut().unwrap();
    let seat = Team::seat(w).unwrap();
    let p = (0..w.programs.len()).find(|&p| Team::College(p) != seat).unwrap();
    let team = Team::College(p);
    let panel: Vec<Who> = [afm_engine::world::folk::cast::Role::AthleticDirector]
        .iter()
        .flat_map(|&r| afm_engine::world::folk::cast::all(w, team, r))
        .map(|c| Who::Cast(c.r.token()))
        .collect();
    let mut s = Scene::new(Kind::JobInterview, w.today, "Interview");
    s.team = Some(team);
    s.place = "A hotel conference room near the airport".into();
    s.a = p as u64;
    s.stops = false;
    s.who = panel;
    let id = scene::add(w, s);
    ok(&mut g, "people_scene", json!({ "id": id }));
    let (pride, _) = tag(&g, id, "his pride");
    for k in 0..4 {
        f.push(EAR, Ok(ear(vec![], vec![json!({"on": "", "item": pride, "how": "respected"})], false)));
        f.push(VOICE, Ok(voice("That's what we like to hear.", "")));
        let v = say(&mut g, id, &format!("I'll build this the right way, year by year, line {k}."));
        assert_eq!(v["state"], "done", "{v}");
    }
    let s = scene_of(&g, id);
    assert!(s.f[3] > 0.0 && s.f[3] <= meetings::TALK_SCORE_MAX + 1e-4, "talk score {}", s.f[3]);
}

#[test]
fn a_meeting_with_nobody_across_takes_no_typed_line() {
    let (mut g, _f, _) = game("old-nobody", true);
    let w = g.world.as_mut().unwrap();
    // A tour with nobody stored to give it (the group words only).
    let mut s = Scene::new(Kind::Tour, w.today, "The tour");
    s.stops = false;
    s.opened = true;
    s.who.push(Who::Group("Nobody you know".into()));
    s.pick(120, "Thank him for the tour.", "Praise", "warm", "Filed.");
    let id = scene::add(w, s);
    assert!(scene::takes_typed(&scene_of(&g, id)), "the core would have read it onto the chip");
    let e = g.call("people_say", &json!({ "id": id, "text": "Nice place." })).unwrap_err();
    assert!(e.contains("no typed"), "{e}");
    // How the world took your hire is the papers: no box at all.
    let w = g.world.as_mut().unwrap();
    let mut s = Scene::new(Kind::HireReaction, w.today, "How the world took your hire");
    s.opened = true;
    s.who.push(Who::Group("The press and the fans".into()));
    s.pick(1, "Read it and close the laptop.", "Listen", "calm", "Filed.");
    let id = scene::add(w, s);
    assert!(!scene::takes_typed(&scene_of(&g, id)));
}

#[test]
fn with_no_key_the_older_meetings_keep_their_chips() {
    let (mut g, _f, him) = game("old-nokey", false);
    let id = meeting(&mut g, Kind::Discipline, him, 0);
    let v = ok(&mut g, "people_scene", json!({ "id": id }));
    assert!(v.get("say").is_none(), "no box with no key: {v}");
    let s = scene_of(&g, id);
    let code = s.picks[0].code;
    ok(&mut g, "people_answer", json!({ "id": id, "code": code }));
    assert!(g.world.as_ref().unwrap().folk.live.persona.talks.is_empty(), "no v3 store is written");
}
